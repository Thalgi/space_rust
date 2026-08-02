//! Briques de construction réutilisables pour les stations spatiales.
//!
//! Chaque fonction dessine un élément paramétré (position, orientation, taille,
//! couleurs) sans supposer de palette : les couleurs sont fournies par
//! l'appelant. Ces briques sont la base de la génération procédurale de
//! stations — voir `docs/conception/stations.md`, Partie A.
//!
//! **Sortie abstraite** : les briques partagées avec `composant.rs` sont
//! génériques sur [`Peintre`], donc utilisables aussi bien en dessin immédiat
//! (galeries `brique_demo`/`iss`) qu'en cuisson de maillage. Un seul code de
//! géométrie alimente les deux, ce qui interdit toute dérive visuelle entre eux.
//! `module` et `paire_ailes` ne servent qu'aux galeries : elles restent en
//! immédiat (`paire_ailes` utilise des fils, qu'un maillage ne porte pas).

use super::peintre::Peintre;
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_3, FRAC_PI_6, TAU};

/// Épaisseur des traits (nervures, coutures) quand ils sont cuits en géométrie.
/// Ignorée par la sortie immédiate, qui trace un vrai segment d'un pixel.
const TRAIT: f32 = 0.02;

/// Pale à **tuiles hexagonales** légèrement espacées : un maillage de petits
/// hexagones (flat-top) réduits pour laisser un jour entre eux. Déployée depuis
/// `pied` le long de `deploy`, large de `largeur` selon `largeur_axe`.
pub(crate) fn pale_hexagonale<P: Peintre>(
    p: &mut P,
    pied: Vec3,
    deploy: Vec3,
    largeur_axe: Vec3,
    longueur: f32,
    largeur: f32,
    couleur: Color,
) {
    let d = deploy.normalize();
    let w = largeur_axe.normalize();
    let r = 0.16; // rayon d'une tuile (centre → sommet)
    let hr = r * 0.86; // tuile réduite → espace visible entre tuiles
    let dcol = 1.5 * r; // pas entre colonnes (hexagones flat-top)
    let drow = 3.0_f32.sqrt() * r; // pas entre rangées
    let mut sommets: Vec<Vec3> = Vec::new();
    let mut inds: Vec<u16> = Vec::new();
    let ncol = (largeur / dcol) as i32;
    let nrow = (longueur / drow) as i32;
    let x0 = -(ncol - 1) as f32 * dcol * 0.5; // centrage horizontal des colonnes
    for col in 0..ncol {
        let cx = x0 + col as f32 * dcol;
        let decal = if col % 2 == 0 { 0.0 } else { drow * 0.5 };
        for row in 0..=nrow {
            let cz = decal + row as f32 * drow;
            if cz > longueur {
                continue;
            }
            let centre = pied + w * cx + d * cz;
            let i0 = sommets.len() as u16;
            sommets.push(centre);
            for k in 0..6 {
                let a = FRAC_PI_3 * k as f32;
                sommets.push(centre + w * (hr * a.cos()) + d * (hr * a.sin()));
            }
            for k in 0..6u16 {
                let a = i0 + 1 + k;
                let b = i0 + 1 + (k + 1) % 6;
                inds.extend_from_slice(&[i0, a, b, i0, b, a]); // double face
            }
        }
    }
    p.triangles(&sommets, &inds, couleur);
}

/// **Sphère à tuiles triangulaires** : icosphère (icosaèdre dont chaque face est
/// subdivisée `freq` fois puis projetée sur la sphère). Chaque facette est
/// **rétrécie vers son centre** de `jour` pour laisser un joint visible entre
/// tuiles. Un seul lot de triangles, double face.
pub(crate) fn sphere_triangulee<P: Peintre>(
    p: &mut P,
    centre: Vec3,
    rayon: f32,
    freq: usize,
    jour: f32,
    couleur: Color,
) {
    // Émet une tuile (triangle rétréci vers son centre) dans les tampons.
    fn tuile(a: Vec3, b: Vec3, c: Vec3, s: &mut Vec<Vec3>, ix: &mut Vec<u16>, jour: f32) {
        let ctr = (a + b + c) / 3.0;
        let k = 1.0 - jour;
        let i0 = s.len() as u16;
        s.push(ctr + (a - ctr) * k);
        s.push(ctr + (b - ctr) * k);
        s.push(ctr + (c - ctr) * k);
        ix.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 1]); // double face
    }

    let gold = (1.0 + 5.0_f32.sqrt()) * 0.5; // nombre d'or
    let base: [Vec3; 12] = [
        vec3(-1.0, gold, 0.0), vec3(1.0, gold, 0.0), vec3(-1.0, -gold, 0.0), vec3(1.0, -gold, 0.0),
        vec3(0.0, -1.0, gold), vec3(0.0, 1.0, gold), vec3(0.0, -1.0, -gold), vec3(0.0, 1.0, -gold),
        vec3(gold, 0.0, -1.0), vec3(gold, 0.0, 1.0), vec3(-gold, 0.0, -1.0), vec3(-gold, 0.0, 1.0),
    ];
    let faces: [[usize; 3]; 20] = [
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ];
    let f = freq.max(1);
    let mut sommets: Vec<Vec3> = Vec::new();
    let mut inds: Vec<u16> = Vec::new();
    for face in faces {
        let va = base[face[0]].normalize();
        let vb = base[face[1]].normalize();
        let vc = base[face[2]].normalize();
        // Point (i, j) de la grille barycentrique, projeté sur la sphère.
        let pt = |i: usize, j: usize| -> Vec3 {
            let (fi, fj, ff) = (i as f32, j as f32, f as f32);
            let w = va * (ff - fi - fj) + vb * fi + vc * fj;
            centre + w.normalize() * rayon
        };
        for i in 0..f {
            for j in 0..(f - i) {
                tuile(pt(i, j), pt(i + 1, j), pt(i, j + 1), &mut sommets, &mut inds, jour);
                if j < f - i - 1 {
                    tuile(pt(i + 1, j), pt(i + 1, j + 1), pt(i, j + 1), &mut sommets, &mut inds, jour);
                }
            }
        }
    }
    p.triangles(&sommets, &inds, couleur);
}

