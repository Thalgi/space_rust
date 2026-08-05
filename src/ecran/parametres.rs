//! **Écran des paramètres**, atteignable depuis l'accueil.
//!
//! Six réglages : affichage et taille de fenêtre, puis le pipeline de rendu et
//! ses trois réglages de pixel art (palette, tramage, saturation), qui se
//! **grisent** hors du mode palette. Les entrées sont une **liste**, pas des
//! rectangles posés un par un, si bien qu'en ajouter une tient en une ligne.
//!
//! Le modèle vit dans [`crate::reglages`] et se teste ; ici il n'y a que « où
//! le rectangle se pose ».

use crate::reglages::Reglages;
use crate::ui::minitel_ligne;
use macroquad::prelude::*;

/// Ce qu'une ligne du menu déclenche quand on la clique.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Passe au mode d'affichage suivant.
    CyclerMode,
    /// Passe à la taille suivante. Sans effet en plein écran.
    CyclerResolution,
    /// Passe au pipeline de rendu suivant : net, pixel art, pixel art + palette.
    CyclerRendu,
    /// Passe à la palette suivante. Sans objet hors du mode palette.
    CyclerPalette,
    /// Passe au niveau de tramage suivant. Sans objet hors du mode palette.
    CyclerTramage,
    /// Passe au niveau de saturation suivant. Sans objet hors du mode palette.
    CyclerSaturation,
    /// Revient à l'accueil.
    Retour,
}

/// Une ligne : son libellé, son action, et si elle est active.
pub struct Ligne {
    pub libelle: String,
    pub action: Action,
    /// Grisée : visible, mais sans effet. Mieux qu'absente — la disparition
    /// d'une ligne au changement de mode ferait sauter celles d'en dessous.
    pub active: bool,
}

/// Largeur des lignes du menu.
const LARGEUR: f32 = 520.0;
const HAUTEUR: f32 = 42.0;
const ECART: f32 = 14.0;

/// Les lignes à afficher, dans l'ordre.
///
/// **Déduites de l'état**, jamais recopiées : le libellé du mode vient de
/// `ModeAffichage::nom`, celui de la taille de `Resolution::libelle`. Un
/// second texte à tenir à jour finirait par mentir sur le réglage courant.
pub fn lignes(r: &Reglages) -> Vec<Ligne> {
    vec![
        Ligne {
            libelle: format!("AFFICHAGE : {}", r.mode.nom()),
            action: Action::CyclerMode,
            active: true,
        },
        Ligne {
            libelle: format!("TAILLE : {}", r.resolution.libelle()),
            action: Action::CyclerResolution,
            // En plein écran, c'est l'écran qui décide : le bouton se grise
            // plutôt que de faire semblant.
            active: r.mode.taille_reglable(),
        },
        Ligne {
            libelle: format!("RENDU : {}", r.rendu.nom()),
            action: Action::CyclerRendu,
            active: true,
        },
        Ligne {
            libelle: format!("PALETTE : {}", crate::palette::palette(r.palette).nom),
            action: Action::CyclerPalette,
            // Hors du mode palette, elle ne sert à rien : grisée plutôt
            // qu'absente, pour ne pas faire sauter les lignes du dessous.
            active: r.rendu.quantifie(),
        },
        Ligne {
            libelle: format!("TRAMAGE : {}", r.tramage.nom()),
            action: Action::CyclerTramage,
            active: r.rendu.quantifie(),
        },
        Ligne {
            libelle: format!("SATURATION : {}", r.saturation.nom()),
            action: Action::CyclerSaturation,
            active: r.rendu.quantifie(),
        },
        Ligne { libelle: "RETOUR".to_string(), action: Action::Retour, active: true },
    ]
}

/// Combien de rejets on détaille avant de résumer. Au-delà, la liste mangerait
/// le bas de l'écran ; le compte total reste dit.
const REJETS_DETAILLES: usize = 3;

