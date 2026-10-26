//! **Les bras de matière qu'avale l'étoile** (lot 12, T Tauri, 2026-10-23).
//!
//! Une T Tauri accrète par sa **magnétosphère** (Königl 1991 ; Bouvier et al.
//! 2007 ; Hartmann et al. 2016) : son champ, à peu près dipolaire, tronque
//! le disque à quelques rayons stellaires ; le gaz quitte le bord interne et
//! remonte les **lignes de champ** en colonnes — les entonnoirs, *funnel
//! flows* — jusqu'à des taches chaudes à haute latitude, en chute libre.
//! Dipôle incliné sur l'axe de rotation, il se rassemble en **deux flots**,
//! un par hémisphère (simulations de Romanova et al. 2003-2004). Plus loin,
//! la matière gagne le bord interne par des **bras spiraux**.
//!
//! Tout est calculé dans le **repère local du disque** (sa normale en Y,
//! comme `nuage_3d`), relatif au centre de l'étoile, et rendu en boules
//! lumineuses qui coulent le long de ces chemins — le pinceau des nuages.

use super::DisqueConfig;
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

/// Une boule d'un bras : `(position locale, rayon, couleur, éclat)`.
pub type Boule = (Vec3, f32, Vec3, f32);

/// **Les bras sont-ils allumés ?**
pub fn actifs(c: &DisqueConfig) -> bool {
    c.bord_brulant > 0.0
        || c.flot.longueur > 0.0
        || (c.rayon_astre > 0.0 && (c.entonnoirs > 0 || c.spirales_accretion > 0 || c.pluie > 0 || c.fragments > 0))
}

/// **Un point d'une ligne de champ dipolaire**, de son pied `pied` (dans le
/// disque) jusqu'à la surface de l'astre, `s` allant de 0 à 1.
///
/// Une ligne de champ d'un dipôle d'axe `m` suit `r = L · sin²θ`, `θ` mesuré
/// depuis `m`. Elle monte vers le pôle de l'hémisphère où est son pied
/// (`hemisphere` tranche quand le pied est sur l'équateur magnétique) et
/// touche l'astre où `sin²θ = rayon / L`. `None` si elle n'en sort pas.
pub fn ligne_dipole(pied: Vec3, m: Vec3, rayon: f32, hemisphere: f32, s: f32) -> Option<Vec3> {
    let r0 = pied.length();
    if r0 <= rayon || rayon <= 0.0 {
        return None;
    }
    let cos0 = (pied.dot(m) / r0).clamp(-1.0, 1.0);
    let theta0 = cos0.acos();
    let sin0 = theta0.sin();
    if sin0 < 1e-4 {
        return None;
    }
    let l = r0 / (sin0 * sin0);
    let e = (pied - m * pied.dot(m)).normalize_or_zero();
    let nord = if cos0.abs() > 1e-4 { cos0 > 0.0 } else { hemisphere >= 0.0 };
    let theta_astre = (rayon / l).sqrt().clamp(0.0, 1.0).asin();
    let theta1 = if nord { theta_astre } else { PI - theta_astre };
    let theta = theta0 + (theta1 - theta0) * s.clamp(0.0, 1.0);
    let r = l * theta.sin().powi(2);
    Some((m * theta.cos() + e * theta.sin()) * r)
}

/// **L'axe du dipôle**, incliné de `inclinaison` sur la normale du disque
/// (Y local) et tournant avec l'étoile : azimut `phase`.
pub fn axe_dipole(inclinaison: f32, phase: f32) -> Vec3 {
    let (si, ci) = inclinaison.sin_cos();
    vec3(si * phase.cos(), ci, si * phase.sin())
}

/// Pseudo-hasard déterministe dans `[0 ; 1[`.
fn h(i: f32, s: f32) -> f32 {
    ((i * 12.9898 + s * 37.719).sin() * 43_758.547).fract().abs()
}

