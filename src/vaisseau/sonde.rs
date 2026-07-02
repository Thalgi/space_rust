use super::panneau;
use macroquad::prelude::*;

/// Petite sonde/satellite : corps cubique, antenne, deux panneaux solaires.
pub fn dessiner_sonde() {
    let gris = Color::new(0.78, 0.80, 0.85, 1.0);
    let gris_fonce = Color::new(0.35, 0.36, 0.40, 1.0);
    let bleu_panneau = Color::new(0.06, 0.08, 0.28, 1.0);
    let blanc = Color::new(0.95, 0.95, 0.98, 1.0);

    // Corps.
    let taille = vec3(0.42, 0.50, 0.42);
    draw_cube(Vec3::ZERO, taille, None, gris);
    draw_cube_wires(Vec3::ZERO, taille, gris_fonce);

    // Antenne : tige + parabole (simplifiée en petite sphère).
    draw_line_3d(vec3(0.0, 0.25, 0.0), vec3(0.0, 0.68, 0.0), gris_fonce);
    draw_sphere(vec3(0.0, 0.72, 0.0), 0.08, None, blanc);

    // Panneaux solaires de part et d'autre du corps.
    for signe in [-1.0_f32, 1.0] {
        let coin = vec3(signe * 0.21, 0.17, -0.15);
        let e1 = vec3(signe * 1.05, 0.0, 0.0);
        let e2 = vec3(0.0, 0.0, 0.30);
        panneau(coin, e1, e2, bleu_panneau);
        // Cadre (silhouette) du panneau.
        draw_line_3d(coin, coin + e1, gris_fonce);
        draw_line_3d(coin + e2, coin + e1 + e2, gris_fonce);
        draw_line_3d(coin, coin + e2, gris_fonce);
        draw_line_3d(coin + e1, coin + e1 + e2, gris_fonce);
    }
}
