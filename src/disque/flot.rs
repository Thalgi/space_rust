//! **Le flot bipolaire d'une étoile qui accrète** (lot 12, 2026-10-24) : ce
//! qui sort par ses pôles, et qu'on veut le **standard de tous les jets
//! polaires** du chantier.
//!
//! Une étoile jeune éjecte autant qu'elle avale : un **jet** fin et rapide le
//! long de l'axe du disque, et autour de lui un vent plus large qui entraîne
//! la poussière et creuse deux **cavités** en cône — dont les parois, éclairées
//! par l'étoile, dessinent un sablier (HH 30, le *Flying Saucer*). Le long du
//! jet filent des **nœuds** brillants, des ondes de choc en forme d'étrave
//! (les objets de Herbig-Haro), rouges de l'hydrogène et du soufre ionisés.
//!
//! *« Ceux-ci canalisent une partie de la poussière et laissent une traînée
//! plus large »* — l'utilisateur, en ouvrant ce rework.
//!
//! Même repère que `bras` : local au disque (sa normale, l'axe des jets, en
//! Y), relatif au centre de l'étoile ; des boules qui coulent vers le haut et
//! vers le bas.

use super::bras::Boule;
use macroquad::prelude::*;
use std::f32::consts::TAU;

/// **Ce qui compose un flot bipolaire.** Tout à zéro : aucun.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flot {
    /// Longueur du flot de chaque côté (unités du monde), 0 = aucun.
    pub longueur: f32,
    /// Rayon de l'astre, d'où part le jet.
    pub rayon_astre: f32,
    /// Le jet lui-même, fin et vif, 0..1.
    pub jet: f32,
    /// **J1** : le cône de poussière entraînée, qui s'élargit en
    /// s'éloignant — la traînée. 0..1.
    pub cone: f32,
    /// **J2** : nœuds de Herbig-Haro par côté.
    pub noeuds: u32,
    /// **J3** : parois des cavités, éclairées — le sablier. 0..1.
    pub parois: f32,
    /// **J5** : finesse des parois (0 = épaisses, 1 = un fil) ; au-delà de 0,
    /// elles ne sont plus éclairées partout, mais par plaques, d'un côté.
    pub finesse: f32,
    /// **Dissymétrie** des deux lobes, 0..1 : à 0, l'un est le miroir de
    /// l'autre — *« ça se voit que c'est une symétrie »* (2026-10-24, J5). Au-
    /// delà, le lobe du dessous est plus court, plus fermé, plus terne, et le
    /// flot se courbe en S (la précession du jet), comme les vrais.
    pub asymetrie: f32,
    /// Ouverture de la cavité (multiplicateur, 0 = 1) : posée par lobe.
    pub ouverture: f32,
}

/// **Où commence le sablier**, en part de la longueur : il partait de
/// l'étoile, et son pied, le plus vif, la noyait — *« il faut augmenter la
/// distance entre le début du sablier et l'astre, car on ne le voit plus ;
/// il faut que ce soit léger »* (2026-10-24, J3/J4).
pub const DEPART_PAROIS: f32 = 0.15;

/// Pseudo-hasard déterministe dans `[0 ; 1[`.
fn h(i: f32, s: f32) -> f32 {
    ((i * 12.9898 + s * 37.719).sin() * 43_758.547).fract().abs()
}

/// **Le rayon de la cavité** à la hauteur `z` : un sablier parabolique,
/// étroit au pied, large de 45 % de la longueur au bout.
pub fn rayon_cavite(f: &Flot, z: f32) -> f32 {
    let ouverture = if f.ouverture > 0.0 { f.ouverture } else { 1.0 };
    0.45 * ouverture * (f.longueur * z.abs().min(f.longueur)).sqrt()
}

/// **Le lobe d'un côté** (`cote` = ±1) : le flot tel quel au-dessus ; au-
/// dessous, plus court, plus fermé et plus terne selon la dissymétrie.
pub fn lobe(f: &Flot, cote: f32) -> Flot {
    let a = if cote > 0.0 { 0.0 } else { f.asymetrie };
    let vif = 1.0 - 0.45 * a;
    Flot {
        longueur: f.longueur * (1.0 - 0.3 * a),
        ouverture: 1.0 - 0.2 * a,
        jet: f.jet * vif,
        cone: f.cone * vif,
        parois: f.parois * vif,
        ..*f
    }
}

