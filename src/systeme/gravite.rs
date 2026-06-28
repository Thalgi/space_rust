use super::{Systeme, G};
use crate::astre::Categorie;
use macroquad::prelude::*;

const SOFT: f32 = 0.05; // adoucissement (évite les singularités)

impl Systeme {
    /// Un pas d'intégration leapfrog (kick-drift-kick) : bien conservatif pour des orbites.
    pub(super) fn gravite(&mut self, h: f32) {
        let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
        let mass: Vec<f32> = self.astres.iter().map(|a| a.corps().masse).collect();

        let a1 = accelerations(&pos, &mass);
        for (i, a) in self.astres.iter_mut().enumerate() {
            if a.categorie() != Categorie::Planete {
                continue; // l'étoile reste fixe à l'origine ; la ceinture s'anime seule
            }
            let c = a.corps_mut();
            c.vitesse += a1[i] * (0.5 * h);
            let v = c.vitesse;
            c.position += v * h;
        }

        let pos2: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
        let a2 = accelerations(&pos2, &mass);
        for (i, a) in self.astres.iter_mut().enumerate() {
            if a.categorie() != Categorie::Planete {
                continue;
            }
            a.corps_mut().vitesse += a2[i] * (0.5 * h);
        }
    }
}

fn accelerations(pos: &[Vec3], mass: &[f32]) -> Vec<Vec3> {
    let n = pos.len();
    let mut acc = vec![Vec3::ZERO; n];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let d = pos[j] - pos[i];
                let r2 = d.length_squared() + SOFT * SOFT;
                acc[i] += d * (G * mass[j] / (r2 * r2.sqrt()));
            }
        }
    }
    acc
}
