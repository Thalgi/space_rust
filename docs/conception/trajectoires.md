# Conception — Trajectoires : vaisseaux et stations mobiles

> Document de conception, **2026-08-14**. Écrit **avant tout code**, à
> l'ouverture de ce qui est annoncé comme le plus gros chantier du projet à
> ce jour : faire qu'un vaisseau assemblé dans le VAB (`vaisseau/`) — ou une
> station déjà en orbite — puisse **voyager réellement** d'un astre à un
> autre, au lieu de rester posé sur un rail fixe ou fini sans destination.
>
> Il fait ce que fait tout document de `conception/` dans ce projet : dire
> **ce qui existe déjà et sur quoi s'appuyer** (§1–§2), **ce qui manque**
> honnêtement, y compris ce qui ne saute pas aux yeux (§3). Une première
> version s'arrêtait là, sur une liste de questions sans réponse. **Une
> session de discussion avec l'utilisateur (2026-08-14, après
> [`concept_jeu.md`](concept_jeu.md)) les a tranchées** — §4 consigne ces
> décisions, §5 leurs conséquences techniques, §6 un découpage en lots
> aligné dessus. Ce qui reste vraiment ouvert est isolé en §7, pour ne pas
> se mélanger avec ce qui est acté.

---

## 1. Le problème, en une phrase

Aujourd'hui, tout `Astre` qui a un parent (lune, astéroïde, ISS) est mû par
`orbiter_autour(centre, dt)` (`src/astre.rs:212`, appelé depuis
`systeme/mod.rs:474`) : sa position à l'instant *t* est une **fonction
analytique fermée** d'un angle de phase autour d'**un seul foyer fixe**, ré-
évaluée chaque frame, sans mémoire d'un instant à l'autre. C'est exactement
le mécanisme des lunes, transposé tel quel à `EnginOrbital` (`src/engin.rs`)
pour l'ISS.

Un vaisseau **en transfert** n'entre pas dans ce moule : il n'a pas de foyer
fixe par définition — il quitte la sphère d'influence d'un astre pour
rallier celle d'un autre, et sa position à l'instant *t* dépend de son
**histoire** (vitesse acquise, poussées passées), pas d'une formule fermée
autour d'un centre unique. C'est un changement de nature du modèle de
mouvement, pas un paramètre de plus sur l'existant.

## 2. Ce qui existe déjà, et sur quoi s'appuyer

Rien ne part de zéro — trois chantiers clos fournissent chacun un morceau :

| Brique | Où | Ce qu'elle donne déjà |
|---|---|---|
| **L'assembleur (VAB)** | `src/vaisseau/` | Un modèle complet de construction (`Chantier`, `EtatStation`, ports, symétrie, sauvegarde disque) qui **cuit** un design en `MaillageStation`. Le Starship et l'ISV sont des designs finis, testés, produits par ce chemin. |
| **Le pont vaisseau ↔ système** | `src/engin.rs` | `EnginOrbital` sait déjà prendre un `EtatStation` cuit, le mettre à l'échelle du monde et le faire orbiter — mais avec un **foyer et un rayon d'orbite figés à la construction** (voir `ORBITE_ISS`, `INCLINAISON_ISS`) : un seul cas concret aujourd'hui, l'ISS autour de la Terre. |
| **Le rendu de trajectoires** | `systeme/rendu.rs`, `conception/interface.md` | Le tracé en semis de points à espacement écran constant existe déjà pour les orbites elliptiques fixes des planètes/lunes. Réutilisable pour **prévisualiser** un transfert, mais conçu pour une ellipse fermée, pas un arc ouvert entre deux foyers. |
| **Le contrôle du temps** | déjà en jeu (bucket list §5 bis) | Espace = pause, Haut/Bas = ×0,125 à ×16. Un transfert qui dure des mois simulés devra rester lisible sous cette accélération, pas la contourner. |
| **La gravité N-corps** | `src/systeme/gravite.rs` | Existe, mais **seulement pour les étoiles** des systèmes multiples (foyers mobiles entre eux) — jamais utilisée pour un corps sous poussée. Le principe (intégrer une accélération plutôt qu'évaluer une formule fermée) est le bon voisin technique, la portée est différente (un couple de foyers massifs, pas un mobile léger qui traverse plusieurs sphères d'influence). |

