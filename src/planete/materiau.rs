use super::apparence::Apparence;
use crate::astre::CameraInfo;
use crate::impostor;
use macroquad::miniquad::{BlendFactor, BlendState, Equation};
use macroquad::prelude::*;
use std::cell::RefCell;

/// Demi-étendue du disque impostor planète : le rayon du corps PLUS la coquille
/// d'atmosphère. **Source unique** — le quad (`mod.rs`) et l'uniform `disc` lu
/// par les shaders doivent valoir la même chose, sinon la coquille est ou bien
/// coupée par le bord du quad, ou bien dessinée dans le vide.
///
/// 1.10 : la coquille fait 10 % du rayon. Portée à ce double après avoir vu le
/// rendu — à 5 % le liseré était juste, mais trop mince pour se lire, surtout
/// sur les corps à atmosphère ténue. Marge de cadrage : la galerie et les
/// vignettes regardent de 3 r, où le champ fait 1,243 r de demi-hauteur ; la
/// coquille en occupe 89 %, donc elle tient encore dans la case.
pub(super) const DISC: f32 = 1.10;

/// Épaisseur minimale (px) au-delà de laquelle la coquille atmosphérique mérite
/// sa passe de rendu. Sous ce seuil elle tient dans moins d'un pixel : invisible,
/// et une passe GPU gâchée — la vue système aligne des dizaines de corps qui ne
/// font que quelques pixels.
const ATMO_PX_MIN: f32 = 0.75;

/// Une valeur d'uniform : on n'a besoin que de scalaires et de vec3.
enum Val {
    F1(f32),
    F3(Vec3),
}

