# Conception — Chantier Genèse V2 : la carte avant la refonte

> Ouvert le 2026-10-28, à la demande : *« est-ce qu'on est capable de
> quantifier aujourd'hui la complexité complète du chantier Genèse ? J'ai
> l'impression qu'on risque de rater des trucs »*. Ce document ne refait rien :
> il recense **ce que la genèse fait**, **ce que les autres chantiers attendent
> d'elle**, **ce qu'il faut trancher avant de coder** et **les mécanismes qui
> n'existent pas encore**, puis propose un découpage en lots. Même rôle que
> [`opti_v2.md`](opti_v2.md) pour la performance.

## 1. Ce que fait la genèse aujourd'hui

`src/genese/` : `mod.rs` 2 625 lignes, `presets.rs` 1 092, `apparences.rs` 353,
`taille.rs` 78, `persistance.rs` 25 ; 55 tests. Un système **simple** se
construit en une passe (`construire_simple`) :

1. **L'étoile** : `ProfilEtoile::aleatoire` (29 types ; 5 % de particulières),
   habillée par `Variante::appliquer` (source unique), masse dérivée de la
   luminosité (sauf trou noir, quasi-étoile, naine brune : masses propres).
2. **L'échelle** : `echelle_systeme` = `√L` (rémanents et naine brune : plages
   propres). La première orbite passe au-delà de 1,5 rayon d'étoile.
3. **Les planètes** : 3 à 6, espacement géométrique ×1,5–1,85 (Titius-Bode),
   type (tellurique / gazeuse / glacée) tiré selon la distance normalisée
   `a / ech`, **apparence fabriquée** d'après la température d'équilibre
   (`apparence_tellurique(t)` : lave > 600 K, étuve, désert, tempérée
   255–350 K, toundra, gelée). Lunes selon le type.
4. **Les ceintures** (principale, Kuiper) à l'échelle `ech`, le **grand disque**
   des jeunes étoiles, les **comètes**.
5. **Systèmes multiples** (`construire_multiple`) : arbre hiérarchique
   d'étoiles sur rails, plans de placement circumbinaires (`Plan`), mêmes
   planètes ensuite.
6. **Presets écrits à la main** (`presets.rs`) : Soleil, Tau Ceti, Alpha
   Centauri, Proxima, trinaire… à **distances réelles**, apparences du
   catalogue par nom.

