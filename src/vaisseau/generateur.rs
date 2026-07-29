//! Générateur procédural de stations, posé sur le [`Chantier`]
//! (`docs/conception/stations.md`, Partie A §6–7). Une **grammaire** pilote le
//! constructeur : choix d'une ossature, puis habillage des ports libres, le tout
//! borné par un budget et rendu déterministe par une graine.

use super::chantier::Chantier;
use super::montage::{cuire, port_monde, poser};
use super::{
    Assembleur, Composant, EtatStation, GenrePort, Profil, Repere, Sorties, StyleTreillis,
    VarianteAntenne, VarianteCaisson, VarianteCharge, VarianteCoiffe, VarianteModule,
    VariantePanneau, VariantePropulseur, VarianteRadiateur,
};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

// ---------------------------------------------------------------------------
// RNG déterministe (splitmix64) — pas de dépendance externe.
// ---------------------------------------------------------------------------

pub struct Rng {
    etat: u64,
}

impl Rng {
    pub fn new(graine: u64) -> Self {
        Self { etat: graine ^ 0x9E37_79B9_7F4A_7C15 }
    }

    fn suivant(&mut self) -> u64 {
        self.etat = self.etat.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.etat;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Flottant dans [0, 1).
    fn unite(&mut self) -> f32 {
        (self.suivant() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + self.unite() * (b - a)
    }

    fn chance(&mut self, p: f32) -> bool {
        self.unite() < p
    }

    fn choix<T: Copy>(&mut self, v: &[T]) -> T {
        v[(self.suivant() as usize) % v.len()]
    }
}

// ---------------------------------------------------------------------------
// Style : contraint les variantes tirées pour une station cohérente.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    /// Argent + ambre, façon ISS.
    Historique,
    /// Or + bleu, façon Mir / segment russe.
    Russe,
    /// Métal + cyan, futuriste.
    Futuriste,
}

impl Style {
    pub const TOUS: [Style; 3] = [Style::Historique, Style::Russe, Style::Futuriste];

    pub fn nom(self) -> &'static str {
        match self {
            Style::Historique => "HISTORIQUE",
            Style::Russe => "RUSSE",
            Style::Futuriste => "FUTURISTE",
        }
    }

    fn module(self, rng: &mut Rng) -> VarianteModule {
        use VarianteModule::*;
        rng.choix(match self {
            Style::Historique => &[Standard, Hublots, Labo][..],
            Style::Russe => &[Dore, Hublots][..],
            Style::Futuriste => &[Coupole, Gonflable, Labo][..],
        })
    }

    fn panneau(self, rng: &mut Rng) -> VariantePanneau {
        use VariantePanneau::*;
        rng.choix(match self {
            Style::Historique => &[RigideUS, RollOut][..],
            Style::Russe => &[RusseBleu][..],
            Style::Futuriste => &[Futuriste, Hexagonal][..],
        })
    }

    fn radiateur(self, rng: &mut Rng) -> VarianteRadiateur {
        use VarianteRadiateur::*;
        rng.choix(match self {
            Style::Historique => &[PanneauSimple, AccordeonATCS, Caloducs][..],
            Style::Russe => &[PanneauSimple, PivotantTRRJ][..],
            Style::Futuriste => &[Gouttelettes, Deroulable, Corps][..],
        })
    }

    fn antenne(self, rng: &mut Rng) -> VarianteAntenne {
        use VarianteAntenne::*;
        rng.choix(match self {
            Style::Historique => &[ParaboleGG, Cornets][..],
            Style::Russe => &[Fouet, ParaboleGG][..],
            Style::Futuriste => &[ReseauPhase, Helice][..],
        })
    }
}

// ---------------------------------------------------------------------------
// Paramètres + point d'entrée.
// ---------------------------------------------------------------------------

/// Famille d'ossature.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ossature {
    /// Poutre-épine type ISS.
    Iss,
    /// Enfilade de modules type Mir.
    Mir,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ParamsStation {
    pub graine: u64,
    /// Complexité **1..4** : pilote le budget, la longueur et le nombre de branches.
    pub complexite: u8,
    pub style: Style,
    /// `None` = tirée à la graine ; `Some` = forcée (presets).
    pub ossature: Option<Ossature>,
}

impl Default for ParamsStation {
    fn default() -> Self {
        Self { graine: 0, complexite: 2, style: Style::Historique, ossature: None }
    }
}

/// Génère une station complète (déterministe pour une graine donnée).
pub fn generer(p: &ParamsStation) -> EtatStation {
    let c = p.complexite.clamp(1, 4);
    let mut rng = Rng::new(p.graine);
    // **Étalonné sur le réel** : le preset ISS mesure 371 de coût pour 49
    // pièces, et sert de référence pour la complexité 2. Les niveaux
    // s'échelonnent autour : 1 ≈ un tiers d'ISS (Tiangong), 4 ≈ 2,3 × ISS.
    // L'ancienne formule (40 + 55c) plafonnait à 150 au niveau 2 — deux fois
    // et demie trop peu, ce qui expliquait des stations toujours maigres.
    let budget = 120.0 + (c as f32 - 1.0) * 250.0;
    let mut ch = Chantier::avec_budget(budget);

    let iss = match p.ossature {
        Some(o) => o == Ossature::Iss,
        None => rng.chance(0.5),
    };
    // Grammaire en rôles explicites, plutôt qu'en tirages au sort :
    //   1. un **cœur pressurisé** (épine ou grappe selon l'ossature) ;
    //   2. **une** structure de puissance, arrimée au cœur — toujours, quelle
    //      que soit l'ossature : c'est elle qui portera tous les panneaux ;
    //   3. la croissance des modules autour du cœur, d'un seul tenant ;
    //   4. la terminaison des bouts libres ;
    //   5. l'habillage, panneaux sur la poutre et le reste ailleurs.
    if iss {
        coeur_iss(&mut ch, &mut rng, p.style);
    } else {
        coeur_mir(&mut ch, &mut rng, p.style);
    }
    greffer_structure_puissance(&mut ch, &mut rng, c);
    // **La poutre est habillée tout de suite.** Les panneaux sont l'organe
    // vital d'une station : les servir en dernier, c'est les voir sauter quand
    // la grappe a déjà tout dépensé — et livrer une barre nue.
    habiller_surface(&mut ch, &mut rng, p.style, true, None);
    // **La puissance dimensionne l'habitat** (façon ISS : 8 ailes pour 16
    // éléments pressurisés). La grappe croît jusqu'à son quota de modules — ou
    // jusqu'au plancher de budget si la complexité ne permet pas de l'atteindre.
    let quota_modules = (compte_arrays(&ch) as f32 * MODULES_PAR_ARRAY).round() as usize;
    let plancher = budget * 0.20;
    let mut passe = 0u8;
    while compte_modules(&ch) < quota_modules && ch.budget_restant() > plancher && passe < 8 {
        brancher(&mut ch, &mut rng, p.style, passe + 1 < c, plancher, quota_modules);
        passe += 1;
    }
    greffer_propulsion(&mut ch, &mut rng, p.style);
    terminer_extremites(&mut ch, &mut rng);
    // **L'habitat dimensionne le reste** : refroidissement proportionnel aux
    // modules (les radiateurs de poutre comptent), un jeu d'antennes pour les
    // comms. Les faces restantes demeurent nues.
    let modules = compte_modules(&ch);
    let quota_rad = ((modules as f32 * RADIATEURS_PAR_MODULE).round() as usize)
        .saturating_sub(compte_radiateurs(&ch));
    let quota_ant = (1 + modules / 8).saturating_sub(compte_antennes(&ch));
    // Pièces techniques (racks, avionique) plaquées à la grappe : environ un
    // caisson pour quatre modules — les caissons de poutre ne comptent pas,
    // ce sont des équipements exposés (type ELC), pas des racks d'habitat.
    let quota_tech = 1 + modules / 4;
    habiller_surface(&mut ch, &mut rng, p.style, false, Some((quota_rad, quota_ant, quota_tech)));
    ch.terminer()
}

// ---------------------------------------------------------------------------
// Quotas d'équipement, étalonnés sur l'ISS. Le principe : la puissance
// disponible dimensionne le segment pressurisé, qui dimensionne à son tour le
// refroidissement et les comms. Références — preset maison : 13 modules P1 +
// 6 nœuds pour 8 arrays, 10 radiateurs ; ISS réelle : 16 éléments pressurisés
// pour 8 ailes, 14 radiateurs (6 ATCS + 8 PV), quelques antennes (S/Ku-band,
// Lira), moteurs principaux à l'arrière de Zvezda.
// ---------------------------------------------------------------------------

/// Modules P1 par grande aile solaire (preset : 13/8 ≈ 1,6 ; réel : 16/8 = 2).
const MODULES_PAR_ARRAY: f32 = 1.6;
/// Radiateurs par module pressurisé (preset : 10/13 ≈ 0,75 ; réel : 14/16).
const RADIATEURS_PAR_MODULE: f32 = 0.75;

fn compte(ch: &Chantier, f: impl Fn(&Composant) -> bool) -> usize {
    (0..ch.nb_pieces()).filter(|&i| ch.piece(i).is_some_and(|p| f(&p.composant))).count()
}

/// Grandes ailes (celles de la poutre — les petits panneaux ne comptent pas).
fn compte_arrays(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::PanneauSolaire { longueur, .. } if *longueur > 4.5))
}

fn compte_modules(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::ModuleAxial { profil: Profil::P1, .. }))
}

fn compte_radiateurs(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::Radiateur { .. }))
}

fn compte_antennes(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::Antenne { .. }))
}

/// Propulsion principale au bout **le plus en aval** du segment pressurisé
/// (l'ISS a ses moteurs à l'arrière de Zvezda). Posée avant les terminaisons,
/// pour que ce bout reçoive un moteur et non un nez de docking.
fn greffer_propulsion(ch: &mut Chantier, rng: &mut Rng, style: Style) {
    use VariantePropulseur::*;
    let bout = ch
        .libres()
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            p.genre == GenrePort::ModuleAxial
                && p.profil == Profil::P1
                && sur_pressurise(ch, p.origine)
                && p.repere.avant().z.abs() > 0.9
        })
        .max_by(|a, b| a.1.repere.pos.z.abs().total_cmp(&b.1.repere.pos.z.abs()))
        .map(|(i, _)| i);
    let Some(i) = bout else {
        return;
    };
    let variante = match style {
        Style::Historique | Style::Russe => TuyereCloche,
        Style::Futuriste => rng.choix(&[IoniqueGrille, EffetHall, Vasimr]),
    };
    let moteur = Composant::Propulseur { profil: Profil::P1, variante, taille: rng.entre(1.2, 1.8) };
    ch.poser(i, moteur, 0);
}

// Indices des ports d'un nœud Six : 0=+Z, 1=−Z, 2=+X, 3=−X, 4=+Y, 5=−Y. Le boom
// Z1 et la poutre visent ces faces par index ; le reste du preset docke par
// **direction monde** (`porter_vers`), robuste au basculement des nœuds.
const HUB_X_PLUS: usize = 2;
const HUB_X_MOINS: usize = 3;
const HUB_Y_PLUS: usize = 4;

fn hab(v: VarianteModule) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: v, longueur: 3.0 }
}
fn hub6() -> Composant {
    Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six }
}

/// Pose `enfant` (par son port `montage`) sur le port `idx` d'un hôte placé en
/// `hote_monde`, l'ajoute à l'assemblage et renvoie son repère monde.
fn poser_sur(
    asm: &mut Assembleur,
    hote_monde: Repere,
    hote: &Composant,
    idx: usize,
    enfant: &Composant,
    montage: usize,
) -> Repere {
    let m = poser(port_monde(hote_monde, hote, idx), enfant, montage);
    asm.ajouter(cuire(m, enfant));
    m
}

