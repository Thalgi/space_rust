use crate::camera::Camera;
use crate::etoile::ProfilEtoile;
use crate::fond::Fond;
use crate::genese::planete_aleatoire;
use crate::planete::{Apparence, Planete};
use crate::rendu::{Rendu, RenduStandard};
use crate::soleil::Soleil;
use crate::systeme::Systeme;
use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Le corps affiché, mémorisé pour pouvoir le reconstruire à l'identique
/// (hot-reload des shaders sur ce specimen précis).
///
/// `app` est **boxée** : `Apparence` pèse ~376 octets contre 48 pour la variante
/// étoile, et un enum coûte toujours sa plus grosse variante. Sans le Box, toute
/// copie de `Specimen` — y compris celle d'une étoile, qui n'en a que faire —
/// traînerait les 376 octets.
enum Specimen {
    Soleil {
        rayon: f32,
        couleur: Vec3,
        lumi: f32,
        nom: String,
        couronne: f32,
        /// Régime imposé, pour les trous noirs du défilé. `None` : il se
        /// dérive de la graine, comme en jeu.
        regime: Option<crate::disque::Accretion>,
    },
    Planete { rayon: f32, app: Box<Apparence> },
    /// Un **prototype en cours** (touche 4), par son rang dans
    /// `prototypes::prototypes()`.
    Prototype(usize),
}

impl Specimen {
    /// Tire un specimen au hasard. `forcer` : Some(true)=soleil, Some(false)=planète.
    fn aleatoire(forcer: Option<bool>) -> Self {
        let soleil = forcer.unwrap_or(gen_range(0.0_f32, 1.0) < 0.5);
        if soleil {
            let p = ProfilEtoile::aleatoire();
            if let Some(s) = Self::enveloppe_gazeuse(p.nom, p.temperature, p.rayon) {
                return s;
            }
            Specimen::Soleil {
                rayon: p.rayon,
                couleur: p.couleur,
                lumi: p.luminosite,
                nom: p.nom().to_string(),
                couronne: p.couronne,
                regime: None,
            }
        } else {
            let (rayon, app) = planete_aleatoire();
            Specimen::Planete { rayon, app: Box::new(app) }
        }
    }

    fn rayon(&self) -> f32 {
        match self {
            Specimen::Soleil { rayon, .. } => *rayon,
            Specimen::Planete { rayon, .. } => *rayon,
            // Le rayon de l'étoile qui le porte : c'est lui que mesure la
            // taille écran du bloc de debug.
            Specimen::Prototype(_) => 1.6,
        }
    }

    /// **Les types du tirage qui ne sont pas des photosphères.** `None` pour
    /// tous les autres.
    ///
    /// ⚠️ Aujourd'hui la naine brune, et elle seule. Le dépôt la rend en géante
    /// gazeuse — bandes, vortex, émission thermique — et la bâtir en `Soleil`
    /// ici lui collerait granulation et couronne. La vue de travail montrerait
    /// alors autre chose que le jeu, sur le seul type dont c'était justement le
    /// point. Le prédicat vit dans `etoile`, pas ici.
    fn enveloppe_gazeuse(nom: &str, temp: f32, rayon: f32) -> Option<Self> {
        crate::etoile::est_naine_brune(nom).then(|| Specimen::Planete {
            rayon,
            app: Box::new(crate::genese::preset_gazeuse(crate::genese::preset_naine_brune(temp))),
        })
    }

