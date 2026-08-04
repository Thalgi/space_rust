//! **Icônes de ressources**, chargées une fois et partagées par tout le projet.
//!
//! Quatorze sprites 16 × 16 RGBA vivent dans `assets/sprites/`. Ils sont le
//! vocabulaire complet de l'économie du jeu — il n'y en a pas un quinzième à
//! produire, parce qu'il n'y a **pas de monnaie** : c'est l'énergie qui en tient
//! le rôle (`docs/conception/interface.md` §3.1).
//!
//! Ce module suit le modèle de [`crate::police`], qui a déjà résolu le même
//! problème pour la police Minitel : chargement **unique** au démarrage,
//! stockage en `thread_local` (macroquad est mono-thread), et **repli
//! silencieux** si un fichier manque. Un sprite absent ne doit jamais faire
//! tomber le jeu — au pire son emplacement reste vide.
//!
//! # Ce que le pixel art impose
//!
//! Deux contraintes, faciles à manquer et immédiatement visibles à l'écran :
//!
//! 1. **Filtrage au plus proche voisin.** macroquad filtre en linéaire par
//!    défaut ; du 16 × 16 agrandi en linéaire devient une bouillie. Appliqué
//!    une fois au chargement, ici et nulle part ailleurs.
//! 2. **Échelles entières seulement.** Un facteur 1,5 fait tomber un pixel
//!    source sur une frontière et produit des traits d'épaisseur inégale dans
//!    une image qui n'en a que de 1 px. [`taille_ecran`] choisit donc un
//!    **palier** au lieu d'interpoler.

use macroquad::prelude::*;
use std::cell::RefCell;

/// Côté d'un sprite source, en pixels. Tous les fichiers le respectent, et
/// [`charger`] refuse ceux qui s'en écartent : mieux vaut un emplacement vide
/// qu'une icône déformée au milieu d'une grille alignée.
pub const COTE: u16 = 16;

/// Une ressource du jeu — et donc une icône.
///
/// L'ordre de déclaration est celui de la **grille de la barre** (deux lignes de
/// sept, `conception/interface.md` §3.4) : la ligne du haut d'abord, puis celle
/// du bas, chacune de gauche à droite. Les colonnes 1 à 3 alignent ainsi chaque
/// produit sous sa matière première.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ressource {
    // --- Ligne du haut : matières brutes, intrants, flux ---
    Minerai,
    MineraiRare,
    NourritureBrute,
    Hydrogene,
    Antimatiere,
    /// **Tient le rôle de la monnaie.** Rien ne s'achète avec un nombre
    /// abstrait dans ce jeu : tout se paie en énergie.
    Energie,
    Recherche,
    // --- Ligne du bas : produits raffinés, choses bâties, main-d'œuvre ---
    Metal,
    MetalRare,
    NourritureTransformee,
    MateriauConstruction,
    Superstructure,
    Population,
    Robots,
}

impl Ressource {
    /// Les quatorze, dans l'ordre de la grille.
    ///
    /// Exhaustif à dessein : ajouter une variante casse la compilation tant
    /// qu'elle n'est pas ici. C'est le même dispositif que la chaîne
    /// d'échantillons des composants — la couverture est une propriété du
    /// compilateur, pas une discipline à se rappeler.
    pub const TOUTES: [Ressource; 14] = [
        Ressource::Minerai,
        Ressource::MineraiRare,
        Ressource::NourritureBrute,
        Ressource::Hydrogene,
        Ressource::Antimatiere,
        Ressource::Energie,
        Ressource::Recherche,
        Ressource::Metal,
        Ressource::MetalRare,
        Ressource::NourritureTransformee,
        Ressource::MateriauConstruction,
        Ressource::Superstructure,
        Ressource::Population,
        Ressource::Robots,
    ];

    /// Nombre de colonnes de la grille : la moitié des ressources.
    ///
    /// **Dérivé**, jamais écrit en dur. Un 15ᵉ sprite un jour, et la grille
    /// suit toute seule au lieu de laisser un trou.
    pub const COLONNES: usize = Ressource::TOUTES.len() / 2;

