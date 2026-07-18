mod apparences;
mod persistance;
mod presets;
mod taille;

pub use taille::{ClasseTaille, rayon_visuel};

pub use persistance::{charger_presets, sauver_presets, PresetSauve};
pub use presets::{
    construire_preset_alpha_centauri, construire_preset_avatar, construire_preset_binaire,
    construire_preset_proxima, construire_preset_quadruple, construire_preset_solaire,
    construire_preset_tau_ceti, construire_preset_trinaire,
};

use crate::astre::Foyer;
use crate::disque::{Disque, DisqueConfig};
use crate::etoile::{self, ProfilEtoile};
use crate::planete::{Apparence, Planete, TypePlanete};
use crate::soleil::Soleil;
use crate::stabilite;
use crate::stellaire::{ArbreStellaire, Feuille, Noeud, Variante};
use crate::systeme::{self, Systeme};
pub use apparences::apparence_gazeuse; // aussi utilisée par la galerie (cellules aléatoires)
use apparences::{apparence_glacee, apparence_tellurique, gazeuse, tellurique};
use macroquad::prelude::*;
use macroquad::rand::{gen_range, srand};
use std::f32::consts::TAU;

pub const MASSE_ETOILE: f32 = 1000.0; // masse gravitationnelle (indépendante du rayon visuel)

/// Construit un système aléatoire à partir d'une graine. Renvoie aussi un texte d'info.
/// La multiplicité (1 à 4 étoiles) est dérivée du seed lui-même, ce qui laisse la
/// séquence RNG du cas mono-étoile intacte (systèmes sauvegardés reproductibles).
pub fn construire_systeme(seed: u64) -> (Systeme, String) {
    srand(seed);
    match seed % 100 {
        0..=54 => construire_simple(),  // ~55 % : étoile unique
        55..=82 => construire_multiple(2), // binaire
        83..=93 => construire_multiple(3), // trinaire hiérarchique
        _ => construire_multiple(4),       // quadruple
    }
}

/// Système à une seule étoile (Titius-Bode + ceintures). `srand` déjà appelé.
fn construire_simple() -> (Systeme, String) {
    let profil = ProfilEtoile::aleatoire();

    let mut sys = Systeme::new();
    let soleil = Soleil::new(vec3(0.0, 0.0, 0.0), profil.rayon, profil.couleur, profil.luminosite);
    let soleil = match profil.couronne as i32 {
        1 => soleil.avec_jets(),
        2 => soleil.avec_vent(),
        3 => soleil.avec_pulsar(),
        4 => soleil.avec_magnetar(),
        _ => soleil,
    };
    // Rémanents (pulsar/magnétar/étoile à neutrons) : pas de zone habitable à tracer.
    let soleil = if profil.est_remnant() {
        soleil.sans_zone_habitable()
    } else {
        soleil
    };
    sys.ajouter(Box::new(soleil));

    // Échelle du système (UA), ancrée sur le TYPE d'étoile — distances à l'échelle :
    //  - étoiles normales : ∝ √L, car la zone habitable se déplace en √L. Étalonné
    //    pour que le Soleil (L=1) reproduise les distances historiques (~0,4–0,8 UA).
    //    Un astre lumineux étale donc son système, une naine le resserre.
    //  - rémanents : aucune zone habitable ; planètes rescapées sur orbites serrées
    //    (cf. PSR B1257+12), indépendamment de leur maigre éclat visible.
    let ech: f32 = if profil.est_remnant() {
        gen_range(0.25_f32, 0.5)
    } else {
        profil.luminosite.max(0.001).sqrt()
    };

    // Planètes : distances en UA suivant une loi de Titius-Bode, mises à l'échelle.
    let nb: i32 = gen_range(3, 7);
    let mut a: f32 = gen_range(0.4, 0.8) * ech;
    // Dernière planète posée (a, rayon) -> évite que deux corps se chevauchent quand
    // l'espacement géométrique les rapproche trop (systèmes compacts / rémanents).
    let mut precedent: Option<(f32, f32)> = None;
    for _ in 0..nb {
        // Position normalisée (équivalent solaire) : le TYPE dépend de la distance
        // relative à la ligne des neiges, qui suit l'échelle `ech`. Sans ça, une
        // étoile brillante (a étalé) classerait ses mondes internes comme glacés.
        let a_norm = a / ech;
        let p: f32 = gen_range(0.0, 1.0);
        let type_p = if a_norm < 2.0 {
            if p < 0.8 { TypePlanete::Tellurique } else { TypePlanete::Gazeuse }
        } else if a_norm < 6.0 {
            if p < 0.5 {
                TypePlanete::Gazeuse
            } else if p < 0.85 {
                TypePlanete::Tellurique
            } else {
                TypePlanete::Glacee
            }
        } else if p < 0.55 {
            TypePlanete::Glacee
        } else if p < 0.8 {
            TypePlanete::Gazeuse
        } else {
            TypePlanete::Tellurique
        };

        let temp = etoile::temp_equilibre(profil.luminosite, a);
        let (rayon, masse, app) = match type_p {
            TypePlanete::Tellurique => apparence_tellurique(temp),
            TypePlanete::Gazeuse => apparence_gazeuse(),
            TypePlanete::Glacee => apparence_glacee(),
        };

        // Garde-fou anti-collision : au moins (r_prec + r + marge) entre deux centres.
        if let Some((a_prec, r_prec)) = precedent {
            let min_a = a_prec + (r_prec + rayon) / etoile::UA + 0.03;
            a = a.max(min_a);
        }

        let e: f32 = gen_range(0.0, 0.2);
        let incl: f32 = gen_range(0.0, 0.16);
        let idx = ajouter_planete(&mut sys, a, e, incl, rayon, masse, app);
        precedent = Some((a, rayon));

        let n_lunes: i32 = match type_p {
            TypePlanete::Gazeuse => gen_range(1, 4),
            TypePlanete::Tellurique => if a_norm > 0.8 { gen_range(0, 2) } else { 0 },
            TypePlanete::Glacee => gen_range(0, 2),
        };
        for _ in 0..n_lunes {
            ajouter_lune(&mut sys, idx, rayon);
        }

        a *= gen_range(1.5_f32, 1.85);
    }

    // Ceinture principale + ceinture de Kuiper — mises à la même échelle `ech` que
    // les planètes (la ligne des neiges suit aussi la luminosité), pour rester
    // cohérentes avec le système au lieu d'un rayon fixe.
    let bi: f32 = gen_range(2.0, 3.0) * ech;
    let bo: f32 = bi + gen_range(0.6, 1.4) * ech;
    sys.ajouter(Box::new(Disque::new(DisqueConfig::asteroides(
        900, bi * etoile::UA, bo * etoile::UA, MASSE_ETOILE,
    ))));
    let ki: f32 = bo + gen_range(20.0, 30.0) * ech;
    let ko: f32 = ki + gen_range(12.0, 20.0) * ech;
    sys.ajouter(Box::new(Disque::new(DisqueConfig::kuiper(
        1400, ki * etoile::UA, ko * etoile::UA, MASSE_ETOILE,
    ))));

    let info = format!("Etoile {}  -  {} K", profil.nom(), profil.temperature as i32);
    (sys, info)
}

/// Plan de placement des planètes d'un système multiple (zone stable circumbinaire).
struct Plan {
    foyer: Foyer,
    masse_centrale: f32,
    lumi_ref: f32, // luminosité de référence (température/HZ des planètes)
    a_min: f32,    // demi-grand axe min (UA) — hors des orbites stellaires
    a_max: f32,    // demi-grand axe max (UA)
}

/// Tire une étoile-feuille aléatoire. Renvoie (feuille, masse, luminosité).
fn feuille_alea() -> (Feuille, f32, f32) {
    let p = ProfilEtoile::aleatoire();
    // Masse approchée par la relation masse-luminosité (L ∝ M^3.5 -> M ∝ L^0.286).
    let masse = (MASSE_ETOILE * p.luminosite.powf(0.286)).clamp(0.25 * MASSE_ETOILE, 5.0 * MASSE_ETOILE);
    let var = match p.couronne as i32 {
        1 => Variante::Jets,
        2 => Variante::Vent,
        3 => Variante::Pulsar,
        4 => Variante::Magnetar,
        _ => Variante::Normale,
    };
    (
        Feuille::new(p.rayon, p.couleur, p.luminosite, masse)
            .variante(var)
            .avec_remnant(p.est_remnant()),
        masse,
        p.luminosite,
    )
}

/// Tire une séparation en UA suivant une loi **log-uniforme** (approx. de la
/// log-normale observée, Raghavan et al. 2010 : pic ~40 UA, très large).
fn sep_log(min_ua: f32, max_ua: f32) -> f32 {
    gen_range(min_ua.ln(), max_ua.ln()).exp()
}

/// Plan « type S » : planètes autour de l'étoile-feuille `k` (0.3 UA .. 75 % du rayon
/// critique circumstellaire).
fn plan_s(k: usize, masse_hote: f32, lumi_hote: f32, a_crit: f32) -> Plan {
    Plan {
        foyer: Foyer::Etoile(k),
        masse_centrale: masse_hote,
        lumi_ref: lumi_hote,
        a_min: 0.3,
        a_max: a_crit * 0.75,
    }
}

/// Plan « type P » : planètes circumbinaires autour du barycentre, au-delà du rayon
/// critique circumbinaire (marge +25 %).
fn plan_p(sep: f32, m1: f32, m2: f32, l1: f32, l2: f32, e: f32) -> Plan {
    let a_min = stabilite::a_crit_p(sep, stabilite::rapport_masse(m1, m2), e) * 1.25;
    Plan {
        foyer: Foyer::Barycentre,
        masse_centrale: m1 + m2,
        lumi_ref: l1 + l2,
        a_min,
        a_max: a_min * 2.4,
    }
}

