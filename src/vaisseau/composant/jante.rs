//! **Jante** : portion de tore en **canal ouvert**, façon jante de vélo — une
//! paroi qui court du flanc à l'autre en passant par le bord extérieur (le
//! « pneu »), et un **vide côté moyeu** : rien ne referme le côté qui regarde
//! l'axe, on voit au travers depuis l'intérieur de l'anneau.
//!
//! Même surface paramétrique que [`super::tore`] ([`surface_tore`], partagée
//! via `commun.rs`), mais habillée autrement, et en trois parties :
//!
//! - la **paroi** (tread + la plus grande partie des flancs), avec une vraie
//!   **épaisseur** — coque externe et coque interne, décalées le long de la
//!   normale locale ;
//! - le **sol**, une bande plate et verte posée à l'intérieur du canal, sur la
//!   face interne du « pneu » — le sol (provisoire) de la mégastructure ;
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
/// la coque pleine.
const VITRAGE_SPAN: f32 = 0.35;

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

    sol(p, rayon_majeur, rayon_mineur, epaisseur, arc, segments);

    for (vv0, vv1) in v_vitrages(ouverture) {
        vitrage(p, rayon_majeur, rayon_mineur, arc, segments, vv0, vv1);
    }
}

/// Demi-largeur du sol (le long de `v`, autour du « pneu », `v = 0`) — assez
/// étroit pour rester loin des flancs à tous les gabarits raisonnables
/// (`ouverture` borne `v_paroi` par en dessous).
const SOL_DEMI_LARGEUR: f32 = 0.55;
/// Retrait du sol par rapport à la face interne de la paroi, en fraction de
/// l'épaisseur — juste assez pour ne pas coïncider exactement avec elle (deux
/// surfaces confondues clignoteraient, cf. `hangar.rs` sur le même piège).
const SOL_RETRAIT_FRAC: f32 = 0.15;
/// Vert de terrain provisoire — voir la note de tête de fichier.
const SOL_VERT: Color = Color { r: 0.18, g: 0.52, b: 0.20, a: 1.0 };
const SOL_VERT_DESSOUS: Color = Color { r: 0.10, g: 0.30, b: 0.12, a: 1.0 };

/// **Le sol de la mégastructure** : une bande plate, verte, posée à
/// l'intérieur du canal, contre la face interne du « pneu ».
///
/// Une station en anneau tourne pour simuler la gravité : le sol, c'est la
/// face **extérieure** du tube (le pneu), et « en dessous » y pointe vers
/// l'axe de rotation. Ce que les habitants foulent est donc la face
/// **interne** de la coque externe — la même surface que `bande_double_face`
/// dessine déjà côté `COQUE_INT`, mais en léger retrait pour se distinguer
/// d'elle comme une pièce à part, et non un simple changement de couleur.
///
/// Provisoire, et assumé comme tel : vert plat, sans texture — un terrain à
/// habiller viendra plus tard.
fn sol<P: Peintre>(p: &mut P, rayon_majeur: f32, rayon_mineur: f32, epaisseur: f32, arc: f32, segments: usize) {
    let r_sol = (rayon_mineur - epaisseur * (1.0 + SOL_RETRAIT_FRAC)).max(0.01);
    let (s, i) = peau_arc(rayon_majeur, r_sol, segments, 1, arc, -SOL_DEMI_LARGEUR, SOL_DEMI_LARGEUR);
    bande_double_face(p, &s, &i, SOL_VERT, SOL_VERT_DESSOUS);
}

const VERRE: Color = Color { r: 0.55, g: 0.78, b: 0.88, a: 0.35 };
const GRILLE: Color = Color { r: 0.28, g: 0.29, b: 0.33, a: 1.0 };
/// Pas des meneaux le long de l'arc, en unités monde — même idée que
/// `tore::MENEAU_PAS` : un vitrage courbe continu ne se fabrique pas d'un
/// tenant, il se pose en panneaux, et c'est ce pas qui donne son échelle
/// apparente au canal.
const MENEAU_PAS: f32 = 2.4;
/// Section d'un meneau, en fraction de `rayon_mineur`.
const MENEAU_SECTION_FRAC: f32 = 0.05;
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

    let saillie = rayon_mineur * MENEAU_SECTION_FRAC;
    let nervure_section = |p: &mut P, u: f32| {
        for k in 0..MENEAU_PAS_ARC {
            let a = v0 + (v1 - v0) * k as f32 / MENEAU_PAS_ARC as f32;
            let b = v0 + (v1 - v0) * (k + 1) as f32 / MENEAU_PAS_ARC as f32;
            let (pa, na) = surface_tore(rayon_majeur, rayon_mineur, u, a);
            let (pb, nb) = surface_tore(rayon_majeur, rayon_mineur, u, b);
            p.cylindre(pa + na * saillie, pb + nb * saillie, saillie, GRILLE);
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
            p.cylindre(pa + na * saillie, pb + nb * saillie, saillie * 1.3, GRILLE);
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
