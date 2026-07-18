use crate::systeme::G;
use macroquad::prelude::*;

/// Réglages d'un champ de débris (voir CONCEPTION_CEINTURES.md).
/// Deux couches superposables :
/// - **particules** (`nb > 0`) : corps discrets, orbites animées par le GPU ;
/// - **voile** (`voile_alpha > 0`) : annulus continu à profil procédural
///   (anneaux denses, gaz/poussière des disques proto*).
#[derive(Clone, Copy)]
pub struct DisqueConfig {
    // — Repère —
    pub normale: Vec3,   // plan du disque (anneaux planétaires inclinés)
    pub interne: f32,    // rayon interne (unités monde)
    pub externe: f32,    // rayon externe (unités monde)
    pub gm: f32,         // G * masse du parent (vitesses képlériennes)
    pub graine: f32,     // semences (arcs, granulation, stries)

    // — Distribution des particules —
    pub nb: usize,       // nombre de particules (0 = couche désactivée)
    pub epaisseur: f32,  // dispersion verticale (inclinaison max, rad)
    pub spherite: f32,   // 0 = disque … 1 = coquille isotrope (Oort)
    pub ecc_max: f32,    // excentricité max des orbites (disque épars)
    pub profil_radial: f32, // exposant du tirage radial (1 = uniforme, >1 = concentré interne)
    pub clumping: f32,   // 0 = uniforme … 1 = tout en amas (débris récents)
    pub taille_min: f32, // taille visuelle mini
    pub taille_max: f32, // taille visuelle maxi (gros rares, biais u⁴)
    pub bimodal: f32,    // proba (0..1) qu'une particule soit un GROS bloc
                         // (0.8..1.8 × taille_max, forme très irrégulière) —
                         // essaim de cailloux + quelques fragments massifs
    pub ringlets: f32,   // fraction (0..1) des particules resserrées en
                         // annelets fins (sous-anneaux, positions par graine)
    pub couleur: Vec3,   // teinte au bord externe
    pub couleur2: Vec3,  // teinte au bord interne (gradient radial)

    // — Bandes/lacunes (partagées par les deux couches) —
    // (t centre 0..1, demi-largeur, profondeur, réservé ondulation phase 3).
    // Profondeur > 0 : lacune (creusée par une lune/proto-planète) ;
    // profondeur < 0 : SURDENSITÉ (bande brillante du voile).
    // Les particules ne réagissent qu'aux lacunes (rejet à la création).
    pub lacunes: [Vec4; 4],

    // — Couche voile (voile_alpha = 0 → désactivée) —
    pub voile_alpha: f32,       // opacité maximale
    pub voile_couleur: Vec3,    // teinte au bord externe
    pub voile_couleur2: Vec3,   // teinte au bord interne
    pub voile_plateau: f32,     // densité de base (1 = annulus plein, 0 = bandes seules)
    pub voile_alpha_interne: f32, // facteur d'alpha au bord interne (anneau C : 0.4)
    pub voile_bord: f32,        // largeur des bords doux (fraction radiale)
    pub granulation: f32,       // bruit de valeur (0 lisse … 1 granuleux épars)
    pub gran_seuil: f32,        // seuil des amas visibles (granulation)
    pub gran_freq: Vec2,        // (fréquence spatiale ~cellules/rayon, poids octave fine 0..1)
    pub arcs: f32,              // 0 = anneau complet … 1 = arcs isolés (Neptune)
    pub emissif: f32,           // lueur chaude au bord interne (disques proto*)
    pub rotation_voile: f32,    // vitesse angulaire visuelle au bord interne (rad/s)
}