/// Construit un arbre stellaire aléatoire (`nb` étoiles) + la LISTE des zones stables
/// à peupler : type S autour d'une étoile ET/OU type P circumbinaire, qui **coexistent**.
/// Séparations réalistes (log-uniforme ~Raghavan) + ratios hiérarchiques stables (Holman-Wiegert).
fn arbre_et_plan(nb: usize) -> (Noeud, String, Vec<Plan>) {
    let inc = || gen_range(-0.3_f32, 0.3);
    let ph = || gen_range(0.0_f32, TAU);
    let chance = |p: f32| gen_range(0.0_f32, 1.0) < p;

    match nb {
        2 => {
            let (mut fa, mut ma, mut la) = feuille_alea();
            let (mut fb, mut mb, mut lb) = feuille_alea();
            // Hôte (étoile primaire, index 0) = la plus massive.
            if mb > ma {
                std::mem::swap(&mut fa, &mut fb);
                std::mem::swap(&mut ma, &mut mb);
                std::mem::swap(&mut la, &mut lb);
            }
            // Séparation réaliste (log-uniforme, ~Raghavan 2010) : de serrée à large.
            let sep = sep_log(1.0, 60.0);
            let e = gen_range(0.0, 0.45);
            let racine = Noeud::paire(Noeud::etoile(fa), Noeud::etoile(fb), sep, e, inc(), ph());

            // Zones stables (peuvent COEXISTER) : S autour de A, S autour de B, P circumbinaire.
            let mut plans = Vec::new();
            let a_s_a = stabilite::a_crit_s(sep, mb / (ma + mb), e); // zone autour de A (primaire)
            if a_s_a >= 0.8 {
                // Binaire assez large -> système planétaire type S (cas « Avatar »).
                plans.push(plan_s(0, ma, la, a_s_a));
            }
            let a_s_b = stabilite::a_crit_s(sep, ma / (ma + mb), e); // zone autour de B (secondaire)
            if a_s_b >= 0.8 && chance(0.5) {
                plans.push(plan_s(1, mb, lb, a_s_b));
            }
            // P-type circumbinaire : garanti si aucune zone S (binaire serré) ; sinon
            // COEXISTE avec le S, mais seulement si le rayon critique reste raisonnable
            // (pas de planète à des centaines d'UA pour un binaire très large).
            let a_p = stabilite::a_crit_p(sep, stabilite::rapport_masse(ma, mb), e);
            if plans.is_empty() || (a_p < 30.0 && chance(0.6)) {
                plans.push(plan_p(sep, ma, mb, la, lb, e));
            }
            (racine, "Binaire (aleatoire)".to_string(), plans)
        }
        3 => {
            // ((A·B) · C) : paire SERRÉE + étoile C ISOLÉE à grande distance.
            // C (isolée) héberge un vrai système planétaire type S ; AB = double
            // étoile lointaine dans son ciel.
            let (fa, ma, _) = feuille_alea();
            let (fb, mb, _) = feuille_alea();
            let (fc, mc, lc) = feuille_alea();
            let sep_in = sep_log(0.3, 1.5); // paire interne serrée
            let e_in = gen_range(0.0, 0.3);
            let ab = Noeud::paire(Noeud::etoile(fa), Noeud::etoile(fb), sep_in, e_in, inc(), ph());
            let m_ab = ma + mb;
            let e_out = gen_range(0.0, 0.4);
            let sep_out_min = stabilite::a_crit_p(sep_in, stabilite::rapport_masse(ma, mb), e_in) * 1.6;
            let lo = sep_out_min.max(15.0); // C nettement à l'écart (>= 15 UA)
            let sep_out = sep_log(lo, lo.max(45.0));
            let racine = Noeud::paire(ab, Noeud::etoile(fc), sep_out, e_out, inc(), ph());
            // C = feuille d'index 2 (ordre de déploiement A, B, C).
            let a_s_c = stabilite::a_crit_s(sep_out, m_ab / (m_ab + mc), e_out);
            let plans = vec![plan_s(2, mc, lc, a_s_c.max(1.0))];
            (racine, "Trinaire (A-B)+C - planetes autour de C".to_string(), plans)
        }
        _ => {
            // (((A·B) · C) · D) : triple interne COMPACT + étoile D ISOLÉE à grande
            // distance. D héberge un système planétaire type S.
            let (fa, ma, _) = feuille_alea();
            let (fb, mb, _) = feuille_alea();
            let (fc, mc, _) = feuille_alea();
            let (fd, md, ld) = feuille_alea();
            // Paire interne serrée.
            let sep_ab = sep_log(0.3, 1.2);
            let e_ab = gen_range(0.0, 0.3);
            let ab = Noeud::paire(Noeud::etoile(fa), Noeud::etoile(fb), sep_ab, e_ab, inc(), ph());
            let m_ab = ma + mb;
            // C à distance modérée -> triple compact ((A·B)·C).
            let e_c = gen_range(0.0, 0.3);
            let sep_c_min = stabilite::a_crit_p(sep_ab, stabilite::rapport_masse(ma, mb), e_ab) * 1.6;
            let sep_c = sep_c_min.max(2.0) * gen_range(1.0, 2.0);
            let abc = Noeud::paire(ab, Noeud::etoile(fc), sep_c, e_c, inc(), ph());
            let m_abc = m_ab + mc;
            // D isolée, loin -> système type S autour de D.
            let e_out = gen_range(0.0, 0.4);
            let sep_out_min = stabilite::a_crit_p(sep_c, stabilite::rapport_masse(m_ab, mc), e_c) * 1.6;
            let lo = sep_out_min.max(20.0);
            let sep_out = sep_log(lo, lo.max(55.0));
            let racine = Noeud::paire(abc, Noeud::etoile(fd), sep_out, e_out, inc(), ph());
            // D = feuille d'index 3 (ordre de déploiement A, B, C, D).
            let a_s_d = stabilite::a_crit_s(sep_out, m_abc / (m_abc + md), e_out);
            let plans = vec![plan_s(3, md, ld, a_s_d.max(1.0))];
            (racine, "Quadruple ((A-B)-C)+D - planetes autour de D".to_string(), plans)
        }
    }
}

/// Système à plusieurs étoiles : arbre hiérarchique + planètes placées dans CHAQUE
/// zone stable (type S autour d'une étoile ET/OU type P circumbinaire). `srand` déjà appelé.
fn construire_multiple(nb: usize) -> (Systeme, String) {
    let mut sys = Systeme::new();
    let (racine, info, plans) = arbre_et_plan(nb);
    deployer_arbre(&mut sys, racine);

    let mut focus: Option<(usize, f32)> = None;
    for plan in &plans {
        placer_planetes(&mut sys, plan);
        // Vue par défaut : focaliser la 1re étoile hôte (zone S) sur sa zone planétaire
        // — un système type S est trop étalé pour voir ses planètes en cadrant tout.
        if focus.is_none() {
            if let Foyer::Etoile(k) = plan.foyer {
                focus = Some((k, (plan.a_max * etoile::UA * 3.0).max(150.0)));
            }
        }
    }
    if let Some((k, d)) = focus {
        sys.definir_vue(k, d);
    }

    (sys, info)
}

/// Peuple une zone stable (`plan`) d'une courte séquence de planètes (Titius-Bode),
/// de `a_min` à `a_max`, autour du foyer du plan.
fn placer_planetes(sys: &mut Systeme, plan: &Plan) {
    let n_pl: i32 = gen_range(1, 5);
    let mut a = plan.a_min * gen_range(1.0, 1.3);
    for _ in 0..n_pl {
        if a > plan.a_max {
            break;
        }
        let temp = etoile::temp_equilibre(plan.lumi_ref, a);
        let p: f32 = gen_range(0.0, 1.0);
        let type_p = if temp > 250.0 {
            if p < 0.55 { TypePlanete::Tellurique } else { TypePlanete::Gazeuse }
        } else if temp > 150.0 {
            if p < 0.5 { TypePlanete::Gazeuse } else { TypePlanete::Glacee }
        } else {
            TypePlanete::Glacee
        };
        let (rayon, masse, app) = match type_p {
            TypePlanete::Tellurique => apparence_tellurique(temp),
            TypePlanete::Gazeuse => apparence_gazeuse(),
            TypePlanete::Glacee => apparence_glacee(),
        };
        let e: f32 = gen_range(0.0, 0.12);
        let incl: f32 = gen_range(0.0, 0.08);
        let idx = ajouter_planete_autour(sys, plan.foyer, plan.masse_centrale, a, e, incl, rayon, masse, app);
        let n_lunes: i32 = match type_p {
            TypePlanete::Gazeuse => gen_range(1, 4),
            _ => gen_range(0, 2),
        };
        for _ in 0..n_lunes {
            ajouter_lune(sys, idx, rayon);
        }
        a *= gen_range(1.4, 1.8);
    }
}

/// Ajoute une planète orbitant un `foyer` (étoile hôte S-type, ou barycentre P-type),
/// autour d'une `masse_centrale` donnée. `a_au` = demi-grand axe relatif au foyer.
pub(crate) fn ajouter_planete_autour(
    sys: &mut Systeme,
    foyer: Foyer,
    masse_centrale: f32,
    a_au: f32,
    e: f32,
    incl: f32,
    rayon: f32,
    masse: f32,
    app: Apparence,
) -> usize {
    let a_monde = a_au * etoile::UA;
    let phi: f32 = gen_range(0.0, TAU);
    let a1 = vec3(phi.cos(), 0.0, phi.sin());
    let a2 = vec3(-phi.sin(), 0.0, phi.cos());
    let q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();

    // Orbite de Kepler analytique (relative au foyer). m0 = 0 -> départ au périastre.
    let mu = systeme::G * masse_centrale;
    let orb = crate::orbite::Orbite::new(a_monde, e, a1, q, mu, 0.0);
    let (pos, vel) = orb.etat(0.0);
    let orbite = orb.polyligne(96);
    sys.ajouter(Box::new(
        Planete::new(pos, vel, rayon, masse, app, orbite)
            .avec_orbite(orb)
            .avec_foyer(foyer),
    ))
}

/// Planète autour de l'étoile unique à l'origine (cas mono-étoile).
pub(crate) fn ajouter_planete(
    sys: &mut Systeme,
    a_au: f32,
    e: f32,
    incl: f32,
    rayon: f32,
    masse: f32,
    app: Apparence,
) -> usize {
    ajouter_planete_autour(sys, Foyer::Barycentre, MASSE_ETOILE, a_au, e, incl, rayon, masse, app)
}

