//! Modèle de station **assemblée**, son état de rendu et les garde-fous de
//! taille (voir `docs/stations_fondations.md`, §1 et §3, et
//! `docs/stations_raccordement.md`, §3 — sous-étape 2b).
//!
//! - Étape 2 : on ne dessine jamais une station à moitié construite. On
//!   assemble dans un tampon local (`Assembleur`), `terminer` **consomme**
//!   l'assembleur et publie un `EtatStation` ; une `Station` publiée est
//!   **immuable**. Cohérence garantie *par construction*.
//! - Étape 3 : chaque `Piece` porte un **composant** (qui connaît son coût de
//!   rendu et son rayon local), un `Budget` flottant plafonne la complexité
//!   totale, et chaque `Station` connaît son **rayon englobant** (calculé une
//!   fois) pour le cadrage / l'anti-collision.

use super::Composant;
use macroquad::prelude::*;

/// Une pièce placée : sa transformée monde est déjà **cuite** en `Mat4` à la
/// génération (couche cuite, cf. `stations_raccordement.md` §2 — la `Mat4`
/// encode aussi les réflexions du miroir, qu'un `Quat` ne pourrait pas porter).
///
/// Elle référence son `composant`, qui fournit `cout()` (pondération du budget)
/// et `rayon_local()` (contribution à la sphère englobante).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Piece {
    pub transforme: Mat4,
    pub composant: Composant,
}

impl Piece {
    pub fn new(transforme: Mat4, composant: Composant) -> Self {
        Self { transforme, composant }
    }

    /// Centre monde de la pièce (translation de la transformée cuite).
    pub fn centre(&self) -> Vec3 {
        self.transforme.transform_point3(Vec3::ZERO)
    }
}

/// Une station terminée et **immuable**. Construite entièrement (via
/// [`Assembleur`] ou [`Station::depuis_pieces`]) puis jamais mutée : `pieces`
/// est privé, seul un accès lecture est exposé. Son `rayon` englobant est figé
/// à la construction.
#[derive(Clone, PartialEq, Debug)]
pub struct Station {
    pieces: Vec<Piece>,
    rayon: f32,
}

impl Station {
    /// Construit une station à partir de pièces déjà placées.
    /// `None` si la liste est vide — une station sans pièce n'existe pas.
    pub fn depuis_pieces(pieces: Vec<Piece>) -> Option<Station> {
        if pieces.is_empty() {
            return None;
        }
        // Sphère englobante centrée à l'origine : pour chaque pièce, la distance
        // de son centre au repère monde plus le rayon englobant de son composant.
        let rayon = pieces
            .iter()
            .fold(0.0_f32, |m, p| m.max(p.centre().length() + p.composant.rayon_local()));
        Some(Station { pieces, rayon })
    }

    /// Accès **lecture seule** aux pièces (ordre d'assemblage préservé).
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    pub fn nb_pieces(&self) -> usize {
        self.pieces.len()
    }

    /// Rayon de la sphère englobante (unités monde), figé à la construction.
    pub fn rayon(&self) -> f32 {
        self.rayon
    }

    /// Dessine toutes les pièces (le repère caméra 3D doit déjà être actif).
    /// Chaque pièce pousse sa transformée **cuite** (`Mat4`) puis délègue le
    /// tracé à son composant — c'est le branchement rendu de la couche cuite.
    pub fn dessiner(&self) {
        for p in &self.pieces {
            unsafe {
                get_internal_gl().quad_gl.push_model_matrix(p.transforme);
            }
            p.composant.dessiner();
            unsafe {
                get_internal_gl().quad_gl.pop_model_matrix();
            }
        }
    }

