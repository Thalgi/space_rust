use crate::vaisseau::chantier::Chantier;
use crate::vaisseau::{
    Composant, EtatStation, GenrePort, Profil, Sorties, StyleTreillis,
    VarianteAntenne, VarianteCaisson, VarianteModule,
    VariantePanneau, VariantePropulseur, VarianteRadiateur,
};
use macroquad::prelude::*;

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

    pub(crate) fn suivant(&mut self) -> u64 {
        self.etat = self.etat.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.etat;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Flottant dans [0, 1).
    pub(crate) fn unite(&mut self) -> f32 {
        (self.suivant() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub(crate) fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + self.unite() * (b - a)
    }

    pub(crate) fn chance(&mut self, p: f32) -> bool {
        self.unite() < p
    }

    pub(crate) fn choix<T: Copy>(&mut self, v: &[T]) -> T {
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
pub(crate) const MODULES_PAR_ARRAY: f32 = 1.6;
/// Radiateurs par module pressurisé (preset : 10/13 ≈ 0,75 ; réel : 14/16).
pub(crate) const RADIATEURS_PAR_MODULE: f32 = 0.75;

pub(crate) fn compte(ch: &Chantier, f: impl Fn(&Composant) -> bool) -> usize {
    ch.pieces().filter(|(_, p)| f(&p.composant)).count()
}

/// Grandes ailes (celles de la poutre — les petits panneaux ne comptent pas).
pub(crate) fn compte_arrays(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::PanneauSolaire { longueur, .. } if *longueur > 4.5))
}

pub(crate) fn compte_modules(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::ModuleAxial { profil: Profil::P1, .. }))
}

pub(crate) fn compte_radiateurs(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::Radiateur { .. }))
}

pub(crate) fn compte_antennes(ch: &Chantier) -> usize {
    compte(ch, |c| matches!(c, Composant::Antenne { .. }))
}

/// Propulsion principale au bout **le plus en aval** du segment pressurisé
/// (l'ISS a ses moteurs à l'arrière de Zvezda). Posée avant les terminaisons,
/// pour que ce bout reçoive un moteur et non un nez de docking.
pub(crate) fn greffer_propulsion(ch: &mut Chantier, rng: &mut Rng, style: Style) {
    use VariantePropulseur::*;
    let bout = ch
        .libres()
        .iter()
        .filter(|p| {
            p.genre == GenrePort::ModuleAxial
                && p.profil == Profil::P1
                && sur_pressurise(ch, p.origine)
                && p.repere.avant().z.abs() > 0.9
        })
        .max_by(|a, b| a.repere.pos.z.abs().total_cmp(&b.repere.pos.z.abs()))
        .map(|p| p.id);
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

/// Nez de docking (PMA/IDA, port Soyouz, port Shenzhou…).
pub(crate) fn nez_docking() -> Composant {
    Composant::Adaptateur { grand: Profil::P1, petit: Profil::P0, longueur: 1.2 }
}

// ---------------------------------------------------------------------------
// Grammaire.
// ---------------------------------------------------------------------------

pub(crate) fn module(style: Style, rng: &mut Rng, longueur: f32) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: style.module(rng), longueur }
}


/// Identifiant d'un port libre dont l'avant **monde** vise `dir`. C'est
/// l'équivalent de `porter_vers` côté chantier : viser une direction et non un
/// indice, les nœuds basculant à l'accouplement.
pub(crate) fn port_vers(ch: &Chantier, genre: GenrePort, dir: Vec3) -> Option<u64> {
    ch.libres()
        .iter()
        .find(|p| p.genre == genre && p.repere.avant().normalize_or_zero().dot(dir) > 0.85)
        .map(|p| p.id)
}

