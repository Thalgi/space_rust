use super::{cylindre, parabole, voile};
use macroquad::prelude::*;

/// Station Tiangong (CSS) détaillée : configuration en T. Module cœur Tianhe
/// le long de Z, nœud d'amarrage avec deux laboratoires (Wentian, Mengtian)
/// sur ±X, grandes ailes solaires souples nervurées, bras robotique, vaisseau
/// cargo Tianzhou et vaisseau habité Shenzhou amarrés.
pub fn dessiner_tiangong() {
    let blanc = Color::new(0.86, 0.86, 0.83, 1.0);
    let gris = Color::new(0.72, 0.74, 0.78, 1.0);
    let sombre = Color::new(0.30, 0.30, 0.33, 1.0);
    let bleu = Color::new(0.06, 0.12, 0.34, 1.0);
    let vert = Color::new(0.30, 0.45, 0.32, 1.0);

    // Module cœur Tianhe (le long de Z) + cargo Tianzhou à l'arrière.
    cylindre(vec3(0.0, 0.0, -1.2), vec3(0.0, 0.0, 0.7), 0.34, blanc);
    cylindre(vec3(0.0, 0.0, -1.2), vec3(0.0, 0.0, -1.8), 0.28, gris);
    draw_sphere(vec3(0.0, 0.0, -1.85), 0.16, None, vert); // vaisseau cargo

    // Nœud avant + deux laboratoires sur ±X (barre du T).
    let noeud = vec3(0.0, 0.0, 0.8);
    draw_sphere(noeud, 0.28, None, gris);
    for signe in [-1.0_f32, 1.0] {
        let bout = noeud + vec3(signe * 1.35, 0.0, 0.0);
        cylindre(noeud, bout, 0.30, blanc);
        draw_sphere(bout, 0.16, None, gris); // sas en bout de labo
        // Grandes ailes solaires souples à l'extrémité (deux voiles en ±Z).
        for sz in [-1.0_f32, 1.0] {
            voile(
                vec3(bout.x - 0.35, -0.04, sz * 0.15),
                vec3(0.7, 0.0, 0.0),
                vec3(0.0, 0.0, sz * 1.2),
                bleu,
                7,
            );
        }
    }

    // Ailes solaires nervurées sur le module cœur.
    for signe in [-1.0_f32, 1.0] {
        voile(
            vec3(signe * 0.34, -0.04, -1.05),
            vec3(signe * 0.9, 0.0, 0.0),
            vec3(0.0, 0.0, 0.6),
            bleu,
            5,
        );
    }

    // Vaisseau habité Shenzhou amarré au port avant (+Z).
    let shenzhou = noeud + vec3(0.0, 0.0, 0.5);
    cylindre(noeud, shenzhou, 0.2, blanc);
    cylindre(shenzhou, shenzhou + vec3(0.0, 0.0, 0.35), 0.16, vert);
    for signe in [-1.0_f32, 1.0] {
        voile(
            shenzhou + vec3(signe * 0.16, -0.25, 0.05),
            vec3(signe * 0.55, 0.0, 0.0),
            vec3(0.0, 0.5, 0.0),
            bleu,
            3,
        );
    }

    // Bras robotique (Chinarm) articulé sur le cœur.
    let a0 = vec3(0.3, 0.28, -0.2);
    let a1 = vec3(0.7, 0.6, 0.1);
    let a2 = vec3(1.1, 0.3, 0.3);
    cylindre(a0, a1, 0.05, gris);
    cylindre(a1, a2, 0.05, gris);
    draw_sphere(a1, 0.07, None, sombre);

    // Antenne parabolique orientée vers l'avant.
    parabole(vec3(0.0, 0.3, 0.55), vec3(0.0, 0.4, 1.0), 0.2, gris);
}
