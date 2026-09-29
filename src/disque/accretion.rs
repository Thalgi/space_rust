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

/// **Tout le vocabulaire d'un disque d'accrétion**, et rien d'autre.
///
/// ⚠️ Un type à part plutôt qu'une dizaine de champs de plus sur
/// `DisqueConfig` : celui-ci est partagé avec les anneaux de Saturne, et la
/// recherche lui reprochait déjà de mélanger les deux vocabulaires
/// (`astres_v2_disque_accretion_2.md` §2). Corollaire heureux : deux cibles
/// visuelles ne diffèrent plus que par des **constantes**, ce qui permet de
/// les comparer côte à côte en galerie.
#[derive(Clone, Copy, Debug)]
pub struct Accretion {
    /// Teinte du bord interne, le plus chaud. ⚠️ **Ordonnée `R > G > B`.**
    /// Sous blending additif on ne peut pas tone-mapper la somme — elle
    /// s'accumule dans le framebuffer, après le shader. Mais le **clipping
    /// par canal** fait le travail gratuitement : le rouge sature d'abord,
    /// puis le vert, puis le bleu, et le cœur passe orange → jaune → blanc à
    /// mesure que l'intensité monte. Une teinte bleutée (`B ≥ R`) saturerait
    /// vers le **cyan** — c'était notre défaut.
    pub chaud: Vec3,
    /// Teinte du bord externe, le plus froid.
    pub froid: Vec3,
    /// Intensité au bord interne. **Laissée dépasser 1** : c'est elle qui
    /// déclenche le clipping décrit ci-dessus.
    pub eclat: f32,
    /// Dureté du profil de **couleur**. La physique donne `T ∝ r^(-3/4)` et
    /// une émission en `T⁴`, soit `I ∝ r^(-3)`.
    pub exposant_couleur: f32,
    /// Dureté du profil d'**opacité**, volontairement plus douce : un seul
    /// exposant raide éteint le bord externe au point qu'il disparaît, et on
    /// perd le disque.
    pub exposant_opacite: f32,
    /// Vitesse orbitale au bord interne, en fraction de `c`. Alimente le
    /// beaming `I ∝ D³`.
    pub beta: f32,
    /// **Plafond du beaming.** ⚠️ C'est l'arbitrage du chantier : à
    /// β = 0,35 la loi en `D³` donne un rapport de 8 à 10 entre les deux
    /// côtés, et l'ombre du trou noir devient **illisible** — le papier
    /// d'Interstellar a retiré le beaming pour cette raison exacte. Le
    /// plafond garde le trait sans perdre la silhouette.
    pub beaming_max: f32,
    /// Cellules de bruit : `(radiales, azimutales)`. L'azimutale doit être
    /// **entière** — le hash s'enroule dessus pour qu'il n'y ait pas de
    /// couture. Un rapport d'aspect de 8:1 à 20:1 donne des filaments ; à
    /// 1:1 on obtient des bulles.
    pub cellules: Vec2,
    /// Enroulement du bruit en **spirale logarithmique**, en radians par
    /// e-fold de rayon. Sans lui les structures restent concentriques au
    /// lieu de s'enrouler.
    pub spirale: f32,
    /// Force de la turbulence (0 = disque lisse).
    pub turbulence: f32,
    /// Vitesse angulaire visuelle au bord interne.
    pub rotation: f32,
    /// Douceur du bord **interne**. Quasi nul : l'ISCO est une coupure
    /// franche, pas un fondu.
    pub bord_interne: f32,
    /// Douceur du bord **externe**, large : le disque s'y dilue.
    pub bord_externe: f32,
    /// Amplitude du pli (sous-lot 5.2), en part du rayon de l'ombre.
    pub pli: f32,
    /// Épaisseur géométrique `h/R`. Le disque est dessiné en plusieurs
    /// nappes décalées de ±h, ce qui lui donne une tranche crédible au lieu
    /// d'une ligne d'épaisseur nulle.
    pub epaisseur: f32,
    /// Nombre de nappes empilées. ⚠️ L'éclat est divisé d'autant : en
    /// additif, empiler sans diviser part en **burn-out**, un blanc plat qui
    /// détruit la silhouette.
    pub nappes: usize,
    /// Éclat du **faux bloom** : un annulus large et faible, sans bruit. On
    /// n'a pas de post-traitement global — sans lui, un disque fin se lit
    /// « rouge et terne » au lieu de « blanc brûlant ».
    pub halo: f32,
    /// Éclat de l'**anneau de lentille**, en multiple de l'émission directe.
    /// ⚠️ Ce n'est **pas** l'anneau de photons, qui ne gagne qu'un
    /// renforcement logarithmique et reste négligeable. C'est une image
    /// démagnifiée de la face arrière, 2 à 3 fois plus brillante que
    /// l'émission directe — et qui porte donc la couleur, le bruit et
    /// l'asymétrie du disque, jamais un cercle uniforme.
    pub anneau: f32,
}

