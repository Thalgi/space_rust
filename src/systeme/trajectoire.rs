//! **Trajectoire** (Lot B, `docs/conception/trajectoires.md` §6) : l'arc
//! géométrique qui relie un point de départ fixe à la position **future**
//! connue d'une cible sur rails (voir `Systeme::position_future_de`).
//!
//! Pas de mécanique orbitale réelle (coniques raccordées, équation de
//! Kepler) — décision assumée, pas un manque : voir `trajectoires.md` §5.
//! Un arc paramétrique **garantit** le rendez-vous par construction (son
//! point d'arrivée EST la position prédite, pas recalculée en vol), cohérent
//! avec la décision 2 (statistiques de vol en hybride, pas de simulation
//! complète).
//!
//! Écrit et testé pour son propre compte, pas encore câblé au jeu : c'est le
//! Lot B/C qui donnera à un `Astre` mobile la charge de suivre `position(t)`
//! frame après frame. D'ici là la cible binaire ne l'appelle nulle part —
//! `dead_code` ne sait pas ce qu'est un test (`methode.md`).
#![allow(dead_code)]

use macroquad::prelude::*;

/// Le sommet de l'arc s'écarte de la droite départ→arrivée d'une fraction de
/// sa longueur — un bombement visible, pas un détour absurde. **Provisoire**,
/// comme les chiffres de `vaisseau::vol` — à ajuster à l'œil une fois qu'un
/// trajet s'affiche vraiment.
const FRACTION_BOMBEMENT: f32 = 0.15;

/// Un trajet entre deux points **fixes dans le temps** : un départ figé (le
/// vaisseau a déjà quitté son foyer d'origine) et une arrivée **prédite** à
/// la construction, jamais recalculée en vol.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trajectoire {
    depart: Vec3,
    arrivee: Vec3,
    /// Direction unitaire perpendiculaire au segment départ→arrivée, dans
    /// laquelle l'arc se bombe. Nulle seulement si départ == arrivée (trajet
    /// dégénéré, longueur nulle).
    bombement_dir: Vec3,
    t_depart: f64,
    t_arrivee: f64,
}

impl Trajectoire {
    /// Construit un trajet de `depart` vers `arrivee` (déjà **prédite** —
    /// voir `Systeme::position_future_de`), partant à `t_depart`, à la
    /// vitesse de croisière `vitesse` (unités de monde par seconde de
    /// simulation — voir `vaisseau::vol::StatsVol::vitesse`).
    ///
    /// `normale_plan` oriente le bombement de l'arc (typiquement la normale
    /// du plan orbital du système) ; un vecteur nul ou colinéaire au trajet
    /// retombe sur un axe perpendiculaire arbitraire plutôt que de dégénérer
    /// en droite.
    pub fn nouvelle(depart: Vec3, arrivee: Vec3, t_depart: f64, vitesse: f32, normale_plan: Vec3) -> Self {
        let distance = (arrivee - depart).length();
        let duree = if vitesse > 1e-4 { (distance / vitesse) as f64 } else { 0.0 };
        Self {
            depart,
            arrivee,
            bombement_dir: perpendiculaire(arrivee - depart, normale_plan),
            t_depart,
            t_arrivee: t_depart + duree,
        }
    }

    /// Date de départ (même horloge que `Systeme::temps`).
    pub fn depart_a(&self) -> f64 {
        self.t_depart
    }

    /// Date d'arrivée (même horloge que `Systeme::temps`).
    pub fn arrivee_a(&self) -> f64 {
        self.t_arrivee
    }

    /// Le trajet est-il achevé à l'instant `t` ?
    pub fn terminee(&self, t: f64) -> bool {
        t >= self.t_arrivee
    }

    /// Fraction parcourue à l'instant `t`, bornée à `[0, 1]` — pas
    /// d'extrapolation avant le départ ni après l'arrivée.
    fn progres(&self, t: f64) -> f32 {
        let duree = self.t_arrivee - self.t_depart;
        if duree <= 0.0 {
            return 1.0;
        }
        (((t - self.t_depart) / duree) as f32).clamp(0.0, 1.0)
    }

