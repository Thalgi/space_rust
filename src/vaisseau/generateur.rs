//! Générateur procédural de stations, posé sur le [`Chantier`]
//! (`docs/conception/stations.md`, Partie A §6–7). Une **grammaire** pilote le
//! constructeur : choix d'une ossature, puis habillage des ports libres, le tout
//! borné par un budget et rendu déterministe par une graine.

use super::chantier::Chantier;
use super::montage::{cuire, port_monde, poser};
use super::{
    Assembleur, Composant, EtatStation, GenrePort, PiedHexa, Profil, Repere, Sorties, StyleTreillis,
    BOUCLIER_ELANCEMENT,
    VarianteAntenne, VarianteCaisson, VarianteCharge, VarianteCoiffe, VarianteModule,
    VariantePanneau, VariantePropulseur, VarianteRadiateur,
};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI, TAU};

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

// --- Échelle de l'ossature de l'ISV -----------------------------------------
/// Facteur appliqué à l'**épine et à la propulsion** (2026-07-29, à l'œil).
///
/// Il est appliqué **géométriquement**, en composant une mise à l'échelle dans
/// les transformées cuites, et non en multipliant des cotes : l'épaisseur du
/// treillis, le diamètre des modules Cœur et le gabarit des hexagones viennent
/// de `Profil`, un enum discret plafonné à P3 — aucune constante ne saurait les
/// étirer de 20 %.
const ISV_ECHELLE: f32 = 1.2;

/// Extension hors-tout de la flèche de l'épine **à l'échelle 1** : demi-section
/// 0,5, longerons de rayon 0,225 posés sur les **coins** du carré, donc à
/// √2·0,5 de l'axe. Mesurée sur le dessin de [`Composant::Charpente`].
const EPINE_FLECHE: f32 = 0.5 * std::f32::consts::SQRT_2 + 0.225;
/// La même, au gabarit réel du vaisseau. **C'est la référence** de tout ce qui
/// vient se placer autour de l'épine : le fret et l'habitat gardent leur
/// taille, mais leurs rayons de couronne se recalent là-dessus.
const EPINE: f32 = EPINE_FLECHE * ISV_ECHELLE;

/// La même cote pour l'épine **hexagonale**, à l'échelle 1.
///
/// Même forme que [`EPINE_FLECHE`], et ce n'est pas un hasard : le circonradius
/// de la section hexagonale est **repris des coins du carré** (`0,5·√2`, cf.
/// `treillis::hexa_rayons`), donc les sommets sont à la même distance de l'axe.
/// Seule diffère l'épaisseur des longerons — `0,12 · 1,5·√2 = 0,2546` contre
/// `0,15 · 1,5 = 0,225`.
///
/// ⚠️ L'épine hexagonale est donc **3,2 % plus large hors-tout**. Ça paraît
/// négligeable et ça ne l'est pas : c'est exactement la nature de l'erreur de
/// §C.6, où l'épine avait grossi et la charge utile s'était retrouvée plantée
/// dans la structure. D'où le paramétrage qui suit — aucun rayon de couronne
/// n'est écrit en dur, tous se déduisent de la variante d'épine.
const EPINE_FLECHE_HEXA: f32 = 0.5 * std::f32::consts::SQRT_2 + 0.2546;

/// Section de l'épine du vaisseau. Les deux variantes coexistent le temps de
/// trancher à l'écran — voir `docs/suivi/stations.md` §C.9.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Epine {
    /// Treillis à section **carrée** : l'épine historique.
    #[default]
    Carree,
    /// Treillis à section **hexagonale** : silhouette bien plus constante selon
    /// l'angle (donc lisible sous filtre pixel), et pied en **tour coaxiale** qui
    /// prolonge le cône au lieu d'un cadre couché à raccorder.
    Hexagonale,
}

impl Epine {
    /// Hors-tout de la flèche, **au gabarit du vaisseau**. C'est la référence de
    /// tout ce qui vient se placer autour de l'épine : la charge utile garde sa
    /// taille propre, mais ses rayons de couronne se recalent là-dessus.
    const fn hors_tout(self) -> f32 {
        ISV_ECHELLE
            * match self {
                Epine::Carree => EPINE_FLECHE,
                Epine::Hexagonale => EPINE_FLECHE_HEXA,
            }
    }

    /// La charpente correspondante, à cotes identiques.
    fn charpente(self, grand: Profil, petit: Profil, longueur: f32, courbure: f32) -> Composant {
        match self {
            Epine::Carree => Composant::Charpente { grand, petit, longueur, courbure, aiguille: true },
            Epine::Hexagonale => {
                // **Pied en pavillon** (§C.11) : la corolle et son fût, sur lequel
                // la propulsion viendra se reposer. C'est le seul point où les deux
                // ISV diffèrent en plus de la section de leur épine.
                Composant::CharpenteHexa {
                    grand,
                    petit,
                    longueur,
                    courbure,
                    pied: PiedHexa::Pavillon,
                }
            }
        }
    }

    /// Libellé pour la vue.
    pub fn nom(self) -> &'static str {
        match self {
            Epine::Carree => "EPINE CARREE",
            Epine::Hexagonale => "EPINE HEXAGONALE",
        }
    }
}

/// **Déport latéral** (±X) des deux ensembles propulsion — aile radiateur et bloc
/// moteur — dans le repère de l'ossature.
///
/// Descendu à **6,5** (2026-07-30) pour que la propulsion se **pose sur** l'épine
/// au lieu de la longer : 6,5 → 9,5 → 8,5 → 6,5, au fil des passes visuelles.
///
/// ⚠️ **La valeur se mesure, elle ne se déduit pas**, et de deux façons :
///
/// 1. le rayon interne de l'aile n'est pas `déport − largeur/2` — le collecteur du
///    radiateur rentre plus près de l'axe que la pointe des ailettes ;
/// 2. il faut mesurer **dans la tranche que l'aile et le pied partagent**. Sur
///    l'ensemble de sa longueur, l'aile descend bien plus près de l'axe que là où
///    elle croise le fût : à 7,5 on lisait un engagement de −1,31 alors qu'au droit
///    du fût il n'était que de **−0,04**. Autant dire aucun contact.
///
/// Relevé au droit du fût (bande partagée), pied à 5,65 :
///
/// | déport | rayon interne de l'aile | engagement |
/// |---|---|---|
/// | 7,5 | 5,61 | −0,04 — se frôlent à peine |
/// | 7,0 | 5,01 | −0,64 |
/// | **6,5** | **4,41** | **−1,24** |
/// | 6,0 | 3,81 | −1,84 |
const PROPULSION_DEPORT: f32 = 6.5;

/// **Avancée** de la propulsion et des cuves le long de l'épine, **vers les
/// tuyères** (donc −Y dans le repère de l'ossature, −X sur le modèle couché).
///
/// ⚠️ **Bornée par le fût.** L'aile radiateur s'enracine à `−20` et se déploie vers
/// le bas ; le fût, lui, s'arrête à `−22`. Seuls ces 2 unités de recouvrement font
/// que la propulsion touche encore la structure. Avancer de plus de ~2 la ferait
/// **sortir sous le fût** : elle pendrait alors dans le vide, et l'engagement
/// latéral obtenu en resserrant `PROPULSION_DEPORT` ne servirait plus à rien.
const PROPULSION_AVANCE: f32 = 1.0;

/// **Jeu** entre les deux cuves de carburant d'un même côté.
///
/// Leur écart se déduit de leur propre rayon (`2·res_r + jeu`) et **non** de
/// l'écartement des plaques hexagonales, avec lequel il était confondu — deux
/// cotes qui n'ont aucune raison d'être égales, et qui laissaient les cuves
/// s'interpénétrer de 1,30.
const RESERVOIR_JEU: f32 = 0.7;

/// Longueur de la charpente-épine **à l'échelle 1**. L'évasement de la base
/// étant à distance absolue, la rallonger n'allonge que la flèche.
const EPINE_LONGUEUR: f32 = 84.0;
/// Ancrage de la base de l'épine, côté tuyères.
const EPINE_BASE_Y: f32 = -16.0;
/// Décalage supplémentaire vers le haut, en fraction de la longueur : la
/// charpente monte, radiateurs et moteurs restent en bas.
const EPINE_DECALAGE: f32 = 0.1;
/// **Sommet de l'épine** à l'échelle 1. C'est la limite au-delà de laquelle une
/// charge utile n'aurait plus rien à quoi s'accrocher — la section d'équipage,
/// qui est la pièce la plus haute du vaisseau, doit rester en dessous.
const EPINE_SOMMET_Y: f32 = EPINE_BASE_Y + EPINE_LONGUEUR * (1.0 + EPINE_DECALAGE);

// --- Section fret de l'ISV -------------------------------------------------
// Gabarit de base d'une rangée, puis une **échelle** unique par-dessus : c'est
// le rapport fret/vaisseau qui se juge à l'œil, et on le règle d'un seul
// chiffre plutôt qu'en retouchant trois cotes qui doivent rester cohérentes.
const FRET_LONG: f32 = 6.6; // longueur d'une rangée
const FRET_PAS: f32 = 7.2; // entraxe entre rangées
/// Nombre de rangées enfilées sur l'épine.
const FRET_RANGEES: usize = 4;
/// Bord **bas** du bloc de fret (côté moteurs), le long de l'épine, **à
/// l'échelle 1** (il est multiplié par [`ISV_ECHELLE`] à la pose, comme tout ce
/// qui se repère le long de l'épine).
///
/// Le bloc est ancré par ce bord et non par son centre : c'est là que finit le
/// tronçon d'épine nu et que commence la charge utile, donc le point qui a un
/// sens physique. Ajouter ou retirer une rangée allonge ou raccourcit le bloc
/// **vers le haut**, en libérant d'autant l'extrémité — celle qui doit encore
/// recevoir l'habitat, les modules d'équipage et le bouclier antidébris.
const FRET_DEBUT_Y: f32 = 42.5;
/// Échelle d'ensemble du fret — **le seul chiffre à toucher** pour juger le
/// rapport fret/vaisseau à l'œil. Porte sur les trois cotes du fret (longueur
/// de rangée, entraxe **et** taille de conteneur), pour qu'elles ne puissent
/// pas dériver les unes par rapport aux autres.
/// Passée de 0,70 à **0,56** (−20 %, 2026-07-29).
const FRET_ECHELLE: f32 = 0.56;

/// Rayon hors-tout d'un conteneur **à l'échelle 1**.
const FRET_NACELLE_BASE: f32 = 2.947;
/// Rayon hors-tout d'un conteneur, au gabarit réel.
///
/// Il ne dépend **pas** de [`FRET_RAYON`], et c'est le point : quand l'épine
/// change de gabarit, c'est la couronne qui s'ouvre, pas le fret qui grossit.
const FRET_NACELLE: f32 = FRET_NACELLE_BASE * FRET_ECHELLE;
/// **Jeu** entre la face interne d'un conteneur et la surface de l'épine.
///
/// Le fret est **déjà au ras** : ce qui commande, c'est le **coin** du treillis
/// carré, et le vide qu'on croit voir entre eux vient de ses **faces**, en
/// retrait de ~40 % par rapport à ses coins (côté à `0,5·k`, coin à
/// `√2·0,5·k`). Descendre beaucoup plus bas ferait mordre les conteneurs sur
/// les longerons d'angle.
const FRET_JEU: f32 = 0.02;
/// Rayon de la couronne de fret : juste assez pour que le creux central laisse
/// passer l'épine. Le creux vaut `rayon − r_nacelle·(0,5 + f/2)`, d'où la
/// formule — plus de nombre magique à re-régler quand [`ISV_ECHELLE`] bouge.
const fn fret_rayon(epine: f32) -> f32 {
    epine + FRET_JEU + FRET_NACELLE * (0.5 + 0.5 * 0.22)
}
/// Le même, pour l'épine carrée — la valeur historique, gardée pour les tests et
/// pour que le calcul n'existe **qu'une fois**.
const FRET_RAYON: f32 = fret_rayon(EPINE);

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
pub fn demo_radiateur_mega(chaleur: f32) -> EtatStation {
    let mut asm = Assembleur::new();
    let r = Composant::RadiateurMega { profil: Profil::P0, longueur: 26.0, largeur: 5.5, ailettes: 34, chaleur };
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
    isv(Epine::Carree, true, 0.0)
}

/// Le **second ISV**, identique au premier à une chose près : son épine est
/// [`Epine::Hexagonale`].
///
/// Les deux presets partagent **tout le reste du code** — même ossature, même
/// propulsion, même charge utile, même section d'équipage rotative. C'est
/// délibéré : à la moindre duplication, la comparaison ne voudrait plus rien dire,
/// puisqu'on ne saurait plus si un écart vient de la section d'épine ou d'une
/// dérive entre deux copies.
///
/// Les rayons de couronne, eux, **se recalent tout seuls** : l'épine hexagonale
/// est 3,2 % plus large hors-tout, et fret, habitat et alésage de collier se
/// déduisent tous de `Epine::hors_tout()`.
// Non réexporté (cf. `vaisseau/mod.rs`) : la vue passe par les deux moitiés, et
// seul un test consomme le vaisseau d'un seul tenant.
#[allow(dead_code)]
pub fn preset_isv_hexa() -> EtatStation {
    isv(Epine::Hexagonale, true, 0.0)
}

/// L'ISV **sans** sa section d'équipage : tout ce qui ne tourne pas.
///
/// La vue s'en sert pour cuire le vaisseau en **deux maillages** — le fixe et le
/// tournant ([`preset_isv_equipage`]) — et n'appliquer la rotation qu'au second.
/// Sans cette séparation, faire tourner la section obligerait soit à faire
/// tourner le vaisseau entier, soit à le recuire à chaque frame.
/// `chaleur` (0 à 1) porte les ailes radiateur au rouge puis à l'orange. Elle
/// est ici et non dans un réglage d'affichage parce qu'elle **change la
/// géométrie cuite** : les couleurs sont dans les sommets, donc chauffer un
/// radiateur veut dire le recuire.
pub fn preset_isv_fixe(epine: Epine, chaleur: f32) -> EtatStation {
    isv(epine, false, chaleur)
}

