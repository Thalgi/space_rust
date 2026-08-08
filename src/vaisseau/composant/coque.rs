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

/// Rayon de l'ogive a la distance `x` de la **pointe**.
///
/// Meme courbe que celle que dessine la coque : ce qui se pose sur le nez
/// (hublot, bardage) la lit **ici** au lieu de l'approcher. Un vitrage pose sur
/// un rayon suppose flotterait au-dessus du vide ou s'enfoncerait dans la peau,
/// et c'est invisible tant qu'on ne regarde pas l'engin de trois quarts.
pub fn ogive_rayon(rayon: f32, longueur: f32, x: f32) -> f32 {
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

// =============================== Jupe moteur ===============================

/// Combien de la hauteur d'un moteur la jupe doit **couvrir**, au minimum.
///
/// Un lanceur ne montre pas ses moteurs à nu : la jupe arrière porte la poussée
/// et les enferme, seules les cloches dépassent. Sans elle, les six Raptor
/// pendent sous la coque comme des lampes.
pub const JUPE_COUVERTURE: f32 = 2.0 / 3.0;

/// Hauteur de la nervure de renfort du bord inférieur, en fraction du rayon.
/// C'est elle qui donne son épaisseur au bord ; une jupe à bord vif lit comme
/// une découpe, pas comme une structure.
const JUPE_NERVURE: f32 = 0.02;

/// Rayon **utile** de la jupe : sa paroi interieure.
///
/// C'est lui qui borne les moteurs, pas le rayon nominal -- la paroi rentre de
/// [`JUPE_NERVURE`], et un RVac passe a quelques centimetres. Expose pour que
/// le test le lise **a la source** au lieu de recopier la fraction.
///
/// **Test seulement** : aucun dessin n'en a besoin (le trace calcule la paroi
/// lui-meme), seule la verification de garde des moteurs s'en sert. Sans le
/// `cfg`, le binaire la compte morte tout au long de la chaine de reexport.
#[cfg(test)]
pub fn jupe_rayon_interieur(profil: Profil) -> f32 {
    profil.rayon() * (1.0 - JUPE_NERVURE)
}

/// **Jupe moteur** : virole ouverte aux deux bouts, autour de la grappe de
/// propulsion.
///
/// Ouverte, et c'est tout l'intérêt : on voit les cloches à l'intérieur. Un
/// cylindre plein les cacherait entièrement, ce qui reviendrait à ne pas les
/// avoir modelées. Elle est donc dessinée **à double paroi** — extérieur et
/// intérieur — sinon on verrait à travers par l'arrière, le rendu ne faisant pas
/// de faces doubles.
pub(super) fn jupe_dessiner<P: Peintre>(p: &mut P, profil: Profil, longueur: f32) {
    let r = profil.rayon();
    let demi = longueur * 0.5;
    let z0 = -demi;
    let n_vir = (longueur / VIROLE).round().max(1.0) as usize;

    // Paroi extérieure, virole par virole comme le fût : même pas, donc les
    // soudures s'alignent d'une pièce à l'autre.
    let mut sommets = Vec::new();
    let mut indices = Vec::new();
    let mut prec = couronne(z0, r);
    for k in 1..=n_vir {
        let cur = couronne(z0 + longueur * k as f32 / n_vir as f32, r);
        ceinture(&prec, &cur, &mut sommets, &mut indices);
        prec = cur;
    }
    p.triangles(&sommets, &indices, INOX);

    // Paroi intérieure, légèrement rentrée et **plus sombre** : c'est un fond de
    // jupe, il est dans l'ombre de lui-même.
    let ri = r * (1.0 - JUPE_NERVURE);
    let mut si = Vec::new();
    let mut ii = Vec::new();
    let mut prec = couronne(z0, ri);
    for k in 1..=n_vir {
        let cur = couronne(z0 + longueur * k as f32 / n_vir as f32, ri);
        ceinture(&prec, &cur, &mut si, &mut ii);
        prec = cur;
    }
    p.triangles(&si, &ii, SOUDURE);

    // Nervure du bord inférieur : un tore de barres, comme les cordons.
    let bas = couronne(z0, r);
    for j in 0..FACETTES {
        p.cylindre(bas[j], bas[(j + 1) % FACETTES], r * JUPE_NERVURE, SOUDURE);
    }
    // Cordons de virole, à l'identique du fût.
    for k in 0..=n_vir {
        let c = couronne(z0 + longueur * k as f32 / n_vir as f32, r * (1.0 + SOUDURE_AFFLEURE));
        for j in 0..FACETTES {
            p.cylindre(c[j], c[(j + 1) % FACETTES], r * SOUDURE_SECTION, SOUDURE);
        }
    }
}

/// Deux ports axiaux, comme le fût : la jupe se glisse **entre** la coque et
/// rien du tout (les moteurs se posent à la main, ils ne sont pas dans l'axe).
pub(super) fn jupe_ports(profil: Profil, longueur: f32) -> Vec<Port> {
    let demi = longueur * 0.5;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn jupe_cout(longueur: f32) -> f32 {
    2.0 + longueur
}

pub(super) fn jupe_rayon_local(profil: Profil, longueur: f32) -> f32 {
    (longueur * 0.5).hypot(rayon_hors_tout(profil))
}

// ================================= Volets ==================================

/// **Volet de rentrée** : le panneau articulé qui donne au Starship sa
/// silhouette, et la seule pièce du vaisseau qui **bouge**.
///
/// # Ce n'est pas une aile
///
/// Un volet de Starship ne porte pas : il **traîne**. À la rentrée, l'engin
/// tombe à plat, ventre vers le bas, et les quatre volets modulent leur traînée
/// pour tenir l'assiette — deux courts à l'avant, deux longs à l'arrière. C'est
/// pour ça qu'ils sont épais et trapus là où une aile serait fine et élancée :
/// un profil d'aile ici lirait comme un avion, ce que ce vaisseau n'est pas.
///
/// # Le braquage
///
/// ⚠️ **Le sens était décrit à l'envers ici.** `braquage` vaut 0 quand le volet
/// est **droit** — tendu dans l'axe de sa charnière, perpendiculaire à la coque —
/// et 1 quand il est **braqué** de [`VOLET_BRAQUAGE_MAX`] vers le ventre. Ce
/// n'est pas « replié / déployé » : c'est l'angle de la gouverne, et un
/// Starship au sol l'a à zéro.
///
/// L'articulation est une vraie rotation autour de la charnière, pas un
/// glissement : c'est la charnière qui reste fixe, et le bout qui décrit un arc.
///
/// L'angle maximal est volontairement inférieur à 90° — un volet perpendiculaire
/// à la coque n'existe sur aucune image de l'engin, et lit comme une avarie.
pub const VOLET_BRAQUAGE_MAX: f32 = 75.0 * PI / 180.0;

/// Épaisseur du volet, en fraction de sa longueur. Un volet de rentrée est un
/// caisson structural, pas une tôle : trop mince, il disparaît de profil.
const VOLET_EPAISSEUR: f32 = 0.13;

/// Fraction de la corde qui se rétrécit vers le bout. Le volet n'est pas un
/// rectangle : son bord d'attaque est incliné, ce qui est **la** signature de
/// la silhouette vue de côté.
const VOLET_EFFILEMENT: f32 = 0.62;

/// Ardoise du bouclier thermique : les volets sont sur la face au vent, donc
/// tuilés dessus et inox dessous.
const TUILE: Color = Color { r: 0.13, g: 0.13, b: 0.15, a: 1.0 };

/// Un volet, dessiné dans son repère local : charnière sur l'axe X, le volet
/// s'étendant vers `+Y` au repos et pivotant autour de X.
///
/// `longueur` est l'envergure (de la charnière au bout), `corde` la dimension le
/// long de l'axe du vaisseau.
pub(super) fn volet_dessiner<P: Peintre>(p: &mut P, longueur: f32, corde: f32, braquage: f32) {
    let angle = braquage.clamp(0.0, 1.0) * VOLET_BRAQUAGE_MAX;
    let (sa, ca) = angle.sin_cos();
    // Le volet s'ouvre en tournant autour de la charnière (axe X) : le bout
    // s'écarte de la coque et recule légèrement.
    let ecart = |t: f32| -> Vec3 { vec3(0.0, longueur * t * ca, -longueur * t * sa) };

    let ep = corde * VOLET_EPAISSEUR;
    let demi_pied = corde * 0.5;
    let demi_bout = demi_pied * VOLET_EFFILEMENT;

    // Quatre coins du pied (a la charniere) et quatre du bout.
    let coins = |t: f32, demi: f32| -> [Vec3; 4] {
        let c = ecart(t);
        [
            c + vec3(-demi, 0.0, ep * 0.5),
            c + vec3(demi, 0.0, ep * 0.5),
            c + vec3(demi, 0.0, -ep * 0.5),
            c + vec3(-demi, 0.0, -ep * 0.5),
        ]
    };
    let pied = coins(0.0, demi_pied);
    let bout = coins(1.0, demi_bout);

    // Caisson : les quatre flancs, plus le bout. Le pied reste ouvert, il est
    // contre la coque.
    let mut s = Vec::new();
    let mut i = Vec::new();
    let mut quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
        let n = s.len() as u16;
        s.extend_from_slice(&[a, b, c, d]);
        i.extend_from_slice(&[n, n + 1, n + 2, n, n + 2, n + 3]);
    };
    for k in 0..4 {
        let k1 = (k + 1) % 4;
        quad(pied[k], pied[k1], bout[k1], bout[k]);
    }
    quad(bout[0], bout[1], bout[2], bout[3]);
    p.triangles(&s, &i, INOX);

    // Face au vent : une nappe de tuiles noires, legerement decollee. C'est
    // elle qu'on voit d'en dessous, et elle porte tout le contraste du vaisseau.
    let sur = ep * 0.5 + corde * 0.006;
    let mut st = Vec::new();
    let mut it = Vec::new();
    let n = st.len() as u16;
    st.extend_from_slice(&[
        vec3(-demi_pied, 0.0, -sur),
        vec3(demi_pied, 0.0, -sur),
        ecart(1.0) + vec3(demi_bout, 0.0, -sur),
        ecart(1.0) + vec3(-demi_bout, 0.0, -sur),
    ]);
    it.extend_from_slice(&[n, n + 1, n + 2, n, n + 2, n + 3]);
    p.triangles(&st, &it, TUILE);

    // Ligne de charniere : le fut du verin, visible entre coque et volet.
    p.cylindre(
        vec3(-demi_pied, 0.0, 0.0),
        vec3(demi_pied, 0.0, 0.0),
        ep * 0.35,
        SOUDURE,
    );
}

/// **Bout du volet** dans son repère local, braquage compris.
///
/// Source unique : le dessin et l'enveloppe s'en servent tous deux. Recopier
/// l'arc dans l'enveloppe la ferait mentir dès qu'on retoucherait le braquage.
pub fn volet_bout(longueur: f32, braquage: f32) -> Vec3 {
    let angle = braquage.clamp(0.0, 1.0) * VOLET_BRAQUAGE_MAX;
    let (sa, ca) = angle.sin_cos();
    vec3(0.0, longueur * ca, -longueur * sa)
}

/// Rayon de la capsule d'un volet : sa demi-corde, **épaisseur comprise**.
///
/// Les coins du caisson sont à `±corde/2` en X et `±ép/2` en Z de l'axe : c'est
/// donc l'hypoténuse qui compte, pas la demi-corde seule. Et la nappe de tuiles
/// décolle encore un peu plus loin.
pub fn volet_rayon_capsule(corde: f32) -> f32 {
    let demi_ep = corde * VOLET_EPAISSEUR * 0.5 + corde * 0.006;
    (corde * 0.5).hypot(demi_ep)
}

/// Envergure hors-tout d'un volet, quel que soit son braquage : c'est la
/// longueur, la rotation ne fait que la répartir entre `Y` et `Z`.
pub(super) fn volet_rayon_local(longueur: f32, corde: f32) -> f32 {
    longueur + volet_rayon_capsule(corde)
}

pub(super) fn volet_cout(longueur: f32, corde: f32) -> f32 {
    1.0 + longueur * corde * 0.5
}

/// Pas de port : un volet se pose à la main sur le flanc de la coque, comme la
/// [`Composant::Charniere`] dont il est le cousin.
pub(super) fn volet_ports() -> Vec<Port> {
    Vec::new()
}

// ============================= Bardage de tuiles =============================

/// Arc couvert par le bardage, en radians — un peu plus d'une demi-circonférence.
///
/// Un Starship ne rentre pas nez en avant : il tombe **à plat, ventre au vent**,
/// et seule cette face voit le plasma. Les tuiles ne couvrent donc qu'un côté —
/// c'est ce contraste noir/inox qui fait reconnaître l'engin, bien plus que sa
/// silhouette. Un peu plus de 180° parce que le flux déborde sur les flancs.
pub const BARDAGE_ARC: f32 = PI * 1.15;

/// De combien les tuiles décollent de la peau, en fraction du rayon. Elles sont
/// **posées dessus**, pas encastrées : sans ce décalage elles se battraient avec
/// la coque dans le tampon de profondeur (z-fighting) et clignoteraient.
const BARDAGE_EPAISSEUR: f32 = 0.012;

/// Largeur visée d'une tuile, en unités monde (~30 cm réels). Le **nombre** s'en
/// déduit, si bien qu'une coque plus large porte plus de tuiles et non des
/// tuiles plus grandes — même raisonnement que [`VIROLE`].
const BARDAGE_TUILE: f32 = 0.14;

/// **Bardage de tuiles** : la face au vent, sur l'arc `arc` centré sur `+X`,
/// couvrant le fût (`longueur`) **et l'ogive** (`nez`, 0 pour s'arrêter au
/// raccord).
///
/// `+X` et pas un azimut réglable : le ventre du vaisseau est une direction
/// unique, et la donner en paramètre inviterait à poser deux boucliers qui ne
/// regardent pas au même endroit. Les volets s'alignent sur cette convention.
pub(super) fn bardage_dessiner<P: Peintre>(
    p: &mut P,
    profil: Profil,
    longueur: f32,
    nez: f32,
    arc: f32,
) {
    let r0 = profil.rayon() * (1.0 + BARDAGE_EPAISSEUR);
    // Le repère est celui du fût ; l'ogive le prolonge vers `+Z`.
    let demi = longueur * 0.5;
    let arc = arc.clamp(0.0, TAU);

    // Découpage : autant de colonnes que de tuiles dans l'arc, autant de rangs
    // que de tuiles dans la hauteur — **ogive comprise**. La nappe de tuiles
    // d'un Starship est continue du fût à la pointe ; la couper au raccord
    // laisserait un nez d'inox nu que la rentrée n'aimerait pas.
    let cols = ((arc * r0) / BARDAGE_TUILE).round().max(3.0) as usize;
    let total = longueur + nez;
    let rangs = (total / BARDAGE_TUILE).round().max(1.0) as usize;

    let point = |ic: usize, ir: usize| -> Vec3 {
        let a = -arc * 0.5 + arc * ic as f32 / cols as f32;
        let h = total * ir as f32 / rangs as f32; // hauteur depuis le bas du fût
        // Sous le raccord : rayon constant. Au-dessus : la même ogive tangente
        // que la coque, lue à la source (`ogive`) et non réinventée ici.
        let r = if h <= longueur || nez <= 1e-4 {
            r0
        } else {
            let x = h - longueur; // distance depuis le raccord
            ogive(profil.rayon(), nez, nez - x).max(0.0) * (1.0 + BARDAGE_EPAISSEUR)
        };
        vec3(r * a.cos(), r * a.sin(), -demi + h)
    };

    let mut s = Vec::new();
    let mut i = Vec::new();
    for ir in 0..rangs {
        for ic in 0..cols {
            let n = s.len() as u16;
            s.extend_from_slice(&[
                point(ic, ir),
                point(ic + 1, ir),
                point(ic + 1, ir + 1),
                point(ic, ir + 1),
            ]);
            // Double face : de l'intérieur de la coque on doit voir le dos des
            // tuiles et non le vide, la vue pouvant passer derrière un volet.
            i.extend_from_slice(&[n, n + 1, n + 2, n, n + 2, n + 3, n, n + 2, n + 1, n, n + 3, n + 2]);
        }
    }
    p.triangles(&s, &i, TUILE);

    // Les deux bords longitudinaux : la ligne de rupture noir/inox est ce qui se
    // lit le mieux de trois quarts, elle mérite une arête franche.
    for cote in [0usize, cols] {
        for ir in 0..rangs {
            p.cylindre(point(cote, ir), point(cote, ir + 1), r0 * BARDAGE_EPAISSEUR, SOUDURE);
        }
    }
}

/// Le bardage épouse la coque : son enveloppe est celle du fût qu'il couvre.
pub(super) fn bardage_rayon_local(profil: Profil, longueur: f32, nez: f32) -> f32 {
    // La nappe déborde de `nez` vers +Z : la capsule doit l'englober, sans quoi
    // `les_rayons_declares_contiennent_la_piece` le dira.
    ((longueur + nez) * 0.5 + longueur * 0.5).hypot(profil.rayon() * (1.0 + BARDAGE_EPAISSEUR))
}

pub(super) fn bardage_cout(longueur: f32, arc: f32) -> f32 {
    // Une tuile se paie à l'unité, donc à la surface couverte.
    2.0 + longueur * arc * 0.6
}

/// Aucun port : un bardage se colle sur une coque, il ne s'accouple pas.
pub(super) fn bardage_ports() -> Vec<Port> {
    Vec::new()
}

// ================================= Hublot ==================================

/// Vitrage : bleu très sombre. Repris **à l'identique** de `equipage.rs` —
/// deux teintes de verre sur un même parc de briques se verraient.
const VITRE: Color = Color { r: 0.16, g: 0.22, b: 0.30, a: 1.0 };

/// **Hublot** : un vitrage encastré dans un cadre, regardant `+Z`.
///
/// Le Starship en porte deux petits en haut du fût, côté ventre — la seule
/// tache froide d'un engin qui n'est qu'inox et tuiles. Sans eux la coque n'a
/// aucune échelle humaine : rien ne dit qu'un équipage tient dedans.
pub(super) fn hublot_dessiner<P: Peintre>(p: &mut P, rayon: f32) {
    // Vitrage : une plaque mince, à peine sortie de la peau.
    p.cube(Vec3::ZERO, vec3(rayon, rayon, rayon * 0.25), VITRE);
    // Cadre : quatre barres autour, en inox. Un hublot sans cadre lit comme un
    // trou dans la coque.
    let c = rayon * 1.05;
    let e = rayon * 0.16;
    for (a, b) in [
        (vec3(-c, -c, 0.0), vec3(c, -c, 0.0)),
        (vec3(c, -c, 0.0), vec3(c, c, 0.0)),
        (vec3(c, c, 0.0), vec3(-c, c, 0.0)),
        (vec3(-c, c, 0.0), vec3(-c, -c, 0.0)),
    ] {
        p.cylindre(a, b, e, SOUDURE);
    }
}

pub(super) fn hublot_rayon_local(rayon: f32) -> f32 {
    rayon * 1.5
}

pub(super) fn hublot_cout(rayon: f32) -> f32 {
    1.0 + rayon * rayon * 4.0
}

/// Aucun port : un hublot se perce dans une coque.
pub(super) fn hublot_ports() -> Vec<Port> {
    Vec::new()
}
