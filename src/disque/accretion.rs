//! Couche **disque d'accrétion** (sous-lot 5.1) : même annulus et même
//! vertex shader que le voile, mais son propre fragment shader et son propre
//! vocabulaire.
//!
//! Le sous-lot 5.2 y a ajouté le **pli** : la moitié arrière du disque
//! relevée par-dessus et par-dessous l'astre, pour imiter la silhouette que
//! produit la déviation des rayons. Voir `disque_accretion.vert.glsl`.
//!
//! **Pourquoi un shader à part.** Le voile partagé porte celui des anneaux
//! planétaires — lacunes, arcs, annelets, ombre du corps central — dont rien
//! ne sert ici, et ne porte aucun des termes d'un disque d'accrétion :
//! profil de température, beaming, turbulence cisaillée. Le lot 5 avait
//! commencé à l'y glisser (`doppler`), ce qui aurait fini par alourdir chaque
//! anneau de Saturne du jeu d'uniforms qu'il laisse à zéro
//! (`astres_v2_disque_accretion.md` §2). Un pipeline de plus sur les 32 du
//! moteur, mesh et batching réutilisés tels quels.

use super::DisqueConfig;
use crate::impostor;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

const VERT: &str = include_str!("../shaders/disque_accretion.vert.glsl");
const FRAG: &str = include_str!("../shaders/disque_accretion.frag.glsl");

/// **Les passes à dessiner** pour une moitié de disque : `(amplitude du pli,
/// signe)` pour chacune.
///
/// La moitié **arrière** se dessine **deux fois**, relevée au-dessus puis
/// au-dessous de l'astre : c'est le pli. La moitié **avant** reste à plat —
/// elle n'a jamais été cachée par l'astre, donc rien ne la dévie, et la
/// relever ferait décoller le disque de ses propres bords.
///
/// Un disque sans pli garde une seule passe par moitié, exactement comme
/// avant le sous-lot 5.2.
pub(super) fn passes_pli(pli: f32, moitie: f32) -> Vec<(f32, f32)> {
    if pli <= 0.0 || moitie >= 0.0 {
        return vec![(0.0, 1.0)];
    }
    vec![(pli, 1.0), (pli, -1.0)]
}

/// Pousse les uniforms du disque d'accrétion.
// Ce sont les UNIFORMS du shader : les grouper ne ferait que déplacer la
// liste d'un cran, comme pour `uniforms_voile`.
#[allow(clippy::too_many_arguments)]
pub(super) fn uniforms_accretion(
    mat: &Material,
    c: &DisqueConfig,
    temps: f32,
    // `centre` au **repère de rendu** (`CameraInfo::rel`), comme le voile :
    // le vertex shader est le même, il lui faut la même origine qu'à la
    // caméra.
    centre: Vec3,
    axes: (Vec3, Vec3, Vec3),
    tocam: Vec3,
    moitie: f32,
    // `(amplitude, signe)` du pli. `(0, _)` dessine le disque à plat.
    pli: (f32, f32),
) {
    let (u, n, v) = axes;
    mat.set_uniform("time", temps);
    mat.set_uniform("seed", c.graine);
    mat.set_uniform("centre", (centre.x, centre.y, centre.z));
    mat.set_uniform("axe_u", (u.x, u.y, u.z));
    mat.set_uniform("axe_n", (n.x, n.y, n.z));
    mat.set_uniform("axe_v", (v.x, v.y, v.z));
    mat.set_uniform("tocam", (tocam.x, tocam.y, tocam.z));
    mat.set_uniform("moitie", moitie);
    mat.set_uniform("alpha", c.voile_alpha);
    // `voile_couleur2` est le bord interne, `voile_couleur` l'externe : même
    // convention que les autres presets, pour que la config reste lisible.
    let (ch, fr) = (c.voile_couleur2, c.voile_couleur);
    mat.set_uniform("chaud", (ch.x, ch.y, ch.z));
    mat.set_uniform("froid", (fr.x, fr.y, fr.z));
    mat.set_uniform("exposant", c.temp_exposant);
    mat.set_uniform("turbulence", c.granulation);
    mat.set_uniform("turb_freq", (c.gran_freq.x, c.gran_freq.y));
    mat.set_uniform("rotation", c.rotation_voile);
    mat.set_uniform("r_ratio", (c.externe / c.interne.max(1e-3)).max(1.001));
    mat.set_uniform("doppler", c.doppler);
    mat.set_uniform("bord", c.voile_bord);
    mat.set_uniform("pli", pli.0);
    mat.set_uniform("pli_signe", pli.1);
}

