//! **Ossature** : la poutre en treillis, la charpente courbe à section variable
//! (l'épine de l'ISV) et l'anneau hexagonal autonome.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI};

use super::commun::*;

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

// --- Poutre ----------------------------------------------------------------

pub(super) fn ports(profil: Profil, longueur: f32) -> Vec<Port> {
    let demi = longueur * 0.5;
    let sx = profil.rayon() * TREILLIS_SECTION; // sortie latérale
    let mut v = vec![
        // Bouts axiaux (chaînables avec modules/nœuds).
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
        Port::new(
            Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)),
            GenrePort::ModuleAxial,
            profil,
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

pub(super) fn dessiner<P: Peintre>(p: &mut P, profil: Profil, longueur: f32, style: StyleTreillis) {
    let demi = longueur * 0.5;
    let sec = profil.rayon() * TREILLIS_SECTION;
    let (a, b) = (vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, demi));
    match style {
        StyleTreillis::Carre => crate::vaisseau::pieces::treillis(p, a, b, sec, COULEUR, SOMBRE),
        StyleTreillis::Triangulaire => {
            crate::vaisseau::pieces::treillis_triangulaire(p, a, b, sec, COULEUR, SOMBRE)
        }
    }
}

pub(super) fn cout(longueur: f32) -> f32 { 2.0 + longueur }

/// Demi-longueur de la poutre (l'extension dominante).
pub(super) fn rayon_local(profil: Profil, longueur: f32) -> f32 {
    longueur * 0.5 + profil.rayon() * TREILLIS_SECTION
}

// --- Charpente -------------------------------------------------------------

pub(super) fn charpente_ports(grand: Profil, petit: Profil, longueur: f32) -> Vec<Port> {
    // Deux bouts axiaux : `petit` en +Z (l'apex étroit), `grand` en
    // −Z (la base évasée), à leurs profils respectifs.
    let demi = longueur * 0.5;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, petit),
        Port::new(Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, grand),
    ]
}

