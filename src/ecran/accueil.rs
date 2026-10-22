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
    GalerieCometes,
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
pub struct Accueil {
    /// Le logo du jeu, à la place du titre écrit (2026-10-22).
    logo: Texture2D,
}

/// **Le logo**, embarqué dans l'exécutable : le jeu ne dépend pas du dossier
/// d'où on le lance.
const LOGO_PNG: &[u8] = include_bytes!("../../assets/logo/Logo.png");
/// Ses dimensions en pixels. Tenues d'accord avec le fichier par un test.
const LOGO_L: f32 = 256.0;
const LOGO_H: f32 = 129.0;
/// Espace entre le logo et sa bordure, et marge au-dessus.
const BORD_LOGO: f32 = 4.0;
const MARGE_HAUT: f32 = 8.0;
/// Place du titre d'un bloc au-dessus de son premier bouton (`bloc` l'écrit à
/// `y0 − 10`, en 20 px).
const PLACE_TITRE_BLOC: f32 = 30.0;
/// Écart bloc → PARAMETRES (voir la note de `frame`).
const ECART_PARAMETRES: f32 = 14.0;
/// **Bleu Minitel** : celui des bordures des panneaux (`ui::minitel_panel`).
const BLEU_MINITEL: Color = Color::new(0.0, 0.85, 0.85, 1.0);

/// **Mise en page de l'accueil** : `(échelle du logo, rectangle du logo, y du
/// premier bouton)`.
///
/// ⚠️ **Échelle entière, jamais étirée** : le logo est en pixel art, et une
/// échelle fractionnaire ferait des pixels de tailles inégales. On prend la
/// plus grande qui tient — ×1 (256 px) dans la fenêtre par défaut, ×2 (512 px)
/// et plus dès qu'elle est assez haute — sans jamais repousser QUITTER hors
/// de l'écran. Choisi par l'utilisateur plutôt qu'agrandir la fenêtre.
fn mise_en_page(ecran: Vec2) -> (f32, Rect, f32) {
    let hauteur_bloc = BLOC_ASTRES.len().max(BLOC_STATIONS.len()) as f32 * PAS_BOUTON;
    // Tout ce qui vient sous le logo, plus 3 px de marge au bas.
    let dessous = PLACE_TITRE_BLOC + hauteur_bloc + ECART_PARAMETRES + 2.0 * HAUTEUR_BOUTON + ECART_BOUTON + 3.0;
    let dispo_h = ecran.y - MARGE_HAUT - 2.0 * BORD_LOGO - dessous;
    let dispo_l = ecran.x - 2.0 * BORD_LOGO - 16.0;
    let k = (dispo_h / LOGO_H).min(dispo_l / LOGO_L).floor().max(1.0);
    let (l, h) = (LOGO_L * k, LOGO_H * k);
    let r = Rect::new(ecran.x * 0.5 - l * 0.5, MARGE_HAUT + BORD_LOGO, l, h);
    (k, r, r.y + h + BORD_LOGO + PLACE_TITRE_BLOC)
}

