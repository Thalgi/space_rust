//! **Catalogue des items de la vue station** : une table, et une seule.
//!
//! Avant cette table, la même information vivait à **quatre endroits
//! indépendants** (`docs/conception/assembleur.md` §5.1) :
//!
//! 1. le compte (`Categorie::Briques => 27`, écrit à la main) ;
//! 2. les bras d'un `match i` géant qui construisaient l'item ;
//! 3. des constantes d'indice nommées (`BRIQUE_RADIATEUR = 6`,
//!    `BRIQUE_EQUIPAGE = 20`, `MEGA_ISV = [1]`) qui disaient quels boutons
//!    activer ;
//! 4. un `match` séparé qui disait quelle épine portait l'ISV complet.
//!
//! Et un **attrape-tout `_ =>`** au bout du `match`, qui absorbait
//! silencieusement toute entrée en trop : trois indices de menu pouvaient
//! afficher la même brique sans qu'aucune erreur ne remonte nulle part.
//!
//! Insérer une brique au milieu décalait tout et ne cassait **rien** — ni la
//! compilation, ni un test (il n'y en avait aucun sur `ecran/`). Le bouton
//! « allumer » se retrouvait simplement sur la mauvaise pièce.
//!
//! ⚠️ **La leçon vient du chantier ISV** (`suivi/stations.md` §C.29) : une
//! valeur qui a plusieurs sources finit par diverger. Ici la correction est
//! structurelle plutôt que défensive — les capacités d'un item ne sont plus
//! *déclarées à côté* de lui par son indice, elles se **déduisent de ce qu'il
//! est fait**. Un item bâti par [`Fabrique::Regime`] a une propulsion à
//! allumer parce qu'il lit le régime, pas parce qu'une constante le dit.
//! Il n'y a donc plus rien à tenir d'accord.

use crate::vaisseau::{
    cuire, echantillons, Assembleur, Composant, Repere,
    demo_anneaux, demo_antennes, demo_bouclier_grand, demo_bouclier_petit, demo_bouclier_thermique,
    demo_caissons, demo_cargo, demo_chantier, demo_charpente, demo_charpente_hexa, demo_coiffes,
    demo_epine_pavillon, demo_equipage, demo_panneaux_mega, demo_raptor, demo_habitat_isv, demo_habitats, demo_moteur_antimatiere,
    demo_moteur_antimatiere_principal, demo_panneaux, demo_poutres, demo_propulsion,
    demo_radiateur_mega, demo_radiateurs, demo_reservoir, demo_station, demo_treillis,
    preset_anneau, preset_elysium, preset_stanford, preset_starship, preset_comsat, preset_iss, preset_isv_equipage, preset_isv_fixe,
    preset_isv_moteur, preset_mir, preset_sonde, preset_tiangong, Epine, EtatStation,
    FamillePropulsion, Station, ISV_AXE,
};
use macroquad::prelude::Vec3;

/// Réglages d'animation que **certains** items lisent au moment d'être bâtis.
///
/// Regroupés en un seul type plutôt que passés en paramètres séparés : la table
/// est uniforme, et ajouter un troisième réglage un jour ne touchera pas les
/// 27 entrées.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Reglages {
    /// Repli de la section d'équipage : 0 déployée, 1 repliée.
    pub repli: f32,
    /// Régime moteur : 0 à l'arrêt, 1 à pleine poussée.
    pub regime: f32,
}

/// Ce qu'un item produit une fois bâti.
pub struct Bati {
    pub etat: EtatStation,
    /// **Moitié tournante**, quand l'item en dissocie une. Séparée pour que la
    /// rotation et le repli n'agissent que sur elle, et pour n'avoir à recuire
    /// qu'elle pendant l'animation (`suivi/stations.md` §C.7).
    pub tournant: Option<EtatStation>,
}

impl Bati {
    fn seul(etat: EtatStation) -> Self {
        Self { etat, tournant: None }
    }
}

