//! **Mesureur de composants** : tranche la géométrie et en rend le profil.
//!
//! Il répond, sans avis et sans œil, aux questions qu'on posait jusqu'ici en
//! regardant la vue : quelle est la section de cette pièce à mi-longueur ?
//! est-ce qu'elle tient dans l'enveloppe qu'elle déclare ? cette enveloppe
//! est-elle serrée ou grotesquement large ?
//!
//! ⚠️ **Il tranche les [`Fil`]s, jamais le maillage cuit.** C'est le point de
//! conception, et il vient d'une erreur commise **trois fois** dans ce projet
//! (`suivi/stations.md` §C.13 et §C.29) : un maillage cuit n'a de sommets qu'aux
//! **frontières de facettes**, si bien qu'une tranche prise à mi-portée d'un
//! cylindre ou d'un cône revient **vide** — et la mesure lit zéro sans rien
//! signaler. Trois mesures fausses de l'ISV viennent de là.
//!
//! Un fil, lui, est un segment analytique doté d'un rayon. « Quelle est la
//! section à z = 4,2 ? » y a une réponse fermée, pas un échantillon : tout fil
//! dont la portée croise le plan contribue, et rien ne tombe entre deux
//! facettes. Échantillonner **le long d'un segment droit** est exact, là
//! qu'échantillonner un maillage facetté ne l'est pas — c'est toute la
//! différence, et c'est ce qui rend ce module fiable là où la même idée
//! appliquée aux sommets cuits ne l'était pas.

// Le mesureur n'a pour l'instant qu'un consommateur, et il est en `cfg(test)` :
// le générateur de `docs/reference/fils.md`. C'est voulu — mesurer est une
// activité d'atelier, pas de rendu. L'assembleur s'en servira ensuite pour
// afficher le profil d'une pièce sélectionnée.
#![allow(dead_code)]

use super::{Composant, Enveloppe, Fil, GenreFil, Noyau};
use macroquad::prelude::*;

/// Points relevés le long de chaque fil. Un segment étant droit, l'interpolation
/// est **exacte** : ce nombre ne règle que la finesse du profil, jamais sa
/// justesse.
const PAS_PAR_FIL: usize = 24;

/// Une tranche : sa position le long de l'axe, et le rayon de la matière qu'on y
/// trouve.
#[derive(Clone, Copy, Debug)]
pub struct Coupe {
    /// Position le long de l'axe, depuis le pied de la pièce.
    pub t: f32,
    /// Plus grand rayon de matière rencontré dans la tranche. `0` = tranche vide
    /// (la pièce est creuse ou s'interrompt à cet endroit).
    pub rayon: f32,
}

/// Le relevé complet d'un composant.
#[derive(Clone, Debug)]
pub struct Mesure {
    /// Axe le long duquel la pièce a été tranchée.
    pub axe: Vec3,
    /// Étendue le long de l'axe.
    pub bas: f32,
    pub haut: f32,
    /// Plus grand rayon rencontré, toutes tranches confondues.
    pub rayon_max: f32,
    pub coupes: Vec<Coupe>,
    /// Rayon dont l'enveloppe déclarée **aurait besoin** pour contenir la pièce.
    pub besoin: f32,
    /// Rayon que l'enveloppe **déclare**.
    pub declare: f32,
}

impl Mesure {
    pub fn longueur(&self) -> f32 {
        self.haut - self.bas
    }

    /// Serrage de l'enveloppe : `besoin / déclaré`.
    ///
    /// - `> 1` : l'enveloppe **ne contient pas** la pièce — la collision ment ;
    /// - `≈ 1` : serrée, c'est la cible ;
    /// - `< 1` : elle contient, mais réserve du vide — des poses valides seront
    ///   refusées.
    pub fn serrage(&self) -> f32 {
        if self.declare <= 1e-6 {
            return 0.0;
        }
        self.besoin / self.declare
    }

    /// Élancement : longueur sur diamètre. Au-dessus de ~1,5 une sphère devient
    /// un mauvais englobant et la capsule s'impose (`suivi/assembleur.md` L1.6).
    pub fn elancement(&self) -> f32 {
        if self.rayon_max <= 1e-6 {
            return 0.0;
        }
        self.longueur() / (2.0 * self.rayon_max)
    }

    /// Verdict d'une ligne, pour le catalogue.
    pub fn verdict(&self) -> &'static str {
        match self.serrage() {
            s if s > 1.001 => "DEBORDE",
            s if s > 0.75 => "serree",
            s if s > 0.45 => "lache",
            _ => "TRES LACHE",
        }
    }
}

