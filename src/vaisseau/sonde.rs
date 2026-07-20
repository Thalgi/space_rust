use super::{cone, parabole, voile};
use macroquad::prelude::*;

/// Sonde / satellite scientifique détaillé : corps cubique en feuille dorée,
/// bus renforcé, deux ailes solaires nervurées, antenne parabolique orientée,
/// antennes fouet, viseur d'étoiles et grappe de propulseurs.
pub fn dessiner_sonde() {
    let gris = Color::new(0.78, 0.80, 0.85, 1.0);
    let sombre = Color::new(0.35, 0.36, 0.40, 1.0);
    let or = Color::new(0.74, 0.62, 0.24, 1.0);
    let bleu = Color::new(0.07, 0.10, 0.34, 1.0);
    let blanc = Color::new(0.95, 0.95, 0.98, 1.0);

    // Corps : bus doré (feuille MLI) ceinturé d'une bande grise.
    let taille = vec3(0.42, 0.52, 0.42);
    draw_cube(Vec3::ZERO, taille, None, or);
    draw_cube_wires(Vec3::ZERO, taille, sombre);
    draw_cube(vec3(0.0, 0.0, 0.0), vec3(0.44, 0.14, 0.44), None, gris); // bande équipements

    // Antenne à grand gain : tige + parabole orientée vers le haut.
    draw_line_3d(vec3(0.0, 0.26, 0.0), vec3(0.0, 0.55, 0.0), sombre);
    parabole(vec3(0.0, 0.55, 0.0), Vec3::Y, 0.17, gris);

    // Antennes fouet basse fréquence (deux, inclinées).
    draw_line_3d(vec3(0.18, 0.26, 0.18), vec3(0.42, 0.62, 0.42), sombre);
    draw_line_3d(vec3(-0.18, 0.26, -0.18), vec3(-0.42, 0.62, -0.42), sombre);

    // Viseur d'étoiles (petit cube noir) et capteur solaire.
    draw_cube(vec3(0.12, 0.10, 0.22), vec3(0.10, 0.10, 0.05), None, sombre);
    draw_sphere(vec3(-0.14, 0.14, 0.22), 0.04, None, blanc);

    // Deux ailes solaires nervurées de part et d'autre du corps.
    for signe in [-1.0_f32, 1.0] {
        // Bras de liaison.
        draw_line_3d(vec3(signe * 0.21, 0.0, 0.0), vec3(signe * 0.32, 0.0, 0.0), sombre);
        let coin = vec3(signe * 0.32, 0.15, -0.16);
        let e1 = vec3(signe * 1.0, 0.0, 0.0);
        let e2 = vec3(0.0, 0.0, 0.32);
        voile(coin, e1, e2, bleu, 6);
    }

    // Grappe de propulseurs à l'arrière (bas du bus).
    for dx in [-0.12_f32, 0.12] {
        cone(vec3(dx, -0.26, 0.0), -Vec3::Y, 0.05, 0.08, 0.12, sombre);
    }
}
