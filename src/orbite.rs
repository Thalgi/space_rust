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
    // Départ à π pour les fortes excentricités : c'est le choix sûr, la
    // tangente y est douce et Newton ne part pas de l'autre côté du périastre.
    let mut ea = if e < 0.8 { m } else { PI };
    for _ in 0..MAX_NEWTON {
        let fp = 1.0 - e * ea.cos();
        // `fp` tend vers `1 - e`, donc vers 0 quand `e` tend vers 1 : la garde
        // évite une division qui enverrait l'astre à l'infini.
        let pas = (ea - e * ea.sin() - m) / fp.max(1e-6);
        ea -= pas;
        if pas.abs() < 1e-6 {
            break;
        }
    }
    ea
}

/// **Plafond d'itérations de Newton**, et sortie dès convergence.
///
/// ⚠️ **Six itérations fixes ne suffisaient pas aux comètes**, et c'est mesuré :
/// l'écart angulaire après six passes vaut 0 jusqu'à `e = 0,90`, puis
/// **0,021 rad** à `e = 0,97` et **0,114 rad** — 6,5° — à `e = 0,99`. Or Halley
/// est à 0,967 et Hale-Bopp à 0,995 : les comètes vivent exactement là où le
/// solveur décrochait, et l'erreur est maximale **au périastre**, là où l'astre
/// file le plus vite et où un décalage se voit le plus.
///
/// La boucle sort dès que la correction passe sous le seuil, donc une planète
/// quasi circulaire coûte toujours deux passes : le plafond ne se paie que là
/// où il sert.
const MAX_NEWTON: usize = 24;

#[cfg(test)]
mod tests_kepler {
    use super::*;

    /// **L'équation de Kepler est résolue, y compris aux excentricités de
    /// comète.**
    ///
    /// ⚠️ C'est un défaut **mesuré**, pas supposé. Le solveur faisait six
    /// itérations fixes : exact jusqu'à `e = 0,90`, puis l'écart angulaire
    /// atteignait 0,021 rad à `e = 0,97` et **0,114 rad — 6,5°** à `e = 0,99`.
    /// Halley est à 0,967, Hale-Bopp à 0,995 : les comètes du lot 9 vivent
    /// précisément là où il décrochait.
    ///
    /// Et il décrochait **au périastre**, là où l'astre file le plus vite : un
    /// corps posé 6° à côté sur la portion la plus rapide de son orbite, qu'on
    /// aurait mis sur le compte du rendu.
    ///
    /// Le test balaie l'anomalie moyenne entière, parce que l'erreur est très
    /// localisée : une poignée de valeurs choisies à la main l'auraient ratée.
    #[test]
    fn kepler_converge_jusqu_aux_excentricites_de_comete() {
        for e in [0.0_f32, 0.2, 0.5, 0.8, 0.9, 0.95, 0.97, 0.99] {
            let mut pire = 0.0_f32;
            for k in 0..4000 {
                let m = TAU * k as f32 / 4000.0;
                let ea = resoudre_kepler(m, e);
                // Le résidu de l'équation elle-même : E − e·sin E − M.
                pire = pire.max((ea - e * ea.sin() - m).abs());
            }
            assert!(
                pire < 1e-4,
                "e = {e} : résidu de {pire} rad — l'astre est posé à côté de son orbite"
            );
        }
    }

    /// **Une orbite très excentrique reste une ellipse fermée.**
    ///
    /// ⚠️ Le vrai risque d'un solveur qui ne converge pas n'est pas une erreur
    /// diffuse : c'est un **saut**. Le test suit la position tout au long d'une
    /// révolution et refuse qu'elle bondisse d'un pas à l'autre — un astre qui
    /// se téléporte au périastre est ce qu'on aurait vu à l'écran.
    #[test]
    fn une_orbite_de_comete_ne_saute_pas_au_periastre() {
        let mu = 1000.0_f32;
        let o = Orbite::new(40.0, 0.96, Vec3::X, Vec3::Z, mu, 0.0);
        let periode = TAU / o.n;
        let pas = periode / 2000.0;
        let mut precedent = o.position(0.0);
        let mut plus_grand_saut = 0.0_f32;
        for k in 1..=2000 {
            let p = o.position((k as f32 * pas) as f64);
            plus_grand_saut = plus_grand_saut.max((p - precedent).length());
            precedent = p;
        }
        // Au périastre la comète file : le pas y est forcément le plus grand.
        // Ce qu'on interdit, c'est qu'il dépasse ce que la vitesse explique.
        // Vitesse au périastre ≈ sqrt(mu/a · (1+e)/(1−e)).
        let v_max = (mu / o.a * (1.0 + o.e) / (1.0 - o.e)).sqrt();
        let attendu = v_max * pas;
        assert!(
            plus_grand_saut < attendu * 1.5,
            "saut de {plus_grand_saut} pour {attendu} attendu : le solveur décroche"
        );
        // Et l'orbite se referme : après une révolution, on est revenu.
        let retour = (o.position(periode as f64) - o.position(0.0)).length();
        assert!(retour < o.a * 1e-3, "l'orbite ne se referme pas : {retour}");
    }
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
