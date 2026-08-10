//! **Miroir primaire** — le grand réflecteur d'un tore de Stanford,
//! `docs/conception/stations.md` §D (mégastructures).
//!
//! C'est la pièce iconique du tore, et elle manquait : le preset n'avait que le
//! moyeu, les bras, les habitats et l'anneau. Sans miroir, rien n'explique
//! **d'où vient la lumière** dans un tore vitré — or c'est tout le principe de
//! l'objet : un disque incliné renvoie le soleil vers l'anneau, qui le reçoit
//! par ses jonctions vitrées.
//!
//! ## Un disque, pas une surface de révolution
//!
//! Le miroir est **plat, ou très légèrement creux**. On le construit en éventail
//! de triangles ([`Peintre::triangles`]) plutôt qu'en primitives composées : un
//! disque est la seule forme du parc qui n'est ni une boîte, ni un cylindre, ni
//! un panneau rectangulaire, et l'approcher par l'une des trois donnerait un
//! polygone visible ou un volume parasite.
//!
//! ## Il se pose, donc il est dans la palette
//!
//! ⚠️ `Composant::Tore` n'expose **aucun** port : il est rangé en « posé à la
//! main » et l'assembleur ne peut donc pas le proposer. Le miroir, lui, porte un
//! port de montage — c'est ce qui le rend constructible dans le VAB, et c'est
//! délibéré : une pièce de cette taille ne sert à rien si on ne peut pas
//! l'accrocher à un mât.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

/// Débord du miroir par rapport à l'anneau qu'il éclaire.
///
/// Le disque couvre l'anneau *et* le déborde largement. Au gabarit exact, la
/// jante resterait dans son ombre au moindre désalignement ; plus large, il
/// éclaire jusqu'au bord.
///
/// Parti de 110 %, porté à **165 %** après l'avoir vu incliné : une fois de
/// biais, un disque ne présente à l'anneau que sa projection — donc d'autant
/// moins de surface qu'il penche. La cote qui compte est ce qu'il **couvre**,
/// pas son rayon nu.
pub const DEBORD_ANNEAU: f32 = 1.65;

/// Rayon d'un miroir taillé pour un anneau de rayon `rayon_anneau`.
pub fn rayon_pour_anneau(rayon_anneau: f32) -> f32 {
    rayon_anneau * DEBORD_ANNEAU
}

/// Segments du bord. 64 : au-delà, un disque de cette taille ne gagne plus rien
/// à l'œil, et chaque segment coûte deux triangles sur une pièce déjà large.
const SEGMENTS: usize = 64;

/// Nervures dorsales. Elles sont ce qui distingue un miroir **construit** d'un
/// jeton flottant : une membrane de cette portée ne tient pas sans support.
const NERVURES: usize = 8;

/// Côtés de la couronne centrale. Assez pour lire un cercle, assez peu pour
/// qu'il reste une **structure** et non une surface.
const MOYEU_COTES: usize = 12;

/// Variantes de [`Composant::Miroir`].
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VarianteMiroir {
    /// Membrane pleine sur nervures — le grand réflecteur primaire.
    Primaire,
    /// Couronne réfléchissante percée en son centre : laisse passer le mât et
    /// l'axe du moyeu, ce qu'un disque plein obligerait à contourner.
    Anneau,
    /// Miroir segmenté en pétales, à la manière des grands réflecteurs
    /// déployables — mêmes cotes, lecture très différente.
    Petales,
}

impl VarianteMiroir {
    /// ⚠️ Meme limite que `VarianteArc` : la palette ne liste qu'une
    /// sous-variante par famille. Seul `Anneau` est atteignable a l'ecran,
    /// via le preset de Stanford.
    #[allow(dead_code)]
    pub const TOUS: [VarianteMiroir; 3] =
        [VarianteMiroir::Primaire, VarianteMiroir::Anneau, VarianteMiroir::Petales];

