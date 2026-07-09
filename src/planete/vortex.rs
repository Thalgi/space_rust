//! Slots de vortex des géantes gazeuses (V2 phase 4, CONCEPTION_GAZEUSES_V2.md § 5).
//!
//! Un seul système pour la grande tache, les taches sombres, les ovales
//! blancs, les barges brunes et les chapelets de perles : 8 slots générés ici
//! (CPU, déterministe par seed) et poussés en uniforms `vortex[8]`/`vortex2[8]`.
//! Le shader tord le fond autour de chaque slot (aspiration) et rend le slot
//! dominant au pixel — plus aucun « seuil de fbm » ni autocollant.
//!
//! Encodage (miroir du .glsl) :
//! - `vortex[i]`  : xyz = direction du centre, w = type + rayon angulaire
//!   (type = floor : 0 GRS, 1 sombre, 2 ovale blanc, 3 barge, 4 chapelet ;
//!   inactif si fract(w) ≈ 0).
//! - `vortex2[i]` : x = dérive (u normalisé du jet local — le vortex RIDE son
//!   jet), y = spin (±1), z = index du slot (graine locale), w = libre.

use super::apparence::Apparence;
use super::zonal;
use macroquad::prelude::*;
use std::f32::consts::TAU;

pub const N_VORTEX: usize = 8;

/// Hash déterministe 0..1 (stable, indépendant du RNG global).
fn h01(seed: f32, i: f32) -> f32 {
    let v = ((seed * 12.9898 + i * 78.233).sin()) * 43758.5453_f32;
    v - v.floor()
}

/// Spin visuel : anticyclones (GRS, sombres, ovales, perles) vs cyclones
/// (barges), inversé par hémisphère (Coriolis).
fn spin_de(ty: f32, sphi: f32) -> f32 {
    let hemi = if sphi >= 0.0 { 1.0 } else { -1.0 };
    let cycl = if ty == 3.0 { 1.0 } else { -1.0 };
    hemi * cycl
}

/// Génère les 8 slots de vortex d'une gazeuse. Slot 0 = la tache du preset
/// (compatibilité totale) ; slots 1..7 activés par la densité `tempetes`.
pub fn generer_vortex(a: &Apparence) -> ([Vec4; N_VORTEX], [Vec4; N_VORTEX]) {
    let mut v = [Vec4::ZERO; N_VORTEX]; // w = 0 -> slot inactif
    let mut v2 = [Vec4::ZERO; N_VORTEX];
    let prof = zonal::profil(a);
    let k = a.axe.normalize_or_zero();
    // Base tangente autour de l'axe pour poser (latitude, longitude).
    let refv = if k.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let e1 = k.cross(refv).normalize();
    let e2 = k.cross(e1);

    // Slot 0 : la tache du preset (0 = GRS, 1 = sombre, 2 = tête blanche GTB).
    if a.tache_taille > 0.0 {
        let dir = a.tache_dir.normalize_or_zero();
        let ty = if a.tache_type > 1.5 {
            2.0
        } else if a.tache_type > 0.5 {
            1.0
        } else {
            0.0
        };
        let sphi = dir.dot(k);
        v[0] = vec4(dir.x, dir.y, dir.z, ty + a.tache_taille.clamp(0.05, 0.9));
        v2[0] = vec4(prof.u_at(sphi), spin_de(ty, sphi), 0.0, 0.0);
    }

    // Slots 1..7 : densité pilotée par `tempetes`.
    for (i, (vi, vi2)) in v.iter_mut().zip(v2.iter_mut()).enumerate().skip(1) {
        let fi = i as f32;
        if h01(a.seed, fi * 3.3) >= a.tempetes {
            continue;
        }
        // Type : ovales blancs fréquents, barges, sombres, chapelet rare.
        let ht = h01(a.seed, fi * 7.1);
        let ty = if ht < 0.42 {
            2.0
        } else if ht < 0.68 {
            3.0
        } else if ht < 0.85 {
            1.0
        } else {
            4.0
        };
        // Latitude : 6 candidats, on garde celui dont la bande correspond au
        // type (zones claires pour ovales/chapelets, belts pour les sombres).
        let veut_zone = ty == 2.0 || ty == 4.0;
        let (mut meilleur, mut score_max) = (0.3_f32, -1.0_f32);
        for c in 0..6 {
            let sl = (h01(a.seed, fi * 13.7 + c as f32 * 1.9) * 2.0 - 1.0) * 0.8;
            if sl.abs() < 0.12 {
                continue; // pas sur l'équateur
            }
            let bb = prof.b_at(sl);
            let score = if veut_zone { bb } else { 1.0 - bb };
            if score > score_max {
                score_max = score;
                meilleur = sl;
            }
        }
        let lon = h01(a.seed, fi * 5.9) * TAU;
        let cl = (1.0 - meilleur * meilleur).max(0.0).sqrt();
        let dir = (e1 * lon.cos() + e2 * lon.sin()) * cl + k * meilleur;
        let ray = match ty as i32 {
            2 => 0.07 + 0.06 * h01(a.seed, fi * 9.4),  // ovale blanc
            3 => 0.10 + 0.07 * h01(a.seed, fi * 9.4),  // barge
            1 => 0.12 + 0.08 * h01(a.seed, fi * 9.4),  // tache sombre
            _ => 0.16 + 0.10 * h01(a.seed, fi * 9.4),  // chapelet (échelle de l'arc)
        };
        *vi = vec4(dir.x, dir.y, dir.z, ty + ray);
        *vi2 = vec4(prof.u_at(meilleur), spin_de(ty, meilleur), fi, 0.0);
    }
    (v, v2)
}
