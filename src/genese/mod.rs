mod apparences;
mod persistance;
pub(crate) mod presets;
mod taille;

pub use taille::ClasseTaille;

pub use persistance::{charger_presets, sauver_presets, PresetSauve};
pub use presets::{
    construire_preset_alpha_centauri, construire_preset_avatar, construire_preset_binaire,
    construire_preset_proxima, construire_preset_quadruple, construire_preset_solaire,
    construire_preset_tau_ceti, construire_preset_trinaire,
};

use crate::astre::Foyer;
use crate::disque::{Disque, DisqueConfig};
use crate::etoile::{self, ProfilEtoile};
use crate::planete::{Apparence, Planete, TypePlanete};
use crate::soleil::Soleil;
use crate::stabilite;
use crate::stellaire::{ArbreStellaire, Feuille, Noeud, Variante};
use crate::systeme::{self, Systeme};
pub use apparences::apparence_gazeuse; // aussi utilisée par la galerie (cellules aléatoires)
use apparences::{apparence_glacee, apparence_tellurique, gazeuse, tellurique};
use macroquad::prelude::*;
use macroquad::rand::{gen_range, srand};
use std::f32::consts::TAU;

pub const MASSE_ETOILE: f32 = 1000.0; // masse gravitationnelle (indépendante du rayon visuel)

/// Masse dérivée de la luminosité par la relation masse-luminosité des étoiles
/// de la séquence principale (L ∝ M^3.5 ⇒ M ∝ L^0.286), bornée entre 0,25× et
/// 5× `MASSE_ETOILE` pour éviter les extrêmes (naine très pâle, supergéante).
///
/// **Source unique** de ce calcul (`docs/methode.md` : une source par fait) —
/// il ne doit plus être recopié ailleurs. `masse_luminosite(1.0) == MASSE_ETOILE`
/// exactement, ce qui garde intactes les masses (et donc les périodes
/// orbitales) de tout système déjà calé sur une luminosité de 1.
///
/// Ne s'applique **pas** au trou noir ni à la quasi-étoile (lots suivants) :
/// leur masse ne vient pas de leur éclat et sera posée explicitement par leur
/// type plutôt que dérivée d'ici.
pub(crate) fn masse_luminosite(luminosite: f32) -> f32 {
    // ⚠️ `max` et non `clamp` sur l'entrée : une luminosité négative donne un
    // `powf` NaN, et **NaN traverse `clamp`** — ses comparaisons `<`/`>` sont
    // fausses des deux côtés, si bien que la borne ne le retient pas. Le cas
    // est inatteignable aujourd'hui (aucune branche de `ProfilEtoile::aleatoire`
    // ni aucun preset ne produit ≤ 0), mais les lots trou noir et quasi-étoile
    // ouvriront ce chemin : une masse NaN contaminerait silencieusement les
    // barycentres et les périodes de tout le système.
    let luminosite = luminosite.max(0.0);
    (MASSE_ETOILE * luminosite.powf(0.286)).clamp(0.25 * MASSE_ETOILE, 5.0 * MASSE_ETOILE)
}

/// **Masse d'un trou noir stellaire.**
///
/// ⚠️ La relation masse-luminosité **ne s'y applique pas** : un trou noir n'a
/// pas de photosphère, et le peu qu'il rayonne vient de son disque
/// d'accrétion. Passé à [`masse_luminosite`], sa faible luminosité le
/// coucherait au **plancher** — un quart de masse stellaire pour l'objet le
/// plus lourd du système, et des barycentres à l'envers.
///
/// Huit masses de référence : les trous noirs stellaires connus pèsent 5 à
/// 20 masses solaires. La valeur dépasse délibérément le **plafond** de
/// `masse_luminosite`, lequel borne les *étoiles* pour écarter naines pâles
/// et supergéantes extrêmes — une borne qui n'a rien à dire ici.
pub(crate) const MASSE_TROU_NOIR: f32 = 8.0 * MASSE_ETOILE;

/// **Masse d'une naine brune.** ~0,07 masse solaire : sous la limite de fusion
/// de l'hydrogène, c'est ce qui en fait une naine brune.
///
/// ⚠️ Comme le trou noir, elle a besoin de sa propre constante — mais par
/// l'autre bout. `masse_luminosite` **plancher** à un quart de masse solaire
/// pour écarter les naines pâles ; une naine brune est quatre fois plus légère
/// encore, et s'y verrait quadruplée. Le trou noir, lui, dépasse le plafond.
pub(crate) const MASSE_NAINE_BRUNE: f32 = 0.07 * MASSE_ETOILE;

/// **Le preset de naine brune, d'après sa température.**
///
/// Les quatre entrées du catalogue gazeux forment une **séquence spectrale** :
/// poussiéreuse (L), méthane (T), froide (Y). Les relier à la température plutôt
/// que de tirer au hasard fait que la teinte du corps dit sa classe — et la
/// lumière qu'il jette vient de la même température.
///
/// ⚠️ Les noms sont ceux du catalogue (`catalogue_gazeuses`) : une faute de
/// frappe ici fait paniquer `preset_gazeuse`, et un test la rattrape.
pub(crate) fn preset_naine_brune(temp: f32) -> &'static str {
    if temp >= 2000.0 {
        "Naine brune"
    } else if temp >= 1400.0 {
        "Naine brune L (poussiereuse)"
    } else if temp >= 900.0 {
        "Naine brune T (methane)"
    } else {
        "Naine brune Y (froide)"
    }
}

/// **L'échelle d'un système, en UA.** `u` dans `[0 ; 1]` tient la place du
/// tirage pour les types qui ont une plage.
///
/// - **étoiles normales** : `√L`, car la zone habitable se déplace en `√L`.
///   Étalonné pour que le Soleil (L = 1) reproduise les distances historiques.
///   Un astre lumineux étale son système, une naine le resserre.
/// - **rémanents** : aucune zone habitable, planètes rescapées sur orbites
///   serrées (cf. PSR B1257+12), indépendamment de leur maigre éclat.
/// - **naine brune** : plus serré encore. Sa luminosité réelle donnerait
///   `√L ≈ 0,003` et tout tiendrait dans son propre rayon.
///
/// ⚠️ **Extrait de `instancier` exprès.** La règle y vivait en clair, donc hors
/// d'atteinte de tout test, et la garde que le lot 6.1 venait d'écrire la
/// **recopiait** pour pouvoir l'éprouver — comparer une copie à une copie ne
/// prouve rien (`docs/methode.md`). Elle a maintenant un seul endroit.
pub(crate) fn echelle_systeme(profil: &ProfilEtoile, u: f32) -> f32 {
    let entre = |a: f32, b: f32| a + (b - a) * u.clamp(0.0, 1.0);
    if profil.est_naine_brune() {
        // ⚠️ 0,20 et non 0,15 : à 0,15 la première planète tombait **sous** le
        // plancher de `premiere_orbite`, donc à 1,5 rayon de la naine, et c'est
        // la garde `aucun_type_du_tirage_n_a_besoin_du_plancher` qui l'a dit.
        // Un système comprimé au point que le plancher le porte est un système
        // qu'on n'a pas choisi.
        entre(0.20, 0.32)
    } else if profil.est_remnant() {
        entre(0.25, 0.5)
    } else {
        profil.luminosite.max(0.001).sqrt()
    }
}

/// **La première planète se pose au-delà de la surface de son étoile.**
///
/// ⚠️ Rien ne l'imposait. Le garde-fou anti-collision de `instancier` ne compare
/// que les planètes **entre elles** ; l'étoile, elle, n'y figure pas. Ça n'avait
/// pas de conséquence tant que le plus gros astre du tirage était une géante
/// rouge de 7 unités, car l'échelle du système suit `√L` et une géante étale
/// déjà ses mondes — mais c'était une coïncidence, pas une garantie.
///
/// La supergéante rouge du lot 6 monte à 12 unités, et la quasi-étoile du lot 11
/// est annoncée **large comme une orbite** : celle-là poserait ses planètes dans
/// sa propre photosphère. La règle est donc posée maintenant, une fois, au seul
/// endroit qui décide de cette distance.
///
/// Une fois et demie le rayon : assez pour que le monde ne rase pas la surface,
/// assez peu pour qu'un système compact le reste.
pub(crate) fn premiere_orbite(brute_ua: f32, rayon_etoile: f32) -> f32 {
    brute_ua.max(rayon_etoile / etoile::UA * 1.5)
}

#[cfg(test)]
mod tests_orbite_interne {
    use super::*;

    /// **Aucun type du tirage n'a besoin du plancher — il doit rester inerte.**
    ///
    /// ⚠️ **Écrit d'abord à l'envers**, comme « la première planète tombe hors
    /// de son étoile ». C'était une **tautologie** : `premiere_orbite` le
    /// garantit par construction, donc retirer le plancher laissait le test
    /// vert. C'est le sabotage qui l'a dit, pas la relecture.
    ///
    /// Ce qu'il y a à garder est l'inverse, et ce n'est pas tautologique : le
    /// plancher est là pour un type **à venir** — la quasi-étoile du lot 11,
    /// large comme une orbite. Le jour où il se met à mordre sur un type
    /// ordinaire, c'est que ce type étale ou resserre son système autrement
    /// qu'on ne croit, et le système sortira comprimé sans que rien le dise.
    ///
    /// (Le plancher lui-même est gardé par le test suivant, sur son cas
    /// d'usage réel.)
    #[test]
    fn aucun_type_du_tirage_n_a_besoin_du_plancher() {
        // `instancier` : `a = premiere_orbite(gen_range(0.4, 0.8) * ech, r)`,
        // avec `ech = √L` hors rémanents. Le pire cas est donc 0,4 et le plus
        // grand rayon du type.
        // ⚠️ `q` aux deux bouts. Le rayon et la luminosité montent **ensemble**
        // dans `ordinaire` — un gros astre est un astre brillant, donc son
        // système s'étale d'autant — et c'est précisément ce qui garde le
        // plancher inerte. Les découpler un jour le rendrait porteur.
        for p in 0..2000 {
            let p = p as f32 / 2000.0;
            for q in [0.0_f32, 1.0] {
                for (nom, gab) in
                    [("ordinaire", etoile::ordinaire(p, q)), ("particuliere", etoile::particuliere(p))]
                {
                    let (_, _, rayon, lumi, type_nom, couronne) = gab;
                    // ⚠️ L'échelle vient d'`echelle_systeme`, **la vraie**, et
                    // non d'une copie de sa règle : cette garde la recopiait, et
                    // comparer une copie à une copie ne prouve rien
                    // (`docs/methode.md`). `u = 0` prend le bas de chaque plage,
                    // soit le système le plus resserré — le pire cas.
                    let profil = etoile::ProfilEtoile {
                        temperature: 5000.0,
                        couleur: Vec3::ONE,
                        rayon,
                        luminosite: lumi,
                        nom: type_nom,
                        couronne,
                    };
                    let brute = 0.4 * echelle_systeme(&profil, 0.0);
                    assert_eq!(
                        premiere_orbite(brute, rayon),
                        brute,
                        "{nom} « {type_nom} » : le plancher a dû repousser sa première planète — \
                         rayon {rayon:.1} pour une orbite brute de {:.1} unités",
                        brute * etoile::UA
                    );
                }
            }
        }
    }

    /// **Les quatre naines brunes sont atteignables, et nommées juste.**
    ///
    /// ⚠️ **Le nom est un lien vers le catalogue gazeux**, et `preset_gazeuse`
    /// **panique** sur un nom inconnu : une faute de frappe ici ne se verrait
    /// pas à la compilation, seulement au lancement du jeu, quand une graine
    /// tombe sur une naine brune. Le test le dit tout de suite.
    ///
    /// Et les quatre doivent sortir : une classe qu'aucune température n'atteint
    /// est une classe qui n'existe pas — le piège qui tenait le trou noir hors
    /// du jeu.
    #[test]
    fn les_quatre_naines_brunes_sont_atteignables_et_nommees_juste() {
        let catalogue: Vec<String> =
            catalogue_gazeuses().into_iter().map(|(n, _)| n).collect();
        // La plage de température que le tirage donne à une naine brune, prise à
        // sa source.
        let (tmin, tmax, ..) = etoile::ordinaire(0.25, 0.5);
        let mut vus: Vec<&str> = Vec::new();
        for k in 0..2000 {
            let t = tmin + (tmax - tmin) * (k as f32 / 2000.0);
            let nom = preset_naine_brune(t);
            assert!(
                catalogue.iter().any(|n| n == nom),
                "« {nom} » n'est pas au catalogue gazeux : `preset_gazeuse` paniquera"
            );
            if !vus.contains(&nom) {
                vus.push(nom);
            }
        }
        assert_eq!(vus.len(), 4, "seulement {} classes atteintes : {vus:?}", vus.len());
        // La séquence va du plus chaud au plus froid : la classe dit la
        // température, sinon elle ne dit rien.
        assert_eq!(vus[0], "Naine brune Y (froide)", "la plus froide n'ouvre pas la séquence");
        assert_eq!(vus[3], "Naine brune", "la plus chaude ne la ferme pas");
    }

    /// **Une naine brune pèse moins que le plancher des étoiles.**
    ///
    /// C'est ce qui en fait une naine brune : sous la limite de fusion de
    /// l'hydrogène. ⚠️ `masse_luminosite` plancher à un quart de masse solaire
    /// pour écarter les naines pâles — appliquée ici, elle la **quadruplerait**,
    /// et les périodes orbitales de tout son système seraient fausses. Symétrique
    /// exact du trou noir, qui dépasse le plafond.
    #[test]
    fn une_naine_brune_pese_moins_que_le_plancher_des_etoiles() {
        let plancher = masse_luminosite(0.0); // le plancher, par construction
        assert!(
            MASSE_NAINE_BRUNE < plancher,
            "{MASSE_NAINE_BRUNE} : au-dessus du plancher {plancher}, donc pas besoin d'elle"
        );
        // Et elle reste une masse crédible, pas un caillou : au-dessus du dixième
        // du plancher stellaire.
        assert!(MASSE_NAINE_BRUNE > plancher * 0.1, "trop légère pour tenir un système");
    }

    /// **L'échelle d'un système de naine brune est la plus serrée de toutes.**
    ///
    /// Sa luminosité réelle donnerait `√L ≈ 0,003` et tout tiendrait dans son
    /// propre rayon ; la valeur du jeu (~0,2) est donc posée à la main. Elle doit
    /// rester en deçà de celle d'un rémanent, elle-même en deçà d'une étoile
    /// ordinaire — sinon l'ordre des compacités ne veut plus rien dire.
    #[test]
    fn l_echelle_d_une_naine_brune_est_la_plus_serree() {
        let profil = |nom: &'static str, lumi: f32| etoile::ProfilEtoile {
            temperature: 3000.0,
            couleur: Vec3::ONE,
            rayon: 1.0,
            luminosite: lumi,
            nom,
            couronne: 0.0,
        };
        let brune = profil("Naine brune", 0.2);
        let remnant = profil("Pulsar", 0.25);
        let soleil = profil("G (jaune)", 1.0);
        for u in [0.0_f32, 0.5, 1.0] {
            let (b, r, s) = (
                echelle_systeme(&brune, u),
                echelle_systeme(&remnant, u),
                echelle_systeme(&soleil, u),
            );
            assert!(b < r, "u={u} : naine brune {b} pas plus serrée qu'un rémanent {r}");
            assert!(r < s, "u={u} : rémanent {r} pas plus serré qu'une G {s}");
        }
        // ⚠️ Une étoile ordinaire ignore `u` : son échelle vient de sa
        // luminosité, pas d'un tirage. Sinon deux systèmes de même étoile
        // n'auraient pas la même étendue.
        assert_eq!(echelle_systeme(&soleil, 0.0), echelle_systeme(&soleil, 1.0));
    }

    /// **Le plancher ne se déclenche que quand il sert.** Une étoile ordinaire
    /// ne doit pas voir son système repoussé : ce serait resserrer tous les
    /// autres pour un cas limite.
    #[test]
    fn le_plancher_orbital_ne_touche_pas_aux_etoiles_ordinaires() {
        // Le Soleil : L = 1, rayon 2,0 dans les unités du tirage.
        assert_eq!(premiere_orbite(0.4, 2.0), 0.4, "le plancher a repoussé une G");
        // Une naine M : petit rayon, système resserré.
        assert_eq!(premiere_orbite(0.4 * 0.4_f32.sqrt(), 1.3), 0.4 * 0.4_f32.sqrt());
        // Et il mord bien là où il faut : une quasi-étoile large comme une
        // orbite (lot 11), posée à 0,1 UA, se voit repoussée.
        let quasi = premiere_orbite(0.1, 1.2 * etoile::UA);
        assert!(quasi > 1.2, "une quasi-étoile poserait un monde dans sa photosphère : {quasi}");
    }
}

/// Construit un système aléatoire à partir d'une graine. Renvoie aussi un texte d'info.
/// La multiplicité (1 à 4 étoiles) est dérivée du seed lui-même, ce qui laisse la
/// séquence RNG du cas mono-étoile intacte (systèmes sauvegardés reproductibles).
pub fn construire_systeme(seed: u64) -> (Systeme, String) {
    srand(seed);
    match seed % 100 {
        0..=54 => construire_simple(),  // ~55 % : étoile unique
        55..=82 => construire_multiple(2), // binaire
        83..=93 => construire_multiple(3), // trinaire hiérarchique
        _ => construire_multiple(4),       // quadruple
    }
}

