//! **Hangar** : un module d'habitat prolongé d'une **baie à section hexagonale**
//! qui abrite les engins.
//!
//! Le parc savait faire des cylindres (modules, réservoirs) et des caissons
//! techniques ([`Composant::Caisson`], la seule géométrie anguleuse), mais rien
//! qui **abrite** — pas de volume creux avec une ouverture par où sort un
//! vaisseau. Or c'est ce qu'il faut pour poser un Ranger sur une Endurance, ou
//! une navette sur un tore.
//!
//! ## Pourquoi un cylindre **et** une baie
//!
//! Un hangar tout cylindrique perdrait ses coins : un engin est plus large que
//! haut, et le volume utile d'un tube est bien plus petit que son encombrement.
//! Une baie seule, à l'inverse, ne se raccorde à rien — tout le reste du parc
//! est de révolution, et un caisson nu posé sur une écoutille se lit comme une
//! caisse sanglée, pas comme une partie du vaisseau.
//!
//! Le hangar est donc les deux : un **fût** qui reprend le vocabulaire des
//! modules et donne l'accroche, puis une **baie** qui donne le volume et la
//! porte.
//!
//! ## Pourquoi un hexagone et pas une boîte
//!
//! La baie était d'abord un parallélépipède. Deux défauts :
//!
//! 1. **Le vocabulaire.** L'hexagone est déjà partout dans le parc — treillis,
//!    tours, pales, ceinture des nacelles. Une boîte à angles droits est la
//!    seule forme du vaisseau qui ne rime avec rien d'autre, et elle se lit
//!    comme un colis arrimé plutôt que comme une structure.
//! 2. **Le raccord.** Aucun évasement de révolution ne rejoint proprement un
//!    rectangle ; l'ancien tronc conique débordait en haut et en bas de la
//!    silhouette. Une section hexagonale, elle, se raccorde au fût par une
//!    homothétie d'elle-même (voir [`echelle_pied`]).
//!
//! L'hexagone est **allongé** : ses deux côtés parallèles à x valent
//! [`ALLONGE`] fois les quatre obliques. C'est ce qui en fait un *hangar* et
//! non un tuyau à six pans — le volume part en largeur, là où sont les ailes.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::PI;

/// Variantes de [`Composant::Hangar`].
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VarianteHangar {
    /// Baie **ouverte** : la porte est béante, on voit le vide à l'intérieur.
    /// C'est l'état d'un hangar en service.
    Ouvert,
    /// Baie **fermée** : deux vantaux joints, liseré de joint au milieu.
    Ferme,
}

impl VarianteHangar {
    #[allow(dead_code)] // Même limite que les autres familles : la palette ne
    // liste qu'une sous-variante par composant (voir `module_arc.rs`).
    pub const TOUS: [VarianteHangar; 2] = [VarianteHangar::Ouvert, VarianteHangar::Ferme];

    #[allow(dead_code)] // voir la note sur TOUS
    pub fn nom(self) -> &'static str {
        match self {
            VarianteHangar::Ouvert => "HANGAR OUVERT",
            VarianteHangar::Ferme => "HANGAR FERME",
        }
    }

    pub(super) fn cout(self) -> f32 {
        match self {
            VarianteHangar::Ouvert => 30.0,
            VarianteHangar::Ferme => 32.0,
        }
    }
}

/// **Allongement** de la section : les deux côtés parallèles à x valent ce
/// multiple des quatre côtés obliques.
///
/// À 1 on aurait l'hexagone régulier — un tuyau à six pans, aussi haut que
/// large, qui n'abrite rien de mieux qu'un cylindre. C'est ce facteur, et lui
/// seul, qui fait la différence entre une section de coque et une **baie**.
pub const ALLONGE: f32 = 2.0;

/// Demi-hauteur de la baie : la moitié de la distance entre les deux côtés
/// plats. C'est la cote **maîtresse** — tout le reste de la section en dérive
/// par la géométrie de l'hexagone, pas par des facteurs écrits à la main.
pub fn demi_hauteur(profil: Profil) -> f32 {
    profil.rayon() * 1.05
}

/// Longueur d'un côté **oblique**. Dans un hexagone à côtés plats horizontaux,
/// les obliques montent à 60°, donc la demi-hauteur vaut `côté·√3/2`.
fn cote(profil: Profil) -> f32 {
    demi_hauteur(profil) * 2.0 / 3.0_f32.sqrt()
}

