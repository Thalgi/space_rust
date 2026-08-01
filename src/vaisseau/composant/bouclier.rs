//! **Boucliers de tête de l'ISV** : les plaques hexagonales qui parent la
//! poussière interstellaire pendant la croisière.
//!
//! Ce n'est **pas** le bouclier thermique de l'épine (celui-là protège de
//! l'échappement des moteurs et n'est qu'un détail de surface) : celui-ci
//! protège d'un milieu qui ne bouge pas, mais que le vaisseau traverse à 0,7 c.
//! À cette vitesse un grain de poussière n'est plus un impact mais une
//! détonation.
//!
//! La tête en porte **quatre**, enfilés sur un même mât : un [petit](ports) en
//! premier, puis **trois grands identiques**. C'est un étagement — la première
//! plaque vaporise le grain, les suivantes encaissent un nuage déjà dilué, et
//! c'est l'espacement entre elles qui fait le travail, pas leur épaisseur.
//! D'où deux composants et non un seul réglable : la petite plaque est une
//! **pièce de structure**, nervurée et striée, tandis que les grandes sont des
//! **miroirs** bleutés sur leurs deux faces.
//!
//! Forme commune : un hexagone **pointe en haut**, dans le plan local XY, percé
//! d'un moyeu que traverse le mât. Les grandes plaques sont le même hexagone
//! **étiré** selon Y — deux longs bords parallèles, une pointe en haut et une en
//! bas. L'axe **+Z regarde vers l'avant** (côté poussière) ; les deux ports sont
//! aux bouts du moyeu, parce qu'on enfile ces plaques au lieu de les bouter.
//!
//! Nervures : elles sont **coniques**, épaisses au moyeu et effilées à la jante.
//! Ce n'est pas une coquetterie — c'est la répartition d'un longeron en flexion,
//! et c'est aussi ce qui donne, vu par la tranche, le profil en nœud papillon du
//! schéma plutôt qu'une simple barre. Leur **tracé**, en revanche, diffère d'une
//! plaque à l'autre : le petit bouclier porte douze nervures et une ceinture
//! (voir [`ossature`]), la grande n'en porte que huit, toutes rayonnantes et
//! rien en travers (voir [`motif_grand`]).

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::Enveloppe;
use crate::vaisseau::pieces;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_3, PI};

use super::commun::*;

/// **Miroir** des grandes plaques : clair et franchement **bleuté**.
///
/// Le rendu est à plat (une couleur par sommet, pas d'éclairage) : « miroir » ne
/// peut donc pas se dire par une réflexion, seulement par une **valeur haute et
/// une teinte froide**, à l'opposé de l'alu neutre de toute la structure. C'est
/// aussi ce qui sépare au premier coup d'œil les trois grandes plaques de la
/// petite, qui reste métallique.
const MIROIR: Color = Color { r: 0.56, g: 0.68, b: 0.88, a: 1.0 };

/// Modulation de valeur d'un secteur à l'autre, sur le **dos du petit
/// bouclier** — et là seulement.
///
/// Sans elle, un panneau à plat est une tache d'une seule couleur et ses six
/// secteurs ne se lisent pas. Les coefficients sont **fixes** (un maillage cuit
/// doit être identique d'une frame à l'autre) et volontairement irréguliers,
/// pour que la face n'ait pas l'air peinte en dégradé.
///
/// ⚠️ Ne pas l'appliquer aux grandes plaques : six triangles de valeurs
/// différentes rayonnant d'un moyeu, c'est le dessin d'une **pierre taillée**.
/// C'est exactement le grief relevé à l'écran sur la première version — voir
/// [`grand_dessiner`].
const FACETTES: [f32; 6] = [1.00, 0.87, 0.95, 1.06, 0.89, 0.99];

