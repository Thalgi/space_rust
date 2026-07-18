mod apparence;
mod materiau;
mod palette;
pub mod terrain;
mod vortex;
mod zonal;

pub use apparence::{Apparence, TypePlanete};

/// Vide le cache de materials des planètes (hot-reload des shaders).
pub fn vider_cache_materials() {
    materiau::vider_cache();
}

/// Hauteur (px) du viewport de rendu courant, pour le LOD des gazeuses.
/// À appeler avant de dessiner dans un viewport partiel (galerie) ; remettre
/// à 0 (= plein écran) ensuite.
pub fn set_viewport_h(h: f32) {
    materiau::set_viewport_h(h);
}

use crate::astre::{Astre, Categorie, CameraInfo, CorpsBase, Foyer};
use crate::disque::{Disque, DisqueConfig};
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use materiau::{appliquer_uniforms, mat_corps};

pub struct Planete {
    pub base: CorpsBase,
    app: Apparence,
    orbite: Vec<Vec3>,
    mat: Material,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
    // Anneau : champ de débris (couche voile) qui suit la planète. Rendu en
    // deux moitiés autour du corps (voir draw). Voir CONCEPTION_CEINTURES.md.
    anneau: Option<Disque>,
    // Terrain précalculé (telluriques) : atlas cube-sphere + niveau de mer.
    // Généré en ASYNCHRONE au premier draw (budget global de jobs -> pas de gel,
    // la galerie affiche un placeholder le temps que le terrain arrive).
    terrain_tex: Option<(Texture2D, f32)>,
    terrain_job: Option<std::thread::JoinHandle<terrain::DonneesTerrain>>,
    // Profil zonal 1D (gazeuses) : texture jets/bandes/cisaillement + borne
    // polaire pole_lat (zonal.rs).
    zonal_tex: Option<(Texture2D, f32)>,
    // Slots de vortex (gazeuses) : tache/ovales/barges/chapelets (vortex.rs).
    vortex_unis: Option<([Vec4; vortex::N_VORTEX], [Vec4; vortex::N_VORTEX])>,
    // Lune : si parent défini, orbite analytique autour de l'astre `parent`.
    parent: Option<usize>,
    l_angle: f32,
    l_omega: f32,
    l_r: f32,
    l_a1: Vec3,
    l_q: Vec3,
    // Planète « sur rails » : orbite de Kepler analytique autour de son foyer.
    orbite_kep: Option<crate::orbite::Orbite>,
    foyer: Foyer,
}

impl Planete {
    pub fn new(
        position: Vec3,
        vitesse: Vec3,
        rayon: f32,
        masse: f32,
        app: Apparence,
        orbite: Vec<Vec3>,
    ) -> Self {
        let mut base = CorpsBase::new(position, masse, rayon);
        base.vitesse = vitesse;

        // Material partagé (cloné) -> un seul pipeline GPU pour toutes les planètes/lunes.
        let mat = mat_corps();

        // Construire l'anneau si nécessaire : traduction des champs anneau_*
        // d'Apparence vers le système unifié (style -> preset DisqueConfig).
        let anneau = if app.anneau {
            let mut d = Disque::new(config_anneau(rayon, &app));
            d.set_ombre_rayon(rayon); // la planète projette son ombre sur l'anneau
            Some(d)
        } else {
            None
        };

        // Profil zonal + slots de vortex (gazeuses) : synchrone, < 1 ms.
        let (zonal_tex, vortex_unis) = if app.type_p == TypePlanete::Gazeuse {
            (Some(zonal::generer_zonal(&app)), Some(vortex::generer_vortex(&app)))
        } else {
            (None, None)
        };

        Self {
            base,
            app,
            orbite,
            mat,
            verts: Vec::new(),
            inds: Vec::new(),
            anneau,
            terrain_tex: None,
            terrain_job: None,
            zonal_tex,
            vortex_unis,
            parent: None,
            l_angle: 0.0,
            l_omega: 0.0,
            l_r: 0.0,
            l_a1: Vec3::X,
            l_q: Vec3::Z,
            orbite_kep: None,
            foyer: Foyer::Barycentre,
        }
    }

    /// Attache une orbite de Kepler (planète « sur rails »).
    pub fn avec_orbite(mut self, o: crate::orbite::Orbite) -> Self {
        self.orbite_kep = Some(o);
        self
    }

    /// Définit le foyer d'orbite (étoile hôte S-type, ou barycentre P-type).
    pub fn avec_foyer(mut self, f: Foyer) -> Self {
        self.foyer = f;
        self
    }

    /// Niveau d'extension des lumières de villes (0 = aucune … 4 = très étendu).
    pub fn set_villes(&mut self, niveau: f32) {
        self.app.villes = niveau;
    }

    /// Vrai si la planète possède un anneau (la galerie recule la caméra pour le cadrer).
    pub fn a_un_anneau(&self) -> bool {
        self.app.anneau
    }

    /// Vrai si le rendu est définitif (terrain généré, ou corps sans terrain).
    /// Sert aux captures de non-régression : on ne fige pas un placeholder.
    pub fn terrain_pret(&self) -> bool {
        self.app.type_p != TypePlanete::Tellurique || self.terrain_tex.is_some()
    }

    /// Copie de l'apparence (pour le bench de génération).
    pub fn apparence(&self) -> Apparence {
        self.app
    }

    /// Rayon (visuel, unités monde) du corps. Sert au cadrage caméra de la galerie.
    pub fn rayon(&self) -> f32 {
        self.base.rayon
    }

    /// Rayon externe de l'anneau (× rayon planète) ; 0 si aucun anneau.
    pub fn rayon_anneau(&self) -> f32 {
        if self.app.anneau {
            self.app.anneau_out
        } else {
            0.0
        }
    }

