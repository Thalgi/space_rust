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
    colonne_depuis(ecran, MARGE)
}

/// La colonne, mais **commençant à `haut`**.
///
/// La vue système pose un bandeau de ressources en haut de l'écran ; la colonne
/// doit passer dessous. Le décalage est **transmis** plutôt que recopié : deux
/// constantes à tenir d'accord finiraient par diverger, et les deux zones se
/// recouvriraient sans que rien ne le dise (`conception/interface.md` §1.2).
pub fn colonne_depuis(ecran: Vec2, haut: f32) -> Rect {
    colonne_depuis_echelle(ecran, haut, PART_LARGEUR)
}

/// Comme [`colonne_depuis`], avec une part de largeur d'écran **explicite**
/// plutôt que [`PART_LARGEUR`] — pour les colonnes qui ont besoin de plus de
/// place (des icônes rendues plus grandes, par exemple) sans faire bouger le
/// plafond des **autres** colonnes (la vitrine de composants), qui partagent
/// ce module sans avoir grossi.
pub fn colonne_depuis_echelle(ecran: Vec2, haut: f32, part_largeur: f32) -> Rect {
    let haut = haut.max(MARGE);
    Rect::new(MARGE, haut, ecran.x * part_largeur - MARGE * 2.0, (ecran.y - haut - MARGE).max(0.0))
}

/// Hauteur d'une ligne : la place disponible divisée par le nombre d'items,
/// bornée. Si tout ne rentre pas, la liste **déborde** — c'est visible, et
/// préférable à des lignes illisibles.
pub fn hauteur_ligne(colonne: Rect, n: usize) -> f32 {
    hauteur_ligne_echelle(colonne, n, 1.0)
}

/// Comme [`hauteur_ligne`], mais avec un multiplicateur explicite : pour les
/// colonnes qui posent plus qu'un simple libellé (une icône, par exemple) et
/// ont besoin de lignes plus hautes que le texte seul — au prix, sur une
/// longue liste, d'un débordement plus tôt (assumé, voir tête de fichier).
pub fn hauteur_ligne_echelle(colonne: Rect, n: usize, facteur: f32) -> f32 {
    if n == 0 {
        return LIGNE_MIN * facteur;
    }
    (colonne.h / n as f32).clamp(LIGNE_MIN * facteur, LIGNE_MAX * facteur)
}

/// Rectangle de la ligne `i`.
pub fn ligne(colonne: Rect, n: usize, i: usize) -> Rect {
    ligne_echelle(colonne, n, i, 1.0)
}

/// Comme [`ligne`], à l'échelle de [`hauteur_ligne_echelle`].
pub fn ligne_echelle(colonne: Rect, n: usize, i: usize, facteur: f32) -> Rect {
    let h = hauteur_ligne_echelle(colonne, n, facteur);
    Rect::new(colonne.x, colonne.y + h * i as f32, colonne.w, h - 1.0)
}

/// Décalage vertical, en pixels, pour ancrer une liste de `n` lignes (à
/// `facteur`) au **bas** de la colonne plutôt qu'en haut — l'espace que la
/// liste laisse **au-dessus** d'elle si elle ne remplit pas toute la
/// hauteur, zéro si elle la remplit ou la dépasse.
fn decalage_bas(colonne: Rect, n: usize, facteur: f32) -> f32 {
    let h = hauteur_ligne_echelle(colonne, n, facteur);
    (colonne.h - h * n as f32).max(0.0)
}

/// Comme [`ligne_echelle`], mais **ancrée en bas** de la colonne : une liste
/// trop courte pour remplir toute la hauteur colle à son bas, laissant le
/// vide au-dessus plutôt qu'en dessous.
pub fn ligne_echelle_bas(colonne: Rect, n: usize, i: usize, facteur: f32) -> Rect {
    let mut r = ligne_echelle(colonne, n, i, facteur);
    r.y += decalage_bas(colonne, n, facteur);
    r
}

