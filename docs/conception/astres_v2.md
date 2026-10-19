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

⚠️ **Second retour écran (2026-09-25)** — un offset résiduel sur l'origine
des arches, et « quand une tache disparaît des arches peuvent continuer à
apparaître ». Deux causes de plus :

3. **Le rayon dessiné n'est pas le rayon nominal.** `rendu.rs` encode pour le
   shader `taille × intensité` — une tache n'atteint sa taille qu'au sommet
   de sa vie — pendant que l'ancrage visait `taille`. Pendant l'apparition
   (1,5 s) et l'extinction (2 s), le pied se retrouvait donc dehors. Les deux
   lisent maintenant `eruptions::rayon_visible`, **source unique**. Une tache
   sous `ANCRAGE_MIN` d'intensité ne porte plus d'arche du tout : ses pieds
   sont fixés à sa naissance, elle rétrécit sous eux. Le suivi **dynamique**
   des pieds reste au sous-lot 3.5.
4. **Le repli aléatoire faisait naître des arches sans tache.** Il existait
   « pour qu'une étoile sans tache continue d'éruptionner » — c'est
   précisément ce que l'utilisateur a vu. Supprimé, pour les arches **et**
   pour les flares, qui avaient le même : un sursaut est une reconnexion de
   champ, il ne flambe pas sur une surface vide. Une étoile calme a le droit
   d'être calme. `dir_aleatoire` n'avait plus d'appelant et a été supprimée —
   **plus rien, dans ce fichier, ne se pose au hasard sur la sphère.**

⚠️ **Troisième retour écran (2026-09-25)** — « les arches ont souvent toutes
la même taille », « ça fait très fonction du second degré », et la demande de
**petites boucles qui ne relient pas deux taches**. Trois causes, toutes
dans la géométrie :

5. **La hauteur était absolue** (`0,25 + 1,3 × puissance`), indépendante de
   la portée. Comme l'écart entre les deux taches d'une région ne varie
   guère, toutes les arches montaient à la même altitude au-dessus du même
   écart : même taille, même forme. C'est désormais le **rapport**
   hauteur/écart qui se tire (`apex_pour`), jamais la hauteur.
6. **La courbe était une Bézier quadratique** — un seul point de contrôle,
   donc littéralement une **parabole**, la même pour toutes à l'échelle
   près. Passée en **cubique** : deux points de contrôle, d'où un col pincé
   ou un dôme évasé (`pincee`) et une inclinaison hors du plan radial
   (`inclinaison`). Même coût, un polynôme par point.
7. **Toutes les boucles reliaient deux taches.** La moitié d'entre elles
   (`PART_COURTES`) repartent maintenant **dans leur propre tache**
   (`champ::pieds_courts`) : petites, éparses, bornées par le rayon de la
   tache comme un pied d'arche. Ce sont les plus nombreuses dans la réalité,
   et c'est d'elles que vient l'essentiel de la variété de gabarit.

8. **Toutes les boucles avaient la même épaisseur** — `r × 0,05`, constante
   pour toutes *et* sur toute leur longueur : un chapelet de billboards du
   même calibre. Chaque boucle porte désormais son **calibre** propre
   (`Boucle::epaisseur`, du fil ténu au ruban massif) et un **galbe** le long
   du tube (`rendu::galbe`, pieds serrés et ventre plus large, comme un tube
   de flux qui s'évase en s'élevant). Le galbe redistribue l'épaisseur, il ne
   l'augmente pas — sa moyenne reste à 1. Le cœur d'une éjection suit le même
   calibre.