/// **Comment** un item se bâtit — et, par voie de conséquence, ce qu'il sait
/// faire.
///
/// C'est le cœur du dispositif : les capacités ne sont pas une colonne de plus
/// dans la table (qu'il faudrait tenir d'accord avec le constructeur), elles
/// **découlent** de la variante. Un item qui ne lit pas le régime n'a pas de
/// propulsion à allumer, par construction, et aucune retouche ne peut rendre
/// les deux incohérents.
#[derive(Clone)]
pub enum Fabrique {
    /// Brique statique qui rend une `Station` déjà valide.
    Brique(fn() -> Station),
    /// Item statique qui rend un `EtatStation` (il peut être vide).
    Etat(fn() -> EtatStation),
    /// Vitrine de propulsion : même constructeur, une famille par item.
    Propulsion(FamillePropulsion),
    /// Item qui lit le **repli** → a donc une section à replier et à tourner.
    Repli(fn(f32) -> EtatStation),
    /// Item qui lit le **régime** → a donc une propulsion à allumer.
    Regime(fn(f32) -> EtatStation),
    /// **ISV complet** : le seul item en deux moitiés (coque fixe + section
    /// d'équipage), et le seul à porter une épine. Il lit les deux réglages.
    Isv(Epine),
    /// Habitat **tournant d'un seul tenant** : un anneau qui tourne autour de
    /// son axe pour faire de la gravité. Rien à replier ni à allumer — il n'a
    /// qu'un degré de liberté, et il s'applique à la station **entière**, pas à
    /// une moitié comme sur l'ISV.
    Rotatif(fn() -> EtatStation),
    /// **Une pièce seule**, telle quelle.
    ///
    /// C'est la fabrique des entrées **engendrées** : le catalogue en produit
    /// une par variante de `Composant`, tirée de la chaîne d'échantillons de
    /// L1.2. Contrairement aux autres, elle porte une *valeur* et non un
    /// pointeur de fonction — c'est ce qui permet de la construire au vol.
    Piece(Composant),
}

impl Fabrique {
    fn batir(&self, r: Reglages) -> Bati {
        match self {
            Fabrique::Brique(f) => Bati::seul(EtatStation::Prete(f())),
            Fabrique::Etat(f) | Fabrique::Rotatif(f) => Bati::seul(f()),
            Fabrique::Piece(c) => {
                let mut asm = Assembleur::new();
                asm.ajouter(cuire(Repere::IDENTITE, c));
                Bati::seul(asm.terminer())
            }
            Fabrique::Propulsion(famille) => {
                Bati::seul(EtatStation::Prete(demo_propulsion(*famille)))
            }
            Fabrique::Repli(f) => Bati::seul(f(r.repli)),
            Fabrique::Regime(f) => Bati::seul(f(r.regime)),
            // La section d'équipage doit être bâtie avec **la même épine** que
            // la coque : son alésage de collier se déduit du gabarit, et la
            // reconstruire avec l'autre variante la décalerait de 3,2 % — assez
            // pour que le collier morde dans la flèche ou s'en détache
            // (`suivi/stations.md` §C.10). Les deux moitiés lisant `*e`, elles
            // ne peuvent plus diverger.
            Fabrique::Isv(e) => Bati {
                etat: preset_isv_fixe(*e, r.regime),
                tournant: Some(preset_isv_equipage(*e, r.repli)),
            },
        }
    }
}

/// Une entrée du catalogue : un libellé et de quoi la bâtir. Rien d'autre —
/// tout le reste se déduit (cf. [`Fabrique`]).
pub struct Item {
    /// Libellé fixe. Le titre affiché peut lui ajouter ce que la fabrique
    /// porte (cf. [`Item::titre`]) — pour qu'il n'y ait pas non plus deux
    /// sources là.
    pub libelle: std::borrow::Cow<'static, str>,
    pub fabrique: Fabrique,
}

