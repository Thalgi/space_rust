// La source unique d'où découlent taches (sous-lot 3.2), pieds d'arche (3.3)
// et assombrissement du shader (3.4) — `docs/conception/astres_v2.md` §6 bis.
mod champ;
mod eruptions;
mod materiau;
mod rendu;

use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase};
use crate::disque::{Accretion, Disque, DisqueConfig};
use champ::Pole;
use eruptions::{Boucle, Flare, Tache};
use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use materiau::{mat_plasma_partage, mats_soleil};
use rendu::{texture_halo, TAILLE_HALO};

// Doit correspondre à spots[8] dans le shader. Partagé entre taches ET
// flares (voir `eruptions::encoder_flare`) : pas de second tableau d'uniforms.
const MAX_TACHES: usize = 8;

/// **Rayon apparent minimal d'une étoile, en pixels.**
///
/// À l'échelle vraie une étoile fait **1,14 px de rayon** au cadrage par défaut
/// de la vue système (§ P.3) : ni granulation, ni tache, ni couleur ne peuvent
/// s'y lire. Ce n'est pas un bug — une étoile *est* un point à deux milliards de
/// kilomètres — mais c'est illisible.
///
/// Ce que 8 px achètent, mesuré contre la table de P.3 :
///
/// - **disque rond, teinte, assombrissement du limbe** : à partir de ~4 px ;
/// - **taches** (`taille ≈ 0,2` du rayon) : ~2 px de large ici, lisibles ;
/// - **boucles d'éruption** (dessinées à `r × 0,05`) : il en faudrait 20 px pour
///   qu'elles atteignent le pixel, 60 pour qu'elles se lisent comme des arches.
///   **Aucun plancher raisonnable ne les montre au cadrage par défaut** — le
///   Soleil serait plus large que l'orbite de Mercure. Elles restent la
///   récompense de l'approche, ce qui est de toute façon là qu'elles sont belles.
///
/// Les corps sont déjà grossis d'un facteur fixe 5 (`presets::ECHELLE_CORPS`,
/// distances laissées vraies) : le plancher ne crée pas l'exagération, il la rend
/// dépendante de la distance. Et il **s'efface** en approchant (voir
/// [`impostor::rayon_plancher`]).
///
/// **Jugé à l'œil (2026-08-14) : 8 px faisait un Soleil trop gros au cadrage
/// par défaut.** Ramené à **4 px** — le seuil identifié juste au-dessus comme
/// celui où le disque rond, la teinte et l'assombrissement du limbe
/// commencent déjà à se lire ; en dessous, les taches (~2 px) resteraient les
/// seules encore lisibles.
pub const PX_MIN: f32 = 4.0;

/// Vide le cache de materials du soleil (hot-reload des shaders).
pub fn vider_cache_materials() {
    materiau::vider_cache();
}