    #[allow(dead_code)] // voir la note sur TOUS
    pub fn nom(self) -> &'static str {
        match self {
            VarianteMiroir::Primaire => "MIROIR PRIMAIRE",
            VarianteMiroir::Anneau => "MIROIR ANNULAIRE",
            VarianteMiroir::Petales => "MIROIR A PETALES",
        }
    }

    pub(super) fn cout(self) -> f32 {
        match self {
            VarianteMiroir::Primaire => 30.0,
            VarianteMiroir::Anneau => 26.0,
            VarianteMiroir::Petales => 34.0,
        }
    }

    /// Rayon du trou central, en fraction du rayon. Zéro = disque plein.
    fn trou(self) -> f32 {
        match self {
            VarianteMiroir::Primaire => 0.0,
            VarianteMiroir::Anneau => 0.34,
            VarianteMiroir::Petales => 0.18,
        }
    }
}

/// Un seul port de montage, générique `Surface` comme tout appendice : le
/// miroir se boulonne au bout d'un mât ou sur le moyeu.
pub(super) fn ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(
        Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
        GenrePort::Surface,
        profil,
    )]
}

/// Point de la membrane, en repère local : le disque s'étale dans le plan XY et
/// sa normale est +Z, comme un panneau.
///
/// `creux` est la flèche au centre, en fraction du rayon : la membrane est
/// légèrement concave, ce qui la fait lire comme tendue plutôt que comme une
/// tôle découpée.
fn membrane(rayon: f32, creux: f32, r: f32, angle: f32) -> Vec3 {
    let t = (r / rayon).clamp(0.0, 1.0);
    // Parabole : plate au bord, creusée au centre.
    let z = -creux * rayon * (1.0 - t * t);
    vec3(r * angle.cos(), r * angle.sin(), z)
}

/// Teinte de la face **réfléchissante**, du centre (`t = 0`) au bord (`t = 1`).
///
/// Il n'y a ni texture ni normale dans ce parc de composants : une couleur
/// plate donnerait un jeton uni, et un miroir uni ne ressemble à rien. Le
/// dégradé radial — bleu profond au centre, blanc-bleu au bord — imite ce que
/// renvoie un réflecteur : le ciel au milieu, la lumière rasante sur la jante.
fn teinte_reflet(t: f32) -> Color {
    let a = Color::new(0.16, 0.30, 0.52, 1.0); // bleu sombre, au centre
    let b = Color::new(0.86, 0.92, 1.00, 1.0); // presque blanc, au bord
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        1.0,
    )
}

/// Bandes radiales du dégradé. Cinq suffisent à lire une transition ; chacune
/// coûte deux appels de dessin (une face, puis l'autre).
const BANDES: usize = 5;