/// Axe de tranchage d'un composant.
///
/// Repris de l'**enveloppe déclarée** quand c'est une capsule : c'est déjà l'axe
/// que la pièce s'est reconnu, et en prendre un autre ferait mesurer une
/// silhouette qui n'est celle de personne. Une pièce ramassée (sphère) est
/// tranchée le long de `+Z`, l'axe de montage par convention dans tout le projet.
/// Une plaque (boudin) est tranchée le long de sa **normale** — dans
/// l'épaisseur, seule direction où elle a une étendue à mesurer.
fn axe(env: &Enveloppe) -> Vec3 {
    if env.est_sphere() {
        return Vec3::Z;
    }
    match env.noyau {
        Noyau::Segment { a, b } => (b - a).normalize_or_zero(),
        Noyau::Rectangle { eu, ev, .. } => eu.cross(ev),
    }
}

/// Relève les points d'un fil, avec le rayon qu'il porte **à cet endroit**.
///
/// Le rayon est interpolé entre les deux bouts : sur un tronc de cône, c'est son
/// profil exact. C'est la correction du premier relevé, où un cône se voyait
/// prêter partout le rayon de son gros bout et gonflait son besoin d'enveloppe.
fn points(f: &Fil) -> Vec<(Vec3, f32)> {
    let n = if f.longueur() < 1e-4 { 1 } else { PAS_PAR_FIL };
    (0..=n)
        .map(|k| {
            let t = k as f32 / n as f32;
            (f.a + (f.b - f.a) * t, f.rayon_a_t(t))
        })
        .collect()
}

/// Tranche un composant en `tranches` coupes et rend son profil.
pub fn mesurer(comp: &Composant, tranches: usize) -> Mesure {
    let env = comp.enveloppe_locale();
    let axe = axe(&env);
    let fils: Vec<Fil> = super::fils(comp);
    let releve: Vec<(Vec3, f32)> = fils.iter().flat_map(points).collect();

    if releve.is_empty() {
        return Mesure {
            axe,
            bas: 0.0,
            haut: 0.0,
            rayon_max: 0.0,
            coupes: Vec::new(),
            besoin: 0.0,
            declare: env.rayon,
        };
    }

    // Étendue le long de l'axe, **rayons compris** : un cylindre s'arrête à son
    // bout, pas à l'axe de son bout.
    let long = |p: Vec3| p.dot(axe);
    let bas = releve.iter().fold(f32::MAX, |m, (p, r)| m.min(long(*p) - r));
    let haut = releve.iter().fold(f32::MIN, |m, (p, r)| m.max(long(*p) + r));

    // Rayon **autour du noyau de l'enveloppe** : c'est lui que la collision
    // utilise, donc le seul par rapport auquel « serré » veut dire quelque chose.
    let radial = |p: Vec3| match env.noyau {
        Noyau::Segment { a, b } => super::distance_point_segment(p, a, b),
        Noyau::Rectangle { centre, eu, ev, hu, hv } => {
            super::distance_point_rectangle(p, centre, eu, ev, hu, hv)
        }
    };

    let n = tranches.max(1);
    let mut coupes = vec![Coupe { t: 0.0, rayon: 0.0 }; n];
    let etendue = (haut - bas).max(1e-6);
    for (i, c) in coupes.iter_mut().enumerate() {
        c.t = bas + etendue * (i as f32 + 0.5) / n as f32;
    }
    for (p, r) in &releve {
        // La matière d'un fil occupe la **bande** `[t-r, t+r]` le long de l'axe,
        // pas le seul plan de son axe. Sans ça, un cylindre transverse ne
        // marquerait qu'une tranche sur dix et le profil serait en dents de scie.
        let t = long(*p);
        let (lo, hi) = (t - r, t + r);
        let i0 = (((lo - bas) / etendue) * n as f32).floor().max(0.0) as usize;
        let i1 = ((((hi - bas) / etendue) * n as f32).ceil() as usize).min(n);
        let rr = radial(*p) + r;
        for c in coupes.iter_mut().take(i1).skip(i0) {
            c.rayon = c.rayon.max(rr);
        }
    }

    let rayon_max = coupes.iter().fold(0.0_f32, |m, c| m.max(c.rayon));
    let besoin = releve.iter().fold(0.0_f32, |m, (p, r)| m.max(radial(*p) + r));
    Mesure { axe, bas, haut, rayon_max, coupes, besoin, declare: env.rayon }
}