/// Petits arrays russes (bleus) sur les flancs ±X d'un module.
fn arrays_russes(asm: &mut Assembleur, corps: Repere, comp: &Composant) {
    for p in comp.ports() {
        if p.genre != GenrePort::Surface || p.repere.avant().x.abs() < 0.9 {
            continue;
        }
        let pan = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RusseBleu,
            longueur: 3.0,
            largeur: 1.1,
        };
        asm.ajouter(cuire(poser(corps.compose(p.repere), &pan, 0), &pan));
    }
}

/// Pose `app` sur le port `Surface` d'un module situé du côté `dir` (monde), le
/// premier trouvé. Sert à placer des radiateurs/arrays sur une face précise
/// (ex. nadir −Y) sans dépendre de l'index du port.
fn appendice_sur_module(asm: &mut Assembleur, corps: Repere, comp: &Composant, dir: Vec3, app: &Composant) {
    for p in comp.ports() {
        if p.genre != GenrePort::Surface {
            continue;
        }
        let w = corps.compose(p.repere);
        if (w.pos - corps.pos).normalize_or_zero().dot(dir) > 0.7 {
            asm.ajouter(cuire(poser(w, app, 0), app));
            return;
        }
    }
}

/// Docke `enfant` (par son port `montage`) sur le port **structurel** de `hote`
/// dont l'**avant monde** pointe le plus vers `dir`. On vise une direction monde
/// plutôt qu'un index de port : les nœuds basculent (demi-tour) à l'accouplement,
/// donc « le port −Z » ne pointe pas forcément vers −Z monde. Renvoie le repère
/// monde de l'enfant.
fn porter_vers(asm: &mut Assembleur, hote_monde: Repere, hote: &Composant, dir: Vec3, enfant: &Composant, montage: usize) -> Repere {
    let mut best = 0usize;
    let mut best_dot = f32::NEG_INFINITY;
    for (i, p) in hote.ports().iter().enumerate() {
        if p.genre == GenrePort::Surface {
            continue; // on chaîne sur les ports structurels (axiaux/radiaux)
        }
        let avant = (hote_monde.rot * p.repere.avant()).normalize_or_zero();
        let d = avant.dot(dir);
        if d > best_dot {
            best_dot = d;
            best = i;
        }
    }
    let hote_port = hote_monde.compose(hote.ports()[best].repere);
    let w = poser(hote_port, enfant, montage);
    asm.ajouter(cuire(w, enfant));
    w
}

/// Preset : **reproduction de l'ISS**, assemblée à la main (référence pour juger
/// ce qu'il manque au générateur, cf. `docs/suivi/stations.md` Partie B). Poutre déportée
/// au **zénith** par un boom Z1 (elle ne traverse plus le cœur) ; segment US
/// habité (Destiny→Harmony + Columbus/Kibo + grappe Node3/Cupola + nez PMA) et
/// segment russe (FGB→SM + petits arrays + nœud MRM) fore/aft ; Sas Quest, PMM
/// et radiateurs au nadir sur le cœur.
pub fn preset_iss() -> EtatStation {
    let mut asm = Assembleur::new();

    // Unity (Node 1) — cœur.
    let hub = hub6();
    let hub_m = Repere::IDENTITE;
    asm.ajouter(cuire(hub_m, &hub));

    // ===== Poutre au zénith via boom Z1 : Unity(+Y) → Z1 → nœud S0 → poutre ±X.
    // Arrays sur la moitié externe (z < 0), radiateurs sur la moitié interne. =====
    let z1 = Composant::Treillis { profil: Profil::P1, longueur: 3.0, style: StyleTreillis::Carre };
    let z1m = poser_sur(&mut asm, hub_m, &hub, HUB_Y_PLUS, &z1, 0);
    let s0 = hub6();
    let s0m = poser_sur(&mut asm, z1m, &z1, 1, &s0, 1);
    for axe in [HUB_X_PLUS, HUB_X_MOINS] {
        let truss = Composant::Treillis { profil: Profil::P2, longueur: 15.0, style: StyleTreillis::Carre };
        let tm = poser_sur(&mut asm, s0m, &s0, axe, &truss, 0);
        for p in truss.ports() {
            if p.genre != GenrePort::Surface {
                continue;
            }
            let z = p.repere.pos.z; // vers S0 (inboard) ou vers l'extérieur
            let app = if z < -3.5 {
                Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 6.5, largeur: 2.0 }
            } else if z > 3.5 {
                Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::AccordeonATCS, longueur: 3.5, largeur: 1.5 }
            } else if p.repere.pos.x > 0.0 {
                // Zone médiane de poutre : équipements exposés (type ELC),
                // sur une seule face pour ne pas la surcharger.
                Composant::Caisson { profil: Profil::P0, variante: VarianteCaisson::Ossature, longueur: 1.6, largeur: 1.1 }
            } else {
                continue;
            };
            asm.ajouter(cuire(poser(tm.compose(p.repere), &app, 0), &app));
        }
    }

    // ===== Segment US (aft, −Z) : Node1 → Destiny → Harmony, ramifié. =====
    let node1 = hub6();
    let n1 = porter_vers(&mut asm, hub_m, &hub, Vec3::NEG_Z, &node1, 1);
    let lab = hab(VarianteModule::Labo);
    let labm = porter_vers(&mut asm, n1, &node1, Vec3::NEG_Z, &lab, 1);
    let node2 = hub6();
    let n2 = porter_vers(&mut asm, labm, &lab, Vec3::NEG_Z, &node2, 1);
    // Columbus (tribord) et Kibō (bâbord) latéraux sur Harmony.
    porter_vers(&mut asm, n2, &node2, Vec3::X, &hab(VarianteModule::Labo), 1);
    porter_vers(&mut asm, n2, &node2, Vec3::NEG_X, &hab(VarianteModule::Hublots), 1);
    // Module avant + nez de docking PMA/IDA (adaptateur conique P1→P0).
    let av = hab(VarianteModule::Hublots);
    let avm = porter_vers(&mut asm, n2, &node2, Vec3::NEG_Z, &av, 1);
    let nez = Composant::Adaptateur { grand: Profil::P1, petit: Profil::P0, longueur: 1.2 };
    porter_vers(&mut asm, avm, &av, Vec3::NEG_Z, &nez, 0);

    // Grappe Tranquility (Node3) sous Node1 : Cupola (nadir), BEAM (tribord),
    // PMM/Leonardo (bâbord).
    let node3 = hub6();
    let n3 = porter_vers(&mut asm, n1, &node1, Vec3::NEG_Y, &node3, 1);
    porter_vers(&mut asm, n3, &node3, Vec3::NEG_Y, &hab(VarianteModule::Coupole), 1);
    porter_vers(&mut asm, n3, &node3, Vec3::X, &hab(VarianteModule::Gonflable), 1);
    porter_vers(&mut asm, n3, &node3, Vec3::NEG_X, &hab(VarianteModule::Standard), 1);

    // ===== Segment russe (fore, +Z) : Zarya → Zvezda (arrays) → nœud + MRM. =====
    let fgb = hab(VarianteModule::Coeur);
    let fgbm = porter_vers(&mut asm, hub_m, &hub, Vec3::Z, &fgb, 1);
    arrays_russes(&mut asm, fgbm, &fgb);
    let sm = hab(VarianteModule::Coeur);
    let smm = porter_vers(&mut asm, fgbm, &fgb, Vec3::Z, &sm, 1);
    arrays_russes(&mut asm, smm, &sm);
    let rn = hub6();
    let rnm = porter_vers(&mut asm, smm, &sm, Vec3::Z, &rn, 1);
    for dir in [Vec3::Y, Vec3::NEG_Y, Vec3::Z] {
        porter_vers(&mut asm, rnm, &rn, dir, &hab(VarianteModule::Dore), 1);
    }

    // ===== Sur le cœur : Sas Quest (tribord) + radiateurs nadir sur modules. =====
    porter_vers(&mut asm, hub_m, &hub, Vec3::X, &hab(VarianteModule::Sas), 1);
    let radia = Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::PanneauSimple, longueur: 2.6, largeur: 1.2 };
    appendice_sur_module(&mut asm, labm, &lab, Vec3::NEG_Y, &radia);
    appendice_sur_module(&mut asm, smm, &sm, Vec3::NEG_Y, &radia);

    asm.terminer()
}

/// Module habitat de longueur libre (les stations réelles n'ont pas des modules
/// tous identiques).
fn hab_l(v: VarianteModule, longueur: f32) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: v, longueur }
}

/// Nez de docking (PMA/IDA, port Soyouz, port Shenzhou…).
fn nez_docking() -> Composant {
    Composant::Adaptateur { grand: Profil::P1, petit: Profil::P0, longueur: 1.2 }
}

/// Paire d'ailes solaires opposées sur les flancs d'un module, selon l'axe monde
/// `axe`. Les stations russes et chinoises en portent de part et d'autre de
/// presque chaque module — c'est une bonne part de leur silhouette.
#[allow(clippy::too_many_arguments)]
fn paire_ailes(
    asm: &mut Assembleur,
    corps: Repere,
    comp: Composant,
    axe: Vec3,
    variante: VariantePanneau,
    longueur: f32,
    largeur: f32,
) {
    for dir in [axe, -axe] {
        let pan = Composant::PanneauSolaire { profil: Profil::P0, variante, longueur, largeur };
        appendice_sur_module(asm, corps, &comp, dir, &pan);
    }
}

/// Vaisseau amarré (Soyouz-TM, Progress-M) : petit corps à deux ailes. Mir en
/// portait presque toujours un à chaque port axial — sur les vues de référence
/// ils font partie intégrante de la silhouette.
fn vaisseau_amarre(
    asm: &mut Assembleur,
    hote_monde: Repere,
    hote: Composant,
    dir: Vec3,
    axe_ailes: Vec3,
    variante: VariantePanneau,
) {
    let corps = Composant::ModuleAxial {
        profil: Profil::P0,
        variante: VarianteModule::Dore,
        longueur: 2.2,
    };
    let vm = porter_vers(asm, hote_monde, &hote, dir, &corps, 1);
    paire_ailes(asm, vm, corps, axe_ailes, variante, 2.6, 0.9);
}

/// Pose `enfant` sur la **face** de `hote` dont l'avant monde vise `dir`.
/// Contrairement à `porter_vers`, on cible ici les ports **hôtes `Surface`** :
/// c'est ce qui permet d'habiller les cinq faces d'un caisson, par exemple le
/// bus d'un satellite.
fn sur_face(
    asm: &mut Assembleur,
    hote_monde: Repere,
    hote: &Composant,
    dir: Vec3,
    enfant: &Composant,
) -> Option<Repere> {
    let ports = hote.ports();
    let cible = ports.iter().enumerate().find(|(i, p)| {
        // L'index 0 d'un appendice/porteur est son propre montage : on l'ignore.
        *i > 0
            && p.genre == GenrePort::Surface
            && (hote_monde.rot * p.repere.avant()).normalize_or_zero().dot(dir) > 0.9
    })?;
    let w = poser(hote_monde.compose(cible.1.repere), enfant, 0);
    asm.ajouter(cuire(w, enfant));
    Some(w)
}

