use crate::systeme::trajectoire::Trajectoire;
use crate::vaisseau::vol::StatsVol;
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
    /// **Origine du repere de rendu** : le point du monde envoye au GPU comme
    /// zero. Tout ce qui part vers la carte graphique passe par [`Self::rel`].
    ///
    /// Pourquoi : les positions sont des `Vec3` f32 heliocentriques absolues ;
    /// a 460 unites (Saturne) deux f32 voisins sont separes de 3 x 10^-5 unite.
    /// La matrice vue-projection de macroquad est batie en f32, si bien qu'un
    /// astre pourtant SUIVI par la camera sautait a l'ecran -- 0,6 px a
    /// Saturne, 2,7 px a Neptune a `dist = 0,05`. En posant l'origine sur la
    /// cible, `p` et `origine` tiennent dans un facteur 2 l'un de l'autre :
    /// leur difference f32 est alors EXACTE (Sterbenz), et l'astre focalise se
    /// rend en zero, immobile par construction.
    /// Voir `docs/conception/repere_flottant.md`.
    pub origine: Vec3,
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

impl CameraInfo {
    /// Coordonnee **du repere de rendu** du point monde `p`.
    ///
    /// **Source unique** de la conversion : c'est le seul endroit ou une
    /// position monde devient une position GPU. Le risque de ce chantier est
    /// la regression silencieuse -- un `centre`, une `lumiere` ou une
    /// translation de `Mat4` oubliee en absolu ne casse aucun test, elle
    /// decale seulement un terminateur de planete. Faire passer TOUT par cette
    /// fonction est le garde-fou (`docs/methode.md`, « une source par fait »).
    ///
    /// ⚠️ `pos`, lui, reste **absolu** : le picking, les distances et les LOD
    /// (`rayon_dessine`, `px_rayon`, seuils sub-pixel des debris) et
    /// `projeter_ecran` s'appuient dessus.
    pub fn rel(&self, p: Vec3) -> Vec3 {
        p - self.origine
    }

    /// Les quatre positions d'eclairage, rendues au repere de rendu.
    ///
    /// Les emplacements inutilises portent une couleur nulle : les translater
    /// aussi ne change rien au rendu, et evite d'avoir a distinguer ici les
    /// sources vivantes des mortes.
    pub fn lights_rel(&self) -> [Vec3; 4] {
        self.lights_pos.map(|p| self.rel(p))
    }
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

    /// Normale du plan de l'anneau, si l'astre en a un — `None` sinon.
    ///
    /// Sert à cadrer l'icône du sélecteur sous un angle **choisi**, plutôt
    /// que de subir l'azimut arbitraire dicté par la position orbitale du
    /// moment : un anneau incliné (Saturne en a vraiment un, ~21° hors de
    /// l'axe Y) peut, selon d'où on regarde, se présenter presque de face —
    /// grand, dominant — ou presque de profil, selon un hasard qui n'a rien
    /// à voir avec ce qui se lit le mieux en 48 px.
    fn normale_anneau(&self) -> Option<Vec3> {
        None
    }

    /// Le rendu de cet astre est-il **définitif** ? Vrai par défaut — la
    /// plupart des astres n'ont rien à attendre.
    ///
    /// Faux tant qu'une tellurique n'a pas fini de générer son terrain (job
    /// asynchrone, budget limité de générations simultanées — voir
    /// `Planete::terrain_pret`) : avant ça, son relief/altitude retombe sur
    /// un texture-espace réservé (le repli « corps non tellurique », voir
    /// `materiau.rs`), pas la vraie géographie. Un rendu pris **pendant**
    /// cette fenêtre — une icône du sélecteur, par exemple — figerait ce
    /// repli pour de bon dans le cache, indépendamment de ce que dit
    /// vraiment le preset (eau, végétation…) : l'astre en cause a l'air
    /// définitivement mal généré alors qu'il ne l'était pas **encore**.
    fn rendu_pret(&self) -> bool {
        true
    }

    /// Cette planète est-elle une **géante gazeuse** ? `None` pour ce qui n'a
    /// pas de type planétaire (étoiles, lunes, ceintures, engins) — l'appelant
    /// retombe alors sur la taille d'icône par défaut.
    ///
    /// Sert au sélecteur, où une tellurique se dessine à la **moitié** de la
    /// taille d'une gazeuse : deux mondes de rayons physiques très différents
    /// (une Terre, une Jupiter) ne doivent pas se lire comme deux pastilles de
    /// même poids visuel.
    fn gazeuse(&self) -> Option<bool> {
        None
    }

    /// **Le type de cette planète** (chantier planètes) — habitable,
    /// colonisable, géante… `None` pour ce qui n'en a pas : étoiles,
    /// ceintures, engins, comètes.
    fn classe_planete(&self) -> Option<crate::planete::classe::ClassePlanete> {
        None
    }

    /// Rayon **réellement dessiné**, en unités monde, vu depuis `cam_pos`.
    ///
    /// Par défaut le rayon physique : ce que voit l'œil est ce qui existe. Seule
    /// l'étoile s'en écarte (plancher en pixels, voir `soleil::PX_MIN`), parce
    /// qu'à l'échelle vraie elle fait un pixel de large au cadrage par défaut.
    ///
    /// **`Systeme::pick` lit cette valeur-là**, pas `corps().rayon` : sans quoi
    /// la cible de clic resterait celle du rayon physique et l'étoile deviendrait
    /// un disque de 8 px qu'il faudrait toucher au pixel près.
    fn rayon_dessine(&self, _cam_pos: Vec3) -> f32 {
        self.corps().rayon
    }

