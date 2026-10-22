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

use crate::astre::Foyer;
use crate::disque::DisqueConfig;
use crate::etoile::UA;
use crate::genese::{ajouter_planete_autour, masse_luminosite, preset_gazeuse, preset_tellurique};
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre (la vue isolée cadre d'ordinaire un astre seul, pas un disque de
/// 8 UA).
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// Ce qui fait un prototype de disque : posé sur n'importe quel anneau
/// protoplanétaire, il l'habille (couches, nuages, amas, bras…).
type Habillage = fn(DisqueConfig) -> DisqueConfig;

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 12.2 : le disque protoplanétaire, plusieurs façons de lui donner du
/// volume et de la vie. Chaque habillage se décline en quatre (2026-10-22) :
/// - **base** : l'anneau de la galerie, deux sillons ondulés ;
/// - **-F** (*full*) : l'anneau complet, un seul sillon — celui qu'une
///   planète y creuse ;
/// - **-R** (*ringed*) : deux ou trois anneaux du même habillage, des
///   planètes dispersées entre eux et au-delà ;
/// - **-M** (*mixed*) : deux anneaux de l'habillage parmi d'autres types
///   (ceinture d'astéroïdes, Kuiper), pour l'ajuster au milieu des autres.
pub fn prototypes() -> Vec<Prototype> {
    let mut v = Vec::new();
    for (rang, (code, nom, habiller)) in habillages().into_iter().enumerate() {
        let nb = if rang % 2 == 0 { 3 } else { 2 };
        v.push(Prototype {
            nom: format!("Disque proto {code} : {nom}"),
            construire: Box::new(move || systeme(&[habiller(proto())], &[])),
            distance: cadre(8.0),
        });
        v.push(Prototype {
            nom: format!("Disque proto {code}-F : {nom}, complet + 1 planete"),
            construire: Box::new(move || complet(habiller)),
            distance: cadre(8.0),
        });
        v.push(Prototype {
            nom: format!("Disque proto {code}-R : {nom}, {nb} anneaux + planetes"),
            construire: Box::new(move || annele(habiller, nb)),
            distance: cadre(ANNELE_EXTERNE),
        });
        v.push(Prototype {
            nom: format!("Disque proto {code}-M : {nom}, parmi d'autres anneaux"),
            construire: Box::new(move || mixte(habiller)),
            distance: cadre(MIXTE_EXTERNE),
        });
    }
    v
}

/// Les habillages, dans l'ordre de la touche 4.
///
/// ⚠️ **A (voile bouillonnant) refusé seul** à l'écran le 2026-10-22, et
/// retiré de la liste — mais **gardé dans D**, à la demande.
fn habillages() -> Vec<(&'static str, &'static str, Habillage)> {
    vec![
        // Validés à l'écran le 2026-10-22 (« pour le moment les cinq protos
        // sont bons »), et renommés d'après ce qu'ils montrent.
        ("B", "voile en couches", |c| c.avec_couches(7, 0.08)),
        ("C", "nuages epars", |c| c.avec_nuages_3d(500, 0.08)),
        ("D", "voile bouillonnant + couches + nuages", |c| {
            c.avec_nuages(0.8).avec_couches(7, 0.08).avec_nuages_3d(500, 0.08)
        }),
        ("E", "nuages epars denses", |c| c.avec_nuages_3d(1500, 0.08)),
        // « Des nuages plus cotonneux » : grappes de six boules, éclairées
        // d'un côté ; puis « un autocollant sur le disque » — d'où une
        // épaisseur triplée et la granulation du voile atténuée.
        ("F", "nuages cotonneux", |c| DisqueConfig { granulation: 0.15, ..c }.avec_nuages_3d(500, 0.25).avec_grappe(6)),
        // « Moins en orbites mais plus massé, l'aspect de gravité », décliné
        // en G1 tel quel, G2 deux fois plus de gros amas, G3 six fois plus.
        ("G1", "14 amas masses", |c| amas_masses(c, 14)),
        ("G2", "28 amas masses", |c| amas_masses(c, 28)),
        ("G3", "84 amas masses", |c| amas_masses(c, 84)),
        // « Encore plus fourni, un aspect artistique de nuages de débris ».
        ("H", "nebuleuse de debris", |c| {
            DisqueConfig { granulation: 0.1, ..c }.avec_nuages_3d(1600, 0.2).avec_grappe(5).avec_aspect_nuages(1.5, 0.8)
        }),
        // « Un aspect un peu ouragan » : l'étoile est l'œil ; 7 bras inégaux,
        // 1 800 nuages tassés au centre.
        ("I", "ouragan", |c| {
            DisqueConfig { granulation: 0.15, ..c }
                .avec_nuages_3d(1800, 0.1)
                .avec_grappe(5)
                .avec_bras(7)
                .avec_aspect_nuages(1.2, 0.0)
        }),
    ]
}

/// **Prototype G** : `n` amas massés près du plan, qui tournent d'un bloc —
/// trente nuages par amas, grappes de 8, nuages ×1,4, voile adouci.
fn amas_masses(c: DisqueConfig, n: u32) -> DisqueConfig {
    DisqueConfig { granulation: 0.15, ..c }
        .avec_nuages_3d(n * 30, 0.12)
        .avec_grappe(8)
        .avec_amas(n)
        .avec_aspect_nuages(1.4, 0.0)
}

/// L'étoile : une T Tauri (L = 0,6).
fn masse() -> f32 {
    masse_luminosite(0.6)
}

/// Le cadre d'un système de rayon `externe_ua` : 2,2 fois ce rayon.
fn cadre(externe_ua: f32) -> f32 {
    externe_ua * UA * 2.2
}

/// Un anneau protoplanétaire nu, entre deux rayons (UA). Ses particules
/// suivent sa largeur, pour une densité égale d'un anneau à l'autre.
fn anneau(interne_ua: f32, externe_ua: f32) -> DisqueConfig {
    let part = ((externe_ua - interne_ua) / 7.7 * 5000.0) as usize;
    DisqueConfig::protoplanetaire(part, interne_ua * UA, externe_ua * UA, masse())
}

/// Le disque protoplanétaire de la galerie des disques, sillons compris.
fn proto() -> DisqueConfig {
    anneau(0.3, 8.0).avec_lacune_ondulee(0.38, 0.05, 0.92, 0.35).avec_lacune_ondulee(0.66, 0.06, 0.92, 0.35)
}

/// Une planète du prototype : `(demi-grand axe en UA, géante ?)`.
type Planete = (f32, bool);

/// **-F** : l'anneau complet ; un seul sillon, creusé par une proto-planète
/// en fusion qui y orbite (type PDS 70b). Sillon sans festons : ceux-ci
/// suivent l'horloge du disque, la planète celle du système — ils ne
/// resteraient pas accordés.
fn complet(habiller: Habillage) -> Systeme {
    const T: f32 = 0.5;
    let c = habiller(anneau(0.3, 8.0).avec_lacune(T, 0.07, 0.95));
    let a = c.interne + T * (c.externe - c.interne);
    systeme(&[c], &[(a / UA, false)])
}

/// Le plus grand rayon des systèmes -R et -M (UA).
const ANNELE_EXTERNE: f32 = 12.5;
const MIXTE_EXTERNE: f32 = 13.0;

/// **Les anneaux d'un -R**, en UA, et ses planètes : dans les écarts entre
/// anneaux, et une géante au-delà.
fn plan_annele(nb: usize) -> (Vec<(f32, f32)>, Vec<Planete>) {
    if nb >= 3 {
        (vec![(0.3, 2.2), (3.2, 5.2), (6.4, 9.0)], vec![(2.7, false), (5.8, false), (ANNELE_EXTERNE - 0.5, true)])
    } else {
        (vec![(0.3, 3.5), (5.0, 9.0)], vec![(4.2, false), (ANNELE_EXTERNE - 0.5, true)])
    }
}

/// Partage les nuages d'un habillage entre `n` anneaux (et les amas avec,
/// trente nuages chacun) : sans quoi un -R de G3 porterait trois fois ses
/// 20 000 boules.
fn partage(mut c: DisqueConfig, n: u32) -> DisqueConfig {
    c.nuages_3d /= n;
    if c.nuages_amas > 0 {
        c.nuages_amas = (c.nuages_amas / n).max(1);
    }
    c
}

/// **-R** : deux ou trois anneaux du même habillage.
fn annele(habiller: Habillage, nb: usize) -> Systeme {
    let (anneaux, planetes) = plan_annele(nb);
    let disques: Vec<_> = anneaux.iter().map(|&(i, e)| partage(habiller(anneau(i, e)), nb as u32)).collect();
    systeme(&disques, &planetes)
}

/// **-M** : deux anneaux de l'habillage, une ceinture d'astéroïdes entre eux,
/// une ceinture de Kuiper au-delà ; une planète dans l'écart, une géante.
fn mixte(habiller: Habillage) -> Systeme {
    let disques = [
        partage(habiller(anneau(0.3, 2.8)), 2),
        DisqueConfig::asteroides(900, 3.4 * UA, 4.4 * UA, masse()),
        partage(habiller(anneau(5.5, 8.0)), 2),
        DisqueConfig::kuiper(1400, 10.5 * UA, MIXTE_EXTERNE * UA, masse()),
    ];
    systeme(&disques, &[(4.9, false), (9.3, true)])
}

/// **Une jeune étoile (T Tauri, L = 0,6) portant ces disques, et ces
/// planètes.** Avec des planètes, disques et planètes partagent le plan de
/// l'écliptique : une planète creuse son sillon dans le disque, pas à côté.
fn systeme(disques: &[DisqueConfig], planetes: &[Planete]) -> Systeme {
    let mut sys = Systeme::new();
    let mut s = Soleil::new(Vec3::ZERO, 1.6, vec3(1.0, 0.62, 0.3), 0.6);
    for &c in disques {
        s = if planetes.is_empty() { s.avec_disque(c) } else { s.avec_disque_ecliptique(c) };
    }
    sys.ajouter(Box::new(s));
    for &(a, geante) in planetes {
        // Grossies comme dans la galerie des disques : à l'échelle, une
        // planète ne se lit pas à côté d'un disque de 8 UA.
        let (app, rayon) = if geante { (preset_gazeuse("Jupiter"), 4.0) } else { (preset_tellurique("Lave"), 2.4) };
        ajouter_planete_autour(&mut sys, Foyer::Etoile(0), masse(), a, 0.0, 0.0, rayon, 1.0, app, None);
    }
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
        assert_eq!(p.len(), habillages().len() * 4, "chaque habillage : base, -F, -R, -M");
    }

    /// **-R : les planètes passent entre les anneaux, jamais dedans**, et
    /// tout tient dans le cadre.
    #[test]
    fn les_planetes_d_un_annele_passent_entre_les_anneaux() {
        for nb in [2, 3] {
            let (anneaux, planetes) = plan_annele(nb);
            assert_eq!(anneaux.len(), nb);
            for w in anneaux.windows(2) {
                assert!(w[0].1 < w[1].0, "{nb} anneaux : {w:?} se chevauchent");
            }
            for &(a, _) in &planetes {
                assert!(!anneaux.iter().any(|&(i, e)| (i..=e).contains(&a)), "planète à {a} UA dans un anneau");
                assert!(a <= ANNELE_EXTERNE, "planète à {a} UA hors cadre");
            }
        }
    }
}