impl Accretion {
    /// **Cible « intermédiaire »** (sous-lot 5.3) : la physique d'un disque
    /// stellaire actif — blanc-bleu saturant en blanc — mais un beaming
    /// plafonné pour que l'ombre reste lisible.
    pub fn chaude() -> Self {
        Self {
            // R > G > B : le clipping additif emmène le cœur vers le blanc.
            // ⚠️ Volontairement **peu** chaude : l'ordre des canaux est ce
            // qui évite le cyan, mais plus l'écart est grand, plus la montée
            // vers le blanc passe par un jaune marqué. Resserrée pour que le
            // cœur se lise franchement blanc.
            chaud: vec3(1.0, 0.97, 0.93),
            froid: vec3(0.85, 0.45, 0.22),
            // ⚠️ Descendu de 3,2 : après la correction du `couche` appliqué
            // deux fois — qui rendait tout neuf fois trop sombre — la moitié
            // qui s'approche se mettait à **écrêter partout**, et sa texture
            // disparaissait sous un aplat gris. On veut un cœur qui sature,
            // pas une moitié de disque qui sature.
            eclat: 1.6,
            exposant_couleur: 2.2,
            exposant_opacite: 0.6,
            beta: 0.35,
            beaming_max: 4.0,
            cellules: vec2(16.0, 3.0), // ~5:1 puis étiré par la spirale
            spirale: 2.4,
            turbulence: 0.65,
            rotation: 0.35,
            bord_interne: 0.015,
            bord_externe: 0.30,
            // Relevé de 0,55 : l'arc doit passer franchement au-dessus de
            // l'ombre, pas la raser.
            pli: 1.1,
            epaisseur: 0.07,
            nappes: 3,
            halo: 0.10,
            anneau: 2.4,
        }
    }
}

/// Ce que chaque passe de dessin a de particulier.
#[derive(Clone, Copy)]
pub(super) struct Passe {
    /// `(amplitude du pli, signe)`. `(0, _)` : à plat.
    pub pli: (f32, f32),
    /// Décalage vertical de la nappe (épaisseur).
    pub decalage: f32,
    /// Éclat de cette nappe. ⚠️ Divisé par le nombre de nappes : en
    /// additif, empiler sans diviser part en **burn-out**.
    pub couche: f32,
    /// > 0 : c'est le faux bloom, sans bruit ni bord franc.
    pub halo: f32,
    /// > 0 : c'est l'anneau de lentille.
    pub anneau: f32,
}

impl Passe {
    fn nappe(pli: (f32, f32), decalage: f32, couche: f32) -> Self {
        Self { pli, decalage, couche, halo: 0.0, anneau: 0.0 }
    }
}

/// **Toutes les passes d'une moitié de disque.**
///
/// La moitié **arrière** se dessine deux fois — les deux arcs du pli — la
/// moitié **avant** reste à plat : elle n'a jamais été cachée par l'astre,
/// rien ne la dévie. Chacune est ensuite empilée en `nappes` pour donner au
/// disque une épaisseur, et la moitié avant porte en plus le faux bloom et
/// l'anneau de lentille.
pub(super) fn passes(a: &Accretion, moitie: f32) -> Vec<Passe> {
    let arcs: &[(f32, f32)] =
        if moitie < 0.0 { &[(a.pli, 1.0), (a.pli, -1.0)] } else { &[(0.0, 1.0)] };
    // ⚠️ **L'épaisseur n'est empilée que sur la moitié à plat.** Les arcs du
    // pli sont déjà étalés par la déformation, et leur donner des nappes
    // faisait passer le disque de 26 880 à 42 240 sommets — au-dessus d'un
    // vaisseau entier, pour un objet qui n'est qu'un décor. Ce qu'on perd
    // est invisible ; ce qu'on gagne est mesurable.
    let n = if moitie < 0.0 { 1 } else { a.nappes.max(1) };
    let couche = 1.0 / n as f32;
    let mut out = Vec::with_capacity(arcs.len() * n + 2);
    for arc in arcs {
        for i in 0..n {
            // Nappes réparties symétriquement autour du plan.
            let u = if n == 1 { 0.0 } else { i as f32 / (n - 1) as f32 * 2.0 - 1.0 };
            out.push(Passe::nappe(*arc, u * a.epaisseur, couche));
        }
    }
    if moitie >= 0.0 {
        if a.halo > 0.0 {
            out.push(Passe { halo: a.halo, ..Passe::nappe((0.0, 1.0), 0.0, 1.0) });
        }
        if a.anneau > 0.0 {
            out.push(Passe { anneau: a.anneau, ..Passe::nappe((0.0, 1.0), 0.0, 1.0) });
        }
    }
    out
}

