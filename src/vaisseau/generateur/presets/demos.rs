use crate::vaisseau::montage::cuire;
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, PiedHexa, Profil, Repere, StyleTreillis, BOUCLIER_ELANCEMENT,
};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use super::commun::porter_vers;
use super::isv::{EPINE, EPINE_SOMMET_Y, ISV_ECHELLE};

/// Vue briques : la **charpente de l'ISV** (treillis conique courbe) à gauche, et
/// à droite la **même mais terminée en tête d'aiguille** (apex prolongé en flèche
/// fine). Debout, apex vers le haut.
pub fn demo_charpente() -> EtatStation {
    let mut asm = Assembleur::new();
    let debout = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    for (dx, aiguille) in [(-8.0_f32, false), (8.0, true)] {
        let ch = Composant::Charpente {
            grand: Profil::P3,
            petit: Profil::P0,
            longueur: 40.0,
            courbure: 2.6,
            aiguille,
        };
        asm.ajouter(cuire(Repere::new(vec3(dx, 0.0, 0.0), debout), &ch));
    }
    asm.terminer()
}

/// Vue briques : l'**épine hexagonale candidate**, à côté de l'épine carrée
/// actuelle pour que la comparaison soit directe et non de mémoire.
///
/// De gauche à droite : carrée nue, **hexagonale nue**, carrée avec cadre de
/// propulsion, **hexagonale avec cadre**. Ce sont les deux paires qui comptent —
/// la silhouette de la poutre, et la façon dont le pied se raccorde au cadre.
///
/// Ce qu'il faut juger :
/// 1. **de loin, filtre pixel (X) allumé** : l'hexagonale doit rester visible
///    sous tous les angles alors que la carrée maigrit de trois quarts. C'est le
///    grief d'origine, et la seule chose que le calcul promet
///    (`pieces::HEXA_GAIN_SILHOUETTE`) ;
/// 2. **au pied** : la transition six-pour-six et sa baie de torsion, contre les
///    quatre coins convergeant sur deux sommets de la version carrée.
///
/// **Rien n'est encore monté sur l'ISV** : `preset_isv` utilise toujours
/// `Composant::Charpente`. Le remplacement attend la validation à l'écran.
pub fn demo_charpente_hexa() -> EtatStation {
    let mut asm = Assembleur::new();
    let debout = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    // Écartement : de quoi séparer les quatre spécimens sans les perdre de vue.
    for (dx, hexa, aiguille) in [
        (-21.0_f32, false, false),
        (-7.0, true, false),
        (7.0, false, true),
        (21.0, true, true),
    ] {
        let (grand, petit, longueur, courbure) = (Profil::P3, Profil::P0, 40.0, 2.6);
        let ch = if hexa {
            let pied = if aiguille { PiedHexa::Tour } else { PiedHexa::Aucun };
            Composant::CharpenteHexa { grand, petit, longueur, courbure, pied }
        } else {
            Composant::Charpente { grand, petit, longueur, courbure, aiguille }
        };
        asm.ajouter(cuire(Repere::new(vec3(dx, 0.0, 0.0), debout), &ch));
    }
    asm.terminer()
}

/// Vue briques : l'épine hexagonale **à pavillon**, la forme demandée au schéma
/// du 2026-07-30 — le cône ne s'arrête plus sur une tour, il **continue de
/// s'ouvrir** jusqu'à une large embouchure hexagonale évidée.
///
/// À gauche la version **tour** (§C.9), à droite la version **pavillon** (§C.11) :
/// c'est le même cône, seul le pied change, et les deux se comparent d'un regard.
///
/// Ce qu'il faut juger :
/// 1. **l'ouverture** — la corolle doit lire comme une corolle, pas comme un cône
///    tronqué (`PAVILLON_OUVERTURE`, `PAVILLON_HAUTEUR`) ;
/// 2. **l'anneau d'embouchure** vu de bout (tourner la caméra pour regarder dans
///    l'axe) : hexagone extérieur, hexagone intérieur, six panneaux. Son
///    écrasement selon Y donne **quatre** arêtes obliques égales et **deux**
///    arêtes horizontales égales — les A…F du schéma
///    (`PAVILLON_ETIREMENT` ; à 1 les six redeviendraient égales).
///
/// **Rien n'est monté sur l'ISV** : les deux presets restent en pied *tour*. Le
/// pavillon attend sa validation, et la structure de propulsion qui viendra s'y
/// poser.
pub fn demo_epine_pavillon() -> EtatStation {
    let mut asm = Assembleur::new();
    let debout = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);
    for (dx, pied) in [(-11.0_f32, PiedHexa::Tour), (11.0, PiedHexa::Pavillon)] {
        let ch = Composant::CharpenteHexa {
            grand: Profil::P3,
            petit: Profil::P0,
            longueur: 40.0,
            courbure: 2.6,
            pied,
        };
        asm.ajouter(cuire(Repere::new(vec3(dx, 0.0, 0.0), debout), &ch));
    }
    asm.terminer()
}

