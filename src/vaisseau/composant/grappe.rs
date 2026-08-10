//! **Grappe de réservoirs** : des cuves sphériques groupées en triangle, sur
//! plusieurs rangées empilées le long de l'axe.
//!
//! Le parc n'avait que le réservoir **unitaire** ([`Composant::Reservoir`]), et
//! une soute d'ergols se construisait donc en posant les cuves une par une —
//! autant de pièces, autant de ports consommés, et une grappe qu'aucune
//! opération ne traitait comme un tout. La grappe est **une** pièce : elle se
//! pose, se déplace et se retire d'un bloc.
//!
//! ## Trois par rangée, et pourquoi
//!
//! Trois cuves à 120° est le plus petit groupe qui **enserre un axe** : deux
//! laisseraient l'ensemble plat et sans centre, quatre commencent à gaspiller le
//! volume entre les cuves. C'est la disposition des grappes d'ergols réelles,
//! pour la même raison — trois points suffisent à caler un cylindre.
//!
//! ## Les cuves se touchent, et c'est voulu
//!
//! Le rayon de disposition est calculé pour que **deux cuves voisines d'une même
//! rangée soient tangentes** : c'est ce qui fait une grappe et non trois
//! réservoirs voisins. Le calcul est dans [`rayon_disposition`], pas écrit à la
//! main — le jour où le nombre par rangée change, la tangence suit.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::pieces;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

/// Cuves par rangée. Voir l'en-tête : trois est le plus petit groupe qui enserre
/// un axe.
pub const PAR_RANGEE: usize = 3;

/// Distance de l'axe au centre d'une cuve, pour que deux voisines de la même
/// rangée se **touchent**.
///
/// Les `n` centres sont sur un cercle de rayon `d` ; deux voisins y sont
/// distants de `2·d·sin(π/n)`, et l'on veut que ça vaille `2·r`. D'où
/// `d = r / sin(π/n)`.
pub fn rayon_disposition(rayon_cuve: f32) -> f32 {
    rayon_cuve / (PI / PAR_RANGEE as f32).sin()
}

/// Rayon hors-tout d'une rangée : l'axe jusqu'à la peau extérieure d'une cuve.
pub fn rayon_hors_tout(rayon_cuve: f32) -> f32 {
    rayon_disposition(rayon_cuve) + rayon_cuve
}

/// Hauteur totale d'une grappe de `rangees` rangées.
///
/// Les rangées sont espacées d'un **diamètre** : elles se touchent verticalement
/// comme les cuves d'une même rangée se touchent latéralement.
pub fn hauteur(rayon_cuve: f32, rangees: usize) -> f32 {
    2.0 * rayon_cuve * rangees.max(1) as f32
}

/// Centre de la cuve `i` de la rangée `k`, en repère local.
///
/// La grappe est déployée le long de **+Z** depuis son port de montage, comme
/// tout module axial. Une rangée sur deux est **tournée d'un demi-pas**, pour
/// que les cuves se nichent dans les creux de la rangée du dessous au lieu de
/// s'empiler en colonnes.
///
/// ⚠️ **C'est un choix de lecture, et rien de plus** : le red-check l'a établi.
/// Les rangées étant espacées d'un diamètre, l'empilement en colonnes garde les
/// cuves tangentes tout autant — supprimer le décalage ne rougit **aucun** test,
/// et c'est normal. Une première rédaction prétendait que ça « serrait le
/// volume » ; c'est faux, la densité est la même. Aucune garde ne le protège
/// donc, et il ne faut pas en inventer une : on ne teste pas un goût.
fn centre_cuve(rayon_cuve: f32, k: usize, i: usize) -> Vec3 {
    let d = rayon_disposition(rayon_cuve);
    let decalage = if k % 2 == 1 { PI / PAR_RANGEE as f32 } else { 0.0 };
    let a = TAU * i as f32 / PAR_RANGEE as f32 + decalage;
    let z = rayon_cuve * (2 * k + 1) as f32;
    vec3(d * a.cos(), d * a.sin(), z)
}