/// Preset : **satellite de communication** géostationnaire, bâti avec le même
/// vocabulaire que les stations. Bus **parallélépipédique** (`Caisson` fermé),
/// deux grandes ailes solaires opposées, une parabole principale vers la Terre
/// encadrée de deux antennes secondaires, radiateurs sur les faces froides et
/// propulsion électrique de maintien à poste.
pub fn preset_comsat() -> EtatStation {
    let mut asm = Assembleur::new();
    let bus = Composant::Caisson {
        profil: Profil::P1,
        variante: VarianteCaisson::Ferme,
        longueur: 2.6,
        largeur: 2.0,
    };
    let bm = Repere::IDENTITE;
    asm.ajouter(cuire(bm, &bus));

    // Ailes solaires : les deux grandes faces opposées.
    for dir in [Vec3::X, Vec3::NEG_X] {
        let aile = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 6.0,
            largeur: 1.5,
        };
        sur_face(&mut asm, bm, &bus, dir, &aile);
    }
    // Parabole principale vers la Terre + deux antennes secondaires.
    let grande = Composant::Antenne {
        profil: Profil::P0,
        variante: VarianteAntenne::ParaboleGG,
        taille: 1.5,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Z, &grande);
    for (dir, v) in [
        (Vec3::Y, VarianteAntenne::ParaboleOffset),
        (Vec3::NEG_Y, VarianteAntenne::Cornets),
    ] {
        let a = Composant::Antenne { profil: Profil::P0, variante: v, taille: 0.9 };
        sur_face(&mut asm, bm, &bus, dir, &a);
    }
    // Radiateur et propulsion de maintien à poste sur les faces restantes.
    let radia = Composant::Radiateur {
        profil: Profil::P0,
        variante: VarianteRadiateur::PanneauSimple,
        longueur: 2.2,
        largeur: 1.1,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Y, &radia);
    let tuyere = Composant::Propulseur {
        profil: Profil::P0,
        variante: VariantePropulseur::EffetHall,
        taille: 0.8,
    };
    sur_face(&mut asm, bm, &bus, Vec3::NEG_Y, &tuyere);

    asm.terminer()
}

/// Preset : **sonde interplanétaire**. Bus à ossature, grande parabole de
/// liaison, propulsion ionique, palette d'instruments et **perche de
/// magnétomètre** — la perche déportée est la signature d'une sonde.
pub fn preset_sonde() -> EtatStation {
    let mut asm = Assembleur::new();
    let bus = Composant::Caisson {
        profil: Profil::P1,
        variante: VarianteCaisson::Ossature,
        longueur: 2.2,
        largeur: 1.8,
    };
    let bm = Repere::IDENTITE;
    asm.ajouter(cuire(bm, &bus));

    // Grande parabole de liaison, à l'avant.
    let hga = Composant::Antenne {
        profil: Profil::P0,
        variante: VarianteAntenne::ParaboleGG,
        taille: 1.8,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Z, &hga);
    // Ailes solaires modestes : loin du Soleil, on ne peut pas compter dessus.
    for dir in [Vec3::X, Vec3::NEG_X] {
        let aile = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RollOut,
            longueur: 4.2,
            largeur: 1.1,
        };
        sur_face(&mut asm, bm, &bus, dir, &aile);
    }
    // Palette d'instruments sur une face, propulsion ionique de l'autre.
    let palette = Composant::ChargeUtile {
        profil: Profil::P0,
        variante: VarianteCharge::Palette,
        longueur: 1.6,
        largeur: 1.0,
    };
    sur_face(&mut asm, bm, &bus, Vec3::NEG_Y, &palette);
    let moteur = Composant::Propulseur {
        profil: Profil::P0,
        variante: VariantePropulseur::IoniqueGrille,
        taille: 1.0,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Y, &moteur);

    // Perche de magnétomètre : un treillis fin, éloigné du bus pour fuir ses
    // perturbations magnétiques, avec le capteur au bout.
    let perche = Composant::Treillis {
        profil: Profil::P0,
        longueur: 5.0,
        style: StyleTreillis::Triangulaire,
    };
    let pm = poser(bm.compose(bus.ports()[0].repere), &perche, 0);
    asm.ajouter(cuire(pm, &perche));
    // Tête de capteur au bout : montage axial (index 1 = extrémité libre de la
    // poutre), le seul genre de port qu'accepte le bout d'un treillis.
    let capteur = Composant::ModuleAxial {
        profil: Profil::P0,
        variante: VarianteModule::Standard,
        longueur: 0.6,
    };
    poser_sur(&mut asm, pm, &perche, 1, &capteur, 0);

    asm.terminer()
}

/// Preset : **reproduction de Mir** en configuration finale (1996–2001).
///
/// Topologie caractéristique : le cœur DOS-7 porte à l'avant un **nœud
/// sphérique à 5 ports** — un port axial (Soyouz/Progress) et **quatre radiaux**
/// occupés par Kvant-2, Kristall, Spektr et Priroda, d'où la « croix » de
/// modules si reconnaissable. Kvant-1, lui, est amarré à l'**arrière** du cœur
/// (et non au nœud) et offre le second port Soyouz. Module d'amarrage navette
/// au bout de Kristall, ailes solaires sur presque chaque module.
pub fn preset_mir() -> EtatStation {
    /// Cote réelle (mètres) → unités monde. Le profil P1 (rayon 1 U) représente
    /// le rayon du module central, dont le diamètre réel est 4,15 m ; les
    /// longueurs ci-dessous sont donc les cotes publiées, à l'échelle.
    fn cote(metres: f32) -> f32 {
        metres / 2.075
    }

    let mut asm = Assembleur::new();
    let bleu = VariantePanneau::RusseBleu;
    // Grandes ailes de Mir : ~10,6 m × 3,9 m. Elles sont presque aussi longues
    // que les modules — c'est très marqué sur les vues de référence.
    let (aile_l, aile_w) = (cote(10.6), cote(3.9));

    // Cœur DOS-7 (13,13 m), deux grandes ailes latérales + antennes arrière.
    let coeur = hab_l(VarianteModule::Coeur, cote(13.13));
    let cm = Repere::IDENTITE;
    asm.ajouter(cuire(cm, &coeur));
    paire_ailes(&mut asm, cm, coeur.clone(), Vec3::Y, bleu, aile_l, aile_w);
    let antenne = Composant::Antenne {
        profil: Profil::P0,
        variante: VarianteAntenne::ParaboleOffset,
        taille: 0.9,
    };
    appendice_sur_module(&mut asm, cm, &coeur, Vec3::X, &antenne);

    // Nœud sphérique avant (5 ports libres) + Soyouz-TM au port axial.
    let noeud = hub6();
    let nm = porter_vers(&mut asm, cm, &coeur, Vec3::Z, &noeud, 1);
    vaisseau_amarre(&mut asm, nm, noeud.clone(), Vec3::Z, Vec3::Y, bleu);

    // La croix autour du nœud, dans la disposition du schéma d'assemblage :
    // Priroda au zénith, Kristall au nadir, Kvant-2 et Spektr sur les flancs.

    // Priroda : le seul module **sans aile** (il tournait sur batteries).
    let priroda = hab_l(VarianteModule::Labo, cote(11.90));
    porter_vers(&mut asm, nm, &noeud, Vec3::Y, &priroda, 1);

    // Kristall : **sans aile non plus** en configuration finale — les siennes
    // ont été transférées sur Kvant-1. Il porte le module d'amarrage navette
    // (4,70 m × Ø 2,20), un vrai module étroit et non un simple nez.
    let kristall = hab_l(VarianteModule::Labo, cote(11.90));
    let kri = porter_vers(&mut asm, nm, &noeud, Vec3::NEG_Y, &kristall, 1);
    let so = Composant::ModuleAxial {
        profil: Profil::P0,
        variante: VarianteModule::Standard,
        longueur: cote(4.70),
    };
    let som = porter_vers(&mut asm, kri, &kristall, Vec3::NEG_Y, &so, 1);
    porter_vers(&mut asm, som, &so, Vec3::NEG_Y, &nez_docking(), 0);

    // Kvant-2 : le plus long des modules (13,73 m).
    let kvant2 = hab_l(VarianteModule::Hublots, cote(13.73));
    let kv2 = porter_vers(&mut asm, nm, &noeud, Vec3::X, &kvant2, 1);
    paire_ailes(&mut asm, kv2, kvant2, Vec3::Y, bleu, aile_l, aile_w);

    // Spektr : **quatre** ailes (deux paires croisées), sa signature.
    let spektr = hab_l(VarianteModule::Standard, cote(9.10));
    let spe = porter_vers(&mut asm, nm, &noeud, Vec3::NEG_X, &spektr, 1);
    paire_ailes(&mut asm, spe, spektr.clone(), Vec3::Y, bleu, cote(8.6), cote(3.4));
    paire_ailes(&mut asm, spe, spektr, Vec3::Z, bleu, cote(7.0), cote(3.0));

    // Kvant-1 à l'arrière du cœur : tonneau court (5,80 m), mais porteur des
    // grandes ailes reprises de Kristall. Progress-M à son port arrière.
    let kvant1 = hab_l(VarianteModule::Dore, cote(5.80));
    let kv1 = porter_vers(&mut asm, cm, &coeur, Vec3::NEG_Z, &kvant1, 1);
    paire_ailes(&mut asm, kv1, kvant1.clone(), Vec3::Y, bleu, aile_l, aile_w);
    vaisseau_amarre(&mut asm, kv1, kvant1, Vec3::NEG_Z, Vec3::Y, bleu);

    asm.terminer()
}

/// Preset : **reproduction de Tiangong** (configuration en T, depuis 2022).
///
/// Le cœur **Tianhe** porte à l'avant un nœud dont deux ports **radiaux
/// opposés** reçoivent les laboratoires **Wentian** et **Mengtian** — d'où le T
/// caractéristique. Le nœud offre aussi un port avant et un port **nadir**
/// (Shenzhou habité) ; l'arrière du cœur reçoit le cargo Tianzhou. Les ailes des
/// laboratoires sont nettement plus grandes que celles du cœur (27 m contre
/// 12,6 m dans la réalité), ce qui fait beaucoup de la silhouette.
pub fn preset_tiangong() -> EtatStation {
    let mut asm = Assembleur::new();
    let ailes = VariantePanneau::RollOut; // grandes ailes souples, sombres

    // Tianhe : cœur + ailes moyennes.
    let tianhe = hab_l(VarianteModule::Standard, 5.0);
    let thm = Repere::IDENTITE;
    asm.ajouter(cuire(thm, &tianhe));
    paire_ailes(&mut asm, thm, tianhe.clone(), Vec3::Y, ailes, 4.5, 1.2);
    // Port arrière : cargo Tianzhou.
    porter_vers(&mut asm, thm, &tianhe, Vec3::NEG_Z, &nez_docking(), 0);

    // Nœud avant : port axial (Shenzhou / télescope) + port nadir habité.
    let noeud = hub6();
    let nm = porter_vers(&mut asm, thm, &tianhe, Vec3::Z, &noeud, 1);
    porter_vers(&mut asm, nm, &noeud, Vec3::Z, &nez_docking(), 0);
    porter_vers(&mut asm, nm, &noeud, Vec3::NEG_Y, &nez_docking(), 0);

    // La barre du T : les deux laboratoires, avec leurs grandes ailes.
    let wentian = hab_l(VarianteModule::Labo, 5.0);
    let wm = porter_vers(&mut asm, nm, &noeud, Vec3::X, &wentian, 1);
    paire_ailes(&mut asm, wm, wentian.clone(), Vec3::Y, ailes, 7.5, 1.6);
    // Sas EVA au bout de Wentian.
    porter_vers(&mut asm, wm, &wentian, Vec3::X, &hab_l(VarianteModule::Sas, 2.0), 1);

    let mengtian = hab_l(VarianteModule::Labo, 5.0);
    let mm = porter_vers(&mut asm, nm, &noeud, Vec3::NEG_X, &mengtian, 1);
    paire_ailes(&mut asm, mm, mengtian.clone(), Vec3::Y, ailes, 7.5, 1.6);
    // Sas cargo au bout de Mengtian, et sa **plateforme exposée** — Mengtian
    // en porte une, qui accueille des charges utiles hors pression.
    porter_vers(&mut asm, mm, &mengtian, Vec3::NEG_X, &hab_l(VarianteModule::Sas, 2.0), 1);
    let palette = Composant::ChargeUtile {
        profil: Profil::P0,
        variante: VarianteCharge::Palette,
        longueur: 2.6,
        largeur: 1.4,
    };
    appendice_sur_module(&mut asm, mm, &mengtian, Vec3::Z, &palette);

    asm.terminer()
}