/// Cœur type ISS : une **épine pressurisée**, modules alignés de part et d'autre
/// d'un nœud central. Le nœud racine est toujours un `Six` — c'est lui qui
/// garantit une face zénith libre pour y arrimer la structure de puissance.
pub(crate) fn coeur_iss(ch: &mut Chantier, rng: &mut Rng, style: Style) {
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
pub(crate) fn coeur_mir(ch: &mut Chantier, rng: &mut Rng, style: Style) {
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
pub(crate) fn greffer_structure_puissance(ch: &mut Chantier, rng: &mut Rng, c: u8) -> bool {
    let ancre = port_vers(ch, GenrePort::ModuleRadial, Vec3::Y)
        .or_else(|| port_vers(ch, GenrePort::ModuleRadial, Vec3::NEG_Y))
        .or_else(|| ch.libres().iter().find(|p| p.genre == GenrePort::ModuleRadial).map(|p| p.id));
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
    let tete_idx = ch.derniere_piece().unwrap();
    let mut poses = 0;
    for _ in 0..2 {
        // Monté par sa face radiale +Y, la tête garde ses deux écoutilles
        // axiales **horizontales** : ce sont les départs de traverse.
        let Some(k) = ch
            .libres()
            .iter()
            .find(|p| p.origine == tete_idx && p.genre == GenrePort::ModuleAxial)
            .map(|p| p.id)
        else {
            break;
        };
        let traverse = Composant::Treillis { profil: Profil::P1, longueur: 5.5, style: st };
        if !ch.poser(k, traverse, 0) {
            continue;
        }
        let tr_idx = ch.derniere_piece().unwrap();
        let Some(b) = ch
            .libres()
            .iter()
            .find(|p| p.origine == tr_idx && p.genre == GenrePort::ModuleAxial)
            .map(|p| p.id)
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
/// port), pas par un `id` mémorisé d'une itération sur l'autre : avec deux
/// jonctions à la même hauteur (structure en H), « le port le plus haut » ne
/// désignerait plus la bonne une fois la première jonction posée.
pub(crate) fn poser_jonction_et_bras(
    ch: &mut Chantier,
    port_id: u64,
    long: f32,
    st: StyleTreillis,
) -> usize {
    // Une jonction en T : un nœud à six sorties laisserait quatre faces vides.
    let jonction = Composant::Noeud { profil: Profil::P1, sorties: Sorties::T };
    if !ch.poser(port_id, jonction, 2) {
        return 0;
    }
    let j_idx = ch.derniere_piece().unwrap();
    let mut poses = 0;
    for _ in 0..2 {
        let Some(bras) = ch
            .libres()
            .iter()
            .find(|p| p.origine == j_idx && p.genre == GenrePort::ModuleRadial)
            .map(|p| p.id)
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
        let ad_idx = ch.derniere_piece().unwrap();
        let Some(gros) = ch
            .libres()
            .iter()
            .find(|p| p.origine == ad_idx && p.genre == GenrePort::ModuleAxial && p.profil == Profil::P2)
            .map(|p| p.id)
        else {
            continue;
        };
        if ch.poser(gros, Composant::Treillis { profil: Profil::P2, longueur: long, style: st }, 0) {
            poses += 1;
        }
    }
    poses
}

/// Identifiant d'un port libre repéré par sa **position monde** : la position
/// dans `libres()` se décale à chaque pose, on ne peut donc pas la mémoriser
/// d'une itération sur l'autre — mais un point de l'espace, lui, ne bouge pas.
pub(crate) fn index_port(ch: &Chantier, genre: GenrePort, pos: Vec3) -> Option<u64> {
    ch.libres()
        .iter()
        .find(|p| p.genre == genre && p.repere.pos.distance(pos) < 1e-3)
        .map(|p| p.id)
}

/// Port libre le plus **élevé** du genre et du profil voulus. La structure de
/// puissance étant dressée en haut du boom, la hauteur est ce qui distingue ses
/// faces de celles, homonymes, du cœur. On ne peut pas se fier à l'ordre
/// d'insertion : `Chantier` consomme ses ports par `swap_remove`, qui réordonne
/// la liste.
pub(crate) fn port_le_plus_haut(ch: &Chantier, genre: GenrePort, profil: Profil) -> Option<u64> {
    ch.libres()
        .iter()
        .filter(|p| p.genre == genre && p.profil == profil)
        .max_by(|a, b| a.repere.pos.y.total_cmp(&b.repere.pos.y))
        .map(|p| p.id)
}

/// Le corridor vers la structure de puissance est **réservé**. Sans cela, les
/// modules poussent aussi vers le zénith et viennent envelopper le boom : la
/// poutre cesse d'être une barre nettement séparée pour devenir un élément noyé
/// au milieu de la grappe.
pub(crate) fn corridor_libre(avant: Vec3) -> bool {
    avant.y < 0.5
}

/// Le port appartient-il au segment **pressurisé** (module ou nœud) ? On ne
/// coiffe pas le bout nu d'une poutre d'un nez de docking.
pub(crate) fn sur_pressurise(ch: &Chantier, origine: u64) -> bool {
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
pub(crate) fn brancher(
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
    let mut paires: Vec<((u64, u8), Vec<Vec3>)> = Vec::new();
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
                let org = ch.derniere_piece().unwrap();
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
pub(crate) fn terminer_extremites(ch: &mut Chantier, rng: &mut Rng) {
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
                .rev()
                .find(|p| p.genre == GenrePort::ModuleAxial && p.profil == Profil::P0)
                .map(|p| p.id);
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

pub(crate) fn fabrique_appendice(style: Style, rng: &mut Rng, categorie: u8) -> Composant {
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
pub(crate) fn cle_surface(pos: Vec3, avant: Vec3) -> (u8, i64, i64, u8) {
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
        // ⚠️ `|x|`, pas `x` : les deux faces d'une même bande sont **le miroir
        // l'une de l'autre**, et les clefs signées les mettaient dans deux
        // groupes distincts. Chacune était alors servie — ou refusée par
        // l'anti-collision — indépendamment, d'où des stations à radiateur
        // unique d'un seul bord. Le défaut était **latent** : tant que les
        // englobants étaient sphériques, les deux côtés étaient refusés
        // ensemble par symétrie, et rien ne se voyait.
        (1, q(pos.x.abs()), q(pos.z), 1) // ±Y → radiateur
    } else {
        (2, q(pos.x.abs()), q(pos.y), 2) // ±Z → antenne
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
pub(crate) fn habiller_surface(
    ch: &mut Chantier,
    rng: &mut Rng,
    style: Style,
    poutres: bool,
    quotas: Option<(usize, usize, usize)>,
) {
    let ports: Vec<(Vec3, Vec3, u64)> = ch
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
    let mut bornes: Vec<(u64, f32, f32)> = Vec::new();
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

    let cle = |pos: Vec3, avant: Vec3, origine: u64| -> (u8, i64, i64, u8) {
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
        // Le quota est une **proportion visée**, pas un plafond dur, et un
        // groupe symétrique est indivisible : on le sert dès qu'il reste de quoi
        // commencer, quitte à dépasser d'un membre ou deux.
        //
        // ⚠️ La règle précédente (« servir seulement si le groupe tient en
        // entier ») marchait tant qu'un groupe valait un port ou deux. Depuis
        // que les faces ±Y/±Z sont groupées par `|x|` — donc par paire miroir —
        // les groupes ont doublé, et un quota impair laissait le dernier
        // **entièrement** de côté : une station de 16 modules retombait à 6
        // radiateurs au lieu des 8 visés.
        match k.3 {
            1 if rad_restant == 0 => continue,
            2 if ant_restant == 0 => continue,
            3 if tech_restant == 0 => continue,
            1 => rad_restant = rad_restant.saturating_sub(taille),
            2 => ant_restant = ant_restant.saturating_sub(taille),
            3 => tech_restant = tech_restant.saturating_sub(taille),
            _ => {}
        }
        let app = fabrique_appendice(style, rng, k.3); // un seul pour tout le groupe
        let vises: Vec<(Vec3, Vec3)> = ports
            .iter()
            .filter(|(p, a, o)| cle(*p, *a, *o) == k)
            .map(|(p, a, _)| (*p, *a))
            .collect();
        // Le port libre correspondant à une position/orientation visée.
        let trouver = |ch: &Chantier, pos: Vec3, avant: Vec3| {
            ch.libres()
                .iter()
                .find(|q| {
                    q.genre == GenrePort::Surface
                        && q.repere.pos.distance(pos) < 1e-3
                        && (q.repere.avant() - avant).length() < 1e-3
                })
                .map(|q| q.id)
        };
        // **Le groupe n'est pas atomique** : chaque membre est posé pour son
        // propre compte, et l'anti-collision peut n'en accepter qu'une partie.
        //
        // ⚠️ C'est un choix, pas un oubli (arbitrage rendu le 2026-08-01). Une
        // version atomique a été essayée — vérifier que *tous* les membres
        // passent avant d'en poser un seul — au motif qu'une demi-paire donne
        // une station bancale. Elle tient la symétrie mais **perd des pièces** :
        // un groupe entier saute dès qu'un seul de ses ports est encombré, et
        // une station de 16 modules retombait à 6 radiateurs au lieu de 8.
        //
        // Or la symétrie n'est pas recherchée partout : elle vaut pour la barre
        // de l'ISS (bâbord/tribord), pas pour la coque, où des équipements
        // dépareillés sont la règle sur les vraies stations. On préfère donc
        // servir ce qui tient. Le contrôle `Chantier::peut_poser` reste en place
        // — c'est la palette de l'éditeur qui en a besoin, pas la grammaire.
        for (pos, avant) in &vises {
            if let Some(i) = trouver(ch, *pos, *avant) {
                ch.poser(i, app.clone(), 0);
            }
        }
    }
}
