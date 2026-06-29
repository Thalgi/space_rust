mod apparences;
mod persistance;
mod presets;

pub use persistance::{charger_presets, sauver_presets, PresetSauve};
pub use presets::{construire_preset_solaire, construire_preset_tau_ceti};

use crate::ceinture::{Ceinture, CeintureConfig};
use crate::etoile::{self, ProfilEtoile};
use crate::planete::{Apparence, Planete, TypePlanete};
use crate::soleil::Soleil;
use crate::systeme::{self, Systeme};
use apparences::{apparence_gazeuse, apparence_glacee, apparence_tellurique, gazeuse, tellurique};
use macroquad::prelude::*;
use macroquad::rand::{gen_range, srand};
use std::f32::consts::TAU;

pub const MASSE_ETOILE: f32 = 1000.0; // masse gravitationnelle (indépendante du rayon visuel)

/// Construit un système aléatoire à partir d'une graine. Renvoie aussi un texte d'info.
pub fn construire_systeme(seed: u64) -> (Systeme, String) {
    srand(seed);
    let profil = ProfilEtoile::aleatoire();

    let mut sys = Systeme::new();
    let soleil = Soleil::new(vec3(0.0, 0.0, 0.0), profil.rayon, profil.couleur, profil.luminosite);
    let soleil = match profil.couronne as i32 {
        1 => soleil.avec_jets(),
        2 => soleil.avec_vent(),
        3 => soleil.avec_pulsar(),
        4 => soleil.avec_magnetar(),
        _ => soleil,
    };
    sys.ajouter(Box::new(soleil));

    // Planètes : distances en UA suivant une loi de Titius-Bode.
    let nb: i32 = gen_range(3, 7);
    let mut a: f32 = gen_range(0.4, 0.8);
    for _ in 0..nb {
        let p: f32 = gen_range(0.0, 1.0);
        let type_p = if a < 2.0 {
            if p < 0.8 { TypePlanete::Tellurique } else { TypePlanete::Gazeuse }
        } else if a < 6.0 {
            if p < 0.5 {
                TypePlanete::Gazeuse
            } else if p < 0.85 {
                TypePlanete::Tellurique
            } else {
                TypePlanete::Glacee
            }
        } else if p < 0.55 {
            TypePlanete::Glacee
        } else if p < 0.8 {
            TypePlanete::Gazeuse
        } else {
            TypePlanete::Tellurique
        };

        let temp = etoile::temp_equilibre(profil.luminosite, a);
        let (rayon, masse, app) = match type_p {
            TypePlanete::Tellurique => apparence_tellurique(temp),
            TypePlanete::Gazeuse => apparence_gazeuse(),
            TypePlanete::Glacee => apparence_glacee(),
        };

        let e: f32 = gen_range(0.0, 0.2);
        let incl: f32 = gen_range(0.0, 0.16);
        let idx = ajouter_planete(&mut sys, a, e, incl, rayon, masse, app);

        let n_lunes: i32 = match type_p {
            TypePlanete::Gazeuse => gen_range(1, 4),
            TypePlanete::Tellurique => if a > 0.8 { gen_range(0, 2) } else { 0 },
            TypePlanete::Glacee => gen_range(0, 2),
        };
        for _ in 0..n_lunes {
            ajouter_lune(&mut sys, idx, rayon);
        }

        a *= gen_range(1.5_f32, 1.85);
    }

    // Ceinture principale + ceinture de Kuiper.
    let bi: f32 = gen_range(2.0, 3.0);
    let bo: f32 = bi + gen_range(0.6, 1.4);
    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::asteroides(
        900, bi * etoile::UA, bo * etoile::UA, MASSE_ETOILE,
    ))));
    let ki: f32 = bo + gen_range(20.0, 30.0);
    let ko: f32 = ki + gen_range(12.0, 20.0);
    sys.ajouter(Box::new(Ceinture::new(CeintureConfig::kuiper(
        1400, ki * etoile::UA, ko * etoile::UA, MASSE_ETOILE,
    ))));

    let info = format!("Etoile {}  -  {} K", profil.nom(), profil.temperature as i32);
    (sys, info)
}

/// Ajoute une planète d'éléments orbitaux donnés (demi-grand axe en UA). Renvoie son index.
pub(crate) fn ajouter_planete(
    sys: &mut Systeme,
    a_au: f32,
    e: f32,
    incl: f32,
    rayon: f32,
    masse: f32,
    app: Apparence,
) -> usize {
    let a_monde = a_au * etoile::UA;
    let phi: f32 = gen_range(0.0, TAU);
    let a1 = vec3(phi.cos(), 0.0, phi.sin());
    let a2 = vec3(-phi.sin(), 0.0, phi.cos());
    let q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();

    let r_p = a_monde * (1.0 - e);
    let v_p = (systeme::G * MASSE_ETOILE / a_monde * (1.0 + e) / (1.0 - e)).sqrt();
    let pos = a1 * r_p;
    let vel = q * v_p;

    let n = 96;
    let mut orbite = Vec::with_capacity(n);
    for k in 0..n {
        let nu = k as f32 / n as f32 * TAU;
        let rr = a_monde * (1.0 - e * e) / (1.0 + e * nu.cos());
        orbite.push(a1 * (rr * nu.cos()) + q * (rr * nu.sin()));
    }
    sys.ajouter(Box::new(Planete::new(pos, vel, rayon, masse, app, orbite)))
}

/// Ajoute une lune en orbite analytique autour de la planète d'index `parent`.
pub(crate) fn ajouter_lune(sys: &mut Systeme, parent: usize, rayon_planete: f32) {
    let r_orbite = rayon_planete * gen_range(2.5, 5.5);
    let sens = if gen_range(0.0_f32, 1.0) < 0.5 { 1.0 } else { -1.0 };
    let omega = sens * gen_range(0.5, 1.6);
    let incl: f32 = gen_range(-0.4, 0.4);
    let phase: f32 = gen_range(0.0, TAU);
    let rayon = rayon_planete * gen_range(0.16, 0.32);
    let app = if gen_range(0.0_f32, 1.0) < 0.5 {
        let g: f32 = gen_range(0.4, 0.7);
        app_simple(TypePlanete::Tellurique, vec3(g, g * 0.95, g * 0.9), vec3(g * 0.6, g * 0.58, g * 0.55), Vec3::ZERO, 0.0)
    } else {
        app_simple(TypePlanete::Glacee, vec3(0.72, 0.77, 0.85), vec3(0.55, 0.6, 0.7), Vec3::ZERO, 0.0)
    };
    let lune = Planete::new(Vec3::ZERO, Vec3::ZERO, rayon, 0.05, app, Vec::new())
        .en_lune(parent, r_orbite, omega, incl, phase);
    sys.ajouter(Box::new(lune));
}

pub(crate) fn app_simple(type_p: TypePlanete, c1: Vec3, c2: Vec3, c3: Vec3, eau: f32) -> Apparence {
    Apparence::new(type_p, c1, c2, c3, eau)
}