/// Styles d'anneau. Le helper `poser_anneau` place le même squelette (segments
/// sur les arêtes + joints aux sommets) ; la variante décide **quoi** met sur
/// chaque arête, et à quel gabarit. C'est ce qui fait qu'un même code sert la
/// roue habitée, le grand tore, la ceinture agricole de l'O'Neill et l'armature
/// nue d'un anneau en construction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VarianteAnneau {
    /// Roue habitée : modules pressurisés à hublots (P1).
    Habitation,
    /// Grand anneau lourd : gros modules lisses (P2), à grand rayon.
    Large,
    /// Ceinture agricole : modules-serres vitrés (P1) — type O'Neill.
    Serre,
    /// Armature nue : segments de treillis, pas encore habillés.
    Structure,
}

impl VarianteAnneau {
    pub const TOUS: [VarianteAnneau; 4] = [
        VarianteAnneau::Habitation,
        VarianteAnneau::Large,
        VarianteAnneau::Serre,
        VarianteAnneau::Structure,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteAnneau::Habitation => "HABITATION",
            VarianteAnneau::Large => "GRAND ANNEAU (P2)",
            VarianteAnneau::Serre => "CEINTURE AGRICOLE",
            VarianteAnneau::Structure => "ARMATURE NUE",
        }
    }

    /// Profil des segments (donc du joint) — P2 pour le grand anneau, P1 sinon.
    fn profil(self) -> Profil {
        match self {
            VarianteAnneau::Large => Profil::P2,
            _ => Profil::P1,
        }
    }

    /// Segment posé sur une arête de longueur `corde`.
    fn segment(self, corde: f32, st: StyleTreillis) -> Composant {
        match self {
            VarianteAnneau::Habitation => Composant::ModuleAxial {
                profil: Profil::P1,
                variante: VarianteModule::Hublots,
                longueur: corde * 0.78,
            },
            VarianteAnneau::Serre => Composant::ModuleAxial {
                profil: Profil::P1,
                variante: VarianteModule::Serre,
                longueur: corde * 0.78,
            },
            VarianteAnneau::Large => Composant::ModuleAxial {
                profil: Profil::P2,
                // Lisse (pas `Coeur` étagé) : le tambour cassait le cercle.
                variante: VarianteModule::Hublots,
                longueur: corde * 0.9,
            },
            VarianteAnneau::Structure => Composant::Treillis {
                profil: Profil::P1,
                longueur: corde * 0.9,
                style: st,
            },
        }
    }
}

/// Pose un **anneau fermé** dans le plan défini par `axe`, centré sur `centre`.
/// Un anneau ne se construit pas par chaînage de ports : `accoupler`
/// accumulerait une erreur d'orientation à chaque joint et la boucle ne se
/// refermerait jamais. On calcule donc directement les `n` positions sur le
/// cercle et on **cuit** chaque pièce à sa place. Helper réutilisable — l'ISV et
/// l'O'Neill en posent plusieurs, de styles et de tailles différents.
pub(crate) fn poser_anneau(
    asm: &mut Assembleur,
    centre: Vec3,
    axe: Vec3,
    n: usize,
    rayon: f32,
    style: VarianteAnneau,
) {
    let axe = axe.normalize_or_zero();
    if axe == Vec3::ZERO || n < 3 {
        return;
    }
    // Base orthonormée du plan de l'anneau (u dans le plan, v = axe × u).
    let ref_u = if axe.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = (ref_u - axe * ref_u.dot(axe)).normalize();
    let v = axe.cross(u);
    let sommet = |a: f32| centre + (u * a.cos() + v * a.sin()) * rayon;
    let corde = 2.0 * rayon * (PI / n as f32).sin();
    let profil = style.profil();
    let lj = if profil == Profil::P2 { 1.0 } else { 0.8 };
    for k in 0..n {
        let a0 = TAU * k as f32 / n as f32;
        let p0 = sommet(a0);
        let p1 = sommet(TAU * (k as f32 + 1.0) / n as f32);
        // Segment sur l'arête, orienté le long de la corde.
        let dir = (p1 - p0).normalize_or_zero();
        let seg = style.segment(corde, StyleTreillis::Carre);
        asm.ajouter(cuire(Repere::new((p0 + p1) * 0.5, Quat::from_rotation_arc(Vec3::Z, dir)), &seg));
        // Joint d'angle au sommet, aligné sur la **tangente** : un court
        // adaptateur qui masque le coude entre deux segments successifs.
        let tang = (-u * a0.sin() + v * a0.cos()).normalize_or_zero();
        let joint = Composant::Adaptateur { grand: profil, petit: profil, longueur: lj };
        asm.ajouter(cuire(Repere::new(p0, Quat::from_rotation_arc(Vec3::Z, tang)), &joint));
    }
}

/// Vitrine des styles d'anneau, alignés côte à côte. Chaque style a **sa propre
/// échelle** : les gros modules P2 exigent un anneau bien plus grand pour former
/// des segments longs et fins plutôt qu'un pâté de blocs trapus.
pub fn demo_anneaux() -> EtatStation {
    let mut asm = Assembleur::new();
    // (segments, rayon) par style : le grand anneau tourne à ~12 avec peu de
    // segments (modules longs), les autres restent compacts.
    let params = |s: VarianteAnneau| match s {
        VarianteAnneau::Large => (9usize, 12.0f32),
        VarianteAnneau::Structure => (16, 6.0),
        _ => (12, 6.0),
    };
    let gap = 4.0;
    let mut x = 0.0;
    let mut prev_r = 0.0;
    for (i, style) in VarianteAnneau::TOUS.iter().enumerate() {
        let (n, r) = params(*style);
        if i > 0 {
            x += prev_r + gap + r;
        }
        poser_anneau(&mut asm, Vec3::new(x, 0.0, 0.0), Vec3::Z, n, r, *style);
        prev_r = r;
    }
    asm.terminer()
}

/// Preset : **station à anneau** — un moyeu sur une épine courte, un anneau
/// d'habitation encerclant l'axe, relié au moyeu par quatre rayons en treillis.
/// Premier jalon vers les grandes stations (et l'ISV) : gravité artificielle par
/// rotation, silhouette de « roue » façon 2001 / Stanford.
pub fn preset_anneau() -> EtatStation {
    let mut asm = Assembleur::new();
    let rayon = 12.0;
    let n = 16;

    // Moyeu + épine courte le long de l'axe Z.
    let hub = Composant::Noeud { profil: Profil::P2, sorties: Sorties::Six };
    let hm = Repere::IDENTITE;
    asm.ajouter(cuire(hm, &hub));
    for dir in [Vec3::Z, Vec3::NEG_Z] {
        let m = Composant::ModuleAxial { profil: Profil::P2, variante: VarianteModule::Coeur, longueur: 3.0 };
        porter_vers(&mut asm, hm, &hub, dir, &m, 1);
    }

    // Anneau d'habitation dans le plan XY (axe = Z, celui de l'épine).
    poser_anneau(&mut asm, Vec3::ZERO, Vec3::Z, n, rayon, VarianteAnneau::Habitation);

    // Rayons : quatre bras en treillis du moyeu vers la jante.
    let (inner, outer) = (3.6, rayon - 1.2);
    for s in 0..4 {
        let a = TAU * s as f32 / 4.0;
        let radial = Vec3::new(a.cos(), a.sin(), 0.0);
        let mid = radial * ((inner + outer) * 0.5);
        let bras = Composant::Treillis { profil: Profil::P1, longueur: outer - inner, style: StyleTreillis::Carre };
        asm.ajouter(cuire(Repere::new(mid, Quat::from_rotation_arc(Vec3::Z, radial)), &bras));
    }

    asm.terminer()
}

/// Vue détachée d'**un seul** radiateur méga, en gros, pour travailler sa forme :
/// collecteur (parallélépipède fin + connecteur central) + aile trapézoïdale
/// courbe très allongée.
pub fn demo_radiateur_mega() -> EtatStation {
    let mut asm = Assembleur::new();
    let r = Composant::RadiateurMega { profil: Profil::P0, longueur: 26.0, largeur: 5.5, ailettes: 34 };
    asm.ajouter(cuire(Repere::IDENTITE, &r));
    asm.terminer()
}

/// Vue détachée de la **brique du bloc propulsion** : conteneur (style ossature,
/// treillis dessus) côté +Z, puis **connecteur → rangée de 5 habitats gonflables
/// (BEAM, id 4)** en ligne selon X, Ø calé sur l'épaisseur du bloc, côté radiateur
/// (−Z). Le bout de la rangée docke sur le connecteur du radiateur.
pub fn demo_moteur_antimatiere() -> EtatStation {
    let mut asm = Assembleur::new();
    let bloc = Composant::BlocMoteur { profil: Profil::P2, largeur: 4.4 };
    let bm = Repere::IDENTITE;
    asm.ajouter(cuire(bm, &bloc));
    // Habitat n°9 (Cœur) de l'autre côté de la brique technique (+Z).
    let hab9 = Composant::ModuleAxial { profil: Profil::P2, variante: VarianteModule::Coeur, longueur: 4.5 };
    porter_vers(&mut asm, bm, &bloc, Vec3::Z, &hab9, 1); // montage 1 = habitat retourné
    asm.terminer()
}