/// **Toutes les boules du flot** au temps `temps`, les deux côtés.
pub fn boules(f: &Flot, temps: f32) -> Vec<Boule> {
    if f.longueur <= 0.0 {
        return Vec::new();
    }
    let mut v = Vec::new();
    for cote in [1.0_f32, -1.0] {
        let graine = if cote > 0.0 { 0.0 } else { 50_000.0 };
        let fc = lobe(f, cote);
        let debut = v.len();
        jet(&fc, temps, cote, graine, &mut v);
        cone(&fc, temps, cote, graine, &mut v);
        noeuds(&fc, temps, cote, graine, &mut v);
        parois(&fc, temps, cote, graine, &mut v);
        // La précession courbe le flot en S : chaque lobe part de son côté,
        // d'autant plus qu'il s'éloigne.
        if f.asymetrie > 0.0 {
            for b in &mut v[debut..] {
                let y = b.0.y.abs() / f.longueur;
                b.0.x += cote * f.asymetrie * 0.12 * f.longueur * y * y;
            }
        }
    }
    v
}

/// Le jet : un filet vif qui file le long de l'axe, en se perdant.
fn jet(f: &Flot, temps: f32, cote: f32, g: f32, v: &mut Vec<Boule>) {
    if f.jet <= 0.0 {
        return;
    }
    let l = f.longueur;
    for j in 0..300 {
        let i = j as f32 + g;
        let s = (h(i, 1.0) + temps * 0.25).fract();
        let z = f.rayon_astre + s * (l - f.rayon_astre);
        let ecart = 0.012 * l * (0.3 + s);
        let a = h(i, 2.0) * TAU;
        let p = vec3(a.cos() * ecart * h(i, 3.0), cote * z, a.sin() * ecart * h(i, 3.0));
        v.push((p, 0.012 * l, vec3(0.85, 0.9, 1.0), f.jet * 0.4 * (1.0 - s).sqrt()));
    }
}

/// **J1** — la traînée : la poussière entraînée remplit un cône qui
/// s'élargit, monte lentement et s'efface au loin ; claire au pied, où
/// l'étoile l'éclaire, brune plus haut.
fn cone(f: &Flot, temps: f32, cote: f32, g: f32, v: &mut Vec<Boule>) {
    if f.cone <= 0.0 {
        return;
    }
    let l = f.longueur;
    for j in 0..1500 {
        let i = j as f32 + g + 1_000.0;
        let s = (h(i, 1.0) + temps * 0.04).fract();
        let z = s * l;
        let rho = rayon_cavite(f, z) * 0.9 * h(i, 2.0).sqrt();
        let a = h(i, 3.0) * TAU;
        let p = vec3(a.cos() * rho, cote * z, a.sin() * rho);
        let c = vec3(1.0, 0.7, 0.45).lerp(vec3(0.55, 0.35, 0.22), s);
        // Effacée au ras de l'étoile, qu'elle noierait.
        let monte = (s / 0.12).min(1.0).powi(2);
        v.push((p, 0.025 * l + 0.05 * z, c, f.cone * 0.32 * (1.0 - s) * monte));
    }
}

/// **J2** — les nœuds : des étraves de choc qui filent le long du jet,
/// régulièrement espacées, rouges, l'apex en avant.
fn noeuds(f: &Flot, temps: f32, cote: f32, g: f32, v: &mut Vec<Boule>) {
    let n = f.noeuds;
    let l = f.longueur;
    for k in 0..n {
        let s = (k as f32 / n as f32 + temps * 0.05).fract();
        let z = f.rayon_astre * 2.0 + s * l;
        let rk = 0.05 * l * (0.5 + s);
        let fondu = (1.0 - s) * (s * 8.0).min(1.0);
        for j in 0..40 {
            let i = (k * 40 + j) as f32 + g + 3_000.0;
            let alpha = h(i, 1.0) * 1.1;
            let phi = h(i, 2.0) * TAU;
            // Une calotte tournée vers l'avant : l'apex devant, les ailes
            // rabattues vers l'étoile.
            let p = vec3(alpha.sin() * phi.cos() * rk, cote * (z - (1.0 - alpha.cos()) * rk * 1.6), alpha.sin() * phi.sin() * rk);
            let apex = 1.0 - alpha / 1.1;
            v.push((p, rk * 0.4, vec3(1.0, 0.45, 0.42).lerp(vec3(1.0, 0.85, 0.8), apex * 0.6), fondu * (0.45 + 0.55 * apex)));
        }
    }
}

