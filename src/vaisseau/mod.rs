//! Vue « hangar » : maquettes simples de sondes, satellites, navettes et
//! stations spatiales. Pour l'instant ce sont des spécimens isolés, dessinés
//! avec des primitives 3D (cube/cylindre/cône/parallélogramme) — pas encore
//! intégrés à `Systeme`/`Astre`. La sélection, le déplacement dans un système
//! et la mise en orbite autour d'un astre viendront plus tard (voir
//! BUCKETLIST) : ce module est volontairement découplé de la gravité N-corps.

mod assemblage;
mod atterrisseur;
mod brique_demo;
mod chantier;
mod composant;
mod generateur;
mod comsat;
mod cubesat;
mod futur;
mod gps;
mod iss;
mod montage;
mod navette;
mod pieces;
mod port;
mod sonde;
mod station; // module Mir (fichier historique réutilisé)
mod symetrie;
mod telescope;
mod tiangong;
mod unites;
mod voyager;

pub use assemblage::{Assembleur, Budget, EtatStation, Piece, Station};
pub use composant::{
    Composant, Sorties, StyleTreillis, VarianteAntenne, VarianteModule, VariantePanneau,
    VarianteRadiateur,
};
pub use generateur::{generer, preset_iss, preset_mir, Ossature, ParamsStation, Style};
pub use montage::{
    cuire, demo_antennes, demo_chantier, demo_deux_modules, demo_habitats, demo_panneaux,
    demo_poutres, demo_radiateurs, demo_station, demo_treillis,
};
pub use port::{accoupler, GenrePort, Port, Repere};
pub use symetrie::Symetrie;
pub use unites::Profil;

use macroquad::prelude::*;

pub use atterrisseur::dessiner_atterrisseur;
pub use brique_demo::Brique;
pub use comsat::dessiner_comsat;
pub use cubesat::dessiner_cubesat;
pub use futur::dessiner_futur;
pub use gps::dessiner_gps;
pub use iss::dessiner_iss;
pub use navette::dessiner_navette;
pub use sonde::dessiner_sonde;
pub use station::dessiner_mir;
pub use telescope::dessiner_telescope;
pub use tiangong::dessiner_tiangong;
pub use voyager::dessiner_voyager;

/// Les engins présentés dans la galerie.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypeEngin {
    Sonde,
    CubeSat,
    Gps,
    ComSat,
    Telescope,
    Voyager,
    Atterrisseur,
    Navette,
    Mir,
    Iss,
    Tiangong,
    Futur,
}

impl TypeEngin {
    /// Tous les engins, dans l'ordre d'affichage de la grille (3 par ligne).
    pub const TOUS: [TypeEngin; 12] = [
        TypeEngin::Sonde,
        TypeEngin::CubeSat,
        TypeEngin::Gps,
        TypeEngin::ComSat,
        TypeEngin::Telescope,
        TypeEngin::Voyager,
        TypeEngin::Atterrisseur,
        TypeEngin::Navette,
        TypeEngin::Mir,
        TypeEngin::Iss,
        TypeEngin::Tiangong,
        TypeEngin::Futur,
    ];

    pub fn nom(&self) -> &'static str {
        match self {
            TypeEngin::Sonde => "SONDE",
            TypeEngin::CubeSat => "CUBESAT 3U",
            TypeEngin::Gps => "SAT. NAVIGATION",
            TypeEngin::ComSat => "SAT. COMM",
            TypeEngin::Telescope => "TELESCOPE",
            TypeEngin::Voyager => "SONDE VOYAGER",
            TypeEngin::Atterrisseur => "ATTERRISSEUR",
            TypeEngin::Navette => "NAVETTE",
            TypeEngin::Mir => "STATION MIR",
            TypeEngin::Iss => "STATION ISS",
            TypeEngin::Tiangong => "STATION TIANGONG",
            TypeEngin::Futur => "STATION ORBITALE",
        }
    }

    /// Demi-dimensions (x = demi-largeur, y = demi-hauteur) approximatives de la
    /// maquette à l'écran, utilisées pour espacer la grille sans chevauchement.
    pub fn demi_dim(&self) -> Vec2 {
        match self {
            TypeEngin::Sonde => vec2(1.30, 0.85),
            TypeEngin::CubeSat => vec2(0.35, 0.75),
            TypeEngin::Gps => vec2(1.60, 0.60),
            TypeEngin::ComSat => vec2(2.05, 0.55),
            TypeEngin::Telescope => vec2(1.50, 0.95),
            TypeEngin::Voyager => vec2(1.65, 1.15),
            TypeEngin::Atterrisseur => vec2(0.50, 1.00),
            TypeEngin::Navette => vec2(1.20, 0.85),
            TypeEngin::Mir => vec2(1.50, 1.30),
            TypeEngin::Iss => vec2(2.85, 1.70),
            TypeEngin::Tiangong => vec2(1.95, 1.30),
            TypeEngin::Futur => vec2(1.90, 1.75),
        }
    }

    /// Dessine l'engin centré à l'origine (repère caméra 3D déjà actif).
    pub fn dessiner(&self) {
        match self {
            TypeEngin::Sonde => dessiner_sonde(),
            TypeEngin::CubeSat => dessiner_cubesat(),
            TypeEngin::Gps => dessiner_gps(),
            TypeEngin::ComSat => dessiner_comsat(),
            TypeEngin::Telescope => dessiner_telescope(),
            TypeEngin::Voyager => dessiner_voyager(),
            TypeEngin::Atterrisseur => dessiner_atterrisseur(),
            TypeEngin::Navette => dessiner_navette(),
            TypeEngin::Mir => dessiner_mir(),
            TypeEngin::Iss => dessiner_iss(),
            TypeEngin::Tiangong => dessiner_tiangong(),
            TypeEngin::Futur => dessiner_futur(),
        }
    }
}

