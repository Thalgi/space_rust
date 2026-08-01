//! **Inventaire des fils d'un composant** : une troisième sortie du trait
//! [`Peintre`], qui n'affiche rien et ne cuit rien — elle **compte**.
//!
//! Le but est de pouvoir désigner une pièce de charpente par un numéro plutôt
//! que par une périphrase. « Le fil 37 est trop long » vaut mieux que « la barre
//! en haut à gauche, celle qui part en biais », et c'est la différence entre une
//! correction en un aller-retour et cinq.
//!
//! ⚠️ **Pourquoi une sortie de `Peintre` et pas une liste écrite à la main.**
//! Le projet a déjà deux sorties pour une seule géométrie (`Immediat` pour le
//! rendu direct, `Batisseur` pour le maillage cuit) — c'est tout l'intérêt du
//! trait. Une troisième hérite gratuitement de la **même** géométrie : les
//! numéros ne peuvent pas se désynchroniser du dessin, puisqu'ils sont produits
//! par le même code. Une table tenue à la main aurait dérivé au premier
//! ajustement, exactement comme les quatre sources du catalogue de briques
//! (`conception/assembleur.md` §5.1).
//!
//! Les positions sont dans le repère **local** du composant, transformées par la
//! pile de `empiler_transforme` — donc un `SousEnsemble` rend les fils de ses
//! enfants à leur vraie place.

use super::peintre::Peintre;
use macroquad::prelude::*;

/// Nature du trait relevé. Le mot « fil » couvre tout ce qu'un composant émet ;
/// le genre dit ce que c'était à l'origine, ce qui rend le catalogue lisible.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GenreFil {
    /// Longeron, montant, diagonale — la barre de treillis.
    Cylindre,
    /// Tronc de cône : fût, tuyère, jupe.
    Cone,
    /// Bille, dôme.
    Sphere,
    /// Caisse.
    Cube,
    /// Voile plate (panneau solaire, radiateur).
    Panneau,
    /// Trait fin, sans volume.
    Ligne,
    /// Maille brute (coiffes, plaques de bouclier).
    Maille,
}

impl GenreFil {
    #[allow(dead_code)] // consommé par le catalogue, qui est en `cfg(test)`
    pub fn nom(self) -> &'static str {
        match self {
            GenreFil::Cylindre => "cylindre",
            GenreFil::Cone => "cone",
            GenreFil::Sphere => "sphere",
            GenreFil::Cube => "cube",
            GenreFil::Panneau => "panneau",
            GenreFil::Ligne => "ligne",
            GenreFil::Maille => "maille",
        }
    }
}

/// Un trait relevé, avec son **numéro d'ordre d'émission**.
///
/// Le numéro est l'ordre dans lequel le composant l'a dessiné. C'est stable tant
/// que la fonction `dessiner` n'est pas réordonnée — et si elle l'est, le
/// catalogue le montre (`suivi/…` : le fichier de référence est comparé par un
/// test, il ne peut pas se périmer en silence).
#[derive(Clone, Copy, Debug)]
pub struct Fil {
    pub numero: usize,
    pub genre: GenreFil,
    /// Extrémités du trait. Pour un volume ramassé (sphère, cube), `a == b`.
    pub a: Vec3,
    pub b: Vec3,
    /// Demi-épaisseur **en `a`** et **en `b`**.
    ///
    /// ⚠️ Deux rayons et non un seul, et c'est ce qui rend le mesureur exact.
    /// Un cône s'effile : lui prêter partout le rayon de son gros bout gonflait
    /// artificiellement le « besoin » d'enveloppe, et le premier relevé rendait
    /// 22 pièces débordantes sur 30 dont une bonne part n'était que cette
    /// approximation (`suivi/assembleur.md` L1.8).
    pub rayon_a: f32,
    pub rayon_b: f32,
}

impl Fil {
    #[allow(dead_code)] // `longueur` sert au catalogue (compilé en test), `milieu` à l'étiquetage à venir
    pub fn longueur(&self) -> f32 {
        self.a.distance(self.b)
    }
    #[allow(dead_code)]
    pub fn milieu(&self) -> Vec3 {
        (self.a + self.b) * 0.5
    }
    /// Le plus grand des deux rayons — pour l'affichage et les bornes grossières.
    pub fn rayon_max(&self) -> f32 {
        self.rayon_a.max(self.rayon_b)
    }
    /// Rayon à la fraction `t` du segment. **Interpolation linéaire** : c'est
    /// exactement le profil d'un tronc de cône, donc exact et pas approché.
    pub fn rayon_a_t(&self, t: f32) -> f32 {
        self.rayon_a + (self.rayon_b - self.rayon_a) * t.clamp(0.0, 1.0)
    }
}

