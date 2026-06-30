use crate::astre::{Astre, CameraInfo};
use crate::genese::{catalogue_gazeuses, catalogue_telluriques, Habitabilite};
use crate::planete::Planete;
use crate::ui::minitel_ligne;
use macroquad::prelude::*;
use macroquad::rand::srand;

/// Statut d'habitabilité humaine pour l'affichage en galerie.
#[derive(Clone, Copy)]
enum Habitation {
    Colonisable,       // ∖ plein air
    HabitableFermee,   // ∕ colonies scellées
    HabitableExtreme,  // ∔ conditions extrêmes supportables
    Inhabitable,       // ∗ trop hostile
}

impl Habitation {
    fn from_hab(h: Habitabilite) -> Self {
        match h {
            Habitabilite::Colonisable => Habitation::Colonisable,
            Habitabilite::ColonieFermee => Habitation::HabitableFermee,
            Habitabilite::Extreme => Habitation::HabitableExtreme,
            Habitabilite::Inhabitable => Habitation::Inhabitable,
        }
    }

    /// Retourne le texte du jeton + sa couleur.
    fn token(&self) -> (&str, Color) {
        match self {
            Habitation::Colonisable => ("∖ ", Color::new(0.2, 1.0, 0.3, 1.0)),   // vert vif
            Habitation::HabitableFermee => ("∕ ", Color::new(1.0, 0.85, 0.1, 1.0)), // jaune doré
            Habitation::HabitableExtreme => ("∔ ", Color::new(1.0, 0.45, 0.1, 1.0)), // orange-rouge
            Habitation::Inhabitable => ("∗ ", Color::new(1.0, 0.2, 0.2, 1.0)),   // rouge gras
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
    font: Font,    // police Minitel (chargée async dans main.rs)
}

impl Galerie {
    /// Crée une galerie avec la police fournie.
    pub fn new(gazeuse: bool, font: Font) -> Self {
        let mut g = Self {
            seed: 1,
            cellules: Vec::new(),
            scroll: 0.0,
            jour: false,
            villes: 2, // démarre sur « actuel » (niveau 1.0)
            gazeuse,
            font,
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
            .map(|p| {
                let habitation = Habitation::from_hab(p.habitabilite);
                (
                    p.nom,
                    p.rare,
                    habitation,
                    Planete::new(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, p.apparence, Vec::new()),
                )
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
        let cols = ((screen_width() / 400.0).floor() as usize).clamp(1, n);
        let cw = screen_width() / cols as f32;
        let ch = 200.0;
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
        let cyan = Color::new(0.3, 0.7, 1.0, 1.0);
        let font_size: u16 = 12;

        for (nom, rare, habitation, cell_x, y) in &labels {
            // Construction du label complet : [gazeuse] [habitation] [R] Nom
            let (token_text, token_col) = habitation.token();

            // Largeurs individuelles pour centrer le bloc complet dans la cellule.
            let gazeuse_w = if self.gazeuse {
                measure_text("∓ ", Some(&self.font), font_size, 1.0).width
            } else {
                0.0
            };
            let token_w = measure_text(token_text, Some(&self.font), font_size, 1.0).width;
            let rare_w = if *rare { measure_text("[R] ", Some(&self.font), font_size, 1.0).width } else { 0.0 };
            let nom_w = measure_text(nom, Some(&self.font), font_size, 1.0).width;
            let total_w = gazeuse_w + token_w + rare_w + nom_w;

            let x_base = cell_x + (cw - total_w) * 0.5;
            let mut x_cur = x_base;

            // Badge gazeuse ∓ (si on est dans la galerie gazeuse).
            if self.gazeuse {
                draw_text_ex("∓ ", x_cur, *y, TextParams {
                    font: Some(&self.font),
                    font_size,
                    color: cyan,
                    ..Default::default()
                });
                x_cur += gazeuse_w;
            }

            // Jeton d'habitabilité.
            draw_text_ex(token_text, x_cur, *y, TextParams {
                font: Some(&self.font),
                font_size :20,
                color: token_col,
                ..Default::default()
            });
            x_cur += token_w;

            // Badge rareté.
            if *rare {
                draw_text_ex("[R] ", x_cur, *y, TextParams {
                    font: Some(&self.font),
                    font_size,
                    color: violet,
                    ..Default::default()
                });
                x_cur += rare_w;
            }

            // Nom de l'astre.
            draw_text_ex(nom, x_cur, *y, TextParams {
                font: Some(&self.font),
                font_size,
                color: nom_col,
                ..Default::default()
            });
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

        // Légende en bas à gauche (police Minitel).
        let leg_x = 12.0;
        let leg_y = 56.0;
        draw_text_ex("∖ Colonisable   ∕ Fermée   ∔ Extrême   ∗ Inhabitable   ∓ Gazeuse   [R] Rare", leg_x, leg_y, TextParams {
            font: Some(&self.font),
            font_size: 10,
            color: Color::new(0.6, 0.8, 0.8, 1.0),
            ..Default::default()
        });

        draw_text_ex("molette: defiler   G: regenerer   R: shaders   Echap: menu", 12.0, 72.0, TextParams {
            font: Some(&self.font),
            font_size: 10,
            color: Color::new(0.6, 0.8, 0.8, 1.0),
            ..Default::default()
        });

        false
    }
}