# STATE — où en est le chantier

> Point de reprise, **2026-08-01**. À lire en premier dans une nouvelle session.
> Détail complet : [`docs/conception/assembleur.md`](docs/conception/assembleur.md)
> (le quoi et le pourquoi) et [`docs/suivi/assembleur.md`](docs/suivi/assembleur.md)
> (le journal, étape par étape).

## ⚠️ Rien n'est commité

Dernier commit : `32300e9` « Vaisseau : ISV complet avec VFX ».

**Tout ce qui suit est dans l'arbre de travail, non commité** — c'est une
consigne debout de l'utilisateur : *« no commit i'll say when commit is
needed »*. Ne pas commiter sans qu'il le demande.

État vérifié : **255 tests verts** (`cargo test --release`, ~0,9 s),
**54 avertissements**, build release propre.

## Chantier en cours : l'assembleur de véhicules (vue VAB)

L'ISV est **clos** (asset DONE, `docs/suivi/stations.md` §C.29). Le chantier
courant est l'éditeur d'assemblage interactif.

### Fait — Lot 1 (assainir les fondations)

| | | Où |
|---|---|---|
| L1.1 | Catalogue de briques : 4 sources → 1 table, capacités déduites | `src/ecran/catalogue.rs` |
| L1.2 | Balayage des 31 variantes ; la couverture est une **erreur de compilation** | `composant/mod.rs` (chaîne `suivante`) |
| L1.3 | `generer_est_deterministe` réécrit sur la géométrie (l'ancien était vide de sens) | `generateur.rs` |
| L1.4 | Rayons déclarés : défaut `Charpente` corrigé, dette relevée | `composant/mod.rs` |
| L1.5 | `complexite_influe…` : `>=` → `>` | `generateur.rs` |
| L1.6 | Collision en **capsules** au lieu de sphères | `src/vaisseau/enveloppe.rs` |
| L1.7 | Overlay : enveloppes (**E**) et fils numérotés (**F**) | `ecran/enveloppes.rs`, `ecran/fils.rs` |
| L1.8 | Mesureur par tranchage + catalogue généré | `vaisseau/mesure.rs`, `vaisseau/inventaire.rs` |

Sorties utiles : **`docs/reference/fils.md`** — 31 composants, 1080 fils
numérotés, profil et serrage par pièce. Régénérer avec
`FILS=1 cargo test --release le_catalogue_des_fils`.

### Fait — §9 « le boudin »

Enveloppe **rectangle** (`Noyau::Rectangle`, dans `src/vaisseau/enveloppe.rs`),
pour les pièces plates. `Enveloppe` porte maintenant un `Noyau` explicite
(`Segment` ou `Rectangle`) au lieu d'un couple `(a, b)` nu ; `ecart`/`profondeur`
ne changent pas de forme. Nouveau : `distance_point_rectangle` (trivial),
`distance_segment_rectangle` (exact, méthode des bornes comme
`distance_segments`), `distance_rectangles` (exact, orientation libre —
huit balayages bord-contre-rectangle-entier, voir le journal pour pourquoi
l'énumération bord↔bord naïve ne suffit pas). Appliqué aux boucliers de l'ISV
(`bouclier.rs`) : le gain qui motivait le chantier, ~×10 sur l'épaisseur
réservée par l'empilement à quatre plaques sur le mât. Détail complet, y
compris le red-check (5 sabotages) : `docs/suivi/assembleur.md` §9.

### Fait — L2.1 identifiants stables de ports libres

`Chantier::poser`/`peut_poser` prennent maintenant un `PortLibre::id: u64`
(compteur monotone, jamais recyclé) au lieu d'une position brute dans
`libres()` — le `swap_remove` interne continue de décaler les positions, mais
ça ne fuit plus vers les appelants. `compatibles` rend des `id`, pas des
positions. 30 sites appelants corrigés (`generateur.rs`, `montage.rs`,
`composant/mod.rs`), tous trouvés par le compilateur après le changement de
signature — pas par grep. Les trois tests demandés par la conception (§6.1) :
un `id` survit à une pose ailleurs, ne se recycle jamais, et poser sur un `id`
périmé échoue proprement (pas de panique, aucun effet de bord). Deux ont raté
leur premier red-check (le test inspectait la structure au lieu d'emprunter
le vrai chemin de code, ou fixait une variable du scénario au lieu de sonder
l'état réel) et ont été renforcés — détail dans le journal,
`docs/suivi/assembleur.md` §L2.1. `peut_poser` a changé de signature mais n'a
toujours aucun test dédié (dette préexistante, notée en L1.6) : à reprendre
quand L2.4 lui donnera un vrai consommateur.

### Fait — L2.2 `retirer` et la propriété d'aller-retour

`Chantier::retirer(id)` retire une pièce **et son sous-arbre**, restitue le
port hôte qu'elle consommait (sous un `id` neuf — L2.1 oblige), et rembourse
le budget (`Budget::rembourser`, nouveau, symétrique de `depenser`). A forcé
la même correction qu'en L2.1, mais côté pièces cette fois :
`PortLibre::origine` était un indice brut dans `pieces`, qui ne survivrait
pas à un retrait au milieu de la séquence (`Vec::retain` déplace les pièces
suivantes) — devenu un `id` stable, chaque pièce interne portant désormais
`{ id, piece, hote: Option<PortLibre> }` (`hote` = le port exact qu'elle a
consommé, restitué tel quel à `retirer`). ~15 sites dans `generateur.rs`
retypés en conséquence ; un (`compte()`, qui balayait `0..nb_pieces()` en
supposant l'absence de trous) corrigé pour de bon plutôt que retypé, via un
nouvel itérateur `Chantier::pieces()`. Les quatre tests (les trois demandés
par la conception + la propriété d'aller-retour qui les résume) sont passés
au vert **du premier coup**, à l'inverse de L2.1 — attribué à la même
discipline qui avait manqué la fois précédente : passer par l'API réelle,
sonder l'état plutôt que d'en fixer une partie. Le red-check (6 sabotages) a
quand même trouvé un défaut qu'aucun des trois tests nommés n'aurait vu —
seule la propriété d'aller-retour l'a pris, exactement comme §6.2 le
prédisait. Détail : `docs/suivi/assembleur.md` §L2.2.

### Fait — L2.3 annuler/refaire

`Chantier::annuler`/`refaire`, en **instantané complet** plutôt qu'en
inversion opération par opération : `retirer` peut faire disparaître un
sous-arbre entier, et le reposer ne recréerait ni les mêmes pièces ni la
même arborescence. Un `Instantane` (même forme que l'état mutable de
`Chantier`) est capturé juste avant chaque mutation et restitué tel quel ;
deux piles (annuler/refaire), la seconde vidée par toute nouvelle opération
réussie. **Opt-in** (`Chantier::avec_historique()`) — le générateur pose des
centaines de fois par station et n'annule jamais rien, il ne doit pas payer
le clone d'état à chaque pose.

Le test de la propriété (§6.3 : *n* opérations puis *n* annulations = état
initial, *n* refaits = état d'après) est passé au vert **du premier coup** —
et à tort : un red-check sur le point de capture de `retirer` (l'instantané
pris après ses mutations au lieu d'avant) n'a rien fait rougir, sur 30
graines. Cause structurelle : `restaurer` **écrase** l'état d'un bloc, donc
après *n* annulations le résultat final ne dépend que du **premier**
`avant` empilé (toujours une pose, jamais un `retirer`, qui ne peut pas être
la toute première opération) — un instantané corrompu au milieu de la pile
est invisible à un test qui ne compare que les deux bouts de la séquence.
Corrigé en comparant l'état à chaque étape intermédiaire, pas seulement au
départ et à l'arrivée ; sous cette version, le même sabotage rougit
immédiatement. Détail : `docs/suivi/assembleur.md` §L2.3.

### Fait — L2.4 la palette

`posables(genre, profil) -> Vec<(Composant, usize)>` (`composant/mod.rs`) :
la duale de `Chantier::compatibles` — part d'un port, rend les composants
posables dessus, avec un indice de montage valide pour chacun. Réutilise
l'échantillon de L1.2 (`suivante`/`echantillons`, promu hors de
`#[cfg(test)]` — trois consommateurs maintenant : balayage de couverture,
`fils.md`, palette) plutôt que d'écrire une seconde liste de 31 variantes.
`Categorie` (10 valeurs : Structure, Habitat, Energie, Communication, Cargo,
Propulsion, Bouclier, Composite, PoseeAMain, Effet) reprend le découpage déjà
présent dans les fichiers de famille (`treillis.rs`, `propulsion.rs`…),
plutôt qu'une taxonomie neuve. Trouvé au passage, hors sujet : `GenrePort::
PoutreBout` n'est posé sur aucun composant du jeu actuel — variante morte,
non touchée ici.

Le test d'accord palette/`compatibles` a raté deux fois avant de tenir — pas
un défaut de `posables`, un défaut du test : il comparait à l'indice de
montage exact, alors que `ModuleAxial` a deux écoutilles symétriques et
`posables` n'en retient qu'une (ce que §6.5 demande réellement : « toute
variante… posable », pas tous ses indices). Resserré à la bonne granularité
— la variante, pas l'indice — le test est resté vert sur le comportement
correct et a immédiatement rougi sous 2 sabotages réels (égalité stricte au
lieu de `.compatible()`, révélant que `ModuleAxial`/`ModuleRadial` sont
mutuellement compatibles ; palette qui propose tout sans regarder le port).
Détail : `docs/suivi/assembleur.md` §L2.4.

### Fait — L2.5 sérialisation et aller-retour géométrique

**Dernière étape du Lot 2** (§6.4) — clos, détail complet dans
`docs/suivi/assembleur.md` §L2.5. `Chantier::recette()`/`depuis_recette()`,
format `Etape { composant, hote: Option<(usize, usize)>, montage }` où un
hôte s'adresse par `(indice de séquence, indice local de port)` — pas par un
`id` de session (L2.1), qui ne survivrait pas à la fermeture du jeu. Nouveau
champ `PortLibre::indice: usize` pour porter cet indice local. Décision
tranchée (répondant à la question ouverte laissée en L2.4) : `SousEnsemble`
sérialise ses **pièces déjà cuites**, pas sa recette — performance (il est
fait pour être réutilisé, rejouer sa recette à chaque désérialisation referait
un travail de collision déjà validé) et évolutivité (une recette rejouée est
vulnérable aux retouches futures de géométrie/collision, un vrai risque vu
l'historique du projet ; des pièces cuites sont immunisées). `glam` ajouté en
dépendance directe pour sa feature `serde` (déjà tiré par macroquad, Cargo
unifie), `serde` gagne la feature `rc` (`Rc<DonneesSousEnsemble>`).

Le red-check a reproduit **exactement** le défaut de L2.1 : sabotage de
`depuis_recette` (port hôte retrouvé en ignorant l'indice local, premier port
libre de l'hôte pris) raté par 2 tests sur 3, tous deux construits sur
`ch.libres()[0].id` — coïncidant avec ce que le sabotage rend aussi. Durci en
visant délibérément un port qui n'est pas le premier ; les trois rougissent
ensuite. Détail : `docs/suivi/assembleur.md` §L2.5.

**Lot 2 clos** (L2.1 → L2.5). Le modèle de `Chantier` interactif est complet ;
aucun écran ne le consomme encore — dette relevée à chaque étape, pas un
défaut. Suite éventuelle (câbler un écran d'assemblage dessus) hors du
périmètre défini par §7, à discuter avec l'utilisateur.

### À faire — Lot 3, compléter le modèle pour l'écran

L'écran d'assemblage **n'existe pas** et n'est pas au menu. §8 le spécifie en
entier mais n'était rattaché à aucun lot ; le découpage a été arrêté le
2026-08-01 (`docs/conception/assembleur.md` §7.1). Trois manques relevés en
confrontant §8 au `Chantier` livré — additifs, en lecture seule (pas le
risque de rétrofit qui avait imposé L2.1 en tête), et **testables sans vue**,
donc red-checkables comme tout le Lot 2 :

1. ~~**L3.1 `pose_prevue`**~~ ✅ — rend la `Piece` exacte qu'une pose
   produirait. Fait mieux que publier le calcul : `corps_prevu` a été extrait
   et est désormais appelé par **les trois** méthodes qui posent la même
   question (`poser`, `peut_poser`, `pose_prevue`) — plus rien à tenir
   d'accord. Contrat volontaire : répond la **géométrie**, pas l'acceptation
   (§8.5 a besoin du fantôme d'une pose *refusée*). Détail et red-check :
   `docs/suivi/assembleur.md` §L3.1.
2. **L3.2 `sous_arbre(id)`** — surlignage de la sélection. `retirer` le
   calcule déjà en une passe, sans l'exposer. À extraire, pas à réécrire.
3. **L3.3 désignation** — port et pièce sous le curseur. Rien n'existe :
   `Camera::pick` ne vise que les astres. Le port se traite en espace écran
   (`PortLibre::repere` est déjà en monde) ; la pièce est un rayon contre une
   `Enveloppe`, à placer dans `enveloppe.rs` près des fonctions de distance.

Ensuite : **Lot 4** l'écran (§8.2–8.4), découpage à arrêter à la fin du
Lot 3 — choix explicite de l'utilisateur, contre un découpage figé d'avance.
**Lot 5** ce que seul l'écran permet : overlay §8.5, sauvegarde disque,
arbitrage L1.4, composites (`figer`).

⚠️ Le Lot 4 sera le **premier lot majoritairement non testé** (§6.6 : pas de
test de rendu). La discipline change de forme : pousser hors du code de
dessin tout ce qui se décide, vers des requêtes de modèle red-checkables. Ce
qui reste non testé doit être seulement *où le rectangle se pose*.

Dette de L1.4 restée ouverte : ~19 variantes sous-déclarent leur `rayon_local`
(cadrage caméra ; mesuré : 2 vues sur 35 bougeraient, de 1 % et 2 %).
`MARGE_RAYON = 1.40` marque la dette dans le code. **Elle se solde en Lot 5** :
§8.5 dit que la serre d'une enveloppe ne se juge qu'à l'œil, et personne n'a
encore vu une seule enveloppe.

Dette de L1.6 toujours ouverte : `peut_poser` n'a aucun test dédié, reporté
trois fois faute de consommateur réel (L2.4 est passée par `posables`). Son
vrai consommateur est le code couleur de §8.4 — donc Lot 4.

## Décisions prises (ne pas les rouvrir sans raison)

- **Pièce d'abord** (façon KSP) : palette permanente à gauche, ports compatibles
  qui s'allument. Pas « port d'abord ».
- **Bac à sable libre** : pas de plafond de coût.
- **Capsules** plutôt que sphères pour la collision — et **pas** d'AABB/OBB.
- **Symétrie non recherchée partout** : la grammaire pose ce qui tient, les
  groupes ne sont pas atomiques.

## Questions ouvertes

*(vide)*

## Méthode (ce qui a marché, à ne pas perdre)

- **Red-check systématique** : tout test ajouté est cassé volontairement sur sa
  cible pour vérifier qu'il rougit. 17 sabotages sur le Lot 1, 5 sur §9, 5 sur
  L2.1, 6 sur L2.2, 5 sur L2.3, 3 sur L2.4, 4 sur L2.5, tous rougis au final.
  C'est le seul filet qui ait trouvé de vrais défauts — sur §9 il a fallu
  **muscler un test existant** (un cas adversarial de plus) ; sur L2.1, **deux
  tests sur trois ont raté leur premier essai** : l'un inspectait la structure
  de données après coup au lieu d'emprunter le vrai chemin de code (`poser`),
  l'autre fixait une variable du scénario au lieu de sonder l'état réel avant
  d'agir. Sur L2.2, les quatre tests sont passés au premier coup — la
  discipline se transmet d'une étape à l'autre quand elle est écrite, pas
  seulement retenue — et la propriété d'aller-retour a quand même trouvé,
  seule, un défaut qu'aucun des trois tests nommés n'aurait vu (ports
  orphelins d'un sous-arbre retiré). Sur L2.3, **nouvelle forme du même
  problème** : la propriété round-trip elle-même ne comparait que les deux
  bouts de la séquence, et `restaurer` écrasant l'état d'un bloc plutôt que
  d'appliquer une différence, un instantané corrompu au milieu de la pile
  devenait invisible — seul le tout premier `avant` empilé pèse sur le
  résultat final. Sur L2.4, **le red-check a démasqué un test trop strict**
  plutôt qu'un défaut de production : comparer à l'indice de montage exact au
  lieu de la variante posable faisait rougir le test sous un code *correct*
  (`ModuleAxial` a deux écoutilles symétriques, `posables` n'en garde qu'une)
  — un rappel que « le test rougit » ne veut pas toujours dire « le code a
  tort », et qu'il faut vérifier lequel des deux avant de corriger. Sur L2.5,
  **retour exact du défaut de L2.1** : deux tests sur trois construits sur
  `ch.libres()[0]` n'ont pas vu un sabotage qui, lui aussi, retombe sur « le
  premier port venu » — la même parade (viser délibérément un choix non
  trivial dans le scénario) a suffi, une fois de plus.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent d'une
  valeur qui en a deux (`docs/suivi/stations.md` §C.29). Sur §9 : l'épaisseur du
  boudin dérive des mêmes constantes que le dessin (`MOYEU_DEMI`…), pas d'un
  recalcul à la main — et ça a évité de reproduire le défaut de L1.4.
- **Mesurer avant d'affirmer.** Deux corrections du mesureur ont été annulées
  parce qu'elles contredisaient un test existant sur la même grandeur.
- **Mettre à jour les `.md`** à chaque étape — consigne debout de l'utilisateur.

## Dettes préexistantes, sans rapport

- `cargo clippy` ne compile pas (`src/planete/terrain.rs:154`).
- Avertissements `dead_code` de fond sur le binaire : les méthodes du noyau
  rectangle (L1.6), `Chantier::retirer`/`annuler`/`refaire`/`avec_historique`/
  `peut_poser`/`compatibles`/`figer`/`recette`/`depuis_recette`, et
  `Categorie`/`categorie`/`posables` ne servent pour l'instant qu'aux tests,
  en attendant un écran qui consomme le modèle (hors scope du Lot 2 tel que
  défini par §7/§6.6).
- `GenrePort::PoutreBout` n'est posé sur aucun composant actuel (trouvé en
  L2.4, sans rapport) — variante d'enum morte.
