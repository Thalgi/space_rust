//! **La magnétosphère des rémanents, en 3D** (lot 13, 2026-10-26) : les
//! faisceaux du pulsar et les boucles du magnétar, posés dans le monde au lieu
//! d'être peints dans le plan de l'écran (`couronne_type` 3 et 4 de
//! `soleil.frag.glsl`, qui restaient verticaux quand on tournait autour).
//!
//! Le champ d'une étoile à neutrons est un dipôle dont l'axe est **incliné**
//! sur l'axe de rotation : il tourne avec elle. Le pulsar émet deux faisceaux
//! le long de cet axe magnétique — un phare. Le magnétar porte un champ mille
//! fois plus fort, dont les lignes fermées s'illuminent.
//!
//! Pur : positions et rayons, relatifs au centre. Le dessin est dans `rendu`.

use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

/// **L'axe magnétique** au temps `phase` (angle de rotation, rad) : incliné de
/// `inclinaison` sur l'axe de rotation `rot` (unitaire), et tournant avec lui.
pub fn axe_magnetique(rot: Vec3, inclinaison: f32, phase: f32) -> Vec3 {
    let u = rot.any_orthonormal_vector();
    let v = rot.cross(u);
    let (si, ci) = inclinaison.sin_cos();
    (rot * ci + (u * phase.cos() + v * phase.sin()) * si).normalize_or(rot)
}

/// **Un faisceau de pulsar** : `(position relative, demi-largeur, éclat)` le
/// long du faisceau, de la surface (`r`) à `longueur` rayons.
///
/// ⚠️ **Il se courbe au loin**, à rebours du sens de rotation : un point du
/// faisceau a quitté l'étoile un peu plus tôt, quand l'axe magnétique pointait
/// un peu en arrière — `retard` radians de rotation au bout du faisceau. C'est
/// le jet d'un tuyau d'arrosage qu'on fait tourner. *« Des pôles qui se
/// courbent un peu par rapport au sens de rotation, à une longue distance de
/// l'astre »* (l'utilisateur, en ouvrant le lot 13).
pub fn faisceau(rot: Vec3, inclinaison: f32, phase: f32, sens: f32, r: f32, longueur: f32, retard: f32) -> Vec<(Vec3, f32, f32)> {
    const N: usize = 48;
    (0..=N)
        .map(|k| {
            let s = k as f32 / N as f32;
            // Le retard croît avec le carré de la distance : droit au pied,
            // courbé au bout.
            let m = axe_magnetique(rot, inclinaison, phase - retard * s * s) * sens;
            let dist = r * (1.05 + longueur * s);
            let demi = r * (0.12 + 0.9 * s);
            (m * dist, demi, (1.0 - s).powf(1.3))
        })
        .collect()
}

/// **Une boucle de champ fermée** du magnétar : une ligne du dipôle d'axe `m`
/// qui sort de la surface (`r`) et y retombe dans l'autre hémisphère, montant
/// jusqu'à `apogee` (> `r`), à l'azimut `phi` autour de `m`. `torsion` la fait
/// tourner autour de l'axe en montant (champ tordu, rad). Une ligne de dipôle
/// suit `ρ = L·sin²θ`.
pub fn boucle(m: Vec3, r: f32, apogee: f32, phi: f32, torsion: f32) -> Vec<Vec3> {
    const N: usize = 40;
    let u = m.any_orthonormal_vector();
    let v = m.cross(u);
    let theta0 = (r / apogee).sqrt().clamp(0.0, 1.0).asin();
    (0..=N)
        .map(|k| {
            let s = k as f32 / N as f32;
            let theta = theta0 + (PI - 2.0 * theta0) * s;
            let rho = apogee * theta.sin().powi(2);
            // Tordue : l'azimut glisse en montant, revient en descendant.
            let az = phi + torsion * (PI * s).sin();
            let e = u * az.cos() + v * az.sin();
            (m * theta.cos() + e * theta.sin()) * rho
        })
        .collect()
}

/// Pseudo-hasard déterministe dans `[0 ; 1[`.
pub(super) fn h(i: f32, s: f32) -> f32 {
    ((i * 12.9898 + s * 37.719).sin() * 43_758.547).fract().abs()
}

