use super::{cylindre, parabole, voile};
use macroquad::prelude::*;

/// Télescope spatial façon Hubble, détaillé : tube cylindrique avec pare-lumière
/// et volet d'ouverture, miroir primaire au fond, deux ailes solaires nervurées,
/// deux antennes paraboliques sur perches, mains-courantes et anneaux de
/// structure.
pub fn dessiner_telescope() {
    let argent = Color::new(0.80, 0.78, 0.55, 1.0);
    let noir = Color::new(0.03, 0.03, 0.05, 1.0);
    let sombre = Color::new(0.30, 0.30, 0.33, 1.0);
    let bleu = Color::new(0.07, 0.11, 0.30, 1.0);
    let gris = Color::new(0.78, 0.80, 0.84, 1.0);

    // Tube principal (cylindre le long de Z).
    cylindre(vec3(0.0, 0.0, -0.9), vec3(0.0, 0.0, 0.9), 0.36, argent);
    // Anneaux de structure.
    for z in [-0.5_f32, 0.0, 0.5] {
        cylindre(vec3(0.0, 0.0, z - 0.01), vec3(0.0, 0.0, z + 0.01), 0.37, sombre);
    }

    // Pare-lumière + volet d'ouverture entrouvert à l'avant.
    cylindre(vec3(0.0, 0.0, 0.9), vec3(0.0, 0.0, 1.05), 0.36, gris);
    parabole(vec3(0.0, 0.0, 0.35), Vec3::Z, 0.30, noir); // miroir primaire au fond
    // Volet relevé (panneau incliné au bord de l'ouverture).
    super::panneau(
        vec3(-0.36, 0.05, 1.05),
        vec3(0.72, 0.0, 0.0),
        vec3(0.0, 0.5, 0.25),
        gris,
    );

    // Deux ailes solaires nervurées.
    for signe in [-1.0_f32, 1.0] {
        draw_line_3d(vec3(signe * 0.36, 0.0, 0.0), vec3(signe * 0.5, 0.0, 0.0), sombre);
        let coin = vec3(signe * 0.5, -0.45, -0.5);
        let e1 = vec3(signe * 1.05, 0.0, 0.0);
        let e2 = vec3(0.0, 0.0, 1.0);
        voile(coin, e1, e2, bleu, 7);
    }

    // Deux antennes paraboliques sur perches (haut et bas).
    draw_line_3d(vec3(0.0, 0.36, 0.4), vec3(0.0, 0.62, 0.5), sombre);
    parabole(vec3(0.0, 0.62, 0.5), vec3(0.0, 1.0, 0.3), 0.15, gris);
    draw_line_3d(vec3(0.0, -0.36, -0.4), vec3(0.0, -0.62, -0.5), sombre);
    parabole(vec3(0.0, -0.62, -0.5), vec3(0.0, -1.0, -0.3), 0.15, gris);

    // Mains-courantes (aide EVA) le long du tube.
    for s in [-1.0_f32, 1.0] {
        draw_line_3d(vec3(s * 0.30, 0.22, -0.3), vec3(s * 0.30, 0.22, 0.3), sombre);
    }
}
