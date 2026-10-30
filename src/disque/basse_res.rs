//! **Les nuages en basse résolution** (Opti-v2 O4, dette D6).
//!
//! Les nuages et les bras d'une T Tauri repeignent ~100 écrans par image : des
//! milliers de boules floues qui se chevauchent. Elles n'ont pas de détail
//! fin — les dessiner dans une cible réduite de [`REDUCTION`] par axe divise
//! leur remplissage par `REDUCTION²`. Leur matériau est purement additif
//! (One, One) : accumulées à part puis ajoutées à l'image, elles donnent la
//! même somme. Sur le petit PC, « tout » passe de 70 à 49 ms ; 1/4 ne
//! descendait qu'à 41 ms, et rien de mieux de près — retenu : 1/2
//! (2026-10-30).
//!
//! La cible a sa propre profondeur : une sphère **noire** à la place de
//! l'étoile y cache les nuages qui passent derrière elle — en additif, le noir
//! n'ajoute rien à l'image.
//!
//! La composition passe par le pipeline plasma, sans shader de plus : ses UV
//! décalées de 2 lui font lire la cible telle quelle (`soleil_plasma.frag`).

use macroquad::prelude::*;
use std::cell::RefCell;

/// Réduction par axe de la cible des nuages.
pub(super) const REDUCTION: u32 = 2;

thread_local! {
    static CIBLE: RefCell<Option<RenderTarget>> = const { RefCell::new(None) };
}

/// La caméra de l'image courante, redirigée vers la cible réduite.
struct Reduite {
    matrice: Mat4,
    cible: RenderTarget,
}

impl Camera for Reduite {
    fn matrix(&self) -> Mat4 {
        self.matrice
    }
    fn depth_enabled(&self) -> bool {
        true
    }
    fn render_pass(&self) -> Option<RenderPass> {
        Some(self.cible.render_pass.clone())
    }
    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        None
    }
}

/// Ce qu'il faut pour revenir à l'image : sa matrice et son viewport.
pub(super) struct Retour {
    matrice: Mat4,
    viewport: Option<(i32, i32, i32, i32)>,
    passe: RenderPass,
    taille: (i32, i32),
}

impl Retour {
    /// La matrice de l'image, que la cible reprend.
    pub fn matrice(&self) -> Mat4 {
        self.matrice
    }

    /// La cible, pour qui y dessine sans macroquad (`nuages_gpu`) : sa
    /// passe miniquad et sa taille.
    pub fn cible(&self) -> (macroquad::miniquad::RenderPass, (i32, i32)) {
        (self.passe.raw_miniquad_id(), self.taille)
    }
}

/// Bascule le dessin vers la cible réduite (vidée), avec la matrice de
/// l'image courante. À refermer par [`fermer`].
pub(super) fn ouvrir(viewport_h: f32) -> Retour {
    let h = (viewport_h as u32 / REDUCTION).max(2);
    let l = ((viewport_h * screen_width() / screen_height().max(1.0)) as u32 / REDUCTION).max(2);
    let cible = CIBLE.with(|c| {
        let mut c = c.borrow_mut();
        if c.as_ref().is_none_or(|rt| rt.texture.size() != vec2(l as f32, h as f32)) {
            let rt = render_target_ex(l, h, RenderTargetParams { sample_count: 1, depth: true });
            rt.texture.set_filter(FilterMode::Linear);
            *c = Some(rt);
        }
        c.clone().unwrap()
    });
    let gl = unsafe { get_internal_gl() }.quad_gl;
    // ⚠️ Sans viewport posé, `get_viewport` rend l'**écran** entier : le
    // reposer tel quel, dans la cible deux fois plus petite du rendu Palette,
    // grossissait et décalait toute la suite de l'image.
    let vp = gl.get_viewport();
    let ecran = (0, 0, screen_width() as i32, screen_height() as i32);
    let retour = Retour {
        matrice: gl.get_projection_matrix(),
        viewport: (vp != ecran).then_some(vp),
        passe: cible.render_pass.clone(),
        taille: (l as i32, h as i32),
    };
    push_camera_state();
    set_camera(&Reduite { matrice: retour.matrice, cible });
    clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
    retour
}

/// Revient à l'image et y ajoute la cible : un quad qui couvre le viewport,
/// posé par la matrice inverse juste derrière le plan proche.
///
/// ⚠️ `pop_camera_state` de macroquad ne rend **pas** le viewport : les
/// galeries, qui dessinent par cellule, se retrouvaient en plein écran.
pub(super) fn fermer(retour: Retour) {
    pop_camera_state();
    unsafe { get_internal_gl() }.quad_gl.viewport(retour.viewport);
    let inv = retour.matrice.inverse();
    let coin = |x: f32, y: f32| {
        let p = inv * vec4(x, y, -0.9999, 1.0);
        Vertex::new2(p.truncate() / p.w, vec2(2.0 + (x + 1.0) * 0.5, 2.0 + (y + 1.0) * 0.5), WHITE)
    };
    let texture = CIBLE.with(|c| c.borrow().as_ref().map(|rt| rt.texture.clone()));
    gl_use_material(&crate::soleil::materiau_ruban_lumineux());
    draw_mesh(&Mesh {
        vertices: vec![coin(-1.0, -1.0), coin(1.0, -1.0), coin(1.0, 1.0), coin(-1.0, 1.0)],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture,
    });
    gl_use_default_material();
}
