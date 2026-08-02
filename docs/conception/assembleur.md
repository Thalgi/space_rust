# Conception — Assembleur de véhicules (vue VAB)

> Document de conception, **2026-07-31**. Écrit *avant* de coder quoi que ce
> soit de l'assembleur, et à partir du chantier ISV qui vient d'être clos
> ([`suivi/stations.md`](../suivi/stations.md) §C.16–C.29).
>
> Il fait deux choses, dans cet ordre :
> 1. **auditer le parc de tests existant** — lesquels portent leur poids,
>    lesquels non, et pourquoi (§1–§4) ;
> 2. en déduire **ce qui manque**, d'abord pour le code actuel (§5), puis pour
>    une vue d'assemblage interactive (§6).
>
> Il prolonge [`stations.md`](stations.md) **Partie E.4**, qui listait déjà les
> *fonctionnalités* que l'éditeur demanderait (retrait, undo, palette,
> sérialisation). Ce qui suit dit la même chose du point de vue des
> **garanties** : quoi doit être vrai, et comment on le saura.

---

## 1. État mesuré du parc de tests

`cargo test --release` : **186 tests, tous verts, en 0,07 s**. Le coût
d'exécution est donc **nul** — aucun arbitrage à faire sur la durée, et
« supprimer un test pour aller plus vite » n'est jamais un argument valable
dans ce projet. Le seul coût d'un test ici est le coût de **le lire et de le
maintenir quand il ment**.

| Fichier | Tests | Ce qu'ils gardent |
|---|---:|---|
| `vaisseau/composant/mod.rs` | 72 | ports, coût, rayon, et **géométrie** des 31 variantes |
| `vaisseau/generateur.rs` | 23 | grammaire du générateur + **le montage de l'ISV** |
| `vaisseau/assemblage.rs` | 20 | `Station`/`Piece`/`Budget` : cycle de vie et bornes |
| `vaisseau/port.rs` | 13 | **algèbre d'accouplement** (`accoupler`, compatibilité) |
| `vaisseau/symetrie.rs` | 12 | copies radiales et miroirs |
| `planete/terrain.rs` | 11 | *(hors périmètre — planètes)* |
| `vaisseau/maillage.rs` | 10 | cuisson, lotissement, transformées |
| `vaisseau/unites.rs` | 9 | `Profil`, proportions |
| `vaisseau/chantier.rs` | 9 | pose incrémentale, ports libres, collision |
| `vaisseau/montage.rs` | 5 | chaînage, symétrie appliquée |
| `ui.rs` | 2 | *(hors périmètre)* |

**Ce qui n'a aucun test du tout** — et ce n'est pas un oubli mineur :

| Module | Lignes | Testé |
|---|---:|---|
| `ecran/` (toutes les vues) | ~2 500 | **0** |
| `vaisseau/pieces.rs` (primitives de géométrie) | 860 | **0** *(indirectement)* |
| `vaisseau/peintre.rs` (le trait à deux sorties) | 189 | **0** *(indirectement)* |

`pieces.rs` et `peintre.rs` sont couverts *par ricochet* : tout test de
composant les traverse. C'est acceptable. `ecran/` ne l'est pas, et §5.1 dit
pourquoi c'est précisément le trou qui va faire mal.

---

## 2. Taxonomie : quatre familles, pas une

Le parc n'est pas homogène. Quatre familles s'y distinguent nettement, et
elles n'ont **ni la même valeur, ni la même durée de vie**.

### A — Tests de contrat (≈ 69)

`assemblage.rs`, `chantier.rs`, `port.rs`, `symetrie.rs`, `maillage.rs`,
`montage.rs`. Ils gardent une **algèbre** : accoupler deux repères, composer
des transformées, propager un budget, découper un maillage en lots.

Propriétés : entrée/sortie pures, aucune dépendance à l'œil, **vrais pour
toujours**. Ce sont les seuls tests du projet qui ne dépendent d'aucun choix
esthétique — donc les seuls qui ne devront jamais être réécrits.

> **C'est cette famille qui portera l'assembleur.** Un éditeur interactif est
> exactement une machine à appeler `accoupler`/`poser`/`compatible` des
> milliers de fois sur des séquences qu'aucun preset n'a jamais produites.

### B — Tests de décision (≈ 49)

`composant/mod.rs` (les briques ISV) et `generateur.rs` (le montage). Ils
gardent un **choix validé à l'écran**, que rien dans le code ne rend évident.

Exemples représentatifs :

- `le_collier_dequipage_enveloppe_lepine_sans_jour` — deux bornes qui se
  contredisent si on relâche l'une ;
- `la_propulsion_touche_le_pied_de_lepine` — **a changé de sens deux fois**,
  délibérément, et le commentaire le dit ;
- `la_charge_utile_suit_le_gabarit_de_lepine` — boucle sur les deux variantes
  d'épine, parce qu'un écart de 3,2 % replante la charge utile sans qu'on le
  voie ;
- `le_mat_de_tete_passe_le_plus_petit_alesage` ;
- `les_ecailles_du_bardage_se_recouvrent` ;
- `seules_les_parties_grises_du_radiateur_chauffent`.

Propriétés : ils ne sont vrais **que tant que la décision tient**. Leur valeur
n'est pas de prouver que le code est correct — elle est de **rendre l'arbitrage
opposable**. Quand un futur réglage les casse, le message dit *pourquoi* la
valeur était là.

> Ces tests-là **ne se réparent pas** quand ils rougissent : ils se relisent.
> Les meilleurs du parc portent déjà l'avertissement en commentaire (« ne pas
> corriger le sens de ces assertions sans avoir regardé la vue »).

### C — Tests d'énumération (≈ 25)

`toutes_variantes_module`, `toutes_variantes_radiateur`,
`toutes_variantes_antenne`, `toutes_variantes_un_port_montage`,
`toutes_variantes_caisson_montage_plus_faces_hotes`,
`familles_de_propulsion_partitionnent_les_variantes`, `ossature_forcee_est_
respectee_et_finie`, `toutes_graines_donnent_une_station_finie`.

Ils bouclent sur `Variante::TOUS` et vérifient une propriété faible (« un
port `Surface` », « ça ne panique pas »).

**Verdict : à garder, et à multiplier.** Ils ne prouvent presque rien sur une
variante donnée, mais ils prouvent une chose qu'aucun autre test ne peut
donner : **qu'aucune variante n'a été oubliée**. C'est le seul mécanisme du
projet qui transforme « ajouter une variante » en « le test rougit tant que tu
n'as pas traité la nouvelle ». Pour une palette d'éditeur, c'est central.

