use crate::astre::{Astre, CameraInfo};
use crate::disque::{Disque, DisqueConfig};
use crate::etoile::UA;
use crate::genese::{preset_gazeuse, preset_tellurique, MASSE_ETOILE};
use crate::planete::Planete;
use crate::soleil::Soleil;
use macroquad::prelude::*;
use macroquad::rand::srand;

/// Galerie des champs de débris : UN cas par écran, défilement molette case
/// par case. Les cas stellaires sont à l'ÉCHELLE RÉELLE du jeu (rayons en UA,
/// masse de genese, étoile de référence au centre) : ce qu'on voit ici est
/// l'aspect final en skymap. Banc de validation de docs/conception/ceintures.md.
pub struct GalerieDisques {
    seed: u64,
    cellules: Vec<Cellule>,
    index: usize,      // case affichée (le scroll converge vers index * ch)
    scroll: f32,
    vitesse: f32,      // multiplicateur temporel (Haut/Bas), comme en skymap
    cible: Option<RenderTarget>,
    rt_dims: (u32, u32),
    // Colonne de gauche : atteindre un cas d'un clic au lieu de dérouler la
    // molette. Repliable — treize cas, c'est une liste courte, mais la colonne
    // mange un dixième de la largeur du rendu.
    liste_repliee: bool,
}

impl GalerieDisques {
    /// Dessin de la colonne. **Passe interface uniquement** — le calcul
    /// (colonne, lignes, item visé, troncature) vit dans `ecran::liste` et s'y
    /// teste ; ici il ne reste que la pose des rectangles.
    fn dessiner_liste(&self, ecran: Vec2, haut: f32, souris: Vec2) {
        use crate::ecran::liste;
        let cyan = Color::new(0.0, 0.85, 0.85, 1.0);

        if !self.liste_repliee {
            let col = liste::colonne_items_depuis(ecran, haut);
            let n = self.cellules.len();
            draw_rectangle(col.x, col.y, col.w, col.h, Color::new(0.02, 0.03, 0.12, 0.88));
            draw_rectangle_lines(col.x, col.y, col.w, col.h, 1.0, Color::new(0.0, 0.5, 0.5, 0.9));

            let h = liste::hauteur_ligne(col, n);
            let taille = (h * 0.55).clamp(crate::police::TAILLE_MIN, 15.0);
            for (i, cel) in self.cellules.iter().enumerate() {
                let r = liste::ligne(col, n, i);
                if r.y + r.h > col.y + col.h {
                    break; // déborde : on n'écrit pas sous la colonne
                }
                let courant = i == self.index;
                let survol = r.contains(souris);
                let (fond, texte) = match (courant, survol) {
                    (true, _) => (cyan, BLACK),
                    (false, true) => (Color::new(0.0, 0.45, 0.45, 0.35), Color::new(0.8, 1.0, 1.0, 1.0)),
                    _ => (Color::new(0.0, 0.0, 0.0, 0.0), Color::new(0.55, 0.8, 0.8, 1.0)),
                };
                draw_rectangle(r.x, r.y, r.w, r.h, fond);
                // `abrege` coupe au deux-points, `tronquer` mesure avec la vraie
                // police : « Ceinture d'asteroides (2.2 - 3.3 UA) » ne tient pas
                // dans un dixième d'écran.
                let t = liste::tronquer(liste::abrege(&cel.nom), r.w - 8.0, taille as u16);
                crate::police::texte(&t, r.x + 4.0, r.y + r.h * 0.5 + taille * 0.34, taille, texte);
            }
        }

        // Le bouton reste à la MÊME place repliée ou non : il faut pouvoir
        // rouvrir sans aller le chercher.
        let bouton = liste::bouton_repli(ecran);
        let survol = bouton.contains(souris);
        draw_rectangle(
            bouton.x,
            bouton.y,
            bouton.w,
            bouton.h,
            if survol { Color::new(0.0, 0.45, 0.45, 0.9) } else { Color::new(0.02, 0.03, 0.12, 0.9) },
        );
        draw_rectangle_lines(bouton.x, bouton.y, bouton.w, bouton.h, 1.0, cyan);
        let fleche = if self.liste_repliee { ">" } else { "<" };
        let tw = crate::police::mesure(fleche, 14);
        crate::police::texte(
            fleche,
            bouton.x + (bouton.w - tw) * 0.5,
            bouton.y + bouton.h * 0.5 + 5.0,
            14.0,
            cyan,
        );
    }
}

