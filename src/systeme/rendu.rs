use super::Systeme;
use crate::astre::{CameraInfo, Categorie};
use macroquad::prelude::*;

impl Systeme {
    pub fn draw(&mut self, cam: &CameraInfo, montrer_orbites: bool, montrer_zone: bool) {
        // L'étoile sert de source de lumière pour éclairer les planètes.
        let etoile = self
            .astres
            .iter()
            .find(|a| a.categorie() == Categorie::Etoile);
        // Étoile présente -> elle éclaire. Sinon, lumière de secours (planète isolée).
        let secours = self.lumiere_secours();
        let light = etoile
            .map(|a| a.corps().position)
            .or(secours.map(|(p, _)| p))
            .unwrap_or(Vec3::ZERO);
        let light_color = etoile
            .and_then(|a| a.lumiere())
            .or(secours.map(|(_, c)| c))
            .unwrap_or(Vec3::ONE);

        // Zone habitable (anneaux verts) — change selon le type d'étoile.
        if montrer_zone {
            if let Some((interne, externe)) = etoile.and_then(|a| a.zone_viable()) {
                let vert = Color::new(0.3, 0.9, 0.4, 0.45);
                dessiner_cercle(light, interne, vert);
                dessiner_cercle(light, externe, vert);
            }
        }

        // Trajectoires des planètes (ellipses).
        if montrer_orbites {
            let col = Color::new(0.4, 0.5, 0.75, 0.6);
            for a in &self.astres {
                let pts = a.orbite();
                for i in 0..pts.len() {
                    let p = light + pts[i];
                    let qn = light + pts[(i + 1) % pts.len()];
                    draw_line_3d(p, qn, col);
                }
            }
        }

        let mut c = *cam;
        c.light_pos = light;
        c.light_color = light_color;
        for a in &mut self.astres {
            a.draw(&c);
        }
    }
}

/// Trace un cercle dans le plan xz (orbite ou zone habitable).
fn dessiner_cercle(centre: Vec3, r: f32, col: Color) {
    let n = 96;
    let mut prec = centre + vec3(r, 0.0, 0.0);
    for i in 1..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let p = centre + vec3(r * a.cos(), 0.0, r * a.sin());
        draw_line_3d(prec, p, col);
        prec = p;
    }
}
