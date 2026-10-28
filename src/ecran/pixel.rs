//! **Le filtre « pixel art »**, et le blit qui le termine.
//!
//! La mécanique est la même partout dans le jeu : la scène 3D est rendue dans
//! une cible basse résolution, puis remontée à l'écran au plus proche voisin. Le
//! fond stellaire et les textes restent nets — seul ce qui est dessiné entre
//! `preparer` et `presenter` est pixelisé.
//!
//! Ce module tient **trois faits en un seul endroit**, parce qu'ils sont
//! partagés par quatre vues (skymap, objet, galerie, stations) :
//!
//! 1. le facteur de réduction [`PIX`] et la taille de cible qui en découle ;
//! 2. le **blit** final, et donc le moment où la quantification de palette
//!    s'applique ;
//! 3. le matériau de quantification lui-même, construit une seule fois.
//!
//! Le mode actif, lui, n'est pas ici : il appartient aux réglages
//! ([`crate::reglages::mode_rendu`]), qui sont ce que l'écran des paramètres
//! pilote.

use crate::palette;
use macroquad::prelude::*;
use std::cell::{Cell, RefCell};

/// Facteur de réduction : plus petit = plus de pixels.
///
/// **Source unique** — `rendu.rs` et la galerie s'en servent aussi. Deux
/// facteurs différents donneraient des tailles de pixel qui changent selon la
/// vue, ce qui se voit immédiatement.
pub const PIX: u32 = 2;

/// Taille de la cible basse résolution pour l'écran courant.
pub fn dimensions() -> (u32, u32) {
    (
        (screen_width() as u32 / PIX).max(2),
        (screen_height() as u32 / PIX).max(2),
    )
}

/// Crée une cible basse résolution prête pour une passe 3D.
///
/// `depth: true` est **indispensable** : sans attachement de profondeur, le
/// depth test est muet dans la cible et c'est l'ordre de dessin qui gagne (les
/// ceintures passeraient devant les planètes).
pub fn nouvelle_cible(l: u32, h: u32) -> RenderTarget {
    let rt = render_target_ex(l, h, RenderTargetParams { sample_count: 1, depth: true });
    // Sans le plus proche voisin, l'agrandissement interpole et il ne reste
    // aucun bord net : tout l'effet disparaît.
    rt.texture.set_filter(FilterMode::Nearest);
    rt
}

thread_local! {
    /// Le matériau de quantification.
    ///
    /// `None` : pas encore tenté. `Some(None)` : le shader n'a pas compilé — on
    /// retombe alors sur un blit nu, sans jamais réessayer. Un jeu qui perd ses
    /// couleurs vaut mieux qu'un jeu qui s'arrête, et qu'un jeu qui retente une
    /// compilation ratée à chaque frame.
    static QUANTIFICATEUR: RefCell<Option<Option<Material>>> = const { RefCell::new(None) };
    /// Quelle palette est actuellement chargée dans les uniformes. Les tableaux
    /// ne sont réécrits que sur **changement** : 128 vec3 par frame seraient du
    /// gaspillage pour une donnée qui ne bouge qu'au clic.
    static PALETTE_CHARGEE: Cell<Option<usize>> = const { Cell::new(None) };
    /// La cible où la scène se quantifie, **à sa propre taille** (Opti-v2).
    static QUANTIFIEE: RefCell<Option<RenderTarget>> = const { RefCell::new(None) };
}

/// Compile le matériau de quantification.
fn charger() -> Option<Material> {
    let mat = load_material(
        ShaderSource::Glsl {
            // Le vertex shader est celui de macroquad, repris **intégralement**
            // avec ses quatre attributs : miniquad résout les attributs par nom
            // au moment de construire le pipeline, et un `color0` manquant
            // laisserait un attribut non lié.
            vertex: VERT,
            fragment: &crate::impostor::source("palette.frag.glsl", FRAG),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::array(
                    UniformDesc::new("palette_lab", UniformType::Float3),
                    palette::MAX,
                ),
                UniformDesc::array(
                    UniformDesc::new("palette_rgb", UniformType::Float3),
                    palette::MAX,
                ),
                UniformDesc::new("nb_couleurs", UniformType::Float1),
                UniformDesc::new("taille_cible", UniformType::Float2),
                UniformDesc::new("gamma", UniformType::Float1),
                UniformDesc::new("ecretage_seuil", UniformType::Float1),
                UniformDesc::new("ecretage_force", UniformType::Float1),
                UniformDesc::new("saturation", UniformType::Float1),
                UniformDesc::new("sat_hautes", UniformType::Float1),
                UniformDesc::new("sat_rolloff", UniformType::Float2),
            ],
            // **Sans mélange** : la quantification écrit dans une cible vide
            // (`quantifier`), couleur et alpha tels quels. C'est le blit à
            // l'écran, au matériau par défaut, qui compose ensuite par-dessus
            // le fond stellaire net — mélanger deux fois éteindrait les bords.
            pipeline_params: PipelineParams::default(),
            ..Default::default()
        },
    )
    .map_err(|e| error!("shader de palette non compilé, rendu sans quantification : {e}"))
    .ok()?;

    mat.set_uniform("ecretage_seuil", palette::ECRETAGE_SEUIL);
    mat.set_uniform("ecretage_force", palette::ECRETAGE_FORCE);
    mat.set_uniform("sat_hautes", palette::SAT_HAUTES);
    mat.set_uniform("sat_rolloff", palette::SAT_ROLLOFF);
    Some(mat)
}

