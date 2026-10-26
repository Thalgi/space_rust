mod apparence;
pub mod classe;
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
    // Coquille atmosphérique : passe additive séparée, dessinée par-dessus le
    // corps (voir `draw` et shaders/atmosphere.frag.glsl).
    mat_atmo: Material,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
    // Anneau : champ de débris (couche voile) qui suit la planète. Rendu en
    // deux moitiés autour du corps (voir draw). Voir docs/conception/ceintures.md.
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
    // Trace de l'orbite lunaire, en coordonnées RELATIVES au parent.
    //
    // Rangée à part de `orbite` (qui est celle des planètes, relative au foyer
    // stellaire) : la boucle des trajectoires planétaires centre sur `foyer()`,
    // ce qui poserait l'orbite d'une lune autour du barycentre du système au
    // lieu de sa planète. Deux traces, deux repères, deux boucles.
    //
    // Calculée UNE fois : elle ne dépend que des éléments orbitaux, qui ne
    // bougent pas. La régénérer par frame allouerait pour rien.
    orbite_lune: Vec<Vec3>,
    l_angle: f32,
    l_omega: f32,
    l_r: f32,
    l_a1: Vec3,
    l_q: Vec3,
    // Planète « sur rails » : orbite de Kepler analytique autour de son foyer.
    orbite_kep: Option<crate::orbite::Orbite>,
    foyer: Foyer,
    // Ce corps tient la place d'une étoile, et rayonne cette luminosité.
    // Voir [`Planete::au_centre_du_systeme`].
    role_central: Option<f32>,
    // Ce corps est une comète. Voir [`Planete::en_comete`].
    comete: bool,
    // Maillage du noyau cométaire, en rayon unité. Vide pour tout le reste.
    noyau: (Vec<crate::comete::SommetNoyau>, Vec<u16>),
    // Index dans `comete::CATALOGUE` : la matière du noyau et les queues qu'elle
    // porte. Ignoré hors des comètes.
    preset_comete: usize,
    // Horloge et graine de la fluctuation des queues (`comete::fluctuer`) :
    // l'horloge avance avec le jeu (pause comprise), la graine décale les
    // phases pour que deux comètes ne palpitent pas ensemble.
    temps_comete: f32,
    graine_comete: f32,
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

        // Materials partagés (clonés) -> deux pipelines GPU pour toutes les planètes/lunes.
        let mat = mat_corps();
        let mat_atmo = materiau::mat_atmo();

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
            mat_atmo,
            verts: Vec::new(),
            inds: Vec::new(),
            anneau,
            terrain_tex: None,
            terrain_job: None,
            zonal_tex,
            vortex_unis,
            parent: None,
            orbite_lune: Vec::new(),
            l_angle: 0.0,
            l_omega: 0.0,
            l_r: 0.0,
            l_a1: Vec3::X,
            l_q: Vec3::Z,
            orbite_kep: None,
            foyer: Foyer::Barycentre,
            role_central: None,
            comete: false,
            noyau: (Vec::new(), Vec::new()),
            preset_comete: 0,
            temps_comete: 0.0,
            graine_comete: 0.0,
        }
    }

    /// **Ce corps est une comète.**
    ///
    /// ⚠️ Ne change que la **catégorie**, et c'est délibéré. Une comète est un
    /// noyau sombre sur une orbite très excentrique : le corps est celui d'un
    /// petit caillou, et tout ce qui la distingue au lot 9.1 tient dans son
    /// orbite. La **queue** est du rendu neuf, et viendra au 9.2 — la poser ici
    /// derrière ce drapeau mélangerait deux objets dans un seul type.
    ///
    /// La catégorie, elle, doit être juste **tout de suite** : c'est elle que
    /// lisent le sélecteur, la fiche et la désignation, et
    /// `Categorie::Comete` existait depuis le début **sans que rien ne la
    /// produise** — une catégorie morte, du même genre que le trou noir avant
    /// le lot 4.
    ///
    /// ⚠️ Le **maillage du noyau** est construit ici, une fois : un patatoïde,
    /// pas la sphère de l'impostor. Un noyau de comète est trop petit pour que
    /// sa gravité l'arrondisse — signalé à l'écran, « pour le moment c'est une
    /// bille ».
    pub fn en_comete(self, graine: f32) -> Self {
        self.en_comete_preset(graine, crate::comete::preset_de_graine(graine))
    }

    /// La même, avec une **famille** choisie plutôt que tirée de la graine :
    /// Halley est glacée parce qu'elle l'est, pas parce qu'un reste de division
    /// l'a décidé.
    pub fn en_comete_preset(mut self, graine: f32, preset: usize) -> Self {
        self.comete = true;
        self.preset_comete = preset;
        self.graine_comete = graine;
        self.noyau = crate::comete::noyau_de(graine, crate::comete::preset(preset));
        self
    }

    /// **Ce corps tient la place d'une étoile** : il reçoit donc le plancher en
    /// pixels, comme elle.
    ///
    /// ⚠️ Le plancher est une **exception**, et son exposé de motifs dit
    /// pourquoi : seule l'étoile tombe sous le pixel au cadrage par défaut, et
    /// relever une Terre de la même façon la rendrait plus grosse que le Soleil
    /// tout en donnant à chaque caillou une cible de clic de huit pixels
    /// (`soleil/mod.rs`, `le_defaut_du_trait_rend_le_rayon_physique`).
    ///
    /// Ce raisonnement porte sur les planètes **parmi** les planètes. Un corps
    /// au **centre**, lui, joue le rôle de l'étoile : une naine brune de deux
    /// unités y ferait quelques pixels, et on arriverait dans un système dont
    /// le cœur est invisible. C'est le même fait qui justifie le plancher, pas
    /// une entorse.
    ///
    /// ⚠️ **Un seul champ pour un seul fait.** Le rôle entraîne trois
    /// conséquences — le plancher, la lumière qu'il jette sur ses mondes, et la
    /// luminosité que lisent les fiches et la zone habitable. Portées par un
    /// booléen plus une luminosité rangée ailleurs, elles auraient pu diverger ;
    /// portées par `Option<f32>`, un corps qui éclaire est un corps qui a le
    /// plancher, par construction.
    ///
    /// Sans elle, `luminosite_totale` valait zéro pour un tel système et chaque
    /// fiche de planète y annonçait **0 K**.
    pub fn au_centre_du_systeme(mut self, luminosite: f32) -> Self {
        self.role_central = Some(luminosite);
        self
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
        // Trace de l'orbite, dans le plan (a1, q) : le MÊME couple de vecteurs
        // que `orbiter_autour`, pour que le trait passe exactement là où la lune
        // passera. Une seconde formule dériverait sans que rien ne le dise.
        // Même raison qu'en `genese` : la trace est dessinée en points, et le
        // nombre d'échantillons plafonne leur densité de près.
        const N: usize = 256;
        self.orbite_lune = (0..N)
            .map(|i| {
                let t = i as f32 / N as f32 * std::f32::consts::TAU;
                self.l_a1 * (self.l_r * t.cos()) + self.l_q * (self.l_r * t.sin())
            })
            .collect();
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
        if self.comete {
            Categorie::Comete
        } else if self.parent.is_some() {
            Categorie::Lune
        } else {
            Categorie::Planete
        }
    }
    fn parent(&self) -> Option<usize> {
        self.parent
    }
    fn orbite_lune(&self) -> &[Vec3] {
        &self.orbite_lune
    }
    fn orbiter_autour(&mut self, centre: Vec3, dt: f32) {
        self.l_angle += self.l_omega * dt;
        self.base.position =
            centre + self.l_a1 * (self.l_r * self.l_angle.cos()) + self.l_q * (self.l_r * self.l_angle.sin());
    }
    fn maj_rail(&mut self, foyer: Vec3, t: f64) {
        if let Some(o) = &self.orbite_kep {
            // ⚠️ **La vitesse aussi.** Elle restait figée à sa valeur de `t = 0`
            // depuis la construction : un corps sur rails n'en avait pas besoin,
            // puisque sa position vient de l'orbite. La queue d'une comète, elle,
            // s'incurve vers sa **traînée** — avec une vitesse périmée, elle se
            // serait incurvée du même côté sur toute l'orbite. `position` passe
            // déjà par `etat`, donc ça ne coûte rien.
            let (p, v) = o.etat(t);
            self.base.position = foyer + p;
            self.base.vitesse = v;
        }
    }
    fn foyer(&self) -> Option<Foyer> {
        Some(self.foyer)
    }
    /// Même formule que `maj_rail`, en lecture seule : `None` pour une lune
    /// (`orbite_kep` n'est posé que pour une planète « sur rails » — une
    /// lune se déplace par `orbiter_autour`, pas encore couvert ici, voir
    /// `docs/conception/trajectoires.md` §7).
    fn position_future(&self, foyer_predit: Vec3, t: f64) -> Option<Vec3> {
        self.orbite_kep.map(|o| foyer_predit + o.position(t))
    }
    /// Teinte d'ensemble, tirée de l'**apparence réelle** de la planète.
    ///
    /// Pour une tellurique, la couleur vue de loin est un mélange de ses terres
    /// et de son océan, pondéré par la part d'eau : une planète-océan lit bleu,
    /// un désert lit ocre. Pour une gazeuse, ses deux bandes principales se
    /// moyennent — c'est ce que l'œil retient d'un disque rayé.
    fn etendue_visuelle(&self) -> f32 {
        if self.a_un_anneau() {
            self.rayon_anneau().max(1.0)
        } else {
            1.0
        }
    }
    fn normale_anneau(&self) -> Option<Vec3> {
        self.a_un_anneau().then_some(self.app.anneau_normal)
    }
    fn rendu_pret(&self) -> bool {
        self.terrain_pret()
    }
    /// Rayon dessiné : physique par défaut, **relevé au plancher** pour un corps
    /// qui tient la place d'une étoile ([`Planete::au_centre_du_systeme`]).
    ///
    /// ⚠️ La même source que le dessin et que `Systeme::pick`, comme chez
    /// l'étoile : une cible de clic qui ne suivrait pas la taille peinte se
    /// décalerait justement sur le corps que le plancher agrandit.
    fn rayon_dessine(&self, cam_pos: Vec3) -> f32 {
        if self.role_central.is_none() {
            return self.base.rayon;
        }
        crate::impostor::rayon_plancher(
            self.base.rayon,
            (cam_pos - self.base.position).length(),
            crate::impostor::viewport_h(),
            crate::soleil::PX_MIN,
        )
    }
    /// Luminosité rayonnée : **rien**, sauf pour un corps qui tient la place
    /// d'une étoile ([`Planete::au_centre_du_systeme`]). C'est elle que somment
    /// `luminosite_totale`, la zone habitable et la fiche d'astre.
    fn luminosite(&self) -> Option<f32> {
        self.role_central
    }
    /// Lumière jetée sur le système, dérivée de la **teinte réelle** du corps —
    /// jamais d'une couleur posée à côté, sinon retoucher son apparence
    /// n'éclairerait plus ses mondes de la même façon.
    fn lumiere(&self) -> Option<Vec3> {
        let l = self.role_central?;
        Some(self.teinte().unwrap_or(Vec3::ONE) * l)
    }
    fn gazeuse(&self) -> Option<bool> {
        Some(self.app.type_p == TypePlanete::Gazeuse)
    }
    /// Le type de la planète (`classe::classer`) ; une comète n'en a pas —
    /// elle aura son logo à elle.
    fn classe_planete(&self) -> Option<classe::ClassePlanete> {
        (!self.comete).then(|| classe::classer(&self.app))
    }
    fn teinte(&self) -> Option<Vec3> {
        let a = &self.app;
        Some(match a.type_p {
            TypePlanete::Tellurique => {
                let terre = (a.couleur + a.couleur2) * 0.5;
                terre.lerp(a.couleur3, a.eau.clamp(0.0, 1.0))
            }
            TypePlanete::Gazeuse => (a.couleur + a.couleur2) * 0.5,
            // Glacée : ses deux tons sont déjà clairs, la moyenne suffit.
            TypePlanete::Glacee => (a.couleur + a.couleur2) * 0.5,
        })
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
        if self.comete {
            self.temps_comete += dt;
        }
    }
    fn orbite(&self) -> &[Vec3] {
        &self.orbite
    }

    fn draw(&mut self, cam: &CameraInfo) {
        let c = self.base.position;
        let r = self.base.rayon;

        // **La queue d'une comète** (lot 9.2), avant le noyau : elle en part, et
        // le noyau doit se poser par-dessus.
        //
        // ⚠️ Tout ce qui décide vit dans `crate::comete`, en fonctions **pures**
        // et testables ; il ne reste ici que le ruban et le tirage.
        //
        // ⚠️ **L'étoile, c'est `cam.light_pos`**, et non l'origine. Les deux se
        // confondent dans le préset solaire et dans la galerie, ce qui rendait le
        // raccourci invisible — mais une **binaire** met son barycentre à
        // l'origine et ses étoiles ailleurs : la queue fuyait alors un point où
        // il n'y a rien, et le noyau était éclairé du même mauvais côté.
        // `systeme::rendu::eclairage` porte déjà la position de l'étoile
        // primaire : la reprendre fait une source unique au lieu de deux
        // vérités.
        if self.comete {
            // Le seuil vient de **son** orbite : les systemes sont mis a
            // l'echelle de leur etoile, donc un seuil en UA allumerait toutes
            // les cometes d'une naine en permanence.
            let seuil = self
                .orbite_kep
                .map(|o| crate::comete::seuil_activite(o.a, o.e))
                .unwrap_or(0.0);
            let p = crate::comete::preset(self.preset_comete);
            // ⚠️ **Le material du plasma solaire, pas celui par défaut.** Celui
            // de macroquad, en caméra 3D, **écrit la profondeur** — sur tout le
            // ruban, parties transparentes comprises. La queue de poussière,
            // tracée la première, posait donc un écran invisible : la queue
            // d'ions était rejetée partout où elle passait derrière, c'est-à-dire
            // près du noyau, et n'apparaissait qu'au-delà de la poussière.
            // Le plasma teste la profondeur sans l'écrire, et il est additif :
            // sur le noir de l'espace c'est le même rendu, et là où les deux
            // queues se croisent elles s'additionnent au lieu de se masquer.
            // Zéro pipeline de plus : c'est celui des arches du soleil.
            let mat = crate::soleil::materiau_ruban_lumineux();
            gl_use_material(&mat);
            // Animées : voir `comete::fluctuer`, sans quoi les queues se lisent
            // comme des sabres laser.
            let queues = crate::comete::rubans_animes(
                p,
                c,
                cam.light_pos,
                self.base.vitesse,
                r,
                seuil,
                self.temps_comete,
                self.graine_comete,
            );
            for nuages in queues {
                let mut v = Vec::new();
                let mut i = Vec::new();
                // ⚠️ **Un chapelet de nuages, plus un ruban** : vue dans l'axe,
                // une bande se croisait en rayons (`comete::nuage`).
                for (centre, rayon, couleur) in nuages {
                    crate::impostor::push_quad(&mut v, &mut i, cam, centre, rayon, couleur);
                }
                let mesh =
                    Mesh { vertices: v, indices: i, texture: Some(crate::impostor::halo_partage()) };
                draw_mesh(&mesh);
            }
            gl_use_default_material();

            // **Le noyau : un patatoïde**, et non la sphère de l'impostor.
            //
            // ⚠️ **Éclairé sur le CPU**, et c'est ce qui évite un pipeline de
            // plus : le maillage fait deux cents sommets, un lambert par sommet
            // ne coûte rien, et macroquad sait dessiner un maillage à couleurs de
            // sommets sans material. La lumière est l'étoile primaire, là où elle
            // est vraiment — même source que la queue, juste au-dessus.
            if self.noyau.0.len() >= 3 {
                let vers_lumiere = (cam.light_pos - c).normalize_or_zero();
                let base = cam.rel(c);
                let verts: Vec<Vertex> = self
                    .noyau
                    .0
                    .iter()
                    .map(|(p, n, col)| {
                        // Un peu d'ambiante : la face nuit d'un caillou noir sur
                        // fond noir disparaîtrait, et avec elle sa silhouette.
                        let l = 0.12 + 0.88 * n.dot(vers_lumiere).max(0.0);
                        Vertex::new2(
                            base + *p * r,
                            vec2(0.5, 0.5),
                            Color::new(col.r * l, col.g * l, col.b * l, 1.0),
                        )
                    })
                    .collect();
                draw_mesh(&Mesh {
                    vertices: verts,
                    indices: self.noyau.1.clone(),
                    texture: None,
                });
            }
            return;
        }

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
        let demi = r * materiau::DISC;
        crate::impostor::push_quad(&mut self.verts, &mut self.inds, cam, c, demi, WHITE);

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
        // **Vider le lot MAINTENANT.**
        //
        // Toutes les planetes partagent le meme materiau (`mat_corps` rend des
        // clones du meme pipeline). `set_texture` n'attache donc pas la texture
        // a CE dessin : il l'ecrit dans le pipeline, et macroquad ne l'applique
        // qu'au vidage du lot. Sans ce flush, c'est le dernier corps dessine qui
        // impose ses textures a tous les precedents -- dans le systeme solaire,
        // une tellurique (donc `zonal = tex_vide`), et les quatre geantes
        // sortaient d'un brun uni faute de profil de bandes.
        //
        // La galerie y echappait par accident : elle change de viewport a chaque
        // cellule, ce qui vide le lot entre deux planetes. C'est pour ca que le
        // defaut ne se voyait QUE dans la vue systeme.
        unsafe {
            get_internal_gl().flush();
        }
        self.verts = quad.vertices;
        self.inds = quad.indices;
        gl_use_default_material();

        // --- Coquille atmosphérique ---
        //
        // APRÈS le corps : elle se dépose sur le limbe et déborde de la
        // silhouette, là où le corps a fait `discard`. AVANT la moitié avant de
        // l'anneau, qui doit passer devant elle comme devant la planète.
        if materiau::atmo_visible(&self.app, cam, c, r) {
            self.verts.clear();
            self.inds.clear();
            crate::impostor::push_quad(&mut self.verts, &mut self.inds, cam, c, demi, WHITE);
            materiau::appliquer_uniforms_atmo(&self.mat_atmo, &self.app, cam, c, r);
            gl_use_material(&self.mat_atmo);
            let halo = Mesh {
                vertices: std::mem::take(&mut self.verts),
                indices: std::mem::take(&mut self.inds),
                texture: None,
            };
            draw_mesh(&halo);
            // Même raison que pour le corps : le material est partagé, les
            // uniforms n'atteignent le GPU qu'au vidage du lot.
            unsafe {
                get_internal_gl().flush();
            }
            self.verts = halo.vertices;
            self.inds = halo.indices;
            gl_use_default_material();
        }

        // Anneau : moitié avant APRÈS le corps (passe devant la planète).
        if let Some(a) = &mut self.anneau {
            a.draw_moitie(cam, 1.0);
        }
    }
}
