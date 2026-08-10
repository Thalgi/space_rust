//! Écran d'assemblage (VAB) — `docs/conception/assembleur.md` §8 et §10.
//!
//! **Lot 4.1 : le squelette.** Zones, caméra orbitale, boussole, bandeau, et un
//! chantier qu'on regarde tourner. Rien ne se pose encore : la palette (L4.2) et
//! le clic (L4.3) viennent après. Le but de cette étape n'est pas d'être
//! utilisable, c'est que le cadre existe et se juge à l'œil.
//!
//! Le modèle est fini depuis le Lot 3 : `Chantier` sait poser, retirer,
//! annuler, refaire, désigner un port et une pièce. Cet écran n'ajoute aucune
//! règle — il **branche**.

use crate::astre::CameraInfo;
use crate::camera::Camera;
use crate::ecran::palette::{self, Entree};
use crate::ecran::{clic, designation, enveloppes, pixel::FiltrePixel, ports};
use crate::fond::Fond;
use crate::ui;
use crate::vaisseau::{eclairage, sauvegarde, Chantier, Composant, EtatStation, MaillageStation};
use macroquad::prelude::*;
use std::collections::HashSet;

/// Largeur de la colonne de palette. Fixe : §8.1 — « pièce d'abord » demande
/// une palette **permanente**, donc une colonne réservée, pas un menu qui
/// apparaît et disparaît.
const PALETTE_L: f32 = 220.0;

/// Hauteur du bandeau bas (compteurs, undo/redo, bascules).
const BANDEAU_H: f32 = 56.0;

/// Rayon de la boussole, coin bas-droit — même valeur que `VueStation`, pour
/// que l'œil la retrouve au même endroit d'un écran à l'autre.
const BOUSSOLE_RAYON: f32 = 34.0;
const BOUSSOLE_MARGE: f32 = 30.0;

/// Recul initial de la caméra. Le chantier démarre vide : sans repère on ne
/// verrait rien du tout, d'où la grille de §« chantier vide » ci-dessous.
const RECUL: f32 = 12.0;

// Les trois couleurs de §8.4, plus le neutre du repos.
//
// **Vert et rouge, et c'est un choix contraint.** Ailleurs dans le projet
// (`ui::COULEUR_ACTIF`) la paire rouge/vert a été écartée : c'est la pire pour
// un deutéranope. Ici elle est imposée par la convention — vert « ça passe »,
// rouge « ça bloque » — donc la **taille du marqueur** double le code couleur :
// une bille posable est nettement plus grosse qu'une encombrée, elle-même plus
// grosse qu'une éteinte. Le canal qui survit au daltonisme est la luminance et
// la forme, jamais la teinte seule.
const COL_POSABLE: Color = Color::new(0.30, 1.0, 0.45, 1.0);
const COL_ENCOMBRE: Color = Color::new(1.0, 0.32, 0.24, 1.0);
const COL_ETEINT: Color = Color::new(0.28, 0.30, 0.34, 1.0);
const COL_NEUTRE: Color = Color::new(0.65, 0.72, 0.80, 1.0);
/// Surlignage du sous-arbre sélectionné. Ni vert ni rouge : ces deux-là ont déjà
/// un sens ferme sur cet écran (§8.4), et un troisième message dans les mêmes
/// teintes se lirait de travers.
const COL_SELECTION: Color = Color::new(1.0, 0.85, 0.25, 1.0);

/// Les trois zones de l'écran (§8.2).
///
/// **Elles ne servent pas qu'à dessiner.** Savoir de quelle zone relève un point
/// est une *décision* — la caméra ne doit pas pivoter quand on glisse sur la
/// palette, et un clic dans la palette ne doit pas poser une pièce dans le vide.
/// C'est à ce titre que ça se teste, quand la disposition elle-même ne se teste
/// pas (§6.6, §10.9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zones {
    pub palette: Rect,
    pub vue3d: Rect,
    pub bandeau: Rect,
}

/// Zone dont relève un point de l'écran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Palette,
    Vue3d,
    Bandeau,
}

impl Zones {
    /// Découpe un écran de taille `taille`. Le bandeau prend toute la largeur en
    /// bas ; au-dessus, la palette à gauche et la vue 3D sur le reste.
    pub fn pour(taille: Vec2) -> Zones {
        let h_haut = (taille.y - BANDEAU_H).max(0.0);
        let l_palette = PALETTE_L.min(taille.x);
        Zones {
            palette: Rect::new(0.0, 0.0, l_palette, h_haut),
            vue3d: Rect::new(l_palette, 0.0, (taille.x - l_palette).max(0.0), h_haut),
            bandeau: Rect::new(0.0, h_haut, taille.x, taille.y - h_haut),
        }
    }

    /// À quelle zone appartient `p`.
    ///
    /// L'ordre des tests est **indifférent** : les trois rectangles sont
    /// disjoints, `les_zones_pavent_l_ecran` l'exige. (Une première rédaction
    /// prétendait que le bandeau « l'emportait » sur les autres — faux, et le
    /// red-check l'a montré en laissant l'ordre inversé passer au vert.)
    ///
    /// La vue 3D sert de **repli** : un point hors de l'écran — souris sortie
    /// de la fenêtre pendant un glisser — continue de faire orbiter la caméra,
    /// ce qui est le comportement voulu.
    pub fn zone(&self, p: Vec2) -> Zone {
        if self.bandeau.contains(p) {
            Zone::Bandeau
        } else if self.palette.contains(p) {
            Zone::Palette
        } else {
            Zone::Vue3d
        }
    }

    /// Vrai si le point capte le glisser de caméra. La vue 3D orbite ; la
    /// palette et le bandeau la laissent tranquille — sans quoi choisir une
    /// pièce ferait pivoter le vaisseau derrière (le défaut que `VueStation`
    /// corrige avec `liste::sur_la_liste`).
    pub fn hors_de_la_vue(&self, p: Vec2) -> bool {
        self.zone(p) != Zone::Vue3d
    }
}

/// Ce qu'un clic dans la palette déclenche. Décidé par `dessiner_palette`,
/// appliqué par `frame` : la méthode de dessin ne tient qu'un `&self`.
enum Action {
    /// Plier / déplier la catégorie d'indice donné.
    Plier(usize),
    /// Prendre l'entrée en main (ou la poser, chantier vide — §10.1).
    Choisir(usize),
}