/// Repère local orthonormé (avant, droite, haut) déduit d'un axe principal.
fn repere(axe: Vec3) -> (Vec3, Vec3, Vec3) {
    let f = axe.normalize();
    let ref_haut = if f.dot(Vec3::Y).abs() > 0.95 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let droite = f.cross(ref_haut).normalize();
    let haut = droite.cross(f).normalize();
    (f, droite, haut)
}

/// **Prisme hexagonal en treillis**, hexagone dans le plan X‑Z local, centré en
/// `centre`, à **arêtes haute et basse horizontales** (parallèles à X). Chaque
/// arête est une poutre à section **rectangulaire** : demi‑épaisseur `sec` dans
/// le plan de l'hexagone, demi‑profondeur `prof` **hors‑plan** (≈ Y) — ce sont
/// ces faces hors‑plan qui donnent au volume son épaisseur. `cote` est la
/// longueur d'arête (largeur du côté haut, celui « en contact »). Renvoie le
/// milieu de l'arête haute (raccord éventuel).
pub(crate) fn treillis_hexagone<P: Peintre>(
    p: &mut P,
    centre: Vec3,
    cote: f32,
    sec: f32,
    prof: f32,
    metal: Color,
    sombre: Color,
) -> Vec3 {
    let r = cote; // hexagone régulier : circonradius = côté
    let ap = cote * 3.0_f32.sqrt() * 0.5; // apothème (centre → milieu d'arête)
    let demi = cote * 0.5;
    let v = [
        centre + vec3(-demi, 0.0, ap),  // haut‑gauche
        centre + vec3(demi, 0.0, ap),   // haut‑droit
        centre + vec3(r, 0.0, 0.0),     // droit
        centre + vec3(demi, 0.0, -ap),  // bas‑droit
        centre + vec3(-demi, 0.0, -ap), // bas‑gauche
        centre + vec3(-r, 0.0, 0.0),    // gauche
    ];
    let t = sec.min(prof);
    // Coins de section **à chaque sommet**, partagés par les deux arêtes qui s'y
    // rejoignent → aucun écart aux angles. Offset **radial** (centre → sommet)
    // pour l'épaisseur dans le plan, et ±Y pour la profondeur.
    let coins = |k: usize| -> [Vec3; 4] {
        let ur = (v[k] - centre).normalize_or_zero(); // radial dans le plan X‑Z
        [
            v[k] - ur * sec - Vec3::Y * prof,
            v[k] + ur * sec - Vec3::Y * prof,
            v[k] + ur * sec + Vec3::Y * prof,
            v[k] - ur * sec + Vec3::Y * prof,
        ]
    };
    let mut prev = coins(5);
    for k in 0..6 {
        let cur = coins(k);
        for i in 0..4 {
            p.cylindre(prev[i], cur[i], t * 0.30, metal); // longeron (rail continu)
        }
        for w in 0..4 {
            p.cylindre(cur[w], cur[(w + 1) % 4], t * 0.22, sombre); // cadre au sommet
        }
        p.cylindre(prev[0], cur[2], t * 0.18, sombre); // diagonale de baie
        prev = cur;
    }
    centre + vec3(0.0, 0.0, ap) // milieu de l'arête haute
}

/// Poutre en treillis à section carrée reliant `a` à `b` : quatre longerons et
/// des cadres/diagonales de baie répartis sur la longueur.
pub(crate) fn treillis<P: Peintre>(p: &mut P, a: Vec3, b: Vec3, demi: f32, metal: Color, sombre: Color) {
    let axe = b - a;
    let long = axe.length();
    if long < 1e-4 {
        return;
    }
    let (_, d, h) = repere(axe);
    let coins = [
        d * -demi + h * -demi,
        d * demi + h * -demi,
        d * demi + h * demi,
        d * -demi + h * demi,
    ];
    for c in coins {
        p.cylindre(a + c, b + c, demi * 0.18, metal); // longerons
    }
    let baies = (long / (demi * 3.0)).round().max(1.0) as usize;
    for k in 0..=baies {
        let c = a + axe * (k as f32 / baies as f32);
        for w in 0..4 {
            p.cylindre(c + coins[w], c + coins[(w + 1) % 4], demi * 0.10, sombre); // cadre
        }
        if k < baies {
            let c2 = a + axe * ((k + 1) as f32 / baies as f32);
            p.cylindre(c + coins[0], c2 + coins[2], demi * 0.09, sombre); // diagonales
            p.cylindre(c + coins[1], c2 + coins[3], demi * 0.09, sombre);
        }
    }
}

