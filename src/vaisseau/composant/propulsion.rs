//! **Propulsion classique** : les neuf variantes de propulseur (chimique,
//! électrique, nucléaire), la nacelle motrice et la brique de bloc moteur.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::Enveloppe;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};


/// Grandes familles de propulsion — une **vue par famille**, leurs silhouettes
/// n'ayant presque rien en commun.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FamillePropulsion {
    /// Ergols brûlés : tuyère à divergent, forte poussée, formes massives.
    Chimique,
    /// Ions accélérés : corps courts, grilles et anneaux, faible poussée.
    Electrique,
    /// Réacteur embarqué : cuve, bouclier, très gros par rapport à la tuyère.
    Nucleaire,
}

impl FamillePropulsion {
    pub const TOUTES: [FamillePropulsion; 3] = [
        FamillePropulsion::Chimique,
        FamillePropulsion::Electrique,
        FamillePropulsion::Nucleaire,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            FamillePropulsion::Chimique => "CHIMIQUE",
            FamillePropulsion::Electrique => "ELECTRIQUE",
            FamillePropulsion::Nucleaire => "NUCLEAIRE",
        }
    }
}

/// Variantes de [`Composant::Propulseur`]. Chacune sait à quelle famille elle
/// appartient et **comment elle se monte** : un moteur principal se boulonne en
/// bout de corps (écoutille axiale), une grappe de contrôle se pose sur un flanc
/// (port `Surface`, comme les appendices).
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VariantePropulseur {
    /// Moteur à ergols liquides : chambre, col, divergent à tubes.
    TuyereCloche,
    /// Propergol solide : long étui segmenté, tuyère courte, pas de plomberie.
    BoosterSolide,
    /// Grappe de contrôle d'attitude : bloc et tuyères en éventail.
    GrappeRcs,
    /// Petit moteur hypergolique de maintien à poste, avec sa sphère d'ergol.
    Hypergolique,
    /// Ionique à grilles : corps court fermé par une **grille perforée plate**.
    IoniqueGrille,
    /// Effet Hall : **canal annulaire** autour d'un cœur central, bobines.
    EffetHall,
    /// Plasma pulsé : petit boîtier à rails d'électrodes.
    PlasmaPulse,
    /// VASIMR : **anneaux de bobines** en enfilade et tuyère magnétique évasée.
    Vasimr,
    /// Thermique nucléaire (NERVA) : cuve réacteur large sur tuyère régénérative.
    ThermiqueNerva,
    /// Nucléaire-électrique : réacteur, **bouclier conique**, radiateurs, tuyère.
    NucleaireElectrique,
}

impl VariantePropulseur {
    pub const TOUS: [VariantePropulseur; 10] = [
        VariantePropulseur::TuyereCloche,
        VariantePropulseur::BoosterSolide,
        VariantePropulseur::GrappeRcs,
        VariantePropulseur::Hypergolique,
        VariantePropulseur::IoniqueGrille,
        VariantePropulseur::EffetHall,
        VariantePropulseur::PlasmaPulse,
        VariantePropulseur::Vasimr,
        VariantePropulseur::ThermiqueNerva,
        VariantePropulseur::NucleaireElectrique,
    ];

    pub fn famille(self) -> FamillePropulsion {
        use VariantePropulseur::*;
        match self {
            TuyereCloche | BoosterSolide | GrappeRcs | Hypergolique => FamillePropulsion::Chimique,
            IoniqueGrille | EffetHall | PlasmaPulse | Vasimr => FamillePropulsion::Electrique,
            ThermiqueNerva | NucleaireElectrique => FamillePropulsion::Nucleaire,
        }
    }

    /// `true` = moteur principal, monté par une **écoutille axiale** en bout de
    /// corps. `false` = grappe posée sur un flanc, port `Surface`.
    pub fn axial(self) -> bool {
        !matches!(
            self,
            VariantePropulseur::GrappeRcs
                | VariantePropulseur::Hypergolique
                | VariantePropulseur::PlasmaPulse
        )
    }

