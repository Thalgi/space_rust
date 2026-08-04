//! **Barre de ressources** : les quatorze compteurs, en haut à gauche.
//!
//! Deux lignes de sept, et les deux lignes ne sont pas qu'un moyen de faire
//! tenir quatorze compteurs — elles donnent une **grille**, et une grille
//! permet d'aligner chaque produit **sous** sa matière première. Lire une
//! colonne de haut en bas, c'est lire une chaîne de raffinage
//! (`docs/conception/interface.md` §3.4).
//!
//! # Ce qui est ici, et ce qui n'y est pas
//!
//! La **mise en page** (rectangle du bandeau, case d'une ressource, hauteur
//! occupée) et le **format des nombres** vivent ici et se testent. Le dessin
//! reste dans la vue, et l'ordre des ressources appartient à
//! [`crate::sprites::Ressource`].
//!
//! # ⚠️ Les quantités sont figées
//!
//! Il n'y a **aucune économie** dans ce dépôt : ni production, ni consommation,
//! ni coût, ni recherche. [`Tresorerie`] est un bouche-trou rempli à la main
//! (dette **D-INT-2** dans `STATE.md`). C'est délibéré : la question ouverte de
//! cette étape n'est pas « combien de minerai ai-je ? » mais « **est-ce
//! lisible ?** », et celle-là se juge à l'œil, sans modèle derrière.

use crate::sprites::Ressource;
use macroquad::prelude::*;

/// Marge autour du bandeau.
const MARGE: f32 = 8.0;
/// Largeur d'une case, bornée. Trop étroite, le nombre ne tient pas ; trop
/// large, les quatorze compteurs s'éparpillent sur tout l'écran.
const CASE_MIN: f32 = 58.0;
const CASE_MAX: f32 = 108.0;
/// Hauteur d'une ligne : de quoi loger une icône 16 px avec de l'air.
const LIGNE: f32 = 22.0;
/// Place réservée sous les deux lignes pour le **nom du système**.
const NOM_SYSTEME: f32 = 24.0;

/// Stock de chaque ressource.
///
/// ⚠️ **Bouche-trou** : rempli par [`Self::demo`], rien ne le fait bouger. Voir
/// l'en-tête du module et la dette D-INT-2.
pub struct Tresorerie {
    /// Une quantité par ressource, dans l'ordre de `Ressource::TOUTES`.
    quantites: [f64; Ressource::TOUTES.len()],
}

impl Tresorerie {
    /// Valeurs d'exemple, choisies pour **couvrir les ordres de grandeur** que
    /// la barre doit savoir afficher : de l'unité au million. Une trésorerie
    /// toute en petits nombres ne dirait pas si l'abrégé fonctionne.
    pub fn demo() -> Self {
        let mut q = [0.0; Ressource::TOUTES.len()];
        for (i, r) in Ressource::TOUTES.iter().enumerate() {
            q[i] = match r {
                Ressource::Energie => 512_000.0,
                Ressource::Recherche => 48_500.0,
                Ressource::Minerai => 1_250_000.0,
                Ressource::MineraiRare => 8_400.0,
                Ressource::NourritureBrute => 96_000.0,
                Ressource::Hydrogene => 5_000_000.0,
                Ressource::Antimatiere => 3.0,
                Ressource::Metal => 74_000.0,
                Ressource::MetalRare => 920.0,
                Ressource::NourritureTransformee => 12_400.0,
                Ressource::MateriauConstruction => 6_100.0,
                Ressource::Superstructure => 42.0,
                Ressource::Population => 2_400.0,
                Ressource::Robots => 180.0,
            };
        }
        Self { quantites: q }
    }

    pub fn quantite(&self, r: Ressource) -> f64 {
        let i = Ressource::TOUTES.iter().position(|x| *x == r).unwrap_or(0);
        self.quantites[i]
    }
}

/// Abrège un nombre pour l'affichage : `1 250 000` → `1.2M`.
///
/// **Une seule fonction pour les quatorze compteurs.** Les ordres de grandeur
/// vont de l'unité au million — sur le schéma même, 500 K de crédits contre 20
/// unités de métal. Deux compteurs qui formateraient chacun de leur côté
/// finiraient par écrire la même quantité de deux façons.
pub fn abreger(n: f64) -> String {
    const PALIERS: [(f64, &str); 3] = [(1e9, "G"), (1e6, "M"), (1e3, "K")];
    let signe = if n < 0.0 { "-" } else { "" };
    let a = n.abs();
    for (seuil, suffixe) in PALIERS {
        if a >= seuil {
            let v = a / seuil;
            // Une décimale sous 10 (1.2M se lit ; 1M perd trop), aucune au-delà
            // (à 3 chiffres la décimale n'apprend rien et fait déborder).
            return if v < 10.0 {
                format!("{signe}{v:.1}{suffixe}")
            } else {
                format!("{signe}{v:.0}{suffixe}")
            };
        }
    }
    format!("{signe}{:.0}", a)
}