    /// Gizmos de debug (2f) : pour chaque port de chaque pièce, trace en monde
    /// l'axe **avant** (sens d'accouplement, orange) et l'axe **haut** (roulis,
    /// vert), plus une bille à l'origine du port. Les directions sont obtenues en
    /// appliquant la partie linéaire de la transformée cuite (valide même avec
    /// une réflexion miroir). À appeler après [`Station::dessiner`].
    pub fn dessiner_ports(&self) {
        let orange = Color::new(1.0, 0.6, 0.1, 1.0);
        let vert = Color::new(0.3, 1.0, 0.4, 1.0);
        let bille = Color::new(1.0, 1.0, 1.0, 1.0);
        for p in &self.pieces {
            for port in p.composant.ports() {
                let o = p.transforme.transform_point3(port.repere.pos);
                let avant = p.transforme.transform_vector3(port.repere.avant()).normalize_or_zero();
                let haut = p.transforme.transform_vector3(port.repere.haut()).normalize_or_zero();
                draw_sphere(o, 0.06, None, bille);
                draw_line_3d(o, o + avant * 0.6, orange);
                draw_line_3d(o, o + haut * 0.35, vert);
            }
        }
    }
}

/// État de la station affichée. **Seul `Prete` est dessiné** : c'est le
/// garde-fou contre les rendus tronqués/incohérents (point 1).
#[derive(Clone, PartialEq, Debug, Default)]
pub enum EtatStation {
    #[default]
    Vide,
    Prete(Station),
}

impl EtatStation {
    /// Garde-fou de rendu : renvoie la station **uniquement** si elle est
    /// prête. Le code d'affichage fait `if let Some(s) = etat.doit_dessiner()`.
    pub fn doit_dessiner(&self) -> Option<&Station> {
        match self {
            EtatStation::Prete(s) => Some(s),
            EtatStation::Vide => None,
        }
    }

    pub fn est_prete(&self) -> bool {
        matches!(self, EtatStation::Prete(_))
    }
}

/// Plafond de complexité (§3.1). Le budget est un **flottant** car les pièces
/// n'ont pas le même coût de rendu (un treillis nu ≪ une aile solaire
/// nervurée) : plafonner un simple nombre de pièces serait trompeur.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    restant: f32,
}

impl Budget {
    /// Nouveau budget. Un total négatif est borné à 0 (rien de finançable).
    pub fn new(total: f32) -> Self {
        Self { restant: total.max(0.0) }
    }

    pub fn restant(&self) -> f32 {
        self.restant
    }

    pub fn epuise(&self) -> bool {
        self.restant <= 0.0
    }

    /// Le coût est-il finançable ? (Un coût négatif est traité comme 0.)
    pub fn peut_payer(&self, cout: f32) -> bool {
        cout.max(0.0) <= self.restant
    }

    /// Dépense atomique : si finançable, décrémente et renvoie `true` ; sinon
    /// laisse le restant intact et renvoie `false`. Un coût négatif ne peut
    /// jamais recharger le budget (borné à 0).
    pub fn depenser(&mut self, cout: f32) -> bool {
        let c = cout.max(0.0);
        if c <= self.restant {
            self.restant -= c;
            true
        } else {
            false
        }
    }
}

/// Constructeur **atomique**. On ajoute les pièces dans un tampon local, puis
/// [`Assembleur::terminer`] consomme l'assembleur et publie l'état. Comme
/// `terminer` prend `self` par valeur, impossible de continuer à muter après
/// publication → invariant « pas de station partielle observable » tenu par le
/// type.
#[derive(Default)]
pub struct Assembleur {
    pieces: Vec<Piece>,
}

impl Assembleur {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute une pièce au tampon en cours (chaînable), sans contrôle de budget.
    pub fn ajouter(&mut self, piece: Piece) -> &mut Self {
        self.pieces.push(piece);
        self
    }

    /// Ajoute une pièce **seulement si** le budget couvre le `cout()` de son
    /// composant. Renvoie `true` quand la pièce a été posée (et le budget
    /// débité), `false` sinon. C'est le point d'intégration du générateur.
    pub fn ajouter_finance(&mut self, piece: Piece, budget: &mut Budget) -> bool {
        if budget.depenser(piece.composant.cout()) {
            self.pieces.push(piece);
            true
        } else {
            false
        }
    }

    /// Nombre de pièces déjà posées dans le tampon (avant publication).
    pub fn nb_en_cours(&self) -> usize {
        self.pieces.len()
    }

