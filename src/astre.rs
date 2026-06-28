use macroquad::prelude::*;

/// Les différentes catégories d'astres du système.
/// On en ajoutera au fur et à mesure (planètes, lunes, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Lune/Comete prévues pour la suite
pub enum Categorie {
    Etoile,
    Planete,
    Lune,
    Asteroide,
    Comete,
}

/// Données physiques communes à TOUS les astres.
/// (Un trait ne peut pas stocker de champs : on factorise donc ici.)
pub struct CorpsBase {
    pub position: Vec3,
    pub vitesse: Vec3,
    pub masse: f32,
    pub rayon: f32,
}

impl CorpsBase {
    pub fn new(position: Vec3, masse: f32, rayon: f32) -> Self {
        Self {
            position,
            vitesse: Vec3::ZERO,
            masse,
            rayon,
        }
    }
}

/// Repère de la caméra, transmis au dessin pour orienter les billboards /
/// impostors face à l'objectif et calculer l'éclairage.
#[derive(Clone, Copy)]
pub struct CameraInfo {
    pub pos: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,     // de la caméra vers la cible
    pub light_pos: Vec3,   // position de l'étoile (éclairage des planètes)
    pub light_color: Vec3, // couleur * intensité de l'étoile
}

/// La "superclasse" : tout ce qui est un astre sait se mettre à jour
/// et se dessiner, et expose ses données physiques de base.
pub trait Astre {
    fn categorie(&self) -> Categorie;
    fn corps(&self) -> &CorpsBase;
    fn corps_mut(&mut self) -> &mut CorpsBase; // accès mutable pour l'intégrateur gravitationnel
    fn update(&mut self, dt: f32);
    /// Dessine l'astre. `cam` fournit le repère caméra (orientation des
    /// billboards/impostors + éclairage). `&mut` car on réutilise des tampons.
    fn draw(&mut self, cam: &CameraInfo);

    // Méthodes par défaut, utilisables par toutes les sous-classes.
    #[allow(dead_code)] // utilitaires prévus pour la suite
    fn position(&self) -> Vec3 {
        self.corps().position
    }
    #[allow(dead_code)]
    fn masse(&self) -> f32 {
        self.corps().masse
    }
    /// Réglage des éruptions (ignoré par défaut ; seules les étoiles l'utilisent).
    fn set_eruptions(&mut self, _freq: f32, _forme: f32, _puissance: f32, _alea: f32) {}

    /// Couleur*intensité émise si l'astre est une source de lumière (étoile).
    fn lumiere(&self) -> Option<Vec3> {
        None
    }

    /// Bornes (interne, externe) de la zone habitable, si l'astre est une étoile.
    fn zone_viable(&self) -> Option<(f32, f32)> {
        None
    }

    /// Polyligne de la trajectoire (relative au foyer/étoile), pour tracer l'orbite.
    fn orbite(&self) -> &[Vec3] {
        &[]
    }

    /// Index de l'astre parent si c'est une lune (sinon None). Une lune n'est pas
    /// intégrée par la gravité N-corps : elle orbite analytiquement son parent.
    fn parent(&self) -> Option<usize> {
        None
    }
    /// Place la lune autour de `centre` (position du parent) et avance son orbite.
    fn orbiter_autour(&mut self, _centre: Vec3, _dt: f32) {}
}
