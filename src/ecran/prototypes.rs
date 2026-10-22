//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22) — Maj+4 pour revenir en arrière.
//!
//! ⚠️ **La règle, posée par l'utilisateur** : cette section n'accueille **que**
//! des prototypes, et un prototype **validé en est retiré** — il passe alors
//! dans le jeu par sa source unique (le tirage, `Variante::appliquer`, la
//! genèse…). Quand la liste est vide, la touche 4 affiche « aucun prototype en
//! cours ». Les galeries montrent ce que le jeu produit ; les essais vivent ici.
//!
//! Les habillages du disque protoplanétaire en sont sortis le 2026-10-23,
//! versés dans la galerie « Ceintures & disques » (`disque::habillage`).
//! Place à la **T Tauri** : son disque, rhabillé avec eux.
//!
//! La liste ne construit rien à la déclaration : chaque entrée porte une
//! fonction de construction, appelée à l'affichage (un `Soleil` exige le
//! contexte graphique, la liste elle-même se teste sans lui).

use crate::disque::DisqueConfig;
use crate::soleil::{Soleil, ACCRETION_PROTO, JETS_PROTO};
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre.
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// Le type travaillé, tel que le catalogue le nomme.
const NOM: &str = "T Tauri (jeune)";

/// Un degré de coton : rhabille le petit disque, `largeur` étant son rayon
/// externe rapporté à celui du jeu (16 rayons stellaires) — les nuages
/// suivent, pour garder leur densité.
type Coton = fn(DisqueConfig, f32) -> DisqueConfig;

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 12, la T Tauri. Premier tour (2026-10-23) : T0 la T Tauri du jeu, T1
/// son petit disque en couches + nuages cotonneux, T2 en ouragan, T3 avec le
/// grand disque (PDS 70), T4 avec trois anneaux (HL Tau). Jugé : *« le proto
/// T1 est le mieux, même si ça manque de largeur et d'un côté cotonneux. T1
/// devient T0 »*. Les autres partent de lui, pour avancer pas à pas : plus
/// large, plus cotonneux, puis les deux.
pub fn prototypes() -> Vec<Prototype> {
    let r = gabarit().2;
    let t = |nom: &str, externe: f32, coton: Coton| Prototype {
        nom: format!("T Tauri {nom}"),
        construire: Box::new(move || jeune(externe, coton)),
        // Le disque se cadre à 2,2 fois son rayon externe.
        distance: r * externe * 2.2,
    };
    vec![
        t("T0 : couches + nuages cotonneux (retenu)", 16.0, cotonneux),
        t("T1 : T0 1,5 fois plus large", 24.0, cotonneux),
        t("T2 : T0 2 fois plus large", 32.0, cotonneux),
        t("T3 : T0 plus cotonneux", 16.0, plus_cotonneux),
        t("T4 : 1,5 fois plus large, plus cotonneux", 24.0, plus_cotonneux),
        t("T5 : 2 fois plus large, tres cotonneux", 32.0, tres_cotonneux),
    ]
}

/// **T0** : l'habillage F de la galerie — 500 grappes de six boules,
/// ±37 % du rayon, voile adouci.
fn cotonneux(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    DisqueConfig { granulation: 0.15, ..c }.avec_nuages_3d((500.0 * largeur) as u32, 0.25).avec_grappe(6)
}

/// Plus cotonneux : plus de grappes, de huit boules, et des nuages ×1,4.
fn plus_cotonneux(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    DisqueConfig { granulation: 0.15, ..c }
        .avec_nuages_3d((800.0 * largeur) as u32, 0.25)
        .avec_grappe(8)
        .avec_aspect_nuages(1.4, 0.0)
}

/// Très cotonneux : encore plus de grappes, plus grosses et plus épaisses.
fn tres_cotonneux(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    DisqueConfig { granulation: 0.1, ..c }
        .avec_nuages_3d((1200.0 * largeur) as u32, 0.3)
        .avec_grappe(8)
        .avec_aspect_nuages(1.7, 0.0)
}

/// `(tmin, tmax, rayon, luminosité, nom, mode)` de la T Tauri, pris au
/// catalogue plutôt que recopié.
fn gabarit() -> crate::etoile::Gabarit {
    crate::etoile::gabarit_du_type(NOM, 0.5).expect("la T Tauri est au catalogue")
}

/// **La T Tauri**, ses jets, ses taches d'accrétion, et son petit disque en
/// couches, porté à `externe` rayons stellaires et rhabillé de `coton`.
fn jeune(externe: f32, coton: Coton) -> Systeme {
    let (tmin, tmax, rayon, lumi, _, _) = gabarit();
    let s = Soleil::new(Vec3::ZERO, rayon, crate::etoile::couleur_etoile(NOM, (tmin + tmax) * 0.5), lumi)
        .avec_jets_3d(JETS_PROTO)
        .avec_accretion_polaire(ACCRETION_PROTO);
    let mut c = s.config_protosolaire();
    let largeur = externe * rayon / c.externe;
    c.externe = externe * rayon;
    let c = coton(c.avec_couches(7, 0.08), largeur);
    let mut sys = Systeme::new();
    sys.ajouter(Box::new(s.avec_disque(c)));
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

    /// **Plus large, mêmes nuages au mètre carré près** : un disque élargi
    /// porte plus de grappes, à proportion — sinon il s'éclaircirait.
    #[test]
    fn un_disque_elargi_garde_ses_nuages() {
        let c = DisqueConfig::protosolaire(0, 3.0, 16.0, 1000.0);
        assert_eq!(cotonneux(c, 1.0).nuages_3d, 500);
        assert_eq!(cotonneux(c, 2.0).nuages_3d, 1000);
        assert!(tres_cotonneux(c, 1.0).nuages_3d > plus_cotonneux(c, 1.0).nuages_3d);
        assert!(plus_cotonneux(c, 1.0).nuages_3d > cotonneux(c, 1.0).nuages_3d);
    }
}