    /// Transforme cette planète en lune orbitant l'astre d'index `parent`.
    /// `r_orbite` en unités monde, `omega` vitesse angulaire, `incl` inclinaison, `phase` départ.
    pub fn en_lune(mut self, parent: usize, r_orbite: f32, omega: f32, incl: f32, phase: f32) -> Self {
        let phi = phase;
        let a1 = vec3(phi.cos(), 0.0, phi.sin());
        let a2 = vec3(-phi.sin(), 0.0, phi.cos());
        self.parent = Some(parent);
        self.l_r = r_orbite;
        self.l_omega = omega;
        self.l_angle = phase;
        self.l_a1 = a1;
        self.l_q = (a2 * incl.cos() + Vec3::Y * incl.sin()).normalize();
        self
    }

}

/// Traduit les champs `anneau_*` d'`Apparence` (API conservée : les presets et
/// la persistance ne changent pas) en config du système unifié. Le style V1
/// (0 Saturne, 1 granuleux, 2 arcs, 3 débris, 4 Uranus) choisit le preset.
fn config_anneau(rayon: f32, app: &Apparence) -> DisqueConfig {
    let r_in = rayon * app.anneau_in;
    let r_out = rayon * app.anneau_out;
    let c = app.anneau_couleur;
    let g = app.seed;
    let cfg = match app.anneau_style as i32 {
        1 => DisqueConfig::anneau_granuleux(r_in, r_out, c, g),
        2 => DisqueConfig::anneau_arcs(r_in, r_out, c, g),
        3 => DisqueConfig::anneau_debris(r_in, r_out, c, g),
        4 => DisqueConfig::anneau_uranus(r_in, r_out, c, g),
        _ => DisqueConfig::anneau_saturne(r_in, r_out, c, g),
    };
    cfg.avec_normale(app.anneau_normal)
}

impl Astre for Planete {
    fn categorie(&self) -> Categorie {
        if self.parent.is_some() {
            Categorie::Lune
        } else {
            Categorie::Planete
        }
    }
    fn parent(&self) -> Option<usize> {
        self.parent
    }
    fn orbiter_autour(&mut self, centre: Vec3, dt: f32) {
        self.l_angle += self.l_omega * dt;
        self.base.position =
            centre + self.l_a1 * (self.l_r * self.l_angle.cos()) + self.l_q * (self.l_r * self.l_angle.sin());
    }
    fn maj_rail(&mut self, foyer: Vec3, t: f64) {
        if let Some(o) = &self.orbite_kep {
            self.base.position = foyer + o.position(t);
        }
    }
    fn amorcer_ncorps(&mut self, foyer_pos: Vec3, foyer_vel: Vec3, t: f64) {
        if let Some(o) = &self.orbite_kep {
            let (p, v) = o.etat(t);
            self.base.position = foyer_pos + p;
            self.base.vitesse = foyer_vel + v;
        }
    }
    fn foyer(&self) -> Option<Foyer> {
        Some(self.foyer)
    }
    fn corps(&self) -> &CorpsBase {
        &self.base
    }
    fn corps_mut(&mut self) -> &mut CorpsBase {
        &mut self.base
    }
    fn update(&mut self, dt: f32) {
        // L'anneau s'anime (rotation différentielle du voile) au rythme du jeu.
        if let Some(a) = &mut self.anneau {
            a.update(dt);
        }
    }
    fn orbite(&self) -> &[Vec3] {
        &self.orbite
    }

    fn draw(&mut self, cam: &CameraInfo) {
        let c = self.base.position;
        let r = self.base.rayon;

        // Terrain précalculé (telluriques) : génération en thread de fond,
        // upload GPU quand le job aboutit. En attendant, placeholder (tex 1×1).
        if self.terrain_tex.is_none() && self.app.type_p == TypePlanete::Tellurique {
            if self.terrain_job.as_ref().is_some_and(|j| j.is_finished()) {
                if let Ok(d) = self.terrain_job.take().unwrap().join() {
                    let tex = Texture2D::from_rgba8(d.largeur, d.hauteur, &d.atlas);
                    tex.set_filter(FilterMode::Linear);
                    self.terrain_tex = Some((tex, d.niveau_mer));
                }
            } else if self.terrain_job.is_none() && terrain::reserver_job() {
                let app = self.app;
                self.terrain_job = Some(std::thread::spawn(move || terrain::generer_job(&app)));
            }
        }

        // --- Corps (impostor) ---
        self.verts.clear();
        self.inds.clear();
        crate::impostor::push_quad(&mut self.verts, &mut self.inds, c, cam.right, cam.up, r * 1.05, WHITE);

        let terr = self.terrain_tex.as_ref().map(|(t, nm)| (t, *nm));
        appliquer_uniforms(
            &self.mat,
            &self.app,
            cam,
            c,
            r,
            terr,
            self.zonal_tex.as_ref().map(|(tex, pl)| (tex, *pl)),
            self.vortex_unis.as_ref(),
        );

        // Anneau : moitié arrière AVANT le corps (la planète la masquera).
        if let Some(a) = &mut self.anneau {
            a.corps_mut().position = c; // l'anneau suit la planète
            a.draw_moitie(cam, -1.0);
        }

        gl_use_material(&self.mat);
        let quad = Mesh {
            vertices: std::mem::take(&mut self.verts),
            indices: std::mem::take(&mut self.inds),
            texture: None,
        };
        draw_mesh(&quad);
        self.verts = quad.vertices;
        self.inds = quad.indices;
        gl_use_default_material();

        // Anneau : moitié avant APRÈS le corps (passe devant la planète).
        if let Some(a) = &mut self.anneau {
            a.draw_moitie(cam, 1.0);
        }
    }
}