/// Contenu d'une case : champ stellaire (étoile de référence + disque à
/// l'échelle réelle + corps embarqués dans les lacunes), planète annelée,
/// ou planète + champ de particules.
enum Contenu {
    /// Le Vec associe (index de lacune, corps) : le corps est replacé chaque
    /// frame sur `Disque::position_lacune` (lune bergère, proto-planète).
    EtoileEtDisque(Soleil, Disque, Vec<(usize, Planete)>),
    Planete(Planete),
    /// Plusieurs disques superposables (anneaux multiples, plans différents).
    PlaneteEtDisques(Planete, Vec<Disque>),
}

struct Cellule {
    nom: String,
    contenu: Contenu,
    cam_pos: Vec3, // cadrage précalculé à la construction
}

impl GalerieDisques {
    pub fn new() -> Self {
        let mut g = Self {
            seed: 1,
            cellules: Vec::new(),
            index: 0,
            scroll: 0.0,
            vitesse: 8.0, // les périodes réelles sont lentes (Kepler à l'UA)
            cible: None,
            rt_dims: (0, 0),
            liste_repliee: false,
        };
        g.construire();
        g
    }

    /// Catalogue des cas. Cas stellaires : mêmes rayons/masse que `genese`
    /// (échelle réelle) — les tailles de particules sont donc jugeables telles
    /// qu'elles apparaîtront en skymap.
    fn construire(&mut self) {
        srand(self.seed);
        self.cellules.clear();
        let m = MASSE_ETOILE;

        // Étoile + disque, vue en surplomb cadrée sur le bord externe.
        let jaune = vec3(1.0, 0.93, 0.78);
        let orange = vec3(1.0, 0.62, 0.3);
        let mut e = |nom: &str, cfg: DisqueConfig, soleil: Soleil, recul: f32,
                     corps: Vec<(usize, Planete)>| {
            let cam_pos = vec3(0.0, cfg.externe * 0.95, cfg.externe * recul);
            self.cellules.push(Cellule {
                nom: nom.to_string(),
                contenu: Contenu::EtoileEtDisque(soleil, Disque::new(cfg), corps),
                cam_pos,
            });
        };

        // Petit corps à loger dans une lacune (lune bergère, proto-planète).
        let corps_lacune = |nom_preset: &str, taille: f32| {
            let mut app = preset_tellurique(nom_preset);
            app.taille = taille;
            Planete::new(Vec3::ZERO, Vec3::ZERO, taille, 1.0, app, Vec::new())
        };

        // --- Champs stellaires, échelle réelle (genese : 2.2..3.3 UA etc.) ---
        e(
            "Ceinture d'asteroides (2.2 - 3.3 UA)",
            DisqueConfig::asteroides(900, 2.2 * UA, 3.3 * UA, m),
            Soleil::new(Vec3::ZERO, 1.8, jaune, 1.0),
            2.15,
            Vec::new(),
        );
        e(
            "Ceinture de Kuiper (30 - 46 UA)",
            DisqueConfig::kuiper(1400, 30.0 * UA, 46.0 * UA, m),
            Soleil::new(Vec3::ZERO, 1.8, jaune, 1.0),
            2.15,
            Vec::new(),
        );
        e(
            "Disque epars (40 - 70 UA)",
            DisqueConfig::epars(1100, 40.0 * UA, 70.0 * UA, m),
            Soleil::new(Vec3::ZERO, 1.8, jaune, 1.0),
            2.15,
            Vec::new(),
        );
        e(
            "Nuage de Oort (55 - 70 UA, compresse)",
            DisqueConfig::oort(1800, 55.0 * UA, 70.0 * UA, m),
            Soleil::new(Vec3::ZERO, 1.8, jaune, 1.0),
            1.9, // coquille : caméra plus proche (zfar)
            Vec::new(),
        );
        e(
            "Debris recents (0.8 - 1.6 UA)",
            DisqueConfig::debris_recents(1400, 0.8 * UA, 1.6 * UA, m),
            Soleil::new(Vec3::ZERO, 1.8, jaune, 1.0),
            2.15,
            Vec::new(),
        );
        // Lacune vivante : la ceinture reçoit un voile fin pour rendre les
        // festons lisibles, et le corps orbite DANS son sillon.
        e(
            "Sillon de lune bergere + festons (1.6 - 3.2 UA)",
            DisqueConfig {
                voile_alpha: 0.35,
                voile_couleur: vec3(0.6, 0.55, 0.48),
                voile_couleur2: vec3(0.66, 0.6, 0.52),
                voile_bord: 0.06,
                rotation_voile: 0.02,
                ..DisqueConfig::asteroides(2000, 1.6 * UA, 3.2 * UA, m)
            }
            .avec_lacune_ondulee(0.5, 0.045, 0.97, 0.8),
            Soleil::new(Vec3::ZERO, 1.8, jaune, 1.0),
            2.15,
            vec![(0, corps_lacune("Fer (Mercure)", 1.8))],
        );
        e(
            "Disque protoplanetaire (0.3 - 8 UA)",
            DisqueConfig::protoplanetaire(5000, 0.3 * UA, 8.0 * UA, m)
                .avec_lacune_ondulee(0.38, 0.05, 0.92, 0.35)
                .avec_lacune_ondulee(0.66, 0.06, 0.92, 0.35),
            Soleil::new(Vec3::ZERO, 1.6, orange, 0.6), // T Tauri
            2.15,
            // Proto-planètes EN FUSION dans leurs sillons (type PDS 70b) :
            // preset Lave émissif — chaudes, donc visibles à l'échelle du disque.
            vec![
                (0, corps_lacune("Lave", 2.0)),
                (1, corps_lacune("Lave", 2.4)),
            ],
        );
        e(
            "Disque protosolaire (0.1 - 6 UA)",
            DisqueConfig::protosolaire(6000, 0.1 * UA, 6.0 * UA, m),
            Soleil::new(Vec3::ZERO, 1.4, orange, 0.5).avec_jets(), // protoétoile
            2.15,
            Vec::new(),
        );

        // --- Débris autour d'une planète (échelle : rayon planète) ---
        let mut pd = |nom: &str, app: crate::planete::Apparence, nb: usize| {
            let r = app.taille;
            let cfg = DisqueConfig::debris_planetaire(nb, r * 1.9, r * 3.4, 1.0)
                .avec_gm(0.6 * r * r * r); // période lisible quel que soit le rayon
            let ext = cfg.externe;
            let planete = Planete::new(Vec3::ZERO, Vec3::ZERO, r, 1.0, app, Vec::new());
            let mut dq = Disque::new(cfg);
            dq.set_ombre_rayon(r); // la planète projette son ombre sur les débris
            self.cellules.push(Cellule {
                nom: nom.to_string(),
                contenu: Contenu::PlaneteEtDisques(planete, vec![dq]),
                cam_pos: vec3(0.0, ext * 0.8, ext * 2.0),
            });
        };
        pd("Debris planetaire - tellurique", preset_tellurique("Carbone"), 1100);
        pd("Debris planetaire - geante gazeuse", preset_gazeuse("Neptune"), 1400);

        // --- Anneaux planétaires (voile procédural du système unifié) ---
        let mut p = |nom: &str, app: crate::planete::Apparence| {
            let r = app.taille;
            let ext = r * if app.anneau { app.anneau_out } else { 2.0 };
            let planete = Planete::new(Vec3::ZERO, Vec3::ZERO, r, 1.0, app, Vec::new());
            self.cellules.push(Cellule {
                nom: nom.to_string(),
                contenu: Contenu::Planete(planete),
                cam_pos: vec3(0.0, ext * 0.5, ext * 2.6),
            });
        };
        p("Anneaux denses (Saturne)", preset_gazeuse("Saturne"));
        p("Anneau fin (type Uranus)", preset_gazeuse("Anneau monobande (type Uranus)"));
        p("Anneaux en arcs (type Neptune)", preset_gazeuse("Anneaux en arcs (type Neptune)"));
        p("Anneau de debris recent", preset_gazeuse("Anneau de debris recent"));

        // --- Cas exotiques ---

        // J1407b : « super-Saturne » — système d'anneaux de ~0.6 UA de rayon
        // (~200x Saturne), lacune à ~0.4 UA attribuée à une exolune
        // (Kenworthy & Mamajek 2015). Ici : voile immense, lacune ondulée.
        {
            let mut app = preset_gazeuse("Geante annelee massive");
            app.anneau = false; // son propre anneau est remplacé par le géant
            let r = app.taille;
            let cfg = DisqueConfig {
                voile_alpha: 0.55,
                voile_couleur: vec3(0.5, 0.42, 0.3),
                voile_couleur2: vec3(0.72, 0.6, 0.42),
                voile_plateau: 1.0,
                voile_alpha_interne: 0.65,
                voile_bord: 0.02,
                rotation_voile: 0.02,
                ..DisqueConfig::asteroides(0, r * 4.0, r * 24.0, 1.0)
            }
            .avec_lacune_ondulee(0.42, 0.02, 0.95, 0.5) // sillon de l'exolune
            .avec_lacune(0.68, 0.012, 0.8);
            let ext = cfg.externe;
            let planete = Planete::new(Vec3::ZERO, Vec3::ZERO, r, 1.0, app, Vec::new());
            let mut dq = Disque::new(cfg);
            dq.set_ombre_rayon(r);
            self.cellules.push(Cellule {
                nom: "Anneaux geants (type J1407b)".to_string(),
                contenu: Contenu::PlaneteEtDisques(planete, vec![dq]),
                cam_pos: vec3(0.0, ext * 0.45, ext * 1.9),
            });
        }

        // Ceinture très fine à gros blocs : anneau étroit (type F de Saturne)
        // peuplé de fragments massifs, souligné par un voile ténu.
        {
            let app = preset_gazeuse("Uranus");
            let r = app.taille;
            let cfg = DisqueConfig {
                epaisseur: 0.012,
                taille_min: 0.05,
                taille_max: 0.5, // gros blocs (rares : biais u⁴)
                couleur: vec3(0.62, 0.66, 0.72),
                couleur2: vec3(0.7, 0.72, 0.76),
                voile_alpha: 0.22,
                voile_couleur: vec3(0.6, 0.65, 0.72),
                voile_couleur2: vec3(0.6, 0.65, 0.72),
                voile_plateau: 0.0,
                voile_bord: 0.08,
                rotation_voile: 0.05,
                ..DisqueConfig::debris_planetaire(550, r * 2.25, r * 2.55, 1.0)
            }
            .avec_gm(0.6 * r * r * r)
            .avec_bande(0.5, 0.2, 0.5);
            let ext = cfg.externe;
            let planete = Planete::new(Vec3::ZERO, Vec3::ZERO, r, 1.0, app, Vec::new());
            let mut dq = Disque::new(cfg);
            dq.set_ombre_rayon(r);
            self.cellules.push(Cellule {
                nom: "Ceinture fine a gros blocs".to_string(),
                contenu: Contenu::PlaneteEtDisques(planete, vec![dq]),
                cam_pos: vec3(0.0, ext * 0.55, ext * 2.1),
            });
        }

        // Anneaux multiples (façon Planète au Trésor) : plateau nul + 4 bandes
        // dorées bien séparées, aux largeurs variées.
        {
            let mut app = preset_gazeuse("Saturne");
            app.anneau = false;
            let r = app.taille;
            let cfg = DisqueConfig {
                voile_alpha: 0.8,
                voile_couleur: vec3(0.9, 0.74, 0.46),
                voile_couleur2: vec3(0.98, 0.86, 0.6),
                voile_plateau: 0.0,
                voile_bord: 0.015,
                rotation_voile: 0.03,
                ..DisqueConfig::asteroides(0, r * 1.6, r * 3.8, 1.0)
            }
            .avec_bande(0.10, 0.045, 0.9)
            .avec_bande(0.32, 0.06, 0.85)
            .avec_bande(0.58, 0.075, 0.92)
            .avec_bande(0.86, 0.05, 0.8);
            let ext = cfg.externe;
            let planete = Planete::new(Vec3::ZERO, Vec3::ZERO, r, 1.0, app, Vec::new());
            let mut dq = Disque::new(cfg);
            dq.set_ombre_rayon(r);
            self.cellules.push(Cellule {
                nom: "Anneaux multiples (Planete au tresor)".to_string(),
                contenu: Contenu::PlaneteEtDisques(planete, vec![dq]),
                cam_pos: vec3(0.0, ext * 0.5, ext * 2.4),
            });
        }

        self.index = self.index.min(self.cellules.len().saturating_sub(1));
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::G) {
            self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.construire();
        }
        if is_key_pressed(KeyCode::R) {
            crate::planete::vider_cache_materials();
            crate::soleil::vider_cache_materials();
            crate::disque::vider_cache_materials();
            self.construire();
        }
        if is_key_pressed(KeyCode::P) {
            // Cycle NET -> PIXEL -> PIXEL+PALETTE, sur le réglage global — le
            // même que la galerie des planètes. Un interrupteur local n'aurait
            // donné qu'un rendu de plus à tenir d'accord avec les autres.
            crate::reglages::regler_rendu(crate::reglages::mode_rendu().suivant());
        }
        // V : couche voile on/off. **Instrument de mesure** (P.2 bis) — c'est le
        // seul coût GPU plausible restant des disques, et il ne s'isole qu'en
        // l'éteignant à cadrage identique.
        if is_key_pressed(KeyCode::V) {
            crate::disque::set_voiles(!crate::disque::voiles_actifs());
        }
        // Vitesse du temps (les périodes képlériennes réelles sont lentes).
        if is_key_pressed(KeyCode::Up) {
            self.vitesse = (self.vitesse * 2.0).min(64.0);
        }
        if is_key_pressed(KeyCode::Down) {
            self.vitesse = (self.vitesse * 0.5).max(0.125);
        }

