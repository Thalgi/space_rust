use crate::astre::CameraInfo;
use crate::camera::Camera;
use crate::fond::Fond;
use crate::genese::{
    construire_preset_alpha_centauri, construire_preset_avatar, construire_preset_binaire,
    construire_preset_proxima, construire_preset_quadruple, construire_preset_solaire,
    construire_preset_tau_ceti, construire_preset_trinaire, construire_systeme,
    construire_systeme_autour,
};
use super::{bandeau, fiche, liste, marqueur, selecteur, trace_orbite};
use crate::menu::{ActionMenu, Menu};
use crate::rendu::{Rendu, RenduStandard};
use crate::systeme::Systeme;
use crate::{planete, soleil};
use macroquad::prelude::*;

use crate::camera::DIST_DEFAUT;

/// D'où provient le système courant : graine procédurale ou preset écrit à la main.
/// Sert à le reconstruire à l'identique lors du hot-reload des shaders.
enum Source {
    Graine(u64),
    /// Graine + type d'étoile imposé (menu « choisir astre »).
    Astre(u64, &'static str),
    Solaire,
    TauCeti,
    Avatar,
    AlphaCentauri,
    Proxima,
    Binaire,
    Trinaire,
    Quadruple,
}

/// **Skymap V2** : la vue système, plus des marqueurs triangulaires sur les
/// planètes et, quand une planète est sélectionnée, sur ses lunes — avec les
/// orbites de ces lunes. Fork de `Skymap` (V1 reste intacte), voir
/// `docs/superpowers/specs/2026-09-22-skymap-v2-marqueurs-design.md`.
pub struct SkymapV2 {
    seed: u64,
    sys: Systeme,
    info: String,
    fond: Fond,
    cam: Camera,
    rendu: RenduStandard,
    menu: Menu,
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
    /// Portrait d'astre du panneau de droite — capturé une fois par astre et
    /// gardé en cache (voir `ecran/vignette.rs`), pas refait chaque frame.
    ///
    /// **Doit être vidé chaque fois que `sys` est remplacé**, même raison et
    /// mêmes points de vidage que `icones` ci-dessous.
    vignette: super::vignette::Vignette,
    /// Icônes du sélecteur (colonne de gauche), une par astre déjà rencontré.
    ///
    /// **Doit être vidé chaque fois que `sys` est remplacé** (nouveau preset,
    /// graine aléatoire, hot-reload) : les icônes sont indexées par position
    /// dans `sys.astres`, et cet index ne désigne plus le même corps une fois
    /// le système reconstruit.
    icones: super::icone::IconesAstres,
    /// Astre dont le panneau de droite est ouvert.
    ///
    /// Distinct du **focus camera** : la camera suit un astre, le panneau en
    /// decrit un. Ils coincident aujourd'hui (cliquer centre **et** ouvre) mais
    /// ce sont deux questions differentes, et les confondre interdirait plus
    /// tard de consulter une planete sans deplacer la vue
    /// (`conception/interface.md` 4.1).
    fiche_ouverte: Option<usize>,
    /// Progression du glissement du panneau, 0 (fermé) à 1 (ouvert) — voir
    /// `fiche::avancer_ouverture`.
    fiche_anim: f32,
    /// Dernier astre affiché par le panneau — distinct de `fiche_ouverte`,
    /// qui redevient `None` dès la fermeture demandée : le panneau doit
    /// continuer à montrer **son** contenu pendant qu'il glisse encore hors
    /// champ, plutôt que de disparaître d'un coup pendant que l'animation
    /// tourne encore.
    fiche_affiche: Option<usize>,
    /// Vaisseau **armé** pour un lancement (bouton LANCER cliqué) — le
    /// prochain clic sur un autre astre devient sa destination
    /// (`docs/conception/trajectoires.md` §4 décision 8). `None` en usage
    /// normal ; jamais persistant au-delà d'un clic (succès, échec silencieux
    /// sur une destination non prédictible, ou annulation par reclic/vide).
    lancement: Option<usize>,
}

impl SkymapV2 {
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
            cam: Camera::new(DIST_DEFAUT),
            rendu: RenduStandard::new(),
            menu: Menu::v2(),
            source: Source::Solaire,
            vitesse: 1.0,
            pause: false,
            selecteur_replie: false,
            tresorerie: bandeau::Tresorerie::demo(),
            vignette: super::vignette::Vignette::new(),
            icones: super::icone::IconesAstres::new(),
            fiche_ouverte: None,
            fiche_anim: 0.0,
            fiche_affiche: None,
            lancement: None,
        }
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
            // V : couche voile des anneaux on/off. Instrument de mesure (P.2 bis) :
            // à cadrage identique, l'écart de FPS ne peut venir que d'elle.
            if is_key_pressed(KeyCode::V) {
                crate::disque::set_voiles(!crate::disque::voiles_actifs());
            }
            if is_key_pressed(KeyCode::G) {
                self.seed = nouvelle_graine(self.seed);
                self.source = Source::Graine(self.seed);
                let (s, i) = construire_systeme(self.seed);
                self.sys = s;
                self.info = i;
                self.icones = super::icone::IconesAstres::new();
                self.vignette = super::vignette::Vignette::new();
                self.oublier_selection();
                self.appliquer_vue();
            }
            if is_key_pressed(KeyCode::R) {
                // Hot-reload : recompile les shaders et reconstruit à l'identique.
                // Les icônes et la vignette aussi : c'est justement pour voir un
                // shader retouché que R existe, un cache resté en place le
                // manquerait.
                planete::vider_cache_materials();
                soleil::vider_cache_materials();
                let (s, i) = self.reconstruire();
                self.sys = s;
                self.info = i;
                self.icones = super::icone::IconesAstres::new();
                self.vignette = super::vignette::Vignette::new();
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
            self.menu.input(m, clic, &[], self.cam.focus_actif(), outils, sous_bandeau);
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
            || liste::zone_active_echelle_bouton(
                ecran, self.selecteur_replie, haut, selecteur::PART_LARGEUR_COLONNE, selecteur::ECHELLE_BOUTON_REPLI,
            ).contains(m)
            || (self.fiche_anim > 0.0 && fiche::rectangle(ecran).contains(m));

        // Bouton LANCER de la fiche affichée, si c'est un vaisseau pilotable
        // pas déjà armé — inclus dans `sur_ui` ci-dessus (couvert par le
        // rectangle du panneau tout entier), donc pas de zone à y ajouter :
        // juste le clic à traiter.
        if clic && self.lancement.is_none() {
            if let Some(idx) = self.fiche_affiche {
                if self.sys.pilotable_de(idx) && fiche::bouton_lancer(fiche::rectangle(ecran)).contains(m) {
                    self.lancement = Some(idx);
                }
            }
        }

        // Le **calcul** de la colonne se fait ici, tôt, parce que la caméra en
        // dépend. Le **dessin**, lui, attend la passe interface : dessiné
        // maintenant, il serait effacé par le rendu 3D qui suit — c'est
        // exactement le défaut qu'a connu la colonne de la vue composants
        // (`suivi/stations.md` F.7).
        let choix = self.selecteur_input(ecran, haut, m, clic);
        if let Some(idx) = choix {
            if let Some(idx_vaisseau) = self.lancement {
                self.tenter_lancement(idx_vaisseau, idx);
            } else {
                self.cam.set_focus(idx);
                self.fiche_ouverte = Some(idx);
            }
        }

        self.cam.input_orbite(sur_ui);
        let dt_sim = if self.pause { 0.0 } else { dt * self.vitesse };
        // L'animation des astres suit le temps RÉEL : le multiplicateur existe
        // pour faire tourner les orbites, pas pour accélérer les éruptions, qui
        // sont réglées comme un spectacle. La pause, elle, fige tout.
        let dt_anim = crate::systeme::DtAnimation(if self.pause { 0.0 } else { dt });
        self.sys.update(dt_sim, dt_anim);

        let aspect = screen_width() / screen_height();
        let target = self.cam.cible(&self.sys);
        let (cam_info, cam3d) = self.cam.construire(target, aspect);
        // Survol : le marqueur sous la souris, jamais à travers l'interface.
        let survole = if sur_ui {
            None
        } else {
            marqueur::plus_proche(&self.marqueurs(&cam_info, aspect), m, marqueur::RAYON_HIT_PX)
        };
        if clic && !sur_ui {
            // Un marqueur touché désigne son astre — même suite qu'un pick 3D
            // réussi, `set_focus` compris (c'est ce que fait `Camera::pick`).
            // Sinon, repli sur le pick 3D : étoiles, engins, tout ce qui n'a
            // pas de marqueur reste cliquable comme en V1.
            let pick = match survole {
                Some(idx) => {
                    self.cam.set_focus(idx);
                    Some(idx)
                }
                None => self.cam.pick(&self.sys, &cam_info, aspect),
            };
            if let Some(idx_vaisseau) = self.lancement {
                // Armé : ce clic choisit la destination (astre touché) ou
                // annule (clic dans le vide) — jamais une ouverture de fiche
                // normale tant que l'armement n'est pas résolu.
                match pick {
                    Some(idx) => self.tenter_lancement(idx_vaisseau, idx),
                    None => self.lancement = None,
                }
            } else {
                // Cliquer un astre l'ouvre ; cliquer le vide referme. Pas de croix
                // ni d'Echap : Echap sert deja a quitter l'ecran, et lui confier un
                // second role selon qu'un panneau est ouvert serait un etat cache
                // (`conception/interface.md` 4.1).
                self.fiche_ouverte = pick;
            }
        }
        // L'astre a pu disparaitre sous le panneau : changement de systeme par
        // G, R ou un preset. Un index perime designerait un autre corps.
        if self.fiche_ouverte.is_some_and(|i| i >= self.sys.nb_astres()) {
            self.fiche_ouverte = None;
        }

        // Anime le glissement du panneau vers sa cible (ouvert/fermé), et
        // retient le dernier astre affiché : `fiche_ouverte` redevient
        // `None` dès la fermeture demandée, mais le panneau doit continuer à
        // montrer son contenu pendant qu'il glisse encore hors champ.
        self.fiche_anim = fiche::avancer_ouverture(self.fiche_anim, self.fiche_ouverte.is_some(), dt);
        if self.fiche_ouverte.is_some() {
            self.fiche_affiche = self.fiche_ouverte;
        }

        let previsualisation = self.previsualisation_trajectoire(&cam_info, aspect, m, survole);
        self.rendu
            .rendre(
                cam3d,
                &cam_info,
                &mut self.fond,
                &mut self.sys,
                crate::systeme::Calques {
                    planetes: self.menu.orbites,
                    planetes_pleines: true,
                    etoiles: self.menu.orbites_etoiles,
                    cometes: self.menu.orbites_cometes,
                    // Le calque global tout-ou-rien est coupé en V2 : les
                    // orbites de lunes n'apparaissent que pour la planète
                    // sélectionnée (spec §6.4), quel que soit le bouton — et
                    // elles se tracent en passe interface (`trace_orbite`),
                    // pas ici, parce qu'un trait large n'existe pas en 3D.
                    lunes: false,
                    zone: self.menu.zone,
                    focus: self.cam.focus(),
                    previsualisation,
                },
            );

        // Orbites des lunes, puis marqueurs : passe interface, AVANT
        // bandeau/colonne/fiche, qui les recouvrent naturellement là où ils se
        // superposent. Les orbites d'abord, pour que les triangles restent
        // au-dessus du trait. Recalculés ici (et non repris du survol) parce
        // que le clic a pu changer la sélection, donc les lunes à montrer,
        // dans cette même frame.
        let ecran_px = vec2(screen_width(), screen_height());
        if let Some(p) = marqueur::planete_active(&self.sys, self.fiche_ouverte) {
            trace_orbite::dessiner_orbites_lunes(&self.sys, p, &cam_info, ecran_px, aspect);
        }
        let marqueurs = self.marqueurs(&cam_info, aspect);
        marqueur::dessiner(&self.sys, &marqueurs, survole, self.fiche_ouverte);

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
                "{}   |   {} FPS   |   zoom x{:.2}   |   vitesse {}   clic: centrer   P: pixel   V: voiles   G: aleatoire   R: shaders   Espace: pause   Haut/Bas: vitesse   Echap: menu",
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
        self.fiche_dessiner(ecran_ui, m);
        self.menu.dessiner(m, &[], self.cam.focus_actif(),
            bandeau::strip_outils(ecran_ui), bandeau::hauteur_occupee(ecran_ui));
        false
    }

