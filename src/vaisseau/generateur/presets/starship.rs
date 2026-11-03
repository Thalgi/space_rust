use crate::vaisseau::montage::cuire;
use crate::vaisseau::symetrie::Symetrie;
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, Profil, Repere, StyleTreillis, ogive_rayon, BARDAGE_ARC, JUPE_COUVERTURE, VarianteModule, VarianteRaptor,
};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

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

    #[allow(dead_code)] // Libelle du catalogue de l'assembleur (Lot 4) : ecrit avec la brique, consomme par l'ecran.
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
/// l'O'Neill en posent plusieurs, de styles et de tailles différentes.
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

/// Vitrine des **deux Raptor**, côte à côte et à leur taille relative réelle.
///
/// Ils ne sont **pas** ramenés à la même taille : c'est justement le rapport
/// entre les deux qu'il faut juger — le RVac fait presque le double de
/// diamètre, et c'est ce qui décide de leur implantation sur le cul du vaisseau.
pub fn demo_raptor() -> EtatStation {
    let mut asm = Assembleur::new();
    // Agrandis d'un facteur commun : à leur taille réelle (0,58 U de sortie)
    // ils sont trop petits pour qu'on juge la powerhead. Le **rapport** entre
    // les deux, lui, est préservé — c'est tout ce qui compte ici.
    let zoom = 4.0;
    let ecart: f32 = VarianteRaptor::TOUTES
        .iter()
        .map(|v| v.rayon_nominal() * zoom)
        .fold(0.0, f32::max)
        * 2.6;
    let n = VarianteRaptor::TOUTES.len();
    for (i, variante) in VarianteRaptor::TOUTES.into_iter().enumerate() {
        let x = (i as f32 - (n as f32 - 1.0) * 0.5) * ecart;
        let m = Composant::Raptor {
            profil: Profil::P0,
            variante,
            rayon: variante.rayon_nominal() * zoom,
        };
        asm.ajouter(cuire(Repere::new(vec3(x, 0.0, 0.0), Quat::IDENTITY), &m));
    }
    asm.terminer()
}

// --- Starship ---------------------------------------------------------------

/// Cotes du Starship, **prises sur l'engin réel** et converties à l'échelle du
/// projet (1 U ≈ 2,25 m) — jamais choisies à l'œil.
///
/// | | réel | ici |
/// |---|---:|---:|
/// | diamètre | 9 m | 4,0 U → rayon **2,0 U = `P2` pile** |
/// | hauteur hors-tout | 50 m | 22,2 U |
/// | ogive | ~11 m | 4,9 U (22 % du total) |
///
/// C'est la première cible du projet dont le gabarit tombe **exactement** sur la
/// grille `Profil` : aucune dérogation à prévoir pour ce qui viendra s'y monter.
pub(crate) const STARSHIP_RAYON: Profil = Profil::P2;
pub(crate) const STARSHIP_HAUTEUR: f32 = 50.0 / 2.25;
pub(crate) const STARSHIP_NEZ: f32 = 11.0 / 2.25;
/// Hauteur de la **jupe moteur**, mesurée depuis le cul du vaisseau.
///
/// Le haut de la jupe est au niveau du sommet du plus grand moteur : c'est là
/// qu'est la structure de poussée, et au-dessus commence le réservoir.
pub(crate) const STARSHIP_JUPE_HAUT: f32 = raptor_hauteur(VarianteRaptor::Vide);

/// Le **bord** inférieur de la jupe, mesuré depuis le cul.
///
/// Réglé par le moteur le plus **court** : c'est lui qui décide, puisqu'il faut
/// couvrir au moins [`JUPE_COUVERTURE`] de *chaque* moteur et qu'à hauteur de
/// couverture égale, le petit est le premier à ressortir.
pub(crate) const STARSHIP_JUPE_BORD: f32 =
    raptor_hauteur(VarianteRaptor::Atmospherique) * (1.0 - JUPE_COUVERTURE);