impl DisqueConfig {
    fn base(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            normale: Vec3::Y,
            interne,
            externe,
            gm: G * masse,
            graine: 0.0,
            nb,
            epaisseur: 0.05,
            spherite: 0.0,
            ecc_max: 0.0,
            profil_radial: 1.0,
            clumping: 0.0,
            taille_min: 0.04,
            taille_max: 0.20,
            bimodal: 0.0,
            ringlets: 0.0,
            couleur: vec3(0.55, 0.5, 0.45),
            couleur2: vec3(0.55, 0.5, 0.45),
            lacunes: [Vec4::ZERO; 4],
            voile_alpha: 0.0,
            voile_couleur: vec3(0.8, 0.75, 0.65),
            voile_couleur2: vec3(0.8, 0.75, 0.65),
            voile_plateau: 1.0,
            voile_alpha_interne: 1.0,
            voile_bord: 0.05,
            granulation: 0.0,
            gran_seuil: 0.45,
            gran_freq: vec2(12.0, 0.6),
            arcs: 0.0,
            emissif: 0.0,
            rotation_voile: 0.02,
        }
    }

    /// Base « voile seul » (anneaux planétaires) : aucune particule.
    fn voile(interne: f32, externe: f32, couleur: Vec3, graine: f32) -> Self {
        Self {
            graine,
            voile_couleur: couleur,
            voile_couleur2: couleur,
            ..Self::base(0, interne, externe, 1.0)
        }
    }

    // ---- Champs de particules (ceintures & co) ----

    /// Ceinture principale d'astéroïdes : rocheuse, fine, corps gris majoritairement petits.
    pub fn asteroides(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self::base(nb, interne, externe, masse)
    }

    /// Ceinture de Kuiper : glacée, plus épaisse et dispersée, quelques gros corps.
    pub fn kuiper(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            epaisseur: 0.28,
            taille_min: 0.06,
            taille_max: 0.5,
            couleur: vec3(0.6, 0.66, 0.78),
            couleur2: vec3(0.6, 0.66, 0.78),
            ..Self::base(nb, interne, externe, masse)
        }
    }

    /// Disque épars : au-delà de Kuiper, orbites excentriques et très inclinées.
    pub fn epars(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            epaisseur: 0.5,
            ecc_max: 0.55,
            taille_min: 0.05,
            taille_max: 0.4,
            couleur: vec3(0.55, 0.6, 0.72),
            couleur2: vec3(0.62, 0.64, 0.7),
            ..Self::base(nb, interne, externe, masse)
        }
    }

    /// Nuage de Oort : coquille sphérique ténue de petits corps glacés.
    pub fn oort(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            spherite: 1.0,
            ecc_max: 0.2,
            taille_min: 0.012,
            taille_max: 0.05,
            couleur: vec3(0.6, 0.68, 0.8),
            couleur2: vec3(0.6, 0.68, 0.8),
            ..Self::base(nb, interne, externe, masse)
        }
    }

    /// Débris récents (lune brisée, impact) : amas denses encore mal étalés.
    pub fn debris_recents(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            epaisseur: 0.09,
            clumping: 0.75,
            taille_min: 0.02,
            taille_max: 0.16,
            couleur: vec3(0.62, 0.44, 0.32),
            couleur2: vec3(0.78, 0.56, 0.4),
            ..Self::base(nb, interne, externe, masse)
        }
    }

    /// Ceinture de débris autour d'une planète (tellurique ou gazeuse).
    /// Vue de près, donc travaillée : essaim de petits cailloux + quelques
    /// gros fragments (bimodal), particules resserrées en annelets, et voile
    /// de POUSSIÈRE (plateau ténu + 2 bandes fines + granulation).
    pub fn debris_planetaire(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            epaisseur: 0.04,
            taille_min: 0.015,
            taille_max: 0.09,
            bimodal: 0.06,
            ringlets: 0.45,
            couleur: vec3(0.58, 0.54, 0.5),
            couleur2: vec3(0.66, 0.62, 0.56),
            voile_alpha: 0.16,
            voile_couleur: vec3(0.6, 0.56, 0.5),
            voile_couleur2: vec3(0.68, 0.63, 0.55),
            voile_plateau: 0.2,
            voile_bord: 0.07,
            granulation: 0.5,
            gran_seuil: 0.4,
            gran_freq: vec2(16.0, 0.5),
            rotation_voile: 0.04,
            ..Self::base(nb, interne, externe, masse)
        }
        .avec_bande(0.3, 0.02, 0.5)  // bandes de poussière fines
        .avec_bande(0.7, 0.015, 0.4)
    }

    /// Disque protoplanétaire : particules (planétésimaux) + voile de poussière
    /// chaude. Sillons des proto-planètes à poser via `avec_lacune`.
    pub fn protoplanetaire(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            epaisseur: 0.06,
            profil_radial: 1.4,
            taille_min: 0.015,
            taille_max: 0.07,
            couleur: vec3(0.5, 0.32, 0.26),
            couleur2: vec3(1.0, 0.85, 0.58),
            voile_alpha: 0.5,
            voile_couleur: vec3(0.42, 0.26, 0.22),
            voile_couleur2: vec3(0.95, 0.72, 0.45),
            voile_bord: 0.10,
            granulation: 0.5,
            gran_seuil: 0.45,
            gran_freq: vec2(9.0, 0.7),
            emissif: 0.5,
            rotation_voile: 0.05,
            ..Self::base(nb, interne, externe, masse)
        }
    }

    /// Disque protosolaire : matière jusqu'au centre, très chaud à l'intérieur.
    pub fn protosolaire(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            epaisseur: 0.05,
            profil_radial: 1.8,
            taille_min: 0.015,
            taille_max: 0.06,
            couleur: vec3(0.72, 0.42, 0.24),
            couleur2: vec3(1.0, 0.95, 0.78),
            voile_alpha: 0.65,
            voile_couleur: vec3(0.5, 0.28, 0.18),
            voile_couleur2: vec3(1.0, 0.88, 0.6),
            voile_bord: 0.12,
            granulation: 0.45,
            gran_seuil: 0.4,
            gran_freq: vec2(7.0, 0.7),
            emissif: 1.0,
            rotation_voile: 0.07,
            ..Self::base(nb, interne, externe, masse)
        }
    }

    // ---- Anneaux planétaires (voile seul) — parité des 5 styles V1 ----

    /// Saturne : annulus dense, anneau C interne ténu, lacunes Cassini + Encke.
    pub fn anneau_saturne(interne: f32, externe: f32, couleur: Vec3, graine: f32) -> Self {
        Self {
            voile_alpha: 0.78,
            voile_plateau: 1.0,
            voile_alpha_interne: 0.4,
            voile_bord: 0.03,
            rotation_voile: 0.03,
            ..Self::voile(interne, externe, couleur, graine)
        }
        .avec_lacune(0.58, 0.030, 0.93) // Cassini
        .avec_lacune(0.88, 0.012, 0.85) // Encke
    }

    /// Uranus : une seule bande fine et nette.
    pub fn anneau_uranus(interne: f32, externe: f32, couleur: Vec3, graine: f32) -> Self {
        Self {
            voile_alpha: 0.8,
            voile_plateau: 0.0,
            voile_bord: 0.02,
            rotation_voile: 0.04,
            ..Self::voile(interne, externe, couleur, graine)
        }
        .avec_bande(0.60, 0.05, 0.9)
    }

    /// Neptune : arcs partiels sur l'anneau principal + voile ténu.
    pub fn anneau_arcs(interne: f32, externe: f32, couleur: Vec3, graine: f32) -> Self {
        Self {
            voile_alpha: 0.85,
            voile_plateau: 0.07,
            voile_bord: 0.03,
            arcs: 0.9,
            rotation_voile: 0.04,
            ..Self::voile(interne, externe, couleur, graine)
        }
        .avec_bande(0.86, 0.04, 0.85)
        .avec_bande(0.62, 0.03, 0.3)
    }

    /// Petite ceinture granuleuse : particules éparses sur une large couronne.
    pub fn anneau_granuleux(interne: f32, externe: f32, couleur: Vec3, graine: f32) -> Self {
        Self {
            voile_alpha: 0.6,
            voile_plateau: 0.0,
            voile_bord: 0.06,
            granulation: 1.0,
            gran_seuil: 0.55,
            gran_freq: vec2(14.0, 0.5),
            rotation_voile: 0.03,
            ..Self::voile(interne, externe, couleur, graine)
        }
        .avec_bande(0.55, 0.33, 0.65)
    }

    /// Débris récents : amas irréguliers et brillants.
    pub fn anneau_debris(interne: f32, externe: f32, couleur: Vec3, graine: f32) -> Self {
        Self {
            voile_alpha: 0.85,
            voile_plateau: 0.0,
            voile_bord: 0.05,
            granulation: 1.0,
            gran_seuil: 0.58,
            gran_freq: vec2(16.0, 0.6),
            rotation_voile: 0.05,
            ..Self::voile(interne, externe, couleur, graine)
        }
        .avec_bande(0.55, 0.30, 0.85)
    }

    // ---- Ajustements (style builder, pour composer les presets) ----

    /// Ouvre une lacune : `t` centre (0 interne .. 1 externe), `demi_largeur`,
    /// `profondeur` (1 = vide total). Rejette aussi les particules.
    pub fn avec_lacune(mut self, t: f32, demi_largeur: f32, profondeur: f32) -> Self {
        if let Some(l) = self.lacunes.iter_mut().find(|l| l.y == 0.0) {
            *l = vec4(t, demi_largeur, profondeur, 0.0);
        }
        self
    }

    /// Lacune VIVANTE : comme `avec_lacune`, plus une ondulation des bords
    /// (festons de Daphnis) synchronisée sur la phase orbitale du corps qui
    /// creuse le sillon (voir `Disque::position_lacune`). `ondulation` 0..1.
    pub fn avec_lacune_ondulee(
        mut self,
        t: f32,
        demi_largeur: f32,
        profondeur: f32,
        ondulation: f32,
    ) -> Self {
        if let Some(l) = self.lacunes.iter_mut().find(|l| l.y == 0.0) {
            *l = vec4(t, demi_largeur, profondeur, ondulation);
        }
        self
    }

    /// Ajoute une SURDENSITÉ du voile (bande brillante) : profondeur négative.
    pub fn avec_bande(mut self, t: f32, demi_largeur: f32, force: f32) -> Self {
        if let Some(l) = self.lacunes.iter_mut().find(|l| l.y == 0.0) {
            *l = vec4(t, demi_largeur, -force, 0.0);
        }
        self
    }

    /// Force la vitesse orbitale (gm direct) — utile en galerie où les rayons
    /// sont en unités de cellule, pas à l'échelle du système.
    pub fn avec_gm(mut self, gm: f32) -> Self {
        self.gm = gm;
        self
    }

    /// Oriente le plan du disque (anneaux planétaires inclinés).
    pub fn avec_normale(mut self, n: Vec3) -> Self {
        self.normale = n;
        self
    }
}
