//! **Pièces de raccord et d'embout** : l'adaptateur tronconique qui relie deux
//! profils (ou sert de nez de docking), et la coiffe qui ferme le nez d'un
//! module.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::Enveloppe;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

use super::commun::*;

/// Formes de [`Composant::Coiffe`].
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VarianteCoiffe {
    /// Demi-dôme lisse (calotte surbaissée fermée), rien ne dépasse sous la base.
    Bombee,
    /// Fût hexagonal légèrement tronconique **fermé par une face hexagonale
    /// plate** (pas de pointe).
    Hexagonale,
    /// Adaptateur d'amarrage : jupe tronconique + anneau d'accostage (type
    /// APAS/IDA) + trappe centrale fermée.
    Amarrage,
}

impl VarianteCoiffe {
    pub const TOUS: [VarianteCoiffe; 3] =
        [VarianteCoiffe::Bombee, VarianteCoiffe::Hexagonale, VarianteCoiffe::Amarrage];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteCoiffe::Bombee => "BOMBEE (DEMI-DOME)",
            VarianteCoiffe::Hexagonale => "HEXAGONALE (FACE PLATE)",
            VarianteCoiffe::Amarrage => "ADAPTATEUR D'AMARRAGE",
        }
    }
}

// --- Adaptateur ------------------------------------------------------------

pub(super) fn ports(grand: Profil, petit: Profil, longueur: f32) -> Vec<Port> {
    // Deux écoutilles axiales de profils différents, au bout des cols.
    let demi = longueur * 0.5;
    vec![
        Port::new(
            Repere::new(vec3(0.0, 0.0, -(demi + grand.rayon() * COL_LONG)), Quat::from_rotation_y(PI)),
            GenrePort::ModuleAxial,
            grand,
        ),
        Port::new(
            Repere::new(vec3(0.0, 0.0, demi + petit.rayon() * COL_LONG), Quat::IDENTITY),
            GenrePort::ModuleAxial,
            petit,
        ),
    ]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, grand: Profil, petit: Profil, longueur: f32) {
    let demi = longueur * 0.5;
    // Tronc de cône grand (−Z) → petit (+Z) + collerettes de docking
    // terminées par une bague d'accostage alu clair.
    p.cone(vec3(0.0, 0.0, -demi), Vec3::Z, grand.rayon(), petit.rayon(), longueur, COULEUR);
    for (s, prof) in [(-1.0_f32, grand), (1.0_f32, petit)] {
        let (lc, rc) = (prof.rayon() * COL_LONG, prof.rayon() * COL_RAYON);
        let bout = s * (demi + lc);
        p.cylindre(vec3(0.0, 0.0, s * demi), vec3(0.0, 0.0, bout), rc, SOMBRE);
        p.cylindre(vec3(0.0, 0.0, bout - s * lc * 0.28), vec3(0.0, 0.0, bout), rc * 1.10, BAGUE);
    }
}

pub(super) fn cout() -> f32 {
    3.0
}

/// Jusqu'au bout du col du grand côté.
pub(super) fn rayon_local(grand: Profil, longueur: f32) -> f32 {
    (longueur * 0.5 + grand.rayon() * COL_LONG).max(grand.rayon())
}

// --- Coiffe ----------------------------------------------------------------

pub(super) fn coiffe_ports(profil: Profil) -> Vec<Port> {
    // Base en Z=0, nez vers +Z : l'écoutille de montage regarde le
    // module (avant −Z) et se clipse sur son écoutille axiale.
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::ModuleAxial,
        profil,
    )]
}