/// Deux écoutilles axiales, comme un module : la grappe se met **en ligne** dans
/// une pile, elle n'est pas un appendice qu'on accroche sur un flanc.
pub(super) fn ports(profil: Profil, rayon_cuve: f32, rangees: usize) -> Vec<Port> {
    let h = hauteur(rayon_cuve, rangees);
    vec![
        Port::new(Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, h), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn dessiner<P: Peintre>(p: &mut P, rayon_cuve: f32, rangees: usize) {
    let corps = Color::new(0.82, 0.84, 0.88, 1.0); // alu clair (tuiles)
    let joint = Color::new(0.30, 0.31, 0.34, 1.0); // sous-couche (joints)

    // **Rien que les cuves.**
    //
    // ⚠️ Une première version ajoutait une âme centrale, des colliers de
    // fixation vers l'axe et un cerclage par rangée — de la structure destinée à
    // « expliquer ce qui relie les cuves ». À l'écran, ça brouillait tout : la
    // grappe ne ressemblait plus au réservoir unitaire dont elle est censée être
    // le pluriel, mais à un assemblage indistinct. Le jugement est venu de
    // l'œil, et il est sans appel — neuf sphères propres se lisent
    // immédiatement, neuf sphères enrubannées non.
    //
    // Le traitement est donc **exactement** celui de `reservoir.rs` : une
    // sous-couche sombre pour que les joints ressortent, puis l'icosphère
    // facettée par-dessus, à la même subdivision. Deux pièces de la même famille
    // doivent se peindre pareil, sinon leur parenté ne se voit pas.
    for k in 0..rangees.max(1) {
        for i in 0..PAR_RANGEE {
            let c = centre_cuve(rayon_cuve, k, i);
            p.sphere(c, rayon_cuve * 0.97, joint);
            pieces::sphere_triangulee(p, c, rayon_cuve, 3, 0.16, corps);
        }
    }
}

/// Sphère englobante : le rayon hors-tout dans le plan, la hauteur le long de Z.
pub(super) fn rayon_local(rayon_cuve: f32, rangees: usize) -> f32 {
    let h = hauteur(rayon_cuve, rangees);
    rayon_hors_tout(rayon_cuve).hypot(h * 0.5) + h * 0.5
}

pub(super) fn cout(rayon_cuve: f32, rangees: usize) -> f32 {
    // Le coût suit le **volume** d'ergols emporté, donc le cube du rayon et le
    // nombre de cuves. Un coût linéaire ferait passer une grappe de grosses
    // cuves pour une grappe de petites.
    let cuves = (PAR_RANGEE * rangees.max(1)) as f32;
    2.0 * cuves * rayon_cuve.powi(3).max(0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Deux cuves voisines d'une rangée se touchent.** C'est ce qui fait une
    /// grappe et non trois réservoirs côte à côte, et c'est la seule cote que
    /// la disposition ait à respecter.
    #[test]
    fn les_cuves_dune_rangee_sont_tangentes() {
        for r in [0.4_f32, 1.0, 3.5] {
            for i in 0..PAR_RANGEE {
                let a = centre_cuve(r, 0, i);
                let b = centre_cuve(r, 0, (i + 1) % PAR_RANGEE);
                let d = a.truncate().distance(b.truncate());
                assert!(
                    (d - 2.0 * r).abs() < 1e-4,
                    "rayon {r} : voisines à {d} au lieu de {}",
                    2.0 * r
                );
            }
        }
    }

    /// **Aucune cuve n'en traverse une autre**, y compris entre rangées — le
    /// décalage d'une demi-rangée niche les cuves sans les faire mordre.
    #[test]
    fn aucune_cuve_nen_traverse_une_autre() {
        let r = 1.0_f32;
        let mut centres = Vec::new();
        for k in 0..3 {
            for i in 0..PAR_RANGEE {
                centres.push(centre_cuve(r, k, i));
            }
        }
        for (a, x) in centres.iter().enumerate() {
            for y in centres.iter().skip(a + 1) {
                let d = x.distance(*y);
                assert!(
                    d > 2.0 * r - 1e-3,
                    "deux cuves se traversent : distance {d} pour un diamètre de {}",
                    2.0 * r
                );
            }
        }
    }

    /// **Les rangées s'empilent sans trou ni recouvrement** : la grappe mesure
    /// exactement un diamètre par rangée, et la dernière cuve affleure le bout.
    #[test]
    fn les_rangees_remplissent_la_hauteur() {
        let r = 1.0_f32;
        for n in [1_usize, 2, 3, 6] {
            let h = hauteur(r, n);
            assert!((h - 2.0 * r * n as f32).abs() < 1e-5);
            // La dernière rangée touche le bout, la première touche le pied.
            let premiere = centre_cuve(r, 0, 0).z;
            let derniere = centre_cuve(r, n - 1, 0).z;
            assert!((premiere - r).abs() < 1e-5, "la première rangée ne part pas du pied");
            assert!((derniere + r - h).abs() < 1e-5, "la dernière rangée n'atteint pas le bout");
        }
    }

    /// **Les deux écoutilles sont aux deux bouts**, et regardent dehors : la
    /// grappe se met en ligne dans une pile.
    #[test]
    fn les_ecoutilles_sont_aux_deux_bouts() {
        let (r, n) = (1.0_f32, 3);
        let ports = ports(Profil::P1, r, n);
        assert_eq!(ports.len(), 2);
        assert!((ports[0].repere.pos.z).abs() < 1e-5, "l'écoutille basse n'est pas au pied");
        assert!(
            (ports[1].repere.pos.z - hauteur(r, n)).abs() < 1e-5,
            "l'écoutille haute n'est pas au bout"
        );
        // Elles s'opposent : l'une regarde −Z, l'autre +Z.
        assert!(ports[0].repere.avant().dot(ports[1].repere.avant()) < -0.99);
    }

    /// **La grappe se peint comme le réservoir unitaire dont elle est le
    /// pluriel** : sous-couche sombre, puis icosphère facettée à la même
    /// subdivision. Deux pièces de la même famille doivent se peindre pareil,
    /// sinon leur parenté ne se voit pas — c'est ce que l'œil a reproché à une
    /// première version chargée de structure.
    ///
    /// Le test compte ce que reçoit le peintre : **deux sphères par cuve**, et
    /// rien d'autre. Toute structure rajoutée (âme, colliers, cerclages) émet
    /// des cylindres, et le compte s'en aperçoit.
    #[test]
    fn la_grappe_ne_peint_que_ses_cuves() {
        #[derive(Default)]
        struct Compteur {
            spheres: usize,
            autres: usize,
        }
        impl Peintre for Compteur {
            fn cylindre(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {
                self.autres += 1;
            }
            fn cone(&mut self, _: Vec3, _: Vec3, _: f32, _: f32, _: f32, _: Color) {
                self.autres += 1;
            }
            fn sphere(&mut self, _: Vec3, _: f32, _: Color) {
                self.spheres += 1;
            }
            fn cube(&mut self, _: Vec3, _: Vec3, _: Color) {
                self.autres += 1;
            }
            fn panneau(&mut self, _: Vec3, _: Vec3, _: Vec3, _: Color) {
                self.autres += 1;
            }
            fn ligne(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {
                self.autres += 1;
            }
            // L'icosphère facettée sort en triangles bruts : c'est la seconde
            // moitié du traitement, pas de la structure ajoutée.
            fn triangles(&mut self, _: &[Vec3], _: &[u16], _: Color) {}
            fn empiler_transforme(&mut self, _: Mat4) {}
            fn depiler_transforme(&mut self) {}
        }

        for rangees in [1_usize, 3] {
            let mut c = Compteur::default();
            dessiner(&mut c, 1.0, rangees);
            let cuves = PAR_RANGEE * rangees;
            assert_eq!(
                c.spheres, cuves,
                "{rangees} rangées : {} sous-couches pour {cuves} cuves",
                c.spheres
            );
            assert_eq!(
                c.autres, 0,
                "{rangees} rangées : {} primitives de structure — la grappe s'est rechargée",
                c.autres
            );
        }
    }

    /// Le rayon déclaré contient toutes les cuves, peau comprise.
    #[test]
    fn le_rayon_local_contient_les_cuves() {
        let (r, n) = (1.0_f32, 3);
        let borne = rayon_local(r, n);
        for k in 0..n {
            for i in 0..PAR_RANGEE {
                let d = centre_cuve(r, k, i).length() + r;
                assert!(d <= borne + 1e-4, "cuve à {d} hors du rayon {borne}");
            }
        }
    }

    /// **Le coût suit le volume**, pas le nombre de pièces : doubler le rayon
    /// multiplie par huit, ajouter une rangée par trois demis.
    #[test]
    fn le_cout_suit_le_volume() {
        let a = cout(1.0, 3);
        assert!((cout(2.0, 3) / a - 8.0).abs() < 1e-3, "doubler le rayon ne fait pas ×8");
        assert!((cout(1.0, 6) / a - 2.0).abs() < 1e-3, "doubler les rangées ne fait pas ×2");
    }
}
