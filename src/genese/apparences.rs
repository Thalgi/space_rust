use super::taille::ClasseTaille;
use crate::planete::{Apparence, TypePlanete};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::f32::consts::TAU;

/// Tellurique dont l'aspect dépend de sa température d'équilibre `t` (Kelvin).
pub fn apparence_tellurique(t: f32) -> (f32, f32, Apparence) {
    let h: f32 = gen_range(0.0, 1.0);
    // Valeurs par défaut, surchargées par groupe climatique.
    let mut lave = 0.0;
    let mut eau = 0.0;
    let mut eau_motif = 1.0; // continents
    let mut grad_lat = 0.3;
    let mut calotte = 1.0; // pas de banquise
    let mut veg_couv = 0.0;
    let mut veg_couleur = vec3(0.2, 0.5, 0.2);
    let mut rivieres = 0.0;
    let mut nuages = 0.0;
    let mut nuages_couleur = vec3(1.0, 1.0, 1.0);
    let mut nuages_type = 0.0;
    let mut relief = 0.35;
    let mut dunes = 0.0;
    let mut mesa = 0.0;
    let mut pics = 0.0;
    let mut recifs = 0.0;
    let mut basalt = 0.0;
    let mut voile = 0.0;
    let mut voile_couleur = vec3(0.9, 0.8, 0.5);
    let mut crateres = 0.0;
    let mut cryo = 0.0;
    let mut biolum = 0.0;

    let (couleur, couleur2, couleur3) = if t > 600.0 {
        // Lave : croûte sombre variée (rouge sombre ↔ basalte gris), relief volcanique
        // possible ; les fissures émissives (variées par graine) viennent du shader.
        lave = 1.0;
        grad_lat = 0.0;
        relief = gen_range(0.0, 0.5);
        let base = if gen_range(0.0_f32, 1.0) < 0.5 {
            vec3(0.16, 0.10, 0.08) // basalte rougeâtre
        } else {
            vec3(0.12, 0.12, 0.13) // basalte gris
        };
        let c1 = base * gen_range(0.7_f32, 1.3);
        (c1, c1 * 0.5, Vec3::ZERO)
    } else if t > 450.0 {
        // Étuve / brûlée : ocre-gris, sèche, parfois sous un voile type Vénus.
        grad_lat = 0.15;
        dunes = gen_range(0.2, 0.4);
        if gen_range(0.0_f32, 1.0) < 0.5 {
            voile = gen_range(0.8, 0.97);
            voile_couleur = vec3(0.92, 0.82, 0.5);
        } else {
            crateres = gen_range(0.3, 0.6); // rocheuse sans air -> cratérisée
            dunes = 0.0;
        }
        let c1 = hsv(h, gen_range(0.15_f32, 0.4), gen_range(0.55_f32, 0.82));
        (c1, c1 * gen_range(0.6_f32, 0.78), Vec3::ZERO)
    } else if t > 350.0 {
        // Sec / désert : quelques mers intérieures possibles.
        eau = gen_range(0.0, 0.08);
        eau_motif = 2.0;
        grad_lat = 0.25;
        calotte = 0.95;
        dunes = gen_range(0.4, 0.7);
        if gen_range(0.0_f32, 1.0) < 0.4 {
            mesa = gen_range(0.4, 0.8); // certains déserts sont des badlands/mesas
            dunes *= 0.4;
        }
        let c1 = hsv(h, gen_range(0.3_f32, 0.6), gen_range(0.55_f32, 0.8));
        (c1, c1 * gen_range(0.55_f32, 0.78), hsv(h, 0.3, 0.5))
    } else if t > 255.0 {
        // Humide tempérée : océans + continents + calottes modérées.
        eau = gen_range(0.45, 0.7);
        eau_motif = 1.0;
        grad_lat = 0.4;
        calotte = 0.82;
        veg_couv = gen_range(0.6, 0.9);
        veg_couleur = vec3(gen_range(0.14, 0.28), gen_range(0.5, 0.62), gen_range(0.14, 0.28));
        rivieres = gen_range(0.2, 0.5);
        nuages = gen_range(0.3, 0.6);
        relief = gen_range(0.35, 0.6);
        // Variété de temps : surtout classique, parfois tempête ou cyclone.
        let m: f32 = gen_range(0.0, 1.0);
        if m < 0.15 {
            nuages_type = 2.0; // cyclone
            nuages = gen_range(0.6, 0.85);
        } else if m < 0.35 {
            nuages_type = 1.0; // tempête
            nuages_couleur = vec3(0.55, 0.57, 0.62);
            nuages = gen_range(0.6, 0.85);
        }
        if gen_range(0.0_f32, 1.0) < 0.35 {
            recifs = gen_range(0.4, 0.7); // certains mondes humides ont des récifs
        }
        if gen_range(0.0_f32, 1.0) < 0.2 {
            biolum = gen_range(0.4, 0.7); // mondes bioluminescents
        }
        let terre = hsv(h, gen_range(0.4_f32, 0.7), gen_range(0.45_f32, 0.7));
        let roche = terre * gen_range(0.5_f32, 0.75);
        let ocean = hsv(gen_range(0.55_f32, 0.66), gen_range(0.5_f32, 0.85), gen_range(0.45_f32, 0.72));
        (terre, roche, ocean)
    } else if t > 180.0 {
        // Froide / toundra : roche + lichen, large banquise.
        eau = gen_range(0.0, 0.3);
        grad_lat = 0.6;
        calotte = 0.6;
        veg_couv = gen_range(0.1, 0.3);
        veg_couleur = vec3(0.32, 0.42, 0.28);
        nuages = gen_range(0.15, 0.4);
        relief = gen_range(0.5, 0.75);
        if gen_range(0.0_f32, 1.0) < 0.3 {
            basalt = gen_range(0.4, 0.7); // mondes de toundra basaltique
        }
        let c1 = hsv(h, gen_range(0.2_f32, 0.45), gen_range(0.4_f32, 0.6));
        (c1, c1 * gen_range(0.55_f32, 0.75), hsv(0.6, 0.2, 0.8))
    } else {
        // Gelée / boule de neige : banquise quasi globale, bleuté pâle.
        grad_lat = 0.7;
        calotte = 0.15;
        pics = gen_range(0.3, 0.7);
        if gen_range(0.0_f32, 1.0) < 0.3 {
            cryo = gen_range(0.4, 0.7); // mondes cryovolcaniques
        }
        let c1 = hsv(h, gen_range(0.05_f32, 0.2), gen_range(0.8_f32, 1.0));
        (c1, c1 * gen_range(0.85_f32, 0.95), hsv(0.6, 0.15, 0.9))
    };

    let atmo = if eau > 0.4 {
        vec3(0.35, 0.55, 1.0) * 0.9
    } else if t > 255.0 {
        couleur * 0.15
    } else {
        vec3(0.6, 0.8, 1.0) * 0.2
    };
    let mut app = Apparence::new(TypePlanete::Tellurique, couleur, couleur2, couleur3, eau).avec_atmo(atmo);
    app.lave = lave;
    app.eau_motif = eau_motif;
    app.grad_lat = grad_lat;
    app.calotte = calotte;
    app.veg_couleur = veg_couleur;
    app.veg_couv = veg_couv;
    app.rivieres = rivieres;
    app.nuages = nuages;
    app.nuages_couleur = nuages_couleur;
    app.nuages_type = nuages_type;
    if nuages_type == 2.0 {
        app.cyclones_nb = gen_range(0.3, 0.8); // quantité de vortex variable
    }
    app.relief = relief;
    app.dunes = dunes;
    app.mesa = mesa;
    app.pics = pics;
    app.recifs = recifs;
    app.basalt = basalt;
    app.voile = voile;
    app.voile_couleur = voile_couleur;
    app.crateres = crateres;
    app.cryo = cryo;
    app.biolum = biolum;
    app.seed = gen_range(0.0, 1000.0);
    // Taille : majorité de standards, avec des naines (rocheuses sèches) et quelques
    // super-Terres pour la variété. La masse reste dans sa plage historique (physique
    // inchangée) : seul le rayon visuel dépend désormais de la classe.
    let classe = {
        let r: f32 = gen_range(0.0, 1.0);
        if r < 0.22 {
            ClasseTaille::Naine
        } else if r < 0.85 {
            ClasseTaille::Tellurique
        } else {
            ClasseTaille::SuperTerre
        }
    };
    let rayon = classe.rayon_aleatoire();
    app.taille = rayon;
    (rayon, gen_range(1.0, 3.0), app)
}

