//! **Overlay de débogage des enveloppes de collision.**
//!
//! Conception : `docs/conception/assembleur.md` §8.5.
//!
//! Il ne sert pas à « voir les capsules » — il répond à **une** question, celle
//! qui rendra l'assembleur incompréhensible si elle reste sans réponse :
//! *pourquoi ce port refuse-t-il ma pièce ?* Un port rouge sans explication est
//! un bug du point de vue de l'utilisateur, même quand le refus est correct.
//!
//! D'où le contenu : les enveloppes en fil de fer, **et surtout** le segment de
//! plus courte approche entre deux d'entre elles, qui transforme le refus en
//! phrase — « ce radiateur est à 1,2 de cette poutre, il en faudrait 1,8 ».
//!
//! Deuxième usage, immédiat : personne n'a encore **vu** une seule des
//! enveloppes converties en capsules (`suivi/assembleur.md` L1.6). Les tests
//! mesurent qu'elles *contiennent* la pièce ; ils ne disent rien de leur
//! **serre**, et une enveloppe peut contenir la pièce en restant ridiculement
//! large. Ça, seul l'œil le dit.

use crate::vaisseau::{Enveloppe, Noyau, Piece, Station};
use macroquad::prelude::*;

/// Segments du contour d'un anneau. 16 suffit : c'est un repère de débogage,
/// pas de la géométrie — et un fil trop dense masque la pièce qu'il entoure.
const SEGMENTS: usize = 16;

/// Anneaux intermédiaires le long du fût, en plus des deux bouts. Ils donnent
/// la **direction** de l'axe d'un coup d'œil, ce que deux anneaux seuls ne font
/// pas quand la capsule est vue en raccourci.
const ANNEAUX: usize = 3;

/// Enveloppe **au repos** : cyan sombre, elle doit rester derrière la pièce.
pub const CALME: Color = Color { r: 0.20, g: 0.65, b: 0.75, a: 1.0 };
/// Enveloppe de la pièce **proposée** (le fantôme).
///
/// Consommée depuis L5.2 : l'assembleur trace ce conflit dès qu'on vise un port
/// rouge.
pub const PROPOSEE: Color = Color { r: 1.00, g: 1.00, b: 1.00, a: 1.0 };
/// Enveloppe **en cause** dans un refus, et le segment de plus courte approche.
pub const REFUS: Color = Color { r: 1.00, g: 0.25, b: 0.20, a: 1.0 };

/// Repère orthonormé dont `w` est l'axe donné. Sert à tracer les anneaux
/// perpendiculairement au fût.
fn base(axe: Vec3) -> (Vec3, Vec3) {
    let w = axe.normalize_or_zero();
    if w == Vec3::ZERO {
        return (Vec3::X, Vec3::Y);
    }
    // On prend le plus petit axe canonique pour éviter un produit vectoriel
    // dégénéré quand `w` est déjà proche de l'un d'eux.
    let aide = if w.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = w.cross(aide).normalize();
    (u, w.cross(u))
}

/// Trace un anneau de rayon `r` centré en `c`, dans le plan `(u, v)`.
fn anneau(c: Vec3, u: Vec3, v: Vec3, r: f32, couleur: Color) {
    let point = |k: usize| {
        let a = std::f32::consts::TAU * k as f32 / SEGMENTS as f32;
        c + (u * a.cos() + v * a.sin()) * r
    };
    let mut prec = point(0);
    for k in 1..=SEGMENTS {
        let p = point(k);
        draw_line_3d(prec, p, couleur);
        prec = p;
    }
}

/// Dessine une enveloppe en fil de fer.
///
/// Deux noyaux, deux tracés — dispatché une seule fois ici, pour que le reste
/// du fichier (station, conflit) ne connaisse jamais la forme.
pub fn fil(env: &Enveloppe, couleur: Color) {
    match env.noyau {
        Noyau::Segment { a, b } => fil_capsule(a, b, env.rayon, couleur),
        Noyau::Rectangle { centre, eu, ev, hu, hv } => fil_boudin(centre, eu, ev, hu, hv, env.rayon, couleur),
    }
}

