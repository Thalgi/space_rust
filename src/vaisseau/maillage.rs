//! Cuisson d'une station en **maillage figé**.
//!
//! Pourquoi : le rendu immédiat ré-émet la géométrie à chaque frame, et surtout
//! chaque `push_model_matrix` change l'uniforme `Model` (cf.
//! `shaders/station.vert.glsl`) — ce qui **force un draw call par primitive**.
//! Une station ISS, avec ses treillis (4 longerons + 6 barres par baie),
//! approche le millier d'appels par frame. Ici on génère les sommets **une
//! seule fois** (la `Station` est immuable après construction) et on dessine en
//! quelques `draw_mesh`.
//!
//! Deux conséquences de conception :
//! - **Tout est cuit dans le repère de la station**, pas en coordonnées monde :
//!   `Piece.transforme` est relatif à l'origine de la station, donc le maillage
//!   l'est aussi. On le dessine avec **une seule** `push_model_matrix(station →
//!   monde)`. C'est ce qui rend la station gratuite à déplacer (une matrice, pas
//!   une recuisson) et permet de **partager un maillage entre stations
//!   identiques**. Le shader reste correct : `v_wpos = Model * position` donne
//!   bien la position monde pour l'éclairage.
//! - **Les sommets sont partagés sans crainte** : l'éclairage dérive les
//!   normales de facette à l'écran (`dFdx/dFdy`), il n'y a donc pas de normales
//!   par sommet à casser sur les arêtes.
//!
//! Les indices d'un `Mesh` macroquad sont des `u16` : on découpe donc en
//! **lots**, comme `disque/mod.rs` et `disque/voile.rs` le font déjà.

use super::peintre::Peintre;
use super::Station;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

/// Tessellation des solides de révolution (cylindres, cônes, sphères).
/// C'est le curseur coût/qualité : macroquad tessellait ses cylindres autour de
/// cette valeur, on la garde pour ne pas changer la silhouette.
const SEGMENTS: usize = 20;

/// Nombre d'anneaux d'une sphère (hors pôles).
const ANNEAUX: usize = 10;

/// Plafond par lot. **Ce n'est pas la limite des indices `u16` (65 535)** mais
/// celle du *batcher* de macroquad : au-delà, `draw_mesh` abandonne le maillage
/// silencieusement — des composants entiers disparaissent. On reprend la valeur
/// éprouvée du reste du projet (`disque` : 400 quads = 1 600 sommets / 2 400
/// indices). Nos solides ont beaucoup plus d'indices que de sommets, d'où le
/// plafond **sur les deux** compteurs.
const MAX_SOMMETS: usize = 1_600;
const MAX_INDICES: usize = 2_400;

/// En dessous, une longueur/un rayon est considéré nul : on n'émet rien plutôt
/// que de produire des triangles dégénérés (normales de facette indéfinies).
const EPS: f32 = 1e-6;

/// Accumulateur de géométrie. On pose une transformée (celle de la pièce), on
/// émet des primitives dans le repère **local** de la pièce, et le bâtisseur
/// les convertit en monde.
pub struct Batisseur {
    lots: Vec<Mesh>,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
    transforme: Mat4,
}

impl Default for Batisseur {
    fn default() -> Self {
        Self::new()
    }
}

impl Batisseur {
    pub fn new() -> Self {
        Self {
            lots: Vec::new(),
            verts: Vec::new(),
            inds: Vec::new(),
            transforme: Mat4::IDENTITY,
        }
    }

    /// Transformée appliquée aux primitives émises ensuite (la `Mat4` cuite de
    /// la pièce en cours).
    pub fn poser_transforme(&mut self, m: Mat4) {
        self.transforme = m;
    }

    /// Clôt le lot courant et rend tous les lots prêts à dessiner.
    pub fn terminer(mut self) -> Vec<Mesh> {
        self.vider();
        self.lots
    }

    // ---- interne ----