/// Système à une seule étoile (Titius-Bode + ceintures). `srand` déjà appelé.
fn construire_simple() -> (Systeme, String) {
    let profil = ProfilEtoile::aleatoire();
    // Masse de l'unique étoile : dérivée de sa luminosité, pas la constante
    // brute -- une naine froide ne pèse pas comme une géante bleue.
    // ⚠️ Sauf un trou noir : ce qu'il rayonne vient de son disque, pas d'une
    // photosphère, et la relation masse-luminosité le coucherait au plancher.
    // Même règle que `feuille_alea` -- c'est le MÊME fait.
    let masse_etoile = if profil.est_trou_noir() {
        MASSE_TROU_NOIR
    } else if profil.est_naine_brune() {
        // Sous la limite de fusion : elle passe **sous** le plancher de
        // `masse_luminosite`, là où le trou noir dépasse son plafond.
        MASSE_NAINE_BRUNE
    } else {
        masse_luminosite(profil.luminosite)
    };

    let mut sys = Systeme::new();
    // ⚠️ **Une naine brune n'est pas une photosphère.** Elle ne fusionne pas
    // l'hydrogène, et le dépôt la rend depuis toujours en géante gazeuse —
    // bandes, vortex, émission thermique côté nuit — ce qui vaut bien mieux que
    // ce qu'une étoile en ferait. Le centre du système est donc ici une
    // `Planete`, et `au_centre_du_systeme` lui rend ce que le rôle implique :
    // le plancher en pixels, et la lumière qu'elle jette sur ses mondes.
    if profil.est_naine_brune() {
        let app = preset_gazeuse(preset_naine_brune(profil.temperature));
        sys.ajouter(Box::new(
            Planete::new(Vec3::ZERO, Vec3::ZERO, profil.rayon, masse_etoile, app, Vec::new())
                .au_centre_du_systeme(profil.luminosite),
        ));
    } else {
        let soleil =
            Soleil::new(vec3(0.0, 0.0, 0.0), profil.rayon, profil.couleur, profil.luminosite)
                // Naine active ou géante calme (§1.3) : dérivé du même gabarit
                // que la masse ci-dessus, pas d'un second tirage pour le même
                // fait.
                .avec_activite(etoile::activite_type(profil.rayon, profil.luminosite));
        // ⚠️ **QUATRIÈME copie de cette chaîne**, trouvée par le garde-fou de
        // `stellaire.rs` — et la plus coûteuse : `construire_simple` est le
        // chemin des systèmes à **une seule étoile**, donc le plus fréquent du
        // jeu. Un trou noir tiré ici s'affichait en étoile ordinaire, et n'était
        // correct que dans les systèmes multiples.
        let soleil = Variante::de_mode(profil.couronne).appliquer(soleil);
        // Rémanents (pulsar/magnétar/étoile à neutrons) : pas de zone habitable.
        let soleil = if profil.est_remnant() {
            soleil.sans_zone_habitable()
        } else {
            soleil
        };
        sys.ajouter(Box::new(soleil));
    }

    // Échelle du système (UA), ancrée sur le TYPE d'étoile. La règle vit dans
    // `echelle_systeme` — un seul endroit, et testable.
    let ech: f32 = echelle_systeme(&profil, gen_range(0.0_f32, 1.0));

    // Planètes : distances en UA suivant une loi de Titius-Bode, mises à l'échelle.
    let nb: i32 = gen_range(3, 7);
    let mut a: f32 = premiere_orbite(gen_range(0.4, 0.8) * ech, profil.rayon);
    // Dernière planète posée (a, rayon) -> évite que deux corps se chevauchent quand
    // l'espacement géométrique les rapproche trop (systèmes compacts / rémanents).
    let mut precedent: Option<(f32, f32)> = None;
    for _ in 0..nb {
        // Position normalisée (équivalent solaire) : le TYPE dépend de la distance
        // relative à la ligne des neiges, qui suit l'échelle `ech`. Sans ça, une
        // étoile brillante (a étalé) classerait ses mondes internes comme glacés.
        let a_norm = a / ech;
        let p: f32 = gen_range(0.0, 1.0);
        let type_p = if a_norm < 2.0 {
            if p < 0.8 { TypePlanete::Tellurique } else { TypePlanete::Gazeuse }
        } else if a_norm < 6.0 {
            if p < 0.5 {
                TypePlanete::Gazeuse
            } else if p < 0.85 {
                TypePlanete::Tellurique
            } else {
                TypePlanete::Glacee
            }
        } else if p < 0.55 {
            TypePlanete::Glacee
        } else if p < 0.8 {
            TypePlanete::Gazeuse
        } else {
            TypePlanete::Tellurique
        };

        let temp = etoile::temp_equilibre(profil.luminosite, a);
        let (rayon, masse, app) = match type_p {
            TypePlanete::Tellurique => apparence_tellurique(temp),
            TypePlanete::Gazeuse => apparence_gazeuse(),
            TypePlanete::Glacee => apparence_glacee(),
        };

        // Garde-fou anti-collision : au moins (r_prec + r + marge) entre deux centres.
        if let Some((a_prec, r_prec)) = precedent {
            let min_a = a_prec + (r_prec + rayon) / etoile::UA + 0.03;
            a = a.max(min_a);
        }

        let e: f32 = gen_range(0.0, 0.2);
        let incl: f32 = gen_range(0.0, 0.16);
        let idx = ajouter_planete(&mut sys, masse_etoile, a, e, incl, rayon, masse, app, None);
        precedent = Some((a, rayon));

        let n_lunes: i32 = match type_p {
            TypePlanete::Gazeuse => gen_range(1, 4),
            TypePlanete::Tellurique => if a_norm > 0.8 { gen_range(0, 2) } else { 0 },
            TypePlanete::Glacee => gen_range(0, 2),
        };
        for _ in 0..n_lunes {
            ajouter_lune(&mut sys, idx, rayon);
        }

        a *= gen_range(1.5_f32, 1.85);
    }

    // Ceinture principale + ceinture de Kuiper — mises à la même échelle `ech` que
    // les planètes (la ligne des neiges suit aussi la luminosité), pour rester
    // cohérentes avec le système au lieu d'un rayon fixe.
    let bi: f32 = gen_range(2.0, 3.0) * ech;
    let bo: f32 = bi + gen_range(0.6, 1.4) * ech;
    sys.ajouter(Box::new(Disque::new(DisqueConfig::asteroides(
        900, bi * etoile::UA, bo * etoile::UA, masse_etoile,
    ))));
    let ki: f32 = bo + gen_range(20.0, 30.0) * ech;
    let ko: f32 = ki + gen_range(12.0, 20.0) * ech;
    sys.ajouter(Box::new(Disque::new(DisqueConfig::kuiper(
        1400, ki * etoile::UA, ko * etoile::UA, masse_etoile,
    ))));

    let info = format!("Etoile {}  -  {} K", profil.nom(), profil.temperature as i32);
    (sys, info)
}

/// Plan de placement des planètes d'un système multiple (zone stable circumbinaire).
struct Plan {
    foyer: Foyer,
    masse_centrale: f32,
    lumi_ref: f32, // luminosité de référence (température/HZ des planètes)
    a_min: f32,    // demi-grand axe min (UA) — hors des orbites stellaires
    a_max: f32,    // demi-grand axe max (UA)
}

/// Tire une étoile-feuille aléatoire. Renvoie (feuille, masse, luminosité).
fn feuille_alea() -> (Feuille, f32, f32) {
    // ⚠️ **Pas de naine brune en compagne.** Une feuille de l'arbre stellaire
    // devient un `Soleil` (`stellaire::Feuille`), et une naine brune y serait
    // donc peinte en étoile — granulation, couronne — exactement l'étiquette
    // que le §3 bis interdit. Elle peut être le **centre** d'un système (lot
    // 6.2), pas encore une compagne : ça demanderait que l'arbre sache porter
    // un corps qui n'est pas une photosphère.
    //
    // Retiré ici plutôt qu'au tirage : `ordinaire` décrit ce qui **existe**, et
    // c'est à chaque bâtisseur de dire ce qu'il sait construire. Bornée à huit
    // essais — une boucle non bornée sur un `gen_range` est un risque qu'on ne
    // prend pas pour un cas qui sort une fois sur trente-cinq.
    let mut p = ProfilEtoile::aleatoire();
    for _ in 0..8 {
        if !p.est_naine_brune() {
            break;
        }
        p = ProfilEtoile::aleatoire();
    }
    let p = p;
    let masse = if p.est_trou_noir() { MASSE_TROU_NOIR } else { masse_luminosite(p.luminosite) };
    let var = Variante::de_mode(p.couronne);
    (
        Feuille::new(p.rayon, p.couleur, p.luminosite, masse)
            .variante(var)
            .avec_remnant(p.est_remnant()),
        masse,
        p.luminosite,
    )
}

/// Tire une séparation en UA suivant une loi **log-uniforme** (approx. de la
/// log-normale observée, Raghavan et al. 2010 : pic ~40 UA, très large).
fn sep_log(min_ua: f32, max_ua: f32) -> f32 {
    gen_range(min_ua.ln(), max_ua.ln()).exp()
}

/// Plan « type S » : planètes autour de l'étoile-feuille `k` (0.3 UA .. 75 % du rayon
/// critique circumstellaire).
fn plan_s(k: usize, masse_hote: f32, lumi_hote: f32, a_crit: f32) -> Plan {
    Plan {
        foyer: Foyer::Etoile(k),
        masse_centrale: masse_hote,
        lumi_ref: lumi_hote,
        a_min: 0.3,
        a_max: a_crit * 0.75,
    }
}

/// Plan « type P » : planètes circumbinaires autour du barycentre, au-delà du rayon
/// critique circumbinaire (marge +25 %).
fn plan_p(sep: f32, m1: f32, m2: f32, l1: f32, l2: f32, e: f32) -> Plan {
    let a_min = stabilite::a_crit_p(sep, stabilite::rapport_masse(m1, m2), e) * 1.25;
    Plan {
        foyer: Foyer::Barycentre,
        masse_centrale: m1 + m2,
        lumi_ref: l1 + l2,
        a_min,
        a_max: a_min * 2.4,
    }
}

/// Construit un arbre stellaire aléatoire (`nb` étoiles) + la LISTE des zones stables
/// à peupler : type S autour d'une étoile ET/OU type P circumbinaire, qui **coexistent**.
/// Séparations réalistes (log-uniforme ~Raghavan) + ratios hiérarchiques stables (Holman-Wiegert).
fn arbre_et_plan(nb: usize) -> (Noeud, String, Vec<Plan>) {
    let inc = || gen_range(-0.3_f32, 0.3);
    let ph = || gen_range(0.0_f32, TAU);
    let chance = |p: f32| gen_range(0.0_f32, 1.0) < p;

    match nb {
        2 => {
            let (mut fa, mut ma, mut la) = feuille_alea();
            let (mut fb, mut mb, mut lb) = feuille_alea();
            // Hôte (étoile primaire, index 0) = la plus massive.
            if mb > ma {
                std::mem::swap(&mut fa, &mut fb);
                std::mem::swap(&mut ma, &mut mb);
                std::mem::swap(&mut la, &mut lb);
            }
            // Séparation réaliste (log-uniforme, ~Raghavan 2010) : de serrée à large.
            let sep = sep_log(1.0, 60.0);
            let e = gen_range(0.0, 0.45);
            let racine = Noeud::paire(Noeud::etoile(fa), Noeud::etoile(fb), sep, e, inc(), ph());

            // Zones stables (peuvent COEXISTER) : S autour de A, S autour de B, P circumbinaire.
            let mut plans = Vec::new();
            let a_s_a = stabilite::a_crit_s(sep, mb / (ma + mb), e); // zone autour de A (primaire)
            if a_s_a >= 0.8 {
                // Binaire assez large -> système planétaire type S (cas « Avatar »).
                plans.push(plan_s(0, ma, la, a_s_a));
            }
            let a_s_b = stabilite::a_crit_s(sep, ma / (ma + mb), e); // zone autour de B (secondaire)
            if a_s_b >= 0.8 && chance(0.5) {
                plans.push(plan_s(1, mb, lb, a_s_b));
            }
            // P-type circumbinaire : garanti si aucune zone S (binaire serré) ; sinon
            // COEXISTE avec le S, mais seulement si le rayon critique reste raisonnable
            // (pas de planète à des centaines d'UA pour un binaire très large).
            let a_p = stabilite::a_crit_p(sep, stabilite::rapport_masse(ma, mb), e);
            if plans.is_empty() || (a_p < 30.0 && chance(0.6)) {
                plans.push(plan_p(sep, ma, mb, la, lb, e));
            }
            (racine, "Binaire (aleatoire)".to_string(), plans)
        }
        3 => {
            // ((A·B) · C) : paire SERRÉE + étoile C ISOLÉE à grande distance.
            // C (isolée) héberge un vrai système planétaire type S ; AB = double
            // étoile lointaine dans son ciel.
            let (fa, ma, _) = feuille_alea();
            let (fb, mb, _) = feuille_alea();
            let (fc, mc, lc) = feuille_alea();
            let sep_in = sep_log(0.3, 1.5); // paire interne serrée
            let e_in = gen_range(0.0, 0.3);
            let ab = Noeud::paire(Noeud::etoile(fa), Noeud::etoile(fb), sep_in, e_in, inc(), ph());
            let m_ab = ma + mb;
            let e_out = gen_range(0.0, 0.4);
            let sep_out_min = stabilite::a_crit_p(sep_in, stabilite::rapport_masse(ma, mb), e_in) * 1.6;
            let lo = sep_out_min.max(15.0); // C nettement à l'écart (>= 15 UA)
            let sep_out = sep_log(lo, lo.max(45.0));
            let racine = Noeud::paire(ab, Noeud::etoile(fc), sep_out, e_out, inc(), ph());
            // C = feuille d'index 2 (ordre de déploiement A, B, C).
            let a_s_c = stabilite::a_crit_s(sep_out, m_ab / (m_ab + mc), e_out);
            let plans = vec![plan_s(2, mc, lc, a_s_c.max(1.0))];
            (racine, "Trinaire (A-B)+C - planetes autour de C".to_string(), plans)
        }
        _ => {
            // (((A·B) · C) · D) : triple interne COMPACT + étoile D ISOLÉE à grande
            // distance. D héberge un système planétaire type S.
            let (fa, ma, _) = feuille_alea();
            let (fb, mb, _) = feuille_alea();
            let (fc, mc, _) = feuille_alea();
            let (fd, md, ld) = feuille_alea();
            // Paire interne serrée.
            let sep_ab = sep_log(0.3, 1.2);
            let e_ab = gen_range(0.0, 0.3);
            let ab = Noeud::paire(Noeud::etoile(fa), Noeud::etoile(fb), sep_ab, e_ab, inc(), ph());
            let m_ab = ma + mb;
            // C à distance modérée -> triple compact ((A·B)·C).
            let e_c = gen_range(0.0, 0.3);
            let sep_c_min = stabilite::a_crit_p(sep_ab, stabilite::rapport_masse(ma, mb), e_ab) * 1.6;
            let sep_c = sep_c_min.max(2.0) * gen_range(1.0, 2.0);
            let abc = Noeud::paire(ab, Noeud::etoile(fc), sep_c, e_c, inc(), ph());
            let m_abc = m_ab + mc;
            // D isolée, loin -> système type S autour de D.
            let e_out = gen_range(0.0, 0.4);
            let sep_out_min = stabilite::a_crit_p(sep_c, stabilite::rapport_masse(m_ab, mc), e_c) * 1.6;
            let lo = sep_out_min.max(20.0);
            let sep_out = sep_log(lo, lo.max(55.0));
            let racine = Noeud::paire(abc, Noeud::etoile(fd), sep_out, e_out, inc(), ph());
            // D = feuille d'index 3 (ordre de déploiement A, B, C, D).
            let a_s_d = stabilite::a_crit_s(sep_out, m_abc / (m_abc + md), e_out);
            let plans = vec![plan_s(3, md, ld, a_s_d.max(1.0))];
            (racine, "Quadruple ((A-B)-C)+D - planetes autour de D".to_string(), plans)
        }
    }
}

/// Système à plusieurs étoiles : arbre hiérarchique + planètes placées dans CHAQUE
/// zone stable (type S autour d'une étoile ET/OU type P circumbinaire). `srand` déjà appelé.
fn construire_multiple(nb: usize) -> (Systeme, String) {
    let mut sys = Systeme::new();
    let (racine, info, plans) = arbre_et_plan(nb);
    deployer_arbre(&mut sys, racine);

    let mut focus: Option<(usize, f32)> = None;
    for plan in &plans {
        placer_planetes(&mut sys, plan);
        // Vue par défaut : focaliser la 1re étoile hôte (zone S) sur sa zone planétaire
        // — un système type S est trop étalé pour voir ses planètes en cadrant tout.
        if focus.is_none() {
            if let Foyer::Etoile(k) = plan.foyer {
                focus = Some((k, (plan.a_max * etoile::UA * 3.0).max(150.0)));
            }
        }
    }
    if let Some((k, d)) = focus {
        sys.definir_vue(k, d);
    }

    (sys, info)
}

/// Peuple une zone stable (`plan`) d'une courte séquence de planètes (Titius-Bode),
/// de `a_min` à `a_max`, autour du foyer du plan.
fn placer_planetes(sys: &mut Systeme, plan: &Plan) {
    let n_pl: i32 = gen_range(1, 5);
    let mut a = plan.a_min * gen_range(1.0, 1.3);
    for _ in 0..n_pl {
        if a > plan.a_max {
            break;
        }
        let temp = etoile::temp_equilibre(plan.lumi_ref, a);
        let p: f32 = gen_range(0.0, 1.0);
        let type_p = if temp > 250.0 {
            if p < 0.55 { TypePlanete::Tellurique } else { TypePlanete::Gazeuse }
        } else if temp > 150.0 {
            if p < 0.5 { TypePlanete::Gazeuse } else { TypePlanete::Glacee }
        } else {
            TypePlanete::Glacee
        };
        let (rayon, masse, app) = match type_p {
            TypePlanete::Tellurique => apparence_tellurique(temp),
            TypePlanete::Gazeuse => apparence_gazeuse(),
            TypePlanete::Glacee => apparence_glacee(),
        };
        let e: f32 = gen_range(0.0, 0.12);
        let incl: f32 = gen_range(0.0, 0.08);
        let idx = ajouter_planete_autour(sys, plan.foyer, plan.masse_centrale, a, e, incl, rayon, masse, app, None);
        let n_lunes: i32 = match type_p {
            TypePlanete::Gazeuse => gen_range(1, 4),
            _ => gen_range(0, 2),
        };
        for _ in 0..n_lunes {
            ajouter_lune(sys, idx, rayon);
        }
        a *= gen_range(1.4, 1.8);
    }
}

/// Ajoute une planète orbitant un `foyer` (étoile hôte S-type, ou barycentre P-type),
/// autour d'une `masse_centrale` donnée. `a_au` = demi-grand axe relatif au foyer.
/// Le **nom** est un parametre, et il n'a pas de valeur par defaut : chaque
/// appelant doit dire `Some("Terre")` ou `None`. C'etait la dette D-INT-5 —
/// tant que le nom se posait par un `sys.nommer` apres coup, une planete de
/// preset pouvait le perdre sans que rien ne rougisse (aucun test ne peut batir
/// un systeme, `conception/interface.md` 5.1 bis). Le rendre obligatoire deplace
/// la garantie dans le compilateur, comme pour `ajouter_lune_preset`.
///
/// `None` est le cas de la **generation procedurale** : ses systemes retombent
/// sur la numerotation orbitale, par decision de conception (2.2a).
// Elements orbitaux + apparence : une liste coherente, pas un fourre-tout. Un
// type `ElementsOrbitaux` serait plus propre et touche TOUS les presets ; note
// en dette plutot que fait a la va-vite au milieu d'une passe de nettoyage.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ajouter_planete_autour(
    sys: &mut Systeme,
    foyer: Foyer,
    masse_centrale: f32,
    a_au: f32,
    e: f32,
    incl: f32,
    rayon: f32,
    masse: f32,
    app: Apparence,
    nom: Option<&'static str>,
) -> usize {
    let a_monde = a_au * etoile::UA;
    let phi: f32 = gen_range(0.0, TAU);
    let a1 = vec3(phi.cos(), 0.0, phi.sin());
    let a2 = vec3(-phi.sin(), 0.0, phi.cos());
    let q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();

    // Orbite de Kepler analytique (relative au foyer). m0 = 0 -> départ au périastre.
    let mu = systeme::G * masse_centrale;
    let orb = crate::orbite::Orbite::new(a_monde, e, a1, q, mu, 0.0);
    let (pos, vel) = orb.etat(0.0);
    // Résolution de la trace : elle est dessinée en POINTS, et c'est le nombre
    // d'échantillons qui plafonne leur densité. À 96, un zoom serré ne montrait
    // plus qu'une poignée de points sur tout l'écran — impossible d'y lire un
    // cercle. Le tracé en saute ensuite selon la distance (`dessiner_points`),
    // donc une réserve haute ne coûte rien de loin ; elle sert de près.
    // 512 points = 6 Ko par planète, et rien à l'écran qu'on ne veuille voir.
    let orbite = orb.polyligne(crate::orbite::Orbite::N_TRACE);
    let idx = sys.ajouter(Box::new(
        Planete::new(pos, vel, rayon, masse, app, orbite)
            .avec_orbite(orb)
            .avec_foyer(foyer),
    ));
    if let Some(n) = nom {
        sys.nommer(idx, n);
    }
    idx
}

