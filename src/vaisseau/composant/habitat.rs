//! **Habitat principal** de l'ISV : l'habitat *fixe*, solidaire de l'épine —
//! à ne pas confondre avec les modules d'équipage **rotatifs**, qui sont une
//! autre brique.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::pieces;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::PI;

use super::commun::*;

/// Deux écoutilles axiales, comme tout fût pressurisé.
pub(super) fn ports(profil: Profil, longueur: f32) -> Vec<Port> {
    let demi = longueur * 0.5;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, profil: Profil, longueur: f32, spin: f32, attache: f32) {
        let r = profil.rayon();
        let demi = longueur * 0.5;

        // Fût nu : la section onigiri des nacelles, en plus gros. Rien
        // d'autre sur la coque — pas de collerette de bout, pas de rail
        // d'arête : le composite reste franc, les seules pièces
        // rapportées sont les armatures ci-dessous.
        pieces::prisme_onigiri(p, -Vec3::Z * demi, longueur, r, spin, COMPOSITE);

        // **Armatures hexagonales** au quart et aux trois quarts.
        // Hexagone et non triangle : le côté plat d'un triangle **congé**
        // est repoussé à `r·(0,5 + f/2)`, alors qu'une corde d'un coin au
        // suivant passe à `r/2` — une armature triangulaire s'enfonce
        // donc **dans** la coque à mi-côté.
        //
        // L'échelle n'est pas choisie à l'œil : `onigiri_hex_echelle_mini`
        // donne le facteur en dessous duquel un segment (le court, en
        // travers d'un coin, est le plus exigeant) replonge sous la
        // coque. On prend cette borne, plus une marge franche.
        let quarts = [0.25_f32, 0.75];
        let ra = r * pieces::onigiri_hex_echelle_mini() * 1.04;
        let ep = r * 0.055;
        for t in quarts {
            let z = -demi + longueur * t;
            let h = pieces::onigiri_hexagone(ra, spin, z);
            for k in 0..6 {
                p.cylindre(h[k], h[(k + 1) % 6], ep, HABITAT_ARMATURE);
            }
        }

        // **Bande de repérage jaune** à mi-longueur, là où était la
        // troisième armature : un simple bandeau plaqué sur la coque
        // (même section, à peine au large) plutôt qu'un cadre en relief.
        let bande = longueur * 0.12;
        pieces::prisme_onigiri(
            p,
            -Vec3::Z * (bande * 0.5),
            bande,
            r * 1.02,
            spin,
            HABITAT_BANDE,
        );

        // **Ferrures d'attache**, sur un seul côté plat : celui dont la
        // normale sortante regarde `spin + 180°`. **Deux** ferrures
        // écartées plutôt qu'une seule centrale — deux appuis courts
        // tiennent mieux qu'un long bras isolé — chacune à **mi-portée**
        // de l'ancienne. Les jambes partent des stations d'armature :
        // l'effort passe dans les cadres, pas dans la coque nue.
        if attache > 1e-4 {
            let u = vec3((spin + PI).cos(), (spin + PI).sin(), 0.0);
            let w = vec3(-u.y, u.x, 0.0); // le long de la face
            let base = pieces::onigiri_inscrit(r);
            let lat = pieces::onigiri_demi_face(r) * 0.5;
            let z0 = -demi + longueur * quarts[0];
            let z1 = -demi + longueur * quarts[quarts.len() - 1];
            for s in [-1.0_f32, 1.0] {
                let pied = u * base + w * (s * lat);
                let rail = u * (base + attache * 0.5) + w * (s * lat);
                p.cylindre(rail + Vec3::Z * z0, rail + Vec3::Z * z1, ep * 1.1, HABITAT_ARMATURE);
                for t in quarts {
                    let z = Vec3::Z * (-demi + longueur * t);
                    p.cylindre(pied + z, rail + z, ep, HABITAT_ARMATURE);
                }
            }
        }
}

pub(super) fn cout() -> f32 {
    16.0
}

pub(super) fn rayon_local(profil: Profil, longueur: f32, attache: f32) -> f32 {
    (longueur * 0.5)
        .max(pieces::onigiri_inscrit(profil.rayon()) + attache)
        .max(profil.rayon() * 1.05)
}

/// Demi-section transversale du fût, ferrures comprises.
pub(super) fn demi_section(profil: Profil, attache: f32) -> f32 {
    (pieces::onigiri_inscrit(profil.rayon()) + attache).max(profil.rayon() * 1.05)
}
