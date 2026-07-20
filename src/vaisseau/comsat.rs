use super::{cone, parabole, voile};
use macroquad::prelude::*;

/// Satellite de télécommunication géostationnaire détaillé : gros corps en
/// feuille dorée, grande parabole émettrice + deux paraboles secondaires et
/// des cornets d'alimentation, deux très longues ailes solaires nervurées,
/// tuyère de maintien à poste.
pub fn dessiner_comsat() {
    let or = Color::new(0.75, 0.62, 0.22, 1.0);
    let gris = Color::new(0.80, 0.82, 0.86, 1.0);
    let sombre = Color::new(0.30, 0.30, 0.33, 1.0);
    let bleu = Color::new(0.05, 0.08, 0.30, 1.0);

    // Corps + panneau de charge utile (face avant, cornets).
    let corps = vec3(0.6, 0.7, 0.6);
    draw_cube(Vec3::ZERO, corps, None, or);
    draw_cube_wires(Vec3::ZERO, corps, sombre);
    draw_cube(vec3(0.0, 0.0, 0.31), vec3(0.5, 0.6, 0.02), None, gris);
    for dx in [-0.14_f32, 0.14] {
        for dy in [-0.14_f32, 0.14] {
            cone(vec3(dx, dy, 0.32), Vec3::Z, 0.03, 0.07, 0.12, sombre); // cornets
        }
    }

    // Grande parabole principale orientée vers +Z.
    draw_line_3d(vec3(0.0, 0.0, 0.3), vec3(0.0, 0.0, 0.5), sombre);
    parabole(vec3(0.0, 0.0, 0.5), Vec3::Z, 0.34, gris);
    // Deux paraboles secondaires (est/ouest), plus petites.
    for signe in [-1.0_f32, 1.0] {
        draw_line_3d(vec3(signe * 0.25, 0.28, 0.2), vec3(signe * 0.32, 0.4, 0.35), sombre);
        parabole(vec3(signe * 0.32, 0.4, 0.35), vec3(signe * 0.3, 0.4, 1.0), 0.15, gris);
    }

    // Deux très longues ailes solaires nervurées (axe X).
    for signe in [-1.0_f32, 1.0] {
        draw_line_3d(vec3(signe * 0.3, 0.0, 0.0), vec3(signe * 0.45, 0.0, 0.0), sombre);
        let coin = vec3(signe * 0.45, -0.25, -0.25);
        let e1 = vec3(signe * 1.65, 0.0, 0.0);
        let e2 = vec3(0.0, 0.0, 0.5);
        voile(coin, e1, e2, bleu, 9);
    }

    // Tuyère d'apogée / maintien à poste (vers −Z).
    cone(vec3(0.0, -0.2, -0.31), -Vec3::Z, 0.05, 0.10, 0.16, sombre);
}
