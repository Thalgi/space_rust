use crate::ui::minitel_ligne;
use macroquad::prelude::*;

/// Destination choisie depuis l'accueil.
pub enum Cible {
    Skymap,
    Objet,
    Galerie,
    GalerieGaz,
    GalerieEtoiles,
}

/// Écran d'accueil : titre + boutons de mode.
pub struct Accueil;

impl Accueil {
    pub fn new() -> Self {
        Self
    }

    /// Dessine l'accueil et renvoie la destination si un bouton est cliqué.
    pub fn frame(&mut self) -> Option<Cible> {
        clear_background(Color::new(0.01, 0.01, 0.04, 1.0));
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);

        let cx = screen_width() * 0.5;
        let titre = "* GENERATEUR DE SYSTEMES *";
        let tw = measure_text(titre, None, 36, 1.0).width;
        draw_text(titre, cx - tw * 0.5, screen_height() * 0.28, 36.0, Color::new(0.0, 0.9, 0.9, 1.0));

        let bw = 340.0;
        let bh = 44.0;
        let gap = 14.0;
        let y0 = screen_height() * 0.4;
        let b1 = Rect::new(cx - bw * 0.5, y0, bw, bh);
        let b2 = Rect::new(cx - bw * 0.5, y0 + (bh + gap), bw, bh);
        let b3 = Rect::new(cx - bw * 0.5, y0 + 2.0 * (bh + gap), bw, bh);
        let b4 = Rect::new(cx - bw * 0.5, y0 + 3.0 * (bh + gap), bw, bh);
        let b5 = Rect::new(cx - bw * 0.5, y0 + 4.0 * (bh + gap), bw, bh);
        minitel_ligne(b1, "SKYMAP - SYSTEME COMPLET", m);
        minitel_ligne(b2, "OBJET CELESTE - VUE ISOLEE", m);
        minitel_ligne(b3, "GALERIE - TYPES TELLURIQUES", m);
        minitel_ligne(b4, "GALERIE - GEANTES GAZEUSES", m);
        minitel_ligne(b5, "GALERIE - ETOILES", m);

        if clic {
            if b1.contains(m) {
                return Some(Cible::Skymap);
            }
            if b2.contains(m) {
                return Some(Cible::Objet);
            }
            if b3.contains(m) {
                return Some(Cible::Galerie);
            }
            if b4.contains(m) {
                return Some(Cible::GalerieGaz);
            }
            if b5.contains(m) {
                return Some(Cible::GalerieEtoiles);
            }
        }
        None
    }
}