/// Cœur du placement d'une lune : la pose sur le prochain créneau orbital dans le
/// **domaine gravitationnel** de la planète (approximation de sa sphère de Hill,
/// bornée par sa distance à l'étoile) — les lunes restent donc proches de leur
/// planète et ne peuvent pas empiéter sur l'orbite de la voisine. Réparties du bord
/// de Roche (~2,4 R) jusqu'à ~14 % de la distance à l'étoile, sur des créneaux
/// croissants (nombre de lunes déjà là). ω suit la 3e loi de Kepler. `rayon_lune` et
/// `app` sont fournis par l'appelant. Renvoie `false` (rien posé) si la planète est
/// trop proche de son étoile pour une lune stable.
fn poser_lune(sys: &mut Systeme, parent: usize, rayon_planete: f32, rayon_lune: f32, app: Apparence) -> bool {
    let roche = rayon_planete * 2.4; // bord interne : pas de lune sous la limite de Roche
    let dist_etoile = sys.position(parent).length();
    let hill_max = 0.14 * dist_etoile;
    if hill_max <= roche * 1.1 {
        return false; // trop près de l'étoile : pas d'espace pour une lune stable
    }
    // Créneau croissant réparti dans la bande [roche, hill_max] (indice = lunes déjà là).
    let i = sys.nb_lunes(parent) as f32;
    let frac = ((i + 0.6) / 5.0).min(0.92);
    let bande = hill_max - roche;
    let r_orbite = roche + bande * frac + gen_range(-0.04, 0.04) * bande;

    // Sens majoritairement prograde (lunes régulières) ; rétrograde occasionnel (Triton).
    let sens = if gen_range(0.0_f32, 1.0) < 0.85 { 1.0 } else { -1.0 };
    // Kepler : ω décroît en (roche / r)^1.5 -> lunes internes rapides, externes lentes.
    const OMEGA_REF: f32 = 1.2;
    let omega = sens * OMEGA_REF * (roche / r_orbite).powf(1.5);
    // Lunes régulières quasi coplanaires (plan équatorial du parent) : faible inclinaison.
    let incl: f32 = gen_range(-0.12, 0.12);
    let phase: f32 = gen_range(0.0, TAU);
    let lune = Planete::new(Vec3::ZERO, Vec3::ZERO, rayon_lune.max(0.05), 0.05, app, Vec::new())
        .en_lune(parent, r_orbite, omega, incl, phase);
    sys.ajouter(Box::new(lune));
    true
}

/// Ajoute une lune **générique** (petit corps gris ou glacé aléatoire) : utilisée par
/// la génération procédurale. Taille réduite (une lune est bien plus petite que sa
/// planète), plancher de visibilité.
pub(crate) fn ajouter_lune(sys: &mut Systeme, parent: usize, rayon_planete: f32) {
    let rayon = rayon_planete * gen_range(0.05, 0.12);
    let app = if gen_range(0.0_f32, 1.0) < 0.5 {
        let g: f32 = gen_range(0.4, 0.7);
        app_simple(TypePlanete::Tellurique, vec3(g, g * 0.95, g * 0.9), vec3(g * 0.6, g * 0.58, g * 0.55), Vec3::ZERO, 0.0)
    } else {
        app_simple(TypePlanete::Glacee, vec3(0.72, 0.77, 0.85), vec3(0.55, 0.6, 0.7), Vec3::ZERO, 0.0)
    };
    poser_lune(sys, parent, rayon_planete, rayon, app);
}

/// Ajoute une lune d'**apparence donnée** (preset nommé), de taille `taille_rel × R
/// planète`. Sert aux presets scénarisés (lunes distinctes : Io, Europe, Titan…).
pub(crate) fn ajouter_lune_preset(sys: &mut Systeme, parent: usize, rayon_planete: f32, taille_rel: f32, app: Apparence) {
    poser_lune(sys, parent, rayon_planete, rayon_planete * taille_rel, app);
}

/// Déploie un arbre stellaire (systèmes multiples) dans le système : crée les
/// étoiles-feuilles et installe l'arbre évaluable. Voir `crate::stellaire`.
pub(crate) fn deployer_arbre(sys: &mut Systeme, racine: Noeud) {
    let mut arbre = ArbreStellaire::new();
    deployer_noeud(sys, &mut arbre, &racine, None, None);
    sys.definir_arbre(arbre);
}

/// Récursion : ajoute le nœud (feuille -> Soleil, paire -> deux orbites autour du
/// barycentre) et renvoie son index dans l'arbre déployé.
fn deployer_noeud(
    sys: &mut Systeme,
    arbre: &mut ArbreStellaire,
    noeud: &Noeud,
    parent: Option<usize>,
    orbite: Option<crate::orbite::Orbite>,
) -> usize {
    match noeud {
        Noeud::Etoile(f) => {
            let mut s = Soleil::new(Vec3::ZERO, f.rayon, f.couleur, f.luminosite);
            s = match f.variante {
                Variante::Normale => s,
                Variante::Jets => s.avec_jets(),
                Variante::Vent => s.avec_vent(),
                Variante::Pulsar => s.avec_pulsar(),
                Variante::Magnetar => s.avec_magnetar(),
            };
            if f.remnant {
                s = s.sans_zone_habitable(); // pulsar/magnétar/étoile à neutrons : pas de HZ
            }
            s.base.masse = f.masse; // masse personnalisée -> barycentres corrects
            let idx_astre = sys.ajouter(Box::new(s));
            arbre.ajouter(parent, orbite, Some(idx_astre))
        }
        Noeud::Paire { a, b, sep, e, incl, phase } => {
            // Nœud de paire d'abord (ordre topologique : parent avant enfants).
            let idx = arbre.ajouter(parent, orbite, None);

            let ma = a.masse();
            let mb = b.masse();
            let m = (ma + mb).max(1e-6);
            let sep_monde = *sep * etoile::UA;

            let phi: f32 = gen_range(0.0, TAU);
            let a1 = vec3(phi.cos(), 0.0, phi.sin());
            let a2 = vec3(-phi.sin(), 0.0, phi.cos());
            let q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();

            // Moyen mouvement partagé (même période pour les deux côtés).
            let n = (systeme::G * m / (sep_monde * sep_monde * sep_monde)).sqrt();
            // Chaque côté orbite le barycentre : demi-grand axe ∝ masse de l'autre.
            // A du côté opposé (base -a1,-q).
            let orb_a = crate::orbite::Orbite::avec_n(sep_monde * mb / m, *e, -a1, -q, n, *phase);
            let orb_b = crate::orbite::Orbite::avec_n(sep_monde * ma / m, *e, a1, q, n, *phase);

            deployer_noeud(sys, arbre, a, Some(idx), Some(orb_a));
            deployer_noeud(sys, arbre, b, Some(idx), Some(orb_b));
            idx
        }
    }
}

pub(crate) fn app_simple(type_p: TypePlanete, c1: Vec3, c2: Vec3, c3: Vec3, eau: f32) -> Apparence {
    Apparence::new(type_p, c1, c2, c3, eau)
}

/// Catalogue des telluriques actuellement générables, pour la galerie de validation
/// visuelle. Aujourd'hui : un échantillon par bande de température. Cette liste
/// grandira au fur et à mesure qu'on ajoute des variantes/presets (étape 1+).
/// Presets considérés « rares » (affichés avec un [R] violet en galerie).
const RARES: &[&str] = &[
    "Megaflora", "Petrified", "Recif", "Archipelago", "Geothermal", "Bioluminescent",
    "Salines", "Aquifer", "Coral (aride)", "Primal", "Baobab", "Geoglyph", "Storm",
    "Iceberg", "Cryflora", "Lichen", "Glaciovolcanic", "Lanthanide", "Eyeball humide",
    "Eyeball sec", "Eyeball gele", "Wet Superhabitable", "Dry Superhabitable",
    "Cold Superhabitable", "Pandora", "Polyphemus (Avatar)", "Tempete planetaire (GTB)",
];

/// Un preset est-il rare ?
pub fn est_rare(nom: &str) -> bool {
    RARES.contains(&nom)
}

/// Classe de taille d'un preset tellurique nommé. Les mondes Gaia / super-habitables
/// sont des super-Terres (plus gros) ; les corps rocheux sans air / lunes sont des
/// naines (plus petits). Tout le reste est une tellurique standard.
fn classe_tellurique(nom: &str) -> ClasseTaille {
    match nom {
        "Dry Gaia" | "Cold Gaia" | "Wet Superhabitable" | "Dry Superhabitable"
        | "Cold Superhabitable" => ClasseTaille::SuperTerre,
        "Lune" | "Titan" | "Fer (Mercure)" | "Carbone" | "Diamant" => ClasseTaille::Naine,
        _ => ClasseTaille::Tellurique,
    }
}

/// Classe de taille d'un preset gazeux nommé. Les géantes de glace (Neptune/Uranus
/// et apparentées) sont plus petites que les géantes gazeuses ; le sous-Neptune est
/// le plus petit. Naines brunes et géantes classiques → géante gazeuse.
fn classe_gazeuse(nom: &str) -> ClasseTaille {
    match nom {
        "Sub-Neptune" => ClasseTaille::SousNeptune,
        "Uranus" | "Neptune" | "Classe III (sans nuage, azur)" | "Neptune chaud"
        | "Anneau monobande (type Uranus)" | "Anneaux en arcs (type Neptune)" => {
            ClasseTaille::GeanteGlace
        }
        _ => ClasseTaille::GeanteGaz,
    }
}