/// **Déclaration des uniforms. Source unique** : sert à créer le material, et
/// de référence au test de correspondance avec le `.glsl`.
fn uniforms_accretion_decl() -> Vec<UniformDesc> {
    vec![
        UniformDesc::new("time", UniformType::Float1),
        UniformDesc::new("seed", UniformType::Float1),
        UniformDesc::new("centre", UniformType::Float3),
        UniformDesc::new("axe_u", UniformType::Float3),
        UniformDesc::new("axe_n", UniformType::Float3),
        UniformDesc::new("axe_v", UniformType::Float3),
        UniformDesc::new("tocam", UniformType::Float3),
        UniformDesc::new("moitie", UniformType::Float1),
        UniformDesc::new("alpha", UniformType::Float1),
        UniformDesc::new("chaud", UniformType::Float3),
        UniformDesc::new("froid", UniformType::Float3),
        UniformDesc::new("exposant", UniformType::Float1),
        UniformDesc::new("turbulence", UniformType::Float1),
        UniformDesc::new("turb_freq", UniformType::Float2),
        UniformDesc::new("rotation", UniformType::Float1),
        UniformDesc::new("r_ratio", UniformType::Float1),
        UniformDesc::new("doppler", UniformType::Float1),
        UniformDesc::new("bord", UniformType::Float1),
        UniformDesc::new("pli", UniformType::Float1),
        UniformDesc::new("pli_signe", UniformType::Float1),
    ]
}

/// Material du disque d'accrétion. **Additif** — et c'est la différence de
/// fond avec le voile : un anneau planétaire *occulte* ce qu'il y a derrière
/// (alpha blend), un disque d'accrétion *émet*, ses recouvrements s'ajoutent.
pub(super) fn material_accretion() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("disque_accretion.vert.glsl", VERT),
            fragment: &impostor::source("disque_accretion.frag.glsl", FRAG),
        },
        MaterialParams {
            uniforms: uniforms_accretion_decl(),
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::One,
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::impostor::verif::{uniforms_glsl, uniforms_inutilises, uniforms_rust};

    /// **Un uniform lu par le `.glsl` mais absent du Rust vaut silencieusement
    /// zéro** : aucun message, l'effet disparaît. La garde que le voile n'a
    /// reçue qu'au lot 5, posée ici **dès le premier commit**.
    #[test]
    fn uniforms_accretion_correspondent_au_shader() {
        let glsl = uniforms_glsl(&[VERT, FRAG]);
        let rust = uniforms_rust(&uniforms_accretion_decl());
        assert_eq!(glsl, rust, "accrétion : GLSL (gauche) vs Rust (droite)");
    }

    /// **Et aucun uniform déclaré n'est ignoré par le shader** : un réglage
    /// qui ne fait rien est pire qu'absent.
    #[test]
    fn aucun_uniform_d_accretion_n_est_mort() {
        let morts = uniforms_inutilises(&[VERT, FRAG]);
        assert!(morts.is_empty(), "uniforms déclarés mais jamais lus : {morts:?}");
    }

    /// **Seule la moitié arrière se dédouble, et ses deux arcs sont
    /// opposés.**
    ///
    /// C'est toute la silhouette : la face arrière du disque apparaît
    /// au-dessus de l'astre **et** au-dessous. Deux arcs de même signe se
    /// superposeraient — un seul arc, deux fois trop brillant en additif ; et
    /// plier la moitié avant décollerait le disque de ses propres bords,
    /// puisqu'elle n'a jamais été cachée.
    #[test]
    fn seule_la_moitie_arriere_se_dedouble() {
        let arriere = passes_pli(1.15, -1.0);
        assert_eq!(arriere.len(), 2, "la moitié arrière ne donne pas deux arcs");
        assert!(arriere[0].1 * arriere[1].1 < 0.0, "les deux arcs vont du même côté");
        assert!(arriere.iter().all(|p| p.0 > 0.0), "un arc dessiné à plat");

        let avant = passes_pli(1.15, 1.0);
        assert_eq!(avant.len(), 1, "la moitié avant a été dédoublée");
        assert_eq!(avant[0].0, 0.0, "la moitié avant a été pliée");
    }

    /// **Un disque sans pli dessine exactement comme avant** : une passe par
    /// moitié, à plat. Le pli ne doit rien coûter à ce qui ne s'en sert pas.
    #[test]
    fn sans_pli_rien_ne_change() {
        for moitie in [-1.0_f32, 0.0, 1.0] {
            let p = passes_pli(0.0, moitie);
            assert_eq!(p.len(), 1, "moitié {moitie} : passe de trop sans pli");
            assert_eq!(p[0].0, 0.0);
        }
    }
}
