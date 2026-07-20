use crate::camera::Camera;
use crate::fond::Fond;
use crate::vaisseau::Brique;
use macroquad::prelude::*;

/// Vue « atelier de briques » : chaque groupe de composant (structure, habitat,
/// panneau solaire, radiateur, antenne, parabole…) est présenté isolément,
/// centré à l'origine, pour le travailler au cas par cas. Flèches haut/bas pour
/// changer de brique ; le nom s'affiche en bas à gauche.
pub struct Briques {
    courant: usize,
    cam: Camera,
    fond: Fond,
}

impl Briques {
    pub fn new() -> Self {
        let mut cam = Camera::new(6.0);
        cam.yaw = 0.7;
        cam.pitch = 0.35;
        let mut vue = Self {
            courant: 0,
            cam,
            fond: Fond::new(400),
        };
        vue.cadrer();
        vue
    }

    fn brique(&self) -> Brique {
        Brique::TOUS[self.courant]
    }

    fn cadrer(&mut self) {
        let d = self.brique().demi_dim();
        let demi = d.x.max(d.y);
        let demi_fov = 45.0_f32.to_radians() * 0.5;
        self.cam.set_dist((demi + 0.5) / demi_fov.tan() * 1.35);
    }

    fn changer(&mut self, delta: i32) {
        let n = Brique::TOUS.len() as i32;
        self.courant = (((self.courant as i32 + delta) % n + n) % n) as usize;
        self.cadrer();
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::Up) {
            self.changer(-1);
        }
        if is_key_pressed(KeyCode::Down) {
            self.changer(1);
        }

        self.cam.input_orbite(false);

        let aspect = screen_width() / screen_height();
        let (cam_info, cam3d) = self.cam.construire(Vec3::ZERO, aspect);

        set_camera(&cam3d);
        clear_background(BLACK);
        self.fond.draw(&cam_info);
        self.brique().dessiner();
        set_default_camera();

        // Nom de la brique courante, en bas à gauche.
        let h = screen_height();
        let gris = Color::new(0.70, 0.72, 0.78, 1.0);
        crate::police::texte(
            &format!("{} / {}", self.courant + 1, Brique::TOUS.len()),
            20.0,
            h - 54.0,
            16.0,
            gris,
        );
        crate::police::texte(self.brique().nom(), 20.0, h - 24.0, 30.0, WHITE);

        crate::police::texte(
            "Fleches haut/bas: brique   glisser: pivoter   molette: zoom   Echap: menu",
            12.0,
            24.0,
            18.0,
            WHITE,
        );
        false
    }
}
