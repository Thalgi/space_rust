//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22) — Maj+4 pour revenir en arrière.
//!
//! ⚠️ **La règle, posée par l'utilisateur** : cette section n'accueille **que**
//! des prototypes, et un prototype **validé en est retiré** — il passe alors
//! dans le jeu par sa source unique (le tirage, `Variante::appliquer`, la
//! genèse…). Quand la liste est vide, la touche 4 affiche « aucun prototype en
//! cours ». Les galeries montrent ce que le jeu produit ; les essais vivent ici.
//!
//! En cours (2026-10-29) : les nuages en basse résolution (Opti-v2 O4).
//!
//! La liste ne construit rien à la déclaration : chaque entrée porte une
//! fonction de construction, appelée à l'affichage (un `Soleil` exige le
//! contexte graphique, la liste elle-même se teste sans lui).

use crate::systeme::Systeme;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre.
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Opti-v2 O4 : la T Tauri, ses nuages et ses bras dessinés à pleine
/// résolution, à ½ et à ¼ par axe (`Disque::nuages_reduits`) — comparer
/// l'image et les images par seconde, à cadrage égal.
pub fn prototypes() -> Vec<Prototype> {
    [(1, "pleine résolution"), (2, "à 1/2"), (4, "à 1/4")]
        .into_iter()
        .filter_map(|(k, nom)| {
            let (construire, distance) =
                super::objet::defile_retouche("T Tauri (jeune)", move |s| s.avec_nuages_reduits(k))?;
            Some(Prototype { nom: format!("T Tauri, nuages {nom}"), construire, distance })
        })
        .collect()
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
