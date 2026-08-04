//! **Vignette d'astre** : le portrait qui coiffe le panneau de droite.
//!
//! Le schéma d'interface montre la planète cliquée, rendue, en haut de l'encart
//! (`docs/conception/interface.md` §1.2, ⓔ). C'était la dette **D-INT-3** — un
//! disque de couleur unie en tenait lieu.
//!
//! # Pourquoi une cible de rendu, et non un viewport
//!
//! La galerie dessine ses planètes dans des **viewports** découpés dans l'écran
//! (`ecran/galerie.rs`), et c'est le bon outil là-bas : elle ne rend qu'elles.
//! Ici, la scène du système est **déjà dessinée**, profondeur comprise. Un
//! viewport poserait la vignette dans ce tampon-là, et l'astre se ferait
//! découper par ce qui traîne devant lui dans le monde — un anneau, une lune,
//! parfois rien du tout selon l'angle de caméra.
//!
//! Une cible séparée a son propre tampon de profondeur, effacé à chaque
//! rendu : la vignette ne peut donc pas se faire mordre. Elle se recolle
//! ensuite comme une simple texture.
//!
//! # Le corps n'est pas déplacé
//!
//! On approche la **caméra** de l'astre là où il est, plutôt que de le poser à
//! l'origine : sa position sert à son propre dessin (terminateur, éclairage
//! multi-étoiles, anneaux orientés). Le bouger pour le portrait le montrerait
//! éclairé autrement que dans la vue.

use crate::astre::CameraInfo;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Côté de la cible, en pixels. Fixe et généreux : la vignette est ensuite mise
/// à l'échelle du panneau, et repartir d'une cible plus fine que l'affichage
/// donnerait une image molle sur un grand écran.
pub const COTE: u32 = 192;

/// Recul de la caméra, en rayons de l'astre. 3 R cadre le corps en laissant un
/// peu de vide : plus près il déborde, plus loin il se perd dans la case.
const RECUL: f32 = 3.0;
/// Recul pour un corps à anneau, en rayons **d'anneau** : sinon l'anneau sort
/// du cadre et la planète a l'air posée sur deux traits.
const RECUL_ANNEAU: f32 = 2.6;
/// Élévation de la caméra, en fraction du recul. Une vue strictement équatoriale
/// réduit un anneau à un trait ; un léger dessus lui rend son ellipse.
const ELEVATION: f32 = 0.22;

/// Cible de rendu réutilisée d'une frame à l'autre.
pub struct Vignette {
    cible: Option<RenderTarget>,
}

impl Vignette {
    pub fn new() -> Self {
        Self { cible: None }
    }

    /// Rend l'astre `idx` dans la cible, puis la dessine dans `zone`.
    ///
    /// Ne fait rien si l'index est invalide. Le rendu a lieu **à chaque frame**
    /// tant que le panneau est ouvert : c'est un seul corps, et le mettre en
    /// cache demanderait d'invalider au bon moment (rotation, éclairage qui
    /// bouge, changement de système) — plus de risque que de gain.
    pub fn dessiner(&mut self, sys: &mut Systeme, idx: usize, zone: Rect) {
        let Some(rayon) = sys.rayon_de(idx) else { return };
        let rayon = rayon.max(1e-4);

        let cible = self.cible.get_or_insert_with(|| {
            // `depth: true` — sans tampon de profondeur, les faces arrière d'une
            // sphère passent devant les faces avant.
            let rt = render_target_ex(COTE, COTE, RenderTargetParams { sample_count: 1, depth: true });
            rt.texture.set_filter(FilterMode::Linear);
            rt
        });

        let centre = sys.position(idx);
        let etendue = sys.rayon_visuel_de(idx).unwrap_or(rayon).max(rayon);
        let recul = if etendue > rayon * 1.05 { etendue * RECUL_ANNEAU } else { rayon * RECUL };
        // La caméra regarde l'astre depuis le **côté éclairé**, un peu en
        // hauteur : de face contre la lumière on ne verrait qu'un croissant, et
        // à contre-jour un disque noir.
        let vers_lumiere = (sys.position_lumiere() - centre).normalize_or_zero();
        let cote = if vers_lumiere == Vec3::ZERO { Vec3::Z } else { vers_lumiere };
        let pos = centre + cote * recul + Vec3::Y * (recul * ELEVATION);

        let cam3d = Camera3D {
            position: pos,
            target: centre,
            up: Vec3::Y,
            fovy: 45.0_f32.to_radians(),
            aspect: Some(1.0),
            render_target: Some(cible.clone()),
            ..Default::default()
        };
        set_camera(&cam3d);
        clear_background(Color::new(0.01, 0.02, 0.05, 1.0));

        let forward = (centre - pos).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let cam = CameraInfo {
            pos,
            right,
            up,
            forward,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        };
        sys.dessiner_astre(idx, &cam);
        set_default_camera();

        // La texture d'une cible de rendu est **retournée** verticalement par
        // rapport à l'écran (origine GL en bas à gauche) : sans `flip_y`, la
        // planète apparaîtrait la tête en bas, ce qui ne se voit que sur les
        // corps à anneau incliné.
        draw_texture_ex(
            &cible.texture,
            zone.x,
            zone.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(zone.w, zone.h)),
                flip_y: true,
                ..Default::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Le cadrage est la seule chose de ce module qui se calcule ; le reste est
    // du dessin. Un corps à anneau doit être vu de **plus loin**, sinon
    // l'anneau sort du cadre — c'est la raison d'être des deux constantes.
    #[test]
    fn un_corps_a_anneau_se_cadre_de_plus_loin() {
        let rayon = 1.0_f32;
        let sans = rayon * RECUL;
        // Un anneau de Saturne s'étend à ~2,3 rayons.
        let avec = (rayon * 2.3) * RECUL_ANNEAU;
        assert!(avec > sans, "l'anneau tiendrait hors du cadre : {avec} contre {sans}");
    }

    // La cible est carrée et l'aspect vaut 1 : une cible rectangulaire
    // écraserait la planète en ovale, ce qui ne se remarque pas tout de suite.
    #[test]
    fn la_cible_est_carree_et_assez_fine() {
        assert!(COTE >= 128, "une vignette de {COTE} px serait molle sur un grand écran");
    }
}