    fn vider(&mut self) {
        if !self.inds.is_empty() {
            self.lots.push(Mesh {
                vertices: std::mem::take(&mut self.verts),
                indices: std::mem::take(&mut self.inds),
                texture: None,
            });
        }
    }

    /// Garantit qu'une primitive de `sommets`/`indices` tient dans le lot
    /// courant. À appeler **avant** de l'émettre : ses indices sont relatifs au
    /// lot, elle ne doit donc jamais être coupée en deux.
    fn reserver(&mut self, sommets: usize, indices: usize) {
        if self.verts.len() + sommets > MAX_SOMMETS || self.inds.len() + indices > MAX_INDICES {
            self.vider();
        }
    }

    /// Ajoute un sommet (position **locale**, convertie en monde) et renvoie son
    /// index dans le lot courant.
    fn sommet(&mut self, p: Vec3, couleur: Color) -> u16 {
        let monde = self.transforme.transform_point3(p);
        let i = self.verts.len() as u16;
        self.verts.push(Vertex::new2(monde, Vec2::ZERO, couleur));
        i
    }

    fn triangle(&mut self, a: u16, b: u16, c: u16) {
        self.inds.extend_from_slice(&[a, b, c]);
    }

    /// Quad `a,b,c,d` (dans l'ordre du contour) émis **des deux côtés** :
    /// macroquad ne double-face pas, et les panneaux/voiles se voient de dos.
    fn quad_indices(&mut self, a: u16, b: u16, c: u16, d: u16) {
        self.inds.extend_from_slice(&[a, b, c, a, c, d]);
        self.inds.extend_from_slice(&[a, c, b, a, d, c]);
    }

    /// Tronc de cône entre `a` (rayon `ra`) et `b` (rayon `rb`), fermé aux deux
    /// bouts. Toutes les révolutions (`cylindre`, `cone`) s'y ramènent.
    fn tronc(&mut self, a: Vec3, b: Vec3, ra: f32, rb: f32, couleur: Color) {
        let axe = b - a;
        let h = axe.length();
        if h < EPS || (ra.abs() < EPS && rb.abs() < EPS) {
            return;
        }
        let n = axe / h;
        let (u, v) = base_perp(n);

        // 2 anneaux partagés + 2 centres de capuchon ; 4 triangles par segment.
        self.reserver(2 * SEGMENTS + 2, SEGMENTS * 12);
        let base_a = self.verts.len() as u16;
        for k in 0..SEGMENTS {
            let (s, c) = angle(k).sin_cos();
            let d = u * c + v * s;
            self.sommet(a + d * ra, couleur);
        }
        let base_b = self.verts.len() as u16;
        for k in 0..SEGMENTS {
            let (s, c) = angle(k).sin_cos();
            let d = u * c + v * s;
            self.sommet(b + d * rb, couleur);
        }
        let centre_a = self.sommet(a, couleur);
        let centre_b = self.sommet(b, couleur);

        for k in 0..SEGMENTS {
            let kn = ((k + 1) % SEGMENTS) as u16;
            let k = k as u16;
            // Flanc — enroulement tel que la normale géométrique sorte du solide
            // (les capuchons ci-dessous suivent la même convention).
            self.triangle(base_a + k, base_b + kn, base_b + k);
            self.triangle(base_a + k, base_a + kn, base_b + kn);
            // Capuchons (les sommets d'anneau sont réutilisés : pas de normales
            // par sommet à préserver).
            self.triangle(centre_a, base_a + kn, base_a + k);
            self.triangle(centre_b, base_b + k, base_b + kn);
        }
    }
}

// Le bâtisseur est l'une des deux **sorties** de géométrie (l'autre étant le
// dessin immédiat) : c'est le trait qui garantit qu'un seul code de forme
// alimente les deux, donc aucune dérive visuelle entre galerie et station cuite.
impl Peintre for Batisseur {
    fn cylindre(&mut self, a: Vec3, b: Vec3, rayon: f32, couleur: Color) {
        self.tronc(a, b, rayon, rayon, couleur);
    }