/// Rayon du moyeu, en fraction du rayon de plaque.
const MOYEU: f32 = 0.09;
/// Demi-longueur du moyeu : il dépasse des deux côtés, puisque c'est lui qui
/// enfile la plaque sur le mât.
const MOYEU_DEMI: f32 = 0.09;
/// Alésage du moyeu, en fraction du moyeu : les quatre boucliers sont **enfilés
/// sur un mât commun**, pas boutés l'un contre l'autre.
///
/// ⚠️ Cette fraction est **liée au moyeu et au mât à la fois**. Le moyeu ayant
/// été resserré (0,16 → 0,09 du rayon de plaque), un alésage à 0,45 ne laissait
/// plus que 0,223 de passage pour un mât qui en fait 0,290 : la petite plaque
/// était **empalée** au lieu d'être enfilée. Porté à 0,75, le trou revient à
/// 0,371 et le mât repasse. Le moyeu est devenu une **bague** (paroi de 0,124
/// pour un extérieur de 0,495) plutôt qu'un disque percé — ce qui est d'ailleurs
/// plus juste pour une pièce qu'on enfile.
const ALESAGE: f32 = 0.75;

/// Demi-épaisseur de la nappe. Deux peaux plutôt qu'une seule à double face :
/// une plaque doit avoir une tranche, sinon elle lit comme une découpe de
/// papier dès qu'on la regarde de biais.
const PEAU: f32 = 0.015;
/// Cordon de jante, qui ferme les deux peaux sur le pourtour.
const JANTE: f32 = 0.028;

/// Section d'une nervure principale (moyeu → sommet) à sa racine.
const NERVURE: f32 = 0.055;
/// Section d'une nervure secondaire (moyeu → milieu d'arête).
const NERVURE_FINE: f32 = 0.044;
/// Section des nervures d'une grande plaque. Plus fines que celles du petit
/// bouclier : sur le schéma ce sont des **traits** posés sur le miroir, pas les
/// membrures apparentes d'une pièce de structure — et une nervure épaisse
/// mangerait la surface réfléchissante qu'elle est censée tenir.
const NERVURE_MIROIR: f32 = 0.050;
/// Section au bout de la nervure, en fraction de sa racine.
const EFFILEMENT: f32 = 0.12;
/// Position de la ceinture le long des nervures (0 au moyeu, 1 à la jante).
const CEINTURE: f32 = 0.58;

/// Nombre de lames par secteur sur la face striée du petit bouclier. Six par
/// secteur font trente-six stries sur le tour — assez pour que la face lise
/// comme un rayonnage et pas comme un éventail de facettes.
const STRIES: usize = 6;

/// Élancement des grandes plaques.
///
/// C'est la **demi-hauteur**, en multiples du rayon — et rien d'autre : la
/// largeur relève de [`ETROITESSE`], qui se règle indépendamment. Les deux
/// ensemble donnent le rapport hauteur/largeur `2e / (√3 · étroitesse)`.
///
/// Valeur d'origine relevée sur la photo du schéma (≈ 298 px de haut pour 220 de
/// large, soit 1,35 de rapport, d'où `e ≈ 1,17`), arrondie à **1,30** parce que
/// le cliché est pris de biais et raccourcit le grand axe.
///
/// (Première valeur essayée : 1,75. Beaucoup trop — c'est ce qui faisait lire la
/// plaque comme une pierre taillée en long.)
pub const ELANCEMENT: f32 = 1.30;

/// Largeur du **méplat** qui rogne les deux pointes d'une grande plaque, en
/// fraction de la largeur de la plaque.
///
/// Les deux schémas le dessinent, en haut comme en bas : la pointe n'est pas
/// franche, elle est coupée par un petit bord droit. `contour()` étant écrit
/// comme un rognage, cette fraction *est* directement la largeur du bord —
/// `TAB × largeur` — sans autre cote à accorder.
///
/// C'est la seule arête de la plaque qui coure parallèlement à **Z** dans la vue
/// Briques (les longs bords sont selon Y quelle que soit la pose), donc la plus
/// facile à désigner et la plus visible quand elle est de travers.
///
/// Réglée à l'écran : 0,16 jugé trop épais, 0,08 trop maigre, **0,12** retenu.
const TAB: f32 = 0.12;

