//! Vue « hangar » : maquettes simples de sondes, navettes et stations
//! spatiales. Pour l'instant ce sont des spécimens isolés, dessinés à
//! l'origine avec des primitives 3D (cube/sphère/parallélogramme) — pas
//! encore intégrés à `Systeme`/`Astre`. La sélection, le déplacement dans
//! un système et la mise en orbite autour d'un astre viendront plus tard
//! (voir BUCKETLIST) : ce module est volontairement découplé de la gravité
//! N-corps en attendant cette étape.

mod navette;
mod sonde;
mod station;

use macroquad::prelude::*;

pub use navette::dessiner_navette;
pub use sonde::dessiner_sonde;
pub use station::dessiner_station;

/// Les trois types d'engins prévus pour l'instant.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypeEngin {
    Sonde,
    Navette,
    Station,
}

impl TypeEngin {
    pub fn nom(&self) -> &'static str {
        match self {
            TypeEngin::Sonde => "SONDE - SATELLITE",
            TypeEngin::Navette => "NAVETTE SPATIALE",
            TypeEngin::Station => "STATION SPATIALE (MIR/ISS)",
        }
    }

    /// Distance caméra adaptée à la taille de l'engin.
    pub fn distance_camera(&self) -> f32 {
        match self {
            TypeEngin::Sonde => 4.0,
            TypeEngin::Navette => 7.0,
            TypeEngin::Station => 10.0,
        }
    }

    /// Dessine l'engin centré à l'origine (le repère caméra 3D doit déjà être actif).
    pub fn dessiner(&self) {
        match self {
            TypeEngin::Sonde => dessiner_sonde(),
            TypeEngin::Navette => dessiner_navette(),
            TypeEngin::Station => dessiner_station(),
        }
    }

    pub fn suivant(&self) -> TypeEngin {
        match self {
            TypeEngin::Sonde => TypeEngin::Navette,
            TypeEngin::Navette => TypeEngin::Station,
            TypeEngin::Station => TypeEngin::Sonde,
        }
    }
}

/// Panneau plat visible des deux côtés : macroquad ne double-face pas les
/// parallélogrammes, donc on le redessine avec les arêtes inversées.
pub(crate) fn panneau(coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color) {
    draw_affine_parallelogram(coin, e1, e2, None, couleur);
    draw_affine_parallelogram(coin, e2, e1, None, couleur);
}
