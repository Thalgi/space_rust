//! **Instancier un plan** (G1.4) : construire le `Systeme` que l'écran dessine,
//! à partir d'un [`PlanSysteme`], **sans aucun tirage de genèse** — tout ce qui
//! fait le système est dans le plan.
//!
//! Deux parties :
//! - des **calculs purs**, testés : l'orbite d'un corps sur rails, la
//!   configuration d'un disque, le cadrage d'un multiple ;
//! - l'**assemblage**, qui crée les objets dessinables (étoiles, planètes,
//!   disques). Leurs constructeurs chargent des matériaux GPU : il ne se teste
//!   pas sans fenêtre, il se vérifie à l'écran (Skymap V3, G1.5).
//!
//! C'est ici, et seulement ici, que se décident les **nombres de rendu** : les
//! particules des ceintures suivent le réglage de qualité (§5.4, Opti-v2 O5),
//! sans changer le système.

use super::apparences;
use super::plan::{CorpsPlan, DisquePlan, EtoilePlan, FoyerPlan, GenreDisque, Nature, NoeudEtoiles, OrbitePlan, PlanSysteme};
use super::alea::Alea;
use crate::astre::Foyer;
use crate::disque::{Disque, DisqueConfig};
use crate::etoile::{self, ProfilEtoile};
use crate::genese::presets::{ECHELLE_CORPS, R_SOLEIL, R_TERRE};
use crate::genese::MASSE_ETOILE;
use crate::orbite::Orbite;
use crate::planete::Planete;
use crate::reglages::{self, Qualite};
use crate::soleil::Soleil;
use crate::stellaire::{ArbreStellaire, Variante};
use crate::systeme::{self, Systeme};
use macroquad::prelude::*;
use macroquad::rand::srand;

/// Particules d'une ceinture et d'une ceinture de Kuiper en qualité haute (les
/// valeurs de la V1).
const PARTICULES_CEINTURE: usize = 900;
const PARTICULES_KUIPER: usize = 1400;

/// Exposant de la **compression du rayon des étoiles** à l'écran (§7.14) : le Soleil
/// garde sa taille du système solaire, une supergéante de 700 rayons fait ~1 UA, une
/// naine blanche reste visible.
const COMPRESSION_ETOILES: f32 = 0.6;

/// **Le rayon d'une étoile à l'écran**, en unités du monde. Les objets compacts et la
/// quasi-étoile gardent leur taille de rendu : l'image d'un pulsar, ce sont son disque et
/// ses jets, proportionnels à son rayon ; à 20 km, ils disparaîtraient.
pub(crate) fn rayon_affiche(nom: &str, rayon_vrai: f32, rayon_rendu: f32) -> f32 {
    let compact = matches!(nom, "Trou noir" | "Pulsar" | "Etoile a neutrons" | "Magnetar") || etoile::est_quasi_etoile(nom);
    if compact {
        rayon_rendu
    } else {
        R_SOLEIL * rayon_vrai.powf(COMPRESSION_ETOILES)
    }
}

