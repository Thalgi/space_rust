//! **Les prototypes en cours**, atteignables par la **touche 4** de la vue
//! isolée (2026-10-22) — Maj+4 pour revenir en arrière.
//!
//! ⚠️ **La règle, posée par l'utilisateur** : cette section n'accueille **que**
//! des prototypes, et un prototype **validé en est retiré** — il passe alors
//! dans le jeu par sa source unique (le tirage, `Variante::appliquer`, la
//! genèse…). Quand la liste est vide, la touche 4 affiche « aucun prototype en
//! cours ». Les galeries montrent ce que le jeu produit ; les essais vivent ici.
//!
//! La liste ne construit rien à la déclaration : chaque entrée porte une
//! fonction de construction, appelée à l'affichage (un `Soleil` exige le
//! contexte graphique, la liste elle-même se teste sans lui).

use crate::disque::DisqueConfig;
use crate::genese::masse_luminosite;
use crate::soleil::Soleil;
use crate::stellaire::Variante;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un prototype : son nom, sa construction, et la distance de caméra qui le
/// cadre.
pub struct Prototype {
    pub nom: String,
    pub construire: Box<dyn Fn() -> Systeme>,
    pub distance: f32,
}

/// La naine blanche du catalogue : `(rayon, luminosité)`, au milieu de ses
/// plages (`etoile::ordinaire`).
const NAINE: (f32, f32) = (0.65, 0.27);

/// Ce qu'une naine polluée porte autour d'elle.
#[derive(Clone, Copy)]
enum Autour {
    Rien,
    /// L'anneau des débris de ses planètes.
    Anneau,
    /// Un corps qui se désagrège en orbite serrée, sa traînée de poussière.
    Fragments,
    /// Une pluie diffuse de débris, chacun sur sa spirale.
    Pluie,
    /// Un anneau de gaz chaud au ras de l'étoile.
    Gaz,
}

/// **Les prototypes en cours.** Vide = aucun prototype : la touche 4 le dit.
///
/// Lot 14 (2026-10-26), les étoiles vertes — **elles existent** : *« check un
/// peu le net, on a possiblement découvert une étoile verte »*. WD J1330+6435
/// (université de Warwick, RASTI, octobre 2026), à ~280 années-lumière dans
/// le Dragon, est une **naine blanche polluée** : elle a avalé les débris
/// rocheux de son système planétaire, et le sodium de son atmosphère absorbe
/// l'orange du spectre — reste un vert-jaune (chartreuse), prédit d'après son
/// spectre. La même équipe annonce des naines polluées roses, violettes,
/// jaunes ou cyan, et la plus rouge connue (WD J1011+2845).
pub fn prototypes() -> Vec<Prototype> {
    let v = |nom: &str, couleur: Vec3, autour: Autour| Prototype {
        nom: format!("Naine polluee {nom}"),
        construire: Box::new(move || naine(couleur, autour)),
        distance: NAINE.0 * if matches!(autour, Autour::Rien) { 9.0 } else { 38.0 },
    };
    let chartreuse = vec3(0.78, 1.0, 0.42);
    vec![
        v("V1 : verte (sodium), seule", chartreuse, Autour::Rien),
        v("V2 : verte + anneau des debris de ses planetes", chartreuse, Autour::Anneau),
        // ⚠️ V3 (anneau + trois bras spiraux) rendait mal — *« il faut
        // changer de la spirale à trois branches »* : trois manières de
        // tomber, tirées des naines polluées observées.
        v("V3a : verte + un corps qui se desagrege (WD 1145+017)", chartreuse, Autour::Fragments),
        v("V3b : verte + pluie diffuse de debris", chartreuse, Autour::Pluie),
        v("V3c : verte + anneau de gaz chaud au ras de l'etoile", chartreuse, Autour::Gaz),
        // La famille annoncée, pour varier plus tard.
        v("V4 : rose", vec3(1.0, 0.7, 0.86), Autour::Rien),
        v("V5 : cyan", vec3(0.55, 0.95, 1.0), Autour::Rien),
        v("V6 : la plus rouge (WD J1011+2845)", vec3(1.0, 0.42, 0.32), Autour::Rien),
    ]
}

/// **Une naine blanche polluée** : la naine blanche du jeu (sa surface, par
/// la source unique), teinte de ce qu'elle a avalé, ses arches à sa teinte ;
/// et ce qu'il reste de ses planètes autour d'elle.
fn naine(couleur: Vec3, autour: Autour) -> Systeme {
    let (rayon, lumi) = NAINE;
    let s = Variante::Normale.appliquer(Soleil::new(Vec3::ZERO, rayon, couleur, lumi), "Naine blanche").avec_arches_du_type();
    let masse = masse_luminosite(lumi);
    // L'anneau à distance de l'étoile, de 7 à 16 rayons : *« plus de
    // distance entre les anneaux de débris et l'astre »* (2026-10-26). Et
    // allégé : 1 800 cailloux, *« un peu trop de débris »*.
    let anneau = || DisqueConfig::debris_recents(600, rayon * 7.0, rayon * 16.0, masse);
    let s = match autour {
        Autour::Rien => s,
        Autour::Anneau => s.avec_disque(anneau()),
        Autour::Fragments => s.avec_disque(anneau().avec_fragments(2, rayon)),
        Autour::Pluie => s.avec_disque(anneau().avec_pluie(1400, rayon)),
        // Le gaz au ras de l'étoile, sur un disque à lui : l'anneau rocheux
        // reste au loin.
        Autour::Gaz => s
            .avec_disque(anneau())
            .avec_disque(DisqueConfig::debris_recents(0, rayon * 1.6, rayon * 2.0, masse).avec_bord_brulant(1.4)),
    };
    let mut sys = Systeme::new();
    sys.ajouter(Box::new(s));
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
}
