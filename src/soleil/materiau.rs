use crate::impostor;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::cell::RefCell;

// Materials partagÃ©s : un seul pipeline chacun, clonÃ©s Ã  chaque soleil.
/// **Les deux passes de l'Ã©toile** : `(zone, Ã©crit la profondeur)`.
///
/// Le disque de l'Ã©toile (`zone` 0) est opaque : il doit occulter ce qui passe
/// derriÃ¨re, sinon un corps cachÃ© par l'Ã©toile se dessinerait par-dessus (la vue
/// systÃ¨me ne trie pas ses astres). La couronne (`zone` 1) est translucide : elle
/// ne doit RIEN occulter. Son quad est plat, face camÃ©ra, et s'Ã©tend jusqu'Ã  2,1
/// rayons ; en Ã©crivant la profondeur elle posait un plan qui rejetait tout ce
/// qui passait derriÃ¨re â€” d'oÃ¹ la barre sombre en travers des Ã©toiles des disques
/// protosolaires, lÃ  oÃ¹ ce plan coupait le voile.
///
/// GLSL ES 100 ne sait pas couper l'Ã©criture de profondeur par fragment : il faut
/// deux pipelines sur le mÃªme shader, que `zone` aiguille (`soleil.frag.glsl`).
///
/// **Source unique** : le nombre de materials, l'ordre de dessin (`rendu.rs`) et
/// le test d'invariant lisent tous cette table.
pub(super) const PASSES: [(f32, bool); 2] = [(0.0, true), (1.0, false)];

/// **Rayon d'Einstein**, en rayons de l'ombre. C'est lui qui place l'anneau.
///
/// âš ï¸ RÃ©glage d'image, pas une constante physique : la formule de lentille mince
/// dÃ©crit une masse ponctuelle et une source plane, ni l'un ni l'autre exact ici.
/// PosÃ© un peu au-delÃ  du bord de l'ombre pour que l'anneau se dÃ©tache du noir
/// sans se confondre avec le liserÃ© ni avec l'anneau de lentille du disque, qui
/// occupent dÃ©jÃ  ce voisinage.
pub(super) const RAYON_EINSTEIN: f32 = 1.35;

thread_local! {
    static TPL_PASSES: RefCell<Option<Vec<Material>>> = const { RefCell::new(None) };
    static TPL_PLASMA: RefCell<Option<Material>> = const { RefCell::new(None) };
    static TPL_LENTILLE: RefCell<Option<Material>> = const { RefCell::new(None) };
    /// **Le ciel de la vue courante**, posÃ© par `rendu.rs` â€” le seul endroit oÃ¹
    /// le `Fond` et le systÃ¨me sont tous deux en main.
    ///
    /// âš ï¸ `None` tant que personne ne l'a posÃ©, et la passe de lentille est alors
    /// **sautÃ©e**. Dessiner sans la texture ferait paniquer macroquad (Â« Image
    /// count in bindings and shader did not match Â»), et plusieurs Ã©crans
    /// affichent des Ã©toiles sans passer par `Rendu::rendre`.
    static FOND_CIEL: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
}

/// **Pose le ciel que la lentille Ã©chantillonnera.** Ã€ appeler avant de dessiner
/// un systÃ¨me, avec la cubemap du `Fond` de **cette** vue : chaque Ã©cran tire son
/// propre champ d'Ã©toiles, et en prendre un autre afficherait dans l'anneau un
/// ciel qui n'existe nulle part.
pub fn poser_fond_stellaire(cube: Texture2D) {
    FOND_CIEL.with(|c| *c.borrow_mut() = Some(cube));
}

/// Le ciel courant, s'il y en a un.
pub(super) fn fond_stellaire() -> Option<Texture2D> {
    FOND_CIEL.with(|c| c.borrow().clone())
}

/// Material de la passe de lentille. **Ne s'applique qu'aux trous noirs**, d'oÃ¹
/// un material Ã  part : lier une cubemap Ã  celui de toutes les Ã©toiles obligerait
/// chacune Ã  la fournir, sous peine de panique.
pub(super) fn mat_lentille() -> Material {
    TPL_LENTILLE.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_lentille());
        }
        c.borrow().clone().unwrap()
    })
}

