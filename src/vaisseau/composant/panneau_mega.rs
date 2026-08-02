//! **Panneaux solaires d'échelle mégastructure**, et leur suivi du soleil.
//!
//! # Pourquoi une brique neuve et pas un `PanneauSolaire` agrandi
//!
//! Le panneau existant est taillé pour une station modulaire : un mât, une pale,
//! un socle. Passé à 30 m d'envergure il ne tient plus debout — ni visuellement
//! (une pale nue de cette taille lit comme une feuille de papier), ni
//! structurellement (rien ne reprend la flexion). Les très grands générateurs
//! réels changent de **famille**, pas de taille : ossature en caisson, membrane
//! tendue sur un cadre, concentrateurs, ou corolle déployable.
//!
//! # L'échelle, mesurée et non supposée
//!
//! L'unité du projet vaut ~2,25 m (`P1` = le rayon d'un module ISS). Repères :
//!
//! | | projet | réel |
//! |---|---:|---:|
//! | `PanneauSolaire` actuel | 6,5 × 2,0 U ≈ 14,6 × 4,5 m | SAW de l'ISS : **34 × 12 m** |
//! | Tore de Stanford | rayon 25,8 U ≈ 58 m | — |
//! | Bras du tore | 21 U ≈ 47 m | — |
//!
//! ⚠️ Le panneau du parc est donc déjà **2,3× sous-dimensionné** par rapport à
//! son propre modèle. Les gabarits d'ici partent du réel : `LONGUEUR_TYPE` vaut
//! 16 U ≈ 36 m, soit un vrai SAW, et l'aile en compte deux — de quoi être à
//! l'échelle d'un anneau de 58 m sans l'écraser.
//!
//! # Le suivi solaire
//!
//! Deux articulations, comme le vrai : un **azimut** autour du mât (l'ISS
//! l'appelle le joint Alpha, qui tourne avec l'orbite) et une **inclinaison**
//! autour de l'axe long de l'aile (le joint Beta, qui rattrape la saison). Les
//! deux ensemble amènent la normale n'importe où dans un hémisphère.
//!
//! Ils sont portés par le **composant** et non par la pose : c'est l'aile qui
//! bouge dans son berceau, le berceau restant boulonné à la structure. Un
//! panneau qu'on orienterait en tournant sa `Repere` de pose ferait tourner son
//! socle avec — il ne suivrait pas le soleil, il serait monté de travers.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

use super::commun::*;

/// Longueur de référence d'une aile, en unités monde (~36 m). Calée sur le SAW
/// de l'ISS, pas sur le panneau du parc, qui en fait moins de la moitié.
pub const LONGUEUR_TYPE: f32 = 16.0;
/// Largeur de référence (~13,5 m).
pub const LARGEUR_TYPE: f32 = 6.0;

/// Hauteur du mât qui porte le berceau, en fraction de la largeur. Il écarte
/// l'aile de la structure porteuse pour qu'elle puisse pivoter sans la heurter.
const MAT: f32 = 0.55;