/// Poutre en treillis à section **triangulaire** (3 longerons) — plus légère,
/// look « sonde ». Mêmes cadres/diagonales que la version carrée, en 3 côtés.
pub(crate) fn treillis_triangulaire<P: Peintre>(
    p: &mut P,
    a: Vec3,
    b: Vec3,
    demi: f32,
    metal: Color,
    sombre: Color,
) {
    let axe = b - a;
    let long = axe.length();
    if long < 1e-4 {
        return;
    }
    let (_, d, h) = repere(axe);
    let coin = |deg: f32| {
        let r = deg.to_radians();
        d * (demi * r.cos()) + h * (demi * r.sin())
    };
    let coins = [coin(90.0), coin(210.0), coin(330.0)]; // 1 en haut, 2 en bas
    for c in coins {
        p.cylindre(a + c, b + c, demi * 0.16, metal); // longerons
    }
    let baies = (long / (demi * 3.0)).round().max(1.0) as usize;
    for k in 0..=baies {
        let c = a + axe * (k as f32 / baies as f32);
        for w in 0..3 {
            p.cylindre(c + coins[w], c + coins[(w + 1) % 3], demi * 0.10, sombre); // cadre
        }
        if k < baies {
            let c2 = a + axe * ((k + 1) as f32 / baies as f32);
            for w in 0..3 {
                p.cylindre(c + coins[w], c2 + coins[(w + 1) % 3], demi * 0.08, sombre); // diagonales
            }
        }
    }
}

/// Poutre en treillis à section **variable et courbe** : carrée de demi-section
/// `demi_a` à `a`, `demi_b` à `b`, l'interpolation suivant `(1 − t)^courbure`.
/// `courbure = 1` → cône droit (pyramide) ; `courbure > 1` → base qui s'évase
/// puis longue flèche affinée (silhouette d'ISV). Les longerons sont tracés en
/// **polyligne** de baie en baie pour **suivre la courbe** au lieu de la couper
/// en droite.
pub(crate) fn treillis_conique<P: Peintre>(
    p: &mut P,
    a: Vec3,
    b: Vec3,
    demi_a: f32,
    demi_b: f32,
    courbure: f32,
    metal: Color,
    sombre: Color,
) {
    let axe = b - a;
    let long = axe.length();
    if long < 1e-4 {
        return;
    }
    let (_, d, h) = repere(axe);
    let carre = |c: Vec3, s: f32| [c + d * -s + h * -s, c + d * s + h * -s, c + d * s + h * s, c + d * -s + h * s];
    let epais = demi_a.max(demi_b);
    // Assez de baies pour que la courbe soit lisse (bornée pour le batcher).
    let baies = ((long / (epais.max(0.4) * 1.3)) as usize).clamp(10, 48);
    // L'évasement se joue sur une **distance absolue** (proportionnelle à la
    // base), pas sur la fraction de longueur : ainsi rallonger la charpente
    // n'agrandit **que la flèche**, la base garde sa courbe. Au-delà, section
    // constante = `demi_b` (la tige fine).
    let flare = (demi_a * 9.0).min(long);
    let section = |t: f32| {
        let f = (t * long / flare).min(1.0);
        demi_b + (demi_a - demi_b) * (1.0 - f).powf(courbure)
    };
    let anneau = |k: usize| {
        let t = k as f32 / baies as f32;
        let s = section(t);
        (carre(a + axe * t, s), s)
    };
    let (mut prev, mut ps) = anneau(0);
    for w in 0..4 {
        p.cylindre(prev[w], prev[(w + 1) % 4], ps * 0.10, sombre); // cadre de base
    }
    for k in 1..=baies {
        let (cur, cs) = anneau(k);
        for i in 0..4 {
            p.cylindre(prev[i], cur[i], demi_a * 0.15, metal); // longeron : **Ø constant** = tiges blanches de l'hexagone (pas d'amincissement)
        }
        for w in 0..4 {
            p.cylindre(cur[w], cur[(w + 1) % 4], cs * 0.10, sombre); // cadre
        }
        p.cylindre(prev[0], cur[2], ps * 0.08, sombre); // diagonales
        p.cylindre(prev[1], cur[3], ps * 0.08, sombre);
        prev = cur;
        ps = cs;
    }
}

/// Rayon d'un longeron, en fraction du circonradius de la section de **référence**
/// de la pièce.
///
/// « De référence » et non « courante » : un longeron ne s'épaissit **pas** parce que
/// la section s'évase. Le cône hexagonal et le pavillon prennent donc tous deux la
/// section de leur base, et la tour qui couronne le pavillon doit faire de même —
/// sinon ses barres grossissent avec l'embouchure et écrasent la silhouette.
pub(crate) const LONGERON: f32 = 0.12;
/// Ceinture et diagonale, en fraction de l'épaisseur de longeron. Rapports repris
/// du treillis d'origine (0,09 et 0,07 pour 0,12).
const CEINTURE: f32 = 0.75;
const DIAGONALE: f32 = 0.583;

/// Rapport de largeur de silhouette entre un hexagone régulier et le **carré de
/// même circonradius**, dans leur orientation la plus défavorable.
///
/// C'est le chiffre qui justifie de passer l'épine en section hexagonale. La
/// largeur apparente d'un polygone régulier de circonradius `R` oscille, selon
/// l'angle de vue, entre `2R·cos(π/n)` (vu de face) et `2R` (vu par un sommet) :
///
/// | | mini | maxi | rapport |
/// |---|---|---|---|
/// | carré (n=4) | 1,414 R | 2 R | **1,41** |
/// | hexagone (n=6) | 1,732 R | 2 R | **1,15** |
///
/// À circonradius égal, l'hexagone a donc le **même encombrement maximal** que
/// le carré, mais il est **22 % plus large dans son pire angle** — et c'est le
/// pire angle qui décide de la lisibilité : sous le filtre pixel, un montant qui
/// tombe sous le pixel dans certaines orientations disparaît par intermittence.
/// Un hexagone garde une épaisseur quasi constante d'où qu'on le regarde.
// Valeur de référence : elle documente et verrouille le gain, et n'est consommée
// que par le test qui le mesure — d'où l'attribut.
#[allow(dead_code)]
pub(crate) const HEXA_GAIN_SILHOUETTE: f32 = 1.224_744_9; // √(3)/√(2) = cos30°/cos45°

