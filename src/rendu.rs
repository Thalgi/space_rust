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
        orbites_planetes: bool,
        orbites_etoiles: bool,
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
        orbites_planetes: bool,
        orbites_etoiles: bool,
        zone: bool,
    ) {
        if self.pixelise {
            let w = (screen_width() as u32 / PIX_SCALE).max(2);
            let h = (screen_height() as u32 / PIX_SCALE).max(2);
            if (w, h) != self.rt {
                // depth: true — sans attachement de profondeur, le depth test
                // est muet dans la cible et l'ordre de dessin gagne (ceintures
                // par-dessus planètes/soleil en mode pixel).
                self.cible = render_target_ex(
                    w,
                    h,
                    RenderTargetParams { sample_count: 1, depth: true },
                );
                self.cible.texture.set_filter(FilterMode::Nearest);
                self.rt = (w, h);
            }
            cam3d.render_target = Some(self.cible.clone());
            // Clamp sub-pixel des champs de débris : la cible est en demi-
            // résolution, et le plus petit débris = 1 pixel de la grille.
            crate::disque::set_viewport_h(h as f32);
            crate::disque::set_px_min(1.0);
        }

        set_camera(&cam3d);
        clear_background(BLACK);
        fond.draw(cam); // étoiles lointaines (derrière tout)
        sys.draw(cam, orbites_planetes, orbites_etoiles, zone);

        set_default_camera();
        crate::disque::set_viewport_h(0.0);
        crate::disque::set_px_min(0.0);
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