impl Item {
    const fn new(libelle: &'static str, fabrique: Fabrique) -> Self {
        Self { libelle: std::borrow::Cow::Borrowed(libelle), fabrique }
    }

    /// Entrée **engendrée** : libellé calculé, donc possédé. Le marqueur « · »
    /// la distingue d'une vitrine curatée dans la liste — et évite qu'un
    /// composant porte le même libellé qu'une vitrine qui le met en scène
    /// (« RADIATEUR MEGA » l'aurait fait, et le test de cyclage l'a pris).
    fn engendre(libelle: String, fabrique: Fabrique) -> Self {
        Self { libelle: std::borrow::Cow::Owned(libelle), fabrique }
    }

    /// Titre affiché. Pour l'ISV il nomme **l'épine que la table lui donne**,
    /// jamais une chaîne recopiée à la main : changer `Fabrique::Isv(…)` change
    /// le titre du même coup.
    pub fn titre(&self) -> String {
        match self.epine() {
            Some(e) => format!("{} — EPINE {}", self.libelle, e.nom().to_uppercase()),
            None => self.libelle.to_string(),
        }
    }

    pub fn batir(&self, r: Reglages) -> Bati {
        self.fabrique.batir(r)
    }

    /// L'item montre-t-il une section d'équipage à faire tourner ou à replier ?
    pub fn rotation(&self) -> bool {
        matches!(self.fabrique, Fabrique::Repli(_) | Fabrique::Isv(_) | Fabrique::Rotatif(_))
    }

    /// L'item lit-il le **repli** ? À distinguer de [`Self::rotation`] : jusqu'à
    /// l'arrivée des anneaux, tout ce qui tournait se repliait aussi (la section
    /// d'équipage de l'ISV), et les deux capacités étaient confondues par
    /// coïncidence. Un anneau tourne **sans rien replier** — sa rotation est une
    /// matrice de vue, elle ne recuit aucune géométrie.
    pub fn repli(&self) -> bool {
        matches!(self.fabrique, Fabrique::Repli(_) | Fabrique::Isv(_))
    }

    /// **Axe** autour duquel l'item tourne, quand il tourne.
    ///
    /// Déduit de la fabrique, comme les autres capacités (§5.1). Il vivait
    /// auparavant dans la vue, choisi sur la **catégorie de menu** — une
    /// seconde source, et fausse dès que deux items d'une même catégorie
    /// tournent autour d'axes différents. C'est exactement ce qui est arrivé :
    /// l'ISV est couché sur `ISV_AXE`, un anneau tourne sur `Y`, et les deux
    /// sont des mégastructures.
    pub fn axe_rotation(&self) -> Vec3 {
        match self.fabrique {
            Fabrique::Isv(_) => ISV_AXE,
            _ => Vec3::Y,
        }
    }

    /// L'item a-t-il une propulsion à allumer ? Deux cas : la brique du
    /// radiateur méga (qui n'en montre que la chauffe, faute de tuyère) et
    /// l'ISV complet.
    pub fn allumage(&self) -> bool {
        matches!(self.fabrique, Fabrique::Regime(_) | Fabrique::Isv(_))
    }

    /// Épine du vaisseau, pour les items qui en portent une.
    pub fn epine(&self) -> Option<Epine> {
        match self.fabrique {
            Fabrique::Isv(e) => Some(e),
            _ => None,
        }
    }
}

