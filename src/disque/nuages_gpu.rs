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
//!
//! **Le flot bipolaire** (les jets, O7c) suit la même recette avec son propre
//! shader, `flot.vert.glsl`, copie de `flot::animer` : chaque boule y coule le
//! long d'une trajectoire fixe.

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

/// **Le flot bipolaire, figé** (Opti-v2 O7c) : ses boules fixes, le côté du
/// dessus d'abord (`flot::boules_fixes`), et combien il y en a dessus. Le
/// côté du plan d'une boule est son lobe.
pub(super) struct FlotFixe {
    pub boules: Vec<super::flot::BouleFixe>,
    pub dessus: usize,
}

impl FlotFixe {
    pub fn new(f: &super::flot::Flot) -> Self {
        let boules = super::flot::boules_fixes(f);
        let dessus = boules.iter().filter(|b| b.cote > 0.0).count();
        FlotFixe { boules, dessus }
    }

    /// La tranche `(début, nombre)` d'un côté du plan.
    pub fn tranche(&self, dessus: bool) -> (usize, usize) {
        if dessus {
            (0, self.dessus)
        } else {
            (self.dessus, self.boules.len() - self.dessus)
        }
    }
}

#[repr(C)]
struct SommetFlot {
    boule: [f32; 4],
    tirage: [f32; 4],
    coin: [f32; 2],
}

#[repr(C)]
pub(super) struct UniformesFlot {
    pub mvp: Mat4,
    pub centre: [f32; 3],
    pub axe_u: [f32; 3],
    pub axe_n: [f32; 3],
    pub axe_v: [f32; 3],
    pub cam_right: [f32; 3],
    pub cam_up: [f32; 3],
    pub temps: f32,
    pub lobe_haut: [f32; 4],
    pub lobe_bas: [f32; 4],
    pub parois: [f32; 2],
    pub flot: [f32; 4],
    pub longueur: f32,
}

impl UniformesFlot {
    /// Ce que le shader lit du flot `f` : ses deux lobes (`flot::lobe`).
    pub fn lobes(f: &super::flot::Flot) -> ([f32; 4], [f32; 4], [f32; 2], [f32; 4], f32) {
        let (h, b) = (super::flot::lobe(f, 1.0), super::flot::lobe(f, -1.0));
        (
            [h.longueur, h.ouverture, h.jet, h.cone],
            [b.longueur, b.ouverture, b.jet, b.cone],
            [h.parois, b.parois],
            [f.rayon_astre, f.finesse, f.asymetrie, f.forme],
            f.longueur,
        )
    }
}

const COINS: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];

/// Les buffers GPU de quads, construits une fois.
pub(super) struct Tampons {
    vb: BufferId,
    ib: BufferId,
}

impl Tampons {
    /// Les boules des nuages.
    pub fn nuages(f: &Fixes) -> Self {
        let sommets: Vec<Sommet> = f
            .boules
            .iter()
            .flat_map(|b| {
                COINS.map(|coin| Sommet {
                    orbite: [b.r, b.z, b.ang0, b.omega],
                    decal: [b.decal.x, b.decal.y, b.decal.z, b.taille],
                    couleur: [b.couleur.x, b.couleur.y, b.couleur.z, b.alpha],
                    coin,
                })
            })
            .collect();
        Self::de_quads(&sommets)
    }

    /// Les boules du flot.
    pub fn flot(f: &FlotFixe) -> Self {
        use super::flot::Famille;
        let sommets: Vec<SommetFlot> = f
            .boules
            .iter()
            .flat_map(|b| {
                let famille = match b.famille {
                    Famille::Jet => 0.0,
                    Famille::Cone => 1.0,
                    Famille::Noeud => 2.0,
                    Famille::Paroi => 3.0,
                };
                COINS.map(|coin| SommetFlot { boule: [famille, b.cote, b.s0, 0.0], tirage: [b.k[0], b.k[1], b.k[2], 0.0], coin })
            })
            .collect();
        Self::de_quads(&sommets)
    }

