use super::eruptions::EtatBoucle;
use super::{Soleil, MAX_TACHES};
use crate::astre::CameraInfo;
use crate::impostor::push_quad;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use std::f32::consts::PI;

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
        self.mat.set_uniform("axe", (self.axe.x, self.axe.y, self.axe.z));
        self.mat.set_uniform("gran_scale", self.gran_scale);
        self.mat.set_uniform("gran_contraste", self.gran_contraste);

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

        // ---- 2) Boucles : RUBANS triangulés (nappes/loft) en additif ----
        // Chaque arche devient un ruban : on échantillonne la courbe, on l'élargit
        // perpendiculairement à la tangente (face caméra). L'uv vertical (0..1) mappe le
        // profil radial du halo -> bords doux, cœur brillant (vraie nappe, pas un chapelet).
        gl_use_material(&self.mat_plasma);
        self.verts.clear();
        self.inds.clear();
        let mut quads = 0;
        let temps = self.temps;
        let n = 28;
        for b in &self.boucles {
            let a_w = vers_monde(b.sa, rot);
            let b_w = vers_monde(b.sb, rot);
            // Normale du PLAN de la boucle (constante) -> ruban stable, sans vrillage caméra.
            let mut nrm = a_w.cross(b_w).normalize_or_zero();
            if nrm.length_squared() < 0.01 {
                nrm = a_w.cross(Vec3::Y).normalize_or_zero();
            }
            let mut prev: Option<(Vec3, Vec3, Color)> = None;
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let p = c + point_boucle(a_w, b_w, b.apex, b.skew, t, r);
                // Tangente par différence finie ; largeur dans le plan de la boucle.
                let e = 0.012;
                let pa = c + point_boucle(a_w, b_w, b.apex, b.skew, (t - e).max(0.0), r);
                let pb = c + point_boucle(a_w, b_w, b.apex, b.skew, (t + e).min(1.0), r);
                let tang = (pb - pa).normalize_or_zero();
                let wdir = tang.cross(nrm).normalize_or_zero();
                // Largeur : fine aux pieds, plus large au sommet ; s'évase fort pour la CME.
                let prof = (t * PI).sin();
                let mut hw = r * (0.025 + 0.07 * prof);
                if b.etat == EtatBoucle::Rupture {
                    hw *= 1.0 + 0.9 * (b.apex / b.apex_max.max(0.1) - 1.0).clamp(0.0, 2.0); // nappe d'éjection
                }
                let aedge = p + wdir * hw;
                let bedge = p - wdir * hw;
                let flow = 0.55 + 0.45 * (t * 16.0 - temps * 6.0).sin();
                let lum = (b.intensite * (0.5 + 0.5 * flow)).clamp(0.0, 1.0);
                let col = Color::new(1.0, 0.8 + 0.15 * lum, 0.5 + 0.3 * lum, lum * 0.9);
                let hidden = cache_par_sphere(p, c, r, cam.pos);
                if let Some((a0, b0, c0)) = prev {
                    if !hidden {
                        pousser_segment(&mut self.verts, &mut self.inds, a0, b0, c0, aedge, bedge, col);
                        quads += 1;
                        if quads >= QUADS_PAR_LOT {
                            flush(&mut self.verts, &mut self.inds, &self.halo);
                            quads = 0;
                        }
                    }
                }
                prev = if hidden { None } else { Some((aedge, bedge, col)) };
            }
        }

        // ---- 3) Flares : flash impulsif + rubans + arcade post-flare + CME ----
        for f in &self.flares {
            let age = f.age;
            let site = vers_monde(f.centre, rot); // direction radiale du site (monde)
            let tang = vers_monde(f.tang, rot); // ligne d'inversion (monde)
            let perp = site.cross(tang).normalize_or_zero(); // ⟂ ligne d'inversion, sur la surface
            let l = f.echelle; // demi-longueur angulaire des rubans

            // --- Flash impulsif : montée quasi instantanée, décroissance exponentielle. ---
            let flash = (smoothstep(0.0, 0.05, age) * (-age * 5.0).exp() * f.force).clamp(0.0, 1.0);
            if flash > 0.01 {
                let pf = c + site * (r * 1.05);
                if !cache_par_sphere(pf, c, r, cam.pos) {
                    for k in 0..3 {
                        let s = r * (0.16 + 0.20 * k as f32) * (0.8 + 0.4 * f.force);
                        let a = flash * (1.0 - 0.25 * k as f32);
                        let col = Color::new(0.78, 0.86, 1.0, a); // blanc-bleu (continuum)
                        push_quad(&mut self.verts, &mut self.inds, pf, cam.right, cam.up, s, col);
                        quads += 1;
                    }
                    // léger embrasement global : la naine M « s'éclaire » d'un coup.
                    let pg = c + (cam.pos - c).normalize_or_zero() * (r * 0.4);
                    let cg = Color::new(0.72, 0.83, 1.0, flash * 0.22);
                    push_quad(&mut self.verts, &mut self.inds, pg, cam.right, cam.up, r * 1.7, cg);
                    quads += 1;
                }
                if quads >= QUADS_PAR_LOT {
                    flush(&mut self.verts, &mut self.inds, &self.halo);
                    quads = 0;
                }
            }

            // Écartement des rubans / pieds de l'arcade (les rubans « s'éloignent » de la ligne).
            let sep = 0.03 + 0.06 * smoothstep(0.0, 3.5, age);

            // --- Deux rubans chromosphériques de part et d'autre de la ligne d'inversion. ---
            let ribbon = smoothstep(0.04, 0.2, age) * (1.0 - smoothstep(2.5, 4.5, age));
            if ribbon > 0.01 {
                let nseg = 10;
                for side in [-1.0_f32, 1.0] {
                    for i in 0..=nseg {
                        let u = (i as f32 / nseg as f32 - 0.5) * 2.0 * l;
                        let dir = (site + tang * u + perp * (side * sep)).normalize_or_zero();
                        let p = c + dir * (r * 1.02);
                        if cache_par_sphere(p, c, r, cam.pos) {
                            continue;
                        }
                        let edge = 0.6 + 0.4 * (i as f32 * 0.9 + f.graine).sin();
                        let col = Color::new(1.0, 0.5, 0.66, ribbon * edge); // rose-blanc (Hα)
                        push_quad(&mut self.verts, &mut self.inds, p, cam.right, cam.up, r * 0.06, col);
                        quads += 1;
                    }
                }
                if quads >= QUADS_PAR_LOT {
                    flush(&mut self.verts, &mut self.inds, &self.halo);
                    quads = 0;
                }
            }

            // --- Arcade de boucles post-flare reliant les deux rubans, qui s'élève. ---
            let arc = smoothstep(0.4, 1.3, age) * (1.0 - smoothstep(4.0, 6.0, age));
            if arc > 0.01 {
                let apex = (0.12 + 0.5 * smoothstep(0.5, 4.5, age)) * (0.6 + 0.6 * f.force);
                let narch = 5;
                for jx in 0..narch {
                    let u = ((jx as f32 + 0.5) / narch as f32 - 0.5) * 2.0 * l;
                    let da = (site + tang * u + perp * sep).normalize_or_zero();
                    let db = (site + tang * u - perp * sep).normalize_or_zero();
                    let n = 24;
                    for i in 0..=n {
                        let t = i as f32 / n as f32;
                        let pos = c + point_boucle(da, db, apex, 0.0, t, r);
                        if cache_par_sphere(pos, c, r, cam.pos) {
                            continue;
                        }
                        let flow = 0.6 + 0.4 * (t * 12.0 - self.temps * 5.0).sin();
                        let col = Color::new(1.0, 0.78, 0.5, arc * flow); // plasma chaud
                        push_quad(&mut self.verts, &mut self.inds, pos, cam.right, cam.up, r * 0.05, col);
                        quads += 1;
                    }
                    if quads >= QUADS_PAR_LOT {
                        flush(&mut self.verts, &mut self.inds, &self.halo);
                        quads = 0;
                    }
                }
            }

            // --- CME : bulle de plasma douce qui se détache et s'éloigne (sprites superposés). ---
            if f.cme {
                let app = smoothstep(0.0, 0.4, age);
                let life = 1.0 - smoothstep(4.0, 7.0, f.cme_dist); // s'estompe en s'éloignant
                let cme_a = (app * life).clamp(0.0, 1.0);
                let centre_b = c + site * (r * f.cme_dist);
                if cme_a > 0.01 && !cache_par_sphere(centre_b, c, r, cam.pos) {
                    let rb = (r * (0.28 + 0.25 * (f.cme_dist - 1.0))).max(r * 0.22); // grossit en montant
                    // Halo diffus + cœur + bord d'attaque : une bulle ronde, pas un anneau de points.
                    let glow = Color::new(1.0, 0.6, 0.78, cme_a * 0.28);
                    push_quad(&mut self.verts, &mut self.inds, centre_b, cam.right, cam.up, rb * 1.7, glow);
                    let core = Color::new(1.0, 0.82, 0.88, cme_a * 0.5);
                    push_quad(&mut self.verts, &mut self.inds, centre_b, cam.right, cam.up, rb, core);
                    let front = centre_b + site * (rb * 0.6); // front de choc plus brillant
                    let fc = Color::new(1.0, 0.9, 0.95, cme_a * 0.4);
                    push_quad(&mut self.verts, &mut self.inds, front, cam.right, cam.up, rb * 0.7, fc);
                    quads += 3;
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

/// Interpolation lissée (Hermite) — locale au rendu.
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

/// Pousse un quad de ruban entre deux sections (loft) : bords (a,b) avec uv vertical 0/1
/// -> profil radial du halo (cœur brillant, bords doux). Couleur par section.
fn pousser_segment(
    verts: &mut Vec<Vertex>,
    inds: &mut Vec<u16>,
    a0: Vec3,
    b0: Vec3,
    c0: Color,
    a1: Vec3,
    b1: Vec3,
    c1: Color,
) {
    let i0 = verts.len() as u16;
    verts.push(Vertex::new2(a0, vec2(0.5, 0.02), c0));
    verts.push(Vertex::new2(b0, vec2(0.5, 0.98), c0));
    verts.push(Vertex::new2(b1, vec2(0.5, 0.98), c1));
    verts.push(Vertex::new2(a1, vec2(0.5, 0.02), c1));
    inds.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
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
