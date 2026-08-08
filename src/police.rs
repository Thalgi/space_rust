//! Police Minitel partagée par tout le projet.
//!
//! La police (fichier TTF/OTF) est déposée dans `assets/` (voir `assets/README.md`).
//! Elle est chargée une seule fois au démarrage via [`charger`], stockée en
//! `thread_local` (macroquad tourne sur un seul thread), puis utilisée partout
//! via [`texte`] et [`mesure`]. Si le fichier est absent, on retombe
//! silencieusement sur la police par défaut de macroquad.
//!
//! # ⚠️ La police ne couvre que 157 caractères
//!
//! `assets/minitel.ttf` fait 12 Ko. Il porte les accents **minuscules**
//! (à â ä ç è é ê ë î ï ô ö ù û ü) — c'est l'essentiel du look Minitel — mais
//! **pas** leurs majuscules, ni la ponctuation typographique (`…` `—` `«` `»`
//! `·` `×`). Un caractère sans glyphe ne se dessine pas : l'écran montre un
//! blanc ou un rectangle, ce que l'œil lit comme un texte corrompu.
//!
//! Deux cas se produisaient pour de bon :
//!
//! - `to_uppercase()` sur un libellé français — « Système généré » devient
//!   « SYSTÈME GÉNÉRÉ », et `È` comme `É` disparaissent (titre de la vue
//!   système, `skymap.rs`) ;
//! - `liste::tronquer`, qui ajoute une **ellipse** `…` à tout libellé trop
//!   long — donc à toutes les entrées coupées du sélecteur d'astres.
//!
//! [`assainir`] remplace ces caractères par un équivalent que la police a. Elle
//! est appliquée dans [`texte`] **et** dans [`mesure`] : mesurer une chaîne
//! qu'on ne dessine pas ferait déborder les libellés qu'on vient de tronquer.

use macroquad::prelude::*;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    static POLICE: RefCell<Option<Font>> = const { RefCell::new(None) };
    /// Tailles de glyphe déjà gravées dans l'atlas. Voir [`graver`].
    static TAILLES_GRAVEES: RefCell<HashSet<u16>> = RefCell::new(HashSet::new());
    /// Vrai une fois [`precharger`] passé : au-delà, toute gravure a lieu **en
    /// cours de frame**, ce qui est précisément le risque — on le signale.
    static PRECHARGE_FINI: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// **Plus petite taille de texte que le jeu a le droit de demander.**
///
/// Les vues dont la liste s'adapte au nombre d'items calculent leur taille
/// (`station.rs`, `skymap.rs`) et s'y bornent. Déclarée **ici** et non dans
/// chaque vue, parce que c'est [`TAILLES_PRECHARGEES`] qui doit la couvrir : un
/// `clamp` posé à l'œil dans un écran a suffi à faire graver une taille en
/// pleine frame, donc à repeindre tout le texte en noir.
// Mortes pour le binaire `mhd`, qui inclut ce module par `#[path]` sans
// dessiner de liste. Le jeu, lui, s'en sert dans `station`, `skymap`,
// `accueil` et `parametres`.
#[allow(dead_code)]
pub const TAILLE_MIN: f32 = 9.0;

/// Plus grande taille demandée (le titre de l'accueil).
// Mortes pour le binaire `mhd`, qui inclut ce module par `#[path]` sans
// dessiner de liste. Le jeu, lui, s'en sert dans `station`, `skymap`,
// `accueil` et `parametres`.
#[allow(dead_code)]
pub const TAILLE_MAX: f32 = 36.0;