Le plafond des boucles est **séparé par catégorie** (`MAX_ARCHES = 4`,
`MAX_COURTES = 12`) : sous un plafond commun, en faire naître plus de petites
revenait à en voir moins de grandes, alors qu'elles ne coûtent pas la même
chose à la lecture. Et la **répulsion est relative** — une fraction de
l'empreinte des deux boucles (`MARGE_ARCHES`) et non un angle fixe : les
petites boucles se serrent, puisque leurs halos sont petits eux aussi (le
calibre suit l'étendue, `rendu::RAPPORT_TUBE`). Comme `PART_COURTES`, ce sont
des réglages qui se jugent à l'œil.

**La galerie d'étoiles passe à 5 colonnes**, cellules dimensionnées sur la
largeur. Elle divisait la largeur d'écran par 210 px : en 1440p, douze
étoiles minuscules — et c'est pourtant la seule vue où l'on puisse juger la
granulation, une tache ou une boucle, rien ne se lisant sous ~10 px de rayon
(§0).

**3.4** (`poles[6]` en uniform, `soleil.frag.glsl`) : la surface lit enfin le
champ. Un **voile sombre** là où il est fort — la convection y est bloquée
bien avant qu'une tache s'y creuse — et une **granulation étirée** dans le
sens des lignes, obtenue en comprimant la direction d'échantillonnage du
bruit le long du champ. C'est la seconde moitié du « sticker » : la tache ne
s'arrête plus net sur une surface qui l'ignore.

Deux points à retenir :

- ⚠️ **L'intensité lue par le shader n'est pas `champ::champ`.** Le GPU somme
  les **valeurs absolues** des contributions, le CPU garde la somme
  **signée**. Les deux polarités d'une région s'annulant, la somme signée
  passe par zéro **sur la ligne neutre** — exactement entre les deux taches,
  là où le champ est fort et couché à l'horizontale. S'y fier éteignait
  l'effet au seul endroit où il compte. Mêmes pôles, même noyau, même
  portée : c'est la question posée qui diffère, polarité d'un côté, intensité
  de l'autre.
- **`NB_POLES` et `PORTEE` sont dupliqués en GLSL.** C'est la dette CPU/GPU
  connue, sous sa forme **tolérable** — des constantes géométriques, pas un
  `hash` (`astres_v2_champ_partage.md` §3). Deux tests comparent les
  littéraux du `.glsl` à ceux du Rust, et le test d'uniforms existant couvre
  la déclaration.

**3.5** (`impostor::push_ruban`, `rendu::ancrage`) : le **ruban** remplace le
chapelet de billboards, et les pieds sont **ancrés visuellement**.

- Une bande continue le long de la courbe, dont la largeur s'ouvre
  perpendiculairement à la course apparente et dont largeur et couleur
  s'interpolent aux jointures. Ce qui manquait au chapelet, c'est la
  continuité **longitudinale** : on devinait les pastilles une à une même sur
  une arche isolée (`astres_v2_boucles.md` §2). Les UV traversent la texture
  de halo par son **axe médian**, ce qui en donne exactement le profil
  transversal d'un tube — vif au centre, éteint aux bords — sans texture ni
  matériau de plus. **Zéro pipeline ajouté**, et deux sommets par point au
  lieu de quatre.
- Un ruban coupé derrière l'étoile reprend en un **nouveau brin** : un
  `continue`, comme sur les billboards, aurait relié les deux bords par-dessus
  le disque.
- **Ancrage** (lot D de la recherche) : près d'un pied, le plasma cède la
  place à la teinte de l'étoile et perd de l'éclat (`ANCRAGE`), et une lueur
  marque l'articulation avec la surface. Un ruban qui garde tout son éclat
  jusqu'au bout s'arrête net sur la photosphère et se lit comme posé devant
  elle ; un ruban qui s'éteint tout à fait n'a plus de pied. Les deux
  amplitudes se jugent à l'œil.

⚠️ **Ce que 3.5 ne fait pas** : suivre géométriquement une tache qui
rétrécit. La recherche a tranché que l'ancrage attendu était **visuel** (halo
et teinte, §5), pas un recalcul des pieds à chaque frame. Le garde-fou contre
la dérive reste `ANCRAGE_MIN` du sous-lot 3.3 — on n'ancre pas sur une tache
pâle.

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

## 6 ter. Lot 4, livré (2026-09-26)

**On peut rencontrer un trou noir en explorant.** Son rendu existait déjà
(`Soleil::avec_trou_noir`, mode de couronne 5) et la galerie l'exposait —
mais **personne ne produisait ce mode** et **personne ne l'attendait** : le
`match` de `feuille_alea` n'avait pas de bras `5`, qui retombait donc sur
`Normale` en silence. Un objet entier hors du jeu, sans un message.

Quatre points :

