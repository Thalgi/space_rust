mod gravite;
mod rendu;

use crate::astre::{Astre, Categorie};
use macroquad::prelude::*;

pub const G: f32 = 1.0; // constante gravitationnelle
const SOUS_PAS: usize = 4; // sous-pas de physique par frame (stabilité)

/// Le système : un ensemble d'astres soumis à la gravité mutuelle.
pub struct Systeme {
    astres: Vec<Box<dyn Astre>>,
    /// Lumière de secours (pos, couleur) utilisée quand il n'y a pas d'étoile
    /// (ex. vue d'une planète isolée). Ignorée dès qu'une étoile est présente.
    lumiere_manuelle: Option<(Vec3, Vec3)>,
}

impl Systeme {
    pub fn new() -> Self {
        Self {
            astres: Vec::new(),
            lumiere_manuelle: None,
        }
    }

    /// Définit une lumière directionnelle de secours (sans étoile dans la scène).
    pub fn set_lumiere(&mut self, pos: Vec3, couleur: Vec3) {
        self.lumiere_manuelle = Some((pos, couleur));
    }

    pub(crate) fn lumiere_secours(&self) -> Option<(Vec3, Vec3)> {
        self.lumiere_manuelle
    }

    /// Ajoute un astre et renvoie son index (utile pour rattacher des lunes).
    pub fn ajouter(&mut self, a: Box<dyn Astre>) -> usize {
        self.astres.push(a);
        self.astres.len() - 1
    }

    /// Position d'un astre par index (origine si invalide).
    pub fn position(&self, idx: usize) -> Vec3 {
        self.astres
            .get(idx)
            .map(|a| a.corps().position)
            .unwrap_or(Vec3::ZERO)
    }

    /// Sélection au rayon : renvoie l'astre touché le plus proche (hors ceinture).
    pub fn pick(&self, origine: Vec3, dir: Vec3) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, a) in self.astres.iter().enumerate() {
            if a.categorie() == Categorie::Asteroide {
                continue;
            }
            let centre = a.corps().position;
            let rayon = a.corps().rayon.max(0.3) * 1.4; // marge pour cliquer facilement
            let oc = centre - origine;
            let tca = oc.dot(dir);
            if tca < 0.0 {
                continue; // derrière la caméra
            }
            let d2 = oc.length_squared() - tca * tca;
            let rr = rayon * rayon;
            if d2 <= rr {
                let t = tca - (rr - d2).sqrt();
                if best.map_or(true, |(bt, _)| t < bt) {
                    best = Some((t, i));
                }
            }
        }
        best.map(|(_, i)| i)
    }

    pub fn update(&mut self, dt: f32) {
        let h = dt / SOUS_PAS as f32;
        for _ in 0..SOUS_PAS {
            self.gravite(h);
        }
        // Animation propre de chaque astre (éruptions du soleil, etc.).
        for a in &mut self.astres {
            a.update(dt);
        }

        // Lunes : orbite analytique autour de leur parent (positions courantes).
        let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
        for a in &mut self.astres {
            if let Some(p) = a.parent() {
                a.orbiter_autour(pos[p], dt);
            }
        }
    }

    /// Transmet les réglages d'éruptions à l'étoile.
    pub fn reglages_etoile(&mut self, freq: f32, forme: f32, puissance: f32, alea: f32) {
        for a in &mut self.astres {
            if a.categorie() == Categorie::Etoile {
                a.set_eruptions(freq, forme, puissance, alea);
            }
        }
    }
}