/// Hauteur d'un bouton et espace vertical jusqu'au suivant, **une seule
/// source** pour `bloc()` (qui espace ses entrées de ce pas) et `frame()`
/// (qui doit connaître la hauteur totale d'un bloc pour placer PARAMETRES et
/// QUITTER en dessous) : deux littéraux recopiés auraient fini par diverger.
///
/// ⚠️ **34 et 8 depuis le 2026-10-22** (40 et 12 avant). Le bloc ASTRES est
/// passé à 9 entrées avec la galerie des comètes, et QUITTER **sortait déjà**
/// de la fenêtre par défaut (bas à 742 px pour 700) — personne ne l'avait vu.
/// Le logo, à la place du titre, en demandait plus encore. Gardé désormais par
/// `le_logo_grandit_sans_jamais_deborder`, qui recompte les blocs.
const HAUTEUR_BOUTON: f32 = 34.0;
const ECART_BOUTON: f32 = 8.0;
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
    ("GALERIE - COMETES", Cible::GalerieCometes),
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
        let logo = Texture2D::from_file_with_format(LOGO_PNG, Some(ImageFormat::Png));
        // Pixel art : agrandi au plus proche voisin, sinon il baverait.
        logo.set_filter(FilterMode::Nearest);
        Self { logo }
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
        // **Le logo à la place du titre écrit** (2026-10-22), à échelle
        // entière, cerné d'une fine bordure bleu Minitel.
        let (_, lr, y0) = mise_en_page(vec2(screen_width(), screen_height()));
        draw_texture_ex(
            &self.logo,
            lr.x,
            lr.y,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(lr.w, lr.h)), ..Default::default() },
        );
        draw_rectangle_lines(
            lr.x - BORD_LOGO,
            lr.y - BORD_LOGO,
            lr.w + 2.0 * BORD_LOGO,
            lr.h + 2.0 * BORD_LOGO,
            2.0,
            BLEU_MINITEL,
        );

        let bw = 380.0;
        let ecart = 48.0;
        let xg = cx - bw - ecart * 0.5;
        let xd = cx + ecart * 0.5;

        let choix = Self::bloc("[ ASTRES & GALERIES ]", BLOC_ASTRES, xg, y0, m, clic)
            .or_else(|| Self::bloc("[ STATIONS & MEGASTRUCTURES ]", BLOC_STATIONS, xd, y0, m, clic));

        // Les paramètres valent pour **tout le jeu**, pas pour une famille de
        // vues : ils se posent donc à part, sous les deux blocs, et non dans
        // l'un des deux.
        //
        // Écart bloc -> PARAMETRES : 14 px. Le calcul de la place, logo
        // compris, vit dans `mise_en_page` ; c'est sa garde qui vérifie que
        // QUITTER tient dans la fenêtre — un commentaire chiffré ici avait
        // cessé d'être vrai à la première entrée ajoutée.
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

#[cfg(test)]
mod tests_logo {
    use super::*;

    /// **Les dimensions du logo sont celles du fichier.** La mise en page les
    /// tient en constantes (elle se calcule sans contexte graphique) : si le
    /// fichier change, elle doit suivre, sans quoi le logo serait étiré.
    #[test]
    fn les_dimensions_du_logo_sont_celles_du_fichier() {
        // En-tête PNG : largeur puis hauteur, en grand-boutiste, octets 16 à 24.
        let l = u32::from_be_bytes(LOGO_PNG[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(LOGO_PNG[20..24].try_into().unwrap());
        assert_eq!((l as f32, h as f32), (LOGO_L, LOGO_H));
    }

    /// **Le logo n'est jamais étiré, et ne repousse jamais QUITTER hors de
    /// l'écran** : échelle entière, la plus grande qui tient. ×1 dans la
    /// fenêtre par défaut, au moins ×2 (512 px) dès qu'elle est assez haute.
    #[test]
    fn le_logo_grandit_sans_jamais_deborder() {
        let hauteur_bloc = BLOC_ASTRES.len().max(BLOC_STATIONS.len()) as f32 * PAS_BOUTON;
        for (l, h) in [(1000.0_f32, 700.0_f32), (1280.0, 800.0), (1000.0, 830.0), (1920.0, 1080.0), (2560.0, 1440.0)] {
            let (k, r, y0) = mise_en_page(vec2(l, h));
            assert_eq!(k, k.floor(), "{l}x{h} : échelle {k} non entière, le pixel art serait déformé");
            assert!((r.w / r.h - LOGO_L / LOGO_H).abs() < 1e-4, "{l}x{h} : logo étiré");
            assert!(r.y + r.h + BORD_LOGO < y0, "{l}x{h} : le logo recouvre les blocs");
            let bas_quitter = y0 + hauteur_bloc + ECART_PARAMETRES + 2.0 * HAUTEUR_BOUTON + ECART_BOUTON;
            assert!(bas_quitter <= h, "{l}x{h} : QUITTER sort de l'écran ({bas_quitter})");
        }
        assert_eq!(mise_en_page(vec2(1000.0, 700.0)).0, 1.0, "fenêtre par défaut : ×1 attendu");
        assert!(mise_en_page(vec2(1920.0, 1080.0)).0 >= 2.0, "plein écran 1080p : au moins 512 px attendus");
    }
}