/// **J3 / J5** — les parois des cavités : une coque en sablier, éclairée par
/// l'étoile. Elle commence à `DEPART_PAROIS` de la longueur, légère d'abord,
/// vive ensuite, éteinte au bout. Fine (`finesse`), elle se réduit à un fil
/// et ne s'éclaire plus que par plaques, d'un côté — comme HH 30.
fn parois(f: &Flot, temps: f32, cote: f32, g: f32, v: &mut Vec<Boule>) {
    if f.parois <= 0.0 {
        return;
    }
    let l = f.longueur;
    let epaisseur = 0.18 * (1.0 - f.finesse).max(0.05);
    let cote_eclaire = 0.7 + if cote > 0.0 { 0.0 } else { 2.4 };
    // Fine, la paroi n'est vue que par plaques : deux fois plus de boules,
    // pour qu'elles se lisent comme des plaques et pas comme des points.
    let n = if f.finesse > 0.0 { 3200 } else { 1600 };
    for j in 0..n {
        let i = j as f32 + g + 6_000.0;
        let s = (h(i, 1.0) + temps * 0.01).fract().powf(1.5);
        let z = (DEPART_PAROIS + s * (1.0 - DEPART_PAROIS)) * l;
        let phi = h(i, 2.0) * TAU;
        let rho = rayon_cavite(f, z) * (1.0 + epaisseur * (h(i, 3.0) - 0.5));
        let monte = (s / 0.3).min(1.0);
        let mut lum = monte * monte * (3.0 - 2.0 * monte) * (1.0 - s).powf(0.7);
        if f.finesse > 0.0 {
            // Par plaques, d'un côté : le reste de la paroi est dans l'ombre.
            let face = (0.5 + 0.5 * (phi - cote_eclaire).cos()).powi(3);
            let plaque = ((h((phi * 6.0).floor() + (s * 10.0).floor() * 13.0 + g, 7.0) - 0.35) / 0.65).clamp(0.0, 1.0);
            lum *= face * plaque;
            if lum < 0.02 {
                continue;
            }
        }
        let p = vec3(phi.cos() * rho, cote * z, phi.sin() * rho);
        let taille = l * (0.018 + 0.024 * (1.0 - f.finesse)) * (0.4 + s);
        // Fine, la paroi concentre son éclat sur ses plaques : plus vive.
        let eclat = 0.6 + 0.6 * f.finesse;
        v.push((p, taille, vec3(1.0, 0.85, 0.65), f.parois * eclat * lum));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flot() -> Flot {
        Flot { longueur: 50.0, rayon_astre: 1.0, jet: 1.0, cone: 1.0, noeuds: 4, parois: 1.0, ..Flot::default() }
    }

    /// **Un flot sort par les deux pôles, le long de l'axe, et ne déborde
    /// pas sa cavité** : la traînée s'élargit en montant.
    #[test]
    fn le_flot_sort_par_les_deux_poles_et_s_elargit() {
        let f = flot();
        let b = boules(&f, 3.0);
        let haut = b.iter().filter(|x| x.0.y > 1.0).count();
        let bas = b.iter().filter(|x| x.0.y < -1.0).count();
        assert!(haut > 500 && bas > 500, "{haut} en haut, {bas} en bas");
        for x in &b {
            let rho = vec2(x.0.x, x.0.z).length();
            assert!(x.0.y.abs() <= f.longueur * 1.01 + f.rayon_astre * 2.0, "hors du flot : {}", x.0.y);
            assert!(rho <= rayon_cavite(&f, x.0.y) * 1.25 + f.longueur * 0.08, "hors de la cavité : ρ {rho} à z {}", x.0.y);
        }
        assert!(rayon_cavite(&f, 40.0) > rayon_cavite(&f, 10.0) * 1.5, "la traînée ne s'élargit pas");
        assert!(boules(&Flot::default(), 1.0).is_empty());
    }

    /// **Le sablier laisse l'étoile à découvert** : ses parois commencent à
    /// distance, et s'y allument doucement.
    #[test]
    fn le_sablier_commence_loin_de_l_etoile() {
        let f = Flot { jet: 0.0, cone: 0.0, noeuds: 0, ..flot() };
        let b = boules(&f, 2.0);
        assert!(!b.is_empty());
        let min = b.iter().map(|x| x.0.y.abs()).fold(f32::MAX, f32::min);
        assert!(min >= DEPART_PAROIS * f.longueur * 0.99, "paroi à {min} de l'étoile");
        let pied: Vec<_> = b.iter().filter(|x| x.0.y.abs() < (DEPART_PAROIS + 0.03) * f.longueur).collect();
        let plus_haut: Vec<_> = b.iter().filter(|x| (0.3..0.4).contains(&(x.0.y.abs() / f.longueur))).collect();
        let moy = |v: &[&Boule]| v.iter().map(|x| x.3).sum::<f32>() / v.len().max(1) as f32;
        assert!(moy(&pied) < moy(&plus_haut) * 0.5, "le pied n'est pas léger : {} vs {}", moy(&pied), moy(&plus_haut));
    }

    /// **Dissymétriques, les deux lobes ne sont plus le miroir l'un de
    /// l'autre** : celui du dessous est plus court et plus terne, et le flot
    /// se courbe. Sans dissymétrie, ils se valent.
    #[test]
    fn deux_lobes_dissymetriques_ne_se_refletent_pas() {
        let etendue = |f: &Flot, cote: f32| {
            boules(f, 1.0).iter().filter(|x| x.0.y * cote > 0.0).map(|x| x.0.y.abs()).fold(0.0, f32::max)
        };
        let sym = flot();
        assert!((etendue(&sym, 1.0) - etendue(&sym, -1.0)).abs() < sym.longueur * 0.05);
        let asym = Flot { asymetrie: 1.0, ..flot() };
        assert!(etendue(&asym, 1.0) > etendue(&asym, -1.0) * 1.2, "même longueur");
        assert!(lobe(&asym, -1.0).parois < lobe(&asym, 1.0).parois * 0.7, "même éclat");
        let bout_haut = boules(&asym, 1.0).iter().filter(|x| x.0.y > 0.8 * asym.longueur).map(|x| x.0.x).sum::<f32>();
        assert!(bout_haut > 0.0, "pas de courbure");
    }

    /// **Fine, la paroi ne s'éclaire plus partout** : bien plus mince que la
    /// paroi épaisse, et l'essentiel de son éclat d'un même côté.
    #[test]
    fn une_paroi_fine_s_eclaire_par_plaques() {
        let epaisse = Flot { jet: 0.0, cone: 0.0, noeuds: 0, ..flot() };
        let fine = Flot { finesse: 0.9, ..epaisse };
        let (be, bf) = (boules(&epaisse, 1.0), boules(&fine, 1.0));
        let ecart = |b: &[Boule]| {
            b.iter()
                .filter(|x| x.0.y > 5.0)
                .map(|x| (vec2(x.0.x, x.0.z).length() / rayon_cavite(&epaisse, x.0.y) - 1.0).abs())
                .fold(0.0, f32::max)
        };
        assert!(ecart(&bf) < ecart(&be) * 0.3, "pas plus fine : {} vs {}", ecart(&bf), ecart(&be));
        let haut: Vec<_> = bf.iter().filter(|x| x.0.y > 0.0).collect();
        let total: f32 = haut.iter().map(|x| x.3).sum();
        let face: f32 = haut.iter().filter(|x| x.0.x * 0.7f32.cos() + x.0.z * 0.7f32.sin() > 0.0).map(|x| x.3).sum();
        assert!(face > total * 0.8, "pas d'un seul côté : {face} / {total}");
    }
}