    pub fn nom(self) -> &'static str {
        match self {
            VariantePropulseur::TuyereCloche => "TUYERE CLOCHE (ERGOLS LIQUIDES)",
            VariantePropulseur::BoosterSolide => "PROPERGOL SOLIDE",
            VariantePropulseur::GrappeRcs => "GRAPPE RCS",
            VariantePropulseur::Hypergolique => "HYPERGOLIQUE (MAINTIEN A POSTE)",
            VariantePropulseur::IoniqueGrille => "IONIQUE A GRILLES",
            VariantePropulseur::EffetHall => "EFFET HALL",
            VariantePropulseur::PlasmaPulse => "PLASMA PULSE",
            VariantePropulseur::Vasimr => "VASIMR (MAGNETOPLASMA)",
            VariantePropulseur::ThermiqueNerva => "THERMIQUE NUCLEAIRE (NERVA)",
            VariantePropulseur::NucleaireElectrique => "NUCLEAIRE-ELECTRIQUE",
        }
    }

    pub(super) fn cout(self) -> f32 {
        match self {
            VariantePropulseur::TuyereCloche => 12.0,
            VariantePropulseur::BoosterSolide => 6.0,
            VariantePropulseur::GrappeRcs => 8.0,
            VariantePropulseur::Hypergolique => 5.0,
            VariantePropulseur::IoniqueGrille => 9.0,
            VariantePropulseur::EffetHall => 10.0,
            VariantePropulseur::PlasmaPulse => 4.0,
            VariantePropulseur::Vasimr => 14.0,
            VariantePropulseur::ThermiqueNerva => 18.0,
            VariantePropulseur::NucleaireElectrique => 20.0,
        }
    }

    /// Dessine le propulseur depuis `base`. `sens` vaut −1 pour un moteur axial
    /// (il s'enfonce vers l'arrière du corps porteur) et +1 pour une grappe
    /// posée sur un flanc : la géométrie est identique, seule la direction de
    /// sortie change.
    fn dessiner<P: Peintre>(self, p: &mut P, base: Vec3, sens: f32, taille: f32) {
        let metal = Color::new(0.62, 0.64, 0.68, 1.0);
        let sombre = Color::new(0.20, 0.22, 0.26, 1.0);
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        let cuivre = Color::new(0.66, 0.45, 0.28, 1.0);
        let lueur = Color::new(0.42, 0.66, 0.95, 1.0);
        let (d, w, h) = (Vec3::Z * sens, Vec3::X, Vec3::Y);
        let t = taille;
        match self {
            VariantePropulseur::TuyereCloche => {
                // Chambre, col resserré, puis divergent évasé nervuré de tubes.
                p.cylindre(base, base + d * (t * 0.34), t * 0.30, clair);
                p.cone(base + d * (t * 0.34), d, t * 0.30, t * 0.16, t * 0.16, metal);
                p.cone(base + d * (t * 0.50), d, t * 0.16, t * 0.52, t * 0.62, metal);
                for k in 0..10 {
                    let a = TAU * k as f32 / 10.0;
                    let dir = w * a.cos() + h * a.sin();
                    p.cylindre(
                        base + d * (t * 0.52) + dir * (t * 0.17),
                        base + d * (t * 1.10) + dir * (t * 0.52),
                        t * 0.022,
                        sombre,
                    );
                }
                // Turbopompe et conduite, sur le côté.
                let tp = base + d * (t * 0.18) + w * (t * 0.34);
                p.cylindre(tp, tp + d * (t * 0.20), t * 0.11, sombre);
                p.cylindre(tp, base + d * (t * 0.05), t * 0.04, metal);
            }
            VariantePropulseur::BoosterSolide => {
                // Étui long segmenté + tuyère courte : aucune plomberie.
                p.cylindre(base, base + d * (t * 1.05), t * 0.30, clair);
                for k in 1..4 {
                    let z = t * 1.05 * k as f32 / 4.0;
                    p.cylindre(
                        base + d * (z - t * 0.012),
                        base + d * (z + t * 0.012),
                        t * 0.315,
                        sombre,
                    );
                }
                p.cone(base + d * (t * 1.05), d, t * 0.26, t * 0.40, t * 0.34, metal);
            }
            VariantePropulseur::GrappeRcs => {
                // Bloc d'alimentation + quatre tuyères en éventail.
                p.cube(base + d * (t * 0.18), Vec3::splat(t * 0.34), clair);
                for k in 0..4 {
                    let a = TAU * k as f32 / 4.0 + 0.78;
                    let dir = (d * 0.72 + (w * a.cos() + h * a.sin()) * 0.69).normalize();
                    let o = base + d * (t * 0.34);
                    p.cone(o, dir, t * 0.07, t * 0.15, t * 0.30, metal);
                }
            }
            VariantePropulseur::Hypergolique => {
                // Petite cloche sur potence + sphère d'ergol.
                p.sphere(base + d * (t * 0.26) + h * (t * 0.20), t * 0.20, clair);
                p.cylindre(base, base + d * (t * 0.30), t * 0.05, metal);
                p.cone(base + d * (t * 0.30), d, t * 0.09, t * 0.26, t * 0.34, metal);
            }
            VariantePropulseur::IoniqueGrille => {
                // Corps court fermé par une **grille perforée plate** : c'est la
                // signature de l'ionique face au Hall.
                p.cylindre(base, base + d * (t * 0.46), t * 0.38, clair);
                let g = base + d * (t * 0.46);
                p.cylindre(g, g + d * (t * 0.05), t * 0.40, sombre);
                for k in 1..4 {
                    let r = t * 0.40 * k as f32 / 4.0;
                    p.cylindre(g + d * (t * 0.05), g + d * (t * 0.065), r, metal);
                }
                // Cathode neutralisante déportée.
                let c = base + w * (t * 0.42) + d * (t * 0.30);
                p.cylindre(c, c + d * (t * 0.22), t * 0.05, metal);
            }
            VariantePropulseur::EffetHall => {
                // **Canal annulaire** autour d'un cœur central, cerné de bobines.
                p.cylindre(base, base + d * (t * 0.34), t * 0.40, clair);
                let s = base + d * (t * 0.34);
                p.cylindre(s, s + d * (t * 0.06), t * 0.42, sombre); // paroi externe
                p.cylindre(s, s + d * (t * 0.10), t * 0.30, lueur); // canal
                p.cylindre(s, s + d * (t * 0.12), t * 0.16, metal); // cœur central
                for k in 0..6 {
                    let a = TAU * k as f32 / 6.0;
                    let dir = w * a.cos() + h * a.sin();
                    let b = base + dir * (t * 0.40) + d * (t * 0.16);
                    p.cylindre(b, b + d * (t * 0.14), t * 0.06, cuivre); // bobines
                }
            }
            VariantePropulseur::PlasmaPulse => {
                // Boîtier plat à rails d'électrodes et éclateur.
                p.cube(base + d * (t * 0.16), vec3(t * 0.42, t * 0.30, t * 0.32), clair);
                for s in [-1.0_f32, 1.0] {
                    let r = base + d * (t * 0.34) + w * (t * 0.13 * s);
                    p.cube(r, vec3(t * 0.05, t * 0.22, t * 0.20), sombre);
                }
                p.cylindre(base + d * (t * 0.40), base + d * (t * 0.50), t * 0.05, lueur);
            }
            VariantePropulseur::Vasimr => {
                // Enfilade d'**anneaux de bobines** puis tuyère magnétique évasée.
                p.cylindre(base, base + d * (t * 0.78), t * 0.20, sombre);
                for k in 0..4 {
                    let z = t * (0.10 + 0.20 * k as f32);
                    p.cylindre(
                        base + d * (z - t * 0.045),
                        base + d * (z + t * 0.045),
                        t * 0.34,
                        cuivre,
                    );
                }
                p.cone(base + d * (t * 0.78), d, t * 0.22, t * 0.50, t * 0.42, metal);
                p.cylindre(base + d * (t * 0.86), base + d * (t * 1.14), t * 0.12, lueur);
            }
            VariantePropulseur::ThermiqueNerva => {
                // Cuve réacteur large en tête, transition, puis grande tuyère
                // régénérative : le rapport cuve/tuyère fait la silhouette.
                p.cylindre(base, base + d * (t * 0.46), t * 0.42, clair);
                for k in 0..3 {
                    let z = t * (0.10 + 0.13 * k as f32);
                    p.cylindre(base + d * (z - t * 0.02), base + d * (z + t * 0.02), t * 0.44, metal);
                }
                p.cone(base + d * (t * 0.46), d, t * 0.42, t * 0.17, t * 0.22, sombre);
                p.cone(base + d * (t * 0.68), d, t * 0.17, t * 0.54, t * 0.60, metal);
                for k in 0..12 {
                    let a = TAU * k as f32 / 12.0;
                    let dir = w * a.cos() + h * a.sin();
                    p.cylindre(
                        base + d * (t * 0.70) + dir * (t * 0.18),
                        base + d * (t * 1.28) + dir * (t * 0.54),
                        t * 0.02,
                        sombre,
                    );
                }
                let tp = base + d * (t * 0.30) + w * (t * 0.46);
                p.cylindre(tp, tp + d * (t * 0.18), t * 0.10, sombre);
            }
            VariantePropulseur::NucleaireElectrique => {
                // Réacteur compact, **bouclier conique** d'ombre, poutre, grands
                // radiateurs, et une tuyère électrique minuscule au bout.
                p.cylindre(base, base + d * (t * 0.30), t * 0.30, clair);
                p.cone(base + d * (t * 0.30), d, t * 0.34, t * 0.14, t * 0.30, sombre);
                p.cylindre(base + d * (t * 0.60), base + d * (t * 1.10), t * 0.07, metal);
                for s in [-1.0_f32, 1.0] {
                    // Aile de radiateur : dans le plan (axe, largeur), déployée
                    // de part et d'autre de la poutre.
                    let coin = base + d * (t * 0.64) + w * (t * 0.07 * s);
                    p.panneau(coin, d * (t * 0.42), w * (t * 0.55 * s), clair);
                }
                p.cylindre(base + d * (t * 1.10), base + d * (t * 1.22), t * 0.14, sombre);
                p.cylindre(base + d * (t * 1.22), base + d * (t * 1.34), t * 0.10, lueur);
            }
        }
    }
}

