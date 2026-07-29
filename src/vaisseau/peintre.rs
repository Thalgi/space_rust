//! Sortie de géométrie abstraite — le pivot du passage au maillage cuit.
//!
//! Problème : la géométrie des composants (treillis, panneaux, radiateurs…) doit
//! pouvoir être soit **dessinée immédiatement** (galerie `brique_demo`, ISS
//! historique de `iss.rs`), soit **cuite dans un maillage** figé pour les
//! stations. Dupliquer le code de géométrie ferait fatalement diverger les deux
//! rendus ; on abstrait donc la *sortie*, pas la géométrie.
//!
//! Un seul jeu de fonctions de forme, deux implémentations :
//! - [`Immediat`] → appels macroquad directs (comportement historique, inchangé) ;
//! - [`super::maillage::Batisseur`] → accumulation de sommets/indices.
//!
//! Le trait est consommé par des fonctions **génériques** (`fn f<P: Peintre>`),
//! donc monomorphisées : aucune indirection dynamique, aucun coût.

use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

/// Cible de dessin. Les primitives de base sont requises ; les formes composées
/// (`voile`, `parabole`) ont une implémentation par défaut bâtie dessus, que
/// [`Immediat`] surcharge pour conserver *exactement* le rendu historique.
pub trait Peintre {
    /// Cylindre plein reliant deux points.
    fn cylindre(&mut self, a: Vec3, b: Vec3, rayon: f32, couleur: Color);

    /// Cône / tronc : `r_base` à `base`, `r_bout` à `base + direction*longueur`.
    fn cone(&mut self, base: Vec3, direction: Vec3, r_base: f32, r_bout: f32, longueur: f32, couleur: Color);

    fn sphere(&mut self, centre: Vec3, rayon: f32, couleur: Color);

    /// Boîte alignée sur les axes locaux (`taille` = dimensions pleines).
    fn cube(&mut self, centre: Vec3, taille: Vec3, couleur: Color);

    /// Parallélogramme plein, visible des deux côtés.
    fn panneau(&mut self, coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color);

    /// Segment. `epaisseur` n'a de sens que pour une sortie maillage (un
    /// `draw_line_3d` immédiat est toujours d'un pixel) — elle est ignorée par
    /// [`Immediat`].
    fn ligne(&mut self, a: Vec3, b: Vec3, epaisseur: f32, couleur: Color);

    /// Lot de triangles bruts, couleur uniforme. Pour les formes qui se
    /// génèrent déjà sous forme de maillage (tuiles hexagonales d'un panneau)
    /// plutôt que par composition de primitives.
    fn triangles(&mut self, sommets: &[Vec3], indices: &[u16], couleur: Color);

    /// Empile une transformée **composée** par-dessus celle déjà active : les
    /// primitives émises jusqu'au `depiler_transforme` correspondant sont
    /// placées comme si elles vivaient dans le repère local de `m`. Seul
    /// moyen pour un composant **composite** ([`super::Composant::
    /// SousEnsemble`]) de placer chacun de ses enfants à sa propre position
    /// sans connaître l'implémentation concrète du peintre — `Immediat`
    /// empile sur la pile GL de macroquad (déjà composée nativement) ;
    /// `Batisseur` compose et sauvegarde son unique champ `transforme` (pas
    /// de pile GL, il faut la sienne). Toujours appelé par paires
    /// équilibrées, jamais imbriqué au-delà de la profondeur des
    /// `SousEnsemble` eux-mêmes (quelques niveaux tout au plus).
    fn empiler_transforme(&mut self, m: Mat4);

    /// Dépile la dernière transformée empilée par `empiler_transforme`.
    fn depiler_transforme(&mut self);

    /// Arêtes d'une boîte. En sortie maillage ce sont **12 fines barres** : un
    /// maillage de triangles ne porte pas de fils.
    fn cube_fil(&mut self, centre: Vec3, taille: Vec3, couleur: Color) {
        let h = taille * 0.5;
        let ep = (h.x.min(h.y).min(h.z) * 0.12).max(0.01);
        let coins = [
            vec3(-h.x, -h.y, -h.z),
            vec3(h.x, -h.y, -h.z),
            vec3(h.x, h.y, -h.z),
            vec3(-h.x, h.y, -h.z),
            vec3(-h.x, -h.y, h.z),
            vec3(h.x, -h.y, h.z),
            vec3(h.x, h.y, h.z),
            vec3(-h.x, h.y, h.z),
        ];
        const ARETES: [(usize, usize); 12] = [
            (0, 1), (1, 2), (2, 3), (3, 0),
            (4, 5), (5, 6), (6, 7), (7, 4),
            (0, 4), (1, 5), (2, 6), (3, 7),
        ];
        for (a, b) in ARETES {
            self.ligne(centre + coins[a], centre + coins[b], ep, couleur);
        }
    }

