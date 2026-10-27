//! **Le type d'une planète, et ce qu'on peut y faire** (chantier planètes,
//! 2026-10-27) : huit types, chacun son icône (`assets/sprites/planet_type_*`).
//!
//! Le type se lit sur **l'apparence** de la planète, et seulement sur elle :
//! c'est la genèse qui fabrique cette apparence d'après la température
//! d'équilibre (`genese::apparence_tellurique` — lave au-dessus de 600 K,
//! étuve, désert, tempérée, toundra, gelée), donc l'apparence la porte déjà.
//! Une seule règle vaut ainsi partout : dans un système généré, dans la galerie
//! des planètes (où il n'y a ni étoile ni distance), pour une lune.
//!
//! **Les cinq niveaux d'une rocheuse sont une échelle** — ce qu'il en coûte à
//! une colonie humaine de s'y installer durablement : habitable (sans
//! difficulté) → colonisable (plus de matériaux) → dangereuse (une recherche)
//! → hostile (une recherche de plus) → inhabitable (personne en surface).
//!
//! **Le type de base d'un preset du catalogue se règle à la main** dans
//! [`FICHIER`] (vue tableur, ou édition directe : relu à chaud). Un preset
//! absent du fichier, et toute apparence fabriquée par la genèse, retombent sur
//! la règle ([`selon_apparence`]). Le même fichier donne à un preset son nom
//! affiché et peut le retirer : son nom de code, lui, ne change jamais.

use super::apparence::{Apparence, TypePlanete};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;

/// **Le type d'une planète**, du pire au meilleur pour une rocheuse, puis les
/// gazeuses.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassePlanete {
    /// Rien n'y survivra : un monde de lave.
    Inhabitable,
    /// Une étuve sous un voile écrasant, comme Vénus.
    Hostile,
    /// Glacée, ou gelée sous la banquise : la vie y tient, mais à grand-peine.
    Dangereuse,
    /// On s'y installe sous dôme : désert, toundra, roche nue.
    Colonisable,
    /// Océans et végétation : on y vit à l'air libre.
    Habitable,
    GeanteGazeuse,
    GeanteAnnelee,
    NaineBrune,
}

impl ClassePlanete {
    /// **L'échelle des rocheuses**, du plus simple au plus dur à coloniser :
    /// les colonnes de la vue tableur.
    pub const ECHELLE: [ClassePlanete; 5] = [
        ClassePlanete::Habitable,
        ClassePlanete::Colonisable,
        ClassePlanete::Dangereuse,
        ClassePlanete::Hostile,
        ClassePlanete::Inhabitable,
    ];

    pub const TOUTES: [ClassePlanete; 8] = [
        ClassePlanete::Inhabitable,
        ClassePlanete::Hostile,
        ClassePlanete::Dangereuse,
        ClassePlanete::Colonisable,
        ClassePlanete::Habitable,
        ClassePlanete::GeanteGazeuse,
        ClassePlanete::GeanteAnnelee,
        ClassePlanete::NaineBrune,
    ];