    /// **Un gabarit du défilé**, pris à sa source (`etoile::particuliere` et
    /// `etoile::ordinaire`) au lieu d'être recopié : le catalogue des types
    /// n'a qu'un seul endroit où vivre.
    ///
    /// La température est prise au milieu de sa plage plutôt que tirée : une
    /// vue de travail doit redonner deux fois la même image.
    fn du_defile(i: usize) -> Self {
        let types = types_au_defile();
        // **En tête du défilé : les types du chantier en cours** (`EN_TETE`),
        // au premier appui, puis tout le catalogue — où ils reviennent à leur
        // place. Demandé pour aller droit à ce qu'on travaille.
        let rang = i % (types.len() + EN_TETE.len());
        let (g, regime) = match EN_TETE.get(rang) {
            Some(nom) => *types.iter().find(|(g, _)| g.4 == *nom).expect("type en tête absent du défilé"),
            None => types[rang - EN_TETE.len()],
        };
        let (tmin, tmax, rayon, lumi, nom, couronne) = g;
        // Les types qui ne sont pas des photosphères se rendent autrement.
        if let Some(s) = Self::enveloppe_gazeuse(nom, (tmin + tmax) * 0.5, rayon) {
            return s;
        }
        // ⚠️ Le régime se nomme lui-même. Le nommer par son **rang** dans
        // la liste marchait tant que l'ordre ne bougeait pas — c'est-à-dire
        // jusqu'au premier qui l'aurait changé.
        // ⚠️ Même source que le jeu et la galerie : une carbonée tient sa
        // teinte d'un filtre moléculaire, pas du seul corps noir. Calculée
        // **avant** que le libellé du régime soit accolé au nom — le filtre se
        // reconnaît au nom du type, et « Trou noir (chaud actif) » n'en est pas
        // un.
        let couleur = crate::etoile::couleur_etoile(nom, (tmin + tmax) * 0.5);
        let nom = match regime {
            Some(r) => format!("{nom} ({})", r.libelle()),
            None => nom.to_string(),
        };
        Specimen::Soleil {
            rayon,
            couleur,
            lumi,
            nom,
            couronne,
            regime,
        }
    }

    fn label(&self) -> String {
        match self {
            Specimen::Soleil { nom, .. } => format!("ETOILE - {}", nom),
            Specimen::Planete { .. } => "PLANETE".to_string(),
            Specimen::Prototype(i) => match super::prototypes::prototypes().get(*i) {
                Some(p) => format!("PROTOTYPE - {}", p.nom),
                None => "PROTOTYPE - (retire)".to_string(),
            },
        }
    }
}

/// **Les types particuliers, un par type et dans l'ordre du tirage.**
///
/// Balaie `etoile::particuliere` et retire les doublons : c'est donc la
/// liste exacte des objets exotiques que le jeu sait produire, sans en
/// recopier un seul. Sert à les faire défiler en vue isolée — chercher un
/// trou noir en régénérant la skymap est laborieux, et il n'apparaît que
/// dans ~1 % des tirages.
fn types_particuliers() -> Vec<(crate::etoile::Gabarit, Option<crate::disque::Accretion>)> {
    use crate::disque::Accretion;
    let mut out: Vec<(crate::etoile::Gabarit, Option<Accretion>)> = Vec::new();
    for k in 0..200 {
        let g = crate::etoile::particuliere(k as f32 / 200.0);
        if out.iter().any(|(d, _)| d.4 == g.4) {
            continue;
        }
        // ⚠️ Le trou noir compte **pour tout le catalogue** : son régime se
        // dérive de la graine, donc en jeu on tombe sur l'un d'eux et il
        // faudrait régénérer pour voir les autres. Une vue de travail doit
        // pouvoir les montrer à la demande — y compris la cible concurrente,
        // qu'aucune graine n'atteint et qui ne se juge que là.
        if g.4 == "Trou noir" {
            for r in Accretion::catalogue() {
                out.push((g, Some(r)));
            }
        } else {
            out.push((g, None));
        }
    }
    out
}

/// **Types mis en tête du défilé** pendant le chantier qui les travaille :
/// aujourd'hui les jeunes étoiles du lot 12. Ils restent aussi à leur place
/// dans le catalogue. Le défilé et ses gardes en tiennent compte ; vide, il
/// redevient le catalogue seul.
const EN_TETE: &[&str] = &["T Tauri (jeune)"];

