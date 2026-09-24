# Recherche visuelle — taches, arches, éruptions (chantier astres v2)

> Cadrage, **2026-09-24**. Pas de code, pas de commit. Répond à une plainte
> précise de l'utilisateur : trop de boucles d'un coup (« un croissant, pas des
> boucles »), et des taches en **autocollants** plutôt qu'en structures qui
> émergent. Jugé « vital » pour la v2. Ce document fixe la **cible** et les
> critères qu'on pourra confronter à l'écran — pas comment coder.
>
> S'appuie sur `docs/conception/astres_v2.md` (chiffrage pixels) et
> `docs/suivi/etoiles.md` (chantier P.3, échelle et horloge). Je ne
> réenquête pas ces chiffres, je les réutilise.

---

## 1. Anatomie d'une région active — ce qui se voit, et dans quel spectre

Une région active solaire complète, en lumière blanche **et** en H-alpha /
UV, empile ceci (du plus profond au plus haut) :

- **Pores** : mini-taches sans pénombre, quelques centaines à ~2 500 km.
  Premiers signes visibles d'une région qui émerge.
- **Tache (umbra + penumbra)** : cœur sombre (umbra, champ quasi vertical) +
  halo strié plus clair (penumbra, champ qui s'incline vers l'extérieur).
  C'est la seule structure de ce document **entièrement visible en lumière
  blanche** — c'est ce que peint déjà notre shader.
- **Plage** : zone légèrement plus brillante que la photosphère calme,
  entourant le groupe. Visible en lumière blanche mais discrète (contraste
  faible) ; beaucoup plus nette en Ca II K ou H-alpha.
- **Ligne neutre (polarity inversion line)** : la frontière entre polarité
  + et − du groupe bipolaire. **Invisible en tant que telle** en lumière
  blanche — c'est une donnée magnétique, pas une texture. Ce qui la rend
  lisible indirectement : les arches et les filaments s'alignent le long
  d'elle.
- **Filaments / fibrilles superpénombrales** : structures chromosphériques
  fines qui rayonnent depuis la tache. **Invisibles en lumière blanche**,
  ce sont des traceurs H-alpha.
- **Arcade de boucles coronales** : le grand système d'arches qui relie les
  deux polarités du groupe. **Invisible en lumière blanche** — les boucles
  qu'on voit sur les photos spectaculaires (SDO/AIA 171 Å, 193 Å) sont de
  l'**extrême ultraviolet**, un canal que le Soleil n'émet quasiment pas en
  visible. Une étoile de type solaire observée à l'œil nu depuis l'espace ne
  montre **pas** ces arches dorées : elles existent, mais dans un autre
  spectre.

**Le point capital, à répéter parce qu'il change la conception** : la moitié
des images « boucles de plasma » qui inspirent spontanément (SDO en UV faux
couleur doré/orange) ne sont **pas** ce que verrait un œil humain regardant le
Soleil en lumière visible. Un flare en lumière blanche existe (rare, bref,
un point plus brillant que la photosphère), mais l'arcade en fil d'or qui fait
« ça » sur les vidéos NASA est une reconstruction UV.

**Conséquence pour le jeu.** Deux postures honnêtes, à choisir consciemment
(le dépôt a déjà pris ce genre de parti ailleurs — voir couronne stylisée
dans `etoiles.md`) :

1. **Assumer un rendu « composite »** façon documentaire spatial : on montre
   les boucles comme si l'œil voyait l'UV, exactement ce que fait déjà notre
   shader (billboards additifs dorés). C'est un choix de lisibilité, pas
   d'exactitude — et il faut alors que le reste (couleur, intensité) reste
   cohérent avec ce parti pris plutôt que de prétendre à du blanc réaliste.
2. Ou limiter les arches au strict flare/CME (bref, localisé, hors disque),
   et laisser le disque en lumière blanche pure pour le reste.

Le jeu a déjà tranché implicitement pour (1) — couronne, spicules, boucles
dorées sont un habillage stylisé assumé, pas un rendu spectral vrai. Ce
document ne recommande pas d'en sortir : il recommande de le rendre
**cohérent avec la géométrie magnétique** même en restant stylisé (voir §3).

---

