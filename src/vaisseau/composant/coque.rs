//! **Coque lisse à ogive** : la première structure *porteuse et fermée* du parc.
//!
//! Tout le vocabulaire existant est ajouré ou court — treillis, charpentes,
//! modules pressurisés de quelques mètres. Un lanceur, lui, est un **fût
//! continu** : sa peau *est* sa structure, et elle porte la poussée sur toute sa
//! hauteur. Rien ne savait dessiner ça (`docs/conception/stations.md` §D.4).
//!
//! # Ce qui fait qu'une coque lit comme un vaisseau et non comme un tube
//!
//! Trois choses, et aucune n'est décorative :
//!
//! 1. **L'ogive.** Un cône lit comme une fusée d'enfant. La vraie coiffe est une
//!    *ogive tangente* — la courbe qui rejoint le fût **sans cassure de pente**,
//!    et c'est cette continuité qu'on voit, pas la finesse de la pointe.
//! 2. **Les viroles.** Un fût inox est un empilement d'anneaux soudés d'environ
//!    1,8 m. Les cordons horizontaux donnent l'échelle : sans eux, une coque de
//!    50 m et une de 5 m sont le même dessin.
//! 3. **Le méplat de base.** Un lanceur est plat dessous, là où sont les
//!    moteurs. Un fond bombé le ferait lire comme un réservoir.
//!
//! # Ogive tangente
//!
//! Pour un rayon de base `R` et une longueur d'ogive `L`, le rayon de courbure
//! vaut `ρ = (R² + L²) / 2R`, et le profil, mesuré depuis la **pointe** :
//!
//! ```text
//! y(x) = √(ρ² − (L − x)²) + R − ρ,   x ∈ [0, L]
//! ```
//!
//! Elle vaut 0 à la pointe et exactement `R` à la base, où sa tangente est
//! parallèle à l'axe — c'est là toute la propriété : la jonction avec le fût est
//! lisse par construction, pas par réglage.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

use super::commun::*;

/// Inox nu, non peint : clair et légèrement chaud. C'est la signature d'un
/// Starship — un lanceur blanc lirait comme une Ariane.
const INOX: Color = Color { r: 0.78, g: 0.79, b: 0.81, a: 1.0 };
/// Cordon de soudure entre deux viroles : à peine plus sombre. Le contraste
/// doit rester faible — ce sont des soudures, pas des jointures de blindage.
const SOUDURE: Color = Color { r: 0.62, g: 0.63, b: 0.66, a: 1.0 };

/// Facettes autour de l'axe. 24 suffit à ce diamètre : au-delà, le gain est
/// invisible et le coût en sommets, lui, ne l'est pas.
const FACETTES: usize = 24;
/// Pas de la virole, en unités monde (~1,8 m). Absolu et non proportionnel :
/// une coque deux fois plus haute a **deux fois plus** d'anneaux, pas des
/// anneaux deux fois plus hauts — c'est ce qui fait que les viroles donnent
/// l'échelle au lieu de la suivre.
pub const VIROLE: f32 = 0.8;

/// Un cordon de soudure **affleure** la peau (il ne s'y noie pas) et porte sa
/// propre section. Les deux en fraction du rayon.
const SOUDURE_AFFLEURE: f32 = 0.004;
const SOUDURE_SECTION: f32 = 0.012;

/// Rayon **hors-tout**, cordons compris.
///
/// Une source unique pour le dessin et pour l'enveloppe : les cordons dépassent
/// de 1,6 % du rayon, ce qui est invisible à l'œil et suffit largement à faire
/// sortir des sommets d'une capsule calée sur le rayon nominal. C'est un
/// débordement de la même famille que ceux de L1.4 — petit, mais réel.
pub(super) fn rayon_hors_tout(profil: Profil) -> f32 {
    profil.rayon() * (1.0 + SOUDURE_AFFLEURE + SOUDURE_SECTION)
}

/// Rayon de l'ogive tangente à la distance `x` de la **pointe**.
fn ogive(rayon: f32, longueur: f32, x: f32) -> f32 {
    if longueur <= 1e-4 {
        return rayon;
    }
    let rho = (rayon * rayon + longueur * longueur) / (2.0 * rayon);
    let d = longueur - x.clamp(0.0, longueur);
    (rho * rho - d * d).max(0.0).sqrt() + rayon - rho
}

/// La formule d'ogive, exposee pour que les tests la mesurent **a la source**
/// plutot que de la recopier.
#[cfg(test)]
pub(super) fn ogive_essai(rayon: f32, longueur: f32, x: f32) -> f32 {
    ogive(rayon, longueur, x)
}

