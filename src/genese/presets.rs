use super::apparences::{gazeuse, tellurique};
use super::{
    ajouter_lune, ajouter_lune_preset, ajouter_planete, ajouter_planete_autour, app_simple,
    deployer_arbre, preset_gazeuse, preset_tellurique, MASSE_ETOILE,
};
use crate::astre::Foyer;
use crate::stellaire::{Feuille, Noeud, Variante};
use crate::ceinture::{Ceinture, CeintureConfig};
use crate::etoile;
use crate::planete::{Planete, TypePlanete};
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;
use macroquad::rand::srand;

/// Preset reproduisant notre système solaire (Mercure → Pluton + ceintures).
pub fn construire_preset_solaire() -> (Systeme, String) {
    srand(42);
    let deg = std::f32::consts::PI / 180.0;

    let mut sys = Systeme::new();
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        2.0,
        etoile::couleur_corps_noir(5800.0),
        1.0,
    )));

    // Apparences tirées des catalogues (source unique avec la galerie) ; on conserve
    // les distances (UA réelles), excentricités, inclinaisons, rayons et masses.
    ajouter_planete(&mut sys, 0.39, 0.205, 7.0 * deg, 0.32, 0.3,
        preset_tellurique("Fer (Mercure)"));
    ajouter_planete(&mut sys, 0.72, 0.007, 3.4 * deg, 0.52, 1.0,
        preset_tellurique("Venus (etuve)"));
    let terre = ajouter_planete(&mut sys, 1.0, 0.017, 0.0, 0.55, 1.0,
        preset_tellurique("Terre"));
    ajouter_lune_preset(&mut sys, terre, 0.55, 0.24, preset_tellurique("Lune")); // la Lune
    let mars = ajouter_planete(&mut sys, 1.52, 0.093, 1.85 * deg, 0.42, 0.4,
        preset_tellurique("Badlands")); // analogue de Mars (rouille, canyons)
    // Phobos + Déimos : deux petits corps sombres capturés.
    ajouter_lune_preset(&mut sys, mars, 0.42, 0.05, preset_tellurique("Carbone")); // Phobos
    ajouter_lune_preset(&mut sys, mars, 0.42, 0.05, preset_tellurique("Carbone")); // Déimos
    let jupiter = ajouter_planete(&mut sys, 5.2, 0.049, 1.3 * deg, 1.7, 20.0,
        preset_gazeuse("Jupiter"));
    // 4 lunes galiléennes (interne → externe), aspects distincts.
    ajouter_lune_preset(&mut sys, jupiter, 1.7, 0.10, preset_tellurique("Io (soufre)")); // Io
    ajouter_lune_preset(&mut sys, jupiter, 1.7, 0.09, preset_tellurique("Subglaciaire")); // Europe
    ajouter_lune_preset(&mut sys, jupiter, 1.7, 0.13, preset_tellurique("Lune")); // Ganymède
    ajouter_lune_preset(&mut sys, jupiter, 1.7, 0.12, preset_tellurique("Carbone")); // Callisto
    let saturne = ajouter_planete(&mut sys, 9.58, 0.056, 2.49 * deg, 1.45, 12.0,
        preset_gazeuse("Saturne"));
    // Encelade, Rhéa, Titan (le plus gros), Japet (interne → externe).
    ajouter_lune_preset(&mut sys, saturne, 1.45, 0.06, preset_tellurique("Pics de glace")); // Encelade
    ajouter_lune_preset(&mut sys, saturne, 1.45, 0.07, preset_tellurique("Boule de neige")); // Rhéa
    ajouter_lune_preset(&mut sys, saturne, 1.45, 0.13, preset_tellurique("Titan")); // Titan
    ajouter_lune_preset(&mut sys, saturne, 1.45, 0.07, preset_tellurique("Carbone")); // Japet
    let uranus = ajouter_planete(&mut sys, 19.2, 0.046, 0.77 * deg, 1.0, 6.0,
        preset_gazeuse("Uranus"));
    // Ariel, Umbriel, Titania, Obéron (interne → externe).
    ajouter_lune_preset(&mut sys, uranus, 1.0, 0.07, preset_tellurique("Boule de neige")); // Ariel
    ajouter_lune_preset(&mut sys, uranus, 1.0, 0.07, preset_tellurique("Carbone")); // Umbriel
    ajouter_lune_preset(&mut sys, uranus, 1.0, 0.09, preset_tellurique("Subglaciaire")); // Titania
    ajouter_lune_preset(&mut sys, uranus, 1.0, 0.08, preset_tellurique("Lune")); // Obéron
    let neptune = ajouter_planete(&mut sys, 30.05, 0.010, 1.77 * deg, 1.0, 6.0,
        preset_gazeuse("Neptune"));
    ajouter_lune_preset(&mut sys, neptune, 1.0, 0.11, preset_tellurique("Cryovolcan")); // Triton
    ajouter_planete(&mut sys, 39.5, 0.249, 17.1 * deg, 0.3, 0.1,
        preset_tellurique("Boule de neige")); // analogue de Pluton (nain glacé)

    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::asteroides(
        900, 2.2 * etoile::UA, 3.3 * etoile::UA, MASSE_ETOILE,
    ))));
    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::kuiper(
        2000, 30.0 * etoile::UA, 48.0 * etoile::UA, MASSE_ETOILE,
    ))));

    (sys, "Systeme solaire - jusqu'a Pluton".to_string())
}