/// **Construit le système d'un plan**, et son libellé (comme la V1).
pub fn instancier(plan: &PlanSysteme) -> (Systeme, String) {
    let mut sys = Systeme::new();
    let info = match &plan.etoiles {
        NoeudEtoiles::Etoile(e) => poser_etoile_seule(&mut sys, e, plan.graine),
        racine => {
            let mut arbre = ArbreStellaire::new();
            let mut rang = 0;
            deployer(&mut sys, &mut arbre, racine, None, None, plan.graine, &mut rang);
            sys.definir_arbre(arbre);
            if let Some((k, d)) = vue(plan) {
                sys.definir_vue(k, d);
            }
            match rang {
                2 => "Binaire (aleatoire)".to_string(),
                3 => "Trinaire (A-B)+C - planetes autour de C".to_string(),
                _ => "Quadruple ((A-B)-C)+D - planetes autour de D".to_string(),
            }
        }
    };

    // L'ordre de la V1 : planètes et lunes, puis disques, puis comètes.
    let mut index = vec![usize::MAX; plan.corps.len()];
    for (i, c) in plan.corps.iter().enumerate().filter(|(_, c)| c.nature != Nature::Comete) {
        index[i] = poser_corps(&mut sys, c, &index);
    }
    let orbites: Vec<f32> = plan.corps.iter().filter_map(|c| match (&c.nature, &c.orbite) {
        (Nature::Planete(_), OrbitePlan::Rails { a_ua, .. }) => Some(*a_ua),
        _ => None,
    }).collect();
    for d in &plan.disques {
        // Le grand disque part du bord du petit disque de l'étoile, dont la taille est
        // celle de l'affichage ; sans la place de s'étendre, il n'est pas posé.
        let mut d = d.clone();
        if d.genre == GenreDisque::GrandDisque {
            d.interne_ua = interne_grand_disque(&plan.etoiles);
            if d.externe_ua <= d.interne_ua * 1.5 {
                continue;
            }
        }
        // ponytail: `Disque::new` tire ses particules dans le flux global ; on
        // l'amorce par la graine du disque, donc il est rejoué à l'identique.
        // Disparaît si le disque reçoit un jour son propre `Alea`.
        srand(d.graine);
        sys.ajouter(Box::new(Disque::new(config_disque(&d, &orbites, reglages::qualite()))));
    }
    for (i, c) in plan.corps.iter().enumerate().filter(|(_, c)| c.nature == Nature::Comete) {
        index[i] = poser_corps(&mut sys, c, &index);
    }
    (sys, info)
}

/// Le bord interne du grand disque, en UA : le bord du petit disque de l'étoile.
fn interne_grand_disque(n: &NoeudEtoiles) -> f32 {
    let NoeudEtoiles::Etoile(e) = n else { return 0.0 };
    crate::soleil::RAYON_PROTOSOLAIRE.1 * rendu(e).0.rayon / etoile::UA
}

/// **Ce que l'écran montre d'une étoile du plan** : le profil et son activité. Le
/// gabarit donne la luminosité de rendu et l'activité, inchangées (§7.14) ; le rayon
/// est celui de l'affichage ; la couleur se déduit du type et de la température.
fn rendu(e: &EtoilePlan) -> (ProfilEtoile, f32) {
    let g = etoile::gabarit_du_type(&e.type_nom, e.q).unwrap_or_else(|| panic!("type d'étoile inconnu du tirage : {}", e.type_nom));
    let (rayon_rendu, luminosite, nom) = (g.2, g.3, g.4);
    let profil = ProfilEtoile {
        temperature: e.temperature,
        couleur: etoile::couleur_etoile(nom, e.temperature),
        rayon: rayon_affiche(nom, e.rayon, rayon_rendu),
        luminosite,
        nom,
        couronne: e.couronne,
    };
    (profil, etoile::activite_type(rayon_rendu, luminosite))
}

/// Une étoile du plan en `Soleil`, sa variante et son activité appliquées.
fn soleil(p: &ProfilEtoile, activite: f32, graine: u64) -> Soleil {
    // ponytail: `Soleil::new` tire sa graine et ses minuteries dans le flux
    // global ; on l'amorce, pour qu'une étoile soit rejouée à l'identique.
    srand(graine);
    let s = Soleil::new(Vec3::ZERO, p.rayon, p.couleur, p.luminosite).avec_activite(activite);
    let s = Variante::de_mode(p.couronne).appliquer(s, p.nom);
    if p.est_remnant() { s.sans_zone_habitable() } else { s }
}

/// La graine qui amorce la k-ième étoile : dérivée du plan, jamais stockée.
fn graine_soleil(graine: u64, k: usize) -> u64 {
    Alea::flux(graine, "soleil", k as u64).suivant()
}