    fn cone(&mut self, base: Vec3, direction: Vec3, r_base: f32, r_bout: f32, longueur: f32, couleur: Color) {
        let d = direction.normalize_or_zero();
        if d == Vec3::ZERO {
            return;
        }
        self.tronc(base, base + d * longueur, r_base, r_bout, couleur);
    }

    /// Sphère UV (nœuds, billes).
    fn sphere(&mut self, centre: Vec3, rayon: f32, couleur: Color) {
        if rayon.abs() < EPS {
            return;
        }
        let na = ANNEAUX;
        let ns = SEGMENTS;
        self.reserver((na + 1) * (ns + 1), na * ns * 6);
        let base = self.verts.len() as u16;
        for i in 0..=na {
            let phi = std::f32::consts::PI * (i as f32) / (na as f32);
            let (sp, cp) = phi.sin_cos();
            for j in 0..=ns {
                let theta = std::f32::consts::TAU * (j as f32) / (ns as f32);
                let (st, ct) = theta.sin_cos();
                let p = vec3(sp * ct, cp, sp * st) * rayon;
                self.sommet(centre + p, couleur);
            }
        }
        let pas = (ns + 1) as u16;
        for i in 0..na as u16 {
            for j in 0..ns as u16 {
                let a = base + i * pas + j;
                let b = a + pas;
                self.triangle(a, b + 1, b);
                self.triangle(a, a + 1, b + 1);
            }
        }
    }

