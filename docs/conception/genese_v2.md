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
| **Performance** (Opti-v2) | un budget d'objets par système (disques, comètes, lunes) | lots O1–O8 à faire |

**La conséquence** : la genèse V2 ne peut pas attendre l'économie, et ne doit
pas la deviner. Elle doit produire **la vérité physique** de chaque corps
(type, masse, distance, température, preset, habitabilité, étoiles
exposées) et laisser **une étape « ressources » branchable** plus tard, qui ne
lira que cette vérité. Si la fiche de sortie est fixée maintenant, l'économie
viendra s'y brancher sans rouvrir la genèse.

## 4. Les décisions à trancher avant de coder

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
