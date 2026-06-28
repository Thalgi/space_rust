use crate::camera::Camera;
use crate::etoile::ProfilEtoile;
use crate::fond::Fond;
use crate::genese::planete_aleatoire;
use crate::planete::{Apparence, Planete};
use crate::rendu::{Rendu, RenduStandard};
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Le corps affiché, mémorisé pour pouvoir le reconstruire à l'identique
/// (hot-reload des shaders sur ce specimen précis).
enum Specimen {
    Soleil { rayon: f32, couleur: Vec3, lumi: f32, nom: String, couronne: f32 },
    Planete { rayon: f32, app: Apparence },
}

impl Specimen {
    /// Tire un specimen au hasard. `forcer` : Some(true)=soleil, Some(false)=planète.
    fn aleatoire(forcer: Option<bool>) -> Self {
        let soleil = forcer.unwrap_or(gen_range(0.0_f32, 1.0) < 0.5);
        if soleil {
            let p = ProfilEtoile::aleatoire();
            Specimen::Soleil {
                rayon: p.rayon,
                couleur: p.couleur,
                lumi: p.luminosite,
                nom: p.nom().to_string(),
                couronne: p.couronne,
            }
        } else {
            let (rayon, app) = planete_aleatoire();
            Specimen::Planete { rayon, app }
        }
    }

    fn rayon(&self) -> f32 {
        match self {
            Specimen::Soleil { rayon, .. } => *rayon,
            Specimen::Planete { rayon, .. } => *rayon,
        }
    }

    fn label(&self) -> String {
        match self {
            Specimen::Soleil { nom, .. } => format!("ETOILE - {}", nom),
            Specimen::Planete { .. } => "PLANETE".to_string(),
        }
    }
}

/// Construit un système à un seul astre, centré à l'origine.
fn batir(spec: &Specimen) -> Systeme {
    let mut sys = Systeme::new();
    match spec {
        Specimen::Soleil { rayon, couleur, lumi, couronne, .. } => {
            let s = Soleil::new(Vec3::ZERO, *rayon, *couleur, *lumi);
            let s = match *couronne as i32 {
                1 => s.avec_jets(),
                2 => s.avec_vent(),
                3 => s.avec_pulsar(),
                4 => s.avec_magnetar(),
                _ => s,
            };
            sys.ajouter(Box::new(s));
        }
        Specimen::Planete { rayon, app } => {
            sys.ajouter(Box::new(Planete::new(Vec3::ZERO, Vec3::ZERO, *rayon, 1.0, *app, Vec::new())));
            // Pas d'étoile : lumière latérale de secours pour modeler le relief.
            sys.set_lumiere(vec3(6.0, 3.0, 6.0), Vec3::ONE);
        }
    }
    sys
}

/// Distance caméra adaptée à la taille du corps (couronne du soleil incluse).
fn dist_pour(spec: &Specimen) -> f32 {
    (spec.rayon() * 5.0 + 5.0).max(6.0)
}

/// Vue d'un astre isolé : pratique pour travailler le rendu d'un seul corps.
pub struct Objet {
    spec: Specimen,
    sys: Systeme,
    fond: Fond,
    cam: Camera,
    rendu: RenduStandard,
}

impl Objet {
    pub fn new() -> Self {
        let spec = Specimen::aleatoire(None);
        let sys = batir(&spec);
        let cam = Camera::new(dist_pour(&spec));
        Self {
            spec,
            sys,
            fond: Fond::new(700),
            cam,
            rendu: RenduStandard::new(),
        }
    }

    fn charger(&mut self, forcer: Option<bool>) {
        self.spec = Specimen::aleatoire(forcer);
        self.sys = batir(&self.spec);
        self.cam.set_dist(dist_pour(&self.spec));
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        let dt = get_frame_time().min(0.05);

        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::G) {
            self.charger(None);
        }
        if is_key_pressed(KeyCode::Key1) {
            self.charger(Some(true));
        }
        if is_key_pressed(KeyCode::Key2) {
            self.charger(Some(false));
        }
        if is_key_pressed(KeyCode::P) {
            self.rendu.toggle_pixel();
        }
        if is_key_pressed(KeyCode::R) {
            crate::planete::vider_cache_materials();
            crate::soleil::vider_cache_materials();
            self.sys = batir(&self.spec);
            self.fond.recharger_material();
        }

        self.cam.input_orbite(false);
        self.sys.update(dt);

        let aspect = screen_width() / screen_height();
        let (cam_info, cam3d) = self.cam.construire(Vec3::ZERO, aspect);
        self.rendu
            .rendre(cam3d, &cam_info, &mut self.fond, &mut self.sys, false, false);

        draw_text(
            &format!(
                "{}   |   {} FPS   G: aleatoire   1: soleil   2: planete   P: pixel   R: shaders   Echap: menu",
                self.spec.label(),
                get_fps()
            ),
            12.0,
            24.0,
            18.0,
            WHITE,
        );
        false
    }
}