- `etoile::particuliere` est **extrait** du tirage : `gen_range` exige le
  contexte graphique, donc le tirage ne se teste pas — mais ce qu'on en fait,
  si. C'est désormais le seul endroit où se décide quels objets exotiques
  existent, et un test balaie ses bornes pour vérifier qu'aucun type n'est
  devenu injoignable.
- Le trou noir tient **un cinquième des particulières**, soit ~1 % des
  tirages. Bien au-dessus du réel, comme les 5 % de particulières elles-mêmes :
  le but est la rencontre, pas le recensement.
- ⚠️ **Sa masse ne se déduit pas de sa luminosité** (`MASSE_TROU_NOIR`). Le
  peu qu'il rayonne vient du disque d'accrétion, pas d'une photosphère :
  passé à `masse_luminosite`, il se couchait au **plancher** — l'objet le plus
  lourd du système pesant un quart de masse stellaire, et des barycentres à
  l'envers. La valeur dépasse délibérément le plafond de `masse_luminosite`,
  qui borne les *étoiles*. La garde est un `const assert` : elle échoue à la
  **compilation**.
- Il est **rémanent** : pas de zone habitable. Elle se déduit de la
  luminosité, qui ici décrit un disque d'accrétion — la tracer donnerait un
  anneau d'eau liquide autour d'un horizon.

## 6 quater. Lot 5, livré (2026-09-26)

**Le disque d'accrétion a un plan.** Il était peint dans le quad de
l'impostor : une ellipse en coordonnées **écran**, d'inclinaison câblée à
0,40. Elle présentait donc éternellement la même face — impossible d'en faire
le tour, impossible de la voir par la tranche, et elle ignorait l'inclinaison
du système. C'est maintenant un vrai [`Disque`], dessiné en **deux moitiés**
autour de l'impostor, exactement comme un anneau planétaire (`planete/mod.rs`).
Ne restent peints dans le shader que l'anneau de photons et l'horizon, qui
appartiennent vraiment à la sphère.

- **Preset `accretion`** : du gaz, pas des cailloux (aucune particule). Son
  gradient thermique est **inversé** par rapport aux disques proto\* —
  blanc-bleu au bord *interne*, orange sombre à l'externe.
- **Beaming Doppler** (`DisqueConfig::doppler`) : le côté qui vient vers la
  caméra s'éclaircit, l'autre s'assombrit, d'autant plus fort près du bord
  interne où la matière tourne le plus vite. C'est le signe qui distingue un
  disque d'accrétion d'un anneau ; à `0` pour tout le reste, un anneau de
  Saturne n'ayant aucune raison d'être plus brillant d'un côté. Effet
  **peint**, pas relativiste (§7).
- **Le plan est dérivé de la graine** : deux trous noirs ne présentent pas la
  même face. ⚠️ Dérivé, jamais tiré — même piège que `champ::poles`.
- **T Tauri porte enfin son disque protosolaire**, via un mode de couronne 6
  et une `Variante::Proto`. Il partageait le mode 1 avec l'étoile à neutrons,
  qui n'a que faire d'un berceau de planètes.

⚠️ **Trouvé en chemin : le voile n'avait aucune garde uniforms↔GLSL**, à la
différence des shaders d'étoile et de planète, alors qu'il en porte près de
trente. Deux tests ajoutés — correspondance, et absence d'uniform mort.
Le premier sabotage est passé **à côté de sa cible** (l'indentation avait
changé avec une extraction) et le vert obtenu ne voulait rien dire : vérifier
que le sabotage a bien eu lieu fait partie du red-check, pas seulement lire
la couleur.

**Reste ouvert** : à longue distance l'étoile est agrandie par le plancher en
pixels (`PX_MIN`) alors que le disque garde sa taille physique — le disque
passe donc sous l'horizon dessiné. Sans conséquence visible à cette échelle,
où plus rien ne se lit, mais c'est une incohérence réelle.

## 6 quinquies. Le lot 5, prolongé en deux sous-lots (2026-09-26)

Le disque d'accrétion du lot 5 a un plan, mais ne convainc pas. La recherche
[`astres_v2_disque_accretion.md`](astres_v2_disque_accretion.md) a classé
quatre causes et tranché : la décisive n'est pas la matière mais la
**silhouette** — sans déviation des rayons, un disque incliné *est* un anneau
incliné, et personne n'a cette image en tête en pensant « trou noir ».