/// Capsule `[a, b]` gonflée de `r` : les anneaux du fût, quatre génératrices,
/// et les calottes des deux bouts.
///
/// Une sphère (capsule dégénérée) se réduit d'elle-même à trois grands cercles
/// — le fût est de longueur nulle, les anneaux se superposent et seules les
/// calottes restent visibles. Aucun cas particulier à écrire.
fn fil_capsule(a: Vec3, b: Vec3, r: f32, couleur: Color) {
    let axe = b - a;
    let (u, v) = base(if axe.length_squared() > 1e-9 { axe } else { Vec3::Z });

    // Fût : anneaux répartis d'un bout à l'autre.
    if axe.length_squared() > 1e-9 {
        for k in 0..=ANNEAUX + 1 {
            let t = k as f32 / (ANNEAUX + 1) as f32;
            anneau(a + axe * t, u, v, r, couleur);
        }
        // Génératrices : quatre traits qui relient les deux bouts. Sans elles,
        // une pile d'anneaux ne se lit pas comme un volume.
        for d in [u, -u, v, -v] {
            draw_line_3d(a + d * r, b + d * r, couleur);
        }
    }

    // Calottes : deux demi-cercles par bout, dans les deux plans qui contiennent
    // l'axe. C'est ce qui distingue une capsule d'un cylindre à l'écran, et donc
    // ce qui montre le **débord en bout** qu'on a choisi d'accepter.
    let w = if axe.length_squared() > 1e-9 { axe.normalize() } else { Vec3::Z };
    for (bout, sens) in [(a, -1.0_f32), (b, 1.0)] {
        for lat in [u, v] {
            let mut prec = bout + lat * r;
            for k in 1..=SEGMENTS / 2 {
                let ang = std::f32::consts::PI * k as f32 / (SEGMENTS / 2) as f32;
                let p = bout + lat * (r * ang.cos()) + w * (sens * r * ang.sin());
                draw_line_3d(prec, p, couleur);
                prec = p;
            }
        }
    }
}

/// Rectangle `(centre, eu, ev, hu, hv)` gonflé de `r` : deux faces parallèles
/// décalées de `±r` le long de la normale, reliées à leurs quatre coins — un
/// pavé aplati, pas les coins arrondis exacts du vrai boudin (une somme de
/// Minkowski rectangle+sphère). C'est un repère de débogage : la lecture
/// (« la plaque a cette épaisseur-là, gonflée d'un rayon ») compte plus que
/// l'arrondi exact des arêtes.
fn fil_boudin(centre: Vec3, eu: Vec3, ev: Vec3, hu: f32, hv: f32, r: f32, couleur: Color) {
    let n = eu.cross(ev);
    let coin = |su: f32, sv: f32, sn: f32| centre + eu * (su * hu) + ev * (sv * hv) + n * (sn * r);
    for sn in [-1.0_f32, 1.0] {
        let coins = [coin(1.0, 1.0, sn), coin(1.0, -1.0, sn), coin(-1.0, -1.0, sn), coin(-1.0, 1.0, sn)];
        for k in 0..4 {
            draw_line_3d(coins[k], coins[(k + 1) % 4], couleur);
        }
    }
    for (su, sv) in [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)] {
        draw_line_3d(coin(su, sv, -1.0), coin(su, sv, 1.0), couleur);
    }
}

/// L'enveloppe d'**une** pièce. Sert au surlignage du sous-arbre sélectionné
/// dans l'assembleur (L4.5) : le volume que le modèle connaît déjà, plutôt
/// qu'une seconde géométrie de surlignage inventée pour l'occasion.
pub fn piece(p: &Piece, couleur: Color) {
    fil(&p.composant.enveloppe_locale().transformee(p.transforme), couleur);
}

/// Toutes les enveloppes d'une station, au repos.
pub fn station(st: &Station, couleur: Color) {
    for p in st.pieces() {
        piece(p, couleur);
    }
}

/// **Le cœur de l'overlay** : ce qui, dans `st`, s'oppose à `propose`.
///
/// Dessine l'enveloppe proposée en blanc, celles qui la refusent en rouge, et
/// entre chacune et elle le **segment de plus courte approche** — le trait qui
/// dit d'où vient le refus et de combien il manque.
///
/// `marge` est le facteur d'adjacence de `Chantier` (`FACTEUR_COLLISION`) : on
/// reprend **exactement** son critère, sinon l'overlay expliquerait un refus
/// que le modèle n'a pas prononcé, ce qui serait pire que pas d'overlay du tout.
///
/// Renvoie le nombre de gêneurs et le **manque** le plus criant — l'écart le
/// plus négatif —, pour que la vue puisse écrire la phrase de §8.5 : *« ce
/// radiateur est à 1,2 de cette poutre, il en faut 1,8 »*.
pub fn conflit(
    st: &Station,
    propose: &Enveloppe,
    marge: f32,
    exempte: Option<usize>,
) -> (usize, f32) {
    fil(propose, PROPOSEE);
    let genes = gene_par(st, propose, marge, exempte);
    let mut pire = 0.0_f32;
    for &(i, manque) in &genes {
        pire = pire.min(manque);
        let piece = &st.pieces()[i];
        let sienne = piece.composant.enveloppe_locale().transformee(piece.transforme);
        fil(&sienne, REFUS);
        // Le segment de plus courte approche, matérialisé entre les deux
        // **axes** : c'est sur eux que porte le critère, et le montrer entre les
        // surfaces donnerait un trait qui ne correspond à aucun calcul.
        let (p, q) = plus_proches(propose, &sienne);
        draw_line_3d(p, q, REFUS);
        draw_sphere(p, 0.08, None, REFUS);
        draw_sphere(q, 0.08, None, REFUS);
    }
    (genes.len(), pire)
}

