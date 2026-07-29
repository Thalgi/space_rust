//! **Radiateurs thermiques** : les huit variantes d'échelle station, et la
//! grande voile en arête de poisson d'échelle vaisseau.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::PI;

use super::commun::*;

/// Variantes de radiateur thermique, d'après les technologies existantes (plus
/// une exotique). Toutes montées par un port `Surface`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VarianteRadiateur {
    /// Panneau plat rainuré (body-mounted classique).
    PanneauSimple,
    /// Bank de panneaux repliés type ISS ATCS.
    AccordeonATCS,
    /// Panneau sur joint rotatif visible (TRRJ).
    PivotantTRRJ,
    /// Caloducs apparents (loop heat pipe) courant sur le panneau.
    Caloducs,
    /// Déroulable (roll-out) : rouleau à la base, panneau étroit.
    Deroulable,
    /// Radiateur de coque, large et plat (body-mounted large).
    Corps,
    /// **Exotique** : radiateur à gouttelettes liquides (LDR) — rideau de
    /// gouttes entre deux booms.
    Gouttelettes,
    /// **Grande voile** radiante (échelle vaisseau, type ISV) : panneau plein à
    /// quille centrale et de bord, nervuré, teinte chaude (surface chauffée).
    Voile,
}

impl VarianteRadiateur {
    pub const TOUS: [VarianteRadiateur; 8] = [
        VarianteRadiateur::PanneauSimple,
        VarianteRadiateur::AccordeonATCS,
        VarianteRadiateur::PivotantTRRJ,
        VarianteRadiateur::Caloducs,
        VarianteRadiateur::Deroulable,
        VarianteRadiateur::Corps,
        VarianteRadiateur::Gouttelettes,
        VarianteRadiateur::Voile,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteRadiateur::PanneauSimple => "PANNEAU SIMPLE",
            VarianteRadiateur::AccordeonATCS => "ACCORDEON ATCS",
            VarianteRadiateur::PivotantTRRJ => "PIVOTANT TRRJ",
            VarianteRadiateur::Caloducs => "CALODUCS (LHP)",
            VarianteRadiateur::Deroulable => "DEROULABLE",
            VarianteRadiateur::Corps => "RADIATEUR DE COQUE",
            VarianteRadiateur::Gouttelettes => "GOUTTELETTES (LDR)",
            VarianteRadiateur::Voile => "GRANDE VOILE (VAISSEAU)",
        }
    }

    pub(super) fn cout(self) -> f32 {
        match self {
            VarianteRadiateur::Gouttelettes => 10.0,
            VarianteRadiateur::Voile => 14.0,
            VarianteRadiateur::AccordeonATCS => 7.0,
            _ => 5.0,
        }
    }

    /// Couleur dominante — bien contrastée d'une techno à l'autre.
    fn couleur(self) -> Color {
        match self {
            VarianteRadiateur::PanneauSimple => Color::new(0.88, 0.89, 0.92, 1.0), // blanc
            VarianteRadiateur::AccordeonATCS => Color::new(0.60, 0.72, 0.88, 1.0), // bleu-gris
            VarianteRadiateur::PivotantTRRJ => Color::new(0.90, 0.80, 0.58, 1.0),  // chaud
            VarianteRadiateur::Caloducs => Color::new(0.80, 0.82, 0.85, 1.0),      // clair (tubes cuivre)
            VarianteRadiateur::Deroulable => Color::new(0.80, 0.60, 0.18, 1.0),    // kapton doré
            VarianteRadiateur::Corps => Color::new(0.30, 0.34, 0.40, 1.0),         // sombre
            VarianteRadiateur::Gouttelettes => Color::new(0.55, 0.85, 1.0, 1.0),   // gouttes cyan
            VarianteRadiateur::Voile => Color::new(0.82, 0.52, 0.40, 1.0),         // surface chauffée
        }
    }

    /// Dessine le corps du radiateur depuis `pied`, déployé le long de +Z, large
    /// selon X. Chaque techno a sa couleur, ses proportions et sa silhouette.
    fn dessiner<P: Peintre>(self, p: &mut P, pied: Vec3, longueur: f32, largeur: f32) {
        let col = self.couleur();
        let sombre = Color::new(0.24, 0.26, 0.30, 1.0);
        let d = Vec3::Z;
        let w = Vec3::X;
        let lignes = (longueur / 0.4).max(3.0) as usize;
        match self {
            VarianteRadiateur::PanneauSimple => {
                crate::vaisseau::pieces::radiateur(p, pied, d, w, longueur, largeur, lignes, col, sombre);
            }
            VarianteRadiateur::Corps => {
                // Large et court (hugging), franchement plus sombre.
                crate::vaisseau::pieces::radiateur(p, pied, d, w, longueur * 0.5, largeur * 2.2, lignes, col, sombre);
            }
            VarianteRadiateur::Caloducs => {
                crate::vaisseau::pieces::radiateur(p, pied, d, w, longueur, largeur, lignes, col, sombre);
                let cuivre = Color::new(0.82, 0.45, 0.16, 1.0);
                let ntube = 6;
                for i in 0..ntube {
                    let x = (-0.5 + (i as f32 + 0.5) / ntube as f32) * largeur;
                    p.cylindre(pied + w * x - d * 0.1, pied + w * x + d * (longueur + 0.1), 0.05, cuivre);
                }
            }
            VarianteRadiateur::AccordeonATCS => {
                // Vraie corrugation : zigzag de plis en Y le long du déploiement.
                let n = 7;
                let dz = longueur / n as f32;
                let amp = largeur * 0.22;
                let mut prev = pied;
                for k in 0..n {
                    let y = if k % 2 == 0 { amp } else { -amp };
                    let next = pied + d * ((k + 1) as f32 * dz) + Vec3::Y * y;
                    p.panneau(prev - w * (largeur * 0.5), w * largeur, next - prev, col);
                    p.ligne(next - w * (largeur * 0.5), next + w * (largeur * 0.5), TRAIT_FIN, sombre);
                    prev = next;
                }
            }
            VarianteRadiateur::PivotantTRRJ => {
                // Gros joint rotatif visible (tambour) puis le panneau décalé.
                p.cylindre(pied - d * 0.15, pied + d * 0.4, largeur * 0.3, sombre);
                crate::vaisseau::pieces::radiateur(p, pied + d * 0.55, d, w, longueur, largeur, lignes, col, sombre);
            }
            VarianteRadiateur::Deroulable => {
                // Gros rouleau (tambour) à la base + longue bande étroite dorée.
                p.cylindre(pied - w * (largeur * 0.5), pied + w * (largeur * 0.5), 0.2, sombre);
                crate::vaisseau::pieces::radiateur(p, pied + d * 0.25, d, w, longueur * 1.5, largeur * 0.4, lignes, col, sombre);
            }
            VarianteRadiateur::Gouttelettes => {
                // LDR : deux booms + collecteurs + rideau de gouttelettes cyan.
                let g = largeur * 0.5;
                let a0 = pied - w * g;
                let a1 = pied + w * g;
                let boom = Color::new(0.5, 0.5, 0.55, 1.0);
                p.cylindre(a0, a0 + d * longueur, 0.06, boom);
                p.cylindre(a1, a1 + d * longueur, 0.06, boom);
                p.cylindre(a0, a1, 0.06, boom);
                p.cylindre(a0 + d * longueur, a1 + d * longueur, 0.06, boom);
                let (nx, nz) = (5, 12);
                for ix in 0..nx {
                    for iz in 0..nz {
                        let fx = (ix as f32 + 0.5) / nx as f32 - 0.5;
                        let fz = (iz as f32 + 0.5) / nz as f32;
                        p.sphere(pied + w * (fx * largeur) + d * (fz * longueur), 0.035, col);
                    }
                }
            }
            VarianteRadiateur::Voile => {
                // Grande voile radiante : quilles (centre + deux bords) qui
                // portent le panneau, panneau plein nervuré à teinte chaude.
                for s in [-1.0_f32, 0.0, 1.0] {
                    let e = w * (largeur * 0.5 * s);
                    p.cylindre(pied + e, pied + d * longueur + e, largeur * 0.028, sombre);
                }
                crate::vaisseau::pieces::radiateur(p, pied, d, w, longueur, largeur, lignes, col, sombre);
            }
        }
    }
}

