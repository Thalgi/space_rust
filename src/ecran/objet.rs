use super::widgets::Panneau;
use crate::astre::{Astre, CameraInfo};
use crate::camera::Camera;
use crate::fond::Fond;
use crate::genese::{
    catalogue_gazeuses, catalogue_telluriques, charger_edits, sauver_edits, Habitabilite, PresetEdit,
    PresetPlanete,
};
use crate::planete::{Apparence, Planete, TypePlanete};
use crate::rendu::{Rendu, RenduStandard};
use crate::systeme::Systeme;
use macroquad::prelude::*;

const HAB_LABELS: [&str; 4] = ["Colonisable", "Colonie fermee", "Extreme", "Inhabitable"];
const FOOTER_H: f32 = 150.0; // bandeau fixe de boutons en bas du panneau d'édition

/// Instantané de l'état éditable (pour undo/redo et détection de modification).
#[derive(Clone, PartialEq)]
struct EtatEdit {
    app: Apparence,
    nom: String,
    hab: Habitabilite,
    rare: bool,
}

const BAR_H: f32 = 26.0; // hauteur de la barre/onglet du tiroir
const DRAWER_H: f32 = 134.0; // hauteur du tiroir ouvert (barre incluse)
const CW: f32 = 104.0; // pas horizontal entre vignettes (espacement)
const THUMB: f32 = 70.0; // taille rendue de la planète (plus petite que la cellule)
const PAD_TOP: f32 = 8.0; // marge entre le haut du tiroir et la vignette

/// Largeur du panneau d'infos (au moins 1/4 de l'écran).
fn panneau_w() -> f32 {
    (screen_width() * 0.27).max(300.0)
}

/// Vue isolée : visualiseur de planètes du catalogue, avec panneau d'infos (droite)
/// et un tiroir catalogue déroulable en bas (vignettes + recherche).
pub struct Objet {
    presets: Vec<PresetPlanete>,
    vignettes: Vec<Planete>, // mini-rendu de chaque preset (catalogue)
    idx: usize,
    sys: Systeme,
    fond: Fond,
    cam: Camera,
    rendu: RenduStandard,
    font: Font,
    drawer_ouvert: bool,
    recherche: String,
    drawer_scroll: f32,
    // --- Édition ---
    edition: bool,
    travail: Apparence,  // copie de travail (apparence éditée, planète affichée)
    t_nom: String,
    t_hab: Habitabilite,
    t_rare: bool,
    widget_actif: Option<usize>, // slider en cours de drag
    edit_scroll: f32,
    edit_contenu: f32, // hauteur totale du contenu (pour clamper le scroll)
    focus_nom: bool,   // le champ Nom a le focus clavier ?
    undo: Vec<EtatEdit>,
    redo: Vec<EtatEdit>,
    sauvegarde: EtatEdit,         // état de référence (entrée d'édition / dernière sauvegarde)
    edits: Vec<PresetEdit>,       // overrides persistés (catalogue_edits.json)
    confirm_idx: Option<usize>,   // changement de preset en attente (modifs non sauvées)
}

fn batir(p: &PresetPlanete) -> Systeme {
    batir_app(p.apparence)
}

/// Système à une planète à partir d'une apparence directe (aperçu d'édition).
fn batir_app(app: Apparence) -> Systeme {
    let mut sys = Systeme::new();
    sys.ajouter(Box::new(Planete::new(Vec3::ZERO, Vec3::ZERO, 1.5, 1.0, app, Vec::new())));
    sys.set_lumiere(vec3(6.0, 3.0, 6.0), Vec3::ONE);
    sys
}

fn hab_index(h: Habitabilite) -> usize {
    match h {
        Habitabilite::Colonisable => 0,
        Habitabilite::ColonieFermee => 1,
        Habitabilite::Extreme => 2,
        Habitabilite::Inhabitable => 3,
    }
}

fn index_hab(i: usize) -> Habitabilite {
    match i {
        0 => Habitabilite::Colonisable,
        1 => Habitabilite::ColonieFermee,
        2 => Habitabilite::Extreme,
        _ => Habitabilite::Inhabitable,
    }
}

/// Caméra fixe pour une vignette (planète de rayon 1, éclairée de face).
fn cam_vignette() -> CameraInfo {
    let pos = vec3(0.0, 0.0, 3.0);
    let forward = (Vec3::ZERO - pos).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward).normalize();
    CameraInfo { pos, right, up, forward, light_pos: vec3(2.0, 1.5, 6.0), light_color: Vec3::ONE }
}