/// **Qui s'oppose à `propose`, et de combien il manque** — la *décision* de
/// l'overlay, séparée de son dessin (§10.9) et donc testable.
///
/// Rend, pour chaque pièce gênante, son indice dans la station et le manque :
/// distance des axes moins le seuil, donc **négatif**, et c'est exactement ce
/// qu'il faudrait gagner pour que la pose passe.
///
/// ⚠️ Le critère est **littéralement** celui de `Chantier` — distance des axes
/// contre `marge × (r1 + r2)`. Un overlay qui expliquerait un refus que le
/// modèle n'a pas prononcé serait pire que pas d'overlay du tout.
///
/// ⚠️ **`exempte` n'est pas couvert par les tests.** Le red-check l'a montré :
/// supprimer l'exemption laisse `loverlay_dit_la_meme_chose_que_le_modele` au
/// vert. Sur les modules essayés, l'hôte reste hors du seuil même au joint —
/// `FACTEUR_COLLISION = 0.85` tolère l'adjacence de docking, et l'exemption ne
/// change alors rien. Elle reste, parce que `Chantier::collision` l'applique et
/// que les deux doivent dire la même chose sur *toutes* les formes, pas
/// seulement celles qu'on a essayées ; mais c'est une garde **non exercée**, et
/// mieux vaut l'écrire que de croire le contraire.
pub fn gene_par(
    st: &Station,
    propose: &Enveloppe,
    marge: f32,
    exempte: Option<usize>,
) -> Vec<(usize, f32)> {
    st.pieces()
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != exempte)
        .filter_map(|(i, piece)| {
            let sienne = piece.composant.enveloppe_locale().transformee(piece.transforme);
            let manque = propose.distance_axes(&sienne) - marge * (propose.rayon + sienne.rayon);
            (manque < 0.0).then_some((i, manque))
        })
        .collect()
}

/// Point du noyau au paramètre `(u, v)` — chacun dans `[0, 1]`. `v` est ignoré
/// pour un segment (un seul degré de liberté) ; les deux comptent pour un
/// rectangle.
fn point_noyau(e: &Enveloppe, u: f32, v: f32) -> Vec3 {
    match e.noyau {
        Noyau::Segment { a, b } => a + (b - a) * u,
        Noyau::Rectangle { centre, eu, ev, hu, hv } => {
            centre + eu * (hu * (2.0 * u - 1.0)) + ev * (hv * (2.0 * v - 1.0))
        }
    }
}

