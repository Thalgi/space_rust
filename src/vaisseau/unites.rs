//! Standard d'unités du générateur de stations (voir
//! `docs/stations_fondations.md`, §2).
//!
//! Principe : **une seule unité de base `U`**, et toute dimension s'exprime en
//! `n * U`. Les diamètres ne sont pas libres mais tirés d'une famille discrète
//! de **profils** (façon « form factors » de Kerbal Space Program) : deux ports
//! ne s'accouplent que s'ils partagent le même profil, ce qui garantit des
//! jonctions propres et des proportions homogènes.

/// Unité de base : **rayon** du module « standard » (profil `P1`) = 1 U.
/// Changer cette constante rescale toute station d'un coup.
pub const U: f32 = 1.0;

/// Diamètres discrets autorisés. Le rayon est un multiple simple de `U`.
///
/// - `P0` : sondes, cubesats, appendices.
/// - `P1` : module habitat standard (référence).
/// - `P2` : gros module / nœud d'amarrage.
/// - `P3` : cœur / épine dorsale.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profil {
    P0,
    P1,
    P2,
    P3,
}

impl Profil {
    /// Tous les profils, du plus petit au plus grand.
    pub const TOUS: [Profil; 4] = [Profil::P0, Profil::P1, Profil::P2, Profil::P3];

    /// Rayon nominal du profil, en unités monde (`U`).
    pub fn rayon(self) -> f32 {
        let k = match self {
            Profil::P0 => 0.5,
            Profil::P1 => 1.0,
            Profil::P2 => 2.0,
            Profil::P3 => 3.0,
        };
        k * U
    }

    /// Diamètre nominal (= `2 * rayon`).
    pub fn diametre(self) -> f32 {
        2.0 * self.rayon()
    }

    /// Deux profils ne s'accouplent que s'ils sont identiques.
    /// (Compatibilité triviale : évite un module de 4 m sur un port de 0,5 m.)
    pub fn compatible(self, autre: Profil) -> bool {
        self == autre
    }

    /// Nom court, pour l'atelier / le débogage.
    pub fn nom(self) -> &'static str {
        match self {
            Profil::P0 => "P0",
            Profil::P1 => "P1",
            Profil::P2 => "P2",
            Profil::P3 => "P3",
        }
    }
}

/// Proportions dérivées du diamètre — pour ne jamais fixer de taille absolue
/// arbitraire (§2.3). `facteur` est borné à une plage réaliste. API prévue pour
/// le générateur (encore non câblée hors tests).
#[allow(dead_code)]
pub mod proportion {
    /// Longueur d'un module à partir de son diamètre.
    /// Réaliste entre ~1,5× et ~4× le diamètre.
    pub fn longueur_module(diametre: f32, facteur: f32) -> f32 {
        diametre * facteur.clamp(1.5, 4.0)
    }

    /// Demi-section d'un treillis à partir du diamètre du module porteur.
    /// Réaliste entre ~0,5× et ~1× le diamètre.
    pub fn demi_section_treillis(diametre: f32, facteur: f32) -> f32 {
        diametre * facteur.clamp(0.5, 1.0)
    }

    /// Longueur d'un panneau solaire à partir de sa largeur.
    pub fn longueur_panneau(largeur: f32, facteur: f32) -> f32 {
        largeur * facteur.clamp(2.0, 8.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rayons_proportionnels() {
        assert_eq!(Profil::P1.rayon(), U);
        assert_eq!(Profil::P2.rayon(), 2.0 * U);
        assert_eq!(Profil::P0.diametre(), Profil::P1.rayon()); // 0.5U*2 == 1U
    }

    // Cas limite : les rayons sont strictement croissants dans l'ordre de TOUS.
    #[test]
    fn rayons_strictement_croissants() {
        let r: Vec<f32> = Profil::TOUS.iter().map(|p| p.rayon()).collect();
        assert!(r.windows(2).all(|w| w[0] < w[1]), "rayons non croissants: {r:?}");
    }

    // Cas limite : invariant diametre == 2*rayon pour chaque profil.
    #[test]
    fn diametre_double_du_rayon() {
        for p in Profil::TOUS {
            assert_eq!(p.diametre(), 2.0 * p.rayon(), "profil {}", p.nom());
        }
    }

    #[test]
    fn compatibilite_stricte() {
        assert!(Profil::P1.compatible(Profil::P1));
        assert!(!Profil::P1.compatible(Profil::P2));
    }

    // Cas limite : compatibilité réflexive et symétrique, incompatibles sinon.
    #[test]
    fn compatibilite_reflexive_et_symetrique() {
        for a in Profil::TOUS {
            assert!(a.compatible(a), "réflexivité {}", a.nom());
            for b in Profil::TOUS {
                assert_eq!(a.compatible(b), b.compatible(a), "symétrie {}/{}", a.nom(), b.nom());
                assert_eq!(a.compatible(b), a == b);
            }
        }
    }

    // Cas limite : noms tous distincts (pour l'atelier / le débogage).
    #[test]
    fn noms_distincts() {
        let noms: Vec<&str> = Profil::TOUS.iter().map(|p| p.nom()).collect();
        for i in 0..noms.len() {
            for j in (i + 1)..noms.len() {
                assert_ne!(noms[i], noms[j]);
            }
        }
    }

    // Cas limite : proportions clampées aux bornes ET passantes dans la plage.
    #[test]
    fn longueur_module_bornes_et_plage() {
        assert_eq!(proportion::longueur_module(2.0, 2.5), 2.0 * 2.5); // dans la plage
        assert_eq!(proportion::longueur_module(2.0, 1.0), 2.0 * 1.5); // sous la borne
        assert_eq!(proportion::longueur_module(2.0, 100.0), 2.0 * 4.0); // au-dessus
        assert_eq!(proportion::longueur_module(2.0, -3.0), 2.0 * 1.5); // négatif → borne basse
    }

    #[test]
    fn demi_section_treillis_bornes_et_plage() {
        assert_eq!(proportion::demi_section_treillis(2.0, 0.75), 2.0 * 0.75);
        assert_eq!(proportion::demi_section_treillis(2.0, 0.0), 2.0 * 0.5);
        assert_eq!(proportion::demi_section_treillis(2.0, 5.0), 2.0 * 1.0);
    }

    #[test]
    fn longueur_panneau_bornes_et_plage() {
        assert_eq!(proportion::longueur_panneau(1.5, 5.0), 1.5 * 5.0);
        assert_eq!(proportion::longueur_panneau(1.5, 1.0), 1.5 * 2.0);
        assert_eq!(proportion::longueur_panneau(1.5, 100.0), 1.5 * 8.0);
    }
}
