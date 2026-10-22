//! **Les habillages du disque protoplanétaire** (lot 12.2) : ce qui lui
//! donne du volume et de la vie — couches, nuages en volume, grappes, amas,
//! bras. Mis au point en prototypes (touche 4), jugés bons le 2026-10-23
//! (*« c'est bien comme rendu sur les anneaux de débris, on garde tout »*),
//! puis versés dans la galerie « Ceintures & disques » à la demande.
//!
//! Un habillage se pose sur **n'importe quel** anneau protoplanétaire, et se
//! décline en quatre (`declinaisons`) :
//! - **base** : l'anneau de la galerie, 0,3–8 UA, deux sillons ondulés ;
//! - **-F** (*full*) : l'anneau complet, un seul sillon — celui qu'une planète
//!   y creuse ;
//! - **-R** (*ringed*) : deux ou trois anneaux du même habillage, des planètes
//!   dispersées entre eux et au-delà ;
//! - **-M** (*mixed*) : deux anneaux de l'habillage parmi d'autres types
//!   (ceinture d'astéroïdes, Kuiper).

use super::DisqueConfig;
use crate::etoile::UA;
use crate::genese::masse_luminosite;

/// Ce qui fait un habillage : posé sur un anneau, il le rhabille.
pub type Habillage = fn(DisqueConfig) -> DisqueConfig;

/// **Les habillages**, dans l'ordre de la galerie : `(code, nom, habillage)`.
///
/// ⚠️ **A (voile bouillonnant) refusé seul** à l'écran le 2026-10-22 — mais
/// **gardé dans D**, à la demande.
pub fn habillages() -> Vec<(&'static str, &'static str, Habillage)> {
    vec![
        ("B", "voile en couches", |c| c.avec_couches(7, 0.08)),
        ("C", "nuages epars", |c| c.avec_nuages_3d(500, 0.08)),
        ("D", "voile bouillonnant + couches + nuages", |c| {
            c.avec_nuages(0.8).avec_couches(7, 0.08).avec_nuages_3d(500, 0.08)
        }),
        ("E", "nuages epars denses", |c| c.avec_nuages_3d(1500, 0.08)),
        // « Des nuages plus cotonneux » : grappes de six boules, éclairées
        // d'un côté ; puis « un autocollant sur le disque » — d'où une
        // épaisseur triplée et la granulation du voile atténuée.
        ("F", "nuages cotonneux", cotonneux),
        // « Moins en orbites mais plus massé, l'aspect de gravité », décliné
        // en G1 tel quel, G2 deux fois plus de gros amas, G3 six fois plus.
        ("G1", "14 amas masses", |c| amas_masses(c, 14)),
        ("G2", "28 amas masses", |c| amas_masses(c, 28)),
        ("G3", "84 amas masses", |c| amas_masses(c, 84)),
        // « Encore plus fourni, un aspect artistique de nuages de débris ».
        ("H", "nebuleuse de debris", |c| {
            DisqueConfig { granulation: 0.1, ..c }.avec_nuages_3d(1600, 0.2).avec_grappe(5).avec_aspect_nuages(1.5, 0.8)
        }),
        // « Un aspect un peu ouragan » : l'étoile est l'œil ; 7 bras inégaux,
        // 1 800 nuages tassés au centre.
        ("I", "ouragan", ouragan),
    ]
}

/// **F** : 500 grappes de six boules éclairées, ±37 % du rayon, voile adouci.
pub fn cotonneux(c: DisqueConfig) -> DisqueConfig {
    DisqueConfig { granulation: 0.15, ..c }.avec_nuages_3d(500, 0.25).avec_grappe(6)
}

/// **I** : l'ouragan, dont l'étoile est l'œil.
pub fn ouragan(c: DisqueConfig) -> DisqueConfig {
    DisqueConfig { granulation: 0.15, ..c }.avec_nuages_3d(1800, 0.1).avec_grappe(5).avec_bras(7).avec_aspect_nuages(1.2, 0.0)
}