/// Les quatre familles a l'essai.
///
/// Aucune n'est un `PanneauSolaire` agrandi : chacune repond a une contrainte
/// que la petite echelle ne pose pas, et c'est cette contrainte qui lui donne
/// sa forme.
///
/// | Variante | La contrainte a laquelle elle repond |
/// |---|---|
/// | `FermeModulaire` | on ne fabrique pas un monolithe de 40 m en orbite |
/// | `ConcentrateurSymetrique` | le silicium coute cher, les miroirs non |
/// | `RubanTendu` | a cette surface, la masse de l'ossature ecrase tout |
/// | `PlisseDeployable` | il faut que ca tienne dans une coiffe au lancement |
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VariantePanneauMega {
    /// **Ferme modulaire** : une poutre-treillis **profonde** portant une grille
    /// de cassettes photovoltaiques identiques.
    ///
    /// C'est la que convergent les etudes reelles de centrale orbitale (SPS
    /// ALPHA, Caltech SSPP) : on n'assemble pas une grande surface, on **repete
    /// un petit module** par milliers. La ferme a une vraie epaisseur - deux
    /// nappes de longerons entretoisees - parce qu'une structure de 36 m
    /// travaille en flexion et qu'un cadre plat n'y suffirait pas.
    FermeModulaire,
    /// **Concentrateur symetrique** : deux grands miroirs en coquille renvoient
    /// la lumiere sur une **etroite** bande active centrale.
    ///
    /// Reprend le principe de l'*Integrated Symmetrical Concentrator* etudie
    /// pour les centrales orbitales : la surface qui collecte est en aluminium
    /// mince, pas en silicium, et seule une fraction du champ est equipee. La
    /// contrepartie est thermique et se voit - la bande centrale porte ses
    /// **radiateurs**, faute de quoi elle cuirait.
    ConcentrateurSymetrique,
    /// **Rubans tendus** : de tres longues lanieres etroites, maintenues planes
    /// par la **tension** (mise en rotation lente), sans ossature rigide.
    ///
    /// C'est la logique de l'heliogyre. A cette echelle, l'ossature pese plus
    /// que ce qu'elle porte ; la supprimer entierement et se reposer sur la
    /// force centrifuge est la seule facon connue de descendre la masse
    /// surfacique d'un ordre de grandeur. Silhouette de pales, pas d'aile.
    RubanTendu,
    /// **Nappe plissee deployable** : un caisson de rangement, un mat
    /// enroulable, et la nappe qui sort en **accordeon**.
    ///
    /// La contrainte est au lancement, pas en vol : le generateur doit tenir
    /// plie dans un volume derisoire. Les plis restent visibles une fois
    /// deploiyes - c'est la signature des nappes reelles (ROSA, DOLCE), et ce
    /// qui interdit de les dessiner comme une surface lisse.
    PlisseDeployable,
}

impl VariantePanneauMega {
    pub const TOUTES: [VariantePanneauMega; 4] = [
        VariantePanneauMega::FermeModulaire,
        VariantePanneauMega::ConcentrateurSymetrique,
        VariantePanneauMega::RubanTendu,
        VariantePanneauMega::PlisseDeployable,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VariantePanneauMega::FermeModulaire => "FERME MODULAIRE A CASSETTES",
            VariantePanneauMega::ConcentrateurSymetrique => "CONCENTRATEUR SYMETRIQUE",
            VariantePanneauMega::RubanTendu => "RUBANS TENDUS (HELIOGYRE)",
            VariantePanneauMega::PlisseDeployable => "NAPPE PLISSEE DEPLOYABLE",
        }
    }
}

/// Teinte des cellules : le bleu-noir du silicium sous verre.
const CELLULE: Color = Color { r: 0.10, g: 0.14, b: 0.30, a: 1.0 };
/// Cadre et lattes.
const CADRE: Color = Color { r: 0.62, g: 0.63, b: 0.66, a: 1.0 };
/// Réflecteur des concentrateurs : de l'aluminium poli, presque blanc.
const MIROIR: Color = Color { r: 0.88, g: 0.89, b: 0.92, a: 1.0 };
/// Bande active au foyer d'une auge — elle chauffe, donc elle est sombre.
const ABSORBEUR: Color = Color { r: 0.20, g: 0.18, b: 0.16, a: 1.0 };
/// Radiateurs de la bande concentree : blancs, comme tout ce qui rayonne.
const RADIATEUR: Color = Color { r: 0.90, g: 0.91, b: 0.92, a: 1.0 };
/// Verre de couverture, une nuance plus claire que la cellule : ce lisere est
/// ce qui fait lire une **cassette** plutot qu'un aplat sombre.
const VITRAGE: Color = Color { r: 0.20, g: 0.27, b: 0.44, a: 1.0 };

/// Monté comme un appendice : un seul port `Surface`, avant vers l'hôte.
pub(super) fn ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)), GenrePort::Surface, profil)]
}

pub(super) fn cout(longueur: f32, largeur: f32) -> f32 {
    // À l'aire : une centrale se paie en surface collectrice, pas en pièces.
    (longueur * largeur * 0.12).max(6.0)
}

/// Demi-encombrement : l'aile pivote, donc **le rayon doit couvrir toutes ses
/// orientations**, pas seulement celle du moment. C'est la différence avec une
/// pièce fixe, et l'oublier ferait rentrer la caméra dans le panneau dès qu'il
/// tourne.
pub(super) fn rayon_local(longueur: f32, largeur: f32) -> f32 {
    let mat = largeur * MAT;
    mat + longueur.max(largeur) * 0.5 + (longueur.hypot(largeur)) * 0.5
}

