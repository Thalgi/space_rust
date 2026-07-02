use super::apparence::Apparence;
use crate::astre::CameraInfo;
use crate::impostor;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::cell::RefCell;

/// Demi-étendue du disque impostor planète (marge sur le rayon pour l'atmosphère).
const DISC: f32 = 1.05;

/// Une valeur d'uniform : on n'a besoin que de scalaires et de vec3.
enum Val {
    F1(f32),
    F3(Vec3),
}

/// Table déclarative des uniforms pilotés par l'`Apparence`. Source unique : sert
/// à la fois à déclarer les uniforms (descripteurs) et à les pousser chaque frame.
/// Ajouter un paramètre visuel = une seule ligne ici (puis l'utiliser dans le .glsl).
const TABLE: &[(&str, UniformType, fn(&Apparence) -> Val)] = &[
    ("couleur", UniformType::Float3, |a| Val::F3(a.couleur)),
    ("couleur2", UniformType::Float3, |a| Val::F3(a.couleur2)),
    ("couleur3", UniformType::Float3, |a| Val::F3(a.couleur3)),
    ("type_p", UniformType::Float1, |a| Val::F1(a.type_p.code())),
    ("eau", UniformType::Float1, |a| Val::F1(a.eau)),
    ("tache_dir", UniformType::Float3, |a| Val::F3(a.tache_dir)),
    ("tache_w", UniformType::Float1, |a| Val::F1(a.tache_taille)),
    ("tache_col", UniformType::Float3, |a| Val::F3(a.tache_couleur)),
    ("tache_type", UniformType::Float1, |a| Val::F1(a.tache_type)),
    ("axe", UniformType::Float3, |a| Val::F3(a.axe)),
    ("band_scale", UniformType::Float1, |a| Val::F1(a.band_scale)),
    ("warp_amt", UniformType::Float1, |a| Val::F1(a.warp_amt)),
    ("seed", UniformType::Float1, |a| Val::F1(a.seed)),
    ("poly_cotes", UniformType::Float1, |a| Val::F1(a.poly_cotes)),
    ("atmo", UniformType::Float3, |a| Val::F3(a.atmo)),
    ("lave", UniformType::Float1, |a| Val::F1(a.lave)),
    ("eau_motif", UniformType::Float1, |a| Val::F1(a.eau_motif)),
    ("grad_lat", UniformType::Float1, |a| Val::F1(a.grad_lat)),
    ("calotte", UniformType::Float1, |a| Val::F1(a.calotte)),
    ("veg_couleur", UniformType::Float3, |a| Val::F3(a.veg_couleur)),
    ("veg_couv", UniformType::Float1, |a| Val::F1(a.veg_couv)),
    ("rivieres", UniformType::Float1, |a| Val::F1(a.rivieres)),
    ("nuages", UniformType::Float1, |a| Val::F1(a.nuages)),
    ("nuages_couleur", UniformType::Float3, |a| Val::F3(a.nuages_couleur)),
    ("nuages_type", UniformType::Float1, |a| Val::F1(a.nuages_type)),
    ("relief", UniformType::Float1, |a| Val::F1(a.relief)),
    ("dunes", UniformType::Float1, |a| Val::F1(a.dunes)),
    ("mesa", UniformType::Float1, |a| Val::F1(a.mesa)),
    ("pics", UniformType::Float1, |a| Val::F1(a.pics)),
    ("recifs", UniformType::Float1, |a| Val::F1(a.recifs)),
    ("basalt", UniformType::Float1, |a| Val::F1(a.basalt)),
    ("voile", UniformType::Float1, |a| Val::F1(a.voile)),
    ("voile_couleur", UniformType::Float3, |a| Val::F3(a.voile_couleur)),
    ("crateres", UniformType::Float1, |a| Val::F1(a.crateres)),
    ("eyeball", UniformType::Float1, |a| Val::F1(a.eyeball)),
    ("eye_glace", UniformType::Float1, |a| Val::F1(a.eye_glace)),
    ("eye_lave", UniformType::Float1, |a| Val::F1(a.eye_lave)),
    ("eye_ring", UniformType::Float1, |a| Val::F1(a.eye_ring)),
    ("cryo", UniformType::Float1, |a| Val::F1(a.cryo)),
    ("biolum", UniformType::Float1, |a| Val::F1(a.biolum)),
    ("riv_lave", UniformType::Float1, |a| Val::F1(a.riv_lave)),
    ("villes", UniformType::Float1, |a| Val::F1(a.villes)),
    ("cyclones_pol", UniformType::Float1, |a| Val::F1(a.cyclones_pol)),
    ("thermique", UniformType::Float1, |a| Val::F1(a.thermique)),
    ("thermique_couleur", UniformType::Float3, |a| Val::F3(a.thermique_couleur)),
    ("tempetes", UniformType::Float1, |a| Val::F1(a.tempetes)),
    ("aurore", UniformType::Float1, |a| Val::F1(a.aurore)),
    ("aurore_couleur", UniformType::Float3, |a| Val::F3(a.aurore_couleur)),
    ("brume", UniformType::Float1, |a| Val::F1(a.brume)),
    ("brume_couleur", UniformType::Float3, |a| Val::F3(a.brume_couleur)),
    ("g_pole", UniformType::Float3, |a| Val::F3(a.g_pole)),
    ("jet_profil", UniformType::Float1, |a| Val::F1(a.jet_profil)),
];