fn isv(epine: Epine, avec_equipage: bool, chaleur: f32) -> EtatStation {
    // Gabarit hors-tout de l'épine : **toute** la charge utile s'y recale.
    let gabarit = epine.hors_tout();
    let mut asm = Assembleur::new();
    // L'**ossature** (épine + propulsion) se construit dans son propre tampon,
    // à l'échelle 1, puis est reversée agrandie de [`ISV_ECHELLE`]. Passer par
    // une mise à l'échelle *géométrique* est le seul moyen d'agrandir aussi ce
    // que `Profil` quantifie — épaisseur de treillis, diamètre des Cœurs,
    // gabarit des hexagones. La charge utile, elle, garde sa taille : elle est
    // ajoutée **après**, directement dans `asm`, à des positions simplement
    // recalées sur le nouveau gabarit.
    let mut oss = Assembleur::new();

    // Charpente continue, axe le long de +Y (« vers le haut »). Base (P3, bout
    // −Z local) vers le bas, apex (P0, +Z local) vers le haut. `courbure` élevée
    // → l'évasement se concentre en bas, le reste file en flèche fine.
    // Évasement fixe (distance absolue) → rallonger `longueur` n'allonge que la
    // flèche : ~34 → 40 ajoute un tiers de tige sans toucher à la base.
    // Pied posé (`aiguille`) : anneau hexagonal couché pour l'épine carrée, tour
    // hexagonale coaxiale pour l'hexagonale. `petit` = P1 (bout agrandi, section
    // doublée par rapport au P0 d'origine).
    let longueur = EPINE_LONGUEUR;
    let charpente = epine.charpente(Profil::P3, Profil::P1, longueur, 2.6);
    // Position = centre local de la charpente. Base ancrée à Y = −16, la
    // rallonge part vers le **haut** (Y_centre = base + L/2), plus un **décalage
    // vers le haut d'un bon dixième de la hauteur** (la charpente monte, les
    // radiateurs/moteurs restent en bas).
    let decalage = longueur * EPINE_DECALAGE;
    let y_centre = EPINE_BASE_Y + longueur * 0.5 + decalage;
    let base = Repere::new(vec3(0.0, y_centre, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::Y));
    oss.ajouter(cuire(base, &charpente));

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
            chaleur,
        };
        let orient = Quat::from_rotation_arc(Vec3::Z, Vec3::NEG_Y); // sens inversé
        let rot = Quat::from_rotation_z(cote * tilt) * orient;
        let pos = Vec3::new(-PROPULSION_DEPORT * cote, -20.0 - PROPULSION_AVANCE, 0.0);
        let repere = Repere::new(pos, rot);
        oss.ajouter(cuire(repere, &aile));
        // Bloc moteur docké au collecteur de CE radiateur, comme dans la vue
        // radiateur+bloc moteur. Le côté −X est le **flip** de l'autre (miroir).
        // `propulseur = true` : version **complète** (Cœur 3 noir + chapes bombées
        // sur Cœur 1/2 + propulseur antimatière) intégrée à la charpente.
        poser_bloc_moteur(&mut oss, repere, radia_w, cote < 0.0, true, chaleur);
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
    // dupliquée plus bas le long de la charpente, avec un **demi-tour en Z**
    // supplémentaire.
    //
    // Écart des **plaques hexagonales** = `2·ap` : leurs demi-hauteurs (`ap` en Y)
    // se touchent alors bord à bord, sans montants.
    let dy = 2.0 * ap;
    // Écart des **cuves**, désormais **découplé** de celui des plaques.
    //
    // Les deux tenaient le même écart, et c'était trop juste pour elles : deux
    // sphères de rayon `res_r` distantes de `2·ap` = 5,20 s'interpénètrent de
    // **1,30**, ce qui se voyait — la note « léger chevauchement accepté » sous-
    // estimait franchement. La cote se déduit maintenant du rayon des cuves plutôt
    // que de la géométrie des plaques, qui n'a rien à voir avec elles.
    let res_ecart = 2.0 * res_r + RESERVOIR_JEU;
    for sz in [1.0_f32, -1.0] {
        let base = if sz > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_x(PI) };
        // Le retournement (rotation_x PI) de la cuve −Z **mire** le triangle, ce qui
        // désaligne son sommet de 60° : on le rattrape pour que les deux cuves
        // pointent un sommet vers la tige de la charpente.
        let corr = if sz > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_z(PI / 3.0) };
        let rot = base * spin * corr;
        let z = sz * (prof + res_r - 1.0); // réservoir enfoncé dans la charpente (écart −1.0)
        // Cuve d'origine.
        oss.ajouter(cuire(Repere::new(vec3(0.0, hex_y - PROPULSION_AVANCE, z), rot), &reservoir));
        // Cuve dupliquée : plus bas le long de la charpente + demi-tour Z. Elle
        // descend selon `res_ecart` et non `dy` — c'est ce découplage qui la
        // décolle de sa jumelle sans déplacer les plaques hexagonales.
        let rot2 = rot * Quat::from_rotation_z(PI);
        oss.ajouter(cuire(Repere::new(vec3(0.0, hex_y - res_ecart - PROPULSION_AVANCE, z), rot2), &reservoir));
    }

    // **Second anneau hexagonal** au niveau du groupe de réservoirs dupliqué
    // (`hex_y - dy`) : comme l'écart vaut `2·ap`, il **touche** bord à bord celui
    // du pied de la charpente. Plus besoin de montants (`liaison = 0`).
    let hexa = Composant::TreillisHexagone { profil: Profil::P3, liaison: 0.0 };
    let hexa_rot = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    oss.ajouter(cuire(Repere::new(vec3(0.0, hex_y - dy, 0.0), hexa_rot), &hexa));

    // **Section charge utile** : quatre rangées de fret enfilées sur l'épine, à
    // l'**autre bout** que les moteurs. C'est la disposition du vrai ISV, qui
    // est un **tracteur** : les moteurs tirent, la charge suit au bout d'une
    // longue épine en tension (elle encaisse mieux la traction que la poussée,
    // et l'écart protège la charge des tuyères).
    //
    // Rangées en **triforce** : trois conteneurs par rangée, dont le creux
    // triangulaire central laisse passer l'épine. Vérifié : au-delà de ~16 % de
    // sa longueur la charpente est réduite à sa flèche fine (demi-section 0,5,
    // soit ~0,93 hors-tout longerons compris) ; à `rayon` 3,2 le creux offre un
    // cercle inscrit de 1,40. La marge est donc franche, et c'est *pour ça* que
    // le fret est ici et pas plus bas, où la base évasée (1,5) mordrait dedans.
    //
    // On s'arrête vers Y = 66 : le haut de l'épine reste libre pour l'habitat,
    // les modules d'équipage et le bouclier antidébris, encore à faire.
    // Ossature terminée : on la verse **agrandie** dans l'assemblage final.
    verser_a_echelle(&mut asm, oss.terminer(), ISV_ECHELLE);

    // À partir d'ici, tout est posé **au gabarit final** : la charge utile garde
    // sa taille propre, seules ses positions le long de l'épine et ses rayons de
    // couronne suivent l'échelle de l'ossature.
    let rat_long = FRET_LONG * FRET_ECHELLE;
    let rat_pas = FRET_PAS * FRET_ECHELLE; // un jour net : les rangées se lisent séparément
    for k in 0..FRET_RANGEES {
        let ratelier = Composant::RatelierCargo {
            profil: Profil::P1,
            longueur: rat_long,
            // La couronne s'ouvre pour laisser passer l'épine élargie ; le
            // conteneur, lui, garde exactement sa taille validée à l'écran.
            rayon: fret_rayon(gabarit),
            nacelles: 3,
            nacelle: FRET_NACELLE,
        };
        let y = (FRET_DEBUT_Y + rat_long * 0.5 + k as f32 * rat_pas) * ISV_ECHELLE;
        asm.ajouter(cuire(Repere::new(vec3(0.0, y, 0.0), hexa_rot), &ratelier));
    }

    // **Habitat principal**, juste au-dessus du fret : l'ordre le long de
    // l'épine suit celui du vrai vaisseau — moteurs, épine nue, fret, puis
    // l'habitat le plus loin possible des tuyères. Les ferrures de chaque
    // module viennent se poser sur la flèche (leur portée est calée dessus,
    // cf. `HAB_ATTACHE`).
    poser_grappe_habitat(&mut asm, HAB_CENTRE_Y * ISV_ECHELLE, hexa_rot, gabarit);

    // **Section d'équipage rotative**, au-delà de l'habitat : le point du
    // vaisseau le plus éloigné des tuyères. Elle n'est incluse ici que pour un
    // rendu d'un seul tenant ; la vue, qui doit la faire tourner, la reprend
    // séparément via `preset_isv_equipage`.
    if avec_equipage {
        poser_equipage(
            &mut asm,
            EQUIPAGE_CENTRE_Y * ISV_ECHELLE,
            hexa_rot,
            EtatEquipage::default().repli(),
            gabarit,
        );
    }

    // **Bouclier thermique** au droit des moteurs, là où l'épine prend le
    // rayonnement des tuyères de plein fouet. C'est un **détail de surface** —
    // il n'ajoute ni masse visible ni encombrement, et ne change donc rien aux
    // proportions d'ensemble.
    asm.ajouter(cuire(
        Repere::new(vec3(0.0, THERMIQUE_DEBUT_Y, 0.0), hexa_rot),
        &bouclier_thermique(),
    ));

    // **Tête de bouclier**, tout au bout : la petite plaque puis les trois
    // grandes, enfilées sur leur propre mât au-delà du sommet d'épine. Elle est
    // posée avec la coque fixe et non avec la section d'équipage — c'est de la
    // structure, elle ne tourne pas.
    poser_tete_bouclier(&mut asm, BOUCLIER_DEBUT_Y, hexa_rot);

    // **Modèle complet à l'horizontale** : rotation globale de 90° autour de Z
    // (l'axe +Y du vaisseau bascule vers +X), appliquée à toutes les pièces.
    pivoter(asm.terminer(), ISV_COUCHE)
}

/// Pivot final du modèle : le vaisseau est couché, son axe +Y basculant sur +X.
/// Sorti en constante parce que **les deux moitiés** de l'ISV (la partie fixe et
/// la section d'équipage) doivent subir exactement le même, sous peine de ne
/// plus se rejoindre.
const ISV_COUCHE: Quat = Quat::from_xyzw(0.0, 0.0, -FRAC_1_SQRT_2, FRAC_1_SQRT_2);

/// L'**axe du vaisseau** dans le modèle final (après [`ISV_COUCHE`]) : c'est
/// autour de lui que tourne la section d'équipage.
pub const ISV_AXE: Vec3 = Vec3::X;

/// La **seule section d'équipage** de l'ISV, placée dans le repère du vaisseau
/// fini — donc superposable à [`preset_isv`].
///
/// Elle est fournie à part précisément parce qu'elle **tourne** : la faire
/// pivoter revient à composer une matrice sur ce maillage-là, au lieu de recuire
/// tout le vaisseau à chaque frame. Le repli, lui, change la géométrie et
/// impose bien une reconstruction — d'où le paramètre.
/// `repli` est la valeur **continue** (0 déployé → 1 replié) et non un
/// [`EtatEquipage`] : la vue anime le passage d'un état à l'autre, et a donc
/// besoin des positions intermédiaires.
pub fn preset_isv_equipage(epine: Epine, repli: f32) -> EtatStation {
    let mut asm = Assembleur::new();
    let hexa_rot = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    poser_equipage(
        &mut asm,
        EQUIPAGE_CENTRE_Y * ISV_ECHELLE,
        hexa_rot,
        repli,
        epine.hors_tout(),
    );
    pivoter(asm.terminer(), ISV_COUCHE)
}

/// Recopie les pièces d'un état dans `dest`, en composant une **mise à
/// l'échelle** dans leur transformée cuite.
///
/// Contrairement à un facteur passé aux cotes, ça agrandit **aussi** ce que
/// `Profil` quantifie (section de treillis, diamètre de module, gabarit
/// d'hexagone) — des enums discrets qu'aucune constante ne saurait étirer.
///
/// *Limite connue* : `rayon_local()` d'un composant ignore cette échelle, donc
/// la sphère englobante de la station la sous-estime (~9 % ici). Sans
/// conséquence — elle ne sert qu'au cadrage caméra, qui garde 35 % de marge.
fn verser_a_echelle(dest: &mut Assembleur, source: EtatStation, k: f32) {
    let EtatStation::Prete(s) = source else {
        return;
    };
    let m = Mat4::from_scale(Vec3::splat(k));
    for p in s.pieces() {
        dest.ajouter(super::Piece::new(m * p.transforme, p.composant.clone()));
    }
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
        chaleur: 0.0,
    };
    asm.ajouter(cuire(Repere::IDENTITE, &radia));

    // Bloc moteur docké au collecteur du radiateur (radiateur au repère identité).
    // `propulseur = true` : Cœur 3 reçoit le propulseur à antimatière complet.
    poser_bloc_moteur(&mut asm, Repere::IDENTITE, lx, false, true, 0.0);

    asm.terminer()
}

