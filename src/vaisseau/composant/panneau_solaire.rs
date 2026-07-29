//! **Panneaux solaires** : cinq variantes de pale, toutes montées par un port
//! `Surface` et déployées le long de +Z.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::pieces;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::PI;

use super::commun::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VariantePanneau {
    /// Ambre rigide, deux lés (type ISS SAW).
    RigideUS,
    /// Bleu, plus court (segment russe).
    RusseBleu,
    /// iROSA : bande étroite et sombre (déroulable).
    RollOut,
    /// Cyan, futuriste.
    Futuriste,
    /// Tuiles hexagonales légèrement espacées.
    Hexagonal,
}

impl VariantePanneau {
    pub const TOUS: [VariantePanneau; 5] = [
        VariantePanneau::RigideUS,
        VariantePanneau::RusseBleu,
        VariantePanneau::RollOut,
        VariantePanneau::Futuriste,
        VariantePanneau::Hexagonal,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VariantePanneau::RigideUS => "RIGIDE US",
            VariantePanneau::RusseBleu => "RUSSE BLEU",
            VariantePanneau::RollOut => "ROLL-OUT (iROSA)",
            VariantePanneau::Futuriste => "FUTURISTE",
            VariantePanneau::Hexagonal => "HEXAGONAL",
        }
    }

    /// `(couleur, facteur longueur, facteur largeur)` — pour varier l'allure au-delà
    /// de la seule couleur.
    pub(super) fn style(self) -> (Color, f32, f32) {
        match self {
            VariantePanneau::RigideUS => (Color::new(0.50, 0.38, 0.16, 1.0), 1.0, 1.0),
            VariantePanneau::RusseBleu => (Color::new(0.12, 0.20, 0.48, 1.0), 0.7, 1.0),
            VariantePanneau::RollOut => (Color::new(0.10, 0.12, 0.18, 1.0), 1.25, 0.5),
            VariantePanneau::Futuriste => (Color::new(0.10, 0.45, 0.50, 1.0), 1.0, 1.1),
            VariantePanneau::Hexagonal => (Color::new(0.22, 0.24, 0.44, 1.0), 1.0, 1.0),
        }
    }
}

/// Un unique port de montage `Surface` (avant −Z, vers l'hôte) ; la pale se
/// déploie de l'autre côté.
pub(super) fn ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
    )]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, variante: VariantePanneau, longueur: f32, largeur: f32) {
        // Jonction côté hôte (−Z) : un bras qui rejoint l'hôte + un petit
        // socle (gimbal) à l'origine — c'est la liaison module ↔ panneau.
        p.cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
        p.cube(Vec3::ZERO, Vec3::splat(0.22), COULEUR);
        p.cube_fil(Vec3::ZERO, Vec3::splat(0.22), SOMBRE);
        // Mât depuis le socle (origine) vers +Z, puis la pale selon la variante.
        let pied = vec3(0.0, 0.0, MAST_PANNEAU);
        p.cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
        let (col, lf, wf) = variante.style();
        let (lon, lar) = (longueur * lf, largeur * wf);
        match variante {
            VariantePanneau::Hexagonal => {
                pieces::pale_hexagonale(p, pied, Vec3::Z, Vec3::X, lon, lar, col);
            }
            _ => {
                let cellules = (lon / 0.35).max(2.0) as usize;
                pieces::pale_solaire(p, pied, Vec3::Z, Vec3::X, lon, lar, cellules, col);
            }
        }
}

pub(super) fn cout() -> f32 {
    6.0
}

/// Diagonale mât + déploiement contre demi-largeur (borne haute avec le
/// facteur de longueur max des variantes, ~1,25).
pub(super) fn rayon_local(longueur: f32, largeur: f32) -> f32 {
    (MAST_PANNEAU + longueur * 1.25).hypot(largeur * 0.5)
}