/// Planète autour de l'étoile unique à l'origine (cas mono-étoile).
///
/// `masse_etoile` est la masse de CETTE étoile (via `masse_luminosite`, pas la
/// constante brute) : une planète orbite la masse de son hôte, pas un chiffre
/// fixe qui ignorerait une naine rouge ou une géante bleue.
// Meme liste que `ajouter_planete_autour`, foyer en moins.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ajouter_planete(
    sys: &mut Systeme,
    masse_etoile: f32,
    a_au: f32,
    e: f32,
    incl: f32,
    rayon: f32,
    masse: f32,
    app: Apparence,
    nom: Option<&'static str>,
) -> usize {
    ajouter_planete_autour(sys, Foyer::Barycentre, masse_etoile, a_au, e, incl, rayon, masse, app, nom)
}

/// Cœur du placement d'une lune : la pose sur le prochain créneau orbital dans le
/// **domaine gravitationnel** de la planète (approximation de sa sphère de Hill,
/// bornée par sa distance à l'étoile) — les lunes restent donc proches de leur
/// planète et ne peuvent pas empiéter sur l'orbite de la voisine. Réparties du bord
/// de Roche (~2,4 R) jusqu'à ~14 % de la distance à l'étoile, sur des créneaux
/// croissants (nombre de lunes déjà là). ω suit la 3e loi de Kepler. `rayon_lune` et
/// `app` sont fournis par l'appelant. Renvoie `false` (rien posé) si la planète est
/// trop proche de son étoile pour une lune stable.
fn poser_lune(sys: &mut Systeme, parent: usize, rayon_planete: f32, rayon_lune: f32, app: Apparence) -> Option<usize> {
    let roche = rayon_planete * 2.4; // bord interne : pas de lune sous la limite de Roche
    let dist_etoile = sys.position(parent).length();
    let hill_max = 0.14 * dist_etoile;
    if hill_max <= roche * 1.1 {
        return None; // trop pres de l'etoile : pas d'espace pour une lune stable
    }
    // Créneau croissant réparti dans la bande [roche, hill_max] (indice = lunes déjà là).
    let i = sys.nb_lunes(parent) as f32;
    let frac = ((i + 0.6) / 5.0).min(0.92);
    let bande = hill_max - roche;
    let r_orbite = roche + bande * frac + gen_range(-0.04, 0.04) * bande;

    // Sens majoritairement prograde (lunes régulières) ; rétrograde occasionnel (Triton).
    let sens = if gen_range(0.0_f32, 1.0) < 0.85 { 1.0 } else { -1.0 };
    // Kepler : ω décroît en (roche / r)^1.5 -> lunes internes rapides, externes lentes.
    const OMEGA_REF: f32 = 1.2;
    let omega = sens * OMEGA_REF * (roche / r_orbite).powf(1.5);
    // Lunes régulières quasi coplanaires (plan équatorial du parent) : faible inclinaison.
    let incl: f32 = gen_range(-0.12, 0.12);
    let phase: f32 = gen_range(0.0, TAU);
    let lune = Planete::new(Vec3::ZERO, Vec3::ZERO, rayon_lune.max(0.05), 0.05, app, Vec::new())
        .en_lune(parent, r_orbite, omega, incl, phase);
    Some(sys.ajouter(Box::new(lune)))
}

/// Ajoute une lune **générique** (petit corps gris ou glacé aléatoire) : utilisée par
/// la génération procédurale. Taille réduite (une lune est bien plus petite que sa
/// planète), plancher de visibilité.
pub(crate) fn ajouter_lune(sys: &mut Systeme, parent: usize, rayon_planete: f32) {
    let rayon = rayon_planete * gen_range(0.05, 0.12);
    let app = if gen_range(0.0_f32, 1.0) < 0.5 {
        let g: f32 = gen_range(0.4, 0.7);
        app_simple(TypePlanete::Tellurique, vec3(g, g * 0.95, g * 0.9), vec3(g * 0.6, g * 0.58, g * 0.55), Vec3::ZERO, 0.0)
    } else {
        app_simple(TypePlanete::Glacee, vec3(0.72, 0.77, 0.85), vec3(0.55, 0.6, 0.7), Vec3::ZERO, 0.0)
    };
    poser_lune(sys, parent, rayon_planete, rayon, app);
}

/// Pose une lune à une distance **donnée**, sans placement procédural.
///
/// `poser_lune` répartit les lunes entre la limite de Roche et 14 % de la
/// distance à l'étoile : commode pour un système inventé, faux pour le nôtre. À
/// l'échelle vraie l'écart saute aux yeux — la Lune se retrouvait vers 0,8 unité
/// quand la vraie est à 0,123, six fois trop loin.
///
/// Pas de plancher sur le rayon non plus : `poser_lune` force `.max(0.05)`, ce
/// qui à l'échelle vraie fait **vingt-quatre fois la Terre**. Phobos doit avoir
/// le droit d'être invisible.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ajouter_lune_reelle(
    sys: &mut Systeme,
    parent: usize,
    rayon_lune: f32,
    r_orbite: f32,
    omega: f32,
    incl: f32,
    phase: f32,
    app: Apparence,
    nom: &'static str,
) {
    let lune = Planete::new(Vec3::ZERO, Vec3::ZERO, rayon_lune, 0.05, app, Vec::new())
        .en_lune(parent, r_orbite, omega, incl, phase);
    let idx = sys.ajouter(Box::new(lune));
    sys.nommer(idx, nom);
}


/// Déploie un arbre stellaire (systèmes multiples) dans le système : crée les
/// étoiles-feuilles et installe l'arbre évaluable. Voir `crate::stellaire`.
pub(crate) fn deployer_arbre(sys: &mut Systeme, racine: Noeud) {
    let mut arbre = ArbreStellaire::new();
    deployer_noeud(sys, &mut arbre, &racine, None, None);
    sys.definir_arbre(arbre);
}

/// Récursion : ajoute le nœud (feuille -> Soleil, paire -> deux orbites autour du
/// barycentre) et renvoie son index dans l'arbre déployé.
fn deployer_noeud(
    sys: &mut Systeme,
    arbre: &mut ArbreStellaire,
    noeud: &Noeud,
    parent: Option<usize>,
    orbite: Option<crate::orbite::Orbite>,
) -> usize {
    match noeud {
        Noeud::Etoile(f) => {
            let mut s = Soleil::new(Vec3::ZERO, f.rayon, f.couleur, f.luminosite)
                .avec_activite(etoile::activite_type(f.rayon, f.luminosite));
            s = f.variante.appliquer(s);
            if f.remnant {
                s = s.sans_zone_habitable(); // pulsar/magnétar/étoile à neutrons : pas de HZ
            }
            s.base.masse = f.masse; // masse personnalisée -> barycentres corrects
            let idx_astre = sys.ajouter(Box::new(s));
            arbre.ajouter(parent, orbite, Some(idx_astre))
        }
        Noeud::Paire { a, b, sep, e, incl, phase } => {
            // Nœud de paire d'abord (ordre topologique : parent avant enfants).
            let idx = arbre.ajouter(parent, orbite, None);

            let ma = a.masse();
            let mb = b.masse();
            let m = (ma + mb).max(1e-6);
            let sep_monde = *sep * etoile::UA;

            let phi: f32 = gen_range(0.0, TAU);
            let a1 = vec3(phi.cos(), 0.0, phi.sin());
            let a2 = vec3(-phi.sin(), 0.0, phi.cos());
            let q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();

            // Moyen mouvement partagé (même période pour les deux côtés).
            let n = (systeme::G * m / (sep_monde * sep_monde * sep_monde)).sqrt();
            // Chaque côté orbite le barycentre : demi-grand axe ∝ masse de l'autre.
            // A du côté opposé (base -a1,-q).
            let orb_a = crate::orbite::Orbite::avec_n(sep_monde * mb / m, *e, -a1, -q, n, *phase);
            let orb_b = crate::orbite::Orbite::avec_n(sep_monde * ma / m, *e, a1, q, n, *phase);

            deployer_noeud(sys, arbre, a, Some(idx), Some(orb_a));
            deployer_noeud(sys, arbre, b, Some(idx), Some(orb_b));
            idx
        }
    }
}

pub(crate) fn app_simple(type_p: TypePlanete, c1: Vec3, c2: Vec3, c3: Vec3, eau: f32) -> Apparence {
    Apparence::new(type_p, c1, c2, c3, eau)
}

/// Catalogue des telluriques actuellement générables, pour la galerie de validation
/// visuelle. Aujourd'hui : un échantillon par bande de température. Cette liste
/// grandira au fur et à mesure qu'on ajoute des variantes/presets (étape 1+).
/// Presets considérés « rares » (affichés avec un [R] violet en galerie).
const RARES: &[&str] = &[
    "Megaflora", "Petrified", "Recif", "Archipelago", "Geothermal", "Bioluminescent",
    "Salines", "Aquifer", "Coral (aride)", "Primal", "Baobab", "Geoglyph", "Storm",
    "Iceberg", "Cryflora", "Lichen", "Glaciovolcanic", "Lanthanide", "Eyeball humide",
    "Eyeball sec", "Eyeball gele", "Wet Superhabitable", "Dry Superhabitable",
    "Cold Superhabitable", "Pandora", "Polyphemus (Avatar)", "Tempete planetaire (GTB)",
];

/// Un preset est-il rare ?
pub fn est_rare(nom: &str) -> bool {
    RARES.contains(&nom)
}

