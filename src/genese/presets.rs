use super::apparences::{gazeuse, tellurique};
use super::{
    ajouter_lune, ajouter_lune_reelle, ajouter_planete, ajouter_planete_autour, app_simple,
    deployer_arbre, masse_luminosite, preset_gazeuse, preset_tellurique, MASSE_ETOILE,
};
use crate::astre::Foyer;
use crate::stellaire::{Feuille, Noeud, Variante};
use crate::disque::{Disque, DisqueConfig};
use crate::etoile;
use crate::planete::{Apparence, Planete, TypePlanete};
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;
use macroquad::rand::srand;

/// **Facteur global** applique aux rayons du Soleil, des planetes et des lunes.
///
/// `1.0` = echelle vraie. Le monter grossit les corps **sans toucher aux
/// distances** : c'est le compromis habituel pour rendre un systeme lisible, et
/// il est ici en UN seul endroit au lieu d'etre saupoudre dans chaque appel.
///
/// ATTENTION : au-dela d'une cinquantaine, une planete rattrape l'orbite de ses
/// lunes (la Lune n'est qu'a 60 rayons terrestres). `les_lunes_orbitent_hors_de_
/// leur_planete` garde cette limite.
pub(crate) const ECHELLE_CORPS: f32 = 5.0;

/// Kilometres par unite astronomique.
const KM_PAR_UA: f32 = 149_597_870.0;

/// Distance heliocentrique en unites de monde, depuis des kilometres reels.
/// **Non affectee par [`ECHELLE_CORPS`]** : les distances au Soleil restent vraies.
pub(crate) fn distance_monde(km: f32) -> f32 {
    km / KM_PAR_UA * etoile::UA
}

/// Distance d'une lune a sa planete, **magnifiee comme les corps**.
///
/// L'echelle grossit chaque planete AVEC son cortege : sans cela, grossir Mars
/// de cinq l'amene a avaler Phobos, qui n'orbite qu'a 2,76 rayons martiens. Le
/// plafond utile de `ECHELLE_CORPS` serait alors 1,15 -- autant dire aucun.
///
/// Seules les distances au Soleil restent vraies. C'est le compromis classique :
/// un systeme planete-lunes lisible, pose aux bonnes distances heliocentriques.
pub(crate) fn distance_lune(km: f32) -> f32 {
    distance_monde(km) * ECHELLE_CORPS
}

/// Lunes : `(nom, rayon en rayons terrestres, demi-grand axe en km, retrograde)`.
///
/// Les demi-grands axes sont les **vrais**. Le placement procedural de
/// `poser_lune` les repartissait entre la limite de Roche et 14 % de la distance
/// a l'etoile : commode pour un systeme invente, faux pour le notre.
pub(crate) const LUNES: [(&str, f32, f32, bool); 16] = [
    ("Lune", 0.2727, 384_400.0, false),
    ("Phobos", 0.00177, 9_376.0, false),
    ("Deimos", 0.00097, 23_463.0, false),
    ("Io", 0.2858, 421_700.0, false),
    ("Europe", 0.2450, 671_034.0, false),
    ("Ganymede", 0.4134, 1_070_412.0, false),
    ("Callisto", 0.3783, 1_882_709.0, false),
    ("Encelade", 0.0396, 237_948.0, false),
    ("Rhea", 0.1199, 527_108.0, false),
    ("Titan", 0.4042, 1_221_870.0, false),
    ("Japet", 0.1154, 3_560_820.0, false),
    ("Ariel", 0.0909, 190_900.0, false),
    ("Umbriel", 0.0918, 266_000.0, false),
    ("Titania", 0.1238, 435_910.0, false),
    ("Oberon", 0.1195, 583_520.0, false),
    // Triton est **retrograde** : capture, pas formation in situ.
    ("Triton", 0.2124, 354_759.0, true),
];

/// Masses des planetes qui portent des lunes, en masses terrestres.
///
/// Elles ne servent qu'a la 3e loi de Kepler : Io et la Lune sont a peu pres a
/// la meme distance de leur planete, mais Io boucle en 1,8 jour contre 27,3 --
/// parce que Jupiter pese 318 Terres. Sans les masses, toutes les lunes
/// tourneraient a la meme vitesse et la difference se verrait.
pub(crate) const MASSES_TERRESTRES: [(&str, f32); 6] = [
    ("Terre", 1.0),
    ("Mars", 0.107),
    ("Jupiter", 317.8),
    ("Saturne", 95.16),
    ("Uranus", 14.54),
    ("Neptune", 17.15),
];

/// Masse d'une planete en masses terrestres, lue dans [`MASSES_TERRESTRES`].
///
/// Pendant de [`rayon_monde`]. Sans elle, la table etait **morte** et chaque
/// masse etait recopiee a la main dans le preset ET dans les tests : la table
/// annoncait une source unique qu'elle n'etait pas, et corriger une masse a un
/// endroit laissait les deux autres mentir.
pub(crate) fn masse_monde(nom: &str) -> f32 {
    MASSES_TERRESTRES
        .iter()
        .find(|(n, _)| *n == nom)
        .unwrap_or_else(|| panic!("masse inconnue : {nom}"))
        .1
}

/// Vitesse angulaire d'une lune, par la 3e loi de Kepler.
///
/// La constante cale la periode de **notre** Lune sur 27,32 jours pour une annee
/// terrestre de 365,25 : tout le reste en decoule. Verification : Io ressort a
/// 1,76 jour, la vraie valeur est 1,769.
pub(crate) fn omega_lune(masse_parent: f32, r_orbite: f32, retrograde: bool) -> f32 {
    const K: f32 = 0.003035;
    let w = (K * crate::systeme::G * masse_parent / (r_orbite * r_orbite * r_orbite)).sqrt();
    if retrograde { -w } else { w }
}

/// Rayon terrestre en **unites astronomiques** : 6 371 km sur 149 597 870.
const RAYON_TERRE_UA: f32 = 4.2588e-5;

/// Rayon terrestre en unites de monde, **a l'echelle vraie**.
///
/// Plus aucune exageration : `UA` vaut 48 unites, donc la Terre mesure 0,002
/// unite. C'est minuscule -- et c'est exact. Une planete n'occupe quelques
/// pixels qu'une fois la camera vraiment approchee ; vue depuis l'orbite de
/// Neptune, le systeme interieur est un point. C'est le systeme solaire.
pub(crate) const R_TERRE: f32 = RAYON_TERRE_UA * etoile::UA;

/// Rayon du Soleil : 109,2 rayons terrestres.
///
/// Il valait 2,0 unites, soit **mille fois** la Terre au lieu de cent-neuf. A
/// l'echelle vraie il doit retrecir comme le reste, sinon les rapports
/// redeviennent faux la ou ils se voient le plus.
pub(crate) const R_SOLEIL: f32 = R_TERRE * 109.2 * ECHELLE_CORPS;

/// Rayons equatoriaux des planetes, en rayons terrestres (valeurs UAI).
///
/// **Source unique** des tailles. Avant, chaque rayon etait ecrit a la main dans
/// l'appel : Jupiter valait 3,1 fois la Terre au lieu de 11, Pluton trois fois
/// trop gros. Les rapports etaient faux et rien ne le disait.
pub(crate) const RAYONS_TERRESTRES: [(&str, f32); 9] = [
    ("Mercure", 0.3829),
    ("Venus", 0.9499),
    ("Terre", 1.0),
    ("Mars", 0.5320),
    ("Jupiter", 10.973),
    ("Saturne", 9.140),
    ("Uranus", 3.981),
    ("Neptune", 3.865),
    ("Pluton", 0.1868),
];

/// Rayon d'un corps en unites de monde, deduit des tables.
///
/// **A utiliser partout**, y compris pour l'argument `rayon_planete` de
/// `ajouter_lune_preset` : il sert a calculer la limite de Roche, et un rayon
/// recopie a la main y poserait les lunes **dans** la planete le jour ou la
/// taille change.
pub(crate) fn rayon_monde(nom: &str) -> f32 {
    let r = RAYONS_TERRESTRES
        .iter()
        .copied()
        .chain(LUNES.iter().map(|(n, r, _, _)| (*n, *r)))
        .find(|(n, _)| *n == nom)
        .unwrap_or_else(|| panic!("rayon inconnu : {nom}"))
        .1;
    R_TERRE * r * ECHELLE_CORPS
}

