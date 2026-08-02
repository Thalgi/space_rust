//! **Tore** : la première primitive *paramétrique* du parc de composants.
//!
//! Tout le reste du vocabulaire est une **accrétion de briques** clipsées par
//! ports (`docs/conception/stations.md` Partie A §7). Un anneau d'habitat ne
//! l'est pas, et §6 du même document l'avait vu venir : « un anneau est une
//! boucle ; l'assemblage est un arbre… le plus KISS est de dessiner l'anneau
//! **en une primitive paramétrique**, pas de l'assembler par segments ».
//!
//! La raison est mesurable. Carreler un anneau de modules coûte **linéairement**
//! en son rayon (~2,9 pièces et ~2 900 sommets par unité de rayon, mesuré sur
//! `poser_anneau`) : un tore de rayon 25 revient à ~70 pièces, un de rayon 400 à
//! plus de 1 100 — davantage que l'ISV entier. Une surface paramétrique, elle,
//! coûte ce qu'on lui demande de facettes, indépendamment de son rayon.
//!
//! # Les trois bandes de la section
//!
//! L'angle `v` court autour du tube, dans le plan qui contient l'axe de
//! l'anneau. `v = 0` désigne le point le plus **loin** de l'axe, `v = π` le plus
//! **proche** — celui qui regarde le moyeu.
//!
//! ```text
//! v :   0°──────90°─────140°──────220°─────270°──────360°
//!       │ TUILES  │ COQUE │ FENÊTRE │ COQUE │  TUILES  │
//!       ↑         ↑       ↑         ↑       ↑          ↑
//!   EXTÉRIEUR   haut    ←── vers le moyeu ──→  bas   EXTÉRIEUR
//! ```
//!
//! - **Tuiles** : les 180° tournés vers l'espace, bardés d'hexagones à
//!   épaisseur. C'est la face exposée — micrométéorites, rayonnement.
//! - **Fenêtre** : 40° de part et d'autre de `v = π`, soit 80° face au moyeu.
//!   Pleine et bleue pour l'instant ; elle deviendra translucide, et c'est pour
//!   ça qu'elle est émise comme une **surface à part** dès maintenant.
//! - **Coque** : les deux épaulements de 50° qui restent, laissés lisses.

use crate::vaisseau::peintre::Peintre;
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::commun::*;

/// Bornes des bandes, en radians. Une seule source : le dessin **et** les tests
/// les lisent d'ici plutôt que de recopier des degrés.
pub(super) const V_TUILES: f32 = FRAC_PI_2; // ±90° autour de l'extérieur
pub(super) const V_FENETRE: f32 = 40.0 * PI / 180.0; // ±40° autour du moyeu

/// Pas du bardage — le côté visé d'une tuile, en unités monde. Le **nombre** de
/// tuiles s'en déduit, jamais l'inverse : un anneau deux fois plus grand porte
/// deux fois plus de tuiles de la même taille, au lieu des mêmes tuiles deux
/// fois plus grandes.
const TUILE: f32 = 1.24;
/// Épaisseur de la tuile, en unités monde — **absolue**, pas une fraction du
/// pas.
///
/// Elle l'était, et c'était un piège : doubler le pas doublait alors la
/// saillie, alors qu'un panneau plus large n'est pas un panneau plus épais.
/// Exprimée en dur, elle ne bouge plus quand on retaille le bardage, et il n'y
/// a plus rien à « re-régler en conséquence ».
const TUILE_EPAISSEUR: f32 = 0.09;
/// Largeur du joint entre deux tuiles, en unités monde — **absolue** aussi, et
/// pour la même raison : un joint est une tolérance de pose, il ne grandit pas
/// avec le panneau. Sans joint les hexagones se touchent et le bardage relit
/// comme une tôle continue.
const TUILE_JOINT: f32 = 0.075;

