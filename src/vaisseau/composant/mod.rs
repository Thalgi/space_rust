//! Composant concret : **enum fermé** qui sait exposer ses ports et se dessiner
//! (voir `docs/conception/stations.md`, Partie C §3 et §4 — sous-étape 2a).
//!
//! Choix acté : dispatch par `match` sur un enum, **pas** de trait objet — KISS,
//! zéro allocation, monomorphisé. Une seule fonction
//! par capacité (`ports`, `dessiner`, `cout`, `rayon_local`), qui `match` sur la
//! variante. Les styles/palettes viendront à l'Étape 5. Composants existants :
//! `ModuleAxial` (cylindre) et `Noeud` (hub sphérique 4 ou 6 sorties).

use super::{Enveloppe, GenrePort, Piece, Port, Profil};
use super::chantier::Chantier;
use macroquad::prelude::*;
use super::peintre::Peintre;

mod commun;
mod equipage;
mod habitat;
mod antimatiere;
mod adaptateur;
pub use adaptateur::VarianteCoiffe;
mod antenne;
mod module_axial;
pub use module_axial::VarianteModule;
mod noeud;
mod propulsion;
pub use propulsion::{FamillePropulsion, VariantePropulseur};
pub use noeud::Sorties;
mod panneau_solaire;
mod radiateur;
pub use radiateur::VarianteRadiateur;
pub use panneau_solaire::VariantePanneau;
pub use antenne::VarianteAntenne;
mod caisson;
pub use caisson::{VarianteCaisson, VarianteCharge};
mod bouclier;
mod panache;
mod panneau_mega;
/// Rayon du jet à la fraction `t` de sa longueur, exposé pour que l'assemblage
/// puisse vérifier que le panache **contourne** la charge utile remorquée.
#[allow(unused_imports)]
pub(crate) use panache::{rayon as rayon_panache, teinte as teinte_panache};
mod thermique;
mod tore;
/// Section du bardage thermique, exposée pour que l'assemblage puisse vérifier
/// qu'il **épouse** l'épine. C'est la surface qui se plaque sur la poutre — la
/// mesurer sur les sommets cuits attraperait les lèvres, relevées par
/// construction, et ferait croire à un bardage qui flotte.
#[allow(unused_imports)]
pub(crate) use thermique::section as section_bardage;
pub use bouclier::ELANCEMENT as BOUCLIER_ELANCEMENT;
mod cargo;
mod reservoir;
mod treillis;
pub use panneau_mega::VariantePanneauMega;
pub use treillis::{hexagone_ceinture, PiedHexa, StyleTreillis};
use commun::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::rc::Rc;

/// Une brique concrète, dessinable et dotée de ports. Enum fermé : on ajoute une
/// variante ici et on complète les cinq `match` (`ports`, `dessiner`, `cout`,
/// `rayon_local`, `englobant_local`).
///
/// **Pas `Copy`** depuis l'ajout de [`Composant::SousEnsemble`] (Partie E.3) :
/// son champ `donnees: Rc<..>` ne l'est pas. Les 19 autres variantes restent
/// aussi bon marché à cloner qu'à copier (champs `f32`/enums `Copy`) ; seul
/// `SousEnsemble` fait un clone réel — un `Rc::clone`, donc un compteur de
/// référence, pas une copie du sous-arbre.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Composant {
    /// Module pressurisé cylindrique, aligné sur Z, avec une **écoutille axiale
    /// à chaque bout** (avant sortant : +Z et −Z). `profil` fixe le rayon.
    ModuleAxial { profil: Profil, variante: VarianteModule, longueur: f32 },
    /// Nœud : hub sphérique multi-ports, toutes les écoutilles du même `profil`.
    /// `sorties` fixe le nombre/disposition (croix plane ou 3D).
    Noeud { profil: Profil, sorties: Sorties },
    /// Panneau solaire : mât + pale, monté par un unique port `Surface`.
    /// `variante` fixe le style (couleur, proportions, tuiles). Se déploie le long
    /// de +Z ; se pose en **paire miroir** via la symétrie.
    PanneauSolaire { profil: Profil, variante: VariantePanneau, longueur: f32, largeur: f32 },
    /// Treillis / poutre : ossature alignée sur Z. Bouts axiaux (`ModuleAxial`,
    /// chaînables avec modules/nœuds) + **ports hôtes `Surface`** par paires ±X
    /// répartis sur la longueur — accueillent n'importe quel appendice (panneau,
    /// radiateur, antenne). `style` = section ; `profil` fixe le gabarit.
    Treillis { profil: Profil, longueur: f32, style: StyleTreillis },
    /// Radiateur thermique : monté par un port `Surface`, déployé le long de +Z.
    /// `variante` fixe la technologie/allure.
    Radiateur { profil: Profil, variante: VarianteRadiateur, longueur: f32, largeur: f32 },
    /// Antenne / parabole : montée par un port `Surface`, pointe vers +Z.
    /// `variante` fixe le type (parabole, cornets, fouets, réseau, hélice).
    Antenne { profil: Profil, variante: VarianteAntenne, taille: f32 },
    /// Adaptateur tronconique : relie deux **profils** (ou sert de nez de docking
    /// PMA/IDA). Écoutilles axiales `grand` (−Z) et `petit` (+Z).
    Adaptateur { grand: Profil, petit: Profil, longueur: f32 },
    /// Caisson technique **non pressurisé** : boîte à ossature, palette
    /// d'expériences, berceau de réservoirs, instrument pointé. Se monte comme
    /// un appendice (port `Surface`) — donc partout où va un radiateur : sur une
    /// poutre ou sur le flanc d'un module.
    Caisson { profil: Profil, variante: VarianteCaisson, longueur: f32, largeur: f32 },
    /// Charge utile posée **sur** une structure ou sur un caisson technique :
    /// plateforme d'expériences, réservoirs, instrument. Un seul port de
    /// montage, comme tout appendice.
    ChargeUtile { profil: Profil, variante: VarianteCharge, longueur: f32, largeur: f32 },
    /// Propulseur. Selon sa variante il se monte **en bout de corps** (écoutille
    /// axiale, moteur principal) ou **sur un flanc** (port `Surface`, grappe de
    /// contrôle d'attitude) — cf. [`VariantePropulseur::axial`].
    Propulseur { profil: Profil, variante: VariantePropulseur, taille: f32 },
    /// Charpente en treillis **à section variable et courbe** : évasée du profil
    /// `grand` (base) au profil `petit` (bout), l'affinement suivant `courbure`
    /// (1 = cône droit, >1 = base évasée puis longue flèche, type ISV).
    /// Écoutilles axiales aux deux bouts, à leurs profils respectifs.
    Charpente { grand: Profil, petit: Profil, longueur: f32, courbure: f32, aiguille: bool },
    /// **Charpente à section hexagonale** : même loi de taille et mêmes ports que
    /// [`Composant::Charpente`], mais six longerons au lieu de quatre.
    ///
    /// Variante **candidate** pour l'épine de l'ISV, pas encore assemblée. Deux
    /// raisons : sa largeur apparente ne varie que de 1,15 selon l'angle (contre
    /// 1,41 pour un carré), ce qui la rend lisible sous le filtre pixel d'où
    /// qu'on la regarde ; et son pied se raccorde au cadre hexagonal de la
    /// propulsion **six sommets pour six**, là où le carré faisait converger
    /// quatre coins sur deux sommets.
    CharpenteHexa { grand: Profil, petit: Profil, longueur: f32, courbure: f32, pied: PiedHexa },
    /// **Radiateur de mégastructure** : aile en arête de poisson (boom central +
    /// `ailettes` panneaux plats de chaque côté), à l'échelle du vaisseau/km, pas
    /// des petits radiateurs de station. Se monte par un port `Surface`.
    /// Première brique de la famille « méga » (ISV, puis O'Neill, Elysium).
    /// `chaleur` va de 0 (froid, gris) à 1 (pleine chauffe, orange). Seules les
    /// parties **grises** — panneau, tubes calorifiques, rails de bord — la
    /// suivent ; la colonne vertébrale et le réservoir restent noirs. Et elle
    /// **décroît vers la pointe** : un radiateur se refroidit sur sa longueur,
    /// c'est même toute sa fonction.
    RadiateurMega { profil: Profil, longueur: f32, largeur: f32, ailettes: usize, chaleur: f32 },
    /// **Nacelle moteur de l'ISV** : nœud d'attache + hub + 4 sphères d'hydrogène
    /// + **deux moteurs jumeaux inclinés de ±1,5°** (corps magnétique, anneaux de
    /// confinement, tuyère évasée, boucliers anti-radiation, radiateurs).
    /// `echelle` met le tout à l'échelle. Écoutille axiale au nœud d'attache.
    Motrice { profil: Profil, echelle: f32 },
    /// **Brique de base du bloc propulsion** (repartie de zéro) : un
    /// parallélépipède (longueur Z = 2·`largeur`, hauteur Y = `largeur`) branché
    /// par un col fin à un **gros cylindre** aligné sur +Z, qui **ne recoupe pas**
    /// la caisse (col + petit intervalle). Écoutilles axiales aux deux bouts.
    BlocMoteur { profil: Profil, largeur: f32 },
    /// **Prototype réservoir de carburant** : cuve **sphérique** à tuiles,
    /// optionnellement cerclée d'une **cage tétraédrique** de 4 barres métal
    /// (`cage`). Écoutilles axiales aux deux bouts.
    Reservoir { profil: Profil, longueur: f32, cage: bool },
    /// **Moteur à antimatière (partie principale)** : corps de confinement
    /// magnétique bagué d'anneaux de bobines (silhouette VASIMR), cœur
    /// d'annihilation émissif, tuyère magnétique évasée et long jet de plasma.
    /// Poussée le long de **−Z** ; unique écoutille axiale de montage (+Z).
    MoteurAntimatiere { profil: Profil, taille: f32 },
    /// **Coiffe de module** : capuchon de nez posé sur l'écoutille axiale d'un
    /// module. `profil` fixe le rayon de base (à accorder au module coiffé) ;
    /// `variante` en fixe la forme. Base en Z=0, nez déployé vers **+Z** ;
    /// unique écoutille axiale de montage (avant −Z, vers le module).
    Coiffe { profil: Profil, variante: VarianteCoiffe },
    /// **Réacteur d'antimatière** : bloc **d'injection/confinement** qui se
    /// branche en amont de la tuyère ([`Composant::MoteurAntimatiere`]). Cuve
    /// sombre ceinturée de **bobines électromagnétiques** (cryostat) et de
    /// **tuyauterie**, coiffée d'un injecteur et de pièges à antiprotons. Base
    /// en Z=0 (écoutille de montage vers −Z, côté tuyère), corps vers **+Z**,
    /// second port axial en tête pour chaîner l'alimentation.
    ReacteurAntimatiere { profil: Profil, taille: f32 },
    /// **Anneau hexagonal en treillis** autonome — le même que celui du pied de
    /// [`Composant::Charpente`] (`aiguille`), mais posé seul. `profil` fixe le
    /// gabarit (côté = `profil.rayon()`). `liaison` > 0 tire **6 montants** de
    /// chaque sommet le long de **+Z local** (→ vers un hexagone identique situé
    /// `liaison` plus loin) : les deux hexagones deviennent un **prisme** relié,
    /// sans écart.
    TreillisHexagone { profil: Profil, liaison: f32 },
    /// **Nacelle de fret** d'échelle vaisseau (ISV) : long conteneur à section
    /// **onigiri** (triangle arrondi), déployé le long de +Z depuis son port de
    /// montage. `profil` fixe le rayon de section, `longueur` l'élancement (les
    /// nacelles de l'ISV sont très longues devant leur section). `spin` tourne
    /// la section autour de son axe, pour présenter un lobe ou un côté plat au
    /// voisin. Se monte comme un appendice (port `Surface`).
    ///
    /// **Pourquoi une brique neuve** : `Caisson`/`ChargeUtile` sont du
    /// vocabulaire ISS (porteurs d'ORU, berceaux FRAM, poignées EVA) — mauvaise
    /// échelle et mauvaise silhouette pour du fret interstellaire.
    NacelleCargo { profil: Profil, longueur: f32, spin: f32 },
    /// **Râtelier de fret** : une « rangée » de la section charge utile de
    /// l'ISV. Une couronne de `nacelles` [`Composant::NacelleCargo`] serrées
    /// autour de l'axe +Z, à la distance `rayon`, tenues par une cage ouverte
    /// (anneaux aux deux bouts et à mi-longueur + rayons vers l'axe). Écoutilles
    /// axiales aux deux bouts : les râteliers se **chaînent** le long de l'épine.
    ///
    /// Les nacelles sont dessinées **par le râtelier** (comme les ailettes d'un
    /// [`Composant::RadiateurMega`]) : elles sont identiques et nombreuses, une
    /// pièce par nacelle ferait exploser le compte pour rien.
    ///
    /// `nacelle` est le rayon hors-tout d'un conteneur. **0 = empilement
    /// serré** : le rayon est alors déduit de `rayon` pour que les conteneurs
    /// se touchent. Le fixer permet d'**ouvrir la couronne sans grossir le
    /// fret** — c'est ce qu'il faut quand l'épine qui passe au milieu change de
    /// gabarit alors que la charge utile, elle, ne doit pas changer de taille.
    RatelierCargo { profil: Profil, longueur: f32, rayon: f32, nacelles: usize, nacelle: f32 },
    /// **Module d'habitat principal** (ISV) — l'habitat **fixe**, solidaire de
    /// l'épine ; à ne pas confondre avec les modules d'équipage **rotatifs**
    /// (gravité artificielle), qui sont une autre brique, encore à faire.
    ///
    /// Même **section onigiri** que les
    /// nacelles de fret, en plus gros — la famille visuelle du vaisseau tient à
    /// ça. Coque **composite** nue (teinte os, non métallique : l'habitat du
    /// vrai vaisseau évite le métal, qui transformerait les rayons cosmiques en
    /// rayonnement secondaire dans les couchettes), **sans** les collerettes
    /// sombres ni les rails d'arête de la nacelle.
    ///
    /// Habillage : trois **armatures triangulaires** ceinturant le fût aux quarts
    /// (¼, ½, ¾), et — sur **un seul** côté plat, celui que désigne `spin` — une
    /// **ferrure d'attache** (longeron + jambes) par laquelle le module se
    /// solidarise de l'épine de l'ISV. `attache` est la portée de cette ferrure
    /// (0 = aucune, pour un module présenté seul).
    ///
    /// Écoutilles axiales aux deux bouts. Brique neuve plutôt qu'une 11ᵉ
    /// variante de `ModuleAxial` : ce dernier porte tout le vocabulaire ISS
    /// (collerettes d'accostage, embouts, mains courantes EVA) qui n'a ni le
    /// gabarit ni le sens ici.
    ModuleHabitat { profil: Profil, longueur: f32, spin: f32, attache: f32 },
    /// **Module d'équipage rotatif** (ISV) : nacelle habitée au bout d'une
    /// traverse, mise en rotation pour fabriquer de la gravité. Fût
    /// **cylindrique** — un plancher courbe est ce qu'impose la rotation, là où
    /// une section onigiri ferait varier l'inclinaison du « bas » le long de la
    /// paroi. Axe +Z **vers l'extérieur** : ce bout est le plancher, les
    /// `hublots` le ceinturent, et le port de montage est à l'autre bout.
    ///
    /// À distinguer de [`Composant::ModuleHabitat`], qui est l'habitat **fixe**.
    ModuleEquipage { profil: Profil, longueur: f32, hublots: usize },
    /// **Petit bouclier de tête** (ISV) : plaque hexagonale **régulière**,
    /// première de la pile de quatre qui pare la poussière interstellaire à
    /// 0,7 c. Deux faces distinctes — l'avant (+Z, côté poussière) est strié,
    /// l'arrière porte les nervures. Moyeu **traversant** : les quatre plaques
    /// sont enfilées sur un mât commun, d'où deux ports axiaux.
    ///
    /// À distinguer du **bouclier thermique** de l'épine, qui pare l'échappement
    /// des moteurs et n'est qu'un détail de surface.
    BouclierPetit { profil: Profil, rayon: f32 },
    /// **Grand bouclier de tête** (ISV) : le même hexagone **étiré** selon Y
    /// (`elancement`), donc à deux longs bords parallèles et une pointe en haut
    /// et en bas. Miroir bleuté sur ses **deux** faces, ossature centrée sur le
    /// plan de la plaque. La tête en porte trois identiques, derrière le
    /// [`Composant::BouclierPetit`].
    BouclierGrand { profil: Profil, rayon: f32, elancement: f32 },
    /// **Bouclier thermique d'épine** (ISV) : bardage d'**écailles imbriquées**
    /// qui protège la poutre du rayonnement des tuyères. Section hexagonale
    /// comme l'épine qu'il habille, axe +Z, base (côté moteurs) à l'origine.
    ///
    /// Chaque écaille recouvre la **suivante vers +Z**, donc dans le sens où
    /// s'éloigne la chaleur : le flux glisse d'une plaque à l'autre sans
    /// rencontrer de tranche. Monté à l'envers, chaque joint offrirait une arête
    /// au rayonnement.
    ///
    /// À distinguer de [`Composant::BouclierPetit`] / [`Composant::
    /// BouclierGrand`], qui parent la poussière interstellaire à l'autre bout du
    /// vaisseau. Pièce de surface, posée à la main : pas de port.
    /// `rayon_pied` est le **gros** bout (côté moteurs, à l'origine) et
    /// `rayon_bout` le petit : le bardage se monte au droit des tuyères, là où
    /// l'épine s'ouvre vers son pied, et il doit l'épouser. `courbure` est la
    /// même que celle du treillis qu'il habille.
    BouclierThermique {
        rayon_pied: f32,
        rayon_bout: f32,
        courbure: f32,
        longueur: f32,
        rangs: usize,
    },
    /// **Panache d'antimatière** : le jet qui sort d'une tuyère, le long de +Z
    /// depuis l'origine. `intensite` va de 0 (moteur coupé, rien n'est dessiné)
    /// à 1 (pleine poussée).
    ///
    /// C'est un **effet**, pas une pièce : coût nul, encombrement nul, aucun
    /// port. Le compter dans l'englobant ferait reculer la caméra de deux
    /// longueurs de vaisseau à l'allumage.
    Panache { longueur: f32, rayon_col: f32, rayon_bout: f32, intensite: f32 },
    /// **Tore** d'habitat : une coque de révolution, pas un assemblage de
    /// briques. Première primitive **paramétrique** du parc (voir `tore.rs`
    /// pour le pourquoi, et `stations.md` §6 qui l'avait prévue).
    ///
    /// Tracé dans le plan **X‑Z**, axe **Y** — le même que le moyeu hexagonal.
    /// `segments` facette le grand cercle, `anneaux` la section. Aucun port :
    /// il se pose à la main, comme `TreillisHexagone`.
    /// `jonctions` / `phase` disent **où des bras arrivent** sur l'anneau : le
    /// vitrage y est interrompu au profit d'une coque pleine nervurée. Le tore
    /// doit le savoir lui-même — c'est sa propre géométrie qui change, pas
    /// quelque chose qu'on poserait par-dessus après coup.
    /// **Panneau solaire d'échelle mégastructure**, avec suivi solaire à deux
    /// axes (`azimut` autour du mât, `inclinaison` autour de l'axe de l'aile).
    /// Voir `panneau_mega.rs` : ce n'est pas un `PanneauSolaire` agrandi mais
    /// une autre famille de structures.
    PanneauMega {
        profil: Profil,
        variante: VariantePanneauMega,
        longueur: f32,
        largeur: f32,
        azimut: f32,
        inclinaison: f32,
    },
    Tore {
        rayon_majeur: f32,
        rayon_mineur: f32,
        segments: usize,
        anneaux: usize,
        jonctions: usize,
        phase: f32,
    },
    /// **Collier de rotation** de la section d'équipage : tambour **creux** qui
    /// ceinture l'épine et porte les bras. `alesage` est le rayon intérieur
    /// libre — il doit dégager l'épine, sinon la section ne pourrait pas
    /// tourner. Écoutilles axiales aux deux bouts.
    /// `rayon` est la jaquette extérieure, **découplée de `profil`** comme le
    /// `rayon` de [`Composant::RatelierCargo`] : cette cote se règle contre la
    /// structure traversante, et les crans de `Profil` (0,5 / 1 / 2 / 3) sont
    /// trop grossiers pour ça — il n'y a rien entre « trop maigre pour couvrir
    /// l'épine » et « deux fois trop gros ». `profil` ne sert plus qu'à déclarer
    /// les ports.
    CollierRotatif { profil: Profil, rayon: f32, alesage: f32, longueur: f32 },
    /// **Charnière de repli** d'un bras d'équipage : chape à deux joues, axe
    /// traversant et **vérin** télescopique en biais. `repli` va de 0 (bras
    /// déployé, radial) à 1 (bras rabattu le long de la coque), et le vérin
    /// s'allonge réellement avec lui. Pièce d'articulation : pas de port, elle
    /// se pose à la main entre le collier et le bras.
    Charniere { taille: f32, repli: f32 },
    /// **Sous-ensemble figé** : un groupe de pièces déjà assemblées (par
    /// [`crate::vaisseau::Chantier::figer`]), traité comme **une seule
    /// brique** réutilisable (pattern Composite — voir `docs/conception/
    /// stations.md` Partie E.3). `profil` est le profil du port de montage
    /// présenté au parent. `dessiner` délègue à chaque pièce du sous-arbre ;
    /// `cout`/`rayon_local`/`englobant_local` lisent des valeurs précalculées
    /// à la congélation (pas de parcours à chaque appel).
    SousEnsemble { profil: Profil, donnees: Rc<DonneesSousEnsemble> },
}

/// Données figées d'un [`Composant::SousEnsemble`] : le sous-arbre de pièces
/// (repère **local** au sous-ensemble, comme dans une `Station`), les ports
/// hôtes restés libres à la congélation (aussi en repère local — le port de
/// montage exposé au parent n'en fait pas partie, il est encodé à part par
/// `Composant::SousEnsemble::profil`), et le coût/rayon précalculés (sommés
/// une fois, à la construction, plutôt que reparcourus à chaque appel de
/// `Chantier::poser`).
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct DonneesSousEnsemble {
    pub pieces: Vec<Piece>,
    pub ports_exposes: Vec<Port>,
    pub cout: f32,
    pub rayon: f32,
}







