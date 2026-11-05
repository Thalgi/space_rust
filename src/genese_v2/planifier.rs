//! **Planifier un système** (G1.3) : les règles de la genèse V1, relues dans
//! les flux nommés, et rendues sous forme de [`PlanSysteme`] — sans aucun
//! objet graphique, donc testables par milliers.
//!
//! Les règles sont celles de la V1 (`genese::construire_simple`,
//! `construire_multiple`), tirage pour tirage ; seule la source du hasard
//! change. Les **noms de flux** :
//!
//! | Flux | Ce qu'il tire |
//! |---|---|
//! | `genre` | simple ou multiple |
//! | `etoile`, k | le profil de la k-ième étoile tirée |
//! | `arbre` | séparations, excentricités et orientations des paires |
//! | `echelle` | l'échelle du système simple |
//! | `zone`, z | une zone de planètes : leur nombre et leur espacement |
//! | ↳ `planete`, i | une planète : type, orbite, lunes, graine d'apparence |
//! | ↳↳ `lune`, j | une lune : taille, orbite, aspect |
//! | `ceinture`, `kuiper`, `grand_disque` | un disque chacun |
//! | `cometes` ↳ `comete`, k | le nombre de comètes, puis chacune |

use super::alea::Alea;
use super::apparences;
use super::plan::{
    CorpsPlan, DisquePlan, EtoilePlan, FoyerPlan, GenreDisque, IdCorps, Nature, NoeudEtoiles, OrbitePlan,
    PlanSysteme, RecetteApparence, TypeCorps,
};
use crate::etoile::{self, Gabarit, ProfilEtoile};
use crate::genese::{
    echelle_systeme, masse_luminosite, orbite_comete, premiere_orbite, MASSE_NAINE_BRUNE, MASSE_QUASI_ETOILE,
    MASSE_TROU_NOIR, NB_COMETES,
};
use crate::stabilite;
use crate::stellaire::Variante;
use std::f32::consts::TAU;

/// **Un système tiré de sa graine.** ~55 % à une étoile, puis binaires,
/// trinaires et quadruples, comme la V1.
pub fn planifier(graine: u64) -> PlanSysteme {
    let u = Alea::flux(graine, "genre", 0).unite();
    if u < 0.55 {
        simple(graine, profil_alea(&mut Alea::flux(graine, "etoile", 0)))
    } else if u < 0.83 {
        multiple(graine, 2)
    } else if u < 0.94 {
        multiple(graine, 3)
    } else {
        multiple(graine, 4)
    }
}

/// **Un système à une étoile d'un type choisi** (le menu « choisir astre »,
/// la starmap). `None` si le tirage ne connaît pas ce type.
pub fn planifier_autour(graine: u64, type_nom: &str) -> Option<PlanSysteme> {
    let mut a = Alea::flux(graine, "etoile", 0);
    let g = etoile::gabarit_du_type(type_nom, a.unite())?;
    Some(simple(graine, profil_de(g, &mut a)))
}

/// Le tirage d'étoile de la V1 (`ProfilEtoile::aleatoire`) : une particulière
/// dans 5 % des cas, sinon une ordinaire.
fn profil_alea(a: &mut Alea) -> ProfilEtoile {
    let g = if a.unite() < etoile::PART_PARTICULIERES {
        etoile::particuliere(a.unite())
    } else {
        let p = a.unite();
        etoile::ordinaire(p, a.unite())
    };
    profil_de(g, a)
}

fn profil_de((tmin, tmax, rayon, luminosite, nom, couronne): Gabarit, a: &mut Alea) -> ProfilEtoile {
    let temperature = a.entre(tmin, tmax);
    ProfilEtoile { temperature, couleur: etoile::couleur_etoile(nom, temperature), rayon, luminosite, nom, couronne }
}

fn etoile_plan(p: &ProfilEtoile, masse: f32) -> EtoilePlan {
    EtoilePlan {
        type_nom: p.nom.to_string(),
        temperature: p.temperature,
        rayon: p.rayon,
        luminosite: p.luminosite,
        couronne: p.couronne,
        masse,
    }
}

