use super::eruptions::EtatBoucle;
use super::{Soleil, MAX_TACHES};
use crate::astre::CameraInfo;
use crate::impostor::push_quad;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

pub(super) const TAILLE_HALO: usize = 64;
const QUADS_PAR_LOT: usize = 200;

impl Soleil {
    pub(super) fn dessiner(&mut self, cam: &CameraInfo) {
        let c = self.base.position;
        let r = self.base.rayon;
        let rot = self.temps * 0.12;

        // ---- 1) Corps : quad + shader ----
        self.verts.clear();
        self.inds.clear();
        push_quad(&mut self.verts, &mut self.inds, c, cam.right, cam.up, r * self.couronne, WHITE);

        // Uniforms communs (temps, repère caméra, échelle du disque = couronne).
        crate::impostor::uniforms_standard(&self.mat, cam, self.temps, self.couronne);
        self.mat
            .set_uniform("teinte", (self.couleur.x, self.couleur.y, self.couleur.z));
        self.mat.set_uniform("couronne", self.couronne);
        self.mat.set_uniform("couronne_irreg", self.couronne_irreg);
        self.mat.set_uniform("couronne_type", self.couronne_type);

        // Taches -> uniform array (xyz = direction surface, w = rayon effectif)
        let mut spots = [Vec4::ZERO; MAX_TACHES];
        for (i, t) in self.taches.iter().take(MAX_TACHES).enumerate() {
            spots[i] = vec4(t.dir.x, t.dir.y, t.dir.z, t.taille * t.intensite);
        }
        self.mat.set_uniform_array("spots", &spots[..]);

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

        // ---- 2) Boucles + particules : billboards en additif ----
        gl_use_material(&self.mat_plasma);
        self.verts.clear();
        self.inds.clear();
        let mut quads = 0;

        let taille_l = r * 0.05; // un peu épais : cœur lumineux par accumulation
        for b in &self.boucles {
            let a_w = vers_monde(b.sa, rot);
            let b_w = vers_monde(b.sb, rot);
            let n = 48;
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let pos = point_boucle(a_w, b_w, b.apex, b.skew, t, r);
                let monde = c + pos;
                if cache_par_sphere(monde, c, r, cam.pos) {
                    continue; // derrière le soleil -> pas visible à travers
                }
                // onde de matière qui circule + intensité de la boucle
                let flow = 0.55 + 0.45 * (t * 16.0 - self.temps * 6.0).sin();
                let lum = (b.intensite * (0.55 + 0.45 * flow)).clamp(0.0, 1.0);
                // plasma chaud, additif -> les recouvrements forment un cœur blanc
                let col = Color::new(1.0, 0.8 + 0.15 * lum, 0.5 + 0.3 * lum, lum);
                push_quad(&mut self.verts, &mut self.inds, monde, cam.right, cam.up, taille_l, col);
                quads += 1;
                if quads >= QUADS_PAR_LOT {
                    flush(&mut self.verts, &mut self.inds, &self.halo);
                    quads = 0;
                }
            }
        }

        // Cœur dense brillant au sommet des éjections (matière de proéminence).
        for b in &self.boucles {
            if b.etat == EtatBoucle::Rupture {
                let a_w = vers_monde(b.sa, rot);
                let b_w = vers_monde(b.sb, rot);
                let pos = c + point_boucle(a_w, b_w, b.apex, b.skew, 0.5, r);
                if !cache_par_sphere(pos, c, r, cam.pos) {
                    let lum = b.intensite.clamp(0.0, 1.0);
                    let col = Color::new(1.0, 0.72, 0.45, lum);
                    push_quad(&mut self.verts, &mut self.inds, pos, cam.right, cam.up, r * 0.16, col);
                    quads += 1;
                    if quads >= QUADS_PAR_LOT {
                        flush(&mut self.verts, &mut self.inds, &self.halo);
                        quads = 0;
                    }
                }
            }
        }
        flush(&mut self.verts, &mut self.inds, &self.halo);
        gl_use_default_material();
    }
}

/// Passe une direction du repère "surface" (où vivent taches/boucles) au repère monde,
/// en inversant la rotation appliquée par le shader.
fn vers_monde(s: Vec3, a: f32) -> Vec3 {
    let (sa, ca) = a.sin_cos();
    vec3(s.x * ca + s.z * sa, s.y, -s.x * sa + s.z * ca)
}

/// Point sur l'arche (Bézier quadratique a -> sommet -> b), relatif au centre.
/// `skew` décale le sommet le long de la ligne des pieds (arches asymétriques).
fn point_boucle(a: Vec3, b: Vec3, apex: f32, skew: f32, t: f32, rayon: f32) -> Vec3 {
    let pa = a * rayon;
    let pb = b * rayon;
    let mid = (pa + pb) * 0.5;
    let outward = mid.normalize_or_zero();
    let along = (pb - pa).normalize_or_zero();
    let ctrl = mid + outward * (2.0 * apex * rayon) + along * (skew * rayon);
    let u = 1.0 - t;
    pa * (u * u) + ctrl * (2.0 * u * t) + pb * (t * t)
}

/// Vrai si le point `p` est masqué par la sphère (centre `c`, rayon `r`) vu depuis `cam`.
fn cache_par_sphere(p: Vec3, c: Vec3, r: f32, cam: Vec3) -> bool {
    let axe = (cam - c).normalize_or_zero(); // du centre vers la caméra
    let rel = p - c;
    let along = rel.dot(axe);
    if along >= 0.0 {
        return false; // côté caméra -> visible
    }
    let perp = rel - axe * along; // distance à l'axe de visée
    perp.length() < r * 0.98 // dans la silhouette ET derrière -> caché
}

fn flush(verts: &mut Vec<Vertex>, inds: &mut Vec<u16>, tex: &Texture2D) {
    if inds.is_empty() {
        return;
    }
    let mesh = Mesh {
        vertices: std::mem::take(verts),
        indices: std::mem::take(inds),
        texture: Some(tex.clone()),
    };
    draw_mesh(&mesh);
    let mut v = mesh.vertices;
    v.clear();
    let mut i = mesh.indices;
    i.clear();
    *verts = v;
    *inds = i;
}

pub(super) fn texture_halo(taille: usize) -> Texture2D {
    let mut bytes = vec![0u8; taille * taille * 4];
    let centre = (taille as f32 - 1.0) / 2.0;
    for y in 0..taille {
        for x in 0..taille {
            let dx = (x as f32 - centre) / centre;
            let dy = (y as f32 - centre) / centre;
            let a = (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let a = a * a;
            let i = (y * taille + x) * 4;
            bytes[i] = 255;
            bytes[i + 1] = 255;
            bytes[i + 2] = 255;
            bytes[i + 3] = (a * 255.0) as u8;
        }
    }
    let t = Texture2D::from_rgba8(taille as u16, taille as u16, &bytes);
    t.set_filter(FilterMode::Linear);
    t
}
