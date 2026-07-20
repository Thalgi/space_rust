//! Modèle de points d'accroche (« ports »), cœur de l'assemblage par nœuds
//! façon Kerbal Space Program (voir `docs/stations_procedurales.md`, §3).
//!
//! Un composant expose des **ports** : des repères orientés par lesquels il se
//! clipse sur un port libre d'un composant déjà posé. [`accoupler`] calcule la
//! transformée monde de l'enfant pour que son port de montage vienne
//! **face à face** avec le port hôte.

use super::Profil;
use macroquad::prelude::*;
use std::f32::consts::PI;

/// Repère orthonormé orienté : position + rotation. Conventions :
/// **avant** = `rot * Z` (sens d'accouplement sortant), **haut** = `rot * Y`,
/// **droite** = `rot * X`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Repere {
    pub pos: Vec3,
    pub rot: Quat,
}

impl Repere {
    pub const IDENTITE: Repere = Repere { pos: Vec3::ZERO, rot: Quat::IDENTITY };

    pub fn new(pos: Vec3, rot: Quat) -> Self {
        Self { pos, rot }
    }

    /// Axe d'accouplement sortant.
    pub fn avant(&self) -> Vec3 {
        self.rot * Vec3::Z
    }
    pub fn haut(&self) -> Vec3 {
        self.rot * Vec3::Y
    }
    pub fn droite(&self) -> Vec3 {
        self.rot * Vec3::X
    }

    /// Transforme un point du repère local vers le repère parent.
    pub fn transforme_point(&self, p: Vec3) -> Vec3 {
        self.rot * p + self.pos
    }

    /// Compose un repère local exprimé dans *ce* repère → repère parent.
    /// (Sert à obtenir le repère monde d'un port à partir du repère monde du
    /// corps.)
    pub fn compose(&self, local: Repere) -> Repere {
        Repere {
            pos: self.rot * local.pos + self.pos,
            rot: self.rot * local.rot,
        }
    }

    /// Matrice modèle pour le rendu (`push_model_matrix`).
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rot, self.pos)
    }
}

/// Genre de connexion — détermine quels ports peuvent s'apparier (§3.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GenrePort {
    /// Écoutille en bout de module (accouplement axial).
    ModuleAxial,
    /// Face d'un nœud (accouplement radial).
    ModuleRadial,
    /// Extrémité d'une poutre / treillis.
    PoutreBout,
    /// Montage d'appendice externe **générique** : panneau solaire, radiateur,
    /// antenne, capteur… Un port hôte `Surface` accepte n'importe quel appendice
    /// (le `profil` gère la taille) — c'est le point de montage **factorisé**.
    Surface,
}

impl GenrePort {
    /// Table de compatibilité (symétrique). Un module peut se poser aussi bien
    /// sur un port axial que radial ; les appendices partagent tous le montage
    /// `Surface` (factorisé) ; les autres genres s'apparient à l'identique.
    pub fn compatible(self, autre: GenrePort) -> bool {
        use GenrePort::*;
        matches!(
            (self, autre),
            (ModuleAxial | ModuleRadial, ModuleAxial | ModuleRadial)
                | (PoutreBout, PoutreBout)
                | (Surface, Surface)
        )
    }
}

/// Un point d'accroche d'un composant : repère local orienté + genre + profil.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Port {
    pub repere: Repere,
    pub genre: GenrePort,
    pub profil: Profil,
}

impl Port {
    pub fn new(repere: Repere, genre: GenrePort, profil: Profil) -> Self {
        Self { repere, genre, profil }
    }

    /// Deux ports s'accouplent si leurs **genres** sont compatibles **et** leurs
    /// **profils** identiques.
    pub fn compatible(&self, autre: &Port) -> bool {
        self.genre.compatible(autre.genre) && self.profil.compatible(autre.profil)
    }
}

