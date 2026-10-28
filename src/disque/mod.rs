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

pub mod accretion;
pub mod bras;
pub mod flot;
mod config;
pub mod habillage;
mod voile;

pub use accretion::Accretion;
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
    static TPL_ACCRETION: RefCell<Option<Material>> = const { RefCell::new(None) };
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

/// Le material du disque d'accrétion, mis en cache comme les autres : le
/// moteur plafonne à 32 pipelines et `load_material` en consomme un à chaque
/// appel (`suivi/technique.md` P.2 ter).
fn mat_accretion() -> Material {
    TPL_ACCRETION.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = Some(accretion::material_accretion());
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
    TPL_ACCRETION.with(|c| *c.borrow_mut() = None);
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
    /// **Les nuages de l'image** (Opti-v2 O1) : position monde, rayon, couleur
    /// prémultipliée, côté caméra du plan. Calculés une fois, dessinés par
    /// moitié — ils l'étaient deux fois par image (`calculer_nuages`).
    nuages: Vec<(Vec3, f32, Color, bool)>,
    /// Ce qui les a fait calculer : caméra, position, temps, viewport. Tant
    /// qu'il ne change pas, `nuages` vaut pour cette image.
    nuages_cle: Option<[f32; 14]>,
    /// Les boules avant l'amorti, et les quads : réutilisés d'une image à
    /// l'autre plutôt que réalloués.
    boules: Vec<(Vec3, f32, Vec3, f32, bool)>,
    quads: (Vec<Vertex>, Vec<u16>),
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
        // Même annulus dans les deux cas — seul le material change. Un
        // disque d'accrétion n'est pas un anneau : il **émet** au lieu
        // d'occulter, et son shader porte un tout autre vocabulaire
        // (`accretion.rs`).
        let (voile_lots, mat_voile) = if c.voile_alpha > 0.0 {
            let mat = if c.accretion.is_some() { mat_accretion() } else { mat_voile() };
            (voile::construire_annulus(&c), Some(mat))
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
            nuages: Vec::new(),
            nuages_cle: None,
            boules: Vec::new(),
            quads: (Vec::new(), Vec::new()),
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
        Some(self.point_du_plan(r, self.phase_lacune(i)))
    }

    /// **L'horloge des effets du lot 12** — nuages 3D, bras de matière,
    /// flots bipolaires — : celle du disque, ralentie (`RALENTI_NUAGES`).
    fn temps_nuages(&self) -> f32 {
        self.temps * RALENTI_NUAGES
    }

    /// **L'axe magnétique** de l'astre qui porte ce disque, en coordonnées du
    /// monde, quand ses entonnoirs suivent un dipôle **incliné** — c'est là
    /// que la matière s'écrase. `None` sinon : les pôles magnétiques sont
    /// alors ceux de rotation.
    pub fn axe_magnetique(&self) -> Option<Vec3> {
        if self.cfg.entonnoirs == 0 || self.cfg.inclinaison_dipole.abs() < 1e-3 {
            return None;
        }
        let m = bras::axe_dipole(self.cfg.inclinaison_dipole, bras::rotation(&self.cfg) * self.temps_nuages());
        Some((self.axe_u * m.x + self.axe_n * m.y + self.axe_v * m.z).normalize_or(self.axe_n))
    }

    /// **Un point du plan du disque**, à `r` du centre et à l'angle `a` —
    /// dans le sens où le disque tourne (angles croissants).
    pub fn point_du_plan(&self, r: f32, a: f32) -> Vec3 {
        self.base.position + (self.axe_u * a.cos() + self.axe_v * a.sin()) * r
    }

    /// Dessine une moitié du disque : `moitie` = -1 (arrière, avant le corps),
    /// +1 (avant, après le corps), 0 (tout, en une passe). Les DEUX couches
    /// respectent la moitié (discard GPU) : le rendu peintre
    /// arrière -> corps -> avant reste correct même sans depth buffer
    /// (cible basse résolution du mode pixel).
    pub fn draw_moitie(&mut self, cam: &CameraInfo, moitie: f32) {
        let p = self.base.position;
        // **Trois points monde pour le GPU.** Le vertex shader des particules
        // reconstruit l'orbite lui-même (`pos = centre + ...`) puis n'utilise
        // plus que des DIFFÉRENCES (`pos - centre`, `cam_pos - pos`,
        // `centre - light_pos`) : les translater tous les trois du même
        // vecteur laisse le calcul inchangé, et retire les grands nombres.
        let (p_rel, cam_rel) = (cam.rel(p), cam.rel(cam.pos));
        let lum_rel = cam.rel(cam.light_pos);

        {
            if let Some(mat) = &self.mat {
                mat.set_uniform("time", self.temps);
                mat.set_uniform("gm", self.cfg.gm);
                mat.set_uniform("ecc_max", self.cfg.ecc_max);
                mat.set_uniform("centre", (p_rel.x, p_rel.y, p_rel.z));
                mat.set_uniform("axe_u", (self.axe_u.x, self.axe_u.y, self.axe_u.z));
                mat.set_uniform("axe_n", (self.axe_n.x, self.axe_n.y, self.axe_n.z));
                mat.set_uniform("axe_v", (self.axe_v.x, self.axe_v.y, self.axe_v.z));
                mat.set_uniform("cam_right", (cam.right.x, cam.right.y, cam.right.z));
                mat.set_uniform("cam_up", (cam.up.x, cam.up.y, cam.up.z));
                mat.set_uniform("viewport_h", viewport_h());
                mat.set_uniform("px_min", px_min());
                mat.set_uniform("light_pos", (lum_rel.x, lum_rel.y, lum_rel.z));
                mat.set_uniform("cam_pos", (cam_rel.x, cam_rel.y, cam_rel.z));
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

        // Les nuages 3D au-delà du plan, vus de la caméra : avant le voile,
        // qui les couvre (voir `dessiner_nuages_3d`).
        if moitie < 0.5 {
            self.dessiner_nuages_3d(cam, false);
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
            // Un disque d'accrétion ne reçoit pas de lumière, il en produit :
            // ni face nuit, ni ombre du corps central, ni teinte d'étoile —
            // ses uniforms n'ont presque rien de commun avec ceux du voile.
            if let Some(a) = self.cfg.accretion {
                // **La moitié arrière se dessine deux fois**, relevée
                // au-dessus puis au-dessous de l'astre : c'est le pli
                // (sous-lot 5.2). La moitié avant reste à plat — elle n'a
                // jamais été cachée, rien ne la dévie.
                for passe in accretion::passes(&a, moitie) {
                    accretion::uniforms_accretion(
                        mat, &self.cfg, &a, self.temps, p_rel,
                        (self.axe_u, self.axe_n, self.axe_v),
                        tocam, moitie, passe,
                    );
                    gl_use_material(mat);
                    for lot in &self.voile_lots {
                        draw_mesh(lot);
                    }
                    // ⚠️ Vidage entre les deux passes : les materials sont
                    // partagés, `set_uniform` écrit dans le pipeline et non
                    // dans le clone. Sans ça, la seconde passe imposerait son
                    // signe de pli à la première (`technique.md` P.2 ter).
                    vider_lot();
                }
                gl_use_default_material();
                return;
            } else {
                voile::uniforms_voile(
                    mat, &self.cfg, self.temps, p_rel,
                    (self.axe_u, self.axe_n, self.axe_v),
                    tocam, moitie, phases,
                    dir_lum, face_lum, self.ombre_rayon, cam.light_color,
                );
            }
            // **Prototype B** (lot 12) : le voile en couches empilées le long
            // de la normale, chacune sa graine — une épaisseur, lisible par la
            // tranche. Une seule couche : le rendu de toujours.
            let n = self.cfg.couches.max(1);
            for k in 0..n {
                if n > 1 {
                    let (alpha, couche, graine) = couche_voile(&self.cfg, k);
                    mat.set_uniform("alpha", alpha * fondu_couche(couche, cote_cam));
                    mat.set_uniform("couche", couche);
                    mat.set_uniform("seed", graine);
                }
                gl_use_material(mat);
                for lot in &self.voile_lots {
                    draw_mesh(lot);
                }
                vider_lot();
            }
            gl_use_default_material();
        }

        // Les nuages 3D du côté de la caméra : après le voile, par-dessus.
        if moitie > -0.5 {
            self.dessiner_nuages_3d(cam, true);
        }
    }

    /// **Prototype C** (lot 12) : des nuages de poussière en volume, au-dessus
    /// et au-dessous du plan, en orbite de Kepler — le pinceau des queues de
    /// comète et des jets (disques flous face caméra, additifs). Ne dessine
    /// que les boules du côté demandé du plan (`cote_camera`).
    ///
    /// ⚠️ **Rangées par côté du PLAN, boule par boule — pas par moitié
    /// avant/arrière.** Elles suivaient d'abord le partage du voile (`moitie`,
    /// selon le CENTRE du nuage) : un amas à cheval sur la frontière, qui à
    /// l'écran est une horizontale passant par l'étoile, passait en entier
    /// sous la moitié avant du voile ou au-dessus — *« une ligne horizontale
    /// […] ça change complètement le rendu entre la partie basse et haute de
    /// la fenêtre »* (2026-10-22, dès le prototype G et ses gros amas). Or le
    /// voile est un plan : ce qui est au-delà, vu de la caméra, est sous lui
    /// partout, et ce qui est en deçà par-dessus. L'étoile, elle, cache ce qui
    /// passe derrière elle par la profondeur.
    fn dessiner_nuages_3d(&mut self, cam: &CameraInfo, cote_camera: bool) {
        let p = self.base.position;
        if !nuages_visibles(self.cfg.externe, (cam.pos - p).length(), viewport_h()) {
            return;
        }
        if self.cfg.nuages_3d > 0 || bras::actifs(&self.cfg) {
            self.calculer_nuages(cam);
            // ⚠️ **Envoyé au fil de l'eau, un lot dès qu'il est plein**
            // (`QUADS_PAR_DESSIN`) : la numérotation `u16` de `push_quad`
            // repart de zéro à chaque lot, donc rien ne déborde, et aucun
            // appel ne passe la limite de macroquad. Le plafond n'est plus
            // qu'une affaire de coût.
            gl_use_material(&crate::soleil::materiau_ruban_lumineux());
            let (v, i) = &mut self.quads;
            for &(q, t, c, cote) in &self.nuages {
                if cote != cote_camera {
                    continue;
                }
                crate::impostor::push_quad(v, i, cam, q, t, c);
                if v.len() >= crate::impostor::QUADS_PAR_DESSIN * 4 {
                    crate::impostor::dessiner_par_lots(v, i, Some(crate::impostor::halo_partage()));
                }
            }
            crate::impostor::dessiner_par_lots(v, i, Some(crate::impostor::halo_partage()));
            vider_lot();
            gl_use_default_material();
        }
    }

    /// **Les nuages et les bras de l'image, calculés une seule fois**
    /// (Opti-v2 O1, dette D1) : ils l'étaient à chaque moitié du disque, soit
    /// deux fois par image, l'empilement compris, pour n'en garder chaque fois
    /// que la moitié — 23 ms par image pour la T Tauri sur la machine de dev.
    /// Rien ne change tant que la caméra, la position, le temps et le viewport
    /// sont les mêmes : la seconde moitié relit le premier calcul.
    fn calculer_nuages(&mut self, cam: &CameraInfo) {
        let p = self.base.position;
        let cle = [
            cam.pos.x, cam.pos.y, cam.pos.z,
            cam.forward.x, cam.forward.y, cam.forward.z,
            cam.right.x, cam.right.y, cam.right.z,
            p.x, p.y, p.z,
            self.temps_nuages(), viewport_h(),
        ];
        if self.nuages_cle == Some(cle) {
            return;
        }
        self.nuages_cle = Some(cle);
        self.nuages.clear();
        let cote_cam = self.axe_n.dot(cam.pos - p);
        {
            let grappe = self.cfg.grappe.max(1);
            let n = self.cfg.nuages_3d.min(40_000 / grappe);
            // Toutes les boules, des deux côtés du plan : l'empilement à
            // l'écran se compte sur l'ensemble (voir `attenuation_globale`).
            // Sortie le temps du calcul (`self` y est lu), rendue à la fin.
            let mut boules = std::mem::take(&mut self.boules);
            boules.clear();
            for k in 0..n {
                let (loc, taille, eclat, t) = nuage_3d(k, &self.cfg, self.temps_nuages());
                let eclat = eclat * hors_lacunes(t, &self.cfg);
                if eclat <= 0.0 {
                    continue;
                }
                let rel = self.axe_u * loc.x + self.axe_n * loc.y + self.axe_v * loc.z;
                // ⚠️ **Pas teinté par la lumière de l'étoile** : elle monte à
                // chaque flare, et le disque entier battait avec — « un effet
                // de battement de cœur » (2026-10-22).
                let c = self.cfg.voile_couleur2.lerp(self.cfg.voile_couleur, t) * teinte_nuage(k, &self.cfg);
                // Vers l'étoile, depuis le nuage : c'est d'où vient la lumière.
                let vers_etoile = (-rel).normalize_or_zero();
                for j in 0..grappe {
                    let (d, t_j, poids) = boule_de_grappe(k, j, grappe, taille);
                    let dw = self.axe_u * d.x + self.axe_n * d.y + self.axe_v * d.z;
                    let cote = du_cote_camera(rel + dw, self.axe_n, cote_cam);
                    let lum = eclairement_boule(dw, vers_etoile);
                    boules.push((p + rel + dw, t_j, c * lum, eclat * 0.35 * poids, cote));
                }
            }
            // **L'empilement, vu de la caméra** : en tangentes d'angle (le
            // plan de l'écran à distance 1), cases de la taille moyenne d'une
            // boule (voir `Empilement`).
            let ecran = |q: Vec3| {
                let d = q - cam.pos;
                let z = d.dot(cam.forward).max(1e-3);
                (vec2(d.dot(cam.right), d.dot(cam.up)) / z, z)
            };
            let somme: f32 = boules.iter().map(|b| b.1 / ecran(b.0).1).sum();
            let mut pile = Empilement::new(2.0 * somme / boules.len().max(1) as f32);
            for b in boules.iter() {
                let (e, z) = ecran(b.0);
                pile.verser(e, b.1 / z, b.3);
            }
            // Un seul facteur pour tout le disque (voir `attenuation_globale`).
            let f = attenuation_globale(pile.pic());
            for &(q, t_j, c, a, cote) in boules.iter() {
                let a = a * f;
                self.nuages.push((q, t_j, Color::new(c.x * a, c.y * a, c.z * a, a), cote));
            }
            self.boules = boules;
            // **Les bras de matière** (`bras`) : de la lumière propre, hors
            // de l'amorti — c'est le gaz qui chauffe en tombant.
            let t = self.temps_nuages();
            let flots = bras::entonnoirs(&self.cfg, t)
                .into_iter()
                .chain(bras::spirales(&self.cfg, t))
                .chain(bras::bord_brulant(&self.cfg, t))
                .chain(bras::pluie(&self.cfg, t))
                .chain(bras::fragments(&self.cfg, t))
                .chain(flot::boules(&self.cfg.flot, t));
            for (loc, t_b, c, a) in flots {
                let rel = self.axe_u * loc.x + self.axe_n * loc.y + self.axe_v * loc.z;
                let cote = du_cote_camera(rel, self.axe_n, cote_cam);
                self.nuages.push((p + rel, t_b, Color::new(c.x * a, c.y * a, c.z * a, a), cote));
            }
        }
    }
}

/// **Le pas des effets du lot 12** — nuages 3D, bras de matière, flots
/// bipolaires —, rapporté à l'horloge de spectacle : *« il va falloir diviser
/// la vitesse du temps par 8 au moins, je suis obligé de la mettre en 0,125
/// pour avoir un rendu correct »* (2026-10-24). Eux seuls : les particules et
/// le voile du disque, comme les animations des étoiles, gardent leur pas,
/// validé aux lots précédents. L'axe magnétique suit la même horloge que ses
/// entonnoirs, sans quoi les taches ne tomberaient plus sous les flots.
pub(crate) const RALENTI_NUAGES: f32 = 1.0 / 8.0;

/// **Rayon apparent d'un disque, en pixels, sous lequel ses nuages et ses
/// bras ne se dessinent plus** : le voile seul le porte.
const DETAIL_NUAGES_PX: f32 = 40.0;

/// **Niveau de détail des nuages 3D** (lot 12, adoption de la T Tauri) :
/// le disque d'une jeune étoile porte ~30 000 boules, recalculées à chaque
/// image. En vue système, il ne fait que quelques pixels — les calculer
/// serait payer pour rien, et plusieurs étoiles peuvent en porter. On les
/// dessine dès qu'il dépasse `DETAIL_NUAGES_PX` de rayon apparent (champ
/// vertical de 45°, la caméra de macroquad).
pub(crate) fn nuages_visibles(externe: f32, dist: f32, hauteur_px: f32) -> bool {
    const TAN_DEMI_CHAMP: f32 = 0.414; // tan(22,5°)
    externe / dist.max(1e-3) / TAN_DEMI_CHAMP * hauteur_px * 0.5 >= DETAIL_NUAGES_PX
}

/// **Ce que les sillons laissent d'un nuage** posé en `t` : 1 hors sillon,
/// `1 − profondeur` en son cœur, un fondu sur le dernier tiers du bord.
///
/// Sans ça, seul le voile se creusait : les nuages 3D comblaient le sillon
/// qu'une planète vient d'ouvrir (prototypes -F, 2026-10-22).
pub(crate) fn hors_lacunes(t: f32, c: &DisqueConfig) -> f32 {
    c.lacunes
        .iter()
        .filter(|l| l.y > 0.0 && l.z > 0.0 && (t - l.x).abs() < l.y)
        .map(|l| {
            let bord = 1.0 - ((t - l.x).abs() / l.y - 0.66).clamp(0.0, 0.34) / 0.34;
            1.0 - l.z.min(1.0) * bord
        })
        .product()
}

/// **L'empilement des nuages à l'écran** : une grille de cases, chacune
/// cumulant l'éclat des boules qui la **couvrent**, en tangentes d'angle vues
/// de la caméra (±1 : au-delà, hors champ).
///
/// ⚠️ **Chaque boule verse dans toutes les cases qu'elle couvre**, dégressif
/// vers son bord. Elle ne versait d'abord que dans la case de son centre : au
/// cœur d'un disque vu par la tranche s'y tassaient des centaines de petites
/// boules, et les grosses dont le centre tombait là s'éteignaient avec elles,
/// alors qu'elles couvraient dix fois plus large — le centre, le plus
/// chargé, ressortait **noir** (T Tauri par la tranche, 2026-10-23). Une
/// boule plus petite qu'une case n'y verse que la part qu'elle en couvre, et
/// le dégressif est celui du halo dessiné, `(1 − d)²` : la pile mesure ce
/// qu'on voit.
struct Empilement {
    cote: usize,
    case: f32,
    pile: Vec<f32>,
}

impl Empilement {
    /// Demi-champ couvert par la grille, en tangente d'angle.
    const BORD: f32 = 1.0;

    /// Une grille de cases d'environ `case` (tangente d'angle), 512 de côté
    /// au plus.
    fn new(case: f32) -> Self {
        let cote = ((2.0 * Self::BORD / case.max(1e-6)).ceil() as usize).clamp(1, 512);
        Self { cote, case: 2.0 * Self::BORD / cote as f32, pile: vec![0.0; cote * cote] }
    }

    fn indice(&self, x: i32, y: i32) -> Option<usize> {
        let n = self.cote as i32;
        (x >= 0 && y >= 0 && x < n && y < n).then(|| y as usize * self.cote + x as usize)
    }

    /// Le centre d'une boule, en cases (réelles).
    fn en_cases(&self, e: Vec2) -> Vec2 {
        (e + Vec2::splat(Self::BORD)) / self.case
    }

    /// Verse l'éclat `a` d'une boule de centre `e` et de rayon `r`.
    fn verser(&mut self, e: Vec2, r: f32, a: f32) {
        let c = self.en_cases(e);
        let rc = r / self.case;
        let couverture = (2.0 * rc).min(1.0).powi(2);
        let portee = rc + 0.5;
        let n = portee.ceil().min(12.0) as i32;
        let (cx, cy) = (c.x.floor() as i32, c.y.floor() as i32);
        // **Les boucles bornées à la grille** (Opti-v2 O2) : caméra dans le
        // disque, la grille dégénère en une case (`new`) et chaque boule
        // parcourait jusqu'à 625 cases hors champ pour rien — 7,2 ms par
        // calcul pour la T Tauri. Mêmes cases visitées, même ordre : même
        // résultat.
        let m = self.cote as i32 - 1;
        for dy in (-n).max(cy.saturating_neg())..=n.min(m.saturating_sub(cy)) {
            for dx in (-n).max(cx.saturating_neg())..=n.min(m.saturating_sub(cx)) {
                let d2 = (dx * dx + dy * dy) as f32;
                if d2 >= portee * portee {
                    continue;
                }
                if let Some(i) = self.indice(cx + dx, cy + dy) {
                    self.pile[i] += a * couverture * (1.0 - d2.sqrt() / portee).powi(2);
                }
            }
        }
    }

    /// L'éclat cumulé là où tombe le centre `e` ; 0 hors champ.
    #[cfg(test)]
    fn lire(&self, e: Vec2) -> f32 {
        let c = self.en_cases(e);
        self.indice(c.x.floor() as i32, c.y.floor() as i32).map_or(0.0, |i| self.pile[i])
    }

    /// **Le pic de l'empilement** : le 98ᵉ centile des cases éclairées —
    /// le plus chargé, sans qu'une case isolée ne décide pour tout le disque.
    fn pic(&self) -> f32 {
        let mut p: Vec<f32> = self.pile.iter().copied().filter(|x| *x > 0.0).collect();
        if p.is_empty() {
            return 0.0;
        }
        let k = (p.len() * 98 / 100).min(p.len() - 1);
        *p.select_nth_unstable_by(k, f32::total_cmp).1
    }
}

/// **Pic d'empilement sous lequel rien ne bouge**, et **marge** au-delà :
/// le pic affiché ne dépasse jamais `SEUIL_PIC + MARGE_PIC`.
///
/// Mesuré le 2026-10-23 (98ᵉ centile, en éclat de halo) : de face, T Tauri
/// T0 ~11, T1 ~14 — sans brûler ; par la tranche, T0 ~24, T1 ~141 — brûlé.
/// À une marge de 10, T1 par la tranche tombait à ×0,18 et s'éteignait ; à
/// 40, il garde ×0,39 : une bande chaude, sans brûler. En mode Palette, les
/// halos faibles du bord y deviennent une couronne noire et un liseré gris
/// — la quantification de la palette, absente en rendu net.
const SEUIL_PIC: f32 = 15.0;
const MARGE_PIC: f32 = 40.0;

/// **Un seul facteur pour tous les nuages d'un disque**, d'après le pic de
/// leur empilement à l'écran : 1 sous le seuil, puis juste ce qu'il faut pour
/// que le pic ne dépasse pas `SEUIL_PIC + MARGE_PIC` (coude doux, `tanh`).
///
/// Les nuages sont additifs (le pipeline plasma, partagé : pas de matériau
/// de plus sous le plafond) : vus par la tranche, des dizaines s'empilaient
/// et brûlaient l'image — *« le cumul de nuages finit par donner des zones
/// très brillantes quand on regarde à travers G3 sur le plan horizontal du
/// disque »* (2026-10-22).
///
/// ⚠️ **Global, pas boule par boule.** L'amorti était d'abord local — chaque
/// boule selon l'empilement de sa case. Le plus chargé y perdait plus que
/// ses voisins n'en gardaient : deux fois de suite, le cœur d'un disque vu
/// par la tranche est ressorti **noir** — *« il a le même problème de disque
/// qui noircit au milieu quand on regarde l'horizontale »* (2026-10-23). Un
/// facteur commun garde les contrastes tels quels : le plus chargé reste le
/// plus clair.
pub(crate) fn attenuation_globale(pic: f32) -> f32 {
    if pic <= SEUIL_PIC {
        return 1.0;
    }
    (SEUIL_PIC + MARGE_PIC * ((pic - SEUIL_PIC) / MARGE_PIC).tanh()) / pic
}

/// **Une boule est-elle du côté de la caméra par rapport au plan du disque ?**
/// `rel` : depuis le centre ; `cote_cam` : hauteur de la caméra le long de la
/// normale. Seul compte le côté du plan — pas l'avant ou l'arrière du disque.
pub(crate) fn du_cote_camera(rel: Vec3, axe_n: Vec3, cote_cam: f32) -> bool {
    rel.dot(axe_n) * cote_cam >= 0.0
}

/// **Une couche du voile** (prototype B) : `(alpha, décalage le long de la
/// normale en part du rayon, graine)`. Les couches vont de −h/r à +h/r ; les
/// plus proches du plan pèsent le plus (profil gaussien vertical d'un vrai
/// disque), et leurs alphas sont répartis pour que la somme vue de face
/// reste celle du voile à plat.
fn couche_voile(c: &DisqueConfig, k: u32) -> (f32, f32, f32) {
    let n = c.couches.max(2);
    let x = k as f32 / (n - 1) as f32 * 2.0 - 1.0;
    let poids = (-2.0 * x * x).exp();
    let total: f32 = (0..n).map(|j| (-2.0 * (j as f32 / (n - 1) as f32 * 2.0 - 1.0).powi(2)).exp()).sum();
    (c.voile_alpha * poids / total * 1.6, c.h_r * x, c.graine + k as f32 * 7.3)
}

/// **Ce qui reste d'une couche du voile selon l'angle de vue** : 1 pour la
/// couche du plan, toujours ; pour les autres, 1 dès qu'on voit le disque à
/// plus de ~20° de sa tranche, 0 en dessous de ~3°.
///
/// ⚠️ Chaque couche hors du plan est un **cône** (son décalage croît avec le
/// rayon). Vus par la tranche, les cônes se réduisent à des droites issues de
/// l'étoile, granulation étirée dessus : *« quand tu regardes avec le disque
/// à l'horizontale ça fait des rayons bizarres »* (2026-10-23, T Tauri T5).
/// Par la tranche, l'épaisseur est portée par les nuages 3D.
pub(crate) fn fondu_couche(couche: f32, cote_cam: f32) -> f32 {
    if couche == 0.0 {
        return 1.0;
    }
    let s = ((cote_cam.abs() - 0.05) / 0.3).clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

/// **Une boule d'une grappe** (prototype F) : `(décalage LOCAL depuis le
/// centre du nuage, rayon de la boule, poids d'éclat)`.
///
/// Un nuage cotonneux est fait de **boules qui se chevauchent**, de tailles
/// inégales, groupées autour du centre et décalées dans les trois
/// directions : c'est le contour bosselé d'un cumulus, là où un disque flou
/// seul se lit comme une tache lisse. La première boule est au centre et la
/// plus grosse. Une seule boule : le nuage d'origine, inchangé.
///
/// ⚠️ **Le poids, mesuré à l'écran.** Réparti d'abord pour que la grappe ne
/// brille pas plus qu'un nuage seul (1,8 / boules), il la rendait invisible :
/// chaque boule ne pesait plus que 0,3 sur un halo déjà concentré au centre.
/// Les « taches » qu'on prenait pour les nuages étaient la granulation du
/// voile. Sondé à ×11 (trop), réglé à ~×4 (2026-10-22).
pub(crate) fn boule_de_grappe(k: u32, j: u32, grappe: u32, taille: f32) -> (Vec3, f32, f32) {
    if grappe <= 1 {
        return (Vec3::ZERO, taille, 1.0);
    }
    let h = |s: f32| ((k as f32 * 4.1414 + j as f32 * 31.337 + s * 17.17).sin() * 43_758.547).fract().abs();
    let poids = 8.0 / grappe as f32;
    if j == 0 {
        return (Vec3::ZERO, taille * 0.85, poids);
    }
    let d = vec3(h(1.0) * 2.0 - 1.0, (h(2.0) * 2.0 - 1.0) * 0.6, h(3.0) * 2.0 - 1.0);
    let d = d.normalize_or(Vec3::X) * taille * (0.45 + 0.4 * h(4.0));
    (d, taille * (0.45 + 0.35 * h(5.0)), poids)
}

/// **Éclairement d'une boule de grappe** selon sa place dans le nuage :
/// claire du côté de l'étoile, sombre du côté opposé, mi-teinte au cœur.
///
/// C'est ce qui donne du **volume** à un nuage additif, qui ne peut ni
/// masquer ni assombrir ce qu'il y a derrière lui : sans ce dégradé, la
/// grappe se lisait comme un autocollant posé sur le disque (2026-10-22).
pub(crate) fn eclairement_boule(decalage: Vec3, vers_etoile: Vec3) -> f32 {
    let cote = decalage.normalize_or_zero().dot(vers_etoile); // -1 ombre … +1 éclairé
    0.35 + 0.65 * (0.5 + 0.5 * cote)
}

/// **Un nuage de poussière en volume** (prototype C), à l'instant `temps` :
/// `(position LOCALE (u, n, v), rayon du nuage, éclat 0..1, t radial 0..1)`.
///
/// Pur et sans état : tout se dérive de son rang et de la graine du disque —
/// le même nuage revient au même endroit à graine égale. Orbite de **Kepler**
/// (`ω = √(gm/r³)`, la vitesse des particules) : l'intérieur file, le large
/// traîne. Hauteur dans ±1,5 h/r du plan.
///
/// ⚠️ **Sa taille et son éclat sont fixes.** La taille suivait d'abord un
/// cycle de vie — *« les nuages de fumée gonflent et dégonflent »* —, puis
/// l'éclat, que la grappe du prototype F faisait s'allumer et s'éteindre d'un
/// bloc — *« un effet de battement de cœur »* (2026-10-22). Un nuage ne
/// respire plus : il tourne, c'est tout.
pub(crate) fn nuage_3d(k: u32, c: &DisqueConfig, temps: f32) -> (Vec3, f32, f32, f32) {
    use std::f32::consts::TAU;
    let h = |s: f32| hache(k as f32, c.graine, s);
    let rayon = |t: f32| c.interne + t * (c.externe - c.interne);
    let kepler = |r: f32| (c.gm / (r * r * r).max(1e-6)).sqrt();
    let (t, ang, z_rel, gros) = if c.nuages_amas > 0 {
        // **G — des amas massés** : chaque nuage appartient à un amas, posé
        // près du plan ; l'amas tourne d'un bloc à la vitesse de son centre,
        // sans se disperser le long de l'orbite — la gravité les rassemble.
        let a = (k % c.nuages_amas) as f32;
        let ha = |s: f32| hache(a + 9_000.0, c.graine, s);
        let tc = 0.08 + 0.84 * ha(1.0);
        let t = (tc + (h(1.0) - 0.5) * 0.09).clamp(0.0, 1.0);
        let ang = ha(2.0) * TAU + (h(2.0) - 0.5) * 0.45 + kepler(rayon(tc)) * temps;
        (t, ang, (h(3.0) * 2.0 - 1.0) * 0.5, 1.0)
    } else if c.nuages_bras > 0 {
        // **I — un ouragan dont l'œil est l'étoile** : un quart des nuages
        // fait le **mur de l'œil**, un anneau serré tout autour d'elle ; les
        // autres suivent des bras en spirale logarithmique qui en partent, et
        // le motif tourne d'un bloc — à la vitesse du milieu de l'anneau — au
        // lieu d'être cisaillé par Kepler.
        // ⚠️ Les bras partaient d'abord de t = 0,1, tirés vers l'extérieur
        // (√) : le centre restait vide, et l'on voyait *« le truc à trois
        // bras »*, pas un cyclone autour du soleil (2026-10-22).
        // Second retour : *« la zone où la majorité des débris est cumulée
        // doit être plus au centre […] plus de bras, randomiser leur taille /
        // aspect »* — d'où des nuages tassés vers l'œil (h^1,8), et des bras
        // inégaux : longueur, largeur, enroulement, taille de leurs nuages et
        // départ propres à chacun.
        let tourne = kepler(rayon(0.5)) * temps;
        if h(6.0) < 0.2 {
            (0.04 + 0.08 * h(1.0), h(2.0) * TAU + tourne, (h(3.0) * 2.0 - 1.0) * 0.6, 1.0)
        } else {
            let bras = k % c.nuages_bras;
            let hb = |s: f32| hache(bras as f32 + 7_000.0, c.graine, s);
            let longueur = 0.35 + 0.65 * hb(1.0);
            let largeur = 0.25 + 0.6 * hb(2.0);
            let enroulement = 1.9 + 0.6 * hb(3.0);
            let depart = (bras as f32 + (hb(4.0) - 0.5) * 0.6) * TAU / c.nuages_bras as f32;
            let t = 0.04 + (longueur - 0.04) * h(1.0).powf(1.8);
            let spirale = depart + enroulement * (rayon(t) / c.interne.max(1e-3)).ln();
            (t, spirale + (h(2.0) - 0.5) * largeur + tourne, (h(3.0) * 2.0 - 1.0) * 0.8, 0.7 + 0.7 * hb(5.0))
        }
    } else {
        // **C — des nuages épars** sur l'anneau, chacun à sa vitesse de Kepler.
        let t = h(1.0);
        let ang = h(2.0) * TAU + kepler(rayon(t)) * temps;
        (t, ang, (h(3.0) * 2.0 - 1.0) * 1.5, 1.0)
    };
    let r = rayon(t);
    let z = z_rel * c.h_r * r * (r / c.externe.max(1e-6)).powf(c.evasement);
    // Constant, mais inégal d'un nuage à l'autre.
    let eclat = 0.6 + 0.4 * h(4.0);
    // L'ouragan garde des nuages visibles jusqu'au mur de l'œil : leur
    // taille ne fond pas avec le rayon sous le quart de l'anneau.
    let r_taille = if c.nuages_bras > 0 { r.max(rayon(0.25)) } else { r };
    let taille = r_taille * (0.03 + 0.05 * h(5.0)) * c.taille_nuages * gros;
    (vec3(ang.cos() * r, z, ang.sin() * r), taille, eclat, t)
}

/// Pseudo-hasard déterministe dans `[0 ; 1[`, propre à un rang et une graine.
fn hache(i: f32, graine: f32, s: f32) -> f32 {
    ((i * 12.9898 + graine * 78.233 + s * 37.719).sin() * 43_758.547).fract().abs()
}

/// **Teinte propre à un nuage** (prototype H) : un multiplicateur autour de 1,
/// qui pousse chaque nuage vers le chaud ou le froid. Neutre si
/// `teintes_nuages` est nul.
fn teinte_nuage(k: u32, c: &DisqueConfig) -> Vec3 {
    let e = c.teintes_nuages;
    if e <= 0.0 {
        return Vec3::ONE;
    }
    let chaud = hache(k as f32, c.graine, 6.0) * 2.0 - 1.0; // −1 froid … +1 chaud
    let eclat = hache(k as f32, c.graine, 7.0) * 2.0 - 1.0;
    vec3(1.0 + 0.45 * e * chaud, 1.0 + 0.15 * e * eclat, 1.0 - 0.45 * e * chaud).max(Vec3::splat(0.2))
}

#[cfg(test)]
mod tests_prototypes {
    use super::*;

    fn proto() -> DisqueConfig {
        DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0).avec_nuages_3d(300, 0.08)
    }

    /// **Les nuages 3D tournent comme un disque** : chacun garde son rayon
    /// dans l'anneau, reste dans l'épaisseur annoncée, et l'intérieur file
    /// plus vite que le large (Kepler). Leur éclat reste dans `[0 ; 1]`.
    #[test]
    fn les_nuages_tournent_comme_un_disque() {
        let c = proto();
        for k in 0..300 {
            let (p0, taille, eclat, _) = nuage_3d(k, &c, 0.0);
            let (p1, _, _, _) = nuage_3d(k, &c, 50.0);
            let r0 = vec2(p0.x, p0.z).length();
            assert!((r0 - vec2(p1.x, p1.z).length()).abs() < 1e-2, "nuage {k} : change de rayon");
            assert!(r0 >= c.interne - 1e-3 && r0 <= c.externe + 1e-3, "nuage {k} hors de l'anneau ({r0})");
            assert!(p0.y.abs() <= c.h_r * r0 * 1.5 + 1e-3, "nuage {k} hors de l'épaisseur");
            assert!((0.0..=1.0).contains(&eclat) && taille > 0.0);
            // Ni taille ni éclat ne respirent (gonflement, puis battement).
            assert_eq!(nuage_3d(k, &c, 23.0).2, eclat, "nuage {k} : son éclat bat");
            // Taille fixe : un nuage qui enfle et dégonfle se lit comme une
            // respiration, refusé à l'écran.
            for instant in [3.0_f32, 11.0, 40.0] {
                assert_eq!(nuage_3d(k, &c, instant).1, taille, "nuage {k} : sa taille respire");
            }
        }
        // Kepler : l'angle parcouru décroît avec le rayon.
        let tour = |k: u32| {
            let (a, _, _, _) = nuage_3d(k, &c, 0.0);
            let (b, _, _, _) = nuage_3d(k, &c, 1.0);
            let r = vec2(a.x, a.z).length();
            (r, vec2(a.x, a.z).angle_between(vec2(b.x, b.z)).abs())
        };
        let mut v: Vec<(f32, f32)> = (0..300).map(tour).collect();
        v.sort_by(|x, y| x.0.total_cmp(&y.0));
        assert!(v[0].1 > v[v.len() - 1].1 * 2.0, "l'intérieur ne file pas plus vite que le large");
    }

    /// **G : un amas tourne d'un bloc.** Deux nuages du même amas gardent leur
    /// écart au fil du temps — c'est ce qui les distingue des nuages épars,
    /// que Kepler disperse le long de l'orbite.
    #[test]
    fn un_amas_tourne_d_un_bloc() {
        let c = DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0).avec_nuages_3d(300, 0.05).avec_amas(6);
        // Les nuages 0 et 6 sont dans le même amas (rang modulo 6).
        let ecart = |t: f32| (nuage_3d(0, &c, t).0 - nuage_3d(6, &c, t).0).length();
        assert!((ecart(0.0) - ecart(80.0)).abs() < ecart(0.0) * 0.05 + 1e-3, "l'amas se disperse");
        // Et l'amas reste un amas : ses nuages sont proches les uns des autres.
        assert!(ecart(0.0) < 0.3 * (c.externe - c.interne), "amas trop étalé : {}", ecart(0.0));
    }

    /// **I : l'étoile est l'œil, et le motif tourne d'un bloc.** Rien au
    /// ras de l'étoile, mais un mur de nuages tout autour d'elle ; et le
    /// motif ne se cisaille pas : deux instants donnent la même spirale,
    /// tournée.
    #[test]
    fn l_ouragan_garde_sa_spirale() {
        let c = DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0).avec_nuages_3d(300, 0.05).avec_bras(3);
        let mut quadrants = [0; 4];
        for k in 0..300 {
            let (p, _, _, t) = nuage_3d(k, &c, 0.0);
            assert!(t >= 0.01, "nuage {k} dans l'œil");
            if t < 0.1 {
                let q = (p.z.atan2(p.x).rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_2) as usize;
                quadrants[q.min(3)] += 1;
            }
            let (q, _, _, _) = nuage_3d(k, &c, 40.0);
            let rot = |p: Vec3| p.z.atan2(p.x);
            let tour = (rot(q) - rot(p)).rem_euclid(std::f32::consts::TAU);
            let (r0, _, _, _) = nuage_3d(0, &c, 0.0);
            let (r1, _, _, _) = nuage_3d(0, &c, 40.0);
            let tour0 = (rot(r1) - rot(r0)).rem_euclid(std::f32::consts::TAU);
            assert!((tour - tour0).abs() < 1e-3, "nuage {k} : le motif se cisaille");
        }
        assert!(quadrants.iter().all(|&n| n >= 10), "pas de mur autour de l'œil : {quadrants:?}");
        // Les débris se tassent vers l'œil : plus de la moitié dans le tiers
        // intérieur de l'anneau.
        let dedans = (0..300).filter(|&k| nuage_3d(k, &c, 0.0).3 < 0.33).count();
        assert!(dedans > 150, "débris pas assez au centre : {dedans} / 300");
    }

    /// **H : la teinte varie d'un nuage à l'autre, sans jamais s'éteindre** ;
    /// neutre quand l'écart est nul.
    #[test]
    fn les_teintes_varient_sans_s_eteindre() {
        let neutre = DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0);
        assert_eq!(teinte_nuage(5, &neutre), Vec3::ONE);
        let c = neutre.avec_aspect_nuages(1.0, 1.0);
        let teintes: Vec<Vec3> = (0..50).map(|k| teinte_nuage(k, &c)).collect();
        assert!(teintes.iter().all(|t| t.min_element() >= 0.2), "un nuage s'éteint");
        assert!(teintes.iter().any(|t| t.x > t.z * 1.3) && teintes.iter().any(|t| t.z > t.x * 1.3), "pas de variété");
    }

    /// **Une boule se range selon le côté du plan, jamais selon l'avant ou
    /// l'arrière du disque** : sinon la frontière avant/arrière, horizontale à
    /// l'écran, coupait les amas qui la chevauchent.
    #[test]
    fn une_boule_se_range_par_cote_du_plan() {
        let (n, cam) = (Vec3::Y, 5.0); // caméra au-dessus du plan
        for avant in [-3.0, 0.0, 3.0] {
            assert!(du_cote_camera(vec3(1.0, 0.2, avant), n, cam), "au-dessus, avant = {avant}");
            assert!(!du_cote_camera(vec3(1.0, -0.2, avant), n, cam), "au-dessous, avant = {avant}");
        }
        // Caméra au-dessous : tout s'inverse.
        assert!(du_cote_camera(vec3(0.0, -0.2, 2.0), n, -cam));
    }

    /// **De loin, pas de nuages ; de près, si** — et la limite suit la taille
    /// du disque et celle de l'écran.
    #[test]
    fn les_nuages_n_apparaissent_que_de_pres() {
        assert!(nuages_visibles(43.0, 95.0, 1440.0), "T Tauri en vue isolée");
        assert!(!nuages_visibles(43.0, 20_000.0, 1440.0), "T Tauri en vue système");
        assert!(nuages_visibles(43.0, 300.0, 1440.0) && !nuages_visibles(43.0, 300.0, 100.0), "l'écran ne compte pas");
        assert!(nuages_visibles(400.0, 3000.0, 1440.0) && !nuages_visibles(40.0, 3000.0, 1440.0), "la taille ne compte pas");
    }

    /// **Une étoile verte teinte de vert ce qu'elle éclaire**, légèrement ;
    /// une blanche, rien.
    #[test]
    fn une_etoile_verte_teinte_ses_debris() {
        let c = DisqueConfig::debris_recents(0, 4.0, 10.0, 1000.0);
        assert_eq!(c.teinte_eclairage, Vec3::ONE);
        let t = c.avec_teinte_eclairage(vec3(0.78, 1.0, 0.42), 0.35).teinte_eclairage;
        assert!(t.y > t.x && t.x > t.z, "pas vert : {t}");
        assert!(t.y > 0.99 && t.z > 0.75, "trop fort : {t}");
        assert_eq!(c.avec_teinte_eclairage(Vec3::ONE, 0.35).teinte_eclairage, Vec3::ONE);
    }

    /// **Un sillon se vide aussi de ses nuages**, et seulement lui : intact
    /// au loin, vidé selon sa profondeur en son cœur ; une bande brillante
    /// (profondeur négative) n'y touche pas.
    #[test]
    fn un_sillon_se_vide_de_ses_nuages() {
        let c = DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0).avec_lacune(0.5, 0.05, 0.9).avec_bande(0.2, 0.05, 0.5);
        assert!((hors_lacunes(0.5, &c) - 0.1).abs() < 1e-4, "cœur du sillon : {}", hors_lacunes(0.5, &c));
        assert_eq!(hors_lacunes(0.8, &c), 1.0);
        assert_eq!(hors_lacunes(0.2, &c), 1.0, "la bande brillante vide les nuages");
        let bord = hors_lacunes(0.547, &c);
        assert!(bord > 0.1 && bord < 1.0, "pas de fondu au bord : {bord}");
    }

    /// **Une grosse boule n'est pas éteinte par les petites qui tombent
    /// près de son centre** : seule, elle ne s'amortit pas ; cent petites
    /// boules au même endroit pèsent ce qu'elles couvrent, pas leur nombre.
    #[test]
    fn une_grosse_boule_ne_s_eteint_pas_sous_les_petites() {
        let mut seule = Empilement::new(0.05);
        seule.verser(Vec2::ZERO, 0.2, 0.3);
        assert!(seule.lire(Vec2::ZERO) <= 0.3 + 1e-4, "une boule seule s'amortit elle-même");

        let mut tas = Empilement::new(0.05);
        tas.verser(Vec2::ZERO, 0.2, 0.3);
        for k in 0..100 {
            tas.verser(vec2(k as f32 * 1e-4, 0.0), 0.002, 0.3);
        }
        let p = tas.lire(Vec2::ZERO);
        assert!(p < 1.0, "cent poussières pèsent comme {p}");
        // Et ce qui s'empile vraiment pèse davantage.
        let mut dense = Empilement::new(0.05);
        for _ in 0..20 {
            dense.verser(Vec2::ZERO, 0.2, 0.3);
        }
        assert!(dense.lire(Vec2::ZERO) > 5.0);
        assert_eq!(dense.lire(vec2(5.0, 0.0)), 0.0, "hors champ");
    }

    /// **Borner les boucles à la grille ne change rien** (Opti-v2 O2) :
    /// l'ancien parcours, gardé ici en référence, et le nouveau remplissent
    /// les mêmes cases des mêmes valeurs — grille dégénérée (une case, la
    /// caméra dans le disque) comme grille ordinaire, boules hors champ
    /// comprises.
    #[test]
    fn borner_l_empilement_ne_change_rien() {
        fn verser_sans_borne(e: &mut Empilement, centre: Vec2, r: f32, a: f32) {
            let c = e.en_cases(centre);
            let rc = r / e.case;
            let couverture = (2.0 * rc).min(1.0).powi(2);
            let portee = rc + 0.5;
            let n = portee.ceil().min(12.0) as i32;
            let (cx, cy) = (c.x.floor() as i32, c.y.floor() as i32);
            for dy in -n..=n {
                for dx in -n..=n {
                    let d2 = (dx * dx + dy * dy) as f32;
                    if d2 >= portee * portee {
                        continue;
                    }
                    if let Some(i) = e.indice(cx + dx, cy + dy) {
                        e.pile[i] += a * couverture * (1.0 - d2.sqrt() / portee).powi(2);
                    }
                }
            }
        }
        for case in [5.0, 0.3, 0.05, 0.004] {
            let (mut borne, mut libre) = (Empilement::new(case), Empilement::new(case));
            for k in 0..3000u32 {
                let h = |m: f32| ((k as f32 * m).sin() * 43758.547).fract();
                let centre = vec2(h(12.9) * 6.0 - 3.0, h(78.2) * 6.0 - 3.0);
                let r = 0.001 + h(37.7).abs() * 0.4;
                borne.verser(centre, r, 0.3);
                verser_sans_borne(&mut libre, centre, r, 0.3);
            }
            assert_eq!(borne.cote, libre.cote);
            assert!(borne.pile == libre.pile, "case {case} ({} de côté) : les piles diffèrent", borne.cote);
        }
    }

    /// **Les nuages superposés saturent au lieu de s'additionner** : sous le
    /// seuil rien ne bouge, au-delà le pic ne dépasse jamais seuil + marge,
    /// et en ajouter n'assombrit pas.
    #[test]
    fn les_nuages_superposes_saturent() {
        assert_eq!(attenuation_globale(0.0), 1.0);
        assert_eq!(attenuation_globale(SEUIL_PIC), 1.0, "une vue de face est amortie");
        let mut avant = 0.0;
        for k in 1..400 {
            let pic = k as f32;
            let affiche = pic * attenuation_globale(pic);
            assert!(affiche <= SEUIL_PIC + MARGE_PIC + 1e-3, "pic {pic} : {affiche} dépasse le plafond");
            assert!(affiche >= avant - 1e-4, "pic {pic} : plus de nuages, moins d'éclat");
            avant = affiche;
        }
        let mut e = Empilement::new(0.05);
        assert_eq!(e.pic(), 0.0);
        for _ in 0..10 {
            e.verser(Vec2::ZERO, 0.2, 0.3);
        }
        assert!(e.pic() > 1.0 && e.pic() <= 3.0 + 1e-4, "pic de dix boules : {}", e.pic());
    }

    /// **Une grappe a un côté éclairé et un côté dans l'ombre** : c'est ce qui
    /// lui donne du volume. Le cœur est entre les deux.
    #[test]
    fn une_grappe_s_eclaire_du_cote_de_l_etoile() {
        let etoile = Vec3::X;
        let face = eclairement_boule(Vec3::X, etoile);
        let dos = eclairement_boule(-Vec3::X, etoile);
        let coeur = eclairement_boule(Vec3::ZERO, etoile);
        assert!(face > coeur && coeur > dos, "{face} / {coeur} / {dos}");
        assert!(dos > 0.0, "le côté ombre disparaît");
        assert!(face - dos > 0.5, "contraste trop faible pour lire un volume : {face} / {dos}");
    }

    /// **Une grappe reste un nuage** : ses boules restent près du centre, la
    /// première au centre et la plus grosse, et une seule boule redonne le
    /// nuage d'origine.
    #[test]
    fn une_grappe_reste_un_nuage() {
        assert_eq!(boule_de_grappe(3, 0, 1, 2.0), (Vec3::ZERO, 2.0, 1.0));
        for k in 0..50 {
            let (d0, t0, _) = boule_de_grappe(k, 0, 6, 2.0);
            assert_eq!(d0, Vec3::ZERO, "la première boule n'est pas au centre");
            for j in 1..6 {
                let (d, t, poids) = boule_de_grappe(k, j, 6, 2.0);
                assert!(d.length() <= 2.0 * 0.85 + 1e-4, "boule {j} du nuage {k} trop loin : {}", d.length());
                assert!(t < t0 && t > 0.0, "boule {j} plus grosse que le cœur");
                assert!(poids > 0.0 && poids < 2.0);
            }
        }
    }

    /// **Un disque évasé est mince près de l'étoile, et garde son épaisseur
    /// au bord** ; sans évasement, rien ne change.
    #[test]
    fn un_disque_evase_s_amincit_vers_l_etoile() {
        let c = DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0).avec_nuages_3d(400, 0.3);
        let e = DisqueConfig { evasement: 0.8, ..c };
        let (mut dedans, mut bord) = (0, 0);
        for k in 0..400 {
            let (p, _, _, t) = nuage_3d(k, &c, 0.0);
            let (q, _, _, _) = nuage_3d(k, &e, 0.0);
            let (hp, hq) = (p.y.abs(), q.y.abs());
            assert!(hq <= hp + 1e-4, "nuage {k} : l'évasement l'épaissit");
            if t < 0.2 && hp > 1e-3 && hq < hp * 0.5 {
                dedans += 1;
            }
            if t > 0.95 && (hq - hp).abs() <= hp * 0.1 + 1e-4 {
                bord += 1;
            }
        }
        assert!(dedans > 20, "pas plus mince près de l'étoile : {dedans}");
        assert!(bord > 5, "le bord a perdu son épaisseur : {bord}");
    }

    /// **Par la tranche, seule la couche du plan reste** : les autres, des
    /// cônes, y feraient des rayons ; de face, toutes sont là.
    #[test]
    fn par_la_tranche_seule_la_couche_du_plan_reste() {
        let c = DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0).avec_couches(7, 0.08);
        for k in 0..7 {
            let (_, couche, _) = couche_voile(&c, k);
            assert_eq!(fondu_couche(couche, 0.8), 1.0, "couche {k} effacée de face");
            assert_eq!(fondu_couche(couche, -0.8), 1.0, "couche {k} effacée de face, par-dessous");
            let tranche = fondu_couche(couche, 0.02);
            if k == 3 {
                assert_eq!(tranche, 1.0, "la couche du plan s'efface");
            } else {
                assert_eq!(tranche, 0.0, "couche {k} encore là par la tranche");
            }
        }
        let mut avant = 0.0;
        for i in 0..=20 {
            let f = fondu_couche(0.08, i as f32 * 0.05);
            assert!(f >= avant, "le fondu n'est pas monotone");
            avant = f;
        }
    }

    /// **Les couches donnent une épaisseur sans assombrir ni éclaircir** :
    /// symétriques autour du plan, la plus forte au milieu, et la somme de
    /// leurs alphas proche du voile à plat.
    #[test]
    fn les_couches_s_empilent_autour_du_plan() {
        let c = DisqueConfig::protoplanetaire(0, 10.0, 100.0, 1000.0).avec_couches(7, 0.08);
        let couches: Vec<_> = (0..7).map(|k| couche_voile(&c, k)).collect();
        assert!((couches[0].1 + couches[6].1).abs() < 1e-6, "couches non symétriques");
        assert!(couches[3].1.abs() < 1e-6 && couches[3].0 > couches[0].0, "le plan ne pèse pas le plus");
        let somme: f32 = couches.iter().map(|x| x.0).sum();
        assert!((somme / c.voile_alpha - 1.6).abs() < 1e-3, "somme des alphas : {somme}");
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
        let teinte = (c.couleur2 * (1.0 - t) + c.couleur * t) * g * c.teinte_eclairage;
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

    /// Les types qui **portent** un `Disque` et doivent donc faire avancer
    /// son horloge.
    const PORTEURS: [(&str, &str); 2] = [
        ("planete/mod.rs", include_str!("../planete/mod.rs")),
        ("soleil/mod.rs", include_str!("../soleil/mod.rs")),
    ];

    /// **Tout porteur de disque fait avancer son horloge.**
    ///
    /// ⚠️ `Disque::update` est la **seule mutation par frame** : tout le
    /// reste est calculé sur le GPU à partir de `temps`. Un porteur qui
    /// oublie de l'appeler obtient un disque qui se dessine parfaitement et
    /// **ne tourne jamais** — rien ne plante, rien ne manque à l'écran, il
    /// est juste figé.
    ///
    /// C'est arrivé au disque d'accrétion : `Planete::update` faisait
    /// avancer son anneau, `Soleil::update` non. Le défaut était invisible
    /// en galerie des disques, qui appelle `update` sur ses disques
    /// elle-même, et ne se voyait qu'en skymap et en vue isolée.
    #[test]
    fn tout_porteur_de_disque_avance_son_horloge() {
        for (nom, src) in PORTEURS {
            // `Option<Disque>` ou `Vec<Disque>` — le soleil en porte
            // plusieurs depuis que le trou noir vorace a une ceinture en
            // plus de son disque.
            assert!(
                src.contains("Disque>"),
                "{nom} ne porte plus de disque : retirer cette garde ou la corriger"
            );
            let avance = src
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .any(|l| l.contains(".update(dt)"));
            assert!(avance, "{nom} : son disque ne recevra jamais `dt`, donc il restera figé");
        }
    }

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