/// Longueur de la virole de jupe.
pub(crate) const STARSHIP_JUPE: f32 = STARSHIP_JUPE_HAUT - STARSHIP_JUPE_BORD;

/// Fût = tout ce qui n'est ni l'ogive **ni la jupe**.
///
/// ⚠️ La jupe est **prise dans** les 50 m, pas ajoutée dessous. Une fusée se
/// mesure jusqu'au plan de sortie de ses tuyères ; la poser sous une coque déjà
/// haute de 50 m donnerait un engin de 54 m qui n'est plus un Starship.
pub(crate) const STARSHIP_FUT: f32 = STARSHIP_HAUTEUR - STARSHIP_NEZ - STARSHIP_JUPE_HAUT;

/// Hauteur d'un Raptor à son calibre nominal, en `const fn` pour que les cotes
/// de la jupe s'en déduisent au lieu d'être recopiées.
pub(crate) const fn raptor_hauteur(v: VarianteRaptor) -> f32 {
    match v {
        VarianteRaptor::Atmospherique => (1.30 / 2.0 / 2.25) * (3.10 / (1.30 / 2.0)),
        VarianteRaptor::Vide => (2.40 / 2.0 / 2.25) * (4.60 / (2.40 / 2.0)),
    }
}

/// Rayons des **deux couronnes de moteurs**, lus du réel comme le reste.
///
/// | | réel (rayon) | ici |
/// |---|---:|---:|
/// | couronne des 3 RSL (atmosphériques) | 1,10 m | 0,489 U |
/// | couronne des 3 RVac (vide) | 3,10 m | 1,378 U |
///
/// La disposition n'est pas décorative : la cloche de détente 90 d'un RVac fait
/// presque **le double** du diamètre de sortie d'un RSL, elle ne rentre nulle
/// part ailleurs qu'en périphérie. C'est pour ça que les trois moteurs de
/// pilotage — ceux qui sont sur cardan — sont ceux du centre.
pub(crate) const STARSHIP_COURONNE_RSL: f32 = 1.10 / 2.25;
pub(crate) const STARSHIP_COURONNE_RVAC: f32 = 3.10 / 2.25;

/// Décalage angulaire de la couronne extérieure : un **demi-pas** (60°).
///
/// Les six moteurs ne sont pas alignés sur trois rayons. Alignés, les RVac
/// s'inscriraient pile derrière les RSL et le cul perdrait sa lecture — on n'y
/// verrait que trois directions au lieu de six.
pub(crate) const STARSHIP_DEPHASAGE_RVAC: f32 = std::f32::consts::PI / 3.0;

/// **Les six moteurs du Starship** : le composant et son repère, dans le repère
/// de la coque.
///
/// Sortie de [`preset_starship`] pour être **mesurable**. Un placement se
/// vérifie — rien ne dépasse de la jupe, rien ne s'interpénètre, les deux
/// couronnes ne se chevauchent pas — et rien de tout cela n'exige un contexte
/// graphique. Laissé dans le preset, il ne se jugerait qu'à l'œil.
///
/// Le **cul du vaisseau** (`starship_cul`) est le plan de sortie des tuyères :
/// c'est jusque-là qu'on mesure les 50 m, donc rien ne descend plus bas. Les six
/// moteurs y ont tous leur sortie, et montent d'autant qu'ils sont hauts — un
/// RVac dépasse donc un RSL **par le haut**, jamais par le bas.
///
/// Un Raptor crache vers `−Z` dans son propre repère (`raptor.rs`,
/// `z_sortie = −demi`) : aucune rotation n'est nécessaire, il suffit de le
/// remonter d'une demi-hauteur pour poser sa sortie sur ce plan.
pub(crate) fn starship_moteurs() -> Vec<(Composant, Repere)> {
    let cul = starship_cul();
    let mut v = Vec::with_capacity(6);
    for (variante, couronne, phase) in [
        (VarianteRaptor::Atmospherique, STARSHIP_COURONNE_RSL, 0.0),
        (VarianteRaptor::Vide, STARSHIP_COURONNE_RVAC, STARSHIP_DEPHASAGE_RVAC),
    ] {
        let rayon = variante.rayon_nominal();
        let z = cul + variante.hauteur(rayon) * 0.5;
        let comp = Composant::Raptor { profil: Profil::P0, variante, rayon };
        // Une couronne de trois **est** une symétrie radiale : on pose le
        // premier moteur et `cuire_symetrie` fait les deux autres. Recopier
        // trois angles à la main laisserait la porte ouverte à une couronne
        // qui n'est plus régulière sans que rien ne le dise.
        let base = Repere::new(vec3(couronne * phase.cos(), couronne * phase.sin(), z), Quat::IDENTITY);
        for t in Symetrie::Radiale(3).transformations(Vec3::Z, Vec3::Z) {
            let p = t.transform_point3(base.pos);
            v.push((comp.clone(), Repere::new(p, base.rot)));
        }
    }
    v
}

