//! **Sélecteur d'astres** : la colonne de gauche de la vue système.
//!
//! Le schéma d'interface la montre comme une bande étroite de pastilles, une par
//! astre, repliable par un `«` en bas (`docs/conception/interface.md` §1.2, ⓒ).
//!
//! # Ce qui est ici, et ce qui n'y est pas
//!
//! Ce module décide **quels astres se listent et dans quel ordre**. La
//! géométrie — rectangle de colonne, hauteur de ligne, item sous le curseur —
//! vit dans [`super::liste`], qui ne connaît ni astres ni composants et sert
//! déjà à la vue des briques. Le **dessin** est dans la vue.
//!
//! C'est la règle de `conception/assembleur.md` §10.9 : tout ce qui se décide
//! sort du code de dessin, vers des requêtes qu'on peut casser exprès pour voir
//! si un test rougit.

use crate::astre::Categorie;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Une ligne du sélecteur.
pub struct Entree {
    /// Index de l'astre dans le système — ce que la caméra veut pour cadrer.
    pub idx: usize,
    /// Ce qui s'affiche : nom propre, ou désignation orbitale.
    pub libelle: String,
    /// 0 pour une étoile ou une planète, 1 pour une lune. Sert au retrait.
    pub profondeur: usize,
    pub categorie: Categorie,
}

/// Les astres **sélectionnables**, dans l'ordre d'affichage.
///
/// Trois décisions, toutes visibles dans le code ci-dessous :
///
/// 1. **Les ceintures sont exclues.** Un champ de débris n'est pas un lieu où
///    l'on va ; `Systeme::pick` les ignore déjà, donc les lister donnerait une
///    entrée qu'un clic dans la vue ne pourrait jamais désigner en retour.
/// 2. **L'ordre est celui du système, pas celui de l'ajout** : étoiles d'abord,
///    puis les planètes par distance croissante, chacune suivie de ses lunes.
///    C'est ainsi qu'on lit une carte de système, et c'est le même classement
///    que celui qui donne les chiffres romains (`Systeme::designation`).
/// 3. **Les lunes sont listées**, en retrait. Les exclure cacherait 16 des 26
///    corps du preset solaire ; les mettre à plat ferait perdre à quelle planète
///    elles appartiennent.
pub fn entrees(sys: &Systeme) -> Vec<Entree> {
    let mut v = Vec::new();

    let pousser = |v: &mut Vec<Entree>, idx: usize, profondeur: usize| {
        v.push(Entree {
            idx,
            libelle: sys.designation(idx),
            profondeur,
            categorie: sys.categorie_de(idx).unwrap_or(Categorie::Planete),
        });
    };

    // Les étoiles d'abord : elles sont le centre, et le système porte leur nom.
    for i in 0..sys.nb_astres() {
        if sys.categorie_de(i) == Some(Categorie::Etoile) {
            pousser(&mut v, i, 0);
        }
    }

    // Puis les planètes, **par distance**, chacune suivie de ses lunes.
    let mut planetes: Vec<usize> = (0..sys.nb_astres())
        .filter(|i| sys.categorie_de(*i) == Some(Categorie::Planete) && sys.parent_de(*i).is_none())
        .collect();
    planetes.sort_by(|a, b| sys.position(*a).length().total_cmp(&sys.position(*b).length()));

    for p in planetes {
        pousser(&mut v, p, 0);
        let mut lunes: Vec<usize> = (0..sys.nb_astres())
            .filter(|i| sys.parent_de(*i) == Some(p))
            .collect();
        let centre = sys.position(p);
        lunes.sort_by(|a, b| {
            (sys.position(*a) - centre).length().total_cmp(&(sys.position(*b) - centre).length())
        });
        for l in lunes {
            pousser(&mut v, l, 1);
        }
    }
    v
}

