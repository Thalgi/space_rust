// La source unique d'où découlent taches (sous-lot 3.2), pieds d'arche (3.3)
// et assombrissement du shader (3.4) — `docs/conception/astres_v2.md` §6 bis.
mod champ;
mod eruptions;
mod magnetosphere;
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
/// **Le material des rubans lumineux** (plasma additif, sans écriture de
/// profondeur), partagé : les arches du soleil, et les queues de comète.
pub fn materiau_ruban_lumineux() -> Material {
    mat_plasma_partage()
}

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
    /// **Pulsation** : `(amplitude relative du rayon, période en secondes)`.
    /// `(0, _)` : l'étoile ne pulse pas, et c'est le cas de presque toutes.
    /// Dérivée du type par `etoile::pulsation`, appliquée par
    /// `Variante::appliquer`.
    pulse: (f32, f32),
    /// **Ébullition de surface** (2026-10-19) : `0` garde la granulation
    /// fine, au-delà la surface devient de grosses cellules qui gonflent et
    /// éclatent, et l'étoile gonfle de −2 % à +3 %. `1` = cellules géantes,
    /// `0,5` = plus nombreuses. Dérivée du type par `etoile::ebullition`.
    ebullition: f32,
    /// [`EBUL_PLAT`], [`EBUL_RELIEF`] ou [`EBUL_CONTOUR`].
    ebul_style: f32,
    /// Ses variations d'éclat (flares, pulsation) se voient-elles **à
    /// l'image** ? Dérivé du type par `etoile::eclat_visible` : la céphéide
    /// seule pour l'instant. Sinon, elles n'éclairent que les planètes.
    eclat_visible: bool,
    /// **Surface propre au type** (lot 10) : `(échelle du grain, contraste du
    /// grain, assombrissement du bord)`, dérivée du type par `etoile::surface`.
    /// La valeur par défaut `(11, 1, 0)` est **exactement** le rendu d'avant
    /// le lot — celui d'une étoile construite sans passer par son type.
    surface: Vec3,
    /// **Réglages de l'ébullition** (lot 10) : `(finesse des cellules, clarté
    /// minimale)`, dérivés du type par `etoile::reglage_ebullition`. `(1, 0,5)`
    /// est exactement le rendu d'avant.
    ebul_reglage: Vec2,
    /// **Lueur du cœur** (lot 11) : la quasi-étoile est chauffée par un trou
    /// noir central ; sa chaleur transparaît au centre du disque. Dérivée du
    /// type par `etoile::coeur` ; 0 pour toutes les autres.
    coeur: f32,
    /// **Jets en 3D** (lot 12) : longueur en rayons, 0 = aucun. Ils suivent
    /// l'**axe du disque** (`normale_disque`, la même source) au lieu d'être
    /// peints le long de l'axe vertical de l'écran — ce qui les faisait partir
    /// de travers dès que le disque s'inclinait. Posés par la variante `Proto`.
    jets_3d: f32,
    /// **Faisceaux de pulsar en 3D** (lot 13) : `(longueur en rayons,
    /// retard au bout en rad)`. Longueur 0 = aucun.
    faisceaux: Vec2,
    /// **Boucles magnétiques en 3D** (lot 13, magnétar) : nombre de lignes,
    /// torsion, et période des sursauts (0 = aucun).
    boucles_mag: u32,
    torsion_mag: f32,
    sursauts_mag: f32,
    /// Inclinaison de l'axe magnétique sur l'axe de rotation (rad), et vitesse
    /// de rotation (rad/s) des rémanents du lot 13.
    inclinaison_mag: f32,
    rotation_mag: f32,
    /// **Inclinaison de l'axe de rotation** (lot 12) : amène le pôle
    /// nord de la surface sur l'axe réel de l'étoile. Identité pour presque
    /// toutes ; pour une étoile à jets, l'axe de son disque — une étoile tourne
    /// autour de l'axe de son disque, et ses jets sortent par ses pôles.
    axe_rot: Quat,
    /// **Taches d'accrétion polaires** (lot 12) : 0 = aucune.
    accretion_pol: f32,
    /// **Aplatissement** (lot 12, l'étoile Be) : le rayon polaire vaut
    /// `1 − aplatissement` fois l'équatorial. 0 = une sphère.
    aplatissement: f32,
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
///
/// Lot 12 (2026-10-24) : de 3–16 à **5–48**. Le bord interne est la cavité
/// magnétosphérique (le champ tronque le disque, 3 à 8 rayons chez une
/// T Tauri) ; l'externe, la largeur retenue à l'écran (T1, *« plus large »*).
/// `habillage::jeune_etoile` affine la cavité d'après la graine.
pub(crate) const RAYON_PROTOSOLAIRE: (f32, f32) = (5.0, 48.0);