/// Une nacelle moteur de l'ISV, en repère local (poussée le long de −Z), posée
/// en `base` avec l'orientation `rot`, échelle `s`. Corps de réaction bagué de
/// 8 anneaux, tuyère évasée classique, cône de plasma émissif à l'intérieur.
pub(super) fn dessiner_moteur_seul<P: Peintre>(p: &mut P, base: Vec3, rot: Quat, s: f32) {
    let xf = |v: Vec3| base + rot * (v * s); // point local → monde
    let dir = rot * Vec3::NEG_Z; // axe de poussée (sortie tuyère)
    let plasma = Color::new(0.7, 0.9, 1.0, 1.0); // jet émissif blanc-bleu

    // --- Corps de réaction (cylindre principal, Z ∈ [1, −3]) ---
    p.cylindre(xf(vec3(0.0, 0.0, 1.0)), xf(vec3(0.0, 0.0, -3.0)), 0.7 * s, DARKGRAY);
    // --- 8 anneaux épais métalliques répartis le long du corps ---
    for k in 0..8 {
        let z = 1.0 - k as f32 * (4.0 / 7.0);
        p.cylindre(xf(vec3(0.0, 0.0, z + 0.12)), xf(vec3(0.0, 0.0, z - 0.12)), 0.85 * s, LIGHTGRAY);
    }
    // --- Tuyère évasée classique (col 0.7 → sortie 1.4 sur 2.5) ---
    p.cone(xf(vec3(0.0, 0.0, -3.0)), dir, 0.7 * s, 1.4 * s, 2.5 * s, DARKGRAY);
    // --- Cône de plasma émissif à l'intérieur de la tuyère ---
    p.cone(xf(vec3(0.0, 0.0, -3.2)), dir, 0.25 * s, 0.9 * s, 2.4 * s, plasma);
}