/// Vue briques : le **fret d'échelle vaisseau** (section charge utile de l'ISV).
/// En haut une nacelle **seule** — sa section « onigiri » (triangle arrondi) se
/// lit de bout, avec ses trois rails d'arête et ses collerettes. En dessous deux
/// **râteliers** de 6 et 8 nacelles : même rayon de couronne, donc des nacelles
/// plus fines quand elles sont plus nombreuses (le pas angulaire commande).
/// Tout est couché le long de X pour montrer l'élancement réel des conteneurs.
pub fn demo_cargo() -> EtatStation {
    let mut asm = Assembleur::new();
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);

    // Nacelle isolée, en gros gabarit : c'est la vue qui sert à juger la section.
    let seule = Composant::NacelleCargo { profil: Profil::P1, longueur: 15.0, spin: 0.0 };
    asm.ajouter(cuire(Repere::new(vec3(-7.5, 11.0, 0.0), couche), &seule));

    // Deux dispositions de râtelier : la **triforce** (3 nacelles de même
    // orientation qui se touchent par la pointe, creux triangulaire au milieu)
    // et la **couronne** de 6 (coin vers l'axe). Le rayon de triforce est plus
    // petit : à 3, la nacelle fait à elle seule tout le rayon de la grappe.
    for (i, (n, rayon)) in [(3usize, 2.6_f32), (6, 4.5)].into_iter().enumerate() {
        let r = Composant::RatelierCargo { profil: Profil::P2, longueur: 16.0, rayon, nacelles: n, nacelle: 0.0 };
        asm.ajouter(cuire(Repere::new(vec3(0.0, -(i as f32) * 13.0, 0.0), couche), &r));
    }
    asm.terminer()
}

// --- Section habitat principal de l'ISV -------------------------------------
// À ne pas confondre avec les **modules d'équipage rotatifs**, qui tournent
// autour du vaisseau pour la gravité artificielle : ceux-là sont une autre
// brique, encore à faire. Ici c'est l'habitat **fixe**, solidaire de l'épine.
/// Longueur d'un module d'habitat (réduite de 33 % à la validation visuelle).
pub(crate) const HAB_LONG: f32 = 8.0;
/// Nombre de modules autour de l'épine (le vrai ISV en a trois).
pub(crate) const HAB_MODULES: usize = 3;
/// Rayon inscrit d'un module (centre → côté plat), pour un fût P2.
pub(crate) const HAB_INSCRIT: f32 = 2.0 * (0.5 + 0.5 * 0.22);
/// **Jeu** entre le côté plat d'un module et la surface de l'épine — le seul
/// chiffre à toucher pour serrer ou desserrer la grappe. Les ferrures ne font
/// que franchir ce jeu, elles se raccourcissent donc avec lui.
///
/// **Seule l'épine borne le serrage** : le plancher est 0 (le module vient au
/// contact). Les trois modules, eux, ne se gênent jamais entre eux — leur
/// propre rayon inscrit (1,22) impose déjà une couronne large devant ce qu'il
/// leur faudrait pour se croiser. Vérifié plutôt que supposé, dans
/// `la_charge_utile_suit_le_gabarit_de_lepine`.
pub(crate) const HAB_JEU: f32 = 0.25;
/// Rayon de la couronne d'habitat (axe → centre de module), déduit du jeu et du
/// gabarit de l'épine : la grappe suit d'elle-même tout changement d'échelle de
/// l'ossature.
pub(crate) const fn hab_rayon(epine: f32) -> f32 {
    epine + HAB_JEU + HAB_INSCRIT
}
/// Rayon de couronne **minimal** pour que les trois modules ne se croisent pas
/// entre eux, indépendamment de l'épine qu'ils entourent.
///
/// Portée d'une section onigiri P2 dans la direction de sa voisine — à 60° d'un
/// coin, donc `dv·cos 30° + ρ` — rapportée à la corde de la couronne
/// (`2·R·sin 60°`). C'est le calcul que faisait déjà
/// `la_charge_utile_suit_le_gabarit_de_lepine` pour **vérifier** le montage de
/// l'ISV ; il **décide** désormais, ce qui est sa place (§10.9). Sur l'épine de
/// l'ISV il ne change rien : le jeu y impose déjà une couronne plus large.
pub(crate) fn hab_rayon_mini() -> f32 {
    let r = Profil::P2.rayon();
    let f = crate::vaisseau::pieces::ONIGIRI_FILET;
    let portee = r * ((1.0 - f) * (PI / 6.0).cos() + f);
    portee / (PI / 3.0).sin()
}

/// Le même, pour l'épine carrée (valeur historique, et calcul unique).
#[allow(dead_code)] // Mesure de geometrie tenue par ses tests ; aucun dessin ne l'appelle aujourd'hui.
pub(crate) const HAB_RAYON: f32 = hab_rayon(EPINE);
/// **Rayon de couronne retenu** : la plus large des deux contraintes.
///
/// Le jeu voulu par rapport à l'épine, ou l'encombrement mutuel des trois
/// modules. Sur l'épine de l'ISV c'est la première qui gagne et rien ne change ;
/// sur un mât fin — la flèche polaire du tore, en P0 — c'est la seconde, et sans
/// elle les trois fûts **se croisent**.
pub(crate) fn hab_couronne(epine: f32) -> f32 {
    hab_rayon(epine).max(hab_rayon_mini() + HAB_JEU)
}

/// Portée de la ferrure d'attache : du côté plat du module jusqu'à la surface
/// de l'épine. Le module se pose **contre** la structure, il ne flotte pas à
/// côté.
///
/// Doublée, parce que les deux ferrures se posent à **mi-portée** de ce champ :
/// c'est `attache/2` qui donne le déport réel des longerons.
///
/// C'est la **ferrure** qui absorbe l'écart quand la couronne s'élargit au-delà
/// du jeu nominal, jamais la couronne qui se rapproche : le longeron doit
/// tomber sur la surface de l'épine quoi qu'il arrive.
pub(crate) fn hab_attache(epine: f32) -> f32 {
    2.0 * (hab_couronne(epine) - HAB_INSCRIT - epine)
}

