//! **Enveloppe de collision** : un noyau convexe, gonflé d'un rayon.
//!
//! Pourquoi pas une sphère, qui était le choix d'origine : une sphère est un
//! bon volume englobant pour une pièce **compacte**, et un très mauvais pour
//! une pièce **allongée**. Un radiateur méga fait 30 de long sur 8 de large ;
//! la plus petite sphère qui le contient a un rayon de 15, et réserve donc un
//! volume vide de 15 dans **toutes** les directions, y compris les trois où la
//! pièce n'a que 4 d'épaisseur.
//!
//! Tant que seul le générateur posait des pièces, c'était sans conséquence : une
//! pose refusée est un non-événement, la grammaire réessaie ailleurs. Face à un
//! humain qui vient de cliquer, c'en est une — il voit un emplacement
//! manifestement libre être refusé, et rien ne le lui explique
//! (`docs/conception/assembleur.md` §5.3).
//!
//! **Le principe retenu, et qui couvre tout** (`conception/assembleur.md` §9.3) :
//! une enveloppe est un **noyau convexe gonflé d'un rayon** — une somme de
//! Minkowski. La distance entre deux enveloppes vaut alors la distance entre
//! leurs noyaux, moins la somme des rayons ([`Enveloppe::ecart`]), quelle que
//! soit la forme des noyaux.
//!
//! | Noyau | Enveloppe obtenue | Pour |
//! |---|---|---|
//! | un **point** | sphère | pièces ramassées |
//! | un **segment** | capsule | pièces allongées |
//! | un **rectangle** | boudin | plaques (boucliers, panneaux) |
//!
//! Point et segment partagent une seule représentation ([`Noyau::Segment`], `a
//! == b` donnant le point) ; le rectangle est le troisième noyau
//! ([`Noyau::Rectangle`]), pour les pièces **plates** — une plaque n'est ni
//! ramassée ni allongée, elle a deux dimensions fines et une seule large.
//!
//! ⚠️ **Un noyau survit exactement aux transformées de ce projet.** Les poses
//! sont des rotations+translations (`Repere::to_mat4`) et les symétries des
//! réflexions (déterminant −1) : toutes préservent les distances et les angles,
//! donc il suffit de transformer les points comme des points, les axes du
//! rectangle comme des vecteurs, et de **garder les rayons/étendues**. Une boîte
//! alignée sur les axes (AABB) ne survivrait pas à une rotation, et une boîte
//! orientée (OBB) demanderait le théorème des axes séparateurs.

use macroquad::prelude::*;

/// Le noyau d'une enveloppe : ce que le rayon vient gonfler.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Noyau {
    /// Tous les points du segment `[a, b]`. `a == b` dégénère en un point.
    Segment { a: Vec3, b: Vec3 },
    /// Le rectangle centré en `centre`, dans le plan orthonormé `(eu, ev)`, de
    /// demi-étendues `hu` (selon `eu`) et `hv` (selon `ev`).
    Rectangle { centre: Vec3, eu: Vec3, ev: Vec3, hu: f32, hv: f32 },
}

/// Capsule, sphère ou boudin : un [`Noyau`] gonflé de `rayon`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Enveloppe {
    pub noyau: Noyau,
    pub rayon: f32,
}

impl Enveloppe {
    /// Sphère centrée en `centre`.
    pub fn sphere(centre: Vec3, rayon: f32) -> Self {
        Self { noyau: Noyau::Segment { a: centre, b: centre }, rayon }
    }

    /// Capsule d'axe `[a, b]`.
    pub fn capsule(a: Vec3, b: Vec3, rayon: f32) -> Self {
        Self { noyau: Noyau::Segment { a, b }, rayon }
    }

    /// Capsule d'axe `direction`, centrée sur `centre`.
    ///
    /// **Comment cadrer une pièce en boîte ou en cylindre** de demi-longueur `L`
    /// et de demi-épaisseur `w` : prendre `demi_axe = L` et `rayon = w`. Aucune
    /// capsule ne contient exactement une boîte — il faut choisir *où* elle
    /// déborde, et ce réglage-ci la fait déborder d'une calotte de `w`
    /// **au bout**, là où ça ne gêne personne, plutôt qu'en travers.
    ///
    /// L'autre réglage possible (`demi_axe = L − w`, `rayon = w√2`) déborde
    /// moins en longueur mais **41 % plus large**. C'est exactement ce qu'on
    /// cherche à éviter : les poses voisines se font de côté, pas dans l'axe.
    pub fn axe(centre: Vec3, direction: Vec3, demi_axe: f32, rayon: f32) -> Self {
        let d = direction.normalize_or_zero() * demi_axe.max(0.0);
        Self { noyau: Noyau::Segment { a: centre - d, b: centre + d }, rayon }
    }

    /// Boudin : noyau **rectangle**, pour une pièce plate (`conception/assembleur.md`
    /// §9.2). `eu` donne le plan ; `ev` est réprojeté hors de `eu` puis les deux
    /// sont normalisés — comme [`Self::axe`] le fait déjà pour sa direction, pas
    /// besoin de fournir une base déjà orthonormée. `hu`, `hv` sont les
    /// demi-étendues dans ce plan, `rayon` la demi-épaisseur gonflée d'un jeu.
    pub fn plaque(centre: Vec3, eu: Vec3, ev: Vec3, hu: f32, hv: f32, rayon: f32) -> Self {
        let eu = eu.normalize_or_zero();
        let ev = (ev - eu * eu.dot(ev)).normalize_or_zero();
        Self { noyau: Noyau::Rectangle { centre, eu, ev, hu: hu.max(0.0), hv: hv.max(0.0) }, rayon }
    }

    /// La capsule est-elle une sphère ? Faux pour un boudin, qui n'est jamais
    /// dégénéré au point d'être un point.
    pub fn est_sphere(&self) -> bool {
        matches!(self.noyau, Noyau::Segment { a, b } if a.distance_squared(b) < 1e-12)
    }

