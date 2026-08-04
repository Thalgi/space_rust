use crate::camera::Camera;
use crate::fond::Fond;
use crate::genese::{
    charger_presets, construire_preset_alpha_centauri, construire_preset_avatar,
    construire_preset_binaire, construire_preset_proxima, construire_preset_quadruple,
    construire_preset_solaire, construire_preset_tau_ceti, construire_preset_trinaire,
    construire_systeme, sauver_presets, PresetSauve,
};
use super::{bandeau, fiche, liste, selecteur};
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
    /// Colonne de gauche repliee ? L'etat vit **dans l'ecran** : `liste` et
    /// `selecteur` restent des calculateurs sans memoire
    /// (`conception/interface.md` 4.3).
    selecteur_replie: bool,
    /// Stocks affiches par le bandeau.
    ///
    /// ATTENTION : bouche-trou fige, rien ne le fait bouger — il n'y a pas
    /// d'economie dans ce depot (dette D-INT-2). La barre a ete ecrite pour
    /// pouvoir l'attendre : elle affiche une `Tresorerie`, d'ou qu'elle vienne.
    tresorerie: bandeau::Tresorerie,
    /// Cible de rendu du portrait d'astre (panneau de droite).
    vignette: super::vignette::Vignette,
    /// Astre dont le panneau de droite est ouvert.
    ///
    /// Distinct du **focus camera** : la camera suit un astre, le panneau en
    /// decrit un. Ils coincident aujourd'hui (cliquer centre **et** ouvre) mais
    /// ce sont deux questions differentes, et les confondre interdirait plus
    /// tard de consulter une planete sans deplacer la vue
    /// (`conception/interface.md` 4.1).
    fiche_ouverte: Option<usize>,
}

