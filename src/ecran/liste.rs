//! **Liste des items de la vue courante**, en colonne à gauche.
//!
//! Jusqu'ici on ne changeait d'item qu'en **cyclant** (touche `D`) : pour
//! atteindre le 27ᵉ des 28 composants, vingt-sept pressions, et aucun moyen de
//! savoir ce qui existe sans faire le tour. La colonne rend le catalogue
//! **visible** et chaque entrée atteignable en un clic.
//!
//! # Contrainte de largeur
//!
//! Un dixième de l'écran, pas plus — c'est la demande, et elle est structurante :
//! à 1000 px de large la colonne fait **100 px**, où un libellé comme
//! « PANNEAUX MEGASTRUCTURE : 4 FAMILLES » ne rentre pas. D'où deux
//! conséquences assumées :
//!
//! 1. le texte est **tronqué** à la largeur disponible, mesuré et non deviné ;
//! 2. la hauteur de ligne s'adapte au nombre d'items, entre un plancher lisible
//!    et un plafond au-delà duquel trois entrées feraient des pavés.
//!
//! # Ce qui est ici, et ce qui n'y est pas
//!
//! Le **calcul** (rectangle de la colonne, rectangle d'une ligne, item sous le
//! curseur, troncature) vit ici et se teste. Le **dessin** reste dans la vue.
//! C'est la règle posée en `conception/assembleur.md` §10.9 : tout ce qui se
//! décide sort du code de dessin, et ce qui y reste ne doit plus être que « où
//! le rectangle se pose ».

use macroquad::prelude::*;

/// Part de la largeur d'écran occupée par la colonne. **Un dixième**, plafond
/// demandé.
pub const PART_LARGEUR: f32 = 0.10;
/// Hauteur de ligne minimale : en dessous, deux lignes se touchent et le texte
/// n'a plus de place verticale.
const LIGNE_MIN: f32 = 16.0;
/// Hauteur de ligne maximale : sans plafond, une vue à trois items étalerait
/// trois pavés sur toute la hauteur.
const LIGNE_MAX: f32 = 30.0;
/// Marge autour de la colonne.
const MARGE: f32 = 6.0;

/// Rectangle de la colonne, pour un écran donné.
pub fn colonne(ecran: Vec2) -> Rect {
    Rect::new(MARGE, MARGE, ecran.x * PART_LARGEUR - MARGE * 2.0, ecran.y - MARGE * 2.0)
}

/// Hauteur d'une ligne : la place disponible divisée par le nombre d'items,
/// bornée. Si tout ne rentre pas, la liste **déborde** — c'est visible, et
/// préférable à des lignes illisibles.
pub fn hauteur_ligne(colonne: Rect, n: usize) -> f32 {
    if n == 0 {
        return LIGNE_MIN;
    }
    (colonne.h / n as f32).clamp(LIGNE_MIN, LIGNE_MAX)
}

/// Rectangle de la ligne `i`.
pub fn ligne(colonne: Rect, n: usize, i: usize) -> Rect {
    let h = hauteur_ligne(colonne, n);
    Rect::new(colonne.x, colonne.y + h * i as f32, colonne.w, h - 1.0)
}

/// Index de l'item sous le curseur, s'il y en a un.
///
/// Renvoie `None` hors de la colonne **et** au-delà de la dernière ligne : sans
/// cette seconde borne, cliquer sous la liste sélectionnerait un item qui n'est
/// pas là.
pub fn item_sous_curseur(colonne: Rect, n: usize, souris: Vec2) -> Option<usize> {
    if n == 0 || !colonne.contains(souris) {
        return None;
    }
    let h = hauteur_ligne(colonne, n);
    let i = ((souris.y - colonne.y) / h).floor();
    if i < 0.0 {
        return None;
    }
    let i = i as usize;
    (i < n).then_some(i)
}

/// Le curseur est-il **sur la colonne** ? La caméra doit l'ignorer alors, sinon
/// tout clic dans la liste ferait aussi pivoter la vue derrière.
pub fn sur_la_liste(ecran: Vec2, souris: Vec2) -> bool {
    colonne(ecran).contains(souris)
}

/// Ce qu'on **affiche** d'un libellé : ce qui précède le deux-points.
///
/// Les libellés du catalogue disent la vitrine entière — « PANNEAUX :
/// 5 VARIANTES », « RAPTOR : ATMOSPHERIQUE vs VIDE ». Dans une colonne de
/// 100 px, la partie après le deux-points ne tient pas et n'apprend rien : le
/// sujet est **avant**. On la coupe donc à la source plutôt que de la laisser
/// se faire tronquer en « PANNEAUX : 5 VARI… ».
///
/// Pas de moteur d'expressions régulières pour ça : `split_once` **est** la
/// règle « tout ce qui précède le premier deux-points », en une ligne et sans
/// dépendance. Ajouter la caisse `regex` pour un séparateur fixe coûterait
/// plus que ce qu'elle rendrait.
///
/// Le libellé **complet** reste celui de l'item : il sert au titre de la vue et
/// aux tests d'unicité. Seul l'affichage de la colonne est abrégé.
pub fn abrege(libelle: &str) -> &str {
    libelle.split_once(':').map_or(libelle, |(avant, _)| avant).trim_end()
}

