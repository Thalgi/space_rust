//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22) — Maj+4 pour revenir en arrière.
//!
//! ⚠️ **La règle, posée par l'utilisateur** : cette section n'accueille **que**
//! des prototypes, et un prototype **validé en est retiré** — il passe alors
//! dans le jeu par sa source unique (le tirage, `Variante::appliquer`, la
//! genèse…). Quand la liste est vide, la touche 4 affiche « aucun prototype en
//! cours ». Les galeries montrent ce que le jeu produit ; les essais vivent ici.
//!
//! Les habillages du disque protoplanétaire en sont sortis le 2026-10-23,
//! versés dans la galerie « Ceintures & disques » (`disque::habillage`).
//! Place à la **T Tauri** : son disque, rhabillé avec eux.
//!
//! La liste ne construit rien à la déclaration : chaque entrée porte une
//! fonction de construction, appelée à l'affichage (un `Soleil` exige le
//! contexte graphique, la liste elle-même se teste sans lui).

use crate::disque::DisqueConfig;
use crate::soleil::{Soleil, ACCRETION_PROTO, JETS_PROTO};
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre.
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// Le type travaillé, tel que le catalogue le nomme.
const NOM: &str = "T Tauri (jeune)";

/// Un degré de coton : rhabille le petit disque, `largeur` étant son rayon
/// externe rapporté à celui du jeu (16 rayons stellaires) — les nuages
/// suivent, pour garder leur densité.
type Coton = fn(DisqueConfig, f32) -> DisqueConfig;

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 12, la T Tauri. Premier tour (2026-10-23) : T1, petit disque en couches
/// et nuages cotonneux, retenu. Second tour : élargi (×1,5, ×2) et plus
/// cotonneux ; T5, deux fois plus large et très cotonneux, retenu — *« il
/// devient le nouveau T0. Pour T1 j'aimerais que le disque soit plus large et
/// un peu plus plat, pour les autres protos teste des idées »*.
pub fn prototypes() -> Vec<Prototype> {
    let r = gabarit().2;
    let t = |nom: &str, externe: f32, coton: Coton| Prototype {
        nom: format!("T Tauri {nom}"),
        construire: Box::new(move || jeune(externe, coton)),
        // Le disque se cadre à 2,2 fois son rayon externe.
        distance: r * externe * 2.2,
    };
    let b = |nom: &str, coton: Coton| Prototype {
        nom: format!("T Tauri {nom}"),
        construire: Box::new(move || jeune(48.0, coton)),
        distance: r * 24.0 * 2.2,
    };
    vec![
        t("T0 : large et tres cotonneux (retenu)", 32.0, t0),
        t("T1 : T0 plus large et plus plat", 48.0, plus_plat),
        t("T2 : T0, voile allege", 32.0, voile_allege),
        t("T3 : T0, teintes variees", 32.0, teintes),
        t("T4 : T0 en grumeaux", 32.0, grumeaux),
        t("T5 : T0 evase", 32.0, evase),
        // **Les bras de matière** (2026-10-23, `disque::bras`) : sur T1 — *« T1
        // est bien »* —, plusieurs solutions à comparer. Cadrées plus serré,
        // sur 24 rayons : les bras vivent près de l'étoile.
        b("T1B1 : dipole incline, deux flots", t1b1),
        b("T1B2 : dipole aligne, rideaux tout autour", t1b2),
        b("T1B3 : bras spiraux jusqu'a l'etoile", t1b3),
        b("T1B4 : bras spiraux puis flots magnetiques", t1b4),
    ]
}

/// **T1B1** : dipôle incliné de 20°, deux flots — un par hémisphère, face à
/// chaque pôle magnétique (simulations MHD de Romanova et al.).
fn t1b1(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    cavite(plus_plat(c, largeur)).avec_entonnoirs(2, gabarit().2, 0.35)
}

/// **La cavité magnétosphérique** : le champ tronque le disque vers 5 rayons
/// stellaires (3 à 8 chez les T Tauri), contre 3 au petit disque du jeu —
/// les flots qui en partent y gagnent leur longueur.
fn cavite(c: DisqueConfig) -> DisqueConfig {
    DisqueConfig { interne: 5.0 * gabarit().2, ..c }
}

/// **T1B2** : dipôle aligné, six rideaux tout autour, vers les deux pôles.
fn t1b2(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    cavite(plus_plat(c, largeur)).avec_entonnoirs(6, gabarit().2, 0.0)
}

/// **T1B3** : quatre bras spiraux, du tiers interne du disque jusqu'à
/// l'étoile, la matière chauffant en tombant.
fn t1b3(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    cavite(plus_plat(c, largeur)).avec_spirales_accretion(4, gabarit().2)
}

