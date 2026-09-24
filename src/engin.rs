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
use crate::systeme::trajectoire::Trajectoire;
use crate::transit::VaisseauEnTransit;
use crate::vaisseau::vol::{self, ClasseVaisseau, StatsVol};
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

/// Rayon d'orbite du Starship posté, **en rayons de sa planète**. Distinct de
/// [`ORBITE_ISS`] pour que les deux orbites ne se confondent pas à l'écran —
/// **provisoire**, comme le reste des chiffres du Lot C
/// (`docs/suivi/trajectoires.md` § « Placeholders »).
pub const ORBITE_STARSHIP: f32 = 1.85;

/// Envergure du Starship posté à l'écran, **en rayons de sa planète**. Même
/// logique d'exagération d'échelle que [`ECHELLE_ISS`] (voir l'en-tête du
/// module).
pub const ECHELLE_STARSHIP: f32 = 0.30;

/// Vitesse angulaire, en radians par seconde de simulation — même ordre de
/// grandeur que l'ISS, pas la vraie orbite d'un Starship en LEO.
const OMEGA_STARSHIP: f32 = 0.45;

/// Inclinaison de l'orbite, en radians — 28,5°, l'inclinaison réelle d'un
/// lancement depuis le Kennedy Space Center (Starbase est à une latitude
/// proche, ~26°). Assez pour se distinguer visuellement de l'ISS (51,6°).
const INCLINAISON_STARSHIP: f32 = 28.5 * std::f32::consts::PI / 180.0;

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
    /// Statistiques de vol si cet engin est un vaisseau **pilotable** —
    /// `None` pour une station postée (ISS, habitats), qui ne l'est pas
    /// (`docs/conception/trajectoires.md` §4 décision 10). Calculées une
    /// fois à la pose, depuis la `Station` d'origine, avant qu'elle ne soit
    /// cuite en maillage : les pièces ne changent plus une fois posé.
    vol: Option<(ClasseVaisseau, StatsVol)>,
}

