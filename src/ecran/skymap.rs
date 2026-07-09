use crate::camera::Camera;
use crate::fond::Fond;
use crate::genese::{
    charger_presets, construire_preset_alpha_centauri, construire_preset_avatar,
    construire_preset_binaire, construire_preset_proxima, construire_preset_quadruple,
    construire_preset_solaire, construire_preset_tau_ceti, construire_preset_trinaire,
    construire_systeme, sauver_presets, PresetSauve,
};
use crate::menu::{ActionMenu, Menu};
use crate::rendu::{Rendu, RenduStandard};
use crate::starmap::Destination;
use crate::systeme::{ModePhysique, Systeme};
use crate::{planete, soleil};
use macroquad::prelude::*;

/// D'où provient le système courant : graine procédurale ou preset écrit à la main.
/// Sert à le reconstruire à l'identique lors du hot-reload des shaders.
enum Source {
    Graine(u64),
    Solaire,
    TauCeti,
    Avatar,
    AlphaCentauri,
    Proxima,
    Binaire,
    Trinaire,
    Quadruple,
}

/// Vue système complète : étoile, planètes, lunes, ceintures, UI Minitel.
pub struct Skymap {
    seed: u64,
    sys: Systeme,
    info: String,
    fond: Fond,
    cam: Camera,
    rendu: RenduStandard,
    menu: Menu,
    presets: Vec<PresetSauve>,
    source: Source,
    vitesse: f32,
    pause: bool,
}

impl Skymap {
    pub fn new() -> Self {
        let seed: u64 = 1;
        let (sys, info) = construire_systeme(seed);
        Self {
            seed,
            sys,
            info,
            fond: Fond::new(900),
            cam: Camera::new(360.0),
            rendu: RenduStandard::new(),
            menu: Menu::new(),
            presets: charger_presets(),
            source: Source::Graine(seed),
            vitesse: 1.0,
            pause: false,
        }
    }

    /// Ouvre directement la Skymap sur le système d'une étoile de la Starmap.
    /// Presets nommés -> réutilise `appliquer` (cadrage caméra inclus) ; graine -> génère.
    pub fn depuis_destination(dest: Destination) -> Self {
        let mut s = Self::new();
        match dest {
            Destination::Solaire => s.appliquer(ActionMenu::Solaire),
            Destination::Proxima => s.appliquer(ActionMenu::Proxima),
            Destination::AlphaCentauri => s.appliquer(ActionMenu::AlphaCentauri),
            Destination::TauCeti => s.appliquer(ActionMenu::TauCeti),
            Destination::Graine(g) => {
                s.seed = g;
                let (sys, info) = construire_systeme(g);
                s.sys = sys;
                s.info = info;
                s.source = Source::Graine(g);
                s.appliquer_vue();
            }
        }
        s
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        let dt = get_frame_time().min(0.05);
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);

        // Réglages d'éruptions (fixes pour l'instant).
        let (freq, forme, puissance, alea) = (0.5_f32, 0.0_f32, 0.10_f32, 0.0_f32);

        // Raccourcis clavier (désactivés pendant la saisie de nom).
        if !self.menu.saisie {
            if is_key_pressed(KeyCode::Escape) {
                return true; // retour à l'accueil
            }
            if is_key_pressed(KeyCode::P) {
                self.rendu.toggle_pixel();
            }
            if is_key_pressed(KeyCode::G) {
                self.seed = nouvelle_graine(self.seed);
                self.source = Source::Graine(self.seed);
                let (s, i) = construire_systeme(self.seed);
                self.sys = s;
                self.info = i;
                self.appliquer_vue();
            }
            if is_key_pressed(KeyCode::R) {
                // Hot-reload : recompile les shaders et reconstruit à l'identique.
                planete::vider_cache_materials();
                soleil::vider_cache_materials();
                let (s, i) = self.reconstruire();
                self.sys = s;
                self.info = i;
                self.fond.recharger_material();
            }
            if is_key_pressed(KeyCode::Space) {
                self.pause = !self.pause;
            }
            if is_key_pressed(KeyCode::Up) {
                self.vitesse = (self.vitesse * 2.0).min(16.0);
            }
            if is_key_pressed(KeyCode::Down) {
                self.vitesse = (self.vitesse * 0.5).max(0.125);
            }
        }

        // Saisie d'un nom de preset -> sauvegarde JSON.
        if let Some(nom) = self.menu.clavier() {
            self.presets.push(PresetSauve { nom: nom.clone(), graine: self.seed });
            sauver_presets(&self.presets);
            self.info = nom;
        }

        self.sys.reglages_etoile(freq, forme, puissance, alea);

        // UI -> action éventuelle + zone cliquable (pour bloquer la caméra).
        let (sur_ui, action) = self.menu.input(m, clic, self.presets.len(), self.cam.focus_actif());
        if let Some(a) = action {
            self.appliquer(a);
        }

        self.cam.input_orbite(sur_ui);
        // Synchronise le mode physique avec le bouton (idempotent : hand-off au seul changement).
        self.sys.regler_mode(if self.menu.phys_rails {
            ModePhysique::SurRails
        } else {
            ModePhysique::NCorps
        });
        let dt_sim = if self.pause { 0.0 } else { dt * self.vitesse };
        self.sys.update(dt_sim);