impl Objet {
    pub fn new(font: Font) -> Self {
        let mut presets = catalogue_telluriques();
        presets.extend(catalogue_gazeuses());
        let vignettes = presets
            .iter()
            .map(|p| Planete::new(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, p.apparence, Vec::new()))
            .collect();
        let sys = batir(&presets[0]);
        let travail = presets[0].apparence;
        let t_nom = presets[0].nom.clone();
        let t_hab = presets[0].habitabilite;
        let t_rare = presets[0].rare;
        let sauvegarde = EtatEdit { app: travail, nom: t_nom.clone(), hab: t_hab, rare: t_rare };
        let edits = charger_edits();
        Self {
            presets,
            vignettes,
            idx: 0,
            sys,
            fond: Fond::new(700),
            cam: Camera::new(6.0),
            rendu: RenduStandard::new(),
            font,
            drawer_ouvert: false,
            recherche: String::new(),
            drawer_scroll: 0.0,
            edition: false,
            travail,
            t_nom,
            t_hab,
            t_rare,
            widget_actif: None,
            edit_scroll: 0.0,
            edit_contenu: 0.0,
            focus_nom: false,
            undo: Vec::new(),
            redo: Vec::new(),
            sauvegarde,
            edits,
            confirm_idx: None,
        }
    }

    fn snapshot(&self) -> EtatEdit {
        EtatEdit { app: self.travail, nom: self.t_nom.clone(), hab: self.t_hab, rare: self.t_rare }
    }

    fn appliquer_snapshot(&mut self, e: EtatEdit) {
        self.travail = e.app;
        self.t_nom = e.nom;
        self.t_hab = e.hab;
        self.t_rare = e.rare;
    }

    /// L'état courant diffère-t-il de la dernière sauvegarde / entrée d'édition ?
    fn modifie(&self) -> bool {
        self.snapshot() != self.sauvegarde
    }

    /// Le preset d'index `i` a-t-il un override sauvegardé (personnalisé) ?
    fn est_edite(&self, i: usize) -> bool {
        let id = &self.presets[i].id;
        self.edits.iter().any(|e| &e.id == id)
    }