/// Disposition des nacelles d'un [`Composant::RatelierCargo`] : renvoie le rayon
/// hors-tout d'une nacelle et, pour chacune, `(position de station, spin)`.
/// **Partagée par le dessin et par le calcul d'encombrement** — les deux ne
/// peuvent donc pas diverger.
///
/// Deux régimes, parce que trois nacelles ne se rangent pas comme six :
/// - **3 → triforce** : trois conteneurs de **même orientation** posés aux
///   sommets d'un triangle, pointe contre pointe, laissant un creux
///   triangulaire au milieu ;
/// - **≥ 4 → couronne** : réparties en angle, **coin vers l'axe**, les côtés
///   plats se faisant face. Le rayon vient du demi-pas angulaire, donc elles ne
///   se croisent jamais quel que soit leur nombre.
///
/// **Écartement de la triforce** (l'erreur à ne pas refaire) : des triangles à
/// coins **vifs** se touchent pointe contre pointe quand la distance au centre
/// vaut leur circonrayon. Nos sections ont des coins **congés** — ce sont donc
/// des triangles nus *gonflés* du rayon de congé ρ, et à cette distance elles
/// s'interpénètrent. Il faut écarter les triangles nus de 2ρ. Leurs pointes
/// s'écartent de `√3·(D − r_nu)` quand on écarte les centres de `D`, d'où
/// `D = r_nu + 2ρ/√3`, soit `D = r·(1 − f + 2f/√3)` avec `f` la fraction de
/// congé. Le facteur `JEU` ajoute par-dessus un vrai jour visible.
fn grappe_cargo(rayon: f32, nacelles: usize, nacelle: f32) -> (f32, Vec<(Vec3, f32)>) {
    let n = nacelles.max(3);
    let dir = |a: f32| vec3(a.cos(), a.sin(), 0.0);
    // Rayon imposé (`nacelle > 0`) : la couronne peut alors s'ouvrir sans que le
    // fret grossisse. Sinon, on le déduit pour un empilement serré.
    let impose = nacelle > 1e-4;
    if n == 3 {
        const JEU: f32 = 1.05;
        let f = super::pieces::ONIGIRI_FILET;
        let ecart = (1.0 - f + 2.0 * f / 3.0_f32.sqrt()) * JEU;
        let places = (0..3)
            .map(|k| {
                let a = FRAC_PI_2 + TAU * k as f32 / 3.0;
                (dir(a) * rayon, FRAC_PI_2)
            })
            .collect();
        (if impose { nacelle } else { rayon / ecart }, places)
    } else {
        let rnac = rayon * (PI / n as f32).sin() * 0.9;
        let places = (0..n)
            .map(|k| {
                let a = TAU * k as f32 / n as f32;
                (dir(a) * rayon, a + PI)
            })
            .collect();
        (if impose { nacelle } else { rnac }, places)
    }
}



















/// Enveloppe d'un **appendice en aile** : long et mince, déployé vers `+Z`
/// depuis son montage à l'origine.
///
/// `portee` est ce que la famille déclare comme extension (son `rayon_local`),
/// `demi_epaisseur` sa demi-largeur en travers. L'axe part du montage et va
/// jusqu'au bout : c'est la seule façon de couvrir le coin extérieur de l'aile
/// sans gonfler le rayon (cf. [`Enveloppe::axe`]).
fn enveloppe_aile(portee: f32, demi_epaisseur: f32) -> Enveloppe {
    Enveloppe::capsule(Vec3::ZERO, Vec3::Z * portee, demi_epaisseur)
}

impl Composant {
    /// Ports dans le repère **local** du composant (montage + hôtes libres,
    /// indistincts : on marque l'occupé à l'assemblage). Convention `Repere` :
    /// `avant = rot*Z` sortant, `haut = rot*Y`.
    pub fn ports(&self) -> Vec<Port> {
        match self {
            Composant::ModuleAxial { profil, longueur, .. } => module_axial::ports(*profil, *longueur),
            Composant::Noeud { profil, sorties } => noeud::ports(*profil, *sorties),
            Composant::PanneauSolaire { profil, .. } => panneau_solaire::ports(*profil),
            Composant::Treillis { profil, longueur, .. } => treillis::ports(*profil, *longueur),
            Composant::Radiateur { profil, .. } => radiateur::ports(*profil),
            Composant::Caisson { profil, longueur, largeur, .. } => caisson::ports(*profil, *longueur, *largeur),
            Composant::ChargeUtile { profil, .. } => caisson::charge_ports(*profil),
            Composant::Charpente { grand, petit, longueur, .. } => treillis::charpente_ports(*grand, *petit, *longueur),
            Composant::CharpenteHexa { grand, petit, longueur, .. } => treillis::charpente_hexa_ports(*grand, *petit, *longueur),
            Composant::RadiateurMega { profil, .. } => radiateur::mega_ports(*profil),
            Composant::Motrice { profil, echelle } => propulsion::motrice_ports(*profil, *echelle),
            Composant::BlocMoteur { profil, largeur } => propulsion::bloc_ports(*profil, *largeur),
            Composant::Reservoir { profil, longueur, .. } => reservoir::ports(*profil, *longueur),
            Composant::MoteurAntimatiere { profil, .. } => antimatiere::moteur_ports(*profil),
            Composant::Coiffe { profil, .. } => adaptateur::coiffe_ports(*profil),
            Composant::ReacteurAntimatiere { profil, taille } => antimatiere::reacteur_ports(*profil, *taille),
            // Anneau décoratif posé à la main (via un `Repere` cuit) : pas de port.
            Composant::TreillisHexagone { .. } => vec![],
            // Nacelle de fret : un appendice, monté par sa base (avant −Z vers
            // l'hôte), le conteneur se déployant vers +Z.
            Composant::NacelleCargo { profil, .. } => cargo::nacelle_ports(*profil),
            // Module d'habitat : deux écoutilles axiales, comme tout fût pressurisé.
            Composant::ModuleHabitat { profil, longueur, .. } => habitat::ports(*profil, *longueur),
            Composant::ModuleEquipage { profil, .. } => equipage::ports(*profil),
            // Boucliers de tête : enfilés sur un mât, donc un port à chaque bout
            // du moyeu — c'est la même logique que le collier rotatif.
            Composant::BouclierPetit { profil, rayon } => bouclier::ports(*profil, *rayon),
            Composant::BouclierGrand { profil, rayon, .. } => bouclier::ports(*profil, *rayon),
            // Effet posé à la main sur la tuyère : pas de port.
            Composant::Panache { .. } => vec![],
            Composant::PanneauMega { profil, .. } => panneau_mega::ports(*profil),
            Composant::Tore { .. } => vec![],
            // Bardage de surface enfilé sur l'épine, posé à la main : pas de port.
            Composant::BouclierThermique { .. } => vec![],
            Composant::CollierRotatif { profil, longueur, .. } => equipage::collier_ports(*profil, *longueur),
            // Articulation posée à la main : aucun port.
            Composant::Charniere { .. } => vec![],
            // Râtelier : deux écoutilles axiales, comme une poutre — les rangées
            // se chaînent bout à bout le long de l'épine.
            Composant::RatelierCargo { profil, longueur, .. } => cargo::ratelier_ports(*profil, *longueur),
            // Ports hôtes restés libres à la congélation, déjà en repère local.
            Composant::SousEnsemble { donnees, .. } => donnees.ports_exposes.clone(),
            Composant::Propulseur { profil, variante, taille } => propulsion::ports(*profil, *variante, *taille),
            Composant::Antenne { profil, .. } => antenne::ports(*profil),
            Composant::Adaptateur { grand, petit, longueur } => adaptateur::ports(*grand, *petit, *longueur),
        }
    }

    /// Dessine dans le repère **local** (la transformée monde est déjà poussée
    /// par l'appelant via `push_model_matrix`).
    pub fn dessiner<P: Peintre>(&self, p: &mut P) {
        match self {
            Composant::ModuleAxial { profil, longueur, variante } => module_axial::dessiner(p, *profil, *variante, *longueur),
            Composant::Noeud { profil, sorties } => noeud::dessiner(p, *profil, *sorties),
            Composant::PanneauSolaire { variante, longueur, largeur, .. } => panneau_solaire::dessiner(p, *variante, *longueur, *largeur),
            Composant::Treillis { profil, longueur, style } => treillis::dessiner(p, *profil, *longueur, *style),
            Composant::Radiateur { variante, longueur, largeur, .. } => radiateur::dessiner(p, *variante, *longueur, *largeur),
            Composant::Caisson { variante, longueur, largeur, .. } => caisson::dessiner(p, *variante, *longueur, *largeur),
            Composant::ChargeUtile { variante, longueur, largeur, .. } => caisson::charge_dessiner(p, *variante, *longueur, *largeur),
            Composant::Propulseur { variante, taille, .. } => propulsion::dessiner(p, *variante, *taille),
            Composant::Charpente { grand, petit, longueur, courbure, aiguille } => treillis::charpente_dessiner(p, *grand, *petit, *longueur, *courbure, *aiguille),
            Composant::CharpenteHexa { grand, petit, longueur, courbure, pied } => treillis::charpente_hexa_dessiner(p, *grand, *petit, *longueur, *courbure, *pied),
            Composant::RadiateurMega { longueur, largeur, ailettes, chaleur, .. } => radiateur::mega_dessiner(p, *longueur, *largeur, *ailettes, *chaleur),
            Composant::Motrice { echelle, .. } => propulsion::motrice_dessiner(p, *echelle),
            Composant::BlocMoteur { largeur, .. } => propulsion::bloc_dessiner(p, *largeur),
            Composant::Reservoir { longueur, cage, .. } => reservoir::dessiner(p, *longueur, *cage),
            Composant::MoteurAntimatiere { taille, .. } => antimatiere::moteur_dessiner(p, *taille),
            Composant::Coiffe { profil, variante } => adaptateur::coiffe_dessiner(p, *profil, *variante),
            Composant::ReacteurAntimatiere { taille, .. } => antimatiere::reacteur_dessiner(p, *taille),
            Composant::TreillisHexagone { profil, liaison } => treillis::hexagone_dessiner(p, *profil, *liaison),
            Composant::NacelleCargo { profil, longueur, spin } => cargo::nacelle_dessiner(p, *profil, *longueur, *spin),
            Composant::ModuleHabitat { profil, longueur, spin, attache } => habitat::dessiner(p, *profil, *longueur, *spin, *attache),
            Composant::ModuleEquipage { profil, longueur, hublots } => equipage::dessiner(p, *profil, *longueur, *hublots),
            Composant::BouclierPetit { rayon, .. } => bouclier::petit_dessiner(p, *rayon),
            Composant::BouclierGrand { rayon, elancement, .. } => bouclier::grand_dessiner(p, *rayon, *elancement),
            // Rendu à part, en additif : voir `panache::dessiner`.
            Composant::Panache { .. } => panache::dessiner(p),
            Composant::PanneauMega { variante, longueur, largeur, azimut, inclinaison, .. } => {
                panneau_mega::dessiner(p, *variante, *longueur, *largeur, *azimut, *inclinaison)
            }
            Composant::Tore { rayon_majeur, rayon_mineur, segments, anneaux, jonctions, phase } => {
                tore::dessiner(p, *rayon_majeur, *rayon_mineur, *segments, *anneaux, *jonctions, *phase)
            }
            Composant::BouclierThermique { rayon_pied, rayon_bout, courbure, longueur, rangs } => {
                thermique::dessiner(p, *rayon_pied, *rayon_bout, *courbure, *longueur, *rangs)
            }
            Composant::CollierRotatif { rayon, alesage, longueur, .. } => equipage::collier_dessiner(p, *rayon, *alesage, *longueur),
            Composant::Charniere { taille, repli } => equipage::charniere_dessiner(p, *taille, *repli),
            Composant::RatelierCargo { longueur, rayon, nacelles, nacelle, .. } => cargo::ratelier_dessiner(p, *longueur, *rayon, *nacelles, *nacelle),
            // Composite : empile la transformée LOCALE de chaque enfant
            // (composée par-dessus celle déjà active, cf. `Peintre::
            // empiler_transforme`) et lui délègue son propre dessin — même
            // mécanique que `Station::dessiner`, à un niveau d'indirection.
            Composant::SousEnsemble { donnees, .. } => {
                for piece in &donnees.pieces {
                    p.empiler_transforme(piece.transforme);
                    piece.composant.dessiner(p);
                    p.depiler_transforme();
                }
            }
            Composant::Antenne { variante, taille, .. } => antenne::dessiner(p, *variante, *taille),
            Composant::Adaptateur { grand, petit, longueur } => adaptateur::dessiner(p, *grand, *petit, *longueur),
        }
    }

    /// Coût de rendu ≈ nombre de primitives dessinées (pondère le `Budget`,
    /// fondations §3.1).
    pub fn cout(&self) -> f32 {
        match self {
            // corps + 2 embouts + 2 collerettes de docking = 5.
            Composant::ModuleAxial { .. } => module_axial::cout(),
            // sphère + (bras + collerette) par sortie.
            Composant::Noeud { sorties, .. } => noeud::cout(*sorties),
            // mât + pale nervurée : poids représentatif (une aile ≫ un tube nu).
            Composant::PanneauSolaire { .. } => panneau_solaire::cout(),
            // treillis ajouré : coût qui croît avec la longueur (baies de plus).
            Composant::Treillis { longueur, .. } => treillis::cout(*longueur),
            // radiateur : coût selon la technologie (accordéon/LDR plus lourds).
            Composant::Radiateur { variante, .. } => variante.cout(),
            // caisson : boîte + ossature, coût selon le type.
            Composant::Caisson { variante, .. } => variante.cout(),
            // charge utile : selon le type.
            Composant::ChargeUtile { variante, .. } => variante.cout(),
            // propulseur : selon la technologie.
            Composant::Propulseur { variante, .. } => variante.cout(),
            // charpente : treillis évasé, coût qui croît avec la longueur.
            Composant::Charpente { longueur, .. } => treillis::charpente_cout(*longueur),
            Composant::CharpenteHexa { longueur, .. } => treillis::charpente_hexa_cout(*longueur),
            // radiateur méga : grande aile, coût lourd (échelle mégastructure).
            Composant::RadiateurMega { longueur, .. } => radiateur::mega_cout(*longueur),
            // nacelle moteur : très lourde (bloc propulsion complet).
            Composant::Motrice { .. } => propulsion::motrice_cout(),
            Composant::BlocMoteur { largeur, .. } => propulsion::bloc_cout(*largeur),
            Composant::Reservoir { longueur, .. } => reservoir::cout(*longueur),
            // corps + cœur + collier + 6 bobines + 2 cônes + jet ≈ 11.
            Composant::MoteurAntimatiere { .. } => antimatiere::moteur_cout(),
            // coiffe : capuchon léger (collier + dôme/pyramide/couronne) ≈ 6.
            Composant::Coiffe { .. } => adaptateur::coiffe_cout(),
            // réacteur antimatière : cuve + bobines + tuyauterie + tête ≈ 14.
            Composant::ReacteurAntimatiere { .. } => antimatiere::reacteur_cout(),
            // anneau hexagonal en treillis : 6 baies × ~9 barres ≈ 12.
            Composant::TreillisHexagone { .. } => treillis::hexagone_cout(),
            // module d'habitat : fût + 3 armatures triangulaires + ferrure.
            Composant::ModuleHabitat { .. } => habitat::cout(),
            Composant::ModuleEquipage { .. } => equipage::cout(),
            Composant::BouclierPetit { .. } => bouclier::petit_cout(),
            Composant::BouclierGrand { .. } => bouclier::grand_cout(),
            Composant::Panache { .. } => panache::cout(),
            Composant::PanneauMega { longueur, largeur, .. } => panneau_mega::cout(*longueur, *largeur),
            Composant::Tore { rayon_majeur, .. } => tore::cout(*rayon_majeur),
            Composant::BouclierThermique { .. } => thermique::cout(),
            Composant::CollierRotatif { .. } => equipage::collier_cout(),
            Composant::Charniere { .. } => equipage::charniere_cout(),
            // nacelle : prisme + 2 collerettes + 3 rails ≈ 6.
            Composant::NacelleCargo { .. } => cargo::nacelle_cout(),
            // râtelier : la cage (3 anneaux × 2 barres par station) plus le fret.
            Composant::RatelierCargo { nacelles, .. } => cargo::ratelier_cout(*nacelles),
            // sous-ensemble : précalculé une fois à la congélation (somme des
            // enfants), pas reparcouru à chaque appel.
            Composant::SousEnsemble { donnees, .. } => donnees.cout,
            // antenne : coût léger selon le type.
            Composant::Antenne { variante, .. } => variante.cout(),
            // adaptateur : cône + 2 collerettes.
            Composant::Adaptateur { .. } => adaptateur::cout(),
        }
    }

    /// Rayon englobant **local** (remplace l'ancien `Piece.profil` pour la
    /// sphère de `Station`) : la plus grande extension, radiale ou axiale.
    pub fn rayon_local(&self) -> f32 {
        match self {
            Composant::ModuleAxial { profil, longueur, variante } => module_axial::rayon_local(*profil, *variante, *longueur),
            // Sphère + bras + collerette : rayon jusqu'au bout des sorties.
            Composant::Noeud { profil, .. } => noeud::rayon_local(*profil),
            // Diagonale mât+déploiement / demi-largeur (borne haute avec le facteur
            // de longueur max des variantes, ~1.25).
            Composant::PanneauSolaire { longueur, largeur, .. } => panneau_solaire::rayon_local(*longueur, *largeur),
            // Demi-longueur de la poutre (l'extension dominante).
            Composant::Treillis { profil, longueur, .. } => treillis::rayon_local(*profil, *longueur),
            // Diagonale déploiement / demi-largeur (largeur élargie pour « Corps »).
            Composant::Radiateur { longueur, largeur, .. } => {
                (MAST_PANNEAU + longueur * 1.25).hypot(largeur * 0.8)
            }
            // Caisson : platine courte + longueur de la boîte.
            Composant::Caisson { longueur, largeur, .. } => caisson::rayon_local(*longueur, *largeur),
            // Charge à plat : demi-diagonale dans le plan + épaisseur.
            Composant::ChargeUtile { longueur, largeur, .. } => caisson::charge_rayon_local(*longueur, *largeur),
            // Propulseur : le plus long (NERVA, VASIMR) atteint ~1,35 × taille.
            Composant::Propulseur { taille, .. } => propulsion::rayon_local(*taille),
            // Charpente : demi-longueur ou demi-largeur de la base évasée —
            // **aiguille comprise**. Elle pend sous la base, exactement comme la
            // tour du pied de la variante hexagonale juste en dessous ; l'ignorer
            // faisait déclarer 10,0 à une pièce qui s'étend à 17,0 (×1,70), donc
            // une caméra qui la coupe et une sphère de collision qui ment.
            Composant::Charpente { grand, longueur, aiguille, .. } => {
                let (bas, large) = treillis::charpente_pied(*grand, *aiguille);
                let plat = (grand.rayon() * TREILLIS_SECTION * 1.5).max(large);
                (longueur * 0.5 + bas).hypot(large).max(plat)
            }
            // Idem, au circonradius hexagonal (√2 fois la demi-largeur carrée),
            // **tour du pied comprise** : elle pend sous la base, donc c'est elle
            // qui fixe l'extension quand l'aiguille est posée.
            Composant::CharpenteHexa { grand, longueur, pied, .. } => {
                let rg = grand.rayon() * TREILLIS_SECTION * std::f32::consts::SQRT_2;
                let bas = treillis::charpente_hexa_pied(*grand, *pied);
                // Le pavillon déborde en **rayon** autant qu'en longueur : les
                // deux comptent, sinon sa corolle sort de l'englobant.
                let large = rg.max(treillis::charpente_hexa_pied_rayon(*grand, *pied));
                (longueur * 0.5 + bas).hypot(large).max(large * 1.5)
            }
            // Radiateur méga : déploiement (longueur) ou demi-envergure.
            Composant::RadiateurMega { longueur, largeur, .. } => radiateur::mega_rayon_local(*longueur, *largeur),
            // Nacelle moteur : extension max (hub avant → boucliers arrière).
            Composant::Motrice { echelle, .. } => propulsion::motrice_rayon_local(*echelle),
            // Brique bloc-moteur : rangée d'habitats (large en X, longue en −Z).
            Composant::BlocMoteur { largeur, .. } => propulsion::bloc_rayon_local(*largeur),
            // Réservoir : demi-longueur ou bout des barres tétraédriques (r + 2.5r).
            Composant::Reservoir { profil, longueur, .. } => reservoir::rayon_local(*profil, *longueur),
            // Moteur antimatière : la cage de stabilisation atteint ~1,6 × taille.
            Composant::MoteurAntimatiere { taille, .. } => antimatiere::moteur_rayon_local(*taille),
            // Coiffe : nez déployé jusqu'à ~1,4 × rayon vers +Z.
            Composant::Coiffe { profil, .. } => adaptateur::coiffe_rayon_local(*profil),
            // Réacteur antimatière : corps + tête déployés jusqu'à ~1,2 × taille.
            Composant::ReacteurAntimatiere { taille, .. } => antimatiere::reacteur_rayon_local(*taille),
            // Anneau hexagonal (+ montants de liaison le long de +Z).
            // Point le plus loin de l'origine : un sommet de l'hexagone jumeau,
            // donc l'hypoténuse du rayon et de la hauteur du prisme.
            Composant::TreillisHexagone { profil, liaison } => treillis::hexagone_rayon(*profil)
                .hypot(treillis::hexagone_hauteur(*profil, *liaison)),
            // Module d'habitat : demi-longueur, ou la ferrure si elle porte loin.
            Composant::ModuleHabitat { profil, longueur, attache, .. } => habitat::rayon_local(*profil, *longueur, *attache),
            Composant::ModuleEquipage { profil, longueur, .. } => equipage::rayon_local(*profil, *longueur),
            Composant::BouclierPetit { rayon, .. } => bouclier::petit_rayon_local(*rayon),
            Composant::BouclierGrand { rayon, elancement, .. } => bouclier::grand_rayon_local(*rayon, *elancement),
            Composant::Panache { .. } => panache::rayon_local(),
            Composant::PanneauMega { longueur, largeur, .. } => panneau_mega::rayon_local(*longueur, *largeur),
            Composant::Tore { rayon_majeur, rayon_mineur, .. } => tore::rayon_local(*rayon_majeur, *rayon_mineur),
            Composant::BouclierThermique { rayon_pied, rayon_bout, longueur, .. } => thermique::rayon_local(*rayon_pied, *rayon_bout, *longueur),
            Composant::CollierRotatif { rayon, longueur, .. } => equipage::collier_rayon_local(*rayon, *longueur),
            Composant::Charniere { taille, .. } => equipage::charniere_rayon_local(*taille),
            // Nacelle : appendice déployé le long de +Z, la longueur domine.
            Composant::NacelleCargo { profil, longueur, .. } => cargo::nacelle_rayon_local(*profil, *longueur),
            // Râtelier : le coin le plus loin (demi-longueur, station + nacelle).
            // Même disposition que le dessin, donc jamais de divergence.
            Composant::RatelierCargo { longueur, rayon, nacelles, nacelle, .. } => cargo::ratelier_rayon_local(*longueur, *rayon, *nacelles, *nacelle),
            // Sous-ensemble : rayon englobant du sous-arbre, précalculé.
            Composant::SousEnsemble { donnees, .. } => donnees.rayon,
            // Antenne : mât + taille (les fouets/hélice dépassent un peu).
            Composant::Antenne { taille, .. } => antenne::rayon_local(*taille),
            // Adaptateur : jusqu'au bout du col du grand côté.
            Composant::Adaptateur { grand, longueur, .. } => adaptateur::rayon_local(*grand, *longueur),
        }
    }