        let n = self.cellules.len().max(1);
        // Navigation case par case : molette / PageUp / PageDown.
        let roue = mouse_wheel().1;
        if roue < -0.01 || is_key_pressed(KeyCode::PageDown) {
            self.index = (self.index + 1).min(n - 1);
        }
        if roue > 0.01 || is_key_pressed(KeyCode::PageUp) {
            self.index = self.index.saturating_sub(1);
        }

        clear_background(Color::new(0.02, 0.02, 0.05, 1.0));

        // Un cas par écran : la case occupe toute la hauteur sous la barre.
        let top = 40.0;
        let ch = (screen_height() - top).max(120.0);
        let label_h = 30.0;
        let render_h = ch - label_h;

        // --- Colonne de gauche : choisir un cas d'un clic ---
        //
        // Traitée AVANT le rendu 3D parce qu'elle en fixe la largeur : le
        // viewport doit démarrer au bord de la colonne, sinon elle recouvre une
        // bande de l'image au lieu de la côtoyer.
        let ecran = vec2(screen_width(), screen_height());
        let souris = Vec2::from(mouse_position());
        let clic = is_mouse_button_pressed(MouseButton::Left);
        if clic && crate::ecran::liste::bouton_repli(ecran).contains(souris) {
            self.liste_repliee = !self.liste_repliee;
        } else if clic && !self.liste_repliee {
            let col = crate::ecran::liste::colonne_items_depuis(ecran, top);
            if let Some(i) = crate::ecran::liste::item_sous_curseur(col, n, souris) {
                self.index = i;
            }
        }
        let gauche = crate::ecran::liste::bord_droit(ecran, self.liste_repliee);
        let cw = (screen_width() - gauche).max(1.0);

