//! **Jante** : portion de tore en **canal ouvert**, façon jante de vélo — une
//! paroi qui court du flanc à l'autre en passant par le bord extérieur (le
//! « pneu »), et un **vide côté moyeu** : rien ne referme le côté qui regarde
//! l'axe, on voit au travers depuis l'intérieur de l'anneau.
//!
//! Même surface paramétrique que [`super::tore`] ([`surface_tore`], partagée
//! via `commun.rs`), mais habillée autrement, et en quatre parties :
//!
//! - la **paroi** (tread + la plus grande partie des flancs), avec une vraie
//!   **épaisseur** — coque externe et coque interne, décalées le long de la
//!   normale locale ;
//! - le **sol**, une bande verte posée à l'intérieur du canal, sur la face
//!   interne du « pneu » — le sol (provisoire) de la mégastructure ;
//! - la **ville**, un motif géométrique bruité (grille de petits
//!   parallélépipèdes, hauteur et présence tirées cellule par cellule) posé
//!   sur ce sol — une silhouette urbaine, provisoire elle aussi ;
//! - le **vitrage**, aux deux derniers tronçons de la paroi (près du vide côté
//!   moyeu) : une paroi de verre translucide tenue par une grille de meneaux,
//!   qui suit la même courbure que la paroi pleine, à la place de la coque.

use crate::vaisseau::peintre::Peintre;
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

use super::commun::*;