/// Classe de taille d'un preset tellurique nommé. Les mondes Gaia / super-habitables
/// sont des super-Terres (plus gros) ; les corps rocheux sans air / lunes sont des
/// naines (plus petits). Tout le reste est une tellurique standard.
/// Hachage FNV-1a 32 bits d'un nom. Court, sans dépendance, bien dispersant.
///
/// C'est la **source unique** du peu d'aléa apparent que garde le catalogue :
/// géographie et taille en sont déduites, ce qui les rend reproductibles.
fn hachage(nom: &str) -> u32 {
    let mut h: u32 = 2166136261;
    for o in nom.as_bytes() {
        h ^= *o as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

/// Graine de géographie déduite d'un nom, dans la plage du catalogue (0..1000).
///
/// # Pourquoi le catalogue ne tire plus au sort
///
/// Il posait une graine **aléatoire** à chaque construction. Conséquence : la
/// « Terre » de la galerie n'était pas une référence mais une planète
/// différente à chaque ouverture de l'écran — et le système solaire, qui puise
/// dans le même catalogue, ne pouvait donc **pas** lui ressembler. Il n'y avait
/// rien à quoi ressembler.
///
/// Le brassage n'a pas disparu, il a changé de place : c'est la galerie qui
/// l'applique (touche G), ce qui est son rôle.
pub fn graine_de_nom(nom: &str) -> f32 {
    (hachage(nom) % 100_000) as f32 / 100.0
}

/// Position dans les bornes de classe (0..1), déduite d'un nom.
///
/// Salée différemment de [`graine_de_nom`] : sans ça, deux presets de graine
/// voisine auraient aussi des tailles voisines, et la corrélation se verrait
/// dans la grille de la galerie.
fn fraction_de_nom(nom: &str) -> f32 {
    (hachage(&format!("{nom}~taille")) % 10_000) as f32 / 10_000.0
}

fn classe_tellurique(nom: &str) -> ClasseTaille {
    match nom {
        "Dry Gaia" | "Cold Gaia" | "Wet Superhabitable" | "Dry Superhabitable"
        | "Cold Superhabitable" => ClasseTaille::SuperTerre,
        "Lune" | "Titan" | "Fer (Mercure)" | "Carbone" | "Diamant" | "Mars" => ClasseTaille::Naine,
        _ => ClasseTaille::Tellurique,
    }
}

/// Classe de taille d'un preset gazeux nommé. Les géantes de glace (Neptune/Uranus
/// et apparentées) sont plus petites que les géantes gazeuses ; le sous-Neptune est
/// le plus petit. Naines brunes et géantes classiques → géante gazeuse.
fn classe_gazeuse(nom: &str) -> ClasseTaille {
    match nom {
        "Sub-Neptune" => ClasseTaille::SousNeptune,
        "Uranus" | "Neptune" | "Classe III (sans nuage, azur)" | "Neptune chaud"
        | "Anneau monobande (type Uranus)" | "Anneaux en arcs (type Neptune)" => {
            ClasseTaille::GeanteGlace
        }
        _ => ClasseTaille::GeanteGaz,
    }
}

pub fn catalogue_telluriques() -> Vec<(String, Apparence)> {
    let bleu = vec3(0.35, 0.55, 1.0) * 0.9; // atmosphère océanique
    let voile = vec3(0.6, 0.8, 1.0) * 0.2; // voile glacé
    let sec = vec3(0.8, 0.7, 0.5) * 0.12; // atmosphère sèche légère
    let z = Vec3::ZERO;

    let mut v: Vec<(String, Apparence)> = Vec::new();
    // Chaque preset reçoit une graine aléatoire -> géographie unique, et la touche G
    // (qui change la graine de la galerie) régénère des cartes différentes.
    // **Deterministe.** Une entree de catalogue est une reference : elle doit
    // etre la meme d'une lecture a l'autre, sinon rien ne peut s'y comparer.
    // Le brassage vit desormais dans la galerie (touche G).
    let mut push = |nom: &str, mut app: Apparence| {
        app.seed = graine_de_nom(nom);
        app.taille = classe_tellurique(nom).rayon_pour(fraction_de_nom(nom));
        // **Pas d'air ni d'eau, pas de villes.**
        //
        // `Apparence::villes` vaut 1 par defaut et le shader n'exigeait rien de
        // plus qu'un monde ni de lave ni voile : Mercure, la Lune, Callisto et
        // tous les cailloux morts s'allumaient de lumieres de ville cote nuit.
        // La condition est ici, dans la donnee, parce qu'elle se teste.
        if app.atmo_densite <= 0.0 && app.eau <= 0.0 {
            app.villes = 0.0;
        }
        v.push((nom.to_string(), app));
    };

    // --- Humide ---
    let blanc = vec3(1.0, 1.0, 1.0);
    push("Forest", tellurique(vec3(0.45, 0.4, 0.3), vec3(0.35, 0.3, 0.24), vec3(0.1, 0.32, 0.7), 0.55, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.16, 0.55, 0.16), 0.98).avec_rivieres(0.4).avec_nuages(0.4, blanc));
    push("Monde-ocean", tellurique(vec3(0.2, 0.45, 0.3), vec3(0.25, 0.3, 0.25), vec3(0.06, 0.3, 0.62), 0.92, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_nuages(0.5, blanc));
    // Océan PUR : aucune terre émergée (eau = 1.0 -> quantile au max), banquise polaire.
    push("Ocean pur", tellurique(vec3(0.2, 0.35, 0.4), vec3(0.18, 0.28, 0.36), vec3(0.04, 0.26, 0.58), 1.0, 0.0, 0.4, 0.85, bleu).avec_nuages(0.45, blanc));
    // Hycéan : océan global sous atmosphère épaisse de vapeur (candidat exoplanètes K2-18b).
    // « Épaisse » est sa DÉFINITION — une enveloppe d'hydrogène et de vapeur de
    // plusieurs dizaines de bars — et elle sortait à la moitié de la Terre, pour
    // la même raison que Vénus : sa teinte pâle tirait sa densité vers le bas.
    push("Hycean", tellurique(vec3(0.2, 0.4, 0.45), vec3(0.16, 0.32, 0.4), vec3(0.05, 0.35, 0.55), 1.0, 0.0, 0.25, 1.0, z)
        .avec_air(vec3(0.55, 0.82, 0.92), 1.30)
        .avec_nuages(0.75, vec3(0.95, 0.97, 1.0))
        .avec_voile(0.22, vec3(0.7, 0.85, 0.92)));
    // Océan de MAGMA : mer de roche fondue globale (planète nouveau-née / ultra-chaude).
    // `lave` -> réseau de fissures émissives : la mer brûle aussi côté nuit.
    let mut magma = tellurique(vec3(0.3, 0.16, 0.1), vec3(0.22, 0.12, 0.08), vec3(1.0, 0.38, 0.06), 1.0, 0.0, 0.1, 1.0, vec3(1.0, 0.5, 0.2) * 0.35).avec_nuages(0.25, vec3(0.45, 0.3, 0.28));
    magma.lave = 0.5;
    push("Ocean de magma", magma);
    push("Lacs", tellurique(vec3(0.42, 0.4, 0.28), vec3(0.36, 0.32, 0.24), vec3(0.15, 0.4, 0.7), 0.42, 3.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.52, 0.2), 0.7));
    push("Marais", tellurique(vec3(0.34, 0.36, 0.2), vec3(0.28, 0.26, 0.16), vec3(0.3, 0.42, 0.35), 0.4, 3.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.3, 0.42, 0.2), 0.7));
    push("Algues roses", tellurique(vec3(0.3, 0.45, 0.25), vec3(0.3, 0.3, 0.22), vec3(0.85, 0.35, 0.6), 0.8, 0.0, 0.4, 0.85, bleu));
    push("Recif", tellurique(vec3(0.3, 0.5, 0.3), vec3(0.3, 0.32, 0.24), vec3(0.1, 0.45, 0.7), 0.85, 0.0, 0.4, 0.85, bleu).avec_recifs(0.9).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.4));
    push("Columnar", tellurique(vec3(0.36, 0.37, 0.4), vec3(0.22, 0.23, 0.26), vec3(0.1, 0.32, 0.6), 0.55, 0.0, 0.4, 0.85, bleu).avec_basalt(0.9));

    // --- Végétation colorée (teinte de végétation) ---
    push("Sakura", tellurique(vec3(0.5, 0.42, 0.34), vec3(0.38, 0.3, 0.26), vec3(0.12, 0.34, 0.66), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.95, 0.55, 0.75), 0.85));
    push("Retinal", tellurique(vec3(0.4, 0.36, 0.34), vec3(0.3, 0.26, 0.26), vec3(0.1, 0.3, 0.62), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.55, 0.25, 0.7), 0.85));
    push("Carotene", tellurique(vec3(0.5, 0.42, 0.3), vec3(0.4, 0.32, 0.22), vec3(0.12, 0.34, 0.6), 0.45, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.88, 0.55, 0.12), 0.85));
    push("Mousse", tellurique(vec3(0.3, 0.34, 0.24), vec3(0.26, 0.28, 0.2), vec3(0.15, 0.36, 0.5), 0.35, 1.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.26, 0.62, 0.22), 1.0));
    push("Bioluminescent", tellurique(vec3(0.12, 0.2, 0.18), vec3(0.08, 0.14, 0.13), vec3(0.04, 0.18, 0.28), 0.6, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.1, 0.3, 0.22), 0.6).avec_biolum(0.95).avec_nuages(0.3, vec3(0.5, 0.6, 0.7)));

    // --- Météo (couche nuageuse) ---
    push("Brumeux", tellurique(vec3(0.4, 0.45, 0.4), vec3(0.32, 0.35, 0.32), vec3(0.2, 0.4, 0.55), 0.6, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.25, 0.45, 0.25), 0.6).avec_nuages(0.85, blanc));
    push("Orageux", tellurique(vec3(0.3, 0.42, 0.26), vec3(0.26, 0.32, 0.22), vec3(0.08, 0.3, 0.55), 0.75, 0.0, 0.35, 0.88, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.7).avec_meteo(0.85, vec3(0.5, 0.52, 0.58), 1.0));
    push("Cyclone", tellurique(vec3(0.25, 0.42, 0.3), vec3(0.26, 0.3, 0.24), vec3(0.06, 0.3, 0.6), 0.78, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_meteo(0.8, blanc, 2.0));
    push("Tempete de poussiere", tellurique(vec3(0.78, 0.58, 0.34), vec3(0.55, 0.4, 0.24), z, 0.0, 1.0, 0.2, 0.95, sec).avec_meteo(0.6, vec3(0.78, 0.62, 0.4), 1.0));

    // --- Sec ---
    push("Desert", tellurique(vec3(0.8, 0.6, 0.35), vec3(0.55, 0.4, 0.25), vec3(0.4, 0.5, 0.6), 0.04, 2.0, 0.25, 0.95, sec).avec_dunes(0.6));
    push("Dune (erg)", tellurique(vec3(0.85, 0.66, 0.38), vec3(0.62, 0.45, 0.26), z, 0.0, 1.0, 0.15, 0.97, sec).avec_relief(0.15).avec_dunes(0.9));
    push("Sable ferreux", tellurique(vec3(0.7, 0.3, 0.2), vec3(0.5, 0.22, 0.16), z, 0.0, 1.0, 0.2, 0.95, sec).avec_dunes(0.55));
    push("Mediterraneenne", tellurique(vec3(0.6, 0.55, 0.3), vec3(0.45, 0.4, 0.24), vec3(0.15, 0.4, 0.6), 0.42, 2.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.35, 0.5, 0.2), 0.45).avec_rivieres(0.7));
    push("Salines", tellurique(vec3(0.85, 0.82, 0.78), vec3(0.6, 0.55, 0.5), z, 0.0, 1.0, 0.2, 0.95, sec));
    push("Mesa", tellurique(vec3(0.72, 0.42, 0.28), vec3(0.5, 0.3, 0.22), vec3(0.2, 0.45, 0.55), 0.12, 2.0, 0.3, 0.9, sec).avec_relief(0.5).avec_mesa(0.8).avec_rivieres(0.5).avec_vegetation(vec3(0.3, 0.45, 0.2), 0.3));
    push("Striped", tellurique(vec3(0.72, 0.5, 0.3), vec3(0.32, 0.46, 0.7), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.95));

    // --- Froid ---
    push("Toundra", tellurique(vec3(0.4, 0.42, 0.32), vec3(0.3, 0.3, 0.26), vec3(0.6, 0.7, 0.8), 0.15, 1.0, 0.6, 0.6, voile));
    push("Arctique", tellurique(vec3(0.55, 0.6, 0.62), vec3(0.4, 0.45, 0.5), vec3(0.5, 0.6, 0.75), 0.2, 1.0, 0.7, 0.4, voile));
    push("Boule de neige", tellurique(vec3(0.85, 0.9, 0.96), vec3(0.7, 0.78, 0.88), z, 0.0, 1.0, 0.7, 0.15, voile).avec_pics(0.5));
    push("Alpin", tellurique(vec3(0.42, 0.4, 0.34), vec3(0.34, 0.32, 0.3), vec3(0.2, 0.4, 0.6), 0.25, 1.0, 0.55, 0.7, voile).avec_vegetation(vec3(0.18, 0.42, 0.2), 0.6).avec_relief(0.9).avec_nuages(0.25, blanc));
    push("Pics de glace", tellurique(vec3(0.78, 0.85, 0.95), vec3(0.62, 0.72, 0.85), z, 0.0, 1.0, 0.7, 0.3, voile).avec_relief(0.4).avec_pics(0.9));
    // Combo : montagnes + dunes en plaine + forêts en altitude (features cumulées).
    push("Dune Forest", tellurique(vec3(0.62, 0.52, 0.32), vec3(0.42, 0.36, 0.24), vec3(0.15, 0.4, 0.6), 0.18, 1.0, 0.4, 0.7, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.75).avec_relief(0.75).avec_dunes(0.7).avec_nuages(0.25, blanc));
    let mut cryovolc = tellurique(vec3(0.4, 0.44, 0.4), vec3(0.3, 0.32, 0.3), vec3(0.4, 0.55, 0.72), 0.12, 1.0, 0.5, 0.45, voile).avec_vegetation(vec3(0.32, 0.42, 0.3), 0.4).avec_relief(0.55).avec_mesa(0.4).avec_rivieres(0.4).avec_riv_lave();
    cryovolc.lave = 0.25; // chaleur volcanique sous la glace + toundra
    push("Cryovolcan", cryovolc);

    // --- Extrêmes / exotiques ---
    let mut lave = tellurique(vec3(0.16, 0.10, 0.08), vec3(0.07, 0.05, 0.04), z, 0.0, 1.0, 0.0, 1.0, z);
    lave.lave = 1.0;
    lave.relief = 0.0;
    push("Lave", lave);
    // **Vénus était notée deux fois plus fine que la Terre.** Son air valait
    // `* 0.25` : la plus basse densité du catalogue après les cailloux nus, pour
    // 92 bars — l'atmosphère la plus lourde de tout le système solaire interne.
    //
    // Ce n'était pas une erreur de jugement mais une impossibilite d'ecriture :
    // `atmo` portait la teinte ET la quantite dans un seul Vec3, si bien qu'un
    // halo TERNE ne pouvait etre note que comme un halo MINCE. Vénus est
    // précisément le monde « épais mais sombre » — sa brume d'acide sulfurique
    // ne brille pas, elle étouffe. Elle n'était rattrapée que par le terme
    // `voile` du shader, c'est-à-dire par un effet de bord.
    //
    // Densité et teinte séparées, elle passe au sommet de l'échelle.
    push("Venus (etuve)", tellurique(vec3(0.7, 0.6, 0.4), vec3(0.5, 0.42, 0.28), z, 0.0, 1.0, 0.15, 1.0, z)
        .avec_air(vec3(0.90, 0.78, 0.42), 1.45)
        .avec_voile(0.93, vec3(0.93, 0.82, 0.5)));
    // Titan porte un voile orange epais -- c'est meme ce qui le caracterise --
    // mais son `atmo` etait a ZERO : le halo de limbe manquait, alors que le
    // voile, lui, etait bien la. Une lune a l'atmosphere la plus dense du
    // systeme solaire rendue sans halo.
    // Même correction, même cause : 1,5 bar, plus dense que l'air terrestre, et
    // une brume orange qui monte à 600 km — la couche la plus haute du système.
    // Notée `* 0.3`, elle sortait quatre fois plus fine que la Terre.
    push("Titan", tellurique(vec3(0.6, 0.45, 0.25), vec3(0.45, 0.32, 0.2), vec3(0.5, 0.35, 0.15), 0.25, 3.0, 0.2, 0.9, z)
        .avec_air(vec3(0.85, 0.55, 0.25), 1.20)
        .avec_voile(0.78, vec3(0.85, 0.55, 0.25)));
    push("Fer (Mercure)", tellurique(vec3(0.5, 0.48, 0.45), vec3(0.32, 0.3, 0.28), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.4).avec_crateres(0.85));
    // Mars a son preset propre, comme Mercure et Venus -- il empruntait
    // « Badlands », dont la rouille est trop sourde et le mesa trop marque.
    //
    // Les teintes viennent des vraies : regions claires en oxyde de fer
    // (butterscotch, pas rouge brique) et regions sombres basaltiques. Ce
    // CONTRASTE entre les deux est ce qui fait lire Mars ; une planete d'une
    // seule rouille uniforme paraît terne, surtout apres quantification.
    // Son atmosphere valait `* 0.10` -- la plus faible du catalogue hors zero,
    // sous meme la constante des mondes secs. Ce chiffre reglait l'ANCIEN halo,
    // qui se peignait SUR la surface : le monter aurait delave le contraste
    // clair/sombre travaille juste au-dessus. La coquille, elle, vit surtout
    // DEHORS -- au centre du disque elle ne laisse que ~4 %.
    //
    // 0.40 n'est pas le rapport des pressions (Mars fait 0,6 % de la Terre :
    // a ce compte elle serait nue). C'est ce que Mars *montre* : une brume
    // chargee de poussiere, franchement visible au limbe depuis l'orbite, et
    // dont l'echelle de hauteur vaut 2,4 fois celle de la Terre rapportee au
    // rayon. Mince en densite, haute en geometrie.
    push("Mars", tellurique(vec3(0.80, 0.44, 0.24), vec3(0.46, 0.26, 0.19), z, 0.0, 1.0, 0.35, 0.90, vec3(0.85, 0.5, 0.35) * 0.40)
        .avec_relief(0.45)
        .avec_mesa(0.45)      // Valles Marineris, plateaux etages
        .avec_dunes(0.35)     // ergs polaires
        .avec_crateres(0.55)  // hemisphere sud crateurise
        .avec_meteo(0.25, vec3(0.82, 0.62, 0.42), 0.6)); // tempetes de poussiere
    push("Carbone", tellurique(vec3(0.12, 0.12, 0.14), vec3(0.06, 0.06, 0.07), z, 0.0, 1.0, 0.0, 1.0, z).avec_crateres(0.4));
    push("Lune", tellurique(vec3(0.55, 0.54, 0.52), vec3(0.34, 0.33, 0.32), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.4).avec_crateres(0.95));
    push("Diamant", tellurique(vec3(0.22, 0.26, 0.34), vec3(0.1, 0.12, 0.17), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.5).avec_basalt(0.6));
    push("Subglaciaire", tellurique(vec3(0.82, 0.88, 0.95), vec3(0.7, 0.64, 0.62), z, 0.0, 1.0, 0.2, 0.05, voile).avec_relief(0.2));
    let mut chth = tellurique(vec3(0.32, 0.13, 0.1), vec3(0.18, 0.08, 0.07), z, 0.0, 1.0, 0.0, 1.0, z).avec_relief(0.6);
    chth.lave = 0.5;
    push("Chthonien", chth);
    // Ash portait `z` : aucune atmosphère. Or ce qui fait ce monde — ce qui lui
    // donne son nom — ce sont les cendres EN SUSPENSION, et de la cendre en
    // suspension est de l'air. La note à zéro venait du même amalgame : sa brume
    // est grise et sombre, donc inécrivable tant que la teinte portait la
    // quantité. Fine (entre le caillou sec et Mars), mais elle existe.
    let mut ash = tellurique(vec3(0.3, 0.28, 0.27), vec3(0.16, 0.15, 0.14), z, 0.0, 1.0, 0.1, 1.0, z)
        .avec_air(vec3(0.42, 0.38, 0.35), 0.32)
        .avec_dunes(0.4);
    ash.lave = 0.3;
    // Donner de l'air à Ash la fait passer la garde « ni air ni eau -> pas de
    // villes » : un monde de cendre et de lave se serait mis à briller de
    // lumières urbaines côté nuit. La cendre n'est pas respirable, et le shader
    // ne bloque qu'au-delà de `lave >= 0.5`. Décision portée par la donnée.
    ash.villes = 0.0;
    push("Ash (volcanique)", ash);

    // --- Première passe de variantes nommées (presets de paramètres) ---
    // Monde type Terre : océans + continents verts + déserts (zones sèches) + calottes
    // polaires + montagnes + rivières + nuages -> toute la diversité de notre planète.
    //
    // `eau = 0,71`, la vraie fraction d'océan terrestre (62 % sous-estimait
    // la vraie planète). Ce n'était **pas** la cause d'une Terre trop verte
    // dans l'icône du sélecteur — remonter `eau` jusqu'à 0,78 n'y changeait
    // quasiment rien, la vraie cause étant ailleurs (le terrain se génère de
    // façon asynchrone, et l'icône pouvait figer un repli capté avant la fin
    // du job — voir `Astre::rendu_pret`, corrigé dans `ecran/icone.rs`).
    push("Terre", tellurique(vec3(0.5, 0.42, 0.3), vec3(0.4, 0.34, 0.26), vec3(0.07, 0.32, 0.7), 0.71, 1.0, 0.5, 0.8, bleu).avec_vegetation(vec3(0.16, 0.5, 0.18), 0.82).avec_relief(0.65).avec_rivieres(0.55).avec_dunes(0.3).avec_nuages(0.42, blanc));

    // Mélange de styles (un seul preset cumule plusieurs axes) : lune type Pandora.
    push("Pandora", tellurique(vec3(0.2, 0.4, 0.3), vec3(0.22, 0.3, 0.26), vec3(0.06, 0.34, 0.55), 0.7, 0.0, 0.42, 0.9, bleu).avec_vegetation(vec3(0.14, 0.52, 0.32), 0.95).avec_relief(0.85).avec_rivieres(0.4).avec_biolum(0.9).avec_nuages(0.4, blanc));

    // Humide
    push("Mushroom", tellurique(vec3(0.4, 0.36, 0.3), vec3(0.3, 0.27, 0.24), vec3(0.12, 0.34, 0.6), 0.45, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.45, 0.3, 0.45), 0.85).avec_nuages(0.3, blanc));
    push("Tepid", tellurique(vec3(0.45, 0.42, 0.28), vec3(0.34, 0.3, 0.22), vec3(0.1, 0.36, 0.66), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.5, 0.62, 0.15), 0.9).avec_rivieres(0.4));
    push("Swamp", tellurique(vec3(0.34, 0.36, 0.22), vec3(0.26, 0.26, 0.18), vec3(0.2, 0.36, 0.32), 0.55, 3.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.28, 0.4, 0.2), 0.75).avec_nuages(0.4, blanc));
    push("Kelp", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.1, 0.4, 0.4), 0.85, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.4));
    push("Tidepool", tellurique(vec3(0.42, 0.42, 0.36), vec3(0.32, 0.3, 0.26), vec3(0.12, 0.42, 0.55), 0.6, 1.0, 0.4, 0.85, bleu).avec_relief(0.5).avec_vegetation(vec3(0.25, 0.45, 0.25), 0.4));
    // Tropical
    push("Atoll", tellurique(vec3(0.3, 0.5, 0.3), vec3(0.3, 0.32, 0.24), vec3(0.1, 0.5, 0.65), 0.82, 0.0, 0.35, 0.95, bleu).avec_recifs(0.85).avec_vegetation(vec3(0.2, 0.55, 0.2), 0.5).avec_nuages(0.4, blanc));
    push("Mangrove", tellurique(vec3(0.32, 0.4, 0.24), vec3(0.26, 0.3, 0.2), vec3(0.15, 0.42, 0.45), 0.7, 3.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.2, 0.45, 0.2), 0.7).avec_nuages(0.45, blanc));
    // Savane / aride
    push("Steppe", tellurique(vec3(0.7, 0.62, 0.32), vec3(0.5, 0.44, 0.26), z, 0.06, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.55, 0.55, 0.2), 0.5));
    push("Veldt", tellurique(vec3(0.65, 0.55, 0.3), vec3(0.46, 0.4, 0.24), z, 0.05, 1.0, 0.3, 0.92, sec).avec_relief(0.5).avec_vegetation(vec3(0.5, 0.52, 0.22), 0.45));
    push("Acacia", tellurique(vec3(0.6, 0.55, 0.3), vec3(0.44, 0.4, 0.24), vec3(0.2, 0.4, 0.5), 0.12, 2.0, 0.32, 0.9, sec).avec_vegetation(vec3(0.4, 0.5, 0.2), 0.5).avec_rivieres(0.4));
    push("Badlands", tellurique(vec3(0.7, 0.4, 0.28), vec3(0.5, 0.28, 0.2), z, 0.0, 1.0, 0.25, 0.95, sec).avec_mesa(0.7).avec_relief(0.4));
    push("Fog Desert", tellurique(vec3(0.72, 0.62, 0.42), vec3(0.52, 0.42, 0.3), z, 0.02, 2.0, 0.25, 0.95, sec).avec_dunes(0.4).avec_nuages(0.7, vec3(0.9, 0.9, 0.92)));
    push("Amethyst", tellurique(vec3(0.5, 0.35, 0.6), vec3(0.32, 0.22, 0.4), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.5).avec_relief(0.4));
    push("Cactus", tellurique(vec3(0.75, 0.6, 0.35), vec3(0.55, 0.42, 0.26), z, 0.03, 2.0, 0.25, 0.95, sec).avec_dunes(0.3).avec_vegetation(vec3(0.3, 0.5, 0.25), 0.25));
    // Froid
    push("Boreal", tellurique(vec3(0.35, 0.4, 0.3), vec3(0.3, 0.32, 0.28), vec3(0.2, 0.4, 0.55), 0.25, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.16, 0.4, 0.22), 0.6).avec_relief(0.5));
    push("Taiga", tellurique(vec3(0.32, 0.38, 0.3), vec3(0.28, 0.3, 0.26), vec3(0.18, 0.38, 0.52), 0.2, 1.0, 0.55, 0.5, voile).avec_vegetation(vec3(0.15, 0.36, 0.2), 0.55).avec_relief(0.7).avec_nuages(0.3, blanc));
    push("Fjord", tellurique(vec3(0.4, 0.44, 0.38), vec3(0.32, 0.34, 0.3), vec3(0.15, 0.4, 0.55), 0.35, 1.0, 0.55, 0.5, voile).avec_relief(0.85).avec_vegetation(vec3(0.18, 0.42, 0.24), 0.45));
    push("Glacial", tellurique(vec3(0.6, 0.66, 0.72), vec3(0.45, 0.5, 0.56), vec3(0.4, 0.55, 0.7), 0.2, 1.0, 0.7, 0.4, voile).avec_pics(0.3));
    push("Cold Desert", tellurique(vec3(0.62, 0.6, 0.58), vec3(0.45, 0.43, 0.42), z, 0.0, 1.0, 0.6, 0.45, voile).avec_dunes(0.3));
    push("Ice Dunes", tellurique(vec3(0.8, 0.85, 0.92), vec3(0.62, 0.68, 0.78), z, 0.0, 1.0, 0.7, 0.25, voile).avec_dunes(0.7));
    push("Bog", tellurique(vec3(0.36, 0.38, 0.26), vec3(0.28, 0.28, 0.22), vec3(0.22, 0.34, 0.3), 0.4, 3.0, 0.6, 0.6, voile).avec_vegetation(vec3(0.3, 0.4, 0.22), 0.5));

    // --- Deuxième passe de variantes nommées ---
    // Continental
    push("Megaflora", tellurique(vec3(0.3, 0.34, 0.24), vec3(0.24, 0.26, 0.2), vec3(0.1, 0.32, 0.6), 0.5, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.1, 0.4, 0.14), 1.0).avec_relief(0.55).avec_nuages(0.4, blanc));
    push("Petrified", tellurique(vec3(0.6, 0.55, 0.45), vec3(0.42, 0.38, 0.32), z, 0.0, 1.0, 0.3, 0.9, sec).avec_mesa(0.4).avec_relief(0.4));
    // Océan
    push("Cascadian", tellurique(vec3(0.35, 0.42, 0.3), vec3(0.3, 0.32, 0.26), vec3(0.1, 0.34, 0.55), 0.6, 1.0, 0.45, 0.7, bleu).avec_vegetation(vec3(0.16, 0.42, 0.2), 0.7).avec_relief(0.8).avec_nuages(0.6, vec3(0.8, 0.82, 0.85)));
    push("Archipelago", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.06, 0.32, 0.6), 0.88, 0.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.2, 0.5, 0.2), 0.5).avec_relief(0.4));
    push("Crag", tellurique(vec3(0.45, 0.45, 0.42), vec3(0.34, 0.34, 0.32), vec3(0.1, 0.36, 0.55), 0.7, 1.0, 0.55, 0.5, voile).avec_relief(0.7));
    // Tropical
    push("Geothermal", tellurique(vec3(0.5, 0.55, 0.55), vec3(0.4, 0.44, 0.46), vec3(0.3, 0.5, 0.6), 0.15, 1.0, 0.55, 0.45, voile).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.35).avec_relief(0.5).avec_rivieres(0.35).avec_riv_lave());
    push("Tepui", tellurique(vec3(0.4, 0.42, 0.28), vec3(0.32, 0.3, 0.22), vec3(0.1, 0.4, 0.6), 0.4, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.16, 0.45, 0.2), 0.7).avec_mesa(0.6).avec_relief(0.5).avec_nuages(0.5, blanc));
    push("Lilypad", tellurique(vec3(0.3, 0.45, 0.3), vec3(0.28, 0.3, 0.24), vec3(0.2, 0.5, 0.3), 0.85, 0.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.25, 0.55, 0.25), 0.45));
    push("Obsidian", tellurique(vec3(0.25, 0.4, 0.3), vec3(0.08, 0.08, 0.1), vec3(0.1, 0.4, 0.55), 0.55, 1.0, 0.35, 0.88, bleu).avec_vegetation(vec3(0.2, 0.5, 0.22), 0.5).avec_basalt(0.5));
    // Désert / aride
    push("Oasis", tellurique(vec3(0.82, 0.64, 0.36), vec3(0.58, 0.42, 0.26), vec3(0.1, 0.45, 0.6), 0.06, 2.0, 0.25, 0.95, sec).avec_dunes(0.5).avec_vegetation(vec3(0.25, 0.5, 0.22), 0.3).avec_rivieres(0.3));
    push("Outback", tellurique(vec3(0.7, 0.45, 0.3), vec3(0.5, 0.32, 0.22), z, 0.03, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.45, 0.45, 0.2), 0.3).avec_dunes(0.3));
    push("Coastal", tellurique(vec3(0.7, 0.6, 0.4), vec3(0.5, 0.42, 0.3), vec3(0.12, 0.4, 0.6), 0.45, 1.0, 0.3, 0.9, bleu).avec_vegetation(vec3(0.35, 0.5, 0.25), 0.3).avec_nuages(0.6, vec3(0.9, 0.9, 0.92)));
    push("Sodalite", tellurique(vec3(0.6, 0.55, 0.4), vec3(0.25, 0.35, 0.62), z, 0.0, 1.0, 0.2, 0.95, sec).avec_mesa(0.9));
    push("Coral (aride)", tellurique(vec3(0.75, 0.5, 0.4), vec3(0.55, 0.35, 0.3), vec3(0.2, 0.55, 0.6), 0.15, 2.0, 0.3, 0.92, sec).avec_recifs(0.7).avec_vegetation(vec3(0.4, 0.5, 0.25), 0.2));
    push("Opal", tellurique(vec3(0.7, 0.8, 0.85), vec3(0.55, 0.6, 0.7), z, 0.0, 1.0, 0.2, 0.92, sec).avec_mesa(0.6));
    push("Superbloom", tellurique(vec3(0.75, 0.55, 0.35), vec3(0.55, 0.4, 0.26), z, 0.02, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.85, 0.4, 0.55), 0.4));
    let mut primal = tellurique(vec3(0.35, 0.22, 0.18), vec3(0.2, 0.12, 0.1), z, 0.0, 1.0, 0.2, 0.95, z).avec_relief(0.5);
    primal.lave = 0.4;
    push("Primal", primal);
    // Savane
    push("Pampa", tellurique(vec3(0.65, 0.62, 0.35), vec3(0.48, 0.44, 0.28), vec3(0.15, 0.4, 0.55), 0.1, 2.0, 0.35, 0.9, bleu).avec_vegetation(vec3(0.5, 0.6, 0.2), 0.6).avec_rivieres(0.3));
    push("Heath", tellurique(vec3(0.5, 0.42, 0.4), vec3(0.36, 0.3, 0.3), z, 0.05, 1.0, 0.4, 0.85, voile).avec_vegetation(vec3(0.4, 0.3, 0.4), 0.5));
    push("Bushveld", tellurique(vec3(0.6, 0.55, 0.32), vec3(0.44, 0.4, 0.24), vec3(0.15, 0.4, 0.5), 0.1, 2.0, 0.35, 0.9, sec).avec_vegetation(vec3(0.3, 0.48, 0.2), 0.55).avec_relief(0.4));
    // Arctique
    push("Antarctic", tellurique(vec3(0.8, 0.86, 0.92), vec3(0.62, 0.68, 0.78), z, 0.0, 1.0, 0.75, 0.25, voile).avec_pics(0.4).avec_relief(0.4));
    push("Iceberg", tellurique(vec3(0.75, 0.82, 0.9), vec3(0.55, 0.62, 0.72), vec3(0.3, 0.5, 0.7), 0.5, 0.0, 0.7, 0.4, voile).avec_pics(0.3));
    push("Storm", tellurique(vec3(0.45, 0.5, 0.55), vec3(0.35, 0.4, 0.45), vec3(0.1, 0.3, 0.5), 0.8, 0.0, 0.6, 0.45, voile).avec_meteo(0.85, vec3(0.55, 0.57, 0.63), 2.0).avec_cyclones(1.0));
    // Toundra
    let mut mud = tellurique(vec3(0.42, 0.36, 0.26), vec3(0.3, 0.25, 0.18), vec3(0.3, 0.28, 0.22), 0.4, 3.0, 0.55, 0.6, voile).avec_vegetation(vec3(0.35, 0.38, 0.22), 0.4);
    mud.lave = 0.2;
    push("Mud", mud);
    push("Travertine", tellurique(vec3(0.88, 0.85, 0.78), vec3(0.6, 0.58, 0.52), vec3(0.3, 0.5, 0.55), 0.1, 2.0, 0.5, 0.55, voile).avec_mesa(0.85));
    push("Lichen", tellurique(vec3(0.5, 0.52, 0.42), vec3(0.38, 0.4, 0.34), z, 0.0, 1.0, 0.55, 0.5, voile).avec_vegetation(vec3(0.45, 0.5, 0.3), 0.45).avec_relief(0.6));
    push("Cryflora", tellurique(vec3(0.6, 0.7, 0.78), vec3(0.45, 0.55, 0.65), z, 0.05, 1.0, 0.6, 0.4, voile).avec_vegetation(vec3(0.2, 0.7, 0.5), 0.4).avec_biolum(0.7));
    // Alpin
    push("Highland", tellurique(vec3(0.45, 0.46, 0.4), vec3(0.34, 0.36, 0.32), vec3(0.15, 0.4, 0.5), 0.2, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.3, 0.42, 0.25), 0.45).avec_relief(0.6).avec_nuages(0.7, vec3(0.85, 0.87, 0.9)));
    push("Snow", tellurique(vec3(0.7, 0.76, 0.84), vec3(0.55, 0.6, 0.7), vec3(0.2, 0.4, 0.6), 0.15, 1.0, 0.65, 0.35, voile).avec_vegetation(vec3(0.16, 0.4, 0.22), 0.5));
    push("Blossom", tellurique(vec3(0.5, 0.5, 0.42), vec3(0.38, 0.38, 0.32), vec3(0.15, 0.4, 0.55), 0.2, 1.0, 0.5, 0.5, voile).avec_vegetation(vec3(0.8, 0.5, 0.7), 0.6).avec_relief(0.5));
    let mut glaciovolc = tellurique(vec3(0.55, 0.62, 0.7), vec3(0.42, 0.48, 0.56), vec3(0.3, 0.5, 0.6), 0.12, 1.0, 0.55, 0.4, voile).avec_relief(0.6).avec_mesa(0.6).avec_rivieres(0.5).avec_riv_lave();
    glaciovolc.lave = 0.35; // lave qui transparaît sous la glace fracturée
    push("Glaciovolcanic", glaciovolc);
    // Gaia / superhabitables
    push("Dry Gaia", tellurique(vec3(0.7, 0.6, 0.35), vec3(0.5, 0.42, 0.26), vec3(0.12, 0.45, 0.65), 0.35, 1.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.4, 0.55, 0.2), 0.7).avec_rivieres(0.4).avec_nuages(0.3, blanc));
    push("Cold Gaia", tellurique(vec3(0.4, 0.45, 0.36), vec3(0.32, 0.36, 0.3), vec3(0.12, 0.4, 0.6), 0.4, 1.0, 0.5, 0.55, voile).avec_vegetation(vec3(0.2, 0.5, 0.25), 0.7).avec_relief(0.5).avec_nuages(0.3, blanc));
    push("Wet Superhabitable", tellurique(vec3(0.3, 0.5, 0.28), vec3(0.3, 0.34, 0.24), vec3(0.08, 0.4, 0.7), 0.6, 1.0, 0.4, 0.84, bleu).avec_vegetation(vec3(0.16, 0.55, 0.18), 0.85).avec_rivieres(0.5).avec_nuages(0.4, blanc));

    // --- Troisième passe (variantes restantes + flavor) ---
    push("Barnacle", tellurique(vec3(0.5, 0.48, 0.42), vec3(0.36, 0.34, 0.3), vec3(0.1, 0.36, 0.55), 0.8, 0.0, 0.4, 0.85, bleu).avec_relief(0.5).avec_basalt(0.3));
    push("Cenote", tellurique(vec3(0.4, 0.5, 0.3), vec3(0.32, 0.34, 0.24), vec3(0.12, 0.45, 0.6), 0.25, 3.0, 0.4, 0.88, bleu).avec_vegetation(vec3(0.18, 0.5, 0.2), 0.7));
    push("Fungal", tellurique(vec3(0.4, 0.36, 0.34), vec3(0.3, 0.26, 0.26), vec3(0.12, 0.4, 0.55), 0.4, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.5, 0.3, 0.45), 0.8).avec_nuages(0.3, blanc));
    push("Aerial", tellurique(vec3(0.3, 0.46, 0.3), vec3(0.28, 0.32, 0.24), vec3(0.1, 0.4, 0.62), 0.5, 1.0, 0.4, 0.85, bleu).avec_vegetation(vec3(0.18, 0.52, 0.2), 0.8).avec_nuages(0.6, blanc));
    push("Aquifer", tellurique(vec3(0.82, 0.62, 0.36), vec3(0.58, 0.42, 0.26), vec3(0.1, 0.45, 0.6), 0.05, 2.0, 0.25, 0.95, sec).avec_dunes(0.6).avec_vegetation(vec3(0.3, 0.5, 0.22), 0.2));
    push("Fungi", tellurique(vec3(0.42, 0.4, 0.38), vec3(0.28, 0.27, 0.26), z, 0.0, 1.0, 0.25, 0.92, sec).avec_vegetation(vec3(0.45, 0.4, 0.3), 0.35).avec_dunes(0.3));
    push("Succulent", tellurique(vec3(0.7, 0.58, 0.36), vec3(0.5, 0.4, 0.26), z, 0.02, 2.0, 0.3, 0.92, sec).avec_vegetation(vec3(0.35, 0.52, 0.28), 0.35));
    push("Baobab", tellurique(vec3(0.66, 0.58, 0.32), vec3(0.48, 0.42, 0.26), vec3(0.15, 0.4, 0.5), 0.08, 2.0, 0.32, 0.9, sec).avec_vegetation(vec3(0.4, 0.5, 0.2), 0.4).avec_relief(0.3));
    push("Scrubland", tellurique(vec3(0.68, 0.56, 0.34), vec3(0.5, 0.4, 0.26), z, 0.04, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.45, 0.45, 0.24), 0.4));
    push("Geoglyph", tellurique(vec3(0.7, 0.6, 0.36), vec3(0.5, 0.42, 0.26), z, 0.05, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.5, 0.52, 0.22), 0.45).avec_mesa(0.3));
    push("Termite", tellurique(vec3(0.66, 0.5, 0.3), vec3(0.48, 0.36, 0.22), z, 0.04, 2.0, 0.32, 0.92, sec).avec_vegetation(vec3(0.45, 0.46, 0.22), 0.4).avec_dunes(0.25));
    push("Amber", tellurique(vec3(0.7, 0.55, 0.28), vec3(0.5, 0.38, 0.2), vec3(0.2, 0.4, 0.45), 0.1, 2.0, 0.32, 0.9, vec3(0.9, 0.6, 0.2) * 0.3).avec_vegetation(vec3(0.7, 0.5, 0.15), 0.55));
    push("Aeolian", tellurique(vec3(0.6, 0.62, 0.68), vec3(0.45, 0.47, 0.52), z, 0.0, 1.0, 0.6, 0.45, voile).avec_mesa(0.6).avec_relief(0.6).avec_nuages(0.5, vec3(0.82, 0.84, 0.88)).avec_cyclones(0.3));
    push("Supraglacial", tellurique(vec3(0.78, 0.84, 0.92), vec3(0.6, 0.66, 0.76), vec3(0.35, 0.55, 0.72), 0.35, 3.0, 0.7, 0.3, voile));
    let mut crevasse = tellurique(vec3(0.62, 0.72, 0.85), vec3(0.4, 0.5, 0.66), z, 0.0, 1.0, 0.6, 0.4, voile).avec_mesa(0.85).avec_relief(0.6).avec_rivieres(0.55).avec_riv_lave();
    crevasse.lave = 0.3; // chaleur géothermique qui transparaît au fond des crevasses
    push("Crevasse", crevasse);
    push("Ferrosprings", tellurique(vec3(0.72, 0.78, 0.86), vec3(0.55, 0.6, 0.7), vec3(0.7, 0.25, 0.15), 0.18, 3.0, 0.65, 0.4, voile).avec_rivieres(0.45));
    push("Tuya", tellurique(vec3(0.5, 0.55, 0.6), vec3(0.4, 0.44, 0.48), z, 0.0, 1.0, 0.6, 0.45, voile).avec_relief(0.6).avec_rivieres(0.35).avec_riv_lave());
    push("Peatland", tellurique(vec3(0.4, 0.38, 0.28), vec3(0.3, 0.28, 0.22), vec3(0.25, 0.32, 0.28), 0.35, 3.0, 0.55, 0.6, voile).avec_vegetation(vec3(0.34, 0.4, 0.24), 0.5));
    push("Treeline", tellurique(vec3(0.5, 0.55, 0.6), vec3(0.4, 0.44, 0.48), vec3(0.2, 0.4, 0.55), 0.2, 1.0, 0.7, 0.35, voile).avec_vegetation(vec3(0.16, 0.45, 0.22), 0.8));
    push("Lanthanide", tellurique(vec3(0.55, 0.5, 0.5), vec3(0.36, 0.33, 0.36), z, 0.0, 1.0, 0.3, 0.9, z).avec_relief(0.85).avec_crateres(0.3));
    push("Ravine", tellurique(vec3(0.5, 0.46, 0.4), vec3(0.38, 0.34, 0.3), vec3(0.15, 0.4, 0.55), 0.25, 1.0, 0.5, 0.55, voile).avec_mesa(0.6).avec_rivieres(0.5).avec_relief(0.6).avec_vegetation(vec3(0.2, 0.42, 0.24), 0.4));
    push("Craton", tellurique(vec3(0.65, 0.58, 0.42), vec3(0.48, 0.42, 0.3), z, 0.05, 2.0, 0.35, 0.9, sec).avec_mesa(0.4));
    push("Dry Superhabitable", tellurique(vec3(0.68, 0.6, 0.34), vec3(0.5, 0.42, 0.26), vec3(0.12, 0.45, 0.65), 0.4, 1.0, 0.35, 0.85, bleu).avec_vegetation(vec3(0.4, 0.55, 0.2), 0.75).avec_rivieres(0.5).avec_nuages(0.3, blanc));
    push("Cold Superhabitable", tellurique(vec3(0.4, 0.46, 0.36), vec3(0.32, 0.36, 0.3), vec3(0.12, 0.4, 0.6), 0.45, 1.0, 0.5, 0.5, voile).avec_vegetation(vec3(0.2, 0.52, 0.26), 0.75).avec_relief(0.5).avec_nuages(0.3, blanc));
    // Mondes-grottes (flavor : rendus comme leur famille parente)
    push("Wet Cave", tellurique(vec3(0.4, 0.42, 0.3), vec3(0.3, 0.3, 0.24), vec3(0.1, 0.34, 0.55), 0.4, 1.0, 0.4, 0.8, bleu).avec_vegetation(vec3(0.2, 0.45, 0.22), 0.5).avec_relief(0.5));
    push("Dry Cave", tellurique(vec3(0.7, 0.55, 0.34), vec3(0.5, 0.38, 0.24), z, 0.02, 2.0, 0.3, 0.92, sec).avec_dunes(0.3).avec_relief(0.5));
    push("Cold Cave", tellurique(vec3(0.6, 0.64, 0.7), vec3(0.46, 0.5, 0.56), z, 0.0, 1.0, 0.6, 0.4, voile).avec_relief(0.5).avec_pics(0.3));
    // Soufre / Io
    let mut io = tellurique(vec3(0.85, 0.7, 0.2), vec3(0.6, 0.4, 0.1), z, 0.0, 1.0, 0.1, 1.0, vec3(0.9, 0.8, 0.3) * 0.2).avec_dunes(0.2);
    io.lave = 0.5;
    push("Io (soufre)", io);

    // --- Verrouillées par marée (eyeball : jour brûlé / nuit gelée) ---
    // Gelé : océan global GELÉ, œil d'eau libre au subsolaire (glint spéculaire),
    // anneau de slush au bord, nuages d'évaporation au-dessus de l'œil.
    push("Eyeball gele", tellurique(vec3(0.55, 0.62, 0.68), vec3(0.42, 0.5, 0.58), vec3(0.06, 0.32, 0.58), 0.96, 0.0, 0.0, 1.0, bleu).avec_nuages(0.25, blanc).avec_eyeball_zones(0.55, 0.0, 0.0));
    // Sec : subsolaire en lave/obsidienne, anneau de vie au terminateur, désert, puis glace.
    push("Eyeball sec", tellurique(vec3(0.72, 0.56, 0.34), vec3(0.5, 0.38, 0.24), z, 0.0, 1.0, 0.0, 1.0, sec).avec_dunes(0.4).avec_vegetation(vec3(0.32, 0.45, 0.2), 0.0).avec_eyeball_zones(-0.05, 1.0, 1.0));
    // Humide : continents et océans côté jour, anneau végétal au terminateur,
    // capuchon de tempête permanent au subsolaire, glace en nuit profonde seulement.
    push("Eyeball humide", tellurique(vec3(0.62, 0.5, 0.3), vec3(0.45, 0.36, 0.24), vec3(0.08, 0.38, 0.62), 0.55, 1.0, 0.0, 1.0, bleu).avec_vegetation(vec3(0.2, 0.5, 0.22), 0.6).avec_rivieres(0.35).avec_nuages(0.55, blanc).avec_eyeball_zones(-0.12, 0.0, 1.0));

    v
}