pub fn catalogue_telluriques() -> Vec<(String, Apparence)> {
    let bleu = vec3(0.35, 0.55, 1.0) * 0.9; // atmosphère océanique
    let voile = vec3(0.6, 0.8, 1.0) * 0.2; // voile glacé
    let sec = vec3(0.8, 0.7, 0.5) * 0.12; // atmosphère sèche légère
    let z = Vec3::ZERO;

    let mut v: Vec<(String, Apparence)> = Vec::new();
    // Chaque preset reçoit une graine aléatoire -> géographie unique, et la touche G
    // (qui change la graine de la galerie) régénère des cartes différentes.
    let mut push = |nom: &str, mut app: Apparence| {
        app.seed = gen_range(0.0, 1000.0);
        app.taille = classe_tellurique(nom).rayon_aleatoire();
        v.push((nom.to_string(), app));
    };

    // --- Humide ---
    let blanc = vec3(1.0, 1.0, 1.0);
    push("Forest", tellurique(vec3(0.45, 0.4, 0.3), vec3(0.35, 0.3, 0.24), vec3(0.1, 0.32, 0.7), 0.55, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.16, 0.55, 0.16), 0.98).avec_rivieres(0.4).avec_nuages(0.4, blanc));
    push("Monde-ocean", tellurique(vec3(0.2, 0.45, 0.3), vec3(0.25, 0.3, 0.25), vec3(0.06, 0.3, 0.62), 0.92, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_nuages(0.5, blanc));
    // Océan PUR : aucune terre émergée (eau = 1.0 -> quantile au max), banquise polaire.
    push("Ocean pur", tellurique(vec3(0.2, 0.35, 0.4), vec3(0.18, 0.28, 0.36), vec3(0.04, 0.26, 0.58), 1.0, 0.0, 0.4, 0.85, bleu).avec_nuages(0.45, blanc));
    // Hycéan : océan global sous atmosphère épaisse de vapeur (candidat exoplanètes K2-18b).
    push("Hycean", tellurique(vec3(0.2, 0.4, 0.45), vec3(0.16, 0.32, 0.4), vec3(0.05, 0.35, 0.55), 1.0, 0.0, 0.25, 1.0, vec3(0.5, 0.8, 0.9) * 0.45).avec_nuages(0.75, vec3(0.95, 0.97, 1.0)).avec_voile(0.22, vec3(0.7, 0.85, 0.92)));
    // Océan de MAGMA : mer de roche fondue globale (planète nouveau-née / ultra-chaude).
    // `lave` -> réseau de fissures émissives : la mer brûle aussi côté nuit.
    let mut magma = tellurique(vec3(0.3, 0.16, 0.1), vec3(0.22, 0.12, 0.08), vec3(1.0, 0.38, 0.06), 1.0, 0.0, 0.1, 1.0, vec3(1.0, 0.5, 0.2) * 0.35).avec_nuages(0.25, vec3(0.45, 0.3, 0.28));
    magma.lave = 0.5;
    push("Ocean de magma", magma);
    push("Lacs", tellurique(vec3(0.42, 0.4, 0.28), vec3(0.36, 0.32, 0.24), vec3(0.15, 0.4, 0.7), 0.42, 3.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.52, 0.2), 0.7));
    push("Marais", tellurique(vec3(0.34, 0.36, 0.2), vec3(0.28, 0.26, 0.16), vec3(0.3, 0.42, 0.35), 0.4, 3.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.3, 0.42, 0.2), 0.7));
    push("Algues roses", tellurique(vec3(0.3, 0.45, 0.25), vec3(0.3, 0.3, 0.22), vec3(0.85, 0.35, 0.6), 0.8, 0.0, 0.4, 0.85, bleu));
    push("Recif", tellurique(vec3(0.3, 0.5, 0.3), vec3(0.3, 0.32, 0.24), vec3(0.1, 0.45, 0.7), 0.85, 0.0, 0.4, 0.85, bleu).avec_recifs(0.9).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.4));
    push("Columnar", tellurique(vec3(0.36, 0.37, 0.4), vec3(0.22, 0.23, 0.26), vec3(0.1, 0.32, 0.6), 0.55, 0.0, 0.4, 0.85, bleu).avec_basalt(0.9));

    // --- Végétation colorée (teinte de végétation) ---
    push("Sakura", tellurique(vec3(0.5, 0.42, 0.34), vec3(0.38, 0.3, 0.26), vec3(0.12, 0.34, 0.66), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.95, 0.55, 0.75), 0.85));
    push("Retinal", tellurique(vec3(0.4, 0.36, 0.34), vec3(0.3, 0.26, 0.26), vec3(0.1, 0.3, 0.62), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.55, 0.25, 0.7), 0.85));
    push("Carotene", tellurique(vec3(0.5, 0.42, 0.3), vec3(0.4, 0.32, 0.22), vec3(0.12, 0.34, 0.6), 0.45, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.88, 0.55, 0.12), 0.85));
    push("Mousse", tellurique(vec3(0.3, 0.34, 0.24), vec3(0.26, 0.28, 0.2), vec3(0.15, 0.36, 0.5), 0.35, 1.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.26, 0.62, 0.22), 1.0));
    push("Bioluminescent", tellurique(vec3(0.12, 0.2, 0.18), vec3(0.08, 0.14, 0.13), vec3(0.04, 0.18, 0.28), 0.6, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.1, 0.3, 0.22), 0.6).avec_biolum(0.95).avec_nuages(0.3, vec3(0.5, 0.6, 0.7)));

    // --- Météo (couche nuageuse) ---
    push("Brumeux", tellurique(vec3(0.4, 0.45, 0.4), vec3(0.32, 0.35, 0.32), vec3(0.2, 0.4, 0.55), 0.6, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.25, 0.45, 0.25), 0.6).avec_nuages(0.85, blanc));
    push("Orageux", tellurique(vec3(0.3, 0.42, 0.26), vec3(0.26, 0.32, 0.22), vec3(0.08, 0.3, 0.55), 0.75, 0.0, 0.35, 0.88, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.7).avec_meteo(0.85, vec3(0.5, 0.52, 0.58), 1.0));
    push("Cyclone", tellurique(vec3(0.25, 0.42, 0.3), vec3(0.26, 0.3, 0.24), vec3(0.06, 0.3, 0.6), 0.78, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_meteo(0.8, blanc, 2.0));
    push("Tempete de poussiere", tellurique(vec3(0.78, 0.58, 0.34), vec3(0.55, 0.4, 0.24), z, 0.0, 1.0, 0.2, 0.95, sec).avec_meteo(0.6, vec3(0.78, 0.62, 0.4), 1.0));

    // --- Sec ---
    push("Desert", tellurique(vec3(0.8, 0.6, 0.35), vec3(0.55, 0.4, 0.25), vec3(0.4, 0.5, 0.6), 0.04, 2.0, 0.25, 0.95, sec).avec_dunes(0.6));
    push("Dune (erg)", tellurique(vec3(0.85, 0.66, 0.38), vec3(0.62, 0.45, 0.26), z, 0.0, 1.0, 0.15, 0.97, sec).avec_relief(0.15).avec_dunes(0.9));
    push("Sable ferreux", tellurique(vec3(0.7, 0.3, 0.2), vec3(0.5, 0.22, 0.16), z, 0.0, 1.0, 0.2, 0.95, sec).avec_dunes(0.55));
    push("Mediterraneenne", tellurique(vec3(0.6, 0.55, 0.3), vec3(0.45, 0.4, 0.24), vec3(0.15, 0.4, 0.6), 0.42, 2.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.35, 0.5, 0.2), 0.45).avec_rivieres(0.7));
    push("Salines", tellurique(vec3(0.85, 0.82, 0.78), vec3(0.6, 0.55, 0.5), z, 0.0, 1.0, 0.2, 0.95, sec));
    push("Mesa", tellurique(vec3(0.72, 0.42, 0.28), vec3(0.5, 0.3, 0.22), vec3(0.2, 0.45, 0.55), 0.12, 2.0, 0.3, 0.9, sec).avec_relief(0.5).avec_mesa(0.8).avec_rivieres(0.5).avec_vegetation(vec3(0.3, 0.45, 0.2), 0.3));
    push("Striped", tellurique(vec3(0.72, 0.5, 0.3), vec3(0.32, 0.46, 0.7), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.95));

    // --- Froid ---
    push("Toundra", tellurique(vec3(0.4, 0.42, 0.32), vec3(0.3, 0.3, 0.26), vec3(0.6, 0.7, 0.8), 0.15, 1.0, 0.6, 0.6, voile));
    push("Arctique", tellurique(vec3(0.55, 0.6, 0.62), vec3(0.4, 0.45, 0.5), vec3(0.5, 0.6, 0.75), 0.2, 1.0, 0.7, 0.4, voile));
    push("Boule de neige", tellurique(vec3(0.85, 0.9, 0.96), vec3(0.7, 0.78, 0.88), z, 0.0, 1.0, 0.7, 0.15, voile).avec_pics(0.5));
    push("Alpin", tellurique(vec3(0.42, 0.4, 0.34), vec3(0.34, 0.32, 0.3), vec3(0.2, 0.4, 0.6), 0.25, 1.0, 0.55, 0.7, voile).avec_vegetation(vec3(0.18, 0.42, 0.2), 0.6).avec_relief(0.9).avec_nuages(0.25, blanc));
    push("Pics de glace", tellurique(vec3(0.78, 0.85, 0.95), vec3(0.62, 0.72, 0.85), z, 0.0, 1.0, 0.7, 0.3, voile).avec_relief(0.4).avec_pics(0.9));
    // Combo : montagnes + dunes en plaine + forêts en altitude (features cumulées).
    push("Dune Forest", tellurique(vec3(0.62, 0.52, 0.32), vec3(0.42, 0.36, 0.24), vec3(0.15, 0.4, 0.6), 0.18, 1.0, 0.4, 0.7, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.75).avec_relief(0.75).avec_dunes(0.7).avec_nuages(0.25, blanc));
    let mut cryovolc = tellurique(vec3(0.4, 0.44, 0.4), vec3(0.3, 0.32, 0.3), vec3(0.4, 0.55, 0.72), 0.12, 1.0, 0.5, 0.45, voile).avec_vegetation(vec3(0.32, 0.42, 0.3), 0.4).avec_relief(0.55).avec_mesa(0.4).avec_rivieres(0.4).avec_riv_lave();
    cryovolc.lave = 0.25; // chaleur volcanique sous la glace + toundra
    push("Cryovolcan", cryovolc);

    // --- Extrêmes / exotiques ---
    let mut lave = tellurique(vec3(0.16, 0.10, 0.08), vec3(0.07, 0.05, 0.04), z, 0.0, 1.0, 0.0, 1.0, z);
    lave.lave = 1.0;
    lave.relief = 0.0;
    push("Lave", lave);
    push("Venus (etuve)", tellurique(vec3(0.7, 0.6, 0.4), vec3(0.5, 0.42, 0.28), z, 0.0, 1.0, 0.15, 1.0, vec3(0.8, 0.7, 0.3) * 0.25).avec_voile(0.93, vec3(0.93, 0.82, 0.5)));
    push("Titan", tellurique(vec3(0.6, 0.45, 0.25), vec3(0.45, 0.32, 0.2), vec3(0.5, 0.35, 0.15), 0.25, 3.0, 0.2, 0.9, z).avec_voile(0.78, vec3(0.85, 0.55, 0.25)));
    push("Fer (Mercure)", tellurique(vec3(0.5, 0.48, 0.45), vec3(0.32, 0.3, 0.28), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.4).avec_crateres(0.85));
    push("Carbone", tellurique(vec3(0.12, 0.12, 0.14), vec3(0.06, 0.06, 0.07), z, 0.0, 1.0, 0.0, 1.0, z).avec_crateres(0.4));
    push("Lune", tellurique(vec3(0.55, 0.54, 0.52), vec3(0.34, 0.33, 0.32), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.4).avec_crateres(0.95));
    push("Diamant", tellurique(vec3(0.22, 0.26, 0.34), vec3(0.1, 0.12, 0.17), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.5).avec_basalt(0.6));
    push("Subglaciaire", tellurique(vec3(0.82, 0.88, 0.95), vec3(0.7, 0.64, 0.62), z, 0.0, 1.0, 0.2, 0.05, voile).avec_relief(0.2));
    let mut chth = tellurique(vec3(0.32, 0.13, 0.1), vec3(0.18, 0.08, 0.07), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.6);
    chth.lave = 0.5;
    push("Chthonien", chth);
    let mut ash = tellurique(vec3(0.3, 0.28, 0.27), vec3(0.16, 0.15, 0.14), z, 0.0, 1.0, 0.1, 1.0, z).avec_dunes(0.4);
    ash.lave = 0.3;
    push("Ash (volcanique)", ash);

    // --- Première passe de variantes nommées (presets de paramètres) ---
    // Monde type Terre : océans + continents verts + déserts (zones sèches) + calottes
    // polaires + montagnes + rivières + nuages -> toute la diversité de notre planète.
    push("Terre", tellurique(vec3(0.5, 0.42, 0.3), vec3(0.4, 0.34, 0.26), vec3(0.07, 0.32, 0.7), 0.62, 1.0, 0.5, 0.8, bleu).avec_vegetation(vec3(0.16, 0.5, 0.18), 0.82).avec_relief(0.65).avec_rivieres(0.55).avec_dunes(0.3).avec_nuages(0.42, blanc));

    // Mélange de styles (un seul preset cumule plusieurs axes) : lune type Pandora.
    push("Pandora", tellurique(vec3(0.2, 0.4, 0.3), vec3(0.22, 0.3, 0.26), vec3(0.06, 0.34, 0.55), 0.7, 0.0, 0.42, 0.9, bleu).avec_vegetation(vec3(0.14, 0.52, 0.32), 0.95).avec_relief(0.85).avec_rivieres(0.4).avec_biolum(0.9).avec_nuages(0.4, blanc));

    // Humide
    push("Mushroom", tellurique(vec3(0.4, 0.36, 0.3), vec3(0.3, 0.27, 0.24), vec3(0.12, 0.34, 0.6), 0.45, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.45, 0.3, 0.45), 0.85).avec_nuages(0.3, blanc));
    push("Tepid", tellurique(vec3(0.45, 0.42, 0.28), vec3(0.34, 0.3, 0.22), vec3(0.1, 0.36, 0.66), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.5, 0.62, 0.15), 0.9).avec_rivieres(0.4));
    push("Swamp", tellurique(vec3(0.34, 0.36, 0.22), vec3(0.26, 0.26, 0.18), vec3(0.2, 0.36, 0.32), 0.55, 3.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.28, 0.4, 0.2), 0.75).avec_nuages(0.4, blanc));
    push("Kelp", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.1, 0.4, 0.4), 0.85, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.4));
    push("Tidepool", tellurique(vec3(0.42, 0.42, 0.36), vec3(0.32, 0.3, 0.26), vec3(0.12, 0.42, 0.55), 0.6, 1.0, 0.4, 0.85, bleu).avec_relief(0.5).avec_vegetation(vec3(0.25, 0.45, 0.25), 0.4));
    // Tropical
    push("Atoll", tellurique(vec3(0.3, 0.5, 0.3), vec3(0.3, 0.32, 0.24), vec3(0.1, 0.5, 0.65), 0.82, 0.0, 0.35, 0.95, bleu).avec_recifs(0.85).avec_vegetation(vec3(0.2, 0.55, 0.2), 0.5).avec_nuages(0.4, blanc));
    push("Mangrove", tellurique(vec3(0.32, 0.4, 0.24), vec3(0.26, 0.3, 0.2), vec3(0.15, 0.42, 0.45), 0.7, 3.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.2, 0.45, 0.2), 0.7).avec_nuages(0.45, blanc));
    // Savane / aride
    push("Steppe", tellurique(vec3(0.7, 0.62, 0.32), vec3(0.5, 0.44, 0.26), z, 0.06, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.55, 0.55, 0.2), 0.5));
    push("Veldt", tellurique(vec3(0.65, 0.55, 0.3), vec3(0.46, 0.4, 0.24), z, 0.05, 1.0, 0.3, 0.92, sec).avec_relief(0.5).avec_vegetation(vec3(0.5, 0.52, 0.22), 0.45));
    push("Acacia", tellurique(vec3(0.6, 0.55, 0.3), vec3(0.44, 0.4, 0.24), vec3(0.2, 0.4, 0.5), 0.12, 2.0, 0.32, 0.9, sec).avec_vegetation(vec3(0.4, 0.5, 0.2), 0.5).avec_rivieres(0.4));
    push("Badlands", tellurique(vec3(0.7, 0.4, 0.28), vec3(0.5, 0.28, 0.2), z, 0.0, 1.0, 0.25, 0.95, sec).avec_mesa(0.7).avec_relief(0.4));
    push("Fog Desert", tellurique(vec3(0.72, 0.62, 0.42), vec3(0.52, 0.42, 0.3), z, 0.02, 2.0, 0.25, 0.95, sec).avec_dunes(0.4).avec_nuages(0.7, vec3(0.9, 0.9, 0.92)));
    push("Amethyst", tellurique(vec3(0.5, 0.35, 0.6), vec3(0.32, 0.22, 0.4), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.5).avec_relief(0.4));
    push("Cactus", tellurique(vec3(0.75, 0.6, 0.35), vec3(0.55, 0.42, 0.26), z, 0.03, 2.0, 0.25, 0.95, sec).avec_dunes(0.3).avec_vegetation(vec3(0.3, 0.5, 0.25), 0.25));
    // Froid
    push("Boreal", tellurique(vec3(0.35, 0.4, 0.3), vec3(0.3, 0.32, 0.28), vec3(0.2, 0.4, 0.55), 0.25, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.16, 0.4, 0.22), 0.6).avec_relief(0.5));
    push("Taiga", tellurique(vec3(0.32, 0.38, 0.3), vec3(0.28, 0.3, 0.26), vec3(0.18, 0.38, 0.52), 0.2, 1.0, 0.55, 0.5, voile).avec_vegetation(vec3(0.15, 0.36, 0.2), 0.55).avec_relief(0.7).avec_nuages(0.3, blanc));
    push("Fjord", tellurique(vec3(0.4, 0.44, 0.38), vec3(0.32, 0.34, 0.3), vec3(0.15, 0.4, 0.55), 0.35, 1.0, 0.55, 0.5, voile).avec_relief(0.85).avec_vegetation(vec3(0.18, 0.42, 0.24), 0.45));
    push("Glacial", tellurique(vec3(0.6, 0.66, 0.72), vec3(0.45, 0.5, 0.56), vec3(0.4, 0.55, 0.7), 0.2, 1.0, 0.7, 0.4, voile).avec_pics(0.3));
    push("Cold Desert", tellurique(vec3(0.62, 0.6, 0.58), vec3(0.45, 0.43, 0.42), z, 0.0, 1.0, 0.6, 0.45, voile).avec_dunes(0.3));
    push("Ice Dunes", tellurique(vec3(0.8, 0.85, 0.92), vec3(0.62, 0.68, 0.78), z, 0.0, 1.0, 0.7, 0.25, voile).avec_dunes(0.7));
    push("Bog", tellurique(vec3(0.36, 0.38, 0.26), vec3(0.28, 0.28, 0.22), vec3(0.22, 0.34, 0.3), 0.4, 3.0, 0.6, 0.6, voile).avec_vegetation(vec3(0.3, 0.4, 0.22), 0.5));

    // --- Deuxième passe de variantes nommées ---
    // Continental
    push("Megaflora", tellurique(vec3(0.3, 0.34, 0.24), vec3(0.24, 0.26, 0.2), vec3(0.1, 0.32, 0.6), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.1, 0.4, 0.14), 1.0).avec_relief(0.55).avec_nuages(0.4, blanc));
    push("Petrified", tellurique(vec3(0.6, 0.55, 0.45), vec3(0.42, 0.38, 0.32), z, 0.0, 1.0, 0.3, 0.9, sec).avec_mesa(0.4).avec_relief(0.4));
    // Océan
    push("Cascadian", tellurique(vec3(0.35, 0.42, 0.3), vec3(0.3, 0.32, 0.26), vec3(0.1, 0.34, 0.55), 0.6, 1.0, 0.45, 0.7, bleu).avec_vegetation(vec3(0.16, 0.42, 0.2), 0.7).avec_relief(0.8).avec_nuages(0.6, vec3(0.8, 0.82, 0.85)));
    push("Archipelago", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.06, 0.32, 0.6), 0.88, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_relief(0.4));
    push("Crag", tellurique(vec3(0.45, 0.45, 0.42), vec3(0.34, 0.34, 0.32), vec3(0.1, 0.36, 0.55), 0.7, 1.0, 0.55, 0.5, voile).avec_relief(0.7));
    // Tropical
    push("Geothermal", tellurique(vec3(0.5, 0.55, 0.55), vec3(0.4, 0.44, 0.46), vec3(0.3, 0.5, 0.6), 0.15, 1.0, 0.55, 0.45, voile).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.35).avec_relief(0.5).avec_rivieres(0.35).avec_riv_lave());
    push("Tepui", tellurique(vec3(0.4, 0.42, 0.28), vec3(0.32, 0.3, 0.22), vec3(0.1, 0.4, 0.6), 0.4, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.16, 0.45, 0.2), 0.7).avec_mesa(0.6).avec_relief(0.5).avec_nuages(0.5, blanc));
    push("Lilypad", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.2, 0.5, 0.3), 0.85, 0.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.25, 0.55, 0.25), 0.45));
    push("Obsidian", tellurique(vec3(0.25, 0.4, 0.3), vec3(0.08, 0.08, 0.1), vec3(0.1, 0.4, 0.55), 0.55, 1.0, 0.35, 0.88, bleu).avec_vegetation(vec3(0.2, 0.5, 0.22), 0.5).avec_basalt(0.5));
    // Désert / aride
    push("Oasis", tellurique(vec3(0.82, 0.64, 0.36), vec3(0.58, 0.42, 0.26), vec3(0.1, 0.45, 0.6), 0.06, 2.0, 0.25, 0.95, sec).avec_dunes(0.5).avec_vegetation(vec3(0.25, 0.5, 0.22), 0.3).avec_rivieres(0.3));
    push("Outback", tellurique(vec3(0.7, 0.45, 0.3), vec3(0.5, 0.32, 0.22), z, 0.03, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.45, 0.45, 0.2), 0.3).avec_dunes(0.3));
    push("Coastal", tellurique(vec3(0.7, 0.6, 0.4), vec3(0.5, 0.42, 0.3), vec3(0.12, 0.4, 0.6), 0.45, 1.0, 0.3, 0.9, bleu).avec_vegetation(vec3(0.35, 0.5, 0.25), 0.3).avec_nuages(0.6, vec3(0.9, 0.9, 0.92)));
    push("Sodalite", tellurique(vec3(0.6, 0.55, 0.4), vec3(0.25, 0.35, 0.62), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.9));
    push("Coral (aride)", tellurique(vec3(0.75, 0.5, 0.4), vec3(0.55, 0.35, 0.3), vec3(0.2, 0.55, 0.6), 0.15, 2.0, 0.3, 0.92, sec).avec_recifs(0.7).avec_vegetation(vec3(0.4, 0.5, 0.25), 0.2));
    push("Opal", tellurique(vec3(0.7, 0.8, 0.85), vec3(0.55, 0.6, 0.7), z, 0.0, 1.0, 0.2, 0.92, sec).avec_mesa(0.6));
    push("Superbloom", tellurique(vec3(0.75, 0.55, 0.35), vec3(0.55, 0.4, 0.26), z, 0.02, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.85, 0.4, 0.55), 0.4));
    let mut primal = tellurique(vec3(0.35, 0.22, 0.18), vec3(0.2, 0.12, 0.1), z, 0.0, 1.0, 0.2, 0.95, z).avec_relief(0.5);
    primal.lave = 0.4;
    push("Primal", primal);
    // Savane
    push("Pampa", tellurique(vec3(0.65, 0.62, 0.35), vec3(0.48, 0.44, 0.28), vec3(0.15, 0.4, 0.55), 0.1, 2.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.5, 0.6, 0.2), 0.6).avec_rivieres(0.3));
    push("Heath", tellurique(vec3(0.5, 0.42, 0.4), vec3(0.36, 0.3, 0.3), z, 0.05, 1.0, 0.4, 0.85, voile).avec_vegetation(vec3(0.4, 0.3, 0.4), 0.5));
    push("Bushveld", tellurique(vec3(0.6, 0.55, 0.32), vec3(0.44, 0.4, 0.24), vec3(0.15, 0.4, 0.5), 0.1, 2.0, 0.35, 0.9, sec).avec_vegetation(vec3(0.3, 0.48, 0.2), 0.55).avec_relief(0.4));
    // Arctique
    push("Antarctic", tellurique(vec3(0.8, 0.86, 0.92), vec3(0.62, 0.68, 0.78), z, 0.0, 1.0, 0.75, 0.25, voile).avec_pics(0.4).avec_relief(0.4));
    push("Iceberg", tellurique(vec3(0.75, 0.82, 0.9), vec3(0.55, 0.62, 0.72), vec3(0.3, 0.5, 0.7), 0.5, 0.0, 0.7, 0.4, voile).avec_pics(0.3));
    push("Storm", tellurique(vec3(0.45, 0.5, 0.55), vec3(0.35, 0.4, 0.45), vec3(0.1, 0.3, 0.5), 0.8, 0.0, 0.6, 0.45, voile).avec_meteo(0.85, vec3(0.55, 0.57, 0.63), 2.0).avec_cyclones(1.0));
    // Toundra
    let mut mud = tellurique(vec3(0.42, 0.36, 0.26), vec3(0.3, 0.25, 0.18), vec3(0.3, 0.28, 0.22), 0.4, 3.0, 0.55, 0.6, voile).avec_vegetation(vec3(0.35, 0.38, 0.22), 0.4);
    mud.lave = 0.2;
    push("Mud", mud);
    push("Travertine", tellurique(vec3(0.88, 0.85, 0.78), vec3(0.6, 0.58, 0.52), vec3(0.3, 0.5, 0.55), 0.1, 2.0, 0.5, 0.55, voile).avec_mesa(0.85));
    push("Lichen", tellurique(vec3(0.5, 0.52, 0.42), vec3(0.38, 0.4, 0.34), z, 0.0, 1.0, 0.55, 0.5, voile).avec_vegetation(vec3(0.45, 0.5, 0.3), 0.45).avec_relief(0.6));
    push("Cryflora", tellurique(vec3(0.6, 0.7, 0.78), vec3(0.45, 0.55, 0.65), z, 0.05, 1.0, 0.6, 0.4, voile).avec_vegetation(vec3(0.2, 0.7, 0.5), 0.4).avec_biolum(0.7));
    // Alpin
    push("Highland", tellurique(vec3(0.45, 0.46, 0.4), vec3(0.34, 0.36, 0.32), vec3(0.15, 0.4, 0.5), 0.2, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.3, 0.42, 0.25), 0.45).avec_relief(0.6).avec_nuages(0.7, vec3(0.85, 0.87, 0.9)));
    push("Snow", tellurique(vec3(0.7, 0.76, 0.84), vec3(0.55, 0.6, 0.7), vec3(0.2, 0.4, 0.6), 0.15, 1.0, 0.65, 0.35, voile).avec_vegetation(vec3(0.16, 0.4, 0.22), 0.5));
    push("Blossom", tellurique(vec3(0.5, 0.5, 0.42), vec3(0.38, 0.38, 0.32), vec3(0.15, 0.4, 0.55), 0.2, 1.0, 0.5, 0.5, voile).avec_vegetation(vec3(0.8, 0.5, 0.7), 0.6).avec_relief(0.5));
    let mut glaciovolc = tellurique(vec3(0.55, 0.62, 0.7), vec3(0.42, 0.48, 0.56), vec3(0.3, 0.5, 0.6), 0.12, 1.0, 0.55, 0.4, voile).avec_relief(0.6).avec_mesa(0.6).avec_rivieres(0.5).avec_riv_lave();
    glaciovolc.lave = 0.35; // lave qui transparaît sous la glace fracturée
    push("Glaciovolcanic", glaciovolc);
    // Gaia / superhabitables
    push("Dry Gaia", tellurique(vec3(0.7, 0.6, 0.35), vec3(0.5, 0.42, 0.26), vec3(0.12, 0.45, 0.65), 0.35, 1.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.4, 0.55, 0.2), 0.7).avec_rivieres(0.4).avec_nuages(0.3, blanc));
    push("Cold Gaia", tellurique(vec3(0.4, 0.45, 0.36), vec3(0.32, 0.36, 0.3), vec3(0.12, 0.4, 0.6), 0.4, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.7).avec_relief(0.5).avec_nuages(0.3, blanc));
    push("Wet Superhabitable", tellurique(vec3(0.3, 0.5, 0.28), vec3(0.3, 0.34, 0.24), vec3(0.08, 0.4, 0.7), 0.6, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.16, 0.55, 0.18), 0.85).avec_rivieres(0.5).avec_nuages(0.4, blanc));

    // --- Troisième passe (variantes restantes + flavor) ---
    push("Barnacle", tellurique(vec3(0.5, 0.48, 0.42), vec3(0.36, 0.34, 0.3), vec3(0.1, 0.36, 0.55), 0.8, 0.0, 0.4, 0.85, bleu).avec_relief(0.5).avec_basalt(0.3));
    push("Cenote", tellurique(vec3(0.4, 0.5, 0.3), vec3(0.32, 0.34, 0.24), vec3(0.12, 0.45, 0.6), 0.25, 3.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.18, 0.5, 0.2), 0.7));
    push("Fungal", tellurique(vec3(0.4, 0.36, 0.34), vec3(0.3, 0.26, 0.26), vec3(0.12, 0.4, 0.55), 0.4, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.5, 0.3, 0.45), 0.8).avec_nuages(0.3, blanc));
    push("Aerial", tellurique(vec3(0.3, 0.46, 0.3), vec3(0.28, 0.32, 0.24), vec3(0.1, 0.4, 0.62), 0.5, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.18, 0.52, 0.2), 0.8).avec_nuages(0.6, blanc));
    push("Aquifer", tellurique(vec3(0.82, 0.62, 0.36), vec3(0.58, 0.42, 0.26), vec3(0.1, 0.45, 0.6), 0.05, 2.0, 0.25, 0.95, sec).avec_dunes(0.6).avec_vegetation(vec3(0.3, 0.5, 0.22), 0.2));
    push("Fungi", tellurique(vec3(0.42, 0.4, 0.38), vec3(0.28, 0.27, 0.26), z, 0.0, 1.0, 0.25, 0.92, sec).avec_vegetation(vec3(0.45, 0.4, 0.3), 0.35).avec_dunes(0.3));
    push("Succulent", tellurique(vec3(0.7, 0.58, 0.36), vec3(0.5, 0.4, 0.26), z, 0.02, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.35, 0.52, 0.28), 0.35));
    push("Baobab", tellurique(vec3(0.66, 0.58, 0.32), vec3(0.48, 0.42, 0.26), vec3(0.15, 0.4, 0.5), 0.08, 2.0, 0.32, 0.9, sec).avec_vegetation(vec3(0.4, 0.5, 0.2), 0.4).avec_relief(0.3));
    push("Scrubland", tellurique(vec3(0.68, 0.56, 0.34), vec3(0.5, 0.4, 0.26), z, 0.04, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.45, 0.45, 0.24), 0.4));
    push("Geoglyph", tellurique(vec3(0.7, 0.6, 0.36), vec3(0.5, 0.42, 0.26), z, 0.05, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.5, 0.52, 0.22), 0.45).avec_mesa(0.3));
    push("Termite", tellurique(vec3(0.66, 0.5, 0.3), vec3(0.48, 0.36, 0.22), z, 0.04, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.45, 0.46, 0.22), 0.4).avec_dunes(0.25));
    push("Amber", tellurique(vec3(0.7, 0.55, 0.28), vec3(0.5, 0.38, 0.2), vec3(0.2, 0.4, 0.45), 0.1, 2.0, 0.32, 0.9, vec3(0.9, 0.6, 0.2) * 0.3).avec_vegetation(vec3(0.7, 0.5, 0.15), 0.55));
    push("Aeolian", tellurique(vec3(0.6, 0.62, 0.68), vec3(0.45, 0.47, 0.52), z, 0.0, 1.0, 0.6, 0.45, voile).avec_mesa(0.6).avec_relief(0.6).avec_nuages(0.5, vec3(0.82, 0.84, 0.88)).avec_cyclones(0.3));
    push("Supraglacial", tellurique(vec3(0.78, 0.84, 0.92), vec3(0.6, 0.66, 0.76), vec3(0.35, 0.55, 0.72), 0.35, 3.0, 0.7, 0.3, voile));
    let mut crevasse = tellurique(vec3(0.62, 0.72, 0.85), vec3(0.4, 0.5, 0.66), z, 0.0, 1.0, 0.6, 0.4, voile).avec_mesa(0.85).avec_relief(0.6).avec_rivieres(0.55).avec_riv_lave();
    crevasse.lave = 0.3; // chaleur géothermique qui transparaît au fond des crevasses
    push("Crevasse", crevasse);
    push("Ferrosprings", tellurique(vec3(0.72, 0.78, 0.86), vec3(0.55, 0.6, 0.7), vec3(0.7, 0.25, 0.15), 0.18, 3.0, 0.65, 0.4, voile).avec_rivieres(0.45));
    push("Tuya", tellurique(vec3(0.5, 0.55, 0.6), vec3(0.4, 0.44, 0.48), z, 0.0, 1.0, 0.6, 0.45, voile).avec_relief(0.6).avec_rivieres(0.35).avec_riv_lave());
    push("Peatland", tellurique(vec3(0.4, 0.38, 0.28), vec3(0.3, 0.28, 0.22), vec3(0.25, 0.32, 0.28), 0.35, 3.0, 0.55, 0.6, voile).avec_vegetation(vec3(0.34, 0.4, 0.24), 0.5));
    push("Treeline", tellurique(vec3(0.5, 0.55, 0.6), vec3(0.4, 0.44, 0.48), vec3(0.2, 0.4, 0.55), 0.2, 1.0, 0.7, 0.35, voile).avec_vegetation(vec3(0.16, 0.45, 0.22), 0.8));
    push("Lanthanide", tellurique(vec3(0.55, 0.5, 0.5), vec3(0.36, 0.33, 0.36), z, 0.0, 1.0, 0.3, 0.9, z).avec_relief(0.85).avec_crateres(0.3));
    push("Ravine", tellurique(vec3(0.5, 0.46, 0.4), vec3(0.38, 0.34, 0.3), vec3(0.15, 0.4, 0.55), 0.25, 1.0, 0.5, 0.55, voile).avec_mesa(0.6).avec_rivieres(0.5).avec_relief(0.6).avec_vegetation(vec3(0.2, 0.42, 0.24), 0.4));
    push("Craton", tellurique(vec3(0.65, 0.58, 0.42), vec3(0.48, 0.42, 0.3), z, 0.05, 2.0, 0.35, 0.9, sec).avec_mesa(0.4));
    push("Dry Superhabitable", tellurique(vec3(0.68, 0.6, 0.34), vec3(0.5, 0.42, 0.26), vec3(0.12, 0.45, 0.65), 0.4, 1.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.4, 0.55, 0.2), 0.75).avec_rivieres(0.5).avec_nuages(0.3, blanc));
    push("Cold Superhabitable", tellurique(vec3(0.4, 0.46, 0.36), vec3(0.32, 0.36, 0.3), vec3(0.12, 0.4, 0.6), 0.45, 1.0, 0.5, 0.5, voile).avec_vegetation(vec3(0.2, 0.52, 0.26), 0.75).avec_relief(0.5).avec_nuages(0.3, blanc));
    // Mondes-grottes (flavor : rendus comme leur famille parente)
    push("Wet Cave", tellurique(vec3(0.4, 0.42, 0.3), vec3(0.3, 0.3, 0.24), vec3(0.1, 0.34, 0.55), 0.4, 1.0, 0.4, 0.8, bleu).avec_vegetation(vec3(0.2, 0.45, 0.22), 0.5).avec_relief(0.5));
    push("Dry Cave", tellurique(vec3(0.7, 0.55, 0.34), vec3(0.5, 0.38, 0.24), z, 0.02, 2.0, 0.3, 0.92, sec).avec_dunes(0.3).avec_relief(0.5));
    push("Cold Cave", tellurique(vec3(0.6, 0.64, 0.7), vec3(0.46, 0.5, 0.56), z, 0.0, 1.0, 0.6, 0.4, voile).avec_relief(0.5).avec_pics(0.3));
    // Soufre / Io
    let mut io = tellurique(vec3(0.85, 0.7, 0.2), vec3(0.6, 0.4, 0.1), z, 0.0, 1.0, 0.1, 1.0, vec3(0.9, 0.8, 0.3) * 0.2).avec_dunes(0.2);
    io.lave = 0.5;
    push("Io (soufre)", io);

    // --- Verrouillées par marée (eyeball : jour brûlé / nuit gelée) ---
    // Gelé : océan global GELÉ, œil d'eau libre au subsolaire (glint spéculaire),
    // anneau de slush au bord, nuages d'évaporation au-dessus de l'œil.
    push("Eyeball gele", tellurique(vec3(0.55, 0.62, 0.68), vec3(0.42, 0.5, 0.58), vec3(0.06, 0.32, 0.58), 0.96, 0.0, 0.0, 1.0, bleu).avec_nuages(0.25, blanc).avec_eyeball_zones(0.55, 0.0, 0.0));
    // Sec : subsolaire en lave/obsidienne, anneau de vie au terminateur, désert, puis glace.
    push("Eyeball sec", tellurique(vec3(0.72, 0.56, 0.34), vec3(0.5, 0.38, 0.24), z, 0.0, 1.0, 0.0, 1.0, sec).avec_dunes(0.4).avec_vegetation(vec3(0.32, 0.45, 0.2), 0.0).avec_eyeball_zones(-0.05, 1.0, 1.0));
    // Humide : continents et océans côté jour, anneau végétal au terminateur,
    // capuchon de tempête permanent au subsolaire, glace en nuit profonde seulement.
    push("Eyeball humide", tellurique(vec3(0.62, 0.5, 0.3), vec3(0.45, 0.36, 0.24), vec3(0.08, 0.38, 0.62), 0.55, 1.0, 0.0, 1.0, bleu).avec_vegetation(vec3(0.2, 0.5, 0.22), 0.6).avec_rivieres(0.35).avec_nuages(0.55, blanc).avec_eyeball_zones(-0.12, 0.0, 1.0));

    v
}

