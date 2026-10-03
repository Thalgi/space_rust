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
pub const LONGUEUR_MAX: f32 = 1.6;

/// **Longueur minimale, en rayons de noyau.**
///
/// ⚠️ Sans elle, la queue était **invisible** — signalé à l'écran. Sa longueur
/// est proportionnelle à la distance à l'étoile, ce qui est juste ; mais le
/// noyau, lui, a une taille **fixe et exagérée** (un vrai noyau de 11 km serait
/// sub-pixel). Les deux échelles n'ont donc aucun rapport : dans la galerie des
/// comètes, où l'orbite tient en 3,5 unités, la queue mesurait 0,17 unité pour
/// un noyau de 0,18 de diamètre — plus courte que le caillou qui l'émet.
///
/// Le plancher est en **rayons de noyau** parce que c'est à lui qu'on la compare
/// à l'œil. Vingt-cinq : assez pour se lire comme une traînée, pas assez pour
/// dominer un système.
pub const LONGUEUR_MIN_NOYAUX: f32 = 25.0;

/// **Demi-largeur de la queue, en fraction de SA PROPRE LONGUEUR** : à sa
/// naissance au ras du noyau, puis à son bout.
///
/// ⚠️ **C'est le défaut qui la rendait invisible, et c'est le même que celui de
/// [`LONGUEUR_MIN_NOYAUX`] : deux échelles sans rapport l'une avec l'autre.** La
/// largeur suivait le **rayon du noyau** quand la longueur suit la **distance à
/// l'étoile**. Halley au périhélie : 20 unités de long pour 0,15 de large — un
/// fil de 1 pour 130, sub-pixel dès qu'on cadre le système. Il y avait bien une
/// queue, elle était plus fine qu'un trait.
///
/// Une queue de comète est un **cône**, et son ouverture ne sait rien du caillou
/// qui l'émet : c'est le rapport longueur/largeur qui se lit à l'œil.
pub const LARGEUR_BASE: f32 = 0.04;
pub const LARGEUR_BOUT: f32 = 0.22;

/// **Éclat de la queue**, au plus fort de son activité.
///
/// ⚠️ **Franchement exagéré, et demandé tel quel** : *« au pire on la rend
/// extrêmement visible pour le moment au point que ce n'est pas naturel »*. Une
/// vraie queue est un voile qu'on ne photographie qu'en pose longue ; à 0,5 elle
/// disparaissait sur fond d'étoiles. À baisser quand le reste du rendu cométaire
/// sera réglé — c'est une dette assumée, pas un choix physique.
pub const ECLAT: f32 = 1.0;

/// Nombre de segments du ruban. Assez pour que la courbure se lise, assez peu
/// pour qu'une comète reste un décor : trois comètes par système au plus.
const SEGMENTS: usize = 14;