    /// Position sur l'arc à l'instant absolu `t`. Bézier quadratique :
    /// départ, un point de contrôle décalé au milieu, arrivée — garantit les
    /// deux bouts **exacts** par construction (`u = 0` rend `depart`, `u = 1`
    /// rend `arrivee`, quel que soit le point de contrôle).
    pub fn position(&self, t: f64) -> Vec3 {
        let u = self.progres(t);
        let distance = (self.arrivee - self.depart).length();
        let milieu = (self.depart + self.arrivee) * 0.5;
        let controle = milieu + self.bombement_dir * (distance * FRACTION_BOMBEMENT);
        let un_moins_u = 1.0 - u;
        self.depart * (un_moins_u * un_moins_u) + controle * (2.0 * un_moins_u * u) + self.arrivee * (u * u)
    }

    /// Échantillonne l'arc en `n` points, du départ à l'arrivée inclus — pour
    /// l'aperçu du trajet avant confirmation (Lot D,
    /// `docs/conception/trajectoires.md` §6). Contrairement à
    /// `Orbite::polyligne`, [`Self::position`] rend déjà des coordonnées
    /// **monde** absolues : pas de centre à additionner côté appelant, et pas
    /// de bouclage — c'est un arc ouvert, pas une orbite fermée.
    ///
    /// `n < 2` retombe sur 2 : les deux seuls points qui ont un sens en
    /// dessous sont le départ et l'arrivée.
    pub fn polyligne(&self, n: usize) -> Vec<Vec3> {
        let n = n.max(2);
        (0..n)
            .map(|i| {
                let u = i as f64 / (n - 1) as f64;
                self.position(self.t_depart + u * (self.t_arrivee - self.t_depart))
            })
            .collect()
    }
}