/// Graine de géographie **déduite du nom du corps**.
///
/// Délègue à [`super::graine_de_nom`], la même fonction que celle qui donne
/// maintenant leur graine aux entrées du catalogue — une seule source.
///
/// Ne sert plus qu'aux corps qui **partagent** une apparence de catalogue :
/// Callisto et Obéron tirent tous deux « Lune », et sans cela ils seraient
/// rigoureusement jumeaux. Les corps à preset unique gardent, eux, la graine du
/// catalogue : ils sont donc **identiques à leur vignette de galerie**, ce qui
/// est tout l'objet de la manoeuvre.
fn graine_nom(nom: &str) -> f32 {
    super::graine_de_nom(nom)
}

/// **Périodes de rotation sidérale**, en jours terrestres. Négatif = rétrograde.
///
/// Vénus et Uranus tournent bel et bien à l'envers — c'est une singularité
/// qu'on peut voir, et qui ne coûte qu'un signe.
pub(crate) const ROTATIONS_JOURS: [(&str, f32); 9] = [
    ("Mercure", 58.646),
    ("Venus", -243.025),
    ("Terre", 1.0),
    ("Mars", 1.026),
    ("Jupiter", 0.4135),
    ("Saturne", 0.4440),
    ("Uranus", -0.7183),
    ("Neptune", 0.6713),
    ("Pluton", -6.387),
];

/// Vitesse de rotation propre (rad/s de jeu) pour une période en jours.
///
/// La Terre sert de référence : un tour en ~10 min de temps réel. Le reste suit
/// le **rapport des vraies périodes** — Jupiter boucle en 10 h, donc 2,4 fois
/// plus vite qu'ici.
///
/// ⚠️ **Bornée.** À l'échelle littérale, Vénus mettrait 42 heures de jeu pour un
/// tour : indiscernable d'une planète figée, ce qui se lit comme un bug plutôt
/// que comme un fait. Le plancher la garde très lente sans la bloquer, et le
/// plafond évite qu'un rotateur rapide ne devienne une toupie.
pub(crate) fn vitesse_rotation(jours: f32) -> f32 {
    const BASE: f32 = 0.01; // Terre
    const RALENTI_MAX: f32 = 40.0;
    const ACCEL_MAX: f32 = 4.0;
    if jours == 0.0 {
        return BASE;
    }
    let v = BASE / jours.abs();
    v.clamp(BASE / RALENTI_MAX, BASE * ACCEL_MAX) * jours.signum()
}

/// Apparence du catalogue, **géographie figée** par le nom du corps.
///
/// C'est aussi le point où les **données réelles** propres à un corps se posent
/// — comme `rayon_monde` le fait pour les tailles. La rotation en fait partie :
/// elle était codée en dur dans le shader, donc identique pour tout le monde.
fn fige(mut app: Apparence, corps: &str) -> Apparence {
    app.seed = graine_nom(corps);
    if let Some((_, j)) = ROTATIONS_JOURS.iter().find(|(n, _)| *n == corps) {
        app.rot_vitesse = vitesse_rotation(*j);
    }
    app
}

/// Preset reproduisant notre système solaire (Mercure → Pluton + ceintures).
/// Les quatre familles de petits corps du système solaire, du plus proche au
/// plus lointain. Toutes sont déjà modélisées par `DisqueConfig` ; les distances
/// sont **réelles** (UA), seuls les effectifs sont réduits — un nuage de Oort
/// compte ~10¹² corps.
///
/// **Source unique** : le preset les instancie, et le test de budget
/// (`disque::tests`) mesure ces mêmes configs. Un effectif gonflé ici fait
/// bouger le budget là-bas, au lieu de passer inaperçu jusqu'à ce que ça rame.
pub fn ceintures_solaires() -> Vec<DisqueConfig> {
    // Le Soleil a une luminosite de 1.0 (voir `construire_preset_solaire`) :
    // `masse_luminosite(1.0) == MASSE_ETOILE` exactement, donc ce calcul ne
    // change rien aux configs d'aujourd'hui -- il leur donne juste une source
    // commune avec `feuille_alea` au lieu de la constante recopiee telle quelle.
    let masse = masse_luminosite(1.0);
    vec![
        DisqueConfig::asteroides(900, 2.2 * etoile::UA, 3.3 * etoile::UA, masse),
        DisqueConfig::kuiper(2000, 30.0 * etoile::UA, 48.0 * etoile::UA, masse),
        // Disque epars : au-dela de Kuiper, orbites excentriques et tres
        // inclinees (c'est de la que viennent les cometes de la famille de
        // Jupiter). Il s'etend reellement jusque vers 1 000 UA -- Sedna monte a
        // 937 a l'aphelie.
        DisqueConfig::epars(900, 48.0 * etoile::UA, 1000.0 * etoile::UA, masse),
        // Nuage de Oort : coquille **spherique**, pas un disque -- il enveloppe
        // le systeme au lieu de le ceinturer.
        //
        // Distances REELLES depuis le 2026-08-05 : le nuage de Hills commence
        // vers 2 000 UA. Le bord interne etait ramene a 300 pour rester
        // cadrable ; c'est desormais la vraie valeur, au prix d'un zoom bien
        // plus large pour l'apercevoir. Le nuage externe (jusqu'a ~100 000 UA)
        // reste tronque a 20 000 : au-dela, meme le systeme entier tient dans
        // un pixel.
        DisqueConfig::oort(2600, 2000.0 * etoile::UA, 20000.0 * etoile::UA, masse),
    ]
}