Le Starship est le cas qui motive tout ça : **terminé depuis le 2026-08-06**
(`suivi/stations.md` §F.10, 5/5), un objet fini « qui n'a nulle part où
aller » (bucket list §10 bis). C'est la pièce que ce chantier doit enfin
faire décoller.

## 3. Ce qui manque — et la liste n'est pas courte

### 3.1 Aucune donnée physique sur les pièces

`vaisseau/composant/propulsion.rs`, `reservoir.rs`, `raptor.rs` sont
**purement géométriques** aujourd'hui : rayon, hauteur, gabarit d'enveloppe,
peinture. Aucun champ de poussée, d'impulsion spécifique, de capacité
d'ergols, de masse sèche ou de masse embarquée n'existe nulle part dans
`vaisseau/`. Un moteur Raptor et une cuve vide sont, du point de vue du
modèle actuel, deux formes sans conséquence sur rien. **Sans un minimum de
ces grandeurs, aucune notion de delta-v, de temps de combustion ou de portée
n'a de sens** — c'est le préalable, pas un lot parmi d'autres. Allégé par la
décision 2 (§4) : pas de simulation complète par pièce requise pour le
premier jet, seulement des modificateurs sur une base par classe.

### 3.2 Le modèle d'orbite ne sait pas changer de foyer

`Foyer` (`systeme/mod.rs`) distingue déjà `Etoile(idx)`/`Barycentre`, mais
c'est une propriété **fixée à la création** de l'astre, jamais réévaluée. Un
vaisseau qui quitte la sphère d'influence de la Terre pour celle de Mars n'a
pas de représentant dans ce modèle : il faudrait soit un foyer qui change en
cours de vie, soit un mode de mouvement entièrement différent (position/
vitesse intégrées) pendant la phase de transfert, avec un retour vers
`orbiter_autour` une fois amarré ou capturé.

### 3.3 Aucune UI de planification

Rien n'existe pour viser un astre, prévisualiser un coût (delta-v, durée),
poser un nœud de manœuvre ou confirmer un départ. Le sélecteur actuel
(`ecran/selecteur.rs`) liste des astres immobiles dans une hiérarchie
étoile → planète → lune ; il ne distingue pas un objet **posé** (la station
ISS, fixe dans son orbite) d'un objet **mobile** qu'on est en train de
piloter ou d'observer en transit.

### 3.4 Aucun rendez-vous avec un corps mobile

Arriver *quelque part* est déjà dur ; arriver **à la bonne heure sur un
astre qui a lui-même bougé pendant le transfert** (rendez-vous orbital) est
un cran au-dessus. Les lunes et l'ISS bougent selon `orbiter_autour` pendant
tout le trajet du vaisseau — la cible n'est pas un point fixe. ⚠️ **Confirmé
requis dès le premier jet** (§4, décision 4) — voir §5 pour pourquoi c'est
plus abordable qu'il n'y paraît ici : la cible est déterministe, pas
incertaine.

### 3.5 Sauvegarde

Le Lot 5 de l'assembleur sauvegarde un **design** (dix emplacements, recette
de pièces). Rien ne sauvegarde un **état en vol** (position, vitesse, ergols
restants, destination visée). Ce sont deux notions différentes qui vont
devoir cohabiter. Allégé par la décision 3 (§4) — pas de route permanente à
sauvegarder dans un premier temps — mais le cas d'un trajet ponctuel en vol
au moment de sauvegarder reste ouvert (§7).

### 3.6 Rendu et attitude

`EnginOrbital::draw` oriente le maillage par un simple lacet cosmétique
(`self.lacet`, rotation lente décorrélée de tout). Un vaisseau qui accélère
dans une direction voulue (pointer son delta-v, ou simplement pointer sa
destination) demande une orientation **pilotée par la trajectoire**, pas
décorative.

## 4. Décisions de gameplay (tranchées le 2026-08-14)

Sept forks, tranchés en discussion avec l'utilisateur après
[`concept_jeu.md`](concept_jeu.md). **À ne pas rouvrir sans raison** (même
consigne que les décisions actées de `STATE.md`) :

1. **Trajet automatisé, pas de plan de manœuvre.** Le joueur choisit une
   destination et confirme ; le vaisseau calcule et suit son propre chemin,
   arrivée après une durée de trajet. Pas d'écran de nœuds façon KSP.