/// Les deux points les plus proches des **noyaux** de deux enveloppes.
///
/// Recherche par échantillonnage puis raffinement local (compass search) : un
/// noyau se paramètre par `(u, v) ∈ [0,1]²`, et la distance à un point fixe y
/// est convexe (segment comme rectangle), donc cette descente à pas décroissant
/// converge — c'est un tracé de débogage, pas un calcul de physique, et
/// l'écart résiduel après trente pas est très en dessous de l'épaisseur d'un
/// trait à l'écran. Écrire ici une seconde résolution analytique dupliquerait
/// `distance_segment_rectangle` — et le Lot 1 a passé son temps à supprimer ce
/// genre de doublon.
#[allow(dead_code)]
fn plus_proches(a: &Enveloppe, b: &Enveloppe) -> (Vec3, Vec3) {
    let (mut pa, mut pb) = ((0.5_f32, 0.5_f32), (0.5_f32, 0.5_f32));
    let mut pas = 0.5_f32;
    // Meilleur `(u, v)` sur `e` pour viser `cible`, en partant de `depart`.
    let affiner = |e: &Enveloppe, depart: (f32, f32), cible: Vec3, pas: f32| -> (f32, f32) {
        let mut meilleur = (depart, point_noyau(e, depart.0, depart.1).distance(cible));
        for du in [-pas, 0.0, pas] {
            for dv in [-pas, 0.0, pas] {
                if du == 0.0 && dv == 0.0 {
                    continue;
                }
                let cand = ((depart.0 + du).clamp(0.0, 1.0), (depart.1 + dv).clamp(0.0, 1.0));
                let dist = point_noyau(e, cand.0, cand.1).distance(cible);
                if dist < meilleur.1 {
                    meilleur = (cand, dist);
                }
            }
        }
        meilleur.0
    };
    for _ in 0..30 {
        // Descente alternée : `b` fixé on cherche le meilleur point sur `a`,
        // puis l'inverse. C'est convexe côté par côté — ça converge.
        pa = affiner(a, pa, point_noyau(b, pb.0, pb.1), pas);
        pb = affiner(b, pb, point_noyau(a, pa.0, pa.1), pas);
        pas *= 0.6;
    }
    (point_noyau(a, pa.0, pa.1), point_noyau(b, pb.0, pb.1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::{Chantier, Composant, Profil, VarianteModule, FACTEUR_COLLISION};

    /// **L'overlay n'explique que des refus que le modèle a prononcés** (§8.5).
    ///
    /// La garde vaut dans les deux sens, et c'est le second qui compte : un
    /// overlay qui dessinerait un conflit là où `peut_poser` accepte ferait
    /// douter d'une pose parfaitement valide — « pire que pas d'overlay du
    /// tout », dit la conception.
    ///
    /// Vérifié sur le **vrai** critère du modèle (`FACTEUR_COLLISION`), et sur
    /// une pose fournie par `pose_prevue`, celle-là même que le fantôme montre.
    #[test]
    fn loverlay_dit_la_meme_chose_que_le_modele() {
        let court =
            Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 1.0 };
        let mut ch = Chantier::new();
        ch.racine(court.clone());
        let id = ch.libres()[0].id;
        assert!(ch.poser(id, court.clone(), 0));

        let station = match ch.apercu() {
            crate::vaisseau::EtatStation::Prete(s) => s,
            _ => panic!("chantier vide"),
        };

        let mut accord_refus = 0;
        let mut accord_pose = 0;
        for m in 0..court.ports().len() {
            for p in ch.libres() {
                let Some(piece) = ch.pose_prevue(p.id, &court, m) else { continue };
                let propose = court.enveloppe_locale().transformee(piece.transforme);
                // Hôte exempté, comme dans l'anti-collision : `apercu` conserve
                // l'ordre de `Chantier::pieces`, d'où l'indice.
                let exempte = ch.pieces().position(|(pid, _)| pid == p.origine);
                let genes = gene_par(&station, &propose, FACTEUR_COLLISION, exempte);

                // `peut_poser` refuse aussi pour incompatibilité de genre ou de
                // profil ; l'overlay ne parle que d'encombrement. On ne compare
                // donc que sur les ports que le montage accepte.
                if !ch.compatibles(&court, m).contains(&p.id) {
                    continue;
                }
                if ch.peut_poser(p.id, &court, m) {
                    assert!(genes.is_empty(), "overlay : conflit annoncé sur une pose que le modèle accepte");
                    accord_pose += 1;
                } else {
                    assert!(!genes.is_empty(), "overlay : aucun gêneur sur une pose que le modèle refuse");
                    // Le manque est bien un manque : négatif, donc à combler.
                    assert!(genes.iter().all(|(_, m)| *m < 0.0), "un « manque » positif");
                    accord_refus += 1;
                }
            }
        }
        assert!(accord_pose > 0, "aucune pose acceptée : la moitié de la garde ne s'exerce pas");
        assert!(accord_refus > 0, "aucune pose refusée : l'autre moitié non plus");
    }

    /// Configurations couvrant les cas où une résolution approchée peut fauter :
    /// croisées, sécantes, parallèles superposées, parallèles disjointes (le
    /// minimum est alors sur un bout), très inégales en longueur, et une sphère
    /// contre une capsule.
    fn cas() -> Vec<(Enveloppe, Enveloppe)> {
        vec![
            (
                Enveloppe::capsule(vec3(-1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), 0.3),
                Enveloppe::capsule(vec3(0.0, -1.0, 2.0), vec3(0.0, 1.0, 2.0), 0.3),
            ),
            (
                Enveloppe::capsule(vec3(-1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), 0.3),
                Enveloppe::capsule(vec3(0.0, -1.0, 0.0), vec3(0.0, 1.0, 0.0), 0.3),
            ),
            (
                Enveloppe::capsule(vec3(-5.0, 0.0, 0.0), vec3(5.0, 0.0, 0.0), 1.0),
                Enveloppe::capsule(vec3(-5.0, 3.0, 0.0), vec3(5.0, 3.0, 0.0), 1.0),
            ),
            (
                Enveloppe::capsule(vec3(0.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0), 0.5),
                Enveloppe::capsule(vec3(5.0, 3.0, 0.0), vec3(6.0, 3.0, 0.0), 0.5),
            ),
            (
                Enveloppe::capsule(vec3(0.0, 0.0, -20.0), vec3(0.0, 0.0, 20.0), 2.0),
                Enveloppe::capsule(vec3(4.0, 0.0, 1.0), vec3(4.2, 0.0, 1.1), 1.0),
            ),
            (
                Enveloppe::sphere(vec3(3.0, 1.0, -2.0), 1.5),
                Enveloppe::capsule(vec3(-4.0, 1.0, 0.0), vec3(4.0, 1.0, 0.0), 0.8),
            ),
            // Un boudin contre une capsule qui plonge à travers son plan.
            (
                Enveloppe::plaque(Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0, 0.5),
                Enveloppe::capsule(vec3(1.0, 1.0, -8.0), vec3(1.0, 1.0, 8.0), 0.3),
            ),
            // Deux boudins empilés, comme les boucliers de l'ISV sur leur mât.
            (
                Enveloppe::plaque(Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 5.0, 0.5),
                Enveloppe::plaque(vec3(0.0, 0.0, 6.0), Vec3::X, Vec3::Y, 5.0, 5.0, 0.5),
            ),
        ]
    }

    // **L'invariant qui rend l'overlay honnête.** Le trait rouge qu'il dessine
    // doit matérialiser *la* distance sur laquelle `Chantier` a fondé son refus.
    // Si les deux divergent, l'overlay explique un conflit que le modèle n'a pas
    // prononcé — et un outil de débogage qui ment est pire que pas d'outil.
    //
    // ⚠️ Le test compare donc au **modèle** (`Enveloppe::distance_axes`) et non à
    // une seconde formule écrite ici : c'est exactement le doublon qu'on veut
    // éviter, et `distance_axes` est déjà contrôlé en force brute de son côté.
    #[test]
    fn le_trait_de_conflit_mesure_la_distance_du_modele() {
        for (a, b) in cas() {
            let (p, q) = plus_proches(&a, &b);
            let attendu = a.distance_axes(&b);
            assert!(
                (p.distance(q) - attendu).abs() < 2e-2,
                "trait {:.4} contre modèle {attendu:.4} pour {a:?} / {b:?}",
                p.distance(q)
            );
        }
    }

    /// Distance d'un point au noyau d'une enveloppe, quelle que soit sa forme.
    fn sur_le_noyau(p: Vec3, e: &Enveloppe) -> f32 {
        match e.noyau {
            Noyau::Segment { a, b } => crate::vaisseau::distance_point_segment(p, a, b),
            Noyau::Rectangle { centre, eu, ev, hu, hv } => {
                crate::vaisseau::distance_point_rectangle(p, centre, eu, ev, hu, hv)
            }
        }
    }

    // Les deux points doivent être **sur** les noyaux qu'ils prétendent joindre,
    // sinon le trait part de nulle part. Une descente mal bornée les ferait
    // sortir du segment ou du rectangle.
    #[test]
    fn les_deux_points_restent_sur_leurs_noyaux() {
        for (a, b) in cas() {
            let (p, q) = plus_proches(&a, &b);
            assert!(sur_le_noyau(p, &a) < 1e-3, "p hors de son noyau");
            assert!(sur_le_noyau(q, &b) < 1e-3, "q hors de son noyau");
        }
    }

    // Le repère des anneaux doit être orthonormé **quel que soit l'axe**, y
    // compris quand il est déjà colinéaire à un axe canonique — c'est là que le
    // produit vectoriel se dégénère, et un anneau plat ne se voit pas à l'écran.
    #[test]
    fn le_repere_des_anneaux_tient_sur_tous_les_axes() {
        for axe in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::NEG_X, vec3(1.0, 1.0, 1.0), Vec3::ZERO] {
            let (u, v) = base(axe);
            assert!((u.length() - 1.0).abs() < 1e-4, "u non unitaire pour {axe:?}");
            assert!((v.length() - 1.0).abs() < 1e-4, "v non unitaire pour {axe:?}");
            assert!(u.dot(v).abs() < 1e-4, "u et v non perpendiculaires pour {axe:?}");
            if axe != Vec3::ZERO {
                let w = axe.normalize();
                assert!(u.dot(w).abs() < 1e-4 && v.dot(w).abs() < 1e-4, "anneau non perpendiculaire à {axe:?}");
            }
        }
    }
}
