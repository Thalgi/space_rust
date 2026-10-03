# Journal — chantier Astres V2

Journal du chantier décrit par
[`conception/astres_v2.md`](../conception/astres_v2.md), lot par lot.

**Où se trouve le travail** : branche `chantier-astres`, dans le worktree
`.claude/worktrees/astres-v2`. **Rien n'est fusionné dans `main`**, rien
n'est poussé. 32 commits d'avance sur `main` au 2026-09-26.

**État au 2026-10-03** : 904 tests verts, clippy à zéro. **Le lot 5 est clos**
(ses sept presets de trou noir sont au tirage), **le 6 est livré** (supergéante
rouge, sous-naine sdB, étoile carbonée, naine brune) et **le 7 aussi** : la
pulsation, donc céphéide, Mira et ZZ Ceti. Tout est **à juger à l'écran**.

Deux dettes restent, petites et consignées plus bas : le disque passe sous
l'horizon dessiné à très longue distance (`PX_MIN`), et la convention de
silhouette « mince » n'a jamais été essayée.

---

## Où en est chaque lot

| Lot | État | Commit |
|---|---|---|
| 0 — masse dérivée de la luminosité | fait | |
| 1 — graine par étoile, mondes glacés | fait | |
| 2 — plancher en pixels du plasma | fait | |
| 3 — flares, arches ancrées, activité par type | fait | |
| 3 bis — arches espacées, taches non rondes | fait | |
| **3.1** — le champ magnétique | fait | `8761ca7` |
| **3.2** — taches sur les maxima, en groupes | fait | `ca6c1bb` |
| **3.3** — pieds sur deux polarités, répulsion | fait, **+ 4 correctifs** | `672f578`, `7ee33a6`, `f6ea783`, `9478798`, `1d78aac` |
| **3.4** — le shader lit le champ | fait | `f6e05f2` |
| **3.5** — le ruban, l'ancrage des pieds | fait, **+ 2 correctifs** | `2840050`, `c4edc55`, `bd677da` |
| **4** — le trou noir atteignable au tirage | fait | `4160e49` |
| **5** — le disque d'accrétion a un plan | fait | `f43691a` |
| **5.1** — shader dédié | fait | `8c2cec6` |
| **5.2** — le pli | fait | `f4a0e2d` |
| **5.3** — l'aspect, corrigé sur mesures | **fait et validé à l'écran** | `c32e3bf` + 7 correctifs |
| **5.4** — la cible Interstellar, à comparer | fait, jugée « très satisfaisante » | `689b8f4` |
| **5.5** — la cible froide déclinée en appétits, plus la braise | fait | `689b8f4` |
| **5 — clos** : les sept presets au tirage | **fait et tranché** | |
| **6.1** — supergéante rouge, sous-naine, carbonée au tirage | fait, à juger à l'écran | `05bef4e` |
| **6.1 bis** — le défilé de la vue isolée couvre les ordinaires | **fait** | |
| **6.2** — la naine brune comme **centre** d'un système | **fait**, à juger à l'écran | |
| **6.2 bis** — ses quatre classes dans la planche des étoiles | **fait** | |
| **7** — pulsation : céphéide, Mira, ZZ Ceti | **fait**, à juger à l'écran | |
| **8.1** — le ciel en cubemap | **abandonné**, code retiré | `7f6e1aa` |
| **8.2** — la déviation dans le shader | **abandonné**, code retiré | `dd55404`, `5e66b10` |
| **9.1** — les comètes au tirage, et leur planche | **fait**, à juger à l'écran | |
| **9.2** — la queue | **fait**, à juger à l'écran | |
| 10 à 13 | à faire | |

Deux correctifs hors lot : le disque manquant de T Tauri (`3e2eb99`) et le
`crash.log` sorti du dépôt.

---

## Ce qu'il faut savoir pour reprendre

### ⚠️ Une taille de texte non bornée pouvait tuer le jeu (2026-10-03)

Remonté depuis la console :

```text
miniquad-0.4.10\src\graphics\gl.rs:181: assertion `left == right` failed
  left: 0
 right: 4294967296
```

**Lu, pas deviné.** Cette ligne est `Texture::new`, qui vérifie
`format.size(w, h) == bytes.len()`. 4 294 967 296, c'est **2³²** : la seule
texture qui donne ce couple est une **32768 × 32768**, dont `w * h * 4` déborde
`u32` et vaut 0 en release — tandis que son tampon, lui, fait bien 4 Gio.

Une seule chose double de taille dans tout le programme : l'**atlas de glyphes**
de macroquad (`text/atlas.rs`). Quand un glyphe n'y tient pas, il double ses deux
côtés et recommence, en toutes lettres dans son commentaire : *« if we tried to
fit gigantic texture into a small atlas, new_width will still be not enough »*.
512 → 1024 → … → 32768, et miniquad meurt.

**Pour qu'un glyphe soit gigantesque, il faut une taille gigantesque.** Or
`preparer` faisait `(taille * ECHELLE).round().max(1.0) as u16`, et `as u16`
**sature** : une hauteur divisée par un compte nul rend l'infini, qui devient
65535. Un glyphe de 65535 px, et le jeu meurt au lancement.

⚠️ `TAILLE_MIN` et `TAILLE_MAX` existaient **depuis la correction des carrés
noirs**, et le commentaire du module disait déjà *« un `clamp` posé à l'œil dans
un écran a suffi à faire graver une taille en pleine frame »*. La leçon avait été
tirée, la conclusion pas : chaque vue restait priée de se borner elle-même. Le
bornage est désormais **dans `preparer`**, le point de passage commun de `texte`
et de `mesure` — là où il ne peut plus être oublié.

Et la garde existante, `les_tailles_du_jeu_sont_toutes_prechargees`, balayait une
**liste relevée à la main** des tailles demandées aujourd'hui : elle ne disait
rien de celle qu'une vue calculera demain, et c'est toujours une taille
**calculée** qui déborde. La nouvelle ne relève rien, elle balaie l'absurde —
zéro, négatif, NaN, les deux infinis, `f32::MAX`, une division par zéro.

**Non reproduit ici** : 40 s sur l'accueil avec le même binaire, sans incident.
Le trou est réel et bouché à sa source ; reste à savoir quelle vue calculait la
taille fautive, ce qu'une trace (`RUST_BACKTRACE=1`) dirait en une ligne.

### Lot 9.3 : cinq familles de comètes, et une galerie qui les compare (2026-10-03)

Signalé à l'écran, capture à l'appui : *« la queue d'ions n'est toujours pas
assez visible ou théâtralisée, c'est très léger. Je voyais plus ça comme une
comète un peu brillante et bleutée pour Halley. Il faut revoir la vue comète
pour mettre des exemples avec plusieurs colorations par rapport à la matière et
plusieurs types de queue. La vue comète avec des trajectoires n'est pas vraiment
ce que je recherchais à la base. »*

#### Pourquoi la queue restait grise

⚠️ **`ECLAT` multipliait l'activité, et l'activité plafonne.** Au périhélie,
`activite` vaut **0,44** — elle s'allume à trois périhélies et croît en
`(1 − d/seuil)²`. L'opacité valant l'activité, aucune comète ne pouvait dépasser
**44 % d'opacité**, quelle que soit sa matière et quoi qu'on règle par ailleurs.
C'est le voile gris de la capture.

Le facteur est désormais un **trait de la famille** (`Preset::eclat`, 1,4 à 2,6)
et il est plafonné à 1 : au cœur du plongeon, la queue est franche, et une comète
presque éteinte reste terne. Dette assumée sur les valeurs.

#### Le catalogue

| # | Famille | Noyau | Queues |
|---|---|---|---|
| 0 | Glacée (1P/Halley) | roche sombre, glace bleutée | poussière + ions |
| 1 | Poussiéreuse (Hale-Bopp) | brun sableux | poussière dorée |
| 2 | Carbonée (presque éteinte) | quasi noir, albédo 0,04 | poussière rousse |
| 3 | Ionique (Lovejoy) | gris-bleu | ions seuls |
| 4 | Verte au carbone C₂ (Lulin) | glace verdâtre | poussière + ions |

Le vert n'est pas une fantaisie : c'est le **carbone diatomique C₂** de la
chevelure, qui émet à 518 nm. Halley prend l'entrée 0 **explicitement** dans le
préset solaire ; les comètes tirées piochent leur famille dans leur graine —
dérivée, jamais tirée, sinon un `gen_range` de plus décalerait tout le système.

#### La queue d'ions

Une deuxième physique, pas une deuxième couleur : les ions sont emportés par le
champ magnétique du vent stellaire, cent fois plus vite que les grains. Sa queue
est donc **plus longue** (×1,4), **fine** (1 % à 4,5 % de sa longueur contre 4 %
à 22 %), **rigoureusement droite** et elle **s'éteint plus lentement**. Une queue
d'ions incurvée serait une queue de poussière mal peinte, et c'est le seul trait
qui sépare les deux à l'œil : une garde le tient.

#### La galerie refaite