`familles_de_propulsion_partitionnent_les_variantes` est le meilleur du lot :
il vérifie une **partition** (couverture *et* non-vacuité), pas juste une
boucle.

### D — Verrous de valeur (≈ 20)

Les 8 tests de fumée « classe C » ajoutés en 2026-07-29 avant le découpage
(§E.1 de `stations.md`), et les assertions du type
`assert_eq!(c.cout(), 5.0)` / `assert_eq!(c.rayon_local(), 1.75)`.

Ils figent une valeur calculée, sans dire d'où elle vient. Ce sont des
**détecteurs de changement**, pas des détecteurs d'erreur : ils rougissent
quand la valeur bouge, jamais quand elle est fausse.

**Verdict : légitimes, mais datés.** Ils ont été écrits pour une raison
précise et avouée — servir de filet pendant l'extraction de `composant.rs` en
modules. Cette extraction est **faite** (§E.2, 2026-07-29). Le filet a servi.
Ils ne coûtent rien, donc rien n'oblige à les retirer, mais il ne faut pas
les confondre avec de la couverture : **8 variantes de classe C n'ont
toujours aucun test qui dise ce qu'elles doivent *être*.**

---

## 3. Le critère qui sépare les bons des mauvais

Le chantier ISV a produit un enseignement net, déjà consigné en §C.29 : **la
quasi-totalité des erreurs rencontrées ont été des tests qui mesuraient autre
chose que ce qu'ils prétendaient** — jamais des erreurs de géométrie. Six cas
distincts, tous du même genre.

D'où le critère opérationnel, qui vaut pour tout ce qui suit :

> **Un test est utile s'il existe une modification plausible du code qui le
> fait rougir, et si cette modification est exactement celle contre laquelle
> on voulait se protéger.**

Corollaire pratique, et c'est le seul filet qui a réellement fonctionné : le
**red-check**. Casser volontairement ce que l'assertion prétend garder, et
vérifier qu'elle rougit. Sur l'ISV il a trouvé **trois défauts dans le code**
(pas dans les tests), dont la position de tête codée en dur qui aurait
détaché les boucliers de toute épine redimensionnée.

Trois exemples de ce que le red-check a démasqué, et qui montrent la forme du
piège :

| Le test disait | Il mesurait en fait | Ce qui restait vert |
|---|---|---|
| « les écailles se recouvrent » | la denture, pas le recouvrement | `RECOUVREMENT = 0` (écailles bout à bout) |
| « la tête coiffe le bout de l'épine » | une position absolue | épine divisée par deux, tête restée à 95,0 |
| « le bouclier ne clippe pas » | des circonrayons | rotation relative des hexagones ignorée |

---

## 4. Ce qui, aujourd'hui, ne porte pas son poids

Nommément, et sans en faire un drame — le coût d'exécution est nul, l'enjeu
est la **lisibilité du parc**, pas la performance.

### 4.1 Tautologies (`unites.rs`)

| Test | Problème |
|---|---|
| `diametre_double_du_rayon` | `diametre()` **est** `2.0 * rayon()` — le test récite l'implémentation |
| `compatibilite_reflexive_et_symetrique` | `compatible()` **est** `self == autre` — idem |
| `noms_distincts` | garde un `match` de 4 littéraux distincts à l'œil nu |
| `rayons_strictement_croissants` | garde un `match` de 4 constantes ordonnées |

Aucune modification plausible ne les fait rougir sans être déjà visible à la
lecture. **À laisser** (ils ne gênent pas), mais à ne pas prendre pour modèle :
ils gonflent le compteur de 4 unités sans rien garder.

En revanche `longueur_module_bornes_et_plage` et ses deux sœurs **sont bons** :
ils testent les bornes du `clamp` *et* le passage dans la plage *et* le
négatif. C'est la forme à copier.

### 4.2 Assertions trop lâches (`generateur.rs`)

| Test | Assertion | Pourquoi c'est faible |
|---|---|---|
| `generer_est_deterministe` | même **nombre de pièces** | deux stations totalement différentes passent. Le déterminisme porte sur la géométrie, pas sur un compteur |
| `complexite_influe_sur_le_nombre_de_pieces` | `grande >= petite` | l'égalité passe — donc « la complexité n'influe pas » passe aussi |
| `presets_iss_et_mir_produisent_des_stations` | `nb >= 5` / `nb >= 4` | de la fumée sur deux presets historiques |

