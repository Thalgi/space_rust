//! **Module axial pressurisé** : le cylindre habité et ses dix variantes
//! d'habitat, avec ses collerettes de docking et ses embouts.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

use super::commun::*;

/// Variantes d'habitat (module pressurisé) — change couleur et détails de surface.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VarianteModule {
    /// Blanc simple.
    Standard,
    /// Teinte or (segment russe).
    Dore,
    /// Rangée de hublots + mains courantes EVA.
    Hublots,
    /// Grande fenêtre + rack externe (type Destiny).
    Labo,
    /// Profil bombé (type BEAM gonflable).
    Gonflable,
    /// Coupole vitrée à un bout (type Cupola).
    Coupole,
    /// Sas (type Quest) : écoutille EVA saillante + main courante.
    Sas,
    /// Grand habitat gonflable (type B330) : toile tendue de forte section,
    /// ancrée par des **cônes métalliques rigides** aux deux bouts, cerclée de
    /// **sangles de retenue**, percée de grandes fenêtres. À ne pas confondre
    /// avec `Gonflable`, qui n'est qu'un petit bombement façon BEAM.
    GrandGonflable,
    /// Serre / module agricole : longues **baies vitrées** teintées de vert par
    /// la culture, rails de racks et rampes d'éclairage. Registre futuriste.
    Serre,
    /// Cœur de station russe (Mir, Zvezda, Zarya) : corps **étagé** — tambour
    /// arrière plus large que le compartiment de travail. C'est une affaire de
    /// **dessin**, pas de profil : les ports restent au calibre du module, seule
    /// la silhouette change.
    Coeur,
    /// Le **même corps étagé** que [`VarianteModule::Coeur`], en gris clair au
    /// lieu du doré russe.
    ///
    /// Une variante et non un paramètre de teinte : dans ce parc, la couleur
    /// fait partie du **vocabulaire** (`Dore`, `Labo`, `Serre`…), pas d'un
    /// réglage qu'on passerait à la pose. Ajouter un champ `couleur` à
    /// `ModuleAxial` obligerait tous ses appelants à en choisir une.
    CoeurGris,
}

