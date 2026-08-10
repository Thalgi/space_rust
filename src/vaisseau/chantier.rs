//! Constructeur incrémental de station qui suit les **ports hôtes libres** —
//! le fondement du générateur (`docs/conception/stations.md`, Partie A §7).
//!
//! Idée : au lieu d'assembler à la main, on tient à jour la liste des ports
//! hôtes libres (en monde). La grammaire du générateur pilote :
//! 1. choisir un port libre compatible (`compatibles`) ;
//! 2. y clipser un composant (`poser`) — le port hôte est **consommé** et les
//!    ports restants de l'enfant deviennent **libres** à leur tour ;
//! 3. répéter jusqu'à épuisement du budget / de la grammaire, puis `terminer`.
//!
//! Les transformées sont cuites en `Mat4` dans les `Piece` (couche cuite), mais
//! le chaînage garde le repère monde en `Repere` (couche construction).
//!
//! ## Pourquoi `dead_code` est autorise ici
//!
//! Ce module est **ecrit et teste avant son consommateur** : l'ecran
//! d'assemblage (Lot 4) est ce qui l'appellera. Il n'est donc pas mort, il est
//! en attente -- et ses tests le prouvent, ils l'exercent en entier.
//!
//! L'autorisation est **volontairement limitee a ce module** : partout ailleurs,
//! un element mort reste un avertissement visible. C'est tout l'objet de la
//! revue -- ramener le bruit a zero pour que le prochain vrai defaut se voie.
#![allow(dead_code)]

use super::{
    accoupler_roule, cuire, Budget, Composant, DonneesSousEnsemble, Enveloppe, EtatStation, GenrePort,
    Piece, Port, Profil, Repere, Station,
};
use macroquad::prelude::{Mat4, Vec3};
use std::collections::HashSet;
use std::rc::Rc;

/// Tolérance de recouvrement : deux enveloppes ne déclenchent une collision que
/// si la distance de leurs **axes** passe sous `FACTEUR × (r1 + r2)`.
/// < 1 pour autoriser l'adjacence de docking (les composants voisins se touchent).
///
/// ⚠️ **Publique à dessein.** L'overlay de débogage
/// (`ecran::enveloppes`) doit expliquer *exactement* les refus que ce module
/// prononce : s'il appliquait sa propre marge, il montrerait un conflit que le
/// modèle n'a pas vu — pire que pas d'overlay du tout.
pub(crate) const FACTEUR_COLLISION: f32 = 0.85;

/// Un port hôte **libre**, en coordonnées monde, prêt à recevoir un enfant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortLibre {
    /// Identifiant **stable** : un compteur monotone, jamais recyclé. C'est
    /// lui — pas la position dans [`Chantier::libres`] — qui identifie un
    /// port d'un appel à l'autre (`docs/conception/assembleur.md` §6.1) :
    /// poser ailleurs retire un élément par `swap_remove`, qui décale les
    /// positions des autres, mais ne touche à aucun `id`.
    pub id: u64,
    pub repere: Repere,
    pub genre: GenrePort,
    pub profil: Profil,
    /// Identifiant **stable** de la pièce qui expose ce port (exemptée de
    /// l'anti-collision) — un `Chantier::piece`, pas une position dans un
    /// `Vec` : `retirer` (§6.2) fait disparaître des pièces au milieu de la
    /// séquence, ce qu'une position brute ne survivrait pas.
    pub origine: u64,
    /// Position de ce port dans `origine.composant.ports()` — **locale**,
    /// indépendante de tout `id` de session. Sert à `Chantier::recette` (§6.4)
    /// à désigner un port hôte de façon rejouable : un `PortLibre::id` n'a de
    /// sens que dans la session qui l'a distribué, mais « le 2ᵉ port de la
    /// pièce n°3 » se retrouve pareil à chaque rejeu déterministe.
    pub indice: usize,
}

/// Une pièce posée, avec sa provenance : le port hôte qu'elle a consommé en
/// se posant, et l'indice de **son propre** port de montage utilisé pour
/// l'accoupler. `None` pour la racine, qui n'a ni l'un ni l'autre.
///
/// Conservé pour trois raisons, toutes propres à un mécanisme qui rejoue les
/// poses : retrouver le **sous-arbre** d'une pièce lors d'un `retirer` (§6.2,
/// en suivant la chaîne des `hote.origine`), **rendre** le port consommé à
/// `libres` quand la pièce disparaît, et — depuis L2.5 — **exporter la
/// recette** (§6.4) qui reproduit la pose exacte.
#[derive(Clone)]
struct Entree {
    id: u64,
    piece: Piece,
    hote: Option<(PortLibre, usize)>,
    /// Roulis appliqué à l'accouplement (§11.3). Gardé parce que `pivoter` doit
    /// pouvoir le **changer** : sans lui, on ne saurait pas de quel angle la
    /// pièce est déjà tournée, donc pas quel delta appliquer.
    roulis: f32,
}

/// Une étape de [`Recette`] : quoi poser, et où (`docs/conception/assembleur.md`
/// §6.4). Le format de sauvegarde — une liste ordonnée qu'il suffit de
/// rejouer (voir [`Chantier::depuis_recette`]) pour reconstruire un chantier
/// identique, sans jamais sérialiser de `Mat4`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Etape {
    pub composant: Composant,
    /// `None` pour la racine (la première étape, toujours). Sinon :
    /// `(indice de la pièce hôte dans la séquence déjà rejouée, indice
    /// **local** de son port — `PortLibre::indice`)`. Jamais un `PortLibre::id` :
    /// il n'a de sens que dans la session de `Chantier` qui l'a distribué,
    /// et change à chaque rejeu (§6.1).
    pub hote: Option<(usize, usize)>,
    /// Indice du port de montage du composant qu'on pose (dans son propre
    /// `ports()`) — ignoré pour la racine.
    pub montage: usize,
    /// **Roulis** autour de l'axe d'accostage, en radians
    /// (`conception/assembleur.md` §11.3). Ignoré pour la racine.
    ///
    /// `serde(default)` : les recettes écrites avant le Lot 7 se rechargent
    /// sans conversion, et le zéro qu'elles reçoivent signifie exactement le
    /// comportement qu'elles avaient — « les hauts restent alignés ».
    #[serde(default)]
    pub roulis: f32,
}

/// Une recette complète : de quoi reconstruire un chantier à l'identique.
pub type Recette = Vec<Etape>;

/// Instantané complet de l'état mutable de [`Chantier`] — **hors** historique
/// lui-même, qui ne se sauvegarde pas soi-même. Sert à `annuler`/`refaire`
/// (§6.3) : plutôt que de tenter d'inverser chaque opération une par une (ce
/// que `retirer` d'un sous-arbre entier ne sait pas faire proprement — le
/// reposer ne recrée pas la même arborescence), on capture l'état complet
/// juste avant chaque opération réussie et on le restitue tel quel.
#[derive(Clone)]
struct Instantane {
    pieces: Vec<Entree>,
    libres: Vec<PortLibre>,
    budget: Option<Budget>,
    prochain_id_port: u64,
    prochain_id_piece: u64,
}

/// Ce qu'un déplacement ferait, une fois tous ses contrôles passés — voir
/// [`Chantier::plan_deplacement`].
struct PlanDeplacement {
    /// Le sous-arbre qui bouge.
    bouge: HashSet<u64>,
    /// Transformation rigide qui l'emmène de son ancienne pose à la nouvelle.
    delta: Mat4,
    /// Port de destination, et sa position courante dans `libres`.
    cible: PortLibre,
    pos_cible: usize,
    /// Port hôte à rendre à `libres` sous un `id` neuf.
    ancien_hote: PortLibre,
}

/// Constructeur de station à bookkeeping des ports libres.
#[derive(Default)]
pub struct Chantier {
    pieces: Vec<Entree>,
    libres: Vec<PortLibre>,
    budget: Option<Budget>,
    /// Prochain identifiant de port libre à distribuer. Ne décroît jamais et
    /// ne recule jamais : c'est ce qui garantit qu'un `id` n'est jamais
    /// réutilisé, même après un `swap_remove`.
    prochain_id_port: u64,
    /// Même principe pour les pièces : `retirer` (§6.2) en fait disparaître
    /// au milieu de la séquence, un `Vec::retain` déplacerait leurs voisines
    /// vers des positions différentes si on s'y fiait comme identifiant.
    prochain_id_piece: u64,
    /// Historique actif ? Coûte une copie de l'état à chaque opération
    /// réussie (`avant_mutation`) — hors sujet pour le générateur, qui n'en a
    /// jamais besoin et pose des centaines de fois par station. D'où l'opt-in
    /// via [`Self::avec_historique`] plutôt qu'un suivi permanent.
    historique: bool,
    pile_annuler: Vec<Instantane>,
    pile_refaire: Vec<Instantane>,
    /// Compteur de mutations **réussies**, monotone. Voir [`Self::version`].
    version: u64,
}