/// Catalogue des géantes gazeuses pour la galerie dédiée. Basé sur la classification
/// de Sudarsky (classes I–V par température/nuages) + géantes de glace + naines brunes.
pub fn catalogue_gazeuses() -> Vec<(String, Apparence)> {
    let mut v: Vec<(String, Apparence)> = Vec::new();
    let mut push = |nom: &str, mut app: Apparence| {
        app.seed = gen_range(0.0, 1000.0);
        app.taille = classe_gazeuse(nom).rayon_aleatoire();
        v.push((nom.to_string(), app));
    };
    let spot = vec3(0.6, -0.22, 0.77);

    // Géantes du système solaire
    push("Jupiter", gazeuse(vec3(0.9, 0.66, 0.4), vec3(0.74, 0.44, 0.26), vec3(0.99, 0.95, 0.86), 5.0, 1.9, vec3(0.85, 0.7, 0.5) * 0.3).avec_pole(vec3(0.6, 0.64, 0.7)).avec_jets(1.0).avec_tache(spot, 0.27, vec3(0.85, 0.34, 0.18)).avec_cyclones_pol().avec_tempetes(0.7));
    push("Saturne", gazeuse(vec3(0.9, 0.78, 0.5), vec3(0.7, 0.56, 0.34), vec3(0.97, 0.91, 0.68), 6.0, 0.9, vec3(0.88, 0.8, 0.55) * 0.3).avec_jets(0.9).avec_hexagone().avec_aurore(0.5, vec3(0.5, 0.7, 1.0)).avec_brume(0.22, vec3(0.95, 0.89, 0.68)).avec_pole(vec3(0.66, 0.63, 0.55)).avec_anneau_saturne(vec3(0.86, 0.79, 0.62)));
    push("Uranus", gazeuse(vec3(0.6, 0.82, 0.82), vec3(0.45, 0.7, 0.72), vec3(0.72, 0.9, 0.9), 3.0, 0.6, vec3(0.6, 0.85, 0.88) * 0.3).avec_jets(0.15).avec_zonal_flou(0.5).avec_axe(vec3(0.25, 0.2, 0.95)).avec_brume(0.45, vec3(0.62, 0.84, 0.86)).avec_pole(vec3(0.62, 0.84, 0.86)));
    push("Neptune", gazeuse(vec3(0.45, 0.56, 0.86), vec3(0.10, 0.18, 0.42), vec3(0.36, 0.5, 0.88), 4.0, 1.5, vec3(0.3, 0.45, 0.9) * 0.3).avec_jets(0.55).avec_zonal_flou(0.3).avec_tache_sombre(spot, 0.2, vec3(0.05, 0.07, 0.18)).avec_tempetes(0.9).avec_pole(vec3(0.3, 0.46, 0.74)));

    // Classification de Sudarsky (par température)
    push("Classe I (ammoniac)", gazeuse(vec3(0.86, 0.8, 0.66), vec3(0.62, 0.56, 0.44), vec3(0.95, 0.92, 0.82), 5.0, 1.4, vec3(0.85, 0.8, 0.6) * 0.3).avec_jets(0.5).avec_zonal_flou(0.25).avec_pole(vec3(0.62, 0.6, 0.55)));
    push("Classe II (eau, albedo haut)", gazeuse(vec3(0.85, 0.88, 0.93), vec3(0.68, 0.74, 0.82), vec3(0.97, 0.98, 1.0), 4.0, 1.0, vec3(0.8, 0.85, 0.95) * 0.35).avec_jets(0.45).avec_zonal_flou(0.35).avec_brume(0.25, vec3(0.9, 0.92, 0.97)).avec_pole(vec3(0.82, 0.86, 0.92)));
    push("Classe III (sans nuage, azur)", gazeuse(vec3(0.16, 0.32, 0.62), vec3(0.1, 0.2, 0.45), vec3(0.28, 0.46, 0.8), 2.0, 0.4, vec3(0.25, 0.45, 0.85) * 0.4).avec_jets(0.25).avec_zonal_flou(0.45).avec_pole(vec3(0.2, 0.36, 0.62)));
    push("Classe IV (alcalins, sombre)", gazeuse(vec3(0.26, 0.13, 0.11), vec3(0.12, 0.06, 0.06), vec3(0.42, 0.22, 0.16), 5.0, 1.8, vec3(0.3, 0.12, 0.08) * 0.25).avec_jets(0.9).avec_thermique(0.45, vec3(0.5, 0.08, 0.02)).avec_pole(vec3(0.18, 0.1, 0.09)));
    push("Classe V (silicates, chaud)", gazeuse(vec3(0.55, 0.28, 0.16), vec3(0.3, 0.14, 0.1), vec3(0.85, 0.55, 0.3), 6.0, 2.2, vec3(0.9, 0.5, 0.25) * 0.4).avec_jets(1.0).avec_thermique(0.7, vec3(0.9, 0.32, 0.06)).avec_pole(vec3(0.36, 0.18, 0.12)));

    // Variantes
    push("Jupiter chaud", gazeuse(vec3(0.6, 0.3, 0.2), vec3(0.32, 0.14, 0.12), vec3(0.85, 0.45, 0.25), 6.0, 2.4, vec3(0.9, 0.45, 0.2) * 0.4).avec_jets(1.0).avec_tache(spot, 0.2, vec3(0.9, 0.35, 0.15)).avec_thermique(0.5, vec3(0.78, 0.22, 0.05)).avec_tempetes(0.6).avec_pole(vec3(0.4, 0.2, 0.16)));
    push("Geante de methane", gazeuse(vec3(0.2, 0.55, 0.45), vec3(0.1, 0.35, 0.3), vec3(0.4, 0.78, 0.62), 4.0, 1.4, vec3(0.3, 0.7, 0.55) * 0.3).avec_jets(0.65).avec_pole(vec3(0.34, 0.5, 0.44)));
    push("Geante de soufre", gazeuse(vec3(0.8, 0.7, 0.2), vec3(0.55, 0.45, 0.12), vec3(0.95, 0.88, 0.4), 5.0, 1.8, vec3(0.85, 0.75, 0.25) * 0.3).avec_jets(0.7).avec_pole(vec3(0.55, 0.5, 0.34)));
    push("Naine brune", gazeuse(vec3(0.4, 0.15, 0.1), vec3(0.2, 0.08, 0.06), vec3(0.6, 0.25, 0.15), 7.0, 2.6, vec3(0.5, 0.15, 0.08) * 0.35).avec_jets(1.1).avec_thermique(0.85, vec3(0.6, 0.12, 0.03)).avec_tempetes(0.6).avec_pole(vec3(0.26, 0.1, 0.07)));
    push("Sub-Neptune", gazeuse(vec3(0.4, 0.5, 0.6), vec3(0.3, 0.4, 0.5), vec3(0.55, 0.65, 0.75), 3.0, 1.0, vec3(0.5, 0.6, 0.7) * 0.3).avec_zonal_flou(0.55).avec_brume(0.7, vec3(0.62, 0.68, 0.78)).avec_axe(vec3(0.2, 0.85, 0.3)).avec_pole(vec3(0.5, 0.56, 0.64)));

    // Nouveaux types
    push("Geante d'helium", gazeuse(vec3(0.86, 0.85, 0.82), vec3(0.7, 0.69, 0.66), vec3(0.97, 0.97, 0.95), 4.0, 0.7, vec3(0.85, 0.85, 0.82) * 0.3).avec_zonal_flou(0.4).avec_brume(0.35, vec3(0.93, 0.93, 0.9)).avec_pole(vec3(0.78, 0.78, 0.76)));
    push("Naine brune L (poussiereuse)", gazeuse(vec3(0.55, 0.22, 0.12), vec3(0.32, 0.12, 0.07), vec3(0.72, 0.34, 0.18), 8.0, 2.6, vec3(0.6, 0.2, 0.08) * 0.35).avec_jets(1.1).avec_thermique(0.8, vec3(0.7, 0.18, 0.04)).avec_tempetes(0.7).avec_pole(vec3(0.32, 0.14, 0.08)));
    push("Naine brune T (methane)", gazeuse(vec3(0.35, 0.2, 0.32), vec3(0.18, 0.1, 0.2), vec3(0.5, 0.3, 0.48), 7.0, 2.3, vec3(0.4, 0.2, 0.4) * 0.3).avec_jets(1.0).avec_thermique(0.45, vec3(0.55, 0.12, 0.25)).avec_tempetes(0.5).avec_pole(vec3(0.22, 0.13, 0.24)));
    push("Naine brune Y (froide)", gazeuse(vec3(0.16, 0.12, 0.18), vec3(0.07, 0.05, 0.1), vec3(0.26, 0.18, 0.3), 5.0, 1.8, vec3(0.18, 0.12, 0.22) * 0.25).avec_jets(0.7).avec_thermique(0.2, vec3(0.4, 0.1, 0.18)).avec_pole(vec3(0.12, 0.09, 0.15)));
    push("Neptune chaud", gazeuse(vec3(0.35, 0.46, 0.6), vec3(0.22, 0.32, 0.45), vec3(0.55, 0.66, 0.78), 4.0, 1.3, vec3(0.4, 0.55, 0.72) * 0.3).avec_jets(0.5).avec_zonal_flou(0.35).avec_brume(0.4, vec3(0.55, 0.66, 0.76)).avec_tempetes(0.4).avec_pole(vec3(0.34, 0.45, 0.58)));
    push("Geante de carbone", gazeuse(vec3(0.18, 0.17, 0.16), vec3(0.08, 0.08, 0.08), vec3(0.3, 0.28, 0.25), 5.0, 1.6, vec3(0.12, 0.1, 0.1) * 0.2).avec_jets(0.55).avec_zonal_flou(0.3).avec_pole(vec3(0.14, 0.13, 0.12)));
    push("Proto-geante chaude", gazeuse(vec3(0.7, 0.32, 0.16), vec3(0.45, 0.16, 0.08), vec3(0.95, 0.55, 0.25), 6.0, 2.8, vec3(1.0, 0.5, 0.2) * 0.45).avec_jets(1.0).avec_thermique(0.95, vec3(1.0, 0.4, 0.08)).avec_tempetes(0.8).avec_pole(vec3(0.5, 0.22, 0.12)));
    push("Geante rayee extreme", gazeuse(vec3(0.92, 0.6, 0.3), vec3(0.3, 0.14, 0.1), vec3(1.0, 0.95, 0.82), 9.0, 1.4, vec3(0.8, 0.6, 0.4) * 0.3).avec_jets(1.2).avec_tempetes(0.5).avec_pole(vec3(0.5, 0.45, 0.4)));
    // Tempête planétaire (§ 6 bis) : instantané d'une Grande Tache Blanche de
    // Saturne — tête convective blanche massive (slot 0 type 2) + activité de
    // vortex maximale le long des jets, turbulence dopée.
    push("Tempete planetaire (GTB)", gazeuse(vec3(0.9, 0.78, 0.5), vec3(0.66, 0.52, 0.32), vec3(0.97, 0.91, 0.7), 6.0, 2.2, vec3(0.88, 0.8, 0.55) * 0.3).avec_jets(1.0).avec_tache_blanche(vec3(0.55, 0.3, 0.78), 0.3).avec_tempetes(1.0).avec_pole(vec3(0.66, 0.63, 0.55)));

    // Géante emblématique
    push("Polyphemus (Avatar)", gazeuse(vec3(0.32, 0.6, 0.58), vec3(0.12, 0.34, 0.42), vec3(0.6, 0.84, 0.78), 5.0, 1.7, vec3(0.4, 0.7, 0.68) * 0.3).avec_jets(0.95).avec_tache(spot, 0.24, vec3(0.88, 0.34, 0.16)).avec_cyclones_pol().avec_tempetes(0.6).avec_pole(vec3(0.4, 0.56, 0.56)).avec_anneau_saturne(vec3(0.66, 0.78, 0.74)));

    // Anneaux : exemples de styles variés
    push("Geante annelee massive", gazeuse(vec3(0.86, 0.76, 0.54), vec3(0.62, 0.5, 0.34), vec3(0.95, 0.9, 0.72), 5.0, 1.0, vec3(0.85, 0.78, 0.55) * 0.3).avec_jets(0.85).avec_brume(0.2, vec3(0.94, 0.88, 0.68)).avec_pole(vec3(0.64, 0.62, 0.55)).avec_anneau_saturne(vec3(0.92, 0.86, 0.68)));
    push("Anneau monobande (type Uranus)", gazeuse(vec3(0.5, 0.78, 0.78), vec3(0.34, 0.6, 0.62), vec3(0.66, 0.88, 0.88), 3.0, 0.6, vec3(0.55, 0.82, 0.85) * 0.3).avec_jets(0.2).avec_zonal_flou(0.4).avec_brume(0.3, vec3(0.6, 0.82, 0.84)).avec_pole(vec3(0.55, 0.78, 0.8)).avec_anneau_uranus(vec3(0.55, 0.8, 0.97)));
    push("Anneau ceinture d'asteroides", gazeuse(vec3(0.55, 0.5, 0.42), vec3(0.36, 0.32, 0.26), vec3(0.72, 0.66, 0.56), 5.0, 1.4, vec3(0.6, 0.55, 0.45) * 0.3).avec_pole(vec3(0.42, 0.4, 0.36)).avec_anneau_ceinture(vec3(0.78, 0.74, 0.66)));
    push("Anneaux en arcs (type Neptune)", gazeuse(vec3(0.45, 0.56, 0.86), vec3(0.10, 0.18, 0.42), vec3(0.36, 0.5, 0.88), 4.0, 1.5, vec3(0.3, 0.45, 0.9) * 0.3).avec_jets(0.55).avec_zonal_flou(0.3).avec_tempetes(0.8).avec_pole(vec3(0.3, 0.46, 0.74)).avec_anneau_neptune(vec3(0.6, 0.66, 0.85)));
    push("Anneau de debris recent", gazeuse(vec3(0.62, 0.4, 0.3), vec3(0.38, 0.22, 0.16), vec3(0.82, 0.6, 0.45), 5.0, 1.8, vec3(0.7, 0.45, 0.3) * 0.3).avec_thermique(0.4, vec3(0.7, 0.25, 0.08)).avec_tempetes(0.6).avec_pole(vec3(0.4, 0.26, 0.2)).avec_anneau_debris(vec3(0.85, 0.7, 0.55)));

    v
}