pub struct Soleil {
    pub base: CorpsBase,
    /// **Graine de surface**, propre à cette étoile.
    ///
    /// Sans elle, la granulation du shader est une fonction déterministe de
    /// la seule direction : **toutes les étoiles portaient exactement le
    /// même motif**, et deux compagnes d'un binaire étaient jumelles image
    /// par image. Elle décale les champs de bruit ; le décalage de `temps`
    /// ci-dessous s'occupe du reste (voir sa doc).
    graine: f32,
    /// **Les pôles magnétiques de cette étoile**, dérivés de [`Self::graine`]
    /// une fois pour toutes (jamais tirés : voir [`champ::poles`]). C'est de
    /// là que viennent les régions actives où naissent les taches.
    poles: Vec<Pole>,
    /// Horloge d'animation, **décalée d'une étoile à l'autre** — voir
    /// [`temps_initial`].
    temps: f32,
    taches: Vec<Tache>,
    boucles: Vec<Boucle>,
    flares: Vec<Flare>,
    prochaine_tache: f32,
    prochaine_eruption: f32,
    prochain_flare: f32,
    // Réglages pilotables (sliders), 0..1
    freq: f32,
    forme: f32,
    puissance: f32,
    alea: f32,
    /// **Activité propre au type d'étoile** (§1.3) : `1,0` par défaut (les
    /// vues qui n'appellent pas [`Astre::set_eruptions`] gardent leur réglage
    /// tel quel), dérivée du gabarit par `etoile::activite_type` pour les
    /// étoiles générées. `set_eruptions` la **module**, ne l'écrase pas —
    /// sinon la même valeur figée continuerait de s'appliquer à toutes.
    activite: f32,
    couleur: Vec3,       // teinte de l'étoile (selon son type spectral)
    luminosite: f32,     // intensité lumineuse relative
    couronne: f32,       // extension de la couronne (× rayon) selon le type
    couronne_irreg: f32, // irrégularité (rayons/spicules) de la couronne
    couronne_type: f32,  // 0 = halo, 1 = jets bipolaires (pulsar), 2 = vent (WR)
    zone_hab: bool,      // false = rémanent (pulsar/magnétar/étoile à neutrons) : pas de zone habitable
    /// **Disque de matière autour de l'étoile** — accrétion d'un trou noir,
    /// disque protosolaire d'une jeune étoile.
    ///
    /// ⚠️ Un vrai [`Disque`], avec un **plan propre** : il se voit donc par
    /// la tranche quand on tourne autour, et il s'incline comme le reste du
    /// système. Le trou noir en portait un **peint à plat dans le quad**,
    /// ellipse d'inclinaison câblée qui présentait éternellement la même face
    /// à la caméra — impossible de faire le tour, impossible de le voir par
    /// la tranche.
    /// ⚠️ Un **vecteur** et non un seul : un trou noir vorace porte son
    /// disque d'accrétion **et** une ceinture de débris épars bien au-delà.
    /// Ce sont deux objets distincts, à deux échelles — les fondre en un
    /// donnerait une galette unique.
    disques: Vec<Disque>,
    mats: Vec<Material>,  // un par passe de `PASSES` (disque, puis couronne)
    mat_plasma: Material, // billboards en blending additif (plasma lumineux)
    halo: Texture2D,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
}

/// **Bornes du disque protosolaire**, en rayons stellaires. Bien plus large
/// que le disque d'accrétion : c'est un berceau de planètes, pas une
/// couronne de gaz surchauffé.
const RAYON_PROTOSOLAIRE: (f32, f32) = (3.0, 16.0);

/// **Normale du plan d'un disque stellaire, dérivée de la graine.**
///
/// ⚠️ Dérivée, jamais tirée : un `gen_range` de plus décalerait tout ce qui
/// est tiré ensuite et changerait le système entier à graine égale — le même
/// piège que les pôles magnétiques (`champ::poles`) et `temps_initial`.
fn normale_disque(graine: f32) -> Vec3 {
    // Deux angles sans rapport l'un avec l'autre, pris dans la graine.
    let incl = 0.25 + (graine * 0.37).fract() * 0.9; // ~14° à ~66° de la verticale
    let lon = (graine * 0.113).fract() * std::f32::consts::TAU;
    let (si, ci) = incl.sin_cos();
    vec3(si * lon.cos(), ci, si * lon.sin()).normalize_or(Vec3::Y)
}

/// **Instant de départ de l'animation**, dérivé de la graine de l'étoile.
///
/// Deux étoiles de même type, démarrées ensemble, bouillonneraient au même
/// rythme et verraient naître leurs taches en même temps : la graine seule
/// ne les distingue pas, puisqu'elle ne décale que l'espace, pas le temps.
///
/// **Dérivé plutôt que tiré une seconde fois** : « cette étoile est unique »
/// est *un* fait, il n'a droit qu'à une source (`docs/methode.md`). Et une
/// dérivation se teste, là où un second `gen_range` exigerait le contexte
/// graphique.
fn temps_initial(graine: f32) -> f32 {
    // Un demi-tour de graine : large devant les rythmes du spectacle (une
    // tache vit 8 à 16 s), sans partir si loin que les `f32` s'épaississent.
    graine * 0.5
}

