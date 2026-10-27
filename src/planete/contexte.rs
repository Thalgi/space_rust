//! **L'habitabilité en contexte** (chantier planètes, P3, 2026-10-27) : le
//! type de base d'une rocheuse (`classe`), corrigé par les étoiles qui
//! l'éclairent et par sa place dans le système.
//!
//! *« Une planète tellurique type Mars autour d'un pulsar… c'est pas
//! viable »* : les règles vivent dans [`FICHIER`], une entrée par type
//! d'étoile du tirage, relu à chaud comme `assets/planetes.json`. Elles ne font
//! que **dégrader** : le type de base est le meilleur cas, une étoile calme
//! (G, K) n'y change rien. Le fichier sera réglé finement avec la genèse.
//!
//! Pour chaque étoile : `crans` fait descendre d'autant sur l'échelle,
//! `proche` en ajoute en deçà d'une distance, `plafond` borne le meilleur
//! niveau possible. Plusieurs étoiles : la pire l'emporte. La zone (trop
//! chaud, trop froid) descend ensuite. Les gazeuses ne sont pas touchées.

use super::classe::{relu, ClassePlanete, Relu};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// **Le fichier des règles**, éditable à la main.
pub const FICHIER: &str = "assets/habitabilite.json";

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Regles {
    #[serde(default)]
    pub zone: RegleZone,
    /// Par type d'étoile, tel que le tirage le nomme (`etoile::types_du_tirage`).
    #[serde(default)]
    pub astres: BTreeMap<String, RegleAstre>,
}

/// Crans perdus hors de la zone tempérée.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct RegleZone {
    #[serde(default)]
    pub trop_chaud: u8,
    #[serde(default)]
    pub trop_froid: u8,
}

/// Ce qu'un type d'étoile fait aux rocheuses qui l'entourent.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct RegleAstre {
    /// Crans perdus, à toute distance.
    #[serde(default)]
    pub crans: u8,
    /// Le meilleur niveau possible autour de cette étoile.
    #[serde(default)]
    pub plafond: Option<ClassePlanete>,
    /// Crans perdus en plus, en deçà d'une distance.
    #[serde(default)]
    pub proche: Option<Proche>,
    /// Le mot affiché dans la fiche quand la règle a joué (police Minitel :
    /// majuscules sans accents).
    #[serde(default)]
    pub raison: String,
    /// Le pourquoi, pour qui règle le fichier ; jamais affiché.
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proche {
    pub ua: f32,
    pub crans: u8,
}

/// La place de la planète par rapport à la zone tempérée du système.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Zone {
    Temperee,
    TropChaud,
    TropFroid,
    /// Pas d'astre lumineux : la zone ne dit rien.
    SansObjet,
}

/// Descend `c` de `n` crans sur l'échelle, sans passer sous « inhabitable ».
fn descendre(c: ClassePlanete, n: u8) -> ClassePlanete {
    let e = &ClassePlanete::ECHELLE;
    let i = e.iter().position(|x| *x == c).unwrap_or(0);
    e[(i + n as usize).min(e.len() - 1)]
}

/// Le pire des deux niveaux.
fn pire(a: ClassePlanete, b: ClassePlanete) -> ClassePlanete {
    let rang = |c| ClassePlanete::ECHELLE.iter().position(|x| *x == c).unwrap_or(0);
    if rang(b) > rang(a) {
        b
    } else {
        a
    }
}

/// **Le niveau d'une planète de type `base`**, exposée aux `etoiles` (type,
/// distance en UA), à sa place `zone`, selon `regles` — et les raisons de
/// chaque cran perdu.
pub fn juger_selon(
    base: ClassePlanete,
    etoiles: &[(&str, f32)],
    zone: Zone,
    regles: &Regles,
) -> (ClassePlanete, Vec<String>) {
    if !ClassePlanete::ECHELLE.contains(&base) {
        return (base, Vec::new());
    }
    let mut niveau = base;
    let mut raisons = Vec::new();
    for (t, ua) in etoiles {
        let Some(r) = regles.astres.get(*t) else { continue };
        let proche = r.proche.as_ref().filter(|p| *ua < p.ua).map_or(0, |p| p.crans);
        let mut c = descendre(base, r.crans.saturating_add(proche));
        if let Some(p) = r.plafond {
            c = pire(c, p);
        }
        if c != base && !r.raison.is_empty() && !raisons.contains(&r.raison) {
            raisons.push(r.raison.clone());
        }
        niveau = pire(niveau, c);
    }
    let (n, mot) = match zone {
        Zone::TropChaud => (regles.zone.trop_chaud, "TROP CHAUD"),
        Zone::TropFroid => (regles.zone.trop_froid, "TROP FROID"),
        _ => (0, ""),
    };
    let c = descendre(niveau, n);
    if c != niveau {
        raisons.push(mot.to_string());
        niveau = c;
    }
    (niveau, raisons)
}

