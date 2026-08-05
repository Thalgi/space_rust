//! **Raptor** (SpaceX) : moteur méthalox à **combustion étagée à flux intégral**,
//! en deux versions — atmosphérique et vide.
//!
//! # Pourquoi une brique et pas un `Propulseur` de plus
//!
//! `Propulseur` est un tronc de cône générique : il dit « il y a une tuyère
//! ici ». Un Raptur se reconnaît à ce qu'un cône n'a pas — une **cloche**
//! (pas un cône), des **cannelures** de refroidissement, et surtout la
//! *powerhead* : le fouillis de turbopompes et de conduites au-dessus de la
//! chambre, qui fait la moitié de la hauteur du moteur. C'est cette masse
//! technique, et non la tuyère, qui distingue un moteur moderne d'un pot.
//!
//! # Les cotes viennent du réel
//!
//! Converties à l'échelle du projet (1 U ≈ 2,25 m) :
//!
//! | | réel | ici |
//! |---|---:|---:|
//! | sortie RSL (atmosphérique) | Ø 1,30 m | **0,58 U** |
//! | hauteur RSL | 3,10 m | 1,38 U |
//! | sortie RVac (vide) | Ø 2,40 m | **1,07 U** |
//! | hauteur RVac | 4,60 m | 2,04 U |
//!
//! Le rapport **hauteur/sortie** diffère (2,4 contre 1,9) et ce n'est pas un
//! détail : le RVac est plus large *relativement* à sa longueur, parce que sa
//! détente de 90 lui impose une cloche très évasée. C'est ce qui les distingue
//! au premier coup d'œil, plus encore que la taille absolue.
//!
//! # La cloche n'est pas un cône
//!
//! Une tuyère de détente s'ouvre **vite près du col**, puis s'aplatit : le gaz y
//! est déjà supersonique et la paroi ne fait plus que l'accompagner. D'où un
//! profil en `t^0,55` et non linéaire. Un cône droit lit comme un entonnoir.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::TAU;


/// Les deux versions embarquées sur un Starship.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VarianteRaptor {
    /// **Atmosphérique** : petite cloche (détente ~40), montée sur cardan —
    /// c'est elle qui pilote. Trois au centre du cul.
    Atmospherique,
    /// **Vide** : cloche très évasée (détente ~90) prolongée d'une jupe
    /// **refroidie par rayonnement**, visiblement rapportée. Fixe, et disposée
    /// en périphérie parce qu'elle ne rentrerait pas ailleurs.
    Vide,
}

impl VarianteRaptor {
    pub const TOUTES: [VarianteRaptor; 2] = [VarianteRaptor::Atmospherique, VarianteRaptor::Vide];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteRaptor::Atmospherique => "RAPTOR ATMOSPHERIQUE (RSL)",
            VarianteRaptor::Vide => "RAPTOR VIDE (RVac)",
        }
    }

    /// Rayon de sortie **nominal**, en unités monde. Lu du réel.
    pub fn rayon_nominal(self) -> f32 {
        match self {
            VarianteRaptor::Atmospherique => 1.30 / 2.0 / 2.25,
            VarianteRaptor::Vide => 2.40 / 2.0 / 2.25,
        }
    }

    /// Hauteur hors-tout pour un rayon de sortie donné. Le rapport diffère
    /// entre les deux versions — c'est leur signature.
    pub fn hauteur(self, rayon: f32) -> f32 {
        let k = match self {
            VarianteRaptor::Atmospherique => 3.10 / (1.30 / 2.0),
            VarianteRaptor::Vide => 4.60 / (2.40 / 2.0),
        };
        rayon * k
    }

    /// Rapport col / sortie, tiré du rapport de détente (`r_col = r_e / √ε`).
    fn col(self) -> f32 {
        let eps: f32 = match self {
            VarianteRaptor::Atmospherique => 40.0,
            VarianteRaptor::Vide => 90.0,
        };
        1.0 / eps.sqrt()
    }
}

/// Alliage de la cloche : plus sombre que l'inox de la coque, un peu cuivré —
/// les parois régénératives sont en alliage de cuivre sous une peau d'Inconel.
const CLOCHE: Color = Color { r: 0.46, g: 0.41, b: 0.38, a: 1.0 };
/// Cannelures de refroidissement : le liseré qui trahit les canaux brasés.
const CANNELURE: Color = Color { r: 0.30, g: 0.27, b: 0.25, a: 1.0 };
/// Jupe de la version vide, refroidie par rayonnement : elle cuit, donc elle
/// est plus sombre et plus mate que la partie régénérative.
const JUPE: Color = Color { r: 0.34, g: 0.29, b: 0.27, a: 1.0 };
/// Turbopompes et corps de la powerhead : de la mécanique, claire et propre.
const POMPE: Color = Color { r: 0.66, g: 0.67, b: 0.70, a: 1.0 };
/// Conduites : plus sombres, elles doivent se détacher des corps qu'elles
/// relient, sinon la powerhead lit comme un bloc.
const CONDUITE: Color = Color { r: 0.40, g: 0.42, b: 0.46, a: 1.0 };