/// Preset : **charpente de l'ISV** (première pierre du vaisseau fidèle). Une
/// **seule** charpente à profil **courbe** — plus de cône + tige assemblés : la
/// base (P3) s'évase vers le bas puis s'affine (P0) en une longue flèche vers le
/// haut, d'un seul tenant. Moteurs, habitat et voiles radiateurs viendront s'y
/// accrocher ensuite.
pub fn preset_isv() -> EtatStation {
    let mut asm = Assembleur::new();

    // Charpente continue, axe le long de +Y (« vers le haut »). Base (P3, bout
    // −Z local) vers le bas, apex (P0, +Z local) vers le haut. `courbure` élevée
    // → l'évasement se concentre en bas, le reste file en flèche fine.
    // Évasement fixe (distance absolue) → rallonger `longueur` n'allonge que la
    // flèche : ~34 → 40 ajoute un tiers de tige sans toucher à la base.
    // `aiguille: true` → anneau hexagonal en treillis à la base (côté moteurs).
    let longueur = 84.0; // flèche rallongée (+20 % : la « barre » agrandie)
    let charpente = Composant::Charpente {
        grand: Profil::P3,
        petit: Profil::P1, // bout agrandi (P0 → P1, section doublée)
        longueur,
        courbure: 2.6,
        aiguille: true,
    };
    // Position = centre local de la charpente. Base ancrée à Y = −16, la
    // rallonge part vers le **haut** (Y_centre = base + L/2), plus un **décalage
    // vers le haut d'un bon dixième de la hauteur** (la charpente monte, les
    // radiateurs/moteurs restent en bas).
    let decalage = longueur * 0.1;
    let y_centre = -16.0 + longueur * 0.5 + decalage;
    let base = Repere::new(vec3(0.0, y_centre, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::Y));
    asm.ajouter(cuire(base, &charpente));

    // Deux grandes **ailes radiateur méga** près de la base évasée (côté
    // moteurs). Le vaisseau s'étend en Y ∈ [−20, +20], base vers −Y.
    //
    // **Côte à côte, même sens** (pas en miroir) : les deux se déploient le long
    // de +Y (vers le corps), connecteur en bas côté base. Puis on les **bascule
    // de 5°** pour que la **pointe rentre vers l'intérieur** — donc le connecteur
    // (où viendra le propulseur à antimatière) part **vers l'extérieur** : le
    // moteur ne tire plus dans la station.
    let tilt = 5.0_f32.to_radians();
    let radia_w = 4.4 * 1.2; // largeur du radiateur, +20 %
    for cote in [1.0_f32, -1.0] {
        let aile = Composant::RadiateurMega {
            profil: Profil::P0,
            longueur: 16.5,
            largeur: radia_w,
            ailettes: 28,
        };
        let orient = Quat::from_rotation_arc(Vec3::Z, Vec3::NEG_Y); // sens inversé
        let rot = Quat::from_rotation_z(cote * tilt) * orient;
        let pos = Vec3::new(-6.5 * cote, -20.0, 0.0); // permutées, écartées un peu plus, descendues d'une demi-épine
        let repere = Repere::new(pos, rot);
        asm.ajouter(cuire(repere, &aile));
        // Bloc moteur docké au collecteur de CE radiateur, comme dans la vue
        // radiateur+bloc moteur. Le côté −X est le **flip** de l'autre (miroir).
        // `propulseur = true` : version **complète** (Cœur 3 noir + chapes bombées
        // sur Cœur 1/2 + propulseur antimatière) intégrée à la charpente.
        poser_bloc_moteur(&mut asm, repere, radia_w, cote < 0.0, true);
    }

    // **Réservoirs de carburant** : une cuve sphérique de **chaque côté (±X)** du
    // volume hexagonal, à sa hauteur. Le centre de l'hexagone est recalculé comme
    // au dessin (`Charpente`) : base de la charpente moins `ap + sec`.
    let sg = Profil::P3.rayon() * 0.5; // section du treillis (= TREILLIS_SECTION)
    let cote = 2.0 * sg; // côté de l'hexagone
    let ap = cote * 3.0_f32.sqrt() * 0.5; // apothème
    let hex_y = y_centre - longueur * 0.5 - (ap + sg * 0.5); // centre hexagone (avant pivot)
    let prof = sg; // demi-profondeur du volume hexagonal (hors-plan = axe Z)
    let res_long = 5.0_f32; // taille des cuves
    let res_r = res_long * 0.5 * 1.3; // rayon de la cuve sphérique (cf. dessin Reservoir)
    let reservoir = Composant::Reservoir { profil: Profil::P1, longueur: res_long, cage: false };
    // Une cuve sur **chaque face** de l'hexagone, le long de ±Z — l'axe
    // **perpendiculaire** aux radiateurs. Radiateurs sur ±X (→ nord/sud), cuves
    // sur ±Z (→ est/ouest). Reculées assez pour dégager la face de l'hexagone.
    // Le tétraèdre a sa face plate côté −Z local : la cuve +Z la présente déjà à
    // l'hexagone ; on **retourne** la cuve −Z (½ tour en X) pour que sa face
    // plate regarde l'hexagone elle aussi. Les deux faces plates restent
    // **parallèles** à la surface de l'hexagone.
    // Rotation autour de l'axe Z local : fait pivoter la face triangulaire
    // (perpendiculaire à Z) présentée à l'hexagone, sans changer son orientation
    // générale. 30° d'alignement **+ 180°** (demi-tour Z : les cuves étaient
    // orientées à l'envers dans la vue ISV).
    let spin = Quat::from_rotation_z(PI / 6.0 + PI);
    // **Deux cuves par côté** : l'existante au niveau de l'hexagone, et une
    // dupliquée de l'**autre côté de la charpente** (sous l'hexagone, −Y) avec un
    // **demi-tour en Z** supplémentaire.
    // Écart = **2·ap** : les deux plaques hexagonales (demi-hauteur `ap` en Y) se
    // touchent bord à bord (plus d'écart). Les réservoirs suivent (léger
    // chevauchement des cuves accepté).
    let dy = 2.0 * ap;
    for sz in [1.0_f32, -1.0] {
        let base = if sz > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_x(PI) };
        // Le retournement (rotation_x PI) de la cuve −Z **mire** le triangle, ce qui
        // désaligne son sommet de 60° : on le rattrape pour que les deux cuves
        // pointent un sommet vers la tige de la charpente.
        let corr = if sz > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_z(PI / 3.0) };
        let rot = base * spin * corr;
        let z = sz * (prof + res_r - 1.0); // réservoir enfoncé dans la charpente (écart −1.0)
        // Cuve d'origine.
        asm.ajouter(cuire(Repere::new(vec3(0.0, hex_y, z), rot), &reservoir));
        // Cuve dupliquée : autre côté de la charpente + demi-tour Z.
        let rot2 = rot * Quat::from_rotation_z(PI);
        asm.ajouter(cuire(Repere::new(vec3(0.0, hex_y - dy, z), rot2), &reservoir));
    }

    // **Second anneau hexagonal** au niveau du groupe de réservoirs dupliqué
    // (`hex_y - dy`) : comme l'écart vaut `2·ap`, il **touche** bord à bord celui
    // du pied de la charpente. Plus besoin de montants (`liaison = 0`).
    let hexa = Composant::TreillisHexagone { profil: Profil::P3, liaison: 0.0 };
    let hexa_rot = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    asm.ajouter(cuire(Repere::new(vec3(0.0, hex_y - dy, 0.0), hexa_rot), &hexa));

    // **Modèle complet à l'horizontale** : rotation globale de 90° autour de Z
    // (l'axe +Y du vaisseau bascule vers +X), appliquée à toutes les pièces.
    pivoter(asm.terminer(), Quat::from_rotation_z(-FRAC_PI_2))
}

/// Applique une rotation globale `q` (autour de l'origine monde) à **toutes** les
/// pièces d'un état déjà publié : chaque transformée cuite est pré-multipliée par
/// `q`. Sert à réorienter un modèle complet (ex. ISV couché à l'horizontale).
fn pivoter(etat: EtatStation, q: Quat) -> EtatStation {
    let EtatStation::Prete(s) = etat else {
        return etat;
    };
    let rot = Mat4::from_quat(q);
    let pieces: Vec<super::Piece> = s
        .pieces()
        .iter()
        .map(|p| super::Piece::new(rot * p.transforme, p.composant.clone()))
        .collect();
    super::Station::depuis_pieces(pieces)
        .map(EtatStation::Prete)
        .unwrap_or(EtatStation::Vide)
}

/// Seconde vue ISV (chantier progressif) : un **radiateur méga** et le **bloc
/// moteur** (caisse collecteur + `ModuleAxial` doré) assemblé **au bout de son
/// connecteur**. On fait évoluer la jonction petit à petit.
pub fn preset_isv_moteur() -> EtatStation {
    let mut asm = Assembleur::new();

    // Largeur hors-tout du bloc moteur = envergure de sa rangée de 5 habitats
    // (même formule que `BlocMoteur::dessiner`, largeur de caisse 4.4).
    let bloc_w = 4.4_f32;
    let hy = bloc_w * 0.42;
    let lx = 4.0 * (hy * 1.05 * 0.66) + 2.0 * (hy * 0.5 * 0.66);

    // Radiateur méga : **même largeur** que le bloc moteur, **hauteur doublée**
    // (longueur 16.5 → 33). Centré en X (pas d'offset).
    let radia = Composant::RadiateurMega {
        profil: Profil::P0,
        longueur: 33.0,
        largeur: lx,
        ailettes: 28,
    };
    asm.ajouter(cuire(Repere::IDENTITE, &radia));

    // Bloc moteur docké au collecteur du radiateur (radiateur au repère identité).
    // `propulseur = true` : Cœur 3 reçoit le propulseur à antimatière complet.
    poser_bloc_moteur(&mut asm, Repere::IDENTITE, lx, false, true);

    asm.terminer()
}

/// Pose le **bloc moteur** complet (caisse + rangée d'habitats + les 3 Cœurs),
/// **docké au collecteur** du radiateur dont le repère monde est `radia` (de
/// largeur `radia_largeur`) — exactement comme la vue « radiateur + bloc moteur ».
/// Tout est composé dans le repère du radiateur, donc valable même incliné.
fn poser_bloc_moteur(asm: &mut Assembleur, radia: Repere, radia_largeur: f32, miroir: bool, propulseur: bool) {
    let bloc_w = 4.4_f32;
    let hy = bloc_w * 0.42;
    let lx = 4.0 * (hy * 1.05 * 0.66) + 2.0 * (hy * 0.5 * 0.66); // envergure rangée

    // Bloc retourné, bout −Z de la rangée **au ras du départ du panneau**
    // (z0 = +bd/2). Le collecteur ayant été retiré, c'est là que commence la
    // matière du radiateur : on comble ainsi l'écart laissé par la caisse.
    let bd = (radia_largeur * 0.5).max(1.5);
    let face = bd * 0.5;
    let bm = radia.compose(Repere::new(vec3(0.0, 0.0, face - 4.1), Quat::from_rotation_y(PI)));

    let bloc = Composant::BlocMoteur { profil: Profil::P2, largeur: bloc_w };
    asm.ajouter(cuire(bm, &bloc));

    let port9 = bm.compose(bloc.ports()[1].repere); // port +Z du bloc
    // Axe de la rangée de Cœurs. `miroir` inverse le côté → l'autre moteur est
    // l'exact **flip en X** (symétrie non cassée).
    let xdir = bm.rot * if miroir { Vec3::X } else { Vec3::NEG_X };

    // Cœur 1 : P2, longueur 9.0, offset +25 % le long de xdir.
    let coeur1 = Composant::ModuleAxial { profil: Profil::P2, variante: VarianteModule::Coeur, longueur: 9.0 };
    let mut w1 = poser(port9, &coeur1, 1);
    w1.pos += xdir * (0.25 * lx);
    asm.ajouter(cuire(w1, &coeur1));

    // Cœur 2 : P2, longueur 4.5, tambour contre tambour à Cœur 1 (chevauche le bloc).
    let coeur2 = Composant::ModuleAxial { profil: Profil::P2, variante: VarianteModule::Coeur, longueur: 4.5 };
    let port_tambour = w1.compose(coeur1.ports()[1].repere);
    let w2 = poser(port_tambour, &coeur2, 1);
    asm.ajouter(cuire(w2, &coeur2));

    // Chapes **bombées** sur les bouts **exposés** (+Z) de Cœur 1 et Cœur 2
    // (leurs bouts −Z sont joints tambour contre tambour). Posées **à ras du
    // corps** (face +Z, à `demi`) et non au bout de la collerette — sinon un cou
    // de docking resterait visible sous la chape.
    for (w, demi) in [(w1, 9.0_f32 * 0.5), (w2, 4.5_f32 * 0.5)] {
        let coiffe = Composant::Coiffe { profil: Profil::P2, variante: VarianteCoiffe::Bombee };
        let cw = w.compose(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY));
        asm.ajouter(cuire(cw, &coiffe));
    }

    // Cœur 3 : P1 (diamètre moitié), longueur 9.0, collé sans écart côté −xdir de Cœur 1.
    let coeur3 = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Coeur, longueur: 9.0 };
    let mut w3 = poser(port9, &coeur3, 1);
    w3.pos = w1.pos - xdir * (Profil::P2.rayon() + Profil::P1.rayon());
    asm.ajouter(cuire(w3, &coeur3));

    // Propulseur à antimatière accroché au **bout libre (+Z)** de Cœur 3 : le
    // réacteur se clipse par sa **tête** (port 1), et la tuyère se monte sous sa
    // **base** (port 0) — la poussée pointe donc vers l'extérieur du module.
    if propulseur {
        // Taille choisie pour que le **corps du réacteur** (rayon = 0.40·taille,
        // cf. `ReacteurAntimatiere::dessiner`) ait le **même diamètre que Cœur 3**
        // (Profil::P1) au niveau du raccord : 0.40·taille = P1.rayon().
        let taille = Profil::P1.rayon() / 0.40;
        let port_c3 = w3.compose(coeur3.ports()[0].repere); // écoutille axiale libre (+Z)
        let reacteur = Composant::ReacteurAntimatiere { profil: Profil::P1, taille };
        let rw = poser(port_c3, &reacteur, 1); // montage par la tête (+Z)
        asm.ajouter(cuire(rw, &reacteur));
        let tuyere = Composant::MoteurAntimatiere { profil: Profil::P1, taille };
        let port_base = rw.compose(reacteur.ports()[0].repere); // base −Z du réacteur
        asm.ajouter(cuire(poser(port_base, &tuyere, 0), &tuyere));
    }
}