/// Peintre qui relève au lieu de dessiner.
#[derive(Default)]
pub struct Inventaire {
    fils: Vec<Fil>,
    transforme: Mat4,
    pile: Vec<Mat4>,
}

impl Inventaire {
    pub fn new() -> Self {
        Self { fils: Vec::new(), transforme: Mat4::IDENTITY, pile: Vec::new() }
    }

    pub fn terminer(self) -> Vec<Fil> {
        self.fils
    }

    fn pousser(&mut self, genre: GenreFil, a: Vec3, b: Vec3, ra: f32, rb: f32) {
        self.fils.push(Fil {
            numero: self.fils.len(),
            genre,
            a: self.transforme.transform_point3(a),
            b: self.transforme.transform_point3(b),
            rayon_a: ra,
            rayon_b: rb,
        });
    }
}

impl Peintre for Inventaire {
    fn cylindre(&mut self, a: Vec3, b: Vec3, rayon: f32, _couleur: Color) {
        self.pousser(GenreFil::Cylindre, a, b, rayon, rayon);
    }

    fn cone(&mut self, base: Vec3, direction: Vec3, r_base: f32, r_bout: f32, longueur: f32, _couleur: Color) {
        // Les **deux** rayons sont conservés : le cône s'effile, et c'est
        // précisément l'information que jeter rendait le mesureur pessimiste.
        let d = direction.normalize_or_zero();
        self.pousser(GenreFil::Cone, base, base + d * longueur, r_base, r_bout);
    }

    fn sphere(&mut self, centre: Vec3, rayon: f32, _couleur: Color) {
        self.pousser(GenreFil::Sphere, centre, centre, rayon, rayon);
    }

    fn cube(&mut self, centre: Vec3, taille: Vec3, _couleur: Color) {
        // L'axe du fil est la **plus grande** dimension de la caisse : c'est
        // celle qui la fait lire comme une barre quand elle est allongée.
        let demi = taille * 0.5;
        let (axe, reste) = if demi.x >= demi.y && demi.x >= demi.z {
            (Vec3::X * demi.x, demi.y.hypot(demi.z))
        } else if demi.y >= demi.z {
            (Vec3::Y * demi.y, demi.x.hypot(demi.z))
        } else {
            (Vec3::Z * demi.z, demi.x.hypot(demi.y))
        };
        self.pousser(GenreFil::Cube, centre - axe, centre + axe, reste, reste);
    }

    fn panneau(&mut self, coin: Vec3, e1: Vec3, e2: Vec3, _couleur: Color) {
        // Diagonale de la voile, rayon nul.
        //
        // ⚠️ **Sous-couvre volontairement**, et c'est une dette assumée. Une
        // capsule médiane (axe au milieu, rayon = demi-largeur) couvrirait la
        // voile entière et serait plus juste en principe — essayée le
        // 2026-08-01, elle fait bondir le besoin du `RadiateurMega` de ×1,07 à
        // ×1,72, en **contradiction** avec `les_rayons_declares_contiennent_la_
        // piece`, qui mesure la même chose sur les sommets cuits et reste vert.
        //
        // Deux mesures de la même grandeur qui se contredisent, c'est exactement
        // le défaut que ce chantier traque (§C.29) : tant qu'on ne sait pas
        // laquelle a tort, on garde la version conservatrice et on note l'écart
        // (`suivi/assembleur.md` L1.8, « ce qui reste à trancher »).
        self.pousser(GenreFil::Panneau, coin, coin + e1 + e2, 0.0, 0.0);
    }

    fn ligne(&mut self, a: Vec3, b: Vec3, _epaisseur: f32, _couleur: Color) {
        self.pousser(GenreFil::Ligne, a, b, 0.0, 0.0);
    }

