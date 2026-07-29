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
