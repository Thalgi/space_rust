//! **Le plan d'un système** (G1.2, `conception/genese_v2.md` §5.4, §7.4,
//! §7.11) : des données pures, sérialisables, sans aucun objet graphique.
//!
//! `planifier` (G1.3) le remplit en tirant dans les flux nommés ;
//! `instancier` (G1.4) en construit le `Systeme` **sans aucun tirage**. Une
//! sauvegarde garde le plan entier : un système découvert ne change plus.
//!
//! Deux règles tiennent ce fichier :
//! - **une source par fait** : on stocke ce qui est tiré, pas ce qui s'en
//!   déduit (la couleur d'une étoile vient de son type et de sa température,
//!   sa variante de son mode de couronne) ;
//! - **aucun nombre de rendu** : particules, boules, couches, résolution se
//!   décident à la construction, selon le réglage de qualité
//!   (`aucun_nombre_de_rendu_dans_le_plan`).
//!
//! **Le plan porte la vérité physique** (G3.2, §7.1) : distances et orbites de lunes
//! en **UA**, étoiles en unités solaires, planètes et lunes en unités terrestres. Ce
//! que l'écran dessine, grossi ou compressé, se décide à la construction.

use serde::{Deserialize, Serialize};

/// **L'identifiant d'un corps** : sa place dans [`PlanSysteme::corps`].
///
/// Stable par construction : un plan sauvegardé ne se réordonne jamais, et un
/// corps ajouté prend la place suivante. C'est lui que porteront les stations
/// et les points de Lagrange occupés (§7.11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdCorps(pub u32);

/// Le plan entier d'un système.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanSysteme {
    /// La graine dont tout a été tiré.
    pub graine: u64,
    /// Les étoiles, en arbre hiérarchique (une seule feuille pour un système
    /// simple). Les feuilles se numérotent dans l'ordre de déploiement, A
    /// d'abord : c'est ce rang que désigne [`FoyerPlan::Etoile`].
    pub etoiles: NoeudEtoiles,
    /// Planètes, lunes et comètes. L'index est l'[`IdCorps`].
    pub corps: Vec<CorpsPlan>,
    /// Ceintures et grand disque.
    pub disques: Vec<DisquePlan>,
}

/// Un nœud de l'arbre stellaire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NoeudEtoiles {
    Etoile(EtoilePlan),
    /// Deux sous-arbres en orbite autour de leur barycentre.
    Paire {
        a: Box<NoeudEtoiles>,
        b: Box<NoeudEtoiles>,
        /// Demi-grand axe relatif a ↔ b, en UA.
        sep: f32,
        e: f32,
        /// Inclinaison du plan orbital (rad).
        incl: f32,
        /// Anomalie moyenne de départ (rad).
        phase: f32,
        /// Direction du périastre dans le plan (rad) — tirée au déploiement
        /// par la V1, donc à porter ici.
        azimut: f32,
    },
}

/// Une étoile : ce que le tirage a fixé.
///
/// Le **type** est le nom du catalogue d'`etoile.rs` (« Pulsar », « Naine
/// brune »…) ; s'en déduisent la couleur (avec la température), la variante (avec
/// `couronne`), le rémanent, et le cas de la naine brune, qui se construit en géante
/// gazeuse au centre du système.
///
/// `rayon`, `luminosite` et `masse` sont les **vraies valeurs**, en unités solaires
/// (`physique`) : figées ici, un système découvert ne change pas si la table bouge.
/// Celles du rendu se relisent dans le gabarit du type, à la place `q`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EtoilePlan {
    pub type_nom: String,
    pub temperature: f32,
    /// Sa place dans les plages de son type, de 0 à 1.
    pub q: f32,
    pub rayon: f32,
    pub luminosite: f32,
    /// Mode de couronne (0 halo, 1 jets, 2 vent, 3 pulsar, 4 magnétar…).
    pub couronne: f32,
    pub masse: f32,
}

