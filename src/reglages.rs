//! **Réglages du jeu** : mode d'affichage, taille de fenêtre, pipeline de rendu.
//!
//! Le modèle vit ici et se teste ; l'écran qui le montre est
//! [`crate::ecran::parametres`]. C'est la règle du projet : tout ce qui se
//! décide sort du code de dessin.
//!
//! # ⚠️ « Plein écran » et « sans bordure » sont la même chose ici
//!
//! macroquad n'expose qu'un `set_fullscreen(bool)`. Sur Windows, miniquad
//! l'implémente en passant la fenêtre en `WS_POPUP` redimensionnée à l'écran
//! entier (`native/windows.rs`) — c'est **exactement** un plein écran sans
//! bordure. Il n'y a pas de mode exclusif : aucun changement de mode vidéo,
//! aucune prise en main du balayage.
//!
//! Proposer les deux comme des entrées distinctes donnerait deux boutons au
//! comportement identique. On n'en propose donc que **deux modes réels**, et la
//! variante exclusive est nommée dans l'énumération pour le jour où le socle
//! saura la faire — pas avant.

use macroquad::prelude::*;
use std::cell::Cell;

/// Comment la scène 3D est rendue.
///
/// Trois états, pas deux : le filtre « gros pixels » existe dans le jeu depuis
/// longtemps (touche P) et reste utile seul. La quantification de palette est
/// une **seconde** étape qui s'empile dessus — l'offrir sans les gros pixels
/// n'aurait pas de sens (des à-plats de palette en pleine résolution), et
/// remplacer le mode existant ferait perdre un rendu qui marche.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeRendu {
    /// Aucun filtre : la scène est dessinée à la résolution de l'écran.
    Net,
    /// Cible basse résolution remontée au plus proche voisin. Les couleurs
    /// restent des dégradés RGB continus.
    Pixel,
    /// Idem, **plus** la quantification vers la palette choisie. C'est ce
    /// qui produit les à-plats, donc le pixel art proprement dit.
    Palette,
}

impl ModeRendu {
    pub const TOUS: [ModeRendu; 3] = [ModeRendu::Net, ModeRendu::Pixel, ModeRendu::Palette];

    pub fn nom(self) -> &'static str {
        match self {
            ModeRendu::Net => "NET",
            ModeRendu::Pixel => "PIXEL ART",
            ModeRendu::Palette => "PIXEL ART + PALETTE",
        }
    }

    /// Le mode suivant, en boucle.
    pub fn suivant(self) -> Self {
        let i = Self::TOUS.iter().position(|m| *m == self).unwrap_or(0);
        Self::TOUS[(i + 1) % Self::TOUS.len()]
    }

    /// Faut-il passer par la cible basse résolution ?
    pub fn pixelise(self) -> bool {
        self != ModeRendu::Net
    }

    /// Faut-il quantifier vers la palette au moment du blit ?
    ///
    /// **Implique [`Self::pixelise`]** : quantifier sans réduire la résolution
    /// donnerait des à-plats en pleine définition, ce qui n'est pas du pixel
    /// art. L'invariant est tenu par `quantifier_implique_pixeliser`.
    pub fn quantifie(self) -> bool {
        self == ModeRendu::Palette
    }
}

/// Force du tramage ordonné appliqué avant la quantification.
///
/// # Pourquoi c'est nécessaire
///
/// Mesuré sur Resurrect 64 : un dégradé de gris ne tombe que sur **8 couleurs**,
/// et une marche fait sauter la clarté de L=49 à L=69. Sans tramage, une région
/// entière d'une planète bascule d'un coup quand l'ombrage la traverse — c'est
/// le scintillement observé à l'écran.
///
/// Le tramage rend les teintes intermédiaires en mélangeant spatialement deux
/// entrées voisines, si bien que la transition devient progressive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tramage {
    /// Aucun. Aplats francs, et les marches de la palette en pleine face.
    Non,
    /// Discret : adoucit sans que le motif se remarque.
    Leger,
    /// Assez fort pour traverser les grandes marches d'une palette d'artiste.
    Fort,
}

