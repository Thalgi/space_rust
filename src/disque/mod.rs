//! Champ de débris unifié (ceintures, anneaux planétaires, disques proto*) —
//! voir docs/conception/ceintures.md. Principe : la géométrie est construite UNE
//! fois à la création (éléments orbitaux encodés dans les canaux de sommet),
//! puis les shaders animent via `time`. Zéro travail CPU par frame en dehors
//! des uniforms.
//!
//! Deux couches :
//! - **particules** : quads billboards, orbite képlérienne dans le vertex
//!   shader. Encodage : `position` = (phi, incl, r) ; `uv` = (coin + graine,
//!   taille) ; `color` = teinte.
//! - **voile** : annulus low-poly statique + fragment shader procédural
//!   (plateau, bandes/lacunes signées, granulation, arcs, émissif, rotation
//!   différentielle). Pour les anneaux planétaires, rendu en deux moitiés
//!   (uniform `moitie` + discard) autour du corps de la planète.

mod config;
mod voile;

pub use config::DisqueConfig;

use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase};
use crate::impostor;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::cell::{Cell, RefCell};

// La hauteur du viewport vit dans `impostor` (partagée avec corps et étoiles).
// Elle sert ici au clamp de taille MINIMALE des particules : une particule
// sub-pixel disparaît, on la fige à ~1.5 px en l'assombrissant — Kuiper/Oort
// restent visibles aux échelles réelles.
thread_local! {
    static PX_MIN: RefCell<f32> = const { RefCell::new(0.0) };
    // Materials partagés (créés une fois, clonés ensuite).
    //
    // **macroquad plafonne à 32 pipelines** (`quad_gl.rs`, `MAX_PIPELINES`), et
    // `load_material` en consomme un à chaque appel. Chaque `Disque` en créait
    // les siens : la galerie de débris en aligne douze, jusqu'à deux pipelines
    // chacun — le plafond sautait, `Pipelines amount exceeded`.
    //
    // Un clone de `Material` partage le pipeline (c'est un `Arc`) : douze
    // disques coûtent maintenant deux pipelines au lieu de vingt-quatre. En
    // contrepartie ils partagent aussi **l'état des uniforms** — d'où le vidage
    // de lot dans `draw_moitie`, sans quoi le dernier disque dessiné imposerait
    // ses réglages à tous les précédents.
    static TPL_PART: RefCell<Option<Material>> = const { RefCell::new(None) };
    static TPL_VOILE: RefCell<Option<Material>> = const { RefCell::new(None) };
    // Instrument de mesure (voir set_voiles) : allumé par défaut.
    static VOILES: Cell<bool> = const { Cell::new(true) };
}

fn mat_particules() -> Material {
    TPL_PART.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(material_particules());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

fn mat_voile() -> Material {
    TPL_VOILE.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(voile::material_voile());
        }
        c.borrow().as_ref().unwrap().clone()
    })
}

/// **Interrupteur de la couche voile** — instrument de mesure, pas un réglage.
///
/// P.2 a chiffré la couche particules : 18 appels de dessin et 25 600 sommets
/// pour les quatre ceintures solaires, du bruit pour un GPU. Le seul coût
/// plausible restant est le **remplissage de pixels du voile** : 960 quads
/// seulement, mais potentiellement plein écran, et chaque pixel paie
/// `disque_voile.frag.glsl` — 155 lignes de bruit procédural.
///
/// Ça ne se compte pas en appels de dessin, ça se mesure au chronomètre. Et
/// comparer « caméra serrée sur Saturne » à « caméra au large » ne l'isole pas :
/// approcher change aussi le coût du corps, de son atmosphère, de ses lunes.
/// Éteindre le voile **à cadrage identique** ne change qu'une chose.
pub fn set_voiles(actifs: bool) {
    VOILES.with(|v| v.set(actifs));
}

/// Vrai si la couche voile est dessinée (défaut).
pub fn voiles_actifs() -> bool {
    VOILES.with(|v| v.get())
}

/// Vide le cache de materials des disques -> la prochaine création recompile
/// depuis les `.glsl` (hot-reload, touche R).
pub fn vider_cache_materials() {
    TPL_PART.with(|c| *c.borrow_mut() = None);
    TPL_VOILE.with(|c| *c.borrow_mut() = None);
}

