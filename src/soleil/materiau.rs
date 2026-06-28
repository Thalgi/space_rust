use crate::impostor;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::cell::RefCell;

// Materials partagés : un seul pipeline chacun, clonés à chaque soleil.
thread_local! {
    static TPL_SOLEIL: RefCell<Option<Material>> = RefCell::new(None);
    static TPL_PLASMA: RefCell<Option<Material>> = RefCell::new(None);
}

pub(super) fn mat_soleil() -> Material {
    TPL_SOLEIL.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_soleil());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

pub(super) fn mat_plasma_partage() -> Material {
    TPL_PLASMA.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_plasma());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// Vide le cache de materials -> recompilation depuis les .glsl (hot-reload).
pub(super) fn vider_cache() {
    TPL_SOLEIL.with(|c| *c.borrow_mut() = None);
    TPL_PLASMA.with(|c| *c.borrow_mut() = None);
}

fn charger_soleil() -> Material {
    let mut uniforms = impostor::uniforms_communs();
    uniforms.extend([
        UniformDesc::new("teinte", UniformType::Float3),
        UniformDesc::new("couronne", UniformType::Float1),
        UniformDesc::new("couronne_irreg", UniformType::Float1),
        UniformDesc::new("couronne_type", UniformType::Float1),
        UniformDesc::array(UniformDesc::new("spots", UniformType::Float4), super::MAX_TACHES),
    ]);
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR),
            fragment: &impostor::source("soleil.frag.glsl", FRAG),
        },
        MaterialParams {
            uniforms,
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
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

fn charger_plasma() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("soleil_plasma.vert.glsl", VERT_PLASMA),
            fragment: &impostor::source("soleil_plasma.frag.glsl", FRAG_PLASMA),
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One)),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

// Plasma : texture de halo teintée par la couleur du sommet, sortie prémultipliée
// pour un blending purement additif (aucune composante noire).
const VERT_PLASMA: &str = include_str!("../shaders/soleil_plasma.vert.glsl");
const FRAG_PLASMA: &str = include_str!("../shaders/soleil_plasma.frag.glsl");
const FRAG: &str = include_str!("../shaders/soleil.frag.glsl");
