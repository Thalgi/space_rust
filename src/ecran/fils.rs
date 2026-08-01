//! **Numérotation des fils** : chaque trait de charpente porte son numéro,
//! posé **dans une coupure du fil**.
//!
//! À quoi ça sert : désigner une pièce sans périphrase. « Le fil 37 est trop
//! long » vaut mieux que « la barre en haut à gauche, celle qui part en biais »,
//! et c'est la différence entre une correction en un aller-retour et cinq. Le
//! relevé est produit par [`crate::vaisseau::fils`], donc par **le même code**
//! que le dessin — les numéros ne peuvent pas désigner une autre géométrie que
//! celle affichée.
//!
//! ⚠️ **Le numéro coupe le fil, il ne se pose pas à côté.** Un chiffre flottant
//! près d'un enchevêtrement de barres n'appartient visiblement à aucune : on ne
//! sait pas laquelle il désigne, et l'outil rate son seul but. Le trait est donc
//! dessiné en **deux morceaux**, avec un trou au milieu, et le chiffre occupe le
//! trou — c'est la convention des cotes en dessin technique, pour exactement
//! cette raison.

use crate::vaisseau::{fils, Fil, GenreFil, Station};
use macroquad::prelude::*;

/// Couleur des fils numérotés. Jaune vif : ils doivent ressortir sur la coque et
/// se distinguer du cyan des enveloppes, avec lesquelles ils cohabitent.
const TRAIT: Color = Color { r: 1.00, g: 0.85, b: 0.20, a: 1.0 };

/// Taille du texte, en pixels.
const TEXTE: f32 = 15.0;

/// Demi-largeur de la coupure, **en pixels d'écran**.
///
/// En pixels et non en unités monde : la coupure doit rester juste assez large
/// pour le chiffre quelle que soit la distance de la caméra. Calculée en monde,
/// elle avalerait un fil entier de loin et disparaîtrait de près.
const COUPURE_PX: f32 = 13.0;

/// Un fil trop court à l'écran ne peut pas porter de numéro lisible : le couper
/// n'en laisserait rien. Sous ce seuil (en pixels), on le laisse entier et sans
/// chiffre — mieux vaut un fil muet qu'un chiffre illisible collé à trois autres.
const LONGUEUR_MINI_PX: f32 = 34.0;

/// Projette un point monde en pixels écran. `None` s'il est derrière la caméra.
fn projeter(vp: Mat4, p: Vec3) -> Option<Vec2> {
    let clip = vp * p.extend(1.0);
    if clip.w <= 1e-4 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some(vec2(
        (ndc.x * 0.5 + 0.5) * screen_width(),
        (1.0 - (ndc.y * 0.5 + 0.5)) * screen_height(),
    ))
}

/// Dessine un fil coupé en deux et son numéro dans la coupure.
///
/// Le tracé est fait **en 2D**, après projection : c'est ce qui permet de
/// dimensionner la coupure en pixels, donc de garder le chiffre lisible à toute
/// distance. Un tracé 3D imposerait une coupure en unités monde, avec les deux
/// défauts décrits sur [`COUPURE_PX`].
fn fil_numerote(vp: Mat4, f: &Fil, transforme: Mat4) {
    let (a, b) = (transforme.transform_point3(f.a), transforme.transform_point3(f.b));
    let (Some(pa), Some(pb)) = (projeter(vp, a), projeter(vp, b)) else {
        return;
    };
    let etiquette = format!("{}", f.numero);

    // Un volume ramassé (sphère, cube cubique) se projette en un point : pas de
    // fil à couper, on pose le chiffre dessus.
    let long = pa.distance(pb);
    if long < LONGUEUR_MINI_PX {
        if long < 1.0 {
            crate::police::texte(&etiquette, pa.x, pa.y, TEXTE, TRAIT);
        } else {
            draw_line(pa.x, pa.y, pb.x, pb.y, 1.0, TRAIT);
        }
        return;
    }

    let dir = (pb - pa) / long;
    let mid = (pa + pb) * 0.5;
    // Coupure proportionnée au **nombre de chiffres** : « 7 » n'a pas besoin de
    // la même place que « 128 », et une coupure fixe laisserait le trait passer
    // à travers les nombres longs.
    let demi = COUPURE_PX * (0.5 + 0.25 * etiquette.len() as f32);
    let c1 = mid - dir * demi;
    let c2 = mid + dir * demi;
    draw_line(pa.x, pa.y, c1.x, c1.y, 1.0, TRAIT);
    draw_line(c2.x, c2.y, pb.x, pb.y, 1.0, TRAIT);
    // Le chiffre est centré sur la coupure : `police::texte` ancre en haut à
    // gauche, d'où le recentrage à la main.
    crate::police::texte(
        &etiquette,
        mid.x - TEXTE * 0.30 * etiquette.len() as f32,
        mid.y + TEXTE * 0.35,
        TEXTE,
        TRAIT,
    );
}

/// Numérote les fils d'une station entière.
///
/// La numérotation est **par pièce** : chaque composant repart de 0, parce que
/// c'est le composant qu'on corrige, pas l'assemblage. Sur l'ISV, dire « pièce
/// 12, fil 37 » désigne une barre de façon unique et renvoie directement à la
/// fonction `dessiner` à modifier.
///
/// À appeler **après** `set_default_camera()` : le tracé est en 2D.
pub fn station(st: &Station, vp: Mat4, piece_visee: Option<usize>) {
    for (i, piece) in st.pieces().iter().enumerate() {
        if piece_visee.is_some_and(|v| v != i) {
            continue;
        }
        for f in fils(&piece.composant) {
            // Les mailles brutes n'ont pas d'axe : leur « fil » est une diagonale
            // d'englobant qui ne longe aucune arête réelle. La numéroter
            // induirait en erreur — on la saute.
            if f.genre == GenreFil::Maille {
                continue;
            }
            fil_numerote(vp, &f, piece.transforme);
        }
    }
}

/// Nombre de fils numérotables d'une pièce — pour l'afficher dans le bandeau.
///
/// Pas encore consommé : il servira quand la vue affichera « pièce 12 · 37 fils »
/// à côté de la bascule, ce qui suppose d'avoir choisi une pièce (donc le
/// picking de l'assembleur).
#[allow(dead_code)]
pub fn compte(st: &Station, piece_visee: Option<usize>) -> usize {
    st.pieces()
        .iter()
        .enumerate()
        .filter(|(i, _)| piece_visee.is_none_or(|v| v == *i))
        .map(|(_, p)| fils(&p.composant).iter().filter(|f| f.genre != GenreFil::Maille).count())
        .sum()
}
