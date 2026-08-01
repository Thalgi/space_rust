//! **Module d'équipage rotatif** (ISV) : la nacelle habitée que l'on met en
//! rotation, au bout d'une traverse, pour fabriquer de la gravité.
//!
//! C'est la **seule pièce tournante** du vaisseau, et sa forme le dit : un fût
//! **cylindrique** là où le fret et l'habitat fixe sont en sections onigiri. Un
//! plancher courbe est ce qu'impose la rotation — sur une section triangulaire,
//! le « bas » changerait d'inclinaison le long de la paroi.
//!
//! Orientation : l'axe +Z du module pointe **vers l'extérieur** (loin de
//! l'épine). C'est donc le bout +Z qui fait office de **plancher**, la force
//! centrifuge poussant les occupants vers là ; les hublots regardent la même
//! direction, et le port de montage est à l'autre bout, côté traverse.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::Enveloppe;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::commun::*;

/// Vitrage des hublots : bleu sombre, pour lire comme du verre et non comme un
/// trou. La seule teinte froide du vaisseau.
const HUBLOT: Color = Color { r: 0.16, g: 0.22, b: 0.30, a: 1.0 };

/// Fraction de la longueur occupée par la calotte du plancher.
const CALOTTE: f32 = 0.28;

/// Un unique port axial à la base (avant −Z, vers la traverse) ; le module se
/// déploie vers l'extérieur.
pub(super) fn ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::ModuleAxial,
        profil,
    )]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, profil: Profil, longueur: f32, hublots: usize) {
    let r = profil.rayon();
    let dome = (r * CALOTTE * 2.0).min(longueur * CALOTTE);
    let fut = longueur - dome; // partie droite, du montage au plancher

    // Col de raccord à la traverse : un cou plus étroit, comme sur les modules
    // pressurisés — le joint se lit au lieu d'être un tube qui bute.
    p.cylindre(
        Vec3::ZERO,
        Vec3::Z * (fut * 0.10),
        r * COL_RAYON,
        BAGUE,
    );

    // Fût principal.
    p.cylindre(Vec3::Z * (fut * 0.06), Vec3::Z * fut, r, COULEUR);

    // Plancher : calotte bombée vers l'extérieur. C'est le point bas sous
    // rotation, donc la face que l'on veut voir fermée et pleine.
    p.cone(Vec3::Z * fut, Vec3::Z, r, r * 0.55, dome * 0.62, COULEUR);
    p.sphere(Vec3::Z * (fut + dome - r * 0.30), r * 0.30, COULEUR);

    // Bandeau de **hublots** en couronne, juste au-dessus du plancher : c'est
    // là que se tient l'équipage quand ça tourne.
    let n = hublots.max(3);
    let zh = fut - r * 0.55;
    for k in 0..n {
        let a = TAU * k as f32 / n as f32;
        let d = vec3(a.cos(), a.sin(), 0.0);
        // Léger débord radial : le hublot affleure la coque au lieu d'y être noyé.
        p.cube(d * (r * 0.99) + Vec3::Z * zh, Vec3::splat(r * 0.30), HUBLOT);
    }

    // Ceinture d'équipement à mi-fût, et rails longitudinaux : donne l'échelle
    // et casse le cylindre nu.
    p.cylindre(
        Vec3::Z * (fut * 0.45),
        Vec3::Z * (fut * 0.55),
        r * 1.04,
        SOMBRE,
    );
    for k in 0..4 {
        let a = TAU * k as f32 / 4.0 + PI / 4.0;
        let o = vec3(a.cos(), a.sin(), 0.0) * (r * 1.01);
        p.cylindre(o + Vec3::Z * (fut * 0.12), o + Vec3::Z * (fut * 0.92), r * 0.05, SOMBRE);
    }
}

pub(super) fn cout() -> f32 {
    12.0
}

/// Déployé le long de +Z : la longueur domine.
pub(super) fn rayon_local(profil: Profil, longueur: f32) -> f32 {
    longueur.max(profil.rayon() * 1.05)
}

/// Déployé d'un seul côté (+Z) : **capsule** couchée le long du fût, du montage
/// jusqu'au bout. C'est une nacelle — longue et fine —, et la sphère qui la
/// contenait réservait `hypot(demi, rayon)` dans toutes les directions, donc
/// mordait sur sa voisine de traverse alors que rien ne se touche.
pub(super) fn englobant(profil: Profil, longueur: f32) -> Enveloppe {
    let demi = longueur * 0.5;
    Enveloppe::axe(Vec3::Z * demi, Vec3::Z, demi, profil.rayon() * 1.05)
}

// --- Collier rotatif -------------------------------------------------------