/// Pousse les uniforms du disque d'accrétion.
#[allow(clippy::too_many_arguments)]
pub(super) fn uniforms_accretion(
    mat: &Material,
    c: &DisqueConfig,
    a: &Accretion,
    temps: f32,
    centre: Vec3,
    axes: (Vec3, Vec3, Vec3),
    tocam: Vec3,
    moitie: f32,
    passe: Passe,
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
    mat.set_uniform("chaud", (a.chaud.x, a.chaud.y, a.chaud.z));
    mat.set_uniform("froid", (a.froid.x, a.froid.y, a.froid.z));
    mat.set_uniform("eclat", a.eclat);
    mat.set_uniform("exp_couleur", a.exposant_couleur);
    mat.set_uniform("exp_opacite", a.exposant_opacite);
    mat.set_uniform("beta", a.beta);
    mat.set_uniform("beaming_max", a.beaming_max.max(1.0));
    mat.set_uniform("cellules", (a.cellules.x, a.cellules.y));
    mat.set_uniform("spirale", a.spirale);
    mat.set_uniform("turbulence", a.turbulence);
    mat.set_uniform("rotation", a.rotation);
    mat.set_uniform("r_ratio", (c.externe / c.interne.max(1e-3)).max(1.001));
    mat.set_uniform("bord_int", a.bord_interne);
    mat.set_uniform("bord_ext", a.bord_externe);
    mat.set_uniform("pli", passe.pli.0);
    mat.set_uniform("pli_signe", passe.pli.1);
    mat.set_uniform("interne", c.interne);
    mat.set_uniform("decalage", passe.decalage * c.interne);
    mat.set_uniform("couche", passe.couche);
    mat.set_uniform("halo", passe.halo);
    mat.set_uniform("anneau", passe.anneau);
}