/// Une entrée de [`TABLE`] : nom GLSL, type, et comment la lire dans l'apparence.
type Entree = (&'static str, UniformType, fn(&Apparence) -> Val);

/// Table déclarative des uniforms pilotés par l'`Apparence`. Source unique : sert
/// à la fois à déclarer les uniforms (descripteurs) et à les pousser chaque frame.
/// Ajouter un paramètre visuel = une seule ligne ici (puis l'utiliser dans le .glsl).
const TABLE: &[Entree] = &[
    ("couleur", UniformType::Float3, |a| Val::F3(a.couleur)),
    ("couleur2", UniformType::Float3, |a| Val::F3(a.couleur2)),
    ("couleur3", UniformType::Float3, |a| Val::F3(a.couleur3)),
    ("type_p", UniformType::Float1, |a| Val::F1(a.type_p.code())),
    ("eau", UniformType::Float1, |a| Val::F1(a.eau)),
    ("tache_col", UniformType::Float3, |a| Val::F3(a.tache_couleur)),
    ("axe", UniformType::Float3, |a| Val::F3(a.axe)),
    ("rot_vitesse", UniformType::Float1, |a| Val::F1(a.rot_vitesse)),
    // Géométrie de l'anneau : sert à projeter son OMBRE sur la planète. Elle
    // servait déjà à construire le disque, mais n'atteignait pas ce shader-ci.
    ("anneau", UniformType::Float1, |a| Val::F1(if a.anneau { 1.0 } else { 0.0 })),
    ("anneau_in", UniformType::Float1, |a| Val::F1(a.anneau_in)),
    ("anneau_out", UniformType::Float1, |a| Val::F1(a.anneau_out)),
    ("anneau_normal", UniformType::Float3, |a| Val::F3(a.anneau_normal)),
    ("warp_amt", UniformType::Float1, |a| Val::F1(a.warp_amt)),
    ("seed", UniformType::Float1, |a| Val::F1(a.seed)),
    ("poly_cotes", UniformType::Float1, |a| Val::F1(a.poly_cotes)),
    // Pas de teinte d'air ici : le corps ne peint plus le halo, il lui suffit de
    // savoir qu'il y a de l'air (hydrologie, nuages). La teinte ne part qu'à la
    // coquille — voir `uniforms_atmo`.
    ("atmo_densite", UniformType::Float1, |a| Val::F1(a.atmo_densite)),
    ("lave", UniformType::Float1, |a| Val::F1(a.lave)),
    ("eau_motif", UniformType::Float1, |a| Val::F1(a.eau_motif)),
    ("grad_lat", UniformType::Float1, |a| Val::F1(a.grad_lat)),
    ("calotte", UniformType::Float1, |a| Val::F1(a.calotte)),
    ("veg_couleur", UniformType::Float3, |a| Val::F3(a.veg_couleur)),
    ("veg_couv", UniformType::Float1, |a| Val::F1(a.veg_couv)),
    ("rivieres", UniformType::Float1, |a| Val::F1(a.rivieres)),
    ("nuages", UniformType::Float1, |a| Val::F1(a.nuages)),
    ("nuages_couleur", UniformType::Float3, |a| Val::F3(a.nuages_couleur)),
    ("nuages_type", UniformType::Float1, |a| Val::F1(a.nuages_type)),
    ("cyclones_nb", UniformType::Float1, |a| Val::F1(a.cyclones_nb)),
    ("relief", UniformType::Float1, |a| Val::F1(a.relief)),
    ("dunes", UniformType::Float1, |a| Val::F1(a.dunes)),
    ("mesa", UniformType::Float1, |a| Val::F1(a.mesa)),
    ("pics", UniformType::Float1, |a| Val::F1(a.pics)),
    ("recifs", UniformType::Float1, |a| Val::F1(a.recifs)),
    ("basalt", UniformType::Float1, |a| Val::F1(a.basalt)),
    ("voile", UniformType::Float1, |a| Val::F1(a.voile)),
    ("voile_couleur", UniformType::Float3, |a| Val::F3(a.voile_couleur)),
    ("crateres", UniformType::Float1, |a| Val::F1(a.crateres)),
    ("eyeball", UniformType::Float1, |a| Val::F1(a.eyeball)),
    ("eye_glace", UniformType::Float1, |a| Val::F1(a.eye_glace)),
    ("eye_lave", UniformType::Float1, |a| Val::F1(a.eye_lave)),
    ("eye_ring", UniformType::Float1, |a| Val::F1(a.eye_ring)),
    ("cryo", UniformType::Float1, |a| Val::F1(a.cryo)),
    ("biolum", UniformType::Float1, |a| Val::F1(a.biolum)),
    ("riv_lave", UniformType::Float1, |a| Val::F1(a.riv_lave)),
    ("villes", UniformType::Float1, |a| Val::F1(a.villes)),
    ("cyclones_pol", UniformType::Float1, |a| Val::F1(a.cyclones_pol)),
    ("thermique", UniformType::Float1, |a| Val::F1(a.thermique)),
    ("thermique_couleur", UniformType::Float3, |a| Val::F3(a.thermique_couleur)),
    ("aurore", UniformType::Float1, |a| Val::F1(a.aurore)),
    ("aurore_couleur", UniformType::Float3, |a| Val::F3(a.aurore_couleur)),
    ("brume", UniformType::Float1, |a| Val::F1(a.brume)),
    ("brume_couleur", UniformType::Float3, |a| Val::F3(a.brume_couleur)),
    ("g_pole", UniformType::Float3, |a| Val::F3(a.g_pole)),
];

fn set_val(mat: &Material, nom: &str, v: Val) {
    match v {
        Val::F1(x) => mat.set_uniform(nom, x),
        Val::F3(p) => mat.set_uniform(nom, (p.x, p.y, p.z)),
    }
}

// Materials partagés (créés une seule fois, clonés ensuite) : `Material` est un
// `Arc` sur le pipeline, donc un clone n'en consomme pas un de plus. macroquad
// en plafonne le nombre à 32 -> sans ce cache, « Pipelines amount exceeded ».
//
// ⚠️ Un clone partage AUSSI l'état des uniforms : `set_uniform` écrit dans le
// pipeline, pas dans le clone. D'où le vidage de lot après chaque corps dessiné
// (voir `mod.rs`), sans quoi le dernier dessiné impose ses réglages aux autres.
thread_local! {
    static TPL_CORPS: RefCell<Option<Material>> = const { RefCell::new(None) };
    static TPL_ATMO: RefCell<Option<Material>> = const { RefCell::new(None) };
}

/// Indique la hauteur du viewport de rendu courant (galerie : hauteur de
/// cellule, éventuellement divisée par le facteur pixel). 0 = plein écran.
///
/// La valeur vit dans `impostor` : corps, disques et étoiles la lisaient dans
/// des cellules séparées qu'il fallait régler en cascade.
pub(super) fn set_viewport_h(h: f32) {
    impostor::set_viewport_h(h);
}

pub(super) fn mat_corps() -> Material {
    TPL_CORPS.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_corps());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

pub(super) fn mat_atmo() -> Material {
    TPL_ATMO.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(charger_atmo());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// Vide le cache de materials -> la prochaine création recompile depuis les .glsl
/// (utilisé par le hot-reload).
pub(super) fn vider_cache() {
    TPL_CORPS.with(|c| *c.borrow_mut() = None);
    TPL_ATMO.with(|c| *c.borrow_mut() = None);
}

/// Rayon apparent (px) du corps dans le viewport courant. La formule est
/// partagée : voir [`impostor::px_rayon`].
pub(super) fn px_rayon(cam: &CameraInfo, c: Vec3, r: f32) -> f32 {
    impostor::px_rayon(r, (cam.pos - c).length(), impostor::viewport_h())
}

/// Vrai si la coquille atmosphérique mérite sa passe de rendu : le corps doit
/// avoir une atmosphère, et celle-ci doit couvrir au moins un pixel à l'écran.
fn atmo_visible_pur(atmo_densite: f32, px_rayon: f32) -> bool {
    atmo_densite > 0.0 && px_rayon * (DISC - 1.0) >= ATMO_PX_MIN
}

/// Voir [`atmo_visible_pur`] : même décision, mesures prises sur la scène.
pub(super) fn atmo_visible(a: &Apparence, cam: &CameraInfo, c: Vec3, r: f32) -> bool {
    atmo_visible_pur(a.atmo_densite, px_rayon(cam, c, r))
}

// Texture de secours (1×1) : corps sans terrain précalculé (gazeuses, glacées)
// et telluriques dont la génération asynchrone n'a pas fini (placeholder :
// h ≈ 0.55 -> sol plat uni, ni océan global ni noir).
thread_local! {
    static TEX_VIDE: RefCell<Option<Texture2D>> = const { RefCell::new(None) };
}

fn tex_vide() -> Texture2D {
    TEX_VIDE.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(Texture2D::from_rgba8(1, 1, &[140, 0, 0, 128]));
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// Pousse tous les uniforms : communs (socle) + dynamiques (caméra/corps) + table.
/// `terrain` : atlas cube-sphere précalculé + niveau de mer (telluriques).
///
/// Huit arguments — matériau, apparence, caméra, pose, et les trois textures
/// optionnelles : la liste est **imposée par le shader**, la regrouper ne ferait
/// que la déplacer d'un cran.
#[allow(clippy::too_many_arguments)]
pub(super) fn appliquer_uniforms(
    mat: &Material,
    a: &Apparence,
    cam: &CameraInfo,
    c: Vec3,
    r: f32,
    terrain: Option<(&Texture2D, f32)>,
    zonal: Option<(&Texture2D, f32)>,
    vortex: Option<&([Vec4; super::vortex::N_VORTEX], [Vec4; super::vortex::N_VORTEX])>,
) {
    impostor::uniforms_standard(mat, cam, get_time() as f32, DISC);
    // **Positions au repère de rendu.** `c` et les sources de lumière sont des
    // points monde ; le shader les compare au rayon issu du sommet, qui arrive
    // lui aussi translaté de `cam.origine`. Les laisser en absolu ferait
    // reconstruire la sphère à 460 unités du quad. Aucune ligne de GLSL ne
    // change pour autant : les uniformes sont déjà des `f32`, on leur passe
    // simplement de petits nombres (`docs/conception/repere_flottant.md`).
    let centre = cam.rel(c);
    let lumiere = cam.rel(cam.light_pos);
    mat.set_uniform("centre", (centre.x, centre.y, centre.z));
    mat.set_uniform("rayon", r);
    // LOD (phase 6) : le shader estompe le micro-détail des gazeuses sur les
    // petites sphères (galerie, vues système). Mesuré en MONDE (`cam.pos` est
    // absolu, `c` aussi) : la distance ne dépend pas du repère de rendu.
    mat.set_uniform("px_rayon", px_rayon(cam, c, r));
    mat.set_uniform("lumiere", (lumiere.x, lumiere.y, lumiere.z));
    mat.set_uniform(
        "light_color",
        (cam.light_color.x, cam.light_color.y, cam.light_color.z),
    );
    // Éclairage multi-source (jusqu'à 4 étoiles ; indice 0 = primaire).
    mat.set_uniform_array("lights_pos", &cam.lights_rel()[..]);
    mat.set_uniform_array("lights_color", &cam.lights_color[..]);
    // Palette paramétrique des gazeuses : 8 teintes dérivées des couleurs du
    // preset (voir palette.rs) -> plus de couleurs codées en dur dans le .glsl.
    mat.set_uniform_array("gaz_pal", &super::palette::gaz_palette(a)[..]);
    match terrain {
        Some((tex, niveau)) => {
            mat.set_texture("terrain", tex.clone());
            mat.set_uniform("niveau_mer", niveau);
            mat.set_uniform("atlas_n", super::terrain::N_ATLAS as f32);
        }
        None => {
            mat.set_texture("terrain", tex_vide());
            mat.set_uniform("niveau_mer", 0.5f32);
            mat.set_uniform("atlas_n", 1.0f32);
        }
    }
    // Profil zonal 1D (gazeuses) : jets, type de bande, cisaillement, et borne
    // polaire pole_lat (zonal.rs).
    match zonal {
        Some((tex, pl)) => {
            mat.set_texture("zonal", tex.clone());
            mat.set_uniform("pole_lat", pl);
        }
        None => {
            mat.set_texture("zonal", tex_vide());
            mat.set_uniform("pole_lat", 0.9f32);
        }
    }
    // Slots de vortex (gazeuses, vortex.rs) ; zéros = tous inactifs.
    const VORTEX_ZERO: [Vec4; super::vortex::N_VORTEX] = [Vec4::ZERO; super::vortex::N_VORTEX];
    let (va, vb) = match vortex {
        Some((a1, a2)) => (&a1[..], &a2[..]),
        None => (&VORTEX_ZERO[..], &VORTEX_ZERO[..]),
    };
    mat.set_uniform_array("vortex", va);
    mat.set_uniform_array("vortex2", vb);
    for (nom, _, get) in TABLE {
        set_val(mat, nom, get(a));
    }
}

/// Pousse les uniforms de la coquille atmosphérique. Beaucoup plus court que
/// ceux du corps : la coquille ne connaît ni terrain, ni bandes, ni vortex —
/// seulement sa géométrie, l'éclairage, et la couleur de l'air.
pub(super) fn appliquer_uniforms_atmo(mat: &Material, a: &Apparence, cam: &CameraInfo, c: Vec3, r: f32) {
    impostor::uniforms_standard(mat, cam, get_time() as f32, DISC);
    // Mêmes positions relatives que le corps : la coquille se pose sur le
    // limbe, elle doit partager exactement son repère de rendu.
    let centre = cam.rel(c);
    let lumiere = cam.rel(cam.light_pos);
    mat.set_uniform("centre", (centre.x, centre.y, centre.z));
    mat.set_uniform("rayon", r);
    mat.set_uniform("lumiere", (lumiere.x, lumiere.y, lumiere.z));
    mat.set_uniform(
        "light_color",
        (cam.light_color.x, cam.light_color.y, cam.light_color.z),
    );
    mat.set_uniform_array("lights_pos", &cam.lights_rel()[..]);
    mat.set_uniform_array("lights_color", &cam.lights_color[..]);
    mat.set_uniform("atmo", (a.atmo.x, a.atmo.y, a.atmo.z));
    mat.set_uniform("atmo_densite", a.atmo_densite);
    mat.set_uniform("voile", a.voile);
    mat.set_uniform(
        "voile_couleur",
        (a.voile_couleur.x, a.voile_couleur.y, a.voile_couleur.z),
    );
}

/// Déclaration des uniforms de la coquille atmosphérique. **Source unique** :
/// sert à créer le material, et de référence au test de correspondance avec le
/// `.glsl` — un uniform lu par le shader mais absent d'ici vaut silencieusement
/// zéro, et la coquille disparaît sans le moindre message d'erreur.
fn uniforms_atmo() -> Vec<UniformDesc> {
    let mut u = impostor::uniforms_communs();
    u.extend([
        UniformDesc::new("centre", UniformType::Float3),
        UniformDesc::new("rayon", UniformType::Float1),
        UniformDesc::new("lumiere", UniformType::Float3),
        UniformDesc::new("light_color", UniformType::Float3),
        UniformDesc::array(UniformDesc::new("lights_pos", UniformType::Float3), 4),
        UniformDesc::array(UniformDesc::new("lights_color", UniformType::Float3), 4),
        UniformDesc::new("atmo", UniformType::Float3),
        UniformDesc::new("atmo_densite", UniformType::Float1),
        UniformDesc::new("voile", UniformType::Float1),
        UniformDesc::new("voile_couleur", UniformType::Float3),
    ]);
    u
}

/// Crée le material de la coquille atmosphérique (une seule fois, puis cloné).
///
/// Additif et **sans écriture de profondeur** : l'atmosphère AJOUTE de la
/// lumière au fond du ciel comme à la surface, et ne doit rien occulter — la
/// moitié avant d'un anneau, dessinée juste après, passe devant elle.
fn charger_atmo() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR),
            fragment: &impostor::source("atmosphere.frag.glsl", FRAG_ATMO),
        },
        MaterialParams {
            uniforms: uniforms_atmo(),
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

/// Déclaration des uniforms du corps. Même rôle de source unique que
/// [`uniforms_atmo`].
fn uniforms_corps() -> Vec<UniformDesc> {
    let mut uniforms = impostor::uniforms_communs();
    uniforms.extend([
        UniformDesc::new("centre", UniformType::Float3),
        UniformDesc::new("rayon", UniformType::Float1),
        UniformDesc::new("lumiere", UniformType::Float3),
        UniformDesc::new("light_color", UniformType::Float3),
        UniformDesc::array(UniformDesc::new("lights_pos", UniformType::Float3), 4),
        UniformDesc::array(UniformDesc::new("lights_color", UniformType::Float3), 4),
        UniformDesc::array(
            UniformDesc::new("gaz_pal", UniformType::Float3),
            super::palette::GAZ_PAL_N,
        ),
        UniformDesc::array(
            UniformDesc::new("vortex", UniformType::Float4),
            super::vortex::N_VORTEX,
        ),
        UniformDesc::array(
            UniformDesc::new("vortex2", UniformType::Float4),
            super::vortex::N_VORTEX,
        ),
        UniformDesc::new("niveau_mer", UniformType::Float1),
        UniformDesc::new("atlas_n", UniformType::Float1),
        UniformDesc::new("pole_lat", UniformType::Float1),
        UniformDesc::new("px_rayon", UniformType::Float1),
    ]);
    uniforms.extend(TABLE.iter().map(|(nom, t, _)| UniformDesc::new(nom, *t)));
    uniforms
}

/// Crée le material du corps des planètes (appelé une seule fois, puis cloné).
fn charger_corps() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("impostor.vert.glsl", impostor::VERT_IMPOSTOR),
            fragment: &impostor::source("planete.frag.glsl", FRAG),
        },
        MaterialParams {
            uniforms: uniforms_corps(),
            textures: vec!["terrain".to_string(), "zonal".to_string()],
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            },
        },
    )
    .unwrap()
}

