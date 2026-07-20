use super::{cylindre, parabole, voile};
use macroquad::prelude::*;

/// Station Mir détaillée : module central DOS, nœud sphérique multi-ports avec
/// modules qui rayonnent (Kvant-2, Kristall, Spektr, Priroda), module Kvant à
/// l'arrière, plusieurs paires de panneaux solaires nervurés, un vaisseau
/// Soyouz amarré et des antennes.
pub fn dessiner_mir() {
    let blanc = Color::new(0.85, 0.85, 0.82, 1.0);
    let gris = Color::new(0.70, 0.72, 0.75, 1.0);
    let or = Color::new(0.72, 0.60, 0.25, 1.0);
    let sombre = Color::new(0.30, 0.30, 0.33, 1.0);
    let bleu = Color::new(0.05, 0.10, 0.32, 1.0);
    let vert = Color::new(0.30, 0.45, 0.30, 1.0);

    // Module central (le long de Z) + Kvant à l'arrière.
    cylindre(vec3(0.0, 0.0, -1.0), vec3(0.0, 0.0, 0.7), 0.30, blanc);
    cylindre(vec3(0.0, 0.0, -1.0), vec3(0.0, 0.0, -1.7), 0.27, gris);
    draw_sphere(vec3(0.0, 0.0, -1.7), 0.22, None, or); // nœud arrière

    // Nœud d'amarrage sphérique avant (multi-ports) + modules radiaux.
    let noeud = vec3(0.0, 0.0, 0.85);
    draw_sphere(noeud, 0.26, None, gris);
    let modules = [
        (vec3(0.0, 1.0, 0.15), or),   // Kvant-2 (haut)
        (vec3(0.0, -1.0, 0.15), blanc), // Priroda (bas)
        (vec3(1.0, 0.0, 0.15), or),   // Kristall (droite)
        (vec3(-1.0, 0.0, 0.15), gris), // Spektr (gauche)
    ];
    for (dir, couleur) in modules {
        cylindre(noeud, noeud + dir, 0.20, couleur);
        draw_sphere(noeud + dir, 0.10, None, sombre); // port en bout
    }
    // Module axial avant (vers +Z) + Soyouz amarré (vert).
    cylindre(noeud, noeud + vec3(0.0, 0.0, 0.55), 0.22, blanc);
    let soyouz = noeud + vec3(0.0, 0.0, 0.85);
    cylindre(soyouz, soyouz + vec3(0.0, 0.0, 0.4), 0.16, vert);
    draw_sphere(soyouz + vec3(0.0, 0.0, 0.42), 0.14, None, vert);

    // Panneaux solaires nervurés : une paire sur le module central, une paire
    // sur les modules haut/droite.
    for signe in [-1.0_f32, 1.0] {
        let coin = vec3(signe * 0.3, -0.45, -0.35);
        voile(coin, vec3(signe * 1.15, 0.0, 0.0), vec3(0.0, 0.0, 0.7), bleu, 6);
    }
    voile(
        noeud + vec3(-0.4, 0.85, 0.0),
        vec3(0.8, 0.0, 0.0),
        vec3(0.0, 0.0, 0.55),
        bleu,
        4,
    );
    voile(
        noeud + vec3(0.85, -0.4, 0.0),
        vec3(0.0, 0.8, 0.0),
        vec3(0.0, 0.0, 0.55),
        bleu,
        4,
    );

    // Antenne parabolique de communication.
    parabole(noeud + vec3(0.0, 1.05, 0.15), vec3(0.2, 1.0, 0.4), 0.18, gris);
}