/// **Tout le catalogue d'étoiles au défilé** : les particulières d'abord —
/// c'est pour elles que la touche existe — puis les **ordinaires**.
///
/// ⚠️ Les ordinaires manquaient, et ça se paie exactement comme pour le trou
/// noir avant le lot 4 : les trois types du lot 6.1 ne se voyaient qu'en
/// régénérant des systèmes jusqu'à tomber dessus, une étoile sur douze.
/// « Laborieux », et c'est le mot que l'utilisateur avait employé.
///
/// ⚠️ **La vue isolée ne juge pas les tailles.** `dist_pour` cadre chaque corps
/// pour qu'il remplisse l'écran, donc une supergéante y paraît aussi grande
/// qu'une naine. Ce qu'on vient y juger est la **teinte** et la surface ; le
/// rayon et la luminosité se lisent en **chiffres** dans le bloc de debug.
/// Pour comparer des tailles, c'est un vrai système qu'il faut.
fn types_au_defile() -> Vec<(crate::etoile::Gabarit, Option<crate::disque::Accretion>)> {
    let mut out = types_particuliers();
    // `q = 0.5` : le milieu de la plage de rayon et de luminosité de chaque
    // type. Une vue de travail doit redonner deux fois la même image, donc pas
    // de tirage — même raison que la température prise au milieu de sa plage.
    for k in 0..400 {
        let g = crate::etoile::ordinaire(k as f32 / 400.0, 0.5);
        if out.iter().any(|(d, _)| d.4 == g.4) {
            continue;
        }
        out.push((g, None));
    }
    out
}

/// **Le bloc de debug du coin haut-gauche.**
///
/// La vue isolée sert à **travailler un rendu** : on veut voir d'un coup ce
/// qui est chargé, sous quel angle, et surtout **à quelle taille à l'écran**
/// — c'est elle qui décide de ce qui est lisible, et tout le chantier astres
/// s'est heurté à des effets invisibles sous ~10 px de rayon
/// (`astres_v2.md` §0).
///
/// Fonction **pure** : elle rend les lignes, elle ne les dessine pas. C'est
/// ce qui la rend testable, `police::texte` exigeant le contexte graphique.
/// ⚠️ Prend les **nombres** de la caméra, pas la caméra : `Camera::new`
/// exige le contexte graphique, et une fonction qui le réclame ne se teste
/// pas. Elle n'a de toute façon besoin que de ces quatre-là.
fn lignes_debug(
    spec: &Specimen,
    orientation: (f32, f32),
    dist: f32,
    zoom: f32,
    ecran: Vec2,
    px: f32,
) -> Vec<String> {
    let (yaw, pitch) = orientation;
    let mut l = vec![spec.label()];
    match spec {
        Specimen::Soleil { rayon, lumi, couronne, .. } => {
            let v = crate::stellaire::Variante::de_mode(*couronne);
            l.push(format!("  rayon {rayon:.2}   lumi {lumi:.2}"));
            l.push(format!("  couronne {} -> {}", *couronne as i32, v.nom()));
        }
        Specimen::Planete { rayon, .. } => {
            l.push(format!("  rayon {rayon:.3}"));
        }
        Specimen::Prototype(i) => {
            l.push(format!("  prototype {} / {}", i + 1, super::prototypes::prototypes().len()));
        }
    }
    l.push(format!(
        "CAM  yaw {:.0}°  pitch {:.0}°  dist {:.2}  zoom x{:.2}",
        yaw.to_degrees(),
        pitch.to_degrees(),
        dist,
        zoom
    ));
    // ⚠️ Rayon **physique** projeté : le plancher en pixels des étoiles
    // (`soleil::PX_MIN`) peut l'élargir au loin. En vue isolée la caméra est
    // proche, donc le plancher dort et le chiffre est exact.
    l.push(format!("ECRAN  rayon ~{px:.0} px   {:.0}x{:.0}", ecran.x, ecran.y));
    l
}

