//! Vue « hangar » : maquettes simples de sondes, satellites, navettes et
//! stations spatiales. Pour l'instant ce sont des spécimens isolés, dessinés
//! avec des primitives 3D (cube/cylindre/cône/parallélogramme) — pas encore
//! intégrés à `Systeme`/`Astre`. La sélection, le déplacement dans un système
//! et la mise en orbite autour d'un astre viendront plus tard (voir
//! BUCKETLIST) : ce module est volontairement découplé de la gravité N-corps.

mod assemblage;
mod chantier;
mod composant;
pub mod eclairage;
mod generateur;
mod maillage;
mod montage;
mod peintre;
mod pieces;
mod port;
mod symetrie;
mod unites;

pub use assemblage::{Assembleur, Budget, EtatStation, Piece, Station};
pub use composant::{
    Composant, DonneesSousEnsemble, FamillePropulsion, Sorties, StyleTreillis, VarianteAntenne,
    VarianteCaisson, VarianteCharge, VarianteCoiffe, VarianteModule, VariantePanneau,
    VariantePropulseur, VarianteRadiateur,
};
pub use generateur::{
    demo_anneaux, demo_cargo, demo_charpente, demo_moteur_antimatiere,
    demo_moteur_antimatiere_principal, demo_radiateur_mega, demo_reservoir,
    generer, preset_anneau,
    preset_comsat, preset_iss, preset_isv, preset_isv_moteur, preset_mir, preset_sonde,
    preset_tiangong, Ossature,
    ParamsStation, Style,
};
pub use maillage::MaillageStation;
pub use montage::{
    cuire, demo_antennes, demo_caissons, demo_chantier, demo_coiffes,
    demo_habitats, demo_panneaux, demo_poutres, demo_propulsion, demo_radiateurs, demo_station,
    demo_treillis,
};
pub use port::{accoupler, GenrePort, Port, Repere};
pub use symetrie::Symetrie;
pub use unites::Profil;

use macroquad::prelude::*;

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