/// Les lignes d'avertissement à afficher sous le menu, pour les fichiers de
/// palette refusés.
///
/// Séparé du dessin **parce que c'est là qu'est la décision** : quoi dire, et
/// combien. Sans rejet, aucune ligne — un écran qui annonce « 0 problème » est
/// du bruit.
pub fn lignes_de_rejet(rejets: &[crate::palette::Rejet]) -> Vec<String> {
    if rejets.is_empty() {
        return Vec::new();
    }
    let mut v = vec![format!(
        "{} fichier(s) de assets/palettes refuse(s) :",
        rejets.len()
    )];
    for r in rejets.iter().take(REJETS_DETAILLES) {
        // Le **nom du fichier** d'abord : c'est ce qu'on va aller corriger.
        v.push(format!("  {} : {}", r.fichier, r.raison));
    }
    if rejets.len() > REJETS_DETAILLES {
        v.push(format!("  ... et {} autre(s)", rejets.len() - REJETS_DETAILLES));
    }
    v
}

/// Rectangle de la ligne `i`, pour un écran donné.
pub fn rectangle(ecran: Vec2, i: usize) -> Rect {
    let x = ecran.x * 0.5 - LARGEUR * 0.5;
    let y0 = ecran.y * 0.30;
    Rect::new(x, y0 + i as f32 * (HAUTEUR + ECART), LARGEUR, HAUTEUR)
}

/// L'action sous le curseur, si la ligne est active.
pub fn action_sous_curseur(ecran: Vec2, lignes: &[Ligne], souris: Vec2) -> Option<Action> {
    lignes
        .iter()
        .enumerate()
        .find(|(i, l)| l.active && rectangle(ecran, *i).contains(souris))
        .map(|(_, l)| l.action)
}

/// L'écran lui-même.
pub struct Parametres;

