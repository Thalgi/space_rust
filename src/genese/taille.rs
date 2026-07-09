//! Échelle de taille des planètes — **source unique de vérité** pour le rayon
//! visuel, partagée par toutes les vues (galerie, vue objet, systèmes générés).
//!
//! On part d'un rayon exprimé en rayons terrestres (R⊕) réaliste par classe, puis
//! on le **compresse en racine carrée** : une géante gazeuse (~11 R⊕) ne fait
//! ainsi que ~3× une tellurique à l'écran au lieu de 11×. On garde l'ordre correct
//!   géante gazeuse > géante de glace > sous-Neptune > super-Terre > tellurique
//!   > monde de glace > naine > lune
//! sans que les géantes n'écrasent visuellement les petits corps voisins (lunes).
//!
//! Calibrage : `rayon_visuel(1.0) = 0.6` (une Terre ≈ 0.6 unité de jeu), ce qui
//! reproduit les valeurs peintes à la main des presets canon (Polyphemus ≈ 1.7
//! pour ~10 R⊕, géante de glace ≈ 1.2 pour ~4 R⊕).

use macroquad::rand::gen_range;

/// Classe de taille d'un corps. Chaque classe couvre une plage de rayon réel
/// (R⊕) ; le rayon de jeu en découle par [`rayon_visuel`].
#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(dead_code)] // certaines classes ne servent encore que via les presets
pub enum ClasseTaille {
    Lune,        // satellite : 0.20–0.45 R⊕ (Lune 0.27, Titan 0.40)
    Naine,       // planète naine / Mercure-Mars : 0.38–0.62 R⊕
    Tellurique,  // Terre / Vénus : 0.85–1.15 R⊕
    SuperTerre,  // super-habitable / Gaia / super-Terre : 1.30–1.90 R⊕
    Glacee,      // monde glacé : 0.50–1.00 R⊕
    SousNeptune, // mini-Neptune : 1.80–3.00 R⊕
    GeanteGlace, // Neptune / Uranus : 3.60–4.30 R⊕
    GeanteGaz,   // Saturne / Jupiter : 8.00–12.00 R⊕
}

impl ClasseTaille {
    /// Bornes (min, max) du rayon réel de la classe, en rayons terrestres.
    fn bornes_terrestres(self) -> (f32, f32) {
        match self {
            ClasseTaille::Lune => (0.20, 0.45),
            ClasseTaille::Naine => (0.38, 0.62),
            ClasseTaille::Tellurique => (0.85, 1.15),
            ClasseTaille::SuperTerre => (1.30, 1.90),
            ClasseTaille::Glacee => (0.50, 1.00),
            ClasseTaille::SousNeptune => (1.80, 3.00),
            ClasseTaille::GeanteGlace => (3.60, 4.30),
            ClasseTaille::GeanteGaz => (8.00, 12.00),
        }
    }

    /// Rayon visuel **médian** (unités de jeu) — déterministe. À utiliser pour les
    /// presets nommés qui doivent garder une taille stable d'une graine à l'autre.
    #[allow(dead_code)]
    pub fn rayon_median(self) -> f32 {
        let (lo, hi) = self.bornes_terrestres();
        rayon_visuel((lo + hi) * 0.5)
    }

    /// Rayon visuel **tiré au hasard** dans la plage de la classe (génération
    /// procédurale : donne de la variété de taille au sein d'une même classe).
    pub fn rayon_aleatoire(self) -> f32 {
        let (lo, hi) = self.bornes_terrestres();
        rayon_visuel(gen_range(lo, hi))
    }
}

/// Compression racine du rayon réel (R⊕) vers les unités de jeu.
/// Terre (1 R⊕) → 0.60 ; Jupiter (~11) → ~1.99 ; Lune (~0.27) → ~0.31.
pub fn rayon_visuel(rayon_terrestre: f32) -> f32 {
    0.6 * rayon_terrestre.max(0.0).sqrt()
}