impl Soleil {
    pub fn new(position: Vec3, rayon: f32, couleur: Vec3, luminosite: f32) -> Self {
        // Materials partagés (clonés) -> un seul pipeline chacun, même en régénérant.
        let mats = mats_soleil();
        let mat_plasma = mat_plasma_partage();

        let graine = gen_range(0.0, 1000.0);
        Self {
            base: CorpsBase::new(position, 1000.0, rayon),
            graine,
            poles: champ::poles(graine),
            temps: temps_initial(graine),
            taches: Vec::new(),
            boucles: Vec::new(),
            flares: Vec::new(),
            prochaine_tache: gen_range(0.5, 2.0),
            prochaine_eruption: gen_range(2.0, 4.0),
            prochain_flare: gen_range(1.5, 4.0),
            freq: 0.5,
            forme: 0.5,
            puissance: 0.5,
            alea: 0.5,
            activite: 1.0,
            couleur,
            luminosite,
            // Étoile chaude/lumineuse -> couronne plus large et plus irrégulière (vents stellaires).
            couronne: (1.25 + 0.14 * luminosite).min(2.1),
            couronne_irreg: ((luminosite - 0.4) / 3.6).clamp(0.0, 1.0),
            couronne_type: 0.0,
            zone_hab: true,
            disques: Vec::new(),
            mats,
            mat_plasma,
            halo: texture_halo(TAILLE_HALO),
            verts: Vec::new(),
            inds: Vec::new(),
        }
    }

    /// Couronne en jets bipolaires (pulsar, protoétoile) : 2 cônes le long de l'axe + disque.
    pub fn avec_jets(mut self) -> Self {
        self.couronne_type = 1.0;
        self.couronne = 5.0; // place pour de longs jets
        self.couronne_irreg = 0.0;
        self
    }

    /// Pulsar : jets bipolaires qui tournent (effet phare) + flash périodique.
    pub fn avec_pulsar(mut self) -> Self {
        self.couronne_type = 3.0;
        self.couronne = 5.0;
        self.couronne_irreg = 0.0;
        self
    }

    /// Magnétar : arcs de champ magnétique dipolaire brillants autour de l'étoile.
    pub fn avec_magnetar(mut self) -> Self {
        self.couronne_type = 4.0;
        self.couronne = 2.8;
        self
    }

    /// **Fixe l'activité propre au type d'étoile** (§1.3, `etoile::activite_type`).
    /// À poser à la génération, avant tout appel à `set_eruptions` : c'est ce
    /// facteur que celui-ci module au lieu d'écraser.
    pub fn avec_activite(mut self, activite: f32) -> Self {
        self.activite = activite.clamp(0.0, 1.0);
        self
    }

    /// Marque l'étoile comme rémanent compact (pulsar/magnétar/étoile à neutrons) :
    /// aucune zone habitable ne sera tracée (`zone_viable` renvoie `None`).
    pub fn sans_zone_habitable(mut self) -> Self {
        self.zone_hab = false;
        self
    }

    /// Couronne en vent stellaire épais et turbulent (Wolf-Rayet, supergéante bleue).
    pub fn avec_vent(mut self) -> Self {
        self.couronne_type = 2.0;
        self.couronne = 2.7; // enveloppe étendue
        self
    }

    /// Trou noir : horizon des événements noir + anneau de photons + disque d'accrétion
    /// incliné (Doppler + spirale). Rendu **stylisé** (pas de lentille gravitationnelle).
    pub fn avec_trou_noir(mut self) -> Self {
        self.couronne_type = 5.0;
        self.couronne = 3.4; // place pour le disque d'accrétion
        self.couronne_irreg = 0.0;
        // Le disque **est** la lumière d'un trou noir : il n'a pas de
        // photosphère. Il vient donc avec l'objet, jamais séparément.
        //
        // Le **régime** — endormi, actif ou vorace — se dérive de la graine :
        // deux trous noirs du même système ne sont pas au même stade de leur
        // repas. Jamais tiré, sous peine de décaler tout ce qui suit.
        let a = Accretion::regime(self.graine);
        self.avec_regime(a)
    }

    /// **Impose le régime** d'un trou noir au lieu de le dériver de sa
    /// graine.
    ///
    /// Pour les vues de travail — galerie et vue isolée — où l'on veut voir
    /// les trois côte à côte plutôt que de régénérer jusqu'à tomber sur
    /// celui qu'on cherche. En jeu, le régime reste **dérivé** : c'est une
    /// propriété de l'astre, pas un choix.
    ///
    /// ⚠️ Remplace les disques existants : sans ça, appeler deux régimes de
    /// suite les empilerait, et en additif l'éclat doublerait.
    pub fn avec_regime(mut self, a: Accretion) -> Self {
        self.disques.clear();
        let (r, masse) = (self.base.rayon, self.base.masse);
        let mut s = self.avec_disque(DisqueConfig::accretion_reglee(
            r * a.bornes.0,
            r * a.bornes.1,
            masse,
            a,
        ));
        if let Some((int, ext)) = a.ceinture {
            // Des cailloux, pas du gaz : le voile des ceintures, pas le
            // shader d'accrétion. C'est ce qui les distingue à l'œil.
            s = s.avec_disque(DisqueConfig::debris_recents(900, r * int, r * ext, masse));
        }
        s
    }