impl Parametres {
    pub fn new() -> Self {
        Self
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil.
    pub fn frame(&mut self, reglages: &mut Reglages) -> bool {
        clear_background(Color::new(0.01, 0.01, 0.04, 1.0));
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);
        let ecran = vec2(screen_width(), screen_height());

        if is_key_pressed(KeyCode::Escape) {
            return true;
        }

        let l = lignes(reglages);
        let titre = "* PARAMETRES *";
        let tw = crate::police::mesure(titre, 36);
        crate::police::texte(
            titre,
            ecran.x * 0.5 - tw * 0.5,
            ecran.y * 0.16,
            36.0,
            Color::new(0.0, 0.9, 0.9, 1.0),
        );

        for (i, ligne) in l.iter().enumerate() {
            let r = rectangle(ecran, i);
            if ligne.active {
                minitel_ligne(r, &ligne.libelle, m);
            } else {
                // Grisée : même cadre, teinte éteinte, et le curseur ne
                // l'allume pas — elle doit se lire comme hors service.
                draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.02, 0.03, 0.08, 0.9));
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.25, 0.32, 0.34, 1.0));
                crate::police::texte(
                    &ligne.libelle,
                    r.x + 12.0,
                    r.y + r.h * 0.66,
                    22.0,
                    Color::new(0.38, 0.45, 0.47, 1.0),
                );
            }
        }

        // Note honnête sous les boutons : deux modes seulement, et pourquoi.
        crate::police::texte(
            "Le plein ecran de macroquad est deja sans bordure : il n'existe pas de mode exclusif.",
            ecran.x * 0.5 - LARGEUR * 0.5,
            rectangle(ecran, l.len()).y + 8.0,
            15.0,
            Color::new(0.45, 0.6, 0.62, 1.0),
        );
        crate::police::texte(
            "Echap : retour",
            ecran.x * 0.5 - LARGEUR * 0.5,
            rectangle(ecran, l.len()).y + 30.0,
            15.0,
            Color::new(0.45, 0.6, 0.62, 1.0),
        );

        // **Les palettes refusées se disent à l'écran.** Un fichier déposé qui
        // n'apparaît jamais au menu, sans que rien ne l'explique, est le défaut
        // qui a coûté le plus cher : la seule trace partait dans une console.
        let x = ecran.x * 0.5 - LARGEUR * 0.5;
        let mut y = rectangle(ecran, l.len()).y + 58.0;
        let ambre = Color::new(0.95, 0.7, 0.25, 1.0);
        for ligne in lignes_de_rejet(crate::palette::rejets()) {
            crate::police::texte(&ligne, x, y, 15.0, ambre);
            y += 20.0;
        }

        if !clic {
            return false;
        }
        match action_sous_curseur(ecran, &l, m) {
            Some(Action::CyclerMode) => {
                reglages.mode = reglages.mode.suivant();
                reglages.appliquer();
                false
            }
            Some(Action::CyclerResolution) => {
                reglages.resolution = reglages.resolution.suivante();
                reglages.appliquer();
                false
            }
            Some(Action::CyclerRendu) => {
                reglages.rendu = reglages.rendu.suivant();
                reglages.appliquer();
                false
            }
            Some(Action::CyclerPalette) => {
                reglages.palette = (reglages.palette + 1) % crate::palette::toutes().len();
                reglages.appliquer();
                false
            }
            Some(Action::CyclerTramage) => {
                reglages.tramage = reglages.tramage.suivant();
                reglages.appliquer();
                false
            }
            Some(Action::CyclerSaturation) => {
                reglages.saturation = reglages.saturation.suivant();
                reglages.appliquer();
                false
            }
            Some(Action::Retour) => true,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reglages::{ModeAffichage, ModeRendu, Resolution};

    const ECRAN: Vec2 = Vec2::new(1000.0, 700.0);

    // **Les libellés viennent de l'état**, jamais d'un texte parallèle : changer
    // le réglage doit changer ce qui est écrit.
    #[test]
    fn les_libelles_suivent_les_reglages() {
        let mut r = Reglages::default();
        let avant = lignes(&r);
        r.mode = r.mode.suivant();
        r.resolution = r.resolution.suivante();
        let apres = lignes(&r);
        assert_ne!(avant[0].libelle, apres[0].libelle, "le mode n'apparaît pas dans le libellé");
        assert_ne!(avant[1].libelle, apres[1].libelle, "la taille n'apparaît pas dans le libellé");
        // Et ils disent bien le réglage courant.
        assert!(apres[0].libelle.contains(r.mode.nom()));
        assert!(apres[1].libelle.contains(&r.resolution.largeur.to_string()));
    }

    // **La taille se grise en plein écran** : le bouton reste là — le faire
    // disparaître ferait sauter les lignes du dessous — mais il ne fait rien.
    #[test]
    fn la_ligne_de_taille_se_grise_en_plein_ecran() {
        let mut r = Reglages::default();
        r.mode = ModeAffichage::Fenetre;
        assert!(lignes(&r)[1].active, "en fenêtré, la taille doit être réglable");
        r.mode = ModeAffichage::SansBordure;
        let l = lignes(&r);
        assert!(!l[1].active, "en plein écran, la taille ne doit pas être réglable");
        assert_eq!(l.len(), lignes(&Reglages::default()).len(), "une ligne a disparu au lieu d'être grisée");
    }

    // **Le pipeline de rendu reste réglable dans tous les modes d'affichage** :
    // il ne dépend pas de la fenêtre. Une ligne grisée par erreur rendrait le
    // pixel art inaccessible en plein écran, c'est-à-dire là où on le regarde.
    #[test]
    fn le_rendu_est_reglable_dans_tous_les_modes_daffichage() {
        for mode in crate::reglages::ModeAffichage::TOUS {
            let mut r = Reglages::default();
            r.mode = mode;
            let l = lignes(&r);
            let i = l.iter().position(|l| l.action == Action::CyclerRendu).expect("pas de ligne rendu");
            assert!(l[i].active, "{mode:?} : la ligne de rendu est grisée");
        }
    }

    // **Palette et tramage ne se règlent que quand ils servent.** Hors du mode
    // palette, ils ne changent rien à l'écran : les laisser actifs ferait croire
    // à un réglage sans effet. Grisés, pas retirés — une ligne qui disparaît
    // fait sauter celles du dessous.
    #[test]
    fn les_reglages_de_palette_se_grisent_hors_du_mode_palette() {
        let concernees =
            [Action::CyclerPalette, Action::CyclerTramage, Action::CyclerSaturation];
        for rendu in ModeRendu::TOUS {
            let mut r = Reglages::default();
            r.rendu = rendu;
            let l = lignes(&r);
            assert_eq!(l.len(), lignes(&Reglages::default()).len(), "{rendu:?} : ligne disparue");
            for a in concernees {
                let i = l.iter().position(|l| l.action == a).expect("ligne absente");
                assert_eq!(
                    l[i].active,
                    rendu.quantifie(),
                    "{a:?} en mode {rendu:?} : activité incohérente"
                );
                // Et une ligne grisée ne répond pas au clic.
                if !rendu.quantifie() {
                    assert_eq!(
                        action_sous_curseur(ECRAN, &l, rectangle(ECRAN, i).center()),
                        None,
                        "{a:?} grisée mais cliquable"
                    );
                }
            }
        }
    }

    // Le libellé de la palette **suit le réglage** et nomme la palette courante :
    // sinon rien ne dirait laquelle est active.
    #[test]
    fn le_libelle_de_la_palette_nomme_la_palette_courante() {
        let n = crate::palette::toutes().len();
        let mut vus = std::collections::HashSet::new();
        for i in 0..n {
            let mut r = Reglages::default();
            r.rendu = ModeRendu::Palette;
            r.palette = i;
            let l = lignes(&r);
            let j = l.iter().position(|l| l.action == Action::CyclerPalette).unwrap();
            let nom = &crate::palette::palette(i).nom;
            assert!(l[j].libelle.contains(nom.as_str()), "« {} » ne nomme pas {nom}", l[j].libelle);
            vus.insert(l[j].libelle.clone());
        }
        assert_eq!(vus.len(), n, "des palettes affichent le même libellé");
    }

    // Le libellé du rendu **suit le réglage**, et les trois modes s'y lisent
    // différemment : un libellé figé laisserait croire que le clic n'a rien fait.
    #[test]
    fn le_libelle_du_rendu_distingue_les_trois_modes() {
        let mut vus = std::collections::HashSet::new();
        for rendu in ModeRendu::TOUS {
            let mut r = Reglages::default();
            r.rendu = rendu;
            let l = lignes(&r);
            let i = l.iter().position(|l| l.action == Action::CyclerRendu).unwrap();
            assert!(l[i].libelle.contains(rendu.nom()), "{rendu:?} absent de « {} »", l[i].libelle);
            vus.insert(l[i].libelle.clone());
        }
        assert_eq!(vus.len(), ModeRendu::TOUS.len(), "deux modes affichent le même libellé");
    }

    // Une ligne grisée **ne se clique pas**. Sans ça, elle changerait un réglage
    // que l'écran présente comme hors service.
    #[test]
    fn une_ligne_grisee_ne_repond_pas_au_clic() {
        let mut r = Reglages::default();
        r.mode = ModeAffichage::SansBordure;
        let l = lignes(&r);
        // Les lignes sont retrouvées par leur action, pas par un indice écrit en
        // dur : en insérer une nouvelle ne doit pas casser ce test.
        let indice = |a: Action| l.iter().position(|l| l.action == a).unwrap();
        let grisee = indice(Action::CyclerResolution);
        assert_eq!(
            action_sous_curseur(ECRAN, &l, rectangle(ECRAN, grisee).center()),
            None,
            "la ligne grisée a répondu"
        );
        // Les actives, elles, répondent bien.
        for a in [Action::CyclerMode, Action::CyclerRendu, Action::Retour] {
            assert_eq!(
                action_sous_curseur(ECRAN, &l, rectangle(ECRAN, indice(a)).center()),
                Some(a),
                "{a:?} ne répond pas au clic"
            );
        }
    }

    // Les lignes ne se **chevauchent pas** et tiennent dans l'écran : deux
    // lignes superposées rendraient l'une des deux inatteignable.
    #[test]
    fn les_lignes_ne_se_chevauchent_pas_et_tiennent_dans_lecran() {
        for (l, h) in [(1000.0_f32, 700.0_f32), (1280.0, 720.0), (1920.0, 1080.0)] {
            let e = vec2(l, h);
            let n = lignes(&Reglages::default()).len();
            for i in 0..n {
                let a = rectangle(e, i);
                assert!(a.x >= 0.0 && a.x + a.w <= l + 1e-3, "{l}x{h} : ligne {i} déborde");
                assert!(a.y >= 0.0 && a.y + a.h <= h + 1e-3, "{l}x{h} : ligne {i} hors écran");
                for j in i + 1..n {
                    let b = rectangle(e, j);
                    assert!(
                        a.y + a.h <= b.y + 1e-3 || b.y + b.h <= a.y + 1e-3,
                        "{l}x{h} : lignes {i} et {j} se chevauchent"
                    );
                }
            }
        }
    }

    // Hors des lignes, **rien** ne se déclenche — y compris juste sous la
    // dernière, où le menu n'a plus d'entrée.
    #[test]
    fn hors_des_lignes_rien_ne_se_declenche() {
        let r = Reglages::default();
        let l = lignes(&r);
        assert_eq!(action_sous_curseur(ECRAN, &l, vec2(10.0, 10.0)), None, "coin haut gauche");
        let sous = rectangle(ECRAN, l.len()).center();
        assert_eq!(action_sous_curseur(ECRAN, &l, sous), None, "sous la dernière ligne");
    }

    // **Sans rejet, aucune ligne.** Un écran qui annonce « 0 problème » est du
    // bruit, et il pousserait le reste vers le bas pour rien.
    #[test]
    fn sans_rejet_aucune_ligne_davertissement() {
        assert!(lignes_de_rejet(&[]).is_empty());
    }

    // **Chaque rejet nomme son fichier**, jusqu'à la limite de détail, et le
    // **compte total** est toujours dit — sinon on croirait n'en avoir qu'un.
    #[test]
    fn les_rejets_nomment_leur_fichier_et_disent_le_total() {
        let faux = |n: usize| -> Vec<crate::palette::Rejet> {
            (0..n)
                .map(|i| crate::palette::Rejet {
                    fichier: format!("p{i}.hex"),
                    raison: format!("raison {i}"),
                })
                .collect()
        };

        // Peu de rejets : tous détaillés.
        let r = faux(2);
        let l = lignes_de_rejet(&r);
        assert!(l[0].contains('2'), "le total manque : {}", l[0]);
        for x in &r {
            assert!(l.iter().any(|s| s.contains(&x.fichier)), "{} absent", x.fichier);
            assert!(l.iter().any(|s| s.contains(&x.raison)), "raison de {} absente", x.fichier);
        }

        // Beaucoup : on détaille les premiers, on résume le reste, mais le total
        // reste exact.
        let r = faux(REJETS_DETAILLES + 4);
        let l = lignes_de_rejet(&r);
        assert!(l[0].contains(&r.len().to_string()), "total faux : {}", l[0]);
        assert!(
            l.len() <= REJETS_DETAILLES + 2,
            "{} lignes : la liste va déborder de l'écran",
            l.len()
        );
        assert!(l.last().unwrap().contains('4'), "le reste n'est pas résumé : {}", l.last().unwrap());
    }

    // Chaque action est **atteignable** : une action qu'aucune ligne ne porte
    // serait du code mort, et l'inverse une ligne sans effet.
    #[test]
    fn chaque_action_est_portee_par_une_ligne() {
        let mut r = Reglages::default();
        r.mode = ModeAffichage::Fenetre;
        r.resolution = Resolution::TOUTES[0];
        let portees: Vec<Action> = lignes(&r).iter().map(|l| l.action).collect();
        for a in [
            Action::CyclerMode,
            Action::CyclerResolution,
            Action::CyclerRendu,
            Action::CyclerPalette,
            Action::CyclerTramage,
            Action::CyclerSaturation,
            Action::Retour,
        ] {
            assert!(portees.contains(&a), "{a:?} n'est portée par aucune ligne");
        }
    }
}