/// Pose le **bloc moteur** complet (caisse + rangée d'habitats + les 3 Cœurs),
/// **docké au collecteur** du radiateur dont le repère monde est `radia` (de
/// largeur `radia_largeur`) — exactement comme la vue « radiateur + bloc moteur ».
/// Tout est composé dans le repère du radiateur, donc valable même incliné.
#[allow(clippy::too_many_arguments)]
fn poser_bloc_moteur(asm: &mut Assembleur, radia: Repere, radia_largeur: f32, miroir: bool, propulseur: bool, regime: f32) {
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
        let pose = poser(port_base, &tuyere, 0);
        asm.ajouter(cuire(pose, &tuyere));

        // **Panache**, posé au bout de la tuyère et dans son axe. Le jet part le
        // long du **−Z local** du moteur (c'est là que sont la buse et les deux
        // anneaux de stabilisation) : on retourne donc le repère d'un demi-tour.
        if regime > 1e-3 {
            let sortie = Repere::new(
                pose.pos + pose.rot * (Vec3::NEG_Z * (taille * PANACHE_SORTIE)),
                pose.rot * Quat::from_rotation_y(PI),
            );
            asm.ajouter(cuire(
                sortie,
                &Composant::Panache {
                    longueur: PANACHE_LONGUEUR / ISV_ECHELLE,
                    rayon_col: taille * PANACHE_COL,
                    rayon_bout: PANACHE_BOUT / ISV_ECHELLE,
                    intensite: regime,
                },
            ));
        }
    }
}

// --- Panache d'antimatière ---------------------------------------------------
/// Longueur du jet à pleine poussée, **au gabarit final**.
///
/// Deux longueurs de vaisseau (168 × 2 ≈ 336). Ce n'est pas une exagération : un
/// jet de pions relativistes n'a rien qui l'arrête, et sa portée visible dit
/// exactement ce que la propulsion a d'inhabituel. Une queue courte donnerait un
/// moteur chimique.
const PANACHE_LONGUEUR: f32 = 336.0;
/// Rayon du jet à son bout, au gabarit final.
///
/// ⚠️ **Cette cote décide si le panache lèche la charge utile.** Les tuyères sont
/// braquées de 5° vers l'extérieur précisément pour que le jet passe à côté du
/// vaisseau remorqué ; un panache trop ouvert annulerait ce braquage et
/// reviendrait sur la coque. Gardée par `le_panache_ne_leche_pas_la_charge_utile`.
///
/// Réglée à l'écran : 22 jugé trop large, **divisé par deux**.
const PANACHE_BOUT: f32 = 11.0;
/// Rayon au col, en fraction de la taille du moteur.
///
/// La moitié des anneaux de stabilisation (0,30), et non leur diamètre : le jet
/// sort **pincé** par la tuyère magnétique, plus étroit que l'ouverture qui le
/// laisse passer. C'est d'ailleurs ce que fait un col magnétique — il resserre
/// le faisceau avant de le lâcher.
const PANACHE_COL: f32 = 0.15;
/// Distance de la sortie au repère du moteur, en fraction de sa taille — juste
/// au-delà du second anneau de stabilisation (1,60).
const PANACHE_SORTIE: f32 = 1.66;

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

/// Vue briques : l'**épine hexagonale candidate**, à côté de l'épine carrée
/// actuelle pour que la comparaison soit directe et non de mémoire.
///
/// De gauche à droite : carrée nue, **hexagonale nue**, carrée avec cadre de
/// propulsion, **hexagonale avec cadre**. Ce sont les deux paires qui comptent —
/// la silhouette de la poutre, et la façon dont le pied se raccorde au cadre.
///
/// Ce qu'il faut juger :
/// 1. **de loin, filtre pixel (X) allumé** : l'hexagonale doit rester visible
///    sous tous les angles alors que la carrée maigrit de trois quarts. C'est le
///    grief d'origine, et la seule chose que le calcul promet
///    (`pieces::HEXA_GAIN_SILHOUETTE`) ;
/// 2. **au pied** : la transition six-pour-six et sa baie de torsion, contre les
///    quatre coins convergeant sur deux sommets de la version carrée.
///
/// **Rien n'est encore monté sur l'ISV** : `preset_isv` utilise toujours
/// `Composant::Charpente`. Le remplacement attend la validation à l'écran.
pub fn demo_charpente_hexa() -> EtatStation {
    let mut asm = Assembleur::new();
    let debout = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    // Écartement : de quoi séparer les quatre spécimens sans les perdre de vue.
    for (dx, hexa, aiguille) in [
        (-21.0_f32, false, false),
        (-7.0, true, false),
        (7.0, false, true),
        (21.0, true, true),
    ] {
        let (grand, petit, longueur, courbure) = (Profil::P3, Profil::P0, 40.0, 2.6);
        let ch = if hexa {
            let pied = if aiguille { PiedHexa::Tour } else { PiedHexa::Aucun };
            Composant::CharpenteHexa { grand, petit, longueur, courbure, pied }
        } else {
            Composant::Charpente { grand, petit, longueur, courbure, aiguille }
        };
        asm.ajouter(cuire(Repere::new(vec3(dx, 0.0, 0.0), debout), &ch));
    }
    asm.terminer()
}

/// Vue briques : l'épine hexagonale **à pavillon**, la forme demandée au schéma
/// du 2026-07-30 — le cône ne s'arrête plus sur une tour, il **continue de
/// s'ouvrir** jusqu'à une large embouchure hexagonale évidée.
///
/// À gauche la version **tour** (§C.9), à droite la version **pavillon** (§C.11) :
/// c'est le même cône, seul le pied change, et les deux se comparent d'un regard.
///
/// Ce qu'il faut juger :
/// 1. **l'ouverture** — la corolle doit lire comme une corolle, pas comme un cône
///    tronqué (`PAVILLON_OUVERTURE`, `PAVILLON_HAUTEUR`) ;
/// 2. **l'anneau d'embouchure** vu de bout (tourner la caméra pour regarder dans
///    l'axe) : hexagone extérieur, hexagone intérieur, six panneaux. Son
///    écrasement selon Y donne **quatre** arêtes obliques égales et **deux**
///    arêtes horizontales égales — les A…F du schéma
///    (`PAVILLON_ETIREMENT` ; à 1 les six redeviendraient égales).
///
/// **Rien n'est monté sur l'ISV** : les deux presets restent en pied *tour*. Le
/// pavillon attend sa validation, et la structure de propulsion qui viendra s'y
/// poser.
pub fn demo_epine_pavillon() -> EtatStation {
    let mut asm = Assembleur::new();
    let debout = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    for (dx, pied) in [(-11.0_f32, PiedHexa::Tour), (11.0, PiedHexa::Pavillon)] {
        let ch = Composant::CharpenteHexa {
            grand: Profil::P3,
            petit: Profil::P0,
            longueur: 40.0,
            courbure: 2.6,
            pied,
        };
        asm.ajouter(cuire(Repere::new(vec3(dx, 0.0, 0.0), debout), &ch));
    }
    asm.terminer()
}

/// Vue briques : le **fret d'échelle vaisseau** (section charge utile de l'ISV).
/// En haut une nacelle **seule** — sa section « onigiri » (triangle arrondi) se
/// lit de bout, avec ses trois rails d'arête et ses collerettes. En dessous deux
/// **râteliers** de 6 et 8 nacelles : même rayon de couronne, donc des nacelles
/// plus fines quand elles sont plus nombreuses (le pas angulaire commande).
/// Tout est couché le long de X pour montrer l'élancement réel des conteneurs.
pub fn demo_cargo() -> EtatStation {
    let mut asm = Assembleur::new();
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);

    // Nacelle isolée, en gros gabarit : c'est la vue qui sert à juger la section.
    let seule = Composant::NacelleCargo { profil: Profil::P1, longueur: 15.0, spin: 0.0 };
    asm.ajouter(cuire(Repere::new(vec3(-7.5, 11.0, 0.0), couche), &seule));

    // Deux dispositions de râtelier : la **triforce** (3 nacelles de même
    // orientation qui se touchent par la pointe, creux triangulaire au milieu)
    // et la **couronne** de 6 (coin vers l'axe). Le rayon de triforce est plus
    // petit : à 3, la nacelle fait à elle seule tout le rayon de la grappe.
    for (i, (n, rayon)) in [(3usize, 2.6_f32), (6, 4.5)].into_iter().enumerate() {
        let r = Composant::RatelierCargo { profil: Profil::P2, longueur: 16.0, rayon, nacelles: n, nacelle: 0.0 };
        asm.ajouter(cuire(Repere::new(vec3(0.0, -(i as f32) * 13.0, 0.0), couche), &r));
    }
    asm.terminer()
}

// --- Section habitat principal de l'ISV -------------------------------------
// À ne pas confondre avec les **modules d'équipage rotatifs**, qui tournent
// autour du vaisseau pour la gravité artificielle : ceux-là sont une autre
// brique, encore à faire. Ici c'est l'habitat **fixe**, solidaire de l'épine.
/// Longueur d'un module d'habitat (réduite de 33 % à la validation visuelle).
const HAB_LONG: f32 = 8.0;
/// Nombre de modules autour de l'épine (le vrai ISV en a trois).
const HAB_MODULES: usize = 3;
/// Rayon inscrit d'un module (centre → côté plat), pour un fût P2.
const HAB_INSCRIT: f32 = 2.0 * (0.5 + 0.5 * 0.22);
/// **Jeu** entre le côté plat d'un module et la surface de l'épine — le seul
/// chiffre à toucher pour serrer ou desserrer la grappe. Les ferrures ne font
/// que franchir ce jeu, elles se raccourcissent donc avec lui.
///
/// **Seule l'épine borne le serrage** : le plancher est 0 (le module vient au
/// contact). Les trois modules, eux, ne se gênent jamais entre eux — leur
/// propre rayon inscrit (1,22) impose déjà une couronne large devant ce qu'il
/// leur faudrait pour se croiser. Vérifié plutôt que supposé, dans
/// `la_charge_utile_suit_le_gabarit_de_lepine`.
const HAB_JEU: f32 = 0.25;
/// Rayon de la couronne d'habitat (axe → centre de module), déduit du jeu et du
/// gabarit de l'épine : la grappe suit d'elle-même tout changement d'échelle de
/// l'ossature.
const fn hab_rayon(epine: f32) -> f32 {
    epine + HAB_JEU + HAB_INSCRIT
}
/// Le même, pour l'épine carrée (valeur historique, et calcul unique).
const HAB_RAYON: f32 = hab_rayon(EPINE);
/// Portée de la ferrure d'attache : du côté plat du module jusqu'à la surface
/// de l'épine. Le module se pose **contre** la structure, il ne flotte pas à
/// côté.
///
/// Doublé, parce que les deux ferrures se posent à **mi-portée** de ce champ :
/// c'est `attache/2` qui donne le déport réel des longerons.
const HAB_ATTACHE: f32 = 2.0 * HAB_JEU;
/// Centre de la grappe d'habitat le long de l'épine (avant pivot). Posée juste
/// au-dessus du fret (qui finit vers Y ≈ 57), en laissant le haut de l'épine
/// libre pour les modules d'équipage rotatifs et le bouclier antidébris.
const HAB_CENTRE_Y: f32 = 63.0;

/// Pose la **grappe d'habitat principal** : `HAB_MODULES` fûts composites en
/// couronne autour de l'axe, centrés sur `y_centre`, dans le plan
/// perpendiculaire donné par `orient` (leur axe long suit l'épine).
///
/// Chaque module est tourné (`spin = a`) pour présenter **un coin vers
/// l'extérieur** et donc **un côté plat vers l'axe** — c'est ce côté qui porte
/// la ferrure et vient se boulonner sur l'épine.
///
/// Partagée par la vue Briques et par `preset_isv` : ce qu'on valide à l'écran
/// est **exactement** ce qui part sur le vaisseau.
fn poser_grappe_habitat(asm: &mut Assembleur, y_centre: f32, orient: Quat, epine: f32) {
    let rayon = hab_rayon(epine);
    for k in 0..HAB_MODULES {
        let a = FRAC_PI_2 + TAU * k as f32 / HAB_MODULES as f32;
        let module = Composant::ModuleHabitat {
            profil: Profil::P2,
            longueur: HAB_LONG,
            spin: a,
            attache: HAB_ATTACHE,
        };
        // Décalage radial exprimé dans le repère de la grappe, puis tourné avec
        // elle : la couronne reste perpendiculaire à l'épine quel que soit
        // `orient`.
        let radial = orient * (vec3(a.cos(), a.sin(), 0.0) * rayon);
        asm.ajouter(cuire(Repere::new(radial + Vec3::Y * y_centre, orient), &module));
    }
}

/// Vue briques : l'**habitat principal** de l'ISV (fixe, à distinguer des
/// modules d'équipage rotatifs, encore à faire). À gauche un module seul, sans
/// ferrure — coque composite nue à section onigiri, ceinturée de ses trois
/// armatures triangulaires. À droite la grappe de trois telle qu'elle sera
/// posée : coins vers l'extérieur, côtés plats et ferrures tournés vers l'axe,
/// où passera l'épine.
pub fn demo_habitat_isv() -> EtatStation {
    let mut asm = Assembleur::new();
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);

    let seul = Composant::ModuleHabitat {
        profil: Profil::P2,
        longueur: HAB_LONG,
        spin: 0.0,
        attache: 0.0, // présenté seul : pas de ferrure, on juge la coque
    };
    asm.ajouter(cuire(Repere::new(vec3(0.0, 11.0, 0.0), couche), &seul));

    poser_grappe_habitat(&mut asm, 0.0, couche, EPINE);
    asm.terminer()
}

// --- Section d'équipage rotative de l'ISV -----------------------------------
// Gabarit **divisé par deux** le 2026-07-30 : à la taille précédente (bras 9,
// module 7, donc 16 unités de demi-envergure) la section dominait la silhouette
// et venait chevaucher les panaches de propulsion à l'écran.
//
// Le **collier ne suit pas** cette réduction : son rayon extérieur est imposé
// par l'épine qu'il doit envelopper (cf. `EQUIPAGE_ALESAGE`), pas par les
// proportions de la section. Il consomme donc une constante de 2 unités sur la
// demi-envergure, qui passe de 16 à 9 — un peu plus que la moitié.

