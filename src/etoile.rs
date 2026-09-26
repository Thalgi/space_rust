use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Profil physique d'une étoile.
pub struct ProfilEtoile {
    pub temperature: f32, // Kelvin
    pub couleur: Vec3,    // couleur de corps noir
    pub rayon: f32,       // rayon visuel (unités du jeu)
    pub luminosite: f32,  // relative (éclaire les planètes)
    pub nom: &'static str,
    pub couronne: f32,    // mode de couronne : 0 halo, 1 jets, 2 vent, 3 pulsar, 4 magnétar
}

/// Un gabarit d'étoile : `(t_min, t_max, rayon, luminosité, nom, couronne)`.
pub(crate) type Gabarit = (f32, f32, f32, f32, &'static str, f32);

/// Part du tirage réservée aux étoiles **particulières**.
const PART_PARTICULIERES: f32 = 0.05;

/// **Quelle étoile particulière**, pour un tirage `p` dans `[0 ; 1]`.
///
/// Extrait de [`ProfilEtoile::aleatoire`] pour être **testable** :
/// `macroquad::rand::gen_range` exige le contexte graphique, donc le tirage
/// lui-même ne se teste pas — mais ce qu'on en fait, si. C'est le seul
/// endroit du dépôt où se décide quels objets exotiques existent.
pub(crate) fn particuliere(p: f32) -> Gabarit {
    // ⚠️ Le trou noir tient **un cinquième** des particulières, soit 1 % de
    // tous les tirages. C'est bien plus que sa part réelle parmi les objets
    // stellaires — mais le but est qu'on puisse en **rencontrer un en
    // explorant**, pas de simuler un recensement. Même esprit que les 5 %
    // de particulières, déjà très au-dessus du réel.
    if p < 0.2 {
        (5500.0, 7500.0, 0.62, 0.4, "Trou noir", 5.0)
    } else if p < 0.36 {
        (35000.0, 45000.0, 0.42, 0.25, "Pulsar", 3.0)
    } else if p < 0.52 {
        (35000.0, 45000.0, 0.42, 0.2, "Etoile a neutrons", 1.0)
    } else if p < 0.64 {
        (35000.0, 45000.0, 0.45, 0.3, "Magnetar", 4.0)
    } else if p < 0.8 {
        (38000.0, 45000.0, 1.1, 5.0, "Wolf-Rayet", 2.0)
    } else if p < 0.92 {
        (18000.0, 24000.0, 1.4, 6.0, "Supergeante bleue", 2.0)
    } else {
        (4000.0, 4600.0, 0.9, 1.2, "T Tauri (jeune)", 6.0)
    }
}

impl ProfilEtoile {
    /// Tire une étoile au hasard. Quelques variantes spéciales (géante rouge,
    /// naine blanche) en plus de la séquence principale O–M (M majoritaires).
    pub fn aleatoire() -> Self {
        let v: f32 = gen_range(0.0, 1.0);
        // (t_min, t_max, rayon, luminosité, nom, mode de couronne)
        let (tmin, tmax, rayon, lumi, nom, couronne) = if v < PART_PARTICULIERES {
            particuliere(gen_range(0.0, 1.0))
        } else if v < 0.12 {
            // Géante rouge : énorme, froide, très lumineuse.
            (3000.0, 3900.0, gen_range(4.5, 7.0), gen_range(3.0, 6.0), "Geante rouge", 0.0)
        } else if v < 0.18 {
            // Naine blanche : minuscule, très chaude, peu lumineuse.
            (12000.0, 28000.0, gen_range(0.5, 0.8), gen_range(0.15, 0.4), "Naine blanche", 0.0)
        } else {
            // Séquence principale (poids biaisés pour la variété).
            let r: f32 = gen_range(0.0, 1.0);
            if r < 0.36 {
                (2400.0, 3700.0, 1.3, 0.4, "M (naine rouge)", 0.0)
            } else if r < 0.58 {
                (3700.0, 5200.0, 1.6, 0.7, "K (orange)", 0.0)
            } else if r < 0.74 {
                (5200.0, 6000.0, 2.0, 1.0, "G (jaune)", 0.0)
            } else if r < 0.85 {
                (6000.0, 7500.0, 2.3, 1.4, "F (blanc-jaune)", 0.0)
            } else if r < 0.93 {
                (7500.0, 11000.0, 2.6, 2.0, "A (blanche)", 0.0)
            } else if r < 0.98 {
                (11000.0, 30000.0, 3.2, 3.0, "B (bleu-blanc)", 0.0)
            } else {
                (30000.0, 45000.0, 4.0, 4.0, "O (bleue)", 0.0)
            }
        };
        let temperature = gen_range(tmin, tmax);
        Self {
            temperature,
            couleur: couleur_corps_noir(temperature),
            rayon,
            luminosite: lumi,
            nom,
            couronne,
        }
    }

    pub fn nom(&self) -> &'static str {
        self.nom
    }

    /// Rémanent stellaire compact (pulsar, magnétar, étoile à neutrons). Ces objets
    /// n'ont **pas de zone habitable** pertinente (leur éclat visible ne trace pas
    /// une zone d'eau liquide) et leurs planètes rescapées orbitent serré
    /// (cf. PSR B1257+12 : 0,19–0,46 UA). Sert à adapter distances et affichage.
    pub fn est_remnant(&self) -> bool {
        matches!(self.nom, "Pulsar" | "Etoile a neutrons" | "Magnetar" | "Trou noir")
    }

    /// **Trou noir.** Il se distingue des autres rémanents sur deux points :
    /// aucune surface ne s'y allume (le moteur d'éruptions s'arrête net, voir
    /// `eruptions::maj`), et sa **masse ne se déduit pas de sa luminosité**
    /// (`genese::MASSE_TROU_NOIR`).
    pub fn est_trou_noir(&self) -> bool {
        self.nom == "Trou noir"
    }
}