/// Active le matériau de quantification et lui passe l'état courant. Renvoie
/// `false` s'il n'est pas disponible — l'appelant blitte alors sans lui.
fn activer_quantificateur(cible: Vec2) -> bool {
    let etat = crate::reglages::etat_rendu();
    QUANTIFICATEUR.with(|c| {
        let mut c = c.borrow_mut();
        let Some(m) = c.get_or_insert_with(charger).as_ref() else {
            return false;
        };

        // La palette ne repart vers le GPU que si elle a changé.
        let p = palette::palette(etat.palette);
        if PALETTE_CHARGEE.with(|c| c.get()) != Some(etat.palette) {
            m.set_uniform_array("palette_lab", &p.tableau(&p.lab)[..]);
            m.set_uniform_array("palette_rgb", &p.tableau(&p.rgb)[..]);
            m.set_uniform("nb_couleurs", p.nb() as f32);
            PALETTE_CHARGEE.with(|c| c.set(Some(etat.palette)));
        }

        // Ceux-ci changent avec la fenêtre ou le réglage : à chaque blit.
        m.set_uniform("taille_cible", (cible.x, cible.y));
        m.set_uniform("gamma", etat.gamma);
        m.set_uniform("saturation", etat.saturation.gain());
        gl_use_material(m);
        true
    })
}

/// **Remonte une cible basse résolution à l'écran.**
///
/// À appeler après `set_default_camera`. Quantifie vers la palette si le mode de
/// rendu le demande. C'est le seul endroit du jeu qui décide de l'un ou de
/// l'autre : les quatre vues passent toutes par ici.
pub fn blit(texture: &Texture2D) {
    let quantifiee = if crate::reglages::mode_rendu().quantifie() { quantifier(texture) } else { None };
    draw_texture_ex(
        quantifiee.as_ref().unwrap_or(texture),
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            // `flip_y` : une cible de rendu s'adresse de bas en haut.
            dest_size: Some(vec2(screen_width(), screen_height())),
            flip_y: true,
            ..Default::default()
        },
    );
}

/// **Quantifie la cible à sa propre taille**, dans une seconde cible ; `None`
/// si le matériau manque.
///
/// ⚠️ La quantification se faisait **pendant** l'agrandissement à l'écran :
/// chaque gros pixel, `PIX` × `PIX` pixels d'écran, était quantifié quatre
/// fois — jusqu'à 182 distances Lab chacune. Sur le premier APU Ryzen de
/// l'utilisateur, le rendu Net allait plus vite que le Palette (Opti-v2,
/// `DIAG`). Le résultat est le même : un gros pixel ne lit qu'un texel, et le
/// shader n'a pas de tramage lié à l'écran.
fn quantifier(texture: &Texture2D) -> Option<Texture2D> {
    let cible = vec2(texture.width(), texture.height());
    QUANTIFIEE.with(|q| {
        let mut q = q.borrow_mut();
        if q.as_ref().is_none_or(|rt| rt.texture.size() != cible) {
            let rt = render_target_ex(cible.x as u32, cible.y as u32, RenderTargetParams { sample_count: 1, depth: false });
            rt.texture.set_filter(FilterMode::Nearest);
            *q = Some(rt);
        }
        let rt = q.as_ref()?;
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, cible.x, cible.y));
        cam.render_target = Some(rt.clone());
        set_camera(&cam);
        clear_background(BLANK);
        let ok = activer_quantificateur(cible);
        if ok {
            // `flip_y` : la cible quantifiée garde le sens de la source (de bas
            // en haut), que le blit à l'écran retourne comme avant.
            draw_texture_ex(texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(cible), flip_y: true, ..Default::default() });
            gl_use_default_material();
        }
        set_default_camera();
        ok.then(|| rt.texture.clone())
    })
}

/// Vertex shader par défaut de macroquad, recopié : un matériau doit fournir le
/// sien, et celui-ci doit déclarer exactement les attributs que macroquad pousse.
const VERT: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