/// Comme [`item_sous_curseur_echelle`], pour une liste ancrée en bas
/// ([`ligne_echelle_bas`]) — **doit** rester d'accord avec elle au même
/// facteur, sinon on ne clique plus sur ce qu'on voit. Ramène la souris au
/// même décalage que les lignes plutôt que de refaire le calcul : les deux
/// fonctions ne peuvent alors pas diverger sur le montant du décalage.
pub fn item_sous_curseur_echelle_bas(colonne: Rect, n: usize, souris: Vec2, facteur: f32) -> Option<usize> {
    if n == 0 || !colonne.contains(souris) {
        return None;
    }
    let decalage = decalage_bas(colonne, n, facteur);
    item_sous_curseur_echelle(colonne, n, vec2(souris.x, souris.y - decalage), facteur)
}

/// Index de l'item sous le curseur, s'il y en a un.
///
/// Renvoie `None` hors de la colonne **et** au-delà de la dernière ligne : sans
/// cette seconde borne, cliquer sous la liste sélectionnerait un item qui n'est
/// pas là.
pub fn item_sous_curseur(colonne: Rect, n: usize, souris: Vec2) -> Option<usize> {
    item_sous_curseur_echelle(colonne, n, souris, 1.0)
}

/// Comme [`item_sous_curseur`], à l'échelle de [`hauteur_ligne_echelle`] —
/// **doit** rester d'accord avec [`ligne_echelle`] au même facteur : c'est la
/// propriété qui fait qu'on clique sur ce qu'on voit.
pub fn item_sous_curseur_echelle(colonne: Rect, n: usize, souris: Vec2, facteur: f32) -> Option<usize> {
    if n == 0 || !colonne.contains(souris) {
        return None;
    }
    let h = hauteur_ligne_echelle(colonne, n, facteur);
    let i = ((souris.y - colonne.y) / h).floor();
    if i < 0.0 {
        return None;
    }
    let i = i as usize;
    (i < n).then_some(i)
}

// ---------------------------------------------------------------------------
// Défilement
// ---------------------------------------------------------------------------
//
// **Pourquoi il a fallu en ajouter un.** `hauteur_ligne` divise la place
// disponible par le nombre d'items, borné à `LIGNE_MIN`. Tant que la liste
// tenait, c'était juste ; passé une quarantaine d'entrées, deux choses
// arrivaient ensemble — les lignes tombaient au minimum (texte au plus petit,
// donc illisible) **et** le surplus était coupé net, sans que rien ne dise
// qu'il existait. Le catalogue des composants a franchi ce seuil.
//
// Une liste qui défile règle les deux : la ligne garde une hauteur confortable,
// et l'ascenseur dit à la fois **qu'il y a plus** et **où l'on en est**.

/// Hauteur d'une ligne défilante. **Fixe**, contrairement à
/// [`hauteur_ligne`] : c'est tout l'intérêt du défilement, la ligne cesse de
/// rétrécir quand la liste s'allonge.
pub const LIGNE_DEFIL: f32 = 24.0;

/// Largeur de l'ascenseur.
pub const ASCENSEUR_L: f32 = 6.0;

/// Combien de lignes entières la colonne peut montrer.
pub fn lignes_visibles(colonne: Rect) -> usize {
    (colonne.h / LIGNE_DEFIL).floor().max(0.0) as usize
}

/// Faut-il défiler ? Faux quand tout tient — et il ne faut alors **pas**
/// d'ascenseur, une barre qui occupe toute sa glissière ne dit rien.
pub fn defile(colonne: Rect, n: usize) -> bool {
    n > lignes_visibles(colonne)
}

/// Décalage maximal, en pixels. Zéro si tout tient.
///
/// Borné de façon que le **dernier item soit entièrement visible** en bout de
/// course : s'arrêter plus tôt laisserait une entrée qu'on ne peut jamais lire,
/// ce qui est exactement le défaut qu'on corrige.
pub fn defilement_max(colonne: Rect, n: usize) -> f32 {
    (n as f32 * LIGNE_DEFIL - colonne.h).max(0.0)
}

/// Ramène un décalage dans ses bornes.
pub fn borner_defilement(colonne: Rect, n: usize, offset: f32) -> f32 {
    offset.clamp(0.0, defilement_max(colonne, n))
}