    /// Centre du noyau. Sert au repli sphérique et aux messages.
    pub fn centre(&self) -> Vec3 {
        match self.noyau {
            Noyau::Segment { a, b } => (a + b) * 0.5,
            Noyau::Rectangle { centre, .. } => centre,
        }
    }

    /// Rayon de la **plus petite sphère** qui contient le noyau, depuis son
    /// propre centre — plus `rayon`. C'est la mesure qu'on perdait en passant à
    /// la capsule (puis au boudin), et dont le cadrage caméra a encore besoin.
    pub fn rayon_sphere(&self) -> f32 {
        let r = match self.noyau {
            Noyau::Segment { a, b } => a.distance(b) * 0.5,
            Noyau::Rectangle { hu, hv, .. } => hu.hypot(hv),
        };
        r + self.rayon
    }

    /// La même enveloppe, déplacée par une transformée **rigide** (rotation,
    /// translation, réflexion). Les points du noyau se transforment comme des
    /// points, `eu`/`ev` comme des vecteurs (`transform_vector3`, qui ignore la
    /// translation) ; le rayon et les demi-étendues sont conservés, puisqu'une
    /// transformée rigide préserve les longueurs et les angles.
    pub fn transformee(&self, m: Mat4) -> Self {
        let noyau = match self.noyau {
            Noyau::Segment { a, b } => {
                Noyau::Segment { a: m.transform_point3(a), b: m.transform_point3(b) }
            }
            Noyau::Rectangle { centre, eu, ev, hu, hv } => Noyau::Rectangle {
                centre: m.transform_point3(centre),
                eu: m.transform_vector3(eu),
                ev: m.transform_vector3(ev),
                hu,
                hv,
            },
        };
        Self { noyau, rayon: self.rayon }
    }

    /// Distance entre les **noyaux** de deux enveloppes (rayons non compris).
    /// La formule ne change pas selon les formes en présence : c'est la
    /// propriété qui fait tenir la généralisation en un seul noyau convexe
    /// gonflé (`conception/assembleur.md` §9.3).
    pub fn distance_axes(&self, autre: &Enveloppe) -> f32 {
        match (self.noyau, autre.noyau) {
            (Noyau::Segment { a, b }, Noyau::Segment { a: a2, b: b2 }) => {
                distance_segments(a, b, a2, b2)
            }
            (Noyau::Segment { a, b }, Noyau::Rectangle { centre, eu, ev, hu, hv }) => {
                distance_segment_rectangle(a, b, centre, eu, ev, hu, hv)
            }
            (Noyau::Rectangle { centre, eu, ev, hu, hv }, Noyau::Segment { a, b }) => {
                distance_segment_rectangle(a, b, centre, eu, ev, hu, hv)
            }
            (
                Noyau::Rectangle { centre: c1, eu: eu1, ev: ev1, hu: hu1, hv: hv1 },
                Noyau::Rectangle { centre: c2, eu: eu2, ev: ev2, hu: hu2, hv: hv2 },
            ) => distance_rectangles(c1, eu1, ev1, hu1, hv1, c2, eu2, ev2, hu2, hv2),
        }
    }

    /// Écart entre les deux **surfaces** : négatif si elles s'interpénètrent.
    pub fn ecart(&self, autre: &Enveloppe) -> f32 {
        self.distance_axes(autre) - (self.rayon + autre.rayon)
    }

    /// Distance du point à la surface : ≤ 0 si le point est dedans.
    pub fn profondeur(&self, p: Vec3) -> f32 {
        let d = match self.noyau {
            Noyau::Segment { a, b } => distance_point_segment(p, a, b),
            Noyau::Rectangle { centre, eu, ev, hu, hv } => {
                distance_point_rectangle(p, centre, eu, ev, hu, hv)
            }
        };
        d - self.rayon
    }

    pub fn contient(&self, p: Vec3) -> bool {
        self.profondeur(p) <= 0.0
    }

    /// La demi-droite `origine + t·direction` (`t ≥ 0`) traverse-t-elle cette
    /// enveloppe ? Rend alors la **distance de l'origine à la surface** — 0 si
    /// l'origine est dedans.
    ///
    /// Sert à désigner la pièce sous le curseur (`conception/assembleur.md`
    /// §8.3, état « pièce sélectionnée ») : c'est cette distance qui départage
    /// deux pièces percées par le même rayon, la plus proche de l'œil étant
    /// celle qu'on voit. On rend la distance à la **surface**, et non le
    /// paramètre du point de plus courte approche : ce qui décide de « qui est
    /// devant » est la distance à la pièce elle-même, pas l'endroit où le rayon
    /// la frôle au plus près.
    ///
    /// Une enveloppe étant un noyau convexe gonflé de `rayon`, la demi-droite
    /// la touche exactement quand elle passe à moins de `rayon` du noyau. Le
    /// calcul réutilise donc les distances **segment↔noyau** déjà écrites, en
    /// bornant la demi-droite à `portee` : ce n'est pas une approximation mais
    /// une borne **exacte**, aucun point du noyau n'étant plus loin que ça de
    /// l'origine — le point le plus proche est donc forcément atteint avant.
    pub fn touche_rayon(&self, origine: Vec3, direction: Vec3) -> Option<f32> {
        let dir = direction.normalize_or_zero();
        if dir == Vec3::ZERO {
            return None;
        }
        let portee = (self.centre() - origine).length() + self.rayon_sphere();
        let bout = origine + dir * portee;
        let au_rayon = match self.noyau {
            Noyau::Segment { a, b } => distance_segments(origine, bout, a, b),
            Noyau::Rectangle { centre, eu, ev, hu, hv } => {
                distance_segment_rectangle(origine, bout, centre, eu, ev, hu, hv)
            }
        };
        (au_rayon <= self.rayon).then(|| self.profondeur(origine).max(0.0))
    }
}

/// Distance d'un point au segment `[a, b]`.
pub fn distance_point_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let l2 = ab.length_squared();
    if l2 < 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Distance entre les segments `[p1, q1]` et `[p2, q2]`.