| # | Contenu | Ce qu'il livre |
|---|---|---|
| **5.1** | Un **shader dédié** : profil de température en puissance, beaming **coloré**, turbulence cisaillée. Le vocabulaire des anneaux quitte le disque d'accrétion, et réciproquement | La matière. **Rien ne change de silhouette** — c'est voulu, ça isole la régression |
| **5.2** | Le **pli** : la moitié arrière du disque relevée par-dessus et par-dessous l'astre, déformation par sommet pilotée par `tocam` | L'image. Un anneau qui se referme sur l'astre |

**Ordre de livraison, demandé par l'utilisateur** : chaque sous-lot apparaît
d'abord dans **GALERIE - CEINTURES & DISQUES**, pour que l'objet soit
répertorié à côté de ses voisins et jugeable seul, **puis** dans la vue des
étoiles. Un disque qu'on ne peut comparer ni à un anneau de Saturne ni à un
disque protosolaire ne se juge pas.

Ce sont des **sous-lots du lot 5**, pas un lot neuf : le disque d'accrétion
est son sujet, et il n'est pas fini. Le lot 6 garde son contenu.

**Journal du chantier** :
[`suivi/astres_v2.md`](../suivi/astres_v2.md) — état de chaque lot, ce qui
reste ouvert, et ce que le chantier a appris.