pub(super) fn dessiner<P: Peintre>(
    p: &mut P,
    variante: VariantePanneauMega,
    longueur: f32,
    largeur: f32,
    azimut: f32,
    inclinaison: f32,
) {
    let mat = largeur * MAT;
    // Berceau fixe : bras vers l'hôte, socle, mât. Rien de tout ça ne tourne —
    // c'est le repère par rapport auquel le suivi se lit.
    p.cylindre(vec3(0.0, 0.0, -mat * 0.35), Vec3::ZERO, largeur * 0.045, SOMBRE);
    p.cube(Vec3::ZERO, Vec3::splat(largeur * 0.16), CADRE);
    p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, mat), largeur * 0.05, CADRE);

    // Les deux articulations, composées : azimut autour du mât (+Z), puis
    // inclinaison autour de l'axe long de l'aile (+X une fois orientée).
    let gimbal = Mat4::from_translation(vec3(0.0, 0.0, mat))
        * Mat4::from_rotation_z(azimut)
        * Mat4::from_rotation_x(inclinaison);
    p.empiler_transforme(gimbal);
    // Chape du joint, pour qu'on voie **où** ça tourne.
    p.cylindre(
        vec3(-largeur * 0.12, 0.0, 0.0),
        vec3(largeur * 0.12, 0.0, 0.0),
        largeur * 0.06,
        SOMBRE,
    );
    match variante {
        VariantePanneauMega::FermeModulaire => ferme_modulaire(p, longueur, largeur),
        VariantePanneauMega::ConcentrateurSymetrique => concentrateur_sym(p, longueur, largeur),
        VariantePanneauMega::RubanTendu => rubans_tendus(p, longueur, largeur),
        VariantePanneauMega::PlisseDeployable => nappe_plissee(p, longueur, largeur),
    }
    p.depiler_transforme();
}

/// **Ferme modulaire.** Une poutre-treillis profonde le long de +/-X, portant
/// une grille de cassettes sur sa face solaire (+Z local).
///
/// La profondeur (`prof`) n'est pas decorative : c'est elle qui donne l'inertie
/// de flexion. Une nappe plate de 36 m tenue en son milieu flechirait sous la
/// moindre acceleration de controle d'attitude.
fn ferme_modulaire<P: Peintre>(p: &mut P, longueur: f32, largeur: f32) {
    let demi = longueur * 0.5;
    let hw = largeur * 0.5;
    let prof = largeur * 0.22;
    let barre = largeur * 0.018;

    let travees = (longueur / (largeur * 0.5)).round().max(3.0) as usize;
    for s in [-1.0_f32, 1.0] {
        let y = s * hw;
        p.cylindre(vec3(-demi, y, 0.0), vec3(demi, y, 0.0), barre, CADRE);
        p.cylindre(vec3(-demi, y, -prof), vec3(demi, y, -prof), barre, CADRE);
    }
    // Membrure basse centrale : c'est un treillis triangule, pas une boite.
    p.cylindre(vec3(-demi, 0.0, -prof), vec3(demi, 0.0, -prof), barre, CADRE);
    for i in 0..=travees {
        let x = -demi + longueur * i as f32 / travees as f32;
        p.cylindre(vec3(x, -hw, 0.0), vec3(x, hw, 0.0), barre * 0.8, CADRE);
        p.cylindre(vec3(x, -hw, 0.0), vec3(x, 0.0, -prof), barre * 0.6, SOMBRE);
        p.cylindre(vec3(x, hw, 0.0), vec3(x, 0.0, -prof), barre * 0.6, SOMBRE);
        if i < travees {
            let x1 = -demi + longueur * (i + 1) as f32 / travees as f32;
            let (a, b) = if i % 2 == 0 { (-hw, hw) } else { (hw, -hw) };
            p.cylindre(vec3(x, a, 0.0), vec3(x1, b, 0.0), barre * 0.5, SOMBRE);
        }
    }

    // Cassettes : la grille d'unites identiques. Le jeu qui les separe est par
    // ou sort la chaleur, et c'est lui qui rend le module lisible.
    let nx = travees * 2;
    let ny = 3usize;
    let (px, py) = (longueur / nx as f32, largeur / ny as f32);
    let jeu = 0.86;
    for i in 0..nx {
        for j in 0..ny {
            let x = -demi + px * (i as f32 + 0.5);
            let y = -hw + py * (j as f32 + 0.5);
            let (cx, cy) = (px * 0.5 * jeu, py * 0.5 * jeu);
            let coin = vec3(x - cx, y - cy, barre);
            p.panneau(coin, vec3(2.0 * cx, 0.0, 0.0), vec3(0.0, 2.0 * cy, 0.0), CELLULE);
            p.cylindre(coin, coin + vec3(2.0 * cx, 0.0, 0.0), barre * 0.28, VITRAGE);
            p.cylindre(
                coin + vec3(0.0, 2.0 * cy, 0.0),
                coin + vec3(2.0 * cx, 2.0 * cy, 0.0),
                barre * 0.28,
                VITRAGE,
            );
        }
    }
}

