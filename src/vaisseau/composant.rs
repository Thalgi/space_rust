//! Composant concret : **enum fermé** qui sait exposer ses ports et se dessiner
//! (voir `docs/conception/stations.md`, Partie C §3 et §4 — sous-étape 2a).
//!
//! Choix acté : dispatch par `match` sur un enum, **pas** de trait objet — KISS,
//! zéro allocation, monomorphisé. Une seule fonction
//! par capacité (`ports`, `dessiner`, `cout`, `rayon_local`), qui `match` sur la
//! variante. Les styles/palettes viendront à l'Étape 5. Composants existants :
//! `ModuleAxial` (cylindre) et `Noeud` (hub sphérique 4 ou 6 sorties).

use super::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use super::peintre::Peintre;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Épaisseur des traits (nervures, arêtes) une fois cuits en géométrie.
/// Ignorée par la sortie immédiate, qui trace un segment d'un pixel.
const TRAIT_FIN: f32 = 0.02;

// Palette provisoire (les styles arriveront à l'Étape 5).
const COULEUR: Color = Color { r: 0.85, g: 0.85, b: 0.88, a: 1.0 };
const SOMBRE: Color = Color { r: 0.25, g: 0.25, b: 0.28, a: 1.0 };
/// Alu clair des bagues d'accostage (bout des collerettes).
const BAGUE: Color = Color { r: 0.62, g: 0.64, b: 0.68, a: 1.0 };

// Panneau solaire : mât entre le montage et le pied de la pale, et bras de base
// côté hôte (−Z) qui matérialise la jonction module ↔ panneau.
const MAST_PANNEAU: f32 = 0.4;
const BASE_ARM_PANNEAU: f32 = 0.3;
/// Hauteur de la platine de fixation d'un caisson : il est boulonné court sur sa
/// structure, contrairement à un panneau qui se déploie au bout d'un mât.
const CAISSON_PLATINE: f32 = 0.16;

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
    /// Caisson technique **non pressurisé** : boîte à ossature, palette
    /// d'expériences, berceau de réservoirs, instrument pointé. Se monte comme
    /// un appendice (port `Surface`) — donc partout où va un radiateur : sur une
    /// poutre ou sur le flanc d'un module.
    Caisson { profil: Profil, variante: VarianteCaisson, longueur: f32, largeur: f32 },
    /// Charge utile posée **sur** une structure ou sur un caisson technique :
    /// plateforme d'expériences, réservoirs, instrument. Un seul port de
    /// montage, comme tout appendice.
    ChargeUtile { profil: Profil, variante: VarianteCharge, longueur: f32, largeur: f32 },
    /// Propulseur. Selon sa variante il se monte **en bout de corps** (écoutille
    /// axiale, moteur principal) ou **sur un flanc** (port `Surface`, grappe de
    /// contrôle d'attitude) — cf. [`VariantePropulseur::axial`].
    Propulseur { profil: Profil, variante: VariantePropulseur, taille: f32 },
    /// Charpente en treillis **à section variable et courbe** : évasée du profil
    /// `grand` (base) au profil `petit` (bout), l'affinement suivant `courbure`
    /// (1 = cône droit, >1 = base évasée puis longue flèche, type ISV).
    /// Écoutilles axiales aux deux bouts, à leurs profils respectifs.
    Charpente { grand: Profil, petit: Profil, longueur: f32, courbure: f32, aiguille: bool },
    /// **Radiateur de mégastructure** : aile en arête de poisson (boom central +
    /// `ailettes` panneaux plats de chaque côté), à l'échelle du vaisseau/km, pas
    /// des petits radiateurs de station. Se monte par un port `Surface`.
    /// Première brique de la famille « méga » (ISV, puis O'Neill, Elysium).
    RadiateurMega { profil: Profil, longueur: f32, largeur: f32, ailettes: usize },
    /// **Nacelle moteur de l'ISV** : nœud d'attache + hub + 4 sphères d'hydrogène
    /// + **deux moteurs jumeaux inclinés de ±1,5°** (corps magnétique, anneaux de
    /// confinement, tuyère évasée, boucliers anti-radiation, radiateurs).
    /// `echelle` met le tout à l'échelle. Écoutille axiale au nœud d'attache.
    Motrice { profil: Profil, echelle: f32 },
    /// **Brique de base du bloc propulsion** (repartie de zéro) : un
    /// parallélépipède (longueur Z = 2·`largeur`, hauteur Y = `largeur`) branché
    /// par un col fin à un **gros cylindre** aligné sur +Z, qui **ne recoupe pas**
    /// la caisse (col + petit intervalle). Écoutilles axiales aux deux bouts.
    BlocMoteur { profil: Profil, largeur: f32 },
    /// **Prototype réservoir de carburant** : cuve **sphérique** à tuiles,
    /// optionnellement cerclée d'une **cage tétraédrique** de 4 barres métal
    /// (`cage`). Écoutilles axiales aux deux bouts.
    Reservoir { profil: Profil, longueur: f32, cage: bool },
    /// **Moteur à antimatière (partie principale)** : corps de confinement
    /// magnétique bagué d'anneaux de bobines (silhouette VASIMR), cœur
    /// d'annihilation émissif, tuyère magnétique évasée et long jet de plasma.
    /// Poussée le long de **−Z** ; unique écoutille axiale de montage (+Z).
    MoteurAntimatiere { profil: Profil, taille: f32 },
    /// **Coiffe de module** : capuchon de nez posé sur l'écoutille axiale d'un
    /// module. `profil` fixe le rayon de base (à accorder au module coiffé) ;
    /// `variante` en fixe la forme. Base en Z=0, nez déployé vers **+Z** ;
    /// unique écoutille axiale de montage (avant −Z, vers le module).
    Coiffe { profil: Profil, variante: VarianteCoiffe },
    /// **Réacteur d'antimatière** : bloc **d'injection/confinement** qui se
    /// branche en amont de la tuyère ([`Composant::MoteurAntimatiere`]). Cuve
    /// sombre ceinturée de **bobines électromagnétiques** (cryostat) et de
    /// **tuyauterie**, coiffée d'un injecteur et de pièges à antiprotons. Base
    /// en Z=0 (écoutille de montage vers −Z, côté tuyère), corps vers **+Z**,
    /// second port axial en tête pour chaîner l'alimentation.
    ReacteurAntimatiere { profil: Profil, taille: f32 },
    /// **Anneau hexagonal en treillis** autonome — le même que celui du pied de
    /// [`Composant::Charpente`] (`aiguille`), mais posé seul. `profil` fixe le
    /// gabarit (côté = `profil.rayon()`). `liaison` > 0 tire **6 montants** de
    /// chaque sommet le long de **+Z local** (→ vers un hexagone identique situé
    /// `liaison` plus loin) : les deux hexagones deviennent un **prisme** relié,
    /// sans écart.
    TreillisHexagone { profil: Profil, liaison: f32 },
}

/// Formes de [`Composant::Coiffe`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VarianteCoiffe {
    /// Demi-dôme lisse (calotte surbaissée fermée), rien ne dépasse sous la base.
    Bombee,
    /// Fût hexagonal légèrement tronconique **fermé par une face hexagonale
    /// plate** (pas de pointe).
    Hexagonale,
    /// Adaptateur d'amarrage : jupe tronconique + anneau d'accostage (type
    /// APAS/IDA) + trappe centrale fermée.
    Amarrage,
}

impl VarianteCoiffe {
    pub const TOUS: [VarianteCoiffe; 3] =
        [VarianteCoiffe::Bombee, VarianteCoiffe::Hexagonale, VarianteCoiffe::Amarrage];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteCoiffe::Bombee => "BOMBEE (DEMI-DOME)",
            VarianteCoiffe::Hexagonale => "HEXAGONALE (FACE PLATE)",
            VarianteCoiffe::Amarrage => "ADAPTATEUR D'AMARRAGE",
        }
    }
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

/// Une nacelle moteur de l'ISV, en repère local (poussée le long de −Z), posée
/// en `base` avec l'orientation `rot`, échelle `s`. Corps de réaction bagué de
/// 8 anneaux, tuyère évasée classique, cône de plasma émissif à l'intérieur.
fn dessiner_moteur_seul<P: Peintre>(p: &mut P, base: Vec3, rot: Quat, s: f32) {
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
/// Demi-hauteur d'un caisson, dérivée de sa largeur. Partagée par le dessin et
/// par le calcul des **ports hôtes**, pour qu'ils tombent bien sur les faces.
fn caisson_haut(largeur: f32) -> f32 {
    largeur * 0.62
}

/// Variantes de [`Composant::Caisson`] — le **porteur** technique non
/// pressurisé, seule géométrie anguleuse du jeu de composants (tout le reste est
/// de révolution). Il expose des ports hôtes sur ses faces : c'est lui qui
/// reçoit les charges utiles.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VarianteCaisson {
    /// Cadre ouvert à longerons apparents (type ExPRESS Logistics Carrier).
    Ossature,
    /// Caisson fermé, panneaux et isolant — avionique, batteries.
    Ferme,
    /// Porteur à tiroirs d'ORU enfichables, poignées apparentes.
    Rack,
    /// Berceau bas d'interface (type FRAM) : platine + entretoises, fait pour
    /// porter d'autres éléments plutôt que pour contenir.
    Berceau,
}

