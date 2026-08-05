use crate::astre::CameraInfo;
use crate::ecran::pixel;
use crate::fond::Fond;
use crate::systeme::Systeme;
use macroquad::prelude::*;

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

/// Rendu standard. Le filtre « rétro » ne s'applique PAS à tout l'écran : seuls
/// les corps célestes sont rendus en basse résolution puis upscalés (pixel art).
/// Le fond stellaire et les trajectoires/zones restent nets, dessinés par-dessous.
///
/// L'interrupteur n'est plus ici : le mode vient des réglages
/// ([`crate::reglages::mode_rendu`]), pour que la touche P et l'écran des
/// paramètres ne racontent pas deux choses différentes.
pub struct RenduStandard {
    cible: RenderTarget,
    rt: (u32, u32),
}

impl RenduStandard {
    pub fn new() -> Self {
        Self {
            cible: render_target(2, 2),
            rt: (0, 0),
        }
    }
}

impl Rendu for RenduStandard {
    fn pixelise(&self) -> bool {
        crate::reglages::mode_rendu().pixelise()
    }
    fn toggle_pixel(&mut self) {
        crate::reglages::regler_rendu(crate::reglages::mode_rendu().suivant());
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
        // --- Couche NETTE (plein écran) : fond stellaire + trajectoires/zones ---
        set_camera(&cam3d);
        clear_background(BLACK);
        fond.draw(cam); // étoiles lointaines (derrière tout)
        sys.draw_orbites(orbites_planetes, orbites_etoiles, zone);
        set_default_camera();

        if self.pixelise() {
            // --- Couche PIXEL : corps célestes seuls, en basse résolution ---
            let (w, h) = pixel::dimensions();
            if (w, h) != self.rt {
                self.cible = pixel::nouvelle_cible(w, h);
                self.rt = (w, h);
            }
            cam3d.render_target = Some(self.cible.clone());
            // Clamp sub-pixel des champs de débris : la cible est en demi-
            // résolution, et le plus petit débris = 1 pixel de la grille.
            crate::disque::set_viewport_h(h as f32);
            crate::disque::set_px_min(1.0);
            // **Même correction pour les planètes.** `px_rayon` (le LOD du
            // shader) retombe sur `screen_height()` quand rien n'est réglé :
            // sans cette ligne, la planète est ombrée avec le détail d'un plein
            // écran alors qu'elle est dessinée dans une cible deux fois plus
            // petite. Le micro-détail qui en résulte ne tient pas dans les
            // pixels disponibles — il scintille, et la quantification l'amplifie.
            crate::planete::set_viewport_h(h as f32);

            set_camera(&cam3d);
            clear_background(Color::new(0.0, 0.0, 0.0, 0.0)); // transparent : ne masque pas le décor
            sys.draw_corps(cam);
            set_default_camera();
            crate::disque::set_viewport_h(0.0);
            crate::disque::set_px_min(0.0);
            crate::planete::set_viewport_h(0.0);

            // Upscale Nearest par-dessus la couche nette (alpha : corps opaques,
            // vide transparent), quantifié vers la palette si le mode le demande.
            pixel::blit(&self.cible.texture);
        } else {
            // Sans filtre : corps dessinés nets, dans la même passe que le décor.
            set_camera(&cam3d);
            sys.draw_corps(cam);
            set_default_camera();
        }
    }
}