/// Rectangle de la ligne `i`, décalé de `offset`. Peut sortir de la colonne :
/// c'est à l'appelant de sauter celles que [`ligne_visible`] écarte.
pub fn ligne_defilee(colonne: Rect, i: usize, offset: f32) -> Rect {
    Rect::new(
        colonne.x,
        colonne.y + i as f32 * LIGNE_DEFIL - offset,
        colonne.w - ASCENSEUR_L - 2.0,
        LIGNE_DEFIL - 2.0,
    )
}

/// La ligne se voit-elle, même en partie ?
pub fn ligne_visible(colonne: Rect, r: Rect) -> bool {
    r.y + r.h > colonne.y && r.y < colonne.y + colonne.h
}

/// Index de l'item sous le curseur, **décalage compris**.
///
/// Doit rester d'accord avec [`ligne_defilee`] : c'est la propriété qui fait
/// qu'on clique sur ce qu'on voit, et elle se teste pour les deux à la fois.
pub fn item_sous_curseur_defile(colonne: Rect, n: usize, offset: f32, souris: Vec2) -> Option<usize> {
    if n == 0 || !colonne.contains(souris) {
        return None;
    }
    let i = ((souris.y - colonne.y + offset) / LIGNE_DEFIL).floor();
    if i < 0.0 {
        return None;
    }
    let i = i as usize;
    (i < n).then_some(i)
}

/// Rail et curseur de l'ascenseur, ou `None` si tout tient.
///
/// Le curseur a une **taille minimale** : proportionnel sans plancher, il
/// deviendrait un trait invisible sur une liste très longue — or c'est
/// précisément là qu'on en a le plus besoin.
pub fn ascenseur(colonne: Rect, n: usize, offset: f32) -> Option<(Rect, Rect)> {
    if !defile(colonne, n) {
        return None;
    }
    let rail = Rect::new(colonne.x + colonne.w - ASCENSEUR_L, colonne.y, ASCENSEUR_L, colonne.h);
    let total = n as f32 * LIGNE_DEFIL;
    let h = (colonne.h * (colonne.h / total)).max(ASCENSEUR_L * 3.0).min(colonne.h);
    let course = colonne.h - h;
    let t = offset / defilement_max(colonne, n).max(1e-6);
    let curseur = Rect::new(rail.x, rail.y + course * t.clamp(0.0, 1.0), rail.w, h);
    Some((rail, curseur))
}

/// Côté du bouton de repli, en pixels. Assez large pour être visé au doigt
/// comme à la souris, assez petit pour ne pas manger une ligne d'item.
pub const COTE_REPLI: f32 = 22.0;

/// Rectangle du bouton `«` / `»`, en bas à gauche.
///
/// Sa position ne dépend **pas** de l'état de repli : le bouton ne bouge pas
/// quand on replie, sinon il faudrait aller le rechercher pour rouvrir.
pub fn bouton_repli(ecran: Vec2) -> Rect {
    bouton_repli_echelle(ecran, 1.0)
}

/// Comme [`bouton_repli`], avec un facteur d'échelle explicite — pour un
/// bouton plus grand sans toucher au bouton des **autres** colonnes qui
/// partagent ce module (la vitrine de composants), qui n'a pas grossi.
pub fn bouton_repli_echelle(ecran: Vec2, facteur: f32) -> Rect {
    let cote = COTE_REPLI * facteur;
    Rect::new(MARGE, ecran.y - MARGE - cote, cote, cote)
}

/// **Ce que la colonne mange à la souris**, selon qu'elle est repliée ou non.
///
/// Repliée, seul le bouton reste actif — le reste de la bande rend la main à la
/// caméra, faute de quoi replier la liste ne libérerait pas la vue et n'aurait
/// aucun intérêt.
///
/// Toujours appelée avec une part de largeur et un facteur de bouton
/// explicites (voir [`colonne_depuis_echelle`]/[`bouton_repli_echelle`]) :
/// **doit** rester d'accord avec ce qui est réellement dessiné, sinon la
/// caméra continue de tourner sous une colonne (ou un bouton) agrandi que la
/// souris croit pourtant avoir pris.
pub fn zone_active_echelle_bouton(
    ecran: Vec2,
    repliee: bool,
    haut: f32,
    part_largeur: f32,
    facteur_bouton: f32,
) -> Rect {
    if repliee {
        bouton_repli_echelle(ecran, facteur_bouton)
    } else {
        colonne_depuis_echelle(ecran, haut, part_largeur)
    }
}