    /// Sphère englobante **locale** `(centre, rayon)` pour l'anti-collision. Les
    /// composants structurels sont centrés sur l'origine ; les appendices se
    /// déploient le long de +Z, donc leur sphère est décalée à mi-déploiement
    /// (sinon, centrée sur le montage, elle recouvrirait à tort les voisins).
    /// **Enveloppe de collision**, en repère local : une capsule
    /// ([`Enveloppe`]), dont la sphère est le cas dégénéré.
    ///
    /// À ne pas confondre avec [`Self::rayon_local`], qui sert au **cadrage
    /// caméra** et reste un scalaire depuis l'origine locale. Les deux mesurent
    /// l'encombrement, mais pour deux usages qui n'ont pas les mêmes exigences :
    /// la caméra veut une sphère (elle recule dans toutes les directions), la
    /// collision veut coller à la forme.
    pub fn enveloppe_locale(&self) -> Enveloppe {
        // Sphère centrée sur l'origine locale : la pièce est ramassée autour de
        // son point de montage.
        let centree = || Enveloppe::sphere(Vec3::ZERO, self.rayon_local());
        match self {
            // Pièces **ramassées** : leur plus grande dimension n'excède pas
            // franchement leur section, la sphère est le bon compromis et la
            // capsule n'apporterait rien.
            Composant::CollierRotatif { .. }
            | Composant::Charniere { .. }
            | Composant::Noeud { .. }
            | Composant::Adaptateur { .. }
            | Composant::Motrice { .. }
            | Composant::BlocMoteur { .. }
            | Composant::Reservoir { .. } => centree(),

            // --- Pièces **allongées** : capsule couchée sur leur axe ---------
            //
            // Toutes suivent le même patron : l'axe couvre la longueur utile, le
            // rayon vaut la demi-section. La sphère équivalente réservait
            // `hypot(demi_longueur, section)` **dans toutes les directions**,
            // c'est-à-dire 4 à 6 fois la section réelle sur les flancs — là
            // précisément où l'on vient poser les voisins.
            Composant::ModuleAxial { profil, variante, longueur } => {
                let demi = module_axial::rayon_local(*profil, *variante, *longueur);
                Enveloppe::axe(Vec3::ZERO, Vec3::Z, demi, module_axial::demi_section(*profil, *variante))
            }
            Composant::Treillis { profil, longueur, .. } => {
                Enveloppe::axe(Vec3::ZERO, Vec3::Z, longueur * 0.5, treillis::demi_section(*profil))
            }
            // Charpente : cône couché, plus l'aiguille qui pend sous la base.
            // L'axe va donc du bout `+Z` jusque **sous** l'anneau.
            Composant::Charpente { grand, longueur, aiguille, .. } => {
                let (bas, large) = treillis::charpente_pied(*grand, *aiguille);
                let demi = longueur * 0.5;
                Enveloppe::capsule(
                    Vec3::Z * demi,
                    Vec3::NEG_Z * (demi + bas),
                    treillis::demi_section(*grand).max(large),
                )
            }
            Composant::CharpenteHexa { grand, longueur, pied, .. } => {
                let bas = treillis::charpente_hexa_pied(*grand, *pied);
                let large = treillis::charpente_hexa_pied_rayon(*grand, *pied);
                let demi = longueur * 0.5;
                Enveloppe::capsule(
                    Vec3::Z * demi,
                    Vec3::NEG_Z * (demi + bas),
                    treillis::demi_section_hexa(*grand).max(large),
                )
            }
            // Un propulseur s'étend d'un seul côté de son montage, comme les
            // appendices — mais vers l'arrière quand il est axial.
            Composant::Propulseur { variante, taille, .. } => propulsion::englobant(*variante, *taille),
            // Moteur antimatière : masse déployée vers l'arrière (−Z), comme un
            // propulseur axial, sphère décalée à mi-corps.
            Composant::MoteurAntimatiere { taille, .. } => antimatiere::moteur_englobant(*taille),
            // Coiffe : nez déployé vers +Z, sphère décalée à mi-hauteur.
            Composant::Coiffe { profil, .. } => adaptateur::coiffe_englobant(*profil),
            // Réacteur antimatière : masse déployée vers +Z, sphère à mi-corps.
            Composant::ReacteurAntimatiere { taille, .. } => antimatiere::reacteur_englobant(*taille),
            // Anneau hexagonal (+ montants) : englobant centré, borné par la liaison.
            // Capsule le long de la **normale** (+Y local) : l'anneau seul est
            // un disque plat, le prisme (`liaison > 0`) une galette étirée. Une
            // sphère centrée réservait le rayon dans toutes les directions et
            // sous-déclarait dès que les montants dépassaient (§9, L1.6).
            Composant::TreillisHexagone { profil, liaison } => {
                let (_, _, prof) = treillis::hexagone_cotes(*profil);
                Enveloppe::capsule(
                    Vec3::new(0.0, -prof, 0.0),
                    Vec3::new(0.0, treillis::hexagone_hauteur(*profil, *liaison), 0.0),
                    treillis::hexagone_rayon(*profil),
                )
            }
            // Nacelle : déployée d'un seul côté (+Z), sphère décalée à mi-corps
            // — sinon, centrée sur le montage, elle mordrait sur les voisines.
            Composant::NacelleCargo { profil, longueur, .. } => cargo::nacelle_englobant(*profil, *longueur),
            // Module d'équipage : déployé vers l'extérieur depuis la traverse.
            Composant::ModuleEquipage { profil, longueur, .. } => equipage::englobant(*profil, *longueur),
            Composant::BouclierPetit { rayon, .. } => bouclier::petit_englobant(*rayon),
            Composant::BouclierGrand { rayon, elancement, .. } => bouclier::grand_englobant(*rayon, *elancement),
            Composant::Panache { .. } => panache::englobant(),
            // ⚠️ Un tore est **creux** : aucune des trois formes de `Noyau`
            // (point, segment, rectangle) ne l'épouse. La sphère englobante est
            // donc large — elle réserve tout le disque central, où il n'y a
            // rien. Sans conséquence tant que le tore est posé à la main et
            // seul à ce rayon ; à revoir le jour où l'anti-collision devra
            // laisser passer quelque chose **dans** l'anneau.
            // L'aile pivote : la sphère couvre toutes ses orientations.
            Composant::PanneauMega { .. } => centree(),
            Composant::Tore { .. } => centree(),
            Composant::BouclierThermique { rayon_pied, rayon_bout, longueur, .. } => thermique::englobant(*rayon_pied, *rayon_bout, *longueur),
            // Râtelier : trois nacelles en triforce autour de l'axe — aussi
            // large que long, la sphère est juste.
            Composant::RatelierCargo { .. } => centree(),
            // Module d'habitat : fût couché sur son axe.
            Composant::ModuleHabitat { profil, longueur, attache, .. } => Enveloppe::axe(
                Vec3::ZERO,
                Vec3::Z,
                longueur * 0.5,
                habitat::demi_section(*profil, *attache),
            ),
            // Sous-ensemble : centré sur l'origine du sous-ensemble, comme une Station.
            Composant::SousEnsemble { donnees, .. } => Enveloppe::sphere(Vec3::ZERO, donnees.rayon),
            // Appendices **compacts** déployés d'un seul côté (+Z) : sphère
            // décalée à mi-corps. Une antenne ou une charge utile sont aussi
            // larges que longues, la capsule n'y gagnerait rien.
            Composant::Antenne { .. } | Composant::ChargeUtile { .. } => {
                let r = self.rayon_local();
                Enveloppe::sphere(Vec3::Z * (r * 0.5), r * 0.55)
            }
            // Appendices **en aile** : longs et minces, déployés vers +Z depuis
            // leur montage. C'est la famille où la sphère coûtait le plus cher —
            // un radiateur de 3,5 de long sur 0,5 d'épaisseur s'y voyait
            // réserver 2,3 de rayon, soit près de cinq fois sa demi-section.
            Composant::PanneauSolaire { longueur, largeur, .. } => {
                enveloppe_aile(panneau_solaire::rayon_local(*longueur, *largeur), *largeur * 0.5)
            }
            Composant::Radiateur { longueur, largeur, .. } => {
                enveloppe_aile((MAST_PANNEAU + longueur * 1.25).hypot(largeur * 0.8), *largeur * 0.8)
            }
            Composant::Caisson { longueur, largeur, .. } => {
                enveloppe_aile(caisson::rayon_local(*longueur, *largeur), *largeur * 0.8)
            }
            Composant::RadiateurMega { longueur, largeur, .. } => {
                enveloppe_aile(radiateur::mega_rayon_local(*longueur, *largeur), *largeur * 0.55)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Échantillons : une chaîne, pas une liste (`docs/conception/assembleur.md` §5.5)
// ---------------------------------------------------------------------------

/// Échantillon **suivant** dans une chaîne qui passe par les 31 variantes,
/// une fois chacune.
///
/// ⚠️ **Pourquoi une chaîne et pas une simple liste.** Le `match` ci-dessous
/// est *exhaustif* : ajouter une variante à `Composant` **casse la
/// compilation ici**, et la seule façon de réparer est de lui donner un
/// échantillon et de l'insérer dans la chaîne. Une `Vec` d'échantillons, au
/// contraire, se compilerait très bien en oubliant une variante — et les
/// tests de balayage passeraient au vert **en la ratant**, ce qui est
/// exactement le mode de défaillance que ce lot cherche à supprimer
/// (`suivi/stations.md` §C.29 : un test qui mesure autre chose que ce qu'il
/// annonce).
///
/// Les cotes sont celles des tests unitaires de chaque famille : rien de
/// dégénéré, rien d'extrême — on vérifie ici la **santé** de la sortie, pas
/// une dimension.
fn suivante(c: &Composant) -> Option<Composant> {
    let s = match c {
        Composant::ModuleAxial { .. } => Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six },
        Composant::Noeud { .. } => Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 3.0, largeur: 1.2 },
        Composant::PanneauSolaire { .. } => Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre },
        Composant::Treillis { .. } => Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::Caloducs, longueur: 3.0, largeur: 1.0 },
        Composant::Radiateur { .. } => Composant::Antenne { profil: Profil::P0, variante: VarianteAntenne::ParaboleGG, taille: 1.0 },
        Composant::Antenne { .. } => Composant::Adaptateur { grand: Profil::P2, petit: Profil::P1, longueur: 2.0 },
        Composant::Adaptateur { .. } => Composant::Caisson { profil: Profil::P0, variante: VarianteCaisson::Ossature, longueur: 2.0, largeur: 1.0 },
        Composant::Caisson { .. } => Composant::ChargeUtile { profil: Profil::P0, variante: VarianteCharge::TOUS[0], longueur: 1.6, largeur: 0.9 },
        Composant::ChargeUtile { .. } => Composant::Propulseur { profil: Profil::P1, variante: VariantePropulseur::TuyereCloche, taille: 1.5 },
        Composant::Propulseur { .. } => Composant::Charpente { grand: Profil::P3, petit: Profil::P1, longueur: 20.0, courbure: 2.0, aiguille: true },
        Composant::Charpente { .. } => Composant::CharpenteHexa { grand: Profil::P3, petit: Profil::P1, longueur: 20.0, courbure: 2.0, pied: PiedHexa::Pavillon },
        Composant::CharpenteHexa { .. } => Composant::RadiateurMega { profil: Profil::P1, longueur: 30.0, largeur: 8.0, ailettes: 6, chaleur: 1.0 },
        Composant::RadiateurMega { .. } => Composant::Motrice { profil: Profil::P1, echelle: 2.0 },
        Composant::Motrice { .. } => Composant::BlocMoteur { profil: Profil::P1, largeur: 4.0 },
        Composant::BlocMoteur { .. } => Composant::Reservoir { profil: Profil::P2, longueur: 6.0, cage: true },
        Composant::Reservoir { .. } => Composant::MoteurAntimatiere { profil: Profil::P1, taille: 6.0 },
        Composant::MoteurAntimatiere { .. } => Composant::Coiffe { profil: Profil::P1, variante: VarianteCoiffe::TOUS[0] },
        Composant::Coiffe { .. } => Composant::ReacteurAntimatiere { profil: Profil::P1, taille: 6.0 },
        Composant::ReacteurAntimatiere { .. } => Composant::TreillisHexagone { profil: Profil::P1, liaison: 3.0 },
        Composant::TreillisHexagone { .. } => Composant::NacelleCargo { profil: Profil::P1, longueur: 8.0, spin: 0.0 },
        Composant::NacelleCargo { .. } => Composant::RatelierCargo { profil: Profil::P1, longueur: 8.0, rayon: 3.0, nacelles: 3, nacelle: 1.0 },
        Composant::RatelierCargo { .. } => Composant::ModuleHabitat { profil: Profil::P1, longueur: 8.0, spin: 0.0, attache: 3.0 },
        Composant::ModuleHabitat { .. } => Composant::ModuleEquipage { profil: Profil::P1, longueur: 4.0, hublots: 5 },
        Composant::ModuleEquipage { .. } => Composant::BouclierPetit { profil: Profil::P0, rayon: 5.5 },
        Composant::BouclierPetit { .. } => Composant::BouclierGrand { profil: Profil::P0, rayon: 10.0, elancement: BOUCLIER_ELANCEMENT },
        Composant::BouclierGrand { .. } => Composant::BouclierThermique { rayon_pied: 3.5, rayon_bout: 1.7, courbure: 1.5, longueur: 13.0, rangs: 10 },
        Composant::BouclierThermique { .. } => Composant::Panache { longueur: 336.0, rayon_col: 0.15, rayon_bout: 11.0, intensite: 1.0 },
        Composant::Panache { .. } => Composant::PanneauMega {
            profil: Profil::P0,
            variante: VariantePanneauMega::FermeModulaire,
            longueur: panneau_mega::LONGUEUR_TYPE,
            largeur: panneau_mega::LARGEUR_TYPE,
            azimut: 0.4,
            inclinaison: 0.3,
        },
        Composant::PanneauMega { .. } => Composant::Tore { rayon_majeur: 12.0, rayon_mineur: 1.5, segments: 48, anneaux: 12, jonctions: 3, phase: 0.5 },
        Composant::Tore { .. } => Composant::CollierRotatif { profil: Profil::P1, rayon: 3.0, alesage: 2.0, longueur: 3.0 },
        Composant::CollierRotatif { .. } => Composant::Charniere { taille: 1.0, repli: 0.5 },
        Composant::Charniere { .. } => sous_ensemble_echantillon(),
        // Fin de chaîne. C'est la seule variante sans successeur.
        Composant::SousEnsemble { .. } => return None,
    };
    Some(s)
}

/// Un composite non trivial : deux modules bout à bout, figés.
fn sous_ensemble_echantillon() -> Composant {
    let mut ch = Chantier::new();
    let m = || Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };
    ch.racine(m());
    let axial = ch.libres().iter().find(|p| p.genre == GenrePort::ModuleAxial).unwrap().id;
    assert!(ch.poser(axial, m(), 1));
    ch.figer(Profil::P1).expect("un composite de deux modules")
}

/// La chaîne déroulée, du premier au dernier : **31 échantillons**, un par
/// variante. Sert au balayage de couverture (§5.5, tests) et, depuis L2.4, à
/// la palette (ci-dessous) : les mêmes valeurs, pas une seconde liste à tenir
/// à jour en double.
pub fn echantillons() -> Vec<Composant> {
    let depart =
        Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 };
    let mut v = vec![depart];
    while let Some(s) = suivante(v.last().unwrap()) {
        v.push(s);
        assert!(v.len() < 200, "la chaîne d'échantillons boucle");
    }
    v
}

// ---------------------------------------------------------------------------
// Palette : la duale de `Chantier::compatibles` (`docs/conception/assembleur.md` §6.5)
// ---------------------------------------------------------------------------

/// Regroupement d'une variante pour la palette de l'éditeur (façon KSP : une
/// palette permanente organisée par catégories — `STATE.md`, « Décisions
/// prises »). Pure organisation de menu : ne change rien à ce qui se pose où,
/// seul [`Composant::ports`] en décide.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Categorie {
    Structure,
    Habitat,
    Energie,
    Communication,
    Cargo,
    Propulsion,
    Bouclier,
    Composite,
    /// Se pose à la main, hors du système de ports : aucune de ces variantes
    /// n'a de port de montage (`ports()` rend `[]`), donc [`posables`] ne les
    /// proposera jamais, quel que soit le port visé.
    PoseeAMain,
    /// Un effet visuel, pas une pièce (coût nul, aucun port).
    Effet,
}

impl Categorie {
    pub const TOUTES: [Categorie; 10] = [
        Categorie::Structure,
        Categorie::Habitat,
        Categorie::Energie,
        Categorie::Communication,
        Categorie::Cargo,
        Categorie::Propulsion,
        Categorie::Bouclier,
        Categorie::Composite,
        Categorie::PoseeAMain,
        Categorie::Effet,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            Categorie::Structure => "STRUCTURE",
            Categorie::Habitat => "HABITAT",
            Categorie::Energie => "ENERGIE",
            Categorie::Communication => "COMMUNICATION",
            Categorie::Cargo => "CARGO",
            Categorie::Propulsion => "PROPULSION",
            Categorie::Bouclier => "BOUCLIER",
            Categorie::Composite => "COMPOSITE",
            Categorie::PoseeAMain => "POSEE A LA MAIN",
            Categorie::Effet => "EFFET",
        }
    }
}

/// Catégorie de palette d'une variante. Exhaustif à dessein, comme la chaîne
/// d'échantillons ci-dessus : ajouter une variante à `Composant` casse la
/// compilation ici — la couverture de la palette est une propriété du
/// compilateur, pas une discipline à se rappeler.
pub fn categorie(c: &Composant) -> Categorie {
    match c {
        Composant::ModuleAxial { .. }
        | Composant::Noeud { .. }
        | Composant::Treillis { .. }
        | Composant::Charpente { .. }
        | Composant::CharpenteHexa { .. }
        | Composant::Adaptateur { .. }
        | Composant::Coiffe { .. }
        | Composant::BlocMoteur { .. } => Categorie::Structure,
        Composant::ModuleHabitat { .. } | Composant::ModuleEquipage { .. } | Composant::CollierRotatif { .. } => {
            Categorie::Habitat
        }
        Composant::PanneauSolaire { .. }
        | Composant::PanneauMega { .. }
        | Composant::Radiateur { .. }
        | Composant::RadiateurMega { .. } => {
            Categorie::Energie
        }
        Composant::Antenne { .. } => Categorie::Communication,
        Composant::Caisson { .. }
        | Composant::ChargeUtile { .. }
        | Composant::NacelleCargo { .. }
        | Composant::RatelierCargo { .. } => Categorie::Cargo,
        Composant::Propulseur { .. }
        | Composant::Motrice { .. }
        | Composant::MoteurAntimatiere { .. }
        | Composant::ReacteurAntimatiere { .. }
        | Composant::Reservoir { .. } => Categorie::Propulsion,
        Composant::BouclierPetit { .. } | Composant::BouclierGrand { .. } => Categorie::Bouclier,
        Composant::SousEnsemble { .. } => Categorie::Composite,
        // Le tore rejoint les pièces **posées à la main** : comme
        // `TreillisHexagone`, il n'expose aucun port, donc `posables` ne peut
        // structurellement jamais le proposer (L2.4).
        Composant::TreillisHexagone { .. }
        | Composant::BouclierThermique { .. }
        | Composant::Charniere { .. }
        | Composant::Tore { .. } => Categorie::PoseeAMain,
        Composant::Panache { .. } => Categorie::Effet,
    }
}

