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

use super::flot::Flot;
use super::DisqueConfig;
use macroquad::prelude::*;
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

// ---- La jeune étoile du jeu (lot 12, adoptée le 2026-10-24) ----

/// **Le disque d'une jeune étoile, tel que le jeu le pose** : T1B4 pour la
/// T Tauri, T1B2 pour la Herbig Ae/Be, retenus à l'écran.
///
/// `c` est son disque protosolaire nu (`Soleil::config_protosolaire`, de 5 à
/// 48 rayons stellaires), `rayon` celui de l'étoile, `graine` la sienne.
///
/// - **Habillage commun** (l'ex-T1) : voile en sept couches, grappes de huit
///   boules ×1,7, voile adouci. ⚠️ `avec_couches` et `avec_nuages_3d` règlent
///   **le même** `h_r`, et le dernier appel gagne : le T1 jugé à l'écran avait
///   h/r **0,05** (nuages compris), pas les 0,18 que sa doc annonçait. C'est ce
///   qu'on a vu et validé, c'est ce qui est posé.
/// - **T Tauri** : T1B4 — bras spiraux puis flots magnétiques —, **varié par
///   la graine** comme le montraient les tirages (*« le reste a sa pertinence,
///   on pourrait l'utiliser pour d'autres systèmes random »*) : cavité de 4 à
///   7 rayons, dipôle de 0 à 34° (rideaux s'il est presque droit, deux flots
///   sinon), deux à quatre bras. Dérivé, jamais tiré.
/// - **Herbig Ae/Be** : T1B2 — cavité de 5 rayons, dipôle aligné, six
///   rideaux.
///
/// Le bord interne brûlant (T1B4-A) et le grand disque (T1B4-G) restent en
/// prototypes : pas encore validés.
pub fn jeune_etoile(c: DisqueConfig, rayon: f32, graine: f32, herbig: bool) -> DisqueConfig {
    let largeur = c.externe / rayon / 16.0;
    let c = DisqueConfig { granulation: 0.1, ..c }
        .avec_nuages_3d((1200.0 * largeur) as u32, 0.05)
        .avec_couches(7, 0.05)
        .avec_grappe(8)
        .avec_aspect_nuages(1.7, 0.0);
    let c = c.avec_flot(jet_standard(rayon, c.externe));
    if herbig {
        return DisqueConfig { interne: 5.0 * rayon, ..c }.avec_entonnoirs(6, rayon, 0.0);
    }
    let f = |k: f32| (graine * 0.618_034 * k).fract();
    let incl = 0.6 * f(7.13);
    let flots = if incl > 0.15 { 2 } else { 4 + (f(2.9) * 3.0) as u32 };
    DisqueConfig { interne: (4.0 + 3.0 * f(11.1)) * rayon, ..c }
        .avec_spirales_accretion(2 + (f(3.7) * 3.0) as u32, rayon)
        .avec_entonnoirs(flots, rayon, incl)
}

/// **Le grand disque protoplanétaire d'un système jeune, choisi par la
/// genèse** (2026-10-24) : cotonneux, de `interne_ua` à `externe_ua`, avec un
/// **sillon à chaque orbite de planète** qui le traverse — c'est elle qui l'y
/// creuse. *« Le disque doit pouvoir être choisi pour travailler correctement
/// avec le module genèse »* : la démo du prototype T1B4-G, rendue au système
/// qui le porte. Quatre sillons au plus (les emplacements de `lacunes`) : les
/// planètes suivantes restent dans le disque.
///
/// Normale **−Y** : le plan des planètes de `poser_sur_rails`, et leur sens
/// de rotation (`a1 × q = −Y`) — sans quoi le disque tournerait à rebours
/// d'elles.
pub fn grand_disque(interne_ua: f32, externe_ua: f32, masse: f32, orbites_ua: &[f32]) -> DisqueConfig {
    let part = ((externe_ua - interne_ua) / 7.7 * 5000.0).max(0.0) as usize;
    let mut c = DisqueConfig::protoplanetaire(part, interne_ua * UA, externe_ua * UA, masse).avec_normale(-Vec3::Y);
    for &a in orbites_ua.iter().filter(|&&a| a > interne_ua && a < externe_ua) {
        c = c.avec_lacune((a - interne_ua) / (externe_ua - interne_ua), 0.04, 0.92);
    }
    cotonneux(c)
}