    /// **Disque protosolaire** d'une jeune étoile (T Tauri) : le berceau des
    /// planètes, encore là.
    ///
    /// Beaucoup plus large que le disque d'accrétion d'un trou noir, et sans
    /// beaming — sa matière est froide et lente. ⚠️ Il reste très en deçà de
    /// l'échelle réelle (une centaine d'UA) : à cette taille il couvrirait
    /// tout le système et masquerait ses planètes. Même exagération à
    /// l'envers que `ECHELLE_CORPS`.
    pub fn avec_disque_protosolaire(self) -> Self {
        let (r, masse) = (self.base.rayon, self.base.masse);
        let cfg = DisqueConfig::protosolaire(
            0,
            r * RAYON_PROTOSOLAIRE.0,
            r * RAYON_PROTOSOLAIRE.1,
            masse,
        );
        self.avec_disque(cfg)
    }

    /// **Retire le disque que l'astre porte de lui-même.**
    ///
    /// Pour la galerie des disques, qui catalogue le disque **à part** et le
    /// fournit elle-même : sans ça l'astre et la galerie en dessinent chacun
    /// un, et comme le disque d'accrétion est en blending **additif**, les
    /// deux s'ajoutent au lieu de se recouvrir — un disque deux fois trop
    /// brillant, sans rien pour le signaler.
    pub fn sans_disque(mut self) -> Self {
        self.disques.clear();
        self
    }

    /// Pose un disque autour de l'étoile, dans un plan **dérivé de sa
    /// graine** : deux trous noirs ne présentent pas la même face.
    pub fn avec_disque(mut self, cfg: DisqueConfig) -> Self {
        let cfg = cfg.avec_normale(normale_disque(self.graine));
        let mut d = Disque::new(cfg);
        d.corps_mut().position = self.base.position;
        self.disques.push(d);
        self
    }

    /// **Facteur d'éclat courant** : `1,0` hors sursaut, brièvement au-delà
    /// pendant un flare. C'est la seule trace du lot 3 qui se voit à
    /// **n'importe quelle échelle** (§1) — le ruban, lui, ne se lit qu'à
    /// partir de quelques pixels. Suit `self.temps`, la même horloge de
    /// spectacle que les taches et les boucles, jamais l'horloge orbitale.
    fn eclat(&self) -> f32 {
        1.0 + 0.6 * self.flares.iter().map(|f| f.intensite).sum::<f32>().min(1.0)
    }
}

/// Combine le réglage global (sliders UI, `Systeme::reglages_etoile`) et
/// l'activité propre de l'étoile : **module**, ne remplace pas. Écraser
/// aurait ramené toutes les étoiles au même réglage malgré `activite_type`
/// (§1.3) — exactement le défaut que ce lot corrige.
fn moduler(base: f32, activite: f32) -> f32 {
    (base * activite).clamp(0.0, 1.0)
}

