//! Constantes de palette et de forme **partagées** par les familles de
//! composants, plus les quelques helpers géométriques qu'aucune famille ne
//! possède à elle seule.
//!
//! Tout ce qui n'est utilisé que par une seule famille vit dans son module :
//! ici, on ne garde que ce qui serait dupliqué autrement.

use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};


/// Épaisseur des traits (nervures, arêtes) une fois cuits en géométrie.
/// Ignorée par la sortie immédiate, qui trace un segment d'un pixel.
pub(super) const TRAIT_FIN: f32 = 0.02;

// Palette provisoire (les styles arriveront à l'Étape 5).
pub(super) const COULEUR: Color = Color { r: 0.85, g: 0.85, b: 0.88, a: 1.0 };
pub(super) const SOMBRE: Color = Color { r: 0.25, g: 0.25, b: 0.28, a: 1.0 };
/// Dessus d'une tuile de bardage : un gris clair, proche de la coque mais pas
/// identique — le bardage doit se lire comme un revêtement, pas comme la peau.
pub(super) const TUILE_DESSUS: Color = Color { r: 0.72, g: 0.73, b: 0.78, a: 1.0 };
/// Flanc d'une tuile. C'est **lui** qui donne le relief : sans contraste entre
/// le dessus et la tranche, un bardage à épaisseur relit comme un aplat.
pub(super) const TUILE_FLANC: Color = Color { r: 0.40, g: 0.41, b: 0.45, a: 1.0 };

/// Coque **pleine** aux jonctions du tore : là où un bras arrive, il n'y a pas
/// de vitrage — on veut lire une reprise d'effort, donc du blanc franc, plus
/// clair que la coque courante qui, elle, n'est qu'un flanc.
pub(super) const COQUE_JONCTION: Color = Color { r: 0.93, g: 0.94, b: 0.95, a: 1.0 };
/// Meneaux et lèvres du vitrage : la menuiserie. Sombre pour découper les
/// panneaux — un cadre clair sur un vitrage clair ne se voit pas.
pub(super) const MENEAU: Color = Color { r: 0.38, g: 0.40, b: 0.44, a: 1.0 };

/// Face **interne** d'un tore d'habitat : le pont, celui qui regarde l'axe.
/// Teinte à part pour qu'il se distingue de la coque — et point d'entrée de la
/// translucidité à venir (le vitrage d'un tore de Stanford).
pub(super) const PONT: Color = Color { r: 0.30, g: 0.55, b: 0.85, a: 1.0 };
/// Alu clair des bagues d'accostage (bout des collerettes).
pub(super) const BAGUE: Color = Color { r: 0.62, g: 0.64, b: 0.68, a: 1.0 };
/// Teinte des conteneurs de fret : un gris **plus chaud et plus sourd** que la
/// coque pressurisée — le fret ne se confond pas avec l'habitat.
pub(super) const CARGO: Color = Color { r: 0.60, g: 0.58, b: 0.55, a: 1.0 };
/// Coque **composite** des modules d'habitat : os/ivoire, franchement **non
/// métallique**. Ce n'est pas qu'une variation de palette — l'habitat du vrai
/// ISV est bâti quasiment sans métal, pour que les rayons cosmiques n'y
/// produisent pas de rayonnement secondaire. La teinte doit donc le distinguer
/// au premier coup d'œil de tout le reste du vaisseau, qui est en alu.
pub(super) const COMPOSITE: Color = Color { r: 0.80, g: 0.78, b: 0.73, a: 1.0 };
/// Armatures et ferrures des modules d'habitat : **gris sombre**. J'avais
/// d'abord pris un métal moyen, en pensant qu'un ton franc salirait le
/// composite clair ; à l'écran c'est l'inverse — le contraste fort est ce qui
/// détache les cadres de la coque et les fait lire comme de la structure.
pub(super) const HABITAT_ARMATURE: Color = Color { r: 0.24, g: 0.24, b: 0.27, a: 1.0 };
/// Bande de repérage à mi-module : **jaune** de marquage. La seule couleur
/// franche du vaisseau — elle donne l'échelle et casse le monochrome, comme les
/// bandes peintes sur les lanceurs.
pub(super) const HABITAT_BANDE: Color = Color { r: 0.88, g: 0.74, b: 0.16, a: 1.0 };

// Panneau solaire : mât entre le montage et le pied de la pale, et bras de base
// côté hôte (−Z) qui matérialise la jonction module ↔ panneau.
pub(super) const MAST_PANNEAU: f32 = 0.4;
pub(super) const BASE_ARM_PANNEAU: f32 = 0.3;
/// Hauteur de la platine de fixation d'un caisson : il est boulonné court sur sa
/// structure, contrairement à un panneau qui se déploie au bout d'un mât.
pub(super) const CAISSON_PLATINE: f32 = 0.16;

// Treillis : demi-section (fraction du rayon) et distance visée entre paires
// de montages d'ailes le long de la poutre.
pub(super) const TREILLIS_SECTION: f32 = 0.5;
pub(super) const TREILLIS_PAS_AILE: f32 = 2.25;

// Collerette de docking à chaque écoutille axiale : un col plus étroit qui
// dépasse du corps. Le port se pose à son **extrémité** → pincement net (offset
// visible) à chaque joint, et deux modules dockés forment un col reconnaissable
// au lieu d'un tube continu. Dimensions en fraction du rayon du module.
pub(super) const COL_LONG: f32 = 0.25; // longueur du col
pub(super) const COL_RAYON: f32 = 0.45; // rayon du col

// Embout coiffant chaque disque de bout : un petit cylindre qui **chevauche** le
// corps (aucune face coplanaire → pas de z-fighting, cause du halo bizarre) et
// déborde légèrement pour marquer l'arête. Fractions du rayon.
pub(super) const EMBOUT_LONG: f32 = 0.08; // dépassement hors du corps
pub(super) const EMBOUT_ENFONCE: f32 = 0.03; // chevauchement dans le corps
pub(super) const EMBOUT_RAYON: f32 = 1.02; // léger débord radial

// Nœud : la sphère centrale fait 1.2× le rayon de profil (hub plus présent) ; les
// bras partent de sa surface.
pub(super) const NOEUD_SPHERE: f32 = 1.2; // rayon de la sphère, en fraction du rayon de profil
pub(super) const JONCTION_OFFSET: f32 = 0.2; // enfoncement de la base du bras dans la sphère

// Bras cylindrique d'une sortie de nœud : un vrai tronçon entre la sphère et la
// collerette de docking, pour que le hub ait de la présence. Fractions du rayon.
pub(super) const BRAS_LONG: f32 = 0.45; // longueur du bras
pub(super) const BRAS_RAYON: f32 = 0.6; // rayon du bras


pub(super) fn faces_principales() -> [(Vec3, Quat); 6] {
    [
        (Vec3::X, Quat::from_rotation_y(FRAC_PI_2)),
        (Vec3::NEG_X, Quat::from_rotation_y(-FRAC_PI_2)),
        (Vec3::Y, Quat::from_rotation_x(-FRAC_PI_2)),
        (Vec3::NEG_Y, Quat::from_rotation_x(FRAC_PI_2)),
        (Vec3::Z, Quat::IDENTITY),
        (Vec3::NEG_Z, Quat::from_rotation_y(PI)),
    ]
}