/// Une planète, une lune ou une comète.
///
/// `rayon` et `masse` en unités terrestres. **Sauf le rayon d'une comète**, en unités
/// du monde : un noyau de 10 km est invisible à toute échelle, et on vient voir sa
/// queue. C'est une exagération assumée, la même que dans le système solaire.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CorpsPlan {
    pub nature: Nature,
    pub orbite: OrbitePlan,
    pub rayon: f32,
    pub masse: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Nature {
    Planete(RecetteApparence),
    Lune(RecetteApparence),
    /// Noyau d'apparence fixe ; sa graine se dérive de l'orbite.
    Comete,
}

/// **De quoi refaire une apparence** : `Apparence` n'est pas sérialisable, et
/// la stocker figerait 95 tirages là où trois valeurs suffisent.
///
/// `temperature` est l'**entrée de la recette**, figée au tirage. La
/// température d'équilibre que lit le jeu, elle, se recalcule (§7.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecetteApparence {
    pub type_corps: TypeCorps,
    pub temperature: f32,
    pub graine: u64,
}

/// Miroir sérialisable de `planete::TypePlanete`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeCorps {
    Tellurique,
    Gazeuse,
    Glacee,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OrbitePlan {
    /// Sur rails autour d'un foyer (planètes, comètes) : Kepler, départ au
    /// périastre. `masse_centrale` en masses solaires.
    Rails {
        foyer: FoyerPlan,
        masse_centrale: f32,
        a_ua: f32,
        e: f32,
        incl: f32,
        /// Direction du périastre (rad).
        azimut: f32,
    },
    /// Cercle autour d'une planète.
    Lune {
        parent: IdCorps,
        /// Rayon de l'orbite, en UA.
        rayon_orbite: f32,
        /// Vitesse angulaire, signée (négative : rétrograde).
        omega: f32,
        incl: f32,
        phase: f32,
    },
}

