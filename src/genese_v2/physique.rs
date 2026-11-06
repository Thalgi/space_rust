//! **Les vraies valeurs des étoiles** (G3.1, `conception/genese_v2.md` §7.1,
//! §7.13) : rayon et masse en unités solaires, pour chacun des 29 types du
//! tirage. Elles servent aux **calculs** (température, étalement, Kepler) ; ce
//! que l'écran dessine reste la luminosité de rendu d'`etoile.rs`.
//!
//! **La luminosité se déduit** du rayon et de la température, tirée par le
//! gabarit : `L = R² (T / T☉)⁴`, loi de Stefan-Boltzmann. Une seule source par
//! fait : elle ne peut pas contredire le rayon. Seuls les objets dont l'éclat
//! ne vient pas d'une photosphère portent une luminosité explicite.
//!
//! Sources (2026-11-06), détail et réserves dans `docs/suivi/genese_v2.md`, G3.1 :
//! - séquence principale M à O : table de Mamajek (Pecaut & Mamajek 2013, tenue
//!   à jour), <https://www.pas.rochester.edu/~emamajek/EEM_dwarf_UBVIJHK_colors_Teff.txt> ;
//! - le reste : Wikipédia en anglais et étoiles nommées, **ordres de grandeur** ;
//!   `// ~` marque une plage que les sources n'encadrent pas entièrement.

// ponytail: `planifier` s'en sert en G3.2 ; l'allow tombe alors.
#![allow(dead_code)]

/// Température effective du Soleil (K), valeur nominale de l'UAI.
pub const T_SOLEIL: f32 = 5772.0;

/// Plages `(min, max)` d'un type, en unités solaires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plages {
    pub rayon: (f32, f32),
    pub masse: (f32, f32),
    /// `None` : une photosphère, dont la luminosité se déduit.
    pub luminosite: Option<(f32, f32)>,
}

/// Les vraies valeurs d'une étoile tirée, en unités solaires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vraies {
    pub rayon: f32,
    pub masse: f32,
    pub luminosite: f32,
}

/// `L = R² (T / T☉)⁴`, en luminosités solaires.
pub fn stefan_boltzmann(rayon: f32, temperature: f32) -> f32 {
    rayon * rayon * (temperature / T_SOLEIL).powi(4)
}

