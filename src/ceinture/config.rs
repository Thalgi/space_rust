use macroquad::prelude::*;

/// Réglages d'une ceinture. Deux presets : `asteroides` (rocheuse, fine, petits
/// corps gris) et `kuiper` (glacée, épaisse, dispersée, quelques gros corps).
pub struct CeintureConfig {
    pub(super) nb: usize,
    pub(super) interne: f32,    // rayon interne (unités monde)
    pub(super) externe: f32,    // rayon externe (unités monde)
    pub(super) masse: f32,      // masse de l'étoile (vitesse orbitale)
    pub(super) taille_min: f32, // taille visuelle mini
    pub(super) taille_max: f32, // taille visuelle maxi (gros rares)
    pub(super) epaisseur: f32,  // dispersion verticale (inclinaison max)
    pub(super) couleur: Vec3,   // teinte de base
}

impl CeintureConfig {
    /// Ceinture principale d'astéroïdes : rocheuse, fine, corps gris majoritairement petits.
    pub fn asteroides(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            nb,
            interne,
            externe,
            masse,
            taille_min: 0.04,
            taille_max: 0.20,
            epaisseur: 0.05,
            couleur: vec3(0.55, 0.5, 0.45),
        }
    }

    /// Ceinture de Kuiper : glacée, plus épaisse et dispersée, quelques gros corps.
    pub fn kuiper(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            nb,
            interne,
            externe,
            masse,
            taille_min: 0.06,
            taille_max: 0.5,
            epaisseur: 0.28,
            couleur: vec3(0.6, 0.66, 0.78),
        }
    }
}