    /// Boîte alignée sur les axes locaux (`taille` = dimensions pleines).
    fn cube(&mut self, centre: Vec3, taille: Vec3, couleur: Color) {
        let h = taille * 0.5;
        if h.x.abs() < EPS && h.y.abs() < EPS && h.z.abs() < EPS {
            return;
        }
        self.reserver(8, 36);
        let base = self.verts.len() as u16;
        for &(sx, sy, sz) in &[
            (-1.0f32, -1.0f32, -1.0f32),
            (1.0, -1.0, -1.0),
            (1.0, 1.0, -1.0),
            (-1.0, 1.0, -1.0),
            (-1.0, -1.0, 1.0),
            (1.0, -1.0, 1.0),
            (1.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
        ] {
            self.sommet(centre + vec3(h.x * sx, h.y * sy, h.z * sz), couleur);
        }
        // 6 faces, sommets dans l'ordre du contour.
        for &[a, b, c, d] in &[
            [0u16, 1, 2, 3], // -Z
            [5, 4, 7, 6],    // +Z
            [4, 0, 3, 7],    // -X
            [1, 5, 6, 2],    // +X
            [4, 5, 1, 0],    // -Y
            [3, 2, 6, 7],    // +Y
        ] {
            // Ordre inversé : la normale géométrique doit sortir de la boîte.
            self.inds
                .extend_from_slice(&[base + a, base + c, base + b, base + a, base + d, base + c]);
        }
    }

    /// Parallélogramme plein, visible des deux côtés (panneaux, voiles).
    fn panneau(&mut self, coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color) {
        if e1.length() < EPS || e2.length() < EPS {
            return;
        }
        self.reserver(4, 12);
        let a = self.sommet(coin, couleur);
        let b = self.sommet(coin + e1, couleur);
        let c = self.sommet(coin + e1 + e2, couleur);
        let d = self.sommet(coin + e2, couleur);
        self.quad_indices(a, b, c, d);
    }

    /// Segment épais : un `draw_line_3d` ne peut pas entrer dans un maillage de
    /// triangles, on le remplace par une **croix de deux quads** — visible sous
    /// tous les angles et indépendante de la caméra (contrairement à un
    /// billboard, impossible à cuire).
    fn triangles(&mut self, sommets: &[Vec3], indices: &[u16], couleur: Color) {
        // Chaque triangle est ré-émis avec ses propres sommets : un lot peut
        // ainsi être coupé n'importe où sans avoir à remapper des indices
        // partagés — un maillage brut peut dépasser à lui seul la capacité d'un
        // lot (c'est le cas d'un panneau à tuiles hexagonales).
        for t in indices.chunks_exact(3) {
            let (ia, ib, ic) = (t[0] as usize, t[1] as usize, t[2] as usize);
            if ia >= sommets.len() || ib >= sommets.len() || ic >= sommets.len() {
                continue; // indice hors bornes : on ignore plutôt que paniquer
            }
            self.reserver(3, 3);
            let a = self.sommet(sommets[ia], couleur);
            let b = self.sommet(sommets[ib], couleur);
            let c = self.sommet(sommets[ic], couleur);
            self.triangle(a, b, c);
        }
    }

    fn ligne(&mut self, a: Vec3, b: Vec3, epaisseur: f32, couleur: Color) {
        let axe = b - a;
        let h = axe.length();
        if h < EPS || epaisseur < EPS {
            return;
        }
        let n = axe / h;
        let (u, v) = base_perp(n);
        let r = epaisseur * 0.5;
        for d in [u, v] {
            self.panneau(a - d * r, axe, d * (2.0 * r), couleur);
        }
    }
}

/// Maillage cuit d'une station : sa géométrie figée, en **repère station**.
///
/// C'est le remplaçant du rendu immédiat pièce par pièce. Là où celui-ci
/// ré-émettait la géométrie et changeait l'uniforme `Model` à chaque primitive
/// (donc un draw call par primitive, ~1 000 pour une ISS), on dessine ici un
/// `draw_mesh` par lot. Le nombre de lots est dicté par la capacité du batcher
/// macroquad (cf. `MAX_SOMMETS`/`MAX_INDICES`), pas par la limite `u16`.
///
/// La station se place dans le monde par **une** `push_model_matrix` autour de
/// [`MaillageStation::dessiner`] : le maillage n'a pas à être recuit quand elle
/// bouge, et deux stations identiques peuvent partager le même.
pub struct MaillageStation {
    lots: Vec<Mesh>,
}

impl MaillageStation {
    /// Cuit la géométrie de toutes les pièces. Opération **pure** (aucun appel
    /// GL) : elle peut tourner hors contexte graphique, donc être testée.
    pub fn cuire(station: &Station) -> Self {
        let mut b = Batisseur::new();
        for piece in station.pieces() {
            b.poser_transforme(piece.transforme);
            piece.composant.dessiner(&mut b);
        }
        Self { lots: b.terminer() }
    }

    /// Dessine le maillage (repère caméra 3D et matrice modèle déjà en place).
    pub fn dessiner(&self) {
        for lot in &self.lots {
            draw_mesh(lot);
        }
    }

    /// Nombre de `draw_mesh` nécessaires — c'est le coût réel en draw calls.
    pub fn nb_lots(&self) -> usize {
        self.lots.len()
    }

    pub fn nb_sommets(&self) -> usize {
        self.lots.iter().map(|m| m.vertices.len()).sum()
    }