/// Construit une apparence tellurique à partir de paramètres explicites (presets
/// nommés de la galerie / futurs presets de génération). `motif`/`grad`/`calotte`
/// sont les axes du groupe climatique. La lave se règle ensuite via `app.lave`.
#[allow(clippy::too_many_arguments)]
pub fn tellurique(c1: Vec3, c2: Vec3, c3: Vec3, eau: f32, motif: f32, grad: f32, calotte: f32, atmo: Vec3) -> Apparence {
    let mut a = Apparence::new(TypePlanete::Tellurique, c1, c2, c3, eau).avec_atmo(atmo);
    a.eau_motif = motif;
    a.grad_lat = grad;
    a.calotte = calotte;
    a
}

/// Construit un preset de géante gazeuse à partir de paramètres explicites.
/// `c2` = ceintures sombres, `c1` = accent (filaments chauds), `c3` = zones claires.
/// `bandes` = paires de jets par hémisphère (2..9, profil zonal — voir zonal.rs).
pub fn gazeuse(c1: Vec3, c2: Vec3, c3: Vec3, bandes: f32, warp: f32, atmo: Vec3) -> Apparence {
    let mut a = Apparence::new(TypePlanete::Gazeuse, c1, c2, c3, 0.0);
    a.nb_bandes = bandes;
    a.warp_amt = warp;
    a.atmo = atmo;
    a
}