impl Tramage {
    pub const TOUS: [Tramage; 3] = [Tramage::Non, Tramage::Leger, Tramage::Fort];

    pub fn nom(self) -> &'static str {
        match self {
            Tramage::Non => "NON",
            Tramage::Leger => "LEGER",
            Tramage::Fort => "FORT",
        }
    }

    pub fn suivant(self) -> Self {
        let i = Self::TOUS.iter().position(|t| *t == self).unwrap_or(0);
        Self::TOUS[(i + 1) % Self::TOUS.len()]
    }

    /// Amplitude, en unités sRGB.
    ///
    /// `Fort` vaut 0,18 parce que c'est **la taille mesurée de la pire marche**
    /// de Resurrect 64 dans les gris (de 0,50 à 0,68) : en deçà, le tramage ne
    /// traverse pas et la bande bascule quand même.
    pub fn force(self) -> f32 {
        match self {
            Tramage::Non => 0.0,
            Tramage::Leger => 0.08,
            Tramage::Fort => 0.18,
        }
    }
}

/// Combien on ravive la couleur avant de quantifier.
///
/// # Pourquoi c'est nécessaire
///
/// Mesuré : une planète voilée par son atmosphère n'a qu'une chroma modérée, et
/// les entrées **neutres** d'une palette, voisines en CIELAB, l'emportent. Une
/// forêt voilée de chroma 17,6 tombait sur `#374e4a`, de chroma 9,8 ; une rampe
/// de Terre plausible tombait trois fois sur douze sur `#625565`, le gris-violet
/// qu'on voyait à la place des océans.
///
/// Raviver avant la recherche pousse la couleur sur les **rampes colorées** de
/// la palette au lieu de ses neutres — c'est d'ailleurs ce que fait un
/// dessinateur, qui ne peint pas avec les teintes intermédiaires ternes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Saturation {
    /// Aucune retouche : la couleur part telle que la scène l'a produite.
    Non,
    Moyen,
    /// Mesuré : +53 % de chroma en sortie sur une rampe de Terre.
    Fort,
}

impl Saturation {
    pub const TOUS: [Saturation; 3] = [Saturation::Non, Saturation::Moyen, Saturation::Fort];

    pub fn nom(self) -> &'static str {
        match self {
            Saturation::Non => "NON",
            Saturation::Moyen => "MOYEN",
            Saturation::Fort => "FORT",
        }
    }

    pub fn suivant(self) -> Self {
        let i = Self::TOUS.iter().position(|s| *s == self).unwrap_or(0);
        Self::TOUS[(i + 1) % Self::TOUS.len()]
    }

    /// Facteur multiplicatif de la chroma. 1 = inchangé.
    pub fn gain(self) -> f32 {
        match self {
            Saturation::Non => 1.0,
            Saturation::Moyen => 1.45,
            Saturation::Fort => 1.9,
        }
    }
}

/// L'état de rendu au démarrage. **Une seule constante** pour l'état global et
/// pour [`Reglages::default`] : deux valeurs écrites séparément finiraient par
/// diverger, et l'écran des paramètres annoncerait un réglage que le rendu
/// n'applique pas.
const RENDU_INITIAL: EtatRendu = EtatRendu {
    mode: ModeRendu::Net,
    palette: 0,
    tramage: Tramage::Fort,
    // `Fort` par défaut : c'est le réglage qui corrige l'aspect terne constaté à
    // l'écran, et il est calé sur une mesure, pas sur un goût.
    saturation: Saturation::Fort,
};

/// Ce que le code de dessin a besoin de savoir du rendu.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EtatRendu {
    pub mode: ModeRendu,
    /// Indice dans [`crate::palette::toutes`] — le nombre de palettes dépend du
    /// dossier d'assets, donc il est ramené dans les bornes à la lecture.
    pub palette: usize,
    pub tramage: Tramage,
    pub saturation: Saturation,
}

