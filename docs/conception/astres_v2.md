# Conception — Astres V2 : le chantier intermédiaire

> Cadrage, **2026-09-24**, écrit **avant tout code** et n'en produisant aucun.
> Quatre sujets demandés ensemble : **flares et CME**, **variété des textures**,
> **astres exotiques**, **trous noirs** (sous-chantier **ceintures de débris**).
> Il applique la règle apprise à ses dépens ([`../methode.md`](../methode.md),
> chantier P.3 de [`../suivi/etoiles.md`](../suivi/etoiles.md)) : **constater
> d'abord.** Un chantier « éruptions » a déjà été ouvert sur le malentendu
> inverse — le code existait, c'étaient l'horloge et l'échelle qui ne suivaient
> pas. Chaque section part de ce qui existe, `fichier:ligne` à l'appui.

---

## 0. L'unité de mesure de tout ce document

Tout se juge en **pixels de rayon apparent** : seule chose qui décide qu'un
effet existe. `impostor::px_rayon` (`src/impostor.rs:96`) donne
`px = r/d × h × 1,2` ; avec `R_SOLEIL = 1,116` et h = 1 080 (§ P.3) :

| cadrage | rayon étoile | boucle (`r × 0,05`) | cœur d'éjection (`r × 0,16`) | **étendue d'une CME** (≈ 6 r) |
|---|---|---|---|---|
| 1 280 (défaut) | **4,0** (plancher `PX_MIN`) | 0,20 | 0,64 | **24 px** |
| 145 (≈ 3 UA) — taches lisibles | 10,0 | 0,50 | 1,60 | 60 px |
| 72 (≈ 1,5 UA) — boucle au pixel | 20,0 | 1,00 | 3,20 | 120 px |
| 24 (≈ 0,5 UA) — boucle lisible | 60,0 | 3,00 | 9,60 | 360 px |

⚠️ La fenêtre de jeu fait **700 px** (`main.rs:43`) : diviser par 1,54 — les
seuils de 20 et 60 px tombent à **0,98** et **0,33 UA**.

## 1. Flares et éjections de masse coronale

**Ce qui existe.** Taches à cycle de vie complet (`soleil/eruptions.rs:15`)
poussées en `spots[8]` (`rendu.rs:27`) ; boucles en Bézier à deux pieds ancrés,
machine `Montée→Tenue→Rupture→Retombée` (`eruptions.rs:26`, l. 115-146) ;
**éjection (CME)** — l'état `Rupture` (l. 131-138) est une corde de flux qui
**accélère** (`apex_vel += 1,2·dt`) et s'estompe en 2,2 s. Rendu : 49 billboards
additifs par boucle + un « cœur » à `r × 0,16` (`soleil/rendu.rs:79-122`) ;
granulation `fbm` + domain warping (`soleil.frag.glsl:102-132`). **La CME est
déjà grande** : aux réglages par défaut son sommet atteint **5 à 6 rayons
stellaires**, soit **24 px au cadrage par défaut**.

**Ce qui manque vraiment — trois choses, une seule de rendu :**

1. **Le sursaut lumineux localisé n'existe pas.** `spots[8]` ne sait
   qu'**assombrir** (`soleil.frag.glsl:128`) : rien ne rend une région plus
   brillante. C'est le flare, et il est absent.
2. **Les boucles ne sont pas ancrées aux taches.** `declencher_boucle`
   (`eruptions.rs:40`) tire `dir_aleatoire()` alors que son appelant annonce
   « Déclenchement des boucles (sur les taches) » (l. 105). Or un flare se lit
   *parce que* le ruban relie deux pieds d'une tache.
3. **Le taux d'activité est figé, et commun à toutes les étoiles.**
   ⚠️ *Corrigé après vérification : une première rédaction affirmait que
   `Systeme::reglages_etoile` (`systeme/mod.rs:574`) n'avait aucun appelant.
   C'est faux — `skymap.rs:207` et `skymap_v2.rs:196` l'appellent à chaque
   frame.* Le vrai défaut est dans ses **arguments** : les deux vues passent
   des littéraux, `(0.5, 0.0, 0.10, 0.0)`, sous le commentaire « Réglages
   d'éruptions (fixes pour l'instant) ». La méthode pousse donc la même
   valeur à *toutes* les étoiles du système — naine M active et géante rouge
   éruptent pareil, et rien ne peut les distinguer.

**À quelle échelle — et la surprise.** P.3 a tranché que les boucles étaient une
récompense d'approche parce qu'elles sont dessinées à `r × 0,05`. **Exact pour
la boucle, trompeur pour l'éjection** : ce qui tombe sous le pixel n'est pas le
sujet mais le **pinceau** — une CME de 24 px peinte avec 49 points de 0,20 px.
Le remède est **déjà écrit ailleurs** : `disque_particules.vert.glsl:146` borne
la taille écran d'une particule à `px_min` **et compense par un fondu d'alpha**
(l. 149) ; les billboards de plasma n'en profitent pas (`rendu.rs:79`).

