use crate::vaisseau::montage::{cuire, poser};
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, GenrePort, Profil, Repere, Sorties, StyleTreillis, VarianteModule, VariantePanneauMega,
};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};
use super::starship::{poser_anneau, VarianteAnneau};

pub(crate) fn hub6() -> Composant {
    Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six }
}

/// Pose `app` sur le port `Surface` d'un module situé du côté `dir` (monde), le
/// premier trouvé. Sert à placer des radiateurs/arrays sur une face précise
/// (ex. nadir −Y) sans dépendre de l'index du port.
pub(crate) fn appendice_sur_module(asm: &mut Assembleur, corps: Repere, comp: &Composant, dir: Vec3, app: &Composant) {
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
pub(crate) fn porter_vers(asm: &mut Assembleur, hote_monde: Repere, hote: &Composant, dir: Vec3, enfant: &Composant, montage: usize) -> Repere {
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
/// Verse un état publié dans un assemblage, **tel quel**.
///
/// Sert aux presets dissociés (moyeu fixe + anneau tournant) : la vue d'un seul
/// tenant se compose de ses deux moitiés au lieu de les rebâtir, ce qui garantit
/// que la vue animée et la vue statique montrent **exactement** le même objet.
pub(crate) fn verser_a_echelle(dest: &mut Assembleur, source: EtatStation, k: f32) {
    let EtatStation::Prete(s) = source else {
        return;
    };
    let m = Mat4::from_scale(Vec3::splat(k));
    for p in s.pieces() {
        dest.ajouter(crate::vaisseau::Piece::new(m * p.transforme, p.composant.clone()));
    }
}

/// Gabarit d'essai des panneaux d'échelle mégastructure : 16 × 6 U, soit
/// ~36 × 13,5 m — la taille d'un vrai SAW de l'ISS, et non celle du
/// `PanneauSolaire` du parc, qui en fait moins de la moitié.
pub(crate) const PANNEAU_MEGA_LONGUEUR: f32 = 16.0;
pub(crate) const PANNEAU_MEGA_LARGEUR: f32 = 6.0;

/// **Vitrine des quatre panneaux d'échelle mégastructure**, alignés pour être
/// comparés, et tous orientés par le **même** couple d'angles.
///
/// `suivi` va de 0 à 1 et balaie le débattement des deux joints : c'est ce qui
/// permet de juger l'articulation autant que la silhouette. Les quatre suivent
/// ensemble — les comparer à des angles différents ne dirait rien de leur
/// différence de forme.
///
/// L'écart entre deux panneaux est pris sur leur **encombrement en rotation**
/// (`rayon_local`) et non sur leur largeur : une aile qui pivote balaie un
/// disque, et deux voisines calées sur leur largeur se traverseraient dès le
/// premier quart de tour.
pub fn demo_panneaux_mega(suivi: f32) -> EtatStation {
    let mut asm = Assembleur::new();
    let (long, larg) = (PANNEAU_MEGA_LONGUEUR, PANNEAU_MEGA_LARGEUR);
    let azimut = suivi * PI;
    let inclinaison = (suivi - 0.5) * 2.0 * 40.0_f32.to_radians();

    let gabarit = |v: VariantePanneauMega, a: f32, i: f32| Composant::PanneauMega {
        profil: Profil::P0,
        variante: v,
        longueur: long,
        largeur: larg,
        azimut: a,
        inclinaison: i,
    };
    // Pas pris sur l encombrement **en rotation** et non sur la largeur : une
    // aile qui pivote balaie un disque, et deux voisines calees sur leur
    // largeur se traverseraient des le premier quart de tour.
    let pas = gabarit(VariantePanneauMega::FermeModulaire, 0.0, 0.0).rayon_local() * 1.25;
    for (i, variante) in VariantePanneauMega::TOUTES.into_iter().enumerate() {
        // 2x2 plutot qu une rangee : a cette echelle, quatre de front tiennent
        // dans le cadre mais chacun trop petit pour se juger.
        let (cx, cy) = ((i % 2) as f32 - 0.5, 0.5 - (i / 2) as f32);
        asm.ajouter(cuire(
            Repere::new(vec3(cx * pas * 2.0, cy * pas * 1.35, 0.0), Quat::IDENTITY),
            &gabarit(variante, azimut, inclinaison),
        ));
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