pub(super) fn dessiner<P: Peintre>(p: &mut P, variante: VarianteMiroir, rayon: f32, creux: f32) {
    let dos = Color::new(0.26, 0.28, 0.33, 1.0);
    let metal = Color::new(0.55, 0.58, 0.63, 1.0);
    // Face arrière : bleu sombre. C'est le **dos** d'un miroir, pas un second
    // réflecteur — mais il doit se voir, sinon la pièce paraît trouée dès qu'on
    // passe derrière.
    let envers = Color::new(0.10, 0.15, 0.26, 1.0);
    let trou = variante.trou() * rayon;
    let petales = matches!(variante, VarianteMiroir::Petales);

    // --- La membrane, en bandes de quadrilatères ---
    //
    // Un éventail depuis le centre donnerait des triangles très étirés au bord ;
    // des bandes concentriques restent régulières, gèrent le cas percé sans
    // branche séparée, **et** portent le dégradé.
    //
    // ⚠️ **Chaque bande est émise DEUX fois, en sens inverse.** macroquad ne
    // double-face pas (c'est déjà pourquoi `vaisseau::panneau` se dessine deux
    // fois) : une seule orientation laissait toute une face du miroir
    // invisible — on voyait au travers dès qu'on passait derrière.
    let interne = trou.max(rayon * 0.02);
    for bande in 0..BANDES {
        let (f0, f1) = (bande as f32 / BANDES as f32, (bande + 1) as f32 / BANDES as f32);
        let mut sommets: Vec<Vec3> = Vec::with_capacity(SEGMENTS * 2 + 2);
        let mut indices: Vec<u16> = Vec::with_capacity(SEGMENTS * 6);
        for k in 0..=SEGMENTS {
            let a = TAU * k as f32 / SEGMENTS as f32;
            // Les pétales laissent un liseré vide entre eux : le rayon extérieur
            // rentre périodiquement, ce qui découpe la membrane sans la trouer.
            let creuse = petales && (k * 12 / SEGMENTS) % 2 == 1;
            let dehors = if creuse { rayon * 0.86 } else { rayon };
            let r0 = interne + (dehors - interne) * f0;
            let r1 = interne + (dehors - interne) * f1;
            sommets.push(membrane(rayon, creux, r0, a));
            sommets.push(membrane(rayon, creux, r1, a));
        }
        for k in 0..SEGMENTS {
            let i = (k * 2) as u16;
            indices.extend_from_slice(&[i, i + 1, i + 3, i, i + 3, i + 2]);
        }
        p.triangles(&sommets, &indices, teinte_reflet((f0 + f1) * 0.5));
        // Le même maillage, enroulé à l'envers : c'est l'autre face.
        let envers_idx: Vec<u16> = indices.chunks_exact(3).flat_map(|t| [t[0], t[2], t[1]]).collect();
        p.triangles(&sommets, &envers_idx, envers);
    }

    // --- Le dos : nervures radiales + jante ---
    for k in 0..NERVURES {
        let a = TAU * k as f32 / NERVURES as f32;
        let interne = membrane(rayon, creux, trou.max(rayon * 0.04), a);
        let externe = membrane(rayon, creux, rayon * 0.97, a);
        p.cylindre(
            interne - Vec3::Z * (rayon * 0.012),
            externe - Vec3::Z * (rayon * 0.012),
            rayon * 0.012,
            dos,
        );
    }
    // Jante : une couronne de cordes, assez fine pour ne pas masquer la surface.
    for k in 0..SEGMENTS / 2 {
        let (a, b) = (
            TAU * k as f32 / (SEGMENTS / 2) as f32,
            TAU * (k + 1) as f32 / (SEGMENTS / 2) as f32,
        );
        p.cylindre(
            membrane(rayon, creux, rayon, a),
            membrane(rayon, creux, rayon, b),
            rayon * 0.010,
            metal,
        );
    }

    // --- Le moyeu : ce par quoi il tient ---
    //
    // ⚠️ **Un anneau, pas un point.** Une première version plantait un mât au
    // centre et y faisait converger trois haubans : sur la variante percée, ce
    // mât flottait au milieu du trou, rattaché à rien de visible, et la pièce
    // paraissait tenue par un piquet. Un **cercle de membrures** au bord du
    // trou dit ce qui se passe vraiment — la membrane est tendue sur une
    // couronne, et c'est la couronne qui porte.
    //
    // Sans ce moyeu le miroir flotte, et rien ne dit de quel côté il regarde.
    let r_moyeu = interne.max(rayon * 0.14);
    let arriere = -Vec3::Z * (rayon * 0.05);
    let anneau = |k: usize| {
        let a = TAU * k as f32 / MOYEU_COTES as f32;
        membrane(rayon, creux, r_moyeu, a) + arriere
    };
    for k in 0..MOYEU_COTES {
        // La couronne elle-même.
        p.cylindre(anneau(k), anneau((k + 1) % MOYEU_COTES), rayon * 0.020, metal);
        // Et ses membrures, qui la relient à la membrane — vers l'extérieur,
        // là où la portée est la plus grande.
        let a = TAU * k as f32 / MOYEU_COTES as f32;
        p.cylindre(anneau(k), membrane(rayon, creux, rayon * 0.52, a), rayon * 0.014, dos);
    }
}

/// Le disque plus sa flèche : la sphère englobante d'un plat très large est
/// dominée par son rayon.
pub(super) fn rayon_local(rayon: f32, creux: f32) -> f32 {
    rayon.hypot(creux * rayon + rayon * 0.10)
}