/// Demi-longueur du côté **plat** (celui qui est parallèle à x). C'est la cote
/// utile dès qu'on pose quelque chose sur le toit : au-delà, on est sur la
/// pente et plus sur le plat.
pub fn demi_plat(profil: Profil) -> f32 {
    cote(profil) * ALLONGE * 0.5
}

/// Demi-largeur hors-tout : du centre à la pointe latérale.
///
/// **Plus large que le fût**, et c'est tout l'intérêt : une baie au gabarit du
/// cylindre n'abriterait rien de plus qu'un module ordinaire.
pub fn demi_largeur(profil: Profil) -> f32 {
    demi_plat(profil) + cote(profil) * 0.5
}

/// Les six sommets de la section, à la hauteur `z` et à l'échelle `echelle`,
/// dans l'ordre **trigonométrique** vu depuis +Z (l'enroulement dont dépend
/// l'orientation des faces du prisme).
///
/// Ordre : pointe droite, coin haut-droit, coin haut-gauche, pointe gauche,
/// coin bas-gauche, coin bas-droit.
pub fn section(profil: Profil, z: f32, echelle: f32) -> [Vec3; 6] {
    let a = demi_plat(profil) * echelle;
    let hl = demi_largeur(profil) * echelle;
    let hh = demi_hauteur(profil) * echelle;
    [
        vec3(hl, 0.0, z),
        vec3(a, hh, z),
        vec3(-a, hh, z),
        vec3(-hl, 0.0, z),
        vec3(-a, -hh, z),
        vec3(a, -hh, z),
    ]
}

/// Rayon circonscrit de la section : le sommet le plus loin de l'axe.
///
/// Sert aux enveloppes. C'est **moins** que la demi-diagonale de la boîte
/// englobante (l'hexagone n'a pas de coin en `(demi_largeur, demi_hauteur)`),
/// et réserver ce coin-là interdirait de poser une pièce dans un vide réel.
pub fn rayon_section(profil: Profil) -> f32 {
    demi_largeur(profil).max(demi_plat(profil).hypot(demi_hauteur(profil)))
}

/// Aire de la section : un rectangle central flanqué de deux triangles de
/// pointe. C'est le volume *réellement* abrité, à quoi s'indexe le coût — la
/// boîte englobante facturerait un cinquième de vide en plus.
pub fn aire_section(profil: Profil) -> f32 {
    let (a, hl, hh) = (demi_plat(profil), demi_largeur(profil), demi_hauteur(profil));
    4.0 * a * hh + 2.0 * hh * (hl - a)
}

/// Échelle de la section au **pied** de l'évasement, celle qui l'inscrit
/// exactement dans le cercle du fût.
///
/// Le raccord rond → hexagone est le point où l'ancienne boîte échouait : un
/// tronc de cône assez large pour rejoindre les pointes latérales dépassait de
/// la silhouette en haut et en bas. Ici l'évasement est l'hexagone lui-même,
/// réduit jusqu'à tenir dans le fût, donc rien ne peut déborder.
pub fn echelle_pied(profil: Profil) -> f32 {
    profil.rayon() / demi_largeur(profil)
}

/// Longueur de l'évasement, en fraction de la profondeur de baie.
const EVASEMENT: f32 = 0.12;

/// Enfoncement de l'évasement **dans** la baie, et retrait d'échelle qui va
/// avec. Sans eux, la face haute de l'évasement et la face basse de la baie se
/// disputeraient le plan `z = fut` et clignoteraient l'une à travers l'autre —
/// même piège, même remède, que les collerettes de `pieces::nacelle_cargo`.
const NOYAGE: f32 = 0.03;
const RETRAIT: f32 = 0.99;

