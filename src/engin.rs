//! **Engin orbital** : un vaisseau ou une station en orbite autour d'un astre.
//!
//! Fait le pont entre les deux moitiés du projet — le `vaisseau/`, qui sait
//! assembler et cuire une station en maillage, et le `systeme/`, qui sait faire
//! orbiter des corps. L'ISS en orbite terrestre est le premier cas.
//!
//! # ⚠️ L'échelle est **fausse**, et il n'y a pas moyen de faire autrement
//!
//! L'ISS mesure 109 m. La Terre du preset fait 0,55 unité de monde pour
//! 6 371 km de rayon réel, soit ~11 600 km par unité. À l'échelle, la station
//! ferait **9 × 10⁻⁶ unité** — mille fois plus petite qu'un pixel, à toutes les
//! distances de caméra possibles.
//!
//! Elle est donc dessinée **exagérée**, comme le sont déjà les lunes et les
//! rayons planétaires dans les presets. Le facteur est nommé
//! ([`ECHELLE_ISS`]) plutôt que noyé dans un calcul : un mensonge d'échelle
//! assumé et visible vaut mieux qu'un mensonge implicite. Le jour où une vue
//! « orbite basse » existera, elle pourra rétablir les proportions.

use crate::astre::{Astre, CameraInfo, Categorie, CorpsBase};
use crate::vaisseau::{EtatStation, MaillageStation};
use macroquad::prelude::*;

/// Rayon d'orbite de l'ISS, **en rayons de sa planète**.
///
/// Le vrai vaut 1,06 (400 km au-dessus d'un rayon de 6 371) : la station
/// raserait la surface et se perdrait dans le limbe. Relevée pour qu'on voie
/// qu'elle orbite.
pub const ORBITE_ISS: f32 = 1.55;

/// Envergure de l'ISS à l'écran, **en rayons de sa planète**. Voir l'en-tête du
/// module : à l'échelle réelle, elle serait invisible.
pub const ECHELLE_ISS: f32 = 0.30;

/// Vitesse angulaire, en radians par seconde de simulation. L'ISS boucle en
/// ~92 min ; ici on cherche un mouvement **perceptible** sans être un moulin.
const OMEGA_ISS: f32 = 0.55;

/// Inclinaison de l'orbite, en radians. La vraie est à 51,6° — assez pour que
/// l'orbite se lise comme inclinée et non comme un anneau équatorial.
const INCLINAISON_ISS: f32 = 51.6 * std::f32::consts::PI / 180.0;

/// Un engin construit dans `vaisseau/`, posé en orbite dans `systeme/`.
pub struct EnginOrbital {
    base: CorpsBase,
    maillage: MaillageStation,
    parent: usize,
    /// Rayon d'orbite, en unités de monde.
    rayon_orbite: f32,
    omega: f32,
    angle: f32,
    /// Base du plan orbital : `a1` et `q` sont orthogonaux, `q` incliné.
    a1: Vec3,
    q: Vec3,
    /// Facteur appliqué au maillage. Le vaisseau est cuit en **unités
    /// d'assemblage** (1 U ≈ 2,25 m) ; il faut le ramener au monde.
    echelle: f32,
    /// Rotation propre, pour que la station ne présente pas éternellement la
    /// même face.
    lacet: f32,
}

impl EnginOrbital {
    /// Met `etat` en orbite autour de l'astre `parent`.
    ///
    /// `rayon_planete` est le rayon monde du parent : tout le dimensionnement
    /// en découle, si bien qu'agrandir la planète emmène sa station avec elle.
    /// `None` si la station est vide (rien à dessiner).
    pub fn nouveau(
        etat: &EtatStation,
        parent: usize,
        rayon_planete: f32,
        orbite_rel: f32,
        envergure_rel: f32,
        omega: f32,
        inclinaison: f32,
    ) -> Option<Self> {
        let station = etat.doit_dessiner()?;
        let maillage = MaillageStation::cuire(station);
        // Rayon d'assemblage de la station : sert à la ramener à l'envergure
        // voulue. Une source unique — si la station grossit d'une pièce, son
        // échelle suit au lieu de la faire déborder.
        let rayon_assemblage = station.rayon().max(1e-3);
        let envergure = rayon_planete * envergure_rel;

        let a1 = Vec3::X;
        let q = (Vec3::Z * inclinaison.cos() + Vec3::Y * inclinaison.sin()).normalize();
        Some(Self {
            base: CorpsBase::new(Vec3::ZERO, 0.0, envergure),
            maillage,
            parent,
            rayon_orbite: rayon_planete * orbite_rel,
            omega,
            angle: 0.0,
            a1,
            q,
            echelle: envergure / rayon_assemblage,
            lacet: 0.0,
        })
    }

