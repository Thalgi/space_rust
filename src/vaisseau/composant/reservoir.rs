//! **Réservoir de carburant** : cuve sphérique à tuiles, optionnellement tenue
//! par une cage tétraédrique de quatre barres.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::pieces;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::PI;

pub(super) fn ports(profil: Profil, longueur: f32) -> Vec<Port> {
    let demi = longueur * 0.5;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, longueur: f32, cage: bool) {
        let rs = longueur * 0.5; // rayon de référence de la cage tétraédrique
        let r_cuve = rs * 1.3; // cuve **gonflée de 30 %**
        let corps = Color::new(0.82, 0.84, 0.88, 1.0); // alu clair (tuiles)
        let joint = Color::new(0.30, 0.31, 0.34, 1.0); // sous-couche (joints)
        let metal = Color::new(0.55, 0.57, 0.62, 1.0);
        // Cuve **sphérique** à **tuiles triangulaires** : sous-couche sombre
        // (les joints ressortent) + icosphère facettée par-dessus.
        p.sphere(Vec3::ZERO, r_cuve * 0.97, joint);
        pieces::sphere_triangulee(p, Vec3::ZERO, r_cuve, 3, 0.16, corps);
        if cage {
            // 4 barres métal en **position tétraédrique**, depuis la surface.
            // Tétraèdre **pointe en +Z, face opposée perpendiculaire à Z**
            // (les 3 sommets de base à z = −1/3) : cette face est donc plane
            // et parallèle au plan X‑Y — orientable parallèle à l'hexagone.
            let lb = rs * 1.25; // dépassement des barres hors de la cuve
            let s2 = 2.0_f32.sqrt(); // √2
            let s6 = 6.0_f32.sqrt(); // √6
            let dirs = [
                vec3(0.0, 0.0, 1.0),                       // pointe (+Z)
                vec3(2.0 * s2 / 3.0, 0.0, -1.0 / 3.0),     // base
                vec3(-s2 / 3.0, s6 / 3.0, -1.0 / 3.0),     // base
                vec3(-s2 / 3.0, -s6 / 3.0, -1.0 / 3.0),    // base
            ];
            let mut sommets = [Vec3::ZERO; 4];
            for (i, d) in dirs.iter().enumerate() {
                let dir = d.normalize();
                let bout = dir * (rs + lb);
                p.cylindre(dir * r_cuve, bout, rs * 0.10, metal); // barre radiale
                p.sphere(bout, rs * 0.13, metal); // embout = sommet
                sommets[i] = bout;
            }
            // Relier les 4 sommets deux à deux (6 arêtes) → tétraèdre.
            for i in 0..4 {
                for j in (i + 1)..4 {
                    p.cylindre(sommets[i], sommets[j], rs * 0.08, metal);
                }
            }
        }
}

pub(super) fn cout(longueur: f32) -> f32 {
    8.0 + longueur
}

pub(super) fn rayon_local(profil: Profil, longueur: f32) -> f32 {
    (longueur * 0.5 + profil.rayon()).max(profil.rayon() * 3.5)
}