impl Skymap {
    /// **Le système solaire est la vue de départ.**
    ///
    /// C'était une graine procédurale (`1`), utile tant que l'écran servait à
    /// mettre au point le générateur. Depuis que la vue système porte
    /// l'interface de jeu, elle doit ouvrir sur un lieu **connu** : on y
    /// reconnaît les planètes, on y juge les distances, et l'ISS y donne
    /// l'échelle. `G` continue de tirer un système au hasard.
    pub fn new() -> Self {
        let seed: u64 = 1;
        let (sys, info) = construire_preset_solaire();
        Self {
            seed,
            sys,
            info,
            fond: Fond::new(900),
            // Recul qui cadre le systeme solaire jusqu'a Neptune : la meme
            // valeur que `ActionMenu::Solaire`, pour que le depart et le
            // rechargement du preset donnent la meme vue.
            cam: Camera::new(1280.0),
            rendu: RenduStandard::new(),
            menu: Menu::new(),
            presets: charger_presets(),
            source: Source::Solaire,
            vitesse: 1.0,
            pause: false,
            selecteur_replie: false,
            tresorerie: bandeau::Tresorerie::demo(),
            vignette: super::vignette::Vignette::new(),
            fiche_ouverte: None,
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
        //
        // Le menu **reçoit** sa place au lieu de la calculer : la bande basse
        // réservée à l'outillage et le premier `y` libre sous la barre de
        // ressources. Il ne connaît pas la mise en page du jeu, il s'y range.
        let ecran_ui = vec2(screen_width(), screen_height());
        let outils = bandeau::strip_outils(ecran_ui);
        let sous_bandeau = bandeau::hauteur_occupee(ecran_ui);
        let (sur_menu, action) =
            self.menu.input(m, clic, self.presets.len(), self.cam.focus_actif(), outils, sous_bandeau);
        if let Some(a) = action {
            self.appliquer(a);
        }

        // **Une seule porte** pour tout ce qui mange la souris : caméra et
        // picking la consultent tous les deux. Une zone qui oublierait de s'y
        // déclarer ferait pivoter la vue derrière elle, et rien en test ne le
        // dirait (`conception/interface.md` 4.2).
        let ecran = vec2(screen_width(), screen_height());
        // Le bandeau dit lui-meme la place qu'il prend, et la colonne la lui
        // demande : deux constantes recopiees finiraient par diverger, et les
        // deux zones se recouvriraient sans que rien ne le dise.
        let haut = bandeau::hauteur_occupee(ecran);
        let sur_ui = sur_menu
            || bandeau::rectangle(ecran).contains(m)
            || liste::zone_active(ecran, self.selecteur_replie, haut).contains(m)
            || (self.fiche_ouverte.is_some() && fiche::rectangle(ecran).contains(m));

        // Le **calcul** de la colonne se fait ici, tôt, parce que la caméra en
        // dépend. Le **dessin**, lui, attend la passe interface : dessiné
        // maintenant, il serait effacé par le rendu 3D qui suit — c'est
        // exactement le défaut qu'a connu la colonne de la vue composants
        // (`suivi/stations.md` F.7).
        let choix = self.selecteur_input(ecran, haut, m, clic);
        if let Some(idx) = choix {
            self.cam.set_focus(idx);
            self.fiche_ouverte = Some(idx);
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
            // Cliquer un astre l'ouvre ; cliquer le vide referme. Pas de croix
            // ni d'Echap : Echap sert deja a quitter l'ecran, et lui confier un
            // second role selon qu'un panneau est ouvert serait un etat cache
            // (`conception/interface.md` 4.1).
            self.fiche_ouverte = self.cam.pick(&self.sys, &cam_info, aspect);
        }
        // L'astre a pu disparaitre sous le panneau : changement de systeme par
        // G, R ou un preset. Un index perime designerait un autre corps.
        if self.fiche_ouverte.is_some_and(|i| i >= self.sys.nb_astres()) {
            self.fiche_ouverte = None;
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
        // Ligne d'outillage de developpement (graine, FPS, raccourcis). Passee
        // **en bas** : le haut de l'ecran appartient desormais au bandeau de
        // ressources, et les deux se recouvraient.
        crate::police::texte(
            &format!(
                "{}   |   {} FPS   |   zoom x{:.2}   |   vitesse {}   clic: centrer   P: pixel   G: aleatoire   R: shaders   Espace: pause   Haut/Bas: vitesse   Echap: menu",
                self.info,
                get_fps(),
                self.cam.zoom(),
                temps
            ),
            bandeau::strip_outils(vec2(screen_width(), screen_height())).x,
            screen_height() - 8.0,
            15.0,
            Color::new(0.55, 0.7, 0.75, 1.0),
        );
        let ecran_ui = vec2(screen_width(), screen_height());
        self.bandeau_dessiner(ecran_ui);
        self.selecteur_dessiner(ecran_ui, bandeau::hauteur_occupee(ecran_ui), m);
        self.fiche_dessiner(ecran_ui);
        self.menu.dessiner(m, &self.presets, self.cam.focus_actif(),
            bandeau::strip_outils(ecran_ui), bandeau::hauteur_occupee(ecran_ui));
        false
    }

    /// Entrees et clic de la colonne. Rend l'astre choisi, s'il y en a un.
    ///
    /// Ne dessine rien : appele **tot** dans la frame, avant le rendu 3D, parce
    /// que la camera a besoin de savoir si la souris est prise.
    fn selecteur_input(&mut self, ecran: Vec2, haut: f32, souris: Vec2, clic: bool) -> Option<usize> {
        if clic && liste::bouton_repli(ecran).contains(souris) {
            self.selecteur_replie = !self.selecteur_replie;
            return None; // le clic a servi au bouton, pas a une selection
        }
        if self.selecteur_replie || !clic {
            return None;
        }
        let col = liste::colonne_items_depuis(ecran, haut);
        let entrees = selecteur::entrees(&self.sys);
        let i = liste::item_sous_curseur(col, entrees.len(), souris)?;
        Some(entrees[i].idx)
    }

    /// Barre de ressources et nom du systeme. **Passe interface uniquement.**
    fn bandeau_dessiner(&self, ecran: Vec2) {
        let b = bandeau::rectangle(ecran);
        draw_rectangle(b.x, b.y, b.w, b.h, Color::new(0.02, 0.03, 0.12, 0.82));
        draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.0, Color::new(0.0, 0.5, 0.5, 0.85));

        for r in crate::sprites::Ressource::TOUTES {
            let c = bandeau::case(ecran, r);
            let cote = crate::sprites::taille_ecran(c.h - 4.0);
            let y = c.y + (c.h - cote) * 0.5;
            crate::sprites::dessiner(r, c.x + 3.0, y, cote);
            let t = bandeau::abreger(self.tresorerie.quantite(r));
            crate::police::texte(&t, c.x + cote + 6.0, c.y + c.h * 0.72, 15.0, WHITE);
        }

        // Nom du systeme : celui de l'etoile hote, s'il existe. Sinon le
        // libelle de generation — un systeme engendre n'a pas de nom propre.
        let nom = bandeau::ligne_nom(ecran);
        let titre = match self.sys.nom_systeme() {
            Some(n) => format!("SYSTEME {}", n.to_uppercase()),
            None => self.info.to_uppercase(),
        };
        let titre = liste::tronquer(&titre, nom.w - 8.0, 19);
        crate::police::texte(&titre, nom.x + 4.0, nom.y + nom.h * 0.72, 19.0,
            Color::new(0.0, 0.9, 0.9, 1.0));
    }

    /// Panneau de droite. **Passe interface uniquement**, comme la colonne.
    fn fiche_dessiner(&mut self, ecran: Vec2) {
        let Some(idx) = self.fiche_ouverte else { return };
        let Some(f) = fiche::fiche(&self.sys, idx) else { return };
        let r = fiche::rectangle(ecran);
        crate::ui::minitel_panel(r, &f.designation);

        // Vignette : l'astre **reellement rendu**, dans sa propre cible de
        // profondeur (voir `ecran/vignette.rs`).
        let cyan = Color::new(0.55, 1.0, 0.75, 1.0);
        let cote = (r.w * 0.42).min(r.h * 0.36);
        let zone = Rect::new(r.x + (r.w - cote) * 0.5, r.y + 32.0, cote, cote);
        self.vignette.dessiner(&mut self.sys, idx, zone);
        draw_rectangle_lines(zone.x, zone.y, zone.w, zone.h, 1.0, Color::new(0.0, 0.5, 0.5, 0.9));

        let mut y = zone.y + zone.h + 22.0;
        let mut ligne = |t: String, c: Color, y: &mut f32| {
            crate::police::texte(&t, r.x + 12.0, *y, 17.0, c);
            *y += 22.0;
        };
        ligne(fiche::nom_categorie(f.categorie).to_string(), cyan, &mut y);
        if f.categorie != crate::astre::Categorie::Etoile {
            ligne(format!("Distance : {:.2} UA", f.distance_ua), cyan, &mut y);
        }
        ligne(format!("Rayon : {:.2}", f.rayon), cyan, &mut y);

        // L'habitabilite est la seule ligne qui se colore : c'est le
        // renseignement que le schema met en avant.
        use crate::ecran::fiche::Habitabilite;
        let teinte_hab = match f.habitabilite {
            Habitabilite::Habitable => Color::new(0.4, 1.0, 0.5, 1.0),
            Habitabilite::TropChaud => Color::new(1.0, 0.55, 0.35, 1.0),
            Habitabilite::TropFroid => Color::new(0.55, 0.75, 1.0, 1.0),
            Habitabilite::SansObjet => Color::new(0.5, 0.55, 0.6, 1.0),
        };
        if f.habitabilite != Habitabilite::SansObjet {
            ligne(f.habitabilite.libelle().to_string(), teinte_hab, &mut y);
        }
    }

    /// Dessin de la colonne. **Passe interface uniquement.**
    fn selecteur_dessiner(&self, ecran: Vec2, haut: f32, souris: Vec2) {
        let cyan = Color::new(0.0, 0.85, 0.85, 1.0);
        let bouton = liste::bouton_repli(ecran);

        if !self.selecteur_replie {
            let col = liste::colonne_items_depuis(ecran, haut);
            let entrees = selecteur::entrees(&self.sys);
            let n = entrees.len();
            draw_rectangle(col.x, col.y, col.w, col.h, Color::new(0.02, 0.03, 0.12, 0.85));
            draw_rectangle_lines(col.x, col.y, col.w, col.h, 1.0, Color::new(0.0, 0.5, 0.5, 0.9));

            let h = liste::hauteur_ligne(col, n);
            let taille = (h * 0.62).clamp(11.0, 18.0);
            let focus = self.cam.focus();
            for (i, e) in entrees.iter().enumerate() {
                let r = liste::ligne(col, n, i);
                if r.y + r.h > col.y + col.h {
                    break; // deborde : on n'ecrit pas sous la colonne
                }
                if r.contains(souris) {
                    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.45, 0.45, 0.35));
                }
                if focus == Some(e.idx) {
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, cyan);
                }
                // Pastille : bouche-trou en attendant une vraie vignette (D-INT-1).
                let retrait = 4.0 + e.profondeur as f32 * 8.0;
                let rayon = (h * 0.26).clamp(3.0, 7.0);
                let cx = r.x + retrait + rayon;
                draw_circle(cx, r.y + r.h * 0.5, rayon, selecteur::teinte_astre(&self.sys, e.idx, e.categorie));

                let tx = cx + rayon + 4.0;
                let dispo = r.x + r.w - tx - 3.0;
                let t = liste::tronquer(&e.libelle, dispo, taille as u16);
                crate::police::texte(&t, tx, r.y + r.h * 0.5 + taille * 0.32, taille, WHITE);
            }
        }

        // Le bouton reste **a la meme place** repliee ou non : il faut pouvoir
        // rouvrir sans aller le chercher.
        let survol = bouton.contains(souris);
        draw_rectangle(bouton.x, bouton.y, bouton.w, bouton.h,
            if survol { Color::new(0.0, 0.45, 0.45, 0.9) } else { Color::new(0.02, 0.03, 0.12, 0.9) });
        draw_rectangle_lines(bouton.x, bouton.y, bouton.w, bouton.h, 1.0, cyan);
        let signe = if self.selecteur_replie { ">>" } else { "<<" };
        crate::police::texte(signe, bouton.x + 4.0, bouton.y + bouton.h * 0.72, 18.0, cyan);
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
