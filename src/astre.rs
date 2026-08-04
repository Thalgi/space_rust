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
    /// **Engin construit** : station ou vaisseau assemblé dans `vaisseau/`, mis
    /// en orbite dans un système. Voir `engin.rs`.
    Engin,
}

/// Foyer d'une planète « sur rails » : autour de quoi elle orbite.
/// - `Barycentre` : le centre de masse du système (origine) — type P (circumbinaire)
///   ou étoile unique à l'origine.
/// - `Etoile(idx)` : une étoile hôte précise (mobile) — type S (circumstellaire).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Foyer {
    Barycentre,
    Etoile(usize),
}

/// Données physiques communes à TOUS les astres.
/// (Un trait ne peut pas stocker de champs : on factorise donc ici.)
pub struct CorpsBase {
    pub position: Vec3,
    pub vitesse: Vec3,
    pub masse: f32,
    pub rayon: f32,
    /// **Nom propre**, s'il en a un. Seuls les presets écrits à la main en
    /// portent : les systèmes engendrés retombent sur la numérotation orbitale
    /// (`Systeme::designation`), plutôt que sur des noms inventés — un mauvais
    /// nom procédural se remarque bien plus qu'un chiffre
    /// (`docs/conception/interface.md` §2.2a).
    ///
    /// Ici et non dans une table parallèle à `Systeme::astres` : deux vecteurs
    /// à tenir alignés, c'est un désaccord qui attend, et `ajouter` devrait
    /// alors pousser dans les deux.
    pub nom: Option<&'static str>,
}

impl CorpsBase {
    pub fn new(position: Vec3, masse: f32, rayon: f32) -> Self {
        Self {
            position,
            vitesse: Vec3::ZERO,
            masse,
            rayon,
            nom: None,
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
    pub light_pos: Vec3,   // position de l'étoile PRIMAIRE (spéculaire, terminateur…)
    pub light_color: Vec3, // couleur * intensité de l'étoile primaire
    // Éclairage multi-source (systèmes à plusieurs étoiles). Indice 0 = primaire
    // (= light_pos/light_color) ; entrées inutilisées : couleur nulle.
    pub lights_pos: [Vec3; 4],
    pub lights_color: [Vec3; 4],
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

    /// Luminosité scalaire (intensité relative) si l'astre est une étoile. Sert au
    /// calcul de la zone habitable combinée (circumbinaire) des systèmes multiples.
    fn luminosite(&self) -> Option<f32> {
        None
    }

    /// **Teinte d'ensemble** de l'astre, telle qu'on la résumerait en un point :
    /// la couleur de sa pastille dans le sélecteur.
    ///
    /// Tirée de l'apparence réelle du corps, jamais d'une table posée à côté —
    /// sinon deux planètes bleues et rouges auraient la même pastille, et une
    /// retouche d'apparence ne s'y verrait pas. `None` pour ce qui n'a pas de
    /// couleur propre (ceintures), et l'appelant retombe alors sur la catégorie.
    fn teinte(&self) -> Option<Vec3> {
        None
    }

    /// Étendue visuelle, **en rayons du corps** : 1 pour une boule nue, plus
    /// pour ce qui déborde (l'anneau d'une géante). Sert au cadrage d'un
    /// portrait, où un anneau doit tenir dans la case.
    fn etendue_visuelle(&self) -> f32 {
        1.0
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

    /// Nom propre de l'astre, s'il en a un. Lu du corps : une seule source.
    fn nom(&self) -> Option<&'static str> {
        self.corps().nom
    }
    /// Place la lune autour de `centre` (position du parent) et avance son orbite.
    fn orbiter_autour(&mut self, _centre: Vec3, _dt: f32) {}

    /// Mode « sur rails » : place le corps sur son orbite de Kepler analytique à
    /// l'instant `t`, autour de `foyer` (position de l'astre central). No-op pour
    /// les corps sans orbite de Kepler (étoiles, lunes, ceintures).
    fn maj_rail(&mut self, _foyer: Vec3, _t: f64) {}

    /// Hand-off vers le mode N-corps : amorce position/vitesse depuis l'orbite
    /// analytique (état vis-viva à `t`), pour partir d'un état cohérent. No-op par défaut.
    fn amorcer_ncorps(&mut self, _foyer_pos: Vec3, _foyer_vel: Vec3, _t: f64) {}

    /// Foyer d'orbite si le corps est une planète « sur rails » (autour d'une étoile
    /// hôte ou du barycentre). `None` par défaut (étoiles, lunes, ceintures).
    fn foyer(&self) -> Option<Foyer> {
        None
    }
}