    /// Nom du fichier, sans dossier ni extension. C'est **la seule** ligne qui
    /// relie une variante à un fichier du disque.
    pub fn fichier(self) -> &'static str {
        match self {
            Ressource::Minerai => "raw_ore",
            Ressource::MineraiRare => "raw_rare_ore",
            Ressource::NourritureBrute => "raw_food",
            Ressource::Hydrogene => "hydrogen",
            Ressource::Antimatiere => "antimater",
            Ressource::Energie => "energy",
            Ressource::Recherche => "research",
            Ressource::Metal => "metal",
            Ressource::MetalRare => "rare_metal",
            Ressource::NourritureTransformee => "processed_food",
            Ressource::MateriauConstruction => "construction_material",
            Ressource::Superstructure => "superstructure",
            Ressource::Population => "population_number",
            Ressource::Robots => "robot",
        }
    }

    /// Libellé court, pour une infobulle ou un panneau détaillé. La barre, elle,
    /// n'affiche que l'icône et le nombre.
    pub fn nom(self) -> &'static str {
        match self {
            Ressource::Minerai => "MINERAI",
            Ressource::MineraiRare => "MINERAI RARE",
            Ressource::NourritureBrute => "NOURRITURE BRUTE",
            Ressource::Hydrogene => "HYDROGENE",
            Ressource::Antimatiere => "ANTIMATIERE",
            Ressource::Energie => "ENERGIE",
            Ressource::Recherche => "RECHERCHE",
            Ressource::Metal => "METAL",
            Ressource::MetalRare => "METAL RARE",
            Ressource::NourritureTransformee => "NOURRITURE TRANSFORMEE",
            Ressource::MateriauConstruction => "MATERIAU DE CONSTRUCTION",
            Ressource::Superstructure => "SUPERSTRUCTURE",
            Ressource::Population => "POPULATION",
            Ressource::Robots => "ROBOTS",
        }
    }

    /// Place dans la grille de la barre : `(colonne, ligne)`, ligne 0 en haut.
    ///
    /// Déduite de l'ordre de [`Self::TOUTES`] — la grille **est** cet ordre, et
    /// réarranger la barre ne demande que de réordonner le tableau.
    pub fn case(self) -> (usize, usize) {
        let i = Ressource::TOUTES.iter().position(|r| *r == self).unwrap_or(0);
        (i % Ressource::COLONNES, i / Ressource::COLONNES)
    }
}

thread_local! {
    /// Une case par ressource, dans l'ordre de `TOUTES`. `None` = fichier
    /// absent ou refusé ; l'emplacement reste vide, le jeu tourne.
    static ICONES: RefCell<Vec<Option<Texture2D>>> = const { RefCell::new(Vec::new()) };
}

/// Charge les quatorze icônes. À appeler une fois au démarrage, avant la boucle
/// de rendu — comme [`crate::police::charger`].
pub async fn charger() {
    let mut chargees = Vec::with_capacity(Ressource::TOUTES.len());
    let mut manquantes = Vec::new();

    for r in Ressource::TOUTES {
        let chemin = format!("assets/sprites/{}.png", r.fichier());
        match load_texture(&chemin).await {
            Ok(t) if t.width() as u16 == COTE && t.height() as u16 == COTE => {
                // Sans ça, macroquad interpole : une icône de 16 px agrandie
                // devient floue et perd les traits de 1 px dont elle est faite.
                t.set_filter(FilterMode::Nearest);
                chargees.push(Some(t));
            }
            Ok(t) => {
                manquantes.push(format!("{} ({}x{}, attendu {COTE}x{COTE})", r.fichier(), t.width(), t.height()));
                chargees.push(None);
            }
            Err(_) => {
                manquantes.push(r.fichier().to_string());
                chargees.push(None);
            }
        }
    }

    if !manquantes.is_empty() {
        warn!("Sprites de ressources absents ou mal dimensionnés : {manquantes:?} — emplacements laissés vides.");
    }
    ICONES.with(|i| *i.borrow_mut() = chargees);
}

/// Taille d'affichage, en pixels : le plus grand **multiple entier** de [`COTE`]
/// qui tienne dans `souhaitee`, et jamais moins qu'un exemplaire.
///
/// C'est ici que se tient la contrainte du pixel art. Rendre `souhaitee` telle
/// quelle donnerait des facteurs fractionnaires, donc des traits d'épaisseur
/// inégale ; on descend donc au palier inférieur plutôt que d'approcher au plus
/// près.
pub fn taille_ecran(souhaitee: f32) -> f32 {
    let facteur = (souhaitee / COTE as f32).floor().max(1.0);
    facteur * COTE as f32
}

/// Dessine l'icône de `r`, coin haut-gauche en `(x, y)`, sur `cote` pixels.
///
/// Ne dessine **rien** si le sprite manque : c'est le repli, et il est
/// silencieux (l'avertissement a déjà été émis une fois au chargement).
pub fn dessiner(r: Ressource, x: f32, y: f32, cote: f32) {
    ICONES.with(|i| {
        let icones = i.borrow();
        let Some(Some(t)) = icones.get(indice(r)) else { return };
        draw_texture_ex(
            t,
            x,
            y,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(cote, cote)), ..Default::default() },
        );
    });
}

/// L'icône de `r` est-elle disponible ? Sert aux tests et à la mise en page,
/// qui peut vouloir réserver la place même sans sprite.
pub fn presente(r: Ressource) -> bool {
    ICONES.with(|i| matches!(i.borrow().get(indice(r)), Some(Some(_))))
}

