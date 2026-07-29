//! Composant concret : **enum fermé** qui sait exposer ses ports et se dessiner
//! (voir `docs/conception/stations.md`, Partie C §3 et §4 — sous-étape 2a).
//!
//! Choix acté : dispatch par `match` sur un enum, **pas** de trait objet — KISS,
//! zéro allocation, monomorphisé. Une seule fonction
//! par capacité (`ports`, `dessiner`, `cout`, `rayon_local`), qui `match` sur la
//! variante. Les styles/palettes viendront à l'Étape 5. Composants existants :
//! `ModuleAxial` (cylindre) et `Noeud` (hub sphérique 4 ou 6 sorties).

use super::{Piece, Port, Profil};
use macroquad::prelude::*;
use super::peintre::Peintre;

mod commun;
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
mod cargo;
mod reservoir;
mod treillis;
pub use treillis::StyleTreillis;
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
#[derive(Clone, PartialEq, Debug)]
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
    /// **Radiateur de mégastructure** : aile en arête de poisson (boom central +
    /// `ailettes` panneaux plats de chaque côté), à l'échelle du vaisseau/km, pas
    /// des petits radiateurs de station. Se monte par un port `Surface`.
    /// Première brique de la famille « méga » (ISV, puis O'Neill, Elysium).
    RadiateurMega { profil: Profil, longueur: f32, largeur: f32, ailettes: usize },
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
#[derive(Clone, PartialEq, Debug)]
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
            Composant::RadiateurMega { longueur, largeur, ailettes, .. } => radiateur::mega_dessiner(p, *longueur, *largeur, *ailettes),
            Composant::Motrice { echelle, .. } => propulsion::motrice_dessiner(p, *echelle),
            Composant::BlocMoteur { largeur, .. } => propulsion::bloc_dessiner(p, *largeur),
            Composant::Reservoir { longueur, cage, .. } => reservoir::dessiner(p, *longueur, *cage),
            Composant::MoteurAntimatiere { taille, .. } => antimatiere::moteur_dessiner(p, *taille),
            Composant::Coiffe { profil, variante } => adaptateur::coiffe_dessiner(p, *profil, *variante),
            Composant::ReacteurAntimatiere { taille, .. } => antimatiere::reacteur_dessiner(p, *taille),
            Composant::TreillisHexagone { profil, liaison } => treillis::hexagone_dessiner(p, *profil, *liaison),
            Composant::NacelleCargo { profil, longueur, spin } => cargo::nacelle_dessiner(p, *profil, *longueur, *spin),
            Composant::ModuleHabitat { profil, longueur, spin, attache } => habitat::dessiner(p, *profil, *longueur, *spin, *attache),
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
            // Charpente : demi-longueur ou demi-largeur de la base évasée.
            Composant::Charpente { grand, longueur, .. } => {
                (longueur * 0.5).max(grand.rayon() * TREILLIS_SECTION * 1.5)
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
            Composant::TreillisHexagone { profil, liaison } => (profil.rayon() * 1.1).max(*liaison),
            // Module d'habitat : demi-longueur, ou la ferrure si elle porte loin.
            Composant::ModuleHabitat { profil, longueur, attache, .. } => habitat::rayon_local(*profil, *longueur, *attache),
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
    pub fn englobant_local(&self) -> (Vec3, f32) {
        match self {
            Composant::ModuleAxial { .. }
            | Composant::Noeud { .. }
            | Composant::Treillis { .. }
            | Composant::Adaptateur { .. }
            | Composant::Charpente { .. }
            | Composant::Motrice { .. }
            | Composant::BlocMoteur { .. }
            | Composant::Reservoir { .. } => (Vec3::ZERO, self.rayon_local()),
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
            Composant::TreillisHexagone { profil, liaison } => (Vec3::ZERO, (profil.rayon() * 1.1).max(*liaison)),
            // Nacelle : déployée d'un seul côté (+Z), sphère décalée à mi-corps
            // — sinon, centrée sur le montage, elle mordrait sur les voisines.
            Composant::NacelleCargo { profil, longueur, .. } => cargo::nacelle_englobant(*profil, *longueur),
            // Râtelier et module d'habitat : structurels, centrés sur leur axe.
            Composant::RatelierCargo { .. } | Composant::ModuleHabitat { .. } => {
                (Vec3::ZERO, self.rayon_local())
            }
            // Sous-ensemble : centré sur l'origine du sous-ensemble, comme une Station.
            Composant::SousEnsemble { donnees, .. } => (Vec3::ZERO, donnees.rayon),
            Composant::PanneauSolaire { .. }
            | Composant::Radiateur { .. }
            | Composant::Antenne { .. }
            | Composant::Caisson { .. }
            | Composant::ChargeUtile { .. }
            | Composant::RadiateurMega { .. } => {
                let r = self.rayon_local();
                (Vec3::Z * (r * 0.5), r * 0.55)
            }
        }
    }
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
        assert_eq!(sous.englobant_local(), (Vec3::ZERO, rayon_attendu));
    }

    #[test]
    fn sous_ensemble_se_clipse_comme_nimporte_quel_composant() {
        // Gèle "nœud + un module dessus" en une brique, puis clipse CETTE
        // brique sur un port libre d'un chantier différent : la composabilité
        // recherchée (assembler plusieurs composants, dont des composites).
        let mut interne = Chantier::new();
        interne.racine(Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six });
        assert!(interne.poser(0, Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 }, 1));
        let brique = interne.figer(Profil::P1).unwrap();

        let mut externe = Chantier::new();
        externe.racine(Composant::Treillis { profil: Profil::P1, longueur: 4.0, style: StyleTreillis::Carre });
        let port_axial = externe
            .libres()
            .iter()
            .position(|p| p.genre == GenrePort::ModuleAxial)
            .expect("le treillis a des bouts axiaux");
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
        let axial = ch.libres().iter().position(|p| p.genre == GenrePort::ModuleAxial).unwrap();
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

    #[test]
    fn radiateur_mega_un_port_surface() {
        let c = Composant::RadiateurMega { profil: Profil::P0, longueur: 10.0, largeur: 5.5, ailettes: 34 };
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
        assert!((c.rayon_local() - 1.1).abs() < 1e-4); // (P1.rayon() * 1.1).max(liaison)
        let mut b = Batisseur::new();
        c.dessiner(&mut b);
        assert!(!b.terminer().is_empty());
    }
}
