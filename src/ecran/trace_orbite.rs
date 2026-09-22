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

/// Longueur d'un cycle complet du motif, en pixels.
const CYCLE: f32 = 22.0;

/// Avance l'état du motif de `l` pixels **sans rien produire**, en temps
/// constant.
///
/// C'est ce qui rend le tracé insensible au zoom : la part invisible d'une
/// arête — qui peut faire des millions de pixels — ne se **parcourt** pas,
/// elle se saute au modulo. Le motif reste pourtant en phase, donc la portion
/// visible est dessinée exactement comme si on l'avait parcourue.
fn avancer(mut phase: usize, reste: f32, mut l: f32) -> (usize, f32) {
    if l <= 0.0 {
        return (phase, reste);
    }
    if l < reste {
        return (phase, reste - l);
    }
    l -= reste;
    phase = (phase + 1) % MOTIF.len();
    l %= CYCLE; // le gros du saut part ici, en une opération
    while l >= MOTIF[phase].0 {
        l -= MOTIF[phase].0;
        phase = (phase + 1) % MOTIF.len();
    }
    (phase, MOTIF[phase].0 - l)
}

/// Portion **visible** d'une arête, en abscisses curvilignes `[t0, t1]` le
/// long de `dir` depuis `a` — `None` si l'arête ne touche pas l'écran.
///
/// Découpe de Liang-Barsky contre le rectangle de l'écran, élargi de
/// l'épaisseur du trait pour qu'un trait rasant le bord ne disparaisse pas.
fn portion_visible(a: Vec2, dir: Vec2, longueur: f32, ecran: Vec2) -> Option<(f32, f32)> {
    let marge = EPAISSEUR_PX;
    let (mut t0, mut t1) = (0.0_f32, longueur);
    for (p, q) in [
        (-dir.x, a.x + marge),
        (dir.x, ecran.x + marge - a.x),
        (-dir.y, a.y + marge),
        (dir.y, ecran.y + marge - a.y),
    ] {
        if p.abs() <= f32::EPSILON {
            // Arête parallèle à ce bord : dedans ou dehors, mais pas coupée.
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = t0.max(r);
        } else {
            t1 = t1.min(r);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((t0, t1))
}

/// Découpe une polyligne **écran** selon [`MOTIF`], et rend les segments à
/// encrer — **ceux qui se voient seulement**.
///
/// Le motif se poursuit d'un segment de polyligne au suivant (il ne repart
/// pas de zéro à chaque sommet) : sans ça, une orbite échantillonnée fin
/// donnerait un trait plein, et une orbite échantillonnée grossièrement un
/// motif à la longueur variable. Ce qui compte est la longueur parcourue **à
/// l'écran**, pas le nombre de sommets.
///
/// ⚠️ **Le découpage à l'écran n'est pas une optimisation, c'est la
/// condition pour que ça tienne.** Mesuré : à ×25000 de zoom sur une lune,
/// une seule arête d'orbite projetée faisait 10⁷ px, soit **909 091
/// segments** — pour une orbite de 96 arêtes et par frame. Le tracé épuisait
/// la mémoire et le jeu tombait. Le travail suit désormais ce qui se voit,
/// jamais ce qui déborde.
pub fn segments_motif(points: &[Vec2], ecran: Vec2) -> Vec<(Vec2, Vec2)> {
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
        let Some((t0, t1)) = portion_visible(a, dir, longueur, ecran) else {
            // Rien de visible sur cette arête : on saute son motif d'un bloc.
            (phase, reste) = avancer(phase, reste, longueur);
            continue;
        };
        (phase, reste) = avancer(phase, reste, t0);
        let mut parcouru = t0;
        while parcouru < t1 {
            let pas = reste.min(t1 - parcouru);
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
        (phase, reste) = avancer(phase, reste, longueur - t1);
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
                    tracer(&chaine, ecran);
                    chaine.clear();
                }
            }
        }
        tracer(&chaine, ecran);
    }
}