/// Portée nominale, celle du montage de l'ISV — où c'est le jeu qui commande.
/// Sert de **repère de non-régression** : `hab_attache(EPINE)` doit la redonner.
#[cfg(test)]
pub(crate) const HAB_ATTACHE: f32 = 2.0 * HAB_JEU;
/// Centre de la grappe d'habitat le long de l'épine (avant pivot). Posée juste
/// au-dessus du fret (qui finit vers Y ≈ 57), en laissant le haut de l'épine
/// libre pour les modules d'équipage rotatifs et le bouclier antidébris.
pub(crate) const HAB_CENTRE_Y: f32 = 63.0;

/// Pose la **grappe d'habitat principal** : `HAB_MODULES` fûts composites en
/// couronne autour de l'axe, centrés sur `y_centre`, dans le plan
/// perpendiculaire donné par `orient` (leur axe long suit l'épine).
///
/// Chaque module est tourné (`spin = a`) pour présenter **un coin vers
/// l'extérieur** et donc **un côté plat vers l'axe** — c'est ce côté qui porte
/// la ferrure et vient se boulonner sur l'épine.
///
/// Partagée par la vue Briques et par `preset_isv` : ce qu'on valide à l'écran
/// est **exactement** ce qui part sur le vaisseau.
pub(crate) fn poser_grappe_habitat(asm: &mut Assembleur, y_centre: f32, orient: Quat, epine: f32) {
    let rayon = hab_couronne(epine);
    let attache = hab_attache(epine);
    for k in 0..HAB_MODULES {
        let a = FRAC_PI_2 + TAU * k as f32 / HAB_MODULES as f32;
        let module = Composant::ModuleHabitat {
            profil: Profil::P2,
            longueur: HAB_LONG,
            spin: a,
            attache,
        };
        // Décalage radial exprimé dans le repère de la grappe, puis tourné avec
        // elle : la couronne reste perpendiculaire à l'épine quel que soit
        // `orient`.
        let radial = orient * (vec3(a.cos(), a.sin(), 0.0) * rayon);
        asm.ajouter(cuire(Repere::new(radial + Vec3::Y * y_centre, orient), &module));
    }
}

/// Vue briques : l'**habitat principal** de l'ISV (fixe, à distinguer des
/// modules d'équipage rotatifs, encore à faire). À gauche un module seul, sans
/// ferrure — coque composite nue à section onigiri, ceinturée de ses trois
/// armatures triangulaires. À droite la grappe de trois telle qu'elle sera
/// posée : coins vers l'extérieur, côtés plats et ferrures tournés vers l'axe,
/// où passera l'épine.
pub fn demo_habitat_isv() -> EtatStation {
    let mut asm = Assembleur::new();
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);

    let seul = Composant::ModuleHabitat {
        profil: Profil::P2,
        longueur: HAB_LONG,
        spin: 0.0,
        attache: 0.0, // présenté seul : pas de ferrure, on juge la coque
    };
    asm.ajouter(cuire(Repere::new(vec3(0.0, 11.0, 0.0), couche), &seul));

    poser_grappe_habitat(&mut asm, 0.0, couche, EPINE);
    asm.terminer()
}

// --- Section d'équipage rotative de l'ISV -----------------------------------
// Gabarit **divisé par deux** le 2026-07-30 : à la taille précédente (bras 9,
// module 7, donc 16 unités de demi-envergure) la section dominait la silhouette
// et venait chevaucher les panaches de propulsion à l'écran.
//
// Le **collier ne suit pas** cette réduction : son rayon extérieur est imposé
// par l'épine qu'il doit envelopper (cf. `EQUIPAGE_ALESAGE`), pas par les
// proportions de la section. Il consomme donc une constante de 2 unités sur la
// demi-envergure, qui passe de 16 à 9 — un peu plus que la moitié.

/// Longueur d'un module d'équipage (du raccord au plancher).
pub(crate) const EQUIPAGE_LONG: f32 = 3.5;
/// Demi-envergure de la traverse : distance axe → raccord d'un module. C'est
/// elle qui fixe le rayon de rotation, donc la gravité obtenue. Le bras utile
/// vaut cette valeur **moins** le rayon du collier.
pub(crate) const EQUIPAGE_BRAS: f32 = 5.5;
/// Longueur du collier de rotation le long de l'épine.
pub(crate) const EQUIPAGE_COLLIER: f32 = 1.4;
/// Rayon extérieur du collier — **réduit de 30 %** le 2026-07-30 (2,0 → 1,4) :
/// le tambour dominait encore le centre de la section.
///
/// Cote **libre et non un cran de `Profil`** : elle se règle contre l'épine, et
/// les crans (1,0 puis 2,0) n'offrent rien entre « plus maigre que la flèche »
/// et « le double ». Sa borne basse est dure — il faut rester **au-dessus** de
/// [`EPINE`] (1,12), sinon les longerons ressortent à travers la jaquette. À 1,4
/// il ne reste que 0,28 de marge : c'est le test
/// `le_collier_dequipage_enveloppe_lepine_sans_jour` qui la garde.
pub(crate) const EQUIPAGE_COLLIER_RAYON: f32 = 1.4;
/// Alésage du collier : le trou par lequel passe l'épine.
///
/// Pris **volontairement plus étroit que l'épine** ([`EPINE`]), donc plus étroit
/// que sa propre section inscrite : les membrures de la flèche mordent alors dans
/// la paroi du tambour et il ne subsiste **aucun jour** entre les deux. On ne voit
/// plus que les surfaces extérieures du collier, et il lit comme solidaire de
/// l'épine.
///
/// Un alésage plus large que l'épine — ce qu'un vrai palier demanderait — laisse
/// au contraire voir l'anneau de vide tout autour, et la pièce a l'air *enfilée*
/// sur la flèche comme une chaussette plutôt que montée dessus. Le rendu a été
/// tranché en faveur de la lecture : c'est la seule cote de la section qui ne
/// décrit pas une mécanique plausible.
pub(crate) const fn equipage_alesage(epine: f32) -> f32 {
    epine * 0.45
}
/// Le même, pour l'épine carrée (valeur historique, et calcul unique).
#[allow(dead_code)] // Mesure de geometrie tenue par ses tests ; aucun dessin ne l'appelle aujourd'hui.
pub(crate) const EQUIPAGE_ALESAGE: f32 = equipage_alesage(EPINE);
/// Gabarit de la charnière de repli (demi-largeur de chape).
pub(crate) const EQUIPAGE_CHARNIERE: f32 = 0.31;
/// Centre de la section d'équipage le long de l'épine (à l'échelle 1) : au-delà
/// de l'habitat, sur le tronçon d'épine encore libre. C'est le bout du vaisseau
/// le plus loin des tuyères, donc celui où l'équipage vit le mieux.
pub(crate) const EQUIPAGE_CENTRE_Y: f32 = 71.0;