/// Tronque `libelle` à ce qui tient dans `largeur`, en ajoutant une ellipse.
///
/// Mesuré avec la **vraie** police, jamais estimé au nombre de caractères : les
/// glyphes n'ont pas tous la même chasse, et une troncature au compte déborderait
/// sur les libellés riches en majuscules larges.
pub fn tronquer(libelle: &str, largeur: f32, taille: u16) -> String {
    if crate::police::mesure(libelle, taille) <= largeur {
        return libelle.to_string();
    }
    let mut court = String::new();
    for c in libelle.chars() {
        let essai = format!("{court}{c}…");
        if crate::police::mesure(&essai, taille) > largeur {
            break;
        }
        court.push(c);
    }
    if court.is_empty() {
        // La colonne est plus étroite qu'un seul caractère : on rend vide
        // plutôt que de dessiner une ellipse qui déborde quand même.
        return String::new();
    }
    format!("{court}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ECRAN: Vec2 = Vec2::new(1000.0, 700.0);

    // La contrainte de l'étape, et elle est chiffrée : **un dixième**, jamais
    // plus. Vérifiée sur plusieurs largeurs d'écran, pas seulement la nominale.
    #[test]
    fn la_colonne_ne_depasse_jamais_un_dixieme_de_lecran() {
        for l in [640.0_f32, 1000.0, 1440.0, 1920.0, 3840.0] {
            let c = colonne(vec2(l, 700.0));
            // Comparé à **un dixième littéral** et non à `PART_LARGEUR` : se
            // mesurer contre sa propre constante, c'est réciter la formule, et
            // le test suivrait n'importe quelle valeur qu'on lui donnerait.
            assert!(
                c.x + c.w <= l / 10.0 + 1e-3,
                "largeur {l} : la colonne va jusqu'à {} pour un plafond de {}",
                c.x + c.w,
                l / 10.0
            );
        }
    }

    // Toute ligne reste **dans** la colonne : une ligne plus large déborderait
    // sur la vue 3D, et le plafond ci-dessus ne dirait plus rien.
    #[test]
    fn les_lignes_tiennent_dans_la_colonne() {
        let c = colonne(ECRAN);
        for n in [1usize, 3, 12, 28, 60] {
            for i in 0..n {
                let l = ligne(c, n, i);
                assert!(l.x >= c.x - 1e-3 && l.x + l.w <= c.x + c.w + 1e-3, "n={n} i={i}");
            }
        }
    }

    // Cliquer sur une ligne doit rendre **cette** ligne. Sondé au centre de
    // chacune, sur plusieurs tailles de liste — viser une seule ligne ne
    // distinguerait pas un calcul juste d'un calcul qui rendrait toujours 0.
    #[test]
    fn le_clic_retombe_sur_la_ligne_visee() {
        let c = colonne(ECRAN);
        for n in [1usize, 3, 12, 28] {
            for i in 0..n {
                let l = ligne(c, n, i);
                let centre = vec2(l.x + l.w * 0.5, l.y + l.h * 0.5);
                assert_eq!(item_sous_curseur(c, n, centre), Some(i), "n={n} i={i}");
            }
        }
    }

    // Hors colonne, et **sous la dernière ligne**, rien n'est désigné. Le
    // second cas est le piège : la colonne descend jusqu'en bas de l'écran, donc
    // un clic dans son vide appartient bien au rectangle sans appartenir à
    // aucune ligne.
    #[test]
    fn hors_liste_rien_nest_designe() {
        let c = colonne(ECRAN);
        let n = 3usize;
        assert_eq!(item_sous_curseur(c, n, vec2(500.0, 300.0)), None, "à droite");
        assert_eq!(item_sous_curseur(c, n, vec2(c.x + 5.0, c.y - 20.0)), None, "au-dessus");
        let sous = c.y + hauteur_ligne(c, n) * n as f32 + 5.0;
        assert!(sous < c.y + c.h, "le scénario doit viser le vide *dans* la colonne");
        assert_eq!(item_sous_curseur(c, n, vec2(c.x + 5.0, sous)), None, "sous la dernière");
    }

    #[test]
    fn labrege_coupe_au_deux_points() {
        assert_eq!(abrege("PANNEAUX : 5 VARIANTES"), "PANNEAUX");
        assert_eq!(abrege("RAPTOR : ATMOSPHERIQUE vs VIDE"), "RAPTOR");
        // Sans deux-points, rien ne bouge.
        assert_eq!(abrege("· MODULE AXIAL"), "· MODULE AXIAL");
        assert_eq!(abrege("NOEUDS 4 / 6 / T / TETRA"), "NOEUDS 4 / 6 / T / TETRA");
        // Plusieurs deux-points : on coupe au **premier**.
        assert_eq!(abrege("A : B : C"), "A");
    }

    // Abréger raccourcit, donc **rapproche** : deux entrées distinctes peuvent
    // se réduire au même bouton. C'est le risque propre à cette étape, et il ne
    // se verrait qu'à l'écran, sur deux lignes jumelles.
    #[test]
    fn deux_items_ne_se_reduisent_pas_au_meme_bouton() {
        for (cat, table) in crate::ecran::catalogue::TOUTES {
            let liste = crate::ecran::catalogue::items(table, cat == "BRIQUES");
            let vus: Vec<&str> = liste.iter().map(|i| abrege(&i.libelle)).collect();
            for (i, a) in vus.iter().enumerate() {
                for b in vus.iter().skip(i + 1) {
                    assert_ne!(a, b, "{cat} : deux boutons abrégés identiques");
                }
            }
        }
    }

    #[test]
    fn la_liste_vide_ne_designe_rien() {
        let c = colonne(ECRAN);
        assert_eq!(item_sous_curseur(c, 0, vec2(c.x + 5.0, c.y + 5.0)), None);
    }
}