pub fn construire_preset_solaire() -> (Systeme, String) {
    srand(42);
    let deg = std::f32::consts::PI / 180.0;

    let mut sys = Systeme::new();
    let soleil = sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        R_SOLEIL,
        etoile::couleur_corps_noir(5800.0),
        1.0,
    )));
    sys.nommer(soleil, "Soleil");
    // Masse du Soleil : derivee de SA luminosite (1.0 ci-dessus, pas recopiee)
    // -- vaut exactement MASSE_ETOILE, donc les periodes des neuf planetes ne
    // bougent pas.
    let masse_soleil = masse_luminosite(1.0);

    // Apparences tirees des catalogues (source unique avec la galerie) ; on conserve
    // les distances (UA reelles), excentricites, inclinaisons, rayons et masses.
    //
    // ATTENTION : les noms passes a `preset_*` sont ceux d'une **apparence**,
    // pas d'un astre -- "Lune" sert a la fois a notre Lune et a Callisto. Le nom
    // propre se pose a part, par `nommer` : il vivait jusqu'ici en commentaire,
    // donc nulle part pour le programme (`conception/interface.md` 2.2a).
    //
    // `fige(..., "<corps>")` arrete la geographie sur une graine deduite du nom :
    // sans elle, chaque lancement redessine les continents (voir `graine_nom`).
    //
    // Appariements revus le 2026-08-05, pour cesser de recycler le meme preset
    // sur des corps que rien ne rapproche :
    //   Mars     : "Badlands" -> "Mars"      preset propre, rouille plus franche
    //   Ganymede : "Lune" -> "Crevasse"      ses sillons (sulci) sont sa signature
    //   Callisto : "Carbone" -> "Lune"       le corps le plus cratelise connu
    //   Ariel    : "Boule de neige" -> "Supraglacial"  terrains fractures brillants
    //   Pluton   : "Boule de neige" -> "Ice Dunes"     New Horizons y a vu des dunes
    // Japet garde "Carbone" (son hemisphere sombre) et n'est donc plus un doublon.
    // Une lune, a sa VRAIE distance et sa vraie taille. `i` sert seulement a
    // decaler les phases pour que les lunes ne s'alignent pas au demarrage.
    //
    // `parent` porte l'indice ET le nom : la masse se **lit dans la table**
    // (`masse_monde`) au lieu d'etre recopiee a chaque appel. Une masse ecrite a
    // la main ici, une autre dans la table, une troisieme dans les tests : les
    // trois derivaient sans que rien ne rougisse.
    let poser = |sys: &mut Systeme, parent: (usize, &str), nom: &'static str,
                     i: usize, app: Apparence| {
        let (idx, parent_nom) = parent;
        let (_, _, a_km, retro) = LUNES.iter().find(|(n, _, _, _)| *n == nom).expect("lune inconnue");
        let r = distance_lune(*a_km);
        let phase = i as f32 * 1.7;
        ajouter_lune_reelle(
            sys, idx, rayon_monde(nom), r,
            omega_lune(masse_monde(parent_nom), r, *retro),
            (i as f32 * 0.031) - 0.05, phase, app, nom,
        );
    };
    // Rayons pris a la table : une seule source, rapports exacts.
    let rayon_mars = rayon_monde("Mars");
    let rayon_jupiter = rayon_monde("Jupiter");
    let rayon_saturne = rayon_monde("Saturne");
    let rayon_uranus = rayon_monde("Uranus");
    let rayon_neptune = rayon_monde("Neptune");
    // Mercure, Venus et Pluton ne portent pas de lune : leur indice ne sert a
    // rien, on ne le nomme pas.
    ajouter_planete(&mut sys, masse_soleil, 0.39, 0.205, 7.0 * deg, rayon_monde("Mercure"), 0.3,
        preset_tellurique("Fer (Mercure)"), Some("Mercure"));
    ajouter_planete(&mut sys, masse_soleil, 0.72, 0.007, 3.4 * deg, rayon_monde("Venus"), 1.0,
        preset_tellurique("Venus (etuve)"), Some("Venus"));
    // Rayon de la Terre : nomme parce que l'ISS s'en sert pour son orbite et son
    // envergure. Recopie, il ferait deriver la station le jour ou la planete
    // change de taille.
    let rayon_terre = rayon_monde("Terre");
    let terre = ajouter_planete(&mut sys, masse_soleil, 1.0, 0.017, 0.0, rayon_terre, 1.0,
        preset_tellurique("Terre"), Some("Terre"));
    poser(&mut sys, (terre, "Terre"), "Lune", 0, fige(preset_tellurique("Lune"), "Lune"));
    let mars = ajouter_planete(&mut sys, masse_soleil, 1.52, 0.093, 1.85 * deg, rayon_mars, 0.4,
        preset_tellurique("Mars"), Some("Mars"));
    // Phobos + Deimos : deux petits corps sombres captures.
    poser(&mut sys, (mars, "Mars"), "Phobos", 0, fige(preset_tellurique("Carbone"), "Phobos"));
    poser(&mut sys, (mars, "Mars"), "Deimos", 1, fige(preset_tellurique("Carbone"), "Deimos"));
    let jupiter = ajouter_planete(&mut sys, masse_soleil, 5.2, 0.049, 1.3 * deg, rayon_jupiter, 20.0,
        preset_gazeuse("Jupiter"), Some("Jupiter"));
    // 4 lunes galileennes (interne -> externe), aspects distincts.
    poser(&mut sys, (jupiter, "Jupiter"), "Io", 0, preset_tellurique("Io (soufre)"));
    poser(&mut sys, (jupiter, "Jupiter"), "Europe", 1, preset_tellurique("Subglaciaire"));
    poser(&mut sys, (jupiter, "Jupiter"), "Ganymede", 2, preset_tellurique("Crevasse"));
    poser(&mut sys, (jupiter, "Jupiter"), "Callisto", 3, fige(preset_tellurique("Lune"), "Callisto"));
    let saturne = ajouter_planete(&mut sys, masse_soleil, 9.58, 0.056, 2.49 * deg, rayon_saturne, 12.0,
        preset_gazeuse("Saturne"), Some("Saturne"));
    // Encelade, Rhea, Titan (le plus gros), Japet (interne -> externe).
    poser(&mut sys, (saturne, "Saturne"), "Encelade", 0, preset_tellurique("Pics de glace"));
    poser(&mut sys, (saturne, "Saturne"), "Rhea", 1, preset_tellurique("Boule de neige"));
    poser(&mut sys, (saturne, "Saturne"), "Titan", 2, preset_tellurique("Titan"));
    poser(&mut sys, (saturne, "Saturne"), "Japet", 3, fige(preset_tellurique("Carbone"), "Japet"));
    let uranus = ajouter_planete(&mut sys, masse_soleil, 19.2, 0.046, 0.77 * deg, rayon_uranus, 6.0,
        preset_gazeuse("Uranus"), Some("Uranus"));
    // Ariel, Umbriel, Titania, Oberon (interne -> externe).
    poser(&mut sys, (uranus, "Uranus"), "Ariel", 0, preset_tellurique("Supraglacial"));
    poser(&mut sys, (uranus, "Uranus"), "Umbriel", 1, fige(preset_tellurique("Carbone"), "Umbriel"));
    poser(&mut sys, (uranus, "Uranus"), "Titania", 2, fige(preset_tellurique("Subglaciaire"), "Titania"));
    poser(&mut sys, (uranus, "Uranus"), "Oberon", 3, fige(preset_tellurique("Lune"), "Oberon"));
    let neptune = ajouter_planete(&mut sys, masse_soleil, 30.05, 0.010, 1.77 * deg, rayon_neptune, 6.0,
        preset_gazeuse("Neptune"), Some("Neptune"));
    poser(&mut sys, (neptune, "Neptune"), "Triton", 0, preset_tellurique("Cryovolcan"));
    ajouter_planete(&mut sys, masse_soleil, 39.5, 0.249, 17.1 * deg, rayon_monde("Pluton"), 0.1,
        preset_tellurique("Ice Dunes"), Some("Pluton")); // nain glace

    // ===== Population de petits corps, du plus proche au plus lointain =====
    // Quatre familles distinctes, toutes deja modelisees par `DisqueConfig` mais
    // dont ce preset n'utilisait que les deux premieres. Les distances sont
    // reelles (UA) ; seuls les effectifs sont reduits, un nuage de Oort compte
    // ~10^12 corps.
    for cfg in ceintures_solaires() {
        sys.ajouter(Box::new(Disque::new(cfg)));
    }

    // ===== L'ISS, en orbite terrestre =====
    // Le pont entre les deux moities du projet : la station est **assemblee**
    // par `vaisseau::preset_iss` (le meme modele que la vue stations), cuite en
    // maillage, puis mise en orbite comme une lune. Voir `engin.rs` pour
    // l'exageration d'echelle, qui est assumee et inevitable.
    if let Some(iss) = crate::engin::EnginOrbital::iss(
        &crate::vaisseau::preset_iss(),
        terre,
        rayon_terre,
    ) {
        let idx_iss = sys.ajouter(Box::new(iss));
        sys.nommer(idx_iss, "ISS");
    }

    // ===== Le Starship, posté en orbite terrestre =====
    // Premier vaisseau **pilotable** du jeu (chantier trajectoires, Lot C) :
    // assemblé par `vaisseau::preset_starship` (même modèle que la vue
    // stations), posté comme l'ISS mais avec ses statistiques de vol
    // calculées à la pose (`EnginOrbital::starship_pose`) — c'est ce qui lui
    // permettra de décoller vers une destination choisie en jeu.
    if let Some(starship) = crate::engin::EnginOrbital::starship_pose(
        &crate::vaisseau::preset_starship(),
        terre,
        rayon_terre,
    ) {
        let idx_starship = sys.ajouter(Box::new(starship));
        sys.nommer(idx_starship, "STARSHIP");
    }

    (sys, "Systeme solaire - jusqu'a Pluton".to_string())
}

