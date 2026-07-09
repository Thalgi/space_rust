use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum TypePlanete {
    Tellurique,
    Gazeuse,
    Glacee,
}

impl TypePlanete {
    pub fn code(self) -> f32 {
        match self {
            TypePlanete::Tellurique => 0.0,
            TypePlanete::Gazeuse => 1.0,
            TypePlanete::Glacee => 2.0,
        }
    }
}

/// Apparence d'une planète (construite façon builder).
#[derive(Clone, Copy)]
pub struct Apparence {
    pub type_p: TypePlanete,
    pub couleur: Vec3,
    pub couleur2: Vec3,
    pub couleur3: Vec3, // tellurique: océan ; gazeuse: 3e bande
    pub eau: f32,
    // Grande tache (gazeuses) : direction sur la sphère, taille angulaire, couleur.
    pub tache_dir: Vec3,
    pub tache_taille: f32, // 0 = pas de tache
    pub tache_couleur: Vec3,
    pub tache_type: f32, // 0 = tache rouge (GRS), 1 = tache sombre (Neptune)
    // Anneau.
    pub anneau: bool,
    pub anneau_couleur: Vec3,
    pub anneau_normal: Vec3, // axe perpendiculaire au plan de l'anneau
    pub anneau_in: f32,      // rayon interne (× rayon planète)
    pub anneau_out: f32,     // rayon externe (× rayon planète)
    pub anneau_style: f32,   // 0 = dense+lacunes (Saturne), 1 = fins (Uranus), 2 = arcs (Neptune), 3 = débris
    pub axe: Vec3,           // axe de rotation (bandes/pôles s'alignent dessus)
    // Variabilité atmosphérique (gazeuses) — profil zonal précalculé (zonal.rs).
    pub nb_bandes: f32,  // paires de jets par hémisphère (2..9) -> nombre de bandes lisible
    pub jets_force: f32, // amplitude du profil zonal (0 = calme type Uranus, 1 = Jupiter)
    pub zonal_asym: f32, // asymétrie nord/sud du profil (0 = miroir, 1 = très différent)
    pub zonal_flou: f32, // adoucissement des frontières de bandes (classes voilées)
    pub warp_amt: f32,   // force du domain warping
    pub seed: f32,       // décalage de bruit -> chaque géante est unique
    pub poly_cotes: f32, // vortex polaire : 0 = aucun, sinon nb de côtés (6 = hexagone Saturne)
    pub atmo: Vec3,      // couleur*intensité du halo atmosphérique (0 = pas d'atmosphère)
    pub lave: f32,       // monde de lave : fissures incandescentes émissives (0 = aucun)
    // Telluriques — groupe climatique.
    pub eau_motif: f32,  // 0 océan global, 1 continents, 2 mers intérieures, 3 marais
    pub grad_lat: f32,   // contraste de biome équateur->pôle
    pub calotte: f32,    // latitude (0..1) de début de banquise (1 = aucune)
    pub veg_couleur: Vec3, // teinte de la végétation
    pub veg_couv: f32,   // couverture végétale (0 = sol nu)
    pub rivieres: f32,   // densité de rivières (0 = aucune)
    pub nuages: f32,     // densité de la couche nuageuse (0 = ciel clair)
    pub nuages_couleur: Vec3, // teinte des nuages
    pub nuages_type: f32, // 0 classique, 1 tempête, 2 cyclone
    pub cyclones_nb: f32, // proportion (0..1) des emplacements de cyclones actifs (type 2)
    pub relief: f32,     // amplitude des montagnes (0 = plat)
    pub dunes: f32,      // ondulations de dunes (0 = aucune)
    pub mesa: f32,       // plateaux étagés / canyons / strates (0 = aucun)
    pub pics: f32,       // pics/aiguilles de glace (0 = aucun)
    pub recifs: f32,     // récifs/atolls sur les hauts-fonds (0 = aucun)
    pub basalt: f32,     // orgues basaltiques (0 = aucun)
    pub voile: f32,      // voile atmosphérique opaque (0 = aucun)
    pub voile_couleur: Vec3, // teinte du voile
    pub crateres: f32,   // cratères d'impact (0 = aucun)
    pub eyeball: f32,    // verrouillage de marée (0 = aucun)
    pub eye_glace: f32,  // angle solaire de la limite de calotte
    pub eye_lave: f32,   // 1 = zone subsolaire lave/obsidienne
    pub eye_ring: f32,   // 1 = anneau de forêt au terminateur
    pub cryo: f32,       // cryovolcanisme émissif (0 = aucun)
    pub biolum: f32,     // bioluminescence nocturne (0 = aucun)
    pub riv_lave: f32,   // rivières de lave au lieu d'eau (0 = eau, 1 = lave)
    // Géantes gazeuses (features dédiées).
    pub cyclones_pol: f32, // amas de cyclones aux pôles (0 = aucun)
    pub thermique: f32,    // émission thermique nocturne (géantes chaudes)
    pub thermique_couleur: Vec3,
    pub tempetes: f32,     // densité de petites tempêtes (0 = aucune)
    pub aurore: f32,       // aurores polaires émissives (0 = aucune)
    pub aurore_couleur: Vec3,
    pub brume: f32,        // voile de brume qui adoucit les bandes (0 = aucun)
    pub brume_couleur: Vec3,
    pub g_pole: Vec3,      // teinte des régions polaires (dégradé latitudinal gazeuses)
    // Mode d'affichage (global, piloté par l'UI ; pas par la génération).
    pub villes: f32,     // 1 = lumières de villes côté nuit, 0 = monde non colonisé
    // Taille physique (rayon visuel en unités de jeu). Source unique de vérité,
    // consommée par toutes les vues (galerie, objet, systèmes). N'est PAS un
    // uniform shader : le rayon transite séparément via CorpsBase. Voir
    // `genese::taille`. 0.6 ≈ une Terre standard.
    pub taille: f32,
}

