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
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre.
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// Rayon des rémanents du catalogue (`etoile::particuliere`).
const R: f32 = 0.42;

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 13 (2026-10-26), les rémanents refaits en 3D — *« le pulsar plus blanc
/// en couleur, avec des pôles qui se courbent un peu par rapport au sens de
/// rotation à une longue distance de l'astre ; l'étoile à neutrons avec un
/// set de couleurs qui la distinguerait de ses homologues bleus ; pour le
/// magnétar, rendre visibles en 3D les boucles magnétiques au lieu d'un
/// sticker »*.
pub fn prototypes() -> Vec<Prototype> {
    let p = |nom: &str, longueur: f32, retard: f32, incl: f32, rotation: f32| Prototype {
        nom: format!("Pulsar {nom}"),
        construire: Box::new(move || {
            seule(Soleil::new(Vec3::ZERO, R, vec3(0.92, 0.96, 1.0), 0.25).avec_faisceaux(longueur, retard, incl, rotation))
        }),
        distance: R * longueur * 1.6,
    };
    let n = |nom: &str, couleur: Vec3, jets: bool| Prototype {
        nom: format!("Etoile a neutrons {nom}"),
        construire: Box::new(move || {
            let s = Soleil::new(Vec3::ZERO, R, couleur, 0.2);
            seule(if jets { s.avec_jets_3d(4.0) } else { s })
        }),
        distance: R * if jets { 14.0 } else { 9.0 },
    };
    let m = |nom: &str, n: u32, torsion: f32, sursauts: f32| Prototype {
        nom: format!("Magnetar {nom}"),
        construire: Box::new(move || {
            let couleur = crate::etoile::couleur_etoile("Magnetar", 40_000.0);
            seule(Soleil::new(Vec3::ZERO, 0.45, couleur, 0.3).avec_boucles_magnetiques(n, torsion, sursauts, 0.35))
        }),
        distance: 0.45 * 12.0,
    };
    let violet = vec3(0.78, 0.6, 1.0);
    vec![
        // Blanc, deux faisceaux en 3D le long d'un axe magnétique incliné,
        // qui tourne ; courbés au loin, à rebours de la rotation.
        p("P1 : blanc, faisceaux legerement courbes", 20.0, 0.6, 0.5, 1.2),
        p("P2 : faisceaux longs, tres courbes", 40.0, 1.6, 0.6, 1.2),
        p("P3 : axe presque aligne, rotation rapide", 25.0, 0.8, 0.25, 2.5),
        // Un jeu de couleurs propre, loin du bleu des deux autres.
        n("N1 : violet", violet, false),
        n("N2 : turquoise", vec3(0.5, 1.0, 0.9), false),
        n("N3 : rose-blanc", vec3(1.0, 0.72, 0.88), false),
        n("N4 : violet + fins jets 3D", violet, true),
        // Des lignes de champ fermées, de la surface à la surface.
        m("M1 : boucles magnetiques 3D", 8, 0.0, 0.0),
        m("M2 : M1 + sursauts", 8, 0.0, 6.0),
        m("M3 : boucles torsadees + sursauts", 10, 1.2, 6.0),
    ]
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