/// **Catalogue des briques.** L'ordre est l'ordre d'affichage (touche D), et
/// le compte est `BRIQUES.len()` — plus de constante à tenir à jour.
pub const BRIQUES: &[Item] = &[
    Item::new("POUTRES (2 STYLES x 6 GABARITS)", Fabrique::Brique(demo_poutres)),
    Item::new("OSSATURE : POUTRE + APPENDICES", Fabrique::Brique(demo_treillis)),
    Item::new("HABITATS : 10 VARIANTES", Fabrique::Brique(demo_habitats)),
    Item::new("NOEUDS 4 / 6 / T / TETRA", Fabrique::Brique(demo_station)),
    Item::new("PANNEAUX : 5 VARIANTES", Fabrique::Brique(demo_panneaux)),
    Item::new("PANNEAUX MEGASTRUCTURE : 4 FAMILLES", Fabrique::Repli(demo_panneaux_mega)),
    Item::new("RADIATEURS : 8 VARIANTES", Fabrique::Brique(demo_radiateurs)),
    Item::new("RADIATEUR MEGA", Fabrique::Regime(demo_radiateur_mega)),
    Item::new("ANTENNES : 6 VARIANTES", Fabrique::Brique(demo_antennes)),
    Item::new("CAISSONS + CHARGES UTILES", Fabrique::Brique(demo_caissons)),
    Item::new("RAPTOR : ATMOSPHERIQUE vs VIDE", Fabrique::Etat(demo_raptor)),
    Item::new("PROPULSION CHIMIQUE", Fabrique::Propulsion(FamillePropulsion::Chimique)),
    Item::new("PROPULSION ELECTRIQUE", Fabrique::Propulsion(FamillePropulsion::Electrique)),
    Item::new("PROPULSION NUCLEAIRE", Fabrique::Propulsion(FamillePropulsion::Nucleaire)),
    Item::new("ANNEAUX : 4 STYLES", Fabrique::Etat(demo_anneaux)),
    Item::new("BLOC MOTEUR : CAISSE COLLECTEUR + MODULE", Fabrique::Etat(demo_moteur_antimatiere)),
    Item::new("CHARPENTE ISV (+ HEXAGONE EN BAS)", Fabrique::Etat(demo_charpente)),
    Item::new("RESERVOIR CARBURANT (SPHERE)", Fabrique::Etat(demo_reservoir)),
    Item::new(
        "MOTEUR ANTIMATIERE : TUYERE + REACTEUR",
        Fabrique::Etat(demo_moteur_antimatiere_principal),
    ),
    Item::new("COIFFES DE MODULES (3 FORMES)", Fabrique::Brique(demo_coiffes)),
    Item::new("FRET ISV : NACELLE + TRIFORCE + COURONNE 6", Fabrique::Etat(demo_cargo)),
    Item::new("HABITAT PRINCIPAL ISV : MODULE + GRAPPE DE 3", Fabrique::Etat(demo_habitat_isv)),
    Item::new("EQUIPAGE ROTATIF ISV : MODULE + TRAVERSE", Fabrique::Repli(demo_equipage)),
    Item::new(
        "PETIT BOUCLIER ISV : FACE AVANT STRIEE / FACE ARRIERE NERVUREE",
        Fabrique::Etat(demo_bouclier_petit),
    ),
    Item::new(
        "GRAND BOUCLIER ISV : PLAQUE MIROIR ELANCEE + PILE DE 3",
        Fabrique::Etat(demo_bouclier_grand),
    ),
    Item::new(
        "BOUCLIER THERMIQUE D'EPINE : BARDAGE D'ECAILLES",
        Fabrique::Etat(demo_bouclier_thermique),
    ),
    Item::new(
        "EPINE : CARREE (ACTUELLE) vs HEXAGONALE (CANDIDATE)",
        Fabrique::Etat(demo_charpente_hexa),
    ),
    Item::new("EPINE HEXA : PIED TOUR vs PIED PAVILLON (COROLLE)", Fabrique::Etat(demo_epine_pavillon)),
    Item::new("CONSTRUCTEUR PAR PORTS LIBRES", Fabrique::Brique(demo_chantier)),
];

