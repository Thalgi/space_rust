# Rendu pixel art : basse résolution + palette 64 couleurs

Source de départ : le guide *« Rendu 3D Pixel Art Crisp avec Macroquad, Rust &
Palette Personnalisée (64 couleurs) »* fourni le 2026-08-04. Ce document en
garde la substance, **corrigée de ce qui ne marchait pas ici**, et note les
décisions prises pour l'intégrer au jeu existant.

---

## 1. Pourquoi la palette, et pas seulement les gros pixels

Le jeu savait déjà réduire la résolution : depuis longtemps, `rendu.rs`, la
galerie et `ecran/pixel.rs` rendent la scène 3D dans une cible basse résolution
puis la remontent au plus proche voisin. Ça donne de **gros pixels** — et rien
de plus.

Ce qui manquait, c'est la **seconde étape** :

| Étape | Effet | État avant ce chantier |
|---|---|---|
| Sous-échantillonnage + plus proche voisin | Gros pixels, bords nets | Déjà là (touche P) |
| Quantification vers une palette fixe | **À-plats** de couleur | Nouveau |

Sans la quantification, l'éclairage 3D produit des dégradés RGB continus : on
obtient de la 3D basse résolution façon 1996, pas du pixel art. Le regroupement
des ombres en à-plats (*color banding*) est ce qui donne la signature.

## 2. Pourquoi CIELAB

Chercher la couleur la plus proche **en RGB** revient à mesurer dans un espace
dont les axes ne pèsent pas ce que l'œil perçoit. CIELAB est construit pour que
la distance euclidienne approche l'écart *perçu*.

Ce n'est pas une préférence de principe : `le_choix_lab_differe_du_choix_rgb`
balaye le cube RGB et vérifie que les deux méthodes **choisissent réellement
différemment**. Si un jour elles s'accordaient partout, la conversion serait un
coût pur et le test le dirait.

L'optimisation du guide est conservée : la palette est convertie en LAB **une
fois, sur le CPU** (`src/palette.rs`). Le shader ne convertit que le pixel
courant — une conversion par pixel au lieu de soixante-cinq.

## 2 bis. Les deux défauts constatés à l'écran, et leur cause commune

Au premier essai visuel, deux choses cassaient l'image :

1. **les couleurs basculaient trop vite** entre elles, par régions entières ;
2. **le reflet de la lumière devenait un aplat blanc** bien trop large.

Ce ne sont pas deux bugs mais **un seul fait mesuré** : *une palette d'artiste
n'est pas une rampe de dégradé*. Sur Resurrect 64, un dégradé de gris ne tombe
que sur **8 couleurs** :

| Entrée | Couleur obtenue | L |
|---|---|---|
| 0 % | `#2e222f` | 15,1 |
| 12 % | `#313638` | 22,2 |
| 30 % | `#374e4a` | 31,2 |
| 35 % | `#625565` | 37,9 |
| 47 % | `#7f708a` | 49,4 |
| **51 %** | **`#9babb2`** | **69,0** |
| 77 % | `#c7dcd0` | 85,9 |
| **89 %** | **`#ffffff`** | **100,0** |

Deux marches expliquent tout :

- **47 % → 51 %** : 4 points d'entrée font sauter la clarté de **L=49 à L=69**.
  Quand le terminateur d'une planète balaie la surface, toute une bande franchit
  ce seuil *en même temps* → le basculement observé.
- **au-delà de 89 %** : tout s'écrase sur le blanc pur. Or le spéculaire des
  océans est **additif** (`planete.frag.glsl` : `col += vec3(1,0.97,0.9) * spec
  * wet * diff`) et monte bien au-dessus de 1,0 : tout son halo franchit le
  seuil d'un bloc → l'aplat blanc.

La racine : **au-dessus de L=70, Resurrect 64 n'a que deux couleurs quasi
neutres** (`#c7dcd0` et `#ffffff`). C'est normal — ces palettes sont faites pour
qu'un dessinateur choisisse ses teintes, pas pour quantifier un ombrage continu.

### Remède 1 — tramage ordonné (Bayer 8 × 8)

Un seuil pris dans une matrice de Bayer est ajouté à la couleur **avant** la
recherche. Deux entrées voisines de la palette se mélangent alors spatialement,
ce qui restitue les teintes intermédiaires : la transition devient progressive
au lieu de basculer d'un bloc.

