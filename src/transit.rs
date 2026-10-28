//! **Vaisseau en transit** (Lot B, `docs/conception/trajectoires.md`) : un
//! vaisseau assemblé qui suit une [`Trajectoire`] au lieu d'orbiter un
//! parent. Le pendant d'`EnginOrbital` (`engin.rs`) pour un astre
//! **mobile** — même pont `vaisseau/` ↔ `systeme/`, mécanique de position
//! différente : pas de foyer, pas de `dt` incrémental, la position est déjà
//! connue d'avance (voir `Trajectoire`).
//!
//! Ce que ce fichier ne fait **pas** : décider où un trajet démarre en jeu,
//! ce qui se passe à l'arrivée (amarrage, consommation à la colonisation),
//! ou piloter plusieurs vaisseaux depuis l'écran — tout ça reste aux lots
//! C/E (`trajectoires.md` §6/§7). Ici, un vaisseau suit fidèlement une
//! trajectoire déjà construite, rien de plus.
//!
//! Écrit et testé pour son propre compte, pas encore câblé au jeu : rien ne
//! construit encore de `VaisseauEnTransit` en jeu (Lot C — où un trajet
//! démarre — reste à faire). D'ici là la cible binaire ne l'appelle nulle
//! part — `dead_code` ne sait pas ce qu'est un test (`methode.md`).
#![allow(dead_code)]

use crate::astre::{Astre, CameraInfo, Categorie, CorpsBase};
use crate::systeme::trajectoire::Trajectoire;
use crate::vaisseau::{EtatStation, MaillageStation};
use macroquad::prelude::*;

/// Un engin construit dans `vaisseau/`, **en transit** le long d'une
/// [`Trajectoire`] plutôt qu'en orbite fixe.
pub struct VaisseauEnTransit {
    base: CorpsBase,
    maillage: MaillageStation,
    trajectoire: Trajectoire,
    /// Facteur appliqué au maillage — même rôle qu'`EnginOrbital::echelle`
    /// (le vaisseau est cuit en unités d'assemblage, pas en unités monde).
    echelle: f32,
}

impl VaisseauEnTransit {
    /// Lance `etat` sur `trajectoire`. `envergure` est la taille voulue à
    /// l'écran, en unités monde — pas de rayon de planète pour la dériver
    /// ici, contrairement à `EnginOrbital::nouveau` : un trajet n'a pas de
    /// parent unique dont hériter une échelle. `None` si la station est
    /// vide (rien à dessiner).
    pub fn nouveau(etat: &EtatStation, trajectoire: Trajectoire, envergure: f32) -> Option<Self> {
        let station = etat.doit_dessiner()?;
        let maillage = MaillageStation::cuire(station);
        // Même garde-fou qu'`EnginOrbital` : rayon d'assemblage borné pour
        // qu'une station quasi ponctuelle ne fasse pas exploser l'échelle.
        let rayon_assemblage = station.rayon().max(1e-3);
        let echelle = envergure / rayon_assemblage;
        Some(Self::depuis_maillage(maillage, echelle, envergure, None, trajectoire))
    }

    /// Construit directement depuis un maillage **déjà cuit**, sans nouvelle
    /// cuisson — sert `EnginOrbital`'s `Astre::tenter_lancer` (décision 8,
    /// `docs/conception/trajectoires.md` §4) : convertir un engin posté en
    /// vaisseau en transit ne doit pas retravailler sa géométrie, seulement
    /// changer son mode de mouvement. `rayon` est le rayon d'affichage/de
    /// clic (même rôle que `envergure` dans [`Self::nouveau`]) ; `nom` est
    /// reporté depuis l'astre d'origine, s'il en avait un.
    pub(crate) fn depuis_maillage(
        maillage: MaillageStation,
        echelle: f32,
        rayon: f32,
        nom: Option<&'static str>,
        trajectoire: Trajectoire,
    ) -> Self {
        let position_initiale = trajectoire.position(trajectoire.depart_a());
        let mut base = CorpsBase::new(position_initiale, 0.0, rayon);
        base.nom = nom;
        Self { base, maillage, trajectoire, echelle }
    }

    /// Le trajet est-il achevé à l'instant `t` ? Voir
    /// [`Trajectoire::terminee`] — le vaisseau reste visible, immobile à
    /// l'arrivée, tant que rien (Lot E) ne décide de son sort.
    pub fn arrive(&self, t: f64) -> bool {
        self.trajectoire.terminee(t)
    }
}

impl Astre for VaisseauEnTransit {
    fn categorie(&self) -> Categorie {
        Categorie::Engin
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn update(&mut self, _dt: f32) {}

    /// Place le vaisseau sur sa trajectoire à l'instant absolu `t` — pas
    /// d'`orbiter_autour` incrémental, la position est **connue d'avance**.
    fn suivre_trajectoire(&mut self, t: f64) {
        self.base.position = self.trajectoire.position(t);
    }

    fn draw(&mut self, cam: &CameraInfo) {
        // Même schéma qu'`EnginOrbital::draw` : translation + échelle, la
        // translation seule passant au repère de rendu. Pas encore
        // d'orientation pilotée par la trajectoire (attitude) — dette connue,
        // `trajectoires.md` §3.6.
        crate::engin::dessiner_engin(&self.maillage, cam, self.base.position, self.base.rayon, self.echelle, Quat::IDENTITY);
    }

    /// Même teinte qu'`EnginOrbital` : le métal clair d'une station à
    /// contre-jour.
    fn teinte(&self) -> Option<Vec3> {
        Some(vec3(0.78, 0.80, 0.84))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::demo_station;

    fn trajet() -> Trajectoire {
        Trajectoire::nouvelle(vec3(0.0, 0.0, 0.0), vec3(10.0, 0.0, 0.0), 100.0, 2.0, Vec3::Y)
    }

    #[test]
    fn nouveau_rend_none_pour_une_station_vide() {
        assert!(VaisseauEnTransit::nouveau(&EtatStation::Vide, trajet(), 1.0).is_none());
    }

    // Avant tout appel à `suivre_trajectoire`, le vaisseau doit déjà être au
    // point de départ — pas à l'origine du monde ni au point d'arrivée.
    #[test]
    fn nouveau_place_le_vaisseau_au_point_de_depart() {
        let etat = EtatStation::Prete(demo_station());
        let t = trajet();
        let attendu = t.position(t.depart_a());
        let v = VaisseauEnTransit::nouveau(&etat, t, 1.0).unwrap();
        assert_eq!(v.corps().position, attendu);
    }

    // `suivre_trajectoire` délègue vraiment à la `Trajectoire` — pas une
    // formule à part qui pourrait diverger.
    #[test]
    fn suivre_trajectoire_avance_le_long_de_larc() {
        let etat = EtatStation::Prete(demo_station());
        let t = trajet();
        let mi_temps = (t.depart_a() + t.arrivee_a()) * 0.5;
        let attendu = t.position(mi_temps);
        let mut v = VaisseauEnTransit::nouveau(&etat, t, 1.0).unwrap();
        v.suivre_trajectoire(mi_temps);
        assert_eq!(v.corps().position, attendu);
    }

    #[test]
    fn arrive_devient_vrai_a_larrivee_pas_avant() {
        let etat = EtatStation::Prete(demo_station());
        let t = trajet();
        let arrivee_a = t.arrivee_a();
        let v = VaisseauEnTransit::nouveau(&etat, t, 1.0).unwrap();
        assert!(!v.arrive(arrivee_a - 0.1));
        assert!(v.arrive(arrivee_a));
    }
}