/// Vue briques : la **charpente de l'ISV** (treillis conique courbe) à gauche, et
/// à droite la **même mais terminée en tête d'aiguille** (apex prolongé en flèche
/// fine). Debout, apex vers le haut.
pub fn demo_charpente() -> EtatStation {
    let mut asm = Assembleur::new();
    let debout = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    for (dx, aiguille) in [(-8.0_f32, false), (8.0, true)] {
        let ch = Composant::Charpente {
            grand: Profil::P3,
            petit: Profil::P0,
            longueur: 40.0,
            courbure: 2.6,
            aiguille,
        };
        asm.ajouter(cuire(Repere::new(vec3(dx, 0.0, 0.0), debout), &ch));
    }
    asm.terminer()
}

/// Vue briques : prototype de **réservoir de carburant** cylindrique (calottes
/// sphériques), cerclé de 4 barres métal en position tétraédrique.
pub fn demo_reservoir() -> EtatStation {
    let mut asm = Assembleur::new();
    let res = Composant::Reservoir { profil: Profil::P2, longueur: 6.0, cage: false };
    asm.ajouter(cuire(Repere::IDENTITE, &res));
    asm.terminer()
}

/// Vue briques : **moteur à antimatière** — tuyère (silhouette VASIMR gonflée)
/// surmontée du **bloc réacteur/injection** (cuve sombre, bobines EM, tuyauterie)
/// branché sur son écoutille +Z. Poussée le long de −Z.
pub fn demo_moteur_antimatiere_principal() -> EtatStation {
    let mut asm = Assembleur::new();
    let t = 6.0;
    let moteur = Composant::MoteurAntimatiere { profil: Profil::P2, taille: t };
    let mm = Repere::IDENTITE;
    asm.ajouter(cuire(mm, &moteur));
    // Bloc réacteur/injection clipsé sur l'écoutille axiale +Z de la tuyère.
    let reacteur = Composant::ReacteurAntimatiere { profil: Profil::P2, taille: t };
    porter_vers(&mut asm, mm, &moteur, Vec3::Z, &reacteur, 0);
    asm.terminer()
}

// ---------------------------------------------------------------------------
// Grammaire.
// ---------------------------------------------------------------------------

fn module(style: Style, rng: &mut Rng, longueur: f32) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: style.module(rng), longueur }
}


/// Index d'un port libre dont l'avant **monde** vise `dir`. C'est l'équivalent
/// de `porter_vers` côté chantier : viser une direction et non un index, les
/// nœuds basculant à l'accouplement.
fn port_vers(ch: &Chantier, genre: GenrePort, dir: Vec3) -> Option<usize> {
    ch.libres()
        .iter()
        .position(|p| p.genre == genre && p.repere.avant().normalize_or_zero().dot(dir) > 0.85)
}

/// Cœur type ISS : une **épine pressurisée**, modules alignés de part et d'autre
/// d'un nœud central. Le nœud racine est toujours un `Six` — c'est lui qui
/// garantit une face zénith libre pour y arrimer la structure de puissance.
fn coeur_iss(ch: &mut Chantier, rng: &mut Rng, style: Style) {
    ch.racine(Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six });
    for dir in [Vec3::Z, Vec3::NEG_Z] {
        if let Some(i) = port_vers(ch, GenrePort::ModuleAxial, dir) {
            let longueur = rng.entre(2.5, 3.5);
            let m = module(style, rng, longueur);
            ch.poser(i, m, 1);
        }
    }
}

/// Cœur type Mir : une **grappe radiale**, modules rayonnant d'un nœud unique.
/// Les faces ±Y restent libres pour la structure de puissance.
fn coeur_mir(ch: &mut Chantier, rng: &mut Rng, style: Style) {
    ch.racine(Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six });
    for (dir, genre) in [
        (Vec3::Z, GenrePort::ModuleAxial),
        (Vec3::NEG_Z, GenrePort::ModuleAxial),
        (Vec3::X, GenrePort::ModuleRadial),
        (Vec3::NEG_X, GenrePort::ModuleRadial),
    ] {
        if let Some(i) = port_vers(ch, genre, dir) {
            let longueur = rng.entre(2.5, 3.5);
            let m = module(style, rng, longueur);
            ch.poser(i, m, 1);
        }
    }
}

/// Greffe **l'unique structure de puissance** : un boom court qui l'écarte du
/// segment habité, une jonction en T, puis deux demi-poutres qui porteront
/// *tous* les panneaux solaires.
///
/// Elle est **universelle** : quelle que soit la forme du cœur, une station a
/// une barre de puissance et une seule. C'est la règle que décrit l'ISS — une
/// poutre porteuse d'un côté, le segment habité perpendiculaire de l'autre — et
/// c'est aussi ce qui évite les armatures éparpillées. On préfère la face
/// zénith, mais on se rabat sur n'importe quelle face libre plutôt que de
/// livrer une station sans structure.
fn greffer_structure_puissance(ch: &mut Chantier, rng: &mut Rng, c: u8) -> bool {
    let ancre = port_vers(ch, GenrePort::ModuleRadial, Vec3::Y)
        .or_else(|| port_vers(ch, GenrePort::ModuleRadial, Vec3::NEG_Y))
        .or_else(|| ch.libres().iter().position(|p| p.genre == GenrePort::ModuleRadial));
    let Some(i) = ancre else {
        return false;
    };
    // **Le boom doit être long.** C'est lui qui sépare franchement la poutre du
    // segment habité : trop court, la structure se retrouve noyée au milieu de
    // la grappe au lieu de former une barre distincte. Il grandit avec la
    // station, sinon la séparation se perd quand la grappe s'étoffe.
    let boom = Composant::Treillis {
        profil: Profil::P1,
        longueur: 4.5 + c as f32 * 1.8,
        style: StyleTreillis::Carre,
    };
    if !ch.poser(i, boom, 0) {
        return false;
    }
    let st = rng.choix(&[StyleTreillis::Carre, StyleTreillis::Triangulaire]);
    let Some(j) = port_le_plus_haut(ch, GenrePort::ModuleAxial, Profil::P1) else {
        return false;
    };
    if c < 4 {
        let long = 8.0 + c as f32 * 3.5;
        return poser_jonction_et_bras(ch, j, long, st) > 0;
    }

    // **Complexité 4 : structure en H.** Une tête en croix au sommet du boom,
    // une traverse de chaque côté, une barre complète au bout de chaque
    // traverse : deux barres parallèles, de quoi alimenter et refroidir une
    // grappe deux fois plus grosse. La traverse est dimensionnée pour que les
    // deux barres passent l'anti-collision (sphères englobantes des treillis) :
    // avec des demi-poutres de 12, il faut ≥ ~11 d'écartement.
    let tete = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
    if !ch.poser(j, tete, 4) {
        // Repli : la barre simple vaut mieux qu'aucune structure.
        let long = 8.0 + c as f32 * 3.5;
        return poser_jonction_et_bras(ch, j, long, st) > 0;
    }
    let tete_idx = ch.nb_pieces() - 1;
    let mut poses = 0;
    for _ in 0..2 {
        // Monté par sa face radiale +Y, la tête garde ses deux écoutilles
        // axiales **horizontales** : ce sont les départs de traverse.
        let Some(k) = ch
            .libres()
            .iter()
            .position(|p| p.origine == tete_idx && p.genre == GenrePort::ModuleAxial)
        else {
            break;
        };
        let traverse = Composant::Treillis { profil: Profil::P1, longueur: 5.5, style: st };
        if !ch.poser(k, traverse, 0) {
            continue;
        }
        let tr_idx = ch.nb_pieces() - 1;
        let Some(b) = ch
            .libres()
            .iter()
            .position(|p| p.origine == tr_idx && p.genre == GenrePort::ModuleAxial)
        else {
            continue;
        };
        poses += poser_jonction_et_bras(ch, b, 12.0, st);
    }
    poses > 0
}

/// Pose la jonction en **T** sur le port `port_idx` (par sa tige), puis un bras
/// de barre de chaque côté : adaptateur P1→P2 et demi-poutre. Renvoie le nombre
/// de demi-poutres posées.
///
/// Toutes les sélections se font par **`origine`** (la pièce qui expose le
/// port) : les index de `libres` se décalent à chaque pose (`swap_remove`), et
/// avec deux jonctions à la même hauteur (structure en H), « le port le plus
/// haut » ne désigne plus rien.
fn poser_jonction_et_bras(
    ch: &mut Chantier,
    port_idx: usize,
    long: f32,
    st: StyleTreillis,
) -> usize {
    // Une jonction en T : un nœud à six sorties laisserait quatre faces vides.
    let jonction = Composant::Noeud { profil: Profil::P1, sorties: Sorties::T };
    if !ch.poser(port_idx, jonction, 2) {
        return 0;
    }
    let j_idx = ch.nb_pieces() - 1;
    let mut poses = 0;
    for _ in 0..2 {
        let Some(bras) = ch
            .libres()
            .iter()
            .position(|p| p.origine == j_idx && p.genre == GenrePort::ModuleRadial)
        else {
            break;
        };
        // **Passage P1 → P2.** Le chantier vérifie les profils : une poutre P2
        // ne se visse pas sur un bras P1, elle serait refusée en silence et le
        // bras finirait garni de modules. L'adaptateur est fait pour ça.
        let marche = Composant::Adaptateur { grand: Profil::P2, petit: Profil::P1, longueur: 1.0 };
        if !ch.poser(bras, marche, 1) {
            continue;
        }
        let ad_idx = ch.nb_pieces() - 1;
        let Some(gros) = ch
            .libres()
            .iter()
            .position(|p| p.origine == ad_idx && p.genre == GenrePort::ModuleAxial && p.profil == Profil::P2)
        else {
            continue;
        };
        if ch.poser(gros, Composant::Treillis { profil: Profil::P2, longueur: long, style: st }, 0) {
            poses += 1;
        }
    }
    poses
}

/// Index d'un port libre repéré par sa **position monde** : les index se
/// décalent à chaque pose, on ne peut donc pas les mémoriser d'une itération
/// sur l'autre.
fn index_port(ch: &Chantier, genre: GenrePort, pos: Vec3) -> Option<usize> {
    ch.libres()
        .iter()
        .position(|p| p.genre == genre && p.repere.pos.distance(pos) < 1e-3)
}

/// Port libre le plus **élevé** du genre et du profil voulus. La structure de
/// puissance étant dressée en haut du boom, la hauteur est ce qui distingue ses
/// faces de celles, homonymes, du cœur. On ne peut pas se fier à l'ordre
/// d'insertion : `Chantier` consomme ses ports par `swap_remove`, qui réordonne
/// la liste.
fn port_le_plus_haut(ch: &Chantier, genre: GenrePort, profil: Profil) -> Option<usize> {
    ch.libres()
        .iter()
        .enumerate()
        .filter(|(_, p)| p.genre == genre && p.profil == profil)
        .max_by(|a, b| a.1.repere.pos.y.total_cmp(&b.1.repere.pos.y))
        .map(|(i, _)| i)
}

