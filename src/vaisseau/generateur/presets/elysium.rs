use crate::vaisseau::montage::cuire;
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, Piece, Profil, Repere,
    Station,
};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use super::isv::EPINE_LONGUEUR;
use super::stanford::{bras_et_tunnels, elysium_rayon_jante, habitats_accouples, hauteur_moyeu, moyeu_elysium};

// --- Elysium ---------------------------------------------------------------

/// Longueur d'un bras d'Elysium : **la moitié de l'épine de l'ISV**.
///
/// Dérivée, jamais recopiée : le jour où l'épine du vaisseau bouge, les bras
/// suivent. C'est la leçon d'`Epine::hors_tout` (§C.5), où une cote recopiée à
/// la main avait planté la charge utile dans la structure.
pub(crate) const ELYSIUM_BRAS: f32 = EPINE_LONGUEUR * 0.5;

/// Bras du **tore de Stanford** : la moitié de ceux d'Elysium, donc le quart
/// de l'épine de l'ISV.
///
/// Les deux longueurs coexistent le temps de trancher à l'écran — même
/// dispositif que les deux `Epine` de l'ISV (§C.9), et pour la même raison :
/// une proportion d'ensemble ne se juge pas sur un nombre, elle se regarde.
pub(crate) const STANFORD_BRAS: f32 = ELYSIUM_BRAS * 0.5;

/// Nombre de bras d'Elysium : **six**, une face sur six de l'hexagone du moyeu
/// au lieu d'une sur deux ([`STANFORD_N_BRAS`]). L'hexagone en a exactement six
/// — Elysium prend donc chaque face, Stanford une sur deux.
pub(crate) const ELYSIUM_N_BRAS: usize = 6;

/// Nombre de bras du tore de Stanford — inchangé, une face sur deux.
pub(crate) const STANFORD_N_BRAS: usize = 3;

/// Longueur d'un module de tunnel. Le **nombre** s'en déduit de la longueur du
/// bras, si bien qu'un bras deux fois plus long porte deux fois plus de modules
/// au lieu de modules deux fois plus longs.
pub(crate) const TUNNEL_MODULE: f32 = 3.0;

/// Longueur d'**un** des deux habitats du complexe Elysium ; accouplés, ils
/// font le double. Plus long que la hauteur du prisme (le côté de l'hexagone) :
/// l'ensemble dépasse de part et d'autre, sans quoi on ne le verrait pas.
pub(crate) const ELYSIUM_HABITAT: f32 = 13.5;


/// Réduction de l'habitat du **tore de Stanford** : un tiers de moins
/// qu'Elysium, **dans toutes ses dimensions**.
///
/// Appliquée **géométriquement**, en composant une mise à l'échelle dans les
/// transformées cuites, et non en raccourcissant la cote : le diamètre d'un
/// module vient de `Profil`, une grille discrète (P1 = 1, P2 = 2) où rien ne
/// vaut 0,67 × P2. Ne réduire que `longueur` aurait allongé la silhouette au
/// lieu de la rapetisser — c'est le même constat qui a fait mettre l'ossature
/// de l'ISV à l'échelle plutôt que de multiplier ses cotes (`ISV_ECHELLE`).
pub(crate) const STANFORD_ECHELLE: f32 = 2.0 / 3.0;

/// Profil des habitats du moyeu.
///
/// **P2 et non P3**, alors que P3 serait le triplement littéral de l'ancien P1.
/// `Profil` est une grille discrète, et le cran du dessus ne passe pas : le
/// tambour d'un `Coeur` vaut `rayon × 1,16`, soit **3,48** en P3 — au-delà du
/// bord extérieur des barres du moyeu (**3,35**). L'habitat avalerait
/// l'hexagone au lieu d'être ceinturé par lui. En P2 le tambour vaut 2,32 et se
/// loge entre les barres (1,85 → 3,35).
///
/// C'est le cas de figure de §C.8 : une cote qui se règle **contre une autre
/// pièce** ne tombe pas sur la grille. Ici on a préféré rester sur la grille et
/// s'arrêter au cran qui tient, plutôt que sortir le rayon de `Profil`.
pub(crate) const HABITAT_PROFIL: Profil = Profil::P2;

/// Profil du moyeu **et** de la base des bras. Les deux doivent être le même :
/// c'est ce qui fait que la base carrée d'une `Charpente` (2·sg) tombe
/// **exactement** sur la face d'un `TreillisHexagone` (côté = 2·sg).
pub(crate) const ELYSIUM_PROFIL: Profil = Profil::P3;

