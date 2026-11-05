//! **Le hasard en flux nommés** (G1.1, `conception/genese_v2.md` §4.4).
//!
//! La genèse V1 tire tout dans le flux global de macroquad : un tirage ajouté
//! n'importe où décale tout ce qui suit, et rien ne se teste sans contexte
//! graphique. Ici, chaque partie d'un système a **son propre flux**, dérivé de
//! la graine, d'un nom et d'un rang (`Alea::flux(graine, "planete", 2)`) :
//! ajouter des tirages aux comètes ne déplace plus les planètes.
//!
//! Générateur : SplitMix64. Petit, sans dépendance, et ses sorties ne doivent
//! **jamais changer** d'une version à l'autre — une graine rejoue son système
//! (`le_flux_rejoue_des_valeurs_figees`).

/// Un flux de hasard. Se clone pour rejouer la suite depuis le même point.
#[derive(Clone, Debug)]
pub struct Alea(u64);

/// Constante de Weyl de SplitMix64 (partie fractionnaire du nombre d'or).
const PAS: u64 = 0x9E37_79B9_7F4A_7C15;

/// Le mélangeur final de SplitMix64 : une bijection qui disperse chaque bit.
fn melanger(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Alea {
    /// Le flux `nom`, n° `rang`, de la graine `graine`. Ne dépend **que** de
    /// ces trois valeurs : aucun état partagé entre les flux.
    pub fn flux(graine: u64, nom: &str, rang: u64) -> Self {
        let mut h = melanger(graine.wrapping_add(PAS));
        for b in nom.bytes() {
            h = melanger(h ^ u64::from(b));
        }
        Alea(melanger(h ^ rang.wrapping_mul(PAS)))
    }

    /// Le flux enfant `nom`/`rang` de ce flux (une lune dans le flux de sa
    /// planète), sans consommer de tirage du parent.
    pub fn enfant(&self, nom: &str, rang: u64) -> Self {
        Self::flux(self.0, nom, rang)
    }

    /// 64 bits uniformes.
    pub fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(PAS);
        melanger(self.0)
    }

    /// Un réel uniforme dans `[0, 1[` (24 bits : la précision d'un `f32`).
    pub fn unite(&mut self) -> f32 {
        (self.suivant() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Un réel uniforme dans `[a, b[`.
    pub fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unite()
    }

    /// Un entier uniforme dans `[a, b[`, comme `gen_range` sur des entiers.
    pub fn entier(&mut self, a: i32, b: i32) -> i32 {
        debug_assert!(a < b, "intervalle vide [{a}, {b}[");
        let n = (b - a) as u64;
        a + (self.suivant() % n) as i32
    }

    /// Vrai avec la probabilité `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.unite() < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn premiers(mut a: Alea, n: usize) -> Vec<u64> {
        (0..n).map(|_| a.suivant()).collect()
    }

    /// Les sorties sont **figées** : une graine doit rejouer le même système
    /// d'une version du jeu à l'autre. Changer le générateur, le hachage du
    /// nom ou la dérivation fait rougir ce test — c'est voulu.
    #[test]
    fn le_flux_rejoue_des_valeurs_figees() {
        let v = premiers(Alea::flux(42, "etoile", 0), 3);
        assert_eq!(v, [0x4311_139C_BFCD_A814, 0xFAAD_8C53_373B_809C, 0x36BE_67F4_0491_06CD]);
    }

    #[test]
    fn la_graine_le_nom_et_le_rang_donnent_chacun_un_autre_flux() {
        let base = premiers(Alea::flux(7, "planete", 1), 4);
        for autre in [Alea::flux(8, "planete", 1), Alea::flux(7, "planetes", 1), Alea::flux(7, "planete", 2)] {
            assert_ne!(premiers(autre, 4), base);
        }
    }

    /// Le cœur des flux nommés : tirer dans un flux ne déplace pas un autre.
    #[test]
    fn tirer_dans_un_flux_ne_deplace_pas_les_autres() {
        let attendu = premiers(Alea::flux(3, "planete", 0), 8);
        let mut cometes = Alea::flux(3, "comete", 0);
        for _ in 0..1000 {
            cometes.suivant();
        }
        assert_eq!(premiers(Alea::flux(3, "planete", 0), 8), attendu);
    }

    #[test]
    fn un_enfant_ne_consomme_pas_son_parent() {
        let parent = Alea::flux(5, "planete", 2);
        let _lune = parent.enfant("lune", 0);
        assert_eq!(premiers(parent.clone(), 4), premiers(Alea::flux(5, "planete", 2), 4));
        assert_ne!(premiers(parent.enfant("lune", 0), 4), premiers(parent.enfant("lune", 1), 4));
    }

    /// Bornes respectées et répartition plausible : un mélangeur cassé (sortie
    /// constante, bits hauts morts) se voit à la moyenne ou à l'histogramme.
    #[test]
    fn les_tirages_sont_bornes_et_uniformes() {
        let mut a = Alea::flux(11, "uniforme", 0);
        let n = 20_000;
        let mut cases = [0u32; 10];
        let mut somme = 0.0;
        for _ in 0..n {
            let u = a.unite();
            assert!((0.0..1.0).contains(&u), "unite hors [0, 1[ : {u}");
            cases[(u * 10.0) as usize] += 1;
            somme += u;
            let e = a.entre(-3.0, 5.0);
            assert!((-3.0..5.0).contains(&e), "entre hors bornes : {e}");
            let k = a.entier(1, 4);
            assert!((1..4).contains(&k), "entier hors [1, 4[ : {k}");
        }
        let moyenne = somme / n as f32;
        assert!((moyenne - 0.5).abs() < 0.01, "moyenne {moyenne}");
        for (i, &c) in cases.iter().enumerate() {
            assert!((1800..2200).contains(&c), "case {i} : {c} sur {n}");
        }
    }
}
