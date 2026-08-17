# Conception — Concept du jeu : SpaceRust

> Document de conception, **2026-08-14**. Capture la vision de jeu telle que
> décrite par l'utilisateur en discussion — avant tout chantier détaillé par
> système. C'est un **chapeau** au-dessus des autres `docs/conception/*.md`
> (chacun détaille un système ; celui-ci dit pourquoi ces systèmes existent
> et comment ils s'articulent). Rien ici n'est codé. À affiner au fil des
> chantiers, pas figé.
>
> Écrit **juste avant** [`trajectoires.md`](trajectoires.md), et pas par
> hasard : le principe directeur du §2 ci-dessous (l'isolement des
> ressources) est la raison de fond pour laquelle le déplacement des
> vaisseaux **doit** exister — sans lui, rien ne peut être acheminé nulle
> part.

---

## 1. Le pitch

Un **RTS à travers l'espace** : développer sa colonie et sa résilience, en
progressant de l'échelle **stellaire** à l'échelle **interstellaire**.

## 2. Principe directeur : l'isolement des ressources

**Chaque site producteur de ressources est isolé.** Il doit soit recevoir
des ressources apportées depuis ailleurs, soit être **autonome**. C'est le
principe qui structure tout le reste : le jeu n'a de sens que si des
ressources doivent voyager entre des sites qui ne se suffisent pas à
eux-mêmes — d'où l'importance du chantier trajectoires.

## 3. Production : le plateau de tuiles

Chaque **planète**, **station** et **habitat** génère ses ressources sur un
plateau de tuiles qui lui est propre. La **taille** des tuiles et leur
**regroupement** (« clustering ») entre elles créent des bonus de
performance — les accommodations (pas seulement la production brute) passent
par le même plateau.

⚠️ Pas encore conçu en détail : types de tuiles, formule des bonus de
regroupement, règles de placement. Rattaché au chantier économie (D-INT-2,
`STATE.md`), volontairement reporté jusqu'à la fin de la vue système.

## 4. La recherche : un troisième type d'objet

Les objets de recherche ne se comportent **pas** comme des stations
ordinaires. Progression en trois temps :

1. Une **sonde** découvre d'autres planètes en premier.
2. Une **station de recherche** s'améliore ensuite, **tier par tier**.
3. Le **tier 3** débloque la **colonisation** d'une planète ou d'une lune.

Ces objets produisent de la **recherche** comme ressource propre. Un **arbre
technologique** complet reste à créer — rien de tel n'existe dans le code
aujourd'hui. C'est une extension du chantier économie (D-INT-2), qui ne
portait jusqu'ici que sur production/consommation/coûts ; la recherche et la
colonisation en tier 3 s'y rattachent maintenant explicitement.

**Précisé le 2026-08-15** : la recherche débloque aussi le **pourcentage de
réutilisabilité** des vaisseaux réutilisables (§8, décision 9 de
[`trajectoires.md`](trajectoires.md)) — un second exemple concret de ce que
l'arbre technologique fait bouger, à côté du tier 3 de colonisation.

## 5. Habitats spatiaux : torus, Elysium et au-delà

Les habitats (tore de Stanford, complexe Elysium, et ceux à venir) se
placent à un **point de Lagrange**, ou en **orbite d'une géante gazeuse**.
**Une fois complétés, ils se comportent comme une planète ou une lune** —
un site de plein droit, pas un simple objet en orbite.

**Précisé le 2026-08-15** : les **cinq** points de Lagrange sont utilisables
(L1/L2/L3 colinéaires **et** L4/L5 triangulaires), pour la variété de
distance et le réalisme du placement. **L'instabilité réelle de L1/L2/L3
n'entre pas en jeu** — décision explicite, pas un oubli : tous les points
sont traités comme des positions **stables**, sans mécanique de maintien en
poste. Techniquement, les cinq restent des fonctions **fermées** de la
position de la planète hôte (L1/L2 le long de l'axe étoile-planète, L3 à
l'opposé de l'étoile, L4/L5 à ±60° de phase orbitale) — donc calculables
exactement comme une orbite `orbiter_autour` ordinaire, sans intégration
numérique. Voir [`trajectoires.md`](trajectoires.md) §5 : c'est cette même
propriété (position future connue en forme fermée) qui rend le rendez-vous
automatisé possible, et elle tient aussi pour un habitat en Lagrange.

C'est aussi **là** que se construisent les **méga-vaisseaux** : l'ISV
(fait), le futur **Jupitris**, et un **vaisseau cargo sans nom** (les deux
derniers restent à concevoir).

⚠️ Écart avec le modèle actuel : un habitat est aujourd'hui un
`Categorie::Engin` comme l'ISS (`src/engin.rs`) — un objet en orbite
analytique autour d'un parent, pas un site producteur au même titre qu'une
planète. Le faire agir comme une planète une fois « complété » est un
changement de modèle, pas un réglage.

## 6. Superstructures

Se construisent **près** des habitats spatiaux. Elles produisent de
l'énergie, de la nourriture ou des minerais, ou collectent du gaz sur une
géante gazeuse (hydrogène). Leur point commun : elles dépendent de
l'énergie pour rester en place — mais **pas toutes de la même façon**.
Trois comportements distincts en cas de manque d'énergie :

| Comportement | Exemples | Ce qui se passe sans énergie |
|---|---|---|
| **Mise à l'arrêt** | usine en microgravité, fonderie | Production **stoppée**, reprend dès le retour du courant — rien n'est perdu |
| **Destruction** | station de collecte de gaz, filet à antimatière | Pas d'énergie → pas de maintien en position → **chute** dans la géante gazeuse ou le trou noir qu'elle exploite. **Un mois de jeu** sans énergie suffit à la détruire |
| **Aucun risque** | réflecteur de Dyson, concentrateur de Dyson | Insensibles à une coupure d'énergie |

⚠️ Les **trous noirs** n'existent pas dans le modèle d'astres actuel
(`Categorie` : `Etoile`, `Planete`, `Lune`, `Asteroide`, `Comete`, `Engin` —
`src/astre.rs`) : à ajouter le jour où le filet à antimatière sera conçu.

## 6 bis. Durabilité : dégâts et réparation (précisé le 2026-08-15)

**Les stations postées ne sont pas pilotables** — tranché
([`trajectoires.md`](trajectoires.md) §4 décision 10), pas une supposition
laissée en l'air. En contrepartie, elles portent une **barre de PV**, comme
les vaisseaux (postés ou en vol) : une éruption solaire ou un impact
d'astéroïde inflige des dégâts, qui demandent une **réparation** — même
logique de durabilité pour toute structure spatiale, distincte du minuteur
d'énergie des superstructures (§6 ci-dessus, qui ne réagit qu'à l'absence de
courant, pas à un choc extérieur).