/// Les états de l'interaction (§8.3). « Pièce sélectionnée » arrive en L4.5.
enum Interaction {
    /// Le vaisseau seul.
    Repos,
    /// Une pièce est en main : cliquer un port compatible la pose.
    /// L'indice pointe dans `entrees`.
    EnMain(usize),
    /// Une pièce **posée** est sélectionnée : elle et son sous-arbre sont
    /// surlignés, Suppr les retire. Identifiant **stable** de pièce, jamais une
    /// position : `retirer` en fait disparaître au milieu de la séquence.
    Selectionnee(u64),
    /// Un sous-arbre **déjà posé** est en main (§10.10, touche M).
    ///
    /// Ce n'est **pas un quatrième état** au sens de §8.3 : c'est « pièce en
    /// main » dont la source est le vaisseau au lieu de la palette. Les trois
    /// couleurs, le fantôme, le cycle de montage et la discrimination
    /// clic/glissé servent tels quels — seule l'issue du clic diffère.
    Deplacee(u64),
}

/// Écran d'assemblage.
pub struct VueAssembleur {
    /// Le modèle. `avec_historique` : le bandeau porte UNDO/REDO (§8.2), et
    /// l'historique est en opt-in parce qu'il coûte une copie par mutation —
    /// hors sujet pour le générateur, indispensable ici.
    chantier: Chantier,
    /// Station **dérivée** du chantier, refaite à chaque modification et non à
    /// chaque image (§10.3). `Chantier::apercu` est la seule source.
    apercu: EtatStation,
    /// Géométrie cuite de l'aperçu. Même cycle de vie.
    maillage: Option<MaillageStation>,
    cam: Camera,
    fond: Fond,
    /// Ports visibles (touche P). Ils sont l'objet même de l'écran, donc
    /// **allumés par défaut** — contrairement à `VueStation`, où l'on vient
    /// d'abord regarder une station finie.
    ports: bool,
    /// Enveloppes de collision (touche E) — §8.5. Répondra à « pourquoi ce port
    /// est-il rouge ? » quand les couleurs arriveront (L4.4).
    enveloppes: bool,
    pixel: FiltrePixel,
    /// Contenu de la palette. Bâti **une fois** : il ne dépend pas du chantier.
    entrees: Vec<Entree>,
    /// Grisage, parallèle à `entrees` : dépend du chantier, donc refait par
    /// `rafraichir` — jamais par image (§10.3).
    posables: Vec<bool>,
    /// Catégories repliées (§8.2). Repliées d'entrée : trente entrées d'un coup
    /// dans une colonne de 220 px seraient illisibles, et c'est justement ce que
    /// le pliage existe pour éviter.
    replie: Vec<bool>,
    /// Décalage de défilement de la colonne, en pixels.
    defilement: f32,
    /// État de l'interaction (§8.3).
    interaction: Interaction,
    /// Discrimination clic / glissé (§10.4) : le même bouton fait tourner la
    /// caméra et pose une pièce.
    clic: clic::Discriminateur,
    /// Port libre sous le curseur, recalculé **chaque image** — il suit la
    /// souris, comme le fantôme (§10.3).
    port_vise: Option<u64>,
    /// Port de montage retenu sur la pièce en main (§10.5). Cyclé par **Tab**.
    montage: usize,
    /// État vert/rouge/éteint de chaque port libre (§8.4).
    ///
    /// **Le calcul cher de l'écran** : un `peut_poser` par port, chacun testant
    /// la collision contre toutes les pièces. Gardé d'une image à l'autre et
    /// jeté sur `cle_ports` (§10.3).
    etats_ports: Vec<(u64, ports::Etat)>,
    /// Ce pour quoi `etats_ports` a été calculé : version du chantier, pièce en
    /// main, montage. **Tout ce dont le résultat dépend**, et rien d'autre — un
    /// champ oublié ici, et l'écran peint des couleurs périmées.
    cle_ports: Option<(u64, (u8, u64), usize)>,
    /// Sous-arbre de la pièce sélectionnée. Même logique de cache que
    /// `etats_ports` : il ne dépend que du chantier et de la sélection.
    surligne: HashSet<u64>,
    cle_surligne: Option<(u64, u64)>,
    /// Emplacement de sauvegarde courant, 1 à 9 (L5.1).
    ///
    /// **Des emplacements numérotés plutôt qu'un nom saisi** : le projet n'a de
    /// champ de saisie nulle part, et §10.6 a déjà écarté les boîtes de dialogue.
    /// L'idiome de la sauvegarde rapide donne le service sans rien inventer.
    emplacement: usize,
    /// Emplacements déjà occupés, relevés au disque. **Pas par image** :
    /// `read_dir` à 60 Hz pour afficher neuf caractères serait absurde. Relu à
    /// l'entrée et après chaque écriture, les seuls moments où ça change.
    occupes: [bool; 9],
    /// Dernier message de sauvegarde/chargement, et depuis quand. Une action
    /// disque qui ne dit rien laisse croire qu'elle n'a pas eu lieu.
    message: Option<(String, f64)>,
    /// De combien il manque pour que la pose visée passe (§8.5). Rempli par
    /// l'overlay au moment du dessin, écrit par le bandeau juste après.
    manque: Option<f32>,
    /// **Roulis** de la pièce en main, en radians (§11.3). Pour une pièce déjà
    /// posée, l'angle vit dans le `Chantier` ; celui-ci ne sert qu'à la pose.
    roulis: f32,
}

/// Pas de roulis : 24 crans sur le tour.
///
/// **Quantifié, et pas continu.** Aligner deux pièces à l'œil sur un angle libre
/// est pénible ; 15° tombe juste sur les fractions qu'on veut réellement (un
/// quart de tour en 6 crans, un demi en 12). Le continu viendra avec le gizmo,
/// s'il se révèle nécessaire.
const PAS_ROULIS: f32 = std::f32::consts::TAU / 24.0;

/// Quels emplacements portent un vaisseau.
fn emplacements_occupes() -> [bool; 9] {
    let noms = sauvegarde::liste();
    let mut v = [false; 9];
    for (i, occupe) in v.iter_mut().enumerate() {
        *occupe = noms.iter().any(|n| n == &format!("vaisseau_{}", i + 1));
    }
    v
}

impl VueAssembleur {
    pub fn new() -> Self {
        let mut cam = Camera::new(RECUL);
        cam.yaw = 0.7;
        cam.pitch = 0.3;
        let chantier = Chantier::new().avec_historique();
        let apercu = chantier.apercu();
        let entrees = palette::contenu();
        let posables = palette::grisage(&chantier, &entrees);
        // **La première catégorie s'ouvre d'entrée.** Tout replier protégeait la
        // lisibilité (§L4.2) mais ouvrait l'écran sur huit en-têtes et **rien de
        // cliquable** : l'action principale était invisible au premier regard.
        // Une ouverte, sept fermées, montre le mécanisme ET donne de quoi
        // commencer.
        let mut replie = vec![true; palette::categories(&entrees).len()];
        if let Some(premier) = replie.first_mut() {
            *premier = false;
        }
        Self {
            chantier,
            apercu,
            maillage: None,
            cam,
            fond: Fond::new(900),
            ports: true,
            enveloppes: false,
            pixel: FiltrePixel::new(),
            entrees,
            posables,
            replie,
            defilement: 0.0,
            interaction: Interaction::Repos,
            clic: clic::Discriminateur::default(),
            port_vise: None,
            montage: 0,
            etats_ports: Vec::new(),
            cle_ports: None,
            surligne: HashSet::new(),
            cle_surligne: None,
            emplacement: 1,
            occupes: emplacements_occupes(),
            message: None,
            manque: None,
            roulis: 0.0,
        }
    }