/// Les plages d'un type du tirage. `None` si le nom n'en est pas un.
pub fn plages(nom: &str) -> Option<Plages> {
    let photosphere = |rayon, masse| Some(Plages { rayon, masse, luminosite: None });
    let compact = |rayon, masse, l| Some(Plages { rayon, masse, luminosite: Some(l) });
    // Le rayon d'une étoile à neutrons, 9 à 12 km sur les 696 340 d'un rayon solaire.
    const R_NEUTRONS: (f32, f32) = (1.29e-5, 1.72e-5);
    match nom {
        // Séquence principale : les bornes de chaque classe dans la table de
        // Mamajek (M8V–M0V, K9V–K0V, G8V–G0V, F9V–F0V, A9V–A0V, B9V–B0V, O9V–O5V).
        "M (naine rouge)" => photosphere((0.114, 0.588), (0.085, 0.57)),
        "K (orange)" => photosphere((0.608, 0.813), (0.59, 0.88)),
        "G (jaune)" => photosphere((0.914, 1.10), (0.94, 1.06)),
        "F (blanc-jaune)" => photosphere((1.167, 1.728), (1.13, 1.61)),
        "A (blanche)" => photosphere((1.747, 2.193), (1.75, 2.18)),
        "B (bleu-blanc)" => photosphere((2.49, 7.16), (2.75, 17.7)),
        "O (bleue)" => photosphere((7.72, 11.45), (20.2, 43.0)),

        // Étoiles évoluées.
        "Geante rouge" => photosphere((15.0, 120.0), (0.3, 8.0)),
        "Supergeante rouge" => photosphere((370.0, 1500.0), (10.0, 26.0)),
        "Cepheide" => photosphere((30.0, 200.0), (4.0, 20.0)),
        "Mira" => photosphere((250.0, 400.0), (0.5, 8.0)),
        "Etoile carbonee (C)" => photosphere((200.0, 400.0), (2.5, 5.4)),
        "Sous-naine (sdB)" => photosphere((0.15, 0.25), (0.45, 0.55)),

        // Massives. Le rayon est **effectif** : un Wolf-Rayet est plus chaud que
        // les 38 000–45 000 K de son gabarit, et son rayon le compense pour
        // retrouver la luminosité publiée.
        "Wolf-Rayet" => photosphere((8.0, 20.0), (10.0, 33.0)), // ~
        "Hypergeante (LBV)" => photosphere((50.0, 100.0), (30.0, 120.0)), // ~
        "Supergeante bleue" => photosphere((15.0, 60.0), (10.0, 40.0)), // ~
        "Etoile Be" => photosphere((4.0, 11.0), (5.0, 17.0)), // ~

        // Jeunes.
        "T Tauri (jeune)" => photosphere((1.5, 3.0), (0.5, 2.0)), // ~
        "Herbig Ae/Be (jeune)" => photosphere((1.5, 4.0), (2.0, 8.0)), // ~

        // Naines blanches : 0,8 à 2 % du rayon solaire, 0,5 à 0,7 masse.
        "Naine blanche" | "Naine blanche polluee" => photosphere((0.008, 0.02), (0.5, 0.7)),
        "Naine blanche pulsante (ZZ Ceti)" => photosphere((0.012, 0.013), (0.55, 0.65)),
        // Fictive : l'univers est trop jeune pour en former une.
        "Naine noire" => photosphere((0.012, 0.013), (0.5, 0.7)),
        // 13 à 80 masses de Jupiter ; un rayon proche de celui de Jupiter.
        "Naine brune" => photosphere((0.08, 0.12), (0.012, 0.076)),

        // Pas de photosphère : la température du gabarit sert au rendu, pas à la
        // physique. Les luminosités sont des **ordres de grandeur**.
        "Etoile a neutrons" | "Pulsar" => compact(R_NEUTRONS, (1.0, 2.0), (0.001, 0.26)), // ~
        "Magnetar" => compact(R_NEUTRONS, (1.0, 2.0), (0.26, 26.0)), // ~
        // Rayon de Schwarzschild, ~2,95 km par masse solaire.
        "Trou noir" => compact((2.1e-5, 6.4e-5), (5.0, 15.0), (0.01, 1.0)), // ~
        // Hypothétique : 38 à 189 UA, au moins 1 000 masses, « aussi lumineuse
        // qu'une petite galaxie ».
        "Quasi-etoile" => compact((8200.0, 40700.0), (1000.0, 10000.0), (1e8, 1e10)), // ~
        _ => None,
    }
}

/// Interpolation géométrique : ces grandeurs couvrent des ordres de grandeur.
fn entre_log((a, b): (f32, f32), q: f32) -> f32 {
    a * (b / a).powf(q.clamp(0.0, 1.0))
}

