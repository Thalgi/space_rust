//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22) — Maj+4 pour revenir en arrière.
//!
//! ⚠️ **La règle, posée par l'utilisateur** : cette section n'accueille **que**
//! des prototypes, et un prototype **validé en est retiré** — il passe alors
//! dans le jeu par sa source unique (le tirage, `Variante::appliquer`, la
//! genèse…). Quand la liste est vide, la touche 4 affiche « aucun prototype en
//! cours ». Les galeries montrent ce que le jeu produit ; les essais vivent ici.
//!
//! Adoptés le 2026-10-24, et donc retirés : la T Tauri en T1B4 (variée par la
//! graine) et la Herbig Ae/Be en T1B2 — `disque::habillage::jeune_etoile`.
//! Restent les essais non validés.
//!
//! La liste ne construit rien à la déclaration : chaque entrée porte une
//! fonction de construction, appelée à l'affichage (un `Soleil` exige le
//! contexte graphique, la liste elle-même se teste sans lui).

use crate::disque::flot::Flot;
use crate::disque::{habillage, DisqueConfig};
use crate::etoile::UA;
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

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
pub fn prototypes() -> Vec<Prototype> {
    let r = gabarit().2;
    // **Le rework des jets polaires** (2026-10-24) : *« ceux-ci canalisent une
    // partie de la poussière et laissent une traînée plus large »* — le
    // standard de tous les jets du chantier. Sur la T Tauri en T1B4 ; cadrés
    // large, le flot dépasse le disque.
    let j = |nom: &str, regler: fn(Flot) -> Flot| Prototype {
        nom: format!("Jets {nom}"),
        construire: Box::new(move || {
            let s = etoile_jeune();
            let c = t1b4(&s);
            let base = Flot { longueur: 1.2 * c.externe, rayon_astre: s.base.rayon, jet: 1.0, ..Flot::default() };
            seule(s.avec_disque(c.avec_flot(regler(base))))
        }),
        distance: r * 70.0 * 2.2,
    };
    vec![
        j("J1 : jet + trainee de poussiere", |f| Flot { cone: 1.0, ..f }),
        j("J2 : J1 + noeuds de Herbig-Haro", |f| Flot { cone: 1.0, noeuds: 5, ..f }),
        j("J3 : J1 + parois eclairees en sablier", |f| Flot { cone: 1.0, parois: 1.0, ..f }),
        j("J4 : tout ensemble", |f| Flot { cone: 1.0, noeuds: 5, parois: 1.0, ..f }),
        // Second tour : les deux lobes ne se reflètent plus (*« ça se voit
        // que c'est une symétrie »*).
        j("J5 : J1 + sablier tres fin, par plaques, lobes dissymetriques", |f| Flot {
            cone: 1.0,
            parois: 1.0,
            finesse: 0.9,
            asymetrie: 1.0,
            ..f
        }),
        // Le bord interne brûlant : l'anneau incandescent au ras de la cavité
        // de l'image d'artiste. Cadré serré : il vit près de l'étoile.
        Prototype {
            nom: "T Tauri T1B4-A : + bord interne brulant".into(),
            construire: Box::new(|| {
                let s = etoile_jeune();
                let c = t1b4(&s).avec_bord_brulant(1.0);
                seule(s.avec_disque(c))
            }),
            distance: r * 24.0 * 2.2,
        },
        // Le grand disque cotonneux, de la lisière du petit (0,9 UA) à 8 UA :
        // l'échelle encore en question.
        Prototype {
            nom: "T Tauri T1B4-G : + grand disque cotonneux (0,9 - 8 UA)".into(),
            construire: Box::new(|| {
                let s = etoile_jeune();
                let petit = t1b4(&s);
                seule(s.avec_disque(petit).avec_disque(habillage::cotonneux(habillage::anneau(0.9, 8.0))))
            }),
            distance: 8.0 * UA * 2.2,
        },
    ]
}

/// `(tmin, tmax, rayon, luminosité, nom, mode)` de la T Tauri, pris au
/// catalogue plutôt que recopié.
fn gabarit() -> crate::etoile::Gabarit {
    crate::etoile::gabarit_du_type(NOM, 0.5).expect("la T Tauri est au catalogue")
}

/// La T Tauri, ses jets et ses taches d'accrétion, sans disque.
fn etoile_jeune() -> Soleil {
    let (tmin, tmax, rayon, lumi, _, _) = gabarit();
    Soleil::new(Vec3::ZERO, rayon, crate::etoile::couleur_etoile(NOM, (tmin + tmax) * 0.5), lumi)
        .avec_jets_3d(JETS_PROTO)
        .avec_accretion_polaire(ACCRETION_PROTO)
}

/// **T1B4 tel que retenu** — cavité de 5 rayons, trois bras, deux flots d'un
/// dipôle incliné de 20° —, sur l'habillage du jeu. Le jeu, lui, le varie
/// par la graine.
fn t1b4(s: &Soleil) -> DisqueConfig {
    let r = s.base.rayon;
    let c = habillage::jeune_etoile(s.config_protosolaire(), r, 0.0, false);
    DisqueConfig { interne: 5.0 * r, ..c }.avec_spirales_accretion(3, r).avec_entonnoirs(2, r, 0.35)
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