    /// Nom de fichier de l'emplacement courant.
    fn nom_emplacement(&self) -> String {
        format!("vaisseau_{}", self.emplacement)
    }

    /// Écrit le chantier courant (L5.1). **L'échec se dit** : contrairement aux
    /// réglages, qui retombent en silence sur leurs défauts parce qu'un réglage
    /// perdu n'est rien, c'est ici le travail de l'utilisateur.
    fn sauver(&mut self) {
        let nom = self.nom_emplacement();
        let texte = match sauvegarde::enregistrer(&self.chantier, &nom) {
            Ok(chemin) => format!("ENREGISTRE : {}", chemin.display()),
            Err(e) => format!("ECHEC D'ECRITURE : {e}"),
        };
        self.occupes = emplacements_occupes();
        self.message = Some((texte, get_time()));
    }

    /// Relit l'emplacement courant et **remplace** le chantier.
    ///
    /// Tout ce qui dérivait de l'ancien doit tomber : l'aperçu, le maillage, le
    /// grisage, les caches de ports et de surlignage, et l'interaction — une
    /// sélection ou une pièce en main désignerait sinon des identifiants d'un
    /// vaisseau qui n'existe plus.
    fn recharger(&mut self) {
        let nom = self.nom_emplacement();
        match sauvegarde::charger(&nom) {
            Some(ch) => {
                self.chantier = ch;
                self.interaction = Interaction::Repos;
                self.cle_ports = None;
                self.cle_surligne = None;
                self.surligne.clear();
                self.etats_ports.clear();
                self.rafraichir();
                self.message = Some((format!("CHARGE : {nom}"), get_time()));
            }
            None => {
                self.message = Some((format!("AUCUN VAISSEAU EN {}", self.emplacement), get_time()));
            }
        }
    }

    /// Sous-arbre surligné, recalculé seulement si le chantier ou la sélection
    /// ont bougé — même discipline de cache que `maj_etats_ports`.
    fn maj_surligne(&mut self) {
        // Surligné aussi pendant un déplacement : c'est précisément le bloc qui
        // va bouger, et le voir avant de cliquer est tout l'intérêt.
        let (Interaction::Selectionnee(id) | Interaction::Deplacee(id)) = self.interaction else {
            self.surligne.clear();
            self.cle_surligne = None;
            return;
        };
        let cle = (self.chantier.version(), id);
        if self.cle_surligne == Some(cle) {
            return;
        }
        self.surligne = self.chantier.sous_arbre(id);
        self.cle_surligne = Some(cle);
        // Une pièce retirée par un `annuler` laisse une sélection orpheline :
        // `sous_arbre` rend alors l'ensemble vide, et on repasse au repos plutôt
        // que de garder une sélection qui ne désigne plus rien.
        if self.surligne.is_empty() {
            self.interaction = Interaction::Repos;
            self.cle_surligne = None;
        }
    }

    /// Recalcule l'état des ports **si et seulement si** ce dont il dépend a
    /// changé (§10.3). La clé porte la version du chantier — qui avance à toute
    /// mutation, annulation comprise —, la pièce en main et le montage.
    fn maj_etats_ports(&mut self) {
        let Some(cle_main) = self.cle_en_main() else {
            // Rien en main : pas de couleurs, et le cache se vide pour que la
            // prochaine prise en main le recalcule.
            self.etats_ports.clear();
            self.cle_ports = None;
            return;
        };
        let cle = (self.chantier.version(), cle_main, self.montage);
        if self.cle_ports == Some(cle) {
            return;
        }
        self.etats_ports = match self.interaction {
            // Pièce neuve : « cette pièce tiendrait-elle ici ? »
            Interaction::EnMain(i) => ports::etats(&self.chantier, &self.entrees[i].comp, self.montage),
            // Bloc déplacé : la question n'est pas la même — c'est tout un
            // sous-arbre qui bouge, et son ancienne place se libère au passage.
            // `peut_poser` répondrait à côté (§10.10).
            Interaction::Deplacee(id) => ports::etats_deplacement(&self.chantier, id, self.montage),
            _ => Vec::new(),
        };
        self.cle_ports = Some(cle);
    }

    /// Clé de cache de ce qui est en main : la **source** compte autant que
    /// l'identifiant, sinon l'entrée n°3 de la palette et la pièce n°3 du
    /// vaisseau porteraient la même clé.
    fn cle_en_main(&self) -> Option<(u8, u64)> {
        match self.interaction {
            Interaction::EnMain(i) => Some((0, i as u64)),
            Interaction::Deplacee(id) => Some((1, id)),
            _ => None,
        }
    }

    /// Le composant en main, quelle qu'en soit la source — palette (L4.3) ou
    /// vaisseau (L4.6).
    fn comp_en_main(&self) -> Option<Composant> {
        match self.interaction {
            Interaction::EnMain(i) => Some(self.entrees[i].comp.clone()),
            Interaction::Deplacee(id) => self.chantier.piece(id).map(|p| p.composant.clone()),
            _ => None,
        }
    }

    /// Nombre de ports de montage de la pièce en main — borne du cycle §10.5.
    fn nb_montages(&self) -> usize {
        self.comp_en_main().map(|c| c.ports().len()).unwrap_or(0)
    }

    /// Un clic achevé dans la vue 3D. **Deux gestes, jamais ambigus** :
    ///
    /// - pièce en main *et* port visé → on pose ;
    /// - sinon → on sélectionne la pièce sous le rayon, ou l'on revient au repos
    ///   si le clic tombe dans le vide (§8.3 : « clic ailleurs → repos »).
    ///
    /// Le port l'emporte sur la pièce quand les deux sont sous le curseur : un
    /// port est un point qu'on vise délibérément, et il est **sur** une pièce
    /// par construction — l'ordre inverse rendrait tout port inatteignable.
    fn clic_dans_la_vue(&mut self, p: Vec2, ecran: Vec2, cam: &CameraInfo, aspect: f32) {
        // **Le montage employé est celui que le fantôme a montré.** Le résoudre
        // ici, au clic, poserait dans une orientation que rien n'avait annoncée.
        let m = self.montage;
        match self.interaction {
            Interaction::EnMain(i) if self.port_vise.is_some() => {
                let port = self.port_vise.unwrap();
                let comp = self.entrees[i].comp.clone();
                let r = self.roulis;
                self.muter(|ch| ch.poser_roule(port, comp, m, r));
                return;
            }
            Interaction::Deplacee(id) if self.port_vise.is_some() => {
                let port = self.port_vise.unwrap();
                // Le déplacement réussi rend la main : la pièce est posée, on la
                // garde sélectionnée pour pouvoir enchaîner.
                self.muter(|ch| ch.deplacer(id, port, m));
                self.interaction = Interaction::Selectionnee(id);
                return;
            }
            _ => {}
        }
        let dir = crate::camera::rayon_ecran(p, ecran, cam, aspect);
        self.interaction = match self.chantier.piece_sous_rayon(cam.pos, dir) {
            Some(id) => Interaction::Selectionnee(id),
            None => Interaction::Repos,
        };
    }

