use super::pixel::FiltrePixel;
use crate::camera::Camera;
use crate::fond::Fond;
use crate::vaisseau::eclairage;
use crate::vaisseau::{
    demo_anneaux, demo_antennes, demo_caissons, demo_chantier, demo_charpente, demo_coiffes,
    demo_cargo, demo_habitat_isv, demo_habitats,
    demo_moteur_antimatiere, demo_moteur_antimatiere_principal, demo_panneaux, demo_poutres,
    demo_propulsion, demo_radiateur_mega,
    demo_radiateurs, demo_reservoir, demo_station, demo_treillis, generer, preset_anneau, preset_comsat,
    preset_iss, preset_isv, preset_isv_moteur, preset_mir,
    preset_sonde, preset_tiangong, EtatStation, FamillePropulsion, MaillageStation, Ossature,
    ParamsStation, Style,
};
use macroquad::prelude::*;

/// Catégories de la vue station : chacune ne fait cycler (touche **D**) que ses
/// propres items. Le menu route quatre entrées vers cette même vue.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Categorie {
    /// Catalogue des composants (briques).
    Briques,
    /// Reproductions de vraies petites stations + engins.
    PetitesStations,
    /// Générateur procédural (G / S / 1-4 / O).
    Generateur,
    /// Grandes stations & mégastructures.
    Megastructures,
}

impl Categorie {
    fn nb(self) -> usize {
        match self {
            Categorie::Briques => 21,
            Categorie::PetitesStations => 5,
            Categorie::Generateur => 1,
            Categorie::Megastructures => 3,
        }
    }
    fn nom(self) -> &'static str {
        match self {
            Categorie::Briques => "BRIQUES",
            Categorie::PetitesStations => "PETITES STATIONS",
            Categorie::Generateur => "GENERATEUR",
            Categorie::Megastructures => "MEGASTRUCTURES",
        }
    }
}

/// Vue station : quatre catégories (menu), chacune cyclée par **D**. Le
/// générateur réagit en plus à G (graine), S (style), 1-4 (complexité), O
/// (ossature). P = ports, N = numéros, M = cuit/immédiat, X = pixel.
pub struct VueStation {
    categorie: Categorie,
    etat: EtatStation,
    titre: String,
    idx: usize,
    params: ParamsStation,
    cam: Camera,
    fond: Fond,
    ports: bool,
    numeros: bool,
    pixel: FiltrePixel,
    /// Géométrie cuite de la station courante (refaite à chaque `charger`).
    maillage: Option<MaillageStation>,
    /// `true` = maillage cuit, `false` = rendu immédiat (bascule M).
    cuit: bool,
}

impl VueStation {
    pub fn new(categorie: Categorie) -> Self {
        let params = ParamsStation { graine: 1, complexite: 2, style: Style::Historique, ossature: None };
        let mut cam = Camera::new(12.0);
        cam.yaw = 0.7;
        cam.pitch = 0.3;
        let mut vue = Self {
            categorie,
            etat: EtatStation::Vide,
            titre: String::new(),
            idx: 0,
            params,
            cam,
            fond: Fond::new(400),
            ports: false,
            numeros: false,
            pixel: FiltrePixel::new(),
            maillage: None,
            cuit: true,
        };
        vue.charger();
        vue
    }