impl Apparence {
    pub fn new(type_p: TypePlanete, couleur: Vec3, couleur2: Vec3, couleur3: Vec3, eau: f32) -> Self {
        Self {
            type_p,
            couleur,
            couleur2,
            couleur3,
            eau,
            tache_dir: Vec3::Y,
            tache_taille: 0.0,
            tache_couleur: Vec3::ZERO,
            tache_type: 0.0,
            anneau: false,
            anneau_couleur: Vec3::ZERO,
            anneau_normal: Vec3::Y,
            anneau_in: 1.4,
            anneau_out: 2.2,
            anneau_style: 0.0,
            axe: Vec3::Y,
            nb_bandes: 5.0,
            jets_force: 0.6,
            zonal_asym: 0.35,
            zonal_flou: 0.15,
            warp_amt: 1.6,
            seed: 0.0,
            poly_cotes: 0.0,
            atmo: Vec3::ZERO,
            lave: 0.0,
            eau_motif: 1.0,
            grad_lat: 0.3,
            calotte: 1.0,
            veg_couleur: vec3(0.2, 0.5, 0.2),
            veg_couv: 0.0,
            rivieres: 0.0,
            nuages: 0.0,
            nuages_couleur: vec3(1.0, 1.0, 1.0),
            nuages_type: 0.0,
            cyclones_nb: 0.5,
            relief: 0.35,
            dunes: 0.0,
            mesa: 0.0,
            pics: 0.0,
            recifs: 0.0,
            basalt: 0.0,
            voile: 0.0,
            voile_couleur: vec3(0.9, 0.8, 0.5),
            crateres: 0.0,
            eyeball: 0.0,
            eye_glace: 0.0,
            eye_lave: 0.0,
            eye_ring: 0.0,
            cryo: 0.0,
            biolum: 0.0,
            riv_lave: 0.0,
            cyclones_pol: 0.0,
            thermique: 0.0,
            thermique_couleur: vec3(0.5, 0.1, 0.03),
            tempetes: 0.0,
            aurore: 0.0,
            aurore_couleur: vec3(0.3, 0.9, 0.6),
            brume: 0.0,
            brume_couleur: vec3(0.6, 0.66, 0.74),
            g_pole: vec3(0.6, 0.6, 0.63),
            villes: 1.0,
            taille: 0.6, // tellurique standard par défaut (≈ 1 R⊕)
        }
    }