/// Preset Tau Ceti (G8V, ~5344 K, L ≈ 0.52) : 4 super-Terres + disque de débris.
/// Données : Feng et al. 2017 (g 0.133 UA, h 0.243, e 0.538 tempérée, f 1.34).
pub fn construire_preset_tau_ceti() -> (Systeme, String) {
    srand(8552);
    let deg = std::f32::consts::PI / 180.0;
    let z = Vec3::ZERO;

    let mut sys = Systeme::new();
    let lumi_tau_ceti = 0.52;
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        1.8,
        etoile::couleur_corps_noir(5344.0),
        lumi_tau_ceti,
    )));
    let masse_tau_ceti = masse_luminosite(lumi_tau_ceti);

    use TypePlanete::Tellurique;
    ajouter_planete(&mut sys, masse_tau_ceti, 0.133, 0.06, 2.0 * deg, 0.45, 2.0,
        app_simple(Tellurique, vec3(0.55, 0.5, 0.45), vec3(0.34, 0.31, 0.28), z, 0.0), None);
    ajouter_planete(&mut sys, masse_tau_ceti, 0.243, 0.23, 1.5 * deg, 0.45, 2.0,
        app_simple(Tellurique, vec3(0.7, 0.45, 0.3), vec3(0.45, 0.28, 0.2), z, 0.0), None);
    ajouter_planete(&mut sys, masse_tau_ceti, 0.538, 0.18, 1.0 * deg, 0.6, 4.0,
        app_simple(Tellurique, vec3(0.3, 0.5, 0.25), vec3(0.4, 0.34, 0.25), vec3(0.1, 0.35, 0.7), 1.0), None);
    ajouter_planete(&mut sys, masse_tau_ceti, 1.34, 0.16, 1.5 * deg, 0.6, 4.0,
        app_simple(Tellurique, vec3(0.6, 0.4, 0.3), vec3(0.4, 0.26, 0.2), vec3(0.7, 0.8, 0.9), 0.1), None);

    sys.ajouter(Box::new(Disque::new(DisqueConfig::kuiper(
        1600, 3.0 * etoile::UA, 16.0 * etoile::UA, masse_tau_ceti,
    ))));

    (sys, "Tau Ceti (G8V) - 4 super-Terres".to_string())
}

/// Preset du système d'Avatar : la lune habitée **Pandora** orbite la géante
/// gazeuse **Polyphemus**, elle-même autour d'**Alpha Centauri A** (G2V, ~5790 K,
/// L ≈ 1.5). NB : dans le canon Avatar l'étoile est Alpha Centauri A, pas Proxima.
pub fn construire_preset_avatar() -> (Systeme, String) {
    srand(1409);
    let deg = std::f32::consts::PI / 180.0;
    let z = Vec3::ZERO;
    let bleu = vec3(0.35, 0.55, 1.0) * 0.9; // atmosphère océanique
    let blanc = vec3(1.0, 1.0, 1.0);
    let spot = vec3(0.6, -0.22, 0.77); // direction de la tache de Polyphemus

    let mut sys = Systeme::new();
    // Alpha Centauri A : jumelle du Soleil, légèrement plus lumineuse.
    let lumi_aca_avatar = 1.5;
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        2.05,
        etoile::couleur_corps_noir(5790.0),
        lumi_aca_avatar,
    )));
    let masse_aca_avatar = masse_luminosite(lumi_aca_avatar);

    use TypePlanete::Tellurique;
    // Petite tellurique interne brûlée (flavor système).
    ajouter_planete(&mut sys, masse_aca_avatar, 0.55, 0.05, 3.0 * deg, 0.34, 0.4,
        app_simple(Tellurique, vec3(0.55, 0.42, 0.32), vec3(0.36, 0.28, 0.22), z, 0.0), None);

    // Polyphemus : géante gazeuse bleu-vert dans la zone habitable, avec anneaux.
    let poly = ajouter_planete(&mut sys, masse_aca_avatar, 1.35, 0.03, 1.2 * deg, 1.7, 20.0,
        gazeuse(vec3(0.32, 0.6, 0.58), vec3(0.12, 0.34, 0.42), vec3(0.6, 0.84, 0.78), 5.0, 1.7, vec3(0.4, 0.7, 0.68) * 0.3)
            .avec_jets(0.95)
            .avec_tache(spot, 0.24, vec3(0.88, 0.34, 0.16))
            .avec_cyclones_pol()
            .avec_tempetes(0.6)
            .avec_pole(vec3(0.4, 0.56, 0.56))
            .avec_anneau_saturne(vec3(0.66, 0.78, 0.74)), Some("Polyphemus"));

    // Pandora : lune luxuriante, bioluminescente, montagnes flottantes (relief fort).
    let pandora_app = tellurique(
        vec3(0.2, 0.4, 0.3), vec3(0.22, 0.3, 0.26), vec3(0.06, 0.34, 0.55),
        0.7, 0.0, 0.42, 0.9, bleu,
    )
    .avec_vegetation(vec3(0.14, 0.52, 0.32), 0.95)
    .avec_relief(0.85)
    .avec_rivieres(0.4)
    .avec_biolum(0.9)
    .avec_nuages(0.4, blanc);
    let r_poly = 1.7; // rayon visuel de Polyphemus (cf. ajouter_planete ci-dessus)
    let pandora = Planete::new(Vec3::ZERO, Vec3::ZERO, r_poly * 0.34, 0.1, pandora_app, Vec::new())
        .en_lune(poly, r_poly * 4.2, 0.9, 0.12, 0.0);
    let idx_pandora = sys.ajouter(Box::new(pandora));
    sys.nommer(idx_pandora, "Pandora");

    // Lunes secondaires de Polyphemus (le système en compte une douzaine).
    for _ in 0..4 {
        ajouter_lune(&mut sys, poly, r_poly);
    }

    // Géante externe glacée + sa lune.
    let externe = ajouter_planete(&mut sys, masse_aca_avatar, 4.8, 0.05, 1.6 * deg, 1.2, 10.0,
        gazeuse(vec3(0.5, 0.62, 0.74), vec3(0.3, 0.42, 0.56), vec3(0.7, 0.82, 0.92), 4.0, 1.0, vec3(0.5, 0.65, 0.8) * 0.3)
            .avec_brume(0.35, vec3(0.66, 0.78, 0.88))
            .avec_pole(vec3(0.5, 0.6, 0.7)), None);
    ajouter_lune(&mut sys, externe, 1.2);

    // Ceinture d'astéroïdes entre Polyphemus et la géante externe.
    sys.ajouter(Box::new(Disque::new(DisqueConfig::asteroides(
        800, 2.3 * etoile::UA, 3.4 * etoile::UA, masse_aca_avatar,
    ))));

    (sys, "Avatar - Pandora / Polyphemus (Alpha Centauri A)".to_string())
}

