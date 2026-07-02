use super::panneau;
use macroquad::prelude::*;

/// Petite station modulaire façon Mir/ISS : nœud central, modules radiaux
/// en croix, treillis et grands panneaux solaires aux extrémités.
pub fn dessiner_station() {
    let blanc_casse = Color::new(0.85, 0.85, 0.82, 1.0);
    let gris = Color::new(0.70, 0.72, 0.75, 1.0);
    let gris_fonce = Color::new(0.30, 0.30, 0.33, 1.0);
    let bleu_panneau = Color::new(0.05, 0.10, 0.32, 1.0);

    // Nœud central.
    let noeud = vec3(0.5, 0.5, 0.5);
    draw_cube(Vec3::ZERO, noeud, None, blanc_casse);
    draw_cube_wires(Vec3::ZERO, noeud, gris_fonce);

    // Modules radiaux (croix, façon Mir).
    let modules: [(Vec3, Vec3); 4] = [
        (vec3(0.85, 0.0, 0.0), vec3(1.2, 0.35, 0.35)),
        (vec3(-0.85, 0.0, 0.0), vec3(1.2, 0.35, 0.35)),
        (vec3(0.0, 0.0, 0.85), vec3(0.35, 0.35, 1.2)),
        (vec3(0.0, 0.0, -0.85), vec3(0.35, 0.35, 1.2)),
    ];
    for (pos, taille) in modules {
        draw_cube(pos, taille, None, gris);
        draw_cube_wires(pos, taille, gris_fonce);
    }

    // Treillis + grands panneaux solaires aux deux extrémités (façon ISS).
    for signe in [-1.0_f32, 1.0] {
        let base = vec3(signe * 1.45, 0.0, 0.0);
        let bout = vec3(signe * 2.6, 0.0, 0.0);
        draw_line_3d(base, bout, gris_fonce);
        for haut in [-1.0_f32, 1.0] {
            let coin = vec3(bout.x, haut * 0.05, -0.5);
            let e1 = vec3(0.0, 0.0, 1.0);
            let e2 = vec3(0.0, haut * 1.1, 0.0);
            panneau(coin, e1, e2, bleu_panneau);
        }
    }
}