/// La colonne, amputée de la place du bouton de repli : c'est là que les items
/// se posent. Sans ça, la dernière ligne passerait **sous** le bouton et serait
/// impossible à cliquer.
#[allow(dead_code)] // Interaction de l'assembleur (Lot 4) : deja teste, pas encore branche a une vue.
pub fn colonne_items(ecran: Vec2) -> Rect {
    colonne_items_depuis(ecran, MARGE)
}

/// Idem, sous un bandeau de hauteur `haut`.
pub fn colonne_items_depuis(ecran: Vec2, haut: f32) -> Rect {
    colonne_items_depuis_echelle(ecran, haut, PART_LARGEUR)
}

/// Comme [`colonne_items_depuis`], avec une part de largeur d'écran
/// explicite — voir [`colonne_depuis_echelle`].
pub fn colonne_items_depuis_echelle(ecran: Vec2, haut: f32, part_largeur: f32) -> Rect {
    colonne_items_depuis_echelle_bouton(ecran, haut, part_largeur, 1.0)
}

/// Comme [`colonne_items_depuis_echelle`], avec en plus un facteur d'échelle
/// pour le bouton de repli — **doit** rester d'accord avec celui réellement
/// dessiné, sinon la dernière ligne passe sous un bouton agrandi.
pub fn colonne_items_depuis_echelle_bouton(
    ecran: Vec2,
    haut: f32,
    part_largeur: f32,
    facteur_bouton: f32,
) -> Rect {
    let c = colonne_depuis_echelle(ecran, haut, part_largeur);
    let espace_bouton = COTE_REPLI * facteur_bouton;
    Rect::new(c.x, c.y, c.w, (c.h - espace_bouton - MARGE).max(0.0))
}

/// Le curseur est-il **sur la colonne** ? La caméra doit l'ignorer alors, sinon
/// tout clic dans la liste ferait aussi pivoter la vue derrière.
pub fn sur_la_liste(ecran: Vec2, souris: Vec2) -> bool {
    colonne(ecran).contains(souris)
}