/// Une étoile seule, Titius-Bode, deux ceintures, le grand disque d'une jeune
/// étoile, des comètes (`genese::construire_simple`).
fn simple(graine: u64, p: ProfilEtoile) -> PlanSysteme {
    let masse = if p.est_trou_noir() {
        MASSE_TROU_NOIR
    } else if p.est_quasi_etoile() {
        MASSE_QUASI_ETOILE
    } else if p.est_naine_brune() {
        MASSE_NAINE_BRUNE
    } else {
        masse_luminosite(p.luminosite)
    };
    let mut plan = PlanSysteme::new(graine, NoeudEtoiles::Etoile(etoile_plan(&p, masse)));
    let ech = echelle_systeme(&p, Alea::flux(graine, "echelle", 0).unite());

    let mut zone = Alea::flux(graine, "zone", 0);
    let nb = zone.entier(3, 7);
    let mut a = premiere_orbite(zone.entre(0.4, 0.8) * ech, p.rayon);
    let mut precedent: Option<(f32, f32)> = None;
    for i in 0..nb {
        let mut pa = zone.enfant("planete", i as u64);
        // Le type suit la distance relative à la ligne des neiges, qui suit `ech`.
        let a_norm = a / ech;
        let u = pa.unite();
        let type_corps = if a_norm < 2.0 {
            if u < 0.8 { TypeCorps::Tellurique } else { TypeCorps::Gazeuse }
        } else if a_norm < 6.0 {
            if u < 0.5 {
                TypeCorps::Gazeuse
            } else if u < 0.85 {
                TypeCorps::Tellurique
            } else {
                TypeCorps::Glacee
            }
        } else if u < 0.55 {
            TypeCorps::Glacee
        } else if u < 0.8 {
            TypeCorps::Gazeuse
        } else {
            TypeCorps::Tellurique
        };
        let temperature = etoile::temp_equilibre(p.luminosite, a);
        let recette = RecetteApparence { type_corps, temperature, graine: pa.suivant() };
        let (rayon, masse_p, _) = apparences::planete(&recette);
        // Garde-fou anti-collision : au moins (r_prec + r + marge) entre deux centres.
        if let Some((a_prec, r_prec)) = precedent {
            a = a.max(a_prec + (r_prec + rayon) / etoile::UA + 0.03);
        }
        let e = pa.entre(0.0, 0.2);
        let incl = pa.entre(0.0, 0.16);
        let orbite =
            OrbitePlan::Rails { foyer: FoyerPlan::Barycentre, masse_centrale: masse, a_ua: a, e, incl, azimut: pa.entre(0.0, TAU) };
        let id = plan.ajouter(CorpsPlan { nature: Nature::Planete(recette), orbite, rayon, masse: masse_p });
        precedent = Some((a, rayon));
        let n_lunes = match type_corps {
            TypeCorps::Gazeuse => pa.entier(1, 4),
            TypeCorps::Tellurique if a_norm > 0.8 => pa.entier(0, 2),
            TypeCorps::Tellurique => 0,
            TypeCorps::Glacee => pa.entier(0, 2),
        };
        ajouter_lunes(&mut plan, id, &pa, n_lunes, rayon, a * (1.0 - e) * etoile::UA, temperature);
        a *= zone.entre(1.5, 1.85);
    }

    let mut c = Alea::flux(graine, "ceinture", 0);
    let bi = c.entre(2.0, 3.0) * ech;
    let bo = bi + c.entre(0.6, 1.4) * ech;
    plan.disques.push(DisquePlan { genre: GenreDisque::Ceinture, interne_ua: bi, externe_ua: bo, masse_centrale: masse, graine: c.suivant() });
    // Le grand disque d'une jeune étoile : de la lisière de son petit disque à
    // la ceinture. Ses sillons suivent les orbites des planètes du plan.
    if matches!(Variante::de_mode(p.couronne), Variante::Proto) {
        let interne = crate::soleil::RAYON_PROTOSOLAIRE.1 * p.rayon / etoile::UA;
        let externe = 0.9 * bi;
        if externe > interne * 1.5 {
            let graine_d = Alea::flux(graine, "grand_disque", 0).suivant();
            plan.disques.push(DisquePlan { genre: GenreDisque::GrandDisque, interne_ua: interne, externe_ua: externe, masse_centrale: masse, graine: graine_d });
        }
    }
    let mut k = Alea::flux(graine, "kuiper", 0);
    let ki = bo + k.entre(20.0, 30.0) * ech;
    let ko = ki + k.entre(12.0, 20.0) * ech;
    plan.disques.push(DisquePlan { genre: GenreDisque::Kuiper, interne_ua: ki, externe_ua: ko, masse_centrale: masse, graine: k.suivant() });

    let cometes = Alea::flux(graine, "cometes", 0);
    let n = cometes.clone().entier(NB_COMETES.0, NB_COMETES.1);
    for i in 0..n {
        let mut co = cometes.enfant("comete", i as u64);
        let u_a = co.unite();
        let (a_ua, e) = orbite_comete(ech, u_a, co.unite());
        // Inclinaisons franches, jusqu'à 60° : une comète ne vient pas du plan
        // des planètes.
        let orbite = OrbitePlan::Rails {
            foyer: FoyerPlan::Barycentre,
            masse_centrale: masse,
            a_ua,
            e,
            incl: co.entre(0.0, 1.05),
            azimut: co.entre(0.0, TAU),
        };
        plan.ajouter(CorpsPlan { nature: Nature::Comete, orbite, rayon: co.entre(0.05, 0.12), masse: 1e-6 });
    }
    plan
}

