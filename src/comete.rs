//! La **queue** d'une comète (lot 9.2).
//!
//! ⚠️ **Tout ce qui décide est ici, et tout ce qui décide est pur.** Le module ne
//! dessine rien : il rend la liste de points d'un ruban. C'est ce qui le rend
//! testable — `Planete::new` charge un material, donc exige le contexte
//! graphique, et une queue construite dans la méthode de dessin n'aurait aucune
//! garde.
//!
//! **Une seule queue, de poussière**, décidé par défaut au §8 bis de la
//! conception. La queue d'ions, droite et bleue, reste un raffinement.

use crate::impostor::PointRuban;
use macroquad::prelude::*;

/// **Distance à laquelle la queue s'éteint**, en unités de l'échelle du système.
///
/// Dans le système solaire, la sublimation de la glace d'eau devient sensible
/// vers 3 UA — c'est la « ligne des neiges », et c'est déjà l'échelle sur
/// laquelle la genèse place ses ceintures. Au-delà, une comète est un caillou
/// noir ; en deçà, elle se met à fumer.
pub const SEUIL_ACTIVITE: f32 = 3.0;

/// **Longueur de la queue au maximum d'activité**, en unités monde.
///
/// ⚠️ Démesurée, et assumée. Une vraie queue cométaire s'étend sur des dizaines
/// de millions de kilomètres — bien plus que le diamètre du noyau multiplié par
/// mille. À l'échelle du jeu, la rendre proportionnelle au noyau la rendrait
/// invisible ; elle est donc proportionnelle à la **distance à l'étoile**, ce qui
/// est à la fois plus juste et plus lisible.
pub const LONGUEUR_MAX: f32 = 0.55;

/// Nombre de segments du ruban. Assez pour que la courbure se lise, assez peu
/// pour qu'une comète reste un décor : trois comètes par système au plus.
const SEGMENTS: usize = 14;

/// **Part de traînée dans la direction de la queue.**
///
/// ⚠️ **Une queue de poussière n'est pas droite.** Les grains libérés par le
/// noyau gardent leur propre orbite et prennent du retard : la queue s'incurve
/// entre l'anti-solaire et la trajectoire. C'est ce qui la distingue de la queue
/// d'**ions**, celle-là rigoureusement anti-solaire, et c'est la différence qui
/// se voit en premier sur une photographie.
const COURBURE: f32 = 0.45;

/// **Activité d'une comète**, de 0 (inerte) à 1 (au plus près).
///
/// ⚠️ C'est elle qui fait vivre la comète : la queue **naît à l'approche et
/// s'efface au loin**, décision n° 1 du §8 bis — et c'est physiquement juste.
/// Une queue permanente en ferait un décor.
///
/// La montée est **quadratique** : la sublimation s'emballe près de l'étoile, et
/// une rampe linéaire donnerait une queue déjà bien visible à mi-chemin, là où
/// une vraie comète ne fume pas encore.
pub fn activite(dist: f32, seuil: f32) -> f32 {
    if seuil <= 0.0 || dist >= seuil || dist < 0.0 {
        return 0.0;
    }
    let t = 1.0 - dist / seuil;
    t * t
}

/// **La distance à laquelle une comète s'allume**, dérivée de **sa propre
/// orbite**.
///
/// ⚠️ Pas un seuil absolu, et c'est le point. Dans le réel, la glace sublime vers
/// 3 UA quelle que soit la comète ; mais les systèmes du jeu sont **mis à
/// l'échelle** de leur étoile (`genese::echelle_systeme`), si bien qu'un seuil en
/// UA allumerait toutes les comètes d'une naine en permanence et n'allumerait
/// jamais celles d'une supergéante.
///
/// Trois fois le périhélie : la comète s'allume donc sur le dernier tiers de sa
/// descente, quelle que soit l'échelle — et le plafond à l'aphélie évite qu'une
/// orbite presque ronde fume en permanence.
pub fn seuil_activite(demi_grand_axe: f32, excentricite: f32) -> f32 {
    let e = excentricite.clamp(0.0, 0.999);
    let perihelie = demi_grand_axe.max(0.0) * (1.0 - e);
    let aphelie = demi_grand_axe.max(0.0) * (1.0 + e);
    (SEUIL_ACTIVITE * perihelie).min(aphelie * 0.6)
}