/// Unités du monde par UA (unité astronomique). Les distances orbitales sont
/// raisonnées en UA puis converties à l'échelle de rendu via cette constante.
pub const UA: f32 = 48.0;

/// Température d'équilibre approximative d'une planète (Kelvin), d'après la
/// luminosité de l'étoile et la distance **en UA** : T = 278 · L^¼ / √(d_UA).
/// Calée sur le réel : L=1, d=1 UA → 278 K (la Terre).
pub fn temp_equilibre(luminosite: f32, distance_ua: f32) -> f32 {
    278.0 * luminosite.powf(0.25) / distance_ua.max(0.01).sqrt()
}

/// Bornes (interne, externe) de la zone habitable **en UA** : eau liquide
/// (~320 K → ~260 K). Se déplace avec la luminosité de l'étoile.
pub fn zone_habitable(luminosite: f32) -> (f32, f32) {
    let k = 278.0 * luminosite.powf(0.25);
    let interne = (k / 320.0).powi(2);
    let externe = (k / 260.0).powi(2);
    (interne, externe)
}

/// **Taux d'activité éruptive propre au type d'étoile**, dérivé de son gabarit
/// (rayon, luminosité) — les deux seuls traits que connaît déjà `Soleil::new`,
/// pas besoin d'un troisième canal pour ce qui n'est qu'un facteur d'échelle.
///
/// Une naine M (petite, peu lumineuse) est active ; une géante (grande, très
/// lumineuse) l'est peu. `gabarit = rayon × √luminosité` croît avec les deux,
/// et l'activité en décroît — borné à `[0,15 ; 1,0]` : jamais nulle (une
/// géante flambe rarement, pas jamais), jamais démesurée.
pub fn activite_type(rayon: f32, luminosite: f32) -> f32 {
    let gabarit = rayon.max(0.01) * luminosite.max(0.0).sqrt();
    (1.6 / (1.0 + gabarit)).clamp(0.15, 1.0)
}

/// Couleur approximative d'un corps noir à `temp` Kelvin (approx. de Tanner Helland).
pub fn couleur_corps_noir(temp: f32) -> Vec3 {
    let t = temp.clamp(1000.0, 40000.0) / 100.0;

    let r = if t <= 66.0 {
        255.0
    } else {
        (329.698_73 * (t - 60.0).powf(-0.133_204_76)).clamp(0.0, 255.0)
    };
    let g = if t <= 66.0 {
        (99.470_8 * t.ln() - 161.119_57).clamp(0.0, 255.0)
    } else {
        (288.122_2 * (t - 60.0).powf(-0.075_514_85)).clamp(0.0, 255.0)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        (138.517_73 * (t - 10.0).ln() - 305.044_8).clamp(0.0, 255.0)
    };

    vec3(r / 255.0, g / 255.0, b / 255.0)
}

#[cfg(test)]
mod tests_particulieres {
    use super::*;

    /// Les noms que le tirage des particulières doit savoir produire.
    const ATTENDUS: [&str; 7] = [
        "Trou noir",
        "Pulsar",
        "Etoile a neutrons",
        "Magnetar",
        "Wolf-Rayet",
        "Supergeante bleue",
        "T Tauri (jeune)",
    ];