/// **Starship (SpaceX)** — étapes 1 et 2 : la coque, et la grappe de six Raptor.
///
/// Ordre de travail (`docs/conception/stations.md` §D.5) : coque, moteurs,
/// volets, bouclier thermique, hublot. Chaque étape se juge **seule** avant la
/// suivante — c'est la leçon 1 du chantier ISV (§C.29).
pub fn preset_starship() -> EtatStation {
    let mut asm = Assembleur::new();
    let cul = starship_cul();
    let debout = starship_debout();

    // La coque commence **au-dessus de la jupe**, pas au cul : les deux se
    // partagent les 50 m au lieu de s'additionner.
    let coque = Composant::CoqueOgive {
        profil: STARSHIP_RAYON,
        longueur: STARSHIP_FUT,
        nez: STARSHIP_NEZ,
    };
    let z_coque = cul + STARSHIP_JUPE_HAUT + (STARSHIP_FUT + STARSHIP_NEZ) * 0.5;
    asm.ajouter(cuire(debout.compose(Repere::new(vec3(0.0, 0.0, z_coque), Quat::IDENTITY)), &coque));

    let jupe = Composant::JupeMoteur { profil: STARSHIP_RAYON, longueur: STARSHIP_JUPE };
    let z_jupe = cul + (STARSHIP_JUPE_BORD + STARSHIP_JUPE_HAUT) * 0.5;
    asm.ajouter(cuire(debout.compose(Repere::new(vec3(0.0, 0.0, z_jupe), Quat::IDENTITY)), &jupe));

    let mut pieces = starship_moteurs();
    pieces.extend(starship_volets());
    pieces.extend(starship_hublots());
    pieces.push(starship_bardage());
    for (comp, repere) in pieces {
        asm.ajouter(cuire(debout.compose(repere), &comp));
    }
    asm.terminer()
}

/// **Les quatre volets**, cotes prises sur l'engin réel puis converties.
///
/// | | envergure | corde | position (depuis le cul) |
/// |---|---:|---:|---:|
/// | 2 avant (courts) | 4,5 m → 2,00 U | 5,0 m → 2,22 U | **déduite** (voir plus bas) |
/// | 2 arrière (longs) | 7,5 m → 3,33 U | 8,0 m → 3,56 U | 8 m → 3,6 U |
///
/// **Ils ne sont pas à la même hauteur, et c'est la silhouette.** Les courts
/// sont haut sur le fût, les longs juste au-dessus de la jupe ; c'est cet
/// écartement qui fait lire l'engin de profil. Les rapprocher donnerait une
/// croix, pas un Starship.
pub(crate) const VOLET_AVANT_ENVERGURE: f32 = 4.5 / 2.25;
pub(crate) const VOLET_AVANT_CORDE: f32 = 5.0 / 2.25;

/// Jeu entre le haut du volet avant et le raccord fût/ogive.
pub(crate) const VOLET_AVANT_JEU: f32 = 0.4;