// --- Géométrie commune -------------------------------------------------------

/// Sommets de la plaque : hexagone **pointe en haut** (sommet 0 sur +Y), demi-cotes
/// `demi_large` × `demi_haut`, épaules à la hauteur `epaule`, les deux pointes
/// éventuellement **rognées** d'un méplat de largeur `tab`.
///
/// L'orientation n'est pas libre. C'est la seule qui donne, une fois étirée,
/// **deux longs bords parallèles** de part et d'autre plus une pointe en haut et
/// en bas — la silhouette du schéma. Étirer un hexagone pointe **sur le côté**
/// donnerait au contraire un dessus plat et six arêtes toutes différentes.
///
/// Elle prend les **deux demi-cotes directement**, plutôt qu'un rayon assorti de
/// facteurs : largeur et hauteur se règlent séparément sur cette pièce, et les
/// faire transiter par un rayon commun obligerait à compenser l'une quand on
/// touche à l'autre. Le rayon garde son rôle partout ailleurs (moyeu, alésage,
/// section des nervures), là où il représente vraiment un gabarit.
///
/// Les deux autres cotes sont des **fractions**, donc sans unité à accorder :
///
/// - `epaule` : hauteur des quatre épaules, en fraction de la demi-hauteur.
///   [`EPAULE_REGULIER`] (0,5) redonne l'hexagone régulier étiré ; en dessous,
///   les épaules remontent vers le milieu et **les longs bords raccourcissent**
///   sans que la plaque perde un pouce de hauteur ;
/// - `tab` : largeur du méplat, en fraction de la largeur. C'est un **rognage**,
///   pas un appendice — la pointe est coupée à cette fraction du chemin vers ses
///   deux voisines, ce qui donne un bord droit exactement large de
///   `tab × largeur`. Un onglet posé *en plus* de la pointe aurait demandé sa
///   propre longueur, sa propre largeur, et se serait désaccordé à chaque
///   changement de proportion.
fn contour(demi_large: f32, demi_haut: f32, epaule: f32, tab: f32) -> Vec<Vec3> {
    let ep = demi_haut * epaule;
    // Ordre trigonométrique en partant de la pointe haute.
    let hexa = vec![
        vec3(0.0, demi_haut, 0.0),
        vec3(-demi_large, ep, 0.0),
        vec3(-demi_large, -ep, 0.0),
        vec3(0.0, -demi_haut, 0.0),
        vec3(demi_large, -ep, 0.0),
        vec3(demi_large, ep, 0.0),
    ];
    if tab <= 0.0 {
        return hexa;
    }
    let t = tab.min(0.9);
    let mut v = Vec::with_capacity(8);
    for k in 0..6 {
        // Sommets 0 (haut) et 3 (bas) : les deux pointes, chacune remplacée par
        // le couple de points où le méplat coupe ses arêtes.
        if k == 0 || k == 3 {
            let (avant, apres) = (hexa[(k + 5) % 6], hexa[(k + 1) % 6]);
            v.push(hexa[k].lerp(avant, t));
            v.push(hexa[k].lerp(apres, t));
        } else {
            v.push(hexa[k]);
        }
    }
    // Le rognage a inséré les deux points d'une pointe dans l'ordre
    // (côté précédent, côté suivant) : c'est déjà le sens trigonométrique.
    v
}

/// Projection d'un point de la nappe sur le **cercle** du moyeu. Le moyeu reste
/// rond quand la plaque s'étire : c'est une pièce mécanique, elle ne suit pas la
/// déformation du panneau.
fn au_moyeu(v: Vec3, r_moyeu: f32) -> Vec3 {
    v.normalize_or_zero() * r_moyeu
}

