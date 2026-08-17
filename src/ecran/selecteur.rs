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

use crate::astre::{Categorie, Foyer};
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Ce qu'une ligne dessine à sa gauche pour matérialiser l'arbre.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Branche {
    /// Racine : une étoile, ou le barycentre d'un système multiple. Rien à
    /// gauche.
    Racine,
    /// Enfant, avec d'autres après lui — « ├─ ».
    Fils,
    /// **Dernier** enfant de son parent — « └─ ». Sans cette distinction,
    /// le trait vertical descendrait sous la dernière planète, vers rien.
    Dernier,
}

/// Une ligne du sélecteur.
pub struct Entree {
    /// Index de l'astre dans le système — ce que la caméra veut pour cadrer.
    /// `None` pour le **barycentre**, qui n'est pas un astre.
    pub idx: Option<usize>,
    /// Ce qui s'affiche : nom propre, ou désignation orbitale.
    pub libelle: String,
    /// 0 à la racine, 1 pour un corps qui orbite une racine.
    pub profondeur: usize,
    /// Forme du trait d'arbre à gauche.
    pub branche: Branche,
    pub categorie: Categorie,
}

impl Entree {
    /// Une entrée sélectionnable pointe un astre ; le barycentre, non.
    #[allow(dead_code)] // Interaction de l'assembleur (Lot 4) : deja teste, pas encore branche a une vue.
    pub fn selectionnable(&self) -> bool {
        self.idx.is_some()
    }
}

/// Les astres **sélectionnables**, en arbre : étoiles d'abord, chacune suivie
/// des corps qui l'orbitent.
///
/// Quatre décisions :
///
/// 1. **Ni lunes, ni ceintures, ni engins.** Le preset solaire compte 16
///    lunes pour 9 planètes : listées, elles noient la structure du système,
///    qui est ce que cette colonne sert à lire. Les ceintures, elles, ne sont
///    pas sélectionnables dans la vue (`Systeme::pick` les ignore). Les
///    engins (l'ISS) sont des objets du joueur, pas des astres : ce sélecteur
///    est celui des **planètes**, pas un inventaire.
/// 2. **La hiérarchie vient du foyer**, pas de la distance. Une planète de
///    type S se range sous son étoile hôte, une circumbinaire sous le
///    barycentre. C'est la seule chose qui rende lisible un système multiple.
/// 3. **Le barycentre n'apparaît que s'il le faut** : à une seule étoile, tout
///    se range sous elle et une racine « BARYCENTRE » n'apprendrait rien.
pub fn entrees(sys: &Systeme) -> Vec<Entree> {
    let mut v: Vec<Entree> = Vec::new();

    let etoiles: Vec<usize> = (0..sys.nb_astres())
        .filter(|i| sys.categorie_de(*i) == Some(Categorie::Etoile))
        .collect();
    let multiple = etoiles.len() > 1;

    // Corps qui orbitent directement une racine : une planète (jamais une
    // lune, qui a un `parent`). Les engins n'y figurent pas — voir la
    // décision 1 ci-dessus.
    let orbite_directement = |i: usize| matches!(sys.categorie_de(i), Some(Categorie::Planete));

    // Enfants d'une racine, classés par distance à l'astre central — sauf
    // Pluton, toujours en dernier.
    //
    // Son orbite, très excentrique, la fait parfois passer plus près du
    // Soleil que Neptune (ce que le jeu modélise avec la vraie excentricité,
    // 0,249) : un tri à la pure distance la ferait alors sauter avant
    // Neptune dans la colonne selon l'instant, plutôt que de rester à sa
    // place conventionnelle en bout de liste.
    let enfants = |foyer_attendu: Option<usize>| -> Vec<usize> {
        let centre = foyer_attendu.map_or(Vec3::ZERO, |e| sys.position(e));
        let mut v: Vec<usize> = (0..sys.nb_astres())
            .filter(|i| orbite_directement(*i))
            .filter(|i| racine_de(sys, *i, multiple) == foyer_attendu)
            .collect();
        v.sort_by(|a, b| {
            let pa = sys.designation(*a) == "Pluton";
            let pb = sys.designation(*b) == "Pluton";
            pa.cmp(&pb).then_with(|| {
                (sys.position(*a) - centre).length().total_cmp(&(sys.position(*b) - centre).length())
            })
        });
        v
    };

    let pousser = |v: &mut Vec<Entree>, idx: Option<usize>, libelle: String,
                       profondeur: usize, branche: Branche, cat: Categorie| {
        v.push(Entree { idx, libelle, profondeur, branche, categorie: cat });
    };

    for e in &etoiles {
        pousser(&mut v, Some(*e), sys.designation(*e), 0, Branche::Racine, Categorie::Etoile);
        let fils = enfants(Some(*e));
        for (k, f) in fils.iter().enumerate() {
            let b = if k + 1 == fils.len() { Branche::Dernier } else { Branche::Fils };
            let cat = sys.categorie_de(*f).unwrap_or(Categorie::Planete);
            pousser(&mut v, Some(*f), sys.designation(*f), 1, b, cat);
        }
    }

    // Circumbinaires : elles n'orbitent aucune étoile en particulier. Une racine
    // à part, et seulement s'il y en a.
    // Circumbinaires : elles n'orbitent aucune étoile en particulier, donc une
    // racine à part.
    //
    // Pas de garde `if multiple` ici : `racine_de` range **tout** sous l'unique
    // étoile quand il n'y en a qu'une, si bien que cette liste ne peut être non
    // vide que dans un système multiple. La garde existait, et aucun sabotage ne
    // pouvait l'atteindre — une branche morte qu'un lecteur croit vivante. Le
    // test `pas_de_circumbinaire_sans_plusieurs_etoiles` tient l'invariant.
    let libres = enfants(None);
    if !libres.is_empty() {
        debug_assert!(multiple, "des circumbinaires sans plusieurs étoiles");
        pousser(&mut v, None, "BARYCENTRE".to_string(), 0, Branche::Racine, Categorie::Etoile);
        for (k, f) in libres.iter().enumerate() {
            let b = if k + 1 == libres.len() { Branche::Dernier } else { Branche::Fils };
            let cat = sys.categorie_de(*f).unwrap_or(Categorie::Planete);
            pousser(&mut v, Some(*f), sys.designation(*f), 1, b, cat);
        }
    }
    v
}

