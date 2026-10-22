//! **L'icône de la fenêtre**, tirée de `assets/logo/model.ico`.
//!
//! miniquad attend trois images RGBA carrées (16, 32 et 64 px) ; le fichier
//! n'en porte qu'une, de 128 px. Il est **embarqué** dans l'exécutable : le jeu
//! ne dépend pas du dossier d'où on le lance, et `window_conf` s'exécute avant
//! toute boucle de chargement asynchrone.
//!
//! Le `.ico` est décodé à la main plutôt qu'en ajoutant une dépendance : c'est
//! une image **BMP 32 bits** derrière deux petits en-têtes, quelques lignes en
//! tout. Un `.ico` à image PNG compressée n'est pas lu — la fenêtre garde alors
//! l'icône par défaut, sans planter.

use macroquad::miniquad::conf::Icon;

const ICO: &[u8] = include_bytes!("../assets/logo/model.ico");

/// L'icône de la fenêtre, ou `None` si le fichier n'est pas lisible.
pub fn icone() -> Option<Icon> {
    let (cote, rgba) = decoder_ico(ICO)?;
    Some(Icon {
        small: reduire(&rgba, cote, 16).try_into().ok()?,
        medium: reduire(&rgba, cote, 32).try_into().ok()?,
        big: reduire(&rgba, cote, 64).try_into().ok()?,
    })
}

fn u16_le(o: &[u8], i: usize) -> Option<usize> {
    Some(u16::from_le_bytes(o.get(i..i + 2)?.try_into().ok()?) as usize)
}

fn u32_le(o: &[u8], i: usize) -> Option<usize> {
    Some(u32::from_le_bytes(o.get(i..i + 4)?.try_into().ok()?) as usize)
}

/// **Première image d'un `.ico`**, en RGBA de haut en bas : `(côté, pixels)`.
///
/// Format : en-tête de 6 octets (type 1 = icône), une entrée de 16 octets par
/// image (dont la taille et le décalage de ses données), puis un BMP **sans**
/// en-tête de fichier — en-tête d'information, pixels BGRA **de bas en haut**,
/// et un masque 1 bit que l'alpha 32 bits rend inutile.
pub(crate) fn decoder_ico(o: &[u8]) -> Option<(usize, Vec<u8>)> {
    if u16_le(o, 2)? != 1 || u16_le(o, 4)? == 0 {
        return None;
    }
    let debut = u32_le(o, 18)?;
    // Une image PNG compressée : non lue (voir la doc du module).
    if o.get(debut..debut + 4)? == [0x89, b'P', b'N', b'G'] {
        return None;
    }
    let entete = u32_le(o, debut)?;
    let cote = u32_le(o, debut + 4)?;
    // La hauteur du BMP compte l'image ET son masque : le double du côté.
    if u32_le(o, debut + 8)? != cote * 2 || u16_le(o, debut + 14)? != 32 || cote == 0 {
        return None;
    }
    let pixels = o.get(debut + entete..debut + entete + cote * cote * 4)?;
    let mut rgba = vec![0u8; cote * cote * 4];
    for y in 0..cote {
        // De bas en haut dans le fichier.
        let source = &pixels[(cote - 1 - y) * cote * 4..(cote - y) * cote * 4];
        for x in 0..cote {
            let (s, d) = (&source[x * 4..x * 4 + 4], y * cote * 4 + x * 4);
            rgba[d..d + 4].copy_from_slice(&[s[2], s[1], s[0], s[3]]);
        }
    }
    Some((cote, rgba))
}