impl EnginOrbital {
    /// Met `etat` en orbite autour de l'astre `parent`, **posté** — pas
    /// pilotable (voir [`Self::nouveau_pilotable`] pour un vaisseau qui
    /// pourra décoller).
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
        Self::nouveau_interne(etat, parent, rayon_planete, orbite_rel, envergure_rel, omega, inclinaison, None)
    }

    /// Comme [`Self::nouveau`], mais **pilotable** : `classe` fixe la base
    /// de ses statistiques de vol (`vaisseau::vol`), calculées ici une fois
    /// pour toutes — c'est ce qui permettra à [`Astre::tenter_lancer`] de
    /// faire décoller ce vaisseau plus tard, sans avoir à reparcourir sa
    /// `Station` d'origine.
    #[allow(clippy::too_many_arguments)] // Même géométrie orbitale que `nouveau`, plus la classe pilotée.
    pub fn nouveau_pilotable(
        etat: &EtatStation,
        classe: ClasseVaisseau,
        parent: usize,
        rayon_planete: f32,
        orbite_rel: f32,
        envergure_rel: f32,
        omega: f32,
        inclinaison: f32,
    ) -> Option<Self> {
        let station = etat.doit_dessiner()?;
        let stats = vol::calculer(classe, station);
        Self::nouveau_interne(
            etat,
            parent,
            rayon_planete,
            orbite_rel,
            envergure_rel,
            omega,
            inclinaison,
            Some((classe, stats)),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn nouveau_interne(
        etat: &EtatStation,
        parent: usize,
        rayon_planete: f32,
        orbite_rel: f32,
        envergure_rel: f32,
        omega: f32,
        inclinaison: f32,
        vol: Option<(ClasseVaisseau, StatsVol)>,
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
            vol,
        })
    }

    /// L'ISS autour de la Terre, avec les valeurs de ce module. Postée, pas
    /// pilotable (décision 10).
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

    /// Un Starship posté (pilotable) autour de la Terre, avec les valeurs de
    /// ce module — le premier vaisseau réellement lançable du jeu (Lot C).
    pub fn starship_pose(etat: &EtatStation, parent: usize, rayon_planete: f32) -> Option<Self> {
        Self::nouveau_pilotable(
            etat,
            ClasseVaisseau::Starship,
            parent,
            rayon_planete,
            ORBITE_STARSHIP,
            ECHELLE_STARSHIP,
            OMEGA_STARSHIP,
            INCLINAISON_STARSHIP,
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

    fn draw(&mut self, cam: &CameraInfo) {
        // La station est cuite autour de son origine : une translation, une
        // mise à l'échelle et un lacet suffisent.
        //
        // **Un seul nombre à rendre relatif, et c'est la translation.** Les
        // milliers de sommets du maillage sont déjà relatifs à l'origine de la
        // station (`vaisseau/maillage.rs`) : seule cette `Mat4` porte une
        // coordonnée héliocentrique, donc seule elle a besoin de `rel`.
        let m = Mat4::from_scale_rotation_translation(
            Vec3::splat(self.echelle),
            Quat::from_rotation_y(self.lacet),
            cam.rel(self.base.position),
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

    /// Vrai seulement pour un engin construit via `nouveau_pilotable`.
    fn pilotable(&self) -> bool {
        self.vol.is_some()
    }

    fn stats_vol(&self) -> Option<StatsVol> {
        self.vol.map(|(_, stats)| stats)
    }

    /// Décision 8 (`docs/conception/trajectoires.md` §4) : le départ d'un
    /// trajet est la position courante d'un `EnginOrbital` posté. `None` si
    /// non pilotable (une station postée, l'ISS par exemple) — rien n'est
    /// touché. Ne cuit rien de nouveau : le maillage et l'échelle déjà
    /// calculés à la pose sont **repris tels quels** (`std::mem::take`),
    /// pas régénérés — seul le mode de mouvement change.
    fn tenter_lancer(&mut self, trajectoire: Trajectoire) -> Option<Box<dyn Astre>> {
        self.vol?;
        let maillage = std::mem::take(&mut self.maillage);
        Some(Box::new(VaisseauEnTransit::depuis_maillage(
            maillage,
            self.echelle,
            self.base.rayon,
            self.base.nom,
            trajectoire,
        )))
    }
}

#[cfg(test)]
// Ces tests affirment des choses sur des CONSTANTES, et clippy y voit une
// condition toujours vraie. Elle l'est — aujourd'hui. Le jour où quelqu'un
// abaisse la constante, l'assertion devient fausse et le test tombe : c'est
// précisément son rôle, garder la valeur ET dire pourquoi elle compte.
#[allow(clippy::assertions_on_constants)]
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

    fn etat_essai() -> EtatStation {
        EtatStation::Prete(crate::vaisseau::demo_station())
    }

    // Un engin posé via `nouveau` (pas `nouveau_pilotable`) n'est jamais
    // pilotable — c'est le cas ISS/habitat par défaut (décision 10).
    #[test]
    fn nouveau_nest_jamais_pilotable() {
        let e = EnginOrbital::nouveau(&etat_essai(), 0, 1.0, 1.5, 0.3, 0.5, 0.5).unwrap();
        assert!(!e.pilotable());
        assert_eq!(e.stats_vol(), None);
    }

    // `nouveau_pilotable` calcule et garde de vraies statistiques de vol —
    // pas un espace réservé vide.
    #[test]
    fn nouveau_pilotable_calcule_les_stats_de_vol() {
        let etat = etat_essai();
        let station = etat.doit_dessiner().unwrap();
        let attendu = vol::calculer(ClasseVaisseau::Starship, station);
        let e = EnginOrbital::nouveau_pilotable(&etat, ClasseVaisseau::Starship, 0, 1.0, 1.5, 0.3, 0.5, 0.5).unwrap();
        assert!(e.pilotable());
        assert_eq!(e.stats_vol(), Some(attendu));
    }

    // `tenter_lancer` sur un engin **non** pilotable rend `None`, sans
    // toucher à l'engin — pas de vaisseau en transit fantôme.
    #[test]
    fn tenter_lancer_refuse_un_engin_non_pilotable() {
        let mut e = EnginOrbital::nouveau(&etat_essai(), 0, 1.0, 1.5, 0.3, 0.5, 0.5).unwrap();
        let t = Trajectoire::nouvelle(Vec3::ZERO, Vec3::X * 10.0, 0.0, 1.0, Vec3::Y);
        assert!(e.tenter_lancer(t).is_none());
        assert!(!e.pilotable(), "l'engin d'origine n'a pas dû changer de nature");
    }

    // `tenter_lancer` sur un engin pilotable réussit, sans recuire le
    // maillage (même rayon reporté tel quel, position au point de départ).
    #[test]
    fn tenter_lancer_reussit_pour_un_engin_pilotable() {
        let mut e = EnginOrbital::nouveau_pilotable(&etat_essai(), ClasseVaisseau::Starship, 0, 1.0, 1.5, 0.3, 0.5, 0.5).unwrap();
        let rayon_avant = e.corps().rayon;
        let depart = Vec3::ZERO;
        let arrivee = Vec3::X * 20.0;
        let t = Trajectoire::nouvelle(depart, arrivee, 0.0, 1.0, Vec3::Y);
        let v = e.tenter_lancer(t).expect("un engin pilotable doit décoller");
        assert_eq!(v.corps().rayon, rayon_avant, "le rayon d'affichage n'a pas dû changer");
        assert_eq!(v.corps().position, depart, "part bien du point de départ du trajet");
    }
}