/// Tailles de glyphe gravées **au chargement**, avant la première frame.
///
/// ⚠️ C'est le cœur de la correction des carrés noirs. L'atlas de macroquad se
/// **repaquette entièrement** quand il déborde (`text/atlas.rs`,
/// `Atlas::cache_sprite`) : tous les glyphes changent de place, et les quads
/// déjà empilés dans la frame gardent leurs anciennes coordonnées de texture —
/// d'où des tuiles noires sur *tout* le texte de la frame, pas seulement sur le
/// caractère fautif.
///
/// Graver ici, **hors frame**, met tout débordement là où il ne peut rien
/// casser. La plage couvre largement les tailles observées (8 à 22) : le jeu
/// demande `taille × 0,6`, donc de 14 px à 36 px de police.
/// Bornes déduites de [`TAILLE_MIN`] / [`TAILLE_MAX`] passées par `preparer`
/// (× `ECHELLE`, arrondi) : 9 → 5 et 36 → 22. Une marge de part et d'autre
/// absorbe un arrondi voisin sans rouvrir le trou.
const TAILLES_PRECHARGEES: std::ops::RangeInclusive<u16> = 4..=24;

/// **Tout ce que [`assainir`] peut rendre**, et donc tout ce que l'atlas doit
/// contenir. ASCII imprimable, plus les accents minuscules que la police porte.
///
/// C'est la contrepartie de `assainir` : elle garantit que rien d'autre
/// n'atteint `draw_text_ex`, donc que [`graver`] peut préparer l'atlas **une
/// fois pour toutes** au lieu de le laisser grandir en cours de frame.
const ALPHABET: &str = concat!(
    " !\"#$%&'()*+,-./0123456789:;<=>?@",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`",
    "abcdefghijklmnopqrstuvwxyz{|}~",
    "àâäçèéêëîïôöùûüœ°",
);

/// Facteur d'échelle appliqué à *toutes* les tailles de texte. La police Minitel
/// a une hampe bien plus grande que la police macroquad par défaut à taille px
/// égale : on compense ici, en un seul endroit. Baisser pour un texte plus petit.
const ECHELLE: f32 = 0.6;

/// Chemins essayés pour trouver la police Minitel (le premier qui charge gagne).
const CANDIDATS: &[&str] = &[
    "assets/minitel.ttf",
    "assets/minitel.otf",
    "assets/Minitel.ttf",
    "assets/Minitel.otf",
];

/// Ce que la police n'a pas, et par quoi le remplacer.
///
/// **Relevé sur la `cmap` du fichier**, pas deviné : ce sont exactement les
/// caractères que `minitel.ttf` ne contient pas, parmi le latin-1 et la
/// ponctuation qu'un libellé français peut produire.
///
/// Les accents **minuscules** n'y sont pas : la police les a, et les remplacer
/// abîmerait le rendu là où il est juste.
const REPLIS: &[(char, &str)] = &[
    // Majuscules accentuées — le cas de `to_uppercase()`.
    ('À', "A"), ('Á', "A"), ('Â', "A"), ('Ã', "A"), ('Ä', "A"), ('Å', "A"),
    ('Æ', "AE"),
    ('Ç', "C"),
    ('È', "E"), ('É', "E"), ('Ê', "E"), ('Ë', "E"),
    ('Ì', "I"), ('Í', "I"),
    ('Ñ', "N"),
    ('Ò', "O"), ('Ó', "O"), ('Ô', "O"), ('Ø', "O"),
    ('Ù', "U"), ('Ú', "U"), ('Û', "U"), ('Ü', "U"),
    ('Ý', "Y"), ('Ÿ', "Y"),
    // Minuscules que la police n'a pas non plus (le français n'en use guère,
    // mais un nom propre peut en porter).
    ('á', "a"), ('ã', "a"), ('å', "a"), ('æ', "ae"),
    ('ì', "i"), ('í', "i"),
    ('ñ', "n"),
    ('ò', "o"), ('ó', "o"), ('õ', "o"), ('ø', "o"),
    ('ú', "u"),
    ('ý', "y"), ('ÿ', "y"),
    // Ponctuation typographique — le cas de `tronquer` et des titres.
    ('…', "..."),
    ('—', "-"), ('–', "-"),
    ('«', "<"), ('»', ">"),
    ('“', "\""), ('”', "\""), ('‘', "'"),
    ('·', "-"), ('•', "-"),
    ('×', "x"),
    ('≤', "<="), ('≥', ">="),
    ('µ', "u"),
];

/// Le remplacement d'un caractère, s'il en faut un.
///
/// Sort tout de suite sur l'ASCII : c'est la quasi-totalité du texte du jeu, et
/// [`mesure`] est appelée **par caractère** depuis `liste::tronquer`.
fn repli(c: char) -> Option<&'static str> {
    // Sortie immédiate sur ce que la police dessine déjà : la quasi-totalité du
    // texte du jeu, et `mesure` est appelée **par caractère** depuis
    // `liste::tronquer`.
    if ALPHABET.contains(c) {
        return None;
    }
    // Sinon la table ; et à défaut `?`, pour que la sortie de `assainir` tienne
    // **toujours** dans `ALPHABET`. Sans ce filet, un caractère oublié (€, ✓, un
    // emoji…) ferait grandir l'atlas en pleine frame — voir [`graver`].
    Some(
        REPLIS
            .iter()
            .find(|(k, _)| *k == c)
            .map(|(_, r)| *r)
            .unwrap_or("?"),
    )
}

/// Remplace ce que la police ne sait pas dessiner.
///
/// Rend un [`Cow`] emprunté quand il n'y a rien à faire — le cas courant, et
/// aucune allocation.
pub fn assainir(t: &str) -> Cow<'_, str> {
    if !t.chars().any(|c| repli(c).is_some()) {
        return Cow::Borrowed(t);
    }
    let mut s = String::with_capacity(t.len());
    for c in t.chars() {
        match repli(c) {
            Some(r) => s.push_str(r),
            None => s.push(c),
        }
    }
    Cow::Owned(s)
}

/// Charge la police Minitel depuis `assets/`. À appeler une fois au démarrage,
/// avant la boucle de rendu.
pub async fn charger() {
    for chemin in CANDIDATS {
        if let Ok(font) = load_ttf_font(chemin).await {
            POLICE.with(|p| *p.borrow_mut() = Some(font));
            precharger();
            return;
        }
    }
    warn!(
        "Police Minitel introuvable dans assets/ (essaye: {:?}) - repli sur la police par defaut.",
        CANDIDATS
    );
}

/// Ce que [`texte`] et [`mesure`] font **toutes deux** avant d'appeler
/// macroquad : assainir la chaîne, et convertir la taille demandée en taille de
/// glyphe.
///
/// ⚠️ **Un seul endroit, délibérément.** `liste::tronquer` mesure une chaîne
/// puis la fait dessiner ; si les deux ne la traitaient pas à l'identique, il
/// mesurerait une ellipse `…` d'un caractère et en dessinerait trois, donc
/// déborderait de la colonne qu'il vient d'ajuster. Le désaccord n'est pas
/// testable — `mesure` exige un contexte GL — il est donc rendu **difficile à
/// écrire** : les deux fonctions ne sont que des enveloppes de celle-ci.
fn preparer(t: &str, taille: f32) -> (Cow<'_, str>, u16) {
    (assainir(t), (taille * ECHELLE).round().max(1.0) as u16)
}

/// **Grave tout l'alphabet dans l'atlas, à la première demande d'une taille.**
///
/// ⚠️ C'est la correction des « carrés noirs ». macroquad range les glyphes dans
/// un atlas qui grandit à la demande, et quand il grandit il **détruit l'ancienne
/// texture** pour en créer une plus grande (`text/atlas.rs`, `Atlas::texture`) :
///
/// ```text
/// ctx.delete_texture(self.texture);
/// self.texture = ctx.new_texture_from_rgba8(...);
/// ```
///
/// Or macroquad ne dessine pas tout de suite : il **empile**. Tout le texte déjà
/// empilé dans la frame référence la texture qu'on vient de détruire — et sort
/// en rectangles noirs. Ça se déclenchait en ouvrant la fiche d'un astre, qui
/// est le seul endroit de la vue système à demander les tailles de glyphe 10
/// et 12 : premier emploi, l'atlas grandit, et tout le texte de la frame noircit.
///
/// Deux précautions, dans cet ordre :
/// 1. **vider le lot** avant de toucher à l'atlas, pour que plus rien n'attende ;
/// 2. graver **tout** [`ALPHABET`] d'un coup, pour que cette taille ne fasse
///    plus jamais grandir l'atlas.
///
/// Appelée depuis `texte` et `mesure`, donc toute taille nouvelle est prise en
/// charge d'elle-même — y compris celle qu'un futur appelant inventera.
fn graver(taille: u16) {
    if TAILLES_GRAVEES.with(|s| s.borrow().contains(&taille)) {
        return;
    }
    POLICE.with(|p| {
        // ⚠️ On ne marque la taille comme gravée **que si elle l'a été**. La
        // version précédente insérait avant de vérifier la police : police
        // absente, la taille était réputée gravée pour toujours et plus rien ne
        // la rattrapait — exactement le cas où le repli sur la police par défaut
        // de macroquad rend des carrés noirs.
        let emprunt = p.borrow();
        let Some(f) = emprunt.as_ref() else { return };
        if PRECHARGE_FINI.with(|f| f.get()) {
            warn!(
                "POLICE : taille {taille} gravee en pleine frame - a ajouter a                  TAILLES_PRECHARGEES, sinon risque de tuiles noires"
            );
        }
        // Vider le lot **avant** de toucher a l'atlas : ce qui attend encore
        // reference les positions de glyphes d'avant le repaquetage.
        unsafe {
            get_internal_gl().flush();
        }
        let alphabet: Vec<char> = ALPHABET.chars().collect();
        f.populate_font_cache(&alphabet, taille);
        TAILLES_GRAVEES.with(|s| s.borrow_mut().insert(taille));
    });
}

/// Grave d'avance toutes les tailles de [`TAILLES_PRECHARGEES`].
///
/// Appelée une fois, juste apres le chargement de la police et **avant la
/// premiere frame** : l'atlas atteint alors sa taille definitive hors frame, la
/// ou un repaquetage ne peut rien casser.
fn precharger() {
    for t in TAILLES_PRECHARGEES {
        graver(t);
    }
    PRECHARGE_FINI.with(|f| f.set(true));
}

/// [`preparer`], plus la mise en cache de l'atlas. Le point de passage commun
/// **réel** de `texte` et `mesure` : `preparer` reste pur (donc testable sans
/// contexte graphique), `graver` porte l'effet.
fn preparer_et_graver(t: &str, taille: f32) -> (Cow<'_, str>, u16) {
    let (t, font_size) = preparer(t, taille);
    graver(font_size);
    (t, font_size)
}

/// Dessine du texte avec la police Minitel (repli automatique si absente).
pub fn texte(t: &str, x: f32, y: f32, taille: f32, couleur: Color) {
    let (t, font_size) = preparer_et_graver(t, taille);
    POLICE.with(|p| {
        let ref_p = p.borrow();
        let params = TextParams {
            font: ref_p.as_ref(),
            font_size,
            color: couleur,
            ..Default::default()
        };
        draw_text_ex(&t, x, y, params);
    });
}

/// Largeur (en pixels) d'un texte rendu avec la police Minitel.
// Morte pour le binaire `mhd`, qui inclut ce module par `#[path]` sans jamais
// mesurer. Le jeu, lui, l'appelle partout (`liste::tronquer`, `accueil`,
// galeries, `starmap`).
#[allow(dead_code)]
pub fn mesure(t: &str, taille: u16) -> f32 {
    let (t, taille) = preparer_et_graver(t, taille as f32);
    POLICE.with(|p| measure_text(&t, p.borrow().as_ref(), taille, 1.0).width)
}

#[cfg(test)]
mod tests {
    use super::*;

    // **Aucun caractère de la table ne survit.** C'est l'invariant : ce qui est
    // déclaré absent de la police ne doit jamais atteindre `draw_text_ex`.
    #[test]
    fn aucun_caractere_absent_ne_passe() {
        for (c, remplacement) in REPLIS {
            let entree = format!("a{c}b");
            let sortie = assainir(&entree);
            assert_eq!(
                sortie,
                format!("a{remplacement}b"),
                "U+{:04X} n'est pas remplace",
                *c as u32
            );
            assert!(
                !sortie.contains(*c),
                "U+{:04X} subsiste dans la sortie",
                *c as u32
            );
        }
    }

    // **Les accents minuscules restent intacts.** La police les a, et c'est
    // l'essentiel du look : les aplatir en ASCII rendrait « SYSTÈME » correct
    // mais « généré » laid, pour rien.
    #[test]
    fn les_accents_que_la_police_a_sont_preserves() {
        let gardes = "àâäçèéêëîïôöùûü";
        assert_eq!(
            assainir(gardes),
            gardes,
            "un accent que la police dessine a ete aplati"
        );
    }

    // Le cas qui a motivé tout ceci : un titre français mis en majuscules.
    #[test]
    fn un_titre_francais_en_majuscules_reste_lisible() {
        let titre = "Système généré à côté".to_uppercase();
        assert_eq!(assainir(&titre), "SYSTEME GENERE A COTE");
    }

    // L'autre cas : l'ellipse que `liste::tronquer` ajoute à tout libellé coupé.
    #[test]
    fn lellipse_de_troncature_devient_trois_points() {
        assert_eq!(assainir("SATURNE…"), "SATURNE...");
    }

    // **Le texte ordinaire n'est pas recopié.** `mesure` est appelée par
    // caractère depuis `tronquer` : allouer à chaque appel se paierait.
    #[test]
    fn un_texte_sans_caractere_absent_est_emprunte() {
        assert!(
            matches!(assainir("PARAMETRES 1920x1080"), Cow::Borrowed(_)),
            "une chaine sans caractere a remplacer a ete recopiee"
        );
        // Et un accent minuscule ne déclenche pas la copie non plus.
        assert!(matches!(assainir("généré"), Cow::Borrowed(_)));
    }

    // **`preparer` est le point de passage commun** de `texte` et `mesure`. Il
    // est testable, elles ne le sont pas (contexte GL). Ce test tient donc ce
    // qui compte : la chaîne préparée est assainie, et la taille suit `ECHELLE`.
    #[test]
    fn preparer_assainit_et_met_a_lechelle() {
        let (t, taille) = preparer("SATURNE…", 20.0);
        assert_eq!(t, "SATURNE...", "la chaine preparee n'est pas assainie");
        assert_eq!(taille, (20.0 * ECHELLE).round() as u16, "l'echelle n'est pas appliquee");
        // Jamais zéro : une taille de glyphe nulle ne dessine rien.
        assert!(preparer("x", 0.0).1 >= 1, "taille de glyphe nulle");
    }

    // ⚠️ **L'invariant qui rend `graver` suffisant.**
    //
    // `graver` prépare l'atlas pour `ALPHABET` et rien d'autre. Si `assainir`
    // laissait passer un seul caractère hors de cet alphabet, il ferait grandir
    // l'atlas en pleine frame — donc détruire sa texture — et tout le texte
    // déjà empilé sortirait en carrés noirs. C'est exactement le défaut qu'on
    // corrige : ce test est ce qui empêche qu'il revienne.
    #[test]
    fn assainir_ne_sort_jamais_de_lalphabet() {
        let majuscules = "Système généré".to_uppercase();
        let eprouvettes = [
            "Système généré à côté",
            majuscules.as_str(),
            "SATURNE…",
            "ISV — RADIATEUR",
            "· MODULE « AXIAL »",
            // Des caractères qu'aucune table ne prévoit : ils doivent tout de
            // même être ramenés dans l'alphabet, pas laissés filer.
            "€ ✓ ★ 10⁶ ⊕ 😀 ½",
            "Ærø Þórr Ñuñez",
        ];
        for e in eprouvettes {
            for c in assainir(e).chars() {
                assert!(
                    ALPHABET.contains(c),
                    "« {e} » laisse passer U+{:04X} ({c}), hors alphabet grave dans l'atlas",
                    c as u32
                );
            }
        }
    }

    // Le repli de dernier recours doit lui-même être dessinable, sinon il
    // rouvrirait le trou qu'il est censé boucher.
    #[test]
    fn le_repli_de_dernier_recours_est_dans_lalphabet() {
        assert!(ALPHABET.contains('?'));
        assert_eq!(assainir("\u{1F600}"), "?", "un emoji doit devenir '?'");
    }

    // ⚠️ **Toutes les tailles du jeu doivent être préchargées.**
    //
    // Une taille absente de [`TAILLES_PRECHARGEES`] se grave en pleine frame, et
    // c'est là que l'atlas se repaquette : tout le texte déjà empilé sort en
    // tuiles noires. Le jeu le signale dans la console, mais mieux vaut le
    // savoir ici — ce test rougit dès qu'un écran demande une taille neuve.
    #[test]
    fn les_tailles_du_jeu_sont_toutes_prechargees() {
        // Relevées sur les appels à `police::texte` / `police::mesure`.
        const DEMANDEES: &[f32] =
            &[14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 22.0, 24.0, 30.0, 36.0];
        // ⚠️ Et surtout les **bornes calculees** : ce sont elles qui ont manque.
        // `station.rs` bornait sa liste a 9,0 -> glyphe 5, hors d'une plage qui
        // commencait a 6. Une seule vue, une seule taille, et tout le texte de
        // la frame passait en noir.
        for t in DEMANDEES.iter().copied().chain([TAILLE_MIN, TAILLE_MAX]) {
            let (_, taille) = preparer("x", t);
            assert!(
                TAILLES_PRECHARGEES.contains(&taille),
                "taille de texte {t} -> glyphe {taille}, hors de la plage prechargee {:?}",
                TAILLES_PRECHARGEES
            );
        }
        for t in DEMANDEES {
            let (_, taille) = preparer("x", *t);
            assert!(
                TAILLES_PRECHARGEES.contains(&taille),
                "taille de texte {t} -> glyphe {taille}, hors de la plage prechargee {:?}",
                TAILLES_PRECHARGEES
            );
        }
    }

    // Et la plage n'est pas absurdement large : la graver coûte de la mémoire
    // d'atlas au démarrage. Ce test tient l'autre bord, celui que le précédent
    // ne voit pas — on pourrait sinon « corriger » un dépassement en mettant
    // 1..=200 et en avalant 40 Mo de texture.
    #[test]
    fn la_plage_prechargee_reste_raisonnable() {
        let n = TAILLES_PRECHARGEES.clone().count();
        assert!(n <= 32, "{n} tailles prechargees : l'atlas va exploser");
        assert!(*TAILLES_PRECHARGEES.start() >= 1, "une taille de glyphe nulle ne dessine rien");
    }

    // La table ne doit pas contenir deux fois le même caractère : la seconde
    // entrée serait morte, et on croirait avoir corrigé ce qu'on n'a pas.
    #[test]
    fn la_table_de_replis_na_pas_de_doublon() {
        let mut vus = std::collections::HashSet::new();
        for (c, _) in REPLIS {
            assert!(vus.insert(*c), "U+{:04X} est declare deux fois", *c as u32);
        }
    }

    // Un remplacement ne doit lui-même contenir que de l'ASCII, sans quoi il
    // pourrait réintroduire un caractère absent.
    #[test]
    fn les_remplacements_sont_en_ascii() {
        for (c, r) in REPLIS {
            assert!(
                r.is_ascii(),
                "le repli de U+{:04X} n'est pas de l'ASCII : {r}",
                *c as u32
            );
        }
    }
}