/// Preset **Alpha Centauri** selon le canon *Avatar* (Pandorapedia / The Science of Avatar) :
/// binaire **ACA** (Rigil Kentaurus, G2V, ~20 % plus grande que le Soleil) + **ACB** (Toliman,
/// K1V, orangée, ~500 K plus froide), séparation 11–35 UA (a≈23, e≈0.52, période ~80 ans).
/// ACA héberge Odyssey, Ulysses, Oceanus, **Polyphemus** (+ Pandora, Dante, Hadès…) et Coeus ;
/// ACB héberge Vulcain → Poséidon (système « miroir » du Solaire). Proxima (ACC), à >10 000 UA,
/// fait l'objet de son propre preset.
pub fn construire_preset_alpha_centauri() -> (Systeme, String) {
    srand(2154); // année d'Avatar
    let deg = std::f32::consts::PI / 180.0;
    let spot = vec3(0.6, -0.22, 0.77); // direction de l'œil de Polyphemus

    let ma = 1.1 * MASSE_ETOILE; // ACA (primaire)
    let mb = 0.9 * MASSE_ETOILE; // ACB (secondaire)

    let mut sys = Systeme::new();
    // Binaire ACA·ACB. Ordre de déploiement -> ACA = Etoile(0), ACB = Etoile(1).
    let a = Noeud::etoile(Feuille::new(2.05, etoile::couleur_corps_noir(5790.0), 1.5, ma));
    let b = Noeud::etoile(Feuille::new(1.55, etoile::couleur_corps_noir(5290.0), 0.5, mb));
    deployer_arbre(&mut sys, Noeud::paire(a, b, 23.0, 0.52, 0.04, 0.0));

    // ===== Système d'ACA (5 planètes) — Foyer::Etoile(0) =====
    // Apparences = presets nommés du catalogue (source unique avec la galerie), + modifs ponctuelles.
    let aca = Foyer::Etoile(0);
    // Odyssey & Ulysses : telluriques « glace autour d'un noyau rocheux », orbites chaotiques (e élevé).
    ajouter_planete_autour(&mut sys, aca, ma, 0.5, 0.28, 6.0 * deg, 0.32, 0.4,
        preset_tellurique("Boule de neige"), None);
    ajouter_planete_autour(&mut sys, aca, ma, 0.82, 0.24, 5.0 * deg, 0.36, 0.5,
        preset_tellurique("Subglaciaire"), None);
    // Oceanus : géante gazeuse entièrement recouverte d'eau (nuages d'eau, albédo haut), essaim de lunes.
    let oceanus = ajouter_planete_autour(&mut sys, aca, ma, 1.1, 0.03, 1.0 * deg, 1.3, 12.0,
        fige(preset_gazeuse("Classe II (eau, albedo haut)"), "Oceanus"), Some("Oceanus"));
    for _ in 0..4 {
        ajouter_lune(&mut sys, oceanus, 1.3);
    }
    // Polyphemus : la plus grande planète d'ACA, dans la zone habitable ; œil de tempête agrandi
    // (plus grand que la Grande Tache Rouge, cf. canon) par rapport au preset galerie.
    let poly = ajouter_planete_autour(&mut sys, aca, ma, 1.5, 0.03, 1.2 * deg, 1.8, 22.0,
        preset_gazeuse("Polyphemus (Avatar)").avec_tache(spot, 0.3, vec3(0.88, 0.34, 0.16)), Some("Polyphemus"));
    let r_poly = 1.8;
    // Pandora : lune luxuriante, bioluminescente (preset catalogue « Pandora »).
    sys.ajouter(Box::new(
        Planete::new(Vec3::ZERO, Vec3::ZERO, r_poly * 0.34, 0.1, preset_tellurique("Pandora"), Vec::new())
            .en_lune(poly, r_poly * 4.2, 0.9, 0.12, 0.0),
    ));
    // Dante : lune volcanique en éruption perpétuelle (preset « Io (soufre) »).
    sys.ajouter(Box::new(
        Planete::new(Vec3::ZERO, Vec3::ZERO, r_poly * 0.2, 0.05, preset_tellurique("Io (soufre)"), Vec::new())
            .en_lune(poly, r_poly * 2.6, 1.5, 0.05, 1.2),
    ));
    // Hadès : lune brûlante (~900 K), quasi-fusion (preset « Lave »).
    sys.ajouter(Box::new(
        Planete::new(Vec3::ZERO, Vec3::ZERO, r_poly * 0.22, 0.05, preset_tellurique("Lave"), Vec::new())
            .en_lune(poly, r_poly * 3.3, 1.1, 0.18, 3.0),
    ));
    // Cassandra + Chaos + lunes ordinaires (Polyphemus en compte 14 au total).
    for _ in 0..3 {
        ajouter_lune(&mut sys, poly, r_poly);
    }
    // Coeus : la plus petite géante gazeuse, la plus externe (preset « Classe I (ammoniac) », terne) ;
    // lunes Dionysos + Bacchus.
    let coeus = ajouter_planete_autour(&mut sys, aca, ma, 2.3, 0.06, 2.0 * deg, 1.1, 8.0,
        fige(preset_gazeuse("Classe I (ammoniac)"), "Coeus"), Some("Coeus"));
    let r_coeus = 1.1;
    // Dionysos : grosse lune glacée (~19 000 km), orbite classique.
    sys.ajouter(Box::new(
        Planete::new(Vec3::ZERO, Vec3::ZERO, r_coeus * 0.3, 0.05, preset_tellurique("Boule de neige"), Vec::new())
            .en_lune(coeus, r_coeus * 3.5, 0.8, 0.06, 0.5),
    ));
    // Bacchus : minuscule lune (<500 km) à l'orbite « en pétales de fleur » (fortement inclinée/excentrée).
    sys.ajouter(Box::new(
        Planete::new(Vec3::ZERO, Vec3::ZERO, r_coeus * 0.12, 0.02, preset_tellurique("Lune"), Vec::new())
            .en_lune(coeus, r_coeus * 5.5, 2.4, 0.6, 2.0),
    ));

    // ===== Système d'ACB (8 planètes, « miroir » du Solaire) — Foyer::Etoile(1) =====
    // Apparences = presets nommés du catalogue.
    let acb = Foyer::Etoile(1);
    // Vulcain & Hermès : enfers sans air, cratérisés (façon Mercure).
    ajouter_planete_autour(&mut sys, acb, mb, 0.3, 0.08, 5.0 * deg, 0.3, 0.3,
        preset_tellurique("Fer (Mercure)"), None);
    ajouter_planete_autour(&mut sys, acb, mb, 0.46, 0.06, 4.0 * deg, 0.32, 0.35,
        preset_tellurique("Lune"), None);
    // Aphrodite : presque aucune atmosphère (contrairement à Vénus), croûte de sels (preset « Salines »). 1 lune.
    let aphrodite = ajouter_planete_autour(&mut sys, acb, mb, 0.66, 0.02, 2.0 * deg, 0.42, 0.6,
        fige(preset_tellurique("Salines"), "Aphrodite"), Some("Aphrodite"));
    ajouter_lune(&mut sys, aphrodite, 0.42);
    // Gaea : effet de serre massif emballé (preset « Venus (etuve) »). 2 lunes.
    let gaea = ajouter_planete_autour(&mut sys, acb, mb, 0.9, 0.02, 1.0 * deg, 0.46, 0.7,
        fige(preset_tellurique("Venus (etuve)"), "Gaea"), Some("Gaea"));
    for _ in 0..2 {
        ajouter_lune(&mut sys, gaea, 0.46);
    }
    // Ares : atmosphère fine, rougeâtre, badlands (façon Mars). Quelques lunes.
    let ares = ajouter_planete_autour(&mut sys, acb, mb, 1.25, 0.09, 1.8 * deg, 0.4, 0.5,
        fige(preset_tellurique("Badlands"), "Ares"), Some("Ares"));
    for _ in 0..2 {
        ajouter_lune(&mut sys, ares, 0.4);
    }
    // Zeus : la plus grande planète de tout le système (preset « Jupiter »).
    let zeus = ajouter_planete_autour(&mut sys, acb, mb, 1.7, 0.05, 1.3 * deg, 2.0, 24.0,
        fige(preset_gazeuse("Jupiter"), "Zeus"), Some("Zeus"));
    for _ in 0..3 {
        ajouter_lune(&mut sys, zeus, 2.0);
    }
    // Cronus : géante à anneaux, orbite chaotique (preset « Saturne »).
    let cronus = ajouter_planete_autour(&mut sys, acb, mb, 2.15, 0.11, 2.5 * deg, 1.5, 12.0,
        fige(preset_gazeuse("Saturne"), "Cronus"), Some("Cronus"));
    ajouter_lune(&mut sys, cronus, 1.5);
    // Poséidon : géante bleue annelée, orbite chaotique (preset « Anneaux en arcs (type Neptune) »).
    let poseidon = ajouter_planete_autour(&mut sys, acb, mb, 2.55, 0.13, 1.8 * deg, 1.3, 10.0,
        fige(preset_gazeuse("Anneaux en arcs (type Neptune)"), "Poseidon"), Some("Poseidon"));
    ajouter_lune(&mut sys, poseidon, 1.3);

    // Vue par défaut : focaliser ACA (Etoile 0) sur sa zone planétaire.
    sys.definir_vue(0, 3.0 * etoile::UA);

    (sys, "Alpha Centauri (Avatar) - ACA + ACB".to_string())
}