/// Cote du volet avant — **déduite, plus lue du réel**.
///
/// ⚠️ Elle valait 41 m, soit 18,2 U, alors que le raccord fût/ogive est à
/// `HAUTEUR − NEZ` = 17,3 U : le volet mordait donc **sur l'ogive**, où la coque
/// se rétrécit déjà. Sa charnière, posée au rayon du fût, flottait au-dessus du
/// vide — ce qui se voit tout de suite de profil.
///
/// Le volet est maintenant calé pour que son **bord haut** reste sous le
/// raccord : une cote qui suit toute retouche de l'ogive au lieu de la
/// contredire.
pub(crate) const VOLET_AVANT_COTE: f32 =
    STARSHIP_HAUTEUR - STARSHIP_NEZ - VOLET_AVANT_CORDE * 0.5 - VOLET_AVANT_JEU;
pub(crate) const VOLET_ARRIERE_ENVERGURE: f32 = 7.5 / 2.25;
pub(crate) const VOLET_ARRIERE_CORDE: f32 = 8.0 / 2.25;
pub(crate) const VOLET_ARRIERE_COTE: f32 = 8.0 / 2.25;

/// **Azimut du ventre du Starship** : la direction que regarde tout ce qui est
/// tuilé — le bardage, et les faces tuilées des quatre volets.
///
/// ⚠️ **Une seule constante, et c'est la correction.** Le côté était écrit deux
/// fois — en dur dans `bardage_dessiner` (`+X`) et dans le signe de la rotation
/// des volets — si bien que « les retourner tous ensemble » n'avait pas de
/// bouton. Les tests ne voyaient qu'un **accord** entre les deux, jamais leur
/// orientation d'ensemble, donc l'engin pouvait présenter son dos à la caméra
/// sans qu'un seul rougisse.
///
/// Seules deux valeurs ont un sens tant que les charnières sont sur `±Y` : le
/// ventre doit rester à 90° d'elles, donc `0` (+X) ou `PI` (−X).
pub(crate) const STARSHIP_VENTRE: f32 = PI;

/// Braquage au repos : **zéro**, volets droits.
///
/// C'est la position de l'engin au sol et en montée — les gouvernes ne se
/// braquent qu'à la rentrée. Un braquage permanent donnait une silhouette
/// affaissée, les quatre volets penchés vers le ventre.
pub(crate) const VOLET_BRAQUAGE: f32 = 0.0;