/// Nappe d'une face.
///
/// Les six secteurs sont découpés en `lames` lames radiales, chacune peinte avec
/// `palette[choix(secteur, lame)]`. Une lame par secteur et une palette d'un
/// seul ton donnent le miroir uni des grandes plaques ; `STRIES` lames en deux
/// tons donnent la face striée du petit bouclier ; une lame par secteur en six
/// tons donne son dos facetté. Le regroupement **par teinte** est ce qui garde
/// le nombre de lots bas : une lame par appel ferait trente-six `draw_mesh` pour
/// une seule face.
fn nappe<P: Peintre>(
    p: &mut P,
    c: &[Vec3],
    r_moyeu: f32,
    z: f32,
    devant: bool,
    lames: usize,
    palette: &[Color],
    choix: impl Fn(usize, usize) -> usize,
) {
    let n = lames.max(1);
    for (t, couleur) in palette.iter().enumerate() {
        let mut s: Vec<Vec3> = Vec::new();
        let mut ix: Vec<u16> = Vec::new();
        for secteur in 0..c.len() {
            let (a, b) = (c[secteur], c[(secteur + 1) % c.len()]);
            for j in 0..n {
                if choix(secteur, j) != t {
                    continue;
                }
                let p0 = a.lerp(b, j as f32 / n as f32);
                let p1 = a.lerp(b, (j + 1) as f32 / n as f32);
                let base = s.len() as u16;
                for v in [au_moyeu(p0, r_moyeu), p0, p1, au_moyeu(p1, r_moyeu)] {
                    s.push(vec3(v.x, v.y, z));
                }
                // Enroulement pris depuis la face regardée : une nappe cousue à
                // l'envers disparaît purement et simplement — macroquad ne
                // double-face pas les triangles.
                ix.extend_from_slice(&if devant {
                    [base, base + 1, base + 2, base, base + 2, base + 3]
                } else {
                    [base, base + 2, base + 1, base, base + 3, base + 2]
                });
            }
        }
        p.triangles(&s, &ix, *couleur);
    }
}

/// Une nervure radiale : cône du moyeu vers `bout`, à l'altitude `z_axe`.
///
/// La section est **circulaire**, donc la nervure déborde de la nappe d'autant
/// vers l'avant que vers l'arrière. C'est ce qu'on veut sur les grandes plaques
/// (`z_axe = 0`, miroir des deux côtés) ; le petit bouclier la décale derrière
/// pour garder sa face avant nette.
fn nervure<P: Peintre>(p: &mut P, bout: Vec3, r_moyeu: f32, section: f32, z_axe: f32) {
    let dir = bout.normalize_or_zero();
    let pied = dir * r_moyeu;
    let l = (bout - pied).length();
    if l < 1e-4 {
        return;
    }
    p.cone(pied + Vec3::Z * z_axe, dir, section, section * EFFILEMENT, l, SOMBRE);
}

/// Point de ceinture porté par la nervure qui vise `bout`.
fn point_ceinture(bout: Vec3, r_moyeu: f32, z_axe: f32) -> Vec3 {
    let pied = bout.normalize_or_zero() * r_moyeu;
    pied.lerp(bout, CEINTURE) + Vec3::Z * z_axe
}

/// Ossature d'une plaque : douze nervures (six vers les sommets, six vers les
/// milieux d'arête) et la ceinture qui les solidarise.
///
/// Les nervures secondaires ne sont pas décoratives : une fois la plaque étirée,
/// les deux longs bords sont bien plus longs que les quatre autres, et rien ne
/// les tiendrait en leur milieu.
fn ossature<P: Peintre>(p: &mut P, c: &[Vec3], rayon: f32, r_moyeu: f32, z_axe: f32) {
    for k in 0..6 {
        nervure(p, c[k], r_moyeu, rayon * NERVURE, z_axe);
        let milieu = (c[k] + c[(k + 1) % 6]) * 0.5;
        nervure(p, milieu, r_moyeu, rayon * NERVURE_FINE, z_axe);
    }

    // Ceinture : douze cordes reliant les nervures dans l'ordre angulaire
    // (sommet, milieu, sommet, …).
    let mut anneau: Vec<Vec3> = Vec::with_capacity(12);
    for k in 0..6 {
        anneau.push(point_ceinture(c[k], r_moyeu, z_axe));
        anneau.push(point_ceinture((c[k] + c[(k + 1) % 6]) * 0.5, r_moyeu, z_axe));
    }
    for k in 0..12 {
        p.cylindre(anneau[k], anneau[(k + 1) % 12], rayon * NERVURE_FINE * 0.7, SOMBRE);
    }
}