///
/// Algorithme classique des **points les plus proches de deux segments**
/// (Ericson, *Real-Time Collision Detection* §5.1.9) : on résout le système au
/// point le plus proche des deux **droites**, puis on ramène les paramètres dans
/// `[0, 1]` — en recalculant l'autre paramètre après chaque serrage, sans quoi
/// le couple obtenu n'est plus le plus proche. Les cas dégénérés (un segment
/// réduit à un point, ou les deux) sont traités à part, parce que le
/// dénominateur s'y annule.
pub fn distance_segments(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
    const EPS: f32 = 1e-12;
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.length_squared(), d2.length_squared(), d2.dot(r));

    // Les deux segments sont des points.
    if a <= EPS && e <= EPS {
        return p1.distance(p2);
    }
    // Le premier est un point.
    if a <= EPS {
        return distance_point_segment(p1, p2, q2);
    }
    // Le second est un point.
    if e <= EPS {
        return distance_point_segment(p2, p1, q1);
    }

    let c = d1.dot(r);
    let b = d1.dot(d2);
    let denom = a * e - b * b; // ≥ 0, nul si les segments sont parallèles

    // Parallèles : n'importe quel `s` convient pour la droite, on part de 0 et
    // on laisse le serrage ci-dessous placer les deux paramètres.
    let mut s = if denom > EPS { ((b * f - c * e) / denom).clamp(0.0, 1.0) } else { 0.0 };
    let mut t = (b * s + f) / e;

    // Serrage de `t`, puis **recalcul de `s`** pour rester au plus proche.
    if t < 0.0 {
        t = 0.0;
        s = (-c / a).clamp(0.0, 1.0);
    } else if t > 1.0 {
        t = 1.0;
        s = ((b - c) / a).clamp(0.0, 1.0);
    }
    (p1 + d1 * s).distance(p2 + d2 * t)
}

/// Distance d'un point au rectangle `(centre, eu, ev, hu, hv)` (`eu`, `ev`
/// unitaires et orthogonaux).
///
/// Exact et immédiat : projeté dans le plan, le point le plus proche du
/// rectangle est le **bornage indépendant** de chaque coordonnée `(s, t)` dans
/// `[-hu, hu]` et `[-hv, hv]` — la distance à une boîte est séparable par axe,
/// et rien d'autre à résoudre.
pub fn distance_point_rectangle(p: Vec3, centre: Vec3, eu: Vec3, ev: Vec3, hu: f32, hv: f32) -> f32 {
    let d = p - centre;
    let s = d.dot(eu).clamp(-hu, hu);
    let t = d.dot(ev).clamp(-hv, hv);
    p.distance(centre + eu * s + ev * t)
}

/// Distance entre le segment `[a, b]` et le rectangle `(centre, eu, ev, hu,
/// hv)` (`eu`, `ev` unitaires et orthogonaux).
///
/// Exact, par la même famille d'idée que [`distance_segments`] : projetées
/// dans le plan du rectangle, les coordonnées `(s, t)` du point courant du
/// segment — et son décalage hors-plan `w` — sont toutes trois **affines** en
/// le paramètre `r` du segment. Le carré de la distance,
/// `w(r)² + excès(s(r))² + excès(t(r))²`, est donc une quadratique **par
/// morceaux** : les morceaux se recollent où `s` ou `t` franchit une bordure du
/// rectangle (au plus quatre franchissements), et chaque morceau a un minimum
/// en forme close (sommet de parabole). Il suffit d'évaluer ce minimum sur
/// chaque morceau et de garder le plus petit.
pub fn distance_segment_rectangle(
    a: Vec3,
    b: Vec3,
    centre: Vec3,
    eu: Vec3,
    ev: Vec3,
    hu: f32,
    hv: f32,
) -> f32 {
    let n = eu.cross(ev);
    let d = b - a;
    let rel = a - centre;
    let (s0, ds) = (rel.dot(eu), d.dot(eu));
    let (t0, dt) = (rel.dot(ev), d.dot(ev));
    let (w0, dw) = (rel.dot(n), d.dot(n));

    // Carré exact de la distance en `r`, tous morceaux confondus — sert à
    // évaluer le candidat trouvé sur chaque morceau, sans reformuler la formule.
    let f = |r: f32| -> f32 {
        let (s, t, w) = (s0 + r * ds, t0 + r * dt, w0 + r * dw);
        let es = (s.abs() - hu).max(0.0);
        let et = (t.abs() - hv).max(0.0);
        w * w + es * es + et * et
    };

    // Bornes des morceaux : 0, 1, et les franchissements de ±hu / ±hv dans
    // `]0, 1[`. Six au plus (tableau fixe, pas d'allocation).
    let mut bornes = [0.0_f32, 1.0, 0.0, 0.0, 0.0, 0.0];
    let mut n_bornes = 2;
    for (v0, dv, h) in [(s0, ds, hu), (t0, dt, hv)] {
        if dv.abs() > 1e-9 {
            for cible in [h, -h] {
                let r = (cible - v0) / dv;
                if r > 0.0 && r < 1.0 {
                    bornes[n_bornes] = r;
                    n_bornes += 1;
                }
            }
        }
    }
    let bornes = &mut bornes[..n_bornes];
    bornes.sort_by(|x, y| x.partial_cmp(y).unwrap());

    let mut meilleur = f32::INFINITY;
    for fenetre in bornes.windows(2) {
        let (r_lo, r_hi) = (fenetre[0], fenetre[1]);
        if r_hi - r_lo < 1e-9 {
            meilleur = meilleur.min(f(r_lo));
            continue;
        }
        // Signe de `s` et `t` au milieu du morceau : constant sur tout le
        // morceau par construction des bornes, donc chaque excès y est soit
        // nul, soit une fonction affine connue de `r`.
        let r_mid = (r_lo + r_hi) * 0.5;
        let (s_mid, t_mid) = (s0 + r_mid * ds, t0 + r_mid * dt);
        let (pente_s, decalage_s) = branche(s_mid, s0, ds, hu);
        let (pente_t, decalage_t) = branche(t_mid, t0, dt, hv);

        // f(r) = (w0 + dw·r)² + (decalage_s + pente_s·r)² + (decalage_t + pente_t·r)²
        // — une quadratique A·r² + B·r + C, sommet en r* = −B / 2A.
        let coef_a = dw * dw + pente_s * pente_s + pente_t * pente_t;
        let coef_b = 2.0 * (w0 * dw + decalage_s * pente_s + decalage_t * pente_t);
        let r_etoile =
            if coef_a > 1e-12 { (-coef_b / (2.0 * coef_a)).clamp(r_lo, r_hi) } else { r_lo };
        meilleur = meilleur.min(f(r_etoile));
    }
    meilleur.max(0.0).sqrt()
}