/// Sommets d'une section **hexagonale régulière**, premier sommet sur `+d`.
///
/// L'orientation n'est pas libre : c'est elle qui met **deux longerons dans le
/// plan (axe, d)**, donc pile en face des deux sommets latéraux du cadre
/// hexagonal du pied. Sans ça la jupe de raccord n'a aucune arête franche.
pub(crate) fn hexa_section(centre: Vec3, d: Vec3, h: Vec3, rayon: f32) -> [Vec3; 6] {
    let mut v = [Vec3::ZERO; 6];
    for k in 0..6 {
        let a = FRAC_PI_3 * k as f32;
        v[k] = centre + d * (rayon * a.cos()) + h * (rayon * a.sin());
    }
    v
}

/// **Treillis conique à section hexagonale** : six longerons courant de `a` à
/// `b`, la section passant du circonradius `rayon_a` à `rayon_b` selon la même
/// loi que [`treillis_conique`] (évasement sur une distance absolue, donc
/// rallonger la poutre n'allonge que la partie fine).
///
/// Version hexagonale de l'épine — voir [`HEXA_GAIN_SILHOUETTE`] pour le pourquoi.
#[allow(clippy::too_many_arguments)]
pub(crate) fn treillis_conique_hexa<P: Peintre>(
    p: &mut P,
    a: Vec3,
    b: Vec3,
    rayon_a: f32,
    rayon_b: f32,
    courbure: f32,
    metal: Color,
    sombre: Color,
) {
    let axe = b - a;
    let long = axe.length();
    if long < 1e-4 {
        return;
    }
    let (_, d, h) = repere(axe);
    let epais = rayon_a.max(rayon_b);
    let baies = ((long / (epais.max(0.4) * 1.3)) as usize).clamp(10, 40);
    // Évasement sur une **distance absolue** (cf. `treillis_conique`).
    let flare = (rayon_a * 9.0).min(long);
    let section = |t: f32| {
        let f = (t * long / flare).min(1.0);
        rayon_b + (rayon_a - rayon_b) * (1.0 - f).powf(courbure)
    };
    let anneau = |k: usize| {
        let t = k as f32 / baies as f32;
        let s = section(t);
        (hexa_section(a + axe * t, d, h, s), s)
    };
    let (mut prev, mut ps) = anneau(0);
    for w in 0..6 {
        p.cylindre(prev[w], prev[(w + 1) % 6], ps * 0.10, sombre); // cadre de base
    }
    for k in 1..=baies {
        let (cur, cs) = anneau(k);
        for i in 0..6 {
            // Longeron à **Ø constant** : il doit garder la même épaisseur que
            // les tiges du cadre hexagonal qu'il rejoint (cf. `treillis_conique`).
            p.cylindre(prev[i], cur[i], rayon_a * LONGERON, metal);
        }
        for w in 0..6 {
            p.cylindre(cur[w], cur[(w + 1) % 6], cs * 0.10, sombre); // cadre
        }
        // Diagonales sur une face sur deux : de quoi trianguler sans tripler le
        // nombre de cylindres par rapport à la version carrée.
        for i in [0usize, 2, 4] {
            p.cylindre(prev[i], cur[(i + 1) % 6], ps * 0.075, sombre);
        }
        prev = cur;
        ps = cs;
    }
}

/// **Tour hexagonale** droite, coaxiale à l'axe Z local, suspendue sous
/// `sommet` : un prisme hexagonal de `etages` niveaux, montants aux six sommets,
/// ceinture à chaque niveau et diagonales alternées.
///
/// À ne pas confondre avec [`treillis_hexagone`], qui est un cadre **plat** dont
/// le plan *contient* l'axe : celui-là se présente de travers à une poutre
/// axiale, et il faut une jupe vrillée pour l'y raccorder. La tour, elle, a sa
/// section **perpendiculaire** à l'axe — donc parallèle à celle de la poutre, et
/// le raccord devient une simple continuation.
///
/// ⚠️ **Aucune ceinture au niveau 0** (le sommet), volontairement : ce niveau est
/// déjà fermé par le cadre de base de la poutre qui s'y pose. En dessiner une
/// deuxième mettrait deux anneaux exactement coplanaires — du z-fighting garanti.
///
/// Les sommets viennent de [`hexa_section`], la **même** fonction que la section
/// de [`treillis_conique_hexa`] : à rayon **et écrasement** égaux, l'accostage est
/// exact par construction, il n'y a rien à faire coïncider à la main.
///
/// ⚠️ `etirement` doit reprendre celui de la pièce sur laquelle la tour se pose —
/// 1 sur la base d'un cône (section régulière), celui de l'embouchure quand elle
/// couronne un [`pavillon_hexagonal`]. Le laisser à 1 sur une embouchure écrasée
/// remettrait exactement le désaccord que l'écrasement progressif a corrigé : les
/// deux sommets portés par X coïncideraient, les quatre obliques non.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tour_hexagonale<P: Peintre>(
    p: &mut P,
    sommet: Vec3,
    rayon: f32,
    hauteur: f32,
    etirement: f32,
    epaisseur: f32,
    etages: usize,
    metal: Color,
    sombre: Color,
) {
    let n = etages.max(1);
    let pas = hauteur / n as f32;
    let haut = Vec3::Y * etirement;
    let niveau =
        |k: usize| hexa_section(sommet - Vec3::Z * (k as f32 * pas), Vec3::X, haut, rayon);
    let mut prev = niveau(0);
    for k in 1..=n {
        let cur = niveau(k);
        for i in 0..6 {
            p.cylindre(prev[i], cur[i], epaisseur, metal); // montant
            p.cylindre(cur[i], cur[(i + 1) % 6], epaisseur * CEINTURE, sombre); // ceinture
        }
        // Diagonale sur une face sur deux, sens inversé d'un étage au suivant :
        // c'est le motif qui fait lire une tour treillis plutôt qu'un fût strié.
        let dec = k % 2;
        for i in 0..3 {
            let a = (2 * i + dec) % 6;
            p.cylindre(prev[a], cur[(a + 1) % 6], epaisseur * DIAGONALE, sombre);
        }
        prev = cur;
    }
}

