use crate::starmap::{voisinage, Destination, Etoile, Projection};
use crate::ui::minitel_panel;
use macroquad::prelude::*;

/// Ce que la Starmap demande à `main` en fin de frame.
pub enum SortieStarmap {
    /// Échap : revenir à l'accueil.
    Accueil,
    /// Clic sur une étoile : ouvrir la Skymap de son système.
    Systeme(Destination),
}

/// Vue galactique — voisinage stellaire (la vue la plus haute du jeu, au-dessus de la Skymap).
///
/// Sol en diagonale (nuage de points) + billes d'étoiles à hauteur variable (z réel exagéré),
/// reliées au sol par une tige pointillée. Survol -> surbrillance + panneau info ; clic ->
/// zoom vers la Skymap de l'étoile (cf. `CONCEPTION_STARMAP.md`).
pub struct Starmap {
    etoiles: Vec<Etoile>,
    noms: bool,
}

impl Starmap {
    pub fn new() -> Self {
        Self {
            etoiles: voisinage(),
            noms: true,
        }
    }

    /// Une frame. Renvoie une `SortieStarmap` si l'écran doit changer (Échap ou clic étoile).
    pub fn frame(&mut self) -> Option<SortieStarmap> {
        clear_background(Color::new(0.01, 0.02, 0.06, 1.0));

        if is_key_pressed(KeyCode::Escape) {
            return Some(SortieStarmap::Accueil);
        }
        if is_key_pressed(KeyCode::N) {
            self.noms = !self.noms;
        }

        // Échelle/exagération choisies pour que tout le voisinage tienne à l'écran, y compris
        // Tau Ceti très bas (b ≈ −73°) et Wolf 359 / Lalande haut au-dessus du plan.
        let proj = Projection {
            origine: vec2(screen_width() * 0.5, screen_height() * 0.40),
            echelle: screen_height() * 0.025, // s'adapte au plein écran (≈36 en 1440p, ≈17.5 en 700p)
            kz: 1.3,
        };

        // Survol : étoile dont le glyphe est le plus proche du curseur (dans son rayon + marge).
        let m = vec2(mouse_position().0, mouse_position().1);
        let survol = self.viser(&proj, m);

        crate::starmap::dessiner(&proj, &self.etoiles, self.noms, survol);

        // Panneau info + clic de sélection.
        if let Some(i) = survol {
            self.panneau_info(&self.etoiles[i], m);
            if is_mouse_button_pressed(MouseButton::Left) {
                return Some(SortieStarmap::Systeme(self.etoiles[i].dest));
            }
        }

        crate::police::texte(
            "* STARMAP - VOISINAGE STELLAIRE *",
            12.0,
            26.0,
            22.0,
            Color::new(0.0, 0.9, 0.9, 1.0),
        );
        crate::police::texte(
            "Survol: infos   Clic: entrer dans le systeme   N: noms   Echap: retour",
            12.0,
            screen_height() - 20.0,
            16.0,
            Color::new(0.5, 0.8, 0.7, 1.0),
        );
        None
    }

    /// Index de l'étoile pointée (glyphe le plus proche du curseur, dans son rayon + marge).
    fn viser(&self, proj: &Projection, m: Vec2) -> Option<usize> {
        let mut meilleur: Option<(usize, f32)> = None;
        for (i, e) in self.etoiles.iter().enumerate() {
            let p = proj.project(e.position()) + e.decalage;
            let d = p.distance(m);
            if d <= e.rayon + 8.0 && meilleur.map_or(true, |(_, best)| d < best) {
                meilleur = Some((i, d));
            }
        }
        meilleur.map(|(i, _)| i)
    }

    /// Petit panneau Minitel près du curseur : nom, type, distance, hauteur z.
    fn panneau_info(&self, e: &Etoile, m: Vec2) {
        let (w, h) = (238.0_f32, 104.0_f32);
        let x = (m.x + 16.0).min(screen_width() - w - 8.0);
        let y = (m.y + 16.0).min(screen_height() - h - 8.0);
        minitel_panel(Rect::new(x, y, w, h), e.nom);

        let z = e.position().z;
        let cyan = Color::new(0.55, 1.0, 0.75, 1.0);
        let bin = if e.double { "  (binaire)" } else { "" };
        crate::police::texte(&format!("Type spectral : {}{}", e.classe(), bin), x + 10.0, y + 46.0, 17.0, cyan);
        crate::police::texte(&format!("Distance : {:.2} al", e.d), x + 10.0, y + 68.0, 17.0, cyan);
        crate::police::texte(
            &format!("Hauteur z : {:+.2} al", z),
            x + 10.0,
            y + 90.0,
            17.0,
            cyan,
        );
    }
}
