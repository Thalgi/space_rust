use crate::camera::Camera;
use crate::fond::Fond;
use crate::vaisseau::{
    demo_antennes, demo_chantier, demo_deux_modules, demo_habitats, demo_panneaux, demo_poutres,
    demo_radiateurs, demo_station, demo_treillis, generer, preset_iss, preset_mir, EtatStation,
    Ossature, ParamsStation, Style,
};
use macroquad::prelude::*;

const NB_DEMOS: usize = 12;

/// Vue « station » : la démo 0 est le **générateur** procédural (G = nouvelle
/// graine, S = style) ; les autres démos (D) montrent les composants. P bascule
/// les gizmos de ports.
pub struct VueStation {
    etat: EtatStation,
    titre: String,
    idx: usize,
    params: ParamsStation,
    cam: Camera,
    fond: Fond,
    ports: bool,
    numeros: bool,
}

impl VueStation {
    pub fn new() -> Self {
        let params = ParamsStation { graine: 1, complexite: 2, style: Style::Historique, ossature: None };
        let mut cam = Camera::new(12.0);
        cam.yaw = 0.7;
        cam.pitch = 0.3;
        let mut vue = Self {
            etat: EtatStation::Vide,
            titre: String::new(),
            idx: 0,
            params,
            cam,
            fond: Fond::new(400),
            ports: false,
            numeros: false,
        };
        vue.charger();
        vue
    }

    /// (Re)construit la démo courante et son titre.
    fn charger(&mut self) {
        let (etat, titre) = match self.idx % NB_DEMOS {
            0 => (
                generer(&self.params),
                format!(
                    "GENERATEUR — {} — {} — cplx {} — graine {}",
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
            1 => (preset_iss(), "PRESET — STATION TYPE ISS".into()),
            2 => (preset_mir(), "PRESET — STATION TYPE MIR".into()),
            3 => (EtatStation::Prete(demo_chantier()), "CONSTRUCTEUR : OSSATURE PAR PORTS LIBRES".into()),
            4 => (EtatStation::Prete(demo_treillis()), "OSSATURE : POUTRE + APPENDICES".into()),
            5 => (EtatStation::Prete(demo_habitats()), "HABITATS : 6 VARIANTES DE MODULE".into()),
            6 => (EtatStation::Prete(demo_poutres()), "POUTRES SEULES (2 STYLES x 6 GABARITS)".into()),
            7 => (EtatStation::Prete(demo_panneaux()), "PANNEAUX : 5 VARIANTES".into()),
            8 => (EtatStation::Prete(demo_radiateurs()), "RADIATEURS : 7 VARIANTES".into()),
            9 => (EtatStation::Prete(demo_antennes()), "ANTENNES : 6 VARIANTES".into()),
            10 => (EtatStation::Prete(demo_station()), "NOEUDS 4 / 6 / T / TETRA".into()),
            _ => (EtatStation::Prete(demo_deux_modules()), "DEUX MODULES BOUT A BOUT".into()),
        };
        self.etat = etat;
        self.titre = titre;
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
        if is_key_pressed(KeyCode::D) {
            self.idx += 1;
            self.charger();
        }
        if is_key_pressed(KeyCode::G) {
            self.params.graine = self.params.graine.wrapping_add(1);
            self.idx = 0; // le G ramène sur le générateur
            self.charger();
        }
        if is_key_pressed(KeyCode::S) {
            self.params.style = *Style::TOUS
                .iter()
                .cycle()
                .skip_while(|s| **s != self.params.style)
                .nth(1)
                .unwrap();
            self.idx = 0;
            self.charger();
        }
        // Complexité 1..4 (touches numériques).
        for (touche, c) in [(KeyCode::Key1, 1), (KeyCode::Key2, 2), (KeyCode::Key3, 3), (KeyCode::Key4, 4)] {
            if is_key_pressed(touche) {
                self.params.complexite = c;
                self.idx = 0;
                self.charger();
            }
        }
        // Ossature : auto → ISS → Mir → auto.
        if is_key_pressed(KeyCode::O) {
            self.params.ossature = match self.params.ossature {
                None => Some(Ossature::Iss),
                Some(Ossature::Iss) => Some(Ossature::Mir),
                Some(Ossature::Mir) => None,
            };
            self.idx = 0;
            self.charger();
        }

        self.cam.input_orbite(false);

        let aspect = screen_width() / screen_height();
        let (cam_info, cam3d) = self.cam.construire(Vec3::ZERO, aspect);

        set_camera(&cam3d);
        clear_background(BLACK);
        self.fond.draw(&cam_info);
        if let Some(station) = self.etat.doit_dessiner() {
            station.dessiner();
            if self.ports {
                station.dessiner_ports();
            }
        }
        set_default_camera();

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
        let etat_ports = if self.ports { "ON" } else { "OFF" };
        let etat_num = if self.numeros { "ON" } else { "OFF" };
        crate::police::texte(
            &format!("1-4: complexite   O: ossature   G: graine   S: style   D: demo   P: ports ({etat_ports})   N: numeros ({etat_num})   Echap: menu"),
            12.0,
            24.0,
            17.0,
            WHITE,
        );
        false
    }
}
