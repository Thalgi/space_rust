use crate::ui::minitel_ligne;
use macroquad::prelude::*;

/// Destination choisie depuis l'accueil.
#[derive(Clone, Copy)]
pub enum Cible {
    Starmap,
    Skymap,
    Objet,
    Galerie,
    GalerieGaz,
    GalerieDisques,
    GalerieEtoiles,
    Vaisseaux,
    Briques,
    Station,
}

/// Écran d'accueil : titre + deux blocs de boutons (astres / vaisseaux).
pub struct Accueil;

/// Bloc de gauche : exploration et galeries d'astres.
const BLOC_ASTRES: &[(&str, Cible)] = &[
    ("SKYMAP - SYSTEME COMPLET", Cible::Skymap),
    ("OBJET CELESTE - VUE ISOLEE", Cible::Objet),
    ("STARMAP - VOISINAGE STELLAIRE", Cible::Starmap),
    ("GALERIE - TYPES TELLURIQUES", Cible::Galerie),
    ("GALERIE - GEANTES GAZEUSES", Cible::GalerieGaz),
    ("GALERIE - CEINTURES & DISQUES", Cible::GalerieDisques),
    ("GALERIE - ETOILES", Cible::GalerieEtoiles),
];

/// Bloc de droite : vaisseaux, briques et stations.
const BLOC_VAISSEAUX: &[(&str, Cible)] = &[
    ("VAISSEAUX - SONDES / NAVETTES / STATIONS", Cible::Vaisseaux),
    ("BRIQUES - COMPOSANTS DE STATION", Cible::Briques),
    ("STATION - ASSEMBLAGE (DEMO)", Cible::Station),
];

impl Accueil {
    pub fn new() -> Self {
        Self
    }

    /// Dessine un bloc (titre + boutons empilés) et renvoie la cible cliquée.
    fn bloc(
        titre: &str,
        entrees: &'static [(&str, Cible)],
        x: f32,
        y0: f32,
        m: Vec2,
        clic: bool,
    ) -> Option<&'static Cible> {
        let bw = 380.0;
        let bh = 40.0;
        let gap = 12.0;
        crate::police::texte(titre, x + 4.0, y0 - 10.0, 20.0, Color::new(0.5, 0.9, 0.7, 1.0));
        let mut choix = None;
        for (i, (label, cible)) in entrees.iter().enumerate() {
            let r = Rect::new(x, y0 + i as f32 * (bh + gap), bw, bh);
            minitel_ligne(r, label, m);
            if clic && r.contains(m) {
                choix = Some(cible);
            }
        }
        choix
    }

    /// Dessine l'accueil et renvoie la destination si un bouton est cliqué.
    pub fn frame(&mut self) -> Option<Cible> {
        clear_background(Color::new(0.01, 0.01, 0.04, 1.0));
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);

        let cx = screen_width() * 0.5;
        let titre = "* GENERATEUR DE SYSTEMES *";
        let tw = crate::police::mesure(titre, 36);
        crate::police::texte(titre, cx - tw * 0.5, screen_height() * 0.12, 36.0, Color::new(0.0, 0.9, 0.9, 1.0));

        let bw = 380.0;
        let ecart = 48.0;
        let y0 = screen_height() * 0.24;
        let xg = cx - bw - ecart * 0.5;
        let xd = cx + ecart * 0.5;

        let choix = Self::bloc("[ ASTRES & GALERIES ]", BLOC_ASTRES, xg, y0, m, clic)
            .or_else(|| Self::bloc("[ VAISSEAUX & STATIONS ]", BLOC_VAISSEAUX, xd, y0, m, clic));

        choix.copied()
    }
}