⚠️ **Elle montrait des orbites, elle montre des comètes.** L'argument du premier
jet — un noyau fait quelques pixels et se ressemble d'un exemplaire à l'autre —
était vrai du **noyau** et faux de la **comète** : ce qu'on vient voir, c'est la
queue. Chaque case pose maintenant la même comète au même point de son plongeon
et ne change **qu'une chose**, sa famille : toute différence visible vient donc
de la matière, de rien d'autre.

Le cadrage est **calculé** à partir des rubans eux-mêmes (`etendue_queues`), et
non réglé à la main : la queue d'ions est la plus longue, et un cadre fixé sur
« à peu près deux unités » la couperait au premier réglage de `comete`.

#### ⚠️ Une garde avec une échappatoire

`les_familles_se_distinguent_a_l_oeil` acceptait d'abord deux familles identiques
**dès lors que leurs types de queue différaient** (`|| a.queues != b.queues`). Un
sabotage qui recopiait les couleurs de « Glacée » dans « Poussiéreuse » l'a
laissé **vert**. Or ce que la planche promet est la coloration **selon la
matière** : deux cases de la même couleur ne l'illustrent pas, quoi qu'elles
portent. La garde exige maintenant les deux écarts à la fois, sans échappatoire.

C'est la même famille de faute que les tests qui se trouvent eux-mêmes : une
disjonction de plus dans un `assert!` est une porte de sortie, et un catalogue de
cinq gris serait passé par là.

### ⚠️ La courbure était là et ne se voyait pas (2026-10-03)

Signalé à l'écran : *« la queue, je pense, doit être en relation avec la
trajectoire de la comète ; ce n'est pas le cas ici, on dirait que la queue est
poussée toujours vers l'extérieur du système par le vent solaire, à voir si
c'est à rectifier ou normal »*.

**La moitié de l'observation est normale, et c'est la bonne moitié.** Une queue
de comète est **anti-solaire** : elle fuit l'étoile quoi que fasse la comète. Sur
le trajet de retour, la comète voyage donc **queue devant**. C'est exact, c'est
contre-intuitif, et c'est précisément ce que `la_queue_fuit_l_etoile_et_non_la_trajectoire`
tient depuis le lot 9.2 — le sillage de bateau est l'erreur classique.

**L'autre moitié était un vrai défaut.** Le lien à la trajectoire existe : c'est
la **courbure** de la queue de poussière, les grains qui gardent leur orbite et
prennent du retard. Mesurée :

| réglage | déviation à mi-opacité | au dernier point |
|---|---|---|
| `0.45` sur `t²` (avant) | **4,7°** | 24° |
| `0.75` sur `t^1.3` (après) | **14,0°** | 37° |

À 4,7°, la queue se lit **droite** : une queue d'ions, pas de poussière. Toute la
courbure vivait **dans la partie transparente du ruban** — l'opacité décroît en
`(1-t)^1,1`, et le `t²` ne faisait décrocher les grains qu'au bout, où il ne reste
rien à voir.