/// Panneau plat visible des deux côtés : macroquad ne double-face pas les
/// parallélogrammes, donc on le redessine avec les arêtes inversées.
pub(crate) fn panneau(coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color) {
    draw_affine_parallelogram(coin, e1, e2, None, couleur);
    draw_affine_parallelogram(coin, e2, e1, None, couleur);
}

/// Cylindre (module pressurisé, poutre) reliant deux points de l'espace.
/// `draw_cylinder` est aligné sur +Y ; on le réoriente via une matrice de
/// modèle pour aller de `a` à `b`.
pub(crate) fn cylindre(a: Vec3, b: Vec3, rayon: f32, couleur: Color) {
    let axe = b - a;
    let h = axe.length();
    if h < 1e-5 {
        return;
    }
    let rot = Quat::from_rotation_arc(Vec3::Y, axe / h);
    unsafe {
        get_internal_gl()
            .quad_gl
            .push_model_matrix(Mat4::from_rotation_translation(rot, a));
    }
    draw_cylinder(Vec3::ZERO, rayon, rayon, h, None, couleur);
    unsafe {
        get_internal_gl().quad_gl.pop_model_matrix();
    }
}

/// Panneau solaire nervuré : le parallélogramme plein, son contour et
/// `cellules-1` nervures parallèles à `e1` réparties le long de `e2`.
pub(crate) fn voile(coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color, cellules: usize) {
    let bord = Color::new(couleur.r * 0.5, couleur.g * 0.5, couleur.b * 0.5, 1.0);
    panneau(coin, e1, e2, couleur);
    draw_line_3d(coin, coin + e1, bord);
    draw_line_3d(coin + e2, coin + e1 + e2, bord);
    draw_line_3d(coin, coin + e2, bord);
    draw_line_3d(coin + e1, coin + e1 + e2, bord);
    for n in 1..cellules.max(1) {
        let a = coin + e2 * (n as f32 / cellules as f32);
        draw_line_3d(a, a + e1, bord);
    }
}

/// Cône ou tronc de cône orienté (tuyère, jupe, capot). `r_base` est le rayon
/// à `base`, `r_bout` le rayon à `base + direction*longueur`.
pub(crate) fn cone(base: Vec3, direction: Vec3, r_base: f32, r_bout: f32, longueur: f32, couleur: Color) {
    let d = direction.normalize_or_zero();
    if d == Vec3::ZERO {
        return;
    }
    let rot = Quat::from_rotation_arc(Vec3::Y, d);
    unsafe {
        get_internal_gl()
            .quad_gl
            .push_model_matrix(Mat4::from_rotation_translation(rot, base));
    }
    draw_cylinder(Vec3::ZERO, r_bout, r_base, longueur, None, couleur);
    unsafe {
        get_internal_gl().quad_gl.pop_model_matrix();
    }
}

/// Antenne parabolique orientée : cône peu profond dont la face ouverte pointe
/// vers `direction`. Contrairement à une sphère, sa silhouette dépend de
/// l'angle de vue — elle ne « fait plus systématiquement face » à la caméra.
pub(crate) fn parabole(centre: Vec3, direction: Vec3, rayon: f32, couleur: Color) {
    let d = direction.normalize_or_zero();
    if d == Vec3::ZERO {
        return;
    }
    let prof = rayon * 0.5;
    let bord = Color::new(couleur.r * 0.5, couleur.g * 0.5, couleur.b * 0.5, 1.0);
    let rot = Quat::from_rotation_arc(Vec3::Y, d);
    unsafe {
        get_internal_gl()
            .quad_gl
            .push_model_matrix(Mat4::from_rotation_translation(rot, centre));
    }
    // Apex du cône en (0,0,0), bord (rim) ouvert en (0,prof,0) → face vers `d`.
    draw_cylinder(Vec3::ZERO, rayon, 0.0, prof, None, couleur);
    draw_cylinder_wires(Vec3::ZERO, rayon, 0.0, prof, None, bord);
    unsafe {
        get_internal_gl().quad_gl.pop_model_matrix();
    }
    // Petite source (feed) au foyer, devant la parabole.
    draw_line_3d(centre + d * prof, centre + d * (prof + rayon * 0.6), bord);
    draw_sphere(centre + d * (prof + rayon * 0.6), rayon * 0.12, None, bord);
}
