use super::{cone, cylindre, parabole, voile};
use macroquad::prelude::*;

/// Atterrisseur planétaire façon Philae/InSight, détaillé : plateforme
/// hexagonale, trépied de pattes à amortisseurs et patins, pont solaire
/// nervuré, mât d'imagerie stéréo, bras robotique avec godet, antenne
/// parabolique, générateur RTG et antenne fouet.
pub fn dessiner_atterrisseur() {
    let or = Color::new(0.74, 0.62, 0.24, 1.0);
    let gris = Color::new(0.72, 0.74, 0.78, 1.0);
    let sombre = Color::new(0.28, 0.28, 0.31, 1.0);
    let bleu = Color::new(0.07, 0.10, 0.30, 1.0);
    let rouge = Color::new(0.5, 0.16, 0.10, 1.0);

    // Plateforme (corps) hexagonale approximée par un cylindre à faces.
    cylindre(vec3(0.0, 0.02, 0.0), vec3(0.0, 0.28, 0.0), 0.42, or);
    draw_cube_wires(vec3(0.0, 0.15, 0.0), vec3(0.72, 0.26, 0.72), sombre);

    // Trois pattes : jambe principale + contrefiche + patin.
    let pieds = [
        vec3(0.0, 0.0, 0.5),
        vec3(-0.43, 0.0, -0.3),
        vec3(0.43, 0.0, -0.3),
    ];
    for p in pieds {
        let haut = vec3(p.x * 0.5, 0.15, p.z * 0.5);
        let bas = vec3(p.x, -0.45, p.z);
        cone(haut, (bas - haut).normalize(), 0.05, 0.03, (bas - haut).length(), gris); // jambe
        draw_line_3d(vec3(p.x * 0.5, 0.28, p.z * 0.5), bas, gris); // contrefiche
        draw_sphere(bas, 0.08, None, sombre); // patin
    }

    // Pont solaire nervuré sur le dessus.
    voile(
        vec3(-0.34, 0.30, -0.34),
        vec3(0.68, 0.0, 0.0),
        vec3(0.0, 0.0, 0.68),
        bleu,
        5,
    );

    // Mât d'imagerie stéréo (tête caméra).
    draw_line_3d(vec3(0.22, 0.3, 0.22), vec3(0.22, 0.9, 0.22), sombre);
    let tete = vec3(0.22, 0.92, 0.22);
    draw_cube(tete, vec3(0.16, 0.08, 0.08), None, gris);
    draw_sphere(tete + vec3(-0.05, 0.0, 0.05), 0.03, None, sombre);
    draw_sphere(tete + vec3(0.05, 0.0, 0.05), 0.03, None, sombre);

    // Bras robotique (deux segments) + godet.
    let b0 = vec3(-0.2, 0.3, 0.3);
    let b1 = vec3(-0.55, 0.15, 0.55);
    let b2 = vec3(-0.75, -0.35, 0.6);
    cone(b0, (b1 - b0).normalize(), 0.04, 0.04, (b1 - b0).length(), gris);
    cone(b1, (b2 - b1).normalize(), 0.04, 0.03, (b2 - b1).length(), gris);
    draw_sphere(b1, 0.06, None, sombre);
    draw_cube(b2, vec3(0.12, 0.06, 0.1), None, sombre); // godet

    // Antenne parabolique orientée vers le ciel.
    draw_line_3d(vec3(-0.2, 0.3, -0.2), vec3(-0.28, 0.55, -0.28), sombre);
    parabole(vec3(-0.28, 0.55, -0.28), vec3(-0.3, 1.0, -0.3), 0.16, gris);

    // Générateur RTG (boîtier rouge à ailettes) et antenne fouet.
    draw_cube(vec3(0.3, 0.36, -0.25), vec3(0.16, 0.12, 0.16), None, rouge);
    draw_line_3d(vec3(-0.3, 0.3, 0.0), vec3(-0.4, 0.75, 0.05), sombre);
}
