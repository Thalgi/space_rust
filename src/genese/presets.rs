use super::apparences::gazeuse;
use super::{ajouter_lune, ajouter_planete, app_simple, MASSE_ETOILE};
use crate::ceinture::{Ceinture, CeintureConfig};
use crate::etoile;
use crate::planete::TypePlanete;
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;
use macroquad::rand::srand;

/// Preset reproduisant notre système solaire (Mercure → Pluton + ceintures).
pub fn construire_preset_solaire() -> (Systeme, String) {
    srand(42);
    let deg = std::f32::consts::PI / 180.0;
    let z = Vec3::ZERO;

    let mut sys = Systeme::new();
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        2.0,
        etoile::couleur_corps_noir(5800.0),
        1.0,
    )));
    let spot = vec3(0.6, -0.22, 0.77); // direction de la Grande Tache (hémisphère sud)

    use TypePlanete::{Glacee, Tellurique};
    ajouter_planete(&mut sys, 0.39, 0.205, 7.0 * deg, 0.32, 0.3,
        app_simple(Tellurique, vec3(0.55, 0.52, 0.48), vec3(0.32, 0.30, 0.28), z, 0.0));
    ajouter_planete(&mut sys, 0.72, 0.007, 3.4 * deg, 0.52, 1.0,
        app_simple(Tellurique, vec3(0.9, 0.82, 0.55), vec3(0.7, 0.62, 0.4), z, 0.0));
    let terre = ajouter_planete(&mut sys, 1.0, 0.017, 0.0, 0.55, 1.0,
        app_simple(Tellurique, vec3(0.3, 0.55, 0.25), vec3(0.4, 0.34, 0.25), vec3(0.1, 0.35, 0.75), 1.0)
            .avec_atmo(vec3(0.35, 0.55, 1.0) * 0.9));
    ajouter_lune(&mut sys, terre, 0.55);
    ajouter_planete(&mut sys, 1.52, 0.093, 1.85 * deg, 0.42, 0.4,
        app_simple(Tellurique, vec3(0.7, 0.38, 0.24), vec3(0.45, 0.24, 0.16), vec3(0.7, 0.8, 0.9), 0.05));
    let jupiter = ajouter_planete(&mut sys, 5.2, 0.049, 1.3 * deg, 1.7, 20.0,
        gazeuse(vec3(0.9, 0.66, 0.4), vec3(0.74, 0.44, 0.26), vec3(0.99, 0.95, 0.86), 11.0, 1.9, vec3(0.85, 0.7, 0.5) * 0.3)
            .avec_pole(vec3(0.47, 0.5, 0.55)).avec_jet_profil()
            .avec_tache(spot, 0.27, vec3(0.85, 0.34, 0.18))
            .avec_cyclones_pol().avec_tempetes(0.7));
    for _ in 0..3 {
        ajouter_lune(&mut sys, jupiter, 1.7);
    }
    let saturne = ajouter_planete(&mut sys, 9.58, 0.056, 2.49 * deg, 1.45, 12.0,
        gazeuse(vec3(0.9, 0.78, 0.5), vec3(0.7, 0.56, 0.34), vec3(0.97, 0.91, 0.68), 13.0, 0.9, vec3(0.88, 0.8, 0.55) * 0.3)
            .avec_jet_profil().avec_hexagone().avec_brume(0.22, vec3(0.95, 0.89, 0.68))
            .avec_pole(vec3(0.66, 0.63, 0.55))
            .avec_anneau_saturne(vec3(0.86, 0.79, 0.62)));
    for _ in 0..3 {
        ajouter_lune(&mut sys, saturne, 1.45);
    }
    let uranus = ajouter_planete(&mut sys, 19.2, 0.046, 0.77 * deg, 1.0, 6.0,
        gazeuse(vec3(0.5, 0.78, 0.78), vec3(0.34, 0.6, 0.62), vec3(0.66, 0.88, 0.88), 6.0, 0.6, vec3(0.55, 0.82, 0.85) * 0.3)
            .avec_brume(0.3, vec3(0.6, 0.82, 0.84)).avec_pole(vec3(0.55, 0.78, 0.8))
            .avec_anneau_uranus(vec3(0.55, 0.8, 0.97)));
    for _ in 0..2 {
        ajouter_lune(&mut sys, uranus, 1.0);
    }
    let neptune = ajouter_planete(&mut sys, 30.05, 0.010, 1.77 * deg, 1.0, 6.0,
        gazeuse(vec3(0.45, 0.56, 0.86), vec3(0.08, 0.16, 0.44), vec3(0.66, 0.8, 0.99), 9.0, 1.5, vec3(0.3, 0.45, 0.9) * 0.3)
            .avec_tache_sombre(spot, 0.2, vec3(0.05, 0.07, 0.18)).avec_tempetes(0.9)
            .avec_pole(vec3(0.3, 0.46, 0.74)).avec_anneau_neptune(vec3(0.6, 0.66, 0.85)));
    ajouter_lune(&mut sys, neptune, 1.0);
    ajouter_planete(&mut sys, 39.5, 0.249, 17.1 * deg, 0.3, 0.1,
        app_simple(Glacee, vec3(0.8, 0.72, 0.6), vec3(0.65, 0.6, 0.52), z, 0.0));

    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::asteroides(
        900, 2.2 * etoile::UA, 3.3 * etoile::UA, MASSE_ETOILE,
    ))));
    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::kuiper(
        2000, 30.0 * etoile::UA, 48.0 * etoile::UA, MASSE_ETOILE,
    ))));

    (sys, "Systeme solaire - jusqu'a Pluton".to_string())
}

/// Preset Tau Ceti (G8V, ~5344 K, L ≈ 0.52) : 4 super-Terres + disque de débris.
/// Données : Feng et al. 2017 (g 0.133 UA, h 0.243, e 0.538 tempérée, f 1.34).
pub fn construire_preset_tau_ceti() -> (Systeme, String) {
    srand(8552);
    let deg = std::f32::consts::PI / 180.0;
    let z = Vec3::ZERO;

    let mut sys = Systeme::new();
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        1.8,
        etoile::couleur_corps_noir(5344.0),
        0.52,
    )));

    use TypePlanete::Tellurique;
    ajouter_planete(&mut sys, 0.133, 0.06, 2.0 * deg, 0.45, 2.0,
        app_simple(Tellurique, vec3(0.55, 0.5, 0.45), vec3(0.34, 0.31, 0.28), z, 0.0));
    ajouter_planete(&mut sys, 0.243, 0.23, 1.5 * deg, 0.45, 2.0,
        app_simple(Tellurique, vec3(0.7, 0.45, 0.3), vec3(0.45, 0.28, 0.2), z, 0.0));
    ajouter_planete(&mut sys, 0.538, 0.18, 1.0 * deg, 0.6, 4.0,
        app_simple(Tellurique, vec3(0.3, 0.5, 0.25), vec3(0.4, 0.34, 0.25), vec3(0.1, 0.35, 0.7), 1.0));
    ajouter_planete(&mut sys, 1.34, 0.16, 1.5 * deg, 0.6, 4.0,
        app_simple(Tellurique, vec3(0.6, 0.4, 0.3), vec3(0.4, 0.26, 0.2), vec3(0.7, 0.8, 0.9), 0.1));

    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::kuiper(
        1600, 3.0 * etoile::UA, 16.0 * etoile::UA, MASSE_ETOILE,
    ))));

    (sys, "Tau Ceti (G8V) - 4 super-Terres".to_string())
}
