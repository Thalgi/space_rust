use crate::vaisseau::montage::{cuire, poser};
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, PiedHexa, Profil, Repere, VarianteCoiffe, VarianteModule,
};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_1_SQRT_2, PI};
use super::demos::{
    bouclier_thermique, poser_equipage, poser_grappe_habitat, poser_tete_bouclier, BOUCLIER_DEBUT_Y,
    EQUIPAGE_CENTRE_Y, HAB_CENTRE_Y, THERMIQUE_DEBUT_Y,
};
use crate::vaisseau::generateur::presets::demos::EtatEquipage;

// --- Échelle de l'ossature de l'ISV -----------------------------------------
/// Facteur appliqué à l'**épine et à la propulsion** (2026-07-29, à l'œil).
///
/// Il est appliqué **géométriquement**, en composant une mise à l'échelle dans
/// les transformées cuites, et non en multipliant des cotes : l'épaisseur du
/// treillis, le diamètre des modules Cœur et le gabarit des hexagones viennent
/// de `Profil`, un enum discret plafonné à P3 — aucune constante ne saurait les
/// étirer de 20 %.
pub(crate) const ISV_ECHELLE: f32 = 1.2;

/// Extension hors-tout de la flèche de l'épine **à l'échelle 1** : demi-section
/// 0,5, longerons de rayon 0,225 posés sur les **coins** du carré, donc à
/// √2·0,5 de l'axe. Mesurée sur le dessin de [`Composant::Charpente`].
pub(crate) const EPINE_FLECHE: f32 = 0.5 * std::f32::consts::SQRT_2 + 0.225;
/// La même, au gabarit réel du vaisseau. **C'est la référence** de tout ce qui
/// vient se placer autour de l'épine : le fret et l'habitat gardent leur
/// taille, mais leurs rayons de couronne se recalent là-dessus.
pub(crate) const EPINE: f32 = EPINE_FLECHE * ISV_ECHELLE;

/// La même cote pour l'épine **hexagonale**, à l'échelle 1.
///
/// Même forme que [`EPINE_FLECHE`], et ce n'est pas un hasard : le circonradius
/// de la section hexagonale est **repris des coins du carré** (`0,5·√2`, cf.
/// `treillis::hexa_rayons`), donc les sommets sont à la même distance de l'axe.
/// Seule diffère l'épaisseur des longerons — `0,12 · 1,5·√2 = 0,2546` contre
/// `0,15 · 1,5 = 0,225`.
///
/// ⚠️ L'épine hexagonale est donc **3,2 % plus large hors-tout**. Ça paraît
/// négligeable et ça ne l'est pas : c'est exactement la nature de l'erreur de
/// §C.6, où l'épine avait grossi et la charge utile s'était retrouvée plantée
/// dans la structure. D'où le paramétrage qui suit — aucun rayon de couronne
/// n'est écrit en dur, tous se déduisent de la variante d'épine.
pub(crate) const EPINE_FLECHE_HEXA: f32 = 0.5 * std::f32::consts::SQRT_2 + 0.2546;

/// Section de l'épine du vaisseau. Les deux variantes coexistent le temps de
/// trancher à l'écran — voir `docs/suivi/stations.md` §C.9.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Epine {
    /// Treillis à section **carrée** : l'épine historique.
    #[default]
    Carree,
    /// Treillis à section **hexagonale** : silhouette bien plus constante selon
    /// l'angle (donc lisible sous filtre pixel), et pied en **tour coaxiale** qui
    /// prolonge le cône au lieu d'un cadre couché à raccorder.
    Hexagonale,
}

impl Epine {
    /// Hors-tout de la flèche, **au gabarit du vaisseau**. C'est la référence de
    /// tout ce qui vient se placer autour de l'épine : la charge utile garde sa
    /// taille propre, mais ses rayons de couronne se recalent là-dessus.
    pub(crate) const fn hors_tout(self) -> f32 {
        ISV_ECHELLE
            * match self {
                Epine::Carree => EPINE_FLECHE,
                Epine::Hexagonale => EPINE_FLECHE_HEXA,
            }
    }

    /// La charpente correspondante, à cotes identiques.
    fn charpente(self, grand: Profil, petit: Profil, longueur: f32, courbure: f32) -> Composant {
        match self {
            Epine::Carree => Composant::Charpente { grand, petit, longueur, courbure, aiguille: true },
            Epine::Hexagonale => {
                // **Pied en pavillon** (§C.11) : la corolle et son fût, sur lequel
                // la propulsion viendra se reposer. C'est le seul point où les deux
                // ISV diffèrent en plus de la section de leur épine.
                Composant::CharpenteHexa {
                    grand,
                    petit,
                    longueur,
                    courbure,
                    pied: PiedHexa::Pavillon,
                }
            }
        }
    }

    /// Libellé pour la vue.
    pub fn nom(self) -> &'static str {
        match self {
            Epine::Carree => "EPINE CARREE",
            Epine::Hexagonale => "EPINE HEXAGONALE",
        }
    }
}