/// Sous quelle racine se range l'astre `i` : `Some(etoile)` pour un type S,
/// `None` pour une circumbinaire.
///
/// À **une seule étoile**, tout se range sous elle : `Foyer::Barycentre` y
/// désigne le centre du système, qui est l'étoile elle-même.
fn racine_de(sys: &Systeme, i: usize, multiple: bool) -> Option<usize> {
    match sys.foyer_de(i) {
        Some(Foyer::Etoile(e)) => Some(e),
        _ if !multiple => (0..sys.nb_astres()).find(|k| sys.categorie_de(*k) == Some(Categorie::Etoile)),
        _ => None,
    }
}

/// Facteur d'échelle des lignes de la colonne, transmis à
/// `liste::hauteur_ligne_echelle`/`ligne_echelle`/`item_sous_curseur_echelle`.
/// Doublé (sur demande) pour laisser respirer l'icône rendue de chaque astre
/// — une simple pastille de couleur n'en avait pas besoin.
pub const ECHELLE_LIGNE: f32 = 2.0;

/// Part de la largeur d'écran occupée par la colonne du sélecteur, transmise
/// à `liste::colonne_items_depuis_echelle`/`colonne_depuis_echelle` — plus
/// large que `liste::PART_LARGEUR` (10 %, le plafond des **autres**
/// colonnes, dont la vitrine de composants, qui n'a pas grossi et n'a pas de
/// raison de suivre).
///
/// À 10 %, les icônes doublées ([`ECHELLE_ICONE`]) et le trait d'arbre à
/// droite de chacune ne laissaient quasiment plus de place au nom : mesuré à
/// l'écran, « Soleil »/« Mercure »/« Saturne »/« Uranus »/« Neptune »/
/// « Pluton » (les plus longs) rendaient tous « So… »/« Me… »/etc.
pub const PART_LARGEUR_COLONNE: f32 = 0.18;

/// Retrait horizontal d'une planète sous son étoile, en pixels — comme une
/// tabulation dans un fichier texte : ce que la ligne montre (icône, arbre,
/// nom) se décale d'un bloc plutôt que de rester à l'aplomb de la racine,
/// pour que la colonne se lise d'un coup d'œil comme une liste **sous** le
/// Soleil, pas une liste à plat.
pub const INDENT_TAB: f32 = 20.0;

