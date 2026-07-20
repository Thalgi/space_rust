mod eruptions;
mod materiau;
mod rendu;

use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase};
use eruptions::{Boucle, Tache};
use macroquad::models::Vertex;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use materiau::{mat_plasma_partage, mat_soleil};
use rendu::{texture_halo, TAILLE_HALO};

const MAX_TACHES: usize = 8; // doit correspondre à spots[8] dans le shader

/// Vide le cache de materials du soleil (hot-reload des shaders).
pub fn vider_cache_materials() {
    materiau::vider_cache();
}

pub struct Soleil {
    pub base: CorpsBase,
    temps: f32,
    taches: Vec<Tache>,
    boucles: Vec<Boucle>,
    prochaine_tache: f32,
    prochaine_eruption: f32,
    // Réglages pilotables (sliders), 0..1
    freq: f32,
    forme: f32,
    puissance: f32,
    alea: f32,
    couleur: Vec3,       // teinte de l'étoile (selon son type spectral)
    luminosite: f32,     // intensité lumineuse relative
    couronne: f32,       // extension de la couronne (× rayon) selon le type
    couronne_irreg: f32, // irrégularité (rayons/spicules) de la couronne
    couronne_type: f32,  // 0 = halo, 1 = jets bipolaires (pulsar), 2 = vent (WR)
    zone_hab: bool,      // false = rémanent (pulsar/magnétar/étoile à neutrons) : pas de zone habitable
    mat: Material,        // shader du corps
    mat_plasma: Material, // billboards en blending additif (plasma lumineux)
    halo: Texture2D,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
}

impl Soleil {
    pub fn new(position: Vec3, rayon: f32, couleur: Vec3, luminosite: f32) -> Self {
        // Materials partagés (clonés) -> un seul pipeline chacun, même en régénérant.
        let mat = mat_soleil();
        let mat_plasma = mat_plasma_partage();

        Self {
            base: CorpsBase::new(position, 1000.0, rayon),
            temps: 0.0,
            taches: Vec::new(),
            boucles: Vec::new(),
            prochaine_tache: gen_range(0.5, 2.0),
            prochaine_eruption: gen_range(2.0, 4.0),
            freq: 0.5,
            forme: 0.5,
            puissance: 0.5,
            alea: 0.5,
            couleur,
            luminosite,
            // Étoile chaude/lumineuse -> couronne plus large et plus irrégulière (vents stellaires).
            couronne: (1.25 + 0.14 * luminosite).min(2.1),
            couronne_irreg: ((luminosite - 0.4) / 3.6).clamp(0.0, 1.0),
            couronne_type: 0.0,
            zone_hab: true,
            mat,
            mat_plasma,
            halo: texture_halo(TAILLE_HALO),
            verts: Vec::new(),
            inds: Vec::new(),
        }
    }

    /// Couronne en jets bipolaires (pulsar, protoétoile) : 2 cônes le long de l'axe + disque.
    pub fn avec_jets(mut self) -> Self {
        self.couronne_type = 1.0;
        self.couronne = 5.0; // place pour de longs jets
        self.couronne_irreg = 0.0;
        self
    }

    /// Pulsar : jets bipolaires qui tournent (effet phare) + flash périodique.
    pub fn avec_pulsar(mut self) -> Self {
        self.couronne_type = 3.0;
        self.couronne = 5.0;
        self.couronne_irreg = 0.0;
        self
    }

    /// Magnétar : arcs de champ magnétique dipolaire brillants autour de l'étoile.
    pub fn avec_magnetar(mut self) -> Self {
        self.couronne_type = 4.0;
        self.couronne = 2.8;
        self
    }

    /// Marque l'étoile comme rémanent compact (pulsar/magnétar/étoile à neutrons) :
    /// aucune zone habitable ne sera tracée (`zone_viable` renvoie `None`).
    pub fn sans_zone_habitable(mut self) -> Self {
        self.zone_hab = false;
        self
    }

    /// Couronne en vent stellaire épais et turbulent (Wolf-Rayet, supergéante bleue).
    pub fn avec_vent(mut self) -> Self {
        self.couronne_type = 2.0;
        self.couronne = 2.7; // enveloppe étendue
        self
    }

    /// Trou noir : horizon des événements noir + anneau de photons + disque d'accrétion
    /// incliné (Doppler + spirale). Rendu **stylisé** (pas de lentille gravitationnelle).
    pub fn avec_trou_noir(mut self) -> Self {
        self.couronne_type = 5.0;
        self.couronne = 3.4; // place pour le disque d'accrétion
        self.couronne_irreg = 0.0;
        self
    }
}

impl Astre for Soleil {
    fn categorie(&self) -> Categorie {
        Categorie::Etoile
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn update(&mut self, dt: f32) {
        self.maj(dt);
    }
    fn draw(&mut self, cam: &CameraInfo) {
        self.dessiner(cam);
    }
    fn set_eruptions(&mut self, freq: f32, forme: f32, puissance: f32, alea: f32) {
        self.freq = freq;
        self.forme = forme;
        self.puissance = puissance;
        self.alea = alea;
    }
    fn lumiere(&self) -> Option<Vec3> {
        Some(self.couleur * self.luminosite)
    }
    fn luminosite(&self) -> Option<f32> {
        Some(self.luminosite)
    }
    fn zone_viable(&self) -> Option<(f32, f32)> {
        if !self.zone_hab {
            return None; // rémanent : pas de zone habitable à tracer
        }
        let (i, o) = crate::etoile::zone_habitable(self.luminosite);
        Some((i * crate::etoile::UA, o * crate::etoile::UA)) // UA -> unités monde
    }
}