impl Astre for Soleil {
    fn categorie(&self) -> Categorie {
        Categorie::Etoile
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn update(&mut self, dt: f32) {
        // Le disque s'anime au rythme du jeu, comme l'anneau d'une planète
        // (`Planete::update`). Sans ça son horloge reste à **zéro** : il se
        // dessine, mais il ne tourne pas — défaut signalé à l'écran, visible
        // en skymap et en vue isolée, et invisible en galerie des disques,
        // qui appelle `update` sur ses disques elle-même.
        //
        // ⚠️ **Ici et non dans `maj`** : celui-ci s'interrompt tôt pour les
        // trous noirs (pas d'activité de surface sur un horizon), et ce sont
        // précisément eux qui portent un disque. Placé après ce retour, le
        // disque d'un trou noir ne tournerait jamais.
        for d in &mut self.disques {
            d.update(dt);
        }
        self.maj(dt);
    }
    fn draw(&mut self, cam: &CameraInfo) {
        self.dessiner(cam);
    }
    /// Rayon élargi par le plancher en pixels ([`PX_MIN`]). C'est **la** source
    /// de la taille de l'étoile : `dessiner` en dérive le quad, les boucles et
    /// l'occultation, `Systeme::pick` sa cible de clic.
    fn rayon_dessine(&self, cam_pos: Vec3) -> f32 {
        crate::impostor::rayon_plancher(
            self.base.rayon,
            (cam_pos - self.base.position).length(),
            crate::impostor::viewport_h(),
            PX_MIN,
        )
    }
    /// `freq` et `puissance` gouvernent *combien* et *combien fort* l'étoile
    /// éruptionne : c'est ce que l'activité de type doit moduler. `forme` et
    /// `alea` ne font que dessiner la silhouette des arches, indépendamment
    /// du type — ils passent tels quels.
    fn set_eruptions(&mut self, freq: f32, forme: f32, puissance: f32, alea: f32) {
        self.freq = moduler(freq, self.activite);
        self.forme = forme;
        self.puissance = moduler(puissance, self.activite);
        self.alea = alea;
    }
    fn lumiere(&self) -> Option<Vec3> {
        Some(self.couleur * self.luminosite * self.eclat())
    }
    fn luminosite(&self) -> Option<f32> {
        Some(self.luminosite * self.eclat())
    }
    /// Une étoile se résume à sa couleur de corps noir : c'est déjà ce qui la
    /// distingue à l'œil (une naine rouge d'une géante bleue).
    fn teinte(&self) -> Option<Vec3> {
        Some(self.couleur)
    }
    fn zone_viable(&self) -> Option<(f32, f32)> {
        if !self.zone_hab {
            return None; // rémanent : pas de zone habitable à tracer
        }
        let (i, o) = crate::etoile::zone_habitable(self.luminosite);
        Some((i * crate::etoile::UA, o * crate::etoile::UA)) // UA -> unités monde
    }
}

#[cfg(test)]
mod tests {
    use super::PX_MIN;
    use crate::camera::DIST_DEFAUT;
    use crate::etoile::UA;
    use crate::genese::presets::R_SOLEIL;
    use crate::impostor::{px_rayon, rayon_plancher};

    /// Hauteur d'écran de référence, celle de la table du § P.3.
    const VH: f32 = 1080.0;

    /// **Le plancher tient sa promesse au cadrage qui a motivé sa création.**
    /// C'est là — et seulement là — que la décision a été prise : 1,14 px de
    /// rayon à l'échelle vraie.
    #[test]
    fn au_cadrage_par_defaut_l_etoile_atteint_son_plancher() {
        let vrai = px_rayon(R_SOLEIL, DIST_DEFAUT, VH);
        assert!(
            vrai < 2.0,
            "l'échelle vraie ne donne plus un point ({vrai} px) : le plancher n'a plus lieu d'être"
        );
        let dessine = px_rayon(rayon_plancher(R_SOLEIL, DIST_DEFAUT, VH, PX_MIN), DIST_DEFAUT, VH);
        assert!((dessine - PX_MIN).abs() < 1e-3, "{dessine} px au lieu de {PX_MIN}");
    }

    /// **Et il ne mange pas l'orbite de Mercure.**
    ///
    /// ⚠️ La garde existante (`presets.rs`, « le Soleil mange l'orbite de
    /// Mercure ») porte sur la **constante** `R_SOLEIL` et exige un quart de
    /// l'orbite. Un plancher agrandit au moment du DESSIN : il passe devant elle
    /// sans la déclencher. D'où cette garde-ci, sur la taille effectivement
    /// peinte.
    ///
    /// À 4 px (`PX_MIN`, jugé à l'œil le 2026-08-14 — 8 px faisait un Soleil
    /// trop gros au cadrage par défaut) le Soleil dessiné occupe **21 %** du
    /// rayon de l'orbite de Mercure (mesuré ; c'était 42 % à 8 px). Le seuil
    /// ci-dessous dit ce qui resterait acceptable, pas ce qui est mesuré :
    /// au-delà de la moitié, le disque toucherait visiblement l'orbite et le
    /// système cesserait de se lire.
    #[test]
    fn l_etoile_agrandie_reste_dans_l_orbite_de_mercure() {
        let dessine = rayon_plancher(R_SOLEIL, DIST_DEFAUT, VH, PX_MIN);
        let mercure = 0.39 * UA; // demi-grand axe, mêmes unités monde
        assert!(
            dessine < mercure * 0.5,
            "le Soleil dessiné ({dessine}) couvre {:.0} % du rayon de l'orbite de Mercure ({mercure})",
            dessine / mercure * 100.0
        );
    }