        let aspect = screen_width() / screen_height();
        let target = self.cam.cible(&self.sys);
        let (cam_info, cam3d) = self.cam.construire(target, aspect);
        if clic && !sur_ui {
            self.cam.pick(&self.sys, &cam_info, aspect);
        }

        self.rendu
            .rendre(
                cam3d,
                &cam_info,
                &mut self.fond,
                &mut self.sys,
                self.menu.orbites,
                self.menu.orbites_etoiles,
                self.menu.zone,
            );

        let temps = if self.pause {
            "PAUSE".to_string()
        } else {
            format!("x{:.2}", self.vitesse)
        };
        crate::police::texte(
            &format!(
                "{}   |   {} FPS   |   {}   clic: centrer   P: pixel   G: aleatoire   R: shaders   Espace: pause   Haut/Bas: vitesse   Echap: menu",
                self.info,
                get_fps(),
                temps
            ),
            12.0,
            24.0,
            18.0,
            WHITE,
        );
        self.menu.dessiner(m, &self.presets, self.cam.focus_actif());
        false
    }

    /// Applique la vue par défaut du système : focalise l'étoile hôte (systèmes type S,
    /// trop étalés pour tout cadrer) ou recule pour englober tout (type P / étoile unique).
    fn appliquer_vue(&mut self) {
        match self.sys.vue() {
            Some((idx, d)) => {
                self.cam.set_focus(idx);
                self.cam.set_dist(d);
            }
            None => {
                self.cam.reset_focus();
                self.cam.set_dist((self.sys.rayon_englobant() * 1.5).clamp(350.0, 30000.0));
            }
        }
    }

    /// Reconstruit le système courant à partir de sa source (sans toucher la caméra).
    fn reconstruire(&self) -> (Systeme, String) {
        match &self.source {
            Source::Graine(g) => construire_systeme(*g),
            Source::Solaire => construire_preset_solaire(),
            Source::TauCeti => construire_preset_tau_ceti(),
            Source::Avatar => construire_preset_avatar(),
            Source::AlphaCentauri => construire_preset_alpha_centauri(),
            Source::Proxima => construire_preset_proxima(),
            Source::Binaire => construire_preset_binaire(),
            Source::Trinaire => construire_preset_trinaire(),
            Source::Quadruple => construire_preset_quadruple(),
        }
    }

    /// Applique l'action choisie dans le menu.
    fn appliquer(&mut self, a: ActionMenu) {
        match a {
            ActionMenu::Solaire => {
                let (s, i) = construire_preset_solaire();
                self.sys = s;
                self.info = i;
                self.source = Source::Solaire;
                self.cam.set_dist(1280.0);
                self.cam.reset_focus();
            }
            ActionMenu::TauCeti => {
                let (s, i) = construire_preset_tau_ceti();
                self.sys = s;
                self.info = i;
                self.source = Source::TauCeti;
                self.cam.set_dist(480.0);
                self.cam.reset_focus();
            }
            ActionMenu::Avatar => {
                let (s, i) = construire_preset_avatar();
                self.sys = s;
                self.info = i;
                self.source = Source::Avatar;
                self.cam.set_dist(560.0);
                self.cam.reset_focus();
            }
            ActionMenu::AlphaCentauri => {
                let (s, i) = construire_preset_alpha_centauri();
                self.sys = s;
                self.info = i;
                self.source = Source::AlphaCentauri;
                // Binaire large : focaliser ACA sur sa zone planétaire (voir definir_vue du preset).
                self.appliquer_vue();
            }
            ActionMenu::Proxima => {
                let (s, i) = construire_preset_proxima();
                self.sys = s;
                self.info = i;
                self.source = Source::Proxima;
                self.cam.set_dist(320.0);
                self.cam.reset_focus();
            }
            ActionMenu::Binaire => {
                let (s, i) = construire_preset_binaire();
                self.sys = s;
                self.info = i;
                self.source = Source::Binaire;
                self.cam.set_dist(1300.0);
                self.cam.reset_focus();
            }
            ActionMenu::Trinaire => {
                let (s, i) = construire_preset_trinaire();
                self.sys = s;
                self.info = i;
                self.source = Source::Trinaire;
                self.cam.set_dist(1100.0);
                self.cam.reset_focus();
            }
            ActionMenu::Quadruple => {
                let (s, i) = construire_preset_quadruple();
                self.sys = s;
                self.info = i;
                self.source = Source::Quadruple;
                self.cam.set_dist(1400.0);
                self.cam.reset_focus();
            }
            ActionMenu::Charger(idx) => {
                self.seed = self.presets[idx].graine;
                let (s, _) = construire_systeme(self.seed);
                self.sys = s;
                self.info = self.presets[idx].nom.clone();
                self.source = Source::Graine(self.seed);
                self.appliquer_vue();
            }
            ActionMenu::Aleatoire => {
                self.seed = nouvelle_graine(self.seed);
                let (s, i) = construire_systeme(self.seed);
                self.sys = s;
                self.info = i;
                self.source = Source::Graine(self.seed);
                self.appliquer_vue();
            }
            ActionMenu::Retour => self.cam.reset_focus(),
            ActionMenu::Quitter => {
                sauver_presets(&self.presets);
                std::process::exit(0);
            }
        }
    }
}

fn nouvelle_graine(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}
