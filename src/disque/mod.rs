//! Champ de débris unifié (ceintures, anneaux planétaires, disques proto*) —
//! voir CONCEPTION_CEINTURES.md. Principe : la géométrie est construite UNE
//! fois à la création (éléments orbitaux encodés dans les canaux de sommet),
//! puis les shaders animent via `time`. Zéro travail CPU par frame en dehors
//! des uniforms.
//!
//! Deux couches :
//! - **particules** : quads billboards, orbite képlérienne dans le vertex
//!   shader. Encodage : `position` = (phi, incl, r) ; `uv` = (coin + graine,
//!   taille) ; `color` = teinte.
//! - **voile** : annulus low-poly statique + fragment shader procédural
//!   (plateau, bandes/lacunes signées, granulation, arcs, émissif, rotation
//!   différentielle). Pour les anneaux planétaires, rendu en deux moitiés
//!   (uniform `moitie` + discard) autour du corps de la planète.

mod config;
mod voile;

pub use config::DisqueConfig;

use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase};
use crate::impostor;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::cell::RefCell;

// Hauteur (px) du viewport de rendu courant : sert au clamp de taille MINIMALE
// des particules (une particule sub-pixel disparaît ; on la fige à ~1.5 px en
// l'assombrissant — Kuiper/Oort restent visibles aux échelles réelles).
// 0 = plein écran. Même convention que planete::set_viewport_h.
thread_local! {
    static VIEWPORT_H: RefCell<f32> = RefCell::new(0.0);
    static PX_MIN: RefCell<f32> = RefCell::new(0.0);
}

/// Hauteur du viewport courant (galerie : hauteur de cellule / facteur pixel).
pub fn set_viewport_h(h: f32) {
    VIEWPORT_H.with(|v| *v.borrow_mut() = h);
}

/// Demi-taille écran minimale d'une particule, en pixels DE RENDU.
/// 0 = défaut (1.5). En mode pixel-art, passer ~1.15 : le plus petit débris
/// vaut alors 1 pixel de la grille au lieu de 1.5 pixel de rendu (= 3 px
/// écran après upscale), qui grossissait les champs de débris.
pub fn set_px_min(p: f32) {
    PX_MIN.with(|v| *v.borrow_mut() = p);
}

fn viewport_h() -> f32 {
    let h = VIEWPORT_H.with(|v| *v.borrow());
    if h > 0.0 {
        h
    } else {
        screen_height()
    }
}

fn px_min() -> f32 {
    let p = PX_MIN.with(|v| *v.borrow());
    if p > 0.0 {
        p
    } else {
        1.5
    }
}

const QUADS_PAR_LOT: usize = 400; // limite éprouvée du batcher macroquad

const VERT_PART: &str = include_str!("../shaders/disque_particules.vert.glsl");
const FRAG_PART: &str = include_str!("../shaders/disque_particules.frag.glsl");

/// Champ de débris : particules et/ou voile, animés par le GPU.
/// Masse nulle -> n'influence pas la gravité.
pub struct Disque {
    base: CorpsBase,
    cfg: DisqueConfig,
    temps: f32,
    // Rayon du corps central qui projette une ombre cylindrique sur le champ
    // (planète annelée). 0 = pas d'ombre (disques stellaires : la lumière
    // vient du centre).
    ombre_rayon: f32,
    // Base orthonormée du plan du disque (axe_n = normale).
    axe_u: Vec3,
    axe_n: Vec3,
    axe_v: Vec3,
    lots: Vec<Mesh>, // particules : buffers figés (≤ 400 quads chacun)
    mat: Option<Material>,
    voile_lots: Vec<Mesh>, // annulus figé (local, plan Y)
    mat_voile: Option<Material>,
}