/// **Déport latéral** (±X) des deux ensembles propulsion — aile radiateur et bloc
/// moteur — dans le repère de l'ossature.
///
/// Descendu à **6,5** (2026-07-30) pour que la propulsion se **pose sur** l'épine
/// au lieu de la longer : 6,5 → 9,5 → 8,5 → 6,5, au fil des passes visuelles.
///
/// ⚠️ **La valeur se mesure, elle ne se déduit pas**, et de deux façons :
///
/// 1. le rayon interne de l'aile n'est pas `déport − largeur/2` — le collecteur du
///    radiateur rentre plus près de l'axe que la pointe des ailettes ;
/// 2. il faut mesurer **dans la tranche que l'aile et le pied partagent**. Sur
///    l'ensemble de sa longueur, l'aile descend bien plus près de l'axe que là où
///    elle croise le fût : à 7,5 on lisait un engagement de −1,31 alors qu'au droit
///    du fût il n'était que de **−0,04**. Autant dire aucun contact.
///
/// Relevé au droit du fût (bande partagée), pied à 5,65 :
///
/// | déport | rayon interne de l'aile | engagement |
/// |---|---|---|
/// | 7,5 | 5,61 | −0,04 — se frôlent à peine |
/// | 7,0 | 5,01 | −0,64 |
/// | **6,5** | **4,41** | **−1,24** |
/// | 6,0 | 3,81 | −1,84 |
pub(crate) const PROPULSION_DEPORT: f32 = 6.5;

/// **Avancée** de la propulsion et des cuves le long de l'épine, **vers les
/// tuyères** (donc −Y dans le repère de l'ossature, −X sur le modèle couché).
///
/// ⚠️ **Bornée par le fût.** L'aile radiateur s'enracine à `−20` et se déploie vers
/// le bas ; le fût, lui, s'arrête à `−22`. Seuls ces 2 unités de recouvrement font
/// que la propulsion touche encore la structure. Avancer de plus de ~2 la ferait
/// **sortir sous le fût** : elle pendrait alors dans le vide, et l'engagement
/// latéral obtenu en resserrant `PROPULSION_DEPORT` ne servirait plus à rien.
pub(crate) const PROPULSION_AVANCE: f32 = 1.0;

/// **Jeu** entre les deux cuves de carburant d'un même côté.
///
/// Leur écart se déduit de leur propre rayon (`2·res_r + jeu`) et **non** de
/// l'écartement des plaques hexagonales, avec lequel il était confondu — deux
/// cotes qui n'ont aucune raison d'être égales, et qui laissaient les cuves
/// s'interpénétrer de 1,30.
pub(crate) const RESERVOIR_JEU: f32 = 0.7;

/// Longueur de la charpente-épine **à l'échelle 1**. L'évasement de la base
/// étant à distance absolue, la rallonger n'allonge que la flèche.
pub(crate) const EPINE_LONGUEUR: f32 = 84.0;
/// Ancrage de la base de l'épine, côté tuyères.
pub(crate) const EPINE_BASE_Y: f32 = -16.0;
/// Décalage supplémentaire vers le haut, en fraction de la longueur : la
/// charpente monte, radiateurs et moteurs restent en bas.
pub(crate) const EPINE_DECALAGE: f32 = 0.1;
/// **Sommet de l'épine** à l'échelle 1. C'est la limite au-delà de laquelle une
/// charge utile n'aurait plus rien à quoi s'accrocher — la section d'équipage,
/// qui est la pièce la plus haute du vaisseau, doit rester en dessous.
pub(crate) const EPINE_SOMMET_Y: f32 = EPINE_BASE_Y + EPINE_LONGUEUR * (1.0 + EPINE_DECALAGE);

// --- Section fret de l'ISV -------------------------------------------------
// Gabarit de base d'une rangée, puis une **échelle** unique par-dessus : c'est
// le rapport fret/vaisseau qui se juge à l'œil, et on le règle d'un seul
// chiffre plutôt qu'en retouchant trois cotes qui doivent rester cohérentes.
pub(crate) const FRET_LONG: f32 = 6.6; // longueur d'une rangée
pub(crate) const FRET_PAS: f32 = 7.2; // entraxe entre rangées
/// Nombre de rangées enfilées sur l'épine.
pub(crate) const FRET_RANGEES: usize = 4;
/// Bord **bas** du bloc de fret (côté moteurs), le long de l'épine, **à
/// l'échelle 1** (il est multiplié par [`ISV_ECHELLE`] à la pose, comme tout ce
/// qui se repère le long de l'épine).
///
/// Le bloc est ancré par ce bord et non par son centre : c'est là que finit le
/// tronçon d'épine nu et que commence la charge utile, donc le point qui a un
/// sens physique. Ajouter ou retirer une rangée allonge ou raccourcit le bloc
/// **vers le haut**, en libérant d'autant l'extrémité — celle qui doit encore
/// recevoir l'habitat, les modules d'équipage et le bouclier antidébris.
pub(crate) const FRET_DEBUT_Y: f32 = 42.5;
/// Échelle d'ensemble du fret — **le seul chiffre à toucher** pour juger le
/// rapport fret/vaisseau à l'œil. Porte sur les trois cotes du fret (longueur
/// de rangée, entraxe **et** taille de conteneur), pour qu'elles ne puissent
/// pas dériver les unes par rapport aux autres.
/// Passée de 0,70 à **0,56** (−20 %, 2026-07-29).
pub(crate) const FRET_ECHELLE: f32 = 0.56;