pub(super) fn charpente_dessiner<P: Peintre>(p: &mut P, grand: Profil, petit: Profil, longueur: f32, courbure: f32, aiguille: bool) {
    let demi = longueur * 0.5;
    let sg = grand.rayon() * TREILLIS_SECTION;
    let sp = petit.rayon() * TREILLIS_SECTION;
    crate::vaisseau::pieces::treillis_conique(
        p,
        vec3(0.0, 0.0, -demi),
        vec3(0.0, 0.0, demi),
        sg,
        sp,
        courbure,
        COULEUR,
        SOMBRE,
    );
    if aiguille {
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
        crate::vaisseau::pieces::treillis_hexagone(p, centre, cote, sec, prof, COULEUR, SOMBRE);

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

pub(super) fn charpente_cout(longueur: f32) -> f32 { 3.0 + longueur }

// --- Charpente hexagonale --------------------------------------------------
// Variante **candidate** de l'épine, en cours de validation à l'écran : elle ne
// remplace pas `Charpente` tant que le rendu n'a pas tranché.
//
// Deux motifs, aucun décoratif :
//
// 1. **Lisibilité.** Une section carrée voit sa largeur apparente varier d'un
//    facteur 1,41 selon l'angle ; l'hexagone, seulement 1,15
//    (`pieces::HEXA_GAIN_SILHOUETTE`). Sous le filtre pixel, c'est le **pire
//    angle** qui décide : un montant qui tombe sous le pixel de trois quarts
//    disparaît par intermittence. Le circonradius est repris tel quel de la
//    version carrée, si bien que l'épine hexagonale n'est **pas plus grosse**
//    vue par un sommet — elle est seulement 22 % plus épaisse dans son pire cas.
// 2. **Cohérence.** Tout ce que porte l'épine est déjà hexagonal ou triangulaire
//    (cadre de propulsion, montures d'habitat, sections onigiri) ; le carré était
//    la dernière forme isolée du vaisseau.

/// **Pied** d'une charpente hexagonale : ce qui termine l'épine côté propulsion.
///
/// Les deux formes coexistent le temps de trancher à l'écran — voir
/// `docs/suivi/stations.md` §C.11.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PiedHexa {
    /// Rien : la charpente s'arrête sur sa base.
    Aucun,
    /// **Tour** hexagonale à section constante, qui prolonge le cône au même
    /// rayon (§C.9).
    Tour,
    /// **Pavillon** : le cône continue de s'ouvrir jusqu'à une large embouchure
    /// hexagonale ouverte, qui portera la propulsion (§C.11).
    Pavillon,
}

/// Ouverture de l'embouchure du pavillon, en rayons de base de cône. C'est le
/// « bloom » : au-delà de ~1,5 la corolle se lit franchement comme une corolle.
const PAVILLON_OUVERTURE: f32 = 2.1;
/// Hauteur du pavillon, en rayons de base de cône.
const PAVILLON_HAUTEUR: f32 = 2.0;
/// Écrasement de la section du pavillon selon Y.
///
/// **C'est ce chiffre qui crée les deux familles d'arêtes** : les deux côtés
/// perpendiculaires à Y gardent leur longueur (ils sont portés par X, que
/// l'écrasement ne touche pas), les quatre obliques raccourcissent. À 1 l'hexagone
/// redeviendrait régulier et les six côtés seraient égaux.
///
/// Réglé à **0,55** (2026-07-30, était 0,82) pour obtenir la silhouette de
/// **taille émeraude** demandée : deux longs côtés dominants et quatre biseaux
/// courts. C'est le *contraste* entre les deux familles qui fait la lecture, et il
/// dépend fortement de ce chiffre :
///
/// | écrasement | largeur / hauteur | grand côté / biseau |
/// |---|---|---|
/// | 0,82 | 1,41 | 1,15 — les deux familles se distinguent à peine |
/// | **0,55** | **2,10** | **1,45** |
/// | 0,45 | 2,57 | 1,58 — l'hexagone commence à s'aplatir en losange |
///
/// À 0,82 la section lisait comme un hexagone légèrement irrégulier, pas comme une
/// pierre taillée : les biseaux étaient presque aussi longs que les grands côtés.
pub(super) const PAVILLON_ETIREMENT: f32 = 0.55;
/// Niveaux de treillis dans la corolle.
const PAVILLON_ETAGES: usize = 3;
/// Hauteur de la **tour couronnant le pavillon**, en rayons de base de cône.
///
/// Passée de 0,8 à **4,8** (×6, 2026-07-30) : ce n'est plus une virole d'interface
/// mais un vrai **fût de propulsion**, à peu près aussi haut que l'embouchure est
/// large (10,2 contre 8,9). C'est lui qui accueille le bloc moteur.
const PAVILLON_TOUR_HAUTEUR: f32 = 4.8;
/// Niveaux de cette tour, choisis pour garder des baies de longueur comparable à
/// celles de la corolle (~1,7 contre ~1,4) : trop peu et les diagonales
/// s'étireraient en fuseaux, trop et les ceintures feraient un fût strié.
const PAVILLON_TOUR_ETAGES: usize = 6;

/// Circonradius de la section, aux deux bouts.
///
/// `√2` parce qu'on reprend le **circonradius du carré** (`sg·√2`, ses coins) et
/// non sa demi-largeur : c'est ce qui garantit que la nouvelle épine occupe la
/// même silhouette maximale que l'ancienne.
fn hexa_rayons(grand: Profil, petit: Profil) -> (f32, f32) {
    let k = TREILLIS_SECTION * std::f32::consts::SQRT_2;
    (grand.rayon() * k, petit.rayon() * k)
}

pub(super) fn charpente_hexa_ports(grand: Profil, petit: Profil, longueur: f32) -> Vec<Port> {
    charpente_ports(grand, petit, longueur)
}

pub(super) fn charpente_hexa_dessiner<P: Peintre>(p: &mut P, grand: Profil, petit: Profil, longueur: f32, courbure: f32, pied: PiedHexa) {
    let demi = longueur * 0.5;
    let (rg, rp) = hexa_rayons(grand, petit);
    crate::vaisseau::pieces::treillis_conique_hexa(
        p,
        vec3(0.0, 0.0, -demi),
        vec3(0.0, 0.0, demi),
        rg,
        rp,
        courbure,
        COULEUR,
        SOMBRE,
    );
    match pied {
        PiedHexa::Aucun => return,
        PiedHexa::Pavillon => {
            // **Le cône continue de s'ouvrir** au lieu de passer à une section
            // constante. L'embouchure est un large hexagone ouvert — c'est lui
            // qui recevra la propulsion.
            //
            // L'accostage tient parce que l'écrasement est **progressif** : nul au
            // col, il n'atteint `PAVILLON_ETIREMENT` qu'au bord. Appliqué d'emblée
            // il désaccorderait le col de la base du cône, qui est régulière.
            let r_bord = rg * PAVILLON_OUVERTURE;
            let h_pav = rg * PAVILLON_HAUTEUR;
            crate::vaisseau::pieces::pavillon_hexagonal(
                p,
                vec3(0.0, 0.0, -demi),
                rg,
                r_bord,
                h_pav,
                PAVILLON_ETIREMENT,
                PAVILLON_ETAGES,
                COULEUR,
                SOMBRE,
            );
            // **Tour au sommet du pavillon** : un tronçon droit qui prolonge
            // l'embouchure, sur lequel la propulsion viendra se poser.
            //
            // Elle reprend **le rayon et l'écrasement de l'embouchure**, pas ceux du
            // cône : c'est sur l'embouchure qu'elle se pose. Lui laisser une section
            // régulière remettrait le désaccord corrigé au col — les deux sommets
            // portés par X coïncideraient, les quatre obliques non.
            crate::vaisseau::pieces::tour_hexagonale(
                p,
                vec3(0.0, 0.0, -demi - h_pav),
                r_bord,
                rg * PAVILLON_TOUR_HAUTEUR,
                PAVILLON_ETIREMENT,
                // Épaisseur calée sur la section du **cône** et non sur
                // l'embouchure : sinon les barres grossissent avec l'ouverture
                // (×2,1) et le fût écrase la silhouette. Elles prolongent ainsi
                // exactement les longerons de la corolle, qui prennent déjà `rg`.
                rg * crate::vaisseau::pieces::LONGERON,
                PAVILLON_TOUR_ETAGES,
                COULEUR,
                SOMBRE,
            );
            return;
        }
        PiedHexa::Tour => {}
    }

    // **Tour hexagonale** sous le pied, en remplacement du cadre plat.
    //
    // Le cadre de la version carrée est un hexagone **couché** : son plan
    // *contient* l'axe de la poutre, si bien qu'il se présente de travers et qu'il
    // fallait une jupe vrillée d'un quart de tour pour l'accrocher. Basculé de 90°,
    // sa section devient **perpendiculaire à l'axe**, donc parallèle à celle du
    // cône — et il n'y a plus rien à raccorder : la tour **prolonge** le cône.
    //
    // L'accostage est exact **par construction** et non par réglage : les sommets
    // de la tour et la section du cône sortent de la même `hexa_section`, avec les
    // mêmes axes et le même rayon `rg`. Les six longerons descendent tout droit.
    let hauteur = rg * TOUR_HAUTEUR;
    crate::vaisseau::pieces::tour_hexagonale(
        p,
        vec3(0.0, 0.0, -demi),
        rg,
        hauteur,
        1.0, // section **régulière** : c'est la base du cône qu'elle prolonge
        rg * crate::vaisseau::pieces::LONGERON, // inchangé : la tour *est* à `rg`
        TOUR_ETAGES,
        COULEUR,
        SOMBRE,
    );

    // Plancher de la tour : trois cordes en étoile entre sommets opposés. C'est là
    // que viendront se poser les blocs de propulsion, il faut donc de la matière
    // au centre et pas seulement une ceinture.
    let bas = crate::vaisseau::pieces::hexa_section(
        vec3(0.0, 0.0, -demi - hauteur),
        Vec3::X,
        Vec3::Y,
        rg,
    );
    for i in 0..3 {
        p.cylindre(bas[i], bas[i + 3], rg * 0.08, SOMBRE);
    }
}

/// Hauteur de la tour du pied, en circonradius de section. Au-delà de ~2 elle
/// lit comme une **tour** et non comme une collerette épaisse.
const TOUR_HAUTEUR: f32 = 2.4;
/// Niveaux de la tour. Trois donnent assez de ceintures pour l'échelle sans
/// noyer la silhouette dans les diagonales.
const TOUR_ETAGES: usize = 3;

/// Extension **axiale** sous la base, selon le pied posé.
///
/// Pour le pavillon, la corolle **et** la tour qui la couronne : oublier la
/// seconde ferait sous-estimer la pièce, et c'est elle qui portera la propulsion.
pub(super) fn charpente_hexa_pied(grand: Profil, pied: PiedHexa) -> f32 {
    let rg = hexa_rayons(grand, grand).0;
    match pied {
        PiedHexa::Aucun => 0.0,
        PiedHexa::Tour => rg * TOUR_HAUTEUR,
        PiedHexa::Pavillon => rg * (PAVILLON_HAUTEUR + PAVILLON_TOUR_HAUTEUR),
    }
}

/// Hauteur de la seule **corolle**, sans la tour : c'est là que se trouve le plan
/// de l'embouchure, donc le repère de tout ce qui s'y raccorde.
pub(super) fn charpente_hexa_embouchure(grand: Profil) -> f32 {
    hexa_rayons(grand, grand).0 * PAVILLON_HAUTEUR
}

/// Extension **radiale** du pied. Le pavillon déborde largement la section de
/// l'épine : sans ça l'englobant de la pièce sous-estimerait sa corolle.
pub(super) fn charpente_hexa_pied_rayon(grand: Profil, pied: PiedHexa) -> f32 {
    let rg = hexa_rayons(grand, grand).0;
    match pied {
        PiedHexa::Aucun => 0.0,
        PiedHexa::Tour => rg,
        PiedHexa::Pavillon => rg * PAVILLON_OUVERTURE,
    }
}

pub(super) fn charpente_hexa_cout(longueur: f32) -> f32 { 3.5 + longueur }

// --- Hexagone --------------------------------------------------------------

pub(super) fn hexagone_dessiner<P: Peintre>(p: &mut P, profil: Profil, liaison: f32) {
    // Même anneau que le pied de la charpente (mêmes proportions).
    let sg = profil.rayon() * TREILLIS_SECTION;
    let cote = 2.0 * sg;
    let sec = sg * 0.5;
    let prof = sg;
    crate::vaisseau::pieces::treillis_hexagone(p, Vec3::ZERO, cote, sec, prof, COULEUR, SOMBRE);
    if liaison > 0.0 {
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
            p.cylindre(s, s + Vec3::Z * liaison, sec * 0.30, COULEUR);
        }
    }
}

pub(super) fn hexagone_cout() -> f32 { 12.0 }