/// **Les quatre volets** : le composant et son repère, dans le repère du
/// vaisseau.
///
/// Sortie du preset pour la même raison que les moteurs : un placement se
/// mesure. Ils vont **par paires opposées** — c'est une symétrie miroir, et
/// c'est ce qui garantit qu'aucun ne dérive tout seul.
///
/// Le volet est dessiné charnière sur `X`, s'écartant vers `+Y`. Pour le poser
/// sur le flanc, on le tourne autour de `Z` : le `+Y` local devient la normale
/// sortante de la coque, à l'azimut voulu.
pub(crate) fn starship_volets() -> Vec<(Composant, Repere)> {
    let cul = starship_cul();
    let r = STARSHIP_RAYON.rayon();
    let mut v = Vec::with_capacity(4);
    for (envergure, corde, cote) in [
        (VOLET_AVANT_ENVERGURE, VOLET_AVANT_CORDE, VOLET_AVANT_COTE),
        (VOLET_ARRIERE_ENVERGURE, VOLET_ARRIERE_CORDE, VOLET_ARRIERE_COTE),
    ] {
        let comp = Composant::Volet { longueur: envergure, corde, braquage: VOLET_BRAQUAGE };
        let z = cul + cote;
        // Les deux volets d'une paire sont **opposés** : 90° et 270°. Ils
        // encadrent le ventre du vaisseau, qui est la face au vent.
        for (k, azimut) in [FRAC_PI_2, FRAC_PI_2 + PI].into_iter().enumerate() {
            let pos = vec3(r * azimut.cos(), r * azimut.sin(), z);
            // Deux rotations, et l'ordre compte (`R_z * R_y * v_local`).
            //
            // 1. **−90° autour de Y**, dans le repère du volet. Il est dessiné
            //    corde sur `X` et épaisseur sur `Z` ; sans ce quart de tour, la
            //    corde s'enroulerait **autour** de la coque et l'épaisseur se
            //    coucherait le long de l'axe du vaisseau — soit exactement
            //    l'inverse d'un volet. Y étant l'axe d'envergure, il n'est pas
            //    touché : le volet continue de pointer hors de la coque.
            // 2. **azimut − 90° autour de Z**, qui amène ce `+Y` local sur la
            //    normale sortante de la coque, à l'azimut voulu.
            //
            // ⚠️ Et un **demi-tour sur le second de la paire**. Deux volets
            // opposés sont **en miroir** par rapport au plan du ventre, pas en
            // rotation l'un de l'autre : sans ce demi-tour, la simple rotation
            // d'azimut retourne le panneau, et l'un présente son inox là où
            // l'autre présente ses tuiles. Ça ne se voit que sur l'engin monté.
            let demi_tour = if k == 1 { PI } else { 0.0 };
            // Le quart de tour se fait **du côté du ventre** : `−90°` amène la
            // face tuilée sur `+X`, `+90°` sur `−X`. Le signe se lit donc dans
            // `STARSHIP_VENTRE` au lieu d'être écrit ici une seconde fois.
            let quart = -FRAC_PI_2 * STARSHIP_VENTRE.cos();
            let rot = Quat::from_rotation_z(azimut - FRAC_PI_2)
                * Quat::from_rotation_y(quart + demi_tour);
            v.push((comp.clone(), Repere::new(pos, rot)));
        }
    }
    v
}

/// **Le bardage de tuiles** : le composant et son repère.
///
/// Il couvre le **fût entier**, de la jupe au raccord de l'ogive — c'est-à-dire
/// exactement la partie à rayon constant, la seule qu'un manchon cylindrique
/// puisse épouser.
///
/// ⚠️ **C'est lui qui définit le ventre.** Le bardage regarde `+X` (convention de
/// `coque::bardage_dessiner`), et les faces tuilées des volets doivent regarder
/// là aussi : sinon l'engin porte ses tuiles d'un côté et ses volets tuilés de
/// l'autre. Tant qu'il n'existait pas, rien ne pouvait épingler ce côté — c'est
/// la lacune signalée à l'étape 4/5, et elle se referme ici.
pub(crate) fn starship_bardage() -> (Composant, Repere) {
    let z = starship_cul() + STARSHIP_JUPE_HAUT + STARSHIP_FUT * 0.5;
    (
        Composant::BardageTuiles {
            profil: STARSHIP_RAYON,
            longueur: STARSHIP_FUT,
            // ⚠️ **L'ogive est tuilée aussi.** La nappe d'un Starship est
            // continue du fût à la pointe ; s'arrêter au raccord laisserait un
            // nez d'inox nu, qui est exactement la partie qui prend le plasma
            // en premier.
            nez: STARSHIP_NEZ,
            arc: BARDAGE_ARC,
        },
        // Le bardage est dessiné centré sur son `+X` : cette rotation l'amène
        // sur le ventre choisi.
        Repere::new(vec3(0.0, 0.0, z), Quat::from_rotation_z(STARSHIP_VENTRE)),
    )
}

/// Rayon d'un hublot, et leur écartement — deux petits, côte à côte.
pub(crate) const HUBLOT_RAYON: f32 = 0.5 / 2.25;
pub(crate) const HUBLOT_ECART: f32 = 1.4 / 2.25;

/// Où les hublots se posent sur l'ogive, en fraction de sa flèche depuis le
/// raccord. Assez haut pour être sur le nez, assez bas pour que la peau y soit
/// encore large — à la pointe, la coque est trop étroite pour un vitrage.
pub(crate) const HUBLOT_MONTEE: f32 = 0.28;