/// Rayon hors-tout d'un conteneur **à l'échelle 1**.
pub(crate) const FRET_NACELLE_BASE: f32 = 2.947;
/// Rayon hors-tout d'un conteneur, au gabarit réel.
///
/// Il ne dépend **pas** de [`FRET_RAYON`], et c'est le point : quand l'épine
/// change de gabarit, c'est la couronne qui s'ouvre, pas le fret qui grossit.
pub(crate) const FRET_NACELLE: f32 = FRET_NACELLE_BASE * FRET_ECHELLE;
/// **Jeu** entre la face interne d'un conteneur et la surface de l'épine.
///
/// Le fret est **déjà au ras** : ce qui commande, c'est le **coin** du treillis
/// carré, et le vide qu'on croit voir entre eux vient de ses **faces**, en
/// retrait de ~40 % par rapport à ses coins (côté à `0,5·k`, coin à
/// `√2·0,5·k`). Descendre beaucoup plus bas ferait mordre les conteneurs sur
/// les longerons d'angle.
pub(crate) const FRET_JEU: f32 = 0.02;
/// Rayon de la couronne de fret : juste assez pour que le creux central laisse
/// passer l'épine. Le creux vaut `rayon − r_nacelle·(0,5 + f/2)`, d'où la
/// formule — plus de nombre magique à re-régler quand [`ISV_ECHELLE`] bouge.
pub(crate) const fn fret_rayon(epine: f32) -> f32 {
    epine + FRET_JEU + FRET_NACELLE * (0.5 + 0.5 * 0.22)
}
/// Le même, pour l'épine carrée — la valeur historique, gardée pour les tests et
/// pour que le calcul n'existe **qu'une fois**.
#[allow(dead_code)] // Mesure de geometrie tenue par ses tests ; aucun dessin ne l'appelle aujourd'hui.
pub(crate) const FRET_RAYON: f32 = fret_rayon(EPINE);

/// Preset : **charpente de l'ISV** (première pierre du vaisseau fidèle). Une
/// **seule** charpente à profil **courbe** — plus de cône + tige assemblés : la
/// base (P3) s'évase vers le bas puis s'affine (P0) en une longue flèche vers le
/// haut, d'un seul tenant. Moteurs, habitat et voiles radiateurs viendront s'y
/// accrocher ensuite.
#[allow(dead_code)] // ISV **monolithique**, remplace au catalogue par `preset_isv_fixe` + `preset_isv_equipage` ; garde comme reference d'assemblage complet.
pub fn preset_isv() -> EtatStation {
    isv(Epine::Carree, true, 0.0)
}

/// Le **second ISV**, identique au premier à une chose près : son épine est
/// [`Epine::Hexagonale`].
///
/// Les deux presets partagent **tout le reste du code** — même ossature, même
/// propulsion, même charge utile, même section d'équipage rotative. C'est
/// délibéré : à la moindre duplication, la comparaison ne voudrait plus rien dire,
/// puisqu'on ne saurait plus si un écart vient de la section d'épine ou d'une
/// dérive entre deux copies.
///
/// Les rayons de couronne, eux, **se recalent tout seuls** : l'épine hexagonale
/// est 3,2 % plus large hors-tout, et fret, habitat et alésage de collier se
/// déduisent tous de `Epine::hors_tout()`.
// Non réexporté (cf. `vaisseau/mod.rs`) : la vue passe par les deux moitiés, et
// seul un test consomme le vaisseau d'un seul tenant.
#[allow(dead_code)]
pub fn preset_isv_hexa() -> EtatStation {
    isv(Epine::Hexagonale, true, 0.0)
}

/// L'ISV **sans** sa section d'équipage : tout ce qui ne tourne pas.
///
/// La vue s'en sert pour cuire le vaisseau en **deux maillages** — le fixe et le
/// tournant ([`preset_isv_equipage`]) — et n'appliquer la rotation qu'au second.
/// Sans cette séparation, faire tourner la section obligerait soit à faire
/// tourner le vaisseau entier, soit à le recuire à chaque frame.
/// `chaleur` (0 à 1) porte les ailes radiateur au rouge puis à l'orange. Elle
/// est ici et non dans un réglage d'affichage parce qu'elle **change la
/// géométrie cuite** : les couleurs sont dans les sommets, donc chauffer un
/// radiateur veut dire le recuire.
pub fn preset_isv_fixe(epine: Epine, chaleur: f32) -> EtatStation {
    isv(epine, false, chaleur)
}