impl VarianteCaisson {
    pub const TOUS: [VarianteCaisson; 4] = [
        VarianteCaisson::Ossature,
        VarianteCaisson::Ferme,
        VarianteCaisson::Rack,
        VarianteCaisson::Berceau,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteCaisson::Ossature => "OSSATURE (ELC)",
            VarianteCaisson::Ferme => "CAISSON FERME",
            VarianteCaisson::Rack => "RACK A TIROIRS",
            VarianteCaisson::Berceau => "BERCEAU (FRAM)",
        }
    }

    fn cout(self) -> f32 {
        match self {
            VarianteCaisson::Ossature => 12.0,
            VarianteCaisson::Ferme => 7.0,
            VarianteCaisson::Rack => 11.0,
            VarianteCaisson::Berceau => 6.0,
        }
    }

    /// Dessine le caisson depuis `pied`, déployé le long de +Z, large selon X.
    fn dessiner<P: Peintre>(self, p: &mut P, pied: Vec3, longueur: f32, largeur: f32) {
        let metal = Color::new(0.62, 0.64, 0.68, 1.0);
        let sombre = Color::new(0.22, 0.24, 0.28, 1.0);
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        let (d, w, h) = (Vec3::Z, Vec3::X, Vec3::Y);
        let haut = caisson_haut(largeur);
        let coins = |dx: f32, dy: f32| {
            [
                w * -dx + h * -dy,
                w * dx + h * -dy,
                w * dx + h * dy,
                w * -dx + h * dy,
            ]
        };
        match self {
            VarianteCaisson::Ossature => {
                let centre = pied + d * (longueur * 0.5);
                p.cube(centre, vec3(largeur, haut, longueur), clair);
                // Longerons d'angle + cadres transversaux : l'ossature apparente.
                let c4 = coins(largeur * 0.5, haut * 0.5);
                for o in c4 {
                    p.cylindre(pied + o, pied + d * longueur + o, largeur * 0.045, metal);
                }
                for k in 0..=2 {
                    let c = pied + d * (longueur * k as f32 * 0.5);
                    for i in 0..4 {
                        p.cylindre(c + c4[i], c + c4[(i + 1) % 4], largeur * 0.035, sombre);
                    }
                }
                // Boîtiers d'équipement sur le dessus.
                p.cube(
                    centre + h * (haut * 0.56) + d * (longueur * 0.18),
                    vec3(largeur * 0.34, haut * 0.26, longueur * 0.26),
                    sombre,
                );
            }
            VarianteCaisson::Ferme => {
                // Caisson fermé : panneaux, joints de tôle et connectiques.
                let centre = pied + d * (longueur * 0.5);
                p.cube(centre, vec3(largeur, haut, longueur), clair);
                let joint = Color::new(clair.r * 0.82, clair.g * 0.82, clair.b * 0.85, 1.0);
                for k in 1..3 {
                    let z = longueur * k as f32 / 3.0;
                    p.cube(
                        pied + d * z,
                        vec3(largeur * 1.01, haut * 1.01, longueur * 0.02),
                        joint,
                    );
                }
                for s in [-1.0_f32, 1.0] {
                    p.cube(
                        centre + w * (largeur * 0.5 * s) + d * (longueur * 0.24),
                        vec3(largeur * 0.06, haut * 0.30, longueur * 0.18),
                        sombre,
                    );
                }
            }
            VarianteCaisson::Rack => {
                // Porteur à tiroirs d'ORU : bâti + tiroirs enfichables à poignée.
                let centre = pied + d * (longueur * 0.5);
                p.cube(centre, vec3(largeur * 0.94, haut, longueur), sombre);
                let n = 3;
                for k in 0..n {
                    let t = (k as f32 + 0.5) / n as f32;
                    let c = pied + d * (longueur * t) + h * (haut * 0.06);
                    p.cube(
                        c + w * (largeur * 0.06),
                        vec3(largeur * 0.92, haut * 0.72, longueur * 0.86 / n as f32),
                        clair,
                    );
                    // Poignée de manutention EVA.
                    let poi = c + w * (largeur * 0.52) + h * (haut * 0.18);
                    p.cylindre(poi - d * (longueur * 0.06), poi + d * (longueur * 0.06), largeur * 0.03, metal);
                }
            }
            VarianteCaisson::Berceau => {
                // Interface basse (type FRAM) : platine + entretoises + rebord.
                p.cube(
                    pied + d * (longueur * 0.5) + h * (haut * 0.10),
                    vec3(largeur, haut * 0.14, longueur),
                    clair,
                );
                for (sx, sz) in [(-1.0_f32, 0.12_f32), (1.0, 0.12), (-1.0, 0.88), (1.0, 0.88)] {
                    let base = pied + w * (largeur * 0.42 * sx) + d * (longueur * sz);
                    p.cylindre(base, base + h * (haut * 0.10), largeur * 0.05, metal);
                }
                for s in [-1.0_f32, 1.0] {
                    let o = w * (largeur * 0.5 * s) + h * (haut * 0.20);
                    p.cylindre(pied + o, pied + d * longueur + o, largeur * 0.045, sombre);
                }
            }
        }
    }
}

/// Variantes de [`Composant::ChargeUtile`] — ce qui se **pose sur** une
/// structure ou sur un caisson technique : plateformes d'expériences,
/// réservoirs, instruments. Un seul port de montage, comme tout appendice.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VarianteCharge {
    /// Plateau plat portant une grappe d'expériences (type JEM-EF).
    Palette,
    /// Bouteilles haute pression cerclées sur berceau (ergols, gaz).
    Reservoirs,
    /// Instrument massif à radiateurs (type AMS-02).
    Instrument,
}