/// **Complexe Elysium** : ossature à bras longs — un moyeu en prisme hexagonal,
/// trois bras d'épine à 120° dans son plan.
///
/// Trois choix, tous mesurés plutôt que supposés :
///
/// 1. **Le moyeu est un prisme**, pas un anneau plat : deux `TreillisHexagone`
///    parés d'un jeu de montants (`liaison`). Sa hauteur vaut le **côté** de
///    l'hexagone, ce qui rend ses six faces latérales **carrées** — d'où des
///    faces sur lesquelles une base de bras se pose sans rattrapage.
/// 2. **Bras en `Charpente` carrée**, pas `CharpenteHexa`. Ce n'est pas un
///    repli : la base carrée mesure `2·sg`, soit **exactement** le côté de
///    l'hexagone (`charpente_dessiner` le dit : « une extrémité du cône fait
///    exactement la taille d'une face extérieure de l'hexagone »). La variante
///    hexagonale reprend le circonradius du carré (×√2) pour garder la même
///    silhouette, ce qui la rend **22,5 % trop large entre plats** (3,67 contre
///    3,00) : elle déborderait la face au lieu de s'y asseoir.
/// 3. **Sur les faces, pas sur les sommets.** Un hexagone a six faces ; trois
///    bras à 120° en prennent une sur deux, et une face est un appui plan
///    quand un sommet est une arête.
///
/// ⚠️ Les bras ne sont **pas reliés par une membrure courbe** — la conception en
/// prévoit une joignant les longerons d'un bras à l'autre autour du moyeu, et
/// aucune primitive du projet ne trace une poutre à **axe courbe**
/// (`treillis_conique` courbe sa *section*, pas son axe). C'est une brique à
/// écrire, donc une étape à part.
///
/// **Six bras**, pas trois : Elysium prend **toutes** les faces de l'hexagone du
/// moyeu au lieu d'une sur deux (§F.1). Un premier essai d'anneau (une jante de
/// modules d'habitat, un tiers de tour) a été jugé à l'écran et **retiré** ;
/// à la place, un tour **complet** de [`Composant::Jante`] — canal ouvert
/// façon jante de vélo, un sixième par bras, refermé sur les six
/// ([`jante_elysium`]).
///
/// Bâtie en interne autour de **Y** (bras et jante dans le plan XZ — voir
/// [`bras_et_tunnels`]), puis **pivotée globalement** d'un quart de tour
/// autour de X ([`tournee`]) pour présenter sa face — son propre axe de
/// rotation — côté **Z**, plutôt que de réécrire chacune de ses fonctions de
/// placement autour d'un second axe interne.
pub fn preset_elysium() -> EtatStation {
    let mut asm = Assembleur::new();
    moyeu_elysium(&mut asm);
    bras_et_tunnels(&mut asm, ELYSIUM_BRAS, ELYSIUM_N_BRAS);
    habitats_accouples(&mut asm, ELYSIUM_HABITAT);
    jante_elysium(&mut asm);
    second_anneau_elysium(&mut asm);
    tournee(asm.terminer(), Quat::from_rotation_x(FRAC_PI_2))
}

/// Rayon majeur du second anneau : **un tiers du diamètre** de la jante
/// extérieure (donc deux tiers de son rayon), lui-même **divisé par deux**
/// sur demande — écrit en deux facteurs plutôt que réduit à
/// `elysium_rayon_jante() / 3.0` pour garder visible que le second anneau a
/// deux origines (le tiers de diamètre, puis la réduction de moitié), pas une
/// seule fraction choisie d'un coup.
pub(crate) fn elysium_rayon_second_anneau() -> f32 {
    (2.0 * elysium_rayon_jante() / 3.0) * 0.5
}

/// **Un second anneau**, entre le moyeu et la jante extérieure : une copie du
/// `Composant::Tore` de Stanford (même construction que
/// [`preset_stanford_anneau`] — section, segments, anneaux), mais **sans**
/// le moyeu ni les bras qui l'accompagnent là-bas : Elysium a déjà les siens,
/// portés par [`moyeu_elysium`]/[`bras_et_tunnels`] — les reposer ici les
/// dupliquerait pour rien.
///
/// Placé à l'origine, comme [`jante_elysium`] : bâti en interne dans le même
/// plan XZ que le reste d'Elysium, il suit le même pivot global vers Z
/// appliqué en fin de [`preset_elysium`], sans rien à ajuster ici.
///
/// Jonctions calées sur les **six** bras d'Elysium (pas les trois de
/// Stanford) : mêmes angles que `bras_et_tunnels`, pour que le vitrage du
/// tore n'arrive pas là où un bras le traverse.
pub(crate) fn second_anneau_elysium(asm: &mut Assembleur) {
    let tore = Composant::Tore {
        rayon_majeur: elysium_rayon_second_anneau(),
        rayon_mineur: hauteur_moyeu() * 0.5,
        segments: 96,
        anneaux: 16,
        jonctions: ELYSIUM_N_BRAS,
        phase: PI / 6.0,
    };
    asm.ajouter(cuire(Repere::IDENTITE, &tore));
}