/// Abscisse à partir de laquelle une vue peut dessiner **sans passer sous la
/// colonne**.
///
/// Les vues qui occupent tout l'écran (galeries) doivent rétrécir leur viewport
/// 3D d'autant, sinon la colonne masque une bande de l'image au lieu de la
/// côtoyer. **Repliée, la colonne ne prend plus rien** : la vue récupère toute
/// la largeur — c'est tout l'intérêt de pouvoir la replier.
pub fn bord_droit(ecran: Vec2, repliee: bool) -> f32 {
    if repliee {
        0.0
    } else {
        let c = colonne(ecran);
        c.x + c.w + MARGE
    }
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

    /// **Une vue plein écran doit pouvoir se ranger à côté de la colonne.**
    ///
    /// `bord_droit` donne l'abscisse à partir de laquelle dessiner sans passer
    /// dessous. Déployée, elle doit laisser passer TOUTE la colonne ; repliée,
    /// elle ne doit plus rien prendre — sinon replier ne servirait à rien.
    #[test]
    fn le_bord_droit_libere_la_place_de_la_colonne() {
        for l in [640.0_f32, 1000.0, 1920.0] {
            let ecran = vec2(l, 700.0);
            let col = colonne(ecran);
            let bord = bord_droit(ecran, false);
            assert!(
                bord >= col.x + col.w,
                "largeur {l} : le rendu démarre à {bord}, la colonne finit à {}",
                col.x + col.w
            );
            // Et elle ne mange pas plus que sa part : la vue garde l'essentiel.
            assert!(bord <= l * PART_LARGEUR + 2.0 * MARGE, "largeur {l} : {bord} px mangés");
            assert_eq!(bord_droit(ecran, true), 0.0, "repliée, la colonne doit tout rendre");
        }
    }

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

    // **Une part de largeur explicite élargit vraiment la colonne**, et
    // `colonne_depuis` (sans argument) reste au plafond de 10 % — sinon la
    // colonne du sélecteur aurait élargi celle de la vitrine de composants
    // avec elle.
    #[test]
    fn colonne_depuis_echelle_elargit_sans_toucher_au_plafond_par_defaut() {
        let e = vec2(1280.0, 700.0);
        let large = colonne_depuis_echelle(e, MARGE, 0.18);
        let standard = colonne_depuis(e, MARGE);
        assert!(large.w > standard.w, "large={} standard={}", large.w, standard.w);
        assert!((standard.w - (colonne(e).w)).abs() < 1e-3, "colonne() a dérivé de son plafond de 10 %");
    }

    // `zone_active_echelle_bouton` **doit** manger la même largeur que la
    // colonne réellement dessinée : sinon la caméra continuerait de tourner
    // sous la bande élargie, dans la zone entre l'ancien et le nouveau bord
    // droit.
    #[test]
    fn zone_active_echelle_couvre_toute_la_colonne_elargie() {
        let e = vec2(1280.0, 700.0);
        let part = 0.18;
        let zone = zone_active_echelle_bouton(e, false, MARGE, part, 1.0);
        let col = colonne_depuis_echelle(e, MARGE, part);
        assert!((zone.w - col.w).abs() < 1e-3, "zone={} colonne={}", zone.w, col.w);
    }

    // Même propriété, côté **bouton de repli** : sa zone active doit couvrir
    // le bouton réellement dessiné, agrandi ou non — sinon la partie
    // agrandie du bouton laisserait passer un clic à la caméra.
    #[test]
    fn zone_active_echelle_couvre_le_bouton_agrandi() {
        let e = vec2(1280.0, 700.0);
        let zone = zone_active_echelle_bouton(e, true, MARGE, PART_LARGEUR, 2.0);
        let bouton = bouton_repli_echelle(e, 2.0);
        assert_eq!(zone, bouton);
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
    /// **On clique sur ce qu'on voit, quel que soit le décalage.** C'est la
    /// propriété qui lie `ligne_defilee` et `item_sous_curseur_defile` : deux
    /// calculs séparés qui doivent parler du même écran, sinon la liste
    /// sélectionne une entrée voisine de celle qu'on a visée — le genre de
    /// défaut qu'on met longtemps à croire.
    #[test]
    fn le_clic_defile_retombe_sur_la_ligne_vue() {
        let col = colonne(vec2(1280.0, 720.0));
        let n = 41;
        for offset in [0.0_f32, 7.0, LIGNE_DEFIL, 100.0, defilement_max(col, n)] {
            let mut vues = 0;
            for i in 0..n {
                let r = ligne_defilee(col, i, offset);
                if !ligne_visible(col, r) {
                    continue;
                }
                // Un point franchement à l'intérieur de la ligne.
                let p = vec2(r.x + r.w * 0.5, r.y + r.h * 0.5);
                if !col.contains(p) {
                    continue; // ligne coupée par le bord : hors sujet
                }
                assert_eq!(
                    item_sous_curseur_defile(col, n, offset, p),
                    Some(i),
                    "offset {offset} : le clic au milieu de la ligne {i} désigne autre chose"
                );
                vues += 1;
            }
            assert!(vues > 3, "offset {offset} : trop peu de lignes visibles pour prouver quoi que ce soit");
        }
    }

    /// **En bout de course, le dernier item est entièrement lisible.**
    /// S'arrêter plus tôt laisserait une entrée qu'on ne peut jamais lire — le
    /// défaut même qu'on corrige.
    #[test]
    fn le_dernier_item_se_lit_en_bas_de_course() {
        let col = colonne(vec2(1280.0, 720.0));
        for n in [41_usize, 60, 200] {
            let max = defilement_max(col, n);
            let r = ligne_defilee(col, n - 1, max);
            assert!(r.y >= col.y - 0.5, "n={n} : le dernier item sort par le haut");
            assert!(
                r.y + r.h <= col.y + col.h + 0.5,
                "n={n} : le dernier item dépasse encore ({} > {})",
                r.y + r.h,
                col.y + col.h
            );
        }
    }

    /// **Pas d'ascenseur quand tout tient**, et un ascenseur dès que non. Une
    /// barre qui remplit toute sa glissière n'apprend rien et occupe de la place.
    #[test]
    fn l_ascenseur_n_apparait_que_s_il_sert() {
        let col = colonne(vec2(1280.0, 720.0));
        let tient = lignes_visibles(col);
        assert!(tient > 2, "colonne trop courte pour que le test ait un sens");
        assert!(ascenseur(col, tient, 0.0).is_none(), "ascenseur alors que tout tient");
        assert_eq!(defilement_max(col, tient), 0.0, "un décalage possible alors que tout tient");
        assert!(ascenseur(col, tient + 1, 0.0).is_some(), "pas d'ascenseur alors qu'un item déborde");
    }

    /// **Le curseur reste dans son rail**, à tout décalage, et garde une taille
    /// visible même sur une liste très longue — c'est là qu'on en a le plus
    /// besoin.
    #[test]
    fn le_curseur_de_l_ascenseur_reste_dans_son_rail() {
        let col = colonne(vec2(1280.0, 720.0));
        // ⚠️ Le dernier `n` est **hors d'échelle exprès**. Le plancher de taille
        // du curseur ne mord qu'au-delà d'un millier d'items environ (à 24 px de
        // ligne sur une colonne de ~700 px) — donc jamais avec les catalogues
        // réels, qui en comptent quarante. C'est une garde **défensive**, et
        // sans un `n` de cet ordre le red-check la laissait passer : retirer le
        // plancher ne rougissait rien.
        for n in [40_usize, 100, 5000] {
            let max = defilement_max(col, n);
            for t in [0.0_f32, 0.25, 0.5, 1.0] {
                let (rail, cur) = ascenseur(col, n, max * t).expect("devrait défiler");
                assert!(cur.y >= rail.y - 0.5, "n={n} t={t} : curseur au-dessus du rail");
                assert!(
                    cur.y + cur.h <= rail.y + rail.h + 0.5,
                    "n={n} t={t} : curseur sous le rail"
                );
                assert!(cur.h >= ASCENSEUR_L * 2.0, "n={n} : curseur trop fin pour être vu");
            }
        }
    }

    /// Le décalage est borné des deux côtés : ni au-dessus du premier item, ni
    /// au-delà du dernier.
    #[test]
    fn le_defilement_est_borne() {
        let col = colonne(vec2(1280.0, 720.0));
        let n = 41;
        assert_eq!(borner_defilement(col, n, -500.0), 0.0);
        assert_eq!(borner_defilement(col, n, 1e6), defilement_max(col, n));
        // Et une liste courte ne défile pas du tout.
        assert_eq!(borner_defilement(col, 2, 1e6), 0.0);
    }

    /// **La ligne défilante ne rétrécit pas.** C'est la raison d'être du
    /// défilement : `hauteur_ligne` tombait au minimum dès qu'il y avait trop
    /// d'items, et le texte avec elle.
    #[test]
    fn la_ligne_defilante_garde_sa_hauteur() {
        let col = colonne(vec2(1280.0, 720.0));
        for n in [3_usize, 41, 500] {
            let r = ligne_defilee(col, 0, 0.0);
            assert!((r.h - (LIGNE_DEFIL - 2.0)).abs() < 1e-4, "n={n} : hauteur {} inattendue", r.h);
        }
        // Alors que la version non défilante, elle, s'écrase.
        assert!(hauteur_ligne(col, 500) < LIGNE_DEFIL, "le défaut d'origine a disparu tout seul ?");
    }

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

    // **Un facteur d'échelle agrandit vraiment la ligne**, et pas seulement en
    // apparence : sur une colonne assez haute pour que le plancher/plafond ne
    // morde pas, une ligne à facteur 2 doit faire le double d'une ligne à
    // facteur 1. `hauteur_ligne` (facteur 1, implicite) et `hauteur_ligne_echelle`
    // ne doivent jamais diverger : la première **délègue** à la seconde.
    #[test]
    fn le_facteur_dechelle_double_vraiment_la_ligne() {
        let c = colonne(vec2(1280.0, 2000.0)); // haute : la colonne ne borne pas
        let n = 4;
        let simple = hauteur_ligne(c, n);
        let double = hauteur_ligne_echelle(c, n, 2.0);
        assert!((double - simple * 2.0).abs() < 1e-3, "simple={simple} double={double}");
    }

    // **Même propriété qu'au facteur 1** : on clique sur ce qu'on voit. Testée
    // séparément parce que `ligne_echelle` et `item_sous_curseur_echelle` sont
    // deux calculs distincts qui doivent parler du même écran — exactement le
    // genre de défaut que `le_clic_defile_retombe_sur_la_ligne_vue` garde déjà
    // pour la version défilante.
    #[test]
    fn le_clic_a_lechelle_retombe_sur_la_ligne_visee() {
        let c = colonne(ECRAN);
        for facteur in [1.0_f32, 2.0, 3.0] {
            for n in [1usize, 3, 12] {
                for i in 0..n {
                    let l = ligne_echelle(c, n, i, facteur);
                    let centre = vec2(l.x + l.w * 0.5, l.y + l.h * 0.5);
                    assert_eq!(
                        item_sous_curseur_echelle(c, n, centre, facteur),
                        Some(i),
                        "facteur={facteur} n={n} i={i}"
                    );
                }
            }
        }
    }

    // **Une liste trop courte pour remplir la colonne colle à son bas** :
    // la dernière ligne finit au bord bas de la colonne, pas quelque part au
    // milieu avec du vide en dessous.
    #[test]
    fn la_liste_ancree_en_bas_colle_au_bord_bas() {
        let c = colonne(vec2(1280.0, 2000.0)); // haute : la liste ne la remplit pas
        let n = 4;
        let derniere = ligne_echelle_bas(c, n, n - 1, 1.0);
        assert!(
            (derniere.y + derniere.h + 1.0 - (c.y + c.h)).abs() < 1.0,
            "la dernière ligne finit à {}, la colonne à {}",
            derniere.y + derniere.h,
            c.y + c.h
        );
        // Et la première ligne ne commence pas au sommet de la colonne : le
        // vide doit être au-dessus, pas en dessous.
        let premiere = ligne_echelle_bas(c, n, 0, 1.0);
        assert!(premiere.y > c.y + 1.0, "la liste ne colle pas au bas : {} ~ {}", premiere.y, c.y);
    }

    // **Une liste qui remplit (ou dépasse) la colonne se comporte comme
    // avant** : rien à décaler, l'ancrage en bas ne doit pas la faire sortir
    // par le haut.
    #[test]
    fn la_liste_ancree_en_bas_ne_bouge_pas_si_elle_remplit_la_colonne() {
        let c = colonne(ECRAN);
        let n = 60; // assez pour que LIGNE_MIN morde et remplisse la colonne
        let premiere_normale = ligne(c, n, 0);
        let premiere_ancree = ligne_echelle_bas(c, n, 0, 1.0);
        assert!((premiere_normale.y - premiere_ancree.y).abs() < 1e-3);
    }

    // **On clique sur ce qu'on voit**, ancrage en bas compris — même
    // propriété que `le_clic_a_lechelle_retombe_sur_la_ligne_visee`, mais
    // pour `ligne_echelle_bas`/`item_sous_curseur_echelle_bas`.
    #[test]
    fn le_clic_ancre_en_bas_retombe_sur_la_ligne_visee() {
        let c = colonne(vec2(1280.0, 2000.0));
        let n = 4;
        for i in 0..n {
            let l = ligne_echelle_bas(c, n, i, 1.0);
            let centre = vec2(l.x + l.w * 0.5, l.y + l.h * 0.5);
            assert_eq!(item_sous_curseur_echelle_bas(c, n, centre, 1.0), Some(i), "i={i}");
        }
        // Et un clic dans le vide laissé au-dessus de la liste ne désigne rien.
        let vide = vec2(c.x + 5.0, c.y + 5.0);
        assert_eq!(item_sous_curseur_echelle_bas(c, n, vide, 1.0), None, "clic dans le vide au-dessus");
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

    // **Le bouton de repli ne bouge pas** quand on replie : sinon il faudrait
    // aller le rechercher ailleurs pour rouvrir la colonne.
    #[test]
    fn le_bouton_de_repli_ne_bouge_pas() {
        for l in [640.0_f32, 1000.0, 1920.0] {
            let e = vec2(l, 700.0);
            let b = bouton_repli(e);
            // Dans la colonne, et en bas de l'écran.
            let c = colonne(e);
            assert!(b.x >= c.x - 1e-3 && b.x + b.w <= c.x + c.w + 1e-3, "largeur {l}");
            assert!(b.y + b.h <= e.y - 1e-3, "le bouton dépasse en bas");
            assert!(b.y > c.y + c.h * 0.5, "le bouton devrait être en bas de la colonne");
        }
    }

    // **Repliée, la colonne rend la main à la caméra** : seul le bouton reste
    // actif. Sans ça, replier la liste ne libérerait pas la vue derrière, et
    // n'aurait aucun intérêt.
    #[test]
    fn repliee_la_colonne_ne_mange_plus_que_son_bouton() {
        let e = ECRAN;
        let dans_la_bande = vec2(colonne(e).x + 5.0, colonne(e).y + 30.0);
        assert!(
            zone_active_echelle_bouton(e, false, MARGE, PART_LARGEUR, 1.0).contains(dans_la_bande),
            "déployée, la colonne prend le clic"
        );
        assert!(
            !zone_active_echelle_bouton(e, true, MARGE, PART_LARGEUR, 1.0).contains(dans_la_bande),
            "repliée, elle devrait le rendre"
        );
        // Le bouton, lui, reste pris dans les deux états.
        let sur_bouton = bouton_repli(e).center();
        assert!(zone_active_echelle_bouton(e, false, MARGE, PART_LARGEUR, 1.0).contains(sur_bouton));
        assert!(zone_active_echelle_bouton(e, true, MARGE, PART_LARGEUR, 1.0).contains(sur_bouton));
    }

    // Les items ne descendent **pas sous le bouton** : la dernière ligne serait
    // recouverte, donc impossible à cliquer.
    #[test]
    fn les_items_ne_passent_pas_sous_le_bouton_de_repli() {
        let e = ECRAN;
        let items = colonne_items(e);
        let bouton = bouton_repli(e);
        assert!(
            items.y + items.h <= bouton.y + 1e-3,
            "les items descendent à {} et le bouton commence à {}",
            items.y + items.h,
            bouton.y
        );
        // Et il reste de la place pour lister : un rectangle vide serait un bug
        // que le test précédent laisserait passer.
        assert!(items.h > ECRAN.y * 0.5, "la zone d'items est trop courte : {}", items.h);
    }

    // Même propriété, avec un bouton **agrandi** : sans le facteur transmis
    // à `colonne_items_depuis_echelle_bouton`, la zone d'items réserverait
    // toujours la place de l'ancien (petit) bouton, et la dernière ligne
    // passerait sous la partie agrandie.
    #[test]
    fn les_items_ne_passent_pas_sous_un_bouton_agrandi() {
        let e = ECRAN;
        let items = colonne_items_depuis_echelle_bouton(e, MARGE, PART_LARGEUR, 2.0);
        let bouton = bouton_repli_echelle(e, 2.0);
        assert!(
            items.y + items.h <= bouton.y + 1e-3,
            "les items descendent à {} et le bouton (agrandi) commence à {}",
            items.y + items.h,
            bouton.y
        );
    }

    #[test]
    fn la_liste_vide_ne_designe_rien() {
        let c = colonne(ECRAN);
        assert_eq!(item_sous_curseur(c, 0, vec2(c.x + 5.0, c.y + 5.0)), None);
    }
}