/// Pose la **section d'équipage rotative** : une traverse perpendiculaire à
/// l'épine, centrée sur `y_centre`, et un module habité à **chaque bout**,
/// tourné vers l'extérieur.
///
/// Déployée (configuration de croisière, celle qui tourne) plutôt que repliée
/// le long de la coque : c'est la silhouette reconnaissable du vaisseau, et le
/// repli n'a de sens que sous poussée.
///
/// Partagée par la vue Briques et par `preset_isv`, comme les autres grappes :
/// ce qui est validé à l'écran est ce qui part sur le vaisseau.
/// Configuration de la section d'équipage. **C'est un état du vaisseau**, pas
/// un réglage de maquette : il change avec ce que le vaisseau est en train de
/// faire.
///
/// - **Déployée** — en station devant un astre : les bras sont sortis et la
///   section tourne, ce qui donne sa gravité à l'équipage ;
/// - **Repliée** — en transit : bras rabattus le long de la coque. C'est
///   obligatoire sous poussée (une structure sortie encaisserait mal
///   l'accélération) et ça réduit la cible offerte aux poussières à 0,7 c.
///
/// `repli()` donne la valeur continue, pour animer le passage de l'un à
/// l'autre plutôt que de sauter.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum EtatEquipage {
    /// En orbite : bras sortis, section en rotation.
    #[default]
    Deploye,
    /// En transit : bras rabattus le long de la coque.
    Replie,
}

impl EtatEquipage {
    /// Fraction de repli : 0 = déployé, 1 = replié.
    pub fn repli(self) -> f32 {
        match self {
            EtatEquipage::Deploye => 0.0,
            EtatEquipage::Replie => 1.0,
        }
    }
}

/// Direction de pivot du bras `dehors` (dans le repère local de la grappe, où
/// l'épine est +Z) : perpendiculaire au bras **et** à l'épine, donc le
/// tangentiel. Les deux bras tournent autour d'axes opposés et se rabattent
/// donc symétriquement.
pub(crate) fn axe_pivot(dehors: Vec3) -> Vec3 {
    dehors.cross(Vec3::Z).normalize()
}

pub(crate) fn poser_equipage(asm: &mut Assembleur, y_centre: f32, orient: Quat, repli: f32, epine: f32) {
    // **Collier de rotation** au centre : le moyeu qui porte les bras. Son
    // alésage est plus étroit que l'épine (cf. `EQUIPAGE_ALESAGE`), si bien que
    // la flèche traverse la paroi du tambour au lieu de flotter dans un jour —
    // c'est ce qui le fait lire comme solidaire du vaisseau. Son rayon extérieur
    // (P2) est la seule cote de la section calée sur l'épine et non sur les
    // proportions de la section : il faut qu'il **dépasse** le hors-tout de la
    // flèche, sinon les longerons ressortiraient à travers sa jaquette.
    let collier = Composant::CollierRotatif {
        profil: Profil::P2,
        rayon: EQUIPAGE_COLLIER_RAYON,
        alesage: equipage_alesage(epine),
        longueur: EQUIPAGE_COLLIER,
    };
    asm.ajouter(cuire(Repere::new(Vec3::Y * y_centre, orient), &collier));

    // Deux bras **séparés**, partant de la jaquette du collier : une poutre
    // unique de part en part ne passerait pas, l'alésage devant rester vide.
    // Le bras comble ce que le collier ne prend pas : rétrécir le tambour
    // **allonge** le bras d'autant, et la demi-envergure (donc le rayon de
    // rotation) ne bouge pas. C'est `EQUIPAGE_BRAS` qui tient la silhouette.
    let r_collier = EQUIPAGE_COLLIER_RAYON;
    let bras_long = EQUIPAGE_BRAS - r_collier;
    let angle = repli.clamp(0.0, 1.0) * FRAC_PI_2;
    let base = Vec3::Y * y_centre;

    for cote in [1.0_f32, -1.0] {
        let dehors = if cote > 0.0 { Vec3::X } else { Vec3::NEG_X };
        let axe = axe_pivot(dehors);
        // Le bras pivote **autour de la charnière**, donc autour du point où il
        // rejoint la jaquette — pas autour du centre du collier.
        let plie = Quat::from_axis_angle(axe, angle);
        let dir = plie * dehors; // direction du bras une fois replié
        let pivot = dehors * r_collier;

        // Charnière au pivot : son +Z local suit le bras **déployé**, son +X
        // l'axe d'articulation. Sa partie mobile se replie toute seule, du même
        // angle que le bras — les deux lisent donc la même valeur.
        let hx = axe;
        let hz = dehors;
        let hy = hz.cross(hx);
        let rot_charniere = orient * Quat::from_mat3(&Mat3::from_cols(hx, hy, hz));
        let charniere = Composant::Charniere { taille: EQUIPAGE_CHARNIERE, repli };
        asm.ajouter(cuire(Repere::new(orient * pivot + base, rot_charniere), &charniere));

        // Bras en treillis **triangulaire** : trois longerons, c'est la section
        // qui ne se déforme pas — et elle se distingue du carré employé partout
        // ailleurs sur le vaisseau.
        // Section divisée par deux comme le reste : `Profil` étant discret,
        // ça se fait d'un cran (P1 → P0, soit 1,0 → 0,5) et non par un facteur.
        let bras = Composant::Treillis {
            profil: Profil::P0,
            longueur: bras_long,
            style: StyleTreillis::Triangulaire,
        };
        let rot = orient * Quat::from_rotation_arc(Vec3::Z, dir);
        let milieu = orient * (pivot + dir * (bras_long * 0.5)) + base;
        asm.ajouter(cuire(Repere::new(milieu, rot), &bras));

        // Module au bout, axe **vers l'extérieur** (son +Z fuit la charnière).
        let module = Composant::ModuleEquipage {
            profil: Profil::P0,
            longueur: EQUIPAGE_LONG,
            hublots: 8,
        };
        let pos = orient * (pivot + dir * bras_long) + base;
        asm.ajouter(cuire(Repere::new(pos, rot), &module));
    }
}

