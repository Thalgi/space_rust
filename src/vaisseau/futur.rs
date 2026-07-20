use super::{cone, cylindre, parabole, voile};
use macroquad::prelude::*;
use std::f32::consts::TAU;

/// Station orbitale « maison », futuriste et détaillée : grand anneau
/// d'habitation rotatif ponctué de hublots lumineux et de nacelles, moyeu
/// central étagé relié par des rayons, spire d'amarrage axiale, tuyères à
/// plasma cyan, deux grands panneaux solaires nervurés et une antenne.
pub fn dessiner_futur() {
    let metal = Color::new(0.62, 0.64, 0.70, 1.0);
    let sombre = Color::new(0.22, 0.23, 0.28, 1.0);
    let cyan = Color::new(0.25, 0.90, 0.98, 1.0);
    let bleu = Color::new(0.05, 0.12, 0.34, 1.0);

    let r = 1.5_f32;
    let n = 24_usize;
    let point = |i: usize| {
        let a = i as f32 / n as f32 * TAU;
        vec3(r * a.cos(), r * a.sin(), 0.0)
    };

    // Anneau d'habitation (segments épais dans le plan XY).
    for i in 0..n {
        cylindre(point(i), point(i + 1), 0.16, metal);
    }
    // Hublots lumineux + nacelles d'habitation réparties sur l'anneau.
    for i in 0..n {
        draw_sphere(point(i), 0.06, None, cyan);
        if i % 4 == 0 {
            let a = i as f32 / n as f32 * TAU;
            let ext = vec3((r + 0.22) * a.cos(), (r + 0.22) * a.sin(), 0.0);
            draw_cube(ext, vec3(0.22, 0.22, 0.3), None, metal);
            draw_cube(ext + vec3(0.0, 0.0, 0.16), vec3(0.1, 0.1, 0.02), None, cyan);
        }
    }

    // Moyeu central étagé + spire d'amarrage axiale (le long de Z).
    draw_sphere(Vec3::ZERO, 0.45, None, metal);
    cylindre(vec3(0.0, 0.0, -0.35), vec3(0.0, 0.0, 0.35), 0.5, sombre); // anneau central
    cylindre(vec3(0.0, 0.0, -1.0), vec3(0.0, 0.0, 1.0), 0.16, metal); // spire
    cone(vec3(0.0, 0.0, 1.0), Vec3::Z, 0.16, 0.02, 0.25, metal); // pointe avant
    draw_sphere(vec3(0.0, 0.0, 0.6), 0.12, None, cyan); // feu d'amarrage

    // Rayons reliant le moyeu à l'anneau (6, avec combas cyan).
    for k in 0..6 {
        let a = k as f32 / 6.0 * TAU;
        let p = vec3(r * a.cos(), r * a.sin(), 0.0);
        cylindre(Vec3::ZERO, p, 0.06, metal);
        draw_sphere(p * 0.5, 0.05, None, cyan);
    }

    // Deux grands panneaux solaires nervurés en retrait derrière l'anneau.
    for signe in [-1.0_f32, 1.0] {
        cylindre(vec3(signe * 0.35, 0.0, -0.3), vec3(signe * 0.35, 0.0, -1.3), 0.05, sombre);
        voile(
            vec3(signe * 0.35, -0.7, -1.35),
            vec3(signe * 1.5, 0.0, 0.0),
            vec3(0.0, 1.4, 0.0),
            bleu,
            8,
        );
    }

    // Grappe de tuyères à plasma cyan à l'arrière.
    for dxy in [vec3(0.18, 0.0, 0.0), vec3(-0.18, 0.0, 0.0), vec3(0.0, 0.18, 0.0)] {
        cone(dxy + vec3(0.0, 0.0, -1.0), -Vec3::Z, 0.06, 0.12, 0.2, sombre);
        draw_sphere(dxy + vec3(0.0, 0.0, -1.22), 0.06, None, cyan);
    }

    // Antenne parabolique orientée.
    parabole(vec3(0.55, -0.55, 0.25), vec3(0.4, -0.3, 1.0), 0.2, metal);
}
