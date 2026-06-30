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
    pub flares: f32,      // 0 = calme, 1 = active (flares fréquents : naine M, T Tauri)
}

impl ProfilEtoile {
    /// Tire une étoile au hasard. Quelques variantes spéciales (géante rouge,
    /// naine blanche) en plus de la séquence principale O–M (M majoritaires).
    pub fn aleatoire() -> Self {
        let v: f32 = gen_range(0.0, 1.0);
        // (t_min, t_max, rayon, luminosité, nom, mode de couronne)
        let (tmin, tmax, rayon, lumi, nom, couronne) = if v < 0.05 {
            // Étoiles particulières (rares).
            let p: f32 = gen_range(0.0, 1.0);
            if p < 0.2 {
                (35000.0, 45000.0, 0.42, 0.25, "Pulsar", 3.0)
            } else if p < 0.4 {
                (35000.0, 45000.0, 0.42, 0.2, "Etoile a neutrons", 1.0)
            } else if p < 0.55 {
                (35000.0, 45000.0, 0.45, 0.3, "Magnetar", 4.0)
            } else if p < 0.75 {
                (38000.0, 45000.0, 1.1, 5.0, "Wolf-Rayet", 2.0)
            } else if p < 0.9 {
                (18000.0, 24000.0, 1.4, 6.0, "Supergeante bleue", 2.0)
            } else {
                (4000.0, 4600.0, 0.9, 1.2, "T Tauri (jeune)", 1.0)
            }
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
        // Étoiles actives : naines M (très magnétiquement actives) et T Tauri jeunes.
        let flares = if nom.contains("naine rouge") || nom.contains("Tauri") {
            1.0
        } else {
            0.0
        };
        Self {
            temperature,
            couleur: couleur_corps_noir(temperature),
            rayon,
            luminosite: lumi,
            nom,
            couronne,
            flares,
        }
    }

    pub fn nom(&self) -> &'static str {
        self.nom
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
        (288.122_19 * (t - 60.0).powf(-0.075_514_85)).clamp(0.0, 255.0)
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