thread_local! {
    /// L'état de rendu, lisible depuis le code de dessin.
    ///
    /// Un état global plutôt qu'un paramètre passé de main aux sept vues : c'est
    /// déjà la convention du projet pour l'état de rendu transversal
    /// (`disque::set_viewport_h`, `planete::set_viewport_h`). macroquad est
    /// mono-thread, le `thread_local` n'est donc jamais partagé.
    static RENDU: Cell<EtatRendu> = const { Cell::new(RENDU_INITIAL) };
}

/// L'état de rendu courant.
pub fn etat_rendu() -> EtatRendu {
    RENDU.with(|r| r.get())
}

/// Le mode de rendu courant — ce que lit le code de dessin.
pub fn mode_rendu() -> ModeRendu {
    etat_rendu().mode
}

/// Change le **mode** seul, sans toucher à la palette ni au tramage : c'est ce
/// que font les raccourcis clavier des vues.
pub fn regler_rendu(m: ModeRendu) {
    RENDU.with(|r| {
        let mut e = r.get();
        e.mode = m;
        r.set(e);
    });
}

/// Pousse l'état complet — appelé par [`Reglages::appliquer`].
pub fn regler_etat_rendu(e: EtatRendu) {
    RENDU.with(|r| r.set(e));
}

/// Comment la fenêtre occupe l'écran.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeAffichage {
    /// Fenêtre classique, à la taille choisie dans [`Reglages::resolution`].
    Fenetre,
    /// Plein écran **sans bordure** : la fenêtre couvre l'écran, le bureau
    /// reste derrière. C'est ce que fait `set_fullscreen(true)`.
    SansBordure,
}

impl ModeAffichage {
    pub const TOUS: [ModeAffichage; 2] = [ModeAffichage::Fenetre, ModeAffichage::SansBordure];

    pub fn nom(self) -> &'static str {
        match self {
            ModeAffichage::Fenetre => "FENETRE",
            ModeAffichage::SansBordure => "PLEIN ECRAN SANS BORDURE",
        }
    }

    /// Le mode suivant, en boucle.
    pub fn suivant(self) -> Self {
        let i = Self::TOUS.iter().position(|m| *m == self).unwrap_or(0);
        Self::TOUS[(i + 1) % Self::TOUS.len()]
    }

    /// La taille de fenêtre a-t-elle un sens dans ce mode ? En plein écran,
    /// non : c'est l'écran qui décide. Le bouton doit alors être grisé plutôt
    /// que de faire semblant.
    pub fn taille_reglable(self) -> bool {
        self == ModeAffichage::Fenetre
    }
}

/// Une taille de fenêtre proposée.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Resolution {
    pub largeur: u32,
    pub hauteur: u32,
}

impl Resolution {
    /// Les tailles proposées, **par ratio classique**, de la plus petite à la
    /// plus grande.
    ///
    /// **Pas de 4K** : c'est la demande, et elle est raisonnable — le rendu est
    /// en impostors plein écran, et le coût monte comme le nombre de pixels.
    /// Le plafond est donc 1920 × 1200.
    pub const TOUTES: [Resolution; 9] = [
        // 4:3 — les deux tailles historiques.
        Resolution { largeur: 1024, hauteur: 768 },
        Resolution { largeur: 1280, hauteur: 960 },
        // 16:10
        Resolution { largeur: 1280, hauteur: 800 },
        Resolution { largeur: 1440, hauteur: 900 },
        Resolution { largeur: 1680, hauteur: 1050 },
        Resolution { largeur: 1920, hauteur: 1200 },
        // 16:9
        Resolution { largeur: 1280, hauteur: 720 },
        Resolution { largeur: 1600, hauteur: 900 },
        Resolution { largeur: 1920, hauteur: 1080 },
    ];