**Ce qui se verra** : au cadrage par défaut, la CME devient une traînée qui se
détache de l'étoile — seul détail stellaire à franchir ce cadrage, parce qu'il
est **hors** du disque. **Le flare se verra deux crans de zoom plus loin que la
boucle**, se dessinant à l'échelle d'une tache (10 px, soit 3 UA) et non à
`r × 0,05` (20 px). Et **une modulation d'éclat pendant le sursaut** se voit
**à n'importe quelle échelle**, même à 4 px : luminosité, pas forme — le seul
canal qui échappe au problème. Les arches restent la récompense de l'approche.

## 2. Variété des textures d'astres

**Tranché : oui, toutes les étoiles partagent le même motif.** Aucun uniform de
graine n'atteint le shader stellaire — `uniforms_soleil()`
(`soleil/materiau.rs:254-265`) déclare `teinte`, `couronne`, `couronne_irreg`,
`couronne_type`, `spots`, `zone`, rien d'autre — et la granulation
(`soleil.frag.glsl:105`) vaut `fbm(d × 11 + warp × 1,2 + …)`, **déterministe de
la direction dans le repère propre de l'étoile**. Pire : le `time` qui pourrait
les décaler est `self.temps` (`soleil/mod.rs:82`), initialisé à **0,0 pour
chacune**. Deux étoiles d'un binaire sont, à la teinte près, **image pour image
identiques**. Diffèrent déjà : teinte, rayon, luminosité, **taches**.

**Les planètes, elles, ont une graine** : `Apparence::seed`
(`planete/apparence.rs:54` — « décalage de bruit → chaque géante est unique »)
passe en uniform (`planete/materiau.rs:54`) et pilote l'atlas de terrain
(`planete/terrain.rs:754`) comme quinze sites du fragment shader
(`planete.frag.glsl:194, 263, 632…`) ; les disques ont la leur
(`disque/config.rs:29`). **L'étoile est le seul corps du jeu qui n'en a pas.**

**Un second trou, du même genre : `apparence_glacee` ne pose jamais de graine.**
`apparence_tellurique` (`genese/apparences.rs:165`) et `apparence_gazeuse`
(l. 237) font `app.seed = gen_range(0.0, 1000.0)` ; la version glacée
(l. 315-333) sort sans y toucher, donc avec le défaut `seed: 0.0` — **toutes les
lunes et planètes glacées générées** (`genese/mod.rs:111` et `:358`) ont la même
géographie. Seul le catalogue nommé y échappe, et c'est lui seul que garde le
test « géographie en double » (`mod.rs:1219`).

**Ce qui se verra** : la graine d'étoile, partout où l'étoile dépasse ~6 px
(galerie, vue objet, approche) — pas au cadrage par défaut, faute de motif à
différencier ; le gain réel est en **systèmes multiples**. La graine glacée, dès
le premier zoom sur un système extérieur.

## 3. Astres exotiques

Manque de **rendu**, de **donnée** (pas tiré par `ProfilEtoile::aleatoire`,
`etoile.rs:17-70`), ou des deux.

| Type | Galerie (`galerie_etoiles.rs:7-33`) | Tiré | Manque |
|---|---|---|---|
| Pulsar, magnétar, étoile à neutrons, Wolf-Rayet, supergéante bleue | oui (`couronne_type` 3/1/4/2/2) | **oui** | rien |
| T Tauri | oui (type 1) | **oui** | son disque protosolaire n'est jamais posé (§5) |
| Supergéante rouge, sous-naine (sdB) | oui, mode 0 | non | **donnée** ; le rendu normal leur convient |
| Étoile carbonée (C) | oui, mode 0 | non | **donnée** ; distincte par la seule teinte |
| **Étoile à flares (M)** | oui, mode 0 | non | **les deux** — elle ne diffère d'une naine M que par 200 K et 0,05 de luminosité : aucun signe distinctif |
| **Trou noir** | oui (type 5) | non | **donnée** : `ProfilEtoile.couronne` ne vaut jamais 5, et ni `genese/mod.rs:51-54` ni `ecran/objet.rs:65-68` n'ont de bras `5 =>`. Il n'existe **qu'**en galerie |
| Naine brune | — (côté gazeuses) | non | **rien au rendu** : quatre presets soignés (`genese/mod.rs:1012, 1017-1019`). Manque de pouvoir être le **centre** d'un système |
| Céphéide / Mira, naine blanche pulsante | non | non | **les deux** : `pulse_amp`/`pulse_freq` n'existent nulle part. Une seule pièce pour les trois |

**Le gros du travail « exotique » est de la donnée, pas du rendu** : cinq types
sur onze sont déjà peints et n'attendent qu'une ligne dans `aleatoire` plus un
bras dans trois `match` ; un sixième (la naine brune) est mieux rendu que la
plupart des étoiles. Seule pièce neuve, la **pulsation** couvre trois types d'un
coup — peu coûteuse, mais elle touche une source déclarée unique
(`soleil/mod.rs:169-179`, lue par le rendu **et** par `Systeme::pick`) : toute
pulsation du rayon bouge la cible de clic, piège n° 1 de P.3 bis. ⚠️ `Variante`
(`stellaire.rs:23`) ignore trou noir comme pulsation : un exotique ajouté sans
elle n'apparaîtra jamais dans un système multiple. **Ce qui se verra** : la
pulsation à **n'importe quelle échelle** ; les types « donnée seulement », par
couleur et taille dès le cadrage par défaut.