/// Catalogue des telluriques actuellement générables, pour la galerie de validation
/// visuelle. Aujourd'hui : un échantillon par bande de température. Cette liste
/// grandira au fur et à mesure qu'on ajoute des variantes/presets (étape 1+).
/// Presets considérés « rares » (affichés avec un [R] violet en galerie).
const RARES: &[&str] = &[
    "Megaflora", "Petrified", "Recif", "Archipelago", "Geothermal", "Bioluminescent",
    "Salines", "Aquifer", "Coral (aride)", "Primal", "Baobab", "Geoglyph", "Storm",
    "Iceberg", "Cryoflora", "Lichen", "Glaciovolcanic", "Lanthanide", "Eyeball humide",
    "Eyeball sec", "Eyeball gele", "Wet Superhabitable", "Dry Superhabitable",
    "Cold Superhabitable", "Pandora", "Polyphemus (Avatar)",
];

/// Un preset est-il rare ?
pub fn est_rare(nom: &str) -> bool {
    RARES.contains(&nom)
}

pub fn catalogue_telluriques() -> Vec<(String, Apparence)> {
    let bleu = vec3(0.35, 0.55, 1.0) * 0.9; // atmosphère océanique
    let voile = vec3(0.6, 0.8, 1.0) * 0.2; // voile glacé
    let sec = vec3(0.8, 0.7, 0.5) * 0.12; // atmosphère sèche légère
    let z = Vec3::ZERO;

    let mut v: Vec<(String, Apparence)> = Vec::new();
    // Chaque preset reçoit une graine aléatoire -> géographie unique, et la touche G
    // (qui change la graine de la galerie) régénère des cartes différentes.
    let mut push = |nom: &str, mut app: Apparence| {
        app.seed = gen_range(0.0, 1000.0);
        v.push((nom.to_string(), app));
    };

    // --- Humide ---
    let blanc = vec3(1.0, 1.0, 1.0);
    push("Forest", tellurique(vec3(0.45, 0.4, 0.3), vec3(0.35, 0.3, 0.24), vec3(0.1, 0.32, 0.7), 0.55, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.16, 0.55, 0.16), 0.98).avec_rivieres(0.4).avec_nuages(0.4, blanc));
    push("Monde-ocean", tellurique(vec3(0.2, 0.45, 0.3), vec3(0.25, 0.3, 0.25), vec3(0.06, 0.3, 0.62), 0.92, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_nuages(0.5, blanc));
    push("Lacs", tellurique(vec3(0.42, 0.4, 0.28), vec3(0.36, 0.32, 0.24), vec3(0.15, 0.4, 0.7), 0.42, 3.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.52, 0.2), 0.7));
    push("Marais", tellurique(vec3(0.34, 0.36, 0.2), vec3(0.28, 0.26, 0.16), vec3(0.3, 0.42, 0.35), 0.4, 3.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.3, 0.42, 0.2), 0.7));
    push("Algues roses", tellurique(vec3(0.3, 0.45, 0.25), vec3(0.3, 0.3, 0.22), vec3(0.85, 0.35, 0.6), 0.8, 0.0, 0.4, 0.85, bleu));
    push("Recif", tellurique(vec3(0.3, 0.5, 0.3), vec3(0.3, 0.32, 0.24), vec3(0.1, 0.45, 0.7), 0.85, 0.0, 0.4, 0.85, bleu).avec_recifs(0.9).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.4));
    push("Columnar", tellurique(vec3(0.36, 0.37, 0.4), vec3(0.22, 0.23, 0.26), vec3(0.1, 0.32, 0.6), 0.55, 0.0, 0.4, 0.85, bleu).avec_basalt(0.9));

    // --- Végétation colorée (teinte de végétation) ---
    push("Sakura", tellurique(vec3(0.5, 0.42, 0.34), vec3(0.38, 0.3, 0.26), vec3(0.12, 0.34, 0.66), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.95, 0.55, 0.75), 0.85));
    push("Retinal", tellurique(vec3(0.4, 0.36, 0.34), vec3(0.3, 0.26, 0.26), vec3(0.1, 0.3, 0.62), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.55, 0.25, 0.7), 0.85));
    push("Carotene", tellurique(vec3(0.5, 0.42, 0.3), vec3(0.4, 0.32, 0.22), vec3(0.12, 0.34, 0.6), 0.45, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.88, 0.55, 0.12), 0.85));
    push("Mousse", tellurique(vec3(0.3, 0.34, 0.24), vec3(0.26, 0.28, 0.2), vec3(0.15, 0.36, 0.5), 0.35, 1.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.26, 0.62, 0.22), 1.0));
    push("Bioluminescent", tellurique(vec3(0.12, 0.2, 0.18), vec3(0.08, 0.14, 0.13), vec3(0.04, 0.18, 0.28), 0.6, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.1, 0.3, 0.22), 0.6).avec_biolum(0.95).avec_nuages(0.3, vec3(0.5, 0.6, 0.7)));

    // --- Météo (couche nuageuse) ---
    push("Brumeux", tellurique(vec3(0.4, 0.45, 0.4), vec3(0.32, 0.35, 0.32), vec3(0.2, 0.4, 0.55), 0.6, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.25, 0.45, 0.25), 0.6).avec_nuages(0.85, blanc));
    push("Orageux", tellurique(vec3(0.3, 0.42, 0.26), vec3(0.26, 0.32, 0.22), vec3(0.08, 0.3, 0.55), 0.75, 0.0, 0.35, 0.88, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.7).avec_meteo(0.85, vec3(0.5, 0.52, 0.58), 1.0));
    push("Cyclone", tellurique(vec3(0.25, 0.42, 0.3), vec3(0.26, 0.3, 0.24), vec3(0.06, 0.3, 0.6), 0.78, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_meteo(0.8, blanc, 2.0));
    push("Tempete de poussiere", tellurique(vec3(0.78, 0.58, 0.34), vec3(0.55, 0.4, 0.24), z, 0.0, 1.0, 0.2, 0.95, sec).avec_meteo(0.6, vec3(0.78, 0.62, 0.4), 1.0));

    // --- Sec ---
    push("Desert", tellurique(vec3(0.8, 0.6, 0.35), vec3(0.55, 0.4, 0.25), vec3(0.4, 0.5, 0.6), 0.04, 2.0, 0.25, 0.95, sec).avec_dunes(0.6));
    push("Dune (erg)", tellurique(vec3(0.85, 0.66, 0.38), vec3(0.62, 0.45, 0.26), z, 0.0, 1.0, 0.15, 0.97, sec).avec_relief(0.15).avec_dunes(0.9));
    push("Sable ferreux", tellurique(vec3(0.7, 0.3, 0.2), vec3(0.5, 0.22, 0.16), z, 0.0, 1.0, 0.2, 0.95, sec).avec_dunes(0.55));
    push("Mediterraneenne", tellurique(vec3(0.6, 0.55, 0.3), vec3(0.45, 0.4, 0.24), vec3(0.15, 0.4, 0.6), 0.42, 2.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.35, 0.5, 0.2), 0.45).avec_rivieres(0.7));
    push("Salines", tellurique(vec3(0.85, 0.82, 0.78), vec3(0.6, 0.55, 0.5), z, 0.0, 1.0, 0.2, 0.95, sec));
    push("Mesa", tellurique(vec3(0.72, 0.42, 0.28), vec3(0.5, 0.3, 0.22), vec3(0.2, 0.45, 0.55), 0.12, 2.0, 0.3, 0.9, sec).avec_relief(0.5).avec_mesa(0.8).avec_rivieres(0.5).avec_vegetation(vec3(0.3, 0.45, 0.2), 0.3));
    push("Striped", tellurique(vec3(0.72, 0.5, 0.3), vec3(0.32, 0.46, 0.7), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.95));

    // --- Froid ---
    push("Toundra", tellurique(vec3(0.4, 0.42, 0.32), vec3(0.3, 0.3, 0.26), vec3(0.6, 0.7, 0.8), 0.15, 1.0, 0.6, 0.6, voile));
    push("Arctique", tellurique(vec3(0.55, 0.6, 0.62), vec3(0.4, 0.45, 0.5), vec3(0.5, 0.6, 0.75), 0.2, 1.0, 0.7, 0.4, voile));
    push("Boule de neige", tellurique(vec3(0.85, 0.9, 0.96), vec3(0.7, 0.78, 0.88), z, 0.0, 1.0, 0.7, 0.15, voile).avec_pics(0.5));
    push("Alpin", tellurique(vec3(0.42, 0.4, 0.34), vec3(0.34, 0.32, 0.3), vec3(0.2, 0.4, 0.6), 0.25, 1.0, 0.55, 0.7, voile).avec_vegetation(vec3(0.18, 0.42, 0.2), 0.6).avec_relief(0.9).avec_nuages(0.25, blanc));
    push("Pics de glace", tellurique(vec3(0.78, 0.85, 0.95), vec3(0.62, 0.72, 0.85), z, 0.0, 1.0, 0.7, 0.3, voile).avec_relief(0.4).avec_pics(0.9));
    // Combo : montagnes + dunes en plaine + forêts en altitude (features cumulées).
    push("Dune Forest", tellurique(vec3(0.62, 0.52, 0.32), vec3(0.42, 0.36, 0.24), vec3(0.15, 0.4, 0.6), 0.18, 1.0, 0.4, 0.7, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.75).avec_relief(0.75).avec_dunes(0.7).avec_nuages(0.25, blanc));
    let mut cryovolc = tellurique(vec3(0.4, 0.44, 0.4), vec3(0.3, 0.32, 0.3), vec3(0.4, 0.55, 0.72), 0.12, 1.0, 0.5, 0.45, voile).avec_vegetation(vec3(0.32, 0.42, 0.3), 0.4).avec_relief(0.55).avec_mesa(0.4).avec_rivieres(0.4).avec_riv_lave();
    cryovolc.lave = 0.25; // chaleur volcanique sous la glace + toundra
    push("Cryovolcan", cryovolc);

    // --- Extrêmes / exotiques ---
    let mut lave = tellurique(vec3(0.16, 0.10, 0.08), vec3(0.07, 0.05, 0.04), z, 0.0, 1.0, 0.0, 1.0, z);
    lave.lave = 1.0;
    lave.relief = 0.0;
    push("Lave", lave);
    push("Venus (etuve)", tellurique(vec3(0.7, 0.6, 0.4), vec3(0.5, 0.42, 0.28), z, 0.0, 1.0, 0.15, 1.0, vec3(0.8, 0.7, 0.3) * 0.25).avec_voile(0.93, vec3(0.93, 0.82, 0.5)));
    push("Titan", tellurique(vec3(0.6, 0.45, 0.25), vec3(0.45, 0.32, 0.2), vec3(0.5, 0.35, 0.15), 0.25, 3.0, 0.2, 0.9, z).avec_voile(0.78, vec3(0.85, 0.55, 0.25)));
    push("Fer (Mercure)", tellurique(vec3(0.5, 0.48, 0.45), vec3(0.32, 0.3, 0.28), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.4).avec_crateres(0.85));
    push("Carbone", tellurique(vec3(0.12, 0.12, 0.14), vec3(0.06, 0.06, 0.07), z, 0.0, 1.0, 0.0, 1.0, z).avec_crateres(0.4));
    push("Lune", tellurique(vec3(0.55, 0.54, 0.52), vec3(0.34, 0.33, 0.32), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.4).avec_crateres(0.95));
    push("Diamant", tellurique(vec3(0.22, 0.26, 0.34), vec3(0.1, 0.12, 0.17), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.5).avec_basalt(0.6));
    push("Subglaciaire", tellurique(vec3(0.82, 0.88, 0.95), vec3(0.7, 0.64, 0.62), z, 0.0, 1.0, 0.2, 0.05, voile).avec_relief(0.2));
    let mut chth = tellurique(vec3(0.32, 0.13, 0.1), vec3(0.18, 0.08, 0.07), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.6);
    chth.lave = 0.5;
    push("Chthonien", chth);
    let mut ash = tellurique(vec3(0.3, 0.28, 0.27), vec3(0.16, 0.15, 0.14), z, 0.0, 1.0, 0.1, 1.0, z).avec_dunes(0.4);
    ash.lave = 0.3;
    push("Ash (volcanique)", ash);

    // --- Première passe de variantes nommées (presets de paramètres) ---
    // Monde type Terre : océans + continents verts + déserts (zones sèches) + calottes
    // polaires + montagnes + rivières + nuages -> toute la diversité de notre planète.
    push("Terre", tellurique(vec3(0.5, 0.42, 0.3), vec3(0.4, 0.34, 0.26), vec3(0.07, 0.32, 0.7), 0.62, 1.0, 0.5, 0.8, bleu).avec_vegetation(vec3(0.16, 0.5, 0.18), 0.82).avec_relief(0.65).avec_rivieres(0.55).avec_dunes(0.3).avec_nuages(0.42, blanc));

    // Mélange de styles (un seul preset cumule plusieurs axes) : lune type Pandora.
    push("Pandora", tellurique(vec3(0.2, 0.4, 0.3), vec3(0.22, 0.3, 0.26), vec3(0.06, 0.34, 0.55), 0.7, 0.0, 0.42, 0.9, bleu).avec_vegetation(vec3(0.14, 0.52, 0.32), 0.95).avec_relief(0.85).avec_rivieres(0.4).avec_biolum(0.9).avec_nuages(0.4, blanc));

    // Humide
    push("Mushroom", tellurique(vec3(0.4, 0.36, 0.3), vec3(0.3, 0.27, 0.24), vec3(0.12, 0.34, 0.6), 0.45, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.45, 0.3, 0.45), 0.85).avec_nuages(0.3, blanc));
    push("Tepid", tellurique(vec3(0.45, 0.42, 0.28), vec3(0.34, 0.3, 0.22), vec3(0.1, 0.36, 0.66), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.5, 0.62, 0.15), 0.9).avec_rivieres(0.4));
    push("Swamp", tellurique(vec3(0.34, 0.36, 0.22), vec3(0.26, 0.26, 0.18), vec3(0.2, 0.36, 0.32), 0.55, 3.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.28, 0.4, 0.2), 0.75).avec_nuages(0.4, blanc));
    push("Kelp", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.1, 0.4, 0.4), 0.85, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.4));
    push("Tidepool", tellurique(vec3(0.42, 0.42, 0.36), vec3(0.32, 0.3, 0.26), vec3(0.12, 0.42, 0.55), 0.6, 1.0, 0.4, 0.85, bleu).avec_relief(0.5).avec_vegetation(vec3(0.25, 0.45, 0.25), 0.4));
    // Tropical
    push("Atoll", tellurique(vec3(0.3, 0.5, 0.3), vec3(0.3, 0.32, 0.24), vec3(0.1, 0.5, 0.65), 0.82, 0.0, 0.35, 0.95, bleu).avec_recifs(0.85).avec_vegetation(vec3(0.2, 0.55, 0.2), 0.5).avec_nuages(0.4, blanc));
    push("Mangrove", tellurique(vec3(0.32, 0.4, 0.24), vec3(0.26, 0.3, 0.2), vec3(0.15, 0.42, 0.45), 0.7, 3.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.2, 0.45, 0.2), 0.7).avec_nuages(0.45, blanc));
    // Savane / aride
    push("Steppe", tellurique(vec3(0.7, 0.62, 0.32), vec3(0.5, 0.44, 0.26), z, 0.06, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.55, 0.55, 0.2), 0.5));
    push("Veldt", tellurique(vec3(0.65, 0.55, 0.3), vec3(0.46, 0.4, 0.24), z, 0.05, 1.0, 0.3, 0.92, sec).avec_relief(0.5).avec_vegetation(vec3(0.5, 0.52, 0.22), 0.45));
    push("Acacia", tellurique(vec3(0.6, 0.55, 0.3), vec3(0.44, 0.4, 0.24), vec3(0.2, 0.4, 0.5), 0.12, 2.0, 0.32, 0.9, sec).avec_vegetation(vec3(0.4, 0.5, 0.2), 0.5).avec_rivieres(0.4));
    push("Badlands", tellurique(vec3(0.7, 0.4, 0.28), vec3(0.5, 0.28, 0.2), z, 0.0, 1.0, 0.25, 0.95, sec).avec_mesa(0.7).avec_relief(0.4));
    push("Fog Desert", tellurique(vec3(0.72, 0.62, 0.42), vec3(0.52, 0.42, 0.3), z, 0.02, 2.0, 0.25, 0.95, sec).avec_dunes(0.4).avec_nuages(0.7, vec3(0.9, 0.9, 0.92)));
    push("Amethyst", tellurique(vec3(0.5, 0.35, 0.6), vec3(0.32, 0.22, 0.4), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.5).avec_relief(0.4));
    push("Cactus", tellurique(vec3(0.75, 0.6, 0.35), vec3(0.55, 0.42, 0.26), z, 0.03, 2.0, 0.25, 0.95, sec).avec_dunes(0.3).avec_vegetation(vec3(0.3, 0.5, 0.25), 0.25));
    // Froid
    push("Boreal", tellurique(vec3(0.35, 0.4, 0.3), vec3(0.3, 0.32, 0.28), vec3(0.2, 0.4, 0.55), 0.25, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.16, 0.4, 0.22), 0.6).avec_relief(0.5));
    push("Taiga", tellurique(vec3(0.32, 0.38, 0.3), vec3(0.28, 0.3, 0.26), vec3(0.18, 0.38, 0.52), 0.2, 1.0, 0.55, 0.5, voile).avec_vegetation(vec3(0.15, 0.36, 0.2), 0.55).avec_relief(0.7).avec_nuages(0.3, blanc));
    push("Fjord", tellurique(vec3(0.4, 0.44, 0.38), vec3(0.32, 0.34, 0.3), vec3(0.15, 0.4, 0.55), 0.35, 1.0, 0.55, 0.5, voile).avec_relief(0.85).avec_vegetation(vec3(0.18, 0.42, 0.24), 0.45));
    push("Glacial", tellurique(vec3(0.6, 0.66, 0.72), vec3(0.45, 0.5, 0.56), vec3(0.4, 0.55, 0.7), 0.2, 1.0, 0.7, 0.4, voile).avec_pics(0.3));
    push("Cold Desert", tellurique(vec3(0.62, 0.6, 0.58), vec3(0.45, 0.43, 0.42), z, 0.0, 1.0, 0.6, 0.45, voile).avec_dunes(0.3));
    push("Ice Dunes", tellurique(vec3(0.8, 0.85, 0.92), vec3(0.62, 0.68, 0.78), z, 0.0, 1.0, 0.7, 0.25, voile).avec_dunes(0.7));
    push("Bog", tellurique(vec3(0.36, 0.38, 0.26), vec3(0.28, 0.28, 0.22), vec3(0.22, 0.34, 0.3), 0.4, 3.0, 0.6, 0.6, voile).avec_vegetation(vec3(0.3, 0.4, 0.22), 0.5));

    // --- Deuxième passe de variantes nommées ---
    // Continental
    push("Megaflora", tellurique(vec3(0.3, 0.34, 0.24), vec3(0.24, 0.26, 0.2), vec3(0.1, 0.32, 0.6), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.1, 0.4, 0.14), 1.0).avec_relief(0.55).avec_nuages(0.4, blanc));
    push("Petrified", tellurique(vec3(0.6, 0.55, 0.45), vec3(0.42, 0.38, 0.32), z, 0.0, 1.0, 0.3, 0.9, sec).avec_mesa(0.4).avec_relief(0.4));
    // Océan
    push("Cascadian", tellurique(vec3(0.35, 0.42, 0.3), vec3(0.3, 0.32, 0.26), vec3(0.1, 0.34, 0.55), 0.6, 1.0, 0.45, 0.7, bleu).avec_vegetation(vec3(0.16, 0.42, 0.2), 0.7).avec_relief(0.8).avec_nuages(0.6, vec3(0.8, 0.82, 0.85)));
    push("Archipelago", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.06, 0.32, 0.6), 0.88, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_relief(0.4));
    push("Crag", tellurique(vec3(0.45, 0.45, 0.42), vec3(0.34, 0.34, 0.32), vec3(0.1, 0.36, 0.55), 0.7, 1.0, 0.55, 0.5, voile).avec_relief(0.7));
    // Tropical
    push("Geothermal", tellurique(vec3(0.5, 0.55, 0.55), vec3(0.4, 0.44, 0.46), vec3(0.3, 0.5, 0.6), 0.15, 1.0, 0.55, 0.45, voile).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.35).avec_relief(0.5).avec_rivieres(0.35).avec_riv_lave());
    push("Tepui", tellurique(vec3(0.4, 0.42, 0.28), vec3(0.32, 0.3, 0.22), vec3(0.1, 0.4, 0.6), 0.4, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.16, 0.45, 0.2), 0.7).avec_mesa(0.6).avec_relief(0.5).avec_nuages(0.5, blanc));
    push("Lilypad", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.2, 0.5, 0.3), 0.85, 0.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.25, 0.55, 0.25), 0.45));
    push("Obsidian", tellurique(vec3(0.25, 0.4, 0.3), vec3(0.08, 0.08, 0.1), vec3(0.1, 0.4, 0.55), 0.55, 1.0, 0.35, 0.88, bleu).avec_vegetation(vec3(0.2, 0.5, 0.22), 0.5).avec_basalt(0.5));
    // Désert / aride
    push("Oasis", tellurique(vec3(0.82, 0.64, 0.36), vec3(0.58, 0.42, 0.26), vec3(0.1, 0.45, 0.6), 0.06, 2.0, 0.25, 0.95, sec).avec_dunes(0.5).avec_vegetation(vec3(0.25, 0.5, 0.22), 0.3).avec_rivieres(0.3));
    push("Outback", tellurique(vec3(0.7, 0.45, 0.3), vec3(0.5, 0.32, 0.22), z, 0.03, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.45, 0.45, 0.2), 0.3).avec_dunes(0.3));
    push("Coastal", tellurique(vec3(0.7, 0.6, 0.4), vec3(0.5, 0.42, 0.3), vec3(0.12, 0.4, 0.6), 0.45, 1.0, 0.3, 0.9, bleu).avec_vegetation(vec3(0.35, 0.5, 0.25), 0.3).avec_nuages(0.6, vec3(0.9, 0.9, 0.92)));
    push("Sodalite", tellurique(vec3(0.6, 0.55, 0.4), vec3(0.25, 0.35, 0.62), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.9));
    push("Coral (aride)", tellurique(vec3(0.75, 0.5, 0.4), vec3(0.55, 0.35, 0.3), vec3(0.2, 0.55, 0.6), 0.15, 2.0, 0.3, 0.92, sec).avec_recifs(0.7).avec_vegetation(vec3(0.4, 0.5, 0.25), 0.2));
    push("Opal", tellurique(vec3(0.7, 0.8, 0.85), vec3(0.55, 0.6, 0.7), z, 0.0, 1.0, 0.2, 0.92, sec).avec_mesa(0.6));
    push("Superbloom", tellurique(vec3(0.75, 0.55, 0.35), vec3(0.55, 0.4, 0.26), z, 0.02, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.85, 0.4, 0.55), 0.4));
    let mut primal = tellurique(vec3(0.35, 0.22, 0.18), vec3(0.2, 0.12, 0.1), z, 0.0, 1.0, 0.2, 0.95, z).avec_relief(0.5);
    primal.lave = 0.4;
    push("Primal", primal);
    // Savane
    push("Pampa", tellurique(vec3(0.65, 0.62, 0.35), vec3(0.48, 0.44, 0.28), vec3(0.15, 0.4, 0.55), 0.1, 2.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.5, 0.6, 0.2), 0.6).avec_rivieres(0.3));
    push("Heath", tellurique(vec3(0.5, 0.42, 0.4), vec3(0.36, 0.3, 0.3), z, 0.05, 1.0, 0.4, 0.85, voile).avec_vegetation(vec3(0.4, 0.3, 0.4), 0.5));
    push("Bushveld", tellurique(vec3(0.6, 0.55, 0.32), vec3(0.44, 0.4, 0.24), vec3(0.15, 0.4, 0.5), 0.1, 2.0, 0.35, 0.9, sec).avec_vegetation(vec3(0.3, 0.48, 0.2), 0.55).avec_relief(0.4));
    // Arctique
    push("Antarctic", tellurique(vec3(0.8, 0.86, 0.92), vec3(0.62, 0.68, 0.78), z, 0.0, 1.0, 0.75, 0.25, voile).avec_pics(0.4).avec_relief(0.4));
    push("Iceberg", tellurique(vec3(0.75, 0.82, 0.9), vec3(0.55, 0.62, 0.72), vec3(0.3, 0.5, 0.7), 0.5, 0.0, 0.7, 0.4, voile).avec_pics(0.3));
    push("Storm", tellurique(vec3(0.45, 0.5, 0.55), vec3(0.35, 0.4, 0.45), vec3(0.1, 0.3, 0.5), 0.8, 0.0, 0.6, 0.45, voile).avec_meteo(0.85, vec3(0.55, 0.57, 0.63), 1.0));
    // Toundra
    let mut mud = tellurique(vec3(0.42, 0.36, 0.26), vec3(0.3, 0.25, 0.18), vec3(0.3, 0.28, 0.22), 0.4, 3.0, 0.55, 0.6, voile).avec_vegetation(vec3(0.35, 0.38, 0.22), 0.4);
    mud.lave = 0.2;
    push("Mud", mud);
    push("Travertine", tellurique(vec3(0.88, 0.85, 0.78), vec3(0.6, 0.58, 0.52), vec3(0.3, 0.5, 0.55), 0.1, 2.0, 0.5, 0.55, voile).avec_mesa(0.85));
    push("Lichen", tellurique(vec3(0.5, 0.52, 0.42), vec3(0.38, 0.4, 0.34), z, 0.0, 1.0, 0.55, 0.5, voile).avec_vegetation(vec3(0.45, 0.5, 0.3), 0.45).avec_relief(0.6));
    push("Cryoflora", tellurique(vec3(0.6, 0.7, 0.78), vec3(0.45, 0.55, 0.65), z, 0.05, 1.0, 0.0, 0.4, voile).avec_vegetation(vec3(0.0, 0.9, 1.0), 0.4).avec_rivieres(0.4).avec_biolum(0.7));
    // Alpin
    push("Highland", tellurique(vec3(0.45, 0.46, 0.4), vec3(0.34, 0.36, 0.32), vec3(0.15, 0.4, 0.5), 0.2, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.3, 0.42, 0.25), 0.45).avec_relief(0.6).avec_nuages(0.7, vec3(0.85, 0.87, 0.9)));
    push("Snow", tellurique(vec3(0.7, 0.76, 0.84), vec3(0.55, 0.6, 0.7), vec3(0.2, 0.4, 0.6), 0.15, 1.0, 0.65, 0.35, voile).avec_vegetation(vec3(0.16, 0.4, 0.22), 0.5));
    push("Blossom", tellurique(vec3(0.5, 0.5, 0.42), vec3(0.38, 0.38, 0.32), vec3(0.15, 0.4, 0.55), 0.2, 1.0, 0.5, 0.5, voile).avec_vegetation(vec3(0.8, 0.5, 0.7), 0.6).avec_relief(0.5));
    let mut glaciovolc = tellurique(vec3(0.55, 0.62, 0.7), vec3(0.42, 0.48, 0.56), vec3(0.3, 0.5, 0.6), 0.12, 1.0, 0.55, 0.4, voile).avec_relief(0.6).avec_mesa(0.6).avec_rivieres(0.5).avec_riv_lave();
    glaciovolc.lave = 0.35; // lave qui transparaît sous la glace fracturée
    push("Glaciovolcanic", glaciovolc);
    // Gaia / superhabitables
    push("Dry Gaia", tellurique(vec3(0.7, 0.6, 0.35), vec3(0.5, 0.42, 0.26), vec3(0.12, 0.45, 0.65), 0.35, 1.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.4, 0.55, 0.2), 0.7).avec_rivieres(0.4).avec_nuages(0.3, blanc));
    push("Cold Gaia", tellurique(vec3(0.4, 0.45, 0.36), vec3(0.32, 0.36, 0.3), vec3(0.12, 0.4, 0.6), 0.4, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.7).avec_relief(0.5).avec_nuages(0.3, blanc));
    push("Wet Superhabitable", tellurique(vec3(0.3, 0.5, 0.28), vec3(0.3, 0.34, 0.24), vec3(0.08, 0.4, 0.7), 0.6, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.16, 0.55, 0.18), 0.85).avec_rivieres(0.5).avec_nuages(0.4, blanc));

    // --- Troisième passe (variantes restantes + flavor) ---
    push("Barnacle", tellurique(vec3(0.5, 0.48, 0.42), vec3(0.36, 0.34, 0.3), vec3(0.1, 0.36, 0.55), 0.8, 0.0, 0.4, 0.85, bleu).avec_relief(0.5).avec_basalt(0.3));
    push("Cenote", tellurique(vec3(0.4, 0.5, 0.3), vec3(0.32, 0.34, 0.24), vec3(0.12, 0.45, 0.6), 0.25, 3.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.18, 0.5, 0.2), 0.7));
    push("Fungal", tellurique(vec3(0.4, 0.36, 0.34), vec3(0.3, 0.26, 0.26), vec3(0.12, 0.4, 0.55), 0.4, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.5, 0.3, 0.45), 0.8).avec_nuages(0.3, blanc));
    push("Aerial", tellurique(vec3(0.3, 0.46, 0.3), vec3(0.28, 0.32, 0.24), vec3(0.1, 0.4, 0.62), 0.5, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.18, 0.52, 0.2), 0.8).avec_nuages(0.6, blanc));
    push("Aquifer", tellurique(vec3(0.82, 0.62, 0.36), vec3(0.58, 0.42, 0.26), vec3(0.1, 0.45, 0.6), 0.05, 2.0, 0.25, 0.95, sec).avec_dunes(0.6).avec_vegetation(vec3(0.3, 0.5, 0.22), 0.2));
    push("Fungi", tellurique(vec3(0.42, 0.4, 0.38), vec3(0.28, 0.27, 0.26), z, 0.0, 1.0, 0.25, 0.92, sec).avec_vegetation(vec3(0.45, 0.4, 0.3), 0.35).avec_dunes(0.3));
    push("Succulent", tellurique(vec3(0.7, 0.58, 0.36), vec3(0.5, 0.4, 0.26), z, 0.02, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.35, 0.52, 0.28), 0.35));
    push("Baobab", tellurique(vec3(0.66, 0.58, 0.32), vec3(0.48, 0.42, 0.26), vec3(0.15, 0.4, 0.5), 0.08, 2.0, 0.32, 0.9, sec).avec_vegetation(vec3(0.4, 0.5, 0.2), 0.4).avec_relief(0.3));
    push("Scrubland", tellurique(vec3(0.68, 0.56, 0.34), vec3(0.5, 0.4, 0.26), z, 0.04, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.45, 0.45, 0.24), 0.4));
    push("Geoglyph", tellurique(vec3(0.7, 0.6, 0.36), vec3(0.5, 0.42, 0.26), z, 0.05, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.5, 0.52, 0.22), 0.45).avec_mesa(0.3));
    push("Termite", tellurique(vec3(0.66, 0.5, 0.3), vec3(0.48, 0.36, 0.22), z, 0.04, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.45, 0.46, 0.22), 0.4).avec_dunes(0.25));
    push("Amber", tellurique(vec3(0.7, 0.55, 0.28), vec3(0.5, 0.38, 0.2), vec3(0.2, 0.4, 0.45), 0.1, 2.0, 0.32, 0.9, vec3(0.9, 0.6, 0.2) * 0.3).avec_vegetation(vec3(0.7, 0.5, 0.15), 0.55));
    push("Aeolian", tellurique(vec3(0.6, 0.62, 0.68), vec3(0.45, 0.47, 0.52), z, 0.0, 1.0, 0.6, 0.45, voile).avec_mesa(0.6).avec_relief(0.6));
    push("Supraglacial", tellurique(vec3(0.78, 0.84, 0.92), vec3(0.6, 0.66, 0.76), vec3(0.35, 0.55, 0.72), 0.35, 3.0, 0.7, 0.3, voile));
    let mut crevasse = tellurique(vec3(0.62, 0.72, 0.85), vec3(0.4, 0.5, 0.66), z, 0.0, 1.0, 0.6, 0.4, voile).avec_mesa(0.85).avec_relief(0.6).avec_rivieres(0.55).avec_riv_lave();
    crevasse.lave = 0.3; // chaleur géothermique qui transparaît au fond des crevasses
    push("Crevasse", crevasse);
    push("Ferrosprings", tellurique(vec3(0.72, 0.78, 0.86), vec3(0.55, 0.6, 0.7), vec3(0.7, 0.25, 0.15), 0.18, 3.0, 0.65, 0.4, voile).avec_rivieres(0.45));
    push("Tuya", tellurique(vec3(0.5, 0.55, 0.6), vec3(0.4, 0.44, 0.48), z, 0.0, 1.0, 0.6, 0.45, voile).avec_relief(0.6).avec_rivieres(0.35).avec_riv_lave());
    push("Peatland", tellurique(vec3(0.4, 0.38, 0.28), vec3(0.3, 0.28, 0.22), vec3(0.25, 0.32, 0.28), 0.35, 3.0, 0.55, 0.6, voile).avec_vegetation(vec3(0.34, 0.4, 0.24), 0.5));
    push("Treeline", tellurique(vec3(0.5, 0.55, 0.6), vec3(0.4, 0.44, 0.48), vec3(0.2, 0.4, 0.55), 0.2, 1.0, 0.7, 0.35, voile).avec_vegetation(vec3(0.16, 0.45, 0.22), 0.8));
    push("Lanthanide", tellurique(vec3(0.55, 0.5, 0.5), vec3(0.36, 0.33, 0.36), z, 0.0, 1.0, 0.3, 0.9, z).avec_relief(0.85).avec_crateres(0.3));
    push("Ravine", tellurique(vec3(0.5, 0.46, 0.4), vec3(0.38, 0.34, 0.3), vec3(0.15, 0.4, 0.55), 0.25, 1.0, 0.5, 0.55, voile).avec_mesa(0.6).avec_rivieres(0.5).avec_relief(0.6).avec_vegetation(vec3(0.2, 0.42, 0.24), 0.4));
    push("Craton", tellurique(vec3(0.65, 0.58, 0.42), vec3(0.48, 0.42, 0.3), z, 0.05, 2.0, 0.35, 0.9, sec).avec_mesa(0.4));
    push("Dry Superhabitable", tellurique(vec3(0.68, 0.6, 0.34), vec3(0.5, 0.42, 0.26), vec3(0.12, 0.45, 0.65), 0.4, 1.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.4, 0.55, 0.2), 0.75).avec_rivieres(0.5).avec_nuages(0.3, blanc));
    push("Cold Superhabitable", tellurique(vec3(0.4, 0.46, 0.36), vec3(0.32, 0.36, 0.3), vec3(0.12, 0.4, 0.6), 0.45, 1.0, 0.5, 0.5, voile).avec_vegetation(vec3(0.2, 0.52, 0.26), 0.75).avec_relief(0.5).avec_nuages(0.3, blanc));
    // Mondes-grottes (flavor : rendus comme leur famille parente)
    push("Wet Cave", tellurique(vec3(0.4, 0.42, 0.3), vec3(0.3, 0.3, 0.24), vec3(0.1, 0.34, 0.55), 0.4, 1.0, 0.4, 0.8, bleu).avec_vegetation(vec3(0.2, 0.45, 0.22), 0.5).avec_relief(0.5));
    push("Dry Cave", tellurique(vec3(0.7, 0.55, 0.34), vec3(0.5, 0.38, 0.24), z, 0.02, 2.0, 0.3, 0.92, sec).avec_dunes(0.3).avec_relief(0.5));
    push("Cold Cave", tellurique(vec3(0.6, 0.64, 0.7), vec3(0.46, 0.5, 0.56), z, 0.0, 1.0, 0.6, 0.4, voile).avec_relief(0.5).avec_pics(0.3));
    // Soufre / Io
    let mut io = tellurique(vec3(0.85, 0.7, 0.2), vec3(0.6, 0.4, 0.1), z, 0.0, 1.0, 0.1, 1.0, vec3(0.9, 0.8, 0.3) * 0.2).avec_dunes(0.2);
    io.lave = 0.5;
    push("Io (soufre)", io);

    // --- Verrouillées par marée (eyeball : jour brûlé / nuit gelée) ---
    // Gelé : la zone subsolaire reste une forêt avec étendues d'eau, le reste gèle.
    push("Eyeball gele", tellurique(vec3(0.3, 0.5, 0.28), vec3(0.32, 0.34, 0.24), vec3(0.08, 0.4, 0.7), 0.4, 1.0, 0.0, 1.0, bleu).avec_vegetation(vec3(0.18, 0.5, 0.2), 0.75).avec_eyeball_zones(0.35, -0.1, 0.0));
    // Sec : subsolaire en lave/obsidienne, anneau de forêt au terminateur, désert, puis glace.
    push("Eyeball sec", tellurique(vec3(0.72, 0.56, 0.34), vec3(0.5, 0.38, 0.24), z, 0.0, 1.0, 0.0, 1.0, sec).avec_dunes(0.4).avec_eyeball_zones(-0.05, 1.0, 0.0));
    // Humide : subsolaire desséché en désert, zone moins exposée en calotte glaciaire.
    push("Eyeball humide", tellurique(vec3(0.78, 0.6, 0.36), vec3(0.32, 0.34, 0.24), vec3(0.08, 0.4, 0.7), 0.4, 3.0, 0.0, 1.0, bleu).avec_relief(0.85).avec_dunes(0.0).avec_eyeball_zones(-0.05,0.0, 2.0));
    // eyeball archipel
    push("Eyeball Archipelago", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.06, 0.32, 0.6), 0.88, 3.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_relief(0.9).avec_eyeball_zones(0.35, -0.1, 0.0));


    v
}

