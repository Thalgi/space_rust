use crate::ui::minitel_ligne;
use macroquad::prelude::*;

/// Destination choisie depuis l'accueil.
#[derive(Clone, Copy)]
pub enum Cible {
    Starmap,
    Skymap,
    /// Skymap V2 : marqueurs de planètes et de lunes (fork de `Skymap`).
    SkymapV2,
    Objet,
    Galerie,
    GalerieGaz,
    GalerieDisques,
    GalerieEtoiles,
    /// Catalogue des composants (vue station, catégorie Briques).
    Briques,
    /// Reproductions de petites stations réelles.
    PetitesStations,
    /// Générateur procédural.
    Generateur,
    /// Grandes stations & mégastructures.
    Megastructures,
    /// Vaisseaux et supervaisseaux (ISV, puis Endurance et Starship).
    Vaisseaux,
    /// Assembleur : construire un vaisseau à la main (`conception/assembleur.md`).
    Assembleur,
    /// Réglages du jeu (affichage, taille de fenêtre, rendu).
    Parametres,
    /// Ferme le jeu.
    Quitter,
}

/// Écran d'accueil : titre + deux blocs de boutons (astres / vaisseaux).
pub struct Accueil;

/// Hauteur d'un bouton et espace vertical jusqu'au suivant, **une seule
/// source** pour `bloc()` (qui espace ses entrées de ce pas) et `frame()`
/// (qui doit connaître la hauteur totale d'un bloc pour placer PARAMETRES et
/// QUITTER en dessous) : deux littéraux recopiés auraient fini par diverger.
const HAUTEUR_BOUTON: f32 = 40.0;
const ECART_BOUTON: f32 = 12.0;
const PAS_BOUTON: f32 = HAUTEUR_BOUTON + ECART_BOUTON;

/// Bloc de gauche : exploration et galeries d'astres.
const BLOC_ASTRES: &[(&str, Cible)] = &[
    ("SKYMAP - SYSTEME COMPLET", Cible::Skymap),
    ("SKYMAP V2 - MARQUEURS", Cible::SkymapV2),
    ("OBJET CELESTE - VUE ISOLEE", Cible::Objet),
    ("STARMAP - VOISINAGE STELLAIRE", Cible::Starmap),
    ("GALERIE - TYPES TELLURIQUES", Cible::Galerie),
    ("GALERIE - GEANTES GAZEUSES", Cible::GalerieGaz),
    ("GALERIE - CEINTURES & DISQUES", Cible::GalerieDisques),
    ("GALERIE - ETOILES", Cible::GalerieEtoiles),
];

/// Bloc de droite : briques, stations et mégastructures.
const BLOC_STATIONS: &[(&str, Cible)] = &[
    ("BRIQUES - COMPOSANTS", Cible::Briques),
    ("PETITES STATIONS - ISS / MIR / TIANGONG", Cible::PetitesStations),
    ("GENERATEUR PROCEDURAL", Cible::Generateur),
    ("MEGASTRUCTURES - ANNEAUX / TORES", Cible::Megastructures),
    ("VAISSEAUX & SUPERVAISSEAUX", Cible::Vaisseaux),
    ("ASSEMBLEUR - CONSTRUIRE (VAB)", Cible::Assembleur),
];

impl Accueil {
    pub fn new() -> Self {
        Self
    }

    /// Dessine un bloc (titre + boutons empilés) et renvoie la cible cliquée.
    fn bloc(
        titre: &str,
        entrees: &'static [(&str, Cible)],
        x: f32,
        y0: f32,
        m: Vec2,
        clic: bool,
    ) -> Option<&'static Cible> {
        let bw = 380.0;
        let bh = HAUTEUR_BOUTON;
        crate::police::texte(titre, x + 4.0, y0 - 10.0, 20.0, Color::new(0.5, 0.9, 0.7, 1.0));
        let mut choix = None;
        for (i, (label, cible)) in entrees.iter().enumerate() {
            let r = Rect::new(x, y0 + i as f32 * PAS_BOUTON, bw, bh);
            minitel_ligne(r, label, m);
            if clic && r.contains(m) {
                choix = Some(cible);
            }
        }
        choix
    }

    /// Dessine l'accueil et renvoie la destination si un bouton est cliqué.
    pub fn frame(&mut self) -> Option<Cible> {
        clear_background(Color::new(0.01, 0.01, 0.04, 1.0));
        let m = vec2(mouse_position().0, mouse_position().1);
        let clic = is_mouse_button_pressed(MouseButton::Left);

        let cx = screen_width() * 0.5;
        let titre = "* GENERATEUR DE SYSTEMES *";
        let tw = crate::police::mesure(titre, crate::police::TAILLE_MAX as u16);
        crate::police::texte(titre, cx - tw * 0.5, screen_height() * 0.12, crate::police::TAILLE_MAX, Color::new(0.0, 0.9, 0.9, 1.0));

        let bw = 380.0;
        let ecart = 48.0;
        let y0 = screen_height() * 0.24;
        let xg = cx - bw - ecart * 0.5;
        let xd = cx + ecart * 0.5;

        let choix = Self::bloc("[ ASTRES & GALERIES ]", BLOC_ASTRES, xg, y0, m, clic)
            .or_else(|| Self::bloc("[ STATIONS & MEGASTRUCTURES ]", BLOC_STATIONS, xd, y0, m, clic));

        // Les paramètres valent pour **tout le jeu**, pas pour une famille de
        // vues : ils se posent donc à part, sous les deux blocs, et non dans
        // l'un des deux.
        //
        // Écart bloc -> PARAMETRES resserré à 14 px (au lieu de 28) depuis que
        // SKYMAP V2 a porté BLOC_ASTRES à 8 entrées : à h = 700,
        // y0 = 0.24·700 = 168, hauteur_bloc = 8·52 = 416, d'où bas de QUITTER
        // = 168 + 416 + 14 + 40 + 12 + 40 = 690 ≤ 700 − 8 (marge 10 px). Avec
        // l'ancien écart de 28, ce bas atteignait 704 > 700 : QUITTER dépassait
        // le bas de la fenêtre par défaut.
        const ECART_PARAMETRES: f32 = 14.0;
        let hauteur_bloc = BLOC_ASTRES.len().max(BLOC_STATIONS.len()) as f32 * PAS_BOUTON;
        let r = Rect::new(cx - 190.0, y0 + hauteur_bloc + ECART_PARAMETRES, 380.0, HAUTEUR_BOUTON);
        minitel_ligne(r, "PARAMETRES", m);
        if clic && r.contains(m) {
            return Some(Cible::Parametres);
        }

        // QUITTER, sous les paramètres. En dernier, parce qu'un bouton qui ferme
        // le jeu ne doit jamais border un bouton qu'on clique souvent.
        let q = Rect::new(cx - 190.0, r.y + r.h + ECART_BOUTON, 380.0, HAUTEUR_BOUTON);
        minitel_ligne(q, "QUITTER", m);
        if clic && q.contains(m) {
            return Some(Cible::Quitter);
        }

        choix.copied()
    }
}