/// **Pavillon hexagonal** : la corolle qui termine l'épine côté propulsion.
///
/// Le cône ne s'arrête plus sur une tour à section constante — il **continue de
/// s'ouvrir** jusqu'à une large embouchure, dont le bord est un **anneau** :
/// hexagone extérieur *et* hexagone intérieur, reliés par six panneaux radiaux.
/// C'est cet anneau qui portera la propulsion, et c'est pour ça qu'il est évidé :
/// une embouchure pleine ne servirait qu'à cacher les tuyères.
///
/// `etirement_bord` écrase la section selon Y, **à l'embouchure seulement**. Sa
/// conséquence est la raison d'être du paramètre : à 1 l'hexagone est régulier et
/// ses six côtés sont égaux, tandis qu'en dessous les **deux** côtés
/// perpendiculaires à Y gardent leur longueur (ils sont portés par X) alors que les
/// **quatre** côtés obliques raccourcissent. Deux familles d'arêtes, 4 et 2 — la
/// seule façon d'y arriver avec un hexagone.
///
/// ⚠️ **L'écrasement est progressif, et il doit l'être.** Il vaut 1 au col et
/// n'atteint `etirement_bord` qu'au bord. Appliqué d'emblée, il désaccorderait le
/// col de la base du cône, qui est un hexagone **régulier** : les quatre sommets
/// obliques tomberaient à un autre Y et le raccord se verrait. La section *morphe*
/// donc d'un hexagone régulier vers la pierre taillée le long de la corolle.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pavillon_hexagonal<P: Peintre>(
    p: &mut P,
    sommet: Vec3,
    rayon_col: f32,
    rayon_bord: f32,
    hauteur: f32,
    etirement_bord: f32,
    etages: usize,
    metal: Color,
    sombre: Color,
) {
    let n = etages.max(1);
    // Section écrasée en passant un `h` mis à l'échelle : `hexa_section` place le
    // sommet `k` à `d·r·cos + h·r·sin`, donc un `h` plus court écrase la section
    // sans toucher aux deux sommets portés par `d`. Aucune primitive de plus.
    let niveau = |k: usize| {
        let t = k as f32 / n as f32;
        // Évasement **linéaire** : le tracé demandé a des flancs droits, pas une
        // courbe de cloche.
        let r = rayon_col + (rayon_bord - rayon_col) * t;
        let e = etirement_progressif(t, etirement_bord);
        (hexa_section(sommet - Vec3::Z * (hauteur * t), Vec3::X, Vec3::Y * e, r), r)
    };

    let (mut prev, _) = niveau(0);
    for k in 1..=n {
        let (cur, r) = niveau(k);
        let bord = k == n;
        for i in 0..6 {
            p.cylindre(prev[i], cur[i], rayon_col * LONGERON, metal); // longeron de corolle
            // Ceinture d'embouchure plus forte : c'est la **jante** de la corolle,
            // et c'est tout ce qui la termine depuis qu'il n'y a plus d'anneau.
            let (ep, teinte) = if bord { (r * 0.075, metal) } else { (r * 0.055, sombre) };
            p.cylindre(cur[i], cur[(i + 1) % 6], ep, teinte);
        }
        // Diagonales alternées, comme sur la tour : c'est ce motif qui fait lire
        // un treillis et non une tôle pliée.
        let dec = k % 2;
        for i in 0..3 {
            let a = (2 * i + dec) % 6;
            p.cylindre(prev[a], cur[(a + 1) % 6], rayon_col * 0.07, sombre);
        }
        prev = cur;
    }
}

/// Écrasement de la section d'un pavillon à l'avancement `t` (0 au col, 1 au bord).
///
/// Sorti pour être testable : `etirement_progressif(0, _) == 1` **exactement** est
/// ce qui garantit que le col reste un hexagone régulier, donc superposable à la
/// base du cône. C'est la condition d'accostage, et elle était fausse au premier
/// jet — l'écrasement était appliqué d'emblée.
pub(crate) fn etirement_progressif(t: f32, etirement_bord: f32) -> f32 {
    1.0 + (etirement_bord - 1.0) * t
}

/// Une pale solaire : deux lés séparés par une couture centrale, un cadre et
/// des nervures de cellules. Déployée depuis `racine` le long de `deploy`,
/// large de `largeur` selon `largeur_axe`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pale_solaire<P: Peintre>(
    p: &mut P,
    racine: Vec3,
    deploy: Vec3,
    largeur_axe: Vec3,
    longueur: f32,
    largeur: f32,
    cellules: usize,
    couleur: Color,
) {
    let d = deploy.normalize();
    let w = largeur_axe.normalize();
    let bord = Color::new(couleur.r * 0.45, couleur.g * 0.45, couleur.b * 0.45, 1.0);
    let coin = racine - w * (largeur * 0.5);
    let e1 = w * largeur;
    let e2 = d * longueur;
    p.panneau(coin, e1, e2, couleur);
    p.ligne(coin, coin + e1, TRAIT, bord);
    p.ligne(coin + e2, coin + e1 + e2, TRAIT, bord);
    p.ligne(coin, coin + e2, TRAIT, bord);
    p.ligne(coin + e1, coin + e1 + e2, TRAIT, bord);
    p.ligne(racine, racine + e2, TRAIT, bord); // couture entre les deux lés
    for n in 1..cellules {
        let m = coin + e2 * (n as f32 / cellules as f32);
        p.ligne(m, m + e1, TRAIT, bord); // nervures de cellules
    }
}

