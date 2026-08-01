use super::catalogue::{self, Item, Reglages};
use super::enveloppes;
use super::fils;
use super::pixel::FiltrePixel;
use super::panache::RenduPanache;
use crate::camera::Camera;
use crate::fond::Fond;
use crate::vaisseau::eclairage;
use crate::vaisseau::{
    generer, preset_isv_equipage, Epine, EtatStation, MaillageStation, Ossature, EtatEquipage,
    ParamsStation, Style, ISV_AXE,
};
use macroquad::prelude::*;

/// Catégories de la vue station : chacune ne fait cycler (touche **D**) que ses
/// propres items. Le menu route quatre entrées vers cette même vue.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Categorie {
    /// Catalogue des composants (briques).
    Briques,
    /// Reproductions de vraies petites stations + engins.
    PetitesStations,
    /// Générateur procédural (G / S / 1-4 / O).
    Generateur,
    /// Grandes stations & mégastructures.
    Megastructures,
}

/// Vitesse de rotation de la section d'équipage, en radians par seconde.
/// Choisie pour la lecture — un tour en ~13 s — et non pour la fidélité :
/// l'ISV réel tourne bien plus lentement.
const VITESSE_ROTATION: f32 = 0.48;

/// Durée de la montée (ou de la descente) en régime, en secondes. Plus lente que
/// le repli : une masse chauffe et refroidit lentement, et c'est ce que la durée
/// doit dire. C'est aussi ce qui laisse voir le passage par le rouge sombre des
/// radiateurs et la poussée progressive du panache — sautés l'un et l'autre sur
/// une transition brusque.
const DUREE_ALLUMAGE: f32 = 3.5;

/// Durée du repli (ou du déploiement) complet, en secondes. Assez lent pour
/// qu'on voie la charnière travailler.
const DUREE_REPLI: f32 = 2.5;

/// Rayon des branches de la boussole d'axes (coin bas-droit).
const BOUSSOLE_RAYON: f32 = 34.0;
/// Encombrement total de la boussole, fond et étiquettes comprises.
///
/// C'est **cette** valeur qui décale les boutons vers le haut : les deux se
/// disputent le coin bas-droit, et la faire dériver du rayon évite de les voir se
/// chevaucher au prochain réglage.
const BOUSSOLE_BOITE: f32 = BOUSSOLE_RAYON * 2.0 + 44.0;

impl Categorie {
    /// Table de la catégorie. **Vide pour le générateur**, qui n'est pas
    /// énuméré : son unique item est paramétrique (graine, style, complexité,
    /// ossature) et se rebâtit à chaque touche.
    fn items(self) -> &'static [Item] {
        match self {
            Categorie::Briques => catalogue::BRIQUES,
            Categorie::PetitesStations => catalogue::PETITES_STATIONS,
            Categorie::Megastructures => catalogue::MEGASTRUCTURES,
            Categorie::Generateur => &[],
        }
    }

    /// Nombre d'items à cycler. Dérivé de la table — il n'y a plus de compte
    /// écrit à la main à tenir d'accord avec elle. Le `max(1)` ne sert qu'au
    /// générateur, dont l'unique item n'est pas dans une table.
    fn nb(self) -> usize {
        self.items().len().max(1)
    }

    /// Item courant de la catégorie, ou `None` pour le générateur.
    fn item(self, idx: usize) -> Option<&'static Item> {
        self.items().get(idx % self.nb())
    }

    fn nom(self) -> &'static str {
        match self {
            Categorie::Briques => "BRIQUES",
            Categorie::PetitesStations => "PETITES STATIONS",
            Categorie::Generateur => "GENERATEUR",
            Categorie::Megastructures => "MEGASTRUCTURES",
        }
    }
}

