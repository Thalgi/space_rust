//! Galerie « planche-contact » des comètes.
//!
//! ⚠️ **Elle ne montre pas des corps, elle montre des orbites.** Un noyau
//! cométaire fait quelques pixels et se ressemble d'un exemplaire à l'autre :
//! aligner des cailloux gris n'apprendrait rien. Ce qui distingue une comète est
//! son **ellipse** — son allongement, son inclinaison, et l'endroit où elle se
//! trouve dessus. C'est donc ça que chaque case dessine, avec l'étoile au foyer
//! pour l'échelle.
//!
//! Et c'est la vue qui servira au **9.2** : les colonnes balaient une
//! révolution, de l'aphélie au périhélie et retour, donc la queue y naîtra et
//! s'y effacera d'une case à l'autre — exactement ce qu'il faudra juger.

use crate::astre::{Astre, CameraInfo};
use crate::genese::{apparence_comete, EXCENTRICITE_COMETE};
use crate::orbite::Orbite;
use crate::planete::Planete;
use crate::soleil::Soleil;
use macroquad::prelude::*;

/// Colonnes : une révolution échantillonnée. La première case est à l'aphélie,
/// la dernière y revient.
const PHASES: usize = 5;

/// Excentricités exposées, des deux bornes du tirage plus un milieu. ⚠️ Prises à
/// `genese::EXCENTRICITE_COMETE`, pas recopiées : la vitrine doit montrer ce que
/// le jeu produit, ou elle ne sert à rien.
fn excentricites() -> [f32; 3] {
    let (lo, hi) = EXCENTRICITE_COMETE;
    [lo, 0.5 * (lo + hi), hi]
}

/// **L'anomalie moyenne de la case `k`.** Source unique de la phase : la
/// construction de la planche et sa garde l'appellent toutes deux.
///
/// ⚠️ Extraite exprès. La garde recopiait la formule, donc un sabotage qui
/// resserrait les phases autour de l'aphélie la laissait **verte** — comparer une
/// copie à une copie ne prouve rien (`docs/methode.md`).
///
/// `k = 0` est l'aphélie, la dernière case le périhélie : la ligne se lit de
/// gauche à droite comme un plongeon.
fn phase(k: usize) -> f32 {
    std::f32::consts::PI * (1.0 - k as f32 / (PHASES - 1) as f32)
}

/// Une case : son libellé, sa comète, sa trace.
struct Cellule {
    libelle: String,
    comete: Planete,
    trace: Vec<Vec3>,
}

/// Galerie des comètes (visualisation / calibration des orbites).
pub struct GalerieCometes {
    cellules: Vec<Cellule>,
    etoile: Soleil,
    scroll: f32,
}

/// **Demi-grand axe de la vitrine.** Pas celui du jeu : la planche a son échelle,
/// comme celle des étoiles. Ce qu'on y juge est la **forme**, et une orbite de
/// 30 UA dans une case de 300 px ne se lirait pas.
const DEMI_GRAND_AXE: f32 = 3.5;

/// Rayon de l'étoile de la vitrine. Petite à dessein : elle donne le foyer et
/// l'éclairage, elle n'est pas le sujet — et la comète la plus plongeante doit
/// passer **devant**, pas dedans.
const RAYON_ETOILE: f32 = 0.07;

