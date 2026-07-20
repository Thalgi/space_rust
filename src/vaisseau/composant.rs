//! Composant concret : **enum fermé** qui sait exposer ses ports et se dessiner
//! (voir `docs/stations_raccordement.md`, §3 et §4 — sous-étape 2a).
//!
//! Choix acté : dispatch par `match` sur un enum, **pas** de trait objet — KISS,
//! zéro allocation, monomorphisé, cohérent avec `TypeEngin`. Une seule fonction
//! par capacité (`ports`, `dessiner`, `cout`, `rayon_local`), qui `match` sur la
//! variante. Les styles/palettes viendront à l'Étape 5. Composants existants :
//! `ModuleAxial` (cylindre) et `Noeud` (hub sphérique 4 ou 6 sorties).

use super::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

// Palette provisoire (les styles arriveront à l'Étape 5).
const COULEUR: Color = Color { r: 0.85, g: 0.85, b: 0.88, a: 1.0 };
const SOMBRE: Color = Color { r: 0.25, g: 0.25, b: 0.28, a: 1.0 };

// Panneau solaire : mât entre le montage et le pied de la pale, et bras de base
// côté hôte (−Z) qui matérialise la jonction module ↔ panneau.
const MAST_PANNEAU: f32 = 0.4;
const BASE_ARM_PANNEAU: f32 = 0.3;

// Treillis : demi-section (fraction du rayon) et distance visée entre paires
// de montages d'ailes le long de la poutre.
const TREILLIS_SECTION: f32 = 0.5;
const TREILLIS_PAS_AILE: f32 = 2.25;

// Collerette de docking à chaque écoutille axiale : un col plus étroit qui
// dépasse du corps. Le port se pose à son **extrémité** → pincement net (offset
// visible) à chaque joint, et deux modules dockés forment un col reconnaissable
// au lieu d'un tube continu. Dimensions en fraction du rayon du module.
const COL_LONG: f32 = 0.25; // longueur du col
const COL_RAYON: f32 = 0.45; // rayon du col

// Embout coiffant chaque disque de bout : un petit cylindre qui **chevauche** le
// corps (aucune face coplanaire → pas de z-fighting, cause du halo bizarre) et
// déborde légèrement pour marquer l'arête. Fractions du rayon.
const EMBOUT_LONG: f32 = 0.08; // dépassement hors du corps
const EMBOUT_ENFONCE: f32 = 0.03; // chevauchement dans le corps
const EMBOUT_RAYON: f32 = 1.02; // léger débord radial

// Nœud : la sphère centrale fait 1.2× le rayon de profil (hub plus présent) ; les
// bras partent de sa surface.
const NOEUD_SPHERE: f32 = 1.2; // rayon de la sphère, en fraction du rayon de profil
const JONCTION_OFFSET: f32 = 0.2; // enfoncement de la base du bras dans la sphère

// Bras cylindrique d'une sortie de nœud : un vrai tronçon entre la sphère et la
// collerette de docking, pour que le hub ait de la présence. Fractions du rayon.
const BRAS_LONG: f32 = 0.45; // longueur du bras
const BRAS_RAYON: f32 = 0.6; // rayon du bras

/// Une brique concrète, dessinable et dotée de ports. Enum fermé : on ajoute une
/// variante ici et on complète les quatre `match`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Composant {
    /// Module pressurisé cylindrique, aligné sur Z, avec une **écoutille axiale
    /// à chaque bout** (avant sortant : +Z et −Z). `profil` fixe le rayon.
    ModuleAxial { profil: Profil, variante: VarianteModule, longueur: f32 },
    /// Nœud : hub sphérique multi-ports, toutes les écoutilles du même `profil`.
    /// `sorties` fixe le nombre/disposition (croix plane ou 3D).
    Noeud { profil: Profil, sorties: Sorties },
    /// Panneau solaire : mât + pale, monté par un unique port `Surface`.
    /// `variante` fixe le style (couleur, proportions, tuiles). Se déploie le long
    /// de +Z ; se pose en **paire miroir** via la symétrie.
    PanneauSolaire { profil: Profil, variante: VariantePanneau, longueur: f32, largeur: f32 },
    /// Treillis / poutre : ossature alignée sur Z. Bouts axiaux (`ModuleAxial`,
    /// chaînables avec modules/nœuds) + **ports hôtes `Surface`** par paires ±X
    /// répartis sur la longueur — accueillent n'importe quel appendice (panneau,
    /// radiateur, antenne). `style` = section ; `profil` fixe le gabarit.
    Treillis { profil: Profil, longueur: f32, style: StyleTreillis },
    /// Radiateur thermique : monté par un port `Surface`, déployé le long de +Z.
    /// `variante` fixe la technologie/allure.
    Radiateur { profil: Profil, variante: VarianteRadiateur, longueur: f32, largeur: f32 },
    /// Antenne / parabole : montée par un port `Surface`, pointe vers +Z.
    /// `variante` fixe le type (parabole, cornets, fouets, réseau, hélice).
    Antenne { profil: Profil, variante: VarianteAntenne, taille: f32 },
    /// Adaptateur tronconique : relie deux **profils** (ou sert de nez de docking
    /// PMA/IDA). Écoutilles axiales `grand` (−Z) et `petit` (+Z).
    Adaptateur { grand: Profil, petit: Profil, longueur: f32 },
}

/// Disposition des ports d'un [`Composant::Noeud`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sorties {
    /// 4 ports : 2 axiaux (±Z) + 2 radiaux (±X) — croix plane.
    Quatre,
    /// 6 ports : 2 axiaux (±Z) + 4 radiaux (±X, ±Y) — croix 3D.
    Six,
    /// 3 ports en T dans le plan XZ : barre ±X + tige −Z (lisible vu de dessous).
    T,
    /// 4 ports vers les sommets d'un tétraèdre régulier (répartition 3D isotrope).
    Tetra,
}