/// Largeur d'une case, pour un écran donné.
fn largeur_case(ecran: Vec2) -> f32 {
    let dispo = ecran.x - MARGE * 2.0;
    (dispo / Ressource::COLONNES as f32).clamp(CASE_MIN, CASE_MAX)
}

/// Hauteur reservee **en bas** de l'ecran a l'outillage de developpement :
/// une rangee de bascules plus la ligne d'etat.
///
/// Le partage est celui-ci : **le haut appartient au jeu** (ressources, nom du
/// systeme), **le bas a l'atelier** (graine, physique, orbites, FPS). Avant ce
/// reglage les bascules etaient dessinees a y = 34, c'est-a-dire au milieu de
/// la barre de ressources.
pub const BAS_OUTILS: f32 = 56.0;

/// Bande basse laissee a l'outillage, a droite de la colonne d'astres.
///
/// Une seule source : la fiche d'astre s'en sert pour savoir ou s'arreter, et
/// le menu pour savoir ou poser ses bascules. Deux constantes recopiees se
/// seraient recouvertes.
pub fn strip_outils(ecran: Vec2) -> Rect {
    let gauche = ecran.x * crate::ecran::liste::PART_LARGEUR + MARGE;
    Rect::new(gauche, ecran.y - BAS_OUTILS, (ecran.x - gauche - MARGE).max(0.0), BAS_OUTILS)
}

/// Rectangle du bandeau entier — les deux lignes **et** le nom du système.
pub fn rectangle(ecran: Vec2) -> Rect {
    let l = largeur_case(ecran) * Ressource::COLONNES as f32;
    Rect::new(MARGE, MARGE, l, LIGNE * 2.0 + NOM_SYSTEME)
}

/// Hauteur totale occupée en haut de l'écran, marge comprise.
///
/// C'est ce que la colonne de gauche doit laisser passer : sans cette valeur
/// partagée, les deux se recouvriraient, et l'accord entre elles tiendrait à
/// deux constantes recopiées.
pub fn hauteur_occupee(ecran: Vec2) -> f32 {
    let r = rectangle(ecran);
    r.y + r.h + MARGE
}

/// Rectangle de la case d'une ressource.
pub fn case(ecran: Vec2, r: Ressource) -> Rect {
    let (col, ligne) = r.case();
    let l = largeur_case(ecran);
    let b = rectangle(ecran);
    Rect::new(b.x + l * col as f32, b.y + LIGNE * ligne as f32, l, LIGNE)
}

