//! Orbite de Kepler analytique : `position = f(t)` (socle « sur rails »).
//!
//! Un corps sur rails ne subit aucune intégration N-corps : sa position est
//! l'évaluation exacte de son ellipse à l'instant `t`. Stable, déterministe,
//! et le temps peut être accéléré/sauté librement (jeu incrémental).
//!
//! Convention de base : `a1` = vecteur unité vers le **périastre**, `q` = vecteur
//! unité perpendiculaire **dans le sens du mouvement** au périastre. Le plan
//! orbital est engendré par (a1, q). Foyer (l'astre central) à l'origine du repère
//! local ; l'appelant ajoute la position du foyer.

use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy)]
pub struct Orbite {
    pub a: f32,   // demi-grand axe (unités monde)
    pub e: f32,   // excentricité (0 = cercle)
    pub a1: Vec3, // unité vers le périastre
    pub q: Vec3,  // unité perpendiculaire (sens du mouvement)
    pub n: f32,   // moyen mouvement = sqrt(mu / a^3)   (G = 1)
    pub m0: f32,  // anomalie moyenne à t = 0 (0 = au périastre)
}

impl Orbite {
    /// Construit depuis les éléments usuels. `mu` = G·masse_centrale.
    pub fn new(a: f32, e: f32, a1: Vec3, q: Vec3, mu: f32, m0: f32) -> Self {
        let a = a.max(1e-4);
        let n = (mu / (a * a * a)).sqrt();
        Self {
            a,
            e: e.clamp(0.0, 0.99),
            a1: a1.normalize_or_zero(),
            q: q.normalize_or_zero(),
            n,
            m0,
        }
    }

    /// Variante où le moyen mouvement `n` est fourni **directement** (au lieu d'être
    /// dérivé de `mu`/`a`). Utile pour les orbites barycentriques d'un binaire : les
    /// deux étoiles ont des demi-grands axes différents mais **partagent le même `n`**
    /// (même période), donc restent diamétralement opposées.
    pub fn avec_n(a: f32, e: f32, a1: Vec3, q: Vec3, n: f32, m0: f32) -> Self {
        Self {
            a: a.max(1e-4),
            e: e.clamp(0.0, 0.99),
            a1: a1.normalize_or_zero(),
            q: q.normalize_or_zero(),
            n,
            m0,
        }
    }

    /// Position **et** vitesse relatives au foyer, à l'instant `t`.
    /// (La vitesse sert au hand-off vers le mode N-corps.)
    pub fn etat(&self, t: f64) -> (Vec3, Vec3) {
        // Anomalie moyenne repliée dans [0, 2π) pour garder la précision dans le temps.
        let m = (self.m0 as f64 + self.n as f64 * t).rem_euclid(std::f64::consts::TAU) as f32;
        let ea = resoudre_kepler(m, self.e);
        let (se, ce) = ea.sin_cos();
        let b = self.a * (1.0 - self.e * self.e).max(0.0).sqrt();

        // Position dans le repère périfocal (a1, q).
        let x = self.a * (ce - self.e);
        let y = b * se;
        let pos = self.a1 * x + self.q * y;

        // Vitesse : dérivée, avec Ė = n / (1 - e·cos E).
        let edot = self.n / (1.0 - self.e * ce).max(1e-4);
        let vx = -self.a * se * edot;
        let vy = b * ce * edot;
        let vel = self.a1 * vx + self.q * vy;

        (pos, vel)
    }

    /// Position seule (cas courant : rendu sur rails).
    pub fn position(&self, t: f64) -> Vec3 {
        self.etat(t).0
    }

    /// Polyligne de l'ellipse (relative au foyer), pour tracer l'orbite.
    /// **Nombre d'échantillons d'une trace d'orbite.**
    ///
    /// C'est le PLAFOND de densité du semis de points : le tracé peut en sauter
    /// (`dessiner_points` règle l'espacement à l'écran), jamais en inventer. À 96
    /// — la valeur d'origine — un zoom serré ne laissait qu'une poignée de marques
    /// sur tout l'écran, et le cercle ne se lisait plus.
    ///
    /// 512 : ~6 Ko par orbite, et de quoi tenir un semis serré de très près.
    pub const N_TRACE: usize = 512;

    pub fn polyligne(&self, n_pts: usize) -> Vec<Vec3> {
        let b = self.a * (1.0 - self.e * self.e).max(0.0).sqrt();
        (0..n_pts)
            .map(|k| {
                let ea = k as f32 / n_pts as f32 * TAU;
                let (se, ce) = ea.sin_cos();
                self.a1 * (self.a * (ce - self.e)) + self.q * (b * se)
            })
            .collect()
    }
}

/// Résout l'équation de Kepler `M = E − e·sin E` pour l'anomalie excentrique `E`
/// (Newton-Raphson, convergence en quelques itérations pour e < 1).
fn resoudre_kepler(m: f32, e: f32) -> f32 {
    let mut ea = if e < 0.8 { m } else { PI };
    for _ in 0..6 {
        let f = ea - e * ea.sin() - m;
        let fp = 1.0 - e * ea.cos();
        ea -= f / fp;
    }
    ea
}

#[cfg(test)]
mod tests_trace {
    use super::*;

    /// **La réserve d'échantillons doit rester haute.**
    ///
    /// Elle ne se voit pas de loin (le tracé en saute), mais elle plafonne la
    /// densité de près — et c'est exactement là que le défaut s'était vu : « pas
    /// assez de points pour visualiser le cercle ». Le seuil est posé comme une
    /// borne, pas comme une copie de la valeur : baisser N_TRACE sous 256
    /// ramènerait le défaut.
    #[test]
    fn la_trace_garde_de_quoi_faire_un_cercle_de_pres() {
        const _: () = assert!(
            Orbite::N_TRACE >= 256,
            "N_TRACE trop bas : plus assez de points pour lire un cercle en zoom serré"
        );
        // Et le tirage rend bien ce qu'on lui demande.
        let o = Orbite::new(10.0, 0.2, Vec3::X, Vec3::Z, 1.0, 0.0);
        assert_eq!(o.polyligne(Orbite::N_TRACE).len(), Orbite::N_TRACE);
    }
}
