//! **Caissons techniques et charges utiles** : la seule famille anguleuse du
//! jeu (tout le reste est de révolution). Le caisson est le *porteur* qui
//! expose des faces hôtes ; la charge utile est ce qu'on pose dessus.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::PI;

use super::commun::*;

/// Demi-hauteur d'un caisson, dérivée de sa largeur. Partagée par le dessin et
/// par le calcul des **ports hôtes**, pour qu'ils tombent bien sur les faces.
pub(super) fn caisson_haut(largeur: f32) -> f32 {
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

    pub(super) fn cout(self) -> f32 {
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

    pub(super) fn cout(self) -> f32 {
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

pub(super) fn ports(profil: Profil, longueur: f32, largeur: f32) -> Vec<Port> {
    // Le caisson est un **porteur** : port de montage (index 0) vers
    // sa structure, plus des ports hôtes sur ses cinq faces libres,
    // qui reçoivent charges utiles, radiateurs ou antennes.
    let haut = caisson_haut(largeur);
    let cz = CAISSON_PLATINE + longueur * 0.5;
    let mut v = vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
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

pub(super) fn charge_ports(profil: Profil) -> Vec<Port> {
    // Montage générique `Surface`, comme les autres appendices.
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
    )]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, variante: VarianteCaisson, longueur: f32, largeur: f32) {
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
    variante.dessiner(p, pied, longueur, largeur);
}

pub(super) fn charge_dessiner<P: Peintre>(p: &mut P, variante: VarianteCharge, longueur: f32, largeur: f32) {
    // **Aucune entretoise** : la charge est boulonnée à même la face
    // qui la porte, sa platine faisant office d'interface.
    variante.dessiner(p, Vec3::ZERO, longueur, largeur);
}

/// Platine courte + longueur de la boîte.
pub(super) fn rayon_local(longueur: f32, largeur: f32) -> f32 {
    (CAISSON_PLATINE + longueur).hypot(largeur * 0.8)
}

/// Charge à plat : demi-diagonale dans le plan + épaisseur.
pub(super) fn charge_rayon_local(longueur: f32, largeur: f32) -> f32 {
    (longueur * 0.5).hypot(largeur * 0.5) + largeur * 0.8
}