    /// L'ISS autour de la Terre, avec les valeurs de ce module.
    pub fn iss(etat: &EtatStation, parent: usize, rayon_planete: f32) -> Option<Self> {
        Self::nouveau(
            etat,
            parent,
            rayon_planete,
            ORBITE_ISS,
            ECHELLE_ISS,
            OMEGA_ISS,
            INCLINAISON_ISS,
        )
    }
}

impl Astre for EnginOrbital {
    fn categorie(&self) -> Categorie {
        Categorie::Engin
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn parent(&self) -> Option<usize> {
        Some(self.parent)
    }

    fn update(&mut self, dt: f32) {
        // Rotation propre lente, décorrélée de l'orbite : une station qui garde
        // exactement la même face en tournant a l'air posée sur un rail.
        self.lacet += dt * 0.25;
    }

    /// Orbite analytique autour du parent — même mécanique que les lunes.
    fn orbiter_autour(&mut self, centre: Vec3, dt: f32) {
        self.angle += self.omega * dt;
        self.base.position = centre
            + self.a1 * (self.rayon_orbite * self.angle.cos())
            + self.q * (self.rayon_orbite * self.angle.sin());
    }

    fn draw(&mut self, _cam: &CameraInfo) {
        // La station est cuite autour de son origine : une translation, une
        // mise à l'échelle et un lacet suffisent.
        let m = Mat4::from_scale_rotation_translation(
            Vec3::splat(self.echelle),
            Quat::from_rotation_y(self.lacet),
            self.base.position,
        );
        unsafe {
            get_internal_gl().quad_gl.push_model_matrix(m);
        }
        self.maillage.dessiner();
        unsafe {
            get_internal_gl().quad_gl.pop_model_matrix();
        }
    }

    /// Métal clair : ce qu'on retient d'une station à contre-jour.
    fn teinte(&self) -> Option<Vec3> {
        Some(vec3(0.78, 0.80, 0.84))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // **L'engin tient dans son orbite** : son envergure ne doit pas dépasser le
    // rayon auquel il tourne, sinon il traverse sa planète à chaque tour.
    #[test]
    fn lenvergure_reste_sous_le_rayon_dorbite() {
        assert!(
            ECHELLE_ISS < ORBITE_ISS - 1.0,
            "envergure {ECHELLE_ISS} contre une altitude de {} rayons",
            ORBITE_ISS - 1.0
        );
    }

    // L'orbite est **au-dessus** de la surface : un rayon orbital sous 1 mettrait
    // la station dans la planète.
    #[test]
    fn lorbite_est_au_dessus_de_la_surface() {
        assert!(ORBITE_ISS > 1.0, "l'ISS orbiterait sous la surface");
    }

    // Le plan orbital est **incliné**, et sa base reste orthonormée : `a1` et
    // `q` de travers donneraient une orbite elliptique là où on veut un cercle.
    #[test]
    fn le_plan_orbital_est_incline_et_orthonorme() {
        let a1 = Vec3::X;
        let q = (Vec3::Z * INCLINAISON_ISS.cos() + Vec3::Y * INCLINAISON_ISS.sin()).normalize();
        assert!(a1.dot(q).abs() < 1e-5, "base non orthogonale : {}", a1.dot(q));
        assert!((q.length() - 1.0).abs() < 1e-5);
        // Réellement inclinée : une orbite équatoriale aurait q.y nul.
        assert!(q.y > 0.5, "orbite trop plate : q.y = {}", q.y);
        // Et pas polaire non plus.
        assert!(q.z > 0.3, "orbite quasi polaire : q.z = {}", q.z);
    }
}