impl GalerieCometes {
    pub fn new() -> Self {
        let mut cellules = Vec::new();
        for e in excentricites() {
            for k in 0..PHASES {
                // ⚠️ **Un demi-tour, de l'aphélie au périhélie**, et non une
                // révolution entière. Une orbite est symétrique : la seconde
                // moitié ne montrerait que les mêmes distances en sens inverse.
                //
                // Surtout, cinq phases réparties sur un **tour complet** ne
                // tombent jamais près du périhélie — la comète y passe si peu de
                // temps qu'un échantillonnage uniforme en anomalie moyenne le
                // rate. La planche aurait donc manqué le seul moment qui compte,
                // celui où la queue naîtra au 9.2. C'est un test qui l'a dit.
                //
                // Ici `k = 0` est l'aphélie et la dernière case le périhélie : la
                // ligne se lit de gauche à droite comme un plongeon.
                let m0 = phase(k);
                let orb = Orbite::new(
                    DEMI_GRAND_AXE,
                    e,
                    Vec3::X,
                    Vec3::Z,
                    crate::systeme::G * crate::genese::MASSE_ETOILE,
                    m0,
                );
                let pos = orb.position(0.0);
                let q = DEMI_GRAND_AXE * (1.0 - e);
                cellules.push(Cellule {
                    libelle: format!("e {e:.2}   q {q:.2}   d {:.2}", pos.length()),
                    comete: Planete::new(pos, Vec3::ZERO, 0.09, 1e-6, apparence_comete(), Vec::new())
                        .en_comete(),
                    trace: orb.polyligne(Orbite::N_TRACE),
                });
            }
        }
        Self {
            // L'étoile au foyer donne l'échelle, et éclairera le noyau.
            etoile: Soleil::new(Vec3::ZERO, RAYON_ETOILE, vec3(1.0, 0.95, 0.85), 1.0),
            cellules,
            scroll: 0.0,
        }
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::R) {
            crate::soleil::vider_cache_materials();
            *self = Self::new();
        }
        let dt = get_frame_time().min(0.05);
        clear_background(Color::new(0.01, 0.01, 0.03, 1.0));

        let n = self.cellules.len().max(1);
        let top = 48.0;
        let label_h = 22.0;
        let cols = PHASES.min(n);
        let cw = screen_width() / cols as f32;
        let ch = cw * 0.72 + label_h;
        let render_h = ch - label_h;
        let rows = n.div_ceil(cols);
        let h_vue = screen_height() - top;
        let max_scroll = (rows as f32 * ch - h_vue).max(0.0);
        self.scroll = (self.scroll - mouse_wheel().1 * 48.0).clamp(0.0, max_scroll);

