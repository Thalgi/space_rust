//! Petite boîte à widgets en mode immédiat pour l'éditeur de planètes.
//! Un `Panneau` pose les contrôles verticalement (curseur `y` qui avance), gère un
//! clipping grossier (widgets hors zone visible non dessinés) et l'état de drag des
//! sliders (`actif`, persistant entre frames, fourni par l'appelant).

use macroquad::prelude::*;

const GAP: f32 = 8.0; // écart vertical entre widgets (lisibilité)
const GRIS: Color = Color::new(0.55, 0.7, 0.72, 1.0);
const BLANC: Color = Color::new(0.9, 0.95, 0.92, 1.0);
const CYAN: Color = Color::new(0.0, 0.85, 0.85, 1.0);
const FOND: Color = Color::new(0.1, 0.12, 0.16, 1.0);
const REMP: Color = Color::new(0.25, 0.6, 0.7, 1.0);
const POIGNEE: Color = Color::new(0.5, 0.9, 0.95, 1.0);
const BORD: Color = Color::new(0.2, 0.4, 0.45, 1.0);
const BTN: Color = Color::new(0.1, 0.22, 0.26, 1.0);

pub struct Panneau<'a> {
    font: &'a Font,
    m: Vec2,
    down: bool,    // bouton gauche maintenu
    pressed: bool, // clic (front montant)
    x: f32,
    w: f32,
    y: f32, // curseur vertical, avance à chaque widget
    clip_top: f32,
    clip_bot: f32,
    id: usize,                    // compteur d'id de widget (drag)
    actif: &'a mut Option<usize>, // slider en cours de drag (persistant)
    pub change: bool,             // un widget a modifié une valeur cette frame
    pub geste: bool,              // un nouveau geste a commencé (pour snapshot undo)
}