/// Radiateur thermique : panneau clair rainuré, orienté `deploy`/`largeur_axe`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn radiateur<P: Peintre>(
    p: &mut P,
    base: Vec3,
    deploy: Vec3,
    largeur_axe: Vec3,
    longueur: f32,
    largeur: f32,
    lignes: usize,
    couleur: Color,
    sombre: Color,
) {
    let d = deploy.normalize();
    let w = largeur_axe.normalize();
    let coin = base - w * (largeur * 0.5);
    let e1 = w * largeur;
    let e2 = d * longueur;
    p.panneau(coin, e1, e2, couleur);
    for n in 1..lignes {
        let m = coin + e2 * (n as f32 / lignes as f32);
        p.ligne(m, m + e1, TRAIT, sombre);
    }
}

/// Points par **coin** du contour onigiri (3 coins → 3× ce nombre au total).
/// 8 suffit : ce sont les seules parties courbes, les côtés sont droits.
const ONIGIRI_ARC: usize = 8;

/// Rayon de congé des coins, en fraction du rayon hors-tout. Plus il est petit,
/// plus le triangle est franc. À 0.22 les **côtés restent bien droits** et les
/// coins sont juste adoucis : c'est la lecture « structure triangulaire », pas
/// « tube mou ».
///
/// Exposé au module : l'empilement en triforce a besoin du rayon de congé pour
/// calculer l'écartement sans recouvrement (un triangle **congé** est un
/// triangle nu *gonflé* de ce rayon, il ne peut donc pas se toucher pointe
/// contre pointe comme un triangle à coins vifs).
pub(crate) const ONIGIRI_FILET: f32 = 0.22;

/// Contour d'une section onigiri de rayon hors-tout `rayon`, tourné de `spin`,
/// dans le plan Z = `z`. **Vrai triangle à coins congés** : trois arcs de
/// cercle reliés par trois **segments droits** (les extrémités d'arcs
/// consécutifs sont alignées, la corde entre elles *est* le côté du triangle).
///
/// Le point le plus éloigné du centre vaut exactement `rayon`, dans la
/// direction de chaque coin (`spin + k·120°`) — c'est ce qui rend l'empilement
/// en triforce calculable sans marge empirique.
fn contour_onigiri(pied: Vec3, rayon: f32, spin: f32, z: f32) -> Vec<Vec3> {
    let rho = rayon * ONIGIRI_FILET; // rayon du congé
    let dv = rayon - rho; // centre → centre de congé (max = dv + rho = rayon)
    let mut pts = Vec::with_capacity(3 * ONIGIRI_ARC);
    for k in 0..3 {
        let ak = spin + TAU * k as f32 / 3.0;
        let c = vec3(dv * ak.cos(), dv * ak.sin(), 0.0);
        for i in 0..ONIGIRI_ARC {
            // Chaque coin balaie 120° (angle extérieur d'un triangle équilatéral).
            let t = i as f32 / (ONIGIRI_ARC - 1) as f32;
            let a = ak - FRAC_PI_3 + t * 2.0 * FRAC_PI_3;
            pts.push(pied + c + vec3(rho * a.cos(), rho * a.sin(), z));
        }
    }
    pts
}

/// Prisme à section **onigiri** (triangle à coins congés) extrudé le long de
/// +Z, de `pied` à `pied + Z·longueur`, fermé aux deux bouts.
///
/// Sortie en triangles bruts plutôt qu'en primitives composées : aucune
/// primitive du [`Peintre`] n'a de section non circulaire.
pub(crate) fn prisme_onigiri<P: Peintre>(
    p: &mut P,
    pied: Vec3,
    longueur: f32,
    rayon: f32,
    spin: f32,
    couleur: Color,
) {
    if longueur.abs() < 1e-4 || rayon < 1e-4 {
        return;
    }
    let n = 3 * ONIGIRI_ARC;
    let contour = |z: f32| contour_onigiri(pied, rayon, spin, z);
    let mut sommets = contour(0.0);
    sommets.extend(contour(longueur));
    sommets.push(pied); // centre de l'embout bas
    sommets.push(pied + Vec3::Z * longueur); // centre de l'embout haut
    let (nb, cb, ch) = (n as u16, 2 * n as u16, 2 * n as u16 + 1);

    let mut indices = Vec::with_capacity(n * 12);
    for k in 0..nb {
        let kn = (k + 1) % nb;
        // Flanc (deux triangles par segment) puis les deux embouts en éventail.
        indices.extend_from_slice(&[k, k + nb, kn + nb, k, kn + nb, kn]);
        indices.extend_from_slice(&[cb, kn, k]);
        indices.extend_from_slice(&[ch, k + nb, kn + nb]);
    }
    p.triangles(&sommets, &indices, couleur);
}