/// Hauteur du viewport courant (galerie : hauteur de cellule / facteur pixel).
/// La valeur est partagée avec les corps et les étoiles (voir `impostor`).
pub fn set_viewport_h(h: f32) {
    crate::impostor::set_viewport_h(h);
}

/// Demi-taille écran minimale d'une particule, en pixels DE RENDU. 0 = défaut (1.5).
///
/// ⚠️ **C'est une DEMI-taille** : `px_min = 1.5` donne un caillou de **3 px de
/// côté**, et l'ancienne marge de lobes le portait à 3,45. Le commentaire
/// d'origine promettait « 1 pixel de la grille » pour 1.15 — faux d'un facteur
/// ~2,3, et c'est une des raisons pour lesquelles les champs de débris
/// paraissent gros. **Pour viser un débris de N px de côté, passer N/2.**
pub fn set_px_min(p: f32) {
    PX_MIN.with(|v| *v.borrow_mut() = p);
}

fn viewport_h() -> f32 {
    crate::impostor::viewport_h()
}

fn px_min() -> f32 {
    let p = PX_MIN.with(|v| *v.borrow());
    if p > 0.0 {
        p
    } else {
        1.5
    }
}

const QUADS_PAR_LOT: usize = 400; // limite éprouvée du batcher macroquad

const VERT_PART: &str = include_str!("../shaders/disque_particules.vert.glsl");
const FRAG_PART: &str = include_str!("../shaders/disque_particules.frag.glsl");

/// Champ de débris : particules et/ou voile, animés par le GPU.
/// Masse nulle -> n'influence pas la gravité.
pub struct Disque {
    base: CorpsBase,
    cfg: DisqueConfig,
    temps: f32,
    // Rayon du corps central qui projette une ombre cylindrique sur le champ
    // (planète annelée). 0 = pas d'ombre (disques stellaires : la lumière
    // vient du centre).
    ombre_rayon: f32,
    // Base orthonormée du plan du disque (axe_n = normale).
    axe_u: Vec3,
    axe_n: Vec3,
    axe_v: Vec3,
    lots: Vec<Mesh>, // particules : buffers figés (≤ 400 quads chacun)
    mat: Option<Material>,
    voile_lots: Vec<Mesh>, // annulus figé (local, plan Y)
    mat_voile: Option<Material>,
}

impl Disque {
    pub fn new(c: DisqueConfig) -> Self {
        // Base du plan : mêmes conventions que l'ex-anneau (normal, u, v).
        let n = c.normale.normalize_or_zero();
        let n = if n == Vec3::ZERO { Vec3::Y } else { n };
        let tmp = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
        let u = n.cross(tmp).normalize();
        let v = n.cross(u);

        let (lots, mat) = if c.nb > 0 {
            (construire_lots(&c), Some(mat_particules()))
        } else {
            (Vec::new(), None)
        };
        let (voile_lots, mat_voile) = if c.voile_alpha > 0.0 {
            (voile::construire_annulus(&c), Some(mat_voile()))
        } else {
            (Vec::new(), None)
        };

        Self {
            base: CorpsBase::new(Vec3::ZERO, 0.0, 0.0),
            cfg: c,
            temps: 0.0,
            ombre_rayon: 0.0,
            axe_u: u,
            axe_n: n,
            axe_v: v,
            lots,
            mat,
            voile_lots,
            mat_voile,
        }
    }

    /// Rayon du corps central projetant une ombre sur le champ (planète annelée).
    pub fn set_ombre_rayon(&mut self, r: f32) {
        self.ombre_rayon = r;
    }

    /// Phase orbitale (rad) du corps qui creuse la lacune `i` : Kepler au rayon
    /// de la lacune. C'est LE seul couplage CPU par frame (4 sqrt), il anime
    /// l'ondulation des bords ET place le corps embarqué (position_lacune).
    fn phase_lacune(&self, i: usize) -> f32 {
        let l = self.cfg.lacunes[i];
        let r = self.cfg.interne + l.x * (self.cfg.externe - self.cfg.interne);
        let omega = (self.cfg.gm / (r * r * r).max(1e-6)).sqrt();
        omega * self.temps
    }

