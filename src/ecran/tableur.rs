//! **Le tableur d'habitabilité** (chantier planètes, 2026-10-27) : une ligne
//! par preset rocheux du catalogue — sa vignette, son nom, une case par niveau
//! de l'échelle (`ClassePlanete::ECHELLE`) et une case « retirée ».
//!
//! Un clic écrit aussitôt `assets/planetes.json` ; une édition à la main du
//! fichier revient ici dans la seconde (`classe::table`, relue à chaud). Une
//! ligne que le fichier ne règle pas montre le niveau de la règle, en creux.
//! Les gazeuses n'y sont pas : leur type reste automatique.

use super::galerie::vignette;
use crate::genese::catalogue_telluriques;
use crate::planete::classe::{self, ClassePlanete};
use crate::planete::Planete;
use macroquad::prelude::*;

const HAUT: f32 = 130.0;
const LIGNE: f32 = 60.0;
const X_NOM: f32 = 84.0;
const X_CASES: f32 = 420.0;
const COLONNE: f32 = 130.0;

pub struct Tableur {
    lignes: Vec<(String, Planete)>,
    scroll: f32,
    erreur: Option<String>,
}

impl Tableur {
    pub fn new() -> Self {
        let lignes = catalogue_telluriques()
            .into_iter()
            .map(|(nom, app)| (nom, Planete::new(Vec3::ZERO, Vec3::ZERO, app.taille, 1.0, app, Vec::new())))
            .collect();
        Self { lignes, scroll: 0.0, erreur: None }
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        let max_scroll = (self.lignes.len() as f32 * LIGNE - (screen_height() - HAUT)).max(0.0);
        self.scroll = (self.scroll - super::defilement::crans() * super::defilement::PAS).clamp(0.0, max_scroll);
        let mut table = classe::table();
        let m = vec2(mouse_position().0, mouse_position().1);

        clear_background(Color::new(0.02, 0.02, 0.05, 1.0));

        // --- Les vignettes, en 3D. ---
        let visibles: Vec<usize> = (0..self.lignes.len())
            .filter(|&i| {
                let y = HAUT + i as f32 * LIGNE - self.scroll;
                y + LIGNE > HAUT && y < screen_height()
            })
            .collect();
        for &i in &visibles {
            let y = HAUT + i as f32 * LIGNE - self.scroll;
            // Le survol avant la vignette : dessiné après, il la couvrirait.
            if (y..y + LIGNE).contains(&m.y) && m.y >= HAUT {
                draw_rectangle(0.0, y, screen_width(), LIGNE, Color::new(0.1, 0.14, 0.18, 1.0));
            }
            vignette(&mut self.lignes[i].1, Rect::new(12.0, y + 4.0, 56.0, 52.0), vec3(2.5, 1.8, 7.0), None);
        }
        crate::planete::set_viewport_h(0.0);
        set_default_camera();

        // --- Les lignes : nom, cases, et le clic. ---
        let clic = is_mouse_button_pressed(MouseButton::Left) && m.y >= HAUT;
        let gris = Color::new(0.5, 0.6, 0.6, 1.0);
        let mut change = false;
        for &i in &visibles {
            let y = HAUT + i as f32 * LIGNE - self.scroll;
            let (id, planete) = &self.lignes[i];
            let r = table.get(id).cloned().unwrap_or_default();
            let nom_col = if r.retiree { gris } else { Color::new(0.7, 0.9, 0.8, 1.0) };
            crate::police::texte(r.nom.as_deref().unwrap_or(id), X_NOM, y + 30.0, 18.0, nom_col);
            if r.nom.is_some() {
                crate::police::texte(id, X_NOM, y + 48.0, 14.0, gris);
            }
            // Le niveau de la règle, montré en creux quand le fichier se tait.
            let regle = classe::selon_apparence(&planete.apparence());
            // Une colonne par niveau, puis la case « retirée » (`niveau` = None).
            for k in 0..=ClassePlanete::ECHELLE.len() {
                let niveau = ClassePlanete::ECHELLE.get(k).copied();
                let case = Rect::new(X_CASES + k as f32 * COLONNE + (COLONNE - 20.0) * 0.5, y + 20.0, 20.0, 20.0);
                let (coche, creux) = match niveau {
                    None => (r.retiree, false),
                    Some(c) => (r.habitabilite == Some(c), r.habitabilite.is_none() && regle == c),
                };
                let teinte = niveau.map_or(gris, ClassePlanete::couleur);
                draw_rectangle_lines(case.x, case.y, case.w, case.h, 2.0, teinte);
                if coche {
                    draw_rectangle(case.x + 4.0, case.y + 4.0, case.w - 8.0, case.h - 8.0, teinte);
                } else if creux {
                    draw_rectangle_lines(case.x + 6.0, case.y + 6.0, case.w - 12.0, case.h - 12.0, 2.0, teinte);
                }
                if clic && case.contains(m) {
                    let e = table.entry(id.clone()).or_default();
                    match niveau {
                        None => e.retiree = !e.retiree,
                        Some(c) => e.habitabilite = Some(c),
                    }
                    change = true;
                }
            }
        }
        if change {
            self.erreur = classe::ecrire(&table).err().map(|e| format!("ECRITURE IMPOSSIBLE : {e}"));
        }

        // --- L'en-tête, par-dessus les lignes qui défilent. ---
        draw_rectangle(0.0, 0.0, screen_width(), HAUT, Color::new(0.02, 0.02, 0.05, 1.0));
        crate::police::texte("TABLEUR - HABITABILITE DES TELLURIQUES", 12.0, 28.0, 22.0, Color::new(0.4, 0.95, 0.85, 1.0));
        let aide = self.erreur.clone().unwrap_or_else(|| {
            format!("clic: regler   molette: defiler   Echap: menu      -> {}   (case en creux = regle par defaut)", classe::FICHIER)
        });
        crate::police::texte(&aide, 12.0, 52.0, 16.0, if self.erreur.is_some() { RED } else { gris });
        crate::police::texte("NOM", X_NOM, HAUT - 12.0, 16.0, gris);
        let cote = crate::sprites::taille_ecran(32.0);
        for (k, c) in ClassePlanete::ECHELLE.iter().enumerate() {
            let x = X_CASES + k as f32 * COLONNE;
            let tw = crate::police::mesure(c.libelle(), 14);
            crate::sprites::dessiner_classe(*c, x + (COLONNE - cote) * 0.5, HAUT - 30.0 - cote, cote);
            crate::police::texte(c.libelle(), x + (COLONNE - tw) * 0.5, HAUT - 12.0, 14.0, c.couleur());
        }
        let x = X_CASES + ClassePlanete::ECHELLE.len() as f32 * COLONNE;
        let tw = crate::police::mesure("RETIREE", 14);
        crate::police::texte("RETIREE", x + (COLONNE - tw) * 0.5, HAUT - 12.0, 14.0, gris);
        draw_line(0.0, HAUT - 2.0, screen_width(), HAUT - 2.0, 1.0, gris);
        false
    }
}