/// **Nacelle de fret** d'échelle vaisseau (ISV) : un long conteneur à section
/// **onigiri** (triangle à coins congés), posé depuis `pied` le long de +Z. La
/// section triangulaire n'est pas décorative : c'est la forme qui **tient** (le
/// triangle ne se déforme pas sous charge) et celle qui s'empaquette sans vides
/// autour d'une épine, là où des cylindres gaspilleraient tout l'entre-deux.
///
/// `spin` tourne la section autour de son axe : c'est ainsi qu'on choisit ce
/// qu'on présente aux voisines (un coin ou un côté plat). Habillage : trois
/// rails d'arête sur les coins et deux collerettes de bout, qui donnent
/// l'échelle et cassent le fuselé nu.
pub(crate) fn nacelle_cargo<P: Peintre>(
    p: &mut P,
    pied: Vec3,
    longueur: f32,
    rayon: f32,
    spin: f32,
    corps: Color,
    bague: Color,
) {
    if longueur < 1e-4 || rayon < 1e-4 {
        return;
    }
    prisme_onigiri(p, pied, longueur, rayon, spin, corps);

    // Collerettes de bout : mêmes section et orientation, à peine plus larges →
    // une arête nette au lieu d'un tube qui s'arrête dans le vide.
    //
    // Elles **débordent** hors du corps (`deb`) tout en s'y **enfonçant**
    // (`ec`) : aucune de leurs faces n'est alors coplanaire avec un bout du
    // corps. Posées à ras, les deux faces se disputaient le même plan et la
    // collerette clignotait à travers le conteneur (z-fighting) — même piège,
    // et même remède, que les embouts de module (`EMBOUT_*` dans `composant`).
    let ec = (longueur * 0.05).min(rayon * 0.6);
    let deb = ec * 0.35;
    prisme_onigiri(p, pied - Vec3::Z * deb, ec + deb, rayon * 1.06, spin, bague);
    prisme_onigiri(
        p,
        pied + Vec3::Z * (longueur - ec),
        ec + deb,
        rayon * 1.06,
        spin,
        bague,
    );

    // Rails d'arête : dans l'axe des trois coins, au rayon hors-tout.
    for k in 0..3 {
        let t = spin + TAU * k as f32 / 3.0;
        let r = rayon * 0.99;
        let o = vec3(r * t.cos(), r * t.sin(), 0.0);
        p.cylindre(pied + o + Vec3::Z * ec, pied + o + Vec3::Z * (longueur - ec), rayon * 0.07, bague);
    }
}

/// Rayon **inscrit** d'une section onigiri (centre → milieu d'un côté plat),
/// pour un rayon hors-tout donné. C'est la cote qui sert dès qu'on veut poser
/// quelque chose **contre un côté** : ferrure d'attache, jeu par rapport à une
/// épine qui passe à côté, contact entre deux nacelles côte à côte.
pub(crate) fn onigiri_inscrit(rayon: f32) -> f32 {
    rayon * (0.5 + 0.5 * ONIGIRI_FILET)
}

/// Demi-angle des sommets d'hexagone sur l'arc de congé, mesuré depuis l'axe du
/// coin. **Pas 60°** (les points de tangence) : une corde tendue de tangence à
/// tangence coupe l'arc et **rentre dans la coque** de `ρ·(1 − cos 60°) = ρ/2`
/// au droit du coin. À 30° la corde ne mord plus que de `ρ·(1 − cos 30°)`, sept
/// fois moins, et le facteur d'échelle de l'armature suffit à la ressortir.
const ONIGIRI_HEX_PHI: f32 = FRAC_PI_6;

/// Les **six** sommets du contour hexagonal qui ceinture une section onigiri.
///
/// Ordre : `A0 B0 A1 B1 A2 B2`. Les segments `B_k → A_{k+1}` longent les côtés
/// **plats** (les longs), les segments `A_k → B_k` coupent les **coins** (les
/// courts, sept fois plus courts).
///
/// **Tout le contour est en dehors** de la section de rayon `rayon / echelle`,
/// à condition que `echelle` dépasse les deux seuils que voici (c'est le sens
/// de [`onigiri_hex_echelle_mini`]) :
/// - le long des faces, la corde passe à `echelle·[0,5(1−f) + f·cos(60° − φ)]`
///   contre `0,5 + f/2` pour la coque ;
/// - en travers d'un coin, à `echelle·[1 − f + f·cos φ]` contre `1`.
pub(crate) fn onigiri_hexagone(rayon: f32, spin: f32, z: f32) -> [Vec3; 6] {
    let rho = rayon * ONIGIRI_FILET;
    let dv = rayon - rho;
    let mut v = [Vec3::ZERO; 6];
    for k in 0..3 {
        let ak = spin + TAU * k as f32 / 3.0;
        let c = vec3(dv * ak.cos(), dv * ak.sin(), z);
        for j in 0..2 {
            let a = ak + if j == 0 { -ONIGIRI_HEX_PHI } else { ONIGIRI_HEX_PHI };
            v[2 * k + j] = c + vec3(rho * a.cos(), rho * a.sin(), 0.0);
        }
    }
    v
}

/// Échelle **minimale** d'une armature hexagonale pour qu'aucun de ses segments
/// ne plonge dans la coque. Le maximum des deux contraintes du §doc de
/// [`onigiri_hexagone`] — en pratique c'est le **coin** qui commande.
pub(crate) fn onigiri_hex_echelle_mini() -> f32 {
    let f = ONIGIRI_FILET;
    let phi = ONIGIRI_HEX_PHI;
    let par_coin = 1.0 / (1.0 - f + f * phi.cos());
    let par_face = (0.5 + 0.5 * f) / (0.5 * (1.0 - f) + f * (FRAC_PI_3 - phi).cos());
    par_coin.max(par_face)
}

/// Demi-largeur du **côté plat** d'une section onigiri (du milieu du côté à son
/// extrémité). Sert à répartir des ferrures sur une face sans déborder.
pub(crate) fn onigiri_demi_face(rayon: f32) -> f32 {
    (rayon - rayon * ONIGIRI_FILET) * 3.0_f32.sqrt() * 0.5
}