## 2. Échelles et proportions — des chiffres

| Structure | Taille / hauteur typique | Rapport au rayon solaire (R☉ ≈ 696 000 km) | Durée de vie |
|---|---|---|---|
| Pore | quelques centaines de km – 2 500 km | ~0,0004 R☉ | heures |
| Tache moyenne | ~6 000–20 000 km de diamètre | 0,004–0,015 R☉ | jours à ~2 semaines (50 % des groupes < 2 j) |
| Grande tache / groupe | jusqu'à ~60 000 km | ~0,04 R☉ (≈ 4 % du rayon) | jusqu'à ~100 j pour les plus grands groupes |
| Boucle coronale « active region » | dizaines de milliers de km, souvent < 0,1–0,3 R☉ de hauteur | 0,05–0,3 R☉ | dizaines de minutes (10–300 min typique en 193 Å) |
| Protubérance / filament éruptif | dizaines à centaines de milliers de km, parfois > 100 000 km | 0,1 à plusieurs R☉ | minutes à heures (éruptive) ; jours (quiescente) |
| CME | atteint 3–6 R☉ dans le champ LASCO C2, jusqu'à ~30 R☉ en C3 | plusieurs R☉ à quelques dizaines de R☉ | minutes à formation, des heures à traverser le champ proche |
| Vitesse de CME | 200–3 000 km/s (typique ~500 km/s) | — | — |

Ce que ce tableau confirme, en le recoupant avec `astres_v2.md` §1 (qui donne
déjà les tailles en pixels du jeu à `R_SOLEIL = 1,116`, `PX_MIN = 4`) : une
**tache** est un détail petit et net (quelques % du rayon), une **arche
individuelle** est comparable ou plus petite qu'une tache, et c'est la **CME**
qui est, réellement, l'objet le plus grand du lot — cohérent avec le constat
du dépôt qu'elle atteint déjà 5-6 rayons stellaires dans le code actuel.
Le rapport taille/importance visuelle est donc **inversé** par rapport à
l'intuition « plus il y a de boucles, plus ça a l'air actif » : une seule CME
bien dessinée porte plus loin qu'une dizaine de petites arches empilées.

---

## 3. Ce qui fait qu'une image « fait vrai »

Recoupé sur les images SDO/AIA et la littérature de reconnexion magnétique :

- **Les arches d'un même groupe partagent une orientation.** Elles ne
  rayonnent pas dans huit directions aléatoires depuis un point : elles
  relient deux pôles et s'alignent **perpendiculairement à la ligne neutre**,
  formant une nappe cohérente (« arcade »), pas un bouquet radial. C'est très
  exactement le défaut nommé dans `astres_v2.md` §1.2 : `declencher_boucle`
  tire une direction aléatoire au lieu d'ancrer sur les taches.
- **Les pieds sont visiblement ancrés.** Une arche part d'un point sombre
  (la tache ou sa pénombre), pas du vide à côté. C'est ce qui distingue
  « structure qui émerge de la surface » d'« autocollant » : le pied doit
  toucher/déformer visuellement la tache, pas flotter à une distance
  arbitraire.
- **L'épaisseur et l'éclat varient le long du tube**, plus brillants et fins
  près des pieds (plasma dense, champ fort), plus larges et diffus au
  sommet. Une arche à épaisseur constante lit comme un ruban plaqué, pas
  comme un tube de plasma contraint magnétiquement.
- **Profil vs face.** Une boucle vue de profil (limbe) montre un arc net,
  fermé ; vue de face (disque), elle s'aplatit en un filament sombre ou
  clair selon le fond — la 3D compte. Un billboard toujours face caméra rate
  cette différence ; c'est un choix déjà connu comme limite de ce type de
  rendu, à assumer plutôt qu'à ignorer.
