//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22).
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

use crate::disque::DisqueConfig;
use crate::etoile::UA;
use crate::genese::masse_luminosite;
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre (la vue isolée cadre d'ordinaire un astre seul, pas un disque de
/// 8 UA).
pub struct Prototype {
    pub nom: &'static str,
    pub construire: fn() -> Systeme,
    pub distance: f32,
}

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
pub fn prototypes() -> Vec<Prototype> {
    // Lot 12.2 : le disque protoplanétaire, quatre façons de lui donner du
    // volume et de la vie. Le cadre : 2,2 fois son rayon externe.
    let d = 8.0 * UA * 2.2;
    vec![
        Prototype { nom: "Disque proto A : voile bouillonnant", construire: || disque(proto().avec_nuages(0.8)), distance: d },
        Prototype { nom: "Disque proto B : disque epais (couches)", construire: || disque(proto().avec_couches(7, 0.08)), distance: d },
        Prototype { nom: "Disque proto C : nuages de poussiere 3D", construire: || disque(proto().avec_nuages_3d(500, 0.08)), distance: d },
        Prototype {
            nom: "Disque proto D : les trois",
            construire: || disque(proto().avec_nuages(0.8).avec_couches(7, 0.08).avec_nuages_3d(500, 0.08)),
            distance: d,
        },
    ]
}

/// Le disque protoplanétaire de la galerie des disques, sillons compris :
/// la base commune des prototypes du lot 12.2.
fn proto() -> DisqueConfig {
    DisqueConfig::protoplanetaire(5000, 0.3 * UA, 8.0 * UA, masse_luminosite(0.6))
        .avec_lacune_ondulee(0.38, 0.05, 0.92, 0.35)
        .avec_lacune_ondulee(0.66, 0.06, 0.92, 0.35)
}

/// Une jeune étoile (T Tauri, L = 0,6) portant ce disque.
fn disque(cfg: DisqueConfig) -> Systeme {
    let mut sys = Systeme::new();
    let s = Soleil::new(Vec3::ZERO, 1.6, vec3(1.0, 0.62, 0.3), 0.6).avec_disque(cfg);
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