    fn triangles(&mut self, sommets: &[Vec3], _indices: &[u16], _couleur: Color) {
        // Une maille brute n'a pas d'axe : on la désigne par sa plus grande
        // étendue, ce qui donne un trait qui la traverse.
        if sommets.is_empty() {
            return;
        }
        let (mut lo, mut hi) = (sommets[0], sommets[0]);
        for s in sommets {
            lo = lo.min(*s);
            hi = hi.max(*s);
        }
        // ⚠️ **Rayon nul, donc sous-couverture assumée.** Créditer la maille du
        // rayon de son sommet le plus éloigné de la diagonale — essayé le
        // 2026-08-01 — **double-compte** : un point pris au milieu de la
        // diagonale se voit alors prêter la distance d'un coin, et le besoin du
        // `RadiateurMega` bondit de ×1,07 à ×1,72, en contradiction avec
        // `les_rayons_declares_contiennent_la_piece` qui mesure la même chose
        // sur les sommets cuits et reste vert.
        //
        // Le fond du problème : une plaque plate n'est pas une capsule, et aucun
        // couple (axe, rayon) ne la borne sans gaspiller. Il faudrait un autre
        // genre de fil pour ça. En attendant, ne rien prétendre vaut mieux que
        // prétendre faux (`suivi/assembleur.md` L1.8).
        self.pousser(GenreFil::Maille, lo, hi, 0.0, 0.0);
    }

    fn empiler_transforme(&mut self, m: Mat4) {
        self.pile.push(self.transforme);
        self.transforme *= m;
    }

    fn depiler_transforme(&mut self) {
        if let Some(m) = self.pile.pop() {
            self.transforme = m;
        }
    }
}

/// Relève les fils d'un composant, dans son repère local.
pub fn fils(comp: &super::Composant) -> Vec<Fil> {
    let mut inv = Inventaire::new();
    comp.dessiner(&mut inv);
    inv.terminer()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::{Composant, Profil, StyleTreillis, VarianteModule};

    fn treillis() -> Composant {
        Composant::Treillis { profil: Profil::P1, longueur: 8.0, style: StyleTreillis::Carre }
    }

    // Les numéros doivent être **consécutifs à partir de 0** et sans trou :
    // c'est tout ce qui permet de désigner un fil à l'oral.
    #[test]
    fn les_numeros_sont_consecutifs() {
        let f = fils(&treillis());
        assert!(!f.is_empty(), "un treillis a des barres");
        for (i, fil) in f.iter().enumerate() {
            assert_eq!(fil.numero, i, "numérotation trouée");
        }
    }

    // **L'inventaire doit voir la même géométrie que le dessin.** Il partage le
    // trait `Peintre` avec les deux autres sorties, donc un composant qui cuit
    // des sommets doit relever au moins un fil — et réciproquement. Sans ça, les
    // numéros désigneraient une géométrie qui n'est pas celle affichée.
    #[test]
    fn tout_ce_qui_se_dessine_sinventorie() {
        use crate::vaisseau::maillage::Batisseur;
        for comp in [
            treillis(),
            Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 },
            Composant::BouclierPetit { profil: Profil::P0, rayon: 5.5 },
            Composant::Panache { longueur: 10.0, rayon_col: 0.1, rayon_bout: 1.0, intensite: 1.0 },
        ] {
            let mut b = Batisseur::new();
            comp.dessiner(&mut b);
            let sommets: usize = b.terminer().iter().map(|l| l.vertices.len()).sum();
            let n = fils(&comp).len();
            assert_eq!(
                sommets > 0,
                n > 0,
                "{comp:?} : {sommets} sommets mais {n} fils — les deux sorties divergent"
            );
        }
    }

    // Les fils d'un composant doivent tenir **dans son enveloppe** : ce sont les
    // mêmes primitives que celles qui la remplissent. Un fil dehors signalerait
    // que l'inventaire applique mal la pile de transformées.
    #[test]
    fn les_fils_tiennent_dans_lenveloppe_de_la_piece() {
        for comp in [treillis(), Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 }] {
            let env = comp.enveloppe_locale();
            for f in fils(&comp) {
                for p in [f.a, f.b] {
                    assert!(
                        env.profondeur(p) <= env.rayon * 0.4,
                        "{comp:?} fil {} déborde de {:.2}",
                        f.numero,
                        env.profondeur(p)
                    );
                }
            }
        }
    }
}