/// Faces d'un nœud : `(direction sortante, rotation du port, genre)`. La rotation
/// oriente l'**avant** du port (`rot*Z`) le long de la direction (sortant).
fn faces_noeud(sorties: Sorties) -> Vec<(Vec3, Quat, GenrePort)> {
    let z_plus = (Vec3::Z, Quat::IDENTITY, GenrePort::ModuleAxial);
    let z_moins = (Vec3::NEG_Z, Quat::from_rotation_y(PI), GenrePort::ModuleAxial);
    let x_plus = (Vec3::X, Quat::from_rotation_y(FRAC_PI_2), GenrePort::ModuleRadial);
    let x_moins = (Vec3::NEG_X, Quat::from_rotation_y(-FRAC_PI_2), GenrePort::ModuleRadial);
    let y_plus = (Vec3::Y, Quat::from_rotation_x(-FRAC_PI_2), GenrePort::ModuleRadial);
    let y_moins = (Vec3::NEG_Y, Quat::from_rotation_x(FRAC_PI_2), GenrePort::ModuleRadial);
    match sorties {
        Sorties::Quatre => vec![z_plus, z_moins, x_plus, x_moins],
        Sorties::Six => vec![z_plus, z_moins, x_plus, x_moins, y_plus, y_moins],
        // Barre ±X + tige −Z, tout dans le plan XZ (horizontal).
        Sorties::T => vec![x_plus, x_moins, z_moins],
        // Sommets d'un tétraèdre : rotation générique Z→direction via l'arc.
        Sorties::Tetra => [
            vec3(1.0, 1.0, 1.0),
            vec3(1.0, -1.0, -1.0),
            vec3(-1.0, 1.0, -1.0),
            vec3(-1.0, -1.0, 1.0),
        ]
        .into_iter()
        .map(|d| {
            let dir = d.normalize();
            (dir, Quat::from_rotation_arc(Vec3::Z, dir), GenrePort::ModuleRadial)
        })
        .collect(),
    }
}

/// Les 6 directions principales (±X, ±Y, ±Z) avec la rotation orientant l'avant
/// du port vers l'extérieur (`avant = rot*Z = dir`). Sert aux ports hôtes
/// `Surface` radiaux (les 4 premières = ±X, ±Y).
fn faces_principales() -> [(Vec3, Quat); 6] {
    [
        (Vec3::X, Quat::from_rotation_y(FRAC_PI_2)),
        (Vec3::NEG_X, Quat::from_rotation_y(-FRAC_PI_2)),
        (Vec3::Y, Quat::from_rotation_x(-FRAC_PI_2)),
        (Vec3::NEG_Y, Quat::from_rotation_x(FRAC_PI_2)),
        (Vec3::Z, Quat::IDENTITY),
        (Vec3::NEG_Z, Quat::from_rotation_y(PI)),
    ]
}

/// Variantes visuelles de [`Composant::PanneauSolaire`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VariantePanneau {
    /// Ambre rigide, deux lés (type ISS SAW).
    RigideUS,
    /// Bleu, plus court (segment russe).
    RusseBleu,
    /// iROSA : bande étroite et sombre (déroulable).
    RollOut,
    /// Cyan, futuriste.
    Futuriste,
    /// Tuiles hexagonales légèrement espacées.
    Hexagonal,
}

impl VariantePanneau {
    pub const TOUS: [VariantePanneau; 5] = [
        VariantePanneau::RigideUS,
        VariantePanneau::RusseBleu,
        VariantePanneau::RollOut,
        VariantePanneau::Futuriste,
        VariantePanneau::Hexagonal,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VariantePanneau::RigideUS => "RIGIDE US",
            VariantePanneau::RusseBleu => "RUSSE BLEU",
            VariantePanneau::RollOut => "ROLL-OUT (iROSA)",
            VariantePanneau::Futuriste => "FUTURISTE",
            VariantePanneau::Hexagonal => "HEXAGONAL",
        }
    }

    /// `(couleur, facteur longueur, facteur largeur)` — pour varier l'allure au-delà
    /// de la seule couleur.
    fn style(self) -> (Color, f32, f32) {
        match self {
            VariantePanneau::RigideUS => (Color::new(0.50, 0.38, 0.16, 1.0), 1.0, 1.0),
            VariantePanneau::RusseBleu => (Color::new(0.12, 0.20, 0.48, 1.0), 0.7, 1.0),
            VariantePanneau::RollOut => (Color::new(0.10, 0.12, 0.18, 1.0), 1.25, 0.5),
            VariantePanneau::Futuriste => (Color::new(0.10, 0.45, 0.50, 1.0), 1.0, 1.1),
            VariantePanneau::Hexagonal => (Color::new(0.22, 0.24, 0.44, 1.0), 1.0, 1.0),
        }
    }
}

/// Style structurel d'un [`Composant::Treillis`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StyleTreillis {
    /// Section carrée (4 longerons) — treillis ajouré classique.
    Carre,
    /// Section triangulaire (3 longerons) — plus léger.
    Triangulaire,
}

impl StyleTreillis {
    pub const TOUS: [StyleTreillis; 2] = [StyleTreillis::Carre, StyleTreillis::Triangulaire];
}

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
}