- **Les taches viennent par groupes inégaux**, jamais seules : un leader
  plus gros et plus stable, un ou plusieurs suiveurs plus petits, parfois
  des pores épars autour. Une tache unique, ronde, isolée, ne ressemble à
  rien d'observé — c'est probablement la racine du reproche « autocollant » :
  8 taches indépendantes tirées séparément (comme le fait `spots[8]`
  aujourd'hui selon `etoiles.md`) sans relation de groupe ni d'inégalité de
  taille lisent comme des stickers posés un par un.
- **La forme de la tache n'est pas un disque parfait.** Umbra irrégulière,
  pénombre fibreuse (stries radiales, pas un dégradé lisse) — un cercle à
  bord net est justement la signature « autocollant ». Le commit récent
  (« des taches qui ne sont plus rondes », log `57be998`) va déjà dans ce
  sens ; à pousser vers une **irrégularité corrélée entre taches d'un même
  groupe** plutôt qu'indépendante tache par tache.

---

## 4. Hiérarchie d'importance par distance d'observation

Le jeu observe une étoile à des distances très variables ; `astres_v2.md`
donne déjà les seuils en unités/pixels. Je les reprends et les classe par
rapport gain/complexité :

| Distance (cadrage) | Rayon apparent | Ce qui commence à compter | Rapport gain/complexité |
|---|---|---|---|
| Défaut système (1280 u) | 4 px (plancher) | Disque, teinte, un léger assombrissement du limbe ; **rien de structurel** | — (aucun budget de détail à dépenser ici) |
| ~3 UA (145 u) | 10 px | Taches lisibles comme marbrures (pas encore comme forme) | Faible — investir seulement si le style le permet à 2-3 px de taille |
| ~1,5 UA (72 u), ~0,98 UA à 700 px | 20 px | Une **arche isolée** atteint tout juste le pixel | Moyen — c'est le seuil où *une* arche bien ancrée vaut mieux que huit dispersées |
| ~0,5 UA (24 u), ~0,33 UA à 700 px | 60 px | Arche **lisible comme arche** (courbe, épaisseur variable) | Élevé — c'est ici que l'alignement sur la ligne neutre, l'ancrage visible et le gradient d'éclat le long du tube deviennent perceptibles et donc rentables |
| Approche rapprochée (< 20 u) | > 200 px | Groupe de taches, umbra/pénombre distinctes, filaments, CME en plein développement | Maximal — tout le vocabulaire de la §3 se lit |

**Ce qui ne vaut rien à ces distances, et qu'il faut abandonner sans
regret** : granulation fine, fibrilles superpénombrales, distinction umbra/
pénombre en dessous de ~10 px de rayon stellaire, toute texture de tache plus
riche qu'une tache sombre grossière avant ~60 px. Le dépôt a déjà cette dette
consignée (`etoiles.md`, chantier P.3) — ce document ne fait que confirmer
qu'elle reste vraie pour les taches/arches spécifiquement.

**Seul objet qui porte à *toutes* les distances, y compris au cadrage par
défaut à 4 px : la CME**, précisément parce qu'elle sort du disque (elle
n'est pas bornée par le rayon stellaire comme les boucles) et parce qu'une
**modulation de luminosité globale** (un flare qui fait clignoter l'éclat de
l'étoile un instant) est un signal qui survit à n'importe quelle échelle —
c'est un changement de couleur/intensité d'un pixel, pas une forme à
résoudre. `astres_v2.md` §1 le note déjà : « le seul canal qui échappe au
problème ».

---

## 5. Références visuelles nommées

- **NASA SDO — Ultra HD Gallery** (https://sdo.gsfc.nasa.gov/gallery/ultrahd/)
  et la vidéo *NASA's Solar Dynamics Observatory: Year 7 Ultra HD (4K)*
  (https://www.youtube.com/watch?v=kJPz-oRnRDE) : référence pour
  **l'alignement des arcades sur la ligne neutre** et le gradient
  d'épaisseur/éclat le long d'un tube. À prendre : la cohérence directionnelle
  d'un groupe d'arches, pas la couleur dorée saturée (déjà présente dans le
  jeu) ni la densité (le jeu doit en montrer beaucoup moins à la fois — c'est
  la plainte de départ).
- **SDO — « Solar Active Region Sports Huge Coronal Loops »**
  (https://www.youtube.com/shorts/lEgeCv2QxrU) : bon exemple compact d'une
  **arcade unique et cohérente** plutôt qu'un feu d'artifice de boucles — la
  direction opposée du défaut actuel (« impression de croissant »).
- **SOHO/LASCO — Best Of SOHO**
  (https://soho.nascom.nasa.gov/gallery/SolarCorona/las016.html) et la galerie
  courante (https://soho.nascom.nasa.gov/gallery/images/triplec2.html) : la
  référence pour la **CME** — un front diffus qui s'élargit et s'éclaircit peu
  en s'éloignant, pas un jet de particules ponctuelles. À prendre : la forme
  en « chou-fleur »/bulle qui se détache proprement du limbe.
- **DayStar Filters — Sunspots** (https://www.daystarfilters.com/sunspots/) :
  bonnes photos rapprochées umbra/pénombre montrant la **fibrillation
  radiale** de la pénombre et l'irrégularité du bord — à prendre pour casser
  le contour de disque parfait une fois qu'on dépasse ~60 px.
- **NOAA SWPC — Sunspots/Solar Cycle** (https://www.spaceweather.gov/phenomena/sunspotssolar-cycle)
  : bonne référence pour la **taille relative et le groupement inégal** des
  taches d'une même région (leader plus gros, suiveurs plus petits, pores
  épars).
- **`github.com/bradleyJT-CS/sun-shader`** : exemple concret de soleil
  procédural en GLSL (disque + couronne + flares en coordonnées polaires) —
  pertinent comme référence de **structure de shader**, pas de fidélité
  scientifique ; à regarder pour la technique d'assemblage plus que pour le
  résultat visuel.

Rien trouvé de convaincant côté jeux commerciaux (No Man's Sky, Universe
Sandbox) : leurs étoiles restent des sources de lumière avec un effet de
flare générique. Aucune recommandation à en tirer — je le dis franchement
plutôt que d'inventer une référence.

---

## 6. Critères d'acceptation visuels

À confronter directement à l'écran, dans l'ordre d'importance :

1. **Une tache n'apparaît jamais seule** : elle vient avec au moins un
   compagnon de taille différente (groupe leader/suiveur), même petit.
2. **Les arches d'un même groupe partagent une direction commune** — elles
   forment une nappe alignée sur l'axe du groupe, pas un bouquet radial
   partant d'un point.
3. **Le pied d'une arche touche visiblement une tache** (ou sa pénombre),
   jamais un point du disque sans tache.
4. **Une arche est plus fine et plus brillante près des pieds, plus large et
   plus diffuse au sommet** — pas une épaisseur constante.
5. **Moins de boucles simultanées, mieux ancrées** : au cadrage où une arche
   devient lisible (~60 px de rayon stellaire), on voit une arcade (2-4
   arches cohérentes) plutôt qu'un buisson de 8.
6. **Le contour d'une tache n'est pas un cercle net** : bord irrégulier,
   umbra plus sombre que la pénombre, distinction visible dès qu'elle
   dépasse quelques pixels de large.
7. **La CME reste l'élément le plus lisible à toutes les distances**, parce
   qu'elle sort du disque — elle ne doit pas être sacrifiée au profit du
   détail des arches, qui elles restent une récompense d'approche.
8. **Un flare se traduit par une variation d'éclat localisée**, visible même
   quand l'étoile ne fait que quelques pixels — pas seulement par une forme
   qui de toute façon ne passera pas le pixel à cette distance.
9. **En dessous du seuil où une structure ne peut pas se lire** (granulation
   fine, fibrilles, umbra/pénombre sous ~10 px), rien n'est dessiné à ce
   niveau de détail — pas de complexité qui ne rapporte aucun pixel
   supplémentaire.

---

## Ce qui reste incertain

Je n'ai pas de réponse ferme sur **combien** d'arches par groupe donne
l'impression d'« arcade » plutôt que de « buisson » ou de « croissant vide » —
les références NASA montrent des dizaines de boucles fines simultanées,
impossible à transposer telles quelles vu le budget de billboards actuel. Un
nombre de l'ordre de 2 à 5 arches cohérentes, de tailles différentes, semble
un compromis raisonnable mais reste un choix esthétique, pas une mesure. De
même, je n'ai pas de source qui chiffre à quel point la pénombre doit rester
« striée » avant de redevenir illisible en dessous de 60 px — c'est un
jugement à faire à l'écran, comme le rappelle `etoiles.md` pour le reste du
chantier P.3.