/// Preset **Proxima Centauri** (ACC) selon le canon *Avatar* : naine rouge M (~20 % du rayon
/// solaire, moins de la moitié de la température → lueur rouge terne), avec une géante gazeuse
/// proche et deux planètes rocheuses. Trop lointaine (>10 000 UA) pour être cadrée avec ACA/ACB,
/// d'où son preset dédié.
pub fn construire_preset_proxima() -> (Systeme, String) {
    srand(4243);
    let deg = std::f32::consts::PI / 180.0;

    let mut sys = Systeme::new();
    // Naine rouge : L réelle ≈ 0.0017, relevée à 0.06 pour un rendu visible.
    let lumi_proxima = 0.06;
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        0.75,
        etoile::couleur_corps_noir(3040.0),
        lumi_proxima,
    )));
    let masse_proxima = masse_luminosite(lumi_proxima);

    // Apparences = presets nommés du catalogue.
    // Géante gazeuse proche et chaude (preset « Jupiter chaud »). Rayon réduit (0.85) pour ne pas
    // écraser la petite naine rouge ; ses lunes se dimensionnent sur ce rayon -> plus discrètes.
    let r_gg = 0.85;
    let gg = ajouter_planete(&mut sys, masse_proxima, 0.2, 0.04, 2.0 * deg, r_gg, 8.0, preset_gazeuse("Jupiter chaud"), None);
    for _ in 0..2 {
        ajouter_lune(&mut sys, gg, r_gg);
    }
    // Planète rocheuse 1 : désert aride (preset « Desert »).
    ajouter_planete(&mut sys, masse_proxima, 0.42, 0.1, 3.0 * deg, 0.34, 0.5, preset_tellurique("Desert"), None);
    // Planète rocheuse 2 : plus externe, froide, verrouillée par les marées -> monde « eyeball » gelé.
    ajouter_planete(&mut sys, masse_proxima, 0.85, 0.07, 2.0 * deg, 0.4, 0.7, preset_tellurique("Eyeball gele"), None);

    (sys, "Proxima Centauri (Avatar) - naine rouge M".to_string())
}

/// Binaire A+B : deux étoiles orbitant leur barycentre. Arbre `(A·B)`.
pub fn construire_preset_binaire() -> (Systeme, String) {
    srand(20260704);
    let mut sys = Systeme::new();
    // A : type G (jaune) ; B : type K (orangée, moins massive).
    let a = Noeud::etoile(Feuille::new(1.7, etoile::couleur_corps_noir(5800.0), 1.0, 1.1 * MASSE_ETOILE));
    let b = Noeud::etoile(Feuille::new(1.3, etoile::couleur_corps_noir(4300.0), 0.6, 0.9 * MASSE_ETOILE));
    deployer_arbre(&mut sys, Noeud::paire(a, b, 6.0, 0.2, 0.0, 0.0));

    // Planète TYPE S : orbite la primaire A (étoile d'index 0). Éclairée par les deux
    // soleils — sa face nuit s'illumine quand le compagnon passe derrière.
    let terre = ajouter_planete_autour(&mut sys, Foyer::Etoile(0), 1.1 * MASSE_ETOILE, 1.2, 0.02, 0.0, 0.55, 1.0,
        app_simple(TypePlanete::Tellurique, vec3(0.3, 0.55, 0.25), vec3(0.4, 0.34, 0.25), vec3(0.1, 0.35, 0.75), 1.0)
            .avec_atmo(vec3(0.35, 0.55, 1.0) * 0.9), None);
    ajouter_lune(&mut sys, terre, 0.55);

    // Planète TYPE P (circumbinaire, « Tatooine ») : orbite le BARYCENTRE du couple,
    // au-delà du rayon critique de stabilité (~18 UA pour ce couple) -> 24 UA.
    ajouter_planete_autour(&mut sys, Foyer::Barycentre, 2.0 * MASSE_ETOILE, 24.0, 0.05, 0.05, 0.7, 0.6,
        app_simple(TypePlanete::Tellurique, vec3(0.55, 0.45, 0.35), vec3(0.4, 0.3, 0.24), vec3(0.6, 0.75, 0.9), 0.15), None);

    (sys, "Binaire A+B : planetes type S + type P".to_string())
}

/// Trinaire hiérarchique `((A·B)·C)` : paire serrée G+K + naine rouge lointaine.
pub fn construire_preset_trinaire() -> (Systeme, String) {
    srand(31);
    let mut sys = Systeme::new();
    let a = Noeud::etoile(Feuille::new(1.6, etoile::couleur_corps_noir(5800.0), 1.0, 1.1 * MASSE_ETOILE));
    let b = Noeud::etoile(Feuille::new(1.2, etoile::couleur_corps_noir(4400.0), 0.5, 0.9 * MASSE_ETOILE));
    let ab = Noeud::paire(a, b, 3.0, 0.15, 0.1, 0.0); // paire serrée
    let c = Noeud::etoile(Feuille::new(0.9, etoile::couleur_corps_noir(3200.0), 0.15, 0.4 * MASSE_ETOILE));
    // C lointain : le barycentre de (A·B) et C orbitent leur barycentre commun.
    deployer_arbre(&mut sys, Noeud::paire(ab, c, 14.0, 0.3, 0.25, 1.0));
    (sys, "Trinaire hierarchique (A-B)+C".to_string())
}

/// Quadruple 2+2 `((A·B)·(C·D))` : deux paires orbitant leur barycentre commun.
/// D est une Wolf-Rayet (vent stellaire) pour illustrer les types d'astres variés.
pub fn construire_preset_quadruple() -> (Systeme, String) {
    srand(42);
    let mut sys = Systeme::new();
    let a = Noeud::etoile(Feuille::new(1.6, etoile::couleur_corps_noir(5900.0), 1.0, 1.1 * MASSE_ETOILE));
    let b = Noeud::etoile(Feuille::new(1.2, etoile::couleur_corps_noir(4300.0), 0.5, 0.9 * MASSE_ETOILE));
    let c = Noeud::etoile(Feuille::new(1.4, etoile::couleur_corps_noir(4800.0), 0.7, 1.0 * MASSE_ETOILE));
    let d = Noeud::etoile(
        Feuille::new(1.1, etoile::couleur_corps_noir(9000.0), 0.9, 0.7 * MASSE_ETOILE)
            .variante(Variante::Vent),
    );
    let ab = Noeud::paire(a, b, 2.5, 0.1, 0.0, 0.0);
    let cd = Noeud::paire(c, d, 3.2, 0.2, 0.15, 0.5);
    deployer_arbre(&mut sys, Noeud::paire(ab, cd, 18.0, 0.25, 0.2, 0.0));
    (sys, "Quadruple 2+2 ((A-B)+(C-D))".to_string())
}