## 3 bis. Les types absents du catalogue (2026-09-24)

⚠️ Section ajoutée après relecture : la §3 fait l'inventaire de ce que le
dépôt **connaît**, pas de ce qu'il **ignore**. Filtre appliqué, posé par
l'utilisateur : **un type n'entre au catalogue que s'il peut avoir une
représentation distincte à l'écran.** Un objet qui se rendrait exactement
comme un type déjà présent n'est pas un type, c'est une étiquette.

**Retenus** — ils se distinguent vraiment :

| Type | Ce qui le rend distinct | Ce qu'il coûte |
|---|---|---|
| **Quasi-étoile** (*black hole star*) | Enveloppe de plusieurs milliers de masses solaires chauffée par **accrétion sur un trou noir central**, pas par la fusion : surface très froide (~4 000 K, rouge sombre) pour un rayon de l'ordre de l'**UA**. Un disque stellaire qui dépasse l'orbite d'une planète interne. | Rendu : une supergéante rouge démesurée + la signature du cœur. **Appartient au chantier trou noir (§4)**, pas à côté : c'est le même objet vu du dehors. Attention à `PX_MIN` et à `l_etoile_agrandie_reste_dans_l_orbite_de_mercure` — ce type-là dépasse **vraiment** l'orbite interne, et la garde devra le prévoir. |
| **Hypergéante / variable lumineuse bleue** (Eta Carinae) | Éjections violentes, nébuleuse bipolaire en sablier. | Le `couronne_type 2` (vent Wolf-Rayet) en fait déjà la moitié ; reste la forme en sablier. |
| **Étoile Be** | Rotation si rapide que l'étoile s'**aplatit**, entourée d'un **disque de décrétion**. | Le disque existe (`src/disque/`) ; l'aplatissement rejoint §3 ter. Bon rapport : deux pièces déjà là. |
| **Herbig Ae/Be** | La grande sœur massive de T Tauri. | **Donnée seulement** : même rendu (jets + disque), autre plage de masse. |

**Écartés, faute de représentation distincte** — gardés ici pour qu'on ne
repose pas la question : étoile à **quarks** et objet de **Thorne-Żytkow**
(l'une se rend comme une étoile à neutrons, l'autre comme une supergéante
rouge), **naine noire** (par définition invisible), **blue straggler** (une
étoile bleue ordinaire au mauvais endroit — c'est une information de
catalogue, pas un rendu).

## 3 ter. Distinguer les **types** entre eux, et pas seulement les individus

La §2 traite d'un défaut : deux étoiles **du même type** sont jumelles. Il en
existe un second, distinct, que la graine ne réglera pas : **deux types
différents partagent la même recette de surface.**

Un type ne fait varier aujourd'hui que la teinte, le rayon, la luminosité et
la forme de couronne. La granulation, elle, est la même recette pour une
naine M et une supergéante — alors que leurs cellules de convection
diffèrent de plusieurs ordres de grandeur (quelques centaines de kilomètres
contre une fraction du rayon stellaire). Manquent, dans le même esprit :

- **granulation paramétrée** (`gran_scale`/`gran_contraste`), déjà réclamée
  par `suivi/etoiles.md` ;
- **assombrissement centre-bord** : le shader fait aujourd'hui l'inverse, il
  éclaircit le limbe (`suivi/etoiles.md`, « limb darkening vrai ») ;
- **taux d'activité par type** — c'est le réglage figé du §1.3 ;
- **aplatissement des rotateurs rapides**, qui sert aussi l'étoile Be.

⚠️ **Conséquence sur l'ordre de travail** : le lot « graine » (§6, lot 1) ne
répond qu'à la moitié du reproche « les textures sont identiques ». Sans ce
lot-ci, deux étoiles cesseront d'être jumelles mais une naine M continuera
de ressembler à une supergéante.

## 3 quater. Les événements (nova, supernova) : hors jeu, et pour la même raison qu'ailleurs

Tranché le 2026-09-24. Le projet a déjà écarté l'évolution temporelle longue
pour les géantes gazeuses (`conception/geantes_gazeuses.md` §6 bis : on garde
ce qui est **spatial et statique**, on omet ce qui est un **événement**). La
même règle s'applique aux étoiles : ni nova, ni supernova, ni cycle
d'activité pluriannuel. **Pas d'exception.** Gardé sous le coude si le
gameplay réclame un jour un élément plus dur — une supernova est alors une
mécanique de jeu (une menace, une échéance), pas un effet visuel de plus.

## 4. Trous noirs, et le sous-chantier des ceintures de débris