/// Pas des **meneaux** le long de l'anneau : un vitrage courbe continu sur
/// 150 unités n'existe pas, il se fabrique en panneaux. C'est ce pas qui donne
/// leur taille aux panneaux, donc l'échelle apparente de tout l'anneau.
const MENEAU_PAS: f32 = 2.4;
/// Section d'un meneau et d'une lèvre, en fraction du rayon de section.
const MENEAU_SECTION: f32 = 0.055;
/// Nombre de segments d'un meneau le long de la section : il suit la courbure
/// du tube, il ne la coupe pas en corde.
const MENEAU_PAS_ARC: usize = 5;
/// Demi-largeur d'une **jonction** (là où un bras rejoint l'anneau), en unités
/// monde le long de l'anneau. Large devant la pointe du bras (~1 unité) : on
/// veut voir une reprise d'effort, pas un simple contact.
const JONCTION_DEMI: f32 = 3.2;

/// Point de la surface du tore, et sa normale sortante.
fn surface(rayon_majeur: f32, rayon_mineur: f32, u: f32, v: f32) -> (Vec3, Vec3) {
    let (su, cu) = u.sin_cos();
    let (sv, cv) = v.sin_cos();
    let n = vec3(cv * cu, sv, cv * su);
    let r = rayon_majeur + rayon_mineur * cv;
    (vec3(r * cu, rayon_mineur * sv, r * su), n)
}