#[cfg(test)]
// Ces tests affirment des choses sur des CONSTANTES, et clippy y voit une
// condition toujours vraie. Elle l'est -- aujourd'hui. Le jour ou quelqu'un
// abaisse la constante, l'assertion devient fausse et le test tombe : c'est
// precisement son role, garder la valeur ET dire pourquoi elle compte.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    /// Les corps nommés du preset solaire.
    ///
    /// ⚠️ **Fixture recopiée**, pas la source : `construire_preset_solaire` a
    /// besoin du contexte graphique (`srand`, `gen_range`) et ne peut donc pas
    /// tourner en test. Si un corps s'ajoute là-haut sans être ajouté ici, ce
    /// test **couvre moins** — il ne devient pas faux. C'est le compromis assumé.
    const CORPS: &[&str] = &[
        "Mercure", "Venus", "Terre", "Lune", "Mars", "Phobos", "Deimos", "Jupiter", "Io",
        "Europe", "Ganymede", "Callisto", "Saturne", "Encelade", "Rhea", "Titan", "Japet",
        "Uranus", "Ariel", "Umbriel", "Titania", "Oberon", "Neptune", "Triton", "Pluton",
    ];

    /// Parent de chaque lune, pour les tests d'orbite.
    // Ne rend que le NOM du parent : la masse se lit dans `MASSES_TERRESTRES`
    // via `masse_monde`. Une copie ici rendrait le test aveugle a une masse
    // fausse dans la table -- il verifierait sa propre copie.
    fn parent_de(nom: &str) -> &'static str {
        match nom {
            "Lune" => "Terre",
            "Phobos" | "Deimos" => "Mars",
            "Io" | "Europe" | "Ganymede" | "Callisto" => "Jupiter",
            "Encelade" | "Rhea" | "Titan" | "Japet" => "Saturne",
            "Ariel" | "Umbriel" | "Titania" | "Oberon" => "Uranus",
            "Triton" => "Neptune",
            _ => panic!("lune sans parent : {nom}"),
        }
    }

    // **Chaque lune orbite HORS de sa planete**, au-dela de la limite de Roche.
    //
    // C'est l'invariant que casse le bouton d'echelle : les rayons grossissent,
    // les distances non. La Lune n'est qu'a 60 rayons terrestres -- vers x25 la
    // Terre la rattrape. Ce test dit ou est le plafond.
    #[test]
    fn les_lunes_orbitent_hors_de_leur_planete() {
        for (nom, _, a_km, _) in LUNES {
            let parent = parent_de(nom);
            let r = distance_lune(a_km);
            let roche = rayon_monde(parent) * 2.4;
            assert!(
                r > roche,
                "{nom} orbite a {r:.4} sous la limite de Roche de {parent} ({roche:.4})                  -- ECHELLE_CORPS = {ECHELLE_CORPS} est trop grand"
            );
            // Et la lune reste plus petite que sa planete.
            assert!(rayon_monde(nom) < rayon_monde(parent), "{nom} depasse {parent}");
        }
    }

    // **Les distances de lunes sont les vraies.** Le placement procedural les
    // mettait six fois trop loin : notre Lune vers 0,8 unite au lieu de 0,123.
    #[test]
    fn les_lunes_sont_a_leur_vraie_distance() {
        // Lu DANS la table, pas recopie ici : une distance ecrite en dur dans le
        // test le rendrait insensible a la table qu'il est cense verifier.
        let d = |n: &str| distance_lune(LUNES.iter().find(|(x, _, _, _)| *x == n).unwrap().2);
        let lune = d("Lune");
        let attendu = 0.1233 * ECHELLE_CORPS;
        assert!((lune - attendu).abs() < 0.002 * ECHELLE_CORPS,
            "la Lune est a {lune:.4} au lieu de {attendu:.4}");
        // 60,3 rayons terrestres : le chiffre que tout le monde connait.
        let en_rayons = lune / rayon_monde("Terre");
        assert!((en_rayons - 60.3).abs() < 1.0, "la Lune est a {en_rayons:.1} rayons au lieu de 60,3");
        // Les lunes galileennes se suivent dans l'ordre.
        assert!(d("Io") < d("Europe") && d("Europe") < d("Ganymede") && d("Ganymede") < d("Callisto"));
    }

    // **Les periodes suivent Kepler, avec la masse du parent.** Io et la Lune
    // sont a peu pres a la meme distance de leur planete, mais Io boucle en
    // 1,8 jour contre 27,3 : c'est Jupiter, 318 fois la Terre, qui fait la
    // difference. Sans les masses, toutes les lunes tourneraient pareil.
    #[test]
    fn les_periodes_de_lunes_suivent_kepler() {
        let periode = |nom: &str| {
            let (_, _, a_km, retro) = *LUNES.iter().find(|(n, _, _, _)| *n == nom).unwrap();
            let r = distance_lune(a_km);
            std::f32::consts::TAU / omega_lune(masse_monde(parent_de(nom)), r, retro).abs()
        };
        // Rapport des periodes Lune / Io : 27,32 / 1,769 = 15,4.
        let rapport = periode("Lune") / periode("Io");
        assert!(
            (rapport - 15.44).abs() < 1.0,
            "Lune/Io = {rapport:.2} au lieu de 15,44 : la masse du parent est ignoree"
        );
        // Kepler interne : Ganymede met 4 fois plus longtemps qu'Io (7,15 / 1,77).
        let g_io = periode("Ganymede") / periode("Io");
        assert!((g_io - 4.04).abs() < 0.2, "Ganymede/Io = {g_io:.2} au lieu de 4,04");
        // Triton est retrograde : son omega est negatif.
        let (_, _, a, retro) = *LUNES.iter().find(|(n, _, _, _)| *n == "Triton").unwrap();
        assert!(retro, "Triton devrait etre retrograde");
        assert!(
            omega_lune(masse_monde("Neptune"), distance_lune(a), retro) < 0.0,
            "Triton tourne a l'endroit"
        );
    }

    // **Plus aucune exageration : l'echelle est vraie.**
    //
    // Le rayon terrestre doit valoir exactement 6 371 km rapportes a l'UA, fois
    // le nombre d'unites par UA. Il y avait un facteur 269 d'exageration ; le
    // retirer est ce qui rend le systeme fidele, au prix de planetes minuscules.
    #[test]
    fn les_rayons_sont_a_lechelle_vraie() {
        let attendu = 6371.0 / 149_597_870.0 * etoile::UA as f64;
        assert!(
            (R_TERRE as f64 - attendu).abs() / attendu < 0.01,
            "R_TERRE = {R_TERRE} au lieu de {attendu} : une exageration a ete reintroduite"
        );
        // Le Soleil suit la meme echelle : 109 fois la Terre, pas mille.
        let rapport = R_SOLEIL / rayon_monde("Terre");
        assert!(
            (rapport - 109.2).abs() < 1.0,
            "Soleil a {rapport:.1} rayons terrestres au lieu de 109,2"
        );
        // Et il reste tres petit devant l'orbite de Mercure (0,39 UA).
        assert!(R_SOLEIL < 0.39 * etoile::UA * 0.25, "le Soleil mange l'orbite de Mercure");
    }

    // **Les lunes sont a la meme echelle que les planetes.**
    //
    // Elles etaient exagerees a part : la Lune faisait 0,44 rayon terrestre au
    // lieu de 0,27, et Ganymede sortait plus petite qu'elle alors qu'elle est
    // une fois et demie plus grosse. A l'echelle vraie l'incoherence saute aux
    // yeux, donc elles viennent de la meme table.
    #[test]
    fn les_lunes_suivent_la_meme_echelle_que_les_planetes() {
        // Ganymede est la plus grosse lune du systeme, devant Titan puis Callisto.
        assert!(rayon_monde("Ganymede") > rayon_monde("Titan"), "Titan depasse Ganymede");
        assert!(rayon_monde("Titan") > rayon_monde("Callisto"), "Callisto depasse Titan");
        assert!(rayon_monde("Ganymede") > rayon_monde("Lune"), "la Lune depasse Ganymede");
        // Ganymede est meme plus grosse que Mercure -- un fait bien connu.
        assert!(rayon_monde("Ganymede") > rayon_monde("Mercure"), "Mercure depasse Ganymede");
        // Mais toutes les lunes restent sous la Terre.
        for (nom, _, _, _) in LUNES {
            assert!(rayon_monde(nom) < rayon_monde("Terre"), "{nom} depasse la Terre");
        }
        // Phobos est un caillou : moins d'un millieme de la Terre.
        assert!(rayon_monde("Phobos") < rayon_monde("Terre") * 0.002, "Phobos trop gros");
    }

    // **Le preset solaire PUISE ses rayons dans la table.**
    //
    // Les deux tests ci-dessous prouvent que la table est juste ; aucun ne
    // prouve qu'on s'en sert. Un rayon reecrit a la main dans l'appel
    // (`..., 1.7, 20.0,`) les laisse tous verts et fait mentir le systeme --
    // c'est exactement ce que faisait le code avant.
    //
    // `construire_preset_solaire` exige le contexte graphique et ne peut pas
    // tourner ici. On inspecte donc le **texte source**. C'est inhabituel, mais
    // c'est le seul garde possible, et il attrape la regression qui compte.
    #[test]
    fn le_preset_solaire_puise_ses_rayons_dans_la_table() {
        let src = include_str!("presets.rs");
        let debut = src.find("pub fn construire_preset_solaire").expect("fonction absente");
        let fin = src[debut..].find("Population de petits corps").expect("fin absente") + debut;
        let corps = &src[debut..fin];

        // Les neuf planetes tirent leur rayon de `rayon_monde`, directement ou
        // par une variable qui en vient.
        let n = corps.matches("rayon_monde(").count();
        assert!(
            n >= 9,
            "seulement {n} rayons pris a la table : un rayon a ete reecrit a la main"
        );
        // Et aucune trace des valeurs codees en dur d'avant.
        for ancien in [", 1.7, 20.0,", ", 1.45, 12.0,", ", 0.32, 0.3,", ", 0.3, 0.1,"] {
            assert!(!corps.contains(ancien), "rayon code en dur retrouve : « {ancien} »");
        }
    }

    // **Les rapports de taille des planetes sont exacts.**
    //
    // Ils etaient ecrits a la main dans chaque appel, et faux : Jupiter valait
    // 3,1 fois la Terre au lieu de 11, Pluton pres de trois fois trop gros. Rien
    // ne le disait, et a l'ecran le systeme ne se lisait pas.
    #[test]
    fn les_rapports_de_taille_des_planetes_sont_exacts() {
        let terre = rayon_monde("Terre");
        assert!(terre > 0.0);
        for (nom, reel) in RAYONS_TERRESTRES {
            let rapport = rayon_monde(nom) / terre;
            assert!(
                (rapport - reel).abs() < 0.01,
                "{nom} : {rapport:.3} fois la Terre au lieu de {reel:.3}"
            );
        }
        // Quelques faits que tout le monde connait, pour qu'une table recopiee
        // de travers se voie : Jupiter ecrase tout, Pluton est minuscule.
        assert!(rayon_monde("Jupiter") > rayon_monde("Terre") * 10.0, "Jupiter trop petit");
        assert!(rayon_monde("Jupiter") > rayon_monde("Saturne"), "Saturne depasse Jupiter");
        assert!(rayon_monde("Pluton") < rayon_monde("Mercure"), "Pluton depasse Mercure");
        assert!(rayon_monde("Mars") < rayon_monde("Venus"), "Mars depasse Venus");
    }

    // **Les planetes sont classees par taille comme dans la realite.** Une seule
    // valeur mal recopiee dans la table casse cet ordre.
    #[test]
    fn lordre_des_tailles_suit_la_realite() {
        let attendu = [
            "Pluton", "Mercure", "Mars", "Venus", "Terre", "Neptune", "Uranus", "Saturne", "Jupiter",
        ];
        let mut noms: Vec<&str> = RAYONS_TERRESTRES.iter().map(|(n, _)| *n).collect();
        noms.sort_by(|a, b| rayon_monde(a).total_cmp(&rayon_monde(b)));
        assert_eq!(noms, attendu, "ordre des tailles inattendu");
    }

    // **La géographie d'un corps ne dépend que de son nom.** C'est tout l'objet
    // de `graine_nom` : avant, la graine venait du flux aléatoire, donc insérer
    // une planète redessinait les continents de toutes les suivantes.
    #[test]
    fn la_graine_ne_depend_que_du_nom() {
        for c in CORPS {
            assert_eq!(graine_nom(c), graine_nom(c), "{c} : graine instable");
        }
        // Et elle ne dépend pas de l'ordre d'appel : deux lectures encadrant
        // d'autres appels donnent le même résultat.
        let avant = graine_nom("Terre");
        for c in CORPS {
            let _ = graine_nom(c);
        }
        assert_eq!(graine_nom("Terre"), avant, "la graine dépend de l'ordre d'appel");
    }

    // **Deux corps n'ont pas la même géographie.** Une collision donnerait deux
    // lunes rigoureusement identiques — le défaut le plus visible ici, puisque
    // plusieurs partagent déjà la même apparence de catalogue.
    #[test]
    fn deux_corps_nont_pas_la_meme_geographie() {
        for (i, a) in CORPS.iter().enumerate() {
            for b in &CORPS[i + 1..] {
                assert_ne!(graine_nom(a), graine_nom(b), "{a} et {b} partagent leur géographie");
            }
        }
    }

    // La graine reste **dans la plage où le générateur de terrain a été réglé**
    // (celle du catalogue). En sortir exposerait du bruit jamais éprouvé.
    #[test]
    fn la_graine_reste_dans_la_plage_du_catalogue() {
        for c in CORPS {
            let g = graine_nom(c);
            assert!((0.0..1000.0).contains(&g), "{c} : graine {g} hors plage");
        }
        // Et elle occupe vraiment la plage : un hash qui rendrait toujours de
        // petites valeurs passerait l'assertion ci-dessus tout en concentrant
        // toutes les géographies au même endroit.
        let max = CORPS.iter().map(|c| graine_nom(c)).fold(0.0_f32, f32::max);
        let min = CORPS.iter().map(|c| graine_nom(c)).fold(1000.0_f32, f32::min);
        assert!(max > 700.0 && min < 300.0, "graines tassées entre {min} et {max}");
    }

    /// **Chaque planète tourne à SON rythme.**
    ///
    /// La vitesse était codée en dur dans le shader (`time * 0.01`) : neuf
    /// planètes, une seule cadence. Or Jupiter boucle en 10 h et Vénus en 243
    /// jours — une différence qui se voit à l'œil dès qu'on regarde deux corps
    /// côte à côte.
    #[test]
    fn les_planetes_ne_tournent_pas_toutes_au_meme_rythme() {
        let v = |n: &str| {
            let (_, j) = ROTATIONS_JOURS.iter().find(|(x, _)| *x == n).expect(n);
            vitesse_rotation(*j)
        };
        // Jupiter tourne franchement plus vite que la Terre.
        assert!(v("Jupiter") > v("Terre") * 2.0, "Jupiter devrait filer");
        // Et les traînards sont loin derrière.
        assert!(v("Venus").abs() < v("Terre") * 0.1, "Vénus devrait ramper");
        assert!(v("Mercure").abs() < v("Terre") * 0.1, "Mercure devrait ramper");

        // Deux corps au moins doivent tourner à l'ENVERS : Vénus et Uranus.
        let retro: Vec<&str> = ROTATIONS_JOURS
            .iter()
            .filter(|(_, j)| vitesse_rotation(*j) < 0.0)
            .map(|(n, _)| *n)
            .collect();
        assert!(retro.contains(&"Venus"), "Vénus tourne à l'envers : {retro:?}");
        assert!(retro.contains(&"Uranus"), "Uranus tourne à l'envers : {retro:?}");

        // Et l'ensemble reste étalé : sans écart, autant garder une constante.
        let vitesses: Vec<f32> = ROTATIONS_JOURS.iter().map(|(_, j)| vitesse_rotation(*j).abs()).collect();
        let (mn, mx) = vitesses.iter().fold((f32::MAX, 0.0f32), |(a, b), v| (a.min(*v), b.max(*v)));
        assert!(mx / mn > 8.0, "rythmes trop resserrés : {mn} à {mx}");
    }

    /// Les bornes de [`vitesse_rotation`], chacune contre un travers précis.
    #[test]
    fn la_vitesse_de_rotation_reste_dans_le_lisible() {
        // Le plancher : à l'échelle littérale, Vénus mettrait 42 h de jeu pour
        // un tour, ce qui se lit comme une planète cassée et non comme un fait.
        assert!(vitesse_rotation(-243.0).abs() >= 0.01 / 40.0 - 1e-9);
        // Le plafond : un rotateur très rapide ne doit pas devenir une toupie.
        assert!(vitesse_rotation(0.001) <= 0.01 * 4.0 + 1e-9);
        // Le signe passe : c'est lui qui dit le sens.
        assert!(vitesse_rotation(-1.0) < 0.0 && vitesse_rotation(1.0) > 0.0);
        // Une période nulle n'a pas de sens : on retombe sur la RÉFÉRENCE (la
        // Terre), et non sur la vitesse maximale.
        //
        // ⚠️ Sans le garde-fou, `BASE / 0.0` vaut l'infini, que `clamp` ramène
        // sagement au plafond — donc la planète la plus rapide du système. Le
        // résultat restait fini et positif : une assertion qui se contentait de
        // ça laissait passer le défaut. Il faut nommer la valeur attendue.
        assert!((vitesse_rotation(0.0) - vitesse_rotation(1.0)).abs() < 1e-9);
    }

    /// La table de rotation ne parle que de corps que le catalogue connaît.
    #[test]
    fn les_corps_qui_tournent_ont_un_rayon() {
        for (n, _) in ROTATIONS_JOURS {
            assert!(
                RAYONS_TERRESTRES.iter().any(|(r, _)| *r == n),
                "{n} a une rotation mais aucun rayon : une des deux tables ment"
            );
        }
    }
}
