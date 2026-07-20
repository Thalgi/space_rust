use super::Menu;
use crate::genese::PresetSauve;
use crate::ui::{minitel_ligne, minitel_panel};
use macroquad::prelude::*;

impl Menu {
    /// Saisie clavier du nom (mode sauvegarde). Renvoie le nom validé (Entrée).
    pub fn clavier(&mut self) -> Option<String> {
        if !self.saisie {
            return None;
        }
        while let Some(c) = get_char_pressed() {
            if (c.is_alphanumeric() || c == ' ' || c == '-' || c == '_') && self.nom.len() < 24 {
                self.nom.push(c);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.nom.pop();
        }
        if is_key_pressed(KeyCode::Escape) {
            self.saisie = false;
            self.nom.clear();
        }
        if is_key_pressed(KeyCode::Enter) {
            let n = self.nom.trim().to_string();
            self.saisie = false;
            self.nom.clear();
            if !n.is_empty() {
                return Some(n);
            }
        }
        None
    }

    pub fn dessiner(&self, m: Vec2, presets: &[PresetSauve], focus: bool) {
        minitel_ligne(Self::btn_orb(), if self.orbites { "ORB PLAN: ON" } else { "ORB PLAN: OFF" }, m);
        minitel_ligne(Self::btn_orb_et(), if self.orbites_etoiles { "ORB ETOI: ON" } else { "ORB ETOI: OFF" }, m);
        minitel_ligne(Self::btn_zone(), if self.zone { "ZONE: ON" } else { "ZONE: OFF" }, m);
        minitel_ligne(
            Self::btn_phys(),
            if self.phys_rails { "PHYS: SUR RAILS" } else { "PHYS: N-CORPS" },
            m,
        );
        minitel_ligne(Self::bouton(), "MENU", m);
        if focus {
            minitel_ligne(Self::retour(), "RETOUR", m);
        }
        if !self.ouvert {
            return;
        }
        let mr = self.menu_rect(presets.len());
        minitel_panel(mr, "* MINITEL * MENU");
        let mut labels: Vec<String> = vec![
            "SYSTEME SOLAIRE".into(),
            "TAU CETI".into(),
            "AVATAR (PANDORA)".into(),
            "ALPHA CENTAURI A+B".into(),
            "PROXIMA CENTAURI".into(),
            "BINAIRE A+B".into(),
            "TRINAIRE (A-B)+C".into(),
            "QUADRUPLE 2+2".into(),
        ];
        for p in presets {
            labels.push(p.nom.clone());
        }
        labels.push("+ ALEATOIRE".into());
        labels.push("SAUVER CE SYSTEME".into());
        labels.push("QUITTER (SAUVE)".into());
        for (i, lab) in labels.iter().enumerate() {
            minitel_ligne(self.entry_rect(i, presets.len()), lab, m);
        }
        if self.saisie {
            let y = mr.y + 34.0 + labels.len() as f32 * 30.0 + 6.0;
            crate::police::texte(&format!("NOM: {}_", self.nom), mr.x + 12.0, y + 16.0, 20.0, Color::new(0.9, 1.0, 0.6, 1.0));
            crate::police::texte("[ENTREE] valider   [ECHAP] annuler", mr.x + 12.0, y + 34.0, 14.0, Color::new(0.5, 0.8, 0.7, 1.0));
        }
    }
}