/// **Le disque de décrétion d'une étoile Be** (lot 12, 2026-10-24) : la
/// matière qu'elle **éjecte** par son équateur, tant elle tourne vite — un
/// disque de gaz mince, sans poussière, collé à sa surface (de 1 à `etendue`
/// rayons), qui brille de lui-même : blanc-bleu contre l'étoile, rose plus
/// loin (la raie Hα qui fait d'elles des « Be », *e* pour émission).
/// `nuages` > 0 : des nappes de gaz en volume, très minces.
pub fn decretion(rayon: f32, masse: f32, etendue: f32, nuages: u32) -> DisqueConfig {
    let c = DisqueConfig {
        voile_alpha: 0.6,
        voile_couleur: vec3(1.0, 0.45, 0.6),
        voile_couleur2: vec3(0.85, 0.9, 1.0),
        voile_bord: 0.15,
        granulation: 0.2,
        emissif: 1.0,
        ..DisqueConfig::protosolaire(0, rayon * 1.02, rayon * etendue, masse)
    }
    .avec_couches(3, 0.02);
    if nuages == 0 {
        return c;
    }
    c.avec_nuages_3d(nuages, 0.03).avec_grappe(4).avec_aspect_nuages(0.8, 0.0)
}

/// **Le jet polaire standard du chantier** (J5, validé le 2026-10-24) :
/// traînée de poussière en cône, sablier très fin éclairé par plaques, lobes
/// dissymétriques courbés en S ; long de 1,2 fois le rayon `externe` du
/// disque. *« Ce qui rendra au niveau des jets polaires aura été mis au
/// standard pour tous les cas de jets polaires de ce chantier »* : tout astre
/// qui **accrète** et porte un jet le prend d'ici (jeunes étoiles ; au
/// lot 13, les étoiles à neutrons accrétantes — pas le pulsar isolé).
pub fn jet_standard(rayon_astre: f32, externe: f32) -> Flot {
    Flot {
        longueur: 1.2 * externe,
        rayon_astre,
        jet: 1.0,
        cone: 1.0,
        parois: 1.0,
        finesse: 0.9,
        asymetrie: 1.0,
        ..Flot::default()
    }
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

    /// **La jeune étoile du jeu** : la Herbig a ses six rideaux droits ; la
    /// T Tauri ses bras et ses flots, dans les bornes des tirages, la même à
    /// graine égale et pas la même d'une graine à l'autre.
    #[test]
    fn la_jeune_etoile_varie_avec_sa_graine() {
        let nu = DisqueConfig::protosolaire(0, 5.0, 48.0, 1000.0);
        let h = jeune_etoile(nu, 1.0, 3.0, true);
        assert_eq!((h.entonnoirs, h.inclinaison_dipole, h.spirales_accretion), (6, 0.0, 0));
        // Le jet standard, pour l'une comme pour l'autre.
        assert_eq!(h.flot, jet_standard(1.0, 48.0));
        assert_eq!(jeune_etoile(nu, 1.0, 3.0, false).flot, jet_standard(1.0, 48.0));
        assert_eq!(h.interne, 5.0);
        let mut vus = std::collections::HashSet::new();
        for k in 0..40 {
            let g = k as f32 * 23.7;
            let t = jeune_etoile(nu, 1.0, g, false);
            assert!((4.0..=7.0).contains(&t.interne), "cavité {}", t.interne);
            assert!((2..=4).contains(&t.spirales_accretion) && (2..=6).contains(&t.entonnoirs));
            assert!((0.0..=0.6).contains(&t.inclinaison_dipole));
            assert_eq!(t.nuages_3d, 3600, "la densité de T1 (48 rayons)");
            let u = jeune_etoile(nu, 1.0, g, false);
            assert_eq!((t.interne, t.entonnoirs, t.spirales_accretion), (u.interne, u.entonnoirs, u.spirales_accretion));
            vus.insert((t.entonnoirs, t.spirales_accretion));
        }
        assert!(vus.len() >= 4, "les graines ne varient presque pas : {vus:?}");
    }

    /// **Le grand disque se creuse aux orbites qui le traversent**, et
    /// seulement là ; il tourne dans le plan et le sens des planètes.
    #[test]
    fn le_grand_disque_se_creuse_aux_orbites() {
        let c = grand_disque(1.0, 5.0, 1000.0, &[0.5, 2.0, 3.0, 9.0]);
        let sillons: Vec<f32> = c.lacunes.iter().filter(|l| l.y > 0.0 && l.z > 0.0).map(|l| l.x).collect();
        assert_eq!(sillons.len(), 2, "{sillons:?}");
        assert!((sillons[0] - 0.25).abs() < 1e-4 && (sillons[1] - 0.5).abs() < 1e-4, "{sillons:?}");
        assert_eq!(c.normale, -Vec3::Y);
        assert!(c.nuages_3d > 0, "pas cotonneux");
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
