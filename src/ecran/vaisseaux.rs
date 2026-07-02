use crate::camera::Camera;
use crate::fond::Fond;
use crate::vaisseau::TypeEngin;
use macroquad::prelude::*;

/// Vue « hangar » : exemples de sonde, navette et station, navigables au
/// clavier. Maquettes isolées à l'origine (pas de `Systeme`) en attendant la
/// sélection/déplacement dans la starmap — voir `crate::vaisseau`.
pub struct Vaisseaux {
    courant: TypeEngin,
    cam: Camera,
    fond: Fond,
}

impl Vaisseaux {
    pub fn new() -> Self {
        let courant = TypeEngin::Sonde;
        Self {
            cam: Camera::new(courant.distance_camera()),
            courant,
            fond: Fond::new(500),
        }
    }

    fn choisir(&mut self, t: TypeEngin) {
        self.courant = t;
        self.cam.set_dist(t.distance_camera());
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::Key1) {
            self.choisir(TypeEngin::Sonde);
        }
        if is_key_pressed(KeyCode::Key2) {
            self.choisir(TypeEngin::Navette);
        }
        if is_key_pressed(KeyCode::Key3) {
            self.choisir(TypeEngin::Station);
        }
        if is_key_pressed(KeyCode::Tab) {
            self.choisir(self.courant.suivant());
        }

        self.cam.input_orbite(false);

        let aspect = screen_width() / screen_height();
        let (cam_info, cam3d) = self.cam.construire(Vec3::ZERO, aspect);

        set_camera(&cam3d);
        clear_background(BLACK);
        self.fond.draw(&cam_info); // étoiles lointaines en arrière-plan
        self.courant.dessiner();
        set_default_camera();

        draw_text(
            &format!(
                "{}   |   1: sonde   2: navette   3: station   Tab: suivant   Echap: menu",
                self.courant.nom()
            ),
            12.0,
            24.0,
            18.0,
            WHITE,
        );
        false
    }
}