/// Miroir sérialisable de `astre::Foyer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoyerPlan {
    Barycentre,
    /// Rang de la feuille dans l'ordre de déploiement de l'arbre.
    Etoile(usize),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisquePlan {
    pub genre: GenreDisque,
    /// Vaut 0 pour le grand disque : son bord interne suit le petit disque de
    /// l'étoile, dont la taille est celle de l'affichage (`instancier`).
    pub interne_ua: f32,
    pub externe_ua: f32,
    /// En masses solaires.
    pub masse_centrale: f32,
    /// Amorce les tirages internes du disque à sa construction.
    pub graine: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenreDisque {
    Ceinture,
    Kuiper,
    /// Le grand disque d'une jeune étoile ; ses sillons suivent les orbites
    /// des planètes, lues dans le plan à la construction.
    GrandDisque,
}

impl PlanSysteme {
    pub fn new(graine: u64, etoiles: NoeudEtoiles) -> Self {
        Self { graine, etoiles, corps: Vec::new(), disques: Vec::new() }
    }

    /// Ajoute un corps à la place suivante et rend son identifiant.
    pub fn ajouter(&mut self, corps: CorpsPlan) -> IdCorps {
        self.corps.push(corps);
        IdCorps(self.corps.len() as u32 - 1)
    }

    #[allow(dead_code)] // Lu par les tests ; le jeu le lira avec la sauvegarde de partie et G9.
    pub fn corps(&self, id: IdCorps) -> Option<&CorpsPlan> {
        self.corps.get(id.0 as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn etoile(type_nom: &str, masse: f32) -> NoeudEtoiles {
        NoeudEtoiles::Etoile(EtoilePlan {
            type_nom: type_nom.to_string(),
            temperature: 5200.0,
            q: 0.5,
            rayon: 1.3,
            luminosite: 0.8,
            couronne: 0.0,
            masse,
        })
    }

    fn planete(foyer: FoyerPlan, a_ua: f32, type_corps: TypeCorps) -> CorpsPlan {
        CorpsPlan {
            nature: Nature::Planete(RecetteApparence { type_corps, temperature: 280.0, graine: 17 }),
            orbite: OrbitePlan::Rails { foyer, masse_centrale: 900.0, a_ua, e: 0.05, incl: 0.02, azimut: 1.1 },
            rayon: 0.4,
            masse: 2.0,
        }
    }

    /// Un plan qui touche chaque forme : binaire, planète autour d'une étoile
    /// et autour du barycentre, lune, comète, les trois disques.
    fn plan_representatif() -> PlanSysteme {
        let paire = NoeudEtoiles::Paire {
            a: Box::new(etoile("Naine jaune", 1000.0)),
            b: Box::new(etoile("Naine rouge", 300.0)),
            sep: 12.5,
            e: 0.3,
            incl: -0.1,
            phase: 2.0,
            azimut: 0.7,
        };
        let mut plan = PlanSysteme::new(123, paire);
        let geante = plan.ajouter(planete(FoyerPlan::Etoile(0), 3.2, TypeCorps::Gazeuse));
        plan.ajouter(CorpsPlan {
            nature: Nature::Lune(RecetteApparence { type_corps: TypeCorps::Glacee, temperature: 120.0, graine: 9 }),
            orbite: OrbitePlan::Lune { parent: geante, rayon_orbite: 0.03, omega: -0.4, incl: 0.05, phase: 3.0 },
            rayon: 0.05,
            masse: 0.05,
        });
        plan.ajouter(planete(FoyerPlan::Barycentre, 40.0, TypeCorps::Glacee));
        plan.ajouter(CorpsPlan {
            nature: Nature::Comete,
            orbite: OrbitePlan::Rails { foyer: FoyerPlan::Barycentre, masse_centrale: 1300.0, a_ua: 60.0, e: 0.9, incl: 0.8, azimut: 4.0 },
            rayon: 0.08,
            masse: 1e-6,
        });
        for (genre, i, x) in [(GenreDisque::Ceinture, 2.0, 2.9), (GenreDisque::GrandDisque, 0.1, 1.8), (GenreDisque::Kuiper, 30.0, 45.0)] {
            plan.disques.push(DisquePlan { genre, interne_ua: i, externe_ua: x, masse_centrale: 1300.0, graine: 5 });
        }
        plan
    }

    /// Une sauvegarde rend exactement le plan : aucun champ perdu ni déformé.
    #[test]
    fn le_plan_revient_intact_de_json() {
        let plan = plan_representatif();
        let json = serde_json::to_string(&plan).unwrap();
        let relu: PlanSysteme = serde_json::from_str(&json).unwrap();
        assert_eq!(relu, plan);
    }

    /// Un corps ajouté prend la place suivante, sans déplacer les autres : les
    /// identifiants déjà donnés désignent toujours le même corps.
    #[test]
    fn un_identifiant_designe_toujours_le_meme_corps() {
        let mut plan = plan_representatif();
        let avant: Vec<CorpsPlan> = plan.corps.clone();
        let comete = plan.ajouter(plan.corps[3].clone());
        assert_eq!(comete, IdCorps(avant.len() as u32));
        for (i, c) in avant.iter().enumerate() {
            assert_eq!(plan.corps(IdCorps(i as u32)), Some(c), "le corps {i} a changé de place");
        }
        let OrbitePlan::Lune { parent, .. } = plan.corps[1].orbite else { panic!("le corps 1 est la lune") };
        assert!(matches!(plan.corps(parent).map(|c| &c.nature), Some(Nature::Planete(_))));
    }

    /// **Aucun nombre de rendu dans le plan** (§5.4) : la qualité décide à la
    /// construction combien on dessine, sans changer le système. La garde lit
    /// les noms de champs du JSON — ceux d'un compte de rendu n'y ont pas
    /// leur place.
    #[test]
    fn aucun_nombre_de_rendu_dans_le_plan() {
        let json = serde_json::to_value(plan_representatif()).unwrap();
        let mut cles = Vec::new();
        fn parcourir(v: &serde_json::Value, cles: &mut Vec<String>) {
            match v {
                serde_json::Value::Object(m) => m.iter().for_each(|(k, v)| {
                    cles.push(k.to_lowercase());
                    parcourir(v, cles);
                }),
                serde_json::Value::Array(a) => a.iter().for_each(|v| parcourir(v, cles)),
                _ => {}
            }
        }
        parcourir(&json, &mut cles);
        const INTERDITS: [&str; 7] = ["nb", "particule", "boule", "couche", "resolution", "trace", "lod"];
        for k in &cles {
            assert!(!INTERDITS.iter().any(|i| k.contains(i)), "champ de rendu dans le plan : {k}");
        }
        assert!(cles.len() > 20, "le parcours n'a vu que {} clés", cles.len());
    }
}