    /// **La porte unique des mutations** (§10.3). Tout ce qui change le chantier
    /// — poser, retirer, annuler, refaire — passe par ici et rafraîchit ce qui
    /// en dérive. Deux chemins, et le jour où l'un oublie d'invalider, l'écran
    /// affiche un vaisseau périmé : la famille de défaut que ce projet traque
    /// depuis le Lot 1, transposée à l'affichage.
    fn muter(&mut self, f: impl FnOnce(&mut Chantier) -> bool) {
        if f(&mut self.chantier) {
            self.rafraichir();
        }
    }

    /// Suite d'un clic dans la palette : prendre la pièce en main — ou la poser
    /// **d'emblée** si le chantier est vide (§10.1 : la première pièce se pose
    /// à l'origine par `racine`, il n'y a aucun port à viser, donc cliquer une
    /// pièce *est* la poser). Sans cette règle, le premier clic ne ferait rien
    /// et rien n'expliquerait pourquoi.
    fn choisir(&mut self, i: usize) {
        if !self.posables.get(i).copied().unwrap_or(false) {
            return; // entrée grisée : elle ne se pose nulle part
        }
        if self.chantier.nb_pieces() == 0 {
            let comp = self.entrees[i].comp.clone();
            self.muter(|ch| ch.racine(comp));
            return;
        }
        self.interaction = Interaction::EnMain(i);
        // §10.5 : le premier montage qui mène quelque part, pas l'indice 0 —
        // prendre une pièce en main et ne voir aucun port allumé alors qu'un
        // autre montage en aurait allumé serait une impasse muette.
        self.montage = designation::premier_montage(&self.chantier, &self.entrees[i].comp);
        self.roulis = 0.0; // une nouvelle pièce part droite
    }

    /// Recalcule ce qui dérive du chantier. **Un seul endroit** : appelé après
    /// toute mutation, jamais depuis le dessin. C'est ce qui garantit que
    /// l'aperçu et son maillage ne peuvent pas diverger du modèle.
    fn rafraichir(&mut self) {
        self.apercu = self.chantier.apercu();
        self.maillage = self.apercu.doit_dessiner().map(MaillageStation::cuire);
        // Le grisage dépend des ports libres : il change à chaque pose comme à
        // chaque retrait, et **seulement** là (§10.2, §10.3).
        self.posables = palette::grisage(&self.chantier, &self.entrees);
    }

    /// Zones de l'écran courant.
    fn zones(&self) -> Zones {
        Zones::pour(vec2(screen_width(), screen_height()))
    }

    /// Une image. `true` = sortir vers l'accueil.
    pub fn frame(&mut self) -> bool {
        // **Échap repose d'abord la pièce en main** (§8.3), et ne quitte l'écran
        // qu'au repos. Sortir directement ferait perdre le vaisseau au geste qui
        // sert le plus souvent à annuler une prise en main.
        //
        // ⚠️ Quitter détruit tout de même le chantier (§10.7 b) : arbitrage rendu
        // le 2026-08-09 — on assume la perte jusqu'à la sauvegarde du Lot 5.
        if is_key_pressed(KeyCode::Escape) {
            if matches!(self.interaction, Interaction::Repos) {
                return true;
            }
            self.interaction = Interaction::Repos;
        }
        // **Suppr retire la pièce sélectionnée et tout son sous-arbre.**
        // Pas de confirmation, même sur la racine — §10.6 : le retrait est
        // annulable, et le projet n'a de boîte de dialogue nulle part.
        if is_key_pressed(KeyCode::Delete) {
            if let Interaction::Selectionnee(id) = self.interaction {
                self.muter(|ch| ch.retirer(id));
                self.interaction = Interaction::Repos;
            }
        }
        // **M lève le sous-arbre sélectionné** (§10.10). L'écran retombe dans
        // « pièce en main » — mêmes couleurs, même fantôme, même cycle de
        // montage — mais la pièce vient du vaisseau, pas de la palette.
        if is_key_pressed(KeyCode::M) {
            if let Interaction::Selectionnee(id) = self.interaction {
                if let Some(p) = self.chantier.piece(id) {
                    let comp = p.composant.clone();
                    self.montage = designation::premier_montage(&self.chantier, &comp);
                    self.interaction = Interaction::Deplacee(id);
                }
            }
        }
        if is_key_pressed(KeyCode::P) {
            self.ports = !self.ports;
        }
        if is_key_pressed(KeyCode::E) {
            self.enveloppes = !self.enveloppes;
        }
        if is_key_pressed(KeyCode::X) {
            self.pixel.basculer();
        }
        // **Cycle de montage** (§10.5). Il n'a de sens qu'avec le fantôme, qui
        // montre ce que le changement fait — d'où son arrivée ici et pas en
        // L4.3, où il aurait été un réglage aveugle.
        if is_key_pressed(KeyCode::Tab) && self.nb_montages() > 0 {
            self.montage = (self.montage + 1) % self.nb_montages();
        }
        // **ROTATION** (§11.3, L7.2). R tourne d'un cran, Maj+R en sens inverse.
        //
        // Au clavier, et pas au gizmo : une pression est **déjà** un geste
        // atomique, donc une seule entrée d'historique. Toute la machinerie de
        // « geste continu » que §11.6 réclamait ne sert qu'aux glissés — la
        // rotation, elle, n'en a pas besoin et peut arriver tout de suite.
        if is_key_pressed(KeyCode::R) {
            let sens = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                -1.0
            } else {
                1.0
            };
            match self.interaction {
                // Pièce en main : on règle l'angle **avant** de poser, et le
                // fantôme le montre. Rien n'est muté.
                Interaction::EnMain(_) => self.roulis += sens * PAS_ROULIS,
                // Pièce posée : le chantier tourne, en un coup d'historique.
                Interaction::Selectionnee(id) => {
                    let vise = self.chantier.roulis(id) + sens * PAS_ROULIS;
                    self.muter(|ch| ch.pivoter(id, vise));
                }
                _ => {}
            }
        }
        // Annuler / refaire au clavier dès maintenant : l'historique existe
        // (L2.3) et rien ne justifie d'attendre les boutons pour le brancher.
        // `rafraichir` est appelé parce que le modèle a bougé.
        // ---- Disque (L5.1) : emplacements numérotés, façon sauvegarde rapide.
        for (i, touche) in [
            KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5,
            KeyCode::Key6, KeyCode::Key7, KeyCode::Key8, KeyCode::Key9,
        ]
        .into_iter()
        .enumerate()
        {
            if is_key_pressed(touche) {
                self.emplacement = i + 1;
            }
        }
        if is_key_pressed(KeyCode::F5) {
            self.sauver();
        }
        if is_key_pressed(KeyCode::F9) {
            self.recharger();
        }
        if is_key_pressed(KeyCode::Z) {
            self.muter(|ch| ch.annuler());
        }
        if is_key_pressed(KeyCode::Y) {
            self.muter(|ch| ch.refaire());
        }