// --- Boucliers de tête -------------------------------------------------------
/// Rayon (circonradius) de la **petite** plaque de tête.
///
/// Cette cote a une contrainte qu'on ne devine pas en la regardant : c'est elle
/// qui fixe le **plus petit alésage de la pile**, donc la section maximale du
/// mât commun (`rayon × MOYEU × ALESAGE` = 0,396 ici). Un mât plus gros ne
/// passerait pas la petite plaque, quand bien même il passerait les trois
/// grandes — voir [`BOUCLIER_MAT`].
pub(crate) const BOUCLIER_PETIT_RAYON: f32 = 5.5;
/// Rayon de la **grande** plaque, mesuré sur son petit axe. La hauteur vaut
/// `rayon × élancement × 2` (≈ 26) et la largeur `rayon × √3 × étroitesse`
/// (≈ 13,9) : c'est la hauteur qui fait le gabarit, et elle se compare au
/// diamètre du vaisseau aux radiateurs (25,2).
pub(crate) const BOUCLIER_GRAND_RAYON: f32 = 10.0;
/// Nombre de grandes plaques. Le schéma en demande **trois identiques** derrière
/// la petite : c'est un étagement, et un étage ne se lit pas.
pub(crate) const BOUCLIER_GRANDS: usize = 3;
/// Espacement entre plaques le long du mât. C'est **la** cote qui blinde — elle
/// donne au nuage de plasma la place de s'étaler avant la plaque suivante. Sur
/// le vrai ISV c'est ~100 m ; ici on garde un rapport lisible à l'échelle du
/// modèle.
pub(crate) const BOUCLIER_ECART: f32 = 8.0;

/// Profil du **mât** qui enfile les quatre plaques.
///
/// P0 et pas plus gros : sa section transversale mesurée est 0,290, contre un
/// alésage de 0,396 sur la petite plaque. Le cran au-dessus (0,58) ne passerait
/// pas, et l'ouvrir en agrandissant la petite plaque la porterait à un gabarit
/// que le schéma ne montre pas. Élancement du mât ≈ 100:1, exactement celui de
/// la flèche d'épine qu'il prolonge — donc rien d'anormal à l'œil.
pub(crate) const BOUCLIER_MAT: Profil = Profil::P0;

/// Jeu entre la fin du raccord conique et la première plaque : de quoi voir le
/// mât sortir de son col avant qu'une plaque s'y enfile.
pub(crate) const BOUCLIER_JEU: f32 = 0.9;

/// Position de la **première** plaque (la petite) le long de l'axe, au gabarit
/// final : juste après le raccord posé au sommet d'épine.
///
/// **Déduite et non écrite en dur.** Elle valait 95,0 en clair, ce qui marchait
/// tant que l'épine gardait sa longueur — et la laissait en place si on la
/// changeait, la tête restant plantée dans le vide à distance du vaisseau. C'est
/// exactement le défaut que le gabarit d'épine (`Epine::hors_tout`) évite pour
/// la charge utile ; il n'y a pas de raison que la tête y échappe. Trouvé en
/// vérifiant rouge le test de proportions : raccourcir l'épine de moitié ne
/// raccourcissait pas le vaisseau.
pub(crate) const BOUCLIER_DEBUT_Y: f32 = EPINE_SOMMET_Y * ISV_ECHELLE + BOUCLIER_RACCORD + BOUCLIER_JEU;

/// Longueur du **raccord conique** entre le sommet d'épine et le mât.
///
/// Il n'est pas décoratif. La flèche d'épine finit à 0,9 de rayon et le mât fait
/// 0,29 : posés bout à bout, la section chute d'un facteur **trois d'un coup**,
/// et la tête a l'air rapportée plutôt que portée. Le raccord ramène ça à deux
/// marches franches — 0,9 → 1,0 (léger débord, jamais une face coplanaire) puis
/// 1,0 → 0,5 sur sa longueur — après quoi le mât sort de son col.
///
/// C'est aussi ce qui borne `BOUCLIER_DEBUT_Y` par le bas : le col du raccord
/// fait 0,5 de rayon, plus que l'alésage de la petite plaque (0,396). Poser la
/// plaque avant la fin du raccord la ferait empaler dessus.
pub(crate) const BOUCLIER_RACCORD: f32 = 2.4;

