//! **Le banc statistique** (G2.1, `conception/genese_v2.md` §5.3) : les deux
//! genèses comparées sur des milliers de graines, en rapport texte.
//!
//! `BANC=<n> cargo run --release` construit `n` systèmes de chaque genèse et
//! les **mesure avec le même code**, sur les systèmes construits : une
//! différence au rapport vient des genèses, pas de la mesure. C'est pourquoi le
//! banc tourne jeu lancé, comme `DIAG` : construire un système charge des
//! matériaux GPU.

use super::instancier::instancier;
use super::planifier::planifier;
use crate::astre::Categorie;
use crate::systeme::Systeme;
use std::collections::{BTreeMap, BTreeSet};

/// Ce qu'on a compté sur un lot de systèmes.
#[derive(Default)]
pub struct Bilan {
    /// Section → valeur → nombre d'occurrences.
    comptes: BTreeMap<&'static str, BTreeMap<String, u32>>,
    /// Grandeur → (somme, nombre).
    moyennes: BTreeMap<&'static str, (f64, u32)>,
}

impl Bilan {
    fn compter(&mut self, section: &'static str, valeur: impl ToString) {
        *self.comptes.entry(section).or_default().entry(valeur.to_string()).or_insert(0) += 1;
    }

    fn ajouter(&mut self, grandeur: &'static str, x: f32) {
        let m = self.moyennes.entry(grandeur).or_insert((0.0, 0));
        m.0 += f64::from(x);
        m.1 += 1;
    }

    fn total(&self, section: &str) -> u32 {
        self.comptes.get(section).map_or(0, |m| m.values().sum())
    }

    fn compte(&self, section: &str, valeur: &str) -> u32 {
        self.comptes.get(section).and_then(|m| m.get(valeur)).copied().unwrap_or(0)
    }

    /// Mesure un système construit. `info` est son libellé de génération.
    fn mesurer(&mut self, sys: &Systeme, info: &str) {
        let nb_etoiles = sys.nb_etoiles().max(1); // une naine brune au centre est une planète
        let simple = nb_etoiles == 1;
        self.compter("etoiles par systeme", nb_etoiles);
        if let Some(t) = type_etoile(info) {
            self.compter("type d'etoile (systemes simples)", t);
        }
        let (mut planetes, mut cometes, mut disques) = (0, 0, 0);
        let mut distances: Vec<f32> = Vec::new();
        for i in 0..sys.nb_astres() {
            let rayon = sys.rayon_de(i).unwrap_or(0.0);
            match sys.categorie_de(i) {
                Some(Categorie::Planete) => {
                    let d = sys.ua_de_monde(sys.position(i).length());
                    if d < 1e-4 {
                        continue; // la naine brune au centre du système
                    }
                    planetes += 1;
                    distances.push(d);
                    if let Some(f) = crate::ecran::fiche::fiche(sys, i) {
                        self.compter("zone d'une planete (fiche)", f.habitabilite.libelle());
                    }
                    self.ajouter("rayon d'une planete", rayon);
                    if let Some(c) = sys.classe_planete_de(i) {
                        self.compter("classe d'une planete", format!("{c:?}"));
                    }
                    let section = if sys.gazeuse_de(i) == Some(true) { "lunes d'une gazeuse" } else { "lunes d'une autre planete" };
                    self.compter(section, sys.nb_lunes(i));
                }
                Some(Categorie::Lune) => {
                    self.ajouter("rayon d'une lune", rayon);
                    if let Some(p) = sys.orbite_lune_de(i).first() {
                        let g = if simple { "orbite d'une lune (simples)" } else { "orbite d'une lune (multiples)" };
                        self.ajouter(g, p.length());
                    }
                    if let Some(c) = sys.classe_planete_de(i) {
                        self.compter("classe d'une lune", format!("{c:?}"));
                    }
                }
                Some(Categorie::Comete) => cometes += 1,
                Some(Categorie::Asteroide) => disques += 1,
                _ => {}
            }
        }
        if simple {
            self.compter("planetes (systemes simples)", planetes);
            self.compter("cometes (systemes simples)", cometes);
            self.compter("disques (systemes simples)", disques);
            distances.sort_by(f32::total_cmp);
            if let (Some(a), Some(z)) = (distances.first(), distances.last()) {
                self.ajouter("periastre 1re planete, UA (simples)", *a);
                self.ajouter("periastre derniere planete, UA (simples)", *z);
            }
        } else {
            self.compter("planetes (systemes multiples)", planetes);
        }
    }
}

/// Le type d'étoile d'un libellé de système simple : « Etoile Pulsar  -  44334 K ».
/// `None` pour un multiple. Une pollution (« Etoile X (sodium)  -  … ») n'en fait
/// pas partie.
fn type_etoile(info: &str) -> Option<&str> {
    let reste = info.strip_prefix("Etoile ")?;
    let fin = reste.find("  -  ")?;
    let nom = &reste[..fin];
    Some(nom.find(" (").map_or(nom, |p| &nom[..p]))
}