2. **Statistiques de vol en hybride.** Chaque classe de vaisseau
   (Starship/Jupitris/ISV) porte des statistiques de base ; les pièces VAB
   choisies à l'assemblage appliquent des **modificateurs** par-dessus
   (ex. une cuve de plus prolonge la portée). Pas de simulation complète
   masse/poussée/Isp par pièce.
3. **Logistique manuelle d'abord.** Chaque trajet est reprogrammé à la main.
   Les routes d'approvisionnement répétées (assigner un vaisseau à une
   boucle A↔B permanente) sont explicitement **reportées**, pas conçues
   maintenant — décision revisitable, pas actée en détail.
4. **Rendez-vous avec cible mobile dès le premier jet**, y compris les
   habitats en orbite de géante gazeuse ou à un point de Lagrange. Pas de
   simplification « destination fixe » même temporaire. **Précisé le
   2026-08-15** (`concept_jeu.md` §5) : les cinq points (L1–L5) sont
   utilisables, et leur instabilité réelle (vraie pour L1/L2/L3) est
   délibérément **hors jeu** — tous traités comme des positions stables,
   fonctions fermées de la position de la planète hôte. Ça ne change rien
   au calcul du §5 ci-dessous : c'est toujours une cible déterministe.
5. **La décroissance des superstructures sans énergie est un minuteur
   abstrait**, pas une vraie trajectoire orbitale calculée par ce système —
   mécanique indépendante, hors périmètre des lots ci-dessous.