`generer_est_deterministe` est le plus problématique des trois : c'est le seul
qui **prétend** garantir quelque chose de fort (le déterminisme du générateur,
sur lequel repose l'idée même de « graine ») et qui ne le garantit pas. À
réécrire en comparant les pièces cuites (composant + transformée), pas leur
nombre.

`complexite_influe_sur_le_nombre_de_pieces` se corrige en `>`.

### 4.3 Ce qui n'est pas un défaut, contrairement aux apparences

Deux choses pourraient passer pour de la redondance et n'en sont pas :

- **Les 49 tests de décision de l'ISV.** L'asset est clos ; on pourrait croire
  qu'ils ne servent plus. C'est l'inverse : leur seule raison d'être commence
  **maintenant**, quand quelqu'un touchera à `Profil`, à `ISV_ECHELLE` ou aux
  primitives de `pieces.rs` sans savoir ce qui en dépend.
- **Les 8 tests de fumée classe C.** Périmés dans leur *motif* (l'extraction
  est faite), mais ils restent le seul filet sur ces variantes.

---

## 5. Ce qui manque **avant même** de parler d'assembleur

Cinq trous, indépendants de tout éditeur, classés par ce qu'ils coûteront s'ils
ne sont pas comblés.

### 5.1 Le catalogue de briques n'a aucun test — et il est indexé à la main

`ecran/station.rs` tient le catalogue des briques dans un `match` sur un
`usize`, et la **même information y vit à trois endroits indépendants** :

```rust
Categorie::Briques => 27,                   // ligne  75 — le compte
const BRIQUE_RADIATEUR: usize = 6;          // ligne  40 — un indice nommé
const MEGA_ISV: [usize; 1] = [1];           // ligne  45 — idem
// …
6 => (demo_radiateur_mega(self.regime), …), // ligne 250 — le contenu réel
_ => (EtatStation::Prete(demo_chantier()), …), // ligne 270 — l'attrape-tout
```

Trois défauts s'empilent :

1. **Le `6` est écrit deux fois**, sous deux formes différentes. Insérer une
   brique en position 3 décale tout et ne casse **rien** — ni compilation, ni
   test. Le bouton « allumer » se retrouve simplement sur la mauvaise pièce.
2. **Le compte `27` est une troisième source.** Rien ne le relie aux bras du
   `match`. Ajouter une brique sans toucher au compte la rend inatteignable ;
   augmenter le compte sans ajouter le bras…
3. …tombe dans **l'attrape-tout `_ =>`**, qui rend silencieusement la brique
   « CONSTRUCTEUR PAR PORTS LIBRES ». Trois entrées de menu montreraient alors
   la même chose sans qu'aucune erreur ne soit signalée nulle part.

C'est exactement la classe d'erreur que le chantier ISV a passé son temps à
chasser — **une valeur qui a plusieurs sources** — sauf qu'ici il n'y a
*aucun* test. Le hasard seul a voulu que ça ne morde pas encore : les briques
ont toujours été ajoutées en fin de liste.

**Manque** : un test qui exerce chaque indice de `0` à `compte-1`, vérifie que
chacun produit une station non vide, que **le dernier bras nommé est bien
`compte-1`** (donc que l'attrape-tout n'absorbe rien), et que les indices
nommés (`BRIQUE_RADIATEUR`, `MEGA_ISV`) désignent la brique attendue par son
titre. Coût : quelques lignes. C'est le meilleur rapport valeur/effort de
toute cette liste — et l'assembleur, qui aura une palette bien plus grande
qu'un `match` de 27 bras, ne survivra pas à cette forme-là.

### 5.2 Rien ne vérifie que `rayon_local` et `englobant_local` s'accordent

Deux mesures d'encombrement coexistent, pour deux usages :

- `rayon_local()` → cadrage caméra, `Station::rayon()` ;
- `englobant_local()` → **anti-collision** de `Chantier::poser`.

Pour une douzaine de variantes elles sont identiques par construction
(`(Vec3::ZERO, self.rayon_local())`), pour les autres elles divergent
volontairement (une sphère décalée pour un propulseur, une coiffe, une
nacelle). Rien ne vérifie que la sphère décalée **contient réellement** la
pièce.

Conséquence pour l'assembleur : une pose refusée sans recouvrement visible, ou
acceptée avec interpénétration franche. Dans le générateur, une pose refusée
est un non-événement (la grammaire réessaie ailleurs). **Face à un humain qui
vient de cliquer, c'est un bug.**

Cas limite déjà présent : `Composant::Panache` renvoie `rayon_local() = 0` et
`englobant() = (ZERO, 0)` — délibérément, c'est un effet et non une pièce
(sinon la caméra recule de deux longueurs de vaisseau à l'allumage). Mais dans
un `Chantier`, une pièce de rayon nul **ne peut jamais entrer en collision**.
Correct aujourd'hui ; à documenter comme exception explicite avant qu'une
palette ne le propose au clic.

### 5.3 La collision est sphérique, et une seule direction est testée

`FACTEUR_COLLISION = 0.85` sur des sphères englobantes, et un seul test :
`collision_rejette_recouvrement` — un **vrai positif**. Aucun test de **faux
positif** : deux pièces longues et fines côte à côte (un radiateur le long
d'un treillis) ont des sphères qui se recouvrent largement alors que les
géométries ne se touchent pas.

Le générateur s'en accommode. Un éditeur non : le joueur verra un
emplacement manifestement libre être refusé.

**Manque** : au minimum un test qui *documente* la limite (« deux pièces
élancées parallèles sont refusées alors qu'elles ne se touchent pas »), pour
que la décision de garder ou remplacer les sphères soit prise en connaissance
de cause plutôt que découverte à l'usage.

### 5.4 `generer_est_deterministe` ne garantit pas le déterminisme

Voir §4.2. À réécrire avant d'y adosser quoi que ce soit.

### 5.5 Aucune variante n'est testée pour sa **sortie de dessin**

`dessiner()` est exercé (via `Batisseur`) uniquement pour les 8 briques classe
C, et seulement en « ne panique pas ». Les tests de géométrie de l'ISV
mesurent des sommets cuits — mais brique par brique, à la main.

**Manque** : un test d'énumération (famille C) qui, pour **toute** variante,
cuit la géométrie et vérifie trois invariants faibles mais universels :
géométrie non vide, indices dans les bornes, sommets finis (ni `NaN` ni `inf`).
Un `NaN` dans une dimension se propage silencieusement jusqu'à faire
disparaître un lot entier à l'écran.

---

## 6. Ce que l'assembleur exigera en plus

§E.4 de `stations.md` listait les **fonctionnalités** manquantes : retrait,
undo/redo, métadonnées de palette, sérialisation. Cette section dit ce qu'il
faudra **garantir**, et signale trois défauts de conception que l'audit fait
remonter.

### 6.1 Défaut : les indices de ports libres sont instables

`Chantier::poser` retire le port consommé par `swap_remove` — et le code le
dit lui-même :

> « Les indices ne sont valides que jusqu'à la prochaine `poser`/`racine`. »

Pour le générateur, c'est sans conséquence : il choisit un port et pose
immédiatement, dans la même expression. **Pour une interface, c'est
rédhibitoire** : l'utilisateur clique un port à l'image *n*, la pose a lieu à
l'image *n+3*, et entre-temps un `swap_remove` a fait migrer un port
quelconque sur l'indice qu'on tenait. Le symptôme sera une pièce qui se pose
au mauvais endroit, de façon **intermittente et non reproductible** — la
pire catégorie de bug à diagnostiquer.

Ce n'est pas un manque de test, c'est un **choix de représentation** qui n'a
jamais eu à supporter un consommateur asynchrone. Il faut un **identifiant
stable** de port libre (compteur monotone, jamais réutilisé), et `libres()`
devient une vue indexée par cet identifiant.

Tests à écrire, dans l'ordre :

1. l'identifiant d'un port libre **survit** à la pose d'une pièce ailleurs ;
2. l'identifiant d'un port **consommé** ne se recycle jamais ;
3. poser sur un identifiant périmé **échoue proprement** (pas de panique, pas
   de pose au mauvais endroit).

Le point 3 est celui qui compte : il définit le comportement face à une UI
qui a du retard sur le modèle.

### 6.2 Le retrait : la moitié manquante, et sa symétrie

`poser` existe, `retirer` non. Écrire `retirer` demande de décider trois
choses que rien ne tranche aujourd'hui, et chacune est un test :

| Question | Test qui la fige |
|---|---|
| Retirer une pièce retire-t-il son sous-arbre ? | `retirer_une_branche_emporte_ses_enfants` |
| Le port hôte redevient-il libre ? | `retirer_libere_le_port_qui_portait_la_piece` |
| Le budget est-il remboursé ? | `retirer_rembourse_exactement_le_cout_pose` |

Et surtout l'invariant qui les résume, et qui est le vrai filet :

> **`poser` puis `retirer` ramène le chantier à un état indiscernable de
> l'état initial** — mêmes pièces, mêmes ports libres, même budget restant.

C'est une **propriété d'aller-retour** (round-trip). Elle vaut plus que les
trois tests séparés, parce qu'elle reste vraie quand on ajoute un champ au
`Chantier` que les trois autres ignoreraient.

### 6.3 Undo/redo : la même propriété, à l'échelle d'une session

Une fois `retirer` acquis, undo/redo est une pile d'opérations réversibles.
La propriété est la même, appliquée à une **séquence arbitraire** :

> *n* opérations suivies de *n* undo ramènent à l'état initial ; *n* redo
> ramènent à l'état après les *n* opérations.

C'est ici que le projet gagnerait à sortir de l'exemple choisi à la main. Tous
les tests actuels exercent des séquences **écrites par un humain qui savait ce
qu'il cherchait** — et c'est précisément ce qui a laissé passer les six
mesures fausses de §3. Une séquence pseudo-aléatoire d'opérations
pose/retire/undo/redo, rejouable par graine (le générateur sait déjà faire
ça : `toutes_graines_donnent_une_station_finie`), trouvera ce qu'aucun cas
écrit à la main ne trouve.

### 6.4 Sérialisation : la seconde propriété d'aller-retour

Format proposé en §E.4 : la liste ordonnée des `(Composant, port hôte visé)`,
suffisante pour rejouer la construction sans sérialiser les `Mat4` cuites.

Ce choix a une conséquence directe, et elle doit être testée :

> **Rejouer la liste doit reproduire la géométrie cuite au sommet près.**

Ce qui n'est vrai que si la construction est **déterministe** — d'où le fait
que §5.4 (`generer_est_deterministe` qui ne teste qu'un compteur) n'est pas un
détail cosmétique mais un **prérequis** de la sauvegarde. Si l'ordre de pose
influence la géométrie autrement que par les ports visés, une sauvegarde ne se
recharge pas à l'identique et personne ne s'en apercevra avant qu'un joueur ne
le signale.

Deuxième piège, plus discret : `SousEnsemble` porte un `Rc<DonneesSousEnsemble>`
contenant des `Mat4` **déjà cuites**. Un assemblage qui contient un
sous-ensemble ne peut donc pas se rejouer par la seule liste des poses — il
faut soit sérialiser la recette du sous-ensemble, soit ses pièces. À trancher
avant d'écrire le format, pas après.

### 6.5 La palette : le seul endroit où l'énumération devient critique

§E.4 le dit : il manque « énumérer tous les composants posables sur CE port
libre ». `Chantier::compatibles(&comp, montage_idx)` fait l'inverse (les ports
qui acceptent un composant donné). Il faut la duale.

Et là, la famille C (§2) cesse d'être un confort et devient la garantie
centrale :

- **une variante absente de la palette est invisible pour le joueur** — elle
  existe dans le code, coûte de la maintenance, et n'est jamais posée ;
- **une variante présente mais impossible à poser** est une entrée morte qui
  se clique et ne fait rien.

Les deux tests correspondants sont des partitions, sur le modèle de
`familles_de_propulsion_partitionnent_les_variantes` :

1. toute variante de `Composant` apparaît dans **exactement une** catégorie de
   palette (couverture *et* non-duplication) ;
2. toute variante proposée sur un port donné y est **effectivement posable** —
   c'est-à-dire que `compatibles` et la palette disent la même chose,
   vérifié en bouclant sur toutes les variantes × tous les genres de ports.

Le test 2 est celui qui empêche la palette et le moteur de diverger, et c'est
la divergence qui est certaine à terme : ce sont deux `match` sur le même enum,
écrits à deux endroits, exactement la configuration qui a produit le doublon
d'indice de §5.1.

### 6.6 Ce qu'il ne faut **pas** tester

Par symétrie avec l'audit, et pour ne pas refaire à l'assembleur ce qui a été
fait de mieux sur l'ISV :

- **pas de test de rendu** (couleurs, disposition des boutons, position d'un
  panneau à l'écran) — ce sont des arbitrages qui se rendent à l'œil, et un
  test les fige sans les justifier. L'ISV a montré que ces arbitrages
  **changent de sens délibérément** (`la_propulsion_touche_le_pied_de_lepine`,
  trois fois) ; un test de rendu se contenterait de les contredire ;
- **pas de verrou de valeur nouveau** (§2 famille D) : ils avaient une raison
  datée — sécuriser une extraction — pas une raison permanente ;
- **pas de test qui recalcule** ce qu'il vérifie. C'est le piège n° 1 du
  catalogue §C.29 : un test qui recalcule le seuil au lieu de le lire passe
  toujours.

---

## 7. Ordre de travail proposé

Deux lots au départ. Le premier ne dépend pas de l'assembleur et vaut d'être
fait quoi qu'il arrive ; le second est l'assembleur lui-même. Les deux sont
clos ; la suite (lots 3 à 5, l'écran) est arrêtée en **§7.1**.

**Lot 1 — combler ce qui manque au code actuel** (§5, ~une demi-journée)

1. Test d'énumération du catalogue de briques (§5.1) — meilleur rapport
   valeur/effort, et supprime la double source d'indices.
2. Test d'énumération « toute variante cuit une géométrie finie et non vide »
   (§5.5).
3. Réécrire `generer_est_deterministe` sur la géométrie (§5.4) — **prérequis**
   de la sérialisation (§6.4).
4. Test d'accord `rayon_local` / `englobant_local` (§5.2), et documenter
   l'exception `Panache`.
5. `complexite_influe_sur_le_nombre_de_pieces` : `>=` → `>`.

**Lot 2 — l'assembleur**, dans cet ordre, chaque étape se validant seule

1. **Identifiants stables de ports libres** (§6.1). En premier : tout le reste
   s'appuie dessus, et le rétrofit après coup toucherait chaque appelant.
2. **`retirer`** + la propriété d'aller-retour (§6.2).
3. **Undo/redo** + la propriété de séquence, exercée par graine (§6.3).
4. **Palette** + les deux partitions (§6.5).
5. **Sérialisation** + l'aller-retour géométrique (§6.4), après avoir tranché
   le cas `SousEnsemble`.

Les points 1 à 3 sont du **modèle** (`chantier.rs`), testables entièrement
sans vue. Le point 4 touche la vue ; le point 5 est indépendant. Aucun ne
demande de toucher aux 31 variantes de composants — c'était l'objet du
découpage §E.2, et il tient.

### 7.1 La suite, arrêtée après la clôture du Lot 2 (2026-08-01)

Les deux lots ci-dessus sont clos. §8 décrit l'écran en entier mais n'avait
jamais été rattaché à un lot — le voici découpé, après une relecture du
modèle livré **contre** ce que §8 exige réellement.

**Lot 3 — compléter le modèle pour l'écran.** Trois manques relevés en
confrontant §8 au `Chantier` livré. Aucun n'était prévisible depuis §7, qui
listait ce qui se voyait dans l'abstrait ; ceux-ci ne se voient qu'en lisant
la spec de l'écran ligne à ligne. Tous trois sont **additifs et en lecture
seule** — donc sans le risque de rétrofit qui avait imposé L2.1 en tête — et
tous trois sont **testables sans vue**, donc red-checkables comme le Lot 2.

1. **`pose_prevue(hote_id, comp, montage)`** (§8.3). Le fantôme doit être à la
   pose **exacte** qu'aurait la pièce au clic ; `poser` calcule aujourd'hui
   cette transformée en interne et ne la publie pas. Sans cette méthode, la
   vue la recalcule — la deuxième source que §8.3 interdit nommément, et
   exactement le doublon que le Lot 1 a passé son temps à supprimer.
2. **`sous_arbre(id)`** (§8.3, état « pièce sélectionnée »). `retirer` le
   calcule déjà en une passe, sans l'exposer. À extraire, pas à réécrire côté
   vue.
3. **Désignation** : le port libre et la pièce sous le curseur. Rien
   n'existe — `Camera::pick` ne sait viser que les astres d'un `Systeme`. Deux
   problèmes distincts : le port se traite en espace écran (projeter
   `PortLibre::repere.pos`, qui est déjà en monde, et prendre le plus proche
   sous un rayon), la pièce est une question de géométrie (rayon contre
   `Enveloppe`) qui a sa place dans `enveloppe.rs`, à côté des fonctions de
   distance, et se contrôle en force brute comme elles.

**Lot 4 — l'écran d'assemblage** (§8.2 à §8.4) : entrée au menu, colonne
palette, les trois états d'interaction, les trois couleurs, le bandeau bas.
Découpage arrêté à la clôture du Lot 3, une fois le modèle réellement complet
— c'est le choix rendu par l'utilisateur le 2026-08-01, contre un découpage
figé d'avance. **Le détail est en §10**, qui tranche aussi ce que §8 laissait
ouvert (chantier vide, grisage de la palette, cache d'état des ports,
clic contre glissé, choix du montage).

⚠️ **C'est le premier lot majoritairement non testé, et c'est voulu** (§6.6 :
pas de test de rendu). La discipline change donc de forme : plutôt que
« red-checker chaque test », il s'agit de **pousser hors du code de dessin**
tout ce qui se décide, vers des requêtes de modèle qui, elles, se
red-checkent. Ce qui reste non testé doit être seulement *où le rectangle se
pose*, jamais *ce qu'il signifie*. Le Lot 3 est le premier versement de cette
règle.

C'est aussi là que se solde une dette ouverte depuis L1.6 : `peut_poser` n'a
toujours aucun test dédié, reporté trois fois faute de consommateur réel
(L2.4 est passée par `posables`, pas par lui). Son vrai consommateur est le
code couleur de §8.4.

**Lot 5 — l'écran qui garde et qui explique** : sauvegarde/chargement sur
disque (`recette`/`depuis_recette` de L2.5 n'ont toujours aucun consommateur)
et l'**overlay** de §8.5 adapté à l'assembleur. Détail en §10.8.

**Lot 6 — ce que seul l'usage dira**, à ne pas commencer avant d'avoir
construit quelque chose à la main : l'**arbitrage de L1.4** (19 variantes
sous-déclarent leur `rayon_local` ; §8.5 dit que la serre d'une enveloppe ne
se juge qu'à l'œil, et la dette est parquée depuis le Lot 1 faute d'avoir pu
en *voir* une), les **composites** (`figer`, arbitrage rendu le 2026-08-01 :
c'est en assemblant qu'on verra quels regroupements reviennent), et le sort
de `GenrePort::PoutreBout`. Détail en §10.8.

---

## 8. L'écran d'assemblage

> Conception arrêtée le **2026-08-01**. Deux choix ont été tranchés par
> l'utilisateur avant d'écrire quoi que ce soit, parce qu'ils décident de la
> disposition et non d'un détail :
>
> - **pièce d'abord** (façon KSP) : on choisit dans une palette toujours
>   visible, puis les ports compatibles s'allument sur le vaisseau ;
> - **bac à sable libre** : aucun plafond de coût. Le coût reste affiché, à
>   titre indicatif.

### 8.1 Pourquoi « pièce d'abord » change la disposition

Les deux modèles ne demandent pas le même écran. « Port d'abord » n'a besoin
que d'un menu contextuel, apparaissant au clic et disparaissant ensuite — la
vue 3D occupe tout. « Pièce d'abord » demande une **palette permanente**, donc
une colonne réservée, et transforme la 3D en zone de dépôt.

C'est aussi le modèle qui **passe à l'échelle** : la palette grossira (« il
faudra ajouter de nouveaux composants pour meubler l'assemblage »), et une
liste catégorisée que l'on parcourt à froid supporte trente entrées, là où un
menu contextuel qui en propose trente au clic est illisible.

### 8.2 Zones

```
┌──────────┬──────────────────────────────────────────────┐
│ PALETTE  │                                              │
│ ▸ STRUCT │            ○ ports compatibles               │
│ ▸ HABITAT│          ╔═══╗                               │
│ ▸ PROPUL │       ○──╢   ╟──○                            │
│ ▸ ENERGIE│          ╚═══╝                               │
│ ▸ THERMIQ│            ○         ▒ fantôme sous curseur  │
│          │                                              │
│ ┌──────┐ │                                              │
│ │TREILL│ │                                              │
│ │ P1   │ │                                              │
│ └──────┘ │                                              │
│ SELECTION│                                    ⊕ boussole│
├──────────┴──────────────────────────────────────────────┤
│ 45 pieces · cout 128 · 110k sommets   [UNDO] [REDO]  E P│
└─────────────────────────────────────────────────────────┘
```

- **Colonne palette** (gauche, largeur fixe) : catégories repliables, puis les
  pièces. Réutilise `ui::minitel_panel` / `ui::minitel_ligne` — l'assembleur
  n'invente pas une esthétique, il reprend celle des autres vues.
- **Vue 3D** : le vaisseau, les ports, le fantôme. Caméra orbitale comme
  `VueStation`.
- **Bandeau bas** : compteurs, undo/redo, et les bascules de débogage.
- **Boussole** : coin bas-droit, comme partout ailleurs (`ui::boussole_axes`).

### 8.3 Les trois états de l'interaction

Tout tient en trois états, et c'est volontairement peu :

| État | Ce qu'on voit | Ce qui fait sortir |
|---|---|---|
| **Repos** | le vaisseau seul | choisir une pièce dans la palette |
| **Pièce en main** | les ports **compatibles** allumés, un fantôme sur le port survolé | clic sur un port → pose ; Échap → repos |
| **Pièce sélectionnée** | la pièce et son sous-arbre surlignés | Suppr → retrait ; clic ailleurs → repos |

**Le fantôme est la pièce maîtresse de la lisibilité.** Il est dessiné à la
pose **exacte** qu'aurait la pièce si on cliquait — pas une approximation. Il
n'y a qu'une façon de le garantir : demander la pose au même code que la pose
réelle (`accoupler(port_hôte, port_montage)`), pas la recalculer côté vue. Une
deuxième source ici donnerait un fantôme qui ment, et c'est exactement le
genre de doublon que le Lot 1 a passé son temps à supprimer.

### 8.4 Trois couleurs, trois sens

Un port compatible n'est pas forcément **posable** : le profil peut convenir et
l'anti-collision refuser. La distinction doit se voir sans cliquer, sinon
l'utilisateur essaie et ne comprend pas.

| Couleur | Sens | Source |
|---|---|---|
| vert | posable | `Chantier::peut_poser` → `true` |
| rouge | compatible mais **encombré** | compatible, mais `peut_poser` → `false` |
| éteint | profil ou genre incompatible | pas dans `compatibles` |

`peut_poser` existe déjà (ajouté en L1.6) : c'est précisément ce contrôle sans
effet de bord qui rend cet affichage possible.

### 8.5 L'overlay de débogage : expliquer le refus, pas décorer

L'overlay n'est pas « dessiner les sphères pour voir ». Il répond à **une**
question, celle qui rendra l'assembleur incompréhensible si elle reste sans
réponse : *pourquoi ce port est-il rouge ?*

Ce qu'il dessine, par bascule (touche **E**) :

1. **L'enveloppe de chaque pièce** — la capsule en fil de fer : un anneau à
   chaque bout, quatre génératrices, les calottes. En cyan sombre, pour rester
   derrière la géométrie.
2. **L'enveloppe du fantôme**, en blanc — celle qu'on est en train de proposer.
3. **Le segment de plus courte approche** entre le fantôme et la pièce qui le
   refuse, tracé en rouge, avec l'écart chiffré.

Le troisième point est le seul qui compte vraiment. Un refus devient alors une
phrase : *« ce radiateur est à 1,2 de cette poutre, il en faut 1,8 »* — au lieu
d'un port rouge sans explication. `Enveloppe::ecart` rend déjà ce nombre.

**Un deuxième usage, immédiat celui-là** : l'overlay est la seule façon de voir
les enveloppes trop lâches. Le relevé de L1.4 dit que 19 variantes
sous-déclarent leur rayon, et L1.6 a converti les allongées en capsules — mais
personne n'a encore *vu* une seule de ces enveloppes. L'overlay les met à
l'épreuve du regard, ce qu'aucun test ne fait.

⚠️ **Il ne remplace pas les tests.** La contenance se mesure
(`les_rayons_declares_contiennent_la_piece`) ; l'overlay sert à juger la
**serre** — une enveloppe peut contenir la pièce et rester ridiculement large,
et ça, seul l'œil le dit.

### 8.6 Ce que l'écran demande au modèle, et qui manque encore

Rien de cette section ne se code sans les étapes du Lot 2, et c'est le point de
l'avoir écrite d'abord — elle dit **pourquoi** chacune est nécessaire :

| Il faut | Pour | Étape |
|---|---|---|
| un identifiant **stable** de port libre | tenir le port survolé d'une image à l'autre ; `swap_remove` le fait migrer | L2.1 |
| `retirer` | l'état « pièce sélectionnée » et la touche Suppr | L2.2 |
| undo/redo | le bandeau bas | L2.3 |
| énumérer les pièces posables | remplir la palette et la griser | L2.4 |

**L'overlay, lui, ne dépend d'aucune.** Il ne lit que des enveloppes et des
poses, qui existent déjà. C'est donc ce qu'on peut construire **maintenant**,
et ce qui servira à valider tout le reste ensuite.

---

## 9. Le « boudin » : l'enveloppe qui manque aux plaques

> Idée de l'utilisateur, **2026-08-01**. Elle résout le point resté ouvert en
> L1.8 : une plaque plate n'est pas une capsule, et aucun couple (axe, rayon) ne
> la borne sans gaspiller.

### 9.1 Le problème

Une capsule autour d'une plaque de bouclier (12 de rayon, 1,8 d'épaisseur) a un
rayon de 12 **dans toutes les directions**, y compris les deux où la pièce est
mince. C'est la même faute que la sphère autour d'un radiateur (§L1.6), d'un cran
plus loin : la capsule règle **une** dimension, la plaque en a **deux** de fines.

### 9.2 La forme : aplatir la capsule là où il n'y a rien

La capsule doit **s'aplatir** dans la direction où la matière s'arrête, et garder
son arrondi ailleurs — un **boudin**, pas un cylindre. Concrètement : la surface
médiane de la plaque, gonflée d'un petit rayon.

```
        capsule                        boudin
     (rayon 12 partout)        (aplati sur l'épaisseur)

      ╭───────────╮                ╭─────────────╮
     │             │              ╰───────────────╯
     │    ▬▬▬▬▬    │  ← la plaque      ▬▬▬▬▬▬▬
     │             │              ╭───────────────╮
      ╰───────────╯                ╰─────────────╯
      énorme volume vide           colle à la pièce
```

Le rayon du boudin est la **demi-épaisseur plus un petit jeu** : l'enveloppe se
tient ainsi légèrement au-dessus de la plaque, sans jamais la traverser, et garde
son arrondi sur les bords — c'est ce qui la distingue d'une boîte, dont les
arêtes vives rendraient la distance coûteuse à calculer.

### 9.3 La généralisation qui tient les trois formes

Plutôt qu'un troisième cas particulier, une seule idée les couvre toutes :

> **Une enveloppe est un noyau convexe, gonflé d'un rayon.**

| Noyau | Enveloppe obtenue | Pour |
|---|---|---|
| un **point** | sphère | pièces ramassées |
| un **segment** | capsule | pièces allongées |
| un **rectangle** | boudin | plaques |

C'est une somme de Minkowski, et elle a la propriété qui compte ici : la distance
entre deux enveloppes vaut **la distance entre leurs noyaux, moins la somme des
rayons**. Exactement la formule déjà employée par `Enveloppe::ecart` — la sphère
et la capsule en deviennent les cas dégénérés (segment de longueur nulle,
rectangle de largeur nulle), et `Chantier::collision` ne change pas d'un iota.

Ce qu'il faut écrire en plus : **distance rectangle↔rectangle** et
**rectangle↔segment**. Rien de plus, et rien qui touche le reste.

### 9.4 Ce que ça règle, au-delà des plaques

- **Les boucliers** : `BouclierPetit` et `BouclierGrand` sont des disques, et
  leur sphère actuelle réserve leur rayon entier en épaisseur. Ce sont les deux
  pièces que l'ISV empile à quatre exemplaires sur un mât — donc précisément là
  où un englobant trop épais interdirait l'empilement à un humain.
- **Les voiles** (panneaux solaires, radiateurs) : leur capsule règle la
  longueur, pas la largeur.
- **Le mesureur** : le relevé de L1.8 pourra alors être exact sur ces familles,
  au lieu de rendre une borne qu'on doit vérifier à la main.

### 9.5 Ordre

À faire **avant** L2.1 si l'on veut que l'assembleur naisse avec une collision
honnête sur les plaques ; après, si l'on préfère voir l'éditeur tourner d'abord.
Le travail ne dépend d'aucune étape du Lot 2 et n'en bloque aucune.

---

## 10. L'écran, en détail : ce que §8 laissait ouvert

> Écrit le **2026-08-01**, à la clôture du Lot 3, quand le modèle a été complet
> — c'est-à-dire au moment où les questions de l'écran cessent d'être
> hypothétiques. §8 décrit *ce qu'on voit* ; cette section décide *ce qui se
> passe*, et c'est là que sont les vrais choix.

### 10.1 Le chantier vide : un quatrième état, ou plutôt un demi

§8.3 donne trois états — repos, pièce en main, pièce sélectionnée — et les
trois **supposent un vaisseau déjà là**. Or un `Chantier` naît vide, et sa
première pièce ne se pose pas comme les autres : `racine(comp)` la place à
l'origine, sans port hôte, parce qu'il n'y en a aucun.

D'où la règle, qui n'ajoute pas d'état mais dédouble le repos :

- **chantier vide** : n'importe quelle pièce de la palette se pose
  immédiatement au clic, comme racine. Aucun port à viser, donc aucune étape
  intermédiaire — cliquer une pièce *est* la poser ;
- **chantier non vide** : le comportement de §8.3, pièce en main puis port.

Ne pas l'écrire aurait donné un écran où le premier clic ne fait rien, sans que
rien n'explique pourquoi.

### 10.2 « Pièce d'abord » et `posables` : lever une contradiction apparente

§8.1 a tranché **pièce d'abord** : on choisit dans la palette, *puis* les ports
compatibles s'allument. C'est `Chantier::compatibles(comp, montage)` — du
composant vers les ports.

Or L2.4 a construit `posables(genre, profil)` — **du port vers les composants**,
la duale. Dans un écran pièce d'abord, il n'y a pas de port sélectionné au
moment où l'on remplit la palette. À quoi sert-elle donc ?

**À griser la palette**, et c'est un vrai besoin : une entrée qui ne peut se
poser **nulle part** sur le vaisseau courant doit se voir, sinon l'utilisateur
la choisit et découvre un vaisseau sans aucun port allumé — un cul-de-sac muet.

La façon naïve de calculer ce grisage est de balayer, pour chacun des 31
composants, tous les ports libres. La bonne s'appuie sur `posables` :

1. relever l'ensemble des couples `(genre, profil)` **distincts** parmi les
   ports libres — il y en a une poignée (quatre genres, quelques profils), pas
   un par port ;
2. appeler `posables` une fois par couple, faire l'union.

Le coût passe de « 31 × nombre de ports » à « 31 × nombre de couples
distincts », c'est-à-dire qu'il **cesse de croître avec la station**. C'est la
raison d'être de la duale dans un écran pièce d'abord, et ça mérite d'être noté
ici : sans cette phrase, quelqu'un « corrigera » un jour la palette en bouclant
sur les ports.

### 10.3 Ce qui se recalcule, et quand — le point de performance

Trois choses de coûts très différents cohabitent dans une frame :

| Quoi | Dépend de | Recalculé |
|---|---|---|
| le **maillage cuit** du vaisseau | le chantier | à chaque **mutation**, jamais par frame |
| l'**état des ports** (vert/rouge/éteint) | chantier + pièce en main + montage | à chaque mutation **ou** changement de sélection |
| le **fantôme** | tout ça + le port survolé | **chaque frame** (il suit le curseur) |

Le deuxième est le piège. Colorer les ports demande un `peut_poser` par port,
et chaque `peut_poser` teste la collision contre **toutes** les pièces : sur une
station de 45 pièces et 60 ports libres, c'est 2 700 distances d'enveloppes par
frame, pour un résultat qui **ne change pas** tant qu'on ne touche ni au
vaisseau ni à la pièce en main. Le recalculer à chaque frame serait le gaspiller
entièrement.

Le fantôme, lui, est le seul qui doive suivre le curseur — et il ne coûte qu'un
`pose_prevue`, c'est-à-dire un `accoupler`.

**Conséquence sur la structure** : toute mutation du chantier passe par **une
seule porte** dans la vue (une méthode qui pose/retire/annule/refait *et*
invalide le maillage et l'état des ports). Deux chemins de mutation, et le jour
où l'un oublie d'invalider, l'écran affiche un vaisseau périmé — la famille de
défaut que ce projet traque depuis le Lot 1, transposée à l'affichage.

### 10.4 Clic ou glissé : la discrimination qu'on oublie toujours

`Camera::input_orbite` fait tourner la vue au **glisser gauche**. §8.3 pose au
**clic gauche**. Le même bouton, donc — et sans discrimination, chaque rotation
de caméra se terminerait par une pose accidentelle sous le curseur.

Règle : on mémorise la position à l'enfoncement, et le relâchement ne vaut clic
que s'il a **peu bougé** (quelques pixels). Pas de seuil de durée : un clic long
mais immobile reste un clic, et c'est le geste de quelqu'un qui vise
soigneusement — précisément l'utilisateur qu'on ne veut pas punir.

C'est une petite machine à états, sans rien de graphique : **elle se teste**, et
elle doit l'être. C'est le type même de logique que §7.1 demande de pousser hors
du code de dessin.

### 10.5 Par quel port la pièce s'accroche-t-elle ?

Une pièce a souvent **plusieurs** ports de montage — un `ModuleAxial` a deux
écoutilles. §8 n'en dit rien, et `posables` n'en rend qu'un. Or le choix décide
de l'**orientation** de la pièce posée.

Décision : **le premier montage valide par défaut, et une touche pour cycler**
parmi les autres. Le cycle n'a de sens qu'accompagné du fantôme — c'est lui qui
montre ce que le changement fait —, donc les deux vont dans la même étape. Sans
fantôme, cycler serait un réglage aveugle.

### 10.6 Ce que l'écran ne fera **pas**, et pourquoi

Écrit ici pour que ce soit une décision et non un oubli qu'on redécouvre :

- **pas de roulis libre autour du port.** `accoupler` fixe le roulis (« les
  hauts restent alignés »). L'ajouter voudrait dire un paramètre de plus à
  `poser`, donc un champ de plus dans `Etape` — **le format de sauvegarde**
  (L2.5). Ce n'est pas un réglage d'affichage, c'est une modification du
  modèle et de la persistance : à décider comme telle, pas en passant ;
- **pas de symétrie dans l'éditeur.** Arbitrage déjà rendu (L1.6) : « on ne
  cherche pas la symétrie partout ». La grammaire du générateur s'en sert, la
  main de l'utilisateur n'en a pas besoin ;
- **pas d'occlusion des marqueurs de ports** (relevé en L3.3) : un port derrière
  le vaisseau mais plus près du curseur l'emporte. §8.4 allume les ports
  compatibles, donc l'utilisateur voit ce qu'il vise ;
- **pas de confirmation avant de retirer la racine.** `retirer(racine)` vide le
  vaisseau, et c'est annulable. Le projet n'a de boîte de dialogue nulle part.

### 10.7 Deux points qui demandent un arbitrage

**(a) Le coût affiché.** Le bac à sable n'a pas de budget, donc
`Chantier::budget_restant()` rend `INFINITY` — inutilisable pour le bandeau.
Le coût affiché doit être la **somme des coûts des pièces posées**, calculée par
la vue. Sans conséquence, mais à ne pas confondre.

**(b) Sortir de l'écran perd le vaisseau.** `main.rs` reconstruit un `Accueil`
et **détruit** la vue : Échap efface le travail en cours. Trois issues, dans
l'ordre de coût croissant — garder la vue vivante dans `Etat` (une ligne, mais
le vaisseau survit alors à un aller-retour au menu sans que rien ne le dise),
demander confirmation (le projet n'en a nulle part), ou **assumer la perte
jusqu'à la sauvegarde du Lot 5**. À trancher avec l'utilisateur.

### 10.8 Découpage retenu

**Lot 4 — l'écran d'assemblage** (§8.2 à §8.4). Le but est qu'à la fin *on
puisse construire un vaisseau à la main*, pas qu'il soit confortable.

| # | Étape | Testable ? |
|---|---|---|
| L4.1 | Squelette : entrée au menu, zones (§8.2), caméra orbitale, boussole, bandeau. Un chantier vide qu'on regarde tourner. | non (dessin pur) |
| L4.2 | La palette : catégories repliables, entrées, **grisage** par la voie de §10.2. | le grisage, oui |
| L4.3 | Le clic : discrimination clic/glissé (§10.4), désignation branchée (L3.3), pose racine (§10.1) et pose sur port. | la discrimination, oui |
| L4.4 | Fantôme (`pose_prevue`), les trois couleurs (§8.4) avec le **cache** de §10.3, cycle du montage (§10.5). | l'invalidation du cache, oui |
| L4.5 | Sélection (`piece_sous_rayon`), surlignage du sous-arbre (`sous_arbre`), Suppr, undo/redo au bandeau. | non (branchement) |

Premier geste de L4.1 : réexporter `Chantier` depuis `vaisseau` — laissé de
côté en L3.3 faute de consommateur.

**Lot 5 — l'écran qui garde et qui explique.** Les deux choses qui font passer
l'assembleur de démonstration à outil :

| # | Étape |
|---|---|
| L5.1 | Sauvegarde / chargement sur disque, sur `recette`/`depuis_recette` (L2.5), qui n'ont toujours aucun consommateur |
| L5.2 | Overlay §8.5 adapté à l'assembleur : enveloppes, fantôme en blanc, **segment de plus courte approche** et écart chiffré — répondre à « pourquoi ce port est-il rouge ? » |

**Lot 6 — ce que seul l'usage dira.** À ne pas commencer avant d'avoir
réellement construit quelque chose avec l'écran :

| # | Étape |
|---|---|
| L6.1 | **Arbitrage de L1.4** : les 19 variantes qui sous-déclarent leur `rayon_local`, jugées à l'œil grâce à l'overlay de L5.2 — la dette est parquée depuis le Lot 1 faute d'avoir pu *voir* une enveloppe |
| L6.2 | **Composites** : figer la sélection en `SousEnsemble` (`Chantier::figer`), et la palette qui liste ceux qu'on a créés. C'est en assemblant à la main qu'on verra quels regroupements reviennent — donc ce qu'il vaut la peine de figer |
| L6.3 | `GenrePort::PoutreBout`, posé sur aucun composant (trouvé en L2.4) : lui donner un usage ou le retirer |

### 10.9 La discipline de test, pour un lot majoritairement non testé

§6.6 interdit les tests de rendu, et le Lot 4 est donc le premier lot dont
l'essentiel ne sera pas couvert. La règle de conduite, déjà énoncée en §7.1 et
appliquée une première fois en L3.3 (`ecran::designation` ne dessine rien, et
se teste entièrement) :

> Tout ce qui **se décide** sort du code de dessin. Ce qui reste dedans ne doit
> plus être que *où le rectangle se pose*, jamais *ce qu'il signifie*.

Concrètement, sur ce lot, quatre choses sortent et se testent : le grisage de
la palette (L4.2), la discrimination clic/glissé (L4.3), l'invalidation du cache
d'état des ports (L4.4), et — déjà fait — la désignation (L3.3). Le reste est
de la disposition, et se juge à l'œil, comme §6.6 le demande.

---

## Sources internes

- [`conception/stations.md`](stations.md) Partie C §1 (deux couches),
  Partie E.1–E.4 (refonte des composants, besoins de l'éditeur)
- [`suivi/stations.md`](../suivi/stations.md) §C.29 (bilan de l'ISV : le
  catalogue des six mesures fausses, le red-check)
- [`suivi/bucketlist_globale.md`](../suivi/bucketlist_globale.md) §7 (taille
  des fichiers)