/// Reproductions de vraies petites stations et engins.
pub const PETITES_STATIONS: &[Item] = &[
    Item::new("ISS (CONFIGURATION FINALE)", Fabrique::Etat(preset_iss)),
    Item::new("MIR (CONFIGURATION FINALE)", Fabrique::Etat(preset_mir)),
    Item::new("TIANGONG (CONFIGURATION EN T)", Fabrique::Etat(preset_tiangong)),
    Item::new("SATELLITE DE COMMUNICATION", Fabrique::Etat(preset_comsat)),
    Item::new("SONDE INTERPLANETAIRE", Fabrique::Etat(preset_sonde)),
];

/// Grandes stations et mégastructures.
///
/// La variante d'ISV à **épine carrée** n'est plus exposée : elle a servi à
/// valider l'hexagonale par comparaison (vue Briques « EPINE : CARREE vs
/// HEXAGONALE »), et c'est l'hexagonale qui est retenue. [`Epine::Carree`] et
/// tout ce qu'elle entraîne restent en place — `preset_isv()` la construit
/// encore et des tests s'en servent — seule la vitrine disparaît.
pub const MEGASTRUCTURES: &[Item] = &[
    Item::new("STATION A ANNEAU (ROUE)", Fabrique::Etat(preset_anneau)),
    Item::new("TORE DE STANFORD", Fabrique::Rotatif(preset_stanford)),
    Item::new("COMPLEXE ELYSIUM", Fabrique::Etat(preset_elysium)),
];

/// **Vaisseaux et supervaisseaux** : ce qui se **déplace**, par opposition aux
/// mégastructures, qui tiennent une orbite.
///
/// La séparation n'est pas cosmétique. Une station se dimensionne sur ce
/// qu'elle abrite ; un vaisseau se dimensionne sur ce qu'il emporte et sur la
/// poussée qu'il encaisse — d'où une épine, des réservoirs, une tuyère, et une
/// silhouette **allongée** là où une station est ramassée autour d'un axe de
/// rotation. Les mêler dans une même vitrine rendait les deux illisibles.
///
/// La table couvre trois ordres de grandeur, et c'est voulu : un Starship fait
/// 50 m, l'Endurance 64 m de diamètre, l'ISV près de 400 m. « Supervaisseau »
/// commence là où le véhicule cesse d'être lançable d'une pièce.
///
/// Prochaines cibles, anatomie et ordre de travail :
///  **Partie D bis**.
pub const VAISSEAUX: &[Item] = &[
    Item::new("STARSHIP — COQUE (1/5)", Fabrique::Etat(preset_starship)),
    Item::new("ISV COMPLET (FRET + HABITAT + EQUIPAGE)", Fabrique::Isv(Epine::Hexagonale)),
    Item::new("ISV — RADIATEUR + BLOC MOTEUR", Fabrique::Etat(preset_isv_moteur)),
];

/// **Une entrée par variante de `Composant`**, engendrée depuis la chaîne
/// d'échantillons (L1.2).
///
/// C'est le cœur du dispositif « hybride » : les vitrines curatées ci-dessus
/// montrent des **familles** (« PANNEAUX : 5 VARIANTES »), ce qu'aucune
/// génération ne saurait composer ; celles-ci garantissent qu'**aucun composant
/// n'est absent de l'écran**, sans table à tenir à jour.
///
/// Ajouter une variante à `Composant` casse la compilation en trois endroits —
/// la chaîne `suivante`, `Composant::nom`, la palette — et une fois les trois
/// réparés, le bouton **existe déjà**. Il n'y a rien à penser à ajouter ici,
/// donc rien qui puisse être oublié.
pub fn engendrees() -> Vec<Item> {
    crate::vaisseau::echantillons()
        .into_iter()
        .map(|c| Item::engendre(format!("· {}", c.nom()), Fabrique::Piece(c)))
        .collect()
}

/// Les items **effectifs** d'une table : ses entrées curatées, puis une entrée
/// par composant. Seules les briques reçoivent les engendrées — un vaisseau ou
/// une station est un assemblage, pas une pièce.
pub fn items(table: &'static [Item], avec_pieces: bool) -> Vec<Item> {
    let mut v: Vec<Item> = table
        .iter()
        .map(|i| Item { libelle: i.libelle.clone(), fabrique: i.fabrique.clone() })
        .collect();
    if avec_pieces {
        v.extend(engendrees());
    }
    v
}