/// Vue station : quatre catégories (menu), chacune cyclée par **D**. Le
/// générateur réagit en plus à G (graine), S (style), 1-4 (complexité), O
/// (ossature). P = ports, N = numéros, M = cuit/immédiat, X = pixel.
pub struct VueStation {
    categorie: Categorie,
    etat: EtatStation,
    titre: String,
    idx: usize,
    params: ParamsStation,
    cam: Camera,
    fond: Fond,
    ports: bool,
    /// Overlay des **enveloppes de collision** (touche E).
    ///
    /// Distinct de `ports` : les ports disent où l'on peut clipser, les
    /// enveloppes disent ce qui **s'oppose** à la pose. Les deux se regardent
    /// souvent ensemble, mais ils répondent à deux questions différentes et
    /// tout afficher d'un coup rend la vue illisible
    /// (`docs/conception/assembleur.md` §8.5).
    enveloppes: bool,
    /// Numérotation des **fils** de charpente (touche F).
    ///
    /// Distincte de `numeros`, qui numérote les *pièces* de l'assemblage : ici
    /// on descend d'un cran, dans la géométrie d'une pièce, pour pouvoir
    /// désigner une barre précise (`conception/assembleur.md` §8.5).
    fils: bool,
    numeros: bool,
    pixel: FiltrePixel,
    /// Géométrie cuite de la station courante (refaite à chaque `charger`).
    maillage: Option<MaillageStation>,
    /// **Moitié tournante**, quand l'item en a une : sur l'ISV, seule la section
    /// d'équipage tourne, pas le vaisseau. On la garde dans un état et un
    /// maillage séparés pour lui appliquer sa propre matrice au moment du rendu.
    ///
    /// `None` = rien de dissocié, et la rotation s'applique alors à l'item
    /// entier (c'est le cas de la brique de démonstration, qui *est* la section).
    tournant: Option<(EtatStation, Option<MaillageStation>)>,
    /// `true` = maillage cuit, `false` = rendu immédiat (bascule M).
    cuit: bool,
    /// Rotation de la section d'équipage sur elle-même. Le bouton qui la pilote
    /// n'a de sens que sur les vues qui en montrent une (brique de démo, ISV
    /// complet) : ailleurs, rien ne tourne, et il est grisé.
    tourne: bool,
    /// Angle courant, en radians. Cumulé tant que `tourne` est vrai.
    angle: f32,
    /// Repli **courant** de la section d'équipage : 0 déployé, 1 replié. C'est
    /// un état du vaisseau (déployé en orbite, replié en transit) et non un
    /// simple réglage d'affichage — d'où l'animation entre les deux.
    repli: f32,
    /// État **visé** de la section. C'est lui qui porte le sens (en transit /
    /// en orbite) ; `repli` n'est que sa valeur animée.
    equipage: EtatEquipage,
    /// **Régime moteur** courant : 0 à l'arrêt, 1 à pleine poussée. Valeur
    /// animée, comme `repli`.
    ///
    /// Un seul nombre pilote **deux** manifestations — les ailes qui rougissent
    /// et le panache qui pousse — parce qu'elles ont une seule et même cause.
    /// Deux réglages séparés auraient permis un vaisseau qui pousse sans
    /// évacuer sa chaleur, ce qui n'existe pas.
    regime: f32,
    /// Régime **visé**. Le bouton le bascule, `regime` le rejoint.
    allume: bool,
    /// Rendu des panaches : un material et ses tampons, gardés d'une frame à
    /// l'autre plutôt que rechargés.
    panaches: RenduPanache,
}

impl VueStation {
    pub fn new(categorie: Categorie) -> Self {
        let params = ParamsStation { graine: 1, complexite: 2, style: Style::Historique, ossature: None };
        let mut cam = Camera::new(12.0);
        cam.yaw = 0.7;
        cam.pitch = 0.3;
        let mut vue = Self {
            categorie,
            etat: EtatStation::Vide,
            titre: String::new(),
            idx: 0,
            params,
            cam,
            fond: Fond::new(400),
            ports: false,
            enveloppes: false,
            fils: false,
            numeros: false,
            pixel: FiltrePixel::new(),
            maillage: None,
            tournant: None,
            cuit: true,
            tourne: false,
            angle: 0.0,
            repli: 0.0,
            regime: 0.0,
            allume: false,
            panaches: RenduPanache::new(),
            equipage: EtatEquipage::Deploye,
        };
        vue.charger();
        vue
    }

