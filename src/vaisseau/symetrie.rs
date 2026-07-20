//! Opérations de symétrie pour l'assemblage de stations (voir
//! `docs/stations_fondations.md`, §4). Deux opérations seulement, façon Kerbal
//! Space Program : **miroir** (réflexion à travers un plan) et **radiale** (n
//! copies réparties autour d'un axe).
//!
//! Chaque opération produit la liste des transformations (`Mat4`, dans le repère
//! du parent) à appliquer à l'enfant de base pour obtenir toutes ses copies. La
//! première transformation est toujours l'identité : l'enfant d'origine fait
//! partie du groupe symétrique.

use macroquad::prelude::*;
use std::f32::consts::TAU;

const EPS: f32 = 1e-6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Symetrie {
    /// Original + sa réflexion à travers le plan de symétrie du parent.
    Miroir,
    /// `n` copies réparties tous les 360°/n autour de l'axe du parent.
    Radiale(u8),
}

impl Symetrie {
    /// Nombre **nominal** de copies. (Sur entrée dégénérée — axe ou normale nuls
    /// — [`transformations`](Self::transformations) peut en renvoyer moins.)
    pub fn nb_copies(self) -> usize {
        match self {
            Symetrie::Miroir => 2,
            Symetrie::Radiale(n) => (n as usize).max(1),
        }
    }

    /// Transformations à appliquer à l'enfant de base pour obtenir ses copies.
    /// La première est toujours l'identité. `axe` sert à la symétrie radiale,
    /// `normale` (du plan) au miroir ; le paramètre non pertinent est ignoré.
    pub fn transformations(self, axe: Vec3, normale: Vec3) -> Vec<Mat4> {
        match self {
            Symetrie::Miroir => {
                let n = normale.normalize_or_zero();
                if n.length_squared() < EPS {
                    return vec![Mat4::IDENTITY]; // plan dégénéré → une seule copie
                }
                vec![Mat4::IDENTITY, reflexion(n)]
            }
            Symetrie::Radiale(cnt) => {
                let cnt = (cnt as usize).max(1);
                let a = axe.normalize_or_zero();
                if a.length_squared() < EPS {
                    return vec![Mat4::IDENTITY]; // axe dégénéré → une seule copie
                }
                (0..cnt)
                    .map(|k| Mat4::from_axis_angle(a, TAU * k as f32 / cnt as f32))
                    .collect()
            }
        }
    }
}