/// **Part de traînée dans la direction de la queue**, et **où elle arrive**.
///
/// ⚠️ **Une queue de poussière n'est pas droite.** Les grains libérés par le
/// noyau gardent leur propre orbite et prennent du retard : la queue s'incurve
/// entre l'anti-solaire et la trajectoire. C'est ce qui la distingue de la queue
/// d'**ions**, celle-là rigoureusement anti-solaire, et c'est la différence qui
/// se voit en premier sur une photographie.
///
/// ⚠️ **La courbure était réelle et invisible** — signalé à l'écran : *« on
/// dirait que la queue est poussée toujours vers l'extérieur du système »*. À
/// 0,45 sur un `t²`, la déviation valait **5,6° à mi-queue** et n'atteignait ses
/// 24° qu'au tout dernier point, là où l'opacité est déjà tombée à rien. Toute
/// la courbure vivait **dans la partie transparente** : la queue se lisait
/// rigoureusement droite, donc comme une queue d'ions.
///
/// L'exposant est ce qui déplace la courbure vers le **corps éclairé** de la
/// queue : plus il est bas, plus tôt les grains décrochent. À 1,3 la déviation
/// vaut 14° là où la queue a encore la moitié de son opacité, et 37° au bout.
/// Elle reste **progressive**, ce qui est le point physique : près du noyau la
/// poussière vient d'être libérée et suit encore l'anti-solaire.
const COURBURE: f32 = 0.75;
const EXPOSANT_COURBURE: f32 = 1.3;

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
    let depuis_etoile = noyau - etoile;
    let dist = depuis_etoile.length();
    let a = activite(dist, seuil);
    if a <= 0.0 || dist < 1e-4 {
        return Vec::new();
    }
    let anti_solaire = depuis_etoile / dist;
    // Sans vitesse exploitable, la queue reste droite : mieux vaut une queue
    // anti-solaire qu'une direction indéfinie.
    let trainee = if vitesse.length_squared() > 1e-8 { -vitesse.normalize() } else { anti_solaire };

    // ⚠️ **Deux échelles sans rapport l'une avec l'autre.** La longueur physique
    // suit la distance à l'étoile ; le noyau, lui, a une taille fixe et
    // exagérée. Le plancher en rayons de noyau est ce qui rend la queue lisible
    // quelle que soit l'échelle — sans lui, elle était plus courte que le
    // caillou qui l'émet dans la galerie des comètes.
    let longueur = (LONGUEUR_MAX * dist).max(LONGUEUR_MIN_NOYAUX * rayon_noyau) * a;
    let mut pts = Vec::with_capacity(SEGMENTS + 1);
    for k in 0..=SEGMENTS {
        let t = k as f32 / SEGMENTS as f32;
        // ⚠️ La courbure **croît** le long de la queue : près du noyau la
        // poussière vient d'être libérée et suit encore l'anti-solaire ; loin,
        // elle a pris son retard. Une courbure constante donnerait une queue
        // droite, simplement inclinée.
        let dir = (anti_solaire + trainee * (COURBURE * t.powf(EXPOSANT_COURBURE))).normalize();
        let pos = noyau + dir * (longueur * t);
        // ⚠️ **S'évase en fraction de SA LONGUEUR**, jamais du rayon du noyau :
        // voir [`LARGEUR_BASE`]. Le plancher au noyau ne sert qu'au cas d'une
        // queue naissante, pour qu'elle ne soit pas plus fine que ce qui la
        // crache.
        let demi_largeur = (longueur * (LARGEUR_BASE + (LARGEUR_BOUT - LARGEUR_BASE) * t))
            .max(rayon_noyau * 0.9);
        let alpha = a * (1.0 - t).powf(1.1) * ECLAT;
        pts.push((pos, demi_largeur, Color::new(0.86, 0.90, 1.0, alpha)));
    }
    pts
}

/// Résolution du noyau : parallèles × méridiens. Un patatoïde n'a pas besoin de
/// finesse — il en a même besoin du contraire, les facettes se lisent comme des
/// plans de fracture.
const PARALLELES: usize = 10;
const MERIDIENS: usize = 16;

/// **Un sommet du noyau** : sa position (rayon unité), sa normale, sa couleur.
pub type SommetNoyau = (Vec3, Vec3, Color);

