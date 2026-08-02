//! Désignation à la souris pour l'assembleur : quel **port libre** et quelle
//! **pièce** sont sous le curseur (`docs/conception/assembleur.md` §8.3).
//!
//! Les deux moitiés ne se traitent pas de la même façon, et c'est voulu :
//!
//! - un **port** est un point sans épaisseur. Le viser en 3D demanderait de lui
//!   inventer un volume — donc une deuxième cote à tenir d'accord avec le
//!   marqueur dessiné. On le vise donc **en espace écran**, à la distance en
//!   pixels du curseur : c'est littéralement « ce que je vois est ce que je
//!   clique », et la tolérance s'exprime dans la seule unité où l'utilisateur
//!   la perçoit ;
//! - une **pièce** a un volume, déjà décrit par son enveloppe de collision. On
//!   la vise donc par un **rayon** (`Chantier::piece_sous_rayon`), contre ces
//!   mêmes enveloppes — désigner une pièce et refuser d'en traverser une
//!   doivent parler du même volume.
//!
//! Ce module ne dessine rien : il ne fait que du calcul, et se teste donc
//! entièrement (§6.6 n'interdit que les tests de *rendu*).

use crate::astre::CameraInfo;
use crate::camera::projeter_ecran;
use crate::vaisseau::PortLibre;
use macroquad::prelude::*;

/// Rayon d'accroche d'un port, en **pixels**. Généreux à dessein : un port est
/// un point, viser au pixel près serait pénible, et les ports d'une station
/// sont largement plus espacés que ça à l'écran dans les cadrages usuels.
pub const ACCROCHE_PIXELS: f32 = 18.0;