**Ce qui existe.** Une seule branche : `soleil.frag.glsl:61-91`, sous
`couronne_type > 4.5`, qui peint **en 2D dans le repère du quad** un disque
elliptique (`incl = 0,40` **en dur**), une spirale `fbm`, un beaming Doppler, un
anneau de photons, un horizon noir ; l'occultation arrière est un hack,
`step(v_q.y, 0.0)` (l. 78) — « le haut est caché », littéralement. Le disque n'a
**pas de plan dans le monde** : l'incliner ou le voir par la tranche est
impossible. Et `CorpsBase::new(…, 1000.0, …)` (`soleil/mod.rs:81`) lui donne
**la masse de n'importe quelle étoile** — aucun `gm` pour un disque.

**Le lien de `ceintures.md` §7 est plus court qu'annoncé.** Ce §7 écarte le
disque d'accrétion du modèle `Disque`, « candidat naturel à une migration vers
le voile quand la lentille sera conçue ». **La migration ne dépend pas de la
lentille, et elle est presque gratuite** : `Disque` sait déjà dessiner **une
moitié à la fois** (`draw_moitie(cam, ±1)`, `disque/mod.rs:221`) pour le rendu
peintre *arrière → corps → avant*, et **l'ordre d'appel correspondant est déjà
écrit** pour les anneaux planétaires — `planete/mod.rs:349` puis `:408`, de part
et d'autre du corps, exactement la topologie d'un trou noir. Il sait aussi
prendre un **plan quelconque** (`avec_normale`), tourner différentiellement,
émettre au bord interne (`emissif`, `disque_voile.frag.glsl:152`), gérer face
jour/nuit et ombre du corps central. Lui manquent le **beaming Doppler**
(quelques lignes à côté de `face_lum`), un **gradient thermique**, une vitesse
crédible (donc un `gm` propre, donc casser la masse fixe de 1000), et
l'**anneau de photons**, qui reste au shader puisqu'il épouse l'horizon.
**Coût** : un champ dans `Soleil`, deux appels dans `rendu.rs`, un preset
`accretion`, deux uniforms au voile. **Migrer sans attendre la lentille.**

**La lentille : faisable, mais pas là où on croit.** « Pas de passe de
post-traitement dédiée aujourd'hui » est **à moitié faux** : `ecran/pixel.rs`
**est** une telle passe — `nouvelle_cible` (l. 45) crée une `RenderTarget` avec
profondeur, `preparer`/`presenter` (l. 221-248) y rendent la scène 3D puis la
reblittent, `palette.frag.glsl` la ré-échantillonne en plein écran. Deux faits
la rendent impropre : **elle ne contient pas le fond stellaire** —
`rendu.rs:65-68` dessine la couche nette d'abord (`fond.draw`, trajectoires,
zones) *puis* les corps dans la cible nettoyée en transparent (l. 93), si bien
qu'une lentille tordrait les planètes en **laissant le champ d'étoiles droit**,
l'inverse de ce qu'elle doit montrer ; et **elle n'existe qu'en mode pixel**.
Une **lentille locale** est en revanche accessible sans toucher à
l'architecture : un quad impostor plus large que la couronne, dont le fragment
échantillonne le fond stellaire en UV déviés par `déviation ∝ 1/b` — GLSL ES 100
le permet, et `src/fond.rs` a déjà son material. Résultat : **l'anneau
d'Einstein sur les étoiles lointaines**, les corps du système restant non déviés.

**Ce qui se verra** : le trou noir étant **absent du jeu** hors galerie, le
premier gain est de pouvoir en rencontrer un ; ensuite le disque se voit **bien
plus loin que la surface d'une étoile** (`couronne = 3,4`), profitant comme la
CME d'être **hors** du disque central. La lentille ne se lit qu'en approche.

## 5. Dépendances entre sujets

- **La graine (§2) profite à tous les types d'un coup** : un uniform de plus sur
  le seul shader stellaire, dont héritent granulation, campfires, spicules, vent
  WR et arcs de magnétar. Meilleur rapport gain/effort.
- **§1 et §3 partagent le réglage figé** de `reglages_etoile` ; et **§3 a besoin
  de §1 pour l'étoile à flares**, qui sans sursaut n'a aucun signe distinctif.
- **Le trou noir n'a pas besoin de la lentille**, mais du modèle `Disque` —
  **déjà livré** (`src/disque/`, phases 1-3 de `ceintures.md` §6 faites) ; il lui
  faut en revanche une masse variable (`soleil/mod.rs:81`).
- **T Tauri et le disque protosolaire** : `DisqueConfig::protosolaire`
  (`config.rs:240`) et la couronne à jets existent, leur mariage annoncé en
  `ceintures.md` §3.4 n'est pas fait. Même geste, même endroit.

## 5 bis. Le chantier ceintures, au-delà du disque d'accrétion (2026-09-24)

Question posée à la relecture : le sous-chantier ceintures se réduit-il à la
migration du disque d'accrétion ? **Non.** État constaté dans le code, contre
le plan de `ceintures.md` §6 :

