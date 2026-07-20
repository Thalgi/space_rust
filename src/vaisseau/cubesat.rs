use super::voile;
use macroquad::prelude::*;

/// CubeSat 3U détaillé : prisme en feuille dorée avec rails d'angle, cellules
/// solaires collées sur le corps, deux volets déployés nervurés, antennes
/// fouet et une petite optique.
pub fn dessiner_cubesat() {
    let or = Color::new(0.72, 0.60, 0.22, 1.0);
    let sombre = Color::new(0.25, 0.25, 0.28, 1.0);
    let bleu = Color::new(0.07, 0.10, 0.30, 1.0);
    let alu = Color::new(0.80, 0.82, 0.86, 1.0);

    // Corps allongé (format 3U) + rails d'angle en aluminium.
    let corps = vec3(0.30, 0.30, 0.90);
    draw_cube(Vec3::ZERO, corps, None, or);
    draw_cube_wires(Vec3::ZERO, corps, sombre);
    for sx in [-1.0_f32, 1.0] {
        for sy in [-1.0_f32, 1.0] {
            draw_line_3d(
                vec3(sx * 0.15, sy * 0.15, -0.45),
                vec3(sx * 0.15, sy * 0.15, 0.45),
                alu,
            );
        }
    }

    // Cellules solaires collées sur une face (nervures sur la longueur).
    for n in 1..3 {
        let z = -0.45 + 0.90 * n as f32 / 3.0;
        draw_line_3d(vec3(-0.15, 0.151, z), vec3(0.15, 0.151, z), bleu);
    }
    draw_cube(vec3(0.0, 0.151, 0.0), vec3(0.26, 0.001, 0.82), None, bleu);

    // Deux volets solaires déployés vers le haut, nervurés.
    for signe in [-1.0_f32, 1.0] {
        let coin = vec3(-0.15, 0.15, signe * 0.05);
        let e1 = vec3(0.0, 0.55, 0.0);
        let e2 = vec3(0.0, 0.0, signe * 0.42);
        voile(coin, e1, e2, bleu, 4);
    }

    // Optique / capteur en bout, antennes fouet croisées.
    draw_cube(vec3(0.0, 0.0, 0.46), vec3(0.12, 0.12, 0.04), None, sombre);
    draw_line_3d(vec3(0.1, 0.1, 0.45), vec3(0.35, 0.35, 0.75), alu);
    draw_line_3d(vec3(-0.1, 0.1, 0.45), vec3(-0.35, 0.35, 0.75), alu);
}