    /// **Le plancher est une exception, pas la règle.** Tout ce qui n'est pas
    /// une étoile garde l'échelle vraie : elle seule tombe sous le pixel au
    /// cadrage par défaut. Une Terre relevée de la même façon serait plus grosse
    /// que le Soleil, et `Systeme::pick` — qui lit `rayon_dessine` — donnerait à
    /// chaque caillou une cible de clic de huit pixels.
    ///
    /// Le test porte sur l'implémentation **par défaut** du trait, celle dont
    /// héritent planètes, lunes et disques. (`Planete::new` charge un material,
    /// donc un contexte GL : impossible à construire ici.)
    #[test]
    fn le_defaut_du_trait_rend_le_rayon_physique() {
        use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase};
        use macroquad::prelude::{vec3, Vec3};

        struct Caillou(CorpsBase);
        impl Astre for Caillou {
            fn categorie(&self) -> Categorie {
                Categorie::Planete
            }
            fn corps(&self) -> &CorpsBase {
                &self.0
            }
            fn corps_mut(&mut self) -> &mut CorpsBase {
                &mut self.0
            }
            fn update(&mut self, _dt: f32) {}
            fn draw(&mut self, _cam: &CameraInfo) {}
        }

        // Rayon d'une Terre : ~0,01 unité, très en deçà du pixel de loin.
        let terre = Caillou(CorpsBase::new(Vec3::ZERO, 1.0, 0.0102));
        for dist in [1.0_f32, DIST_DEFAUT, 30_000.0] {
            assert_eq!(
                terre.rayon_dessine(vec3(0.0, 0.0, dist)),
                0.0102,
                "à {dist} : une planète a été planchéiée"
            );
        }
    }
}

#[cfg(test)]
mod tests_disque {
    use super::*;

    /// **Chaque étoile présente son disque autrement.** Un plan figé donnerait
    /// des trous noirs tous vus sous le même angle — et c'est exactement ce
    /// que faisait l'ellipse peinte dans le quad, d'inclinaison câblée.
    ///
    /// ⚠️ **Dérivée, jamais tirée** : un `gen_range` de plus décalerait tout
    /// ce qui est tiré ensuite, et le système entier changerait à graine
    /// égale (même piège que `champ::poles` et `temps_initial`).
    #[test]
    fn le_plan_du_disque_depend_de_la_graine() {
        let a = normale_disque(12.5);
        assert_eq!(a, normale_disque(12.5), "deux reconstructions, deux plans");
        let b = normale_disque(880.25);
        assert!(a.dot(b).abs() < 0.999, "deux étoiles, le même plan : {a:?} / {b:?}");
    }

    /// **Et ce plan est utilisable** : unitaire, jamais dégénéré, et jamais
    /// couché à plat dans le plan de l'écran — un disque vu strictement par la
    /// tranche est invisible, un disque vu strictement de face ne montre pas
    /// qu'il a un plan.
    #[test]
    fn le_plan_du_disque_reste_franc() {
        for k in 0..400 {
            let graine = k as f32 * 2.51;
            let n = normale_disque(graine);
            assert!((n.length() - 1.0).abs() < 1e-4, "graine {graine} : normale non unitaire");
            let depuis_verticale = n.dot(Vec3::Y).clamp(-1.0, 1.0).acos();
            assert!(
                (0.1..1.5).contains(&depuis_verticale),
                "graine {graine} : disque à {depuis_verticale} rad de la verticale"
            );
        }
    }

