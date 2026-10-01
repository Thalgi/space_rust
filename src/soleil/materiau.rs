use crate::impostor;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::cell::RefCell;

// Materials partagés : un seul pipeline chacun, clonés à chaque soleil.
/// **Les deux passes de l'étoile** : `(zone, écrit la profondeur)`.
///
/// Le disque de l'étoile (`zone` 0) est opaque : il doit occulter ce qui passe
/// derrière, sinon un corps caché par l'étoile se dessinerait par-dessus (la vue
/// système ne trie pas ses astres). La couronne (`zone` 1) est translucide : elle
/// ne doit RIEN occulter. Son quad est plat, face caméra, et s'étend jusqu'à 2,1
/// rayons ; en écrivant la profondeur elle posait un plan qui rejetait tout ce
/// qui passait derrière — d'où la barre sombre en travers des étoiles des disques
/// protosolaires, là où ce plan coupait le voile.
///
/// GLSL ES 100 ne sait pas couper l'écriture de profondeur par fragment : il faut
/// deux pipelines sur le même shader, que `zone` aiguille (`soleil.frag.glsl`).
///
/// **Source unique** : le nombre de materials, l'ordre de dessin (`rendu.rs`) et
/// le test d'invariant lisent tous cette table.
pub(super) const PASSES: [(f32, bool); 2] = [(0.0, true), (1.0, false)];

/// **Étendue du quad de lentille**, en rayons de l'ombre.
///
/// ⚠️ **Mesuré, pas choisi** (`conception/astres_v2_lentille.md` §2). La
/// déviation décroît en `1/b` : elle vaut encore **13°** à 3,4 rayons — où
/// s'arrêtait le quad de la première version — et ne tombe sous un pixel qu'au
/// delà de **1400**. Aucun quad ne peut donc la contenir entièrement.
///
/// Mais sur un champ d'étoiles sans structure, un décalage **rigide** est
/// indétectable : rien ne dit qu'une étoile n'est pas à sa place. Ce qui se voit
/// est l'**étirement**, c'est-à-dire le gradient — et lui passe sous le pixel
/// vers `b ≈ 6`. Douze rayons laissent donc toute la distorsion visible à
/// l'intérieur, avec de la marge.
pub(super) const ETENDUE_LENTILLE: f32 = 12.0;

/// Rayon de Schwarzschild **en rayons d'ombre**. Le paramètre d'impact critique
/// vaut `b_c = 3√3/2 · r_s = 2,598 r_s`, et le chantier prend déjà l'ombre pour
/// unité depuis la recherche sur le disque d'accrétion : **`b_c = 1`**, donc
/// `r_s = 1/2,598`. Les deux constantes qui suivent en découlent — aucune n'est
/// réglée à l'œil.
const R_S: f32 = 1.0 / 2.598_076;

/// Numérateur de la déviation en **champ faible**, `α = 2 r_s / b`.
pub(super) const DEVIATION_FAIBLE: f32 = 2.0 * R_S;

/// Terme constant de la **déviation forte** de Darwin : `−π + ln[216(7 − 4√3)]`,
/// soit `−π + 2,7414`.
pub(super) const DEVIATION_FORTE: f32 = 2.741_4 - std::f32::consts::PI;