pub(crate) fn isv(epine: Epine, avec_equipage: bool, chaleur: f32) -> EtatStation {
    // Gabarit hors-tout de l'épine : **toute** la charge utile s'y recale.
    let gabarit = epine.hors_tout();
    let mut asm = Assembleur::new();
    // L'**ossature** (épine + propulsion) se construit dans son propre tampon,
    // à l'échelle 1, puis est reversée agrandie de [`ISV_ECHELLE`]. Passer par
    // une mise à l'échelle *géométrique* est le seul moyen d'agrandir aussi ce
    // que `Profil` quantifie — épaisseur de treillis, diamètre des Cœurs,
    // gabarit des hexagones. La charge utile, elle, garde sa taille : elle est
    // ajoutée **après**, directement dans `asm`, à des positions simplement
    // recalées sur le nouveau gabarit.
    let mut oss = Assembleur::new();

    // Charpente continue, axe le long de +Y (« vers le haut »). Base (P3, bout
    // −Z local) vers le bas, apex (P0, +Z local) vers le haut. `courbure` élevée
    // → l'évasement se concentre en bas, le reste file en flèche fine.
    // Évasement fixe (distance absolue) → rallonger `longueur` n'allonge que la
    // flèche : ~34 → 40 ajoute un tiers de tige sans toucher à la base.
    // Pied posé (`aiguille`) : anneau hexagonal couché pour l'épine carrée, tour
    // hexagonale coaxiale pour l'hexagonale. `petit` = P1 (bout agrandi, section
    // doublée par rapport au P0 d'origine).
    let longueur = EPINE_LONGUEUR;
    let charpente = epine.charpente(Profil::P3, Profil::P1, longueur, 2.6);
    // Position = centre local de la charpente. Base ancrée à Y = −16, la
    // rallonge part vers le **haut** (Y_centre = base + L/2), plus un **décalage
    // vers le haut d'un bon dixième de la hauteur** (la charpente monte, les
    // radiateurs/moteurs restent en bas).
    let decalage = longueur * EPINE_DECALAGE;
    let y_centre = EPINE_BASE_Y + longueur * 0.5 + decalage;
    let base = Repere::new(vec3(0.0, y_centre, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::Y));
    oss.ajouter(cuire(base, &charpente));

    // Deux grandes **ailes radiateur méga** près de la base évasée (côté
    // moteurs). Le vaisseau s'étend en Y ∈ [−20, +20], base vers −Y.
    //
    // **Côte à côte, même sens** (pas en miroir) : les deux se déploient le long
    // de +Y (vers le corps), connecteur en bas côté base. Puis on les **bascule
    // de 5°** pour que la **pointe rentre vers l'intérieur** — donc le connecteur
    // (où viendra le propulseur à antimatière) part **vers l'extérieur** : le
    // moteur ne tire plus dans la station.
    let tilt = 5.0_f32.to_radians();
    let radia_w = 4.4 * 1.2; // largeur du radiateur, +20 %
    for cote in [1.0_f32, -1.0] {
        let aile = Composant::RadiateurMega {
            profil: Profil::P0,
            longueur: 16.5,
            largeur: radia_w,
            ailettes: 28,
            chaleur,
        };
        let orient = Quat::from_rotation_arc(Vec3::Z, Vec3::NEG_Y); // sens inversé
        let rot = Quat::from_rotation_z(cote * tilt) * orient;
        let pos = Vec3::new(-PROPULSION_DEPORT * cote, -20.0 - PROPULSION_AVANCE, 0.0);
        let repere = Repere::new(pos, rot);
        oss.ajouter(cuire(repere, &aile));
        // Bloc moteur docké au collecteur de CE radiateur, comme dans la vue
        // radiateur+bloc moteur. Le côté −X est le **flip** de l'autre (miroir).
        // `propulseur = true` : version **complète** (Cœur 3 noir + chapes bombées
        // sur Cœur 1/2 + propulseur antimatière) intégrée à la charpente.
        poser_bloc_moteur(&mut oss, repere, radia_w, cote < 0.0, true, chaleur);
    }

    // **Réservoirs de carburant** : une cuve sphérique de **chaque côté (±X)** du
    // volume hexagonal, à sa hauteur. Le centre de l'hexagone est recalculé comme
    // au dessin (`Charpente`) : base de la charpente moins `ap + sec`.
    let sg = Profil::P3.rayon() * 0.5; // section du treillis (= TREILLIS_SECTION)
    let cote = 2.0 * sg; // côté de l'hexagone
    let ap = cote * 3.0_f32.sqrt() * 0.5; // apothème
    let hex_y = y_centre - longueur * 0.5 - (ap + sg * 0.5); // centre hexagone (avant pivot)
    let prof = sg; // demi-profondeur du volume hexagonal (hors-plan = axe Z)
    let res_long = 5.0_f32; // taille des cuves
    let res_r = res_long * 0.5 * 1.3; // rayon de la cuve sphérique (cf. dessin Reservoir)
    let reservoir = Composant::Reservoir { profil: Profil::P1, longueur: res_long, cage: false };
    // Une cuve sur **chaque face** de l'hexagone, le long de ±Z — l'axe
    // **perpendiculaire** aux radiateurs. Radiateurs sur ±X (→ nord/sud), cuves
    // sur ±Z (→ est/ouest). Reculées assez pour dégager la face de l'hexagone.
    // Le tétraèdre a sa face plate côté −Z local : la cuve +Z la présente déjà à
    // l'hexagone ; on **retourne** la cuve −Z (½ tour en X) pour que sa face
    // plate regarde l'hexagone elle aussi. Les deux faces plates restent
    // **parallèles** à la surface de l'hexagone.
    // Rotation autour de l'axe Z local : fait pivoter la face triangulaire
    // (perpendiculaire à Z) présentée à l'hexagone, sans changer son orientation
    // générale. 30° d'alignement **+ 180°** (demi-tour Z : les cuves étaient
    // orientées à l'envers dans la vue ISV).
    let spin = Quat::from_rotation_z(PI / 6.0 + PI);
    // **Deux cuves par côté** : l'existante au niveau de l'hexagone, et une
    // dupliquée plus bas le long de la charpente, avec un **demi-tour en Z**
    // supplémentaire.
    //
    // Écart des **plaques hexagonales** = `2·ap` : leurs demi-hauteurs (`ap` en Y)
    // se touchent alors bord à bord, sans montants.
    let dy = 2.0 * ap;
    // Écart des **cuves**, désormais **découplé** de celui des plaques.
    //
    // Les deux tenaient le même écart, et c'était trop juste pour elles : deux
    // sphères de rayon `res_r` distantes de `2·ap` = 5,20 s'interpénètrent de
    // **1,30**, ce qui se voyait — la note « léger chevauchement accepté » sous-
    // estimait franchement. La cote se déduit maintenant du rayon des cuves plutôt
    // que de la géométrie des plaques, qui n'a rien à voir avec elles.
    let res_ecart = 2.0 * res_r + RESERVOIR_JEU;
    for sz in [1.0_f32, -1.0] {
        let base = if sz > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_x(PI) };
        // Le retournement (rotation_x PI) de la cuve −Z **mire** le triangle, ce qui
        // désaligne son sommet de 60° : on le rattrape pour que les deux cuves
        // pointent un sommet vers la tige de la charpente.
        let corr = if sz > 0.0 { Quat::IDENTITY } else { Quat::from_rotation_z(PI / 3.0) };
        let rot = base * spin * corr;
        let z = sz * (prof + res_r - 1.0); // réservoir enfoncé dans la charpente (écart −1.0)
        // Cuve d'origine.
        oss.ajouter(cuire(Repere::new(vec3(0.0, hex_y - PROPULSION_AVANCE, z), rot), &reservoir));
        // Cuve dupliquée : plus bas le long de la charpente + demi-tour Z. Elle
        // descend selon `res_ecart` et non `dy` — c'est ce découplage qui la
        // décolle de sa jumelle sans déplacer les plaques hexagonales.
        let rot2 = rot * Quat::from_rotation_z(PI);
        oss.ajouter(cuire(Repere::new(vec3(0.0, hex_y - res_ecart - PROPULSION_AVANCE, z), rot2), &reservoir));
    }

    // **Second anneau hexagonal** au niveau du groupe de réservoirs dupliqué
    // (`hex_y - dy`) : comme l'écart vaut `2·ap`, il **touche** bord à bord celui
    // du pied de la charpente. Plus besoin de montants (`liaison = 0`).
    let hexa = Composant::TreillisHexagone { profil: Profil::P3, liaison: 0.0 };
    let hexa_rot = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    oss.ajouter(cuire(Repere::new(vec3(0.0, hex_y - dy, 0.0), hexa_rot), &hexa));

    // **Section charge utile** : quatre rangées de fret enfilées sur l'épine, à
    // l'**autre bout** que les moteurs. C'est la disposition du vrai ISV, qui
    // est un **tracteur** : les moteurs tirent, la charge suit au bout d'une
    // longue épine en tension (elle encaisse mieux la traction que la poussée,
    // et l'écart protège la charge des tuyères).
    //
    // Rangées en **triforce** : trois conteneurs par rangée, dont le creux
    // triangulaire central laisse passer l'épine. Vérifié : au-delà de ~16 % de
    // sa longueur la charpente est réduite à sa flèche fine (demi-section 0,5,
    // soit ~0,93 hors-tout longerons compris) ; à `rayon` 3,2 le creux offre un
    // cercle inscrit de 1,40. La marge est donc franche, et c'est *pour ça* que
    // le fret est ici et pas plus bas, où la base évasée (1,5) mordrait dedans.
    //
    // On s'arrête vers Y = 66 : le haut de l'épine reste libre pour l'habitat,
    // les modules d'équipage et le bouclier antidébris, encore à faire.
    // Ossature terminée : on la verse **agrandie** dans l'assemblage final.
    super::commun::verser_a_echelle(&mut asm, oss.terminer(), ISV_ECHELLE);

    // À partir d'ici, tout est posé **au gabarit final** : la charge utile garde
    // sa taille propre, seules ses positions le long de l'épine et ses rayons de
    // couronne suivent l'échelle de l'ossature.
    let rat_long = FRET_LONG * FRET_ECHELLE;
    let rat_pas = FRET_PAS * FRET_ECHELLE; // un jour net : les rangées se lisent séparément
    for k in 0..FRET_RANGEES {
        let ratelier = Composant::RatelierCargo {
            profil: Profil::P1,
            longueur: rat_long,
            // La couronne s'ouvre pour laisser passer l'épine élargie ; le
            // conteneur, lui, garde exactement sa taille validée à l'écran.
            rayon: fret_rayon(gabarit),
            nacelles: 3,
            nacelle: FRET_NACELLE,
        };
        let y = (FRET_DEBUT_Y + rat_long * 0.5 + k as f32 * rat_pas) * ISV_ECHELLE;
        asm.ajouter(cuire(Repere::new(vec3(0.0, y, 0.0), hexa_rot), &ratelier));
    }

    // **Habitat principal**, juste au-dessus du fret : l'ordre le long de
    // l'épine suit celui du vrai vaisseau — moteurs, épine nue, fret, puis
    // l'habitat le plus loin possible des tuyères. Les ferrures de chaque
    // module viennent se poser sur la flèche (leur portée est calée dessus,
    // cf. `HAB_ATTACHE`).
    poser_grappe_habitat(&mut asm, HAB_CENTRE_Y * ISV_ECHELLE, hexa_rot, gabarit);

    // **Section d'équipage rotative**, au-delà de l'habitat : le point du
    // vaisseau le plus éloigné des tuyères. Elle n'est incluse ici que pour un
    // rendu d'un seul tenant ; la vue, qui doit la faire tourner, la reprend
    // séparément via `preset_isv_equipage`.
    if avec_equipage {
        poser_equipage(
            &mut asm,
            EQUIPAGE_CENTRE_Y * ISV_ECHELLE,
            hexa_rot,
            EtatEquipage::default().repli(),
            gabarit,
        );
    }

    // **Bouclier thermique** au droit des moteurs, là où l'épine prend le
    // rayonnement des tuyères de plein fouet. C'est un **détail de surface** —
    // il n'ajoute ni masse visible ni encombrement, et ne change donc rien aux
    // proportions d'ensemble.
    asm.ajouter(cuire(
        Repere::new(vec3(0.0, THERMIQUE_DEBUT_Y, 0.0), hexa_rot),
        &bouclier_thermique(),
    ));

    // **Tête de bouclier**, tout au bout : la petite plaque puis les trois
    // grandes, enfilées sur leur propre mât au-delà du sommet d'épine. Elle est
    // posée avec la coque fixe et non avec la section d'équipage — c'est de la
    // structure, elle ne tourne pas.
    poser_tete_bouclier(&mut asm, BOUCLIER_DEBUT_Y, hexa_rot);

    // **Modèle complet à l'horizontale** : rotation globale de 90° autour de Z
    // (l'axe +Y du vaisseau bascule vers +X), appliquée à toutes les pièces.
    pivoter(asm.terminer(), ISV_COUCHE)
}

