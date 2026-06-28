mod config;
mod rendu;

pub use config::CeintureConfig;

use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase};
use crate::systeme::G;
use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Un astéroïde sur une orbite circulaire inclinée, parcourue analytiquement.
struct Asteroide {
    a1: Vec3,
    q: Vec3,
    r: f32,
    angle: f32,
    omega: f32,
    coins: [Vec2; 4], // taille + forme irrégulière précalculées (plan caméra)
    couleur: Vec3,
}

/// Une ceinture : beaucoup de petits corps en orbites analytiques indépendantes.
/// Masse nulle -> n'influence pas la gravité. Rendue en un minimum de draw calls.
pub struct Ceinture {
    base: CorpsBase,
    items: Vec<Asteroide>,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
}

impl Ceinture {
    pub fn new(c: CeintureConfig) -> Self {
        let mut items = Vec::with_capacity(c.nb);
        for _ in 0..c.nb {
            let r: f32 = gen_range(c.interne, c.externe);
            let incl: f32 = gen_range(-c.epaisseur, c.epaisseur);
            let phi: f32 = gen_range(0.0, std::f32::consts::TAU);
            let a1 = vec3(phi.cos(), 0.0, phi.sin());
            let a2 = vec3(-phi.sin(), 0.0, phi.cos());
            let q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();
            let omega = (G * c.masse / (r * r * r)).sqrt();

            // Taille : fort biais vers le petit (u^4) -> gros corps rares.
            let u: f32 = gen_range(0.0, 1.0);
            let taille = c.taille_min + u * u * u * u * (c.taille_max - c.taille_min);
            // Coins irréguliers (formes de cailloux, surtout pour les gros).
            let jit = taille * 0.4;
            let bases = [(-1.0_f32, -1.0_f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
            let mut coins = [Vec2::ZERO; 4];
            for k in 0..4 {
                let (bx, by) = bases[k];
                coins[k] = vec2(
                    bx * taille + gen_range(-jit, jit),
                    by * taille + gen_range(-jit, jit),
                );
            }

            let g: f32 = gen_range(0.7, 1.05);
            items.push(Asteroide {
                a1,
                q,
                r,
                angle: gen_range(0.0, std::f32::consts::TAU),
                omega,
                coins,
                couleur: c.couleur * g,
            });
        }
        Self {
            base: CorpsBase::new(Vec3::ZERO, 0.0, 0.0),
            items,
            verts: Vec::new(),
            inds: Vec::new(),
        }
    }
}

impl Astre for Ceinture {
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
        for a in &mut self.items {
            a.angle += a.omega * dt;
        }
    }
    fn draw(&mut self, cam: &CameraInfo) {
        self.dessiner(cam);
    }
}