/// Ligne du **nom du système**, sous les deux lignes de compteurs.
pub fn ligne_nom(ecran: Vec2) -> Rect {
    let b = rectangle(ecran);
    Rect::new(b.x, b.y + LIGNE * 2.0, b.w, NOM_SYSTEME)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ECRANS: [(f32, f32); 5] =
        [(640.0, 480.0), (1000.0, 700.0), (1440.0, 900.0), (1920.0, 1080.0), (3840.0, 2160.0)];

    // L'abrégé, sur les **bornes** de chaque palier : c'est là qu'une
    // implémentation naïve bascule d'un cran trop tôt ou trop tard.
    #[test]
    fn labrege_change_de_palier_aux_bonnes_bornes() {
        let cas = [
            (0.0, "0"), (7.0, "7"), (42.0, "42"), (999.0, "999"),
            (1_000.0, "1.0K"), (1_500.0, "1.5K"), (9_999.0, "10.0K"),
            (12_000.0, "12K"), (999_000.0, "999K"),
            (1_000_000.0, "1.0M"), (1_250_000.0, "1.2M"), (5_000_000.0, "5.0M"),
            (12_000_000.0, "12M"),
            (1_000_000_000.0, "1.0G"),
        ];
        for (n, attendu) in cas {
            assert_eq!(abreger(n), attendu, "abreger({n})");
        }
    }

    // L'abrégé reste **court** : c'est sa raison d'être. Sans plafond, un
    // compteur à sept chiffres déborderait de sa case sans que rien ne le dise.
    #[test]
    fn labrege_reste_court_a_toutes_les_echelles() {
        let mut n = 1.0_f64;
        while n < 1e12 {
            let s = abreger(n);
            assert!(s.len() <= 5, "abreger({n}) = « {s} », {} caractères", s.len());
            n *= 1.7;
        }
    }

    // **Les quatorze cases tiennent dans le bandeau, sans se recouvrir.** C'est
    // toute la mise en page, et le recouvrement est le défaut qui ne se verrait
    // qu'à l'écran, sur deux nombres superposés.
    #[test]
    fn les_quatorze_cases_pavent_le_bandeau_sans_se_recouvrir() {
        for (l, h) in ECRANS {
            let e = vec2(l, h);
            let b = rectangle(e);
            let cases: Vec<Rect> = Ressource::TOUTES.iter().map(|r| case(e, *r)).collect();
            assert_eq!(cases.len(), 14);
            for (i, a) in cases.iter().enumerate() {
                assert!(
                    a.x >= b.x - 1e-3 && a.x + a.w <= b.x + b.w + 1e-3
                        && a.y >= b.y - 1e-3 && a.y + a.h <= b.y + b.h + 1e-3,
                    "{l}x{h} : la case {i} sort du bandeau"
                );
                for (j, c) in cases.iter().enumerate().skip(i + 1) {
                    let chevauche = a.x < c.x + c.w && c.x < a.x + a.w
                        && a.y < c.y + c.h && c.y < a.y + a.h;
                    assert!(!chevauche, "{l}x{h} : les cases {i} et {j} se recouvrent");
                }
            }
        }
    }

    // Le bandeau ne sort **jamais de l'écran**, même à 640 de large — la
    // largeur où quatorze compteurs sont le plus à l'étroit.
    #[test]
    fn le_bandeau_tient_dans_lecran() {
        for (l, h) in ECRANS {
            let e = vec2(l, h);
            let b = rectangle(e);
            assert!(b.x >= 0.0 && b.x + b.w <= l + 1e-3, "{l}x{h} : déborde ({})", b.x + b.w);
            assert!(b.y + b.h <= h + 1e-3, "{l}x{h} : déborde en bas");
            // Et il reste lisible : une case sous 58 px ne loge pas « 1.2M ».
            assert!(case(e, Ressource::Minerai).w >= CASE_MIN - 1e-3, "{l}x{h} : cases écrasées");
        }
    }

    // **La hauteur occupée couvre bien tout le bandeau**, nom du système
    // compris. C'est la valeur que la colonne de gauche consulte pour ne pas
    // passer dessous : si elle ment, les deux se recouvrent.
    #[test]
    fn la_hauteur_occupee_couvre_le_bandeau_entier() {
        for (l, h) in ECRANS {
            let e = vec2(l, h);
            let occupee = hauteur_occupee(e);
            let nom = ligne_nom(e);
            assert!(occupee >= nom.y + nom.h, "{l}x{h} : la ligne de nom dépasse la hauteur annoncée");
            for r in Ressource::TOUTES {
                let c = case(e, r);
                assert!(occupee >= c.y + c.h, "{l}x{h} : la case {r:?} dépasse");
            }
            // Et elle ne réserve pas non plus la moitié de l'écran.
            assert!(occupee < h * 0.35, "{l}x{h} : le bandeau mange {occupee} px");
        }
    }

    // **La bande d'outils ne touche ni la barre, ni la colonne.** C'est le
    // partage haut/bas : sans lui, les bascules de developpement retombent au
    // milieu des compteurs, ce qui etait le defaut signale.
    #[test]
    fn la_bande_doutils_reste_en_bas_et_a_droite_de_la_colonne() {
        for (l, h) in ECRANS {
            let e = vec2(l, h);
            let s = strip_outils(e);
            let b = rectangle(e);
            assert!(s.y >= b.y + b.h, "{l}x{h} : la bande d'outils remonte dans le bandeau");
            assert!(s.y + s.h <= h + 1e-3, "{l}x{h} : elle deborde en bas");
            // A droite de la colonne d'astres (un dixieme de la largeur).
            assert!(s.x >= l * crate::ecran::liste::PART_LARGEUR, "{l}x{h} : elle mord sur la colonne");
            assert!(s.w > 0.0, "{l}x{h} : bande vide");
        }
    }

    // La ligne du nom est **sous** les compteurs, jamais par-dessus.
    #[test]
    fn le_nom_du_systeme_est_sous_les_compteurs() {
        let e = vec2(1000.0, 700.0);
        let nom = ligne_nom(e);
        for r in Ressource::TOUTES {
            let c = case(e, r);
            assert!(nom.y >= c.y + c.h - 1e-3, "la case {r:?} déborde sur le nom");
        }
    }

    // La trésorerie de démonstration doit **couvrir les ordres de grandeur**,
    // sinon elle ne dit pas si l'abrégé fonctionne — c'est tout ce qu'on lui
    // demande tant qu'il n'y a pas d'économie.
    #[test]
    fn la_tresorerie_de_demo_couvre_les_ordres_de_grandeur() {
        let t = Tresorerie::demo();
        let formats: Vec<String> =
            Ressource::TOUTES.iter().map(|r| abreger(t.quantite(*r))).collect();
        assert!(formats.iter().any(|s| s.ends_with('M')), "aucun million : {formats:?}");
        assert!(formats.iter().any(|s| s.ends_with('K')), "aucun millier : {formats:?}");
        assert!(
            formats.iter().any(|s| !s.ends_with('M') && !s.ends_with('K') && !s.ends_with('G')),
            "aucune petite quantité : {formats:?}"
        );
        // Et aucune ressource n'est laissée à zéro par oubli.
        for r in Ressource::TOUTES {
            assert!(t.quantite(r) > 0.0, "{r:?} est à zéro");
        }
    }
}
