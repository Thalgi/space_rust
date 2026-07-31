//! Rendu des **panaches d'antimatière** : rubans face-caméra en additif.
//!
//! Pourquoi pas de la géométrie, comme tout le reste du vaisseau : un jet de
//! plasma n'a **pas de silhouette**. Le premier jet était une pile de cônes
//! pleins, et ce sont précisément les qualités d'un solide — arête nette, face
//! opaque, bord franc sur le fond étoilé — qui le faisaient lire comme un tube
//! de plastique planté dans la tuyère.
//!
//! On reprend donc le procédé qui marche déjà pour les **jets bipolaires de
//! pulsar** (`shaders/soleil.frag.glsl`) : un quad face-caméra, un fragment
//! shader qui reconstruit une densité, et un **blending additif**. La matière
//! n'a plus de surface, seulement une concentration : là où elle est faible, les
//! étoiles passent au travers.
//!
//! Le composant [`Composant::Panache`] ne dessine donc rien lui-même — il ne
//! sert qu'à **porter la pose** (position de la tuyère, axe du jet, cotes) dans
//! l'assemblage, et c'est cette vue qui la lit pour bâtir le ruban.

use crate::astre::CameraInfo;
use crate::vaisseau::{rayon_panache, teinte_panache, Composant, Station};
use macroquad::miniquad::{BlendFactor, BlendState, Equation};
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

/// Nombre de segments d'un ruban. La turbulence est calculée dans le fragment
/// shader, donc le maillage n'a qu'à porter le **profil** — mais il lui en faut
/// assez pour que la courbe d'évasement ne se lise pas comme une ligne brisée.
const SEGMENTS: usize = 28;

/// Élargissement du ruban par rapport au jet lui-même.
///
/// Le shader éteint la densité **avant** le bord du ruban (profil en cœur) : à
/// largeur égale, le jet paraîtrait plus mince que sa cote. Le ruban est donc
/// tiré un peu plus large que le rayon annoncé, pour que ce soit bien ce rayon
/// qu'on voie.
const DEBORD: f32 = 1.6;

pub struct RenduPanache {
    mat: Material,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
}

impl RenduPanache {
    pub fn new() -> Self {
        let mat = load_material(
            ShaderSource::Glsl {
                vertex: &crate::impostor::source("panache.vert.glsl", VERT),
                fragment: &crate::impostor::source("panache.frag.glsl", FRAG),
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("time", UniformType::Float1),
                    UniformDesc::new("intensite", UniformType::Float1),
                ],
                pipeline_params: PipelineParams {
                    // Additif pur, et **sans écriture de profondeur** : le jet
                    // est un milieu, pas une surface. L'écrire dans le Z-buffer
                    // masquerait le vaisseau et les étoiles derrière lui.
                    depth_test: Comparison::LessOrEqual,
                    depth_write: false,
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("shader de panache");
        Self { mat, verts: Vec::new(), inds: Vec::new() }
    }

    /// Dessine tous les panaches de la station. À appeler **après** la coque :
    /// en additif sans test d'occlusion propre, le jet doit passer par-dessus.
    pub fn dessiner(&mut self, station: &Station, cam: &CameraInfo, temps: f32) {
        for piece in station.pieces() {
            let Composant::Panache { longueur, rayon_col, rayon_bout, intensite } = piece.composant
            else {
                continue;
            };
            if intensite <= 1e-3 {
                continue;
            }
            let m = piece.transforme;
            let origine = m.transform_point3(Vec3::ZERO);
            let axe = m.transform_vector3(Vec3::Z).normalize_or_zero();
            if axe == Vec3::ZERO {
                continue;
            }
            self.ruban(origine, axe, longueur, rayon_col, rayon_bout, intensite, cam, temps);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ruban(
        &mut self,
        origine: Vec3,
        axe: Vec3,
        longueur: f32,
        rayon_col: f32,
        rayon_bout: f32,
        intensite: f32,
        cam: &CameraInfo,
        temps: f32,
    ) {
        // Direction **en travers** du ruban : perpendiculaire à la fois à l'axe
        // du jet et à la visée. C'est ce qui fait qu'un ruban plat garde la même
        // épaisseur apparente d'où qu'on le regarde. Quand on regarde le jet
        // pile dans l'axe, le produit vectoriel s'effondre — on retombe alors
        // sur le repère caméra, faute de mieux, et le jet est de toute façon vu
        // comme un point.
        let vers_cam = -cam.forward;
        let mut lat = axe.cross(vers_cam);
        if lat.length_squared() < 1e-6 {
            lat = cam.right;
        }
        let lat = lat.normalize();

        let porte = longueur * intensite.clamp(0.0, 1.0);
        self.verts.clear();
        self.inds.clear();
        for k in 0..=SEGMENTS {
            let t = k as f32 / SEGMENTS as f32;
            let demi = rayon_panache(rayon_col, rayon_bout, t) * DEBORD;
            let c = origine + axe * (porte * t);
            let couleur = teinte_panache(t);
            let i = self.verts.len() as u16;
            self.verts.push(Vertex::new2(c - lat * demi, vec2(0.0, t), couleur));
            self.verts.push(Vertex::new2(c + lat * demi, vec2(1.0, t), couleur));
            if k > 0 {
                self.inds
                    .extend_from_slice(&[i - 2, i - 1, i + 1, i - 2, i + 1, i]);
            }
        }

        self.mat.set_uniform("time", temps);
        self.mat.set_uniform("intensite", intensite);
        gl_use_material(&self.mat);
        let quad = Mesh {
            vertices: std::mem::take(&mut self.verts),
            indices: std::mem::take(&mut self.inds),
            texture: None,
        };
        draw_mesh(&quad);
        self.verts = quad.vertices;
        self.inds = quad.indices;
        gl_use_default_material();
    }
}

const VERT: &str = include_str!("../shaders/panache.vert.glsl");
const FRAG: &str = include_str!("../shaders/panache.frag.glsl");
