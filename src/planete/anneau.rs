use super::apparence::Apparence;
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::f32::consts::TAU;

const ANNEAU_RADIAL: usize = 44; // bandes radiales (résolution pour les lacunes)
const ANNEAU_SEG: usize = 120; // segments angulaires (assez fins pour les arcs)

/// Hash déterministe 1D (stable, n'utilise pas le RNG global).
fn hash(x: f32) -> f32 {
    let v = (x * 12.9898).sin() * 43758.5453;
    v - v.floor()
}

fn gauss(x: f32, c: f32, w: f32) -> f32 {
    let d = (x - c) / w;
    (-d * d).exp()
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Profil d'un anneau selon le style : `t` = fraction radiale (0 interne .. 1 externe),
/// `ang` = fraction angulaire (0..1). Renvoie (luminosité, alpha).
fn profil(style: i32, t: f32, ang: f32, seed: f32) -> (f32, f32) {
    match style {
        // Ceinture granuleuse : particules éparses sur une large couronne (petite ceinture d'astéroïdes).
        1 => {
            let cell = (t * 22.0).floor() + (ang * 30.0).floor() * 5.7 + seed;
            let g = hash(cell);
            let envel = gauss(t, 0.55, 0.33); // présent sur une couronne large
            let a = if g > 0.45 {
                (g - 0.45) / 0.55 * 0.6 * envel
            } else {
                0.0
            };
            (0.7 + 0.4 * g, a)
        }
        // Monobande FINE : une seule bande étroite et nette (anneau bleu ciel d'Uranus).
        4 => {
            let a = smoothstep(0.5, 0.56, t) * (1.0 - smoothstep(0.64, 0.7, t)) * 0.8;
            (0.97, a)
        }
        // Neptune : arcs partiels (anneau d'Adams) -> alpha modulé en angle.
        2 => {
            let ring = gauss(t, 0.86, 0.04) + 0.35 * gauss(t, 0.62, 0.03);
            // Trois arcs concentrés sur ~1/4 de la circonférence (Fraternité/Égalité/Liberté).
            let mut arc = 0.0_f32;
            for c in [0.12_f32, 0.2, 0.3] {
                let mut d = (ang - c).abs();
                d = d.min(1.0 - d);
                arc += gauss(d, 0.0, 0.022);
            }
            // Léger voile continu très faible pour suggérer l'anneau complet.
            let voile = 0.06 * ring;
            (0.85, (ring * arc.min(1.0) * 0.85 + voile).min(0.9))
        }
        // Débris récents : amas irréguliers et brillants.
        3 => {
            let cell = (t * 26.0).floor() + (ang * 38.0).floor() * 7.13 + seed;
            let clump = hash(cell);
            let a = if clump > 0.5 { (clump - 0.5) / 0.5 * 0.85 } else { 0.0 };
            (0.7 + 0.5 * clump, a)
        }
        // Saturne (0) : anneaux denses + lacunes de Cassini et d'Encke + texture fine.
        _ => {
            let mut a = 0.72_f32;
            if t < 0.18 {
                a = 0.3; // anneau C interne, ténu (crêpe)
            }
            a *= 1.0 - 0.93 * gauss(t, 0.58, 0.022); // lacune de Cassini
            a *= 1.0 - 0.85 * gauss(t, 0.88, 0.007); // lacune d'Encke (fine, externe)
            a *= 0.82 + 0.18 * hash(t * 130.0 + seed); // fine texture radiale
            let lum = 0.78 + 0.22 * (t * 7.0).sin().abs();
            (lum, a.clamp(0.0, 0.9))
        }
    }
}

/// Construit l'anneau en quads (positions relatives au centre de la planète).
pub fn construire_anneau(rayon: f32, app: &Apparence) -> Vec<[Vertex; 4]> {
    let normal = app.anneau_normal.normalize_or_zero();
    let tmp = if normal.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
    let u = normal.cross(tmp).normalize();
    let v = normal.cross(u);

    let r_in = rayon * app.anneau_in;
    let r_out = rayon * app.anneau_out;
    let c = app.anneau_couleur;
    let style = app.anneau_style as i32;
    let seed = app.seed;

    let mut quads: Vec<[Vertex; 4]> = Vec::new();

    for j in 0..ANNEAU_RADIAL {
        let t0 = j as f32 / ANNEAU_RADIAL as f32;
        let t1 = (j + 1) as f32 / ANNEAU_RADIAL as f32;
        let tm = (t0 + t1) * 0.5;
        let f0 = r_in + (r_out - r_in) * t0;
        let f1 = r_in + (r_out - r_in) * t1;

        for k in 0..ANNEAU_SEG {
            let angm = (k as f32 + 0.5) / ANNEAU_SEG as f32;
            let (lum, alpha) = profil(style, tm, angm, seed);
            if alpha < 0.01 {
                continue; // pas de quad invisible -> mesh plus léger pour les styles épars
            }
            let a0 = k as f32 / ANNEAU_SEG as f32 * TAU;
            let a1 = (k + 1) as f32 / ANNEAU_SEG as f32 * TAU;
            let d0 = u * a0.cos() + v * a0.sin();
            let d1 = u * a1.cos() + v * a1.sin();
            let col = Color::new(c.x * lum, c.y * lum, c.z * lum, alpha);
            quads.push([
                Vertex::new2(d0 * f0, vec2(0.0, 0.0), col),
                Vertex::new2(d0 * f1, vec2(1.0, 0.0), col),
                Vertex::new2(d1 * f1, vec2(1.0, 1.0), col),
                Vertex::new2(d1 * f0, vec2(0.0, 1.0), col),
            ]);
        }
    }
    quads
}
