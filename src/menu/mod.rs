mod dessin;

use macroquad::prelude::*;

/// Action que `main` doit appliquer (charger un système, etc.).
pub enum ActionMenu {
    Solaire,
    TauCeti,
    Charger(usize),
    Aleatoire,
    Quitter,
    Retour,
}

/// Menu Minitel + boutons d'affichage. Gère son propre état (ouvert, saisie de nom,
/// toggles), et renvoie une action quand `main` doit charger/quitter.
pub struct Menu {
    pub ouvert: bool,
    pub saisie: bool,
    pub nom: String,
    pub orbites: bool,
    pub zone: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            ouvert: false,
            saisie: false,
            nom: String::new(),
            orbites: true,
            zone: true,
        }
    }

    fn btn_orb() -> Rect {
        Rect::new(10.0, 34.0, 150.0, 26.0)
    }
    fn btn_zone() -> Rect {
        Rect::new(168.0, 34.0, 150.0, 26.0)
    }
    fn bouton() -> Rect {
        Rect::new(screen_width() - 122.0, 8.0, 112.0, 28.0)
    }
    fn retour() -> Rect {
        Rect::new(screen_width() - 122.0, 42.0, 112.0, 28.0)
    }
    fn menu_rect(&self, n_presets: usize) -> Rect {
        let n = 5 + n_presets;
        let h = 40.0 + n as f32 * 30.0 + if self.saisie { 40.0 } else { 0.0 };
        Rect::new(screen_width() * 0.5 - 150.0, 60.0, 300.0, h)
    }
    fn entry_rect(&self, i: usize, n_presets: usize) -> Rect {
        let mr = self.menu_rect(n_presets);
        Rect::new(mr.x + 12.0, mr.y + 34.0 + i as f32 * 30.0, mr.w - 24.0, 26.0)
    }

    /// Gestion des clics. Renvoie (souris_sur_ui, action éventuelle).
    pub fn input(&mut self, m: Vec2, clic: bool, n_presets: usize, focus: bool) -> (bool, Option<ActionMenu>) {
        let (bo, bz, bn, re) = (Self::btn_orb(), Self::btn_zone(), Self::bouton(), Self::retour());
        let mr = self.menu_rect(n_presets);
        let mut action = None;
        if clic {
            if bo.contains(m) {
                self.orbites = !self.orbites;
            } else if bz.contains(m) {
                self.zone = !self.zone;
            } else if bn.contains(m) {
                self.ouvert = !self.ouvert;
            } else if focus && re.contains(m) {
                action = Some(ActionMenu::Retour);
            } else if self.ouvert && !self.saisie {
                for i in 0..(5 + n_presets) {
                    if self.entry_rect(i, n_presets).contains(m) {
                        action = self.activer(i, n_presets);
                        break;
                    }
                }
            }
        }
        let sur_ui = bo.contains(m)
            || bz.contains(m)
            || bn.contains(m)
            || (focus && re.contains(m))
            || (self.ouvert && mr.contains(m));
        (sur_ui, action)
    }

    fn activer(&mut self, i: usize, n: usize) -> Option<ActionMenu> {
        // 0 Solaire | 1 TauCeti | 2..2+n presets | 2+n Aleatoire | 3+n Sauver | 4+n Quitter
        let a = if i == 0 {
            Some(ActionMenu::Solaire)
        } else if i == 1 {
            Some(ActionMenu::TauCeti)
        } else if i < 2 + n {
            Some(ActionMenu::Charger(i - 2))
        } else if i == 2 + n {
            Some(ActionMenu::Aleatoire)
        } else if i == 3 + n {
            self.saisie = true;
            self.nom.clear();
            None
        } else {
            Some(ActionMenu::Quitter)
        };
        if a.is_some() {
            self.ouvert = false;
        }
        a
    }
}
