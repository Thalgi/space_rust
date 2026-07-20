use super::{cone, cylindre, parabole};
use macroquad::prelude::*;

/// Sonde interplanétaire façon Voyager/Pioneer, détaillée : bus décagonal,
/// grande antenne à grand gain, perche des générateurs RTG avec ailettes,
/// perche d'instruments avec plateforme de scan (caméras) et longue perche de
/// magnétomètre, plus le « disque d'or ».
pub fn dessiner_voyager() {
    let or = Color::new(0.78, 0.64, 0.24, 1.0);
    let gris = Color::new(0.82, 0.83, 0.86, 1.0);
    let sombre = Color::new(0.28, 0.28, 0.31, 1.0);
    let rouge = Color::new(0.5, 0.16, 0.10, 1.0);
    let dore = Color::new(0.85, 0.72, 0.30, 1.0);

    // Bus décagonal (approximé par un court cylindre à 10 côtés).
    cylindre(vec3(0.0, -0.13, 0.0), vec3(0.0, 0.13, 0.0), 0.42, or);
    draw_cube_wires(Vec3::ZERO, vec3(0.7, 0.26, 0.7), sombre);

    // Grande antenne à grand gain au-dessus (tige + réflecteur orienté).
    draw_line_3d(vec3(0.0, 0.13, 0.0), vec3(0.0, 0.32, 0.0), sombre);
    parabole(vec3(0.0, 0.32, 0.0), Vec3::Y, 0.52, gris);
    // Sous-réflecteur / cornet au foyer.
    draw_sphere(vec3(0.0, 0.75, 0.0), 0.05, None, sombre);

    // Disque d'or fixé sur le flanc du bus.
    draw_cube(vec3(0.36, 0.0, 0.18), vec3(0.02, 0.22, 0.22), None, dore);

    // Perche RTG : ligne + 3 générateurs à ailettes de refroidissement.
    let dir = vec3(-1.5, -0.18, 0.0);
    draw_line_3d(vec3(-0.3, -0.05, 0.0), dir, sombre);
    for k in 0..3 {
        let p = vec3(-0.8 - 0.25 * k as f32, -0.12, 0.0);
        cone(p + vec3(0.0, 0.0, -0.12), Vec3::Z, 0.11, 0.11, 0.24, rouge); // cylindre RTG
        // Ailettes.
        for a in 0..4 {
            let ang = a as f32 / 4.0 * std::f32::consts::TAU;
            draw_line_3d(
                p,
                p + vec3(0.15 * ang.cos(), 0.15 * ang.sin(), 0.0),
                sombre,
            );
        }
    }

    // Perche d'instruments : plateforme de scan avec caméras (petits cubes).
    let sci = vec3(0.55, -0.35, 0.0);
    draw_line_3d(vec3(0.25, -0.1, 0.0), sci, sombre);
    draw_cube(sci, vec3(0.2, 0.16, 0.2), None, gris);
    draw_cube(sci + vec3(0.05, 0.0, 0.14), vec3(0.08, 0.08, 0.1), None, sombre); // caméra
    draw_cube(sci + vec3(-0.05, 0.08, 0.12), vec3(0.06, 0.06, 0.1), None, sombre);

    // Longue perche de magnétomètre (fine, opposée).
    draw_line_3d(vec3(0.15, 0.08, 0.0), vec3(1.65, 0.3, 0.0), sombre);
    draw_sphere(vec3(1.65, 0.3, 0.0), 0.05, None, gris);
}
