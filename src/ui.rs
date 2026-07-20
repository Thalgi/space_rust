use macroquad::prelude::*;

/// Panneau Minitel (fond bleu nuit, cadre cyan, barre de titre).
pub fn minitel_panel(r: Rect, titre: &str) {
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.02, 0.03, 0.12, 0.97));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.0, 0.85, 0.85, 1.0));
    draw_rectangle(r.x, r.y, r.w, 26.0, Color::new(0.0, 0.6, 0.6, 1.0));
    crate::police::texte(titre, r.x + 10.0, r.y + 18.0, 20.0, BLACK);
}

/// Ligne/bouton cliquable façon télétexte (surbrillance inversée au survol).
pub fn minitel_ligne(r: Rect, label: &str, souris: Vec2) {
    let survol = r.contains(souris);
    let (bg, fg) = if survol {
        (Color::new(0.0, 0.85, 0.85, 1.0), BLACK)
    } else {
        (Color::new(0.04, 0.05, 0.18, 1.0), Color::new(0.55, 1.0, 0.75, 1.0))
    };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.0, 0.7, 0.7, 1.0));
    crate::police::texte(label, r.x + 10.0, r.y + r.h * 0.5 + 6.0, 20.0, fg);
}