impl VarianteRadiateur {
    pub const TOUS: [VarianteRadiateur; 7] = [
        VarianteRadiateur::PanneauSimple,
        VarianteRadiateur::AccordeonATCS,
        VarianteRadiateur::PivotantTRRJ,
        VarianteRadiateur::Caloducs,
        VarianteRadiateur::Deroulable,
        VarianteRadiateur::Corps,
        VarianteRadiateur::Gouttelettes,
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
        }
    }

    fn cout(self) -> f32 {
        match self {
            VarianteRadiateur::Gouttelettes => 10.0,
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
        }
    }

    /// Dessine le corps du radiateur depuis `pied`, déployé le long de +Z, large
    /// selon X. Chaque techno a sa couleur, ses proportions et sa silhouette.
    fn dessiner(self, pied: Vec3, longueur: f32, largeur: f32) {
        let col = self.couleur();
        let sombre = Color::new(0.24, 0.26, 0.30, 1.0);
        let d = Vec3::Z;
        let w = Vec3::X;
        let lignes = (longueur / 0.4).max(3.0) as usize;
        match self {
            VarianteRadiateur::PanneauSimple => {
                super::pieces::radiateur(pied, d, w, longueur, largeur, lignes, col, sombre);
            }
            VarianteRadiateur::Corps => {
                // Large et court (hugging), franchement plus sombre.
                super::pieces::radiateur(pied, d, w, longueur * 0.5, largeur * 2.2, lignes, col, sombre);
            }
            VarianteRadiateur::Caloducs => {
                super::pieces::radiateur(pied, d, w, longueur, largeur, lignes, col, sombre);
                let cuivre = Color::new(0.82, 0.45, 0.16, 1.0);
                let ntube = 6;
                for i in 0..ntube {
                    let x = (-0.5 + (i as f32 + 0.5) / ntube as f32) * largeur;
                    super::cylindre(pied + w * x - d * 0.1, pied + w * x + d * (longueur + 0.1), 0.05, cuivre);
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
                    super::panneau(prev - w * (largeur * 0.5), w * largeur, next - prev, col);
                    draw_line_3d(next - w * (largeur * 0.5), next + w * (largeur * 0.5), sombre);
                    prev = next;
                }
            }
            VarianteRadiateur::PivotantTRRJ => {
                // Gros joint rotatif visible (tambour) puis le panneau décalé.
                super::cylindre(pied - d * 0.15, pied + d * 0.4, largeur * 0.3, sombre);
                super::pieces::radiateur(pied + d * 0.55, d, w, longueur, largeur, lignes, col, sombre);
            }
            VarianteRadiateur::Deroulable => {
                // Gros rouleau (tambour) à la base + longue bande étroite dorée.
                super::cylindre(pied - w * (largeur * 0.5), pied + w * (largeur * 0.5), 0.2, sombre);
                super::pieces::radiateur(pied + d * 0.25, d, w, longueur * 1.5, largeur * 0.4, lignes, col, sombre);
            }
            VarianteRadiateur::Gouttelettes => {
                // LDR : deux booms + collecteurs + rideau de gouttelettes cyan.
                let g = largeur * 0.5;
                let a0 = pied - w * g;
                let a1 = pied + w * g;
                let boom = Color::new(0.5, 0.5, 0.55, 1.0);
                super::cylindre(a0, a0 + d * longueur, 0.06, boom);
                super::cylindre(a1, a1 + d * longueur, 0.06, boom);
                super::cylindre(a0, a1, 0.06, boom);
                super::cylindre(a0 + d * longueur, a1 + d * longueur, 0.06, boom);
                let (nx, nz) = (5, 12);
                for ix in 0..nx {
                    for iz in 0..nz {
                        let fx = (ix as f32 + 0.5) / nx as f32 - 0.5;
                        let fz = (iz as f32 + 0.5) / nz as f32;
                        draw_sphere(pied + w * (fx * largeur) + d * (fz * longueur), 0.035, None, col);
                    }
                }
            }
        }
    }
}

/// Variantes d'antenne / parabole, montées par un port `Surface`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VarianteAntenne {
    /// Parabole grand gain, face vers +Z.
    ParaboleGG,
    /// Parabole à alimentation décalée (offset), inclinée.
    ParaboleOffset,
    /// Grappe de cornets (horns).
    Cornets,
    /// Fouets omnidirectionnels croisés.
    Fouet,
    /// Réseau phasé : plaque plate quadrillée.
    ReseauPhase,
    /// Antenne hélicoïdale.
    Helice,
}

impl VarianteAntenne {
    pub const TOUS: [VarianteAntenne; 6] = [
        VarianteAntenne::ParaboleGG,
        VarianteAntenne::ParaboleOffset,
        VarianteAntenne::Cornets,
        VarianteAntenne::Fouet,
        VarianteAntenne::ReseauPhase,
        VarianteAntenne::Helice,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteAntenne::ParaboleGG => "PARABOLE GRAND GAIN",
            VarianteAntenne::ParaboleOffset => "PARABOLE OFFSET",
            VarianteAntenne::Cornets => "CORNETS",
            VarianteAntenne::Fouet => "FOUETS",
            VarianteAntenne::ReseauPhase => "RESEAU PHASE",
            VarianteAntenne::Helice => "HELICE",
        }
    }

    fn cout(self) -> f32 {
        match self {
            VarianteAntenne::Cornets | VarianteAntenne::Helice => 4.0,
            _ => 3.0,
        }
    }

    /// Dessine l'antenne depuis `pied`, pointant vers +Z.
    fn dessiner(self, pied: Vec3, taille: f32) {
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        let sombre = Color::new(0.30, 0.32, 0.36, 1.0);
        let d = Vec3::Z;
        let w = Vec3::X;
        let up = Vec3::Y;
        match self {
            VarianteAntenne::ParaboleGG => super::parabole(pied, d, taille, clair),
            VarianteAntenne::ParaboleOffset => {
                let dir = (d + up * 0.45).normalize();
                super::parabole(pied, dir, taille * 0.9, clair);
            }
            VarianteAntenne::Cornets => {
                for (dx, dy) in [(-0.25_f32, 0.0_f32), (0.25, 0.0), (0.0, 0.28)] {
                    let base = pied + w * (dx * taille) + up * (dy * taille);
                    super::cone(base, d, taille * 0.06, taille * 0.28, taille * 0.7, clair);
                }
            }
            VarianteAntenne::Fouet => {
                let n = 4;
                for i in 0..n {
                    let a = TAU * i as f32 / n as f32;
                    let dir = (d + w * (0.4 * a.cos()) + up * (0.4 * a.sin())).normalize();
                    super::cylindre(pied, pied + dir * (taille * 1.5), 0.02, clair);
                    draw_sphere(pied + dir * (taille * 1.5), 0.04, None, clair);
                }
            }
            VarianteAntenne::ReseauPhase => {
                let s = taille * 0.9;
                let coin = pied - w * (s * 0.5) - up * (s * 0.5);
                super::panneau(coin, w * s, up * s, sombre);
                let n = 5;
                for i in 1..n {
                    let f = i as f32 / n as f32;
                    draw_line_3d(coin + w * (s * f), coin + w * (s * f) + up * s, clair);
                    draw_line_3d(coin + up * (s * f), coin + up * (s * f) + w * s, clair);
                }
            }
            VarianteAntenne::Helice => {
                let tours = 4.0;
                let n = 40;
                let ray = taille * 0.22;
                let haut = taille * 1.3;
                let point = |t: f32| {
                    let a = TAU * tours * t;
                    pied + w * (ray * a.cos()) + up * (ray * a.sin()) + d * (haut * t)
                };
                for i in 0..n {
                    super::cylindre(point(i as f32 / n as f32), point((i + 1) as f32 / n as f32), 0.025, clair);
                }
                super::cylindre(pied - d * 0.02, pied + d * 0.02, ray * 1.4, sombre); // réflecteur
            }
        }
    }
}