| Phase de `ceintures.md` §6 | État réel |
|---|---|
| 1-3 (particules GPU, voile, nouveaux types) | **faites** |
| 4 (éclairage) | **faite**, et mieux qu'annoncé : face jour/nuit **et** ombre du corps central sur le voile (`disque_voile.frag.glsl:137-146`) |
| 5 (génération) | **partielle** — `oort()` est bien posé dans le preset solaire (`presets.rs:256`), mais `protoplanetaire()`/`protosolaire()` n'existent **qu'en galerie** : aucune jeune étoile du jeu ne porte son disque |

Restent donc, par ordre de visibilité :

1. **Les comètes.** `Categorie::Comete` existe (`astre.rs:14`) et **rien ne
   l'engendre** — la catégorie est morte depuis le début, la bucket list
   (§5) le note. C'est le manque le plus voyant du lot : une queue orientée
   à l'opposé de l'étoile est un objet que le joueur cherche des yeux. Le
   modèle `Disque` ne la couvre pas (une comète est un corps + une traînée,
   pas un champ) ; c'est du rendu neuf.
2. **Les disques proto\* dans les systèmes générés** — phase 5 inachevée,
   et c'est aussi ce qui manque à T Tauri (§5).
3. **Le disque d'accrétion** du trou noir (§4), qui ne dépend pas de la
   lentille contrairement à ce qu'annonce `ceintures.md` §7.
4. **Le remplissage de pixels des voiles**, jamais mesuré : l'instrument
   existe (touche **V**), la mesure jamais prise. Dette ouverte depuis P.2.

Écartés et confirmés tels : troyens L4/L5 et traînées météoritiques
(`ceintures.md` §7, écartés à la demande).

## 6. Ordre de travail proposé

*Chaque lot livre du visible ; aucun ne dépend du suivant.*

| # | Ce qu'il change | Ce qu'il livre |
|---|---|---|
| 1 | Uniform `graine` sur le shader stellaire + champ sur `ProfilEtoile`/`Soleil` ; `app.seed` posé dans `apparence_glacee` | Deux étoiles d'un binaire cessent d'être jumelles, deux lunes glacées d'avoir la même géographie. **Plus petit lot, plus large effet, seul à toucher deux familles** |
| 2 | Plancher en pixels + fondu d'alpha sur les billboards de plasma, modèle `disque_particules.vert.glsl:146` | La CME, déjà large de 24 px au cadrage par défaut, devient visible. **À faire avant les flares** : il dit ce qui est déjà là et évitera d'en réécrire |
| 3 | Ancrer `declencher_boucle` sur les taches ; canal **éclaircissant** dans `spots[8]` (ruban + pic) ; modulation brève de la luminosité ; brancher `reglages_etoile` sur le type, **par étoile** | Une naine M active clignote et se distingue d'une géante : le sursaut se lit dès 3 UA, son éclat à toute distance |
| 4 | Bras `5 =>` dans les trois `match`, entrée dans `aleatoire`, `Variante::TrouNoir`, masse propre | On peut rencontrer un trou noir en explorant |
| 5 | `Option<Disque>` dans `Soleil`, encadrement `draw_moitie(∓1)`, preset `accretion`, Doppler + gradient thermique au voile ; idem pour le disque protosolaire de T Tauri | Un disque qui a un plan, se voit par la tranche et tourne juste |
| 6 | Supergéante rouge, sous-naine, carbonée, naine brune au tirage | Le tirage cesse de se répéter |
| 7 | `pulse_amp`/`pulse_freq`, dérivés dans `rayon_dessine` **et donc** dans `Systeme::pick` | Céphéide, Mira, ZZ Ceti d'un coup, visibles à toute échelle |
| 8 | Lentille **locale** : impostor élargi, UV déviés sur le fond stellaire | L'anneau d'Einstein, en approche — si elle est décidée (§9) |

**Lots ajoutés le 2026-09-24**, à intercaler selon ce qu'on veut voir
d'abord :

| # | Ce qu'il change | Ce qu'il livre |
|---|---|---|
| 9 | **Comètes** (§5 bis) : engendrer `Categorie::Comete`, orbite très excentrique, queue opposée à l'étoile | Le manque le plus voyant des petits corps, et une catégorie morte qui prend enfin vie |
| 10 | **Distinction entre types** (§3 ter) : `gran_scale`/`gran_contraste`, assombrissement centre-bord, activité par type | Une naine M cesse de ressembler à une supergéante. **Sans lui, le lot 1 ne tient que la moitié de la promesse** |
| 11 | **Quasi-étoile** (§3 bis), après le trou noir (lots 4-5) | Un objet qu'on n'oublie pas : une enveloppe rouge sombre large comme une orbite, nourrie par un trou noir |
| 12 | Disques proto\* posés dans les systèmes générés (§5 bis) ; hypergéante, Be, Herbig Ae/Be au tirage | Les jeunes étoiles portent enfin leur disque ; trois types de plus pour peu de rendu neuf |

