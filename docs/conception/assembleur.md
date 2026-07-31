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

Deux lots. Le premier ne dépend pas de l'assembleur et vaut d'être fait quoi
qu'il arrive ; le second est l'assembleur lui-même.

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

---

## Sources internes

- [`conception/stations.md`](stations.md) Partie C §1 (deux couches),
  Partie E.1–E.4 (refonte des composants, besoins de l'éditeur)
- [`suivi/stations.md`](../suivi/stations.md) §C.29 (bilan de l'ISV : le
  catalogue des six mesures fausses, le red-check)
- [`suivi/bucketlist_globale.md`](../suivi/bucketlist_globale.md) §7 (taille
  des fichiers)