/// Le port libre sous le curseur : le plus proche en **pixels**, dans la limite
/// de `accroche`, ou `None` si le curseur n'est près d'aucun.
///
/// Les ports **derrière la caméra** sont écartés par `projeter_ecran`, qui rend
/// `None` pour eux — sans ce filtre, un port dans le dos de l'observateur se
/// projetterait à des coordonnées plausibles et pourrait se retrouver désigné.
///
/// ⚠️ **L'occlusion n'est pas gérée** : un port situé derrière la station, mais
/// plus près du curseur à l'écran, l'emporte sur un port de face. C'est un choix
/// et non un oubli — §8.4 allume les ports compatibles, donc l'utilisateur voit
/// ceux qu'il peut viser, et un port caché reste visé délibérément quand on
/// pointe dessus. À revoir seulement si l'usage montre que ça gêne.
pub fn port_sous_curseur(
    libres: &[PortLibre],
    souris: Vec2,
    ecran: Vec2,
    cam: &CameraInfo,
    aspect: f32,
    accroche: f32,
) -> Option<u64> {
    libres
        .iter()
        .filter_map(|p| {
            let vu = projeter_ecran(p.repere.pos, ecran, cam, aspect)?;
            let d = vu.distance(souris);
            (d <= accroche).then_some((p.id, d))
        })
        .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::{base_orbite, rayon_ecran};
    use crate::vaisseau::{GenrePort, Profil, Repere};

    const ECRAN: Vec2 = Vec2::new(1000.0, 700.0);
    const ASPECT: f32 = 1000.0 / 700.0;

    /// Caméra de test : orbitale, à `dist` de l'origine, regardant l'origine.
    /// Bâtie par `base_orbite` — la **même** base que la vue réelle, pour que
    /// ce que le test mesure soit ce que l'écran fera.
    fn camera(yaw: f32, pitch: f32, dist: f32) -> CameraInfo {
        let (right, up, forward) = base_orbite(yaw, pitch);
        CameraInfo {
            pos: -forward * dist,
            right,
            up,
            forward,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        }
    }

    fn port(id: u64, pos: Vec3) -> PortLibre {
        PortLibre {
            id,
            repere: Repere::new(pos, Quat::IDENTITY),
            genre: GenrePort::Surface,
            profil: Profil::P1,
            origine: 0,
            indice: 0,
        }
    }

    // La propriété qui tient tout le reste : viser un point projeté doit rendre
    // un rayon qui pointe **vers ce point**. C'est ce qui garantit que la
    // désignation par pixels (les ports) et la désignation par rayon (les
    // pièces) parlent du même écran — deux troncs de vue divergents feraient
    // cliquer à côté sans que rien ne le signale.
    #[test]
    fn projeter_puis_viser_redonne_la_direction_du_point() {
        for (yaw, pitch) in [(0.0, 0.0), (0.9, 0.4), (-2.1, -0.7), (3.0, 1.1)] {
            let cam = camera(yaw, pitch, 30.0);
            for p in [
                vec3(0.0, 0.0, 0.0),
                vec3(3.0, 1.0, -2.0),
                vec3(-5.0, 4.0, 6.0),
                vec3(1.0, -7.0, 0.5),
            ] {
                let vu = projeter_ecran(p, ECRAN, &cam, ASPECT).expect("devant la caméra");
                let dir = rayon_ecran(vu, ECRAN, &cam, ASPECT);
                let attendu = (p - cam.pos).normalize();
                assert!(
                    dir.distance(attendu) < 1e-3,
                    "yaw {yaw} pitch {pitch} point {p:?} : {dir:?} au lieu de {attendu:?}"
                );
            }
        }
    }

    // Le piège de la projection : derrière la caméra, la division par une
    // profondeur négative retourne l'image et rend des pixels plausibles.
    #[test]
    fn un_point_derriere_la_camera_ne_se_projette_pas() {
        let cam = camera(0.0, 0.0, 20.0);
        let devant = cam.pos + cam.forward * 5.0;
        let derriere = cam.pos - cam.forward * 5.0;
        assert!(projeter_ecran(devant, ECRAN, &cam, ASPECT).is_some());
        assert!(projeter_ecran(derriere, ECRAN, &cam, ASPECT).is_none());
    }

    // Et la conséquence sur la désignation : le port dans le dos ne doit jamais
    // être désigné, **même** quand son fantôme retourné tombe pile sous le
    // curseur. Le port de face est au centre de l'écran ; son symétrique par
    // rapport à l'œil s'y projetterait aussi, si on le laissait faire.
    #[test]
    fn un_port_derriere_la_camera_nest_jamais_designe() {
        let cam = camera(0.0, 0.0, 20.0);
        let centre = ECRAN * 0.5;
        let derriere = cam.pos - cam.forward * 5.0;
        assert_eq!(
            port_sous_curseur(&[port(7, derriere)], centre, ECRAN, &cam, ASPECT, ACCROCHE_PIXELS),
            None,
            "un port derrière l'œil s'est fait désigner"
        );
        // Le même port, devant, est bien pris : le refus ci-dessus tient à la
        // profondeur, pas à un scénario qui ne désignerait jamais rien.
        let devant = cam.pos + cam.forward * 5.0;
        assert_eq!(
            port_sous_curseur(&[port(7, devant)], centre, ECRAN, &cam, ASPECT, ACCROCHE_PIXELS),
            Some(7)
        );
    }

    #[test]
    fn le_port_designe_est_le_plus_proche_du_curseur() {
        let cam = camera(0.6, 0.3, 25.0);
        let a = vec3(2.0, 0.0, 0.0);
        let b = vec3(-2.0, 0.0, 0.0);
        let vu_a = projeter_ecran(a, ECRAN, &cam, ASPECT).unwrap();
        let vu_b = projeter_ecran(b, ECRAN, &cam, ASPECT).unwrap();
        let ports = [port(1, a), port(2, b)];
        // Curseur posé sur chacun à son tour : l'ordre de la liste ne doit rien
        // décider — un `find` qui prendrait le premier venu passerait sinon.
        let large = vu_a.distance(vu_b);
        assert_eq!(port_sous_curseur(&ports, vu_a, ECRAN, &cam, ASPECT, large), Some(1));
        assert_eq!(port_sous_curseur(&ports, vu_b, ECRAN, &cam, ASPECT, large), Some(2));
    }

    #[test]
    fn hors_du_rayon_daccroche_rien_nest_designe() {
        let cam = camera(0.0, 0.2, 25.0);
        let p = vec3(1.0, 0.0, 0.0);
        let vu = projeter_ecran(p, ECRAN, &cam, ASPECT).unwrap();
        let ports = [port(3, p)];
        assert_eq!(port_sous_curseur(&ports, vu, ECRAN, &cam, ASPECT, 5.0), Some(3));
        let loin = vu + vec2(40.0, 0.0);
        assert_eq!(port_sous_curseur(&ports, loin, ECRAN, &cam, ASPECT, 5.0), None);
    }
}
