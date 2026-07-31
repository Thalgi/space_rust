//! **Panache d'antimatière** : le jet qui sort d'une tuyère à pleine poussée.
//!
//! Repère local : la tuyère est à l'origine, le jet part le long de **+Z**.
//!
//! **Ce n'est pas une flamme.** Une flamme chimique brûle dans une atmosphère et
//! s'y trouve freinée, comprimée, striée de disques de Mach. Ici il n'y a ni
//! comburant ni pression ambiante : l'annihilation matière/antimatière produit
//! des pions chargés qu'une tuyère **magnétique** collimate, et qui partent
//! ensuite en ligne droite dans le vide. Trois conséquences de forme, et aucune
//! n'est décorative :
//!
//! - **pas de disques de Mach**. Ils demandent une pression extérieure pour
//!   recomprimer le jet ; dans le vide il n'y en a pas. Un panache perlé serait
//!   le dessin d'un moteur-fusée atmosphérique ;
//! - **détente libre et lente**. Le jet ne s'ouvre qu'au rythme où le champ le
//!   lâche, d'où un cône très fin sur une très grande longueur — c'est ce qui
//!   fait qu'il **dépasse le vaisseau** au lieu de lui faire une queue courte ;
//! - **il s'éteint en refroidissant** : blanc-bleu, bleu, violet, magenta
//!   sombre. C'est la séquence d'un plasma qui se détend et perd sa température
//!   de couleur.
//!
//! ⚠️ Ce module ne **dessine** rien : il porte la pose et les cotes, le rendu
//! est fait en additif par `ecran::panache`. Voir [`dessiner`].

use crate::vaisseau::peintre::Peintre;
use macroquad::prelude::*;

/// Étapes de la rampe de couleur, du col vers le bout.
///
/// L'ordre suit une **température décroissante** : blanc-bleu au plus chaud,
/// puis bleu, violet, magenta sombre, et enfin le noir du fond. C'est la
/// séquence d'un plasma qui se détend, et c'est aussi ce qui donne au panache sa
/// profondeur — un jet d'une seule couleur lit comme un tube posé sur l'image.
const RAMPE: [(f32, Color); 5] = [
    (0.00, Color { r: 0.92, g: 0.96, b: 1.00, a: 1.0 }),
    (0.08, Color { r: 0.55, g: 0.74, b: 1.00, a: 1.0 }),
    (0.30, Color { r: 0.44, g: 0.34, b: 0.95, a: 1.0 }),
    (0.62, Color { r: 0.26, g: 0.09, b: 0.36, a: 1.0 }),
    (1.00, Color { r: 0.03, g: 0.01, b: 0.05, a: 1.0 }),
];

/// Exposant de la détente : `t^EVASEMENT`. Au-dessus de 1, le jet reste **serré
/// près du col** puis s'ouvre — c'est le comportement d'un plasma tenu par un
/// champ qui le lâche progressivement. À 1 (cône droit) il s'ouvrirait dès la
/// sortie, ce qui est le dessin d'une tuyère sans confinement.
const EVASEMENT: f32 = 1.45;

/// Teinte à la fraction `t` du panache.
pub fn teinte(t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let i = RAMPE.iter().position(|(s, _)| *s >= t).unwrap_or(RAMPE.len() - 1);
    if i == 0 {
        return RAMPE[0].1;
    }
    let ((s0, a), (s1, b)) = (RAMPE[i - 1], RAMPE[i]);
    let k = if (s1 - s0).abs() < 1e-6 { 0.0 } else { (t - s0) / (s1 - s0) };
    Color { r: a.r + (b.r - a.r) * k, g: a.g + (b.g - a.g) * k, b: a.b + (b.b - a.b) * k, a: 1.0 }
}

/// Rayon du jet à la fraction `t`.
pub fn rayon(rayon_col: f32, rayon_bout: f32, t: f32) -> f32 {
    rayon_col + (rayon_bout - rayon_col) * t.clamp(0.0, 1.0).powf(EVASEMENT)
}

/// **Ne dessine rien.**
///
/// Un jet de plasma n'a pas de silhouette, et c'est tout le problème d'en faire
/// de la géométrie : le premier jet était une pile de cônes pleins, et ce sont
/// précisément les qualités d'un solide — arête nette, face opaque, bord franc
/// sur le fond étoilé — qui le faisaient lire comme un tube de plastique planté
/// dans la tuyère.
///
/// Il est donc rendu comme les **jets bipolaires de pulsar** : un ruban
/// face-caméra en additif, monté par `ecran::panache`. Le composant ne sert plus
/// qu'à **porter la pose** — où est la tuyère, où va le jet, à quelles cotes —
/// dans l'assemblage, ce qui est justement ce qu'un `Composant` sait faire et
/// qu'un effet d'écran ne saurait pas.
pub(super) fn dessiner<P: Peintre>(_p: &mut P) {}

pub(super) fn cout() -> f32 {
    0.0
}

/// Le panache est un **effet**, pas une pièce : il ne coûte rien et ne compte pas
/// dans l'encombrement du vaisseau. Renvoyer sa vraie longueur ferait reculer la
/// caméra de deux longueurs de vaisseau au moment de l'allumage, et on ne verrait
/// plus l'ISV.
pub(super) fn rayon_local() -> f32 {
    0.0
}

pub(super) fn englobant() -> (Vec3, f32) {
    (Vec3::ZERO, 0.0)
}