pub(super) fn coiffe_dessiner<P: Peintre>(p: &mut P, profil: Profil, variante: VarianteCoiffe) {
    let r = profil.rayon();
    let metal = Color::new(0.62, 0.64, 0.68, 1.0);
    let clair = Color::new(0.80, 0.82, 0.86, 1.0);
    let sombre = Color::new(0.20, 0.22, 0.26, 1.0);
    // Émet un triangle **des deux côtés** (les triangles bruts sont
    // mono-face et macroquad ne double-face pas).
    fn dbl(idx: &mut Vec<u16>, a: u16, b: u16, c: u16) {
        idx.extend_from_slice(&[a, b, c, a, c, b]);
    }
    // Collier de base commun : couvre l'embout du module (1,02·r) pour
    // qu'aucun rebord ne dépasse au raccord.
    p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, r * 0.06), r * 1.03, metal);
    match variante {
        VarianteCoiffe::Bombee => {
            // **Demi-dôme** = calotte lisse fermée, base en Z=0 (rien ne
            // dépasse sous le module). Grille latitude × longitude d'un
            // demi-ellipsoïde (hauteur surbaissée), triangulée double-face.
            let seg = 18usize; // segments autour
            let rings = 6usize; // anneaux de l'équateur au sommet
            let h = r * 0.5; // hauteur du bombé (surbaissé)
            let mut verts: Vec<Vec3> = Vec::new();
            for i in 0..rings {
                let u = (i as f32 / rings as f32) * (PI * 0.5); // 0..≈π/2
                let (su, cu) = u.sin_cos();
                for j in 0..seg {
                    let a = TAU * j as f32 / seg as f32;
                    verts.push(vec3(r * cu * a.cos(), r * cu * a.sin(), h * su));
                }
            }
            let apex = verts.len() as u16;
            verts.push(vec3(0.0, 0.0, h));
            let mut idx: Vec<u16> = Vec::new();
            for i in 0..rings - 1 {
                for j in 0..seg {
                    let j2 = (j + 1) % seg;
                    let a = (i * seg + j) as u16;
                    let b = (i * seg + j2) as u16;
                    let c = ((i + 1) * seg + j) as u16;
                    let d = ((i + 1) * seg + j2) as u16;
                    dbl(&mut idx, a, b, d);
                    dbl(&mut idx, a, d, c);
                }
            }
            let last = (rings - 1) * seg;
            for j in 0..seg {
                let j2 = (j + 1) % seg;
                dbl(&mut idx, (last + j) as u16, (last + j2) as u16, apex);
            }
            p.triangles(&verts, &idx, clair);
        }
        VarianteCoiffe::Hexagonale => {
            // Fût hexagonal légèrement tronconique **fermé par une face
            // hexagonale plate** (pas de pointe).
            let n = 6usize;
            let hp = r * 0.70; // hauteur
            let rt = r * 0.74; // rayon du dessus (léger fruit)
            let ring = |rad: f32, z: f32| -> Vec<Vec3> {
                (0..n)
                    .map(|k| {
                        let a = TAU * k as f32 / n as f32;
                        vec3(rad * a.cos(), rad * a.sin(), z)
                    })
                    .collect()
            };
            let bas = ring(r, 0.0);
            let haut = ring(rt, hp);
            let mut verts: Vec<Vec3> = bas;
            verts.extend(haut.iter().copied()); // indices n..2n
            // Parois (trapèzes → 2 triangles), double-face.
            let mut idx: Vec<u16> = Vec::new();
            for k in 0..n {
                let k2 = (k + 1) % n;
                let (b0, b1, t0, t1) = (k as u16, k2 as u16, (n + k) as u16, (n + k2) as u16);
                dbl(&mut idx, b0, b1, t1);
                dbl(&mut idx, b0, t1, t0);
            }
            // Face plate hexagonale du dessus (éventail depuis le centre).
            let ctr = verts.len() as u16;
            verts.push(vec3(0.0, 0.0, hp));
            for k in 0..n {
                let k2 = (k + 1) % n;
                dbl(&mut idx, (n + k) as u16, (n + k2) as u16, ctr);
            }
            p.triangles(&verts, &idx, metal);
        }
        VarianteCoiffe::Amarrage => {
            // Jupe tronconique + anneau d'amarrage (APAS/IDA) + trappe
            // centrale fermée + petites gâches de verrouillage.
            let zc = r * 0.55; // haut de la jupe
            p.cone(Vec3::ZERO, Vec3::Z, r, r * 0.55, zc, metal); // jupe
            // Col d'accostage + bague claire (visuel androgyne APAS).
            p.cylindre(vec3(0.0, 0.0, zc), vec3(0.0, 0.0, zc + r * 0.14), r * 0.55, sombre);
            p.cylindre(vec3(0.0, 0.0, zc + r * 0.10), vec3(0.0, 0.0, zc + r * 0.14), r * 0.60, clair);
            // Trappe centrale fermée (la coiffe **obture** le module).
            p.cylindre(vec3(0.0, 0.0, zc + r * 0.14), vec3(0.0, 0.0, zc + r * 0.18), r * 0.5, metal);
            // 4 gâches de verrouillage réparties sur le col.
            for k in 0..4 {
                let a = TAU * k as f32 / 4.0;
                let c = vec3(r * 0.55 * a.cos(), r * 0.55 * a.sin(), zc + r * 0.07);
                p.cube(c, vec3(r * 0.10, r * 0.10, r * 0.14), clair);
            }
        }
    }
}

pub(super) fn coiffe_cout() -> f32 {
    6.0
}

/// Nez déployé jusqu'à ~1,4 × rayon vers +Z.
pub(super) fn coiffe_rayon_local(profil: Profil) -> f32 {
    profil.rayon() * 1.4
}

/// Nez déployé vers +Z : sphère décalée à mi-hauteur.
pub(super) fn coiffe_englobant(profil: Profil) -> Enveloppe {
    let r = profil.rayon();
    Enveloppe::sphere(Vec3::Z * (r * 0.6), r * 0.95)
}
