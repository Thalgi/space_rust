use crate::astre::{Astre, CameraInfo};
use crate::etoile::couleur_corps_noir;
use crate::soleil::Soleil;
use crate::disque::Accretion;
use crate::stellaire::Variante;
use macroquad::prelude::*;

/// **Étoiles par ligne.**
///
/// C'était une division de la largeur d'écran par 210 px : en 1440p, douze
/// colonnes, donc douze étoiles minuscules — trop petites pour qu'on y
/// juge la granulation, une tache ou une boucle, alors que c'est justement
/// cette vue qui sert à les juger (`astres_v2.md` §0 : rien ne se lit sous
/// ~10 px de rayon). Un nombre fixe fait grandir les cellules avec l'écran
/// au lieu de les multiplier.
const COLONNES: usize = 5;

/// Catalogue des étoiles : (nom, rayon, température K, luminosité, couronne 0=halo 1=jets 2=vent).
fn catalogue() -> Vec<(&'static str, f32, f32, f32, f32)> {
    vec![
        // Séquence principale O -> M.
        ("O (bleue)", 1.05, 38000.0, 4.0, 0.0),
        ("B (bleu-blanc)", 0.98, 20000.0, 3.0, 0.0),
        ("A (blanche)", 0.9, 9000.0, 2.0, 0.0),
        ("F (blanc-jaune)", 0.85, 6800.0, 1.4, 0.0),
        ("G (jaune - Soleil)", 0.8, 5700.0, 1.0, 0.0),
        ("K (orange)", 0.75, 4500.0, 0.7, 0.0),
        ("M (naine rouge)", 0.58, 3200.0, 0.35, 0.0),
        // Naines / sous-naines.
        ("Naine blanche", 0.45, 16000.0, 0.3, 0.0),
        ("Sous-naine (sdB)", 0.55, 25000.0, 0.6, 0.0),
        // Géantes / supergéantes.
        ("Geante rouge", 1.15, 3400.0, 4.5, 0.0),
        ("Supergeante rouge", 1.2, 3500.0, 6.0, 0.0),
        ("Supergeante bleue", 1.15, 21000.0, 6.0, 2.0), // vent stellaire
        // Particulières.
        ("Etoile carbonee (C)", 1.0, 2800.0, 2.5, 0.0),
        ("T Tauri (jeune)", 0.85, 4200.0, 1.2, 6.0), // jets bipolaires + disque protosolaire
        ("Wolf-Rayet", 1.05, 40000.0, 5.0, 2.0),     // vent violent
        ("Etoile a flares (M)", 0.58, 3000.0, 0.4, 0.0),
        ("Etoile a neutrons", 0.42, 40000.0, 0.2, 1.0), // jets fixes
        ("Pulsar", 0.42, 40000.0, 0.25, 3.0),           // jets qui tournent (phare)
        ("Magnetar", 0.45, 40000.0, 0.3, 4.0), // arcs magnétiques
    ]
}

/// **Les trois régimes du trou noir**, exposés côte à côte.
///
/// En jeu le régime se **dérive de la graine** : on tombe sur l'un des trois,
/// et il faudrait régénérer jusqu'à voir les autres. Une planche-contact doit
/// les montrer ensemble — c'est sa raison d'être.
///
/// ⚠️ Le gabarit (rayon, température, luminosité) vient de
/// `etoile::particuliere`, **pas d'une copie** : la galerie montre ce que le
/// jeu produit, ou elle ne sert à rien.
fn regimes_trou_noir() -> [Accretion; 3] {
    [Accretion::endormie(), Accretion::chaude(), Accretion::vorace()]
}

/// Les cellules correspondantes. ⚠️ Séparée de [`regimes_trou_noir`] parce
/// qu'elle construit des `Soleil`, ce qui exige le contexte graphique : une
/// fonction qui le réclame ne se teste pas. La **décision** — quels régimes,
/// sous quels noms — reste donc vérifiable.
fn trous_noirs() -> Vec<(String, Soleil)> {
    let (tmin, tmax, rayon, lumi, _, mode) = crate::etoile::particuliere(0.0);
    let couleur = couleur_corps_noir((tmin + tmax) * 0.5);
    regimes_trou_noir()
        .into_iter()
        .map(|regime| {
            let s =
                Variante::de_mode(mode).appliquer(Soleil::new(Vec3::ZERO, rayon, couleur, lumi));
            (format!("Trou noir - {}", regime.nom()), s.avec_regime(regime))
        })
        .collect()
}

/// Galerie « planche-contact » des types d'étoiles (visualisation/calibration).
pub struct GalerieEtoiles {
    cellules: Vec<(String, Soleil)>,
    scroll: f32,
}