/// Couleur de la pastille d'une entrée.
///
/// ⚠️ **Bouche-trou.** Le schéma montre une vignette de l'astre ; en attendant
/// un rendu miniature, on pose un disque uni dont la teinte ne dit que la
/// catégorie. Voir la dette D-INT-1 dans `STATE.md`.
pub fn teinte(cat: Categorie) -> Color {
    match cat {
        Categorie::Etoile => Color::new(1.0, 0.85, 0.35, 1.0),
        Categorie::Planete => Color::new(0.45, 0.70, 0.95, 1.0),
        Categorie::Lune => Color::new(0.72, 0.74, 0.78, 1.0),
        Categorie::Asteroide | Categorie::Comete => Color::new(0.55, 0.50, 0.45, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astre::{Astre, CameraInfo, CorpsBase};

    // Même corps d'essai que `systeme::tests_designation`, et pour la même
    // raison : `genese` tire ses aléas par `macroquad::rand`, qui exige le
    // contexte graphique — aucun test ne peut bâtir un vrai système
    // (`conception/interface.md` §5.1 bis).
    struct CorpsEssai {
        base: CorpsBase,
        cat: Categorie,
        parent: Option<usize>,
    }

    impl Astre for CorpsEssai {
        fn categorie(&self) -> Categorie {
            self.cat
        }
        fn corps(&self) -> &CorpsBase {
            &self.base
        }
        fn corps_mut(&mut self) -> &mut CorpsBase {
            &mut self.base
        }
        fn parent(&self) -> Option<usize> {
            self.parent
        }
        fn update(&mut self, _dt: f32) {}
        fn draw(&mut self, _cam: &CameraInfo) {}
    }

    fn poser(sys: &mut Systeme, cat: Categorie, x: f32, parent: Option<usize>) -> usize {
        sys.ajouter(Box::new(CorpsEssai {
            base: CorpsBase::new(vec3(x, 0.0, 0.0), 1.0, 1.0),
            cat,
            parent,
        }))
    }

    /// Un système d'essai : une étoile, trois planètes **déclarées dans le
    /// désordre**, deux lunes sur la planète du milieu, une ceinture.
    fn systeme_essai() -> (Systeme, [usize; 7]) {
        let mut sys = Systeme::new();
        let etoile = poser(&mut sys, Categorie::Etoile, 0.0, None);
        let loin = poser(&mut sys, Categorie::Planete, 900.0, None);
        let pres = poser(&mut sys, Categorie::Planete, 100.0, None);
        let milieu = poser(&mut sys, Categorie::Planete, 400.0, None);
        let lune_ext = poser(&mut sys, Categorie::Lune, 460.0, Some(milieu));
        let lune_int = poser(&mut sys, Categorie::Lune, 420.0, Some(milieu));
        let ceinture = poser(&mut sys, Categorie::Asteroide, 600.0, None);
        (sys, [etoile, loin, pres, milieu, lune_ext, lune_int, ceinture])
    }

    // **L'ordre est celui du système, pas celui de l'ajout.** Les planètes sont
    // déclarées loin → près → milieu ; la liste doit sortir près → milieu → loin,
    // avec les lunes accrochées à leur planète.
    #[test]
    fn les_astres_sortent_dans_lordre_du_systeme() {
        let (sys, [etoile, loin, pres, milieu, lune_ext, lune_int, _]) = systeme_essai();
        let v: Vec<usize> = entrees(&sys).iter().map(|e| e.idx).collect();
        assert_eq!(v, vec![etoile, pres, milieu, lune_int, lune_ext, loin]);
    }

    // **Les ceintures ne se listent pas.** `Systeme::pick` les ignore, donc une
    // entrée de ceinture serait sélectionnable dans la colonne et jamais dans la
    // vue — une asymétrie que rien n'expliquerait à l'écran.
    #[test]
    fn les_ceintures_ne_sont_pas_listees() {
        let (sys, [.., ceinture]) = systeme_essai();
        let v = entrees(&sys);
        assert!(v.iter().all(|e| e.idx != ceinture), "la ceinture est dans la liste");
        assert!(
            v.iter().all(|e| e.categorie != Categorie::Asteroide),
            "une entrée de catégorie ceinture s'est glissée"
        );
    }

    // Les lunes sont **en retrait**, les planètes et étoiles non : c'est ce qui
    // dit à quelle planète une lune appartient, une fois la liste à plat.
    #[test]
    fn seules_les_lunes_sont_en_retrait() {
        let (sys, _) = systeme_essai();
        for e in entrees(&sys) {
            let attendue = usize::from(e.categorie == Categorie::Lune);
            assert_eq!(e.profondeur, attendue, "{} ({:?})", e.libelle, e.categorie);
        }
    }

    // Chaque entrée porte un libellé **non vide et unique** : deux lignes
    // identiques dans la colonne seraient indiscernables au clic.
    #[test]
    fn les_libelles_sont_non_vides_et_uniques() {
        let (mut sys, [_, _, pres, ..]) = systeme_essai();
        sys.nommer(pres, "Terre");
        let v = entrees(&sys);
        let mut vus = std::collections::HashSet::new();
        for e in &v {
            assert!(!e.libelle.is_empty(), "entrée {} sans libellé", e.idx);
            assert!(vus.insert(e.libelle.clone()), "« {} » listé deux fois", e.libelle);
        }
        // Et le nom propre est bien passé jusqu'ici.
        assert!(v.iter().any(|e| e.libelle == "Terre"));
    }

    // Chaque catégorie a une teinte, et **les trois qui se listent se
    // distinguent** : la pastille est aujourd'hui la seule chose qui différencie
    // une lune d'une planète au premier coup d'œil.
    #[test]
    fn les_categories_listees_ont_des_teintes_distinctes() {
        let couleurs = [
            teinte(Categorie::Etoile),
            teinte(Categorie::Planete),
            teinte(Categorie::Lune),
        ];
        for (i, a) in couleurs.iter().enumerate() {
            for b in couleurs.iter().skip(i + 1) {
                let ecart = (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs();
                assert!(ecart > 0.2, "deux teintes trop proches : {ecart}");
            }
        }
    }
}
