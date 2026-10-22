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

use crate::astre::Foyer;
use crate::disque::habillage::{self, Habillage};
use crate::etoile::UA;
use crate::genese::{preset_gazeuse, preset_tellurique};
use crate::orbite::Orbite;
use crate::planete::Planete;
use crate::soleil::{Soleil, ACCRETION_PROTO, JETS_PROTO};
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

/// Le type travaillé, tel que le catalogue le nomme.
const NOM: &str = "T Tauri (jeune)";

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 12, la T Tauri (2026-10-23) : son disque protosolaire (3 à 16 rayons
/// stellaires, un voile seul) rhabillé avec les habillages du lot 12.2, puis
/// accompagné du grand disque protoplanétaire. T0 est celle du jeu, pour
/// comparer.
pub fn prototypes() -> Vec<Prototype> {
    // Le petit disque se cadre à 2,2 fois son rayon externe (16 rayons).
    let petit = gabarit().2 * 16.0 * 2.2;
    let p = |nom: &str, construire: Box<dyn Fn() -> Systeme>, distance: f32| Prototype {
        nom: format!("T Tauri {nom}"),
        construire,
        distance,
    };
    vec![
        p("T0 : celle du jeu (reference)", Box::new(|| systeme(actuelle(), &[])), petit),
        p(
            "T1 : petit disque, couches + nuages cotonneux",
            Box::new(|| systeme(habillee(|c| habillage::cotonneux(c.avec_couches(7, 0.08))), &[])),
            petit,
        ),
        p("T2 : petit disque en ouragan", Box::new(|| systeme(habillee(habillage::ouragan), &[])), petit),
        // PDS 70 : le grand disque autour du petit, une planète dans son
        // sillon.
        p("T3 : + grand disque cotonneux et sa planete (PDS 70)", Box::new(pds_70), 8.0 * UA * 2.2),
        // HL Tau : l'image la plus connue d'un disque jeune — des anneaux
        // concentriques séparés de sillons.
        p("T4 : + trois anneaux et leurs planetes (HL Tau)", Box::new(hl_tau), 12.5 * UA * 2.2),
    ]
}

/// `(tmin, tmax, rayon, luminosité, nom, mode)` de la T Tauri, pris au
/// catalogue plutôt que recopié.
fn gabarit() -> crate::etoile::Gabarit {
    crate::etoile::gabarit_du_type(NOM, 0.5).expect("la T Tauri est au catalogue")
}

/// L'étoile nue.
fn etoile() -> Soleil {
    let (tmin, tmax, rayon, lumi, _, _) = gabarit();
    Soleil::new(Vec3::ZERO, rayon, crate::etoile::couleur_etoile(NOM, (tmin + tmax) * 0.5), lumi)
}

/// **La T Tauri du jeu**, par la même source que lui.
fn actuelle() -> Soleil {
    Variante::Proto.appliquer(etoile(), NOM)
}

/// La T Tauri, son petit disque rhabillé.
fn habillee(habiller: Habillage) -> Soleil {
    let s = etoile().avec_jets_3d(JETS_PROTO).avec_accretion_polaire(ACCRETION_PROTO);
    let c = habiller(s.config_protosolaire());
    s.avec_disque(c)
}

/// **T3** : le grand disque cotonneux (0,3–8 UA) autour du petit, un sillon
/// à mi-rayon, et la proto-planète qui le creuse.
fn pds_70() -> Systeme {
    const T: f32 = 0.5;
    let grand = habillage::cotonneux(habillage::anneau(0.3, 8.0).avec_lacune(T, 0.07, 0.95));
    let a = grand.interne + T * (grand.externe - grand.interne);
    systeme(actuelle().avec_disque(grand), &[(a / UA, false)])
}

/// **T4** : trois anneaux en couches (le voile lisse des anneaux de
/// poussière), des planètes entre eux et une géante au-delà.
fn hl_tau() -> Systeme {
    let (anneaux, planetes) = habillage::plan_annele(3);
    let s = anneaux.iter().fold(actuelle(), |s, &(i, e)| s.avec_disque(habillage::anneau(i, e).avec_couches(7, 0.08)));
    systeme(s, &planetes)
}

/// **Le plan d'orbite d'une planète dans un disque de normale `n`** : deux
/// axes du plan, dans l'ordre où le disque tourne (`a1 × q = n`).
///
/// ⚠️ `ajouter_planete_autour` pose ses planètes dans le plan horizontal, et
/// tournant dans le sens **opposé** à un disque de normale Y : ses orbites
/// vont de `a1` vers `q` avec `a1 × q = −Y`. Le disque d'une étoile, lui, est
/// incliné selon sa graine.
fn plan_orbital(n: Vec3) -> (Vec3, Vec3) {
    let n = n.normalize_or(Vec3::Y);
    let a1 = n.any_orthonormal_vector();
    (a1, n.cross(a1))
}

/// **L'étoile et ses planètes**, posées sur rails dans le plan de ses
/// disques et dans leur sens.
fn systeme(s: Soleil, planetes: &[habillage::Planete]) -> Systeme {
    let (a1, q) = plan_orbital(s.axe_jets());
    let mu = crate::systeme::G * habillage::masse_t_tauri();
    let mut sys = Systeme::new();
    sys.ajouter(Box::new(s));
    for (k, &(a_ua, geante)) in planetes.iter().enumerate() {
        // Grossies comme dans la galerie des disques : à l'échelle, une
        // planète ne se lit pas à côté d'un disque de 8 UA.
        let (app, taille) = if geante { (preset_gazeuse("Jupiter"), 4.0) } else { (preset_tellurique("Lave"), 2.4) };
        let orb = Orbite::new(a_ua * UA, 0.0, a1, q, mu, k as f32 * 2.4);
        let (pos, vel) = orb.etat(0.0);
        let corps = Planete::new(pos, vel, taille, 1.0, app, orb.polyligne(Orbite::N_TRACE))
            .avec_orbite(orb)
            .avec_foyer(Foyer::Etoile(0));
        sys.ajouter(Box::new(corps));
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
    }

    /// **Une planète tourne dans le plan du disque, et dans son sens** : ses
    /// deux axes sont dans le plan, et vont dans l'ordre du disque.
    #[test]
    fn une_planete_tourne_dans_le_sens_du_disque() {
        for n in [Vec3::Y, vec3(0.3, 0.8, -0.5).normalize(), -Vec3::X] {
            let (a1, q) = plan_orbital(n);
            assert!(a1.dot(n).abs() < 1e-5 && q.dot(n).abs() < 1e-5, "{n} : orbite hors du plan");
            assert!((a1.cross(q) - n).length() < 1e-4, "{n} : la planète tourne à contre-sens");
        }
    }
}
