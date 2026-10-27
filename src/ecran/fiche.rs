//! **Fiche d'un astre** : ce que montre le panneau de droite au clic.
//!
//! Le schéma d'interface le décrit comme un encart qui n'apparaît qu'une fois
//! une planète cliquée, et qui dit au moins « PLANÈTE / Habitable »
//! (`docs/conception/interface.md` §1.2, ⓔ).
//!
//! # Tout se déduit, rien ne se stocke
//!
//! Aucune des lignes ci-dessous n'est un champ posé sur la planète :
//! l'habitabilité se calcule à partir de la luminosité des étoiles et de la
//! distance orbitale, la distance se lit de la position, le rang de la place
//! dans le système. Un booléen `habitable` rangé sur la planète serait une
//! **seconde source** pour un fait que la géométrie décide déjà — la faute qui
//! est à l'origine de presque toutes les erreurs de ce dépôt
//! (`suivi/stations.md` §C.29, leçon 3).

use crate::astre::Categorie;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Verdict d'habitabilité d'un corps.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Habitabilite {
    /// Dans la zone d'eau liquide.
    Habitable,
    /// En deçà du bord interne.
    TropChaud,
    /// Au-delà du bord externe.
    TropFroid,
    /// La question ne se pose pas : une étoile, une ceinture, ou un système
    /// sans astre lumineux (un rémanent n'a pas de zone habitable pertinente).
    SansObjet,
}

impl Habitabilite {
    pub fn libelle(self) -> &'static str {
        match self {
            // « Zone tempérée », pas « habitable » : depuis le chantier
            // planètes, c'est le **type** de la planète qui dit si on peut y
            // vivre (`classe`) ; ceci ne dit que la distance à l'étoile.
            Habitabilite::Habitable => "ZONE TEMPEREE",
            Habitabilite::TropChaud => "TROP CHAUD",
            Habitabilite::TropFroid => "TROP FROID",
            Habitabilite::SansObjet => "-",
        }
    }
}

/// Ce qu'on affiche d'un astre.
pub struct Fiche {
    pub designation: String,
    pub categorie: Categorie,
    /// Rayon dans les unités du monde.
    pub rayon: f32,
    /// Distance à l'étoile, **en UA**. Pour une lune, celle de sa planète : une
    /// lune est chaude ou froide comme le corps qu'elle accompagne, pas comme
    /// sa propre orbite de quelques centièmes d'UA.
    pub distance_ua: f32,
    pub habitabilite: Habitabilite,
    /// **Le type de la planète, en contexte** (chantier planètes) : son type
    /// de base corrigé par ses étoiles et sa zone (`planete::contexte`).
    /// `None` pour une étoile, une ceinture, un engin, une comète.
    pub classe: Option<crate::planete::classe::ClassePlanete>,
    /// Son type de base, avant le contexte.
    pub classe_base: Option<crate::planete::classe::ClassePlanete>,
    /// Ce qui l'a fait descendre de son type de base (« PULSAR »…).
    pub raisons: Vec<String>,
    /// Un vaisseau **pilotable**, prêt à s'élancer — voir
    /// [`crate::astre::Astre::pilotable`]. Décide si le panneau montre le
    /// bouton LANCER (`docs/conception/trajectoires.md` §4 décision 8).
    pub pilotable: bool,
}

/// Rectangle du bouton LANCER (ou du statut « choisir une destination »),
/// ancré en **bas** du panneau `r` — pas sous la dernière ligne de texte,
/// dont le nombre varie selon la catégorie et l'habitabilité.
pub fn bouton_lancer(r: Rect) -> Rect {
    const MARGE: f32 = 10.0;
    const HAUTEUR: f32 = 28.0;
    Rect::new(r.x + MARGE, r.y + r.h - HAUTEUR - MARGE, r.w - 2.0 * MARGE, HAUTEUR)
}