/// **Déviation d'un rayon lumineux**, en radians, pour un paramètre d'impact `b`
/// exprimé **en rayons de l'ombre**.
///
/// Deux régimes, et c'est la littérature qui les donne
/// (`conception/astres_v2_lentille.md` §1) :
///
/// - **champ faible** `α = 2 r_s / b`, exact au loin ;
/// - **déviation forte** (Darwin, précisée par Bozza)
///   `α = −π + ln[216(7 − 4√3) / (1 − b_c/b)]`, qui **diverge** au bord de
///   l'ombre.
///
/// ⚠️ C'est cette divergence qui fait l'image : l'anneau d'Einstein et
/// l'empilement d'images en sortent tout seuls. La **lentille mince** que
/// portait la première version — `β = θ − θ_E²/θ` — décrit une masse ponctuelle
/// avec une source à distance **finie** : elle ignore la sphère de photons, donc
/// l'ombre, donc précisément ce qui fait reconnaître un trou noir.
///
/// Les deux régimes se croisent à `b = 1,764` ; on prend le plus grand, ce qui
/// reste continu puisqu'ils se coupent.
///
/// ⚠️ **Duplication assumée avec le GLSL**, du même ordre que `PORTEE` dans le
/// champ magnétique, et gardée de la même façon : par un test de littéraux.
///
/// ⚠️ **N'existe que pour les gardes**, d'où `cfg(test)` : c'est le GLSL qui
/// l'exécute. Les deux **constantes**, elles, partent bien du Rust par uniform —
/// c'est ce qui évite que les valeurs divergent ; seule la *forme* est écrite
/// deux fois, et un test compare les deux textes.
#[cfg(test)]
pub(super) fn deviation(b: f32) -> f32 {
    if b <= 1.0 {
        return f32::INFINITY; // sous l'ombre : le rayon y tombe
    }
    (DEVIATION_FAIBLE / b).max(DEVIATION_FORTE - (1.0 - 1.0 / b).ln())
}

thread_local! {
    static TPL_PASSES: RefCell<Option<Vec<Material>>> = const { RefCell::new(None) };
    static TPL_PLASMA: RefCell<Option<Material>> = const { RefCell::new(None) };
    static TPL_LENTILLE: RefCell<Option<Material>> = const { RefCell::new(None) };
    /// **Le ciel de la vue courante**, posé par `rendu.rs` — le seul endroit où
    /// le `Fond` et le système sont tous deux en main.
    ///
    /// ⚠️ `None` tant que personne ne l'a posé, et la passe de lentille est alors
    /// **sautée**. Dessiner sans la texture ferait paniquer macroquad (« Image
    /// count in bindings and shader did not match »), et plusieurs écrans
    /// affichent des étoiles sans passer par `Rendu::rendre`.
    static FOND_CIEL: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
}

/// **Pose le ciel que la lentille échantillonnera.** À appeler avant de dessiner
/// un système, avec la cubemap du `Fond` de **cette** vue : chaque écran tire son
/// propre champ d'étoiles, et en prendre un autre afficherait dans l'anneau un
/// ciel qui n'existe nulle part.
pub fn poser_fond_stellaire(cube: Texture2D) {
    FOND_CIEL.with(|c| *c.borrow_mut() = Some(cube));
}

/// Le ciel courant, s'il y en a un.
pub(super) fn fond_stellaire() -> Option<Texture2D> {
    FOND_CIEL.with(|c| c.borrow().clone())
}

/// Material de la passe de lentille. **Ne s'applique qu'aux trous noirs**, d'où
/// un material à part : lier une cubemap à celui de toutes les étoiles obligerait
/// chacune à la fournir, sous peine de panique.
pub(super) fn mat_lentille() -> Material {
    TPL_LENTILLE.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_lentille());
        }
        c.borrow().clone().unwrap()
    })
}

/// Déclaration des uniforms de la lentille. **Source unique**, comme pour le
/// soleil : sert à créer le material et de référence au test de correspondance.
/// ⚠️ **Sans `time`**, donc sans passer par `impostor::uniforms_communs`. La
/// lentille ne bouge pas avec l'horloge : la déviation ne dépend que de la
/// géométrie. Déclarer un uniform qu'on ne lit pas ferait rougir
/// `aucun_uniform_de_lentille_n_est_mort` — à juste titre, c'est un réglage qui
/// ne fait rien — et l'y ajouter pour faire taire le test aurait été inverser
/// la garde.
fn uniforms_lentille() -> Vec<UniformDesc> {
    vec![
        // Le repère caméra, lu par le fragment pour reconstruire la direction de
        // visée ; `disc` par le vertex partagé, qui met le quad à l'échelle.
        UniformDesc::new("cam_right", UniformType::Float3),
        UniformDesc::new("cam_up", UniformType::Float3),
        UniformDesc::new("to_cam", UniformType::Float3),
        UniformDesc::new("disc", UniformType::Float1),
        UniformDesc::new("couronne", UniformType::Float1),
        UniformDesc::new("angle_ombre", UniformType::Float1),
        UniformDesc::new("dev_faible", UniformType::Float1),
        UniformDesc::new("dev_forte", UniformType::Float1),
    ]
}