        let zones = self.zones();
        let souris = vec2(mouse_position().0, mouse_position().1);
        // La palette et le bandeau mangent le glisser (voir `hors_de_la_vue`).
        self.cam.input_orbite(zones.hors_de_la_vue(souris));
        // La molette défile la colonne quand le curseur est dessus, et zoome la
        // vue sinon. `input_orbite` a déjà consommé le zoom : on ne fait ici que
        // le second usage, sur la seule zone qui le réclame.
        if zones.zone(souris) == Zone::Palette {
            let (_, roue) = mouse_wheel();
            if roue != 0.0 {
                self.defilement = (self.defilement - roue * 24.0).max(0.0);
            }
        }

        let aspect = screen_width() / screen_height();
        let (cam_info, mut cam3d) = self.cam.construire(Vec3::ZERO, aspect);

        // Ce que vise le curseur, et ce que le relâchement vaut. Les deux se
        // calculent **avant** de dessiner : le dessin ne décide de rien (§10.9).
        let ecran = vec2(screen_width(), screen_height());
        self.port_vise = match self.interaction {
            Interaction::EnMain(_) | Interaction::Deplacee(_)
                if zones.zone(souris) == Zone::Vue3d =>
            {
                designation::port_sous_curseur(
                    self.chantier.libres(),
                    souris,
                    ecran,
                    &cam_info,
                    aspect,
                    designation::ACCROCHE_PIXELS,
                )
            }
            _ => None,
        };
        let clic_acheve = self.clic.maj(
            souris,
            is_mouse_button_pressed(MouseButton::Left),
            is_mouse_button_released(MouseButton::Left),
        );
        if let Some(p) = clic_acheve {
            if zones.zone(p) == Zone::Vue3d {
                self.clic_dans_la_vue(p, ecran, &cam_info, aspect);
            }
        }
        self.maj_etats_ports();
        self.maj_surligne();

        // Couche nette : fond stellaire plein écran.
        set_camera(&cam3d);
        clear_background(BLACK);
        self.fond.draw(&cam_info);
        set_default_camera();

        // Couche chantier, éventuellement pixelisée.
        self.pixel.preparer(&mut cam3d);
        set_camera(&cam3d);
        // **Le chantier vide n'est pas rien.** Sans repère, une caméra orbitale
        // autour du néant ne montre aucun mouvement : impossible de juger que
        // le squelette marche. La grille donne l'origine, l'échelle et le plan
        // de pose — et c'est ce qu'on « regarde tourner » tant qu'aucune pièce
        // n'est posée.
        grille_chantier();
        if let Some(station) = self.apercu.doit_dessiner() {
            eclairage::avec(cam_info.pos, || match &self.maillage {
                Some(m) => m.dessiner(),
                None => station.dessiner(),
            });
            if self.enveloppes {
                enveloppes::station(station, enveloppes::CALME);
            }
        }
        // **L'overlay qui explique le refus** (§8.5, L5.2). Il ne décore pas :
        // il répond à la seule question qui rendrait l'écran incompréhensible
        // sans réponse — *pourquoi ce port est-il rouge ?*
        self.manque = self.dessiner_conflit();
        // **Ports libres seulement**, et colorés (§8.4). `Station::dessiner_ports`
        // trace TOUS les ports de toutes les pièces, consommés compris : bon pour
        // inspecter une station finie, trompeur ici, où un port déjà occupé
        // ressemblerait à une place libre.
        if self.ports {
            self.dessiner_ports_libres();
        }
        // Sous-arbre sélectionné : ce que Suppr emporterait.
        self.dessiner_surlignage();
        // **Le fantôme**, dernier et par-dessus : c'est ce qu'on est en train de
        // proposer. Recalculé chaque image — il suit le curseur (§10.3).
        self.dessiner_fantome();
        set_default_camera();
        self.pixel.presenter();

        // Port visé : cerclé à l'écran, par-dessus le rendu (donc net même sous
        // le filtre pixel — c'est un marqueur d'interface, pas de la géométrie).
        // Les couleurs de §8.4 arrivent en L4.4 ; ici il ne s'agit que de dire
        // « c'est celui-là que tu vises ».
        if let Some(id) = self.port_vise {
            if let Some(p) = self.chantier.libres().iter().find(|p| p.id == id) {
                if let Some(v) = crate::camera::projeter_ecran(p.repere.pos, ecran, &cam_info, aspect) {
                    draw_circle_lines(v.x, v.y, 9.0, 2.0, ui::COULEUR_ACTIF);
                }
            }
        }

        // **Le chantier vide doit dire quoi faire.** Un écran qui s'ouvre sur une
        // grille nue et une colonne de titres ne se devine pas : la consigne du
        // bandeau existait, mais en petit, tout en bas, au bout d'une ligne trop
        // longue. Ici elle est au milieu de la vue, là où l'œil arrive.
        if self.chantier.nb_pieces() == 0 {
            let phrase = "CHANTIER VIDE";
            let sous = "cliquer une piece dans la palette pour commencer";
            let c = zones.vue3d.center();
            let l1 = crate::police::mesure(phrase, 30);
            let l2 = crate::police::mesure(sous, 18);
            crate::police::texte(phrase, c.x - l1 * 0.5, c.y - 12.0, 30.0, Color::new(0.55, 0.95, 0.80, 0.9));
            crate::police::texte(sous, c.x - l2 * 0.5, c.y + 16.0, 18.0, Color::new(0.45, 0.70, 0.62, 0.9));
        }