    fn annuler(&mut self) {
        if let Some(e) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.appliquer_snapshot(e);
        }
    }

    fn refaire(&mut self) {
        if let Some(e) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.appliquer_snapshot(e);
        }
    }

    /// Sauvegarde explicite : applique en mémoire + upsert dans le JSON d'edits.
    fn sauver_edit(&mut self) {
        let id = self.presets[self.idx].id.clone();
        let edit = PresetEdit {
            id: id.clone(),
            nom: self.t_nom.clone(),
            habitabilite: self.t_hab,
            rare: self.t_rare,
            apparence: self.travail,
        };
        match self.edits.iter_mut().find(|e| e.id == id) {
            Some(e) => *e = edit,
            None => self.edits.push(edit),
        }
        sauver_edits(&self.edits);
        // Applique au preset + vignette en mémoire.
        let p = &mut self.presets[self.idx];
        p.nom = self.t_nom.clone();
        p.habitabilite = self.t_hab;
        p.rare = self.t_rare;
        p.apparence = self.travail;
        self.vignettes[self.idx] = Planete::new(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, self.travail, Vec::new());
        self.sauvegarde = self.snapshot();
    }

    /// Réinitialise : supprime l'override et revient aux valeurs procédurales du code.
    fn reset_edit(&mut self) {
        let id = self.presets[self.idx].id.clone();
        self.edits.retain(|e| e.id != id);
        sauver_edits(&self.edits);
        // Recharge un catalogue frais (edits restants appliqués) et extrait ce preset.
        let mut frais = catalogue_telluriques();
        frais.extend(catalogue_gazeuses());
        if let Some(p) = frais.into_iter().find(|p| p.id == id) {
            self.travail = p.apparence;
            self.t_nom = p.nom.clone();
            self.t_hab = p.habitabilite;
            self.t_rare = p.rare;
            self.presets[self.idx] = p;
            self.vignettes[self.idx] = Planete::new(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, self.travail, Vec::new());
        }
        self.sauvegarde = self.snapshot();
        self.undo.clear();
        self.redo.clear();
    }

    /// Demande de changement de preset : confirme si modifs non sauvegardées (en édition).
    fn demander_switch(&mut self, i: usize) {
        if self.edition && self.modifie() {
            self.confirm_idx = Some(i);
        } else {
            self.charger(i);
            if self.edition {
                self.sauvegarde = self.snapshot();
                self.undo.clear();
                self.redo.clear();
            }
        }
    }

    fn charger(&mut self, idx: usize) {
        self.idx = idx.min(self.presets.len() - 1);
        self.sys = batir(&self.presets[self.idx]);
        // Recharge la copie de travail sur le nouveau preset.
        self.travail = self.presets[self.idx].apparence;
        self.t_nom = self.presets[self.idx].nom.clone();
        self.t_hab = self.presets[self.idx].habitabilite;
        self.t_rare = self.presets[self.idx].rare;
    }

    /// Entre en mode édition (copie de travail déjà synchronisée par `charger`).
    fn entrer_edition(&mut self) {
        self.edition = true;
        self.edit_scroll = 0.0;
        self.focus_nom = false;
        self.sauvegarde = self.snapshot();
        self.undo.clear();
        self.redo.clear();
        self.sys = batir_app(self.travail);
    }

    /// Quitte le mode édition en appliquant la copie de travail au preset en mémoire
    /// (la persistance JSON viendra en Phase 4).
    fn quitter_edition(&mut self) {
        let p = &mut self.presets[self.idx];
        p.nom = self.t_nom.clone();
        p.habitabilite = self.t_hab;
        p.rare = self.t_rare;
        p.apparence = self.travail;
        self.vignettes[self.idx] = Planete::new(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, self.travail, Vec::new());
        self.edition = false;
        self.focus_nom = false;
        self.sys = batir(&self.presets[self.idx]);
    }

    fn filtres(&self) -> Vec<usize> {
        let q = self.recherche.to_lowercase();
        self.presets
            .iter()
            .enumerate()
            .filter(|(_, p)| q.is_empty() || p.nom.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    fn texte(&self, s: &str, x: f32, y: f32, taille: f32, col: Color) {
        draw_text_ex(
            s,
            x,
            y,
            TextParams { font: Some(&self.font), font_size: taille as u16, color: col, ..Default::default() },
        );
    }

    pub fn frame(&mut self) -> bool {
        let dt = get_frame_time().min(0.05);
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);
        let down = is_mouse_button_down(MouseButton::Left);
        let pw = panneau_w();
        let sh = screen_height();
        let main_w = screen_width() - pw;

        if is_key_pressed(KeyCode::Escape) {
            if self.focus_nom {
                self.focus_nom = false;
            } else if self.edition {
                self.quitter_edition();
            } else if self.drawer_ouvert {
                self.drawer_ouvert = false;
            } else {
                return true;
            }
        }
        if is_key_pressed(KeyCode::P) {
            self.rendu.toggle_pixel();
        }
        if is_key_pressed(KeyCode::R) {
            crate::planete::vider_cache_materials();
            self.sys = if self.edition { batir_app(self.travail) } else { batir(&self.presets[self.idx]) };
            self.fond.recharger_material();
        }
        // E : bascule l'édition (sauf si on est en train de taper dans le champ Nom).
        if !self.focus_nom && is_key_pressed(KeyCode::E) {
            if self.edition {
                self.quitter_edition();
            } else {
                self.entrer_edition();
            }
        }

        // Clic sur la barre du tiroir -> ouvrir/fermer (puis on fige la hauteur).
        let barre = Rect::new(0.0, sh - BAR_H, main_w, BAR_H);
        if clic && barre.contains(m) {
            self.drawer_ouvert = !self.drawer_ouvert;
        }
        let drawer_h = if self.drawer_ouvert { DRAWER_H } else { BAR_H };

        // Saisie clavier : priorité au champ Nom (édition), sinon recherche (tiroir).
        if self.focus_nom {
            while let Some(ch) = get_char_pressed() {
                if !ch.is_control() {
                    self.t_nom.push(ch);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.t_nom.pop();
            }
        } else if self.drawer_ouvert {
            while let Some(ch) = get_char_pressed() {
                if !ch.is_control() {
                    self.recherche.push(ch);
                    self.drawer_scroll = 0.0;
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.recherche.pop();
            }
        }

        // Tiroir : défilement + clic vignette ; feuilletage clavier hors édition.
        if self.drawer_ouvert {
            let sur_tiroir = m.y > sh - drawer_h && m.x < main_w;
            if sur_tiroir {
                self.drawer_scroll = (self.drawer_scroll - mouse_wheel().1 * 22.0).max(0.0);
                if clic && m.y < sh - BAR_H {
                    if let Some(i) = self.clic_vignette(m, main_w, drawer_h) {
                        self.demander_switch(i);
                    }
                }
            }
        } else if !self.edition {
            if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::G) {
                self.charger((self.idx + 1) % self.presets.len());
            }
            if is_key_pressed(KeyCode::Left) {
                let n = self.presets.len();
                self.charger((self.idx + n - 1) % n);
            }
        }

        let sur_ui = m.x > screen_width() - pw || m.y > sh - drawer_h;
        self.cam.input_orbite(sur_ui);
        self.sys.update(dt);

        // --- Rendu 3D : planète principale en plein écran (compatible mode pixel) ;
        // le panneau (droite) et le tiroir (bas) sont dessinés par-dessus en 2D. ---
        clear_background(Color::new(0.02, 0.02, 0.05, 1.0));
        let aspect = screen_width() / sh;
        let (cam_info, cam3d) = self.cam.construire(Vec3::ZERO, aspect);
        self.rendu.rendre(cam3d, &cam_info, &mut self.fond, &mut self.sys, false, false);

        // --- Vignettes du catalogue (chacune dans son viewport). ---
        if self.drawer_ouvert {
            self.dessiner_vignettes(main_w, drawer_h);
        }

        set_default_camera();
        if self.edition {
            // Défilement du contenu (au-dessus du footer).
            if m.x > screen_width() - pw && m.y < sh - FOOTER_H {
                let vue = (sh - FOOTER_H - 40.0).max(1.0);
                let max = (self.edit_contenu - vue).max(0.0);
                self.edit_scroll = (self.edit_scroll - mouse_wheel().1 * 30.0).clamp(0.0, max);
            }
            let avant = self.snapshot();
            let (changed, geste, clic_nom) = self.dessiner_editeur(pw, m, down, clic);
            let modif = self.modifie();
            let (undo_ok, redo_ok) = (!self.undo.is_empty(), !self.redo.is_empty());
            let (annuler, refaire, sauver, reset, terminer) =
                self.dessiner_footer(pw, m, clic, undo_ok, redo_ok, modif);

            if geste {
                self.undo.push(avant);
                self.redo.clear();
            }
            if clic_nom {
                self.focus_nom = !self.focus_nom;
            } else if clic && m.x > screen_width() - pw {
                self.focus_nom = false; // clic ailleurs dans le panneau -> défocus
            }
            let mut maj = changed;
            if annuler {
                self.annuler();
                maj = true;
            }
            if refaire {
                self.refaire();
                maj = true;
            }
            if reset {
                self.reset_edit();
                maj = true;
            }
            if sauver {
                self.sauver_edit();
            }
            if maj {
                self.sys = batir_app(self.travail); // aperçu live
            }
            if terminer {
                self.quitter_edition();
            }
        } else if self.dessiner_panneau(pw, m, clic) {
            self.entrer_edition();
        }
        self.dessiner_barre(main_w, drawer_h, m);
        if self.drawer_ouvert {
            self.dessiner_noms(main_w, drawer_h, m);
        }
        self.dessiner_aide();
        if self.confirm_idx.is_some() {
            self.dessiner_dialog(m, clic);
        }
        false
    }

    /// Dialogue modal : confirmation d'abandon des modifications non sauvegardées.
    fn dessiner_dialog(&mut self, m: Vec2, clic: bool) {
        let i = match self.confirm_idx {
            Some(i) => i,
            None => return,
        };
        let (w, h) = (440.0, 150.0);
        let x = (screen_width() - w) * 0.5;
        let y = (screen_height() - h) * 0.5;
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.55));
        draw_rectangle(x, y, w, h, Color::new(0.08, 0.1, 0.16, 1.0));
        draw_rectangle_lines(x, y, w, h, 2.0, Color::new(0.0, 0.7, 0.7, 1.0));
        self.texte("MODIFICATIONS NON SAUVEGARDEES", x + 20.0, y + 34.0, 18.0, Color::new(0.0, 0.9, 0.9, 1.0));
        self.texte("Changer de planete fera perdre vos modifs.", x + 20.0, y + 62.0, 15.0, Color::new(0.85, 0.9, 0.88, 1.0));
        let bw = 180.0;
        let cont = btn(x + 20.0, y + 96.0, bw, 34.0, "Continuer (perdre)", true, m, clic, &self.font);
        let annul = btn(x + w - bw - 20.0, y + 96.0, bw, 34.0, "Annuler", true, m, clic, &self.font);
        if cont {
            self.confirm_idx = None;
            self.charger(i);
            self.sauvegarde = self.snapshot();
            self.undo.clear();
            self.redo.clear();
            self.sys = batir_app(self.travail);
        } else if annul {
            self.confirm_idx = None;
        }
    }

    /// Position X (écran) de la vignette de rang `rang` dans le tiroir.
    fn x_vignette(&self, rang: usize) -> f32 {
        8.0 + rang as f32 * CW - self.drawer_scroll
    }

    fn clic_vignette(&self, m: Vec2, main_w: f32, drawer_h: f32) -> Option<usize> {
        let top = screen_height() - drawer_h;
        let bottom = screen_height() - BAR_H;
        for (rang, &i) in self.filtres().iter().enumerate() {
            let cx = self.x_vignette(rang);
            if cx + CW < 0.0 || cx > main_w {
                continue;
            }
            if m.x >= cx && m.x < cx + CW && m.y >= top && m.y < bottom {
                return Some(i);
            }
        }
        None
    }

    fn dessiner_vignettes(&mut self, main_w: f32, drawer_h: f32) {
        let gl_y = (drawer_h - PAD_TOP - THUMB) as i32; // bas de la vignette en coords GL (origine bas)
        let cam = cam_vignette();
        // On collecte les (index, x) visibles d'abord (filtres() emprunte &self).
        let visibles: Vec<(usize, f32)> = self
            .filtres()
            .iter()
            .enumerate()
            .map(|(rang, &i)| (i, self.x_vignette(rang)))
            .filter(|&(_, cx)| cx + CW >= 0.0 && cx <= main_w)
            .collect();
        for (i, cx) in visibles {
            let vx = cx + (CW - THUMB) * 0.5; // vignette centrée dans la cellule
            let cam3d = Camera3D {
                position: vec3(0.0, 0.0, 3.0),
                target: Vec3::ZERO,
                up: Vec3::Y,
                fovy: 45.0_f32.to_radians(),
                aspect: Some(1.0),
                viewport: Some((vx as i32, gl_y, THUMB as i32, THUMB as i32)),
                ..Default::default()
            };
            set_camera(&cam3d);
            self.vignettes[i].draw(&cam);
        }
    }

    fn dessiner_noms(&self, main_w: f32, drawer_h: f32, m: Vec2) {
        let top = screen_height() - drawer_h;
        let bottom = screen_height() - BAR_H;
        let ny = top + PAD_TOP + THUMB + 14.0;
        for (rang, &i) in self.filtres().iter().enumerate() {
            let cx = self.x_vignette(rang);
            if cx + CW < 0.0 || cx > main_w {
                continue;
            }
            let p = &self.presets[i];
            let survol = m.x >= cx && m.x < cx + CW && m.y >= top && m.y < bottom;
            let col = if i == self.idx {
                Color::new(0.4, 1.0, 0.85, 1.0)
            } else if survol {
                Color::new(0.95, 0.95, 0.7, 1.0)
            } else {
                Color::new(0.7, 0.8, 0.78, 1.0)
            };
            let marque = if self.est_edite(i) { "*" } else { "" };
            let nom = adapter_nom(&p.nom, CW - 12.0);
            self.texte(&format!("{}{}", marque, nom), cx + 2.0, ny, 13.0, col);
        }
    }

    fn dessiner_barre(&self, main_w: f32, _drawer_h: f32, m: Vec2) {
        let sh = screen_height();
        // (Le fond du tiroir est la couleur de clear : les vignettes y flottent.)
        // Barre / onglet cliquable.
        let by = sh - BAR_H;
        let barre = Rect::new(0.0, by, main_w, BAR_H);
        let survol = barre.contains(m);
        let bg = if survol { Color::new(0.1, 0.16, 0.2, 1.0) } else { Color::new(0.06, 0.09, 0.13, 1.0) };
        draw_rectangle(0.0, by, main_w, BAR_H, bg);
        draw_line(0.0, by, main_w, by, 1.0, Color::new(0.2, 0.4, 0.45, 1.0));
        let fleche = if self.drawer_ouvert { "v" } else { "^" };
        self.texte(&format!("{} CATALOGUE ({})", fleche, self.presets.len()), 12.0, by + 19.0, 16.0, Color::new(0.0, 0.9, 0.9, 1.0));
        // Champ de recherche (à droite de la barre) quand ouvert.
        if self.drawer_ouvert {
            let rx = main_w - 260.0;
            self.texte("rech:", rx, by + 19.0, 15.0, Color::new(0.5, 0.7, 0.7, 1.0));
            let q = if self.recherche.is_empty() { "(tapez)".to_string() } else { self.recherche.clone() };
            let qc = if self.recherche.is_empty() { Color::new(0.4, 0.5, 0.5, 1.0) } else { Color::new(0.9, 0.95, 0.7, 1.0) };
            self.texte(&q, rx + 62.0, by + 19.0, 15.0, qc);
        }
    }

    /// Fiche lecture seule + bouton MODIFIER. Renvoie `true` si MODIFIER est cliqué.
    fn dessiner_panneau(&self, pw: f32, m: Vec2, pressed: bool) -> bool {
        let x = screen_width() - pw;
        draw_rectangle(x, 0.0, pw, screen_height(), Color::new(0.04, 0.05, 0.09, 0.94));
        draw_line(x, 0.0, x, screen_height(), 1.0, Color::new(0.2, 0.4, 0.45, 1.0));
        let px = x + 18.0;
        let p = &self.presets[self.idx];
        let titre = Color::new(0.0, 0.9, 0.9, 1.0);
        let label = Color::new(0.5, 0.7, 0.75, 1.0);
        let val = Color::new(0.85, 0.92, 0.9, 1.0);

        self.texte("FICHE PLANETE", px, 36.0, 22.0, titre);
        self.texte(&p.nom, px, 74.0, 26.0, val);
        if self.est_edite(self.idx) {
            let tw = measure_text(&p.nom, Some(&self.font), 26, 1.0).width;
            self.texte("[perso]", px + tw + 10.0, 74.0, 15.0, Color::new(0.7, 0.85, 1.0, 1.0));
        }

        let type_s = match p.apparence.type_p {
            TypePlanete::Tellurique => "Tellurique",
            TypePlanete::Gazeuse => "Geante gazeuse",
            TypePlanete::Glacee => "Glacee",
        };
        let mut y = 112.0;
        self.texte("TYPE", px, y, 16.0, label);
        self.texte(type_s, px, y + 22.0, 19.0, val);
        y += 58.0;

        self.texte("HABITABILITE", px, y, 16.0, label);
        let hc = p.habitabilite.couleur();
        self.texte(p.habitabilite.label(), px, y + 22.0, 18.0, Color::new(hc.x, hc.y, hc.z, 1.0));
        y += 58.0;

        self.texte("RARETE", px, y, 16.0, label);
        if p.rare {
            self.texte("RARE", px, y + 22.0, 19.0, Color::new(0.72, 0.45, 1.0, 1.0));
        } else {
            self.texte("Commune", px, y + 22.0, 19.0, val);
        }
        y += 58.0;

        self.texte("PARTICULARITES", px, y, 16.0, label);
        y += 24.0;
        for f in self.features() {
            self.texte(&format!("- {}", f), px, y, 17.0, val);
            y += 22.0;
            if y > screen_height() - 56.0 {
                break;
            }
        }

        // Bouton MODIFIER (ouvre l'éditeur).
        let bw = pw - 36.0;
        let br = Rect::new(px, screen_height() - 44.0, bw, 30.0);
        let survol = br.contains(m);
        draw_rectangle(br.x, br.y, br.w, br.h, if survol { Color::new(0.1, 0.45, 0.5, 1.0) } else { Color::new(0.08, 0.2, 0.24, 1.0) });
        draw_rectangle_lines(br.x, br.y, br.w, br.h, 1.0, Color::new(0.2, 0.5, 0.55, 1.0));
        self.texte("MODIFIER  (E)", br.x + 12.0, br.y + 20.0, 17.0, Color::new(0.9, 0.97, 0.9, 1.0));
        pressed && survol
    }

    /// Panneau d'édition (scrollable, hors footer). Renvoie (a_change, geste, clic_champ_nom).
    fn dessiner_editeur(&mut self, pw: f32, m: Vec2, down: bool, pressed: bool) -> (bool, bool, bool) {
        let x = screen_width() - pw;
        draw_rectangle(x, 0.0, pw, screen_height(), Color::new(0.04, 0.05, 0.09, 0.98));
        draw_line(x, 0.0, x, screen_height(), 1.0, Color::new(0.2, 0.4, 0.45, 1.0));
        self.texte("EDITION", x + 18.0, 28.0, 22.0, Color::new(0.0, 0.9, 0.9, 1.0));

        let est_gazeux = self.travail.type_p == TypePlanete::Gazeuse;
        let mut hi = hab_index(self.t_hab);
        let mut rare = self.t_rare;
        let nom_aff = self.t_nom.clone();
        let focus = self.focus_nom;
        let mut clic_nom = false;
        let changed;
        let geste;
        let fin_y;
        {
            let px = x + 16.0;
            let pwid = pw - 30.0;
            let y0 = 40.0 - self.edit_scroll;
            let clip_bot = screen_height() - FOOTER_H;
            let mut p = Panneau::new(&self.font, m, down, pressed, px, pwid, y0, 36.0, clip_bot, &mut self.widget_actif);

            p.titre("IDENTITE");
            clic_nom = p.champ("Nom", &nom_aff, focus);
            p.choix("Habitabilite", &mut hi, &HAB_LABELS);
            {
                let mut r = rare;
                p.interrupteur("Rare", &mut r);
                rare = r;
            }

            if est_gazeux {
                p.espace(8.0);
                p.info("Note", "editeur gazeux a venir", Color::new(0.85, 0.72, 0.4, 1.0));
            } else {
                p.titre("COULEURS");
                p.couleur("Sol / terres", &mut self.travail.couleur);
                p.couleur("Roche (cote/plaine)", &mut self.travail.couleur2);
                p.couleur("Montagne (altitude)", &mut self.travail.couleur_mont);
                p.couleur("Ocean", &mut self.travail.couleur3);
                p.couleur("Atmosphere", &mut self.travail.atmo);

                p.titre("BASE");
                p.slider("Eau", &mut self.travail.eau, 0.0, 1.0);
                p.slider("Motif (echelle masses)", &mut self.travail.eau_motif, 0.0, 3.0);
                p.slider("Gradient latitude", &mut self.travail.grad_lat, 0.0, 1.0);
                p.slider("Calotte (latitude)", &mut self.travail.calotte, 0.0, 1.0);
                p.slider("Relief", &mut self.travail.relief, 0.0, 1.0);

                p.titre("FEATURES");
                // Végétation.
                {
                    let mut on = self.travail.veg_couv > 0.0;
                    if p.interrupteur("Vegetation", &mut on) {
                        self.travail.veg_couv = if on { 0.6 } else { 0.0 };
                    }
                    if self.travail.veg_couv > 0.0 {
                        p.slider("  couverture", &mut self.travail.veg_couv, 0.0, 1.0);
                        p.couleur("  teinte", &mut self.travail.veg_couleur);
                    }
                }
                {
                    let mut on = self.travail.rivieres > 0.0;
                    if p.interrupteur("Rivieres", &mut on) {
                        self.travail.rivieres = if on { 0.5 } else { 0.0 };
                    }
                    if self.travail.rivieres > 0.0 {
                        p.slider("  densite", &mut self.travail.rivieres, 0.0, 1.0);
                        p.slider("  fracture", &mut self.travail.riv_fracture, 0.0, 1.0);
                    }
                }
                // Nuages.
                {
                    let mut on = self.travail.nuages > 0.0;
                    if p.interrupteur("Nuages", &mut on) {
                        self.travail.nuages = if on { 0.5 } else { 0.0 };
                    }
                    if self.travail.nuages > 0.0 {
                        p.slider("  densite", &mut self.travail.nuages, 0.0, 1.0);
                        p.couleur("  teinte", &mut self.travail.nuages_couleur);
                        let mut nt = (self.travail.nuages_type as usize).min(2);
                        p.choix("  type", &mut nt, &["Classique", "Tempete", "Cyclone"]);
                        self.travail.nuages_type = nt as f32;
                    }
                }
                feat_scalaire(&mut p, "Dunes", &mut self.travail.dunes, 0.6);
                feat_scalaire(&mut p, "Mesas / canyons", &mut self.travail.mesa, 0.7);
                feat_scalaire(&mut p, "Pics de glace", &mut self.travail.pics, 0.7);
                feat_scalaire(&mut p, "Recifs", &mut self.travail.recifs, 0.7);
                feat_scalaire(&mut p, "Orgues basaltiques", &mut self.travail.basalt, 0.8);
                feat_scalaire(&mut p, "Crateres", &mut self.travail.crateres, 0.5);
                feat_scalaire(&mut p, "Cryovolcanisme", &mut self.travail.cryo, 0.6);
                feat_scalaire(&mut p, "Bioluminescence", &mut self.travail.biolum, 0.8);
                // Voile atmosphérique.
                {
                    let mut on = self.travail.voile > 0.0;
                    if p.interrupteur("Voile atmospherique", &mut on) {
                        self.travail.voile = if on { 0.8 } else { 0.0 };
                    }
                    if self.travail.voile > 0.0 {
                        p.slider("  densite", &mut self.travail.voile, 0.0, 1.0);
                        p.couleur("  teinte", &mut self.travail.voile_couleur);
                    }
                }
                // Eyeball (verrouillage de marée).
                {
                    let mut on = self.travail.eyeball > 0.0;
                    if p.interrupteur("Verrouillage de maree", &mut on) {
                        self.travail.eyeball = if on { 1.0 } else { 0.0 };
                    }
                    if self.travail.eyeball > 0.0 {
                        p.slider("  limite glace", &mut self.travail.eye_glace, -1.0, 1.0);
                        let mut lave = self.travail.eye_lave > 0.5;
                        if p.interrupteur("  zone lave/obsidienne", &mut lave) {
                            self.travail.eye_lave = if lave { 1.0 } else { 0.0 };
                        }
                        let mut ring = self.travail.eye_ring > 0.5;
                        if p.interrupteur("  anneau de foret", &mut ring) {
                            self.travail.eye_ring = if ring { 1.0 } else { 0.0 };
                        }
                    }
                }
            }

            p.espace(8.0);
            changed = p.change;
            geste = p.geste;
            fin_y = p.y();
        }
        self.t_hab = index_hab(hi);
        self.t_rare = rare;
        self.edit_contenu = fin_y - 40.0 + self.edit_scroll;
        (changed, geste, clic_nom)
    }

    /// Footer fixe d'édition (boutons d'action). Renvoie (annuler, refaire, sauver, reset, terminer).
    fn dessiner_footer(&self, pw: f32, m: Vec2, pressed: bool, undo_ok: bool, redo_ok: bool, modif: bool) -> (bool, bool, bool, bool, bool) {
        let x = screen_width() - pw;
        let fy = screen_height() - FOOTER_H;
        draw_rectangle(x, fy, pw, FOOTER_H, Color::new(0.05, 0.06, 0.1, 1.0));
        draw_line(x, fy, x + pw, fy, 1.0, Color::new(0.2, 0.4, 0.45, 1.0));
        let px = x + 16.0;
        let pwid = pw - 32.0;
        let demi = (pwid - 8.0) * 0.5;

        // Ligne 1 : Annuler | Refaire.
        let annuler = btn(px, fy + 8.0, demi, 26.0, "< Annuler", undo_ok, m, pressed, &self.font);
        let refaire = btn(px + demi + 8.0, fy + 8.0, demi, 26.0, "Refaire >", redo_ok, m, pressed, &self.font);
        // Ligne 2 : Reinitialiser | Terminer.
        let reset = btn(px, fy + 40.0, demi, 26.0, "Reinitialiser", true, m, pressed, &self.font);
        let terminer = btn(px + demi + 8.0, fy + 40.0, demi, 26.0, "Terminer", true, m, pressed, &self.font);
        // Ligne 3 : Sauvegarder (pleine largeur, surligné si modifié).
        let sl = if modif { Color::new(0.15, 0.55, 0.3, 1.0) } else { Color::new(0.1, 0.3, 0.2, 1.0) };
        let r = Rect::new(px, fy + 76.0, pwid, 30.0);
        let survol = r.contains(m);
        draw_rectangle(r.x, r.y, r.w, r.h, if survol { Color::new(0.2, 0.7, 0.4, 1.0) } else { sl });
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.3, 0.7, 0.45, 1.0));
        let lab = if modif { "SAUVEGARDER *" } else { "SAUVEGARDER" };
        self.texte(lab, r.x + 12.0, r.y + 21.0, 17.0, Color::new(0.95, 1.0, 0.95, 1.0));
        let sauver = pressed && survol;
        let etat = if modif { "modifie - non sauvegarde" } else { "a jour" };
        self.texte(etat, px, fy + 124.0, 13.0, Color::new(0.6, 0.7, 0.7, 1.0));
        (annuler, refaire, sauver, reset, terminer)
    }

    fn features(&self) -> Vec<&'static str> {
        let a = &self.presets[self.idx].apparence;
        let mut v: Vec<&'static str> = Vec::new();
        if a.eau > 0.4 {
            v.push("oceans");
        } else if a.eau > 0.05 {
            v.push("mers / lacs");
        }
        if a.veg_couv > 0.3 {
            v.push("vegetation");
        }
        if a.biolum > 0.0 {
            v.push("bioluminescence");
        }
        if a.nuages > 0.3 {
            v.push("couche nuageuse");
        }
        if a.rivieres > 0.0 {
            v.push("rivieres");
        }
        if a.recifs > 0.0 {
            v.push("recifs / atolls");
        }
        if a.dunes > 0.0 {
            v.push("dunes");
        }
        if a.mesa > 0.0 {
            v.push("mesas / canyons");
        }
        if a.pics > 0.0 {
            v.push("pics de glace");
        }
        if a.calotte < 0.9 {
            v.push("calottes glaciaires");
        }
        if a.lave > 0.0 {
            v.push("lave incandescente");
        }
        if a.cryo > 0.0 {
            v.push("cryovolcanisme");
        }
        if a.eyeball > 0.0 {
            v.push("verrouillage de maree");
        }
        if a.voile > 0.0 {
            v.push("voile atmospherique");
        }
        if a.anneau {
            v.push("anneaux");
        }
        v
    }

    fn dessiner_aide(&self) {
        self.texte(
            "barre bas: catalogue   <- ->: feuilleter   E: editer   souris: tourner   P: pixel   R: shaders   Echap: menu",
            12.0,
            18.0,
            14.0,
            Color::new(0.55, 0.75, 0.78, 1.0),
        );
    }
}

