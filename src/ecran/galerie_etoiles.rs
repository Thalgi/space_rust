use crate::astre::{Astre, CameraInfo};
use crate::etoile::couleur_corps_noir;
use crate::soleil::Soleil;
use crate::ui::minitel_ligne;
use macroquad::prelude::*;

/// Catalogue des étoiles : (nom, rayon, température K, luminosité, couronne 0=halo 1=jets 2=vent).
fn catalogue() -> Vec<(&'static str, f32, f32, f32, f32)> {
    vec![
        // Séquence principale O -> M.
        ("O (bleue)", 1.05, 38000.0, 4.0, 0.0),
        ("B (bleu-blanc)", 0.98, 20000.0, 3.0, 0.0),
        ("A (blanche)", 0.9, 9000.0, 2.0, 0.0),
        ("F (blanc-jaune)", 0.85, 6800.0, 1.4, 0.0),
        ("G (jaune - Soleil)", 0.8, 5700.0, 1.0, 0.0),
        ("K (orange)", 0.75, 4500.0, 0.7, 0.0),
        ("M (naine rouge)", 0.58, 3200.0, 0.35, 0.0),
        // Naines / sous-naines.
        ("Naine blanche", 0.45, 16000.0, 0.3, 0.0),
        ("Sous-naine (sdB)", 0.55, 25000.0, 0.6, 0.0),
        // Géantes / supergéantes.
        ("Geante rouge", 1.15, 3400.0, 4.5, 0.0),
        ("Supergeante rouge", 1.2, 3500.0, 6.0, 0.0),
        ("Supergeante bleue", 1.15, 21000.0, 6.0, 2.0), // vent stellaire
        // Particulières.
        ("Etoile carbonee (C)", 1.0, 2800.0, 2.5, 0.0),
        ("T Tauri (jeune)", 0.85, 4200.0, 1.2, 1.0), // jets bipolaires
        ("Wolf-Rayet", 1.05, 40000.0, 5.0, 2.0),     // vent violent
        ("Etoile a flares (M)", 0.58, 3000.0, 0.4, 0.0),
        ("Etoile a neutrons", 0.42, 40000.0, 0.2, 1.0), // jets fixes
        ("Pulsar", 0.42, 40000.0, 0.25, 3.0),           // jets qui tournent (phare)
        ("Magnetar", 0.45, 40000.0, 0.3, 4.0), // arcs magnétiques
        ("Trou noir", 0.62, 6500.0, 0.4, 5.0), // horizon + disque d'accretion (stylise)
    ]
}

/// Galerie « planche-contact » des types d'étoiles (visualisation/calibration).
pub struct GalerieEtoiles {
    cellules: Vec<(String, Soleil)>,
    scroll: f32,
}

impl GalerieEtoiles {
    pub fn new() -> Self {
        let cellules = catalogue()
            .into_iter()
            .map(|(nom, rayon, temp, lumi, mode)| {
                let s = Soleil::new(Vec3::ZERO, rayon, couleur_corps_noir(temp), lumi);
                let s = if mode > 4.5 {
                    s.avec_trou_noir()
                } else if mode > 3.5 {
                    s.avec_magnetar()
                } else if mode > 2.5 {
                    s.avec_pulsar()
                } else if mode > 1.5 {
                    s.avec_vent()
                } else if mode > 0.5 {
                    s.avec_jets()
                } else {
                    s
                };
                (nom.to_string(), s)
            })
            .collect();
        Self { cellules, scroll: 0.0 }
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::R) {
            crate::soleil::vider_cache_materials();
            *self = Self::new();
        }
        let dt = get_frame_time().min(0.05);

        clear_background(Color::new(0.01, 0.01, 0.03, 1.0));

        let n = self.cellules.len().max(1);
        let top = 48.0;
        let label_h = 22.0;
        let cols = ((screen_width() / 210.0).floor() as usize).clamp(1, n);
        let cw = screen_width() / cols as f32;
        let ch = 180.0;
        let render_h = ch - label_h;
        let rows = (n + cols - 1) / cols;
        let h_vue = screen_height() - top;
        let max_scroll = (rows as f32 * ch - h_vue).max(0.0);
        self.scroll = (self.scroll - mouse_wheel().1 * 48.0).clamp(0.0, max_scroll);

        // --- Phase 3D : dessiner chaque étoile dans son viewport. ---
        let mut labels: Vec<(String, f32, f32)> = Vec::new();
        for (i, (nom, soleil)) in self.cellules.iter_mut().enumerate() {
            soleil.update(dt); // animation (granulation, taches, éruptions)
            let cell_x = (i % cols) as f32 * cw;
            let cell_y = top + (i / cols) as f32 * ch - self.scroll;
            if cell_y + render_h < top || cell_y > screen_height() {
                continue;
            }
            let pos = vec3(0.0, 0.0, 7.5); // recul pour cadrer couronne / jets / vent
            let cam3d = Camera3D {
                position: pos,
                target: Vec3::ZERO,
                up: Vec3::Y,
                fovy: 45.0_f32.to_radians(),
                aspect: Some(cw / render_h),
                viewport: Some((
                    cell_x as i32,
                    (screen_height() - (cell_y + render_h)) as i32,
                    cw as i32,
                    render_h as i32,
                )),
                ..Default::default()
            };
            set_camera(&cam3d);

            let forward = (Vec3::ZERO - pos).normalize();
            let right = forward.cross(Vec3::Y).normalize();
            let up = right.cross(forward).normalize();
            let cam = CameraInfo {
                pos,
                right,
                up,
                forward,
                light_pos: vec3(0.0, 0.0, 10.0),
                light_color: Vec3::ONE,
                lights_pos: [vec3(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
                lights_color: [Vec3::ONE, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            };
            soleil.draw(&cam);
            labels.push((nom.clone(), cell_x, cell_y + render_h + 16.0));
        }

        // --- Phase 2D : texte après une seule remise en caméra écran. ---
        set_default_camera();
        let col = Color::new(0.9, 0.85, 0.6, 1.0);
        for (nom, cell_x, y) in &labels {
            let tw = crate::police::mesure(nom, 18);
            crate::police::texte(nom, cell_x + (cw - tw) * 0.5, *y, 18.0, col);
        }

        draw_rectangle(0.0, 0.0, screen_width(), top, Color::new(0.01, 0.01, 0.03, 1.0));
        crate::police::texte(
            "GALERIE DES ETOILES   -   molette: defiler   R: shaders   Echap: menu",
            12.0,
            30.0,
            18.0,
            Color::new(0.7, 0.85, 0.85, 1.0),
        );
        false
    }
}