/// **Concentrateur symetrique.** Deux coquilles reflechissantes inclinees vers
/// une bande active centrale, elle-meme flanquee de radiateurs.
///
/// Les miroirs sont facettes : une parabole se lit a sa courbure, et un miroir
/// orbital reel est de toute facon une mosaique de facettes tendues.
fn concentrateur_sym<P: Peintre>(p: &mut P, longueur: f32, largeur: f32) {
    let demi = longueur * 0.5;
    let barre = largeur * 0.02;
    let portee = largeur * 0.95;
    let creux = largeur * 0.42;

    p.cylindre(vec3(-demi, 0.0, 0.0), vec3(demi, 0.0, 0.0), barre * 1.4, CADRE);

    // Bande active : etroite, c'est tout l'interet du concentrateur.
    let hb = largeur * 0.10;
    p.panneau(
        vec3(-demi, -hb, largeur * 0.02),
        vec3(longueur, 0.0, 0.0),
        vec3(0.0, 2.0 * hb, 0.0),
        ABSORBEUR,
    );
    // Radiateurs : sous concentration, la bande evacue ou elle fond. Ils
    // pendent **sous** le plan collecteur, a l'ombre des miroirs.
    for s in [-1.0_f32, 1.0] {
        let y = s * hb * 1.3;
        p.panneau(
            vec3(-demi, y, -largeur * 0.01),
            vec3(longueur, 0.0, 0.0),
            vec3(0.0, s * largeur * 0.06, -largeur * 0.16),
            RADIATEUR,
        );
    }

    let facettes = 5usize;
    for s in [-1.0_f32, 1.0] {
        for f in 0..facettes {
            let t0 = f as f32 / facettes as f32;
            let t1 = (f + 1) as f32 / facettes as f32;
            let (y0, z0) = (s * (hb * 1.6 + portee * t0), creux * t0 * t0);
            let (y1, z1) = (s * (hb * 1.6 + portee * t1), creux * t1 * t1);
            p.panneau(
                vec3(-demi, y0, z0),
                vec3(longueur, 0.0, 0.0),
                vec3(0.0, y1 - y0, z1 - z0),
                MIROIR,
            );
        }
        // Arceaux de tension : une membrane reflechissante ne tient sa forme
        // que tendue sur des nervures.
        let arceaux = 4usize;
        for k in 0..=arceaux {
            let x = -demi + longueur * k as f32 / arceaux as f32;
            p.cylindre(
                vec3(x, s * hb * 1.6, 0.0),
                vec3(x, s * (hb * 1.6 + portee), creux),
                barre * 0.5,
                CADRE,
            );
        }
    }
}