/// Les trois tables, pour les tests qui doivent **toutes** les balayer — c'est
/// ce qui fait qu'ajouter une quatrième catégorie un jour ne pourra pas passer
/// à côté des invariants (le seul point à mettre à jour est ici).
/// Le générateur n'y est pas : il n'est pas catalogué (cf. `Categorie::items`).
#[cfg(test)]
pub const TOUTES: [(&str, &[Item]); 4] = [
    ("BRIQUES", BRIQUES),
    ("PETITES STATIONS", PETITES_STATIONS),
    ("MEGASTRUCTURES", MEGASTRUCTURES),
    ("VAISSEAUX", VAISSEAUX),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Les deux bouts de course de chaque réglage. Une brique animée doit être
    /// valide **aux deux**, pas seulement au repos : c'est l'état allumé qui
    /// ajoute de la géométrie (panache) et change les couleurs.
    const EXTREMES: [Reglages; 3] = [
        Reglages { repli: 0.0, regime: 0.0 },
        Reglages { repli: 1.0, regime: 1.0 },
        Reglages { repli: 0.5, regime: 0.5 },
    ];

    // Le test que l'attrape-tout `_ =>` rendait impossible : **chaque** entrée
    // de **chaque** table produit une station non vide. Avant, un indice en trop
    // retombait silencieusement sur « CONSTRUCTEUR PAR PORTS LIBRES » et rien ne
    // le signalait ; maintenant une entrée qui ne bâtit rien fait rougir.
    #[test]
    fn chaque_item_batit_une_station_non_vide() {
        for (cat, table) in TOUTES {
            assert!(!table.is_empty(), "{cat} : table vide");
            for (i, item) in table.iter().enumerate() {
                for r in EXTREMES {
                    let bati = item.batir(r);
                    let s = bati
                        .etat
                        .doit_dessiner()
                        .unwrap_or_else(|| panic!("{cat} n°{i} « {} » : état vide à {r:?}", item.libelle));
                    assert!(
                        s.nb_pieces() > 0,
                        "{cat} n°{i} « {} » : station sans pièce à {r:?}",
                        item.libelle
                    );
                }
            }
        }
    }

    // Les libellés doivent être distincts : deux entrées de menu identiques sont
    // exactement le symptôme que produisait l'attrape-tout, et rien ne le
    // distinguerait à l'écran d'un catalogue correct.
    // **Le point de la liste engendrée** : tout composant du jeu a son bouton,
    // sans qu'aucune table ne le mentionne. Ajouter une variante à `Composant`
    // casse la compilation ailleurs (chaîne d échantillons, `nom`, palette) et,
    // une fois ces trois-là réparés, l entrée **existe déjà**.
    //
    // Le test compare au parc réel plutôt qu à un compte écrit à la main : un
    // nombre figé ici serait exactement la table qu on cherche à supprimer.
    #[test]
    fn chaque_composant_a_son_entree_sans_table_a_tenir() {
        // Sondé sur la liste **effective** de la vue, pas sur `engendrees()` :
        // la première version interrogeait le générateur d entrées directement,
        // si bien que débrancher son branchement dans `items` ne la faisait pas
        // rougir. Ce qui compte est ce que la vue affiche.
        let liste = items(BRIQUES, true);
        let parc = crate::vaisseau::echantillons();
        for c in &parc {
            assert!(
                liste.iter().any(|i| i.libelle.contains(c.nom())),
                "{} absent de la liste effective",
                c.nom()
            );
        }
        assert!(liste.len() > BRIQUES.len(), "aucune entree engendree n a ete ajoutee");
    }

    // Les entrées engendrées ne doivent pas entrer en collision avec les
    // vitrines curatées : « RADIATEUR MEGA » existait des deux côtés, et le
    // cyclage montrait deux fois le même libellé.
    #[test]
    fn les_entrees_engendrees_ne_masquent_aucune_vitrine() {
        let liste = items(BRIQUES, true);
        for (i, a) in liste.iter().enumerate() {
            for b in liste.iter().skip(i + 1) {
                assert_ne!(a.libelle, b.libelle, "libellé en double dans la liste effective");
            }
        }
    }

    #[test]
    fn les_libelles_sont_distincts() {
        for (cat, table) in TOUTES {
            for (i, a) in table.iter().enumerate() {
                for b in table.iter().skip(i + 1) {
                    assert_ne!(a.libelle, b.libelle, "{cat} : libellé en double");
                }
            }
        }
    }

    // **Ce qui remplace les trois constantes d'indice** (`BRIQUE_EQUIPAGE = 20`,
    // `BRIQUE_RADIATEUR = 6`, `MEGA_ISV = [1]`). Elles disaient quels boutons
    // activer, à côté d'une table qui disait ce que l'item était : deux sources.
    // Ici on vérifie que la déduction retombe bien sur les items voulus — nommés
    // par leur **libellé**, qui est la seule chose qu'un humain reconnaît.
    #[test]
    fn les_capacites_se_deduisent_de_la_fabrique() {
        let porte = |table: &'static [Item], f: fn(&Item) -> bool| -> Vec<&'static str> {
            table.iter().filter(|it| f(it)).map(|it| it.libelle.as_ref()).collect()
        };

        // Une seule brique tourne : la section d'équipage. C'est la seule pièce
        // tournante du vaisseau, donc la seule où le bouton ait un sens.
        // Deux briques bougent, pour deux raisons distinctes : la section
        // d equipage se replie **et** tourne ; les panneaux megastructure, eux,
        // ne se replient pas - leurs deux joints suivent le soleil.
        assert_eq!(
            porte(BRIQUES, Item::rotation),
            ["PANNEAUX MEGASTRUCTURE : 4 FAMILLES", "EQUIPAGE ROTATIF ISV : MODULE + TRAVERSE"]
        );
        // Une seule brique s'allume : le radiateur méga, qui n'en montre que la
        // chauffe (il n'a pas de tuyère, donc pas de panache).
        assert_eq!(porte(BRIQUES, Item::allumage), ["RADIATEUR MEGA"]);

        // Côté mégastructures, deux items tournent — mais pour des raisons
        // différentes, et c'est tout l'intérêt de les distinguer : le tore
        // tourne **d'un seul tenant** (gravité par rotation), l'ISV ne fait
        // tourner qu'une **moitié**, celle qui se replie aussi.
        // Côté mégastructures il ne reste que des **stations** : seul le tore
        // tourne, et rien ne se replie ni ne s'allume.
        assert_eq!(porte(MEGASTRUCTURES, Item::rotation), ["TORE DE STANFORD"]);
        assert!(porte(MEGASTRUCTURES, Item::repli).is_empty());
        assert!(porte(MEGASTRUCTURES, Item::allumage).is_empty());

        // Côté vaisseaux, l'ISV — et lui seul pour l'instant — fait les trois.
        let isv = "ISV COMPLET (FRET + HABITAT + EQUIPAGE)";
        assert_eq!(porte(VAISSEAUX, Item::rotation), [isv]);
        assert_eq!(porte(VAISSEAUX, Item::repli), [isv]);
        assert_eq!(porte(VAISSEAUX, Item::allumage), [isv]);

        // Et leurs axes diffèrent — un tore tourne à plat, l'ISV est couché.
        // C'est ce qu'une catégorie de menu ne pouvait déjà pas dire ; les
        // séparer ne change rien à ça, l'axe reste une propriété de l'item.
        let axe = |t: &[Item], libelle: &str| {
            t.iter().find(|i| i.libelle == libelle).unwrap().axe_rotation()
        };
        assert_eq!(axe(MEGASTRUCTURES, "TORE DE STANFORD"), Vec3::Y);
        assert_ne!(axe(VAISSEAUX, isv), Vec3::Y);

        // Les petites stations sont toutes inertes.
        assert!(porte(PETITES_STATIONS, Item::rotation).is_empty());
        assert!(porte(PETITES_STATIONS, Item::allumage).is_empty());
    }

    // Une capacité n'est **jamais** déclarée à côté d'un constructeur qui ne la
    // lit pas : c'est l'invariant que la refonte achète, et il vaut pour les
    // items futurs autant que pour ceux d'aujourd'hui.
    //
    // ⚠️ Formulé à l'envers de la déduction pour ne pas la réciter : on n'écrit
    // pas « `Repli(_)` ⟹ `rotation()` » (ce serait `matches!` recopié), on
    // **exerce** le constructeur et on vérifie que le réglage annoncé change
    // vraiment quelque chose.
    #[test]
    fn une_capacite_annoncee_change_vraiment_la_geometrie() {
        let repos = Reglages { repli: 0.0, regime: 0.0 };
        for (cat, table) in TOUTES {
            for item in table {
                let base = item.batir(repos);
                if item.repli() {
                    let plie = item.batir(Reglages { repli: 1.0, ..repos });
                    assert!(
                        difference(&base, &plie),
                        "{cat} « {} » annonce le repli mais ne bouge pas",
                        item.libelle
                    );
                }
                if item.allumage() {
                    let chaud = item.batir(Reglages { regime: 1.0, ..repos });
                    assert!(
                        difference(&base, &chaud),
                        "{cat} « {} » annonce l'allumage mais ne change pas",
                        item.libelle
                    );
                }
            }
        }
    }

    /// Les deux bâtis diffèrent-ils, moitié tournante comprise ?
    fn difference(a: &Bati, b: &Bati) -> bool {
        a.etat != b.etat || a.tournant != b.tournant
    }

    // L'ISV est le **seul** item en deux moitiés, et le seul à porter une épine
    // — les deux vont ensemble, puisque c'est l'épine qui règle l'alésage du
    // collier de la section tournante.
    #[test]
    fn seul_litem_a_deux_moities_porte_une_epine() {
        for (cat, table) in TOUTES {
            for item in table {
                let bati = item.batir(Reglages::default());
                assert_eq!(
                    bati.tournant.is_some(),
                    item.epine().is_some(),
                    "{cat} « {} » : moitié tournante et épine doivent aller ensemble",
                    item.libelle
                );
            }
        }
    }

    // **La correction de §C.10, rendue structurelle.** Les deux moitiés de l'ISV
    // doivent être bâties avec la même épine : l'alésage du collier se déduit du
    // gabarit, et 3,2 % d'écart suffisent à faire mordre le collier dans la
    // flèche. Avant, l'épine venait d'un `match` séparé de celui qui bâtissait
    // la coque — deux sources pour un accord obligatoire.
    #[test]
    fn les_deux_moities_de_lisv_partagent_leur_epine() {
        // Cherché sur **toutes** les tables plutôt que sur celle où il se
        // trouve aujourd'hui : ce test garde un accord de géométrie (§C.10),
        // pas un rangement de menu. L'ISV a déjà changé de table une fois.
        let item = TOUTES
            .iter()
            .flat_map(|(_, t)| t.iter())
            .find(|it| it.epine().is_some())
            .expect("un ISV complet dans le catalogue");
        let epine = item.epine().unwrap();
        let bati = item.batir(Reglages::default());
        // La moitié tournante bâtie *à part* avec la même épine doit être
        // identique à celle que le catalogue a produite.
        assert_eq!(
            bati.tournant,
            Some(preset_isv_equipage(epine, 0.0)),
            "la section d'équipage n'a pas été bâtie avec l'épine de la table"
        );
    }
}