**Livrés.** **5.1** : `disque_accretion.frag.glsl` et son pipeline, profil de
température en puissance (adouci à 1,6 — la valeur physique 3 éteint le bord
externe au point qu'il disparaît), beaming coloré, turbulence cisaillée,
blending **additif**. `doppler` quitte le voile partagé, qui redevient le
shader des anneaux. **5.2** : le **pli** — la moitié arrière du disque
dessinée **deux fois**, relevée au-dessus puis au-dessous de l'astre, pour
imiter la silhouette que produit la déviation des rayons.

**5.3** : les onze corrections de
[`astres_v2_disque_accretion_2.md`](astres_v2_disque_accretion_2.md), après
une seconde recherche — le pli **comprime** au lieu de dilater, le bruit
devient anisotrope et enroulé en spirale, la teinte chaude est ordonnée
`R > G > B` pour que le clipping additif la mène au blanc et non au cyan, le
beaming passe en `D³` plafonné, la géométrie est recalibrée sur le rayon de
l'**ombre** et non de l'horizon, et l'anneau brillant devient l'anneau de
**lentille** porté par le disque. Tout le vocabulaire d'accrétion part dans
un type `Accretion`.

**5.3 validé à l'écran le 2026-09-29**, après sept correctifs successifs —
chacun parti d'une capture. Le sous-lot porte aussi, au final, les **trois
régimes** du trou noir : endormi (halo blanc large, peu de matière), actif, et
vorace (disque élargi de moitié plus une ceinture de débris qui mord sur son
bord externe). Les trois ne diffèrent que par des **constantes**, portées par
le type `Accretion` — c'est ce qui rendra le 5.4 possible au même prix.

**5.4** : la cible **Interstellar**, livrée comme prévu — un **preset** de
plus, sans une ligne de shader. Trois écarts avec la
cible chaude, et rien d'autre : orangé à 4500 K **sans gradient radial**,
beaming écrasé par son plafond (β reste inchangé — la vitesse orbitale tient
au rayon, pas à la température), turbulence basse. Bornes, pli et anneau de
lentille sont **identiques** : c'est ce qui fait porter la comparaison sur
l'aspect et non sur la géométrie, et une garde le vérifie.

**Jugée « très satisfaisante » à l'écran le 2026-09-30.**

**5.5** : la cible froide **déclinée en appétits**, plus une quatrième teinte.
Demande de l'utilisateur au vu du 5.4 : les mêmes variantes que le 5.3, et une
de plus, plus rouge, aura comprise.

Le sous-lot est surtout un **découpage**. Jusque-là un preset mêlait deux
questions — *combien l'astre mange* et *de quoi ça a l'air* — et les décliner
l'une par l'autre aurait fait réécrire trois fois « mange moins, brille moins,
tourne moins ». Les deux axes sont désormais séparés : `nom` porte l'appétit,
`cible` la teinte, et `refroidie` applique une teinte à **n'importe quel**
appétit. Le catalogue devient un produit, et une cible de plus coûte une ligne.

⚠️ **La frontière entre absolu et proportionnel n'est pas de commodité.** Ce
que la cible impose est absolu — teinte, gradient retiré, plafond du beaming,
minceur. Ce à travers quoi l'appétit doit continuer de se voir est
proportionnel — éclat, turbulence, rotation, halo, cellules, spirale. Mettre
l'éclat en absolu rendrait les trois appétits froids identiques : trois
cellules pour un seul objet. Une garde tient cette distinction
(`l_appetit_se_voit_encore_une_fois_refroidi`).

Sept presets au catalogue : trois appétits × deux cibles, plus la braise —
déclinée une seule fois, puisque c'est une variante de **teinte** et non
d'appétit.

## 6 sexies. Le lot 5 clos : on garde les sept (2026-09-30)

La décision du §5 de la recherche — comparer les cibles, puis en choisir une —
**n'a pas débouché sur un choix**. Décision de l'utilisateur : *« pour garder
de la variation je suis preneur de garder les 7 cas de représentation de trou
noir, ça rajoute de la diversité »*.

⚠️ **Garder les sept veut dire les brancher sur le tirage.** Quatre n'existaient
que dans les vues de travail ; les y laisser aurait rendu la décision sans
effet sur le jeu. `regime()` puise donc dans le catalogue entier.

**Le poids d'un appétit se partage entre ses cibles.** La décision ajoute de la
*variété*, pas de la *fréquence* : un vorace reste à 20 %, il a simplement deux
visages. Sans ce partage, décliner une teinte de plus rendrait mécaniquement un
état de crise plus courant.

À graine égale un trou noir existant change d'aspect — inhérent à la décision —
mais rien ne se décale en cascade : le preset est **dérivé** de la graine, il ne
consomme pas le flux aléatoire.

**Le lot 5 est clos.** Restent deux dettes consignées au journal, aucune
bloquante : le disque passe sous l'horizon dessiné à très longue distance
(`PX_MIN`, §6 quater), et la convention de silhouette « mince » n'a jamais été
essayée. Le lot suivant est le **6**.

## 6 septies. Le lot 6, découpé en deux (2026-09-30)

| # | Contenu | Ce qu'il livre |
|---|---|---|
| **6.1** | Supergéante rouge, sous-naine sdB, étoile carbonée au tirage ; `filtre_teinte` ; la table sortie dans `etoile::ordinaire(p, q)` ; `genese::premiere_orbite` | Le tirage cesse de se répéter, et sa table devient testable |
| **6.1 bis** | Le défilé de la vue isolée couvre aussi les types **ordinaires** | On peut aller voir les nouveaux types sans régénérer |
| **6.2** | La naine brune comme **centre** d'un système | Un système autour d'un objet qui n'est pas une étoile |

⚠️ **Le 6.2 a fait apparaître une hypothèse que tout le dépôt portait sans le
dire : le corps au centre d'un système est un `Soleil`.** Quatre endroits en
dépendaient — la masse (plancher de `masse_luminosite`), le plancher en pixels
(réservé aux `Soleil`), la source de lumière (cherchée par `Categorie::Etoile`)
et la luminosité totale (les `Planete` n'en rendent aucune, d'où des fiches à
0 K). Chacun a été corrigé au fait qu'il énonce vraiment, pas par un cas
particulier « naine brune ».

**Limite assumée** : une naine brune ne peut pas être la **compagne** d'un
système multiple — l'arbre stellaire ne bâtit que des `Soleil`. `feuille_alea`
l'écarte explicitement plutôt que de la peindre en étoile.

⚠️ **La §3 se trompe sur un point, et c'est ce qui donne son contenu au 6.1.**
Elle donne l'étoile carbonée « distincte par la seule teinte », donc de la
donnée pure. Mesuré : dérivée du corps noir comme les autres, elle est **moins
rouge** qu'une naine M froide — `(1,00 / 0,68 / 0,40)` à 2900 K contre
`(1,00 / 0,61 / 0,24)` à 2400 K. Ce serait une étiquette, pas un type, ce que le
filtre de la §3 bis interdit explicitement.

D'où [`filtre_teinte`] : un multiplicateur sur la couleur de corps noir, qui est
la physique même — une carbonée est une photosphère froide vue **à travers** ses
bandes de C₂ et CN. La sous-naine a le problème symétrique : sa couleur est
celle d'une naine blanche, et seul son **éclat** (cinq à vingt fois supérieur)
la distingue.

**Et le vrai enjeu du 6.1 était la testabilité.** Les types ordinaires vivaient
dans la chaîne de `if` d'`aleatoire`, qui appelle `gen_range` : hors d'atteinte
de tout test, **exactement** comme le trou noir avant le lot 4. Les
particulières avaient reçu leur garde à ce moment-là, les ordinaires jamais. La
table est donc sortie dans `ordinaire(p, q)` — `q` donnant la place dans la
plage de rayon et de luminosité, sans quoi une table figée referait des jumelles.

**Trou structurel trouvé en chemin** : rien n'empêchait de poser une planète
**dans** son étoile — le garde-fou de `instancier` ne compare que les planètes
entre elles. Sans conséquence jusqu'ici par coïncidence (l'échelle suit `√L`),
mais la supergéante monte à 12 unités et la quasi-étoile du lot 11 est annoncée
large comme une orbite. `premiere_orbite` pose la règle une fois.

