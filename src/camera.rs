use crate::astre::CameraInfo;
use crate::systeme::Systeme;
use macroquad::prelude::*;

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
        let cp = self.pitch.cos();
        let offset = vec3(
            self.dist * cp * self.yaw.sin(),
            self.dist * self.pitch.sin(),
            self.dist * cp * self.yaw.cos(),
        );
        let pos = target + offset;
        let forward = (target - pos).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();
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