Trois points de mise en œuvre comptent :

- **La matrice est une texture, pas un tableau d'uniformes.** Même contrainte
  qu'au §3.1 : l'indice de tramage se calcule depuis la position du pixel, donc
  ce n'est pas une constante.
- **L'indexation se fait sur la grille de la *cible***, pas de l'écran
  (`uv * taille_cible`). Indexé sur les pixels écran, le motif serait `PIX` fois
  plus fin que les gros pixels — invisible et inutile.
- **`FORT` vaut 0,18** parce que c'est la taille mesurée de la pire marche
  (0,50 → 0,68 en gris). En deçà, le tramage ne la traverse pas et la bande
  bascule quand même.

`le_tramage_restitue_des_teintes_intermediaires` compare, sur une même rampe, le
nombre de couleurs obtenues avec et sans.

### Remède 2 — écrêtage des hautes lumières

Ce qui dépasse `ECRETAGE_SEUIL = 0,72` est comprimé de `ECRETAGE_FORCE = 0,30`,
**à teinte constante** (les trois composantes divisées par le même facteur).

Le dosage est délibéré : une entrée de 1,0 ressort à 0,80, donc **sous** le seuil
du blanc, tandis qu'un point chaud à 1,5 ressort à 0,95 et reste blanc. Autrement
dit **le cœur du reflet reste blanc, son halo ne l'est plus**. Le but est de
réduire l'aplat, pas de supprimer le reflet — c'est ce que vérifie
`lecretage_calme_le_halo_mais_garde_le_coeur_du_reflet`.

⚠️ **L'écrêtage n'agit que dans la passe pixel art.** Le spéculaire du shader de
planète n'est pas touché : le modifier changerait le rendu `NET`, donc la galerie,
les presets et les captures de non-régression. Si le reflet paraît trop fort en
`NET` aussi, c'est un autre chantier, au rayon d'action bien plus large.

## 2 ter. Le troisième défaut : tout ressortait terne

Constat à l'écran (Terre, Resurrect 64) : **les océans gris-violet**, et
l'ensemble délavé — « on ne voit plus vraiment de couleur », et ce **quelle que
soit la palette**.

### Ce que ce n'était pas

Deux hypothèses écartées par la mesure, et c'est important de le noter parce que
toutes deux étaient plausibles :

- **La recherche CIELAB n'y est pour rien.** À poids égaux elle conserve déjà
  **85 %** de la chroma, et pondérer l'axe L ne change quasiment rien
  (84,9 % → 86,7 % au mieux). Les couleurs d'océan tombent bien sur de vrais
  bleus (`#484a77`, `#4d65b4`).
- **Le tramage n'y est pour rien** non plus : il déplace la chroma moyenne de
  28,3 à 26,5 au pire, et la **remonte** dans d'autres cas.

### Ce que c'était

**Les entrées neutres de la palette sont des attracteurs pour les couleurs peu
saturées.** Une planète est voilée par son atmosphère : ses couleurs n'ont
qu'une chroma modérée, et les gris de la palette sont alors les plus proches
voisins en CIELAB.

| Entrée | Chroma | Sortie | Chroma |
|---|---|---|---|
| forêt voilée | 17,6 | `#374e4a` | **9,8** |
| terre voilée | 9,7 | `#966c6c` | 18,0 |
| rampe de Terre (12 pts) | 15,2 | dont **3 × `#625565`** | 8,0 |

`#625565` est précisément le gris-violet vu à la place des océans.

⚠️ **Le défaut n'est pas une baisse de la chroma moyenne** — mesurée, elle monte
même un peu (15,2 en entrée → 18,9 en sortie). C'est la **dispersion** qui pose
problème : une partie de l'image tombe sur des neutres, et l'œil ne voit que ça.
C'est une assertion que j'avais écrite à tort, et que le test a démentie ; elle
a été remplacée par un comptage des sorties quasi neutres.

### Remède — raviver la chroma avant de quantifier

Un gain de chroma **à luminance constante** (on s'écarte du gris de même clarté,
l'ombrage n'est donc pas touché) pousse la couleur sur les **rampes colorées** de
la palette au lieu de ses neutres. C'est d'ailleurs ce que fait un dessinateur :
il ne peint pas avec les teintes intermédiaires ternes.