/// **G** : `n` amas massés près du plan, qui tournent d'un bloc — trente
/// nuages par amas, grappes de 8, nuages ×1,4, voile adouci.
fn amas_masses(c: DisqueConfig, n: u32) -> DisqueConfig {
    DisqueConfig { granulation: 0.15, ..c }
        .avec_nuages_3d(n * 30, 0.12)
        .avec_grappe(8)
        .avec_amas(n)
        .avec_aspect_nuages(1.4, 0.0)
}

/// La masse de l'étoile qui porte ces disques : une T Tauri (L = 0,6).
pub fn masse_t_tauri() -> f32 {
    masse_luminosite(0.6)
}

/// Un anneau protoplanétaire nu, entre deux rayons (UA). Ses particules
/// suivent sa largeur, pour une densité égale d'un anneau à l'autre.
pub fn anneau(interne_ua: f32, externe_ua: f32) -> DisqueConfig {
    let part = ((externe_ua - interne_ua) / 7.7 * 5000.0) as usize;
    DisqueConfig::protoplanetaire(part, interne_ua * UA, externe_ua * UA, masse_t_tauri())
}

/// Le disque protoplanétaire de la galerie des disques, sillons compris.
pub fn base() -> DisqueConfig {
    anneau(0.3, 8.0).avec_lacune_ondulee(0.38, 0.05, 0.92, 0.35).avec_lacune_ondulee(0.66, 0.06, 0.92, 0.35)
}

/// Une planète d'une déclinaison : `(rayon d'orbite en UA, géante ?)`.
pub type Planete = (f32, bool);

/// **Une déclinaison d'un habillage** : ses disques, ses planètes, et le
/// rayon qui contient le tout (UA), pour le cadrage.
pub struct Declinaison {
    pub suffixe: &'static str,
    pub libelle: String,
    pub disques: Vec<DisqueConfig>,
    pub planetes: Vec<Planete>,
    pub externe_ua: f32,
}

/// Le plus grand rayon des déclinaisons -R et -M (UA).
const ANNELE_EXTERNE: f32 = 12.5;
const MIXTE_EXTERNE: f32 = 13.0;

/// **Les quatre déclinaisons d'un habillage** : base, -F, -R, -M. `rang`
/// alterne les -R à trois et à deux anneaux.
pub fn declinaisons(rang: usize, nom: &str, habiller: Habillage) -> Vec<Declinaison> {
    // -F : un sillon creusé par une proto-planète en fusion (type PDS 70b).
    // Sans festons : la planète ne suit pas l'horloge du disque.
    const T_SILLON: f32 = 0.5;
    let complet = habiller(anneau(0.3, 8.0).avec_lacune(T_SILLON, 0.07, 0.95));
    let a_sillon = (complet.interne + T_SILLON * (complet.externe - complet.interne)) / UA;

    let nb = if rang.is_multiple_of(2) { 3 } else { 2 };
    let (anneaux, planetes_r) = plan_annele(nb);

    vec![
        Declinaison { suffixe: "", libelle: nom.to_string(), disques: vec![habiller(base())], planetes: Vec::new(), externe_ua: 8.0 },
        Declinaison {
            suffixe: "-F",
            libelle: format!("{nom}, complet + 1 planete"),
            disques: vec![complet],
            planetes: vec![(a_sillon, false)],
            externe_ua: 8.0,
        },
        Declinaison {
            suffixe: "-R",
            libelle: format!("{nom}, {nb} anneaux + planetes"),
            disques: anneaux.iter().map(|&(i, e)| partage(habiller(anneau(i, e)), nb as u32)).collect(),
            planetes: planetes_r,
            externe_ua: ANNELE_EXTERNE,
        },
        Declinaison {
            suffixe: "-M",
            libelle: format!("{nom}, parmi d'autres anneaux"),
            disques: vec![
                partage(habiller(anneau(0.3, 2.8)), 2),
                DisqueConfig::asteroides(900, 3.4 * UA, 4.4 * UA, masse_t_tauri()),
                partage(habiller(anneau(5.5, 8.0)), 2),
                DisqueConfig::kuiper(1400, 10.5 * UA, MIXTE_EXTERNE * UA, masse_t_tauri()),
            ],
            planetes: vec![(4.9, false), (9.3, true)],
            externe_ua: MIXTE_EXTERNE,
        },
    ]
}

