use crate::astre::{Astre, CameraInfo};
use crate::genese::{catalogue_gazeuses, catalogue_telluriques};
use crate::planete::Planete;
use crate::ui::minitel_ligne;
use macroquad::prelude::*;
use macroquad::rand::srand;

/// Galerie « planche-contact » : affiche en grille tous les types de telluriques
/// générables, nom dessous. Sert à valider visuellement les changements de rendu.
pub struct Galerie {
    seed: u64,
    cellules: Vec<(String, bool, Planete)>, // (nom, rare, planète)
    scroll: f32,
    scroll_cible: f32, // le scroll réel converge vers la cible (défilement doux)
    // Filtre pixel (P) : phase 3D rendue en basse résolution puis upscalée
    // en plus proche voisin ; les textes restent nets.
    pixelise: bool,
    cible: Option<RenderTarget>,
    rt_dims: (u32, u32),
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
            scroll_cible: 0.0,
            pixelise: false,
            cible: None,
            rt_dims: (0, 0),
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
                (nom, rare, Planete::new(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, app, Vec::new()))
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
        if is_key_pressed(KeyCode::P) {
            self.pixelise = !self.pixelise; // filtre pixel ON/OFF
        }
        if is_key_pressed(KeyCode::B) {
            // Bench complet en tâche de fond (telluriques uniquement) ->
            // bench_terrain.txt ; progression affichée dans l'overlay du bas.
            let presets: Vec<(String, crate::planete::Apparence)> = self
                .cellules
                .iter()
                .filter(|(_, _, p)| p.apparence().type_p == crate::planete::TypePlanete::Tellurique)
                .map(|(nom, _, p)| (nom.clone(), p.apparence()))
                .collect();
            crate::planete::terrain::bench(presets);
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
        // Défilement doux : la molette déplace une CIBLE, le scroll réel y
        // converge exponentiellement (indépendant du framerate).
        self.scroll_cible = (self.scroll_cible - mouse_wheel().1 * 84.0).clamp(0.0, max_scroll);
        let lisse = 1.0 - (-get_frame_time() * 10.0).exp();
        self.scroll += (self.scroll_cible - self.scroll) * lisse;
        if (self.scroll_cible - self.scroll).abs() < 0.3 {
            self.scroll = self.scroll_cible; // évite le tremblement sub-pixel en pixel-art
        }

        // Jour = lumière devant la caméra (face éclairée) ; nuit = lumière derrière
        // (on voit la face nuit -> villes et lueurs visibles). Une seule lumière.
        let light_pos = if self.jour {
            vec3(2.5, 1.8, 7.0)
        } else {
            vec3(-3.0, 1.2, -7.0)
        };

        // Filtre pixel : cible basse résolution (recréée si la fenêtre change).
        const PIX: u32 = 2;
        if self.pixelise {
            let dims = (
                (screen_width() as u32 / PIX).max(2),
                (screen_height() as u32 / PIX).max(2),
            );
            if self.rt_dims != dims || self.cible.is_none() {
                let rt = render_target(dims.0, dims.1);
                rt.texture.set_filter(FilterMode::Nearest);
                self.cible = Some(rt);
                self.rt_dims = dims;
            }
            // Nettoyage de la cible au fond d'écran.
            set_camera(&Camera2D {
                render_target: self.cible.clone(),
                ..Default::default()
            });
            clear_background(Color::new(0.02, 0.02, 0.05, 1.0));
            set_default_camera();
        }

        // --- Phase 3D : dessiner les planètes (viewport par cellule). Aucun texte ici. ---
        let mut labels: Vec<(String, bool, f32, f32)> = Vec::new();
        for (i, (nom, rare, planete)) in self.cellules.iter_mut().enumerate() {
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
            let mut cam3d = Camera3D {
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
            if self.pixelise {
                // Rendu dans la cible basse-déf : MÊME repère GL (bas-gauche)
                // que le chemin écran, à l'échelle près — le blit flip_y de la
                // phase 2D remet l'ensemble à l'endroit.
                let p = PIX as f32;
                cam3d.render_target = self.cible.clone();
                cam3d.viewport = Some((
                    (cell_x / p) as i32,
                    ((screen_height() - (cell_y + render_h)) / p) as i32,
                    (cw / p) as i32,
                    (render_h / p) as i32,
                ));
            }
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
            labels.push((nom.clone(), *rare, cell_x, cell_y + render_h + 16.0));
        }

        // --- Phase 2D : on remet la caméra écran UNE fois, puis tout le texte. ---
        set_default_camera();
        if self.pixelise {
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
        let nom_col = Color::new(0.7, 0.9, 0.8, 1.0);
        let violet = Color::new(0.72, 0.45, 1.0, 1.0);
        for (nom, rare, cell_x, y) in &labels {
            let prefix = if *rare { "[R] " } else { "" };
            let tw = measure_text(&format!("{}{}", prefix, nom), None, 18, 1.0).width;
            let x = cell_x + (cw - tw) * 0.5;
            if *rare {
                draw_text("[R] ", x, *y, 18.0, violet);
                let pw = measure_text("[R] ", None, 18, 1.0).width;
                draw_text(nom, x + pw, *y, 18.0, nom_col);
            } else {
                draw_text(nom, x, *y, 18.0, nom_col);
            }
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
        draw_text(
            "molette: defiler   G: regenerer   R: shaders   P: filtre pixel   C: capturer   B: bench   Echap: menu",
            12.0,
            56.0,
            16.0,
            Color::new(0.6, 0.8, 0.8, 1.0),
        );

        // Overlay de PERFORMANCES : FPS + stats de génération + état du bench.
        let (nb, dernier, total) = crate::planete::terrain::stats();
        let moyen = if nb > 0 { total / nb } else { 0 };
        let bench_txt = crate::planete::terrain::bench_etat()
            .unwrap_or_else(|| "B: lancer le bench".to_string());
        draw_text(
            &format!(
                "{} FPS   pixel: {}   terrains: {}   dernier: {} ms   moyen: {} ms   |   {}",
                get_fps(),
                if self.pixelise { "ON" } else { "off" },
                nb,
                dernier,
                moyen,
                bench_txt,
            ),
            12.0,
            screen_height() - 10.0,
            16.0,
            Color::new(0.55, 0.75, 0.75, 1.0),
        );

        // Capture de NON-RÉGRESSION (C) : exporte chaque cellule visible en
        // PNG dans captures/<horodatage>_seed<N>/. Avant/après une évolution
        // du pipeline, on compare les dossiers -> rien ne casse en silence.
        if is_key_pressed(KeyCode::C) {
            self.capturer(top, cols, cw, ch, render_h);
        }
        false
    }

    /// Exporte les cellules visibles (terrain prêt uniquement) en PNG.
    fn capturer(&self, top: f32, cols: usize, cw: f32, ch: f32, render_h: f32) {
        let img = get_screen_data();
        let (sw, sh) = (img.width as i32, img.height as i32);
        let tag = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dossier = format!("captures/{}_seed{}", tag, self.seed);
        if std::fs::create_dir_all(&dossier).is_err() {
            return;
        }
        // get_screen_data() est renversée verticalement (repère GL) : on lit
        // les lignes en miroir. Passer à `false` si les PNG sortent à l'envers.
        const RENVERSEE: bool = true;
        let mut nb = 0;
        for (i, (nom, _, planete)) in self.cellules.iter().enumerate() {
            let cell_x = (i % cols) as f32 * cw;
            let cell_y = top + (i / cols) as f32 * ch - self.scroll;
            if cell_y < top || cell_y + render_h > screen_height() || !planete.terrain_pret() {
                continue; // hors écran, partiel, ou placeholder en cours
            }
            let (x0, y0, w, h) = (cell_x as i32, cell_y as i32, cw as i32, render_h as i32);
            if x0 + w > sw || y0 + h > sh {
                continue;
            }
            let mut out = Image::gen_image_color(w as u16, h as u16, BLANK);
            for ry in 0..h {
                let sy = if RENVERSEE { sh - 1 - (y0 + ry) } else { y0 + ry };
                for rx in 0..w {
                    out.set_pixel(rx as u32, ry as u32, img.get_pixel((x0 + rx) as u32, sy as u32));
                }
            }
            let slug: String = nom
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            out.export_png(&format!("{}/{}.png", dossier, slug));
            nb += 1;
        }
        println!("capture: {} cellule(s) -> {}", nb, dossier);
    }
}
