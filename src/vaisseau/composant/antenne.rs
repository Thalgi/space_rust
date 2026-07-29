//! **Antennes et paraboles** : six variantes, toutes montées par un port
//! `Surface` et pointant vers +Z.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

use super::commun::*;

/// Variantes d'antenne / parabole, montées par un port `Surface`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VarianteAntenne {
    /// Parabole grand gain, face vers +Z.
    ParaboleGG,
    /// Parabole à alimentation décalée (offset), inclinée.
    ParaboleOffset,
    /// Grappe de cornets (horns).
    Cornets,
    /// Fouets omnidirectionnels croisés.
    Fouet,
    /// Réseau phasé : plaque plate quadrillée.
    ReseauPhase,
    /// Antenne hélicoïdale.
    Helice,
}

impl VarianteAntenne {
    pub const TOUS: [VarianteAntenne; 6] = [
        VarianteAntenne::ParaboleGG,
        VarianteAntenne::ParaboleOffset,
        VarianteAntenne::Cornets,
        VarianteAntenne::Fouet,
        VarianteAntenne::ReseauPhase,
        VarianteAntenne::Helice,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteAntenne::ParaboleGG => "PARABOLE GRAND GAIN",
            VarianteAntenne::ParaboleOffset => "PARABOLE OFFSET",
            VarianteAntenne::Cornets => "CORNETS",
            VarianteAntenne::Fouet => "FOUETS",
            VarianteAntenne::ReseauPhase => "RESEAU PHASE",
            VarianteAntenne::Helice => "HELICE",
        }
    }

    pub(super) fn cout(self) -> f32 {
        match self {
            VarianteAntenne::Cornets | VarianteAntenne::Helice => 4.0,
            _ => 3.0,
        }
    }

    /// Dessine l'antenne depuis `pied`, pointant vers +Z.
    fn dessiner<P: Peintre>(self, p: &mut P, pied: Vec3, taille: f32) {
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        let sombre = Color::new(0.30, 0.32, 0.36, 1.0);
        let d = Vec3::Z;
        let w = Vec3::X;
        let up = Vec3::Y;
        match self {
            VarianteAntenne::ParaboleGG => p.parabole(pied, d, taille, clair),
            VarianteAntenne::ParaboleOffset => {
                let dir = (d + up * 0.45).normalize();
                p.parabole(pied, dir, taille * 0.9, clair);
            }
            VarianteAntenne::Cornets => {
                for (dx, dy) in [(-0.25_f32, 0.0_f32), (0.25, 0.0), (0.0, 0.28)] {
                    let base = pied + w * (dx * taille) + up * (dy * taille);
                    p.cone(base, d, taille * 0.06, taille * 0.28, taille * 0.7, clair);
                }
            }
            VarianteAntenne::Fouet => {
                let n = 4;
                for i in 0..n {
                    let a = TAU * i as f32 / n as f32;
                    let dir = (d + w * (0.4 * a.cos()) + up * (0.4 * a.sin())).normalize();
                    p.cylindre(pied, pied + dir * (taille * 1.5), 0.02, clair);
                    p.sphere(pied + dir * (taille * 1.5), 0.04, clair);
                }
            }
            VarianteAntenne::ReseauPhase => {
                let s = taille * 0.9;
                let coin = pied - w * (s * 0.5) - up * (s * 0.5);
                p.panneau(coin, w * s, up * s, sombre);
                let n = 5;
                for i in 1..n {
                    let f = i as f32 / n as f32;
                    p.ligne(coin + w * (s * f), coin + w * (s * f) + up * s, TRAIT_FIN, clair);
                    p.ligne(coin + up * (s * f), coin + up * (s * f) + w * s, TRAIT_FIN, clair);
                }
            }
            VarianteAntenne::Helice => {
                let tours = 4.0;
                let n = 40;
                let ray = taille * 0.22;
                let haut = taille * 1.3;
                let point = |t: f32| {
                    let a = TAU * tours * t;
                    pied + w * (ray * a.cos()) + up * (ray * a.sin()) + d * (haut * t)
                };
                for i in 0..n {
                    p.cylindre(point(i as f32 / n as f32), point((i + 1) as f32 / n as f32), 0.025, clair);
                }
                p.cylindre(pied - d * 0.02, pied + d * 0.02, ray * 1.4, sombre); // réflecteur
            }
        }
    }
}

/// Un unique port de montage `Surface` (avant −Z, vers l'hôte) ; l'antenne se
/// déploie de l'autre côté.
pub(super) fn ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
    )]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, variante: VarianteAntenne, taille: f32) {
        // Jonction hôte (bras + socle) puis mât court, puis l'antenne.
        p.cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
        p.cube(Vec3::ZERO, Vec3::splat(0.2), COULEUR);
        p.cube_fil(Vec3::ZERO, Vec3::splat(0.2), SOMBRE);
        let pied = vec3(0.0, 0.0, MAST_PANNEAU);
        p.cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
        variante.dessiner(p, pied, taille);
}

/// Mât + taille (les fouets et l'hélice dépassent un peu).
pub(super) fn rayon_local(taille: f32) -> f32 {
    MAST_PANNEAU + taille * 1.5
}