    /// Panneau nervuré : le parallélogramme, son contour et `cellules-1`
    /// nervures parallèles à `e1`.
    fn voile(&mut self, coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color, cellules: usize) {
        let bord = Color::new(couleur.r * 0.5, couleur.g * 0.5, couleur.b * 0.5, 1.0);
        let ep = 0.02;
        self.panneau(coin, e1, e2, couleur);
        self.ligne(coin, coin + e1, ep, bord);
        self.ligne(coin + e2, coin + e1 + e2, ep, bord);
        self.ligne(coin, coin + e2, ep, bord);
        self.ligne(coin + e1, coin + e1 + e2, ep, bord);
        for n in 1..cellules.max(1) {
            let a = coin + e2 * (n as f32 / cellules as f32);
            self.ligne(a, a + e1, ep, bord);
        }
    }

    /// Antenne parabolique : cône peu profond ouvert vers `direction`, plus la
    /// source au foyer. (La version immédiate ajoute les fils de contour, qu'un
    /// maillage de triangles ne peut pas porter.)
    fn parabole(&mut self, centre: Vec3, direction: Vec3, rayon: f32, couleur: Color) {
        let d = direction.normalize_or_zero();
        if d == Vec3::ZERO {
            return;
        }
        let prof = rayon * 0.5;
        let bord = Color::new(couleur.r * 0.5, couleur.g * 0.5, couleur.b * 0.5, 1.0);
        // Apex au centre, ouverture de rayon `rayon` à `centre + d*prof`.
        self.cone(centre, d, 0.0, rayon, prof, couleur);
        let foyer = centre + d * (prof + rayon * 0.6);
        self.ligne(centre + d * prof, foyer, rayon * 0.06, bord);
        self.sphere(foyer, rayon * 0.12, bord);
    }
}

/// Sortie immédiate : délègue aux helpers de `vaisseau/mod.rs` et à macroquad.
/// C'est le comportement historique, conservé tel quel pour les galeries.
pub struct Immediat;

impl Peintre for Immediat {
    fn cylindre(&mut self, a: Vec3, b: Vec3, rayon: f32, couleur: Color) {
        super::cylindre(a, b, rayon, couleur);
    }

    fn cone(&mut self, base: Vec3, direction: Vec3, r_base: f32, r_bout: f32, longueur: f32, couleur: Color) {
        super::cone(base, direction, r_base, r_bout, longueur, couleur);
    }

    fn sphere(&mut self, centre: Vec3, rayon: f32, couleur: Color) {
        draw_sphere(centre, rayon, None, couleur);
    }

    fn cube(&mut self, centre: Vec3, taille: Vec3, couleur: Color) {
        draw_cube(centre, taille, None, couleur);
    }

    fn panneau(&mut self, coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color) {
        super::panneau(coin, e1, e2, couleur);
    }

    fn ligne(&mut self, a: Vec3, b: Vec3, _epaisseur: f32, couleur: Color) {
        draw_line_3d(a, b, couleur);
    }

    fn triangles(&mut self, sommets: &[Vec3], indices: &[u16], couleur: Color) {
        if indices.is_empty() || sommets.is_empty() {
            return;
        }
        let vertices = sommets
            .iter()
            .map(|p| Vertex::new2(*p, Vec2::ZERO, couleur))
            .collect();
        draw_mesh(&Mesh { vertices, indices: indices.to_vec(), texture: None });
    }

    // La pile GL de macroquad compose déjà nativement (chaque push multiplie
    // par-dessus le sommet courant) : rien à sauvegarder à la main ici.
    fn empiler_transforme(&mut self, m: Mat4) {
        unsafe {
            get_internal_gl().quad_gl.push_model_matrix(m);
        }
    }

    fn depiler_transforme(&mut self) {
        unsafe {
            get_internal_gl().quad_gl.pop_model_matrix();
        }
    }

    fn cube_fil(&mut self, centre: Vec3, taille: Vec3, couleur: Color) {
        draw_cube_wires(centre, taille, couleur);
    }

    // Surcharges : on garde le rendu historique exact (nervures au trait fin,
    // fils de contour de la parabole) plutôt que la reconstruction par défaut.
    fn voile(&mut self, coin: Vec3, e1: Vec3, e2: Vec3, couleur: Color, cellules: usize) {
        super::voile(coin, e1, e2, couleur, cellules);
    }

    fn parabole(&mut self, centre: Vec3, direction: Vec3, rayon: f32, couleur: Color) {
        super::parabole(centre, direction, rayon, couleur);
    }
}
