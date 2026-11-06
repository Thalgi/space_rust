//! **L'échelle d'affichage** (G3.3) : comment la vérité du plan se montre à l'écran,
//! réglée sans recompiler dans `assets/echelle.json`, relu à chaque système construit
//! (`conception/genese_v2.md` §7.1, §7.14). Ni le plan ni les calculs n'en dépendent.

use serde::Deserialize;

pub const FICHIER: &str = "assets/echelle.json";

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Echelle {
    /// Facteur des rayons de planètes et de lunes, et des orbites de lunes.
    pub grossissement_corps: f32,
    /// Exposant du rayon vrai des étoiles (1 = proportionnel au Soleil affiché).
    pub compression_etoiles: f32,
    /// Exposant des distances autour de 1 UA (1 = distances vraies).
    pub compression_distances: f32,
}

impl Default for Echelle {
    fn default() -> Self {
        Self { grossissement_corps: 5.0, compression_etoiles: 0.6, compression_distances: 1.0 }
    }
}

impl Echelle {
    /// Une vraie distance, en position à l'écran.
    pub fn monde(&self, ua: f32) -> f32 {
        crate::etoile::monde_de_ua(ua, self.compression_distances)
    }
}

/// **Le fichier**, ou les valeurs par défaut s'il manque. Un fichier illisible (faute de
/// frappe pendant un réglage) le dit et retombe aussi sur le défaut.
pub fn lire() -> Echelle {
    let Ok(texte) = std::fs::read_to_string(FICHIER) else { return Echelle::default() };
    serde_json::from_str(&texte).unwrap_or_else(|e| {
        eprintln!("{FICHIER} illisible ({e}) : valeurs par defaut.");
        Echelle::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le fichier livré se lit, et vaut le défaut** : une faute de frappe y retomberait
    /// en silence sur le défaut pour qui ne regarde pas la console.
    #[test]
    fn le_fichier_livre_vaut_le_defaut() {
        let texte = std::fs::read_to_string(FICHIER).expect(FICHIER);
        assert_eq!(serde_json::from_str::<Echelle>(&texte).expect("JSON valide"), Echelle::default());
    }

    #[test]
    fn un_champ_inconnu_est_refuse() {
        assert!(serde_json::from_str::<Echelle>(r#"{"grossisement_corps": 3.0}"#).is_err());
    }

    /// Les distances et leur retour : 1 UA ne bouge jamais, l'exposant 1 ne change rien.
    #[test]
    fn la_compression_tourne_autour_d_une_ua() {
        use crate::etoile::{monde_de_ua, ua_de_monde, UA};
        for ua in [0.02_f32, 0.5, 1.0, 30.0, 220.0] {
            assert_eq!(monde_de_ua(ua, 1.0), ua * UA);
            assert_eq!(ua_de_monde(ua * UA, 1.0), ua);
            let aller_retour = ua_de_monde(monde_de_ua(ua, 0.6), 0.6);
            assert!((aller_retour - ua).abs() < ua * 1e-4, "{ua} : {aller_retour}");
        }
        assert!((monde_de_ua(1.0, 0.5) - UA).abs() < 1e-4);
        assert!(monde_de_ua(100.0, 0.5) < 100.0 * UA && monde_de_ua(0.01, 0.5) > 0.01 * UA);
    }
}