⚠️ Pas encore conçu en détail : quels événements infligent des dégâts
au-delà des deux cités (éruption, astéroïde), à quel rythme, ce qu'une
réparation consomme concrètement (ressources ? un vaisseau ou une pièce
dédiée ? du temps seul ?), et si la barre de PV a un lien avec le plateau de
tuiles (§3) ou en est indépendante.

## 7. Vaisseaux

| Classe | Rôle | État |
|---|---|---|
| **Starship** | Petit — transport de faible quantité de ressources, **ou** colonisation. Le rôle est **figé à la construction**, pas reconverti ensuite. **Réutilisable** (§ ci-dessous) | fait (voir `suivi/stations.md` §F.10) ; **vole pour de vrai en jeu** depuis le 2026-08-16 (`trajectoires.md` Lot C) — un exemplaire posté en orbite terrestre, lançable par un bouton LANCER, avec un **aperçu du trajet** pendant l'armement et une **caméra qui le suit** une fois parti (Lot D) |
| **Jupitris** | Plus gros, à base d'hydrogène | à concevoir |
| *(cargo sans nom)* | Référence de capacité de fret maximale du jeu | à concevoir, y compris le nom |
| **ISV** | Late-game : **premier** vaisseau capable de rallier un autre système stellaire (saut interstellaire) | fait en tant qu'objet (`suivi/stations.md` §C.29), aucun mécanisme de vol |