    /// Publie l'état : `Vide` si rien n'a été posé, sinon `Prete`.
    pub fn terminer(self) -> EtatStation {
        match Station::depuis_pieces(self.pieces) {
            Some(s) => EtatStation::Prete(s),
            None => EtatStation::Vide,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Profil, VarianteModule};
    use super::*;

    fn mod_p(profil: Profil, longueur: f32) -> Composant {
        Composant::ModuleAxial { profil, variante: VarianteModule::Standard, longueur }
    }

    fn piece_en(centre: Vec3, comp: Composant) -> Piece {
        Piece::new(Mat4::from_translation(centre), comp)
    }

    fn piece() -> Piece {
        piece_en(vec3(1.0, 2.0, 3.0), mod_p(Profil::P1, 1.0))
    }

    // ---- Étape 2 : état & immuabilité ----

    // 1. Zéro pièce : une station sans pièce n'existe pas.
    #[test]
    fn station_sans_piece_nexiste_pas() {
        assert!(Station::depuis_pieces(vec![]).is_none());
    }

    // 1 bis. Un assembleur vide publie `Vide`, jamais un `Prete` vide.
    #[test]
    fn assembleur_vide_donne_vide() {
        let e = Assembleur::new().terminer();
        assert_eq!(e, EtatStation::Vide);
        assert!(!e.est_prete());
    }

    // 2. Garde-fou : `Vide` ne se dessine jamais.
    #[test]
    fn vide_ne_se_dessine_pas() {
        assert!(EtatStation::Vide.doit_dessiner().is_none());
    }

    // 3. Défaut = `Vide`.
    #[test]
    fn defaut_est_vide() {
        assert_eq!(EtatStation::default(), EtatStation::Vide);
    }

    // 4. Une seule pièce : minimal valide → `Prete`.
    #[test]
    fn une_piece_est_prete() {
        let mut a = Assembleur::new();
        a.ajouter(piece());
        let e = a.terminer();
        assert!(e.est_prete());
        assert_eq!(e.doit_dessiner().unwrap().nb_pieces(), 1);
    }

    // 5. `Prete` se dessine et expose ses pièces en lecture.
    #[test]
    fn prete_se_dessine() {
        let s = Station::depuis_pieces(vec![piece()]).unwrap();
        let e = EtatStation::Prete(s);
        let vue = e.doit_dessiner().expect("une station prête doit se dessiner");
        assert_eq!(vue.pieces()[0], piece());
    }

    // 6. Ordre des pièces préservé (déterminisme du rendu).
    #[test]
    fn ordre_preserve() {
        let mut a = Assembleur::new();
        a.ajouter(piece_en(Vec3::ZERO, mod_p(Profil::P0, 1.0)))
            .ajouter(piece_en(vec3(9.0, 0.0, 0.0), mod_p(Profil::P3, 1.0)));
        let e = a.terminer();
        let p = e.doit_dessiner().unwrap().pieces();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].composant, mod_p(Profil::P0, 1.0));
        assert_eq!(p[1].composant, mod_p(Profil::P3, 1.0));
    }

    // 7. `nb_en_cours` suit les ajouts pendant la construction.
    #[test]
    fn nb_en_cours_suit_les_ajouts() {
        let mut a = Assembleur::new();
        assert_eq!(a.nb_en_cours(), 0);
        a.ajouter(piece());
        assert_eq!(a.nb_en_cours(), 1);
    }

    // 8. Republication : régénérer remplace l'état proprement.
    #[test]
    fn republication_remplace_letat() {
        let mut etat = Assembleur::new().terminer(); // Vide
        assert!(!etat.est_prete());
        let mut a = Assembleur::new();
        a.ajouter(piece());
        etat = a.terminer(); // devient Prete
        assert!(etat.est_prete());
    }

    // 9. Grosse station : aucun plafond tant qu'on n'impose pas de budget.
    #[test]
    fn beaucoup_de_pieces() {
        let mut a = Assembleur::new();
        for i in 0..1000 {
            a.ajouter(piece_en(vec3(i as f32, 0.0, 0.0), mod_p(Profil::P1, 1.0)));
        }
        assert_eq!(a.terminer().doit_dessiner().unwrap().nb_pieces(), 1000);
    }

    // ---- Étape 3 : coût (via composant), budget, rayon ----

    // 10. Le coût d'une pièce vient de son composant.
    #[test]
    fn cout_vient_du_composant() {
        assert_eq!(piece().composant.cout(), 5.0); // corps + 2 rives + 2 collerettes
    }

    // 11. Rayon d'une pièce à l'origine = rayon local de son composant.
    // (module P1 court : rayon_local = max(demi-longueur 0.5, rayon P1 1.0) = 1.0)
    #[test]
    fn rayon_piece_origine_egale_rayon_local() {
        let s = Station::depuis_pieces(vec![piece_en(Vec3::ZERO, mod_p(Profil::P1, 1.0))]).unwrap();
        assert_eq!(s.rayon(), 1.0);
    }

    // 12. Rayon décalé = |centre| + rayon local du composant.
    #[test]
    fn rayon_avec_decalage() {
        let s =
            Station::depuis_pieces(vec![piece_en(vec3(3.0, 0.0, 0.0), mod_p(Profil::P1, 1.0))]).unwrap();
        assert_eq!(s.rayon(), 3.0 + 1.0);
    }

    // 13. Rayon d'une station = le max des contributions (sphère englobante).
    #[test]
    fn rayon_est_le_max() {
        let mut a = Assembleur::new();
        a.ajouter(piece_en(Vec3::ZERO, mod_p(Profil::P1, 1.0))) // 0 + 1.0 (rayon P1 domine)
            .ajouter(piece_en(vec3(5.0, 0.0, 0.0), mod_p(Profil::P0, 1.0))); // 5 + 0.625 (col)
        let s = match a.terminer() {
            EtatStation::Prete(s) => s,
            EtatStation::Vide => panic!("devrait être prête"),
        };
        // P0 court : rayon_local = max(0.5 + 0.5×0.25, 0.5) = 0.625 → 5.625.
        assert_eq!(s.rayon(), 5.625);
    }

    // 14. Budget négatif borné à 0.
    #[test]
    fn budget_neuf_borne_a_zero() {
        let b = Budget::new(-5.0);
        assert_eq!(b.restant(), 0.0);
        assert!(b.epuise());
    }

    // 15. Dépense exacte : OK, puis épuisé, puis tout refusé.
    #[test]
    fn budget_depense_exacte() {
        let mut b = Budget::new(10.0);
        assert!(b.depenser(10.0));
        assert_eq!(b.restant(), 0.0);
        assert!(b.epuise());
        assert!(!b.depenser(0.1));
    }

    // 16. Sur-dépense refusée, restant intact (atomicité).
    #[test]
    fn budget_refuse_overspend() {
        let mut b = Budget::new(1.0);
        assert!(!b.depenser(2.0));
        assert_eq!(b.restant(), 1.0);
    }

    // 17. Coût négatif ignoré : jamais de recharge du budget.
    #[test]
    fn budget_ignore_cout_negatif() {
        let mut b = Budget::new(3.0);
        assert!(b.depenser(-5.0)); // traité comme 0 → finançable
        assert_eq!(b.restant(), 3.0); // pas de recharge
    }

    // 18. peut_payer cohérent avec depenser.
    #[test]
    fn budget_peut_payer() {
        let b = Budget::new(2.0);
        assert!(b.peut_payer(2.0));
        assert!(!b.peut_payer(2.01));
    }

    // 19. ajouter_finance respecte le budget : pose tant que le cout() tient.
    // Un module coûte 5.0 ; avec 5.5 de budget, une seule pièce passe.
    #[test]
    fn ajouter_finance_respecte_budget() {
        let mut a = Assembleur::new();
        let mut b = Budget::new(5.5);
        assert!(a.ajouter_finance(piece(), &mut b));
        assert!(!a.ajouter_finance(piece(), &mut b));
        assert_eq!(a.nb_en_cours(), 1); // la 2e n'a pas été posée
        assert!((b.restant() - 0.5).abs() < 1e-6);
    }
}