    /// Le nom affiché (police Minitel : sans accents).
    pub fn libelle(self) -> &'static str {
        match self {
            ClassePlanete::Inhabitable => "INHABITABLE",
            ClassePlanete::Hostile => "HOSTILE",
            ClassePlanete::Dangereuse => "DANGEREUSE",
            ClassePlanete::Colonisable => "COLONISABLE",
            ClassePlanete::Habitable => "HABITABLE",
            ClassePlanete::GeanteGazeuse => "GEANTE GAZEUSE",
            ClassePlanete::GeanteAnnelee => "GEANTE GAZEUSE ANNELEE",
            ClassePlanete::NaineBrune => "NAINE BRUNE",
        }
    }

    /// Le fichier de son icône, dans `assets/sprites/`, sans extension.
    pub fn fichier(self) -> &'static str {
        match self {
            ClassePlanete::Inhabitable => "planet_type_inhabitable",
            ClassePlanete::Hostile => "planet_type_hostile",
            ClassePlanete::Dangereuse => "planet_type_hazardous",
            ClassePlanete::Colonisable => "planet_type_colonizable",
            ClassePlanete::Habitable => "planet_type_habitable",
            ClassePlanete::GeanteGazeuse => "planet_type_gas_giant",
            ClassePlanete::GeanteAnnelee => "planet_type_gas_ringed",
            ClassePlanete::NaineBrune => "planet_type_brown_dwarf",
        }
    }

    /// La teinte de son libellé.
    pub fn couleur(self) -> Color {
        match self {
            ClassePlanete::Inhabitable => Color::new(1.0, 0.35, 0.3, 1.0),
            ClassePlanete::Hostile => Color::new(1.0, 0.6, 0.3, 1.0),
            ClassePlanete::Dangereuse => Color::new(1.0, 0.85, 0.35, 1.0),
            ClassePlanete::Colonisable => Color::new(0.55, 0.85, 1.0, 1.0),
            ClassePlanete::Habitable => Color::new(0.4, 1.0, 0.5, 1.0),
            ClassePlanete::GeanteGazeuse | ClassePlanete::GeanteAnnelee => Color::new(0.85, 0.75, 0.6, 1.0),
            ClassePlanete::NaineBrune => Color::new(0.8, 0.5, 0.4, 1.0),
        }
    }
}

/// **Le fichier des presets**, éditable à la main.
pub const FICHIER: &str = "assets/planetes.json";

/// **Ce que le fichier règle pour un preset.** Tout est facultatif : un champ
/// absent laisse la valeur par défaut (la règle, le nom de code, présent).
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Reglage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub habitabilite: Option<ClassePlanete>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nom: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub retiree: bool,
}

/// Les réglages, par nom de code du preset. Trié : le fichier écrit se lit et
/// se compare (git) sans bruit.
pub type Table = BTreeMap<String, Reglage>;

/// **Un fichier JSON relu à chaud** : sa valeur en mémoire, la date du
/// fichier lu, et le dernier coup d'œil. Sert aussi aux règles de contexte.
pub(super) struct Relu<T> {
    pub valeur: T,
    date: Option<std::time::SystemTime>,
    vu: f64,
}

fn date_fichier(chemin: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(chemin).and_then(|m| m.modified()).ok()
}

/// **La valeur de `chemin`, relue à chaud** : la date du fichier est regardée
/// au plus une fois par seconde, et il n'est relu que s'il a changé. Un fichier
/// illisible (JSON cassé pendant une édition) garde la valeur précédente ; un
/// fichier absent donne la valeur par défaut.
pub(super) fn relu<'a, T: serde::de::DeserializeOwned + Default>(
    chemin: &str,
    c: &'a mut Option<Relu<T>>,
) -> &'a mut Relu<T> {
    let maintenant = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64());
    let r = c.get_or_insert_with(|| Relu { valeur: T::default(), date: None, vu: f64::NEG_INFINITY });
    if maintenant - r.vu >= 1.0 {
        r.vu = maintenant;
        let date = date_fichier(chemin);
        if date != r.date {
            r.date = date;
            match std::fs::read_to_string(chemin).map(|t| serde_json::from_str::<T>(&t)) {
                Ok(Ok(v)) => r.valeur = v,
                Ok(Err(e)) => warn!("{chemin} illisible ({e}) : on garde la version précédente."),
                Err(_) => r.valeur = T::default(),
            }
        }
    }
    r
}

thread_local! {
    static CACHE: RefCell<Option<Relu<Table>>> = const { RefCell::new(None) };
}

fn avec_table<R>(f: impl FnOnce(&Table) -> R) -> R {
    CACHE.with(|c| f(&relu(FICHIER, &mut c.borrow_mut()).valeur))
}

/// **Une copie de la table** (pour la modifier, puis [`ecrire`]).
pub fn table() -> Table {
    avec_table(Table::clone)
}

