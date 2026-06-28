mod anneau;
mod apparence;
mod materiau;

pub use apparence::{Apparence, TypePlanete};

/// Vide le cache de materials des planètes (hot-reload des shaders).
pub fn vider_cache_materials() {
    materiau::vider_cache();
}

use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase};
use anneau::construire_anneau;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use materiau::{appliquer_uniforms, mat_anneau, mat_corps};

pub struct Planete {
    pub base: CorpsBase,
    app: Apparence,
    orbite: Vec<Vec3>,
    mat: Material,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
    // Anneau : quads relatifs au centre + tampons + material dédié.
    anneau_quads: Vec<[Vertex; 4]>,
    anneau_sv: Vec<Vertex>,
    anneau_si: Vec<u16>,
    mat_anneau: Option<Material>,
    // Lune : si parent défini, orbite analytique autour de l'astre `parent`.
    parent: Option<usize>,
    l_angle: f32,
    l_omega: f32,
    l_r: f32,
    l_a1: Vec3,
    l_q: Vec3,
}

impl Planete {
    pub fn new(
        position: Vec3,
        vitesse: Vec3,
        rayon: f32,
        masse: f32,
        app: Apparence,
        orbite: Vec<Vec3>,
    ) -> Self {
        let mut base = CorpsBase::new(position, masse, rayon);
        base.vitesse = vitesse;

        // Material partagé (cloné) -> un seul pipeline GPU pour toutes les planètes/lunes.
        let mat = mat_corps();

        // Construire l'anneau si nécessaire.
        let (anneau_quads, mat_anneau) = if app.anneau {
            (construire_anneau(rayon, &app), Some(mat_anneau()))
        } else {
            (Vec::new(), None)
        };

        Self {
            base,
            app,
            orbite,
            mat,
            verts: Vec::new(),
            inds: Vec::new(),
            anneau_quads,
            anneau_sv: Vec::new(),
            anneau_si: Vec::new(),
            mat_anneau,
            parent: None,
            l_angle: 0.0,
            l_omega: 0.0,
            l_r: 0.0,
            l_a1: Vec3::X,
            l_q: Vec3::Z,
        }
    }

    /// Niveau d'extension des lumières de villes (0 = aucune … 4 = très étendu).
    pub fn set_villes(&mut self, niveau: f32) {
        self.app.villes = niveau;
    }

    /// Vrai si la planète possède un anneau (la galerie recule la caméra pour le cadrer).
    pub fn a_un_anneau(&self) -> bool {
        self.app.anneau
    }

    /// Rayon externe de l'anneau (× rayon planète) ; 0 si aucun anneau.
    pub fn rayon_anneau(&self) -> f32 {
        if self.app.anneau {
            self.app.anneau_out
        } else {
            0.0
        }
    }

    /// Transforme cette planète en lune orbitant l'astre d'index `parent`.
    /// `r_orbite` en unités monde, `omega` vitesse angulaire, `incl` inclinaison, `phase` départ.
    pub fn en_lune(mut self, parent: usize, r_orbite: f32, omega: f32, incl: f32, phase: f32) -> Self {
        let phi = phase;
        let a1 = vec3(phi.cos(), 0.0, phi.sin());
        let a2 = vec3(-phi.sin(), 0.0, phi.cos());
        self.parent = Some(parent);
        self.l_r = r_orbite;
        self.l_omega = omega;
        self.l_angle = phase;
        self.l_a1 = a1;
        self.l_q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();
        self
    }

    /// Dessine la moitié avant (back=false) ou arrière (back=true) de l'anneau.
    /// Le mesh est émis par lots pour ne pas dépasser la taille max d'un draw call.
    fn dessiner_anneau(&mut self, c: Vec3, cam: &CameraInfo, back: bool) {
        let mat_a = match self.mat_anneau.clone() {
            Some(m) => m,
            None => return,
        };
        const MAX_QUADS: usize = 640; // < limite batcher (640*4 verts, 640*6 indices)
        let tocam = (cam.pos - c).normalize_or_zero();
        let mut sv = std::mem::take(&mut self.anneau_sv);
        let mut si = std::mem::take(&mut self.anneau_si);
        sv.clear();
        si.clear();
        gl_use_material(&mat_a);

        let mut nq = 0usize;
        for q in &self.anneau_quads {
            let centre_rel =
                (q[0].position + q[1].position + q[2].position + q[3].position) * 0.25;
            let derriere = centre_rel.dot(tocam) < 0.0;
            if derriere != back {
                continue;
            }
            let i0 = sv.len() as u16;
            for v in q {
                let mut w = *v;
                w.position += c;
                sv.push(w);
            }
            si.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
            nq += 1;
            if nq >= MAX_QUADS {
                let mesh = Mesh {
                    vertices: std::mem::take(&mut sv),
                    indices: std::mem::take(&mut si),
                    texture: None,
                };
                draw_mesh(&mesh);
                sv = mesh.vertices;
                si = mesh.indices;
                sv.clear();
                si.clear();
                nq = 0;
            }
        }
        if !si.is_empty() {
            let mesh = Mesh {
                vertices: std::mem::take(&mut sv),
                indices: std::mem::take(&mut si),
                texture: None,
            };
            draw_mesh(&mesh);
            sv = mesh.vertices;
            si = mesh.indices;
        }
        gl_use_default_material();
        self.anneau_sv = sv;
        self.anneau_si = si;
    }
}

impl Astre for Planete {
    fn categorie(&self) -> Categorie {
        if self.parent.is_some() {
            Categorie::Lune
        } else {
            Categorie::Planete
        }
    }
    fn parent(&self) -> Option<usize> {
        self.parent
    }
    fn orbiter_autour(&mut self, centre: Vec3, dt: f32) {
        self.l_angle += self.l_omega * dt;
        self.base.position =
            centre + self.l_a1 * (self.l_r * self.l_angle.cos()) + self.l_q * (self.l_r * self.l_angle.sin());
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn update(&mut self, _dt: f32) {}
    fn orbite(&self) -> &[Vec3] {
        &self.orbite
    }

    fn draw(&mut self, cam: &CameraInfo) {
        let c = self.base.position;
        let r = self.base.rayon;

        // --- Corps (impostor) ---
        self.verts.clear();
        self.inds.clear();
        crate::impostor::push_quad(&mut self.verts, &mut self.inds, c, cam.right, cam.up, r * 1.05, WHITE);

        appliquer_uniforms(&self.mat, &self.app, cam, c, r);

        // Anneau : moitié arrière AVANT le corps (la planète la masquera).
        if self.mat_anneau.is_some() {
            self.dessiner_anneau(c, cam, true);
        }

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

        // Anneau : moitié avant APRÈS le corps (passe devant la planète).
        if self.mat_anneau.is_some() {
            self.dessiner_anneau(c, cam, false);
        }
    }
}