/// Catalogue des géantes gazeuses pour la galerie dédiée. Basé sur la classification
/// de Sudarsky (classes I–V par température/nuages) + géantes de glace + naines brunes.
pub fn catalogue_gazeuses() -> Vec<(String, Apparence)> {
    let mut v: Vec<(String, Apparence)> = Vec::new();
    let mut push = |nom: &str, mut app: Apparence| {
        app.seed = gen_range(0.0, 1000.0);
        v.push((nom.to_string(), app));
    };
    let spot = vec3(0.6, -0.22, 0.77);

    // Géantes du système solaire
    push("Jupiter", gazeuse(vec3(0.9, 0.66, 0.4), vec3(0.74, 0.44, 0.26), vec3(0.99, 0.95, 0.86), 11.0, 1.9, vec3(0.85, 0.7, 0.5) * 0.3).avec_pole(vec3(0.6, 0.64, 0.7)).avec_jet_profil().avec_tache(spot, 0.27, vec3(0.85, 0.34, 0.18)).avec_cyclones_pol().avec_tempetes(0.7));
    push("Saturne", gazeuse(vec3(0.9, 0.78, 0.5), vec3(0.7, 0.56, 0.34), vec3(0.97, 0.91, 0.68), 13.0, 0.9, vec3(0.88, 0.8, 0.55) * 0.3).avec_jet_profil().avec_hexagone().avec_aurore(0.5, vec3(0.5, 0.7, 1.0)).avec_brume(0.22, vec3(0.95, 0.89, 0.68)).avec_pole(vec3(0.66, 0.63, 0.55)).avec_anneau_saturne(vec3(0.86, 0.79, 0.62)));
    push("Uranus", gazeuse(vec3(0.6, 0.82, 0.82), vec3(0.45, 0.7, 0.72), vec3(0.72, 0.9, 0.9), 6.0, 0.6, vec3(0.6, 0.85, 0.88) * 0.3).avec_axe(vec3(0.25, 0.2, 0.95)).avec_brume(0.45, vec3(0.62, 0.84, 0.86)).avec_pole(vec3(0.62, 0.84, 0.86)));
    push("Neptune", gazeuse(vec3(0.45, 0.56, 0.86), vec3(0.08, 0.16, 0.44), vec3(0.66, 0.8, 0.99), 9.0, 1.5, vec3(0.3, 0.45, 0.9) * 0.3).avec_tache_sombre(spot, 0.2, vec3(0.05, 0.07, 0.18)).avec_tempetes(0.9).avec_pole(vec3(0.3, 0.46, 0.74)));

    // Classification de Sudarsky (par température)
    push("Classe I (ammoniac)", gazeuse(vec3(0.86, 0.8, 0.66), vec3(0.62, 0.56, 0.44), vec3(0.95, 0.92, 0.82), 12.0, 1.4, vec3(0.85, 0.8, 0.6) * 0.3).avec_pole(vec3(0.62, 0.6, 0.55)));
    push("Classe II (eau, albedo haut)", gazeuse(vec3(0.85, 0.88, 0.93), vec3(0.68, 0.74, 0.82), vec3(0.97, 0.98, 1.0), 9.0, 1.0, vec3(0.8, 0.85, 0.95) * 0.35).avec_brume(0.25, vec3(0.9, 0.92, 0.97)).avec_pole(vec3(0.82, 0.86, 0.92)));
    push("Classe III (sans nuage, azur)", gazeuse(vec3(0.16, 0.32, 0.62), vec3(0.1, 0.2, 0.45), vec3(0.28, 0.46, 0.8), 4.0, 0.4, vec3(0.25, 0.45, 0.85) * 0.4).avec_pole(vec3(0.2, 0.36, 0.62)));
    push("Classe IV (alcalins, sombre)", gazeuse(vec3(0.26, 0.13, 0.11), vec3(0.12, 0.06, 0.06), vec3(0.42, 0.22, 0.16), 12.0, 1.8, vec3(0.3, 0.12, 0.08) * 0.25).avec_thermique(0.45, vec3(0.5, 0.08, 0.02)).avec_pole(vec3(0.18, 0.1, 0.09)));
    push("Classe V (silicates, chaud)", gazeuse(vec3(0.55, 0.28, 0.16), vec3(0.3, 0.14, 0.1), vec3(0.85, 0.55, 0.3), 14.0, 2.2, vec3(0.9, 0.5, 0.25) * 0.4).avec_thermique(0.7, vec3(0.9, 0.32, 0.06)).avec_pole(vec3(0.36, 0.18, 0.12)));

    // Variantes
    push("Jupiter chaud", gazeuse(vec3(0.6, 0.3, 0.2), vec3(0.32, 0.14, 0.12), vec3(0.85, 0.45, 0.25), 13.0, 2.4, vec3(0.9, 0.45, 0.2) * 0.4).avec_jet_profil().avec_tache(spot, 0.2, vec3(0.9, 0.35, 0.15)).avec_thermique(0.5, vec3(0.78, 0.22, 0.05)).avec_tempetes(0.6).avec_pole(vec3(0.4, 0.2, 0.16)));
    push("Geante de methane", gazeuse(vec3(0.2, 0.55, 0.45), vec3(0.1, 0.35, 0.3), vec3(0.4, 0.78, 0.62), 8.0, 1.4, vec3(0.3, 0.7, 0.55) * 0.3).avec_pole(vec3(0.34, 0.5, 0.44)));
    push("Geante de soufre", gazeuse(vec3(0.8, 0.7, 0.2), vec3(0.55, 0.45, 0.12), vec3(0.95, 0.88, 0.4), 12.0, 1.8, vec3(0.85, 0.75, 0.25) * 0.3).avec_pole(vec3(0.55, 0.5, 0.34)));
    push("Naine brune", gazeuse(vec3(0.4, 0.15, 0.1), vec3(0.2, 0.08, 0.06), vec3(0.6, 0.25, 0.15), 16.0, 2.6, vec3(0.5, 0.15, 0.08) * 0.35).avec_thermique(0.85, vec3(0.6, 0.12, 0.03)).avec_tempetes(0.6).avec_pole(vec3(0.26, 0.1, 0.07)));
    push("Sub-Neptune", gazeuse(vec3(0.4, 0.5, 0.6), vec3(0.3, 0.4, 0.5), vec3(0.55, 0.65, 0.75), 7.0, 1.0, vec3(0.5, 0.6, 0.7) * 0.3).avec_brume(0.7, vec3(0.62, 0.68, 0.78)).avec_axe(vec3(0.2, 0.85, 0.3)).avec_pole(vec3(0.5, 0.56, 0.64)));

    // Nouveaux types
    push("Geante d'helium", gazeuse(vec3(0.86, 0.85, 0.82), vec3(0.7, 0.69, 0.66), vec3(0.97, 0.97, 0.95), 8.0, 0.7, vec3(0.85, 0.85, 0.82) * 0.3).avec_brume(0.35, vec3(0.93, 0.93, 0.9)).avec_pole(vec3(0.78, 0.78, 0.76)));
    push("Naine brune L (poussiereuse)", gazeuse(vec3(0.55, 0.22, 0.12), vec3(0.32, 0.12, 0.07), vec3(0.72, 0.34, 0.18), 17.0, 2.6, vec3(0.6, 0.2, 0.08) * 0.35).avec_thermique(0.8, vec3(0.7, 0.18, 0.04)).avec_tempetes(0.7).avec_pole(vec3(0.32, 0.14, 0.08)));
    push("Naine brune T (methane)", gazeuse(vec3(0.35, 0.2, 0.32), vec3(0.18, 0.1, 0.2), vec3(0.5, 0.3, 0.48), 15.0, 2.3, vec3(0.4, 0.2, 0.4) * 0.3).avec_thermique(0.45, vec3(0.55, 0.12, 0.25)).avec_tempetes(0.5).avec_pole(vec3(0.22, 0.13, 0.24)));
    push("Naine brune Y (froide)", gazeuse(vec3(0.16, 0.12, 0.18), vec3(0.07, 0.05, 0.1), vec3(0.26, 0.18, 0.3), 12.0, 1.8, vec3(0.18, 0.12, 0.22) * 0.25).avec_thermique(0.2, vec3(0.4, 0.1, 0.18)).avec_pole(vec3(0.12, 0.09, 0.15)));
    push("Neptune chaud", gazeuse(vec3(0.35, 0.46, 0.6), vec3(0.22, 0.32, 0.45), vec3(0.55, 0.66, 0.78), 8.0, 1.3, vec3(0.4, 0.55, 0.72) * 0.3).avec_brume(0.4, vec3(0.55, 0.66, 0.76)).avec_tempetes(0.4).avec_pole(vec3(0.34, 0.45, 0.58)));
    push("Geante de carbone", gazeuse(vec3(0.18, 0.17, 0.16), vec3(0.08, 0.08, 0.08), vec3(0.3, 0.28, 0.25), 10.0, 1.6, vec3(0.12, 0.1, 0.1) * 0.2).avec_pole(vec3(0.14, 0.13, 0.12)));
    push("Proto-geante chaude", gazeuse(vec3(0.7, 0.32, 0.16), vec3(0.45, 0.16, 0.08), vec3(0.95, 0.55, 0.25), 13.0, 2.8, vec3(1.0, 0.5, 0.2) * 0.45).avec_thermique(0.95, vec3(1.0, 0.4, 0.08)).avec_tempetes(0.8).avec_pole(vec3(0.5, 0.22, 0.12)));
    push("Geante rayee extreme", gazeuse(vec3(0.92, 0.6, 0.3), vec3(0.3, 0.14, 0.1), vec3(1.0, 0.95, 0.82), 20.0, 1.4, vec3(0.8, 0.6, 0.4) * 0.3).avec_jet_profil().avec_tempetes(0.5).avec_pole(vec3(0.5, 0.45, 0.4)));

    // Géante emblématique
    push("Polyphemus (Avatar)", gazeuse(vec3(0.32, 0.6, 0.58), vec3(0.12, 0.34, 0.42), vec3(0.6, 0.84, 0.78), 12.0, 1.7, vec3(0.4, 0.7, 0.68) * 0.3).avec_jet_profil().avec_tache(spot, 0.24, vec3(0.88, 0.34, 0.16)).avec_cyclones_pol().avec_tempetes(0.6).avec_pole(vec3(0.4, 0.56, 0.56)).avec_anneau_saturne(vec3(0.66, 0.78, 0.74)));

    // Anneaux : exemples de styles variés
    push("Geante annelee massive", gazeuse(vec3(0.86, 0.76, 0.54), vec3(0.62, 0.5, 0.34), vec3(0.95, 0.9, 0.72), 12.0, 1.0, vec3(0.85, 0.78, 0.55) * 0.3).avec_jet_profil().avec_brume(0.2, vec3(0.94, 0.88, 0.68)).avec_pole(vec3(0.64, 0.62, 0.55)).avec_anneau_saturne(vec3(0.92, 0.86, 0.68)));
    push("Anneau monobande (type Uranus)", gazeuse(vec3(0.5, 0.78, 0.78), vec3(0.34, 0.6, 0.62), vec3(0.66, 0.88, 0.88), 6.0, 0.6, vec3(0.55, 0.82, 0.85) * 0.3).avec_brume(0.3, vec3(0.6, 0.82, 0.84)).avec_pole(vec3(0.55, 0.78, 0.8)).avec_anneau_uranus(vec3(0.55, 0.8, 0.97)));
    push("Anneau ceinture d'asteroides", gazeuse(vec3(0.55, 0.5, 0.42), vec3(0.36, 0.32, 0.26), vec3(0.72, 0.66, 0.56), 10.0, 1.4, vec3(0.6, 0.55, 0.45) * 0.3).avec_pole(vec3(0.42, 0.4, 0.36)).avec_anneau_ceinture(vec3(0.78, 0.74, 0.66)));
    push("Anneaux en arcs (type Neptune)", gazeuse(vec3(0.45, 0.56, 0.86), vec3(0.08, 0.16, 0.44), vec3(0.66, 0.8, 0.99), 9.0, 1.5, vec3(0.3, 0.45, 0.9) * 0.3).avec_tempetes(0.8).avec_pole(vec3(0.3, 0.46, 0.74)).avec_anneau_neptune(vec3(0.6, 0.66, 0.85)));
    push("Anneau de debris recent", gazeuse(vec3(0.62, 0.4, 0.3), vec3(0.38, 0.22, 0.16), vec3(0.82, 0.6, 0.45), 11.0, 1.8, vec3(0.7, 0.45, 0.3) * 0.3).avec_thermique(0.4, vec3(0.7, 0.25, 0.08)).avec_tempetes(0.6).avec_pole(vec3(0.4, 0.26, 0.2)).avec_anneau_debris(vec3(0.85, 0.7, 0.55)));

    v
}

/// Rayon + apparence d'une planète aléatoire (vue « objet » isolée). Couvre les
/// trois types ; la plage de température inclut les mondes de lave.
pub fn planete_aleatoire() -> (f32, Apparence) {
    let t: f32 = gen_range(0.0, 1.0);
    let (rayon, _masse, app) = if t < 0.45 {
        apparence_tellurique(gen_range(120.0, 620.0))
    } else if t < 0.8 {
        apparence_gazeuse()
    } else {
        apparence_glacee()
    };
    (rayon, app)
}