    /// Bornes (interne, externe) de la zone habitable, si l'astre est une étoile.
    fn zone_viable(&self) -> Option<(f32, f32)> {
        None
    }

    /// Polyligne de la trajectoire (relative au foyer/étoile), pour tracer l'orbite.
    fn orbite(&self) -> &[Vec3] {
        &[]
    }

    /// Polyligne de l'orbite d'une **lune**, relative à son parent. Vide pour
    /// tout ce qui n'est pas une lune.
    ///
    /// Séparée d'[`Astre::orbite`] parce que le repère diffère : la trajectoire
    /// d'une planète se centre sur son foyer stellaire, celle d'une lune sur la
    /// planète — qui se déplace. Les confondre poserait les orbites lunaires
    /// autour du barycentre du système.
    fn orbite_lune(&self) -> &[Vec3] {
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

    /// Avance un astre **en transit** (Lot B, `docs/conception/trajectoires.md`) :
    /// place le corps sur sa trajectoire à l'instant absolu `t` (même
    /// horloge que `Systeme::temps`). No-op par défaut — seul un vaisseau en
    /// transit (`VaisseauEnTransit`, `transit.rs`) le fait vraiment.
    ///
    /// Ni `orbiter_autour` (incrémental, prend un `dt` et un centre de
    /// parent) ni `maj_rail` (a besoin d'un foyer) ne conviennent : un
    /// trajet n'a ni parent unique ni foyer, sa position est entièrement
    /// déterminée par son départ, sa destination prédite, et l'heure.
    fn suivre_trajectoire(&mut self, _t: f64) {}

    /// Mode « sur rails » : place le corps sur son orbite de Kepler analytique à
    /// l'instant `t`, autour de `foyer` (position de l'astre central). No-op pour
    /// les corps sans orbite de Kepler (étoiles, lunes, ceintures).
    fn maj_rail(&mut self, _foyer: Vec3, _t: f64) {}

    /// Foyer d'orbite si le corps est une planète « sur rails » (autour d'une étoile
    /// hôte ou du barycentre). `None` par défaut (étoiles, lunes, ceintures).
    fn foyer(&self) -> Option<Foyer> {
        None
    }

    /// Position **prédite** au temps absolu `t` (même horloge que
    /// `Systeme::temps`), **sans muter** l'astre — la même évaluation
    /// analytique que [`Astre::maj_rail`], mais en lecture seule et
    /// projetable dans le futur. `foyer_predit` est la position déjà
    /// résolue du foyer au même instant `t` (l'appelant compose la
    /// hiérarchie, comme pour `maj_rail`).
    ///
    /// `None` par défaut, pour tout ce qui n'a pas d'orbite de Kepler connue
    /// à cette date : les lunes et les engins (`orbiter_autour`, pas encore
    /// couverts — voir `docs/conception/trajectoires.md` §7), les étoiles,
    /// les ceintures. Sert au chantier trajectoires (Lot B) à viser la
    /// position future d'une cible sur rails.
    #[allow(dead_code)] // Consommé par Systeme::position_future_de (Lot B), pas encore câblé au jeu.
    fn position_future(&self, _foyer_predit: Vec3, _t: f64) -> Option<Vec3> {
        None
    }

    /// Cet astre est-il un vaisseau **pilotable**, prêt à s'élancer vers une
    /// destination ? Faux par défaut. Seul `EnginOrbital` (un engin posté,
    /// `engin.rs`) peut répondre vrai — et seulement s'il a été construit
    /// via `EnginOrbital::nouveau_pilotable`, pas `nouveau` : une station
    /// postée (l'ISS, un habitat) ne l'est jamais
    /// (`docs/conception/trajectoires.md` §4 décision 10).
    fn pilotable(&self) -> bool {
        false
    }

    /// Ses statistiques de vol (`vaisseau::vol`), si [`Self::pilotable`].
    /// `None` sinon.
    fn stats_vol(&self) -> Option<StatsVol> {
        None
    }

    /// Tente de convertir cet astre en vaisseau en transit sur
    /// `trajectoire`, s'il est [`Self::pilotable`]. `None` par défaut —
    /// l'appelant ([`Systeme::lancer`](crate::systeme::Systeme::lancer))
    /// garde alors l'astre d'origine intact.
    ///
    /// `&mut self`, pas `self` : rend le défaut trivialement object-safe
    /// (pas de coercion `Box<Self> → Box<dyn Astre>` générique à faire
    /// tenir). Seul `EnginOrbital` le redéfinit
    /// (`docs/conception/trajectoires.md` §4 décision 8) — en vidant ses
    /// champs avec `std::mem::take` plutôt qu'en consommant `self` : l'astre
    /// d'origine, vidé, est de toute façon jeté par l'appelant juste après.
    fn tenter_lancer(&mut self, _trajectoire: Trajectoire) -> Option<Box<dyn Astre>> {
        None
    }
}