    /// Ratio d'aspect, en toutes lettres. **Déduit** des deux dimensions : une
    /// étiquette posée à côté finirait par mentir sur une entrée retouchée.
    pub fn ratio(self) -> &'static str {
        let (l, h) = (self.largeur as f32, self.hauteur as f32);
        let r = l / h;
        // Tolérance large : 1280×800 fait 1,600 et 1680×1050 aussi, mais
        // 1920×1200 fait 1,600 également — tous sont bien du 16:10.
        if (r - 4.0 / 3.0).abs() < 0.02 {
            "4:3"
        } else if (r - 16.0 / 10.0).abs() < 0.02 {
            "16:10"
        } else if (r - 16.0 / 9.0).abs() < 0.02 {
            "16:9"
        } else {
            "?"
        }
    }

    pub fn libelle(self) -> String {
        format!("{} x {}  ({})", self.largeur, self.hauteur, self.ratio())
    }

    /// La suivante de la liste, en boucle.
    pub fn suivante(self) -> Self {
        let i = Self::TOUTES.iter().position(|r| *r == self).unwrap_or(0);
        Self::TOUTES[(i + 1) % Self::TOUTES.len()]
    }
}

/// L'état des réglages.
pub struct Reglages {
    pub mode: ModeAffichage,
    pub resolution: Resolution,
    pub rendu: ModeRendu,
    /// Indice de palette dans [`crate::palette::toutes`].
    pub palette: usize,
    pub tramage: Tramage,
    pub saturation: Saturation,
}

impl Default for Reglages {
    fn default() -> Self {
        // La taille de la fenêtre au démarrage (`window_conf`) n'est dans aucune
        // des listes : on part de la plus proche pour que le bouton n'affiche
        // pas une taille que la fenêtre n'a pas.
        Self {
            mode: ModeAffichage::Fenetre,
            resolution: Resolution::TOUTES[6],
            rendu: RENDU_INITIAL.mode,
            palette: RENDU_INITIAL.palette,
            tramage: RENDU_INITIAL.tramage,
            saturation: RENDU_INITIAL.saturation,
        }
    }
}

