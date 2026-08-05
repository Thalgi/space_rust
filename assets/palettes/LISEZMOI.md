# Palettes de quantification

Tout fichier **`.hex`** déposé ici est ramassé au démarrage du jeu et apparaît
dans **PARAMETRES → PALETTE**, à la suite des palettes intégrées.

## Format

Un hexadécimal par ligne — c'est exactement le format d'export **`.hex` de
Lospec**, donc un fichier téléchargé s'utilise tel quel :

```
2e222f
3e3546
625565
```

Tolérances : le `#` de tête est accepté (`#2e222f`), la casse est indifférente,
les lignes vides sont ignorées, et les commentaires commencent par `;` ou `//`.

Le **nom du fichier** devient le nom affiché dans le menu, en majuscules :
`endesga_32.hex` → `ENDESGA_32`.

## Limites

- **256 couleurs au maximum** (`palette::MAX`), c'est la taille du tableau
  d'uniformes du shader. Une palette plus longue est refusée avec un message,
  pas tronquée en silence. ⚠️ Le message part dans la **console**, pas à l'écran :
  une palette déposée qui n'apparaît pas au menu a probablement été rejetée.
- Le coût de rendu monte avec le nombre de couleurs : la recherche parcourt
  toute la palette **par pixel**. Une palette de 182 coûte presque trois fois
  une de 64.
- Le **nom de fichier** peut faire doublon avec une palette intégrée :
  `resurrect-64.hex` donne « RESURRECT-64 » à côté de « RESURRECT 64 ». Les deux
  fonctionnent, mais autant supprimer le fichier.
- **2 couleurs au minimum.**
- Un fichier illisible est **signalé dans la console et ignoré** : il n'empêche
  pas le jeu de démarrer.

## Ce qui rend une palette bonne pour de la 3D

Une palette d'artiste est faite pour qu'on choisisse ses teintes à la main, pas
pour quantifier un ombrage continu. Mesuré sur Resurrect 64 : un dégradé de gris
ne tombe que sur **8 couleurs**, avec une marche qui saute de L=49 à L=69.

Les palettes qui rendent le mieux ici sont celles qui ont une **rampe de
clartés fournie**, surtout dans les tons neutres et clairs. À défaut, le
**tramage** (PARAMETRES → TRAMAGE) compense en mélangeant spatialement deux
teintes voisines — c'est à ça qu'il sert.