/// **Normale du plan d'un disque stellaire, dérivée de la graine.**
///
/// ⚠️ Dérivée, jamais tirée : un `gen_range` de plus décalerait tout ce qui
/// est tiré ensuite et changerait le système entier à graine égale — le même
/// piège que les pôles magnétiques (`champ::poles`) et `temps_initial`.
/// **Longueur des jets d'une jeune étoile**, en rayons (lot 12, jugé à
/// l'écran).
pub const JETS_PROTO: f32 = 3.0;
/// **Force des taches d'accrétion polaires** d'une jeune étoile (lot 12).
pub const ACCRETION_PROTO: f32 = 0.8;

/// **La règle des variations d'éclat** : elles ne passent que pour un type
/// dont l'éclat varie à l'œil. Libre pour être gardée sans contexte graphique.
fn variation_si_visible(visible: bool, variation: f32) -> f32 {
    if visible { variation } else { 1.0 }
}

/// **Le quaternion qui amène le pôle nord de la surface (+Y) sur `axe`.**
pub(super) fn quat_axe(axe: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::Y, axe.normalize_or(Vec3::Y))
}

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
/// **Amplitude maximale d'une pulsation**, en part du rayon.
///
/// ⚠️ Ce n'est pas un goût, c'est une borne de sûreté. La pulsation traverse
/// `rayon_dessine`, source unique de la taille de l'étoile **et** de la cible de
/// clic de `Systeme::pick` : à 1,0 le facteur toucherait zéro et l'étoile
/// disparaîtrait un instant, en emportant sa cible. La marge orbitale de
/// `genese::premiere_orbite` (1,5 rayon) donne la même borne par l'autre bout.
const AMP_PULSE_MAX: f32 = 0.5;

/// **Facteur de pulsation** : `1,0` sans pulsation, sinon une sinusoïde autour
/// de 1.
///
/// ⚠️ **Fonction libre, et l'écrêtage est ici.** `Soleil::new` charge des
/// materials, donc exige le contexte graphique : une borne posée au constructeur
/// ou dans une méthode ne se testerait pas, et c'est précisément la borne qui
/// empêche un rayon nul. Elle vit donc au seul endroit qu'aucun appelant ne peut
/// contourner, et que les tests atteignent.
fn facteur_pulsation(amplitude: f32, periode: f32, temps: f32) -> f32 {
    let amp = amplitude.clamp(0.0, AMP_PULSE_MAX);
    if amp <= 0.0 {
        return 1.0;
    }
    1.0 + amp * (std::f32::consts::TAU * temps / periode.max(1e-3)).sin()
}

/// **Styles d'ébullition**, que le shader lit dans `ebul_style`.
pub const EBUL_PLAT: f32 = 0.0;
/// La chaleur sert de hauteur : chaque bulle a un flanc éclairé, un flanc ombré.
pub const EBUL_RELIEF: f32 = 1.0;
/// Le bord du disque se soulève là où une grosse bulle passe sur le limbe.
pub const EBUL_CONTOUR: f32 = 2.0;

/// **Part des étoiles bouillonnantes qui sont instables.** Choix par défaut,
/// faute d'enjeu : assez pour en croiser, pas au point d'en faire la règle.
const PART_INSTABLE: f32 = 0.35;

/// **Cette étoile est-elle instable ?** Dérivé de la graine, jamais tiré : un
/// `gen_range` de plus décalerait tout ce qui suit et changerait le système
/// entier à graine égale — même piège que [`normale_disque`].
fn est_instable(graine: f32) -> bool {
    (graine * 0.618_034).fract() < PART_INSTABLE
}

/// **Gonflement d'une surface en ébullition** : de −2 % à +3 % du rayon, au
/// hasard plutôt qu'en cadence. Trois sinus de fréquences sans commune mesure :
/// leur somme ne repasse jamais par le même motif, et c'est ce qui la sépare de
/// la respiration régulière d'une céphéide. La graine décale l'horloge : deux
/// étoiles ne gonflent pas ensemble.
/// **Exposant de l'éclat d'une pulsation** : l'éclat suit le rayon à la
/// puissance 4, et non au carré.
///
/// ⚠️ `L = 4πR²σT⁴` ne donne `L ∝ R²` qu'à température **constante**. Or une
/// étoile pulsante chauffe en se comprimant et se refroidit en se dilatant :
/// δ Cephei varie d'environ ±9 % en température, et son éclat d'un facteur
/// **~2,3** — quand le carré du rayon (±10 %) n'en donnait que 1,5. La
/// température est le plus gros des deux termes. Au carré seul, la céphéide
/// ne variait que de ±20 %, phase brillante saturée au blanc : *« j'ai pas la
/// variation d'éclat de loin »* (2026-10-20). À la puissance 4, `(1,1/0,9)⁴`
/// rend 2,2 : le facteur réel.
const EXPOSANT_ECLAT_PULSATION: f32 = 4.0;