impl GalerieEtoiles {
    pub fn new() -> Self {
        let cellules = catalogue()
            .into_iter()
            .map(|(nom, rayon, temp, lumi, mode)| {
                let s = Soleil::new(Vec3::ZERO, rayon, couleur_corps_noir(temp), lumi);
                // ⚠️ **La même source que la genèse.** Cette chaîne était
                // recopiée ici, et les deux avaient déjà divergé : le
                // `mode > 4.5` local attrapait aussi le mode 6, et le T Tauri
                // du catalogue restait au mode 1, donc sans disque.
                (nom.to_string(), Variante::de_mode(mode).appliquer(s))
            })
            .chain(trous_noirs())
            .collect();
        Self { cellules, scroll: 0.0 }
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
        let cols = COLONNES.min(n);
        let cw = screen_width() / cols as f32;
        // ⚠️ La hauteur de cellule était **fixe** (180 px) alors que la
        // largeur suit l'écran : en 1440p, chaque étoile se retrouvait dans
        // une case large et plate, trop petite pour qu'on y voie la
        // granulation ou une tache. Elle suit maintenant la largeur.
        let ch = cw * 0.72 + label_h;
        let render_h = ch - label_h;
        let rows = n.div_ceil(cols);
        let h_vue = screen_height() - top;
        let max_scroll = (rows as f32 * ch - h_vue).max(0.0);
        self.scroll = (self.scroll - mouse_wheel().1 * 48.0).clamp(0.0, max_scroll);

        // --- Phase 3D : dessiner chaque étoile dans son viewport. ---
        let mut labels: Vec<(String, f32, f32)> = Vec::new();
        for (i, (nom, soleil)) in self.cellules.iter_mut().enumerate() {
            soleil.update(dt); // animation (granulation, taches, éruptions)
            let cell_x = (i % cols) as f32 * cw;
            let cell_y = top + (i / cols) as f32 * ch - self.scroll;
            if cell_y + render_h < top || cell_y > screen_height() {
                continue;
            }
            let pos = vec3(0.0, 0.0, 7.5); // recul pour cadrer couronne / jets / vent
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
                // Le sujet est posé à l'origine du monde et la caméra à
                // quelques unités : aucun grand nombre, donc rien à
                // translater — comportement inchangé à l'identique.
                origine: Vec3::ZERO,
                right,
                up,
                forward,
                light_pos: vec3(0.0, 0.0, 10.0),
                light_color: Vec3::ONE,
                lights_pos: [vec3(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
                lights_color: [Vec3::ONE, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            };
            soleil.draw(&cam);
            labels.push((nom.clone(), cell_x, cell_y + render_h + 16.0));
        }

        // --- Phase 2D : texte après une seule remise en caméra écran. ---
        set_default_camera();
        let col = Color::new(0.9, 0.85, 0.6, 1.0);
        for (nom, cell_x, y) in &labels {
            let tw = crate::police::mesure(nom, 18);
            crate::police::texte(nom, cell_x + (cw - tw) * 0.5, *y, 18.0, col);
        }

        draw_rectangle(0.0, 0.0, screen_width(), top, Color::new(0.01, 0.01, 0.03, 1.0));
        crate::police::texte(
            "GALERIE DES ETOILES   -   molette: defiler   R: shaders   Echap: menu",
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

    /// **Aucune entrée du catalogue ne porte un mode que personne ne sait
    /// traduire.**
    ///
    /// Le catalogue est écrit à la main, et son mode est un `f32` que rien ne
    /// contraint : une valeur inconnue retombe sur `Normale` **en silence**,
    /// et la vitrine montre une étoile ordinaire là où elle promet un pulsar
    /// ou un disque. C'est la vue qui sert à juger les astres — elle ne peut
    /// pas mentir sur ce qu'elle montre.
    #[test]
    fn chaque_entree_du_catalogue_porte_un_mode_connu() {
        for (nom, _, _, _, mode) in catalogue() {
            let inconnu =
                matches!(Variante::de_mode(mode), Variante::Normale) && mode as i32 != 0;
            assert!(!inconnu, "{nom} porte le mode {mode}, que personne ne sait traduire");
        }
    }

    /// **Et les types qui portent un disque sont bien exposés** : c'est la
    /// seule vue où l'on puisse les juger côte à côte.
    #[test]
    fn les_porteurs_de_disque_sont_exposes() {
        let modes: Vec<f32> = catalogue().into_iter().map(|e| e.4).collect();
        assert!(modes.contains(&6.0), "aucune jeune étoile à disque en galerie");
        // ⚠️ Le trou noir a quitté le catalogue pour `trous_noirs()`, qui en
        // montre les **trois régimes**. En jeu celui-ci se dérive de la
        // graine : on tombe sur l'un des trois, et il faudrait régénérer
        // pour voir les autres — une planche-contact doit les montrer
        // ensemble.
        assert!(!modes.contains(&5.0), "le trou noir est resté au catalogue : il y sera en double");
    }

    /// **Les trois régimes sont exposés, et ce sont bien trois régimes
    /// différents.** Trois cellules identiques ne compareraient rien.
    #[test]
    fn les_trois_regimes_du_trou_noir_sont_exposes() {
        let tn = regimes_trou_noir();
        // Chacun se nomme, et les trois noms diffèrent : trois cellules
        // identiques ne compareraient rien.
        let noms: Vec<&str> = tn.iter().map(|r| r.nom()).collect();
        for (i, n) in noms.iter().enumerate() {
            assert!(!noms[..i].contains(n), "deux cellules montrent le régime « {n} »");
        }
        let eclats: Vec<f32> = tn.iter().map(|r| r.eclat).collect();
        for (i, e) in eclats.iter().enumerate() {
            assert!(!eclats[..i].contains(e), "deux cellules ont le même éclat");
        }
        // Et le vorace, seul, traîne sa ceinture.
        assert_eq!(tn.iter().filter(|r| r.ceinture.is_some()).count(), 1);
    }
}