/// **La queue d'une comète**, prête pour `impostor::push_ruban`.
///
/// Vide quand l'activité est nulle : loin de son étoile, une comète n'a pas de
/// queue à dessiner, et ce n'est pas un cas limite mais son état le plus
/// fréquent.
///
/// ⚠️ **Elle part à l'opposé de l'ÉTOILE, pas de la vitesse.** C'est l'erreur
/// classique, et elle se voit immédiatement : une queue qui traîne derrière la
/// comète comme un sillage de bateau est fausse. Le vent stellaire chasse la
/// poussière radialement ; la trajectoire ne fait que l'incurver.
pub fn queue(
    noyau: Vec3,
    etoile: Vec3,
    vitesse: Vec3,
    rayon_noyau: f32,
    seuil: f32,
) -> Vec<PointRuban> {
    let vers_etoile = noyau - etoile;
    let dist = vers_etoile.length();
    let a = activite(dist, seuil);
    if a <= 0.0 || dist < 1e-4 {
        return Vec::new();
    }
    let anti_solaire = vers_etoile / dist;
    // Sans vitesse exploitable, la queue reste droite : mieux vaut une queue
    // anti-solaire qu'une direction indéfinie.
    let trainee = if vitesse.length_squared() > 1e-8 { -vitesse.normalize() } else { anti_solaire };

    let longueur = LONGUEUR_MAX * a * dist;
    let mut pts = Vec::with_capacity(SEGMENTS + 1);
    for k in 0..=SEGMENTS {
        let t = k as f32 / SEGMENTS as f32;
        // ⚠️ La courbure **croît** le long de la queue : près du noyau la
        // poussière vient d'être libérée et suit encore l'anti-solaire ; loin,
        // elle a pris son retard. Une courbure constante donnerait une queue
        // droite, simplement inclinée.
        let dir = (anti_solaire + trainee * (COURBURE * t * t)).normalize();
        let pos = noyau + dir * (longueur * t);
        // S'évase en s'éloignant — la poussière se disperse — et s'éteint.
        let demi_largeur = rayon_noyau * (0.9 + 5.0 * t);
        let alpha = a * (1.0 - t).powf(1.6) * 0.5;
        pts.push((pos, demi_largeur, Color::new(0.80, 0.84, 0.95, alpha)));
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **La queue naît à l'approche et s'efface au loin.**
    ///
    /// C'est la décision n° 1 du §8 bis, et c'est ce qui fait vivre la comète :
    /// une queue permanente en ferait un décor.
    #[test]
    fn l_activite_nait_a_l_approche_et_s_efface_au_loin() {
        let ech = 1.0;
        let seuil = SEUIL_ACTIVITE * ech * crate::etoile::UA;
        assert_eq!(activite(seuil, seuil), 0.0, "la queue vit encore au seuil");
        assert_eq!(activite(seuil * 2.0, seuil), 0.0, "la queue vit au-delà du seuil");
        assert!(activite(seuil * 0.05, seuil) > 0.8, "la queue ne s'allume pas au plus près");
        // Monotone : s'éloigner ne doit jamais raviver la queue.
        let mut precedent = f32::INFINITY;
        for k in 0..=200 {
            let d = seuil * k as f32 / 200.0;
            let a = activite(d, seuil);
            assert!(a <= precedent + 1e-6, "l'activité remonte à {d}");
            assert!((0.0..=1.0).contains(&a), "activité {a} hors bornes");
            precedent = a;
        }
        // ⚠️ **Montée franche** : à mi-chemin du seuil, la comète ne doit pas
        // déjà fumer à moitié — une vraie comète s'allume tard. Une rampe
        // linéaire donnerait 0,5 ici.
        assert!(activite(seuil * 0.5, seuil) < 0.3, "la montée est trop molle");
        // Et le seuil suit l'échelle du système : une naine resserre tout.
        
    }

    /// **Le seuil vient de l'orbite de la comète, pas d'une distance absolue.**
    ///
    /// ⚠️ Dans le réel, la glace sublime vers 3 UA quelle que soit la comète.
    /// Mais les systèmes du jeu sont **mis à l'échelle** de leur étoile : un
    /// seuil en UA allumerait en permanence toutes les comètes d'une naine, et
    /// jamais celles d'une supergéante. Le seuil suit donc le périhélie.
    #[test]
    fn le_seuil_suit_l_orbite_et_non_une_distance_absolue() {
        // Deux systèmes à des échelles très différentes, même forme d'orbite :
        // la comète doit s'allumer au même **endroit de sa descente**.
        let part_allumee = |a: f32| {
            let e = 0.9;
            let s = seuil_activite(a, e);
            s / (a * (1.0 + e)) // part de l'aphélie où la queue existe
        };
        let petite = part_allumee(4.0);
        let grande = part_allumee(400.0);
        assert!(
            (petite - grande).abs() < 1e-4,
            "l'échelle change la part allumée : {petite} contre {grande}"
        );
        // Une orbite **ronde** ne doit pas fumer en permanence : le plafond à
        // l'aphélie l'en empêche.
        let ronde = seuil_activite(10.0, 0.0);
        assert!(ronde < 10.0, "une orbite circulaire fume partout : seuil {ronde}");
        // Et une orbite très excentrique s'allume tard.
        let s = seuil_activite(40.0, 0.96);
        let aphelie = 40.0 * 1.96;
        assert!(s < aphelie * 0.2, "elle s'allume trop tôt : {s} pour un aphélie de {aphelie}");
        assert!(s > 40.0 * 0.04, "elle ne s'allume jamais : {s}");
    }

    /// **Halley s'allume là où elle doit**, et elle est rétrograde.
    ///
    /// Le preset solaire la porte pour qu'on ait une comète sous la main sans
    /// régénérer. ⚠️ `construire_preset_solaire` tire des aléas, donc exige le
    /// contexte graphique et ne se teste pas : ce sont ses **éléments** qui sont
    /// nommés et éprouvés ici.
    #[test]
    fn halley_s_allume_entre_mars_et_la_terre_et_tourne_a_l_envers() {
        let (a, e, incl) = crate::genese::presets::HALLEY;
        let perihelie = a * (1.0 - e);
        // Son périhélie tombe entre Vénus (0,72) et Mercure (0,39) : elle plonge
        // donc au cœur du système, et c'est ce qui la rend spectaculaire.
        assert!(
            (0.39..0.72).contains(&perihelie),
            "périhélie à {perihelie} UA : Halley ne plonge plus entre Mercure et Vénus"
        );
        // ⚠️ **Rétrograde.** Au-delà de 90°, le vecteur perpendiculaire bascule
        // et le sens de parcours s'inverse : sa trace se lit à l'envers de celle
        // des planètes. C'est exact, et c'est le détail qui se voit.
        assert!(incl > 90.0, "inclinaison {incl}° : Halley ne tourne plus à l'envers");
        assert!(incl < 180.0, "inclinaison {incl}° : hors de toute orbite");

        // Et sa queue s'allume à la bonne distance. Dans le réel elle devient
        // visible vers 1 à 2 UA ; le seuil du jeu vaut trois périhélies.
        let seuil = seuil_activite(a, e);
        assert!(
            (1.0..2.5).contains(&seuil),
            "seuil à {seuil} UA : Halley s'allume trop tôt ou trop tard"
        );
        // Éteinte au-delà de Jupiter, franche au périhélie.
        assert_eq!(activite(5.2, seuil), 0.0, "elle fume encore à l'orbite de Jupiter");
        assert!(activite(perihelie, seuil) > 0.3, "aucune queue à son périhélie");
    }

    /// **La queue part à l'opposé de l'étoile, jamais de la vitesse.**
    ///
    /// ⚠️ C'est **l'erreur classique**, et elle se voit immédiatement : une queue
    /// qui traîne derrière la comète comme un sillage de bateau est fausse. Le
    /// vent stellaire chasse la poussière radialement.
    ///
    /// Le test prend une comète dont la vitesse est **perpendiculaire** au rayon
    /// — le cas qui sépare les deux hypothèses — et vérifie que la queue est du
    /// côté opposé à l'étoile.
    #[test]
    fn la_queue_fuit_l_etoile_et_non_la_trajectoire() {
        let etoile = Vec3::ZERO;
        let noyau = vec3(10.0, 0.0, 0.0);
        let vitesse = vec3(0.0, 0.0, 4.0); // perpendiculaire au rayon
        let pts = queue(noyau, etoile, vitesse, 0.1, 30.0);
        assert!(!pts.is_empty(), "aucune queue près de l'étoile");
        let anti_solaire = (noyau - etoile).normalize();
        for (p, _, _) in &pts {
            let d = *p - noyau;
            if d.length() < 1e-5 {
                continue;
            }
            assert!(
                d.normalize().dot(anti_solaire) > 0.5,
                "un point de queue part vers l'étoile ou en travers : {d:?}"
            );
        }
        // Le bout s'éloigne franchement de l'étoile.
        let bout = pts.last().unwrap().0;
        assert!(
            (bout - etoile).length() > (noyau - etoile).length(),
            "le bout de la queue est plus proche de l'étoile que le noyau"
        );
    }

    /// **Elle s'incurve** : c'est une queue de poussière, pas d'ions.
    ///
    /// Les grains gardent leur orbite et prennent du retard. Une queue
    /// rigoureusement droite serait celle des **ions** — un autre objet, écarté
    /// par défaut au §8 bis.
    #[test]
    fn la_queue_de_poussiere_s_incurve_vers_la_trainee() {
        let etoile = Vec3::ZERO;
        let noyau = vec3(10.0, 0.0, 0.0);
        let vitesse = vec3(0.0, 0.0, 4.0);
        let pts = queue(noyau, etoile, vitesse, 0.1, 30.0);
        let dir = |i: usize| (pts[i].0 - noyau).normalize();
        let proche = dir(2);
        let loin = dir(pts.len() - 1);
        assert!(
            proche.dot(loin) < 0.999,
            "la queue est droite : ce sont des ions, pas de la poussière"
        );
        // Et elle s'incurve **du côté de la traînée**, pas au hasard.
        let trainee = -vitesse.normalize();
        assert!(
            loin.dot(trainee) > proche.dot(trainee),
            "la courbure ne va pas vers la traînée"
        );
    }

    /// **Au loin, aucune queue** — pas une queue de longueur nulle, rien du tout.
    ///
    /// ⚠️ Un ruban vide ne coûte aucun sommet. C'est l'état le plus fréquent
    /// d'une comète : l'essentiel de son orbite se passe au froid.
    #[test]
    fn au_loin_la_queue_n_existe_pas() {
        let etoile = Vec3::ZERO;
        let loin = vec3(40.0, 0.0, 0.0);
        assert!(queue(loin, etoile, vec3(0.0, 0.0, 1.0), 0.1, 30.0).is_empty());
        // Et une comète posée sur son étoile ne produit pas de direction folle.
        assert!(queue(etoile, etoile, Vec3::ZERO, 0.1, 30.0).is_empty());
    }

    /// **Elle s'évase et s'éteint vers son bout.** Une queue d'épaisseur ou
    /// d'opacité constante se lirait comme une barre.
    #[test]
    fn la_queue_s_evase_et_s_eteint() {
        let pts = queue(vec3(10.0, 0.0, 0.0), Vec3::ZERO, vec3(0.0, 0.0, 4.0), 0.1, 30.0);
        let (_, l0, c0) = pts[0];
        let (_, l1, c1) = *pts.last().unwrap();
        assert!(l1 > l0 * 3.0, "la queue ne s'évase pas : {l0} -> {l1}");
        assert!(c1.a < c0.a * 0.2, "la queue ne s'éteint pas : {} -> {}", c0.a, c1.a);
        assert!(c1.a >= 0.0, "opacité négative au bout");
        // Et l'opacité décroît sans remonter.
        let mut precedent = f32::INFINITY;
        for (_, _, c) in &pts {
            assert!(c.a <= precedent + 1e-6, "l'opacité remonte le long de la queue");
            precedent = c.a;
        }
    }
}
