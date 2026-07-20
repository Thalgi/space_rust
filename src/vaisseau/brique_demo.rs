//! Catalogue des briques de composants, présentées une par une pour les
//! travailler au cas par cas (voir l'écran `crate::ecran::Briques`). Chaque
//! variante dessine un spécimen représentatif centré à l'origine à partir des
//! fonctions factorisées de [`super::pieces`] et des primitives orientées du
//! module parent.

use super::pieces::{module, paire_ailes, radiateur, treillis};
use super::{cone, cylindre, parabole};
use macroquad::prelude::*;

/// Un groupe de composant du catalogue.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Brique {
    Structure,
    Habitat,
    Noeud,
    PanneauSolaire,
    Radiateur,
    Parabole,
    Antenne,
    Tuyere,
}

impl Brique {
    pub const TOUS: [Brique; 8] = [
        Brique::Structure,
        Brique::Habitat,
        Brique::Noeud,
        Brique::PanneauSolaire,
        Brique::Radiateur,
        Brique::Parabole,
        Brique::Antenne,
        Brique::Tuyere,
    ];

    pub fn nom(&self) -> &'static str {
        match self {
            Brique::Structure => "STRUCTURE (TREILLIS)",
            Brique::Habitat => "HABITAT (MODULE)",
            Brique::Noeud => "NŒUD D'AMARRAGE",
            Brique::PanneauSolaire => "PANNEAU SOLAIRE",
            Brique::Radiateur => "RADIATEUR",
            Brique::Parabole => "PARABOLE",
            Brique::Antenne => "ANTENNE",
            Brique::Tuyere => "TUYERE",
        }
    }

    /// Demi-dimensions à l'écran (cadrage caméra).
    pub fn demi_dim(&self) -> Vec2 {
        match self {
            Brique::Structure => vec2(1.55, 0.5),
            Brique::Habitat => vec2(0.95, 0.6),
            Brique::Noeud => vec2(0.75, 0.75),
            Brique::PanneauSolaire => vec2(1.75, 1.05),
            Brique::Radiateur => vec2(0.95, 0.85),
            Brique::Parabole => vec2(0.7, 0.7),
            Brique::Antenne => vec2(0.6, 0.85),
            Brique::Tuyere => vec2(0.5, 0.65),
        }
    }

    pub fn dessiner(&self) {
        let blanc = Color::new(0.88, 0.88, 0.86, 1.0);
        let metal = Color::new(0.66, 0.68, 0.72, 1.0);
        let sombre = Color::new(0.28, 0.28, 0.31, 1.0);
        let ambre = Color::new(0.60, 0.42, 0.15, 1.0);
        let radia = Color::new(0.85, 0.86, 0.90, 1.0);
        let gris = Color::new(0.78, 0.80, 0.84, 1.0);

        match self {
            Brique::Structure => {
                treillis(vec3(-1.4, 0.0, 0.0), vec3(1.4, 0.0, 0.0), 0.16, metal, sombre);
            }
            Brique::Habitat => {
                module(Vec3::ZERO, Vec3::Z, 1.5, 0.42, blanc, sombre);
                draw_sphere(vec3(0.0, 0.0, 0.8), 0.2, None, metal); // port avant
                // Hublots.
                for z in [-0.4_f32, 0.0, 0.4] {
                    draw_cube(vec3(0.0, 0.42, z), vec3(0.1, 0.02, 0.1), None, sombre);
                }
            }
            Brique::Noeud => {
                draw_sphere(Vec3::ZERO, 0.42, None, blanc);
                for dir in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z] {
                    cylindre(Vec3::ZERO, dir * 0.6, 0.16, metal);
                    draw_sphere(dir * 0.62, 0.1, None, sombre); // écoutille
                }
            }
            Brique::PanneauSolaire => {
                paire_ailes(
                    Vec3::ZERO,
                    Vec3::Z,
                    Vec3::X,
                    0.25,
                    1.35,
                    0.7,
                    9,
                    ambre,
                    metal,
                    sombre,
                );
            }
            Brique::Radiateur => {
                cylindre(vec3(0.0, 0.0, -0.7), vec3(0.0, 0.0, -0.55), 0.1, metal); // pivot
                radiateur(vec3(0.0, 0.0, -0.55), Vec3::Z, Vec3::X, 1.2, 0.85, 7, radia, sombre);
            }
            Brique::Parabole => {
                cylindre(vec3(0.0, 0.0, -0.35), vec3(0.0, 0.0, -0.12), 0.05, sombre); // pied
                parabole(vec3(0.0, 0.0, -0.12), Vec3::Z, 0.5, gris);
            }
            Brique::Antenne => {
                draw_cube(vec3(0.0, -0.35, 0.0), vec3(0.18, 0.18, 0.18), None, gris); // boîtier
                draw_line_3d(vec3(0.0, -0.26, 0.0), vec3(0.0, 0.55, 0.0), sombre); // mât
                // Antennes fouet croisées.
                draw_line_3d(vec3(0.0, 0.1, 0.0), vec3(0.35, 0.5, 0.0), sombre);
                draw_line_3d(vec3(0.0, 0.1, 0.0), vec3(-0.35, 0.5, 0.0), sombre);
                draw_line_3d(vec3(0.0, 0.1, 0.0), vec3(0.0, 0.5, 0.35), sombre);
                // Petite antenne hélicoïdale en tête.
                cone(vec3(0.0, 0.55, 0.0), Vec3::Y, 0.03, 0.08, 0.2, gris);
            }
            Brique::Tuyere => {
                draw_cube(vec3(0.0, 0.4, 0.0), vec3(0.14, 0.14, 0.14), None, gris); // chambre
                cone(vec3(0.0, 0.33, 0.0), -Vec3::Y, 0.07, 0.18, 0.4, sombre); // divergent
            }
        }
    }
}