Le tout tiré par **`macroquad::rand`, un flux global** : une graine rejoue un
système, mais **un tirage ajouté n'importe où décale tout ce qui suit** (le lot
12 a dû poser son grand disque *sans tirage* pour cette raison), et **aucun
test ne peut bâtir un vrai système** (`fiche.rs`, §5.1 bis d'`interface.md`).

## 2. Les dettes déjà notées pour la refonte

| Dette | Source |
|---|---|
| Cas ajoutés par branches successives (naine brune au centre, trou noir et quasi-étoile à masse propre, comètes, rémanents, exclusions des multiples, plancher de première orbite) | `astres_v2.md` §6 undecies |
| Masse bornée de la quasi-étoile (20 masses de jeu) et sa rareté (~1/1 000) | id. |
| **Plan de référence** d'un système : les disques d'étoile partent de travers | id., 2026-10-25 |
| Raretés provisoires : Herbig, Be, hypergéante | id. |
| Grand disque : systèmes simples seulement | `suivi/astres_v2.md`, lot 12 |
| Étiquette de pollution : systèmes simples seulement | lot 14 |
| **Deux modèles de température** : l'apparence (tempérée 255–350 K) contre la zone (260–320 K) — une planète « habitable » peut être « trop chaude » | `suivi/planetes.md` |
| **Le catalogue n'est pas tiré** : les systèmes générés n'utilisent aucun des 132 presets rocheux, donc ni `planetes.json` ni les noms affichés | `suivi/planetes.md` P1 |

## 3. Ce que les autres chantiers attendent de la genèse

C'est la partie qui fait peur à juste titre : la genèse produit des données
que **d'autres systèmes consommeront**, dont certains ne sont pas conçus.

| Consommateur | Ce qu'il lui faut de la genèse | État du consommateur |
|---|---|---|
| **Planètes** (chantier en cours) | une apparence **tirée du catalogue** (preset), un type de base, une température, les étoiles exposées | prêt (P1–P3) |
| **Habitabilité / colonisation** | un niveau cohérent sur tous les systèmes : l'échelle des coûts (matériaux, recherche) | règles en JSON, à étalonner |
| **Trajectoires** | des corps **sur rails** à position future fermée (`position_future`) | lots A–D faits, E (amarrage) à faire |
| **Points de Lagrange, habitats** | L1–L5 de chaque couple étoile–planète (et planète–lune ?), en forme fermée, **traités comme stables** | **rien dans le code** (`concept_jeu.md` §5) |
| **Stations** | un parent et une orbite libre (`Engin` en orbite analytique) ; à terme, un habitat « complété » qui se comporte comme une planète | changement de modèle, non commencé |
| **Économie, tuiles, recherche** | **les ressources de chaque site** (minerai, eau, hydrogène…) | **non conçu** (`concept_jeu.md` §3, §9) |
| **Starmap** | `construire_systeme_autour(graine, type)` : un système par étoile du voisinage | fait |
| **Sauvegarde** | une graine qui rejoue le système **à l'identique, d'une version à l'autre** | fragile (flux global) |
| **Interface** | noms : presets seulement, numérotation sinon (décision 2026-08-04) | fait |
| **Performance** (Opti-v2) | un budget d'objets par système (disques, comètes, lunes) | O1 à O7, O9, O10 livrés ; la part « nombre d'objets » d'O5 attend G1 |

**La conséquence** : la genèse V2 ne peut pas attendre l'économie, et ne doit
pas la deviner. Elle doit produire **la vérité physique** de chaque corps
(type, masse, distance, température, preset, habitabilité, étoiles
exposées) et laisser **une étape « ressources » branchable** plus tard, qui ne
lira que cette vérité. Si la fiche de sortie est fixée maintenant, l'économie
viendra s'y brancher sans rouvrir la genèse.

## 4. Les décisions à trancher avant de coder

> **État au 2026-11-05** (G0, §7) : 1 tranché à l'inverse de la
> recommandation — une luminosité **physique** pour les calculs, une de rendu
> (§7.1) ; 3, 4, 6, 7 tranchés (§7.6–§7.11) ; **2 et 5 jamais confirmés
> explicitement** (§8.2).

1. **L'échelle et la luminosité.** Les luminosités du jeu sont **comprimées**
   (une O vaut 4 Soleils au lieu de ~100 000 ; Proxima relevée de 0,0017 à
   0,06 pour être visible) ; les presets sont à **distances réelles** avec des
   astres **grossis**. Tout ce qui dérive de L (échelle `√L`, température,
   zone) vit donc dans un monde de jeu cohérent **avec lui-même**, pas avec le
   ciel réel. À décider : on garde une seule luminosité « de jeu » partout
   (simple, cohérent), ou une luminosité physique pour les calculs et une de
   rendu pour l'écran. *Recommandation : une seule, de jeu, documentée.*
2. **Un seul modèle de température.** La température d'équilibre décide de
   tout : apparence, zone, habitabilité. Supprimer la zone binaire au profit
   de **bandes de température** (§5.2).
3. **Catalogue ou procédural.** *Recommandation : la genèse tire un preset du
   catalogue, pondéré par la température, le type (tellurique, glacée) et
   l'étoile*, avec une petite variation procédurale (graine, teinte) par-dessus.
   C'est ce qui rend `planetes.json` (habitabilité, noms, retraits) maître du
   jeu entier.
4. **Le hasard.** Des **flux nommés** dérivés de la graine (un flux pour
   l'étoile, un par planète, un pour les comètes…) au lieu du flux global :
   ajouter un tirage ne déplace plus le reste, et la genèse devient testable
   sans contexte graphique.
5. **Le Soleil, référence ou exception ?** Le système solaire est un preset à
   la main, à distances vraies. La genèse procédurale doit-elle pouvoir
   **produire** un système qui lui ressemble (même échelle, même densité) ?
   *Recommandation : oui, c'est le test d'étalonnage* — tirer 10 000 étoiles G
   et vérifier que le Soleil est dans la distribution.
6. **La rareté.** Une table de fréquences unique (types d'étoile, systèmes
   multiples, planètes habitables par système), réglable comme les deux JSON
   du chantier planètes.
7. **Ce qu'on garantit au joueur.** Par exemple : au moins un site
   colonisable (au sens large) par système ? un système de départ toujours
   viable ?

## 5. Les mécanismes qui n'existent pas encore

### 5.1 Les points de Lagrange

Rien dans le code. Ce qu'il faut : pour un couple (hôte, corps) sur rails,
**les cinq positions en forme fermée** — L4/L5 à ±60° de phase sur l'orbite,
L3 à l'opposé, L1/L2 sur l'axe à la distance de Hill `a·(m/3M)^(1/3)` — et un
**mode de placement** `Astre` « au point Lk de (hôte, corps) » qui suit ce
point (comme `orbiter_autour`), donc compatible avec `position_future` et le
rendez-vous des trajectoires. Taille : **un lot moyen**. La genèse n'a qu'à
**ne rien y poser de gênant** (pas de lune au L4 d'une planète) ; le joueur y
construit.

### 5.2 L'habitabilité graduée par la température

Aujourd'hui (P3) la zone est **binaire** : hors zone = −1 cran, qu'on soit à
0,9 ou à 0,05 UA. Une planète colonisable rasant son étoile reste donc
« dangereuse » — faux. Proposition, dans `assets/habitabilite.json`, à la place
de `zone` :

| T d'équilibre | Effet proposé | Lecture |
|---|---|---|
| > 600 K | plafond **inhabitable** | la surface fond |
| 450–600 K | plafond **hostile** | étuve : habitat blindé, recherche ++ |
| 350–450 K | −1 cran | chaud, sous abri |
| 255–350 K | rien | tempéré |
| 180–255 K | −1 cran | froid, sous dôme |
| 100–180 K | plafond **dangereuse** | glacial : habitat extrême, recherche |
| < 100 K | plafond **hostile** | cryogénique |

Les **habitats extrêmes** restent possibles : seule la surface fondue est
interdite ; le froid et la chaleur coûtent de la recherche (dangereuse,
hostile), pas l'accès. La température vient de `temp_equilibre` avec la
luminosité **de toutes les étoiles** (déjà sommée pour la zone). Taille :
**petit lot** — c'est la suite naturelle du chantier planètes (P4).

### 5.3 Le reste

| Mécanisme | Pourquoi | Taille |
|---|---|---|
| **Tirage pondéré dans le catalogue** | §4.3 : un poids par preset selon T, type, étoile — sans doute dans `planetes.json` (`temperature_min/max`, `poids`) | moyen |
| **Flux de hasard nommés** | §4.4, et testabilité | moyen, transverse |
| **Plan de référence** d'un système | aligner disques d'étoile, ceintures et orbites | petit |
| **Banc statistique de la genèse** | *« c'est la genèse testée en boucle qui fera l'affaire »* : 10 000 graines → distributions (types, nombre de planètes, habitabilité, collisions, plancher) en rapport texte, comme `PROFIL` | moyen |
| **La fiche de sortie** | §3 : la structure que l'économie lira | petit, mais à décider tôt |
| **Stabilité des lunes et des habitats** | sphère de Hill : une lune ou une station hors de la sphère de son hôte n'a rien à y faire | petit |
| **Grand disque et pollution en système multiple** | dettes §2 | petit |
| **Habitat « complété » = site** | `concept_jeu.md` §5 : changement de modèle d'`Engin` | grand, **hors genèse** |

### 5.4 Le plan et la construction : ce que la performance y gagne

Ajouté le 2026-10-28, *« pour gagner des perf »*. G1 coupe la genèse en deux :
**`planifier(graine) -> PlanSysteme`** (données pures, aucun objet graphique,
sérialisable) puis **`instancier(&plan) -> Systeme`** (la seule partie qui
touche macroquad, sans aucun tirage). Le coût par image n'en bouge pas — c'est
le rôle d'[`opti_v2.md`](opti_v2.md) (D1 à D11) —, mais trois leviers s'ouvrent :

| Levier | Ce qu'il fait | Gain |
|---|---|---|
| **La qualité à la construction** | le plan dit *ce qui existe* (une ceinture de telle largeur, un grand disque, trois comètes) ; `instancier` décide *combien on en dessine* selon le réglage de qualité (Opti-v2 O5) : particules des ceintures (900 et 1 400, écrites en dur aujourd'hui, mêlées aux tirages), boules des nuages, couches du voile, résolution des terrains | **le plus fort pour l'APU** — et la même graine donne le même système sur toutes les machines |
| **La construction à la demande** | une lune lointaine, une comète hors champ restent une ligne du plan jusqu'à ce que la caméra s'en approche ; construites alors, libérées en s'éloignant | mémoire, temps de chargement, gros systèmes multiples |
| **Les plans lus sans construction** | la starmap planifie ses voisines (nombre de planètes, sites habitables, dangers) sans rien construire ; un plan se calcule hors du fil principal | starmap, génération sans à-coup |

**Règle à tenir dès G1** : aucun nombre de *rendu* (particules, boules,
couches, résolution) dans le plan, aucun tirage dans `instancier`. C'est ce
qui permet de baisser la qualité sans changer les systèmes — et de régler
O5 sans rouvrir la genèse.

## 6. Découpage proposé

> **Remplacé le 2026-11-05 par le §8**, après les décisions de G0. Gardé
> pour mémoire.

| Lot | Contenu | Taille | Dépend de |
|---|---|---|---|
| **G0** | Trancher §4 (échelle, température, catalogue, hasard, Soleil, rareté, garanties) | décisions | — |
| **G1** | Flux de hasard nommés + genèse en deux temps, plan puis construction (§5.4) : testable sans contexte graphique, et le point d'accroche de la qualité d'Opti-v2 O5 | M | G0 |
| **G1 bis** | Construction à la demande (lunes, comètes, terrains selon la distance caméra) | M | G1 |
| **G2** | Banc statistique (rapport sur N graines) — l'outil qui dira si on rate des trucs | M | G1 |
| **G3** | Un seul modèle de température ; habitabilité graduée (§5.2) | S | G0 |
| **G4** | Tirage pondéré dans le catalogue ; `planetes.json` maître | M | G1, G3 |
| **G5** | Plan de référence ; grand disque et pollution en multiple ; raretés en table | S–M | G1 |
| **G6** | Fiche de sortie par corps (vérité physique + étoiles exposées + Lagrange libres) | S | G4 |
| **G7** | Points de Lagrange (calcul fermé + mode de placement) | M | trajectoires B |
| **G8** | Étalonnage : le Soleil dans la distribution des G, réglage des deux JSON | M | G2, G4 |

Ordre de grandeur : **10 lots**, dont 5 moyens — comparable aux lots 9 à 15
d'Astres V2. Le gros risque n'est pas le volume de code, c'est **G0** et la
**fiche de sortie** : tant qu'elles ne sont pas fixées, chaque chantier suivant
(économie, stations, colonisation) rouvrira la genèse.

**Hors genèse, mais à ne pas oublier** : le lot E des trajectoires
(amarrage), l'habitat comme site, l'économie (tuiles, ressources, recherche),
Opti-v2.

## 7. G0 : les décisions, point par point

Ouvert le 2026-11-03 : avant de trancher §4, un tour complet de ce que la
genèse doit porter, en croisant `concept_jeu.md`, `ceintures.md`,
`trajectoires.md`, `starmap.md` et le code. Douze points, traités dans
l'ordre :

1. Les points de Lagrange, et l'échelle des systèmes
2. Les ceintures comme sites (minage)
3. L'orbite des géantes gazeuses (habitats, collecte de gaz)
4. La fiche de sortie (G8, §8) : ce que l'économie lira
5. Les lunes et les comètes comme sites
6. Les dangers (activité stellaire, impacts)
7. La cohérence visuelle (budget d'objets, marqueurs, taille des stations)
8. La starmap et le système de départ
9. Les systèmes multiples (fenêtres de stabilité)
10. Les noms, une fois le catalogue tiré
11. Les identifiants stables des corps (sauvegarde)
12. La comparaison Skymap V2 / V3 (deux genèses côte à côte, graines sauvées)

### 7.1 Lagrange et l'échelle — tranché le 2026-11-03

- **Couples** : étoile–planète et planète–lune.
- **Systèmes multiples** : les points d'un corps se calculent par rapport à
  son **parent dans l'arbre**, avec la masse totale de ce parent (une étoile,
  ou le centre de masse d'un couple pour une planète circumbinaire).
  Approximatif, cohérent avec « tous les points sont stables ». Aucun point
  entre deux étoiles.
- **Affichage** : un point n'apparaît que s'il porte une construction, ou en
  mode construction pour choisir l'emplacement.
- **Point saturé** (bloqué par l'encombrement) : facultatif, hors genèse.
  Porte ouverte : un champ « encombrement » dans la fiche (G8), le jour venu.
- **Troyens** : intéressants si peu coûteux. Pas de prototype ; le modèle de
  ceinture en est proche (deux arcs à ±60° tournant avec la planète : il
  manque une plage d'angle et une rotation en bloc). Prototype à la touche 4
  au moment du lot.
- **L'échelle : la vérité dans le plan, une échelle d'affichage réglable par
  système.** Le plan porte les distances et tailles vraies ; `instancier`
  applique des réglages (grossissement des corps, orbites des lunes liées,
  compression des distances au Soleil par une puissance), lus dans un JSON.
  Réduire toutes les distances du même facteur ne change rien à l'image
  (c'est un zoom) : seuls les **rapports** comptent. Ce réglage remplace les
  trois régimes actuels (corps ×5 des presets, distances vraies des presets,
  `√L` comprimé des systèmes générés).
- **Cohérence** : ce qui se place par rapport à un parent prend l'échelle des
  satellites de ce parent (L1/L2 avec les lunes ; L3–L5 et ceintures avec les
  distances au Soleil). Les calculs (température, Kepler, habitabilité)
  restent sur les valeurs vraies.
- **Conséquence sur §4.1** : la recommandation s'inverse — une luminosité
  **physique** pour les calculs, une de **rendu** pour l'écran. Coût par image
  nul (calculée une fois, à la genèse).
- **Grossissement selon le recul** : second mécanisme, facultatif ; à juger en
  comparant V2 / V3, les marqueurs suffisent peut-être.
- **À vérifier en G1** (contrôles, pas décisions) : précision des positions à
  30 UA hors affichage ; durée des trajets à l'échelle vraie ; un vaisseau
  vole dans le monde affiché avec des durées du monde vrai.

### 7.2 Les ceintures comme sites — tranché le 2026-11-04

- **Trois ressources au maximum, dans tout le jeu** : minerai, minerai rare,
  H2. Chaque site en porte des **proportions raisonnables** ; la zone les fait
  varier un peu, jamais beaucoup. Pas de composition détaillée.
- **Une station s'accroche à une ceinture** : (a) en orbite dans la ceinture,
  à l'endroit choisi (orbite circulaire au rayon de la ceinture, même vitesse
  que ses particules) ; (b) sur un corps — **planète naine ou lune, pas une
  comète** ; (c) sur les Troyens (§7.1).
- **Planètes naines** : 0–1 dans la ceinture principale, 0–2 dans Kuiper,
  400–1 200 km de rayon, posées **dans** leur ceinture sans gêner les orbites
  voisines. Une planète **ordinaire** (rendu des rocheuses/glacées) avec un
  **indicateur « naine »** ; habitabilité selon son environnement, comme
  toute planète.
- **Couleur des ceintures** : peut varier avec la zone, **pour la diversité
  visuelle seulement** ; elle ne décide pas des ressources.
- **Placement lié aux planètes** : la ceinture principale juste à l'intérieur
  de la première géante ; Kuiper au-delà de la dernière planète. **Sans
  géante**, la ceinture principale se pose à la **ligne des glaces**.
  Diversité gardée : une ceinture **supplémentaire** peut naître dans un grand
  écart entre deux planètes, d'où des systèmes à plus de deux ceintures.
  Aujourd'hui, elle est tirée à `2–3 × ech` sans regarder les planètes.
- **Systèmes multiples** : ont des ceintures, dans les fenêtres stables (§7.9).
- **Densité** : une valeur physique dans le plan (rendement, risque
  d'impacts) ; le nombre de particules dessinées en dépend, **et** du réglage
  de qualité.
- **Anneaux des géantes** : pas une source de ressource ; on peut y installer
  une **station de recherche**.
- **Piste hors décisions** : une « Gazeuses V3 » pour des proto-géantes en
  aspect (systèmes jeunes, ligne des glaces sans géante formée).

### 7.3 Autour des géantes — tranché le 2026-11-04

- **2 à 5 grandes lunes** par géante ; pas de petites lunes irrégulières (pas
  de prototype, budget d'objets). Aujourd'hui : 1 à 3.
- **Lunes espacées** : chacune assez près de sa géante pour lui rester liée,
  un écart minimal entre voisines. Corrige un défaut actuel (tirage entre la
  limite de Roche et 14 % de la distance à l'étoile, sans écart) ; c'est aussi
  ce qui permet de plafonner le grossissement d'affichage (§7.1) et de laisser
  des orbites libres aux stations.
- **Les lunes suivent les règles des planètes** : aspect par la température,
  tiré du catalogue (G6), habitabilité. Une lune prend la température de sa
  géante : une géante en zone tempérée peut porter une lune habitable
  (Pandora). Aujourd'hui : roche grise ou glace, à pile ou face.
- **H2** : géantes gazeuses, géantes de glace, naines brunes (ces dernières
  après une recherche).
- **Placement des stations et habitats** : règle de jeu, consignée dans
  [`concept_jeu.md`](concept_jeu.md) §5 (classes, bandes d'altitude, rayons
  distincts, capacités désactivables). La genèse ne garantit que l'espacement
  des lunes.
- **Ouvert** : une lune très proche de sa géante, volcanique par effet de
  marée (Io) quelle que soit sa température — proposé, pas tranché.

### 7.4 La fiche de chaque corps (G8) — tranché le 2026-11-04

**Une source par fait** : la fiche ne **stocke** que ce qui est tiré ; ce qui
se déduit se **calcule** au besoin.

| Bloc | Contenu | Stocké ou calculé |
|---|---|---|
| Identité | identifiant stable (§7.11), parent, catégorie, type, indicateur « naine », nom ou numéro | stocké |
| Physique | masse, rayon, orbite | stocké |
| | température d'équilibre, étoiles exposées | calculé |
| Aspect | modèle du catalogue + petite variation | stocké |
| Habitabilité | les 8 niveaux | calculé |
| Ressources | proportions minerai / minerai rare / H2 | stocké |
| Dangers | §7.6 | — |
| Places | points de Lagrange, bandes d'orbite libres | calculé |
| Encombrement | porte ouverte (§7.1) | plus tard |

- **Les ressources ne s'épuisent pas** (cette itération) : la genèse ne tire
  que des proportions. Une quantité de départ pourra s'ajouter plus tard,
  déduite de la taille et des proportions, sans rouvrir la genèse.
- **Proportions**, avec une petite variation tirée par site :

| Site | Minerai | Minerai rare | H2 |
|---|---|---|---|
| Planète, lune, naine rocheuses | dominant | faible | aucun |
| Ceinture intérieure (avant la ligne des glaces) | dominant | un peu plus qu'ailleurs | aucun |
| Corps glacés, ceinture extérieure, Kuiper | moyen | faible | moyen (glace) |
| Géantes gazeuses et de glace | aucun | aucun | dominant |
| Naines brunes | aucun | aucun | dominant (après recherche) |

- **Découverte** : la genèse fixe toutes les ressources dès la création ; le
  joueur les découvre en explorant. Le « connu / inconnu » appartient au jeu,
  pas à la genèse.

### 7.5 Lunes et comètes — tranché le 2026-11-04

- **Les lunes sont des sites** (§7.2, §7.3).
- **Les comètes sont du décor** : ni station, ni ressource, ni cible
  d'exploration.

### 7.6 Les dangers — tranché le 2026-11-04

- **Plus de points de vie sur les stations** : un **entretien** en ressources
  selon le type et la position ; un manque prolongé déclenche le comportement
  de la structure (arrêt, destruction, rien). Règle de jeu, consignée dans
  [`concept_jeu.md`](concept_jeu.md) §6 bis.
- **Les dangers majorent l'entretien** par un coefficient — contraignant, pas
  punitif. Ils n'infligent pas de dégâts.
- **Une éruption très puissante** peut toucher les colonies au sol (jeu
  émergent).
- **Ce que la genèse fournit** : la **source** de chaque danger — l'activité
  de l'étoile (déjà là : `activite_type`, et les éruptions sont simulées comme
  des événements dans `soleil/eruptions.rs`), la densité des ceintures.
  L'**exposition** d'un site se calcule depuis sa distance.
- **Chantier levé, hors genèse** : définir quelle étoile émet des éruptions
  ou des émissions violentes, à quelle fréquence, avec quelle puissance, et
  le coefficient d'entretien qui en découle.

### 7.7 La cohérence visuelle — tranché le 2026-11-04

- **Marqueurs par famille** : ▲ corps (planète, lune, naine), ◆ structure
  (station, habitat), et les **points de Lagrange** avec leur propre **logo
  16 × 16** (fourni plus tard ; un cercle en attendant). D'abord prévus
  « Lagrange1 à Lagrange5 », ils ne portent finalement **aucun nom**
  (§7.10).
- **Taille des stations** : leur vraie taille, le marqueur quand elles font
  moins de quelques pixels. **Mais** quand l'échelle d'affichage (§7.1)
  grossit les corps, les stations suivent le même réglage, comme tout ce qui
  orbite un parent.
- **Budget d'objets** : un système V3 ne coûte pas plus cher par image qu'un
  système V2 du même type d'étoile, **sur le petit PC**. S'il dépasse, G1 bis
  (construction à la demande) devient obligatoire. Un autre chantier d'opti
  viendra de toute façon.

### 7.8 La starmap et le départ — tranché le 2026-11-04

- **Départ** : le système solaire pour la **V1 de Star Settler** ; un départ
  procédural pour la **V1.1**. La genèse doit donc savoir produire un système
  de départ viable.
- **Garanties, minimales** : toujours du minerai (au moins une ceinture,
  §7.2). Pas de planète habitable garantie. Un site colonisable reste
  presque toujours possible sans contrainte de plus — une lune, une naine, ou
  un habitat à un point de Lagrange — mais **la garantie n'est pas poussée**.
- **Les étoiles proches de la starmap** : une **recherche documentaire
  étendue** sur ce qu'on sait de chacune, pour bâtir des systèmes **réalistes
  et intéressants** (comme Alpha Centauri, qui porte Pandora), puis les mettre
  en forme dans le format du plan. Travail à part entière (lot à ajouter au
  découpage).
- **Ce que la starmap montre** : l'étoile seule avant exploration, un résumé
  tiré du plan après. **La liste des informations affichées doit être facile
  à modifier** (ajouter, retirer).
- **Hors genèse** : le nombre d'étoiles de la carte et le système propre de
  Proxima appartiennent au chantier starmap.

### 7.9 Les systèmes multiples — tranché le 2026-11-04

- **Déjà fait** : fenêtres de stabilité de Holman-Wiegert (module
  `stabilite`), séparations log-uniformes (Raghavan 2010).
- **Fréquences gardées** (55 % simple, 28 % binaire, 11 % trinaire, 6 %
  quadruple), déplacées dans la table de rareté en JSON. Aujourd'hui elles
  dépendent de `seed % 100`.
- **L'âge du système, tiré en premier** : **jeune** (étoiles naissantes,
  disques, grand disque), **mûr**, **vieux** (géantes rouges, naines
  blanches, rémanents) ; les étoiles et le grand disque doivent lui être
  compatibles — aujourd'hui chaque étoile est tirée seule (une T Tauri peut
  avoir une naine blanche pour compagne). Vaut aussi pour les systèmes
  simples.
- **Une quatrième catégorie, les anomalies** : rares, elles enfreignent
  l'âge à dessein (une Herbig qui a capturé une naine blanche…). Liste de
  départ et rareté : §7.10, à la fin.
- **Séparations plafonnées** à quelques centaines d'UA dans un système ;
  au-delà, deux systèmes distincts sur la starmap (Proxima / Alpha Centauri).
  L'échelle d'affichage (§7.1) compresse le reste.
- **Aussi** : ceintures (§7.2), Lagrange par le parent (§7.1), grand disque
  et pollution étendus aux multiples (G5).

### 7.10 Les noms — tranché le 2026-11-05

- **Le nom du modèle tiré du catalogue reste caché** (« Fjord », « Mars »…) :
  le joueur ne voit que la désignation.
- **Un corps sans nom propre se nomme d'après son étoile**, par segments
  séparés de tirets : **Sirius-I** pour la première planète, **Sirius-I-A**
  pour sa première lune. Remplace la numérotation seule (I, II-1) de la
  décision du 2026-08-04 ; les noms propres (Terre, Lune) l'emportent
  toujours.

| Cas | Désignation |
|---|---|
| Planète, planète naine (numérotées ensemble, par distance) | Sirius-I, Sirius-IV |
| Lune | Sirius-I-A |
| Étoiles d'un multiple | Sirius-A, Sirius-B |
| Planète d'une des étoiles / du couple | Sirius-B-I / Sirius-AB-I |
| Ceinture | Sirius-Ceinture-I |
| Grecs (L4) / Troyens (L5) | Sirius-III-Grecs / Sirius-III-Troyens |
| Nom à plusieurs mots | Epsilon-Eridani-II |
| **Point de Lagrange** | **aucun nom** |

- **Systèmes procéduraux** : une **désignation de catalogue générée**, avec
  de la variation (plusieurs styles de préfixes), **10 caractères au plus**
  pour le nom du système.
- **Renommer** : le joueur peut renommer un système **sans nom propre** ; un
  système qui en a un (Sirius, Tau Ceti…) ne se renomme pas.

**Anomalies (§7.9)** — liste de départ validée, à enrichir avec le temps : capture
stellaire (âges incompatibles), planètes de pulsar, planète capturée (orbite
inclinée, rétrograde ou très excentrique), étoile en fuite (ni ceinture ni
comète), Jupiter chaud. Rares (~1–2 %, table de rareté), signalées dans la
fiche une fois découvertes. Extensible sans rouvrir la genèse.

### 7.11 Les identifiants stables — tranché le 2026-11-05

- **Une sauvegarde garde le plan entier** du système (quelques Ko), pas la
  seule graine. Un système découvert ne change plus jamais ; une mise à jour
  de la genèse ne touche que les nouveaux.
- **L'identifiant d'un corps est sa place dans le plan sauvegardé** : il ne
  bouge plus, et porte stations et points de Lagrange occupés.
- **Coût** : faible, G1 rend déjà le plan sérialisable (§5.4). Les flux nommés
  restent utiles (tests, systèmes pas encore découverts).
- Aujourd'hui `presets.json` ne garde qu'un nom et une graine (§7.12).
- **Précisé le 2026-11-05** : G1 rend le plan sérialisable, sans l'écrire sur
  disque. L'écriture viendra avec la **sauvegarde de partie**, qui n'existe pas
  encore ; la sauvegarde de systèmes reste hors du menu (retirée le 2026-10-19).

### 7.12 Comparer la Skymap V2 et la V3 — tranché le 2026-11-05

- **Deux genèses côte à côte** : la nouvelle dans un module à part,
  l'ancienne intacte. La **Skymap V3** est un nouvel écran copié de la V2.
  ~~Une fois validée, la V2 et l'ancienne genèse partent ensemble~~ :
  **révisé (§7.13)**, la V1 et la V2 restent, sur la genèse V1 gelée.
- **Les systèmes écrits à la main ne sont pas communs** : ils sont réécrits au
  format du plan pour la V3 (§7.1, §7.8). La référence commune est le
  **réglage d'échelle** : la V3 réglée pour imiter la V2, puis à l'échelle
  vraie.
- **Comparer** : à type d'étoile égal, à l'œil ; sur des milliers de graines
  avec le banc (G2) ; au coût par image sur le petit PC (§7.7).
- **G1 ne se voit pas** : validé par les tests.
- **IKDAR et ZATURA** (`presets.json`, graine seule) : **on accepte qu'ils
  changent**. Certains systèmes de la starmap évolueront en aspect de toute
  façon (§7.8).
- **Les Skymaps V1 et V2 sont gardées**, sur la genèse V1 gelée (§7.13).

### 7.13 Les clarifications de la relecture — tranché le 2026-11-05

Les cinq points du §8.2.

**1. Un seul modèle de température, qui cohabite avec le type.**
Aujourd'hui l'habitabilité se calcule en trois temps : un **type de base**
lu sur la planète (`planetes.json`, sinon son aspect : les 8 niveaux), des
**crans** selon le type d'étoile (UV, éruptions : `habitabilite.json`), et
une **zone** binaire (−1 cran hors de 260–320 K). La genèse, elle, choisit
l'aspect avec d'autres seuils (tempérée 255–350 K) : d'où des planètes
« habitables mais trop chaudes ». Désormais **une seule température** —
d'équilibre, sommée sur toutes les étoiles, avec la luminosité physique
(G3) — décide de tout :

1. elle **filtre le catalogue** : chaque modèle de `planetes.json` reçoit sa
   plage de température, la genèse ne tire que des modèles compatibles ;
2. le **type de base** reste lu sur le modèle (les 8 niveaux, inchangés,
   `planetes.json` maître) ;
3. les **crans de l'étoile** restent tels quels ;
4. la **zone binaire** est remplacée par les **bandes** du §5.2 (crans et
   plafonds par température), dans `habitabilite.json`.

Le type dit *ce qu'est* la planète, la température *ce qu'elle endure*. Comme
l'aspect est tiré compatible avec la température, ils ne se contredisent
plus. La galerie, sans étoile, ne garde que le type de base.

**2. Deux étalons.** Le **système solaire** (le plus observé) dans la
distribution des étoiles G ; **Alpha Centauri** pour les systèmes doubles.
(G11)

**3. Les valeurs physiques des étoiles** viennent des **tables
d'astrophysique**. Le réglage d'échelle d'affichage reste une **couche
parallèle** du rendu. L'éclairement : une étoile peut ne pas éclairer
pleinement une planète, **mais les teintes doivent ressortir** — une
planète colorée reste lisiblement colorée, quelle que soit son étoile. (G3)

**4. Grecs et Troyens, autour des géantes gazeuses seulement**, si le coût
par image le permet (critère du §7.7). Les deux camps : **Grecs en L4,
Troyens en L5** (convention de Jupiter). **Deux variantes**, tirées par
géante : des **débris** qui apparaissent quand on zoome, ou seulement
quelques **gros corps**. (G9)

**5. La Skymap V1 est gelée**, pas retirée. L'accueil gagne une **catégorie
SKYMAP** à gauche (V1, V2, V3), et la V1 garde son bouton pour comparer
les évolutions du moteur. (G1)

**6. La genèse V1 est gelée, la V1 et la V2 sont gardées.** Les deux
skymaps tournent sur l'ancienne genèse, désormais appelée **genèse V1** :
elle reste en code figé (plus aucune règle nouvelle, seulement de quoi
compiler quand le reste bouge) au lieu d'être retirée en G11. La V3 tourne
seule sur la genèse V2. Coût assumé : ~4 000 lignes qui doivent suivre les
changements d'API (`Systeme`, `Planete`…) et dont les tests restent.

### 7.14 L'échelle, précisions de G3 — 2026-11-06

- **La taille des étoiles à l'écran** (tranché) : une compression par une puissance, le
  Soleil gardant sa taille du système solaire. Les **objets compacts** (trou noir,
  pulsar, magnétar, étoile à neutrons) gardent leur taille de rendu actuelle : leur image
  est leur disque et leurs jets, proportionnels au rayon.
- **Le grossissement des planètes** (tranché) : **×5 pour commencer**, comme le système
  solaire, réglable dans le JSON. Une Terre générée passe de 0,6 à 0,01 unité. Le
  grossissement croissant avec le recul reste le second mécanisme facultatif du §7.1, à
  juger à l'écran.
- **La luminosité de rendu ne change pas** : l'éclairage ne dépend pas de la distance
  (`couleur × luminosité`), les teintes ressortent comme aujourd'hui.
- **Abandon d'un point du §7.12** : la « V3 réglée pour imiter la V2 » ne tient plus, les
  températures et les distances changeant réellement.
- **Ouvert, Q1 : où vont les planètes d'une étoile très brillante ?** Avec `√L` vraie,
  Kuiper partirait à 68 000 unités pour une B, 675 000 pour une O (caméra : 30 000).
  Recommandé : un **plafond** de l'étalement (~5, Kuiper vers 220 UA), la température
  restant vraie — une O n'a que des mondes brûlés. Sinon : étalement vrai, très compressé.
- **Ouvert, pour G7** : Kuiper n'est pas obligatoire. Le §7.8 ne garantit qu'**une ceinture
  au moins, de n'importe quel genre** ; G3 garde la règle de la V1, G7 décidera.

## 8. Découpage révisé (2026-11-05)

Tiré du §7. Le §6 prévoyait 10 lots ; les décisions en ajoutent (l'échelle,
les ceintures, les étoiles proches) et en regroupent. Chaque lot se voit dans
la **Skymap V3**, à côté de la V2, sauf G1.

| Lot | Contenu | Sections | Taille | Dépend de |
|---|---|---|---|---|
| **G0** ✅ | Les décisions | §7 | — | — |
| **G1** | Module à part ; flux de hasard nommés ; `planifier` → plan sérialisable, `instancier` sans tirage ; identifiants = place dans le plan ; plan sérialisable ; écran Skymap V3 (copie de la V2) et catégorie SKYMAP de l'accueil. Reprend d'abord les règles actuelles : **rien ne change à l'écran** | §5.4, §7.11, §7.12 | L | G0 |
| **G2** | Banc statistique sur N graines, des deux genèses ; coût par image V2 / V3 sur le petit PC ; outils de mesure rebranchés dans la V3 | §5.3, §7.7, §7.12 | M | G1 |
| **G3** | **L'échelle** : vérité physique dans le plan (luminosité physique par type d'étoile, tailles et distances vraies), luminosité de rendu à part ; réglage d'affichage en JSON (grossissement, orbites des lunes liées, compression des distances) ; contrôles de précision et de durée des trajets | §7.1 | L | G1 |
| **G4** | Un seul modèle de température ; habitabilité graduée par bandes | §5.2 | S | G3 |
| **G5** | **Les étoiles** : âge du système tiré en premier, types compatibles, anomalies ; table de rareté en JSON (fréquences, types) ; séparations plafonnées ; plan de référence ; grand disque et pollution en multiple | §7.9, §7.10 | M | G1 |
| **G6** | **Planètes et lunes** : tirage pondéré dans le catalogue (lunes comprises), 2–5 lunes espacées par géante, planètes naines avec leur indicateur ; désignations (Sirius-I-A), noms de catalogue des systèmes | §7.2, §7.3, §7.10 | M | G3, G4 |
| **G7** | **Ceintures** : placées d'après les planètes (ligne des glaces sans géante), ceintures supplémentaires, ceintures en multiple, densité, couleur par zone ; anneaux marqués « station de recherche » | §7.2 | M | G3, G6 |
| **G8** | **La fiche** : stocké / calculé, proportions des trois ressources, sources de danger ; résumé pour la starmap, configurable | §7.4, §7.6, §7.8 | S–M | G6, G7 |
| **G9** | **Lagrange, Grecs et Troyens** : points calculés par le parent dans l'arbre, mode de placement, marqueur (logo à venir) ; essaims des géantes en deux variantes, prototype à la touche 4 | §7.1, §7.7, §7.13 | M | G3, G6 |
| **G10** | **Les étoiles proches** : recherche documentaire, puis systèmes réécrits au format du plan (Soleil, Tau Ceti, Alpha Centauri et Pandora, Proxima, les six de la starmap) | §7.8 | M–L | G8 |
| **G11** | **Étalonnage et bascule** : le Soleil dans la distribution des G, réglage des JSON, critère de coût ; puis la V3 devient la vue principale ; V1, V2 et la genèse V1 restent, gelées | §4.5, §7.7, §7.12 | M | tous |
| *G1 bis* | Construction à la demande — **obligatoire** si la V3 dépasse le coût de la V2 | §5.4, §7.7 | M | G2 |

Ordre conseillé : G1, G2, G3, G4, G5, G6, G7, G8, G9, G10, G11 (G5 peut
passer avant G3 : il ne dépend que de G1).

**Hors genèse, levés par G0** : le chantier des éruptions et de l'entretien
(§7.6, `concept_jeu.md` §6 bis) ; les règles de pose des structures et leur
table de capacités (`concept_jeu.md` §5) ; le marqueur ◆ des structures ;
une « Gazeuses V3 » (proto-géantes) ; l'étendue de la starmap ; le sort des
PV des vaisseaux ; le lot E des trajectoires.

### 8.1 Ce qui grossit par rapport au §6

- **L'échelle** devient un lot entier (G3), le plus risqué après G1 : il
  touche les 29 types d'étoile (une luminosité physique à poser pour chacun)
  et tous les systèmes écrits à la main.
- **Deux lots nouveaux** : les ceintures (G7) et les étoiles proches (G10).
- **G1** porte en plus la sauvegarde du plan et l'écran Skymap V3.

### 8.2 À clarifier, repéré à la relecture

> **Résolus le 2026-11-05 → §7.13**, avec le point qu'ils ont fait
> apparaître (§7.13, point 6). G0 est clos.

1. **Un seul modèle de température (§4.2)** : jamais confirmé explicitement,
   alors que §7.3 et §7.4 s'appuient dessus. Il remplacerait aussi la
   classification par l'aspect (les 8 niveaux du chantier planètes) par les
   bandes du §5.2, ou les ferait cohabiter.
2. **Le Soleil comme étalon (§4.5)** : non abordé. G11 en dépend.
3. **Les valeurs physiques des étoiles** : d'où viennent les luminosités et
   rayons vrais des 29 types (tables d'astrophysique), et que devient le
   rendu actuel, réglé sur les valeurs comprimées ?
4. **Les Troyens** : seulement pour les géantes, ou aussi les planètes
   rocheuses (la Terre et Mars en ont quelques-uns) ?
5. **La Skymap V1** : la retirer, ou la geler (§7.12).