/// Variantes d'habitat (module pressurisé) — change couleur et détails de surface.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VarianteModule {
    /// Blanc simple.
    Standard,
    /// Teinte or (segment russe).
    Dore,
    /// Rangée de hublots + mains courantes EVA.
    Hublots,
    /// Grande fenêtre + rack externe (type Destiny).
    Labo,
    /// Profil bombé (type BEAM gonflable).
    Gonflable,
    /// Coupole vitrée à un bout (type Cupola).
    Coupole,
    /// Sas (type Quest) : écoutille EVA saillante + main courante.
    Sas,
}

impl VarianteModule {
    pub const TOUS: [VarianteModule; 7] = [
        VarianteModule::Standard,
        VarianteModule::Dore,
        VarianteModule::Hublots,
        VarianteModule::Labo,
        VarianteModule::Gonflable,
        VarianteModule::Coupole,
        VarianteModule::Sas,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteModule::Standard => "STANDARD",
            VarianteModule::Dore => "DORE (RUSSE)",
            VarianteModule::Hublots => "HUBLOTS",
            VarianteModule::Labo => "LABO",
            VarianteModule::Gonflable => "GONFLABLE (BEAM)",
            VarianteModule::Coupole => "COUPOLE",
            VarianteModule::Sas => "SAS",
        }
    }

    fn couleur(self) -> Color {
        match self {
            VarianteModule::Dore => Color::new(0.72, 0.58, 0.28, 1.0),
            VarianteModule::Labo => Color::new(0.80, 0.82, 0.85, 1.0),
            VarianteModule::Gonflable => Color::new(0.84, 0.81, 0.75, 1.0),
            _ => Color::new(0.85, 0.85, 0.88, 1.0),
        }
    }

    /// Détails de surface, dessinés par-dessus le corps (repère local, axe Z,
    /// corps de rayon `rayon` s'étendant de −`demi` à +`demi`).
    fn details(self, rayon: f32, demi: f32) {
        let sombre = Color::new(0.18, 0.20, 0.24, 1.0);
        let vitre = Color::new(0.15, 0.24, 0.34, 1.0);
        match self {
            VarianteModule::Hublots => {
                let n = ((demi * 2.0 / 0.6) as usize).max(2);
                for i in 0..n {
                    let z = -demi + (i as f32 + 0.5) * (2.0 * demi / n as f32);
                    draw_sphere(vec3(0.0, rayon * 0.92, z), rayon * 0.13, None, sombre);
                }
                for s in [-1.0_f32, 1.0] {
                    let x = rayon * 0.72 * s;
                    super::cylindre(vec3(x, rayon * 0.72, -demi * 0.8), vec3(x, rayon * 0.72, demi * 0.8), 0.03, sombre);
                }
            }
            VarianteModule::Labo => {
                // grande fenêtre plate sur +Y + rack externe sur −Y.
                draw_cube(vec3(0.0, rayon, 0.0), vec3(rayon * 0.9, 0.06, demi * 1.1), None, vitre);
                draw_cube(vec3(0.0, -rayon * 1.05, 0.0), vec3(rayon * 0.7, rayon * 0.3, demi * 0.8), None, sombre);
            }
            VarianteModule::Gonflable => {
                // bombement central (fabric BEAM) : une sphère aplatie au milieu.
                draw_sphere(Vec3::ZERO, rayon * 1.3, None, self.couleur());
            }
            VarianteModule::Coupole => {
                super::cone(vec3(0.0, 0.0, demi), Vec3::Z, rayon * 0.6, rayon * 0.32, rayon * 0.5, vitre);
            }
            VarianteModule::Sas => {
                // Écoutille EVA saillante sur +X + main courante.
                draw_cube(vec3(rayon * 1.05, 0.0, 0.0), vec3(0.4, rayon * 0.7, rayon * 0.7), None, sombre);
                super::cylindre(
                    vec3(rayon * 0.9, rayon * 0.6, -demi * 0.6),
                    vec3(rayon * 0.9, rayon * 0.6, demi * 0.6),
                    0.03,
                    sombre,
                );
            }
            VarianteModule::Standard | VarianteModule::Dore => {}
        }
    }
}