/// Pivot final du modèle : le vaisseau est couché, son axe +Y basculant sur +X.
/// Sorti en constante parce que **les deux moitiés** de l'ISV (la partie fixe et
/// la section d'équipage) doivent subir exactement le même, sous peine de ne
/// plus se rejoindre.
pub(crate) const ISV_COUCHE: Quat = Quat::from_xyzw(0.0, 0.0, -FRAC_1_SQRT_2, FRAC_1_SQRT_2);

/// L'**axe du vaisseau** dans le modèle final (après [`ISV_COUCHE`]) : c'est
/// autour de lui que tourne la section d'équipage.
pub const ISV_AXE: Vec3 = Vec3::X;

/// La **seule section d'équipage** de l'ISV, placée dans le repère du vaisseau
/// fini — donc superposable à [`preset_isv`].
///
/// Elle est fournie à part précisément parce qu'elle **tourne** : la faire
/// pivoter revient à composer une matrice sur ce maillage-là, au lieu de recuire
/// tout le vaisseau à chaque frame. Le repli, lui, change la géométrie et
/// impose bien une reconstruction — d'où le paramètre.
/// `repli` est la valeur **continue** (0 déployé → 1 replié) et non un
/// [`EtatEquipage`] : la vue anime le passage d'un état à l'autre, et a donc
/// besoin des positions intermédiaires.
pub fn preset_isv_equipage(epine: Epine, repli: f32) -> EtatStation {
    let mut asm = Assembleur::new();
    let hexa_rot = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    poser_equipage(
        &mut asm,
        EQUIPAGE_CENTRE_Y * ISV_ECHELLE,
        hexa_rot,
        repli,
        epine.hors_tout(),
    );
    pivoter(asm.terminer(), ISV_COUCHE)
}

