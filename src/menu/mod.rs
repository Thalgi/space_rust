mod dessin;

use macroquad::prelude::*;

/// Action que `main` doit appliquer (charger un système, etc.).
pub enum ActionMenu {
    Solaire,
    TauCeti,
    Avatar,
    AlphaCentauri,
    Proxima,
    Binaire,
    Trinaire,
    Quadruple,
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
    pub orbites: bool,         // orbites des planètes
    pub orbites_etoiles: bool, // orbites des étoiles (systèmes multiples)
    pub zone: bool,
    /// Mode physique des planètes : `true` = sur rails (analytique), `false` = N-corps.
    pub phys_rails: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            ouvert: false,
            saisie: false,
            nom: String::new(),
            orbites: true,
            orbites_etoiles: true,
            zone: true,
            phys_rails: true,
        }
    }

    fn btn_orb() -> Rect {
        Rect::new(10.0, 34.0, 140.0, 26.0)
    }
    fn btn_orb_et() -> Rect {
        Rect::new(156.0, 34.0, 140.0, 26.0)
    }
    fn btn_zone() -> Rect {
        Rect::new(302.0, 34.0, 100.0, 26.0)
    }
    fn btn_phys() -> Rect {
        Rect::new(408.0, 34.0, 175.0, 26.0)
    }
    fn bouton() -> Rect {
        Rect::new(screen_width() - 122.0, 8.0, 112.0, 28.0)
    }
    fn retour() -> Rect {
        Rect::new(screen_width() - 122.0, 42.0, 112.0, 28.0)
    }
    fn menu_rect(&self, n_presets: usize) -> Rect {
        let n = 11 + n_presets;
        let h = 40.0 + n as f32 * 30.0 + if self.saisie { 40.0 } else { 0.0 };
        Rect::new(screen_width() * 0.5 - 150.0, 60.0, 300.0, h)
    }
    fn entry_rect(&self, i: usize, n_presets: usize) -> Rect {
        let mr = self.menu_rect(n_presets);
        Rect::new(mr.x + 12.0, mr.y + 34.0 + i as f32 * 30.0, mr.w - 24.0, 26.0)
    }

    /// Gestion des clics. Renvoie (souris_sur_ui, action éventuelle).
    pub fn input(&mut self, m: Vec2, clic: bool, n_presets: usize, focus: bool) -> (bool, Option<ActionMenu>) {
        let (bo, boe, bz, bp, bn, re) = (
            Self::btn_orb(),
            Self::btn_orb_et(),
            Self::btn_zone(),
            Self::btn_phys(),
            Self::bouton(),
            Self::retour(),
        );
        let mr = self.menu_rect(n_presets);
        let mut action = None;
        if clic {
            if bo.contains(m) {
                self.orbites = !self.orbites;
            } else if boe.contains(m) {
                self.orbites_etoiles = !self.orbites_etoiles;
            } else if bz.contains(m) {
                self.zone = !self.zone;
            } else if bp.contains(m) {
                self.phys_rails = !self.phys_rails;
            } else if bn.contains(m) {
                self.ouvert = !self.ouvert;
            } else if focus && re.contains(m) {
                action = Some(ActionMenu::Retour);
            } else if self.ouvert && !self.saisie {
                for i in 0..(11 + n_presets) {
                    if self.entry_rect(i, n_presets).contains(m) {
                        action = self.activer(i, n_presets);
                        break;
                    }
                }
            }
        }
        let sur_ui = bo.contains(m)
            || boe.contains(m)
            || bz.contains(m)
            || bp.contains(m)
            || bn.contains(m)
            || (focus && re.contains(m))
            || (self.ouvert && mr.contains(m));
        (sur_ui, action)
    }

    fn activer(&mut self, i: usize, n: usize) -> Option<ActionMenu> {
        // 0 Solaire | 1 TauCeti | 2 Avatar | 3 AlphaCentauri | 4 Proxima | 5 Binaire |
        // 6 Trinaire | 7 Quadruple | 8..8+n presets | 8+n Aleatoire | 9+n Sauver | 10+n Quitter
        let a = if i == 0 {
            Some(ActionMenu::Solaire)
        } else if i == 1 {
            Some(ActionMenu::TauCeti)
        } else if i == 2 {
            Some(ActionMenu::Avatar)
        } else if i == 3 {
            Some(ActionMenu::AlphaCentauri)
        } else if i == 4 {
            Some(ActionMenu::Proxima)
        } else if i == 5 {
            Some(ActionMenu::Binaire)
        } else if i == 6 {
            Some(ActionMenu::Trinaire)
        } else if i == 7 {
            Some(ActionMenu::Quadruple)
        } else if i < 8 + n {
            Some(ActionMenu::Charger(i - 8))
        } else if i == 8 + n {
            Some(ActionMenu::Aleatoire)
        } else if i == 9 + n {
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
