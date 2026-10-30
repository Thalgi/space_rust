//! **Les boules des nuages, figées sur le GPU** (Opti-v2 O7, dettes D3, D4, D8).
//!
//! Un nuage ne fait que tourner : rayon, hauteur, décalages de sa grappe,
//! taille, teinte et éclat ne changent pas d'une image à l'autre. Les
//! ~29 000 boules d'une T Tauri étaient pourtant recalculées (1,9 ms sur la
//! machine de dev) puis renvoyées à la carte (1,4 ms) à chaque image. Elles
//! vivent maintenant dans un buffer GPU construit une fois ; le shader
//! (`nuages.vert.glsl`) les fait tourner, les éclaire et pose leurs quads.
//!
//! ⚠️ **Un pipeline miniquad brut, pas un `Material`** : `draw_mesh` recopie
//! les sommets dans le lot de macroquad à chaque appel, c'est justement le
//! renvoi qu'on supprime. Il ne compte donc pas dans le plafond de 32
//! pipelines de macroquad (`le_nombre_de_shaders_reste_sous_le_plafond`).
//!
//! Le côté du plan d'une boule ne change pas en tournant (la rotation est
//! dans le plan) : elles sont rangées **dessus**, puis **dessous**, et chaque
//! moitié du disque dessine une tranche du buffer.

use super::{boule_de_grappe, hors_lacunes, nuage_fixe, teinte_nuage, DisqueConfig};
use macroquad::miniquad::{
    BlendFactor, BlendState, BufferId, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, Equation,
    PassAction, Pipeline, PipelineParams, RenderPass, ShaderMeta, ShaderSource, UniformBlockLayout, UniformDesc,
    UniformType, UniformsSource, VertexAttribute, VertexFormat,
};
use macroquad::prelude::*;
use std::cell::RefCell;

/// Une boule figée : ce qui ne change pas d'une image à l'autre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Boule {
    pub r: f32,
    pub z: f32,
    pub ang0: f32,
    pub omega: f32,
    /// Décalage dans la grappe, repère local (u, n, v).
    pub decal: Vec3,
    pub taille: f32,
    /// Teinte, sans l'éclairement (il suit la position, le shader le fait).
    pub couleur: Vec3,
    /// Alpha avant l'amorti de l'image.
    pub alpha: f32,
}

impl Boule {
    /// Position locale (u, n, v) à l'instant `temps`, décalage compris.
    pub fn locale(&self, temps: f32) -> Vec3 {
        let a = self.ang0 + self.omega * temps;
        vec3(a.cos() * self.r, self.z, a.sin() * self.r) + self.decal
    }
}

/// Les boules d'un disque, rangées **dessus** le plan puis **dessous**, par
/// rang de nuage dans chaque groupe ; et combien il y en a dessus et dessous,
/// en qualité haute puis basse (un nuage sur trois : les premiers rangs).
pub(super) struct Fixes {
    pub boules: Vec<Boule>,
    pub dessus: [usize; 2],
    pub dessous: [usize; 2],
}

impl Fixes {
    pub fn new(c: &DisqueConfig) -> Self {
        let grappe = c.grappe.max(1);
        let n = c.nuages_3d.min(40_000 / grappe);
        let mut v: Vec<(bool, bool, Boule)> = Vec::new();
        for k in 0..n {
            let f = nuage_fixe(k, c);
            let eclat = f.eclat * hors_lacunes(f.t, c);
            if eclat <= 0.0 {
                continue;
            }
            // ⚠️ **Pas teintée par la lumière de l'étoile** : elle monte à
            // chaque flare, et le disque entier battait avec — « un effet de
            // battement de cœur » (2026-10-22).
            let couleur = c.voile_couleur2.lerp(c.voile_couleur, f.t) * teinte_nuage(k, c);
            for j in 0..grappe {
                let (decal, taille, poids) = boule_de_grappe(k, j, grappe, f.taille);
                let b = Boule { r: f.r, z: f.z, ang0: f.ang0, omega: f.omega, decal, taille, couleur, alpha: eclat * 0.35 * poids };
                v.push((f.z + decal.y >= 0.0, k < n / 3, b));
            }
        }
        let compte = |dessus: bool, basse: bool| v.iter().filter(|(d, b, _)| *d == dessus && (*b || !basse)).count();
        let (dessus, dessous) = ([compte(true, false), compte(true, true)], [compte(false, false), compte(false, true)]);
        // Stable : chaque groupe garde l'ordre des rangs, la qualité basse en
        // est le début.
        v.sort_by_key(|(d, _, _)| !*d);
        Fixes { boules: v.into_iter().map(|(_, _, b)| b).collect(), dessus, dessous }
    }

    /// La tranche `(début, nombre)` à dessiner pour un côté du plan.
    pub fn tranche(&self, dessus: bool, basse: bool) -> (usize, usize) {
        let q = basse as usize;
        if dessus {
            (0, self.dessus[q])
        } else {
            (self.dessus[0], self.dessous[q])
        }
    }
}

#[repr(C)]
struct Sommet {
    orbite: [f32; 4],
    decal: [f32; 4],
    couleur: [f32; 4],
    coin: [f32; 2],
}

