//! **Fret d'échelle vaisseau** (ISV) : le conteneur à section onigiri et le
//! râtelier qui en porte une couronne autour de l'épine.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::Enveloppe;
use crate::vaisseau::pieces;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::commun::*;

/// Disposition des nacelles d'un [`Composant::RatelierCargo`] : renvoie le rayon
/// hors-tout d'une nacelle et, pour chacune, `(position de station, spin)`.
/// **Partagée par le dessin et par le calcul d'encombrement** — les deux ne
/// peuvent donc pas diverger.
///
/// Deux régimes, parce que trois nacelles ne se rangent pas comme six :
/// - **3 → triforce** : trois conteneurs de **même orientation** posés aux
///   sommets d'un triangle, pointe contre pointe, laissant un creux
///   triangulaire au milieu ;
/// - **≥ 4 → couronne** : réparties en angle, **coin vers l'axe**, les côtés
///   plats se faisant face. Le rayon vient du demi-pas angulaire, donc elles ne
///   se croisent jamais quel que soit leur nombre.
///
/// **Écartement de la triforce** (l'erreur à ne pas refaire) : des triangles à
/// coins **vifs** se touchent pointe contre pointe quand la distance au centre
/// vaut leur circonrayon. Nos sections ont des coins **congés** — ce sont donc
/// des triangles nus *gonflés* du rayon de congé ρ, et à cette distance elles
/// s'interpénètrent. Il faut écarter les triangles nus de 2ρ. Leurs pointes
/// s'écartent de `√3·(D − r_nu)` quand on écarte les centres de `D`, d'où
/// `D = r_nu + 2ρ/√3`, soit `D = r·(1 − f + 2f/√3)` avec `f` la fraction de
/// congé. Le facteur `JEU` ajoute par-dessus un vrai jour visible.
pub(super) fn grappe_cargo(rayon: f32, nacelles: usize, nacelle: f32) -> (f32, Vec<(Vec3, f32)>) {
    let n = nacelles.max(3);
    let dir = |a: f32| vec3(a.cos(), a.sin(), 0.0);
    // Rayon imposé (`nacelle > 0`) : la couronne peut alors s'ouvrir sans que le
    // fret grossisse. Sinon, on le déduit pour un empilement serré.
    let impose = nacelle > 1e-4;
    if n == 3 {
        const JEU: f32 = 1.05;
        let f = pieces::ONIGIRI_FILET;
        let ecart = (1.0 - f + 2.0 * f / 3.0_f32.sqrt()) * JEU;
        let places = (0..3)
            .map(|k| {
                let a = FRAC_PI_2 + TAU * k as f32 / 3.0;
                (dir(a) * rayon, FRAC_PI_2)
            })
            .collect();
        (if impose { nacelle } else { rayon / ecart }, places)
    } else {
        let rnac = rayon * (PI / n as f32).sin() * 0.9;
        let places = (0..n)
            .map(|k| {
                let a = TAU * k as f32 / n as f32;
                (dir(a) * rayon, a + PI)
            })
            .collect();
        (if impose { nacelle } else { rnac }, places)
    }
}

// --- Nacelle ---------------------------------------------------------------

/// Un appendice, monté par sa base (avant −Z vers l'hôte), le conteneur se
/// déployant vers +Z.
pub(super) fn nacelle_ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
    )]
}

pub(super) fn nacelle_dessiner<P: Peintre>(p: &mut P, profil: Profil, longueur: f32, spin: f32) {
    pieces::nacelle_cargo(p, Vec3::ZERO, longueur, profil.rayon(), spin, CARGO, SOMBRE);
}

pub(super) fn nacelle_cout() -> f32 {
    6.0
}

pub(super) fn nacelle_rayon_local(profil: Profil, longueur: f32) -> f32 {
    longueur.max(profil.rayon())
}

/// Déployée d'un seul côté (+Z) : **capsule** couchée le long de la nacelle.
///
/// C'est la pièce où le gain se voit le plus, parce qu'elles se posent **par
/// trois en triforce**, jointives : la sphère réservait `hypot(demi, rayon)`
/// dans toutes les directions et faisait donc se recouvrir trois nacelles qui,
/// en vérité, se frôlent sans se toucher (§C.6).
pub(super) fn nacelle_englobant(profil: Profil, longueur: f32) -> Enveloppe {
    let demi = longueur * 0.5;
    Enveloppe::axe(Vec3::Z * demi, Vec3::Z, demi, profil.rayon())
}

// --- Râtelier --------------------------------------------------------------

/// Deux écoutilles axiales, comme une poutre — les rangées se chaînent bout à
/// bout le long de l'épine.
pub(super) fn ratelier_ports(profil: Profil, longueur: f32) -> Vec<Port> {
    let demi = longueur * 0.5;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
    ]
}

/// Les nacelles seules : la cage qui les tenait a été retirée (elle noyait le
/// fret au lieu de le structurer).
pub(super) fn ratelier_dessiner<P: Peintre>(
    p: &mut P,
    longueur: f32,
    rayon: f32,
    nacelles: usize,
    nacelle: f32,
) {
    let demi = longueur * 0.5;
    let (rnac, places) = grappe_cargo(rayon, nacelles, nacelle);
    for (poste, spin) in places {
        pieces::nacelle_cargo(p, poste - Vec3::Z * demi, longueur, rnac, spin, CARGO, SOMBRE);
    }
}

pub(super) fn ratelier_cout(nacelles: usize) -> f32 {
    let n = nacelles.max(3) as f32;
    6.0 * n + 6.0 * n
}

/// Le coin le plus loin (demi-longueur, station + nacelle). Même disposition
/// que le dessin, donc jamais de divergence.
pub(super) fn ratelier_rayon_local(longueur: f32, rayon: f32, nacelles: usize, nacelle: f32) -> f32 {
    let (rnac, places) = grappe_cargo(rayon, nacelles, nacelle);
    let etendue = places.iter().fold(0.0_f32, |m, (q, _)| m.max(q.length())) + rnac;
    (longueur * 0.5).hypot(etendue)
}
