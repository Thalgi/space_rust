//! Profil zonal 1D des géantes gazeuses (V2 phase 2, CONCEPTION_GAZEUSES_V2.md § 3).
//!
//! Le SQUELETTE des bandes est précalculé ici (CPU, 1× par planète, < 1 ms) ;
//! le shader n'habille plus que la matière (volutes, grain, frontières
//! ondulées). Texture 256×1 RGBA, indexée par sin(latitude) — le shader
//! échantillonne `vec2(dot(d, axe) * 0.5 + 0.5, 0.5)`, sans asin.
//!
//! Canaux :
//! - R = `u(φ)`  : vitesse zonale signée (0.5 = immobile), jets est/ouest
//!   alternés + jet équatorial prograde dominant. Sert à l'advection
//!   différentielle (phase 3) et à la dérive des vortex (phase 4).
//! - G = `b(φ)`  : type de bande, 0 = belt sombre .. 1 = zone claire. Dérivé
//!   de la vorticité du profil (anticyclonique = zone), comme sur Jupiter.
//! - B = `s(φ)`  : cisaillement local 0..1 (|du/dφ| normalisé) -> festons et
//!   turbulence concentrés aux flancs des jets.
//! - A = libre (255).

use super::apparence::Apparence;
use macroquad::prelude::*;

pub const N_ZONAL: usize = 256;
/// Latitude (radians) de fin du régime de bandes (~70°) : au-delà, extinction
/// douce — le shader passe au régime polaire.
const LAT_MAX: f32 = 1.22;

/// Hash déterministe 0..1 (stable, indépendant du RNG global).
fn h01(seed: f32, i: f32) -> f32 {
    let v = ((seed * 12.9898 + i * 78.233).sin()) * 43758.5453_f32;
    v - v.floor()
}

fn gauss(x: f32, w: f32) -> f32 {
    let d = x / w;
    (-d * d).exp()
}

/// Profil calculé (avant encodage texture) : `u` normalisé -1..1 (même échelle
/// que le canal R de la texture), `b`/`s` 0..1. Indexé par sin(φ) sur N_ZONAL
/// texels. Consommé par `generer_zonal` ET par le placement des vortex.
pub(super) struct Profil {
    pub u: Vec<f32>,
    pub b: Vec<f32>,
    pub s: Vec<f32>,
}

impl Profil {
    fn idx(sphi: f32) -> usize {
        (((sphi.clamp(-1.0, 1.0) * 0.5 + 0.5) * (N_ZONAL as f32 - 1.0)).round()) as usize
    }
    /// Vitesse de jet normalisée (-1..1) à la latitude sin(φ) donnée.
    pub fn u_at(&self, sphi: f32) -> f32 {
        self.u[Self::idx(sphi)]
    }
    /// Type de bande (0 belt sombre .. 1 zone claire) à la latitude sin(φ).
    pub fn b_at(&self, sphi: f32) -> f32 {
        self.b[Self::idx(sphi)]
    }
}