Mesuré à ×1,9 : chroma moyenne en sortie **18,9 → 28,9** (+53 %), et l'océan
passe de `#625565` à `#4d65b4`.

### Et le gain retombe dans les hautes lumières

`SAT_HAUTES = 0,5` sur la bande de luminance 0,50 → 0,82. Deux raisons :

- **un reflet est achromatique** dans la réalité ;
- sans ça, le halo du spéculaire — qui vit vers 0,75-0,85 de luminance — reste
  coloré, et comme la palette n'a que des entrées **chromatiques** vers L≈82-91
  (`#8fd3ff`, `#8ff8e2`), il tombe sur un cyan franc. C'est l'anneau cyan visible
  autour du point chaud sur la capture.

La bande a d'ailleurs dû être abaissée de (0,55 ; 0,90) à (0,50 ; 0,82) : au
premier réglage, le halo gardait encore un gain de 0,81 et restait cyan — c'est
le test qui l'a montré.

### L'ordre des étapes

```
saturer  →  écrêter  →  tramer  →  quantifier
```

Il n'est pas indifférent : la saturation travaille sur la couleur telle que la
scène l'a produite ; l'écrêtage a besoin de voir les dépassements **au-dessus de
1** pour distinguer le cœur d'un reflet de son halo ; le tramage vient en
dernier, juste avant la recherche.

## 3. Corrections apportées au guide

Le code du guide ne fonctionne pas tel quel dans ce projet. Quatre points :

### 3.1 Indexation dynamique d'un tableau d'uniformes (bloquant)

Le guide écrit :

```glsl
int bestIndex = 0;
...
gl_FragColor = vec4(palette_rgb[bestIndex], texColor.a);
```

GLSL ES 1.00 n'autorise l'indexation d'un tableau d'uniformes que par une
**« constant-index-expression »**. Un indice de boucle en est une ; une variable
calculée dans la boucle, non. On retient donc la **couleur**, pas l'indice :

```glsl
vec3 meilleure = palette_rgb[0];
for (int i = 0; i < TAILLE; i++) {
    ...
    if (d2 < dmin) { dmin = d2; meilleure = palette_rgb[i]; }
}
```

### 3.2 API `UniformDesc::array` (bloquant)

Le guide appelle `UniformDesc::array(UniformType::Float3, PALETTE_SIZE)`. La
signature réelle en macroquad 0.4.15 prend un descripteur **déjà nommé** :

```rust
UniformDesc::array(UniformDesc::new("palette_lab", UniformType::Float3), palette::TAILLE)
```

### 3.3 Mélange alpha absent (aurait masqué tout le décor)

`MaterialParams::pipeline_params` a `color_blend: None` par défaut : l'alpha est
ignoré. Or **ici la cible est nettoyée en transparent** et composée par-dessus le
fond stellaire net — sans mélange explicite, le vide de la cible serait écrit en
opaque et effacerait le fond, les orbites et les zones. Le matériau déclare donc
un `BlendState` alpha classique.

Le guide n'avait pas le problème parce qu'il rend une scène opaque plein écran ;
c'est notre choix de composition (§4) qui le crée.

### 3.4 Attributs du vertex shader

Le vertex shader du guide déclare `position/texcoord` seulement. macroquad pousse
`position, texcoord, color0, normal` et résout les attributs **par nom**. Le
vertex shader par défaut de macroquad est donc recopié intégralement dans
`ecran/pixel.rs`.

### 3.5 Deux économies ajoutées

- **Distance au carré** au lieu de `distance()` : la racine est monotone, elle ne
  change pas le gagnant — 64 racines par pixel économisées. `palette.rs` compare
  de la même façon.
- **`discard` sur les pixels transparents** avant la boucle. La cible est
  très majoritairement vide (l'espace) : ça épargne 64 comparaisons sur la plus
  grande partie de l'écran.

## 4. Ce qui est quantifié, et ce qui ne l'est pas

**Seule la couche 3D** passe par la palette. Le fond stellaire, les orbites, les
textes, la barre de ressources et le sélecteur restent nets et hors palette.

C'est la convention déjà en place dans le jeu (« le fond stellaire et les textes
restent nets »), et elle a une raison : la police est fine et cyan, une
quantification la rendrait sale sans rien gagner. Quantifier **tout l'écran**
reste possible plus tard — ce serait une passe supplémentaire après l'UI — mais
ce n'est pas ce qui est fait.

