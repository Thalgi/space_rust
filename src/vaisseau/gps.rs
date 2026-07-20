use super::{cone, voile};
use macroquad::prelude::*;

/// Satellite de navigation (GPS/Galileo) détaillé : corps cubique, réseau
/// d'antennes hélicoïdales tourné vers la Terre (−Y), deux grandes ailes
/// solaires nervurées orientables, tuyère d'apogée et antennes.
pub fn dessiner_gps() {
    let gris = Color::new(0.80, 0.82, 0.86, 1.0);
    let sombre = Color::new(0.30, 0.30, 0.33, 1.0);
    let bleu = Color::new(0.06, 0.09, 0.30, 1.0);
    let or = Color::new(0.72, 0.60, 0.25, 1.0);

    // Corps.
    let corps = vec3(0.55, 0.55, 0.55);
    draw_cube(Vec3::ZERO, corps, None, gris);
    draw_cube_wires(Vec3::ZERO, corps, sombre);
    // Bandes de radiateurs sur les flancs (±Z).
    for sz in [-1.0_f32, 1.0] {
        draw_cube(vec3(0.0, 0.0, sz * 0.28), vec3(0.5, 0.5, 0.01), None, or);
    }

    // Panneau nadir (−Y) portant le réseau d'antennes hélicoïdales.
    draw_cube(vec3(0.0, -0.34, 0.0), vec3(0.5, 0.06, 0.5), None, or);
    for dx in [-0.15_f32, 0.0, 0.15] {
        for dz in [-0.15_f32, 0.0, 0.15] {
            cone(vec3(dx, -0.37, dz), -Vec3::Y, 0.04, 0.07, 0.16, sombre);
        }
    }

    // Deux ailes solaires nervurées sur l'axe X (avec bras + joint).
    for signe in [-1.0_f32, 1.0] {
        draw_line_3d(vec3(signe * 0.28, 0.0, 0.0), vec3(signe * 0.42, 0.0, 0.0), sombre);
        draw_sphere(vec3(signe * 0.42, 0.0, 0.0), 0.05, None, sombre); // joint
        let coin = vec3(signe * 0.42, -0.35, -0.35);
        let e1 = vec3(signe * 1.25, 0.0, 0.0);
        let e2 = vec3(0.0, 0.0, 0.7);
        voile(coin, e1, e2, bleu, 6);
    }

    // Tuyère du moteur d'apogée (vers +Y) et antenne de télémesure.
    cone(vec3(0.0, 0.28, 0.0), Vec3::Y, 0.05, 0.10, 0.14, sombre);
    draw_line_3d(vec3(0.18, 0.28, 0.0), vec3(0.30, 0.5, 0.0), sombre);
}