**Précisé le 2026-08-15 — coût de construction et réutilisabilité**
([`trajectoires.md`](trajectoires.md) §4 décision 9) : le coût d'un
vaisseau **n'est pas plat**, il grimpe fort avec la classe/taille — un gros
vaisseau coûte largement plus qu'une simple extrapolation de sa taille. Pour
les classes **réutilisables** (le Starship, nommément), la partie
récupérable (booster) applique un **pourcentage de réutilisabilité** qui
réduit le coût d'une **reconstruction** du même vaisseau, pourcentage qui
augmente avec la recherche (§4 ci-dessus). Effet de bord assumé : ça
auto-limite combien de gros vaisseaux existent à la fois (chers, donc rares)
sans plafond technique dur, pendant que les Starships bon marché peuvent
proliférer. ⚠️ Pas encore conçu : le barème de coût par classe, la valeur de
départ et la courbe de progression du pourcentage de réutilisabilité.

## 8. Ce que ça change pour les chantiers déjà ouverts

- **[`trajectoires.md`](trajectoires.md)** : la session de conception du
  2026-08-14 a tranché sept des questions que ce document posait — trajet
  automatisé (pas de plan de manœuvre), statistiques de vol en hybride
  (classe + modificateurs VAB), rendez-vous avec cible mobile dès le
  premier jet, colonisation qui consomme le vaisseau (une branche d'arrivée
  parmi d'autres, pas un second système), et deux mécaniques sorties du
  périmètre (le minuteur des superstructures, le saut interstellaire de
  l'ISV — événement distinct, pas le même calcul étiré). Voir `trajectoires.md`
  §4–§7 pour le détail et ce qui reste ouvert. **Lots A, B, C et D faits**
  (2026-08-15/16) : un Starship posté en orbite terrestre peut être lancé
  vers une destination choisie en jeu (bouton LANCER) et vole pour de vrai,
  avec un aperçu du trajet pendant l'armement et une caméra qui le suit une
  fois parti. Toujours vrai : Jupitris et le cargo sans nom n'existent pas
  encore comme designs,
  donc rien à faire voler pour eux avant leur conception. **Quatre décisions
  de plus le 2026-08-15/16** (§4, décisions 8–11) : les vaisseaux non en vol
  sont « postés » via `EnginOrbital` (§6 bis y répond en partie), le coût de
  construction n'est pas plat (§7 ci-dessus), les stations postées ne sont
  pas pilotables mais ont des PV (§6 bis), et le mode `NCorps` a été retiré
  du jeu comme code de test.
- **D-INT-2 (l'économie, `STATE.md`)** : n'est plus juste « production,
  consommation, coûts, recherche » en l'air — a maintenant une forme
  (plateau de tuiles, isolement des ressources, tiers de recherche menant à
  la colonisation, coût de construction non plat + réutilisabilité).
- **`src/engin.rs`** : le modèle « habitat = objet en orbite figée » devra
  s'étendre pour qu'un habitat complété agisse comme un site de plein droit
  (§5) — pas un simple réglage de plus sur `EnginOrbital`.

## 9. Explicitement pas conçu — à ne pas déduire par défaut

- Le détail du plateau de tuiles (types, bonus de regroupement, placement).
- Le contenu de l'arbre technologique et son graphe de prérequis.
- Les chiffres : durée exacte avant destruction d'une superstructure sans
  énergie (« un mois » est la donnée de départ, pas forcément la valeur
  finale — et c'est désormais un minuteur abstrait, pas une vraie
  trajectoire, voir `trajectoires.md` §4 décision 5), tiers de recherche,
  capacités de fret.
- Le mécanisme de colonisation **au-delà** de la consommation du vaisseau
  (`trajectoires.md` §4 décision 7) : ce qui se passe concrètement côté
  site — création du plateau de tuiles, état initial de la nouvelle colonie.
- Jupitris et le cargo sans nom, en tant que designs de vaisseau.
- Les trous noirs comme catégorie d'astre.
- Le détail des dégâts/réparation (§6 bis) : quels événements, à quel
  rythme, ce qu'une réparation consomme.
- Le barème de coût par classe de vaisseau et la courbe du pourcentage de
  réutilisabilité (§7).