impl VarianteModule {
    // Ordre **identique à la déclaration de l'enum** : ainsi l'index affiché
    // (numéros N dans la vue briques = ordre de cette table) == l'index du code.
    pub const TOUS: [VarianteModule; 11] = [
        VarianteModule::Standard,
        VarianteModule::Dore,
        VarianteModule::Hublots,
        VarianteModule::Labo,
        VarianteModule::Gonflable,
        VarianteModule::Coupole,
        VarianteModule::Sas,
        VarianteModule::GrandGonflable,
        VarianteModule::Serre,
        VarianteModule::Coeur,
        VarianteModule::CoeurGris,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            VarianteModule::Standard => "STANDARD",
            VarianteModule::Dore => "DORE (RUSSE)",
            VarianteModule::Hublots => "HUBLOTS",
            VarianteModule::Labo => "LABO",
            VarianteModule::Gonflable => "GONFLABLE (BEAM)",
            VarianteModule::Coupole => "COUPOLE",
            VarianteModule::Sas => "SAS",
            VarianteModule::Coeur => "COEUR ETAGE (RUSSE)",
            VarianteModule::CoeurGris => "COEUR ETAGE (GRIS)",
            VarianteModule::GrandGonflable => "GRAND GONFLABLE (B330)",
            VarianteModule::Serre => "SERRE AGRICOLE",
        }
    }

    pub(super) fn couleur(self) -> Color {
        match self {
            VarianteModule::Dore | VarianteModule::Coeur => Color::new(0.72, 0.58, 0.28, 1.0),
            VarianteModule::CoeurGris => Color::new(0.66, 0.67, 0.70, 1.0),
            VarianteModule::Labo => Color::new(0.80, 0.82, 0.85, 1.0),
            VarianteModule::Gonflable | VarianteModule::GrandGonflable => {
                Color::new(0.84, 0.81, 0.75, 1.0)
            }
            VarianteModule::Serre => Color::new(0.78, 0.82, 0.80, 1.0),
            _ => Color::new(0.85, 0.85, 0.88, 1.0),
        }
    }

    /// Facteur de débord radial de la variante, en multiples du rayon nominal.
    /// Les formes bombées (toile tendue, tambour étagé) sortent du gabarit du
    /// cylindre : le rayon englobant doit le savoir.
    pub(super) fn debord_radial(self) -> f32 {
        match self {
            VarianteModule::GrandGonflable => 1.62,
            VarianteModule::Gonflable => 1.30,
            VarianteModule::Coeur | VarianteModule::CoeurGris => 1.16,
            _ => 1.0,
        }
    }

    /// Habillage de coque : sur le doré, bandes MLI alternées (feuilles
    /// d'isolant) ; sinon coutures de panneaux — anneaux fins en très léger
    /// débord radial (pas de face coplanaire → pas de z-fighting).
    pub(super) fn habillage<P: Peintre>(self, p: &mut P, rayon: f32, demi: f32) {
        let c = self.couleur();
        match self {
            VarianteModule::Dore | VarianteModule::Coeur | VarianteModule::CoeurGris => {
                // Bandes d'isolant multicouche : tuiles de teintes dorées voisines.
                let n = ((demi * 2.0) / 0.55).round().max(2.0) as usize;
                for i in 0..n {
                    let z0 = -demi + (i as f32) * (2.0 * demi / n as f32);
                    let z1 = z0 + (2.0 * demi / n as f32) * 0.92; // jour entre feuilles
                    let t = match i % 3 {
                        0 => 1.06,
                        1 => 0.93,
                        _ => 1.0,
                    };
                    let teinte = Color::new(
                        (c.r * t).min(1.0),
                        (c.g * t).min(1.0),
                        (c.b * t * 0.97).min(1.0),
                        1.0,
                    );
                    p.cylindre(vec3(0.0, 0.0, z0), vec3(0.0, 0.0, z1), rayon * 1.004, teinte);
                }
            }
            // Toile tendue : pas de coutures de tôle.
            VarianteModule::Gonflable | VarianteModule::GrandGonflable => {}
            _ => {
                // Coutures de panneaux : un anneau discret tous les ~0.8 u.
                let seam = Color::new(c.r * 0.80, c.g * 0.80, c.b * 0.82, 1.0);
                let n = ((demi * 2.0) / 0.8).round().max(1.0) as usize;
                for i in 1..n {
                    let z = -demi + (i as f32) * (2.0 * demi / n as f32);
                    p.cylindre(
                        vec3(0.0, 0.0, z - 0.015),
                        vec3(0.0, 0.0, z + 0.015),
                        rayon * 1.004,
                        seam,
                    );
                }
            }
        }
    }

    /// Détails de surface, dessinés par-dessus le corps (repère local, axe Z,
    /// corps de rayon `rayon` s'étendant de −`demi` à +`demi`).
    pub(super) fn details<P: Peintre>(self, p: &mut P, rayon: f32, demi: f32) {
        let sombre = Color::new(0.18, 0.20, 0.24, 1.0);
        let vitre = Color::new(0.15, 0.24, 0.34, 1.0);
        match self {
            VarianteModule::Hublots => {
                let n = ((demi * 2.0 / 0.6) as usize).max(2);
                for i in 0..n {
                    let z = -demi + (i as f32 + 0.5) * (2.0 * demi / n as f32);
                    p.sphere(vec3(0.0, rayon * 0.92, z), rayon * 0.13, sombre);
                }
                for s in [-1.0_f32, 1.0] {
                    let x = rayon * 0.72 * s;
                    p.cylindre(vec3(x, rayon * 0.72, -demi * 0.8), vec3(x, rayon * 0.72, demi * 0.8), 0.03, sombre);
                }
            }
            VarianteModule::Labo => {
                // grande fenêtre plate sur +Y + rack externe sur −Y.
                p.cube(vec3(0.0, rayon, 0.0), vec3(rayon * 0.9, 0.06, demi * 1.1), vitre);
                p.cube(vec3(0.0, -rayon * 1.05, 0.0), vec3(rayon * 0.7, rayon * 0.3, demi * 0.8), sombre);
            }
            VarianteModule::Gonflable => {
                // bombement central (fabric BEAM) : une sphère aplatie au milieu.
                p.sphere(Vec3::ZERO, rayon * 1.3, self.couleur());
            }
            VarianteModule::Coupole => {
                p.cone(vec3(0.0, 0.0, demi), Vec3::Z, rayon * 0.6, rayon * 0.32, rayon * 0.5, vitre);
            }
            VarianteModule::Sas => {
                // Écoutille EVA saillante sur +X + main courante.
                p.cube(vec3(rayon * 1.05, 0.0, 0.0), vec3(0.4, rayon * 0.7, rayon * 0.7), sombre);
                p.cylindre(
                    vec3(rayon * 0.9, rayon * 0.6, -demi * 0.6),
                    vec3(rayon * 0.9, rayon * 0.6, demi * 0.6),
                    0.03,
                    sombre,
                );
            }
            VarianteModule::GrandGonflable => {
                // Ce qui distingue le B330 du BEAM : la toile est ancrée par des
                // **cônes métalliques rigides** aux deux bouts, et cerclée de
                // **sangles de retenue** axiales et circonférentielles.
                let c = self.couleur();
                let rt = rayon * 1.62; // section gonflée
                let dz = demi * 0.62; // portion tendue, hors cônes d'ancrage
                p.cylindre(vec3(0.0, 0.0, -dz), vec3(0.0, 0.0, dz), rt, c);
                p.sphere(vec3(0.0, 0.0, -dz), rt, c);
                p.sphere(vec3(0.0, 0.0, dz), rt, c);
                let metal = Color::new(0.60, 0.62, 0.66, 1.0);
                for s in [-1.0_f32, 1.0] {
                    p.cone(
                        vec3(0.0, 0.0, s * demi),
                        vec3(0.0, 0.0, -s),
                        rayon * 1.02,
                        rt * 0.78,
                        demi - dz,
                        metal,
                    );
                }
                // Sangles axiales, puis cerclages.
                let sangle = Color::new(0.42, 0.40, 0.36, 1.0);
                for k in 0..8 {
                    let a = TAU * k as f32 / 8.0;
                    let dir = vec3(a.cos(), a.sin(), 0.0);
                    p.cylindre(
                        dir * (rt * 0.99) + vec3(0.0, 0.0, -dz),
                        dir * (rt * 0.99) + vec3(0.0, 0.0, dz),
                        rayon * 0.035,
                        sangle,
                    );
                }
                for z in [-dz * 0.55, 0.0, dz * 0.55] {
                    p.cylindre(
                        vec3(0.0, 0.0, z - rayon * 0.03),
                        vec3(0.0, 0.0, z + rayon * 0.03),
                        rt * 1.01,
                        sangle,
                    );
                }
                // Quatre grandes fenêtres.
                for k in 0..4 {
                    let a = TAU * k as f32 / 4.0 + 0.4;
                    let dir = vec3(a.cos(), a.sin(), 0.0);
                    p.cylindre(
                        dir * (rt * 0.98),
                        dir * (rt * 1.04),
                        rayon * 0.22,
                        vitre,
                    );
                }
            }
            VarianteModule::Serre => {
                // Longues baies vitrées teintées par la culture, entre des rails
                // de racks, plus des rampes d'éclairage.
                let verre = Color::new(0.36, 0.62, 0.42, 1.0);
                let rail = Color::new(0.58, 0.60, 0.64, 1.0);
                for k in 0..4 {
                    let a = TAU * k as f32 / 4.0;
                    let dir = vec3(a.cos(), a.sin(), 0.0);
                    // Baie vitrée : panneau bombé le long du corps.
                    p.cylindre(
                        dir * (rayon * 0.62) + vec3(0.0, 0.0, -demi * 0.78),
                        dir * (rayon * 0.62) + vec3(0.0, 0.0, demi * 0.78),
                        rayon * 0.52,
                        verre,
                    );
                    // Rail de rack entre deux baies.
                    let b = TAU * (k as f32 + 0.5) / 4.0;
                    let db = vec3(b.cos(), b.sin(), 0.0);
                    p.cylindre(
                        db * (rayon * 1.02) + vec3(0.0, 0.0, -demi * 0.9),
                        db * (rayon * 1.02) + vec3(0.0, 0.0, demi * 0.9),
                        rayon * 0.07,
                        rail,
                    );
                }
                // Rampes d'éclairage sur deux génératrices.
                for s in [-1.0_f32, 1.0] {
                    let o = vec3(0.0, rayon * 1.10 * s, 0.0);
                    p.cylindre(
                        o + vec3(0.0, 0.0, -demi * 0.7),
                        o + vec3(0.0, 0.0, demi * 0.7),
                        rayon * 0.05,
                        Color::new(0.95, 0.93, 0.72, 1.0),
                    );
                }
            }
            VarianteModule::Coeur | VarianteModule::CoeurGris => {
                // Tambour arrière plus large, puis transition conique vers le
                // compartiment de travail. Le col de docking, plus étroit,
                // continue de dépasser en bout — comme sur Mir ou Zvezda.
                let c = self.couleur();
                let rt = rayon * 1.16;
                let h = demi * 0.75;
                p.cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, -demi + h), rt, c);
                p.cone(vec3(0.0, 0.0, -demi + h), Vec3::Z, rt, rayon, rayon * 0.30, c);
            }
            VarianteModule::Standard | VarianteModule::Dore => {}
        }
    }
}