/// Fait pivoter une station déjà assemblée par un quaternion **global**,
/// autour de l'origine — pour réorienter tout un preset bâti avec sa propre
/// convention d'axe interne sans réécrire chacune de ses fonctions de
/// placement. `Piece::transforme` est déjà une `Mat4` cuite : la pivoter,
/// c'est la prémultiplier — même principe que la rotation clavier de
/// l'assembleur (`Chantier::appliquer_delta_bloc`).
pub(crate) fn tournee(etat: EtatStation, rot: Quat) -> EtatStation {
    let Some(station) = etat.doit_dessiner() else {
        return EtatStation::Vide;
    };
    let delta = Mat4::from_quat(rot);
    let pieces: Vec<Piece> =
        station.pieces().iter().map(|p| Piece::new(delta * p.transforme, p.composant.clone())).collect();
    match Station::depuis_pieces(pieces) {
        Some(s) => EtatStation::Prete(s),
        None => EtatStation::Vide,
    }
}

/// Demi-angle du vide côté moyeu de la jante — voir [`Composant::Jante`].
/// Assez large pour se voir franchement (canal ouvert, pas une fente), assez
/// étroit pour laisser des flancs qui lisent comme des parois et non comme un
/// simple arceau.
pub(crate) const ELYSIUM_JANTE_OUVERTURE: f32 = 0.6;

/// **Le tour complet de la jante d'Elysium** : un sixième de
/// [`Composant::Jante`] par bras, refermé sur les six
/// (`ELYSIUM_N_BRAS`). Posées un temps à cinq exemplaires pour vérifier que
/// le tour se referme symétriquement puis ramenées à un seul le temps de
/// régler la ville qui s'y ajoute (`jante::ville`), les six reprennent leur
/// place une fois celle-ci stable — la formule de rotation n'a pas changé
/// entre-temps, seule la boucle qui la reproduit `ELYSIUM_N_BRAS` fois a
/// été réintroduite.
///
/// `rayon_mineur = hauteur_moyeu()` : la largeur du canal — son étendue le long
/// de Y, entre ses deux flancs (`v = ±π/2`, où `surface_tore` donne
/// `y = ±rayon_mineur`) — **doublée** par rapport au premier essai
/// (`hauteur_moyeu() * 0.5`, le gabarit du tore de Stanford).
///
/// Ni décalage ni recentrage à faire : `v` balaie un intervalle **symétrique**
/// (`±(π − ouverture)`, voir `jante::dessiner`), donc `y = rayon_mineur *
/// sin(v)` reste centré sur 0 quelle que soit la valeur de `rayon_mineur` — la
/// pièce elle-même est déjà posée à `Vec3::ZERO`, sans offset. Doubler le
/// paramètre suffit ; c'est justement ce que garde
/// `la_jante_reste_centree_sur_son_plan` (`jante.rs`).
pub(crate) fn jante_elysium(asm: &mut Assembleur) {
    let comp = Composant::Jante {
        rayon_majeur: elysium_rayon_jante(),
        rayon_mineur: hauteur_moyeu(),
        arc: TAU / ELYSIUM_N_BRAS as f32,
        ouverture: ELYSIUM_JANTE_OUVERTURE,
        segments: 24,
        anneaux: 8,
    };
    // Chaque tronçon démarre à `u = 0` (le +X local) ; tourné autour de Y,
    // il démarre au bras `k` et finit au bras `k + 1`, comme `bras_et_tunnels`
    // les place (`radial = (cos a, 0, sin a)`).
    //
    // ⚠️ `Quat::from_rotation_y` tourne dans le sens **opposé** à cette
    // convention d'angle — un premier essai à `+PI/6` amenait le canal à 330°
    // au lieu de 30°, ce que le test `la_jante_delysium_ferme_le_tour` a pris.
    // D'où le signe négatif, plutôt qu'une deuxième convention d'angle à
    // tenir en tête.
    for k in 0..ELYSIUM_N_BRAS {
        let rot = Quat::from_rotation_y(-(PI / 6.0 + k as f32 * TAU / ELYSIUM_N_BRAS as f32));
        asm.ajouter(cuire(Repere::new(Vec3::ZERO, rot), &comp));
    }
}