        // ---- Interface, par-dessus la 3D ----
        match self.dessiner_palette(zones, souris, clic_acheve) {
            Some(Action::Plier(i)) => self.replie[i] = !self.replie[i],
            Some(Action::Choisir(i)) => self.choisir(i),
            None => {}
        }
        match self.dessiner_bandeau(zones, souris, clic_acheve) {
            Some(Bouton::Annuler) => self.muter(|ch| ch.annuler()),
            Some(Bouton::Refaire) => self.muter(|ch| ch.refaire()),
            None => {}
        }
        ui::boussole_axes(
            vec2(
                zones.vue3d.right() - BOUSSOLE_RAYON - BOUSSOLE_MARGE,
                zones.vue3d.bottom() - BOUSSOLE_RAYON - BOUSSOLE_MARGE,
            ),
            BOUSSOLE_RAYON,
            cam_info.right,
            cam_info.up,
            cam_info.forward,
        );
        false
    }

    /// Les ports **libres**, colorés selon §8.4. Au repos — aucune pièce en
    /// main — il n'y a rien à confronter : ils sont tous neutres, ce qui dit
    /// « voici où ça s'accroche » sans prétendre répondre à une question qu'on
    /// n'a pas posée.
    fn dessiner_ports_libres(&self) {
        for p in self.chantier.libres() {
            let etat = self.etats_ports.iter().find(|(id, _)| *id == p.id).map(|(_, e)| *e);
            let (col, rayon) = match etat {
                Some(ports::Etat::Posable) => (COL_POSABLE, 0.09),
                Some(ports::Etat::Encombre) => (COL_ENCOMBRE, 0.07),
                Some(ports::Etat::Incompatible) => (COL_ETEINT, 0.04),
                None => (COL_NEUTRE, 0.06), // au repos
            };
            let o = p.repere.pos;
            draw_sphere(o, rayon, None, col);
            // L'axe d'accostage : il dit dans quel sens la pièce partira.
            draw_line_3d(o, o + p.repere.avant().normalize_or_zero() * 0.5, col);
        }
    }

    /// Le sous-arbre sélectionné, cerclé de son **enveloppe de collision**.
    ///
    /// C'est le volume que le modèle connaît déjà : le réutiliser plutôt que de
    /// dessiner une boîte à part évite d'inventer une seconde géométrie de
    /// surlignage, et montre au passage ce que l'anti-collision voit.
    ///
    /// **Tout le sous-arbre**, pas la seule pièce cliquée : Suppr emporte les
    /// descendants (§6.2), et l'utilisateur doit le voir **avant** d'appuyer.
    fn dessiner_surlignage(&self) {
        if self.surligne.is_empty() {
            return;
        }
        for (id, piece) in self.chantier.pieces() {
            if self.surligne.contains(&id) {
                enveloppes::piece(piece, COL_SELECTION);
            }
        }
    }

    /// Trace, quand la pose visée serait refusée, l'enveloppe proposée en blanc,
    /// celles qui la refusent en rouge, et **le segment de plus courte
    /// approche** entre chacune et elle (§8.5).
    ///
    /// Rend le manque le plus criant, que le bandeau écrit en clair : un refus
    /// devient une phrase — *« il manque 0,42 »* — au lieu d'un port rouge sans
    /// explication.
    ///
    /// ⚠️ **Ne s'affiche que sur un port refusé.** Dessiné en permanence, il
    /// noierait l'écran de fils de fer et cesserait d'attirer l'œil là où il
    /// faut. La touche E reste, elle, l'affichage *de toutes* les enveloppes,
    /// qui répond à une autre question — « mes enveloppes sont-elles serrées ? »
    /// (la dette de L1.4, que seul le regard peut juger).
    fn dessiner_conflit(&self) -> Option<f32> {
        let port = self.port_vise?;
        // Seul un port ROUGE mérite une explication : vert, il n'y a rien à
        // expliquer ; éteint, ce n'est pas une affaire d'encombrement.
        let etat = self.etats_ports.iter().find(|(id, _)| *id == port).map(|(_, e)| *e)?;
        if etat != ports::Etat::Encombre {
            return None;
        }
        let comp = self.comp_en_main()?;
        let piece = self.chantier.pose_prevue(port, &comp, self.montage)?;
        let station = self.apercu.doit_dessiner()?;

        // La pièce d'accueil est exemptée, comme dans l'anti-collision : elle
        // touche le fantôme au joint, par construction. `apercu` conserve
        // l'ordre de `Chantier::pieces`, d'où l'indice.
        let hote = self.chantier.libres().iter().find(|p| p.id == port).map(|p| p.origine);
        let exempte = hote.and_then(|h| self.chantier.pieces().position(|(id, _)| id == h));

        let propose = comp.enveloppe_locale().transformee(piece.transforme);
        // **Le même critère que le modèle**, à la constante près : un overlay qui
        // expliquerait un refus que `Chantier` n'a pas prononcé serait pire que
        // pas d'overlay du tout.
        let (gene, manque) =
            enveloppes::conflit(station, &propose, crate::vaisseau::FACTEUR_COLLISION, exempte);
        (gene > 0).then_some(manque)
    }

    /// **Le fantôme** (§8.3) : la pièce exactement là où le clic la poserait.
    ///
    /// Il vient de `Chantier::pose_prevue`, qui passe par le **même**
    /// `corps_prevu` que `poser`. La vue ne recalcule pas la pose — un fantôme
    /// calculé à côté mentirait sur l'endroit où le clic va réellement poser,
    /// et c'est le doublon que le Lot 1 a passé son temps à supprimer.
    ///
    /// Dessiné même quand la pose serait **refusée** : `pose_prevue` répond au
    /// *où*, `peut_poser` au *si*. C'est ce que §8.5 exigera pour tracer le
    /// segment de plus courte approche entre le fantôme et la pièce qui le
    /// refuse — le fantôme doit donc exister précisément dans ce cas-là.
    fn dessiner_fantome(&self) {
        let Some(port) = self.port_vise else { return };
        let Some(comp) = self.comp_en_main() else { return };
        let Some(piece) = self.chantier.pose_prevue_roule(port, &comp, self.montage, self.roulis)
        else {
            return;
        };
        piece.dessiner();
    }

    /// Colonne de gauche : catégories repliables, puis les entrées (§8.2).
    ///
    /// Renvoie l'action déclenchée, s'il y en a une — **elle se décide ici et
    /// s'applique dehors** : `frame` tient le `&mut self`, et cette méthode ne
    /// prend que `&self` pour qu'il soit impossible d'y muter le chantier par
    /// mégarde.
    ///
    /// `clic` vient du discriminateur (§10.4), pas de `is_mouse_button_pressed` :
    /// un geste qui commence dans la vue 3D et finit sur la colonne ne doit pas
    /// choisir une pièce au passage.
    fn dessiner_palette(&self, zones: Zones, souris: Vec2, clic: Option<Vec2>) -> Option<Action> {
        ui::minitel_panel(zones.palette, "PALETTE");
        let clic = clic.filter(|p| zones.zone(*p) == Zone::Palette);
        let x = zones.palette.x + 8.0;
        let l = zones.palette.w - 16.0;
        let mut y = zones.palette.y + 40.0 - self.defilement;
        let bas = zones.palette.bottom();
        let mut clique = None;

        for (i, cat) in palette::categories(&self.entrees).into_iter().enumerate() {
            let r = Rect::new(x, y, l, 26.0);
            // Hors du panneau : on ne dessine pas, mais on continue d'avancer
            // `y` — sans quoi le défilement sauterait des catégories entières.
            if r.bottom() > zones.palette.y + 30.0 && r.y < bas {
                let fleche = if self.replie[i] { "+" } else { "-" };
                ui::minitel_ligne(r, &format!("{fleche} {}", cat.nom()), souris);
                if clic.is_some_and(|p| r.contains(p)) {
                    clique = Some(Action::Plier(i));
                }
            }
            y += 28.0;

            if self.replie[i] {
                continue;
            }
            for (j, (e, &posable)) in self.entrees.iter().zip(&self.posables).enumerate() {
                if e.cat != cat {
                    continue;
                }
                let r = Rect::new(x + 10.0, y, l - 10.0, 22.0);
                if r.bottom() > zones.palette.y + 30.0 && r.y < bas {
                    let en_main = matches!(self.interaction, Interaction::EnMain(k) if k == j);
                    ligne_entree(r, e.comp.nom(), posable, en_main, souris);
                    // Une entrée grisée ne se prend pas : `choisir` le
                    // revérifie, mais ne pas l'émettre du tout évite de faire
                    // clignoter une action qui n'aboutira jamais.
                    if posable && clic.is_some_and(|p| r.contains(p)) {
                        clique = Some(Action::Choisir(j));
                    }
                }
                y += 24.0;
            }
        }
        clique
    }

    /// Bandeau bas : compteurs et bascules.
    ///
    /// **Le coût vient de la station, pas du budget** (§10.7 a). Le bac à sable
    /// n'a pas de plafond, donc `Chantier::budget_restant` rend `INFINITY` :
    /// l'afficher n'aurait aucun sens. `Station::cout_total` somme les pièces
    /// posées, ce qui est exactement l'indication voulue.
    fn dessiner_bandeau(&self, zones: Zones, souris: Vec2, clic: Option<Vec2>) -> Option<Bouton> {
        ui::minitel_panel(zones.bandeau, "");
        let (pieces, cout) = match self.apercu.doit_dessiner() {
            Some(s) => (s.nb_pieces(), s.cout_total()),
            None => (0, 0.0),
        };
        let sommets = self.maillage.as_ref().map(|m| m.nb_sommets()).unwrap_or(0);
        let etat = |b: bool| if b { "ON" } else { "OFF" };
        // La consigne dit ce que l'écran attend **maintenant** : sans elle, un
        // chantier vide et un chantier avec une pièce en main se ressemblent, et
        // rien n'explique pourquoi le clic suivant ne fait pas la même chose
        // (§10.1 : la première pièce se pose sans viser de port).
        let consigne = match self.interaction {
            _ if pieces == 0 => "chantier vide : cliquer une piece la pose a l'origine",
            Interaction::EnMain(i) => &format!(
                "en main : {} — {}/{} montage (TAB) · {}° (R) — viser un port vert",
                self.entrees[i].comp.nom(),
                self.montage + 1,
                self.nb_montages().max(1),
                (self.roulis.to_degrees().round() as i32).rem_euclid(360),
            ),
            Interaction::Selectionnee(_) => &format!(
                "{} piece(s) selectionnee(s) — R tourne, M deplace, SUPPR retire",
                self.surligne.len()
            ),
            Interaction::Deplacee(_) => &format!(
                "deplacement de {} piece(s) — montage {}/{} (TAB) — viser un port (ECHAP annule)",
                self.surligne.len(),
                self.montage + 1,
                self.nb_montages().max(1),
            ),
            Interaction::Repos => "choisir une piece dans la palette",
        };
        // §8.5 : le refus devient une phrase, pas un port rouge muet.
        let refus = match self.manque {
            Some(m) => format!("   |   PORT ROUGE : il manque {:.2}", -m),
            None => String::new(),
        };
        // **Deux lignes, et la consigne d'abord.** Tout tenait sur une seule :
        // ~180 caractères, soit bien au-delà d'une fenêtre de 1280 px — la fin
        // était tout simplement coupée, et c'est la fin qui portait les
        // raccourcis. La ligne du haut dit **quoi faire maintenant**, en clair ;
        // celle du bas est l'aide-mémoire, en gris.
        crate::police::texte(
            &format!("{}{refus}", consigne.to_uppercase()),
            zones.bandeau.x + 16.0,
            zones.bandeau.y + 22.0,
            18.0,
            Color::new(0.65, 1.0, 0.80, 1.0),
        );
        crate::police::texte(
            &format!(
                "{pieces} pieces · cout {cout:.0} · {sommets} som.  |  R/Maj+R tourner  TAB montage  SUPPR retirer  M deplacer  Z/Y annuler  |  EMPL {} [{}] F5/F9  |  P E X",
                self.emplacement,
                // Occupés en chiffre, libres en point : l'état des neuf tient
                // dans neuf caractères, sans écran de gestion de fichiers.
                (1..=9)
                    .map(|i| if self.occupes[i - 1] { char::from_digit(i as u32, 10).unwrap() } else { '.' })
                    .collect::<String>(),
            ),
            zones.bandeau.x + 16.0,
            zones.bandeau.y + 44.0,
            14.0,
            GRAY,
        );
        let _ = (etat(self.ports), etat(self.enveloppes));

        // Retour de la dernière action disque, quelques secondes. Une écriture
        // qui ne dit rien laisse croire qu'elle n'a pas eu lieu.
        if let Some((texte, quand)) = &self.message {
            let age = get_time() - quand;
            if age < 4.0 {
                let fane = (1.0 - (age / 4.0) as f32).clamp(0.0, 1.0);
                crate::police::texte(
                    texte,
                    zones.bandeau.x + 16.0,
                    zones.bandeau.y - 10.0,
                    18.0,
                    Color::new(1.0, 0.9, 0.4, fane),
                );
            }
        }

        // UNDO / REDO à droite (§8.2). Le clavier (Z/Y) les fait déjà ; les
        // boutons les rendent **découvrables**, ce qu'un raccourci n'est pas.
        let l = 92.0;
        let h = zones.bandeau.h - 16.0;
        let y = zones.bandeau.y + 8.0;
        let r_refaire = Rect::new(zones.bandeau.right() - l - 12.0, y, l, h);
        let r_annuler = Rect::new(r_refaire.x - l - 8.0, y, l, h);
        ui::minitel_ligne(r_annuler, "< UNDO  Z", souris);
        ui::minitel_ligne(r_refaire, "REDO  Y >", souris);
        if clic.is_some_and(|p| r_annuler.contains(p)) {
            return Some(Bouton::Annuler);
        }
        if clic.is_some_and(|p| r_refaire.contains(p)) {
            return Some(Bouton::Refaire);
        }
        None
    }
}