pub(super) fn ports(profil: Profil, variante: VariantePropulseur, _taille: f32) -> Vec<Port> {
    if variante.axial() {
        // Moteur principal : écoutille axiale, tuyère vers l'arrière.
        vec![Port::new(
            Repere::new(Vec3::ZERO, Quat::IDENTITY),
            GenrePort::ModuleAxial,
            profil,
        )]
    } else {
        // Grappe de contrôle : posée sur un flanc comme un appendice.
        vec![Port::new(
            Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
            GenrePort::Surface,
            profil,
        )]
    }
}

pub(super) fn motrice_ports(profil: Profil, echelle: f32) -> Vec<Port> {
    // Écoutille au nœud d'attache (avant, +Z vers le câble du vaisseau).
    vec![Port::new(
        Repere::new(vec3(0.0, 0.0, 0.5 * echelle), Quat::IDENTITY),
        GenrePort::ModuleAxial,
        profil,
    )]
}

pub(super) fn bloc_ports(profil: Profil, largeur: f32) -> Vec<Port> {
    let pz = largeur * 0.5; // profondeur Z du conteneur
    // Bout −Z de la rangée d'habitats (branchée au ras de la caisse) :
    // caisse(−pz/2) − habitat(3.0). Plus de connecteur intermédiaire.
    let tip_rad = -pz * 0.5 - 3.0;
    vec![
        // −Z : montage vers le radiateur (bout du connecteur B).
        Port::new(Repere::new(vec3(0.0, 0.0, tip_rad), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
        // +Z : chaînage de la suite (face avant de la caisse).
        Port::new(Repere::new(vec3(0.0, 0.0, pz * 0.5), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, variante: VariantePropulseur, taille: f32) {
    // Un moteur axial s'enfonce vers l'arrière (−Z), une grappe sort
    // du flanc (+Z) : même géométrie, sens de sortie opposé.
    let sens = if variante.axial() { -1.0 } else { 1.0 };
    variante.dessiner(p, Vec3::ZERO, sens, taille);
}

pub(super) fn motrice_dessiner<P: Peintre>(p: &mut P, echelle: f32) {
    let s = echelle;
    let carbone = Color::new(0.18, 0.18, 0.20, 1.0);
    // Poutre centrale fine traversant l'ensemble (Z ∈ [2.5, −12]).
    p.cylindre(vec3(0.0, 0.0, 2.5) * s, vec3(0.0, 0.0, -12.0) * s, 0.2 * s, BLACK);
    // Deux grosses sphères carbone alignées sur l'axe Z.
    p.sphere(vec3(0.0, 0.0, 0.0) * s, 2.5 * s, carbone);
    p.sphere(vec3(0.0, 0.0, -4.2) * s, 2.8 * s, carbone);
    // Deux nacelles latérales (Y = ±3.5, Z = −2.5), inclinées 1,5° vers
    // l'extérieur. Radiateurs à part (composant `RadiateurMega`).
    for i in [-1.0_f32, 1.0] {
        let base = vec3(0.0, i * 3.5, -2.5) * s;
        let rot = Quat::from_rotation_x(i * 0.02618); // jet vers l'extérieur
        // Connecteur de fixation reliant la nacelle à la sphère 2.
        p.cone(base, vec3(0.0, -i, 0.0), 0.4 * s, 0.15 * s, 1.2 * s, DARKGRAY);
        dessiner_moteur_seul(p, base, rot, s);
    }
}

pub(super) fn bloc_dessiner<P: Peintre>(p: &mut P, largeur: f32) {
    let w = largeur;
    let hy = w * 0.42; // hauteur du conteneur
    let pz = w * 0.5; // profondeur (Z) du conteneur
    let lmod = 3.0;
    let rhab = hy * 0.5 * 0.66; // rayon habitat (0.66 de l'ancien)
    let dx = hy * 1.05 * 0.66; // pas de la rangée
    let lx = 4.0 * dx + 2.0 * rhab; // longueur caisse = envergure des 5 habitats
    let clair = Color::new(0.80, 0.82, 0.86, 1.0);
    let metal = Color::new(0.62, 0.64, 0.68, 1.0);
    let sombre = Color::new(0.22, 0.24, 0.28, 1.0);
    let beige = Color::new(0.84, 0.81, 0.75, 1.0); // toile d'habitat gonflable (BEAM)
    let sangle = Color::new(0.55, 0.53, 0.48, 1.0); // sangles / cloisons

    // --- Conteneur style ossature (longueur lx = envergure de la
    //     rangée de 5 habitats) : caisse + longerons + cadres + treillis. ---
    p.cube(Vec3::ZERO, vec3(lx, hy, pz), clair);
    let c4 = [
        vec3(-lx * 0.5, -hy * 0.5, 0.0),
        vec3(lx * 0.5, -hy * 0.5, 0.0),
        vec3(lx * 0.5, hy * 0.5, 0.0),
        vec3(-lx * 0.5, hy * 0.5, 0.0),
    ];
    for o in c4 {
        p.cylindre(vec3(0.0, 0.0, -pz * 0.5) + o, vec3(0.0, 0.0, pz * 0.5) + o, w * 0.04, metal);
    }
    for k in 0..=2 {
        let cz = -pz * 0.5 + pz * (k as f32 * 0.5);
        for i in 0..4 {
            let a = c4[i] + vec3(0.0, 0.0, cz);
            let b = c4[(i + 1) % 4] + vec3(0.0, 0.0, cz);
            p.cylindre(a, b, w * 0.03, sombre);
        }
    }
    // Treillis dessus : cadre longitudinal surélevé + traverses.
    let yt = hy * 0.5 + w * 0.12;
    for sx in [-1.0_f32, 1.0] {
        p.cylindre(vec3(sx * lx * 0.3, yt, -pz * 0.5), vec3(sx * lx * 0.3, yt, pz * 0.5), w * 0.03, metal);
    }
    for k in 0..=2 {
        let z = -pz * 0.5 + pz * (k as f32 * 0.5);
        p.cylindre(vec3(-lx * 0.3, yt, z), vec3(lx * 0.3, yt, z), w * 0.025, metal);
        p.cylindre(vec3(-lx * 0.3, hy * 0.5, z), vec3(-lx * 0.3, yt, z), w * 0.02, sombre);
        p.cylindre(vec3(lx * 0.3, hy * 0.5, z), vec3(lx * 0.3, yt, z), w * 0.02, sombre);
    }

    // --- Côté radiateur (−Z) : **rangée de 5 habitats gonflables
    //     (BEAM, id 4)** en ligne selon X, **branchée directement** sur
    //     la face −Z de la caisse (plus de connecteur intermédiaire).
    //     Ø réduit à **0.66** de l'ancien (pas réduit d'autant). ---
    let za = -pz * 0.5; // face −Z de la caisse
    let hb = za; // base des habitats, au ras de la caisse
    let he = hb - lmod; // bout (côté radiateur)
    for i in 0..5 {
        let x = (i as f32 - 2.0) * dx;
        p.cylindre(vec3(x, 0.0, hb), vec3(x, 0.0, he), rhab, beige);
        for k in 1..5 {
            let z = hb - lmod * k as f32 / 5.0;
            p.cylindre(vec3(x, 0.0, z - 0.04), vec3(x, 0.0, z + 0.04), rhab * 1.04, sangle);
        }
        for j in 0..6 {
            let a = TAU * j as f32 / 6.0;
            let (ca, sa) = (a.cos(), a.sin());
            p.cylindre(
                vec3(x + ca * rhab * 1.02, sa * rhab * 1.02, hb),
                vec3(x + ca * rhab * 1.02, sa * rhab * 1.02, he),
                rhab * 0.04,
                sangle,
            );
        }
    }
}

pub(super) fn motrice_cout() -> f32 {
    40.0
}

pub(super) fn bloc_cout(largeur: f32) -> f32 {
    5.0 * largeur
}

/// Le plus long (NERVA, VASIMR) atteint ~1,35 × taille.
pub(super) fn rayon_local(taille: f32) -> f32 {
    taille * 1.4
}

/// Extension max (hub avant → boucliers arrière).
pub(super) fn motrice_rayon_local(echelle: f32) -> f32 {
    12.0 * echelle
}

/// Rangée d'habitats : large en X, longue en −Z.
pub(super) fn bloc_rayon_local(largeur: f32) -> f32 {
    1.3 * largeur
}

/// S'étend d'un seul côté de son montage, comme les appendices — mais vers
/// l'arrière quand il est axial.
pub(super) fn englobant(variante: VariantePropulseur, taille: f32) -> Enveloppe {
    let sens = if variante.axial() { -1.0 } else { 1.0 };
    Enveloppe::sphere(Vec3::Z * (sens * taille * 0.6), taille * 0.95)
}