impl Chantier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructeur avec plafond de complexité (chaque pose dépense `cout()`).
    pub fn avec_budget(total: f32) -> Self {
        Self { budget: Some(Budget::new(total)), ..Self::default() }
    }

    /// Active l'historique annuler/refaire (§6.3). Chaînable :
    /// `Chantier::avec_budget(100.0).avec_historique()`.
    pub fn avec_historique(mut self) -> Self {
        self.historique = true;
        self
    }

    /// Ports hôtes libres actuels (lecture). La **position** dans la tranche
    /// n'est valide que jusqu'à la prochaine `poser`/`racine` (retrait par
    /// `swap_remove`, qui décale un élément quelconque sur la position
    /// libérée) — mais chaque `PortLibre::id` l'est, lui, **indéfiniment**
    /// tant que le port reste libre : c'est lui qu'il faut garder d'un appel
    /// à l'autre, jamais une position.
    pub fn libres(&self) -> &[PortLibre] {
        &self.libres
    }

    /// Pièce qui expose un port (via `PortLibre::origine`), par son
    /// identifiant **stable** — pas une position. Permet à la grammaire de
    /// savoir **sur quoi** elle est en train de poser — par exemple pour
    /// zoner les appendices d'une poutre autrement que ceux d'un module.
    pub fn piece(&self, id: u64) -> Option<&Piece> {
        self.pieces.iter().find(|e| e.id == id).map(|e| &e.piece)
    }

    /// Toutes les pièces encore vivantes (les retirées sont omises), dans
    /// l'ordre de pose, avec leur identifiant **stable**.
    pub fn pieces(&self) -> impl Iterator<Item = (u64, &Piece)> {
        self.pieces.iter().map(|e| (e.id, &e.piece))
    }

    /// Budget encore disponible. Permet à la grammaire de **réserver** de quoi
    /// finir : sans cela, la croissance des modules consomme tout et il ne reste
    /// rien pour les panneaux, qui sont pourtant l'organe vital.
    pub fn budget_restant(&self) -> f32 {
        // Sans budget (chantier libre), rien ne limite : « infini » laisse
        // toutes les comparaisons `<= plancher` répondre non.
        match &self.budget {
            Some(b) => b.restant(),
            None => f32::INFINITY,
        }
    }

    pub fn nb_pieces(&self) -> usize {
        self.pieces.len()
    }

    /// **Numéro de version** : avance à chaque mutation réussie — pose, retrait,
    /// annulation, rétablissement — et jamais autrement.
    ///
    /// Sert de **clé de cache** à l'écran d'assemblage. L'état vert/rouge/éteint
    /// des ports (§8.4) coûte un `peut_poser` par port, donc une collision
    /// contre toutes les pièces : sur 45 pièces et 60 ports libres, 2 700
    /// distances d'enveloppes pour un résultat qui **ne change pas** tant qu'on
    /// ne touche à rien (§10.3). Il faut donc le garder d'une image à l'autre —
    /// et savoir quand le jeter.
    ///
    /// Un compteur, et pas un état comparé : `nb_pieces` ne suffirait pas (poser
    /// puis retirer y ramène le même nombre), et comparer les listes de ports
    /// coûterait ce qu'on cherche à économiser.
    ///
    /// ⚠️ **`annuler` la fait avancer, pas reculer.** Le vaisseau revient en
    /// arrière, la version non : ce qu'elle date, c'est le moment où l'affichage
    /// a cessé d'être à jour, et une annulation le périme autant qu'une pose.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Identifiant **stable** de la dernière pièce posée (racine ou enfant),
    /// ou `None` si le chantier est encore vide. Remplace le piège
    /// `nb_pieces() - 1` : juste après `poser`/`racine`, ça reste vrai que
    /// c'est la dernière entrée, mais s'y fier **suppose** que `pieces.len()`
    /// et le prochain `id` avancent toujours ensemble — faux dès qu'un
    /// `retirer` est intervenu. Cette méthode lit directement la dernière
    /// entrée, sans supposer quoi que ce soit sur son numéro.
    pub fn derniere_piece(&self) -> Option<u64> {
        self.pieces.last().map(|e| e.id)
    }

    /// Pose le composant **racine** à l'origine ; tous ses ports deviennent
    /// libres. Renvoie `false` si le budget ne le couvre pas.
    pub fn racine(&mut self, comp: Composant) -> bool {
        let avant = self.avant_mutation();
        if !self.payer(&comp) {
            return false;
        }
        let corps = Repere::IDENTITE;
        let id = self.nouvel_id_piece();
        self.ajouter_libres(corps, &comp, None, id);
        self.pieces.push(Entree { id, piece: cuire(corps, &comp), hote: None, roulis: 0.0 }); // dernier usage : consomme `comp`
        self.enregistrer(avant);
        true
    }

    /// Clipse `comp` (par son port de montage `montage_idx`) sur le port libre
    /// d'identifiant **stable** `hote_id` (`PortLibre::id`, pas une position).
    /// Vérifie **compatibilité** (genre + profil) puis **budget**. Renvoie
    /// `true` si posé ; le port hôte est alors consommé et les autres ports de
    /// l'enfant ajoutés aux libres.
    ///
    /// Un `hote_id` **périmé** — port déjà consommé par une pose précédente,
    /// ou qui n'a jamais existé — échoue proprement (`false`), jamais de pose
    /// au mauvais endroit : c'est le point qui compte pour une UI qui a du
    /// retard sur le modèle (`docs/conception/assembleur.md` §6.1, point 3).
    pub fn poser(&mut self, hote_id: u64, comp: Composant, montage_idx: usize) -> bool {
        self.poser_roule(hote_id, comp, montage_idx, 0.0)
    }

    /// [`Self::poser`] avec **roulis** autour de l'axe d'accostage (§11.3).
    /// `poser` n'est qu'un appel à zéro : une seule liste de contrôles.
    pub fn poser_roule(
        &mut self,
        hote_id: u64,
        comp: Composant,
        montage_idx: usize,
        roulis: f32,
    ) -> bool {
        let Some((pos, hote)) = self.trouver(hote_id) else { return false };
        let Some((montage, corps)) = Self::corps_prevu_roule(&hote, &comp, montage_idx, roulis)
        else {
            return false;
        };
        if !hote.genre.compatible(montage.genre) || !hote.profil.compatible(montage.profil) {
            return false;
        }
        // Anti-collision : rejette si l'enfant recouvre trop une pièce **autre**
        // que son hôte direct (qu'il est censé toucher au joint).
        if self.collision(corps, &comp, hote.origine) {
            return false;
        }
        let avant = self.avant_mutation();
        if !self.payer(&comp) {
            return false;
        }
        self.libres.swap_remove(pos); // port consommé — l'`id` n'est jamais réutilisé
        let id = self.nouvel_id_piece();
        self.ajouter_libres(corps, &comp, Some(montage_idx), id);
        self.pieces.push(Entree { id, piece: cuire(corps, &comp), hote: Some((hote, montage_idx)), roulis }); // dernier usage : consomme `comp`
        self.enregistrer(avant);
        true
    }

    /// La pose serait-elle acceptée, **sans rien poser** ? Mêmes contrôles que
    /// [`Self::poser`] : compatibilité, anti-collision, budget. Même contrat
    /// sur `hote_id` : un identifiant périmé répond simplement `false`.
    ///
    /// Sert à rendre un **groupe atomique** : la grammaire pose ses paires
    /// symétriques port par port, et une paire dont un seul membre passait
    /// l'anti-collision laissait une station visiblement bancale. Le quota était
    /// déjà servi par groupe entier « pour ne jamais casser une paire
    /// symétrique » — la collision, elle, ne l'était pas.
    ///
    /// C'est aussi exactement ce dont la palette de l'éditeur aura besoin :
    /// savoir si un clic aboutirait, avant de l'avoir fait
    /// (`docs/conception/assembleur.md` §6.5).
    pub fn peut_poser(&self, hote_id: u64, comp: &Composant, montage_idx: usize) -> bool {
        let Some((_, hote)) = self.trouver(hote_id) else { return false };
        let Some((montage, corps)) = Self::corps_prevu(&hote, comp, montage_idx) else {
            return false;
        };
        if !hote.genre.compatible(montage.genre) || !hote.profil.compatible(montage.profil) {
            return false;
        }
        if self.collision(corps, comp, hote.origine) {
            return false;
        }
        self.budget.is_none_or(|b| b.peut_payer(comp.cout()))
    }

    /// La pièce **exacte** qu'une pose sur `hote_id` produirait — transformée
    /// comprise — sans rien poser. C'est la source unique du **fantôme** de
    /// l'éditeur (`docs/conception/assembleur.md` §8.3) : la vue ne recalcule
    /// pas la pose, elle passe par le même [`Self::corps_prevu`] que
    /// [`Self::poser`]. Un fantôme calculé à côté mentirait sur l'endroit où le
    /// clic va réellement poser — exactement le doublon que le Lot 1 a passé
    /// son temps à supprimer.
    ///
    /// Ne répond **que la géométrie** : `Some` dès que la pose est *définie*
    /// (le port existe, `montage_idx` désigne bien un port de `comp`), sans
    /// regarder la compatibilité, l'anti-collision ni le budget. C'est délibéré,
    /// et §8.5 l'exige : l'overlay trace le segment de plus courte approche
    /// entre le fantôme et la pièce qui le **refuse**, donc le fantôme doit
    /// exister précisément dans le cas où la pose n'aboutirait pas. Le *où* est
    /// ici, le *si* est dans [`Self::peut_poser`] — deux questions, deux
    /// méthodes.
    pub fn pose_prevue(&self, hote_id: u64, comp: &Composant, montage_idx: usize) -> Option<Piece> {
        self.pose_prevue_roule(hote_id, comp, montage_idx, 0.0)
    }

    /// [`Self::pose_prevue`] **avec roulis** (§11.3) : le fantôme doit montrer
    /// l'angle que la pose emploiera, sans quoi tourner une pièce en main ne se
    /// verrait qu'après l'avoir posée.
    pub fn pose_prevue_roule(
        &self,
        hote_id: u64,
        comp: &Composant,
        montage_idx: usize,
        roulis: f32,
    ) -> Option<Piece> {
        let (_, hote) = self.trouver(hote_id)?;
        let (_, corps) = Self::corps_prevu_roule(&hote, comp, montage_idx, roulis)?;
        Some(cuire(corps, comp))
    }

    /// Identifiants des ports libres compatibles avec le port `montage_idx` de
    /// `comp` (genre + profil). Sert à la grammaire pour choisir où poser.
    pub fn compatibles(&self, comp: &Composant, montage_idx: usize) -> Vec<u64> {
        let ports = comp.ports();
        let m = match ports.get(montage_idx) {
            Some(p) => *p,
            None => return Vec::new(),
        };
        self.libres
            .iter()
            .filter(|h| h.genre.compatible(m.genre) && h.profil.compatible(m.profil))
            .map(|h| h.id)
            .collect()
    }

    /// Gèle le chantier en un **sous-ensemble réutilisable**
    /// ([`Composant::SousEnsemble`], Partie E.3) : les pièces posées jusqu'ici
    /// deviennent un sous-arbre figé (déjà en repère local, puisque `racine`
    /// place toujours à `Repere::IDENTITE`) et les ports hôtes encore libres
    /// deviennent les ports exposés du composite. `profil` est le profil du
    /// port de **montage** que ce sous-ensemble présentera à qui l'utilisera
    /// — choisi par l'appelant (pas dérivé), rien n'oblige à s'ancrer par un
    /// port précis du sous-arbre. `None` si rien n'a été posé (même invariant
    /// que [`Station::depuis_pieces`] : un sous-ensemble vide n'existe pas).
    pub fn figer(self, profil: Profil) -> Option<Composant> {
        if self.pieces.is_empty() {
            return None;
        }
        let cout = self.pieces.iter().map(|e| e.piece.composant.cout()).sum();
        let rayon = self
            .pieces
            .iter()
            .fold(0.0_f32, |m, e| m.max(e.piece.centre().length() + e.piece.composant.rayon_local()));
        let ports_exposes = self
            .libres
            .iter()
            .map(|pl| Port::new(pl.repere, pl.genre, pl.profil))
            .collect();
        let pieces = self.pieces.into_iter().map(|e| e.piece).collect();
        Some(Composant::SousEnsemble {
            profil,
            donnees: Rc::new(DonneesSousEnsemble { pieces, ports_exposes, cout, rayon }),
        })
    }

    /// Publie la station (immuable). `Vide` si rien n'a été posé.
    pub fn terminer(self) -> EtatStation {
        let pieces = self.pieces.into_iter().map(|e| e.piece).collect();
        match Station::depuis_pieces(pieces) {
            Some(s) => EtatStation::Prete(s),
            None => EtatStation::Vide,
        }
    }

    /// **Aperçu** : la même station que [`Self::terminer`], mais sans consommer
    /// le chantier.
    ///
    /// `terminer` prend `self` — c'est juste, un chantier *publié* ne doit plus
    /// bouger. L'écran d'assemblage, lui, doit dessiner un chantier **qu'on est
    /// en train de modifier**, à chaque image. Sans ce miroir il n'aurait eu que
    /// deux issues, toutes deux mauvaises : cloner le chantier entier par image,
    /// ou redessiner les pièces à la main — donc une **deuxième** façon de
    /// construire une station, exactement le doublon que le Lot 1 a passé son
    /// temps à supprimer.
    ///
    /// Il copie les pièces (l'écran en garde le résultat entre deux
    /// modifications, il ne l'appelle pas à chaque image).
    pub fn apercu(&self) -> EtatStation {
        let pieces = self.pieces.iter().map(|e| e.piece.clone()).collect();
        match Station::depuis_pieces(pieces) {
            Some(s) => EtatStation::Prete(s),
            None => EtatStation::Vide,
        }
    }

    /// Retire la pièce `id` **et tout son sous-arbre**. Le port qu'elle
    /// consommait en se posant redevient libre — sous un `id` **neuf**,
    /// l'ancien reste périmé comme toute pose consommée (§6.1) — et le budget
    /// est remboursé du coût total des pièces retirées. `false`, sans rien
    /// changer, si `id` ne désigne aucune pièce vivante.
    ///
    /// **`poser` puis `retirer` ramène le chantier à un état indiscernable de
    /// l'état initial** — mêmes pièces, mêmes ports libres, même budget
    /// restant (§6.2, la propriété d'aller-retour). « Indiscernable » près du
    /// `PortLibre::id` du port rendu, qui diffère forcément de celui qu'il
    /// portait avant d'être consommé.
    pub fn retirer(&mut self, id: u64) -> bool {
        let Some(hote) = self.pieces.iter().find(|e| e.id == id).map(|e| e.hote) else {
            return false;
        };
        let avant = self.avant_mutation();
        let a_retirer = self.sous_arbre(id);

        let cout: f32 =
            self.pieces.iter().filter(|e| a_retirer.contains(&e.id)).map(|e| e.piece.composant.cout()).sum();
        if let Some(b) = &mut self.budget {
            b.rembourser(cout);
        }

        self.pieces.retain(|e| !a_retirer.contains(&e.id));
        self.libres.retain(|p| !a_retirer.contains(&p.origine));

        if let Some((h, _)) = hote {
            let nouveau = self.prochain_id_port;
            self.prochain_id_port += 1;
            self.libres.push(PortLibre { id: nouveau, ..h });
        }
        self.enregistrer(avant);
        true
    }

    /// **Déplace la pièce `id` — et tout son sous-arbre — sur le port libre
    /// `cible_id`** (`docs/conception/assembleur.md` §10.10).
    ///
    /// Un seul geste, donc **une seule** entrée d'historique. `retirer` puis
    /// `poser` ne ferait pas l'affaire pour trois raisons : `retirer` emporte le
    /// sous-arbre (on perdrait la descendance), l'annulation compterait deux
    /// coups, et le port hôte rendu recevrait un `id` neuf entre les deux.
    ///
    /// **Le sous-arbre suit rigidement** : sa géométrie interne ne change pas,
    /// seule la transformée du bloc bouge. On ne re-pose rien, on translate et
    /// tourne un ensemble.
    ///
    /// Refuse — sans rien changer — dans quatre cas :
    ///
    /// - `id` ne désigne aucune pièce vivante ;
    /// - `id` est la **racine** : elle n'a pas de port hôte, elle est à
    ///   l'origine par définition, et la déplacer ne ferait que translater tout
    ///   le vaisseau dans un chantier centré sur l'origine ;
    /// - le port cible appartient au sous-arbre qui bouge — la pièce
    ///   s'accrocherait à sa propre descendance, un cycle que rien ne prévoit ;
    /// - genre/profil incompatibles, ou le bloc déplacé heurterait une pièce
    ///   restée en place.
    ///
    /// Le budget n'entre pas en jeu : rien n'est créé ni détruit.
    pub fn deplacer(&mut self, id: u64, cible_id: u64, montage_idx: usize) -> bool {
        let Some(plan) = self.plan_deplacement(id, cible_id, montage_idx) else { return false };
        let PlanDeplacement { bouge, delta, cible, pos_cible, ancien_hote } = plan;

        let avant = self.avant_mutation();

        // Le bloc bouge : pièces **et** repères monde des ports qu'il expose.
        // ⚠️ Les oublier donnerait des ports fantômes — dessinés à l'ancienne
        // place, accrochant à la nouvelle. Partagé avec `pivoter`.
        self.appliquer_delta_bloc(&bouge, delta);

        // Le port de destination est consommé, l'ancien hôte redevient libre —
        // sous un `id` **neuf**, comme tout port rendu (§6.1).
        self.libres.swap_remove(pos_cible);
        let neuf = self.prochain_id_port;
        self.prochain_id_port += 1;
        self.libres.push(PortLibre { id: neuf, ..ancien_hote });

        // Et la pièce déplacée change de parent.
        if let Some(e) = self.pieces.iter_mut().find(|e| e.id == id) {
            e.hote = Some((cible, montage_idx));
        }
        self.enregistrer(avant);
        true
    }

    /// **Tourne la pièce `id` (et son sous-arbre) autour de son axe d'accostage.**
    ///
    /// `roulis` est un angle **absolu**, pas un incrément : l'appelant décide de
    /// l'orientation voulue, et rappeler avec la même valeur ne bouge rien. Un
    /// incrément aurait rendu l'opération non idempotente, donc pénible à
    /// annuler et à rejouer.
    ///
    /// Le bloc suit **rigidement** — même machinerie que [`Self::deplacer`]
    /// (§11.5) : tourner une pièce tourne tous ses ports du même angle, donc ses
    /// enfants aussi.
    ///
    /// Refuse, sans rien changer : la racine (pas d'axe d'accostage — elle n'est
    /// accouplée à rien), une pièce inconnue, ou une rotation qui ferait heurter
    /// le bloc contre une pièce restée en place.
    pub fn pivoter(&mut self, id: u64, roulis: f32) -> bool {
        let Some(entree) = self.pieces.iter().find(|e| e.id == id) else { return false };
        let Some((hote, montage_idx)) = entree.hote else { return false };
        let comp = entree.piece.composant.clone();
        let ancienne = entree.piece.transforme;

        let Some((_, corps)) = Self::corps_prevu_roule(&hote, &comp, montage_idx, roulis) else {
            return false;
        };
        let delta = corps.to_mat4() * ancienne.inverse();
        let bouge = self.sous_arbre(id);
        if self.collision_bloc(&bouge, delta, hote.origine) {
            return false;
        }

        let avant = self.avant_mutation();
        self.appliquer_delta_bloc(&bouge, delta);
        if let Some(e) = self.pieces.iter_mut().find(|e| e.id == id) {
            e.roulis = roulis;
        }
        self.enregistrer(avant);
        true
    }

    /// Roulis courant de la pièce, ou 0 (racine, ou pièce inconnue).
    pub fn roulis(&self, id: u64) -> f32 {
        self.pieces.iter().find(|e| e.id == id).map(|e| e.roulis).unwrap_or(0.0)
    }

    /// Applique une transformation rigide à tout un sous-arbre : les poses des
    /// pièces, **et les repères monde des ports libres qu'elles exposent**.
    ///
    /// Factorisée entre [`Self::deplacer`] et [`Self::pivoter`] : les deux font
    /// exactement le même geste sur le bloc, et c'est le port oublié qui
    /// donnerait le défaut le plus discret — un port dessiné à l'ancienne place
    /// et qui accroche à la nouvelle.
    fn appliquer_delta_bloc(&mut self, bouge: &HashSet<u64>, delta: Mat4) {
        for e in self.pieces.iter_mut().filter(|e| bouge.contains(&e.id)) {
            e.piece.transforme = delta * e.piece.transforme;
        }
        let delta_repere = Repere::depuis_mat4(delta);
        for p in self.libres.iter_mut().filter(|p| bouge.contains(&p.origine)) {
            p.repere = delta_repere.compose(p.repere);
        }
    }

    /// **Le déplacement aboutirait-il ?** Même question que [`Self::deplacer`],
    /// sans effet de bord — le pendant de `peut_poser` face à `poser`.
    ///
    /// C'est ce que l'écran interroge pour colorer les ports pendant un
    /// déplacement (§8.4). Sans lui il aurait fallu se rabattre sur
    /// `peut_poser`, qui juge **une pièce seule** contre tout le vaisseau : il
    /// aurait peint en vert des ports que `deplacer` refuse — un port qui ment,
    /// de la même famille que le fantôme qui ment.
    pub fn peut_deplacer(&self, id: u64, cible_id: u64, montage_idx: usize) -> bool {
        self.plan_deplacement(id, cible_id, montage_idx).is_some()
    }

    /// Tous les contrôles de [`Self::deplacer`], et ce qu'ils produisent.
    /// **Source unique** de la décision : `deplacer` l'applique, `peut_deplacer`
    /// se contente de regarder si elle existe. Deux listes de contrôles
    /// séparées finiraient par diverger, et l'écran annoncerait alors une pose
    /// que le modèle refuse.
    fn plan_deplacement(&self, id: u64, cible_id: u64, montage_idx: usize) -> Option<PlanDeplacement> {
        let entree = self.pieces.iter().find(|e| e.id == id)?;
        // Racine : pas d'hôte, donc rien à déplacer.
        let (ancien_hote, _) = entree.hote?;
        let comp = entree.piece.composant.clone();
        let ancienne = entree.piece.transforme;

        let bouge = self.sous_arbre(id);
        let (pos_cible, cible) = self.trouver(cible_id)?;
        // Se poser sur sa propre descendance : un cycle.
        if bouge.contains(&cible.origine) {
            return None;
        }

        let (montage, corps) = Self::corps_prevu(&cible, &comp, montage_idx)?;
        if !cible.genre.compatible(montage.genre) || !cible.profil.compatible(montage.profil) {
            return None;
        }

        // Delta rigide qui emmène le bloc de son ancienne pose à la nouvelle.
        // Composition de transformations rigides = produit de matrices, dans cet
        // ordre : `delta ∘ ancienne`.
        let delta = corps.to_mat4() * ancienne.inverse();
        if self.collision_bloc(&bouge, delta, cible.origine) {
            return None;
        }
        Some(PlanDeplacement { bouge, delta, cible, pos_cible, ancien_hote })
    }

    /// Le bloc `bouge`, transporté par `delta`, heurterait-il une pièce restée
    /// en place ?
    ///
    /// Ne confronte **que** le mouvant à l'immobile : les membres du bloc se
    /// chevauchent entre eux par construction, ils sont accouplés. `exempte`
    /// est la pièce d'accueil, tolérée au joint comme dans [`Self::collision`],
    /// dont ce contrôle reprend le critère à l'identique.
    fn collision_bloc(&self, bouge: &HashSet<u64>, delta: Mat4, exempte: u64) -> bool {
        let immobiles: Vec<Enveloppe> = self
            .pieces
            .iter()
            .filter(|e| !bouge.contains(&e.id) && e.id != exempte)
            .map(|e| e.piece.composant.enveloppe_locale().transformee(e.piece.transforme))
            .collect();
        self.pieces
            .iter()
            .filter(|e| bouge.contains(&e.id))
            .any(|e| {
                let mienne =
                    e.piece.composant.enveloppe_locale().transformee(delta * e.piece.transforme);
                immobiles.iter().any(|sienne| {
                    mienne.distance_axes(sienne) < FACTEUR_COLLISION * (mienne.rayon + sienne.rayon)
                })
            })
    }

    /// La pièce `id` **et toute sa descendance** — l'ensemble exact que
    /// [`Self::retirer`] ferait disparaître.
    ///
    /// Publiée pour l'état « pièce sélectionnée » de l'éditeur
    /// (`docs/conception/assembleur.md` §8.3) : ce qui s'affiche en
    /// surbrillance doit être ce que la touche Suppr emportera, à la pièce
    /// près. Le calcul est **le même** que celui de `retirer` — c'est lui,
    /// littéralement, `retirer` appelle cette méthode — plutôt qu'un parcours
    /// refait côté vue qui pourrait en diverger.
    ///
    /// Vide si `id` ne désigne aucune pièce vivante. Sans ambiguïté : un
    /// résultat valide contient toujours au moins `id` lui-même.
    pub fn sous_arbre(&self, id: u64) -> HashSet<u64> {
        if !self.pieces.iter().any(|e| e.id == id) {
            return HashSet::new();
        }
        // Une pièce en fait partie si sa pièce hôte en fait partie. Une seule
        // passe avant suffit — `self.pieces` reste dans l'ordre de pose
        // (`retain` ne réordonne jamais), et l'hôte d'une pièce y figure donc
        // toujours **avant** elle.
        let mut dedans = HashSet::from([id]);
        for e in &self.pieces {
            if e.hote.is_some_and(|(h, _)| dedans.contains(&h.origine)) {
                dedans.insert(e.id);
            }
        }
        dedans
    }

    /// La pièce que la demi-droite `origine + t·direction` rencontre **en
    /// premier**, ou `None` si elle n'en touche aucune.
    ///
    /// C'est la moitié « pièce » de la désignation (`conception/assembleur.md`
    /// §8.3) : le clic de l'éditeur devient un rayon, et cette méthode dit sur
    /// quoi il tombe. Le départage se fait sur la distance de l'origine à la
    /// surface de chaque enveloppe touchée — la pièce la plus proche de l'œil
    /// est celle qu'on voit.
    ///
    /// Passe par les **mêmes enveloppes** que l'anti-collision
    /// (`Composant::enveloppe_locale` transformée par la pose) : désigner une
    /// pièce et refuser de poser dedans doivent parler du même volume, sans
    /// quoi l'éditeur laisserait cliquer là où il ne laisse pas construire.
    pub fn piece_sous_rayon(&self, origine: Vec3, direction: Vec3) -> Option<u64> {
        self.pieces
            .iter()
            .filter_map(|e| {
                let env = e.piece.composant.enveloppe_locale().transformee(e.piece.transforme);
                env.touche_rayon(origine, direction).map(|d| (e.id, d))
            })
            .min_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
            .map(|(id, _)| id)
    }

    /// Défait la dernière opération réussie (`racine`/`poser`/`retirer`).
    /// `false` — sans rien changer — s'il n'y a rien à défaire, y compris
    /// quand l'historique n'est pas activé ([`Self::avec_historique`]) :
    /// jamais de panique, jamais un demi-retour.
    pub fn annuler(&mut self) -> bool {
        let Some(avant) = self.pile_annuler.pop() else { return false };
        let courant = self.instantane();
        self.pile_refaire.push(courant);
        self.restaurer(avant);
        true
    }

    /// Refait la dernière opération défaite par [`Self::annuler`]. `false` —
    /// sans rien changer — s'il n'y a rien à refaire. Une nouvelle opération
    /// réussie (`racine`/`poser`/`retirer`) vide la pile de refaire : on ne
    /// peut pas refaire un futur qu'une pose entre-temps a rendu caduc.
    pub fn refaire(&mut self) -> bool {
        let Some(apres) = self.pile_refaire.pop() else { return false };
        let courant = self.instantane();
        self.pile_annuler.push(courant);
        self.restaurer(apres);
        true
    }

    /// Exporte la **recette** : la liste ordonnée des poses qui reconstruit ce
    /// chantier à l'identique (`docs/conception/assembleur.md` §6.4). Aucune
    /// `Mat4` sérialisée — chaque port hôte est désigné par une **position**
    /// (indice de la pièce dans la séquence, indice local de son port,
    /// `PortLibre::indice`), jamais par un `id` de session, qui n'aurait plus
    /// aucun sens une fois rechargé.
    ///
    /// Un `Composant::SousEnsemble` voyage dans sa propre `Etape` **déjà
    /// figé** : `donnees` (`Rc<DonneesSousEnsemble>`) porte des `Piece` déjà
    /// cuites, sérialisées telles quelles plutôt que rejouées — voir la
    /// discussion de la question ouverte, `docs/suivi/assembleur.md` §L2.5.
    pub fn recette(&self) -> Recette {
        self.pieces
            .iter()
            .map(|e| {
                let hote = e.hote.map(|(h, _)| {
                    let piece_idx = self
                        .pieces
                        .iter()
                        .position(|e2| e2.id == h.origine)
                        .expect("l'hôte doit précéder son enfant dans la séquence");
                    (piece_idx, h.indice)
                });
                let montage = e.hote.map_or(0, |(_, m)| m);
                Etape { composant: e.piece.composant.clone(), hote, montage, roulis: e.roulis }
            })
            .collect()
    }

    /// Reconstruit un chantier en rejouant une [`Recette`] — l'inverse de
    /// [`Self::recette`]. `None` si une étape échoue : recette tronquée ou
    /// corrompue, ou écrite sous une version du jeu où une pose qui passait
    /// alors ne passe plus (§6.4 : la sauvegarde dépend du **déterminisme**
    /// de la construction, pas de sa stabilité dans le temps — une formule de
    /// géométrie ou une marge de collision retouchée plus tard peut faire
    /// échouer une vieille recette). Sans budget : la recette rejoue des
    /// choix déjà validés, le budget est une règle de construction, pas une
    /// propriété de la géométrie qui en résulte.
    pub fn depuis_recette(etapes: &[Etape]) -> Option<Chantier> {
        let mut ch = Chantier::new();
        let mut ids: Vec<u64> = Vec::with_capacity(etapes.len());
        for etape in etapes {
            let pose_ok = match etape.hote {
                None => ch.racine(etape.composant.clone()),
                Some((piece_idx, port_idx)) => {
                    let hote_id = *ids.get(piece_idx)?;
                    let port_id = ch
                        .libres()
                        .iter()
                        .find(|p| p.origine == hote_id && p.indice == port_idx)
                        .map(|p| p.id)?;
                    // Le roulis fait partie de la pose : le rejouer par `poser`
                    // (donc à zéro) rendrait une géométrie différente de celle
                    // qu'on a sauvegardée, sans que rien ne le signale.
                    ch.poser_roule(port_id, etape.composant.clone(), etape.montage, etape.roulis)
                }
            };
            if !pose_ok {
                return None;
            }
            ids.push(ch.derniere_piece()?);
        }
        Some(ch)
    }

    // ---- interne ----

    /// Port de montage retenu et **repère monde** qu'aurait `comp` clipsé par ce
    /// port sur le port libre `hote`. `None` si `montage_idx` ne désigne aucun
    /// port de `comp`.
    ///
    /// Extrait pour être l'unique endroit où la pose se calcule : `poser`,
    /// `peut_poser` et `pose_prevue` répondent à trois questions différentes
    /// (*pose*, *pourrait-on poser*, *où*) mais doivent parler de la **même**
    /// géométrie. Les tenir d'accord par trois copies d'`accoupler` aurait été
    /// le doublon habituel de ce projet ; ici il n'y a rien à tenir d'accord.
    fn corps_prevu(hote: &PortLibre, comp: &Composant, montage_idx: usize) -> Option<(Port, Repere)> {
        Self::corps_prevu_roule(hote, comp, montage_idx, 0.0)
    }

    /// [`Self::corps_prevu`] avec roulis (§11.3). Source unique : `corps_prevu`
    /// n'est plus qu'un appel à zéro.
    fn corps_prevu_roule(
        hote: &PortLibre,
        comp: &Composant,
        montage_idx: usize,
        roulis: f32,
    ) -> Option<(Port, Repere)> {
        let montage = *comp.ports().get(montage_idx)?;
        Some((montage, accoupler_roule(hote.repere, montage.repere, roulis)))
    }

    fn payer(&mut self, comp: &Composant) -> bool {
        match &mut self.budget {
            Some(b) => b.depenser(comp.cout()),
            None => true,
        }
    }

    fn nouvel_id_piece(&mut self) -> u64 {
        let id = self.prochain_id_piece;
        self.prochain_id_piece += 1;
        id
    }

    fn instantane(&self) -> Instantane {
        Instantane {
            pieces: self.pieces.clone(),
            libres: self.libres.clone(),
            budget: self.budget,
            prochain_id_port: self.prochain_id_port,
            prochain_id_piece: self.prochain_id_piece,
        }
    }

    /// ⚠️ **`Instantane` ne porte pas `version`**, et ne doit pas : restaurer un
    /// état passé rendrait la version non monotone, donc inutilisable comme clé
    /// de cache — deux états différents pourraient porter le même numéro.
    fn restaurer(&mut self, i: Instantane) {
        self.version += 1;
        self.pieces = i.pieces;
        self.libres = i.libres;
        self.budget = i.budget;
        self.prochain_id_port = i.prochain_id_port;
        self.prochain_id_piece = i.prochain_id_piece;
    }

    /// À appeler juste avant la **première** mutation d'une opération, une
    /// fois tous les contrôles de refus passés — inutile de capturer un état
    /// qu'on va de toute façon jeter si l'opération échoue encore après coup.
    /// `None` si l'historique n'est pas activé : aucune copie, le générateur
    /// (qui pose des centaines de fois par station, sans jamais annuler) n'en
    /// paie pas le prix.
    fn avant_mutation(&self) -> Option<Instantane> {
        self.historique.then(|| self.instantane())
    }

    /// Empile l'instantané pris par [`Self::avant_mutation`], si l'historique
    /// est actif — et invalide la pile de refaire : une opération réussie
    /// rend caduc tout ce qu'on aurait pu refaire.
    ///
    /// **Le compteur de version avance ici**, hors du `if` : il date toute
    /// mutation réussie, que l'historique soit actif ou non. Ces deux lignes et
    /// `restaurer` sont les **seuls** endroits qui le touchent — appelés une
    /// fois par mutation réussie, jamais sur un refus, ce qui vaut mieux que
    /// cinq incréments dispersés dans les méthodes publiques.
    fn enregistrer(&mut self, avant: Option<Instantane>) {
        self.version += 1;
        if let Some(a) = avant {
            self.pile_annuler.push(a);
            self.pile_refaire.clear();
        }
    }

    /// L'enfant (`comp` placé en `corps`) recouvre-t-il trop une pièce **autre**
    /// que son hôte `hote_piece` ?
    ///
    /// Mesuré sur des **capsules** ([`Enveloppe`]) et non plus des sphères : une
    /// sphère autour d'une pièce allongée réserve un volume vide énorme sur ses
    /// flancs, et refuse des poses pourtant libres. Sans conséquence pour le
    /// générateur, qui réessaie ailleurs ; rédhibitoire face à un humain qui
    /// vient de cliquer (`docs/conception/assembleur.md` §5.3).
    ///
    /// Le critère garde exactement la **même forme** qu'avant — distance des
    /// axes contre somme des rayons, à `FACTEUR_COLLISION` près — pour que la
    /// tolérance d'adjacence de docking conserve son sens.
    fn collision(&self, corps: Repere, comp: &Composant, hote_piece: u64) -> bool {
        let mienne = comp.enveloppe_locale().transformee(corps.to_mat4());
        self.pieces.iter().any(|e| {
            if e.id == hote_piece {
                return false;
            }
            let sienne = e.piece.composant.enveloppe_locale().transformee(e.piece.transforme);
            mienne.distance_axes(&sienne) < FACTEUR_COLLISION * (mienne.rayon + sienne.rayon)
        })
    }

    /// Ajoute les ports de `comp` (placé en `corps`, exposés par la pièce
    /// `origine`) comme libres, sauf celui consommé (`sauf`). Chacun reçoit un
    /// `id` neuf — le compteur ne recule ni ne se remet à zéro, donc deux
    /// ports n'ont jamais le même `id`, même l'un après l'autre dans le temps.
    fn ajouter_libres(&mut self, corps: Repere, comp: &Composant, sauf: Option<usize>, origine: u64) {
        for (i, p) in comp.ports().into_iter().enumerate() {
            if Some(i) == sauf {
                continue;
            }
            let id = self.prochain_id_port;
            self.prochain_id_port += 1;
            let monde = corps.compose(p.repere);
            self.libres.push(PortLibre { id, repere: monde, genre: p.genre, profil: p.profil, origine, indice: i });
        }
    }

    /// Position actuelle et valeur du port libre d'identifiant `id`, ou `None`
    /// s'il n'est plus libre (déjà consommé, ou `id` jamais distribué).
    fn trouver(&self, id: u64) -> Option<(usize, PortLibre)> {
        self.libres.iter().position(|p| p.id == id).map(|pos| (pos, self.libres[pos]))
    }
}