    /// (Re)construit l'item courant, son titre et son maillage, **puis recadre**
    /// la caméra dessus.
    fn charger(&mut self) {
        self.rebatir();
        self.cadrer();
    }

    /// (Re)construit l'item courant de la catégorie et son titre — **sans
    /// toucher à la caméra**.
    ///
    /// La séparation d'avec [`Self::charger`] n'est pas cosmétique : la montée
    /// en température refait la géométrie à chaque frame, et recadrer à chaque
    /// fois annulerait le zoom de l'utilisateur en continu pendant toute
    /// l'animation. On ne recadre qu'au **changement d'item**, qui est le seul
    /// moment où le gabarit change vraiment.
    fn rebatir(&mut self) {
        // Dissocié par défaut : seul l'ISV complet renseigne une moitié
        // tournante, et c'est sa `Fabrique` qui le décide, pas un indice.
        self.tournant = None;
        let (etat, titre) = match self.categorie.item(self.idx) {
            Some(item) => {
                let bati = item.batir(self.reglages());
                self.tournant = bati.tournant.map(|section| {
                    let maillage = section.doit_dessiner().map(MaillageStation::cuire);
                    (section, maillage)
                });
                (bati.etat, item.titre())
            }
            // Générateur : le seul item non énuméré, parce qu'il est
            // paramétrique et non catalogué.
            None => (
                generer(&self.params),
                format!(
                    "{} — {} — cplx {} — graine {}",
                    self.params.style.nom(),
                    match self.params.ossature {
                        None => "auto",
                        Some(Ossature::Iss) => "ISS",
                        Some(Ossature::Mir) => "Mir",
                    },
                    self.params.complexite,
                    self.params.graine
                ),
            ),
        };
        self.etat = etat;
        self.titre = format!("[{}]  {}", self.categorie.nom(), titre);
        // Cuisson une fois par item chargé (plus de régénération par frame).
        self.maillage = self.etat.doit_dessiner().map(MaillageStation::cuire);
    }

    /// Les réglages d'animation courants, tels que le catalogue les lit.
    fn reglages(&self) -> Reglages {
        Reglages { repli: self.repli, regime: self.regime }
    }

    /// Recuit ce que le **repli** vient de déformer, et rien de plus.
    ///
    /// Sur l'ISV, recuire tout le vaisseau à chaque frame de l'animation serait
    /// du gaspillage : seule la section d'équipage change, les milliers de
    /// sommets du fret et de la propulsion sont identiques d'une frame à
    /// l'autre. On ne refait donc que la moitié tournante quand elle existe.
    /// La brique de démonstration, elle, *est* la section : `charger` suffit.
    fn recuire_repli(&mut self) {
        match self.epine_courante() {
            Some(epine) => {
                let section = preset_isv_equipage(epine, self.repli);
                let maillage = section.doit_dessiner().map(MaillageStation::cuire);
                self.tournant = Some((section, maillage));
            }
            None => self.charger(),
        }
    }

    /// Épine du vaisseau affiché, s'il s'agit d'un ISV complet.
    ///
    /// La section d'équipage doit être **rebâtie avec la même épine** que la
    /// coque : son alésage de collier se déduit du gabarit, et le reconstruire
    /// avec l'autre variante le décalerait de 3,2 % — assez pour que le collier
    /// morde dans la flèche ou s'en détache.
    fn epine_courante(&self) -> Option<Epine> {
        self.categorie.item(self.idx).and_then(Item::epine)
    }