impl Disque {
    pub fn new(c: DisqueConfig) -> Self {
        // Base du plan : mêmes conventions que l'ex-anneau (normal, u, v).
        let n = c.normale.normalize_or_zero();
        let n = if n == Vec3::ZERO { Vec3::Y } else { n };
        let tmp = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
        let u = n.cross(tmp).normalize();
        let v = n.cross(u);

        let (lots, mat) = if c.nb > 0 {
            (construire_lots(&c), Some(material_particules()))
        } else {
            (Vec::new(), None)
        };
        let (voile_lots, mat_voile) = if c.voile_alpha > 0.0 {
            (voile::construire_annulus(&c), Some(voile::material_voile()))
        } else {
            (Vec::new(), None)
        };

        Self {
            base: CorpsBase::new(Vec3::ZERO, 0.0, 0.0),
            cfg: c,
            temps: 0.0,
            ombre_rayon: 0.0,
            axe_u: u,
            axe_n: n,
            axe_v: v,
            lots,
            mat,
            voile_lots,
            mat_voile,
        }
    }

    /// Rayon du corps central projetant une ombre sur le champ (planète annelée).
    pub fn set_ombre_rayon(&mut self, r: f32) {
        self.ombre_rayon = r;
    }

    /// Phase orbitale (rad) du corps qui creuse la lacune `i` : Kepler au rayon
    /// de la lacune. C'est LE seul couplage CPU par frame (4 sqrt), il anime
    /// l'ondulation des bords ET place le corps embarqué (position_lacune).
    fn phase_lacune(&self, i: usize) -> f32 {
        let l = self.cfg.lacunes[i];
        let r = self.cfg.interne + l.x * (self.cfg.externe - self.cfg.interne);
        let omega = (self.cfg.gm / (r * r * r).max(1e-6)).sqrt();
        omega * self.temps
    }

    /// Position monde du corps embarqué dans la lacune `i` (lune bergère,
    /// proto-planète) : sur le cercle de la lacune, à sa phase orbitale.
    /// `None` si la lacune est inactive ou purement décorative (résonance).
    pub fn position_lacune(&self, i: usize) -> Option<Vec3> {
        let l = *self.cfg.lacunes.get(i)?;
        if l.y <= 0.0 || l.z <= 0.0 {
            return None;
        }
        let r = self.cfg.interne + l.x * (self.cfg.externe - self.cfg.interne);
        let a = self.phase_lacune(i);
        Some(self.base.position + (self.axe_u * a.cos() + self.axe_v * a.sin()) * r)
    }

    /// Dessine une moitié du disque : `moitie` = -1 (arrière, avant le corps),
    /// +1 (avant, après le corps), 0 (tout, en une passe). Les DEUX couches
    /// respectent la moitié (discard GPU) : le rendu peintre
    /// arrière -> corps -> avant reste correct même sans depth buffer
    /// (cible basse résolution du mode pixel).
    pub fn draw_moitie(&mut self, cam: &CameraInfo, moitie: f32) {
        let p = self.base.position;

        {
            if let Some(mat) = &self.mat {
                mat.set_uniform("time", self.temps);
                mat.set_uniform("gm", self.cfg.gm);
                mat.set_uniform("ecc_max", self.cfg.ecc_max);
                mat.set_uniform("centre", (p.x, p.y, p.z));
                mat.set_uniform("axe_u", (self.axe_u.x, self.axe_u.y, self.axe_u.z));
                mat.set_uniform("axe_n", (self.axe_n.x, self.axe_n.y, self.axe_n.z));
                mat.set_uniform("axe_v", (self.axe_v.x, self.axe_v.y, self.axe_v.z));
                mat.set_uniform("cam_right", (cam.right.x, cam.right.y, cam.right.z));
                mat.set_uniform("cam_up", (cam.up.x, cam.up.y, cam.up.z));
                mat.set_uniform("viewport_h", viewport_h());
                mat.set_uniform("px_min", px_min());
                mat.set_uniform("light_pos", (cam.light_pos.x, cam.light_pos.y, cam.light_pos.z));
                mat.set_uniform("cam_pos", (cam.pos.x, cam.pos.y, cam.pos.z));
                mat.set_uniform("ombre_rayon", self.ombre_rayon);
                mat.set_uniform("moitie", moitie);
                gl_use_material(mat);
                for lot in &self.lots {
                    draw_mesh(lot);
                }
                gl_use_default_material();
            }
        }

        if let Some(mat) = &self.mat_voile {
            let tocam = (cam.pos - p).normalize_or_zero();
            let phases = vec4(
                self.phase_lacune(0),
                self.phase_lacune(1),
                self.phase_lacune(2),
                self.phase_lacune(3),
            );
            // Éclairage : direction de propagation de la lumière au disque, et
            // face vue (éclairée si caméra et lumière sont du même côté du
            // plan ; assombrie sinon — transition douce près de la tranche).
            let dir_lum = (p - cam.light_pos).normalize_or_zero();
            let cote_lum = self.axe_n.dot((cam.light_pos - p).normalize_or_zero());
            let cote_cam = self.axe_n.dot(tocam);
            let f = (cote_lum * cote_cam / 0.05).clamp(-1.0, 1.0); // lissage ±0.05
            let face_lum = 0.775 + 0.225 * f; // 1.0 face éclairée -> 0.55 face nuit
            voile::uniforms_voile(
                mat, &self.cfg, self.temps, p,
                (self.axe_u, self.axe_n, self.axe_v),
                tocam, moitie, phases,
                dir_lum, face_lum, self.ombre_rayon, cam.light_color,
            );
            gl_use_material(mat);
            for lot in &self.voile_lots {
                draw_mesh(lot);
            }
            gl_use_default_material();
        }
    }
}