/// Facettes autour de l'axe. La cloche est petite à l'écran : 20 suffit.
const FACETTES: usize = 20;
/// Part de la hauteur occupée par la cloche ; le reste est chambre + powerhead.
const PART_CLOCHE: f32 = 0.62;
/// Exposant du profil de cloche. < 1 → ouverture rapide au col puis
/// aplatissement, ce qui **est** la forme d'une tuyère de détente.
const GALBE: f32 = 0.55;
/// Section d'une cannelure, en fraction du rayon de sortie. Elle est **posée
/// sur** la paroi, donc elle depasse — comme les cordons de soudure de la coque
/// (`coque::rayon_hors_tout`), et pour la meme raison : un canal brase fait un
/// relief, il ne se noie pas dans la tole.
const CANNELURE_SECTION: f32 = 0.012;

/// Rayon **hors-tout**, cannelures comprises. Une source unique pour le dessin
/// et pour l'enveloppe.
pub(super) fn rayon_hors_tout(rayon: f32) -> f32 {
    rayon * (1.0 + CANNELURE_SECTION)
}

/// Rayon de la cloche à la fraction `t` de sa longueur (0 = col, 1 = sortie).
fn cloche(col: f32, sortie: f32, t: f32) -> f32 {
    col + (sortie - col) * t.clamp(0.0, 1.0).powf(GALBE)
}

/// Le profil de cloche, expose pour que les tests le mesurent **a la source**
/// plutot que de le recopier. `t` : 0 au col, 1 a la sortie.
#[cfg(test)]
pub(super) fn cloche_essai(variante: VarianteRaptor, rayon: f32, t: f32) -> f32 {
    cloche(rayon * variante.col(), rayon, t)
}

/// Monté par le **haut** : le cardan regarde l'hôte, la tuyère s'en éloigne.
pub(super) fn ports(profil: Profil, variante: VarianteRaptor, rayon: f32) -> Vec<Port> {
    let demi = variante.hauteur(rayon) * 0.5;
    vec![Port::new(
        Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY),
        GenrePort::ModuleAxial,
        profil,
    )]
}

pub(super) fn cout(variante: VarianteRaptor, rayon: f32) -> f32 {
    // Un moteur se paie à la poussée, qui va comme la section de sortie.
    let base = rayon * rayon * 18.0;
    match variante {
        VarianteRaptor::Atmospherique => base + 4.0,
        // La jupe de détente coûte cher pour ce qu'elle pèse.
        VarianteRaptor::Vide => base + 6.0,
    }
}

pub(super) fn rayon_local(variante: VarianteRaptor, rayon: f32) -> f32 {
    (variante.hauteur(rayon) * 0.5).hypot(rayon_hors_tout(rayon))
}

fn couronne(z: f32, r: f32) -> Vec<Vec3> {
    (0..FACETTES)
        .map(|k| {
            let a = TAU * k as f32 / FACETTES as f32;
            vec3(r * a.cos(), r * a.sin(), z)
        })
        .collect()
}