## 6 bis. Le lot 3, redécoupé en sous-lots (2026-09-24)

Le lot 3 a été livré (arches ancrées sur les taches, flare, activité par
étoile), puis un **lot 3 bis** a corrigé deux régressions et la rondeur des
taches. L'utilisateur juge le résultat bon sur les taches, **insuffisant sur
les boucles**, et a demandé une recherche avant d'aller plus loin. Trois
documents en sont sortis :

- [`astres_v2_recherche_visuelle.md`](astres_v2_recherche_visuelle.md) — la
  cible et neuf critères d'acceptation ;
- [`astres_v2_champ_partage.md`](astres_v2_champ_partage.md) — d'où faire
  découler taches et arches ;
- [`astres_v2_boucles.md`](astres_v2_boucles.md) — comment tracer une arche
  qui se lise comme un tube.

⚠️ **Ce que la recherche a corrigé, et qui change le diagnostic.** Ce
document affirmait un « champ magnétique » déjà présent dans
`soleil.frag.glsl` : **il n'existe pas**. Le seul du fichier est celui de la
branche magnétar, dans la couronne, sur un axe câblé. Il n'y a donc pas
trois sources qui divergent, mais **deux sources et un trou** : rien ne dit
où le champ est fort, donc les taches se posent au hasard et les arches
ailleurs. C'est l'origine de l'effet « autocollant », et aucun réglage de
forme ne pouvait la corriger.

Second constat, de `astres_v2_boucles.md` : le « croissant » se forme
**entre** les arches et non dans une seule — le blending additif somme les
halos de deux arches nées côte à côte. Les bornes du lot 3 bis (moins
d'arches, cadence plus lente) atténuaient le symptôme, pas la cause.

La suite du lot 3 est donc découpée en cinq sous-lots, **du plus fondateur
au plus cosmétique**. Chacun se tient seul ; aucun ne dépend du suivant.

| # | Contenu | Ce qu'il livre |
|---|---|---|
| **3.1** | Le **champ magnétique** : quelques pôles dérivés de la graine, en Rust pur, interrogeable en tout point de la sphère | Rien à l'écran — mais la source unique qui manquait. Entièrement testable sans contexte graphique |
| **3.2** | Les **taches naissent sur les maxima** du champ, en **groupes** de tailles inégales (une principale, des suiveuses) | La première moitié du « sticker » : une tache cesse d'être posée au hasard, et cesse d'être seule |
| **3.3** | Les **pieds d'arche sur deux polarités opposées** du même groupe, plus une **répulsion** entre arches voisines | Le lot qui doit tuer le croissant : des arches alignées en arcade au lieu de rayonner d'un point |
| **3.4** | Le **shader lit le champ** : assombrissement et étirement de la granulation suivent la même source que les taches | La seconde moitié du « sticker » : la tache appartient enfin à la surface |
| **3.5** | Le **ruban** à la place du chapelet de billboards, et l'**ancrage visuel** des pieds | Une arche qui se lit comme un tube — et **moins cher** qu'aujourd'hui (32-48 sommets contre 192) |

**Deux risques nommés**, à garder en tête à chaque sous-lot :

1. **Le réglage esthétique n'est pas testable.** Le nombre de pôles et la
   forme du noyau décident entre « deux taches immobiles », « bouillie » et
   le retour des taches trop rondes. Ça se juge à l'œil, au-delà d'environ
   10 px de rayon stellaire.
2. ⚠️ **Ne pas tirer les pôles au hasard.** Un `gen_range` de plus dans
   `Soleil::new` décalerait toute la suite de la génération : à graine
   égale, le système entier changerait. Les pôles se **dérivent** de la
   graine, comme `temps_initial` (lot 1).

**Livrés.** **3.1** (`soleil/champ.rs`) : des paires de pôles de polarités
opposées dérivées de la graine, et `champ(dir)` signé. **3.2**
(`champ::groupe` + `Soleil::naitre_groupe`) : une tache ne naît plus seule ni
n'importe où — une région active naît d'un coup, tache de tête sur une
polarité, suiveuses sur l'autre et à côté, les trois tailles sorties du champ
au lieu de trois tirages sans rapport.

Deux choses que 3.2 a apprises, et qui n'étaient pas prévues :

- ⚠️ **Une région peut être noyée par sa voisine.** Les paires étant posées
  au hasard, un pôle peut tomber près du pôle **opposé** d'une autre paire :
  le champ s'y annule. Mesuré — un groupe naissait dans un champ **plus
  faible que la moyenne de la sphère**, c'est-à-dire à nouveau au hasard.
  D'où `CHAMP_MIN` : sous ce seuil, la région ne fait rien naître et le
  prochain rendez-vous retente ailleurs. Un groupe compte donc **1 à 3**
  taches, pas 3.
- **Les pôles ne bougeant pas, les régions reviennent aux mêmes endroits**
  pendant toute la session. Assumé ici (`astres_v2_champ_partage.md` §7) ;
  une dérive lente coûte un float, elle attend qu'on la juge nécessaire à
  l'œil. Le garde-fou `region_occupee` empêche au moins les taches de
  s'empiler aux trois mêmes points jusqu'à saturer `MAX_TACHES`.

**3.3** (`champ::pieds`, `champ::paire_de`, `Soleil::place_libre`) : une arche
relie désormais **deux polarités opposées de la même région**, au lieu
d'enjamber le centre d'une tache sur une tangente **tirée au hasard** — d'où
l'éventail partant d'un point unique. Les arches coulissent le long de la
**ligne neutre** (`ETALEMENT`), trois places par région, la première libre
gagne : c'est l'**arcade**. Et une arche **refuse de naître** à moins de
0,22 rad d'une autre (`COS_ARCHES`) — la répulsion attaque enfin la cause du
croissant, les halos additifs qui somment *entre* deux arches voisines, là où
le lot 3 bis n'en traitait que le symptôme (moins d'arches, plus lentement).

