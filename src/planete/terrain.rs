//! Grille cube-sphere du terrain tellurique (conception : `conception_planete_v2.md` § 4).
//!
//! 6 faces de N×N texels projetées sur la sphère avec un warp équi-angulaire
//! (`tan`) : l'écart de surface entre texels tombe de ~5× à ~1,3×, pas de
//! singularité polaire. Le voisinage inter-faces se fait par RE-PROJECTION 3D
//! (on prolonge les coordonnées tangentes au-delà du bord puis on re-projette),
//! ce qui remplace la table d'arêtes envisagée : zéro cas particulier.
#![allow(dead_code)]

use super::apparence::Apparence;
use macroquad::prelude::*;
use std::f32::consts::FRAC_PI_4;

pub const NB_FACES: usize = 6;

/// Résolution d'une face (texels). 6×256² ≈ 400 k texels, atlas ~1,5 Mo.
pub const N_ATLAS: usize = 256;

/// Repère de chaque face : (normale, right, up).
/// L'ordre (+X, -X, +Y, -Y, +Z, -Z) est aussi celui de l'atlas (3 colonnes × 2 lignes).
const FACES: [(Vec3, Vec3, Vec3); NB_FACES] = [
    (Vec3::X, Vec3::NEG_Z, Vec3::Y),
    (Vec3::NEG_X, Vec3::Z, Vec3::Y),
    (Vec3::Y, Vec3::X, Vec3::NEG_Z),
    (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    (Vec3::Z, Vec3::X, Vec3::Y),
    (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
];

/// Couches de données du terrain (index = face·n² + y·n + x).
pub struct Terrain {
    pub n: usize,
    pub h: Vec<f32>,       // altitude
    pub flux: Vec<f32>,    // accumulation d'écoulement
    pub hum: Vec<f32>,     // humidité
    pub chaleur: Vec<f32>, // chaleur volcanique (fonte locale, § 11 bis)
}

impl Terrain {
    pub fn new(n: usize) -> Self {
        let total = NB_FACES * n * n;
        Self {
            n,
            h: vec![0.0; total],
            flux: vec![0.0; total],
            hum: vec![0.0; total],
            chaleur: vec![0.0; total],
        }
    }

    #[inline]
    pub fn idx(&self, face: usize, x: usize, y: usize) -> usize {
        face * self.n * self.n + y * self.n + x
    }

    pub fn nb_texels(&self) -> usize {
        NB_FACES * self.n * self.n
    }
}

/// Angle équi-angulaire du texel d'indice `i` (centre) : dans [-π/4, π/4].
#[inline]
fn angle_texel(i: f32, n: usize) -> f32 {
    (2.0 * (i + 0.5) / n as f32 - 1.0) * FRAC_PI_4
}

/// Centre du texel (face, x, y) -> point sur la sphère unité.
pub fn texel_vers_sphere(face: usize, x: usize, y: usize, n: usize) -> Vec3 {
    texel_vers_sphere_f(face, x as f32, y as f32, n)
}

/// Variante continue (coordonnées texel fractionnaires ; x = 0.0 vise le
/// centre du premier texel). Utilisée par le voisinage et, plus tard, le bake
/// de la gouttière.
pub fn texel_vers_sphere_f(face: usize, x: f32, y: f32, n: usize) -> Vec3 {
    let a = angle_texel(x, n);
    let b = angle_texel(y, n);
    let (nm, r, u) = FACES[face];
    (nm + r * a.tan() + u * b.tan()).normalize()
}

/// Face dominante d'une direction (axe de plus grande composante).
#[inline]
pub fn face_de(d: Vec3) -> usize {
    let a = d.abs();
    if a.x >= a.y && a.x >= a.z {
        if d.x >= 0.0 { 0 } else { 1 }
    } else if a.y >= a.z {
        if d.y >= 0.0 { 2 } else { 3 }
    } else if d.z >= 0.0 { 4 } else { 5 }
}

/// Direction sphère -> (face, x, y) en coordonnées texel CONTINUES
/// (0.0 = centre du premier texel ; utile pour l'interpolation bilinéaire).
pub fn sphere_vers_texel_f(d: Vec3, n: usize) -> (usize, f32, f32) {
    let face = face_de(d);
    let (nm, r, u) = FACES[face];
    let inv = 1.0 / d.dot(nm);
    let a = (d.dot(r) * inv).atan() / FRAC_PI_4; // [-1, 1]
    let b = (d.dot(u) * inv).atan() / FRAC_PI_4;
    let fx = (a + 1.0) * 0.5 * n as f32 - 0.5;
    let fy = (b + 1.0) * 0.5 * n as f32 - 0.5;
    (face, fx, fy)
}

/// Direction sphère -> texel entier le plus proche.
pub fn sphere_vers_texel(d: Vec3, n: usize) -> (usize, usize, usize) {
    let (face, fx, fy) = sphere_vers_texel_f(d, n);
    let cl = |v: f32| (v.round().max(0.0) as usize).min(n - 1);
    (face, cl(fx), cl(fy))
}

/// Voisin du texel (face, x, y) dans la direction (dx, dy) ∈ [-1, 1]².
/// Intra-face : arithmétique simple. Au bord : on prolonge les coordonnées
/// tangentes AU-DELÀ de ±45° puis on re-projette -> le texel adjacent de la
/// face voisine, sans table d'arêtes. Aux 8 coins du cube, le voisin diagonal
/// n'existe pas : la re-projection renvoie alors un des 7 voisins réels
/// (doublon toléré par les algos, cf. conception § 4.3).
pub fn voisin(face: usize, x: usize, y: usize, dx: i32, dy: i32, n: usize) -> (usize, usize, usize) {
    let xi = x as i32 + dx;
    let yi = y as i32 + dy;
    let ni = n as i32;
    if xi >= 0 && xi < ni && yi >= 0 && yi < ni {
        return (face, xi as usize, yi as usize);
    }
    let d = texel_vers_sphere_f(face, xi as f32, yi as f32, n);
    sphere_vers_texel(d, n)
}

/// Les 8 voisins d'un texel (avec doublon possible aux coins du cube).
pub fn voisins8(face: usize, x: usize, y: usize, n: usize) -> [(usize, usize, usize); 8] {
    const DIRS: [(i32, i32); 8] = [
        (-1, -1), (0, -1), (1, -1),
        (-1, 0), (1, 0),
        (-1, 1), (0, 1), (1, 1),
    ];
    let mut out = [(0usize, 0usize, 0usize); 8];
    for (k, (dx, dy)) in DIRS.iter().enumerate() {
        out[k] = voisin(face, x, y, *dx, *dy, n);
    }
    out
}

// ---------------------------------------------------------------------------
// Bruit CPU — port exact des fonctions du shader (`planete.frag.glsl`) pour
// que la géographie précalculée garde le caractère visuel actuel.
// ---------------------------------------------------------------------------

fn fract3(p: Vec3) -> Vec3 {
    p - p.floor()
}

fn hash(p: Vec3) -> f32 {
    let mut p = fract3(p * 0.318_309_9 + 0.1);
    p *= 17.0;
    let v = p.x * p.y * p.z * (p.x + p.y + p.z);
    v - v.floor()
}

fn vnoise(x: Vec3) -> f32 {
    let i = x.floor();
    let f = fract3(x);
    let f = f * f * (Vec3::splat(3.0) - 2.0 * f);
    let n = |dx: f32, dy: f32, dz: f32| hash(i + vec3(dx, dy, dz));
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    l(
        l(l(n(0.0, 0.0, 0.0), n(1.0, 0.0, 0.0), f.x), l(n(0.0, 1.0, 0.0), n(1.0, 1.0, 0.0), f.x), f.y),
        l(l(n(0.0, 0.0, 1.0), n(1.0, 0.0, 1.0), f.x), l(n(0.0, 1.0, 1.0), n(1.0, 1.0, 1.0), f.x), f.y),
        f.z,
    )
}

fn fbm_n(mut p: Vec3, octaves: u32) -> f32 {
    let mut v = 0.0;
    let mut a = 0.5;
    for _ in 0..octaves {
        v += a * vnoise(p);
        p *= 2.0;
        a *= 0.5;
    }
    v
}

fn fbm(p: Vec3) -> f32 {
    fbm_n(p, 5)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------
// Génération (conception § 5) — étape 2 : altitude de base + humidité.
// L'érosion (§ 5.2/5.3) et l'hydrologie (§ 5.4) viendront s'insérer ici.
// ---------------------------------------------------------------------------

/// Sorties prêtes pour le GPU : atlas RGBA8 (R+G = altitude 16 bits,
/// B = flux, A = humidité) + niveau de la mer par quantile (§ 9.1).
pub struct DonneesTerrain {
    pub atlas: Vec<u8>,
    pub largeur: u16,
    pub hauteur: u16,
    pub niveau_mer: f32,
}

/// Altitude de base : même recette que le shader (fbm + domain warping),
/// mais les crêtes ridged sont intégrées DANS h -> l'eau et l'érosion les voient.
fn altitude_base(d: Vec3, freq: f32, relief: f32, sd: Vec3) -> f32 {
    let p = d * freq + sd;
    // Le champ de warp ne sert qu'à déformer : 3 octaves suffisent (le détail
    // fin du warp est invisible) -> ~30 % de bruit en moins.
    let q = vec3(fbm_n(p + 1.3, 3), fbm_n(p + 7.2, 3), fbm_n(p + 3.4, 3));
    let mut h = fbm(p + 1.9 * q);
    h = h + (fbm(p * 3.0 + 17.0) - h) * 0.18;
    let rg = 1.0 - (2.0 * fbm(p * 2.2 + 9.0) - 1.0).abs();
    h + relief * 0.35 * rg * smoothstep(0.45, 0.75, h)
}

/// Fréquence de base selon le motif d'eau (identique au shader).
fn frequence_motif(eau_motif: f32) -> f32 {
    if eau_motif < 0.5 {
        1.6
    } else if eau_motif < 1.5 {
        2.4
    } else if eau_motif < 2.5 {
        1.5
    } else {
        4.5
    }
}

// ---------------------------------------------------------------------------
// Érosion (conception § 5.2, § 5.3, § 9) — thermique (éboulis, universelle)
// puis hydraulique (gouttes, intensité pilotée par le climat).
// ---------------------------------------------------------------------------

/// RNG déterministe (SplitMix64) : la géographie ne dépend QUE de la graine
/// (pas du RNG global macroquad, cf. conception § 7 « déterminisme »).
pub struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(2) | 1)
    }
    fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn f32(&mut self) -> f32 {
        (self.suivant() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Paramètres d'érosion, dérivés du climat (conception § 9.3).
pub struct ParamsErosion {
    pub nb_gouttes: u32,   // ∝ intensité hydraulique du climat
    pub inertie: f32,      // 0.05 vallées en V ; ~0.35 vallées en U (glaciaire)
    pub capacite: f32,     // sédiment transportable ∝ pente × eau
    pub erosion: f32,      // fraction arrachée par pas
    pub depot: f32,        // fraction déposée par pas
    pub evaporation: f32,  // haut = rivières courtes (désert)
    pub pas_max: u32,
    pub talus: f32,        // pente critique de l'éboulis (en Δh par texel)
}

/// Les deux « boutons » climatiques (§ 9.3) dérivés de l'`Apparence` existante :
/// pas de nouveau champ tant que la logique de `apparence_tellurique` suffit.
fn params_depuis_apparence(app: &Apparence, n: usize) -> ParamsErosion {
    let intensite = if app.lave > 0.5 {
        0.0 // monde de lave : pas d'eau qui sculpte
    } else {
        (0.15 + app.eau * 0.9).min(1.0) * (1.0 - 0.5 * app.voile)
    };
    let glaciaire = app.calotte < 0.35;
    // Curseur perf/beauté n° 1. 0.15 goutte/texel avec un taux d'érosion
    // relevé (0.38) sculpte quasiment comme 0.25/0.30 pour ~40 % du coût en
    // moins (calibré au bench du 2026-07-02 : l'érosion dominait à ~62 %).
    const QUALITE: f32 = 0.15;
    ParamsErosion {
        nb_gouttes: (QUALITE * intensite * (NB_FACES * n * n) as f32) as u32,
        inertie: if glaciaire { 0.35 } else { 0.05 },
        capacite: 6.0,
        erosion: 0.38,
        depot: 0.3,
        evaporation: if app.eau < 0.15 { 0.06 } else { 0.02 },
        pas_max: 48,
        talus: (4.0 / n as f32) * if app.dunes > 0.0 { 0.6 } else { 1.0 },
    }
}

/// Lit `h` au texel (xi, yi) de `face`, débordement re-projeté sur la face voisine.
#[inline]
fn lire_h(t: &Terrain, face: usize, xi: i32, yi: i32) -> f32 {
    let n = t.n as i32;
    let (f, x, y) = if xi >= 0 && xi < n && yi >= 0 && yi < n {
        (face, xi as usize, yi as usize)
    } else {
        let d = texel_vers_sphere_f(face, xi as f32, yi as f32, t.n);
        sphere_vers_texel(d, t.n)
    };
    t.h[t.idx(f, x, y)]
}

/// Hauteur seule par interpolation bilinéaire (2× moins de lectures que
/// `hauteur_gradient` : pour le point d'ARRIVÉE d'un pas de goutte).
fn hauteur_bi(t: &Terrain, face: usize, fx: f32, fy: f32) -> f32 {
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let u = fx - x0 as f32;
    let v = fy - y0 as f32;
    let h00 = lire_h(t, face, x0, y0);
    let h10 = lire_h(t, face, x0 + 1, y0);
    let h01 = lire_h(t, face, x0, y0 + 1);
    let h11 = lire_h(t, face, x0 + 1, y0 + 1);
    h00 * (1.0 - u) * (1.0 - v) + h10 * u * (1.0 - v) + h01 * (1.0 - u) * v + h11 * u * v
}

/// Hauteur + gradient (repère de la face) par interpolation bilinéaire.
fn hauteur_gradient(t: &Terrain, face: usize, fx: f32, fy: f32) -> (f32, f32, f32) {
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let u = fx - x0 as f32;
    let v = fy - y0 as f32;
    let h00 = lire_h(t, face, x0, y0);
    let h10 = lire_h(t, face, x0 + 1, y0);
    let h01 = lire_h(t, face, x0, y0 + 1);
    let h11 = lire_h(t, face, x0 + 1, y0 + 1);
    let gx = (h10 - h00) * (1.0 - v) + (h11 - h01) * v;
    let gy = (h01 - h00) * (1.0 - u) + (h11 - h10) * u;
    let h = h00 * (1.0 - u) * (1.0 - v) + h10 * u * (1.0 - v) + h01 * (1.0 - u) * v + h11 * u * v;
    (h, gx, gy)
}

/// Dépose (`montant` > 0) ou arrache (< 0) en répartissant sur les 4 texels
/// voisins (poids bilinéaires inverses) -> pas de trous en pointe.
fn deposer(t: &mut Terrain, face: usize, fx: f32, fy: f32, montant: f32) {
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let u = fx - x0 as f32;
    let v = fy - y0 as f32;
    let n = t.n as i32;
    let poids = [
        (0, 0, (1.0 - u) * (1.0 - v)),
        (1, 0, u * (1.0 - v)),
        (0, 1, (1.0 - u) * v),
        (1, 1, u * v),
    ];
    for (dx, dy, w) in poids {
        let (xi, yi) = (x0 + dx, y0 + dy);
        let (f, x, y) = if xi >= 0 && xi < n && yi >= 0 && yi < n {
            (face, xi as usize, yi as usize)
        } else {
            let d = texel_vers_sphere_f(face, xi as f32, yi as f32, t.n);
            sphere_vers_texel(d, t.n)
        };
        let i = t.idx(f, x, y);
        t.h[i] += montant * w;
    }
}

/// Érosion THERMIQUE (éboulis) : si la pente vers le voisin le plus bas dépasse
/// l'angle de talus, la matière glisse. Universelle — c'est elle qui évite
/// l'aspect « bruit plastique » des mondes sans eau (conception § 5.3).
pub fn eroder_thermique(t: &mut Terrain, talus: f32, passes: u32) {
    for _ in 0..passes {
        for f in 0..NB_FACES {
            for y in 0..t.n {
                for x in 0..t.n {
                    let i = t.idx(f, x, y);
                    let h0 = t.h[i];
                    let mut cible = i;
                    let mut plus_grand = 0.0f32;
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let (vf, vx, vy) = voisin(f, x, y, dx, dy, t.n);
                        let j = t.idx(vf, vx, vy);
                        let dh = h0 - t.h[j];
                        if dh > plus_grand {
                            plus_grand = dh;
                            cible = j;
                        }
                    }
                    if plus_grand > talus {
                        let m = (plus_grand - talus) * 0.25;
                        t.h[i] -= m;
                        t.h[cible] += m;
                    }
                }
            }
        }
    }
}

/// Érosion HYDRAULIQUE par gouttes (méthode « droplet », conception § 5.2).
/// Chaque goutte se déplace EN 3D sur la sphère (aucune gestion de couture) :
/// elle descend le gradient, arrache du sédiment quand elle se charge, le
/// dépose quand elle sature ou remonte, s'évapore.
pub fn eroder_hydraulique(t: &mut Terrain, p: &ParamsErosion, rng: &mut Rng) {
    use std::f32::consts::TAU;
    if p.nb_gouttes == 0 {
        return;
    }
    let n = t.n;
    let pas = std::f32::consts::FRAC_PI_2 / n as f32; // ~1 texel par pas

    for _ in 0..p.nb_gouttes {
        // Naissance : point uniforme sur la sphère.
        let z = rng.f32() * 2.0 - 1.0;
        let ph = rng.f32() * TAU;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let mut pos = vec3(r * ph.cos(), z, r * ph.sin());
        let mut vel = Vec3::ZERO;
        let mut eau = 1.0f32;
        let mut sed = 0.0f32;
        let (mut face, mut fx, mut fy) = sphere_vers_texel_f(pos, n);

        for _ in 0..p.pas_max {
            let (h0, gx, gy) = hauteur_gradient(t, face, fx, fy);
            let (_, rr, uu) = FACES[face];
            let grad = rr * gx + uu * gy; // gradient tangent (approx. locale)
            vel = vel * p.inertie - grad * (1.0 - p.inertie);
            vel -= pos * vel.dot(pos); // reste tangent à la sphère
            if vel.length_squared() < 1.0e-12 {
                break; // plaine parfaitement plate : la goutte s'arrête
            }
            pos = (pos + vel.normalize() * pas).normalize();
            let (nf, nfx, nfy) = sphere_vers_texel_f(pos, n);
            let h1 = hauteur_bi(t, nf, nfx, nfy);
            let dh = h1 - h0;

            let cap = (-dh).max(0.01) * eau * p.capacite;
            if dh > 0.0 || sed > cap {
                // La goutte remonte ou sature -> elle dépose.
                let dep = if dh > 0.0 { sed.min(dh) } else { (sed - cap) * p.depot };
                sed -= dep;
                deposer(t, face, fx, fy, dep);
            } else {
                // Elle se charge -> elle creuse (jamais plus que la descente,
                // sinon on inverse la pente et on crée des trous).
                let ero = ((cap - sed) * p.erosion).min(-dh);
                sed += ero;
                deposer(t, face, fx, fy, -ero);
            }

            eau *= 1.0 - p.evaporation;
            if eau < 0.08 {
                break;
            }
            face = nf;
            fx = nfx;
            fy = nfy;
        }
        // Fin de vie : le sédiment restant se dépose sur place (deltas).
        deposer(t, face, fx, fy, sed);
    }
}

// ---------------------------------------------------------------------------
// Volcanisme (conception § 11 bis) : édifices estampés dans h AVANT l'érosion
// (ils ressortent patinés), champ de chaleur pour la fonte cryovolcanique, et
// sommets renvoyés comme SOURCES de flux (les coulées). La caldeira creuse un
// cratère que le priority-flood remplira (lac de cratère) et fera déborder par
// le point bas du rebord -> la coulée suit la vraie vallée, gratuitement.
// ---------------------------------------------------------------------------

/// Sème `~2 + 10×intensite` volcans. Renvoie les texels des centres (sources).
pub fn volcans(t: &mut Terrain, intensite: f32, rng: &mut Rng) -> Vec<usize> {
    use std::f32::consts::TAU;
    if intensite <= 0.001 {
        return Vec::new();
    }
    let n = t.n;
    let nb = 2 + (intensite * 10.0) as usize;
    let mut sommets = Vec::with_capacity(nb);
    for _ in 0..nb {
        let z = rng.f32() * 2.0 - 1.0;
        let ph = rng.f32() * TAU;
        let r = (1.0 - z * z).max(0.0).sqrt();
        let dir = vec3(r * ph.cos(), z, r * ph.sin());
        let ray = (6.0 + rng.f32() * 14.0) / n as f32 * std::f32::consts::FRAC_PI_2;
        let haut = 0.12 + rng.f32() * 0.18;
        let cos_min = (ray * 3.2).cos(); // influence (chaleur) jusqu'à ~3 rayons

        for f in 0..NB_FACES {
            for y in 0..n {
                for x in 0..n {
                    let d = texel_vers_sphere(f, x, y, n);
                    let c = d.dot(dir);
                    if c < cos_min {
                        continue;
                    }
                    let ang = c.clamp(-1.0, 1.0).acos();
                    let i = t.idx(f, x, y);
                    let xr = ang / ray;
                    if xr < 1.0 {
                        // Cône patiné + caldeira : le centre est SOUS le rebord
                        // -> cratère, futur lac (de lave) avec déversoir.
                        let cone = (1.0 - xr).powf(1.6);
                        let cratere = 0.45 * smoothstep(0.18, 0.0, xr);
                        t.h[i] += haut * (cone - cratere);
                    }
                    // Chaleur en cloche, plus large que l'édifice (fonte).
                    let ch = (-(ang / (ray * 1.7)) * (ang / (ray * 1.7))).exp();
                    if ch > t.chaleur[i] {
                        t.chaleur[i] = ch;
                    }
                }
            }
        }
        let (sf, sx, sy) = sphere_vers_texel(dir, n);
        sommets.push(t.idx(sf, sx, sy));
    }
    sommets
}

// ---------------------------------------------------------------------------
// Hydrologie (conception § 5.4, § 9.2, § 10.1) : priority-flood -> lacs au
// niveau de déversement ; drainage -> flux D8 accumulé ; humidité finale par
// distance à l'eau, normalisée par rang (§ 11.2 bis).
// ---------------------------------------------------------------------------

/// (face, x, y) depuis l'index plat.
#[inline]
fn inv_idx(i: usize, n: usize) -> (usize, usize, usize) {
    (i / (n * n), i % n, (i / n) % n)
}

/// Clé de la file de priorité (hauteur de remplissage, index) — total_cmp.
#[derive(PartialEq)]
struct Cle(f32, u32);
impl Eq for Cle {}
impl PartialOrd for Cle {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cle {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0).then(self.1.cmp(&o.1))
    }
}

/// Hydrologie complète. Écrit `flux` (encodé log, lacs = 1.0), remonte `h` au
/// niveau des lacs, réécrit `hum` (proximité de l'eau + bruit, rang 0..1).
/// `sources`/`debit` : injections de flux aux sommets volcaniques (coulées).
/// `fonte` : la chaleur volcanique compte comme humidité (cryovolcanisme).
/// Renvoie le niveau de la mer (quantile sur le h AVANT remontée des lacs).
pub fn hydrologie(t: &mut Terrain, eau: f32, sources: &[usize], debit: f32, fonte: bool) -> f32 {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let n = t.n;
    let total = t.h.len();

    // 1) PRIORITY-FLOOD depuis le minimum global (sphère fermée : tout draine
    //    vers lui). `w` = surface remplie ; `aval[j]` = texel qui a inondé j,
    //    c'est-à-dire la direction d'écoulement (gratuite, pas de D8 séparé).
    let graine = (0..total).min_by(|a, b| t.h[*a].total_cmp(&t.h[*b])).unwrap();
    let mut w = t.h.clone();
    let mut aval = vec![u32::MAX; total];
    let mut ordre: Vec<u32> = Vec::with_capacity(total);
    let mut vu = vec![false; total];
    let mut tas: BinaryHeap<Reverse<Cle>> = BinaryHeap::new();
    vu[graine] = true;
    tas.push(Reverse(Cle(t.h[graine], graine as u32)));
    while let Some(Reverse(Cle(wc, ic))) = tas.pop() {
        ordre.push(ic);
        let (f, x, y) = inv_idx(ic as usize, n);
        for (vf, vx, vy) in voisins8(f, x, y, n) {
            let j = vf * n * n + vy * n + vx;
            if vu[j] {
                continue;
            }
            vu[j] = true;
            w[j] = t.h[j].max(wc); // une dépression se remplit au déversoir
            aval[j] = ic;
            tas.push(Reverse(Cle(w[j], j as u32)));
        }
    }

    // 2) FLUX : 1 de pluie par texel + injections volcaniques (coulées § 11
    //    bis), déversé vers l'aval. Parcours en ordre inverse d'inondation ->
    //    l'amont d'un texel est toujours déjà sommé.
    for &s in sources {
        t.flux[s] += debit;
    }
    for &ic in ordre.iter().rev() {
        let i = ic as usize;
        t.flux[i] += 1.0;
        if aval[i] != u32::MAX {
            t.flux[aval[i] as usize] += t.flux[i];
        }
    }

    // 3) Niveau de mer : quantile AVANT la remontée des lacs.
    let sea = niveau_mer(t, eau);

    // 4) LACS (§ 9.2) : dépression au-dessus de la mer -> plan d'eau plat au
    //    niveau de déversement + flux saturé (marqueur d'eau stagnante).
    let mut lac = vec![false; total];
    for i in 0..total {
        if w[i] > t.h[i] + 1.0e-4 && w[i] > sea {
            t.h[i] = w[i];
            lac[i] = true;
        }
    }

    // 5) Encodage log du flux (§ 10.1) : cours d'eau dans [0, 0.9], lacs = 1.
    let fmax = t.flux.iter().cloned().fold(1.0f32, f32::max);
    let ln_max = (1.0 + fmax).ln();
    for i in 0..total {
        t.flux[i] = if lac[i] {
            1.0
        } else {
            ((1.0 + t.flux[i]).ln() / ln_max * 0.9).min(0.9)
        };
    }

    // 6) HUMIDITÉ finale : proximité de l'eau (mer/lac/rivière) mêlée au bruit
    //    grande échelle, puis rang 0..1 (§ 11.2 bis : couverture garantie).
    //    Fonte cryovolcanique : la chaleur EST de l'humidité (glace -> eau),
    //    appliquée avant le rang -> anneau de forêt autour des volcans.
    if fonte {
        for i in 0..total {
            t.hum[i] = t.hum[i].max(t.chaleur[i] * 0.95);
        }
    }
    humidite_finale(t, sea, &lac);
    sea
}

/// Réécrit `hum` : BFS multi-source depuis l'eau -> proximité, mélange avec le
/// bruit existant, puis normalisation par RANG (distribution uniforme 0..1).
fn humidite_finale(t: &mut Terrain, sea: f32, lac: &[bool]) {
    use std::collections::VecDeque;
    let n = t.n;
    let total = t.h.len();

    let mut dist = vec![f32::MAX; total];
    let mut file: VecDeque<u32> = VecDeque::new();
    for i in 0..total {
        if t.h[i] < sea || lac[i] || t.flux[i] > 0.55 {
            dist[i] = 0.0;
            file.push_back(i as u32);
        }
    }
    if !file.is_empty() {
        while let Some(ic) = file.pop_front() {
            let i = ic as usize;
            let (f, x, y) = inv_idx(i, n);
            for (vf, vx, vy) in voisins8(f, x, y, n) {
                let j = vf * n * n + vy * n + vx;
                if dist[j] > dist[i] + 1.0 {
                    dist[j] = dist[i] + 1.0;
                    file.push_back(j as u32);
                }
            }
        }
        // Décroissance ~40 texels : l'intérieur des grands continents est sec
        // (déserts continentaux émergents, conception § 12).
        for i in 0..total {
            let prox = (-dist[i] / 40.0).exp();
            t.hum[i] = 0.55 * prox + 0.45 * t.hum[i];
        }
    }
    // Rang 0..1 : hum = « plus humide que X % de la planète ».
    let mut idxs: Vec<u32> = (0..total as u32).collect();
    idxs.sort_unstable_by(|a, b| t.hum[*a as usize].total_cmp(&t.hum[*b as usize]));
    let inv = 1.0 / (total - 1).max(1) as f32;
    for (rang, &ic) in idxs.iter().enumerate() {
        t.hum[ic as usize] = rang as f32 * inv;
    }
}

// --- Budget global de générations concurrentes -------------------------------
// La galerie peut demander ~12 terrains d'un coup : on borne le nombre de jobs
// simultanés pour garder l'UI fluide (chaque job parallélise déjà sur 6 faces).

use std::sync::atomic::{AtomicUsize, Ordering};

static JOBS: AtomicUsize = AtomicUsize::new(0);
const MAX_JOBS: usize = 2;

/// Tente de réserver un slot de génération. `false` = réessayer plus tard.
pub fn reserver_job() -> bool {
    JOBS.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
        if v < MAX_JOBS { Some(v + 1) } else { None }
    })
    .is_ok()
}