/// Le profil en une ligne de caractères, pour le catalogue.
///
/// Une silhouette lisible d'un coup d'œil vaut mieux qu'une colonne de nombres
/// quand il s'agit de repérer *où* une pièce enfle ou se creuse — le tableau
/// reste dessous pour les cotes exactes.
pub fn silhouette(m: &Mesure) -> String {
    const ECHELLE: [char; 9] = [' ', '.', ':', '-', '=', '+', '*', '#', '@'];
    if m.rayon_max <= 1e-6 {
        return String::new();
    }
    m.coupes
        .iter()
        .map(|c| {
            let k = ((c.rayon / m.rayon_max) * (ECHELLE.len() - 1) as f32).round() as usize;
            ECHELLE[k.min(ECHELLE.len() - 1)]
        })
        .collect()
}

/// Y a-t-il des mailles brutes, dont le mesureur ne sait rien dire d'exact ?
///
/// Les coiffes et les plaques de bouclier sont émises en `triangles` : elles
/// n'ont pas de forme analytique, et leur « fil » est une diagonale d'englobant
/// qui ne longe aucune arête. Le profil les ignore donc, et le catalogue doit le
/// dire plutôt que de laisser croire à une mesure complète.
pub fn a_des_mailles(comp: &Composant) -> bool {
    super::fils(comp).iter().any(|f| f.genre == GenreFil::Maille)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::{Profil, StyleTreillis, VarianteModule};

    // **Le piège que ce module existe pour éviter** (§C.13, §C.29) : une tranche
    // prise à mi-portée d'un cylindre revient vide sur un maillage cuit, parce
    // qu'il n'a de sommets qu'aux bouts. Ici elle doit trouver de la matière —
    // c'est la propriété qui distingue un mesureur fiable d'un qui ment.
    #[test]
    fn aucune_tranche_dun_cylindre_plein_nest_vide() {
        let c = Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre };
        let m = mesurer(&c, 24);
        assert!(m.longueur() > 7.0, "longueur mesurée {:.2}", m.longueur());
        for (i, coupe) in m.coupes.iter().enumerate() {
            assert!(coupe.rayon > 0.0, "tranche {i} vide à t={:.2}", coupe.t);
        }
    }

    // La longueur mesurée doit retrouver la cote **demandée** à la pièce. C'est
    // le contrôle qui dit que l'axe, les bandes et les rayons se composent bien.
    #[test]
    fn la_longueur_mesuree_retrouve_la_cote_demandee() {
        for l in [4.0_f32, 8.0, 20.0] {
            let c = Composant::Treillis { profil: Profil::P1, longueur: l, style: StyleTreillis::Carre };
            let m = mesurer(&c, 16);
            // Les longerons ont une épaisseur : la pièce dépasse un peu sa cote.
            let ecart = m.longueur() - l;
            assert!((0.0..1.0).contains(&ecart), "longueur {:.2} pour une cote {l}", m.longueur());
        }
    }

    // Le serrage doit **s'accorder avec les tests de contenance** déjà en place :
    // un composant qui déborde son enveloppe a un serrage > 1, et l'inverse.
    // Deux mesures indépendantes de la même chose doivent conclure pareil.
    #[test]
    fn le_serrage_saccorde_avec_la_contenance() {
        for c in [
            Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre },
            Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 },
            Composant::RadiateurMega { profil: Profil::P1, longueur: 30.0, largeur: 8.0, ailettes: 6, chaleur: 0.0 },
        ] {
            let m = mesurer(&c, 16);
            assert!(m.serrage() > 0.0, "{c:?} : serrage nul");
            assert!(
                m.serrage() <= 1.4,
                "{c:?} : déborde son enveloppe (serrage {:.2})",
                m.serrage()
            );
        }
    }

    // L'élancement mesuré doit dire ce que L1.6 a décidé à la main : les pièces
    // allongées ont reçu une capsule, les ramassées sont restées sphériques.
    // Si les deux se contredisent, c'est qu'une conversion a été mal jugée.
    #[test]
    fn lelancement_mesure_justifie_la_forme_de_lenveloppe() {
        let allonge = Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre };
        assert!(mesurer(&allonge, 16).elancement() > 1.5, "un treillis est élancé");
        assert!(!allonge.enveloppe_locale().est_sphere(), "…donc il doit porter une capsule");

        let ramasse = Composant::Noeud { profil: Profil::P1, sorties: crate::vaisseau::Sorties::Six };
        assert!(mesurer(&ramasse, 16).elancement() < 1.5, "un nœud est ramassé");
        assert!(ramasse.enveloppe_locale().est_sphere(), "…donc une sphère lui suffit");
    }

    // La silhouette doit avoir exactement une colonne par tranche, sans quoi on
    // ne peut pas lire une position dessus.
    #[test]
    fn la_silhouette_a_une_colonne_par_tranche() {
        let c = Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre };
        for n in [8usize, 24, 40] {
            assert_eq!(silhouette(&mesurer(&c, n)).chars().count(), n);
        }
    }
}