/// Pose la **tête de bouclier** : le mât, la petite plaque, puis les trois
/// grandes espacées de [`BOUCLIER_ECART`].
///
/// **À quel bout du vaisseau** : à l'opposé des moteurs, sur le haut d'épine
/// libre. La question traînait depuis §C.8, où nos propres notes se
/// contredisaient (§C.2 plaçait le bouclier « en avant », donc côté moteurs,
/// puisque l'ISV est un tracteur ; §C.4 et §C.6 le mettaient en haut d'épine).
/// Le schéma d'assemblage tranche : il montre les quatre plaques **au bout
/// opposé aux radiateurs**, après la charge utile. C'est aussi le seul bout
/// dégagé — côté moteurs il faudrait composer avec les tuyères et des ailes de
/// rayon 12,6.
///
/// Les plaques sont **perpendiculaires à l'axe**, et leur grand axe est mis dans
/// le **plan des radiateurs** : les deux extrémités du vaisseau se lisent alors
/// dans la même vue de profil, comme sur le schéma. Sans la rotation
/// supplémentaire d'un quart de tour, le grand axe tomberait perpendiculaire aux
/// ailes et la tête paraîtrait plate de trois quarts.
pub(crate) fn poser_tete_bouclier(asm: &mut Assembleur, base_y: f32, axe: Quat) {
    // Un quart de tour autour de l'axe de la plaque : met son grand axe dans le
    // plan des ailes radiateur.
    let pose = axe * Quat::from_rotation_z(FRAC_PI_2);

    let petite = Composant::BouclierPetit { profil: Profil::P1, rayon: BOUCLIER_PETIT_RAYON };
    asm.ajouter(cuire(Repere::new(vec3(0.0, base_y, 0.0), pose), &petite));

    let grande = Composant::BouclierGrand {
        profil: Profil::P1,
        rayon: BOUCLIER_GRAND_RAYON,
        elancement: BOUCLIER_ELANCEMENT,
    };
    for k in 1..=BOUCLIER_GRANDS {
        let y = base_y + k as f32 * BOUCLIER_ECART;
        asm.ajouter(cuire(Repere::new(vec3(0.0, y, 0.0), pose), &grande));
    }

    // Raccord conique au sommet d'épine, grand bout (−Z local) contre elle.
    let pied = EPINE_SOMMET_Y * ISV_ECHELLE;
    let raccord = Composant::Adaptateur {
        grand: Profil::P1,
        petit: BOUCLIER_MAT,
        longueur: BOUCLIER_RACCORD,
    };
    asm.ajouter(cuire(
        Repere::new(vec3(0.0, pied + BOUCLIER_RACCORD * 0.5, 0.0), axe),
        &raccord,
    ));

    // Mât : il part du **sommet d'épine** et non de la première plaque — sinon
    // la tête flotterait devant le vaisseau sans rien qui l'y rattache — et
    // dépasse d'un demi-écart au-delà de la dernière, comme un vrai longeron
    // qu'on ne coupe pas à ras de sa dernière ferrure. Sa première section est
    // noyée dans le raccord, qui est plein : le mât en **sort**, il n'y est pas
    // simplement accolé.
    let bout = base_y + BOUCLIER_GRANDS as f32 * BOUCLIER_ECART + BOUCLIER_ECART * 0.5;
    let mat = Composant::Treillis {
        profil: BOUCLIER_MAT,
        longueur: bout - pied,
        style: StyleTreillis::Triangulaire,
    };
    asm.ajouter(cuire(Repere::new(vec3(0.0, (pied + bout) * 0.5, 0.0), axe), &mat));
}

// --- Bouclier thermique d'épine ---------------------------------------------
// Le bardage se monte **au droit des moteurs**, et s'arrête peu après les
// tuyères : c'est là qu'est la chaleur. Il ne couvre pas le long tronçon nu, qui
// n'a rien à parer et que le bardage ferait lire comme une gaine technique.
//
// Toutes les cotes ci-dessous sont relevées sur l'épine assemblée, dont le rayon
// hors-tout décroît ainsi le long de l'axe : 3,75 à X = −12, 2,81 à −9, 2,28 à
// −6, 1,84 à −3, 1,53 à 0, 1,32 à +3, puis 1,15 constant au-delà de +12.