// --- Radiateur de station --------------------------------------------------

/// Un unique port de montage `Surface` (avant −Z, vers l'hôte).
pub(super) fn ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
    )]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, variante: VarianteRadiateur, longueur: f32, largeur: f32) {
        // Jonction hôte (bras + socle) puis mât, comme le panneau.
        p.cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
        p.cube(Vec3::ZERO, Vec3::splat(0.2), COULEUR);
        p.cube_fil(Vec3::ZERO, Vec3::splat(0.2), SOMBRE);
        let pied = vec3(0.0, 0.0, MAST_PANNEAU);
        p.cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
        variante.dessiner(p, pied, longueur, largeur);
}

// --- Radiateur méga (échelle vaisseau) -------------------------------------

/// Un port `Surface`, comme les autres appendices : avant vers l'hôte (−Z),
/// l'aile se déploie de l'autre côté (+Z).
pub(super) fn mega_ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
    )]
}

pub(super) fn mega_dessiner<P: Peintre>(p: &mut P, longueur: f32, largeur: f32, ailettes: usize) {
        // Radiateur d'échelle vaisseau. Trois parties :
        //  1. un **gros parallélépipède fin** (le collecteur), avec au
        //     centre un **module de connexion** (stub où s'accroche le
        //     propulseur, côté −Z) ;
        //  2. un **trapèze franc très allongé** (bords **droits**) qui
        //     s'affine à peine — le petit côté vaut ~90 % du grand ;
        //  3. rempli de **tubes calorifiques** transverses (cylindres →
        //     volume, pas un panneau plat).
        let tige = Color::new(0.66, 0.67, 0.70, 1.0);
        let tube = Color::new(0.55, 0.57, 0.61, 1.0);
        let (d, w) = (Vec3::Z, Vec3::X);
        let demi0 = largeur * 0.5; // demi-largeur à la racine
        let pointe = demi0 * 0.9; // le petit côté ne perd que ~10 % (trapèze franc)

        // 1. Collecteur **retiré** : plus de parallélépipède à la base.
        //    `bd` reste la référence de profondeur d'où part le panneau.
        let bd = (largeur * 0.5).max(1.5);

        // 2. Panneau **solide** en trapèze franc (plan Y = 0).
        let z0 = bd * 0.5;
        let long = (longueur - z0).max(1.0);
        let panneau_col = Color::new(0.71, 0.71, 0.74, 1.0);
        let coins = [
            d * z0 - w * demi0,
            d * z0 + w * demi0,
            d * (z0 + long) + w * pointe,
            d * (z0 + long) - w * pointe,
        ];
        p.triangles(&coins, &[0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2], panneau_col);
        // Rails de bord (bords droits du trapèze).
        for cote in [-1.0_f32, 1.0] {
            p.cylindre(d * z0 + w * (cote * demi0), d * (z0 + long) + w * (cote * pointe), demi0 * 0.03, tige);
        }
        // 3. Boudins (tubes calorifiques) **transverses** (en travers de
        //    l'aile), **espacés** entre eux, sur **chaque face** du panneau.
        let m = ailettes.max(4);
        // Rayon = 30 % du pas → il reste des jours nets entre les boudins.
        let tuber = ((long / m as f32) * 0.30).min(demi0 * 0.06);
        for face in [-1.0_f32, 1.0] {
            let yo = Vec3::Y * (face * 0.10);
            for k in 0..m {
                let t = (k as f32 + 0.5) / m as f32;
                let z = z0 + long * t;
                let hwid = demi0 + (pointe - demi0) * t; // largeur locale du trapèze
                p.cylindre(d * z - w * hwid + yo, d * z + w * hwid + yo, tuber, tube);
            }
        }

        // Diamètres : gros cylindre noir (réservoir) réduit de 25 %, et
        // squelette cylindrique de la **moitié** de son diamètre.
        let rc = (demi0 * 0.4).max(0.9) * 0.75;

        // 4. **Colonne vertébrale** : un **cylindre** central (donc
        //    symétrique de chaque côté), de demi-diamètre du gros, de la
        //    pointe jusqu'à juste avant le module de connexion.
        p.cylindre(d * z0, d * (z0 + long), rc * 0.5, Color::new(0.10, 0.10, 0.12, 1.0));

        // 5. Gros **cylindre noir** central (réservoir/boom) : longueur =
        //    moitié de la hauteur du radiateur, raccourci pour laisser le
        //    connecteur dépasser dessous.
        let noir = Color::new(0.10, 0.10, 0.12, 1.0);
        let lc = long * 0.34;
        let cyl_haut = z0 + long * 0.30;
        let cyl_bas = cyl_haut - lc;
        p.cylindre(d * cyl_bas, d * cyl_haut, rc, noir);
}

pub(super) fn mega_cout(longueur: f32) -> f32 {
    16.0 + longueur
}

/// Déploiement (longueur) ou demi-envergure.
pub(super) fn mega_rayon_local(longueur: f32, largeur: f32) -> f32 {
    longueur.max(largeur)
}
