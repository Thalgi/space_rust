use crate::astre::CameraInfo;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

const RAYON: f32 = 3000.0; // distance des étoiles (sous le far plane, au-delà des planètes)
const LOT: usize = 400;

/// Champ d'étoiles lointaines : billboards sur une sphère centrée sur la caméra
/// (on regarde depuis l'intérieur). Rendues sans test de profondeur, en premier,
/// donc toujours en arrière-plan quelle que soit la distance des objets.
pub struct Fond {
    etoiles: Vec<(Vec3, f32, Color)>, // direction, taille, couleur
    mat: Material,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
}

impl Fond {
    pub fn new(n: usize) -> Self {
        let mut etoiles = Vec::with_capacity(n);
        for _ in 0..n {
            let z: f32 = gen_range(-1.0, 1.0);
            let a: f32 = gen_range(0.0, std::f32::consts::TAU);
            let r = (1.0 - z * z).sqrt();
            let dir = vec3(r * a.cos(), z, r * a.sin());
            let lum: f32 = gen_range(0.35, 1.0);
            let t: f32 = gen_range(0.0, 1.0);
            let col = Color::new(lum * (0.85 + 0.15 * t), lum * 0.95, lum * (1.0 - 0.15 * t), 1.0);
            let taille: f32 = gen_range(1.2, 4.0);
            etoiles.push((dir, taille, col));
        }
        Self {
            etoiles,
            mat: material_etoiles(),
            verts: Vec::new(),
            inds: Vec::new(),
        }
    }

    /// Recharge le material depuis les .glsl (hot-reload) sans toucher au champ d'étoiles.
    pub fn recharger_material(&mut self) {
        self.mat = material_etoiles();
    }

    pub fn draw(&mut self, cam: &CameraInfo) {
        self.verts.clear();
        self.inds.clear();
        let echelle = RAYON * 0.0004; // petites étoiles (points)
        gl_use_material(&self.mat);
        let mut q = 0;
        for (dir, taille, col) in &self.etoiles {
            let pos = cam.pos + *dir * RAYON;
            let s = *taille * echelle;
            let r2 = cam.right * s;
            let u2 = cam.up * s;
            let i0 = self.verts.len() as u16;
            self.verts.push(Vertex::new2(pos - r2 - u2, vec2(0.0, 0.0), *col));
            self.verts.push(Vertex::new2(pos + r2 - u2, vec2(1.0, 0.0), *col));
            self.verts.push(Vertex::new2(pos + r2 + u2, vec2(1.0, 1.0), *col));
            self.verts.push(Vertex::new2(pos - r2 + u2, vec2(0.0, 1.0), *col));
            self.inds
                .extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
            q += 1;
            if q >= LOT {
                flush(&mut self.verts, &mut self.inds);
                q = 0;
            }
        }
        flush(&mut self.verts, &mut self.inds);
        gl_use_default_material();
    }
}

fn material_etoiles() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &crate::impostor::source("fond.vert.glsl", VERT),
            fragment: &crate::impostor::source("fond.frag.glsl", FRAG),
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::Always, // toujours peint -> arrière-plan
                depth_write: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

fn flush(verts: &mut Vec<Vertex>, inds: &mut Vec<u16>) {
    if inds.is_empty() {
        return;
    }
    let mesh = Mesh {
        vertices: std::mem::take(verts),
        indices: std::mem::take(inds),
        texture: None,
    };
    draw_mesh(&mesh);
    let mut v = mesh.vertices;
    v.clear();
    let mut i = mesh.indices;
    i.clear();
    *verts = v;
    *inds = i;
}

const VERT: &str = include_str!("shaders/fond.vert.glsl");

const FRAG: &str = include_str!("shaders/fond.frag.glsl");