    pub fn nb_triangles(&self) -> usize {
        self.lots.iter().map(|m| m.indices.len() / 3).sum()
    }
}

fn angle(k: usize) -> f32 {
    std::f32::consts::TAU * (k as f32) / (SEGMENTS as f32)
}

/// Deux axes unitaires orthogonaux à `n` (supposé normalisé).
fn base_perp(n: Vec3) -> (Vec3, Vec3) {
    let aux = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = n.cross(aux).normalize();
    (u, n.cross(u))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::peintre::Peintre;

    const BLANC: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    /// Tous les indices de tous les lots pointent dans leur propre lot.
    fn indices_valides(lots: &[Mesh]) -> bool {
        lots.iter()
            .all(|m| m.indices.iter().all(|&i| (i as usize) < m.vertices.len()))
    }

    #[test]
    fn cylindre_compte_sommets_et_indices() {
        let mut b = Batisseur::new();
        b.cylindre(Vec3::ZERO, Vec3::Z, 1.0, BLANC);
        let lots = b.terminer();
        assert_eq!(lots.len(), 1);
        // 2 anneaux partagés + 2 centres de capuchon.
        assert_eq!(lots[0].vertices.len(), 2 * SEGMENTS + 2);
        // Par segment : 2 triangles de flanc + 1 par capuchon = 4 triangles.
        assert_eq!(lots[0].indices.len(), SEGMENTS * 4 * 3);
        assert!(indices_valides(&lots));
    }

    #[test]
    fn primitives_produisent_des_indices_valides() {
        let mut b = Batisseur::new();
        b.cylindre(Vec3::ZERO, Vec3::Z * 2.0, 0.5, BLANC);
        b.cone(Vec3::ZERO, Vec3::Y, 1.0, 0.2, 2.0, BLANC);
        b.sphere(Vec3::ZERO, 1.0, BLANC);
        b.cube(Vec3::ZERO, Vec3::ONE, BLANC);
        b.panneau(Vec3::ZERO, Vec3::X, Vec3::Y, BLANC);
        b.ligne(Vec3::ZERO, Vec3::X, 0.05, BLANC);
        let lots = b.terminer();
        assert!(indices_valides(&lots));
        assert!(lots.iter().all(|m| m.indices.len() % 3 == 0), "triangles complets");
    }

    // Cas limite : géométrie dégénérée → rien émis, pas de panique ni de
    // triangle plat (dont la normale de facette serait indéfinie).
    #[test]
    fn degenere_nemet_rien() {
        let mut b = Batisseur::new();
        b.cylindre(Vec3::ZERO, Vec3::ZERO, 1.0, BLANC); // longueur nulle
        b.cylindre(Vec3::ZERO, Vec3::Z, 0.0, BLANC); // rayon nul
        b.sphere(Vec3::ZERO, 0.0, BLANC);
        b.panneau(Vec3::ZERO, Vec3::ZERO, Vec3::Y, BLANC);
        b.ligne(Vec3::ZERO, Vec3::ZERO, 0.1, BLANC);
        b.cone(Vec3::ZERO, Vec3::ZERO, 1.0, 1.0, 1.0, BLANC); // direction nulle
        assert!(b.terminer().is_empty());
    }

    // La transformée de la pièce est bien appliquée à l'émission (cuisson en
    // coordonnées monde).
    #[test]
    fn transforme_appliquee() {
        let mut b = Batisseur::new();
        b.poser_transforme(Mat4::from_translation(vec3(10.0, 0.0, 0.0)));
        b.cube(Vec3::ZERO, Vec3::ONE, BLANC);
        let lots = b.terminer();
        assert!(lots[0].vertices.iter().all(|v| v.position.x >= 9.0));
    }

    // Deux transformées successives ne se cumulent pas : chaque pièce pose la
    // sienne.
    #[test]
    fn transforme_remplacee_pas_cumulee() {
        let mut b = Batisseur::new();
        b.poser_transforme(Mat4::from_translation(vec3(10.0, 0.0, 0.0)));
        b.poser_transforme(Mat4::from_translation(vec3(0.0, 5.0, 0.0)));
        b.cube(Vec3::ZERO, Vec3::ONE, BLANC);
        let lots = b.terminer();
        assert!(lots[0].vertices.iter().all(|v| v.position.x.abs() < 1.0));
        assert!(lots[0].vertices.iter().all(|v| v.position.y >= 4.0));
    }

    /// Garde-fou central : **aucun lot ne doit dépasser la capacité du batcher
    /// macroquad**, sans quoi `draw_mesh` l'abandonne en silence et des
    /// composants entiers disparaissent à l'écran (régression vécue).
    fn lots_dans_les_capacites(lots: &[Mesh]) -> bool {
        lots.iter().all(|m| {
            m.vertices.len() <= MAX_SOMMETS
                && m.indices.len() <= MAX_INDICES
                && m.vertices.len() <= u16::MAX as usize
        })
    }

    #[test]
    fn decoupage_respecte_la_capacite_du_batcher() {
        let mut b = Batisseur::new();
        for i in 0..2000 {
            let z = i as f32;
            b.cylindre(vec3(0.0, 0.0, z), vec3(0.0, 0.0, z + 1.0), 0.5, BLANC);
        }
        let lots = b.terminer();
        assert!(lots.len() > 1, "le découpage doit produire plusieurs lots");
        assert!(lots_dans_les_capacites(&lots));
        assert!(indices_valides(&lots));
    }

    // Un maillage brut peut à lui seul dépasser la capacité d'un lot : il doit
    // être découpé, pas tronqué ni émis d'un bloc.
    #[test]
    fn triangles_bruts_volumineux_sont_decoupes() {
        let mut b = Batisseur::new();
        let sommets: Vec<Vec3> = (0..3000).map(|i| vec3(i as f32, 0.0, 0.0)).collect();
        let indices: Vec<u16> = (0..2999u16).flat_map(|i| [0u16, i, i + 1]).collect();
        b.triangles(&sommets, &indices, BLANC);
        let lots = b.terminer();
        assert!(lots.len() > 1);
        assert!(lots_dans_les_capacites(&lots));
        assert!(indices_valides(&lots));
    }

    // Un bâtisseur sans émission ne produit aucun lot (pas de Mesh vide, que
    // macroquad n'aurait aucune raison de dessiner).
    #[test]
    fn vide_ne_produit_aucun_lot() {
        assert!(Batisseur::new().terminer().is_empty());
    }

    // La cuisson est **pure** (aucun appel GL) : on peut donc vérifier hors
    // contexte graphique qu'une station donne un maillage valide et compact.
    #[test]
    fn cuisson_station_produit_un_maillage_valide() {
        use crate::vaisseau::{Composant, Piece, Profil, Station, VarianteModule};
        let comp = Composant::ModuleAxial {
            profil: Profil::P1,
            variante: VarianteModule::Hublots,
            longueur: 3.0,
        };
        let station = Station::depuis_pieces(vec![
            Piece::new(Mat4::IDENTITY, comp),
            Piece::new(Mat4::from_translation(vec3(0.0, 0.0, 5.0)), comp),
        ])
        .unwrap();
        let m = MaillageStation::cuire(&station);
        assert!(m.nb_sommets() > 0);
        assert!(m.nb_triangles() > 0);
        assert!(indices_valides(&m.lots));
        // Le point qui a causé la régression : chaque lot doit rester dessinable.
        assert!(lots_dans_les_capacites(&m.lots));
    }

    // La transformée de chaque pièce est bien prise en compte : deux modules
    // écartés couvrent une plage en Z plus large qu'un seul.
    #[test]
    fn cuisson_respecte_les_transformees_de_pieces() {
        use crate::vaisseau::{Composant, Piece, Profil, Station, VarianteModule};
        let comp = Composant::ModuleAxial {
            profil: Profil::P1,
            variante: VarianteModule::Standard,
            longueur: 3.0,
        };
        let etendue = |st: &Station| {
            let m = MaillageStation::cuire(st);
            let zs: Vec<f32> = m.lots.iter().flat_map(|l| l.vertices.iter().map(|v| v.position.z)).collect();
            zs.iter().cloned().fold(f32::MIN, f32::max) - zs.iter().cloned().fold(f32::MAX, f32::min)
        };
        let seule = Station::depuis_pieces(vec![Piece::new(Mat4::IDENTITY, comp)]).unwrap();
        let ecartees = Station::depuis_pieces(vec![
            Piece::new(Mat4::IDENTITY, comp),
            Piece::new(Mat4::from_translation(vec3(0.0, 0.0, 20.0)), comp),
        ])
        .unwrap();
        assert!(etendue(&ecartees) > etendue(&seule) + 15.0);
    }
}