        // Défilement doux vers la case visée (indépendant du framerate).
        let cible = self.index as f32 * ch;
        let lisse = 1.0 - (-get_frame_time() * 6.0).exp();
        self.scroll += (cible - self.scroll) * lisse;
        if (cible - self.scroll).abs() < 0.3 {
            self.scroll = cible;
        }

        // Le mode vient des réglages — la vue n'a pas d'interrupteur à elle.
        let pixelise = crate::reglages::mode_rendu().pixelise();

        // Filtre pixel : cible basse résolution (recréée si la fenêtre change).
        //
        // **Tout vient de `ecran::pixel`** — facteur de réduction, cible, et
        // surtout le blit, qui est là où s'applique la quantification de
        // palette. Cette vue en avait sa propre copie : elle pixelisait sans
        // jamais quantifier, d'où des débris qui ne « faisaient pas pixel art »
        // alors que les planètes, elles, passaient bien par la palette.
        if pixelise {
            let dims = super::pixel::dimensions();
            if self.rt_dims != dims || self.cible.is_none() {
                self.cible = Some(super::pixel::nouvelle_cible(dims.0, dims.1));
                self.rt_dims = dims;
            }
            set_camera(&Camera2D {
                render_target: self.cible.clone(),
                ..Default::default()
            });
            clear_background(Color::new(0.02, 0.02, 0.05, 1.0));
            set_default_camera();
        }