/// Applique une rotation globale `q` (autour de l'origine monde) à **toutes** les
/// pièces d'un état déjà publié : chaque transformée cuite est pré-multipliée par
/// `q`. Sert à réorienter un modèle complet (ex. ISV couché à l'horizontale).
pub(crate) fn pivoter(etat: EtatStation, q: Quat) -> EtatStation {
    let EtatStation::Prete(s) = etat else {
        return etat;
    };
    let rot = Mat4::from_quat(q);
    let pieces: Vec<crate::vaisseau::Piece> = s
        .pieces()
        .iter()
        .map(|p| crate::vaisseau::Piece::new(rot * p.transforme, p.composant.clone()))
        .collect();
    crate::vaisseau::Station::depuis_pieces(pieces)
        .map(EtatStation::Prete)
        .unwrap_or(EtatStation::Vide)
}

/// Seconde vue ISV (chantier progressif) : un **radiateur méga** et le **bloc
/// moteur** (caisse collecteur + `ModuleAxial` doré) assemblé **au bout de son
/// connecteur**. On fait évoluer la jonction petit à petit.
pub fn preset_isv_moteur() -> EtatStation {
    let mut asm = Assembleur::new();

    // Largeur hors-tout du bloc moteur = envergure de sa rangée de 5 habitats
    // (même formule que `BlocMoteur::dessiner`, largeur de caisse 4.4).
    let bloc_w = 4.4_f32;
    let hy = bloc_w * 0.42;
    let lx = 4.0 * (hy * 1.05 * 0.66) + 2.0 * (hy * 0.5 * 0.66);

    // Radiateur méga : **même largeur** que le bloc moteur, **hauteur doublée**
    // (longueur 16.5 → 33). Centré en X (pas d'offset).
    let radia = Composant::RadiateurMega {
        profil: Profil::P0,
        longueur: 33.0,
        largeur: lx,
        ailettes: 28,
        chaleur: 0.0,
    };
    asm.ajouter(cuire(Repere::IDENTITE, &radia));

    // Bloc moteur docké au collecteur du radiateur (radiateur au repère identité).
    // `propulseur = true` : Cœur 3 reçoit le propulseur à antimatière complet.
    poser_bloc_moteur(&mut asm, Repere::IDENTITE, lx, false, true, 0.0);

    asm.terminer()
}