fn set_val(mat: &Material, nom: &str, v: Val) {
    match v {
        Val::F1(x) => mat.set_uniform(nom, x),
        Val::F3(p) => mat.set_uniform(nom, (p.x, p.y, p.z)),
    }
}

// Materials partagés (créés une seule fois, clonés ensuite) : un clone partage le
// pipeline GPU mais a ses propres uniforms -> évite « Pipelines amount exceeded ».
thread_local! {
    static TPL_CORPS: RefCell<Option<Material>> = RefCell::new(None);
    static TPL_ANNEAU: RefCell<Option<Material>> = RefCell::new(None);
}

pub(super) fn mat_corps() -> Material {
    TPL_CORPS.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_corps());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

pub(super) fn mat_anneau() -> Material {
    TPL_ANNEAU.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(material_anneau());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// Vide le cache de materials -> la prochaine création recompile depuis les .glsl
/// (utilisé par le hot-reload).
pub(super) fn vider_cache() {
    TPL_CORPS.with(|c| *c.borrow_mut() = None);
    TPL_ANNEAU.with(|c| *c.borrow_mut() = None);
}

// Texture de secours (1×1) : corps sans terrain précalculé (gazeuses, glacées)
// et telluriques dont la génération asynchrone n'a pas fini (placeholder :
// h ≈ 0.55 -> sol plat uni, ni océan global ni noir).
thread_local! {
    static TEX_VIDE: RefCell<Option<Texture2D>> = RefCell::new(None);
}

fn tex_vide() -> Texture2D {
    TEX_VIDE.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(Texture2D::from_rgba8(1, 1, &[140, 0, 0, 128]));
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// Pousse tous les uniforms : communs (socle) + dynamiques (caméra/corps) + table.
/// `terrain` : atlas cube-sphere précalculé + niveau de mer (telluriques).
pub(super) fn appliquer_uniforms(
    mat: &Material,
    a: &Apparence,
    cam: &CameraInfo,
    c: Vec3,
    r: f32,
    terrain: Option<(&Texture2D, f32)>,
) {
    impostor::uniforms_standard(mat, cam, get_time() as f32, DISC);
    mat.set_uniform("centre", (c.x, c.y, c.z));
    mat.set_uniform("rayon", r);
    mat.set_uniform("lumiere", (cam.light_pos.x, cam.light_pos.y, cam.light_pos.z));
    mat.set_uniform(
        "light_color",
        (cam.light_color.x, cam.light_color.y, cam.light_color.z),
    );
    match terrain {
        Some((tex, niveau)) => {
            mat.set_texture("terrain", tex.clone());
            mat.set_uniform("niveau_mer", niveau);
            mat.set_uniform("atlas_n", super::terrain::N_ATLAS as f32);
        }
        None => {
            mat.set_texture("terrain", tex_vide());
            mat.set_uniform("niveau_mer", 0.5f32);
            mat.set_uniform("atlas_n", 1.0f32);
        }
    }
    for (nom, _, get) in TABLE {
        set_val(mat, nom, get(a));
    }
}

/// Crée le material du corps des planètes (appelé une seule fois, puis cloné).
fn charger_corps() -> Material {
    let mut uniforms = impostor::uniforms_communs();
    uniforms.extend([
        UniformDesc::new("centre", UniformType::Float3),
        UniformDesc::new("rayon", UniformType::Float1),
        UniformDesc::new("lumiere", UniformType::Float3),
        UniformDesc::new("light_color", UniformType::Float3),
        UniformDesc::new("niveau_mer", UniformType::Float1),
        UniformDesc::new("atlas_n", UniformType::Float1),
    ]);
    uniforms.extend(TABLE.iter().map(|(nom, t, _)| UniformDesc::new(nom, *t)));
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR),
            fragment: &impostor::source("planete.frag.glsl", FRAG),
        },
        MaterialParams {
            uniforms,
            textures: vec!["terrain".to_string()],
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

/// Material de l'anneau : couleur du sommet, alpha blend, sans écriture de profondeur.
fn material_anneau() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("planete_anneau.vert.glsl", VERT_ANNEAU),
            fragment: &impostor::source("planete_anneau.frag.glsl", FRAG_ANNEAU),
        },
        MaterialParams {
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

const FRAG: &str = include_str!("../shaders/planete.frag.glsl");
const VERT_ANNEAU: &str = include_str!("../shaders/planete_anneau.vert.glsl");
const FRAG_ANNEAU: &str = include_str!("../shaders/planete_anneau.frag.glsl");