/// Variante de `generer` pour un thread de fond : libère le slot même si la
/// génération panique (garde RAII), et alimente les statistiques temps réel.
pub fn generer_job(app: &Apparence) -> DonneesTerrain {
    struct Garde;
    impl Drop for Garde {
        fn drop(&mut self) {
            JOBS.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let _garde = Garde;
    let t0 = std::time::Instant::now();
    let d = generer(app);
    let ms = t0.elapsed().as_millis() as usize;
    STAT_NB.fetch_add(1, Ordering::Relaxed);
    STAT_DERNIER.store(ms, Ordering::Relaxed);
    STAT_TOTAL.fetch_add(ms, Ordering::Relaxed);
    d
}

/// Temps (ms) de chaque étape de génération — instrumentation bench.
#[derive(Clone, Copy, Default)]
pub struct EtapesMs {
    pub bruit: f32,
    pub volcans: f32,
    pub erosion: f32,
    pub hydro: f32,
    pub bake: f32,
}

fn chrono_ms(t: &mut std::time::Instant) -> f32 {
    let d = t.elapsed().as_secs_f32() * 1000.0;
    *t = std::time::Instant::now();
    d
}

/// Génère la géographie d'une planète tellurique et la bake en atlas.
pub fn generer(app: &Apparence) -> DonneesTerrain {
    generer_chrono(app, N_ATLAS).0
}

/// Variante instrumentée, résolution paramétrable (bench 256 vs 512).
pub fn generer_chrono(app: &Apparence, n: usize) -> (DonneesTerrain, EtapesMs) {
    let mut et = EtapesMs::default();
    let mut top = std::time::Instant::now();
    let mut t = Terrain::new(n);
    let sd = vec3(app.seed, app.seed * 1.7, app.seed * 0.3);
    let freq = frequence_motif(app.eau_motif);

    // Remplissage parallèle par BANDES DE LIGNES réparties sur tous les cœurs
    // disponibles (mieux équilibré que « une face par thread », et exploite
    // les machines à 8+ cœurs). Tranches disjointes -> zéro synchronisation.
    let relief = app.relief;
    let nb_th = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(4).clamp(2, 16);
    let lignes_tot = NB_FACES * n; // ligne globale l -> (face l/n, y l%n)
    let par_bande = (lignes_tot + nb_th - 1) / nb_th;
    std::thread::scope(|s| {
        let mut restes_h: &mut [f32] = &mut t.h;
        let mut restes_hum: &mut [f32] = &mut t.hum;
        let mut l0 = 0usize;
        while l0 < lignes_tot {
            let nb_l = par_bande.min(lignes_tot - l0);
            let (h_b, h_reste) = restes_h.split_at_mut(nb_l * n);
            let (hum_b, hum_reste) = restes_hum.split_at_mut(nb_l * n);
            restes_h = h_reste;
            restes_hum = hum_reste;
            s.spawn(move || {
                for li in 0..nb_l {
                    let (f, y) = ((l0 + li) / n, (l0 + li) % n);
                    for x in 0..n {
                        let d = texel_vers_sphere(f, x, y, n);
                        h_b[li * n + x] = altitude_base(d, freq, relief, sd);
                        hum_b[li * n + x] = fbm_n(d * freq * 0.8 + sd + 30.0, 4);
                    }
                }
            });
            l0 += nb_l;
        }
    });

    // Normalisation de h dans [0,1] (histogramme complet disponible sur CPU).
    let (mut hmin, mut hmax) = (f32::MAX, f32::MIN);
    for &v in &t.h {
        hmin = hmin.min(v);
        hmax = hmax.max(v);
    }
    let inv = 1.0 / (hmax - hmin).max(1e-6);
    for v in &mut t.h {
        *v = (*v - hmin) * inv;
    }
    et.bruit = chrono_ms(&mut top);

    let mut rng = Rng::new(app.seed.to_bits() as u64);

    // Volcanisme (§ 11 bis) : édifices semés AVANT l'érosion -> patinés.
    let intensite_volc = app
        .lave
        .max(app.cryo)
        .max(if app.riv_lave > 0.5 { 0.6 } else { 0.0 });
    let sommets = volcans(&mut t, intensite_volc, &mut rng);
    et.volcans = chrono_ms(&mut top);

    // Érosion (§ 5.2/5.3) : thermique d'abord (stabilise les pentes du bruit),
    // hydraulique ensuite (creuse les vallées), thermique légère pour finir
    // (gomme les pointes laissées par les gouttes).
    let params = params_depuis_apparence(app, n);
    eroder_thermique(&mut t, params.talus, 2);
    eroder_hydraulique(&mut t, &params, &mut rng);
    eroder_thermique(&mut t, params.talus, 1);
    et.erosion = chrono_ms(&mut top);

    // Hydrologie (§ 5.4) : lacs, drainage, flux, humidité finale. Les sommets
    // volcaniques injectent du flux -> coulées qui suivent les vraies vallées.
    let debit = (t.h.len() as f32) * 0.005;
    let niveau = hydrologie(&mut t, app.eau, &sommets, debit, app.cryo > 0.001);
    et.hydro = chrono_ms(&mut top);

    let donnees = bake_atlas(&t, niveau);
    et.bake = chrono_ms(&mut top);
    (donnees, et)
}

// --- Statistiques temps réel + bench --------------------------------------

static STAT_NB: AtomicUsize = AtomicUsize::new(0);
static STAT_DERNIER: AtomicUsize = AtomicUsize::new(0);
static STAT_TOTAL: AtomicUsize = AtomicUsize::new(0);

/// (nb terrains générés, dernier temps en ms, temps cumulé en ms).
pub fn stats() -> (usize, usize, usize) {
    (
        STAT_NB.load(Ordering::Relaxed),
        STAT_DERNIER.load(Ordering::Relaxed),
        STAT_TOTAL.load(Ordering::Relaxed),
    )
}

// État du bench, affiché par l'overlay de la galerie.
static BENCH_FAIT: AtomicUsize = AtomicUsize::new(0);
static BENCH_TOTAL: AtomicUsize = AtomicUsize::new(0);
static BENCH_MSG: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Texte d'état du bench pour l'UI : progression, puis chemin du rapport.
pub fn bench_etat() -> Option<String> {
    let total = BENCH_TOTAL.load(Ordering::Relaxed);
    if total == 0 {
        return None;
    }
    let fait = BENCH_FAIT.load(Ordering::Relaxed);
    if fait < total {
        Some(format!("bench en cours: {}/{}", fait, total))
    } else {
        BENCH_MSG.lock().ok().map(|m| m.clone())
    }
}

/// Bench complet en tâche de fond : tout le catalogue en 256², un échantillon
/// en 512². Rapport écrit dans `bench_terrain.txt` (chemin absolu affiché à
/// l'écran et en console). Le fichier est écrit dès la fin de la passe 256².
pub fn bench(presets: Vec<(String, Apparence)>) {
    // Un seul bench à la fois.
    let total = BENCH_TOTAL.load(Ordering::Relaxed);
    if total != 0 && BENCH_FAIT.load(Ordering::Relaxed) < total {
        return;
    }
    BENCH_TOTAL.store(presets.len().max(1), Ordering::Relaxed);
    BENCH_FAIT.store(0, Ordering::Relaxed);
    std::thread::spawn(move || {
        use std::fmt::Write as _;
        let chemin = std::env::current_dir()
            .map(|d| d.join("bench_terrain.txt"))
            .unwrap_or_else(|_| std::path::PathBuf::from("bench_terrain.txt"));
        let coeurs = std::thread::available_parallelism().map(|c| c.get()).unwrap_or(1);
        let profil = if cfg!(debug_assertions) { "debug" } else { "release" };
        let mut r = String::new();
        let _ = writeln!(r, "=== BENCH TERRAIN ===");
        let _ = writeln!(
            r,
            "coeurs logiques: {} | profil: {} | atlas: {}x{} ({} texels) | QUALITE erosion: 0.15",
            coeurs, profil, N_ATLAS, N_ATLAS, NB_FACES * N_ATLAS * N_ATLAS
        );
        let _ = writeln!(
            r,
            "{:<28} {:>8} {:>7} {:>7} {:>8} {:>7} {:>6}",
            "preset", "total", "bruit", "volcan", "erosion", "hydro", "bake"
        );
        let mut totaux: Vec<(String, f32)> = Vec::new();
        for (idx, (nom, app)) in presets.iter().enumerate() {
            let t0 = std::time::Instant::now();
            let (_d, e) = generer_chrono(app, N_ATLAS);
            let tot = t0.elapsed().as_secs_f32() * 1000.0;
            let _ = writeln!(
                r,
                "{:<28} {:>6.0}ms {:>7.0} {:>7.0} {:>8.0} {:>7.0} {:>6.0}",
                nom, tot, e.bruit, e.volcans, e.erosion, e.hydro, e.bake
            );
            totaux.push((nom.clone(), tot));
            BENCH_FAIT.store(idx + 1, Ordering::Relaxed);
            if idx % 10 == 9 {
                println!("bench: {}/{}", idx + 1, presets.len());
            }
        }
        let mut ts: Vec<f32> = totaux.iter().map(|x| x.1).collect();
        ts.sort_by(f32::total_cmp);
        let somme: f32 = ts.iter().sum();
        if !ts.is_empty() {
            let _ = writeln!(
                r,
                "--- stats {}p : n={} | min {:.0} ms | mediane {:.0} ms | moyenne {:.0} ms | max {:.0} ms | cumul {:.1} s",
                N_ATLAS, ts.len(), ts[0], ts[ts.len() / 2], somme / ts.len() as f32,
                ts[ts.len() - 1], somme / 1000.0
            );
        }
        totaux.sort_by(|a, b| b.1.total_cmp(&a.1));
        let _ = writeln!(r, "--- top 5 les plus lents :");
        for (nom, tms) in totaux.iter().take(5) {
            let _ = writeln!(r, "    {:<28} {:>6.0} ms", nom, tms);
        }
        // Écriture INTERMÉDIAIRE : le rapport 256² existe même si on quitte
        // pendant l'échantillon 512².
        let _ = std::fs::write(&chemin, &r);
        let _ = writeln!(r, "--- echantillon 512x512 (x4 texels, x4 memoire GPU) :");
        for (nom, app) in presets.iter().take(4) {
            let t0 = std::time::Instant::now();
            let _ = generer_chrono(app, 512);
            let _ = writeln!(r, "    {:<28} {:>6.0} ms", nom, t0.elapsed().as_secs_f32() * 1000.0);
        }
        let _ = std::fs::write(&chemin, &r);
        if let Ok(mut m) = BENCH_MSG.lock() {
            *m = format!("bench termine -> {}", chemin.display());
        }
        println!("{r}\nbench termine -> {}", chemin.display());
    });
}

/// Niveau de la mer par QUANTILE (§ 9.1) : `eau = 0.7` -> exactement 70 % de
/// la surface sous l'eau. Renvoie -1 s'il n'y a pas d'océan.
pub fn niveau_mer(t: &Terrain, eau: f32) -> f32 {
    if eau <= 0.001 {
        return -1.0;
    }
    let mut tri = t.h.clone();
    tri.sort_unstable_by(f32::total_cmp);
    tri[((tri.len() - 1) as f32 * eau.clamp(0.0, 0.98)) as usize]
}

/// Packe le terrain en atlas 3×2 avec GOUTTIÈRE de 1 texel (§ 5.5) : autour de
/// chaque face, on recopie le texel réel de la face voisine pour que
/// l'interpolation bilinéaire du GPU soit correcte au passage des arêtes.
pub fn bake_atlas(t: &Terrain, niveau_mer: f32) -> DonneesTerrain {
    let n = t.n;
    let cote = n + 2;
    let (lw, lh) = (cote * 3, cote * 2);
    let mut atlas = vec![0u8; lw * lh * 4];

    for f in 0..NB_FACES {
        let (cx, cy) = (f % 3, f / 3);
        for py in 0..cote {
            for px in 0..cote {
                let xi = px as i32 - 1;
                let yi = py as i32 - 1;
                // Texel source : direct dans la face, sinon re-projection
                // (gouttière -> texel réel de la face voisine).
                let (sf, sx, sy) = if xi >= 0 && xi < n as i32 && yi >= 0 && yi < n as i32 {
                    (f, xi as usize, yi as usize)
                } else {
                    let d = texel_vers_sphere_f(f, xi as f32, yi as f32, n);
                    sphere_vers_texel(d, n)
                };
                let i = t.idx(sf, sx, sy);
                let h16 = (t.h[i].clamp(0.0, 1.0) * 65535.0) as u32;
                let o = ((cy * cote + py) * lw + cx * cote + px) * 4;
                atlas[o] = (h16 >> 8) as u8;
                atlas[o + 1] = (h16 & 0xFF) as u8;
                atlas[o + 2] = (t.flux[i].clamp(0.0, 1.0) * 255.0) as u8;
                atlas[o + 3] = (t.hum[i].clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
    }

    DonneesTerrain {
        atlas,
        largeur: lw as u16,
        hauteur: lh as u16,
        niveau_mer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pas angulaire typique entre deux centres de texels voisins.
    fn pas_angulaire(n: usize) -> f32 {
        std::f32::consts::FRAC_PI_2 / n as f32
    }

    fn dist_angulaire(a: Vec3, b: Vec3) -> f32 {
        a.dot(b).clamp(-1.0, 1.0).acos()
    }

    #[test]
    fn points_sur_la_sphere_unite() {
        let n = 16;
        for f in 0..NB_FACES {
            for y in 0..n {
                for x in 0..n {
                    let d = texel_vers_sphere(f, x, y, n);
                    assert!((d.length() - 1.0).abs() < 1e-5);
                }
            }
        }
    }

    #[test]
    fn aller_retour_tous_texels() {
        // texel -> sphère -> texel doit être l'identité (n petit : exhaustif).
        for n in [8usize, 32] {
            for f in 0..NB_FACES {
                for y in 0..n {
                    for x in 0..n {
                        let d = texel_vers_sphere(f, x, y, n);
                        assert_eq!(sphere_vers_texel(d, n), (f, x, y), "n={}", n);
                    }
                }
            }
        }
        // n = 256 : échantillonné.
        let n = 256;
        for f in 0..NB_FACES {
            for y in (0..n).step_by(7) {
                for x in (0..n).step_by(7) {
                    let d = texel_vers_sphere(f, x, y, n);
                    assert_eq!(sphere_vers_texel(d, n), (f, x, y));
                }
            }
        }
    }

    #[test]
    fn voisin_intra_face() {
        let n = 16;
        assert_eq!(voisin(0, 5, 5, 1, 0, n), (0, 6, 5));
        assert_eq!(voisin(3, 5, 5, -1, 1, n), (3, 4, 6));
    }

    #[test]
    fn voisin_bord_est_proche_et_valide() {
        // Tout voisin (y compris à travers une couture) est un texel valide,
        // différent, et angulairement proche (< 2,5 pas).
        let n = 32;
        let pas = pas_angulaire(n);
        for f in 0..NB_FACES {
            for y in 0..n {
                for x in 0..n {
                    if x != 0 && x != n - 1 && y != 0 && y != n - 1 {
                        continue; // seuls les bords nous intéressent
                    }
                    let ici = texel_vers_sphere(f, x, y, n);
                    for (vf, vx, vy) in voisins8(f, x, y, n) {
                        assert!(vf < NB_FACES && vx < n && vy < n);
                        assert!((vf, vx, vy) != (f, x, y), "voisin = soi-même en {:?}", (f, x, y));
                        let la = texel_vers_sphere(vf, vx, vy, n);
                        let dist = dist_angulaire(ici, la);
                        assert!(dist < 2.5 * pas, "voisin trop loin : {} pas en {:?}", dist / pas, (f, x, y));
                    }
                }
            }
        }
    }

    #[test]
    fn voisin_couture_reciproque() {
        // Pour un pas cardinal (non diagonal) à travers une couture, le voisin
        // du voisin doit contenir le texel d'origine.
        let n = 32;
        for f in 0..NB_FACES {
            for y in 0..n {
                for x in 0..n {
                    if x != 0 && x != n - 1 && y != 0 && y != n - 1 {
                        continue;
                    }
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let v = voisin(f, x, y, dx, dy, n);
                        if v.0 == f {
                            continue; // pas de couture traversée
                        }
                        let retours = voisins8(v.0, v.1, v.2, n);
                        assert!(
                            retours.contains(&(f, x, y)),
                            "réciprocité rompue : {:?} -> {:?}",
                            (f, x, y),
                            v
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn volcans_caldeira_et_coulees() {
        // Un volcan sur terrain plat : rebord plus haut que le centre
        // (caldeira), chaleur bornée, et la coulée injectée trace un flux
        // fort en aval du sommet après hydrologie.
        let n = 64;
        let mut t = Terrain::new(n);
        for v in t.h.iter_mut() {
            *v = 0.5;
        }
        // Exutoire global (océan) hors du volcan.
        let fond = t.idx(1, 2, 2);
        t.h[fond] = 0.05;
        let mut rng = Rng::new(7);
        let sommets = volcans(&mut t, 0.8, &mut rng);
        assert!(!sommets.is_empty());
        let hmax = t.h.iter().cloned().fold(0.0f32, f32::max);
        assert!(hmax > 0.55, "les volcans doivent élever le terrain");
        for &c in &t.chaleur {
            assert!((0.0..=1.0).contains(&c));
        }
        // Caldeira : le centre du premier volcan est sous le max local.
        let s0 = sommets[0];
        let (f, x, y) = inv_idx(s0, n);
        let mut rebord: f32 = 0.0;
        for (vf, vx, vy) in voisins8(f, x, y, n) {
            // rebord approximatif : 3 texels plus loin
            let (wf, wx, wy) = voisin(vf, vx, vy, (vx as i32 - x as i32).signum() * 2, (vy as i32 - y as i32).signum() * 2, n);
            rebord = rebord.max(t.h[t.idx(wf, wx, wy)]);
        }
        assert!(rebord > t.h[s0], "caldeira : rebord {} <= centre {}", rebord, t.h[s0]);
        // Coulées : flux injecté aux sommets -> fort flux au sommet même.
        let debit = (t.h.len() as f32) * 0.005;
        hydrologie(&mut t, 0.0, &sommets, debit, true);
        assert!(t.flux[s0] > 0.4, "flux au sommet trop faible : {}", t.flux[s0]);
    }

    #[test]
    fn hydrologie_lacs_et_flux() {
        // Terrain synthétique : une cuvette creusée dans un plateau doit
        // devenir un LAC (flux = 1, h remonté au déversoir), et le flux doit
        // croître vers l'aval (l'embouchure draine plus que la source).
        let n = 48;
        let mut t = Terrain::new(n);
        for f in 0..NB_FACES {
            for y in 0..n {
                for x in 0..n {
                    let i = t.idx(f, x, y);
                    // Pente douce globale sur chaque face + plateau.
                    t.h[i] = 0.4 + 0.4 * (x as f32 / n as f32) + 0.001 * (f as f32);
                }
            }
        }
        // « Océan » : le minimum global doit être HORS de la cuvette (sur une
        // sphère fermée, le bassin du minimum global est l'exutoire universel
        // et ne se remplit jamais — c'est l'océan).
        let fond = t.idx(0, 0, 0);
        t.h[fond] = 0.05;
        // Cuvette au centre de la face 2, fond sous son déversoir mais au-dessus
        // du niveau de la mer.
        let (cx, cy, ray) = (n / 2, n / 2, 5i32);
        for dy in -ray..=ray {
            for dx in -ray..=ray {
                if dx * dx + dy * dy <= ray * ray {
                    let i = t.idx(2, (cx as i32 + dx) as usize, (cy as i32 + dy) as usize);
                    t.h[i] = 0.45;
                }
            }
        }
        let sea = hydrologie(&mut t, 0.3, &[], 0.0, false);
        assert!(sea > 0.0 && sea < 1.0);
        let centre = t.idx(2, cx, cy);
        assert!(t.flux[centre] > 0.99, "le fond de cuvette doit être un lac");
        assert!(t.h[centre] > 0.5, "le lac doit remonter h au déversoir");
        for &v in t.flux.iter() {
            assert!(v.is_finite() && (0.0..=1.0).contains(&v));
        }
        // Humidité : rang uniforme -> moyenne ~0.5, bornée 0..1.
        let m: f32 = t.hum.iter().sum::<f32>() / t.hum.len() as f32;
        assert!((m - 0.5).abs() < 0.05, "hum moyenne {m}");
    }

    #[test]
    fn erosion_stable_et_bornee() {
        // L'érosion ne doit produire ni NaN ni valeurs délirantes, et doit
        // LISSER le terrain (l'écart-type des pentes diminue).
        let n = 32;
        let mut t = Terrain::new(n);
        for f in 0..NB_FACES {
            for y in 0..n {
                for x in 0..n {
                    let i = t.idx(f, x, y);
                    t.h[i] = ((x as f32 * 12.9898 + y as f32 * 78.233 + f as f32 * 37.719).sin() * 43758.547).fract().abs();
                }
            }
        }
        let pente_moy = |t: &Terrain| {
            let mut s = 0.0f64;
            for f in 0..NB_FACES {
                for y in 0..n {
                    for x in 0..n {
                        let (vf, vx, vy) = voisin(f, x, y, 1, 0, n);
                        s += (t.h[t.idx(f, x, y)] - t.h[t.idx(vf, vx, vy)]).abs() as f64;
                    }
                }
            }
            s / (NB_FACES * n * n) as f64
        };
        let avant = pente_moy(&t);
        let p = ParamsErosion {
            nb_gouttes: 4000,
            inertie: 0.05,
            capacite: 6.0,
            erosion: 0.3,
            depot: 0.3,
            evaporation: 0.02,
            pas_max: 48,
            talus: 4.0 / n as f32,
        };
        let mut rng = Rng::new(42);
        eroder_thermique(&mut t, p.talus, 2);
        eroder_hydraulique(&mut t, &p, &mut rng);
        eroder_thermique(&mut t, p.talus, 1);
        for &v in &t.h {
            assert!(v.is_finite() && v > -1.0 && v < 2.0, "h hors bornes : {v}");
        }
        let apres = pente_moy(&t);
        assert!(apres < avant, "l'érosion doit lisser : {avant} -> {apres}");
    }

    #[test]
    fn quantile_donne_la_couverture_demandee() {
        // Terrain synthétique : la fraction sous le niveau doit suivre `eau`.
        let n = 32;
        let mut t = Terrain::new(n);
        for (i, v) in t.h.iter_mut().enumerate() {
            *v = (i as f32 * 0.618_034).fract(); // pseudo-aléatoire déterministe
        }
        for eau in [0.2f32, 0.5, 0.85] {
            let nm = niveau_mer(&t, eau);
            let sous = t.h.iter().filter(|&&h| h < nm).count() as f32 / t.h.len() as f32;
            assert!((sous - eau).abs() < 0.02, "eau={eau} -> couverture {sous}");
        }
        assert_eq!(niveau_mer(&t, 0.0), -1.0);
    }

    #[test]
    fn atlas_dimensions_et_gouttiere() {
        let n = 16;
        let mut t = Terrain::new(n);
        for f in 0..NB_FACES {
            for y in 0..n {
                for x in 0..n {
                    let i = t.idx(f, x, y);
                    t.h[i] = f as f32 / 6.0; // valeur = index de face
                }
            }
        }
        let d = bake_atlas(&t, 0.5);
        let cote = n + 2;
        assert_eq!((d.largeur as usize, d.hauteur as usize), (cote * 3, cote * 2));
        assert_eq!(d.atlas.len(), cote * 3 * cote * 2 * 4);
        // Gouttière gauche de la face 0 (+X) : doit contenir la valeur d'une
        // AUTRE face (re-projection au-delà du bord), pas celle de la face 0.
        let lw = d.largeur as usize;
        let o = ((1 + 1) * lw) * 4; // px = 0 (gouttière), py = 2, face (0,0)
        let h16 = (d.atlas[o] as u32) << 8 | d.atlas[o + 1] as u32;
        let val = h16 as f32 / 65535.0;
        assert!((val - 0.0).abs() > 0.01, "la gouttière de +X ne doit pas venir de +X");
    }

    #[test]
    fn toute_direction_tombe_dans_la_grille() {
        // Balayage grossier de la sphère : jamais de panique, indices valides.
        let n = 64;
        let m = 200;
        for i in 0..m {
            for j in 0..m {
                let th = i as f32 / m as f32 * std::f32::consts::PI;
                let ph = j as f32 / m as f32 * std::f32::consts::TAU;
                let d = vec3(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin());
                let (f, x, y) = sphere_vers_texel(d, n);
                assert!(f < NB_FACES && x < n && y < n);
            }
        }
    }
}
// fin du module terrain