/// Longueur d'un module d'équipage (du raccord au plancher).
const EQUIPAGE_LONG: f32 = 3.5;
/// Demi-envergure de la traverse : distance axe → raccord d'un module. C'est
/// elle qui fixe le rayon de rotation, donc la gravité obtenue. Le bras utile
/// vaut cette valeur **moins** le rayon du collier.
const EQUIPAGE_BRAS: f32 = 5.5;
/// Longueur du collier de rotation le long de l'épine.
const EQUIPAGE_COLLIER: f32 = 1.4;
/// Rayon extérieur du collier — **réduit de 30 %** le 2026-07-30 (2,0 → 1,4) :
/// le tambour dominait encore le centre de la section.
///
/// Cote **libre et non un cran de `Profil`** : elle se règle contre l'épine, et
/// les crans (1,0 puis 2,0) n'offrent rien entre « plus maigre que la flèche »
/// et « le double ». Sa borne basse est dure — il faut rester **au-dessus** de
/// [`EPINE`] (1,12), sinon les longerons ressortent à travers la jaquette. À 1,4
/// il ne reste que 0,28 de marge : c'est le test
/// `le_collier_dequipage_enveloppe_lepine_sans_jour` qui la garde.
const EQUIPAGE_COLLIER_RAYON: f32 = 1.4;
/// Alésage du collier : le trou par lequel passe l'épine.
///
/// Pris **volontairement plus étroit que l'épine** ([`EPINE`]), donc plus étroit
/// que sa propre section inscrite : les membrures de la flèche mordent alors dans
/// la paroi du tambour et il ne subsiste **aucun jour** entre les deux. On ne voit
/// plus que les surfaces extérieures du collier, et il lit comme solidaire de
/// l'épine.
///
/// Un alésage plus large que l'épine — ce qu'un vrai palier demanderait — laisse
/// au contraire voir l'anneau de vide tout autour, et la pièce a l'air *enfilée*
/// sur la flèche comme une chaussette plutôt que montée dessus. Le rendu a été
/// tranché en faveur de la lecture : c'est la seule cote de la section qui ne
/// décrit pas une mécanique plausible.
const fn equipage_alesage(epine: f32) -> f32 {
    epine * 0.45
}
/// Le même, pour l'épine carrée (valeur historique, et calcul unique).
const EQUIPAGE_ALESAGE: f32 = equipage_alesage(EPINE);
/// Gabarit de la charnière de repli (demi-largeur de chape).
const EQUIPAGE_CHARNIERE: f32 = 0.31;
/// Centre de la section d'équipage le long de l'épine (à l'échelle 1) : au-delà
/// de l'habitat, sur le tronçon d'épine encore libre. C'est le bout du vaisseau
/// le plus loin des tuyères, donc celui où l'équipage vit le mieux.
const EQUIPAGE_CENTRE_Y: f32 = 71.0;

/// Pose la **section d'équipage rotative** : une traverse perpendiculaire à
/// l'épine, centrée sur `y_centre`, et un module habité à **chaque bout**,
/// tourné vers l'extérieur.
///
/// Déployée (configuration de croisière, celle qui tourne) plutôt que repliée
/// le long de la coque : c'est la silhouette reconnaissable du vaisseau, et le
/// repli n'a de sens que sous poussée.
///
/// Partagée par la vue Briques et par `preset_isv`, comme les autres grappes :
/// ce qui est validé à l'écran est ce qui part sur le vaisseau.
/// Configuration de la section d'équipage. **C'est un état du vaisseau**, pas
/// un réglage de maquette : il change avec ce que le vaisseau est en train de
/// faire.
///
/// - **Déployée** — en station devant un astre : les bras sont sortis et la
///   section tourne, ce qui donne sa gravité à l'équipage ;
/// - **Repliée** — en transit : bras rabattus le long de la coque. C'est
///   obligatoire sous poussée (une structure sortie encaisserait mal
///   l'accélération) et ça réduit la cible offerte aux poussières à 0,7 c.
///
/// `repli()` donne la valeur continue, pour animer le passage de l'un à
/// l'autre plutôt que de sauter.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum EtatEquipage {
    /// En orbite : bras sortis, section en rotation.
    #[default]
    Deploye,
    /// En transit : bras rabattus le long de la coque.
    Replie,
}

impl EtatEquipage {
    /// Fraction de repli : 0 = déployé, 1 = replié.
    pub fn repli(self) -> f32 {
        match self {
            EtatEquipage::Deploye => 0.0,
            EtatEquipage::Replie => 1.0,
        }
    }
}

/// Direction de pivot du bras `dehors` (dans le repère local de la grappe, où
/// l'épine est +Z) : perpendiculaire au bras **et** à l'épine, donc le
/// tangentiel. Les deux bras tournent autour d'axes opposés et se rabattent
/// donc symétriquement.
fn axe_pivot(dehors: Vec3) -> Vec3 {
    dehors.cross(Vec3::Z).normalize()
}

fn poser_equipage(asm: &mut Assembleur, y_centre: f32, orient: Quat, repli: f32, epine: f32) {
    // **Collier de rotation** au centre : le moyeu qui porte les bras. Son
    // alésage est plus étroit que l'épine (cf. `EQUIPAGE_ALESAGE`), si bien que
    // la flèche traverse la paroi du tambour au lieu de flotter dans un jour —
    // c'est ce qui le fait lire comme solidaire du vaisseau. Son rayon extérieur
    // (P2) est la seule cote de la section calée sur l'épine et non sur les
    // proportions de la section : il faut qu'il **dépasse** le hors-tout de la
    // flèche, sinon les longerons ressortiraient à travers sa jaquette.
    let collier = Composant::CollierRotatif {
        profil: Profil::P2,
        rayon: EQUIPAGE_COLLIER_RAYON,
        alesage: equipage_alesage(epine),
        longueur: EQUIPAGE_COLLIER,
    };
    asm.ajouter(cuire(Repere::new(Vec3::Y * y_centre, orient), &collier));

    // Deux bras **séparés**, partant de la jaquette du collier : une poutre
    // unique de part en part ne passerait pas, l'alésage devant rester vide.
    // Le bras comble ce que le collier ne prend pas : rétrécir le tambour
    // **allonge** le bras d'autant, et la demi-envergure (donc le rayon de
    // rotation) ne bouge pas. C'est `EQUIPAGE_BRAS` qui tient la silhouette.
    let r_collier = EQUIPAGE_COLLIER_RAYON;
    let bras_long = EQUIPAGE_BRAS - r_collier;
    let angle = repli.clamp(0.0, 1.0) * FRAC_PI_2;
    let base = Vec3::Y * y_centre;

    for cote in [1.0_f32, -1.0] {
        let dehors = if cote > 0.0 { Vec3::X } else { Vec3::NEG_X };
        let axe = axe_pivot(dehors);
        // Le bras pivote **autour de la charnière**, donc autour du point où il
        // rejoint la jaquette — pas autour du centre du collier.
        let plie = Quat::from_axis_angle(axe, angle);
        let dir = plie * dehors; // direction du bras une fois replié
        let pivot = dehors * r_collier;

        // Charnière au pivot : son +Z local suit le bras **déployé**, son +X
        // l'axe d'articulation. Sa partie mobile se replie toute seule, du même
        // angle que le bras — les deux lisent donc la même valeur.
        let hx = axe;
        let hz = dehors;
        let hy = hz.cross(hx);
        let rot_charniere = orient * Quat::from_mat3(&Mat3::from_cols(hx, hy, hz));
        let charniere = Composant::Charniere { taille: EQUIPAGE_CHARNIERE, repli };
        asm.ajouter(cuire(Repere::new(orient * pivot + base, rot_charniere), &charniere));

        // Bras en treillis **triangulaire** : trois longerons, c'est la section
        // qui ne se déforme pas — et elle se distingue du carré employé partout
        // ailleurs sur le vaisseau.
        // Section divisée par deux comme le reste : `Profil` étant discret,
        // ça se fait d'un cran (P1 → P0, soit 1,0 → 0,5) et non par un facteur.
        let bras = Composant::Treillis {
            profil: Profil::P0,
            longueur: bras_long,
            style: StyleTreillis::Triangulaire,
        };
        let rot = orient * Quat::from_rotation_arc(Vec3::Z, dir);
        let milieu = orient * (pivot + dir * (bras_long * 0.5)) + base;
        asm.ajouter(cuire(Repere::new(milieu, rot), &bras));

        // Module au bout, axe **vers l'extérieur** (son +Z fuit la charnière).
        let module = Composant::ModuleEquipage {
            profil: Profil::P0,
            longueur: EQUIPAGE_LONG,
            hublots: 8,
        };
        let pos = orient * (pivot + dir * bras_long) + base;
        asm.ajouter(cuire(Repere::new(pos, rot), &module));
    }
}

// --- Boucliers de tête -------------------------------------------------------
/// Rayon (circonradius) de la **petite** plaque de tête.
///
/// Cette cote a une contrainte qu'on ne devine pas en la regardant : c'est elle
/// qui fixe le **plus petit alésage de la pile**, donc la section maximale du
/// mât commun (`rayon × MOYEU × ALESAGE` = 0,396 ici). Un mât plus gros ne
/// passerait pas la petite plaque, quand bien même il passerait les trois
/// grandes — voir [`BOUCLIER_MAT`].
const BOUCLIER_PETIT_RAYON: f32 = 5.5;
/// Rayon de la **grande** plaque, mesuré sur son petit axe. La hauteur vaut
/// `rayon × élancement × 2` (≈ 26) et la largeur `rayon × √3 × étroitesse`
/// (≈ 13,9) : c'est la hauteur qui fait le gabarit, et elle se compare au
/// diamètre du vaisseau aux radiateurs (25,2).
const BOUCLIER_GRAND_RAYON: f32 = 10.0;
/// Nombre de grandes plaques. Le schéma en demande **trois identiques** derrière
/// la petite : c'est un étagement, et un étage ne se lit pas.
const BOUCLIER_GRANDS: usize = 3;
/// Espacement entre plaques le long du mât. C'est **la** cote qui blinde — elle
/// donne au nuage de plasma la place de s'étaler avant la plaque suivante. Sur
/// le vrai ISV c'est ~100 m ; ici on garde un rapport lisible à l'échelle du
/// modèle.
const BOUCLIER_ECART: f32 = 8.0;

/// Profil du **mât** qui enfile les quatre plaques.
///
/// P0 et pas plus gros : sa section transversale mesurée est 0,290, contre un
/// alésage de 0,396 sur la petite plaque. Le cran au-dessus (0,58) ne passerait
/// pas, et l'ouvrir en agrandissant la petite plaque la porterait à un gabarit
/// que le schéma ne montre pas. Élancement du mât ≈ 100:1, exactement celui de
/// la flèche d'épine qu'il prolonge — donc rien d'anormal à l'œil.
const BOUCLIER_MAT: Profil = Profil::P0;

/// Jeu entre la fin du raccord conique et la première plaque : de quoi voir le
/// mât sortir de son col avant qu'une plaque s'y enfile.
const BOUCLIER_JEU: f32 = 0.9;

/// Position de la **première** plaque (la petite) le long de l'axe, au gabarit
/// final : juste après le raccord posé au sommet d'épine.
///
/// **Déduite et non écrite en dur.** Elle valait 95,0 en clair, ce qui marchait
/// tant que l'épine gardait sa longueur — et la laissait en place si on la
/// changeait, la tête restant plantée dans le vide à distance du vaisseau. C'est
/// exactement le défaut que le gabarit d'épine (`Epine::hors_tout`) évite pour
/// la charge utile ; il n'y a pas de raison que la tête y échappe. Trouvé en
/// vérifiant rouge le test de proportions : raccourcir l'épine de moitié ne
/// raccourcissait pas le vaisseau.
const BOUCLIER_DEBUT_Y: f32 = EPINE_SOMMET_Y * ISV_ECHELLE + BOUCLIER_RACCORD + BOUCLIER_JEU;

/// Longueur du **raccord conique** entre le sommet d'épine et le mât.
///
/// Il n'est pas décoratif. La flèche d'épine finit à 0,9 de rayon et le mât fait
/// 0,29 : posés bout à bout, la section chute d'un facteur **trois d'un coup**,
/// et la tête a l'air rapportée plutôt que portée. Le raccord ramène ça à deux
/// marches franches — 0,9 → 1,0 (léger débord, jamais une face coplanaire) puis
/// 1,0 → 0,5 sur sa longueur — après quoi le mât sort de son col.
///
/// C'est aussi ce qui borne `BOUCLIER_DEBUT_Y` par le bas : le col du raccord
/// fait 0,5 de rayon, plus que l'alésage de la petite plaque (0,396). Poser la
/// plaque avant la fin du raccord la ferait empaler dessus.
const BOUCLIER_RACCORD: f32 = 2.4;