/// **Les anneaux d'un -R**, en UA, et ses planètes : dans les écarts entre
/// anneaux, et une géante au-delà.
pub fn plan_annele(nb: usize) -> (Vec<(f32, f32)>, Vec<Planete>) {
    if nb >= 3 {
        (vec![(0.3, 2.2), (3.2, 5.2), (6.4, 9.0)], vec![(2.7, false), (5.8, false), (ANNELE_EXTERNE - 0.5, true)])
    } else {
        (vec![(0.3, 3.5), (5.0, 9.0)], vec![(4.2, false), (ANNELE_EXTERNE - 0.5, true)])
    }
}

/// Partage les nuages d'un habillage entre `n` anneaux (et les amas avec,
/// trente nuages chacun) : sans quoi un -R de G3 porterait trois fois ses
/// 20 000 boules.
fn partage(mut c: DisqueConfig, n: u32) -> DisqueConfig {
    c.nuages_3d /= n;
    if c.nuages_amas > 0 {
        c.nuages_amas = (c.nuages_amas / n).max(1);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Chaque habillage a son code, et quatre déclinaisons qui tiennent
    /// dans leur cadre.** Deux du même nom ne se distingueraient pas dans la
    /// galerie.
    #[test]
    fn chaque_habillage_a_ses_quatre_declinaisons() {
        let h = habillages();
        for (i, (code, nom, habiller)) in h.iter().enumerate() {
            assert!(!h[..i].iter().any(|b| b.0 == *code), "« {code} » en double");
            let d = declinaisons(i, nom, *habiller);
            assert_eq!(d.iter().map(|d| d.suffixe).collect::<Vec<_>>(), ["", "-F", "-R", "-M"]);
            for x in &d {
                let ext = x.disques.iter().map(|c| c.externe).fold(0.0, f32::max) / UA;
                assert!(ext <= x.externe_ua + 1e-3, "{code}{} : disque à {ext} UA hors cadre", x.suffixe);
                assert!(x.planetes.iter().all(|p| p.0 <= x.externe_ua), "{code}{} : planète hors cadre", x.suffixe);
            }
        }
    }

    /// **-R : les planètes passent entre les anneaux, jamais dedans.**
    #[test]
    fn les_planetes_d_un_annele_passent_entre_les_anneaux() {
        for nb in [2, 3] {
            let (anneaux, planetes) = plan_annele(nb);
            assert_eq!(anneaux.len(), nb);
            for w in anneaux.windows(2) {
                assert!(w[0].1 < w[1].0, "{nb} anneaux : {w:?} se chevauchent");
            }
            for &(a, _) in &planetes {
                assert!(!anneaux.iter().any(|&(i, e)| (i..=e).contains(&a)), "planète à {a} UA dans un anneau");
            }
        }
    }

    /// **-F : la planète est dans son sillon.**
    #[test]
    fn la_planete_d_un_complet_est_dans_son_sillon() {
        let d = &declinaisons(0, "x", |c| c)[1];
        let c = &d.disques[0];
        let t = (d.planetes[0].0 * UA - c.interne) / (c.externe - c.interne);
        assert!(c.lacunes.iter().any(|l| l.y > 0.0 && l.z > 0.0 && (t - l.x).abs() < l.y), "planète à t = {t} hors sillon");
    }
}