pub(super) fn cout(variante: VarianteMiroir, rayon: f32) -> f32 {
    // Le coût suit la **surface**, pas le rayon : doubler le rayon quadruple la
    // membrane à tendre. Un coût linéaire ferait passer un miroir géant pour
    // deux fois un petit.
    variante.cout() * (rayon / 10.0).powi(2).max(0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le miroir déborde l'anneau, et du bon côté.** C'est la seule cote qui
    /// compte pour la pièce : trop juste, la jante reste dans l'ombre.
    ///
    /// ⚠️ **Encadré, pas épinglé.** Une première rédaction exigeait exactement
    /// 110 % — un verrou de valeur, que §6.6 écarte : il n'a de raison que le
    /// jour où on l'écrit, et il rougit à chaque réglage à l'œil sans rien avoir
    /// appris à personne. Le débord *exact* est un arbitrage visuel ; ce qui est
    /// une propriété, c'est qu'il déborde, et pas de façon absurde.
    #[test]
    fn le_miroir_deborde_l_anneau_qu_il_eclaire() {
        for anneau in [5.0_f32, 14.2, 120.0] {
            let r = rayon_pour_anneau(anneau);
            assert!(r > anneau, "miroir plus petit que son anneau : {r} vs {anneau}");
            assert!(
                r < anneau * 3.0,
                "miroir démesuré ({}× l'anneau) : ce n'est plus un réflecteur, c'est un couvercle",
                r / anneau
            );
        }
        // Et le facteur est le même à toute échelle : c'est une proportion, pas
        // une taille absolue.
        assert!(
            (rayon_pour_anneau(10.0) / 10.0 - rayon_pour_anneau(100.0) / 100.0).abs() < 1e-6,
            "le débord dépend de la taille de l'anneau"
        );
    }

    /// **La membrane reste dans le disque annoncé.** Un point qui en sortirait
    /// mettrait la géométrie hors de son enveloppe de collision, donc ferait
    /// traverser sans que l'anti-collision s'en aperçoive.
    #[test]
    fn la_membrane_tient_dans_le_rayon() {
        let (r, creux) = (14.0_f32, 0.06);
        for k in 0..64 {
            let a = TAU * k as f32 / 64.0;
            for f in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                let p = membrane(r, creux, r * f, a);
                assert!(
                    p.truncate().length() <= r + 1e-4,
                    "point hors du disque : {p:?}"
                );
                assert!(p.z <= 1e-6, "la membrane bombe du mauvais côté : z = {}", p.z);
            }
        }
    }

    /// **Le creux se creuse au centre et s'annule au bord** : c'est ce qui la
    /// fait lire comme tendue. Un creux appliqué uniformément donnerait un cône.
    #[test]
    fn le_creux_est_maximal_au_centre() {
        let (r, creux) = (14.0_f32, 0.06);
        let centre = membrane(r, creux, 0.0, 0.0).z;
        let bord = membrane(r, creux, r, 0.0).z;
        assert!(centre < bord, "le centre n'est pas creusé");
        assert!(bord.abs() < 1e-5, "le bord devrait être à plat, il est à {bord}");
        assert!((centre + creux * r).abs() < 1e-4, "flèche {centre} au lieu de {}", -creux * r);
        // Un miroir plat reste plat.
        assert_eq!(membrane(r, 0.0, 0.0, 0.0).z, 0.0);
    }

    /// **Le coût suit la surface.** Un miroir deux fois plus large coûte quatre
    /// fois plus : sinon le générateur poserait des réflecteurs géants pour rien.
    #[test]
    fn le_cout_suit_la_surface() {
        let petit = cout(VarianteMiroir::Primaire, 10.0);
        let grand = cout(VarianteMiroir::Primaire, 20.0);
        assert!(
            (grand / petit - 4.0).abs() < 1e-3,
            "doubler le rayon multiplie le coût par {}",
            grand / petit
        );
    }

    /// **Chaque variante a son nom et son coût, tous distincts.** Deux entrées
    /// de palette qui s'appelleraient pareil seraient indiscernables à l'écran,
    /// et un coût nul ferait un réflecteur géant gratuit.
    #[test]
    fn les_variantes_sont_nommees_et_facturees() {
        let mut noms: Vec<&str> = VarianteMiroir::TOUS.iter().map(|v| v.nom()).collect();
        noms.sort_unstable();
        let avant = noms.len();
        noms.dedup();
        assert_eq!(noms.len(), avant, "deux variantes de miroir portent le même nom");
        for v in VarianteMiroir::TOUS {
            assert!(v.cout() > 0.0, "{} est gratuite", v.nom());
            assert!((0.0..1.0).contains(&v.trou()), "{} : trou hors bornes", v.nom());
        }
    }

    /// **Le dégradé va du bleu sombre au blanc, et dans ce sens.** Sans lui la
    /// membrane est un jeton uni — il n'y a ni texture ni normale dans ce parc,
    /// donc c'est la seule façon de faire lire un réflecteur.
    #[test]
    fn le_reflet_s_eclaircit_du_centre_vers_le_bord() {
        let centre = teinte_reflet(0.0);
        let bord = teinte_reflet(1.0);
        let lum = |c: Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
        assert!(lum(bord) > lum(centre) * 1.5, "le dégradé ne s'éclaircit pas assez");
        // Et il reste **bleu** : le canal bleu domine partout, sinon on aurait
        // une tôle grise et non un miroir.
        for t in [0.0_f32, 0.5, 1.0] {
            let c = teinte_reflet(t);
            assert!(c.b >= c.r, "t={t} : la teinte a viré au chaud ({c:?})");
        }
        // Monotone : pas de bosse au milieu.
        let mut prec = lum(teinte_reflet(0.0));
        for k in 1..=10 {
            let l = lum(teinte_reflet(k as f32 / 10.0));
            assert!(l >= prec - 1e-6, "le dégradé revient en arrière à {k}");
            prec = l;
        }
    }

    /// **La membrane est émise des DEUX côtés.**
    ///
    /// macroquad ne double-face pas — `vaisseau::panneau` se dessine déjà deux
    /// fois pour cette raison. Une seule orientation laissait toute une face du
    /// miroir invisible : on voyait au travers dès qu'on passait derrière, ce
    /// que l'usage a signalé tout de suite.
    ///
    /// Le test compte ce qui sort du peintre, plutôt que de relire le code : un
    /// peintre d'essai enregistre les triangles, et l'on vérifie que chaque
    /// triangle a bien son jumeau **d'enroulement inverse**.
    #[test]
    fn chaque_triangle_a_son_jumeau_a_l_envers() {
        #[derive(Default)]
        struct Compteur {
            tris: Vec<[Vec3; 3]>,
        }
        impl Peintre for Compteur {
            fn cylindre(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
            fn cone(&mut self, _: Vec3, _: Vec3, _: f32, _: f32, _: f32, _: Color) {}
            fn sphere(&mut self, _: Vec3, _: f32, _: Color) {}
            fn cube(&mut self, _: Vec3, _: Vec3, _: Color) {}
            fn panneau(&mut self, _: Vec3, _: Vec3, _: Vec3, _: Color) {}
            fn ligne(&mut self, _: Vec3, _: Vec3, _: f32, _: Color) {}
            fn triangles(&mut self, sommets: &[Vec3], indices: &[u16], _: Color) {
                for t in indices.chunks_exact(3) {
                    self.tris.push([
                        sommets[t[0] as usize],
                        sommets[t[1] as usize],
                        sommets[t[2] as usize],
                    ]);
                }
            }
            fn empiler_transforme(&mut self, _: Mat4) {}
            fn depiler_transforme(&mut self) {}
        }

        let mut c = Compteur::default();
        dessiner(&mut c, VarianteMiroir::Primaire, 10.0, 0.05);
        assert!(!c.tris.is_empty(), "aucun triangle émis");
        assert_eq!(c.tris.len() % 2, 0, "nombre impair : un triangle n'a pas de jumeau");

        // Pour chaque triangle, il en existe un autre de mêmes sommets mais
        // d'enroulement oppose — donc de normale opposée.
        let normale = |t: &[Vec3; 3]| (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
        let mut apparies = 0;
        for t in &c.tris {
            let n = normale(t);
            if n == Vec3::ZERO {
                continue;
            }
            let jumeau = c.tris.iter().any(|u| {
                let m = normale(u);
                m.dot(n) < -0.99 && u.contains(&t[0]) && u.contains(&t[1]) && u.contains(&t[2])
            });
            assert!(jumeau, "triangle sans jumeau à l'envers : {t:?}");
            apparies += 1;
        }
        assert!(apparies > 10, "trop peu de triangles pour prouver quoi que ce soit");
    }

    /// L'enveloppe contient le disque, flèche comprise.
    #[test]
    fn le_rayon_local_contient_le_disque() {
        let (r, creux) = (14.0_f32, 0.06);
        let borne = rayon_local(r, creux);
        for k in 0..32 {
            let a = TAU * k as f32 / 32.0;
            for f in [0.0_f32, 0.5, 1.0] {
                assert!(membrane(r, creux, r * f, a).length() <= borne + 1e-4);
            }
        }
    }
}