    /// (Re)construit l'item courant de la catégorie et son titre.
    fn charger(&mut self) {
        let i = self.idx % self.categorie.nb();
        let (etat, titre) = match self.categorie {
            Categorie::Generateur => (
                generer(&self.params),
                format!(
                    "{} — {} — cplx {} — graine {}",
                    self.params.style.nom(),
                    match self.params.ossature {
                        None => "auto",
                        Some(Ossature::Iss) => "ISS",
                        Some(Ossature::Mir) => "Mir",
                    },
                    self.params.complexite,
                    self.params.graine
                ),
            ),
            Categorie::PetitesStations => match i {
                0 => (preset_iss(), "ISS (CONFIGURATION FINALE)".into()),
                1 => (preset_mir(), "MIR (CONFIGURATION FINALE)".into()),
                2 => (preset_tiangong(), "TIANGONG (CONFIGURATION EN T)".into()),
                3 => (preset_comsat(), "SATELLITE DE COMMUNICATION".into()),
                _ => (preset_sonde(), "SONDE INTERPLANETAIRE".into()),
            },
            Categorie::Megastructures => match i {
                0 => (preset_anneau(), "STATION A ANNEAU (ROUE)".into()),
                1 => (preset_isv(), "ISV — PROPULSION + FRET + HABITAT".into()),
                _ => (preset_isv_moteur(), "ISV — RADIATEUR + BLOC MOTEUR".into()),
            },
            Categorie::Briques => match i {
                0 => (EtatStation::Prete(demo_poutres()), "POUTRES (2 STYLES x 6 GABARITS)".into()),
                1 => (EtatStation::Prete(demo_treillis()), "OSSATURE : POUTRE + APPENDICES".into()),
                2 => (EtatStation::Prete(demo_habitats()), "HABITATS : 10 VARIANTES".into()),
                3 => (EtatStation::Prete(demo_station()), "NOEUDS 4 / 6 / T / TETRA".into()),
                4 => (EtatStation::Prete(demo_panneaux()), "PANNEAUX : 5 VARIANTES".into()),
                5 => (EtatStation::Prete(demo_radiateurs()), "RADIATEURS : 8 VARIANTES".into()),
                6 => (demo_radiateur_mega(), "RADIATEUR MEGA".into()),
                7 => (EtatStation::Prete(demo_antennes()), "ANTENNES : 6 VARIANTES".into()),
                8 => (EtatStation::Prete(demo_caissons()), "CAISSONS + CHARGES UTILES".into()),
                9 => (EtatStation::Prete(demo_propulsion(FamillePropulsion::Chimique)), "PROPULSION CHIMIQUE".into()),
                10 => (EtatStation::Prete(demo_propulsion(FamillePropulsion::Electrique)), "PROPULSION ELECTRIQUE".into()),
                11 => (EtatStation::Prete(demo_propulsion(FamillePropulsion::Nucleaire)), "PROPULSION NUCLEAIRE".into()),
                12 => (demo_anneaux(), "ANNEAUX : 4 STYLES".into()),
                13 => (demo_moteur_antimatiere(), "BLOC MOTEUR : CAISSE COLLECTEUR + MODULE".into()),
                14 => (demo_charpente(), "CHARPENTE ISV (+ HEXAGONE EN BAS)".into()),
                15 => (demo_reservoir(), "RESERVOIR CARBURANT (SPHERE)".into()),
                16 => (demo_moteur_antimatiere_principal(), "MOTEUR ANTIMATIERE : TUYERE + REACTEUR".into()),
                17 => (EtatStation::Prete(demo_coiffes()), "COIFFES DE MODULES (3 FORMES)".into()),
                18 => (demo_cargo(), "FRET ISV : NACELLE + TRIFORCE + COURONNE 6".into()),
                19 => (demo_habitat_isv(), "HABITAT PRINCIPAL ISV : MODULE + GRAPPE DE 3".into()),
                _ => (EtatStation::Prete(demo_chantier()), "CONSTRUCTEUR PAR PORTS LIBRES".into()),
            },
        };
        self.etat = etat;
        self.titre = format!("[{}]  {}", self.categorie.nom(), titre);
        // Cuisson une fois par item chargé (plus de régénération par frame).
        self.maillage = self.etat.doit_dessiner().map(MaillageStation::cuire);
        self.cadrer();
    }

    fn cadrer(&mut self) {
        let demi = self.etat.doit_dessiner().map(|s| s.rayon()).unwrap_or(3.0);
        let demi_fov = 45.0_f32.to_radians() * 0.5;
        self.cam.set_dist((demi + 0.5) / demi_fov.tan() * 1.35);
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::P) {
            self.ports = !self.ports;
        }
        if is_key_pressed(KeyCode::N) {
            self.numeros = !self.numeros;
        }
        if is_key_pressed(KeyCode::M) {
            self.cuit = !self.cuit; // comparer maillage cuit / rendu immédiat
        }
        if is_key_pressed(KeyCode::X) {
            self.pixel.basculer(); // filtre pixel ON/OFF
        }
        if is_key_pressed(KeyCode::D) {
            self.idx += 1;
            self.charger();
        }
        // Réglages du générateur : seulement dans sa catégorie.
        if self.categorie == Categorie::Generateur {
            if is_key_pressed(KeyCode::G) {
                self.params.graine = self.params.graine.wrapping_add(1);
                self.charger();
            }
            if is_key_pressed(KeyCode::S) {
                self.params.style = *Style::TOUS
                    .iter()
                    .cycle()
                    .skip_while(|s| **s != self.params.style)
                    .nth(1)
                    .unwrap();
                self.charger();
            }
            for (touche, c) in [(KeyCode::Key1, 1), (KeyCode::Key2, 2), (KeyCode::Key3, 3), (KeyCode::Key4, 4)] {
                if is_key_pressed(touche) {
                    self.params.complexite = c;
                    self.charger();
                }
            }
            if is_key_pressed(KeyCode::O) {
                self.params.ossature = match self.params.ossature {
                    None => Some(Ossature::Iss),
                    Some(Ossature::Iss) => Some(Ossature::Mir),
                    Some(Ossature::Mir) => None,
                };
                self.charger();
            }
        }