/// **Les vraies valeurs d'une étoile** de ce type, `q` donnant sa place dans ses
/// plages (les grandes sont aussi les massives), à la température tirée.
pub fn vraies(nom: &str, q: f32, temperature: f32) -> Option<Vraies> {
    let p = plages(nom)?;
    let rayon = entre_log(p.rayon, q);
    let luminosite = p.luminosite.map_or_else(|| stefan_boltzmann(rayon, temperature), |l| entre_log(l, q));
    Some(Vraies { rayon, masse: entre_log(p.masse, q), luminosite })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::etoile::{gabarit_du_type, types_du_tirage};

    #[test]
    fn chaque_type_du_tirage_a_ses_valeurs() {
        let types = types_du_tirage();
        assert_eq!(types.len(), 29);
        for t in types {
            let p = plages(t).unwrap_or_else(|| panic!("« {t} » n'a pas de valeurs"));
            for (a, b) in [p.rayon, p.masse] {
                assert!(a > 0.0 && a <= b, "{t} : plage {a}..{b}");
            }
        }
        assert!(plages("Licorne").is_none());
    }

    /// **La loi retrouve la table publiée** : `(T, R, log L)` de Mamajek, lus à
    /// la source. Garde la loi et `T_SOLEIL` contre des valeurs qui ne viennent
    /// pas de ce fichier.
    #[test]
    fn la_loi_retrouve_la_sequence_principale_publiee() {
        for (spt, t, r, log_l) in [
            ("O5V", 41400.0, 11.45, 5.54),
            ("B0V", 31400.0, 7.16, 4.65),
            ("B5V", 15700.0, 3.36, 2.77),
            ("A0V", 9700.0, 2.193, 1.58),
            ("F5V", 6550.0, 1.473, 0.56),
            ("G2V", 5770.0, 1.012, 0.01),
            ("K5V", 4440.0, 0.701, -0.76),
            ("M3V", 3430.0, 0.361, -1.79),
            ("M8V", 2570.0, 0.114, -3.28),
        ] {
            let calcule = stefan_boltzmann(r, t).log10();
            assert!((calcule - log_l).abs() < 0.05, "{spt} : log L {calcule} au lieu de {log_l}");
        }
    }

    #[test]
    fn le_soleil_tient_dans_sa_classe() {
        let p = plages("G (jaune)").unwrap();
        assert!(p.rayon.0 < 1.0 && 1.0 < p.rayon.1 && p.masse.0 < 1.0 && 1.0 < p.masse.1);
        assert_eq!(stefan_boltzmann(1.0, T_SOLEIL), 1.0);
    }

    /// De M à O, rayon et masse montent, sans chevauchement notable.
    #[test]
    fn les_classes_de_la_sequence_principale_montent() {
        let classes = ["M (naine rouge)", "K (orange)", "G (jaune)", "F (blanc-jaune)", "A (blanche)", "B (bleu-blanc)", "O (bleue)"];
        for c in classes.windows(2) {
            let (bas, haut) = (plages(c[0]).unwrap(), plages(c[1]).unwrap());
            assert!(bas.rayon.1 <= haut.rayon.0 * 1.05, "rayon {} / {}", c[0], c[1]);
            assert!(bas.masse.1 <= haut.masse.0 * 1.05, "masse {} / {}", c[0], c[1]);
        }
    }

    /// **Chaque luminosité publiée tombe dans ce que le type peut produire** :
    /// les extrêmes de ses rayons et des températures de son gabarit. C'est ce qui
    /// garde le passage par la loi : un rayon faux sortirait de la plage publiée.
    /// Tolérance ×1,5 : ces sources sont des ordres de grandeur (Bételgeuse, 10 000 L☉
    /// pour 370 R☉, ne vérifie pas la loi à sa propre température). Elle reste sous
    /// l'erreur visée : un rayon faux d'un facteur 2 change L d'un facteur 4.
    #[test]
    fn les_luminosites_publiees_sont_atteignables() {
        for (t, publiees) in [
            ("Geante rouge", &[3000.0][..]),
            ("Supergeante rouge", &[10_000.0, 250_000.0, 300_000.0][..]),
            ("Cepheide", &[1_000.0, 50_000.0][..]),
            ("Mira", &[5_700.0][..]),
            ("Etoile carbonee (C)", &[5_417.0, 6_000.0][..]),
            ("Wolf-Rayet", &[251_000.0, 398_000.0, 630_000.0][..]),
            ("Hypergeante (LBV)", &[316_000.0][..]),
            ("Supergeante bleue", &[10_000.0 * 2.5, 1_000_000.0][..]),
            ("T Tauri (jeune)", &[2.0][..]),
            ("Herbig Ae/Be (jeune)", &[50.0][..]),
            ("Naine brune", &[1.0 / 8000.0][..]),
        ] {
            let p = plages(t).unwrap();
            let g = gabarit_du_type(t, 0.5).unwrap();
            let (lo, hi) = (stefan_boltzmann(p.rayon.0, g.0), stefan_boltzmann(p.rayon.1, g.1));
            for &l in publiees {
                assert!(lo / 1.5 <= l && l <= hi * 1.5, "{t} : {l} hors de {lo}..{hi}");
            }
        }
    }

    #[test]
    fn la_place_dans_la_plage_est_geometrique() {
        let v = |q| vraies("Cepheide", q, T_SOLEIL).unwrap();
        assert_eq!(v(0.0).rayon, 30.0);
        assert!((v(1.0).rayon - 200.0).abs() < 1e-2);
        let milieu = (30.0_f32 * 200.0).sqrt();
        assert!((v(0.5).rayon - milieu).abs() < 1e-2, "milieu {}", v(0.5).rayon);
        assert!((v(0.5).luminosite - milieu * milieu).abs() < 1.0, "photosphère : la loi");
        // Une luminosité explicite ne dépend pas de la température du gabarit.
        let pulsar = |t| vraies("Pulsar", 0.5, t).unwrap().luminosite;
        assert_eq!(pulsar(40_000.0), pulsar(1_000.0));
    }
}
