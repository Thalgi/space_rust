//! **Orbites de lunes en trait large** (Skymap V2) : la polyligne d'orbite
//! est projetée à l'écran, puis tracée en 2D au motif « mediumdashdot ».
//!
//! ⚠️ **Pourquoi en 2D et pas avec les autres calques.** `draw_line_3d` de
//! macroquad n'a **pas** d'épaisseur : il rend toujours un pixel, quelle que
//! soit la distance. Les calques de `systeme/rendu.rs` (semis de points)
//! vivent avec cette contrainte ; un trait trois fois plus large ne peut donc
//! pas s'obtenir là-bas. `draw_line`, lui, prend une épaisseur — mais c'est
//! du dessin écran, donc une passe interface.
//!
//! Conséquence assumée : ce trait ne passe pas le test de profondeur, il se
//! voit même quand la lune est derrière sa planète. Sur un anneau d'orbite
//! que l'on n'affiche **que** pour l'astre sélectionné, c'est ce qu'on veut —
//! il sert à dire où chercher la lune.
//!
//! Comme `marqueur`, ce qui **décide** ([`segments_motif`]) est pur et testé ;
//! seul le tracé touche macroquad (`docs/methode.md` : pas de test de rendu).

use crate::astre::{CameraInfo, Categorie};
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Épaisseur du trait, en pixels — trois fois le trait d'un pixel des autres
/// calques, qui ne se lisait pas une fois l'orbite réduite à quelques dizaines
/// de pixels.
pub const EPAISSEUR_PX: f32 = 3.0;

/// Teinte des orbites de lunes en V2 : le violet pâle des calques
/// (`systeme::rendu::COL_LUNES`), remonté en opacité — un trait large et
/// transparent bave, là où un trait fin se lisait encore.
pub const COL_LUNES: Color = Color::new(0.62, 0.55, 0.90, 0.85);

/// Le motif **mediumdashdot**, en pixels écran : trait long, trou, point,
/// trou — répété. `true` = on pose de l'encre.
const MOTIF: [(f32, bool); 4] = [(12.0, true), (4.0, false), (2.0, true), (4.0, false)];

/// Découpe une polyligne **écran** selon [`MOTIF`], et rend les segments à
/// encrer.
///
/// Le motif se poursuit d'un segment de polyligne au suivant (il ne repart
/// pas de zéro à chaque sommet) : sans ça, une orbite échantillonnée fin
/// donnerait un trait plein, et une orbite échantillonnée grossièrement un
/// motif à la longueur variable. Ce qui compte est la longueur parcourue **à
/// l'écran**, pas le nombre de sommets.
pub fn segments_motif(points: &[Vec2]) -> Vec<(Vec2, Vec2)> {
    let mut out = Vec::new();
    if points.len() < 2 {
        return out;
    }
    let (mut phase, mut reste) = (0usize, MOTIF[0].0);
    for paire in points.windows(2) {
        let (a, b) = (paire[0], paire[1]);
        let longueur = a.distance(b);
        // Deux points confondus (une orbite peut en porter) : on saute, sinon
        // `dir` vaudrait 0/0 = NaN. ⚠️ Garde **défensive seulement** : la
        // boucle ci-dessous n'est de toute façon jamais entrée à longueur
        // nulle (`parcouru < longueur` est faux d'emblée), et le NaN ne serait
        // donc jamais employé. Vérifié par sabotage — le retrait de ces trois
        // lignes ne change aucun résultat. Gardée quand même : un NaN qui
        // existe est une bombe pour la prochaine réécriture de la boucle.
        if longueur <= f32::EPSILON {
            continue;
        }
        let dir = (b - a) / longueur;
        let mut parcouru = 0.0;
        while parcouru < longueur {
            let pas = reste.min(longueur - parcouru);
            if MOTIF[phase].1 {
                out.push((a + dir * parcouru, a + dir * (parcouru + pas)));
            }
            parcouru += pas;
            reste -= pas;
            if reste <= f32::EPSILON {
                phase = (phase + 1) % MOTIF.len();
                reste = MOTIF[phase].0;
            }
        }
    }
    out
}