/// DÃ©claration des uniforms de la lentille. **Source unique**, comme pour le
/// soleil : sert Ã  crÃ©er le material et de rÃ©fÃ©rence au test de correspondance.
/// âš ï¸ **Sans `time`**, donc sans passer par `impostor::uniforms_communs`. La
/// lentille ne bouge pas avec l'horloge : la dÃ©viation ne dÃ©pend que de la
/// gÃ©omÃ©trie. DÃ©clarer un uniform qu'on ne lit pas ferait rougir
/// `aucun_uniform_de_lentille_n_est_mort` â€” Ã  juste titre, c'est un rÃ©glage qui
/// ne fait rien â€” et l'y ajouter pour faire taire le test aurait Ã©tÃ© inverser
/// la garde.
fn uniforms_lentille() -> Vec<UniformDesc> {
    vec![
        // Le repÃ¨re camÃ©ra, lu par le fragment pour reconstruire la direction de
        // visÃ©e ; `disc` par le vertex partagÃ©, qui met le quad Ã  l'Ã©chelle.
        UniformDesc::new("cam_right", UniformType::Float3),
        UniformDesc::new("cam_up", UniformType::Float3),
        UniformDesc::new("to_cam", UniformType::Float3),
        UniformDesc::new("disc", UniformType::Float1),
        UniformDesc::new("couronne", UniformType::Float1),
        UniformDesc::new("angle_ombre", UniformType::Float1),
        UniformDesc::new("einstein", UniformType::Float1),
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
            // âš ï¸ `samplerCube`, et non `sampler2D`. macroquad ne connaÃ®t que des
            // `Texture2D`, mais miniquad lie la texture avec **sa propre** cible
            // au moment du dessin : un identifiant de cubemap arrive donc bien
            // sur un `samplerCube` (`fond::televerser_cubemap`).
            textures: vec!["fond".to_string()],
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                // N'Ã©crit **pas** la profondeur : son quad est plat et face
                // camÃ©ra ; il poserait un plan jusqu'Ã  plusieurs rayons et
                // rejetterait tout ce qui passe derriÃ¨re â€” le dÃ©faut dÃ©jÃ  payÃ©
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

/// Les materials des passes de [`PASSES`], dans le mÃªme ordre.
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

/// DÃ©claration des uniforms du soleil. **Source unique** : sert Ã  crÃ©er les deux
/// materials, et de rÃ©fÃ©rence au test de correspondance avec le `.glsl`.
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

/// `profondeur` : vrai pour la passe du disque (opaque, Ã©crit la profondeur),
/// faux pour la couronne (translucide, n'occulte rien).
/// Source embarquÃ©e du shader de lentille (hot-reload par `impostor::source`).
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

// Plasma : texture de halo teintÃ©e par la couleur du sommet, sortie prÃ©multipliÃ©e
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

    /// Un uniform lu par le `.glsl` mais absent de la dÃ©claration Rust vaut
    /// **silencieusement zÃ©ro** : aucun message, l'effet disparaÃ®t.
    ///
    /// **Les uniforms de la lentille correspondent Ã  son shader.**
    ///
    /// âš ï¸ Un uniform lu par le `.glsl` mais absent du Rust vaut silencieusement
    /// **zÃ©ro** : `angle_ombre` Ã  zÃ©ro Ã©craserait tout le ciel dÃ©viÃ© sur un
    /// point, et `einstein` Ã  zÃ©ro supprimerait l'anneau â€” sans le moindre
    /// message, et avec un rendu qui ressemble Ã  un rÃ©glage mal dosÃ©.
    #[test]
    fn uniforms_lentille_correspondent_au_shader() {
        let vert = impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR);
        let glsl = uniforms_glsl(&[&vert, FRAG_LENTILLE]);
        let rust = uniforms_rust(&uniforms_lentille());
        assert_eq!(glsl, rust, "lentille : GLSL (gauche) vs Rust (droite)");
    }

    /// **Et aucun de ses uniforms n'est ignorÃ©.** âš ï¸ `fond` est une **texture**,
    /// pas un uniform : elle se dÃ©clare dans `MaterialParams.textures`, et
    /// `uniforms_glsl` Ã©carte tout ce qui commence par `sampler`. Si ce test
    /// venait Ã  rÃ©clamer `fond`, c'est ce filtre qui aurait cÃ©dÃ©.
    #[test]
    fn aucun_uniform_de_lentille_n_est_mort() {
        let morts = uniforms_inutilises(&[FRAG_LENTILLE]);
        assert!(morts.is_empty(), "uniforms dÃ©clarÃ©s mais jamais lus : {morts:?}");
    }

    /// **Aucune variable employÃ©e avant sa dÃ©claration** dans le shader de
    /// lentille. GLSL n'a pas de dÃ©claration anticipÃ©e, et rien ici ne compile le
    /// GLSL : le dÃ©faut ne se voit qu'au lancement, en panic. C'est arrivÃ© au
    /// sous-lot 5.3.
    #[test]
    fn la_lentille_ne_lit_rien_avant_de_l_avoir_declare() {
        let fautes = usage_avant_declaration(FRAG_LENTILLE);
        assert!(fautes.is_empty(), "{fautes:?}");
    }

    /// **La lentille lit une cubemap, et le dÃ©clare comme telle.**
    ///
    /// âš ï¸ Un `sampler2D` recevant un identifiant de cubemap ne rend rien de bon,
    /// et l'erreur est invisible cÃ´tÃ© Rust â€” macroquad ne connaÃ®t que des
    /// `Texture2D` des deux cÃ´tÃ©s. Le type n'existe que dans le GLSL : c'est donc
    /// lÃ  qu'il faut le vÃ©rifier.
    #[test]
    fn la_lentille_echantillonne_bien_une_cubemap() {
        assert!(
            FRAG_LENTILLE.contains("uniform samplerCube fond;"),
            "le fond n'est pas dÃ©clarÃ© en samplerCube"
        );
        assert!(
            FRAG_LENTILLE.contains("textureCube(fond,"),
            "le fond n'est pas Ã©chantillonnÃ© en cubemap"
        );
        // Et le nom dÃ©clarÃ© cÃ´tÃ© Rust est bien celui-lÃ  : `set_texture` sur un
        // nom inconnu est silencieux.
        assert!(
            charger_lentille_declare_fond(),
            "la texture `fond` n'est pas dÃ©clarÃ©e dans MaterialParams.textures"
        );
    }

    /// Lit la dÃ©claration de textures **Ã  sa source**, sans charger de material
    /// (ce qui exigerait le contexte graphique).
    fn charger_lentille_declare_fond() -> bool {
        include_str!("materiau.rs").contains("textures: vec![\"fond\".to_string()]")
    }

    /// Le cas concret, ici : `zone` sÃ©pare la passe du disque (opaque, Ã©crit la
    /// profondeur) de celle de la couronne (translucide, n'occulte rien). OubliÃ©
    /// cÃ´tÃ© Rust, il resterait Ã  0 dans les deux passes â€” le shader dessinerait
    /// deux fois le disque et **la couronne disparaÃ®trait purement et simplement**.
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

    /// **Le tableau des pÃ´les a la mÃªme taille des deux cÃ´tÃ©s.**
    ///
    /// Le GLSL fixe la sienne en dur (`uniform vec4 poles[6]`) : le langage
    /// n'a pas de constante partagÃ©e avec Rust. Trop court cÃ´tÃ© shader, les
    /// derniers pÃ´les sont **silencieusement ignorÃ©s** â€” une rÃ©gion active
    /// entiÃ¨re disparaÃ®trait de la surface tout en continuant de porter
    /// taches et arches cÃ´tÃ© CPU.
    #[test]
    fn le_tableau_des_poles_a_la_meme_taille_des_deux_cotes() {
        let n = super::super::champ::NB_POLES;
        assert_eq!(define("NB_POLES").as_deref(), Some(n.to_string().as_str()), "#define NB_POLES");
        assert!(
            FRAG.contains(&format!("uniform vec4 poles[{n}];")),
            "la dÃ©claration du tableau ne suit pas NB_POLES = {n}"
        );
    }

    /// **La portÃ©e du champ est la mÃªme des deux cÃ´tÃ©s.**
    ///
    /// C'est la dette CPU/GPU du dÃ©pÃ´t, ici sous sa forme tolÃ©rable : une
    /// constante **gÃ©omÃ©trique** (`astres_v2_champ_partage.md` Â§3). TolÃ©rable
    /// ne veut pas dire libre â€” si les deux divergent, les taches naissent Ã 
    /// un endroit et la surface s'assombrit Ã  un autre, ce qui est exactement
    /// le dÃ©faut que le sous-lot 3.4 corrige.
    #[test]
    fn la_portee_du_champ_est_la_meme_des_deux_cotes() {
        let glsl: f32 = define("PORTEE").expect("#define PORTEE absent").parse().expect("littÃ©ral");
        assert_eq!(glsl, super::super::champ::PORTEE, "PORTEE : GLSL vs Rust");
    }

    /// **Le disque Ã©crit la profondeur, la couronne non.**
    ///
    /// C'est tout l'objet de la sÃ©paration. La couronne est un quad plat, face
    /// camÃ©ra, qui s'Ã©tend jusqu'Ã  2,1 rayons : en Ã©crivant la profondeur elle
    /// posait un plan qui rejetait tout ce qui passe derriÃ¨re. Sur un disque
    /// protosolaire (matiÃ¨re dÃ¨s 0,1 UA), ce plan coupait le voile le long de
    /// son intersection avec le plan du disque â€” **une barre sombre en travers
    /// de l'Ã©toile**. En vue systÃ¨me, il rognait aussi toute planÃ¨te passant
    /// derriÃ¨re l'Ã©toile.
    ///
    /// L'inverse compte autant : sans profondeur sur le disque, un corps situÃ©
    /// derriÃ¨re l'Ã©toile se dessinerait par-dessus (la vue systÃ¨me ne trie pas
    /// ses astres, elle s'en remet au tampon de profondeur).
    ///
    /// Le test porte sur [`PASSES`], la donnÃ©e d'oÃ¹ sortent **et** les materials
    /// **et** l'ordre de dessin â€” pas sur le texte du fichier.
    #[test]
    fn seul_le_disque_de_l_etoile_ecrit_la_profondeur() {
        let opaque = PASSES.iter().filter(|(_, prof)| *prof).count();
        assert_eq!(opaque, 1, "il faut exactement une passe opaque : {PASSES:?}");

        let (zone_opaque, _) = PASSES.iter().find(|(_, prof)| *prof).unwrap();
        let (zone_translucide, _) = PASSES.iter().find(|(_, prof)| !*prof).unwrap();
        // `soleil.frag.glsl` : zone < 0.5 garde rr <= 1 (le disque), zone > 0.5
        // garde rr > 1 (la couronne). L'opaque doit donc Ãªtre la zone basse.
        assert!(
            *zone_opaque < 0.5,
            "la passe opaque doit Ãªtre le disque (zone basse), pas la couronne"
        );
        assert!(*zone_translucide > 0.5, "la couronne doit Ãªtre la zone haute");
    }

    /// Les deux passes doivent couvrir le quad **sans trou ni recouvrement** :
    /// le shader aiguille sur `rr <= 1` contre `rr > 1`, donc il faut exactement
    /// deux zones, distinctes. Une zone en trop et le disque se dessinerait deux
    /// fois ; une en moins et la couronne disparaÃ®trait.
    #[test]
    fn les_deux_passes_couvrent_le_quad_une_fois() {
        assert_eq!(PASSES.len(), 2, "le shader n'aiguille que deux zones");
        assert_ne!(PASSES[0].0, PASSES[1].0, "les deux passes tirent la mÃªme zone");
    }
}