/// Ports : l'écoutille de montage au pied du fût, et un port de **surface** sur
/// le toit de la baie.
///
/// Le second n'est pas un ornement : un hangar porte des antennes, un mât, un
/// projecteur d'approche. Sans lui, la plus grosse pièce plate du vaisseau
/// n'offrirait aucune prise. Il est posé sur le **plat** du toit, pas sur une
/// pente — d'où `demi_hauteur` et non le rayon hors-tout.
pub(super) fn ports(profil: Profil, fut: f32, baie: f32) -> Vec<Port> {
    vec![
        Port::new(Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
        Port::new(
            Repere::new(
                vec3(0.0, demi_hauteur(profil), fut + baie * 0.5),
                Quat::from_rotation_x(-PI / 2.0),
            ),
            GenrePort::Surface,
            Profil::P0,
        ),
    ]
}

/// Prisme fermé tendu entre deux sections de six sommets.
///
/// Aucune primitive du [`Peintre`] n'a de section non circulaire : comme
/// `pieces::prisme_onigiri`, on sort des triangles bruts. Les deux anneaux
/// doivent être donnés dans le même ordre trigonométrique, sans quoi les
/// flancs se croisent.
fn prisme<P: Peintre>(p: &mut P, bas: [Vec3; 6], haut: [Vec3; 6], couleur: Color) {
    const N: u16 = 6;
    let mut sommets = bas.to_vec();
    sommets.extend_from_slice(&haut);
    sommets.push(bas.iter().sum::<Vec3>() / N as f32); // centre du fond
    sommets.push(haut.iter().sum::<Vec3>() / N as f32); // centre du dessus
    let (cb, ch) = (2 * N, 2 * N + 1);

    let mut indices = Vec::with_capacity(N as usize * 12);
    for k in 0..N {
        let kn = (k + 1) % N;
        // Flanc (deux triangles), puis les deux embouts en éventail.
        indices.extend_from_slice(&[k, k + N, kn + N, k, kn + N, kn]);
        indices.extend_from_slice(&[cb, kn, k]);
        indices.extend_from_slice(&[ch, k + N, kn + N]);
    }
    p.triangles(&sommets, &indices, couleur);
}

/// Les dix-huit arêtes du prisme, au trait. C'est ce qui donne à la baie ses
/// pans : peinte d'une seule couleur, une section hexagonale sans arêtes se
/// lit comme un tube écrasé.
fn aretes<P: Peintre>(p: &mut P, profil: Profil, z0: f32, z1: f32, couleur: Color) {
    let (bas, haut) = (section(profil, z0, 1.0), section(profil, z1, 1.0));
    let ep = (demi_hauteur(profil) * 0.05).max(0.01);
    for k in 0..6 {
        let kn = (k + 1) % 6;
        p.ligne(bas[k], bas[kn], ep, couleur);
        p.ligne(haut[k], haut[kn], ep, couleur);
        p.ligne(bas[k], haut[k], ep, couleur);
    }
}

pub(super) fn dessiner<P: Peintre>(
    p: &mut P,
    variante: VarianteHangar,
    profil: Profil,
    fut: f32,
    baie: f32,
) {
    let coque = Color::new(0.78, 0.80, 0.84, 1.0);
    let sombre = Color::new(0.20, 0.22, 0.26, 1.0);
    let metal = Color::new(0.55, 0.58, 0.63, 1.0);
    let interieur = Color::new(0.10, 0.11, 0.14, 1.0);
    let hh = demi_hauteur(profil);
    let plat = demi_plat(profil);
    let r = profil.rayon();

    // --- Le fût : le vocabulaire des modules, et l'accroche ---
    p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, fut), r, coque);
    // Cerclages, comme sur un module pressurisé.
    for k in 1..=2 {
        let z = fut * k as f32 / 3.0;
        p.cylindre(vec3(0.0, 0.0, z - fut * 0.012), vec3(0.0, 0.0, z + fut * 0.012), r * 1.04, metal);
    }

    // --- Raccord fût → baie : l'hexagone lui-même, réduit jusqu'au fût ---
    prisme(
        p,
        section(profil, fut - baie * EVASEMENT, echelle_pied(profil)),
        section(profil, fut + baie * NOYAGE, RETRAIT),
        metal,
    );

    // --- La baie : le volume qui abrite ---
    prisme(p, section(profil, fut, 1.0), section(profil, fut + baie, 1.0), coque);
    aretes(p, profil, fut, fut + baie, sombre);

    // --- La porte, au bout de la baie ---
    let porte = fut + baie;
    match variante {
        VarianteHangar::Ouvert => {
            // Béante : on peint le **fond** de la baie en sombre, et l'ouverture
            // encadrée. C'est le noir au fond qui fait lire un creux — une face
            // claire donnerait une caisse pleine.
            prisme(
                p,
                section(profil, porte - baie * 0.07, 0.90),
                section(profil, porte - baie * 0.05, 0.90),
                interieur,
            );
            // Encadrement : un jonc suivant les six arêtes de la bouche. Six
            // barres droites vaudraient pour un rectangle, pas pour l'hexagone.
            let bouche = section(profil, porte - baie * 0.01, 0.97);
            for k in 0..6 {
                p.cylindre(bouche[k], bouche[(k + 1) % 6], hh * 0.07, metal);
            }
        }
        VarianteHangar::Ferme => {
            // Un tablier plein sur toute la section, fendu au milieu : c'est la
            // fente verticale, et elle seule, qui dit que ça s'ouvre.
            prisme(
                p,
                section(profil, porte - baie * 0.05, 0.97),
                section(profil, porte - baie * 0.01, 0.97),
                coque,
            );
            p.cylindre(
                vec3(0.0, -hh * 0.95, porte),
                vec3(0.0, hh * 0.95, porte),
                hh * 0.05,
                sombre,
            );
        }
    }

    // --- Rails d'approche sur le toit : ce qui dit le sens de la manœuvre ---
    // Cotés sur le **plat**, pas sur la demi-largeur : au-delà on est sur la
    // pente et le rail flotterait au-dessus du vide.
    for s in [-1.0_f32, 1.0] {
        p.cylindre(
            vec3(plat * 0.62 * s, hh, fut + baie * 0.10),
            vec3(plat * 0.62 * s, hh, fut + baie * 0.92),
            hh * 0.05,
            metal,
        );
    }
}

