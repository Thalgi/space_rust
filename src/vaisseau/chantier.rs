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

use super::{
    accoupler, cuire, Budget, Composant, DonneesSousEnsemble, EtatStation, GenrePort, Piece, Port,
    Enveloppe, Profil, Repere, Station,
};
use macroquad::prelude::Vec3;
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
        self.pieces.push(Entree { id, piece: cuire(corps, &comp), hote: None }); // dernier usage : consomme `comp`
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
        let Some((pos, hote)) = self.trouver(hote_id) else { return false };
        let Some((montage, corps)) = Self::corps_prevu(&hote, &comp, montage_idx) else {
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
        self.pieces.push(Entree { id, piece: cuire(corps, &comp), hote: Some((hote, montage_idx)) }); // dernier usage : consomme `comp`
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
        let (_, hote) = self.trouver(hote_id)?;
        let (_, corps) = Self::corps_prevu(&hote, comp, montage_idx)?;
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
                Etape { composant: e.piece.composant.clone(), hote, montage }
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
                    ch.poser(port_id, etape.composant.clone(), etape.montage)
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
        let montage = *comp.ports().get(montage_idx)?;
        Some((montage, accoupler(hote.repere, montage.repere)))
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

    fn restaurer(&mut self, i: Instantane) {
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
    fn enregistrer(&mut self, avant: Option<Instantane>) {
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

            for k in (0..n).rev() {
                assert!(ch.annuler(), "graine {graine} : annulation {k} attendue mais refusée");
                assert_eq!(empreinte(&ch), etapes[k], "graine {graine} : après annulation, l'état ne correspond pas à celui d'avant l'opération {}", k + 1);
            }

            for k in 1..=n {
                assert!(ch.refaire(), "graine {graine} : refait {k} attendu mais refusé");
                assert_eq!(empreinte(&ch), etapes[k], "graine {graine} : après refait, l'état ne correspond pas à celui d'après l'opération {k}");
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