/// Pose le **bloc moteur** complet (caisse + rangée d'habitats + les 3 Cœurs),
/// **docké au collecteur** du radiateur dont le repère monde est `radia` (de
/// largeur `radia_largeur`) — exactement comme la vue « radiateur + bloc moteur ».
/// Tout est composé dans le repère du radiateur, donc valable même incliné.
#[allow(clippy::too_many_arguments)]
pub(crate) fn poser_bloc_moteur(asm: &mut Assembleur, radia: Repere, radia_largeur: f32, miroir: bool, propulseur: bool, regime: f32) {
    let bloc_w = 4.4_f32;
    let hy = bloc_w * 0.42;
    let lx = 4.0 * (hy * 1.05 * 0.66) + 2.0 * (hy * 0.5 * 0.66); // envergure rangée

    // Bloc retourné, bout −Z de la rangée **au ras du départ du panneau**
    // (z0 = +bd/2). Le collecteur ayant été retiré, c'est là que commence la
    // matière du radiateur : on comble ainsi l'écart laissé par la caisse.
    let bd = (radia_largeur * 0.5).max(1.5);
    let face = bd * 0.5;
    let bm = radia.compose(Repere::new(vec3(0.0, 0.0, face - 4.1), Quat::from_rotation_y(PI)));

    let bloc = Composant::BlocMoteur { profil: Profil::P2, largeur: bloc_w };
    asm.ajouter(cuire(bm, &bloc));

    let port9 = bm.compose(bloc.ports()[1].repere); // port +Z du bloc
    // Axe de la rangée de Cœurs. `miroir` inverse le côté → l'autre moteur est
    // l'exact **flip en X** (symétrie non cassée).
    let xdir = bm.rot * if miroir { Vec3::X } else { Vec3::NEG_X };

    // Cœur 1 : P2, longueur 9.0, offset +25 % le long de xdir.
    let coeur1 = Composant::ModuleAxial { profil: Profil::P2, variante: VarianteModule::Coeur, longueur: 9.0 };
    let mut w1 = poser(port9, &coeur1, 1);
    w1.pos += xdir * (0.25 * lx);
    asm.ajouter(cuire(w1, &coeur1));

    // Cœur 2 : P2, longueur 4.5, tambour contre tambour à Cœur 1 (chevauche le bloc).
    let coeur2 = Composant::ModuleAxial { profil: Profil::P2, variante: VarianteModule::Coeur, longueur: 4.5 };
    let port_tambour = w1.compose(coeur1.ports()[1].repere);
    let w2 = poser(port_tambour, &coeur2, 1);
    asm.ajouter(cuire(w2, &coeur2));

    // Chapes **bombées** sur les bouts **exposés** (+Z) de Cœur 1 et Cœur 2
    // (leurs bouts −Z sont joints tambour contre tambour). Posées **à ras du
    // corps** (face +Z, à `demi`) et non au bout de la collerette — sinon un cou
    // de docking resterait visible sous la chape.
    for (w, demi) in [(w1, 9.0_f32 * 0.5), (w2, 4.5_f32 * 0.5)] {
        let coiffe = Composant::Coiffe { profil: Profil::P2, variante: VarianteCoiffe::Bombee };
        let cw = w.compose(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY));
        asm.ajouter(cuire(cw, &coiffe));
    }

    // Cœur 3 : P1 (diamètre moitié), longueur 9.0, collé sans écart côté −xdir de Cœur 1.
    let coeur3 = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Coeur, longueur: 9.0 };
    let mut w3 = poser(port9, &coeur3, 1);
    w3.pos = w1.pos - xdir * (Profil::P2.rayon() + Profil::P1.rayon());
    asm.ajouter(cuire(w3, &coeur3));

    // Propulseur à antimatière accroché au **bout libre (+Z)** de Cœur 3 : le
    // réacteur se clipse par sa **tête** (port 1), et la tuyère se monte sous sa
    // **base** (port 0) — la poussée pointe donc vers l'extérieur du module.
    if propulseur {
        // Taille choisie pour que le **corps du réacteur** (rayon = 0.40·taille,
        // cf. `ReacteurAntimatiere::dessiner`) ait le **même diamètre que Cœur 3**
        // (Profil::P1) au niveau du raccord : 0.40·taille = P1.rayon().
        let taille = Profil::P1.rayon() / 0.40;
        let port_c3 = w3.compose(coeur3.ports()[0].repere); // écoutille axiale libre (+Z)
        let reacteur = Composant::ReacteurAntimatiere { profil: Profil::P1, taille };
        let rw = poser(port_c3, &reacteur, 1); // montage par la tête (+Z)
        asm.ajouter(cuire(rw, &reacteur));
        let tuyere = Composant::MoteurAntimatiere { profil: Profil::P1, taille };
        let port_base = rw.compose(reacteur.ports()[0].repere); // base −Z du réacteur
        let pose = poser(port_base, &tuyere, 0);
        asm.ajouter(cuire(pose, &tuyere));

        // **Panache**, posé au bout de la tuyère et dans son axe. Le jet part le
        // long du **−Z local** du moteur (c'est là que sont la buse et les deux
        // anneaux de stabilisation) : on retourne donc le repère d'un demi-tour.
        if regime > 1e-3 {
            let sortie = Repere::new(
                pose.pos + pose.rot * (Vec3::NEG_Z * (taille * PANACHE_SORTIE)),
                pose.rot * Quat::from_rotation_y(PI),
            );
            asm.ajouter(cuire(
                sortie,
                &Composant::Panache {
                    longueur: PANACHE_LONGUEUR / ISV_ECHELLE,
                    rayon_col: taille * PANACHE_COL,
                    rayon_bout: PANACHE_BOUT / ISV_ECHELLE,
                    intensite: regime,
                },
            ));
        }
    }
}

