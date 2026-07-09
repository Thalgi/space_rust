// Simulation MHD idéale 2.5D (volumes finis) — inspirée de Piantschitsch et al. 2018 :
// une onde magnétosonore se propage dans la couronne et interagit avec un trou
// coronal (région de faible densité) -> réflexion / réfraction / transmission.
//
// Variables conservées par cellule : [rho, rho*vx, rho*vy, rho*vz, Bx, By, Bz, E]
// Unités avec mu0 = 1 (pression magnétique = |B|^2 / 2).
//
// Schéma : flux de Rusanov (local Lax-Friedrichs), Euler explicite en temps,
// termes source de Powell pour limiter l'erreur sur div(B). Conditions aux
// limites : sortie libre (gradient nul).
//
// Lancer :  cargo run --bin mhd

use macroquad::prelude::*;

// Partage la police Minitel du projet (binaire séparé -> inclusion par chemin).
#[path = "../police.rs"]
mod police;

const NX: usize = 200;
const NY: usize = 200;
const DX: f64 = 1.0 / NX as f64;
const DY: f64 = 1.0 / NY as f64;
const GAMMA: f64 = 5.0 / 3.0;
const CFL: f64 = 0.3;
const PAS_PAR_FRAME: usize = 8;

type Cell = [f64; 8];