/// Trace l'orbite de chaque **lune** de `parent`. Passe interface.
///
/// La polyligne d'orbite est relative au parent (`Astre::orbite_lune`), qui
/// bouge : on lui ajoute la position courante du parent avant de projeter.
/// Une orbite est fermée, d'où le retour au premier point.
pub fn dessiner_orbites_lunes(
    sys: &Systeme,
    parent: usize,
    cam: &CameraInfo,
    ecran: Vec2,
    aspect: f32,
) {
    let centre = sys.position(parent);
    for idx in sys.lunes_de(parent) {
        if sys.categorie_de(idx) != Some(Categorie::Lune) {
            continue;
        }
        let pts = sys.orbite_lune_de(idx);
        if pts.is_empty() {
            continue;
        }
        // Un point derrière la caméra n'a pas de projection : il COUPE la
        // polyligne en deux morceaux plutôt que de les relier par un trait
        // fantôme en travers de l'écran.
        let mut chaine: Vec<Vec2> = Vec::with_capacity(pts.len() + 1);
        for p in pts.iter().chain(std::iter::once(&pts[0])) {
            match crate::camera::projeter_ecran(centre + *p, ecran, cam, aspect) {
                Some(v) => chaine.push(v),
                None => {
                    tracer(&chaine);
                    chaine.clear();
                }
            }
        }
        tracer(&chaine);
    }
}

fn tracer(chaine: &[Vec2]) {
    for (a, b) in segments_motif(chaine) {
        draw_line(a.x, a.y, b.x, b.y, EPAISSEUR_PX, COL_LUNES);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Longueur totale d'un motif complet, et part encrée.
    const CYCLE: f32 = 22.0; // 12 + 4 + 2 + 4
    const ENCRE: f32 = 14.0; // 12 + 2

    fn longueur(segments: &[(Vec2, Vec2)]) -> f32 {
        segments.iter().map(|(a, b)| a.distance(*b)).sum()
    }

    /// **Le motif encre la bonne fraction du trait.** C'est ce qui fait lire
    /// « trait-point » plutôt que « trait plein » ou « pointillé ».
    #[test]
    fn le_motif_encre_la_fraction_attendue() {
        // Dix cycles exactement, en ligne droite.
        let l = CYCLE * 10.0;
        let segments = segments_motif(&[vec2(0.0, 0.0), vec2(l, 0.0)]);
        let attendu = ENCRE * 10.0;
        assert!(
            (longueur(&segments) - attendu).abs() < 0.01,
            "{} encré sur {l}, attendu {attendu}",
            longueur(&segments)
        );
        // Et ce sont bien deux traits par cycle (le long, puis le point).
        assert_eq!(segments.len(), 20, "un trait et un point par cycle");
    }

    /// **Le motif ne repart pas de zéro à chaque sommet.** Une orbite
    /// échantillonnée fin sortirait sinon en trait plein : chaque segment
    /// commencerait par le trait long et n'irait jamais jusqu'au trou.
    #[test]
    fn le_motif_se_poursuit_d_un_sommet_a_l_autre() {
        let l = CYCLE * 4.0;
        let entier = segments_motif(&[vec2(0.0, 0.0), vec2(l, 0.0)]);
        // La même droite, mais découpée en 16 petits segments colinéaires.
        let decoupee: Vec<Vec2> = (0..=16).map(|i| vec2(l * i as f32 / 16.0, 0.0)).collect();
        let morcelee = segments_motif(&decoupee);
        assert!(
            (longueur(&entier) - longueur(&morcelee)).abs() < 0.01,
            "{} encré en un trait contre {} en seize",
            longueur(&entier),
            longueur(&morcelee)
        );
    }

    // ⚠️ Un test `une_polyligne_degeneree_ne_produit_rien` a été écrit ici,
    // puis **jeté** : aucun de ses sabotages ne le faisait rougir. Retirer la
    // garde de longueur nulle, retirer celle de `points.len() < 2` — il
    // restait vert dans les deux cas, parce que `windows(2)` ne rend rien
    // sous deux points et que `parcouru < longueur` est faux d'emblée à
    // longueur nulle. Il décrivait des propriétés vraies que rien dans le
    // code ne pouvait casser : il ne gardait rien (`docs/methode.md`).

    /// Le premier trait démarre **au début** de la polyligne : une orbite ne
    /// doit pas commencer par un trou, sinon deux lunes voisines paraissent
    /// décalées l'une de l'autre.
    #[test]
    fn le_motif_commence_par_de_l_encre() {
        let segments = segments_motif(&[vec2(10.0, 10.0), vec2(60.0, 10.0)]);
        assert_eq!(segments[0].0, vec2(10.0, 10.0));
        assert!((segments[0].1.x - 22.0).abs() < 0.01, "premier trait de 12 px : {:?}", segments[0]);
    }
}