/// **Les boucles d'un magnétar** : `n` lignes réparties autour de l'axe,
/// d'apogées inégales (1,6 à 4 rayons), dérivées de `graine`.
pub fn boucles(m: Vec3, r: f32, n: u32, graine: f32, torsion: f32) -> Vec<Vec<Vec3>> {
    (0..n)
        .map(|k| {
            let i = k as f32 + graine * 7.0;
            let apogee = r * (1.6 + 2.4 * h(i, 1.0));
            let phi = TAU * k as f32 / n as f32 + (h(i, 2.0) - 0.5) * 0.6;
            boucle(m, r, apogee, phi, torsion * (0.6 + 0.8 * h(i, 3.0)))
        })
        .collect()
}

/// **Un sursaut de magnétar**, 0..1, au temps `temps` : un éclair bref toutes
/// les `periode` secondes environ, qui retombe vite.
pub fn sursaut(temps: f32, periode: f32, graine: f32) -> f32 {
    let t = (temps / periode + graine).fract();
    (-t * 14.0).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **L'axe magnétique est incliné de ce qu'on lui dit, et tourne autour
    /// de l'axe de rotation.**
    #[test]
    fn l_axe_magnetique_tourne_autour_de_la_rotation() {
        let rot = vec3(0.2, 1.0, -0.3).normalize();
        for k in 0..8 {
            let m = axe_magnetique(rot, 0.5, k as f32 * 0.8);
            assert!((m.dot(rot) - 0.5f32.cos()).abs() < 1e-4, "inclinaison");
        }
        let (a, b) = (axe_magnetique(rot, 0.5, 0.0), axe_magnetique(rot, 0.5, 1.5));
        assert!((a - b).length() > 0.3, "ne tourne pas");
    }

    /// **Un faisceau part droit de la surface et se courbe au loin, à
    /// rebours de la rotation** ; sans retard, il reste droit.
    #[test]
    fn un_faisceau_se_courbe_a_rebours_de_la_rotation() {
        let rot = Vec3::Y;
        let f = faisceau(rot, 0.6, 0.0, 1.0, 1.0, 20.0, 0.8);
        let pied = f[0].0;
        assert!((pied.length() - 1.05).abs() < 1e-3, "ne part pas de la surface");
        let bout = f.last().unwrap().0.normalize();
        let depart = f[0].0.normalize();
        // L'azimut autour de l'axe de rotation : plus petit au bout (en retard).
        let az = |p: Vec3| {
            let u = rot.any_orthonormal_vector();
            p.dot(rot.cross(u)).atan2(p.dot(u))
        };
        let recul = (az(depart) - az(bout)).rem_euclid(TAU);
        assert!(recul > 0.5 && recul < 1.0, "recul de {recul} rad");
        let droit = faisceau(rot, 0.6, 0.0, 1.0, 1.0, 20.0, 0.0);
        assert!(droit.last().unwrap().0.normalize().dot(depart) > 0.9999, "courbé sans retard");
    }

    /// **Une boucle sort de la surface et y retombe**, en montant jusqu'à son
    /// apogée, de part et d'autre de l'équateur magnétique.
    #[test]
    fn une_boucle_part_de_la_surface_et_y_retombe() {
        let m = vec3(0.3, 0.9, 0.1).normalize();
        for torsion in [0.0, 0.8] {
            let b = boucle(m, 1.0, 3.0, 1.0, torsion);
            assert!((b[0].length() - 1.0).abs() < 1e-3 && (b.last().unwrap().length() - 1.0).abs() < 1e-3, "pieds hors de la surface");
            assert!(b[0].dot(m) > 0.0 && b.last().unwrap().dot(m) < 0.0, "un pied par hémisphère");
            let haut = b.iter().map(|p| p.length()).fold(0.0, f32::max);
            assert!((haut - 3.0).abs() < 0.05, "apogée {haut}");
        }
        assert!(sursaut(0.0, 5.0, 0.0) > 0.99 && sursaut(2.5, 5.0, 0.0) < 0.01, "sursaut");
    }
}