⚠️ **Et `la_queue_de_poussiere_s_incurve_vers_la_trainee` restait vert**, parce
qu'il se contentait de constater que la courbure **existe** (`proche.dot(loin) <
0.999`). C'est mot pour mot la leçon de la lentille du lot 8 : *mes tests
garantissaient que l'effet était contenu, aucun ne disait qu'il devait se
**voir***. La garde neuve mesure la déviation **au point où la queue a encore la
moitié de son opacité**, jamais au bout du ruban — et elle borne par le haut, car
au-delà de 60° ce n'est plus le vent stellaire qui mène mais le sillage.

L'exposant est le réglage qui compte : il déplace la courbure vers le **corps
éclairé** de la queue sans la rendre constante — près du noyau, la poussière vient
d'être libérée et suit encore l'anti-solaire.

### La queue était un fil : deux échelles sans commune mesure, encore (2026-10-03)

Signalé à l'écran : *« me manque toujours la queue de la comète, au pire on la
rend extrêmement visible pour le moment au point que ce n'est pas naturel, car
je crois que son orientation a aussi quelques problèmes »*.

**Mesuré avant d'affirmer.** Halley au périhélie, UA = 48 unités monde :

| grandeur | valeur |
|---|---|
| distance à l'étoile | 28,1 |
| seuil d'activité | 84,3 |
| activité | 0,44 |
| **longueur de queue** | **20,0** |
| **demi-largeur au bout** | **0,47** |

Il y avait bien une queue. Elle faisait **1 de large pour 130 de long** : un fil
sub-pixel dès qu'on cadre le système. La largeur suivait le **rayon du noyau**
(0,08) quand la longueur suit la **distance à l'étoile**.

⚠️ **C'est exactement le défaut que `LONGUEUR_MIN_NOYAUX` avait déjà corrigé, à
l'autre bout de la même fonction** : deux grandeurs réglées sur deux échelles
qui n'ont aucun rapport l'une avec l'autre. Corriger la longueur ne l'avait pas
corrigé pour la largeur — une faute de ce genre vit par membre, pas par
fonction.

La largeur est désormais une **fraction de la longueur de la queue**
(`LARGEUR_BASE` → `LARGEUR_BOUT`, 4 % → 22 %), avec le rayon du noyau pour
simple plancher. Une queue de comète est un **cône**, et son ouverture ne sait
rien du caillou qui l'émet. La garde tient le **rapport**, la seule chose qui se
lise à l'œil, et non une largeur absolue — et elle le tient aux deux échelles,
la galerie et Halley.

**`ECLAT` à 1,0**, contre 0,5 : exagéré et demandé tel quel. Une vraie queue est
un voile qu'on ne photographie qu'en pose longue. **Dette assumée**, à baisser
quand le reste du rendu cométaire sera réglé.

**L'orientation, enfin** : la queue fuyait `Vec3::ZERO` et le noyau était
éclairé depuis l'origine. Les deux se confondent avec l'étoile dans le préset
solaire et dans la galerie, ce qui rendait le raccourci invisible — mais une
**binaire** met son barycentre à l'origine et ses étoiles ailleurs : la queue
fuyait un point où il n'y a rien, et le noyau était éclairé du mauvais côté.
`systeme::rendu::eclairage` porte déjà `cam.light_pos` ; la reprendre fait une
source unique au lieu de deux vérités. Pas de garde : c'est une couche de
dessin, et `docs/methode.md` écarte les tests de rendu.

### Un losange pour la comète : le même filtre, la deuxième fois (2026-10-03)

Signalé à l'écran : *« il faut lui mettre un marqueur à la comète car je ne la
vois pas, pourquoi pas un diamant à la place du triangle »*.

**La même cause que juste avant**, à un autre endroit :
`indices_planetes_racines` ne gardait que `Categorie::Planete`. Halley bougeait
désormais, mais sans repère à taille écran fixe elle faisait moins d'un pixel :
rien à voir, rien à cliquer.

C'est le **deuxième** filtre par catégorie que la comète traverse sans être vue,
après la boucle des rails. Ici, contrairement aux rails, il n'y a **rien à
interroger** : « mérite un repère de premier plan » n'est pas une capacité de
l'astre, c'est un choix d'interface. La liste reste donc explicite
(`marque_a_la_racine`), mais une garde la **balaie entièrement** : ajouter un
variant à `Categorie` fait rougir le test tant qu'on n'a pas tranché, là, s'il
porte un repère. La prochaine venue ne pourra plus se glisser muette.

**La forme dit le type.** La teinte était déjà prise : elle porte la sélection
(quatre niveaux de gris). Le `contour(cat, pointe)` est donc pur et rendu en
**données** — la comète reçoit un losange, tout le reste le triangle — et le
dessin n'a plus qu'un éventail depuis la pointe : trois points font un
triangle, quatre un losange, sans un cas de dessin par forme.

Le losange partage **la pointe et la boîte** du triangle (8 × 10 px) ; il écarte
seulement ses flancs à mi-hauteur. Ce n'est pas cosmétique : `RAYON_HIT_PX` est
réglé sur cette boîte, et une forme qui en sortirait se cliquerait moins bien
que ce qu'on voit. La garde tient la boîte en plus du nombre de sommets.

### ⚠️ La comète ne bougeait pas : la catégorie morte, par l'autre bout (2026-10-03)

Signalé à l'écran : *« la comète placée dans le système solaire n'a pas de
mouvement »*. Halley restait posée à son périastre, pour toujours.

**La cause**, dans `Systeme::update` :

```rust
if a.categorie() == Categorie::Planete { … a.maj_rail(f, t); }
```

La boucle « sur rails » supposait **« sur rails ⇒ planète »**. C'était vrai tant
que seules des planètes avaient un foyer. Dès que la comète est arrivée avec sa
**propre** catégorie (lot 9.1), elle a cessé d'être avancée.

⚠️ **C'est le piège de la catégorie morte retourné.** Jusqu'ici il se présentait
comme « une catégorie que rien ne produit » — le trou noir avant le lot 4,
`Comete` avant le 9.1. Ici c'est l'inverse : **rendre la catégorie juste a
suffi à exclure l'astre d'un traitement qui le concernait**. Le défaut n'est pas
dans la nouveauté, il est dans le filtre qui ne l'attendait pas.

La règle qui en sort, et elle vaut au-delà de ce lot : **un filtre doit porter
sur ce que l'astre sait faire, jamais sur ce qu'il est.** Le critère est
désormais `a.foyer().is_some()` — avoir une orbite autour d'un foyer — et
`maj_rail` est de toute façon inerte pour qui n'a pas d'orbite de Kepler.

La garde balaie **toutes** les catégories, pour qu'une nouvelle venue n'ait pas à
réapprendre la leçon, et vérifie aussi qu'un astre **sans** foyer n'est pas
appelé pour rien — sinon le vrai critère serait masqué.
### Le noyau et la queue, repris sur retour écran (2026-10-02)

Deux défauts signalés : *« pour le moment c'est une bille »* et *« la traînée de
la comète n'est pas visible »*.

**1. Le noyau était une sphère.** Il passait par l'impostor, comme une planète.
Un noyau de comète est un **patatoïde** : trop petit pour que sa gravité
l'arrondisse, et creusé par la sublimation qui emporte la glace par plaques.
`comete::noyau` rend désormais un maillage déformé par une somme de sinus,
**dérivé de la graine** — deux comètes diffèrent, la même en redonne une
identique.

⚠️ **Glace et roche sur un second bruit, sans rapport avec le premier.** Un bruit
unique pour la forme et la matière donnerait des creux systématiquement sombres —
une balle peinte, pas un caillou. Une garde **mesure la corrélation** entre
relief et clarté et refuse qu'elle dépasse 0,6 ; le sabotage qui fait calquer la
texture sur le relief la fait rougir.

⚠️ **Éclairé sur le CPU**, et c'est ce qui évite un pipeline de plus : deux cents
sommets, un lambert par sommet ne coûte rien, et macroquad dessine un maillage à
couleurs de sommets sans material. Zéro shader pour tout le lot 9.

**2. La queue mesurait moins que le noyau.** Mesuré : sa longueur valait
`0,55 × activité × distance`. Dans la galerie des comètes, où l'orbite tient en
3,5 unités, ça donnait **0,085 unité** au périhélie pour un noyau de rayon 0,09 —
plus courte que le caillou qui l'émet.

⚠️ **La cause est que les deux échelles n'ont aucun rapport.** La longueur suit la
distance à l'étoile, ce qui est juste ; mais le noyau a une taille **fixe et
exagérée** (un vrai noyau de 11 km serait sub-pixel). D'où un **plancher en
rayons de noyau** — vingt-cinq — en plus de la part proportionnelle. La queue
fait maintenant onze rayons de noyau dans la galerie et vingt unités au périhélie
de Halley. Une garde l'exige **aux deux échelles**, chacune prise à son propre
périhélie.
### 1P/Halley dans le preset solaire (2026-10-02)

Demandée par l'utilisateur : la skymap ne montrait de comètes qu'**au tirage**,
donc il fallait régénérer jusqu'à en croiser une. Le preset solaire en porte
maintenant une, nommée et cliquable.

**Éléments réels** : demi-grand axe 17,834 UA, excentricité 0,967, période ~75
ans. Son périhélie tombe à **0,586 UA**, entre Mercure et Vénus — elle plonge au
cœur du système, et c'est là que sa queue est la plus franche. Comme tout corps
posé par `poser_sur_rails`, elle **démarre à son périastre** : le preset s'ouvre
donc sur une comète en pleine activité.

⚠️ **Inclinaison 162° : elle est RÉTROGRADE**, et ce n'est pas une coquille.
Halley tourne à l'envers des planètes. Le modèle le rend **gratuitement** —
au-delà de 90°, le vecteur perpendiculaire bascule et le sens de parcours
s'inverse. Sa trace se parcourt donc dans l'autre sens que celle de la Terre.

Ses éléments sont **nommés** (`presets::HALLEY`) plutôt qu'écrits en ligne :
`construire_preset_solaire` tire des aléas, donc exige le contexte graphique et
ne se teste pas. La garde atteint ainsi les trois faits qui comptent — périhélie
entre Mercure et Vénus, inclinaison rétrograde, et queue qui s'allume entre 1 et
2,5 UA mais reste éteinte à l'orbite de Jupiter.

### Le lot 9.2 : la queue (2026-10-02)

**Zéro pipeline de plus.** `impostor::push_ruban` existait déjà — il sert aux
arches solaires — et sa doc disait que les UV d'un ruban traversent une texture
de halo par son milieu, ce qui lui donne un profil de **tube**. Une queue de
comète est exactement ça. Le générateur de cette texture a donc déménagé du
soleil vers `impostor`, à côté de son usage, avec un exemplaire partagé : deux
générateurs du même dégradé auraient fini par ne plus se ressembler.

⚠️ **Tout ce qui décide est pur, et c'est délibéré.** `comete::queue` rend une
liste de points ; elle ne dessine rien. `Planete::new` charge un material, donc
exige le contexte graphique : une queue construite dans la méthode de dessin
n'aurait eu **aucune garde**. Il ne reste dans `Planete::draw` que le ruban et
le tirage.

**Trois faits que les gardes tiennent** :

- **Elle fuit l'étoile, pas la trajectoire.** C'est *l'erreur classique*, et elle
  se voit tout de suite : une queue qui traîne derrière la comète comme un
  sillage de bateau est fausse. Le vent stellaire chasse la poussière
  **radialement**. Le test prend une vitesse perpendiculaire au rayon — le cas qui
  sépare les deux hypothèses.
- **Elle s'incurve**, et la courbure **croît** le long de la queue : les grains
  gardent leur orbite et prennent du retard. Une queue rigoureusement droite
  serait celle des **ions**, écartée par défaut au §8 bis.
- **Elle naît à l'approche et s'efface au loin**, décision n° 1 : au froid, la
  fonction rend une liste **vide** — pas une queue de longueur nulle, rien du
  tout, et c'est l'état le plus fréquent d'une comète.

⚠️ **Le seuil d'activité vient de l'orbite de la comète, pas d'une distance
absolue.** Dans le réel la glace sublime vers 3 UA quelle que soit la comète ;
mais les systèmes du jeu sont **mis à l'échelle** de leur étoile, si bien qu'un
seuil en UA allumerait en permanence toutes les comètes d'une naine et jamais
celles d'une supergéante. Il vaut donc **trois périhélies**, plafonné à l'aphélie
pour qu'une orbite ronde ne fume pas partout.

**Conséquence à connaître** : l'activité **plafonne à 0,44** au périhélie, jamais
1 — elle s'allume à trois périhélies et croît en `(1 − d/seuil)²`. Les
constantes visuelles sont calibrées là-dessus, et la garde assertait d'abord
`> 0,5`, ce qui était une valeur et non un fait.

**Trouvé en chemin** : `maj_rail` ne mettait à jour que la **position**. La
vitesse restait figée à sa valeur de `t = 0` depuis la construction — sans
conséquence pour un corps sur rails, mais la queue s'incurve vers sa traînée, et
elle se serait donc incurvée du **même côté sur toute l'orbite**. `position`
passant déjà par `etat`, la corriger ne coûte rien.

⚠️ **Un sabotage a révélé un trou dans la planche** : lui retirer les orbites de
ses comètes — donc leur seuil, donc leur queue — laissait **tous** les tests
verts. Elle aurait affiché des noyaux nus, et c'est précisément la queue qu'elle
sert à juger. Une garde dit maintenant que la queue est éteinte au bout froid de
la ligne et franche au bout chaud.
### Le lot 9.1 : les comètes existent (2026-10-02)

**Une catégorie morte prend vie.** `Categorie::Comete` existait depuis le début
avec un `allow(dead_code)`, reconnue par la désignation, la fiche et le
sélecteur — et **rien ne la produisait**. Même piège que le trou noir avant le
lot 4 : un rendu qui attend, et aucun chemin pour y arriver.

⚠️ **Sur rails, et ce n'est pas un détail d'implémentation.** La décision n° 1 du
§8 bis fait de la comète un objet de jeu, donc une **destination de vol** — ce
qui exige une position **prédictible** (`calculer_trajectoire` rend `None`
sinon, et c'est pourquoi une lune n'est pas une destination valide). Elle passe
donc par `poser_sur_rails`, la même fonction que les planètes, avec un
`Foyer` et une `Orbite`. La rattacher par `parent` l'aurait exclue du vol
sans que rien ne le dise.

**Un défaut mesuré, trouvé avant d'écrire la génération** : le solveur de Kepler
faisait **six itérations fixes**. Exact jusqu'à `e = 0,90`, puis l'écart
angulaire atteignait **0,021 rad** à 0,97 et **0,114 rad — 6,5°** à 0,99. Halley
est à 0,967 : les comètes vivent précisément là où il décrochait, et l'erreur est
maximale **au périastre**, là où l'astre file le plus vite. Il itère maintenant
jusqu'à convergence, avec un plafond — une planète quasi circulaire coûte
toujours deux passes.

**Et une planche-contact**, demandée par l'utilisateur faute d'objet de ce type.
⚠️ Elle ne montre pas des corps mais des **orbites** : un noyau fait quelques
pixels et se ressemble d'un exemplaire à l'autre. Une ligne par excentricité, une
colonne par phase — de l'aphélie au périhélie. C'est la vue qui servira au 9.2 :
la queue y naîtra et s'y effacera **le long de la ligne**.

#### ⚠️ Trois gardes ne gardaient rien, et le red-check l'a dit

1. **Une borne inatteignable.** J'avais posé un plancher sur le périhélie pour
   que la comète ne rase pas l'étoile. Mesuré : il ne **mord jamais** — à
   `e = 0,97`, `a(1−e)` vaut encore 3 % d'un demi-grand axe qui part de 14
   fois l'échelle du système. Du code défensif qu'aucun sabotage ne pouvait faire
   rougir. Retiré ; c'est la garde qui porte désormais le fait, en vérifiant que
   le périhélie dégage **de lui-même** — et elle rougit si l'on élargit
   l'excentricité.
2. **Deux tests qui se trouvaient eux-mêmes**, en citant en clair la chaîne qu'ils
   cherchaient dans le source. **Troisième occurrence** du piège sur ce chantier.
   Motifs assemblés à l'exécution.
3. **Deux tests qui recopiaient l'arithmétique** qu'ils éprouvaient — le calcul
   de l'orbite, puis la formule de phase de la planche. Extraits en fonctions
   pures (`orbite_comete`, `phase`), que la production **et** la garde
   appellent.

#### ⚠️ Et une erreur de manipulation, à ne pas refaire

Mes sauvegardes de red-check nommaient les fichiers par leur **nom de base** :
`genese/mod.rs` et `planete/mod.rs` ont donc partagé le même fichier, le
second écrasant le premier. La restauration a écrit le mauvais contenu dans
`genese/mod.rs`, et le travail du lot y a été **perdu** — rattrapé par `git
checkout` puis refait. **Sauvegarder au chemin complet**, jamais au nom de base.
### ⚠️ Le lot 8 est ABANDONNÉ (2026-10-01)

**Décision de l'utilisateur**, après deux livraisons et une refonte sur
recherche : *« la lentille est devenue bien trop grosse »*. **Le code a été
retiré** — shader, material, passe de dessin, cubemap, tout. Le dépôt est revenu
à son état d'avant le 8.1.

**Ce qui reste, et c'est le seul acquis** : la recherche,
[conception/astres_v2_lentille.md](../conception/astres_v2_lentille.md), qui
chiffre pourquoi l'effet est difficile. Quiconque reprendra ce lot doit la lire
avant d'écrire une ligne — elle épargne les deux impasses que j'ai parcourues.

**Pourquoi « trop grosse », et c'est la leçon** : la recherche a montré que la
déviation décroît en `1/b` et ne tombe sous un pixel qu'au-delà de **1400**
rayons d'ombre. J'en ai conclu qu'il fallait un quad de **12** rayons pour
contenir l'étirement visible — chiffre juste, mais il produit à l'écran une
distorsion qui **domine** le trou noir au lieu de l'habiller. Mes tests
garantissaient que l'effet était *contenu* ; aucun ne disait qu'il devait rester
**discret**. Le physiquement correct et le visuellement souhaitable divergent
ici, et c'est exactement l'arbitrage que le §5 de la recherche sur le disque
d'accrétion avait déjà rencontré — sauf que cette fois, aucun dosage ne
réconcilie les deux.

**Si le lot est repris**, les trois pistes dans l'ordre où je les tenterais :
atténuer la déviation par un facteur d'image assumé (comme `beaming_max` pour
le disque), ce qui revient à mentir sciemment sur `α` ; ou borner l'effet à un
anneau fin autour de l'ombre sans toucher au reste du ciel ; ou l'abandonner
définitivement, ce que la rareté des trous noirs rend très défendable.

**Deux limites restent vraies** et vaudront pour toute reprise : la cubemap est
onze fois plus grossière qu'un pixel d'écran, et 700 à 900 étoiles ne font pas
une couronne.

### Le lot 8, tel qu'il avait été conçu (archive)

**Tranché avec l'utilisateur** : lentille **locale**, fond échantillonné depuis
une **cubemap gravée du champ d'étoiles existant**.

**Pourquoi locale, et ce n'est pas qu'une question de coût.** Le non-local
demanderait la scène déjà rendue comme texture — une cible de rendu plein écran,
une copie, une seconde passe par frame, ce que l'architecture n'a pas (la
recherche sur le disque d'accrétion le notait déjà). Mais surtout **une copie
d'écran ne contient pas ce que le trou noir cache** : ces pixels n'ont jamais été
dessinés. Or l'effet signature d'une lentille est de faire réapparaître en anneau
ce qui est *derrière*. Tordre l'image rendue donnerait une distorsion du décor
visible, **jamais** l'anneau d'Einstein. La cubemap, elle, contient tout le ciel,
y compris les directions occultées à l'écran.

⚠️ **La limite est de fond, pas de budget.** Une cubemap encode des directions
**à l'infini**. Le champ d'étoiles est à 3000 unités, donc assimilable à
l'infini ; une planète à 40 unités a de la **parallaxe** — sa direction apparente
dépend d'où est l'œil — et aucune cubemap échantillonnée par direction seule ne
peut la représenter. Une planète qui passe derrière le trou noir ne sera donc ni
déviée ni reprise dans l'anneau. Il faudrait du raymarching géodésique par pixel.

#### 8.1 — le ciel en cubemap

⚠️ **Chaque vue tire son propre champ d'étoiles** (400 à 900 selon l'écran, sept
sites de `Fond::new`, tous aléatoires). Une cubemap construite à part montrerait
donc **d'autres étoiles** que celles qu'on voit, et l'anneau afficherait un ciel
qui n'existe nulle part. Elle est gravée depuis `self.etoiles`, et `Fond` la
porte — c'est la seule façon que les deux ne puissent pas diverger.

**La partie qu'on ne peut pas déboguer à l'œil** est la convention de cubemap :
elle retourne deux axes selon la face, et s'en écarter ne casse rien de visible
tant que le ciel est seul — *un ciel d'étoiles aléatoires miroité ressemble à un
ciel d'étoiles aléatoires*. Ça n'apparaîtrait que dans l'anneau, où le ciel dévié
ne raccorderait plus le ciel autour : un défaut qu'on mettrait des heures à
attribuer et qu'on corrigerait en tâtonnant sur des signes. D'où `direction_de`,
l'inverse exact de `face_et_uv`, et trois gardes — aller-retour sur 1600
directions, chaque axe sur sa face au centre, chaque étoile gravée dans sa face.

**Le chemin technique est validé** : macroquad n'expose que des `Texture2D`, mais
`Texture2D::from_miniquad_texture` accepte une cubemap créée par en dessous, et
miniquad lie la texture avec **sa propre cible** au moment du dessin
(`gl.rs`, `apply_bindings_from_slice`). Un `samplerCube` recevra donc bien un
`GL_TEXTURE_CUBE_MAP`.

**Trouvé en chemin** : `impostor::verif::uniforms_glsl` n'écartait que
`sampler2D`. Le premier `samplerCube` se serait présenté comme un uniform manquant
côté Rust, et la garde de correspondance aurait accusé le shader au lieu d'elle-même.

Le découpage était délibéré : verrouiller la convention avant qu'une ligne de
GLSL en dépende. Le 8.2 l'a consommée sans avoir à y revenir.

#### 8.2 — la déviation, livrée

**Une passe à part, dessinée avant tout le reste de l'astre.** L'ordre n'est pas
un détail : la lentille rend un ciel **opaque**, qui doit *remplacer* le fond
stellaire déjà dessiné. Posée après, elle effacerait l'arc du pli qui enjambe
l'horizon ; posée en additif, on verrait le ciel réel **et** le ciel dévié
superposés.

**Un material à elle, et c'est ce qui a coûté le douzième pipeline.** Une texture
déclarée sur un material oblige **chaque** objet qui s'en sert à la fournir : la
coller au material du soleil aurait imposé une cubemap à toutes les étoiles du
jeu pour le seul bénéfice des trous noirs. Elle est sautée quand aucun ciel n'a
été posé — plusieurs écrans dessinent des étoiles sans passer par
`Rendu::rendre`, et dessiner sans la texture ferait paniquer macroquad.

**La formule.** Lentille mince : un objet dont la position non déviée est `β`
apparaît en `θ`, avec `β = θ − θ_E²/θ`. On rend l'inverse — pour chaque pixel à
`θ`, on va chercher le ciel à `β`. L'**anneau d'Einstein** en sort tout seul
(`β = 0` en `θ = θ_E`, donc tout ce pourtour montre le point exactement
derrière), et `β` négatif donne l'**image secondaire** de l'autre côté du centre,
gratuitement, puisque `sin` change de signe.

⚠️ **La déviation s'éteint au bord du quad.** Le quad est un objet de rendu, pas
un objet du monde : sans extinction, le ciel dévié s'arrêterait net sur une
frontière invisible et on verrait un disque de ciel décalé posé sur le ciel réel.
Avec elle, `β` rejoint `θ` au bord, donc le ciel échantillonné y **est** le ciel
réel. Effet de bord heureux : c'est ce qui rend la convention de cubemap
vérifiable à l'œil — si elle était fautive, le bord du quad trancherait.

**`atan` et non `r/dist`** pour l'angle apparent : au cadrage de la vue isolée
(cinq rayons) l'approximation des petits angles se trompe de plus de 1 %, et
l'anneau se placerait au mauvais endroit précisément là où on l'examine.

**`RAYON_EINSTEIN = 1,35` rayon d'ombre** — réglage d'image, pas une constante :
la formule décrit une masse ponctuelle et une source plane, ni l'une ni l'autre
exacte ici. Posé un peu au-delà du bord de l'ombre pour que l'anneau se détache
du noir sans se confondre avec le liseré ni avec l'anneau de lentille du disque,
qui occupent déjà ce voisinage.

#### ⚠️ Une garde a cédé, et pas par hasard

`le_nombre_de_shaders_reste_sous_le_plafond` portait un invariant annexe : « le
**double** des pipelines vivants tient sous le plafond de 32 ». À 17 pipelines,
34 dépasse — la règle interdisait donc **tout** shader supplémentaire, à jamais.

Ce n'était pas ce qu'elle voulait dire. Le risque réel est une panique au
lancement au-delà de 32 ; le doublement n'en était qu'un proxy, choisi quand le
dépôt tenait dans huit pipelines. Remplacé par ce qu'on veut vraiment garantir :
**huit emplacements libres**. Relâchée délibérément, donc, et consignée comme
telle — à 17 sur 32 il reste 47 % du budget, et si ce nombre remonte la réponse
n'est plus de relever le seuil mais de **mutualiser des passes**.

#### ⚠️ Correctif 8.2 : l'impostor carré (2026-10-01)

**Signalé à l'écran** : un **rectangle** aux bords nets autour du trou noir.
Mesuré sur la capture avant de supposer — rayon écran annoncé 99 px, rectangle
d'environ 680 px de large, soit exactement `2 × 99 × 3,4` : le quad de lentille,
montré **carré** au lieu d'être découpé en disque.

**La cause** : `disc` est un **rapport** (`etendue`), alors que la demi-taille du
quad est une **longueur monde** (`rayon × etendue`). J'ai passé la seconde là où
le premier était attendu. Le vertex fait `v_q = (texcoord·2−1)·disc` : avec
`disc = 0,62 × 3,4 = 2,1`, `|v_q|` plafonne à 2,97 dans les coins et
`if (rr > couronne) discard` — seuil 3,4 — **ne mordait jamais**. Tout le carré
passait.

**Pourquoi rien ne l'a vu** : deux `f32`, deux appels corrects isolément, aucun
uniform manquant, aucune garde franchie. Les deux nombres décrivent la **même
étendue** et se ressemblent assez pour qu'on les confonde. C'est le genre de
défaut que seul l'écran attrape.

**La parade** : `impostor::poser_quad_impostor` pousse le quad **et** pose `disc`,
à partir des deux mêmes nombres — ils ne peuvent plus se contredire. Et une garde
interdit le geste : `personne_ne_pose_disc_de_sa_propre_main` refuse qu'un
dessineur le règle à la main, et vérifie que les deux fonctions sanctionnées le
posent bien.

⚠️ Son motif est **assemblé à l'exécution** (`format!("set_uniform(\"{}\"",
"disc")`), sinon le test se trouve lui-même : il cite forcément ce qu'il cherche.
C'est la troisième fois que ce piège apparaît sur ce chantier.


#### ⚠️ Refonte 8.2 sur recherche (2026-10-01)

Jugé « ça rend mal » après le correctif du carré. Recherche demandée, consignée
dans [conception/astres_v2_lentille.md](../conception/astres_v2_lentille.md).
**Deux défauts, tous deux mesurés, aucun de dosage.**

**1. La formule n'était pas celle d'un trou noir.** J'avais pris la **lentille
mince** `β = θ − θ_E²/θ` — une masse ponctuelle avec une source à distance
finie. Elle ignore la sphère de photons, donc l'ombre, donc l'empilement d'images
à son bord : tout ce qui fait reconnaître un trou noir. La littérature donne deux
régimes, et les implémentations temps réel sérieuses emploient les deux : champ
faible `2 r_s / b`, et **déviation forte de Darwin**
`α = −π + ln[216(7−4√3)/(1−b_c/b)]`, qui **diverge** au bord de l'ombre.

Heureuse surprise : le paramètre d'impact critique vaut `b_c = 2,598 r_s`, soit
**exactement** le rayon d'ombre que le chantier emploie déjà. Dans nos unités
`b_c = 1`, et les deux constantes se dérivent sans rien régler à l'œil.

**2. Le quad était douze fois trop petit.** La déviation décroît en `1/b` :
elle vaut encore **13°** à 3,4 rayons — où s'arrêtait mon quad — et ne tombe sous
un pixel qu'au-delà de **1400**. Mon atténuation n'adoucissait donc pas un
détail : elle **écrasait l'essentiel de l'effet** sur toute la moitié extérieure.

⚠️ Mais la conclusion n'est pas « un quad de 1400 rayons ». Sur un champ
d'étoiles **sans structure**, un décalage *rigide* est indétectable ; ce qui se
voit est l'**étirement**, donc le gradient — et lui passe sous le pixel vers
`b ≈ 6`. Douze rayons suffisent, et le faux gradient de l'atténuation y reste
sub-pixel. **Les deux affirmations sont des tests**, pas des impressions.

**Ce qui restera limité**, et qu'il ne faut pas espérer corriger par la lentille :
la cubemap est **onze fois plus grossière** qu'un pixel d'écran (256 texels pour
90°), donc les étoiles s'étaleront près de l'anneau ; et 700 à 900 étoiles sur
toute la sphère n'en mettront que quelques-unes dans l'anneau. Densifier le ciel
est un choix d'aspect **global**, à prendre explicitement — pas en douce ici.
#### Ce que le lot 8 ne fait pas

- **Une planète qui passe derrière le trou noir n'est ni déviée ni reprise dans
  l'anneau.** Limite de fond, pas de budget : une cubemap encode des directions
  à l'infini, et une planète à 40 unités a de la parallaxe.
- **L'anneau montre un ciel de 700 à 900 étoiles**, celui de la vue. S'il paraît
  vide, c'est que le ciel l'est — densifier le champ densifierait les deux
  ensemble, ce qui est le comportement voulu.

La formule est posée, il reste à la câbler. Pour un pixel à l'angle `θ` du centre
(azimut `û`), la lentille mince donne la direction source `β = θ − θ_E²/θ` : un
anneau d'Einstein apparaît naturellement en `θ = θ_E`, et `β` négatif produit
l'image secondaire de l'autre côté. Dans le shader :

```glsl
vec2 u = v_q / max(rr, 1e-4);
float theta = rr * angle_ombre;
float beta  = theta - e * e / max(theta, 1e-6);
vec3 lat = u.x * cam_right + u.y * cam_up;
gl_FragColor = textureCube(fond, normalize(-to_cam * cos(beta) + lat * sin(beta)));
```

Il faut : déclarer `samplerCube fond` dans `MaterialParams.textures`, ajouter
l'uniform `angle_ombre` (le rayon angulaire de l'ombre, `R/d`, que le CPU calcule
déjà pour `PX_MIN`), élargir la couronne du trou noir pour laisser place à
l'anneau, et poser la cubemap sur le material depuis `rendu.rs` — le seul endroit
où le `Fond` et le système sont tous deux en main.

**Étape de validation suggérée avant la déviation** : sampler **sans** dévier
(`beta = theta`). Le quad élargi doit alors devenir **invisible** — le ciel
continue à travers lui sans raccord. C'est la preuve que la convention est bonne,
et elle se juge d'un coup d'œil.

### Le lot 7 : la pulsation (2026-09-30)

**Une seule pièce pour trois types**, comme annoncé : céphéide, Mira et ZZ Ceti
ne diffèrent que par deux nombres, `etoile::pulsation(nom) -> (amplitude,
période)`. La **luminosité n'a pas de réglage à elle** — à température constante
`L ∝ R²`, donc elle suit le carré du facteur. C'est la physique qui lie les deux,
et ça évite deux réglages pour un seul fait. Conséquence heureuse : la variation
d'éclat se voit **à n'importe quelle échelle**, là où le gonflement du disque ne
se lit que de près.

**Réponse à la question laissée ouverte au §9** (« jusqu'où va la pulsation ? ») :
les **amplitudes de rayon sont réelles** — δ Cephei varie d'environ 10 %, Mira
d'environ 20 %. Seule **ZZ Ceti est exagérée** : son rayon ne bouge en vrai que
de quelques millièmes, et sa variation de brillance de 0,1 à 3 % serait invisible
ici. Triche assumée, du même ordre que `PX_MIN`. Les **périodes**, elles, sont
toutes compressées et ne peuvent pas ne pas l'être (5,4 jours, 332 jours,
quelques minutes) ; le garde-fou est qu'aucune ne descende au clignotement.

⚠️ **La pulsation n'est pas un mode de couronne** — une céphéide est au mode 0,
comme une G ordinaire — donc elle n'aurait atteint aucune étoile par `Variante`.
Il aurait fallu l'écrire dans les **cinq** endroits qui configurent une étoile
d'après son type. `Variante::appliquer` prend donc maintenant le **nom** en plus
du mode, et `Feuille` porte le nom de son type pour que les systèmes multiples en
bénéficient. La conception l'avait annoncé : « `Variante` ignore trou noir comme
pulsation ; un exotique ajouté sans elle n'apparaîtra jamais dans un système
multiple. » Les deux gardes anti-recopie de `stellaire.rs` couvrent
`avec_pulsation(`.

**Deux types n'ont que leur pulsation pour exister**, et c'est assumé : Mira
recouvre la géante rouge en teinte comme en taille, ZZ Ceti a le gabarit d'une
naine blanche au chiffre près. Le §3 bis l'autorise — une représentation
distincte à l'écran, pas forcément une teinte distincte. Une garde compare chacun
à son jumeau non pulsant, et une autre vérifie que les jumeaux ne pulsent pas.

⚠️ **Le piège annoncé — la cible de clic — n'en était pas un.** La pulsation
entre dans `rayon_dessine`, source unique de la taille : elle bouge donc la cible
*avec* le disque peint, jamais en désaccord avec lui. Et `Systeme::pick` prend
40 % de marge, si bien qu'à ±20 % d'amplitude le rayon cliquable reste toujours
au-dessus du rayon peint. Seule la marge respire. La parade était déjà là.

#### ⚠️ Dette : la pulsation est trop sage — pour **Astres V3** (2026-09-30)

Jugée « correcte pour le moment ». **Le motif, dans les mots de
l'utilisateur** : *« ça fait un côté gonflement sinusoïde que j'aime pas,
j'aurais préféré un côté tumultueux plus asymétrique »*. Il a lui-même noté que
la complexité derrière n'est pas anodine et **repoussé la reprise à une V3** :
ne pas l'engager dans ce chantier.

Donc le défaut n'est **pas** le dosage, et pas non plus la couleur : c'est la
**forme de l'onde**. Une sinusoïde monte et descend au même rythme, et ça se lit
comme une respiration mécanique. Les vraies courbes de lumière sont
**asymétriques** — montée rapide, déclin lent — et c'est précisément cette
dissymétrie qui donne l'impression d'une étoile qui se soulève puis retombe.

Deux pistes, par coût croissant, à reprendre en V3 :

- **Une fonction de forme** à la place de `sin`, avec un paramètre de
  dissymétrie. Un terme de plus dans `facteur_pulsation`, rien de structurel —
  c'est le dosage qui demandera des allers-retours à l'écran.
- **Du tumulte** par-dessus : une seconde harmonique décalée, ou un bruit lent
  ajouté à la phase, pour que deux cycles ne soient pas superposables. C'est ce
  que le mot « tumultueux » demande, et c'est plus difficile à doser sans
  basculer dans le tremblement.

Deux autres manques, relevés par lecture du code et **non** signalés par
l'utilisateur — à garder sous le coude, pas à traiter d'office :

- **La température ne bouge pas** : une vraie céphéide traverse la bande
  d'instabilité et change de **couleur** en se dilatant. Ici `couleur` est figée
  à la génération.
- **Rayon et éclat sont en phase**, l'un étant le carré de l'autre. Une céphéide
  réelle est la plus brillante quand elle se **dilate**, pas quand elle est la
  plus grosse.

**L'écrêtage de l'amplitude vit dans une fonction libre**, `facteur_pulsation` :
`Soleil::new` charge des materials, donc exige le contexte graphique, et une
borne posée au constructeur ne se testerait pas. Or c'est *elle* qui empêche un
rayon nul — donc une étoile qui disparaît en emportant sa cible de clic.

### Le lot 6, découpé en deux (2026-09-30)

**6.1 — les trois types « donnée seulement », et ce qui n'en était pas.** La
fiche du §3 les donne tous les trois pour de la donnée pure, « le rendu normal
leur convient ». **C'est faux pour l'étoile carbonée**, et mesuré : dérivée du
corps noir comme les autres, elle sort à `(1,00 / 0,68 / 0,40)` à 2900 K, quand
une naine M à 2400 K est à `(1,00 / 0,61 / 0,24)`. Elle serait donc **moins
rouge** que ce dont elle doit se distinguer — une étiquette, pas un type, ce que
le filtre de la §3 bis interdit.

D'où `filtre_teinte`, un multiplicateur sur la couleur de corps noir. Et c'est
la physique, pas un correctif : une carbonée est une photosphère froide vue **à
travers** ses bandes de C₂ et CN, qui mangent le bleu puis le vert. Avec le
filtre : `(1,00 / 0,375 / 0,08)`, la chose la plus rouge du catalogue.

**La sous-naine sdB a le problème inverse** : sa couleur est celle d'une naine
blanche — `(0,62 / 0,74 / 1,00)` contre `(0,67 / 0,78 / 1,00)`. Aucun réglage de
teinte ne les séparera. Ce qui la distingue est son **éclat**, cinq à vingt fois
supérieur : cœur d'hélium en fusion contre braise qui refroidit. Ça se lit au
halo, et la zone habitable part au loin. Une garde dit explicitement que les
teintes ne les séparent pas — si un jour elles le faisaient, le commentaire
serait faux et le test le dira.

⚠️ **Le vrai contenu du 6.1 est ailleurs : la table du tirage n'était pas
testable.** Les types ordinaires vivaient dans la chaîne de `if` d'`aleatoire`,
qui appelle `gen_range` — donc hors d'atteinte de tout test, **exactement**
comme le trou noir l'était avant le lot 4. Les particulières avaient reçu leur
garde à ce moment-là ; les ordinaires, non. La table est sortie dans
`ordinaire(p, q)` sur le modèle de `particuliere(p)`, `q` donnant la place dans
la plage de rayon et de luminosité — sans lui, une table de constantes figées
ferait de toutes les géantes rouges des jumelles, soit le défaut que le lot 1 a
corrigé.

**Un trou structurel trouvé en chemin** : rien n'empêchait de poser une planète
**dans** son étoile. Le garde-fou anti-collision de `instancier` ne compare que
les planètes entre elles. Ça n'avait pas de conséquence — le plus gros astre
était une géante de 7 unités et l'échelle du système suit `√L` — mais c'était
une coïncidence. La supergéante monte à 12, et la quasi-étoile du lot 11 est
annoncée large comme une orbite. D'où `premiere_orbite`, posé une fois.

**6.2 — la naine brune comme centre d'un système.** Pas de la donnée : elle est
déjà **mieux rendue que la plupart des étoiles** (quatre presets gazeux soignés),
mais côté planète. C'est le seul type du tirage qui **n'est pas une
photosphère** — d'où `etoile::est_naine_brune`, que **tout bâtisseur de corps
doit consulter**. Trois le font : la genèse simple en fait le centre, le tirage
des feuilles d'un système multiple l'écarte (l'arbre stellaire ne sait bâtir que
des `Soleil`), et la vue isolée en fait une planète. Un quatrième qui
l'oublierait la peindrait en étoile, sans qu'aucun test ne bronche.

Quatre conséquences, chacune découverte en la posant :

- **Sa masse passe sous le plancher** de `masse_luminosite` — symétrique exact
  du trou noir, qui dépasse son plafond. Sans `MASSE_NAINE_BRUNE`, elle serait
  quadruplée et les périodes de tout son système fausses.
- **Le plancher en pixels lui manquait.** Il est réservé aux `Soleil`, pour une
  raison qui porte sur les planètes *parmi* les planètes ; un corps au **centre**
  joue le rôle de l'étoile, et sans lui on arrivait dans un système au cœur
  invisible. D'où `Planete::au_centre_du_systeme`.
- **`lumiere_principale` cherchait la catégorie `Etoile`**, pas un corps qui
  émet. Corrigé : la source est celui dont `lumiere()` rend `Some`. Équivalent
  pour tout l'existant, puisque seul un `Soleil` rayonnait.
- **Les fiches de ses planètes annonçaient 0 K.** `luminosite_totale` somme les
  `Astre::luminosite`, et une `Planete` n'en rendait aucune. Le rôle central
  porte donc la luminosité — un seul champ, `Option<f32>` : un corps qui éclaire
  est un corps qui a le plancher, par construction.

⚠️ **Le lot 6.1 a rendu service au 6.2.** La garde
`aucun_type_du_tirage_n_a_besoin_du_plancher` a signalé que la première échelle
choisie (0,15) faisait tomber la première planète **sous** le plancher orbital —
un système comprimé qu'on n'avait pas choisi. Relevée à 0,20. Et `echelle_systeme`
a été extrait à cette occasion : la règle vivait en clair dans `instancier`, et
la garde du 6.1 la **recopiait** pour pouvoir l'éprouver.

**Ce que le 6.2 n'apporte pas** : une naine brune ne peut pas être la
**compagne** d'un système multiple. L'arbre stellaire ne bâtit que des `Soleil`,
et `feuille_alea` l'écarte explicitement plutôt que de la peindre en étoile.

**6.2 bis — ses quatre classes dans la planche des étoiles.** Demande de
l'utilisateur, et la bonne conséquence du 6.2 : ce qui tient le centre d'un
système se juge parmi les étoiles, à côté d'une naine M qui est ce dont elle se
rapproche le plus. Elle reste aussi au catalogue des gazeuses, et c'est
cohérent — c'est bien par ce rendu qu'elle est peinte.

La planche portait `Vec<(String, Soleil)>` alors que sa boucle de dessin
n'appelait que `update` et `draw`, **tous deux au trait `Astre`** : le type était
plus étroit que l'usage. Passé à `Box<dyn Astre>`, rien d'autre n'a bougé.

⚠️ Deux pièges d'échelle, tous deux gardés :

- **le rayon de la planche n'est pas celui du jeu.** Une O bleue vaut 1,05 en
  vitrine contre 4,0 au tirage. Y poser le rayon du tirage (≈ 2) ferait de la
  naine brune le plus gros objet de la vitrine alors qu'elle est l'un des plus
  petits — de la taille de Jupiter, donc en deçà d'une naine M.
- **les noms des classes ne sont pas écrits dans la galerie.** Elle ne porte que
  des **températures**, et `preset_naine_brune` en déduit la classe : la même
  source que le jeu. Deux températures tombant dans la même classe donneraient
  deux cellules identiques et en cacheraient une — sans que rien ne le dise,
  puisque quatre cases s'afficheraient bien.

**Et une précision de vocabulaire, posée par l'utilisateur** : une naine brune
qui tient le centre d'un système joue un **rôle** stellaire, mais ce n'est pas la
**quasi-étoile** du lot 11 — celle-ci est une enveloppe de plusieurs milliers de
masses solaires chauffée par accrétion sur un trou noir, large comme une UA,
contre 0,07 masse solaire ici. Le lot 11 reste entier.

### Le lot 5 est clos. Tranché le 2026-09-30 : on garde les sept

**Décision de l'utilisateur.** La comparaison entre cibles ne débouche pas sur
un choix : *« pour garder de la variation je suis preneur de garder les 7 cas
de représentation de trou noir, ça rajoute de la diversité »*.

⚠️ **Conséquence, et elle n'est pas documentaire** : garder les sept veut dire
les **brancher sur le tirage**. Quatre d'entre eux n'existaient que dans les
vues de travail ; les laisser là aurait rendu la décision sans effet sur le
jeu. `regime()` puise donc désormais dans le catalogue entier.

**Le poids d'un appétit se partage entre ses cibles**, et c'est le point à ne
pas perdre : la décision ajoute de la *variété*, pas de la *fréquence*. Un trou
noir vorace reste aussi rare qu'avant — 20 % — il a simplement deux visages
possibles. Sans ce partage, décliner une teinte de plus rendrait mécaniquement
le vorace plus courant, et un état de crise cesserait d'en être un. Deux gardes
le tiennent : les sept sont atteignables, et les parts par appétit restent
0,35 / 0,45 / 0,20.

⚠️ **À graine égale, un trou noir existant change d'aspect.** C'est inhérent à
la décision. En revanche rien ne se décale en cascade : le preset est **dérivé**
de la graine, il ne consomme pas le flux aléatoire.

**Les deux axes, depuis le 5.5.** C'est la structure à comprendre avant de
toucher à quoi que ce soit ici :

- **l'appétit** (`nom`) — combien l'astre mange. Trois valeurs, et c'est le
  seul axe que le tirage connaît (`appetits()`, source unique dont `regime()`
  puise).
- **la cible** (`cible`) — de quoi ça a l'air. `refroidie()` applique une
  teinte à **n'importe quel** appétit, et le catalogue est le produit des deux.

Décliner une cible de plus coûte **une ligne**. C'est ce que le 5.5 est venu
vérifier, et ça a marché du premier coup.

Le tirage, lui, ne connaît que les **appétits** : il pondère un appétit, puis
partage ce poids entre les cibles qui le portent. Ajouter une teinte reste donc
une ligne, même du côté du jeu.

⚠️ **Absolu ou proportionnel : la distinction porte tout.** Ce que la cible
impose est absolu (teinte, gradient retiré, plafond du beaming, minceur) ; ce à
travers quoi l'appétit doit continuer de se voir est proportionnel (éclat,
turbulence, rotation, halo, cellules, spirale). Passer l'éclat en absolu
rendrait les trois appétits froids identiques — trois cellules pour un seul
objet. Gardé par `l_appetit_se_voit_encore_une_fois_refroidi`.

Les rapports proportionnels sont ceux **mesurés** entre `chaude()` et la cible
validée du 5.4, arrondis : la cible telle qu'elle a été jugée en ressort à
moins de 0,5 % (éclat 1,104 pour 1,1, turbulence 0,247 pour 0,25, rotation
0,605 pour 0,6). Le reste est exact. C'était la contrainte de l'exercice — un
découpage ne doit pas redéfiner ce qui a déjà été validé à l'écran.

**La braise**, quatrième teinte : ~3000 K, `chaud (1,0 / 0,58 / 0,28)`. Son
aura suit **gratuitement** — le shader prend la couleur du halo dans
`mix(froid, chaud, 0.6)` — et seule sa *présence* se règle (× 1,45), sans quoi
la variante ne se distinguerait que par un détail de bord.

**Les trois écarts de la cible froide, et rien d'autre.** Tous assumés comme
non physiques : ce sont exactement les retraits qu'a faits le film.

| | appétit « actif » | une fois refroidi | Pourquoi |
|---|---|---|---|
| teinte chaude | `(1.0, 0.97, 0.93)` | `(1.0, 0.80, 0.56)` | 4500 K, la valeur que Thorne a spécifiée pour Gargantua — **absolu** |
| `exposant_couleur` | 2,2 | **0,15** | température **constante en rayon** : un disque froid n'accrète pas, il ne s'échauffe pas vers le centre — **absolu** |
| `eclat` | 1,6 | **× 0,69** → 1,10 | sans gradient, `chaleur` vaut ~1 **partout** : l'éclat frappe le disque entier et 1,6 y ferait un aplat saturé. **Proportionnel**, pour que l'appétit se voie encore |
| `beaming_max` | 4,0 | **1,2** | « exceedingly lopsided, with the hole's shadow barely discernible » — **absolu**, c'est ce que la cible retire |
| `beta` | 0,35 | **0,35** | **inchangé** : la vitesse orbitale tient au rayon, pas à la température |
| bornes, pli, anneau | — | **identiques** | sinon on ne compare plus deux aspects mais deux objets |

Le dernier point est la condition de l'exercice, et il a sa garde
(`refroidir_ne_touche_pas_a_la_geometrie`, qui balaie les trois appétits et les
deux cibles froides). Le `beta` inchangé en est le corollaire honnête : c'est
le **plafond** qui porte le retrait du beaming, ce qui le désigne pour ce qu'il
est — un choix d'image, pas une autre physique.

**Les sept cibles sont au tirage** depuis la décision du 2026-09-30. Ce
paragraphe disait l'inverse jusque-là — les cibles froides étaient des
candidates en attente de verdict, volontairement tenues hors du jeu pour ne pas
changer les trous noirs existants avant qu'on ait choisi. Le choix a été de ne
pas choisir.

### Ce qui reste ouvert, et que personne ne garde

- **Le GLSL ne compile qu'au lancement.** Aucun test du dépôt ne peut le
  vérifier. C'est arrivé une fois au 5.3 — une variable employée dix lignes
  avant sa déclaration, panic au démarrage. `verif::usage_avant_declaration`
  garde désormais cette classe d'erreur **dans le corps de `main`**, mais
  elle seule : un type faux, un appel inexistant passeraient encore. Une
  vraie garde demanderait `glslangValidator` en dépendance de test — chantier
  à part, non engagé.
- **Convention de silhouette** : nous prenons l'ombre comme unité (le noir
  remplit l'anneau, convention « épaisse », celle de l'image EHT). La
  convention « mince » — noir à 0,55 × le rayon de l'anneau, anneau tracé
  **par-dessus** la lumière du disque — reste possible et n'a pas été
  essayée. Les mélanger est incohérent (recherche 2, critère 6).
- **Échelle du disque protosolaire de T Tauri** : 16 rayons stellaires, soit
  0,28 UA, contre 6 UA dans la galerie des disques et ~100 UA en réalité.
  L'utilisateur l'a trouvé grand ; il est en fait vingt fois plus petit que
  celui de la galerie. Laissé tel quel, `RAYON_PROTOSOLAIRE` est isolé.
- **Incohérence connue** : à longue distance l'étoile est agrandie par le
  plancher en pixels (`PX_MIN`) alors que le disque garde sa taille
  physique — le disque passe donc sous l'horizon dessiné. Sans conséquence
  visible à cette échelle, mais réel.
- **Fins de ligne** : `core.autocrlf=true` normalise en LF ce que le dépôt
  stockait en CRLF, d'où des diffs gonflés sur les fichiers touchés. C'est
  une incohérence préexistante ; la trancher demande un `.gitattributes`,
  décision de l'utilisateur.

---

## Ce que le chantier a appris

Les leçons qui valent au-delà de leur lot. Chacune a coûté un aller-retour.

### Une absence ne se corrige pas par un réglage

Trois fois, le défaut signalé n'était pas un défaut de dosage mais une
**chose qui n'existait pas** :

- l'effet « autocollant » des taches venait de ce que **rien ne disait où le
  champ était fort** (3.1) ;
- le trou noir était injoignable parce que **personne ne produisait son mode
  de couronne et personne ne l'attendait** — son rendu existait pourtant
  depuis le début (lot 4) ;
- le bruit du disque ne pouvait **pas** produire de filaments : son mapping
  était isotrope par construction, et aucun réglage n'en serait sorti (5.3).

### Deux endroits qui décident du même fait divergent toujours

Quatre occurrences, dont trois trouvées par l'utilisateur à l'écran :

- le **rayon dessiné** d'une tache (`taille × intensité` au rendu, `taille`
  seul à l'ancrage) → les pieds d'arche tombaient hors de leur tache ;
- `PORTEE` du champ, dupliquée en GLSL → deux tests de littéraux ajoutés ;
- la chaîne **mode → constructeur**, recopiée dans la galerie des étoiles →
  T Tauri sans disque, et un mode 6 qui y aurait donné un trou noir ;
- le vocabulaire d'accrétion versé dans le `DisqueConfig` des anneaux de
  Saturne → sorti dans un type à part au 5.3.
- la **liste des régimes**, recopiée **quatre fois** — galerie des disques,
  galerie des étoiles, vue isolée, et une garde. Elle allait resservir au
  5.4 : ajouter un preset à trois de ces quatre endroits ne se serait vu
  qu'à l'écran. `Accretion::catalogue()` est désormais la seule liste.

**Et sa cause profonde, trouvée au 5.5 : deux faits dans un seul champ.** Un
preset mêlait *combien l'astre mange* et *de quoi ça a l'air*. Tant qu'il n'y
avait qu'une cible, ça ne se voyait pas. À la seconde, décliner l'une par
l'autre revenait à réécrire les trois écarts d'appétit pour chaque teinte — et
le jour où l'utilisateur retouche l'endormi, une seule des deux familles
bougerait. **La duplication n'était pas dans le code, elle était dans le
type.** Séparer les deux axes a ramené la troisième cible à une ligne.

**Et sa variante sournoise : un fait déduit au lieu d'être porté.** Le nom
d'un régime était *reconstitué* — une ceinture ⇒ vorace, un halo élevé ⇒
endormi, sinon actif. La déduction tenait pour trois presets qui
s'ordonnaient. Le quatrième n'a aucun trait qui le trahisse : il se serait
nommé « actif », et la galerie aurait affiché deux cellules du même nom.
C'est un champ, maintenant.

### Un sabotage doit être vérifié, pas seulement lu

**Trois fois** un test est resté vert à tort :

- le sabotage n'avait **pas eu lieu** (l'indentation avait changé après une
  extraction) — le vert ne voulait rien dire ;
- les entrées du test **évitaient le cas limite** par chance : un zigzag
  dont la *différence centrée* annulait l'alternance, et trois glissements
  qui rataient tous les 5,6 % d'ancrages refusés ;
- le test ne vérifiait **que les indices**, pas les sommets : un brin d'un
  point laissait deux sommets orphelins par frame.

Quand j'écris les entrées à la main, je les écris trop gentilles.

Deux autres pièges du red-check, rencontrés depuis :

- **saboter le test au lieu du code.** Retirer un nom de la *liste
  d'assertions* ne prouve rien ; c'est la production qu'on casse.
- **un sabotage resté en place.** La commande qui devait le défaire a échoué,
  et l'arbre a été annoncé propre à tort. C'est un test qui l'a rattrapé au
  coup suivant. **Relire avant d'annoncer**, ne pas se fier à la commande.
- **un remplacement littéral frappe les deux côtés.** Au 6.2, deux sabotages
  sont restés verts : le motif existait **aussi** dans le test (`for k in
  0..400`) ou de l'autre côté de la comparaison (le nom d'un preset, écrit dans
  la fonction *et* dans le catalogue). Remplacé partout, le test se sabotait
  avec la production et ne prouvait plus rien. C'est la variante mécanique de
  « saboter le test au lieu du code ». **Compter les occurrences de l'ancre et
  refuser d'agir si elle n'est pas unique** — le script le fait désormais.
- **vérifier que le motif a mordu, systématiquement.** Au 2026-09-30 un
  sabotage a raté parce que le motif supposait des `CRLF` là où le fichier a
  des `LF`. Le script l'a dit — il teste que le motif existe avant de
  remplacer — et sans ce garde-fou j'aurais lu un vert comme une preuve.
  C'est la même leçon que l'indentation changée, et elle est revenue.
- **restaurer par copie remonte le temps.** `Copy-Item` donne au fichier
  restauré la date de **la sauvegarde**, donc plus ancienne que les
  artefacts du build saboté. Cargo compare les dates : il conclut que rien
  n'a changé et **relance le binaire saboté**. Au 5.4 ça s'est vu dans le
  bon sens — deux échecs sur un arbre pourtant sain, et le fichier disait
  bien la bonne valeur. Dans l'autre sens, c'est un sabotage qui passe au
  vert sans qu'on sache pourquoi. Le contenu a raison contre le test :
  vérifier le fichier, puis remettre sa date à l'heure avant de conclure.

### Deux nombres qui décrivent la même chose dans deux unités

Le défaut du 8.2 : `disc` est un **rapport**, la demi-taille du quad une
**longueur monde**, et les deux décrivent la **même étendue**. Rien ne distingue
un `f32` d'un autre `f32` ; les deux appels étaient corrects isolément ; aucune
garde n'a bronché. Seul l'écran l'a montré, sous la forme d'un impostor **carré**.

C'est une variante du « une source par fait » : ici le fait avait **deux
expressions légitimes**, et c'est leur conversion qui s'est perdue. La parade
n'est pas un test — rien côté Rust ne peut lire ce qui part au GPU — mais une
fonction qui **dérive les deux ensemble**, plus une garde qui interdit de les
poser séparément.

Le signe qui devrait alerter : deux paramètres voisins de même type, dont l'un
vaut l'autre multiplié par une grandeur du contexte.

### Un test qui affirme une conséquence garantie ne garde rien

**Deux fois**, et la seconde malgré la leçon de la première :

- « la première planète tombe hors de son étoile » — garanti par
  `premiere_orbite`, donc retirer le plancher laissait le test vert ;
- « une étoile qui pulse n'avale pas sa planète interne » — garanti par la marge
  de 1,5 rayon, donc porter l'amplitude de Mira à 0,49 le laissait vert.

Les deux fois, c'est le **sabotage** qui l'a dit, jamais la relecture : la phrase
sonnait juste et l'assertion était vraie. La parade est la même dans les deux
cas — **mesurer au lieu d'affirmer**. Le second calcule désormais la marge la
plus serrée du catalogue et refuse qu'elle descende sous 1,5 ; donner une
pulsation d'un quart à la naine brune la fait tomber à 1,46, et le test le dit au
chiffre près.

Le signe qui devrait alerter : un test dont la conclusion se déduit d'une borne
posée quelques lignes plus haut dans le code qu'il teste.

### Mesurer désigne le coupable ; supposer coûte deux tours

Sur la pointe au sommet des arches, deux correctifs successifs ont visé à
côté. Le premier chiffre mesuré — **41** de rapport entre les pas
d'échantillonnage — a immédiatement désigné la vraie cause, et a montré que
ma première correction *aggravait* la géométrie tout en améliorant
l'échantillonnage.

### Ce qui ne se teste pas doit être nommé

Le chantier a accumulé des réglages qu'aucune assertion n'arbitre :
`PART_COURTES`, `MAX_ARCHES`/`MAX_COURTES`, `MARGE_ARCHES`, `EVASEMENT`,
`ANCRAGE`, `RAPPORT_TUBE`, `pli`, `beaming_max`, `halo`. Tous portent un
commentaire qui dit **entre quels deux défauts** ils arbitrent — c'est la
seule forme de garde possible.