/// La chute libre : la matière traîne au départ, file à l'arrivée.
fn chute(tau: f32) -> f32 {
    tau * tau
}

/// De l'orangé du disque au blanc-jaune de l'impact : la matière chauffe.
fn chaleur(s: f32) -> Vec3 {
    vec3(1.0, 0.55, 0.22).lerp(vec3(1.0, 0.95, 0.85), s.clamp(0.0, 1.0))
}

/// Vitesse angulaire de l'étoile : la corotation au bord interne du disque
/// (là où elle le tronque).
pub(crate) fn rotation(c: &DisqueConfig) -> f32 {
    (c.gm / c.interne.max(1e-6).powi(3)).sqrt()
}

/// **Les boules des entonnoirs** au temps `temps`.
///
/// Chaque flot part d'un arc du bord interne (±0,35 rad), centré, si le
/// dipôle est incliné, face à son pôle — d'où deux flots, un par hémisphère ;
/// aligné, les flots se répartissent tout autour et chaque boule tire son
/// hémisphère : des rideaux, vers les deux pôles.
pub fn entonnoirs(c: &DisqueConfig, temps: f32) -> Vec<Boule> {
    const PAR_FLOT: u32 = 280;
    let n = c.entonnoirs;
    if n == 0 || c.rayon_astre <= 0.0 {
        return Vec::new();
    }
    let omega = rotation(c);
    let phase = omega * temps;
    let m = axe_dipole(c.inclinaison_dipole, phase);
    let mut v = Vec::with_capacity((n * PAR_FLOT) as usize);
    for k in 0..n {
        let centre = phase + k as f32 * TAU / n as f32;
        for j in 0..PAR_FLOT {
            let i = (k * PAR_FLOT + j) as f32;
            let az = centre + (h(i, 1.0) - 0.5) * 0.7;
            let r0 = c.interne * (1.0 + 0.15 * h(i, 2.0));
            let pied = vec3(az.cos(), 0.0, az.sin()) * r0;
            let hemi = if h(i, 3.0) < 0.5 { 1.0 } else { -1.0 };
            let tau = (h(i, 4.0) + temps * omega / TAU * 2.0).fract();
            let s = chute(tau);
            if let Some(p) = ligne_dipole(pied, m, c.rayon_astre, hemi, s) {
                // Le flot a de l'épaisseur au départ, se resserre à l'arrivée
                // — des filets, pas un tube.
                let ecart = vec3(h(i, 5.0) - 0.5, h(i, 6.0) - 0.5, h(i, 7.0) - 0.5) * c.rayon_astre * 0.6 * (1.0 - s);
                let taille = c.rayon_astre * (0.14 + 0.26 * (1.0 - s));
                v.push((p + ecart, taille, chaleur(s), 0.3 + 0.5 * s));
            }
        }
    }
    v
}

/// **Le bord interne brûlant** : la poussière du bord de la cavité, chauffée
/// à blanc par l'étoile toute proche (elle s'y sublime vers 1 500 K), et
/// gonflée — le *puffed-up inner rim* des disques jeunes. Un anneau vif au ras
/// de la cavité, qui tourne à la vitesse de Kepler du bord.
pub fn bord_brulant(c: &DisqueConfig, temps: f32) -> Vec<Boule> {
    const BOULES: u32 = 600;
    if c.bord_brulant <= 0.0 {
        return Vec::new();
    }
    let omega = rotation(c);
    (0..BOULES)
        .map(|j| {
            let i = j as f32 + 20_000.0;
            let r = c.interne * (1.0 + 0.12 * h(i, 1.0));
            let ang = h(i, 2.0) * TAU + omega * temps;
            let z = (h(i, 3.0) - 0.5) * 0.35 * c.interne;
            let p = vec3(ang.cos() * r, z, ang.sin() * r);
            (p, c.interne * (0.06 + 0.06 * h(i, 4.0)), chaleur(0.55 + 0.35 * h(i, 5.0)), c.bord_brulant * (0.2 + 0.2 * h(i, 6.0)))
        })
        .collect()
}