/// Rectangle du panneau, **en bas à droite**.
///
/// Largeur bornée en fraction de l'écran mais aussi en pixels : sur un écran
/// très large, un panneau proportionnel deviendrait une colonne vide, et sur un
/// écran étroit il mangerait la vue.
///
/// En bas et non en haut : le haut à droite porte déjà les boutons d'outillage
/// (MENU, RETOUR), que le panneau recouvrait. Le bas à droite est le seul coin
/// que rien d'autre ne réclame — la barre de ressources tient le haut à gauche,
/// le sélecteur toute la gauche.
///
/// Il s'arrête **au-dessus** de la bande d'outils, dont il lit la hauteur au
/// lieu de la recopier.
pub fn rectangle(ecran: Vec2) -> Rect {
    const MARGE: f32 = 8.0;
    let l = (ecran.x * 0.22).clamp(180.0, 320.0);
    let h = (ecran.y * 0.42).clamp(150.0, 320.0);
    let bas = ecran.y - super::bandeau::BAS_OUTILS - MARGE;
    Rect::new(ecran.x - l - MARGE, (bas - h).max(MARGE), l, h)
}

/// Durée de l'ouverture/fermeture du panneau, en secondes. Un quart de
/// seconde : assez pour se voir glisser, pas assez pour se sentir attendre.
const DUREE_ANIM: f32 = 0.25;

/// Fait avancer la progression d'ouverture (`actuel`, 0 = fermé à 1 =
/// ouvert) vers sa cible (`ouvert`), à vitesse constante **dans le temps**
/// (`dt`), et la borne à `[0, 1]`.
///
/// Vitesse constante plutôt que fonction du temps écoulé : sans état à
/// tenir d'une frame à l'autre (juste `actuel`), la fermeture repart
/// toujours de la progression réellement atteinte, y compris si on rouvre
/// avant la fin d'une fermeture — pas de saut.
pub fn avancer_ouverture(actuel: f32, ouvert: bool, dt: f32) -> f32 {
    let cible = if ouvert { 1.0 } else { 0.0 };
    let pas = (dt / DUREE_ANIM).max(0.0);
    if actuel < cible {
        (actuel + pas).min(cible)
    } else {
        (actuel - pas).max(cible)
    }
}

/// Ease-in-out (« smoothstep ») : accélère puis ralentit, plutôt qu'une
/// vitesse constante qui donnerait un départ et un arrêt secs — c'est ce qui
/// fait qu'un glissement se sent « gracieux » plutôt que mécanique.
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Décalage horizontal du panneau pour une progression `t` (0 = totalement
/// hors écran à droite, 1 = à sa place de repos `rect_repos`).
///
/// Glisse depuis le bord droit de l'écran : le panneau part **entièrement**
/// hors champ (pas juste masqué au ras de son propre bord), pour que l'œil
/// le voie vraiment entrer plutôt que se matérialiser sur place.
pub fn decalage_horizontal(rect_repos: Rect, ecran_largeur: f32, t: f32) -> f32 {
    let hors_ecran = ecran_largeur - rect_repos.x;
    hors_ecran * (1.0 - ease_in_out(t))
}

/// Nom lisible d'une catégorie.
pub fn nom_categorie(c: Categorie) -> &'static str {
    match c {
        Categorie::Etoile => "ETOILE",
        Categorie::Planete => "PLANETE",
        Categorie::Lune => "LUNE",
        Categorie::Asteroide => "CEINTURE",
        Categorie::Comete => "COMETE",
        Categorie::Engin => "ENGIN",
    }
}

/// Établit la fiche de l'astre `idx`.
pub fn fiche(sys: &Systeme, idx: usize) -> Option<Fiche> {
    let categorie = sys.categorie_de(idx)?;
    // La distance qui compte est celle du corps **hôte** : pour une lune, sa
    // planète.
    let porteur = sys.parent_de(idx).unwrap_or(idx);
    let distance_ua = sys.position(porteur).length() / crate::etoile::UA;
    let hab = habitabilite(sys, categorie, distance_ua);
    let classe_base = sys.classe_planete_de(idx);
    let (classe, raisons) = match classe_base {
        Some(b) => {
            use crate::planete::contexte::{juger, Zone};
            let zone = match hab {
                Habitabilite::Habitable => Zone::Temperee,
                Habitabilite::TropChaud => Zone::TropChaud,
                Habitabilite::TropFroid => Zone::TropFroid,
                Habitabilite::SansObjet => Zone::SansObjet,
            };
            let (c, r) = juger(b, &sys.etoiles_exposees(porteur), zone);
            (Some(c), r)
        }
        None => (None, Vec::new()),
    };
    Some(Fiche {
        designation: sys.designation(idx),
        categorie,
        rayon: sys.rayon_de(idx).unwrap_or(0.0),
        distance_ua,
        habitabilite: hab,
        classe,
        classe_base,
        raisons,
        pilotable: sys.pilotable_de(idx),
    })
}

