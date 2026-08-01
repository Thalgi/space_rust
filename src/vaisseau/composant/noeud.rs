//! **Nœud d'amarrage** : hub sphérique multi-ports, en quatre dispositions.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use super::commun::*;

/// Disposition des ports d'un [`Composant::Noeud`].
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Sorties {
    /// 4 ports : 2 axiaux (±Z) + 2 radiaux (±X) — croix plane.
    Quatre,
    /// 6 ports : 2 axiaux (±Z) + 4 radiaux (±X, ±Y) — croix 3D.
    Six,
    /// 3 ports en T dans le plan XZ : barre ±X + tige −Z (lisible vu de dessous).
    T,
    /// 4 ports vers les sommets d'un tétraèdre régulier (répartition 3D isotrope).
    Tetra,
}

/// Faces d'un nœud : `(direction sortante, rotation du port, genre)`. La rotation
/// oriente l'**avant** du port (`rot*Z`) le long de la direction (sortant).
pub(super) fn faces_noeud(sorties: Sorties) -> Vec<(Vec3, Quat, GenrePort)> {
    let z_plus = (Vec3::Z, Quat::IDENTITY, GenrePort::ModuleAxial);
    let z_moins = (Vec3::NEG_Z, Quat::from_rotation_y(PI), GenrePort::ModuleAxial);
    let x_plus = (Vec3::X, Quat::from_rotation_y(FRAC_PI_2), GenrePort::ModuleRadial);
    let x_moins = (Vec3::NEG_X, Quat::from_rotation_y(-FRAC_PI_2), GenrePort::ModuleRadial);
    let y_plus = (Vec3::Y, Quat::from_rotation_x(-FRAC_PI_2), GenrePort::ModuleRadial);
    let y_moins = (Vec3::NEG_Y, Quat::from_rotation_x(FRAC_PI_2), GenrePort::ModuleRadial);
    match sorties {
        Sorties::Quatre => vec![z_plus, z_moins, x_plus, x_moins],
        Sorties::Six => vec![z_plus, z_moins, x_plus, x_moins, y_plus, y_moins],
        // Barre ±X + tige −Z, tout dans le plan XZ (horizontal).
        Sorties::T => vec![x_plus, x_moins, z_moins],
        // Sommets d'un tétraèdre : rotation générique Z→direction via l'arc.
        Sorties::Tetra => [
            vec3(1.0, 1.0, 1.0),
            vec3(1.0, -1.0, -1.0),
            vec3(-1.0, 1.0, -1.0),
            vec3(-1.0, -1.0, 1.0),
        ]
        .into_iter()
        .map(|d| {
            let dir = d.normalize();
            (dir, Quat::from_rotation_arc(Vec3::Z, dir), GenrePort::ModuleRadial)
        })
        .collect(),
    }
}

pub(super) fn ports(profil: Profil, sorties: Sorties) -> Vec<Port> {
    // Chaque port se pose au bout de sa collerette : sphère + bras + col.
    let t = profil.rayon() * (NOEUD_SPHERE + BRAS_LONG + COL_LONG);
    let faces = faces_noeud(sorties);
    let mut v: Vec<Port> = faces
        .iter()
        .map(|(dir, rot, genre)| Port::new(Repere::new(*dir * t, *rot), *genre, profil))
        .collect();
    // Ports hôtes `Surface` sur les directions principales **libres**
    // (non occupées par un bras) — pour appendices sur le nœud.
    let rs = profil.rayon() * NOEUD_SPHERE;
    for (dir, rot) in faces_principales() {
        if !faces.iter().any(|(d, _, _)| d.dot(dir) > 0.99) {
            v.push(Port::new(Repere::new(dir * rs, rot), GenrePort::Surface, Profil::P0));
        }
    }
    v
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, profil: Profil, sorties: Sorties) {
    let rn = profil.rayon();
    let rs = rn * NOEUD_SPHERE; // sphère gonflée
    let lb = rn * BRAS_LONG;
    let rb = rn * BRAS_RAYON;
    let lc = rn * COL_LONG;
    let rc = rn * COL_RAYON;
    // Base du bras enfoncée dans la sphère (jonction propre, pas tangente).
    let base = rs - rn * JONCTION_OFFSET;
    // Corps sphérique (pas de disque de bout → pas de z-fighting).
    p.sphere(Vec3::ZERO, rs, COULEUR);
    for (dir, _, _) in faces_noeud(sorties) {
        // Bras cylindrique ancré dans la sphère, collerette, puis
        // bague d'accostage alu clair au bout (comme les modules).
        p.cylindre(dir * base, dir * (rs + lb), rb, COULEUR);
        p.cylindre(dir * (rs + lb), dir * (rs + lb + lc), rc, SOMBRE);
        let lbg = lc * 0.28;
        p.cylindre(dir * (rs + lb + lc - lbg), dir * (rs + lb + lc), rc * 1.10, BAGUE);
    }
}

/// Sphère + (bras + collerette) par sortie.
pub(super) fn cout(sorties: Sorties) -> f32 {
    1.0 + 2.0 * faces_noeud(sorties).len() as f32
}

/// Rayon jusqu'au bout des sorties.
pub(super) fn rayon_local(profil: Profil) -> f32 {
    profil.rayon() * (NOEUD_SPHERE + BRAS_LONG + COL_LONG)
}