/// Le centre d'un système simple ; rend le libellé de la V1. Une naine brune
/// n'est pas une photosphère : elle se construit en géante gazeuse au centre.
fn poser_etoile_seule(sys: &mut Systeme, e: &EtoilePlan, graine: u64) -> String {
    let (p, activite) = rendu(e);
    let mut pollution = None;
    if p.est_naine_brune() {
        let app = crate::genese::preset_gazeuse(crate::genese::preset_naine_brune(p.temperature));
        sys.ajouter(Box::new(
            Planete::new(Vec3::ZERO, Vec3::ZERO, p.rayon, e.masse * MASSE_ETOILE, app, Vec::new()).au_centre_du_systeme(p.luminosite),
        ));
    } else {
        let s = soleil(&p, activite, graine_soleil(graine, 0));
        pollution = s.pollution();
        sys.ajouter(Box::new(s));
    }
    match pollution {
        Some(pol) => format!("Etoile {} ({pol})  -  {} K", p.nom(), p.temperature as i32),
        None => format!("Etoile {}  -  {} K", p.nom(), p.temperature as i32),
    }
}

/// Masse d'un sous-arbre stellaire, en unités du jeu.
fn masse(n: &NoeudEtoiles) -> f32 {
    match n {
        NoeudEtoiles::Etoile(e) => e.masse * MASSE_ETOILE,
        NoeudEtoiles::Paire { a, b, .. } => masse(a) + masse(b),
    }
}