/// Les lunes générées d'une planète, sur des créneaux croissants entre la
/// limite de Roche (~2,4 rayons) et ~14 % de sa distance à l'étoile
/// (`genese::poser_lune`). Aucune si la planète est trop près de l'étoile.
fn ajouter_lunes(plan: &mut PlanSysteme, parent: IdCorps, pa: &Alea, n: i32, rayon_planete: f32, dist_etoile: f32, temperature: f32) {
    let roche = rayon_planete * 2.4;
    let hill_max = 0.14 * dist_etoile;
    if hill_max <= roche * 1.1 {
        return;
    }
    let bande = hill_max - roche;
    for j in 0..n {
        let mut l = pa.enfant("lune", j as u64);
        let rayon = (rayon_planete * l.entre(0.05, 0.12)).max(0.05);
        let type_corps = if l.chance(0.5) { TypeCorps::Tellurique } else { TypeCorps::Glacee };
        let recette = RecetteApparence { type_corps, temperature, graine: l.suivant() };
        let frac = ((j as f32 + 0.6) / 5.0).min(0.92);
        let rayon_orbite = roche + bande * frac + l.entre(-0.04, 0.04) * bande;
        // Majoritairement prograde ; rétrograde occasionnel (Triton). Kepler :
        // ω décroît en (roche / r)^1,5.
        let sens = if l.chance(0.85) { 1.0 } else { -1.0 };
        let omega = sens * 1.2 * (roche / rayon_orbite).powf(1.5);
        let orbite = OrbitePlan::Lune { parent, rayon_orbite, omega, incl: l.entre(-0.12, 0.12), phase: l.entre(0.0, TAU) };
        plan.ajouter(CorpsPlan { nature: Nature::Lune(recette), orbite, rayon, masse: 0.05 });
    }
}

/// Une zone de planètes d'un système multiple : type S autour d'une étoile,
/// ou type P autour du barycentre (`genese::Plan`).
struct Zone {
    foyer: FoyerPlan,
    masse_centrale: f32,
    lumi_ref: f32,
    a_min: f32,
    a_max: f32,
}

fn zone_s(k: usize, masse: f32, lumi: f32, a_crit: f32) -> Zone {
    Zone { foyer: FoyerPlan::Etoile(k), masse_centrale: masse, lumi_ref: lumi, a_min: 0.3, a_max: a_crit * 0.75 }
}

fn zone_p(sep: f32, m1: f32, m2: f32, l1: f32, l2: f32, e: f32) -> Zone {
    let a_min = stabilite::a_crit_p(sep, stabilite::rapport_masse(m1, m2), e) * 1.25;
    Zone { foyer: FoyerPlan::Barycentre, masse_centrale: m1 + m2, lumi_ref: l1 + l2, a_min, a_max: a_min * 2.4 }
}

/// Une étoile d'un multiple : ni naine brune ni quasi-étoile (la V1 ne sait
/// pas les construire en compagne), huit essais au plus dans son flux.
fn feuille(graine: u64, k: u64) -> (NoeudEtoiles, f32, f32) {
    let mut a = Alea::flux(graine, "etoile", k);
    let mut p = profil_alea(&mut a);
    for _ in 0..8 {
        if !p.est_naine_brune() && !p.est_quasi_etoile() {
            break;
        }
        p = profil_alea(&mut a);
    }
    let masse = if p.est_trou_noir() { MASSE_TROU_NOIR } else { masse_luminosite(p.luminosite) };
    (NoeudEtoiles::Etoile(etoile_plan(&p, masse)), masse, p.luminosite)
}