/// **Déclaration des uniforms. Source unique** : sert à créer le material,
/// et de référence au test de correspondance avec le `.glsl`.
fn uniforms_accretion_decl() -> Vec<UniformDesc> {
    let f1 = |n: &str| UniformDesc::new(n, UniformType::Float1);
    let f3 = |n: &str| UniformDesc::new(n, UniformType::Float3);
    vec![
        f1("time"), f1("seed"),
        f3("centre"), f3("axe_u"), f3("axe_n"), f3("axe_v"), f3("tocam"),
        f1("moitie"), f1("alpha"),
        f3("chaud"), f3("froid"), f1("eclat"),
        f1("exp_couleur"), f1("exp_opacite"),
        f1("beta"), f1("beaming_max"),
        UniformDesc::new("cellules", UniformType::Float2),
        f1("spirale"), f1("turbulence"), f1("rotation"), f1("r_ratio"),
        f1("bord_int"), f1("bord_ext"),
        f1("pli"), f1("pli_signe"), f1("interne"), f1("decalage"),
        f1("couche"), f1("halo"), f1("anneau"),
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
    /// superposeraient — un seul arc, deux fois trop brillant en additif ;
    /// et plier la moitié avant décollerait le disque de ses propres bords,
    /// puisqu'elle n'a jamais été cachée.
    #[test]
    fn seule_la_moitie_arriere_se_dedouble() {
        let a = Accretion::chaude();
        let signes: Vec<f32> =
            passes(&a, -1.0).iter().filter(|p| p.pli.0 > 0.0).map(|p| p.pli.1).collect();
        assert!(signes.iter().any(|s| *s > 0.0), "aucun arc au-dessus");
        assert!(signes.iter().any(|s| *s < 0.0), "aucun arc au-dessous");
        for p in passes(&a, 1.0) {
            assert_eq!(p.pli.0, 0.0, "la moitié avant a été pliée");
        }
    }

    /// **Empiler des nappes sans diviser l'éclat part en burn-out.**
    ///
    /// C'est l'erreur nommée par les auteurs de ces shaders : en blending
    /// additif, des couches superposées finissent en blanc plat qui détruit
    /// la silhouette. La somme des éclats doit rester celle d'une nappe.
    #[test]
    fn empiler_les_nappes_ne_brule_pas_l_image() {
        for nappes in [1_usize, 2, 3, 5] {
            let a = Accretion { nappes, ..Accretion::chaude() };
            let somme: f32 = passes(&a, 1.0)
                .iter()
                .filter(|p| p.halo == 0.0 && p.anneau == 0.0)
                .map(|p| p.couche)
                .sum();
            assert!(
                (somme - 1.0).abs() < 1e-5,
                "{nappes} nappes somment à {somme} : l'arc brûle ou s'éteint"
            );
        }
    }

    /// **Les nappes encadrent le plan**, au lieu d'être toutes du même
    /// côté : sinon le disque n'a pas d'épaisseur, il est décalé.
    #[test]
    fn les_nappes_encadrent_le_plan() {
        let a = Accretion { nappes: 3, ..Accretion::chaude() };
        let d: Vec<f32> = passes(&a, 1.0)
            .iter()
            .filter(|p| p.halo == 0.0 && p.anneau == 0.0)
            .map(|p| p.decalage)
            .collect();
        assert!(d.iter().any(|x| *x > 0.0) && d.iter().any(|x| *x < 0.0), "{d:?}");
        assert!(d.iter().sum::<f32>().abs() < 1e-5, "nappes décentrées : {d:?}");
        let seule = Accretion { nappes: 1, ..Accretion::chaude() };
        assert_eq!(passes(&seule, 1.0)[0].decalage, 0.0);
    }

    /// **Le halo et l'anneau ne se posent qu'une fois, devant.** Les poser
    /// aussi derrière les dédoublerait — et l'anneau de lentille est déjà
    /// une image de la face arrière : le tracer deux fois le compterait
    /// double.
    #[test]
    fn le_halo_et_l_anneau_ne_sont_poses_qu_une_fois() {
        let a = Accretion::chaude();
        let compte = |m: f32, f: fn(&Passe) -> f32| {
            passes(&a, m).iter().filter(|p| f(p) > 0.0).count()
        };
        assert_eq!(compte(1.0, |p| p.halo), 1, "halo absent ou dupliqué devant");
        assert_eq!(compte(1.0, |p| p.anneau), 1, "anneau absent ou dupliqué devant");
        assert_eq!(compte(-1.0, |p| p.halo), 0, "halo dupliqué derrière");
        assert_eq!(compte(-1.0, |p| p.anneau), 0, "anneau dupliqué derrière");
    }

    /// **La teinte chaude est ordonnée `R > G > B`.**
    ///
    /// Sous blending additif on ne peut pas tone-mapper la somme : elle
    /// s'accumule dans le framebuffer, après le shader. Le clipping par
    /// canal en tient lieu — le rouge sature d'abord, puis le vert, puis le
    /// bleu, et le cœur vire au blanc. Une teinte bleutée saturerait vers le
    /// **cyan** : c'était notre défaut.
    #[test]
    fn la_teinte_chaude_sature_vers_le_blanc_et_non_le_cyan() {
        let a = Accretion::chaude();
        let c = a.chaud;
        assert!(c.x >= c.y && c.y >= c.z, "teinte chaude mal ordonnée : {c:?}");
        assert!(c.x > c.z, "teinte chaude plate : le clipping ne fera rien");
        assert!(a.eclat > 1.0, "éclat sous 1 : rien ne saturera jamais");
    }

    /// **Le budget de sommets reste tenable.** L'annulus fait 960 quads et
    /// chaque passe le redessine ; le disque doit rester très en deçà d'un
    /// vaisseau (25 600 sommets).
    #[test]
    fn le_disque_reste_dans_son_budget_de_sommets() {
        let a = Accretion::chaude();
        let n = passes(&a, -1.0).len() + passes(&a, 1.0).len();
        let sommets = n * 960 * 4;
        assert!(sommets < 32_000, "{n} passes, {sommets} sommets par frame");
    }

}