/// Catalogue des géantes gazeuses pour la galerie dédiée. Basé sur la classification
/// de Sudarsky (classes I–V par température/nuages) + géantes de glace + naines brunes.
pub fn catalogue_gazeuses() -> Vec<(String, Apparence)> {
    let mut v: Vec<(String, Apparence)> = Vec::new();
    // Deterministe, comme les telluriques ci-dessus.
    let mut push = |nom: &str, mut app: Apparence| {
        app.seed = graine_de_nom(nom);
        app.taille = classe_gazeuse(nom).rayon_pour(fraction_de_nom(nom));
        // **Une geante gazeuse n'a ni sol ni villes.**
        //
        // `Apparence::new` donne 1 a `villes` et 0,35 a `relief` : des valeurs
        // qui n'ont aucun sens ici. Elles sont **inertes aujourd'hui** -- le
        // shader garde les deux derriere `type_p < 0.5` -- mais c'est
        // exactement le genre de donnee fausse qui se reveille le jour ou l'on
        // touche au garde. On la corrige a la source.
        app.villes = 0.0;
        app.relief = 0.0;
        v.push((nom.to_string(), app));
    };
    let spot = vec3(0.6, -0.22, 0.77);

    // Géantes du système solaire
    push("Jupiter", gazeuse(vec3(0.9, 0.66, 0.4), vec3(0.74, 0.44, 0.26), vec3(0.99, 0.95, 0.86), 5.0, 1.9, vec3(0.85, 0.7, 0.5) * 0.3).avec_pole(vec3(0.6, 0.64, 0.7)).avec_jets(1.0).avec_tache(spot, 0.27, vec3(0.85, 0.34, 0.18)).avec_cyclones_pol().avec_tempetes(0.7));
    push("Saturne", gazeuse(vec3(0.9, 0.78, 0.5), vec3(0.7, 0.56, 0.34), vec3(0.97, 0.91, 0.68), 6.0, 0.9, vec3(0.88, 0.8, 0.55) * 0.3).avec_jets(0.9).avec_hexagone().avec_aurore(0.5, vec3(0.5, 0.7, 1.0)).avec_brume(0.22, vec3(0.95, 0.89, 0.68)).avec_pole(vec3(0.66, 0.63, 0.55)).avec_anneau_saturne(vec3(0.86, 0.79, 0.62)));
    push("Uranus", gazeuse(vec3(0.6, 0.82, 0.82), vec3(0.45, 0.7, 0.72), vec3(0.72, 0.9, 0.9), 3.0, 0.6, vec3(0.6, 0.85, 0.88) * 0.3).avec_jets(0.15).avec_zonal_flou(0.5).avec_axe(vec3(0.25, 0.2, 0.95)).avec_brume(0.45, vec3(0.62, 0.84, 0.86)).avec_pole(vec3(0.62, 0.84, 0.86)).avec_anneau_uranus(vec3(0.55, 0.72, 0.82)));
    push("Neptune", gazeuse(vec3(0.45, 0.56, 0.86), vec3(0.10, 0.18, 0.42), vec3(0.36, 0.5, 0.88), 4.0, 1.5, vec3(0.3, 0.45, 0.9) * 0.3).avec_jets(0.55).avec_zonal_flou(0.3).avec_tache_sombre(spot, 0.2, vec3(0.05, 0.07, 0.18)).avec_tempetes(0.9).avec_pole(vec3(0.3, 0.46, 0.74)).avec_anneau_neptune(vec3(0.42, 0.5, 0.68)));

    // Classification de Sudarsky (par température)
    push("Classe I (ammoniac)", gazeuse(vec3(0.86, 0.8, 0.66), vec3(0.62, 0.56, 0.44), vec3(0.95, 0.92, 0.82), 5.0, 1.4, vec3(0.85, 0.8, 0.6) * 0.3).avec_jets(0.5).avec_zonal_flou(0.25).avec_pole(vec3(0.62, 0.6, 0.55)));
    push("Classe II (eau, albedo haut)", gazeuse(vec3(0.85, 0.88, 0.93), vec3(0.68, 0.74, 0.82), vec3(0.97, 0.98, 1.0), 4.0, 1.0, vec3(0.8, 0.85, 0.95) * 0.35).avec_jets(0.45).avec_zonal_flou(0.35).avec_brume(0.25, vec3(0.9, 0.92, 0.97)).avec_pole(vec3(0.82, 0.86, 0.92)));
    push("Classe III (sans nuage, azur)", gazeuse(vec3(0.16, 0.32, 0.62), vec3(0.1, 0.2, 0.45), vec3(0.28, 0.46, 0.8), 2.0, 0.4, vec3(0.25, 0.45, 0.85) * 0.4).avec_jets(0.25).avec_zonal_flou(0.45).avec_pole(vec3(0.2, 0.36, 0.62)));
    push("Classe IV (alcalins, sombre)", gazeuse(vec3(0.26, 0.13, 0.11), vec3(0.12, 0.06, 0.06), vec3(0.42, 0.22, 0.16), 5.0, 1.8, vec3(0.3, 0.12, 0.08) * 0.25).avec_jets(0.9).avec_thermique(0.45, vec3(0.5, 0.08, 0.02)).avec_pole(vec3(0.18, 0.1, 0.09)));
    push("Classe V (silicates, chaud)", gazeuse(vec3(0.55, 0.28, 0.16), vec3(0.3, 0.14, 0.1), vec3(0.85, 0.55, 0.3), 6.0, 2.2, vec3(0.9, 0.5, 0.25) * 0.4).avec_jets(1.0).avec_thermique(0.7, vec3(0.9, 0.32, 0.06)).avec_pole(vec3(0.36, 0.18, 0.12)));

    // Variantes
    push("Jupiter chaud", gazeuse(vec3(0.6, 0.3, 0.2), vec3(0.32, 0.14, 0.12), vec3(0.85, 0.45, 0.25), 6.0, 2.4, vec3(0.9, 0.45, 0.2) * 0.4).avec_jets(1.0).avec_tache(spot, 0.2, vec3(0.9, 0.35, 0.15)).avec_thermique(0.5, vec3(0.78, 0.22, 0.05)).avec_tempetes(0.6).avec_pole(vec3(0.4, 0.2, 0.16)));
    push("Geante de methane", gazeuse(vec3(0.2, 0.55, 0.45), vec3(0.1, 0.35, 0.3), vec3(0.4, 0.78, 0.62), 4.0, 1.4, vec3(0.3, 0.7, 0.55) * 0.3).avec_jets(0.65).avec_pole(vec3(0.34, 0.5, 0.44)));
    push("Geante de soufre", gazeuse(vec3(0.8, 0.7, 0.2), vec3(0.55, 0.45, 0.12), vec3(0.95, 0.88, 0.4), 5.0, 1.8, vec3(0.85, 0.75, 0.25) * 0.3).avec_jets(0.7).avec_pole(vec3(0.55, 0.5, 0.34)));
    push("Naine brune", gazeuse(vec3(0.4, 0.15, 0.1), vec3(0.2, 0.08, 0.06), vec3(0.6, 0.25, 0.15), 7.0, 2.6, vec3(0.5, 0.15, 0.08) * 0.35).avec_jets(1.1).avec_thermique(0.85, vec3(0.6, 0.12, 0.03)).avec_tempetes(0.6).avec_pole(vec3(0.26, 0.1, 0.07)));
    push("Sub-Neptune", gazeuse(vec3(0.4, 0.5, 0.6), vec3(0.3, 0.4, 0.5), vec3(0.55, 0.65, 0.75), 3.0, 1.0, vec3(0.5, 0.6, 0.7) * 0.3).avec_zonal_flou(0.55).avec_brume(0.7, vec3(0.62, 0.68, 0.78)).avec_axe(vec3(0.2, 0.85, 0.3)).avec_pole(vec3(0.5, 0.56, 0.64)));

    // Nouveaux types
    push("Geante d'helium", gazeuse(vec3(0.86, 0.85, 0.82), vec3(0.7, 0.69, 0.66), vec3(0.97, 0.97, 0.95), 4.0, 0.7, vec3(0.85, 0.85, 0.82) * 0.3).avec_zonal_flou(0.4).avec_brume(0.35, vec3(0.93, 0.93, 0.9)).avec_pole(vec3(0.78, 0.78, 0.76)));
    push("Naine brune L (poussiereuse)", gazeuse(vec3(0.55, 0.22, 0.12), vec3(0.32, 0.12, 0.07), vec3(0.72, 0.34, 0.18), 8.0, 2.6, vec3(0.6, 0.2, 0.08) * 0.35).avec_jets(1.1).avec_thermique(0.8, vec3(0.7, 0.18, 0.04)).avec_tempetes(0.7).avec_pole(vec3(0.32, 0.14, 0.08)));
    push("Naine brune T (methane)", gazeuse(vec3(0.35, 0.2, 0.32), vec3(0.18, 0.1, 0.2), vec3(0.5, 0.3, 0.48), 7.0, 2.3, vec3(0.4, 0.2, 0.4) * 0.3).avec_jets(1.0).avec_thermique(0.45, vec3(0.55, 0.12, 0.25)).avec_tempetes(0.5).avec_pole(vec3(0.22, 0.13, 0.24)));
    push("Naine brune Y (froide)", gazeuse(vec3(0.16, 0.12, 0.18), vec3(0.07, 0.05, 0.1), vec3(0.26, 0.18, 0.3), 5.0, 1.8, vec3(0.18, 0.12, 0.22) * 0.25).avec_jets(0.7).avec_thermique(0.2, vec3(0.4, 0.1, 0.18)).avec_pole(vec3(0.12, 0.09, 0.15)));
    push("Neptune chaud", gazeuse(vec3(0.35, 0.46, 0.6), vec3(0.22, 0.32, 0.45), vec3(0.55, 0.66, 0.78), 4.0, 1.3, vec3(0.4, 0.55, 0.72) * 0.3).avec_jets(0.5).avec_zonal_flou(0.35).avec_brume(0.4, vec3(0.55, 0.66, 0.76)).avec_tempetes(0.4).avec_pole(vec3(0.34, 0.45, 0.58)));
    push("Geante de carbone", gazeuse(vec3(0.18, 0.17, 0.16), vec3(0.08, 0.08, 0.08), vec3(0.3, 0.28, 0.25), 5.0, 1.6, vec3(0.12, 0.1, 0.1) * 0.2).avec_jets(0.55).avec_zonal_flou(0.3).avec_pole(vec3(0.14, 0.13, 0.12)));
    push("Proto-geante chaude", gazeuse(vec3(0.7, 0.32, 0.16), vec3(0.45, 0.16, 0.08), vec3(0.95, 0.55, 0.25), 6.0, 2.8, vec3(1.0, 0.5, 0.2) * 0.45).avec_jets(1.0).avec_thermique(0.95, vec3(1.0, 0.4, 0.08)).avec_tempetes(0.8).avec_pole(vec3(0.5, 0.22, 0.12)));
    push("Geante rayee extreme", gazeuse(vec3(0.92, 0.6, 0.3), vec3(0.3, 0.14, 0.1), vec3(1.0, 0.95, 0.82), 9.0, 1.4, vec3(0.8, 0.6, 0.4) * 0.3).avec_jets(1.2).avec_tempetes(0.5).avec_pole(vec3(0.5, 0.45, 0.4)));
    // Tempête planétaire (§ 6 bis) : instantané d'une Grande Tache Blanche de
    // Saturne — tête convective blanche massive (slot 0 type 2) + activité de
    // vortex maximale le long des jets, turbulence dopée.
    push("Tempete planetaire (GTB)", gazeuse(vec3(0.9, 0.78, 0.5), vec3(0.66, 0.52, 0.32), vec3(0.97, 0.91, 0.7), 6.0, 2.2, vec3(0.88, 0.8, 0.55) * 0.3).avec_jets(1.0).avec_tache_blanche(vec3(0.55, 0.3, 0.78), 0.3).avec_tempetes(1.0).avec_pole(vec3(0.66, 0.63, 0.55)));

    // Géante emblématique
    push("Polyphemus (Avatar)", gazeuse(vec3(0.32, 0.6, 0.58), vec3(0.12, 0.34, 0.42), vec3(0.6, 0.84, 0.78), 5.0, 1.7, vec3(0.4, 0.7, 0.68) * 0.3).avec_jets(0.95).avec_tache(spot, 0.24, vec3(0.88, 0.34, 0.16)).avec_cyclones_pol().avec_tempetes(0.6).avec_pole(vec3(0.4, 0.56, 0.56)).avec_anneau_saturne(vec3(0.66, 0.78, 0.74)));

    // Anneaux : exemples de styles variés
    push("Geante annelee massive", gazeuse(vec3(0.86, 0.76, 0.54), vec3(0.62, 0.5, 0.34), vec3(0.95, 0.9, 0.72), 5.0, 1.0, vec3(0.85, 0.78, 0.55) * 0.3).avec_jets(0.85).avec_brume(0.2, vec3(0.94, 0.88, 0.68)).avec_pole(vec3(0.64, 0.62, 0.55)).avec_anneau_saturne(vec3(0.92, 0.86, 0.68)));
    push("Anneau monobande (type Uranus)", gazeuse(vec3(0.5, 0.78, 0.78), vec3(0.34, 0.6, 0.62), vec3(0.66, 0.88, 0.88), 3.0, 0.6, vec3(0.55, 0.82, 0.85) * 0.3).avec_jets(0.2).avec_zonal_flou(0.4).avec_brume(0.3, vec3(0.6, 0.82, 0.84)).avec_pole(vec3(0.55, 0.78, 0.8)).avec_anneau_uranus(vec3(0.55, 0.8, 0.97)));
    push("Anneau ceinture d'asteroides", gazeuse(vec3(0.55, 0.5, 0.42), vec3(0.36, 0.32, 0.26), vec3(0.72, 0.66, 0.56), 5.0, 1.4, vec3(0.6, 0.55, 0.45) * 0.3).avec_pole(vec3(0.42, 0.4, 0.36)).avec_anneau_ceinture(vec3(0.78, 0.74, 0.66)));
    push("Anneaux en arcs (type Neptune)", gazeuse(vec3(0.45, 0.56, 0.86), vec3(0.10, 0.18, 0.42), vec3(0.36, 0.5, 0.88), 4.0, 1.5, vec3(0.3, 0.45, 0.9) * 0.3).avec_jets(0.55).avec_zonal_flou(0.3).avec_tempetes(0.8).avec_pole(vec3(0.3, 0.46, 0.74)).avec_anneau_neptune(vec3(0.6, 0.66, 0.85)));
    push("Anneau de debris recent", gazeuse(vec3(0.62, 0.4, 0.3), vec3(0.38, 0.22, 0.16), vec3(0.82, 0.6, 0.45), 5.0, 1.8, vec3(0.7, 0.45, 0.3) * 0.3).avec_thermique(0.4, vec3(0.7, 0.25, 0.08)).avec_tempetes(0.6).avec_pole(vec3(0.4, 0.26, 0.2)).avec_anneau_debris(vec3(0.85, 0.7, 0.55)));

    v
}

