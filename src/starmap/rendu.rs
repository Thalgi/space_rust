//! Rendu rétro de la Starmap : plan galactique en **nuage de points** (grille en diagonale),
//! tiges pointillées verticales, glyphes d'étoiles **pixel art** (billes crénelées) colorés
//! corps noir + noms Minitel. Binaires marquées d'une bille compagne ; survol = crochets de visée.

use super::{Etoile, Projection};
use crate::etoile::couleur_corps_noir;
use macroquad::prelude::*;

// ╔══════════════════════════════════════════════════════════════════════════════╗
// ║  COULEURS DE LA STARMAP — À RÉGLER ICI  (r, g, b, a) entre 0.0 et 1.0          ║
// ╚══════════════════════════════════════════════════════════════════════════════╝
/// Couleur des points de la grille (le plan galactique).
const COULEUR_GRILLE: Color = Color { r: 0.12, g: 0.55, b: 0.68, a: 0.55 };
/// Couleur des points des deux axes centraux (x = 0 et y = 0), un peu plus vifs.
const COULEUR_AXE: Color = Color { r: 0.20, g: 0.75, b: 0.85, a: 0.85 };
/// Couleur des tiges pointillées pied → étoile.
const COULEUR_TIGE: Color = Color { r: 0.55, g: 0.75, b: 0.80, a: 0.50 };
/// Couleur des crochets de visée au survol.
const COULEUR_VISEE: Color = Color { r: 0.90, g: 1.00, b: 1.00, a: 0.95 };
// NB : la couleur des BILLES d'étoiles n'est PAS ici — chaque étoile prend la couleur
//      corps-noir de sa température, voir `couleur_corps_noir(e.temp)` dans `dessiner()`.
//
/// Taille (rayon px) d'un point de la grille.
const TAILLE_POINT: f32 = 1.4;
/// Espacement des points le long d'une ligne (années-lumière).
const PAS_POINT: f32 = 0.5;
/// Côté d'un « pixel » pour les billes d'étoiles crénelées (look pixel art).
const TAILLE_PIXEL: f32 = 3.0;

/// Dessine toute la carte : grille -> tiges -> billes -> noms (dans cet ordre de profondeur).
/// `survol` = index de l'étoile pointée par le curseur (surbrillance), s'il y en a une.
pub fn dessiner(proj: &Projection, etoiles: &[Etoile], noms: bool, survol: Option<usize>) {
    grille(proj);
    for (i, e) in etoiles.iter().enumerate() {
        let p = e.position();
        // `decalage` = léger décalage d'affichage (dé-chevauchement Proxima/Alpha Cen).
        let pied = proj.pied(p) + e.decalage;
        let haut = proj.project(p) + e.decalage;
        // ► COULEUR DE LA BILLE : couleur corps-noir de la température de l'étoile.
        let c = couleur_corps_noir(e.temp);
        let vise = survol == Some(i);

        tige(pied, haut);
        // Repère au sol (pied), en pixel.
        pixel(pied, Color::new(c.x, c.y, c.z, 0.5));
        // Bille pixel art : halo crénelé + disque plein crénelé.
        disque_pixel(haut, e.rayon + 2.5, Color::new(c.x, c.y, c.z, 0.18));
        disque_pixel(haut, e.rayon, Color::new(c.x, c.y, c.z, 1.0));
        // Marqueur « double » : petite bille compagne pour les binaires.
        if e.double {
            let comp = haut + vec2(e.rayon + TAILLE_PIXEL, -(e.rayon * 0.5));
            disque_pixel(comp, (e.rayon * 0.55).max(TAILLE_PIXEL), Color::new(c.x, c.y, c.z, 0.95));
        }
        // Survol : quatre crochets de visée aux coins (look rétro pixel).
        if vise {
            crochets(haut, e.rayon + 6.0);
        }

        if noms || vise {
            let w = crate::police::mesure(e.nom, 14);
            crate::police::texte(
                e.nom,
                haut.x - w * 0.5,
                haut.y - e.rayon - 8.0,
                14.0,
                Color::new(0.7, 0.9, 0.9, 0.9),
            );
        }
    }
}