/// **Le maillage d'un noyau cométaire**, en rayon unité.
///
/// ⚠️ **Pas une bille.** Un noyau de comète est un **patatoïde** : trop petit
/// pour que sa gravité l'arrondisse, et creusé par la sublimation qui emporte la
/// glace par plaques. C'était signalé à l'écran — « pour le moment c'est une
/// bille ».
///
/// Déformé par une somme de sinus sur la direction, ce qui donne des bosses
/// franches sans bruit de Perlin ni table : **dérivé de la graine**, donc deux
/// comètes diffèrent, et la même garde deux fois rend le même caillou.
///
/// **Glace et roche** se mélangent par plaques, sur une seconde somme sans
/// rapport avec la première : la texture ne suit donc pas le relief, ce qui
/// serait le défaut d'un bruit unique — un patatoïde dont les creux seraient
/// systématiquement sombres se lirait comme une balle peinte.
pub fn noyau(graine: f32) -> (Vec<SommetNoyau>, Vec<u16>) {
    /// Roche : sombre, sèche, légèrement rousse. Un noyau est plus noir que du
    /// charbon — albédo ~0,04.
    const ROCHE: Vec3 = Vec3::new(0.16, 0.13, 0.11);
    /// Glace fraîche, mise à nu par la sublimation : bleutée et bien plus claire.
    const GLACE: Vec3 = Vec3::new(0.62, 0.70, 0.78);

    let g = graine.fract().abs() * 17.3;
    // Relief : trois harmoniques, de la bosse large au creux fin.
    let relief = |d: Vec3| {
        let a = (d.x * 2.1 + g).sin() * (d.y * 1.7 - g * 0.7).cos();
        let b = (d.y * 3.9 - g * 1.3).sin() * (d.z * 3.1 + g).cos();
        let c = (d.z * 6.7 + g * 2.1).sin() * (d.x * 5.3 - g).cos();
        0.78 + 0.16 * a + 0.09 * b + 0.05 * c
    };
    // Plaques de glace : phases **sans rapport** avec celles du relief.
    let glace = |d: Vec3| {
        let v = (d.x * 4.3 - g * 2.7).sin() * (d.z * 3.7 + g * 1.9).cos()
            + 0.6 * (d.y * 7.1 + g * 0.4).sin();
        ((v * 0.9 + 0.35).clamp(0.0, 1.0)).powf(1.4)
    };

    let mut sommets = Vec::with_capacity((PARALLELES + 1) * (MERIDIENS + 1));
    for i in 0..=PARALLELES {
        let phi = std::f32::consts::PI * i as f32 / PARALLELES as f32;
        let (sp, cp) = phi.sin_cos();
        for j in 0..=MERIDIENS {
            let theta = std::f32::consts::TAU * j as f32 / MERIDIENS as f32;
            let (st, ct) = theta.sin_cos();
            let dir = vec3(sp * ct, cp, sp * st);
            let r = relief(dir);
            let t = glace(dir);
            let c = ROCHE.lerp(GLACE, t);
            // ⚠️ Normale = la direction. Approximation assumée : le relief est
            // doux et le noyau fait quelques pixels. Une vraie normale demanderait
            // de dériver la somme de sinus, pour un gain que personne ne verra.
            sommets.push((dir * r, dir, Color::new(c.x, c.y, c.z, 1.0)));
        }
    }
    let mut indices = Vec::with_capacity(PARALLELES * MERIDIENS * 6);
    let large = MERIDIENS + 1;
    for i in 0..PARALLELES {
        for j in 0..MERIDIENS {
            let a = (i * large + j) as u16;
            let b = (i * large + j + 1) as u16;
            let c = ((i + 1) * large + j) as u16;
            let d = ((i + 1) * large + j + 1) as u16;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    (sommets, indices)
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

    /// **Le noyau est un patatoïde, pas une bille.**
    ///
    /// ⚠️ Signalé à l'écran. Un noyau de comète est trop petit pour que sa
    /// gravité l'arrondisse, et la sublimation le creuse par plaques. Le test
    /// mesure **l'écart des rayons** : une sphère parfaite le rendrait nul, et
    /// c'est exactement l'état qu'on corrige.
    #[test]
    fn le_noyau_est_un_patatoide_et_non_une_bille() {
        let (s, idx) = noyau(0.37);
        assert!(!s.is_empty() && !idx.is_empty(), "maillage vide");
        assert_eq!(idx.len() % 3, 0, "les indices ne forment pas des triangles");
        assert!(
            idx.iter().all(|i| (*i as usize) < s.len()),
            "un indice sort du tableau de sommets"
        );

        let rayons: Vec<f32> = s.iter().map(|(p, _, _)| p.length()).collect();
        let min = rayons.iter().cloned().fold(f32::MAX, f32::min);
        let max = rayons.iter().cloned().fold(0.0_f32, f32::max);
        // **Bosselé** : au moins 25 % d'écart entre le creux et la bosse.
        assert!(max > min * 1.25, "rayons de {min} à {max} : c'est encore une bille");
        // ⚠️ Mais **pas déchiqueté** : un rayon qui doublerait donnerait des
        // pointes, et un rayon nul ou négatif retournerait la surface.
        assert!(min > 0.3, "creux à {min} : la surface se retourne");
        assert!(max < 1.6, "pointe à {max} : ce n'est plus un caillou");
    }

    /// **Il porte de la glace ET de la roche**, et le mélange ne suit pas le
    /// relief.
    ///
    /// ⚠️ Un bruit unique pour la forme et la matière donnerait des creux
    /// systématiquement sombres — une balle peinte, pas un caillou. Le test
    /// mesure la **corrélation** entre les deux et refuse qu'elle soit forte.
    #[test]
    fn le_noyau_melange_glace_et_roche_sans_calquer_le_relief() {
        let (s, _) = noyau(0.37);
        let clair: Vec<f32> = s.iter().map(|(_, _, c)| c.r + c.g + c.b).collect();
        let min = clair.iter().cloned().fold(f32::MAX, f32::min);
        let max = clair.iter().cloned().fold(0.0_f32, f32::max);
        assert!(max > min * 1.8, "teintes de {min} à {max} : une seule matière");
        // Toutes les faces sont opaques : une transparence ici ferait voir
        // l'intérieur du caillou.
        assert!(s.iter().all(|(_, _, c)| c.a == 1.0), "un sommet translucide");

        // Corrélation relief / clarté, en rangs grossiers : si les creux étaient
        // toujours sombres, les deux classements coïncideraient.
        let n = s.len() as f32;
        let (mr, mc) = (
            s.iter().map(|(p, _, _)| p.length()).sum::<f32>() / n,
            clair.iter().sum::<f32>() / n,
        );
        let mut num = 0.0;
        let (mut dr, mut dc) = (0.0, 0.0);
        for (i, (p, _, _)) in s.iter().enumerate() {
            let (a, b) = (p.length() - mr, clair[i] - mc);
            num += a * b;
            dr += a * a;
            dc += b * b;
        }
        let correl = num / (dr.sqrt() * dc.sqrt()).max(1e-6);
        assert!(
            correl.abs() < 0.6,
            "corrélation relief/matière de {correl} : la texture calque la forme"
        );
    }

    /// **Deux comètes ne sont pas le même caillou**, et la même l'est toujours.
    #[test]
    fn le_noyau_depend_de_sa_graine_et_reste_reproductible() {
        let (a, _) = noyau(0.11);
        let (b, _) = noyau(0.73);
        let (a2, _) = noyau(0.11);
        assert_eq!(a.len(), b.len(), "les maillages n'ont pas la même structure");
        let ecart = |x: &[SommetNoyau], y: &[SommetNoyau]| {
            x.iter().zip(y).map(|(p, q)| (p.0 - q.0).length()).sum::<f32>() / x.len() as f32
        };
        assert!(ecart(&a, &b) > 0.02, "deux graines donnent le même caillou");
        assert_eq!(ecart(&a, &a2), 0.0, "la même graine ne redonne pas le même caillou");
    }

    /// **La queue se voit, quelle que soit l'échelle.**
    ///
    /// ⚠️ Elle était **invisible** — signalé à l'écran. Sa longueur suit la
    /// distance à l'étoile, ce qui est juste ; mais le noyau a une taille fixe et
    /// exagérée, et les deux échelles n'ont aucun rapport. Dans la galerie, où
    /// l'orbite tient en 3,5 unités, la queue était **plus courte que le noyau
    /// qui l'émet**. Le test l'exige dans les deux régimes.
    #[test]
    fn la_queue_reste_plus_longue_que_son_noyau_a_toute_echelle() {
        let rayon = 0.09_f32;
        // Deux échelles **sans commune mesure** : la galerie des comètes, dont
        // l'orbite tient en 3,5 unités, et Halley, dont le demi-grand axe fait
        // 856 unités. Chacune est prise **à son propre périhélie**, le moment où
        // la queue doit être la plus franche.
        for (a, e) in [(3.5_f32, 0.9_f32), (856.0, 0.967)] {
            let seuil = seuil_activite(a, e);
            let perihelie = a * (1.0 - e);
            let pts =
                queue(vec3(perihelie, 0.0, 0.0), Vec3::ZERO, vec3(0.0, 0.0, 1.0), rayon, seuil);
            assert!(!pts.is_empty(), "aucune queue au périhélie de ({a}, {e})");
            let longueur = (pts.last().unwrap().0 - pts[0].0).length();
            assert!(
                longueur > rayon * 6.0,
                "orbite ({a}, {e}) : queue de {longueur} pour un noyau de rayon {rayon} — \
                 plus courte que le caillou qui l'émet, donc invisible"
            );
        }
    }

    /// **Une queue plus fine qu'un fil ne se voit pas**, quelle que soit
    /// l'échelle.
    ///
    /// ⚠️ Signalé à l'écran une deuxième fois : *« me manque toujours la queue de
    /// la comète »*. Il y en avait bien une, longue de 20 unités monde chez
    /// Halley — mais large de 0,15, parce que la largeur suivait le **rayon du
    /// noyau** quand la longueur suit la **distance à l'étoile**. Un fil de 1
    /// pour 130, sub-pixel dès qu'on cadre le système.
    ///
    /// C'est le **même défaut** que celui qu'avait corrigé `LONGUEUR_MIN_NOYAUX`,
    /// à l'autre bout de la même fonction : deux grandeurs réglées sur deux
    /// échelles sans commune mesure. La garde tient donc le **rapport**, la seule
    /// chose qui se lise à l'œil, et non une largeur absolue.
    #[test]
    fn la_queue_est_un_cone_et_non_un_fil() {
        let rayon = 0.09_f32;
        for (a, e) in [(3.5_f32, 0.9_f32), (856.0, 0.967)] {
            let seuil = seuil_activite(a, e);
            let perihelie = a * (1.0 - e);
            let pts =
                queue(vec3(perihelie, 0.0, 0.0), Vec3::ZERO, vec3(0.0, 0.0, 1.0), rayon, seuil);
            let longueur = (pts.last().unwrap().0 - pts[0].0).length();
            let large = pts.last().unwrap().1 * 2.0;
            // Un cône de comète s'ouvre franchement : au bout, sa largeur vaut
            // au moins le dixième de sa longueur. À 1/130, c'était un trait.
            assert!(
                large > longueur / 12.0,
                "orbite ({a}, {e}) : {longueur} de long pour {large} de large — c'est un fil"
            );
            // Mais pas un ballon : une queue aussi large que longue ne se lirait
            // plus comme une direction.
            assert!(large < longueur * 0.8, "orbite ({a}, {e}) : queue aussi large que longue");
            // Et jamais plus fine que le caillou qui l'émet, même à sa naissance.
            assert!(pts[0].1 > rayon * 0.5, "la queue naît plus fine que son noyau");
        }
    }

    /// **Elle se voit sur fond d'étoiles.** ⚠️ Exagéré et demandé tel quel : à
    /// 0,22 d'opacité au mieux, la queue se perdait dans le champ. Dette
    /// assumée, voir [`ECLAT`].
    #[test]
    fn la_queue_est_franchement_opaque_a_son_pic() {
        let seuil = seuil_activite(856.0, 0.967);
        let pts = queue(vec3(28.0, 0.0, 0.0), Vec3::ZERO, vec3(0.0, 0.0, 1.0), 0.08, seuil);
        let base = pts[0].2.a;
        assert!(base > 0.4, "opacité de {base} au ras du noyau : la queue est un voile");
        assert!(base <= 1.0, "opacité de {base} : hors bornes");
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

    /// **La courbure doit se voir là où la queue est encore opaque.**
    ///
    /// ⚠️ Signalé à l'écran : *« la queue doit être en relation avec la
    /// trajectoire, on dirait qu'elle est poussée toujours vers l'extérieur du
    /// système »*. Elle l'était — et
    /// [`la_queue_de_poussiere_s_incurve_vers_la_trainee`] restait **vert**,
    /// parce qu'il se contentait de constater que la courbure **existe**. Elle
    /// existait : 5,6° à mi-queue, 24° au dernier point, dont l'opacité est
    /// nulle. Toute la courbure vivait dans la partie transparente.
    ///
    /// C'est la même leçon que la lentille du lot 8 : *mes tests garantissaient
    /// que l'effet était là, aucun ne disait qu'il devait se **voir***. La garde
    /// mesure donc la déviation **au point où la queue a encore la moitié de son
    /// opacité**, et non au bout du ruban.
    #[test]
    fn la_courbure_se_voit_dans_la_partie_eclairee_de_la_queue() {
        let noyau = vec3(10.0, 0.0, 0.0);
        let vitesse = vec3(0.0, 0.0, 4.0); // perpendiculaire au rayon
        let pts = queue(noyau, Vec3::ZERO, vitesse, 0.1, 30.0);
        let anti_solaire = noyau.normalize();

        // Le dernier point encore franchement visible : la moitié de l'opacité
        // du ras du noyau.
        let seuil_vu = pts[0].2.a * 0.5;
        let (i, _) =
            pts.iter().enumerate().rfind(|(_, p)| p.2.a >= seuil_vu).expect("aucun point visible");
        assert!(i > 0, "la queue s'éteint dès son premier segment");

        let dir = (pts[i].0 - noyau).normalize();
        let angle = dir.dot(anti_solaire).clamp(-1.0, 1.0).acos().to_degrees();
        assert!(
            angle > 10.0,
            "{angle}° d'écart à l'anti-solaire là où la queue se voit encore :              elle se lit droite, donc comme une queue d'ions"
        );
        // Mais pas repliée sur la trajectoire : au-delà de 60° ce n'est plus le
        // vent stellaire qui mène, c'est un sillage de bateau — l'erreur
        // classique, à l'envers.
        assert!(angle < 60.0, "{angle}° : la queue traîne derrière comme un sillage");
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