/// Renvoie l'apparence d'un preset **tellurique** nommé du catalogue, pour la réutiliser
/// dans les presets scénarisés (source unique de vérité avec la galerie). La géographie
/// (`seed`) est tirée aléatoirement, comme en galerie. Panique si le nom est inconnu.
pub fn preset_tellurique(nom: &str) -> Apparence {
    catalogue_telluriques()
        .into_iter()
        .find(|(n, _)| n == nom)
        .unwrap_or_else(|| panic!("preset tellurique inconnu: {}", nom))
        .1
}

/// Idem pour une **géante gazeuse** nommée du catalogue.
pub fn preset_gazeuse(nom: &str) -> Apparence {
    catalogue_gazeuses()
        .into_iter()
        .find(|(n, _)| n == nom)
        .unwrap_or_else(|| panic!("preset gazeuse inconnu: {}", nom))
        .1
}

/// Rayon + apparence d'une planète aléatoire (vue « objet » isolée). Couvre les
/// trois types ; la plage de température inclut les mondes de lave.
pub fn planete_aleatoire() -> (f32, Apparence) {
    let t: f32 = gen_range(0.0, 1.0);
    let (rayon, _masse, app) = if t < 0.45 {
        apparence_tellurique(gen_range(120.0, 620.0))
    } else if t < 0.8 {
        apparence_gazeuse()
    } else {
        apparence_glacee()
    };
    (rayon, app)
}

#[cfg(test)]
mod tests_noms_presets {
    // ⚠️ Aucun test ne peut **bâtir** un système : `genese` tire ses aléas par
    // `macroquad::rand`, qui exige le contexte graphique
    // (`conception/interface.md` §5.1 bis). On ne peut donc pas vérifier à
    // l'exécution que Mercure s'appelle Mercure.
    //
    // Ce qu'on peut vérifier, c'est la **source** : que plus aucun preset ne
    // nomme une planète par un `sys.nommer` après coup. C'était la dette
    // D-INT-5 — tant que ce chemin existait, un preset pouvait l'oublier. Le
    // nom passe désormais par la signature d'`ajouter_planete`, donc par le
    // compilateur ; ce test garde la porte fermée.
    //
    // Lire le fichier source est un procédé qu'on ne se permet qu'ici, faute de
    // tout autre moyen, et **uniquement** pour interdire un motif — jamais pour
    // vérifier un comportement.
    const PRESETS: &str = include_str!("presets.rs");

    #[test]
    fn aucune_planete_de_preset_nest_nommee_apres_coup() {
        for (i, ligne) in PRESETS.lines().enumerate() {
            let l = ligne.trim();
            if !l.starts_with("sys.nommer(") {
                continue;
            }
            // Exceptions légitimes : les corps qui ne passent **pas** par
            // `ajouter_planete` — l'étoile hôte, Pandora (construite en ligne)
            // et les `EnginOrbital` (l'ISS, le Starship posté — pas des
            // planètes). Chacune se reconnaît à un nom de liaison explicite,
            // pour que la liste reste lisible plutôt que d'être une passoire.
            assert!(
                l.contains("soleil") || l.contains("idx_pandora") || l.contains("idx_iss") || l.contains("idx_starship"),
                "presets.rs:{} nomme une planète après coup : « {l} » — le nom                  doit entrer par `ajouter_planete(.., Some(\"…\"))`",
                i + 1
            );
        }
    }

