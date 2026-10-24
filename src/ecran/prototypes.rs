//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22) — Maj+4 pour revenir en arrière.
//!
//! ⚠️ **La règle, posée par l'utilisateur** : cette section n'accueille **que**
//! des prototypes, et un prototype **validé en est retiré** — il passe alors
//! dans le jeu par sa source unique (le tirage, `Variante::appliquer`, la
//! genèse…). Quand la liste est vide, la touche 4 affiche « aucun prototype en
//! cours ». Les galeries montrent ce que le jeu produit ; les essais vivent ici.
//!
//! Sortis le 2026-10-24 : la T Tauri (T1B4), la Herbig Ae/Be (T1B2) et le jet
//! standard (J5), adoptés ; le grand disque (T1B4-G), confié à la genèse ; le
//! bord brûlant (T1B4-A), gardé en preset (`avec_bord_brulant`).
//!
//! La liste ne construit rien à la déclaration : chaque entrée porte une
//! fonction de construction, appelée à l'affichage (un `Soleil` exige le
//! contexte graphique, la liste elle-même se teste sans lui).

use crate::disque::flot::Flot;
use crate::disque::{habillage, DisqueConfig};
use crate::genese::masse_luminosite;
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre.
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// L'étoile Be : `(température, rayon, luminosité)` — une B chaude.
const BE: (f32, f32, f32) = (18_000.0, 1.8, 4.0);
/// L'hypergéante d'Eta Carinae, et la longueur de ses lobes, en rayons.
const ETA: (f32, f32, f32) = (20_000.0, 3.0, 8.0);
const LOBES_ETA: f32 = 28.0;

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 12, fin (2026-10-24) : *« fais-moi ensuite l'étoile Be et l'exemple
/// Eta Carinae »*.
pub fn prototypes() -> Vec<Prototype> {
    let be = |nom: &str, aplati: f32, etendue: f32, nuages: u32| Prototype {
        nom: format!("Etoile Be {nom}"),
        construire: Box::new(move || {
            let s = etoile(BE).avec_aplatissement(aplati);
            let c = habillage::decretion(s.base.rayon, s.base.masse, etendue, nuages);
            seule(s.avec_disque(c))
        }),
        distance: BE.1 * etendue * 2.2,
    };
    let eta = |nom: &str, finesse: f32| Prototype {
        nom: format!("Eta Carinae {nom}"),
        construire: Box::new(move || {
            let s = etoile(ETA).avec_vent();
            let c = jupe(&s).avec_flot(homoncule(s.base.rayon, finesse));
            seule(s.avec_disque(c))
        }),
        distance: ETA.1 * LOBES_ETA * 2.6,
    };
    vec![
        // Une B qui tourne vite, aplatie de 20 %, son disque de décrétion
        // de gaz qui brille de lui-même.
        be("B1 : aplatie, disque de decretion", 0.2, 10.0, 0),
        be("B2 : B1 + nappes de gaz", 0.2, 10.0, 300),
        // Achernar, la plus aplatie connue : un tiers de moins aux pôles.
        be("B3 : rotation extreme (Achernar), disque large", 0.35, 16.0, 300),
        // L'hypergéante dans son Homoncule : deux lobes de poussière pincés à
        // l'étoile, et la jupe équatoriale de l'éruption de 1843.
        eta("E1 : lobes du Homoncule + jupe equatoriale", 0.0),
        eta("E2 : lobes en coques fines, eclairees par plaques", 0.6),
    ]
}

/// Une étoile nue, `(température, rayon, luminosité)`, teinte d'une B.
fn etoile((temp, rayon, lumi): (f32, f32, f32)) -> Soleil {
    let mut s = Soleil::new(Vec3::ZERO, rayon, crate::etoile::couleur_etoile("B (bleu-blanc)", temp), lumi);
    s.base.masse = masse_luminosite(lumi);
    s
}

/// **La jupe équatoriale d'Eta Carinae** : un disque de débris rougeâtres,
/// effiloché, autour de la taille des lobes. Il porte aussi le flot.
fn jupe(s: &Soleil) -> DisqueConfig {
    let r = s.base.rayon;
    DisqueConfig {
        voile_alpha: 0.35,
        voile_couleur: vec3(0.6, 0.28, 0.16),
        voile_couleur2: vec3(1.0, 0.6, 0.35),
        granulation: 0.6,
        ..DisqueConfig::protosolaire(0, r * 1.5, r * 0.5 * LOBES_ETA, s.base.masse)
    }
    .avec_nuages_3d(700, 0.04)
    .avec_grappe(5)
    .avec_aspect_nuages(1.2, 0.5)
}

/// **Les lobes du Homoncule** : deux bulbes de poussière, presque
/// symétriques, pleins (`finesse` 0) ou en coques éclairées par plaques.
fn homoncule(rayon: f32, finesse: f32) -> Flot {
    Flot {
        longueur: rayon * LOBES_ETA,
        rayon_astre: rayon,
        cone: 0.6,
        parois: 1.0,
        finesse,
        forme: 1.0,
        asymetrie: 0.15,
        ..Flot::default()
    }
}

/// Un système de cette seule étoile.
fn seule(s: Soleil) -> Systeme {
    let mut sys = Systeme::new();
    sys.ajouter(Box::new(s));
    sys
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Chaque prototype a un nom à lui, et un cadre qui le contient.**
    /// Deux du même nom ne se distingueraient pas à l'écran ; un cadre trop
    /// serré couperait ce qu'on vient juger.
    #[test]
    fn chaque_prototype_a_son_nom_et_son_cadre() {
        let p = prototypes();
        for (i, a) in p.iter().enumerate() {
            assert!(!a.nom.is_empty());
            assert!(!p[..i].iter().any(|b| b.nom == a.nom), "« {} » en double", a.nom);
            assert!(a.distance > 0.0 && a.distance <= crate::camera::DIST_MAX, "« {} » : cadre {}", a.nom, a.distance);
        }
    }
}