#[repr(C)]
pub(super) struct Uniformes {
    pub mvp: Mat4,
    pub centre: [f32; 3],
    pub axe_u: [f32; 3],
    pub axe_n: [f32; 3],
    pub axe_v: [f32; 3],
    pub cam_right: [f32; 3],
    pub cam_up: [f32; 3],
    pub temps: f32,
    pub amorti: f32,
}

/// Les buffers GPU d'un disque, construits une fois.
pub(super) struct Tampons {
    vb: BufferId,
    ib: BufferId,
}

impl Tampons {
    pub fn new(f: &Fixes) -> Self {
        let mut sommets = Vec::with_capacity(f.boules.len() * 4);
        let mut indices: Vec<u32> = Vec::with_capacity(f.boules.len() * 6);
        for b in &f.boules {
            let i0 = sommets.len() as u32;
            for coin in [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]] {
                sommets.push(Sommet {
                    orbite: [b.r, b.z, b.ang0, b.omega],
                    decal: [b.decal.x, b.decal.y, b.decal.z, b.taille],
                    couleur: [b.couleur.x, b.couleur.y, b.couleur.z, b.alpha],
                    coin,
                });
            }
            indices.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
        }
        // ⚠️ Indices `u32` : plus de 65 536 sommets. OpenGL de bureau les
        // prend toujours ; un GLES 2 sans `OES_element_index_uint`, non.
        let ctx = unsafe { get_internal_gl() }.quad_context;
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&sommets));
        let ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        Tampons { vb, ib }
    }

    /// Dessine `nombre` boules depuis la `debut`-ième, dans la passe
    /// `cible` de taille `(l, h)` — la cible réduite des nuages.
    pub fn dessiner(&self, cible: RenderPass, (l, h): (i32, i32), u: &Uniformes, (debut, nombre): (usize, usize)) {
        if nombre == 0 {
            return;
        }
        let pipeline = pipeline();
        let halo = crate::impostor::halo_partage().raw_miniquad_id();
        let mut gl = unsafe { get_internal_gl() };
        gl.flush(); // ce que macroquad a en file passe avant (la sphère qui cache)
        let ctx = gl.quad_context;
        ctx.begin_pass(Some(cible), PassAction::Nothing);
        ctx.apply_viewport(0, 0, l, h);
        ctx.apply_pipeline(&pipeline);
        ctx.apply_bindings_from_slice(&[self.vb], self.ib, &[halo]);
        ctx.apply_uniforms(UniformsSource::table(u));
        ctx.draw((debut * 6) as i32, (nombre * 6) as i32, 1);
        ctx.end_render_pass();
    }
}

impl Drop for Tampons {
    fn drop(&mut self) {
        let ctx = unsafe { get_internal_gl() }.quad_context;
        ctx.delete_buffer(self.vb);
        ctx.delete_buffer(self.ib);
    }
}

thread_local! {
    static PIPELINE: RefCell<Option<Pipeline>> = const { RefCell::new(None) };
}

/// Vide le pipeline : le prochain dessin relit les `.glsl` (touche R).
pub(super) fn vider_cache() {
    PIPELINE.with(|p| *p.borrow_mut() = None);
}

fn pipeline() -> Pipeline {
    PIPELINE.with(|p| *p.borrow_mut().get_or_insert_with(construire_pipeline))
}

fn construire_pipeline() -> Pipeline {
    let ctx = unsafe { get_internal_gl() }.quad_context;
    let f3 = |n: &str| UniformDesc::new(n, UniformType::Float3);
    let meta = ShaderMeta {
        images: vec!["Texture".to_string()],
        uniforms: UniformBlockLayout {
            uniforms: vec![
                UniformDesc::new("mvp", UniformType::Mat4),
                f3("centre"),
                f3("axe_u"),
                f3("axe_n"),
                f3("axe_v"),
                f3("cam_right"),
                f3("cam_up"),
                UniformDesc::new("temps", UniformType::Float1),
                UniformDesc::new("amorti", UniformType::Float1),
            ],
        },
    };
    let vert = crate::impostor::source("nuages.vert.glsl", VERT);
    let frag = crate::impostor::source("nuages.frag.glsl", FRAG);
    let shader = ctx
        .new_shader(ShaderSource::Glsl { vertex: &vert, fragment: &frag }, meta)
        .expect("shader des nuages");
    ctx.new_pipeline(
        &[BufferLayout::default()],
        &[
            VertexAttribute::new("orbite", VertexFormat::Float4),
            VertexAttribute::new("decal", VertexFormat::Float4),
            VertexAttribute::new("couleur", VertexFormat::Float4),
            VertexAttribute::new("coin", VertexFormat::Float2),
        ],
        shader,
        // Les réglages du plasma, que les boules empruntaient : additif, la
        // profondeur lue (l'étoile les cache) mais jamais écrite.
        PipelineParams {
            depth_test: Comparison::LessOrEqual,
            depth_write: false,
            color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One)),
            ..Default::default()
        },
    )
}

const VERT: &str = include_str!("../shaders/nuages.vert.glsl");
const FRAG: &str = include_str!("../shaders/nuages.frag.glsl");