/// Facteur d'échelle du bouton de repli (`«`/`»`) — doublé sur demande,
/// transmis à `liste::bouton_repli_echelle`/`zone_active_echelle_bouton`/
/// `colonne_items_depuis_echelle_bouton`. Propre au sélecteur : le bouton de
/// la vitrine de composants, qui partage le même module, n'a pas grossi.
pub const ECHELLE_BOUTON_REPLI: f32 = 2.0;

/// Taille de l'icône d'une **tellurique**, en fraction de celle d'une
/// **gazeuse** — la moitié : deux mondes de rayons physiques très différents
/// (une Terre, une Jupiter) ne doivent pas peser pareil dans la colonne. Les
/// autres catégories (étoile, lune, ceinture, engin) gardent la taille pleine.
pub const RATIO_TELLURIQUE: f32 = 0.5;

/// Facteur d'agrandissement de **toutes** les icônes, tellurique ou gazeuse
/// — doublé sur demande, appliqué après le rapport tellurique/gazeuse pour
/// ne pas y toucher : une tellurique reste la moitié d'une gazeuse, la paire
/// grandit ensemble.
pub const ECHELLE_ICONE: f32 = 2.0;

/// Rayon de l'icône d'un astre, en pixels, pour une ligne de hauteur `h`.
/// `gazeuse` vient de [`crate::systeme::Systeme::gazeuse_de`] ; `None` (ce qui
/// n'a pas de type planétaire) garde la taille pleine, comme une gazeuse.
pub fn rayon_icone(h: f32, gazeuse: Option<bool>) -> f32 {
    let plein = (h * 0.42).clamp(6.0, 16.0);
    let base = if gazeuse == Some(false) { plein * RATIO_TELLURIQUE } else { plein };
    base * ECHELLE_ICONE
}

/// Couleur de repli, quand l'astre n'a pas de teinte propre.
///
/// Ne dit que la **catégorie**. Sert aux ceintures et aux corps d'essai ; les
/// vraies planètes passent par [`teinte_astre`], qui lit leur apparence.
pub fn teinte_categorie(cat: Categorie) -> Color {
    match cat {
        Categorie::Etoile => Color::new(1.0, 0.85, 0.35, 1.0),
        Categorie::Planete => Color::new(0.45, 0.70, 0.95, 1.0),
        Categorie::Lune => Color::new(0.72, 0.74, 0.78, 1.0),
        Categorie::Asteroide | Categorie::Comete => Color::new(0.55, 0.50, 0.45, 1.0),
        Categorie::Engin => Color::new(0.80, 0.82, 0.86, 1.0),
    }
}

/// **Pastille d'un astre** : sa teinte réelle, tirée de son apparence.
///
/// C'était la dette D-INT-1 : une couleur par catégorie faisait de toutes les
/// planètes du système la même pastille bleue. Elle vient maintenant du corps
/// lui-même — un monde-océan lit bleu, un désert ocre, Mars rouille — si bien
/// qu'une retouche d'apparence se voit aussitôt dans la colonne, sans seconde
/// table à tenir à jour.
///
/// **Relevée** vers le clair : ces couleurs sont des albédos de surface, faites
/// pour être éclairées par une étoile. Posées telles quelles sur un fond sombre,
/// les plus mates rendraient une pastille presque noire.
pub fn teinte_astre(sys: &Systeme, idx: usize, cat: Categorie) -> Color {
    match sys.teinte_de(idx) {
        Some(c) => {
            let c = relever(c);
            Color::new(c.x, c.y, c.z, 1.0)
        }
        None => teinte_categorie(cat),
    }
}