/// **Écrit la table** dans le fichier et la garde en mémoire telle quelle.
pub fn ecrire(table: &Table) -> std::io::Result<()> {
    let texte = serde_json::to_string_pretty(table).map_err(std::io::Error::other)?;
    std::fs::write(FICHIER, texte + "\n")?;
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let r = relu::<Table>(FICHIER, &mut c);
        r.valeur = table.clone();
        r.date = date_fichier(FICHIER);
    });
    Ok(())
}

/// Le réglage du preset `id`, s'il en a un.
pub fn reglage(id: &str) -> Option<Reglage> {
    avec_table(|t| t.get(id).cloned())
}

/// **Le nom affiché du preset `id`** : celui du fichier, sinon son nom de code.
pub fn nom_affiche(id: &str) -> String {
    reglage(id).and_then(|r| r.nom).unwrap_or_else(|| id.to_string())
}

/// **Le type d'une planète.**
///
/// Gazeuses : toujours la règle (naine brune, puis annelée, puis géante).
/// Rocheuses et glacées : le niveau réglé pour son preset s'il y en a un, la
/// règle sinon.
pub fn classer(app: &Apparence) -> ClassePlanete {
    if app.type_p != TypePlanete::Gazeuse {
        if let Some(c) = app.preset.and_then(reglage).and_then(|r| r.habitabilite) {
            return c;
        }
    }
    selon_apparence(app)
}

