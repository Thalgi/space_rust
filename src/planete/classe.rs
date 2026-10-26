//! **Le type d'une planète, et ce qu'on peut y faire** (chantier planètes,
//! 2026-10-27) : huit types, chacun son icône (`assets/sprites/planet_type_*`).
//!
//! Le type se lit sur **l'apparence** de la planète, et seulement sur elle :
//! c'est la genèse qui fabrique cette apparence d'après la température
//! d'équilibre (`genese::apparence_tellurique` — lave au-dessus de 600 K,
//! étuve, désert, tempérée, toundra, gelée), donc l'apparence la porte déjà.
//! Une seule règle vaut ainsi partout : dans un système généré, dans la galerie
//! des planètes (où il n'y a ni étoile ni distance), pour une lune.

use super::apparence::{Apparence, TypePlanete};
use macroquad::prelude::*;

/// **Le type d'une planète**, du pire au meilleur pour une rocheuse, puis les
/// gazeuses.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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

/// **Le type d'une planète, d'après son apparence.**
///
/// Gazeuses : naine brune, puis annelée, puis géante. Rocheuses et glacées,
/// dans cet ordre : lave → inhabitable ; voile épais (étuve) → hostile ;
/// glacée ou banquise quasi globale → dangereuse ; océans et végétation →
/// habitable ; tout le reste → colonisable.
pub fn classer(app: &Apparence) -> ClassePlanete {
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
