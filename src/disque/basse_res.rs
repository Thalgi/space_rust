//! **Prototype Opti-v2 O4 : les nuages en basse résolution** (dette D6).
//!
//! Les nuages et les bras d'une T Tauri repeignent ~100 écrans par image : des
//! milliers de boules floues qui se chevauchent. Elles n'ont pas de détail
//! fin — les dessiner dans une cible réduite de `k` par axe divise leur
//! remplissage par `k²`. Leur matériau est purement additif (One, One) :
//! accumulées à part puis ajoutées à l'image, elles donnent la même somme.
//!
//! La cible a sa propre profondeur : une sphère **noire** à la place de
//! l'étoile y cache les nuages qui passent derrière elle — en additif, le noir
//! n'ajoute rien à l'image.

use macroquad::miniquad::{BlendFactor, BlendState, Comparison, Equation, PipelineParams};
use macroquad::prelude::*;
use std::cell::RefCell;

thread_local! {
    static CIBLE: RefCell<Option<RenderTarget>> = const { RefCell::new(None) };
    static COMPOSE: RefCell<Option<Material>> = const { RefCell::new(None) };
}

/// La caméra de l'image courante, redirigée vers la cible réduite.
struct Reduite {
    matrice: Mat4,
    cible: RenderTarget,
}

impl Camera for Reduite {
    fn matrix(&self) -> Mat4 {
        self.matrice
    }
    fn depth_enabled(&self) -> bool {
        true
    }
    fn render_pass(&self) -> Option<RenderPass> {
        Some(self.cible.render_pass.clone())
    }
    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        None
    }
}

/// Bascule le dessin vers la cible réduite de `k` (vidée), avec la matrice
/// de l'image courante. Rend cette matrice, à passer à [`fermer`].
pub(super) fn ouvrir(k: u32, viewport_h: f32) -> Mat4 {
    let h = (viewport_h as u32 / k).max(2);
    let l = ((viewport_h * screen_width() / screen_height().max(1.0)) as u32 / k).max(2);
    let cible = CIBLE.with(|c| {
        let mut c = c.borrow_mut();
        if c.as_ref().is_none_or(|rt| rt.texture.size() != vec2(l as f32, h as f32)) {
            let rt = render_target_ex(l, h, RenderTargetParams { sample_count: 1, depth: true });
            rt.texture.set_filter(FilterMode::Linear);
            *c = Some(rt);
        }
        c.clone().unwrap()
    });
    let matrice = get_context_matrice();
    push_camera_state();
    set_camera(&Reduite { matrice, cible });
    clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
    matrice
}

fn get_context_matrice() -> Mat4 {
    unsafe { get_internal_gl() }.quad_gl.get_projection_matrix()
}

/// Revient à l'image et y ajoute la cible : un quad qui couvre l'écran,
/// posé par la matrice inverse.
pub(super) fn fermer(matrice: Mat4) {
    pop_camera_state();
    let inv = matrice.inverse();
    let coin = |x: f32, y: f32| {
        let p = inv * vec4(x, y, 0.0, 1.0);
        Vertex::new2(p.truncate() / p.w, vec2((x + 1.0) * 0.5, (y + 1.0) * 0.5), WHITE)
    };
    let texture = CIBLE.with(|c| c.borrow().as_ref().map(|rt| rt.texture.clone()));
    gl_use_material(&compose());
    draw_mesh(&Mesh {
        vertices: vec![coin(-1.0, -1.0), coin(1.0, -1.0), coin(1.0, 1.0), coin(-1.0, 1.0)],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture,
    });
    gl_use_default_material();
}

fn compose() -> Material {
    COMPOSE.with(|c| {
        c.borrow_mut()
            .get_or_insert_with(|| {
                load_material(
                    ShaderSource::Glsl { vertex: VERT, fragment: FRAG },
                    MaterialParams {
                        pipeline_params: PipelineParams {
                            depth_test: Comparison::Always,
                            depth_write: false,
                            color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .clone()
    })
}

const VERT: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying mediump vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    uv = texcoord;
    gl_Position = Projection * Model * vec4(position, 1.0);
}";

const FRAG: &str = "#version 100
varying mediump vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = texture2D(Texture, uv);
}";