/// Composants posables sur un port libre de ce genre et ce profil — la
/// **duale** de `Chantier::compatibles`, qui part d'un composant et rend les
/// ports qui l'accepteraient. Ici on part du port et on rend les composants
/// (§6.5 : « il manque énumérer tous les composants posables sur CE port
/// libre »).
///
/// Chaque entrée est un échantillon représentatif ([`echantillons`]) et
/// l'indice de son port de montage — le premier compatible, s'il y en a un.
pub fn posables(genre: GenrePort, profil: Profil) -> Vec<(Composant, usize)> {
    echantillons()
        .into_iter()
        .filter_map(|comp| {
            let idx = comp.ports().iter().position(|p| p.genre.compatible(genre) && p.profil.compatible(profil))?;
            Some((comp, idx))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::vaisseau::GenrePort;
    // 2a — un module axial expose exactement deux écoutilles opposées, de même
    // genre/profil, hauts alignés, aux deux bouts.
    #[test]
    fn module_axial_deux_ports_opposes() {
        let long = 3.0;
        let c = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: long };
        let ports = c.ports();

        // Deux écoutilles axiales (les autres ports sont des montages Surface).
        let axiaux: Vec<_> = ports.iter().filter(|p| p.genre == GenrePort::ModuleAxial).collect();
        assert_eq!(axiaux.len(), 2);
        for p in &axiaux {
            assert_eq!(p.profil, Profil::P1);
        }

        // Avants opposés (somme ≈ 0).
        assert!((axiaux[0].repere.avant() + axiaux[1].repere.avant()).length() < 1e-5);

        // Hauts tous deux alignés sur +Y (roulis cohérent).
        assert!((axiaux[0].repere.haut() - Vec3::Y).length() < 1e-5);
        assert!((axiaux[1].repere.haut() - Vec3::Y).length() < 1e-5);

        // Positions au bout des collerettes : ±(demi-longueur + col) en Z.
        let tip = long * 0.5 + Profil::P1.rayon() * COL_LONG;
        assert!((axiaux[0].repere.pos - vec3(0.0, 0.0, tip)).length() < 1e-5);
        assert!((axiaux[1].repere.pos - vec3(0.0, 0.0, -tip)).length() < 1e-5);

        // 4 ports hôtes Surface radiaux (±X, ±Y).
        assert_eq!(ports.iter().filter(|p| p.genre == GenrePort::Surface).count(), 4);
    }

    // Coût et rayon local cohérents.
    #[test]
    fn cout_et_rayon_local() {
        let c = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 };
        assert_eq!(c.cout(), 5.0);
        // demi-longueur 1.5 + col (0.25 × rayon P1) = 1.75 > rayon P1 → 1.75.
        assert_eq!(c.rayon_local(), 1.75);

        // Module court et gros : le rayon domine.
        let trapu = Composant::ModuleAxial { profil: Profil::P3, variante: VarianteModule::Standard, longueur: 1.0 };
        assert_eq!(trapu.rayon_local(), Profil::P3.rayon()); // 3.0
    }

    // Toutes les variantes d'habitat gardent les 2 écoutilles axiales.
    #[test]
    fn toutes_variantes_module() {
        for v in VarianteModule::TOUS {
            let m = Composant::ModuleAxial { profil: Profil::P1, variante: v, longueur: 3.0 };
            let axiaux = m.ports().iter().filter(|p| p.genre == GenrePort::ModuleAxial).count();
            assert_eq!(axiaux, 2, "{}", v.nom());
        }
    }

    // Adaptateur : 2 écoutilles axiales de profils distincts, avants opposés.
    #[test]
    fn adaptateur_relie_deux_profils() {
        let a = Composant::Adaptateur { grand: Profil::P2, petit: Profil::P1, longueur: 2.0 };
        let ports = a.ports();
        assert_eq!(ports.len(), 2);
        for p in &ports {
            assert_eq!(p.genre, GenrePort::ModuleAxial);
        }
        assert_eq!(ports[0].profil, Profil::P2);
        assert_eq!(ports[1].profil, Profil::P1);
        assert!((ports[0].repere.avant() + ports[1].repere.avant()).length() < 1e-5);
    }

    // Invariant commun : chaque port (bras OU montage Surface) a son **avant**
    // pointant vers l'extérieur ; et le nœud a `attendu` ports « module » (bras).
    fn verifie_ports_sortants(n: &Composant, attendu: usize) {
        let ports = n.ports();
        for p in &ports {
            let dir = p.repere.pos.normalize();
            assert!((p.repere.avant() - dir).length() < 1e-5, "avant sortant {dir:?}");
        }
        let modules = ports
            .iter()
            .filter(|p| matches!(p.genre, GenrePort::ModuleAxial | GenrePort::ModuleRadial))
            .count();
        assert_eq!(modules, attendu);
    }

    // Nœud 6 sorties : 6 ports (2 axiaux + 4 radiaux), avants sortants.
    #[test]
    fn noeud_six_ports_sortants() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
        verifie_ports_sortants(&n, 6);
        let ports = n.ports();
        assert_eq!(ports.iter().filter(|p| p.genre == GenrePort::ModuleAxial).count(), 2);
        assert_eq!(ports.iter().filter(|p| p.genre == GenrePort::ModuleRadial).count(), 4);
        assert_eq!(n.cout(), 13.0); // sphère + 6 × (bras + collerette)
    }

    // Nœud 4 sorties (croix plane), avants sortants.
    #[test]
    fn noeud_quatre_ports() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Quatre };
        verifie_ports_sortants(&n, 4);
        assert_eq!(n.cout(), 9.0); // sphère + 4 × (bras + collerette)
    }

    // Nœud en T : 3 ports dans le plan XZ (barre ±X + tige −Z).
    #[test]
    fn noeud_t_trois_ports_plan_xz() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::T };
        verifie_ports_sortants(&n, 3);
        // Les bras (ports module) restent dans le plan XZ → composante Y nulle.
        for p in n.ports().iter().filter(|p| matches!(p.genre, GenrePort::ModuleAxial | GenrePort::ModuleRadial)) {
            assert!(p.repere.pos.y.abs() < 1e-5, "barre T dans le plan XZ");
        }
        assert_eq!(n.cout(), 7.0); // sphère + 3 × (bras + collerette)
    }

    // Nœud tétraédrique : 4 sorties isotropes, avants sortants (rotation par arc).
    #[test]
    fn noeud_tetra_quatre_sorties() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Tetra };
        verifie_ports_sortants(&n, 4);
        assert_eq!(n.cout(), 9.0);
    }

    // Un module (port axial) est compatible avec un port radial du nœud (genres
    // Axial/Radial groupés, même profil).
    #[test]
    fn module_compatible_avec_noeud() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
        let m = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };
        let radial = n.ports()[2]; // un port radial du nœud
        let montage = m.ports()[1]; // écoutille de montage du module
        assert!(montage.compatible(&radial));
    }

    // Panneau solaire : un unique port Surface, avant vers l'hôte (−Z).
    #[test]
    fn panneau_un_port_montage_aile() {
        let p = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 3.0,
            largeur: 1.2,
        };
        let ports = p.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::Surface);
        assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
    }

    // Un montage d'appendice (Surface) s'apparie avec un autre Surface, pas un module.
    #[test]
    fn panneau_compatibilite() {
        let aile = |v| Composant::PanneauSolaire { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
        let p = aile(VariantePanneau::RigideUS);
        let m = Composant::ModuleAxial { profil: Profil::P0, variante: VarianteModule::Standard, longueur: 2.0 };
        assert!(!p.ports()[0].compatible(&m.ports()[0]), "Surface ≠ ModuleAxial");
        assert!(p.ports()[0].compatible(&aile(VariantePanneau::Futuriste).ports()[0]), "deux appendices Surface");
    }

    // Toutes les variantes exposent bien un unique port Surface.
    #[test]
    fn toutes_variantes_un_port_montage() {
        for v in VariantePanneau::TOUS {
            let p = Composant::PanneauSolaire { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
            let ports = p.ports();
            assert_eq!(ports.len(), 1, "{}", v.nom());
            assert_eq!(ports[0].genre, GenrePort::Surface);
        }
    }

    // Treillis : 2 bouts axiaux opposés + des montages d'ailes par paires ±X.
    #[test]
    fn treillis_bouts_et_montages() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre };
        let ports = t.ports();

        let axiaux: Vec<_> = ports.iter().filter(|p| p.genre == GenrePort::ModuleAxial).collect();
        assert_eq!(axiaux.len(), 2, "deux bouts");
        assert!((axiaux[0].repere.avant() + axiaux[1].repere.avant()).length() < 1e-5, "bouts opposés");

        let ailes: Vec<_> = ports.iter().filter(|p| p.genre == GenrePort::Surface).collect();
        assert!(ailes.len() >= 2 && ailes.len() % 2 == 0, "paires d'ailes");
        for p in &ailes {
            assert_eq!(p.profil, Profil::P0);
            assert!(p.repere.avant().x.abs() > 0.99, "avant latéral ±X"); // sortant sur X
            assert!(p.repere.avant().y.abs() + p.repere.avant().z.abs() < 1e-5);
        }
    }

    // Un panneau (P0) est compatible avec un montage d'aile du treillis (P0).
    #[test]
    fn panneau_dock_sur_treillis() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 6.0, style: StyleTreillis::Triangulaire };
        let aile = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 2.0,
            largeur: 1.0,
        };
        let mont = t.ports().into_iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        assert!(aile.ports()[0].compatible(&mont));
    }

    // Radiateur : un unique port Surface, avant vers l'hôte (−Z).
    #[test]
    fn radiateur_port_montage() {
        let r = Composant::Radiateur {
            profil: Profil::P0,
            variante: VarianteRadiateur::PanneauSimple,
            longueur: 3.0,
            largeur: 1.0,
        };
        let ports = r.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::Surface);
        assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
    }

    // Chaque variante sait à quelle famille elle appartient, et les trois vues
    // se partagent bien la totalité — aucune orpheline.
    #[test]
    fn familles_de_propulsion_partitionnent_les_variantes() {
        let total: usize = FamillePropulsion::TOUTES
            .iter()
            .map(|f| VariantePropulseur::TOUS.iter().filter(|v| v.famille() == *f).count())
            .sum();
        assert_eq!(total, VariantePropulseur::TOUS.len());
        for f in FamillePropulsion::TOUTES {
            assert!(
                VariantePropulseur::TOUS.iter().any(|v| v.famille() == f),
                "famille vide : {}",
                f.nom()
            );
        }
    }

    // Le montage suit l'usage : un moteur principal se boulonne par une écoutille
    // axiale, une grappe de contrôle se pose sur un port `Surface`.
    #[test]
    fn propulseurs_montage_selon_usage() {
        for v in VariantePropulseur::TOUS {
            let prop = Composant::Propulseur { profil: Profil::P1, variante: v, taille: 1.5 };
            let ports = prop.ports();
            assert_eq!(ports.len(), 1, "{}", v.nom());
            let attendu = if v.axial() { GenrePort::ModuleAxial } else { GenrePort::Surface };
            assert_eq!(ports[0].genre, attendu, "{}", v.nom());
            assert!(prop.cout() > 0.0 && prop.rayon_local() > 0.0, "{}", v.nom());
        }
    }

    // Un moteur axial doit pouvoir se poser sur l'écoutille arrière d'un module,
    // et une grappe sur son flanc.
    #[test]
    fn propulseurs_saccouplent_a_un_module() {
        let m = Composant::ModuleAxial {
            profil: Profil::P1,
            variante: VarianteModule::Standard,
            longueur: 3.0,
        };
        let ports = m.ports();
        let axial = ports.iter().find(|p| p.genre == GenrePort::ModuleAxial).unwrap();
        let flanc = ports.iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        let moteur = Composant::Propulseur {
            profil: Profil::P1,
            variante: VariantePropulseur::TuyereCloche,
            taille: 1.5,
        };
        let grappe = Composant::Propulseur {
            profil: Profil::P0,
            variante: VariantePropulseur::GrappeRcs,
            taille: 0.8,
        };
        assert!(axial.compatible(&moteur.ports()[0]));
        assert!(flanc.compatible(&grappe.ports()[0]));
    }

    fn caisson(v: VarianteCaisson) -> Composant {
        Composant::Caisson { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 }
    }

    // Le caisson est un **porteur** : port de montage en index 0 (avant −Z, vers
    // l'hôte) + un port hôte sur chacune de ses cinq faces libres.
    #[test]
    fn toutes_variantes_caisson_montage_plus_faces_hotes() {
        for v in VarianteCaisson::TOUS {
            let c = caisson(v);
            let ports = c.ports();
            assert_eq!(ports.len(), 6, "{}", v.nom());
            assert!(ports.iter().all(|p| p.genre == GenrePort::Surface), "{}", v.nom());
            // Index 0 = montage, tourné vers l'hôte.
            assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5, "{}", v.nom());
            // Les cinq autres regardent vers l'extérieur, jamais vers l'hôte.
            for hote in &ports[1..] {
                assert!(hote.repere.avant().z > -0.5, "{} : face tournée vers l'hôte", v.nom());
            }
            assert!(c.cout() > 0.0 && c.rayon_local() > 0.0, "{}", v.nom());
        }
    }

    // Le point de la refonte : une charge utile doit pouvoir se poser sur le
    // **côté** d'un caisson technique, pas seulement sur une structure.
    #[test]
    fn charge_utile_dock_sur_le_flanc_dun_caisson() {
        let c = caisson(VarianteCaisson::Ossature);
        let ports = c.ports();
        let flancs: Vec<_> = ports[1..]
            .iter()
            .filter(|p| p.repere.avant().x.abs() > 0.9)
            .collect();
        assert!(!flancs.is_empty(), "le caisson doit offrir des faces latérales");
        for v in VarianteCharge::TOUS {
            let ch = Composant::ChargeUtile {
                profil: Profil::P0,
                variante: v,
                longueur: 1.6,
                largeur: 0.9,
            };
            assert_eq!(ch.ports().len(), 1, "{}", v.nom());
            assert!(flancs[0].compatible(&ch.ports()[0]), "{}", v.nom());
        }
    }

    // Un caisson doit pouvoir s'accoupler sur un port hôte de treillis : c'est
    // tout l'intérêt de la famille non pressurisée (équipements sur poutre).
    #[test]
    fn caisson_dock_sur_treillis() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre };
        let hote = t.ports().into_iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        let c = Composant::Caisson {
            profil: Profil::P0,
            variante: VarianteCaisson::Ossature,
            longueur: 2.0,
            largeur: 1.0,
        };
        assert!(hote.compatible(&c.ports()[0]));
    }

    // Factorisation : radiateur, panneau (et antenne) partagent le montage
    // `Surface` → compatibles entre eux ; mais pas avec un port de module.
    #[test]
    fn radiateur_compatibilite() {
        let rad = |v| Composant::Radiateur { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
        let r = rad(VarianteRadiateur::Caloducs);
        let aile = Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 2.0, largeur: 1.0 };
        let module = Composant::ModuleAxial { profil: Profil::P0, variante: VarianteModule::Standard, longueur: 2.0 };
        assert!(r.ports()[0].compatible(&aile.ports()[0]), "montages Surface factorisés");
        assert!(!r.ports()[0].compatible(&module.ports()[0]), "pas sur un port de module");
    }

    // Toutes les variantes de radiateur exposent un unique port Surface.
    #[test]
    fn toutes_variantes_radiateur() {
        for v in VarianteRadiateur::TOUS {
            let r = Composant::Radiateur { profil: Profil::P0, variante: v, longueur: 2.0, largeur: 1.0 };
            assert_eq!(r.ports().len(), 1, "{}", v.nom());
            assert_eq!(r.ports()[0].genre, GenrePort::Surface);
        }
    }

    // Antenne : un unique port Surface, avant vers l'hôte (−Z) ; idem toutes variantes.
    #[test]
    fn toutes_variantes_antenne() {
        for v in VarianteAntenne::TOUS {
            let ant = Composant::Antenne { profil: Profil::P0, variante: v, taille: 1.0 };
            let ports = ant.ports();
            assert_eq!(ports.len(), 1, "{}", v.nom());
            assert_eq!(ports[0].genre, GenrePort::Surface);
            assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
        }
    }

    // Factorisation : un treillis accueille indifféremment panneau, radiateur et
    // antenne sur ses ports hôtes Surface (même profil).
    #[test]
    fn treillis_accueille_tous_appendices() {
        let t = Composant::Treillis { profil: Profil::P1, longueur: 6.0, style: StyleTreillis::Carre };
        let hote = t.ports().into_iter().find(|p| p.genre == GenrePort::Surface).unwrap();
        let panneau = Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 2.0, largeur: 1.0 };
        let radiateur = Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::PanneauSimple, longueur: 2.0, largeur: 1.0 };
        let antenne = Composant::Antenne { profil: Profil::P0, variante: VarianteAntenne::ParaboleGG, taille: 1.0 };
        for app in [panneau, radiateur, antenne] {
            assert!(app.ports()[0].compatible(&hote), "appendice sur port hôte Surface");
        }
    }

    // --- Briques classe C / ISV (audit préalable au découpage de composant.rs,
    // docs/suivi/stations.md "Priorités immédiates") : ces 9 variantes n'avaient
    // aucun test dédié. Couverture minimale par variante : ports (genre, nombre,
    // profils), cout, rayon_local (valeurs figées à partir des formules
    // actuelles — verrou de non-régression pour l'extraction en modules), et
    // dessiner() ne panique pas et produit de la géométrie (accumulée dans un
    // `Batisseur`, sans contexte GL réel nécessaire).
    use super::super::chantier::Chantier;
    use super::super::maillage::Batisseur;

    // --- Composant::SousEnsemble (Partie E.3) : le composite qui gèle un
    // sous-arbre de pièces en une seule brique réutilisable.

    #[test]
    fn figer_chantier_vide_donne_rien() {
        assert!(Chantier::new().figer(Profil::P1).is_none());
    }

    #[test]
    fn figer_expose_les_ports_libres_restants() {
        let mut ch = Chantier::new();
        ch.racine(Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six });
        let sous = ch.figer(Profil::P1).expect("chantier non vide");
        let Composant::SousEnsemble { profil, donnees } = &sous else {
            panic!("figer doit produire un SousEnsemble");
        };
        assert_eq!(*profil, Profil::P1);
        assert_eq!(donnees.pieces.len(), 1);
        // Un Noeud Six expose 6 ports ; aucun n'a été consommé (rien posé dessus).
        assert_eq!(donnees.ports_exposes.len(), 6);
        assert_eq!(sous.ports().len(), 6);
    }

    #[test]
    fn figer_precalcule_cout_et_rayon() {
        let mut ch = Chantier::new();
        let noeud = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
        let cout_attendu = noeud.cout();
        let rayon_attendu = noeud.rayon_local();
        ch.racine(noeud);
        let sous = ch.figer(Profil::P1).unwrap();
        assert_eq!(sous.cout(), cout_attendu);
        assert_eq!(sous.rayon_local(), rayon_attendu);
        assert_eq!(sous.enveloppe_locale(), Enveloppe::sphere(Vec3::ZERO, rayon_attendu));
    }

    #[test]
    fn sous_ensemble_se_clipse_comme_nimporte_quel_composant() {
        // Gèle "nœud + un module dessus" en une brique, puis clipse CETTE
        // brique sur un port libre d'un chantier différent : la composabilité
        // recherchée (assembler plusieurs composants, dont des composites).
        let mut interne = Chantier::new();
        interne.racine(Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six });
        let id = interne.libres()[0].id;
        assert!(interne.poser(id, Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 }, 1));
        let brique = interne.figer(Profil::P1).unwrap();

        let mut externe = Chantier::new();
        externe.racine(Composant::Treillis { profil: Profil::P1, longueur: 4.0, style: StyleTreillis::Carre });
        let port_axial = externe
            .libres()
            .iter()
            .find(|p| p.genre == GenrePort::ModuleAxial)
            .expect("le treillis a des bouts axiaux")
            .id;
        assert!(externe.poser(port_axial, brique, 0), "le composite se pose comme un composant normal");
        assert_eq!(externe.nb_pieces(), 2);
    }

    #[test]
    fn sous_ensemble_dessine_ses_enfants_a_leur_vraie_place() {
        // Racine à l'origine + un second module docké sur son port axial : ce
        // dernier se retrouve décalé le long de Z **dans le repère du
        // sous-ensemble**. En dessinant le composite avec une transformée
        // externe qui décale encore de 10 sur Y, tous les sommets doivent
        // apparaître composés (offset local Z-ish **et** +10 en Y) — preuve
        // qu'`empiler_transforme` compose bien par-dessus la transformée
        // active plutôt que de l'écraser (ce que ferait un simple
        // `poser_transforme` réutilisé tel quel pour chaque enfant).
        let mut ch = Chantier::new();
        ch.racine(Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 1.0 });
        let axial = ch.libres().iter().find(|p| p.genre == GenrePort::ModuleAxial).unwrap().id;
        assert!(ch.poser(axial, Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 1.0 }, 1));
        let sous = ch.figer(Profil::P1).unwrap();

        let mut b = Batisseur::new();
        b.poser_transforme(Mat4::from_translation(vec3(0.0, 10.0, 0.0)));
        sous.dessiner(&mut b);
        let lots = b.terminer();
        assert!(!lots.is_empty());
        // Toute la géométrie doit rester dans une bande autour de Y=10 (rayon
        // des modules ≪ 5) : si `empiler_transforme` ignorait ou écrasait la
        // transformée déjà active, les sommets tomberaient près de Y=0.
        for lot in &lots {
            for v in &lot.vertices {
                assert!(
                    (v.position[1] - 10.0).abs() < 5.0,
                    "sommet non composé avec la transformée active : {:?}",
                    v.position
                );
            }
        }
    }

    // --- Habitat principal d'échelle vaisseau (ISV) — pas les modules rotatifs.

    #[test]
    fn module_equipage_un_port_axial_a_la_base() {
        let c = Composant::ModuleEquipage { profil: Profil::P1, longueur: 7.0, hublots: 8 };
        let ports = c.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::ModuleAxial);
        // Monté par sa base : l'avant regarde la traverse (−Z), le module se
        // déploie vers l'extérieur (+Z), où se trouve le plancher.
        assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
        assert_eq!(c.cout(), 12.0);
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    // --- Épine hexagonale candidate ---------------------------------------

    /// Largeur apparente d'une section vue depuis l'angle `phi`, dans son plan.
    fn largeur_silhouette(section: &[Vec2], phi: f32) -> f32 {
        // On regarde selon `phi` : la largeur est l'extension selon la normale.
        let n = vec2(-phi.sin(), phi.cos());
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for v in section {
            let d = v.dot(n);
            lo = lo.min(d);
            hi = hi.max(d);
        }
        hi - lo
    }

    /// Pire et meilleur angle, balayés finement.
    fn extremes(section: &[Vec2]) -> (f32, f32) {
        let (mut mini, mut maxi) = (f32::MAX, f32::MIN);
        for i in 0..3600 {
            let w = largeur_silhouette(section, TAU * i as f32 / 3600.0);
            mini = mini.min(w);
            maxi = maxi.max(w);
        }
        (mini, maxi)
    }

    // **Le calcul qui justifie le passage en hexagone.** À circonradius égal,
    // l'hexagone doit avoir le même encombrement **maximal** que le carré (donc
    // ne pas grossir l'épine) mais être franchement plus large dans son **pire**
    // angle — c'est le pire angle qui décide de la lisibilité sous filtre pixel.
    #[test]
    fn la_section_hexagonale_est_plus_constante_que_la_carree() {
        let s = 1.5_f32; // demi-largeur du carré actuel
        let r = s * std::f32::consts::SQRT_2; // même circonradius (ses coins)

        let carre: Vec<Vec2> = vec![vec2(-s, -s), vec2(s, -s), vec2(s, s), vec2(-s, s)];
        let hexa: Vec<Vec2> = (0..6)
            .map(|k| {
                let a = std::f32::consts::FRAC_PI_3 * k as f32;
                vec2(r * a.cos(), r * a.sin())
            })
            .collect();

        let (c_min, c_max) = extremes(&carre);
        let (h_min, h_max) = extremes(&hexa);

        // Même silhouette maximale : l'épine hexagonale n'est pas plus grosse.
        assert!(
            (h_max - c_max).abs() < 1e-2,
            "encombrement max différent : carré {c_max:.3} vs hexa {h_max:.3}"
        );
        // Et un gain net dans le pire angle, égal au rapport annoncé.
        let gain = h_min / c_min;
        assert!(
            (gain - crate::vaisseau::pieces::HEXA_GAIN_SILHOUETTE).abs() < 5e-3,
            "gain mesuré {gain:.4}, annoncé {:.4}",
            crate::vaisseau::pieces::HEXA_GAIN_SILHOUETTE
        );
        // Et la variation angulaire est bien celle qui rend le carré capricieux.
        assert!(c_max / c_min > 1.40, "carré : {:.3}", c_max / c_min);
        assert!(h_max / h_min < 1.16, "hexa : {:.3}", h_max / h_min);
    }

    /// Rayon maximal des sommets cuits dont le `z` tombe dans `[z0, z1]`.
    fn rayon_tranche(c: &Composant, z0: f32, z1: f32) -> f32 {
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        let mut r: f32 = 0.0;
        for lot in b.terminer() {
            for v in &lot.vertices {
                let z = v.position[2];
                if (z0..=z1).contains(&z) {
                    r = r.max(vec2(v.position[0], v.position[1]).length());
                }
            }
        }
        r
    }

    // **La tour du pied prolonge le cône, elle ne s'y raccorde pas.** C'est tout
    // l'intérêt d'avoir basculé le cadre de 90° : sa section est désormais
    // parallèle à celle du cône, donc de même rayon, et les longerons descendent
    // tout droit. Deux choses à vérifier — que la tour ne se rétrécit pas (c'est
    // un prisme, pas un second cône) et qu'elle part bien du rayon de base du cône.
    #[test]
    fn la_tour_du_pied_prolonge_le_cone_sans_se_rétrecir() {
        let (grand, petit, longueur) = (Profil::P3, Profil::P0, 40.0);
        let demi = longueur * 0.5;
        let hexa = Composant::CharpenteHexa { grand, petit, longueur, courbure: 2.6, pied: PiedHexa::Tour };

        // Rayon du cône juste au-dessus de sa base, et de la tour juste en dessous.
        let r_cone = rayon_tranche(&hexa, -demi + 0.01, -demi + 1.0);
        let r_haut = rayon_tranche(&hexa, -demi - 1.0, -demi - 0.01);
        assert!(
            (r_cone - r_haut).abs() < 0.05 * r_cone,
            "la tour démarre à {r_haut:.3} alors que la base du cône fait {r_cone:.3}"
        );

        // Et en bas de tour, le même rayon : aucun rétrécissement.
        let pied = treillis::charpente_hexa_pied(grand, PiedHexa::Tour);
        let r_bas = rayon_tranche(&hexa, -demi - pied, -demi - pied + 0.6);
        assert!(
            (r_bas - r_cone).abs() < 0.05 * r_cone,
            "bas de tour à {r_bas:.3} contre {r_cone:.3} en haut : la tour se rétrécit"
        );

        // La tour lit comme une tour : plus haute que large.
        assert!(pied > r_cone, "tour haute de {pied:.3} pour un rayon {r_cone:.3}");
    }

    // **Le pavillon s'ouvre**, là où la tour garde une section constante. C'est
    // toute la demande du schéma : « que le cône s'épanouisse encore plus ».
    #[test]
    fn le_pavillon_souvre_au_lieu_de_prolonger_droit() {
        let (grand, petit, longueur) = (Profil::P3, Profil::P0, 40.0);
        let demi = longueur * 0.5;
        let faire = |pied| Composant::CharpenteHexa { grand, petit, longueur, courbure: 2.6, pied };

        let tour = faire(PiedHexa::Tour);
        let pav = faire(PiedHexa::Pavillon);

        // Au raccord, les deux partent du **même** rayon que la base du cône :
        // l'accostage exact ne doit pas être perdu en changeant de pied.
        let r_cone = rayon_tranche(&tour, -demi + 0.01, -demi + 1.0);
        let r_col_t = rayon_tranche(&tour, -demi - 0.8, -demi - 0.01);
        let r_col_p = rayon_tranche(&pav, -demi - 0.8, -demi - 0.01);
        assert!((r_col_t - r_cone).abs() < 0.08 * r_cone, "tour : col à {r_col_t:.3} vs {r_cone:.3}");
        assert!((r_col_p - r_cone).abs() < 0.15 * r_cone, "pavillon : col à {r_col_p:.3} vs {r_cone:.3}");

        // Mais au bord, le pavillon est **franchement** plus large — et la tour,
        // elle, n'a pas bougé.
        let bas_t = treillis::charpente_hexa_pied(grand, PiedHexa::Tour);
        let bas_p = treillis::charpente_hexa_pied(grand, PiedHexa::Pavillon);
        let bord_t = rayon_tranche(&tour, -demi - bas_t, -demi - bas_t + 0.6);
        let bord_p = rayon_tranche(&pav, -demi - bas_p, -demi - bas_p + 0.6);
        assert!(
            bord_p > bord_t * 1.5,
            "le pavillon ne s'ouvre pas assez : bord {bord_p:.3} contre {bord_t:.3} pour la tour"
        );
        // Et l'englobant suit cette ouverture radiale.
        assert!(
            pav.rayon_local() >= bord_p,
            "englobant {:.3} plus petit que la corolle {bord_p:.3}",
            pav.rayon_local()
        );
    }

    /// Empreinte (étendue en X, étendue en Y) des sommets cuits dont le `z` tombe
    /// dans `[z0, z1]`.
    ///
    /// Mesurer **les deux** axes séparément est le point : `rayon_tranche` ne rend
    /// que la distance maximale à l'axe, or un écrasement selon Y laisse les
    /// sommets portés par X intacts — le rayon maximal ne bouge donc pas d'un
    /// poil. C'est précisément ce qui a laissé passer un col écrasé.
    fn etendues_dessin<F: FnOnce(&mut Batisseur)>(dessin: F, z0: f32, z1: f32) -> (f32, f32) {
        let mut b = Batisseur::new();
        dessin(&mut b);
        let (mut x, mut y) = (0.0f32, 0.0f32);
        for lot in b.terminer() {
            for v in &lot.vertices {
                let z = v.position[2];
                if (z0..=z1).contains(&z) {
                    x = x.max(v.position[0].abs());
                    y = y.max(v.position[1].abs());
                }
            }
        }
        (2.0 * x, 2.0 * y)
    }

    // **Le col du pavillon doit épouser la base du cône**, qui est un hexagone
    // régulier. L'écrasement ne peut donc pas être appliqué d'emblée : il part de 1
    // au col et ne se creuse qu'en descendant vers l'embouchure.
    //
    // 🐛 Ce test existe parce que le premier jet écrasait la section dès le col :
    // les quatre sommets obliques tombaient à un autre Y que ceux du cône et le
    // raccord se voyait. `le_pavillon_souvre_au_lieu_de_prolonger_droit` ne l'avait
    // pas vu — il ne compare que des **rayons**, et l'écrasement selon Y ne change
    // pas le rayon maximal. D'où la mesure des deux étendues séparément.
    #[test]
    fn le_col_du_pavillon_epouse_la_section_du_cone() {
        use crate::vaisseau::pieces::etirement_progressif;
        // Condition de forme, exacte : aucun écrasement au col.
        assert_eq!(etirement_progressif(0.0, treillis::PAVILLON_ETIREMENT), 1.0);
        // ...et l'écrasement demandé bien atteint au bord.
        assert!((etirement_progressif(1.0, 0.55) - 0.55).abs() < 1e-6);

        // **Le pavillon est mesuré seul**, sans le cône. Mesuré sur la charpente
        // complète, la tranche du col contient aussi le cadre de base du cône —
        // un hexagone régulier dont l'épaisseur déborde de part et d'autre du plan
        // de jonction. C'est *lui* qu'on mesurerait, et le test resterait vert avec
        // un col écrasé : vérifié.
        let (r_col, r_bord, hauteur) = (2.0_f32, 4.2, 4.0);
        let e = treillis::PAVILLON_ETIREMENT;
        let pavillon = |b: &mut Batisseur| {
            crate::vaisseau::pieces::pavillon_hexagonal(
                b, Vec3::ZERO, r_col, r_bord, hauteur, e, 3, COULEUR, SOMBRE,
            )
        };

        // Au col : hexagone **régulier**, dont l'empreinte vaut 2R en X et R√3 en
        // Y, soit un rapport de 2/√3 ≈ 1,155.
        let (lx, ly) = etendues_dessin(pavillon, -hauteur * 0.10, -0.01);
        let regulier = 2.0 / 3.0_f32.sqrt();
        assert!(
            (lx / ly - regulier).abs() < 0.12,
            "au col, largeur/hauteur = {:.3} au lieu de {regulier:.3} : le col est déjà écrasé, \
             il ne peut pas épouser la base du cône",
            lx / ly
        );

        // À l'embouchure, en revanche, la pierre est bien taillée.
        let (mx, my) = etendues_dessin(pavillon, -hauteur, -hauteur * 0.90);
        assert!(
            mx / my > 1.7,
            "à l'embouchure, largeur/hauteur = {:.3} : l'écrasement ne s'est pas installé",
            mx / my
        );
    }

    // **La tour qui couronne le pavillon se pose sur l'embouchure**, donc sur une
    // section **écrasée** — pas sur un hexagone régulier comme la tour du pied.
    //
    // C'est le même piège que le col, à l'autre bout : `tour_hexagonale` dessinait
    // une section régulière, et la réutiliser telle quelle aurait recréé le
    // désaccord tout juste corrigé. On mesure donc de part et d'autre du plan
    // d'embouchure, en X **et** en Y — un rapport ne suffit pas d'un seul côté.
    #[test]
    fn la_tour_du_pavillon_reprend_la_section_de_lembouchure() {
        let (grand, petit, longueur) = (Profil::P3, Profil::P0, 40.0);
        let demi = longueur * 0.5;
        let pav =
            Composant::CharpenteHexa { grand, petit, longueur, courbure: 2.6, pied: PiedHexa::Pavillon };
        let dessin = |b: &mut Batisseur| pav.dessiner(b);

        // Plan de l'embouchure : fin de la corolle, début de la tour.
        let bouche = -demi - treillis::charpente_hexa_embouchure(grand);
        let bas = treillis::charpente_hexa_pied(grand, PiedHexa::Pavillon);
        let h_tour = bas - treillis::charpente_hexa_embouchure(grand);

        // ⚠️ On échantillonne **aux niveaux**, pas entre eux : un cylindre cuit ne
        // porte de sommets qu'à ses deux bouts, si bien qu'une tranche prise au
        // milieu d'une baie est **vide** — la mesure y rendait NaN.
        let (bx, by) = etendues_dessin(dessin, bouche - 0.5, bouche + 0.5);
        let (tx, ty) = etendues_dessin(dessin, -demi - bas - 0.5, -demi - bas + 0.5);

        let (r_bouche, r_tour) = (bx / by, tx / ty);

        // L'embouchure est bien écrasée…
        assert!(r_bouche > 1.7, "embouchure à {r_bouche:.3} : pas écrasée");
        // …et la tour **aussi**, au lieu d'être régulière (1,155). C'est
        // l'assertion qui attrape une `tour_hexagonale` laissée à `etirement = 1`.
        assert!(
            r_tour > 1.7,
            "la tour est à {r_tour:.3} : section régulière posée sur une embouchure écrasée"
        );
        // Les deux sections se ressemblent : la tour prolonge l'embouchure.
        assert!(
            (r_tour - r_bouche).abs() < 0.25,
            "embouchure {r_bouche:.3} contre tour {r_tour:.3} : les deux sections divergent"
        );
        // Et la tour ne se rétrécit pas : elle reprend le rayon de l'embouchure.
        assert!(
            (tx - bx).abs() < 0.12 * bx,
            "tour large de {tx:.3} contre {bx:.3} à l'embouchure : elle n'est pas droite"
        );
        // **Gabarit du fût.** Ce n'est plus la virole d'interface d'origine : la
        // hauteur a été multipliée par six (2026-07-30) pour en faire le fût qui
        // porte la propulsion. L'assertion précédente — « reste plus courte que la
        // moitié de l'embouchure » — disait donc l'inverse de l'intention actuelle
        // et a été remplacée, pas assouplie.
        //
        // Ce qui reste à tenir : un fût à peu près aussi haut que l'embouchure est
        // large. Plus court, il redevient une bague ; beaucoup plus long, il
        // rallonge le vaisseau du mauvais côté et concurrence l'épine elle-même.
        let elance = h_tour / tx;
        assert!(
            (0.7..1.6).contains(&elance),
            "fût haut de {h_tour:.3} pour une embouchure large de {tx:.3} (rapport {elance:.2})"
        );
        // Et il reste une extrémité, pas une seconde épine.
        assert!(
            h_tour < longueur * 0.35,
            "fût de {h_tour:.3} sur une épine de {longueur} : il concurrence la charpente"
        );
        // L'englobant tient compte de la tour, sinon la pièce serait sous-estimée.
        assert!(pav.rayon_local() > (demi + bas) * 0.99);
    }

    // **Les A…F du schéma.** L'embouchure doit avoir **deux familles
    // d'arêtes** : quatre obliques égales, et deux (perpendiculaires à Y) égales
    // entre elles mais différentes des autres. Un hexagone régulier n'en aurait
    // qu'une seule famille — c'est l'écrasement selon Y qui les sépare.
    #[test]
    fn lembouchure_a_quatre_aretes_obliques_et_deux_droites() {
        // La **vraie** constante de production, pas une copie : sinon le test
        // resterait vert alors que la pièce livrée aurait perdu son écrasement.
        let (r, etirement) = (3.0_f32, treillis::PAVILLON_ETIREMENT);
        let v = crate::vaisseau::pieces::hexa_section(Vec3::ZERO, Vec3::X, Vec3::Y * etirement, r);
        let cote = |i: usize| (v[(i + 1) % 6] - v[i]).length();

        // Sommets 0 et 3 sont sur ±X : les arêtes 1-2 (haut) et 4-5 (bas) sont les
        // deux horizontales, ce sont les C et F du schéma.
        let (c, f) = (cote(4), cote(1));
        let obliques = [cote(0), cote(2), cote(3), cote(5)];

        // C = F.
        assert!((c - f).abs() < 1e-4, "C={c:.4} et F={f:.4} devraient être égales");
        // A = B = D = E.
        for (i, o) in obliques.iter().enumerate() {
            assert!(
                (o - obliques[0]).abs() < 1e-4,
                "arête oblique {i} = {o:.4}, attendu {:.4}",
                obliques[0]
            );
        }
        // Et les deux familles sont bien **distinctes** : sans ça la contrainte du
        // schéma serait satisfaite par un hexagone régulier, donc vide de sens.
        assert!(
            (obliques[0] - c).abs() > 0.05 * r,
            "les deux familles se confondent ({:.4} vs {c:.4}) : l'écrasement ne sert à rien",
            obliques[0]
        );

        // **Silhouette « taille émeraude »** : deux longs côtés dominants et quatre
        // biseaux courts. Le simple fait que les deux familles diffèrent ne suffit
        // pas — à 0,82 elles différaient déjà (rapport 1,15) et la section lisait
        // comme un hexagone vaguement irrégulier. C'est le **contraste** qui fait
        // la forme, d'où un seuil sur le rapport et non sur une simple inégalité.
        let contraste = c / obliques[0];
        assert!(
            contraste > 1.35,
            "grand côté / biseau = {contraste:.2} : trop peu contrasté pour lire une pierre taillée"
        );
        // Mais pas au point d'aplatir l'hexagone en losange : les biseaux doivent
        // rester de vraies arêtes.
        assert!(
            contraste < 1.9,
            "grand côté / biseau = {contraste:.2} : la section s'aplatit, les biseaux disparaissent"
        );
        // Empreinte nettement plus large que haute, comme une table de pierre.
        let (largeur, hauteur) = (2.0 * r, 2.0 * v[1].y);
        let allonge = largeur / hauteur;
        assert!(
            (1.8..2.8).contains(&allonge),
            "largeur/hauteur = {allonge:.2}, hors du gabarit émeraude visé"
        );
        // Les horizontales gardent la longueur du rayon (elles sont portées par X,
        // que l'écrasement ne touche pas).
        assert!((c - r).abs() < 1e-4, "C devrait valoir le rayon {r}, vaut {c:.4}");
    }

    // Sans aiguille, rien ne doit dépasser sous la base : c'est la tour, et elle
    // seule, qui allonge la pièce vers l'arrière.
    #[test]
    fn sans_aiguille_la_charpente_hexa_sarrete_a_sa_base() {
        let (grand, petit, longueur) = (Profil::P3, Profil::P0, 40.0);
        let demi = longueur * 0.5;
        let nue = Composant::CharpenteHexa { grand, petit, longueur, courbure: 2.6, pied: PiedHexa::Aucun };
        let mut b = Batisseur::new();
        nue.dessiner(&mut b);
        for lot in b.terminer() {
            for v in &lot.vertices {
                assert!(
                    v.position[2] >= -demi - 0.4,
                    "sommet à z={} sous la base {}", v.position[2], -demi
                );
            }
        }
        // Et l'englobant reflète la différence entre les deux.
        let avec = Composant::CharpenteHexa { grand, petit, longueur, courbure: 2.6, pied: PiedHexa::Tour };
        assert!(
            avec.rayon_local() > nue.rayon_local(),
            "la tour n'est pas comptée dans l'extension de la pièce"
        );
    }

    #[test]
    fn charpente_hexa_expose_les_memes_ports_que_la_carree() {
        let (grand, petit, longueur) = (Profil::P3, Profil::P0, 40.0);
        let carre = Composant::Charpente { grand, petit, longueur, courbure: 2.6, aiguille: true };
        let hexa = Composant::CharpenteHexa { grand, petit, longueur, courbure: 2.6, pied: PiedHexa::Tour };
        let (pc, ph) = (carre.ports(), hexa.ports());
        assert_eq!(pc.len(), ph.len());
        for (a, b) in pc.iter().zip(ph.iter()) {
            assert_eq!(a.genre, b.genre);
            assert_eq!(a.profil, b.profil);
            assert!((a.repere.pos - b.repere.pos).length() < 1e-5);
        }
        // Et elle dessine quelque chose, cadre compris.
        let mut b = Batisseur::new();
        hexa.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    // --- Bouclier thermique d'épine ----------------------------------------

    /// Sommets du bardage, groupés par **niveau axial** : rayon max à chaque
    /// cote où la pièce a de la matière. Une nappe cuite n'a de sommets qu'aux
    /// bords de ses facettes, donc interroger une tranche quelconque ne
    /// donnerait rien — c'est le piège déjà payé sur les cylindres et les cônes.
    fn niveaux_du_bardage(c: &Composant) -> Vec<(f32, f32)> {
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        let mut n: Vec<(f32, f32)> = Vec::new();
        for lot in b.terminer() {
            for v in &lot.vertices {
                let (z, r) = (v.position[2], vec2(v.position[0], v.position[1]).length());
                match n.iter_mut().find(|(cz, _)| (*cz - z).abs() < 1e-3) {
                    Some(e) => e.1 = e.1.max(r),
                    None => n.push((z, r)),
                }
            }
        }
        n.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        n
    }

    // Deux propriétés, et il faut les deux — la première seule ne dit presque
    // rien (vérifié : elle reste verte avec un recouvrement **nul**, où les
    // écailles se touchent bord à bord au lieu de se chevaucher).
    //
    // 1. le rayon **monte puis retombe** à chaque rang : l'écaille se relève
    //    jusqu'à sa lèvre, la suivante repart plaquée contre l'épine. C'est ce
    //    qui sépare un bardage d'un manchon conique, lequel ne redescend jamais ;
    // 2. les écailles se **recouvrent vraiment** : le bord libre d'un rang est
    //    axialement **au-delà** du bord plaqué du rang suivant. C'est là que
    //    tient toute la fonction — une écaille libre de se dilater sous sa
    //    voisine, et un flux qui ne rencontre jamais de tranche de face. Bord à
    //    bord, la pièce a exactement la même silhouette et ne protège plus rien.
    #[test]
    fn les_ecailles_du_bardage_se_recouvrent() {
        let (rayon, rangs) = (1.25_f32, 13usize);
        // Cas **cylindrique** (pied = bout) : l'imbrication se mesure sur le
        // rayon, et un évasement la masquerait derrière sa propre pente.
        let n = niveaux_du_bardage(&Composant::BouclierThermique {
            rayon_pied: rayon,
            rayon_bout: rayon,
            courbure: 1.0,
            longueur: 16.0,
            rangs,
        });

        let retombees = n.windows(2).filter(|w| w[1].1 < w[0].1 - 1e-4).count();
        assert!(
            retombees >= rangs - 2,
            "{retombees} retombées de rayon pour {rangs} rangs : ce n'est plus un bardage mais un manchon"
        );

        // Bords **plaqués** (rayon nu) et bords **libres** (rayon max), triés.
        let maxi = n.iter().fold(0.0f32, |m, (_, r)| m.max(*r));
        let marge = (maxi - rayon) * 0.05;
        let niveaux = |garder: &dyn Fn(f32) -> bool| -> Vec<f32> {
            n.iter().filter(|(_, r)| garder(*r)).map(|(z, _)| *z).collect()
        };
        let plaques = niveaux(&|r| r < rayon + marge);
        let libres = niveaux(&|r| r > maxi - marge);
        assert_eq!(plaques.len(), rangs, "un bord plaqué par rang attendu");
        assert_eq!(libres.len(), rangs, "un bord libre par rang attendu");

        let pas = plaques[1] - plaques[0];
        for j in 0..rangs - 1 {
            let chevauchement = (libres[j] - plaques[j + 1]) / pas;
            assert!(
                chevauchement > 0.2,
                "rang {j} : le bord libre déborde de {chevauchement:.2} pas sur le rang suivant — \
                 les écailles se touchent au lieu de se recouvrir"
            );
        }
    }

    // « Pas très épais » : le bardage **habille** l'épine, il ne la double pas.
    // La borne est en dur et non lue sur la constante — c'est justement la
    // consigne qu'on garde, pas la valeur du jour.
    #[test]
    fn le_bardage_thermique_reste_mince() {
        let rayon = 1.25_f32;
        let n = niveaux_du_bardage(&Composant::BouclierThermique {
            rayon_pied: rayon,
            rayon_bout: rayon,
            courbure: 1.0,
            longueur: 16.0,
            rangs: 13,
        });
        let maxi = n.iter().fold(0.0f32, |m, (_, r)| m.max(*r));
        let epaisseur = (maxi - rayon) / rayon;
        assert!(
            epaisseur < 0.20,
            "saillie de {:.0} % du rayon : ce ne sont plus des écailles mais des ailettes",
            epaisseur * 100.0
        );
        assert!(epaisseur > 0.04, "saillie de {epaisseur:.3} : le relief ne se verra pas");
    }

    // Le bardage occupe une **place réservée** sur l'épine, entre le pied et le
    // fret. S'il débordait de la longueur annoncée il entrerait dans l'un ou
    // l'autre, et l'englobant mentirait. Le dernier rang déborde du sien de tout
    // le recouvrement : c'est le pas qui doit en tenir compte, pas la pièce qui
    // doit dépasser.
    #[test]
    fn le_bardage_thermique_tient_dans_sa_longueur() {
        for (longueur, rangs) in [(16.0_f32, 13usize), (9.0, 5), (36.0, 30)] {
            let n = niveaux_du_bardage(&Composant::BouclierThermique {
                rayon_pied: 1.25,
                rayon_bout: 1.25,
                courbure: 1.0,
                longueur,
                rangs,
            });
            let (bas, haut) = (n[0].0, n[n.len() - 1].0);
            assert!(
                (-1e-3..=1e-3).contains(&bas),
                "bardage commençant à {bas:.3} au lieu de 0 ({rangs} rangs)"
            );
            assert!(
                (haut - longueur).abs() < 1e-3,
                "bardage finissant à {haut:.3} pour une longueur annoncée de {longueur} ({rangs} rangs)"
            );
        }
    }

    // --- Boucliers de tête -------------------------------------------------

    // Une plaque s'**enfile** sur le mât commun aux quatre boucliers : elle doit
    // donc offrir une sortie de l'autre côté, sinon la pile ne peut pas se
    // chaîner et il faudrait poser chaque plaque à la main.
    #[test]
    fn les_boucliers_setirent_sur_un_mat_traversant() {
        for c in [
            Composant::BouclierPetit { profil: Profil::P1, rayon: 3.0 },
            Composant::BouclierGrand { profil: Profil::P1, rayon: 4.2, elancement: 1.75 },
        ] {
            let ports = c.ports();
            assert_eq!(ports.len(), 2, "{c:?} : une plaque enfilée a deux sorties");
            assert!(ports.iter().all(|p| p.genre == GenrePort::ModuleAxial));
            // Un port de chaque côté, et **dos à dos** : celui de +Z regarde
            // l'avant, celui de −Z regarde le vaisseau.
            let avant = ports.iter().find(|p| p.repere.pos.z > 0.0).expect("port avant");
            let arriere = ports.iter().find(|p| p.repere.pos.z < 0.0).expect("port arrière");
            assert!((avant.repere.avant() - Vec3::Z).length() < 1e-5);
            assert!((arriere.repere.avant() - Vec3::NEG_Z).length() < 1e-5);

            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            assert!(!b.terminer().is_empty());
        }
    }

    // Une plaque est **symétrique de part et d'autre de son plan** — c'est ce
    // qui la distingue de toutes les autres pièces, montées par un bout. Son
    // englobant est donc centré sur l'origine, un **boudin** (noyau rectangle,
    // `conception/assembleur.md` §9) et non plus une sphère — c'est tout le
    // point du chantier — et sa géométrie doit rester mince en Z : une plaque
    // qui s'épaissirait ne serait plus une plaque.
    #[test]
    fn une_plaque_de_bouclier_reste_mince_et_centree() {
        for (c, rayon) in [
            (Composant::BouclierPetit { profil: Profil::P1, rayon: 3.0 }, 3.0_f32),
            (Composant::BouclierGrand { profil: Profil::P1, rayon: 4.2, elancement: 1.75 }, 4.2),
        ] {
            let env = c.enveloppe_locale();
            let centre = env.centre();
            assert!(
                matches!(env.noyau, crate::vaisseau::Noyau::Rectangle { .. }),
                "{c:?} : une plaque veut un boudin (noyau rectangle), pas une capsule"
            );
            assert_eq!(centre, Vec3::ZERO, "{c:?} : englobant décentré");
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            let mut epaisseur = 0.0f32;
            for lot in b.terminer() {
                for v in &lot.vertices {
                    let p = vec3(v.position[0], v.position[1], v.position[2]);
                    assert!(env.contient(p), "sommet {p:?} hors de l'englobant (profondeur {:.3})", env.profondeur(p));
                    epaisseur = epaisseur.max(p.z.abs());
                }
            }
            // Le moyeu et les nervures dépassent, c'est voulu ; ce qui ne doit
            // pas arriver, c'est qu'ils prennent le pas sur la plaque.
            assert!(
                epaisseur < rayon * 0.20,
                "demi-épaisseur {epaisseur:.2} pour un rayon {rayon} : ce n'est plus une plaque"
            );
        }
    }

    // Une nappe cousue à l'envers ne s'affiche **pas du tout** : macroquad ne
    // double-face pas les triangles. C'est une panne muette — la plaque perd une
    // face et rien ne le signale, ni au compilateur ni aux autres tests. On
    // vérifie donc que chaque peau regarde bien dehors, en isolant ses triangles
    // par leur cote (les deux peaux sont les seules surfaces rigoureusement
    // planes de la pièce ; cônes et tubes n'ont jamais trois sommets à la même).
    #[test]
    fn les_deux_peaux_dune_plaque_regardent_chacune_dehors() {
        for (c, rayon) in [
            (Composant::BouclierPetit { profil: Profil::P1, rayon: 3.0 }, 3.0_f32),
            (Composant::BouclierGrand { profil: Profil::P1, rayon: 4.2, elancement: 1.75 }, 4.2),
        ] {
            let peau = bouclier::demi_epaisseur(rayon);
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            let (mut devant, mut derriere) = (0usize, 0usize);
            for lot in b.terminer() {
                for t in lot.indices.chunks_exact(3) {
                    let p: Vec<Vec3> = t
                        .iter()
                        .map(|i| {
                            let v = lot.vertices[*i as usize].position;
                            vec3(v[0], v[1], v[2])
                        })
                        .collect();
                    let cote = |z: f32| (p[0].z - z).abs() < 1e-4 && (p[1].z - z).abs() < 1e-4 && (p[2].z - z).abs() < 1e-4;
                    let nz = (p[1] - p[0]).cross(p[2] - p[0]).z;
                    if cote(peau) {
                        assert!(nz > 0.0, "{c:?} : triangle de face avant cousu à l'envers (nz={nz:.4})");
                        devant += 1;
                    } else if cote(-peau) {
                        assert!(nz < 0.0, "{c:?} : triangle de face arrière cousu à l'envers (nz={nz:.4})");
                        derriere += 1;
                    }
                }
            }
            // Et les deux peaux existent bel et bien : un test qui ne trouve
            // aucun triangle passerait en vert sans rien avoir mesuré.
            assert!(devant >= 12, "{c:?} : {devant} triangles de face avant seulement");
            assert!(derriere >= 12, "{c:?} : {derriere} triangles de face arrière seulement");
        }
    }

    // L'étirement est **toute** la différence de forme entre les deux plaques :
    // la grande doit être franchement plus haute que large, la petite
    // rigoureusement régulière. Mesuré sur la géométrie cuite et non sur la
    // constante, sinon le test ne dit rien de ce qui est dessiné.
    #[test]
    fn la_grande_plaque_est_elancee_la_petite_reguliere() {
        let mesurer = |c: &Composant| {
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            let (mut x, mut y) = (0.0f32, 0.0f32);
            for lot in b.terminer() {
                for v in &lot.vertices {
                    x = x.max(v.position[0].abs());
                    y = y.max(v.position[1].abs());
                }
            }
            y / x
        };
        // Hexagone régulier pointe en haut : demi-hauteur R, demi-largeur
        // R·cos30 = 0,866 R, donc un rapport de 1,155 et pas de 1.
        let petit = mesurer(&Composant::BouclierPetit { profil: Profil::P1, rayon: 3.0 });
        assert!(
            (petit - 2.0 / 3.0f32.sqrt()).abs() < 0.02,
            "petit bouclier : rapport hauteur/largeur {petit:.3}, il n'est plus régulier"
        );
        // L'étirement doit **arriver jusqu'à la géométrie** : c'est le seul
        // moyen de savoir que la constante n'est pas restée en chemin.
        //
        // Mesuré d'une **grande plaque à l'autre** et non contre la petite : le
        // rognage des pointes raccourcit la hauteur de ≈ TAB/2, si bien que le
        // rapport grand/petit ne vaut plus l'élancement. Entre deux grandes il
        // se simplifie, puisqu'elles portent le même méplat.
        let grand = |e: f32| {
            mesurer(&Composant::BouclierGrand { profil: Profil::P1, rayon: 3.0, elancement: e })
        };
        let etire = grand(bouclier::ELANCEMENT) / grand(1.0);
        assert!(
            (etire - bouclier::ELANCEMENT).abs() < 0.02,
            "étirement mesuré {etire:.3} au lieu de {:.3}",
            bouclier::ELANCEMENT
        );
        // Et la grande reste franchement plus élancée que la petite, méplat
        // compris — sans quoi les deux pièces ne se distinguent plus de loin.
        assert!(
            grand(bouclier::ELANCEMENT) > petit * 1.15,
            "grand bouclier : rapport {:.3} contre {petit:.3} pour le petit",
            grand(bouclier::ELANCEMENT)
        );
        // Et l'élancement doit rester dans la fourchette relevée sur le schéma
        // (rapport hauteur/largeur ≈ 1,35, cliché pris de biais donc un peu
        // plus). En dessous elle ne se distingue plus de la petite ; au-dessus
        // elle redevient la pierre taillée en long qu'on a corrigée.
        assert!(
            (1.15..=1.5).contains(&bouclier::ELANCEMENT),
            "élancement {} hors de ce que montre le schéma",
            bouclier::ELANCEMENT
        );
    }

    // La grande plaque est **rétrécie en largeur seule**, sans que rien d'autre
    // bouge. Le vérifier demande de comparer deux plaques de **même rayon** : à
    // rayon égal, la petite donne la largeur pleine et la grande la largeur
    // rabotée, si bien que leur rapport isole exactement le facteur cherché.
    //
    // Il ne suffit pas de mesurer la largeur : la raboter en réduisant le rayon
    // aurait donné la même largeur tout en emportant le moyeu, dont l'alésage ne
    // laisse que 0,012 de jeu au mât. On vérifie donc aussi que le **moyeu n'a
    // pas bougé** — c'est là qu'était le vrai risque.
    #[test]
    fn la_grande_plaque_est_retrecie_en_largeur_seule() {
        let rayon = 3.0_f32;
        let peau = |c: &Composant| {
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            let mut large = 0.0f32;
            let mut alesage = f32::MAX;
            for lot in b.terminer() {
                for v in &lot.vertices {
                    let p = vec3(v.position[0], v.position[1], v.position[2]);
                    if (p.z.abs() - bouclier::demi_epaisseur(rayon)).abs() < 1e-4 {
                        large = large.max(p.x.abs());
                    }
                    // Sommets du moyeu : les seuls à vivre loin du plan médian.
                    if p.z.abs() > rayon * 0.05 {
                        alesage = alesage.min(p.xy().length());
                    }
                }
            }
            (large, alesage)
        };
        let (l_petit, a_petit) = peau(&Composant::BouclierPetit { profil: Profil::P1, rayon });
        let (l_grand, a_grand) =
            peau(&Composant::BouclierGrand { profil: Profil::P1, rayon, elancement: 1.3 });
        let rabot = l_grand / l_petit;
        assert!(
            (rabot - bouclier::ETROITESSE).abs() < 0.02,
            "largeur rabotée d'un facteur {rabot:.3} au lieu de {:.3}",
            bouclier::ETROITESSE
        );
        assert!(
            (a_grand - a_petit).abs() < 1e-3,
            "alésage {a_grand:.3} contre {a_petit:.3} : le moyeu a suivi le rétrécissement, \
             le mât commun ne passera plus"
        );
    }

    // Les deux **longs bords** — les seuls parallèles à Y — ont été raccourcis de
    // moitié en remontant les épaules vers le milieu. La cote qui compte est leur
    // longueur *rapportée à la hauteur*, parce que c'est elle qui décrit la
    // silhouette : un hexagone régulier étiré donne exactement 0,5, et la moitié
    // de ça vise ≈ 0,27 une fois les pointes rognées.
    //
    // Borné des deux côtés. Trop long, le raccourcissement n'a pas eu lieu ; trop
    // court, les épaules se rejoignent et la plaque devient un losange — elle
    // perd les deux bords parallèles qui font toute sa forme.
    #[test]
    fn les_longs_bords_dune_grande_plaque_sont_reduits_de_moitie() {
        let rayon = 4.2_f32;
        let peau = bouclier::demi_epaisseur(rayon);
        let c = Composant::BouclierGrand { profil: Profil::P1, rayon, elancement: 1.3 };
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        let nappe: Vec<Vec3> = b
            .terminer()
            .iter()
            .flat_map(|l| l.vertices.iter())
            .map(|v| vec3(v.position[0], v.position[1], v.position[2]))
            .filter(|v| (v.z.abs() - peau).abs() < 1e-4)
            .collect();
        let (haut, large) = nappe.iter().fold((0.0f32, 0.0f32), |(h, l), v| (h.max(v.y), l.max(v.x)));
        // Le long bord vit à l'abscisse extrême : sa longueur est l'amplitude en
        // Y des sommets qui s'y trouvent.
        let bord: Vec<f32> = nappe
            .iter()
            .filter(|v| (v.x.abs() - large).abs() < 1e-3)
            .map(|v| v.y)
            .collect();
        assert!(!bord.is_empty(), "aucun sommet au bord : rien n'a été mesuré");
        let long = bord.iter().fold(f32::MIN, |m, y| m.max(*y))
            - bord.iter().fold(f32::MAX, |m, y| m.min(*y));
        let part = long / (2.0 * haut);
        assert!(
            part < 0.40,
            "long bord de {long:.2} pour une hauteur de {:.2} ({part:.3}) : \
             c'est encore la proportion d'un hexagone régulier (0,5)",
            2.0 * haut
        );
        assert!(
            part > 0.12,
            "long bord de {long:.2} pour une hauteur de {:.2} ({part:.3}) : \
             les épaules se rejoignent, la plaque n'est plus qu'un losange",
            2.0 * haut
        );
    }

    // Les deux pointes d'une grande plaque sont **rognées** d'un méplat, en haut
    // comme en bas. Deux façons de le rater, opposées et toutes deux muettes :
    // ne pas le poser du tout (la plaque reste pointue), ou le poser si large
    // que la plaque devient un tonneau. On mesure donc la largeur du bord droit
    // à l'extrémité, et on la borne des deux côtés.
    #[test]
    fn les_pointes_dune_grande_plaque_sont_rognees_dun_meplat() {
        let rayon = 4.2_f32;
        let peau = bouclier::demi_epaisseur(rayon);
        let c = Composant::BouclierGrand { profil: Profil::P1, rayon, elancement: 1.3 };
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        // Sommets de nappe uniquement : la jante et les nervures sont des
        // cylindres, dont les couronnes dépasseraient du contour et élargiraient
        // artificiellement le méplat.
        let nappe: Vec<Vec3> = b
            .terminer()
            .iter()
            .flat_map(|l| l.vertices.iter())
            .map(|v| vec3(v.position[0], v.position[1], v.position[2]))
            .filter(|v| (v.z.abs() - peau).abs() < 1e-4)
            .collect();
        let (haut, large) = nappe.iter().fold((0.0f32, 0.0f32), |(h, l), v| (h.max(v.y), l.max(v.x)));
        // Largeur du bord droit tout en haut, mesurée sur les sommets qui y sont.
        let meplat = 2.0
            * nappe
                .iter()
                .filter(|v| (v.y - haut).abs() < 1e-3)
                .fold(0.0f32, |m, v| m.max(v.x.abs()));
        let part = meplat / (2.0 * large);
        assert!(
            part > 0.05,
            "méplat de {meplat:.2} pour une largeur de {:.2} ({part:.3}) : la pointe est restée franche",
            2.0 * large
        );
        assert!(
            part < 0.30,
            "méplat de {meplat:.2} pour une largeur de {:.2} ({part:.3}) : ce n'est plus une pointe rognée mais un bout coupé",
            2.0 * large
        );
    }

    // Le grief à l'écran : la plaque « lisait comme une pierre taillée ». La
    // cause était le facettage — six triangles de valeurs différentes rayonnant
    // du moyeu, ce qui *est* le dessin d'une gemme. Un miroir est **uniforme**,
    // et ce sont les nervures posées dessus qui le structurent. On vérifie donc
    // que chaque face n'a qu'un seul ton, ce qu'aucun test de forme ne dirait.
    #[test]
    fn les_faces_dun_grand_bouclier_sont_des_miroirs_unis() {
        let rayon = 4.2_f32;
        let peau = bouclier::demi_epaisseur(rayon);
        let c = Composant::BouclierGrand { profil: Profil::P1, rayon, elancement: 1.3 };
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        let mut tons: Vec<[u8; 4]> = Vec::new();
        for lot in b.terminer() {
            for t in lot.indices.chunks_exact(3) {
                let s: Vec<&macroquad::models::Vertex> =
                    t.iter().map(|i| &lot.vertices[*i as usize]).collect();
                // Triangle de nappe : ses trois sommets à la cote d'une peau.
                if !s.iter().all(|v| (v.position[2].abs() - peau).abs() < 1e-4) {
                    continue;
                }
                let couleur = s[0].color;
                if !tons.contains(&couleur) {
                    tons.push(couleur);
                }
            }
        }
        // Deux tons : un par face, et rien de plus. La différence avant/arrière
        // est voulue — sans elle on ne sait plus quelle face on regarde.
        assert_eq!(
            tons.len(),
            2,
            "{} tons sur les nappes : la plaque est facettée comme une gemme",
            tons.len()
        );
    }

    // Le module tourne : sa géométrie doit tenir dans sa longueur annoncée,
    // sinon le rayon de rotation calculé depuis la traverse serait faux — et
    // rester **d'un seul côté** du montage, celui de l'extérieur.
    #[test]
    fn module_equipage_se_deploie_vers_lexterieur() {
        for longueur in [4.0_f32, 7.0, 12.0] {
            let c = Composant::ModuleEquipage { profil: Profil::P1, longueur, hublots: 8 };
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            for lot in b.terminer() {
                for v in &lot.vertices {
                    let z = v.position[2];
                    assert!(
                        (-1e-3..=longueur + 1e-3).contains(&z),
                        "longueur {longueur} : géométrie à z={z}, hors de [0, {longueur}]"
                    );
                }
            }
        }
    }

    #[test]
    fn module_habitat_deux_ports_axiaux() {
        let c = Composant::ModuleHabitat { profil: Profil::P2, longueur: 12.0, spin: 0.0, attache: 0.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 2);
        assert!(ports.iter().all(|p| p.genre == GenrePort::ModuleAxial && p.profil == Profil::P2));
        assert_eq!(c.cout(), 16.0);
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    // Le module doit tenir **exactement** dans la longueur annoncée : c'est sur
    // cette cote que reposent tous les placements (grappe, enfilade sur
    // l'épine). Les armatures étant posées aux quarts, rien ne doit dépasser
    // des bouts.
    #[test]
    fn module_habitat_tient_dans_sa_longueur() {
        for (longueur, profil) in [(12.0_f32, Profil::P2), (6.0, Profil::P1), (3.0, Profil::P0)] {
            let c = Composant::ModuleHabitat { profil, longueur, spin: 0.4, attache: 1.0 };
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            let demi = longueur * 0.5;
            for lot in b.terminer() {
                for v in &lot.vertices {
                    assert!(
                        v.position[2].abs() <= demi + 1e-3,
                        "longueur {longueur} : géométrie à z={} hors de ±{demi}",
                        v.position[2]
                    );
                }
            }
        }
    }

    // **L'armature ne doit jamais plonger dans la coque** — le défaut trouvé
    // deux fois à l'écran (d'abord en triangle, puis en hexagone posé sur les
    // tangences). On échantillonne le contour hexagonal et on vérifie que
    // chaque point est hors de la section onigiri.
    //
    // Test d'appartenance : la section est la somme de Minkowski d'un triangle
    // nu (circonrayon `dv`) et d'un disque de rayon ρ ; un point est donc
    // dedans **ssi** sa distance au triangle nu vaut au plus ρ.
    #[test]
    fn armature_hexagonale_reste_hors_de_la_coque() {
        let f = super::super::pieces::ONIGIRI_FILET;
        let mini = super::super::pieces::onigiri_hex_echelle_mini();

        // Distance d'un point à un segment, dans le plan.
        let dist_seg = |q: Vec2, a: Vec2, b: Vec2| -> f32 {
            let ab = b - a;
            let t = ((q - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
            q.distance(a + ab * t)
        };

        for r in [1.0_f32, 2.0, 3.0] {
            let (rho, dv) = (r * f, r * (1.0 - f));
            for spin in [0.0_f32, 0.7, 2.3] {
                // Sommets du triangle NU de la coque.
                let tri: Vec<Vec2> = (0..3)
                    .map(|k| {
                        let a = spin + TAU * k as f32 / 3.0;
                        Vec2::new(dv * a.cos(), dv * a.sin())
                    })
                    .collect();
                let dans_coque = |q: Vec2| -> bool {
                    let d = (0..3)
                        .map(|k| dist_seg(q, tri[k], tri[(k + 1) % 3]))
                        .fold(f32::INFINITY, f32::min);
                    // Intérieur du triangle : tous les côtés « à gauche ».
                    let dedans = (0..3).all(|k| {
                        let (a, b) = (tri[k], tri[(k + 1) % 3]);
                        (b - a).perp_dot(q - a) >= 0.0
                    });
                    dedans || d <= rho
                };

                // À l'échelle retenue par le composant, rien ne doit être dedans.
                let ra = r * mini * 1.04;
                let h = super::super::pieces::onigiri_hexagone(ra, spin, 0.0);
                for k in 0..6 {
                    let (a, b) = (h[k], h[(k + 1) % 6]);
                    for i in 0..=20 {
                        let q3 = a.lerp(b, i as f32 / 20.0);
                        let q = Vec2::new(q3.x, q3.y);
                        assert!(
                            !dans_coque(q),
                            "r={r} spin={spin} : armature dans la coque en {q:?}"
                        );
                    }
                }
            }
        }
    }

    // La ferrure doit sortir d'**un seul** côté, celui que désigne `spin` : c'est
    // ce qui permet d'orienter chaque module vers l'épine dans la grappe. On
    // vérifie que la géométrie déborde du côté visé et **pas** du côté opposé.
    #[test]
    fn module_habitat_ferrure_dun_seul_cote() {
        let r = Profil::P2.rayon();
        for spin in [0.0_f32, 1.0, 2.5] {
            // Portée volontairement généreuse : l'armature hexagonale ceinture
            // le module de **tous** les côtés, donc si la ferrure ne portait
            // qu'un peu plus loin qu'elle, le test ne saurait plus les
            // distinguer et passerait même avec une ferrure du mauvais côté.
            let c = Composant::ModuleHabitat { profil: Profil::P2, longueur: 12.0, spin, attache: 4.0 };
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            // Direction de la ferrure (normale du côté plat porteur).
            let u = vec3((spin + PI).cos(), (spin + PI).sin(), 0.0);
            let mut vers_ferrure = f32::MIN;
            let mut vers_oppose = f32::MIN;
            for lot in b.terminer() {
                for v in &lot.vertices {
                    let q = vec3(v.position[0], v.position[1], 0.0);
                    vers_ferrure = vers_ferrure.max(q.dot(u));
                    vers_oppose = vers_oppose.max(q.dot(-u));
                }
            }
            // Côté ferrure : les longerons portent à **mi-`attache`**, soit 2,0
            // au-delà du côté plat. Côté opposé : rien de plus que l'armature
            // hexagonale et son gabarit de barre (~1,15 r) — surtout pas une
            // ferrure, qui porterait à ~1,6 r.
            assert!(
                vers_ferrure > super::super::pieces::onigiri_inscrit(r) + 1.8,
                "spin {spin} : ferrure trop courte ({vers_ferrure:.2})"
            );
            assert!(
                vers_oppose <= r * 1.20,
                "spin {spin} : ferrure du mauvais côté ({vers_oppose:.2})"
            );
        }
    }

    // --- Fret d'échelle vaisseau (ISV) : nacelle onigiri + râtelier.

    #[test]
    fn nacelle_cargo_un_port_surface() {
        let c = Composant::NacelleCargo { profil: Profil::P1, longueur: 15.0, spin: 0.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::Surface);
        // Montée par sa base : l'avant regarde l'hôte (−Z), le fret part vers +Z.
        assert!((ports[0].repere.avant() - Vec3::NEG_Z).length() < 1e-5);
        assert_eq!(c.cout(), 6.0);
        assert_eq!(c.rayon_local(), 15.0); // longueur > rayon de section
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    #[test]
    fn ratelier_cargo_deux_ports_axiaux_chainables() {
        let c = Composant::RatelierCargo { profil: Profil::P2, longueur: 16.0, rayon: 4.5, nacelles: 6, nacelle: 0.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 2);
        assert!(ports.iter().all(|p| p.genre == GenrePort::ModuleAxial && p.profil == Profil::P2));
        // Bouts opposés, à ±demi-longueur : deux râteliers se chaînent bout à bout.
        assert!(ports.iter().any(|p| (p.repere.pos.z - 8.0).abs() < 1e-4));
        assert!(ports.iter().any(|p| (p.repere.pos.z + 8.0).abs() < 1e-4));
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    // La triforce n'est pas « 3 nacelles en rond » : c'est trois conteneurs de
    // MÊME orientation posés pointe contre pointe.
    #[test]
    fn ratelier_trois_nacelles_forme_une_triforce() {
        let (_, places) = grappe_cargo(2.6, 3, 0.0);
        assert_eq!(places.len(), 3);
        let spin0 = places[0].1;
        assert!(places.iter().all(|(_, s)| (s - spin0).abs() < 1e-5), "orientation commune");
        // Postes aux trois sommets d'un triangle équilatéral (même distance au
        // centre, 120° d'écart) — pas une file ni un tas.
        let d0 = places[0].0.length();
        assert!(places.iter().all(|(q, _)| (q.length() - d0).abs() < 1e-4));
    }

    // **Non-recouvrement** de la triforce — le bug trouvé à l'écran : des
    // triangles à coins congés sont des triangles nus gonflés du rayon de
    // congé, donc les poser à la tangence des coins VIFS les fait se traverser.
    // On vérifie ici la condition exacte : les triangles nus de deux nacelles
    // voisines doivent être écartés d'au moins 2ρ.
    #[test]
    fn ratelier_triforce_ne_se_traverse_pas() {
        for rayon in [1.5_f32, 2.6, 6.0] {
            let (rnac, places) = grappe_cargo(rayon, 3, 0.0);
            let f = super::super::pieces::ONIGIRI_FILET;
            let (rho, r_nu) = (rnac * f, rnac * (1.0 - f));
            // Pointe (du triangle NU) de chaque nacelle dirigée vers sa voisine.
            let pointe = |k: usize, vers: f32| {
                let (poste, spin) = places[k];
                let _ = spin;
                poste + vec3(vers.cos(), vers.sin(), 0.0) * r_nu
            };
            let a120 = TAU / 3.0;
            let v0 = pointe(0, FRAC_PI_2 + a120); // nacelle 0 → voisine 1
            let v1 = pointe(1, FRAC_PI_2); // nacelle 1 → voisine 0
            let ecart = v0.distance(v1);
            assert!(
                ecart >= 2.0 * rho,
                "rayon {rayon} : nacelles qui se traversent ({ecart:.4} < {:.4})",
                2.0 * rho
            );
            // ...mais pas au point de perdre le motif : la triforce reste serrée.
            assert!(ecart < 3.0 * rho, "rayon {rayon} : triforce trop lâche ({ecart:.4})");
        }
    }

    // En couronne, les nacelles doivent se frôler sans jamais se croiser, quel
    // que soit leur nombre — c'est ce que garantit le calcul par demi-pas.
    #[test]
    fn ratelier_couronne_ne_croise_jamais() {
        for n in [4usize, 5, 6, 8, 12] {
            let (rnac, places) = grappe_cargo(4.5, n, 0.0);
            let d = places[0].0.distance(places[1].0);
            assert!(d > 2.0 * rnac, "n={n} : nacelles qui se croisent ({d} <= {})", 2.0 * rnac);
        }
    }

    // Le coût et l'encombrement doivent suivre le nombre de nacelles, sinon le
    // budget du générateur ne verrait pas la différence entre une rangée pleine
    // et une rangée clairsemée.
    #[test]
    fn ratelier_cargo_cout_croit_avec_les_nacelles() {
        let rat = |n: usize| Composant::RatelierCargo { profil: Profil::P2, longueur: 16.0, rayon: 4.5, nacelles: n, nacelle: 0.0 };
        assert!(rat(8).cout() > rat(6).cout());
        // Plus de nacelles à rayon constant = nacelles plus fines : l'encombrement
        // ne doit PAS croître (c'est le pas angulaire qui les amincit).
        assert!(rat(8).rayon_local() < rat(6).rayon_local());
    }

    #[test]
    fn charpente_deux_ports_axiaux_a_leurs_profils() {
        let c = Composant::Charpente { grand: Profil::P3, petit: Profil::P0, longueur: 40.0, courbure: 2.6, aiguille: false };
        let ports = c.ports();
        assert_eq!(ports.len(), 2);
        assert!(ports.iter().all(|p| p.genre == GenrePort::ModuleAxial));
        assert!(ports.iter().any(|p| p.profil == Profil::P3));
        assert!(ports.iter().any(|p| p.profil == Profil::P0));
        assert_eq!(c.cout(), 43.0); // 3.0 + longueur
        assert_eq!(c.rayon_local(), 20.0); // (40*0.5).max(P3.rayon()*0.5*1.5) = 20.0
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    /// Sommets d'une aile, avec leur teinte, rangés par distance à la racine.
    fn aile_chauffee(chaleur: f32) -> Vec<(f32, [u8; 4])> {
        let c = Composant::RadiateurMega {
            profil: Profil::P0,
            longueur: 10.0,
            largeur: 5.5,
            ailettes: 34,
            chaleur,
        };
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        let mut v: Vec<(f32, [u8; 4])> = Vec::new();
        for lot in b.terminer() {
            for s in &lot.vertices {
                v.push((s.position[2], s.color));
            }
        }
        v
    }

    /// Une teinte est-elle **chaude** — franchement plus rouge que verte ? Le
    /// gris de base a ses trois canaux à quelques pourcents l'un de l'autre ;
    /// tout ce qui chauffe s'en écarte massivement.
    fn est_chaude(c: [u8; 4]) -> bool {
        c[0] as i32 - c[1] as i32 > 40
    }

    // Le radiateur chauffe **seulement sur ses parties grises** : panneau, tubes
    // calorifiques et rails. La colonne vertébrale et le réservoir sont noirs et
    // doivent le rester — ce sont des organes internes, pas de la surface
    // radiante, et les voir rougir ferait mentir la pièce.
    #[test]
    fn seules_les_parties_grises_du_radiateur_chauffent() {
        let froid = aile_chauffee(0.0);
        let chaud = aile_chauffee(1.0);
        assert_eq!(froid.len(), chaud.len(), "la chauffe ne doit rien changer à la géométrie");

        // À froid, rien n'est chaud. C'est la moitié du test qui compte : sans
        // elle, une pièce rouge en permanence passerait.
        assert!(
            !froid.iter().any(|(_, c)| est_chaude(*c)),
            "des teintes chaudes à chaleur nulle"
        );

        // Les **noirs** sont repérés à froid puis suivis un par un : la chauffe
        // ne déplace aucun sommet, donc le sommet `i` est le même dans les deux
        // versions. Compter une proportion ne dirait rien — les tubes pèsent à
        // eux seuls 98 % des sommets, et la colonne noire disparaîtrait dans
        // l'arrondi.
        let noir = |c: [u8; 4]| c[0] < 60 && c[1] < 60 && c[2] < 60;
        let (mut noirs, mut chauffes) = (0, 0);
        for (i, (_, froide)) in froid.iter().enumerate() {
            let chaude = chaud[i].1;
            if noir(*froide) {
                noirs += 1;
                assert!(
                    noir(chaude),
                    "sommet {i} : la colonne vertébrale a rougi ({chaude:?}) —                      ce n'est pas de la surface radiante"
                );
            } else if est_chaude(chaude) {
                chauffes += 1;
            }
        }
        assert!(noirs > 20, "{noirs} sommets noirs repérés : le test ne surveille rien");
        let gris = froid.len() - noirs;
        assert!(
            chauffes > gris * 9 / 10,
            "{chauffes} sommets chauds sur {gris} gris : la chauffe n'atteint pas tout le panneau"
        );
    }

    // Un radiateur se **refroidit sur sa longueur** — c'est sa fonction même. La
    // racine doit donc être plus chaude que la pointe, sans quoi l'aile lit
    // comme une plaque peinte plutôt que comme un organe qui évacue.
    #[test]
    fn le_radiateur_est_plus_chaud_a_sa_racine_qu_a_sa_pointe() {
        let mut v = aile_chauffee(1.0);
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (z0, z1) = (v[0].0, v[v.len() - 1].0);
        let rouge = |bande: (f32, f32)| -> f32 {
            let pris: Vec<f32> = v
                .iter()
                .filter(|(z, c)| (bande.0..=bande.1).contains(z) && est_chaude(*c))
                .map(|(_, c)| c[0] as f32 - c[2] as f32)
                .collect();
            assert!(!pris.is_empty(), "aucune teinte chaude dans la bande {bande:?}");
            pris.iter().sum::<f32>() / pris.len() as f32
        };
        let tiers = (z1 - z0) / 3.0;
        let racine = rouge((z0, z0 + tiers));
        let pointe = rouge((z1 - tiers, z1));
        assert!(
            racine > pointe * 1.15,
            "racine {racine:.0} contre pointe {pointe:.0} : l'aile chauffe uniformément, \
             elle ne lit pas comme un radiateur"
        );
    }

    #[test]
    fn radiateur_mega_un_port_surface() {
        let c = Composant::RadiateurMega { profil: Profil::P0, longueur: 10.0, largeur: 5.5, ailettes: 34, chaleur: 0.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::Surface);
        assert_eq!(c.cout(), 26.0); // 16.0 + longueur
        assert_eq!(c.rayon_local(), 10.0); // longueur.max(largeur)
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    #[test]
    fn motrice_un_port_axial() {
        let c = Composant::Motrice { profil: Profil::P2, echelle: 1.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::ModuleAxial);
        assert_eq!(c.cout(), 40.0);
        assert_eq!(c.rayon_local(), 12.0); // 12.0 * echelle
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    #[test]
    fn bloc_moteur_deux_ports_axiaux_memes_profils() {
        let c = Composant::BlocMoteur { profil: Profil::P2, largeur: 4.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 2);
        assert!(ports.iter().all(|p| p.genre == GenrePort::ModuleAxial && p.profil == Profil::P2));
        assert_eq!(c.cout(), 20.0); // 5.0 * largeur
        assert!((c.rayon_local() - 5.2).abs() < 1e-4); // 1.3 * largeur
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    #[test]
    fn reservoir_deux_ports_axiaux() {
        let c = Composant::Reservoir { profil: Profil::P1, longueur: 6.0, cage: true };
        let ports = c.ports();
        assert_eq!(ports.len(), 2);
        assert!(ports.iter().all(|p| p.genre == GenrePort::ModuleAxial && p.profil == Profil::P1));
        assert_eq!(c.cout(), 14.0); // 8.0 + longueur
        assert_eq!(c.rayon_local(), 4.0); // (6*0.5 + 1.0).max(1.0*3.5) = 4.0
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    #[test]
    fn moteur_antimatiere_un_port_axial() {
        let c = Composant::MoteurAntimatiere { profil: Profil::P1, taille: 6.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].genre, GenrePort::ModuleAxial);
        assert_eq!(c.cout(), 11.0);
        assert!((c.rayon_local() - 9.72).abs() < 1e-4); // taille * 1.62
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    #[test]
    fn coiffe_un_port_axial_pour_chaque_forme() {
        for v in [VarianteCoiffe::Bombee, VarianteCoiffe::Hexagonale, VarianteCoiffe::Amarrage] {
            let c = Composant::Coiffe { profil: Profil::P1, variante: v };
            let ports = c.ports();
            assert_eq!(ports.len(), 1, "{:?}", v);
            assert_eq!(ports[0].genre, GenrePort::ModuleAxial);
            assert_eq!(c.cout(), 6.0, "{:?}", v);
            assert!((c.rayon_local() - 1.4).abs() < 1e-4, "{:?}", v); // P1.rayon() * 1.4
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            assert!(!b.terminer().is_empty(), "{:?}", v);
        }
    }

    #[test]
    fn reacteur_antimatiere_deux_ports_axiaux() {
        let c = Composant::ReacteurAntimatiere { profil: Profil::P1, taille: 6.0 };
        let ports = c.ports();
        assert_eq!(ports.len(), 2);
        assert!(ports.iter().all(|p| p.genre == GenrePort::ModuleAxial && p.profil == Profil::P1));
        assert_eq!(c.cout(), 14.0);
        assert!((c.rayon_local() - 7.2).abs() < 1e-4); // taille * 1.2
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    #[test]
    fn treillis_hexagone_pose_seul_sans_port() {
        // Anneau décoratif posé à la main (via un Repere cuit) : aucun port —
        // à distinguer du même hexagone en pied de Charpente (aiguille: true).
        let c = Composant::TreillisHexagone { profil: Profil::P1, liaison: 0.0 };
        assert!(c.ports().is_empty());
        assert_eq!(c.cout(), 12.0);
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }

    // Les montants de `liaison` lèvent un prisme **le long de la normale** de
    // l'anneau (+Y local), pas dans son plan.
    //
    // Mesuré sur les fils, jamais sur la formule : l'assertion précédente à cet
    // endroit recopiait `(rayon * 1.1).max(liaison)` — elle aurait suivi
    // n'importe quel changement de la formule sans rien garder. Les montants
    // partaient en fait le long de **+Z**, donc à plat dans le plan de
    // l'hexagone, en peigne au lieu d'un prisme ; personne ne l'avait vu parce
    // qu'aucun appelant du jeu n'a jamais passé `liaison > 0`.
    // L'enveloppe doit **contenir** le prisme, sans la marge de tolérance de
    // `les_rayons_declares_contiennent_la_piece`.
    //
    // Ce test existe parce que ce balayage-là ne mord pas ici : sa marge
    // (`MARGE_RAYON = 1,40`, la dette de L1.4) absorbe largement le
    // sous-dimensionnement de l'ancienne sphère centrée (×1,08 sur le prisme,
    // ×1,22 sur l'anneau nu). Une dette qui masque un vrai défaut est une
    // raison d'écrire le test serré, pas de s'en remettre au balayage.
    #[test]
    fn lenveloppe_de_lhexagone_contient_ses_montants() {
        for liaison in [0.0_f32, 3.0, 9.0] {
            let c = Composant::TreillisHexagone { profil: Profil::P3, liaison };
            let env = c.enveloppe_locale();
            for f in crate::vaisseau::fils(&c) {
                for p in [f.a, f.b] {
                    assert!(
                        env.contient(p),
                        "liaison {liaison} : {p:?} hors de l'enveloppe (profondeur {:.3})",
                        env.profondeur(p)
                    );
                }
            }
        }
    }

    // Les trois bandes du tore couvrent la section **entière**, sans trou ni
    // recouvrement : tuiles 180°, deux épaulements, fenêtre 80°. Mesuré sur les
    // bornes elles-mêmes plutôt que sur des degrés recopiés.
    #[test]
    fn les_bandes_du_tore_pavent_toute_la_section() {
        use std::f32::consts::{PI, TAU};
        let tuiles = 2.0 * tore::V_TUILES;
        let fenetre = 2.0 * tore::V_FENETRE;
        let epaulement = (PI - tore::V_FENETRE) - tore::V_TUILES;
        assert!(epaulement > 0.0, "les tuiles mordent sur la fenêtre");
        assert!(
            (tuiles + fenetre + 2.0 * epaulement - TAU).abs() < 1e-5,
            "somme des bandes = {} au lieu de {TAU}",
            tuiles + fenetre + 2.0 * epaulement
        );
        // Et les valeurs demandées : 180° de tuiles, 80° de fenêtre.
        assert!((tuiles.to_degrees() - 180.0).abs() < 1e-3);
        assert!((fenetre.to_degrees() - 80.0).abs() < 1e-3);
    }

    // Le bardage suit la **taille** de tuile, pas le rayon : doubler l'anneau
    // doit doubler le nombre de tuiles, pas les agrandir. C'est ce qui permet
    // au même tore de servir à toutes les échelles.
    //
    // Mesuré sur la grille elle-même, et non sur le nombre de sommets du tore :
    // cette version-là marchait tant que les tuiles pesaient l'essentiel du
    // maillage, et a cessé de mordre dès qu'on les a agrandies (les bandes
    // lisses et la menuiserie, à compte fixe, diluaient le rapport à 1,58).
    // Un proxy qui ne tient que dans un régime n'est pas une mesure.
    #[test]
    fn le_bardage_garde_la_taille_de_tuile_quand_lanneau_grandit() {
        let (r1, c1) = tore::grille_tuiles(12.0, 1.5);
        let (r2, c2) = tore::grille_tuiles(24.0, 1.5);
        assert_eq!(r1, r2, "la section n'a pas changé : même nombre de rangs");
        assert!(
            (c2 as f32 / c1 as f32 - 2.0).abs() < 0.05,
            "{c1} → {c2} colonnes pour un rayon doublé"
        );
        // Et la tuile elle-même garde sa cote, quel que soit l'anneau.
        assert!(tore::cote_tuile() > 0.0);
    }

    // --- Panneaux mégastructure : le suivi solaire ---

    /// Fils d'un panneau, à un couple d'angles donné.
    fn panneau_mega(azimut: f32, inclinaison: f32) -> Vec<crate::vaisseau::Fil> {
        crate::vaisseau::fils(&Composant::PanneauMega {
            profil: Profil::P0,
            variante: VariantePanneauMega::FermeModulaire,
            longueur: 16.0,
            largeur: 6.0,
            azimut,
            inclinaison,
        })
    }

    // La contrainte de l'étape : l'aile s'oriente. Les deux joints doivent
    // **chacun** bouger la géométrie — un axe câblé mais inerte donnerait un
    // panneau qui promet de suivre le soleil sans le faire.
    #[test]
    fn les_deux_joints_du_panneau_mega_orientent_laile() {
        let repos = panneau_mega(0.0, 0.0);
        let bouge = |a: f32, i: f32| {
            let autre = panneau_mega(a, i);
            repos
                .iter()
                .zip(&autre)
                .map(|(x, y)| x.a.distance(y.a).max(x.b.distance(y.b)))
                .fold(0.0_f32, f32::max)
        };
        assert!(bouge(0.7, 0.0) > 1.0, "l'azimut ne fait rien bouger");
        assert!(bouge(0.0, 0.5) > 1.0, "l'inclinaison ne fait rien bouger");
    }

    // ...mais le **berceau** ne bouge pas : il reste boulonné à la structure.
    // C'est ce qui distingue un panneau qui suit le soleil d'un panneau monté
    // de travers, et c'est pourquoi les angles sont dans le composant et non
    // dans la `Repere` de pose.
    #[test]
    fn le_berceau_du_panneau_mega_ne_suit_pas_laile() {
        let repos = panneau_mega(0.0, 0.0);
        let tourne = panneau_mega(1.2, 0.6);
        // Le bras d'attache est le seul fil à passer par le port de montage
        // (z < 0) : il ne doit pas broncher.
        let socle = |f: &[crate::vaisseau::Fil]| -> Vec<Vec3> {
            f.iter().flat_map(|x| [x.a, x.b]).filter(|p| p.z < -1e-3).collect()
        };
        let (a, b) = (socle(&repos), socle(&tourne));
        assert!(!a.is_empty(), "aucun fil côté hôte : le scénario ne prouve rien");
        assert_eq!(a.len(), b.len());
        for (p, q) in a.iter().zip(&b) {
            assert!(p.distance(*q) < 1e-4, "le berceau a tourné avec l'aile : {p:?} → {q:?}");
        }
    }

    // Le rayon déclaré doit couvrir **toutes** les orientations : une aile qui
    // pivote balaie un disque, et un rayon calé sur sa pose au repos ferait
    // rentrer la caméra dedans dès le premier quart de tour.
    #[test]
    fn le_rayon_du_panneau_mega_couvre_toutes_ses_orientations() {
        for variante in VariantePanneauMega::TOUTES {
            let c = |a: f32, i: f32| Composant::PanneauMega {
                profil: Profil::P0,
                variante,
                longueur: 16.0,
                largeur: 6.0,
                azimut: a,
                inclinaison: i,
            };
            let r = c(0.0, 0.0).rayon_local();
            for k in 0..12 {
                let a = std::f32::consts::TAU * k as f32 / 12.0;
                for i in [-0.7_f32, 0.0, 0.7] {
                    let loin = crate::vaisseau::fils(&c(a, i))
                        .iter()
                        .flat_map(|f| [(f.a, f.rayon_a), (f.b, f.rayon_b)])
                        .fold(0.0_f32, |m, (p, rr)| m.max(p.length() + rr));
                    assert!(loin <= r, "{variante:?} : déborde à {loin:.2} pour un rayon {r:.2}");
                }
            }
        }
    }

    #[test]
    fn les_montants_de_lhexagone_levent_un_prisme_sur_sa_normale() {
        let etendue_hexa = |liaison: f32| {
            let c = Composant::TreillisHexagone { profil: Profil::P3, liaison };
            let (mut y, mut z) = (0.0f32, 0.0f32);
            for f in crate::vaisseau::fils(&c) {
                for p in [f.a, f.b] {
                    y = y.max(p.y);
                    z = z.max(p.z);
                }
            }
            (y, z)
        };
        let (y0, z0) = etendue_hexa(0.0);
        let (y1, z1) = etendue_hexa(6.0);
        // Sans montants l'anneau est plat : il ne monte que de sa demi-épaisseur.
        assert!(y0 < 2.0, "anneau nu trop épais sur sa normale : {y0:.2}");
        // Avec, il monte **exactement** de `liaison` — les montants partent du
        // plan médian des sommets, pas de la face supérieure.
        assert!((y1 - 6.0).abs() < 1e-3, "le prisme doit monter de `liaison` : {y1:.2}");
        assert!((z1 - z0).abs() < 1e-3, "et ne rien ajouter dans le plan : {z0:.2} → {z1:.2}");
    }

    // ================================================================
    // Balayage de **toutes** les variantes
    // (assembleur, Lot 1 — `docs/conception/assembleur.md` §5.5)
    //
    // `suivante`/`sous_ensemble_echantillon`/`echantillons` ont déménagé hors
    // de ce module de tests en L2.4 : la chaîne sert désormais aussi à la
    // palette (`posables`, en dehors des tests) — une seule source, comme
    // partout ailleurs dans ce lot.
    // ================================================================

    /// Nom de variante, pour des messages d'échec lisibles. Dérivé du `Debug`
    /// du composant — il n'y a donc pas une seconde liste de noms à tenir à
    /// jour à côté de l'enum.
    fn nom(c: &Composant) -> String {
        format!("{c:?}").split([' ', '{', '(']).next().unwrap_or("?").to_string()
    }

    /// Fiche Markdown d'un composant : ses fils, numérotés comme à l'écran.
    fn fiche(c: &Composant) -> String {
        use crate::vaisseau::{fils, GenreFil};
        let mut t = format!("
## {}

", nom(c));
        t.push_str(&format!("`{c:?}`

"));
        let f: Vec<_> = fils(c).into_iter().filter(|x| x.genre != GenreFil::Maille).collect();
        let env = c.enveloppe_locale();
        let forme = match env.noyau {
            _ if env.est_sphere() => "sphere".to_string(),
            crate::vaisseau::Noyau::Segment { a, b } => format!("capsule long {:.2}", a.distance(b)),
            crate::vaisseau::Noyau::Rectangle { hu, hv, .. } => format!("boudin {:.2}x{:.2}", hu, hv),
        };
        t.push_str(&format!("- **{} fils** numérotables
", f.len()));
        t.push_str(&format!("- cout {:.0} · rayon_local {:.2}
", c.cout(), c.rayon_local()));
        t.push_str(&format!("- enveloppe : {forme} rayon {:.2}
", env.rayon));
        // Relevé automatique (`vaisseau::mesure`) : profil tranché, serrage de
        // l'enveloppe, élancement. Aucun jugement humain là-dedans — c'est ce
        // qui permet qu'un composant *futur* soit mesuré sans qu'on y pense.
        let m = crate::vaisseau::mesurer(c, 40);
        if m.rayon_max > 1e-6 {
            t.push_str(&format!(
                "- **mesure** : long {:.2} · rayon max {:.2} · elancement x{:.1}
",
                m.longueur(),
                m.rayon_max,
                m.elancement()
            ));
            t.push_str(&format!(
                "- **serrage {:.2}** ({}) — besoin {:.2} pour {:.2} declare
",
                m.serrage(),
                m.verdict(),
                m.besoin,
                m.declare
            ));
            t.push_str(&format!("- profil : `{}`
", crate::vaisseau::silhouette(&m)));
        }
        if crate::vaisseau::a_des_mailles(c) {
            t.push_str(
                "- ⚠️ contient des **mailles brutes** : le profil les ignore, il est donc partiel
",
            );
        }
        t.push_str("
");
        if f.is_empty() {
            t.push_str("*(aucun fil : la pièce est faite de mailles brutes, ou ne dessine rien)*
");
            return t;
        }
        t.push_str("| n° | genre | de | à | long. | rayon a→b |
|---:|---|---|---|---:|---:|
");
        let p = |v: macroquad::prelude::Vec3| format!("{:.2},{:.2},{:.2}", v.x, v.y, v.z);
        for x in &f {
            t.push_str(&format!(
                "| {} | {} | {} | {} | {:.2} | {} |
",
                x.numero,
                x.genre.nom(),
                p(x.a),
                p(x.b),
                x.longueur(),
                if (x.rayon_a - x.rayon_b).abs() < 1e-3 {
                    format!("{:.2}", x.rayon_a)
                } else {
                    format!("{:.2}→{:.2}", x.rayon_a, x.rayon_b)
                }
            ));
        }
        t
    }

    /// Le catalogue complet, tel qu'il doit se trouver sur disque.
    fn catalogue() -> String {
        let mut t = String::new();
        t.push_str("# Référence — Fils des composants

");
        t.push_str("> **Fichier généré.** Ne pas éditer à la main : il est reconstruit et
");
        t.push_str("> comparé par `le_catalogue_des_fils_est_a_jour`
");
        t.push_str("> (`src/vaisseau/composant/mod.rs`). Pour le régénérer après avoir
");
        t.push_str("> modifié une géométrie :
>
");
        t.push_str("> ```
> FILS=1 cargo test --release le_catalogue_des_fils
> ```

");
        t.push_str("À quoi il sert : désigner une barre par un **numéro** plutôt que par une
");
        t.push_str("périphrase. Les mêmes numéros s'affichent à l'écran avec la touche **F**
");
        t.push_str("(vue station), posés **dans une coupure du fil** qu'ils désignent.

");
        t.push_str("Les cotes sont celles de l'**échantillon de référence** de chaque variante
");
        t.push_str("(la chaîne `suivante`, même source que les tests de balayage) : les positions
");
        t.push_str("bougent avec les paramètres, mais la **numérotation** ne dépend que de
");
        t.push_str("l'ordre des appels dans `dessiner`.

");
        t.push_str("Les **mailles brutes** (coiffes, plaques de bouclier) sont exclues : leur
");
        t.push_str("« fil » serait une diagonale d'englobant qui ne longe aucune arête réelle,
");
        t.push_str("donc un numéro qui désigne quelque chose d'inexistant.

");
        t.push_str("## Lire les mesures

");
        t.push_str("Chaque fiche porte un relevé **automatique** (`vaisseau::mesure`), obtenu en
");
        t.push_str("tranchant les fils — jamais le maillage cuit, qui n'a de sommets qu'aux
");
        t.push_str("frontières de facettes et rendrait des tranches vides à mi-portée
");
        t.push_str("(`suivi/stations.md` §C.13 : l'erreur a été commise trois fois).

");
        t.push_str("- **serrage** = rayon dont l'enveloppe aurait besoin / rayon qu'elle déclare.
");
        t.push_str("  `> 1` elle **ne contient pas** la pièce et la collision ment ; `≈ 1` c'est la
");
        t.push_str("  cible ; `< 1` elle réserve du vide et refusera des poses valides.
");
        t.push_str("- **elancement** = longueur / diamètre. Au-dessus de ~1,5 la sphère devient un
");
        t.push_str("  mauvais englobant et la capsule s'impose (L1.6).
");
        t.push_str("- **profil** : une colonne par tranche, de ` ` (vide) à `@` (rayon maximal).

");
        t.push_str("⚠️ **Le serrage est une borne supérieure, pas une cote.** Un [`Fil`]
");
        t.push_str("porte un rayon **à chaque bout** (le cône est donc exact depuis le 2026-08-01),
");
        t.push_str("mais le besoin vaut `distance a l'axe de l'enveloppe + rayon`, ce qui suppose le
");
        t.push_str("**pire alignement** des que le fil est de biais. Un `DEBORDE` juste au-dessus de 1
");
        t.push_str("est donc a verifier avant d'y toucher — c'est peut-etre la majoration, et la
");
        t.push_str("mesure exacte reste `les_rayons_declares_contiennent_la_piece`, sur sommets cuits.

");
        t.push_str("Le signal fiable, lui : le **classement** (qui déborde le plus), les cas
");
        t.push_str("`lache` (aucune approximation ne rend une enveloppe trop grande), et les
");
        t.push_str("pièces faites de cylindres seuls, où le rayon est exact.
");
        for c in echantillons() {
            t.push_str(&fiche(&c));
        }
        t
    }

    // **Le catalogue des fils ne peut pas se périmer en silence.**
    //
    // Il est régénéré à chaque exécution et comparé à ce qui est sur disque. Un
    // fichier de référence qu'on met à jour à la main dérive dès la première
    // retouche de géométrie — et un numéro qui désigne la mauvaise barre est
    // pire qu'aucun numéro, puisqu'on agit dessus.
    #[test]
    fn le_catalogue_des_fils_est_a_jour() {
        let chemin = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/reference/fils.md");
        let attendu = catalogue();
        if std::env::var("FILS").is_ok() {
            std::fs::create_dir_all(std::path::Path::new(chemin).parent().unwrap()).unwrap();
            std::fs::write(chemin, &attendu).unwrap();
            return;
        }
        let sur_disque = std::fs::read_to_string(chemin).unwrap_or_default();
        // ⚠️ Comparaison **ligne à ligne, lignes vides ignorées**. Ce qui doit
        // rester d'aplomb, c'est la donnée — les en-têtes et les lignes de
        // tableau, donc les numéros et les cotes. Les lignes vides sont de la
        // mise en forme, et elles ne survivent pas identiquement à l'aller-retour
        // disque sous Windows ; comparer les octets bruts ferait rougir le test
        // pour une raison qui n'apprend rien.
        let utiles = |t: &str| -> Vec<String> {
            t.lines().map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect()
        };
        let (a, b) = (utiles(&sur_disque), utiles(&attendu));
        if let Some((n, (x, y))) = a.iter().zip(&b).enumerate().find(|(_, (x, y))| x != y) {
            panic!(
                "docs/reference/fils.md est périmé (ligne {n}) — régénérer avec
                 `FILS=1 cargo test --release le_catalogue_des_fils`
                 disque : {x}
  code : {y}"
            );
        }
        assert_eq!(
            a.len(),
            b.len(),
            "docs/reference/fils.md n'a pas le bon nombre de lignes — régénérer avec              `FILS=1 cargo test --release le_catalogue_des_fils`"
        );
    }

    // La chaîne doit passer par **chaque** variante, une fois et une seule.
    // L'exhaustivité du `match` garantit qu'aucune n'est oubliée ; ce test
    // garantit qu'aucune n'est visitée deux fois — une chaîne mal recousue
    // (deux bras pointant vers le même successeur) sauterait tout un segment
    // sans que rien ne le signale.
    #[test]
    fn la_chaine_dechantillons_visite_chaque_variante_une_fois() {
        let ech = echantillons();
        let mut vus: Vec<std::mem::Discriminant<Composant>> = Vec::new();
        for c in &ech {
            let d = std::mem::discriminant(c);
            assert!(!vus.contains(&d), "variante visitée deux fois : {}", nom(c));
            vus.push(d);
        }
        // ⚠️ **Le seul verrou de valeur assumé de ce lot**, et il a une raison
        // permanente : le compilateur garantit que chaque variante a un *bras*,
        // pas qu'elle est *atteinte*. Un bras qui pointe par-dessus son voisin
        // (`Antenne => Caisson`, sautant `Adaptateur`) compile sans broncher et
        // rend une variante invisible au balayage — les deux tests suivants
        // passeraient au vert en ne la voyant jamais. Rien d'autre que le compte
        // ne le dit.
        //
        // Il rougit donc à l'ajout d'une variante : c'est **voulu**. Le
        // développeur est alors déjà dans ce fichier (le `match` de `suivante`
        // ne compile plus sans lui), et bumper ce nombre est la confirmation
        // qu'il a bien recousu la chaîne au lieu de la court-circuiter.
        assert_eq!(ech.len(), 33, "la chaîne ne visite pas les 33 variantes");
    }

    // ================================================================
    // Palette (L2.4 — `docs/conception/assembleur.md` §6.5)
    // ================================================================

    // Sur le modèle de `familles_de_propulsion_partitionnent_les_variantes` :
    // chaque catégorie déclarée est atteinte par au moins un échantillon.
    // Qu'une variante ne tombe que dans **une** catégorie est garanti par
    // construction (`categorie` est une fonction totale, à un seul bras par
    // appel) ; ce qui reste à vérifier — et que le compilateur ne voit pas —
    // c'est qu'aucune catégorie n'est un compartiment mort.
    #[test]
    fn toute_categorie_de_palette_est_atteinte() {
        let ech = echantillons();
        for cat in Categorie::TOUTES {
            assert!(ech.iter().any(|c| categorie(c) == cat), "catégorie vide : {}", cat.nom());
        }
    }

    // **Le test qui empêche la palette et le moteur de diverger** (§6.5) :
    // deux `match` sur le même enum, écrits à deux endroits — exactement la
    // configuration qui a produit le doublon d'indice de §5.1. Vérifié sur un
    // vrai `Chantier`, pas en re-dérivant le même prédicat dans le test : pour
    // chaque pièce hôte, chaque port qu'elle expose, et chaque cible
    // candidate, le verdict de `posables` doit coïncider avec celui de
    // `Chantier::compatibles`.
    #[test]
    fn palette_et_compatibles_saccordent() {
        let ech = echantillons();
        for hote in &ech {
            let mut ch = Chantier::new();
            assert!(ch.racine(hote.clone()), "{} : la racine ne devrait jamais échouer (pas de budget)", nom(hote));
            for port in ch.libres().to_vec() {
                let attendu = posables(port.genre, port.profil);
                for cible in &ech {
                    // Granularité **variante**, pas montage : `posables` promet
                    // qu'une pièce est posable (avec un indice de montage qui
                    // marche), pas qu'elle liste tous les montages qui
                    // marcheraient — une pièce symétrique (`ModuleAxial`, deux
                    // écoutilles identiques) en a plusieurs, la palette n'en
                    // retient qu'un. C'est aussi ce que §6.5 demande : « toute
                    // variante proposée… y est effectivement posable ».
                    let palette_dit_oui = attendu.iter().any(|(c, _)| c == cible);
                    let modele_dit_oui =
                        (0..cible.ports().len()).any(|m| ch.compatibles(cible, m).contains(&port.id));
                    assert_eq!(
                        palette_dit_oui,
                        modele_dit_oui,
                        "{} (port {:?}/{:?}) vs {} : palette={palette_dit_oui} compatibles={modele_dit_oui}",
                        nom(hote),
                        port.genre,
                        port.profil,
                        nom(cible)
                    );
                }
            }
        }
    }

    // **Santé de la sortie de dessin, pour toutes les variantes.**
    //
    // Trois invariants faibles mais universels, qu'aucun test de famille ne
    // couvrait : un `NaN` glissé dans une cote se propage silencieusement
    // jusqu'à faire disparaître un lot entier à l'écran (macroquad ne dit rien),
    // et un indice hors bornes est un plantage GPU, pas une erreur Rust.
    #[test]
    fn toute_variante_cuit_une_geometrie_saine() {
        for c in echantillons() {
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            for (l, lot) in b.terminer().iter().enumerate() {
                let n = lot.vertices.len();
                for (i, v) in lot.vertices.iter().enumerate() {
                    let p = Vec3::from(v.position);
                    assert!(
                        p.is_finite(),
                        "{} lot {l} sommet {i} : position non finie {p:?}",
                        nom(&c)
                    );
                }
                for &ix in &lot.indices {
                    assert!((ix as usize) < n, "{} lot {l} : indice {ix} hors des {n} sommets", nom(&c));
                }
                assert_eq!(lot.indices.len() % 3, 0, "{} lot {l} : triangles incomplets", nom(&c));
            }
        }
    }

    /// Marge tolérée entre le rayon **déclaré** d'une pièce et son hors-tout
    /// **mesuré**.
    ///
    /// ⚠️ **C'est une dette, pas une cote de conception.** L'invariant juste est
    /// 1,0 : `rayon_local` sert au cadrage caméra (`Station::rayon`) et
    /// `englobant_local` à l'anti-collision de `Chantier::poser` — les deux
    /// doivent *contenir* la pièce, sans quoi la caméra la coupe et la collision
    /// ment. Le relevé du 2026-07-31 (`suivi/assembleur.md` L1.4) montre que
    /// **20 variantes sur 30 débordent**, jusqu'à ×1,37, et pas d'un cheveu :
    /// 81 % des sommets d'une `ChargeUtile` sortent de sa sphère de collision.
    ///
    /// Ces deux fonctions se sont donc écrites comme des **tailles nominales**
    /// (« le gabarit de la pièce ») et non comme des volumes englobants. Les
    /// aligner touche la vingtaine de formules concernées et recule la caméra
    /// partout : c'est un arbitrage rendu à l'utilisateur, pas une retouche.
    ///
    /// En attendant, cette borne fait ce qu'elle peut : elle empêche que ça
    /// **empire**. Une nouvelle pièce qui déclarerait la moitié de sa taille
    /// serait prise ; les débordements d'aujourd'hui, non.
    const MARGE_RAYON: f32 = 1.40;

    // Les deux rayons doivent **contenir** la pièce, à `MARGE_RAYON` près.
    //
    // Deux mesures distinctes pour deux usages distincts, et c'est le point du
    // test : `rayon_local` est pris **depuis l'origine locale** (c'est ainsi que
    // `Station::rayon` le compose : `centre().length() + rayon_local()`), tandis
    // qu'`englobant_local` est pris depuis **son propre centre**, qui peut être
    // décalé — un propulseur ou une coiffe se déploient d'un seul côté de leur
    // montage. Confondre les deux repères rendrait un « débordement » qui
    // n'existe pas, ou masquerait celui qui existe.
    #[test]
    fn les_rayons_declares_contiennent_la_piece() {
        for c in echantillons() {
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            let pts: Vec<Vec3> = b
                .terminer()
                .iter()
                .flat_map(|l| l.vertices.iter().map(|v| Vec3::from(v.position)))
                .collect();
            // Le panache ne dessine rien : les deux rayons valent 0 et le
            // contiennent trivialement (cf. `toute_variante_dessine_sauf_le_panache`).
            if pts.is_empty() {
                continue;
            }

            let rl = c.rayon_local();
            assert!(rl > 0.0, "{} : rayon_local nul alors qu'elle dessine", nom(&c));
            let loin = pts.iter().fold(0.0_f32, |m, p| m.max(p.length()));
            assert!(
                loin <= rl * MARGE_RAYON,
                "{} : cadrage — hors-tout {loin:.2} pour un rayon_local de {rl:.2} (×{:.2})",
                nom(&c),
                loin / rl
            );

            // Côté collision, la mesure est la **distance à l'axe** de la
            // capsule, pas à son centre : c'est tout l'intérêt d'être passé de
            // la sphère à la capsule, et mesurer depuis le centre reviendrait à
            // juger la capsule sur le critère qu'elle remplace (§C.29, piège
            // n° 1 : mesurer un corollaire au lieu de la chose).
            let env = c.enveloppe_locale();
            assert!(env.rayon > 0.0, "{} : enveloppe nulle alors qu'elle dessine", nom(&c));
            let deborde = pts.iter().fold(0.0_f32, |m, p| m.max(env.profondeur(*p)));
            assert!(
                deborde <= env.rayon * (MARGE_RAYON - 1.0),
                "{} : collision — déborde de {deborde:.2} hors d'une enveloppe de rayon {:.2}",
                nom(&c),
                env.rayon
            );
        }
    }

    // **La charpente carrée tient compte de son aiguille.**
    //
    // Sa jumelle hexagonale le fait depuis toujours (`charpente_hexa_pied` :
    // « la tour pend sous la base, donc c'est elle qui fixe l'extension ») ; la
    // carrée avait été oubliée, et déclarait 10,0 en s'étendant à 17,0. Ce
    // n'était pas une marge de détail : 44 % de ses sommets sortaient de la
    // sphère. Le test compare les deux variantes de la **même** pièce, ce qui
    // rend la correction indépendante des cotes choisies.
    #[test]
    fn laiguille_de_la_charpente_compte_dans_son_rayon() {
        let ch = |aiguille| Composant::Charpente {
            grand: Profil::P3,
            petit: Profil::P1,
            longueur: 20.0,
            courbure: 2.0,
            aiguille,
        };
        let (nue, armee) = (ch(false).rayon_local(), ch(true).rayon_local());
        assert!(armee > nue, "l'aiguille doit agrandir le hors-tout ({armee:.2} vs {nue:.2})");
        // Et elle l'agrandit d'au moins l'anneau qu'elle ajoute réellement.
        let (bas, _) = treillis::charpente_pied(Profil::P3, true);
        assert!(
            armee >= nue + bas * 0.9,
            "l'aiguille descend de {bas:.2} mais le rayon ne gagne que {:.2}",
            armee - nue
        );
    }

    // Chaque variante produit de la géométrie — **sauf le panache**, et cette
    // exception est une décision, pas un oubli : un jet de plasma n'a pas de
    // silhouette, il est rendu en additif par `ecran::panache` et le composant
    // ne sert qu'à porter la pose (`suivi/stations.md` §C.28). Lui redonner de
    // la géométrie le ferait dessiner **deux fois**, en volume opaque par-dessus
    // le ruban — c'est exactement l'aspect « tube de plastique » qui avait été
    // rejeté. D'où l'assertion à l'endroit : on **exige** qu'il reste vide.
    #[test]
    fn toute_variante_dessine_sauf_le_panache() {
        for c in echantillons() {
            let mut b = Batisseur::new();
            c.dessiner(&mut b);
            let sommets: usize = b.terminer().iter().map(|l| l.vertices.len()).sum();
            match c {
                Composant::Panache { .. } => {
                    assert_eq!(sommets, 0, "le panache ne doit rien dessiner en géométrie");
                }
                _ => assert!(sommets > 0, "{} ne dessine rien", nom(&c)),
            }
        }
    }
}