/// Pose la **tête de bouclier** : le mât, la petite plaque, puis les trois
/// grandes espacées de [`BOUCLIER_ECART`].
///
/// **À quel bout du vaisseau** : à l'opposé des moteurs, sur le haut d'épine
/// libre. La question traînait depuis §C.8, où nos propres notes se
/// contredisaient (§C.2 plaçait le bouclier « en avant », donc côté moteurs,
/// puisque l'ISV est un tracteur ; §C.4 et §C.6 le mettaient en haut d'épine).
/// Le schéma d'assemblage tranche : il montre les quatre plaques **au bout
/// opposé aux radiateurs**, après la charge utile. C'est aussi le seul bout
/// dégagé — côté moteurs il faudrait composer avec les tuyères et des ailes de
/// rayon 12,6.
///
/// Les plaques sont **perpendiculaires à l'axe**, et leur grand axe est mis dans
/// le **plan des radiateurs** : les deux extrémités du vaisseau se lisent alors
/// dans la même vue de profil, comme sur le schéma. Sans la rotation
/// supplémentaire d'un quart de tour, le grand axe tomberait perpendiculaire aux
/// ailes et la tête paraîtrait plate de trois quarts.
fn poser_tete_bouclier(asm: &mut Assembleur, base_y: f32, axe: Quat) {
    // Un quart de tour autour de l'axe de la plaque : met son grand axe dans le
    // plan des ailes radiateur.
    let pose = axe * Quat::from_rotation_z(FRAC_PI_2);

    let petite = Composant::BouclierPetit { profil: Profil::P1, rayon: BOUCLIER_PETIT_RAYON };
    asm.ajouter(cuire(Repere::new(vec3(0.0, base_y, 0.0), pose), &petite));

    let grande = Composant::BouclierGrand {
        profil: Profil::P1,
        rayon: BOUCLIER_GRAND_RAYON,
        elancement: BOUCLIER_ELANCEMENT,
    };
    for k in 1..=BOUCLIER_GRANDS {
        let y = base_y + k as f32 * BOUCLIER_ECART;
        asm.ajouter(cuire(Repere::new(vec3(0.0, y, 0.0), pose), &grande));
    }

    // Raccord conique au sommet d'épine, grand bout (−Z local) contre elle.
    let pied = EPINE_SOMMET_Y * ISV_ECHELLE;
    let raccord = Composant::Adaptateur {
        grand: Profil::P1,
        petit: BOUCLIER_MAT,
        longueur: BOUCLIER_RACCORD,
    };
    asm.ajouter(cuire(
        Repere::new(vec3(0.0, pied + BOUCLIER_RACCORD * 0.5, 0.0), axe),
        &raccord,
    ));

    // Mât : il part du **sommet d'épine** et non de la première plaque — sinon
    // la tête flotterait devant le vaisseau sans rien qui l'y rattache — et
    // dépasse d'un demi-écart au-delà de la dernière, comme un vrai longeron
    // qu'on ne coupe pas à ras de sa dernière ferrure. Sa première section est
    // noyée dans le raccord, qui est plein : le mât en **sort**, il n'y est pas
    // simplement accolé.
    let bout = base_y + BOUCLIER_GRANDS as f32 * BOUCLIER_ECART + BOUCLIER_ECART * 0.5;
    let mat = Composant::Treillis {
        profil: BOUCLIER_MAT,
        longueur: bout - pied,
        style: StyleTreillis::Triangulaire,
    };
    asm.ajouter(cuire(Repere::new(vec3(0.0, (pied + bout) * 0.5, 0.0), axe), &mat));
}

// --- Bouclier thermique d'épine ---------------------------------------------
// Le bardage se monte **au droit des moteurs**, et s'arrête peu après les
// tuyères : c'est là qu'est la chaleur. Il ne couvre pas le long tronçon nu, qui
// n'a rien à parer et que le bardage ferait lire comme une gaine technique.
//
// Toutes les cotes ci-dessous sont relevées sur l'épine assemblée, dont le rayon
// hors-tout décroît ainsi le long de l'axe : 3,75 à X = −12, 2,81 à −9, 2,28 à
// −6, 1,84 à −3, 1,53 à 0, 1,32 à +3, puis 1,15 constant au-delà de +12.

/// Début du bardage : au pied des tuyères, qui s'étendent de −9,8 à −1,3.
///
/// **Pas plus bas**, et la raison est mesurée : entre −12 et −10,5 le rayon de
/// l'épine tombe de 3,75 à 2,85 d'un seul coup, puis se remet à décroître
/// doucement. Aucune loi en puissance ne suit ce décrochement, et un bardage qui
/// part de −12 flotte de **0,93** au-dessus de la poutre trois unités plus loin
/// (mesuré). À partir de −9 le profil est régulier et l'exposant 2,0 le suit à
/// deux centièmes près.
const THERMIQUE_DEBUT_Y: f32 = -9.0;
/// Fin du bardage. Les tuyères s'arrêtent à −1,3 : le bardage les dépasse de
/// cinq unités et s'arrête là. Au-delà, il n'y a plus rien à parer.
const THERMIQUE_FIN_Y: f32 = 4.0;
/// Rayon au **pied** (côté moteurs), pris au **circonradius** comme celui de
/// l'épine (2,81 au même endroit).
///
/// ⚠️ Il ne suffit pas de dépasser 2,81. Les deux hexagones — celui du bardage
/// et celui du treillis — n'ont **aucune raison d'être calés sur la même
/// orientation** : le bardage a son repère propre, l'épine tient le sien de
/// `repere(axe)`. Dans le pire cas ils sont décalés d'un demi-pas, et c'est
/// alors le **milieu de facette** du bardage (son rayon inscrit, 0,866 × le
/// circonradius) qui passe au droit d'un **longeron** de l'épine. C'est ce
/// pincement qui se voyait à l'écran, alors même que les circonradius
/// mesuraient un jeu positif partout.
///
/// Les rayons sont donc dimensionnés sur le **cas le plus défavorable** :
/// `circonradius_bardage × 0,866 ≥ circonradius_épine + jeu`. D'où le facteur
/// 1,155 appliqué aux cotes relevées — c'est le prix de ne pas dépendre d'un
/// calage angulaire que rien ne garantit.
const THERMIQUE_RAYON_PIED: f32 = 3.50;
/// Rayon au **bout**, au circonradius. L'épine y mesure ≈ 1,28.
const THERMIQUE_RAYON_BOUT: f32 = 1.70;
/// Courbure de l'évasement, ajustée pour que le bardage **épouse** l'épine.
///
/// Elle est réglée par la mesure et non par le calcul : la loi du treillis est
/// écrite depuis la base de la charpente, alors que le bardage commence dix
/// unités plus haut, et le pied en pavillon s'ajoute par-dessus. Refaire
/// l'algèbre donnerait une formule juste pour une pièce qui n'est pas tout à
/// fait celle qu'on habille.
///
/// Ajustée sur les rayons relevés **majorés du facteur de calage** (voir
/// [`THERMIQUE_RAYON_PIED`]), elle tombe à **1,7 ± 0,05** aux trois tranches du
/// milieu : le profil de l'épine *est* une loi en puissance sur cette portion,
/// ce qui n'allait pas de soi. Le jeu résultant, mesuré au pire cas, tient entre
/// **+0,05 et +0,09** sur toute l'emprise — vérifié par
/// `le_bardage_thermique_epouse_lepine`.
const THERMIQUE_COURBURE: f32 = 1.5;
/// Nombre de rangs d'écailles. Réglé pour que le pas (≈ 1,3) reste inférieur à
/// la largeur d'une facette, y compris au bout où elle n'est plus que de 1,4.
const THERMIQUE_RANGS: usize = 10;

/// Le bardage tel qu'il est monté sur l'ISV. Sorti en fonction pour que la
/// vitrine et le vaisseau montrent **la même pièce** : c'est tout l'intérêt de
/// la méthode brique-d'abord, et une vitrine qui diverge du montage ne valide
/// plus rien.
fn bouclier_thermique() -> Composant {
    Composant::BouclierThermique {
        rayon_pied: THERMIQUE_RAYON_PIED,
        rayon_bout: THERMIQUE_RAYON_BOUT,
        courbure: THERMIQUE_COURBURE,
        longueur: THERMIQUE_FIN_Y - THERMIQUE_DEBUT_Y,
        rangs: THERMIQUE_RANGS,
    }
}

/// Vue briques : le **bouclier thermique d'épine** — le bardage d'écailles
/// imbriquées qui pare le rayonnement des tuyères.
///
/// En haut un tronçon court, pour juger une écaille : sa saillie, sa lèvre
/// sombre, et le sens du recouvrement. En dessous le tronçon tel qu'il est monté
/// sur le vaisseau, enfilé sur un bout d'épine hexagonale — c'est le seul moyen
/// de vérifier ce qui compte vraiment, à savoir que le bardage **plaque** sur la
/// poutre au lieu de flotter autour.
pub fn demo_bouclier_thermique() -> EtatStation {
    let mut asm = Assembleur::new();
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);

    let court = Composant::BouclierThermique {
        rayon_pied: 3.0,
        rayon_bout: 2.6,
        courbure: THERMIQUE_COURBURE,
        longueur: 9.0,
        rangs: 4,
    };
    asm.ajouter(cuire(Repere::new(vec3(-4.5, 12.0, 0.0), couche), &court));

    // Monté sur son épine : bardage et poutre au même endroit et au même
    // gabarit, sans quoi on ne juge que le bardage seul — et ce qui compte est
    // qu'il **épouse** l'évasement au lieu de flotter dessus.
    let long = THERMIQUE_FIN_Y - THERMIQUE_DEBUT_Y;
    let epine = Composant::CharpenteHexa {
        grand: Profil::P3,
        petit: Profil::P1,
        longueur: long * 1.6,
        courbure: 2.6,
        pied: PiedHexa::Aucun,
    };
    asm.ajouter(cuire(Repere::new(vec3(0.0, -6.0, 0.0), couche), &epine));
    asm.ajouter(cuire(
        Repere::new(vec3(-long * 0.8, -6.0, 0.0), couche),
        &bouclier_thermique(),
    ));

    asm.terminer()
}

/// Vue briques : la **petite plaque de tête**, présentée sur ses deux faces.
///
/// À gauche la **face avant** (+Z, côté poussière), striée ; à droite la même
/// plaque retournée, donc sa **face arrière** et son ossature. Les deux côte à
/// côte dans le même plan : c'est la seule mise en scène qui laisse juger les
/// deux faces d'un même point de vue, comme sur le schéma. En bas la plaque par
/// la **tranche**, pour voir l'épaisseur et le débord des nervures.
///
/// **Pas encore posée sur l'ISV** : brique d'abord, assemblage ensuite.
pub fn demo_bouclier_petit() -> EtatStation {
    let mut asm = Assembleur::new();
    let plaque = Composant::BouclierPetit { profil: Profil::P1, rayon: BOUCLIER_PETIT_RAYON };
    let ecart = BOUCLIER_PETIT_RAYON * 1.35;

    asm.ajouter(cuire(Repere::new(vec3(-ecart, 3.0, 0.0), Quat::IDENTITY), &plaque));
    asm.ajouter(cuire(
        Repere::new(vec3(ecart, 3.0, 0.0), Quat::from_rotation_y(std::f32::consts::PI)),
        &plaque,
    ));
    asm.ajouter(cuire(
        Repere::new(
            vec3(0.0, -BOUCLIER_PETIT_RAYON * 1.6, 0.0),
            Quat::from_rotation_arc(Vec3::Z, Vec3::X),
        ),
        &plaque,
    ));

    asm.terminer()
}

/// Vue briques : la **grande plaque de tête** — l'hexagone étiré, miroir bleuté
/// uni sur ses deux faces, portant le motif du schéma : huit rayons partant du
/// moyeu et deux barres transversales qui détachent les deux pointes.
///
/// À gauche de face, à droite par la tranche : c'est de profil que se lit le
/// nœud papillon des nervures, épaisses au moyeu et effilées à la jante. En bas
/// les **trois** plaques telles qu'elles seront enfilées sur le mât, pour juger
/// l'espacement — qui est ce qui blinde, bien plus que l'épaisseur des plaques.
///
/// Le mât est le même que sur le vaisseau ([`BOUCLIER_MAT`]) : c'est la petite
/// plaque, et non les grandes, qui borne sa section.
pub fn demo_bouclier_grand() -> EtatStation {
    let mut asm = Assembleur::new();
    let plaque = Composant::BouclierGrand {
        profil: Profil::P1,
        rayon: BOUCLIER_GRAND_RAYON,
        elancement: BOUCLIER_ELANCEMENT,
    };
    let haut = BOUCLIER_GRAND_RAYON * BOUCLIER_ELANCEMENT;

    asm.ajouter(cuire(
        Repere::new(vec3(-BOUCLIER_GRAND_RAYON * 1.3, haut * 1.15, 0.0), Quat::IDENTITY),
        &plaque,
    ));
    asm.ajouter(cuire(
        Repere::new(
            vec3(BOUCLIER_GRAND_RAYON * 1.6, haut * 1.15, 0.0),
            Quat::from_rotation_arc(Vec3::Z, Vec3::X),
        ),
        &plaque,
    ));

    // La pile, couchée le long de +X : les trois plaques enfilées, plus le mât
    // qui les traverse.
    let base = vec3(-BOUCLIER_ECART, -haut * 1.05, 0.0);
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);
    for k in 0..BOUCLIER_GRANDS {
        asm.ajouter(cuire(
            Repere::new(base + Vec3::X * (k as f32 * BOUCLIER_ECART), couche),
            &plaque,
        ));
    }
    let mat = Composant::Treillis {
        profil: BOUCLIER_MAT,
        longueur: (BOUCLIER_GRANDS - 1) as f32 * BOUCLIER_ECART + BOUCLIER_ECART * 0.5,
        style: StyleTreillis::Triangulaire,
    };
    asm.ajouter(cuire(
        Repere::new(base - Vec3::X * (BOUCLIER_ECART * 0.25), couche),
        &mat,
    ));

    asm.terminer()
}

