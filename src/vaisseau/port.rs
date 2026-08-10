//! Modèle de points d'accroche (« ports »), cœur de l'assemblage par nœuds
//! façon Kerbal Space Program (voir `docs/conception/stations.md`, Partie A §3).
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
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
    #[allow(dead_code)] // Interaction de l'assembleur (Lot 4) : deja teste, pas encore branche a une vue.
    pub fn droite(&self) -> Vec3 {
        self.rot * Vec3::X
    }

    /// Transforme un point du repère local vers le repère parent.
    #[allow(dead_code)] // Interaction de l'assembleur (Lot 4) : deja teste, pas encore branche a une vue.
    pub fn transforme_point(&self, p: Vec3) -> Vec3 {
        self.rot * p + self.pos
    }

    /// Repère équivalent à une matrice **rigide** (rotation + translation, sans
    /// mise à l'échelle) — l'inverse de [`Self::to_mat4`].
    ///
    /// Sert au déplacement de sous-arbre (`Chantier::deplacer`, §10.10), où le
    /// delta se calcule naturellement en matrices (produit de deux poses cuites)
    /// mais doit s'appliquer à des `Repere` : ceux des ports libres, qui vivent
    /// en coordonnées monde.
    ///
    /// ⚠️ Ne vaut que pour une matrice rigide. Toutes celles qui circulent ici
    /// le sont — elles viennent de `Repere::to_mat4` ou d'un produit de
    /// telles matrices, et le produit de deux transformations rigides l'est
    /// aussi.
    pub fn depuis_mat4(m: Mat4) -> Repere {
        let (_, rot, pos) = m.to_scale_rotation_translation();
        Repere { pos, rot }
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
    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_rotation_translation(self.rot, self.pos)
    }
}

/// Genre de connexion — détermine quels ports peuvent s'apparier (§3.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
    #[allow(dead_code)] // Interaction de l'assembleur (Lot 4) : deja teste, pas encore branche a une vue.
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
    accoupler_roule(hote, montage, 0.0)
}

/// Accouplement **avec roulis** : même chose, tournée de `roulis` radians autour
/// de l'axe d'accostage (`conception/assembleur.md` §11.3).
///
/// Z est cet axe (`Repere::avant`) : tourner autour de lui *est* le roulis, et
/// c'est la seule rotation qui laisse la pièce accostée. Les deux hauts cessent
/// d'être alignés — c'est précisément ce qu'on veut pouvoir décider.
///
/// **À `roulis = 0`, rend exactement ce que rendait `accoupler`** : c'est la
/// propriété qui rend la migration sûre, et [`accoupler`] n'est plus qu'un appel
/// à zéro plutôt qu'un second calcul qui pourrait en diverger.
pub fn accoupler_roule(hote: Repere, montage: Repere, roulis: f32) -> Repere {
    let face_a_face = hote.rot * Quat::from_rotation_y(PI) * Quat::from_rotation_z(roulis);
    let rot = face_a_face * montage.rot.inverse();
    let pos = hote.pos - rot * montage.pos;
    Repere { pos, rot }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le roulis a zero ne change rien** — la garde de migration de §11.3.
    /// Sans elle, ajouter un parametre pourrait deplacer silencieusement toutes
    /// les pieces deja posees et toutes les recettes deja sauvegardees.
    #[test]
    fn le_roulis_nul_redonne_laccouplement_dorigine() {
        for (h, m) in cas_accouplement() {
            let a = accoupler(h, m);
            let b = accoupler_roule(h, m, 0.0);
            assert!(proche(a.pos, b.pos), "position deplacee a roulis nul");
            assert!(a.rot.abs_diff_eq(b.rot, 1e-6), "orientation changee a roulis nul");
        }
    }

    /// **Le roulis tourne autour de l'axe d'accostage, et de rien d'autre.**
    /// La piece reste accostee : son point de montage ne bouge pas, et son axe
    /// avant reste celui du port. Seul le *haut* tourne.
    #[test]
    fn le_roulis_tourne_autour_de_laxe_et_garde_le_contact() {
        for (h, m) in cas_accouplement() {
            let droit = accoupler(h, m);
            for angle in [0.4_f32, 1.2, -2.0, std::f32::consts::PI] {
                let roule = accoupler_roule(h, m, angle);
                // Le port de montage reste au meme point du monde : la piece
                // n'a pas decolle de son hote.
                let p0 = droit.compose(m).pos;
                let p1 = roule.compose(m).pos;
                assert!(proche(p0, p1), "angle {angle} : la piece a decolle du port");
                // Et son axe d'accostage est inchange.
                let a0 = droit.compose(m).avant();
                let a1 = roule.compose(m).avant();
                assert!(proche(a0, a1), "angle {angle} : l'axe d'accostage a bouge");
            }
        }
    }

    /// Un tour complet revient au point de depart : le roulis est bien un angle.
    #[test]
    fn un_tour_complet_de_roulis_redonne_la_meme_pose() {
        for (h, m) in cas_accouplement() {
            let a = accoupler_roule(h, m, 0.0);
            let b = accoupler_roule(h, m, std::f32::consts::TAU);
            assert!(proche(a.pos, b.pos));
            assert!(a.rot.abs_diff_eq(b.rot, 1e-5) || a.rot.abs_diff_eq(-b.rot, 1e-5));
        }
    }

    /// **Et le roulis fait vraiment quelque chose.** Sans ce test, les trois
    /// precedents passeraient sur une implementation qui ignore l'angle.
    #[test]
    fn un_roulis_non_nul_change_lorientation() {
        let (h, m) = cas_accouplement()[0];
        let droit = accoupler(h, m);
        let roule = accoupler_roule(h, m, std::f32::consts::FRAC_PI_2);
        assert!(
            !droit.rot.abs_diff_eq(roule.rot, 1e-3),
            "un quart de tour n'a rien change : le roulis est ignore"
        );
    }

    /// Quelques couples (hote, montage) varies : axes canoniques et orientations
    /// quelconques, pour que les gardes ne portent pas sur un seul cas facile.
    fn cas_accouplement() -> Vec<(Repere, Repere)> {
        vec![
            (Repere::IDENTITE, Repere::IDENTITE),
            (
                Repere::new(vec3(1.0, 2.0, -3.0), Quat::from_rotation_x(0.7)),
                Repere::new(vec3(0.0, 0.0, 0.5), Quat::IDENTITY),
            ),
            (
                Repere::new(vec3(-2.0, 0.5, 1.0), Quat::from_euler(EulerRot::XYZ, 0.3, -1.1, 2.2)),
                Repere::new(vec3(0.2, -0.4, 0.0), Quat::from_rotation_y(1.0)),
            ),
        ]
    }


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
