use crate::astre::CameraInfo;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Base orthonormée **(droite, haut, avant)** d'une caméra orbitale de lacet
/// `yaw` et de tangage `pitch`. Indépendante de la distance : la direction
/// cible → caméra est déjà unitaire.
///
/// Sortie de [`Camera::construire`] pour être **testable sans contexte
/// graphique** — `Camera::new` lit la position de la souris, la struct est donc
/// inconstructible hors du run macroquad. La boussole d'axes de l'interface
/// s'appuie sur cette même base, et c'est ce partage qui garantit qu'elle ne peut
/// pas mentir sur l'orientation de la vue.
pub fn base_orbite(yaw: f32, pitch: f32) -> (Vec3, Vec3, Vec3) {
    let cp = pitch.cos();
    // Direction cible → caméra, unitaire (cp²·sin² + sin²p + cp²·cos² = 1).
    let dir = vec3(cp * yaw.sin(), pitch.sin(), cp * yaw.cos());
    let forward = -dir;
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward).normalize();
    (right, up, forward)
}

/// Caméra orbitale : tourne autour d'une cible (origine ou astre focalisé),
/// gère le glisser/zoom et la sélection au clic.
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    dist_ref: f32,
    prec: (f32, f32),
    focus: Option<usize>,
}

impl Camera {
    pub fn new(dist: f32) -> Self {
        Self {
            yaw: 0.8,
            pitch: 0.5,
            dist,
            dist_ref: dist,
            prec: mouse_position(),
            focus: None,
        }
    }

    pub fn reset_focus(&mut self) {
        self.focus = None;
    }
    /// Focalise la caméra sur l'astre `idx` (le suit).
    pub fn set_focus(&mut self, idx: usize) {
        self.focus = Some(idx);
    }
    pub fn focus_actif(&self) -> bool {
        self.focus.is_some()
    }
    pub fn set_dist(&mut self, d: f32) {
        self.dist = d;
        self.dist_ref = d;
    }
    /// Multiplicateur de zoom courant : x1.00 au cadrage de référence, >1 en approchant.
    pub fn zoom(&self) -> f32 {
        self.dist_ref / self.dist
    }

    /// Rotation (glisser) + zoom (molette), sauf si la souris est sur l'UI.
    pub fn input_orbite(&mut self, sur_ui: bool) {
        let s = mouse_position();
        if is_mouse_button_down(MouseButton::Left) && !sur_ui {
            self.yaw -= (s.0 - self.prec.0) * 0.005;
            self.pitch = (self.pitch + (s.1 - self.prec.1) * 0.005).clamp(-1.4, 1.4);
        }
        self.prec = s;
        if !sur_ui {
            let mol = mouse_wheel().1;
            if mol != 0.0 {
                self.dist = (self.dist * (1.0 - mol.signum() * 0.1)).clamp(2.0, 30000.0);
            }
        }
    }

    pub fn cible(&self, sys: &Systeme) -> Vec3 {
        match self.focus {
            Some(i) => sys.position(i),
            None => Vec3::ZERO,
        }
    }

    /// Construit le repère caméra (billboards/éclairage) et la caméra 3D.
    pub fn construire(&self, target: Vec3, aspect: f32) -> (CameraInfo, Camera3D) {
        let (right, up, forward) = base_orbite(self.yaw, self.pitch);
        let pos = target - forward * self.dist;
        let info = CameraInfo {
            pos,
            right,
            up,
            forward,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        };
        let cam3d = Camera3D {
            position: pos,
            target,
            up: Vec3::Y,
            aspect: Some(aspect),
            ..Default::default()
        };
        (info, cam3d)
    }

    /// Sélectionne l'astre cliqué (rayon depuis la souris) comme nouvelle cible.
    pub fn pick(&mut self, sys: &Systeme, cam: &CameraInfo, aspect: f32) {
        let s = mouse_position();
        let ndc_x = s.0 / screen_width() * 2.0 - 1.0;
        let ndc_y = 1.0 - s.1 / screen_height() * 2.0;
        let th = (45.0_f32.to_radians() * 0.5).tan();
        let dir =
            (cam.forward + cam.right * (ndc_x * th * aspect) + cam.up * (ndc_y * th)).normalize();
        if let Some(idx) = sys.pick(cam.pos, dir) {
            self.focus = Some(idx);
        }
    }
}