/// **Éclat relatif d'une étoile qui pulse et bouillonne** : la pulsation à la
/// puissance [`EXPOSANT_ECLAT_PULSATION`] (rayon et température), le
/// gonflement d'ébullition au **carré** seulement — il n'a pas de cycle de
/// température, et le relever changerait les géantes rouges validées.
fn lumiere_pulsee(f_pulsation: f32, f_ebullition: f32) -> f32 {
    f_pulsation.powf(EXPOSANT_ECLAT_PULSATION) * f_ebullition * f_ebullition
}

/// **Éclat par unité de surface dessinée**, ce que reçoit le shader.
///
/// ⚠️ **La conservation du flux.** Ce qu'on voit d'une étoile, c'est son éclat
/// **total** : surface dessinée × éclat de surface. Quand elle est assez grande
/// à l'écran, son rayon dessiné suit déjà la pulsation, et la surface porte une
/// part de la variation. Sous le plancher en pixels (`PX_MIN`), le rayon
/// dessiné est **figé** : sans compensation, la pulsation disparaissait
/// entièrement de l'image vue de loin — exactement là où elle devait se voir.
///
/// `lumiere` : éclat total relatif ; `r_repos` : rayon dessiné **sans**
/// pulsation ; `r_dessine` : rayon dessiné avec. Le rapport des surfaces dit
/// quelle part la taille porte déjà, le reste revient à l'éclat de surface.
fn eclat_surfacique(lumiere: f32, r_repos: f32, r_dessine: f32) -> f32 {
    let k = r_repos / r_dessine.max(1e-6);
    lumiere * k * k
}