Trois points du pli, tous vérifiés :

- Le **raccord** se fait tout seul : le pli s'éteint sur les côtés
  (`smoothstep` sur « à quel point ce point est derrière l'astre »), là où la
  matière n'a jamais été cachée. L'arc du dessus est donc continu avec le
  disque plat, sans couture.
- Le **second arc** (celui du dessous) s'efface là où le pli est faible,
  sinon les deux copies se superposeraient exactement — en additif, une bande
  deux fois trop brillante le long du raccord.
- Le pli **s'annule quand le disque est vu de face** : la normale du plan n'a
  alors plus de composante à l'écran, et rien n'est caché derrière l'astre.
  C'est aussi ce qui évite de normaliser un vecteur nul.

⚠️ **La moitié avant ne se plie pas.** Elle n'a jamais été cachée par
l'astre, donc rien ne la dévie ; la relever décollerait le disque de ses
propres bords. Et `vdot`, qui choisit la moitié, se calcule **avant** le
déplacement — sinon un sommet relevé pourrait changer de camp en vol.

**Coût mesuré** : 3 passes d'annulus au lieu de 2, soit ~11 500 sommets par
frame et par trou noir, contre 25 600 pour un vaisseau. Le pipeline ajouté
porte le compte à 11 sur 32 (garde `le_nombre_de_shaders_reste_sous_le_plafond`).

⚠️ **Échelle du disque protosolaire de T Tauri**, question ouverte soulevée à
l'écran (« je n'aurais pas imaginé un disque aussi grand »). Il fait
**16 rayons stellaires**, soit 0,28 UA. À comparer : la galerie des disques
en montre un de **6 UA**, et la réalité tourne autour de **100 UA**. Il
paraît grand parce qu'il se mesure contre l'étoile ; il est en fait déjà
vingt fois plus petit que celui de la galerie. Le mettre à l'échelle de
celle-ci couvrirait tout le système et masquerait ses planètes. Laissé tel
quel faute d'un meilleur compromis, mais `RAYON_PROTOSOLAIRE` est une
constante isolée.

## 6 octies. Le lot 7, livré (2026-09-30)

**Une seule pièce pour trois types**, comme la §3 l'avait prévu :
`etoile::pulsation(nom)` rend `(amplitude du rayon, période)`. La **luminosité
n'a pas de réglage à elle** — `L ∝ R²` à température constante, donc elle suit le
carré. Un seul fait, une seule source, et la variation d'éclat se voit du coup
**à n'importe quelle échelle**.

