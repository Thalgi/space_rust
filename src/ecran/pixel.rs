//! Filtre « pixel art » réutilisable pour les vues plein écran (briques,
//! stations, vaisseaux) : même mécanique que les planètes (`rendu.rs`) — la
//! scène 3D est rendue dans une cible basse résolution puis upscalée en plus
//! proche voisin. Le fond stellaire et les textes restent nets : seul ce qui
//! est dessiné entre `preparer` et `presenter` est pixelisé.

use macroquad::prelude::*;

const PIX: u32 = 2; // plus petit = plus de pixels

pub struct FiltrePixel {
    pub actif: bool,
    cible: Option<RenderTarget>,
    dims: (u32, u32),
}

impl FiltrePixel {
    pub fn new() -> Self {
        Self { actif: false, cible: None, dims: (0, 0) }
    }

    pub fn basculer(&mut self) {
        self.actif = !self.actif;
    }

    /// Si le filtre est actif : (re)crée la cible basse résolution, la nettoie
    /// en transparent (elle sera composée PAR-DESSUS le décor net) et branche
    /// `cam3d` dessus. Sans filtre : ne touche à rien.
    pub fn preparer(&mut self, cam3d: &mut Camera3D) {
        if !self.actif {
            return;
        }
        let dims = (
            (screen_width() as u32 / PIX).max(2),
            (screen_height() as u32 / PIX).max(2),
        );
        if self.dims != dims || self.cible.is_none() {
            // depth: true — indispensable : sans attachement de profondeur, le
            // depth test est muet dans la cible et l'ordre de dessin gagne.
            let rt = render_target_ex(
                dims.0,
                dims.1,
                RenderTargetParams { sample_count: 1, depth: true },
            );
            rt.texture.set_filter(FilterMode::Nearest);
            self.cible = Some(rt);
            self.dims = dims;
        }
        // Nettoyage transparent : le vide laisse voir le fond net en dessous.
        set_camera(&Camera2D {
            render_target: self.cible.clone(),
            ..Default::default()
        });
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        set_default_camera();
        cam3d.render_target = self.cible.clone();
    }

    /// Blit Nearest de la cible sur tout l'écran (après `set_default_camera`).
    pub fn presenter(&self) {
        if !self.actif {
            return;
        }
        if let Some(rt) = &self.cible {
            draw_texture_ex(
                &rt.texture,
                0.0,
                0.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(screen_width(), screen_height())),
                    flip_y: true,
                    ..Default::default()
                },
            );
        }
    }
}