fn facteur_ebullition(graine: f32, temps: f32) -> f32 {
    let t = temps + graine;
    let s = (t * 0.53).sin() + 0.6 * (t * 0.97 + 1.3).sin() + 0.3 * (t * 1.71 + 2.1).sin();
    // s ∈ [−1,9 ; 1,9], ramené sur [0,98 ; 1,03].
    1.005 + 0.025 * s / 1.9
}

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
            pulse: (0.0, 1.0),
            ebullition: 0.0,
            ebul_style: 0.0,
            eclat_visible: false,
            surface: vec3(11.0, 1.0, 0.0),
            ebul_reglage: vec2(1.0, 0.5),
            coeur: 0.0,
            aplatissement: 0.0,
            jets_3d: 0.0,
            faisceaux: Vec2::ZERO,
            boucles_mag: 0,
            torsion_mag: 0.0,
            sursauts_mag: 0.0,
            inclinaison_mag: 0.0,
            rotation_mag: 0.0,
            axe_rot: Quat::IDENTITY,
            accretion_pol: 0.0,
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

    /// **Faisceaux de pulsar en 3D** (lot 13) : deux faisceaux de `longueur`
    /// rayons le long d'un axe magnétique incliné de `inclinaison`, qui tourne
    /// à `rotation` rad/s autour de l'axe propre de l'étoile ; ils se courbent
    /// de `retard` rad au bout. Remplace la couronne peinte dans l'écran.
    pub fn avec_faisceaux(mut self, longueur: f32, retard: f32, inclinaison: f32, rotation: f32) -> Self {
        self.faisceaux = vec2(longueur.max(0.0), retard);
        self.inclinaison_mag = inclinaison;
        self.rotation_mag = rotation;
        self.axe_rot = quat_axe(normale_disque(self.graine));
        self
    }

    /// **Boucles magnétiques en 3D** (lot 13, magnétar) : `n` lignes fermées
    /// du dipôle, tordues de `torsion` rad, et un sursaut toutes les
    /// `sursauts` secondes (0 = aucun).
    pub fn avec_boucles_magnetiques(mut self, n: u32, torsion: f32, sursauts: f32, inclinaison: f32) -> Self {
        self.boucles_mag = n;
        self.torsion_mag = torsion;
        self.sursauts_mag = sursauts;
        self.inclinaison_mag = inclinaison;
        self.rotation_mag = 0.3;
        self.axe_rot = quat_axe(normale_disque(self.graine));
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

    /// **Fait pulser l'étoile** : `amplitude` relative du rayon, `periode` en
    /// secondes. Dérivée du type par `etoile::pulsation`, posée par
    /// `Variante::appliquer` — l'unique endroit qui configure une étoile
    /// d'après son type.
    ///
    /// ⚠️ Une amplitude nulle ou une période nulle laissent l'étoile immobile :
    /// la valeur par défaut est donc inerte, et le cas « ne pulse pas » n'a
    /// besoin d'aucun bras.
    pub fn avec_pulsation(mut self, amplitude: f32, periode: f32) -> Self {
        // Rangée telle quelle : c'est [`facteur_pulsation`] qui écrête, parce
        // que lui se teste.
        self.pulse = (amplitude, periode);
        self
    }

    /// **Surface en ébullition** : voir les champs `ebullition` et `ebul_style`.
    /// Le jeu passe par [`Self::avec_ebullition_du_type`] ; seule la galerie
    /// l'appelle directement, pour montrer les deux aspects côte à côte.
    pub fn avec_ebullition(mut self, e: f32, style: f32) -> Self {
        self.ebullition = e.clamp(0.0, 1.0);
        self.ebul_style = style;
        self
    }

    /// **Étendue de la couronne**, en rayons. Elle se déduit d'ordinaire de la
    /// luminosité ; la quasi-étoile (lot 11) est très lumineuse mais veut une
    /// couronne serrée — d'où ce réglage à part, qui ne touche pas à l'éclat.
    pub fn avec_couronne(mut self, etendue: f32) -> Self {
        self.couronne = etendue.max(1.05);
        self
    }

    /// **Jets en 3D** (lot 12) : remplace les jets peints dans le plan
    /// de l'écran (`avec_jets`) par des jets le long de l'axe du disque.
    /// Couronne remise au halo ordinaire : les jets ne sont plus dans le quad.
    pub fn avec_jets_3d(mut self, longueur: f32) -> Self {
        self.jets_3d = longueur.max(0.0);
        // Les jets sortent par les pôles : l'étoile tourne autour de l'axe de
        // son disque, pas autour de la verticale du monde.
        self.axe_rot = quat_axe(normale_disque(self.graine));
        self.couronne_type = 0.0;
        self.couronne = 1.4;
        self.couronne_irreg = 0.0;
        self
    }

    /// **Taches d'accrétion polaires** (lot 12) : voir le champ.
    /// **Une étoile Be** (lot 12) : aplatie, son disque de décrétion autour,
    /// variés par sa graine (`habillage::etoile_be`).
    pub fn avec_disque_be(self) -> Self {
        let (aplati, c) = crate::disque::habillage::etoile_be(self.base.rayon, self.base.masse, self.graine);
        self.avec_aplatissement(aplati).avec_disque(c)
    }

    /// **Une hypergéante dans sa nébuleuse** (lot 12) : jupe équatoriale et
    /// lobes en bulbe, variés par sa graine (`habillage::hypergeante`).
    pub fn avec_homoncule(self) -> Self {
        let c = crate::disque::habillage::hypergeante(self.base.rayon, self.base.masse, self.graine);
        self.avec_disque(c)
    }

    /// **Une étoile qui tourne si vite qu'elle s'aplatit** (l'étoile Be) :
    /// rayon polaire de `1 − f` fois l'équatorial — Achernar, la plus aplatie
    /// connue, est vers 0,35. Elle tourne autour de l'axe de son disque, comme
    /// une étoile à jets ; ses pôles, plus chauds, sont plus clairs
    /// (assombrissement gravitationnel, von Zeipel).
    pub fn avec_aplatissement(mut self, f: f32) -> Self {
        self.aplatissement = f.clamp(0.0, 0.6);
        self.axe_rot = quat_axe(normale_disque(self.graine));
        self
    }

    pub fn avec_accretion_polaire(mut self, force: f32) -> Self {
        self.accretion_pol = force.clamp(0.0, 1.0);
        self
    }

    /// L'axe des jets : celui du disque, que l'étoile en porte un ou non.
    pub fn axe_jets(&self) -> Vec3 {
        normale_disque(self.graine)
    }

    /// Voir le champ `coeur`.
    pub fn avec_coeur(mut self, intensite: f32) -> Self {
        self.coeur = intensite.clamp(0.0, 1.0);
        self
    }

    /// Voir le champ `ebul_reglage`.
    pub fn avec_reglage_ebullition(mut self, finesse: f32, clarte_min: f32) -> Self {
        self.ebul_reglage = vec2(finesse.max(0.1), clarte_min.clamp(0.0, 0.9));
        self
    }

    /// Voir le champ `surface`.
    pub fn avec_surface(mut self, echelle: f32, contraste: f32, assombrissement: f32) -> Self {
        self.surface = vec3(echelle, contraste.clamp(0.0, 1.0), assombrissement.clamp(0.0, 1.0));
        self
    }

    /// Voir le champ `eclat_visible`.
    pub fn avec_eclat_visible(mut self, oui: bool) -> Self {
        self.eclat_visible = oui;
        self
    }

    /// **L'ébullition propre au type**, stable ou instable selon la graine.
    pub fn avec_ebullition_du_type(self, nom: &str) -> Self {
        let (e, style) = crate::etoile::ebullition(nom, est_instable(self.graine));
        let (finesse, clarte) = crate::etoile::reglage_ebullition(nom);
        self.avec_ebullition(e, style).avec_reglage_ebullition(finesse, clarte)
    }

    /// **La surface propre au type** (lot 10) : voir `etoile::surface`.
    pub fn avec_surface_du_type(self, nom: &str) -> Self {
        let (echelle, contraste, bord) = crate::etoile::surface(nom);
        self.avec_surface(echelle, contraste, bord)
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
    /// **tout le catalogue** côte à côte plutôt que de régénérer jusqu'à
    /// tomber sur celui qu'on cherche. En jeu, le preset reste **dérivé** de
    /// la graine : c'est une propriété de l'astre, pas un choix.
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
        // Des cailloux, pas du gaz : le voile des ceintures, pas le shader
        // d'accrétion. C'est ce qui les distingue à l'œil.
        if let Some(c) = a.config_ceinture(r, masse) {
            s = s.avec_disque(c);
        }
        s
    }

    /// **Le disque d'une jeune étoile, habillé** (lot 12) : nuages en
    /// volume, bras de matière et entonnoirs — T1B4 pour une T Tauri, varié
    /// par la graine ; T1B2 pour une Herbig Ae/Be. Voir
    /// `disque::habillage::jeune_etoile`.
    ///
    /// Beaucoup plus large que le disque d'accrétion d'un trou noir, et sans
    /// beaming — sa matière est froide et lente. ⚠️ Il reste très en deçà de
    /// l'échelle réelle (une centaine d'UA) : à cette taille il couvrirait
    /// tout le système et masquerait ses planètes. Même exagération à
    /// l'envers que `ECHELLE_CORPS`.
    pub fn avec_disque_jeune(self, herbig: bool) -> Self {
        let cfg = crate::disque::habillage::jeune_etoile(self.config_protosolaire(), self.base.rayon, self.graine, herbig);
        self.avec_disque(cfg)
    }

    /// Le disque protosolaire de cette étoile, avant d'être posé : de quoi
    /// l'habiller (prototypes T Tauri, lot 12.2).
    pub fn config_protosolaire(&self) -> DisqueConfig {
        let (r, masse) = (self.base.rayon, self.base.masse);
        DisqueConfig::protosolaire(0, r * RAYON_PROTOSOLAIRE.0, r * RAYON_PROTOSOLAIRE.1, masse)
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

    /// **Facteur de pulsation courant** : `1,0` pour une étoile qui ne pulse
    /// pas, sinon une sinusoïde autour de 1.
    ///
    /// Suit `self.temps`, la même horloge de spectacle que les taches, les
    /// boucles et l'éclat — et **décalée d'une étoile à l'autre** par
    /// [`temps_initial`], si bien que deux céphéides d'un binaire ne battent pas
    /// à l'unisson. C'est exactement la raison pour laquelle ce décalage existe.
    ///
    /// ⚠️ Reste **strictement positif** : l'amplitude est bornée à 0,5 par
    /// [`Self::avec_pulsation`], donc le facteur ne descend jamais sous 0,5. Un
    /// rayon nul ou négatif ferait disparaître l'étoile et renverrait une cible
    /// de clic vide.
    pub(super) fn facteur_pulse(&self) -> f32 {
        // L'ébullition passe par ici pour la même raison que la pulsation :
        // `rayon_dessine` et la cible de clic en dérivent.
        let ebul = if self.ebullition > 0.0 { facteur_ebullition(self.graine, self.temps) } else { 1.0 };
        facteur_pulsation(self.pulse.0, self.pulse.1, self.temps) * ebul
    }

    /// **Part de la pulsation dans l'éclat** : le carré du facteur de rayon.
    ///
    /// À température constante, `L = 4πR²σT⁴` donne `L ∝ R²`. Un seul réglage
    /// gouverne donc les deux — et c'est la raison pour laquelle
    /// `etoile::pulsation` n'a pas d'amplitude de luminosité à part.
    fn pulse_lumiere(&self) -> f32 {
        let ebul = if self.ebullition > 0.0 { facteur_ebullition(self.graine, self.temps) } else { 1.0 };
        lumiere_pulsee(facteur_pulsation(self.pulse.0, self.pulse.1, self.temps), ebul)
    }

    /// **Ce qui fait varier l'éclat** (flares, pulsation, ébullition) — 1 pour
    /// un type dont l'éclat ne varie pas à l'œil. Source unique de la lumière
    /// projetée (`lumiere`) ; l'image, elle, passe par `eclat_dessin`, fermé
    /// par la même règle.
    fn variation_eclat(&self) -> f32 {
        variation_si_visible(self.eclat_visible, self.eclat() * self.pulse_lumiere())
    }

    /// **Éclat de surface à envoyer au shader** : flare et pulsation, compensés
    /// de ce que la taille dessinée porte déjà (voir [`eclat_surfacique`]).
    pub(super) fn eclat_dessin(&self, cam_pos: Vec3) -> f32 {
        if !self.eclat_visible {
            return 1.0;
        }
        let r_repos = crate::impostor::rayon_plancher(
            self.base.rayon,
            (cam_pos - self.base.position).length(),
            crate::impostor::viewport_h(),
            PX_MIN,
        );
        eclat_surfacique(self.eclat() * self.pulse_lumiere(), r_repos, self.rayon_dessine(cam_pos))
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
            // ⚠️ **La pulsation entre ici, et nulle part ailleurs.** C'est la
            // source unique de la taille de l'étoile : le quad, les boucles,
            // l'occultation et la **cible de clic** de `Systeme::pick` en
            // dérivent tous. La conception annonçait le piège — une pulsation
            // du rayon bouge la cible de clic — et la parade est qu'elle la
            // bouge *avec* le disque dessiné, jamais en désaccord avec lui.
            //
            // `Systeme::pick` prend en plus 40 % de marge : à ±20 % d'amplitude,
            // le rayon cliquable reste toujours au-dessus du rayon peint. Un
            // clic posé sur le disque visible atteint donc l'étoile à tout
            // instant de son cycle, et seule la marge respire.
            self.base.rayon * self.facteur_pulse(),
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
    /// ⚠️ **La pulsation y passe** (à la puissance 4 du rayon, voir
    /// `lumiere_pulsee`) — **mais seulement pour un type dont l'éclat varie à
    /// l'œil** (`eclat_visible` : la céphéide, aujourd'hui).
    ///
    /// Jusqu'au 2026-10-22, toute étoile faisait varier la lumière qu'elle
    /// jette : chaque flare (+60 %) et chaque souffle d'ébullition éclairaient
    /// et assombrissaient d'un coup tout ce qui l'entoure — mesuré sur le
    /// disque d'un prototype, −14 % d'une image à l'autre, *« un effet de
    /// battement de cœur »*. La décision du lot 7 (l'éclat ne varie à l'image
    /// que pour les types qui le font vraiment) vaut donc aussi pour la lumière
    /// projetée : une seule règle, les deux canaux.
    fn lumiere(&self) -> Option<Vec3> {
        Some(self.couleur * self.luminosite * self.variation_eclat())
    }
    fn luminosite(&self) -> Option<f32> {
        Some(self.luminosite * self.variation_eclat())
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

    /// **La pulsation oscille autour de 1, et ne s'annule jamais.**
    ///
    /// ⚠️ Elle traverse `rayon_dessine`, la source unique de la taille de
    /// l'étoile — donc aussi la **cible de clic** de `Systeme::pick`. Un facteur
    /// nul ou négatif ferait disparaître l'étoile et rendrait une cible vide ;
    /// c'est pour ça que l'amplitude est bornée à 0,5 et non laissée libre.
    ///
    /// L'éclat, lui, suit le **carré** : `L ∝ R²` à température constante.
    #[test]
    fn la_pulsation_oscille_autour_de_un_sans_jamais_s_annuler() {
        use super::{facteur_pulsation, AMP_PULSE_MAX};
        // Sans pulsation, exactement 1 : le cas par défaut doit être **inerte**,
        // sans quoi toutes les étoiles du jeu respireraient.
        for periode in [0.0_f32, 1.0, 9.0] {
            assert_eq!(facteur_pulsation(0.0, periode, 3.7), 1.0);
        }
        // ⚠️ Amplitude demandée bien au-delà de la borne : elle doit être
        // **écrêtée**, pas obéie. C'est la seule protection contre un rayon nul.
        for amp in [0.1_f32, 0.2, 9.0] {
            let (mut bas, mut haut) = (f32::INFINITY, f32::NEG_INFINITY);
            for k in 0..600 {
                let f = facteur_pulsation(amp, 4.0, k as f32 * 0.02);
                assert!(f > 0.0, "amp {amp} : facteur {f}, l'étoile disparaît avec sa cible de clic");
                bas = bas.min(f);
                haut = haut.max(f);
            }
            // Le cycle passe des deux côtés de 1 : ce qui ne ferait que gonfler
            // serait un gonflement, pas une pulsation.
            assert!(bas < 0.999 && haut > 1.001, "amp {amp} : cycle incomplet, {bas} à {haut}");
            let borne = amp.min(AMP_PULSE_MAX);
            assert!(bas >= 1.0 - borne - 1e-4, "amp {amp} : descend à {bas}");
            assert!(haut <= 1.0 + borne + 1e-4, "amp {amp} : monte à {haut}");
        }
        // Une période nulle ne divise pas par zéro.
        assert!(facteur_pulsation(0.2, 0.0, 1.0).is_finite());
    }

    /// **Le gonflement d'ébullition reste entre −2 % et +3 %**, et passe des
    /// deux côtés de 1.
    #[test]
    fn l_ebullition_gonfle_entre_moins_deux_et_plus_trois_pour_cent() {
        use super::facteur_ebullition;
        let (mut bas, mut haut) = (f32::INFINITY, f32::NEG_INFINITY);
        for k in 0..20000 {
            let f = facteur_ebullition(123.4, k as f32 * 0.05);
            bas = bas.min(f);
            haut = haut.max(f);
        }
        assert!(bas >= 0.98 - 1e-4 && haut <= 1.03 + 1e-4, "{bas} à {haut}");
        assert!(bas < 0.99 && haut > 1.02, "trop sage : {bas} à {haut}");
    }

    /// **La pulsation se voit de loin** : sous le plancher en pixels, où la
    /// taille dessinée est figée, l'éclat de surface porte **toute** la
    /// variation ; de près, où la taille suit, il n'en porte que le reste.
    /// Et l'éclat total d'une céphéide varie du facteur réel (~2), pas des
    /// ±20 % du seul carré du rayon.
    #[test]
    fn la_pulsation_se_voit_de_loin_comme_de_pres() {
        use super::{eclat_surfacique, lumiere_pulsee};
        let f = 1.1_f32;
        let l = lumiere_pulsee(f, 1.0);
        // De loin : rayon figé au plancher, l'éclat de surface fait tout.
        assert!((eclat_surfacique(l, 4.0, 4.0) - l).abs() < 1e-5, "la pulsation s'efface sous le plancher");
        // De près : le rayon dessiné suit, l'éclat de surface ne porte que le
        // reste — et le flux total (surface × éclat) reste l'éclat voulu.
        let e = eclat_surfacique(l, 50.0, 50.0 * f);
        assert!((e * f * f - l).abs() < 1e-4, "le flux total n'est pas conservé : {} au lieu de {l}", e * f * f);
        // L'amplitude : une céphéide (±10 %) varie d'un facteur réel, ~2.
        let rapport = lumiere_pulsee(1.1, 1.0) / lumiere_pulsee(0.9, 1.0);
        assert!(rapport > 1.9, "céphéide : rapport d'éclat {rapport}, trop sage pour se voir de loin");
        // Et sans pulsation ni ébullition : rien ne bouge.
        assert_eq!(lumiere_pulsee(1.0, 1.0), 1.0);
    }

    /// **Une étoile ordinaire ne fait pas battre ce qu'elle éclaire.** Ses
    /// flares (+60 %) et son ébullition faisaient pulser d'un coup toute la
    /// lumière qu'elle jette — mesuré sur un disque, −14 % d'une image à
    /// l'autre (2026-10-22). Seul un type à l'éclat visible la fait varier.
    #[test]
    fn une_etoile_ordinaire_ne_fait_pas_battre_ce_qu_elle_eclaire() {
        use super::variation_si_visible;
        for flare in [1.0_f32, 1.3, 1.6] {
            assert_eq!(variation_si_visible(false, flare), 1.0, "un flare éclaire le disque d'une étoile ordinaire");
            assert_eq!(variation_si_visible(true, flare), flare, "la céphéide ne fait plus varier sa lumière");
        }
    }

    /// **Seule la céphéide montre son éclat à l'image**, pour l'instant :
    /// branché sur toutes, il les faisait clignoter (flares, ébullition). Les
    /// autres types variables viendront un par un, nommément.
    #[test]
    fn seule_la_cepheide_montre_son_eclat_a_l_image() {
        let visibles: Vec<&str> = crate::etoile::types_du_tirage()
            .into_iter()
            .filter(|n| crate::etoile::eclat_visible(n))
            .collect();
        assert_eq!(visibles, ["Cepheide"], "types dont l'éclat atteint l'image : {visibles:?}");
    }

    /// **Le lot 10 : chaque type a sa surface, et les règles tiennent.**
    ///
    /// - chaque type du tirage qui ne bout pas a le **bord assombri** (§8 bis,
    ///   décision 3), le Soleil des presets (nom vide) compris ;
    /// - les **chaudes sont presque lisses** (pas de convection en surface) ;
    /// - une **naine M a le grain plus fin** que le Soleil (plus dense) ;
    /// - les surfaces **validées en ébullition** gardent le rendu d'avant.
    #[test]
    fn chaque_type_a_sa_surface_et_les_regles_tiennent() {
        use crate::etoile::{ebullition, surface};
        for nom in crate::etoile::types_du_tirage() {
            let (_, contraste, bord) = surface(nom);
            if ebullition(nom, false).0 == 0.0 && nom != "Trou noir" {
                assert!(bord > 0.0, "« {nom} » a encore le bord éclairci");
            }
            assert!((0.0..=1.0).contains(&contraste), "« {nom} » : contraste {contraste}");
        }
        assert!(surface("").2 > 0.0, "le Soleil des presets n'a pas le bord assombri");
        for chaude in [
            "O (bleue)",
            "B (bleu-blanc)",
            "A (blanche)",
            "Naine blanche",
            "Sous-naine (sdB)",
            "Supergeante bleue",
            "Wolf-Rayet",
        ] {
            assert!(surface(chaude).1 <= 0.3, "« {chaude} » : chaude mais granuleuse");
        }
        assert!(surface("M (naine rouge)").0 > surface("G (jaune)").0, "la naine M n'a pas le grain plus fin");
        for validee in ["Geante rouge", "Supergeante rouge", "Mira", "Cepheide", "Naine blanche pulsante (ZZ Ceti)"] {
            assert_eq!(surface(validee), (11.0, 1.0, 0.0), "« {validee} » : surface validée modifiée");
        }
    }

    /// **Les deux aspects existent, et le tirage atteint les deux.**
    ///
    /// Stable et instable se distinguent pour chaque type qui bouillonne (sans
    /// quoi la galerie montrerait deux cases identiques), Mira est toujours
    /// instable, et la part d'instables tirée de la graine est celle annoncée.
    #[test]
    fn les_geantes_rouges_ont_un_aspect_stable_et_un_instable() {
        use super::{est_instable, PART_INSTABLE};
        use crate::etoile::ebullition;
        for nom in ["Geante rouge", "Supergeante rouge"] {
            assert!(ebullition(nom, false).0 > 0.0, "{nom} stable ne bouillonne pas");
            assert_ne!(ebullition(nom, false), ebullition(nom, true), "{nom} : un seul aspect");
        }
        assert_eq!(ebullition("Mira", false), ebullition("Mira", true), "Mira doit rester instable");
        assert_eq!(ebullition("Mira", false), ebullition("Geante rouge", true));
        assert_eq!(ebullition("G (jaune)", true).0, 0.0, "une étoile ordinaire s'est mise à bouillir");
        // ZZ Ceti : un mélange (moitié granulation, moitié bouillonnement), à
        // plat, d'un seul aspect — et sa jumelle sans pulsation reste lisse.
        let zz = ebullition("Naine blanche pulsante (ZZ Ceti)", false);
        assert!(zz.0 > 0.0 && zz.0 * 2.0 < 1.0, "ZZ Ceti doit mêler les deux surfaces : {zz:?}");
        assert_eq!(zz.1, super::EBUL_PLAT, "ni relief ni contour pour ZZ Ceti");
        assert_eq!(zz, ebullition("Naine blanche pulsante (ZZ Ceti)", true));
        // La céphéide : exactement le même mélange (« le même shader »).
        assert_eq!(ebullition("Cepheide", false), zz, "la céphéide n'a pas le mélange de ZZ Ceti");
        assert_eq!(ebullition("Cepheide", true), zz);
        assert_eq!(ebullition("Naine blanche", false).0, 0.0, "la naine blanche ordinaire s'est mise à bouillir");

        let n = 5000;
        let instables = (0..n).filter(|k| est_instable(*k as f32 * 0.2)).count() as f32 / n as f32;
        assert!((instables - PART_INSTABLE).abs() < 0.05, "{instables} instables pour {PART_INSTABLE} visés");
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
    ///
    /// ⚠️ **Elle ne couvre pas la pulsation** (lot 7), et la conception l'avait
    /// annoncé : elle porte sur la **constante** `R_SOLEIL`, donc un facteur
    /// appliqué au moment du dessin passe devant elle une seconde fois. Le
    /// Soleil ne pulse pas — `etoile::pulsation("")` est nulle — mais ce n'est
    /// pas cette garde qui l'assure. Celle qui couvre les types variables est
    /// `genese::tests_orbite_interne::une_etoile_qui_pulse_n_avale_pas_sa_planete_interne`.
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
    /// ⚠️ Balaie **tout le catalogue** : les bornes sont portées par le
    /// preset depuis qu'un trou noir vorace a un disque plus large qu'un
    /// endormi, donc la règle doit valoir pour chacun — y compris les
    /// presets — les sept y sont, depuis la décision du 2026-09-30.
    #[test]
    fn le_disque_d_accretion_degage_l_horizon() {
        for a in Accretion::catalogue() {
            let nom = a.nom;
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
            // ⚠️ La ceinture **empiète** sur le disque externe et le
            // dépasse. Cette garde disait l'inverse — « au-delà du disque,
            // deux objets distincts » — et c'était une hypothèse de ma part,
            // démentie à l'écran : sans recouvrement on voit deux anneaux
            // concentriques sans rapport, au lieu de cailloux qui
            // descendent et alimentent le disque.
            if let Some((ci, ce)) = a.ceinture {
                assert!(ci < ext, "{nom} : la ceinture ne mord pas sur le disque");
                // Mais pas jusqu'au cœur : le disque interne vaporise tout
                // solide, des cailloux n'y survivraient pas.
                assert!(ci > ext * 0.5, "{nom} : des cailloux jusque dans le disque brûlant");
                assert!(ce > ext, "{nom} : la ceinture est noyée dans le disque, on ne la verra pas");
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