fn indice(r: Ressource) -> usize {
    Ressource::TOUTES.iter().position(|x| *x == r).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Chaque ressource pointe un **fichier qui existe**. C'est le seul lien
    // entre l'enum et le disque, et rien d'autre ne le vérifierait : un nom mal
    // orthographié ne se verrait qu'à l'écran, par une icône manquante.
    #[test]
    fn chaque_ressource_a_son_fichier_sur_le_disque() {
        for r in Ressource::TOUTES {
            let chemin = format!("assets/sprites/{}.png", r.fichier());
            assert!(
                std::path::Path::new(&chemin).exists(),
                "{:?} pointe {chemin}, qui n'existe pas",
                r
            );
        }
    }

    // Deux ressources ne peuvent pas partager un fichier : ce serait deux
    // compteurs à l'icône identique, indiscernables dans la barre.
    #[test]
    fn deux_ressources_ne_partagent_pas_une_icone() {
        for (i, a) in Ressource::TOUTES.iter().enumerate() {
            for b in Ressource::TOUTES.iter().skip(i + 1) {
                assert_ne!(a.fichier(), b.fichier(), "{a:?} et {b:?} partagent une icône");
                assert_ne!(a.nom(), b.nom(), "{a:?} et {b:?} partagent un libellé");
            }
        }
    }

    // Le dossier ne contient **rien d'autre** que les quatorze. Sans ce test,
    // un sprite ajouté sur le disque resterait invisible au jeu sans que rien
    // ne le signale — exactement le défaut que la colonne d'items a corrigé
    // côté composants (`suivi/stations.md` §F.7).
    #[test]
    fn aucun_sprite_du_dossier_nest_orphelin() {
        let connus: Vec<&str> = Ressource::TOUTES.iter().map(|r| r.fichier()).collect();
        let dossier = std::fs::read_dir("assets/sprites").expect("assets/sprites");
        for e in dossier.flatten() {
            let nom = e.file_name().to_string_lossy().to_string();
            let Some(tige) = nom.strip_suffix(".png") else { continue };
            assert!(
                connus.contains(&tige),
                "{nom} est sur le disque mais aucune Ressource ne le réclame"
            );
        }
    }

    // La grille est **deux lignes pleines**, sans trou : c'est ce qui permet
    // d'aligner chaque produit sous sa matière première.
    #[test]
    fn la_grille_fait_deux_lignes_pleines() {
        assert_eq!(Ressource::TOUTES.len() % 2, 0, "un compte impair laisserait un trou");
        assert_eq!(Ressource::COLONNES, 7);
        let mut vues = std::collections::HashSet::new();
        for r in Ressource::TOUTES {
            let (c, l) = r.case();
            assert!(l < 2, "{r:?} tombe sur la ligne {l}");
            assert!(c < Ressource::COLONNES, "{r:?} tombe sur la colonne {c}");
            assert!(vues.insert((c, l)), "deux ressources sur la case ({c}, {l})");
        }
        assert_eq!(vues.len(), Ressource::TOUTES.len(), "des cases sont vides");
    }

    // **Les trois chaînes de raffinage sont alignées** : chaque produit dans la
    // colonne de sa matière première. C'est la raison d'être des deux lignes,
    // et sans ce test un réarrangement de `TOUTES` la casserait en silence.
    #[test]
    fn chaque_produit_est_sous_sa_matiere_premiere() {
        let chaines = [
            (Ressource::Minerai, Ressource::Metal),
            (Ressource::MineraiRare, Ressource::MetalRare),
            (Ressource::NourritureBrute, Ressource::NourritureTransformee),
        ];
        for (brut, raffine) in chaines {
            let (cb, lb) = brut.case();
            let (cr, lr) = raffine.case();
            assert_eq!(lb, 0, "{brut:?} devrait être en haut");
            assert_eq!(lr, 1, "{raffine:?} devrait être en bas");
            assert_eq!(cb, cr, "{brut:?} en colonne {cb}, {raffine:?} en colonne {cr}");
        }
    }

    // Le palier entier : c'est **toute** la contrainte du pixel art, et la
    // seule chose que `taille_ecran` a le droit de faire.
    #[test]
    fn la_taille_daffichage_est_un_multiple_entier_du_sprite() {
        for souhaitee in [4.0_f32, 16.0, 17.0, 23.9, 24.0, 31.9, 32.0, 47.5, 100.0] {
            let t = taille_ecran(souhaitee);
            assert!(
                (t / COTE as f32).fract().abs() < 1e-6,
                "{souhaitee} → {t}, qui n'est pas un multiple de {COTE}"
            );
            // Jamais au-dessus de ce qu'on demande, sauf le plancher d'un
            // exemplaire — sinon l'icône déborderait de la place réservée.
            assert!(t <= souhaitee.max(COTE as f32) + 1e-6, "{souhaitee} → {t}, qui déborde");
            assert!(t >= COTE as f32, "{souhaitee} → {t}, plus petit qu'un sprite");
        }
        // Et il **grandit** bien quand on lui donne plus de place : un palier
        // qui rendrait toujours 16 passerait tout ce qui précède.
        assert!(taille_ecran(64.0) > taille_ecran(32.0));
        assert_eq!(taille_ecran(33.0), 32.0);
    }
}