/// **Collier de rotation** : le tambour qui ceinture l'épine et porte les bras
/// d'équipage. C'est la pièce qui *tourne*.
///
/// `alesage` est le rayon intérieur, `profil` donne le rayon extérieur. Le
/// composant n'impose rien sur leur rapport à l'épine : c'est l'appelant qui
/// décide s'il veut un palier dégagé (alésage plus large que la structure
/// traversante) ou un moyeu **traversé** par elle. L'ISV prend le second, parce
/// qu'un anneau de vide autour de la flèche se voit et donne une pièce enfilée
/// dessus plutôt que montée — voir `EQUIPAGE_ALESAGE` dans `generateur.rs`.
pub(super) fn collier_ports(profil: Profil, longueur: f32) -> Vec<Port> {
    let demi = longueur * 0.5;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn collier_dessiner<P: Peintre>(p: &mut P, rayon: f32, alesage: f32, longueur: f32) {
    let r = rayon;
    let demi = longueur * 0.5;
    let r_int = alesage.min(r * 0.92);

    // Tambour creux : c'est lui qui tourne autour de l'épine.
    crate::vaisseau::pieces::tube(p, -Vec3::Z * demi, longueur, r, r_int, COULEUR);

    // Deux bagues de roulement en saillie aux extrémités.
    //
    // Elles ne descendent **pas** jusqu'à l'alésage : une bague qui irait de
    // `r_int` à `r*1.06` partagerait exactement sa paroi intérieure avec celle
    // du tambour — deux surfaces dans le même plan, donc du z-fighting plein
    // l'alésage. Ce ne sont que des **frettes extérieures**, posées sur la
    // jaquette, et le vide central reste net.
    for s in [-1.0_f32, 1.0] {
        let z = s * (demi - longueur * 0.10);
        crate::vaisseau::pieces::tube(
            p,
            Vec3::Z * (z - longueur * 0.05),
            longueur * 0.10,
            r * 1.06,
            r * 0.97,
            SOMBRE,
        );
    }

    // Nervures longitudinales sur la jaquette, réparties entre les deux bras.
    for k in 0..6 {
        let a = TAU * k as f32 / 6.0 + PI / 6.0;
        let o = vec3(a.cos(), a.sin(), 0.0) * (r * 1.01);
        p.cylindre(o - Vec3::Z * (demi * 0.75), o + Vec3::Z * (demi * 0.75), r * 0.035, SOMBRE);
    }
}

pub(super) fn collier_cout() -> f32 {
    10.0
}

/// Tambour centré sur son axe : demi-longueur ou rayon extérieur (frettes
/// comprises, d'où le 1,06).
pub(super) fn collier_rayon_local(rayon: f32, longueur: f32) -> f32 {
    (longueur * 0.5).max(rayon * 1.06)
}

// --- Charnière de repli ----------------------------------------------------

/// **Charnière de repli** du bras d'équipage : une vraie articulation
/// mécanique, pas un coude dessiné.
///
/// Trois organes, ceux qu'on trouve sur toute structure déployable réelle :
/// - une **chape** (deux joues parallèles) solidaire du collier ;
/// - un **axe** qui les traverse — c'est lui qui donne la direction du
///   pivot, ici le **tangentiel** (local X), pour que le bras bascule du radial
///   vers l'axe du vaisseau ;
/// - un **vérin** télescopique en biais, décalé de l'axe pour avoir du bras de
///   levier. C'est lui qui fait « engin spatial » plutôt que « pièce de
///   meuble », et il s'allonge réellement avec le repli.
///
/// `repli` va de 0 (déployé, bras radial) à 1 (replié le long de la coque).
/// Repère local : le bras fixe part vers −Z, le bras mobile vers +Z quand
/// `repli = 0`, et pivote autour de +X.
pub(super) fn charniere_dessiner<P: Peintre>(p: &mut P, taille: f32, repli: f32) {
    let e = taille; // demi-gabarit de la chape
    let angle = repli.clamp(0.0, 1.0) * FRAC_PI_2;

    // Joues de la chape, de part et d'autre du plan de travail, portées par un
    // court talon qui les relie au collier.
    for s in [-1.0_f32, 1.0] {
        p.cube(
            vec3(s * e * 0.55, 0.0, -e * 0.35),
            vec3(e * 0.22, e * 1.05, e * 1.20),
            COULEUR,
        );
    }
    p.cube(vec3(0.0, 0.0, -e * 0.95), vec3(e * 1.3, e * 0.85, e * 0.5), COULEUR);

    // Axe d'articulation, traversant et débordant : on doit voir où ça tourne.
    p.cylindre(
        vec3(-e * 0.85, 0.0, 0.0),
        vec3(e * 0.85, 0.0, 0.0),
        e * 0.20,
        SOMBRE,
    );

    // Ferrure mobile : la patte qui part du même axe et emporte le bras.
    let (sa, ca) = angle.sin_cos();
    let bout = vec3(0.0, sa * e * 1.5, ca * e * 1.5);
    p.cylindre(Vec3::ZERO, bout, e * 0.34, COULEUR);
    p.cube(bout, Vec3::splat(e * 0.62), COULEUR);

    // Vérin : du bâti (décalé en +Y, donc hors de l'axe) vers un point de la
    // ferrure mobile. Deux diamètres emboîtés = corps + tige, la tige sortant
    // d'autant plus que le bras se replie.
    let pied = vec3(0.0, e * 0.95, -e * 0.8);
    let attache = vec3(0.0, sa * e * 1.05 + ca * e * 0.5, ca * e * 1.05 - sa * e * 0.5);
    let axe = attache - pied;
    let corps = pied + axe * 0.45;
    p.cylindre(pied, corps, e * 0.17, SOMBRE);
    p.cylindre(corps, attache, e * 0.11, BAGUE);
    p.sphere(pied, e * 0.19, SOMBRE); // rotule de pied
    p.sphere(attache, e * 0.15, SOMBRE); // rotule de tête
}

pub(super) fn charniere_cout() -> f32 {
    8.0
}

pub(super) fn charniere_rayon_local(taille: f32) -> f32 {
    taille * 1.8
}