/// Construit un système à un seul astre, centré à l'origine.
fn batir(spec: &Specimen) -> Systeme {
    let mut sys = Systeme::new();
    match spec {
        Specimen::Soleil { rayon, couleur, lumi, couronne, regime, nom } => {
            let s = Soleil::new(Vec3::ZERO, *rayon, *couleur, *lumi);
            // ⚠️ **La même source que la genèse et la galerie.** Cette vue
            // tenait une troisième copie de la chaîne mode → constructeur,
            // arrêtée aux bras 1 à 4 : un trou noir (mode 5) ou une jeune
            // étoile à disque (mode 6) s'y affichait en étoile ordinaire,
            // sans que rien ne le dise. Troisième occurrence du même défaut
            // sur ce chantier.
            let s = crate::stellaire::Variante::de_mode(*couronne).appliquer(s, nom);
            // Régime impose (trous noirs du defile) : sinon il reste derive
            // de la graine, comme en jeu.
            let s = match regime {
                Some(r) => s.avec_regime(*r),
                None => s,
            };
            sys.ajouter(Box::new(s));
        }
        Specimen::Planete { rayon, app } => {
            sys.ajouter(Box::new(Planete::new(Vec3::ZERO, Vec3::ZERO, *rayon, 1.0, **app, Vec::new())));
            // Pas d'étoile : lumière latérale de secours pour modeler le relief.
            sys.set_lumiere(vec3(6.0, 3.0, 6.0), Vec3::ONE);
        }
        Specimen::Prototype(i) => {
            if let Some(p) = super::prototypes::prototypes().get(*i) {
                return (p.construire)();
            }
        }
    }
    sys
}

/// Distance caméra adaptée à la taille du corps (couronne du soleil incluse).
fn dist_pour(spec: &Specimen) -> f32 {
    if let Specimen::Prototype(i) = spec {
        if let Some(p) = super::prototypes::prototypes().get(*i) {
            return p.distance;
        }
    }
    (spec.rayon() * 5.0 + 5.0).max(6.0)
}

/// **Ce que fait la touche 4** : le prototype suivant, ou — s'il n'y en a
/// aucun en cours — le message qui le dit. Pur : la décision se teste.
fn touche_4(rang: usize, nb_prototypes: usize) -> Option<Specimen> {
    (nb_prototypes > 0).then(|| Specimen::Prototype(rang % nb_prototypes))
}

/// Vue d'un astre isolé : pratique pour travailler le rendu d'un seul corps.
pub struct Objet {
    spec: Specimen,
    /// Rang du type particulier affiché, pour les faire défiler à la touche 3.
    particulier: usize,
    /// Rang du prototype affiché (touche 4).
    prototype: usize,
    /// Le message « aucun prototype en cours » est ouvert.
    sans_prototype: bool,
    sys: Systeme,
    fond: Fond,
    cam: Camera,
    rendu: RenduStandard,
}

impl Objet {
    pub fn new() -> Self {
        let spec = Specimen::aleatoire(None);
        let sys = batir(&spec);
        let cam = Camera::new(dist_pour(&spec));
        Self {
            spec,
            particulier: 0,
            prototype: 0,
            sans_prototype: false,
            sys,
            fond: Fond::new(700),
            cam,
            rendu: RenduStandard::new(),
        }
    }

    /// **Le message « aucun prototype en cours »**, en console Minitel.
    /// Rend `true` quand on clique OK (ou qu'on valide à Entrée).
    fn message_sans_prototype(&self, ecran: Vec2) -> bool {
        let r = Rect::new(ecran.x * 0.5 - 200.0, ecran.y * 0.5 - 70.0, 400.0, 140.0);
        crate::ui::minitel_panel(r, "* MINITEL * PROTOTYPES");
        crate::police::texte("Aucun prototype en cours.", r.x + 20.0, r.y + 62.0, 20.0, Color::new(0.55, 1.0, 0.75, 1.0));
        let ok = Rect::new(r.x + r.w * 0.5 - 50.0, r.y + r.h - 46.0, 100.0, 32.0);
        let m = vec2(mouse_position().0, mouse_position().1);
        crate::ui::minitel_ligne(ok, "   OK", m);
        (is_mouse_button_pressed(MouseButton::Left) && ok.contains(m)) || is_key_pressed(KeyCode::Enter)
    }

    /// La vue isolée ouverte directement sur un prototype (`PROTO=<rang>`,
    /// avec `CAPTURE` : juger un essai sans main sur la souris).
    pub fn sur_prototype(rang: usize) -> Self {
        let mut o = Self::new();
        if let Some(spec) = touche_4(rang, super::prototypes::prototypes().len()) {
            o.prototype = rang + 1;
            o.spec = spec;
            o.sys = batir(&o.spec);
            o.cam.set_dist(dist_pour(&o.spec));
        }
        // `PITCH=<degrés>` : l'inclinaison de la caméra pour la capture — un
        // disque ne se juge pas seulement en plongée.
        if let Some(p) = std::env::var("PITCH").ok().and_then(|p| p.parse::<f32>().ok()) {
            o.cam.pitch = p.to_radians();
        }
        if let Some(y) = std::env::var("YAW").ok().and_then(|y| y.parse::<f32>().ok()) {
            o.cam.yaw = y.to_radians();
        }
        if let Some(d) = std::env::var("DIST").ok().and_then(|d| d.parse::<f32>().ok()) {
            o.cam.set_dist(d);
        }
        o
    }