    /// Position monde du corps embarqué dans la lacune `i` (lune bergère,
    /// proto-planète) : sur le cercle de la lacune, à sa phase orbitale.
    /// `None` si la lacune est inactive ou purement décorative (résonance).
    pub fn position_lacune(&self, i: usize) -> Option<Vec3> {
        let l = *self.cfg.lacunes.get(i)?;
        if l.y <= 0.0 || l.z <= 0.0 {
            return None;
        }
        let r = self.cfg.interne + l.x * (self.cfg.externe - self.cfg.interne);
        let a = self.phase_lacune(i);
        Some(self.base.position + (self.axe_u * a.cos() + self.axe_v * a.sin()) * r)
    }

    /// Dessine une moitié du disque : `moitie` = -1 (arrière, avant le corps),
    /// +1 (avant, après le corps), 0 (tout, en une passe). Les DEUX couches
    /// respectent la moitié (discard GPU) : le rendu peintre
    /// arrière -> corps -> avant reste correct même sans depth buffer
    /// (cible basse résolution du mode pixel).
    pub fn draw_moitie(&mut self, cam: &CameraInfo, moitie: f32) {
        let p = self.base.position;

        {
            if let Some(mat) = &self.mat {
                mat.set_uniform("time", self.temps);
                mat.set_uniform("gm", self.cfg.gm);
                mat.set_uniform("ecc_max", self.cfg.ecc_max);
                mat.set_uniform("centre", (p.x, p.y, p.z));
                mat.set_uniform("axe_u", (self.axe_u.x, self.axe_u.y, self.axe_u.z));
                mat.set_uniform("axe_n", (self.axe_n.x, self.axe_n.y, self.axe_n.z));
                mat.set_uniform("axe_v", (self.axe_v.x, self.axe_v.y, self.axe_v.z));
                mat.set_uniform("cam_right", (cam.right.x, cam.right.y, cam.right.z));
                mat.set_uniform("cam_up", (cam.up.x, cam.up.y, cam.up.z));
                mat.set_uniform("viewport_h", viewport_h());
                mat.set_uniform("px_min", px_min());
                mat.set_uniform("light_pos", (cam.light_pos.x, cam.light_pos.y, cam.light_pos.z));
                mat.set_uniform("cam_pos", (cam.pos.x, cam.pos.y, cam.pos.z));
                mat.set_uniform("ombre_rayon", self.ombre_rayon);
                mat.set_uniform("moitie", moitie);
                gl_use_material(mat);
                for lot in &self.lots {
                    draw_mesh(lot);
                }
                vider_lot();
                gl_use_default_material();
            }
        }

        if let Some(mat) = self.mat_voile.as_ref().filter(|_| voiles_actifs()) {
            let tocam = (cam.pos - p).normalize_or_zero();
            let phases = vec4(
                self.phase_lacune(0),
                self.phase_lacune(1),
                self.phase_lacune(2),
                self.phase_lacune(3),
            );
            // Éclairage : direction de propagation de la lumière au disque, et
            // face vue (éclairée si caméra et lumière sont du même côté du
            // plan ; assombrie sinon — transition douce près de la tranche).
            let dir_lum = (p - cam.light_pos).normalize_or_zero();
            let cote_lum = self.axe_n.dot((cam.light_pos - p).normalize_or_zero());
            let cote_cam = self.axe_n.dot(tocam);
            let f = (cote_lum * cote_cam / 0.05).clamp(-1.0, 1.0); // lissage ±0.05
            let face_lum = 0.775 + 0.225 * f; // 1.0 face éclairée -> 0.55 face nuit
            voile::uniforms_voile(
                mat, &self.cfg, self.temps, p,
                (self.axe_u, self.axe_n, self.axe_v),
                tocam, moitie, phases,
                dir_lum, face_lum, self.ombre_rayon, cam.light_color,
            );
            gl_use_material(mat);
            for lot in &self.voile_lots {
                draw_mesh(lot);
            }
            vider_lot();
            gl_use_default_material();
        }
    }
}