const FRAG: &str = include_str!("../shaders/planete.frag.glsl");
const FRAG_ATMO: &str = include_str!("../shaders/atmosphere.frag.glsl");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::impostor::verif::{uniforms_glsl, uniforms_inutilises, uniforms_rust};

    /// Un uniform lu par le `.glsl` mais absent de la déclaration Rust vaut
    /// silencieusement zéro : le shader s'exécute, aucun message n'apparaît, et
    /// l'effet disparaît. La réciproque (déclaré mais jamais lu) est du poids
    /// mort qui laisse croire qu'un réglage agit. Les deux listes sont écrites
    /// séparément — c'est ce qui donne sa valeur à la comparaison.
    #[test]
    fn uniforms_atmo_correspondent_au_shader() {
        let glsl = uniforms_glsl(&[impostor::VERT_IMPOSTOR, FRAG_ATMO]);
        let rust = uniforms_rust(&uniforms_atmo());
        assert_eq!(glsl, rust, "coquille atmosphérique : GLSL (gauche) vs Rust (droite)");
    }

    #[test]
    fn uniforms_corps_correspondent_au_shader() {
        let glsl = uniforms_glsl(&[impostor::VERT_IMPOSTOR, FRAG]);
        let rust = uniforms_rust(&uniforms_corps());
        assert_eq!(glsl, rust, "corps planétaire : GLSL (gauche) vs Rust (droite)");
    }

    /// **Un uniform déclaré doit être LU.**
    ///
    /// Déclaré des deux côtés mais ignoré par le shader, c'est un réglage qui ne
    /// fait rien : la valeur part du Rust, arrive au GPU, et n'a aucun effet. Le
    /// test de correspondance ne le voit pas — il ne compare que des listes de
    /// déclarations.
    ///
    /// C'est le travers le plus tenace de ce projet (calotte, villes, atmo de
    /// Titan, texture zonale, ventre du Starship, éruptions stellaires) : *la
    /// donnée est juste, la plomberie ne suit pas*. Il ne se lit ni d'un côté ni
    /// de l'autre — seulement en croisant les deux.
    ///
    /// ⚠️ **Portée exacte**, mesurée au red-check : il attrape « déclaré et
    /// jamais mentionné ». Il n'attrape PAS « mentionné dans du code mort » —
    /// débrancher le bloc de l'ombre d'anneau le laisse vert, puisque ses
    /// uniforms restent cités à l'intérieur. Le distinguer demanderait d'analyser
    /// l'atteignabilité du GLSL, hors de proportion ; le cas utile — un réglage
    /// branché sur rien — est bien couvert.
    #[test]
    fn aucun_uniform_du_corps_n_est_ignore() {
        let morts = uniforms_inutilises(&[FRAG]);
        assert!(morts.is_empty(), "déclarés mais jamais lus par le shader : {morts:?}");
    }

    /// Idem pour la coquille atmosphérique.
    ///
    /// `time` y est **volontairement** inutilisé : `impostor::uniforms_standard`
    /// le pose pour tous les corps impostor, et la coquille n'anime rien. On le
    /// tolère nommément plutôt que d'ouvrir une exception muette.
    #[test]
    fn aucun_uniform_de_la_coquille_n_est_ignore() {
        let mut morts = uniforms_inutilises(&[FRAG_ATMO]);
        morts.remove("time");
        assert!(morts.is_empty(), "déclarés mais jamais lus par la coquille : {morts:?}");
    }

    /// Le quad et l'uniform `disc` lisent la MÊME constante : sans quoi la
    /// coquille serait tronquée par le bord du quad (quad trop petit) ou
    /// dessinée sur du vide (quad trop grand).
    #[test]
    fn le_disque_laisse_de_la_place_a_la_coquille() {
        // Le quad doit dépasser le corps, sinon la coquille n'a nulle part où
        // exister. Trop peu et elle ne se voit pas ; trop et le corps flotte
        // dans un halo mou. Bornes du raisonnable, pas de la valeur choisie.
        assert!(
            (1.02..=1.20).contains(&DISC),
            "marge atmosphérique déraisonnable : {DISC}"
        );
    }

    #[test]
    fn pas_de_coquille_sans_atmosphere() {
        // Un corps sans air (Mercure, la Lune) ne doit jamais payer la passe,
        // même occupant tout l'écran.
        assert!(!atmo_visible_pur(0.0, 10_000.0));
    }

    #[test]
    fn pas_de_coquille_sous_le_pixel() {
        let air = 1.0;
        // Épaisseur = px_rayon × (DISC - 1). Le seuil est ATMO_PX_MIN pixels.
        let juste_au_seuil = ATMO_PX_MIN / (DISC - 1.0);
        assert!(atmo_visible_pur(air, juste_au_seuil * 1.01), "au seuil, la coquille se voit");
        assert!(
            !atmo_visible_pur(air, juste_au_seuil * 0.99),
            "sous le seuil, la coquille tient dans moins d'un pixel : passe inutile"
        );
    }

    /// Le LOD dépend de la hauteur du viewport, pas de celle de l'écran : la
    /// galerie dessine dans des cellules, et un corps y est bien plus petit
    /// qu'en plein écran à distance égale.
    #[test]
    fn le_rayon_apparent_suit_le_viewport_et_la_distance() {
        let px = impostor::px_rayon;
        let plein = px(1.0, 10.0, 1000.0);
        assert!((px(1.0, 10.0, 250.0) - plein / 4.0).abs() < 1e-4);
        assert!((px(1.0, 40.0, 1000.0) - plein / 4.0).abs() < 1e-4);
        // Caméra collée au corps : pas de division par zéro.
        assert!(px(1.0, 0.0, 1000.0).is_finite());
    }
}