/// Vue briques : la **section d'équipage rotative** de l'ISV — la seule partie
/// tournante du vaisseau. En haut un module seul (fût cylindrique, plancher
/// bombé vers l'extérieur, couronne de hublots au niveau du pont) ; en dessous
/// l'ensemble déployé : traverse et un module à chaque bout, tournés vers
/// l'extérieur, tel qu'il sera posé sur l'épine.
pub fn demo_equipage(repli: f32) -> EtatStation {
    let mut asm = Assembleur::new();

    let seul = Composant::ModuleEquipage { profil: Profil::P0, longueur: EQUIPAGE_LONG, hublots: 8 };
    asm.ajouter(cuire(Repere::new(vec3(0.0, 7.0, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::X)), &seul));

    // Orientée **comme sur le vaisseau** (collier le long de Y, l'axe de
    // l'épine), et pas à plat : c'est ce qui fait que l'axe de rotation de la
    // vue est le même ici et là-bas — sinon le bouton ferait tourner la
    // maquette autour du mauvais axe.
    poser_equipage(&mut asm, 0.0, Quat::from_rotation_arc(Vec3::Z, Vec3::Y), repli, EPINE);
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

    // Le repli doit amener le bras **le long de l'épine**, et le déploiement le
    // remettre radial — c'est tout l'intérêt du geste. Les deux bras pivotent
    // autour d'axes opposés, donc se rabattent du même côté, pas en ciseaux.
    #[test]
    fn le_repli_couche_les_bras_le_long_de_lepine() {
        for dehors in [Vec3::X, Vec3::NEG_X] {
            let axe = axe_pivot(dehors);
            // Déployé : le bras reste radial.
            let d0 = Quat::from_axis_angle(axe, 0.0) * dehors;
            assert!((d0 - dehors).length() < 1e-5);
            // Replié : le bras est aligné sur l'épine (+Z du repère de grappe),
            // et du **bon côté** — pas rabattu vers les moteurs.
            let d1 = Quat::from_axis_angle(axe, FRAC_PI_2) * dehors;
            assert!(
                (d1 - Vec3::Z).length() < 1e-5,
                "bras {dehors:?} replié vers {d1:?} au lieu de +Z"
            );
            // À mi-course, il est bien en diagonale (l'animation passe par là).
            let dm = Quat::from_axis_angle(axe, FRAC_PI_2 * 0.5) * dehors;
            assert!(dm.z > 0.6 && dm.dot(dehors) > 0.6);
        }
    }

    // La section d'équipage est montée **entre l'habitat et le bout de l'épine**.
    // Ce créneau est étroit et il n'est délimité par rien de visible : trop bas,
    // le collier tourne dans l'habitat fixe ; trop haut, il tourne autour de
    // rien. Ces deux bornes ne se voient qu'en tournant la caméra, d'où le test.
    #[test]
    fn la_section_dequipage_se_glisse_entre_lhabitat_et_le_bout_de_lepine() {
        // ⚠️ **Deux unités à ne pas mélanger.** Les `*_CENTRE_Y` sont des
        // positions le long de l'épine et sont multipliées par `ISV_ECHELLE` à la
        // pose ; les longueurs de composants, elles, ne le sont **pas** (la
        // charge utile garde sa taille, cf. `la_charge_utile_suit_le_gabarit_de_
        // lepine`). Comparer un centre brut à une demi-longueur donne un résultat
        // faux — conservateur ici, mais qui dériverait si `ISV_ECHELLE` bougeait.
        let hab_sommet = HAB_CENTRE_Y * ISV_ECHELLE + HAB_LONG * 0.5;
        let centre = EQUIPAGE_CENTRE_Y * ISV_ECHELLE;
        let collier_bas = centre - EQUIPAGE_COLLIER * 0.5;
        let collier_haut = centre + EQUIPAGE_COLLIER * 0.5;
        let epine_sommet = EPINE_SOMMET_Y * ISV_ECHELLE;

        assert!(
            collier_bas > hab_sommet,
            "le collier descend à {collier_bas} alors que l'habitat monte à {hab_sommet} : \
             la section tournerait dans les modules fixes"
        );
        assert!(
            collier_haut < epine_sommet,
            "le collier monte à {collier_haut}, au-delà du sommet d'épine \
             {epine_sommet} : il n'aurait plus de support"
        );
    }

    // Le collier doit **envelopper** l'épine sans laisser de jour : les deux
    // bornes sont serrées et se contredisent si on les relâche, d'où le test.
    //
    // ⚠️ Ce test dit l'**inverse** de son ancêtre, qui exigeait un jeu franc
    // autour de la flèche comme le voudrait un vrai palier. Cette version-là
    // donnait un anneau de vide visible tout autour, et la pièce avait l'air
    // enfilée sur l'épine plutôt que montée dessus (arbitrage rendu à l'écran le
    // 2026-07-30 : le rendu l'emporte sur le mécanisme). Ne pas « réparer » le
    // sens de ces assertions sans avoir regardé la vue.
    #[test]
    fn le_collier_dequipage_enveloppe_lepine_sans_jour() {
        let r_collier = EQUIPAGE_COLLIER_RAYON;

        // Borne basse : l'alésage passe **sous** le hors-tout de la flèche, donc
        // sous sa section inscrite. Les membrures mordent dans la paroi du
        // tambour et il ne reste rien à voir entre les deux.
        assert!(
            EQUIPAGE_ALESAGE < EPINE,
            "alésage {EQUIPAGE_ALESAGE:.3} ≥ épine {EPINE:.3} : le jour redevient visible"
        );
        // Borne haute : la jaquette doit dépasser le hors-tout de la flèche,
        // sinon les longerons ressortent **à travers** le collier.
        assert!(
            r_collier > EPINE,
            "jaquette {r_collier:.3} ≤ épine {EPINE:.3} : les longerons traversent le collier"
        );
        // Et il reste de la matière : un tambour n'est pas une feuille.
        assert!(
            r_collier - EQUIPAGE_ALESAGE > 0.5,
            "paroi de collier trop mince ({:.3})",
            r_collier - EQUIPAGE_ALESAGE
        );
        // Les bras partent de la jaquette : l'envergure doit la dépasser, sinon
        // les modules seraient posés dans le collier lui-même.
        assert!(
            EQUIPAGE_BRAS > r_collier,
            "envergure {EQUIPAGE_BRAS} ≤ rayon de collier {r_collier}"
        );
    }

    // La tête est un **étagement**, pas un blindage feuilleté : c'est le vide
    // entre plaques qui laisse au nuage de plasma la place de s'étaler. Une pile
    // resserrée ne pare plus rien à 0,7 c. Ce test garde donc le rapport
    // écart/plaque, qui est la seule chose que le montage doit *dire*.
    #[test]
    fn la_tete_reste_un_etagement() {
        assert!(
            BOUCLIER_GRANDS >= 3,
            "moins de 3 grandes plaques : l'étagement ne se lit plus"
        );
        // L'espacement doit rester du même ordre que la plaque, pas un joint.
        let rapport = BOUCLIER_ECART / BOUCLIER_GRAND_RAYON;
        assert!(
            rapport > 0.5,
            "écart {BOUCLIER_ECART} pour un rayon {BOUCLIER_GRAND_RAYON} (rapport {rapport:.2}) : \
             les plaques se touchent presque, l'étagement ne se lit plus"
        );
        // Mais la pile doit rester une **tête**, pas un fuselage : plus longue
        // que haute et elle cesse de lire comme un bouclier.
        let prof = (BOUCLIER_GRANDS - 1) as f32 * BOUCLIER_ECART;
        let haut = 2.0 * BOUCLIER_GRAND_RAYON * BOUCLIER_ELANCEMENT;
        assert!(
            prof < haut,
            "pile longue de {prof:.1} pour une plaque haute de {haut:.1} : c'est un fût, plus une tête"
        );
        // Et la petite plaque doit rester la **petite** : c'est elle qui prend
        // le premier choc, et le schéma la veut nettement en retrait.
        assert!(
            BOUCLIER_PETIT_RAYON < BOUCLIER_GRAND_RAYON * 0.85,
            "petite plaque de {BOUCLIER_PETIT_RAYON} contre {BOUCLIER_GRAND_RAYON} : elles ne se distinguent plus"
        );
    }

    // La section a été **divisée par deux** le 2026-07-30 parce qu'elle écrasait
    // la silhouette. Ce test garde la trace du gabarit retenu : c'est un choix
    // d'échelle validé à l'œil, donc exactement le genre de valeur qu'une
    // retouche ultérieure risque de défaire sans s'en apercevoir.
    #[test]
    fn la_section_dequipage_reste_a_lechelle_du_vaisseau() {
        let demi_envergure = EQUIPAGE_BRAS + EQUIPAGE_LONG;
        // Elle doit rester **sous** la moitié de l'ancien gabarit (16), à la
        // constante près qu'impose le collier.
        assert!(
            demi_envergure < 10.0,
            "demi-envergure {demi_envergure} : la section redomine la silhouette"
        );
        // Mais rester lisible : une section qui ne dépasse plus du fret n'a plus
        // l'air de tourner autour de quoi que ce soit.
        assert!(
            demi_envergure > FRET_RAYON,
            "demi-envergure {demi_envergure} sous le rayon du fret {FRET_RAYON:.3}"
        );
        // Bras et module gardent leur proportion d'origine (≈ 1:1) : c'est ce
        // rapport qui fait lire « nacelle au bout d'une traverse ».
        let bras_utile = EQUIPAGE_BRAS - EQUIPAGE_COLLIER_RAYON;
        let rapport = bras_utile / EQUIPAGE_LONG;
        assert!(
            (0.7..1.4).contains(&rapport),
            "bras {bras_utile} / module {EQUIPAGE_LONG} = {rapport:.2}, hors du 1:1 voulu"
        );
    }

    // La charge utile garde sa taille, mais elle est **calée sur le gabarit de
    // l'épine** : si `ISV_ECHELLE` bouge, rayons de couronne et portées de
    // ferrure doivent suivre tout seuls. C'est exactement ce qui a été raté une
    // fois — l'épine élargie de 20 % et les ferrures d'habitat encore calculées
    // pour l'ancienne, donc plantées **dans** la structure.
    #[test]
    fn la_charge_utile_suit_le_gabarit_de_lepine() {
        // **Les deux variantes d'épine sont vérifiées.** L'hexagonale est 3,2 %
        // plus large hors-tout : c'est peu, et c'est justement l'ordre de
        // grandeur qui replante la charge utile dans la structure sans qu'on le
        // remarque (§C.6). Boucler ici est ce qui garantit qu'ajouter une
        // variante d'épine ne peut pas passer sans recaler les couronnes.
        for epine in [Epine::Carree, Epine::Hexagonale] {
            let g = epine.hors_tout();
            let nom = epine.nom();

            // Fret : le creux central de la triforce laisse passer l'épine.
            let creux = fret_rayon(g) - FRET_NACELLE * (0.5 + 0.5 * 0.22);
            assert!(creux >= g, "{nom} : creux du fret {creux:.3} < épine {g:.3}");

            // Habitat : le longeron de ferrure vient **sur** l'épine — ni enfoncé
            // dedans, ni suspendu dans le vide.
            let rail = hab_rayon(g) - HAB_INSCRIT - HAB_ATTACHE * 0.5;
            assert!(
                (rail - g).abs() < 1e-3,
                "{nom} : ferrure d'habitat à {rail:.3}, épine à {g:.3}"
            );
            // ...et le fût lui-même reste à distance de l'épine.
            let face = hab_rayon(g) - HAB_INSCRIT;
            assert!(face > g, "{nom} : l'habitat mord l'épine ({face:.3} vs {g:.3})");

            // Collier d'équipage : sa jaquette doit toujours dépasser la flèche,
            // sinon les longerons ressortent à travers (cf. §C.8). C'est la marge
            // la plus serrée du vaisseau, et l'épine hexagonale la rogne encore.
            assert!(
                EQUIPAGE_COLLIER_RAYON > g,
                "{nom} : jaquette de collier {EQUIPAGE_COLLIER_RAYON} <= épine {g:.3}"
            );
            // ...et son alésage doit rester **sous** la flèche, pour qu'aucun jour
            // ne se rouvre entre le tambour et la structure.
            assert!(
                equipage_alesage(g) < g,
                "{nom} : alésage de collier {:.3} >= épine {g:.3}",
                equipage_alesage(g)
            );

            // **Les trois modules ne se gênent pas entre eux.** Support d'une
            // section onigiri P2 dans la direction de sa voisine (à 60° d'un
            // coin) : `dv·cos 30° + ρ`. C'est ce qui dit que le serrage n'est
            // borné que par l'épine — sans ce calcul, le « plancher » du jeu ne
            // serait qu'une supposition.
            let (dv, rho) = (2.0 * (1.0 - 0.22), 2.0 * 0.22);
            let portee = dv * (PI / 6.0).cos() + rho;
            let entre_axes = 2.0 * hab_rayon(g) * (PI / 3.0).sin();
            assert!(
                entre_axes > 2.0 * portee,
                "{nom} : modules d'habitat qui se croisent ({entre_axes:.3} <= {:.3})",
                2.0 * portee
            );
        }
    }

    // Les deux cuves d'un même côté ne doivent plus se traverser. Cote purement
    // sphérique : deux sphères de rayon `r` se touchent à `2r`, il faut donc
    // davantage.
    #[test]
    fn les_cuves_de_carburant_ne_se_traversent_pas() {
        let res_long = 5.0_f32;
        let res_r = res_long * 0.5 * 1.3;
        let ecart = 2.0 * res_r + RESERVOIR_JEU;
        assert!(
            ecart > 2.0 * res_r,
            "écart {ecart:.3} pour des cuves de rayon {res_r:.3} : elles se traversent"
        );
        // Et l'ancien écart, celui des plaques hexagonales, est bien insuffisant :
        // c'est ce qui justifie d'avoir découplé les deux cotes.
        let sg = Profil::P3.rayon() * 0.5;
        let ap = 2.0 * sg * 3.0_f32.sqrt() * 0.5;
        assert!(
            2.0 * ap < 2.0 * res_r,
            "l'écart des plaques ({:.3}) suffirait aux cuves : le découplage serait inutile",
            2.0 * ap
        );
    }

    // **La propulsion doit maintenant TOUCHER le pied de l'épine**, pas le dégager.
    //
    // ⚠️ Ce test a changé de sens **deux fois**, et chaque fois délibérément — ce
    // n'est pas une dérive, c'est l'intention de conception qui a bougé :
    //   1. « jeu franc > 0,4 » — la propulsion pendait à côté du pied et le
    //      traversait, il fallait l'écarter ;
    //   2. « pas de recouvrement franc > −0,25 » — déport resserré, contact toléré ;
    //   3. **ici** : engagement **exigé**, la propulsion devant se lire comme montée
    //      sur l'épine et non suspendue à côté.
    // Ne pas « corriger » le sens de ces assertions sans avoir regardé la vue.
    //
    // Mesuré sur la géométrie cuite du preset, jamais sur les cotes : le rayon
    // interne de l'aile n'est pas déductible du déport (cf. `PROPULSION_DEPORT`).
    #[test]
    fn la_propulsion_touche_le_pied_de_lepine() {
        use crate::vaisseau::maillage::Batisseur;
        let EtatStation::Prete(s) = preset_isv_hexa() else { panic!("preset vide") };

        // Le modèle est couché : l'axe du vaisseau est X, le rayon se lit en Y-Z.
        let mut pied: Vec<Vec3> = Vec::new(); // sommets du pied, côté moteurs
        let mut aile: Vec<Vec3> = Vec::new(); // sommets des ailes radiateur
        for p in s.pieces() {
            let charpente = matches!(&p.composant, Composant::CharpenteHexa { .. });
            let radiateur = matches!(&p.composant, Composant::RadiateurMega { .. });
            if !charpente && !radiateur {
                continue;
            }
            let mut b = Batisseur::new();
            p.composant.dessiner(&mut b);
            for lot in b.terminer() {
                for v in &lot.vertices {
                    let w = p
                        .transforme
                        .transform_point3(vec3(v.position[0], v.position[1], v.position[2]));
                    // Bande du pied : au-delà de la base du cône, côté tuyères.
                    if charpente && w.x < -10.0 {
                        pied.push(w);
                    }
                    if radiateur {
                        aile.push(w);
                    }
                }
            }
        }

        // ⚠️ **Les deux mesures doivent porter sur la même tranche du vaisseau.**
        // Comparer le rayon minimal de l'aile au rayon maximal du pied *sans* cette
        // précaution rend un « engagement » qui n'existe pas dès que les deux pièces
        // ne sont plus à la même hauteur : c'est exactement ce que produit
        // `PROPULSION_AVANCE`, qui fait glisser l'aile sous le fût. On restreint donc
        // au **recouvrement axial** des deux, et son absence est en soi un échec.
        let borne = |v: &[Vec3], f: fn(f32, f32) -> f32, init: f32| {
            v.iter().fold(init, |acc, w| f(acc, w.x))
        };
        let bas = borne(&pied, f32::max, f32::MIN).min(borne(&aile, f32::max, f32::MIN));
        let haut = borne(&pied, f32::min, f32::MAX).max(borne(&aile, f32::min, f32::MAX));
        // Et ce recouvrement doit rester **substantiel**. Réduit à un liseré, il ne
        // porte plus que quelques sommets de bord et la mesure part en vrille : à
        // `PROPULSION_AVANCE = 1,5` la bande tombe à 0,1 et le « jeu » bascule à
        // +1,34, c'est-à-dire l'inverse de la réalité. Une mesure sur trop peu de
        // matière est pire qu'aucune mesure.
        assert!(
            bas - haut > 0.3,
            "recouvrement axial de {:.2} seulement (bande [{haut:.1} ; {bas:.1}]) :              l'aile a glissé sous le fût, la mesure n'a plus de sens",
            bas - haut
        );

        let pied_max = pied
            .iter()
            .filter(|w| (haut..=bas).contains(&w.x))
            .fold(0.0f32, |m, w| m.max(w.yz().length()));
        let radia_min = aile
            .iter()
            .filter(|w| (haut..=bas).contains(&w.x))
            .fold(f32::MAX, |m, w| m.min(w.yz().length()));
        let jeu = radia_min - pied_max;
        // **Elle mord dans le pied** : c'est cet engagement qui la fait lire comme
        // montée sur l'épine. Un simple affleurement (−0,11, l'état précédent) ne
        // suffit pas — de trois quarts, il laisse encore voir un jour entre les deux.
        assert!(
            jeu < -0.3,
            "engagement de {jeu:.2} — radiateur à {radia_min:.2}, pied à {pied_max:.2} : \
             la propulsion effleure l'épine au lieu de s'y poser"
        );
        // **Mais elle ne l'avale pas.** Au-delà, l'aile traverserait le fût de part
        // en part et ressortirait de l'autre côté.
        assert!(
            jeu > -3.0,
            "engagement de {jeu:.2} : l'aile traverse le pied au lieu de s'y ancrer"
        );
    }

    // Les tests ci-dessus vérifient les **formules**. Celui-ci vérifie le
    // **câblage** : que `isv()` passe bien le gabarit de son épine à la charge
    // utile, au lieu de lire la constante carrée. C'est une erreur invisible aux
    // tests de formules — et c'est littéralement le bug de §C.6, où le calcul
    // était juste mais appliqué à l'ancien gabarit.
    #[test]
    fn le_second_isv_recale_vraiment_sa_charge_utile() {
        let cotes = |e: EtatStation| {
            let EtatStation::Prete(s) = e else { panic!("preset vide") };
            let mut ratelier = None;
            let mut alesage = None;
            let mut hexa = false;
            let mut carree = false;
            for p in s.pieces() {
                match &p.composant {
                    Composant::RatelierCargo { rayon, .. } => ratelier = Some(*rayon),
                    Composant::CollierRotatif { alesage: a, .. } => alesage = Some(*a),
                    Composant::CharpenteHexa { .. } => hexa = true,
                    Composant::Charpente { .. } => carree = true,
                    _ => {}
                }
            }
            (ratelier.expect("pas de fret"), alesage.expect("pas de collier"), hexa, carree)
        };

        let (r_c, a_c, hexa_c, carree_c) = cotes(preset_isv());
        let (r_h, a_h, hexa_h, carree_h) = cotes(preset_isv_hexa());

        // Chacun porte bien la charpente qu'il annonce, et **seulement** elle.
        assert!(carree_c && !hexa_c, "le premier ISV n'est plus en épine carrée");
        assert!(hexa_h && !carree_h, "le second ISV n'est pas en épine hexagonale");

        // La couronne de fret et l'alésage du collier ont suivi le gabarit.
        let g = Epine::Hexagonale.hors_tout();
        assert!(r_h > r_c, "couronne de fret non recalée ({r_h:.4} vs {r_c:.4})");
        assert!((r_h - fret_rayon(g)).abs() < 1e-4, "fret à {r_h:.4}, attendu {:.4}", fret_rayon(g));
        assert!(a_h > a_c, "alésage de collier non recalé ({a_h:.4} vs {a_c:.4})");
        assert!((a_h - equipage_alesage(g)).abs() < 1e-4);

        // Et le premier ISV est resté sur ses cotes historiques.
        assert!((r_c - FRET_RAYON).abs() < 1e-4);
        assert!((a_c - EQUIPAGE_ALESAGE).abs() < 1e-4);
    }

    // Les constantes historiques doivent rester **exactement** celles de l'épine
    // carrée : le second preset ne doit rien changer au premier. Sans ça, on
    // comparerait deux vaisseaux qui diffèrent par autre chose que leur épine.
    #[test]
    fn le_gabarit_carre_est_inchange_par_lajout_de_lhexagonal() {
        assert!((Epine::Carree.hors_tout() - EPINE).abs() < 1e-6);
        assert!((fret_rayon(EPINE) - FRET_RAYON).abs() < 1e-6);
        assert!((hab_rayon(EPINE) - HAB_RAYON).abs() < 1e-6);
        assert!((equipage_alesage(EPINE) - EQUIPAGE_ALESAGE).abs() < 1e-6);
        // Et l'hexagonale est bien **plus large**, sinon tout ce recalage serait
        // du vent.
        let h = Epine::Hexagonale.hors_tout();
        assert!(h > EPINE, "épine hexagonale {h:.4} pas plus large que {EPINE:.4}");
        let ecart = h / EPINE - 1.0;
        assert!(
            (0.02..0.05).contains(&ecart),
            "écart de gabarit {:.1} %, hors du ~3 % attendu",
            ecart * 100.0
        );
    }

    // **Le panache ne doit pas lécher le vaisseau.** C'est la raison d'être du
    // braquage de 5° des tuyères, décidé bien avant qu'il y ait un panache à
    // regarder : l'ISV est un tracteur, il remorque sa charge utile **dans l'axe
    // de ses propres jets**, et sans ce braquage il la baignerait dans un plasma
    // de pions. Un panache trop ouvert annule le braquage et ramène le problème.
    //
    // Rien de tout ça ne se voit sur le composant seul : c'est un rapport entre
    // trois choses — l'angle des tuyères, l'évasement du jet, et le gabarit de
    // ce qui est remorqué derrière.
    #[test]
    fn le_panache_ne_leche_pas_la_charge_utile() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 1.0) else {
            panic!("l'ISV doit être prête");
        };
        // Axe du jet de chaque tuyère, dans le repère du vaisseau fini.
        let jets: Vec<(Vec3, Vec3)> = s
            .pieces()
            .iter()
            .filter(|p| matches!(p.composant, Composant::Panache { .. }))
            .map(|p| {
                (
                    p.transforme.transform_point3(Vec3::ZERO),
                    p.transforme.transform_vector3(Vec3::Z).normalize(),
                )
            })
            .collect();
        assert_eq!(jets.len(), 2, "un panache par tuyère");

        // Tout ce qui est **remorqué** : la charge utile et la tête, c'est-à-dire
        // ce qui se trouve en aval des moteurs. L'ossature de propulsion est
        // exclue — le jet en sort, il la frôle forcément.
        let remorque: Vec<(Vec3, f32)> = s
            .pieces()
            .iter()
            .filter(|p| {
                matches!(
                    p.composant,
                    Composant::RatelierCargo { .. }
                        | Composant::ModuleHabitat { .. }
                        | Composant::ModuleEquipage { .. }
                        | Composant::BouclierPetit { .. }
                        | Composant::BouclierGrand { .. }
                )
            })
            .map(|p| (p.centre(), p.composant.rayon_local()))
            .collect();
        assert!(remorque.len() >= 8, "{} pièces remorquées : trop peu", remorque.len());

        let mut pire = f32::MAX;
        let mut coupable = Vec3::ZERO;
        for (origine, axe) in &jets {
            for (centre, rayon) in &remorque {
                // Distance de la pièce à l'axe du jet, moins le rayon du jet à
                // cette distance-là : c'est le jeu réel entre les deux volumes.
                let le_long = (*centre - *origine).dot(*axe);
                if le_long <= 0.0 {
                    continue; // en amont de la tuyère : le jet ne va pas par là
                }
                let ecart = (*centre - *origine - *axe * le_long).length();
                let t = le_long / (PANACHE_LONGUEUR);
                let demi_jet = crate::vaisseau::composant::rayon_panache(
                    Profil::P1.rayon() / 0.40 * PANACHE_COL,
                    PANACHE_BOUT,
                    t,
                );
                let jeu = ecart - demi_jet - rayon;
                if jeu < pire {
                    pire = jeu;
                    coupable = *centre;
                }
            }
        }
        assert!(
            pire > 1.0,
            "le panache passe à {pire:.1} d'une pièce remorquée (centre {coupable:?}) : \
             il la baigne au lieu de la contourner"
        );
    }

    // Le bardage se monte **au droit des moteurs**, donc sur la portion où
    // l'épine s'ouvre vers son pied — de 2,81 à 1,28 de rayon sur les treize
    // unités couvertes. Il doit donc l'**épouser**, et il n'y a que deux façons
    // de rater ça, opposées et toutes deux voyantes : trop serré, l'épine
    // ressort au travers ; trop lâche, le bardage flotte autour comme une gaine.
    //
    // ⚠️ La comparaison qui compte est **la surface la plus rentrante du bardage
    // contre la plus saillante de l'épine**, et non leurs deux circonradius. Les
    // deux hexagones ne sont calés sur aucune orientation commune : au pire ils
    // sont décalés d'un demi-pas, et c'est alors le **milieu de facette** du
    // bardage (rayon inscrit = 0,866 × circonradius) qui passe au droit d'un
    // **longeron** de l'épine. Le premier jet de ce test comparait les maxima
    // des deux pièces, trouvait un jeu positif partout — et le pincement se
    // voyait quand même à l'écran.
    #[test]
    fn le_bardage_thermique_epouse_lepine() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 0.0) else {
            panic!("l'ISV doit être prête");
        };
        // Sommets d'une famille, casés par tranche de 0,5 le long de l'axe. Le
        // maillage cuit n'a de sommets qu'aux bords de ses facettes : une
        // tranche plus fine tomberait dans le vide et ne mesurerait rien.
        let par_tranche = |f: &dyn Fn(&Composant) -> bool| -> Vec<(f32, f32, f32)> {
            let mut v: Vec<(f32, f32, f32)> = Vec::new();
            for p in s.pieces().iter().filter(|p| f(&p.composant)) {
                let mut b = crate::vaisseau::maillage::Batisseur::new();
                b.poser_transforme(p.transforme);
                p.composant.dessiner(&mut b);
                for lot in b.terminer() {
                    for vt in &lot.vertices {
                        let w = vec3(vt.position[0], vt.position[1], vt.position[2]);
                        let case = (w.x / 0.5).floor() * 0.5;
                        let r = w.yz().length();
                        match v.iter_mut().find(|(c, _, _)| (*c - case).abs() < 1e-3) {
                            Some(e) => {
                                e.1 = e.1.min(r);
                                e.2 = e.2.max(r);
                            }
                            None => v.push((case, r, r)),
                        }
                    }
                }
            }
            v
        };
        let epine = par_tranche(&|c| matches!(c, Composant::CharpenteHexa { .. }));

        // Le bardage est pris par sa **section de calcul** et non par ses
        // sommets. C'est la surface qui vient se plaquer sur la poutre ; ses
        // lèvres, relevées par construction, ne touchent rien et fausseraient la
        // mesure — une tranche qui ne contient qu'une lèvre lit un rayon bien
        // trop grand et fait croire à un bardage qui flotte.
        let long = THERMIQUE_FIN_Y - THERMIQUE_DEBUT_Y;
        // Rayon **inscrit** : le bardage ne peut pas s'approcher de l'axe plus
        // près que ça, quel que soit le calage angulaire des deux hexagones.
        let inscrit = 3.0_f32.sqrt() * 0.5;

        let (mut jeu_max, mut jeu_min) = (f32::MIN, f32::MAX);
        let (mut pire_x, mut pire) = (0.0f32, 0.0f32);
        let mut mesures = 0;
        for (x, _, _) in epine.iter() {
            if !(THERMIQUE_DEBUT_Y..=THERMIQUE_FIN_Y).contains(x) {
                continue;
            }
            // Rayon de l'épine pris sur une **fenêtre** d'une baie et non sur la
            // seule tranche : certaines tranches ne coupent que des diagonales et
            // lisent un rayon anormalement bas, ce qui ferait croire à un
            // bardage qui flotte là où il est simplement en face d'un vide.
            let epine_max = epine
                .iter()
                .filter(|(c, _, _)| (c - x).abs() <= 1.5)
                .fold(0.0f32, |m, (_, _, r)| m.max(*r));
            let t = (x - THERMIQUE_DEBUT_Y) / long;
            let bardage = crate::vaisseau::composant::section_bardage(
                THERMIQUE_RAYON_PIED,
                THERMIQUE_RAYON_BOUT,
                THERMIQUE_COURBURE,
                t,
            );
            let jeu = bardage * inscrit - epine_max;
            if jeu > jeu_max {
                (pire_x, pire) = (*x, jeu);
            }
            jeu_max = jeu_max.max(jeu);
            jeu_min = jeu_min.min(jeu);
            mesures += 1;
        }
        assert!(mesures >= 6, "{mesures} tranches mesurées : trop peu pour conclure");
        assert!(
            jeu_min > 0.05,
            "pincement de {:.2} : le bardage mord sur l'épine",
            -jeu_min
        );
        assert!(
            jeu_max < 0.40,
            "le bardage flotte de {pire:.2} au-dessus de l'épine vers X = {pire_x:.0}"
        );
    }

    // **Proportions d'ensemble.** Ce qui fait qu'un ISV lit comme un ISV et non
    // comme une fusée quelconque tient à trois rapports, et à rien d'autre : un
    // très long tronçon d'épine **nu**, une charge utile dominée par son fret, et
    // un vaisseau qui reste franchement plus long que large. Chacun des trois se
    // perd par accumulation de petites retouches, sans qu'aucune ne soit fautive
    // prise isolément — d'où ce test, qui les mesure sur le vaisseau assemblé.
    //
    // Les bornes sont larges à dessein : elles ne prescrivent pas une silhouette,
    // elles interdisent de la perdre.
    #[test]
    fn les_proportions_densemble_de_lisv_tiennent() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 0.0) else {
            panic!("l'ISV doit être prête");
        };
        // Étendue axiale et rayon d'une famille de pièces, mesurés sur la
        // géométrie cuite : les `rayon_local()` ignorent l'échelle portée par la
        // transformée de l'ossature (cf. §C.1) et sous-estimeraient tout.
        let etendue = |f: &dyn Fn(&Composant) -> bool| -> (f32, f32, f32) {
            let (mut a, mut z, mut r) = (f32::MAX, f32::MIN, 0.0f32);
            for p in s.pieces().iter().filter(|p| f(&p.composant)) {
                let mut b = crate::vaisseau::maillage::Batisseur::new();
                b.poser_transforme(p.transforme);
                p.composant.dessiner(&mut b);
                for lot in b.terminer() {
                    for v in &lot.vertices {
                        let w = vec3(v.position[0], v.position[1], v.position[2]);
                        a = a.min(w.x);
                        z = z.max(w.x);
                        r = r.max(w.yz().length());
                    }
                }
            }
            (a, z, r)
        };

        let (x0, x1, rayon) = etendue(&|_| true);
        let long = x1 - x0;

        // La **propulsion** est un bloc à un bout : moteurs, nacelles, cuves,
        // ailes. On en prend le bout le plus éloigné des tuyères.
        let (_, prop_fin, _) = etendue(&|c| {
            matches!(
                c,
                Composant::MoteurAntimatiere { .. }
                    | Composant::ReacteurAntimatiere { .. }
                    | Composant::RadiateurMega { .. }
                    | Composant::BlocMoteur { .. }
                    | Composant::Reservoir { .. }
            )
        });
        // La **charge utile** est un bloc à l'autre bout, la tête exclue.
        let (util_debut, util_fin, _) = etendue(&|c| {
            matches!(
                c,
                Composant::RatelierCargo { .. }
                    | Composant::ModuleHabitat { .. }
                    | Composant::ModuleEquipage { .. }
            )
        });
        let (fret_a, fret_b, _) = etendue(&|c| matches!(c, Composant::RatelierCargo { .. }));
        let (hab_a, hab_b, _) = etendue(&|c| matches!(c, Composant::ModuleHabitat { .. }));

        // 1. L'**épine nue** entre les deux blocs : la signature du vaisseau.
        //    C'est une poutre en **tension**, et ce qui le dit est sa longueur à
        //    vide — pas le treillis, qui court d'un bout à l'autre.
        let nu = util_debut - prop_fin;
        assert!(
            nu > long * 0.25,
            "épine nue de {nu:.1} sur {long:.1} ({:.0} %) : le vaisseau n'a plus sa longue poutre en tension",
            nu / long * 100.0
        );
        let utile = util_fin - util_debut;
        assert!(
            nu > utile,
            "épine nue de {nu:.1} contre {utile:.1} de charge utile : la poutre ne domine plus"
        );

        // 2. Le **fret** domine la charge utile. Le vrai ISV porte 4 rangées de
        //    4 modules : c'est la plus grosse masse embarquée, et si l'habitat
        //    lui dispute la place, le vaisseau cesse d'être un cargo.
        let (f, h) = (fret_b - fret_a, hab_b - hab_a);
        assert!(
            f > h * 1.8,
            "fret {f:.1} contre habitat {h:.1} : le vaisseau ne lit plus comme un cargo"
        );

        // 3. **Élancement.** Le diamètre hors-tout est pris aux ailes radiateur ;
        //    en deçà de six fois, la silhouette s'épaissit et l'ISV commence à
        //    ressembler à une station.
        let elancement = long / (2.0 * rayon);
        assert!(
            elancement > 6.0,
            "élancement {elancement:.1} ({long:.1} de long pour {:.1} de diamètre)",
            2.0 * rayon
        );
    }

    // **À quel bout va la tête de bouclier** — la question ouverte depuis §C.8,
    // où nos propres notes se contredisaient. Tranchée par le schéma
    // d'assemblage : au bout **opposé aux moteurs**, après toute la charge utile,
    // sur le haut d'épine libre. Ce test fige la réponse, parce qu'elle ne se
    // déduit d'aucune autre et qu'un déplacement de rangée pourrait la brouiller.
    #[test]
    fn la_tete_de_bouclier_coiffe_le_bout_oppose_aux_moteurs() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 0.0) else {
            panic!("l'ISV doit être prête");
        };
        let x = |f: &dyn Fn(&Composant) -> bool| -> Vec<f32> {
            s.pieces().iter().filter(|p| f(&p.composant)).map(|p| p.centre().x).collect()
        };
        let moteurs = x(&|c| matches!(c, Composant::MoteurAntimatiere { .. }));
        let petite = x(&|c| matches!(c, Composant::BouclierPetit { .. }));
        let grandes = x(&|c| matches!(c, Composant::BouclierGrand { .. }));
        let habitat = x(&|c| matches!(c, Composant::ModuleHabitat { .. }));
        assert_eq!(petite.len(), 1, "une seule petite plaque en tête");
        assert_eq!(grandes.len(), BOUCLIER_GRANDS, "trois grandes plaques");

        let moteur_max = moteurs.iter().fold(f32::MIN, |m, v| m.max(*v));
        let hab_max = habitat.iter().fold(f32::MIN, |m, v| m.max(*v));
        let tete_min = petite[0].min(grandes.iter().fold(f32::MAX, |m, v| m.min(*v)));
        assert!(
            tete_min > hab_max,
            "tête à {tete_min:.1} : elle n'est pas au-delà de l'habitat ({hab_max:.1})"
        );
        assert!(
            tete_min > moteur_max + 80.0,
            "tête à {tete_min:.1} : pas franchement à l'opposé des moteurs ({moteur_max:.1})"
        );
        // La **petite** plaque vient en premier, les trois grandes derrière : le
        // schéma est explicite là-dessus, et l'ordre a un sens — c'est la plaque
        // sacrificielle qui prend le grain, les grandes encaissent le nuage.
        assert!(
            grandes.iter().all(|g| *g > petite[0]),
            "la petite plaque ({:.1}) n'est pas en tête des grandes ({grandes:?})",
            petite[0]
        );
        // Et rien de la tête ne redescend sur l'épine, qui est déjà occupée.
        let sommet = EPINE_SOMMET_Y * ISV_ECHELLE;
        assert!(
            tete_min > sommet,
            "tête à {tete_min:.1}, sommet d'épine à {sommet:.1} : les plaques mordent sur l'épine"
        );
    }

    // Les quatre plaques sont **enfilées sur un mât commun**. C'est donc la
    // **plus petite** qui borne la section du mât, pas les grandes : son alésage
    // est proportionnel à son rayon, et il est le plus étroit de la pile.
    //
    // Le piège est qu'un mât trop gros ne se voit pas — il traverse le moyeu et
    // ressort de l'autre côté sans que rien ne l'arrête, et la pile a l'air
    // enfilée alors qu'elle est empalée. On mesure donc les deux sur la
    // géométrie cuite, plutôt que de refaire le calcul des fractions ici.
    #[test]
    fn le_mat_de_tete_passe_le_plus_petit_alesage() {
        let transversal = |c: &Composant, garder: &dyn Fn(f32) -> bool| -> (f32, f32) {
            let mut b = crate::vaisseau::maillage::Batisseur::new();
            c.dessiner(&mut b);
            let (mut mini, mut maxi) = (f32::MAX, 0.0f32);
            for lot in b.terminer() {
                for v in &lot.vertices {
                    if !garder(v.position[2]) {
                        continue;
                    }
                    let r = vec2(v.position[0], v.position[1]).length();
                    mini = mini.min(r);
                    maxi = maxi.max(r);
                }
            }
            (mini, maxi)
        };

        // Alésage : les sommets du moyeu sont les seuls à vivre loin du plan
        // médian de la plaque.
        let seuil = BOUCLIER_PETIT_RAYON * 0.05;
        let (alesage, _) = transversal(
            &Composant::BouclierPetit { profil: Profil::P1, rayon: BOUCLIER_PETIT_RAYON },
            &|z| z.abs() > seuil,
        );
        let (_, mat) = transversal(
            &Composant::Treillis {
                profil: BOUCLIER_MAT,
                longueur: 20.0,
                style: StyleTreillis::Triangulaire,
            },
            &|_| true,
        );
        assert!(
            mat < alesage - 0.05,
            "mât de {mat:.3} pour un alésage de {alesage:.3} : il n'enfile pas la petite plaque, il la traverse"
        );
        // Et pas ridiculement fin non plus : un mât qui flotte dans son alésage
        // ne porte visiblement rien.
        assert!(
            mat > alesage * 0.5,
            "mât de {mat:.3} pour un alésage de {alesage:.3} : il flotte dans le moyeu"
        );
    }

    // L'ISV est un **tracteur** : les moteurs sont à un bout, la charge utile à
    // l'**autre**, au bout d'une longue épine en tension. C'est la décision de
    // conception la plus facile à casser par inadvertance en déplaçant une
    // rangée — on la verrouille ici. (Le modèle est couché : l'axe du vaisseau
    // est X après le pivot final.)
    #[test]
    fn isv_porte_son_fret_a_loppose_des_moteurs() {
        let EtatStation::Prete(s) = preset_isv() else {
            panic!("l'ISV doit être prête");
        };
        let moyenne_x = |f: &dyn Fn(&Composant) -> bool| -> Option<f32> {
            let v: Vec<f32> = s
                .pieces()
                .iter()
                .filter(|p| f(&p.composant))
                .map(|p| p.centre().x)
                .collect();
            (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32)
        };
        let fret = moyenne_x(&|c| matches!(c, Composant::RatelierCargo { .. })).expect("des rangées de fret");
        let moteurs = moyenne_x(&|c| matches!(c, Composant::MoteurAntimatiere { .. })).expect("des moteurs");
        let habitat = moyenne_x(&|c| matches!(c, Composant::ModuleHabitat { .. })).expect("de l'habitat");
        assert!(
            fret > moteurs + 30.0,
            "fret ({fret:.1}) pas nettement à l'opposé des moteurs ({moteurs:.1})"
        );
        // **Ordre des sections** le long de l'épine : moteurs → fret → habitat.
        // L'habitat est ce qu'on éloigne le plus des tuyères ; s'il repassait
        // devant le fret, c'est tout le sens de la disposition qui tomberait.
        assert!(
            habitat > fret,
            "habitat ({habitat:.1}) pas au-delà du fret ({fret:.1})"
        );

        // Les rangées sont **enfilées sur l'épine**, pas déportées sur un côté :
        // leur centre reste sur l'axe du vaisseau.
        for p in s.pieces() {
            if matches!(p.composant, Composant::RatelierCargo { .. }) {
                let c = p.centre();
                assert!(c.y.abs() < 1e-3 && c.z.abs() < 1e-3, "rangée hors axe : {c:?}");
            }
        }
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