/// Preset Tau Ceti (G8V, ~5344 K, L ≈ 0.52) : 4 super-Terres + disque de débris.
/// Données : Feng et al. 2017 (g 0.133 UA, h 0.243, e 0.538 tempérée, f 1.34).
pub fn construire_preset_tau_ceti() -> (Systeme, String) {
    srand(8552);
    let deg = std::f32::consts::PI / 180.0;
    let z = Vec3::ZERO;

    let mut sys = Systeme::new();
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        1.8,
        etoile::couleur_corps_noir(5344.0),
        0.52,
    )));

    use TypePlanete::Tellurique;
    ajouter_planete(&mut sys, 0.133, 0.06, 2.0 * deg, 0.45, 2.0,
        app_simple(Tellurique, vec3(0.55, 0.5, 0.45), vec3(0.34, 0.31, 0.28), z, 0.0));
    ajouter_planete(&mut sys, 0.243, 0.23, 1.5 * deg, 0.45, 2.0,
        app_simple(Tellurique, vec3(0.7, 0.45, 0.3), vec3(0.45, 0.28, 0.2), z, 0.0));
    ajouter_planete(&mut sys, 0.538, 0.18, 1.0 * deg, 0.6, 4.0,
        app_simple(Tellurique, vec3(0.3, 0.5, 0.25), vec3(0.4, 0.34, 0.25), vec3(0.1, 0.35, 0.7), 1.0));
    ajouter_planete(&mut sys, 1.34, 0.16, 1.5 * deg, 0.6, 4.0,
        app_simple(Tellurique, vec3(0.6, 0.4, 0.3), vec3(0.4, 0.26, 0.2), vec3(0.7, 0.8, 0.9), 0.1));

    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::kuiper(
        1600, 3.0 * etoile::UA, 16.0 * etoile::UA, MASSE_ETOILE,
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
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        2.05,
        etoile::couleur_corps_noir(5790.0),
        1.5,
    )));

    use TypePlanete::Tellurique;
    // Petite tellurique interne brûlée (flavor système).
    ajouter_planete(&mut sys, 0.55, 0.05, 3.0 * deg, 0.34, 0.4,
        app_simple(Tellurique, vec3(0.55, 0.42, 0.32), vec3(0.36, 0.28, 0.22), z, 0.0));

    // Polyphemus : géante gazeuse bleu-vert dans la zone habitable, avec anneaux.
    let poly = ajouter_planete(&mut sys, 1.35, 0.03, 1.2 * deg, 1.7, 20.0,
        gazeuse(vec3(0.32, 0.6, 0.58), vec3(0.12, 0.34, 0.42), vec3(0.6, 0.84, 0.78), 5.0, 1.7, vec3(0.4, 0.7, 0.68) * 0.3)
            .avec_jets(0.95)
            .avec_tache(spot, 0.24, vec3(0.88, 0.34, 0.16))
            .avec_cyclones_pol()
            .avec_tempetes(0.6)
            .avec_pole(vec3(0.4, 0.56, 0.56))
            .avec_anneau_saturne(vec3(0.66, 0.78, 0.74)));

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
    sys.ajouter(Box::new(pandora));

    // Lunes secondaires de Polyphemus (le système en compte une douzaine).
    for _ in 0..4 {
        ajouter_lune(&mut sys, poly, r_poly);
    }

    // Géante externe glacée + sa lune.
    let externe = ajouter_planete(&mut sys, 4.8, 0.05, 1.6 * deg, 1.2, 10.0,
        gazeuse(vec3(0.5, 0.62, 0.74), vec3(0.3, 0.42, 0.56), vec3(0.7, 0.82, 0.92), 4.0, 1.0, vec3(0.5, 0.65, 0.8) * 0.3)
            .avec_brume(0.35, vec3(0.66, 0.78, 0.88))
            .avec_pole(vec3(0.5, 0.6, 0.7)));
    ajouter_lune(&mut sys, externe, 1.2);

    // Ceinture d'astéroïdes entre Polyphemus et la géante externe.
    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::asteroides(
        800, 2.3 * etoile::UA, 3.4 * etoile::UA, MASSE_ETOILE,
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
        preset_tellurique("Boule de neige"));
    ajouter_planete_autour(&mut sys, aca, ma, 0.82, 0.24, 5.0 * deg, 0.36, 0.5,
        preset_tellurique("Subglaciaire"));
    // Oceanus : géante gazeuse entièrement recouverte d'eau (nuages d'eau, albédo haut), essaim de lunes.
    let oceanus = ajouter_planete_autour(&mut sys, aca, ma, 1.1, 0.03, 1.0 * deg, 1.3, 12.0,
        preset_gazeuse("Classe II (eau, albedo haut)"));
    for _ in 0..4 {
        ajouter_lune(&mut sys, oceanus, 1.3);
    }
    // Polyphemus : la plus grande planète d'ACA, dans la zone habitable ; œil de tempête agrandi
    // (plus grand que la Grande Tache Rouge, cf. canon) par rapport au preset galerie.
    let poly = ajouter_planete_autour(&mut sys, aca, ma, 1.5, 0.03, 1.2 * deg, 1.8, 22.0,
        preset_gazeuse("Polyphemus (Avatar)").avec_tache(spot, 0.3, vec3(0.88, 0.34, 0.16)));
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
        preset_gazeuse("Classe I (ammoniac)"));
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
        preset_tellurique("Fer (Mercure)"));
    ajouter_planete_autour(&mut sys, acb, mb, 0.46, 0.06, 4.0 * deg, 0.32, 0.35,
        preset_tellurique("Lune"));
    // Aphrodite : presque aucune atmosphère (contrairement à Vénus), croûte de sels (preset « Salines »). 1 lune.
    let aphrodite = ajouter_planete_autour(&mut sys, acb, mb, 0.66, 0.02, 2.0 * deg, 0.42, 0.6,
        preset_tellurique("Salines"));
    ajouter_lune(&mut sys, aphrodite, 0.42);
    // Gaea : effet de serre massif emballé (preset « Venus (etuve) »). 2 lunes.
    let gaea = ajouter_planete_autour(&mut sys, acb, mb, 0.9, 0.02, 1.0 * deg, 0.46, 0.7,
        preset_tellurique("Venus (etuve)"));
    for _ in 0..2 {
        ajouter_lune(&mut sys, gaea, 0.46);
    }
    // Ares : atmosphère fine, rougeâtre, badlands (façon Mars). Quelques lunes.
    let ares = ajouter_planete_autour(&mut sys, acb, mb, 1.25, 0.09, 1.8 * deg, 0.4, 0.5,
        preset_tellurique("Badlands"));
    for _ in 0..2 {
        ajouter_lune(&mut sys, ares, 0.4);
    }
    // Zeus : la plus grande planète de tout le système (preset « Jupiter »).
    let zeus = ajouter_planete_autour(&mut sys, acb, mb, 1.7, 0.05, 1.3 * deg, 2.0, 24.0,
        preset_gazeuse("Jupiter"));
    for _ in 0..3 {
        ajouter_lune(&mut sys, zeus, 2.0);
    }
    // Cronus : géante à anneaux, orbite chaotique (preset « Saturne »).
    let cronus = ajouter_planete_autour(&mut sys, acb, mb, 2.15, 0.11, 2.5 * deg, 1.5, 12.0,
        preset_gazeuse("Saturne"));
    ajouter_lune(&mut sys, cronus, 1.5);
    // Poséidon : géante bleue annelée, orbite chaotique (preset « Anneaux en arcs (type Neptune) »).
    let poseidon = ajouter_planete_autour(&mut sys, acb, mb, 2.55, 0.13, 1.8 * deg, 1.3, 10.0,
        preset_gazeuse("Anneaux en arcs (type Neptune)"));
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
    sys.ajouter(Box::new(Soleil::new(
        vec3(0.0, 0.0, 0.0),
        0.75,
        etoile::couleur_corps_noir(3040.0),
        0.06,
    )));

    // Apparences = presets nommés du catalogue.
    // Géante gazeuse proche et chaude (preset « Jupiter chaud »). Rayon réduit (0.85) pour ne pas
    // écraser la petite naine rouge ; ses lunes se dimensionnent sur ce rayon -> plus discrètes.
    let r_gg = 0.85;
    let gg = ajouter_planete(&mut sys, 0.2, 0.04, 2.0 * deg, r_gg, 8.0, preset_gazeuse("Jupiter chaud"));
    for _ in 0..2 {
        ajouter_lune(&mut sys, gg, r_gg);
    }
    // Planète rocheuse 1 : désert aride (preset « Desert »).
    ajouter_planete(&mut sys, 0.42, 0.1, 3.0 * deg, 0.34, 0.5, preset_tellurique("Desert"));
    // Planète rocheuse 2 : plus externe, froide, verrouillée par les marées -> monde « eyeball » gelé.
    ajouter_planete(&mut sys, 0.85, 0.07, 2.0 * deg, 0.4, 0.7, preset_tellurique("Eyeball gele"));

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
            .avec_atmo(vec3(0.35, 0.55, 1.0) * 0.9));
    ajouter_lune(&mut sys, terre, 0.55);

    // Planète TYPE P (circumbinaire, « Tatooine ») : orbite le BARYCENTRE du couple,
    // au-delà du rayon critique de stabilité (~18 UA pour ce couple) -> 24 UA.
    ajouter_planete_autour(&mut sys, Foyer::Barycentre, 2.0 * MASSE_ETOILE, 24.0, 0.05, 0.05, 0.7, 0.6,
        app_simple(TypePlanete::Tellurique, vec3(0.55, 0.45, 0.35), vec3(0.4, 0.3, 0.24), vec3(0.6, 0.75, 0.9), 0.15));

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