6. **Le saut interstellaire de l'ISV est un événement distinct**, pas le
   même calcul de trajectoire étiré à plus grande échelle — les distances
   interstellaires changent de repère de référence entièrement (le système
   de destination n'est pas positionné dans le même espace que les astres
   du système d'origine), plus proche d'une transition de vue (façon
   starmap) que d'un vol continu. Son propre petit chantier, plus tard.
7. **La colonisation consomme le vaisseau.** Un Starship qui colonise ne
   survit pas au trajet — il se déconstruit dans le nouveau site à
   l'arrivée. Un trajet de colonisation n'a donc jamais de retour, à la
   différence d'un trajet de fret.

**Quatre de plus, tranchées le 2026-08-15/16** (après le Lot B) :

8. **Un vaisseau qui ne vole pas est « posté »**, pas rangé dans un tiroir
   abstrait : il tient une vraie position, en orbite d'une planète/lune
   (même mécanique que l'ISS) ou « en vol stationnaire » près d'un habitat
   ou d'une station. **Pas de nouveau type** — `EnginOrbital` (`engin.rs`)
   couvre déjà les deux cas : `parent` est un simple index d'astre, et
   `orbiter_autour` n'a jamais supposé que ce parent soit une vraie planète.
   Un habitat qui est lui-même un `EnginOrbital` (§5 de `concept_jeu.md`)
   peut donc déjà servir de parent à un vaisseau qui « hover » près de lui.
   Départ d'un trajet = retirer le `EnginOrbital` posté, lire sa position
   courante, construire une `Trajectoire` depuis là. Arrivée = l'inverse
   (ou consommation, décision 7). Voir §5 pour ce que ça change au Lot C.
9. **Le coût de construction n'est pas plat.** Il grimpe fort avec la
   classe/taille du vaisseau — un gros vaisseau coûte **beaucoup** plus
   qu'une extrapolation linéaire de sa taille. Pour les classes
   **réutilisables** (le Starship nommément) : la partie récupérable
   (booster) applique un **pourcentage de réutilisabilité** qui réduit le
   coût d'une **reconstruction** du même vaisseau — pourcentage qui monte
   avec la recherche (le tech tree de `concept_jeu.md` §4). Mécanique
   d'**économie**, pas de trajectoire — mais elle répond en partie à « combien
   de mobiles à la fois » (§7) : le nombre de gros vaisseaux reste
   auto-limité par leur coût, les Starships bon marché peuvent proliférer.
   Détail économique dans `concept_jeu.md`, pas ici.
10. **Les stations postées ne sont pas pilotables** — tranché, pas une
    supposition. En contrepartie, elles (et les vaisseaux, postés ou en vol)
    portent une **barre de PV** : dégâts d'une éruption solaire ou d'un
    impact d'astéroïde, réparation nécessaire comme toute autre structure
    spatiale. Mécanique de **durabilité**, distincte du minuteur de
    superstructure (décision 5, qui ne concerne que l'absence d'énergie) —
    détail dans `concept_jeu.md`.
11. **Le mode `NCorps` était un test, retiré du jeu** (2026-08-16,
    `systeme/mod.rs`, `menu/`) : `ModePhysique` n'existe plus, il n'y a plus
    qu'un seul mode de propagation. Ça **retire** l'entrée découverte plus
    bas sur la divergence N-corps — elle ne s'applique plus, la prémisse
    « position future connue en forme fermée » (§5) tient désormais **sans
    condition**.

## 5. Conséquences techniques de ces décisions

- **Décision 1 + 4 combinées** résolvent une bonne partie de §3.4 sans
  passer par la version la plus coûteuse de §3.2 : puisque **toute cible
  sur rails a une position future connue en forme fermée**
  (`orbiter_autour` est déterministe — pas de bruit, pas d'incertitude), un
  rendez-vous automatisé n'a pas besoin de replanifier en vol ni
  d'intégrer une gravité à chaque frame. Il suffit de résoudre **un arc**
  (conique unique ou coniques raccordées) qui part d'ici-maintenant et
  arrive **au point que la cible occupera** à l'heure d'arrivée calculée —
  un problème de ciblage classique (façon solveur de Lambert), pas un
  problème de prédiction. Ni « arc décoratif » (perdrait le rendez-vous
  garanti) ni « intégration numérique complète » (inutile : la cible n'a
  pas besoin d'être intégrée, seulement évaluée à une date future).
- **Décision 2** allège §3.1 : pas besoin de masse sèche/poussée/Isp par
  pièce dans un premier temps, seulement un petit jeu de champs-
  modificateurs sur les pièces de propulsion/réservoir concernées (portée,
  vitesse, capacité). Une vraie simulation reste possible plus tard, mais
  n'est plus un préalable.
- **Décision 3** simplifie §3.5 (persistance) pour le premier jet : pas de
  format de « route » permanente à concevoir tout de suite, seulement l'état
  d'un trajet en cours (§7 garde la question de sa survie à une sauvegarde,
  qui reste ouverte).
- **Décisions 5 et 6** retirent deux morceaux du périmètre de ce chantier :
  ni le minuteur de superstructure, ni le saut interstellaire, n'ont besoin
  du mode de mouvement mobile du Lot B ci-dessous. Ce sont des mécaniques
  voisines, pas des cas de ce système.
- **Décision 7** fait de la colonisation un **état terminal** d'un trajet
  (le vaisseau est retiré du monde à l'arrivée, converti en site), pas un
  second système parallèle au fret — même Lot C/E, une branche de plus à la
  résolution d'arrivée.
- **Décision 8** répond à la question laissée ouverte par le journal du Lot
  B (« d'où vient le point de départ en jeu ») : c'est la position courante
  d'un `EnginOrbital` posté, lue au moment du départ — le Lot C n'a **pas**
  besoin d'inventer un nouveau concept de « parking », seulement une
  fonction de conversion `EnginOrbital` ↔ `VaisseauEnTransit` dans les deux
  sens (départ/arrivée).
- **Décision 11** simplifie rétroactivement `Astre::position_future` et
  `Systeme::position_future_de` : plus besoin de vérifier un mode avant de
  faire confiance à la prédiction, il n'y en a qu'un.

## 6. Découpage en lots (aligné sur §4)

Par analogie avec les lots de l'assembleur (`conception/assembleur.md`),
séquencé. État à jour au 2026-08-15 — voir `suivi/trajectoires.md` pour le
détail :

| Lot | Contenu | État |
|---|---|---|
| **A** | Statistiques de vol par classe de vaisseau + modificateurs minimaux sur les pièces VAB concernées (décision 2) — plus léger que l'ancien « Lot A » à simulation complète | ✅ fait (2026-08-15), `src/vaisseau/vol.rs` |
| **B** | Mode de mouvement **mobile** pour un `Astre` (à côté d'`orbiter_autour`, pas à sa place) : résout un arc vers la position future connue d'une cible sur rails (décisions 1 + 4) | ✅ **fait** (2026-08-15) — prédiction (`Astre::position_future`, `Systeme::position_future_de`), géométrie de l'arc (`systeme::trajectoire::Trajectoire`) et le mobile lui-même (`transit::VaisseauEnTransit`, `Astre::suivre_trajectoire`, câblé dans `Systeme::update`). Rien ne le construit encore en jeu — c'est le Lot C. Détail : `suivi/trajectoires.md` |
| **C** | UI de destination : choisir une cible, voir coût/durée estimés, confirmer un départ — pas de nœuds de manœuvre (décision 1). Inclut la branche colonisation (décision 7) et la conversion `EnginOrbital` → `VaisseauEnTransit` au départ (décision 8) | ✅ **fait** (2026-08-16) — bouton LANCER dans la fiche + mode armé dans `skymap.rs` (`Systeme::lancer`, `Astre::tenter_lancer`), un Starship posté réellement en jeu (`genese/presets.rs`). Pas d'aperçu de coût/durée avant de cliquer une destination (Lot D). Détail : `suivi/trajectoires.md` |
| **D** | Rendu de la trajectoire prévue (réemploi du semis de points existant, adapté à un arc ouvert) + caméra qui peut suivre un mobile | ✅ **fait** (2026-08-16) — `Trajectoire::polyligne` + `dessiner_points_ouvert` (variante sans bouclage de `dessiner_points`), aperçu ambre affiché tant qu'un vaisseau est armé et qu'une cible valide est survolée ; caméra : `set_focus` après un lancement réussi, `Camera::cible` suivait déjà n'importe quel astre mobile sans changement. Détail : `suivi/trajectoires.md` |
| **E** | Amarrage/capture à l'arrivée sur une cible mobile — la résolution du rendez-vous de la décision 4, jusqu'au bout | à faire |

Hors périmètre de ces lots, par décision (§4) : le minuteur de
décroissance des superstructures (5), le saut interstellaire (6), les
routes d'approvisionnement répétées (3).

## 7. Ce qui reste vraiment ouvert

Trois points de la première liste (2026-08-14) sont désormais résolus par
les décisions 8–11 (§4) et retirés d'ici :
- ~~Combien de mobiles à la fois~~ — pas de plafond technique dur ; la
  décision 9 (coût non plat + réutilisabilité) l'auto-limite côté économie.
- ~~Ce qui est pilotable~~ — décision 10 : les stations postées ne le sont
  **pas**, seuls les vaisseaux le sont.
- ~~Le mode N-corps~~ — décision 11 : retiré du jeu, la question ne se pose
  plus.

Ce qui reste vraiment ouvert :

- **Persistance d'un trajet en cours** à travers une sauvegarde/relance
  (position, vitesse, ergols, destination) — décision 3 en repousse le
  besoin (pas de routes permanentes), mais un trajet ponctuel en vol au
  moment de sauvegarder reste un cas à trancher. Voir `suivi/trajectoires.md`
  pour l'explication détaillée de ce que ça implique concrètement.
- **La méthode de résolution exacte de l'arc** (§5) : quel solveur pour «
  partir d'ici, arriver là à telle date » — déjà tranché en pratique pour le
  Lot B (Bézier quadratique décoratif, `systeme::trajectoire::Trajectoire`),
  mais la question de passer un jour à une vraie conique reste ouverte si le
  besoin s'en fait sentir.
- **Les chiffres exacts** (durée du minuteur de superstructure, magnitude
  des modificateurs de la décision 2, capacités de fret, pourcentage de
  réutilisabilité de la décision 9, seuils de PV/dégâts de la décision 10)
  — consolidés dans `suivi/trajectoires.md` § « Placeholders », un seul
  endroit pour tous les retrouver au moment de les régler.
- **Le détail des dégâts** (décision 10) : quels événements en infligent
  (éruption solaire, astéroïde — d'autres ?), à quel rythme, ce que
  « réparer » demande concrètement (ressources ? un vaisseau dédié ?
  du temps ?). Décision 10 dit qu'il y a des PV et une réparation, pas
  comment ils fonctionnent.

## 8. Comment reprendre

**Lots A, B, C et D faits** (2026-08-15/16) — voir `suivi/trajectoires.md`
pour le détail au jour le jour. §4 est acté (onze décisions). Un Starship
posté en orbite terrestre peut désormais être lancé vers une destination
choisie en jeu (bouton LANCER, clic de destination), voler pour de vrai,
avec un aperçu ambre du trajet pendant qu'on survole une cible à armement
engagé et une caméra qui le suit une fois parti. **Suite : Lot E** — un
vaisseau arrivé reste immobile pour toujours, rien ne décide encore de son
sort (amarrage, consommation à la colonisation).