/// Renvoie l'apparence d'un preset **tellurique** nommé du catalogue, pour la réutiliser
/// dans les presets scénarisés (source unique de vérité avec la galerie). La géographie
/// (`seed`) est tirée aléatoirement, comme en galerie. Panique si le nom est inconnu.
pub fn preset_tellurique(nom: &str) -> Apparence {
    catalogue_telluriques()
        .into_iter()
        .find(|(n, _)| n == nom)
        .unwrap_or_else(|| panic!("preset tellurique inconnu: {}", nom))
        .1
}

/// Idem pour une **géante gazeuse** nommée du catalogue.
pub fn preset_gazeuse(nom: &str) -> Apparence {
    catalogue_gazeuses()
        .into_iter()
        .find(|(n, _)| n == nom)
        .unwrap_or_else(|| panic!("preset gazeuse inconnu: {}", nom))
        .1
}

/// Rayon + apparence d'une planète aléatoire (vue « objet » isolée). Couvre les
/// trois types ; la plage de température inclut les mondes de lave.
pub fn planete_aleatoire() -> (f32, Apparence) {
    let t: f32 = gen_range(0.0, 1.0);
    let (rayon, _masse, app) = if t < 0.45 {
        apparence_tellurique(gen_range(120.0, 620.0))
    } else if t < 0.8 {
        apparence_gazeuse()
    } else {
        apparence_glacee()
    };
    (rayon, app)
}