/// **Vide le lot de dessin maintenant.**
///
/// Tous les disques partagent deux materials (voir `TPL_PART`/`TPL_VOILE`), donc
/// deux pipelines — et `set_uniform` écrit dans le pipeline, pas dans le clone.
/// macroquad n'applique ces uniforms qu'au vidage du lot, et changer de material
/// ne fait que poser `break_batching` : sans ce vidage, le **dernier** disque
/// dessiné imposerait ses réglages (centre, axes, `gm`, moitié) à tous ceux
/// d'avant — anneaux plaqués au mauvais endroit, moitiés inversées.
///
/// Même raison et même remède que pour les corps planétaires (`planete/mod.rs`).
fn vider_lot() {
    unsafe {
        get_internal_gl().flush();
    }
}

impl Astre for Disque {
    fn categorie(&self) -> Categorie {
        Categorie::Asteroide
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn update(&mut self, dt: f32) {
        self.temps += dt; // seule mutation par frame : le GPU fait le reste
    }
    fn draw(&mut self, cam: &CameraInfo) {
        self.draw_moitie(cam, 0.0);
    }
}

/// Tire les particules selon la config et fige les buffers, par lots.
fn construire_lots(c: &DisqueConfig) -> Vec<Mesh> {
    // Centres d'amas (clumping) : quelques familles de collision.
    let n_amas = 6;
    let amas: Vec<(f32, f32)> = (0..n_amas)
        .map(|_| {
            (
                gen_range(0.0, std::f32::consts::TAU),           // angle de l'amas
                gen_range(c.interne, c.externe),                 // rayon de l'amas
            )
        })
        .collect();

    // Annelets (sous-anneaux fins) : quelques rayons privilégiés, par graine.
    let n_ringlets = 5;
    let ringlets: Vec<f32> = (0..n_ringlets)
        .map(|_| gen_range(c.interne, c.externe))
        .collect();

    let mut lots: Vec<Mesh> = Vec::new();
    let mut verts: Vec<Vertex> = Vec::with_capacity(QUADS_PAR_LOT * 4);
    let mut inds: Vec<u16> = Vec::with_capacity(QUADS_PAR_LOT * 6);

    for _ in 0..c.nb {
        // ---- Rayon : profil radial (ou annelet) + rejet dans les lacunes ----
        let mut r = 0.0_f32;
        for _ in 0..8 {
            r = if c.ringlets > 0.0 && gen_range(0.0_f32, 1.0) < c.ringlets {
                // Resserré sur un annelet fin (~0.5 % de l'étendue radiale).
                let rj = ringlets[gen_range(0, n_ringlets as i32) as usize % n_ringlets];
                (rj + gen_range(-0.006, 0.006) * (c.externe - c.interne))
                    .clamp(c.interne, c.externe)
            } else {
                let u: f32 = gen_range(0.0_f32, 1.0).powf(c.profil_radial);
                c.interne + u * (c.externe - c.interne)
            };
            let t = (r - c.interne) / (c.externe - c.interne).max(1e-6);
            let dans_lacune = c.lacunes.iter().any(|l| {
                l.y > 0.0 && l.z > 0.0 && (t - l.x).abs() < l.y && gen_range(0.0_f32, 1.0) < l.z
            });
            if !dans_lacune {
                break;
            }
        }

        // ---- Plan orbital : disque (± epaisseur) -> coquille (spherite) ----
        let incl_disque: f32 = gen_range(-c.epaisseur, c.epaisseur);
        let incl_sphere: f32 = gen_range(-1.0_f32, 1.0).asin();
        let mut incl = incl_disque * (1.0 - c.spherite) + incl_sphere * c.spherite;
        let phi: f32 = gen_range(0.0, std::f32::consts::TAU);

        // ---- Clumping : une partie des particules se resserre sur un amas.
        let mut phase = gen_range(0.0_f32, 1.0); // graine -> angle0 dans le shader
        if c.clumping > 0.0 && gen_range(0.0_f32, 1.0) < c.clumping {
            let (ang_a, r_a) = amas[gen_range(0, n_amas as i32) as usize % n_amas];
            let serr = 0.05 + 0.10 * gen_range(0.0_f32, 1.0);
            r = (r_a + gen_range(-serr, serr) * (c.externe - c.interne))
                .clamp(c.interne, c.externe);
            // Angle MONDE d'une particule à t=0 : phi + theta0. Pour grouper
            // l'amas en ang_a, la phase compense donc phi. La dispersion
            // radiale + Kepler étirent ensuite l'amas en arc (réaliste).
            phase = ((ang_a - phi) / std::f32::consts::TAU + gen_range(-0.02, 0.02))
                .rem_euclid(1.0);
            incl *= 0.4; // les amas restent groupés verticalement
        }

        // ---- Taille : essaim de petits (biais u⁴) + GROS blocs (bimodal) ----
        let gros = c.bimodal > 0.0 && gen_range(0.0_f32, 1.0) < c.bimodal;
        let taille = if gros {
            c.taille_max * gen_range(0.8, 1.8)
        } else {
            let u: f32 = gen_range(0.0, 1.0);
            c.taille_min + u * u * u * u * (c.taille_max - c.taille_min)
        };
        // Irrégularité de forme (alpha du sommet, canal libre en pipeline
        // opaque) : les gros fragments sont plus anguleux que les cailloux.
        let irr: f32 = if gros {
            gen_range(0.55, 0.95)
        } else {
            gen_range(0.25, 0.5)
        };

        // ---- Couleur : gradient radial interne->externe + variation ----
        let t = (r - c.interne) / (c.externe - c.interne).max(1e-6);
        let g: f32 = gen_range(0.7, 1.05);
        let teinte = (c.couleur2 * (1.0 - t) + c.couleur * t) * g;
        let col = Color::new(
            teinte.x.clamp(0.0, 1.0),
            teinte.y.clamp(0.0, 1.0),
            teinte.z.clamp(0.0, 1.0),
            irr, // hijack : irrégularité, PAS de l'alpha (particules opaques)
        );

        // ---- 4 sommets : mêmes éléments orbitaux, coin 0..3 dans uv.x ----
        let graine = phase.min(0.9995); // fraction stricte (< 1) pour ne pas mordre sur le coin
        let i0 = verts.len() as u16;
        for coin in 0..4u16 {
            verts.push(Vertex::new2(
                vec3(phi, incl, r),
                vec2(coin as f32 + graine, taille),
                col,
            ));
        }
        inds.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);

        if verts.len() >= QUADS_PAR_LOT * 4 {
            lots.push(Mesh {
                vertices: std::mem::take(&mut verts),
                indices: std::mem::take(&mut inds),
                texture: None,
            });
        }
    }
    if !inds.is_empty() {
        lots.push(Mesh {
            vertices: verts,
            indices: inds,
            texture: None,
        });
    }
    lots
}