// --- Panache d'antimatière ---------------------------------------------------
/// Longueur du jet à pleine poussée, **au gabarit final**.
///
/// Deux longueurs de vaisseau (168 × 2 ≈ 336). Ce n'est pas une exagération : un
/// jet de pions relativistes n'a rien qui l'arrête, et sa portée visible dit
/// exactement ce que la propulsion a d'inhabituel. Une queue courte donnerait un
/// moteur chimique.
pub(crate) const PANACHE_LONGUEUR: f32 = 336.0;
/// Rayon du jet à son bout, au gabarit final.
///
/// ⚠️ **Cette cote décide si le panache lèche la charge utile.** Les tuyères sont
/// braquées de 5° vers l'extérieur précisément pour que le jet passe à côté du
/// vaisseau remorqué ; un panache trop ouvert annulerait ce braquage et
/// reviendrait sur la coque. Gardée par `le_panache_ne_leche_pas_la_charge_utile`.
///
/// Réglée à l'écran : 22 jugé trop large, **divisé par deux**.
pub(crate) const PANACHE_BOUT: f32 = 11.0;
/// Rayon au col, en fraction de la taille du moteur.
///
/// La moitié des anneaux de stabilisation (0,30), et non leur diamètre : le jet
/// sort **pincé** par la tuyère magnétique, plus étroit que l'ouverture qui le
/// laisse passer. C'est d'ailleurs ce que fait un col magnétique — il resserre
/// le faisceau avant de le lâcher.
pub(crate) const PANACHE_COL: f32 = 0.15;
/// Distance de la sortie au repère du moteur, en fraction de sa taille — juste
/// au-delà du second anneau de stabilisation (1,60).
pub(crate) const PANACHE_SORTIE: f32 = 1.66;
