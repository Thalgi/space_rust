//! **Le défilement des galeries** : la molette déplace une cible, la vue y
//! glisse.
//!
//! ⚠️ **Né d'un défaut** (2026-10-20) : *« plus on rajoute d'étoiles, plus le
//! scroll est agressif »*. miniquad transmet la molette **brute** — sous
//! Windows, ±120 par cran (`WHEEL_DELTA`), et non ±1. Les galeries des étoiles
//! et des comètes multipliaient ce 120 par 48 px : **5 760 px par cran**,
//! bornés seulement par le bout de la liste. Avec peu de cases, la borne
//! cachait tout ; chaque étoile ajoutée allongeait la liste, donc le saut.
//! La caméra de la Skymap y échappait parce qu'elle ne lit que le signe.

use macroquad::prelude::*;

/// **Pixels par cran de molette.** Celui de la galerie tellurique, qui
/// défilait déjà en douceur : une même molette ne doit pas avoir deux
/// sensibilités selon la galerie.
pub const PAS: f32 = 84.0;

/// **Crans de molette de cette frame**, normalisés : ±1 par cran, quelle que
/// soit la plateforme.
pub fn crans() -> f32 {
    normaliser(mouse_wheel().1)
}

/// La valeur brute de l'OS, ramenée en crans.
fn normaliser(brut: f32) -> f32 {
    if cfg!(target_os = "windows") {
        brut / 120.0
    } else {
        brut
    }
}

/// Rapproche `pos` de `cible` d'une part qui ne dépend que de `dt` — donc du
/// temps, pas du nombre d'images. Colle à la cible sous 0,3 px : en pixel art,
/// une approche sans fin ferait trembler l'image d'un pixel.
pub fn glisser(pos: f32, cible: f32, dt: f32) -> f32 {
    let p = pos + (cible - pos) * (1.0 - (-dt * 10.0).exp());
    if (cible - p).abs() < 0.3 { cible } else { p }
}

/// **Un défilement doux** : où l'on va, et où l'on en est.
#[derive(Default)]
pub struct Defilement {
    pos: f32,
    cible: f32,
}

impl Defilement {
    /// Une frame : la molette déplace la cible, bornée à `[0 ; max]`, et la
    /// vue y glisse. Rend la position à afficher.
    pub fn maj(&mut self, max: f32, dt: f32) -> f32 {
        self.avancer(crans(), max, dt)
    }

    fn avancer(&mut self, crans: f32, max: f32, dt: f32) -> f32 {
        self.cible = (self.cible - crans * PAS).clamp(0.0, max.max(0.0));
        self.pos = glisser(self.pos, self.cible, dt);
        self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Un cran déplace la vue de `PAS`, quelle que soit la longueur de la
    /// liste.** C'était tout le défaut : le saut grandissait avec elle.
    #[test]
    fn un_cran_avance_d_un_pas_quelle_que_soit_la_liste() {
        for max in [500.0_f32, 5_000.0, 50_000.0] {
            let mut d = Defilement::default();
            d.avancer(-1.0, max, 0.0);
            for _ in 0..200 {
                d.avancer(0.0, max, 1.0 / 60.0);
            }
            assert!((d.pos - PAS).abs() < 1e-3, "liste de {max} px : un cran mène à {}", d.pos);
        }
    }

    /// **Sous Windows, 120 est un cran** — la valeur que l'OS envoie.
    #[test]
    fn la_molette_brute_devient_un_cran() {
        let un_cran_brut = if cfg!(target_os = "windows") { 120.0 } else { 1.0 };
        assert_eq!(normaliser(un_cran_brut), 1.0);
        assert_eq!(normaliser(-un_cran_brut), -1.0);
    }

    /// **La vue glisse sans dépasser**, et finit collée à la cible.
    #[test]
    fn la_vue_glisse_sans_depasser() {
        let mut p = 0.0;
        for _ in 0..120 {
            let suivant = glisser(p, 100.0, 1.0 / 60.0);
            assert!(suivant >= p && suivant <= 100.0, "{p} -> {suivant}");
            p = suivant;
        }
        assert_eq!(p, 100.0, "la vue n'a pas rejoint sa cible");
    }
}
