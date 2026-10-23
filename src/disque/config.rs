use crate::systeme::G;
use macroquad::prelude::*;

/// **Finesse des débris d'un anneau planétaire**, en fraction de la largeur de
/// l'anneau (demi-tailles, comme partout dans le module — cf. `set_px_min`).
///
/// Exprimées ainsi, une tellurique et une géante rendent le même grain : c'est
/// la largeur de l'anneau qui donne l'échelle, et le cadrage la suit déjà.
///
/// Les valeurs d'origine, absolues, valaient 0.015 et 0.09 en unités monde ; sur
/// l'anneau de Neptune (largeur 1,80) elles faisaient 0,0083 et 0,0497 de la
/// largeur. **Réduites de 40 %** ensuite, sur demande : le grain restait trop
/// gros même une fois les deux vitrines accordées.
const DEBRIS_MIN: f32 = 0.0050;
const DEBRIS_MAX: f32 = 0.0298;

/// Réglages d'un champ de débris (voir docs/conception/ceintures.md).
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
    /// **Ce disque est un disque d'accrétion**, et voici tout son
    /// vocabulaire. `None` pour tout le reste — anneaux planétaires,
    /// ceintures, disques proto\*.
    ///
    /// ⚠️ Un type à part plutôt qu'une dizaine de champs ici : ce
    /// `DisqueConfig` est partagé avec les anneaux de Saturne, et y verser
    /// du vocabulaire d'accrétion est exactement la dérive que la recherche
    /// lui reprochait (`astres_v2_disque_accretion_2.md` §2).
    pub accretion: Option<super::accretion::Accretion>,
    // — Prototypes du lot 12 (2026-10-22, essais de la galerie des disques) —
    /// **A** : amas de poussière qui gonflent et se dissipent dans le voile,
    /// la recette de l'ébullition des géantes rouges en 2D. 0 = aucun.
    pub nuages: f32,
    /// **B** : couches empilées du voile, qui lui donnent une épaisseur.
    /// 1 = plat (le rendu de toujours).
    pub couches: u32,
    /// **B et C** : demi-épaisseur relative `h/r` — un disque protoplanétaire
    /// s'**évase**, plus épais au large qu'au bord interne.
    pub h_r: f32,
    /// **C** : nuages de poussière en volume, en orbite de Kepler. 0 = aucun.
    pub nuages_3d: u32,
    /// **F** : boules par nuage — à 1, un disque flou lisse ; au-delà, une
    /// grappe de boules qui se chevauchent, au contour bosselé : cotonneux.
    pub grappe: u32,
    /// **G** : les nuages se rassemblent en ce nombre d'**amas**, près du plan,
    /// qui tournent chacun **d'un bloc**. 0 = nuages épars.
    pub nuages_amas: u32,
    /// **I** : les nuages s'alignent sur ce nombre de **bras spiraux**, autour
    /// d'un œil, et le motif tourne d'un bloc. 0 = pas de spirale.
    pub nuages_bras: u32,
    /// Taille des nuages (multiplicateur, 1 = d'origine).
    pub taille_nuages: f32,
    /// **H** : écart de teinte d'un nuage à l'autre, 0 = tous de la teinte du
    /// voile à leur rayon.
    pub teintes_nuages: f32,
    /// **Évasement** des nuages 3D : leur hauteur suit `(r / externe)^e`.
    /// 0 = épaisseur proportionnelle au rayon (le cône) ; au-delà, le disque
    /// est mince près de l'étoile et s'évase au bord, comme un vrai disque
    /// jeune (prototype T Tauri, 2026-10-23).
    pub evasement: f32,
    /// **Bras de matière** (T Tauri, `disque::bras`) : rayon de l'astre qui
    /// avale, 0 = aucun bras.
    pub rayon_astre: f32,
    /// Flots d'accrétion magnétosphérique (entonnoirs), 0 = aucun.
    pub entonnoirs: u32,
    /// Inclinaison du dipôle sur la normale du disque (rad) : 0 = rideaux
    /// tout autour, au-delà = deux flots, un par hémisphère.
    pub inclinaison_dipole: f32,
    /// Bras spiraux d'accrétion dans le plan, 0 = aucun.
    pub spirales_accretion: u32,
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
            accretion: None,
            nuages: 0.0,
            couches: 1,
            h_r: 0.0,
            nuages_3d: 0,
            grappe: 1,
            nuages_amas: 0,
            nuages_bras: 0,
            taille_nuages: 1.0,
            teintes_nuages: 0.0,
            evasement: 0.0,
            rayon_astre: 0.0,
            entonnoirs: 0,
            inclinaison_dipole: 0.0,
            spirales_accretion: 0,
        }
    }

    /// Prototype **A** : voir le champ `nuages`.
    pub fn avec_nuages(mut self, force: f32) -> Self {
        self.nuages = force.clamp(0.0, 1.0);
        self
    }

    /// Prototype **B** : voir les champs `couches` et `h_r`.
    pub fn avec_couches(mut self, n: u32, h_r: f32) -> Self {
        self.couches = n.max(1);
        self.h_r = h_r.max(0.0);
        self
    }

    /// Prototype **C** : voir les champs `nuages_3d` et `h_r`. Borné : le
    /// chapelet tient dans un seul maillage (`u16`).
    pub fn avec_nuages_3d(mut self, n: u32, h_r: f32) -> Self {
        self.nuages_3d = n.min(4000);
        self.h_r = h_r.max(0.0);
        self
    }

    /// Prototype **G** : voir le champ `nuages_amas`.
    pub fn avec_amas(mut self, n: u32) -> Self {
        self.nuages_amas = n;
        self
    }

    /// Prototype **I** : voir le champ `nuages_bras`.
    pub fn avec_bras(mut self, n: u32) -> Self {
        self.nuages_bras = n;
        self
    }

    /// Taille et écart de teinte des nuages (prototypes G, H).
    pub fn avec_aspect_nuages(mut self, taille: f32, teintes: f32) -> Self {
        self.taille_nuages = taille.max(0.1);
        self.teintes_nuages = teintes.clamp(0.0, 1.0);
        self
    }

    /// **Entonnoirs** : `n` flots d'accrétion vers un astre de rayon
    /// `rayon_astre`, dipôle incliné de `inclinaison` (rad).
    pub fn avec_entonnoirs(mut self, n: u32, rayon_astre: f32, inclinaison: f32) -> Self {
        self.entonnoirs = n.min(8);
        self.rayon_astre = rayon_astre.max(0.0);
        self.inclinaison_dipole = inclinaison;
        self
    }

    /// **Bras spiraux d'accrétion** : `n` bras jusqu'à un astre de rayon
    /// `rayon_astre`.
    pub fn avec_spirales_accretion(mut self, n: u32, rayon_astre: f32) -> Self {
        self.spirales_accretion = n.min(8);
        self.rayon_astre = rayon_astre.max(0.0);
        self
    }

    /// Voir le champ `evasement`.
    pub fn avec_evasement(mut self, e: f32) -> Self {
        self.evasement = e.max(0.0);
        self
    }

    /// Prototype **F** : voir le champ `grappe`.
    pub fn avec_grappe(mut self, boules: u32) -> Self {
        self.grappe = boules.clamp(1, 8);
        self
    }

    /// **Disque d'accrétion** d'un trou noir stellaire.
    ///
    /// Du gaz, pas des cailloux : aucune particule, tout est dans le voile.
    /// Le gradient thermique est **radial et inversé** par rapport aux
    /// disques proto\* — blanc-bleu au bord interne, où la matière chauffe à
    /// des millions de degrés, orange sombre au bord externe. Il tourne vite
    /// et il **beame** : c'est ce qui le distingue de tout autre anneau.
    /// **Disque d'accrétion**, dans le régime demandé : endormi, actif ou
    /// vorace. Les trois ne diffèrent que par des **constantes**, jamais par
    /// du code — c'est ce qui permet de les comparer côte à côte.
    pub fn accretion_reglee(
        interne: f32,
        externe: f32,
        masse: f32,
        reglage: super::accretion::Accretion,
    ) -> Self {
        Self {
            voile_alpha: 0.85,
            // Interne = `couleur2`, externe = `couleur` : la convention des
            // autres presets, ici avec le chaud au CENTRE.
            voile_couleur: vec3(1.0, 0.45, 0.12),
            voile_couleur2: vec3(0.95, 0.98, 1.0),
            voile_plateau: 1.0,
            voile_alpha_interne: 1.0,
            voile_bord: 0.14,
            // Une turbulence marquée : le gaz d'accrétion n'est pas lisse.
            // Tout le reste est porté par `Accretion` : les champs du voile
            // ci-dessous (granulation, arcs, lacunes…) ne le concernent pas
            // et son shader ne les lit pas.
            accretion: Some(reglage),
            ..Self::base(0, interne, externe, masse)
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
    ///
    /// **La taille des cailloux suit celle de l'anneau**, elle n'est pas absolue.
    ///
    /// Elle l'était (0.015 à 0.09 en unités monde) alors que TOUT le reste de la
    /// géométrie s'exprime en rayons planétaires — bords de l'anneau, et jusqu'au
    /// recul de la caméra. Résultat, la taille APPARENTE d'un caillou variait
    /// comme l'inverse du rayon de la planète : autour d'une tellurique
    /// (« Carbone », rayon 0,45) les débris sortaient **2,7 fois plus gros**
    /// qu'autour d'une géante (« Neptune », rayon 1,20), pour une config
    /// pourtant identique. Les deux vitrines de la galerie ne se ressemblaient
    /// pas, sans qu'aucun réglage ne dise pourquoi.
    ///
    /// Rapportées à la largeur de l'anneau, les deux rendent pareil.
    pub fn debris_planetaire(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        let largeur = (externe - interne).max(1e-6);
        Self {
            epaisseur: 0.04,
            taille_min: DEBRIS_MIN * largeur,
            taille_max: DEBRIS_MAX * largeur,
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

    /// **Débris tombant vers un trou noir.** Preset à part, et non un
    /// `debris_recents` rapetissé.
    ///
    /// ⚠️ La ceinture du trou noir vorace réutilisait `debris_recents` — une
    /// ceinture autour d'une **planète** — avec un `taille_max` divisé sur
    /// place. Un preset d'un autre domaine corrigé par un multiplicateur
    /// local est exactement la dérive que ce chantier a déjà payée trois
    /// fois : les réglages suivants seraient allés au même endroit, et
    /// personne n'aurait su lesquels appartenaient à quoi.
    ///
    /// Ce qui distingue ces débris-ci :
    ///
    /// - **petits**, et sans gros blocs (`bimodal` à zéro) : à l'échelle
    ///   d'un trou noir, un bloc qui se voit est un astre ;
    /// - **excentriques** (`ecc_max`) : ils **tombent**, ils ne patrouillent
    ///   pas une orbite propre — c'est ce qui les distingue d'un anneau ;
    /// - **en traînées** (`clumping` haut) : la marée les étire en filaments
    ///   plutôt qu'elle ne les répartit ;
    /// - **éclairés par le disque**, donc chauds vers l'intérieur : la
    ///   lumière vient du centre, pas d'une étoile lointaine.
    pub fn debris_trou_noir(nb: usize, interne: f32, externe: f32, masse: f32) -> Self {
        Self {
            epaisseur: 0.07,
            profil_radial: 1.5,
            clumping: 0.85,
            ecc_max: 0.35,
            // ⚠️ Divisées par quatre après coup, `taille_min` avec
            // `taille_max` : ne réduire que le plafond aurait resserré toute
            // la population sur une seule taille, et un champ dont tous les
            // grains sont identiques se lit comme une trame, pas comme des
            // débris. C'est le rapport entre les deux qui fait la variété.
            //
            // À ce calibre les blocs tombent sous le pixel de loin ; c'est
            // le plancher en pixels du champ (`disque::set_px_min`) qui les
            // garde visibles, comme pour les ceintures d'astéroïdes.
            taille_min: 0.003,
            taille_max: 0.0138,
            bimodal: 0.0,
            couleur: vec3(0.50, 0.38, 0.32),
            couleur2: vec3(0.95, 0.68, 0.44),
            ..Self::base(nb, interne, externe, masse)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux vitrines de la galerie, aux rayons réels du catalogue.
    const R_TELLURIQUE: f32 = 0.451; // « Carbone »
    const R_GEANTE: f32 = 1.202; // « Neptune »

    /// L'anneau de débris de la galerie : de 1,9 à 3,4 rayons planétaires.
    fn anneau_de(r: f32) -> DisqueConfig {
        DisqueConfig::debris_planetaire(1000, r * 1.9, r * 3.4, 1.0)
    }

    /// **Un caillou doit avoir le même grain APPARENT autour de n'importe quelle
    /// planète.**
    ///
    /// La caméra recule proportionnellement au rayon (`cam_pos = ext * 2`), donc
    /// la taille à l'écran vaut `taille / rayon`. Avec des tailles absolues, ce
    /// rapport variait comme l'inverse du rayon : les débris d'une tellurique
    /// sortaient 2,7 fois plus gros que ceux d'une géante, **à config
    /// identique**. Rien ne le disait — il fallait comparer les deux vitrines.
    #[test]
    fn le_grain_des_debris_ne_depend_pas_de_la_taille_de_la_planete() {
        let (t, g) = (anneau_de(R_TELLURIQUE), anneau_de(R_GEANTE));
        for (nom, a, b) in [
            ("taille_min", t.taille_min / R_TELLURIQUE, g.taille_min / R_GEANTE),
            ("taille_max", t.taille_max / R_TELLURIQUE, g.taille_max / R_GEANTE),
        ] {
            let ecart = (a - b).abs() / b.max(1e-9);
            assert!(
                ecart < 0.01,
                "{nom} : grain apparent {a:.5} (tellurique) contre {b:.5} (géante), \
                 soit {:.0} % d'écart",
                ecart * 100.0
            );
        }
    }

    /// Et il reste **fin** : un caillou ne doit pas peser une part visible de
    /// l'anneau, sinon on ne lit plus une poussière mais des rochers.
    #[test]
    fn les_debris_restent_fins_devant_l_anneau() {
        let c = anneau_de(R_GEANTE);
        let largeur = c.externe - c.interne;
        assert!(
            c.taille_max < largeur * 0.05,
            "le plus gros caillou fait {:.1} % de la largeur de l'anneau",
            c.taille_max / largeur * 100.0
        );
        assert!(c.taille_min > 0.0, "des cailloux de taille nulle ne se dessinent pas");
        assert!(c.taille_min < c.taille_max, "min et max inversés");
    }
}
