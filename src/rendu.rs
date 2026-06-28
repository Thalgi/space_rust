use crate::astre::CameraInfo;
use crate::fond::Fond;
use crate::systeme::Systeme;
use macroquad::prelude::*;

const PIX_SCALE: u32 = 2; // plus petit = plus de pixels

/// Contrôleur de rendu : sépare la stratégie d'affichage du reste du jeu.
/// On peut fournir une autre implémentation pour changer de pipeline.
pub trait Rendu {
    fn rendre(
        &mut self,
        cam3d: Camera3D,
        cam: &CameraInfo,
        fond: &mut Fond,
        sys: &mut Systeme,
        orbites: bool,
        zone: bool,
    );
    fn pixelise(&self) -> bool {
        false
    }
    fn toggle_pixel(&mut self) {}
}

/// Rendu standard : scène 3D, avec option « filtre pixel » (rendu basse-déf upscalé).
pub struct RenduStandard {
    pixelise: bool,
    cible: RenderTarget,
    rt: (u32, u32),
}

impl RenduStandard {
    pub fn new() -> Self {
        Self {
            pixelise: false,
            cible: render_target(2, 2),
            rt: (0, 0),
        }
    }
}

impl Rendu for RenduStandard {
    fn pixelise(&self) -> bool {
        self.pixelise
    }
    fn toggle_pixel(&mut self) {
        self.pixelise = !self.pixelise;
    }

    fn rendre(
        &mut self,
        mut cam3d: Camera3D,
        cam: &CameraInfo,
        fond: &mut Fond,
        sys: &mut Systeme,
        orbites: bool,
        zone: bool,
    ) {
        if self.pixelise {
            let w = (screen_width() as u32 / PIX_SCALE).max(2);
            let h = (screen_height() as u32 / PIX_SCALE).max(2);
            if (w, h) != self.rt {
                self.cible = render_target(w, h);
                self.cible.texture.set_filter(FilterMode::Nearest);
                self.rt = (w, h);
            }
            cam3d.render_target = Some(self.cible.clone());
        }

        set_camera(&cam3d);
        clear_background(BLACK);
        fond.draw(cam); // étoiles lointaines (derrière tout)
        sys.draw(cam, orbites, zone);

        set_default_camera();
        if self.pixelise {
            clear_background(BLACK);
            draw_texture_ex(
                &self.cible.texture,
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