## 5. Où ça vit

| Fichier | Rôle |
|---|---|
| `src/palette.rs` | Les palettes, sRGB → CIELAB, matrice de Bayer, miroir CPU testable |
| `src/shaders/palette.frag.glsl` | Écrêtage + tramage + quantification, côté GPU |
| `src/ecran/pixel.rs` | Facteur `PIX`, taille de cible, **le blit**, le matériau |
| `src/reglages.rs` | `ModeRendu`, `Tramage`, l'état global lu par le code de dessin |
| `src/ecran/parametres.rs` | Les lignes de menu |
| `assets/palettes/*.hex` | Palettes ajoutées sans toucher au code (voir §6 bis) |

`ecran/pixel.rs` est devenu **la source unique du blit** : `rendu.rs`, la
galerie, les stations et les briques passent tous par `pixel::blit`. C'est ce qui
fait que la quantification s'applique partout sans être décidée à quatre
endroits. Le facteur `PIX` et la création de cible (`depth: true`, filtre
`Nearest`) y sont également centralisés — ils étaient recopiés trois fois.

## 6. Le réglage

Trois états, dans **PARAMETRES → RENDU** :

| Mode | Cible basse résolution | Palette |
|---|---|---|
| `NET` | non | non |
| `PIXEL ART` | oui | non |
| `PIXEL ART + PALETTE 64` | oui | oui |

Trois et pas deux : le filtre gros pixels existait déjà et reste utile seul ;
la palette s'empile dessus. `quantifier_implique_pixeliser` interdit la
combinaison « palette sans gros pixels », qui donnerait des à-plats en pleine
définition — ce qui n'est pas du pixel art.

Deux réglages s'ajoutent quand le mode palette est actif, et se **grisent**
sinon (les laisser cliquables ferait croire à un effet qu'ils n'ont pas) :

| Ligne | Valeurs |
|---|---|
| `PALETTE` | les palettes intégrées, puis celles d'`assets/palettes/` |
| `TRAMAGE` | `NON` / `LEGER` (0,08) / `FORT` (0,18, défaut) |
| `SATURATION` | `NON` (×1,0) / `MOYEN` (×1,45) / `FORT` (×1,9, défaut) |

Les deux défauts sont à `FORT` parce que ce sont les réglages qui corrigent les
défauts constatés à l'écran (§2 bis et §2 ter), et que leurs valeurs sont
**calées sur des mesures**, pas sur un goût.

**Le mode est global, pas par vue.** Il vit dans un `thread_local` de
`reglages.rs`, comme `disque::set_viewport_h` et `planete::set_viewport_h` le
font déjà pour l'état de rendu transversal. Les touches **P** des différentes
vues cyclent ce même état : le menu et le clavier ne peuvent plus se contredire,
et les trois booléens `pixelise` séparés ont disparu.

⚠️ La touche **P** ne change **que le mode** — elle ne remet ni la palette ni le
tramage à leur valeur par défaut au passage (`le_raccourci_ne_touche_quau_mode`).

## 6 bis. Ajouter une palette

Deux chemins, au choix :

- **Un fichier** `assets/palettes/<nom>.hex` — un hexadécimal par ligne, c'est le
  format d'export de Lospec, donc un fichier téléchargé s'utilise tel quel. Le
  nom du fichier devient le nom au menu. Ramassé au démarrage, trié, et un
  fichier illisible est signalé puis ignoré : le jeu démarre quand même.
  Détails : [`assets/palettes/LISEZMOI.md`](../../assets/palettes/LISEZMOI.md).
- **Une constante** dans `palette.rs`, ajoutée à `INTEGREES` — toujours
  disponible, même sans le dossier d'assets. C'est le cas des trois livrées :
  Resurrect 64, Sweetie 16 et PICO-8.

Les palettes n'ont **pas toutes la même longueur** : le shader reçoit
`nb_couleurs` et s'arrête là, le tableau d'uniformes restant dimensionné à `MAX`
(64). Sweetie 16 et PICO-8 sont là pour que ce chemin « palette courte » soit
réellement parcouru, et pas seulement prévu.

