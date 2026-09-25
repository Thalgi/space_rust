//! **Le champ magnétique d'une étoile** (chantier astres, sous-lot 3.1 —
//! `docs/conception/astres_v2.md` §6 bis, et `astres_v2_champ_partage.md`).
//!
//! ## Pourquoi il existe
//!
//! Jusqu'ici, **rien ne disait où le champ était fort**. Les taches se
//! posaient donc au hasard sur la sphère (`eruptions.rs`, `dir_aleatoire`) et
//! les arches ailleurs : deux systèmes sans rapport, d'où des taches qui ont
//! l'air d'**autocollants** posés sur la surface et des boucles qui ne
//! partent de nulle part. Ce n'était pas un défaut de forme — c'était une
//! absence de source commune.
//!
//! Ce module est cette source. Il ne dessine rien et ne décide de rien : il
//! répond à une seule question, « quelle est l'intensité du champ dans cette
//! direction ». Les sous-lots suivants s'en servent — les taches naissent sur
//! ses maxima (3.2), les pieds d'une arche sur deux polarités opposées (3.3),
//! et le shader l'utilisera pour assombrir la surface (3.4).
//!
//! ## Ce qu'il modélise, et ce qu'il ne modélise pas
//!
//! Des **paires** de pôles de polarités opposées, posées côte à côte : c'est
//! la forme minimale qui produise une **région active bipolaire**, avec sa
//! ligne neutre entre les deux — la structure dont découlent, dans la
//! réalité, les groupes de taches et les arcades de boucles
//! (`astres_v2_recherche_visuelle.md`).
//!
//! Ce n'est pas une simulation : pas de lignes de champ intégrées, pas de
//! dipôle exact, pas d'évolution. Une somme de noyaux, bornée et sans
//! singularité, qu'on peut interroger des milliers de fois par frame.

use crate::planete::terrain::Rng;
use macroquad::prelude::*;

/// Nombre de **paires** de pôles par étoile.
///
/// Trois régions actives : assez pour que la surface ne se résume pas à deux
/// taches figées, assez peu pour qu'elles restent distinctes et que le champ
/// garde des creux entre elles. ⚠️ Ce chiffre est un **réglage esthétique**,
/// il ne se teste pas — il se juge à l'œil au-delà d'environ 10 px de rayon
/// stellaire (`astres_v2.md` §6 bis, risque 1).
pub(super) const NB_PAIRES: usize = 3;

/// Écartement angulaire des deux pôles d'une paire, en radians.
///
/// C'est lui qui fixe la **taille d'une région active** : trop serré, les
/// deux polarités s'annulent et il ne reste rien ; trop large, la paire
/// cesse de se lire comme un groupe et donne deux taches sans rapport.
const ECART_PAIRE: f32 = 0.28;

/// Portée angulaire d'un pôle : au-delà, sa contribution est négligeable.
const PORTEE: f32 = 0.42;

/// Un pôle magnétique, posé sur la sphère.
#[derive(Clone, Copy, Debug)]
pub(super) struct Pole {
    /// Direction unitaire depuis le centre de l'étoile.
    pub dir: Vec3,
    /// **Force signée** : le signe est la polarité. Deux pôles de signes
    /// opposés forment une région active, et c'est entre eux que les arches
    /// s'ancreront (sous-lot 3.3).
    pub force: f32,
}

/// Les pôles d'une étoile, **dérivés de sa graine**.
///
/// ⚠️ **Dérivés, jamais tirés.** Un `gen_range` de plus dans `Soleil::new`
/// consommerait le RNG **global** de macroquad, décalant tout ce qui est tiré
/// ensuite : à graine égale, le système entier changerait — planètes
/// comprises. Le dépôt a déjà rencontré ce piège (`genese/mod.rs`, la
/// multiplicité tirée du seed « pour laisser la séquence intacte »). D'où un
/// générateur déterministe local, semé par la graine de l'étoile.
pub(super) fn poles(graine: f32) -> Vec<Pole> {
    let mut rng = Rng::new(graine.to_bits() as u64);
    let mut out = Vec::with_capacity(NB_PAIRES * 2);
    for _ in 0..NB_PAIRES {
        let centre = direction(&mut rng);
        // Une direction tangente quelconque, pour écarter les deux polarités
        // de part et d'autre du centre de la région active.
        let axe = tangente(centre, &mut rng);
        let force = 0.6 + 0.4 * rng.f32();
        for signe in [1.0_f32, -1.0] {
            let d = (centre + axe * (signe * ECART_PAIRE)).normalize_or(centre);
            out.push(Pole { dir: d, force: force * signe });
        }
    }
    out
}

/// **Intensité signée du champ** dans la direction `d` (unitaire).
///
/// Somme de noyaux en cloche, un par pôle : proche de sa force au pôle,
/// nulle loin de tous. Le **signe** dit la polarité dominante, ce dont les
/// pieds d'arche auront besoin ; sa **valeur absolue** dit où la convection
/// est bloquée, donc où une tache peut naître.
pub(super) fn champ(poles: &[Pole], d: Vec3) -> f32 {
    poles
        .iter()
        .map(|p| {
            let ang = p.dir.dot(d).clamp(-1.0, 1.0).acos();
            let t = ang / PORTEE;
            p.force * (-t * t).exp()
        })
        .sum()
}