    fn charger(&mut self, forcer: Option<bool>) {
        self.spec = Specimen::aleatoire(forcer);
        self.sys = batir(&self.spec);
        self.cam.set_dist(dist_pour(&self.spec));
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        let dt = get_frame_time().min(0.05);

        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::G) {
            self.charger(None);
        }
        if is_key_pressed(KeyCode::Key1) {
            self.charger(Some(true));
        }
        if is_key_pressed(KeyCode::Key2) {
            self.charger(Some(false));
        }
        // Le défilé : les particulières d'abord — trou noir et ses sept
        // régimes en tête, puis pulsar, étoile à neutrons, magnétar,
        // Wolf-Rayet, supergéante bleue, T Tauri et son disque — puis les
        // ordinaires, séquence principale comprise.
        if is_key_pressed(KeyCode::Key3) {
            self.spec = Specimen::du_defile(self.particulier);
            self.particulier += 1;
            self.sys = batir(&self.spec);
            self.cam.set_dist(dist_pour(&self.spec));
        }
        // **Les prototypes en cours** (`prototypes.rs`) : un par appui. Aucun
        // en cours : un message le dit, et son OK fait ce que fait la touche 1.
        if is_key_pressed(KeyCode::Key4) {
            match touche_4(self.prototype, super::prototypes::prototypes().len()) {
                Some(spec) => {
                    self.prototype += 1;
                    self.spec = spec;
                    self.sys = batir(&self.spec);
                    self.cam.set_dist(dist_pour(&self.spec));
                }
                None => self.sans_prototype = true,
            }
        }
        if is_key_pressed(KeyCode::P) {
            self.rendu.toggle_pixel();
        }
        if is_key_pressed(KeyCode::R) {
            crate::planete::vider_cache_materials();
            crate::disque::vider_cache_materials();
            crate::soleil::vider_cache_materials();
            self.sys = batir(&self.spec);
            self.fond.recharger_material();
        }

        self.cam.input_orbite(false);
        // Vue objet : pas de multiplicateur, les deux horloges coïncident.
        self.sys.update(dt, crate::systeme::DtAnimation(dt));

        let aspect = screen_width() / screen_height();
        let (cam_info, cam3d) = self.cam.construire(Vec3::ZERO, aspect);
        self.rendu
            .rendre(cam3d, &cam_info, &mut self.fond, &mut self.sys, Default::default());