    /// Les marqueurs de la frame : toutes les planètes, plus les lunes de la
    /// planète active s'il y en a une. Une seule source pour le survol, le
    /// clic et le dessin — deux listes calculées séparément finiraient par ne
    /// plus désigner la même chose.
    fn marqueurs(&self, cam_info: &CameraInfo, aspect: f32) -> Vec<marqueur::Marqueur> {
        let ecran = vec2(screen_width(), screen_height());
        let mut ms = marqueur::marqueurs_racines(&self.sys, cam_info, ecran, aspect);
        if let Some(p) = marqueur::planete_active(&self.sys, self.fiche_ouverte) {
            ms.extend(marqueur::marqueurs_lunes(&self.sys, p, cam_info, ecran, aspect));
        }
        ms
    }

    /// Tente de lancer `idx_vaisseau` (armé, `self.lancement == Some(idx_vaisseau)`)
    /// vers `idx_destination`. Désarme dans tous les cas — succès, échec
    /// silencieux (destination sans position prédictible, voir
    /// `Systeme::calculer_trajectoire`), ou reclic sur le vaisseau lui-même
    /// (annulation) : jamais d'état « armé » qui survit à ce clic.
    ///
    /// Un lancement réussi bascule aussi la caméra sur le vaisseau (Lot D,
    /// `docs/conception/trajectoires.md` §6) : `Camera::cible` relit
    /// `Systeme::position(idx)` chaque frame, quel que soit ce qui la fait
    /// bouger (orbite ou trajet) — suivre un mobile ne demande donc **rien**
    /// de plus qu'un focus, la mécanique existait déjà pour les astres en
    /// orbite.
    fn tenter_lancement(&mut self, idx_vaisseau: usize, idx_destination: usize) {
        self.lancement = None;
        if idx_destination == idx_vaisseau {
            return; // reclic sur le vaisseau armé : annule, ne vise pas lui-même
        }
        if let Some(traj) = self.trajectoire_vers(idx_vaisseau, idx_destination) {
            if self.sys.lancer(idx_vaisseau, traj) {
                self.cam.set_focus(idx_vaisseau);
            }
        }
    }