    // Le corollaire : les presets **nomment vraiment** leurs planètes. Sans
    // cette moitié, supprimer tous les noms passerait le test ci-dessus.
    #[test]
    fn les_presets_nomment_bien_leurs_planetes() {
        // Plancher **généreux**, pas une cible : il y a 19 planètes nommées
        // aujourd'hui, et le seuil est là pour attraper une suppression en
        // masse, pas pour se déclencher à chaque preset qu'on retouche. Un
        // seuil calé sur le compte du jour serait une prophétie
        // auto-réalisatrice — il suivrait n'importe quelle valeur.
        let nommees = PRESETS.matches("Some(\"").count();
        assert!(nommees >= 15, "seulement {nommees} planètes nommées dans les presets");
        // Les lunes passent par un autre chemin : elles sont posées par nom,
        // et `ajouter_lune_reelle` exige déjà ce nom dans sa signature.
        //
        // Le test comptait les appels à `ajouter_lune_preset`, disparu avec le
        // passage aux orbites réelles. On vérifie maintenant la **chose
        // voulue** : chaque lune de la table est effectivement posée.
        for (nom, _, _, _) in super::presets::LUNES {
            assert!(
                PRESETS.contains(&format!("\"{nom}\"")),
                "{nom} est dans la table mais n'est posée nulle part"
            );
        }
        for attendu in ["Mercure", "Terre", "Jupiter", "Pluton", "Polyphemus", "Pandora"] {
            assert!(PRESETS.contains(attendu), "{attendu} a disparu des presets");
        }
    }
}

#[cfg(test)]
mod tests_catalogue {
    use super::*;

    // **Le catalogue est reproductible.** C'est la propriété qui manquait : il
    // posait une graine et une taille tirées au sort à chaque construction, si
    // bien que la « Terre » de la galerie était une planète différente à chaque
    // ouverture de l'écran — et que le système solaire, qui puise dans le même
    // catalogue, ne pouvait pas lui ressembler. Il n'y avait rien à quoi
    // ressembler.
    //
    // Ce test n'était pas écrivable avant : `gen_range` exige le contexte
    // graphique, donc le catalogue ne se construisait pas en test.
    #[test]
    fn le_catalogue_est_reproductible() {
        for (a, b) in [
            (catalogue_telluriques(), catalogue_telluriques()),
            (catalogue_gazeuses(), catalogue_gazeuses()),
        ] {
            assert_eq!(a.len(), b.len(), "le catalogue change de taille");
            assert!(!a.is_empty(), "catalogue vide");
            for ((na, aa), (nb, ab)) in a.iter().zip(b.iter()) {
                assert_eq!(na, nb, "l'ordre du catalogue change");
                assert_eq!(aa.seed, ab.seed, "{na} : la géographie change d'une lecture à l'autre");
                assert_eq!(aa.taille, ab.taille, "{na} : la taille change d'une lecture à l'autre");
            }
        }
    }

    // Deux entrées de même nom rendraient `preset_tellurique` ambigu : il prend
    // la première, l'autre serait inatteignable depuis le système solaire.
    #[test]
    fn aucun_nom_en_double_dans_le_catalogue() {
        for cat in [catalogue_telluriques(), catalogue_gazeuses()] {
            let mut vus = std::collections::HashSet::new();
            for (nom, _) in &cat {
                assert!(vus.insert(nom.clone()), "{nom} apparaît deux fois");
            }
        }
    }

    // La taille reste **dans les bornes de sa classe** : `rayon_pour` interpole,
    // il ne doit pas sortir de la plage que `rayon_aleatoire` couvrait.
    #[test]
    fn chaque_taille_reste_dans_les_bornes_de_sa_classe() {
        for (cat, classe) in [
            (catalogue_telluriques(), classe_tellurique as fn(&str) -> ClasseTaille),
            (catalogue_gazeuses(), classe_gazeuse as fn(&str) -> ClasseTaille),
        ] {
            for (nom, app) in &cat {
                let c = classe(nom);
                // Bornes prises à la SOURCE (`bornes_terrestres`), pas via
                // `rayon_pour` : les déduire de la fonction testée faisait bouger
                // l'attendu avec elle, et le test ne pouvait pas échouer.
                let (lo_t, hi_t) = c.bornes_terrestres();
                let (lo, hi) = (crate::genese::taille::rayon_visuel(lo_t), crate::genese::taille::rayon_visuel(hi_t));
                assert!(
                    app.taille >= lo - 1e-4 && app.taille <= hi + 1e-4,
                    "{nom} : taille {} hors bornes [{lo}, {hi}]",
                    app.taille
                );
            }
        }
    }

    // Les graines occupent vraiment la plage : un hachage qui les tasserait
    // donnerait des géographies toutes semblables.
    #[test]
    fn les_graines_du_catalogue_sont_dispersees() {
        let cat = catalogue_telluriques();
        let min = cat.iter().map(|(_, a)| a.seed).fold(f32::INFINITY, f32::min);
        let max = cat.iter().map(|(_, a)| a.seed).fold(f32::NEG_INFINITY, f32::max);
        assert!(min < 100.0 && max > 900.0, "graines tassées entre {min} et {max}");
        // Et deux presets ne partagent pas leur géographie.
        let mut vues = std::collections::HashSet::new();
        for (nom, a) in &cat {
            assert!(vues.insert(a.seed.to_bits()), "{nom} : géographie en double");
        }
    }

    // **Chaque preset demande par les systemes scenarises existe au catalogue.**
    //
    // C'etait garanti par le seul `panic!` de `preset_tellurique`, donc decouvert
    // au lancement, ecran noir a la cle. Le test n'etait pas ecrivable tant que le
    // catalogue appelait `gen_range` : il exige le contexte graphique.
    //
    // /!\ Les deux listes sont des **fixtures recopiees** de `presets.rs`. Un nom
    // ajoute la-haut sans l'etre ici est moins couvert, il ne devient pas faux.
    #[test]
    fn chaque_preset_demande_existe_au_catalogue() {
        let cat_t: std::collections::HashSet<String> =
            catalogue_telluriques().into_iter().map(|(n, _)| n).collect();
        for nom in DEMANDES_TELLURIQUES {
            assert!(cat_t.contains(*nom), "preset tellurique absent du catalogue : {nom}");
        }
        let cat_g: std::collections::HashSet<String> =
            catalogue_gazeuses().into_iter().map(|(n, _)| n).collect();
        for nom in DEMANDES_GAZEUSES {
            assert!(cat_g.contains(*nom), "preset gazeuse absent du catalogue : {nom}");
        }
    }

    const DEMANDES_TELLURIQUES: &[&str] = &[
        "Badlands",
        "Boule de neige",
        "Carbone",
        "Crevasse",
        "Cryovolcan",
        "Desert",
        "Eyeball gele",
        "Fer (Mercure)",
        "Ice Dunes",
        "Io (soufre)",
        "Lave",
        "Mars",
        "Lune",
        "Pandora",
        "Pics de glace",
        "Salines",
        "Subglaciaire",
        "Supraglacial",
        "Terre",
        "Titan",
        "Venus (etuve)",
    ];

    const DEMANDES_GAZEUSES: &[&str] = &[
        "Anneaux en arcs (type Neptune)",
        "Classe I (ammoniac)",
        "Classe II (eau, albedo haut)",
        "Jupiter",
        "Jupiter chaud",
        "Neptune",
        "Polyphemus (Avatar)",
        "Saturne",
        "Uranus",
    ];
}

#[cfg(test)]
mod tests_coherence {
    use super::*;

    /// Amplitude du bruit qui perturbe le seuil de banquise dans
    /// `planete.frag.glsl` : `±0,42/2` puis `±0,18/2`.
    const BRUIT_CALOTTE: f32 = 0.21 + 0.09;

    // **Pas de climat, pas de banquise.**
    //
    // `grad_lat` est le gradient de température par latitude : à 0, la planète
    // n'a aucun climat latitudinal (Mercure, la Lune, un monde de carbone). Une
    // calotte polaire y serait sans cause.
    //
    // Le seuil doit alors valoir exactement 1, le sentinelle « aucune » —
    // n'importe quelle valeur en dessous laisse le **bruit** (±0,30) franchir le
    // seuil près des pôles, où `froid` vaut déjà 1. C'est très exactement le bug
    // qui coiffait Mercure de glace.
    #[test]
    fn un_monde_sans_climat_na_pas_de_banquise() {
        for (nom, app) in catalogue_telluriques() {
            if app.grad_lat != 0.0 {
                continue;
            }
            assert!(
                app.calotte >= 1.0,
                "{nom} : grad_lat=0 (aucun climat) mais calotte={} — le bruit y \
                 fabriquera de la glace sans raison",
                app.calotte
            );
        }
    }

    // **Un seuil de banquise reste dans 0..1.** Au-delà de 1 il ne veut rien dire
    // de plus que « aucune » ; en dessous de 0, la planète serait intégralement
    // gelée sans que le preset le dise.
    #[test]
    fn les_seuils_de_banquise_sont_dans_leur_plage() {
        for (nom, app) in catalogue_telluriques() {
            assert!(
                (0.0..=1.0).contains(&app.calotte),
                "{nom} : calotte={} hors de 0..1",
                app.calotte
            );
            assert!(
                (0.0..=1.0).contains(&app.grad_lat),
                "{nom} : grad_lat={} hors de 0..1",
                app.grad_lat
            );
        }
    }

    // **Un monde déclaré gelé l'est vraiment.** Un preset dont le nom annonce la
    // glace mais dont le seuil est si haut que le bruit ne l'atteint jamais
    // serait un preset qui ment : il rendrait une planète nue.
    #[test]
    fn un_monde_declare_gele_porte_vraiment_de_la_glace() {
        let glaces = [
            "Boule de neige", "Pics de glace", "Subglaciaire", "Ice Dunes",
            "Supraglacial", "Arctique", "Glacial", "Snow", "Toundra",
        ];
        let cat = catalogue_telluriques();
        for g in glaces {
            let (_, app) = cat
                .iter()
                .find(|(n, _)| n == g)
                .unwrap_or_else(|| panic!("preset {g} absent du catalogue"));
            // Il faut que le seuil soit franchissable : sinon aucune glace.
            assert!(
                app.calotte < 1.0 + BRUIT_CALOTTE,
                "{g} : calotte={} — annoncé gelé, mais aucune glace n'apparaîtra",
                app.calotte
            );
            // Et un monde gelé a forcément un climat latitudinal.
            assert!(app.grad_lat > 0.0, "{g} : gelé mais grad_lat=0");
        }
    }
}

#[cfg(test)]
mod tests_coherence_2 {
    use super::*;

    // **Pas d'air ni d'eau, pas de villes.** Le shader n'excluait que la lave et
    // les mondes voilés : tous les cailloux morts — Mercure, la Lune, Carbone —
    // portaient des lumières de ville côté nuit, `villes` valant 1 par défaut.
    #[test]
    fn un_monde_sans_air_ni_eau_na_pas_de_villes() {
        for (nom, app) in catalogue_telluriques() {
            if app.atmo_densite <= 0.0 && app.eau <= 0.0 {
                assert_eq!(app.villes, 0.0, "{nom} : ni air ni eau, mais des villes");
            }
        }
    }

    // Et l'inverse : **un monde habitable garde ses villes**. Une règle trop
    // large les éteindrait partout, ce qui passerait le test ci-dessus sans que
    // personne ne le voie.
    #[test]
    fn un_monde_habitable_garde_ses_villes() {
        let cat = catalogue_telluriques();
        let habitables: Vec<_> = cat
            .iter()
            .filter(|(_, a)| a.atmo_densite > 0.0 && a.eau > 0.0)
            .collect();
        assert!(habitables.len() > 20, "trop peu de mondes habitables : {}", habitables.len());
        for (nom, app) in habitables {
            assert!(app.villes > 0.0, "{nom} : habitable mais sans villes");
        }
    }

    // **Une rivière coule d'eau ou de lave, jamais de rien.**
    //
    // Première version : « pas de rivières sans eau ». Elle a mordu sur
    // `Crevasse` — à tort. Ce preset a bien `eau = 0`, mais ses rivières sont
    // des **coulées de lave** (`riv_lave`, chaleur géothermique au fond des
    // crevasses) : elles n'ont pas besoin d'océan. Le test disait autre chose que
    // ce que son nom annonçait.
    #[test]
    fn une_riviere_coule_deau_ou_de_lave() {
        let mut vues_lave = 0;
        for (nom, app) in catalogue_telluriques() {
            if app.rivieres > 0.0 && app.eau <= 0.0 {
                assert!(
                    app.riv_lave > 0.0,
                    "{nom} : des rivières sans eau et sans lave — elles couleraient sur la roche nue"
                );
                vues_lave += 1;
            }
            // Les récifs, eux, sont bel et bien marins.
            if app.eau <= 0.0 {
                assert_eq!(app.recifs, 0.0, "{nom} : des récifs sans eau");
            }
        }
        // Et le cas « rivières de lave » existe vraiment : sans ça, la branche
        // ci-dessus ne serait jamais parcourue et le test ne prouverait rien.
        assert!(vues_lave > 0, "aucune rivière de lave au catalogue : la règle n'est pas exercée");
    }
}

#[cfg(test)]
mod tests_coherence_3 {
    use super::*;

    // **Une géante gazeuse n'a ni sol, ni eau, ni villes.**
    //
    // Ces champs existent sur `Apparence`, partagée par les deux types, et
    // `Apparence::new` leur donne des valeurs qui ne valent que pour une
    // tellurique (`villes = 1`, `relief = 0,35`). Le shader les ignore
    // aujourd'hui — tout est gardé derrière `type_p < 0.5` — mais une donnée
    // fausse qui ne se voit pas est une donnée fausse qui attend son heure.
    #[test]
    fn une_geante_gazeuse_na_ni_sol_ni_villes() {
        for (nom, a) in catalogue_gazeuses() {
            assert_eq!(a.villes, 0.0, "{nom} : des villes sur une géante gazeuse");
            assert_eq!(a.relief, 0.0, "{nom} : du relief sur une géante gazeuse");
            assert_eq!(a.eau, 0.0, "{nom} : de l'eau de surface sur une géante");
            assert_eq!(a.veg_couv, 0.0, "{nom} : de la végétation sur une géante");
            assert_eq!(a.crateres, 0.0, "{nom} : des cratères sur une géante");
            assert_eq!(a.rivieres, 0.0, "{nom} : des rivières sur une géante");
            assert!(a.calotte >= 1.0, "{nom} : une banquise sur une géante");
        }
    }

    // **Mars est rousse, et contrastée.** Elle empruntait « Badlands », dont la
    // rouille est trop sourde : à l'écran elle ne se lisait pas comme Mars.
    //
    // Le test tient deux choses. La **teinte** — le rouge domine franchement le
    // bleu, sinon on retombe sur un gris-brun. Et le **contraste** entre régions
    // claires (oxyde de fer) et sombres (basalte) : c'est lui qui fait lire la
    // planète, et une rouille uniforme paraît terne, surtout après quantification.
    #[test]
    fn mars_est_rousse_et_contrastee() {
        let cat = catalogue_telluriques();
        let (_, m) = cat.iter().find(|(n, _)| n == "Mars").expect("preset Mars absent");
        assert!(m.couleur.x > m.couleur.z * 2.5, "Mars n'est pas rousse : {:?}", m.couleur);
        assert!(m.couleur.x > m.couleur.y * 1.5, "Mars tire trop vers le jaune : {:?}", m.couleur);
        // Contraste clair/sombre entre les deux teintes de sol.
        let clair = m.couleur.x + m.couleur.y + m.couleur.z;
        let sombre = m.couleur2.x + m.couleur2.y + m.couleur2.z;
        assert!(clair > sombre * 1.4, "regions claires et sombres trop proches : {clair} vs {sombre}");
        // Mars a des calottes : un climat, et un seuil franchissable.
        assert!(m.grad_lat > 0.0 && m.calotte < 1.0, "Mars sans calotte polaire");
        // Et pas d'eau libre.
        assert_eq!(m.eau, 0.0, "Mars avec de l'eau de surface");
    }

    // **Mars a une brume, et elle doit se voir.** Depuis que l'atmosphère est une
    // coquille rendue au-delà de la silhouette (`shaders/atmosphere.frag.glsl`),
    // `atmo` ne règle plus l'éclat d'un liseré : il règle la **densité** de l'air.
    // Mars valait 0,10 sous l'ancien sens — moins que le monde sec générique — et
    // sortait sans atmosphère visible.
    //
    // Le test se garde d'écrire la formule de densité : elle vit dans le GLSL, et
    // la recopier ici ne prouverait que la recopie. Il **encadre** Mars entre deux
    // autres entrées du catalogue, écrites indépendamment d'elle.
    #[test]
    fn mars_a_une_brume_visible_mais_tenue() {
        let cat = catalogue_telluriques();
        let app = |nom: &str| {
            cat.iter()
                .find(|(n, _)| n == nom)
                .unwrap_or_else(|| panic!("preset absent : {nom}"))
                .1
        };
        let (mars, terre, sec) = (app("Mars"), app("Terre"), app("Badlands"));

        // Plus dense que le caillou sec générique, sinon la brume ne se lit pas.
        assert!(
            mars.atmo_densite > sec.atmo_densite * 1.5,
            "Mars pas plus embrumée qu'un monde sec quelconque : {} vs {}",
            mars.atmo_densite,
            sec.atmo_densite
        );
        // Mais nettement en deçà de la Terre : c'est une brume, pas un ciel.
        assert!(
            mars.atmo_densite < terre.atmo_densite * 0.6,
            "Mars aussi dense que la Terre : {} vs {}",
            mars.atmo_densite,
            terre.atmo_densite
        );
        // Et la brume est rousse : la poussière la teinte, elle ne diffuse pas bleu.
        let t = mars.atmo;
        assert!(t.x > t.z * 1.5, "brume martienne pas assez chaude : {t:?}");
    }

    /// **La teinte de l'air est unitaire, partout.**
    ///
    /// Le shader la consomme telle quelle (`mix(atmo, voile_couleur, …)`) là où
    /// il normalisait auparavant. Une entrée qui écrirait encore `teinte × force`
    /// dans `atmo` peindrait un halo sombre — et rien ne le signalerait, puisque
    /// la densité, elle, serait juste.
    #[test]
    fn la_teinte_de_lair_ne_porte_plus_la_densite() {
        let mut avec_air = 0;
        for (nom, a) in catalogue_telluriques().into_iter().chain(catalogue_gazeuses()) {
            if a.atmo_densite <= 0.0 {
                // Pas d'air -> pas de teinte résiduelle : « rien » ne doit avoir
                // qu'une seule écriture, sinon les gardes ci-dessus divergent.
                assert_eq!(a.atmo, Vec3::ZERO, "{nom} : densité nulle mais teinte non nulle");
                continue;
            }
            avec_air += 1;
            assert!(
                (a.atmo.length() - 1.0).abs() < 1e-3,
                "{nom} : teinte d'air non unitaire ({}) — la densité y est encore mêlée",
                a.atmo.length()
            );
        }
        assert!(avec_air > 30, "seulement {avec_air} mondes avec air : règle peu exercée");
    }