/// Repère d'une orbite : `a1` vers le périastre (l'azimut), `q` dans le sens du
/// mouvement, relevé de l'inclinaison.
fn repere(azimut: f32, incl: f32) -> (Vec3, Vec3) {
    let a1 = vec3(azimut.cos(), 0.0, azimut.sin());
    let a2 = vec3(-azimut.sin(), 0.0, azimut.cos());
    (a1, (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize())
}

/// Déploie l'arbre stellaire (`genese::deployer_noeud`), l'azimut lu dans le plan.
fn deployer(
    sys: &mut Systeme,
    arbre: &mut ArbreStellaire,
    noeud: &NoeudEtoiles,
    parent: Option<usize>,
    orbite: Option<Orbite>,
    graine: u64,
    rang: &mut usize,
) -> usize {
    match noeud {
        NoeudEtoiles::Etoile(e) => {
            let (p, activite) = rendu(e);
            let mut s = soleil(&p, activite, graine_soleil(graine, *rang));
            *rang += 1;
            s.base.masse = e.masse * MASSE_ETOILE; // barycentres justes
            let idx = sys.ajouter(Box::new(s));
            arbre.ajouter(parent, orbite, Some(idx))
        }
        NoeudEtoiles::Paire { a, b, sep, e, incl, phase, azimut } => {
            let idx = arbre.ajouter(parent, orbite, None);
            let (ma, mb) = (masse(a), masse(b));
            let m = (ma + mb).max(1e-6);
            let sep_monde = sep * etoile::UA;
            let (a1, q) = repere(*azimut, *incl);
            // Même période des deux côtés ; chacun à une distance du barycentre
            // proportionnelle à la masse de l'autre, A à l'opposé.
            let n = (systeme::G * m / (sep_monde * sep_monde * sep_monde)).sqrt();
            let orb_a = Orbite::avec_n(sep_monde * mb / m, *e, -a1, -q, n, *phase);
            let orb_b = Orbite::avec_n(sep_monde * ma / m, *e, a1, q, n, *phase);
            deployer(sys, arbre, a, Some(idx), Some(orb_a), graine, rang);
            deployer(sys, arbre, b, Some(idx), Some(orb_b), graine, rang);
            idx
        }
    }
}

/// **L'orbite d'un corps sur rails** et son foyer : Kepler, départ au
/// périastre, dans la direction de l'azimut (`genese::poser_sur_rails`).
pub(crate) fn orbite_rails(o: &OrbitePlan) -> Option<(Orbite, Foyer)> {
    let OrbitePlan::Rails { foyer, masse_centrale, a_ua, e, incl, azimut } = *o else { return None };
    let (a1, q) = repere(azimut, incl);
    let foyer = match foyer {
        FoyerPlan::Barycentre => Foyer::Barycentre,
        FoyerPlan::Etoile(k) => Foyer::Etoile(k),
    };
    Some((Orbite::new(a_ua * etoile::UA, e, a1, q, systeme::G * masse_centrale * MASSE_ETOILE, 0.0), foyer))
}

/// **Le rayon d'un corps à l'écran** : le vrai, grossi comme ceux du système solaire
/// (×5, §7.14). Une comète garde le sien, déjà en unités du monde.
pub(crate) fn rayon_corps(c: &CorpsPlan) -> f32 {
    if c.nature == Nature::Comete { c.rayon } else { c.rayon * R_TERRE * ECHELLE_CORPS }
}

/// Pose une planète, une lune ou une comète ; rend son index dans le système.
fn poser_corps(sys: &mut Systeme, c: &CorpsPlan, index: &[usize]) -> usize {
    match (&c.nature, &c.orbite) {
        (Nature::Lune(r), OrbitePlan::Lune { parent, rayon_orbite, omega, incl, phase }) => {
            // L'orbite d'une lune grossit avec sa planète : sinon celle-ci l'avalerait.
            let lune = Planete::new(Vec3::ZERO, Vec3::ZERO, rayon_corps(c), c.masse, apparences::lune(r), Vec::new())
                .en_lune(index[parent.0 as usize], rayon_orbite * etoile::UA * ECHELLE_CORPS, *omega, *incl, *phase);
            sys.ajouter(Box::new(lune))
        }
        (nature, orbite) => {
            let (orb, foyer) = orbite_rails(orbite).expect("planètes et comètes sont sur rails");
            let app = match nature {
                Nature::Planete(r) => apparences::planete(r).2,
                _ => crate::genese::apparence_comete(),
            };
            let (pos, vel) = orb.etat(0.0);
            let corps = Planete::new(pos, vel, rayon_corps(c), c.masse, app, orb.polyligne(Orbite::N_TRACE))
                .avec_orbite(orb)
                .avec_foyer(foyer);
            let corps = match (nature, orbite) {
                // La graine du noyau se dérive de l'orbite, comme en V1.
                (Nature::Comete, OrbitePlan::Rails { a_ua, e, .. }) => corps.en_comete((a_ua * 7.31 + e * 3.17).fract()),
                _ => corps,
            };
            sys.ajouter(Box::new(corps))
        }
    }
}

/// **La configuration d'un disque**, avec ses particules décidées par la
/// qualité : la moitié en qualité basse. Le système, lui, ne change pas.
pub(crate) fn config_disque(d: &DisquePlan, orbites_ua: &[f32], qualite: Qualite) -> DisqueConfig {
    let part = |n: usize| match qualite {
        Qualite::Haute => n,
        Qualite::Basse => n / 2,
    };
    let (i, x, masse) = (d.interne_ua * etoile::UA, d.externe_ua * etoile::UA, d.masse_centrale * MASSE_ETOILE);
    match d.genre {
        GenreDisque::Ceinture => DisqueConfig::asteroides(part(PARTICULES_CEINTURE), i, x, masse),
        GenreDisque::Kuiper => DisqueConfig::kuiper(part(PARTICULES_KUIPER), i, x, masse),
        GenreDisque::GrandDisque => {
            let mut c = crate::disque::habillage::grand_disque(d.interne_ua, d.externe_ua, masse, orbites_ua);
            c.nb = part(c.nb);
            c
        }
    }
}

/// **Le cadrage d'un multiple** : la première étoile qui porte des planètes,
/// vue d'assez loin pour toutes les montrer. `None` sans planète autour d'une
/// étoile (circumbinaires seules : le cadrage par défaut convient).
pub(crate) fn vue(plan: &PlanSysteme) -> Option<(usize, f32)> {
    let k = plan.corps.iter().find_map(|c| match c.orbite {
        OrbitePlan::Rails { foyer: FoyerPlan::Etoile(k), .. } if matches!(c.nature, Nature::Planete(_)) => Some(k),
        _ => None,
    })?;
    let a_max = plan
        .corps
        .iter()
        .filter_map(|c| match c.orbite {
            OrbitePlan::Rails { foyer: FoyerPlan::Etoile(j), a_ua, .. } if j == k => Some(a_ua),
            _ => None,
        })
        .fold(0.0_f32, f32::max);
    Some((k, (a_max * etoile::UA * 3.0).max(20.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genese_v2::planifier::planifier;

    /// Un corps part de son périastre, à `a(1−e)`, dans la direction de son
    /// azimut, et son plan est relevé de son inclinaison.
    #[test]
    fn un_corps_part_de_son_periastre_dans_la_direction_de_son_azimut() {
        let o = OrbitePlan::Rails { foyer: FoyerPlan::Etoile(1), masse_centrale: 0.9, a_ua: 2.0, e: 0.25, incl: 0.3, azimut: 1.2 };
        let (orb, foyer) = orbite_rails(&o).unwrap();
        assert!(matches!(foyer, Foyer::Etoile(1)));
        let (pos, vel) = orb.etat(0.0);
        assert!((pos.length() - 2.0 * 0.75 * etoile::UA).abs() < 1e-2, "périastre à {}", pos.length());
        let dir = pos.normalize();
        assert!((dir - vec3(1.2_f32.cos(), 0.0, 1.2_f32.sin())).length() < 1e-4, "direction {dir}");
        let pente = vel.normalize().y;
        assert!((pente - 0.3_f32.sin()).abs() < 1e-4, "pente {pente}");
        assert!(orbite_rails(&OrbitePlan::Lune { parent: super::super::plan::IdCorps(0), rayon_orbite: 1.0, omega: 1.0, incl: 0.0, phase: 0.0 }).is_none());
    }

    /// **La Terre à l'écran vaut celle du système solaire** : même rayon pour un rayon
    /// terrestre, la comète garde le sien.
    #[test]
    fn un_corps_a_le_rayon_du_systeme_solaire() {
        let corps = |nature, rayon| CorpsPlan {
            nature,
            orbite: OrbitePlan::Rails { foyer: FoyerPlan::Barycentre, masse_centrale: 1.0, a_ua: 1.0, e: 0.0, incl: 0.0, azimut: 0.0 },
            rayon,
            masse: 1.0,
        };
        let recette = super::super::plan::RecetteApparence { type_corps: super::super::plan::TypeCorps::Tellurique, temperature: 288.0, graine: 1 };
        assert_eq!(rayon_corps(&corps(Nature::Planete(recette), 1.0)), crate::genese::presets::rayon_monde("Terre"));
        assert_eq!(rayon_corps(&corps(Nature::Comete, 0.08)), 0.08);
    }

    /// **La taille des étoiles à l'écran** (§7.14) : le Soleil celle du système solaire,
    /// une supergéante de 700 rayons autour d'une UA, une naine blanche visible mais
    /// petite, les objets compacts leur taille de rendu.
    #[test]
    fn la_taille_des_etoiles_a_l_ecran() {
        assert_eq!(rayon_affiche("G (jaune)", 1.0, 2.0), R_SOLEIL);
        let ua = rayon_affiche("Supergeante rouge", 700.0, 10.0) / etoile::UA;
        assert!((0.7..1.5).contains(&ua), "supergéante : {ua} UA");
        let naine = rayon_affiche("Naine blanche", 0.012, 0.6);
        assert!((0.03..0.2).contains(&(naine / R_SOLEIL)), "naine blanche : {naine}");
        for nom in ["Trou noir", "Pulsar", "Etoile a neutrons", "Magnetar", "Quasi-etoile"] {
            assert_eq!(rayon_affiche(nom, 1e-5, 0.42), 0.42, "{nom}");
        }
    }

    /// **La luminosité et l'activité de rendu ne bougent pas** (§7.14) : une étoile
    /// d'éclat vrai 100 000 reste à son éclat de jeu, à l'écran.
    #[test]
    fn le_rendu_d_une_etoile_ne_lit_pas_ses_vraies_valeurs() {
        for g in 0..200 {
            let plan = crate::genese_v2::planifier::planifier_autour(g, "O (bleue)").unwrap();
            let NoeudEtoiles::Etoile(e) = &plan.etoiles else { panic!("une étoile") };
            let gabarit = etoile::gabarit_du_type("O (bleue)", e.q).unwrap();
            let (p, activite) = rendu(e);
            assert_eq!(p.luminosite, gabarit.3);
            assert_eq!(activite, etoile::activite_type(gabarit.2, gabarit.3));
            assert!(e.luminosite > 1000.0 * p.luminosite, "vraie {} contre rendu {}", e.luminosite, p.luminosite);
        }
    }

    /// **Aucune étoile à l'écran ne recouvre sa première planète** : c'est la garde qui
    /// tient `COMPRESSION_ETOILES` et le plancher d'étalement ensemble.
    #[test]
    fn l_etoile_a_l_ecran_ne_recouvre_pas_ses_planetes() {
        for g in 0..2000 {
            let plan = planifier(g);
            let NoeudEtoiles::Etoile(e) = &plan.etoiles else { continue };
            let rayon = rendu(e).0.rayon;
            for c in &plan.corps {
                if let (Nature::Planete(_), OrbitePlan::Rails { a_ua, e: ex, .. }) = (&c.nature, &c.orbite) {
                    let periastre = a_ua * (1.0 - ex) * etoile::UA;
                    assert!(periastre > rayon, "graine {g}, {} : périastre {periastre}, rayon {rayon}", e.type_nom);
                }
            }
        }
    }

    /// Les particules suivent la qualité ; les rayons, eux, ne bougent pas.
    #[test]
    fn la_qualite_change_les_particules_pas_le_disque() {
        for g in 0..200 {
            let plan = planifier(g);
            let orbites: Vec<f32> = plan.corps.iter().filter_map(|c| match c.orbite {
                OrbitePlan::Rails { a_ua, .. } if matches!(c.nature, Nature::Planete(_)) => Some(a_ua),
                _ => None,
            }).collect();
            for d in &plan.disques {
                let haute = config_disque(d, &orbites, Qualite::Haute);
                let basse = config_disque(d, &orbites, Qualite::Basse);
                assert_eq!((haute.interne, haute.externe), (basse.interne, basse.externe), "graine {g}");
                assert!((haute.interne - d.interne_ua * etoile::UA).abs() < 1e-3, "graine {g}");
                assert!(basse.nb * 2 <= haute.nb && basse.nb > 0, "graine {g} : {} contre {}", basse.nb, haute.nb);
            }
        }
    }

    /// Le cadrage d'un multiple vise l'étoile qui porte des planètes, d'assez
    /// loin pour que l'apoastre de la plus lointaine **tienne dans le champ**
    /// de la caméra (`camera::DEMI_FOV`, la hauteur, le plus étroit).
    #[test]
    fn le_cadrage_vise_l_etoile_qui_porte_des_planetes() {
        let mut vus = 0;
        for g in 0..500 {
            let plan = planifier(g);
            let Some((k, d)) = vue(&plan) else { continue };
            vus += 1;
            let apoastres: Vec<f32> = plan.corps.iter().filter_map(|c| match c.orbite {
                OrbitePlan::Rails { foyer: FoyerPlan::Etoile(j), a_ua, e, .. } if j == k => Some(a_ua * (1.0 + e)),
                _ => None,
            }).collect();
            assert!(!apoastres.is_empty(), "graine {g} : aucune planète autour de l'étoile {k}");
            let demi_champ = d * crate::camera::DEMI_FOV.tan();
            for q in apoastres {
                assert!(q * etoile::UA <= demi_champ, "graine {g} : apoastre à {q} UA hors du champ ({demi_champ})");
            }
        }
        assert!(vus > 50, "seulement {vus} multiples cadrés");
    }
}