/// Sphère englobante : la pointe latérale de la baie, au fond, est le point le
/// plus lointain.
pub(super) fn rayon_local(profil: Profil, fut: f32, baie: f32) -> f32 {
    (fut + baie).hypot(rayon_section(profil))
}

pub(super) fn cout(variante: VarianteHangar, profil: Profil, baie: f32) -> f32 {
    // Le coût suit le **volume abrité** : c'est ce qu'on achète.
    let v = aire_section(profil) * baie;
    variante.cout() * (v / 8.0).max(0.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La baie est plus large que le fût**, sinon elle n'abrite rien de plus
    /// qu'un module ordinaire et le composant n'a pas lieu d'être.
    #[test]
    fn la_baie_deborde_le_fut() {
        for profil in Profil::TOUS {
            let (hl, hh) = (demi_largeur(profil), demi_hauteur(profil));
            assert!(hl > profil.rayon(), "{:?} : baie pas plus large que le fût", profil);
            assert!(hh > profil.rayon() * 0.9, "{:?} : baie écrasée", profil);
            // Plus large que haute : un engin l'est aussi.
            assert!(hl > hh, "{:?} : baie plus haute que large", profil);
        }
    }

    /// **La section est bien un hexagone allongé** : six sommets, deux côtés
    /// parallèles à x qui valent [`ALLONGE`] fois chacun des quatre obliques,
    /// et un contour convexe.
    ///
    /// C'est la demande elle-même. Sans ce test, changer `demi_plat` d'un
    /// facteur laisserait une forme à six pans qui n'est plus l'hexagone
    /// voulu, et rien ne le dirait.
    #[test]
    fn la_section_est_un_hexagone_allonge() {
        for profil in Profil::TOUS {
            let s = section(profil, 0.0, 1.0);
            // Les côtés : [1]→[2] et [4]→[5] sont les plats, parallèles à x.
            let long = |a: Vec3, b: Vec3| (b - a).length();
            let plats = [long(s[1], s[2]), long(s[4], s[5])];
            let obliques = [long(s[0], s[1]), long(s[2], s[3]), long(s[3], s[4]), long(s[5], s[0])];
            for c in plats {
                for o in obliques {
                    assert!(
                        (c / o - ALLONGE).abs() < 1e-3,
                        "{:?} : plat {c} contre oblique {o}, rapport {}",
                        profil,
                        c / o
                    );
                }
            }
            // Les plats sont bien parallèles à x.
            assert!((s[2] - s[1]).y.abs() < 1e-5, "{:?} : le côté haut n'est pas parallèle à x", profil);
            assert!((s[5] - s[4]).y.abs() < 1e-5, "{:?} : le côté bas n'est pas parallèle à x", profil);
            // Convexe : tous les produits vectoriels consécutifs tournent dans
            // le même sens (et non nul — pas de sommet dégénéré).
            for k in 0..6 {
                let (a, b, c) = (s[k], s[(k + 1) % 6], s[(k + 2) % 6]);
                let z = (b - a).x * (c - b).y - (b - a).y * (c - b).x;
                assert!(z > 1e-6, "{:?} : sommet {k} rentrant ou plat (z = {z})", profil);
            }
        }
    }

    /// **Le pied de l'évasement tient dans le fût.** C'est le défaut que
    /// l'hexagone corrige : l'ancien tronc de cône se raccordait à la largeur
    /// de la baie, donc débordait de la silhouette en haut et en bas. Ici la
    /// section réduite doit être *entièrement* dans le cercle du fût.
    #[test]
    fn le_pied_de_l_evasement_tient_dans_le_fut() {
        for profil in Profil::TOUS {
            let e = echelle_pied(profil);
            for (k, v) in section(profil, 0.0, e).iter().enumerate() {
                let d = v.truncate().length();
                assert!(
                    d <= profil.rayon() + 1e-4,
                    "{:?} : sommet {k} à {d} déborde le fût de rayon {}",
                    profil,
                    profil.rayon()
                );
            }
            // Et il touche vraiment le fût, sinon l'évasement flotte dedans.
            assert!(
                (demi_largeur(profil) * e - profil.rayon()).abs() < 1e-4,
                "{:?} : l'évasement n'atteint pas la paroi du fût",
                profil
            );
        }
    }

    /// **Les deux ports servent à deux choses**, et se distinguent : l'écoutille
    /// axiale accroche le hangar au vaisseau, le port de surface offre le toit
    /// de la baie. Sans le second, la plus grosse pièce plate du vaisseau
    /// n'offrirait aucune prise.
    #[test]
    fn le_hangar_offre_une_ecoutille_et_un_toit() {
        let (fut, baie) = (3.0_f32, 5.0);
        let ports = ports(Profil::P2, fut, baie);
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].genre, GenrePort::ModuleAxial);
        assert_eq!(ports[1].genre, GenrePort::Surface);
        // L'écoutille est au pied et regarde en arrière.
        assert!(ports[0].repere.pos.length() < 1e-5);
        assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
        // Le toit est en hauteur, et sa normale monte.
        assert!(ports[1].repere.pos.y > 0.0, "le port de toit n'est pas sur le toit");
        assert!(
            ports[1].repere.avant().dot(Vec3::Y) > 0.99,
            "le port de toit ne regarde pas vers le haut : {:?}",
            ports[1].repere.avant()
        );
        // Et il est sur le **plat**, pas sur une pente : sinon ce qu'on y pose
        // s'enfonce dans la coque d'un côté et flotte de l'autre.
        assert!(
            ports[1].repere.pos.x.abs() < demi_plat(Profil::P2),
            "le port de toit est sur la pente"
        );
    }

    /// L'enveloppe contient la baie, pointes comprises — c'est elle le point le
    /// plus lointain, pas le bout du fût.
    #[test]
    fn le_rayon_local_contient_la_baie() {
        for profil in Profil::TOUS {
            for (fut, baie) in [(2.0_f32, 4.0_f32), (6.0, 2.0), (1.0, 9.0)] {
                let borne = rayon_local(profil, fut, baie);
                for (k, v) in section(profil, fut + baie, 1.0).iter().enumerate() {
                    assert!(
                        v.length() <= borne + 1e-4,
                        "{:?} fût {fut} baie {baie} : sommet {k} à {} hors du rayon {borne}",
                        profil,
                        v.length()
                    );
                }
            }
        }
    }

    /// **Le coût suit le volume abrité**, pas la longueur : c'est le volume
    /// qu'on achète, et une baie deux fois plus profonde coûte deux fois plus.
    #[test]
    fn le_cout_suit_le_volume_abrite() {
        let a = cout(VarianteHangar::Ouvert, Profil::P2, 4.0);
        let b = cout(VarianteHangar::Ouvert, Profil::P2, 8.0);
        assert!((b / a - 2.0).abs() < 1e-3, "doubler la baie ne double pas le coût : {}", b / a);
        // Et un profil plus gros coûte plus, à baie égale.
        assert!(
            cout(VarianteHangar::Ouvert, Profil::P3, 4.0) > a,
            "un hangar plus large ne coûte pas plus"
        );
        // L'aire facturée est celle de l'hexagone, pas de la boîte englobante :
        // sinon on paie un vide qu'on n'abrite pas.
        for profil in Profil::TOUS {
            let boite = 4.0 * demi_largeur(profil) * demi_hauteur(profil);
            assert!(
                aire_section(profil) < boite * 0.95,
                "{:?} : l'aire facturée ne distingue pas l'hexagone de sa boîte",
                profil
            );
        }
    }

    /// Les deux variantes se nomment distinctement et coûtent quelque chose.
    #[test]
    fn les_variantes_sont_nommees_et_facturees() {
        let mut noms: Vec<&str> = VarianteHangar::TOUS.iter().map(|v| v.nom()).collect();
        noms.sort_unstable();
        let avant = noms.len();
        noms.dedup();
        assert_eq!(noms.len(), avant, "deux variantes de hangar portent le même nom");
        for v in VarianteHangar::TOUS {
            assert!(v.cout() > 0.0, "{} est gratuit", v.nom());
        }
    }
}
