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

    // ===== Bascules d'outillage : **la bande du bas** =====
    //
    // Elles etaient a y = 34, c'est-a-dire au milieu de la barre de ressources
    // depuis que la vue systeme porte l'interface de jeu. Le partage est
    // desormais net : le haut au jeu, le bas a l'atelier.
    //
    // `outils` est la bande basse, fournie par l'ecran (`bandeau::strip_outils`)
    // plutot que recalculee ici : deux geometries a tenir d'accord finiraient
    // par se recouvrir, ce qui est exactement le defaut qu'on corrige.

    /// Largeur des bascules, dans l'ordre ou elles se suivent.
    const LARGEURS: [f32; 4] = [140.0, 140.0, 100.0, 175.0];
    /// Hauteur d'une bascule.
    const HAUT_BTN: f32 = 26.0;

    /// Rectangle de la bascule `i` dans la bande d'outils.
    ///
    /// Les quatre se **resserrent** si la bande est trop courte : a 640 px de
    /// large, la colonne d'astres prise, il reste 560 px pour 587 px de
    /// boutons. Sans ce facteur, la derniere sortait de l'ecran.
    fn btn(outils: Rect, i: usize) -> Rect {
        const JEU: f32 = 6.0;
        let bande = Self::bande_bascules(outils);
        let total: f32 = Self::LARGEURS.iter().sum::<f32>() + JEU * (Self::LARGEURS.len() - 1) as f32;
        let k = if total > bande.w && bande.w > 0.0 { bande.w / total } else { 1.0 };
        let x = bande.x + Self::LARGEURS.iter().take(i).map(|l| (l + JEU) * k).sum::<f32>();
        Rect::new(x, bande.y + 4.0, Self::LARGEURS[i] * k, Self::HAUT_BTN)
    }

    /// Les quatre bascules, pour les tests de mise en page.
    #[cfg(test)]
    fn bascules(outils: Rect) -> [Rect; 4] {
        [Self::btn(outils, 0), Self::btn(outils, 1), Self::btn(outils, 2), Self::btn(outils, 3)]
    }

    /// Largeur du couple MENU / RETOUR, reserve a **droite** de la bande.
    const LARGEUR_MENU: f32 = 112.0;

    /// Bouton MENU : bout droit de la bande d'outils.
    ///
    /// Il etait en haut a droite, ou il entrait en collision avec la barre de
    /// ressources des que l'ecran retrecissait (a 640 px la barre va jusqu'a
    /// x = 632, le bouton commencait a 518). Tout l'outillage est desormais
    /// dans la meme bande.
    fn bouton(outils: Rect) -> Rect {
        Rect::new(outils.x + outils.w - Self::LARGEUR_MENU, outils.y + 4.0, Self::LARGEUR_MENU, Self::HAUT_BTN)
    }
    /// RETOUR, a gauche de MENU. N'apparait que camera focalisee.
    fn retour(outils: Rect) -> Rect {
        let b = Self::bouton(outils);
        Rect::new(b.x - Self::LARGEUR_MENU - 6.0, b.y, Self::LARGEUR_MENU, Self::HAUT_BTN)
    }

    /// Place laissee aux bascules : la bande **moins** le couple de droite.
    fn bande_bascules(outils: Rect) -> Rect {
        let pris = Self::LARGEUR_MENU * 2.0 + 12.0;
        Rect::new(outils.x, outils.y, (outils.w - pris).max(40.0), outils.h)
    }
    /// Panneau modal, centre, **sous** le bandeau de ressources.
    fn menu_rect(&self, n_presets: usize, haut: f32) -> Rect {
        let n = 11 + n_presets;
        let h = 40.0 + n as f32 * 30.0 + if self.saisie { 40.0 } else { 0.0 };
        Rect::new(screen_width() * 0.5 - 150.0, haut, 300.0, h)
    }
    fn entry_rect(&self, i: usize, n_presets: usize, haut: f32) -> Rect {
        let mr = self.menu_rect(n_presets, haut);
        Rect::new(mr.x + 12.0, mr.y + 34.0 + i as f32 * 30.0, mr.w - 24.0, 26.0)
    }

    /// Gestion des clics. Renvoie (souris_sur_ui, action éventuelle).
    ///
    /// `outils` est la bande basse reservee a l'outillage, `haut` le premier `y`
    /// libre sous la barre de ressources. Les deux viennent de l'ecran : le menu
    /// ne connait pas la mise en page du jeu, il s'y range.
    pub fn input(
        &mut self,
        m: Vec2,
        clic: bool,
        n_presets: usize,
        focus: bool,
        outils: Rect,
        haut: f32,
    ) -> (bool, Option<ActionMenu>) {
        let (bo, boe, bz, bp, bn, re) = (
            Self::btn(outils, 0),
            Self::btn(outils, 1),
            Self::btn(outils, 2),
            Self::btn(outils, 3),
            Self::bouton(outils),
            Self::retour(outils),
        );
        let mr = self.menu_rect(n_presets, haut);
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
                    if self.entry_rect(i, n_presets, haut).contains(m) {
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

#[cfg(test)]
mod tests_mise_en_page {
    use super::*;

    fn strip(l: f32, h: f32) -> Rect {
        crate::ecran::bandeau::strip_outils(vec2(l, h))
    }

    // **Les quatre bascules tiennent dans la bande**, a toutes les largeurs.
    // C'etait le defaut signale : posees a une position fixe, elles tombaient
    // au milieu de la barre de ressources, et debordaient sur un ecran etroit.
    #[test]
    fn les_bascules_tiennent_dans_la_bande_doutils() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1440.0, 900.0), (1920.0, 1080.0)] {
            let o = strip(l, h);
            let b = Menu::bascules(o);
            for (i, r) in b.iter().enumerate() {
                assert!(r.x >= o.x - 1e-3, "{l}x{h} : bascule {i} a gauche de la bande");
                assert!(r.x + r.w <= o.x + o.w + 1e-3, "{l}x{h} : bascule {i} deborde ({})", r.x + r.w);
                assert!(r.y >= o.y - 1e-3 && r.y + r.h <= o.y + o.h + 1e-3, "{l}x{h} : bascule {i} hors bande");
                assert!(r.w > 20.0, "{l}x{h} : bascule {i} ecrasee a {} px", r.w);
            }
            // Et elles ne se chevauchent pas entre elles.
            for i in 0..b.len() {
                for j in i + 1..b.len() {
                    assert!(
                        b[i].x + b[i].w <= b[j].x + 1e-3 || b[j].x + b[j].w <= b[i].x + 1e-3,
                        "{l}x{h} : bascules {i} et {j} se chevauchent"
                    );
                }
            }
        }
    }

    // MENU et RETOUR sont **dans la bande** eux aussi, et ne chevauchent ni les
    // bascules ni la barre de ressources. Ils etaient en haut a droite, ou la
    // barre les rattrapait des que l'ecran retrecissait.
    #[test]
    fn menu_et_retour_tiennent_dans_la_bande_sans_rien_recouvrir() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1920.0, 1080.0)] {
            let e = vec2(l, h);
            let o = strip(l, h);
            let barre = crate::ecran::bandeau::rectangle(e);
            for (nom, r) in [("MENU", Menu::bouton(o)), ("RETOUR", Menu::retour(o))] {
                assert!(r.x >= o.x - 1e-3 && r.x + r.w <= o.x + o.w + 1e-3, "{l}x{h} : {nom} hors bande");
                let sur_barre = r.x < barre.x + barre.w && barre.x < r.x + r.w
                    && r.y < barre.y + barre.h && barre.y < r.y + r.h;
                assert!(!sur_barre, "{l}x{h} : {nom} recouvre la barre de ressources");
                for (i, b) in Menu::bascules(o).iter().enumerate() {
                    let sur_bascule = r.x < b.x + b.w && b.x < r.x + r.w
                        && r.y < b.y + b.h && b.y < r.y + r.h;
                    assert!(!sur_bascule, "{l}x{h} : {nom} recouvre la bascule {i}");
                }
            }
            // RETOUR est bien a gauche de MENU, sans les superposer.
            assert!(Menu::retour(o).x + Menu::retour(o).w <= Menu::bouton(o).x + 1e-3);
        }
    }

    // Les bascules ne remontent **jamais** dans la barre de ressources : c'est
    // exactement le defaut corrige.
    #[test]
    fn les_bascules_ne_touchent_pas_la_barre_de_ressources() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1920.0, 1080.0)] {
            let e = vec2(l, h);
            let barre = crate::ecran::bandeau::rectangle(e);
            for (i, r) in Menu::bascules(strip(l, h)).iter().enumerate() {
                let chevauche = r.x < barre.x + barre.w && barre.x < r.x + r.w
                    && r.y < barre.y + barre.h && barre.y < r.y + r.h;
                assert!(!chevauche, "{l}x{h} : la bascule {i} recouvre la barre de ressources");
            }
        }
    }
}
