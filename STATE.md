# STATE — où en est le chantier

> Point de reprise, **2026-08-04**. À lire en premier dans une nouvelle session.
> Ce fichier dit **où on en est et quoi faire ensuite**, rien de plus : le
> détail de chaque étape (ce qui a été trouvé, les red-checks, les mesures)
> vit dans les journaux de [`docs/suivi/`](docs/suivi/), et le *pourquoi* dans
> [`docs/conception/`](docs/conception/).

## Commits

Dernier commit : `f843f9c` « Safe commit before GUI ».

Consigne debout : *« no commit i'll say when commit is needed »* — ne jamais
commiter de soi-même. **Mais signaler les points de commit** : l'utilisateur
commite aux jalons qu'on lui indique, depuis qu'un `git checkout --` a détruit
du travail non commité (`docs/suivi/stations.md` §F.8).

⚠️ **Ne jamais employer `git checkout --`, `git restore` ou `git reset --hard`**
sur un fichier contenant du travail non commité. Pour annuler ses propres
modifications : ré-éditer les lignes, ou sauvegarder le fichier dans le
scratchpad **avant** de le toucher (red-checks compris).

État vérifié : **411 tests verts**, **`cargo clippy` : 0 erreur**.

## Chantier courant : finir les planètes et la vue système

**Décidé le 2026-08-06, avant D-INT-2** : l'économie attendra que la vue système
soit finie. On ne conçoit pas une économie par-dessus un décor qu'on va encore
remuer.

| # | Chantier | Nature | État |
|---|---|---|---|
| **P.1** | **Atmosphère planétaire** | vrai manque, confirmé dans le code | ⏳ |
| **P.2** | **Ceintures : finition + optimisation** | mesurer d'abord | ⏳ |
| **P.3** | **Étoiles : éjections et motifs de surface** | ⚠️ **existent déjà** — enquête avant travaux | ⏳ |
| **P.4** | Finitions planétaires (rotation propre, nuages, ombres d'anneaux) | liste ancienne | ⏳ |

### P.1 — Il n'y a pas d'atmosphère, et voici pourquoi

`planete.frag.glsl` ligne 863 : `if (rr > 1.0) discard;`

Tout ce qui dépasse la sphère est **jeté**. L'impostor est pourtant un disque de
`DISC = 1.05`, donc **5 % de marge autour du corps ne servent à rien**. Ce que le
shader appelle « halo atmosphérique » (ligne 1046) n'est qu'un éclaircissement du
limbe **sur la surface elle-même** :

```glsl
float rim = pow(1.0 - z, 3.0);
col += atmo * rim * (0.35 + 0.65 * diff);
```

Rien ne brille **au-delà de la silhouette**. C'est exactement ce qui manque : une
atmosphère se voit d'abord parce qu'elle déborde du disque, et parce qu'elle
s'allume en croissant du côté du soleil.

**Travail** : rendre la coquille dans la marge (rayon 1,0 → 1,05), épaisseur
optique en fonction de l'angle de vue, teinte par diffusion, croissant au
terminateur. ⚠️ Toucher à `DISC` touche au LOD `px_rayon` et au cadrage : à
mesurer avant.

### P.2 — Ceintures : mesurer avant d'optimiser

Ce qui est **déjà bien** : les buffers sont **figés** (`construire_lots`) et
l'orbite est calculée **dans le vertex shader**
(`omega = sqrt(gm/r³)`, `theta = graine·τ + omega·time`). Rien ne remonte au CPU
par frame.

Le préset solaire tire **6 400 particules** en 4 ceintures (900 astéroïdes,
2 000 Kuiper, 900 épars, 2 600 Oort), soit ~16 `draw_mesh` par frame plus les
voiles.

**Le vrai gisement est le culling** : le nuage de Oort s'étend de 2 000 à
20 000 UA et il est dessiné **même quand la caméra est à 1 UA**, où ses
particules font moins d'un pixel. Aucun test de visibilité n'existe
(`disque/mod.rs`).

⚠️ **Mesurer d'abord** : le compteur de FPS est déjà à l'écran en galerie. Sans
chiffre avant/après, « optimiser » est une opinion.

### P.3 — ⚠️ Les éjections stellaires existent déjà

**Vérifié dans le code avant d'ouvrir le chantier**, et c'est important :

| Ce qu'on croyait manquant | Réalité |
|---|---|
| Éjections coronales | `soleil/eruptions.rs` : `Boucle` avec cycle complet **Montée → Tenue → Rupture → Retombée**, corde de flux (`apex_vel`), rendue en billboards additifs (`soleil/rendu.rs`) |
| Taches | `Tache` avec durée de vie, poussées dans `spots[8]` |
| Motif de surface | granulation `fbm` (`soleil.frag.glsl` l. 87), champ magnétique (l. 97) |
| Couronne | trois types : halo, jets bipolaires, vent stellaire |

Et `Astre::update` **est bien appelé** en vue système (`systeme/mod.rs:393`).

**Donc le défaut n'est pas l'absence : c'est que ça n'arrive pas à l'écran.**
L'hypothèse la plus probable est l'**échelle** — à distances réelles le Soleil
fait 1,1 U de rayon quand Neptune orbite à 1 440 U, si bien que boucles et
granulation tombent sous le pixel. C'est le **cinquième** défaut de ce type dans
le projet (calotte, villes, atmo de Titan, texture zonale, ventre) : *la donnée
est juste, la plomberie ou l'échelle ne suit pas*.

**Travail** : d'abord **constater** (galerie d'étoiles contre vue système), puis
décider — LOD sur `px_rayon` comme les géantes, ou exagération d'échelle assumée.
Ne **pas** réécrire ce qui existe.

Reste ensuite un vrai manque possible : la **variété d'une étoile à l'autre**
(même graine de granulation partout ?), à vérifier en galerie.

**479 tests, 0 avertissement clippy.**

### Ce qui vient d'être fait (2026-08-05 → 06)

| Chantier | Résultat |
|---|---|
| **Revue P1** | 117 → **0** avertissement. 90 éléments morts triés, dont **77 exercés par les tests** — la mesure précédente était fausse. A révélé un vrai défaut : `MASSES_TERRESTRES` était documentée « source unique » et **personne ne la lisait**. |
| **Réglages persistés** | `reglages.json`, GAMMA au curseur, tramage retiré. |
| **Zoom** | Plancher à `0,01` (×128 000), plans de coupe suivant la distance. |
| **Texte** | Deux défauts distincts : glyphes absents de la police (157 caractères), puis **tuiles noires** dues au repaquetage de l'atlas. |
| **Starship 1/5 → 5/5** | Coque, 6 Raptor, jupe moteur, 4 volets, bardage de tuiles, hublots, exposé debout. |

## L'interface de jeu (I.0 à I.5 — faites)

Conception écrite le 2026-08-04 :
**[`docs/conception/interface.md`](docs/conception/interface.md)**. À lire avant
d'écrire une ligne. C'est la première interface **de jeu** du projet — tout ce
qui existe aujourd'hui est de l'outillage de développement.

Écran visé : la **Skymap** (vue système), pas la Starmap.