/// Plan galactique dessiné en **nuage de points** : les lignes de la grille (1 case = 2 al)
/// deviennent des chapelets de points ; les deux axes centraux ressortent (COULEUR_AXE).
fn grille(proj: &Projection) {
    let r = 12.0_f32; // demi-emprise (al)
    let pas = 2.0_f32; // écart entre lignes (al)

    let mut x = -r;
    while x <= r + 0.01 {
        let col = if x.abs() < 0.01 { COULEUR_AXE } else { COULEUR_GRILLE };
        semer(proj, vec3(x, -r, 0.0), vec3(x, r, 0.0), col);
        x += pas;
    }
    let mut y = -r;
    while y <= r + 0.01 {
        let col = if y.abs() < 0.01 { COULEUR_AXE } else { COULEUR_GRILLE };
        semer(proj, vec3(-r, y, 0.0), vec3(r, y, 0.0), col);
        y += pas;
    }
}

/// Sème des points réguliers le long du segment monde `a`→`b` (projetés à l'écran).
fn semer(proj: &Projection, a: Vec3, b: Vec3, col: Color) {
    let n = (a.distance(b) / PAS_POINT).ceil().max(1.0) as i32;
    for i in 0..=n {
        let w = a.lerp(b, i as f32 / n as f32);
        let p = proj.project(w);
        draw_circle(p.x, p.y, TAILLE_POINT, col);
    }
}

/// Tige pointillée verticale reliant le pied `(x,y,0)` à l'étoile `(x,y,z)`.
fn tige(pied: Vec2, haut: Vec2) {
    let n = 16;
    for i in 0..n {
        if i % 2 == 0 {
            let a = pied.lerp(haut, i as f32 / n as f32);
            let b = pied.lerp(haut, (i + 1) as f32 / n as f32);
            draw_line(a.x, a.y, b.x, b.y, 1.0, COULEUR_TIGE);
        }
    }
}

/// Un « pixel » carré aligné sur la grille globale de pixels (côté `TAILLE_PIXEL`).
fn pixel(p: Vec2, col: Color) {
    let px = TAILLE_PIXEL;
    draw_rectangle((p.x / px).floor() * px, (p.y / px).floor() * px, px, px, col);
}

/// Disque « pixel art » : remplit les cellules carrées (grille globale de `TAILLE_PIXEL`)
/// dont le centre tombe dans le rayon `r`. Look crénelé cohérent entre toutes les étoiles.
fn disque_pixel(centre: Vec2, r: f32, col: Color) {
    let px = TAILLE_PIXEL;
    let r2 = r * r;
    let gx0 = ((centre.x - r) / px).floor() as i32;
    let gx1 = ((centre.x + r) / px).ceil() as i32;
    let gy0 = ((centre.y - r) / px).floor() as i32;
    let gy1 = ((centre.y + r) / px).ceil() as i32;
    for gy in gy0..gy1 {
        for gx in gx0..gx1 {
            let dx = (gx as f32 + 0.5) * px - centre.x;
            let dy = (gy as f32 + 0.5) * px - centre.y;
            if dx * dx + dy * dy <= r2 {
                draw_rectangle(gx as f32 * px, gy as f32 * px, px, px, col);
            }
        }
    }
}

/// Quatre crochets de visée (pixels aux coins d'un carré de demi-côté `b`) — surbrillance survol.
fn crochets(centre: Vec2, b: f32) {
    for sx in [-1.0_f32, 1.0] {
        for sy in [-1.0_f32, 1.0] {
            pixel(vec2(centre.x + sx * b, centre.y + sy * b), COULEUR_VISEE);
        }
    }
}