    /// Le trajet qu'un lancement **immédiat** de `idx_vaisseau` vers
    /// `idx_destination` produirait — factorisé entre le lancement réel
    /// ([`Self::tenter_lancement`]) et son aperçu
    /// ([`Self::previsualisation_trajectoire`]) : les deux doivent tomber
    /// sur exactement la même formule, sans quoi l'arc affiché avant le
    /// clic mentirait sur celui qu'on obtient après.
    fn trajectoire_vers(
        &self,
        idx_vaisseau: usize,
        idx_destination: usize,
    ) -> Option<crate::systeme::trajectoire::Trajectoire> {
        let depart = self.sys.position(idx_vaisseau);
        let t_depart = self.sys.temps();
        let vitesse = self.sys.stats_vol_de(idx_vaisseau).map_or(0.0, |s| s.vitesse);
        self.sys.calculer_trajectoire(idx_destination, depart, t_depart, vitesse, Vec3::Y)
    }

    /// Aperçu du trajet en cours d'armement (Lot D,
    /// `docs/conception/trajectoires.md` §6) : l'arc qu'un lancement
    /// immédiat vers l'astre **survolé** produirait. `None` hors armement,
    /// souris dans le vide, ou survol du vaisseau armé lui-même.
    ///
    /// Le survol d'un marqueur l'emporte, comme pour le clic ; sinon repli
    /// sur `Systeme::pick` (`&self`, sans effet de bord) plutôt que
    /// `Camera::pick`, qui déplacerait le focus pour un simple survol.
    fn previsualisation_trajectoire(
        &self,
        cam_info: &CameraInfo,
        aspect: f32,
        m: Vec2,
        survole: Option<usize>,
    ) -> Option<Vec<Vec3>> {
        let idx_vaisseau = self.lancement?;
        let idx_destination = match survole {
            Some(idx) => idx,
            None => {
                let ecran = vec2(screen_width(), screen_height());
                let dir = crate::camera::rayon_ecran(m, ecran, cam_info, aspect);
                self.sys.pick(cam_info.pos, dir)?
            }
        };
        if idx_destination == idx_vaisseau {
            return None;
        }
        Some(self.trajectoire_vers(idx_vaisseau, idx_destination)?.polyligne(48))
    }