/// **Le type d'une planète, d'après son apparence seule.**
///
/// Gazeuses : naine brune, puis annelée, puis géante. Rocheuses et glacées,
/// dans cet ordre : lave → inhabitable ; voile épais (étuve) → hostile ;
/// glacée ou banquise quasi globale → dangereuse ; océans et végétation →
/// habitable ; tout le reste → colonisable.
pub fn selon_apparence(app: &Apparence) -> ClassePlanete {
    if app.type_p == TypePlanete::Gazeuse {
        return if app.naine_brune {
            ClassePlanete::NaineBrune
        } else if app.anneau {
            ClassePlanete::GeanteAnnelee
        } else {
            ClassePlanete::GeanteGazeuse
        };
    }
    if app.lave >= 0.5 {
        ClassePlanete::Inhabitable
    } else if app.voile >= 0.5 {
        ClassePlanete::Hostile
    } else if app.type_p == TypePlanete::Glacee || app.calotte <= 0.3 {
        ClassePlanete::Dangereuse
    } else if app.eau >= 0.4 && app.veg_couv >= 0.4 {
        ClassePlanete::Habitable
    } else {
        ClassePlanete::Colonisable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roche() -> Apparence {
        Apparence::new(TypePlanete::Tellurique, Vec3::ONE, Vec3::ONE, Vec3::ONE, 0.0)
    }

    /// **Chaque climat de la genèse tombe dans son type** : lave, étuve,
    /// glace, monde vivant, et le reste.
    #[test]
    fn chaque_monde_tombe_dans_son_type() {
        assert_eq!(classer(&Apparence { lave: 1.0, ..roche() }), ClassePlanete::Inhabitable);
        assert_eq!(classer(&Apparence { voile: 0.9, ..roche() }), ClassePlanete::Hostile);
        assert_eq!(classer(&Apparence { calotte: 0.15, ..roche() }), ClassePlanete::Dangereuse);
        let glace = Apparence::new(TypePlanete::Glacee, Vec3::ONE, Vec3::ONE, Vec3::ONE, 0.0);
        assert_eq!(classer(&glace), ClassePlanete::Dangereuse);
        assert_eq!(classer(&Apparence { eau: 0.6, veg_couv: 0.7, calotte: 0.82, ..roche() }), ClassePlanete::Habitable);
        // Toundra : un peu d'eau, peu de végétation — on s'y installe sous dôme.
        assert_eq!(classer(&Apparence { eau: 0.2, veg_couv: 0.2, calotte: 0.6, ..roche() }), ClassePlanete::Colonisable);
        assert_eq!(classer(&roche()), ClassePlanete::Colonisable, "roche nue");
    }

    /// **Les gazeuses** : naine brune d'abord, annelée ensuite — une naine
    /// brune annelée reste une naine brune.
    #[test]
    fn les_gazeuses_se_rangent() {
        let g = Apparence::new(TypePlanete::Gazeuse, Vec3::ONE, Vec3::ONE, Vec3::ONE, 0.0);
        assert_eq!(classer(&g), ClassePlanete::GeanteGazeuse);
        assert_eq!(classer(&Apparence { anneau: true, ..g }), ClassePlanete::GeanteAnnelee);
        assert_eq!(classer(&Apparence { naine_brune: true, anneau: true, ..g }), ClassePlanete::NaineBrune);
        // Une gazeuse ne passe jamais par les règles des rocheuses.
        assert_eq!(classer(&Apparence { lave: 1.0, ..g }), ClassePlanete::GeanteGazeuse);
    }

    /// **Le fichier ne règle que des presets qui existent, et seulement à
    /// l'échelle des rocheuses.** Un nom de code mal recopié serait ignoré
    /// sans bruit ; un « geante_gazeuse » sur un monde rocheux n'a pas de sens.
    #[test]
    fn le_fichier_regle_des_presets_qui_existent() {
        let texte = std::fs::read_to_string(FICHIER).expect("assets/planetes.json");
        let t: Table = serde_json::from_str(&texte).expect("JSON valide");
        let noms: Vec<String> = crate::genese::catalogue_telluriques()
            .into_iter()
            .chain(crate::genese::catalogue_gazeuses())
            .map(|(n, _)| n)
            .collect();
        for (id, r) in &t {
            assert!(noms.contains(id), "« {id} » n'est pas un preset du catalogue");
            if let Some(c) = r.habitabilite {
                assert!(ClassePlanete::ECHELLE.contains(&c), "« {id} » : {c:?} hors de l'échelle des rocheuses");
            }
        }
    }

    /// **Un preset réglé prend le niveau du fichier ; une gazeuse, jamais.**
    #[test]
    fn le_reglage_prime_sur_la_regle() {
        let id = "Terre";
        let lave = Apparence { lave: 1.0, preset: Some(id), ..roche() };
        let attendu = reglage(id).and_then(|r| r.habitabilite).unwrap_or(ClassePlanete::Inhabitable);
        assert_eq!(classer(&lave), attendu);
        assert_eq!(classer(&Apparence { preset: None, ..lave }), ClassePlanete::Inhabitable);
        let g = Apparence::new(TypePlanete::Gazeuse, Vec3::ONE, Vec3::ONE, Vec3::ONE, 0.0);
        assert_eq!(classer(&Apparence { preset: Some(id), ..g }), ClassePlanete::GeanteGazeuse);
    }

    /// **Écrit `assets/planetes.json` d'après la règle** — le point de départ
    /// des réglages à la main. Lancé à la demande seulement
    /// (`cargo test generer_planetes_json -- --ignored`) : il écrase le fichier.
    #[test]
    #[ignore]
    fn generer_planetes_json() {
        let t: Table = crate::genese::catalogue_telluriques()
            .into_iter()
            .map(|(n, app)| (n, Reglage { habitabilite: Some(selon_apparence(&app)), ..Default::default() }))
            .collect();
        ecrire(&t).expect("écriture");
    }

    /// **Chaque type pointe une icône qui existe**, et chacun la sienne.
    #[test]
    fn chaque_type_a_son_icone_sur_le_disque() {
        for (i, c) in ClassePlanete::TOUTES.iter().enumerate() {
            let chemin = format!("assets/sprites/{}.png", c.fichier());
            assert!(std::path::Path::new(&chemin).exists(), "{c:?} pointe {chemin}, qui n'existe pas");
            assert!(!ClassePlanete::TOUTES[..i].iter().any(|d| d.fichier() == c.fichier()), "{c:?} en double");
        }
    }
}