/// Calcule le profil zonal complet d'une gazeuse depuis son `Apparence`
/// (`nb_bandes`, `jets_force`, `zonal_asym`, `zonal_flou`, `seed`).
pub(super) fn profil(a: &Apparence) -> Profil {
    let n = N_ZONAL;
    let paires = a.nb_bandes.round().clamp(1.0, 10.0);
    let seed = a.seed;
    let force = a.jets_force.clamp(0.0, 1.5);
    let pas = LAT_MAX / (paires + 1.0); // espacement nominal des jets

    // --- 1) u(φ) : somme de gaussiennes. ---
    let mut u = vec![0.0f32; n];
    for (x, ux) in u.iter_mut().enumerate() {
        let sphi = ((x as f32 + 0.5) / n as f32) * 2.0 - 1.0;
        let phi = sphi.clamp(-1.0, 1.0).asin();
        // Jet équatorial prograde, large (l'EZ de Jupiter).
        let mut v = force * gauss(phi, pas * 1.3);
        // Paires de jets alternés vers les pôles, jitter INDÉPENDANT par
        // hémisphère (zonal_asym) -> nord et sud ne sont plus des miroirs.
        for j in 0..paires as i32 {
            let fj = j as f32;
            for (hi, hs) in [(0.0f32, 1.0f32), (1.0, -1.0)] {
                let jit_l = (h01(seed, fj * 2.9 + hi * 17.0) - 0.5) * a.zonal_asym;
                let jit_a = 1.0 - h01(seed, fj * 7.3 + hi * 23.0) * a.zonal_asym * 0.7;
                let lat_j = (fj + 1.0 + jit_l * 0.8) * pas;
                let signe = if j % 2 == 0 { -1.0 } else { 1.0 }; // rétrograde au flanc de l'EZ
                let amp = force * signe * jit_a * (1.0 - fj / (paires + 2.0));
                v += amp * gauss(phi - hs * lat_j, pas * 0.55);
            }
        }
        // Extinction douce vers les pôles (le régime polaire prend le relais).
        let fondu = 1.0 - ((phi.abs() - LAT_MAX * 0.92) / (LAT_MAX * 0.25)).clamp(0.0, 1.0);
        *ux = v * fondu;
    }

    // --- 2) dérivée du/dx -> vorticité et cisaillement. ---
    let mut du = vec![0.0f32; n];
    for x in 1..n - 1 {
        du[x] = u[x + 1] - u[x - 1];
    }
    du[0] = du[1];
    du[n - 1] = du[n - 2];
    let du_max = du.iter().fold(1e-6f32, |m, v| m.max(v.abs()));
    let u_max = u.iter().fold(1e-6f32, |m, v| m.max(v.abs()));

    // b(φ) : anticyclonique = zone claire. En β-plan, anticyclonique au nord
    // = du/dφ > 0, au sud = du/dφ < 0 -> A = du/dφ * signe(φ).
    let mut b = vec![0.5f32; n];
    let mut s = vec![0.0f32; n];
    for x in 0..n {
        let sphi = ((x as f32 + 0.5) / n as f32) * 2.0 - 1.0;
        let a_vort = du[x] / du_max * if sphi >= 0.0 { 1.0 } else { -1.0 };
        b[x] = 0.5 + 0.5 * (2.2 * a_vort).tanh();
        // Zone Équatoriale : claire dès que le jet équatorial existe.
        let phi = sphi.clamp(-1.0, 1.0).asin();
        let ez = gauss(phi, pas * 1.1) * (force * 1.5).min(1.0);
        b[x] = b[x].max(ez * 0.85 + b[x] * 0.15);
        s[x] = (du[x].abs() / du_max).sqrt();
    }

    // --- 3) flou (zonal_flou) : moyenne glissante -> frontières douces
    //     (classes voilées, sub-Neptunes) ; atténue aussi le cisaillement. ---
    let rayon = (a.zonal_flou.clamp(0.0, 1.0) * 28.0) as i32;
    if rayon > 0 {
        let flou1 = |src: &[f32]| -> Vec<f32> {
            (0..n)
                .map(|x| {
                    let (mut somme, mut nb) = (0.0f32, 0.0f32);
                    for dx in -rayon..=rayon {
                        let xi = (x as i32 + dx).clamp(0, n as i32 - 1) as usize;
                        somme += src[xi];
                        nb += 1.0;
                    }
                    somme / nb
                })
                .collect()
        };
        b = flou1(&b);
        s = flou1(&s).iter().map(|v| v * (1.0 - a.zonal_flou * 0.7)).collect();
    }

    // --- 4) normalisation de u (même échelle que le canal R). ---
    for ux in u.iter_mut() {
        *ux /= u_max;
    }
    Profil { u, b, s }
}

/// Génère la texture de profil zonal (encodage RGBA8 du `Profil`) et la borne
/// polaire `pole_lat` : sin(latitude) où le régime de bandes cède au régime
/// polaire — juste après la dernière paire de jets (uniform du shader, phase 5).
pub fn generer_zonal(a: &Apparence) -> (Texture2D, f32) {
    let p = profil(a);
    let mut octets = Vec::with_capacity(N_ZONAL * 4);
    for x in 0..N_ZONAL {
        octets.push(((0.5 + 0.5 * p.u[x]).clamp(0.0, 1.0) * 255.0) as u8);
        octets.push((p.b[x].clamp(0.0, 1.0) * 255.0) as u8);
        octets.push((p.s[x].clamp(0.0, 1.0) * 255.0) as u8);
        octets.push(255);
    }
    let tex = Texture2D::from_rgba8(N_ZONAL as u16, 1, &octets);
    tex.set_filter(FilterMode::Linear);
    let paires = a.nb_bandes.round().clamp(1.0, 10.0);
    let pas = LAT_MAX / (paires + 1.0);
    let pole_lat = ((paires + 0.7) * pas).min(LAT_MAX * 0.95).sin();
    (tex, pole_lat)
}
