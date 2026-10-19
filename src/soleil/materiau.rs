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

thread_local! {
    static TPL_PASSES: RefCell<Option<Vec<Material>>> = const { RefCell::new(None) };
    static TPL_PLASMA: RefCell<Option<Material>> = const { RefCell::new(None) };
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
        UniformDesc::new("ebullition", UniformType::Float1),
        UniformDesc::new("ebul_style", UniformType::Float1),
        UniformDesc::array(
            UniformDesc::new("poles", UniformType::Float4),
            super::champ::NB_POLES,
        ),
    ]);
    uniforms
}

/// `profondeur` : vrai pour la passe du disque (opaque, écrit la profondeur),
/// faux pour la couronne (translucide, n'occulte rien).
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
    use crate::impostor::verif::{uniforms_glsl, uniforms_rust};

    /// Un uniform lu par le `.glsl` mais absent de la déclaration Rust vaut
    /// **silencieusement zéro** : aucun message, l'effet disparaît.
    ///
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
