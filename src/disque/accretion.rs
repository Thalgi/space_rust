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
    /// **Bornes du disque, en rayons de l'ombre.** Portées par le preset et
    /// non par l'astre : c'est la taille du disque qui distingue un trou noir
    /// endormi d'un trou noir vorace, au même titre que son éclat.
    ///
    /// ⚠️ Le bord interne suit le rapport `r_ISCO / r_ombre = 6/5,196 =
    /// 1,155`. Le repousser plus loin transforme le disque en anneau
    /// lointain autour d'une bille.
    pub bornes: (f32, f32),
    /// **Ceinture de débris épars**, en rayons de l'ombre, quand il y en a
    /// une. Un second disque, bien plus large et fait de cailloux — pas du
    /// gaz brûlant du disque d'accrétion.
    pub ceinture: Option<(f32, f32)>,
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
            // ⚠️ Relevé de 0,35 : le bord interne mettait **18 secondes** à
            // faire un tour, ce qui ne se lit pas comme de la matière en
            // mouvement mais comme une image presque fixe. À 1,1 il en met
            // six, et la rotation différentielle en `r^-3/2` en donne
            // dix-huit au bord externe — c'est l'**écart** entre les deux
            // qui se voit, pas la vitesse absolue.
            rotation: 1.1,
            bord_interne: 0.015,
            bord_externe: 0.30,
            // ⚠️ Cette valeur a bougé deux fois, et la seconde était une
            // surcorrection : en retirant le facteur d'inclinaison du
            // soulèvement (qui l'effondrait dès qu'on regardait le disque
            // d'un peu haut), j'ai **aussi** monté l'amplitude — deux gestes
            // dans le même sens, et l'arc est parti trop haut au-dessus de
            // l'horizon. Le premier geste suffisait.
            //
            // La cible : l'arc **frôle** le sommet de l'ombre. L'image
            // lentillée de la face arrière se forme au voisinage du rayon
            // critique, elle ne s'envole pas — un arc trop haut se lit comme
            // un panache, plus comme un anneau qui enjambe.
            pli: 0.72,
            epaisseur: 0.07,
            nappes: 3,
            halo: 0.10,
            anneau: 2.4,
            bornes: (1.155, 3.0),
            ceinture: None,
        }
    }

    /// **Trou noir endormi** : il ne mange presque plus.
    ///
    /// Peu de matière, lente, et surtout **pas de gradient thermique** — un
    /// disque qui ne s'alimente plus refroidit et s'uniformise. Ce qui reste
    /// de plus visible est son **halo**, pâle et large : le peu de gaz qu'il
    /// lui reste diffuse plus qu'il ne rayonne. Le beaming s'efface avec la
    /// vitesse.
    pub fn endormie() -> Self {
        Self {
            // Blanc froid, à peine teinté : plus de bord interne brûlant.
            chaud: vec3(1.0, 0.99, 0.97),
            froid: vec3(0.80, 0.80, 0.84),
            eclat: 0.85,
            exposant_couleur: 0.7,
            exposant_opacite: 0.5,
            beta: 0.12,
            beaming_max: 1.6,
            // Peu de structure : la turbulence s'éteint avec l'accrétion.
            cellules: vec2(10.0, 2.0),
            spirale: 1.2,
            turbulence: 0.30,
            rotation: 0.45,
            bord_interne: 0.05,
            bord_externe: 0.45,
            pli: 0.72,
            epaisseur: 0.05,
            nappes: 2,
            // **Le trait de cette variante.** Large et pâle, il domine un
            // disque qui n'a plus grand-chose à montrer.
            halo: 0.30,
            anneau: 1.6,
            bornes: (1.155, 2.4),
            ceinture: None,
        }
    }

    /// **Trou noir vorace** : il dévore, et ça se voit de loin.
    ///
    /// Disque plus large et plus brillant, turbulence marquée, beaming
    /// franc — et une **ceinture de débris épars** bien au-delà, reste de ce
    /// qu'il est en train d'avaler. Le disque brûle le gaz ; la ceinture,
    /// elle, est faite de cailloux qui n'ont pas encore spiralé.
    pub fn vorace() -> Self {
        Self {
            eclat: 2.4,
            beta: 0.45,
            beaming_max: 5.0,
            cellules: vec2(22.0, 4.0),
            spirale: 3.0,
            turbulence: 0.80,
            rotation: 1.4,
            nappes: 3,
            halo: 0.16,
            anneau: 2.8,
            bornes: (1.155, 4.6),
            // Très au-delà du disque : ce sont deux objets distincts, pas
            // un dégradé. Les confondre donnerait une galette unique.
            ceinture: Some((7.0, 13.0)),
            ..Self::chaude()
        }
    }

    /// **Le régime d'un trou noir, dérivé de sa graine.**
    ///
    /// ⚠️ Dérivé, jamais tiré : un `gen_range` de plus décalerait tout ce
    /// qui est tiré ensuite et changerait le système entier à graine égale —
    /// même piège que `champ::poles` et `temps_initial`.
    ///
    /// Le vorace est le plus rare : c'est un état de crise, pas un état
    /// d'équilibre.
    pub fn regime(graine: f32) -> Self {
        let u = (graine * 0.618_034).fract();
        if u < 0.35 {
            Self::endormie()
        } else if u < 0.80 {
            Self::chaude()
        } else {
            Self::vorace()
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
    /// **Cette passe couvre l'annulus ENTIER**, sans découpe en moitiés.
    ///
    /// ⚠️ Le halo et l'anneau de lentille sont des **couronnes complètes
    /// autour de l'astre** : les dessiner sur une seule moitié les coupait
    /// net le long de la ligne de partage, et cette **cassure horizontale**
    /// traversait l'image dès qu'on levait la caméra — signalé à l'écran.
    /// Ils se posent donc en une passe entière, **avant** l'astre, de sorte
    /// que l'horizon masque ce qui doit passer derrière lui.
    pub entier: bool,
}

impl Passe {
    fn nappe(pli: (f32, f32), decalage: f32, couche: f32) -> Self {
        Self { pli, decalage, couche, halo: 0.0, anneau: 0.0, entier: false }
    }

    /// La découpe en moitiés que le shader doit appliquer : `0` = aucune.
    pub fn moitie(&self, demandee: f32) -> f32 {
        if self.entier {
            0.0
        } else {
            demandee
        }
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
    // ⚠️ Halo et anneau de lentille : **une seule passe, entière, derrière**.
    // Entière parce que ce sont des couronnes complètes — découpées en
    // moitiés, elles s'arrêtaient net sur la ligne de partage et barraient
    // l'image d'une cassure horizontale. Derrière parce que l'astre est
    // dessiné entre les deux moitiés : l'horizon masque alors la part qui
    // doit passer derrière lui, au lieu de la peindre par-dessus.
    if moitie < 0.0 {
        if a.halo > 0.0 {
            out.push(Passe { halo: a.halo, entier: true, ..Passe::nappe((0.0, 1.0), 0.0, 1.0) });
        }
        if a.anneau > 0.0 {
            out.push(Passe { anneau: a.anneau, entier: true, ..Passe::nappe((0.0, 1.0), 0.0, 1.0) });
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
    mat.set_uniform("moitie", passe.moitie(moitie));
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
    use crate::impostor::verif::{
        uniforms_glsl, uniforms_inutilises, uniforms_rust, usage_avant_declaration,
    };

    /// **Aucune variable n'est employée avant d'être déclarée.**
    ///
    /// GLSL n'a pas de déclaration anticipée : c'est une erreur de
    /// compilation. Or **rien ici ne compile le GLSL** — il faut un contexte
    /// graphique — donc le défaut ne se voit qu'au lancement du jeu, en
    /// panic au chargement du material. C'est arrivé sur `ouvert`, déclaré
    /// au soulèvement et employé dix lignes plus haut par la compression.
    #[test]
    fn aucune_variable_n_est_employee_avant_d_etre_declaree() {
        for (nom, src) in [("vertex", VERT), ("fragment", FRAG)] {
            let fautes = usage_avant_declaration(src);
            assert!(fautes.is_empty(), "{nom} : {fautes:?}");
        }
    }

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

    /// **Le halo et l'anneau : une seule fois, et ENTIERS.**
    ///
    /// Une fois, parce que les poser des deux côtés les compterait double —
    /// en additif, deux fois l'éclat. Entiers, parce que ce sont des
    /// **couronnes complètes** autour de l'astre : découpés en moitiés, ils
    /// s'arrêtaient net sur la ligne de partage et barraient l'image d'une
    /// cassure horizontale dès qu'on levait la caméra.
    #[test]
    fn le_halo_et_l_anneau_sont_poses_une_fois_et_entiers() {
        let a = Accretion::chaude();
        let toutes: Vec<Passe> = passes(&a, -1.0).into_iter().chain(passes(&a, 1.0)).collect();
        let halo = (|p: &Passe| p.halo) as fn(&Passe) -> f32;
        let anneau = (|p: &Passe| p.anneau) as fn(&Passe) -> f32;
        for (nom, f) in [("halo", halo), ("anneau", anneau)] {
            let poses: Vec<&Passe> = toutes.iter().filter(|p| f(p) > 0.0).collect();
            assert_eq!(poses.len(), 1, "{nom} : posé {} fois sur les deux moitiés", poses.len());
            assert!(poses[0].entier, "{nom} découpé en moitié : il barrera l'image");
            // Et la découpe demandée est bien neutralisée.
            assert_eq!(poses[0].moitie(-1.0), 0.0);
            assert_eq!(poses[0].moitie(1.0), 0.0);
        }
        // Les nappes du disque, elles, restent découpées : c'est l'astre qui
        // s'intercale entre les deux.
        for p in toutes.iter().filter(|p| p.halo == 0.0 && p.anneau == 0.0) {
            assert_eq!(p.moitie(-1.0), -1.0, "une nappe du disque a perdu sa découpe");
        }
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

    /// **Les trois régimes se distinguent à l'œil**, et dans le bon sens.
    ///
    /// Un endormi qui ressemblerait à un vorace ne servirait à rien : c'est
    /// une déclinaison, pas trois réglages au hasard. L'ordre compte —
    /// l'endormi mange moins, donc brille moins, tourne moins vite, a moins
    /// de structure et un disque plus petit ; son **halo**, lui, est le seul
    /// trait qui grandit quand l'accrétion faiblit.
    #[test]
    fn les_trois_regimes_vont_du_plus_calme_au_plus_vorace() {
        let (d, c, v) = (Accretion::endormie(), Accretion::chaude(), Accretion::vorace());
        for (nom, f) in [
            ("éclat", (|a: &Accretion| a.eclat) as fn(&Accretion) -> f32),
            ("rotation", |a| a.rotation),
            ("turbulence", |a| a.turbulence),
            ("beaming", |a| a.beaming_max),
            ("étendue", |a| a.bornes.1),
        ] {
            assert!(f(&d) < f(&c), "{nom} : l'endormi n'est pas en deçà de l'actif");
            assert!(f(&c) < f(&v), "{nom} : l'actif n'est pas en deçà du vorace");
        }
        // ⚠️ Le halo va à **contre-courant** : moins l'astre accrète, plus
        // ce qui lui reste diffuse au lieu de rayonner. C'est le trait qui
        // fait reconnaître un trou noir endormi.
        assert!(d.halo > c.halo, "l'endormi n'a pas son halo pâle et large");
        // Et lui seul n'a plus de gradient thermique marqué : un disque qui
        // ne s'alimente plus refroidit et s'uniformise.
        assert!(d.exposant_couleur < c.exposant_couleur);
        // Seul le vorace traîne une ceinture.
        assert!(d.ceinture.is_none() && c.ceinture.is_none());
        assert!(v.ceinture.is_some(), "le vorace n'a pas sa ceinture");
    }

    /// **Le régime se dérive de la graine, et les trois sortent.**
    ///
    /// ⚠️ Dérivé, jamais tiré : un `gen_range` de plus décalerait tout ce
    /// qui est tiré ensuite, et le système entier changerait à graine égale.
    /// Et un régime qu'aucune graine n'atteint est un régime qui n'existe
    /// pas — le piège exact qui tenait le trou noir hors du jeu.
    #[test]
    fn chaque_regime_est_atteignable_et_reproductible() {
        assert_eq!(Accretion::regime(12.5).eclat, Accretion::regime(12.5).eclat);
        let mut vus = [0_usize; 3];
        for k in 0..600 {
            let a = Accretion::regime(k as f32 * 1.37);
            let rang = if a.ceinture.is_some() {
                2
            } else if a.halo > Accretion::chaude().halo {
                0
            } else {
                1
            };
            vus[rang] += 1;
        }
        for (i, n) in vus.iter().enumerate() {
            assert!(*n > 30, "le régime {i} ne sort que {n} fois sur 600 : introuvable");
        }
        // Le vorace reste le plus rare : c'est un état de crise.
        assert!(vus[2] < vus[1], "le vorace est plus courant que l'actif");
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