const FRAG: &str = include_str!("../shaders/palette.frag.glsl");

/// Filtre pixel pour les vues qui rendent une scène unique plein écran
/// (briques, stations, vaisseaux).
///
/// Il ne porte **plus** d'interrupteur : le mode vient des réglages, pour que la
/// touche P et l'écran des paramètres ne puissent pas se contredire.
pub struct FiltrePixel {
    cible: Option<RenderTarget>,
    dims: (u32, u32),
}

impl FiltrePixel {
    pub fn new() -> Self {
        Self { cible: None, dims: (0, 0) }
    }

    /// Passe au mode de rendu suivant — c'est le raccourci clavier des vues.
    pub fn basculer(&mut self) {
        crate::reglages::regler_rendu(crate::reglages::mode_rendu().suivant());
    }

    /// Si le filtre est actif : (re)crée la cible basse résolution, la nettoie
    /// en transparent (elle sera composée PAR-DESSUS le décor net) et branche
    /// `cam3d` dessus. Sans filtre : ne touche à rien.
    pub fn preparer(&mut self, cam3d: &mut Camera3D) {
        if !crate::reglages::mode_rendu().pixelise() {
            return;
        }
        let dims = dimensions();
        if self.dims != dims || self.cible.is_none() {
            self.cible = Some(nouvelle_cible(dims.0, dims.1));
            self.dims = dims;
        }
        // Nettoyage transparent : le vide laisse voir le fond net en dessous.
        set_camera(&Camera2D { render_target: self.cible.clone(), ..Default::default() });
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        set_default_camera();
        cam3d.render_target = self.cible.clone();
    }

    /// Blit de la cible sur tout l'écran (après `set_default_camera`).
    pub fn presenter(&self) {
        if !crate::reglages::mode_rendu().pixelise() {
            return;
        }
        if let Some(rt) = &self.cible {
            blit(&rt.texture);
        }
    }
}

#[cfg(test)]
mod tests_partage {
    /// **Qui honore le mode pixel doit blitter par ce module.**
    ///
    /// Trois choses vivent ici : le facteur de réduction, la cible basse
    /// résolution, et le **blit** — qui est là où s'applique la quantification de
    /// palette, donc là où le pixel art devient du pixel art (des à-plats, cf.
    /// `ModeRendu::Palette`).
    ///
    /// La galerie des ceintures s'en était fait une copie : son propre facteur,
    /// sa propre cible, et un blit nu à la place de celui-ci. Elle pixelisait
    /// donc **sans jamais quantifier** — les débris n'avaient pas l'aspect pixel
    /// art alors que les planètes, passées par le module, l'avaient. Le défaut
    /// ne se voit pas en lisant la vue fautive, où tout semble normal ; il ne se
    /// voit qu'en comparant deux vues.
    ///
    /// La règle porte sur le **couplage réel** et non sur l'appel de dessin :
    /// dessiner une texture est légitime ailleurs (la vignette d'astre pose une
    /// miniature dans un panneau, sans rapport avec le filtre). Ce qui ne l'est
    /// pas, c'est de lire le mode pixel et de finir la passe soi-même.
    #[test]
    fn qui_pixelise_passe_par_le_blit_partage() {
        let racine = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        // Aiguilles assemblées : ce fichier parle des deux, il ne doit pas se
        // dénoncer lui-même.
        let lit_le_mode = format!("{}()", "pixelise");
        let blit = format!("{}::{}(", "pixel", "blit");
        let mut consommateurs = 0;
        let mut fautifs = Vec::new();

        fn parcourir(d: &std::path::Path, f: &mut dyn FnMut(&str, &str)) {
            let Ok(entrees) = std::fs::read_dir(d) else { return };
            for e in entrees.flatten() {
                let p = e.path();
                if p.is_dir() {
                    parcourir(&p, f);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let nom = p.file_name().unwrap().to_string_lossy().to_string();
                    f(&nom, &std::fs::read_to_string(&p).unwrap_or_default());
                }
            }
        }

        parcourir(std::path::Path::new(racine), &mut |nom, src| {
            // Le module de blit et celui qui DÉFINIT le mode sont hors sujet.
            if nom == "pixel.rs" || nom == "reglages.rs" {
                return;
            }
            let code: String = src
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("
");
            if !code.contains(&lit_le_mode) {
                return;
            }
            consommateurs += 1;
            if !code.contains(&blit) {
                fautifs.push(nom.to_string());
            }
        });

        assert!(
            consommateurs >= 3,
            "seulement {consommateurs} vue(s) trouvée(s) : le parcours est cassé"
        );
        assert!(
            fautifs.is_empty(),
            "ces vues lisent le mode pixel mais terminent la passe elles-mêmes,              et perdent donc la quantification de palette : {fautifs:?}"
        );
    }
}