/// **Une pluie de débris** (lot 14, la naine blanche polluée) : chaque
/// débris tombe du bord interne de l'anneau jusqu'à l'étoile sur **sa propre**
/// spirale, partie d'un angle à lui — pas de bras, un tourbillon diffus. Les
/// bras spiraux d'une étoile jeune y faisaient *« une spirale à trois
/// branches »* qui rendait mal (2026-10-26).
pub fn pluie(c: &DisqueConfig, temps: f32) -> Vec<Boule> {
    if c.pluie == 0 || c.rayon_astre <= 0.0 {
        return Vec::new();
    }
    let omega = rotation(c);
    let depart = c.interne;
    let arrivee = c.rayon_astre * 1.05;
    (0..c.pluie)
        .map(|j| {
            let i = j as f32 + 30_000.0;
            let tau = (h(i, 1.0) + temps * omega / TAU * 0.3).fract();
            let s = chute(tau);
            let r = depart * (arrivee / depart).powf(s);
            let ang = h(i, 2.0) * TAU + 1.2 * (depart / r).ln() + omega * temps;
            let z = (h(i, 3.0) - 0.5) * 0.12 * r;
            let p = vec3(ang.cos() * r, z, ang.sin() * r);
            (p, r * 0.035 + c.rayon_astre * 0.14, chaleur(s), 0.35 + 0.5 * s)
        })
        .collect()
}

/// **Des corps qui se désagrègent** en orbite serrée (lot 14 ; WD 1145+017,
/// dont on voit les débris planétaires se défaire en direct) : chacun une
/// tête brillante, et derrière lui, le long de son orbite, une traînée de
/// poussière qui s'élargit et s'éteint — une comète autour d'une étoile morte.
pub fn fragments(c: &DisqueConfig, temps: f32) -> Vec<Boule> {
    const TRAINEE: u32 = 90;
    if c.fragments == 0 || c.rayon_astre <= 0.0 {
        return Vec::new();
    }
    let mut v = Vec::new();
    for k in 0..c.fragments {
        let i = k as f32 * 97.0 + 40_000.0;
        let rf = c.rayon_astre * (2.2 + 1.6 * h(i, 1.0));
        let omega = (c.gm / rf.powi(3)).sqrt();
        let tete = h(i, 2.0) * TAU + omega * temps;
        let incl = (h(i, 3.0) - 0.5) * 0.15;
        let point = |ang: f32, ecart: f32, haut: f32| {
            let r = rf + ecart;
            vec3(ang.cos() * r, (ang - tete).sin() * r * incl + haut, ang.sin() * r)
        };
        v.push((point(tete, 0.0, 0.0), c.rayon_astre * 0.22, vec3(1.0, 0.95, 0.88), 1.0));
        for j in 0..TRAINEE {
            let s = j as f32 / TRAINEE as f32;
            let ij = i + j as f32;
            // Derrière la tête : à rebours du sens de rotation.
            let ang = tete - s * 1.4;
            let largeur = c.rayon_astre * (0.08 + 0.6 * s);
            let p = point(ang, (h(ij, 4.0) - 0.5) * largeur, (h(ij, 5.0) - 0.5) * largeur * 0.5);
            let couleur = vec3(1.0, 0.85, 0.65).lerp(vec3(0.7, 0.55, 0.42), s);
            v.push((p, c.rayon_astre * (0.14 + 0.26 * s), couleur, 0.85 * (1.0 - s).powf(1.2)));
        }
    }
    v
}