/// **Rubans tendus.** Six lanieres tres elancees autour d'un moyeu, sans
/// ossature : ce sont la tension et la rotation qui les tiennent planes.
fn rubans_tendus<P: Peintre>(p: &mut P, longueur: f32, largeur: f32) {
    let n = 6usize;
    let moyeu = largeur * 0.16;
    let portee = longueur * 0.62;
    let ruban = largeur * 0.13;

    p.cylindre(vec3(0.0, 0.0, -moyeu * 0.4), vec3(0.0, 0.0, moyeu * 0.4), moyeu, CADRE);
    for k in 0..n {
        let a = TAU * k as f32 / n as f32;
        let (sa, ca) = a.sin_cos();
        let dir = vec3(ca, sa, 0.0);
        let tang = vec3(-sa, ca, 0.0);
        let pas = 6usize;
        for i in 0..pas {
            let (t0, t1) = (i as f32 / pas as f32, (i + 1) as f32 / pas as f32);
            let (r0, r1) = (moyeu + portee * t0, moyeu + portee * t1);
            let (w0, w1) = (ruban * (1.0 - 0.45 * t0), ruban * (1.0 - 0.45 * t1));
            let c0 = dir * r0 - tang * w0;
            p.panneau(c0, dir * (r1 - r0) + tang * (w0 - w1), tang * (2.0 * w0), CELLULE);
            // Cables de rive : ce sont **eux** qui portent l'effort centrifuge.
            p.cylindre(dir * r0 + tang * w0, dir * r1 + tang * w1, largeur * 0.006, CADRE);
            p.cylindre(dir * r0 - tang * w0, dir * r1 - tang * w1, largeur * 0.006, CADRE);
        }
        // Masselotte de bout : sans elle, rien ne met le ruban en tension.
        p.cube(dir * (moyeu + portee), Vec3::splat(largeur * 0.05), SOMBRE);
    }
}

/// **Nappe plissee.** Un caisson, un mat enroulable, et la nappe qui sort en
/// accordeon - les plis restent marques une fois deployee.
fn nappe_plissee<P: Peintre>(p: &mut P, longueur: f32, largeur: f32) {
    let demi = longueur * 0.5;
    let hw = largeur * 0.5;
    let pli = largeur * 0.10;
    let barre = largeur * 0.02;

    // Caisson de rangement a la racine : c'est de la que tout sort.
    let cc = vec3(-demi - largeur * 0.14, 0.0, 0.0);
    let ct = vec3(largeur * 0.26, largeur, largeur * 0.30);
    p.cube(cc, ct, CADRE);
    p.cube_fil(cc, ct, SOMBRE);

    // Mat enroulable, en helice : il se deroule d'une bobine, et sa torsade est
    // ce qui le distingue d'un tube.
    let tours = 26usize;
    for i in 0..tours {
        let (t0, t1) = (i as f32 / tours as f32, (i + 1) as f32 / tours as f32);
        let (a0, a1) = (t0 * TAU * 5.0, t1 * TAU * 5.0);
        let r = largeur * 0.05;
        p.cylindre(
            vec3(-demi + longueur * t0, r * a0.cos(), r * a0.sin()),
            vec3(-demi + longueur * t1, r * a1.cos(), r * a1.sin()),
            barre * 0.45,
            CADRE,
        );
    }

    // La nappe, en accordeon : facettes alternees de part et d'autre du plan.
    let plis = (longueur / (largeur * 0.30)).round().max(4.0) as usize;
    for i in 0..plis {
        let (t0, t1) = (i as f32 / plis as f32, (i + 1) as f32 / plis as f32);
        let (x0, x1) = (-demi + longueur * t0, -demi + longueur * t1);
        let (z0, z1) = if i % 2 == 0 { (0.0, pli) } else { (pli, 0.0) };
        for s in [-1.0_f32, 1.0] {
            let y0 = s * hw * 0.08;
            let y1 = s * hw;
            p.panneau(
                vec3(x0, y0, z0),
                vec3(x1 - x0, 0.0, z1 - z0),
                vec3(0.0, y1 - y0, 0.0),
                if i % 2 == 0 { CELLULE } else { VITRAGE },
            );
        }
        // Arete du pli, marquee : c'est elle qui fait lire le plisse.
        p.cylindre(vec3(x1, -hw, z1), vec3(x1, hw, z1), barre * 0.30, CADRE);
    }
    // Traverse de tension en bout : la nappe est tiree, pas poussee.
    p.cylindre(vec3(demi, -hw * 1.05, 0.0), vec3(demi, hw * 1.05, 0.0), barre, CADRE);
    for s in [-1.0_f32, 1.0] {
        p.cylindre(vec3(demi, s * hw * 1.05, 0.0), vec3(-demi, s * hw * 0.08, 0.0), largeur * 0.005, SOMBRE);
    }
}