        self.cam.input_orbite(false);

        let aspect = screen_width() / screen_height();
        let (cam_info, mut cam3d) = self.cam.construire(Vec3::ZERO, aspect);

        // Couche nette : fond stellaire plein écran.
        set_camera(&cam3d);
        clear_background(BLACK);
        self.fond.draw(&cam_info);
        set_default_camera();

        // Couche station : éclairée, éventuellement pixelisée par-dessus le fond.
        self.pixel.preparer(&mut cam3d);
        set_camera(&cam3d);
        if let Some(station) = self.etat.doit_dessiner() {
            let (cuit, maillage) = (self.cuit, self.maillage.as_ref());
            eclairage::avec(cam_info.pos, || match (cuit, maillage) {
                (true, Some(m)) => m.dessiner(), // quelques draw calls
                _ => station.dessiner(),         // un draw call par primitive
            });
            if self.ports {
                station.dessiner_ports();
            }
        }
        set_default_camera();
        self.pixel.presenter();

        // Numéros de pièce (index d'assemblage) projetés à l'écran, pour pointer
        // les pièces à corriger. L'index = ordre de construction dans le code.
        if self.numeros {
            if let Some(station) = self.etat.doit_dessiner() {
                // Chemin complet : le trait `Camera` de macroquad porte le même
                // nom que la struct `crate::camera::Camera` déjà importée.
                let vp = macroquad::camera::Camera::matrix(&cam3d);
                let (lw, lh) = (screen_width(), screen_height());
                let jaune = Color::new(1.0, 0.85, 0.2, 1.0);
                for (i, piece) in station.pieces().iter().enumerate() {
                    let clip = vp * piece.centre().extend(1.0);
                    if clip.w <= 0.0 {
                        continue; // pièce derrière la caméra
                    }
                    let ndc = clip.truncate() / clip.w;
                    let sx = (ndc.x * 0.5 + 0.5) * lw;
                    let sy = (1.0 - (ndc.y * 0.5 + 0.5)) * lh;
                    crate::police::texte(&format!("{i}"), sx, sy, 20.0, jaune);
                }
            }
        }

        let h = screen_height();
        crate::police::texte(&self.titre, 20.0, h - 24.0, 24.0, WHITE);
        let mode = if self.cuit { "CUIT" } else { "IMMEDIAT" };
        // Coût réel : en cuit, le nombre de lots EST le nombre de draw calls.
        if let Some(m) = &self.maillage {
            // Le coût total sert d'étalon de complexité : c'est l'unité que
            // consomme le budget du générateur.
            let (pieces, cout) = match self.etat.doit_dessiner() {
                Some(s) => (s.nb_pieces(), s.cout_total()),
                None => (0, 0.0),
            };
            crate::police::texte(
                &format!(
                    "{mode} — {} lot(s), {} sommets, {} triangles   |   {pieces} pieces, cout {cout:.0}",
                    m.nb_lots(),
                    m.nb_sommets(),
                    m.nb_triangles()
                ),
                20.0,
                h - 48.0,
                16.0,
                GRAY,
            );
        }
        let etat_ports = if self.ports { "ON" } else { "OFF" };
        let etat_num = if self.numeros { "ON" } else { "OFF" };
        let etat_pix = if self.pixel.actif { "ON" } else { "OFF" };
        // Les réglages du générateur ne sont rappelés que dans sa catégorie.
        let gen = if self.categorie == Categorie::Generateur {
            "1-4: complexite   O: ossature   G: graine   S: style   "
        } else {
            ""
        };
        crate::police::texte(
            &format!("{gen}D: suivant   P: ports ({etat_ports})   N: numeros ({etat_num})   X: pixel ({etat_pix})   M: rendu ({mode})   Echap: menu"),
            12.0,
            24.0,
            17.0,
            WHITE,
        );
        false
    }
}
