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
    Soleil { rayon: f32, couleur: Vec3, lumi: f32, nom: String, couronne: f32 },
    Planete { rayon: f32, app: Box<Apparence> },
}

impl Specimen {
    /// Tire un specimen au hasard. `forcer` : Some(true)=soleil, Some(false)=planète.
    fn aleatoire(forcer: Option<bool>) -> Self {
        let soleil = forcer.unwrap_or(gen_range(0.0_f32, 1.0) < 0.5);
        if soleil {
            let p = ProfilEtoile::aleatoire();
            Specimen::Soleil {
                rayon: p.rayon,
                couleur: p.couleur,
                lumi: p.luminosite,
                nom: p.nom().to_string(),
                couronne: p.couronne,
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
        }
    }

    /// **Un gabarit d'étoile particulière**, pris à sa source
    /// (`etoile::particuliere`) au lieu d'être recopié : le catalogue des
    /// types exotiques n'a qu'un seul endroit où vivre.
    ///
    /// La température est prise au milieu de sa plage plutôt que tirée : une
    /// vue de travail doit redonner deux fois la même image.
    fn particulier(i: usize) -> Self {
        let types = types_particuliers();
        let (tmin, tmax, rayon, lumi, nom, couronne) = types[i % types.len()];
        Specimen::Soleil {
            rayon,
            couleur: crate::etoile::couleur_corps_noir((tmin + tmax) * 0.5),
            lumi,
            nom: nom.to_string(),
            couronne,
        }
    }

    fn label(&self) -> String {
        match self {
            Specimen::Soleil { nom, .. } => format!("ETOILE - {}", nom),
            Specimen::Planete { .. } => "PLANETE".to_string(),
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
fn types_particuliers() -> Vec<crate::etoile::Gabarit> {
    let mut out: Vec<crate::etoile::Gabarit> = Vec::new();
    for k in 0..200 {
        let g = crate::etoile::particuliere(k as f32 / 200.0);
        if !out.iter().any(|d| d.4 == g.4) {
            out.push(g);
        }
    }
    out
}

/// Construit un système à un seul astre, centré à l'origine.
fn batir(spec: &Specimen) -> Systeme {
    let mut sys = Systeme::new();
    match spec {
        Specimen::Soleil { rayon, couleur, lumi, couronne, .. } => {
            let s = Soleil::new(Vec3::ZERO, *rayon, *couleur, *lumi);
            // ⚠️ **La même source que la genèse et la galerie.** Cette vue
            // tenait une troisième copie de la chaîne mode → constructeur,
            // arrêtée aux bras 1 à 4 : un trou noir (mode 5) ou une jeune
            // étoile à disque (mode 6) s'y affichait en étoile ordinaire,
            // sans que rien ne le dise. Troisième occurrence du même défaut
            // sur ce chantier.
            sys.ajouter(Box::new(crate::stellaire::Variante::de_mode(*couronne).appliquer(s)));
        }
        Specimen::Planete { rayon, app } => {
            sys.ajouter(Box::new(Planete::new(Vec3::ZERO, Vec3::ZERO, *rayon, 1.0, **app, Vec::new())));
            // Pas d'étoile : lumière latérale de secours pour modeler le relief.
            sys.set_lumiere(vec3(6.0, 3.0, 6.0), Vec3::ONE);
        }
    }
    sys
}

/// Distance caméra adaptée à la taille du corps (couronne du soleil incluse).
fn dist_pour(spec: &Specimen) -> f32 {
    (spec.rayon() * 5.0 + 5.0).max(6.0)
}

/// Vue d'un astre isolé : pratique pour travailler le rendu d'un seul corps.
pub struct Objet {
    spec: Specimen,
    /// Rang du type particulier affiché, pour les faire défiler à la touche 3.
    particulier: usize,
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
            sys,
            fond: Fond::new(700),
            cam,
            rendu: RenduStandard::new(),
        }
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
        // Les types particuliers au défilé : trou noir en tête, puis pulsar,
        // étoile à neutrons, magnétar, Wolf-Rayet, supergéante bleue,
        // T Tauri et son disque.
        if is_key_pressed(KeyCode::Key3) {
            self.spec = Specimen::particulier(self.particulier);
            self.particulier += 1;
            self.sys = batir(&self.spec);
            self.cam.set_dist(dist_pour(&self.spec));
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

        crate::police::texte(
            &format!(
                "{}   |   {} FPS   G: aleatoire   1: soleil   2: planete   3: exotiques   P: pixel   R: shaders   Echap: menu",
                self.spec.label(),
                get_fps()
            ),
            12.0,
            24.0,
            18.0,
            WHITE,
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le défilé couvre tous les types particuliers, une fois chacun.**
    ///
    /// C'est la liste exacte de ce que `etoile::particuliere` sait produire,
    /// balayée et dédoublonnée — donc si un type apparaît au tirage, il est
    /// atteignable ici. Un type visible en jeu mais introuvable dans la vue
    /// de travail est un type qu'on ne pourra jamais juger.
    #[test]
    fn le_defile_couvre_tous_les_types_particuliers() {
        let types = types_particuliers();
        let noms: Vec<&str> = types.iter().map(|g| g.4).collect();
        assert!(noms.contains(&"Trou noir"), "le trou noir n'est pas au défilé : {noms:?}");
        assert!(noms.contains(&"T Tauri (jeune)"), "la jeune étoile à disque manque");
        // Aucun doublon : chaque pression de la touche montre autre chose.
        for (i, n) in noms.iter().enumerate() {
            assert!(!noms[..i].contains(n), "{n} apparaît deux fois");
        }
        assert!(types.len() >= 6, "seulement {} types au défilé", types.len());
    }

    /// **Le défilé boucle**, et ne sort jamais de la liste : on peut appuyer
    /// indéfiniment sur la touche.
    #[test]
    fn le_defile_boucle_sans_sortir_de_la_liste() {
        let n = types_particuliers().len();
        let nom_de = |i: usize| match Specimen::particulier(i) {
            Specimen::Soleil { nom, .. } => nom,
            Specimen::Planete { .. } => panic!("rang {i} : ce n'est pas une étoile"),
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
            assert!(Specimen::particulier(i).rayon() > 0.0, "rang {i} sans rayon");
        }
    }

    /// **Le trou noir est le premier du défilé**, puisque c'est lui qu'on
    /// vient chercher : le trouver en régénérant la skymap est laborieux, il
    /// ne sort que dans ~1 % des tirages.
    #[test]
    fn le_trou_noir_vient_en_premier() {
        match Specimen::particulier(0) {
            Specimen::Soleil { nom, couronne, .. } => {
                assert_eq!(nom, "Trou noir");
                // Et il porte bien son mode, sans quoi `batir` en ferait une
                // étoile ordinaire — le défaut que cette vue avait.
                assert_eq!(couronne, 5.0);
            }
            Specimen::Planete { .. } => panic!("le premier du défilé n'est pas une étoile"),
        }
    }
}