/// Matrice de réflexion à travers le plan passant par l'origine, de normale
/// **unitaire** `n` : `R = I − 2·n·nᵀ`. Déterminant −1 (change la chiralité).
fn reflexion(n: Vec3) -> Mat4 {
    let m = Mat3::from_cols(
        vec3(1.0 - 2.0 * n.x * n.x, -2.0 * n.x * n.y, -2.0 * n.x * n.z),
        vec3(-2.0 * n.x * n.y, 1.0 - 2.0 * n.y * n.y, -2.0 * n.y * n.z),
        vec3(-2.0 * n.x * n.z, -2.0 * n.y * n.z, 1.0 - 2.0 * n.z * n.z),
    );
    Mat4::from_mat3(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proche(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    // 1. Nombre nominal de copies + bornage de la radiale.
    #[test]
    fn nb_copies_nominal() {
        assert_eq!(Symetrie::Miroir.nb_copies(), 2);
        assert_eq!(Symetrie::Radiale(4).nb_copies(), 4);
        assert_eq!(Symetrie::Radiale(1).nb_copies(), 1);
        assert_eq!(Symetrie::Radiale(0).nb_copies(), 1); // borné
    }

    // 2. Radiale(0) et (1) → une seule copie identité (pas de div/0).
    #[test]
    fn radiale_degeneree_une_copie() {
        for n in [0u8, 1] {
            let t = Symetrie::Radiale(n).transformations(Vec3::Y, Vec3::X);
            assert_eq!(t.len(), 1);
            assert_eq!(t[0], Mat4::IDENTITY);
        }
    }

    // 3. La première copie est toujours l'identité.
    #[test]
    fn premiere_copie_identite() {
        assert_eq!(Symetrie::Miroir.transformations(Vec3::Y, Vec3::X)[0], Mat4::IDENTITY);
        assert_eq!(Symetrie::Radiale(3).transformations(Vec3::Y, Vec3::X)[0], Mat4::IDENTITY);
    }

    // 4. Radiale répartit uniformément : centroïde des copies d'un point
    //    hors-axe = origine (invariant de symétrie).
    #[test]
    fn radiale_centroide_a_lorigine() {
        let base = vec3(1.0, 0.0, 0.0);
        for n in [2u8, 3, 4, 6] {
            let somme: Vec3 = Symetrie::Radiale(n)
                .transformations(Vec3::Y, Vec3::X)
                .iter()
                .map(|m| m.transform_point3(base))
                .sum();
            assert!(proche(somme, Vec3::ZERO), "n={n}, somme={somme:?}");
        }
    }

    // 5. Radiale(2) = demi-tour : le point (1,0,0) → (-1,0,0).
    #[test]
    fn radiale_deux_demi_tour() {
        let t = Symetrie::Radiale(2).transformations(Vec3::Y, Vec3::X);
        assert!(proche(t[1].transform_point3(vec3(1.0, 0.0, 0.0)), vec3(-1.0, 0.0, 0.0)));
    }

    // 6. Radiale conserve la norme (rotation propre, det +1).
    #[test]
    fn radiale_conserve_norme_et_orientation() {
        let base = vec3(1.0, 0.5, -0.3);
        for m in Symetrie::Radiale(5).transformations(Vec3::Y, Vec3::X) {
            assert!((m.transform_point3(base).length() - base.length()).abs() < 1e-5);
            assert!((m.determinant() - 1.0).abs() < 1e-5);
        }
    }

    // 7. Miroir réfléchit à travers le plan (normale X) : x change de signe.
    #[test]
    fn miroir_reflechit() {
        let t = Symetrie::Miroir.transformations(Vec3::Y, Vec3::X);
        assert!(proche(t[1].transform_point3(vec3(1.0, 2.0, 3.0)), vec3(-1.0, 2.0, 3.0)));
    }

    // 8. Un point situé sur le plan de symétrie reste inchangé.
    #[test]
    fn miroir_point_sur_le_plan_inchange() {
        let t = Symetrie::Miroir.transformations(Vec3::Y, Vec3::X);
        let p = vec3(0.0, 4.0, -2.0); // x = 0 → sur le plan de normale X
        assert!(proche(t[1].transform_point3(p), p));
    }

    // 9. Le miroir change la chiralité : déterminant -1.
    #[test]
    fn miroir_determinant_negatif() {
        let t = Symetrie::Miroir.transformations(Vec3::Y, Vec3::X);
        assert!((t[1].determinant() + 1.0).abs() < 1e-5);
    }

    // 10. Normale dégénérée → une seule copie identité (pas de miroir bancal).
    #[test]
    fn miroir_normale_degeneree_une_copie() {
        let t = Symetrie::Miroir.transformations(Vec3::Y, Vec3::ZERO);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0], Mat4::IDENTITY);
    }

    // 11. Axe dégénéré → une seule copie identité (évite le NaN de from_axis_angle).
    #[test]
    fn radiale_axe_degenere_une_copie() {
        let t = Symetrie::Radiale(4).transformations(Vec3::ZERO, Vec3::X);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0], Mat4::IDENTITY);
    }

    // 12. Normale non normalisée : la réflexion la normalise (résultat correct).
    #[test]
    fn miroir_normale_non_unitaire() {
        let t = Symetrie::Miroir.transformations(Vec3::Y, vec3(5.0, 0.0, 0.0));
        assert!(proche(t[1].transform_point3(vec3(1.0, 2.0, 3.0)), vec3(-1.0, 2.0, 3.0)));
    }
}