/// Log-uniforme (Raghavan 2010 : pic ~40 UA, très large).
fn sep_log(ar: &mut Alea, min_ua: f32, max_ua: f32) -> f32 {
    ar.entre(min_ua.ln(), max_ua.ln()).exp()
}

fn paire(ar: &mut Alea, a: NoeudEtoiles, b: NoeudEtoiles, sep: f32, e: f32) -> NoeudEtoiles {
    NoeudEtoiles::Paire {
        a: Box::new(a),
        b: Box::new(b),
        sep,
        e,
        incl: ar.entre(-0.3, 0.3),
        phase: ar.entre(0.0, TAU),
        azimut: ar.entre(0.0, TAU),
    }
}

/// L'arbre stellaire et ses zones stables (`genese::arbre_et_plan`), puis
/// leurs planètes. Ni ceinture ni comète en multiple, comme la V1 (G5, §7.2).
fn multiple(graine: u64, nb: usize) -> PlanSysteme {
    let mut ar = Alea::flux(graine, "arbre", 0);
    let (racine, zones) = match nb {
        2 => {
            let (mut fa, mut ma, mut la) = feuille(graine, 0);
            let (mut fb, mut mb, mut lb) = feuille(graine, 1);
            // L'hôte (feuille 0) est la plus massive.
            if mb > ma {
                std::mem::swap(&mut fa, &mut fb);
                std::mem::swap(&mut ma, &mut mb);
                std::mem::swap(&mut la, &mut lb);
            }
            let sep = sep_log(&mut ar, 1.0, 60.0);
            let e = ar.entre(0.0, 0.45);
            let racine = paire(&mut ar, fa, fb, sep, e);
            let mut zones = Vec::new();
            let a_s_a = stabilite::a_crit_s(sep, mb / (ma + mb), e);
            if a_s_a >= 0.8 {
                zones.push(zone_s(0, ma, la, a_s_a));
            }
            let a_s_b = stabilite::a_crit_s(sep, ma / (ma + mb), e);
            if a_s_b >= 0.8 && ar.chance(0.5) {
                zones.push(zone_s(1, mb, lb, a_s_b));
            }
            // Circumbinaire : garanti sans zone S ; sinon seulement si le rayon
            // critique reste raisonnable.
            let a_p = stabilite::a_crit_p(sep, stabilite::rapport_masse(ma, mb), e);
            if zones.is_empty() || (a_p < 30.0 && ar.chance(0.6)) {
                zones.push(zone_p(sep, ma, mb, la, lb, e));
            }
            (racine, zones)
        }
        3 => {
            // ((A·B) · C) : paire serrée, C isolée loin, qui porte les planètes.
            let (fa, ma, _) = feuille(graine, 0);
            let (fb, mb, _) = feuille(graine, 1);
            let (fc, mc, lc) = feuille(graine, 2);
            let sep_in = sep_log(&mut ar, 0.3, 1.5);
            let e_in = ar.entre(0.0, 0.3);
            let ab = paire(&mut ar, fa, fb, sep_in, e_in);
            let m_ab = ma + mb;
            let e_out = ar.entre(0.0, 0.4);
            let lo = (stabilite::a_crit_p(sep_in, stabilite::rapport_masse(ma, mb), e_in) * 1.6).max(15.0);
            let sep_out = sep_log(&mut ar, lo, lo.max(45.0));
            let racine = paire(&mut ar, ab, fc, sep_out, e_out);
            let a_s_c = stabilite::a_crit_s(sep_out, m_ab / (m_ab + mc), e_out);
            (racine, vec![zone_s(2, mc, lc, a_s_c.max(1.0))])
        }
        _ => {
            // (((A·B) · C) · D) : triple compact, D isolée loin, qui porte les planètes.
            let (fa, ma, _) = feuille(graine, 0);
            let (fb, mb, _) = feuille(graine, 1);
            let (fc, mc, _) = feuille(graine, 2);
            let (fd, md, ld) = feuille(graine, 3);
            let sep_ab = sep_log(&mut ar, 0.3, 1.2);
            let e_ab = ar.entre(0.0, 0.3);
            let ab = paire(&mut ar, fa, fb, sep_ab, e_ab);
            let m_ab = ma + mb;
            let e_c = ar.entre(0.0, 0.3);
            let sep_c_min = stabilite::a_crit_p(sep_ab, stabilite::rapport_masse(ma, mb), e_ab) * 1.6;
            let sep_c = sep_c_min.max(2.0) * ar.entre(1.0, 2.0);
            let abc = paire(&mut ar, ab, fc, sep_c, e_c);
            let m_abc = m_ab + mc;
            let e_out = ar.entre(0.0, 0.4);
            let lo = (stabilite::a_crit_p(sep_c, stabilite::rapport_masse(m_ab, mc), e_c) * 1.6).max(20.0);
            let sep_out = sep_log(&mut ar, lo, lo.max(55.0));
            let racine = paire(&mut ar, abc, fd, sep_out, e_out);
            let a_s_d = stabilite::a_crit_s(sep_out, m_abc / (m_abc + md), e_out);
            (racine, vec![zone_s(3, md, ld, a_s_d.max(1.0))])
        }
    };
    let mut plan = PlanSysteme::new(graine, racine);
    for (z, zone) in zones.iter().enumerate() {
        peupler_zone(&mut plan, zone, Alea::flux(graine, "zone", z as u64));
    }
    plan
}