/// **Les boules des bras spiraux** au temps `temps` : du tiers interne du
/// disque jusqu'à la surface de l'étoile, en spirale logarithmique, le motif
/// tournant avec elle et la matière coulant dedans, de plus en plus chaude.
pub fn spirales(c: &DisqueConfig, temps: f32) -> Vec<Boule> {
    const PAR_BRAS: u32 = 400;
    let n = c.spirales_accretion;
    if n == 0 || c.rayon_astre <= 0.0 {
        return Vec::new();
    }
    let omega = rotation(c);
    let depart = c.interne + 0.35 * (c.externe - c.interne);
    let arrivee = c.rayon_astre * 1.05;
    let mut v = Vec::with_capacity((n * PAR_BRAS) as usize);
    for k in 0..n {
        for j in 0..PAR_BRAS {
            let i = (k * PAR_BRAS + j) as f32 + 9_000.0;
            let tau = (h(i, 1.0) + temps * omega / TAU * 0.5).fract();
            let s = chute(tau);
            let r = depart * (arrivee / depart).powf(s);
            let ang = k as f32 * TAU / n as f32 + 1.6 * (depart / r).ln() + omega * temps + (h(i, 2.0) - 0.5) * 0.25;
            let z = (h(i, 3.0) - 0.5) * 0.08 * r;
            let p = vec3(ang.cos() * r, z, ang.sin() * r);
            v.push((p, r * 0.03 + c.rayon_astre * 0.12, chaleur(s), 0.28 + 0.45 * s));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Une ligne de champ part de son pied et finit sur l'astre**, en
    /// s'en rapprochant sans cesse, et monte vers le pôle de son hémisphère.
    #[test]
    fn une_ligne_de_champ_va_du_disque_a_l_astre() {
        for (m, hemi) in [(Vec3::Y, 1.0), (Vec3::Y, -1.0), (axe_dipole(0.4, 1.0), 1.0)] {
            let pied = vec3(3.0, 0.0, 0.5);
            let debut = ligne_dipole(pied, m, 1.0, hemi, 0.0).unwrap();
            assert!((debut - pied).length() < 1e-3, "{m} : ne part pas de son pied");
            let fin = ligne_dipole(pied, m, 1.0, hemi, 1.0).unwrap();
            assert!((fin.length() - 1.0).abs() < 1e-3, "{m} : finit à {} de l'astre", fin.length());
            let mut avant = f32::MAX;
            for k in 0..=20 {
                let r = ligne_dipole(pied, m, 1.0, hemi, k as f32 / 20.0).unwrap().length();
                assert!(r <= avant + 1e-4, "{m} : la matière s'éloigne");
                avant = r;
            }
            if m == Vec3::Y {
                assert_eq!(fin.y.signum(), hemi, "aligné : mauvais hémisphère");
            }
        }
        assert!(ligne_dipole(vec3(0.5, 0.0, 0.0), Vec3::Y, 1.0, 1.0, 0.5).is_none(), "pied dans l'astre");
    }

    /// **Dipôle incliné : deux flots, un par hémisphère** — chaque flot
    /// monte vers le pôle qui lui fait face.
    #[test]
    fn un_dipole_incline_donne_deux_flots_opposes() {
        let c = DisqueConfig::protosolaire(0, 3.0, 16.0, 1000.0).avec_entonnoirs(2, 1.0, 0.4);
        let b = entonnoirs(&c, 0.0);
        let haut = b.iter().filter(|x| x.0.y > 0.3).count();
        let bas = b.iter().filter(|x| x.0.y < -0.3).count();
        assert!(haut > 30 && bas > 30, "flots : {haut} en haut, {bas} en bas");
        // Entre l'astre et le bord interne, à l'épaisseur du flot près.
        assert!(b.iter().all(|x| x.0.length() <= 3.0 * 1.15 + 0.55 && x.0.length() >= 1.0 - 0.05));
    }

    /// **Le bord brûlant colle au bord de la cavité** — ni dedans, ni loin —,
    /// et s'éteint à intensité nulle.
    #[test]
    fn le_bord_brulant_colle_a_la_cavite() {
        let c = DisqueConfig::protosolaire(0, 3.0, 16.0, 1000.0).avec_bord_brulant(1.0);
        let b = bord_brulant(&c, 1.0);
        assert!(!b.is_empty());
        for x in &b {
            let r = vec2(x.0.x, x.0.z).length();
            assert!((3.0..=3.0 * 1.12 + 1e-3).contains(&r), "boule à r = {r}");
        }
        assert!(bord_brulant(&DisqueConfig::protosolaire(0, 3.0, 16.0, 1000.0), 1.0).is_empty());
    }

    /// **La pluie tombe de partout, sans bras** : ses débris vont du bord de
    /// l'anneau à l'étoile, et à mi-chemin ils sont répartis tout autour, pas
    /// regroupés en quelques branches.
    #[test]
    fn la_pluie_tombe_de_partout_sans_bras() {
        let c = DisqueConfig::debris_recents(0, 4.0, 10.0, 1000.0).avec_pluie(4000, 1.0);
        let b = pluie(&c, 1.0);
        assert!(b.iter().all(|x| (1.0..=4.0 + 1e-3).contains(&vec2(x.0.x, x.0.z).length())));
        let mut secteurs = [0; 12];
        for x in b.iter().filter(|x| (2.0..3.0).contains(&vec2(x.0.x, x.0.z).length())) {
            let a = x.0.z.atan2(x.0.x).rem_euclid(TAU);
            secteurs[((a / TAU * 12.0) as usize).min(11)] += 1;
        }
        let (min, max) = (secteurs.iter().min().unwrap(), secteurs.iter().max().unwrap());
        // Des bras laisseraient des secteurs presque vides ; le hasard, non.
        assert!(*min * 2 > *max, "regroupée en bras : {secteurs:?}");
    }

    /// **Un fragment traîne sa poussière derrière lui**, à rebours de sa
    /// course, sur son orbite.
    #[test]
    fn un_fragment_traine_sa_poussiere_derriere_lui() {
        let c = DisqueConfig::debris_recents(0, 4.0, 10.0, 1000.0).avec_fragments(1, 1.0);
        let b = fragments(&c, 0.5);
        let tete = b[0].0;
        let rf = vec2(tete.x, tete.z).length();
        assert!((2.2..=3.8).contains(&rf), "orbite à {rf}");
        let az = |p: Vec3| p.z.atan2(p.x);
        let queue = b.last().unwrap().0;
        let recul = (az(tete) - az(queue)).rem_euclid(TAU);
        assert!(recul > 1.0 && recul < 1.6, "la traînée n'est pas derrière : {recul}");
        let plus_tard = fragments(&c, 0.6)[0].0;
        assert!((az(plus_tard) - az(tete)).rem_euclid(TAU) < 1.0, "ne tourne pas dans ce sens");
    }

    /// **Les spirales vont du disque à l'étoile**, et chauffent en chemin.
    #[test]
    fn les_spirales_descendent_vers_l_etoile() {
        let c = DisqueConfig::protosolaire(0, 3.0, 16.0, 1000.0).avec_spirales_accretion(3, 1.0);
        let b = spirales(&c, 2.0);
        assert!(!b.is_empty());
        let (proche, loin) = b.iter().fold((None::<&Boule>, None::<&Boule>), |(p, l), x| {
            let p = if p.is_none_or(|q| x.0.length() < q.0.length()) { Some(x) } else { p };
            let l = if l.is_none_or(|q| x.0.length() > q.0.length()) { Some(x) } else { l };
            (p, l)
        });
        let (p, l) = (proche.unwrap(), loin.unwrap());
        assert!(p.0.length() < 1.5 && l.0.length() > 5.0, "de {} à {}", l.0.length(), p.0.length());
        assert!(p.2.z > l.2.z, "pas plus chaud près de l'étoile");
    }
}