| # | Étape | État |
|---|---|---|
| I.0 | Chargement des sprites (atlas, `FilterMode::Nearest`, repli) | ✅ |
| I.1 | Noms d'astres (presets seulement ; numérotation orbitale en repli) | ✅ |
| I.2 | Sélecteur de planètes à gauche, rétractable | ✅ |
| I.3 | Panneau de planète au clic | ✅ |
| I.4 | Barre de ressources + nom du système | ✅ |
| I.5 | Agrégation `sur_ui` (une seule porte) | ✅ faite au fil des étapes |

**Trois décisions déjà prises** (§2.2, §4.1) : ressources en **affichage seul**
(`Tresorerie` figée, pas d'économie), noms d'astres **sur les presets
seulement**, clic sur une planète **centre la caméra** comme aujourd'hui.

**Cinq questions ouvertes** en §5.1 — l'icône des crédits (aucun sprite),
quelles ressources dans la barre (14 sprites existent, 5 tiennent), où vit le
nom, comment se ferme le panneau, les lunes dans le sélecteur.

Deux acquis à ne pas réécrire : **`ecran::liste` *est* le sélecteur** (colonne
bornée, hauteur de ligne adaptative, item sous le curseur — rien en lui ne
connaît les composants), et **l'habitabilité se déduit** de
`etoile::zone_habitable` au lieu de se stocker.

## La vue de départ : le système solaire

La `Skymap` ouvre désormais sur **le preset solaire**, et non plus sur une
graine procédurale : depuis qu'elle porte l'interface de jeu, elle doit ouvrir
sur un lieu connu — on y reconnaît les planètes, on y juge les distances. `G`
tire toujours un système au hasard.

Remis au niveau au passage :

- **Anneaux d'Uranus** (monobande bleu ciel) et **de Neptune** (arcs), ajoutés
  au **catalogue** et non au preset : la galerie et la vue système lisent la
  même apparence.
- **Quatre familles de petits corps** au lieu de deux — ceinture principale,
  Kuiper, **disque épars** (orbites excentriques et inclinées) et **nuage de
  Oort** (coquille sphérique, pas un disque). Toutes existaient déjà dans
  `DisqueConfig` ; le preset n'en utilisait que la moitié.
- **L'ISS en orbite terrestre** (`src/engin.rs`) : le pont entre les deux
  moitiés du projet — la station est assemblée par `vaisseau::preset_iss`, cuite
  en maillage, puis mise en orbite comme une lune. Nouvelle
  `Categorie::Engin`.

⚠️ **L'échelle de l'ISS est fausse, et c'est inévitable.** À l'échelle réelle
elle ferait 9 × 10⁻⁶ unité de monde — mille fois moins qu'un pixel. Les facteurs
sont **nommés** (`ORBITE_ISS`, `ECHELLE_ISS`) plutôt que noyés dans un calcul :
un mensonge d'échelle assumé vaut mieux qu'un mensonge implicite.

## ⚠️ La police ne couvre que 157 caractères

`assets/minitel.ttf` fait 12 Ko. Relevé sur sa table `cmap` : il porte les
accents **minuscules** (à â ä ç è é ê ë î ï ô ö ù û ü) — l'essentiel du look
Minitel — mais **pas leurs majuscules**, ni la ponctuation typographique
(`…` `—` `«` `»` `·` `×`). Un caractère sans glyphe ne se dessine pas : l'écran
montre un blanc, ce que l'œil lit comme un texte corrompu.

Deux cas se produisaient pour de bon, tous deux dans la **vue système** :

1. `skymap.rs` titre par `to_uppercase()`. « Système généré » devient
   « SYSTÈME GÉNÉRÉ » — `È` et `É` disparaissent.
2. `liste::tronquer` ajoute une **ellipse** `…` à tout libellé trop long, donc à
   toutes les entrées coupées du sélecteur d'astres.

Corrigé par `police::assainir`, appliquée au **point de passage unique** de tout
le texte du jeu. La table des remplacements est relevée sur la police, pas
devinée. Les accents minuscules sont **laissés intacts** : la police les a, et
les aplatir abîmerait le rendu là où il est juste.

⚠️ **Ce n'était pas une régression de la revue P1** — le défaut est aussi vieux
que la police. Vérifié : les seules retouches de la passe sur `skymap.rs` et
`selecteur.rs` étaient des retraits de `mut`.

⚠️ **`texte` et `mesure` doivent traiter la chaîne à l'identique.** Sinon
`tronquer` mesure une ellipse d'un caractère et en dessine trois : il déborde de
la colonne qu'il vient d'ajuster. Le désaccord **n'est pas testable** — `mesure`
exige un contexte GL — il est donc rendu difficile à écrire : les deux fonctions
ne sont plus que des enveloppes de `preparer`, qui est seule à décider, et qui
est testée. C'est le sabotage qui l'a imposé : « `mesure` n'assainit plus »
passait au vert.

## ⚠️ Carrés noirs à la place du texte (atlas de police)

**Symptôme** : en ouvrant la fiche d'un astre dans la vue système, *tous* les
caractères déjà dessinés dans la frame sortent en rectangles noirs pleins.

**Cause, dans macroquad** (`src/text/atlas.rs`, `Atlas::texture`) : les glyphes
vivent dans un atlas qui grandit à la demande, et quand il grandit macroquad
**détruit l'ancienne texture** pour en créer une plus grande :

```rust
ctx.delete_texture(self.texture);
self.texture = ctx.new_texture_from_rgba8(...);
```

Or macroquad ne dessine pas tout de suite : il **empile**. Tout le texte déjà
empilé dans la frame pointe alors sur une texture détruite — et sort en noir.

Le déclencheur était la fiche : c'est le seul endroit de la vue système à
demander les tailles de glyphe **10 et 12**. Premier emploi ⇒ l'atlas grandit
⇒ toute la frame noircit. D'où « ça noircit quand je clique sur une planète ».

#### ⚠️ Ce n'est pas la taille de l'atlas, c'est son **repaquetage**

Première correction insuffisante. Le vrai mécanisme est pire que la simple
recréation de texture : quand l'atlas déborde, macroquad **vide et repaquette
tous les glyphes** à de nouvelles positions (`Atlas::cache_sprite`) —

```rust
let sprites = self.sprites.drain().collect::<Vec<_>>();
self.image = Image::gen_image_color(new_width, new_height, ...);
for (key, sprite) in sprites { self.cache_sprite(key, image); }
```

— si bien que les quads **déjà empilés dans la frame** gardent des coordonnées
de texture qui ne désignent plus rien. D'où des tuiles noires sur *tout* le
texte de la frame, et pas seulement sur le caractère fautif.

**Correction définitive** : `precharger()`, appelée juste après le chargement de
la police et **avant la première frame**. Elle grave tout l'alphabet pour toutes
les tailles de `TAILLES_PRECHARGEES` (6..=26, le jeu en demande 8 à 22). Tout
débordement a donc lieu hors frame, là où il ne peut rien casser.

⚠️ **Un défaut de ma première correction** : `graver` insérait la taille dans
`TAILLES_GRAVEES` **avant** de vérifier que la police était chargée. Police
absente, la taille était réputée gravée pour toujours et plus rien ne la
rattrapait — précisément le cas du repli sur la police par défaut de macroquad,
qui a son propre atlas jamais préparé. Corrigé : on ne marque qu'après avoir
gravé.

#### La taille qui manquait : 5

`station.rs` bornait la taille de sa liste d'items à **9,0** (`clamp(9.0, 14.0)`).
Or `preparer` rend `round(9,0 × 0,6) = 5`, et la plage préchargée commençait à
**6**. La vue des vaisseaux — celle-là même où l'on regarde le Starship —
gravait donc la taille 5 **en pleine frame**, l'atlas se repaquetait, et tout le
texte de la frame passait en noir.

Une seule vue, une seule taille, tout le texte.

**Corrigé à la racine** : `police::TAILLE_MIN` / `TAILLE_MAX` sont déclarées
**dans `police`**, et ce sont les écrans qui s'y bornent — au lieu de trois
nombres posés à l'œil dans trois fichiers, dont un seul suffisait à rouvrir le
trou. La plage préchargée (4..=24) les couvre, et le test les vérifie *toutes
les deux* au lieu des seules tailles littérales.

⚠️ **Le test rougit sur l'ancienne plage `6..=26`** : c'est la garantie qu'il
attrape le défaut réel, et pas seulement qu'il décrit le correctif.

⚠️ **Fausse piste écartée en chemin** : je soupçonnais le `flush()` de
`planete::draw` de dessiner le texte en attente avec le matériau de planète.
`quad_gl.rs` dit le contraire — chaque `DrawCall` **mémorise son propre
pipeline** au moment de l'empilement (`geometry`, ligne 886) et `draw` le rejoue
tel quel. Un vidage ne peut pas appliquer le mauvais shader.

⚠️ **Autre piste écartée** : le binaire périmé. L'exe était bien postérieur à la
correction et contenait les symboles de `precharger` — donc le préchargement
tournait, et ne suffisait pas.

⚠️ **Piste laissée pour la prochaine fois.** Je n'ai pas pu prouver quel appel
déclenchait encore le repaquetage : l'analyse statique dit que `graver` couvre
tout. `graver` **écrit donc maintenant dans la console** quand une taille est
gravée en pleine frame, avec son numéro. Si les tuiles noires reviennent, cette
ligne nomme la coupable ; le test `les_tailles_du_jeu_sont_toutes_prechargees`
tient l'autre bout.

⚠️ Vérifié et **écarté** : le DPI. macroquad calcule `ceil(taille × dpi_scale)`
avant de mettre en cache (`text.rs`), ce qui aurait décalé toutes mes gravures —
mais `window_conf` ne demande pas `high_dpi`, donc `window_scale` reste à 1, et
l'écran est à 96 ppp. Fausse piste, mais elle valait la vérification.

**Première correction (conservée)** : `police::graver`, appelée depuis `texte` **et** `mesure`, à la
première demande de chaque taille. Elle vide le lot (plus rien n'attend), puis
grave **tout l'alphabet** d'un coup pour cette taille. L'atlas ne grandit donc
plus jamais en cours de frame, et une taille inventée par un futur appelant est
prise en charge d'elle-même.

⚠️ **`assainir` et `graver` forment une paire.** `graver` prépare `ALPHABET` et
rien d'autre ; `assainir` garantit que rien d'autre n'en sort — tout caractère
inconnu (€, ✓, un emoji) devient `?`. Si l'une des deux lâche, l'atlas
regrandit et les carrés noirs reviennent. C'est ce que tient
`assainir_ne_sort_jamais_de_lalphabet`, red-checké dans les deux sens.

⚠️ **Le défaut n'était pas dans la police mais dans le moment.** Les glyphes
manquants (section précédente) et les carrés noirs se ressemblent à l'œil et
n'ont rien à voir : les premiers touchent *certains* caractères, les seconds
*tous* ceux de la frame. C'est cette différence qui a désigné l'atlas.

## Paramètres du jeu

Écran **PARAMETRES** depuis l'accueil (`src/ecran/parametres.rs`), modèle dans
`src/reglages.rs`. Six réglages, et la place est faite pour les suivants : les
entrées sont une **liste**, en ajouter une tient en une ligne.

Un bouton **QUITTER** est posé sous PARAMETRES sur l'accueil. Il sort de la
boucle de jeu (`return`) plutôt que d'appeler `process::exit`, pour que miniquad
ferme sa fenêtre proprement. ⚠️ Sa géométrie est **calculée en ligne dans
`accueil.rs` et n'est donc pas testée** — comme tout le reste de cet écran ; le
non-recouvrement avec PARAMETRES tient à la construction (52 px d'écart pour
40 px de haut).

⚠️ **Deux modes d'affichage, pas trois.** macroquad n'expose qu'un
`set_fullscreen(bool)`, et miniquad l'implémente sur Windows en passant la
fenêtre en `WS_POPUP` à la taille de l'écran (`native/windows.rs`) — c'est
**déjà** un plein écran sans bordure. Il n'existe pas de mode exclusif
(changement de mode vidéo). Proposer « plein écran » et « sans bordure »
séparément donnerait deux boutons au comportement identique.

Tailles : 9 entrées en 4:3, 16:10 et 16:9, de 1024 × 768 à 1920 × 1200.
**Pas de 4K**, comme demandé — et le rendu est en impostors plein écran, donc le
coût monte comme le nombre de pixels.

**D-PARAM-1 soldée** : les réglages sont **sauvegardés** dans `reglages.json`,
à côté du jeu, et relus au démarrage.

La sauvegarde vit dans `appliquer()` — le **point de passage unique** de tout
changement. Répartie dans les six actions du menu, elle finirait par en oublier
une.

⚠️ **Tout échec de lecture retombe sur `default()`** : fichier absent, JSON
cassé, champ ajouté depuis la dernière version. Un réglage perdu n'est rien ; un
jeu qui refuse de démarrer pour un fichier de confort serait absurde. `assainir`
borne ensuite ce qui a été relu — le fichier est éditable à la main, et un gamma
à zéro ou un indice de palette hors liste donneraient un écran noir sans rien
expliquer.

**GAMMA au curseur** (0,5 → 2,5), appliqué **avant** la quantification : corriger
après ne ferait que déplacer des couleurs déjà choisies hors de la palette. Le
calcul (`gamma_depuis_x` / `x_depuis_gamma`) est séparé du dessin, donc testé.

⚠️ **Le tramage a été retiré** le 2026-08-05, sur jugement visuel : le motif se
voyait trop. C'était le remède aux **marches de la palette** (basculement de
bandes entières décrit plus bas) — elles redeviennent donc franches. La
saturation et l'écrêtage restent.

**La ligne PALETTE se grise à une seule palette installée.** Cycler la
ramènerait sur elle-même : le libellé ne bougerait pas, ce qui se lit exactement
comme un bouton cassé. La règle est dans `palette_reglable(rendu, nb_palettes)`,
à qui le nombre est **donné** — pas lu dans `palette::toutes()`.

⚠️ **C'est le sabotage qui a imposé cette forme.** Écrite d'abord en lisant le
global, la règle n'était testable que dans le cas installé : « griser toujours »
passait au **vert**, la branche « on rallume à partir de deux » n'ayant aucune
garde. En paramétrant, les quatre sabotages (griser toujours, ignorer le nombre,
ignorer le mode, seuil à 0) rougissent tous.

### Rendu pixel art (PARAMETRES → RENDU / PALETTE / TRAMAGE)

Trois états de rendu : `NET`, `PIXEL ART` (gros pixels, ce qui existait déjà sous
la touche **P**), `PIXEL ART + PALETTE` (quantification CIELAB). Trois réglages
s'ajoutent en mode palette, grisés sinon : **palette**, **gamma** et
**saturation**. Conception complète, corrections apportées au guide d'origine et
dettes : [`docs/conception/pixel_art.md`](docs/conception/pixel_art.md).

**Ajouter une palette** : déposer un `.hex` (format Lospec, un hexa par ligne)
dans [`assets/palettes/`](assets/palettes/) — ramassé au démarrage, aucun code à
toucher. **Une seule embarquée : Lospec 2000** — les trois premières (Resurrect 64,
Sweetie 16, PICO-8) ont été retirées le 2026-08-05.

**Rien n'est refusé en silence.** Un `.hex` que le jeu n'a pas pu charger est
listé **à l'écran**, dans PARAMETRES, en ambre, avec son nom de fichier et la
raison (`palette::rejets`). C'est la réponse à la classe d'erreur qui a coûté le
plus cher cette session : deux palettes déposées n'apparaissaient jamais au menu
et la seule trace partait dans une console que personne ne lit.

La décision (quoi dire, combien en détailler) vit dans `lignes_de_rejet`, séparée
du dessin et **testée** ; la lecture disque (`lire_dossier`) ne décide de rien, et
le tri (`trier`) est pur — c'est ce qui rend l'ensemble testable sans contexte
graphique.

⚠️ **Plafond : 256 couleurs** (`palette::MAX`), relevé depuis 64 — Lospec 2000
(182) et AllStars (128) étaient **rejetées puis ignorées**, avec pour seule trace
une ligne de console. Le coût de rendu monte avec le nombre de couleurs : la
recherche parcourt toute la palette **par pixel**.

⚠️ **Une palette d'artiste n'est pas une rampe de dégradé.** Mesuré sur
Mesuré alors sur Resurrect 64 : un dégradé de gris ne tombait que sur **8 couleurs**, avec une
marche de L=49 à L=69, et tout ce qui dépasse 89 % s'écrase sur le blanc. Trois
défauts constatés à l'écran en découlent, chacun avec son remède mesuré :

| Défaut vu | Cause mesurée | Remède |
|---|---|---|
| Bandes qui basculent d'un bloc | marche de 0,18 en gris | ~~tramage de Bayer~~ — **retiré** le 2026-08-05 (motif trop visible). Les marches redeviennent franches : dette assumée. |
| Reflet en aplat blanc | spéculaire additif > 1,0 | **écrêtage** (cœur blanc, halo non) |
| Couleurs ternes, océans gris-violet | les **neutres** de la palette attirent les couleurs peu saturées | **saturation** ×1,9 à luminance constante (+53 % de chroma) |

Deux fausses pistes écartées par la mesure, à ne pas re-explorer : la recherche
CIELAB garde déjà 85 % de la chroma (pondérer L n'y change rien), et le tramage
ne désaturait pas — mesuré du temps où il existait.

⚠️ L'écrêtage n'agit **que dans la passe pixel art** : le spéculaire de
`planete.frag.glsl` n'est pas touché, le modifier changerait le rendu `NET` et
donc la galerie, les presets et les captures de non-régression.

Le mode est **global** (`reglages::etat_rendu`), pas par vue : les touches P et
le menu pilotent le même état, et P ne change que le mode (ni la palette, ni le
gamma, ni la saturation). `ecran/pixel.rs` est devenu la **source unique du blit** — les quatre
vues y passent, ainsi que le facteur `PIX` et la création de cible qui étaient
recopiés trois fois.

⚠️ **Seule la couche 3D est quantifiée.** Fond stellaire, orbites et textes
restent nets, comme c'était déjà la règle.

⚠️ **D-PIX-1** : le miroir CPU testable (`palette.rs`) et le shader
(`palette.frag.glsl`) sont deux écritures du même algorithme ; aucun test ne
compile de GLSL, donc rien ne garantit qu'ils restent d'accord.

## Le système solaire et le catalogue

Le preset solaire (`genese/presets.rs`) tire **déjà** ses apparences du
catalogue de la galerie (`preset_tellurique` / `preset_gazeuse`, qui **paniquent**
sur un nom inconnu — il n'y a pas de repli silencieux). Deux corrections le
2026-08-05 :

**1. Le catalogue est devenu déterministe** — c'était la cause racine.

`catalogue_telluriques()` / `catalogue_gazeuses()` tiraient une graine **et** une
taille au sort à chaque construction. Conséquence : la « Terre » de la galerie
n'était pas une référence, mais une planète différente à chaque ouverture de
l'écran. Le système solaire ne pouvait donc **pas** lui ressembler — il n'y avait
rien à quoi ressembler.

Graine et taille sont maintenant déduites du **nom du preset**
(`genese::graine_de_nom`, FNV-1a, et `ClasseTaille::rayon_pour`). Le brassage n'a
pas disparu : il a changé de place, dans la galerie, où la touche **G**
incrémente une `variation`. À `variation == 0`, le décalage est nul — **la vue
par défaut de la galerie EST le catalogue canonique**, ce qui est ce à quoi le
système solaire s'aligne.

Les corps à preset **unique** prennent donc la graine du catalogue et sont
identiques à leur vignette de galerie. `fige(…, "<corps>")` ne subsiste que là où
un preset est **réutilisé** (Callisto et Obéron tirent tous deux « Lune »), sans
quoi ils seraient rigoureusement jumeaux.

**Gain de côté** : sans `gen_range`, le catalogue **se teste enfin**. Cinq tests
neufs, dont « chaque preset demandé par les systèmes scénarisés existe » — jusqu'ici
garanti par un seul `panic!` au lancement, écran noir à la clé.

**2. Appariements revus**, pour cesser de recycler le même preset :

| Corps | Avant | Après | Pourquoi |
|---|---|---|---|
| Ganymède | `Lune` | `Crevasse` | ses sillons (sulci) |
| Callisto | `Carbone` | `Lune` | le corps le plus cratérisé connu |
| Ariel | `Boule de neige` | `Supraglacial` | terrains fracturés brillants |
| Pluton | `Boule de neige` | `Ice Dunes` | dunes vues par New Horizons |

⚠️ **Fixtures recopiées** : les listes de noms des tests (`DEMANDES_TELLURIQUES`,
`DEMANDES_GAZEUSES`, `CORPS`) sont recopiées de `presets.rs`. Un nom ajouté là-haut
sans l'être ici est **moins couvert**, il ne devient pas faux.

⚠️ **Piège évité de justesse** : `chaque_taille_reste_dans_les_bornes_de_sa_classe`
déduisait d'abord ses bornes de `rayon_pour`, la fonction même qu'il teste —
l'attendu bougeait avec elle et le test ne pouvait pas échouer. Il lit maintenant
`bornes_terrestres()` à la source. C'est le mode de défaillance récurrent du
projet : *un test qui mesure autre chose que ce que son nom annonce*.

⚠️ **Bug de LOD corrigé** (`rendu.rs`) : `planete::set_viewport_h` n'était réglé
**que par la galerie**. Dans la vue système, `px_rayon` retombait sur
`screen_height()` — donc en mode pixel art, les planètes étaient ombrées avec le
détail d'un plein écran alors qu'elles sont dessinées dans une cible deux fois
plus petite. Les débris avaient déjà leur correction, pas les planètes.

## Cohérence du catalogue de planètes

Passe du 2026-08-05, déclenchée par « Mercure a de la glace ». **Trois défauts,
dont deux dans le shader** — la donnée, elle, était déjà cohérente.

| Défaut | Où | Cause |
|---|---|---|
| Banquise sur des mondes sans climat (Mercure, Lune, Carbone, Vénus) | `planete.frag.glsl` | `calotte = 1` était censé dire « aucune », mais le seuil est perturbé par un bruit de ±0,30 et `froid` atteint déjà 1 au pôle : `smoothstep(1, 1.05, 1.5)` rendait 1 |
| Lumières de ville sur les cailloux morts | `apparence.rs` + shader | `villes` vaut **1 par défaut** et le shader n'excluait que lave et voile — aucune condition d'air ou d'eau |
| (faux positif) Rivières sans eau sur `Crevasse` | — | ce sont des **coulées de lave** (`riv_lave`) : c'est le test qui était faux |

`calotte >= 1` est maintenant un **sentinelle exact** dans le shader, et
« ni air ni eau ⇒ pas de villes » est appliqué **dans la donnée** (le `push` du
catalogue), là où ça se teste.

Six tests de cohérence neufs, tous red-checkés. Ils tiennent des invariants, pas
des valeurs : *pas de climat ⇒ pas de banquise*, *un monde annoncé gelé porte
vraiment de la glace*, *un monde habitable garde ses villes* (sans quoi une règle
trop large les éteindrait partout sans qu'on le voie), *une rivière coule d'eau
ou de lave*.

⚠️ Ces tests ne sont possibles que **depuis que le catalogue est déterministe** :
avant, il exigeait le contexte graphique.

## Revue de code du 2026-08-05 — liste à traiter

Établi en passant tout le catalogue de planètes au crible et en dépouillant
`cargo clippy`. **Par ordre d'urgence.**

### P0 — défauts réels

| # | Quoi | État |
|---|---|---|
| 1 | Banquise sur les mondes sans climat (`calotte = 1` n'était pas un sentinelle exact) | ✅ corrigé |
| 2 | Villes sur les cailloux morts (`villes = 1` par défaut, shader sans condition d'air) | ✅ corrigé |
| 3 | **Titan sans atmosphère** : voile orange épais mais `atmo = 0`, donc aucun halo de limbe | ✅ corrigé |
| 4 | Géantes gazeuses avec `villes = 1` et `relief = 0,35` — inertes (gardées par `type_p`), latentes | ✅ corrigé |
| 5 | `large_enum_variant` sur `ecran/objet.rs:14` — `Apparence` (376 o) contre 48 o pour la variante étoile : toute copie de `Specimen` traînait les 376 o. Boxée. | ✅ corrigé |

### P1 — le bruit qui cache les vrais défauts ✅ TERMINÉ

C'était **la leçon de la session** : la palette Lospec était refusée avec une
ligne de console, invisible au milieu du bruit. Tant que `clippy` sortait 117
avertissements, le 118ᵉ ne se voyait pas.

**117 → 0.** `cargo clippy --all-targets` est propre.

| # | Quoi | Volume | État |
|---|---|---|---|
| 6 | Imports inutilisés | 9 | ✅ |
| 7 | Code mort | **59** | ✅ trié |
| 8 | `assertions_on_constants` — légitimes, elles gardent une constante | 12 | ✅ `#[allow]` commenté sur les 6 modules de test |
| 9 | Style : `field_reassign_with_default`, `needless_range_loop`, `doc_lazy_continuation`, `too_many_arguments`, `type_complexity`, `useless_conversion`, `needless_update`, `clone_on_copy`, `manual_checked_ops`, `single_char_add_str`, `wrong_self_convention`, `unused_*` | ~50 | ✅ |

#### ⚠️ La mesure précédente était fausse

STATE.md disait : « 55 éléments morts pour la cible binaire, 57 avec les tests —
donc il est bien inutilisé ». **Non.** En comparant vraiment les deux cibles :

```
morts pour le binaire ................ 90
morts pour le binaire ET les tests ... 13
```

**77 des 90 sont exercés par les tests** — ils ne sont pas morts, ils sont
*écrits avant leur consommateur*. `dead_code` n'a aucune notion de « utilisé par
les tests » : il compte sur la cible binaire, et tout ce que seuls les tests
appellent y ressort mort. La conclusion inverse aurait mené à supprimer du code
testé et qui marche.

#### Ce qui a été fait, par groupe

- **`chantier.rs` et `symetrie.rs`** : `#![allow(dead_code)]` **au module**, avec
  la raison en tête de fichier (l'écran d'assemblage du Lot 4 est le
  consommateur). 27 éléments.
- **Le reste** : `#[allow(dead_code)]` **par élément**, chacun avec sa raison sur
  la même ligne — catalogue de l'assembleur, économie du chantier (D-INT-2),
  interaction/désignation, mesures de géométrie tenues par leurs tests.
- **Supprimé** : `Camera::zoom_max()`, sans aucun appelant, et intestable
  (`Camera::new` exige un contexte GL). Ajoutée la session d'avant, jamais
  branchée.
- **`Palette::depuis_hex`** passée en `#[cfg(test)]` : le jeu lit des fichiers.

L'autorisation est **volontairement locale**. Contrôle rouge : une fonction morte
neuve ajoutée dans `orbite.rs` sort **un** avertissement, seul et visible. C'est
tout l'objet de la manœuvre.

#### 🔴 Un vrai défaut trouvé sous le bruit : `MASSES_TERRESTRES`

La table des masses planétaires se documentait comme la **source unique** — et
**personne ne la lisait**. Les masses étaient écrites à la main à **trois**
endroits : la table, chaque appel `poser(...)` du preset, et `parent_de` dans les
tests. C'est exactement le défaut que la table existait pour empêcher, et le test
de Kepler ne pouvait pas le voir : il vérifiait **sa propre copie**.

Corrigé par `masse_monde(nom)` — le pendant de `rayon_monde` — que le preset et
les tests appellent tous les deux. Contrôle rouge : mettre Jupiter à 31,78 au
lieu de 317,8 fait tomber `les_periodes_de_lunes_suivent_kepler` (Lune/Io = 4,91
au lieu de 15,44). Avant, cette même corruption passait au vert.

C'est la justification de la revue en une ligne : **le défaut était visible
depuis le début, noyé dans 117 avertissements.**

#### Deux choix assumés

- **`too_many_arguments` (7 sites) : autorisés, pas refactorés.** Ce sont des
  listes cohérentes — éléments orbitaux, uniforms de shader, dimensions d'une
  poutre. Un type `ElementsOrbitaux` serait plus propre et touche **tous** les
  presets : c'est un chantier à part, pas un à-côté de passe de nettoyage.
- **Ce qui reste candidat à la suppression** (mort dans les deux cibles, gardé
  faute d'une décision qui n'est pas la mienne) : `police::mesure`,
  `sprites::presente`, `Systeme::mode`, `Apparence::avec_eyeball` (remplacée par
  `avec_eyeball_zones`), `Apparence::avec_cryo`, `generateur::preset_isv`
  (remplacée par `preset_isv_fixe` + `preset_isv_equipage`), `vaisseau::voile` et
  `Peintre::voile`. Chacune porte sa raison en commentaire.

### P2 — dettes déjà inscrites

~~D-PARAM-1~~ (soldée), D-PIX-1/3/4 (D-PIX-5 sans objet), `briques.rs`/`vaisseaux.rs`
non déclarés dans `ecran/mod.rs`, hash CPU/GPU dupliqué dans 4 fichiers.

### Ce que la revue a appris

**Deux de mes trois règles candidates étaient trop strictes, et le catalogue avait
raison.** Les rivières de `Crevasse` sont des coulées de **lave** (`riv_lave`), et
la végétation de `Lichen` et `Fungi` prend son humidité dans l'**air**, pas dans
un océan. Ce sont des cas limites délibérés. Les tests encodent désormais la
règle juste — *une rivière coule d'eau ou de lave*, *la végétation veut de l'eau
ou une atmosphère* — et chacun vérifie que **le cas limite existe encore**, sans
quoi la règle deviendrait vide sans que rien ne le dise.

## ⚠️ Piège : matériau partagé + lots de dessin (D-GPU-1)

**Toutes les planètes partagent un seul matériau** : `mat_corps()` rend des clones
du même template, et les clones partagent le pipeline. `mat.set_texture(...)`
n'attache donc **pas** une texture à *ce* dessin — elle l'écrit dans le pipeline,
et macroquad ne l'applique qu'au **vidage du lot**.

Conséquence : sans vidage explicite, *le dernier corps dessiné impose ses
textures à tous les précédents*. Dans le système solaire, le dernier est une
tellurique, donc `zonal = tex_vide()` (canal vert à 0) → `band = 0` →
`base = belt` → **les quatre géantes sortaient d'un brun uni**.

La galerie y échappait **par accident** : elle change de viewport à chaque
cellule, et chaque `set_camera` vide le lot. C'est pourquoi le défaut ne se
voyait que dans la vue système — et pourquoi le chercher dans les données ne
pouvait pas aboutir (le profil zonal CPU était parfait, c'est testé).

Corrigé par un `get_internal_gl().flush()` après chaque `draw_mesh` de planète.

⚠️ **Le piège reste ouvert pour toute texture par objet.** `terrain` court le même
risque sur les telluriques ; il n'est masqué que parce qu'elles partagent souvent
`tex_vide()`. Toute nouvelle texture par astre doit être posée **entre deux
vidages**.

⚠️ **Non testable** : c'est de l'état GPU, invisible à `cargo test`. Le garde
honnête serait une comparaison d'images rendues, que le projet n'a pas. C'est le
**quatrième** défaut de la session où une valeur juste n'atteignait pas le shader
(calotte, villes, atmo de Titan, texture zonale) : la donnée était bonne, la
plomberie non.

⚠️ **Coût** : le vidage est fait à chaque planète, donc plus d'appels de dessin
qu'avant. Invisible aux effectifs actuels ; si la vue système rame un jour, ne
vider que sur **changement** de textures plutôt qu'inconditionnellement.

## Echelle du système solaire

**Les distances étaient déjà réelles** : `a_monde = a_au * UA` avec `UA = 48`,
linéaire, sans compression, et le preset porte les vraies valeurs (0,39 → 39,5 UA).
Il n'y avait rien à corriger là.

**Échelle vraie depuis le 2026-08-05 : plus aucune exagération.** `R_TERRE` vaut
`4,2588e-5 × UA`, soit **0,002 unité** — le vrai rayon terrestre. Le Soleil suit
(109,2 rayons terrestres au lieu des **mille** qu'il valait). Les lunes aussi :
elles étaient exagérées *à part*, la Lune à 0,44 rayon terrestre au lieu de 0,27,
et Ganymède sortait plus petite qu'elle alors qu'elle la dépasse de moitié.

⚠️ **Conséquence assumée** : les planètes sont minuscules. Il faut approcher
vraiment pour qu'une planète fasse quelques pixels, et depuis l'orbite de Neptune
le système intérieur est un point. C'est le système solaire.

**Les lunes ont désormais leurs vraies orbites** (table `LUNES` : demi-grands
axes réels en km). Le placement procédural les mettait six fois trop loin — notre
Lune vers 0,8 unité au lieu de 0,123, soit 60,3 rayons terrestres.

**Les périodes suivent Kepler avec la masse du parent** (`MASSES_TERRESTRES`).
C'est indispensable : Io et la Lune sont à peu près à la même distance de leur
planète, mais Io boucle en 1,8 jour contre 27,3 — parce que Jupiter pèse 318
Terres. Sans les masses, toutes les lunes tourneraient à la même vitesse. La
constante est calée sur la période de notre Lune ; vérification indépendante :
Io ressort à 1,76 jour (vrai : 1,769). Triton est **rétrograde**.

⚠️ **Deux pièges trouvés en chemin.** `ajouter_lune_preset` prenait une taille
**relative** (`rayon_planete * taille_rel`) : lui passer un rayon absolu rendait
les lunes 2 000 fois trop petites. Et `poser_lune` forçait `.max(0.05)` sur le
rayon, soit **vingt-quatre fois la Terre** à l'échelle vraie — toutes les lunes
sortaient en boules géantes. `ajouter_lune_reelle` n'a ni l'un ni l'autre :
Phobos a le droit d'être invisible.

### Bouton d'échelle

`ECHELLE_CORPS` (dans `presets.rs`) multiplie les rayons du Soleil, des planètes
et des lunes **sans toucher aux distances**. À `1.0` = échelle vraie. Le monter
est le compromis habituel pour rendre un système lisible, et c'est désormais un
seul endroit.

**Réglé à ×5.** L'échelle magnifie chaque planète **avec son cortège** :
`distance_lune` la multiplie aussi. Seules les distances au Soleil restent
vraies.

⚠️ **Pourquoi les orbites de lunes suivent l'échelle.** Elles ne la suivaient
pas d'abord, et ×5 a mis **Phobos à l'intérieur de Mars** : il n'orbite qu'à
2,76 rayons martiens, ce qui plafonnait `ECHELLE_CORPS` à **1,15** — autant dire
aucun réglage. J'avais estimé le plafond à ×25 en regardant la Lune (60 rayons
terrestres) au lieu de la lune la **plus serrée**. C'est le test qui l'a dit.

### Zoom et plans de coupe

`DIST_MIN` (dans `camera.rs`) est passé de `2.0` à **`0.01`** : le plafond de
zoom monte de ×640 à **×128 000** pour un cadrage de référence de 1280. À
`ECHELLE_CORPS = 5`, la Terre mesure 0,010 unité — sans ce plancher, on ne
pouvait pas l'approcher assez pour qu'elle remplisse l'écran.

⚠️ **Baisser le plancher seul n'aurait rien donné.** La caméra prenait
`..Default::default()`, donc les valeurs de macroquad : `z_near = 0.01`,
`z_far = 10000`. Les deux tombaient à côté :

- `z_near` valait **exactement** le nouveau plancher → au zoom maximal la
  planète se posait sur le plan de coupe et disparaissait ;
- `z_far` coupait à 10 000 alors que `DIST_MAX` vaut 30 000 et que le nuage de
  Oort est à 96 000.

Les deux suivent maintenant la distance : `z_near = dist × 0.01`,
`z_far = dist × 1000`. Le **rapport** near/far reste donc constant — un near
minuscule avec un far énorme est ce qui fait clignoter les surfaces.
`les_plans_de_coupe_encadrent_la_cible` tient les deux marges.

### Palette

**Une seule, Lospec 2000**, embarquée par `include_str!` depuis
`assets/palettes/sources/lospec-2000.hex`. Embarquée et non simplement déposée :
il en faut **au moins une** même sans dossier d'assets, sinon le menu n'a rien à
proposer et `palette(0)` rien à rendre.

Les trois anciennes intégrées (Resurrect 64, Sweetie 16, PICO-8) sont retirées.
Les `.hex` déposés ont été **déplacés** dans `assets/palettes/sources/` (pas
supprimés) : ce sous-dossier n'est pas scanné, sinon Lospec 2000 serait chargée
deux fois et apparaîtrait en double. Déposer un `.hex` dans
`assets/palettes/` en ajoute toujours une.

**Ce qui était faux avant ça, ce sont les rapports de rayons.** Écrits à la main dans chaque appel :
Jupiter valait 3,1 fois la Terre au lieu de **11**, Pluton près de trois fois trop
gros. Les rapports étaient incohérents et rien ne le disait — c'est
vraisemblablement ce qui faisait que le système ne se lisait pas.

Ils viennent maintenant d'une table unique, `RAYONS_TERRESTRES` (valeurs UAI), via
`rayon_monde(nom)`. Le facteur d'exagération reste (`R_TERRE = 0,55` ; à l'échelle
vraie la Terre ferait un demi-millième de pixel) mais il est **commun**, donc les
rapports sont exacts.

⚠️ **Piège évité** : `ajouter_lune_preset` reçoit le rayon de la planète pour
calculer la limite de Roche, et ces rayons étaient **recopiés en dur** (`1.7`
pour Jupiter). Grossir Jupiter sans les toucher aurait posé ses lunes **dans** la
planète. Tout passe désormais par `rayon_monde`.

**Ceintures aux distances réelles** : disque épars 48 → 1 000 UA (Sedna monte à
937), nuage de Oort **2 000 → 20 000 UA** au lieu de 300 → 480. Le bord interne
est désormais le vrai ; le nuage externe (jusqu'à ~100 000 UA) reste tronqué.
⚠️ Conséquence assumée : il faut un zoom bien plus large pour apercevoir Oort.

⚠️ **Les rayons des lunes restent exagérés** et sans rapport exact : Phobos
mesure 11 km, soit 0,001 unité — invisible. C'est le même mensonge assumé que
l'échelle de l'ISS.

⚠️ **Limite de test** : `construire_preset_solaire` exige le contexte graphique.
Les tests prouvent que la table est juste, pas que le preset s'en sert — un rayon
réécrit à la main les laissait tous verts. D'où
`le_preset_solaire_puise_ses_rayons_dans_la_table`, qui inspecte le **texte
source** (`include_str!`). Inhabituel, mais c'est le seul garde possible ici.

## Chantiers en pause

| Chantier | Où on en est | Reprendre par |
|---|---|---|
| **Assembleur** (Lot 4) | Lots 1–3 clos, Lot 4 conçu, pas commencé | `conception/assembleur.md` §10, puis L4.1 |
| **Starship** ✅ **5/5** | Coque, 6 Raptor, jupe, 4 volets droits, bardage de tuiles (fût **et** ogive) et 2 hublots. Exposé **debout**, moteurs en bas. | Vaisseau **mobile** (transferts entre astres) |
| **Endurance** | Rien | `conception/stations.md` Partie D bis |

## L'assembleur, en résumé

L'ISV est **clos** (`docs/suivi/stations.md` §C.29). Les mégastructures et le
Starship sont journalisés en **Partie F** du même fichier.

| Lot | Contenu | État |
|---|---|---|
| **Lot 1** (L1.1–L1.8) | Fondations : catalogue à source unique, balayage des variantes, déterminisme, rayons déclarés, **capsules**, overlays, mesureur | ✅ |
| **§9** « le boudin » | Enveloppe **rectangle** pour les plaques, distances exactes | ✅ |
| **Lot 2** (L2.1–L2.5) | Modèle du `Chantier` : identifiants stables, `retirer`, undo/redo, palette, sérialisation | ✅ |
| **Lot 3** (L3.1–L3.3) | `pose_prevue` (fantôme), `sous_arbre`, désignation port/pièce | ✅ |
| **Lot 4** (L4.1–L4.5) | **L'écran d'assemblage** | ⏸ conçu, pas commencé |
| **Lot 5** | Sauvegarde disque (L5.1), overlay §8.5 (L5.2) | à faire |
| **Lot 6** | Arbitrage L1.4 (L6.1), composites `figer` (L6.2), `PoutreBout` (L6.3) | à faire |

Le modèle répond aux quatre questions de §8 et **aucun écran ne le consomme
encore**. Premier geste de L4.1 : réexporter `Chantier` depuis `vaisseau`.

⚠️ Le Lot 4 est le **premier lot majoritairement non testé** (§6.6 : pas de test
de rendu) — comme l'interface de jeu. La discipline change de forme : pousser
hors du code de dessin tout ce qui **se décide**, vers des requêtes
red-checkables. Ce qui reste non testé ne doit être que *où le rectangle se
pose*, jamais *ce qu'il signifie*.

Régénérer le catalogue de fils : `FILS=1 cargo test --release le_catalogue_des_fils`.

## Décisions prises (ne pas les rouvrir sans raison)

- **Pièce d'abord** (façon KSP) : palette permanente à gauche, ports compatibles
  qui s'allument. Pas « port d'abord ».
- **Bac à sable libre** : pas de plafond de coût.
- **Capsules** (et rectangles pour les plaques) plutôt que sphères — et **pas**
  d'AABB/OBB.
- **`SousEnsemble` se sérialise en pièces cuites**, pas en recette rejouée.
- **Pas de roulis libre** autour du port (§10.6).
- **Les centres de Stanford et d'Elysium sont séparés** : ils ne sont pas
  destinés à se ressembler (§F.4).
- **`Composant::nom()` rend la famille**, pas famille + variante : la colonne
  fait 100 px, et la vitrine fait déjà défiler les variantes.

## Questions ouvertes

- **Sortir de l'assembleur perd le vaisseau** (§10.7b) : Échap efface le travail
  en cours. **À trancher avant L4.1.**
- **Les mégastructures à taille réelle** : ~2,85 pièces et ~2 850 sommets par
  unité de rayon d'anneau, soit ~1 140 pièces pour l'Elysium de 1,8 km contre 45
  pour l'ISV (§F.3). L'assemblage pièce à pièce ne passera pas à l'échelle.
- **Membrures à axe courbe** : aucune primitive du projet n'en trace, et les
  bras des mégastructures en ont besoin pour être reliés entre eux (§F.1).

## Méthode (ce qui a marché, à ne pas perdre)

- **Red-check systématique** : tout test ajouté est cassé volontairement sur sa
  cible pour vérifier qu'il rougit. **Sauvegarder le fichier dans le scratchpad
  avant de le saboter**, et le restaurer par copie — jamais par git.
  Trois formes du même piège reviennent : un test qui **inspecte l'état** au
  lieu d'emprunter le vrai chemin de code, qui **fixe une variable** du scénario
  au lieu de la sonder (`libres()[0]`), ou qui se compare à **sa propre
  constante** (§F.7). La parade est toujours la même : muscler le test dès qu'un
  sabotage ne mord pas, jamais l'admettre « suffisamment proche ».
- ⚠️ **Un test rouge ne veut pas dire que le code a tort** (L2.4).
- ⚠️ **Un proxy qui ne tient que dans un régime n'est pas une mesure** (§F.2).
- ⚠️ **Une propriété d'accord entre deux appelants d'un même calcul ne garde pas
  ce calcul — elle garde son unicité.** Il faut un **second** test qui pinne la
  valeur.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent
  d'une valeur qui en a deux (`docs/suivi/stations.md` §C.29).
- **Une cote qui se règle contre une autre pièce ne tombe pas sur `Profil`** —
  vu quatre fois. Réponse : mise à l'échelle **géométrique**, ou rester sur la
  grille et s'arrêter au cran qui tient (§F.4).
- **Brique d'abord, assemblage ensuite.** Jugée seule avant d'être posée.
- ⚠️ **La tolérance d'un test doit être plus petite que l'erreur qu'il cherche.**
  Vu le 2026-08-06 : le placement d'un hublot était borné par l'écart tangentiel
  de la paire (0,62) alors que l'erreur à détecter valait 0,14 — poser le vitrage
  au mauvais rayon passait au vert. La parade : mesurer sur un axe où le bruit
  n'existe pas (ici, la projection sur l'azimut), pas élargir la borne.
- ⚠️ **Comparer une copie à une copie ne prouve rien.** Quatre fois dans la même
  session : masses recopiées dans le test, corde *supposée* axiale au lieu d'être
  lue, ventre auto-référent (bardage comparé aux volets, jamais à une référence),
  commentaire de `braquage` que rien ne vérifiait. Toujours comparer à **la
  source**.
- ⚠️ **`dead_code` ne sait pas ce qu'est un test.** Il compte sur la cible
  binaire : tout ce que seuls les tests appellent y ressort mort. Mesuré le
  2026-08-05 — 90 morts pour le binaire, **13** pour les deux cibles. Conclure
  sans faire la différence mène à supprimer du code testé et qui marche.
- **Mesurer avant d'affirmer.**
- **Mettre à jour les `.md`** à chaque étape — consigne debout de l'utilisateur.

## Dette des bouche-trous de l'interface

Consigne de l'utilisateur (2026-08-04) : **poser un bouche-trou** pour ce qui
n'est pas encore fait, et **inscrire ici** ce qu'il reste à faire. Un
bouche-trou non listé est un mensonge à l'écran — il a l'air fini.

**Quatre sur cinq sont soldés.** Il ne reste que D-INT-2, l'économie, qui est un
chantier de conception à part entière.

| # | Bouche-trou | Où | Ce qu'il faut à la place |
|---|---|---|---|
| **D-INT-1** | ~~Pastille de catégorie~~ — **soldée** : la pastille prend la **teinte réelle** du corps (`Astre::teinte`, tirée de l'apparence), relevée pour rester visible sur fond nuit. Une retouche d'apparence se voit aussitôt dans la colonne. Reste un disque, et c'est voulu : à 6–14 px de diamètre, un rendu de planète serait de la bouillie. | — | — |
| **D-INT-2** | **`Tresorerie` figée** : les quatorze quantités sont écrites à la main, rien ne les fait bouger. | `ecran/bandeau.rs::Tresorerie` | L'économie : production, consommation, coûts, recherche. Chantier de **conception** à part entière ; la barre a été écrite pour pouvoir l'attendre (`conception/interface.md` §2.2b). |
| **D-INT-3** | ~~Vignette du panneau~~ — **soldée** : l'astre est **réellement rendu** (`ecran/vignette.rs`) dans une cible 192×192 avec son propre tampon de profondeur, sous l'éclairage du système, caméra placée du côté éclairé et reculée pour laisser tenir un anneau. | — | — |
| **D-INT-4** | ~~Nom du système~~ — **soldée** : dérivé de l'étoile hôte (`Systeme::nom_systeme`). Repli sur le libellé de génération pour un système engendré, dont les étoiles n'ont pas de nom propre. | — | — |
| **D-INT-5** | ~~Noms de presets non gardés~~ — **soldée** : `ajouter_planete`/`ajouter_planete_autour` exigent un `Option<&'static str>`, comme `ajouter_lune_preset`. Chaque appelant doit dire `Some("Terre")` ou `None`. Deux tests lisent la **source** pour interdire le retour du `sys.nommer` après coup — seul moyen, faute de pouvoir bâtir un système en test. | — | — |

## Dettes préexistantes

- **Le hash CPU/GPU** (traité le 2026-08-02) : `cargo clippy` ne compilait pas à
  cause d'`approx_constant` sur le multiplicateur du hash de `terrain.rs`.
  **Suivre le conseil de clippy aurait introduit un défaut** — `FRAC_1_PI`
  décale la valeur d'**1 ULP**, et le `fract` final amplifie ça en un bruit
  entièrement différent : la géographie CPU cesserait de correspondre au shader,
  sans aucun signal. Constante nommée + `allow` avec l'explication.
  **Dette restante** : la même valeur vit dans 4 fichiers (1 Rust + 3 GLSL) et
  rien ne les tient d'accord → `docs/suivi/planetes.md` §7.
- ⚠️ **Cette entrée disait « les 86 avertissements clippy ne contiennent aucun
  défaut réel ». C'était faux**, et la revue P1 (2026-08-05) l'a montré :
  `MASSES_TERRESTRES` se documentait comme la source unique des masses
  planétaires et **personne ne la lisait** — les masses étaient recopiées à trois
  endroits, et le test de Kepler vérifiait *sa propre copie*. Le défaut était
  visible depuis le début, noyé dans le bruit. **`clippy` est à 0 aujourd'hui**,
  et c'est ce qui rend le prochain visible.
- **`src/ecran/briques.rs` et `src/ecran/vaisseaux.rs` sont commités mais pas
  déclarés** dans `ecran/mod.rs` : ils ne compilent pas. À supprimer ou à
  rebrancher.
- Dette de **L1.4** : ~19 variantes sous-déclarent leur `rayon_local` (cadrage
  caméra ; 2 vues sur 35 bougeraient, de 1 % et 2 %). `MARGE_RAYON = 1.40`
  marque la dette dans le code. **Se solde en Lot 6.**
- Dette de **L1.6** : `peut_poser` n'a aucun test dédié, faute de consommateur.
  Son vrai consommateur est le code couleur de §8.4 — donc **Lot 4**.
- `GenrePort::PoutreBout` n'est posé sur aucun composant — variante morte,
  **Lot 6**.