    /// **Seul un disque d'accrétion porte le vocabulaire de l'accrétion.**
    ///
    /// Tout est rassemblé dans un type à part : c'est ce qui empêche ce
    /// vocabulaire de se répandre dans le `DisqueConfig` partagé avec les
    /// anneaux de Saturne, et c'est aussi ce qui permet à deux cibles
    /// visuelles de ne différer que par des constantes.
    #[test]
    fn seul_un_disque_d_accretion_porte_son_vocabulaire() {
        let acc = DisqueConfig::accretion_reglee(1.4, 3.0, 1000.0, Accretion::chaude());
        let a = acc.accretion.expect("un disque d'accrétion sans réglages");
        assert_eq!(acc.nb, 0, "des cailloux dans un disque d'accrétion");
        assert!(a.beta > 0.0, "sans beaming, c'est une collerette symétrique");
        assert!(a.pli > 0.0, "sans pli, la silhouette reste celle d'un anneau");
        assert!(a.nappes >= 1 && a.epaisseur > 0.0, "un disque d'épaisseur nulle");
        for autre in [
            DisqueConfig::protosolaire(0, 3.0, 16.0, 1000.0),
            DisqueConfig::protoplanetaire(0, 3.0, 16.0, 1000.0),
            DisqueConfig::anneau_saturne(1.2, 2.3, Vec3::ONE, 0.0),
            DisqueConfig::asteroides(10, 2.0, 3.0, 1000.0),
        ] {
            assert!(autre.accretion.is_none(), "un disque ordinaire se croit en accrétion");
        }
    }

    /// **Le bord interne est le plus chaud, et le profil est adouci.**
    ///
    /// La physique donne `I ∝ r^(-3)` ; appliquée telle quelle à l'opacité,
    /// elle éteint le bord externe au point qu'il disparaît. D'où **deux**
    /// exposants : raide pour la couleur, doux pour l'opacité.
    #[test]
    fn le_bord_interne_ecrase_sans_effacer_le_bord_externe() {
        let a = DisqueConfig::accretion_reglee(1.4, 3.0, 1000.0, Accretion::chaude()).accretion.unwrap();
        assert!(
            a.exposant_couleur > a.exposant_opacite,
            "la couleur ne s'effondre pas plus vite que l'opacité : le disque sera plat"
        );
        assert!(a.chaud.length() > a.froid.length(), "le bord interne n'est pas le plus chaud");
        // Ce qui reste d'opacité au bord externe, calculé comme le shader.
        let reste = (3.0_f32 / 1.4).powf(-a.exposant_opacite);
        assert!(
            (0.3..0.95).contains(&reste),
            "le bord externe garde {reste} de l'opacité interne : éteint ou plat"
        );
    }

    /// **Les cellules de bruit sont très allongées, et enroulées.**
    ///
    /// Un rapport d'aspect proche de 1 donne des bulles — c'était notre
    /// défaut. Il en faut 8:1 à 20:1, plus une spirale logarithmique, sans
    /// quoi les structures restent concentriques au lieu de s'enrouler. Et
    /// le compte azimutal doit être **entier** : c'est sur lui que le hash
    /// s'enroule pour qu'il n'y ait pas de couture.
    #[test]
    fn le_bruit_est_allonge_et_enroule() {
        let a = DisqueConfig::accretion_reglee(1.4, 3.0, 1000.0, Accretion::chaude()).accretion.unwrap();
        let aspect = a.cellules.x / a.cellules.y;
        assert!(aspect >= 4.0, "cellules d'aspect {aspect} : des bulles, pas des filaments");
        assert_eq!(a.cellules.y, a.cellules.y.floor(), "compte azimutal non entier : couture");
        assert!(a.cellules.y >= 1.0, "aucune cellule sur le tour");
        assert!(a.spirale > 0.0, "sans spirale, les structures restent concentriques");
    }

    /// **Le disque commence au-dessus de l'horizon, pas dessus.** La dernière
    /// orbite stable d'un trou noir de Schwarzschild vaut trois rayons ; un
    /// disque qui mordrait sur l'horizon se lirait comme une collerette collée
    /// à la sphère.
    ///
    /// ⚠️ Balaie **les trois régimes** : les bornes sont portées par le
    /// preset depuis qu'un trou noir vorace a un disque plus large qu'un
    /// endormi, donc la règle doit valoir pour chacun.
    #[test]
    fn le_disque_d_accretion_degage_l_horizon() {
        let regimes = [
            ("endormie", Accretion::endormie()),
            ("chaude", Accretion::chaude()),
            ("vorace", Accretion::vorace()),
        ];
        for (nom, a) in regimes {
            let (int, ext) = a.bornes;
            assert!(int > 1.0, "{nom} : le disque mord sur l'ombre");
            // ⚠️ Et il la **serre** : r_ISCO / r_ombre = 6/5,196 = 1,155. Un
            // bord interne trop loin transforme le disque en anneau lointain
            // autour d'une bille — c'est ce qu'on voyait.
            assert!(int < 1.3, "{nom} : bord interne à {int}, contre 1,155 — le disque décolle");
            assert!(ext > int * 1.5, "{nom} : disque trop fin pour se lire");
            // Le protosolaire est franchement plus large : ce n'est pas le
            // même objet, il ne doit pas s'y confondre à l'œil.
            assert!(RAYON_PROTOSOLAIRE.1 > ext * 2.0, "{nom} : aussi large qu'un protosolaire");
            // Et la ceinture, quand il y en a une, est **bien au-delà** du
            // disque : deux objets, pas un dégradé.
            if let Some((ci, _)) = a.ceinture {
                assert!(ci > ext * 1.3, "{nom} : la ceinture colle au disque, ils se confondront");
            }
        }
    }
}