/// Direction unitaire uniformément répartie sur la sphère.
///
/// ⚠️ La latitude passe par `asin(2u-1)` et non par un angle uniforme : sans
/// ça, les tirages s'accumulent aux pôles géographiques, et toutes les
/// régions actives d'une étoile naîtraient en haut et en bas.
fn direction(rng: &mut Rng) -> Vec3 {
    let lon = rng.f32() * std::f32::consts::TAU;
    let lat = (2.0 * rng.f32() - 1.0).clamp(-1.0, 1.0).asin();
    let (sl, cl) = lat.sin_cos();
    vec3(cl * lon.cos(), sl, cl * lon.sin())
}

/// Une direction unitaire **perpendiculaire** à `d`, orientée au hasard
/// autour d'elle.
fn tangente(d: Vec3, rng: &mut Rng) -> Vec3 {
    // Deux références pour éviter le produit vectoriel dégénéré quand `d`
    // frôle l'axe choisi — le même piège que `camera::base_camera`.
    let ref_ = if d.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = d.cross(ref_).normalize_or(Vec3::X);
    let v = d.cross(u).normalize_or(Vec3::Z);
    let a = rng.f32() * std::f32::consts::TAU;
    (u * a.cos() + v * a.sin()).normalize_or(u)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Même graine, même champ.** C'est la promesse du dépôt : un système
    /// se reconstruit à l'identique (`bucketlist_globale.md` §1). Une étoile
    /// dont les régions actives changeraient à chaque reconstruction
    /// casserait aussi le hot-reload des shaders, qui rebâtit le système.
    #[test]
    fn les_poles_ne_dependent_que_de_la_graine() {
        let a = poles(123.456);
        let b = poles(123.456);
        assert_eq!(a.len(), b.len());
        for (p, q) in a.iter().zip(b.iter()) {
            assert_eq!(p.dir, q.dir);
            assert_eq!(p.force, q.force);
        }
        // Et deux étoiles différentes ne portent pas la même carte.
        let autre = poles(987.2);
        assert!(
            a.iter().zip(autre.iter()).any(|(p, q)| p.dir != q.dir),
            "deux graines donnent les mêmes pôles : les étoiles resteraient jumelles"
        );
    }

    /// **Les pôles vont par paires de polarités opposées, et proches.**
    ///
    /// C'est toute la structure : une région active est bipolaire, et c'est
    /// entre ses deux polarités que passe la ligne neutre le long de laquelle
    /// s'alignent les arches (sous-lot 3.3). Deux pôles de même signe, ou
    /// trop éloignés, ne formeraient pas un groupe.
    #[test]
    fn chaque_paire_est_bipolaire_et_serree() {
        let p = poles(42.0);
        assert_eq!(p.len(), NB_PAIRES * 2, "il manque des pôles");
        for paire in p.chunks(2) {
            assert!(
                paire[0].force * paire[1].force < 0.0,
                "paire de même polarité : {} et {}",
                paire[0].force,
                paire[1].force
            );
            let ecart = paire[0].dir.dot(paire[1].dir).clamp(-1.0, 1.0).acos();
            assert!(
                ecart < 3.0 * ECART_PAIRE,
                "polarités trop écartées ({ecart} rad) : ce n'est plus un groupe"
            );
            assert!(ecart > 0.1, "polarités confondues ({ecart} rad) : elles s'annulent");
        }
        // Toutes les directions sont unitaires : le champ lit un `dot` dessus.
        for pole in &p {
            assert!((pole.dir.length() - 1.0).abs() < 1e-4, "direction non unitaire");
        }
    }

    /// **Le champ est fort près d'un pôle, nul loin de tous, et son signe dit
    /// la polarité.** C'est le contrat dont les sous-lots suivants dépendent :
    /// 3.2 cherche les maxima, 3.3 cherche deux signes opposés.
    #[test]
    fn le_champ_marque_les_poles_et_s_efface_entre_eux() {
        let p = poles(7.0);
        for pole in &p {
            let au_pole = champ(&p, pole.dir);
            assert!(
                au_pole.signum() == pole.force.signum(),
                "au pôle, le champ ({au_pole}) contredit la polarité ({})",
                pole.force
            );
            // À l'opposé exact de ce pôle, sa contribution est éteinte.
            let oppose = champ(&p, -pole.dir).abs();
            assert!(
                oppose < au_pole.abs(),
                "le champ est aussi fort à l'opposé ({oppose}) qu'au pôle ({})",
                au_pole.abs()
            );
        }
    }

    /// **Le champ reste borné.** Il alimentera un assombrissement et un
    /// placement de taches : une valeur qui s'emballerait donnerait une
    /// étoile noire ou une tache géante. La somme de `NB_PAIRES × 2` noyaux
    /// de force ≤ 1 ne peut pas dépasser leur compte.
    #[test]
    fn le_champ_ne_s_emballe_pas() {
        let p = poles(3.0);
        let plafond = (NB_PAIRES * 2) as f32;
        for graine in 0..200 {
            let mut rng = Rng::new(graine);
            let d = direction(&mut rng);
            let v = champ(&p, d);
            assert!(v.is_finite(), "champ non fini en {d:?}");
            assert!(v.abs() <= plafond, "champ {v} au-delà de {plafond}");
        }
    }
}