    fn cadrer(&mut self) {
        // La moitié tournante est prise en compte : sur l'ISV c'est justement
        // l'extrémité la plus éloignée du centre, donc celle qui déborderait.
        let rayon = |e: &EtatStation| e.doit_dessiner().map(|s| s.rayon()).unwrap_or(0.0);
        let demi = rayon(&self.etat)
            .max(self.tournant.as_ref().map_or(0.0, |(e, _)| rayon(e)))
            .max(3.0);
        let demi_fov = 45.0_f32.to_radians() * 0.5;
        self.cam.set_dist((demi + 0.5) / demi_fov.tan() * 1.35);
    }

    /// L'item affiché comporte-t-il une section d'équipage à faire tourner ou à
    /// replier ? Deux vues sont concernées : la brique de démonstration, et l'ISV
    /// complet où la section est montée sur le vaisseau.
    fn rotation_possible(&self) -> bool {
        self.categorie.item(self.idx).is_some_and(Item::rotation)
    }

    /// L'item affiché a-t-il une propulsion à allumer ? Deux vues : la brique
    /// du radiateur méga (qui n'en montre que la chauffe, faute de tuyère) et
    /// l'ISV complet.
    fn allumage_possible(&self) -> bool {
        self.categorie.item(self.idx).is_some_and(Item::allumage)
    }

    /// Recuit ce que le **régime moteur** vient de changer.
    ///
    /// Contrairement à la rotation, qui n'est qu'une matrice, la chaleur est
    /// dans les **couleurs des sommets**, et le panache est de la géométrie qui
    /// n'existe même pas moteur coupé : il faut donc repasser par la cuisson.
    /// Et contrairement au repli, elle porte sur la moitié **fixe** du vaisseau
    /// (les ailes sont sur l'ossature, elles ne tournent pas) — c'est donc
    /// `charger` qu'il faut refaire. Coût borné : la montée dure deux secondes
    /// et s'arrête, là où une rotation recuite tournerait indéfiniment.
    fn recuire_regime(&mut self) {
        self.rebatir();
    }

    /// Axe de rotation de la section, **dans le repère de l'item affiché**.
    ///
    /// Il diffère d'une vue à l'autre : la brique est présentée collier le long
    /// de +Y, tandis que le modèle de l'ISV est couché, son épine sur
    /// [`ISV_AXE`]. Se tromper d'axe fait tourner la section de travers — c'est
    /// exactement l'erreur qu'on avait au premier essai.
    fn axe_rotation(&self) -> Vec3 {
        match self.categorie {
            Categorie::Megastructures => ISV_AXE,
            _ => Vec3::Y,
        }
    }