    /// Entrees et clic de la colonne. Rend l'astre choisi, s'il y en a un.
    ///
    /// Ne dessine rien : appele **tot** dans la frame, avant le rendu 3D, parce
    /// que la camera a besoin de savoir si la souris est prise.
    fn selecteur_input(&mut self, ecran: Vec2, haut: f32, souris: Vec2, clic: bool) -> Option<usize> {
        if clic && liste::bouton_repli_echelle(ecran, selecteur::ECHELLE_BOUTON_REPLI).contains(souris) {
            self.selecteur_replie = !self.selecteur_replie;
            return None; // le clic a servi au bouton, pas a une selection
        }
        if self.selecteur_replie || !clic {
            return None;
        }
        let col = liste::colonne_items_depuis_echelle_bouton(
            ecran, haut, selecteur::PART_LARGEUR_COLONNE, selecteur::ECHELLE_BOUTON_REPLI,
        );
        let entrees = selecteur::entrees(&self.sys);
        let i = liste::item_sous_curseur_echelle_bas(col, entrees.len(), souris, selecteur::ECHELLE_LIGNE)?;
        // Le barycentre est une ligne d'arbre, pas un astre : rien a cadrer.
        entrees[i].idx
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
    ///
    /// Glisse depuis le bord droit (`fiche_anim`, `fiche::decalage_horizontal`)
    /// plutôt que d'apparaître/disparaître d'un coup, et montre `fiche_affiche`
    /// — pas `fiche_ouverte`, qui redevient `None` dès la fermeture demandée —
    /// pour garder son contenu pendant qu'il glisse encore hors champ.
    fn fiche_dessiner(&mut self, ecran: Vec2, m: Vec2) {
        if self.fiche_anim <= 0.0 {
            return; // completement ferme : rien a dessiner
        }
        let Some(idx) = self.fiche_affiche else { return };
        let Some(f) = fiche::fiche(&self.sys, idx) else { return };
        let repos = fiche::rectangle(ecran);
        let decalage = fiche::decalage_horizontal(repos, ecran.x, self.fiche_anim);
        let r = Rect::new(repos.x + decalage, repos.y, repos.w, repos.h);
        crate::ui::minitel_panel(r, &f.designation);

        // Vignette : l'astre **reellement rendu**, dans sa propre cible de
        // profondeur (voir `ecran/vignette.rs`).
        let cyan = Color::new(0.55, 1.0, 0.75, 1.0);
        let cote = (r.w * 0.42).min(r.h * 0.36);
        let zone = Rect::new(r.x + (r.w - cote) * 0.5, r.y + 32.0, cote, cote);
        self.vignette.dessiner(&mut self.sys, idx, zone);
        draw_rectangle_lines(zone.x, zone.y, zone.w, zone.h, 1.0, Color::new(0.0, 0.5, 0.5, 0.9));

        let mut y = zone.y + zone.h + 22.0;
        let ligne = |t: String, c: Color, y: &mut f32| {
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

        // Bouton LANCER (ou statut d'armement) : un vaisseau pilotable
        // seulement (décision 8, `docs/conception/trajectoires.md` §4). Le
        // clic lui-même est traité dans `frame()`, pas ici — passe dessin
        // uniquement, comme le reste de cette fonction.
        if f.pilotable {
            let bouton = fiche::bouton_lancer(r);
            if self.lancement == Some(idx) {
                crate::ui::minitel_ligne(bouton, "CHOISIR UNE DESTINATION...", m);
            } else {
                crate::ui::minitel_ligne(bouton, "LANCER", m);
            }
        }
    }

    /// Fond et bordure de la colonne, en style « liquide » — sur demande, un
    /// premier essai à ajuster : un dégradé vertical approché par bandes
    /// (plus opaque et légèrement teinté cyan en haut, comme un reflet de
    /// surface, s'effaçant vers le bas) plutôt qu'un aplat, un ménisque net
    /// tout en haut, et une bordure **grise à 30 % d'opacité** — le cyan
    /// plein d'origine est retiré, pas juste éclairci.
    fn colonne_fond_liquide(col: Rect) {
        const BANDES: i32 = 16;
        for i in 0..BANDES {
            let t = i as f32 / (BANDES - 1) as f32; // 0 en haut, 1 en bas
            let h_bande = col.h / BANDES as f32;
            let alpha = 0.85 - 0.25 * t;
            let cyan = 0.06 * (1.0 - t);
            draw_rectangle(
                col.x,
                col.y + h_bande * i as f32,
                col.w,
                h_bande + 1.0,
                Color::new(0.02, 0.03 + cyan, 0.12 + cyan, alpha),
            );
        }
        // Ménisque : un filet clair au tout premier bord, comme la surface
        // d'un liquide qui accroche la lumière.
        draw_rectangle(col.x, col.y, col.w, 2.0, Color::new(0.5, 0.85, 0.9, 0.4));
        draw_rectangle_lines(col.x, col.y, col.w, col.h, 1.0, Color::new(0.5, 0.5, 0.5, 0.3));
    }

    /// Dessin de la colonne. **Passe interface uniquement** (au sens de
    /// `conception/assembleur.md` §10.9 : ce qui se décide reste dans
    /// `selecteur`/`liste`) — sauf l'icône elle-même, qui a besoin d'un rendu
    /// 3D différé et donc de `&mut self` pour son cache (`self.icones`).
    fn selecteur_dessiner(&mut self, ecran: Vec2, haut: f32, souris: Vec2) {
        let cyan = Color::new(0.0, 0.85, 0.85, 1.0);
        let bouton = liste::bouton_repli_echelle(ecran, selecteur::ECHELLE_BOUTON_REPLI);

        if !self.selecteur_replie {
            let col = liste::colonne_items_depuis_echelle_bouton(
                ecran, haut, selecteur::PART_LARGEUR_COLONNE, selecteur::ECHELLE_BOUTON_REPLI,
            );
            let entrees = selecteur::entrees(&self.sys);
            let n = entrees.len();

            // --- Complète le cache d'icônes, avant tout dessin 2D ---
            //
            // Une seule icône manquante rendue par frame (`completer` le
            // garantit) : une bascule de caméra 3D vers une cible hors écran,
            // suivie d'une **seconde** dans la même frame, ressort blanche —
            // mesuré à l'écran, voir `ecran/icone.rs`. La boucle de dessin qui
            // suit n'en déclenche plus aucune, elle ne fait que lire
            // (`IconesAstres::prete`) : le peuplement se termine sur quelques
            // frames plutôt que de tenter tout d'un coup.
            self.icones.completer(&mut self.sys, entrees.iter().filter_map(|e| e.idx));

            Self::colonne_fond_liquide(col);

            let h = liste::hauteur_ligne_echelle(col, n, selecteur::ECHELLE_LIGNE);
            let taille = (h * 0.34).clamp(crate::police::TAILLE_MIN, 18.0);
            let focus = self.cam.focus();
            for (i, e) in entrees.iter().enumerate() {
                let r = liste::ligne_echelle_bas(col, n, i, selecteur::ECHELLE_LIGNE);
                if r.y < col.y {
                    continue; // deborde par le haut : rien a ecrire au-dessus de la colonne
                }
                if r.contains(souris) {
                    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.45, 0.45, 0.35));
                }
                if e.idx.is_some() && focus == e.idx {
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, cyan);
                }

                // Une planète se decale d'une tabulation sous son etoile —
                // comme dans un fichier texte, pour que la colonne se lise
                // d'un coup d'œil comme une liste **sous** le Soleil.
                let indent = if e.profondeur > 0 { selecteur::INDENT_TAB } else { 0.0 };
                let rayon_plein = selecteur::rayon_icone(h, Some(true));
                let cx = r.x + 4.0 + indent + rayon_plein;
                let mi = r.y + r.h * 0.5;

                // --- Trait d'arbre, a droite de l'icone ---
                //
                // Une racine n'en a pas ; un fils recoit un coude, et le
                // **dernier** un coude ferme, faute de quoi le trait vertical
                // descendrait sous la derniere planete vers rien.
                let arbre_x = cx + rayon_plein + 5.0;
                if e.profondeur > 0 {
                    let trait_col = Color::new(0.0, 0.55, 0.55, 0.85);
                    let bas = if e.branche == selecteur::Branche::Dernier { mi } else { r.y + r.h };
                    draw_line(arbre_x, r.y, arbre_x, bas, 1.0, trait_col);
                    draw_line(cx + rayon_plein + 1.0, mi, arbre_x, mi, 1.0, trait_col);
                }

                match e.idx {
                    Some(i) => {
                        let rayon = selecteur::rayon_icone(h, self.sys.gazeuse_de(i));
                        match self.icones.prete(i) {
                            Some(tex) => {
                                // Une cible d'astre à anneau est plus grande
                                // qu'une cible standard (`icone::cote_gazeuse`)
                                // — l'échelle de dessin doit suivre, sinon ce
                                // surplus de pixels (l'anneau) se retrouve
                                // compressé dans la même case au lieu d'en
                                // déborder. Voir `ecran/icone.rs`.
                                let echelle = super::icone::echelle_dessin(tex.width());
                                let cote = rayon * 2.0 * echelle;
                                draw_texture_ex(
                                    tex,
                                    cx - cote * 0.5,
                                    mi - cote * 0.5,
                                    WHITE,
                                    DrawTextureParams { dest_size: Some(vec2(cote, cote)), ..Default::default() },
                                )
                            }
                            // Pas encore rendue (peuplement en cours, une par
                            // frame) ou astre sans rayon : repli sur l'ancienne
                            // pastille de couleur plutôt que rien.
                            None => draw_circle(cx, mi, rayon, selecteur::teinte_astre(&self.sys, i, e.categorie)),
                        }
                    }
                    // Le barycentre n'est pas un corps, donc pas d'icone : un
                    // petit reperage creux, comme avant.
                    None => draw_circle_lines(cx, mi, rayon_plein * 0.8, 1.0, Color::new(0.35, 0.45, 0.5, 1.0)),
                }

                let tx = arbre_x + 5.0;
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
        // Le signe grandit avec le bouton, sinon il flotte, minuscule, dans
        // un bouton deux fois plus grand que lui.
        let taille_signe = 18.0 * selecteur::ECHELLE_BOUTON_REPLI;
        crate::police::texte(signe, bouton.x + bouton.w * 0.18, bouton.y + bouton.h * 0.72, taille_signe, cyan);
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
            Source::Astre(g, nom) => {
                construire_systeme_autour(*g, nom).unwrap_or_else(|| construire_systeme(*g))
            }
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

    /// Efface la sélection courante (V2 seulement, `skymap.rs` n'est pas
    /// touché) : `fiche_ouverte`, `fiche_affiche` et `lancement` désignent un
    /// astre par **index dans `self.sys`**, et cet index ne désigne plus le
    /// même corps une fois le système remplacé (G, presets, aléatoire...) —
    /// contrairement au hot-reload (R), qui reconstruit le **même** système et
    /// n'appelle donc pas cette méthode.
    fn oublier_selection(&mut self) {
        self.fiche_ouverte = None;
        self.fiche_affiche = None;
        self.lancement = None;
    }

    /// Applique l'action choisie dans le menu.
    fn appliquer(&mut self, a: ActionMenu) {
        // Seul `Retour` ne remplace pas `self.sys` (il ne fait que recentrer
        // la caméra) ; `Quitter` sort du process avant d'atteindre la fin de
        // cette methode, donc peu importe sa valeur ici.
        let remplace_systeme = !matches!(&a, ActionMenu::Retour);
        match a {
            ActionMenu::Solaire => {
                let (s, i) = construire_preset_solaire();
                self.sys = s;
                self.info = i;
                self.source = Source::Solaire;
                self.cam.set_dist(DIST_DEFAUT);
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
            ActionMenu::Astre(nom) => {
                // Une nouvelle graine à chaque clic : recliquer le même type
                // donne un autre système (et, pour les géantes rouges ou le trou
                // noir, possiblement un autre aspect ou un autre régime).
                self.seed = nouvelle_graine(self.seed);
                // `None` est impossible pour un nom venu de la liste (elle est
                // balayée dans les tables du tirage) ; retomber sur un tirage
                // quelconque vaut mieux que fermer le jeu.
                let (s, i) = construire_systeme_autour(self.seed, nom)
                    .unwrap_or_else(|| construire_systeme(self.seed));
                self.sys = s;
                self.info = i;
                self.source = Source::Astre(self.seed, nom);
                self.appliquer_vue();
            }
            // Sauvegarde mise de côté (2026-10-19) : pas de systèmes sauvés à
            // charger au menu V2…
            ActionMenu::Charger(_) => {}
            // … et QUITTER sort sans rien écrire.
            ActionMenu::Quitter => std::process::exit(0),
            ActionMenu::Aleatoire => {
                self.seed = nouvelle_graine(self.seed);
                let (s, i) = construire_systeme(self.seed);
                self.sys = s;
                self.info = i;
                self.source = Source::Graine(self.seed);
                self.appliquer_vue();
            }
            ActionMenu::Retour => self.cam.reset_focus(),
        }
        // Toutes les branches sauf Retour/Quitter remplacent `self.sys` — vider
        // sans conditionner à laquelle est la garantie qui ne peut pas rater la
        // prochaine ajoutée. Inoffensif là où `sys` n'a pas bougé : les icônes
        // et la vignette se re-capturent, identiques, dès le prochain dessin.
        self.icones = super::icone::IconesAstres::new();
        self.vignette = super::vignette::Vignette::new();
        if remplace_systeme {
            self.oublier_selection();
        }
    }
}

fn nouvelle_graine(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1)
}
