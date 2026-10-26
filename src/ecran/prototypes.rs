//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22) — Maj+4 pour revenir en arrière.
//!
//! ⚠️ **La règle, posée par l'utilisateur** : cette section n'accueille **que**
//! des prototypes, et un prototype **validé en est retiré** — il passe alors
//! dans le jeu par sa source unique (le tirage, `Variante::appliquer`, la
//! genèse…). Quand la liste est vide, la touche 4 affiche « aucun prototype en
//! cours ». Les galeries montrent ce que le jeu produit ; les essais vivent ici.
//!
//! La liste ne construit rien à la déclaration : chaque entrée porte une
//! fonction de construction, appelée à l'affichage (un `Soleil` exige le
//! contexte graphique, la liste elle-même se teste sans lui).

use crate::soleil::Soleil;
use crate::stellaire::Variante;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre.
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// Le rayon d'une naine noire : celui de la naine blanche qu'elle a été.
const RAYON: f32 = 0.65;

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 15 (2026-10-26), la **naine noire** : une naine blanche refroidie au
/// point de ne plus briller. Il faudrait plus que l'âge de l'univers pour en
/// former une — fantaisie assumée, rouverte par l'utilisateur. Elle ne se
/// voit presque pas : il lui faut une représentation à elle (conception
/// §6 undecies) — silhouette, dernière lueur infrarouge, ou surface figée.
pub fn prototypes() -> Vec<Prototype> {
    let k = |nom: &str, couleur: Vec3, lumi: f32, regler: fn(Soleil) -> Soleil| Prototype {
        nom: format!("Naine noire {nom}"),
        construire: Box::new(move || {
            let s = Variante::Normale.appliquer(Soleil::new(Vec3::ZERO, RAYON, couleur, lumi), "Naine blanche").sans_arches();
            let mut sys = Systeme::new();
            sys.ajouter(Box::new(regler(s)));
            sys
        }),
        distance: RAYON * 9.0,
    };
    vec![
        // Un disque presque noir qui masque les étoiles du fond : on la devine
        // par ce qu'elle cache.
        k("K1 : silhouette", vec3(0.05, 0.035, 0.04), 0.02, |s| s.avec_couronne(1.08)),
        // Ce qui lui reste de chaleur, dans l'infrarouge : une braise.
        k("K2 : braise infrarouge", vec3(0.36, 0.07, 0.04), 0.05, |s| s.avec_couronne(1.3)),
        // Une naine blanche cristallise en refroidissant (carbone et oxygène,
        // mesuré par Gaia en 2019) : sa surface figée en facettes.
        k("K3 : cristallisee", vec3(0.2, 0.24, 0.32), 0.04, |s| s.avec_ebullition_du_type("Supergeante bleue").avec_couronne(1.1)),
        // Noire, et sa dernière chaleur qui transparaît au cœur (le `coeur` de
        // la quasi-étoile).
        k("K4 : noire, coeur encore tiede", vec3(0.06, 0.04, 0.05), 0.03, |s| s.avec_coeur(0.5).avec_couronne(1.15)),
    ]
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