⚠️ **Corrigé après retour écran (2026-09-25)** — « les boucles partent de
zones externes aux taches dans beaucoup de cas ». Non seulement ce n'était
pas normal, mais c'était **systématique** : mesuré, **100 % des pieds**
tombaient hors de leur tache, à 1,26 rayon du centre en médiane et jusqu'à
2,42. Deux fautes, toutes deux d'échelle :

1. **L'étalement de l'arcade était mesuré contre la région** (0,34 rad,
   l'écart entre deux pôles) alors qu'un pied doit tomber dans une **tache**,
   dont le rayon plafonne à ~0,26 rad et vaut souvent 0,15. Un pied coulisse
   désormais dans **une part du rayon de sa propre tache** (`PART_PIED`).
2. **Les pieds s'ancraient sur les pôles, pas sur les taches vivantes.**
   Équivalent en apparence — le champ est maximal au pôle, c'est là que les
   taches naissent — mais `groupe` écarte les sites trop faibles (une région
   sur dix ne porte de tache que d'un côté), et une tache **meurt** en 8 à
   16 s là où un pôle reste. `eruptions::ancres` choisit maintenant la plus
   grosse tache **réelle** de chaque polarité, et **aucune arche ne naît**
   s'il en manque une : mieux vaut pas d'arche qu'une arche sans pied.

Conséquence en cascade : la répulsion valait 0,22 rad, calibrée sur l'ancien
étalement. Les pieds ne se déplaçant plus que de ~0,14 rad, **aucune seconde
arche n'aurait jamais tenu** — et une arcade d'une seule arche n'est pas une
arcade. Ramenée à 0,07 rad. C'est un **réglage esthétique** : il arbitre
entre « croissant » et « arche solitaire », et se juge à l'œil.

Ce que 3.3 a appris, et qui vaut pour la suite du chantier :

- ⚠️ **Un test dont les cas d'entrée sont choisis à la main peut être une
  tautologie sans qu'on le voie.** La garde « les deux pieds sont de
  polarités opposées » restait verte après avoir été *retirée du code* :
  les trois glissements que le test essayait passaient tous à côté des cas
  qu'elle attrape. Mesuré ensuite : elle **refuse 5,6 % des ancrages**
  (2 025 sur 36 000), une région noyée par sa voisine retournant la polarité
  d'un pied. Le test balaie maintenant une grille et **exige de rencontrer
  des refus**.
- **`forme` ne règle plus l'écartement des pieds** : il est celui du champ.
  Le slider garde son effet sur la hauteur et le biais de l'arche. La
  variété de silhouette vient désormais du **cisaillement** (les deux pieds
  coulissent inégalement), pas d'un tirage sur l'écartement.

**Rien ne change encore à l'écran** côté surface : c'est 3.4 qui fera lire le
champ au shader, et 3.5 qui remplacera le chapelet de billboards par un
ruban. C'est voulu — ça isole la régression.

## 7. Hors périmètre

**Rendre les arches visibles au cadrage par défaut** : tranché et chiffré par
P.3 bis (il faudrait un Soleil plus large que l'orbite de Mercure) ; le lot 2
porte sur les **éjections**. **Une passe de post-traitement globale** (§4) :
chantier de rendu à part entière. **La physique relativiste** : le beaming
Doppler reste un effet peint. **L'accrétion simulée** : `ceintures.md` §7 exclut
déjà toute interaction physique des débris. **Refondre `ceinture/`** : déjà fait
— ce document ne rouvre du chantier ceintures que le point de son §7.

## 8. Ce qui ne sera pas testable

[`../methode.md`](../methode.md) est clair : pas de test de rendu.
**Se testent** : que deux étoiles tirées de suite aient des graines distinctes et
que `apparence_glacee` ne rende plus 0,0 (même garde que `genese/mod.rs:1219`) ;
que la taille écran d'un billboard de plasma ne descende pas sous le plancher
(fonction pure, comme `rayon_plancher`) ; qu'une boucle naisse sur une tache ;
que `aleatoire` atteigne chaque type et que chaque `match` couvre tous ses
bras ; que `rayon_dessine` pulsé reste la source unique lue par `pick` ; que le
disque d'accrétion soit dessiné en deux moitiés (invariant d'ordre).

**Ne se jugent qu'à l'œil** : que le motif *paraisse* différent (galerie, deux
cellules côte à côte) ; que la CME *se lise* comme une bulle qui se détache (vue
système, avant/après, même graine) ; que le ruban *ressemble* à un flare (vue
objet, une naine M) ; que la pulsation ne soit pas épileptique. ⚠️ Au lot 7, la
garde `l_etoile_agrandie_reste_dans_l_orbite_de_mercure` (`soleil/mod.rs:246`)
porte sur le rayon *dessiné* : une pulsation qui le gonfle passe devant elle.

## 8 bis. Tranché le 2026-09-24, avant le lancement du chantier

Quatre décisions prises par l'utilisateur sur les lots ajoutés (9 à 12), plus
quatre valeurs par défaut posées faute d'enjeu. **Ne pas les rouvrir sans
raison**, même consigne que les décisions actées de `STATE.md`.

1. **La comète est un objet de jeu, pas un décor.** Cliquable, listée au
   sélecteur, avec sa fiche, et utilisable comme **destination de vol**. Sa
   queue n'apparaît qu'à l'approche de l'étoile et s'efface au loin — c'est
   ce qui la fait vivre, et c'est physiquement juste.
   ⚠️ **Conséquence technique, à ne pas manquer** : une destination de vol
   doit avoir une position **prédictible** (`Systeme::calculer_trajectoire`
   rend `None` sinon — c'est pourquoi une lune n'est aujourd'hui pas une
   destination valide, `conception/trajectoires.md` §7). La comète doit donc
   être un astre **sur rails** au sens de `Foyer::Etoile`/`Orbite`, comme une
   planète, et surtout pas un enfant attaché par `parent`. Ce choix-là
   conditionne tout le lot.
2. **La quasi-étoile garde son échelle vraie.** Elle dépasse réellement les
   orbites internes ; la garde `l_etoile_agrandie_reste_dans_l_orbite_de_mercure`
   (`soleil/mod.rs:246`) doit apprendre ce type plutôt que l'interdire. Un
   système autour d'elle n'a pas de planète interne : ce n'est pas un défaut
   de génération, c'est la conséquence.
3. **L'assombrissement centre-bord est corrigé pour toutes les étoiles.** Le
   shader éclaircit aujourd'hui le limbe ; c'est faux pour une photosphère
   dense, et une photo du Soleil le montre. Le Soleil du preset change donc
   d'aspect : c'est voulu, et c'est une correction, pas un réglage.
4. **Les disques protoplanétaires reçoivent leurs proto-planètes**, placées
   dans leurs sillons — le modèle le prévoit déjà explicitement
   (`conception/ceintures.md` §3.2). Un sillon que rien ne creuse est un
   décor qui ment.

5. **La masse gravitationnelle se dérive du type stellaire** — et c'est le
   **lot 0**, avant tout le reste. `MASSE_ETOILE` vaut aujourd'hui 1000 pour
   toutes (`bucketlist_globale.md` §9 le note depuis longtemps) ; le trou
   noir et la quasi-étoile en ont besoin d'une propre. Le faire d'abord
   plutôt qu'après : une masse « cas particulier » posée pour le seul trou
   noir serait une **seconde source pour la même grandeur**, exactement le
   travers qui a coûté le plus cher à ce dépôt (`methode.md`, « une source
   par fait » ; l'affaire `MASSES_TERRESTRES` de la revue P1). ⚠️ Touche les
   barycentres des systèmes multiples (`stellaire.rs`) et les périodes
   orbitales : à faire pendant que rien d'autre n'est en vol.

**Par défaut, faute d'enjeu** (à rouvrir si le rendu déçoit) : une à trois
comètes par système ; **une seule queue** (poussière), la queue d'ions
droite restant un raffinement ; quasi-étoile **rare** au tirage et jamais
dans un preset écrit à la main ; aplatissement de l'étoile Be porté par un
**uniform** du shader stellaire, réutilisable ensuite par tout rotateur
rapide.

## 9. Ce qui reste à trancher

1. **La lentille : locale ou rien ?** L'accepter locale, c'est accepter que les
   planètes ne soient pas déviées — parti pris visuel, comme `PX_MIN`.
2. **La masse des étoiles.** Le trou noir a besoin d'un `gm` propre, et casser la
   constante à 1000 (déjà notée par `bucketlist_globale.md` §9) touche les
   barycentres des systèmes multiples : chantier à part, ou cas particulier ?
3. **Le trou noir a-t-il des planètes ?** `est_remnant` lui retirerait la zone
   habitable, mais l'échelle orbitale et l'éclairage d'un tel système restent à
   définir.
4. **Jusqu'où va la pulsation ?** Une amplitude honnête (quelques pour cent) ne
   se verra qu'au chiffre ; une amplitude lisible sera de la triche assumée,
   comme `PX_MIN`.