/// Début du bardage : au pied des tuyères, qui s'étendent de −9,8 à −1,3.
///
/// **Pas plus bas**, et la raison est mesurée : entre −12 et −10,5 le rayon de
/// l'épine tombe de 3,75 à 2,85 d'un seul coup, puis se remet à décroître
/// doucement. Aucune loi en puissance ne suit ce décrochement, et un bardage qui
/// part de −12 flotte de **0,93** au-dessus de la poutre trois unités plus loin
/// (mesuré). À partir de −9 le profil est régulier et l'exposant 2,0 le suit à
/// deux centièmes près.
pub(crate) const THERMIQUE_DEBUT_Y: f32 = -9.0;
/// Fin du bardage. Les tuyères s'arrêtent à −1,3 : le bardage les dépasse de
/// cinq unités et s'arrête là. Au-delà, il n'y a plus rien à parer.
pub(crate) const THERMIQUE_FIN_Y: f32 = 4.0;
/// Rayon au **pied** (côté moteurs), pris au **circonradius** comme celui de
/// l'épine (2,81 au même endroit).
///
/// ⚠️ Il ne suffit pas de dépasser 2,81. Les deux hexagones — celui du bardage
/// et celui du treillis — n'ont **aucune raison d'être calés sur la même
/// orientation** : le bardage a son repère propre, l'épine tient le sien de
/// `repere(axe)`. Dans le pire cas ils sont décalés d'un demi-pas, et c'est
/// alors le **milieu de facette** du bardage (son rayon inscrit, 0,866 × le
/// circonradius) qui passe au droit d'un **longeron** de l'épine. C'est ce
/// pincement qui se voyait à l'écran, alors même que les circonradius
/// mesuraient un jeu positif partout.
///
/// Les rayons sont donc dimensionnés sur le **cas le plus défavorable** :
/// `circonradius_bardage × 0,866 ≥ circonradius_épine + jeu`. D'où le facteur
/// 1,155 appliqué aux cotes relevées — c'est le prix de ne pas dépendre d'un
/// calage angulaire que rien ne garantit.
pub(crate) const THERMIQUE_RAYON_PIED: f32 = 3.50;
/// Rayon au **bout**, au circonradius. L'épine y mesure ≈ 1,28.
pub(crate) const THERMIQUE_RAYON_BOUT: f32 = 1.70;
/// Courbure de l'évasement, ajustée pour que le bardage **épouse** l'épine.
///
/// Elle est réglée par la mesure et non par le calcul : la loi du treillis est
/// écrite depuis la base de la charpente, alors que le bardage commence dix
/// unités plus haut, et le pied en pavillon s'ajoute par-dessus. Refaire
/// l'algèbre donnerait une formule juste pour une pièce qui n'est pas tout à
/// fait celle qu'on habille.
///
/// Ajustée sur les rayons relevés **majorés du facteur de calage** (voir
/// [`THERMIQUE_RAYON_PIED`]), elle tombe à **1,7 ± 0,05** aux trois tranches du
/// milieu : le profil de l'épine *est* une loi en puissance sur cette portion,
/// ce qui n'allait pas de soi. Le jeu résultant, mesuré au pire cas, tient entre
/// **+0,05 et +0,09** sur toute l'emprise — vérifié par
/// `le_bardage_thermique_epouse_lepine`.
pub(crate) const THERMIQUE_COURBURE: f32 = 1.5;
/// Nombre de rangs d'écailles. Réglé pour que le pas (≈ 1,3) reste inférieur à
/// la largeur d'une facette, y compris au bout où elle n'est plus que de 1,4.
pub(crate) const THERMIQUE_RANGS: usize = 10;

/// Le bardage tel qu'il est monté sur l'ISV. Sorti en fonction pour que la
/// vitrine et le vaisseau montrent **la même pièce** : c'est tout l'intérêt de
/// la méthode brique-d'abord, et une vitrine qui diverge du montage ne valide
/// plus rien.
pub(crate) fn bouclier_thermique() -> Composant {
    Composant::BouclierThermique {
        rayon_pied: THERMIQUE_RAYON_PIED,
        rayon_bout: THERMIQUE_RAYON_BOUT,
        courbure: THERMIQUE_COURBURE,
        longueur: THERMIQUE_FIN_Y - THERMIQUE_DEBUT_Y,
        rangs: THERMIQUE_RANGS,
    }
}

/// Vue briques : le **bouclier thermique d'épine** — le bardage d'écailles
/// imbriquées qui pare le rayonnement des tuyères.
///
/// En haut un tronçon court, pour juger une écaille : sa saillie, sa lèvre
/// sombre, et le sens du recouvrement. En dessous le tronçon tel qu'il est monté
/// sur le vaisseau, enfilé sur un bout d'épine hexagonale — c'est le seul moyen
/// de vérifier ce qui compte vraiment, à savoir que le bardage **plaque** sur la
/// poutre au lieu de flotter autour.
pub fn demo_bouclier_thermique() -> EtatStation {
    let mut asm = Assembleur::new();
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);

    let court = Composant::BouclierThermique {
        rayon_pied: 3.0,
        rayon_bout: 2.6,
        courbure: THERMIQUE_COURBURE,
        longueur: 9.0,
        rangs: 4,
    };
    asm.ajouter(cuire(Repere::new(vec3(-4.5, 12.0, 0.0), couche), &court));

    // Monté sur son épine : bardage et poutre au même endroit et au même
    // gabarit, sans quoi on ne juge que le bardage seul — et ce qui compte est
    // qu'il **épouse** l'évasement au lieu de flotter dessus.
    let long = THERMIQUE_FIN_Y - THERMIQUE_DEBUT_Y;
    let epine = Composant::CharpenteHexa {
        grand: Profil::P3,
        petit: Profil::P1,
        longueur: long * 1.6,
        courbure: 2.6,
        pied: PiedHexa::Aucun,
    };
    asm.ajouter(cuire(Repere::new(vec3(0.0, -6.0, 0.0), couche), &epine));
    asm.ajouter(cuire(
        Repere::new(vec3(-long * 0.8, -6.0, 0.0), couche),
        &bouclier_thermique(),
    ));

    asm.terminer()
}

/// Vue briques : la **petite plaque de tête**, présentée sur ses deux faces.
///
/// À gauche la **face avant** (+Z, côté poussière), striée ; à droite la même
/// plaque retournée, donc sa **face arrière** et son ossature. Les deux côte à
/// côte dans le même plan : c'est la seule mise en scène qui laisse juger les
/// deux faces d'un même point de vue, comme sur le schéma. En bas la plaque par
/// la **tranche**, pour voir l'épaisseur et le débord des nervures.
///
/// **Pas encore posée sur l'ISV** : brique d'abord, assemblage ensuite.
pub fn demo_bouclier_petit() -> EtatStation {
    let mut asm = Assembleur::new();
    let plaque = Composant::BouclierPetit { profil: Profil::P1, rayon: BOUCLIER_PETIT_RAYON };
    let ecart = BOUCLIER_PETIT_RAYON * 1.35;

    asm.ajouter(cuire(Repere::new(vec3(-ecart, 3.0, 0.0), Quat::IDENTITY), &plaque));
    asm.ajouter(cuire(
        Repere::new(vec3(ecart, 3.0, 0.0), Quat::from_rotation_y(std::f32::consts::PI)),
        &plaque,
    ));
    asm.ajouter(cuire(
        Repere::new(
            vec3(0.0, -BOUCLIER_PETIT_RAYON * 1.6, 0.0),
            Quat::from_rotation_arc(Vec3::Z, Vec3::X),
        ),
        &plaque,
    ));

    asm.terminer()
}

