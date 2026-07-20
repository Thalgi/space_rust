//! Éclairage des maquettes (vaisseaux, briques, stations) : un material unique
//! qui ombre TOUTES les primitives macroquad (cylindres, cubes, sphères,
//! mailles) par normales de facette calculées en dérivées écran — aucune
//! donnée de normale requise dans les sommets, aucune géométrie modifiée.
//!
//! Usage : `eclairage::avec(&cam_pos, || { ...dessins... })`.

use crate::impostor;
use macroquad::prelude::*;
use std::cell::RefCell;

const VERT: &str = include_str!("../shaders/station.vert.glsl");
const FRAG: &str = include_str!("../shaders/station.frag.glsl");

thread_local! {
    static MAT: RefCell<Option<Material>> = const { RefCell::new(None) };
}

/// Direction (monde) VERS la lumière-clé, style « soleil de trois-quarts ».
const LUM_DIR: Vec3 = Vec3::new(0.45, 0.72, 0.52);

fn materiau() -> Material {
    MAT.with(|c| {
        if c.borrow().is_none() {
            let mat = load_material(
                ShaderSource::Glsl {
                    vertex: &impostor::source("station.vert.glsl", VERT),
                    fragment: &impostor::source("station.frag.glsl", FRAG),
                },
                MaterialParams {
                    uniforms: vec![
                        UniformDesc::new("lum_dir", UniformType::Float3),
                        UniformDesc::new("cam_pos", UniformType::Float3),
                    ],
                    pipeline_params: PipelineParams {
                        depth_test: Comparison::LessOrEqual,
                        depth_write: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            *c.borrow_mut() = Some(mat);
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// Libère le material (recréation au prochain usage — utile si le contexte GL
/// est recyclé, même logique que `planete::materiau::reset`).
#[allow(dead_code)]
pub fn reset() {
    MAT.with(|c| *c.borrow_mut() = None);
}

/// Exécute `dessin` avec l'éclairage de facette actif, puis rend la main au
/// material par défaut de macroquad (textes, gizmos, etc. non affectés).
pub fn avec<F: FnOnce()>(cam_pos: Vec3, dessin: F) {
    let mat = materiau();
    mat.set_uniform("lum_dir", LUM_DIR.normalize());
    mat.set_uniform("cam_pos", cam_pos);
    gl_use_material(&mat);
    dessin();
    gl_use_default_material();
}