fn tracer(chaine: &[Vec2], ecran: Vec2) {
    for (a, b) in segments_motif(chaine, ecran) {
        draw_line(a.x, a.y, b.x, b.y, EPAISSEUR_PX, COL_LUNES);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Part encrée d'un cycle (12 + 2 sur 22).
    const ENCRE: f32 = 14.0;
    /// Écran d'essai. Les polylignes des tests de motif tiennent dedans :
    /// c'est voulu, elles mesurent le motif, pas le découpage.
    const ECRAN: Vec2 = Vec2::new(1000.0, 700.0);

    fn longueur(segments: &[(Vec2, Vec2)]) -> f32 {
        segments.iter().map(|(a, b)| a.distance(*b)).sum()
    }

    /// **Le motif encre la bonne fraction du trait.** C'est ce qui fait lire
    /// « trait-point » plutôt que « trait plein » ou « pointillé ».
    #[test]
    fn le_motif_encre_la_fraction_attendue() {
        // Dix cycles exactement, en ligne droite.
        let l = CYCLE * 10.0;
        let segments = segments_motif(&[vec2(0.0, 0.0), vec2(l, 0.0)], ECRAN);
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
        let entier = segments_motif(&[vec2(0.0, 0.0), vec2(l, 0.0)], ECRAN);
        // La même droite, mais découpée en 16 petits segments colinéaires.
        let decoupee: Vec<Vec2> = (0..=16).map(|i| vec2(l * i as f32 / 16.0, 0.0)).collect();
        let morcelee = segments_motif(&decoupee, ECRAN);
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

    /// **Une orbite énorme à l'écran ne doit pas coûter un travail énorme.**
    ///
    /// À fort zoom, la polyligne projetée fait des millions de pixels dont
    /// presque rien n'est visible : découper TOUTE sa longueur au pas du
    /// motif produit des centaines de milliers de segments par lune et par
    /// frame.
    #[test]
    fn une_orbite_geante_ne_coute_que_ce_qui_se_voit() {
        let l = 5.0e6;
        let segments = segments_motif(&[vec2(-l, 350.0), vec2(l, 350.0)], ECRAN);
        // Au plus l'écran en diagonale, divisé par le cycle, avec de la marge.
        assert!(
            segments.len() < 200,
            "{} segments pour une ligne de {} px dont ~1000 visibles",
            segments.len(),
            2.0 * l
        );
    }

    /// **Le saut au modulo doit laisser le motif exactement où le parcours
    /// l'aurait laissé.** C'est ce qui permet de ne pas dessiner l'invisible
    /// sans déphaser le visible : si `avancer` mentait, la partie visible
    /// d'une orbite sortirait avec des traits décalés selon le zoom.
    #[test]
    fn avancer_equivaut_au_parcours_pas_a_pas() {
        // Parcours naïf, celui que `avancer` remplace.
        let lent = |mut phase: usize, mut reste: f32, l: f32| {
            let mut restant = l;
            while restant > 0.0 {
                let pas = reste.min(restant);
                restant -= pas;
                reste -= pas;
                if reste <= f32::EPSILON {
                    phase = (phase + 1) % MOTIF.len();
                    reste = MOTIF[phase].0;
                }
            }
            (phase, reste)
        };
        for (depart, (duree, _)) in MOTIF.iter().enumerate() {
            for l in [0.5_f32, 3.0, 12.0, 13.0, 22.0, 45.0, 1000.0, 123_456.0] {
                let attendu = lent(depart, *duree, l);
                let obtenu = avancer(depart, *duree, l);
                assert_eq!(obtenu.0, attendu.0, "phase après {l} px depuis {depart}");
                assert!(
                    (obtenu.1 - attendu.1).abs() < 0.01,
                    "reste après {l} px depuis {depart} : {} au lieu de {}",
                    obtenu.1,
                    attendu.1
                );
            }
        }
    }

    /// **Rien n'est encré hors de l'écran.** C'est la contrepartie du test
    /// précédent : le découpage doit vraiment couper, pas seulement borner le
    /// nombre de segments.
    #[test]
    fn aucun_segment_ne_sort_de_l_ecran() {
        let l = 50_000.0;
        // Une diagonale qui traverse l'écran de part en part, en débordant
        // très largement des deux côtés.
        let segments = segments_motif(&[vec2(-l, -l), vec2(l, l)], ECRAN);
        assert!(!segments.is_empty(), "la diagonale traverse l'écran, elle doit s'encrer");
        let marge = EPAISSEUR_PX + 0.01;
        for (a, b) in segments {
            for p in [a, b] {
                assert!(
                    p.x >= -marge && p.x <= ECRAN.x + marge && p.y >= -marge && p.y <= ECRAN.y + marge,
                    "segment encré hors écran : {p:?}"
                );
            }
        }
    }

    /// Le premier trait démarre **au début** de la polyligne : une orbite ne
    /// doit pas commencer par un trou, sinon deux lunes voisines paraissent
    /// décalées l'une de l'autre.
    #[test]
    fn le_motif_commence_par_de_l_encre() {
        let segments = segments_motif(&[vec2(10.0, 10.0), vec2(60.0, 10.0)], ECRAN);
        assert_eq!(segments[0].0, vec2(10.0, 10.0));
        assert!((segments[0].1.x - 22.0).abs() < 0.01, "premier trait de 12 px : {:?}", segments[0]);
    }
}