pub fn apparence_gazeuse() -> (f32, f32, Apparence) {
    let h: f32 = gen_range(0.0, 1.0);
    // ARCHÉTYPE STRUCTUREL (phase 7) : la variété vient d'abord de la
    // structure (profil zonal, vortex, voile), pas seulement de la couleur.
    let tirage: f32 = gen_range(0.0, 1.0);
    let hot = tirage < 0.30; //                       chaude : contraste + thermique
    let icy = !hot && tirage < 0.58; //               glace : voilée, tons moyens
    let lisse = !hot && !icy && tirage < 0.70; //     « classe III » : quasi sans nuages

    // Palette HSV : accent COMPLÉMENTAIRE (h+0.5) pour casser la monochromie.
    // Géantes de glace : c3 reste un TON MOYEN SATURÉ (leçon Neptune — la
    // palette éclaircit déjà ; un c3 pâle donne une « blanche inversée »).
    let zone = if icy {
        hsv(h, gen_range(0.45_f32, 0.75), gen_range(0.55_f32, 0.8))
    } else {
        hsv(h, gen_range(0.12_f32, 0.35), gen_range(0.85_f32, 1.0))
    };
    let belt = hsv(h + gen_range(-0.05_f32, 0.05), gen_range(0.45_f32, 0.82), gen_range(0.32_f32, 0.58));
    let accent = hsv(h + 0.5, gen_range(0.3_f32, 0.6), gen_range(0.55_f32, 0.85));

    // Structure par archétype : (paires de jets, force, flou, warp).
    let (bandes, jets, flou, warp) = if lisse {
        (gen_range(2.0, 3.5), gen_range(0.15, 0.35), gen_range(0.4, 0.7), gen_range(0.3, 0.8))
    } else if icy {
        (gen_range(3.0, 5.0), gen_range(0.3, 0.6), gen_range(0.35, 0.65), gen_range(0.8, 1.8))
    } else if hot {
        (gen_range(4.0, 8.0), gen_range(0.8, 1.2), gen_range(0.05, 0.15), gen_range(1.4, 2.8))
    } else {
        (gen_range(4.0, 8.0), gen_range(0.5, 1.0), gen_range(0.05, 0.25), gen_range(1.0, 2.6))
    };
    let mut app = gazeuse(accent, belt, zone, bandes, warp, zone * 0.32);
    app.seed = gen_range(0.0, 1000.0);
    app.jets_force = jets;
    app.zonal_asym = gen_range(0.15, 0.55);
    app.zonal_flou = flou;
    // Pôle : ceinture désaturée vers un gris-bleu feutré (brume polaire).
    app = app.avec_pole(belt.lerp(vec3(0.5, 0.52, 0.56), 0.55));

    // Tempêtes (slots de vortex) : aucune sur les lisses.
    if !lisse && gen_range(0.0_f32, 1.0) < 0.6 {
        app.tempetes = gen_range(0.4, 0.9);
    }
    // Variante RARE (§ 6 bis, ~6 % des classiques) : « Grande Tache Blanche »
    // statique — tête convective blanche + activité de vortex maximale.
    let gtb = !hot && !icy && !lisse && gen_range(0.0_f32, 1.0) < 0.06;
    if gtb {
        let phi: f32 = gen_range(0.0, TAU);
        let cy: f32 = gen_range(0.15, 0.35); // hémisphère nord, comme les vraies
        let st = (1.0 - cy * cy).sqrt();
        let dir = vec3(st * phi.cos(), cy, st * phi.sin());
        app = app.avec_tache_blanche(dir, gen_range(0.24_f32, 0.34));
        app.tempetes = 1.0;
        app.jets_force = gen_range(0.9, 1.2);
    } else if !lisse && gen_range(0.0_f32, 1.0) < 0.5 {
        // Tache : rouge (anticyclone façon GRS) ou sombre (façon Neptune).
        let phi: f32 = gen_range(0.0, TAU);
        let cy: f32 = gen_range(-0.3, -0.12); // hémisphère sud, latitude tempérée
        let st = (1.0 - cy * cy).sqrt();
        let dir = vec3(st * phi.cos(), cy, st * phi.sin());
        if icy || gen_range(0.0_f32, 1.0) < 0.4 {
            app = app.avec_tache_sombre(dir, gen_range(0.16_f32, 0.24), belt * 0.4);
        } else {
            let col = hsv(0.03, gen_range(0.7_f32, 0.9), gen_range(0.55_f32, 0.85)); // brique-orange
            app = app.avec_tache(dir, gen_range(0.18_f32, 0.28), col);
        }
    }

    if gen_range(0.0_f32, 1.0) < 0.4 {
        app = app.avec_cyclones_pol();
    }
    if gen_range(0.0_f32, 1.0) < 0.3 {
        app.poly_cotes = gen_range(5.0_f32, 9.0).floor(); // vortex polaire polygonal
    }
    if hot {
        app = app.avec_thermique(gen_range(0.4, 0.9), hsv(0.02, 0.9, gen_range(0.5_f32, 0.7)));
    }
    if icy {
        app = app.avec_brume(gen_range(0.3, 0.6), zone.lerp(vec3(0.7, 0.8, 0.9), 0.4));
    }

    // Inclinaison d'axe.
    let tilt: f32 = gen_range(0.0, 0.7);
    let pax: f32 = gen_range(0.0, TAU);
    let axe = vec3(tilt.sin() * pax.cos(), tilt.cos(), tilt.sin() * pax.sin());
    app = app.avec_axe(axe);

    // Anneaux : style aléatoire (dense / arcs / débris / monobande).
    if gen_range(0.0_f32, 1.0) < 0.35 {
        let col = hsv(h, gen_range(0.08_f32, 0.35), gen_range(0.65_f32, 0.92));
        app = app.avec_anneau(col, axe, gen_range(1.25_f32, 1.6), gen_range(2.0_f32, 2.5));
        let styles = [0.0_f32, 2.0, 3.0, 4.0];
        app.anneau_style = styles[gen_range(0.0_f32, 4.0).floor() as usize % 4];
    }

    // Taille : géante gazeuse par défaut ; les géantes « de glace » (icy) prennent la
    // classe correspondante (plus petites, bleutées), et une minorité tombe en
    // sous-Neptune. Masse inchangée (plage historique).
    let classe = if gen_range(0.0_f32, 1.0) < 0.15 {
        ClasseTaille::SousNeptune
    } else if icy {
        ClasseTaille::GeanteGlace
    } else {
        ClasseTaille::GeanteGaz
    };
    let rayon = classe.rayon_aleatoire();
    app.taille = rayon;
    (rayon, gen_range(8.0, 20.0), app)
}

pub fn apparence_glacee() -> (f32, f32, Apparence) {
    let h: f32 = gen_range(0.0, 1.0);
    let c1 = hsv(h, gen_range(0.05_f32, 0.25), gen_range(0.85_f32, 1.0));
    let c2 = hsv(h, gen_range(0.1_f32, 0.3), gen_range(0.8_f32, 0.95));
    // Mondes glacés : petits corps (parfois naine). Masse inchangée.
    let classe = if gen_range(0.0_f32, 1.0) < 0.3 {
        ClasseTaille::Naine
    } else {
        ClasseTaille::Glacee
    };
    let rayon = classe.rayon_aleatoire();
    (
        rayon,
        gen_range(2.0, 5.0),
        Apparence::new(TypePlanete::Glacee, c1, c2, Vec3::ZERO, 0.0)
            .avec_atmo(vec3(0.7, 0.85, 1.0) * 0.25)
            .avec_taille(rayon),
    )
}

/// HSV -> RGB (h, s, v dans [0,1]).
fn hsv(h: f32, s: f32, v: f32) -> Vec3 {
    let h = (h.fract() + 1.0).fract() * 6.0;
    let i = h.floor() as i32;
    let f = h - h.floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    vec3(r, g, b)
}