    /// Quatre sommets par quad, dans l'ordre de `COINS`.
    fn de_quads<T>(sommets: &[T]) -> Self {
        // ⚠️ Indices `u32` : plus de 65 536 sommets. OpenGL de bureau les
        // prend toujours ; un GLES 2 sans `OES_element_index_uint`, non.
        let indices: Vec<u32> = (0..sommets.len() as u32 / 4)
            .flat_map(|q| {
                let i0 = q * 4;
                [i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]
            })
            .collect();
        let ctx = unsafe { get_internal_gl() }.quad_context;
        let vb = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(sommets));
        let ib = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        Tampons { vb, ib }
    }

    /// Dessine `nombre` quads depuis le `debut`-ième avec `pipeline`, dans
    /// la passe `cible` de taille `(l, h)` — la cible réduite des nuages.
    pub fn dessiner<U>(
        &self,
        pipeline: Pipeline,
        cible: RenderPass,
        (l, h): (i32, i32),
        u: &U,
        (debut, nombre): (usize, usize),
    ) {
        if nombre == 0 {
            return;
        }
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
    static PIPELINES: RefCell<[Option<Pipeline>; 2]> = const { RefCell::new([None, None]) };
}

/// Vide les pipelines : le prochain dessin relit les `.glsl` (touche R).
pub(super) fn vider_cache() {
    PIPELINES.with(|p| *p.borrow_mut() = [None, None]);
}

/// Les uniforms communs aux deux shaders, dans l'ordre des `Uniformes`.
fn uniforms_communs() -> Vec<UniformDesc> {
    let f3 = |n: &str| UniformDesc::new(n, UniformType::Float3);
    vec![
        UniformDesc::new("mvp", UniformType::Mat4),
        f3("centre"),
        f3("axe_u"),
        f3("axe_n"),
        f3("axe_v"),
        f3("cam_right"),
        f3("cam_up"),
        UniformDesc::new("temps", UniformType::Float1),
    ]
}

/// Le pipeline des nuages.
pub(super) fn pipeline_nuages() -> Pipeline {
    PIPELINES.with(|p| {
        *p.borrow_mut()[0].get_or_insert_with(|| {
            let mut u = uniforms_communs();
            u.push(UniformDesc::new("amorti", UniformType::Float1));
            let attributs = [
                VertexAttribute::new("orbite", VertexFormat::Float4),
                VertexAttribute::new("decal", VertexFormat::Float4),
                VertexAttribute::new("couleur", VertexFormat::Float4),
                VertexAttribute::new("coin", VertexFormat::Float2),
            ];
            construire_pipeline("nuages.vert.glsl", VERT, u, &attributs)
        })
    })
}

/// Le pipeline du flot.
pub(super) fn pipeline_flot() -> Pipeline {
    PIPELINES.with(|p| {
        *p.borrow_mut()[1].get_or_insert_with(|| {
            let mut u = uniforms_communs();
            u.extend([
                UniformDesc::new("lobe_haut", UniformType::Float4),
                UniformDesc::new("lobe_bas", UniformType::Float4),
                UniformDesc::new("parois", UniformType::Float2),
                UniformDesc::new("flot", UniformType::Float4),
                UniformDesc::new("longueur", UniformType::Float1),
            ]);
            let attributs = [
                VertexAttribute::new("boule", VertexFormat::Float4),
                VertexAttribute::new("tirage", VertexFormat::Float4),
                VertexAttribute::new("coin", VertexFormat::Float2),
            ];
            construire_pipeline("flot.vert.glsl", VERT_FLOT, u, &attributs)
        })
    })
}

fn construire_pipeline(nom: &str, defaut: &'static str, uniforms: Vec<UniformDesc>, attributs: &[VertexAttribute]) -> Pipeline {
    let ctx = unsafe { get_internal_gl() }.quad_context;
    let meta = ShaderMeta { images: vec!["Texture".to_string()], uniforms: UniformBlockLayout { uniforms } };
    let vert = crate::impostor::source(nom, defaut);
    let frag = crate::impostor::source("nuages.frag.glsl", FRAG);
    let shader = ctx
        .new_shader(ShaderSource::Glsl { vertex: &vert, fragment: &frag }, meta)
        .unwrap_or_else(|e| panic!("shader {nom} : {e:?}"));
    ctx.new_pipeline(
        &[BufferLayout::default()],
        attributs,
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
const VERT_FLOT: &str = include_str!("../shaders/flot.vert.glsl");
const FRAG: &str = include_str!("../shaders/nuages.frag.glsl");