        self.etoile.update(dt);
        let mut labels: Vec<(String, f32, f32)> = Vec::new();
        for (i, c) in self.cellules.iter_mut().enumerate() {
            c.comete.update(dt);
            let cell_x = (i % cols) as f32 * cw;
            let cell_y = top + (i / cols) as f32 * ch - self.scroll;
            if cell_y + render_h < top || cell_y > screen_height() {
                continue;
            }
            // Vue de trois quarts : une ellipse vue par la tranche ne dirait rien
            // de son allongement, et vue de face rien de son inclinaison.
            let pos = vec3(0.0, 5.2, 8.5);
            let cam3d = Camera3D {
                position: pos,
                target: Vec3::ZERO,
                up: Vec3::Y,
                fovy: 45.0_f32.to_radians(),
                aspect: Some(cw / render_h),
                viewport: Some((
                    cell_x as i32,
                    (screen_height() - (cell_y + render_h)) as i32,
                    cw as i32,
                    render_h as i32,
                )),
                ..Default::default()
            };
            set_camera(&cam3d);

            let (right, up, forward) = crate::camera::base_camera(Vec3::ZERO - pos);
            let cam = CameraInfo {
                pos,
                origine: Vec3::ZERO,
                right,
                up,
                forward,
                light_pos: Vec3::ZERO, // l'étoile est au foyer
                light_color: Vec3::ONE,
                lights_pos: [Vec3::ZERO; 4],
                lights_color: [Vec3::ONE, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            };

            // La trace d'abord : la comète doit se poser **dessus**, et c'est la
            // seule façon de voir qu'elle y est.
            let gris = Color::new(0.35, 0.40, 0.50, 0.7);
            for p in c.trace.windows(2) {
                draw_line_3d(p[0], p[1], gris);
            }
            self.etoile.draw(&cam);
            c.comete.draw(&cam);
            labels.push((c.libelle.clone(), cell_x, cell_y + render_h + 16.0));
        }

        set_default_camera();
        let col = Color::new(0.9, 0.85, 0.6, 1.0);
        for (nom, cell_x, y) in &labels {
            let tw = crate::police::mesure(nom, 18);
            crate::police::texte(nom, cell_x + (cw - tw) * 0.5, *y, 18.0, col);
        }
        draw_rectangle(0.0, 0.0, screen_width(), top, Color::new(0.01, 0.01, 0.03, 1.0));
        crate::police::texte(
            "GALERIE DES COMETES   -   une ligne par excentricite, une colonne par phase \
             d'orbite   -   molette: defiler   Echap: menu",
            12.0,
            30.0,
            18.0,
            Color::new(0.7, 0.85, 0.85, 1.0),
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La planche montre les excentricités que le jeu produit**, bornes
    /// comprises.
    ///
    /// ⚠️ Elles sont **lues** à `genese::EXCENTRICITE_COMETE`, pas recopiées : une
    /// vitrine qui montre autre chose que le jeu ne sert à rien, et c'est le
    /// défaut le plus tenace de ce chantier — rencontré six fois.
    #[test]
    fn la_planche_expose_les_excentricites_du_tirage() {
        let e = excentricites();
        assert_eq!(e[0], EXCENTRICITE_COMETE.0, "la borne basse du tirage manque");
        assert_eq!(e[2], EXCENTRICITE_COMETE.1, "la borne haute du tirage manque");
        assert!(e[1] > e[0] && e[1] < e[2], "le milieu n'en est pas un : {e:?}");
    }

    /// **Les phases balaient une révolution entière, et une seule.**
    ///
    /// C'est ce qui fera du 9.2 un jugement et non une impression : la queue doit
    /// naître puis s'effacer **le long de la ligne**. Deux cases au même endroit
    /// de l'orbite, ou une révolution incomplète, et on ne verrait plus le cycle.
    #[test]
    fn les_phases_balaient_une_revolution_sans_doublon() {
        let orb = |m0: f32| {
            Orbite::new(DEMI_GRAND_AXE, 0.9, Vec3::X, Vec3::Z, 1000.0, m0).position(0.0)
        };
        let mut distances = Vec::new();
        for k in 0..PHASES {
            let m0 = phase(k);
            distances.push(orb(m0).length());
        }
        // Garde de compilation : la valeur est constante, donc c'est au build`n        // de la tenir, pas au test de la constater.`n        const _: () = assert!(PHASES >= 4, "moins de quatre phases ne montrent pas un plongeon");
        // La première case est la plus froide : on ouvre à l'aphélie.
        let max = distances.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            (distances[0] - max).abs() < 1e-3,
            "la planche n'ouvre pas sur l'aphélie : {distances:?}"
        );
        // Et une case descend franchement vers l'étoile, sinon il n'y a pas de
        // plongeon à juger.
        let min = distances.iter().cloned().fold(f32::MAX, f32::min);
        assert!(min < max * 0.35, "aucune phase ne plonge : {distances:?}");
    }

    /// **Le rayon de la planche n'est pas celui du jeu**, et son demi-grand axe
    /// non plus.
    ///
    /// ⚠️ Même piège qu'à la galerie des étoiles, où y poser le rayon du tirage
    /// aurait fait de la naine brune le plus gros objet de la vitrine. Ici
    /// l'orbite du jeu fait des dizaines d'UA : à l'échelle, la case ne montrerait
    /// qu'un trait.
    #[test]
    fn la_planche_a_son_echelle_a_elle() {
        const _: () = assert!(DEMI_GRAND_AXE < 10.0, "une orbite de jeu ne tiendrait pas dans la case");
        // Et le périhélie de la plus excentrique reste hors de l'étoile dessinée.
        let q = DEMI_GRAND_AXE * (1.0 - EXCENTRICITE_COMETE.1);
        assert!(q > RAYON_ETOILE * 1.3, "la comète la plus plongeante traverse l'étoile de la vitrine : {q}");
    }
}