/// Un vecteur unitaire perpendiculaire à `droite`, dans la direction la plus
/// proche de `normale_plan`. Retombe sur un axe perpendiculaire arbitraire
/// (`Vec3::Y`, ou `Vec3::Z` si `droite` est déjà verticale — même parade que
/// `ecran::icone::cadre_camera`) si `normale_plan` est nulle ou colinéaire à
/// `droite`. Vecteur nul en sortie seulement si `droite` l'est déjà.
fn perpendiculaire(droite: Vec3, normale_plan: Vec3) -> Vec3 {
    let d = droite.normalize_or_zero();
    if d == Vec3::ZERO {
        return Vec3::ZERO;
    }
    let n = normale_plan.normalize_or_zero();
    let essai = d.cross(n);
    if essai.length_squared() > 1e-8 {
        return essai.normalize();
    }
    let repli = if d.cross(Vec3::Y).length_squared() > 1e-8 { Vec3::Y } else { Vec3::Z };
    d.cross(repli).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arc() -> Trajectoire {
        Trajectoire::nouvelle(vec3(0.0, 0.0, 0.0), vec3(10.0, 0.0, 0.0), 100.0, 2.0, Vec3::Y)
    }

    #[test]
    fn au_depart_la_position_est_le_point_de_depart() {
        let a = arc();
        assert_eq!(a.position(100.0), vec3(0.0, 0.0, 0.0));
    }

    #[test]
    fn a_larrivee_la_position_est_le_point_darrivee() {
        let a = arc();
        let pos = a.position(a.arrivee_a());
        assert!((pos - vec3(10.0, 0.0, 0.0)).length() < 1e-4, "arrivée imprécise : {pos}");
    }

    #[test]
    fn apres_larrivee_la_position_reste_bloquee_a_larrivee() {
        let a = arc();
        let pos = a.position(a.arrivee_a() + 1000.0);
        assert!((pos - vec3(10.0, 0.0, 0.0)).length() < 1e-4, "extrapole au-delà de l'arrivée : {pos}");
    }

    #[test]
    fn avant_le_depart_la_position_reste_au_depart() {
        let a = arc();
        assert_eq!(a.position(0.0), vec3(0.0, 0.0, 0.0));
    }

    // Distance 10, vitesse 2 -> 5 secondes de trajet, pas une valeur arbitraire.
    #[test]
    fn la_duree_depend_de_la_distance_et_de_la_vitesse() {
        let a = arc();
        assert!((a.arrivee_a() - 105.0).abs() < 1e-4, "arrivée attendue à 105.0, obtenu {}", a.arrivee_a());
    }

    // À mi-parcours, l'arc s'écarte de la droite : sinon ce ne serait qu'un
    // segment, pas un « trajet ».
    #[test]
    fn larc_se_bombe_hors_de_la_droite_au_milieu() {
        let a = arc();
        let mi_temps = (a.t_depart + a.t_arrivee) * 0.5;
        let pos = a.position(mi_temps);
        let milieu_droite = vec3(5.0, 0.0, 0.0);
        assert!((pos - milieu_droite).length() > 0.5, "l'arc ne se bombe pas : {pos}");
    }

    // Le bombement reste DANS le plan orbital (perpendiculaire à sa normale),
    // pas hors de lui : un vaisseau qui décollerait de l'écliptique à chaque
    // trajet serait absurde. Avec `normale_plan = Y` et un trajet selon X,
    // le bombement doit tomber sur Z (perpendiculaire aux deux), jamais sur Y.
    #[test]
    fn le_bombement_reste_dans_le_plan_orbital() {
        let a = arc();
        let mi_temps = (a.t_depart + a.t_arrivee) * 0.5;
        let pos = a.position(mi_temps);
        assert!(pos.z.abs() > 0.5, "le bombement ne s'écarte pas dans le plan : {pos}");
        assert!(pos.y.abs() < 1e-4, "le bombement sort du plan orbital : {pos}");
    }

    #[test]
    fn terminee_devient_vraie_a_larrivee_pas_avant() {
        let a = arc();
        assert!(!a.terminee(a.arrivee_a() - 0.1));
        assert!(a.terminee(a.arrivee_a()));
    }

    // Départ == arrivée : trajet dégénéré, ne doit pas paniquer (division par
    // une longueur nulle dans `perpendiculaire`/`position`).
    #[test]
    fn un_trajet_de_longueur_nulle_ne_panique_pas() {
        let a = Trajectoire::nouvelle(Vec3::ONE, Vec3::ONE, 0.0, 3.0, Vec3::Y);
        assert_eq!(a.position(0.0), Vec3::ONE);
        assert_eq!(a.arrivee_a(), 0.0);
    }

    // `polyligne` rend bien `n` points — pas moins (troncature), pas plus.
    #[test]
    fn polyligne_rend_le_nombre_de_points_demande() {
        let a = arc();
        assert_eq!(a.polyligne(12).len(), 12);
    }

    // Premier et dernier échantillon = départ et arrivée EXACTS — c'est ce qui
    // fait qu'un aperçu raccorde visuellement avec le vaisseau puis la cible,
    // sans laisser deviner un écart d'arrondi.
    #[test]
    fn polyligne_commence_au_depart_et_finit_a_larrivee() {
        let a = arc();
        let pts = a.polyligne(8);
        assert_eq!(pts[0], a.position(a.depart_a()));
        assert_eq!(*pts.last().unwrap(), a.position(a.arrivee_a()));
    }

    // En dessous de 2, un arc n'a pas de sens : on retombe sur les deux seuls
    // points sûrs plutôt que de rendre une liste vide ou un point unique.
    #[test]
    fn polyligne_remonte_a_deux_points_en_dessous() {
        let a = arc();
        assert_eq!(a.polyligne(0).len(), 2);
        assert_eq!(a.polyligne(1).len(), 2);
    }
}
