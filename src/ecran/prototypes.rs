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
    // Lot 12.2 : le disque protoplanétaire, plusieurs façons de lui donner du
    // volume et de la vie. Le cadre : 2,2 fois son rayon externe.
    // ⚠️ **A (voile bouillonnant) refusé seul** à l'écran le 2026-10-22, et
    // retiré de la liste — mais **gardé dans D**, à la demande : son code
    // reste, pour D seulement.
    //
    // Validés à l'écran le 2026-10-22 (« pour le moment les cinq protos sont
    // bons »), et renommés d'après ce qu'ils montrent :
    //   B  voile en 7 couches, qui s'évase (h/r 0,08) ;
    //   C  500 nuages épars, Kepler, ±12 % du rayon ;
    //   D  voile bouillonnant (l'ex-A) + 7 couches + 500 nuages ;
    //   E  1 500 nuages épars ;
    //   F  500 grappes de 6 boules éclairées, ±37 % du rayon, voile adouci.
    // Puis trois de plus, demandés le même jour : G, H, I.
    let d = 8.0 * UA * 2.2;
    vec![
        Prototype { nom: "Disque proto B : voile en couches", construire: || disque(proto().avec_couches(7, 0.08)), distance: d },
        Prototype { nom: "Disque proto C : nuages epars", construire: || disque(proto().avec_nuages_3d(500, 0.08)), distance: d },
        Prototype {
            nom: "Disque proto D : voile bouillonnant + couches + nuages",
            construire: || disque(proto().avec_nuages(0.8).avec_couches(7, 0.08).avec_nuages_3d(500, 0.08)),
            distance: d,
        },
        Prototype { nom: "Disque proto E : nuages epars denses", construire: || disque(proto().avec_nuages_3d(1500, 0.08)), distance: d },
        // Demandé le 2026-10-22 : « des nuages plus cotonneux » — chaque nuage
        // devient une grappe de six boules, éclairée d'un côté. Second retour :
        // « un autocollant sur le disque » — d'où une épaisseur triplée
        // (h/r 0,25 : jusqu'à ±37 % du rayon), pour qu'ils sortent du plan.
        Prototype {
            nom: "Disque proto F : nuages cotonneux",
            // Granulation du voile atténuée : ses taches, peintes à plat, se
            // lisaient comme les nuages eux-mêmes — « un autocollant ».
            construire: || {
                disque(DisqueConfig { granulation: 0.15, ..proto() }.avec_nuages_3d(500, 0.25).avec_grappe(6))
            },
            distance: d,
        },
        // « Encore plus cotonneux, moins en orbites mais plus massé sur le
        // disque pour donner l'aspect de gravité » : 14 amas près du plan, qui
        // tournent d'un bloc, grappes de 8, nuages plus gros.
        // Décliné le 2026-10-22 : G1 tel quel, G2 deux fois plus de gros amas,
        // G3 six fois plus — trente nuages par amas, et la même taille.
        Prototype { nom: "Disque proto G1 : 14 amas masses", construire: || amas_masses(14), distance: d },
        Prototype { nom: "Disque proto G2 : 28 amas masses", construire: || amas_masses(28), distance: d },
        Prototype { nom: "Disque proto G3 : 84 amas masses", construire: || amas_masses(84), distance: d },
        // « Encore plus fourni, un aspect artistique de nuages de débris » :
        // 1 600 grappes de 5, grosses, épaisses, aux teintes variées.
        Prototype {
            nom: "Disque proto H : nebuleuse de debris",
            construire: || {
                disque(
                    DisqueConfig { granulation: 0.1, ..proto() }
                        .avec_nuages_3d(1600, 0.2)
                        .avec_grappe(5)
                        .avec_aspect_nuages(1.5, 0.8),
                )
            },
            distance: d,
        },
        // « Un aspect un peu ouragan » : des bras spiraux autour de l'étoile,
        // qui est l'œil, un motif qui tourne d'un bloc. Second retour : plus
        // de bras (7), inégaux, et deux fois plus de nuages, tassés au centre.
        Prototype {
            nom: "Disque proto I : ouragan",
            construire: || {
                disque(
                    DisqueConfig { granulation: 0.15, ..proto() }
                        .avec_nuages_3d(1800, 0.1)
                        .avec_grappe(5)
                        .avec_bras(7)
                        .avec_aspect_nuages(1.2, 0.0),
                )
            },
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

/// **Prototype G** : `n` amas massés près du plan, qui tournent d'un bloc —
/// trente nuages par amas, grappes de 8, nuages ×1,4, voile adouci.
fn amas_masses(n: u32) -> Systeme {
    disque(
        DisqueConfig { granulation: 0.15, ..proto() }
            .avec_nuages_3d(n * 30, 0.12)
            .avec_grappe(8)
            .avec_amas(n)
            .avec_aspect_nuages(1.4, 0.0),
    )
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