/// Les **deux hublots**, sur l'ogive, **à l'opposé du ventre**.
///
/// ⚠️ Deux corrections par rapport au premier jet, et les deux se voient :
///
/// 1. **Sur le nez, pas sur le fût.** L'équipage d'un Starship regarde par le
///    nez ; des hublots sur le cylindre seraient des hublots de réservoir.
/// 2. **Côté dos, pas côté ventre.** Le ventre est couvert de tuiles — y percer
///    une baie vitrée reviendrait à trouer le bouclier thermique à l'endroit
///    exact où il prend le plasma.
///
/// Sur une ogive, un vitrage plat ne peut pas être simplement radial : il doit
/// **suivre la pente** de la peau. L'inclinaison est lue sur la courbe elle-même
/// (`ogive_rayon`), par différence finie — pas approchée à l'œil.
pub(crate) fn starship_hublots() -> Vec<(Composant, Repere)> {
    let r_base = STARSHIP_RAYON.rayon();
    let raccord = starship_cul() + STARSHIP_HAUTEUR - STARSHIP_NEZ;
    // Rayon de la peau à `dh` au-dessus du raccord.
    let peau = |dh: f32| ogive_rayon(r_base, STARSHIP_NEZ, STARSHIP_NEZ - dh);

    let dh = STARSHIP_NEZ * HUBLOT_MONTEE;
    let r = peau(dh);

    // Pente locale `dr/dh` (négative : l'ogive se resserre), d'où l'inclinaison
    // de la normale sortante vers la pointe.
    let eps = STARSHIP_NEZ * 0.01;
    let pente = (peau(dh + eps) - peau(dh - eps)) / (2.0 * eps);
    let inclinaison = (-pente).atan();

    // Le dos : à l'opposé du ventre.
    let a = STARSHIP_VENTRE + PI;
    // `+Z` local sur la normale de la peau, inclinaison comprise.
    let rot = Quat::from_rotation_z(a) * Quat::from_rotation_y(FRAC_PI_2 - inclinaison);
    let tangent = vec3(-a.sin(), a.cos(), 0.0);

    [-0.5f32, 0.5]
        .into_iter()
        .map(|k| {
            let centre =
                vec3(r * a.cos(), r * a.sin(), raccord + dh) + tangent * (k * HUBLOT_ECART);
            (Composant::Hublot { rayon: HUBLOT_RAYON }, Repere::new(centre, rot))
        })
        .collect()
}

/// **Le vaisseau debout** : la rotation d'exposition.
///
/// Toutes les pièces du parc se bâtissent le long de `+Z`, et le Starship ne
/// fait pas exception — c'est dans ce repère que ses cotes se lisent et que ses
/// tests raisonnent. Mais une fusée se **regarde debout**, moteurs en bas : un
/// quart de tour autour de `X` amène son axe sur `+Y`.
///
/// ⚠️ Appliquée **à l'assemblage seulement**. Tourner la géométrie à la source
/// obligerait chaque cote et chaque test à raisonner en `Y`, pour un gain nul :
/// le vaisseau ne serait pas plus juste, seulement plus difficile à lire.
pub(crate) fn starship_debout() -> Repere {
    Repere::new(Vec3::ZERO, Quat::from_rotation_x(-FRAC_PI_2))
}

/// **Le cul du vaisseau** : le plan de sortie des tuyères, origine de toutes les
/// cotes verticales du Starship.
///
/// Le vaisseau est centré sur l'origine et mesure [`STARSHIP_HAUTEUR`] : son
/// point le plus bas est donc à `−hauteur/2`. Tout se cale là-dessus — jupe,
/// moteurs, coque — pour qu'aucune pièce ne puisse dériver toute seule.
pub(crate) fn starship_cul() -> f32 {
    -STARSHIP_HAUTEUR * 0.5
}