impl Composant {
    /// Ports dans le repère **local** du composant (montage + hôtes libres,
    /// indistincts : on marque l'occupé à l'assemblage). Convention `Repere` :
    /// `avant = rot*Z` sortant, `haut = rot*Y`.
    pub fn ports(&self) -> Vec<Port> {
        match self {
            Composant::ModuleAxial { profil, longueur, .. } => {
                // Le port se pose au **bout de la collerette** (offset de docking).
                let tip = longueur * 0.5 + profil.rayon() * COL_LONG;
                let mut v = vec![
                    // Bout +Z : avant = +Z (rot identité), haut = +Y.
                    Port::new(
                        Repere::new(vec3(0.0, 0.0, tip), Quat::IDENTITY),
                        GenrePort::ModuleAxial,
                        *profil,
                    ),
                    // Bout −Z : demi-tour autour du haut → avant = −Z, haut = +Y.
                    Port::new(
                        Repere::new(vec3(0.0, 0.0, -tip), Quat::from_rotation_y(PI)),
                        GenrePort::ModuleAxial,
                        *profil,
                    ),
                ];
                // Ports hôtes `Surface` radiaux (±X, ±Y) sur le flanc, pour
                // accueillir panneaux / radiateurs / antennes (stations type Mir).
                for (dir, rot) in faces_principales().into_iter().take(4) {
                    v.push(Port::new(Repere::new(dir * profil.rayon(), rot), GenrePort::Surface, Profil::P0));
                }
                v
            }
            Composant::Noeud { profil, sorties } => {
                // Chaque port se pose au bout de sa collerette : sphère + bras + col.
                let t = profil.rayon() * (NOEUD_SPHERE + BRAS_LONG + COL_LONG);
                let faces = faces_noeud(*sorties);
                let mut v: Vec<Port> = faces
                    .iter()
                    .map(|(dir, rot, genre)| Port::new(Repere::new(*dir * t, *rot), *genre, *profil))
                    .collect();
                // Ports hôtes `Surface` sur les directions principales **libres**
                // (non occupées par un bras) — pour appendices sur le nœud.
                let rs = profil.rayon() * NOEUD_SPHERE;
                for (dir, rot) in faces_principales() {
                    if !faces.iter().any(|(d, _, _)| d.dot(dir) > 0.99) {
                        v.push(Port::new(Repere::new(dir * rs, rot), GenrePort::Surface, Profil::P0));
                    }
                }
                v
            }
            Composant::PanneauSolaire { profil, .. } => {
                // Unique port de montage : avant vers l'hôte (−Z), le panneau
                // déploie de l'autre côté (+Z). Se pose sur un port hôte `Surface`.
                vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                    GenrePort::Surface,
                    *profil,
                )]
            }
            Composant::Treillis { profil, longueur, .. } => {
                let demi = longueur * 0.5;
                let sx = profil.rayon() * TREILLIS_SECTION; // sortie latérale
                let mut v = vec![
                    // Bouts axiaux (chaînables avec modules/nœuds).
                    Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, *profil),
                    Port::new(
                        Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)),
                        GenrePort::ModuleAxial,
                        *profil,
                    ),
                ];
                // Ports hôtes `Surface` (profil P0) par paires ±X, répartis sur la
                // longueur — accueillent panneau, radiateur ou antenne indifféremment.
                let paires = ((longueur / TREILLIS_PAS_AILE) as i32).max(1);
                for k in 0..paires {
                    let z = -demi + (k as f32 + 0.5) * (longueur / paires as f32);
                    v.push(Port::new(
                        Repere::new(vec3(sx, 0.0, z), Quat::from_rotation_y(FRAC_PI_2)),
                        GenrePort::Surface,
                        Profil::P0,
                    ));
                    v.push(Port::new(
                        Repere::new(vec3(-sx, 0.0, z), Quat::from_rotation_y(-FRAC_PI_2)),
                        GenrePort::Surface,
                        Profil::P0,
                    ));
                }
                v
            }
            Composant::Radiateur { profil, .. } => {
                // Unique port de montage : avant vers l'hôte (−Z), déploie en +Z.
                vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                    GenrePort::Surface,
                    *profil,
                )]
            }
            Composant::Antenne { profil, .. } => {
                // Même montage générique `Surface`, avant vers l'hôte (−Z).
                vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                    GenrePort::Surface,
                    *profil,
                )]
            }
            Composant::Adaptateur { grand, petit, longueur } => {
                // Deux écoutilles axiales de profils différents, au bout des cols.
                let demi = longueur * 0.5;
                vec![
                    Port::new(
                        Repere::new(vec3(0.0, 0.0, -(demi + grand.rayon() * COL_LONG)), Quat::from_rotation_y(PI)),
                        GenrePort::ModuleAxial,
                        *grand,
                    ),
                    Port::new(
                        Repere::new(vec3(0.0, 0.0, demi + petit.rayon() * COL_LONG), Quat::IDENTITY),
                        GenrePort::ModuleAxial,
                        *petit,
                    ),
                ]
            }
        }
    }

    /// Dessine dans le repère **local** (la transformée monde est déjà poussée
    /// par l'appelant via `push_model_matrix`).
    pub fn dessiner(&self) {
        match self {
            Composant::ModuleAxial { profil, longueur, variante } => {
                let rayon = profil.rayon();
                let demi = *longueur * 0.5;
                let lc = rayon * COL_LONG;
                let rc = rayon * COL_RAYON;
                // Corps : cylindre lisse, teinté par la variante.
                super::cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, demi), rayon, variante.couleur());
                // Embouts : un petit cylindre coiffe chaque disque de bout. Il
                // chevauche le corps (part de `demi - EMBOUT_ENFONCE`) → aucune
                // face coplanaire, donc plus de z-fighting ; léger débord = arête.
                let re = rayon * EMBOUT_RAYON;
                super::cylindre(vec3(0.0, 0.0, demi - EMBOUT_ENFONCE), vec3(0.0, 0.0, demi + EMBOUT_LONG), re, SOMBRE);
                super::cylindre(vec3(0.0, 0.0, -demi + EMBOUT_ENFONCE), vec3(0.0, 0.0, -demi - EMBOUT_LONG), re, SOMBRE);
                // Collerettes de docking : cols étroits qui dépassent à chaque bout.
                super::cylindre(vec3(0.0, 0.0, demi), vec3(0.0, 0.0, demi + lc), rc, SOMBRE);
                super::cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, -demi - lc), rc, SOMBRE);
                // Détails de surface (hublots, fenêtre, coupole, bombement…).
                variante.details(rayon, demi);
            }
            Composant::Noeud { profil, sorties } => {
                let rn = profil.rayon();
                let rs = rn * NOEUD_SPHERE; // sphère gonflée
                let lb = rn * BRAS_LONG;
                let rb = rn * BRAS_RAYON;
                let lc = rn * COL_LONG;
                let rc = rn * COL_RAYON;
                // Base du bras enfoncée dans la sphère (jonction propre, pas tangente).
                let base = rs - rn * JONCTION_OFFSET;
                // Corps sphérique (pas de disque de bout → pas de z-fighting).
                draw_sphere(Vec3::ZERO, rs, None, COULEUR);
                for (dir, _, _) in faces_noeud(*sorties) {
                    // Bras cylindrique ancré dans la sphère, puis collerette au bout.
                    super::cylindre(dir * base, dir * (rs + lb), rb, COULEUR);
                    super::cylindre(dir * (rs + lb), dir * (rs + lb + lc), rc, SOMBRE);
                }
            }
            Composant::PanneauSolaire { variante, longueur, largeur, .. } => {
                // Jonction côté hôte (−Z) : un bras qui rejoint l'hôte + un petit
                // socle (gimbal) à l'origine — c'est la liaison module ↔ panneau.
                super::cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
                draw_cube(Vec3::ZERO, Vec3::splat(0.22), None, COULEUR);
                draw_cube_wires(Vec3::ZERO, Vec3::splat(0.22), SOMBRE);
                // Mât depuis le socle (origine) vers +Z, puis la pale selon la variante.
                let pied = vec3(0.0, 0.0, MAST_PANNEAU);
                super::cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
                let (col, lf, wf) = variante.style();
                let (lon, lar) = (longueur * lf, largeur * wf);
                match variante {
                    VariantePanneau::Hexagonal => {
                        super::pieces::pale_hexagonale(pied, Vec3::Z, Vec3::X, lon, lar, col);
                    }
                    _ => {
                        let cellules = (lon / 0.35).max(2.0) as usize;
                        super::pieces::pale_solaire(pied, Vec3::Z, Vec3::X, lon, lar, cellules, col);
                    }
                }
            }
            Composant::Treillis { profil, longueur, style } => {
                let demi = longueur * 0.5;
                let sec = profil.rayon() * TREILLIS_SECTION;
                let (a, b) = (vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, demi));
                match style {
                    StyleTreillis::Carre => super::pieces::treillis(a, b, sec, COULEUR, SOMBRE),
                    StyleTreillis::Triangulaire => {
                        super::pieces::treillis_triangulaire(a, b, sec, COULEUR, SOMBRE)
                    }
                }
            }
            Composant::Radiateur { variante, longueur, largeur, .. } => {
                // Jonction hôte (bras + socle) puis mât, comme le panneau.
                super::cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
                draw_cube(Vec3::ZERO, Vec3::splat(0.2), None, COULEUR);
                draw_cube_wires(Vec3::ZERO, Vec3::splat(0.2), SOMBRE);
                let pied = vec3(0.0, 0.0, MAST_PANNEAU);
                super::cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
                variante.dessiner(pied, *longueur, *largeur);
            }
            Composant::Antenne { variante, taille, .. } => {
                // Jonction hôte (bras + socle) puis mât court, puis l'antenne.
                super::cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
                draw_cube(Vec3::ZERO, Vec3::splat(0.2), None, COULEUR);
                draw_cube_wires(Vec3::ZERO, Vec3::splat(0.2), SOMBRE);
                let pied = vec3(0.0, 0.0, MAST_PANNEAU);
                super::cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
                variante.dessiner(pied, *taille);
            }
            Composant::Adaptateur { grand, petit, longueur } => {
                let demi = *longueur * 0.5;
                // Tronc de cône grand (−Z) → petit (+Z) + collerettes de docking.
                super::cone(vec3(0.0, 0.0, -demi), Vec3::Z, grand.rayon(), petit.rayon(), *longueur, COULEUR);
                super::cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, -demi - grand.rayon() * COL_LONG), grand.rayon() * COL_RAYON, SOMBRE);
                super::cylindre(vec3(0.0, 0.0, demi), vec3(0.0, 0.0, demi + petit.rayon() * COL_LONG), petit.rayon() * COL_RAYON, SOMBRE);
            }
        }
    }

    /// Coût de rendu ≈ nombre de primitives dessinées (pondère le `Budget`,
    /// fondations §3.1).
    pub fn cout(&self) -> f32 {
        match self {
            // corps + 2 embouts + 2 collerettes de docking = 5.
            Composant::ModuleAxial { .. } => 5.0,
            // sphère + (bras + collerette) par sortie.
            Composant::Noeud { sorties, .. } => 1.0 + 2.0 * faces_noeud(*sorties).len() as f32,
            // mât + pale nervurée : poids représentatif (une aile ≫ un tube nu).
            Composant::PanneauSolaire { .. } => 6.0,
            // treillis ajouré : coût qui croît avec la longueur (baies de plus).
            Composant::Treillis { longueur, .. } => 2.0 + longueur,
            // radiateur : coût selon la technologie (accordéon/LDR plus lourds).
            Composant::Radiateur { variante, .. } => variante.cout(),
            // antenne : coût léger selon le type.
            Composant::Antenne { variante, .. } => variante.cout(),
            // adaptateur : cône + 2 collerettes.
            Composant::Adaptateur { .. } => 3.0,
        }
    }

    /// Rayon englobant **local** (remplace l'ancien `Piece.profil` pour la
    /// sphère de `Station`) : la plus grande extension, radiale ou axiale.
    pub fn rayon_local(&self) -> f32 {
        match self {
            Composant::ModuleAxial { profil, longueur, .. } => {
                // Extension axiale (jusqu'au bout du col) ou radiale, la plus grande.
                (longueur * 0.5 + profil.rayon() * COL_LONG).max(profil.rayon())
            }
            // Sphère + bras + collerette : rayon jusqu'au bout des sorties.
            Composant::Noeud { profil, .. } => profil.rayon() * (NOEUD_SPHERE + BRAS_LONG + COL_LONG),
            // Diagonale mât+déploiement / demi-largeur (borne haute avec le facteur
            // de longueur max des variantes, ~1.25).
            Composant::PanneauSolaire { longueur, largeur, .. } => {
                (MAST_PANNEAU + longueur * 1.25).hypot(largeur * 0.5)
            }
            // Demi-longueur de la poutre (l'extension dominante).
            Composant::Treillis { profil, longueur, .. } => {
                longueur * 0.5 + profil.rayon() * TREILLIS_SECTION
            }
            // Diagonale déploiement / demi-largeur (largeur élargie pour « Corps »).
            Composant::Radiateur { longueur, largeur, .. } => {
                (MAST_PANNEAU + longueur * 1.25).hypot(largeur * 0.8)
            }
            // Antenne : mât + taille (les fouets/hélice dépassent un peu).
            Composant::Antenne { taille, .. } => MAST_PANNEAU + taille * 1.5,
            // Adaptateur : jusqu'au bout du col du grand côté.
            Composant::Adaptateur { grand, longueur, .. } => {
                (longueur * 0.5 + grand.rayon() * COL_LONG).max(grand.rayon())
            }
        }
    }

    /// Sphère englobante **locale** `(centre, rayon)` pour l'anti-collision. Les
    /// composants structurels sont centrés sur l'origine ; les appendices se
    /// déploient le long de +Z, donc leur sphère est décalée à mi-déploiement
    /// (sinon, centrée sur le montage, elle recouvrirait à tort les voisins).
    pub fn englobant_local(&self) -> (Vec3, f32) {
        match self {
            Composant::ModuleAxial { .. }
            | Composant::Noeud { .. }
            | Composant::Treillis { .. }
            | Composant::Adaptateur { .. } => (Vec3::ZERO, self.rayon_local()),
            Composant::PanneauSolaire { .. }
            | Composant::Radiateur { .. }
            | Composant::Antenne { .. } => {
                let r = self.rayon_local();
                (Vec3::Z * (r * 0.5), r * 0.55)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2a — un module axial expose exactement deux écoutilles opposées, de même
    // genre/profil, hauts alignés, aux deux bouts.
    #[test]
    fn module_axial_deux_ports_opposes() {
        let long = 3.0;
        let c = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: long };
        let ports = c.ports();

        // Deux écoutilles axiales (les autres ports sont des montages Surface).
        let axiaux: Vec<_> = ports.iter().filter(|p| p.genre == GenrePort::ModuleAxial).collect();
        assert_eq!(axiaux.len(), 2);
        for p in &axiaux {
            assert_eq!(p.profil, Profil::P1);
        }

        // Avants opposés (somme ≈ 0).
        assert!((axiaux[0].repere.avant() + axiaux[1].repere.avant()).length() < 1e-5);

        // Hauts tous deux alignés sur +Y (roulis cohérent).
        assert!((axiaux[0].repere.haut() - Vec3::Y).length() < 1e-5);
        assert!((axiaux[1].repere.haut() - Vec3::Y).length() < 1e-5);

        // Positions au bout des collerettes : ±(demi-longueur + col) en Z.
        let tip = long * 0.5 + Profil::P1.rayon() * COL_LONG;
        assert!((axiaux[0].repere.pos - vec3(0.0, 0.0, tip)).length() < 1e-5);
        assert!((axiaux[1].repere.pos - vec3(0.0, 0.0, -tip)).length() < 1e-5);

        // 4 ports hôtes Surface radiaux (±X, ±Y).
        assert_eq!(ports.iter().filter(|p| p.genre == GenrePort::Surface).count(), 4);
    }

    // Coût et rayon local cohérents.
    #[test]
    fn cout_et_rayon_local() {
        let c = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 };
        assert_eq!(c.cout(), 5.0);
        // demi-longueur 1.5 + col (0.25 × rayon P1) = 1.75 > rayon P1 → 1.75.
        assert_eq!(c.rayon_local(), 1.75);

        // Module court et gros : le rayon domine.
        let trapu = Composant::ModuleAxial { profil: Profil::P3, variante: VarianteModule::Standard, longueur: 1.0 };
        assert_eq!(trapu.rayon_local(), Profil::P3.rayon()); // 3.0
    }

    // Toutes les variantes d'habitat gardent les 2 écoutilles axiales.
    #[test]
    fn toutes_variantes_module() {
        for v in VarianteModule::TOUS {
            let m = Composant::ModuleAxial { profil: Profil::P1, variante: v, longueur: 3.0 };
            let axiaux = m.ports().iter().filter(|p| p.genre == GenrePort::ModuleAxial).count();
            assert_eq!(axiaux, 2, "{}", v.nom());
        }
    }

    // Adaptateur : 2 écoutilles axiales de profils distincts, avants opposés.
    #[test]
    fn adaptateur_relie_deux_profils() {
        let a = Composant::Adaptateur { grand: Profil::P2, petit: Profil::P1, longueur: 2.0 };
        let ports = a.ports();
        assert_eq!(ports.len(), 2);
        for p in &ports {
            assert_eq!(p.genre, GenrePort::ModuleAxial);
        }
        assert_eq!(ports[0].profil, Profil::P2);
        assert_eq!(ports[1].profil, Profil::P1);
        assert!((ports[0].repere.avant() + ports[1].repere.avant()).length() < 1e-5);
    }

    // Invariant commun : chaque port (bras OU montage Surface) a son **avant**
    // pointant vers l'extérieur ; et le nœud a `attendu` ports « module » (bras).
    fn verifie_ports_sortants(n: &Composant, attendu: usize) {
        let ports = n.ports();
        for p in &ports {
            let dir = p.repere.pos.normalize();
            assert!((p.repere.avant() - dir).length() < 1e-5, "avant sortant {dir:?}");
        }
        let modules = ports
            .iter()
            .filter(|p| matches!(p.genre, GenrePort::ModuleAxial | GenrePort::ModuleRadial))
            .count();
        assert_eq!(modules, attendu);
    }

    // Nœud 6 sorties : 6 ports (2 axiaux + 4 radiaux), avants sortants.
    #[test]
    fn noeud_six_ports_sortants() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
        verifie_ports_sortants(&n, 6);
        let ports = n.ports();
        assert_eq!(ports.iter().filter(|p| p.genre == GenrePort::ModuleAxial).count(), 2);
        assert_eq!(ports.iter().filter(|p| p.genre == GenrePort::ModuleRadial).count(), 4);
        assert_eq!(n.cout(), 13.0); // sphère + 6 × (bras + collerette)
    }

    // Nœud 4 sorties (croix plane), avants sortants.
    #[test]
    fn noeud_quatre_ports() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Quatre };
        verifie_ports_sortants(&n, 4);
        assert_eq!(n.cout(), 9.0); // sphère + 4 × (bras + collerette)
    }

    // Nœud en T : 3 ports dans le plan XZ (barre ±X + tige −Z).
    #[test]
    fn noeud_t_trois_ports_plan_xz() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::T };
        verifie_ports_sortants(&n, 3);
        // Les bras (ports module) restent dans le plan XZ → composante Y nulle.
        for p in n.ports().iter().filter(|p| matches!(p.genre, GenrePort::ModuleAxial | GenrePort::ModuleRadial)) {
            assert!(p.repere.pos.y.abs() < 1e-5, "barre T dans le plan XZ");
        }
        assert_eq!(n.cout(), 7.0); // sphère + 3 × (bras + collerette)
    }

    // Nœud tétraédrique : 4 sorties isotropes, avants sortants (rotation par arc).
    #[test]
    fn noeud_tetra_quatre_sorties() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Tetra };
        verifie_ports_sortants(&n, 4);
        assert_eq!(n.cout(), 9.0);
    }

    // Un module (port axial) est compatible avec un port radial du nœud (genres
    // Axial/Radial groupés, même profil).
    #[test]
    fn module_compatible_avec_noeud() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
        let m = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };
        let radial = n.ports()[2]; // un port radial du nœud
        let montage = m.ports()[1]; // écoutille de montage du module
        assert!(montage.compatible(&radial));
    }

    // Panneau solaire : un unique port Surface, avant vers l'hôte (−Z).
    #[test]
    fn panneau_un_port_montage_aile() {
        let p = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 3.0,
            largeur: 1.2,
        };
        let ports = p.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::Surface);
        assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
    }

    // Un montage d'appendice (Surface) s'apparie avec un autre Surface, pas un module.
    #[test]
    fn panneau_compatibilite() {
        let aile = |v| Composant::PanneauSolaire { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
        let p = aile(VariantePanneau::RigideUS);
        let m = Composant::ModuleAxial { profil: Profil::P0, variante: VarianteModule::Standard, longueur: 2.0 };
        assert!(!p.ports()[0].compatible(&m.ports()[0]), "Surface ≠ ModuleAxial");
        assert!(p.ports()[0].compatible(&aile(VariantePanneau::Futuriste).ports()[0]), "deux appendices Surface");
    }

    // Toutes les variantes exposent bien un unique port Surface.
    #[test]
    fn toutes_variantes_un_port_montage() {
        for v in VariantePanneau::TOUS {
            let p = Composant::PanneauSolaire { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
            let ports = p.ports();
            assert_eq!(ports.len(), 1, "{}", v.nom());
            assert_eq!(ports[0].genre, GenrePort::Surface);
        }
    }

    // Treillis : 2 bouts axiaux opposés + des montages d'ailes par paires ±X.
    #[test]
    fn treillis_bouts_et_montages() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre };
        let ports = t.ports();

        let axiaux: Vec<_> = ports.iter().filter(|p| p.genre == GenrePort::ModuleAxial).collect();
        assert_eq!(axiaux.len(), 2, "deux bouts");
        assert!((axiaux[0].repere.avant() + axiaux[1].repere.avant()).length() < 1e-5, "bouts opposés");

        let ailes: Vec<_> = ports.iter().filter(|p| p.genre == GenrePort::Surface).collect();
        assert!(ailes.len() >= 2 && ailes.len() % 2 == 0, "paires d'ailes");
        for p in &ailes {
            assert_eq!(p.profil, Profil::P0);
            assert!(p.repere.avant().x.abs() > 0.99, "avant latéral ±X"); // sortant sur X
            assert!(p.repere.avant().y.abs() + p.repere.avant().z.abs() < 1e-5);
        }
    }

    // Un panneau (P0) est compatible avec un montage d'aile du treillis (P0).
    #[test]
    fn panneau_dock_sur_treillis() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 6.0, style: StyleTreillis::Triangulaire };
        let aile = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 2.0,
            largeur: 1.0,
        };
        let mont = t.ports().into_iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        assert!(aile.ports()[0].compatible(&mont));
    }

    // Radiateur : un unique port Surface, avant vers l'hôte (−Z).
    #[test]
    fn radiateur_port_montage() {
        let r = Composant::Radiateur {
            profil: Profil::P0,
            variante: VarianteRadiateur::PanneauSimple,
            longueur: 3.0,
            largeur: 1.0,
        };
        let ports = r.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::Surface);
        assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
    }

    // Factorisation : radiateur, panneau (et antenne) partagent le montage
    // `Surface` → compatibles entre eux ; mais pas avec un port de module.
    #[test]
    fn radiateur_compatibilite() {
        let rad = |v| Composant::Radiateur { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
        let r = rad(VarianteRadiateur::Caloducs);
        let aile = Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 2.0, largeur: 1.0 };
        let module = Composant::ModuleAxial { profil: Profil::P0, variante: VarianteModule::Standard, longueur: 2.0 };
        assert!(r.ports()[0].compatible(&aile.ports()[0]), "montages Surface factorisés");
        assert!(!r.ports()[0].compatible(&module.ports()[0]), "pas sur un port de module");
    }

    // Toutes les variantes de radiateur exposent un unique port Surface.
    #[test]
    fn toutes_variantes_radiateur() {
        for v in VarianteRadiateur::TOUS {
            let r = Composant::Radiateur { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
            assert_eq!(r.ports().len(), 1, "{}", v.nom());
            assert_eq!(r.ports()[0].genre, GenrePort::Surface);
        }
    }

    // Antenne : un unique port Surface, avant vers l'hôte (−Z) ; idem toutes variantes.
    #[test]
    fn toutes_variantes_antenne() {
        for v in VarianteAntenne::TOUS {
            let ant = Composant::Antenne { profil: Profil::P0, variante: v, taille: 1.0 };
            let ports = ant.ports();
            assert_eq!(ports.len(), 1, "{}", v.nom());
            assert_eq!(ports[0].genre, GenrePort::Surface);
            assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
        }
    }

    // Factorisation : un treillis accueille indifféremment panneau, radiateur et
    // antenne sur ses ports hôtes Surface (même profil).
    #[test]
    fn treillis_accueille_tous_appendices() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 6.0, style: StyleTreillis::Carre };
        let hote = t.ports().into_iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        let panneau = Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 2.0, largeur: 1.0 };
        let radiateur = Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::PanneauSimple, longueur: 2.0, largeur: 1.0 };
        let antenne = Composant::Antenne { profil: Profil::P0, variante: VarianteAntenne::ParaboleGG, taille: 1.0 };
        for app in [panneau, radiateur, antenne] {
            assert!(app.ports()[0].compatible(&hote), "appendice sur port hôte Surface");
        }
    }
}