pub(super) fn ports(profil: Profil, longueur: f32) -> Vec<Port> {
    // Le port se pose au **bout de la collerette** (offset de docking).
    let tip = longueur * 0.5 + profil.rayon() * COL_LONG;
    let mut v = vec![
        // Bout +Z : avant = +Z (rot identité), haut = +Y.
        Port::new(
            Repere::new(vec3(0.0, 0.0, tip), Quat::IDENTITY),
            GenrePort::ModuleAxial,
            profil,
        ),
        // Bout −Z : demi-tour autour du haut → avant = −Z, haut = +Y.
        Port::new(
            Repere::new(vec3(0.0, 0.0, -tip), Quat::from_rotation_y(PI)),
            GenrePort::ModuleAxial,
            profil,
        ),
    ];
    // Ports hôtes `Surface` radiaux (±X, ±Y) sur le flanc, pour
    // accueillir panneaux / radiateurs / antennes (stations type Mir).
    for (dir, rot) in faces_principales().into_iter().take(4) {
        v.push(Port::new(Repere::new(dir * profil.rayon(), rot), GenrePort::Surface, Profil::P0));
    }
    v
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, profil: Profil, variante: VarianteModule, longueur: f32) {
    let rayon = profil.rayon();
    let demi = longueur * 0.5;
    let lc = rayon * COL_LONG;
    let rc = rayon * COL_RAYON;
    // Corps : cylindre lisse, teinté par la variante.
    let corps = variante.couleur();
    p.cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, demi), rayon, corps);
    // Habillage de coque : coutures de panneaux (ou bandes MLI pour
    // le doré) — anneaux fins en léger débord radial, teinte voisine.
    variante.habillage(p, rayon, demi);
    // Embouts : un petit cylindre coiffe chaque disque de bout. Il
    // chevauche le corps (part de `demi - EMBOUT_ENFONCE`) → aucune
    // face coplanaire, donc plus de z-fighting ; léger débord = arête.
    let re = rayon * EMBOUT_RAYON;
    p.cylindre(vec3(0.0, 0.0, demi - EMBOUT_ENFONCE), vec3(0.0, 0.0, demi + EMBOUT_LONG), re, SOMBRE);
    p.cylindre(vec3(0.0, 0.0, -demi + EMBOUT_ENFONCE), vec3(0.0, 0.0, -demi - EMBOUT_LONG), re, SOMBRE);
    // Collerettes de docking : cols étroits qui dépassent à chaque bout,
    // terminés par une bague d'accostage alu clair (visuel APAS).
    p.cylindre(vec3(0.0, 0.0, demi), vec3(0.0, 0.0, demi + lc), rc, SOMBRE);
    p.cylindre(vec3(0.0, 0.0, -demi), vec3(0.0, 0.0, -demi - lc), rc, SOMBRE);
    let lb = lc * 0.28; // épaisseur de la bague
    p.cylindre(vec3(0.0, 0.0, demi + lc - lb), vec3(0.0, 0.0, demi + lc), rc * 1.10, BAGUE);
    p.cylindre(vec3(0.0, 0.0, -demi - lc + lb), vec3(0.0, 0.0, -demi - lc), rc * 1.10, BAGUE);
    // Détails de surface (hublots, fenêtre, coupole, bombement…).
    variante.details(p, rayon, demi);
}

/// Corps + 2 embouts + 2 collerettes de docking = 5.
pub(super) fn cout() -> f32 {
    5.0
}

/// Extension axiale (jusqu'au bout du col) ou radiale, la plus grande.
/// Les variantes bombées débordent du rayon nominal : il faut en tenir
/// compte pour le cadrage et l'anti-collision.
pub(super) fn rayon_local(profil: Profil, variante: VarianteModule, longueur: f32) -> f32 {
    // Extension axiale (jusqu'au bout du col) ou radiale, la plus
    // grande. Les variantes bombées débordent du rayon nominal, il
    // faut en tenir compte pour le cadrage et l'anti-collision.
    let radial = profil.rayon() * variante.debord_radial();
    (longueur * 0.5 + profil.rayon() * COL_LONG).max(radial)
}

/// Demi-section transversale : ce que la capsule de collision doit couvrir en
/// travers du fût. Les variantes bombées débordent du rayon nominal, d'où la
/// même marge que dans [`rayon_local`].
pub(super) fn demi_section(profil: Profil, variante: VarianteModule) -> f32 {
    profil.rayon() * variante.debord_radial().max(1.0) * 1.05
}
