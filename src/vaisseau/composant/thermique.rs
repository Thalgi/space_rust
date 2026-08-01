//! **Bouclier thermique de l'épine** (ISV) : le bardage qui protège la poutre du
//! rayonnement des tuyères.
//!
//! À ne pas confondre avec les [boucliers de tête](super::bouclier), qui parent
//! la poussière interstellaire. Celui-ci pare une source qui est **à bord** : sur
//! un vaisseau tracteur, l'épine est tendue devant des moteurs dont l'échappement
//! est plus chaud que la surface du Soleil, et elle en prend le rayonnement sur
//! toute sa longueur — d'autant plus fort qu'on est près de la base.
//!
//! **Pourquoi des écailles et pas une tôle.** Une paroi continue de plusieurs
//! dizaines de mètres soumise à ce gradient se déforme, et une paroi encastrée
//! qui se déforme casse ou arrache ses fixations. Le bardage en écailles
//! **imbriquées** résout exactement ça : chaque plaque est petite, tenue par un
//! seul bord, et **libre de se dilater** sous celle qui la recouvre. C'est la
//! logique des tuiles de la navette et des bardages de tuyère, et c'est aussi ce
//! qui donne la forme : de petites plaques qui se chevauchent, pas une gaine.
//!
//! **Le sens du recouvrement n'est pas libre.** La chaleur vient de la base
//! (−Z) : chaque écaille recouvre donc la **suivante** vers +Z, si bien que le
//! flux glisse d'une plaque à l'autre sans jamais rencontrer une tranche de face.
//! Monté à l'envers, chaque joint offrirait une arête au rayonnement — le même
//! raisonnement qu'un toit posé dans le sens de la pluie.
//!
//! **Le bardage est évasé, parce que l'épine l'est.** Il se monte là où la
//! chaleur est — au droit des moteurs — et c'est précisément la portion où la
//! poutre s'ouvre vers son pied. Un manchon droit y flotterait de plusieurs
//! unités à un bout et serait traversé à l'autre. La section suit donc la même
//! loi en puissance que le treillis : `bout + (pied − bout)·(1 − t)^courbure`.
//!
//! Repère local : axe **+Z**, base côté moteurs à l'origine (le **gros** bout),
//! section hexagonale comme l'épine qu'il habille.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::Enveloppe;
use crate::vaisseau::pieces;
use macroquad::prelude::*;


/// Teinte des écailles : un gris **chaud et sourd**, franchement plus sombre que
/// l'alu de la structure. Un bouclier thermique est un matériau réfractaire qui a
/// déjà cuit ; peint comme de la coque, il lirait comme une gaine technique.
const ECAILLE: Color = Color { r: 0.33, g: 0.29, b: 0.26, a: 1.0 };
/// La lèvre relevée de chaque écaille, plus sombre encore : c'est elle qui dessine
/// l'imbrication, et sans contraste le bardage redevient un tube lisse.
const LEVRE: Color = Color { r: 0.20, g: 0.17, b: 0.16, a: 1.0 };

/// Saillie de la lèvre, en fraction du rayon **local**. Prise sur le rayon
/// courant et non sur celui du pied : une écaille du bout, quatre fois plus
/// petite, n'a pas de raison d'avoir la même saillie qu'une écaille de la base.
///
/// **Volontairement faible** : le
/// bardage doit épaissir l'épine d'un cheveu, pas la doubler. Au-delà de ~0,2 les
/// écailles cessent d'être un revêtement et deviennent des ailettes.
const SAILLIE: f32 = 0.13;
/// Part du pas dont chaque écaille recouvre la suivante. En dessous d'un quart le
/// recouvrement ne se voit plus et le bardage lit comme un empilement d'anneaux.
const RECOUVREMENT: f32 = 0.35;
/// Part de l'écaille occupée par la lèvre sombre, mesurée depuis son bord libre.
const PART_LEVRE: f32 = 0.22;

/// Alternance de valeur d'un rang à l'autre : deux tons proches, qui séparent les
/// rangs quand on regarde le bardage de loin et de face.
///
/// Contrairement au facettage refusé sur les grandes plaques de tête, ce n'est
/// pas un décor plaqué sur une surface unie — les rangs **sont** des objets
/// distincts, et la variation ne fait que dire ce qui est déjà là.
const RANGS_TONS: [f32; 2] = [1.0, 0.86];

/// Plafond de sommets par lot. Un bardage long en produit des milliers ; il faut
/// donc vider le tampon en cours de route plutôt que de bâtir un lot que le
/// batcher refuserait.
const LOT_MAX: usize = 1200;

fn assombrir(c: Color, k: f32) -> Color {
    Color { r: c.r * k, g: c.g * k, b: c.b * k, a: 1.0 }
}

/// Accumulateur de quadrilatères, vidé dès qu'il approche [`LOT_MAX`].
struct Lot {
    sommets: Vec<Vec3>,
    indices: Vec<u16>,
    couleur: Color,
}

impl Lot {
    fn new(couleur: Color) -> Self {
        Self { sommets: Vec::new(), indices: Vec::new(), couleur }
    }

