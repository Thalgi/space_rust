use super::{cone, panneau, voile};
use macroquad::prelude::*;

/// Navette spatiale détaillée : fuselage, nez et bords d'attaque thermiques
/// noirs, hublots de cockpit, soute ouverte avec portes et radiateurs, ailes
/// delta à élevons, dérive, deux nacelles OMS, trois moteurs principaux
/// (tuyères) et le volet de corps.
pub fn dessiner_navette() {
    let blanc = Color::new(0.92, 0.92, 0.95, 1.0);
    let noir = Color::new(0.06, 0.06, 0.08, 1.0);
    let sombre = Color::new(0.30, 0.30, 0.33, 1.0);
    let radia = Color::new(0.82, 0.84, 0.88, 1.0);

    // Fuselage.
    let fuselage = vec3(0.55, 0.55, 2.4);
    draw_cube(Vec3::ZERO, fuselage, None, blanc);
    draw_cube_wires(Vec3::ZERO, fuselage, sombre);

    // Nez thermique noir + hublots de cockpit + RCS avant.
    draw_cube(vec3(0.0, 0.0, 1.3), vec3(0.40, 0.40, 0.35), None, noir);
    draw_cube(vec3(0.0, 0.30, 0.9), vec3(0.30, 0.12, 0.5), None, noir);
    for dx in [-0.09_f32, 0.0, 0.09] {
        draw_cube(vec3(dx, 0.33, 1.0), vec3(0.06, 0.06, 0.02), None, radia); // vitres
    }

    // Soute ouverte : ouverture sombre + deux portes relevées tapissées de
    // radiateurs blancs.
    draw_cube(vec3(0.0, 0.28, -0.1), vec3(0.42, 0.02, 1.3), None, noir); // fond de soute
    for signe in [-1.0_f32, 1.0] {
        // Porte inclinée vers l'extérieur.
        voile(
            vec3(signe * 0.02, 0.28, -0.72),
            vec3(0.0, 0.0, 1.28),
            vec3(signe * 0.42, 0.22, 0.0),
            radia,
            6,
        );
    }

    // Ailes delta avec bord d'attaque noir et ligne d'élevon.
    for signe in [-1.0_f32, 1.0] {
        let coin = vec3(signe * 0.275, -0.05, -0.3);
        let e1 = vec3(signe * 0.9, 0.0, -0.6);
        let e2 = vec3(0.0, 0.0, 0.9);
        panneau(coin, e1, e2, blanc);
        draw_line_3d(coin + e1, coin + e1 + e2, noir); // bord d'attaque
        draw_line_3d(coin + e1 * 0.5, coin + e1 * 0.5 + e2, sombre); // élevon
    }

    // Dérive verticale + gouverne.
    let coin = vec3(-0.03, 0.275, -0.9);
    let e1 = vec3(0.06, 0.0, 0.0);
    let e2 = vec3(0.0, 0.55, -0.35);
    panneau(coin, e1, e2, blanc);
    draw_line_3d(coin + e2 * 0.6, coin + e1 + e2 * 0.6, sombre);

    // Nacelles OMS (bosses arrière) de part et d'autre de la dérive.
    for signe in [-1.0_f32, 1.0] {
        draw_cube(vec3(signe * 0.18, 0.18, -1.05), vec3(0.16, 0.18, 0.4), None, blanc);
        cone(vec3(signe * 0.18, 0.2, -1.25), -Vec3::Z, 0.05, 0.08, 0.12, sombre);
    }

    // Trois moteurs principaux (tuyères) en triangle à l'arrière.
    for pos in [
        vec3(-0.15, -0.10, -1.2),
        vec3(0.15, -0.10, -1.2),
        vec3(0.0, 0.16, -1.2),
    ] {
        cone(pos, -Vec3::Z, 0.09, 0.13, 0.24, sombre);
    }

    // Volet de corps sous les moteurs.
    draw_cube(vec3(0.0, -0.22, -1.25), vec3(0.36, 0.05, 0.18), None, blanc);
}