#[cfg(test)]
mod tests {
    use super::super::generateur::Rng;
    use crate::vaisseau::accoupler;
    use super::super::{Sorties, StyleTreillis, VarianteModule, VariantePanneau};
    use super::*;
    use macroquad::prelude::{vec3, Vec3};

    fn module() -> Composant {
        Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 }
    }
    fn panneau() -> Composant {
        Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 2.0, largeur: 1.0 }
    }

    #[test]
    fn chantier_vide_donne_vide() {
        assert_eq!(Chantier::new().terminer(), EtatStation::Vide);
    }

    /// **`apercu` dit la même chose que `terminer`, sans rien consommer.**
    ///
    /// Deux façons de construire une station, c'est la porte ouverte au fantôme
    /// qui ment : l'écran dessinerait un vaisseau que la publication ne
    /// donnerait pas. Le test compare les deux sorties sur le **même** chantier
    /// — et il ne peut le faire que parce qu'`apercu` ne le détruit pas, ce qui
    /// est précisément la propriété visée.
    #[test]
    fn l_apercu_est_la_station_publiee_sans_consommer_le_chantier() {
        let mut ch = Chantier::new();
        assert_eq!(ch.apercu(), EtatStation::Vide, "un chantier vide n'a rien à montrer");

        ch.racine(module());
        let id = ch.libres()[0].id;
        ch.poser(id, module(), 1);

        // Appelé deux fois de suite : le chantier survit au premier appel.
        let a = ch.apercu();
        let b = ch.apercu();
        assert_eq!(a, b, "l'aperçu n'est pas stable d'un appel à l'autre");
        // Et le chantier est intact — on peut encore poser dessus.
        assert_eq!(ch.nb_pieces(), 2);

        // La publication doit donner exactement la même station.
        assert_eq!(a, ch.terminer(), "l'aperçu diverge de la station publiée");
    }

    /// **La version date toute mutation réussie, et elle seule.**
    ///
    /// C'est la clé de cache de l'écran (§10.3) : elle doit avancer sur tout ce
    /// qui change le vaisseau — y compris `annuler`, qui le change autant qu'une
    /// pose — et **rester immobile sur un refus**, sinon le cache se jetterait à
    /// chaque clic raté.
    #[test]
    fn la_version_avance_a_chaque_mutation_reussie_et_pas_autrement() {
        let mut ch = Chantier::new().avec_historique();
        let v0 = ch.version();

        // Refus : rien ne bouge. Un identifiant de port qui n'existe pas.
        assert!(!ch.poser(9999, module(), 0));
        assert_eq!(ch.version(), v0, "un refus a fait avancer la version");
        assert!(!ch.annuler(), "rien à annuler sur un chantier vierge");
        assert_eq!(ch.version(), v0, "une annulation vide a fait avancer la version");

        // Chaque réussite avance, strictement.
        assert!(ch.racine(module()));
        let v1 = ch.version();
        assert!(v1 > v0, "racine n'a pas daté");

        let id = ch.libres()[0].id;
        assert!(ch.poser(id, module(), 1));
        let v2 = ch.version();
        assert!(v2 > v1, "poser n'a pas daté");

        // ⚠️ Annuler fait AVANCER : le vaisseau recule, la date non. Un cache
        // indexé dessus doit se jeter, et deux états distincts ne doivent jamais
        // porter le même numéro.
        assert!(ch.annuler());
        let v3 = ch.version();
        assert!(v3 > v2, "annuler n'a pas daté — le cache garderait l'état d'avant");
        assert_eq!(ch.nb_pieces(), 1, "l'annulation n'a pas défait la pose");

        assert!(ch.refaire());
        assert!(ch.version() > v3, "refaire n'a pas daté");

        // Retrait : dernière porte de mutation.
        let v4 = ch.version();
        let piece = ch.derniere_piece().expect("une pièce posée");
        assert!(ch.retirer(piece));
        assert!(ch.version() > v4, "retirer n'a pas daté");
    }

    // ---- L4.6 `deplacer` (§10.10) ----

    /// **Déplacer garde la descendance, et ne crée ni ne détruit rien.**
    /// C'est la propriété qui justifie l'existence de la méthode : `retirer`
    /// puis `poser` perdrait le sous-arbre.
    #[test]
    fn deplacer_emporte_le_sous_arbre_sans_rien_perdre() {
        let mut ch = Chantier::new();
        ch.racine(module());
        // Une branche : b sur la racine, c sur b.
        let p0 = ch.libres()[0].id;
        assert!(ch.poser(p0, module(), 1));
        let b = ch.derniere_piece().unwrap();
        let sur_b = ch.libres().iter().find(|p| p.origine == b).map(|p| p.id).unwrap();
        assert!(ch.poser(sur_b, module(), 1));
        let c = ch.derniere_piece().unwrap();

        let avant_pieces = ch.nb_pieces();
        let avant_libres = ch.libres().len();
        let sous_arbre = ch.sous_arbre(b);
        assert!(sous_arbre.contains(&c), "le montage du test ne fait pas de c un enfant de b");

        // Une destination hors du sous-arbre déplacé.
        let cible = ch
            .libres()
            .iter()
            .find(|p| !sous_arbre.contains(&p.origine) && ch.peut_poser(p.id, &module(), 1))
            .map(|p| p.id);
        let Some(cible) = cible else {
            panic!("aucune destination libre : le scénario ne teste rien");
        };
        assert!(ch.deplacer(b, cible, 1), "le déplacement aurait dû aboutir");

        assert_eq!(ch.nb_pieces(), avant_pieces, "des pièces ont disparu ou apparu");
        assert_eq!(ch.libres().len(), avant_libres, "le compte de ports libres a changé");
        assert!(ch.piece(c).is_some(), "l'enfant a été perdu par le déplacement");
        assert_eq!(ch.sous_arbre(b), sous_arbre, "le sous-arbre n'est plus le même");
    }

    /// **Le bloc se déplace rigidement** : la position de l'enfant *relative* à
    /// son parent est inchangée. Sans ça le sous-arbre se disloquerait — chaque
    /// pièce garderait son ancienne place pendant que sa racine bouge.
    #[test]
    fn le_sous_arbre_deplace_garde_sa_forme() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        ch.poser(p0, module(), 1);
        let b = ch.derniere_piece().unwrap();
        let sur_b = ch.libres().iter().find(|p| p.origine == b).map(|p| p.id).unwrap();
        ch.poser(sur_b, module(), 1);
        let c = ch.derniere_piece().unwrap();

        let ecart_avant = ch.piece(c).unwrap().centre() - ch.piece(b).unwrap().centre();
        let sous_arbre = ch.sous_arbre(b);
        let cible = ch
            .libres()
            .iter()
            .find(|p| !sous_arbre.contains(&p.origine) && ch.peut_poser(p.id, &module(), 1))
            .map(|p| p.id)
            .expect("aucune destination libre");
        assert!(ch.deplacer(b, cible, 1));

        let ecart_apres = ch.piece(c).unwrap().centre() - ch.piece(b).unwrap().centre();
        assert!(
            (ecart_avant.length() - ecart_apres.length()).abs() < 1e-3,
            "l'enfant s'est éloigné de son parent : {} -> {}",
            ecart_avant.length(),
            ecart_apres.length()
        );
    }

    /// **Les ports libres du bloc suivent le bloc.** `PortLibre::repere` est en
    /// coordonnées monde : ne pas lui appliquer le delta donnerait des ports
    /// fantômes, dessinés à l'ancienne place et accrochant à la nouvelle.
    ///
    /// Vérifié **sans recopier le delta** : on redemande où le port devrait
    /// être, à partir de la pose de sa pièce et de son indice local — c'est-à-dire
    /// par le même chemin que `ajouter_libres`.
    #[test]
    fn les_ports_du_bloc_deplace_suivent_leur_piece() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        ch.poser(p0, module(), 1);
        let b = ch.derniere_piece().unwrap();

        let sous_arbre = ch.sous_arbre(b);
        let cible = ch
            .libres()
            .iter()
            .find(|p| !sous_arbre.contains(&p.origine) && ch.peut_poser(p.id, &module(), 1))
            .map(|p| p.id)
            .expect("aucune destination libre");
        assert!(ch.deplacer(b, cible, 1));

        let mut verifies = 0;
        for p in ch.libres() {
            let Some(piece) = ch.piece(p.origine) else { continue };
            let corps = Repere::depuis_mat4(piece.transforme);
            let local = piece.composant.ports()[p.indice].repere;
            let attendu = corps.compose(local);
            assert!(
                p.repere.pos.distance(attendu.pos) < 1e-3,
                "port {} de la pièce {} : monde {:?}, attendu {:?}",
                p.indice,
                p.origine,
                p.repere.pos,
                attendu.pos
            );
            verifies += 1;
        }
        assert!(verifies > 0, "aucun port vérifié : la garde ne s'exerce pas");
    }

    /// Les quatre refus de §10.10, **sans rien changer** au chantier.
    #[test]
    fn deplacer_refuse_les_cas_impossibles() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let racine = ch.derniere_piece().unwrap();
        let p0 = ch.libres()[0].id;
        ch.poser(p0, module(), 1);
        let b = ch.derniere_piece().unwrap();

        let etat = |c: &Chantier| (c.nb_pieces(), c.libres().len(), c.version());
        let temoin = etat(&ch);

        // Pièce inconnue.
        assert!(!ch.deplacer(u64::MAX, ch.libres()[0].id, 0));
        // La racine ne se déplace pas : pas de port hôte.
        assert!(!ch.deplacer(racine, ch.libres()[0].id, 0));
        // Port cible inconnu.
        assert!(!ch.deplacer(b, u64::MAX, 0));
        // Sur sa propre descendance : un cycle.
        let sien = ch.libres().iter().find(|p| p.origine == b).map(|p| p.id).unwrap();
        assert!(!ch.deplacer(b, sien, 1));

        assert_eq!(etat(&ch), temoin, "un refus a modifié le chantier");
    }

    /// **Un déplacement est UN coup d'historique.** C'est la deuxième raison
    /// d'être de la méthode : `retirer` puis `poser` en compterait deux, et
    /// annuler ne défairait que la moitié du geste.
    #[test]
    fn annuler_un_deplacement_le_defait_en_entier() {
        let mut ch = Chantier::new().avec_historique();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        ch.poser(p0, module(), 1);
        let b = ch.derniere_piece().unwrap();
        let ou_etait = ch.piece(b).unwrap().centre();

        let sous_arbre = ch.sous_arbre(b);
        let cible = ch
            .libres()
            .iter()
            .find(|p| !sous_arbre.contains(&p.origine) && ch.peut_poser(p.id, &module(), 1))
            .map(|p| p.id)
            .expect("aucune destination libre");
        assert!(ch.deplacer(b, cible, 1));
        assert!(
            ch.piece(b).unwrap().centre().distance(ou_etait) > 1e-3,
            "le déplacement n'a rien bougé : le test ne prouverait rien"
        );

        assert!(ch.annuler(), "rien à annuler après un déplacement");
        assert!(
            ch.piece(b).unwrap().centre().distance(ou_etait) < 1e-3,
            "une seule annulation n'a pas ramené la pièce à sa place"
        );
    }

    // ---- L7.2 le roulis (section 11.3) ----

    /// **Pivoter tourne le bloc et rien d'autre** : la piece reste accostee au
    /// meme port, ses enfants suivent, et le compte de pieces ne bouge pas.
    #[test]
    fn pivoter_tourne_la_piece_et_son_sous_arbre() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        assert!(ch.poser(p0, module(), 1));
        let b = ch.derniere_piece().unwrap();
        let sur_b = ch.libres().iter().find(|p| p.origine == b).map(|p| p.id).unwrap();
        assert!(ch.poser(sur_b, module(), 1));
        let c = ch.derniere_piece().unwrap();

        let contact = ch.piece(b).unwrap().centre();
        let avant_pieces = ch.nb_pieces();
        let ecart_avant = (ch.piece(c).unwrap().centre() - contact).length();
        // Reperes AVANT, pour pouvoir exiger que quelque chose ait tourne.
        let haut_avant = ch.piece(b).unwrap().transforme.transform_vector3(Vec3::Y);

        assert!(ch.pivoter(b, std::f32::consts::FRAC_PI_2), "le pivot aurait du aboutir");

        // ⚠️ **Le pivot doit VRAIMENT tourner.** Sans cette assertion, toutes
        // celles qui suivent (la piece reste au contact, le sous-arbre garde sa
        // forme) passent aussi quand `pivoter` ne fait rien du tout — le
        // red-check l'a montre. Un test d'invariant ne prouve pas l'effet.
        let haut_apres = ch.piece(b).unwrap().transforme.transform_vector3(Vec3::Y);
        assert!(
            haut_avant.dot(haut_apres) < 0.9,
            "un quart de tour n'a pas tourne la piece : {haut_avant:?} -> {haut_apres:?}"
        );

        assert_eq!(ch.nb_pieces(), avant_pieces, "des pieces ont disparu ou apparu");
        // La piece pivotee reste au meme endroit : elle tourne SUR son port.
        assert!(
            ch.piece(b).unwrap().centre().distance(contact) < 1e-3,
            "la piece pivotee a quitte son port"
        );
        // Son enfant a suivi, en gardant sa distance : le bloc est rigide.
        let ecart_apres = (ch.piece(c).unwrap().centre() - contact).length();
        assert!(
            (ecart_avant - ecart_apres).abs() < 1e-3,
            "le sous-arbre s'est deforme : {ecart_avant} -> {ecart_apres}"
        );
        assert!((ch.roulis(b) - std::f32::consts::FRAC_PI_2).abs() < 1e-6, "roulis non retenu");
    }

    /// **Le roulis est un angle ABSOLU, pas un increment.** Rappeler avec la
    /// meme valeur ne doit rien bouger — sans quoi l'operation ne serait pas
    /// idempotente, donc penible a annuler et a rejouer.
    #[test]
    fn pivoter_deux_fois_au_meme_angle_ne_bouge_rien() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        ch.poser(p0, module(), 1);
        let b = ch.derniere_piece().unwrap();

        assert!(ch.pivoter(b, 0.8));
        let pose = ch.piece(b).unwrap().transforme;
        assert!(ch.pivoter(b, 0.8));
        let encore = ch.piece(b).unwrap().transforme;
        assert!(
            (pose - encore).abs_diff_eq(macroquad::prelude::Mat4::ZERO, 1e-4),
            "un second pivot au meme angle a bouge la piece"
        );
    }

    /// La racine ne pivote pas : elle n'est accouplee a rien, donc n'a pas
    /// d'axe d'accostage autour duquel tourner.
    #[test]
    fn pivoter_refuse_la_racine_et_les_inconnues() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let racine = ch.derniere_piece().unwrap();
        let temoin = (ch.nb_pieces(), ch.libres().len(), ch.version());
        assert!(!ch.pivoter(racine, 1.0), "la racine a pivote");
        assert!(!ch.pivoter(u64::MAX, 1.0), "une piece inconnue a pivote");
        assert_eq!((ch.nb_pieces(), ch.libres().len(), ch.version()), temoin, "un refus a mute");
    }

    /// **Le roulis survit a l'aller-retour par la recette** (section 6.4).
    /// Le rejouer a zero rendrait une geometrie differente de celle qu'on a
    /// sauvegardee, sans que rien ne le signale.
    #[test]
    fn le_roulis_survit_a_la_recette() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        assert!(ch.poser_roule(p0, module(), 1, 0.9));
        let pose = ch.pieces().map(|(_, p)| p.centre()).collect::<Vec<_>>();

        let recette = ch.recette();
        assert!(recette.iter().any(|e| e.roulis != 0.0), "la recette ne porte pas le roulis");
        let rejoue = Chantier::depuis_recette(&recette).expect("recette rejouable");
        let pose2 = rejoue.pieces().map(|(_, p)| p.centre()).collect::<Vec<_>>();
        assert_eq!(pose.len(), pose2.len());
        for (a, b) in pose.iter().zip(&pose2) {
            assert!(a.distance(*b) < 1e-4, "geometrie changee au rejeu : {a:?} vs {b:?}");
        }
    }

    #[test]
    fn racine_expose_ses_ports() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        assert_eq!(ch.nb_pieces(), 1);
        assert_eq!(ch.libres().len(), 6); // 2 écoutilles axiales + 4 montages Surface
    }

    #[test]
    fn poser_consomme_hote_et_ajoute_enfant() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let id = ch.libres()[0].id; // clipse un module par son écoutille −Z
        assert!(ch.poser(id, module(), 1));
        assert_eq!(ch.nb_pieces(), 2);
        // 6 (racine) − 1 (consommé) + 5 (ports restants de l'enfant) = 10
        assert_eq!(ch.libres().len(), 10);
    }

    #[test]
    fn poser_refuse_incompatible() {
        let mut ch = Chantier::new();
        ch.racine(module()); // ports ModuleAxial
        let id = ch.libres()[0].id; // port 0 = écoutille axiale → incompatible
        assert!(!ch.poser(id, panneau(), 0));
        assert_eq!(ch.nb_pieces(), 1);
        assert_eq!(ch.libres().len(), 6);
    }

    #[test]
    fn poser_refuse_hote_hors_bornes() {
        let mut ch = Chantier::new();
        ch.racine(module());
        assert!(!ch.poser(u64::MAX, module(), 1)); // id jamais distribué
        assert_eq!(ch.nb_pieces(), 1);
    }

    // --- L2.1 : identifiants stables de ports libres (`conception/assembleur.md` §6.1) ---
    //
    // Les trois tests demandés par la conception, dans l'ordre.

    // 1. L'identifiant d'un port libre survit à la pose d'une pièce **ailleurs**.
    //
    // Construit exprès pour faire jouer le `swap_remove` interne : `suivi` est
    // le **dernier** port de la tranche, `ailleurs` le **premier**. Poser sur
    // `ailleurs` retire l'élément 0 en le remplaçant par le dernier (`suivi`),
    // qui **change donc de position** sans que rien ne le prévienne — c'est
    // exactement le décalage que l'ancienne API (position brute) exposait aux
    // appelants. Seul l'`id` doit rester une désignation valide du même port.
    #[test]
    fn lidentifiant_dun_port_libre_survit_a_la_pose_ailleurs() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let libres = ch.libres();
        assert_eq!(libres.len(), 6);
        let suivi = *libres.last().unwrap();
        let ailleurs = libres[0].id;

        assert!(ch.poser(ailleurs, module(), 1));

        // Reposer sur l'id suivi doit atterrir **exactement** là où son port
        // se trouvait à l'origine — pas retomber, par coïncidence de forme,
        // sur un port quelconque qui aurait hérité de sa position dans la
        // tranche après le `swap_remove`. Comparer juste `nb_pieces()` ou un
        // simple `true` ne le distinguerait pas : les ports d'un même module
        // sont assez semblables pour qu'une pose au mauvais endroit réussisse
        // quand même.
        //
        // `suivi` (le dernier port de la tranche d'origine) peut être l'une ou
        // l'autre des deux familles de `module()` (écoutille axiale ou montage
        // Surface) : on choisit le composant compatible en conséquence.
        let (comp, montage) = match suivi.genre {
            GenrePort::ModuleAxial => (module(), 1),
            _ => (panneau(), 0),
        };
        let attendu = accoupler(suivi.repere, comp.ports()[montage].repere).to_mat4().transform_point3(Vec3::ZERO);
        assert!(ch.poser(suivi.id, comp, montage), "l'id suivi ne pose plus, alors que son port est resté libre");
        let obtenu = ch.piece(ch.derniere_piece().unwrap()).unwrap().transforme.transform_point3(Vec3::ZERO);
        assert!(obtenu.distance(attendu) < 1e-4, "la pose sur l'id suivi n'atterrit pas au bon endroit");
    }

    // 2. L'identifiant d'un port **consommé** ne se recycle jamais.
    //
    // Le compteur qui distribue les `id` ne fait que croître : ce test le
    // garde en observable, pour qu'une régression future (par exemple, un
    // recyclage « optimisé » des positions libérées) rougisse ici plutôt que
    // de se découvrir à l'usage, en intermittence, dans une UI.
    //
    // ⚠️ La forme qui compte : vérifier qu'**un** id précis (choisi une fois)
    // ne réapparaît pas ne suffit pas — un recyclage qui retombe sur d'autres
    // valeurs passerait à côté. L'invariant réel est qu'à **tout instant**,
    // tous les ports encore libres ont des `id` deux à deux distincts ; sinon
    // deux ports différents répondraient au même id, et poser sur l'un
    // poserait peut-être sur l'autre.
    #[test]
    fn lidentifiant_dun_port_consomme_ne_se_recycle_jamais() {
        use std::collections::HashSet;
        let mut ch = Chantier::new();
        ch.racine(module());
        for _ in 0..20 {
            let ids: Vec<u64> = ch.libres().iter().map(|p| p.id).collect();
            let uniques: HashSet<u64> = ids.iter().copied().collect();
            assert_eq!(ids.len(), uniques.len(), "deux ports libres partagent le même id");
            let Some(id) = ch.libres().first().map(|p| p.id) else { break };
            if !ch.poser(id, module(), 1) {
                break;
            }
        }
    }

    // 3. Poser sur un identifiant **périmé** échoue proprement.
    //
    // Le point qui compte vraiment (§6.1) : pas de panique, et surtout **rien
    // ne bouge** — une UI en retard sur le modèle ne doit ni planter, ni poser
    // au mauvais endroit en confondant l'id périmé avec une autre pose valide.
    #[test]
    fn poser_sur_un_identifiant_perime_echoue_proprement() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let id = ch.libres()[0].id;
        assert!(ch.poser(id, module(), 1)); // consomme le port : `id` devient périmé
        let (n, libres) = (ch.nb_pieces(), ch.libres().len());

        // Le composant choisi ici épouse **délibérément** le genre du port qui
        // occupe la position 0 de la tranche actuelle : un repli buggé sur
        // « position 0 » plutôt que sur l'id réussirait alors, et se
        // trahirait par les deux assertions qui suivent. Un composant fixe
        // (sans regarder ce qui est réellement à cette position) laisserait
        // passer ce bug par hasard de compatibilité.
        let sonde = ch.libres()[0];
        let (comp, montage) = match sonde.genre {
            GenrePort::ModuleAxial => (module(), 1),
            _ => (panneau(), 0),
        };
        assert!(!ch.poser(id, comp, montage), "le même id, déjà consommé, doit être refusé");
        assert_eq!(ch.nb_pieces(), n, "une pose refusée ne doit ajouter aucune pièce");
        assert_eq!(ch.libres().len(), libres, "une pose refusée ne doit toucher aucun port libre");
    }

    // --- L2.2 : retirer (`conception/assembleur.md` §6.2) ---
    //
    // Les trois tests demandés par la conception, puis la propriété
    // d'aller-retour qui les résume.

    // 1. Retirer une pièce retire aussi son sous-arbre.
    #[test]
    fn retirer_une_branche_emporte_ses_enfants() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let id_racine = ch.derniere_piece().unwrap();
        let port_a = ch.libres()[0].id; // port axial, cf. poser_consomme_hote_et_ajoute_enfant
        assert!(ch.poser(port_a, module(), 1));
        let id_a = ch.derniere_piece().unwrap();
        // B se pose sur l'écoutille axiale que A vient d'exposer.
        let port_b = ch.libres().iter().find(|p| p.origine == id_a && p.genre == GenrePort::ModuleAxial).unwrap().id;
        assert!(ch.poser(port_b, module(), 1));
        let id_b = ch.derniere_piece().unwrap();
        assert_eq!(ch.nb_pieces(), 3);

        assert!(ch.retirer(id_a));

        assert_eq!(ch.nb_pieces(), 1, "A et son enfant B doivent tous deux disparaître");
        assert!(ch.piece(id_racine).is_some(), "la racine doit rester");
        assert!(ch.piece(id_a).is_none());
        assert!(ch.piece(id_b).is_none(), "B (petit-enfant de la racine) doit disparaître avec son parent A");
    }

    // 2. Le port hôte redevient libre — avec la même géométrie, mais un `id`
    // neuf (§6.1 : un `id` consommé ne revient jamais).
    #[test]
    fn retirer_libere_le_port_qui_portait_la_piece() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let hote = ch.libres()[0]; // copie de la géométrie, avant consommation
        assert!(ch.poser(hote.id, module(), 1));
        let id_enfant = ch.derniere_piece().unwrap();
        assert!(ch.libres().iter().all(|p| p.id != hote.id), "le port hôte doit être consommé par la pose");

        assert!(ch.retirer(id_enfant));

        let restitue = ch
            .libres()
            .iter()
            .find(|p| p.origine == hote.origine && p.repere == hote.repere)
            .expect("le port hôte doit redevenir libre");
        assert_eq!(restitue.genre, hote.genre);
        assert_eq!(restitue.profil, hote.profil);
        assert_ne!(restitue.id, hote.id, "un port restitué reçoit un id neuf, l'ancien reste périmé");
    }

    // 3. Le budget est remboursé du coût **exact** des pièces retirées — ici
    // deux pièces (A et son enfant B), pas seulement celle qu'on nomme.
    #[test]
    fn retirer_rembourse_exactement_le_cout_pose() {
        let mut ch = Chantier::avec_budget(100.0);
        assert!(ch.racine(module()));
        let apres_racine = ch.budget_restant();
        let port_a = ch.libres()[0].id;
        assert!(ch.poser(port_a, module(), 1));
        let id_a = ch.derniere_piece().unwrap();
        let port_b = ch.libres().iter().find(|p| p.origine == id_a && p.genre == GenrePort::ModuleAxial).unwrap().id;
        assert!(ch.poser(port_b, module(), 1));
        assert!(ch.budget_restant() < apres_racine, "les deux poses doivent avoir coûté quelque chose");

        assert!(ch.retirer(id_a));

        assert!(
            (ch.budget_restant() - apres_racine).abs() < 1e-6,
            "le remboursement (A + B) doit ramener exactement le budget d'après-racine, obtenu {:.4} attendu {:.4}",
            ch.budget_restant(),
            apres_racine
        );
    }

    // Retirer un identifiant qui ne désigne aucune pièce vivante échoue
    // proprement — le pendant, côté pièces, de `poser_sur_un_identifiant_
    // perime_echoue_proprement` côté ports.
    #[test]
    fn retirer_sur_un_identifiant_inconnu_echoue_proprement() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let (n, libres, budget) = (ch.nb_pieces(), ch.libres().len(), ch.budget_restant());

        assert!(!ch.retirer(u64::MAX));

        assert_eq!(ch.nb_pieces(), n);
        assert_eq!(ch.libres().len(), libres);
        assert_eq!(ch.budget_restant(), budget);
    }

    /// Deux tranches de ports libres sont-elles le **même ensemble**, `id`
    /// mis à part ? Comparaison en multiset (l'ordre n'a aucune raison de se
    /// conserver : `poser` fait un `swap_remove`, `retirer` réinsère en fin
    /// de tranche).
    fn meme_ports_a_lid_pres(a: &Chantier, b: &Chantier) -> bool {
        let projeter = |ch: &Chantier| -> Vec<(Repere, GenrePort, Profil, u64)> {
            ch.libres().iter().map(|p| (p.repere, p.genre, p.profil, p.origine)).collect()
        };
        let (mut pa, mut pb) = (projeter(a), projeter(b));
        if pa.len() != pb.len() {
            return false;
        }
        pa.retain(|x| match pb.iter().position(|y| y == x) {
            Some(i) => {
                pb.remove(i);
                false
            }
            None => true,
        });
        pa.is_empty()
    }

    // **La propriété qui résume les trois tests ci-dessus** (§6.2) : poser
    // puis retirer ramène le chantier à un état indiscernable de l'état
    // initial — mêmes pièces, mêmes ports libres, même budget restant. Elle
    // vaut plus que les tests séparés parce qu'elle reste vraie même si un
    // champ est ajouté à `Chantier` que les trois autres ignoreraient.
    #[test]
    fn poser_puis_retirer_ramene_a_un_etat_indiscernable() {
        let mut avant = Chantier::avec_budget(50.0);
        assert!(avant.racine(module()));

        let mut ch = Chantier::avec_budget(50.0);
        assert!(ch.racine(module()));
        let cible = ch.libres()[0].id; // port axial, cf. poser_consomme_hote_et_ajoute_enfant
        assert!(ch.poser(cible, module(), 1));
        let id_enfant = ch.derniere_piece().unwrap();
        assert!(ch.nb_pieces() > avant.nb_pieces() && ch.budget_restant() < avant.budget_restant());

        assert!(ch.retirer(id_enfant));

        assert_eq!(ch.nb_pieces(), avant.nb_pieces(), "même nombre de pièces");
        assert!((ch.budget_restant() - avant.budget_restant()).abs() < 1e-6, "même budget restant");
        assert!(meme_ports_a_lid_pres(&ch, &avant), "mêmes ports libres (id neuf mis à part)");
    }

    // --- L2.3 : annuler/refaire (`conception/assembleur.md` §6.3) ---

    /// État observable complet : pièces (`id` + contenu) et ports libres —
    /// **pas** l'historique lui-même, qui n'a pas à correspondre (après *n*
    /// opérations puis *n* annulations, `pile_annuler` est vide des deux
    /// côtés mais `pile_refaire` ne l'est pas forcément). Contrairement au
    /// `retirer` de L2.2, un `annuler`/`refaire` restitue l'état **exact**
    /// (même instantané, pas une reconstruction) : comparer les `id` bruts a
    /// donc un sens ici, pas besoin du multiset qui les ignorait en L2.2.
    fn empreinte(ch: &Chantier) -> (Vec<(u64, Piece)>, Vec<PortLibre>, f32) {
        (ch.pieces().map(|(id, p)| (id, p.clone())).collect(), ch.libres().to_vec(), ch.budget_restant())
    }

    /// Une opération aléatoire, jamais sur la racine (la garder évite de
    /// vider le chantier et de bloquer le reste de la séquence). `true` si
    /// elle a réellement muté le chantier — pose acceptée ou retrait réussi
    /// — `false` sinon (tentative refusée : genre incompatible, collision,
    /// budget, ou rien à retirer). Seules les opérations qui **réussissent**
    /// comptent dans le *n* de la propriété d'aller-retour : celles qui
    /// échouent ne touchent ni l'état ni l'historique.
    fn operation_aleatoire(ch: &mut Chantier, rng: &mut Rng) -> bool {
        let candidats = [module(), panneau()];
        if rng.chance(0.6) {
            let ids: Vec<u64> = ch.libres().iter().map(|p| p.id).collect();
            if ids.is_empty() {
                return false;
            }
            let id = rng.choix(&ids);
            let comp = candidats[rng.choix(&[0usize, 1usize])].clone();
            let montage = rng.choix(&[0usize, 1usize]);
            ch.poser(id, comp, montage)
        } else {
            let ids: Vec<u64> = ch.pieces().map(|(id, _)| id).collect();
            if ids.len() <= 1 {
                return false; // rien d'autre que la racine
            }
            let id = rng.choix(&ids[1..]); // jamais ids[0] (la racine, toujours en tête)
            ch.retirer(id)
        }
    }

    #[test]
    fn annuler_defait_la_derniere_pose() {
        // Un budget actif, pas `Chantier::new()` : sans lui, `budget` reste
        // `None` avant comme après, et un `restaurer` qui oublierait ce champ
        // passerait inaperçu — `empreinte` doit avoir quelque chose à perdre.
        let mut ch = Chantier::avec_budget(50.0).avec_historique();
        assert!(ch.racine(module()));
        let avant = empreinte(&ch);
        let id = ch.libres()[0].id;
        assert!(ch.poser(id, module(), 1));
        assert_ne!(empreinte(&ch), avant, "la pose doit avoir un effet observable, sans quoi le test ne prouve rien");

        assert!(ch.annuler());
        assert_eq!(empreinte(&ch), avant, "annuler doit ramener exactement à l'état d'avant la pose");
    }

    #[test]
    fn refaire_refait_la_pose_defaite() {
        // Budget actif, même raison qu'`annuler_defait_la_derniere_pose`.
        let mut ch = Chantier::avec_budget(50.0).avec_historique();
        assert!(ch.racine(module()));
        let id = ch.libres()[0].id;
        assert!(ch.poser(id, module(), 1));
        let apres = empreinte(&ch);

        assert!(ch.annuler());
        assert!(ch.refaire());
        assert_eq!(empreinte(&ch), apres, "refaire doit ramener exactement à l'état d'après la pose");
    }

    // Historique non activé (pas d'`avec_historique`) : `annuler`/`refaire`
    // échouent **toujours**, proprement — jamais de panique sur une pile
    // qu'on n'a jamais commencé à remplir.
    #[test]
    fn annuler_et_refaire_sans_historique_echouent_toujours() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let id = ch.libres()[0].id;
        assert!(ch.poser(id, module(), 1));
        let avant = empreinte(&ch);

        assert!(!ch.annuler());
        assert!(!ch.refaire());
        assert_eq!(empreinte(&ch), avant, "un échec d'annuler/refaire ne doit rien changer");
    }

    // Une nouvelle opération réussie invalide tout ce qu'on aurait pu
    // refaire — sans quoi « refaire » rejouerait une branche que la nouvelle
    // pose a rendue caduque.
    #[test]
    fn une_nouvelle_operation_invalide_la_pile_de_refaire() {
        let mut ch = Chantier::new().avec_historique();
        assert!(ch.racine(module()));
        let id1 = ch.libres()[0].id;
        assert!(ch.poser(id1, module(), 1));
        assert!(ch.annuler());
        assert!(ch.refaire(), "refaire doit fonctionner avant toute nouvelle opération");
        assert!(ch.annuler());

        let id2 = ch.libres()[0].id;
        assert!(ch.poser(id2, module(), 1));
        assert!(!ch.refaire(), "la nouvelle pose doit invalider le refaire laissé par l'annulation précédente");
    }

    // **La propriété qui résume tout §6.3**, exercée par une séquence
    // pseudo-aléatoire rejouable par graine plutôt qu'un scénario écrit à la
    // main — c'est précisément ce que la conception demande, et le motif qui
    // avait laissé passer les six mesures fausses de l'ISV (§3 du document de
    // conception) était toujours le même : des cas choisis par quelqu'un qui
    // savait déjà ce qu'il cherchait.
    #[test]
    fn n_operations_puis_n_annulations_ramenent_a_letat_initial() {
        for graine in 0..30u64 {
            let mut ch = Chantier::avec_budget(500.0).avec_historique();
            assert!(ch.racine(module()));

            // Une empreinte après **chaque** opération réussie, pas
            // seulement au début et à la fin. `restaurer` écrase tout
            // l'état d'un coup : un instantané pris au mauvais moment sur
            // une opération du milieu (ex. juste après au lieu de juste
            // avant) ne fausse que l'étape où il ressort, la retombée finale
            // après *n* annulations peut coïncider par hasard si seule la
            // toute première opération compte réellement. Vérifier chaque
            // étape, pas seulement les deux bouts, referme ce trou.
            let mut etapes = vec![empreinte(&ch)]; // étapes[k] = état après k opérations
            let mut rng = Rng::new(graine);
            for _ in 0..40 {
                if operation_aleatoire(&mut ch, &mut rng) {
                    etapes.push(empreinte(&ch));
                }
            }
            let n = etapes.len() - 1;
            assert!(n > 0, "graine {graine} : aucune opération n'a réussi, le test ne prouve rien");

            for (k, attendu) in etapes.iter().enumerate().take(n).rev() {
                assert!(ch.annuler(), "graine {graine} : annulation {k} attendue mais refusée");
                assert_eq!(&empreinte(&ch), attendu, "graine {graine} : après annulation, l'état ne correspond pas à celui d'avant l'opération {}", k + 1);
            }

            for (k, attendu) in etapes.iter().enumerate().take(n + 1).skip(1) {
                assert!(ch.refaire(), "graine {graine} : refait {k} attendu mais refusé");
                assert_eq!(&empreinte(&ch), attendu, "graine {graine} : après refait, l'état ne correspond pas à celui d'après l'opération {k}");
            }
        }
    }

    // --- L2.5 : sauvegarde et aller-retour géométrique (§6.4) ---

    /// Les pièces seules (composant + transformée cuite), sans les `id` de
    /// session — `depuis_recette` part toujours d'un compteur à zéro, donc
    /// comparer les `id` comparerait une coïncidence, pas la géométrie que
    /// §6.4 demande de reproduire.
    fn pieces_seules(ch: &Chantier) -> Vec<Piece> {
        ch.pieces().map(|(_, p)| p.clone()).collect()
    }

    // **La propriété centrale de L2.5** : rejouer la recette reproduit la
    // géométrie cuite au sommet près.
    #[test]
    fn recette_puis_depuis_recette_reproduit_la_geometrie_cuite() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let id1 = ch.libres()[0].id; // port axial, cf. poser_consomme_hote_et_ajoute_enfant
        assert!(ch.poser(id1, module(), 1));
        let id_a = ch.derniere_piece().unwrap();
        let id2 = ch.libres().iter().find(|p| p.origine == id_a && p.genre == GenrePort::Surface).unwrap().id;
        assert!(ch.poser(id2, panneau(), 0));

        let recette = ch.recette();
        assert_eq!(recette.len(), 3, "racine + 2 poses");

        let rejoue = Chantier::depuis_recette(&recette).expect("la recette doit se rejouer");

        assert_eq!(
            pieces_seules(&rejoue),
            pieces_seules(&ch),
            "le rejeu doit reproduire la géométrie cuite au sommet près"
        );
    }

    // **Le piège que §6.4 signale explicitement** : un `Composant::SousEnsemble`
    // porte des `Piece` déjà cuites (`Rc<DonneesSousEnsemble>`). Décidé :
    // sérialisé **tel quel** (baked), pas rejoué comme une sous-recette —
    // voir `docs/suivi/assembleur.md` §L2.5 pour la discussion. Ce test
    // vérifie que ce choix n'empêche pas le rejeu de la recette qui le
    // *contient*.
    #[test]
    fn recette_reproduit_un_sous_ensemble_baked_a_lidentique() {
        let mut interne = Chantier::new();
        assert!(interne.racine(module()));
        let id = interne.libres()[0].id;
        assert!(interne.poser(id, module(), 1));
        let compo = interne.figer(Profil::P1).expect("un composite de deux modules");

        let mut ch = Chantier::new();
        assert!(ch.racine(Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six }));
        // Volontairement **pas** `libres()[0]` : un nœud à six sorties a six
        // ports de même genre/profil mais à des orientations différentes —
        // n'importe lequel conviendrait à un rejeu qui ignorerait l'indice
        // exact et retomberait sur le premier par coïncidence.
        let port = ch.libres()[3].id;
        let montage = compo
            .ports()
            .iter()
            .position(|p| p.genre == GenrePort::ModuleAxial)
            .expect("le composite garde une écoutille libre");
        assert!(ch.poser(port, compo, montage), "le composite se pose comme un composant normal");

        let recette = ch.recette();
        let rejoue = Chantier::depuis_recette(&recette).expect("une recette avec un SousEnsemble doit se rejouer");

        assert_eq!(pieces_seules(&rejoue), pieces_seules(&ch));
    }

    // La recette n'est utile comme **format de sauvegarde** que si elle
    // survit vraiment un aller-retour texte, pas seulement une copie en
    // mémoire — sérialiser en JSON (`serde_json`, déjà une dépendance du
    // projet) et relire est ce qu'un vrai fichier de sauvegarde ferait.
    #[test]
    fn la_recette_survit_a_un_aller_retour_json() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        // Un port Surface, pas `libres()[0]` (l'écoutille axiale) — même
        // raison que dans `recette_reproduit_un_sous_ensemble_baked_a_lidentique`.
        let id = ch.libres().iter().find(|p| p.genre == GenrePort::Surface).unwrap().id;
        assert!(ch.poser(id, panneau(), 0));

        let recette = ch.recette();
        let json = serde_json::to_string(&recette).expect("la recette doit se sérialiser en JSON");
        let relue: Recette = serde_json::from_str(&json).expect("le JSON doit se relire");

        assert_eq!(relue, recette, "la relecture doit reproduire la recette exacte");
        let rejoue = Chantier::depuis_recette(&relue).expect("la recette relue doit se rejouer");
        assert_eq!(pieces_seules(&rejoue), pieces_seules(&ch));
    }

    // Une recette tronquée (un hôte qui pointe sur une pièce pas encore
    // posée) échoue proprement — `None`, pas de panique. Le pendant, pour la
    // sauvegarde, de `poser_sur_un_identifiant_perime_echoue_proprement` et
    // `retirer_sur_un_identifiant_inconnu_echoue_proprement`.
    #[test]
    fn depuis_recette_sur_une_recette_corrompue_echoue_proprement() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let id = ch.libres()[0].id;
        assert!(ch.poser(id, module(), 1));

        let mut recette = ch.recette();
        // L'étape 1 (le module posé) prétend avoir pour hôte la pièce
        // d'indice 5 — qui n'existe pas encore à ce point de la séquence.
        recette[1].hote = Some((5, 0));

        assert!(Chantier::depuis_recette(&recette).is_none());
    }

    // ---- L3.1 `pose_prevue` : la source unique du fantôme (§8.3) ----

    // La propriété qui fait tout l'intérêt de la méthode : ce que le fantôme
    // annonce est *exactement* ce que le clic pose. Balayée sur chaque port
    // libre × chaque montage plutôt que sur un cas choisi — un test qui ne
    // viserait que `libres()[0]` ne distinguerait pas `pose_prevue` d'une
    // version qui retomberait toujours sur le premier port venu, le défaut
    // trouvé deux fois déjà (L2.1, puis L2.5).
    #[test]
    fn pose_prevue_annonce_exactement_la_pose_que_poser_produit() {
        let mut compares = 0;
        for cible in [module(), panneau()] {
            for port_pos in 0..6 {
                for montage in 0..cible.ports().len() {
                    let mut ch = Chantier::new();
                    assert!(ch.racine(module()));
                    let hote = ch.libres()[port_pos].id;
                    let prevue = ch.pose_prevue(hote, &cible, montage);
                    if !ch.poser(hote, cible.clone(), montage) {
                        continue; // refusée : comparé par le test suivant
                    }
                    let posee = ch.piece(ch.derniere_piece().unwrap()).unwrap();
                    assert_eq!(
                        prevue.as_ref(),
                        Some(posee),
                        "port {port_pos}, montage {montage} : le fantôme ment sur la pose"
                    );
                    compares += 1;
                }
            }
        }
        // Sans ce garde-fou, un balayage qui dégénère à zéro comparaison
        // passerait au vert sans rien avoir vérifié.
        assert!(compares >= 8, "balayage trop maigre : {compares} poses comparées");
    }

    // §8.5 en dépend directement : l'overlay trace le segment de plus courte
    // approche entre le fantôme et la pièce qui le **refuse**. Un fantôme qui
    // n'existerait que pour les poses acceptées rendrait ce tracé impossible —
    // or c'est précisément le cas qu'il doit expliquer.
    #[test]
    fn pose_prevue_repond_meme_quand_la_pose_serait_refusee() {
        // Refus par incompatibilité de genre : un panneau sur une écoutille axiale.
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let axial = ch.libres().iter().find(|p| p.genre == GenrePort::ModuleAxial).unwrap().id;
        assert!(!ch.peut_poser(axial, &panneau(), 0), "le scénario doit bien être un refus");
        assert!(ch.pose_prevue(axial, &panneau(), 0).is_some(), "refus ≠ pose indéfinie");

        // Refus par anti-collision : la deuxième aile large de
        // `collision_rejette_recouvrement`, celle que le fantôme doit montrer.
        let large = |l: f32| Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: l,
            largeur: 8.0,
        };
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let ix = ch.libres().iter().find(|p| p.repere.pos.x > 0.9).unwrap().id;
        assert!(ch.poser(ix, large(7.0), 0));
        let iy = ch.libres().iter().find(|p| p.repere.pos.y > 0.9).unwrap().id;
        assert!(!ch.peut_poser(iy, &large(7.0), 0), "le scénario doit bien être un refus");
        let fantome = ch.pose_prevue(iy, &large(7.0), 0);
        assert!(fantome.is_some(), "l'overlay §8.5 a besoin du fantôme d'une pose refusée");
        // Et il doit être à la bonne place, pas un repli quelconque : la pose
        // refusée reste géométriquement définie, sur le port visé.
        let pos = fantome.unwrap().transforme.w_axis.truncate();
        assert!(pos.y > 0.9, "fantôme posé ailleurs que sur le port visé : {pos:?}");
    }

    // ---- L3.2 `sous_arbre` : ce que le surlignage doit montrer (§8.3) ----

    /// Arbre de test à trois niveaux, reconstruit à l'identique à chaque appel
    /// (les `id` sortent d'un compteur qui repart de zéro avec le chantier) :
    ///
    /// ```text
    ///   a ─┬─ b ─┬─ c   (chaîne axiale de modules)
    ///      │     └─ d   (panneau)
    ///      └─ e         (panneau)
    /// ```
    fn arbre() -> (Chantier, [u64; 5]) {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let a = ch.derniere_piece().unwrap();

        let greffe = |ch: &mut Chantier, sur: u64, genre: GenrePort, comp: Composant, m: usize| {
            let p = ch.libres().iter().find(|p| p.origine == sur && p.genre == genre).unwrap().id;
            assert!(ch.poser(p, comp, m), "pose de montage refusée dans le montage de test");
            ch.derniere_piece().unwrap()
        };

        let b = greffe(&mut ch, a, GenrePort::ModuleAxial, module(), 1);
        let c = greffe(&mut ch, b, GenrePort::ModuleAxial, module(), 1);
        let d = greffe(&mut ch, b, GenrePort::Surface, panneau(), 0);
        let e = greffe(&mut ch, a, GenrePort::Surface, panneau(), 0);

        (ch, [a, b, c, d, e])
    }

    // La **forme** de l'arbre, pinnée indépendamment de `retirer`.
    //
    // Le test d'accord ci-dessous ne peut pas la garder : `retirer` appelle
    // désormais `sous_arbre`, donc une version dégénérée qui rendrait `{id}`
    // seul les mettrait d'accord tous les deux — sur un arbre faux. C'est
    // exactement le constat de L3.1 (sabotage 1), tiré à l'avance cette fois.
    #[test]
    fn le_sous_arbre_liste_toute_la_descendance() {
        let (ch, [a, b, c, d, e]) = arbre();
        assert_eq!(ch.sous_arbre(a), HashSet::from([a, b, c, d, e]), "la racine emporte tout");
        assert_eq!(ch.sous_arbre(b), HashSet::from([b, c, d]), "une branche et ses deux enfants");
        assert_eq!(ch.sous_arbre(c), HashSet::from([c]), "feuille en bout de chaîne");
        assert_eq!(ch.sous_arbre(e), HashSet::from([e]), "feuille d'une autre branche");
        assert!(ch.sous_arbre(u64::MAX).is_empty(), "id jamais distribué");
    }

    // §8.3 : ce qu'on surligne doit être exactement ce que la touche Suppr
    // emporte. Balayé sur chaque pièce de l'arbre, pas sur un cas choisi.
    #[test]
    fn le_sous_arbre_est_exactement_ce_que_retirer_emporte() {
        let (reference, ids) = arbre();
        let toutes: HashSet<u64> = reference.pieces().map(|(id, _)| id).collect();
        for cible in ids {
            let (mut ch, _) = arbre();
            let annonce = ch.sous_arbre(cible);
            assert!(ch.retirer(cible));
            let restantes: HashSet<u64> = ch.pieces().map(|(id, _)| id).collect();
            let emportees: HashSet<u64> = toutes.difference(&restantes).copied().collect();
            assert_eq!(annonce, emportees, "pièce {cible} : surlignage ≠ ce que Suppr emporte");
        }
    }

    // ---- L3.3 `piece_sous_rayon` : la moitié « pièce » de la désignation ----

    // Deux modules alignés sur Z, visés **par les deux bouts** : la pièce
    // désignée doit changer avec le point de vue. Viser d'un seul côté ne
    // distinguerait pas un tri correct d'un `find` qui rendrait toujours la
    // première pièce de la liste — le défaut trouvé en L2.1 puis en L2.5.
    #[test]
    fn piece_sous_rayon_prend_la_plus_proche_de_loeil() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let a = ch.derniere_piece().unwrap();
        let port = ch.libres().iter().find(|p| p.genre == GenrePort::ModuleAxial).unwrap().id;
        assert!(ch.poser(port, module(), 1));
        let b = ch.derniere_piece().unwrap();

        let za = ch.piece(a).unwrap().transforme.w_axis.z;
        let zb = ch.piece(b).unwrap().transforme.w_axis.z;
        assert!((za - zb).abs() > 1.0, "les deux modules doivent être franchement séparés");

        // De très loin, dans l'axe, en regardant vers l'autre : on voit d'abord
        // celui de son côté.
        let loin = 100.0;
        let (proche_de_a, proche_de_b) = if za < zb { (-loin, loin) } else { (loin, -loin) };
        assert_eq!(
            ch.piece_sous_rayon(vec3(0.0, 0.0, proche_de_a), Vec3::Z * (zb - za).signum()),
            Some(a),
            "vu du côté de A"
        );
        assert_eq!(
            ch.piece_sous_rayon(vec3(0.0, 0.0, proche_de_b), Vec3::Z * (za - zb).signum()),
            Some(b),
            "vu du côté de B"
        );
    }

    #[test]
    fn un_rayon_qui_manque_la_station_ne_designe_rien() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        assert_eq!(ch.piece_sous_rayon(vec3(0.0, 50.0, -100.0), Vec3::Z), None, "passe au-dessus");
        assert_eq!(ch.piece_sous_rayon(vec3(0.0, 0.0, -100.0), -Vec3::Z), None, "part à l'opposé");
        assert!(ch.piece_sous_rayon(vec3(0.0, 0.0, -100.0), Vec3::Z).is_some(), "droit dessus");
    }

    #[test]
    fn pose_prevue_echoue_proprement_sur_un_port_perime_ou_un_montage_invalide() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let id = ch.libres()[0].id;
        assert!(ch.pose_prevue(u64::MAX, &module(), 1).is_none(), "id jamais distribué");
        assert!(ch.pose_prevue(id, &module(), 99).is_none(), "montage hors bornes");
        assert!(ch.poser(id, module(), 1));
        assert!(ch.pose_prevue(id, &module(), 1).is_none(), "port déjà consommé");
    }

    #[test]
    fn budget_limite_les_poses() {
        // module coûte 5 ; budget 7 → racine (5) ok, 2 restant < 5 → pose refusée.
        let mut ch = Chantier::avec_budget(7.0);
        assert!(ch.racine(module()));
        let id = ch.libres()[0].id;
        assert!(!ch.poser(id, module(), 1));
        assert_eq!(ch.nb_pieces(), 1);
    }

    #[test]
    fn compatibles_liste_les_ports() {
        let mut ch = Chantier::new();
        ch.racine(module());
        assert_eq!(ch.compatibles(&module(), 1).len(), 2); // 2 ports axiaux compatibles
        assert_eq!(ch.compatibles(&panneau(), 0).len(), 4); // 4 montages Surface radiaux
    }

    // Deux ailes **parallèles et trop proches** se recouvrent vraiment : posées
    // du même côté de deux modules chaînés, elles sont séparées de la longueur
    // d'un module (2) alors qu'elles font 4 de large. Elles s'interpénètrent, et
    // c'est ce que l'anti-collision doit voir.
    //
    // ⚠️ Ce test mesurait autrefois deux ailes **perpendiculaires** (+X et +Y),
    // que les sphères englobantes rejetaient. C'était un **faux positif** : deux
    // ailes minces à angle droit ne se touchent pas, et c'est exactement le
    // refus injustifié qui a motivé le passage aux capsules — voir
    // `deux_ailes_perpendiculaires_ne_se_genent_pas` juste en dessous.
    #[test]
    fn collision_rejette_recouvrement() {
        let large = |l: f32| Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: l,
            largeur: 8.0,
        };
        let mut ch = Chantier::new();
        ch.racine(module()); // module long 2 → ports Surface ±X, ±Y à r=1
        // Deux ailes **très larges** (8 de large) sur deux faces voisines d'un
        // module qui n'en fait que 2 : elles partent à 1 de l'axe et débordent
        // chacune de 4 en travers, donc elles se recouvrent franchement près de
        // la racine — quelle que soit la primitive qui les mesure.
        let ix = ch.libres().iter().find(|p| p.repere.pos.x > 0.9).unwrap().id;
        assert!(ch.poser(ix, large(7.0), 0));
        let iy = ch.libres().iter().find(|p| p.repere.pos.y > 0.9).unwrap().id;
        assert!(!ch.poser(iy, large(7.0), 0), "deux ailes larges à 90° se recouvrent à la racine");
    }

    // **Le faux positif que les capsules suppriment.** Deux ailes minces posées
    // à angle droit sur le même module ne se touchent pas : une sphère
    // englobante les déclarait pourtant en collision, parce qu'elle réserve la
    // longueur de l'aile **dans toutes les directions**. Sans conséquence pour
    // le générateur, qui réessaie ailleurs — rédhibitoire pour un éditeur, où
    // c'est l'utilisateur qui a choisi l'emplacement.
    #[test]
    fn deux_ailes_perpendiculaires_ne_se_genent_pas() {
        let aile = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 7.0,
            largeur: 1.5,
        };
        let mut ch = Chantier::new();
        ch.racine(module());
        let ix = ch.libres().iter().find(|p| p.repere.pos.x > 0.9).unwrap().id;
        assert!(ch.poser(ix, aile.clone(), 0));
        let iy = ch.libres().iter().find(|p| p.repere.pos.y > 0.9).unwrap().id;
        assert!(ch.poser(iy, aile, 0), "deux ailes à 90° ne se recouvrent pas");
        assert_eq!(ch.nb_pieces(), 3);
    }

    #[test]
    fn treillis_accueille_appendices_via_chantier() {
        let mut ch = Chantier::new();
        ch.racine(Composant::Treillis { profil: Profil::P1, longueur: 6.0, style: StyleTreillis::Carre });
        // Docke un panneau sur chaque port Surface libre (ils ne prolifèrent pas).
        let mut poses = 0;
        while let Some(id) = ch.libres().iter().find(|p| p.genre == GenrePort::Surface).map(|p| p.id) {
            assert!(ch.poser(id, panneau(), 0));
            poses += 1;
        }
        assert!(poses >= 2, "au moins une paire de montages");
        assert!(matches!(ch.terminer(), EtatStation::Prete(_)));
    }
}
