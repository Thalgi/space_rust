use super::{Soleil, MAX_TACHES};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum EtatBoucle {
    Montee,
    Tenue,
    Rupture,
    Retombee,
}

/// Tache active (région sombre) sur la surface, repère "surface" (tourne avec le soleil).
pub(super) struct Tache {
    pub dir: Vec3,
    pub taille: f32, // rayon angulaire (rad)
    pub age: f32,
    pub vie_max: f32,
    pub intensite: f32,
}

/// Boucle coronale ancrée par deux pieds (repère surface).
pub(super) struct Boucle {
    pub sa: Vec3,
    pub sb: Vec3,
    pub apex: f32,
    pub apex_max: f32,
    pub skew: f32, // décalage latéral du sommet -> arches asymétriques
    pub etat: EtatBoucle,
    pub timer: f32,
    pub tenue_max: f32,
    pub rompt: bool,
    pub intensite: f32,
    pub apex_vel: f32, // vitesse d'expansion lors de l'éjection (corde de flux)
}

impl Soleil {
    /// Crée une boucle à un endroit aléatoire de la surface (éruptions dispersées).
    pub(super) fn declencher_boucle(&mut self) {
        let centre = dir_aleatoire();
        let taille_spot: f32 = gen_range(0.12, 0.26);

        let p = self.puissance;
        let f = self.forme;
        let j = 0.3 + 1.7 * self.alea; // amplitude de l'aléa sur la forme

        // Deux pieds qui enjambent le centre de la tache.
        // "forme" donne l'écart de base ; "aléa" ajoute du jitter.
        let rnd = dir_aleatoire();
        let tang = (rnd - centre * centre.dot(rnd)).normalize_or_zero();
        let half: f32 =
            (taille_spot * (0.4 + f * 0.9 + gen_range(-0.4, 0.6) * j)).clamp(0.05, 1.4);
        let sa = (centre * half.cos() + tang * half.sin()).normalize();
        let sb = (centre * half.cos() - tang * half.sin()).normalize();

        // Puissance -> hauteur ; forme -> biais ; aléa -> dispersion hauteur/asymétrie.
        let apex_base = 0.25 + 1.3 * p;
        let apex_max: f32 =
            (apex_base * (1.0 + f * 0.3) * (1.0 + gen_range(-0.5, 0.6) * j)).clamp(0.1, 2.6);
        let skew = gen_range(-0.8, 0.8) * j * apex_max;

        // Plus la boucle est grande, plus elle part dans l'espace.
        let prob = smoothstep(0.3, 1.3, apex_max);
        let rompt = gen_range(0.0, 1.0) < prob;

        self.boucles.push(Boucle {
            sa,
            sb,
            apex: 0.0,
            apex_max,
            skew,
            etat: EtatBoucle::Montee,
            timer: 0.0,
            tenue_max: gen_range(0.6, 1.8) + gen_range(0.0, 1.0) * j,
            rompt,
            intensite: 1.0,
            apex_vel: 0.0,
        });
    }

    pub(super) fn maj(&mut self, dt: f32) {
        self.temps += dt;

        // --- Taches : apparition / fondu / disparition ---
        if self.temps >= self.prochaine_tache && self.taches.len() < MAX_TACHES {
            self.taches.push(Tache {
                dir: dir_aleatoire(),
                taille: gen_range(0.12, 0.28),
                age: 0.0,
                vie_max: gen_range(8.0, 16.0),
                intensite: 0.0,
            });
            self.prochaine_tache = self.temps + gen_range(3.0, 7.0);
        }
        for t in &mut self.taches {
            t.age += dt;
            let fade_in = (t.age / 1.5).clamp(0.0, 1.0);
            let fade_out = ((t.vie_max - t.age) / 2.0).clamp(0.0, 1.0);
            t.intensite = fade_in.min(fade_out);
        }
        self.taches.retain(|t| t.age < t.vie_max);

        // --- Déclenchement des boucles (sur les taches) ---
        if self.temps >= self.prochaine_eruption {
            // Plusieurs éruptions par cycle (dispersées), d'autant plus que freq est haut.
            let n_loops = 1 + (self.freq * 2.5) as usize; // 1..=3
            for _ in 0..n_loops {
                self.declencher_boucle();
            }
            // Fréquence haute -> intervalle court.
            let moy = 3.5 - 3.2 * self.freq;
            self.prochaine_eruption = self.temps + (gen_range(0.5, 1.5) * moy).max(0.12);
        }

        // --- Boucles : machine à états ---
        for b in &mut self.boucles {
            b.timer += dt;
            match b.etat {
                EtatBoucle::Montee => {
                    b.apex += (b.apex_max - b.apex) * (dt * 2.5).min(1.0);
                    if b.apex >= b.apex_max * 0.96 {
                        b.etat = EtatBoucle::Tenue;
                        b.timer = 0.0;
                    }
                }
                EtatBoucle::Tenue => {
                    if b.timer >= b.tenue_max {
                        if b.rompt {
                            // Début d'éjection : l'arche commence à s'étendre vers l'espace.
                            b.apex_vel = 0.4 + 0.5 * b.apex_max;
                            b.etat = EtatBoucle::Rupture;
                        } else {
                            b.etat = EtatBoucle::Retombee;
                        }
                        b.timer = 0.0;
                    }
                }
                EtatBoucle::Rupture => {
                    // Corde de flux qui s'éjecte : l'arche gonfle en accélérant,
                    // les pieds restent ancrés (les jambes s'étirent), et elle s'estompe.
                    b.apex_vel += 1.2 * dt; // accélération
                    b.apex += b.apex_vel * dt;
                    b.intensite -= dt * 0.45;
                }
                EtatBoucle::Retombee => {
                    b.apex -= b.apex * (dt * 1.5).min(1.0) + dt * 0.05;
                    b.intensite -= dt * 0.8;
                }
            }
        }
        self.boucles.retain(|b| b.intensite > 0.0 && b.apex < 8.0);
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(super) fn dir_aleatoire() -> Vec3 {
    loop {
        let p = vec3(
            gen_range(-1.0, 1.0),
            gen_range(-1.0, 1.0),
            gen_range(-1.0, 1.0),
        );
        let l2 = p.length_squared();
        if l2 > 0.0001 && l2 <= 1.0 {
            return p / l2.sqrt();
        }
    }
}
