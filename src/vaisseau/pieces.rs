//! Briques de construction réutilisables pour les stations spatiales.
//!
//! Chaque fonction dessine un élément paramétré (position, orientation, taille,
//! couleurs) sans supposer de palette : les couleurs sont fournies par
//! l'appelant. Ces briques sont la base de la future génération procédurale de
//! stations — voir `docs/stations_procedurales.md`. Les blocs qui se
//! ressemblaient dans les maquettes (poutres, modules, ailes, radiateurs) sont
//! désormais centralisés ici.

use super::{cylindre, panneau};
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use std::f32::consts::FRAC_PI_3;

/// Pale à **tuiles hexagonales** légèrement espacées : un maillage de petits
/// hexagones (flat-top) réduits pour laisser un jour entre eux. Déployée depuis
/// `pied` le long de `deploy`, large de `largeur` selon `largeur_axe`.
pub(crate) fn pale_hexagonale(
    pied: Vec3,
    deploy: Vec3,
    largeur_axe: Vec3,
    longueur: f32,
    largeur: f32,
    couleur: Color,
) {
    let d = deploy.normalize();
    let w = largeur_axe.normalize();
    let r = 0.16; // rayon d'une tuile (centre → sommet)
    let hr = r * 0.86; // tuile réduite → espace visible entre tuiles
    let dcol = 1.5 * r; // pas entre colonnes (hexagones flat-top)
    let drow = 3.0_f32.sqrt() * r; // pas entre rangées
    let mut verts: Vec<Vertex> = Vec::new();
    let mut inds: Vec<u16> = Vec::new();
    let ncol = (largeur / dcol) as i32;
    let nrow = (longueur / drow) as i32;
    let x0 = -(ncol - 1) as f32 * dcol * 0.5; // centrage horizontal des colonnes
    for col in 0..ncol {
        let cx = x0 + col as f32 * dcol;
        let decal = if col % 2 == 0 { 0.0 } else { drow * 0.5 };
        for row in 0..=nrow {
            let cz = decal + row as f32 * drow;
            if cz > longueur {
                continue;
            }
            let centre = pied + w * cx + d * cz;
            let i0 = verts.len() as u16;
            verts.push(Vertex::new2(centre, vec2(0.5, 0.5), couleur));
            for k in 0..6 {
                let a = FRAC_PI_3 * k as f32;
                let p = centre + w * (hr * a.cos()) + d * (hr * a.sin());
                verts.push(Vertex::new2(p, vec2(0.0, 0.0), couleur));
            }
            for k in 0..6u16 {
                let a = i0 + 1 + k;
                let b = i0 + 1 + (k + 1) % 6;
                inds.extend_from_slice(&[i0, a, b, i0, b, a]); // double face
            }
        }
    }
    if !inds.is_empty() {
        draw_mesh(&Mesh { vertices: verts, indices: inds, texture: None });
    }
}

/// Repère local orthonormé (avant, droite, haut) déduit d'un axe principal.
fn repere(axe: Vec3) -> (Vec3, Vec3, Vec3) {
    let f = axe.normalize();
    let ref_haut = if f.dot(Vec3::Y).abs() > 0.95 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let droite = f.cross(ref_haut).normalize();
    let haut = droite.cross(f).normalize();
    (f, droite, haut)
}

/// Poutre en treillis à section carrée reliant `a` à `b` : quatre longerons et
/// des cadres/diagonales de baie répartis sur la longueur.
pub(crate) fn treillis(a: Vec3, b: Vec3, demi: f32, metal: Color, sombre: Color) {
    let axe = b - a;
    let long = axe.length();
    if long < 1e-4 {
        return;
    }
    let (_, d, h) = repere(axe);
    let coins = [
        d * -demi + h * -demi,
        d * demi + h * -demi,
        d * demi + h * demi,
        d * -demi + h * demi,
    ];
    for c in coins {
        cylindre(a + c, b + c, demi * 0.18, metal); // longerons
    }
    let baies = (long / (demi * 3.0)).round().max(1.0) as usize;
    for k in 0..=baies {
        let c = a + axe * (k as f32 / baies as f32);
        for w in 0..4 {
            cylindre(c + coins[w], c + coins[(w + 1) % 4], demi * 0.10, sombre); // cadre
        }
        if k < baies {
            let c2 = a + axe * ((k + 1) as f32 / baies as f32);
            cylindre(c + coins[0], c2 + coins[2], demi * 0.09, sombre); // diagonales
            cylindre(c + coins[1], c2 + coins[3], demi * 0.09, sombre);
        }
    }
}