**La question du §9 est tranchée.** Les amplitudes de rayon sont **réelles** pour
la céphéide (≈ 10 %) et Mira (≈ 20 %) ; seule **ZZ Ceti est exagérée** — son
rayon ne bouge en vrai que de quelques millièmes. Les **périodes** sont toutes
compressées, et ne peuvent pas ne pas l'être.

⚠️ **La pulsation n'est pas un mode de couronne.** Une céphéide est au mode 0,
donc elle n'aurait atteint aucune étoile par `Variante` — il aurait fallu
l'écrire cinq fois. `Variante::appliquer` prend désormais le **nom** du type, et
`Feuille` le porte. C'est la réponse à l'avertissement de la §3 : « `Variante`
ignore trou noir comme pulsation ».

⚠️ **Le piège annoncé — la cible de clic — n'en était pas un.** La pulsation passe
par `rayon_dessine`, source unique : elle bouge la cible *avec* le disque peint.
Et `Systeme::pick` prend 40 % de marge, donc le rayon cliquable reste toujours
au-dessus du rayon peint. Seule la marge respire.

**Deux types n'existent que par leur pulsation** : Mira recouvre la géante rouge,
ZZ Ceti a le gabarit d'une naine blanche. Le §3 bis l'autorise — une
représentation distincte à l'écran, pas forcément une teinte distincte.

⚠️ **Révisé le 2026-10-19** : l'amplitude de Mira passe de 20 % à **15 %**,
parce qu'elle bouillonne désormais aussi (§6 nonies) et que les deux mouvements
s'additionnent.

## 6 nonies. Le lot 6.3 : les géantes rouges bouillonnent (2026-10-19)

Ajouté en validant le lot 6, à la demande de l'utilisateur : plus de variété
d'aspect pour la géante et la supergéante rouges, *« une boule de matière en
effervescence »*, *« comme de l'eau bouillant dans une casserole »* — et
explicitement **pas** le gonflement régulier de la céphéide. Mené en essai dans
la galerie, puis adopté. Le récit, les réglages et les deux défauts corrigés
sont au journal ([`../suivi/astres_v2.md`](../suivi/astres_v2.md), lot 6.3).

**Ce qui a été décidé, et par qui :**

| Type | Stable | Instable |
|---|---|---|
| Géante rouge | grosses cellules peintes à plat (ébullition 0,5) | **contour** irrégulier (ébullition 1) |
| Supergéante rouge | grosses cellules peintes à plat (ébullition 1) | **relief** par l'éclairage (ébullition 1) |
| Mira | — | toujours instable : contour, pulsation allégée à 15 % |

- **L'utilisateur** a choisi l'association aspect → type (« la géante en
  ébullition rend bien une géante stable, la supergéante contour rendrait bien
  une géante instable… »), le remplacement des anciens modèles, l'instabilité
  de Mira et l'allègement de sa pulsation.
- **Choix par défaut, faute d'enjeu** : 35 % d'instables au tirage
  (`soleil::PART_INSTABLE`), dérivé de la graine.

**Lien avec le lot 10 (§3 ter, distinguer les types entre eux).** Ce lot en
règle **une part** sans l'avoir prévu : la granulation n'est plus « la même
recette pour une naine M et une supergéante » — les géantes rouges ont
désormais des cellules de convection géantes, ce que la §3 ter réclamait
(« une fraction du rayon stellaire »). Le bord assombri de l'ébullition
anticipe aussi le limbe du lot 10, **pour ces seuls types**. Restent au lot 10 :
la granulation paramétrée des autres types, l'assombrissement centre-bord pour
**toutes** les étoiles (décision 3 du §8 bis), l'activité par type.

**Le relief et le contour sont des impostures assumées**, comme `PX_MIN` :
l'étoile reste un impostor, une sphère parfaite reconstruite dans un quad. Le
relief ne déplace rien (il éclaire une pente calculée), le contour ne déforme
que l'image — cible de clic et occultation restent sur un cercle.

**Le lot 6 est clos le 2026-10-19**, validé à l'écran par l'utilisateur : la
carbonée, l'ébullition, la supergéante rouge et la sdB point par point, les
naines brunes globalement. L'outil qui a permis de juger chaque type en
contexte réel — un système construit autour d'un type choisi, menu « choisir
astre » de la Skymap V2 — est consigné au journal.

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