/// Une courte séquence de planètes, de `a_min` à `a_max` (`genese::placer_planetes`).
fn peupler_zone(plan: &mut PlanSysteme, zone: &Zone, mut z: Alea) {
    let n = z.entier(1, 5);
    let mut a = zone.a_min * z.entre(1.0, 1.3);
    for i in 0..n {
        if a > zone.a_max {
            break;
        }
        let mut pa = z.enfant("planete", i as u64);
        let temperature = etoile::temp_equilibre(zone.lumi_ref, a);
        let u = pa.unite();
        let type_corps = if temperature > 250.0 {
            if u < 0.55 { TypeCorps::Tellurique } else { TypeCorps::Gazeuse }
        } else if temperature > 150.0 {
            if u < 0.5 { TypeCorps::Gazeuse } else { TypeCorps::Glacee }
        } else {
            TypeCorps::Glacee
        };
        let recette = RecetteApparence { type_corps, temperature, graine: pa.suivant() };
        let (rayon, masse_p, _) = apparences::planete(&recette);
        let e = pa.entre(0.0, 0.12);
        let orbite = OrbitePlan::Rails {
            foyer: zone.foyer,
            masse_centrale: zone.masse_centrale,
            a_ua: a,
            e,
            incl: pa.entre(0.0, 0.08),
            azimut: pa.entre(0.0, TAU),
        };
        let id = plan.ajouter(CorpsPlan { nature: Nature::Planete(recette), orbite, rayon, masse: masse_p });
        let n_lunes = if type_corps == TypeCorps::Gazeuse { pa.entier(1, 4) } else { pa.entier(0, 2) };
        ajouter_lunes(plan, id, &pa, n_lunes, rayon, a * (1.0 - e) * etoile::UA, temperature);
        a *= z.entre(1.4, 1.8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAINES: u64 = 2_000;

fn type_planete(t: TypeCorps) -> crate::planete::TypePlanete {
    match t {
        TypeCorps::Tellurique => crate::planete::TypePlanete::Tellurique,
        TypeCorps::Gazeuse => crate::planete::TypePlanete::Gazeuse,
        TypeCorps::Glacee => crate::planete::TypePlanete::Glacee,
    }
}

    fn planetes(plan: &PlanSysteme) -> Vec<(&CorpsPlan, &RecetteApparence)> {
        plan.corps
            .iter()
            .filter_map(|c| match &c.nature {
                Nature::Planete(r) => Some((c, r)),
                _ => None,
            })
            .collect()
    }

    fn a_ua(c: &CorpsPlan) -> f32 {
        match c.orbite {
            OrbitePlan::Rails { a_ua, .. } => a_ua,
            OrbitePlan::Lune { .. } => panic!("une planète est sur rails"),
        }
    }

    #[test]
    fn la_meme_graine_rejoue_le_meme_plan() {
        for g in 0..200 {
            assert_eq!(planifier(g), planifier(g), "graine {g}");
        }
        assert_ne!(planifier(1), planifier(2));
    }

    /// Le rayon et la masse stockés sont ceux que la recette refait : la
    /// construction (G1.4) retrouvera exactement le corps tiré.
    #[test]
    fn la_recette_refait_le_meme_corps() {
        for g in 0..300 {
            for (c, r) in planetes(&planifier(g)) {
                let (rayon, masse, app) = apparences::planete(r);
                assert_eq!((rayon, masse), (c.rayon, c.masse), "graine {g}");
                assert!(app.type_p == type_planete(r.type_corps), "graine {g}");
            }
        }
    }

    /// Les nombres de la V1 : 3 à 6 planètes autour d'une étoile seule, 1 à
    /// 3 comètes, une ceinture et une ceinture de Kuiper ; ni comète ni
    /// ceinture en multiple ; 1 à 3 lunes par géante.
    #[test]
    fn les_comptes_suivent_les_regles_de_la_v1() {
        for g in 0..GRAINES {
            let plan = planifier(g);
            let simple = matches!(plan.etoiles, NoeudEtoiles::Etoile(_));
            let n_pl = planetes(&plan).len();
            let n_co = plan.corps.iter().filter(|c| c.nature == Nature::Comete).count();
            let genre = |x| plan.disques.iter().filter(|d| d.genre == x).count();
            if simple {
                assert!((3..=6).contains(&n_pl), "graine {g} : {n_pl} planètes");
                assert!((1..=3).contains(&n_co), "graine {g} : {n_co} comètes");
                assert_eq!((genre(GenreDisque::Ceinture), genre(GenreDisque::Kuiper)), (1, 1), "graine {g}");
            } else {
                // Au plus trois zones (S, S, P) de quatre planètes.
                assert!((1..=12).contains(&n_pl), "graine {g} : {n_pl} planètes en multiple");
                assert_eq!((n_co, plan.disques.len()), (0, 0), "graine {g}");
            }
            for (i, c) in plan.corps.iter().enumerate() {
                if let Nature::Planete(r) = &c.nature {
                    let lunes = plan
                        .corps
                        .iter()
                        .filter(|l| matches!(l.orbite, OrbitePlan::Lune { parent, .. } if parent == IdCorps(i as u32)))
                        .count();
                    assert!(lunes <= 3, "graine {g} : {lunes} lunes");
                    if r.type_corps != TypeCorps::Gazeuse {
                        assert!(lunes <= 1, "graine {g} : {lunes} lunes autour d'une non-géante");
                    }
                }
            }
        }
    }

    /// Deux planètes voisines d'une étoile seule ne se touchent jamais.
    #[test]
    fn les_planetes_voisines_ne_se_chevauchent_pas() {
        let mut serrees = 0;
        for g in 0..GRAINES {
            let plan = planifier(g);
            if !matches!(plan.etoiles, NoeudEtoiles::Etoile(_)) {
                continue;
            }
            let p = planetes(&plan);
            for w in p.windows(2) {
                let (c0, c1) = (w[0].0, w[1].0);
                let ecart = a_ua(c1) - a_ua(c0) - (c0.rayon + c1.rayon) / etoile::UA;
                assert!(ecart >= 0.03 - 1e-4, "graine {g} : écart {ecart}");
                serrees += usize::from(ecart < 0.031);
            }
        }
        // La garde mord vraiment : sans elle, des voisines se chevauchent.
        assert!(serrees > 0, "le garde-fou n'a jamais servi");
    }

    /// Une lune reste entre la limite de Roche de sa planète et ~14 % de la
    /// distance de celle-ci à l'étoile.
    #[test]
    fn les_lunes_restent_dans_le_domaine_de_leur_planete() {
        for g in 0..GRAINES {
            let plan = planifier(g);
            for l in &plan.corps {
                let OrbitePlan::Lune { parent, rayon_orbite, .. } = l.orbite else { continue };
                let p = plan.corps(parent).expect("parent présent");
                let OrbitePlan::Rails { a_ua, e, .. } = p.orbite else { panic!("parent sur rails") };
                let roche = p.rayon * 2.4;
                let hill = 0.14 * a_ua * (1.0 - e) * etoile::UA;
                assert!(roche < rayon_orbite && rayon_orbite < hill, "graine {g} : {roche} < {rayon_orbite} < {hill}");
            }
        }
    }

    #[test]
    fn un_systeme_sur_deux_environ_a_plusieurs_etoiles() {
        let multiples = (0..GRAINES).filter(|&g| !matches!(planifier(g).etoiles, NoeudEtoiles::Etoile(_))).count();
        let part = multiples as f32 / GRAINES as f32;
        assert!((0.40..0.50).contains(&part), "part de multiples : {part}");
    }

    #[test]
    fn planifier_autour_impose_le_type() {
        for g in 0..50 {
            let plan = planifier_autour(g, "Geante rouge").expect("type connu");
            let NoeudEtoiles::Etoile(e) = &plan.etoiles else { panic!("une seule étoile") };
            assert_eq!(e.type_nom, "Geante rouge");
        }
        assert!(planifier_autour(1, "Licorne").is_none());
    }
}