/// Poutre en treillis à section **triangulaire** (3 longerons) — plus légère,
/// look « sonde ». Mêmes cadres/diagonales que la version carrée, en 3 côtés.
pub(crate) fn treillis_triangulaire(a: Vec3, b: Vec3, demi: f32, metal: Color, sombre: Color) {
    let axe = b - a;
    let long = axe.length();
    if long < 1e-4 {
        return;
    }
    let (_, d, h) = repere(axe);
    let coin = |deg: f32| {
        let r = deg.to_radians();
        d * (demi * r.cos()) + h * (demi * r.sin())
    };
    let coins = [coin(90.0), coin(210.0), coin(330.0)]; // 1 en haut, 2 en bas
    for c in coins {
        cylindre(a + c, b + c, demi * 0.16, metal); // longerons
    }
    let baies = (long / (demi * 3.0)).round().max(1.0) as usize;
    for k in 0..=baies {
        let c = a + axe * (k as f32 / baies as f32);
        for w in 0..3 {
            cylindre(c + coins[w], c + coins[(w + 1) % 3], demi * 0.10, sombre); // cadre
        }
        if k < baies {
            let c2 = a + axe * ((k + 1) as f32 / baies as f32);
            for w in 0..3 {
                cylindre(c + coins[w], c2 + coins[(w + 1) % 3], demi * 0.08, sombre); // diagonales
            }
        }
    }
}

/// Module pressurisé cylindrique centré sur `centre`, aligné sur `axe`, avec
/// deux anneaux de jonction sombres aux extrémités.
pub(crate) fn module(
    centre: Vec3,
    axe: Vec3,
    longueur: f32,
    rayon: f32,
    couleur: Color,
    sombre: Color,
) {
    let d = axe.normalize();
    let a = centre - d * (longueur * 0.5);
    let b = centre + d * (longueur * 0.5);
    cylindre(a, b, rayon, couleur);
    cylindre(a, a + d * 0.03, rayon * 1.06, sombre);
    cylindre(b - d * 0.03, b, rayon * 1.06, sombre);
}

/// Une pale solaire : deux lés séparés par une couture centrale, un cadre et
/// des nervures de cellules. Déployée depuis `racine` le long de `deploy`,
/// large de `largeur` selon `largeur_axe`.
pub(crate) fn pale_solaire(
    racine: Vec3,
    deploy: Vec3,
    largeur_axe: Vec3,
    longueur: f32,
    largeur: f32,
    cellules: usize,
    couleur: Color,
) {
    let d = deploy.normalize();
    let w = largeur_axe.normalize();
    let bord = Color::new(couleur.r * 0.45, couleur.g * 0.45, couleur.b * 0.45, 1.0);
    let coin = racine - w * (largeur * 0.5);
    let e1 = w * largeur;
    let e2 = d * longueur;
    panneau(coin, e1, e2, couleur);
    draw_line_3d(coin, coin + e1, bord);
    draw_line_3d(coin + e2, coin + e1 + e2, bord);
    draw_line_3d(coin, coin + e2, bord);
    draw_line_3d(coin + e1, coin + e1 + e2, bord);
    draw_line_3d(racine, racine + e2, bord); // couture entre les deux lés
    for n in 1..cellules {
        let m = coin + e2 * (n as f32 / cellules as f32);
        draw_line_3d(m, m + e1, bord); // nervures de cellules
    }
}

/// Paire d'ailes solaires : boîtier du joint rotatif au centre et deux pales
/// opposées le long de `deploy`, séparées par `ecart` (l'espace visible entre
/// les deux pales). Reproduit un « Photovoltaic Module » de l'ISS.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paire_ailes(
    base: Vec3,
    deploy: Vec3,
    largeur_axe: Vec3,
    ecart: f32,
    longueur: f32,
    largeur: f32,
    cellules: usize,
    couleur: Color,
    boitier: Color,
    sombre: Color,
) {
    draw_cube(base, Vec3::splat(0.26), None, boitier); // joint / gimbal
    draw_cube_wires(base, Vec3::splat(0.26), sombre);
    let d = deploy.normalize();
    for s in [-1.0_f32, 1.0] {
        let racine = base + d * (s * ecart);
        cylindre(base, racine, 0.04, sombre); // bras jusqu'au pied de pale
        pale_solaire(racine, d * s, largeur_axe, longueur, largeur, cellules, couleur);
    }
}

/// Radiateur thermique : panneau clair rainuré, orienté `deploy`/`largeur_axe`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn radiateur(
    base: Vec3,
    deploy: Vec3,
    largeur_axe: Vec3,
    longueur: f32,
    largeur: f32,
    lignes: usize,
    couleur: Color,
    sombre: Color,
) {
    let d = deploy.normalize();
    let w = largeur_axe.normalize();
    let coin = base - w * (largeur * 0.5);
    let e1 = w * largeur;
    let e2 = d * longueur;
    panneau(coin, e1, e2, couleur);
    for n in 1..lignes {
        let m = coin + e2 * (n as f32 / lignes as f32);
        draw_line_3d(m, m + e1, sombre);
    }
}