/// Vue briques : la **grande plaque de tête** — l'hexagone étiré, miroir bleuté
/// uni sur ses deux faces, portant le motif du schéma : huit rayons partant du
/// moyeu et deux barres transversales qui détachent les deux pointes.
///
/// À gauche de face, à droite par la tranche : c'est de profil que se lit le
/// nœud papillon des nervures, épaisses au moyeu et effilées à la jante. En bas
/// les **trois** plaques telles qu'elles seront enfilées sur le mât, pour juger
/// l'espacement — qui est ce qui blinde, bien plus que l'épaisseur des plaques.
///
/// Le mât est le même que sur le vaisseau ([`BOUCLIER_MAT`]) : c'est la petite
/// plaque, et non les grandes, qui borne sa section.
pub fn demo_bouclier_grand() -> EtatStation {
    let mut asm = Assembleur::new();
    let plaque = Composant::BouclierGrand {
        profil: Profil::P1,
        rayon: BOUCLIER_GRAND_RAYON,
        elancement: BOUCLIER_ELANCEMENT,
    };
    let haut = BOUCLIER_GRAND_RAYON * BOUCLIER_ELANCEMENT;

    asm.ajouter(cuire(
        Repere::new(vec3(-BOUCLIER_GRAND_RAYON * 1.3, haut * 1.15, 0.0), Quat::IDENTITY),
        &plaque,
    ));
    asm.ajouter(cuire(
        Repere::new(
            vec3(BOUCLIER_GRAND_RAYON * 1.6, haut * 1.15, 0.0),
            Quat::from_rotation_arc(Vec3::Z, Vec3::X),
        ),
        &plaque,
    ));

    // La pile, couchée le long de +X : les trois plaques enfilées, plus le mât
    // qui les traverse.
    let base = vec3(-BOUCLIER_ECART, -haut * 1.05, 0.0);
    let couche = Quat::from_rotation_arc(Vec3::Z, Vec3::X);
    for k in 0..BOUCLIER_GRANDS {
        asm.ajouter(cuire(
            Repere::new(base + Vec3::X * (k as f32 * BOUCLIER_ECART), couche),
            &plaque,
        ));
    }
    let mat = Composant::Treillis {
        profil: BOUCLIER_MAT,
        longueur: (BOUCLIER_GRANDS - 1) as f32 * BOUCLIER_ECART + BOUCLIER_ECART * 0.5,
        style: StyleTreillis::Triangulaire,
    };
    asm.ajouter(cuire(
        Repere::new(base - Vec3::X * (BOUCLIER_ECART * 0.25), couche),
        &mat,
    ));

    asm.terminer()
}

/// Vue briques : la **section d'équipage rotative** de l'ISV — la seule partie
/// tournante du vaisseau. En haut un module seul (fût cylindrique, plancher
/// bombé vers l'extérieur, couronne de hublots au niveau du pont) ; en dessous
/// l'ensemble déployé : traverse et un module à chaque bout, tournés vers
/// l'extérieur, tel qu'il sera posé sur l'épine.
pub fn demo_equipage(repli: f32) -> EtatStation {
    let mut asm = Assembleur::new();

    let seul = Composant::ModuleEquipage { profil: Profil::P0, longueur: EQUIPAGE_LONG, hublots: 8 };
    asm.ajouter(cuire(Repere::new(vec3(0.0, 7.0, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::X)), &seul));

    // Orientée **comme sur le vaisseau** (collier le long de Y, l'axe de
    // l'épine), et pas à plat : c'est ce qui fait que l'axe de rotation de la
    // vue est le même ici et là-bas — sinon le bouton ferait tourner la
    // maquette autour du mauvais axe.
    poser_equipage(&mut asm, 0.0, Quat::from_rotation_arc(Vec3::Z, Vec3::Y), repli, EPINE);
    asm.terminer()
}

/// Vue briques : prototype de **réservoir de carburant** cylindrique (calottes
/// sphériques), cerclé de 4 barres métal en position tétraédrique.
pub fn demo_reservoir() -> EtatStation {
    let mut asm = Assembleur::new();
    let res = Composant::Reservoir { profil: Profil::P2, longueur: 6.0, cage: false };
    asm.ajouter(cuire(Repere::IDENTITE, &res));
    asm.terminer()
}

/// Vue briques : **moteur à antimatière** — tuyère (silhouette VASIMR gonflée)
/// surmontée du **bloc réacteur/injection** (cuve sombre, bobines EM, tuyauterie)
/// branché sur son écoutille +Z. Poussée le long de −Z.
pub fn demo_moteur_antimatiere_principal() -> EtatStation {
    let mut asm = Assembleur::new();
    let t = 6.0;
    let moteur = Composant::MoteurAntimatiere { profil: Profil::P2, taille: t };
    let mm = Repere::IDENTITE;
    asm.ajouter(cuire(mm, &moteur));
    // Bloc réacteur/injection clipsé sur l'écoutille axiale +Z de la tuyère.
    let reacteur = Composant::ReacteurAntimatiere { profil: Profil::P2, taille: t };
    porter_vers(&mut asm, mm, &moteur, Vec3::Z, &reacteur, 0);
    asm.terminer()
}