        let dt = get_frame_time().min(0.05) * self.vitesse;
        let light_pos = vec3(2.5, 1.8, 7.0);

        // --- Phase 3D : seules la case courante et ses voisines défilantes. ---
        let mut labels: Vec<(String, f32)> = Vec::new();
        for (i, cel) in self.cellules.iter_mut().enumerate() {
            let cell_y = top + i as f32 * ch - self.scroll;
            if cell_y + render_h < top || cell_y > screen_height() {
                continue; // hors écran : ni update ni draw
            }

            let pos = cel.cam_pos;
            let mut cam3d = Camera3D {
                position: pos,
                target: Vec3::ZERO,
                up: Vec3::Y,
                fovy: 45.0_f32.to_radians(),
                aspect: Some(cw / render_h),
                viewport: Some((
                    gauche as i32,
                    (screen_height() - (cell_y + render_h)) as i32,
                    cw as i32,
                    render_h as i32,
                )),
                ..Default::default()
            };
            if pixelise {
                let p = super::pixel::PIX as f32;
                cam3d.render_target = self.cible.clone();
                cam3d.viewport = Some((
                    (gauche / p) as i32,
                    ((screen_height() - (cell_y + render_h)) / p) as i32,
                    (cw / p) as i32,
                    (render_h / p) as i32,
                ));
            }
            set_camera(&cam3d);

            // Même recette que la vue orbitale : les billboards s'appuient sur ce
            // repère, et il se dégrade quand la visée frôle Y (voir base_camera).
            let (right, up, forward) = crate::camera::base_camera(Vec3::ZERO - pos);
            let cam = CameraInfo {
                pos,
                right,
                up,
                forward,
                light_pos,
                light_color: Vec3::ONE,
                lights_pos: [light_pos, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
                lights_color: [Vec3::ONE, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            };
            let ech = if pixelise { super::pixel::PIX as f32 } else { 1.0 };
            crate::planete::set_viewport_h(render_h / ech);
            crate::disque::set_viewport_h(render_h / ech); // clamp px des particules
            // Pixel-art : le plus petit débris = 1 pixel de la grille (sinon le
            // clamp de 1.5 px de rendu devient 3 px écran -> débris grossis).
            crate::disque::set_px_min(if pixelise { 1.0 } else { 0.0 });
            match &mut cel.contenu {
                Contenu::EtoileEtDisque(so, dq, corps) => {
                    so.update(dt);
                    dq.update(dt);
                    // **Une seule passe, et le tampon de profondeur ordonne.**
                    //
                    // Le disque était dessiné en deux moitiés autour de l'étoile
                    // (arrière -> étoile -> avant). Cette découpe ouvrait la
                    // couture visible sur les disques proto* : dans chaque
                    // moitié, les cailloux sont peints PUIS recouverts par le
                    // voile de leur moitié. Un caillou de la moitié avant dont
                    // le quad déborde à l'écran sur la moitié arrière est donc
                    // peint APRÈS le voile arrière, et le voile avant ne va pas
                    // le couvrir : il reste nu, opaque et sombre sur le voile
                    // clair. Les cailloux traversant la couture au fil de leur
                    // orbite, la ligne scintillait et changeait de côté.
                    //
                    // La découpe ne servait qu'à se passer de profondeur — or la
                    // cible de rendu en a une (`depth: true`, plus haut), et la
                    // vue système dessine déjà les ceintures en une passe, sans
                    // couture. Elle dégénérait en prime vue de dessus : son plan
                    // devient alors parallèle au disque, et « avant/arrière » ne
                    // veut plus rien dire.
                    //
                    // L'étoile d'abord : son disque écrit la profondeur, donc la
                    // matière passant derrière est rejetée, tandis que sa
                    // couronne, elle, n'écrit rien (voir soleil/materiau.rs) et
                    // laisse voir le disque au travers.
                    so.draw(&cam);
                    for (idx, pl) in corps.iter_mut() {
                        if let Some(pos) = dq.position_lacune(*idx) {
                            pl.corps_mut().position = pos;
                            pl.draw(&cam);
                        }
                    }
                    dq.draw_moitie(&cam, 0.0);
                }
                Contenu::Planete(pl) => {
                    pl.update(dt); // anime le voile de l'anneau
                    pl.draw(&cam);
                }
                Contenu::PlaneteEtDisques(pl, disques) => {
                    pl.update(dt);
                    // Même raison qu'au-dessus : la planète d'abord (elle écrit
                    // la profondeur), puis l'anneau en une passe.
                    pl.draw(&cam);
                    for dq in disques.iter_mut() {
                        dq.update(dt);
                        dq.draw_moitie(&cam, 0.0);
                    }
                }
            }
            labels.push((cel.nom.clone(), cell_y + render_h + 8.0));
        }
        crate::planete::set_viewport_h(0.0);
        crate::disque::set_viewport_h(0.0);
        crate::disque::set_px_min(0.0);

        // --- Phase 2D : blit pixelisé puis textes nets. ---
        set_default_camera();
        if pixelise {
            if let Some(rt) = &self.cible {
                super::pixel::blit(&rt.texture);
            }
        }
        let nom_col = Color::new(0.7, 0.9, 0.8, 1.0);
        for (nom, y) in &labels {
            let tw = crate::police::mesure(nom, 18);
            // Centré sur la zone de rendu, pas sur l'écran : sinon le titre
            // glisse sous la colonne quand elle est déployée.
            crate::police::texte(nom, gauche + (cw - tw) * 0.5, *y, 18.0, nom_col);
        }
        self.dessiner_liste(ecran, top, souris);

        // Barre de titre + état.
        draw_rectangle(0.0, 0.0, screen_width(), top, Color::new(0.02, 0.02, 0.05, 1.0));
        crate::police::texte(
            "CEINTURES & DISQUES   clic liste: aller au cas   molette: suivant/precedent   Haut/Bas: vitesse   G: regenerer   R: shaders   P: rendu (net/pixel/palette)   V: voiles   Echap: menu",
            12.0,
            26.0,
            16.0,
            Color::new(0.6, 0.8, 0.8, 1.0),
        );
        crate::police::texte(
            &format!(
                "cas {}/{}   x{:.2}   {} FPS   rendu: {}   voiles: {}",
                self.index + 1,
                n,
                self.vitesse,
                get_fps(),
                crate::reglages::mode_rendu().nom(),
                if crate::disque::voiles_actifs() { "ON" } else { "OFF" }
            ),
            12.0,
            screen_height() - 10.0,
            16.0,
            Color::new(0.55, 0.75, 0.75, 1.0),
        );
        false
    }
}