impl Reglages {
    /// Applique les réglages à la fenêtre.
    ///
    /// ⚠️ **N'est pas idempotent côté système** : `request_new_screen_size`
    /// redimensionne à chaque appel. À n'appeler que sur **changement**, jamais
    /// à chaque frame, sans quoi la fenêtre refuserait tout redimensionnement
    /// à la souris.
    pub fn appliquer(&self) {
        // Le rendu, lui, est idempotent : ce n'est qu'une écriture d'état lue
        // par le code de dessin à la frame suivante.
        regler_etat_rendu(EtatRendu {
            mode: self.rendu,
            palette: self.palette,
            tramage: self.tramage,
            saturation: self.saturation,
        });
        match self.mode {
            ModeAffichage::Fenetre => {
                set_fullscreen(false);
                request_new_screen_size(self.resolution.largeur as f32, self.resolution.hauteur as f32);
            }
            // En plein écran, la taille vient de l'écran : on ne la force pas.
            ModeAffichage::SansBordure => set_fullscreen(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // **Pas de 4K** : c'est la demande. Le plafond est 1920 de large.
    #[test]
    fn aucune_resolution_natteint_la_4k() {
        for r in Resolution::TOUTES {
            assert!(r.largeur <= 1920, "{} est au-delà du plafond", r.libelle());
            assert!(r.hauteur <= 1200, "{} est au-delà du plafond", r.libelle());
        }
    }

    // **Chaque taille tombe sur un ratio classique.** Le ratio est déduit des
    // dimensions ; si une entrée est mal saisie, il sort « ? » et le test mord.
    #[test]
    fn chaque_resolution_a_un_ratio_classique() {
        for r in Resolution::TOUTES {
            assert_ne!(r.ratio(), "?", "{} n'est d'aucun ratio classique", r.libelle());
        }
        // Et les trois familles sont représentées : une liste tout en 16:9
        // passerait le test ci-dessus sans offrir de choix.
        let ratios: std::collections::HashSet<&str> =
            Resolution::TOUTES.iter().map(|r| r.ratio()).collect();
        for attendu in ["4:3", "16:10", "16:9"] {
            assert!(ratios.contains(attendu), "aucune taille en {attendu}");
        }
    }

    // Deux entrées identiques donneraient deux lignes indiscernables dans le
    // menu, et un cycle qui semble bloqué.
    #[test]
    fn les_resolutions_sont_toutes_distinctes() {
        let mut vues = std::collections::HashSet::new();
        for r in Resolution::TOUTES {
            assert!(vues.insert((r.largeur, r.hauteur)), "{} en double", r.libelle());
            assert!(!r.libelle().is_empty());
        }
    }

    // **Le cycle passe par toutes les entrées et revient au début.** Un cycle
    // qui sauterait une taille la rendrait inatteignable — le seul moyen de
    // choisir étant ce bouton.
    #[test]
    fn le_cycle_des_resolutions_les_visite_toutes() {
        let depart = Resolution::TOUTES[0];
        let mut vues = vec![depart];
        let mut r = depart;
        for _ in 0..Resolution::TOUTES.len() - 1 {
            r = r.suivante();
            assert!(!vues.contains(&r), "le cycle repasse par {} trop tôt", r.libelle());
            vues.push(r);
        }
        assert_eq!(vues.len(), Resolution::TOUTES.len(), "des tailles sont inatteignables");
        assert_eq!(r.suivante(), depart, "le cycle ne boucle pas");
    }

    // Idem pour les modes, et **chacun a un nom non vide** : un bouton sans
    // texte ne dit pas dans quel mode on est.
    //
    // ⚠️ Première version : elle partait de `TOUS[0]`, avançait `len` fois et
    // vérifiait qu'on revenait au départ. Avec deux modes, un `suivant` qui
    // rendrait **toujours** `TOUS[0]` passait ce test — on revenait bien au
    // départ, sans jamais en bouger. Il faut compter les modes **visités**.
    #[test]
    fn le_cycle_des_modes_les_visite_tous_et_boucle() {
        let depart = ModeAffichage::TOUS[0];
        let mut vus = vec![depart];
        let mut m = depart;
        for _ in 0..ModeAffichage::TOUS.len() - 1 {
            m = m.suivant();
            assert!(!vus.contains(&m), "le cycle repasse par {m:?} trop tôt");
            vus.push(m);
        }
        assert_eq!(vus.len(), ModeAffichage::TOUS.len(), "des modes sont inatteignables");
        assert_eq!(m.suivant(), depart, "le cycle des modes ne boucle pas");
        for m in ModeAffichage::TOUS {
            assert!(!m.nom().is_empty(), "{m:?} sans nom");
        }
        // Deux modes ne peuvent pas porter le même nom.
        let noms: std::collections::HashSet<&str> =
            ModeAffichage::TOUS.iter().map(|m| m.nom()).collect();
        assert_eq!(noms.len(), ModeAffichage::TOUS.len());
    }

    // **La taille ne se règle qu'en fenêtré.** En plein écran c'est l'écran qui
    // décide : laisser le bouton actif ferait croire à un réglage sans effet.
    #[test]
    fn la_taille_ne_se_regle_quen_fenetre() {
        assert!(ModeAffichage::Fenetre.taille_reglable());
        assert!(!ModeAffichage::SansBordure.taille_reglable());
    }

    // Le réglage par défaut est **dans la liste** : sinon le bouton afficherait
    // une taille absente du cycle, et le premier clic ferait un saut.
    #[test]
    fn le_reglage_par_defaut_est_dans_la_liste() {
        let d = Reglages::default();
        assert!(Resolution::TOUTES.contains(&d.resolution), "défaut hors liste : {}", d.resolution.libelle());
        assert!(ModeAffichage::TOUS.contains(&d.mode));
        assert!(ModeRendu::TOUS.contains(&d.rendu));
    }

    // Même exigence pour les modes de rendu : un mode qu'aucun clic n'atteint
    // serait du code mort, le bouton étant le seul moyen d'en changer.
    #[test]
    fn le_cycle_des_modes_de_rendu_les_visite_tous_et_boucle() {
        let depart = ModeRendu::TOUS[0];
        let mut vus = vec![depart];
        let mut m = depart;
        for _ in 0..ModeRendu::TOUS.len() - 1 {
            m = m.suivant();
            assert!(!vus.contains(&m), "le cycle repasse par {m:?} trop tôt");
            vus.push(m);
        }
        assert_eq!(vus.len(), ModeRendu::TOUS.len(), "des modes de rendu sont inatteignables");
        assert_eq!(m.suivant(), depart, "le cycle des rendus ne boucle pas");
        let noms: std::collections::HashSet<&str> = ModeRendu::TOUS.iter().map(|m| m.nom()).collect();
        assert_eq!(noms.len(), ModeRendu::TOUS.len(), "deux modes de rendu portent le même nom");
    }

    // **Quantifier implique pixeliser.** Quantifier sans réduire la résolution
    // donnerait des à-plats de palette en pleine définition — pas du pixel art,
    // et un blit qui traverserait le shader sans cible basse résolution.
    #[test]
    fn quantifier_implique_pixeliser() {
        for m in ModeRendu::TOUS {
            assert!(!m.quantifie() || m.pixelise(), "{m:?} quantifie sans pixeliser");
        }
        // Et les trois états sont bien distincts deux à deux : sans ça, le menu
        // proposerait un choix sans effet.
        let etats: std::collections::HashSet<(bool, bool)> =
            ModeRendu::TOUS.iter().map(|m| (m.pixelise(), m.quantifie())).collect();
        assert_eq!(etats.len(), ModeRendu::TOUS.len(), "deux modes de rendu font la même chose");
    }

    // **L'état global démarre sur le réglage par défaut.** S'ils divergeaient,
    // l'écran des paramètres annoncerait un mode que le rendu n'applique pas
    // tant qu'on n'a pas cliqué.
    //
    // Lu dans un fil neuf : le `thread_local` y repart de sa valeur initiale,
    // donc le test ne dépend pas de ce que les autres ont réglé avant lui.
    #[test]
    fn letat_global_demarre_sur_le_reglage_par_defaut() {
        let lu = std::thread::spawn(etat_rendu).join().unwrap();
        let d = Reglages::default();
        assert_eq!(lu.mode, d.rendu, "le mode global et le défaut ont divergé");
        assert_eq!(lu.palette, d.palette, "la palette globale et le défaut ont divergé");
        assert_eq!(lu.tramage, d.tramage, "le tramage global et le défaut ont divergé");
        assert_eq!(lu.saturation, d.saturation, "la saturation globale et le défaut ont divergé");
    }

    // Idem pour le tramage : un cycle qui sauterait un niveau le rendrait
    // inatteignable, le menu étant le seul moyen d'en changer.
    #[test]
    fn le_cycle_du_tramage_le_visite_tout_et_boucle() {
        let depart = Tramage::TOUS[0];
        let mut vus = vec![depart];
        let mut t = depart;
        for _ in 0..Tramage::TOUS.len() - 1 {
            t = t.suivant();
            assert!(!vus.contains(&t), "le cycle repasse par {t:?} trop tôt");
            vus.push(t);
        }
        assert_eq!(vus.len(), Tramage::TOUS.len(), "des niveaux sont inatteignables");
        assert_eq!(t.suivant(), depart, "le cycle du tramage ne boucle pas");
        let noms: std::collections::HashSet<&str> = Tramage::TOUS.iter().map(|t| t.nom()).collect();
        assert_eq!(noms.len(), Tramage::TOUS.len(), "deux niveaux portent le même nom");
    }

    // **Les forces de tramage sont croissantes, et seul `Non` est nul.** Deux
    // niveaux de même force donneraient deux entrées de menu indiscernables ;
    // une force nulle ailleurs que sur `Non` serait un réglage sans effet.
    #[test]
    fn les_forces_de_tramage_sont_croissantes() {
        assert_eq!(Tramage::Non.force(), 0.0, "« NON » trame quand même");
        let mut precedente = -1.0;
        for t in Tramage::TOUS {
            assert!(t.force() > precedente, "{t:?} ne trame pas plus que le précédent");
            precedente = t.force();
        }
        // Le plus fort doit couvrir la pire marche mesurée de Resurrect 64
        // (0,50 → 0,68 dans les gris), sinon la bande bascule quand même.
        assert!(
            Tramage::Fort.force() >= 0.18,
            "« FORT » à {} ne traverse pas la marche mesurée (0,18)",
            Tramage::Fort.force()
        );
    }

    // Même exigence de cycle pour la saturation.
    #[test]
    fn le_cycle_de_la_saturation_la_visite_toute_et_boucle() {
        let depart = Saturation::TOUS[0];
        let mut vus = vec![depart];
        let mut s = depart;
        for _ in 0..Saturation::TOUS.len() - 1 {
            s = s.suivant();
            assert!(!vus.contains(&s), "le cycle repasse par {s:?} trop tôt");
            vus.push(s);
        }
        assert_eq!(vus.len(), Saturation::TOUS.len(), "des niveaux sont inatteignables");
        assert_eq!(s.suivant(), depart, "le cycle de la saturation ne boucle pas");
        let noms: std::collections::HashSet<&str> =
            Saturation::TOUS.iter().map(|s| s.nom()).collect();
        assert_eq!(noms.len(), Saturation::TOUS.len(), "deux niveaux portent le même nom");
    }

    // **Les gains sont croissants, et seul `Non` est neutre.** Un gain inférieur
    // à 1 ternirait au lieu de raviver ; deux gains égaux donneraient deux
    // entrées de menu indiscernables.
    #[test]
    fn les_gains_de_saturation_sont_croissants() {
        assert_eq!(Saturation::Non.gain(), 1.0, "« NON » retouche quand même");
        let mut precedent = 0.0;
        for s in Saturation::TOUS {
            assert!(s.gain() >= 1.0, "{s:?} ternit au lieu de raviver");
            assert!(s.gain() > precedent, "{s:?} ne ravive pas plus que le précédent");
            precedent = s.gain();
        }
        // Le plus fort doit apporter un gain net et mesurable : à 1,2 la Terre
        // resterait grise-violette.
        assert!(Saturation::Fort.gain() >= 1.8, "« FORT » à {} est trop timide", Saturation::Fort.gain());
    }

    // **Le raccourci clavier ne change que le mode.** Il ne doit pas remettre la
    // palette ou le tramage à zéro au passage : ce sont des réglages choisis
    // dans le menu, et les perdre en appuyant sur P serait déroutant.
    #[test]
    fn le_raccourci_ne_touche_quau_mode() {
        std::thread::spawn(|| {
            regler_etat_rendu(EtatRendu {
                mode: ModeRendu::Net,
                palette: 2,
                tramage: Tramage::Leger,
                saturation: Saturation::Moyen,
            });
            regler_rendu(ModeRendu::Palette);
            let e = etat_rendu();
            assert_eq!(e.mode, ModeRendu::Palette, "le mode n'a pas changé");
            assert_eq!(e.palette, 2, "la palette a été perdue");
            assert_eq!(e.tramage, Tramage::Leger, "le tramage a été perdu");
            assert_eq!(e.saturation, Saturation::Moyen, "la saturation a été perdue");
        })
        .join()
        .unwrap();
    }

    // Ce qu'on règle est ce qu'on relit — c'est le seul canal entre l'écran des
    // paramètres et le code de dessin.
    #[test]
    fn le_mode_de_rendu_se_relit_tel_quel() {
        std::thread::spawn(|| {
            for m in ModeRendu::TOUS {
                regler_rendu(m);
                assert_eq!(mode_rendu(), m, "{m:?} n'est pas relu tel quel");
            }
            for t in Tramage::TOUS {
                for palette in 0..3 {
                    let e = EtatRendu {
                        mode: ModeRendu::Palette,
                        palette,
                        tramage: t,
                        saturation: Saturation::Moyen,
                    };
                    regler_etat_rendu(e);
                    assert_eq!(etat_rendu(), e, "{e:?} n'est pas relu tel quel");
                }
            }
        })
        .join()
        .unwrap();
    }

    // La palette par défaut **existe** : un indice hors liste afficherait une
    // palette au démarrage et une autre au premier clic.
    #[test]
    fn la_palette_par_defaut_existe() {
        let d = Reglages::default();
        assert!(
            d.palette < crate::palette::toutes().len(),
            "indice {} pour {} palettes",
            d.palette,
            crate::palette::toutes().len()
        );
    }
}