/// Bouton fixe (footer) dessiné avec la police. Renvoie `true` si cliqué (et actif).
#[allow(clippy::too_many_arguments)]
fn btn(x: f32, y: f32, w: f32, h: f32, label: &str, actif: bool, m: Vec2, pressed: bool, font: &Font) -> bool {
    let r = Rect::new(x, y, w, h);
    let survol = r.contains(m);
    let bg = if !actif {
        Color::new(0.06, 0.07, 0.1, 1.0)
    } else if survol {
        Color::new(0.1, 0.4, 0.45, 1.0)
    } else {
        Color::new(0.08, 0.2, 0.24, 1.0)
    };
    draw_rectangle(x, y, w, h, bg);
    draw_rectangle_lines(x, y, w, h, 1.0, Color::new(0.2, 0.45, 0.5, 1.0));
    let col = if actif { Color::new(0.9, 0.95, 0.9, 1.0) } else { Color::new(0.5, 0.55, 0.55, 1.0) };
    draw_text_ex(label, x + 8.0, y + h * 0.5 + 5.0, TextParams { font: Some(font), font_size: 15, color: col, ..Default::default() });
    actif && pressed && survol
}

/// Feature à scalaire simple : interrupteur (on = défaut) + slider d'intensité si actif.
fn feat_scalaire(p: &mut Panneau, label: &str, v: &mut f32, defaut: f32) {
    let mut on = *v > 0.0;
    if p.interrupteur(label, &mut on) {
        *v = if on { defaut } else { 0.0 };
    }
    if *v > 0.0 {
        p.slider("  intensite", v, 0.0, 1.0);
    }
}

/// Tronque un nom pour qu'il tienne dans `largeur` px (approx. 7 px/caractère à 13 px).
fn adapter_nom(nom: &str, largeur: f32) -> String {
    let max = (largeur / 7.0) as usize;
    if nom.chars().count() <= max {
        nom.to_string()
    } else {
        let coupe: String = nom.chars().take(max.saturating_sub(1)).collect();
        format!("{}.", coupe)
    }
}
