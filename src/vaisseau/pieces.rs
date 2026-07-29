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
use std::f32::consts::{FRAC_PI_3, TAU};

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
fn prisme_onigiri<P: Peintre>(
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