/// Bouton du bandeau qui a été cliqué. Même schéma que [`Action`] : décidé au
/// dessin, appliqué par `frame`, qui seul tient le `&mut self`.
enum Bouton {
    Annuler,
    Refaire,
}

/// Une entrée de palette. **Grisée = ne se pose nulle part** sur le chantier
/// courant (§10.2) : le fond s'éteint et le texte passe au gris sourd, pour que
/// l'utilisateur ne la choisisse pas avant de découvrir un vaisseau sans aucun
/// port allumé.
///
/// Elle reste **visible** plutôt que masquée : une palette dont les entrées
/// apparaissent et disparaissent au fil des poses ne se mémorise pas, et
/// `VueStation` a déjà tranché dans ce sens pour son bouton de rotation.
fn ligne_entree(r: Rect, nom: &str, posable: bool, en_main: bool, souris: Vec2) {
    if en_main {
        // Pièce en main : l'entrée reste allumée en permanence, sinon rien à
        // l'écran ne dit **quoi** on est en train de poser dès que le curseur
        // quitte la colonne pour viser un port.
        draw_rectangle(r.x, r.y, r.w, r.h, ui::COULEUR_ACTIF);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, WHITE);
        crate::police::texte(nom, r.x + 10.0, r.y + r.h * 0.5 + 6.0, 18.0, BLACK);
        return;
    }
    if posable {
        ui::minitel_ligne(r, nom, souris);
        return;
    }
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.05, 0.06, 0.09, 1.0));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.20, 0.24, 0.26, 1.0));
    crate::police::texte(nom, r.x + 10.0, r.y + r.h * 0.5 + 6.0, 18.0, Color::new(0.34, 0.38, 0.40, 1.0));
}