## 7. Ce qui est testé, et ce qui ne peut pas l'être

34 tests, tous red-checkés. Ils couvrent le décodage hexa et le format `.hex`,
les repères connus de CIELAB (blanc L=100, gris 50 % → L≈53 et non 50, ce qui
prouve la dé-gammatisation), l'idempotence de la quantification sur les couleurs
de chaque palette (qui prouve d'un coup l'exactitude de la recherche **et**
l'atteignabilité de toutes les entrées), l'appartenance de toute sortie à la
palette, le plafond `MAX`, le fait que la matrice de Bayer est bien une
permutation *et* bien celle de Bayer (première ligne connue), le gain réel du
tramage, le dosage de l'écrêtage, les cycles et la cohérence du menu.

⚠️ **Aucun test ne compile ni n'exécute le shader** — il faudrait un contexte GL.
Voir la dette D-PIX-1 ci-dessous. Ce que les tests garantissent, c'est que
l'algorithme de référence est juste ; que le GPU le suive se vérifie à l'œil.

## 7 bis. Aucun rejet silencieux

Un `.hex` du dossier qui ne charge pas est **conservé avec sa raison**
(`palette::Rejet`) et affiché dans PARAMETRES, en ambre, sous le menu.

Ce n'est pas du confort : c'est le défaut qui a coûté le plus cher. Le plafond
était à 64 couleurs, `lospec-2000.hex` (182) et `allstars.hex` (128) étaient
refusées, et la seule trace partait dans une console. Côté utilisateur : « j'ai
déposé la palette, elle n'est pas là, et les noms du menu sont faux ».

Le découpage suit la règle du projet — ce qui décide sort du code qui dessine :

| Fonction | Rôle | Testable |
|---|---|---|
| `lire_dossier` | lit le disque, ne décide de rien | non (E/S) |
| `trier` | classe en palettes et rejets | **oui**, pure |
| `lignes_de_rejet` | quoi écrire, combien en détailler | **oui**, pure |

Un fichier fautif n'empêche **jamais** les autres de charger, et le total est
toujours annoncé même quand la liste est tronquée — sinon on croirait n'avoir
qu'un seul problème.

## 8. Dettes

| Dette | Quoi |
|---|---|
| **D-PIX-1** | `palette.rs` (CPU, `#[cfg(test)]`) et `palette.frag.glsl` sont **deux écritures du même algorithme**, et rien ne force leur accord. Le miroir CPU couvre maintenant l'écrêtage et le tramage aussi, donc la surface de divergence a **grandi**. À surveiller de pair : la distance **au carré**, `MAX`, la formule d'écrêtage et l'indexation du tramage. |
| **D-PIX-3** | Pas de contour (*outline*) au passage Sobel/profondeur, mentionné par le guide comme amélioration. |
| **D-PIX-4** | `PIX = 2` est figé. Un réglage « taille des pixels » serait une ligne de menu de plus. |
| **D-PIX-5** | Le tramage est en **espace écran**, donc fixe pendant que les objets bougent : c'est le choix rétro habituel, mais sur un objet qui tourne le motif peut « ramper ». L'alternative (tramage ancré à l'objet) demanderait une coordonnée stable par surface, que le blit n'a pas. |

**D-PIX-2 est soldée** : les palettes se choisissent au menu et s'ajoutent par
fichier (§6 bis).

## 9. Limites connues

- **Uniformes** : 128 `vec3` (64 LAB + 64 RGB), quelle que soit la palette
  chargée. Confortable sur GL de bureau ; ce serait à repenser (palette en
  texture) pour une cible mobile/WebGL stricte. Ils ne repartent vers le GPU
  qu'au **changement** de palette, pas à chaque frame.
- **Coût** : jusqu'à `nb_couleurs` comparaisons par pixel non transparent, à la
  résolution de la cible — donc au quart des pixels de l'écran avec `PIX = 2`.
  Une palette de 16 coûte quatre fois moins qu'une de 64.
- Si le shader ne compile pas, le jeu **ne s'arrête pas** : il journalise une
  erreur et blitte sans quantification, une seule fois (pas de recompilation à
  chaque frame).