/// Le corridor vers la structure de puissance est **réservé**. Sans cela, les
/// modules poussent aussi vers le zénith et viennent envelopper le boom : la
/// poutre cesse d'être une barre nettement séparée pour devenir un élément noyé
/// au milieu de la grappe.
fn corridor_libre(avant: Vec3) -> bool {
    avant.y < 0.5
}

/// Le port appartient-il au segment **pressurisé** (module ou nœud) ? On ne
/// coiffe pas le bout nu d'une poutre d'un nez de docking.
fn sur_pressurise(ch: &Chantier, origine: usize) -> bool {
    matches!(
        ch.piece(origine).map(|p| p.composant.clone()),
        Some(Composant::ModuleAxial { .. }) | Some(Composant::Noeud { .. })
    )
}

/// Une passe de croissance du **segment pressurisé**. Deux règles tirées de
/// l'observation des stations réelles :
///
/// - **aucun treillis n'est posé ici.** Toute la structure porteuse — et donc
///   tous les panneaux solaires — appartient à l'ossature. On obtient ainsi
///   *une* barre de puissance et un segment habité perpendiculaire, au lieu
///   d'armatures éparpillées un peu partout ;
/// - **les modules restent d'un seul tenant**, greffés sur les faces libres du
///   cluster existant ; et un nœud n'est posé que s'il **dessert vraiment**
///   quelque chose, des modules lui étant accrochés dans la foulée.
fn brancher(
    ch: &mut Chantier,
    rng: &mut Rng,
    style: Style,
    ramifier: bool,
    plancher: f32,
    quota: usize,
) {
    if ch.budget_restant() <= plancher {
        return;
    }
    // Croissance radiale : le cluster s'étoffe autour de ses nœuds, par
    // **paires de faces opposées** — un seul tirage (chance, variante,
    // longueur) pour les deux ports ±axe d'une même pièce. Tirer chaque face
    // indépendamment donnait une grappe visiblement bancale.
    let mut paires: Vec<((usize, u8), Vec<Vec3>)> = Vec::new();
    for p in ch.libres() {
        if p.genre != GenrePort::ModuleRadial
            || !sur_pressurise(ch, p.origine)
            || !corridor_libre(p.repere.avant())
            // Les nœuds de la structure (tête du H, jonctions) comptent comme
            // pressurisés mais vivent en altitude : la grappe n'y pousse pas.
            || p.repere.pos.y > 2.0
        {
            continue;
        }
        let a = p.repere.avant();
        let axe = if a.x.abs() > 0.7 { 0 } else if a.y.abs() > 0.7 { 1 } else { 2 };
        let cle = (p.origine, axe);
        match paires.iter_mut().find(|(k, _)| *k == cle) {
            Some((_, v)) => v.push(p.repere.pos),
            None => paires.push((cle, vec![p.repere.pos])),
        }
    }
    for (_, positions) in paires {
        if ch.budget_restant() <= plancher || compte_modules(ch) >= quota {
            return;
        }
        if !rng.chance(0.7) {
            continue;
        }
        let longueur = rng.entre(2.0, 3.2);
        let m = module(style, rng, longueur);
        for pos in positions {
            if let Some(i) = index_port(ch, GenrePort::ModuleRadial, pos) {
                ch.poser(i, m.clone(), 1);
            }
        }
    }

    // Prolongement axial : on allonge les chaînes ; un nœud n'apparaît que pour
    // ramifier, et on lui accroche aussitôt de quoi justifier sa présence. Le
    // corridor s'applique ici aussi : les chaînes issues d'un nœud basculé
    // peuvent pointer vers le zénith et venir toucher la poutre.
    let axiaux: Vec<Vec3> = ch
        .libres()
        .iter()
        .filter(|p| {
            p.genre == GenrePort::ModuleAxial
                && sur_pressurise(ch, p.origine)
                && corridor_libre(p.repere.avant())
                && p.repere.pos.y < 2.0
        })
        .map(|p| p.repere.pos)
        .collect();
    for pos in axiaux {
        if ch.budget_restant() <= plancher || compte_modules(ch) >= quota {
            return;
        }
        let Some(i) = index_port(ch, GenrePort::ModuleAxial, pos) else {
            continue;
        };
        if ramifier && rng.chance(0.35) {
            let nd = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
            if ch.poser(i, nd, 1) {
                // On ramifie en **paire symétrique** : les deux faces ±X du
                // nœud qu'on vient de poser, avec le même module de part et
                // d'autre. (Corridor : les faces vers le zénith sont exclues —
                // c'était le trou par lequel les modules remontaient jusqu'à
                // la poutre.)
                let org = ch.nb_pieces() - 1;
                let bras: Vec<Vec3> = ch
                    .libres()
                    .iter()
                    .filter(|p| {
                        p.origine == org
                            && p.genre == GenrePort::ModuleRadial
                            && corridor_libre(p.repere.avant())
                            && p.repere.avant().y.abs() < 0.5 // paire horizontale
                    })
                    .map(|p| p.repere.pos)
                    .collect();
                let longueur = rng.entre(2.0, 3.0);
                let m = module(style, rng, longueur);
                for pos in bras.into_iter().take(2) {
                    if let Some(j) = index_port(ch, GenrePort::ModuleRadial, pos) {
                        ch.poser(j, m.clone(), 1);
                    }
                }
            }
        } else if rng.chance(0.5) {
            let longueur = rng.entre(2.0, 3.2);
            let m = module(style, rng, longueur);
            ch.poser(i, m, 1);
        }
    }
}

/// Coiffe les bouts axiaux libres du segment pressurisé d'un **nez de docking**,
/// et y amarre parfois un vaisseau. Une extrémité de chaîne doit servir à
/// quelque chose : un port d'amarrage est une fin légitime, un nœud à six
/// sorties qui ne dessert rien ne l'est pas.
fn terminer_extremites(ch: &mut Chantier, rng: &mut Rng) {
    let bouts: Vec<Vec3> = ch
        .libres()
        .iter()
        .filter(|p| {
            p.genre == GenrePort::ModuleAxial
                && sur_pressurise(ch, p.origine)
                // Pas de tour nez + cargo dressée vers la poutre : le corridor
                // vaut aussi pour les terminaisons. Et rien sur les nœuds de
                // la structure (tête du H), qui vivent en altitude.
                && corridor_libre(p.repere.avant())
                && p.repere.pos.y < 2.0
        })
        .map(|p| p.repere.pos)
        .collect();
    for pos in bouts {
        let Some(i) = index_port(ch, GenrePort::ModuleAxial, pos) else {
            continue;
        };
        if !ch.poser(i, nez_docking(), 0) {
            continue;
        }
        // Un cargo amarré de temps en temps : ça anime les extrémités.
        if rng.chance(0.35) {
            let libre = ch
                .libres()
                .iter()
                .rposition(|p| p.genre == GenrePort::ModuleAxial && p.profil == Profil::P0);
            if let Some(j) = libre {
                let cargo = Composant::ModuleAxial {
                    profil: Profil::P0,
                    variante: VarianteModule::Dore,
                    longueur: 2.0,
                };
                ch.poser(j, cargo, 0);
            }
        }
    }
}

fn fabrique_appendice(style: Style, rng: &mut Rng, categorie: u8) -> Composant {
    match categorie {
        // Équipements exposés en zone médiane de poutre (type ELC).
        3 => Composant::Caisson {
            profil: Profil::P0,
            variante: rng.choix(&[
                VarianteCaisson::Ossature,
                VarianteCaisson::Ferme,
                VarianteCaisson::Rack,
            ]),
            longueur: rng.entre(1.3, 1.9),
            largeur: 1.0,
        },
        1 => Composant::Radiateur {
            profil: Profil::P0,
            variante: style.radiateur(rng),
            longueur: rng.entre(2.0, 3.0),
            largeur: 1.2,
        },
        2 => Composant::Antenne {
            profil: Profil::P0,
            variante: style.antenne(rng),
            taille: rng.entre(0.9, 1.4),
        },
        // Arrays : ils vivent désormais sur la poutre de puissance, donc à
        // l'échelle de la station et non à celle d'un module. Les petits
        // panneaux de 2,5 faisaient chétif au bout d'une barre de 15.
        _ => Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: style.panneau(rng),
            longueur: rng.entre(5.0, 6.8),
            largeur: 1.9,
        },
    }
}

/// Clé de regroupement d'un port `Surface` : `(axe, perp1, perp2, catégorie)`.
/// L'axe de sortie décide de la **catégorie** — panneaux sur ±X (ailes
/// horizontales, jamais 4 côtés), radiateurs sur ±Y, antennes sur ±Z — et les
/// coordonnées perpendiculaires regroupent les paires opposées (même clé) pour
/// un rendu symétrique.
fn cle_surface(pos: Vec3, avant: Vec3) -> (u8, i64, i64, u8) {
    let q = |v: f32| (v * 4.0).round() as i64;
    if avant.x.abs() > 0.9 {
        // ±X : **jamais de panneau** hors poutre — tous les arrays vivent sur
        // la structure de puissance (les grands panneaux plaqués aux modules
        // étaient le grief n°5). Radiateurs, antennes et **caissons techniques**
        // (racks, avionique) alternent par bande.
        let bande = q(pos.z);
        let cat = match bande.rem_euclid(3) {
            0 => 1,
            1 => 2,
            _ => 3,
        };
        (0, q(pos.y), bande, cat)
    } else if avant.y.abs() > 0.9 {
        (1, q(pos.x), q(pos.z), 1) // ±Y → radiateur
    } else {
        (2, q(pos.x), q(pos.y), 2) // ±Z → antenne
    }
}