/// **T1B4** : le chemin complet — trois bras spiraux amènent la matière au
/// bord interne, deux flots magnétiques la portent jusqu'aux pôles.
fn t1b4(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    cavite(plus_plat(c, largeur)).avec_spirales_accretion(3, gabarit().2).avec_entonnoirs(2, gabarit().2, 0.35)
}

/// **T0** (l'ex-T5) : voile en sept couches, 1 200 grappes de huit boules
/// par largeur de 16 rayons, nuages ×1,7, ±45 % du rayon, voile adouci.
fn t0(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    DisqueConfig { granulation: 0.1, ..c }
        .avec_couches(7, 0.08)
        .avec_nuages_3d((1200.0 * largeur) as u32, 0.3)
        .avec_grappe(8)
        .avec_aspect_nuages(1.7, 0.0)
}

/// **T1** : plus plat — nuages ramenés à ±27 % du rayon, couches resserrées.
fn plus_plat(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    let c = t0(c, largeur);
    DisqueConfig { h_r: 0.18, ..c }.avec_couches(7, 0.05)
}

/// **T2** : le voile à 45 % — par la tranche, la bande sombre qu'il fait
/// laisse voir les nuages d'au-delà.
fn voile_allege(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    t0(DisqueConfig { voile_alpha: c.voile_alpha * 0.45, ..c }, largeur)
}

/// **T3** : chaque nuage poussé vers le chaud ou le froid (l'écart de H).
fn teintes(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    t0(c, largeur).avec_aspect_nuages(1.7, 0.5)
}

/// **T4** : la matière en grumeaux — les nuages groupés en amas de trente,
/// qui tournent d'un bloc (G), comme des planètes en formation.
fn grumeaux(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    let c = t0(c, largeur);
    let amas = (c.nuages_3d / 30).max(1);
    c.avec_amas(amas)
}

/// **T5** : évasé — mince près de l'étoile, épais au bord, la forme d'un
/// vrai disque jeune vu par la tranche.
fn evase(c: DisqueConfig, largeur: f32) -> DisqueConfig {
    t0(c, largeur).avec_evasement(0.6)
}

/// `(tmin, tmax, rayon, luminosité, nom, mode)` de la T Tauri, pris au
/// catalogue plutôt que recopié.
fn gabarit() -> crate::etoile::Gabarit {
    crate::etoile::gabarit_du_type(NOM, 0.5).expect("la T Tauri est au catalogue")
}

/// **La T Tauri**, ses jets, ses taches d'accrétion, et son petit disque en
/// couches, porté à `externe` rayons stellaires et rhabillé de `coton`.
fn jeune(externe: f32, coton: Coton) -> Systeme {
    let (tmin, tmax, rayon, lumi, _, _) = gabarit();
    let s = Soleil::new(Vec3::ZERO, rayon, crate::etoile::couleur_etoile(NOM, (tmin + tmax) * 0.5), lumi)
        .avec_jets_3d(JETS_PROTO)
        .avec_accretion_polaire(ACCRETION_PROTO);
    let mut c = s.config_protosolaire();
    let largeur = externe * rayon / c.externe;
    c.externe = externe * rayon;
    let c = coton(c, largeur);
    let mut sys = Systeme::new();
    sys.ajouter(Box::new(s.avec_disque(c)));
    sys
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Chaque prototype a un nom à lui, et un cadre qui le contient.**
    /// Deux du même nom ne se distingueraient pas à l'écran ; un cadre trop
    /// serré couperait ce qu'on vient juger.
    #[test]
    fn chaque_prototype_a_son_nom_et_son_cadre() {
        let p = prototypes();
        for (i, a) in p.iter().enumerate() {
            assert!(!a.nom.is_empty());
            assert!(!p[..i].iter().any(|b| b.nom == a.nom), "« {} » en double", a.nom);
            assert!(a.distance > 0.0 && a.distance <= crate::camera::DIST_MAX, "« {} » : cadre {}", a.nom, a.distance);
        }
    }

    /// **Plus large, mêmes nuages au mètre carré près** : un disque élargi
    /// porte plus de grappes, à proportion — sinon il s'éclaircirait. Et T1
    /// est bien plus plat que T0.
    #[test]
    fn un_disque_elargi_garde_ses_nuages() {
        let c = DisqueConfig::protosolaire(0, 3.0, 16.0, 1000.0);
        assert_eq!(t0(c, 1.0).nuages_3d, 1200);
        assert_eq!(t0(c, 2.0).nuages_3d, 2400);
        assert!(plus_plat(c, 1.0).h_r < t0(c, 1.0).h_r, "T1 n'est pas plus plat");
    }
}