    /// Fixe la taille physique (rayon visuel, unités de jeu). Voir `genese::taille`.
    pub fn avec_taille(mut self, taille: f32) -> Self {
        self.taille = taille;
        self
    }

    pub fn avec_vegetation(mut self, couleur: Vec3, couverture: f32) -> Self {
        self.veg_couleur = couleur;
        self.veg_couv = couverture;
        self
    }
    pub fn avec_rivieres(mut self, densite: f32) -> Self {
        self.rivieres = densite;
        self
    }
    pub fn avec_nuages(mut self, densite: f32, couleur: Vec3) -> Self {
        self.nuages = densite;
        self.nuages_couleur = couleur;
        self
    }
    /// Nuages avec type de temps : 0 classique, 1 tempête sombre, 2 cyclone.
    pub fn avec_meteo(mut self, densite: f32, couleur: Vec3, type_t: f32) -> Self {
        self.nuages = densite;
        self.nuages_couleur = couleur;
        self.nuages_type = type_t;
        self
    }
    /// Météo cyclonique : `nb` = proportion (0..1) des emplacements de vortex actifs.
    pub fn avec_cyclones(mut self, nb: f32) -> Self {
        self.nuages_type = 2.0;
        self.cyclones_nb = nb;
        self
    }
    pub fn avec_relief(mut self, amplitude: f32) -> Self {
        self.relief = amplitude;
        self
    }
    pub fn avec_dunes(mut self, intensite: f32) -> Self {
        self.dunes = intensite;
        self
    }
    pub fn avec_mesa(mut self, intensite: f32) -> Self {
        self.mesa = intensite;
        self
    }
    pub fn avec_pics(mut self, intensite: f32) -> Self {
        self.pics = intensite;
        self
    }
    pub fn avec_recifs(mut self, intensite: f32) -> Self {
        self.recifs = intensite;
        self
    }
    pub fn avec_basalt(mut self, intensite: f32) -> Self {
        self.basalt = intensite;
        self
    }
    pub fn avec_voile(mut self, densite: f32, couleur: Vec3) -> Self {
        self.voile = densite;
        self.voile_couleur = couleur;
        self
    }
    pub fn avec_crateres(mut self, densite: f32) -> Self {
        self.crateres = densite;
        self
    }
    pub fn avec_eyeball(mut self, intensite: f32) -> Self {
        self.eyeball = intensite;
        self
    }
    /// Eyeball détaillé : `glace` = angle solaire de la limite de banquise,
    /// `lave` (0/1) = zone subsolaire lave/obsidienne, `ring` (0/1) = anneau de forêt.
    pub fn avec_eyeball_zones(mut self, glace: f32, lave: f32, ring: f32) -> Self {
        self.eyeball = 1.0;
        self.eye_glace = glace;
        self.eye_lave = lave;
        self.eye_ring = ring;
        self
    }
    pub fn avec_cryo(mut self, intensite: f32) -> Self {
        self.cryo = intensite;
        self
    }
    pub fn avec_biolum(mut self, intensite: f32) -> Self {
        self.biolum = intensite;
        self
    }
    pub fn avec_riv_lave(mut self) -> Self {
        self.riv_lave = 1.0;
        self
    }
    pub fn avec_atmo(mut self, couleur: Vec3) -> Self {
        self.atmo = couleur;
        self
    }
    pub fn avec_hexagone(mut self) -> Self {
        self.poly_cotes = 6.0;
        self
    }
    pub fn avec_axe(mut self, axe: Vec3) -> Self {
        self.axe = axe.normalize_or_zero();
        self
    }
    pub fn avec_tache(mut self, dir: Vec3, taille: f32, couleur: Vec3) -> Self {
        self.tache_dir = dir.normalize_or_zero();
        self.tache_taille = taille;
        self.tache_couleur = couleur;
        self
    }
    /// Tache sombre (Grande Tache Sombre de Neptune) : ovale sombre, bords fondus, sans collier.
    pub fn avec_tache_sombre(mut self, dir: Vec3, taille: f32, couleur: Vec3) -> Self {
        self.tache_dir = dir.normalize_or_zero();
        self.tache_taille = taille;
        self.tache_couleur = couleur;
        self.tache_type = 1.0;
        self
    }
    /// Tête de « Grande Tache Blanche » (tempête planétaire type Saturne,
    /// CONCEPTION_GAZEUSES_V2 § 6 bis) : gros ovale blanc convectif en slot 0.
    pub fn avec_tache_blanche(mut self, dir: Vec3, taille: f32) -> Self {
        self.tache_dir = dir.normalize_or_zero();
        self.tache_taille = taille;
        self.tache_type = 2.0;
        self
    }
    pub fn avec_cyclones_pol(mut self) -> Self {
        self.cyclones_pol = 1.0;
        self
    }
    pub fn avec_thermique(mut self, intensite: f32, couleur: Vec3) -> Self {
        self.thermique = intensite;
        self.thermique_couleur = couleur;
        self
    }
    pub fn avec_tempetes(mut self, densite: f32) -> Self {
        self.tempetes = densite;
        self
    }
    pub fn avec_aurore(mut self, intensite: f32, couleur: Vec3) -> Self {
        self.aurore = intensite;
        self.aurore_couleur = couleur;
        self
    }
    pub fn avec_brume(mut self, densite: f32, couleur: Vec3) -> Self {
        self.brume = densite;
        self.brume_couleur = couleur;
        self
    }
    pub fn avec_pole(mut self, couleur: Vec3) -> Self {
        self.g_pole = couleur;
        self
    }
    /// Force des jets zonaux (0 = calme type Uranus, ~1 = Jupiter démonstratif).
    pub fn avec_jets(mut self, force: f32) -> Self {
        self.jets_force = force;
        self
    }
    /// Adoucissement des frontières de bandes (classes voilées, sub-Neptunes).
    pub fn avec_zonal_flou(mut self, flou: f32) -> Self {
        self.zonal_flou = flou;
        self
    }
    pub fn avec_anneau(mut self, couleur: Vec3, normal: Vec3, r_in: f32, r_out: f32) -> Self {
        self.anneau = true;
        self.anneau_couleur = couleur;
        self.anneau_normal = normal.normalize_or_zero();
        self.anneau_in = r_in;
        self.anneau_out = r_out;
        self.axe = self.anneau_normal; // l'anneau est dans le plan équatorial -> axe = sa normale
        self
    }
    /// Anneau paramétré avec un style de motif (voir `anneau_style`).
    pub fn avec_anneau_style(
        mut self,
        couleur: Vec3,
        normal: Vec3,
        r_in: f32,
        r_out: f32,
        style: f32,
    ) -> Self {
        self = self.avec_anneau(couleur, normal, r_in, r_out);
        self.anneau_style = style;
        self
    }
    /// Anneau type Saturne : dense, large, avec lacunes (Cassini/Encke), légèrement incliné.
    pub fn avec_anneau_saturne(self, couleur: Vec3) -> Self {
        self.avec_anneau_style(couleur, vec3(0.16, 1.0, 0.34), 1.28, 2.3, 0.0)
    }
    /// Anneau type Uranus : une bande quasi unique, bleu ciel, plan quasi vertical.
    pub fn avec_anneau_uranus(self, couleur: Vec3) -> Self {
        self.avec_anneau_style(couleur, vec3(0.97, 0.12, 0.2), 1.5, 2.1, 4.0)
    }
    /// Anneau « petite ceinture d'astéroïdes » : particules granuleuses éparses sur une large couronne.
    pub fn avec_anneau_ceinture(self, couleur: Vec3) -> Self {
        self.avec_anneau_style(couleur, vec3(0.2, 1.0, 0.32), 1.35, 2.2, 1.0)
    }
    /// Anneau type Neptune : arcs partiels (anneau d'Adams), ténu, incliné.
    pub fn avec_anneau_neptune(self, couleur: Vec3) -> Self {
        self.avec_anneau_style(couleur, vec3(0.18, 1.0, 0.28), 1.5, 2.2, 2.0)
    }
    /// Anneau de débris récent : amas irréguliers et brillants, serrés autour de la planète.
    pub fn avec_anneau_debris(self, couleur: Vec3) -> Self {
        self.avec_anneau_style(couleur, vec3(0.22, 1.0, 0.3), 1.2, 1.95, 3.0)
    }
}