    /// **La scission ne change rien aux mondes qu'on n'a pas touchés.**
    ///
    /// La grande majorité du catalogue écrit encore `teinte × intensité` via
    /// `avec_atmo`. La migration n'est honnête que si ces entrées-là ressortent
    /// au bit près : teinte = direction, densité = longueur. Sans cette garantie,
    /// séparer les deux champs aurait discrètement redessiné cent planètes.
    ///
    /// ⚠️ Une première version de ce test comparait la LUMINANCE des teintes
    /// pour montrer qu'un air peut être « dense et terne ». Elle était fausse
    /// dans sa prémisse : une teinte ramenée à la norme 1 voit sa luminance
    /// dictée par sa TEINTE (le bleu pèse 0,07 en Rec. 709, le jaune 0,93), pas
    /// par son épaisseur. Elle mesurait la couleur en croyant mesurer l'air.
    #[test]
    fn lecriture_historique_de_latmosphere_est_preservee() {
        for v in [
            vec3(0.35, 0.55, 1.0) * 0.9, // bleu océanique
            vec3(0.6, 0.8, 1.0) * 0.2,   // voile glacé
            vec3(0.8, 0.7, 0.5) * 0.12,  // atmosphère sèche
            vec3(1.0, 0.5, 0.2) * 0.35,  // océan de magma
        ] {
            let a = Apparence::new(TypePlanete::Tellurique, Vec3::ONE, Vec3::ONE, Vec3::ZERO, 0.0)
                .avec_atmo(v);
            assert!(
                (a.atmo_densite - v.length()).abs() < 1e-6,
                "densité {} ≠ ancienne longueur {}",
                a.atmo_densite,
                v.length()
            );
            // teinte × densité doit reconstituer le vecteur d'origine.
            assert!(
                (a.atmo * a.atmo_densite - v).length() < 1e-5,
                "teinte × densité ne redonne pas {v:?}"
            );
        }
        // Et le cas nul reste nul dans les DEUX champs.
        let nu = Apparence::new(TypePlanete::Tellurique, Vec3::ONE, Vec3::ONE, Vec3::ZERO, 0.0)
            .avec_atmo(Vec3::ZERO);
        assert_eq!(nu.atmo_densite, 0.0);
        assert_eq!(nu.atmo, Vec3::ZERO);
    }

    /// Et le corollaire : **aucun monde nommé pour son atmosphère ne reste nu.**
    /// Ash porte des cendres en suspension, ce qui EST une atmosphère ; elle
    /// était notée à zéro parce que sa brume est grise et sombre.
    #[test]
    fn les_mondes_a_atmosphere_epaisse_le_sont_vraiment() {
        let cat = catalogue_telluriques();
        let densite = |nom: &str| {
            cat.iter()
                .find(|(n, _)| n == nom)
                .unwrap_or_else(|| panic!("preset absent : {nom}"))
                .1
                .atmo_densite
        };
        let terre = densite("Terre");
        // Trois mondes dont l'épaisseur est la caractéristique première.
        for nom in ["Venus (etuve)", "Titan", "Hycean"] {
            assert!(
                densite(nom) > terre,
                "{nom} : {} — annoncé plus épais que la Terre ({terre}), noté plus fin",
                densite(nom)
            );
        }
        // Ash : une brume ténue, mais pas rien.
        let ash = densite("Ash (volcanique)");
        assert!(ash > 0.0 && ash < terre, "Ash : {ash} — ni cendres en suspension, ni ciel");
    }

    // Et le catalogue gazeux n'est pas vide : sans ça, la boucle ci-dessus ne
    // parcourrait rien et le test passerait sans rien vérifier.
    #[test]
    fn le_catalogue_gazeux_est_peuple() {
        assert!(
            catalogue_gazeuses().len() > 20,
            "seulement {} géantes au catalogue",
            catalogue_gazeuses().len()
        );
    }

    // **Ce qui pousse a besoin d'eau, ce qui flotte a besoin d'air.** Deux
    // incohérences visibles à l'écran : de la végétation sur un monde sec, des
    // nuages sur un monde sans atmosphère.
    /// **La règle « pas de nuages sans air » vaut aussi pour les mondes TIRÉS.**
    ///
    /// L'invariant du catalogue ne couvre que les presets écrits à la main. Or la
    /// skymap ne les utilise pas : elle tire ses planètes avec
    /// `apparence_tellurique`. Un caillou sans air qui en sortirait nuageux ne
    /// serait attrapé par rien — et le shader, lui, refuse désormais de les
    /// peindre, si bien que le défaut serait devenu **invisible** au lieu d'être
    /// corrigé. On vérifie donc la source, pas seulement le rendu.
    #[test]
    fn les_mondes_tires_n_ont_pas_de_nuages_sans_air() {
        for graine in [1u64, 7, 42, 1234, 99999] {
            srand(graine);
            // Tout le spectre de températures d'équilibre, du brûlant au gelé.
            for i in 0..=40 {
                let t = i as f32 / 40.0;
                let (_, _, a) = crate::genese::apparences::apparence_tellurique(t);
                if a.nuages > 0.0 {
                    assert!(
                        a.atmo_densite > 0.0 || a.voile > 0.0,
                        "graine {graine}, t={t:.2} : des nuages sur un monde sans air"
                    );
                }
            }
        }
    }

    /// **Les mondes glacés TIRÉS ne partagent pas une seule géographie.**
    ///
    /// Même défaut que ci-dessus, même remède : l'invariant du catalogue
    /// (`les_graines_du_catalogue_sont_dispersees`) ne couvre que les presets
    /// écrits à la main. `apparence_glacee` a longtemps oublié de poser
    /// `app.seed` (contrairement à `apparence_tellurique` et `apparence_gazeuse`,
    /// ses voisines), si bien que toutes les lunes et planètes glacées tirées
    /// partageaient le défaut `seed: 0.0` — donc la même géographie.
    #[test]
    fn les_mondes_glaces_tires_n_ont_pas_une_seule_geographie() {
        for graine in [1u64, 7, 42, 1234, 99999] {
            srand(graine);
            let mut vues = std::collections::HashSet::new();
            for _ in 0..20 {
                let (_, _, a) = apparence_glacee();
                vues.insert(a.seed.to_bits());
            }
            assert!(
                vues.len() > 1,
                "graine {graine} : 20 tirages glacés partagent tous la même géographie ({vues:?})"
            );
        }
    }

    // ⚠️ Deuxième fois que la règle candidate était **trop stricte** et que le
    // catalogue avait raison. « Pas de végétation sans eau » mordait sur
    // `Lichen` : un lichen pousse sur la roche nue et prend son humidité dans
    // l'**air**, pas dans un océan. Comme les rivières de lave de `Crevasse`,
    // c'est un cas limite délibéré, pas un oubli.
    //
    // La règle juste : il faut de l'eau **ou** une atmosphère — mais pas rien.
    #[test]
    fn la_vie_et_les_nuages_ont_leurs_conditions() {
        let mut vies_aeriennes = 0;
        for (nom, a) in catalogue_telluriques() {
            if a.veg_couv > 0.0 {
                assert!(
                    a.eau > 0.0 || a.atmo_densite > 0.0,
                    "{nom} : de la végétation sans eau NI atmosphère"
                );
                if a.eau <= 0.0 {
                    vies_aeriennes += 1;
                }
            }
            if a.nuages > 0.0 {
                assert!(a.atmo_densite > 0.0, "{nom} : des nuages sans atmosphère");
            }
            if a.voile > 0.0 {
                assert!(a.atmo_densite > 0.0, "{nom} : un voile sans atmosphère");
            }
        }
        // Le cas « vie sans océan » existe vraiment : sans lui, la branche
        // ci-dessus ne serait jamais parcourue et le test ne prouverait rien.
        assert!(vies_aeriennes > 0, "aucune flore aérienne : la règle n'est pas exercée");
    }
}

#[cfg(test)]
mod tests_masse_luminosite {
    use super::*;

    // **La relation masse-luminosité, seule (pas de contexte graphique).**
    //
    // Monotone (une étoile plus lumineuse pèse plus lourd), bornée comme
    // aujourd'hui (0,25x .. 5x `MASSE_ETOILE`), et surtout **exacte à L=1** :
    // c'est ce dernier point qui garde le Soleil du preset solaire à sa masse
    // d'aujourd'hui, donc ses périodes orbitales inchangées.
    #[test]
    fn masse_luminosite_est_monotone_bornee_et_exacte_a_un() {
        // Exacte à L=1 : la source de retro-compatibilité du lot.
        assert_eq!(
            masse_luminosite(1.0),
            MASSE_ETOILE,
            "masse_luminosite(1.0) doit valoir EXACTEMENT MASSE_ETOILE, sinon le \
             Soleil du preset solaire change de masse et ses périodes avec"
        );

        // Monotone : une étoile plus lumineuse est plus massive.
        let faible = masse_luminosite(0.2);
        let moyenne = masse_luminosite(1.0);
        let forte = masse_luminosite(3.0);
        assert!(faible < moyenne, "{faible} devrait etre < {moyenne} (L=0.2 < L=1)");
        assert!(moyenne < forte, "{moyenne} devrait etre < {forte} (L=1 < L=3)");

        // Bornée : une luminosité extrême (naine très pâle, supergéante) ne
        // sort jamais de [0,25x ; 5x] MASSE_ETOILE.
        let minuscule = masse_luminosite(0.0000001);
        let enorme = masse_luminosite(1_000_000.0);
        assert!(
            minuscule >= 0.25 * MASSE_ETOILE - 0.01,
            "masse_luminosite(1e-7) = {minuscule} sous le plancher 0,25x"
        );
        assert!(
            enorme <= 5.0 * MASSE_ETOILE + 0.01,
            "masse_luminosite(1e6) = {enorme} au-dessus du plafond 5x"
        );
    }

    // **Une planète orbite la masse de son étoile hôte, pas un chiffre fixe.**
    //
    // À la même distance, l'orbite autour d'une étoile peu lumineuse doit être
    // plus LENTE (mouvement moyen `n` plus faible) qu'autour d'une étoile
    // brillante -- Kepler 3 : n = sqrt(G·M / a^3), et M dépend de la masse, pas
    // du hasard. `Orbite` est testée directement, sans `Systeme` ni contexte
    // graphique (patron `CorpsEssai` de `systeme::tests`, inapplicable ici car
    // il ne porte pas de masse stellaire).
    #[test]
    fn une_planete_orbite_plus_vite_autour_dune_etoile_plus_massive() {
        let a = 1.0; // même demi-grand axe pour les deux cas
        let e = 0.0;
        let a1 = macroquad::prelude::Vec3::X;
        let q = macroquad::prelude::Vec3::Z;

        let masse_terne = masse_luminosite(0.1); // naine peu lumineuse
        let masse_brillante = masse_luminosite(4.0); // étoile bien plus brillante
        assert!(masse_terne < masse_brillante, "les deux masses doivent différer pour que le test soit probant");

        let mu_terne = crate::systeme::G * masse_terne;
        let mu_brillante = crate::systeme::G * masse_brillante;
        let orb_terne = crate::orbite::Orbite::new(a, e, a1, q, mu_terne, 0.0);
        let orb_brillante = crate::orbite::Orbite::new(a, e, a1, q, mu_brillante, 0.0);

        assert!(
            orb_terne.n < orb_brillante.n,
            "mouvement moyen autour de l'étoile terne ({}) >= celui de l'étoile \
             brillante ({}) : la masse de l'hôte n'est plus prise en compte",
            orb_terne.n, orb_brillante.n
        );
    }

    // ⚠️ Tenté ici et abandonné : faire passer ce test par `ajouter_planete`
    // (plutôt que par `Orbite` directement, comme ci-dessus) pour emprunter le
    // vrai chemin de code. Ça ne compile/tourne pas en test : `ajouter_planete`
    // -> `ajouter_planete_autour` -> `Planete::new` construit un `Material`
    // (`mat_corps`, `mat_atmo`), qui exige le contexte graphique -- panique
    // (`THREAD_ID.is_some()`) hors de `macroquad::main`. C'est la même limite
    // que celle déjà documentée pour `Soleil::new` (`soleil::tests_graine`) :
    // hors de portée d'un test, pas un oubli.

    // **Un plan circumbinaire (type P) pèse la SOMME des deux étoiles, pas
    // une seule.**
    //
    // `plan_p` combine deux masses -- chacune issue de `masse_luminosite` via
    // `feuille_alea` -- en une seule `masse_centrale`. Une planète type P
    // orbite le BARYCENTRE du couple : lui faire perdre la somme (revenir à
    // `m1` seul, par exemple) sous-pèserait silencieusement l'hôte, puisque
    // rien d'autre ne reconstruit cette masse pour comparer.
    #[test]
    fn plan_p_pese_la_somme_des_deux_etoiles() {
        let m1 = masse_luminosite(0.3);
        let m2 = masse_luminosite(2.0);
        assert!(m1 != m2, "les deux masses doivent différer pour que la somme soit probante");

        let plan = plan_p(10.0, m1, m2, 0.3, 2.0, 0.1);

        assert_eq!(
            plan.masse_centrale,
            m1 + m2,
            "masse_centrale = {} au lieu de m1+m2 = {}",
            plan.masse_centrale, m1 + m2
        );
        // Et elle dépasse chacune des deux prise seule -- sinon la somme se
        // serait réduite à une seule étoile sans que l'égalité ci-dessus le
        // capture (une addition qui oublierait m2 resterait \"juste\" à ses
        // propres yeux si le test ne comparait qu'à lui-même).
        assert!(plan.masse_centrale > m1.max(m2), "masse_centrale ne dépasse pas la plus lourde des deux étoiles");
    }

    // **Bornes de `masse_luminosite` à une luminosité nulle.**
    //
    // `ProfilEtoile::aleatoire` ne tire jamais L=0 (toutes ses branches posent
    // une luminosité strictement positive), mais `masse_luminosite` est
    // `pub(crate)` et sa signature n'exclut pas ce cas : `0.0.powf(0.286)`
    // vaut exactement `0.0`, sous le plancher -- le `clamp` doit donc y
    // ramener la masse, et non produire zéro ou un nombre négatif qui
    // enverrait une étoile de masse nulle à la dérive.
    #[test]
    fn masse_luminosite_de_zero_retombe_sur_le_plancher() {
        assert_eq!(
            masse_luminosite(0.0),
            0.25 * MASSE_ETOILE,
            "luminosité nulle : devrait retomber exactement sur le plancher 0,25x MASSE_ETOILE"
        );
    }

    /// **Une luminosité négative ne doit pas rendre NaN.**
    ///
    /// `powf` d'un négatif à un exposant fractionnaire rend NaN, et **NaN
    /// traverse `clamp`** : ses comparaisons sont fausses des deux côtés,
    /// donc la borne ne le retient pas. Une masse NaN se propagerait
    /// silencieusement aux barycentres (`stellaire.rs`) et aux périodes de
    /// tout le système — un astre au mauvais endroit, sans le moindre
    /// message.
    ///
    /// Le cas est inatteignable aujourd'hui (aucune branche de
    /// `ProfilEtoile::aleatoire` ni aucun preset ne produit ≤ 0), mais les
    /// lots trou noir et quasi-étoile ouvriront ce chemin.
    #[test]
    fn une_luminosite_negative_ne_rend_pas_nan() {
        for l in [-1.0_f32, -0.001, f32::MIN] {
            let m = masse_luminosite(l);
            assert!(m.is_finite(), "luminosité {l} -> masse {m}");
            assert_eq!(m, 0.25 * MASSE_ETOILE, "luminosité {l} : attendu le plancher");
        }
    }

    /// **La masse d'un trou noir ne se déduit pas de sa luminosité.**
    ///
    /// Le peu qu'il rayonne vient de son disque d'accrétion, pas d'une
    /// photosphère : passé à `masse_luminosite`, il se coucherait au
    /// **plancher**, soit l'objet le plus lourd du système pesant un quart de
    /// masse stellaire — et des barycentres qui tournent à l'envers.
    #[test]
    fn un_trou_noir_pese_plus_que_ce_qu_il_rayonne() {
        // La luminosité du gabarit tiré (`etoile::particuliere`).
        let par_la_lumiere = masse_luminosite(0.4);
        assert!(
            MASSE_TROU_NOIR > par_la_lumiere * 5.0,
            "un trou noir ({MASSE_TROU_NOIR}) ne pèse guère plus qu'une étoile pâle ({par_la_lumiere})"
        );
        // Il dépasse délibérément le plafond des étoiles : celui-ci borne les
        // naines pâles et les supergéantes, il n'a rien à dire ici.
        const { assert!(MASSE_TROU_NOIR > 5.0 * MASSE_ETOILE) }; // plafond stellaire non applique
        // Sans tomber dans l'absurde : un trou noir stellaire, pas un trou
        // noir supermassif — celui-là relève du lot quasi-étoile.
        const { assert!(MASSE_TROU_NOIR < 100.0 * MASSE_ETOILE) };
    }
}

#[cfg(test)]
mod tests_variante {
    use super::*;

    /// **Chaque mode de couronne trouve sa variante.**
    ///
    /// ⚠️ Un mode sans bras retombe sur `Normale` **en silence** : l'étoile
    /// perd ses jets ou son horizon, son rendu existe toujours, et rien ne le
    /// dit. C'est précisément ce qui tenait le trou noir hors du jeu — le
    /// mode 5 était produit par personne et attendu par personne.
    #[test]
    fn chaque_mode_de_couronne_trouve_sa_variante() {
        let attendus = [
            (0.0, Variante::Normale),
            (1.0, Variante::Jets),
            (2.0, Variante::Vent),
            (3.0, Variante::Pulsar),
            (4.0, Variante::Magnetar),
            (5.0, Variante::TrouNoir),
        ];
        for (mode, attendu) in attendus {
            assert!(
                std::mem::discriminant(&Variante::de_mode(mode)) == std::mem::discriminant(&attendu),
                "le mode {mode} ne donne pas la variante attendue"
            );
        }
    }

    /// **Tous les modes du tirage sont couverts.** Le lien se fait par un
    /// `f32` que rien ne contraint : si `etoile::particuliere` invente un mode
    /// 6, il devient `Normale` sans un mot.
    #[test]
    fn le_tirage_ne_produit_aucun_mode_orphelin() {
        for k in 0..1000 {
            let (_, _, _, _, nom, mode) = crate::etoile::particuliere(k as f32 / 1000.0);
            let orphelin = matches!(Variante::de_mode(mode), Variante::Normale) && mode != 0.0;
            assert!(!orphelin, "{nom} porte le mode {mode}, que personne ne sait traduire");
        }
    }
}
