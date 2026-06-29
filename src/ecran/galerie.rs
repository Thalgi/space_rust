use crate::astre::{Astre, CameraInfo};
use crate::genese::{catalogue_gazeuses, catalogue_telluriques, est_colonisable, est_habitable_fermee};
use crate::planete::Planete;
use crate::ui::minitel_ligne;
use macroquad::prelude::*;
use macroquad::rand::srand;

/// Statut d'habitabilité humaine pour l'affichage en galerie.
#[derive(Clone, Copy)]
enum Habitation {
    Colonisable,      // 🟢 plein air
    HabitableFermee,  // 🟡 colonies scellées
    Inhabitable,      // ❌ trop hostile
}

impl Habitation {
    fn from_nom(nom: &str) -> Self {
        if est_colonisable(nom) {
            Habitation::Colonisable
        } else if est_habitable_fermee(nom) {
            Habitation::HabitableFermee
        } else {
            Habitation::Inhabitable
        }
    }

    /// Retourne le texte du jeton + sa couleur.
    fn token(&self) -> (&str, Color) {
        match self {
            Habitation::Colonisable => ("[O] ", Color::new(0.2, 1.0, 0.3, 1.0)),   // vert vif
            Habitation::HabitableFermee => ("[0] ", Color::new(1.0, 0.85, 0.1, 1.0)), // jaune doré
            Habitation::Inhabitable => ("[X] ", Color::new(1.0, 0.2, 0.2, 1.0)),   // rouge gras
        }
    }
}

/// Galerie « planche-contact » : affiche en grille tous les types de telluriques
/// générables, nom dessous. Sert à valider visuellement les changements de rendu.
pub struct Galerie {
    seed: u64,
    cellules: Vec<(String, bool, Habitation, Planete)>, // (nom, rare, habitation, planète)
    scroll: f32,
    jour: bool,
    villes: u8, // index 0..4 -> niveau 0, 0.5, 1, 1.5, 2
    gazeuse: bool, // false = telluriques, true = géantes gazeuses
}

impl Galerie {
    pub fn new(gazeuse: bool) -> Self {
        let mut g = Self {
            seed: 1,
            cellules: Vec::new(),
            scroll: 0.0,
            jour: false,
            villes: 2, // démarre sur « actuel » (niveau 1.0)
            gazeuse,
        };
        g.construire();
        g
    }

    fn construire(&mut self) {
        srand(self.seed); // catalogue reproductible pour une graine donnée
        let catalogue = if self.gazeuse {
            catalogue_gazeuses()
        } else {
            catalogue_telluriques()
        };
        self.cellules = catalogue
            .into_iter()
            .map(|(nom, app)| {
                let rare = crate::genese::est_rare(&nom);
                let habitation = Habitation::from_nom(&nom);
                (nom, rare, habitation, Planete::new(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, app, Vec::new()))
            })
            .collect();
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::G) {
            self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.construire();
        }
        if is_key_pressed(KeyCode::R) {
            crate::planete::vider_cache_materials();
            self.construire();
        }

