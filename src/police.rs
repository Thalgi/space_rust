//! Police Minitel partagée par tout le projet.
//!
//! La police (fichier TTF/OTF) est déposée dans `assets/` (voir `assets/README.md`).
//! Elle est chargée une seule fois au démarrage via [`charger`], stockée en
//! `thread_local` (macroquad tourne sur un seul thread), puis utilisée partout
//! via [`texte`] et [`mesure`]. Si le fichier est absent, on retombe
//! silencieusement sur la police par défaut de macroquad.

use macroquad::prelude::*;
use std::cell::RefCell;

thread_local! {
    static POLICE: RefCell<Option<Font>> = const { RefCell::new(None) };
}

/// Facteur d'échelle appliqué à *toutes* les tailles de texte. La police Minitel
/// a une hampe bien plus grande que la police macroquad par défaut à taille px
/// égale : on compense ici, en un seul endroit. Baisser pour un texte plus petit.
const ECHELLE: f32 = 0.6;

/// Chemins essayés pour trouver la police Minitel (le premier qui charge gagne).
const CANDIDATS: &[&str] = &[
    "assets/minitel.ttf",
    "assets/minitel.otf",
    "assets/Minitel.ttf",
    "assets/Minitel.otf",
];

/// Charge la police Minitel depuis `assets/`. À appeler une fois au démarrage,
/// avant la boucle de rendu.
pub async fn charger() {
    for chemin in CANDIDATS {
        if let Ok(font) = load_ttf_font(chemin).await {
            POLICE.with(|p| *p.borrow_mut() = Some(font));
            return;
        }
    }
    warn!(
        "Police Minitel introuvable dans assets/ (essayé: {:?}) — repli sur la police par défaut.",
        CANDIDATS
    );
}

/// Dessine du texte avec la police Minitel (repli automatique si absente).
pub fn texte(t: &str, x: f32, y: f32, taille: f32, couleur: Color) {
    POLICE.with(|p| {
        let ref_p = p.borrow();
        let params = TextParams {
            font: ref_p.as_ref(),
            font_size: (taille * ECHELLE).round().max(1.0) as u16,
            color: couleur,
            ..Default::default()
        };
        draw_text_ex(t, x, y, params);
    });
}

/// Largeur (en pixels) d'un texte rendu avec la police Minitel.
pub fn mesure(t: &str, taille: u16) -> f32 {
    let taille = ((taille as f32 * ECHELLE).round().max(1.0)) as u16;
    POLICE.with(|p| measure_text(t, p.borrow().as_ref(), taille, 1.0).width)
}