impl VarianteCharge {
    pub const TOUS: [VarianteCharge; 3] = [
        VarianteCharge::Palette,
        VarianteCharge::Reservoirs,
        VarianteCharge::Instrument,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteCharge::Palette => "PALETTE EXPOSEE",
            VarianteCharge::Reservoirs => "RESERVOIRS",
            VarianteCharge::Instrument => "INSTRUMENT (AMS)",
        }
    }

    fn cout(self) -> f32 {
        match self {
            VarianteCharge::Palette => 10.0,
            VarianteCharge::Reservoirs => 14.0,
            VarianteCharge::Instrument => 9.0,
        }
    }

    /// Dessine la charge **à plat** sur la face qui la porte : elle s'étale dans
    /// le plan X (largeur) × Y (longueur), et ne dépasse selon la normale +Z que
    /// de son épaisseur. C'est ce qui la plaque contre un caisson au lieu de la
    /// percher au bout d'une tige.
    fn dessiner<P: Peintre>(self, p: &mut P, base: Vec3, longueur: f32, largeur: f32) {
        let metal = Color::new(0.62, 0.64, 0.68, 1.0);
        let sombre = Color::new(0.22, 0.24, 0.28, 1.0);
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        // n = normale sortante (épaisseur), w = largeur, l = longueur.
        let (n, w, l) = (Vec3::Z, Vec3::X, Vec3::Y);
        match self {
            VarianteCharge::Palette => {
                let ep = largeur * 0.12;
                p.cube(base + n * (ep * 0.5), vec3(largeur, longueur, ep), clair);
                // Rails sur les deux bords longs.
                for s in [-1.0_f32, 1.0] {
                    let o = w * (largeur * 0.5 * s) + n * (ep * 0.5);
                    p.cylindre(
                        base + o - l * (longueur * 0.5),
                        base + o + l * (longueur * 0.5),
                        largeur * 0.04,
                        metal,
                    );
                }
                // Grappe d'expériences de tailles inégales, posées dessus.
                for (i, &(fy, fx, fs)) in [
                    (-0.34_f32, -0.22_f32, 0.30_f32),
                    (-0.05, 0.25, 0.22),
                    (0.22, -0.10, 0.26),
                    (0.40, 0.28, 0.16),
                ]
                .iter()
                .enumerate()
                {
                    let t = largeur * fs;
                    let c = base + l * (longueur * fy) + w * (largeur * fx) + n * (ep + t * 0.5);
                    let col = if i % 2 == 0 { sombre } else { metal };
                    p.cube(c, vec3(t, longueur * fs * 0.55, t), col);
                }
            }
            VarianteCharge::Reservoirs => {
                // Bouteilles haute pression **cerclées sur un berceau**, comme
                // les réservoirs externes du sas Quest ou les ORU du treillis :
                // des cylindres à fonds bombés, couchés à plat sur la platine.
                let ep = largeur * 0.10;
                p.cube(base + n * (ep * 0.5), vec3(largeur, longueur, ep), clair);
                let r = largeur * 0.21;
                for s in [-1.0_f32, 1.0] {
                    let axe = w * (largeur * 0.26 * s) + n * (ep + r);
                    let a = base + axe - l * (longueur * 0.36);
                    let b = base + axe + l * (longueur * 0.36);
                    p.cylindre(a, b, r, metal);
                    p.sphere(a, r, metal); // fonds bombés
                    p.sphere(b, r, metal);
                    for t in [0.28_f32, 0.72] {
                        let c = a + (b - a) * t;
                        let e = l * (longueur * 0.022);
                        p.cylindre(c - e, c + e, r * 1.14, sombre); // colliers
                    }
                }
                // Bloc de vannes en bout de platine.
                p.cube(
                    base + n * (ep + r * 0.5) + l * (longueur * 0.44),
                    vec3(largeur * 0.24, longueur * 0.10, r),
                    sombre,
                );
            }
            VarianteCharge::Instrument => {
                // Silhouette d'AMS-02 : socle plaqué sur la face, corps massif
                // au-dessus, cœur cylindrique (l'aimant) émergeant, et surtout
                // **deux grands radiateurs plats** sur les flancs.
                let ep = largeur * 0.12;
                p.cube(base + n * (ep * 0.5), vec3(largeur * 1.02, longueur * 0.86, ep), sombre);
                let corps = base + n * (ep + largeur * 0.36);
                p.cube(corps, vec3(largeur * 0.86, longueur * 0.60, largeur * 0.72), clair);
                let ca = corps + n * (largeur * 0.36);
                p.cylindre(ca, ca + n * (largeur * 0.30), largeur * 0.26, metal);
                for s in [-1.0_f32, 1.0] {
                    let coin = corps + w * (largeur * 0.45 * s)
                        - l * (longueur * 0.30)
                        - n * (largeur * 0.34);
                    p.panneau(coin, l * (longueur * 0.60), n * (largeur * 0.70), clair);
                }
            }
        }
    }
}

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
#[derive(Clone, Copy, PartialEq, Debug)]
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

    fn cout(self) -> f32 {
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

    fn cout(self) -> f32 {
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
                super::pieces::radiateur(p, pied, d, w, longueur, largeur, lignes, col, sombre);
            }
            VarianteRadiateur::Corps => {
                // Large et court (hugging), franchement plus sombre.
                super::pieces::radiateur(p, pied, d, w, longueur * 0.5, largeur * 2.2, lignes, col, sombre);
            }
            VarianteRadiateur::Caloducs => {
                super::pieces::radiateur(p, pied, d, w, longueur, largeur, lignes, col, sombre);
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
                super::pieces::radiateur(p, pied + d * 0.55, d, w, longueur, largeur, lignes, col, sombre);
            }
            VarianteRadiateur::Deroulable => {
                // Gros rouleau (tambour) à la base + longue bande étroite dorée.
                p.cylindre(pied - w * (largeur * 0.5), pied + w * (largeur * 0.5), 0.2, sombre);
                super::pieces::radiateur(p, pied + d * 0.25, d, w, longueur * 1.5, largeur * 0.4, lignes, col, sombre);
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
                super::pieces::radiateur(p, pied, d, w, longueur, largeur, lignes, col, sombre);
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
    fn dessiner<P: Peintre>(self, p: &mut P, pied: Vec3, taille: f32) {
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        let sombre = Color::new(0.30, 0.32, 0.36, 1.0);
        let d = Vec3::Z;
        let w = Vec3::X;
        let up = Vec3::Y;
        match self {
            VarianteAntenne::ParaboleGG => p.parabole(pied, d, taille, clair),
            VarianteAntenne::ParaboleOffset => {
                let dir = (d + up * 0.45).normalize();
                p.parabole(pied, dir, taille * 0.9, clair);
            }
            VarianteAntenne::Cornets => {
                for (dx, dy) in [(-0.25_f32, 0.0_f32), (0.25, 0.0), (0.0, 0.28)] {
                    let base = pied + w * (dx * taille) + up * (dy * taille);
                    p.cone(base, d, taille * 0.06, taille * 0.28, taille * 0.7, clair);
                }
            }
            VarianteAntenne::Fouet => {
                let n = 4;
                for i in 0..n {
                    let a = TAU * i as f32 / n as f32;
                    let dir = (d + w * (0.4 * a.cos()) + up * (0.4 * a.sin())).normalize();
                    p.cylindre(pied, pied + dir * (taille * 1.5), 0.02, clair);
                    p.sphere(pied + dir * (taille * 1.5), 0.04, clair);
                }
            }
            VarianteAntenne::ReseauPhase => {
                let s = taille * 0.9;
                let coin = pied - w * (s * 0.5) - up * (s * 0.5);
                p.panneau(coin, w * s, up * s, sombre);
                let n = 5;
                for i in 1..n {
                    let f = i as f32 / n as f32;
                    p.ligne(coin + w * (s * f), coin + w * (s * f) + up * s, TRAIT_FIN, clair);
                    p.ligne(coin + up * (s * f), coin + up * (s * f) + w * s, TRAIT_FIN, clair);
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
                    p.cylindre(point(i as f32 / n as f32), point((i + 1) as f32 / n as f32), 0.025, clair);
                }
                p.cylindre(pied - d * 0.02, pied + d * 0.02, ray * 1.4, sombre); // réflecteur
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
    /// Grand habitat gonflable (type B330) : toile tendue de forte section,
    /// ancrée par des **cônes métalliques rigides** aux deux bouts, cerclée de
    /// **sangles de retenue**, percée de grandes fenêtres. À ne pas confondre
    /// avec `Gonflable`, qui n'est qu'un petit bombement façon BEAM.
    GrandGonflable,
    /// Serre / module agricole : longues **baies vitrées** teintées de vert par
    /// la culture, rails de racks et rampes d'éclairage. Registre futuriste.
    Serre,
    /// Cœur de station russe (Mir, Zvezda, Zarya) : corps **étagé** — tambour
    /// arrière plus large que le compartiment de travail. C'est une affaire de
    /// **dessin**, pas de profil : les ports restent au calibre du module, seule
    /// la silhouette change.
    Coeur,
}

impl VarianteModule {
    // Ordre **identique à la déclaration de l'enum** : ainsi l'index affiché
    // (numéros N dans la vue briques = ordre de cette table) == l'index du code.
    pub const TOUS: [VarianteModule; 10] = [
        VarianteModule::Standard,
        VarianteModule::Dore,
        VarianteModule::Hublots,
        VarianteModule::Labo,
        VarianteModule::Gonflable,
        VarianteModule::Coupole,
        VarianteModule::Sas,
        VarianteModule::GrandGonflable,
        VarianteModule::Serre,
        VarianteModule::Coeur,
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
            VarianteModule::Coeur => "COEUR ETAGE (RUSSE)",
            VarianteModule::GrandGonflable => "GRAND GONFLABLE (B330)",
            VarianteModule::Serre => "SERRE AGRICOLE",
        }
    }

    fn couleur(self) -> Color {
        match self {
            VarianteModule::Dore | VarianteModule::Coeur => Color::new(0.72, 0.58, 0.28, 1.0),
            VarianteModule::Labo => Color::new(0.80, 0.82, 0.85, 1.0),
            VarianteModule::Gonflable | VarianteModule::GrandGonflable => {
                Color::new(0.84, 0.81, 0.75, 1.0)
            }
            VarianteModule::Serre => Color::new(0.78, 0.82, 0.80, 1.0),
            _ => Color::new(0.85, 0.85, 0.88, 1.0),
        }
    }

    /// Facteur de débord radial de la variante, en multiples du rayon nominal.
    /// Les formes bombées (toile tendue, tambour étagé) sortent du gabarit du
    /// cylindre : le rayon englobant doit le savoir.
    fn debord_radial(self) -> f32 {
        match self {
            VarianteModule::GrandGonflable => 1.62,
            VarianteModule::Gonflable => 1.30,
            VarianteModule::Coeur => 1.16,
            _ => 1.0,
        }
    }

    /// Habillage de coque : sur le doré, bandes MLI alternées (feuilles
    /// d'isolant) ; sinon coutures de panneaux — anneaux fins en très léger
    /// débord radial (pas de face coplanaire → pas de z-fighting).
    fn habillage<P: Peintre>(self, p: &mut P, rayon: f32, demi: f32) {
        let c = self.couleur();
        match self {
            VarianteModule::Dore | VarianteModule::Coeur => {
                // Bandes d'isolant multicouche : tuiles de teintes dorées voisines.
                let n = ((demi * 2.0) / 0.55).round().max(2.0) as usize;
                for i in 0..n {
                    let z0 = -demi + (i as f32) * (2.0 * demi / n as f32);
                    let z1 = z0 + (2.0 * demi / n as f32) * 0.92; // jour entre feuilles
                    let t = match i % 3 {
                        0 => 1.06,
                        1 => 0.93,
                        _ => 1.0,
                    };
                    let teinte = Color::new(
                        (c.r * t).min(1.0),
                        (c.g * t).min(1.0),
                        (c.b * t * 0.97).min(1.0),
                        1.0,
                    );
                    p.cylindre(vec3(0.0, 0.0, z0), vec3(0.0, 0.0, z1), rayon * 1.004, teinte);
                }
            }
            // Toile tendue : pas de coutures de tôle.
            VarianteModule::Gonflable | VarianteModule::GrandGonflable => {}
            _ => {
                // Coutures de panneaux : un anneau discret tous les ~0.8 u.
                let seam = Color::new(c.r * 0.80, c.g * 0.80, c.b * 0.82, 1.0);
                let n = ((demi * 2.0) / 0.8).round().max(1.0) as usize;
                for i in 1..n {
                    let z = -demi + (i as f32) * (2.0 * demi / n as f32);
                    p.cylindre(
                        vec3(0.0, 0.0, z - 0.015),
                        vec3(0.0, 0.0, z + 0.015),
                        rayon * 1.004,
                        seam,
                    );
                }
            }
        }
    }

    /// Détails de surface, dessinés par-dessus le corps (repère local, axe Z,
    /// corps de rayon `rayon` s'étendant de −`demi` à +`demi`).
    fn details<P: Peintre>(self, p: &mut P, rayon: f32, demi: f32) {
        let sombre = Color::new(0.18, 0.20, 0.24, 1.0);
        let vitre = Color::new(0.15, 0.24, 0.34, 1.0);
        match self {
            VarianteModule::Hublots => {
                let n = ((demi * 2.0 / 0.6) as usize).max(2);
                for i in 0..n {
                    let z = -demi + (i as f32 + 0.5) * (2.0 * demi / n as f32);
                    p.sphere(vec3(0.0, rayon * 0.92, z), rayon * 0.13, sombre);
                }
                for s in [-1.0_f32, 1.0] {
                    let x = rayon * 0.72 * s;
                    p.cylindre(vec3(x, rayon * 0.72, -demi * 0.8), vec3(x, rayon * 0.72, demi * 0.8), 0.03, sombre);
                }
            }
            VarianteModule::Labo => {
                // grande fenêtre plate sur +Y + rack externe sur −Y.
                p.cube(vec3(0.0, rayon, 0.0), vec3(rayon * 0.9, 0.06, demi * 1.1), vitre);
                p.cube(vec3(0.0, -rayon * 1.05, 0.0), vec3(rayon * 0.7, rayon * 0.3, demi * 0.8), sombre);
            }
            VarianteModule::Gonflable => {
                // bombement central (fabric BEAM) : une sphère aplatie au milieu.
                p.sphere(Vec3::ZERO, rayon * 1.3, self.couleur());
            }
            VarianteModule::Coupole => {
                p.cone(vec3(0.0, 0.0, demi), Vec3::Z, rayon * 0.6, rayon * 0.32, rayon * 0.5, vitre);
            }
            VarianteModule::Sas => {
                // Écoutille EVA saillante sur +X + main courante.
                p.cube(vec3(rayon * 1.05, 0.0, 0.0), vec3(0.4, rayon * 0.7, rayon * 0.7), sombre);
                p.cylindre(
                    vec3(rayon * 0.9, rayon * 0.6, -demi * 0.6),
                    vec3(rayon * 0.9, rayon * 0.6, demi * 0.6),
                    0.03,
                    sombre,
                );
            }
            VarianteModule::GrandGonflable => {
                // Ce qui distingue le B330 du BEAM : la toile est ancrée par des
                // **cônes métalliques rigides** aux deux bouts, et cerclée de
                // **sangles de retenue** axiales et circonférentielles.
                let c = self.couleur();
                let rt = rayon * 1.62; // section gonflée
                let dz = demi * 0.62; // portion tendue, hors cônes d'ancrage
                p.cylindre(vec3(0.0, 0.0, -dz), vec3(0.0, 0.0, dz), rt, c);
                p.sphere(vec3(0.0, 0.0, -dz), rt, c);
                p.sphere(vec3(0.0, 0.0, dz), rt, c);
                let metal = Color::new(0.60, 0.62, 0.66, 1.0);
                for s in [-1.0_f32, 1.0] {
                    p.cone(
                        vec3(0.0, 0.0, s * demi),
                        vec3(0.0, 0.0, -s),
                        rayon * 1.02,
                        rt * 0.78,
                        demi - dz,
                        metal,
                    );
                }
                // Sangles axiales, puis cerclages.
                let sangle = Color::new(0.42, 0.40, 0.36, 1.0);
                for k in 0..8 {
                    let a = TAU * k as f32 / 8.0;
                    let dir = vec3(a.cos(), a.sin(), 0.0);
                    p.cylindre(
                        dir * (rt * 0.99) + vec3(0.0, 0.0, -dz),
                        dir * (rt * 0.99) + vec3(0.0, 0.0, dz),
                        rayon * 0.035,
                        sangle,
                    );
                }
                for z in [-dz * 0.55, 0.0, dz * 0.55] {
                    p.cylindre(
                        vec3(0.0, 0.0, z - rayon * 0.03),
                        vec3(0.0, 0.0, z + rayon * 0.03),
                        rt * 1.01,
                        sangle,
                    );
                }
                // Quatre grandes fenêtres.
                for k in 0..4 {
                    let a = TAU * k as f32 / 4.0 + 0.4;
                    let dir = vec3(a.cos(), a.sin(), 0.0);
                    p.cylindre(
                        dir * (rt * 0.98),
                        dir * (rt * 1.04),
                        rayon * 0.22,
                        vitre,
                    );
                }
            }
            VarianteModule::Serre => {
                // Longues baies vitrées teintées par la culture, entre des rails
                // de racks, plus des rampes d'éclairage.
                let verre = Color::new(0.36, 0.62, 0.42, 1.0);
                let rail = Color::new(0.58, 0.60, 0.64, 1.0);
                for k in 0..4 {
                    let a = TAU * k as f32 / 4.0;
                    let dir = vec3(a.cos(), a.sin(), 0.0);
                    // Baie vitrée : panneau bombé le long du corps.
                    p.cylindre(
                        dir * (rayon * 0.62) + vec3(0.0, 0.0, -demi * 0.78),
                        dir * (rayon * 0.62) + vec3(0.0, 0.0, demi * 0.78),
                        rayon * 0.52,
                        verre,
                    );
                    // Rail de rack entre deux baies.
                    let b = TAU * (k as f32 + 0.5) / 4.0;
                    let db = vec3(b.cos(), b.sin(), 0.0);
                    p.cylindre(
                        db * (rayon * 1.02) + vec3(0.0, 0.0, -demi * 0.9),
                        db * (rayon * 1.02) + vec3(0.0, 0.0, demi * 0.9),
                        rayon * 0.07,
                        rail,
                    );
                }
                // Rampes d'éclairage sur deux génératrices.
                for s in [-1.0_f32, 1.0] {
                    let o = vec3(0.0, rayon * 1.10 * s, 0.0);
                    p.cylindre(
                        o + vec3(0.0, 0.0, -demi * 0.7),
                        o + vec3(0.0, 0.0, demi * 0.7),
                        rayon * 0.05,
                        Color::new(0.95, 0.93, 0.72, 1.0),
                    );
                }
            }
            VarianteModule::Coeur => {
                // Tambour arrière plus large, puis transition conique vers le
                // compartiment de travail. Le col de docking, plus étroit,
                // continue de dépasser en bout — comme sur Mir ou Zvezda.
                let c = self.couleur();
                let rt = rayon * 1.16;
                let h = demi * 0.75;
                p.cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, -demi + h), rt, c);
                p.cone(vec3(0.0, 0.0, -demi + h), Vec3::Z, rt, rayon, rayon * 0.30, c);
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
            Composant::Caisson { profil, longueur, largeur, .. } => {
                // Le caisson est un **porteur** : port de montage (index 0) vers
                // sa structure, plus des ports hôtes sur ses cinq faces libres,
                // qui reçoivent charges utiles, radiateurs ou antennes.
                let haut = caisson_haut(*largeur);
                let cz = CAISSON_PLATINE + longueur * 0.5;
                let mut v = vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                    GenrePort::Surface,
                    *profil,
                )];
                for (dir, rot) in faces_principales() {
                    // La face −Z porte le montage : elle n'accueille personne.
                    if dir.z < -0.5 {
                        continue;
                    }
                    let demi = vec3(largeur * 0.5, haut * 0.5, longueur * 0.5);
                    let pos = vec3(0.0, 0.0, cz) + dir * demi;
                    v.push(Port::new(Repere::new(pos, rot), GenrePort::Surface, Profil::P0));
                }
                v
            }
            Composant::ChargeUtile { profil, .. } => {
                // Montage générique `Surface`, comme les autres appendices.
                vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                    GenrePort::Surface,
                    *profil,
                )]
            }
            Composant::Charpente { grand, petit, longueur, .. } => {
                // Deux bouts axiaux : `petit` en +Z (l'apex étroit), `grand` en
                // −Z (la base évasée), à leurs profils respectifs.
                let demi = longueur * 0.5;
                vec![
                    Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, *petit),
                    Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, *grand),
                ]
            }
            Composant::RadiateurMega { profil, .. } => {
                // Un port `Surface`, comme les autres appendices : avant vers
                // l'hôte (−Z), l'aile se déploie de l'autre côté (+Z).
                vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                    GenrePort::Surface,
                    *profil,
                )]
            }
            Composant::Motrice { profil, echelle } => {
                // Écoutille au nœud d'attache (avant, +Z vers le câble du vaisseau).
                vec![Port::new(
                    Repere::new(vec3(0.0, 0.0, 0.5 * echelle), Quat::IDENTITY),
                    GenrePort::ModuleAxial,
                    *profil,
                )]
            }
            Composant::BlocMoteur { profil, largeur } => {
                let pz = largeur * 0.5; // profondeur Z du conteneur
                // Bout −Z de la rangée d'habitats (branchée au ras de la caisse) :
                // caisse(−pz/2) − habitat(3.0). Plus de connecteur intermédiaire.
                let tip_rad = -pz * 0.5 - 3.0;
                vec![
                    // −Z : montage vers le radiateur (bout du connecteur B).
                    Port::new(Repere::new(vec3(0.0, 0.0, tip_rad), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, *profil),
                    // +Z : chaînage de la suite (face avant de la caisse).
                    Port::new(Repere::new(vec3(0.0, 0.0, pz * 0.5), Quat::IDENTITY), GenrePort::ModuleAxial, *profil),
                ]
            }
            Composant::Reservoir { profil, longueur, .. } => {
                let demi = longueur * 0.5;
                vec![
                    Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, *profil),
                    Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, *profil),
                ]
            }
            Composant::MoteurAntimatiere { profil, .. } => {
                // Unique écoutille axiale de montage en tête (avant +Z, vers le
                // corps porteur) ; le moteur pousse dans le sens opposé (−Z).
                vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::IDENTITY),
                    GenrePort::ModuleAxial,
                    *profil,
                )]
            }
            Composant::Coiffe { profil, .. } => {
                // Base en Z=0, nez vers +Z : l'écoutille de montage regarde le
                // module (avant −Z) et se clipse sur son écoutille axiale.
                vec![Port::new(
                    Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                    GenrePort::ModuleAxial,
                    *profil,
                )]
            }
            Composant::ReacteurAntimatiere { profil, taille } => {
                // Montage vers la tuyère en −Z (base) ; chaînage d'alimentation en
                // tête +Z (bout du corps).
                let lb = taille * 0.95;
                vec![
                    Port::new(Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, *profil),
                    Port::new(Repere::new(vec3(0.0, 0.0, lb), Quat::IDENTITY), GenrePort::ModuleAxial, *profil),
                ]
            }
            // Anneau décoratif posé à la main (via un `Repere` cuit) : pas de port.
            Composant::TreillisHexagone { .. } => vec![],
            Composant::Propulseur { profil, variante, .. } => {
                if variante.axial() {
                    // Moteur principal : écoutille axiale, tuyère vers l'arrière.
                    vec![Port::new(
                        Repere::new(Vec3::ZERO, Quat::IDENTITY),
                        GenrePort::ModuleAxial,
                        *profil,
                    )]
                } else {
                    // Grappe de contrôle : posée sur un flanc comme un appendice.
                    vec![Port::new(
                        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
                        GenrePort::Surface,
                        *profil,
                    )]
                }
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
    pub fn dessiner<P: Peintre>(&self, p: &mut P) {
        match self {
            Composant::ModuleAxial { profil, longueur, variante } => {
                let rayon = profil.rayon();
                let demi = *longueur * 0.5;
                let lc = rayon * COL_LONG;
                let rc = rayon * COL_RAYON;
                // Corps : cylindre lisse, teinté par la variante.
                let corps = variante.couleur();
                p.cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, demi), rayon, corps);
                // Habillage de coque : coutures de panneaux (ou bandes MLI pour
                // le doré) — anneaux fins en léger débord radial, teinte voisine.
                variante.habillage(p, rayon, demi);
                // Embouts : un petit cylindre coiffe chaque disque de bout. Il
                // chevauche le corps (part de `demi - EMBOUT_ENFONCE`) → aucune
                // face coplanaire, donc plus de z-fighting ; léger débord = arête.
                let re = rayon * EMBOUT_RAYON;
                p.cylindre(vec3(0.0, 0.0, demi - EMBOUT_ENFONCE), vec3(0.0, 0.0, demi + EMBOUT_LONG), re, SOMBRE);
                p.cylindre(vec3(0.0, 0.0, -demi + EMBOUT_ENFONCE), vec3(0.0, 0.0, -demi - EMBOUT_LONG), re, SOMBRE);
                // Collerettes de docking : cols étroits qui dépassent à chaque bout,
                // terminés par une bague d'accostage alu clair (visuel APAS).
                p.cylindre(vec3(0.0, 0.0, demi), vec3(0.0, 0.0, demi + lc), rc, SOMBRE);
                p.cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, -demi - lc), rc, SOMBRE);
                let lb = lc * 0.28; // épaisseur de la bague
                p.cylindre(vec3(0.0, 0.0, demi + lc - lb), vec3(0.0, 0.0, demi + lc), rc * 1.10, BAGUE);
                p.cylindre(vec3(0.0, 0.0, -demi - lc + lb), vec3(0.0, 0.0, -demi - lc), rc * 1.10, BAGUE);
                // Détails de surface (hublots, fenêtre, coupole, bombement…).
                variante.details(p, rayon, demi);
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
                p.sphere(Vec3::ZERO, rs, COULEUR);
                for (dir, _, _) in faces_noeud(*sorties) {
                    // Bras cylindrique ancré dans la sphère, collerette, puis
                    // bague d'accostage alu clair au bout (comme les modules).
                    p.cylindre(dir * base, dir * (rs + lb), rb, COULEUR);
                    p.cylindre(dir * (rs + lb), dir * (rs + lb + lc), rc, SOMBRE);
                    let lbg = lc * 0.28;
                    p.cylindre(dir * (rs + lb + lc - lbg), dir * (rs + lb + lc), rc * 1.10, BAGUE);
                }
            }
            Composant::PanneauSolaire { variante, longueur, largeur, .. } => {
                // Jonction côté hôte (−Z) : un bras qui rejoint l'hôte + un petit
                // socle (gimbal) à l'origine — c'est la liaison module ↔ panneau.
                p.cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
                p.cube(Vec3::ZERO, Vec3::splat(0.22), COULEUR);
                p.cube_fil(Vec3::ZERO, Vec3::splat(0.22), SOMBRE);
                // Mât depuis le socle (origine) vers +Z, puis la pale selon la variante.
                let pied = vec3(0.0, 0.0, MAST_PANNEAU);
                p.cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
                let (col, lf, wf) = variante.style();
                let (lon, lar) = (longueur * lf, largeur * wf);
                match variante {
                    VariantePanneau::Hexagonal => {
                        super::pieces::pale_hexagonale(p, pied, Vec3::Z, Vec3::X, lon, lar, col);
                    }
                    _ => {
                        let cellules = (lon / 0.35).max(2.0) as usize;
                        super::pieces::pale_solaire(p, pied, Vec3::Z, Vec3::X, lon, lar, cellules, col);
                    }
                }
            }
            Composant::Treillis { profil, longueur, style } => {
                let demi = longueur * 0.5;
                let sec = profil.rayon() * TREILLIS_SECTION;
                let (a, b) = (vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, demi));
                match style {
                    StyleTreillis::Carre => super::pieces::treillis(p, a, b, sec, COULEUR, SOMBRE),
                    StyleTreillis::Triangulaire => {
                        super::pieces::treillis_triangulaire(p, a, b, sec, COULEUR, SOMBRE)
                    }
                }
            }
            Composant::Radiateur { variante, longueur, largeur, .. } => {
                // Jonction hôte (bras + socle) puis mât, comme le panneau.
                p.cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
                p.cube(Vec3::ZERO, Vec3::splat(0.2), COULEUR);
                p.cube_fil(Vec3::ZERO, Vec3::splat(0.2), SOMBRE);
                let pied = vec3(0.0, 0.0, MAST_PANNEAU);
                p.cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
                variante.dessiner(p, pied, *longueur, *largeur);
            }
            Composant::Caisson { variante, longueur, largeur, .. } => {
                // **Moyen d'attache** explicite : platine boulonnée sur l'hôte +
                // deux jambes de force jusqu'au caisson. C'est ce qui le sangle
                // sur une barre de structure ou sur le flanc d'un module.
                p.cube(vec3(0.0, 0.0, -0.03), vec3(0.34, 0.34, 0.10), COULEUR);
                p.cube_fil(vec3(0.0, 0.0, -0.03), vec3(0.34, 0.34, 0.10), SOMBRE);
                for s in [-1.0_f32, 1.0] {
                    let a = vec3(0.11 * s, 0.0, 0.0);
                    let b = vec3(largeur * 0.30 * s, 0.0, CAISSON_PLATINE);
                    p.cylindre(a, b, 0.035, SOMBRE);
                }
                let pied = vec3(0.0, 0.0, CAISSON_PLATINE);
                variante.dessiner(p, pied, *longueur, *largeur);
            }
            Composant::ChargeUtile { variante, longueur, largeur, .. } => {
                // **Aucune entretoise** : la charge est boulonnée à même la face
                // qui la porte, sa platine faisant office d'interface.
                variante.dessiner(p, Vec3::ZERO, *longueur, *largeur);
            }
            Composant::Propulseur { variante, taille, .. } => {
                // Un moteur axial s'enfonce vers l'arrière (−Z), une grappe sort
                // du flanc (+Z) : même géométrie, sens de sortie opposé.
                let sens = if variante.axial() { -1.0 } else { 1.0 };
                variante.dessiner(p, Vec3::ZERO, sens, *taille);
            }
            Composant::Charpente { grand, petit, longueur, courbure, aiguille } => {
                let demi = *longueur * 0.5;
                let sg = grand.rayon() * TREILLIS_SECTION;
                let sp = petit.rayon() * TREILLIS_SECTION;
                super::pieces::treillis_conique(
                    p,
                    vec3(0.0, 0.0, -demi),
                    vec3(0.0, 0.0, demi),
                    sg,
                    sp,
                    *courbure,
                    COULEUR,
                    SOMBRE,
                );
                if *aiguille {
                    // **Anneau hexagonal en treillis** sous la base évasée. Le
                    // **côté** de l'hexagone = **largeur de bout du cône** (2·sg),
                    // si bien qu'une extrémité du cône fait exactement la taille
                    // d'une face extérieure de l'hexagone.
                    let cote = 2.0 * sg; // côté hexa = largeur de bout du cône
                    let sec = sg * 0.5; // demi‑épaisseur radiale (dans le plan)
                    let prof = sg; // demi‑profondeur hors‑plan → volume épaissi
                    let ap = cote * 3.0_f32.sqrt() * 0.5; // apothème de l'hexagone
                    // On descend d'`ap + sec` : la **face extérieure** du montant
                    // haut (et non son axe) affleure la base → le bout du cône
                    // repose dessus au lieu de la traverser.
                    let centre = vec3(0.0, 0.0, -demi - ap - sec);
                    super::pieces::treillis_hexagone(p, centre, cote, sec, prof, COULEUR, SOMBRE);

                    // **Base évasée vers les sommets 3 (droite) et 6 (gauche)** de
                    // l'hexagone (les deux sommets latéraux les plus larges, à
                    // `±cote` en X), au lieu de se poser sur l'arête du haut (1‑2).
                    // La base du cône s'ouvre en jupe : ses 2 coins droits filent
                    // vers le sommet 3, ses 2 coins gauches vers le sommet 6.
                    let z_hex = centre.z; // niveau des sommets 3 et 6
                    let som3 = vec3(cote, 0.0, z_hex); // sommet droit (r = cote)
                    let som6 = vec3(-cote, 0.0, z_hex); // sommet gauche
                    let cd = [vec3(sg, sg, -demi), vec3(sg, -sg, -demi)]; // coins base droits
                    let cg = [vec3(-sg, sg, -demi), vec3(-sg, -sg, -demi)]; // coins base gauches
                    for c in cd {
                        p.cylindre(c, som3, sg * 0.16, COULEUR); // longeron de jupe
                    }
                    for c in cg {
                        p.cylindre(c, som6, sg * 0.16, COULEUR);
                    }
                    // Croisillons : ferment la bouche des deux côtés + fond 3↔6.
                    p.cylindre(cd[0], cd[1], sg * 0.12, SOMBRE);
                    p.cylindre(cg[0], cg[1], sg * 0.12, SOMBRE);
                    p.cylindre(som3, som6, sg * 0.14, SOMBRE);
                }
            }
            Composant::RadiateurMega { longueur, largeur, ailettes, .. } => {
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
                let m = (*ailettes).max(4);
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
            Composant::Motrice { echelle, .. } => {
                let s = *echelle;
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
            Composant::BlocMoteur { largeur, .. } => {
                let w = *largeur;
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
            Composant::Reservoir { longueur, cage, .. } => {
                let rs = *longueur * 0.5; // rayon de référence de la cage tétraédrique
                let r_cuve = rs * 1.3; // cuve **gonflée de 30 %**
                let corps = Color::new(0.82, 0.84, 0.88, 1.0); // alu clair (tuiles)
                let joint = Color::new(0.30, 0.31, 0.34, 1.0); // sous-couche (joints)
                let metal = Color::new(0.55, 0.57, 0.62, 1.0);
                // Cuve **sphérique** à **tuiles triangulaires** : sous-couche sombre
                // (les joints ressortent) + icosphère facettée par-dessus.
                p.sphere(Vec3::ZERO, r_cuve * 0.97, joint);
                super::pieces::sphere_triangulee(p, Vec3::ZERO, r_cuve, 3, 0.16, corps);
                if *cage {
                    // 4 barres métal en **position tétraédrique**, depuis la surface.
                    // Tétraèdre **pointe en +Z, face opposée perpendiculaire à Z**
                    // (les 3 sommets de base à z = −1/3) : cette face est donc plane
                    // et parallèle au plan X‑Y — orientable parallèle à l'hexagone.
                    let lb = rs * 1.25; // dépassement des barres hors de la cuve
                    let s2 = 2.0_f32.sqrt(); // √2
                    let s6 = 6.0_f32.sqrt(); // √6
                    let dirs = [
                        vec3(0.0, 0.0, 1.0),                       // pointe (+Z)
                        vec3(2.0 * s2 / 3.0, 0.0, -1.0 / 3.0),     // base
                        vec3(-s2 / 3.0, s6 / 3.0, -1.0 / 3.0),     // base
                        vec3(-s2 / 3.0, -s6 / 3.0, -1.0 / 3.0),    // base
                    ];
                    let mut sommets = [Vec3::ZERO; 4];
                    for (i, d) in dirs.iter().enumerate() {
                        let dir = d.normalize();
                        let bout = dir * (rs + lb);
                        p.cylindre(dir * r_cuve, bout, rs * 0.10, metal); // barre radiale
                        p.sphere(bout, rs * 0.13, metal); // embout = sommet
                        sommets[i] = bout;
                    }
                    // Relier les 4 sommets deux à deux (6 arêtes) → tétraèdre.
                    for i in 0..4 {
                        for j in (i + 1)..4 {
                            p.cylindre(sommets[i], sommets[j], rs * 0.08, metal);
                        }
                    }
                }
            }
            Composant::MoteurAntimatiere { taille, .. } => {
                // Silhouette VASIMR **gonflée** : corps de confinement magnétique
                // le long de −Z, anneaux de bobines cuivre, cœur d'annihilation
                // émissif, tuyère magnétique évasée et long jet de plasma.
                let metal = Color::new(0.62, 0.64, 0.68, 1.0);
                let sombre = Color::new(0.20, 0.22, 0.26, 1.0);
                let clair = Color::new(0.80, 0.82, 0.86, 1.0);
                let cuivre = Color::new(0.66, 0.45, 0.28, 1.0);
                let coeur = Color::new(0.90, 0.55, 1.0, 1.0); // annihilation e⁺/e⁻ (violet)
                let (d, w, h) = (Vec3::NEG_Z, Vec3::X, Vec3::Y);
                let t = *taille;
                // Collier structurel clair en tête (côté montage).
                p.cylindre(Vec3::ZERO - d * (t * 0.04), d * (t * 0.10), t * 0.42, clair);
                // Corps de confinement magnétique (fût central sombre).
                p.cylindre(Vec3::ZERO, d * (t * 1.10), t * 0.22, sombre);
                // Cœur d'annihilation émissif au centre du fût.
                p.cylindre(d * (t * 0.28), d * (t * 0.72), t * 0.11, coeur);
                // Six anneaux de bobines de confinement (plus qu'un VASIMR).
                for k in 0..6 {
                    let z = t * (0.14 + 0.16 * k as f32);
                    p.cylindre(d * (z - t * 0.05), d * (z + t * 0.05), t * 0.36, cuivre);
                }
                // **Buse de sortie** : court cylindre (col) au bout du corps.
                p.cylindre(d * (t * 1.02), d * (t * 1.24), t * 0.20, metal);
                // **Structure de stabilisation finale** : deux cercles ouverts
                // (anneaux polygonaux, non pleins) tenus par **4 tiges** — le
                // dernier étage où le plasma est encore contraint avant de partir.
                let rr = t * 0.30; // rayon des anneaux
                let (z1, z2) = (t * 1.34, t * 1.60); // positions axiales des cercles
                let seg = 20usize; // finesse du cercle
                let fil = t * 0.028; // section du fil d'anneau
                for &z in &[z1, z2] {
                    let c = d * z;
                    for i in 0..seg {
                        let a0 = TAU * i as f32 / seg as f32;
                        let a1 = TAU * (i + 1) as f32 / seg as f32;
                        let p0 = c + (w * a0.cos() + h * a0.sin()) * rr;
                        let p1 = c + (w * a1.cos() + h * a1.sin()) * rr;
                        p.cylindre(p0, p1, fil, metal);
                    }
                }
                // 4 tiges longitudinales : elles courent depuis la **buse du
                // propulseur** (ancrage) jusqu'au second cercle, tenant les deux
                // anneaux au passage.
                let z0 = t * 1.06; // ancrage sur la buse
                for k in 0..4 {
                    let a = TAU * k as f32 / 4.0;
                    let dir = w * a.cos() + h * a.sin();
                    p.cylindre(d * z0 + dir * rr, d * z2 + dir * rr, fil * 1.4, metal);
                    // Patte de fixation radiale sur la buse.
                    p.cylindre(d * z0 + dir * rr, d * z0 + dir * (t * 0.20), fil * 1.4, metal);
                }
            }
            Composant::Coiffe { profil, variante } => {
                let r = profil.rayon();
                let metal = Color::new(0.62, 0.64, 0.68, 1.0);
                let clair = Color::new(0.80, 0.82, 0.86, 1.0);
                let sombre = Color::new(0.20, 0.22, 0.26, 1.0);
                // Émet un triangle **des deux côtés** (les triangles bruts sont
                // mono-face et macroquad ne double-face pas).
                fn dbl(idx: &mut Vec<u16>, a: u16, b: u16, c: u16) {
                    idx.extend_from_slice(&[a, b, c, a, c, b]);
                }
                // Collier de base commun : couvre l'embout du module (1,02·r) pour
                // qu'aucun rebord ne dépasse au raccord.
                p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, r * 0.06), r * 1.03, metal);
                match variante {
                    VarianteCoiffe::Bombee => {
                        // **Demi-dôme** = calotte lisse fermée, base en Z=0 (rien ne
                        // dépasse sous le module). Grille latitude × longitude d'un
                        // demi-ellipsoïde (hauteur surbaissée), triangulée double-face.
                        let seg = 18usize; // segments autour
                        let rings = 6usize; // anneaux de l'équateur au sommet
                        let h = r * 0.5; // hauteur du bombé (surbaissé)
                        let mut verts: Vec<Vec3> = Vec::new();
                        for i in 0..rings {
                            let u = (i as f32 / rings as f32) * (PI * 0.5); // 0..≈π/2
                            let (su, cu) = u.sin_cos();
                            for j in 0..seg {
                                let a = TAU * j as f32 / seg as f32;
                                verts.push(vec3(r * cu * a.cos(), r * cu * a.sin(), h * su));
                            }
                        }
                        let apex = verts.len() as u16;
                        verts.push(vec3(0.0, 0.0, h));
                        let mut idx: Vec<u16> = Vec::new();
                        for i in 0..rings - 1 {
                            for j in 0..seg {
                                let j2 = (j + 1) % seg;
                                let a = (i * seg + j) as u16;
                                let b = (i * seg + j2) as u16;
                                let c = ((i + 1) * seg + j) as u16;
                                let d = ((i + 1) * seg + j2) as u16;
                                dbl(&mut idx, a, b, d);
                                dbl(&mut idx, a, d, c);
                            }
                        }
                        let last = (rings - 1) * seg;
                        for j in 0..seg {
                            let j2 = (j + 1) % seg;
                            dbl(&mut idx, (last + j) as u16, (last + j2) as u16, apex);
                        }
                        p.triangles(&verts, &idx, clair);
                    }
                    VarianteCoiffe::Hexagonale => {
                        // Fût hexagonal légèrement tronconique **fermé par une face
                        // hexagonale plate** (pas de pointe).
                        let n = 6usize;
                        let hp = r * 0.70; // hauteur
                        let rt = r * 0.74; // rayon du dessus (léger fruit)
                        let ring = |rad: f32, z: f32| -> Vec<Vec3> {
                            (0..n)
                                .map(|k| {
                                    let a = TAU * k as f32 / n as f32;
                                    vec3(rad * a.cos(), rad * a.sin(), z)
                                })
                                .collect()
                        };
                        let bas = ring(r, 0.0);
                        let haut = ring(rt, hp);
                        let mut verts: Vec<Vec3> = bas;
                        verts.extend(haut.iter().copied()); // indices n..2n
                        // Parois (trapèzes → 2 triangles), double-face.
                        let mut idx: Vec<u16> = Vec::new();
                        for k in 0..n {
                            let k2 = (k + 1) % n;
                            let (b0, b1, t0, t1) = (k as u16, k2 as u16, (n + k) as u16, (n + k2) as u16);
                            dbl(&mut idx, b0, b1, t1);
                            dbl(&mut idx, b0, t1, t0);
                        }
                        // Face plate hexagonale du dessus (éventail depuis le centre).
                        let ctr = verts.len() as u16;
                        verts.push(vec3(0.0, 0.0, hp));
                        for k in 0..n {
                            let k2 = (k + 1) % n;
                            dbl(&mut idx, (n + k) as u16, (n + k2) as u16, ctr);
                        }
                        p.triangles(&verts, &idx, metal);
                    }
                    VarianteCoiffe::Amarrage => {
                        // Jupe tronconique + anneau d'amarrage (APAS/IDA) + trappe
                        // centrale fermée + petites gâches de verrouillage.
                        let zc = r * 0.55; // haut de la jupe
                        p.cone(Vec3::ZERO, Vec3::Z, r, r * 0.55, zc, metal); // jupe
                        // Col d'accostage + bague claire (visuel androgyne APAS).
                        p.cylindre(vec3(0.0, 0.0, zc), vec3(0.0, 0.0, zc + r * 0.14), r * 0.55, sombre);
                        p.cylindre(vec3(0.0, 0.0, zc + r * 0.10), vec3(0.0, 0.0, zc + r * 0.14), r * 0.60, clair);
                        // Trappe centrale fermée (la coiffe **obture** le module).
                        p.cylindre(vec3(0.0, 0.0, zc + r * 0.14), vec3(0.0, 0.0, zc + r * 0.18), r * 0.5, metal);
                        // 4 gâches de verrouillage réparties sur le col.
                        for k in 0..4 {
                            let a = TAU * k as f32 / 4.0;
                            let c = vec3(r * 0.55 * a.cos(), r * 0.55 * a.sin(), zc + r * 0.07);
                            p.cube(c, vec3(r * 0.10, r * 0.10, r * 0.14), clair);
                        }
                    }
                }
            }
            Composant::ReacteurAntimatiere { taille, .. } => {
                // Bloc d'injection/confinement en amont de la tuyère : cuve sombre,
                // bobines électromagnétiques (cryostat), tuyauterie, injecteur et
                // pièges à antiprotons. Base en Z=0 (côté tuyère), corps vers +Z.
                let t = *taille;
                let rb = t * 0.40; // rayon de la cuve
                let lb = t * 0.95; // longueur de la cuve
                let dark = Color::new(0.14, 0.15, 0.18, 1.0);
                let cuivre = Color::new(0.66, 0.45, 0.28, 1.0);
                let metal = Color::new(0.55, 0.57, 0.62, 1.0);
                let clair = Color::new(0.80, 0.82, 0.86, 1.0);
                let lueur = Color::new(0.42, 0.66, 0.95, 1.0);
                // Cuve réacteur sombre.
                p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, lb), rb, dark);
                // Collier de jonction au bloc moteur (base −Z) + liseré de
                // confinement émissif (rappel du plasma de la tuyère).
                p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, t * 0.05), rb * 1.05, metal);
                p.cylindre(vec3(0.0, 0.0, t * 0.01), vec3(0.0, 0.0, t * 0.03), rb * 1.01, lueur);
                // 4 bobines électromagnétiques : anneau de cuivre serré entre deux
                // flasques métal (aspect cryostat de bobine supraconductrice).
                for k in 0..4 {
                    let z = t * (0.20 + 0.19 * k as f32);
                    p.cylindre(vec3(0.0, 0.0, z - t * 0.05), vec3(0.0, 0.0, z + t * 0.05), rb * 1.14, cuivre);
                    for s in [-1.0_f32, 1.0] {
                        let zf = z + s * t * 0.058;
                        p.cylindre(vec3(0.0, 0.0, zf - t * 0.012), vec3(0.0, 0.0, zf + t * 0.012), rb * 1.18, metal);
                    }
                }
                // Tuyauterie : 3 conduites longitudinales (hors des bobines) avec
                // vanne médiane et coudes de raccord vers la cuve.
                for s in 0..3 {
                    let a = TAU * s as f32 / 3.0 + 0.5;
                    let dir = vec3(a.cos(), a.sin(), 0.0);
                    let (z0, z1) = (t * 0.10, lb - t * 0.10);
                    let p0 = dir * (rb * 1.22) + Vec3::Z * z0;
                    let p1 = dir * (rb * 1.22) + Vec3::Z * z1;
                    p.cylindre(p0, p1, t * 0.03, metal);
                    p.cube(dir * (rb * 1.22) + Vec3::Z * (t * 0.55), Vec3::splat(t * 0.07), clair); // vanne
                    p.cylindre(p0, dir * rb + Vec3::Z * z0, t * 0.025, metal); // coude bas
                    p.cylindre(p1, dir * rb + Vec3::Z * z1, t * 0.025, metal); // coude haut
                }
                // Tête : dôme d'obturation + injecteur central, flanqué de deux
                // pièges à antiprotons (petites cuves sphériques) alimentés.
                p.cone(vec3(0.0, 0.0, lb), Vec3::Z, rb, rb * 0.45, t * 0.14, dark);
                p.sphere(vec3(0.0, 0.0, lb + t * 0.14), t * 0.10, metal);
                for s in [-1.0_f32, 1.0] {
                    let c = vec3(s * rb * 0.70, 0.0, lb + t * 0.02);
                    p.sphere(c, t * 0.13, clair);
                    p.cylindre(c, vec3(s * rb * 0.35, 0.0, lb - t * 0.05), t * 0.02, metal);
                }
            }
            Composant::TreillisHexagone { profil, liaison } => {
                // Même anneau que le pied de la charpente (mêmes proportions).
                let sg = profil.rayon() * TREILLIS_SECTION;
                let cote = 2.0 * sg;
                let sec = sg * 0.5;
                let prof = sg;
                super::pieces::treillis_hexagone(p, Vec3::ZERO, cote, sec, prof, COULEUR, SOMBRE);
                if *liaison > 0.0 {
                    // 6 montants depuis chaque sommet le long de +Z local, jusqu'à
                    // l'hexagone jumeau situé `liaison` plus loin → prisme reliant.
                    let r = cote;
                    let ap = cote * 3.0_f32.sqrt() * 0.5;
                    let demi = cote * 0.5;
                    let sommets = [
                        vec3(-demi, 0.0, ap),
                        vec3(demi, 0.0, ap),
                        vec3(r, 0.0, 0.0),
                        vec3(demi, 0.0, -ap),
                        vec3(-demi, 0.0, -ap),
                        vec3(-r, 0.0, 0.0),
                    ];
                    for s in sommets {
                        p.cylindre(s, s + Vec3::Z * *liaison, sec * 0.30, COULEUR);
                    }
                }
            }
            Composant::Antenne { variante, taille, .. } => {
                // Jonction hôte (bras + socle) puis mât court, puis l'antenne.
                p.cylindre(vec3(0.0, 0.0, -BASE_ARM_PANNEAU), Vec3::ZERO, 0.08, SOMBRE);
                p.cube(Vec3::ZERO, Vec3::splat(0.2), COULEUR);
                p.cube_fil(Vec3::ZERO, Vec3::splat(0.2), SOMBRE);
                let pied = vec3(0.0, 0.0, MAST_PANNEAU);
                p.cylindre(Vec3::ZERO, pied, 0.05, SOMBRE);
                variante.dessiner(p, pied, *taille);
            }
            Composant::Adaptateur { grand, petit, longueur } => {
                let demi = *longueur * 0.5;
                // Tronc de cône grand (−Z) → petit (+Z) + collerettes de docking
                // terminées par une bague d'accostage alu clair.
                p.cone(vec3(0.0, 0.0, -demi), Vec3::Z, grand.rayon(), petit.rayon(), *longueur, COULEUR);
                for (s, prof) in [(-1.0_f32, grand), (1.0_f32, petit)] {
                    let (lc, rc) = (prof.rayon() * COL_LONG, prof.rayon() * COL_RAYON);
                    let bout = s * (demi + lc);
                    p.cylindre(vec3(0.0, 0.0, s * demi), vec3(0.0, 0.0, bout), rc, SOMBRE);
                    p.cylindre(vec3(0.0, 0.0, bout - s * lc * 0.28), vec3(0.0, 0.0, bout), rc * 1.10, BAGUE);
                }
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
            // caisson : boîte + ossature, coût selon le type.
            Composant::Caisson { variante, .. } => variante.cout(),
            // charge utile : selon le type.
            Composant::ChargeUtile { variante, .. } => variante.cout(),
            // propulseur : selon la technologie.
            Composant::Propulseur { variante, .. } => variante.cout(),
            // charpente : treillis évasé, coût qui croît avec la longueur.
            Composant::Charpente { longueur, .. } => 3.0 + longueur,
            // radiateur méga : grande aile, coût lourd (échelle mégastructure).
            Composant::RadiateurMega { longueur, .. } => 16.0 + longueur,
            // nacelle moteur : très lourde (bloc propulsion complet).
            Composant::Motrice { .. } => 40.0,
            Composant::BlocMoteur { largeur, .. } => 5.0 * largeur,
            Composant::Reservoir { longueur, .. } => 8.0 + longueur,
            // corps + cœur + collier + 6 bobines + 2 cônes + jet ≈ 11.
            Composant::MoteurAntimatiere { .. } => 11.0,
            // coiffe : capuchon léger (collier + dôme/pyramide/couronne) ≈ 6.
            Composant::Coiffe { .. } => 6.0,
            // réacteur antimatière : cuve + bobines + tuyauterie + tête ≈ 14.
            Composant::ReacteurAntimatiere { .. } => 14.0,
            // anneau hexagonal en treillis : 6 baies × ~9 barres ≈ 12.
            Composant::TreillisHexagone { .. } => 12.0,
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
            Composant::ModuleAxial { profil, longueur, variante } => {
                // Extension axiale (jusqu'au bout du col) ou radiale, la plus
                // grande. Les variantes bombées débordent du rayon nominal, il
                // faut en tenir compte pour le cadrage et l'anti-collision.
                let radial = profil.rayon() * variante.debord_radial();
                (longueur * 0.5 + profil.rayon() * COL_LONG).max(radial)
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
            // Caisson : platine courte + longueur de la boîte.
            Composant::Caisson { longueur, largeur, .. } => {
                (CAISSON_PLATINE + longueur).hypot(largeur * 0.8)
            }
            // Charge à plat : demi-diagonale dans le plan + épaisseur.
            Composant::ChargeUtile { longueur, largeur, .. } => {
                (longueur * 0.5).hypot(largeur * 0.5) + largeur * 0.8
            }
            // Propulseur : le plus long (NERVA, VASIMR) atteint ~1,35 × taille.
            Composant::Propulseur { taille, .. } => taille * 1.4,
            // Charpente : demi-longueur ou demi-largeur de la base évasée.
            Composant::Charpente { grand, longueur, .. } => {
                (longueur * 0.5).max(grand.rayon() * TREILLIS_SECTION * 1.5)
            }
            // Radiateur méga : déploiement (longueur) ou demi-envergure.
            Composant::RadiateurMega { longueur, largeur, .. } => longueur.max(*largeur),
            // Nacelle moteur : extension max (hub avant → boucliers arrière).
            Composant::Motrice { echelle, .. } => 12.0 * echelle,
            // Brique bloc-moteur : rangée d'habitats (large en X, longue en −Z).
            Composant::BlocMoteur { largeur, .. } => 1.3 * largeur,
            // Réservoir : demi-longueur ou bout des barres tétraédriques (r + 2.5r).
            Composant::Reservoir { profil, longueur, .. } => (longueur * 0.5 + profil.rayon()).max(profil.rayon() * 3.5),
            // Moteur antimatière : la cage de stabilisation atteint ~1,6 × taille.
            Composant::MoteurAntimatiere { taille, .. } => taille * 1.62,
            // Coiffe : nez déployé jusqu'à ~1,4 × rayon vers +Z.
            Composant::Coiffe { profil, .. } => profil.rayon() * 1.4,
            // Réacteur antimatière : corps + tête déployés jusqu'à ~1,2 × taille.
            Composant::ReacteurAntimatiere { taille, .. } => taille * 1.2,
            // Anneau hexagonal (+ montants de liaison le long de +Z).
            Composant::TreillisHexagone { profil, liaison } => (profil.rayon() * 1.1).max(*liaison),
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
            | Composant::Adaptateur { .. }
            | Composant::Charpente { .. }
            | Composant::Motrice { .. }
            | Composant::BlocMoteur { .. }
            | Composant::Reservoir { .. } => (Vec3::ZERO, self.rayon_local()),
            // Un propulseur s'étend d'un seul côté de son montage, comme les
            // appendices — mais vers l'arrière quand il est axial.
            Composant::Propulseur { variante, taille, .. } => {
                let sens = if variante.axial() { -1.0 } else { 1.0 };
                (Vec3::Z * (sens * taille * 0.6), taille * 0.95)
            }
            // Moteur antimatière : masse déployée vers l'arrière (−Z), comme un
            // propulseur axial, sphère décalée à mi-corps.
            Composant::MoteurAntimatiere { taille, .. } => (Vec3::NEG_Z * (taille * 0.7), taille * 1.05),
            // Coiffe : nez déployé vers +Z, sphère décalée à mi-hauteur.
            Composant::Coiffe { profil, .. } => {
                let r = profil.rayon();
                (Vec3::Z * (r * 0.6), r * 0.95)
            }
            // Réacteur antimatière : masse déployée vers +Z, sphère à mi-corps.
            Composant::ReacteurAntimatiere { taille, .. } => (Vec3::Z * (taille * 0.5), taille * 0.75),
            // Anneau hexagonal (+ montants) : englobant centré, borné par la liaison.
            Composant::TreillisHexagone { profil, liaison } => (Vec3::ZERO, (profil.rayon() * 1.1).max(*liaison)),
            Composant::PanneauSolaire { .. }
            | Composant::Radiateur { .. }
            | Composant::Antenne { .. }
            | Composant::Caisson { .. }
            | Composant::ChargeUtile { .. }
            | Composant::RadiateurMega { .. } => {
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

    // Chaque variante sait à quelle famille elle appartient, et les trois vues
    // se partagent bien la totalité — aucune orpheline.
    #[test]
    fn familles_de_propulsion_partitionnent_les_variantes() {
        let total: usize = FamillePropulsion::TOUTES
            .iter()
            .map(|f| VariantePropulseur::TOUS.iter().filter(|v| v.famille() == *f).count())
            .sum();
        assert_eq!(total, VariantePropulseur::TOUS.len());
        for f in FamillePropulsion::TOUTES {
            assert!(
                VariantePropulseur::TOUS.iter().any(|v| v.famille() == f),
                "famille vide : {}",
                f.nom()
            );
        }
    }

    // Le montage suit l'usage : un moteur principal se boulonne par une écoutille
    // axiale, une grappe de contrôle se pose sur un port `Surface`.
    #[test]
    fn propulseurs_montage_selon_usage() {
        for v in VariantePropulseur::TOUS {
            let prop = Composant::Propulseur { profil: Profil::P1, variante: v, taille: 1.5 };
            let ports = prop.ports();
            assert_eq!(ports.len(), 1, "{}", v.nom());
            let attendu = if v.axial() { GenrePort::ModuleAxial } else { GenrePort::Surface };
            assert_eq!(ports[0].genre, attendu, "{}", v.nom());
            assert!(prop.cout() > 0.0 && prop.rayon_local() > 0.0, "{}", v.nom());
        }
    }

    // Un moteur axial doit pouvoir se poser sur l'écoutille arrière d'un module,
    // et une grappe sur son flanc.
    #[test]
    fn propulseurs_saccouplent_a_un_module() {
        let m = Composant::ModuleAxial {
            profil: Profil::P1,
            variante: VarianteModule::Standard,
            longueur: 3.0,
        };
        let ports = m.ports();
        let axial = ports.iter().find(|p| p.genre == GenrePort::ModuleAxial).unwrap();
        let flanc = ports.iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        let moteur = Composant::Propulseur {
            profil: Profil::P1,
            variante: VariantePropulseur::TuyereCloche,
            taille: 1.5,
        };
        let grappe = Composant::Propulseur {
            profil: Profil::P0,
            variante: VariantePropulseur::GrappeRcs,
            taille: 0.8,
        };
        assert!(axial.compatible(&moteur.ports()[0]));
        assert!(flanc.compatible(&grappe.ports()[0]));
    }

    fn caisson(v: VarianteCaisson) -> Composant {
        Composant::Caisson { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 }
    }

    // Le caisson est un **porteur** : port de montage en index 0 (avant −Z, vers
    // l'hôte) + un port hôte sur chacune de ses cinq faces libres.
    #[test]
    fn toutes_variantes_caisson_montage_plus_faces_hotes() {
        for v in VarianteCaisson::TOUS {
            let c = caisson(v);
            let ports = c.ports();
            assert_eq!(ports.len(), 6, "{}", v.nom());
            assert!(ports.iter().all(|p| p.genre == GenrePort::Surface), "{}", v.nom());
            // Index 0 = montage, tourné vers l'hôte.
            assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5, "{}", v.nom());
            // Les cinq autres regardent vers l'extérieur, jamais vers l'hôte.
            for hote in &ports[1..] {
                assert!(hote.repere.avant().z > -0.5, "{} : face tournée vers l'hôte", v.nom());
            }
            assert!(c.cout() > 0.0 && c.rayon_local() > 0.0, "{}", v.nom());
        }
    }

    // Le point de la refonte : une charge utile doit pouvoir se poser sur le
    // **côté** d'un caisson technique, pas seulement sur une structure.
    #[test]
    fn charge_utile_dock_sur_le_flanc_dun_caisson() {
        let c = caisson(VarianteCaisson::Ossature);
        let ports = c.ports();
        let flancs: Vec<_> = ports[1..]
            .iter()
            .filter(|p| p.repere.avant().x.abs() > 0.9)
            .collect();
        assert!(!flancs.is_empty(), "le caisson doit offrir des faces latérales");
        for v in VarianteCharge::TOUS {
            let ch = Composant::ChargeUtile {
                profil: Profil::P0,
                variante: v,
                longueur: 1.6,
                largeur: 0.9,
            };
            assert_eq!(ch.ports().len(), 1, "{}", v.nom());
            assert!(flancs[0].compatible(&ch.ports()[0]), "{}", v.nom());
        }
    }

    // Un caisson doit pouvoir s'accoupler sur un port hôte de treillis : c'est
    // tout l'intérêt de la famille non pressurisée (équipements sur poutre).
    #[test]
    fn caisson_dock_sur_treillis() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre };
        let hote = t.ports().into_iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        let c = Composant::Caisson {
            profil: Profil::P0,
            variante: VarianteCaisson::Ossature,
            longueur: 2.0,
            largeur: 1.0,
        };
        assert!(hote.compatible(&c.ports()[0]));
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