        // Boutons Minitel (jour/nuit, lumières de villes) en haut à gauche.
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);
        let btn_jour = Rect::new(12.0, 8.0, 200.0, 26.0);
        let btn_villes = Rect::new(222.0, 8.0, 230.0, 26.0);
        if clic && btn_jour.contains(m) {
            self.jour = !self.jour;
        }
        if !self.gazeuse && clic && btn_villes.contains(m) {
            self.villes = (self.villes + 1) % 5; // 0, 0.5, 1, 1.5, 2 (demi-paliers)
        }

        clear_background(Color::new(0.02, 0.02, 0.05, 1.0));

        let n = self.cellules.len().max(1);
        let top = 64.0;
        let label_h = 22.0;
        // Cases de taille fixe -> grille défilable à la molette (lisible même à 60+ mondes).
        let cols = ((screen_width() / 200.0).floor() as usize).clamp(1, n);
        let cw = screen_width() / cols as f32;
        let ch = 168.0;
        let render_h = ch - label_h;
        let rows = (n + cols - 1) / cols;
        let h_vue = screen_height() - top;
        let max_scroll = (rows as f32 * ch - h_vue).max(0.0);
        self.scroll = (self.scroll - mouse_wheel().1 * 48.0).clamp(0.0, max_scroll);

        // Jour = lumière devant la caméra (face éclairée) ; nuit = lumière derrière
        // (on voit la face nuit -> villes et lueurs visibles). Une seule lumière.
        let light_pos = if self.jour {
            vec3(2.5, 1.8, 7.0)
        } else {
            vec3(-3.0, 1.2, -7.0)
        };

        // --- Phase 3D : dessiner les planètes (viewport par cellule). Aucun texte ici. ---
        let mut labels: Vec<(String, bool, Habitation, f32, f32)> = Vec::new();
        for (i, (nom, rare, habitation, planete)) in self.cellules.iter_mut().enumerate() {
            let cell_x = (i % cols) as f32 * cw;
            let cell_y = top + (i / cols) as f32 * ch - self.scroll;
            // Hors écran -> on saute (pas de viewport inutile).
            if cell_y + render_h < top || cell_y > screen_height() {
                continue;
            }

            // Caméra par cellule : viewport pixel (origine bas-gauche en GL).
            // Les planètes à anneau sont vues de plus loin pour que l'anneau tienne dans la case.
            let (dist, haut) = if planete.a_un_anneau() {
                (3.2 * planete.rayon_anneau(), 0.18 * planete.rayon_anneau())
            } else {
                (3.0, 0.0) // vue inchangée pour les planètes sans anneau
            };
            let pos = vec3(0.0, haut, dist);
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
                light_pos,
                light_color: Vec3::ONE,
            };
            planete.set_villes(self.villes as f32 * 0.5);
            planete.draw(&cam);
            labels.push((nom.clone(), *rare, *habitation, cell_x, cell_y + render_h + 16.0));
        }

        // --- Phase 2D : on remet la caméra écran UNE fois, puis tout le texte. ---
        set_default_camera();
        let nom_col = Color::new(0.7, 0.9, 0.8, 1.0);
        let violet = Color::new(0.72, 0.45, 1.0, 1.0);

        for (nom, rare, habitation, cell_x, y) in &labels {
            // Construction du label complet : [habitation] [R] Nom
            let (token_text, token_col) = habitation.token();
            let rare_prefix = if *rare { "[R] " } else { "" };

            // Calculer la largeur totale pour centrer dans la cellule.
            let token_w = measure_text(token_text, None, 18, 1.0).width;
            let rare_w = if *rare { measure_text("[R] ", None, 18, 1.0).width } else { 0.0 };
            let nom_w = measure_text(nom, None, 18, 1.0).width;
            let total_w = token_w + rare_w + nom_w;

            let x_base = cell_x + (cw - total_w) * 0.5;

            // Dessin séquentiel : jeton d'habitabilité → badge rare → nom
            draw_text(token_text, x_base, *y, 18.0, token_col);
            let mut x_cur = x_base + token_w;

            if *rare {
                draw_text("[R] ", x_cur, *y, 18.0, violet);
                x_cur += rare_w;
            }

            draw_text(nom, x_cur, *y, 18.0, nom_col);
        }

        // Barre de titre + boutons par-dessus la grille.
        draw_rectangle(0.0, 0.0, screen_width(), top, Color::new(0.02, 0.02, 0.05, 1.0));
        minitel_ligne(btn_jour, if self.jour { "ECLAIRAGE: JOUR" } else { "ECLAIRAGE: NUIT" }, m);
        if !self.gazeuse {
            let label_villes = match self.villes {
                0 => "VILLES: 0 (aucune)",
                1 => "VILLES: 1 (faible)",
                2 => "VILLES: 2 (actuel)",
                3 => "VILLES: 3 (moyen)",
                _ => "VILLES: 4 (etendu)",
            };
            minitel_ligne(btn_villes, label_villes, m);
        }

        // Légende en bas à gauche.
        let leg_x = 12.0;
        let leg_y = 56.0;
        draw_text("[O] Colonisable   [0] Colonies fermées   [X] Inhabitable   [R] Rare", leg_x, leg_y, 20.0, Color::new(0.6, 0.8, 0.8, 1.0));

        draw_text(
            "molette: defiler   G: regenerer   R: shaders   Echap: menu",
            12.0,
            72.0,
            20.0,
            Color::new(0.6, 0.8, 0.8, 1.0),
        );
        false
    }
}