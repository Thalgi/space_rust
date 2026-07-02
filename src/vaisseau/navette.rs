use super::panneau;
use macroquad::prelude::*;

/// Navette spatiale simplifiée : fuselage, nez thermique, ailes delta,
/// dérive verticale, cluster de moteurs.
pub fn dessiner_navette() {
    let blanc = Color::new(0.92, 0.92, 0.95, 1.0);
    let noir = Color::new(0.06, 0.06, 0.08, 1.0);
    let gris_fonce = Color::new(0.30, 0.30, 0.33, 1.0);

    // Fuselage.
    let fuselage = vec3(0.55, 0.55, 2.4);
    draw_cube(Vec3::ZERO, fuselage, None, blanc);
    draw_cube_wires(Vec3::ZERO, fuselage, gris_fonce);

    // Nez (bouclier thermique noir) à l'avant.
    draw_cube(vec3(0.0, 0.0, 1.3), vec3(0.40, 0.40, 0.35), None, noir);

    // Bande de cockpit.
    draw_cube(vec3(0.0, 0.30, 0.9), vec3(0.30, 0.12, 0.5), None, noir);

    // Ailes delta.
    for signe in [-1.0_f32, 1.0] {
        let coin = vec3(signe * 0.275, -0.05, -0.3);
        let e1 = vec3(signe * 0.9, 0.0, -0.6);
        let e2 = vec3(0.0, 0.0, 0.9);
        panneau(coin, e1, e2, blanc);
        draw_line_3d(coin + e1, coin + e1 + e2, noir); // bord d'attaque
    }

    // Dérive verticale.
    let coin = vec3(-0.03, 0.275, -0.9);
    let e1 = vec3(0.06, 0.0, 0.0);
    let e2 = vec3(0.0, 0.55, -0.35);
    panneau(coin, e1, e2, blanc);

    // Cluster de moteurs principaux à l'arrière.
    for pos in [
        vec3(-0.15, -0.10, -1.2),
        vec3(0.15, -0.10, -1.2),
        vec3(0.0, 0.18, -1.2),
    ] {
        draw_sphere(pos, 0.1, None, gris_fonce);
    }
}