/// **Réduit une image carrée** de `cote` à `n` px par moyenne de blocs,
/// **pondérée par l'alpha** : sans ça, les pixels transparents (noirs, le plus
/// souvent) assombriraient le contour de l'icône réduite.
pub(crate) fn reduire(rgba: &[u8], cote: usize, n: usize) -> Vec<u8> {
    let mut out = vec![0u8; n * n * 4];
    for ty in 0..n {
        for tx in 0..n {
            let (x0, x1) = (tx * cote / n, ((tx + 1) * cote / n).max(tx * cote / n + 1));
            let (y0, y1) = (ty * cote / n, ((ty + 1) * cote / n).max(ty * cote / n + 1));
            let (mut r, mut g, mut b, mut a, mut nb) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = &rgba[(y * cote + x) * 4..(y * cote + x) * 4 + 4];
                    let pa = p[3] as u32;
                    r += p[0] as u32 * pa;
                    g += p[1] as u32 * pa;
                    b += p[2] as u32 * pa;
                    a += pa;
                    nb += 1;
                }
            }
            let d = (ty * n + tx) * 4;
            // Tout transparent : le pixel reste à zéro.
            if let (Some(r), Some(g), Some(b)) = (r.checked_div(a), g.checked_div(a), b.checked_div(a)) {
                out[d..d + 4].copy_from_slice(&[r as u8, g as u8, b as u8, (a / nb) as u8]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un `.ico` de 2 × 2 fabriqué à la main : rouge en haut à gauche, vert en
    /// haut à droite, bleu en bas à gauche, transparent en bas à droite.
    fn ico_2x2() -> Vec<u8> {
        let mut o = vec![0, 0, 1, 0, 1, 0]; // réservé, type icône, 1 image
        o.extend_from_slice(&[2, 2, 0, 0, 1, 0, 32, 0]);
        o.extend_from_slice(&(40u32 + 16 + 8).to_le_bytes()); // taille des données
        o.extend_from_slice(&22u32.to_le_bytes()); // décalage
        // En-tête d'information BMP : 40 octets.
        let mut h = vec![0u8; 40];
        h[0..4].copy_from_slice(&40u32.to_le_bytes());
        h[4..8].copy_from_slice(&2u32.to_le_bytes());
        h[8..12].copy_from_slice(&4u32.to_le_bytes()); // image + masque
        h[12..14].copy_from_slice(&1u16.to_le_bytes());
        h[14..16].copy_from_slice(&32u16.to_le_bytes());
        o.extend_from_slice(&h);
        // BGRA, ligne du BAS d'abord.
        o.extend_from_slice(&[255, 0, 0, 255, 0, 0, 0, 0]); // bleu, transparent
        o.extend_from_slice(&[0, 0, 255, 255, 0, 255, 0, 255]); // rouge, vert
        o.extend_from_slice(&[0u8; 8]); // masque
        o
    }

    /// **Les lignes sont remises à l'endroit et les canaux dans l'ordre RGBA.**
    #[test]
    fn l_ico_se_lit_a_l_endroit() {
        let (cote, px) = decoder_ico(&ico_2x2()).expect("ico 2x2 illisible");
        assert_eq!(cote, 2);
        assert_eq!(&px[0..4], &[255, 0, 0, 255], "haut gauche : rouge attendu");
        assert_eq!(&px[4..8], &[0, 255, 0, 255], "haut droite : vert attendu");
        assert_eq!(&px[8..12], &[0, 0, 255, 255], "bas gauche : bleu attendu");
        assert_eq!(px[15], 0, "bas droite : transparent attendu");
    }

    /// **La réduction ne noircit pas les bords** : un pixel transparent ne
    /// compte pas dans la couleur, seulement dans l'opacité.
    #[test]
    fn la_reduction_ignore_la_couleur_du_transparent() {
        let (cote, px) = decoder_ico(&ico_2x2()).unwrap();
        let un = reduire(&px, cote, 1);
        // Moyenne de rouge, vert, bleu (le transparent ne colore pas) : 85 chacun.
        assert_eq!(&un[0..3], &[85, 85, 85]);
        assert_eq!(un[3], (255 * 3 / 4) as u8, "opacité : trois pixels pleins sur quatre");
    }

    /// **Le vrai fichier se lit**, à sa taille, et donne les trois tailles
    /// qu'attend la fenêtre. Sans quoi la fenêtre garderait l'icône par défaut
    /// sans que rien ne le dise.
    #[test]
    fn l_icone_du_jeu_se_lit() {
        let (cote, px) = decoder_ico(ICO).expect("model.ico illisible");
        assert_eq!(cote, 128);
        assert!(px.chunks(4).any(|p| p[3] > 0), "icône entièrement transparente");
        assert!(icone().is_some(), "les trois tailles de l'icône ne se construisent pas");
    }
}