/// **Tube creux** (anneau extrudé) de `pied` le long de +Z : paroi extérieure,
/// paroi **intérieure**, et les deux couronnes de bout qui les relient.
///
/// Un `cylindre` ne convient pas : il est plein et fermé, donc tout ce qui
/// passerait dans l'axe s'y enfoncerait. Ici l'alésage est vraiment vide — ce
/// qu'il faut pour un collier qui tourne **autour** d'une épine sans la
/// toucher.
pub(crate) fn tube<P: Peintre>(
    p: &mut P,
    pied: Vec3,
    longueur: f32,
    r_ext: f32,
    r_int: f32,
    couleur: Color,
) {
    if longueur < 1e-4 || r_ext <= r_int || r_int < 0.0 {
        return;
    }
    const N: usize = 28;
    let mut s: Vec<Vec3> = Vec::with_capacity(4 * N);
    for k in 0..N {
        let a = TAU * k as f32 / N as f32;
        let (c, si) = (a.cos(), a.sin());
        s.push(pied + vec3(r_ext * c, r_ext * si, 0.0)); // 0 : ext bas
        s.push(pied + vec3(r_ext * c, r_ext * si, longueur)); // 1 : ext haut
        s.push(pied + vec3(r_int * c, r_int * si, 0.0)); // 2 : int bas
        s.push(pied + vec3(r_int * c, r_int * si, longueur)); // 3 : int haut
    }
    let mut ix: Vec<u16> = Vec::with_capacity(N * 24);
    for k in 0..N {
        let a = (4 * k) as u16;
        let b = (4 * ((k + 1) % N)) as u16;
        let (eb, et, ib, it) = (a, a + 1, a + 2, a + 3);
        let (eb2, et2, ib2, it2) = (b, b + 1, b + 2, b + 3);
        // Paroi extérieure (normale sortante) et intérieure (normale rentrante,
        // donc enroulement inversé : c'est elle qui borde l'alésage).
        ix.extend_from_slice(&[eb, et, et2, eb, et2, eb2]);
        ix.extend_from_slice(&[ib, it2, it, ib, ib2, it2]);
        // Couronnes de bout, qui ferment l'épaisseur de paroi.
        ix.extend_from_slice(&[eb, ib2, ib, eb, eb2, ib2]);
        ix.extend_from_slice(&[et, it, it2, et, it2, et2]);
    }
    p.triangles(&s, &ix, couleur);
}

/// Une **tuile hexagonale à épaisseur**, posée à plat sur une surface.
///
/// Écrite pour le bardage du tore d'habitat, et **pas** dérivée de
/// `BouclierThermique` : celui-ci est taillé pour l'ISV — des rangs de plaques
/// enfilés le long d'un axe, imbriqués dans le sens du flux thermique d'une
/// tuyère. Rien de tout ça ne transpose sur une surface à double courbure, et
/// forcer la ressemblance aurait donné une pièce qui ment sur sa raison d'être.
///
/// La tuile est un **prisme hexagonal** : face supérieure pleine, six flancs,
/// pas de fond (il est plaqué contre la coque et jamais vu). `normale` porte
/// l'épaisseur, `tangente` oriente l'hexagone dans le plan de la surface —
/// c'est elle qui permet d'aligner les rangs plutôt que de les laisser tourner
/// avec la courbure.
///
/// **À plat**, pas en écaille : sur une surface courbe fermée, une lèvre
/// relevée n'a pas de « sens du flux » à suivre. Le relief vient de
/// l'épaisseur et du contraste dessus/flanc, pas d'un recouvrement.
///
/// Les sommets sont **accumulés** dans `dessus` et `flancs` plutôt qu'émis :
/// un bardage compte plus d'un millier de tuiles, et une paire d'appels par
/// tuile ferait autant de lots que de tuiles.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tuile_hexagonale(
    centre: Vec3,
    normale: Vec3,
    tangente: Vec3,
    rayon: f32,
    epaisseur: f32,
    dessus: &mut (Vec<Vec3>, Vec<u16>),
    flancs: &mut (Vec<Vec3>, Vec<u16>),
) {
    let n = normale.normalize_or_zero();
    let t = (tangente - n * n.dot(tangente)).normalize_or_zero();
    if n == Vec3::ZERO || t == Vec3::ZERO {
        return;
    }
    let b = n.cross(t);
    let haut = centre + n * epaisseur;

    // --- face supérieure : éventail depuis son centre ---
    let base = dessus.0.len() as u16;
    dessus.0.push(haut);
    for k in 0..6 {
        let a = FRAC_PI_3 * k as f32;
        dessus.0.push(haut + (t * a.cos() + b * a.sin()) * rayon);
    }
    for k in 0..6u16 {
        dessus.1.extend_from_slice(&[base, base + 1 + k, base + 1 + (k + 1) % 6]);
    }

    // --- flancs : six quads entre le contour haut et le contour bas ---
    let base = flancs.0.len() as u16;
    for k in 0..6 {
        let a = FRAC_PI_3 * k as f32;
        let radial = (t * a.cos() + b * a.sin()) * rayon;
        flancs.0.push(haut + radial);
        flancs.0.push(centre + radial);
    }
    for k in 0..6u16 {
        let (h0, b0) = (base + 2 * k, base + 2 * k + 1);
        let (h1, b1) = (base + 2 * ((k + 1) % 6), base + 2 * ((k + 1) % 6) + 1);
        flancs.1.extend_from_slice(&[h0, b0, b1, h0, b1, h1]);
    }
}
