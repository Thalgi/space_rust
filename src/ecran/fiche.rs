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
            Habitabilite::Habitable => "HABITABLE",
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
}

/// Rectangle du panneau, à droite de l'écran.
///
/// Largeur bornée en **fraction** de l'écran mais aussi en pixels : sur un
/// écran très large, un panneau proportionnel deviendrait une colonne vide, et
/// sur un écran étroit il mangerait la vue.
pub fn rectangle(ecran: Vec2) -> Rect {
    const MARGE: f32 = 8.0;
    let l = (ecran.x * 0.22).clamp(180.0, 320.0);
    let h = (ecran.y * 0.42).clamp(150.0, 320.0);
    Rect::new(ecran.x - l - MARGE, MARGE, l, h)
}

/// Nom lisible d'une catégorie.
pub fn nom_categorie(c: Categorie) -> &'static str {
    match c {
        Categorie::Etoile => "ETOILE",
        Categorie::Planete => "PLANETE",
        Categorie::Lune => "LUNE",
        Categorie::Asteroide => "CEINTURE",
        Categorie::Comete => "COMETE",
    }
}

/// Établit la fiche de l'astre `idx`.
pub fn fiche(sys: &Systeme, idx: usize) -> Option<Fiche> {
    let categorie = sys.categorie_de(idx)?;
    // La distance qui compte est celle du corps **hôte** : pour une lune, sa
    // planète.
    let porteur = sys.parent_de(idx).unwrap_or(idx);
    let distance_ua = sys.position(porteur).length() / crate::etoile::UA;
    Some(Fiche {
        designation: sys.designation(idx),
        categorie,
        rayon: sys.rayon_de(idx).unwrap_or(0.0),
        distance_ua,
        habitabilite: habitabilite(sys, categorie, distance_ua),
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

    // Le panneau tient **dans l'écran**, à toutes les tailles, et ne recouvre
    // pas la colonne de gauche : les deux se disputeraient les clics.
    #[test]
    fn le_panneau_tient_dans_lecran_sans_toucher_la_colonne() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1920.0, 1080.0), (3840.0, 2160.0)] {
            let e = vec2(l, h);
            let r = rectangle(e);
            assert!(r.x >= 0.0 && r.x + r.w <= l + 1e-3, "{l}x{h} : déborde à droite");
            assert!(r.y >= 0.0 && r.y + r.h <= h + 1e-3, "{l}x{h} : déborde en bas");
            // La colonne de gauche fait un dixième de la largeur : le panneau
            // doit commencer bien après.
            assert!(r.x > l / 10.0, "{l}x{h} : le panneau mord sur la colonne");
            // Et il reste lisible : un panneau de 40 px ne dirait rien.
            assert!(r.w >= 180.0, "{l}x{h} : panneau trop étroit ({})", r.w);
        }
    }

    // Chaque catégorie a un libellé, et **aucun n'est vide** : le panneau
    // afficherait une ligne blanche.
    #[test]
    fn chaque_categorie_et_verdict_a_un_libelle() {
        for c in [Categorie::Etoile, Categorie::Planete, Categorie::Lune, Categorie::Asteroide, Categorie::Comete] {
            assert!(!nom_categorie(c).is_empty(), "{c:?}");
        }
        for h in [Habitabilite::Habitable, Habitabilite::TropChaud, Habitabilite::TropFroid, Habitabilite::SansObjet] {
            assert!(!h.libelle().is_empty(), "{h:?}");
        }
    }
}