/// Hauteur hors-tout : le fût **plus** l'ogive.
pub fn hauteur(longueur: f32, nez: f32) -> f32 {
    longueur + nez
}

/// Deux ports axiaux : la base (là où va la propulsion) et le sommet de
/// l'ogive. Le fût lui-même n'expose rien — on ne clipse pas un module sur le
/// flanc d'un lanceur.
pub(super) fn ports(profil: Profil, longueur: f32, nez: f32) -> Vec<Port> {
    let demi = hauteur(longueur, nez) * 0.5;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn cout(longueur: f32, nez: f32) -> f32 {
    4.0 + hauteur(longueur, nez)
}

/// Demi-encombrement depuis le centre : la moitié de la hauteur domine
/// largement le rayon sur un lanceur (élancement ~5,5).
pub(super) fn rayon_local(profil: Profil, longueur: f32, nez: f32) -> f32 {
    (hauteur(longueur, nez) * 0.5).hypot(rayon_hors_tout(profil))
}

/// Une couronne de sommets à la cote `z`, de rayon `r`.
fn couronne(z: f32, r: f32) -> Vec<Vec3> {
    (0..FACETTES)
        .map(|k| {
            let a = TAU * k as f32 / FACETTES as f32;
            vec3(r * a.cos(), r * a.sin(), z)
        })
        .collect()
}

/// Ceinture de quads entre deux couronnes consécutives.
fn ceinture(bas: &[Vec3], haut: &[Vec3], s: &mut Vec<Vec3>, i: &mut Vec<u16>) {
    let base = s.len() as u16;
    s.extend_from_slice(bas);
    s.extend_from_slice(haut);
    let n = FACETTES as u16;
    for k in 0..n {
        let (k1, b0, b1) = ((k + 1) % n, base + k, base + (k + 1) % n);
        let (h0, h1) = (base + n + k, base + n + k1);
        i.extend_from_slice(&[b0, h0, h1, b0, h1, b1]);
    }
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, profil: Profil, longueur: f32, nez: f32) {
    let r = profil.rayon();
    let demi = hauteur(longueur, nez) * 0.5;
    let z0 = -demi; // base plate, côté moteurs
    let z1 = z0 + longueur; // raccord fût / ogive

    let mut sommets = Vec::new();
    let mut indices = Vec::new();

    // --- fût : une ceinture par virole, pour que les soudures tombent sur de
    // vraies arêtes de maillage plutôt que d'être posées par-dessus.
    let n_vir = (longueur / VIROLE).round().max(1.0) as usize;
    let mut prec = couronne(z0, r);
    for k in 1..=n_vir {
        let z = z0 + longueur * k as f32 / n_vir as f32;
        let cur = couronne(z, r);
        ceinture(&prec, &cur, &mut sommets, &mut indices);
        prec = cur;
    }

    // --- ogive : le pas se resserre vers la pointe, là où la courbure est la
    // plus forte. Un pas constant facetterait visiblement le nez.
    let n_og = 14usize;
    for k in 1..=n_og {
        let t = k as f32 / n_og as f32;
        let t = t * t; // resserrement vers le sommet
        let z = z1 + nez * t;
        // `ogive` compte depuis la pointe : la distance vaut donc `nez − nez·t`.
        let cur = couronne(z, ogive(r, nez, nez * (1.0 - t)).max(0.0));
        ceinture(&prec, &cur, &mut sommets, &mut indices);
        prec = cur;
    }
    p.triangles(&sommets, &indices, INOX);

    // --- fond plat : un lanceur est plat dessous, c'est là que sont les
    // moteurs. Un fond bombé le ferait lire comme un réservoir.
    let mut fond = couronne(z0, r);
    let centre = fond.len() as u16;
    fond.push(vec3(0.0, 0.0, z0));
    let mut idx = Vec::new();
    for k in 0..FACETTES as u16 {
        idx.extend_from_slice(&[centre, (k + 1) % FACETTES as u16, k]);
    }
    p.triangles(&fond, &idx, SOUDURE);

    // --- cordons de soudure : ce sont eux qui donnent l'échelle.
    for k in 0..=n_vir {
        let z = z0 + longueur * k as f32 / n_vir as f32;
        let c = couronne(z, r * (1.0 + SOUDURE_AFFLEURE));
        for j in 0..FACETTES {
            p.cylindre(c[j], c[(j + 1) % FACETTES], r * SOUDURE_SECTION, SOUDURE);
        }
    }
}