    fn balayage() -> Vec<&'static str> {
        (0..1000).map(|k| particuliere(k as f32 / 1000.0).4).collect()
    }

    /// **Chaque type particulier est atteignable.** Une borne mal posée en
    /// écarte un **en silence** : son rendu continue d'exister, personne ne le
    /// rencontre jamais. C'est exactement ce qui est arrivé au trou noir.
    #[test]
    fn chaque_type_particulier_est_atteignable() {
        let vus = balayage();
        for nom in ATTENDUS {
            assert!(vus.contains(&nom), "{nom} est injoignable au tirage");
        }
        // Et rien d'autre ne sort : un nom inconnu ne serait rattaché à aucun
        // rendu, ni à `est_remnant`.
        for nom in &vus {
            assert!(ATTENDUS.contains(nom), "type inattendu au tirage : {nom}");
        }
    }

    /// **On peut rencontrer un trou noir en explorant** — c'est tout l'objet
    /// du lot. Un cinquième des particulières, donc environ 1 % des étoiles.
    #[test]
    fn un_trou_noir_est_rencontrable() {
        let part = balayage().iter().filter(|n| **n == "Trou noir").count() as f32 / 1000.0;
        assert!(part > 0.1, "le trou noir ne tient que {part} des particulières : introuvable");
        assert!(part < 0.35, "il en tient {part} : ce n'est plus une rencontre, c'est la règle");
        // Sur l'ensemble du tirage, il reste une rareté.
        let global = part * PART_PARTICULIERES;
        assert!((0.002..0.02).contains(&global), "{global} de tous les tirages");
    }

    /// **Un trou noir porte le mode de couronne 5** — celui que
    /// `Soleil::avec_trou_noir` attend, et sur lequel `eruptions::maj`
    /// s'arrête (pas de taches sur un horizon).
    #[test]
    fn un_trou_noir_porte_son_mode_de_couronne() {
        let (_, _, _, lumi, nom, couronne) = particuliere(0.0);
        assert_eq!(nom, "Trou noir");
        assert_eq!(couronne, 5.0, "le rendu de trou noir ne se déclenchera pas");
        assert!(lumi > 0.0, "un trou noir tout noir n'éclairerait plus rien de son disque");
    }

    /// **Et il n'a pas de zone habitable.** Elle se déduit de la luminosité,
    /// qui ici ne décrit pas une photosphère mais un disque d'accrétion : la
    /// tracer donnerait un anneau d'eau liquide autour d'un trou noir.
    #[test]
    fn un_trou_noir_n_a_pas_de_zone_habitable() {
        let tn = ProfilEtoile {
            temperature: 6500.0,
            couleur: Vec3::ONE,
            rayon: 0.62,
            luminosite: 0.4,
            nom: "Trou noir",
            couronne: 5.0,
        };
        assert!(tn.est_trou_noir());
        assert!(tn.est_remnant(), "une zone habitable serait tracée autour de l'horizon");
    }
}

#[cfg(test)]
mod tests_activite {
    use super::activite_type;

    /// **La hiérarchie qu'on cherche : une naine M dépasse une géante.**
    /// Gabarits tirés de `ProfilEtoile::aleatoire` (`etoile.rs:46` et `:38`).
    #[test]
    fn une_naine_m_est_plus_active_qu_une_geante_rouge() {
        let naine_m = activite_type(1.3, 0.4);
        let geante = activite_type(5.5, 4.0);
        assert!(naine_m > geante, "naine {naine_m} <= géante {geante}");
    }

    /// Une jeune étoile (T Tauri, petite et modérément lumineuse) reste active,
    /// pas assommée par sa luminosité un peu plus haute qu'une naine M.
    #[test]
    fn une_jeune_etoile_reste_active() {
        let t_tauri = activite_type(0.9, 1.2);
        let geante = activite_type(5.5, 4.0);
        assert!(t_tauri > geante, "T Tauri {t_tauri} <= géante {geante}");
    }

    /// **Monotone et borné**, tel que réclamé : grossir le rayon (à luminosité
    /// fixe) ne peut que faire baisser ou stagner l'activité, jamais grimper ;
    /// et le résultat reste dans `[0,15 ; 1,0]` sur toute la plage tirée par
    /// `ProfilEtoile::aleatoire`.
    #[test]
    fn monotone_et_borne() {
        let mut precedent = f32::INFINITY;
        for rayon in [0.4_f32, 0.9, 1.3, 2.0, 3.2, 4.0, 5.5, 7.0] {
            let a = activite_type(rayon, 1.0);
            assert!(a <= precedent + 1e-6, "activité remonte à rayon {rayon} : {a} > {precedent}");
            assert!((0.15..=1.0).contains(&a), "rayon {rayon} -> {a}, hors bornes");
            precedent = a;
        }
    }
}