/// Habille les ports hôtes `Surface`, sous **deux régimes** :
///
/// - sur une **poutre**, la catégorie vient de l'éloignement du centre de la
///   station : arrays aux extrémités, radiateurs en pied, équipements entre les
///   deux. C'est le zonage observé sur l'ISS, et il est indépendant de
///   l'orientation de la poutre ;
/// - ailleurs, elle vient de l'axe de sortie (jamais de panneaux sur les quatre
///   côtés d'un même corps, ils se gêneraient).
///
/// Dans les deux cas, un **seul** appendice est tiré par groupe symétrique.
/// `poutres` sélectionne la passe : `true` pour n'habiller que les structures
/// porteuses (appelée juste après leur greffe, tant que le budget est intact),
/// `false` pour tout le reste, en fin de génération.
fn habiller_surface(
    ch: &mut Chantier,
    rng: &mut Rng,
    style: Style,
    poutres: bool,
    quotas: Option<(usize, usize, usize)>,
) {
    let ports: Vec<(Vec3, Vec3, usize)> = ch
        .libres()
        .iter()
        .filter(|p| {
            if p.genre != GenrePort::Surface {
                return false;
            }
            // Une « poutre » est un treillis **P2** : le boom (P1) reste nu,
            // comme sur l'ISS — l'habiller le faisait ressembler à un mât de
            // sapin de Noël au ras des modules. Exclu des deux passes, ses
            // ports ne sont jamais garnis.
            match ch.piece(p.origine).map(|q| q.composant.clone()) {
                Some(Composant::Treillis { profil, .. }) => poutres && profil == Profil::P2,
                // La jonction en T (sommet du boom) expose aussi des faces
                // `Surface` — un radiateur qui y pousse ruine la barre nette.
                // Le générateur ne pose de `T` qu'à cet endroit : on l'exclut.
                Some(Composant::Noeud { sorties: Sorties::T, .. }) => false,
                _ => !poutres,
            }
        })
        .map(|p| (p.repere.pos, p.repere.avant(), p.origine))
        .collect();

    // Étendue radiale des ports de chaque poutre : elle sert de règle graduée
    // pour le zonage (min = pied, max = extrémité).
    let mut bornes: Vec<(usize, f32, f32)> = Vec::new();
    for (pos, _, origine) in &ports {
        if !matches!(ch.piece(*origine).map(|p| p.composant.clone()), Some(Composant::Treillis { .. })) {
            continue;
        }
        let d = pos.length();
        match bornes.iter_mut().find(|(o, _, _)| o == origine) {
            Some(b) => {
                b.1 = b.1.min(d);
                b.2 = b.2.max(d);
            }
            None => bornes.push((*origine, d, d)),
        }
    }

    let cle = |pos: Vec3, avant: Vec3, origine: usize| -> (u8, i64, i64, u8) {
        let q = |v: f32| (v * 4.0).round() as i64;
        match bornes.iter().find(|(o, _, _)| *o == origine) {
            Some((_, lo, hi)) => {
                let d = pos.length();
                let t = if hi - lo < 1e-3 { 0.5 } else { (d - lo) / (hi - lo) };
                // 0 = panneau, 1 = radiateur, 3 = caisson d'équipement. Les
                // arrays occupent la **moitié externe** : à 0.62 il ne restait
                // qu'une paire par demi-poutre, contre deux sur l'ISS.
                let cat = if t > 0.5 {
                    0
                } else if t < 0.3 {
                    1
                } else {
                    3
                };
                // Groupe : même bande d'éloignement, **toutes poutres
                // confondues** — les deux demi-poutres sont le miroir l'une de
                // l'autre, leurs ports tombent aux mêmes distances. Grouper par
                // origine donnait un tirage indépendant par tronçon, donc une
                // barre visiblement asymétrique.
                (3, 0, q(d), cat)
            }
            None => cle_surface(pos, avant),
        }
    };

    let mut cles: Vec<(u8, i64, i64, u8)> = ports.iter().map(|(p, a, o)| cle(*p, *a, *o)).collect();
    cles.sort_unstable();
    cles.dedup();
    // Les **arrays d'abord** : posés en dernier (tri par distance), ils se
    // faisaient parfois refuser par l'anti-collision au profit des caissons et
    // radiateurs déjà en place — et la station se retrouvait sous-motorisée.
    cles.sort_by_key(|k| (k.0, k.3, k.1, k.2));

    // Hors poutre, les poses sont **contingentées** par les quotas (le reste
    // des faces demeure nu — les stations réelles ont beaucoup de coque vide).
    // Un groupe n'est servi que s'il tient en entier dans le quota, pour ne
    // jamais casser une paire symétrique.
    let (mut rad_restant, mut ant_restant, mut tech_restant) =
        quotas.unwrap_or((usize::MAX, usize::MAX, usize::MAX));
    for k in cles {
        let taille = ports.iter().filter(|(p, a, o)| cle(*p, *a, *o) == k).count();
        match k.3 {
            1 if rad_restant < taille => continue,
            2 if ant_restant < taille => continue,
            3 if tech_restant < taille => continue,
            1 => rad_restant -= taille,
            2 => ant_restant -= taille,
            3 => tech_restant = tech_restant.saturating_sub(taille),
            _ => {}
        }
        let app = fabrique_appendice(style, rng, k.3); // un seul pour tout le groupe
        for (pos, avant, _origine) in ports.iter().filter(|(p, a, o)| cle(*p, *a, *o) == k) {
            if let Some(i) = ch.libres().iter().position(|q| {
                q.genre == GenrePort::Surface
                    && q.repere.pos.distance(*pos) < 1e-3
                    && (q.repere.avant() - *avant).length() < 1e-3
            }) {
                ch.poser(i, app.clone(), 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nb(etat: &EtatStation) -> usize {
        etat.doit_dessiner().map(|s| s.nb_pieces()).unwrap_or(0)
    }

    #[test]
    fn generer_produit_une_station() {
        let etat = generer(&ParamsStation { graine: 1, complexite: 2, style: Style::Historique, ossature: None });
        assert!(matches!(etat, EtatStation::Prete(_)));
        assert!(nb(&etat) >= 3, "au moins une ossature garnie");
    }

    #[test]
    fn generer_est_deterministe() {
        let p = ParamsStation { graine: 42, complexite: 3, style: Style::Russe, ossature: None };
        assert_eq!(nb(&generer(&p)), nb(&generer(&p)));
    }

    #[test]
    fn complexite_influe_sur_le_nombre_de_pieces() {
        // Sur une même graine/style, une station complexe a plus de pièces.
        let petite = nb(&generer(&ParamsStation { graine: 7, complexite: 1, style: Style::Futuriste, ossature: None }));
        let grande = nb(&generer(&ParamsStation { graine: 7, complexite: 4, style: Style::Futuriste, ossature: None }));
        assert!(grande >= petite);
    }

    #[test]
    fn ossature_forcee_est_respectee_et_finie() {
        for oss in [Ossature::Iss, Ossature::Mir] {
            for g in 0..10u64 {
                let etat = generer(&ParamsStation { graine: g, complexite: 3, style: Style::Historique, ossature: Some(oss) });
                assert!(matches!(etat, EtatStation::Prete(_)), "{oss:?} graine {g}");
            }
        }
    }

    #[test]
    fn presets_iss_et_mir_produisent_des_stations() {
        assert!(nb(&preset_iss()) >= 5);
        assert!(nb(&preset_mir()) >= 4);
    }

    #[test]
    fn toutes_graines_donnent_une_station_finie() {
        for g in 0..30u64 {
            for style in Style::TOUS {
                let etat = generer(&ParamsStation { graine: g, complexite: 3, style, ossature: None });
                assert!(matches!(etat, EtatStation::Prete(_)), "graine {g} {style:?}");
            }
        }
    }

    /// La silhouette cible (`docs/suivi/stations.md` Partie A) : **une** barre
    /// de puissance en deux tronçons opposés, panneaux dessus, boom nu, grappe
    /// habitée entièrement sous la barre. Vérifiée sur un lot de graines — c'est
    /// le filet contre les échecs silencieux de `poser` (§2 du doc), qui
    /// laissaient une station plausible mais fausse.
    #[test]
    fn silhouette_generee_converge() {
        for oss in [Ossature::Iss, Ossature::Mir] {
            for c in [2u8, 3, 4] {
                for g in 0..12u64 {
                    let p = ParamsStation { graine: g, complexite: c, style: Style::Historique, ossature: Some(oss) };
                    let etat = generer(&p);
                    let st = etat.doit_dessiner().expect("station finie");
                    let ctx = format!("{oss:?} c={c} graine {g}");

                    // Tronçons de poutre P2 : une barre (2) jusqu'à c=3, une
                    // structure en **H** (2 barres = 4 tronçons) à c=4.
                    let poutres: Vec<Vec3> = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::Treillis { profil: Profil::P2, .. }))
                        .map(|p| p.centre())
                        .collect();
                    if c < 4 {
                        assert_eq!(poutres.len(), 2, "{ctx}: deux demi-poutres");
                        let long = 8.0 + c as f32 * 3.5;
                        assert!(
                            poutres[0].distance(poutres[1]) > long,
                            "{ctx}: demi-poutres opposées, pas bout à bout"
                        );
                    } else {
                        assert_eq!(poutres.len(), 4, "{ctx}: H = quatre demi-poutres");
                        // Deux tronçons de chaque côté de chacun des deux axes :
                        // c'est ce qui fait le H (deux barres parallèles).
                        assert_eq!(poutres.iter().filter(|p| p.x > 0.0).count(), 2, "{ctx}: H, côté +X");
                        assert_eq!(poutres.iter().filter(|p| p.z > 0.0).count(), 2, "{ctx}: H, côté +Z");
                    }
                    let y_poutre = poutres.iter().map(|p| p.y).sum::<f32>() / poutres.len() as f32;

                    // La grappe pressurisée reste **sous** la barre (corridor).
                    for p in st.pieces() {
                        if let Composant::ModuleAxial { profil: Profil::P1, .. } = p.composant {
                            assert!(
                                p.centre().y < y_poutre - 2.0,
                                "{ctx}: module à y={:.1} au ras de la poutre (y={y_poutre:.1})",
                                p.centre().y
                            );
                        }
                    }

                    // Tous les grands panneaux vivent sur la poutre, aux bras.
                    let panneaux: Vec<Vec3> = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::PanneauSolaire { longueur, .. } if longueur > 4.5))
                        .map(|p| p.centre())
                        .collect();
                    assert!(panneaux.len() >= 4, "{ctx}: au moins deux paires d'arrays ({} trouvés)", panneaux.len());
                    for p in &panneaux {
                        assert!(
                            (p.y - y_poutre).abs() < 4.0,
                            "{ctx}: array à y={:.1} loin de la poutre (y={y_poutre:.1})",
                            p.y
                        );
                    }

                    // **Proportions façon ISS** : l'habitat suit la puissance
                    // (1 à 2,5 modules par aile), le refroidissement et la
                    // propulsion suivent l'habitat.
                    let modules = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::ModuleAxial { profil: Profil::P1, .. }))
                        .count();
                    let rads = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::Radiateur { .. }))
                        .count();
                    let props = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::Propulseur { .. }))
                        .count();
                    let ailes = panneaux.len() as f32;
                    assert!(
                        (modules as f32) >= ailes * 0.9,
                        "{ctx}: habitat sous-dimensionné ({modules} modules pour {ailes} ailes)"
                    );
                    assert!(
                        (modules as f32) <= ailes * 2.5 + 2.0,
                        "{ctx}: habitat surdimensionné ({modules} modules pour {ailes} ailes)"
                    );
                    assert!(
                        (rads as f32) >= modules as f32 * 0.4 && (rads as f32) <= modules as f32 * 1.2 + 2.0,
                        "{ctx}: refroidissement disproportionné ({rads} radiateurs pour {modules} modules)"
                    );
                    assert!(props >= 1, "{ctx}: aucune propulsion");

                    // **Symétrie miroir de la barre** : tout appendice posé sur
                    // la structure de puissance a son jumeau de l'autre côté de
                    // la jonction (même type, position réfléchie sur x).
                    let sur_barre: Vec<(u8, Vec3)> = st
                        .pieces()
                        .iter()
                        .filter_map(|p| {
                            let cat = match p.composant {
                                Composant::PanneauSolaire { .. } => 0u8,
                                Composant::Radiateur { .. } => 1,
                                Composant::Antenne { .. } => 2,
                                Composant::Caisson { .. } => 3,
                                _ => return None,
                            };
                            let cp = p.centre();
                            ((cp.y - y_poutre).abs() < 4.0 && cp.x.abs() > 2.0)
                                .then_some((cat, cp))
                        })
                        .collect();
                    for (cat, cp) in &sur_barre {
                        let miroir = vec3(-cp.x, cp.y, cp.z);
                        assert!(
                            sur_barre.iter().any(|(c2, p2)| c2 == cat && p2.distance(miroir) < 0.8),
                            "{ctx}: appendice cat {cat} en ({:.1},{:.1},{:.1}) sans jumeau miroir",
                            cp.x, cp.y, cp.z
                        );
                    }

                    // Le boom et la jonction restent nus : aucun appendice dans
                    // le corridor central.
                    for p in st.pieces() {
                        let est_app = matches!(
                            p.composant,
                            Composant::PanneauSolaire { .. }
                                | Composant::Radiateur { .. }
                                | Composant::Antenne { .. }
                                | Composant::Caisson { .. }
                        );
                        let cpos = p.centre();
                        assert!(
                            !(est_app && cpos.x.abs() < 1.2 && cpos.z.abs() < 1.2 && cpos.y > 1.5),
                            "{ctx}: appendice sur le boom en ({:.1},{:.1},{:.1})",
                            cpos.x, cpos.y, cpos.z
                        );
                    }
                }
            }
        }
    }
}