/// Sommets et indices d'une bande `[v0, v1]`, sur l'arc `[0, arc]` — le pendant
/// de `tore::peau`, mais borné en `u` au lieu de faire le tour complet.
fn peau_arc(
    rayon_majeur: f32,
    rayon_mineur: f32,
    segments: usize,
    anneaux: usize,
    arc: f32,
    v0: f32,
    v1: f32,
) -> (Vec<Vec3>, Vec<u16>) {
    let ns = segments.max(1);
    let nv = anneaux.max(1);
    let mut sommets = Vec::with_capacity((ns + 1) * (nv + 1));
    for i in 0..=ns {
        let u = arc * i as f32 / ns as f32;
        for j in 0..=nv {
            let v = v0 + (v1 - v0) * j as f32 / nv as f32;
            sommets.push(surface_tore(rayon_majeur, rayon_mineur, u, v).0);
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

/// Émet une bande **des deux côtés** : macroquad ne double-face pas
/// (`miroir.rs`), donc toute paroi qu'on doit pouvoir regarder depuis les deux
/// côtés (l'extérieur de la station et l'intérieur du canal) est émise deux
/// fois, la seconde à enroulement inversé.
fn bande_double_face<P: Peintre>(p: &mut P, sommets: &[Vec3], indices: &[u16], exterieur: Color, interieur: Color) {
    p.triangles(sommets, indices, exterieur);
    let envers: Vec<u16> = indices.chunks_exact(3).flat_map(|t| [t[0], t[2], t[1]]).collect();
    p.triangles(sommets, &envers, interieur);
}

const COQUE_EXT: Color = Color { r: 0.80, g: 0.81, b: 0.85, a: 1.0 };
const COQUE_INT: Color = Color { r: 0.30, g: 0.31, b: 0.35, a: 1.0 };

/// Épaisseur de la paroi, en fraction de `rayon_mineur` — absolue en
/// proportion, comme les autres épaisseurs de coque du parc (`tore.rs`,
/// `TUILE_EPAISSEUR`), pour qu'elle suive l'échelle de la pièce plutôt que de
/// rester fixe quand on l'agrandit.
const EPAISSEUR_FRAC: f32 = 0.15;

/// Portion de la section (en radians de `v`, depuis le bord du vide côté
/// moyeu) que le vitrage occupe à **chaque** bout de la paroi, à la place de
/// la coque pleine — **doublée** (0,35 → 0,7) sur demande : la portion de
/// coque blanche adjacente au vide devient du vitrage, la paroi pleine
/// (« blanche ») recule d'autant. `v_paroi` porte le calcul, donc rien
/// d'autre à toucher : la frontière coque/vitrage, le sol qui la rejoint et
/// les tests qui la lisent suivent tous automatiquement.
const VITRAGE_SPAN: f32 = 0.7;

/// Bornes en `v` de la coque pleine (tread + la plus grande partie des
/// flancs), une fois le vitrage retranché à chaque bout. Fonction plutôt que
/// formule recopiée : le dessin **et** les tests la lisent d'ici — sinon la
/// bordure entre coque et vitrage pourrait diverger en silence entre les deux.
fn v_paroi(ouverture: f32) -> (f32, f32) {
    let v_max = PI - ouverture;
    // Le vitrage ne mange jamais plus de 60 % de la demi-section : au-delà, il
    // n'y aurait plus de « paroi » à proprement parler, seulement du vitrage.
    let vitrage = VITRAGE_SPAN.min(v_max * 0.6);
    (-(v_max - vitrage), v_max - vitrage)
}

/// Bornes en `v` des deux zones vitrées — côté négatif puis positif.
fn v_vitrages(ouverture: f32) -> [(f32, f32); 2] {
    let v_max = PI - ouverture;
    let (pv0, pv1) = v_paroi(ouverture);
    [(-v_max, pv0), (pv1, v_max)]
}

pub(super) fn dessiner<P: Peintre>(
    p: &mut P,
    rayon_majeur: f32,
    rayon_mineur: f32,
    arc: f32,
    ouverture: f32,
    segments: usize,
    anneaux: usize,
) {
    let (v0, v1) = v_paroi(ouverture);
    let epaisseur = rayon_mineur * EPAISSEUR_FRAC;

    // La paroi, avec une vraie épaisseur : coque externe (à `rayon_mineur`,
    // comme avant) et coque interne, à un rayon mineur **plus petit** —
    // `surface_tore` dérive un point exactement selon `rayon_mineur` le long
    // de sa propre normale locale, donc réduire ce seul paramètre décale la
    // coque interne d'exactement `epaisseur` vers l'intérieur, sans qu'aucun
    // second calcul de décalage ne puisse diverger du premier.
    let (so, io) = peau_arc(rayon_majeur, rayon_mineur, segments, anneaux, arc, v0, v1);
    p.triangles(&so, &io, COQUE_EXT);
    let (si, ii) = peau_arc(rayon_majeur, (rayon_mineur - epaisseur).max(0.01), segments, anneaux, arc, v0, v1);
    // Enroulement inversé : la coque interne doit regarder **vers l'intérieur**
    // du canal (normale opposée à la coque externe), pas la répéter.
    let ii_rev: Vec<u16> = ii.chunks_exact(3).flat_map(|t| [t[0], t[2], t[1]]).collect();
    p.triangles(&si, &ii_rev, COQUE_INT);

    sol(p, rayon_majeur, rayon_mineur, epaisseur, arc, segments, anneaux, ouverture);
    ville(p, rayon_majeur, rayon_mineur, epaisseur, arc, ouverture);

    for (vv0, vv1) in v_vitrages(ouverture) {
        vitrage(p, rayon_majeur, rayon_mineur, arc, segments, vv0, vv1);
    }
}

/// Retrait du sol par rapport à la face interne de la paroi, en fraction de
/// l'épaisseur — assez pour se voir nettement **au-dessus** d'elle (le sol
/// « commence plus haut », en léger surplomb), pas seulement assez pour ne
/// pas coïncider avec elle (le piège des surfaces confondues qui clignotent,
/// cf. `hangar.rs`).
const SOL_RETRAIT_FRAC: f32 = 0.6;
/// Vert de terrain provisoire — voir la note de tête de fichier.
const SOL_VERT: Color = Color { r: 0.18, g: 0.52, b: 0.20, a: 1.0 };
const SOL_VERT_DESSOUS: Color = Color { r: 0.10, g: 0.30, b: 0.12, a: 1.0 };

/// Rayon mineur du sol (et de la ville qui s'y dresse) : en retrait de la
/// face interne de la paroi — source unique lue par [`sol`] et [`ville`], pour
/// que la ville reste **posée sur** le sol plutôt que de dériver de son propre
/// calcul de hauteur.
fn r_sol(rayon_mineur: f32, epaisseur: f32) -> f32 {
    (rayon_mineur - epaisseur * (1.0 + SOL_RETRAIT_FRAC)).max(0.01)
}

/// **Le sol de la mégastructure** : une bande verte posée à l'intérieur du
/// canal, contre la face interne du « pneu », qui s'étend jusqu'aux deux
/// zones vitrées ([`v_paroi`]) — le sol rejoint le vitrage, il ne flotte pas
/// isolé au centre du tread.
///
/// Une station en anneau tourne pour simuler la gravité : le sol, c'est la
/// face **extérieure** du tube (le pneu), et « en dessous » y pointe vers
/// l'axe de rotation. Ce que les habitants foulent est donc la face
/// **interne** de la coque externe — la même surface que `bande_double_face`
/// dessine déjà côté `COQUE_INT`, mais en retrait pour se distinguer d'elle
/// comme une pièce à part, et non un simple changement de couleur.
///
/// Provisoire, et assumé comme tel : vert plat, sans texture — un terrain à
/// habiller viendra plus tard.
///
/// `anneaux` **doit** suivre celui de la paroi (`dessiner`), pas rester à 1 :
/// avec un seul anneau, `peau_arc` ne pose des sommets qu'aux deux bords
/// `v0`/`v1` (près du vitrage), et relie les deux par une corde **droite** —
/// elle saute entièrement `v = 0`, le pneu, là où la ville se pose. Le sol
/// rendu ne passait jamais par là où les bâtiments sont : de leur point de
/// vue, il n'y avait pas de sol du tout à cet endroit, seulement la coque.
#[allow(clippy::too_many_arguments)]
fn sol<P: Peintre>(
    p: &mut P,
    rayon_majeur: f32,
    rayon_mineur: f32,
    epaisseur: f32,
    arc: f32,
    segments: usize,
    anneaux: usize,
    ouverture: f32,
) {
    let (v0, v1) = v_paroi(ouverture);
    let (s, i) = peau_arc(rayon_majeur, r_sol(rayon_mineur, epaisseur), segments, anneaux.max(1), arc, v0, v1);
    // La face à enroulement d'origine (`exterieur`, ici en premier) est
    // celle dont la normale pointe vers l'axe (comme `COQUE_EXT` pour la
    // coque) : c'est elle qui regarde vers l'intérieur du canal, où se
    // tiennent les habitants — malgré le nom du paramètre.
    bande_double_face(p, &s, &i, SOL_VERT, SOL_VERT_DESSOUS);
}

// --- Ville : un motif géométrique bruité, posé sur le sol -----------------

/// Bruit entier déterministe dans `[0, 1)` — un hachage entier (dispersion à
/// la SplitMix, cf. `planete/terrain.rs::hash` pour le même principe), pas une
/// dépendance de bruit externe. `sel` distingue plusieurs tirages (présence,
/// hauteur, teinte) sur la **même** cellule sans qu'ils se corrèlent.
fn bruit(x: u32, y: u32, sel: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(sel.wrapping_mul(2_246_822_519));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// Cellules de la grille de la ville, le long de l'arc et à travers le sol.
const VILLE_CASES_U: usize = 20;
const VILLE_CASES_V: usize = 5;
/// Demi-largeur de la bande où la ville se pose (en `v`, autour du « pneu »,
/// `v = 0`) — **étroite et centrée**, sans rapport avec l'étendue du sol
/// jusqu'au vitrage (`v_paroi`). Sur les flancs, la section se courbe et la
/// normale locale s'incline avec elle ; y poser des bâtiments les ferait
/// pencher de plus en plus vers le mur qu'ils longent, comme collés à
/// l'intérieur de la jante plutôt que debout sur un terrain plat.
const VILLE_DEMI_LARGEUR: f32 = 0.5;
/// Probabilité qu'une cellule porte un bâtiment — sous 1 pour laisser des
/// rues, sans quoi la grille se lirait comme un mur plutôt qu'une ville.
const VILLE_DENSITE: f32 = 0.55;
/// Emprise d'un bâtiment dans sa cellule (le reste est la rue qui l'entoure).
const VILLE_EMPRISE: f32 = 0.6;
/// Hauteur d'un bâtiment, en fraction de `rayon_mineur` — bornes basse et
/// haute, tirées par cellule. **Abaissée** (0,15–0,9 → 0,05–0,25) : à l'ancien
/// gabarit, les plus hauts bâtiments dépassaient largement le pied du
/// vitrage, une silhouette plus proche d'un mur que d'une ville vue de loin.
const VILLE_HAUTEUR: (f32, f32) = (0.05, 0.25);
/// Encastrement de la base des bâtiments dans le sol, en fraction de
/// `rayon_mineur` — la base est légèrement enfoncée à travers la fine
/// coque du sol, **côté coque** (pas côté canal), donc masquée par elle :
/// sans lui, base et sol sont exactement coplanaires et clignotent
/// (z-fighting). Enfoncer plutôt que surélever : surélever côté canal
/// ouvrirait un vide visible entre le bâtiment et le sol — l'exact inverse
/// de « posé sur, en contact avec » le sol.
const VILLE_ENCASTREMENT_FRAC: f32 = 0.03;
/// Teintes de la ville — délibérément **chaudes** (sable, terre cuite),
/// à l'opposé des gris froids de la coque (`COQUE_EXT`, `COQUE_INT`) : des
/// teintes trop proches se confondaient visuellement avec la paroi, la
/// ville lisait comme un prolongement de la structure plutôt que comme un
/// habitat distinct posé sur le sol.
const VILLE_TEINTE: Color = Color { r: 0.72, g: 0.62, b: 0.48, a: 1.0 };
const VILLE_TEINTE_SOMBRE: Color = Color { r: 0.45, g: 0.33, b: 0.22, a: 1.0 };

/// **La ville** : un motif géométrique — une grille de petits parallélépipèdes
/// de hauteur et de présence bruitées, façon silhouette urbaine vue de loin —
/// posé sur le sol, à l'intérieur de la mégastructure.
///
/// Une grille pure (bâtiment partout) lirait comme un mur crénelé, pas une
/// ville : `VILLE_DENSITE` en saute une partie (des rues), et la hauteur varie
/// cellule par cellule ([`bruit`]) plutôt que d'être uniforme.
fn ville<P: Peintre>(p: &mut P, rayon_majeur: f32, rayon_mineur: f32, epaisseur: f32, arc: f32, ouverture: f32) {
    let r = r_sol(rayon_mineur, epaisseur);
    // Bande étroite et centrée sur `v = 0`, jamais plus large que la paroi
    // elle-même (au cas où `ouverture` la resserrerait davantage).
    let (pv0, pv1) = v_paroi(ouverture);
    let demi = VILLE_DEMI_LARGEUR.min((pv1 - pv0) * 0.5);
    let (v0, v1) = (-demi, demi);
    // Taille d'une cellule, en unités monde — approximative (la métrique de
    // la section n'est pas euclidienne en toute rigueur), largement assez
    // juste pour une silhouette de bâtiments.
    let pas_u = (arc / VILLE_CASES_U as f32) * rayon_majeur;
    let pas_v = ((v1 - v0) / VILLE_CASES_V as f32) * rayon_mineur;

    for i in 0..VILLE_CASES_U {
        for j in 0..VILLE_CASES_V {
            if bruit(i as u32, j as u32, 0) > VILLE_DENSITE {
                continue;
            }
            let u = arc * (i as f32 + 0.5) / VILLE_CASES_U as f32;
            let v = v0 + (v1 - v0) * (j as f32 + 0.5) / VILLE_CASES_V as f32;
            let point_sol = surface_tore(rayon_majeur, r, u, v).0;
            // Le repère (« haut », les deux tangentes) vient de `v = 0` — le
            // point de la section le plus loin de l'axe, pas de la position
            // propre du bâtiment — pour que **tous** les bâtiments d'une même
            // colonne se dressent bien droit et parallèles, plutôt que
            // chacun perpendiculairement à son propre bout de paroi courbée.
            let (_, n0) = surface_tore(rayon_majeur, r, u, 0.0);
            let tangente_u = vec3(-u.sin(), 0.0, u.cos());
            let tangente_v = n0.cross(tangente_u).normalize_or_zero();
            // Encastré dans le sol, côté coque (`+n0`) : la base est cachée
            // derrière la surface visible du sol, en contact avec elle, sans
            // ouvrir de vide côté canal (voir `VILLE_ENCASTREMENT_FRAC`).
            let centre = point_sol + n0 * (rayon_mineur * VILLE_ENCASTREMENT_FRAC);

            let (hmin, hmax) = VILLE_HAUTEUR;
            let hauteur = rayon_mineur * (hmin + (hmax - hmin) * bruit(i as u32, j as u32, 1));
            let couleur = if bruit(i as u32, j as u32, 2) < 0.3 { VILLE_TEINTE_SOMBRE } else { VILLE_TEINTE };

            // La gravité de la station pousse « vers l'extérieur » (le pneu
            // est le sol) : un bâtiment pousse donc vers l'**intérieur** du
            // canal, à l'opposé de la normale sortante de `surface_tore`.
            batiment(p, centre, -n0, tangente_u, tangente_v, pas_u * VILLE_EMPRISE, pas_v * VILLE_EMPRISE, hauteur, couleur);
        }
    }
}

/// Un petit parallélépipède orienté, posé sur la surface : `centre` est son
/// pied, `haut` la direction dans laquelle il s'élève, `tangente_u`/
/// `tangente_v` les deux directions de sa base (perpendiculaires à `haut`).
/// Toit **et** flancs — pas de plancher, invisible sous le sol qui le porte.
#[allow(clippy::too_many_arguments)]
fn batiment<P: Peintre>(
    p: &mut P,
    centre: Vec3,
    haut: Vec3,
    tangente_u: Vec3,
    tangente_v: Vec3,
    largeur: f32,
    profondeur: f32,
    hauteur: f32,
    couleur: Color,
) {
    let (hu, hv) = (tangente_u * (largeur * 0.5), tangente_v * (profondeur * 0.5));
    let sommet = centre + haut * hauteur;
    let bas = [centre - hu - hv, centre + hu - hv, centre + hu + hv, centre - hu + hv];
    let haut4 = [sommet - hu - hv, sommet + hu - hv, sommet + hu + hv, sommet - hu + hv];

    let mut s = Vec::with_capacity(20);
    let mut idx = Vec::with_capacity(36);
    // Toit.
    s.extend_from_slice(&haut4);
    idx.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    // Quatre flancs.
    for k in 0..4 {
        let l = (k + 1) % 4;
        let base = s.len() as u16;
        s.extend_from_slice(&[bas[k], bas[l], haut4[l], haut4[k]]);
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    p.triangles(&s, &idx, couleur);
}

const VERRE: Color = Color { r: 0.55, g: 0.78, b: 0.88, a: 0.35 };
const GRILLE: Color = Color { r: 0.28, g: 0.29, b: 0.33, a: 1.0 };
/// Pas des meneaux le long de l'arc, en unités monde — même idée que
/// `tore::MENEAU_PAS` : un vitrage courbe continu ne se fabrique pas d'un
/// tenant, il se pose en panneaux, et c'est ce pas qui donne son échelle
/// apparente au canal.
const MENEAU_PAS: f32 = 2.4;
/// Section d'un meneau, en fraction de `rayon_mineur` — **réduite de moitié**
/// (0,05 → 0,025) sur jugement visuel : la grille écrasait le verre.
const MENEAU_SECTION_FRAC: f32 = 0.025;
/// Nombre de segments d'un meneau le long de la section : il suit la courbure
/// du canal, il ne la coupe pas en corde.
const MENEAU_PAS_ARC: usize = 4;

/// **Le vitrage** : le dernier tronçon de la paroi, d'un côté, remplacé par une
/// paroi de verre translucide tenue par une grille de meneaux — au lieu de la
/// coque pleine. Suit la même [`surface_tore`] que la paroi : la grille et le
/// verre épousent donc exactement sa courbure, sur `[v0, v1]`.
fn vitrage<P: Peintre>(p: &mut P, rayon_majeur: f32, rayon_mineur: f32, arc: f32, segments: usize, v0: f32, v1: f32) {
    let (s, i) = peau_arc(rayon_majeur, rayon_mineur, segments, 2, arc, v0, v1);
    bande_double_face(p, &s, &i, VERRE, VERRE);

    // `ligne`, pas `cylindre` : un meneau est un fil décoratif fin, pas une
    // poutre — un tronc de cylindre plein coûte dix fois plus de triangles
    // (`SEGMENTS = 20` côtés révolus, cf. `maillage.rs`) qu'une croix de deux
    // panneaux, pour une différence invisible à l'échelle d'un meneau. Avec
    // six tronçons de jante, chacun deux zones vitrées, cet écart s'est avéré
    // être le premier poste de coût du complexe Elysium — voir
    // `docs/suivi/stations.md`.
    let saillie = rayon_mineur * MENEAU_SECTION_FRAC;
    let nervure_section = |p: &mut P, u: f32| {
        for k in 0..MENEAU_PAS_ARC {
            let a = v0 + (v1 - v0) * k as f32 / MENEAU_PAS_ARC as f32;
            let b = v0 + (v1 - v0) * (k + 1) as f32 / MENEAU_PAS_ARC as f32;
            let (pa, na) = surface_tore(rayon_majeur, rayon_mineur, u, a);
            let (pb, nb) = surface_tore(rayon_majeur, rayon_mineur, u, b);
            p.ligne(pa + na * saillie, pb + nb * saillie, saillie * 2.0, GRILLE);
        }
    };
    // Meneaux le long de l'arc, à intervalle régulier.
    let n_meneaux = ((rayon_majeur * arc) / MENEAU_PAS).round().max(2.0) as usize;
    for k in 0..=n_meneaux {
        nervure_section(p, arc * k as f32 / n_meneaux as f32);
    }
    // Lèvres : les deux bords du vitrage, le long de l'arc.
    for v in [v0, v1] {
        for k in 0..segments {
            let ua = arc * k as f32 / segments as f32;
            let ub = arc * (k + 1) as f32 / segments as f32;
            let (pa, na) = surface_tore(rayon_majeur, rayon_mineur, ua, v);
            let (pb, nb) = surface_tore(rayon_majeur, rayon_mineur, ub, v);
            p.ligne(pa + na * saillie, pb + nb * saillie, saillie * 1.3 * 2.0, GRILLE);
        }
    }
}

/// Coût de rendu, proportionnel à la longueur d'arc réellement couverte — pas à
/// la circonférence entière comme [`super::tore::cout`], dont il reprend le
/// facteur (`0.8` par tour complet, donc `0.8 / TAU` par radian).
pub(super) fn cout(rayon_majeur: f32, arc: f32) -> f32 {
    (rayon_majeur * arc * (0.8 / TAU)).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Compteur {
        tris: Vec<[Vec3; 3]>,
    }
    impl Peintre for Compteur {
        fn cylindre(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
        fn cone(&mut self, _: Vec3, _: Vec3, _: f32, _: f32, _: f32, _: Color) {}
        fn sphere(&mut self, _: Vec3, _: f32, _: Color) {}
        fn cube(&mut self, _: Vec3, _: Vec3, _: Color) {}
        fn panneau(&mut self, _: Vec3, _: Vec3, _: Vec3, _: Color) {}
        fn ligne(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
        fn triangles(&mut self, sommets: &[Vec3], indices: &[u16], _: Color) {
            for t in indices.chunks_exact(3) {
                self.tris.push([sommets[t[0] as usize], sommets[t[1] as usize], sommets[t[2] as usize]]);
            }
        }
        fn empiler_transforme(&mut self, _: Mat4) {}
        fn depiler_transforme(&mut self) {}
    }

    /// Peintre qui ne retient QUE les triangles peints d'une couleur donnée —
    /// pour isoler le sol ou le vitrage du reste de la pièce sans dupliquer
    /// leurs fonctions de dessin dans le test.
    struct CompteurFiltre {
        cible: Color,
        tris: Vec<[Vec3; 3]>,
    }
    impl Peintre for CompteurFiltre {
        fn cylindre(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
        fn cone(&mut self, _: Vec3, _: Vec3, _: f32, _: f32, _: f32, _: Color) {}
        fn sphere(&mut self, _: Vec3, _: f32, _: Color) {}
        fn cube(&mut self, _: Vec3, _: Vec3, _: Color) {}
        fn panneau(&mut self, _: Vec3, _: Vec3, _: Vec3, _: Color) {}
        fn ligne(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
        fn triangles(&mut self, sommets: &[Vec3], indices: &[u16], couleur: Color) {
            if couleur.r != self.cible.r || couleur.g != self.cible.g || couleur.b != self.cible.b {
                return;
            }
            for t in indices.chunks_exact(3) {
                self.tris.push([sommets[t[0] as usize], sommets[t[1] as usize], sommets[t[2] as usize]]);
            }
        }
        fn empiler_transforme(&mut self, _: Mat4) {}
        fn depiler_transforme(&mut self) {}
    }

    /// Comme `CompteurFiltre`, mais retient les triangles dont la couleur
    /// satisfait un prédicat — pour la ville, qui alterne entre deux teintes.
    struct CompteurSelon<F: Fn(Color) -> bool> {
        garde: F,
        tris: Vec<[Vec3; 3]>,
    }
    impl<F: Fn(Color) -> bool> Peintre for CompteurSelon<F> {
        fn cylindre(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
        fn cone(&mut self, _: Vec3, _: Vec3, _: f32, _: f32, _: f32, _: Color) {}
        fn sphere(&mut self, _: Vec3, _: f32, _: Color) {}
        fn cube(&mut self, _: Vec3, _: Vec3, _: Color) {}
        fn panneau(&mut self, _: Vec3, _: Vec3, _: Vec3, _: Color) {}
        fn ligne(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
        fn triangles(&mut self, sommets: &[Vec3], indices: &[u16], couleur: Color) {
            if !(self.garde)(couleur) {
                return;
            }
            for t in indices.chunks_exact(3) {
                self.tris.push([sommets[t[0] as usize], sommets[t[1] as usize], sommets[t[2] as usize]]);
            }
        }
        fn empiler_transforme(&mut self, _: Mat4) {}
        fn depiler_transforme(&mut self) {}
    }

    const RAYON_MAJEUR: f32 = 12.0;
    const RAYON_MINEUR: f32 = 1.5;
    const ARC: f32 = TAU / 6.0;
    const OUVERTURE: f32 = 0.6;

    fn dessine() -> Compteur {
        let mut c = Compteur::default();
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        c
    }

    /// **Le canal reste centré sur son plan (`y = 0`), quelle que soit sa
    /// largeur.** `v` balaie un intervalle symétrique autour de 0
    /// (`±(π − ouverture)`), donc `y = rayon_mineur · sin(v)` l'est aussi —
    /// doubler `rayon_mineur` (élargir le canal) ne doit **jamais** exiger de
    /// décalage : c'est cette symétrie qui le garantit, pas une compensation
    /// ajoutée après coup.
    #[test]
    fn la_jante_reste_centree_sur_son_plan() {
        let etendue = |rayon_mineur: f32| -> (f32, f32) {
            let mut c = Compteur::default();
            dessiner(&mut c, RAYON_MAJEUR, rayon_mineur, ARC, OUVERTURE, 24, 8);
            let (mut y_min, mut y_max) = (f32::INFINITY, f32::NEG_INFINITY);
            for t in &c.tris {
                for s in t {
                    y_min = y_min.min(s.y);
                    y_max = y_max.max(s.y);
                }
            }
            (y_min, y_max)
        };

        let (y_min_base, y_max_base) = etendue(RAYON_MINEUR);
        assert!((y_max_base + y_min_base).abs() < 1e-3, "pas centré au gabarit d'origine");
        assert!(y_max_base > 0.1, "étendue quasi nulle, le test ne prouverait rien");

        // Doubler `rayon_mineur` (élargir le canal) : toujours centré, **et**
        // l'étendue double avec lui — sinon la symétrie pourrait tenir sur un
        // canal resté à sa taille d'origine.
        let (y_min, y_max) = etendue(RAYON_MINEUR * 2.0);
        assert!((y_max + y_min).abs() < 1e-3, "pas centré une fois élargi, y ∈ [{y_min:.3}, {y_max:.3}]");
        assert!(
            (y_max / y_max_base - 2.0).abs() < 1e-2,
            "l'étendue ne double pas avec rayon_mineur : {y_max_base:.3} → {y_max:.3}"
        );
    }

    /// **Vide côté moyeu.** Aucun sommet ne doit s'approcher de l'axe plus que
    /// ne le permet `ouverture` — sinon la « jante » serait un tore fermé comme
    /// un autre, et le vide qui la distingue du `Tore` n'existerait pas. Vrai
    /// pour la coque **et** le vitrage, qui vont tous deux jusqu'à
    /// `v = ±(π − ouverture)` — seul le sol, bien plus étroit, n'en approche
    /// jamais et n'a donc pas besoin d'être exclu de la vérification.
    #[test]
    fn aucun_sommet_ne_referme_le_cote_moyeu() {
        let c = dessine();
        assert!(!c.tris.is_empty(), "aucun triangle émis");
        let plancher = RAYON_MAJEUR - RAYON_MINEUR * OUVERTURE.cos();
        for t in &c.tris {
            for s in t {
                let r = (s.x * s.x + s.z * s.z).sqrt();
                assert!(
                    r >= plancher - 1e-3,
                    "un sommet à {r:.3} de l'axe referme le côté moyeu (plancher {plancher:.3})"
                );
            }
        }
    }

    /// **La paroi a une épaisseur mesurable.** La coque interne (`COQUE_INT`)
    /// doit être en retrait de la coque externe (`COQUE_EXT`) d'une distance
    /// proche de `rayon_mineur * EPAISSEUR_FRAC` — sans quoi les deux se
    /// confondraient et la pièce redeviendrait la coquille sans épaisseur
    /// d'avant.
    ///
    /// Capturé en repassant par `dessiner()` (deux `CompteurFiltre`, un par
    /// couleur), pas en rappelant `peau_arc` à côté : un test qui recalcule le
    /// décalage indépendamment du dessin peut rester vert alors que le dessin,
    /// lui, ne décale plus rien — exactement le défaut que ce test doit
    /// attraper (§C.29, une propriété d'accord entre deux calculs ne garde pas
    /// le calcul, elle garde son unicité).
    #[test]
    fn la_paroi_a_une_epaisseur_mesurable() {
        let capter = |cible: Color| -> Vec<Vec3> {
            let mut c = CompteurFiltre { cible, tris: Vec::new() };
            dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
            c.tris.into_iter().flatten().collect()
        };
        let (ext, int) = (capter(COQUE_EXT), capter(COQUE_INT));
        assert!(!ext.is_empty() && !int.is_empty(), "coque externe ou interne absente");

        let epaisseur_attendue = RAYON_MINEUR * EPAISSEUR_FRAC;
        assert!(epaisseur_attendue > 0.01, "épaisseur quasi nulle, le test ne prouverait rien");

        // Pour chaque sommet externe, la plus proche coque interne — sur une
        // paroi à épaisseur constante, ce minimum doit valoir l'épaisseur.
        let mut min_ecart = f32::INFINITY;
        for a in &ext {
            for b in &int {
                min_ecart = min_ecart.min(a.distance(*b));
            }
        }
        assert!(
            (min_ecart - epaisseur_attendue).abs() < 5e-2,
            "écart coque externe/interne mesuré à {min_ecart:.4}, attendu {epaisseur_attendue:.4}"
        );
    }

    /// **Le sol existe, est vert, et repose à l'intérieur de la paroi** — plus
    /// près de l'axe que la coque interne, jamais au-delà.
    #[test]
    fn le_sol_est_vert_et_dans_le_canal() {
        let mut c = CompteurFiltre { cible: SOL_VERT, tris: Vec::new() };
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        assert!(!c.tris.is_empty(), "aucun sol émis");
        let epaisseur = RAYON_MINEUR * EPAISSEUR_FRAC;
        let r_interne = RAYON_MAJEUR + (RAYON_MINEUR - epaisseur); // rayon max de la coque interne (v=0)
        for t in &c.tris {
            for s in t {
                let r = (s.x * s.x + s.z * s.z).sqrt();
                assert!(r <= r_interne + 1e-3, "le sol dépasse la coque interne : {r:.3} > {r_interne:.3}");
            }
        }
    }

    /// **Le sol suit la courbe jusqu'au pneu (`v ≈ 0`)**, pas seulement ses
    /// deux bords côté vitrage. Avec un seul anneau, `peau_arc` ne poserait
    /// des sommets qu'à `v0` et `v1` et relierait les deux par une corde
    /// **droite** qui saute tout le tread — là où la ville se pose. Le rayon
    /// maximal atteint par le sol doit donc s'approcher de
    /// `RAYON_MAJEUR + r_sol` (le rayon exact à `v = 0`), pas rester bloqué
    /// au rayon (plus petit) des bords `v0`/`v1`.
    #[test]
    fn le_sol_suit_la_courbe_jusquau_pneu() {
        let mut c = CompteurFiltre { cible: SOL_VERT, tris: Vec::new() };
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        assert!(!c.tris.is_empty(), "aucun sol émis");
        let epaisseur = RAYON_MINEUR * EPAISSEUR_FRAC;
        let r_pneu = RAYON_MAJEUR + r_sol(RAYON_MINEUR, epaisseur);
        let r_max = c
            .tris
            .iter()
            .flatten()
            .map(|s| (s.x * s.x + s.z * s.z).sqrt())
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            (r_max - r_pneu).abs() < 0.05,
            "le sol culmine à r = {r_max:.3}, loin de r_pneu = {r_pneu:.3} — il ne passe pas par v ≈ 0"
        );
    }

    /// **Le sol s'étend jusqu'au vitrage**, il ne flotte pas isolé au centre
    /// du tread : ses bornes en `v` doivent atteindre `v_paroi` des deux
    /// côtés, à la précision du maillage près.
    #[test]
    fn le_sol_rejoint_le_vitrage() {
        let mut c = CompteurFiltre { cible: SOL_VERT, tris: Vec::new() };
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        let (p0, p1) = v_paroi(OUVERTURE);
        let v_de = |s: &Vec3| {
            let r = (s.x * s.x + s.z * s.z).sqrt() - RAYON_MAJEUR;
            s.y.atan2(r)
        };
        let (mut v_min, mut v_max) = (f32::INFINITY, f32::NEG_INFINITY);
        for t in &c.tris {
            for s in t {
                let v = v_de(s);
                v_min = v_min.min(v);
                v_max = v_max.max(v);
            }
        }
        assert!((v_min - p0).abs() < 0.05, "le sol s'arrête à v = {v_min:.3} avant le bord {p0:.3}");
        assert!((v_max - p1).abs() < 0.05, "le sol s'arrête à v = {v_max:.3} avant le bord {p1:.3}");
    }

    /// **Le vert du sol doit regarder vers l'axe** (l'intérieur du canal,
    /// côté où se tiennent les habitants) — comme `COQUE_INT`, pas comme
    /// `COQUE_EXT`, qui ne regarde que vers l'extérieur de la station,
    /// invisible en jeu.
    ///
    /// Référence : la normale analytique exacte au point (dérivée de sa
    /// propre position, sans repasser par `u`/`v` en trigonométrie inverse
    /// ambiguë) — **pas** une approximation purement radiale (XZ) comme un
    /// premier essai l'a fait. Près des flancs (`v` proche de `π/2`), la vraie
    /// normale devient majoritairement **verticale** (`n = (0, 1, 0)` pile à
    /// `v = π/2`) ; une référence purement radiale n'y mesure presque plus
    /// rien d'utile, et un maillage qui suit désormais la vraie courbe
    /// jusqu'à `v ≈ 0` (contrairement à l'ancien maillage à un seul anneau,
    /// qui ne visitait jamais que les deux bords) y faisait rougir ce test à
    /// tort.
    #[test]
    fn le_sol_vert_est_visible_depuis_linterieur() {
        let mut c = CompteurFiltre { cible: SOL_VERT, tris: Vec::new() };
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        assert!(!c.tris.is_empty(), "aucun sol vert émis");
        let epaisseur = RAYON_MINEUR * EPAISSEUR_FRAC;
        let r = r_sol(RAYON_MINEUR, epaisseur);
        for t in &c.tris {
            let normale = (t[1] - t[0]).cross(t[2] - t[0]);
            let centre = (t[0] + t[1] + t[2]) / 3.0;
            let r_xz = (centre.x * centre.x + centre.z * centre.z).sqrt();
            let cos_v = (r_xz - RAYON_MAJEUR) / r;
            // n = (cos(v)cos(u), sin(v), cos(v)sin(u)), avec cos(u) = x/r_xz,
            // sin(u) = z/r_xz, sin(v) = y/r — dérivé directement de la
            // position, sans repasser par `atan2`.
            let n = vec3(cos_v * centre.x / r_xz, centre.y / r, cos_v * centre.z / r_xz);
            assert!(
                normale.dot(n) < 0.0,
                "une face verte du sol regarde vers l'extérieur, pas vers l'axe (centre={centre:?})"
            );
        }
    }

    fn ville_captee() -> Vec<[Vec3; 3]> {
        let mut c = CompteurSelon {
            garde: |couleur: Color| {
                let proche = |a: Color, b: Color| (a.r - b.r).abs() < 1e-4 && (a.g - b.g).abs() < 1e-4;
                proche(couleur, VILLE_TEINTE) || proche(couleur, VILLE_TEINTE_SOMBRE)
            },
            tris: Vec::new(),
        };
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        c.tris
    }

    /// **La ville existe, et n'est pas une grille pleine.** `VILLE_DENSITE`
    /// (< 1) doit vraiment sauter des cellules — sinon la « ville » serait un
    /// mur crénelé continu, pas des bâtiments séparés par des rues.
    #[test]
    fn la_ville_laisse_des_rues() {
        let tris = ville_captee();
        assert!(!tris.is_empty(), "aucun bâtiment émis");
        // Chaque bâtiment porte 10 triangles (toit : 2, quatre flancs : 2
        // chacun) : le nombre de bâtiments doit rester strictement sous le
        // nombre total de cellules.
        assert_eq!(tris.len() % 10, 0, "un bâtiment incomplet ?");
        let n_batiments = tris.len() / 10;
        assert!(
            n_batiments < VILLE_CASES_U * VILLE_CASES_V,
            "{n_batiments} bâtiments pour {} cellules : aucune rue",
            VILLE_CASES_U * VILLE_CASES_V
        );
        assert!(n_batiments > 0, "aucun bâtiment (densité à zéro ?)");
    }

    /// **Les hauteurs varient** — un bruit qui rendrait toujours la même
    /// valeur donnerait une ville de bâtiments tous identiques, pas une
    /// silhouette. Mesuré sur l'étendue verticale de chaque toit (les deux
    /// premiers triangles de chaque groupe de 10).
    #[test]
    fn la_ville_a_des_hauteurs_variees() {
        let tris = ville_captee();
        // Mesure **la distance toit↔base de chaque bâtiment**, pas la position
        // absolue de son toit : deux bâtiments à la même hauteur mais sur des
        // rangées différentes (`v` différent) n'ont pas le même rayon — une
        // première version comparait les rayons bruts et ne rougissait pas
        // sous une hauteur figée, noyée dans la variation de position.
        //
        // Le toit est `bas + hauteur·haut` sur ses quatre coins (§`batiment`) :
        // son centroïde moins celui de la base vaut donc exactement `hauteur`,
        // quelle que soit la rangée.
        let hauteurs: Vec<f32> = tris
            .chunks_exact(10)
            .map(|batiment| {
                let toit: Vec<Vec3> = batiment[0].iter().chain(batiment[1].iter()).cloned().collect();
                let est_toit = |p: &Vec3| toit.iter().any(|t| t.distance(*p) < 1e-4);
                let bases: Vec<Vec3> =
                    batiment[2..].iter().flatten().filter(|p| !est_toit(p)).cloned().collect();
                let centroide = |v: &[Vec3]| v.iter().fold(Vec3::ZERO, |a, c| a + *c) / v.len() as f32;
                centroide(&toit).distance(centroide(&bases))
            })
            .collect();
        assert!(hauteurs.len() > 3, "trop peu de bâtiments pour juger d'une variation");
        let (min, max) = (
            hauteurs.iter().cloned().fold(f32::INFINITY, f32::min),
            hauteurs.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
        );
        assert!(max - min > 0.05, "tous les bâtiments ont la même hauteur ({min:.3})");
    }

    /// La ville reste dans l'arc `[0, arc]` — même exigence que le reste de la
    /// pièce (`larc_reste_dans_ses_bornes`), vérifiée séparément parce que la
    /// ville est un ajout récent qui pourrait déborder sans que le test global
    /// (qui ne filtre pas par couleur) le distingue d'un autre défaut.
    #[test]
    fn la_ville_reste_dans_larc() {
        let tris = ville_captee();
        for t in &tris {
            for s in t {
                let u = s.z.atan2(s.x).rem_euclid(TAU);
                assert!(u <= ARC + 0.05 || u >= TAU - 0.05, "bâtiment à u = {:.3}° hors de l'arc", u.to_degrees());
            }
        }
    }

    /// **La ville reste dans une bande étroite et centrée** (`v = 0`, le
    /// « pneu »), loin des flancs — pas jusqu'au bord du vitrage comme le
    /// sol lui-même. Sur les flancs la section se courbe ; y poser des
    /// bâtiments les ferait pencher avec elle.
    #[test]
    fn la_ville_reste_dans_une_bande_etroite() {
        let tris = ville_captee();
        assert!(!tris.is_empty(), "aucun bâtiment émis");
        let v_de = |s: &Vec3| {
            let r = (s.x * s.x + s.z * s.z).sqrt() - RAYON_MAJEUR;
            s.y.atan2(r)
        };
        // Marge au-delà de `VILLE_DEMI_LARGEUR` : un bâtiment a une emprise
        // (`VILLE_EMPRISE` d'une cellule), donc ses coins débordent le centre
        // de la rangée la plus excentrée d'environ une cellule — ce n'est
        // pas un défaut, juste ce que « centré et étroit » veut dire une fois
        // qu'on mesure les sommets et non les centres.
        let (pv0, pv1) = v_paroi(OUVERTURE);
        let demi = VILLE_DEMI_LARGEUR.min((pv1 - pv0) * 0.5);
        let marge = (2.0 * demi) / VILLE_CASES_V as f32;
        for t in &tris {
            for s in t {
                let v = v_de(s);
                assert!(
                    v.abs() <= VILLE_DEMI_LARGEUR + marge,
                    "un sommet de bâtiment à v = {v:.3}, hors de la bande [-{VILLE_DEMI_LARGEUR}, {VILLE_DEMI_LARGEUR}] (marge {marge:.3})"
                );
            }
        }
    }

    /// **Chaque bâtiment se dresse à plat, jamais penché vers un flanc.**
    /// Son « haut » (centroïde du toit moins centroïde de la base) vient de
    /// la normale à `v = 0`, purement radiale (`y = 0`) — donc horizontal,
    /// quelle que soit la rangée d'où vient le bâtiment. Avant la correction,
    /// un bâtiment posé sur une rangée excentrée penchait avec la normale
    /// locale de sa propre position, et semblait collé à l'intérieur de la
    /// jante plutôt que debout sur le sol.
    #[test]
    fn les_batiments_se_dressent_a_plat() {
        let tris = ville_captee();
        let mut n = 0;
        for batiment in tris.chunks_exact(10) {
            let toit: Vec<Vec3> = batiment[0].iter().chain(batiment[1].iter()).cloned().collect();
            let est_toit = |p: &Vec3| toit.iter().any(|t| t.distance(*p) < 1e-4);
            let bases: Vec<Vec3> = batiment[2..].iter().flatten().filter(|p| !est_toit(p)).cloned().collect();
            let centroide = |v: &[Vec3]| v.iter().fold(Vec3::ZERO, |a, c| a + *c) / v.len() as f32;
            let direction = (centroide(&toit) - centroide(&bases)).normalize_or_zero();
            assert!(direction.y.abs() < 1e-3, "un bâtiment penche hors du plan horizontal : up.y = {:.4}", direction.y);
            n += 1;
        }
        assert!(n > 3, "trop peu de bâtiments pour juger quoi que ce soit");
    }

    /// **L'encastrement n'est pas nul.** Garde-fou indépendant du test
    /// géométrique suivant : celui-là dérive sa valeur attendue de
    /// `VILLE_ENCASTREMENT_FRAC` lui-même, donc ne peut pas détecter que la
    /// constante ait été remise à zéro — seulement que le code la respecte,
    /// quelle qu'elle soit.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn lencastrement_de_la_ville_nest_pas_nul() {
        assert!(
            VILLE_ENCASTREMENT_FRAC > 0.01,
            "VILLE_ENCASTREMENT_FRAC = {VILLE_ENCASTREMENT_FRAC} — trop faible pour détacher visiblement la ville du sol"
        );
    }

    /// **Les bâtiments sont encastrés dans le sol**, pas pile sur sa
    /// surface : le centre de chaque base (moyenne de ses 4 coins — les
    /// décalages `± tangente_u ± tangente_v` s'y annulent, donc indépendante
    /// de l'emprise) doit être à un rayon **supérieur** de très exactement
    /// `VILLE_ENCASTREMENT_FRAC` au rayon qu'aurait le sol au même point,
    /// sans quoi base et sol coïncident et clignotent (z-fighting), ou pire,
    /// s'écartent l'un de l'autre (un vide visible, l'inverse de « posé sur »).
    #[test]
    fn les_batiments_sont_encastres_dans_le_sol() {
        let tris = ville_captee();
        let epaisseur = RAYON_MINEUR * EPAISSEUR_FRAC;
        let r = r_sol(RAYON_MINEUR, epaisseur);
        let encastrement = RAYON_MINEUR * VILLE_ENCASTREMENT_FRAC;
        let mut n = 0;
        for batiment in tris.chunks_exact(10) {
            let toit: Vec<Vec3> = batiment[0].iter().chain(batiment[1].iter()).cloned().collect();
            let est_toit = |p: &Vec3| toit.iter().any(|t| t.distance(*p) < 1e-4);
            let bases: Vec<Vec3> = batiment[2..].iter().flatten().filter(|p| !est_toit(p)).cloned().collect();
            let mut coins: Vec<Vec3> = Vec::new();
            for b in &bases {
                if !coins.iter().any(|c: &Vec3| c.distance(*b) < 1e-4) {
                    coins.push(*b);
                }
            }
            assert_eq!(coins.len(), 4, "une base n'a pas 4 coins distincts");
            let centre = coins.iter().fold(Vec3::ZERO, |a, c| a + *c) / 4.0;
            // `centre.y` n'est pas touché par l'encastrement (`n0` est
            // horizontal) : il donne directement `v`, d'où le rayon qu'aurait
            // eu le sol à ce point, avant encastrement.
            let v = (centre.y / r).clamp(-1.0, 1.0).asin();
            let rayon_sol = RAYON_MAJEUR + r * v.cos();
            let rayon_centre = (centre.x * centre.x + centre.z * centre.z).sqrt();
            assert!(
                (rayon_centre - rayon_sol - encastrement).abs() < 1e-2,
                "centre à rayon {rayon_centre:.4}, attendu ≈ {:.4} (sol {rayon_sol:.4} + encastrement {encastrement:.4})",
                rayon_sol + encastrement
            );
            n += 1;
        }
        assert!(n > 3, "trop peu de bâtiments pour juger quoi que ce soit");
    }

    /// **Le vitrage remplace la coque aux deux bouts de la paroi, pas au
    /// milieu.** Ses sommets doivent tomber dans les zones `v_vitrages`, à
    /// l'extérieur de `v_paroi` — sinon il empièterait sur la coque pleine ou
    /// laisserait un trou entre les deux.
    #[test]
    fn le_vitrage_occupe_le_bout_de_la_paroi() {
        let mut c = CompteurFiltre { cible: VERRE, tris: Vec::new() };
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        assert!(!c.tris.is_empty(), "aucun vitrage émis");
        let (p0, p1) = v_paroi(OUVERTURE);
        for t in &c.tris {
            for s in t {
                // v se retrouve depuis la hauteur (y = rayon_mineur * sin v)
                // et le rayon radial (r = rayon_majeur + rayon_mineur * cos v).
                let r = (s.x * s.x + s.z * s.z).sqrt() - RAYON_MAJEUR;
                let v = s.y.atan2(r);
                assert!(
                    v <= p0 + 0.05 || v >= p1 - 0.05,
                    "un sommet de vitrage à v = {v:.3} tombe dans la paroi [{p0:.3}, {p1:.3}]"
                );
            }
        }
    }

    /// **La grille du vitrage ne coûte pas de vrais cylindres.** Un meneau est
    /// un fil décoratif fin : `ligne` (une croix de deux panneaux, 8 triangles)
    /// coûte dix fois moins qu'un tronc de cylindre plein (`SEGMENTS = 20`
    /// côtés révolus, cf. `maillage.rs`), pour un rendu indiscernable à cette
    /// échelle. Avec six tronçons de jante, chacun deux zones vitrées, c'était
    /// le premier poste de coût du complexe Elysium (`docs/suivi/stations.md`).
    #[test]
    fn le_vitrage_nutilise_pas_de_cylindres() {
        struct CompteurCylindres {
            n: usize,
        }
        impl Peintre for CompteurCylindres {
            fn cylindre(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {
                self.n += 1;
            }
            fn cone(&mut self, _: Vec3, _: Vec3, _: f32, _: f32, _: f32, _: Color) {}
            fn sphere(&mut self, _: Vec3, _: f32, _: Color) {}
            fn cube(&mut self, _: Vec3, _: Vec3, _: Color) {}
            fn panneau(&mut self, _: Vec3, _: Vec3, _: Vec3, _: Color) {}
            fn ligne(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
            fn triangles(&mut self, _: &[Vec3], _: &[u16], _: Color) {}
            fn empiler_transforme(&mut self, _: Mat4) {}
            fn depiler_transforme(&mut self) {}
        }
        let mut c = CompteurCylindres { n: 0 };
        dessiner(&mut c, RAYON_MAJEUR, RAYON_MINEUR, ARC, OUVERTURE, 24, 8);
        assert_eq!(c.n, 0, "{} cylindre(s) émis — la grille du vitrage devrait n'utiliser que `ligne`", c.n);
    }

    /// L'arc ne déborde pas au-delà de `[0, arc]` : sinon « un sixième » en
    /// couvrirait davantage, et deux jantes voisines se chevaucheraient. Vaut
    /// pour toute la pièce (paroi, sol, vitrage), donc vérifié sans filtrer
    /// par couleur.
    #[test]
    fn larc_reste_dans_ses_bornes() {
        let c = dessine();
        for t in &c.tris {
            for s in t {
                let u = s.z.atan2(s.x).rem_euclid(TAU);
                assert!(
                    u <= ARC + 1e-3 || u >= TAU - 1e-3,
                    "sommet à u = {:.3}° hors de l'arc [0, {:.3}°]",
                    u.to_degrees(),
                    ARC.to_degrees()
                );
            }
        }
    }

}