fn ceinture(bas: &[Vec3], haut: &[Vec3], s: &mut Vec<Vec3>, i: &mut Vec<u16>) {
    let base = s.len() as u16;
    s.extend_from_slice(bas);
    s.extend_from_slice(haut);
    let n = FACETTES as u16;
    for k in 0..n {
        let (b0, b1) = (base + k, base + (k + 1) % n);
        let (h0, h1) = (base + n + k, base + n + (k + 1) % n);
        i.extend_from_slice(&[b0, h0, h1, b0, h1, b1]);
    }
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, variante: VarianteRaptor, rayon: f32) {
    let h = variante.hauteur(rayon);
    let demi = h * 0.5;
    let z_sortie = -demi; // la tuyère crache vers −Z
    let h_cloche = h * PART_CLOCHE;
    let z_col = z_sortie + h_cloche;
    let r_col = rayon * variante.col();

    // --- la cloche, du col vers la sortie ---
    let n = 12usize;
    let mut sommets = Vec::new();
    let mut indices = Vec::new();
    let mut prec = couronne(z_col, r_col);
    for k in 1..=n {
        let t = k as f32 / n as f32;
        let z = z_col - h_cloche * t;
        let cur = couronne(z, cloche(r_col, rayon, t));
        ceinture(&prec, &cur, &mut sommets, &mut indices);
        prec = cur;
    }
    p.triangles(&sommets, &indices, CLOCHE);

    // --- cannelures : les canaux de refroidissement brasés. Sans elles, la
    // cloche est un entonnoir lisse et le moteur perd son échelle.
    let n_can = 16usize;
    for j in 0..n_can {
        let a = TAU * j as f32 / n_can as f32;
        let (sa, ca) = a.sin_cos();
        let pas = 5usize;
        for k in 0..pas {
            let (t0, t1) = (k as f32 / pas as f32, (k + 1) as f32 / pas as f32);
            let (r0, r1) = (cloche(r_col, rayon, t0), cloche(r_col, rayon, t1));
            p.cylindre(
                vec3(r0 * ca, r0 * sa, z_col - h_cloche * t0),
                vec3(r1 * ca, r1 * sa, z_col - h_cloche * t1),
                rayon * CANNELURE_SECTION,
                CANNELURE,
            );
        }
    }

    // --- version vide : la **jupe rapportée**, refroidie par rayonnement.
    // Un bourrelet marque le raccord — sur le vrai moteur c'est une soudure
    // circonférentielle, et c'est ce qui dit que l'extension est une pièce à
    // part et non la suite de la cloche.
    if variante == VarianteRaptor::Vide {
        let t = 0.45;
        let r = cloche(r_col, rayon, t);
        let z = z_col - h_cloche * t;
        let c = couronne(z, r * 1.03);
        for j in 0..FACETTES {
            p.cylindre(c[j], c[(j + 1) % FACETTES], rayon * 0.022, JUPE);
        }
    }

    // --- chambre de combustion : un fût court au-dessus du col ---
    let r_ch = r_col * 2.6;
    let z_ch = z_col + h * 0.10;
    p.cone(vec3(0.0, 0.0, z_col), Vec3::Z, r_col, r_ch, h * 0.10, CLOCHE);
    p.cylindre(vec3(0.0, 0.0, z_ch), vec3(0.0, 0.0, z_ch + h * 0.06), r_ch, POMPE);

    // --- powerhead : deux turbopompes décalées et leurs conduites ---
    //
    // C'est la moitié haute du moteur, et c'est elle qui le fait lire comme du
    // matériel moderne. Deux corps (les prébrûleurs oxygène et méthane), pas
    // un : la combustion étagée à flux intégral en a deux, et leur asymétrie
    // est la signature visuelle du Raptor.
    let z_ph = z_ch + h * 0.06;
    let haut = demi;
    for (i, (dx, dy)) in [(1.0_f32, 0.25_f32), (-0.85, -0.5)].into_iter().enumerate() {
        let o = vec3(dx * r_ch * 0.9, dy * r_ch * 0.9, 0.0);
        let r_p = r_ch * if i == 0 { 0.52 } else { 0.44 };
        let hp = (haut - z_ph) * if i == 0 { 0.72 } else { 0.58 };
        p.cylindre(o + vec3(0.0, 0.0, z_ph), o + vec3(0.0, 0.0, z_ph + hp), r_p, POMPE);
        // Volute d'entrée, en travers : une pompe centrifuge n'est pas un tube.
        p.cylindre(
            o + vec3(-r_p * 1.2, 0.0, z_ph + hp),
            o + vec3(r_p * 1.2, 0.0, z_ph + hp),
            r_p * 0.55,
            POMPE,
        );
        // Conduite qui redescend vers la chambre.
        p.cylindre(o + vec3(0.0, 0.0, z_ph + hp * 0.5), vec3(0.0, 0.0, z_ch), r_ch * 0.2, CONDUITE);
    }
    // Collecteur annulaire autour de la chambre — l'anneau d'injection.
    let c = couronne(z_ch + h * 0.03, r_ch * 1.15);
    for j in 0..FACETTES {
        p.cylindre(c[j], c[(j + 1) % FACETTES], r_ch * 0.11, CONDUITE);
    }

    // --- tête : cardan pour l'atmosphérique, platine fixe pour le vide ---
    match variante {
        VarianteRaptor::Atmospherique => {
            // Rotule + deux vérins : ce moteur **braque**, et ça doit se voir.
            p.sphere(vec3(0.0, 0.0, haut), r_ch * 0.42, POMPE);
            for s in [-1.0_f32, 1.0] {
                p.cylindre(
                    vec3(s * r_ch * 1.1, 0.0, haut - (haut - z_ph) * 0.45),
                    vec3(s * r_ch * 0.3, 0.0, haut),
                    r_ch * 0.13,
                    CONDUITE,
                );
            }
        }
        VarianteRaptor::Vide => {
            // Fixe : une platine boulonnée, pas de rotule.
            p.cylindre(
                vec3(0.0, 0.0, haut - r_ch * 0.12),
                vec3(0.0, 0.0, haut),
                r_ch * 0.95,
                POMPE,
            );
        }
    }
}