/// Transformée monde de l'enfant pour clipser son port de montage `montage`
/// (repère local à l'enfant) sur le port hôte `hote` (repère **monde**).
///
/// On veut : les positions des deux ports coïncident, et l'**avant** du port de
/// montage s'oppose à l'avant du port hôte (face à face), les **hauts** restant
/// alignés. Le demi-tour se fait autour du haut de l'hôte.
pub fn accoupler(hote: Repere, montage: Repere) -> Repere {
    let face_a_face = hote.rot * Quat::from_rotation_y(PI);
    let rot = face_a_face * montage.rot.inverse();
    let pos = hote.pos - rot * montage.pos;
    Repere { pos, rot }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proche(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    // ---- Repère ----

    #[test]
    fn axes_de_lidentite() {
        let r = Repere::IDENTITE;
        assert!(proche(r.avant(), Vec3::Z));
        assert!(proche(r.haut(), Vec3::Y));
        assert!(proche(r.droite(), Vec3::X));
    }

    #[test]
    fn compose_identite_neutre() {
        let r = Repere::new(vec3(1.0, 2.0, 3.0), Quat::from_rotation_x(0.5));
        let c = Repere::IDENTITE.compose(r);
        assert!(proche(c.pos, r.pos));
        assert!(proche(c.avant(), r.avant()));
    }

    #[test]
    fn compose_transforme_comme_transforme_point() {
        let corps = Repere::new(vec3(0.0, 1.0, 0.0), Quat::from_rotation_z(0.9));
        let local = Repere::new(vec3(0.3, 0.0, 0.2), Quat::from_rotation_y(0.4));
        let monde = corps.compose(local);
        assert!(proche(monde.pos, corps.transforme_point(local.pos)));
    }

    // ---- accoupler : invariants ----

    // Invariant fondamental : les positions des ports coïncident (cas identité).
    #[test]
    fn accoupler_positions_coincident_identite() {
        let hote = Repere::new(vec3(5.0, 0.0, 0.0), Quat::IDENTITY);
        let montage = Repere::new(vec3(0.0, 0.0, 0.5), Quat::IDENTITY);
        let corps = accoupler(hote, montage);
        let port_monde = corps.compose(montage);
        assert!(proche(port_monde.pos, hote.pos));
    }

    // Invariant fondamental sous rotations non triviales de l'hôte ET du montage.
    #[test]
    fn accoupler_positions_coincident_rotations() {
        let hote = Repere::new(vec3(1.0, 2.0, 3.0), Quat::from_rotation_x(0.7));
        let montage = Repere::new(vec3(0.2, -0.1, 0.4), Quat::from_rotation_z(1.1));
        let corps = accoupler(hote, montage);
        let port_monde = corps.compose(montage);
        assert!(proche(port_monde.pos, hote.pos), "{:?}", port_monde.pos);
    }

    // Face à face : l'avant du port de montage s'oppose à l'avant de l'hôte.
    #[test]
    fn accoupler_face_a_face() {
        let hote = Repere::new(vec3(1.0, 2.0, 3.0), Quat::from_rotation_x(0.7));
        let montage = Repere::new(vec3(0.2, -0.1, 0.4), Quat::from_rotation_z(1.1));
        let corps = accoupler(hote, montage);
        let port_monde = corps.compose(montage);
        assert!(proche(port_monde.avant(), -hote.avant()));
    }

    // Roulis : les « hauts » restent alignés.
    #[test]
    fn accoupler_hauts_alignes() {
        let hote = Repere::new(Vec3::ZERO, Quat::from_rotation_x(0.7));
        let montage = Repere::new(vec3(0.0, 0.0, 0.5), Quat::from_rotation_z(1.1));
        let corps = accoupler(hote, montage);
        let port_monde = corps.compose(montage);
        assert!(proche(port_monde.haut(), hote.haut()));
    }

    // Chaînage : un enfant clipsé sur un port de l'enfant précédent garde les
    // invariants au 2ᵉ joint.
    #[test]
    fn accoupler_chainage() {
        // Corps A à l'origine ; port hôte libre sur sa face +Z.
        let a_port = Repere::new(vec3(0.0, 0.0, 1.0), Quat::IDENTITY);
        // Corps B : port de montage sur -Z (avant = +Z), un autre port hôte sur +Z.
        let b_montage = Repere::new(vec3(0.0, 0.0, -1.0), Quat::IDENTITY);
        let b_hote_local = Repere::new(vec3(0.0, 0.0, 1.0), Quat::IDENTITY);
        let corps_b = accoupler(a_port, b_montage);
        // Vérifie joint 1.
        assert!(proche(corps_b.compose(b_montage).pos, a_port.pos));
        // Corps C clipsé sur le port hôte de B (en monde).
        let b_hote_monde = corps_b.compose(b_hote_local);
        let c_montage = Repere::new(vec3(0.0, 0.0, -0.5), Quat::IDENTITY);
        let corps_c = accoupler(b_hote_monde, c_montage);
        let joint2 = corps_c.compose(c_montage);
        assert!(proche(joint2.pos, b_hote_monde.pos));
        assert!(proche(joint2.avant(), -b_hote_monde.avant()));
    }

    // ---- Compatibilité ----

    #[test]
    fn compat_meme_genre_meme_profil() {
        let a = Port::new(Repere::IDENTITE, GenrePort::Surface, Profil::P1);
        let b = Port::new(Repere::IDENTITE, GenrePort::Surface, Profil::P1);
        assert!(a.compatible(&b));
    }

    #[test]
    fn compat_module_axial_radial_interchangeables() {
        let ax = Port::new(Repere::IDENTITE, GenrePort::ModuleAxial, Profil::P2);
        let ra = Port::new(Repere::IDENTITE, GenrePort::ModuleRadial, Profil::P2);
        assert!(ax.compatible(&ra));
        assert!(ra.compatible(&ax)); // symétrique
    }

    #[test]
    fn compat_profil_different_incompatible() {
        let a = Port::new(Repere::IDENTITE, GenrePort::ModuleAxial, Profil::P1);
        let b = Port::new(Repere::IDENTITE, GenrePort::ModuleAxial, Profil::P2);
        assert!(!a.compatible(&b));
    }

    #[test]
    fn compat_genre_different_incompatible() {
        let a = Port::new(Repere::IDENTITE, GenrePort::Surface, Profil::P1);
        let b = Port::new(Repere::IDENTITE, GenrePort::ModuleAxial, Profil::P1);
        assert!(!a.compatible(&b));
    }

    #[test]
    fn compat_genres_symetrique_et_reflexive() {
        use GenrePort::*;
        let tous = [ModuleAxial, ModuleRadial, PoutreBout, Surface];
        for a in tous {
            assert!(a.compatible(a), "réflexivité {a:?}");
            for b in tous {
                assert_eq!(a.compatible(b), b.compatible(a), "symétrie {a:?}/{b:?}");
            }
        }
    }
}