fn idx(i: usize, j: usize) -> usize {
    j * NX + i
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Variables primitives : (rho, vx, vy, vz, p, bx, by, bz)
fn prim(u: &Cell) -> (f64, f64, f64, f64, f64, f64, f64, f64) {
    let rho = u[0].max(1e-6);
    let vx = u[1] / rho;
    let vy = u[2] / rho;
    let vz = u[3] / rho;
    let (bx, by, bz) = (u[4], u[5], u[6]);
    let ke = 0.5 * rho * (vx * vx + vy * vy + vz * vz);
    let pb = 0.5 * (bx * bx + by * by + bz * bz);
    let p = ((GAMMA - 1.0) * (u[7] - ke - pb)).max(1e-6);
    (rho, vx, vy, vz, p, bx, by, bz)
}

fn flux_x(u: &Cell) -> Cell {
    let (rho, vx, vy, vz, p, bx, by, bz) = prim(u);
    let pb = 0.5 * (bx * bx + by * by + bz * bz);
    let ptot = p + pb;
    let e = u[7];
    let vdotb = vx * bx + vy * by + vz * bz;
    [
        rho * vx,
        rho * vx * vx + ptot - bx * bx,
        rho * vx * vy - bx * by,
        rho * vx * vz - bx * bz,
        0.0,
        vx * by - vy * bx,
        vx * bz - vz * bx,
        (e + ptot) * vx - bx * vdotb,
    ]
}

fn flux_y(u: &Cell) -> Cell {
    let (rho, vx, vy, vz, p, bx, by, bz) = prim(u);
    let pb = 0.5 * (bx * bx + by * by + bz * bz);
    let ptot = p + pb;
    let e = u[7];
    let vdotb = vx * bx + vy * by + vz * bz;
    [
        rho * vy,
        rho * vy * vx - by * bx,
        rho * vy * vy + ptot - by * by,
        rho * vy * vz - by * bz,
        vy * bx - vx * by,
        0.0,
        vy * bz - vz * by,
        (e + ptot) * vy - by * vdotb,
    ]
}

/// Vitesse magnétosonore rapide + |v| dans une direction (n = 0 : x, n = 1 : y).
fn vitesse(u: &Cell, n: usize) -> f64 {
    let (rho, vx, vy, _vz, p, bx, by, bz) = prim(u);
    let a2 = GAMMA * p / rho;
    let b2 = (bx * bx + by * by + bz * bz) / rho;
    let (vn, bn) = if n == 0 { (vx, bx) } else { (vy, by) };
    let ca2 = bn * bn / rho;
    let disc = ((a2 + b2) * (a2 + b2) - 4.0 * a2 * ca2).max(0.0);
    let cf = (0.5 * (a2 + b2 + disc.sqrt())).max(0.0).sqrt();
    vn.abs() + cf
}

fn rusanov(l: &Cell, r: &Cell, n: usize) -> Cell {
    let (fl, fr) = if n == 0 {
        (flux_x(l), flux_x(r))
    } else {
        (flux_y(l), flux_y(r))
    };
    let s = vitesse(l, n).max(vitesse(r, n));
    let mut f = [0.0; 8];
    for k in 0..8 {
        f[k] = 0.5 * (fl[k] + fr[k]) - 0.5 * s * (r[k] - l[k]);
    }
    f
}

fn dt_max(u: &[Cell]) -> f64 {
    let mut smax: f64 = 1e-6;
    for c in u {
        smax = smax.max(vitesse(c, 0)).max(vitesse(c, 1));
    }
    CFL * DX.min(DY) / smax
}

fn pas(u: &[Cell], dt: f64) -> Vec<Cell> {
    let mut nu = u.to_vec();
    for j in 0..NY {
        for i in 0..NX {
            let c = u[idx(i, j)];
            let xl = u[idx(i.saturating_sub(1), j)];
            let xr = u[idx((i + 1).min(NX - 1), j)];
            let yl = u[idx(i, j.saturating_sub(1))];
            let yr = u[idx(i, (j + 1).min(NY - 1))];

            let fxr = rusanov(&c, &xr, 0);
            let fxl = rusanov(&xl, &c, 0);
            let fyr = rusanov(&c, &yr, 1);
            let fyl = rusanov(&yl, &c, 1);

            // Terme source de Powell (transporte l'erreur sur div B hors du domaine).
            let divb = (xr[4] - xl[4]) / (2.0 * DX) + (yr[5] - yl[5]) / (2.0 * DY);
            let (_rho, vx, vy, vz, _p, bx, by, bz) = prim(&c);
            let vdotb = vx * bx + vy * by + vz * bz;
            let src = [
                0.0,
                -divb * bx,
                -divb * by,
                -divb * bz,
                -divb * vx,
                -divb * vy,
                -divb * vz,
                -divb * vdotb,
            ];

            let mut nc = [0.0; 8];
            for k in 0..8 {
                nc[k] = c[k] - dt / DX * (fxr[k] - fxl[k]) - dt / DY * (fyr[k] - fyl[k])
                    + dt * src[k];
            }
            nc[0] = nc[0].max(1e-6); // densité positive
            nu[idx(i, j)] = nc;
        }
    }
    nu
}

fn reset() -> Vec<Cell> {
    let mut u = vec![[0.0; 8]; NX * NY];
    let bx0 = 0.2; // champ magnétique de fond, uniforme selon x
    for j in 0..NY {
        for i in 0..NX {
            let x = (i as f64 + 0.5) * DX;
            let y = (j as f64 + 0.5) * DY;

            // Trou coronal : densité plus faible (bord adouci).
            let dh = ((x - 0.7).powi(2) + (y - 0.5).powi(2)).sqrt();
            let hole = smoothstep(0.16, 0.12, dh);
            let mut rho = 1.0 - 0.75 * hole;
            let mut p = 0.1;

            // Impulsion initiale : surdensité + surpression (l'onde va rayonner).
            let dp = (x - 0.3).powi(2) + (y - 0.5).powi(2);
            let g = (-dp / 0.002).exp();
            rho += 0.8 * g;
            p += 0.6 * g;

            let (vx, vy, vz) = (0.0, 0.0, 0.0);
            let (bx, by, bz) = (bx0, 0.0, 0.0);
            let ke = 0.5 * rho * (vx * vx + vy * vy + vz * vz);
            let pb = 0.5 * (bx * bx + by * by + bz * bz);
            let e = p / (GAMMA - 1.0) + ke + pb;
            u[idx(i, j)] = [rho, rho * vx, rho * vy, rho * vz, bx, by, bz, e];
        }
    }
    u
}

/// Valeur du champ affiché selon le mode.
fn champ(u: &Cell, mode: i32) -> f64 {
    let (rho, vx, vy, vz, p, bx, by, bz) = prim(u);
    match mode {
        1 => p,
        2 => (bx * bx + by * by + bz * bz).sqrt(),
        3 => (vx * vx + vy * vy + vz * vz).sqrt(),
        _ => rho,
    }
}

fn couleur(t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let r = (t * 3.0).clamp(0.0, 1.0);
    let g = (t * 3.0 - 1.0).clamp(0.0, 1.0);
    let b = (t * 3.0 - 2.0).clamp(0.0, 1.0);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn nom_champ(mode: i32) -> &'static str {
    match mode {
        1 => "pression",
        2 => "|B|",
        3 => "|v|",
        _ => "densite",
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "MHD 2.5D - onde / trou coronal".to_owned(),
        window_width: 820,
        window_height: 860,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    police::charger().await;

    let mut u = reset();
    let mut temps = 0.0;
    let mut pause = false;
    let mut mode: i32 = 0;
    let mut bytes = vec![0u8; NX * NY * 4];

    loop {
        if is_key_pressed(KeyCode::R) {
            u = reset();
            temps = 0.0;
        }
        if is_key_pressed(KeyCode::Space) {
            pause = !pause;
        }
        if is_key_pressed(KeyCode::Key1) {
            mode = 0;
        }
        if is_key_pressed(KeyCode::Key2) {
            mode = 1;
        }
        if is_key_pressed(KeyCode::Key3) {
            mode = 2;
        }
        if is_key_pressed(KeyCode::Key4) {
            mode = 3;
        }

        if !pause {
            for _ in 0..PAS_PAR_FRAME {
                let dt = dt_max(&u);
                u = pas(&u, dt);
                temps += dt;
            }
        }

        // Normalisation dynamique du champ affiché.
        let mut vmin = f64::INFINITY;
        let mut vmax = f64::NEG_INFINITY;
        for c in &u {
            let v = champ(c, mode);
            vmin = vmin.min(v);
            vmax = vmax.max(v);
        }
        let inv = if vmax - vmin > 1e-9 {
            1.0 / (vmax - vmin)
        } else {
            0.0
        };

        for j in 0..NY {
            for i in 0..NX {
                let v = champ(&u[idx(i, j)], mode);
                let t = ((v - vmin) * inv) as f32;
                let (r, g, b) = couleur(t);
                // ligne 0 en haut de l'image
                let p = ((NY - 1 - j) * NX + i) * 4;
                bytes[p] = r;
                bytes[p + 1] = g;
                bytes[p + 2] = b;
                bytes[p + 3] = 255;
            }
        }

        let tex = Texture2D::from_rgba8(NX as u16, NY as u16, &bytes);
        tex.set_filter(FilterMode::Nearest);

        clear_background(BLACK);
        let cote = screen_width().min(screen_height() - 40.0);
        draw_texture_ex(
            &tex,
            0.0,
            40.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(cote, cote)),
                ..Default::default()
            },
        );

        police::texte(
            &format!(
                "t = {:.2}   champ: {}   [1-4] champ  [espace] pause  [R] reset",
                temps,
                nom_champ(mode)
            ),
            10.0,
            26.0,
            22.0,
            WHITE,
        );

        next_frame().await;
    }
}