impl<'a> Panneau<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        font: &'a Font,
        m: Vec2,
        down: bool,
        pressed: bool,
        x: f32,
        w: f32,
        y0: f32,
        clip_top: f32,
        clip_bot: f32,
        actif: &'a mut Option<usize>,
    ) -> Self {
        Self { font, m, down, pressed, x, w, y: y0, clip_top, clip_bot, id: 0, actif, change: false, geste: false }
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    fn visible(&self, h: f32) -> bool {
        self.y + h > self.clip_top && self.y < self.clip_bot
    }

    fn texte(&self, s: &str, x: f32, y: f32, taille: f32, col: Color) {
        draw_text_ex(s, x, y, TextParams { font: Some(self.font), font_size: taille as u16, color: col, ..Default::default() });
    }

    pub fn espace(&mut self, h: f32) {
        self.y += h + GAP;
    }

    /// Titre de section.
    pub fn titre(&mut self, s: &str) {
        let h = 26.0;
        // Un peu d'air avant le titre et davantage après (séparation des sections).
        self.y += GAP;
        let y = self.y;
        self.y += h + GAP + 8.0;
        if self.visible(h) {
            self.texte(s, self.x, y + 18.0, 16.0, CYAN);
            draw_line(self.x, y + 22.0, self.x + self.w, y + 22.0, 1.0, BORD);
        }
    }

    /// Ligne d'info en lecture seule (label + valeur colorée).
    pub fn info(&mut self, label: &str, val: &str, col: Color) {
        let h = 22.0;
        let y = self.y;
        self.y += h + GAP;
        if self.visible(h) {
            self.texte(label, self.x, y + 15.0, 14.0, GRIS);
            self.texte(val, self.x + 92.0, y + 15.0, 15.0, col);
        }
    }

    /// Slider flottant. Renvoie `true` si la valeur a changé cette frame.
    pub fn slider(&mut self, label: &str, val: &mut f32, min: f32, max: f32) -> bool {
        let h = 30.0;
        let y = self.y;
        self.y += h + GAP;
        let id = self.id;
        self.id += 1;
        if !self.visible(h) {
            return false;
        }
        let tx = self.x;
        let tw = self.w;
        let ty = y + 18.0;
        let th = 6.0;
        self.texte(label, tx, y + 12.0, 14.0, GRIS);
        self.texte(&format!("{:.2}", *val), tx + tw - 42.0, y + 12.0, 14.0, BLANC);
        draw_rectangle(tx, ty, tw, th, FOND);
        let t = ((*val - min) / (max - min)).clamp(0.0, 1.0);
        draw_rectangle(tx, ty, tw * t, th, REMP);
        draw_circle(tx + tw * t, ty + th * 0.5, 6.0, POIGNEE);

        let zone = Rect::new(tx, ty - 8.0, tw, th + 16.0);
        if self.pressed && zone.contains(self.m) {
            *self.actif = Some(id);
            self.geste = true;
        }
        let mut chg = false;
        if *self.actif == Some(id) {
            if self.down {
                let nt = ((self.m.x - tx) / tw).clamp(0.0, 1.0);
                let nv = min + nt * (max - min);
                if (nv - *val).abs() > f32::EPSILON {
                    *val = nv;
                    chg = true;
                }
            } else {
                *self.actif = None;
            }
        }
        if chg {
            self.change = true;
        }
        chg
    }

    /// Interrupteur on/off. Renvoie `true` si basculé cette frame.
    pub fn interrupteur(&mut self, label: &str, on: &mut bool) -> bool {
        let h = 28.0;
        let y = self.y;
        self.y += h + GAP;
        if !self.visible(h) {
            return false;
        }
        let (bw, bh) = (40.0, 18.0);
        let (bx, by) = (self.x, y + 5.0);
        let bg = if *on { Color::new(0.2, 0.7, 0.4, 1.0) } else { Color::new(0.3, 0.3, 0.34, 1.0) };
        draw_rectangle(bx, by, bw, bh, bg);
        let knob_x = if *on { bx + bw - bh } else { bx };
        draw_rectangle(knob_x, by, bh, bh, BLANC);
        self.texte(label, bx + bw + 10.0, y + 18.0, 15.0, BLANC);
        if self.pressed && Rect::new(bx, by, bw, bh).contains(self.m) {
            *on = !*on;
            self.geste = true;
            self.change = true;
            return true;
        }
        false
    }

    /// Couleur (3 mini-sliders R/G/B + pastille). Renvoie `true` si changée.
    pub fn couleur(&mut self, label: &str, c: &mut Vec3) -> bool {
        let h = 60.0;
        let y = self.y;
        self.y += h + GAP;
        if !self.visible(h) {
            self.id += 3; // réserve les ids des composantes pour la stabilité
            return false;
        }
        self.texte(label, self.x, y + 12.0, 14.0, GRIS);
        let sw = 20.0;
        let psx = self.x + self.w - sw;
        draw_rectangle(psx, y - 2.0, sw, sw, Color::new(c.x, c.y, c.z, 1.0));
        draw_rectangle_lines(psx, y - 2.0, sw, sw, 1.0, BORD);
        let mut chg = false;
        chg |= self.composante(y + 18.0, "R", &mut c.x);
        chg |= self.composante(y + 32.0, "G", &mut c.y);
        chg |= self.composante(y + 46.0, "B", &mut c.z);
        if chg {
            self.change = true;
        }
        chg
    }

    /// Mini-slider d'une composante de couleur (0..1) à une ligne donnée.
    fn composante(&mut self, ly: f32, lab: &str, val: &mut f32) -> bool {
        let id = self.id;
        self.id += 1;
        let tx = self.x + 16.0;
        let tw = self.w - 44.0;
        let th = 5.0;
        self.texte(lab, self.x, ly + 5.0, 12.0, GRIS);
        draw_rectangle(tx, ly, tw, th, FOND);
        draw_rectangle(tx, ly, tw * val.clamp(0.0, 1.0), th, REMP);
        draw_circle(tx + tw * val.clamp(0.0, 1.0), ly + th * 0.5, 4.5, POIGNEE);
        let zone = Rect::new(tx, ly - 6.0, tw, th + 12.0);
        if self.pressed && zone.contains(self.m) {
            *self.actif = Some(id);
            self.geste = true;
        }
        let mut chg = false;
        if *self.actif == Some(id) {
            if self.down {
                let nv = ((self.m.x - tx) / tw).clamp(0.0, 1.0);
                if (nv - *val).abs() > f32::EPSILON {
                    *val = nv;
                    chg = true;
                }
            } else {
                *self.actif = None;
            }
        }
        chg
    }

    /// Choix discret `< valeur >` (cycle). Renvoie `true` si changé.
    pub fn choix(&mut self, label: &str, idx: &mut usize, options: &[&str]) -> bool {
        let h = 30.0;
        let y = self.y;
        self.y += h + GAP;
        if !self.visible(h) || options.is_empty() {
            return false;
        }
        self.texte(label, self.x, y + 12.0, 14.0, GRIS);
        let bw = 22.0;
        let lx = self.x;
        let rx = self.x + self.w - bw;
        draw_rectangle(lx, y + 16.0, bw, 16.0, BTN);
        draw_rectangle(rx, y + 16.0, bw, 16.0, BTN);
        self.texte("<", lx + 7.0, y + 29.0, 16.0, BLANC);
        self.texte(">", rx + 7.0, y + 29.0, 16.0, BLANC);
        self.texte(options.get(*idx).copied().unwrap_or("?"), lx + bw + 8.0, y + 29.0, 15.0, BLANC);
        let mut chg = false;
        if self.pressed && Rect::new(lx, y + 16.0, bw, 16.0).contains(self.m) {
            *idx = (*idx + options.len() - 1) % options.len();
            chg = true;
        }
        if self.pressed && Rect::new(rx, y + 16.0, bw, 16.0).contains(self.m) {
            *idx = (*idx + 1) % options.len();
            chg = true;
        }
        if chg {
            self.change = true;
            self.geste = true;
        }
        chg
    }

    /// Champ texte (lecture du contenu fournie). Renvoie `true` si la boîte est cliquée
    /// (à l'appelant de gérer le focus + la saisie clavier). Un `_` marque le focus.
    pub fn champ(&mut self, label: &str, texte: &str, focus: bool) -> bool {
        let h = 32.0;
        let y = self.y;
        self.y += h + GAP;
        if !self.visible(h) {
            return false;
        }
        self.texte(label, self.x, y + 12.0, 14.0, GRIS);
        let (bx, by, bw, bh) = (self.x, y + 16.0, self.w, 18.0);
        draw_rectangle(bx, by, bw, bh, Color::new(0.08, 0.1, 0.14, 1.0));
        draw_rectangle_lines(bx, by, bw, bh, 1.0, if focus { CYAN } else { BORD });
        let aff = if focus { format!("{}_", texte) } else { texte.to_string() };
        self.texte(&aff, bx + 5.0, by + 14.0, 14.0, BLANC);
        self.pressed && Rect::new(bx, by, bw, bh).contains(self.m)
    }

    /// Bouton cliquable. `actif=false` -> grisé/inactif. Renvoie `true` si cliqué.
    pub fn bouton(&mut self, label: &str, actif: bool) -> bool {
        let h = 28.0;
        let y = self.y;
        self.y += h + 4.0;
        if !self.visible(h) {
            return false;
        }
        let r = Rect::new(self.x, y, self.w, h);
        let survol = r.contains(self.m);
        let bg = if !actif {
            Color::new(0.06, 0.07, 0.1, 1.0)
        } else if survol {
            Color::new(0.1, 0.4, 0.45, 1.0)
        } else {
            BTN
        };
        draw_rectangle(self.x, y, self.w, h, bg);
        draw_rectangle_lines(self.x, y, self.w, h, 1.0, BORD);
        self.texte(label, self.x + 8.0, y + 19.0, 16.0, if actif { BLANC } else { GRIS });
        actif && self.pressed && survol
    }
}