/// Sommets et indices d'une **portion** de tore, sur la plage `[v0, v1]`.
fn peau(
    rayon_majeur: f32,
    rayon_mineur: f32,
    segments: usize,
    anneaux: usize,
    v0: f32,
    v1: f32,
) -> (Vec<Vec3>, Vec<u16>) {
    let ns = segments.max(3);
    let nv = anneaux.max(2);
    let mut sommets = Vec::with_capacity((ns + 1) * (nv + 1));
    for i in 0..=ns {
        let u = TAU * i as f32 / ns as f32;
        for j in 0..=nv {
            let v = v0 + (v1 - v0) * j as f32 / nv as f32;
            sommets.push(surface(rayon_majeur, rayon_mineur, u, v).0);
        }
    }
    let mut indices = Vec::with_capacity(ns * nv * 6);
    let large = (nv + 1) as u16;
    for i in 0..ns as u16 {
        for j in 0..nv as u16 {
            let a = i * large + j;
            let (b, c, d) = (a + large, a + large + 1, a + 1);
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    (sommets, indices)
}

/// Barde la bande `[−V_TUILES, +V_TUILES]` de tuiles hexagonales, en quinconce.
///
/// Les rangs courent **le long de la section** (en `v`), les colonnes le long
/// de l'anneau (`u`). Un rang sur deux est décalé d'une demi-colonne : c'est ce
/// décalage qui fait un pavage hexagonal et non un damier. Le nombre de
/// colonnes est calculé une fois, sur le rayon **majeur** — les tuiles du bord
/// extérieur sont donc un rien plus espacées que celles des épaulements, ce qui
/// ne se voit pas et évite un compte par rang.
/// Découpage du bardage : `(rangs le long de la section, colonnes le long de
/// l'anneau)`.
///
/// Sorti du dessin pour être **mesurable** : c'est le compte de tuiles qui doit
/// suivre la taille de l'anneau, et le vérifier sur le nombre de sommets du
/// tore entier ne le dit pas — les bandes lisses, les meneaux et les lèvres
/// s'y ajoutent et diluent le rapport.
pub(super) fn grille_tuiles(rayon_majeur: f32, rayon_mineur: f32) -> (usize, usize) {
    let arc = 2.0 * V_TUILES * rayon_mineur; // largeur de la bande, le long de v
    (
        (arc / TUILE).round().max(1.0) as usize,
        (TAU * rayon_majeur / TUILE).round().max(3.0) as usize,
    )
}

/// Côté d'une tuile, joint déduit — la cote que le bardage garde constante.
pub(super) fn cote_tuile() -> f32 {
    TUILE - TUILE_JOINT
}

fn tuiles<P: Peintre>(p: &mut P, rayon_majeur: f32, rayon_mineur: f32) {
    let (rangs, colonnes) = grille_tuiles(rayon_majeur, rayon_mineur);

    let mut dessus = (Vec::new(), Vec::new());
    let mut flancs = (Vec::new(), Vec::new());
    for r in 0..rangs {
        let t = (r as f32 + 0.5) / rangs as f32;
        let v = -V_TUILES + 2.0 * V_TUILES * t;
        let decale = if r % 2 == 0 { 0.0 } else { 0.5 };
        for c in 0..colonnes {
            let u = TAU * (c as f32 + decale) / colonnes as f32;
            let (centre, n) = surface(rayon_majeur, rayon_mineur, u, v);
            // Tangente le long de l'anneau : c'est elle qui aligne les rangs
            // au lieu de les laisser tourner avec la courbure.
            let tangente = vec3(-u.sin(), 0.0, u.cos());
            crate::vaisseau::pieces::tuile_hexagonale(
                centre,
                n,
                tangente,
                (TUILE - TUILE_JOINT) * 0.5,
                TUILE_EPAISSEUR,
                &mut dessus,
                &mut flancs,
            );
        }
    }
    // Deux lots pour tout le bardage, pas deux par tuile.
    p.triangles(&flancs.0, &flancs.1, TUILE_FLANC);
    p.triangles(&dessus.0, &dessus.1, TUILE_DESSUS);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dessiner<P: Peintre>(
    p: &mut P,
    rayon_majeur: f32,
    rayon_mineur: f32,
    segments: usize,
    anneaux: usize,
    jonctions: usize,
    phase: f32,
) {
    let nv = anneaux.max(2);
    // Coque **sous** les tuiles : elles ne se touchent pas, il faut un fond,
    // sombre pour que les joints lisent comme des joints et non comme des trous.
    let (s, i) = peau(rayon_majeur, rayon_mineur, segments, nv, -V_TUILES, V_TUILES);
    p.triangles(&s, &i, SOMBRE);
    // Les deux épaulements lisses, entre les tuiles et la fenêtre.
    let (s, i) = peau(rayon_majeur, rayon_mineur, segments, nv / 2, V_TUILES, PI - V_FENETRE);
    p.triangles(&s, &i, COULEUR);
    let (s, i) = peau(rayon_majeur, rayon_mineur, segments, nv / 2, PI + V_FENETRE, TAU - V_TUILES);
    p.triangles(&s, &i, COULEUR);
    fenetre(p, rayon_majeur, rayon_mineur, segments, nv, jonctions, phase);
    tuiles(p, rayon_majeur, rayon_mineur);
}

/// Le tore n'expose **aucun port** : c'est une coque, pas une brique clipsable.
/// Il se pose à la main, comme `TreillisHexagone`.
pub(super) fn cout(rayon_majeur: f32) -> f32 {
    // Proportionnel à la circonférence : un anneau deux fois plus grand est
    // deux fois plus de coque à payer.
    (rayon_majeur * 0.8).max(1.0)
}

/// Demi-encombrement depuis le centre : le grand rayon plus la section.
pub(super) fn rayon_local(rayon_majeur: f32, rayon_mineur: f32) -> f32 {
    rayon_majeur + rayon_mineur
}

/// Le vitrage du pont : des **panneaux** encadrés, pas une verrière continue.
///
/// Trois choses s'y ajoutent, et chacune répond à une raison physique :
///
/// - **Meneaux** — un vitrage courbe de 150 unités de long ne se fabrique pas
///   d'un tenant. Il se pose en panneaux, et le pas des meneaux est ce qui
///   donne son échelle apparente à tout l'anneau : sans eux, rien ne dit si le
///   tore fait dix mètres ou un kilomètre.
/// - **Lèvres** — les deux bords du vitrage sont la discontinuité la plus
///   sollicitée de la section (coque pleine d'un côté, verre de l'autre). Une
///   nervure les reprend.
/// - **Jonctions** — là où un bras arrive, on ne met pas de verre : c'est par
///   là que passe l'effort. La bande y redevient pleine, et deux nervures
///   transversales encadrent la reprise.
#[allow(clippy::too_many_arguments)]
fn fenetre<P: Peintre>(
    p: &mut P,
    rayon_majeur: f32,
    rayon_mineur: f32,
    segments: usize,
    anneaux: usize,
    jonctions: usize,
    phase: f32,
) {
    let ns = segments.max(3);
    let nv = anneaux.max(2);
    let (v0, v1) = (PI - V_FENETRE, PI + V_FENETRE);
    let demi = if rayon_majeur > 1e-3 { JONCTION_DEMI / rayon_majeur } else { 0.0 };

    // Un `u` tombe-t-il dans une jonction ? Écart **signé** ramené dans
    // [−π, π] : sans ça, la jonction à cheval sur 0 serait ratée.
    let dans_jonction = |u: f32| -> bool {
        (0..jonctions).any(|k| {
            let c = phase + TAU * k as f32 / jonctions.max(1) as f32;
            let d = (u - c + PI).rem_euclid(TAU) - PI;
            d.abs() < demi
        })
    };

    // --- la bande elle-même, quad par quad, verre ou coque pleine ---
    let mut verre = (Vec::new(), Vec::new());
    let mut plein = (Vec::new(), Vec::new());
    for i in 0..ns {
        let ua = TAU * i as f32 / ns as f32;
        let ub = TAU * (i + 1) as f32 / ns as f32;
        let cible = if dans_jonction((ua + ub) * 0.5) { &mut plein } else { &mut verre };
        for j in 0..nv {
            let va = v0 + (v1 - v0) * j as f32 / nv as f32;
            let vb = v0 + (v1 - v0) * (j + 1) as f32 / nv as f32;
            let base = cible.0.len() as u16;
            for (u, v) in [(ua, va), (ub, va), (ub, vb), (ua, vb)] {
                cible.0.push(surface(rayon_majeur, rayon_mineur, u, v).0);
            }
            cible.1.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    p.triangles(&verre.0, &verre.1, PONT);
    p.triangles(&plein.0, &plein.1, COQUE_JONCTION);

    // --- une nervure suivant la section, à `u` fixe ---
    let nervure_section = |p: &mut P, u: f32, ep: f32| {
        for k in 0..MENEAU_PAS_ARC {
            let a = v0 + (v1 - v0) * k as f32 / MENEAU_PAS_ARC as f32;
            let b = v0 + (v1 - v0) * (k + 1) as f32 / MENEAU_PAS_ARC as f32;
            let (pa, na) = surface(rayon_majeur, rayon_mineur, u, a);
            let (pb, nb) = surface(rayon_majeur, rayon_mineur, u, b);
            p.cylindre(pa + na * ep, pb + nb * ep, rayon_mineur * MENEAU_SECTION, MENEAU);
        }
    };

    // --- meneaux : un panneau tous les `MENEAU_PAS`, hors jonctions ---
    let saillie = rayon_mineur * MENEAU_SECTION;
    let n_meneaux = (TAU * rayon_majeur / MENEAU_PAS).round().max(3.0) as usize;
    for k in 0..n_meneaux {
        let u = TAU * k as f32 / n_meneaux as f32;
        if !dans_jonction(u) {
            nervure_section(p, u, saillie);
        }
    }

    // --- jonctions : deux nervures qui encadrent la reprise d'effort ---
    for k in 0..jonctions {
        let c = phase + TAU * k as f32 / jonctions.max(1) as f32;
        for bord in [-demi, demi] {
            nervure_section(p, c + bord, saillie * 1.6);
        }
    }

    // --- lèvres : les deux bords du vitrage, tout autour ---
    for v in [v0, v1] {
        for i in 0..ns {
            let ua = TAU * i as f32 / ns as f32;
            let ub = TAU * (i + 1) as f32 / ns as f32;
            let (pa, na) = surface(rayon_majeur, rayon_mineur, ua, v);
            let (pb, nb) = surface(rayon_majeur, rayon_mineur, ub, v);
            p.cylindre(pa + na * saillie, pb + nb * saillie, rayon_mineur * MENEAU_SECTION * 1.3, MENEAU);
        }
    }
}