        let ecran = vec2(screen_width(), screen_height());
        let px = crate::impostor::px_rayon(
            self.spec.rayon(),
            self.cam.dist,
            crate::impostor::viewport_h(),
        );
        let debug = lignes_debug(
            &self.spec,
            (self.cam.yaw, self.cam.pitch),
            self.cam.dist,
            self.cam.zoom(),
            ecran,
            px,
        );
        for (i, ligne) in debug.iter().enumerate() {
            crate::police::texte(ligne, 12.0, 24.0 + i as f32 * 20.0, 18.0, WHITE);
        }
        if self.sans_prototype && self.message_sans_prototype(ecran) {
            self.sans_prototype = false;
            self.charger(Some(true)); // ce que fait la touche 1
        }
        crate::police::texte(
            &format!(
                "{} FPS   rendu: {:?}   G: aleatoire   1: soleil   2: planete   3: exotiques   4: prototypes   P: pixel   R: shaders   Echap: menu",
                get_fps(),
                crate::reglages::mode_rendu()
            ),
            12.0,
            ecran.y - 16.0,
            18.0,
            Color::new(0.75, 0.75, 0.8, 1.0),
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La touche 4 parcourt les prototypes, et dit quand il n'y en a pas.**
    /// Elle boucle comme le défilé ; sans prototype, elle ne rend rien — c'est
    /// le message qui s'ouvre, pas un spécimen vide.
    #[test]
    fn la_touche_4_parcourt_les_prototypes_ou_le_dit() {
        assert!(touche_4(0, 0).is_none(), "aucun prototype : la touche doit ouvrir le message");
        assert!(touche_4(7, 0).is_none());
        for (rang, n, attendu) in [(0, 4, 0), (3, 4, 3), (4, 4, 0), (9, 4, 1), (5, 1, 0)] {
            match touche_4(rang, n) {
                Some(Specimen::Prototype(i)) => assert_eq!(i, attendu, "rang {rang} sur {n}"),
                _ => panic!("rang {rang} sur {n} : pas de prototype rendu"),
            }
        }
    }

    /// **Le défilé couvre tous les types particuliers, une fois chacun.**
    ///
    /// C'est la liste exacte de ce que `etoile::particuliere` sait produire,
    /// balayée et dédoublonnée — donc si un type apparaît au tirage, il est
    /// atteignable ici. Un type visible en jeu mais introuvable dans la vue
    /// de travail est un type qu'on ne pourra jamais juger.
    #[test]
    fn le_defile_couvre_tous_les_types_particuliers() {
        let types = types_particuliers();
        let noms: Vec<&str> = types.iter().map(|(g, _)| g.4).collect();
        assert!(noms.contains(&"Trou noir"), "le trou noir n'est pas au défilé : {noms:?}");
        assert!(noms.contains(&"T Tauri (jeune)"), "la jeune étoile à disque manque");
        // Aucun doublon — **sauf le trou noir**, qui compte pour tout son
        // catalogue : son régime se dérive de la graine, donc le défilé est
        // le seul endroit où l'on puisse les demander tous — et le seul, tout
        // court, pour la cible visuelle qu'aucune graine n'atteint.
        for (i, (g, r)) in types.iter().enumerate() {
            if r.is_some() {
                continue;
            }
            assert!(!noms[..i].contains(&g.4), "{} apparaît deux fois", g.4);
        }
        let vus: Vec<String> = types.iter().filter_map(|(_, r)| r.map(|r| r.libelle())).collect();
        let attendus: Vec<String> =
            crate::disque::Accretion::catalogue().iter().map(|a| a.libelle()).collect();
        assert_eq!(vus, attendus, "le défilé ne montre pas le catalogue du trou noir");
        assert!(types.len() >= 6, "seulement {} entrées au défilé", types.len());
    }

    /// **Le défilé boucle**, et ne sort jamais de la liste : on peut appuyer
    /// indéfiniment sur la touche.
    #[test]
    fn le_defile_boucle_sans_sortir_de_la_liste() {
        let n = types_au_defile().len() + EN_TETE.len();
        let nom_de = |i: usize| match Specimen::du_defile(i) {
            Specimen::Soleil { nom, .. } => nom,
            Specimen::Planete { .. } | Specimen::Prototype(_) => {
                panic!("rang {i} : ce n'est pas une étoile")
            }
        };
        // ⚠️ **Il BOUCLE**, il ne s'arrête pas au dernier. Vérifier seulement
        // que rien ne panique laissait passer un `min` à la place du modulo :
        // la touche devenait alors inerte une fois arrivée au bout, sans rien
        // qui le dise.
        assert_eq!(nom_de(n), nom_de(0), "le défilé ne revient pas au début");
        assert_eq!(nom_de(n + 1), nom_de(1));
        assert_eq!(nom_de(10_000), nom_de(10_000 % n));
        // Et il ne sort jamais de la liste, quel que soit le rang.
        for i in [0_usize, 1, n - 1, n, n + 3, 10_000] {
            assert!(!nom_de(i).is_empty(), "rang {i} sans nom");
            assert!(Specimen::du_defile(i).rayon() > 0.0, "rang {i} sans rayon");
        }
    }

    /// **Le défilé couvre aussi les types ordinaires.**
    ///
    /// ⚠️ Il ne portait que les particulières, et les trois types du lot 6.1 ne
    /// se voyaient donc qu'en régénérant des systèmes — une étoile sur douze.
    /// C'est le même défaut que le trou noir avant le lot 4 : un rendu qui
    /// existe et qu'on ne peut pas aller voir. Les deux tables sont balayées à
    /// leur source, aucune liste recopiée ici.
    #[test]
    fn le_defile_couvre_aussi_les_types_ordinaires() {
        let noms: Vec<&str> = types_au_defile().iter().map(|(g, _)| g.4).collect();
        for k in 0..400 {
            let attendu = crate::etoile::ordinaire(k as f32 / 400.0, 0.5).4;
            assert!(noms.contains(&attendu), "« {attendu} » n'est pas au défilé : {noms:?}");
        }
        // Et les particulières restent en tête : c'est pour elles que la touche
        // existe, et le trou noir doit rester au premier appui.
        assert_eq!(noms[0], "Trou noir");
    }

    /// **Le bloc de debug dit ce qu'on a sous les yeux.**
    ///
    /// Une vue de travail dont l'affichage ment est pire qu'une vue sans
    /// affichage : on règle à l'aveugle en croyant savoir. Les trois faits
    /// qui comptent — quel astre, sous quel angle, à quelle taille écran —
    /// doivent être là et être justes.
    #[test]
    fn le_debug_dit_ce_qu_on_regarde() {
        let spec = Specimen::du_defile(EN_TETE.len()); // le trou noir, premier type
        let l = lignes_debug(&spec, (0.8, 0.5), 8.0, 1.0, vec2(1920.0, 1080.0), 142.0);
        let tout = l.join("\n");
        assert!(tout.contains("Trou noir"), "le type manque : {tout}");
        // ⚠️ La variante, pas seulement le numéro de mode : c'est elle qui
        // dit si le rendu attendu est bien celui qu'on va voir.
        assert!(tout.contains("trou noir"), "la variante manque : {tout}");
        assert!(tout.contains("142"), "la taille écran manque : {tout}");
        assert!(tout.contains("1920"), "la résolution manque : {tout}");
        assert!(l.len() >= 4, "bloc trop pauvre : {l:?}");
    }

    /// **L'orientation est en degrés, pas en radians.** Un angle en radians
    /// ne se lit pas d'un coup d'œil, et c'est un réglage qu'on compare à
    /// l'œil entre deux captures.
    #[test]
    fn l_orientation_est_lisible() {
        use std::f32::consts::{FRAC_PI_2, PI};
        let tout = lignes_debug(
            &Specimen::du_defile(0),
            (PI, FRAC_PI_2), // 180° et 90°
            10.0,
            1.0,
            vec2(800.0, 600.0),
            10.0,
        )
        .join("\n");
        assert!(tout.contains("180"), "yaw pas en degrés : {tout}");
        assert!(tout.contains("90"), "pitch pas en degrés : {tout}");
    }

    /// **Le trou noir est le premier type du défilé**, puisque c'est lui qu'on
    /// vient chercher : le trouver en régénérant la skymap est laborieux, il
    /// ne sort que dans ~1 % des tirages. Seuls les types du chantier en
    /// cours (`EN_TETE`) passent avant lui.
    #[test]
    fn le_trou_noir_vient_en_premier() {
        // Les trois régimes ouvrent le défilé, dans l'ordre.
        for (rang, attendu) in [(0, "endormi"), (1, "actif"), (2, "vorace")] {
            match Specimen::du_defile(EN_TETE.len() + rang) {
                Specimen::Soleil { nom, couronne, regime, .. } => {
                    assert!(nom.starts_with("Trou noir"), "rang {rang} : {nom}");
                    assert!(nom.contains(attendu), "rang {rang} : attendu {attendu}, eu {nom}");
                    // Et il porte bien son mode, sans quoi `batir` en ferait
                    // une étoile ordinaire — le défaut que cette vue avait.
                    assert_eq!(couronne, 5.0);
                    // ⚠️ Et son régime est **imposé** : sans ça il se
                    // dériverait de la graine, et les trois cellules
                    // montreraient la même chose au hasard.
                    assert!(regime.is_some(), "rang {rang} : régime laissé au hasard");
                }
                Specimen::Planete { .. } | Specimen::Prototype(_) => {
                    panic!("rang {rang} : ce n'est pas une étoile")
                }
            }
        }
    }
}
