//! Discrimination clic / glissé — `docs/conception/assembleur.md` §10.4.
//!
//! `Camera::input_orbite` fait tourner la vue au **glisser gauche**, et §8.3
//! pose au **clic gauche** : le même bouton. Sans discrimination, chaque
//! rotation de caméra se terminerait par une pose accidentelle sous le curseur.
//!
//! Aucune notion de durée : **un clic long mais immobile reste un clic**. C'est
//! le geste de quelqu'un qui vise soigneusement, précisément l'utilisateur qu'on
//! ne veut pas punir.
//!
//! Petite machine à états, sans rien de graphique — donc testable, et §10.9
//! demande qu'elle le soit.

use macroquad::prelude::Vec2;

/// Tolérance de déplacement, en pixels, sous laquelle un relâchement vaut clic.
/// Assez pour absorber le tremblement de la main et la souris qui glisse d'un
/// pixel au moment d'appuyer ; assez peu pour qu'une rotation délibérée de la
/// caméra n'en soit jamais un.
pub const SEUIL_PIXELS: f32 = 4.0;

/// Suit un appui en cours et décide, au relâchement, s'il s'agissait d'un clic.
#[derive(Default)]
pub struct Discriminateur {
    /// Point d'enfoncement, tant que le bouton est tenu.
    origine: Option<Vec2>,
    /// **Écart maximal atteint** depuis l'enfoncement, pas l'écart final.
    ///
    /// La règle naïve — comparer le point de relâchement au point d'appui —
    /// laisse passer le glissé qui **revient** à son point de départ : on fait
    /// tourner la vue d'un tour complet, on relâche là où on avait appuyé, et
    /// l'écran pose une pièce. Retenir le maximum ferme ce cas.
    derive: f32,
}

impl Discriminateur {
    /// À appeler une fois par image, avec l'état du bouton gauche.
    /// Rend le point du clic si un clic vient de s'achever.
    pub fn maj(&mut self, souris: Vec2, enfonce: bool, relache: bool) -> Option<Vec2> {
        if enfonce {
            self.origine = Some(souris);
            self.derive = 0.0;
        }
        if let Some(o) = self.origine {
            self.derive = self.derive.max(o.distance(souris));
        }
        if relache {
            // `take` : un relâchement sans appui connu (bouton enfoncé avant
            // l'entrée dans l'écran, fenêtre qui reprend le focus) ne doit pas
            // fabriquer un clic à partir de rien.
            let suivi = self.origine.take().is_some();
            if suivi && self.derive <= SEUIL_PIXELS {
                return Some(souris);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::vec2;

    /// Enchaîne des images : `(souris, enfonce, relache)`. Rend le clic obtenu.
    fn jouer(images: &[(Vec2, bool, bool)]) -> Option<Vec2> {
        let mut d = Discriminateur::default();
        let mut sortie = None;
        for &(p, e, r) in images {
            if let Some(c) = d.maj(p, e, r) {
                sortie = Some(c);
            }
        }
        sortie
    }

    #[test]
    fn appuyer_et_relacher_au_meme_point_est_un_clic() {
        let p = vec2(300.0, 200.0);
        assert_eq!(jouer(&[(p, true, false), (p, false, true)]), Some(p));
    }

    /// **Un clic long mais immobile reste un clic** (§10.4) : pas de seuil de
    /// durée. Vingt images de bouton tenu, sans bouger.
    #[test]
    fn un_appui_long_mais_immobile_reste_un_clic() {
        let p = vec2(300.0, 200.0);
        let mut images = vec![(p, true, false)];
        images.extend(std::iter::repeat_n((p, false, false), 20));
        images.push((p, false, true));
        assert_eq!(jouer(&images), Some(p));
    }

    #[test]
    fn un_glisse_franc_n_est_pas_un_clic() {
        let a = vec2(300.0, 200.0);
        let b = vec2(500.0, 260.0);
        assert_eq!(jouer(&[(a, true, false), (b, false, false), (b, false, true)]), None);
    }

    /// **Le cas que la règle naïve laisse passer** : faire tourner la vue et
    /// relâcher là où l'on avait appuyé. Comparer début et fin dirait « clic ».
    #[test]
    fn un_glisse_qui_revient_a_son_point_de_depart_n_est_pas_un_clic() {
        let a = vec2(300.0, 200.0);
        let images = [
            (a, true, false),
            (vec2(420.0, 200.0), false, false),
            (vec2(420.0, 320.0), false, false),
            (a, false, false), // revenu au départ
            (a, false, true),
        ];
        assert_eq!(jouer(&images), None);
    }

    /// Le tremblement de la main ne doit pas coûter le clic.
    #[test]
    fn un_tremblement_de_deux_pixels_reste_un_clic() {
        let a = vec2(300.0, 200.0);
        let b = vec2(302.0, 201.0);
        assert!(jouer(&[(a, true, false), (b, false, false), (b, false, true)]).is_some());
    }

    /// **Un relâchement sans appui suivi ne fabrique pas de clic.** Le cas
    /// arrive pour de vrai : bouton déjà enfoncé en entrant dans l'écran, ou
    /// fenêtre qui reprend le focus.
    #[test]
    fn relacher_sans_avoir_appuye_ne_produit_rien() {
        let p = vec2(300.0, 200.0);
        assert_eq!(jouer(&[(p, false, true)]), None);
    }

    /// Deux gestes de suite : le second ne doit pas hériter de la dérive du
    /// premier, sinon un glissé condamnerait tous les clics qui le suivent.
    #[test]
    fn un_glisse_ne_condamne_pas_le_clic_suivant() {
        let a = vec2(300.0, 200.0);
        let mut d = Discriminateur::default();
        d.maj(a, true, false);
        d.maj(vec2(600.0, 400.0), false, false);
        assert_eq!(d.maj(vec2(600.0, 400.0), false, true), None);
        // Geste suivant, propre.
        d.maj(a, true, false);
        assert_eq!(d.maj(a, false, true), Some(a));
    }
}