    /// Ajoute un quadrilatère **visible des deux côtés** : l'épine est un
    /// treillis ajouré, on voit donc le bardage par l'intérieur dès qu'on passe
    /// derrière la poutre.
    fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        let i = self.sommets.len() as u16;
        self.sommets.extend_from_slice(&[a, b, c, d]);
        self.indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
        self.indices.extend_from_slice(&[i, i + 2, i + 1, i, i + 3, i + 2]);
    }

    fn vider<P: Peintre>(&mut self, p: &mut P) {
        if self.indices.is_empty() {
            return;
        }
        p.triangles(&self.sommets, &self.indices, self.couleur);
        self.sommets.clear();
        self.indices.clear();
    }

    fn vider_si_plein<P: Peintre>(&mut self, p: &mut P) {
        if self.sommets.len() >= LOT_MAX {
            self.vider(p);
        }
    }
}

/// Pas entre rangs.
///
/// Divisé par `rangs + RECOUVREMENT` et non par `rangs` : le dernier rang déborde
/// du sien de tout le recouvrement, et sans ce terme le bardage dépasserait la
/// longueur annoncée — donc l'englobant, et la place qu'on lui a réservée sur
/// l'épine.
fn pas(longueur: f32, rangs: usize) -> f32 {
    longueur / (rangs.max(1) as f32 + RECOUVREMENT)
}

/// Rayon de la **section suivie**, à la fraction `t` de la longueur.
///
/// Même loi en puissance que le treillis conique : le bardage doit épouser la
/// poutre, pas la croiser. `courbure` élevée concentre l'évasement près du pied,
/// exactement comme sur l'épine.
pub fn section(rayon_pied: f32, rayon_bout: f32, courbure: f32, t: f32) -> f32 {
    rayon_bout + (rayon_pied - rayon_bout) * (1.0 - t.clamp(0.0, 1.0)).powf(courbure.max(0.1))
}

pub(super) fn dessiner<P: Peintre>(
    p: &mut P,
    rayon_pied: f32,
    rayon_bout: f32,
    courbure: f32,
    longueur: f32,
    rangs: usize,
) {
    let n = rangs.max(1);
    let pas = pas(longueur, n);
    let rayon = |z: f32| section(rayon_pied, rayon_bout, courbure, z / longueur.max(1e-4));

    let mut face = [Lot::new(ECAILLE), Lot::new(assombrir(ECAILLE, RANGS_TONS[1]))];
    let mut levre = [Lot::new(LEVRE), Lot::new(assombrir(LEVRE, RANGS_TONS[1]))];

    for j in 0..n {
        let z0 = j as f32 * pas;
        let z1 = z0 + pas * (1.0 + RECOUVREMENT);
        // Bord **plaqué** contre l'épine à z0, bord **libre et relevé** à z1 :
        // c'est ce décalage radial qui fait passer l'écaille suivante dessous.
        // Le bord libre est relevé par rapport à la section **de sa propre
        // cote**, sinon l'évasement suffirait à le faire paraître relevé là où
        // il ne l'est pas.
        let bas = pieces::hexa_section(Vec3::Z * z0, Vec3::X, Vec3::Y, rayon(z0));
        let haut =
            pieces::hexa_section(Vec3::Z * z1, Vec3::X, Vec3::Y, rayon(z1) * (1.0 + SAILLIE));
        let t = j % 2;

        for i in 0..6 {
            let (a, b) = (bas[i], bas[(i + 1) % 6]);
            let (d, c) = (haut[i], haut[(i + 1) % 6]);
            // La lèvre est prise **sur** l'écaille, pas ajoutée par-dessus : deux
            // surfaces au même endroit se disputeraient le pixel.
            let (pa, pb) = (a.lerp(d, 1.0 - PART_LEVRE), b.lerp(c, 1.0 - PART_LEVRE));
            face[t].quad(a, b, pb, pa);
            levre[t].quad(pa, pb, c, d);
        }
        face[t].vider_si_plein(p);
        levre[t].vider_si_plein(p);
    }

    for l in face.iter_mut().chain(levre.iter_mut()) {
        l.vider(p);
    }
}

pub(super) fn cout() -> f32 {
    6.0
}

/// Le bardage se déploie **d'un seul côté** de son origine, le long de +Z. Le
/// rayon qui majore est celui du **pied**, le gros bout.
pub(super) fn rayon_local(rayon_pied: f32, rayon_bout: f32, longueur: f32) -> f32 {
    longueur.hypot(rayon_pied.max(rayon_bout) * (1.0 + SAILLIE))
}

/// **Capsule** le long de l'épine : le bardage est un manchon, pas une boule.
/// Il fait ici 13 de long pour 3,5 de rayon — la sphère qui le contenait en
/// réservait 7,4 tout autour, soit plus du double de sa section réelle.
pub(super) fn englobant(rayon_pied: f32, rayon_bout: f32, longueur: f32) -> Enveloppe {
    let demi = longueur * 0.5;
    Enveloppe::axe(Vec3::Z * demi, Vec3::Z, demi, rayon_pied.max(rayon_bout) * (1.0 + SAILLIE))
}