thread_local! {
    static CACHE: RefCell<Option<Relu<Regles>>> = const { RefCell::new(None) };
}

/// [`juger_selon`] les règles du fichier, relues à chaud.
pub fn juger(base: ClassePlanete, etoiles: &[(&str, f32)], zone: Zone) -> (ClassePlanete, Vec<String>) {
    CACHE.with(|c| juger_selon(base, etoiles, zone, &relu(FICHIER, &mut c.borrow_mut()).valeur))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ClassePlanete::*;

    fn regles() -> Regles {
        serde_json::from_str(&std::fs::read_to_string(FICHIER).expect("assets/habitabilite.json")).expect("JSON valide")
    }

    /// **Chaque type d'étoile du tirage a sa règle, et chaque règle un type
    /// qui existe.** Un type ajouté au tirage sans règle passerait pour une
    /// étoile sans effet ; une clé mal recopiée serait ignorée sans bruit.
    #[test]
    fn chaque_etoile_du_tirage_a_sa_regle() {
        let r = regles();
        let types = crate::etoile::types_du_tirage();
        for t in &types {
            assert!(r.astres.contains_key(*t), "« {t} » n'a pas de règle dans {FICHIER}");
        }
        for (t, a) in &r.astres {
            assert!(types.contains(&t.as_str()), "« {t} » n'est pas un type du tirage");
            if let Some(p) = a.plafond {
                assert!(ClassePlanete::ECHELLE.contains(&p), "« {t} » : plafond {p:?} hors de l'échelle");
            }
        }
    }

    /// **Le cas posé par l'utilisateur** : une Mars (colonisable) autour d'un
    /// pulsar n'est pas viable ; autour d'une G, elle reste ce qu'elle est.
    #[test]
    fn une_mars_autour_d_un_pulsar_n_est_pas_viable() {
        let r = regles();
        assert_eq!(juger_selon(Colonisable, &[("Pulsar", 1.5)], Zone::Temperee, &r).0, Inhabitable);
        assert_eq!(juger_selon(Colonisable, &[("G (jaune)", 1.5)], Zone::Temperee, &r).0, Colonisable);
    }

    /// **Les mécanismes** : crans, proche (seulement en deçà), plafond, la
    /// pire étoile l'emporte, la zone s'ajoute, le plancher tient, et les
    /// gazeuses n'en sont pas touchées.
    #[test]
    fn les_regles_se_combinent() {
        let mut r = Regles { zone: RegleZone { trop_chaud: 1, trop_froid: 2 }, ..Default::default() };
        let regle = |crans, plafond, proche: Option<(f32, u8)>| RegleAstre {
            crans,
            plafond,
            proche: proche.map(|(ua, crans)| Proche { ua, crans }),
            raison: "X".into(),
            note: String::new(),
        };
        r.astres.insert("Un".into(), regle(1, None, Some((2.0, 2))));
        r.astres.insert("Plafond".into(), regle(0, Some(Hostile), None));
        let j = |base, e: &[(&str, f32)], z| juger_selon(base, e, z, &r).0;
        assert_eq!(j(Habitable, &[("Un", 5.0)], Zone::Temperee), Colonisable);
        assert_eq!(j(Habitable, &[("Un", 1.0)], Zone::Temperee), Hostile, "proche : 1 + 2 crans");
        assert_eq!(j(Habitable, &[("Plafond", 5.0)], Zone::Temperee), Hostile);
        assert_eq!(j(Habitable, &[("Plafond", 5.0), ("Un", 5.0)], Zone::Temperee), Hostile, "la pire l'emporte, même en premier");
        assert_eq!(j(Habitable, &[], Zone::TropFroid), Dangereuse);
        assert_eq!(j(Hostile, &[("Un", 1.0)], Zone::TropChaud), Inhabitable, "plancher");
        assert_eq!(j(GeanteGazeuse, &[("Plafond", 1.0)], Zone::TropChaud), GeanteGazeuse);
        assert_eq!(juger_selon(Habitable, &[("Un", 1.0)], Zone::TropChaud, &r).1, vec!["X", "TROP CHAUD"]);
    }
}