/// Pour [`distance_segment_rectangle`] : la branche de l'excès sur un axe —
/// `(0, 0)` si `v_mid` est dans `[-h, h]` (excès nul partout sur le morceau),
/// sinon la pente et l'ordonnée à l'origine de `v(r) ∓ h`.
fn branche(v_mid: f32, v0: f32, dv: f32, h: f32) -> (f32, f32) {
    if v_mid > h {
        (dv, v0 - h)
    } else if v_mid < -h {
        (dv, v0 + h)
    } else {
        (0.0, 0.0)
    }
}

/// Les quatre bords du rectangle, comme segments, en tournant autour du
/// contour.
fn bords(centre: Vec3, eu: Vec3, ev: Vec3, hu: f32, hv: f32) -> [(Vec3, Vec3); 4] {
    let (pu, mu, pv, mv) = (eu * hu, -eu * hu, ev * hv, -ev * hv);
    [
        (centre + pu + pv, centre + pu + mv),
        (centre + pu + mv, centre + mu + mv),
        (centre + mu + mv, centre + mu + pv),
        (centre + mu + pv, centre + pu + pv),
    ]
}

/// Distance entre deux rectangles, d'orientations quelconques (pas
/// nécessairement parallèles, ni coplanaires).
///
/// **L'observation qui rend ça exact sans énumérer de cas** (coin↔coin,
/// bord↔bord, coin↔intérieur, plans parallèles imbriqués…) : pour deux
/// convexes disjoints, le théorème de l'hyperplan séparateur dit que le point
/// le plus proche sur *au moins un* des deux rectangles est sur son **bord**
/// (l'autre peut être n'importe où sur l'autre face, bord compris — ça n'arrive
/// que si les deux plans sont parallèles). Donc balayer les quatre bords de
/// chaque rectangle contre la face **entière** de l'autre — via
/// [`distance_segment_rectangle`], qui résout déjà exactement « point de ce
/// segment contre tout le rectangle » — couvre tous les cas : les huit bords (4
/// + 4) contiennent forcément celui qui porte le point le plus proche, quel que
/// soit celui des deux rectangles auquel il appartient. Aucun besoin d'un test
/// d'intersection séparé : si les rectangles se transpercent, le bord qui
/// traverse le plan de l'autre passe par le point de contact et y rend une
/// distance nulle.
#[allow(clippy::too_many_arguments)]
pub fn distance_rectangles(
    c1: Vec3,
    eu1: Vec3,
    ev1: Vec3,
    hu1: f32,
    hv1: f32,
    c2: Vec3,
    eu2: Vec3,
    ev2: Vec3,
    hu2: f32,
    hv2: f32,
) -> f32 {
    let mut meilleur = f32::INFINITY;
    for (a, b) in bords(c1, eu1, ev1, hu1, hv1) {
        meilleur = meilleur.min(distance_segment_rectangle(a, b, c2, eu2, ev2, hu2, hv2));
    }
    for (a, b) in bords(c2, eu2, ev2, hu2, hv2) {
        meilleur = meilleur.min(distance_segment_rectangle(a, b, c1, eu1, ev1, hu1, hv1));
    }
    meilleur
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- distance point ↔ segment ---

    #[test]
    fn le_point_se_projette_sur_le_segment_ou_sur_ses_bouts() {
        let (a, b) = (vec3(-1.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0));
        // Au-dessus du milieu : distance perpendiculaire.
        assert!((distance_point_segment(vec3(0.0, 3.0, 0.0), a, b) - 3.0).abs() < 1e-5);
        // Au-delà d'un bout : distance **au bout**, pas à la droite porteuse.
        assert!((distance_point_segment(vec3(5.0, 0.0, 0.0), a, b) - 4.0).abs() < 1e-5);
        // Sur le segment : nulle.
        assert!(distance_point_segment(vec3(0.5, 0.0, 0.0), a, b) < 1e-5);
        // Segment dégénéré (un point) : distance au point — mesurée depuis `a`,
        // pas depuis l'origine.
        assert!((distance_point_segment(a + Vec3::Y * 2.0, a, a) - 2.0).abs() < 1e-5);
    }

    // --- distance segment ↔ segment ---

    #[test]
    fn deux_segments_croises_a_distance_connue() {
        // Perpendiculaires, décalés de 2 en Z : la distance est exactement 2.
        let d = distance_segments(
            vec3(-1.0, 0.0, 0.0),
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, -1.0, 2.0),
            vec3(0.0, 1.0, 2.0),
        );
        assert!((d - 2.0).abs() < 1e-5, "{d}");
    }

    #[test]
    fn deux_segments_qui_se_coupent_sont_a_distance_nulle() {
        let d = distance_segments(
            vec3(-1.0, 0.0, 0.0),
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, -1.0, 0.0),
            vec3(0.0, 1.0, 0.0),
        );
        assert!(d < 1e-5, "{d}");
    }

    // Cas parallèle : le dénominateur s'annule, c'est la branche la plus facile
    // à écrire faux — et elle est très fréquente ici (deux poutres alignées,
    // deux radiateurs côte à côte).
    #[test]
    fn deux_segments_paralleles_gardent_leur_ecart() {
        // Superposés en X, écartés de 3 en Y : distance 3 partout.
        let d = distance_segments(
            vec3(-5.0, 0.0, 0.0),
            vec3(5.0, 0.0, 0.0),
            vec3(-5.0, 3.0, 0.0),
            vec3(5.0, 3.0, 0.0),
        );
        assert!((d - 3.0).abs() < 1e-5, "{d}");
    }

    // Parallèles **et disjoints le long de leur axe** : la distance passe par
    // les bouts, pas par la perpendiculaire. C'est le cas que le serrage de `t`
    // doit rattraper.
    #[test]
    fn deux_segments_paralleles_decales_se_mesurent_bout_a_bout() {
        // [0,1] et [5,6] en X, écartés de 3 en Y : les bouts les plus proches
        // sont (1,0) et (5,3), soit un 3-4-5.
        let d = distance_segments(
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 0.0, 0.0),
            vec3(5.0, 3.0, 0.0),
            vec3(6.0, 3.0, 0.0),
        );
        assert!((d - 5.0).abs() < 1e-5, "attendu 5 (3-4-5), obtenu {d}");
    }

    #[test]
    fn la_distance_entre_segments_est_symetrique() {
        let (p1, q1) = (vec3(-2.0, 1.0, 0.5), vec3(3.0, -1.0, 2.0));
        let (p2, q2) = (vec3(0.0, 4.0, -1.0), vec3(1.0, 4.5, 3.0));
        let ab = distance_segments(p1, q1, p2, q2);
        let ba = distance_segments(p2, q2, p1, q1);
        assert!((ab - ba).abs() < 1e-4, "{ab} vs {ba}");
    }

    // Filet de sécurité sur l'algèbre : la distance segment↔segment doit valoir
    // le **minimum échantillonné** le long des deux segments. Écrit exprès de
    // façon indépendante (force brute), pour ne pas récrire la formule testée.
    #[test]
    fn la_distance_entre_segments_vaut_le_minimum_echantillonne() {
        let cas = [
            (vec3(-2.0, 1.0, 0.5), vec3(3.0, -1.0, 2.0), vec3(0.0, 4.0, -1.0), vec3(1.0, 4.5, 3.0)),
            (vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 10.0), vec3(2.0, 0.0, 3.0), vec3(2.0, 0.0, 7.0)),
            (vec3(-1.0, -1.0, -1.0), vec3(1.0, 1.0, 1.0), vec3(-1.0, 1.0, 0.0), vec3(1.0, -1.0, 0.0)),
            (vec3(0.0, 0.0, 0.0), vec3(5.0, 0.0, 0.0), vec3(10.0, 0.0, 0.0), vec3(15.0, 0.0, 0.0)),
        ];
        for (p1, q1, p2, q2) in cas {
            let exact = distance_segments(p1, q1, p2, q2);
            let n = 400;
            let mut brut = f32::INFINITY;
            for i in 0..=n {
                let u = p1 + (q1 - p1) * (i as f32 / n as f32);
                for j in 0..=n {
                    brut = brut.min(u.distance(p2 + (q2 - p2) * (j as f32 / n as f32)));
                }
            }
            // L'échantillonnage ne peut que **surestimer** : il rate le vrai
            // minimum d'au plus un pas.
            assert!(exact <= brut + 1e-3, "exact {exact} > brut {brut}");
            assert!(brut - exact < 5e-2, "exact {exact} trop loin sous brut {brut}");
        }
    }

    // --- distance point / segment ↔ rectangle ---

    /// Grille de points échantillonnant un rectangle, pour les contrôles en
    /// force brute qui suivent.
    fn grille_rectangle(centre: Vec3, eu: Vec3, ev: Vec3, hu: f32, hv: f32, n: usize) -> Vec<Vec3> {
        (0..=n)
            .flat_map(|i| {
                (0..=n).map(move |j| {
                    let s = -hu + 2.0 * hu * i as f32 / n as f32;
                    let t = -hv + 2.0 * hv * j as f32 / n as f32;
                    (s, t)
                })
            })
            .map(|(s, t)| centre + eu * s + ev * t)
            .collect()
    }

    #[test]
    fn le_point_dedans_le_rectangle_est_a_distance_nulle() {
        let d = distance_point_rectangle(vec3(1.0, 2.0, 0.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 5.0);
        assert!(d < 1e-5, "{d}");
    }

    #[test]
    fn le_point_hors_dun_coin_se_mesure_au_coin() {
        // Coin à (5, 3, 0), point à (8, 7, 0) : distance = hypot(3, 4) = 5.
        let d = distance_point_rectangle(vec3(8.0, 7.0, 0.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0);
        assert!((d - 5.0).abs() < 1e-4, "{d}");
    }

    #[test]
    fn le_point_hors_dun_bord_se_mesure_perpendiculairement() {
        // Bord à x=5, point à (5, 0, 4) : hors-plan pur, distance 4.
        let d = distance_point_rectangle(vec3(5.0, 0.0, 4.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0);
        assert!((d - 4.0).abs() < 1e-4, "{d}");
    }

    // Contrôle en force brute, indépendant de la formule testée : la distance
    // exacte doit être ≤ au minimum échantillonné sur une grille du rectangle
    // (jamais de sous-estimation possible sinon), et proche de lui.
    #[test]
    fn distance_segment_rectangle_vaut_le_minimum_echantillonne() {
        let cas: Vec<(Vec3, Vec3, Vec3, Vec3, Vec3, f32, f32)> = vec![
            // Segment qui reste dans la zone « hors coin » (s et t hors bornes)
            // sur toute sa longueur, minimum au bout **b**, pas au bout **a** ni
            // à une bordure — le cas qui distingue un vrai sommet de parabole
            // d'un repli sur l'extrémité basse du morceau.
            (vec3(9.0, 9.0, -1.0), vec3(6.0, 6.0, -1.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 5.0),
            // Segment franchement au-dessus du centre.
            (vec3(0.0, 0.0, 5.0), vec3(0.0, 0.0, 8.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0),
            // Segment qui traverse le plan du rectangle en son centre.
            (vec3(0.0, 0.0, -3.0), vec3(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0),
            // Segment oblique, passant au-dessus d'un coin.
            (vec3(4.0, 2.0, 6.0), vec3(6.0, 4.0, -6.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0),
            // Segment couché dans le plan du rectangle, à côté (dans le plan).
            (vec3(8.0, -10.0, 0.0), vec3(8.0, 10.0, 0.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0),
            // Segment parallèle à un bord, décalé hors-plan et en dehors.
            (vec3(-2.0, 6.0, 2.0), vec3(2.0, 6.0, 2.0), Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0),
            // Rectangle basculé, segment quelconque.
            (
                vec3(2.0, -4.0, 3.0),
                vec3(-1.0, 5.0, -2.0),
                vec3(1.0, 1.0, 1.0),
                vec3(1.0, 0.0, -1.0),
                vec3(1.0, -2.0, 1.0),
                4.0,
                2.0,
            ),
        ];
        for (a, b, centre, eu_brut, ev_brut, hu, hv) in cas {
            let eu = eu_brut.normalize();
            let ev = (ev_brut - eu * eu.dot(ev_brut)).normalize();
            let exact = distance_segment_rectangle(a, b, centre, eu, ev, hu, hv);

            let n = 300;
            let grille = grille_rectangle(centre, eu, ev, hu, hv, 40);
            let mut brut = f32::INFINITY;
            for i in 0..=n {
                let p = a + (b - a) * (i as f32 / n as f32);
                for g in &grille {
                    brut = brut.min(p.distance(*g));
                }
            }
            assert!(exact <= brut + 1e-2, "exact {exact} > brut {brut} pour {a:?}-{b:?}");
            assert!(brut - exact < 0.15, "exact {exact} trop loin sous brut {brut} (pas de grille)");
        }
    }

    // --- distance rectangle ↔ rectangle ---

    // Le contrôle qui justifie l'algorithme du §9.3 : deux plaques parallèles
    // empilées le long de leur axe commun (le cas des boucliers sur un mât) ont
    // pour distance l'écart pur entre leurs plans, pas la diagonale de leur
    // englobant sphérique.
    #[test]
    fn deux_plaques_paralleles_empilees_se_mesurent_par_lecart_pur() {
        let d = distance_rectangles(
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            5.0,
            5.0,
            Vec3::Z * 7.0,
            Vec3::X,
            Vec3::Y,
            5.0,
            5.0,
        );
        assert!((d - 7.0).abs() < 1e-4, "{d}");
    }

    // Une plaque strictement plus petite, imbriquée dans une autre plaque
    // coplanaire : les bords ne se croisent jamais, et pourtant la distance
    // doit être nulle (elles occupent le même point). C'est le cas que
    // l'énumération bord↔bord seule raterait.
    #[test]
    fn une_petite_plaque_imbriquee_dans_une_grande_est_a_distance_nulle() {
        let d = distance_rectangles(
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            10.0,
            10.0,
            vec3(1.0, 1.0, 0.0),
            Vec3::X,
            Vec3::Y,
            2.0,
            2.0,
        );
        assert!(d < 1e-4, "{d}");
    }

    // Deux plaques dont les plans se croisent obliquement, sans qu'aucun bord
    // ne traverse l'autre — la transperçée franche. Le point de contact est
    // porté par un bord (là où il coupe le plan de l'autre), pas par une
    // intersection bord↔bord.
    #[test]
    fn deux_plaques_qui_se_transpercent_obliquement_sont_a_distance_nulle() {
        let d = distance_rectangles(
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            6.0,
            6.0,
            Vec3::ZERO,
            Vec3::X,
            Vec3::Z,
            6.0,
            6.0,
        );
        assert!(d < 1e-4, "{d}");
    }

    // Contrôle en force brute sur des rectangles d'orientations quelconques
    // (pas nécessairement parallèles) : la distance exacte doit être ≤ au
    // minimum échantillonné sur une grille de chacun, et proche de lui.
    #[test]
    fn distance_rectangles_vaut_le_minimum_echantillonne() {
        let cas: Vec<(Vec3, Vec3, Vec3, f32, f32, Vec3, Vec3, Vec3, f32, f32)> = vec![
            // Parallèles, décalées et translatées dans le plan.
            (
                Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0,
                vec3(3.0, 1.0, 6.0), Vec3::X, Vec3::Y, 4.0, 2.0,
            ),
            // Perpendiculaires, qui ne se touchent pas.
            (
                Vec3::ZERO, Vec3::X, Vec3::Y, 5.0, 3.0,
                vec3(0.0, 0.0, 10.0), Vec3::X, Vec3::Z, 4.0, 2.0,
            ),
            // Orientations obliques quelconques, séparées.
            (
                vec3(-3.0, 2.0, 1.0), vec3(1.0, 1.0, 0.0), vec3(-1.0, 1.0, 1.0), 3.0, 2.0,
                vec3(4.0, -1.0, 5.0), vec3(0.0, 1.0, 1.0), vec3(1.0, 0.0, -1.0), 2.5, 1.5,
            ),
        ];
        for (c1, eu1b, ev1b, hu1, hv1, c2, eu2b, ev2b, hu2, hv2) in cas {
            let eu1 = eu1b.normalize();
            let ev1 = (ev1b - eu1 * eu1.dot(ev1b)).normalize();
            let eu2 = eu2b.normalize();
            let ev2 = (ev2b - eu2 * eu2.dot(ev2b)).normalize();
            let exact = distance_rectangles(c1, eu1, ev1, hu1, hv1, c2, eu2, ev2, hu2, hv2);

            let g1 = grille_rectangle(c1, eu1, ev1, hu1, hv1, 25);
            let g2 = grille_rectangle(c2, eu2, ev2, hu2, hv2, 25);
            let mut brut = f32::INFINITY;
            for p in &g1 {
                for q in &g2 {
                    brut = brut.min(p.distance(*q));
                }
            }
            assert!(exact <= brut + 1e-2, "exact {exact} > brut {brut}");
            assert!(brut - exact < 0.3, "exact {exact} trop loin sous brut {brut} (pas de grille)");
        }
    }

    // --- l'enveloppe elle-même ---

    #[test]
    fn une_sphere_est_une_capsule_degeneree() {
        let s = Enveloppe::sphere(vec3(1.0, 2.0, 3.0), 2.0);
        assert!(s.est_sphere());
        assert!((s.rayon_sphere() - 2.0).abs() < 1e-5);
        assert!(s.contient(vec3(1.0, 2.0, 4.9)));
        assert!(!s.contient(vec3(1.0, 2.0, 5.1)));
    }

    // **Le gain qu'on est venu chercher**, chiffré sur la pièce qui motivait le
    // changement : un radiateur méga de 30 × 8. La sphère qui le contient a un
    // rayon de ~15 ; la capsule qui le contient a un rayon de ~4. C'est ce
    // facteur qui décide qu'une pose voisine est refusée ou acceptée.
    #[test]
    fn la_capsule_serre_une_piece_allongee_bien_mieux_quune_sphere() {
        let (demi_long, demi_large) = (15.0, 4.0);
        let capsule = Enveloppe::axe(Vec3::ZERO, Vec3::Z, demi_long, demi_large);
        // La **plus petite** sphère qui contient la pièce : son rayon passe par
        // les coins. C'est la comparaison honnête — pas la sphère qui
        // contiendrait la capsule, qui serait déjà pénalisée par les calottes.
        let sphere = Enveloppe::sphere(Vec3::ZERO, demi_long.hypot(demi_large));

        // Les deux contiennent bien les quatre coins de la pièce.
        for sz in [-1.0_f32, 1.0] {
            for sx in [-1.0_f32, 1.0] {
                let coin = vec3(sx * demi_large, 0.0, sz * demi_long);
                assert!(capsule.contient(coin), "capsule : coin {coin:?} dehors");
                assert!(sphere.contient(coin), "sphère : coin {coin:?} dehors");
            }
        }
        // Mais la sphère réserve près de **quatre fois** plus large en travers.
        assert!(sphere.rayon > capsule.rayon * 3.5, "{} vs {}", sphere.rayon, capsule.rayon);

        // Conséquence concrète : deux radiateurs côte à côte, écartés de 10 —
        // donc franchement séparés, leurs 8 de large ne se touchant pas. La
        // capsule les accepte, la sphère les refuse. C'est le faux refus qu'on
        // est venu supprimer.
        let voisin = Enveloppe::axe(vec3(10.0, 0.0, 0.0), Vec3::Z, demi_long, demi_large);
        assert!(capsule.ecart(&voisin) > 0.0, "capsule : voisin refusé à tort");
        assert!(sphere.ecart(&voisin) < 0.0, "sphère : voisin accepté, le cas n'illustre plus rien");
    }

    // Le pendant du test précédent, pour le boudin : une plaque de rayon 12 et
    // d'épaisseur 1,8 empilée sur elle-même (le cas des boucliers de l'ISV, sur
    // un mât). La sphère équivalente réserve 12 de vide en épaisseur ; le
    // boudin, ~1. C'est ce facteur qui décide si l'empilement tient.
    #[test]
    fn le_boudin_serre_une_plaque_bien_mieux_quune_sphere() {
        let (rayon_disque, demi_epaisseur) = (12.0, 0.9);
        let boudin = Enveloppe::plaque(Vec3::ZERO, Vec3::X, Vec3::Y, rayon_disque, rayon_disque, demi_epaisseur);
        // La **plus petite** sphère qui contient la plaque : son rayon passe par
        // le bord du disque, épaisseur comprise — pas juste `rayon_disque`, qui
        // laisserait le bord de la tranche dehors.
        let sphere = Enveloppe::sphere(Vec3::ZERO, rayon_disque.hypot(demi_epaisseur));

        // Les deux contiennent la plaque (échantillonnée sur son disque et son
        // épaisseur).
        for k in 0..8 {
            let a = std::f32::consts::TAU * k as f32 / 8.0;
            for z in [-demi_epaisseur, demi_epaisseur] {
                let p = vec3(rayon_disque * a.cos(), rayon_disque * a.sin(), z);
                assert!(boudin.contient(p), "boudin : point {p:?} dehors");
                assert!(sphere.contient(p), "sphère : point {p:?} dehors");
            }
        }
        assert!(sphere.rayon > boudin.rayon * 10.0, "{} vs {}", sphere.rayon, boudin.rayon);

        // Empilement le long de +Z, écart de 3 entre les plaques (comme sur le
        // mât de l'ISV) : le boudin l'accepte, la sphère le refuse à tort.
        let voisin_boudin = boudin.transformee(Mat4::from_translation(Vec3::Z * (2.0 * demi_epaisseur + 3.0)));
        let voisin_sphere = sphere.transformee(Mat4::from_translation(Vec3::Z * (2.0 * demi_epaisseur + 3.0)));
        assert!(boudin.ecart(&voisin_boudin) > 0.0, "boudin : empilement refusé à tort");
        assert!(sphere.ecart(&voisin_sphere) < 0.0, "sphère : empilement accepté, le cas n'illustre plus rien");
    }

    // Une transformée rigide déplace la capsule sans la déformer : c'est
    // l'hypothèse qui permet de ne pas recalculer d'enveloppe à chaque pose.
    #[test]
    fn une_transformee_rigide_deplace_la_capsule_sans_la_deformer() {
        let c = Enveloppe::capsule(vec3(-3.0, 0.0, 0.0), vec3(3.0, 0.0, 0.0), 1.5);
        let m = Mat4::from_rotation_translation(
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            vec3(10.0, -4.0, 2.0),
        );
        let t = c.transformee(m);
        assert!((t.rayon - c.rayon).abs() < 1e-6, "le rayon doit être conservé");
        let Noyau::Segment { a: ta, b: tb } = t.noyau else { panic!("attendu un segment") };
        let Noyau::Segment { a: ca, b: cb } = c.noyau else { panic!("attendu un segment") };
        assert!((ta.distance(tb) - ca.distance(cb)).abs() < 1e-4, "la longueur aussi");
        // Et le contenu suit : un point dedans reste dedans une fois transformé.
        let dedans = vec3(2.0, 1.0, 0.0);
        assert!(c.contient(dedans));
        assert!(t.contient(m.transform_point3(dedans)));
    }

    // Le même contrôle pour le boudin, y compris sous une **réflexion**
    // (déterminant −1, comme les symétries de la station) : `eu`/`ev` doivent
    // rester unitaires et orthogonaux, sans quoi les formules de distance en
    // amont deviennent fausses en silence.
    #[test]
    fn une_transformee_rigide_deplace_le_boudin_sans_le_deformer() {
        let plaque = Enveloppe::plaque(vec3(1.0, -2.0, 0.5), Vec3::X, Vec3::Y, 6.0, 4.0, 0.8);
        // Rotation + réflexion (miroir en X) + translation : le cocktail complet
        // de transformées que ce projet applique aux enveloppes.
        let m = Mat4::from_scale(vec3(-1.0, 1.0, 1.0))
            * Mat4::from_rotation_translation(
                Quat::from_axis_angle(vec3(0.3, 0.7, 0.2).normalize(), 1.1),
                vec3(5.0, -3.0, 2.0),
            );
        let t = plaque.transformee(m);
        let Noyau::Rectangle { eu, ev, hu, hv, .. } = t.noyau else { panic!("attendu un rectangle") };
        let Noyau::Rectangle { hu: hu0, hv: hv0, .. } = plaque.noyau else { unreachable!() };
        assert!((eu.length() - 1.0).abs() < 1e-4, "eu non unitaire après transformée");
        assert!((ev.length() - 1.0).abs() < 1e-4, "ev non unitaire après transformée");
        assert!(eu.dot(ev).abs() < 1e-4, "eu, ev non orthogonaux après transformée");
        assert!((hu - hu0).abs() < 1e-5 && (hv - hv0).abs() < 1e-5, "les demi-étendues doivent être conservées");

        // Et le contenu suit, comme pour la capsule.
        let dedans = vec3(1.0, -2.0, 0.9);
        assert!(plaque.contient(dedans), "point censé être dedans avant transformée");
        assert!(t.contient(m.transform_point3(dedans)), "…et après");
    }

    // Deux enveloppes qui se touchent exactement ont un écart nul ; l'écart est
    // négatif quand elles s'interpénètrent, et c'est **ce signe** que
    // l'anti-collision lit.
    #[test]
    fn lecart_change_de_signe_au_contact() {
        let a = Enveloppe::sphere(Vec3::ZERO, 2.0);
        assert!((a.ecart(&Enveloppe::sphere(vec3(5.0, 0.0, 0.0), 3.0))).abs() < 1e-5, "contact");
        assert!(a.ecart(&Enveloppe::sphere(vec3(6.0, 0.0, 0.0), 3.0)) > 0.0, "séparées");
        assert!(a.ecart(&Enveloppe::sphere(vec3(4.0, 0.0, 0.0), 3.0)) < 0.0, "imbriquées");
    }

    // --- L3.3 : le rayon de désignation (§8.3) ---

    /// Les trois formes de noyau, pour balayer `touche_rayon` sur chacune.
    fn formes() -> [Enveloppe; 3] {
        [
            Enveloppe::sphere(vec3(0.0, 1.0, 0.0), 1.5),
            Enveloppe::capsule(vec3(-3.0, 0.0, 0.0), vec3(3.0, 0.5, 1.0), 0.8),
            Enveloppe::plaque(vec3(0.0, 0.0, 2.0), Vec3::X, Vec3::Y, 2.5, 1.5, 0.2),
        ]
    }

    // Contrôle en force brute, dans le **seul sens où il est concluant** : si un
    // point échantillonné du rayon est dedans, alors le rayon touche — sans
    // réserve. L'implication inverse ne tient pas (un rayon peut effleurer
    // l'enveloppe entre deux échantillons), elle est donc vérifiée séparément,
    // sur des cas francs, par `un_rayon_qui_passe_loin_ne_touche_rien`.
    #[test]
    fn touche_rayon_ne_rate_aucun_point_reellement_traverse() {
        let oeil = vec3(-12.0, 6.0, -9.0);
        let mut touches = 0;
        for env in formes() {
            for i in 0..12 {
                for j in 0..12 {
                    let cible = vec3(i as f32 * 0.6 - 3.5, j as f32 * 0.4 - 2.0, 1.0);
                    let dir = (cible - oeil).normalize();
                    let portee = (env.centre() - oeil).length() + env.rayon_sphere();
                    let dedans = (0..=3000)
                        .any(|k| env.contient(oeil + dir * (portee * k as f32 / 3000.0)));
                    if dedans {
                        assert!(
                            env.touche_rayon(oeil, dir).is_some(),
                            "rayon vers {cible:?} traverse {env:?} sans être détecté"
                        );
                        touches += 1;
                    }
                }
            }
        }
        assert!(touches >= 20, "balayage trop maigre : {touches} traversées réelles");
    }

    #[test]
    fn un_rayon_qui_passe_loin_ne_touche_rien() {
        let env = Enveloppe::capsule(vec3(-2.0, 0.0, 0.0), vec3(2.0, 0.0, 0.0), 0.5);
        // Parallèle à l'axe, décalé bien au-delà du rayon.
        assert!(env.touche_rayon(vec3(-10.0, 4.0, 0.0), Vec3::X).is_none());
        // Vers l'enveloppe, mais **dans le dos** : une demi-droite, pas une droite.
        assert!(env.touche_rayon(vec3(10.0, 0.0, 0.0), Vec3::X).is_none(), "part à l'opposé");
        assert!(env.touche_rayon(vec3(10.0, 0.0, 0.0), -Vec3::X).is_some(), "revient dessus");
    }

    // La valeur rendue est la distance de l'œil à la **surface** : c'est elle
    // qui départage deux pièces percées par le même rayon.
    #[test]
    fn touche_rayon_rend_la_distance_a_la_surface() {
        let env = Enveloppe::sphere(vec3(10.0, 0.0, 0.0), 2.0);
        let d = env.touche_rayon(Vec3::ZERO, Vec3::X).unwrap();
        assert!((d - 8.0).abs() < 1e-4, "8 attendu (10 − 2), obtenu {d}");
        // Œil à l'intérieur : distance nulle, jamais négative.
        let dedans = env.touche_rayon(vec3(10.0, 0.0, 0.0), Vec3::X).unwrap();
        assert_eq!(dedans, 0.0);
    }
}
