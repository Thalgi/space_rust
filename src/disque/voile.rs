//! Couche « voile » d'un champ de débris : annulus low-poly STATIQUE (local,
//! plan Y) + fragment shader procédural. Tout le contenu (plateau, bandes,
//! lacunes, granulation, arcs, émissif, rotation différentielle) vit dans les
//! uniforms — les 5 styles de l'ex-anneau V1 sont devenus des presets de
//! `DisqueConfig` (voir config.rs).

use super::DisqueConfig;
use crate::impostor;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::models::{Mesh, Vertex};
use macroquad::prelude::*;
use std::f32::consts::TAU;

const N_RADIAL: usize = 10; // bandes radiales (le profil fin est dans le shader)
const N_SEG: usize = 96;    // segments angulaires (bord externe lisse)
const QUADS_PAR_LOT: usize = 400;

const VERT: &str = include_str!("../shaders/disque_voile.vert.glsl");
const FRAG: &str = include_str!("../shaders/disque_voile.frag.glsl");

/// Annulus en quads indépendants, coordonnées LOCALES (plan Y, centre 0).
/// L'orientation (normale) et la translation passent en uniforms — le mesh
/// est figé une fois pour toutes.
pub(super) fn construire_annulus(c: &DisqueConfig) -> Vec<Mesh> {
    let mut lots: Vec<Mesh> = Vec::new();
    let mut verts: Vec<Vertex> = Vec::with_capacity(QUADS_PAR_LOT * 4);
    let mut inds: Vec<u16> = Vec::with_capacity(QUADS_PAR_LOT * 6);
    let blanc = Color::new(1.0, 1.0, 1.0, 1.0);

    for j in 0..N_RADIAL {
        let t0 = j as f32 / N_RADIAL as f32;
        let t1 = (j + 1) as f32 / N_RADIAL as f32;
        let r0 = c.interne + (c.externe - c.interne) * t0;
        let r1 = c.interne + (c.externe - c.interne) * t1;
        for k in 0..N_SEG {
            let a0 = k as f32 / N_SEG as f32;
            let a1 = (k + 1) as f32 / N_SEG as f32;
            let (s0, c0) = (a0 * TAU).sin_cos();
            let (s1, c1) = (a1 * TAU).sin_cos();
            let p00 = vec3(c0 * r0, 0.0, s0 * r0);
            let p01 = vec3(c0 * r1, 0.0, s0 * r1);
            let p11 = vec3(c1 * r1, 0.0, s1 * r1);
            let p10 = vec3(c1 * r0, 0.0, s1 * r0);
            let i0 = verts.len() as u16;
            verts.push(Vertex::new2(p00, vec2(t0, a0), blanc));
            verts.push(Vertex::new2(p01, vec2(t1, a0), blanc));
            verts.push(Vertex::new2(p11, vec2(t1, a1), blanc));
            verts.push(Vertex::new2(p10, vec2(t0, a1), blanc));
            inds.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
            if verts.len() >= QUADS_PAR_LOT * 4 {
                lots.push(Mesh {
                    vertices: std::mem::take(&mut verts),
                    indices: std::mem::take(&mut inds),
                    texture: None,
                });
            }
        }
    }
    if !inds.is_empty() {
        lots.push(Mesh {
            vertices: verts,
            indices: inds,
            texture: None,
        });
    }
    lots
}