    /// Un bouton de la vue : rendu normal s'il est actif, **grisé et inerte**
    /// sinon. On le garde visible même éteint — l'utilisateur voit ainsi que la
    /// fonction existe et à quelle vue elle se rattache.
    fn bouton(r: Rect, label: &str, actif: bool, souris: Vec2, clic: bool) -> bool {
        if actif {
            crate::ui::minitel_ligne(r, label, souris);
            return clic && r.contains(souris);
        }
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.06, 0.06, 0.08, 1.0));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.28, 0.28, 0.30, 1.0));
        crate::police::texte(
            label,
            r.x + 10.0,
            r.y + r.h * 0.5 + 6.0,
            20.0,
            Color::new(0.38, 0.38, 0.40, 1.0),
        );
        false
    }

    /// Les deux commandes de la section d'équipage : sa **rotation** et son
    /// **repli**. Toutes deux n'ont de sens que là où une section est affichée —
    /// la brique de démonstration et l'ISV complet ; ailleurs elles sont grisées.
    fn boutons_equipage(&mut self, souris: Vec2, clic: bool) {
        let actif = self.rotation_possible();
        let (x, w, h) = (screen_width() - 250.0, 230.0, 34.0);
        // Empilés **au-dessus** de la boussole, qui occupe tout le bas du coin.
        // `bas` est le bord inférieur de la pile : la boîte de la boussole, plus
        // une marge, sinon le coin d'un bouton vient mordre sur ses étiquettes.
        let bas = screen_height() - BOUSSOLE_BOITE - 8.0;

        let rot = Rect::new(x, bas - h, w, h);
        let label_rot = match (actif, self.tourne) {
            (false, _) => "ROTATION (N/A)",
            (true, true) => "ROTATION: ON",
            (true, false) => "ROTATION: OFF",
        };
        if Self::bouton(rot, label_rot, actif, souris, clic) {
            self.tourne = !self.tourne;
        }

        // Le libellé dit l'**état du vaisseau**, pas l'action : c'est ce qu'on
        // veut lire d'un coup d'œil quand cet état suivra le déplacement du
        // vaisseau (replié en transit, déployé en orbite).
        let rep = Rect::new(x, bas - h - 38.0, w, h);
        let label_rep = match (actif, self.equipage) {
            (false, _) => "EQUIPAGE (N/A)",
            (true, EtatEquipage::Replie) => "EQUIPAGE: REPLIE",
            (true, EtatEquipage::Deploye) => "EQUIPAGE: DEPLOYE",
        };
        if Self::bouton(rep, label_rep, actif, souris, clic) {
            self.equipage = match self.equipage {
                EtatEquipage::Deploye => EtatEquipage::Replie,
                EtatEquipage::Replie => EtatEquipage::Deploye,
            };
        }

        // Chauffe des ailes radiateur. Bouton à part, et actif sur d'autres vues
        // que les deux précédents : la brique du radiateur méga n'a pas de
        // section d'équipage, mais c'est là qu'on juge le dégradé de près.
        let feu = self.allumage_possible();
        let rad = Rect::new(x, bas - h - 76.0, w, h);
        let label_rad = match (feu, self.allume) {
            (false, _) => "PROPULSION (N/A)",
            (true, true) => "PROPULSION: ALLUMEE",
            (true, false) => "PROPULSION: ETEINTE",
        };
        if Self::bouton(rad, label_rad, feu, souris, clic) {
            self.allume = !self.allume;
        }
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::P) {
            self.ports = !self.ports;
        }
        if is_key_pressed(KeyCode::E) {
            self.enveloppes = !self.enveloppes;
        }
        if is_key_pressed(KeyCode::F) {
            self.fils = !self.fils;
        }
        if is_key_pressed(KeyCode::N) {
            self.numeros = !self.numeros;
        }
        if is_key_pressed(KeyCode::M) {
            self.cuit = !self.cuit; // comparer maillage cuit / rendu immédiat
        }
        if is_key_pressed(KeyCode::X) {
            self.pixel.basculer(); // filtre pixel ON/OFF
        }
        if is_key_pressed(KeyCode::D) {
            self.idx += 1;
            self.charger();
        }
        // Réglages du générateur : seulement dans sa catégorie.
        if self.categorie == Categorie::Generateur {
            if is_key_pressed(KeyCode::G) {
                self.params.graine = self.params.graine.wrapping_add(1);
                self.charger();
            }
            if is_key_pressed(KeyCode::S) {
                self.params.style = *Style::TOUS
                    .iter()
                    .cycle()
                    .skip_while(|s| **s != self.params.style)
                    .nth(1)
                    .unwrap();
                self.charger();
            }
            for (touche, c) in [(KeyCode::Key1, 1), (KeyCode::Key2, 2), (KeyCode::Key3, 3), (KeyCode::Key4, 4)] {
                if is_key_pressed(touche) {
                    self.params.complexite = c;
                    self.charger();
                }
            }
            if is_key_pressed(KeyCode::O) {
                self.params.ossature = match self.params.ossature {
                    None => Some(Ossature::Iss),
                    Some(Ossature::Iss) => Some(Ossature::Mir),
                    Some(Ossature::Mir) => None,
                };
                self.charger();
            }
        }

        // Souris : lue une fois par frame, partagée par le bouton de rotation.
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);

        // Avance de la rotation. Elle ne tourne que là où elle a un sens, et
        // l'angle est **conservé** quand on la met en pause : couper la
        // rotation ne doit pas faire sauter la section à zéro.
        if self.tourne && self.rotation_possible() {
            self.angle = (self.angle + get_frame_time() * VITESSE_ROTATION) % std::f32::consts::TAU;
        }

        // Repli : on avance vers la cible et on **recuit** la géométrie, parce
        // que plier des bras déplace des sommets — contrairement à la rotation,
        // qui n'est qu'une matrice.
        let cible = self.equipage.repli();
        if self.rotation_possible() && (self.repli - cible).abs() > 1e-4 {
            let pas = get_frame_time() / DUREE_REPLI;
            self.repli += (cible - self.repli).clamp(-pas, pas);
            if (self.repli - cible).abs() < 1e-4 {
                self.repli = cible;
            }
            self.recuire_repli();
        }

        // Montée (ou descente) en régime, même mécanique que le repli : on
        // avance vers la cible et on recuit. Ailes et panache vivent tous deux
        // dans le maillage fixe — d'où un recuit complet.
        let cible_r = if self.allume { 1.0 } else { 0.0 };
        if self.allumage_possible() && (self.regime - cible_r).abs() > 1e-4 {
            let pas = get_frame_time() / DUREE_ALLUMAGE;
            self.regime += (cible_r - self.regime).clamp(-pas, pas);
            if (self.regime - cible_r).abs() < 1e-4 {
                self.regime = cible_r;
            }
            self.recuire_regime();
        }

        self.cam.input_orbite(false);

        let aspect = screen_width() / screen_height();
        let (cam_info, mut cam3d) = self.cam.construire(Vec3::ZERO, aspect);

        // Couche nette : fond stellaire plein écran.
        set_camera(&cam3d);
        clear_background(BLACK);
        self.fond.draw(&cam_info);
        set_default_camera();

        // Couche station : éclairée, éventuellement pixelisée par-dessus le fond.
        self.pixel.preparer(&mut cam3d);
        set_camera(&cam3d);
        // La rotation est poussée comme **matrice modèle** plutôt que recuite
        // dans le maillage : une matrice contre quelques milliers de sommets à
        // chaque frame. Elle ne s'applique qu'à ce qui tourne réellement —
        // la moitié dissociée si l'item en a une (ISV : le vaisseau reste fixe,
        // seule la section pivote), l'item entier sinon (brique de démo).
        let cuit = self.cuit;
        let axe = self.axe_rotation();
        let angle = self.angle;
        let dessiner = |etat: &EtatStation, maillage: Option<&MaillageStation>, pivote: bool| {
            let Some(station) = etat.doit_dessiner() else { return };
            let pivote = pivote && angle.abs() > 1e-6;
            if pivote {
                unsafe {
                    get_internal_gl()
                        .quad_gl
                        .push_model_matrix(Mat4::from_axis_angle(axe, angle));
                }
            }
            eclairage::avec(cam_info.pos, || match (cuit, maillage) {
                (true, Some(m)) => m.dessiner(), // quelques draw calls
                _ => station.dessiner(),         // un draw call par primitive
            });
            if pivote {
                unsafe {
                    get_internal_gl().quad_gl.pop_model_matrix();
                }
            }
            if self.ports {
                station.dessiner_ports();
            }
            if self.enveloppes {
                enveloppes::station(station, enveloppes::CALME);
            }
        };
        match &self.tournant {
            Some((section, m_section)) => {
                dessiner(&self.etat, self.maillage.as_ref(), false);
                dessiner(section, m_section.as_ref(), true);
            }
            None => dessiner(&self.etat, self.maillage.as_ref(), true),
        }

        // **Panaches**, en dernier et à part : ce sont des rubans en additif, pas
        // de la géométrie cuite. Après la coque, parce qu'ils n'écrivent pas la
        // profondeur et doivent donc se poser par-dessus ce qui est déjà là ;
        // à l'intérieur de la passe pixelisée, pour que le filtre les prenne
        // comme le reste et qu'ils ne flottent pas en net sur un vaisseau
        // pixelisé.
        if let Some(station) = self.etat.doit_dessiner() {
            self.panaches.dessiner(station, &cam_info, get_time() as f32);
        }

        set_default_camera();
        self.pixel.presenter();

        // Numéros de pièce (index d'assemblage) projetés à l'écran, pour pointer
        // les pièces à corriger. L'index = ordre de construction dans le code.
        //
        // Volontairement limité à la moitié **fixe** : les numéros sont projetés
        // depuis les positions cuites, qui ignorent la rotation appliquée au
        // rendu — sur la section tournante ils dériveraient de leur pièce. La
        // brique de démonstration reste le bon endroit pour les numéroter.
        // Numérotation des fils : en 2D, après projection, pour que la coupure
        // du trait garde une largeur en pixels (cf. `fils::COUPURE_PX`).
        if self.fils {
            if let Some(station) = self.etat.doit_dessiner() {
                let vp = macroquad::camera::Camera::matrix(&cam3d);
                fils::station(station, vp, None);
            }
        }

        if self.numeros {
            if let Some(station) = self.etat.doit_dessiner() {
                // Chemin complet : le trait `Camera` de macroquad porte le même
                // nom que la struct `crate::camera::Camera` déjà importée.
                let vp = macroquad::camera::Camera::matrix(&cam3d);
                let (lw, lh) = (screen_width(), screen_height());
                let jaune = Color::new(1.0, 0.85, 0.2, 1.0);
                for (i, piece) in station.pieces().iter().enumerate() {
                    let clip = vp * piece.centre().extend(1.0);
                    if clip.w <= 0.0 {
                        continue; // pièce derrière la caméra
                    }
                    let ndc = clip.truncate() / clip.w;
                    let sx = (ndc.x * 0.5 + 0.5) * lw;
                    let sy = (1.0 - (ndc.y * 0.5 + 0.5)) * lh;
                    crate::police::texte(&format!("{i}"), sx, sy, 20.0, jaune);
                }
            }
        }

        // **Boussole d'axes**, coin bas-droit. Dessinée en 2D après
        // `set_default_camera`, donc jamais pixelisée par le filtre ni cachée par
        // la géométrie : c'est un repère de lecture, il doit rester net.
        //
        // Elle reçoit la base de `cam_info` — les mêmes vecteurs que l'éclairage —
        // pour ne pas pouvoir se désynchroniser de la vue.
        crate::ui::boussole_axes(
            vec2(
                screen_width() - BOUSSOLE_BOITE * 0.5,
                screen_height() - BOUSSOLE_BOITE * 0.5,
            ),
            BOUSSOLE_RAYON,
            cam_info.right,
            cam_info.up,
            cam_info.forward,
        );

        let h = screen_height();
        crate::police::texte(&self.titre, 20.0, h - 24.0, 24.0, WHITE);
        let mode = if self.cuit { "CUIT" } else { "IMMEDIAT" };
        // Coût réel : en cuit, le nombre de lots EST le nombre de draw calls.
        if let Some(m) = &self.maillage {
            // Le coût total sert d'étalon de complexité : c'est l'unité que
            // consomme le budget du générateur.
            let (pieces, cout) = match self.etat.doit_dessiner() {
                Some(s) => (s.nb_pieces(), s.cout_total()),
                None => (0, 0.0),
            };
            crate::police::texte(
                &format!(
                    "{mode} — {} lot(s), {} sommets, {} triangles   |   {pieces} pieces, cout {cout:.0}",
                    m.nb_lots(),
                    m.nb_sommets(),
                    m.nb_triangles()
                ),
                20.0,
                h - 48.0,
                16.0,
                GRAY,
            );
        }
        // **Bouton de rotation**, actif seulement là où quelque chose tourne
        // vraiment (la brique d'équipage). Ailleurs il reste visible mais
        // **grisé** : ça vaut mieux que de le faire disparaître, l'utilisateur
        // voit que la fonction existe et à quoi elle se rattache.
        self.boutons_equipage(m, clic);

        let etat_ports = if self.ports { "ON" } else { "OFF" };
        let etat_env = if self.enveloppes { "ON" } else { "OFF" };
        let etat_fils = if self.fils { "ON" } else { "OFF" };
        let etat_num = if self.numeros { "ON" } else { "OFF" };
        let etat_pix = if self.pixel.actif { "ON" } else { "OFF" };
        // Les réglages du générateur ne sont rappelés que dans sa catégorie.
        let gen = if self.categorie == Categorie::Generateur {
            "1-4: complexite   O: ossature   G: graine   S: style   "
        } else {
            ""
        };
        crate::police::texte(
            &format!("{gen}D: suivant   P: ports ({etat_ports})   E: enveloppes ({etat_env})   F: fils ({etat_fils})   N: numeros ({etat_num})   X: pixel ({etat_pix})   M: rendu ({mode})   Echap: menu"),
            12.0,
            24.0,
            17.0,
            WHITE,
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les quatre catégories, pour les balayer toutes.
    const CATEGORIES: [Categorie; 4] = [
        Categorie::Briques,
        Categorie::PetitesStations,
        Categorie::Generateur,
        Categorie::Megastructures,
    ];

    // Le cyclage (touche **D**) incrémente `idx` sans borne et retombe sur la
    // table par un modulo. Il doit donc **visiter chaque item exactement une
    // fois** par tour : un décalage d'un cran rendrait une brique inatteignable
    // et en montrerait une autre deux fois — précisément ce que l'attrape-tout
    // `_ =>` faisait avant, et qui ne se voyait qu'à l'écran.
    #[test]
    fn un_tour_de_cyclage_visite_chaque_item_une_fois() {
        for cat in CATEGORIES {
            let vus: Vec<&str> = (0..cat.nb())
                .map(|idx| cat.item(idx).map_or("<generateur>", |it| it.libelle))
                .collect();
            assert_eq!(vus.len(), cat.nb(), "{}", cat.nom());
            for (i, a) in vus.iter().enumerate() {
                for b in vus.iter().skip(i + 1) {
                    assert_ne!(a, b, "{} : item vu deux fois en un tour", cat.nom());
                }
            }
            // Et le tour suivant redonne le premier : le cyclage boucle.
            assert_eq!(
                cat.item(cat.nb()).map(|it| it.libelle),
                cat.item(0).map(|it| it.libelle),
                "{} : le cyclage ne reboucle pas",
                cat.nom()
            );
        }
    }

    // **Le générateur n'est pas catalogué**, et c'est ce qui fait tenir le
    // `max(1)` de `nb()` : son unique item est paramétrique (graine, style,
    // complexité, ossature) et se rebâtit à chaque touche, donc aucune table ne
    // peut le décrire. `rebatir` s'en remet à `item(idx) == None` pour prendre
    // cette branche — lui donner une table le ferait basculer silencieusement
    // sur un item figé, et les touches G/S/1-4/O n'auraient plus d'effet.
    #[test]
    fn seul_le_generateur_nest_pas_catalogue() {
        for cat in CATEGORIES {
            let catalogue = cat.item(0).is_some();
            assert_eq!(
                catalogue,
                cat != Categorie::Generateur,
                "{} : catalogué et paramétrique ne se cumulent pas",
                cat.nom()
            );
            assert!(cat.nb() >= 1, "{} : au moins un item à afficher", cat.nom());
        }
    }
}