/// Le corps est-il dans la zone d'eau liquide ?
///
/// La luminosité est **sommée sur toutes les étoiles** : c'est la zone habitable
/// combinée, et c'est déjà ainsi que le rendu la trace pour les systèmes
/// multiples (`systeme/rendu.rs`). Une seule source pour la même grandeur.
fn habitabilite(sys: &Systeme, categorie: Categorie, distance_ua: f32) -> Habitabilite {
    if !matches!(categorie, Categorie::Planete | Categorie::Lune) {
        return Habitabilite::SansObjet;
    }
    let l = sys.luminosite_totale();
    if l <= 0.0 {
        // Aucun astre lumineux : rémanent, ou système sans étoile. Répondre
        // « trop froid » serait une affirmation qu'on ne peut pas soutenir.
        return Habitabilite::SansObjet;
    }
    let (interne, externe) = crate::etoile::zone_habitable(l);
    if distance_ua < interne {
        Habitabilite::TropChaud
    } else if distance_ua > externe {
        Habitabilite::TropFroid
    } else {
        Habitabilite::Habitable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astre::{Astre, CameraInfo, CorpsBase};

    // Corps d'essai : aucun test ne peut bâtir un vrai système, `genese` tirant
    // ses aléas par `macroquad::rand` (`conception/interface.md` §5.1 bis).
    struct CorpsEssai {
        base: CorpsBase,
        cat: Categorie,
        parent: Option<usize>,
        lum: Option<f32>,
    }

    impl Astre for CorpsEssai {
        fn categorie(&self) -> Categorie {
            self.cat
        }
        fn corps(&self) -> &CorpsBase {
            &self.base
        }
        fn corps_mut(&mut self) -> &mut CorpsBase {
            &mut self.base
        }
        fn parent(&self) -> Option<usize> {
            self.parent
        }
        fn luminosite(&self) -> Option<f32> {
            self.lum
        }
        fn update(&mut self, _dt: f32) {}
        fn draw(&mut self, _cam: &CameraInfo) {}
    }

    fn poser(sys: &mut Systeme, cat: Categorie, ua: f32, parent: Option<usize>, lum: Option<f32>) -> usize {
        sys.ajouter(Box::new(CorpsEssai {
            base: CorpsBase::new(vec3(ua * crate::etoile::UA, 0.0, 0.0), 1.0, 1.0),
            cat,
            parent,
            lum,
        }))
    }

    /// Une étoile de luminosité solaire à l'origine.
    fn avec_soleil() -> Systeme {
        let mut sys = Systeme::new();
        poser(&mut sys, Categorie::Etoile, 0.0, None, Some(1.0));
        sys
    }

    // **Le cœur de l'étape** : l'habitabilité se déduit de la distance et de la
    // luminosité. Sondée des deux côtés de chaque borne, et non en un seul
    // point — une implémentation qui répondrait toujours « habitable » passerait
    // un test à un seul échantillon.
    #[test]
    fn lhabitabilite_se_deduit_de_la_distance() {
        let mut sys = avec_soleil();
        let (interne, externe) = crate::etoile::zone_habitable(1.0);
        assert!(interne < 1.0 && 1.0 < externe, "la Terre doit être dans la zone : {interne}–{externe}");

        let cas = [
            (interne * 0.5, Habitabilite::TropChaud),
            (interne * 0.99, Habitabilite::TropChaud),
            (interne * 1.01, Habitabilite::Habitable),
            (1.0, Habitabilite::Habitable),
            (externe * 0.99, Habitabilite::Habitable),
            (externe * 1.01, Habitabilite::TropFroid),
            (externe * 4.0, Habitabilite::TropFroid),
        ];
        for (ua, attendu) in cas {
            let i = poser(&mut sys, Categorie::Planete, ua, None, None);
            let f = fiche(&sys, i).expect("fiche");
            assert_eq!(f.habitabilite, attendu, "à {ua:.3} UA");
        }
    }

    // **La zone suit la luminosité.** Une étoile quatre fois plus lumineuse
    // repousse la zone d'un facteur √2 : ce qui était habitable devient trop
    // chaud. Sans ce test, une zone codée en dur (0,95–1,37 UA) passerait tout
    // ce qui précède.
    #[test]
    fn la_zone_se_deplace_avec_la_luminosite() {
        let mut faible = Systeme::new();
        poser(&mut faible, Categorie::Etoile, 0.0, None, Some(0.05));
        let a = poser(&mut faible, Categorie::Planete, 1.0, None, None);
        assert_eq!(fiche(&faible, a).unwrap().habitabilite, Habitabilite::TropFroid);

        let mut vive = Systeme::new();
        poser(&mut vive, Categorie::Etoile, 0.0, None, Some(16.0));
        let b = poser(&mut vive, Categorie::Planete, 1.0, None, None);
        assert_eq!(fiche(&vive, b).unwrap().habitabilite, Habitabilite::TropChaud);
    }

    // **Les luminosités s'additionnent** : c'est la zone habitable combinée des
    // systèmes multiples, et c'est déjà ainsi que le rendu la trace.
    //
    // ⚠️ Ce test a d'abord été écrit avec 0,3 par étoile, et il a rougi — mais
    // c'était **le test qui avait tort**, pas le code : à L = 0,6 le bord
    // externe tombe à 0,89 UA, donc 1 UA reste trop froid même à deux. Il faut
    // choisir des luminosités qui **encadrent réellement** la borne, sans quoi
    // on ne mesure que le hasard d'un seuil. À 0,5 chacune : seule, le bord
    // externe est à 0,81 UA (trop froid) ; à deux, L = 1 et la zone couvre 1 UA.
    #[test]
    fn les_etoiles_multiples_additionnent_leur_lumiere() {
        const CHACUNE: f32 = 0.5;
        // Le scénario est vérifié avant d'être joué : sans ça, un changement de
        // la formule rendrait le test vert sans qu'il ne teste plus rien.
        assert!(crate::etoile::zone_habitable(CHACUNE).1 < 1.0, "seule, elle doit laisser 1 UA au froid");
        let (i2, e2) = crate::etoile::zone_habitable(2.0 * CHACUNE);
        assert!(i2 < 1.0 && 1.0 < e2, "à deux, 1 UA doit tomber dans la zone");

        let mut une = Systeme::new();
        poser(&mut une, Categorie::Etoile, 0.0, None, Some(CHACUNE));
        let a = poser(&mut une, Categorie::Planete, 1.0, None, None);

        let mut deux = Systeme::new();
        poser(&mut deux, Categorie::Etoile, 0.0, None, Some(CHACUNE));
        poser(&mut deux, Categorie::Etoile, 0.0, None, Some(CHACUNE));
        let b = poser(&mut deux, Categorie::Planete, 1.0, None, None);

        assert_eq!(fiche(&une, a).unwrap().habitabilite, Habitabilite::TropFroid);
        assert_eq!(fiche(&deux, b).unwrap().habitabilite, Habitabilite::Habitable);
    }

    // **Une lune hérite de la distance de sa planète**, pas de la sienne. Sa
    // propre orbite fait quelques centièmes d'UA : mesurée depuis l'étoile, une
    // lune de Jupiter serait jugée sur 5,2 UA — juste — mais une lune posée près
    // de l'origine du monde le serait sur presque zéro, donc « trop chaud ».
    #[test]
    fn une_lune_herite_de_la_distance_de_sa_planete() {
        let mut sys = avec_soleil();
        let planete = poser(&mut sys, Categorie::Planete, 1.0, None, None);
        // La lune est placée **à 0,02 UA de l'origine** : si on la mesurait chez
        // elle, elle serait brûlante.
        let lune = poser(&mut sys, Categorie::Lune, 0.02, Some(planete), None);
        let f = fiche(&sys, lune).unwrap();
        assert_eq!(f.habitabilite, Habitabilite::Habitable, "la lune suit sa planète");
        assert!((f.distance_ua - 1.0).abs() < 1e-3, "distance affichée : {}", f.distance_ua);
    }

    // Étoiles et ceintures n'ont pas d'habitabilité, et un système **sans
    // lumière** non plus : répondre « trop froid » serait affirmer ce qu'on ne
    // peut pas soutenir.
    #[test]
    fn sans_objet_quand_la_question_ne_se_pose_pas() {
        let mut sys = avec_soleil();
        let etoile = 0;
        let ceinture = poser(&mut sys, Categorie::Asteroide, 3.0, None, None);
        assert_eq!(fiche(&sys, etoile).unwrap().habitabilite, Habitabilite::SansObjet);
        assert_eq!(fiche(&sys, ceinture).unwrap().habitabilite, Habitabilite::SansObjet);

        // Système sans astre lumineux (rémanent) : la planète est bien là, mais
        // la question n'a pas de réponse.
        let mut noir = Systeme::new();
        poser(&mut noir, Categorie::Etoile, 0.0, None, None);
        let p = poser(&mut noir, Categorie::Planete, 1.0, None, None);
        assert_eq!(fiche(&noir, p).unwrap().habitabilite, Habitabilite::SansObjet);
    }

    // Un index hors bornes ne rend rien — et surtout ne panique pas : le
    // panneau garde un index d'une frame à l'autre, et le système peut changer
    // sous lui (touche G, chargement d'un preset).
    #[test]
    fn un_index_invalide_ne_donne_pas_de_fiche() {
        let sys = avec_soleil();
        assert!(fiche(&sys, 9999).is_none());
    }

    // La fiche lit vraiment `pilotable` à travers `Systeme` — pas figé à
    // `false` : sans quoi le bouton LANCER ne s'afficherait jamais, quel que
    // soit l'astre.
    #[test]
    fn la_fiche_reporte_le_pilotable_de_lastre() {
        let mut sys = avec_soleil();
        let ordinaire = poser(&mut sys, Categorie::Engin, 1.0, None, None);
        assert!(!fiche(&sys, ordinaire).unwrap().pilotable);

        struct VaisseauEssai {
            base: CorpsBase,
        }
        impl Astre for VaisseauEssai {
            fn categorie(&self) -> Categorie {
                Categorie::Engin
            }
            fn corps(&self) -> &CorpsBase {
                &self.base
            }
            fn corps_mut(&mut self) -> &mut CorpsBase {
                &mut self.base
            }
            fn draw(&mut self, _cam: &CameraInfo) {}
            fn update(&mut self, _dt: f32) {}
            fn pilotable(&self) -> bool {
                true
            }
        }
        let idx = sys.ajouter(Box::new(VaisseauEssai { base: CorpsBase::new(Vec3::ZERO, 1.0, 1.0) }));
        assert!(fiche(&sys, idx).unwrap().pilotable);
    }

    // Le bouton LANCER tient dans le panneau, pas au-delà de son bord bas.
    #[test]
    fn le_bouton_lancer_tient_dans_le_panneau() {
        let r = Rect::new(700.0, 100.0, 250.0, 300.0);
        let b = bouton_lancer(r);
        assert!(b.x >= r.x && b.x + b.w <= r.x + r.w, "déborde horizontalement");
        assert!(b.y >= r.y && b.y + b.h <= r.y + r.h, "déborde du panneau");
    }

    // Le panneau tient **dans l'écran**, ne recouvre ni la colonne de gauche,
    // ni le bandeau de ressources, ni la bande d'outils du bas. Chacun de ces
    // recouvrements ferait se disputer les clics — et c'est précisément ce qui
    // avait été signalé à l'écran.
    #[test]
    fn le_panneau_ne_recouvre_aucune_autre_zone() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1920.0, 1080.0), (3840.0, 2160.0)] {
            let e = vec2(l, h);
            let r = rectangle(e);
            assert!(r.x >= 0.0 && r.x + r.w <= l + 1e-3, "{l}x{h} : déborde à droite");
            assert!(r.y >= 0.0 && r.y + r.h <= h + 1e-3, "{l}x{h} : déborde en bas");
            assert!(r.x > l / 10.0, "{l}x{h} : le panneau mord sur la colonne");
            assert!(r.w >= 180.0, "{l}x{h} : panneau trop étroit ({})", r.w);
            // **En bas** : au-dessus de la bande d'outils, et dans la moitié
            // basse de l'écran dès qu'il y a la place.
            let strip = super::super::bandeau::strip_outils(e);
            assert!(
                r.y + r.h <= strip.y + 1e-3,
                "{l}x{h} : le panneau descend sur la bande d'outils"
            );
            // Et il ne remonte pas dans le bandeau de ressources.
            let b = super::super::bandeau::rectangle(e);
            let chevauche_bandeau = r.x < b.x + b.w && b.x < r.x + r.w
                && r.y < b.y + b.h && b.y < r.y + r.h;
            assert!(!chevauche_bandeau, "{l}x{h} : le panneau recouvre la barre de ressources");
        }
    }

    // Chaque catégorie a un libellé, et **aucun n'est vide** : le panneau
    // afficherait une ligne blanche.
    #[test]
    fn chaque_categorie_et_verdict_a_un_libelle() {
        for c in [Categorie::Etoile, Categorie::Planete, Categorie::Lune, Categorie::Asteroide, Categorie::Comete, Categorie::Engin] {
            assert!(!nom_categorie(c).is_empty(), "{c:?}");
        }
        for h in [Habitabilite::Habitable, Habitabilite::TropChaud, Habitabilite::TropFroid, Habitabilite::SansObjet] {
            assert!(!h.libelle().is_empty(), "{h:?}");
        }
    }

    // **La progression avance vers sa cible, à vitesse constante dans le
    // temps** — et jamais au-delà : sans le `.min`/`.max`, une frame lente
    // (un `dt` généreux) la ferait dépasser 1 ou passer sous 0.
    #[test]
    fn la_progression_avance_vers_sa_cible_sans_la_depasser() {
        let mi_course = avancer_ouverture(0.0, true, DUREE_ANIM * 0.5);
        assert!((mi_course - 0.5).abs() < 1e-4, "à mi-durée : {mi_course}");
        assert_eq!(avancer_ouverture(0.9, true, DUREE_ANIM * 10.0), 1.0, "ne doit pas dépasser 1");
        assert_eq!(avancer_ouverture(0.1, false, DUREE_ANIM * 10.0), 0.0, "ne doit pas dépasser 0");
    }

    // **Rouvrir avant la fin d'une fermeture ne fait pas sauter la
    // progression** : elle repart de là où elle en est réellement, pas de 0
    // ni de 1 — sinon le panneau sauterait visiblement au lieu de glisser.
    #[test]
    fn changer_de_cible_en_cours_de_route_ne_fait_pas_sauter() {
        let mi_fermeture = avancer_ouverture(1.0, false, DUREE_ANIM * 0.5); // ~0.5, en fermeture
        let reouvert = avancer_ouverture(mi_fermeture, true, 0.0); // dt nul : lecture seule
        assert!((reouvert - mi_fermeture).abs() < 1e-6, "un dt nul ne doit rien bouger");
    }

    // **`ease_in_out` est symétrique et va bien de 0 à 1** — sans quoi le
    // panneau n'atteindrait jamais tout à fait sa place de repos, ou
    // partirait déjà décalé.
    #[test]
    fn ease_in_out_va_de_0_a_1_et_reste_symetrique() {
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-4, "devrait être symétrique au milieu");
        // Hors bornes : ramené dans [0, 1], jamais une extrapolation folle.
        assert_eq!(ease_in_out(-1.0), 0.0);
        assert_eq!(ease_in_out(2.0), 1.0);
    }

    // **`ease_in_out` accélère puis ralentit** : la progression au premier
    // quart doit être plus lente qu'une ligne droite, celle du dernier quart
    // aussi — sinon ce ne serait qu'une identité déguisée, pas un
    // assouplissement.
    #[test]
    fn ease_in_out_accelere_puis_ralentit() {
        assert!(ease_in_out(0.25) < 0.25, "devrait démarrer plus lentement qu'une ligne droite");
        assert!(ease_in_out(0.75) > 0.75, "devrait ralentir avant l'arrivée");
    }

    // **Le panneau part entièrement hors écran à t=0**, et à sa place de
    // repos exacte à t=1 — les deux bornes qui font tout le glissement.
    #[test]
    fn le_decalage_va_de_hors_ecran_a_la_place_de_repos() {
        let repos = Rect::new(700.0, 100.0, 250.0, 300.0);
        let largeur_ecran = 1000.0_f32;
        let d0 = decalage_horizontal(repos, largeur_ecran, 0.0);
        assert!((repos.x + d0 - largeur_ecran).abs() < 1e-3, "à t=0, le panneau doit commencer hors écran");
        let d1 = decalage_horizontal(repos, largeur_ecran, 1.0);
        assert_eq!(d1, 0.0, "à t=1, le panneau doit être à sa place de repos");
    }
}