/// Pousse tous les uniforms du voile. `moitie` : -1 arrière / +1 avant / 0 tout
/// (le fragment shader discard l'autre moitié via dot(pos, tocam)).
pub(super) fn uniforms_voile(
    mat: &Material,
    c: &DisqueConfig,
    temps: f32,
    centre: Vec3,
    axes: (Vec3, Vec3, Vec3),
    tocam: Vec3,
    moitie: f32,
    lacune_phases: Vec4,
    dir_lum: Vec3,
    face_lum: f32,
    ombre_rayon: f32,
    lum_couleur: Vec3,
) {
    let (u, n, v) = axes;
    mat.set_uniform("time", temps);
    mat.set_uniform("seed", c.graine);
    mat.set_uniform("centre", (centre.x, centre.y, centre.z));
    mat.set_uniform("axe_u", (u.x, u.y, u.z));
    mat.set_uniform("axe_n", (n.x, n.y, n.z));
    mat.set_uniform("axe_v", (v.x, v.y, v.z));
    mat.set_uniform("tocam", (tocam.x, tocam.y, tocam.z));
    mat.set_uniform("moitie", moitie);
    mat.set_uniform("alpha", c.voile_alpha);
    mat.set_uniform("couleur", (c.voile_couleur.x, c.voile_couleur.y, c.voile_couleur.z));
    mat.set_uniform(
        "couleur2",
        (c.voile_couleur2.x, c.voile_couleur2.y, c.voile_couleur2.z),
    );
    mat.set_uniform("plateau", c.voile_plateau);
    mat.set_uniform("alpha_interne", c.voile_alpha_interne);
    mat.set_uniform("bord", c.voile_bord);
    mat.set_uniform("granulation", c.granulation);
    mat.set_uniform("gran_seuil", c.gran_seuil);
    mat.set_uniform("gran_freq", (c.gran_freq.x, c.gran_freq.y));
    mat.set_uniform("arcs", c.arcs);
    mat.set_uniform("emissif", c.emissif);
    mat.set_uniform("rotation", c.rotation_voile);
    mat.set_uniform("r_ratio", (c.externe / c.interne.max(1e-3)).max(1.001));
    mat.set_uniform(
        "lacune_phase",
        (lacune_phases.x, lacune_phases.y, lacune_phases.z, lacune_phases.w),
    );
    mat.set_uniform("dir_lumiere", (dir_lum.x, dir_lum.y, dir_lum.z));
    mat.set_uniform("face_lum", face_lum);
    mat.set_uniform("ombre_rayon", ombre_rayon);
    mat.set_uniform("lum_couleur", (lum_couleur.x, lum_couleur.y, lum_couleur.z));
    mat.set_uniform_array("bandes", &c.lacunes[..]);
}

/// Material du voile : alpha blend, sans écriture de profondeur (comme
/// l'ex-anneau V1). Hot-reload via R.
pub(super) fn material_voile() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("disque_voile.vert.glsl", VERT),
            fragment: &impostor::source("disque_voile.frag.glsl", FRAG),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("time", UniformType::Float1),
                UniformDesc::new("seed", UniformType::Float1),
                UniformDesc::new("centre", UniformType::Float3),
                UniformDesc::new("axe_u", UniformType::Float3),
                UniformDesc::new("axe_n", UniformType::Float3),
                UniformDesc::new("axe_v", UniformType::Float3),
                UniformDesc::new("tocam", UniformType::Float3),
                UniformDesc::new("moitie", UniformType::Float1),
                UniformDesc::new("alpha", UniformType::Float1),
                UniformDesc::new("couleur", UniformType::Float3),
                UniformDesc::new("couleur2", UniformType::Float3),
                UniformDesc::new("plateau", UniformType::Float1),
                UniformDesc::new("alpha_interne", UniformType::Float1),
                UniformDesc::new("bord", UniformType::Float1),
                UniformDesc::new("granulation", UniformType::Float1),
                UniformDesc::new("gran_seuil", UniformType::Float1),
                UniformDesc::new("gran_freq", UniformType::Float2),
                UniformDesc::new("arcs", UniformType::Float1),
                UniformDesc::new("emissif", UniformType::Float1),
                UniformDesc::new("rotation", UniformType::Float1),
                UniformDesc::new("r_ratio", UniformType::Float1),
                UniformDesc::new("lacune_phase", UniformType::Float4),
                UniformDesc::new("dir_lumiere", UniformType::Float3),
                UniformDesc::new("face_lum", UniformType::Float1),
                UniformDesc::new("ombre_rayon", UniformType::Float1),
                UniformDesc::new("lum_couleur", UniformType::Float3),
                UniformDesc::array(UniformDesc::new("bandes", UniformType::Float4), 4),
            ],
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}