/// Remonte une couleur sombre sans toucher aux vives : la pastille doit se voir
/// sur fond nuit, mais un monde déjà clair ne doit pas être délavé.
fn relever(c: Vec3) -> Vec3 {
    const PLANCHER: f32 = 0.42;
    let max = c.x.max(c.y).max(c.z);
    if max <= 1e-4 {
        return Vec3::splat(PLANCHER);
    }
    if max >= PLANCHER {
        return c.clamp(Vec3::ZERO, Vec3::ONE);
    }
    (c * (PLANCHER / max)).clamp(Vec3::ZERO, Vec3::ONE)
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
        foyer: Option<Foyer>,
        teinte: Option<Vec3>,
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
        fn foyer(&self) -> Option<Foyer> {
            self.foyer
        }
        fn teinte(&self) -> Option<Vec3> {
            self.teinte
        }
        fn update(&mut self, _dt: f32) {}
        fn draw(&mut self, _cam: &CameraInfo) {}
    }

    fn poser(sys: &mut Systeme, cat: Categorie, x: f32, parent: Option<usize>) -> usize {
        poser_complet(sys, cat, x, parent, None, None)
    }

    fn poser_teinte(
        sys: &mut Systeme,
        cat: Categorie,
        x: f32,
        parent: Option<usize>,
        teinte: Option<Vec3>,
    ) -> usize {
        poser_complet(sys, cat, x, parent, None, teinte)
    }

    fn poser_complet(
        sys: &mut Systeme,
        cat: Categorie,
        x: f32,
        parent: Option<usize>,
        foyer: Option<Foyer>,
        teinte: Option<Vec3>,
    ) -> usize {
        sys.ajouter(Box::new(CorpsEssai {
            base: CorpsBase::new(vec3(x, 0.0, 0.0), 1.0, 1.0),
            cat,
            parent,
            foyer,
            teinte,
        }))
    }

    /// Système à une étoile : trois planètes **déclarées dans le désordre**,
    /// deux lunes, une ceinture.
    fn systeme_essai() -> (Systeme, [usize; 7]) {
        let mut sys = Systeme::new();
        let etoile = poser(&mut sys, Categorie::Etoile, 0.0, None);
        let loin = poser_complet(&mut sys, Categorie::Planete, 900.0, None, Some(Foyer::Barycentre), None);
        let pres = poser_complet(&mut sys, Categorie::Planete, 100.0, None, Some(Foyer::Barycentre), None);
        let milieu = poser_complet(&mut sys, Categorie::Planete, 400.0, None, Some(Foyer::Barycentre), None);
        let lune_ext = poser(&mut sys, Categorie::Lune, 460.0, Some(milieu));
        let lune_int = poser(&mut sys, Categorie::Lune, 420.0, Some(milieu));
        let ceinture = poser(&mut sys, Categorie::Asteroide, 600.0, None);
        (sys, [etoile, loin, pres, milieu, lune_ext, lune_int, ceinture])
    }

    fn idx(v: &[Entree]) -> Vec<Option<usize>> {
        v.iter().map(|e| e.idx).collect()
    }

    // **L'étoile est la racine, les planètes sont dessous, par distance.**
    // Elles sont déclarées loin → près → milieu ; la liste doit sortir
    // près → milieu → loin.
    #[test]
    fn letoile_est_la_racine_et_les_planetes_suivent_par_distance() {
        let (sys, [etoile, loin, pres, milieu, ..]) = systeme_essai();
        let v = entrees(&sys);
        assert_eq!(idx(&v), vec![Some(etoile), Some(pres), Some(milieu), Some(loin)]);
        assert_eq!(v[0].profondeur, 0, "l'étoile doit être à la racine");
        assert!(v[1..].iter().all(|e| e.profondeur == 1), "les planètes sont sous l'étoile");
    }

    // **Pluton reste en dernier, même posé plus près du Soleil que Neptune**
    // — son orbite réelle, très excentrique (0,249), le lui permet vraiment
    // (voir le commentaire sur `enfants` dans `entrees`) : un tri à la pure
    // distance le ferait alors sauter devant Neptune selon l'instant.
    #[test]
    fn pluton_reste_dernier_meme_plus_proche_que_neptune() {
        let mut sys = Systeme::new();
        let etoile = poser(&mut sys, Categorie::Etoile, 0.0, None);
        let neptune = poser_complet(&mut sys, Categorie::Planete, 300.0, None, Some(Foyer::Barycentre), None);
        // Pluton posé PLUS PRES que Neptune : la preuve que ce n'est pas la
        // distance qui le place en dernier.
        let pluton = poser_complet(&mut sys, Categorie::Planete, 100.0, None, Some(Foyer::Barycentre), None);
        sys.nommer(neptune, "Neptune");
        sys.nommer(pluton, "Pluton");
        let v = entrees(&sys);
        assert_eq!(idx(&v), vec![Some(etoile), Some(neptune), Some(pluton)], "Pluton devrait rester dernier");
    }

    // **Les lunes ne se listent plus.** Le preset solaire en compte 16 pour 9
    // planètes : listées, elles noient la structure du système, qui est ce que
    // cette colonne sert à lire.
    #[test]
    fn les_lunes_ne_sont_plus_listees() {
        let (sys, [_, _, _, _, lune_ext, lune_int, _]) = systeme_essai();
        let v = entrees(&sys);
        assert!(!idx(&v).contains(&Some(lune_ext)), "une lune est listée");
        assert!(!idx(&v).contains(&Some(lune_int)), "une lune est listée");
        assert!(
            v.iter().all(|e| e.categorie != Categorie::Lune),
            "une entrée de catégorie lune s'est glissée"
        );
    }

    // Les ceintures non plus : `Systeme::pick` les ignore, donc une entrée de
    // ceinture serait cliquable dans la colonne et jamais dans la vue.
    #[test]
    fn les_ceintures_ne_sont_pas_listees() {
        let (sys, [.., ceinture]) = systeme_essai();
        let v = entrees(&sys);
        assert!(!idx(&v).contains(&Some(ceinture)), "la ceinture est dans la liste");
        assert!(v.iter().all(|e| e.categorie != Categorie::Asteroide));
    }

    // **Un système multiple range chaque planète sous son étoile hôte**, et les
    // circumbinaires sous un barycentre à part. C'était le défaut signalé : à
    // plat, on ne voyait pas qui orbite quoi.
    #[test]
    fn un_systeme_multiple_range_chaque_planete_sous_son_etoile() {
        let mut sys = Systeme::new();
        let a = poser(&mut sys, Categorie::Etoile, -20.0, None);
        let b = poser(&mut sys, Categorie::Etoile, 20.0, None);
        let pa = poser_complet(&mut sys, Categorie::Planete, 60.0, None, Some(Foyer::Etoile(a)), None);
        let pb = poser_complet(&mut sys, Categorie::Planete, 90.0, None, Some(Foyer::Etoile(b)), None);
        let circum = poser_complet(&mut sys, Categorie::Planete, 900.0, None, Some(Foyer::Barycentre), None);

        let v = entrees(&sys);
        let noms: Vec<Option<usize>> = idx(&v);
        // A, sa planète, B, sa planète, puis le barycentre et la circumbinaire.
        assert_eq!(noms, vec![Some(a), Some(pa), Some(b), Some(pb), None, Some(circum)]);
        // Le barycentre est une ligne d'arbre, pas un astre.
        assert!(!v[4].selectionnable(), "le barycentre ne doit pas être sélectionnable");
        assert_eq!(v[4].libelle, "BARYCENTRE");
        assert_eq!(v[5].profondeur, 1, "la circumbinaire est sous le barycentre");
    }

    // **Un système à une étoile a exactement une racine, et c'est l'étoile.**
    // Tout se range dessous : ni barycentre fantôme, ni planète promue racine
    // faute d'avoir trouvé son hôte.
    //
    // ⚠️ Première version : elle vérifiait « aucune entrée non sélectionnable »
    // et « aucun libellé BARYCENTRE ». Les deux restaient vraies quand les
    // planètes cessaient de trouver leur étoile — elles devenaient alors des
    // racines *sélectionnables*, sans barycentre pour autant. Le test nommait
    // une propriété et en mesurait une autre. Compter les racines la capture.
    #[test]
    fn un_systeme_a_une_etoile_na_quune_racine() {
        let (sys, [etoile, ..]) = systeme_essai();
        let v = entrees(&sys);
        let racines: Vec<&Entree> = v.iter().filter(|e| e.profondeur == 0).collect();
        assert_eq!(racines.len(), 1, "{} racines au lieu d'une seule", racines.len());
        assert_eq!(racines[0].idx, Some(etoile), "la racine devrait être l'étoile");
        assert!(v.iter().all(|e| e.selectionnable()), "une racine fantôme dans un système simple");
        assert!(v.iter().all(|e| e.libelle != "BARYCENTRE"));
    }

    // **L'invariant qui justifie l'absence de garde** : une liste de
    // circumbinaires ne peut pas exister sans plusieurs étoiles, parce que
    // `racine_de` range tout sous l'unique étoile quand il n'y en a qu'une.
    //
    // Sans ce test, la simplification (retrait du `if multiple`) reposerait sur
    // un raisonnement, pas sur une mesure.
    #[test]
    fn pas_de_circumbinaire_sans_plusieurs_etoiles() {
        // Une seule étoile, trois planètes au foyer barycentre : aucune ne doit
        // se retrouver « libre ».
        let (sys, _) = systeme_essai();
        assert_eq!(sys.nb_etoiles(), 1);
        let v = entrees(&sys);
        assert!(v.iter().all(|e| e.selectionnable()), "un barycentre est apparu");

        // Et le cas symétrique : à deux étoiles, une planète au barycentre en
        // produit bien un.
        let mut duo = Systeme::new();
        let a = poser(&mut duo, Categorie::Etoile, -20.0, None);
        poser(&mut duo, Categorie::Etoile, 20.0, None);
        poser_complet(&mut duo, Categorie::Planete, 900.0, None, Some(Foyer::Barycentre), None);
        let _ = a;
        assert!(entrees(&duo).iter().any(|e| e.libelle == "BARYCENTRE"));
    }

    // **Le dernier enfant se dessine autrement.** Sans cette distinction, le
    // trait vertical de l'arbre descendrait sous la dernière planète vers rien.
    #[test]
    fn le_dernier_enfant_ferme_la_branche() {
        let (sys, _) = systeme_essai();
        let v = entrees(&sys);
        assert_eq!(v[0].branche, Branche::Racine);
        let fils: Vec<Branche> = v[1..].iter().map(|e| e.branche).collect();
        assert_eq!(fils, vec![Branche::Fils, Branche::Fils, Branche::Dernier]);
    }

    // Un engin (l'ISS) ne se liste pas : ce sélecteur est celui des
    // **planètes**, pas un inventaire des objets du joueur.
    #[test]
    fn un_engin_ne_se_liste_pas() {
        let (mut sys, [_, _, pres, ..]) = systeme_essai();
        let iss = poser(&mut sys, Categorie::Engin, 105.0, Some(pres));
        let v = entrees(&sys);
        assert!(!idx(&v).contains(&Some(iss)), "l'engin est listé");
        assert!(v.iter().all(|e| e.categorie != Categorie::Engin), "une entrée de catégorie engin s'est glissée");
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
            assert!(!e.libelle.is_empty(), "entrée {:?} sans libellé", e.idx);
            assert!(vus.insert(e.libelle.clone()), "« {} » listé deux fois", e.libelle);
        }
        assert!(v.iter().any(|e| e.libelle == "Terre"));
    }

    // **La pastille vient du corps, pas de sa catégorie.** C'était la dette
    // D-INT-1 : toutes les planètes rendaient la même pastille bleue.
    //
    // ⚠️ Première version : elle ne posait que des corps **sans** apparence, si
    // bien que les deux chemins rendaient la même couleur — restée verte au
    // sabotage. Un test qui ne visite qu'une branche ne dit rien de l'autre.
    #[test]
    fn la_pastille_vient_du_corps_et_non_de_sa_categorie() {
        let mut sys = Systeme::new();
        let rouge = poser_teinte(&mut sys, Categorie::Planete, 100.0, None, Some(vec3(0.8, 0.1, 0.1)));
        let bleue = poser_teinte(&mut sys, Categorie::Planete, 200.0, None, Some(vec3(0.1, 0.2, 0.9)));
        let ta = teinte_astre(&sys, rouge, Categorie::Planete);
        let tb = teinte_astre(&sys, bleue, Categorie::Planete);
        assert!(ta.r > ta.b, "la rouge devrait tirer au rouge : {ta:?}");
        assert!(tb.b > tb.r, "la bleue devrait tirer au bleu : {tb:?}");
        let cat = teinte_categorie(Categorie::Planete);
        assert_ne!(ta, cat, "la rouge rend la teinte de catégorie");
        assert_ne!(tb, cat, "la bleue rend la teinte de catégorie");
    }

    // Le **repli** doit marcher aussi : sans apparence propre, on retombe sur
    // la catégorie plutôt que sur du noir.
    #[test]
    fn la_pastille_retombe_sur_la_categorie_sans_apparence() {
        let (sys, [etoile, _, pres, ..]) = systeme_essai();
        assert_eq!(teinte_astre(&sys, pres, Categorie::Planete), teinte_categorie(Categorie::Planete));
        assert_eq!(teinte_astre(&sys, etoile, Categorie::Etoile), teinte_categorie(Categorie::Etoile));
        assert_eq!(teinte_astre(&sys, 9999, Categorie::Lune), teinte_categorie(Categorie::Lune));
    }

    // **Une pastille se voit sur fond nuit.** Les couleurs d'apparence sont des
    // albédos de surface : posés tels quels, les plus mats donneraient un disque
    // presque noir.
    #[test]
    fn les_teintes_sombres_sont_relevees_sans_delaver_les_vives() {
        let sombre = relever(vec3(0.04, 0.05, 0.03));
        assert!(sombre.max_element() >= 0.4, "trop sombre : {sombre}");
        let brute = vec3(0.10, 0.05, 0.02);
        let r = relever(brute);
        assert!((r.x / r.y - brute.x / brute.y).abs() < 1e-3, "la teinte a viré : {r}");
        let claire = vec3(0.9, 0.8, 0.7);
        assert!((relever(claire) - claire).length() < 1e-5, "une couleur vive a été modifiée");
        assert!(relever(Vec3::ZERO).max_element() > 0.0);
        for c in [vec3(2.0, 0.5, 0.1), Vec3::ZERO, vec3(0.01, 0.0, 0.0)] {
            let r = relever(c);
            assert!(r.min_element() >= 0.0 && r.max_element() <= 1.0, "{c} -> {r}");
        }
    }

    // Chaque catégorie a une teinte de repli, et elles se distinguent entre
    // elles — `teinte_categorie` sert aussi hors de ce sélecteur (la fiche).
    #[test]
    fn les_categories_ont_des_teintes_distinctes() {
        let couleurs = [
            teinte_categorie(Categorie::Etoile),
            teinte_categorie(Categorie::Planete),
            teinte_categorie(Categorie::Engin),
        ];
        for (i, a) in couleurs.iter().enumerate() {
            for b in couleurs.iter().skip(i + 1) {
                let ecart = (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs();
                assert!(ecart > 0.2, "deux teintes trop proches : {ecart}");
            }
        }
    }

    // **Une tellurique se dessine à la moitié d'une gazeuse**, exactement —
    // c'est la demande, et le point de tout ce mécanisme : sans lui, une Terre
    // et une Jupiter pèseraient pareil dans la colonne.
    #[test]
    fn une_tellurique_fait_la_moitie_dune_gazeuse() {
        for h in [20.0_f32, 40.0, 64.0] {
            let gazeuse = rayon_icone(h, Some(true));
            let tellurique = rayon_icone(h, Some(false));
            assert!((tellurique / gazeuse - RATIO_TELLURIQUE).abs() < 1e-4, "h={h} : {tellurique} vs {gazeuse}");
        }
    }

    // **Ce qui n'a pas de type planétaire garde la taille pleine** — une
    // étoile ou une lune ne doit pas rétrécir faute d'une réponse gazeuse/
    // tellurique qu'elle ne peut pas donner.
    #[test]
    fn sans_type_planetaire_la_taille_reste_pleine() {
        let h = 40.0_f32;
        assert_eq!(rayon_icone(h, None), rayon_icone(h, Some(true)), "None devrait valoir la taille pleine");
    }

    // **Toutes les icônes doublent, tellurique et gazeuse ensemble** — sur
    // demande explicite. Le rapport entre les deux (`RATIO_TELLURIQUE`) doit
    // survivre au doublement, sinon une tellurique cesserait de peser la
    // moitié d'une gazeuse une fois agrandie.
    //
    // ⚠️ Comparé à **2,0 en dur**, pas à `ECHELLE_ICONE` : se mesurer contre
    // sa propre constante, c'est réciter la formule, et le test suivrait
    // n'importe quelle valeur qu'on lui donnerait sans jamais rougir.
    #[test]
    fn toutes_les_icones_doublent_ensemble() {
        assert_eq!(ECHELLE_ICONE, 2.0, "la demande était bien un doublement");
        let h = 40.0_f32;
        let plein_avant_echelle = (h * 0.42_f32).clamp(6.0, 16.0);
        assert_eq!(rayon_icone(h, Some(true)), plein_avant_echelle * 2.0);
        assert_eq!(rayon_icone(h, Some(false)), plein_avant_echelle * RATIO_TELLURIQUE * 2.0);
    }
}