/// Grille du plan de pose : repère d'origine et échelle du chantier vide.
/// Dessinée **sous** la station, sans éclairage — c'est du mobilier d'écran,
/// pas de la géométrie de vaisseau.
fn grille_chantier() {
    const DEMI: i32 = 8; // 8 unités de part et d'autre : le recul initial en cadre 12
    const PAS: f32 = 1.0;
    let bord = DEMI as f32 * PAS;
    let gris = Color::new(0.16, 0.22, 0.20, 1.0);
    for i in -DEMI..=DEMI {
        let t = i as f32 * PAS;
        // Les axes X et Z ressortent : sans eux la grille n'a pas de centre.
        let c = if i == 0 { Color::new(0.28, 0.42, 0.38, 1.0) } else { gris };
        draw_line_3d(vec3(t, 0.0, -bord), vec3(t, 0.0, bord), c);
        draw_line_3d(vec3(-bord, 0.0, t), vec3(bord, 0.0, t), c);
    }
    // Verticale d'origine : donne le haut, que la grille seule ne dit pas.
    draw_line_3d(Vec3::ZERO, vec3(0.0, 2.0, 0.0), Color::new(0.30, 0.46, 0.42, 1.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Les trois zones pavent l'écran sans se chevaucher.** Un trou et le clic
    /// tombe dans le vide ; un recouvrement et deux zones se disputent le même
    /// point — c'est la palette qui capterait un clic destiné au vaisseau, ou
    /// l'inverse.
    #[test]
    fn les_zones_pavent_l_ecran() {
        for taille in [vec2(1280.0, 720.0), vec2(1920.0, 1080.0), vec2(1000.0, 700.0)] {
            let z = Zones::pour(taille);
            let somme = z.palette.w * z.palette.h + z.vue3d.w * z.vue3d.h + z.bandeau.w * z.bandeau.h;
            assert!(
                (somme - taille.x * taille.y).abs() < 1.0,
                "{taille:?} : les zones couvrent {somme} pour un écran de {}",
                taille.x * taille.y
            );
            // Palette et vue se touchent sans se croiser.
            assert_eq!(z.palette.right(), z.vue3d.x, "trou ou recouvrement entre palette et vue");
            assert_eq!(z.palette.bottom(), z.bandeau.y, "trou ou recouvrement au-dessus du bandeau");
        }
    }

    /// **Chaque point est classé dans la zone qui le contient.** Le test vise
    /// les quatre coins et le voisinage des frontières, pas le milieu des
    /// rectangles : c'est aux bords qu'un classement se trompe.
    #[test]
    fn chaque_point_releve_de_sa_zone() {
        let taille = vec2(1280.0, 720.0);
        let z = Zones::pour(taille);
        assert_eq!(z.zone(vec2(4.0, 4.0)), Zone::Palette, "coin haut-gauche");
        assert_eq!(z.zone(vec2(taille.x - 4.0, 4.0)), Zone::Vue3d, "coin haut-droit");
        assert_eq!(z.zone(vec2(4.0, taille.y - 4.0)), Zone::Bandeau, "coin bas-gauche");
        assert_eq!(z.zone(vec2(taille.x - 4.0, taille.y - 4.0)), Zone::Bandeau, "coin bas-droit");
        // De part et d'autre de la frontière palette / vue.
        let x = z.palette.right();
        assert_eq!(z.zone(vec2(x - 1.0, 100.0)), Zone::Palette);
        assert_eq!(z.zone(vec2(x + 1.0, 100.0)), Zone::Vue3d);
        // Le bandeau court SOUS la palette : un point bas à gauche lui revient.
        let y = z.bandeau.y;
        assert_eq!(z.zone(vec2(40.0, y - 1.0)), Zone::Palette);
        assert_eq!(z.zone(vec2(40.0, y + 1.0)), Zone::Bandeau);
    }

    /// **Seule la vue 3D fait pivoter la caméra.** Glisser sur la palette pour
    /// choisir une pièce ne doit pas faire tourner le vaisseau derrière.
    #[test]
    fn seule_la_vue_capte_le_glisser() {
        let z = Zones::pour(vec2(1280.0, 720.0));
        assert!(z.hors_de_la_vue(vec2(40.0, 100.0)), "la palette laisse la caméra tranquille");
        assert!(z.hors_de_la_vue(vec2(640.0, 700.0)), "le bandeau aussi");
        assert!(!z.hors_de_la_vue(vec2(640.0, 300.0)), "la vue 3D, elle, orbite");
    }

    /// Écran étroit : la palette ne doit pas déborder ni créer une vue de
    /// largeur négative. Le cas n'est pas théorique — `Parametres` propose du
    /// 1000×700, et rien n'interdit plus petit.
    #[test]
    fn un_ecran_etroit_ne_produit_pas_de_zone_absurde() {
        let z = Zones::pour(vec2(120.0, 400.0));
        for r in [z.palette, z.vue3d, z.bandeau] {
            assert!(r.w >= 0.0 && r.h >= 0.0, "rectangle de dimension négative : {r:?}");
        }
        assert!(z.palette.right() <= 120.0, "la palette déborde de l'écran");
    }
}