/// Material des particules : sommets orbitaux animés par le vertex shader,
/// rendu opaque (depth write, pas de tri nécessaire). Hot-reload via R.
fn material_particules() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &impostor::source("disque_particules.vert.glsl", VERT_PART),
            fragment: &impostor::source("disque_particules.frag.glsl", FRAG_PART),
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("time", UniformType::Float1),
                UniformDesc::new("gm", UniformType::Float1),
                UniformDesc::new("ecc_max", UniformType::Float1),
                UniformDesc::new("centre", UniformType::Float3),
                UniformDesc::new("axe_u", UniformType::Float3),
                UniformDesc::new("axe_n", UniformType::Float3),
                UniformDesc::new("axe_v", UniformType::Float3),
                UniformDesc::new("cam_right", UniformType::Float3),
                UniformDesc::new("cam_up", UniformType::Float3),
                UniformDesc::new("viewport_h", UniformType::Float1),
                UniformDesc::new("px_min", UniformType::Float1),
                UniformDesc::new("light_pos", UniformType::Float3),
                UniformDesc::new("cam_pos", UniformType::Float3),
                UniformDesc::new("ombre_rayon", UniformType::Float1),
                UniformDesc::new("moitie", UniformType::Float1),
            ],
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                // Les cailloux sont OPAQUES : ils écrivent la profondeur, et
                // c'est elle qui les ordonne entre eux et vis-à-vis des corps.
                //
                // (Essayé puis annulé : passer à `false` pour corriger la couture
                // des disques proto*. Sans effet — la couture venait de l'ordre de
                // peinture entre moitiés, pas de la profondeur. Le remède est en
                // amont : plus de découpe en moitiés, voir `galerie_disques.rs`.)
                depth_write: true,
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

    /// Relevé de charge d'un champ : ce que la carte graphique reçoit par frame.
    /// Un lot = un `draw_mesh` = un appel de dessin.
    struct Charge {
        lots: usize,
        quads: usize,
        sommets: usize,
    }

    /// Mesure la couche particules d'une config, **en appelant le constructeur
    /// de maillage réel** (`construire_lots`). Rien n'est recalculé de tête :
    /// si le lotissement change, la mesure suit.
    fn charge_particules(c: &DisqueConfig) -> Charge {
        let lots = construire_lots(c);
        Charge {
            lots: lots.len(),
            quads: lots.iter().map(|m| m.vertices.len() / 4).sum(),
            sommets: lots.iter().map(|m| m.vertices.len()).sum(),
        }
    }

    fn charge_voile(c: &DisqueConfig) -> Charge {
        let lots = voile::construire_annulus(c);
        Charge {
            lots: lots.len(),
            quads: lots.iter().map(|m| m.vertices.len() / 4).sum(),
            sommets: lots.iter().map(|m| m.vertices.len()).sum(),
        }
    }

    /// Le voile est **allumé par défaut**. C'est un instrument de mesure, pas un
    /// réglage : l'éteindre par défaut ferait disparaître les anneaux de Saturne
    /// du jeu entier, et le défaut d'un interrupteur de diagnostic ne se remarque
    /// que le jour où on cherche pourquoi il ne reste plus rien à voir.
    #[test]
    fn les_voiles_sont_allumes_par_defaut() {
        assert!(voiles_actifs(), "les anneaux ne se dessineraient plus");
    }

    /// **Le budget de dessin de la vue système, chiffré.**
    ///
    /// P.2 disait « mesurer avant d'optimiser ». Voici la mesure. Elle porte sur
    /// ce qui est vérifiable sans écran : le nombre d'appels de dessin et de
    /// sommets soumis par frame. Elle ne dit **rien** du temps GPU réel (fill
    /// rate du voile, coût du fragment shader) — ça, seul le compteur de FPS le
    /// dira.
    ///
    /// Le test existe pour qu'un effectif gonflé se voie ici, et non six mois
    /// plus tard quand « ça rame ».
    #[test]
    fn budget_de_dessin_des_ceintures_solaires() {
        let mut total = Charge { lots: 0, quads: 0, sommets: 0 };
        for c in crate::genese::presets::ceintures_solaires() {
            let ch = charge_particules(&c);
            // Toutes les particules demandées sont bien dans un lot : le
            // lotissement ne doit rien perdre en route.
            assert_eq!(ch.quads, c.nb, "particules perdues au lotissement");
            total.lots += ch.lots;
            total.quads += ch.quads;
            total.sommets += ch.sommets;
            // Aucune des quatre familles ne porte de voile : ce sont des nuages
            // de cailloux, pas des anneaux. Si l'une en gagne un, le fill rate
            // change de nature et ce test doit être relu.
            assert_eq!(c.voile_alpha, 0.0, "une ceinture solaire a gagné un voile");
        }
        assert_eq!(total.quads, 6400, "effectif total des petits corps");
        assert_eq!(total.sommets, 25_600);

        // 18 appels de dessin (lots de 400 quads). C'est **peu** : le goulot des
        // ceintures n'est pas là, il est dans le remplissage de pixels.
        assert_eq!(total.lots, 18, "appels de dessin des ceintures");
        assert!(
            total.lots <= 24,
            "budget d'appels de dessin dépassé : {} (limite 24)",
            total.lots
        );
    }

    /// Un anneau planétaire, lui, est un **voile** : peu de sommets, mais il
    /// peut couvrir tout l'écran, et chaque pixel paie un shader de 155 lignes.
    /// C'est là qu'est le vrai coût, et il ne se compte pas en appels de dessin.
    #[test]
    fn un_anneau_est_peu_de_sommets_mais_beaucoup_de_pixels() {
        let cfg = DisqueConfig::anneau_saturne(1.4, 2.2, vec3(0.8, 0.75, 0.6), 0.0);
        let ch = charge_voile(&cfg);
        assert_eq!(ch.quads, 960, "résolution de l'annulus (10 radial × 96 segments)");
        // Dessiné en DEUX moitiés (avant/arrière du corps) -> le double.
        assert!(ch.lots * 2 <= 8, "anneau trop découpé : {} lots", ch.lots);
        // Et il n'a aucune particule : la parité des styles V1 passe par le voile.
        assert_eq!(cfg.nb, 0, "l'anneau de Saturne ne tire pas de particules");
    }
}