fn charger_lentille() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR),
            fragment: &impostor::source("lentille.frag.glsl", FRAG_LENTILLE),
        },
        MaterialParams {
            uniforms: uniforms_lentille(),
            // ⚠️ `samplerCube`, et non `sampler2D`. macroquad ne connaît que des
            // `Texture2D`, mais miniquad lie la texture avec **sa propre** cible
            // au moment du dessin : un identifiant de cubemap arrive donc bien
            // sur un `samplerCube` (`fond::televerser_cubemap`).
            textures: vec!["fond".to_string()],
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                // N'écrit **pas** la profondeur : son quad est plat et face
                // caméra ; il poserait un plan jusqu'à plusieurs rayons et
                // rejetterait tout ce qui passe derrière — le défaut déjà payé
                // par la couronne.
                depth_write: false,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        },
    )
    .unwrap()
}

/// Les materials des passes de [`PASSES`], dans le même ordre.
pub(super) fn mats_soleil() -> Vec<Material> {
    TPL_PASSES.with(|c| {
        if c.borrow().is_none() {
            let v = PASSES.iter().map(|(_, prof)| charger_soleil(*prof)).collect();
            *c.borrow_mut() = Some(v);
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

pub(super) fn mat_plasma_partage() -> Material {
    TPL_PLASMA.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_plasma());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// Vide le cache de materials -> recompilation depuis les .glsl (hot-reload).
pub(super) fn vider_cache() {
    TPL_PASSES.with(|c| *c.borrow_mut() = None);
    TPL_PLASMA.with(|c| *c.borrow_mut() = None);
    TPL_LENTILLE.with(|c| *c.borrow_mut() = None);
}

/// Déclaration des uniforms du soleil. **Source unique** : sert à créer les deux
/// materials, et de référence au test de correspondance avec le `.glsl`.
fn uniforms_soleil() -> Vec<UniformDesc> {
    let mut uniforms = impostor::uniforms_communs();
    uniforms.extend([
        UniformDesc::new("teinte", UniformType::Float3),
        UniformDesc::new("couronne", UniformType::Float1),
        UniformDesc::new("couronne_irreg", UniformType::Float1),
        UniformDesc::new("couronne_type", UniformType::Float1),
        UniformDesc::array(UniformDesc::new("spots", UniformType::Float4), super::MAX_TACHES),
        UniformDesc::new("zone", UniformType::Float1),
        UniformDesc::new("graine", UniformType::Float1),
        UniformDesc::array(
            UniformDesc::new("poles", UniformType::Float4),
            super::champ::NB_POLES,
        ),
    ]);
    uniforms
}

/// `profondeur` : vrai pour la passe du disque (opaque, écrit la profondeur),
/// faux pour la couronne (translucide, n'occulte rien).
/// Source embarquée du shader de lentille (hot-reload par `impostor::source`).
const FRAG_LENTILLE: &str = include_str!("../shaders/lentille.frag.glsl");

fn charger_soleil(profondeur: bool) -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR),
            fragment: &impostor::source("soleil.frag.glsl", FRAG),
        },
        MaterialParams {
            uniforms: uniforms_soleil(),
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: profondeur,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

fn charger_plasma() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("soleil_plasma.vert.glsl", VERT_PLASMA),
            fragment: &impostor::source("soleil_plasma.frag.glsl", FRAG_PLASMA),
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::One)),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

// Plasma : texture de halo teintée par la couleur du sommet, sortie prémultipliée
// pour un blending purement additif (aucune composante noire).
const VERT_PLASMA: &str = include_str!("../shaders/soleil_plasma.vert.glsl");
const FRAG_PLASMA: &str = include_str!("../shaders/soleil_plasma.frag.glsl");
const FRAG: &str = include_str!("../shaders/soleil.frag.glsl");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::impostor::verif::{
        uniforms_glsl, uniforms_inutilises, uniforms_rust, usage_avant_declaration,
    };

    /// Un uniform lu par le `.glsl` mais absent de la déclaration Rust vaut
    /// **silencieusement zéro** : aucun message, l'effet disparaît.
    ///
    /// **La déviation diverge au bord de l'ombre et décroît en `1/b` au loin.**
    ///
    /// Les deux bouts, parce que ce sont eux qui font l'image : la divergence
    /// donne l'anneau et l'empilement d'images ; la décroissance en `1/b` fait
    /// que le ciel lointain reste à sa place.
    ///
    /// ⚠️ La **lentille mince** de la première version n'avait ni l'un ni
    /// l'autre : `β = θ − θ_E²/θ` décrit une masse ponctuelle avec une source à
    /// distance finie, sans sphère de photons, donc sans ombre.
    #[test]
    fn la_deviation_diverge_au_bord_de_l_ombre_et_decroit_en_un_sur_b() {
        // Sous l'ombre : le rayon y tombe, aucune direction de ciel.
        assert!(deviation(0.5).is_infinite());
        assert!(deviation(1.0).is_infinite());

        // Au bord : plus d'un demi-tour. C'est l'enroulement du ciel.
        assert!(deviation(1.02) > 3.0, "α(1,02) = {}", deviation(1.02));
        assert!(deviation(1.1) > 1.5, "α(1,1) = {}", deviation(1.1));

        // Monotone décroissante : une déviation qui remonterait replierait le
        // ciel sur lui-même au mauvais endroit.
        let mut precedent = f32::INFINITY;
        for k in 1..=400 {
            let b = 1.0 + k as f32 * 0.1;
            let a = deviation(b);
            assert!(a > 0.0, "b = {b} : α = {a}");
            assert!(a < precedent + 1e-6, "α remonte à b = {b}");
            precedent = a;
        }

        // ⚠️ **Au loin, exactement `1/b`** : doubler `b` doit halver `α`. C'est
        // le régime de champ faible, et s'il déviait, le ciel lointain partirait
        // en biais.
        let (a40, a80) = (deviation(40.0), deviation(80.0));
        assert!((a40 / a80 - 2.0).abs() < 0.01, "α(40)/α(80) = {}", a40 / a80);
        // Et la valeur elle-même : 2 r_s / b, r_s = 1/2,598.
        assert!((a40 - DEVIATION_FAIBLE / 40.0).abs() < 1e-6);
    }

    /// **Les deux régimes se croisent proprement**, à `b = 1,764`.
    ///
    /// ⚠️ On prend le **plus grand** des deux : c'est continu *parce qu'ils se
    /// coupent*. Si le croisement disparaissait — une constante retouchée — le
    /// `max` deviendrait une marche, et une marche dans une déviation se lit
    /// comme un anneau parasite dont on chercherait longtemps l'origine.
    #[test]
    fn les_deux_regimes_de_deviation_se_croisent_sans_marche() {
        let faible = |b: f32| DEVIATION_FAIBLE / b;
        let forte = |b: f32| DEVIATION_FORTE - (1.0 - 1.0 / b).ln();
        // Près de l'ombre la forte domine ; au loin, la faible.
        assert!(forte(1.2) > faible(1.2), "la déviation forte ne domine pas près de l'ombre");
        assert!(faible(3.0) > forte(3.0), "le champ faible ne reprend pas la main au loin");
        // Donc il existe un croisement, et il est unique sur la plage.
        let (mut lo, mut hi) = (1.2_f32, 3.0_f32);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if forte(mid) > faible(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let croisement = 0.5 * (lo + hi);
        assert!(
            (croisement - 1.764).abs() < 0.02,
            "croisement à b = {croisement} au lieu de 1,764"
        );
        // Et la courbe ne saute pas : deux points de part et d'autre diffèrent
        // de peu.
        let saut = (deviation(croisement - 0.01) - deviation(croisement + 0.01)).abs();
        assert!(saut < 0.02, "marche de {saut} rad au raccord des deux régimes");
    }

    /// **Le quad contient l'étirement visible, et son atténuation reste
    /// sub-pixel.**
    ///
    /// ⚠️ C'est le calcul qui a condamné la première version
    /// (`astres_v2_lentille.md` §2). La déviation ne tombe sous un pixel qu'au
    /// delà de 1400 rayons d'ombre : **aucun quad ne peut la contenir**. Mais sur
    /// un champ d'étoiles sans structure, un décalage **rigide** est
    /// indétectable — ce qui se voit est le **gradient**.
    ///
    /// Le test vérifie les deux moitiés de ce raisonnement : que l'étirement
    /// vrai soit déjà sub-pixel au bord du quad, et que le faux gradient
    /// introduit par l'atténuation le soit aussi.
    #[test]
    fn l_etendue_du_quad_contient_l_etirement_et_son_attenuation_reste_sub_pixel() {
        // Une ombre d'une centaine de pixels — le cas où on l'examine.
        let ombre_px = 100.0_f32;
        // Largeur d'un pixel, en rayons d'ombre.
        let db = 1.0 / ombre_px;
        // Un pixel, en radians : l'angle de l'ombre divisé par son rayon px.
        let angle_ombre = 0.1_f32;
        let px_rad = angle_ombre / ombre_px;

        let etirement_px = |b: f32| {
            ((deviation(b) - deviation(b + db)).abs()) / px_rad
        };
        // ⚠️ Près de l'ombre, l'étirement est **énorme** — c'est l'effet
        // lui-même. S'il devenait sub-pixel là, il n'y aurait plus de lentille.
        assert!(etirement_px(1.5) > 2.0, "étirement à b=1,5 : {} px", etirement_px(1.5));
        // Et au bord du quad il est retombé sous le pixel : rien d'autre à
        // contenir au-delà.
        let bord = ETENDUE_LENTILLE;
        assert!(
            etirement_px(bord) < 1.0,
            "étirement encore de {} px à {bord} rayons : le quad coupe l'effet",
            etirement_px(bord)
        );

        // **Le faux gradient de l'atténuation.** Elle éteint `α` sur le tiers
        // extérieur ; la pente qu'elle ajoute vaut au pire `α(début) / largeur`.
        let debut = bord * 0.66;
        let faux_gradient_px = deviation(debut) / ((bord - debut) / db) / px_rad;
        assert!(
            faux_gradient_px < 1.0,
            "l'atténuation ajoute {faux_gradient_px} px d'étirement par pixel : elle se verrait"
        );
    }

    /// **Les constantes du GLSL sont celles du Rust.**
    ///
    /// ⚠️ La formule vit **des deux côtés** — Rust pour la tester, GLSL pour
    /// l'exécuter — et c'est une duplication assumée, du même ordre que `PORTEE`
    /// dans le champ magnétique. Elle se garde comme elle : en comparant les
    /// deux textes. Sans ça, retoucher l'un laisse l'autre en place, et l'écran
    /// montre une déviation que plus aucun test ne décrit.
    #[test]
    fn la_formule_du_glsl_est_celle_du_rust() {
        // Le GLSL prend ses deux constantes par uniform, pas en dur : c'est ce
        // qui évite la divergence des valeurs.
        for u in ["dev_faible", "dev_forte"] {
            assert!(FRAG_LENTILLE.contains(u), "le shader n'utilise pas `{u}`");
        }
        // Et la **forme** de la formule, elle, est écrite des deux côtés.
        assert!(
            FRAG_LENTILLE.contains("max(dev_faible / b, dev_forte - log(1.0 - 1.0 / b))"),
            "le shader n'applique plus les deux régimes"
        );
        // La direction échantillonnée est bien `θ + α`, et non la lentille
        // mince `θ − θ_E²/θ` de la première version.
        assert!(
            FRAG_LENTILLE.contains("float phi = rr * angle_ombre + alpha;"),
            "le shader n'échantillonne plus à θ + α"
        );
        assert!(!FRAG_LENTILLE.contains("einstein"), "la lentille mince traîne encore");
    }

    /// **Les uniforms de la lentille correspondent à son shader.**
    ///
    /// ⚠️ Un uniform lu par le `.glsl` mais absent du Rust vaut silencieusement
    /// **zéro** : `angle_ombre` à zéro écraserait tout le ciel dévié sur un
    /// point, et `einstein` à zéro supprimerait l'anneau — sans le moindre
    /// message, et avec un rendu qui ressemble à un réglage mal dosé.
    #[test]
    fn uniforms_lentille_correspondent_au_shader() {
        let vert = impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR);
        let glsl = uniforms_glsl(&[&vert, FRAG_LENTILLE]);
        let rust = uniforms_rust(&uniforms_lentille());
        assert_eq!(glsl, rust, "lentille : GLSL (gauche) vs Rust (droite)");
    }

    /// **Et aucun de ses uniforms n'est ignoré.** ⚠️ `fond` est une **texture**,
    /// pas un uniform : elle se déclare dans `MaterialParams.textures`, et
    /// `uniforms_glsl` écarte tout ce qui commence par `sampler`. Si ce test
    /// venait à réclamer `fond`, c'est ce filtre qui aurait cédé.
    #[test]
    fn aucun_uniform_de_lentille_n_est_mort() {
        let morts = uniforms_inutilises(&[FRAG_LENTILLE]);
        assert!(morts.is_empty(), "uniforms déclarés mais jamais lus : {morts:?}");
    }

    /// **Aucune variable employée avant sa déclaration** dans le shader de
    /// lentille. GLSL n'a pas de déclaration anticipée, et rien ici ne compile le
    /// GLSL : le défaut ne se voit qu'au lancement, en panic. C'est arrivé au
    /// sous-lot 5.3.
    #[test]
    fn la_lentille_ne_lit_rien_avant_de_l_avoir_declare() {
        let fautes = usage_avant_declaration(FRAG_LENTILLE);
        assert!(fautes.is_empty(), "{fautes:?}");
    }

    /// **La lentille lit une cubemap, et le déclare comme telle.**
    ///
    /// ⚠️ Un `sampler2D` recevant un identifiant de cubemap ne rend rien de bon,
    /// et l'erreur est invisible côté Rust — macroquad ne connaît que des
    /// `Texture2D` des deux côtés. Le type n'existe que dans le GLSL : c'est donc
    /// là qu'il faut le vérifier.
    #[test]
    fn la_lentille_echantillonne_bien_une_cubemap() {
        assert!(
            FRAG_LENTILLE.contains("uniform samplerCube fond;"),
            "le fond n'est pas déclaré en samplerCube"
        );
        assert!(
            FRAG_LENTILLE.contains("textureCube(fond,"),
            "le fond n'est pas échantillonné en cubemap"
        );
        // Et le nom déclaré côté Rust est bien celui-là : `set_texture` sur un
        // nom inconnu est silencieux.
        assert!(
            charger_lentille_declare_fond(),
            "la texture `fond` n'est pas déclarée dans MaterialParams.textures"
        );
    }

    /// Lit la déclaration de textures **à sa source**, sans charger de material
    /// (ce qui exigerait le contexte graphique).
    fn charger_lentille_declare_fond() -> bool {
        include_str!("materiau.rs").contains("textures: vec![\"fond\".to_string()]")
    }

    /// Le cas concret, ici : `zone` sépare la passe du disque (opaque, écrit la
    /// profondeur) de celle de la couronne (translucide, n'occulte rien). Oublié
    /// côté Rust, il resterait à 0 dans les deux passes — le shader dessinerait
    /// deux fois le disque et **la couronne disparaîtrait purement et simplement**.
    #[test]
    fn uniforms_soleil_correspondent_au_shader() {
        let glsl = uniforms_glsl(&[impostor::VERT_IMPOSTOR, FRAG]);
        let rust = uniforms_rust(&uniforms_soleil());
        assert_eq!(glsl, rust, "soleil : GLSL (gauche) vs Rust (droite)");
    }

    /// Valeur d'un `#define <nom> <valeur>` du shader.
    fn define(nom: &str) -> Option<String> {
        let prefixe = format!("#define {nom} ");
        FRAG
            .lines()
            .find_map(|l| l.trim().strip_prefix(&prefixe).map(|v| v.trim().to_string()))
    }

    /// **Le tableau des pôles a la même taille des deux côtés.**
    ///
    /// Le GLSL fixe la sienne en dur (`uniform vec4 poles[6]`) : le langage
    /// n'a pas de constante partagée avec Rust. Trop court côté shader, les
    /// derniers pôles sont **silencieusement ignorés** — une région active
    /// entière disparaîtrait de la surface tout en continuant de porter
    /// taches et arches côté CPU.
    #[test]
    fn le_tableau_des_poles_a_la_meme_taille_des_deux_cotes() {
        let n = super::super::champ::NB_POLES;
        assert_eq!(define("NB_POLES").as_deref(), Some(n.to_string().as_str()), "#define NB_POLES");
        assert!(
            FRAG.contains(&format!("uniform vec4 poles[{n}];")),
            "la déclaration du tableau ne suit pas NB_POLES = {n}"
        );
    }

    /// **La portée du champ est la même des deux côtés.**
    ///
    /// C'est la dette CPU/GPU du dépôt, ici sous sa forme tolérable : une
    /// constante **géométrique** (`astres_v2_champ_partage.md` §3). Tolérable
    /// ne veut pas dire libre — si les deux divergent, les taches naissent à
    /// un endroit et la surface s'assombrit à un autre, ce qui est exactement
    /// le défaut que le sous-lot 3.4 corrige.
    #[test]
    fn la_portee_du_champ_est_la_meme_des_deux_cotes() {
        let glsl: f32 = define("PORTEE").expect("#define PORTEE absent").parse().expect("littéral");
        assert_eq!(glsl, super::super::champ::PORTEE, "PORTEE : GLSL vs Rust");
    }

    /// **Le disque écrit la profondeur, la couronne non.**
    ///
    /// C'est tout l'objet de la séparation. La couronne est un quad plat, face
    /// caméra, qui s'étend jusqu'à 2,1 rayons : en écrivant la profondeur elle
    /// posait un plan qui rejetait tout ce qui passe derrière. Sur un disque
    /// protosolaire (matière dès 0,1 UA), ce plan coupait le voile le long de
    /// son intersection avec le plan du disque — **une barre sombre en travers
    /// de l'étoile**. En vue système, il rognait aussi toute planète passant
    /// derrière l'étoile.
    ///
    /// L'inverse compte autant : sans profondeur sur le disque, un corps situé
    /// derrière l'étoile se dessinerait par-dessus (la vue système ne trie pas
    /// ses astres, elle s'en remet au tampon de profondeur).
    ///
    /// Le test porte sur [`PASSES`], la donnée d'où sortent **et** les materials
    /// **et** l'ordre de dessin — pas sur le texte du fichier.
    #[test]
    fn seul_le_disque_de_l_etoile_ecrit_la_profondeur() {
        let opaque = PASSES.iter().filter(|(_, prof)| *prof).count();
        assert_eq!(opaque, 1, "il faut exactement une passe opaque : {PASSES:?}");

        let (zone_opaque, _) = PASSES.iter().find(|(_, prof)| *prof).unwrap();
        let (zone_translucide, _) = PASSES.iter().find(|(_, prof)| !*prof).unwrap();
        // `soleil.frag.glsl` : zone < 0.5 garde rr <= 1 (le disque), zone > 0.5
        // garde rr > 1 (la couronne). L'opaque doit donc être la zone basse.
        assert!(
            *zone_opaque < 0.5,
            "la passe opaque doit être le disque (zone basse), pas la couronne"
        );
        assert!(*zone_translucide > 0.5, "la couronne doit être la zone haute");
    }

    /// Les deux passes doivent couvrir le quad **sans trou ni recouvrement** :
    /// le shader aiguille sur `rr <= 1` contre `rr > 1`, donc il faut exactement
    /// deux zones, distinctes. Une zone en trop et le disque se dessinerait deux
    /// fois ; une en moins et la couronne disparaîtrait.
    #[test]
    fn les_deux_passes_couvrent_le_quad_une_fois() {
        assert_eq!(PASSES.len(), 2, "le shader n'aiguille que deux zones");
        assert_ne!(PASSES[0].0, PASSES[1].0, "les deux passes tirent la même zone");
    }
}