/// **L'écart de deux parts est-il plus que du bruit ?** Test de deux proportions
/// à 3 écarts-types : le rapport compte une cinquantaine de lignes, un seuil à 2
/// en marquerait plusieurs par pur hasard.
fn ecart_significatif(k1: u32, n1: u32, k2: u32, n2: u32) -> bool {
    if n1 == 0 || n2 == 0 {
        return false;
    }
    let (k1, n1, k2, n2) = (f64::from(k1), f64::from(n1), f64::from(k2), f64::from(n2));
    let p = (k1 + k2) / (n1 + n2);
    let sigma = (p * (1.0 - p) * (1.0 / n1 + 1.0 / n2)).sqrt();
    sigma > 0.0 && (k1 / n1 - k2 / n2).abs() > 3.0 * sigma
}

/// **Le rapport côte à côte.** Chaque section liste l'union des valeurs vues : une
/// valeur qu'une seule genèse produit doit se voir. `*` marque un écart
/// significatif.
fn rapport(a: &Bilan, b: &Bilan) -> String {
    let mut s = String::new();
    let sections: BTreeSet<&&str> = a.comptes.keys().chain(b.comptes.keys()).collect();
    for section in sections {
        let (na, nb) = (a.total(section), b.total(section));
        s += &format!("\n{section} ({na} / {nb})\n");
        let valeurs: BTreeSet<&String> = [a, b].iter().filter_map(|x| x.comptes.get(*section)).flat_map(|m| m.keys()).collect();
        for v in valeurs {
            let (ka, kb) = (a.compte(section, v), b.compte(section, v));
            let part = |k: u32, n: u32| 100.0 * f64::from(k) / f64::from(n.max(1));
            let marque = if ecart_significatif(ka, na, kb, nb) { "*" } else { "" };
            s += &format!("  {v:<28} {:>6.1} % {:>6.1} % {marque}\n", part(ka, na), part(kb, nb));
        }
    }
    s += "\nmoyennes\n";
    let grandeurs: BTreeSet<&&str> = a.moyennes.keys().chain(b.moyennes.keys()).collect();
    for g in grandeurs {
        let moyenne = |x: &Bilan| x.moyennes.get(*g).map_or(0.0, |(somme, n)| somme / f64::from((*n).max(1)));
        s += &format!("  {g:<42} {:>9.3} {:>9.3}\n", moyenne(a), moyenne(b));
    }
    s
}

/// **Lance le banc** et imprime le rapport (`BANC=<n>`).
pub fn lancer(n: u64) {
    let (mut v1, mut v2) = (Bilan::default(), Bilan::default());
    for g in 0..n {
        let (sys, info) = crate::genese::construire_systeme(g);
        v1.mesurer(&sys, &info);
        let (sys, info) = instancier(&planifier(g));
        v2.mesurer(&sys, &info);
    }
    println!("BANC : {n} graines, colonnes genese V1 puis genese V2 ; * = ecart significatif (3 sigma)");
    println!("{}", rapport(&v1, &v2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_type_se_lit_dans_le_libelle() {
        assert_eq!(type_etoile("Etoile Pulsar  -  44334 K"), Some("Pulsar"));
        assert_eq!(type_etoile("Etoile Naine blanche polluee (sodium)  -  20000 K"), Some("Naine blanche polluee"));
        assert_eq!(type_etoile("Binaire (aleatoire)"), None);
    }

    /// Le bruit ne déclenche pas, un vrai écart si : 50 % contre 51 % n'est rien
    /// sur 1 000 tirages et net sur 200 000.
    #[test]
    fn seul_un_vrai_ecart_est_signale() {
        assert!(!ecart_significatif(500, 1000, 510, 1000));
        assert!(ecart_significatif(100_000, 200_000, 102_000, 200_000));
        assert!(ecart_significatif(0, 1000, 50, 1000), "une valeur qu'une seule genèse produit");
        assert!(!ecart_significatif(0, 0, 5, 10), "section vide d'un côté");
        assert!(!ecart_significatif(10, 10, 10, 10), "aucune variance");
    }

    /// Une valeur qu'une seule genèse produit figure au rapport, à 0 % de l'autre
    /// côté : c'est exactement ce que le banc doit montrer.
    #[test]
    fn une_valeur_d_une_seule_genese_se_voit_au_rapport() {
        let (mut a, mut b) = (Bilan::default(), Bilan::default());
        for _ in 0..100 {
            a.compter("s", "commun");
            b.compter("s", "commun");
            b.compter("s", "nouveau");
            a.ajouter("g", 2.0);
            b.ajouter("g", 4.0);
        }
        let r = rapport(&a, &b);
        let ligne = r.lines().find(|l| l.contains("nouveau")).expect("valeur absente du rapport");
        assert!(ligne.contains("0.0 %") && ligne.contains("50.0 %") && ligne.ends_with('*'), "{ligne}");
        let commun = r.lines().find(|l| l.contains("commun")).unwrap();
        assert!(commun.contains("100.0 %") && commun.contains("50.0 %"), "{commun}");
        let g = r.lines().find(|l| l.trim_start().starts_with("g ")).unwrap();
        assert!(g.contains("2.000") && g.contains("4.000"), "{g}");
    }
}