impl Astre for Disque {
    fn categorie(&self) -> Categorie {
        Categorie::Asteroide
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn update(&mut self, dt: f32) {
        self.temps += dt; // seule mutation par frame : le GPU fait le reste
    }
    fn draw(&mut self, cam: &CameraInfo) {
        self.draw_moitie(cam, 0.0);
    }
}

/// Tire les particules selon la config et fige les buffers, par lots.
fn construire_lots(c: &DisqueConfig) -> Vec<Mesh> {
    // Centres d'amas (clumping) : quelques familles de collision.
    let n_amas = 6;
    let amas: Vec<(f32, f32)> = (0..n_amas)
        .map(|_| {
            (
                gen_range(0.0, std::f32::consts::TAU),           // angle de l'amas
                gen_range(c.interne, c.externe),                 // rayon de l'amas
            )
        })
        .collect();

    // Annelets (sous-anneaux fins) : quelques rayons privilégiés, par graine.
    let n_ringlets = 5;
    let ringlets: Vec<f32> = (0..n_ringlets)
        .map(|_| gen_range(c.interne, c.externe))
        .collect();

    let mut lots: Vec<Mesh> = Vec::new();
    let mut verts: Vec<Vertex> = Vec::with_capacity(QUADS_PAR_LOT * 4);
    let mut inds: Vec<u16> = Vec::with_capacity(QUADS_PAR_LOT * 6);

    for _ in 0..c.nb {
        // ---- Rayon : profil radial (ou annelet) + rejet dans les lacunes ----
        let mut r = 0.0_f32;
        for _ in 0..8 {
            r = if c.ringlets > 0.0 && gen_range(0.0_f32, 1.0) < c.ringlets {
                // Resserré sur un annelet fin (~0.5 % de l'étendue radiale).
                let rj = ringlets[gen_range(0, n_ringlets as i32) as usize % n_ringlets];
                (rj + gen_range(-0.006, 0.006) * (c.externe - c.interne))
                    .clamp(c.interne, c.externe)
            } else {
                let u: f32 = gen_range(0.0_f32, 1.0).powf(c.profil_radial);
                c.interne + u * (c.externe - c.interne)
            };
            let t = (r - c.interne) / (c.externe - c.interne).max(1e-6);
            let dans_lacune = c.lacunes.iter().any(|l| {
                l.y > 0.0 && l.z > 0.0 && (t - l.x).abs() < l.y && gen_range(0.0_f32, 1.0) < l.z
            });
            if !dans_lacune {
                break;
            }
        }

        // ---- Plan orbital : disque (± epaisseur) -> coquille (spherite) ----
        let incl_disque: f32 = gen_range(-c.epaisseur, c.epaisseur);
        let incl_sphere: f32 = gen_range(-1.0_f32, 1.0).asin();
        let mut incl = incl_disque * (1.0 - c.spherite) + incl_sphere * c.spherite;
        let phi: f32 = gen_range(0.0, std::f32::consts::TAU);

        // ---- Clumping : une partie des particules se resserre sur un amas.
        let mut phase = gen_range(0.0_f32, 1.0); // graine -> angle0 dans le shader
        if c.clumping > 0.0 && gen_range(0.0_f32, 1.0) < c.clumping {
            let (ang_a, r_a) = amas[gen_range(0, n_amas as i32) as usize % n_amas];
            let serr = 0.05 + 0.10 * gen_range(0.0_f32, 1.0);
            r = (r_a + gen_range(-serr, serr) * (c.externe - c.interne))
                .clamp(c.interne, c.externe);
            // Angle MONDE d'une particule à t=0 : phi + theta0. Pour grouper
            // l'amas en ang_a, la phase compense donc phi. La dispersion
            // radiale + Kepler étirent ensuite l'amas en arc (réaliste).
            phase = ((ang_a - phi) / std::f32::consts::TAU + gen_range(-0.02, 0.02))
                .rem_euclid(1.0);
            incl *= 0.4; // les amas restent groupés verticalement
        }

        // ---- Taille : essaim de petits (biais u⁴) + GROS blocs (bimodal) ----
        let gros = c.bimodal > 0.0 && gen_range(0.0_f32, 1.0) < c.bimodal;
        let taille = if gros {
            c.taille_max * gen_range(0.8, 1.8)
        } else {
            let u: f32 = gen_range(0.0, 1.0);
            c.taille_min + u * u * u * u * (c.taille_max - c.taille_min)
        };
        // Irrégularité de forme (alpha du sommet, canal libre en pipeline
        // opaque) : les gros fragments sont plus anguleux que les cailloux.
        let irr: f32 = if gros {
            gen_range(0.55, 0.95)
        } else {
            gen_range(0.25, 0.5)
        };

        // ---- Couleur : gradient radial interne->externe + variation ----
        let t = (r - c.interne) / (c.externe - c.interne).max(1e-6);
        let g: f32 = gen_range(0.7, 1.05);
        let teinte = (c.couleur2 * (1.0 - t) + c.couleur * t) * g;
        let col = Color::new(
            teinte.x.clamp(0.0, 1.0),
            teinte.y.clamp(0.0, 1.0),
            teinte.z.clamp(0.0, 1.0),
            irr, // hijack : irrégularité, PAS de l'alpha (particules opaques)
        );

        // ---- 4 sommets : mêmes éléments orbitaux, coin 0..3 dans uv.x ----
        let graine = phase.min(0.9995); // fraction stricte (< 1) pour ne pas mordre sur le coin
        let i0 = verts.len() as u16;
        for coin in 0..4u16 {
            verts.push(Vertex::new2(
                vec3(phi, incl, r),
                vec2(coin as f32 + graine, taille),
                col,
            ));
        }
        inds.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);

        if verts.len() >= QUADS_PAR_LOT * 4 {
            lots.push(Mesh {
                vertices: std::mem::take(&mut verts),
                indices: std::mem::take(&mut inds),
                texture: None,
            });
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

/// Material des particules : sommets orbitaux animés par le vertex shader,
/// rendu opaque (depth write, pas de tri nécessaire). Hot-reload via R.
fn material_particules() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("disque_particules.vert.glsl", VERT_PART),
            fragment: &impostor::source("disque_particules.frag.glsl", FRAG_PART),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("time", UniformType::Float1),
                UniformDesc::new("gm", UniformType::Float1),
                UniformDesc::new("ecc_max", UniformType::Float1),
                UniformDesc::new("centre", UniformType::Float3),
                UniformDesc::new("axe_u", UniformType::Float3),
                UniformDesc::new("axe_n", UniformType::Float3),
                UniformDesc::new("axe_v", UniformType::Float3),
                UniformDesc::new("cam_right", UniformType::Float3),
                UniformDesc::new("cam_up", UniformType::Float3),
                UniformDesc::new("viewport_h", UniformType::Float1),
                UniformDesc::new("px_min", UniformType::Float1),
                UniformDesc::new("light_pos", UniformType::Float3),
                UniformDesc::new("cam_pos", UniformType::Float3),
                UniformDesc::new("ombre_rayon", UniformType::Float1),
                UniformDesc::new("moitie", UniformType::Float1),
            ],
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