/// Jante : le cordon qui court sur les six arêtes et ferme la tranche.
fn jante<P: Peintre>(p: &mut P, c: &[Vec3], rayon: f32) {
    for k in 0..c.len() {
        p.cylindre(c[k], c[(k + 1) % c.len()], rayon * JANTE, SOMBRE);
    }
}

/// Moyeu **percé** : la plaque s'enfile sur le mât, elle ne s'y visse pas bout à
/// bout. Un cylindre plein boucherait l'axe et le mât disparaîtrait dedans.
fn moyeu<P: Peintre>(p: &mut P, rayon: f32) {
    let (r, demi) = (rayon * MOYEU, rayon * MOYEU_DEMI);
    pieces::tube(p, -Vec3::Z * demi, demi * 2.0, r, r * ALESAGE, BAGUE);
    // Frette de serrage, débordante et plus courte : elle chevauche la jaquette
    // du moyeu au lieu de partager une face avec elle.
    pieces::tube(p, -Vec3::Z * (demi * 0.45), demi * 0.9, r * 1.14, r * 0.94, SOMBRE);
}

/// Deux ports axiaux, aux **bouts du moyeu** : ces plaques s'enfilent en série
/// sur un mât, chacune doit donc offrir une sortie de l'autre côté.
pub(super) fn ports(profil: Profil, rayon: f32) -> Vec<Port> {
    let demi = rayon * MOYEU_DEMI;
    vec![
        Port::new(Repere::new(vec3(0.0, 0.0, demi), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
        Port::new(
            Repere::new(vec3(0.0, 0.0, -demi), Quat::from_rotation_y(PI)),
            GenrePort::ModuleAxial,
            profil,
        ),
    ]
}

// --- Petit bouclier ----------------------------------------------------------

/// Petit bouclier : hexagone **régulier**, deux faces franchement différentes.
///
/// - **face avant (+Z, côté poussière)** : striée, trente-six lames radiales.
///   C'est la face de travail — celle qui encaisse et qui rayonne ;
/// - **face arrière (−Z, côté vaisseau)** : la structure, nervures et ceinture
///   en saillie.
///
/// Les nervures sont décalées **derrière** la peau arrière plutôt que centrées
/// sur le plan de la plaque : centrées, elles traverseraient la face striée et
/// la brouilleraient. Le décalage vaut un demi-diamètre de nervure — assez pour
/// dégager la face avant, pas assez pour laisser un jour entre nervure et peau
/// (deux surfaces qui se frôlent sans se toucher, c'est le défaut qu'on a déjà
/// payé trois fois sur les collerettes).
pub(super) fn petit_dessiner<P: Peintre>(p: &mut P, rayon: f32) {
    let c = contour(rayon * FRAC_PI_3.sin(), rayon, EPAULE_REGULIER, 0.0);
    let r_moyeu = rayon * MOYEU;
    let peau = rayon * PEAU;

    // Face avant striée : deux valeurs en alternance. Six lames par secteur —
    // un nombre pair — pour que l'alternance se referme proprement d'un secteur
    // au suivant au lieu de doubler une strie sur la couture.
    let strie = [COULEUR, assombrir(COULEUR, 0.74)];
    nappe(p, &c, r_moyeu, peau, true, STRIES, &strie, |_, j| j % 2);

    // Face arrière : les six facettes nues, sur lesquelles se pose l'ossature.
    let dos: Vec<Color> = FACETTES.iter().map(|k| assombrir(COULEUR, k * 0.80)).collect();
    nappe(p, &c, r_moyeu, -peau, false, 1, &dos, |s, _| s);

    jante(p, &c, rayon);
    ossature(p, &c, rayon, r_moyeu, -(peau + rayon * NERVURE * 0.55));
    moyeu(p, rayon);
}

pub(super) fn petit_cout() -> f32 {
    10.0
}

// --- Grand bouclier ----------------------------------------------------------

/// Hauteur d'épaule qui redonne un hexagone **régulier** étiré : à mi-hauteur.
/// C'est la valeur que garde le petit bouclier, et celle dont la grande plaque
/// s'écarte pour raccourcir ses longs bords.
const EPAULE_REGULIER: f32 = 0.5;

/// Étroitesse d'une **grande** plaque : facteur appliqué à sa **seule largeur**.
///
/// Réduire la largeur en rabotant le rayon aurait entraîné tout ce que le rayon
/// commande par ailleurs — et notamment le **moyeu**, dont l'alésage (0,302) ne
/// laisse que 0,012 de jeu au mât (0,290). Le rétrécir de 20 % aurait fait
/// passer le mât au travers. La largeur se règle donc ici, et nulle part
/// ailleurs.
pub(super) const ETROITESSE: f32 = 0.80;

/// Hauteur des épaules d'une **grande** plaque, en fraction de la demi-hauteur.
///
/// À `EPAULE_REGULIER` (0,5) les longs bords font la moitié de la hauteur — la
/// proportion d'un hexagone régulier étiré. **0,25 les divise par deux** sans
/// toucher ni à la hauteur ni à la largeur : les épaules remontent vers le
/// milieu, et ce sont les quatre obliques qui s'allongent d'autant. C'est bien
/// le *bord* qu'on raccourcit, pas la plaque qu'on écrase.
const EPAULE: f32 = 0.25;

/// Contour d'une grande plaque : toujours **rogné**, donc toujours huit sommets.
///
/// Le serrage de `TAB` n'est pas de la prudence en l'air : les index de
/// [`motif_grand`] et d'[`EPAULES`] décrivent un contour à huit points, et un
/// méplat nul rendrait l'hexagone à six — les index sortiraient du tableau. Le
/// rognage est une propriété de la pièce, pas un réglage qu'on peut annuler.
fn contour_grand(rayon: f32, elancement: f32) -> Vec<Vec3> {
    contour(
        rayon * FRAC_PI_3.sin() * ETROITESSE,
        rayon * elancement.max(0.1),
        EPAULE,
        TAB.clamp(0.02, 0.6),
    )
}

/// Index des quatre **épaules** dans le contour rogné d'une grande plaque.
///
/// Le rognage remplace chaque pointe par deux sommets, si bien que le contour
/// compte huit points et non six, dans cet ordre trigonométrique :
///
/// | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
/// |---|---|---|---|---|---|---|---|
/// | méplat haut, droite | méplat haut, gauche | épaule haut-gauche | épaule bas-gauche | méplat bas, gauche | méplat bas, droite | épaule bas-droite | épaule haut-droite |
///
/// Les **longs bords** sont donc `2–3` (gauche) et `6–7` (droite), et les deux
/// méplats `0–1` (haut) et `4–5` (bas).
const EPAULES: [usize; 4] = [2, 3, 6, 7];

/// **Motif** d'une grande plaque, relevé sur le schéma.
///
/// Ce n'est pas la même armature que celle du petit bouclier, et la différence
/// est tout le sujet : le petit porte une **ceinture** qui fait le tour, la
/// grande ne porte que ses **huit rayons** — quatre vers les épaules, un vers le
/// milieu de chacun des deux méplats, un vers le milieu de chacun des deux longs
/// bords. Rien ne traverse la plaque.
///
/// Huit cellules, toutes ancrées au moyeu.
///
/// *(Deux barres transversales joignaient les épaules, relevées sur le schéma.
/// Retirées à l'écran : une fois les épaules remontées — voir [`EPAULE`] — elles
/// passaient à quelques dixièmes du moyeu et encombraient le centre au lieu de
/// le structurer.)*
///
/// Les index renvoient au contour **rogné** (huit sommets), dont l'ordre est
/// donné par [`EPAULES`].
fn motif_grand<P: Peintre>(p: &mut P, c: &[Vec3], rayon: f32, r_moyeu: f32) {
    // Les quatre épaules, seuls sommets que le rognage ne touche pas.
    for k in EPAULES {
        nervure(p, c[k], r_moyeu, rayon * NERVURE_MIROIR, 0.0);
    }
    // Milieu des deux méplats (là où était la pointe) et milieu des deux longs
    // bords : quatre rayons de plus, soit huit en tout.
    for (a, b) in [(0, 1), (4, 5), (2, 3), (6, 7)] {
        nervure(p, (c[a] + c[b]) * 0.5, r_moyeu, rayon * NERVURE_MIROIR, 0.0);
    }
}

/// Grand bouclier : la plaque **étirée**, miroir sur ses deux faces.
///
/// Deux choses la séparent du petit, et aucune n'est cosmétique :
///
/// - le **motif** est celui du schéma (huit rayons partant du moyeu) et non la
///   ceinture du petit bouclier ;
/// - la nappe est **d'un seul ton par face**. Elle a d'abord été facettée
///   secteur par secteur, comme la face arrière du petit — mauvaise idée ici :
///   six triangles de valeurs différentes rayonnant d'un moyeu, c'est le dessin
///   d'une **pierre taillée**, pas d'un miroir. Un miroir est uniforme, et ce
///   sont les nervures posées dessus qui lui donnent sa structure.
///
/// L'ossature est **centrée sur le plan de la plaque** : les deux faces sont des
/// faces de travail, aucune n'est un dos, donc les nervures débordent autant de
/// part et d'autre. C'est ce qui donne le profil en nœud papillon vu par la
/// tranche.
pub(super) fn grand_dessiner<P: Peintre>(p: &mut P, rayon: f32, elancement: f32) {
    let c = contour_grand(rayon, elancement);
    let r_moyeu = rayon * MOYEU;
    let peau = rayon * PEAU;

    // Un ton par face, l'arrière un cran plus sourd : à plat, deux faces
    // exactement de la même valeur donnent une plaque sans orientation lisible —
    // on ne sait plus laquelle on regarde. C'est la seule variation de teinte
    // que la pièce s'autorise.
    nappe(p, &c, r_moyeu, peau, true, 1, &[MIROIR], |_, _| 0);
    nappe(p, &c, r_moyeu, -peau, false, 1, &[assombrir(MIROIR, 0.82)], |_, _| 0);

    jante(p, &c, rayon);
    motif_grand(p, &c, rayon, r_moyeu);
    moyeu(p, rayon);
}

pub(super) fn grand_cout() -> f32 {
    18.0
}

// --- Mesures -----------------------------------------------------------------

fn assombrir(c: Color, k: f32) -> Color {
    Color { r: c.r * k, g: c.g * k, b: c.b * k, a: 1.0 }
}

/// Altitude d'une peau, en valeur absolue. Sortie pour que le test d'enroulement
/// sache **où** chercher les triangles de nappe : les reconnaître à leur cote
/// est le seul moyen de les isoler des cônes et des tubes une fois tout cuit
/// dans le même tas.
pub(super) fn demi_epaisseur(rayon: f32) -> f32 {
    rayon * PEAU
}

/// Plus grande distance à l'origine, prise **sur le contour** plutôt que par une
/// formule parallèle.
///
/// C'est un sommet qui majore, mais lequel change avec les proportions : la
/// pointe au-delà d'un élancement de 1, un sommet latéral en deçà — et le
/// rognage peut faire basculer le classement à lui seul. Coder ce basculement à
/// la main serait exactement le raccourci qui se trompe le jour où on retouche
/// une cote.
///
/// La mesure prend le **contour déjà construit**, et chaque plaque passe le sien.
/// Mesurer une petite plaque avec le méplat des grandes sous-estimerait son
/// rayon et l'englobant cesserait de contenir la pièce — un paramètre `tab` de
/// plus serait un paramètre de plus à oublier.
fn mesure(c: &[Vec3], rayon: f32) -> f32 {
    let bord = c.iter().fold(0.0f32, |m, v| m.max(v.length()));
    // La jante et le bout de nervure débordent du sommet ; le moyeu déborde en
    // Z. Aucun des deux ne dépasse le contour, mais on les compte quand même.
    (bord + rayon * JANTE).max(rayon * MOYEU_DEMI.hypot(MOYEU * 1.14))
}

pub(super) fn petit_rayon_local(rayon: f32) -> f32 {
    mesure(&contour(rayon * FRAC_PI_3.sin(), rayon, EPAULE_REGULIER, 0.0), rayon)
}

pub(super) fn grand_rayon_local(rayon: f32, elancement: f32) -> f32 {
    mesure(&contour_grand(rayon, elancement), rayon)
}

/// Demi-étendues **X** et **Y** du contour, jante comprise — le pendant en
/// rectangle de [`mesure`], pour le boudin de collision (`conception/assembleur.md`
/// §9). Lues sur le **contour déjà construit**, comme `mesure` : une plaque
/// étirée (grande, `elancement`) a besoin d'un rectangle, pas d'un carré, sans
/// quoi le boudin gaspille en largeur tout ce qu'il gagne en épaisseur.
fn mesure_xy(c: &[Vec3], rayon: f32) -> (f32, f32) {
    let hu = c.iter().fold(0.0f32, |m, v| m.max(v.x.abs()));
    let hv = c.iter().fold(0.0f32, |m, v| m.max(v.y.abs()));
    (hu + rayon * JANTE, hv + rayon * JANTE)
}

/// Jeu au-dessus de l'étendue analytique, même principe que `SAILLIE` dans
/// `thermique.rs`.
const JEU_BOUDIN: f32 = 0.15;

/// Demi-étendue **axiale** (Z) du boudin de la grande plaque : le moyeu
/// domine, puisque son ossature reste centrée sur le plan (`z_axe = 0` dans
/// [`motif_grand`]) — contrairement au petit bouclier.
fn grand_demi_epaisseur_boudin(rayon: f32) -> f32 {
    rayon * MOYEU_DEMI * (1.0 + JEU_BOUDIN)
}

/// Demi-étendue **axiale** (Z) du boudin du petit bouclier : son ossature est
/// décalée en arrière de la peau (`petit_dessiner`), et la racine de sa
/// nervure principale (rayon `NERVURE`) y dépasse légèrement plus loin que le
/// moyeu côté arrière. Reprend les mêmes constantes que le dessin — une seule
/// source, comme `mesure` pour le contour.
fn petit_demi_epaisseur_boudin(rayon: f32) -> f32 {
    let debord_nervure = rayon * PEAU + rayon * NERVURE * 0.55 + rayon * NERVURE;
    (rayon * MOYEU_DEMI).max(debord_nervure) * (1.0 + JEU_BOUDIN)
}

/// **Centrée sur l'origine** : contrairement à toutes les pièces montées par un
/// bout, une plaque est symétrique de part et d'autre de son plan et son moyeu
/// est traversant. Décaler le boudin le ferait déborder du mauvais côté.
pub(super) fn petit_englobant(rayon: f32) -> Enveloppe {
    let (hu, hv) = mesure_xy(&contour(rayon * FRAC_PI_3.sin(), rayon, EPAULE_REGULIER, 0.0), rayon);
    Enveloppe::plaque(Vec3::ZERO, Vec3::X, Vec3::Y, hu, hv, petit_demi_epaisseur_boudin(rayon))
}

pub(super) fn grand_englobant(rayon: f32, elancement: f32) -> Enveloppe {
    let (hu, hv) = mesure_xy(&contour_grand(rayon, elancement), rayon);
    Enveloppe::plaque(Vec3::ZERO, Vec3::X, Vec3::Y, hu, hv, grand_demi_epaisseur_boudin(rayon))
}