#[cfg(test)]
mod tests_graine {
    use super::*;

    /// **Deux étoiles différentes ne démarrent pas au même instant.**
    ///
    /// Sans ce décalage, la graine ne sert qu'à décaler le motif dans
    /// l'espace : les deux surfaces bouillonneraient encore au même rythme,
    /// et un binaire resterait visiblement jumeau.
    ///
    /// ⚠️ Ce qui ne se teste pas ici : que `Soleil::new` **tire** bien une
    /// graine, et que le shader s'en serve. Construire un `Soleil` crée ses
    /// materials, donc exige un contexte graphique — aucun test du dépôt ne
    /// le peut. La correspondance uniform ↔ `.glsl` est gardée ailleurs
    /// (`materiau.rs`, `uniforms_soleil_correspondent_au_shader`) ; le reste
    /// se juge à l'œil, en galerie.
    #[test]
    fn deux_graines_donnent_deux_instants_de_depart() {
        assert_ne!(temps_initial(0.0), temps_initial(1.0));
        assert_ne!(temps_initial(120.0), temps_initial(880.0));
        // Même graine, même départ : la reconstruction d'un système à graine
        // fixe doit redonner exactement la même étoile (déterminisme).
        assert_eq!(temps_initial(42.0), temps_initial(42.0));
    }

    /// Le décalage reste dans une plage utile : assez large pour désynchroniser
    /// des rythmes qui se comptent en secondes, assez petit pour que l'horloge
    /// garde sa précision.
    #[test]
    fn le_decalage_reste_dans_une_plage_utile() {
        for graine in [0.0_f32, 1.0, 500.0, 1000.0] {
            let t = temps_initial(graine);
            assert!((0.0..=500.0).contains(&t), "graine {graine} -> {t}, hors plage");
        }
        // Et il couvre vraiment cette plage : un décalage écrasé vers zéro
        // ne désynchroniserait rien.
        assert!(temps_initial(1000.0) - temps_initial(0.0) > 100.0);
    }
}

#[cfg(test)]
mod tests_moduler {
    use super::moduler;

    /// **La modulation ne doit ni annuler ni inverser la hiérarchie**
    /// d'`etoile::activite_type` : à réglage égal, l'étoile la plus active
    /// (une naine M) reste au-dessus de la moins active (une géante).
    #[test]
    fn preserve_la_hierarchie_d_activite() {
        let base = 0.6;
        let naine_m = moduler(base, 0.9); // activité haute
        let geante = moduler(base, 0.15); // activité basse (plancher d'`activite_type`)
        assert!(naine_m > geante, "hiérarchie inversée : naine {naine_m} <= géante {geante}");
    }

    /// **N'annule pas** : une activité non nulle (même au plancher) laisse
    /// toujours un réglage non nul passer — une géante flambe rarement, pas
    /// jamais.
    #[test]
    fn n_annule_pas_a_activite_non_nulle() {
        assert!(moduler(0.5, 0.15) > 0.0);
    }

    /// Borné comme un réglage 0..1, même si `base` et `activite` le sont déjà.
    #[test]
    fn reste_borne() {
        for base in [0.0_f32, 0.5, 1.0] {
            for activite in [0.0_f32, 0.15, 1.0] {
                let m = moduler(base, activite);
                assert!((0.0..=1.0).contains(&m), "base {base}, activite {activite} -> {m}");
            }
        }
    }
}
