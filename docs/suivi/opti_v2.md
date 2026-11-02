# Suivi — Chantier-Opti-v2

> Journal du chantier [`conception/opti_v2.md`](../conception/opti_v2.md) — créé
> à l'ouverture du lot O1, pas avant (convention du projet). Branche
> `chantier-opti-v2`, partie de `main` le 2026-10-28.

## Méthode de mesure

Harnais `PROFIL=<libellé>` (synchro coupée, 10 s), mode Palette, machine de
dev (Ryzen 7 3800X + RTX 3090). Vue isolée : `DEFILE=16` (T Tauri),
`DEFILE=17` (Herbig Ae/Be), `DEFILE=31` (G, référence) ; vue système :
`SYSTEME="T Tauri (jeune)"`. Compilé dans `CARGO_TARGET_DIR=target/mesure`
(l'exécutable de `target/release` peut être verrouillé par le jeu ouvert).

## O1 : les nuages calculés une fois par image (2026-10-28)

**Dette D1, et D8 en partie.** `dessiner_nuages_3d` était appelé pour chaque
moitié du disque et refaisait **tout** — boules, empilement, amorti, bras —
pour n'en garder que la moitié. Désormais `calculer_nuages` remplit une liste
par image (position, rayon, couleur prémultipliée, côté du plan), que chaque
moitié relit ; la clé (caméra, position du disque, temps des nuages, viewport)
dit si la liste vaut encore. Les boules et les quads gardent leurs tampons
d'une image à l'autre.

| Mesure | Avant | Après O1 |
|---|---|---|
| T Tauri, vue isolée | 38,9 (1 % bas 30,9) | **76,5** (63,6) |
| Herbig Ae/Be, vue isolée | 36,8 (32,7) | **69,5** (59,5) |
| G (référence) | 2 015 | 2 014 |
| T Tauri, vue système | 391 (279) | 398 (295) |

Le ×2 annoncé par la carte, tenu. La vue système bouge peu : son grand disque
coûtait ~1,2 ms, et c'est surtout l'empilement (O2) qui y pèse.

**Rendu inchangé** : même ordre de dessin (boules puis bras, par côté du
plan), même amorti ; vérifié par capture de la T Tauri avant/après (seule la
phase de l'animation diffère, l'image 120 ne tombant pas au même instant à 48
et à 60 fps). Pas de test nouveau : la logique ajoutée est une comparaison de
clé ; ce qui compte — le rendu — ne se teste pas sans contexte graphique.

## O2 : l'empilement borné à sa grille (2026-10-28)

**Dette D2.** Mesuré (sonde provisoire) : en vue isolée, la grille de
l'empilement **dégénère toujours en une case** (`COTE 1`, 28 792 boules) — la
taille des cases est la moyenne des rayons apparents, et les boules collées à
la caméra la font exploser. Chaque boule parcourait alors jusqu'à 25 × 25 cases
**hors grille**, ignorées une à une. En vue système, la grille fait 512² (le
grand disque, 3 000 boules) : pas de dégénérescence.

**Réparation sans changement d'image** : les boucles de `Empilement::verser`
sont bornées à la grille — mêmes cases visitées, dans le même ordre, donc
**exactement** la même pile. La « médiane au lieu de la moyenne » proposée par
la carte n'est **pas** faite : elle changerait le pic, donc l'amorti, donc
l'éclat des nuages validé à l'écran — une décision d'image, pas d'optimisation.

| Mesure | Après O1 | Après O2 |
|---|---|---|
| T Tauri, vue isolée | 76,5 (1 % bas 63,6) | **170,2** (141,3) |
| Herbig Ae/Be, vue isolée | 69,5 (59,5) | **168,4** (133,4) |
| G (référence) | 2 014 | 2 019 |
| T Tauri, vue système | 398 (295) | 399 (306) |

Garde : `borner_l_empilement_ne_change_rien` — l'ancien parcours, gardé en
référence dans le test, contre le nouveau, sur quatre tailles de case (une
case à 500 de côté) et 3 000 boules dont des hors champ : piles égales au bit
près. Red-check : borne supérieure exclusive (un cran de moins) → rouge.

**Reste de D2/D9** : la grille de 512² de la vue système (allocation et
centile sur 262 144 cases à chaque image, ~0,6 ms) ; la borner (64²) changerait
le pic — même réserve que la médiane. À reprendre avec O5 (qualité) ou si la
cible le demande.

**Bilan O1 + O2** : T Tauri ×4,4 (38,9 → 170), Herbig ×4,6 (36,8 → 168) sur la
machine de dev. Rapporté à la cible (CPU ×2–2,5 plus lent), le calcul des
nuages tient désormais dans le budget ; **le GPU reste le verrou** — O3 (voile
en une passe) et O4 (remplissage borné), qui ne se jugent que sur la cible ou
au plus près.

**Suite : O3**, le voile en une passe.

## Retour du petit PC, `DIAG` et la qualité (O5 avancé) — 2026-10-28

**Mesuré par l'utilisateur sur la cible** (premier APU Ryzen portable),
branche après O1 + O2 : **T Tauri 10 à 14 fps, Herbig pareil** — contre 170 sur
la machine de dev. Et la skymap V2 se mettait mal à l'échelle sur son petit
écran (corrigé à part : la ligne d'état passait sous les bascules et sortait à
droite, la ligne de raison de la fiche débordait).

**Sur la machine de dev, tout est CPU** (sonde provisoire, par image) : les
nuages coûtent ~4,9 ms — construire 1,9, bras et jets 1,2, pousser les quads
0,8, lots 0,4, empilement 0,6. Même ×3 ou ×4 sur la cible, ça ne fait pas
80 ms : **le verrou y est le GPU**, et il ne se mesure que là-bas.

**`DIAG=1`** (avec `DEFILE` ou `SYSTEME`) : un lancement, six phases de 5 s —
tout, sans voile, sans nuages ni bras, ni l'un ni l'autre, rendu Net, qualité
basse —, puis un tableau (fps, image, **CPU** jusqu'à `next_frame`). Une image
bien plus longue que son CPU désigne le GPU. Interrupteur des nuages ajouté
(`disque::set_nuages`), à côté de celui du voile.

**Le réglage QUALITE** (paramètres, sauvegardé ; un `reglages.json` d'avant se
relit en Haute) : **Haute** = le rendu validé, à l'identique ; **Basse** = le
voile en une couche au lieu de sept (le voile d'avant le lot 12) et un amas de
nuages sur trois. Le système ne change pas.

| Machine de dev, T Tauri vue isolée | fps | image | CPU |
|---|---|---|---|
| tout | 161,5 | 6,2 ms | 6,1 ms |
| sans voile | 162,9 | 6,1 | 6,1 |
| sans nuages ni bras | 572 | 1,7 | 0,5 |
| ni voile ni nuages | 1 977 | 0,5 | 0,1 |
| tout, rendu Net | 160,5 | 6,2 | 6,2 |
| **tout, qualité basse** | **307** | 3,3 | 3,2 |

Gardes : `un_fichier_d_avant_la_qualite_garde_ses_reglages` (red-check : sans
`serde(default)`, rouge), l'aller-retour JSON des réglages étendu à la qualité
(en Basse, pour qu'un champ ignoré ne passe pas), la ligne QUALITE portée par
le menu.

**Suite** : le tableau `DIAG` du petit PC décidera — voile (O3), remplissage
des nuages (O4), étoile (O6) ou CPU (O7).

## Le tableau `DIAG` du petit PC, et la palette quantifiée à sa taille (2026-10-28)

Mesuré par l'utilisateur sur la cible (écran 1920 × 1080 à 125 % : le jeu voit
**1536 × 864**), qualité haute, Palette :

| T Tauri, vue isolée | fps | image | CPU |
|---|---|---|---|
| tout | 14,5 | 69,1 ms | 68,0 ms |
| sans nuages ni bras | 51,2 | 19,5 | 18,4 |
| ni voile ni nuages | 279,5 | 3,6 | 1,6 |
| tout, rendu Net | 12,1 | 82,4 | 81,4 |
| tout, qualité basse | 30,7 | 32,5 | 31,6 |

| T Tauri, vue système | fps | image | CPU |
|---|---|---|---|
| tout | 30,4 | 32,9 ms | 30,0 ms |
| sans voile | 87,8 | 11,4 | 9,2 |
| sans nuages ni bras | 30,1 | 33,2 | 30,3 |
| ni voile ni nuages | 89,7 | 11,1 | 8,8 |
| tout, rendu Net | 47,8 | 20,9 | 18,7 |
| tout, qualité basse | 50,4 | 19,8 | 17,0 |

(La ligne « sans voile » de la vue isolée manque au relevé.) Ce qu'il dit :

- **Vue isolée : les nuages et les bras coûtent ~50 ms**, le voile ~16 ms,
  le reste 3,6. Le rendu Net (4 × plus de pixels) n'ajoute que 13 ms : le
  coût des nuages est **par boule** (calcul, envoi, appels de dessin), pas
  par pixel. Le « CPU » colle à l'image partout : les appels de dessin
  bloquent dans le pilote — le GPU et le CPU s'attendent l'un l'autre.
- **Vue système : le voile coûte ~21 ms** (le grand disque, sept couches) ;
  les nuages, rien.
- **Le filtre Palette coûtait plus que le rendu Net** en vue système (32,9
  contre 20,9 ms). Cause trouvée : la quantification se faisait **pendant
  l'agrandissement à l'écran**, donc à la résolution de l'écran — chaque gros
  pixel quantifié quatre fois, jusqu'à 182 distances Lab chacune.

**Corrigé** (`pixel::quantifier`) : la cible se quantifie **à sa propre
taille** dans une seconde cible (sans mélange), puis s'agrandit au matériau
par défaut. Même image — un gros pixel ne lit qu'un texel, et le shader n'a
pas de tramage lié à l'écran (vérifié par captures de la galerie, avant et
après : mêmes couleurs, étiquettes à leur place) —, **quatre fois moins de
quantification**. Invisible sur la machine de dev (sa 3090 ne la sentait pas) ;
à remesurer sur la cible.

**Suite, par ordre de poids sur la cible** : les nuages par boule (O7 : les
figer sur le GPU, plus d'envoi par image), le voile (O3), puis la DPI —
1536 × 864 vient de la mise à l'échelle de Windows, qui agrandit l'image du
jeu (flou) ; rendre à 1920 × 1080 coûterait 56 % de pixels en plus, à
trancher avec l'utilisateur.

## Le tour de diagnostic, `DIAG=tour` (2026-10-28)

*« Ce serait pas sage aussi de faire un diag aléatoire sur d'autres cas ? »*
Un lancement passe en revue **76 cas** — chaque astre du défilé en vue isolée,
chaque type d'étoile en vue système (graine 1), huit systèmes engendrés à graine
fixe (1 à 8, les mêmes partout), le système solaire, la galerie des planètes,
la starmap —, 3,5 s chacun (1 s de chauffe écartée), dans la qualité et le
rendu du joueur ; une ligne par cas, puis le tout du pire au meilleur.
~4 min 30.

Sur la machine de dev (Palette, Haute), le haut du classement :

| Cas | fps | image | CPU |
|---|---|---|---|
| isolée Herbig Ae/Be | 133 | 7,5 ms | 7,4 ms |
| isolée T Tauri | 134 | 7,4 | 7,3 |
| **système solaire** | **170** | **5,9** | **3,9** |
| galerie des planètes | 202 | 4,9 | 1,7 |
| système T Tauri | 295 | 3,4 | 2,6 |
| système Herbig | 347 | 2,9 | 1,2 |
| isolée hypergéante | 422 | 2,4 | 2,3 |

Tout le reste au-delà de 450 fps. **Surprise : le système solaire**, la vue
d'ouverture de la skymap V2, est troisième — et c'est du GPU (5,9 ms d'image
pour 3,9 de CPU), donc ce qui grossira le plus sur la cible. À regarder dès le
tour du petit PC.

### Le tour du petit PC (1536 × 864, Palette, Haute)

Le haut du classement :

| Cas | fps | image | CPU |
|---|---|---|---|
| isolée T Tauri | 14,1 | 70,9 ms | 70,1 ms |
| isolée Herbig Ae/Be | 14,2 | 70,4 | 69,5 |
| **système solaire** | **15,7** | **63,8** | **60,6** |
| isolée naine noire (1er cas) | 30,8 | 32,4 | 29,6 |
| système T Tauri | 31,0 | 32,2 | 29,6 |
| système Herbig | 34,5 | 29,0 | 26,4 |
| système K (orange) | 40,3 | 24,8 | 22,7 |

Le gros des systèmes tient entre 77 et 112 fps (≈ 10-13 ms), la galerie 88,
la starmap 117, les astres isolés sans disque 200-340.

- **La naine noire en tête est un artefact** : c'est le premier cas du tour,
  qui paie encore le chargement ; la même, plus loin, fait 343 fps.
- **Le système K** fait deux fois moins que ses voisins là-bas, mais K et G
  sont identiques au fps près sur la machine de dev (`SYSTEME`, `DIAG=1`) :
  probablement un à-coup de la cible, à revoir au prochain tour.
- **T Tauri et Herbig isolées** restent les nuages par boule (O7).

### Le système solaire : l'ISS et le Starship

`SYSTEME=solaire` ouvre désormais la vue sur le système solaire (avec
`PROFIL` ou `DIAG`). En retirant à l'essai ce qui lui est propre (machine de
dev, `PROFIL`) :

| Retiré | fps |
|---|---|
| rien | 169 |
| les quatre ceintures | 224 |
| Halley | 206 |
| **l'ISS et le Starship** | **485** |
| les trois | 637 |

Leurs maillages — des milliers de triangles — passaient par `draw_mesh`, qui
**recopie et transforme chaque sommet à chaque image**, alors qu'à l'échelle
du système ils tiennent dans moins d'un pixel. `engin::dessiner_engin`
(partagé avec `VaisseauEnTransit`) les dessine désormais en **cube d'un pixel**
de leur teinte sous 2 px de rayon apparent, et en maillage au-delà. Test
`un_engin_lointain_se_dessine_en_point` (rouge vérifié : seuil à 0).

Système solaire, machine de dev : **169 → 484 fps** (image 5,9 → 2,1 ms).
Rapproché d'un engin, le maillage repasse par `draw_mesh` : dette notée, des
tampons GPU persistants si une vue rapprochée rame.

### Les géantes gazeuses de près

Sur la cible, *« le 16 fps vient du fait que j'ai choisi de visualiser une
géante gazeuse »* : le relevé `DIAG` a changé de vue en cours de route, ses
phases ne se comparent donc pas. Deux outils pour le refaire proprement :

- `FOCUS=<astre>` (avec `SYSTEME=solaire`) cadre l'astre à `FOCUS_RAYONS`
  rayons (4 par défaut) ;
- une phase **rendu Pixel** dans `DIAG` : la cible basse résolution sans la
  quantification, pour séparer les deux coûts du rendu Palette.

Machine de dev, la planète remplissant l'écran (`FOCUS_RAYONS=1.6`) :

| | Palette | Net | Pixel |
|---|---|---|---|
| Jupiter | 3,0 ms | 3,9 | 2,7 |
| Saturne | 3,1 | 4,1 | 2,8 |
| Terre | 5,6 (CPU 3,0) | 3,6 | 5,3 |

La géante coûte ~2 ms de GPU ici : le shader enchaîne une vingtaine de `fbm`
à 5 octaves par pixel (courbure, deux niveaux de distorsion, détails). La
quantification ne pèse presque rien ici (Pixel ≈ Palette). La Terre de près
repaie le maillage de l'ISS et du Starship, désormais assez gros pour être
dessinés (dette notée plus haut). À trancher avec le relevé de la cible.

### Relevé propre de la cible, et la palette en table

`SYSTEME=solaire FOCUS=Jupiter DIAG=1` sur le petit PC :

| Phase | image |
|---|---|
| tout | 29,3 ms |
| sans voile | 18,3 |
| sans nuages ni bras | 22,8 |
| ni voile ni nuages | 17,6 |
| rendu Net | 23,1 |
| rendu Pixel | 21,5 |
| qualité basse | 22,0 |

Saturne remplissant l'écran (`FOCUS_RAYONS=1.6`) : 35,3 ms, dont ~12 de voile
(anneaux, ceintures) et ~8 de quantification (Palette 35,3 contre Pixel 27,4).

Trois postes sur la cible, dans le système solaire : **voiles ~11 ms**,
**quantification ~8 ms**, nuages ~5 ms. La quantification ne coûtait presque
rien sur la machine de dev : c'était la boucle de 182 distances Lab par pixel,
lourde pour le premier APU Ryzen.

**La palette en table** : `palette::table` précalcule, une fois par palette,
la couleur retenue pour chacune des 64³ couleurs d'une grille (atlas RGBA
512 × 512) ; le shader n'y fait plus qu'une lecture. Écart à la recherche
exacte : ~6 % des couleurs changent de gagnant, pour un voisin au plus
~3 ΔE plus loin (`la_table_ne_trahit_pas_la_recherche`, rouge vérifié en
écrivant les canaux dans le désordre). Captures avant/après de Jupiter et de
la T Tauri : identiques à l'œil.

⚠️ Piège rencontré : `Material::set_texture` ne retient que l'**identifiant**
GL de la texture. Lâchée aussitôt, elle était libérée et son identifiant
repris par la cible de la scène — le shader lisait la scène en guise de table
(Jupiter jaune et noir). La table est gardée en vie dans `pixel::TABLE`.

**Premier relevé cible avec la table** (Jupiter) : Palette 33,1 ms contre
Pixel 24,4 — le même écart de ~8 ms qu'avant. Toute la machine a dérivé de
~3 ms entre les deux relevés (Pixel 21,5 → 24,4). Si la table est bien dans
le binaire, ces ~8 ms ne viennent pas du calcul de la palette : soit de la
passe elle-même (une cible de plus), soit du voile en mode Palette. Le DIAG
gagne deux phases pour trancher : « sans voile, rendu Pixel » (à comparer à
« sans voile » en Palette) et « tout (bis) », la première phase répétée à la
fin pour mesurer la dérive.

**Le verdict** (Jupiter, binaire 6688b7b confirmé) :

| phase | image (ms) |
|---|---|
| tout | 31,2 |
| sans voile | 17,2 |
| ni voile ni nuages | 17,0 |
| rendu Net | 22,1 |
| rendu Pixel | 22,2 |
| sans voile, rendu Pixel | 16,7 |
| qualité basse | 23,5 |
| tout (bis) | 24,7 |

- La passe Palette ne coûte que **~0,5 ms** (sans voile : 17,2 contre 16,7).
- « tout » et « tout (bis) » sont la même configuration : 31,2 contre 24,7.
  La **première phase** du DIAG était **mal chauffée** (~6 ms de trop) — et
  c'était elle, « tout », qu'on comparait à Pixel. Les « ~8 ms de
  quantification » étaient pour l'essentiel ce biais ; toutes les lectures
  « tout − x » des relevés précédents en sont gonflées d'autant.
- Correctif : la première phase chauffe 4 s au lieu de 1.
- Jupiter en gros plan coûte en vrai ~22–25 ms (40–45 fps), dont ~6 ms de
  voile. Net ≈ Pixel : le fond de ~17 ms ne vient pas du remplissage des
  pixels (le Pixel en dessine quatre fois moins).

## O3 : le voile en une passe (2026-10-29)

L'ordre est recadré : *les ajustements sur le petit PC passent en dernier* —
les lots du plan dans l'ordre, chacun mesuré sur la machine de dev.

**Avant** : sept couches (T Tauri, Herbig ; trois pour la Be), sept passes de
l'annulus entier par moitié, chacune son shader de bruit et son mélange.

**Après** : une passe. L'annulus se pose sur la couche extrême du côté de la
caméra ; chaque pixel suit son rayon de vue jusqu'à chaque couche (un cône :
hauteur = décalage × distance à l'axe, intersection exacte du second degré),
y évalue le voile, et compose les couches **dans l'ordre où elles se
dessinaient** — le résultat des passes successives, en flottant. Les couches
se fondent en une seule, d'alpha égal à leur empilement, quand elles tiennent
sous un pixel d'épaisseur à l'écran ou qu'il ne reste que le plan (par la
tranche) : `couches_a_dessiner`, testée (`les_couches_se_fondent_quand_elles_se_confondent`,
rouge vérifié par trois sabotages).

⚠️ Piège rencontré : l'annulus d'abord posé **sur le plan**. Caméra dans le
rayon du disque (le cadrage par défaut de la T Tauri), les couches hautes du
bord opposé passent au-dessus d'elle : les rayons qui les atteignent montent,
ne croisent jamais le plan — le bord lointain pâlissait. Sur la couche du
côté de la caméra, tout rayon qui atteint une couche la traverse d'abord.

| T Tauri, machine de dev | avant | après |
|---|---|---|
| sans nuages ni bras (Palette) | 1,4 ms | 0,6 ms |
| voile seul, rendu Net (pleine résolution) | 0,9 ms | 0,4 ms |
| tout | 7,3 ms | 6,7 ms |

Le fond sans voile est à 0,4 ms : le voile passe de ~1,0 à ~0,2 ms. Herbig :
même chose (1,5 → 0,6 ; 1,0 → 0,4).

**L'image** : captures avant/après sans nuages, en rendu net (`NUAGES=0`,
`RENDU=net`, nouveaux), six cadrages de la T Tauri (défaut, 12°, 80°, 4°,
loin, tout près) et trois de la Be, comparées pixel à pixel : l'écart reste
sous le bruit d'une image à la suivante partout. De loin, un écart moyen de
0,85/255 sans aucun pixel au-delà de 8 : les stries radiales, arrondies
autrement par la composition en flottant.

**Ce qu'O3 ne fait pas** : les voiles à **une** couche — anneaux de Saturne,
ceintures du système solaire — prennent le chemin de toujours, inchangé.
C'était l'essentiel du voile dans le système solaire sur la cible.

Outillage ajouté : la phase DIAG « voile seul, rendu Net ».

## O4 : le remplissage des nuages, mesuré (2026-10-29)

**Le constat.** Sur la machine de dev, les nuages limitent par le CPU
(image ≈ CPU, ~6 ms) : le GPU absorbe leur remplissage sans broncher, le
chronomètre ne le montre pas. D'où un instrument indépendant de la machine :
`disque::remplissage_nuages`, la surface d'écran couverte par les quads des
nuages et des bras, coupée au cadre, **en écrans**. Colonne « nuages » du
DIAG ; testé (`le_remplissage_se_coupe_au_cadre`, rouge vérifié par deux
sabotages).

| T Tauri, `DEFILE=16` | remplissage |
|---|---|
| cadrage par défaut (dist 9,5) | ~100 écrans |
| dist 3 | ~106 |
| dist 1,5 | ~120 |
| qualité basse | 55 à 79 |

Chaque pixel est repeint une **centaine de fois**, dès le cadrage par
défaut. Une trace temporaire (retirée) l'a ventilé par taille écran : le
remplissage ne vient **pas** de quelques boules géantes collées à la caméra
— celles-là sont surtout hors champ ou derrière elle —, il est réparti sur
des milliers de boules de 128 à 512 px de large (écran de 720), et sur les
bras (un tiers à la moitié selon le tirage). Le plafond de taille écran
prévu au plan n'en retirerait qu'une part.

**Le levier choisi (par l'utilisateur) : le prototype basse résolution.**
Touche 4 de la vue isolée, trois T Tauri : nuages et bras à pleine
résolution, à 1/2 et à 1/4 par axe (`disque::basse_res`). Ils se dessinent
dans une cible réduite avec sa profondeur, puis s'ajoutent à l'image ; leur
matériau est purement additif, la somme est la même. Une sphère noire à la
place de l'étoile y cache ce qui passe derrière elle. Remplissage mesuré :
~100 → ~25 (1/2) → ~6 écrans (1/4).

⚠️ Le prototype compose avec un pipeline à lui, compté **hors de la marge**
de `le_nombre_de_shaders_reste_sous_le_plafond` (17 pour 16) : à régler à
l'adoption — le fondre dans un existant — ou au retrait.

**Mesure du temps, machine au repos** (T Tauri, `PROTO=0/1/2`, image en ms) :

| | pleine | 1/2 | 1/4 |
|---|---|---|---|
| cadrage par défaut, tout | 7,0 | 7,3 | 6,9 |
| cadrage par défaut, rendu Net | 6,4 | 6,3 | 6,8 |

Rien ne bouge : sur la machine de dev l'image reste égale au CPU, le GPU
n'est jamais le verrou. Deux essais pour le rendre visible, sans succès :
- `SURCHARGE_NUAGES=8` (les quads dessinés huit fois, retiré) : c'est le CPU
  qui monte, ~2,8 ms par répétition — **l'envoi des 36 000 quads coûte à lui
  seul ~2,8 ms des ~6,5 ms des nuages** (`push_quad`, la copie des lots dans
  `dessiner_par_lots`, celle de macroquad). Le GPU avale 800 écrans sans
  ralentir l'image. Piste pour O7.
- Un chronomètre GPU : `ElapsedQuery` de miniquad 0.4.10 est un bouchon
  (`get_result` rend toujours 0).

Le seul relevé où le GPU était le verrou est la première série, faite
pendant qu'un jeu le partageait : de près (dist 1,5), « tout » passait de
49 ms à 25 (1/2) et 22 (1/4). Indicatif — c'est sur la cible que le gain se
jugera.

Corrigé en route : `prototypes()` est relu à chaque image par la vue, et la
recherche de la T Tauri parcourait tout le défilé (~1,5 ms par image, même
sans nuages). Le rang est maintenant cherché une fois.

### Sur le petit PC, et l'adoption du 1/2 (2026-10-30)

`DIAG=1 PROTO=0/1/2`, 1920 × 1080, rendu Palette, image en ms :

| T Tauri | pleine | 1/2 | 1/4 |
|---|---|---|---|
| défaut, tout | 70,1 | 48,7 | 40,8 |
| défaut, rendu Net | 86,1 | 70,8 | 46,6 |
| défaut, sans nuages ni bras | 5,6 | 5,6 | 5,5 |
| de près (1,5), tout | 78,4 | 56,5 | 57,8 |
| de près, rendu Net | 91,5 | 96,7 | 87,6 |

- **Le remplissage était bien un verrou de la cible** : ~21 ms de moins à
  1/2, ~0,27 ms par écran repeint (~2 milliards de pixels par seconde).
- **1/4 n'apporte presque plus rien** (−8 ms par défaut, rien de près) :
  **1/2 retenu** par l'utilisateur.
- **Le plancher qui reste, 35 à 50 ms pour les nuages, n'est pas du
  remplissage** : c'est le calcul et l'envoi des ~36 000 boules par image
  (~6,5 ms sur la machine de dev, dont ~2,8 d'envoi). C'est le lot O7.
- Sur la cible, la colonne CPU du DIAG suit l'image même quand c'est le GPU
  qui coûte : le pilote fait attendre le processeur dans les dessins. Là-bas
  c'est la variation de l'image qui parle, pas l'écart image/CPU.
- Noté au passage : en rendu Net de près, le voile seul coûte 21 ms (5 au
  cadrage par défaut).

**L'adoption** : tous les disques dessinent leurs nuages et leurs bras à 1/2
(`disque::basse_res`, `REDUCTION`), l'étoile qui les cache est posée par
`Soleil::avec_disque` et par la galerie des disques (`set_rayon_central`).
La composition passe par le pipeline plasma — des UV décalées de 2 lui font
lire la cible telle quelle —, le compte de pipelines revient à 16. Prototype
retiré, touche 4 vide.

⚠️ Piège trouvé à l'adoption : `pop_camera_state` de macroquad ne rend pas
le viewport, et `get_viewport` rend l'**écran** quand aucun n'est posé. Le
reposer tel quel dans la cible deux fois plus petite du rendu Palette
grossissait toute la suite de l'image ; on ne repose qu'un viewport de
cellule (galeries).

## O7 : les nuages figés sur le GPU (2026-10-30)

**Où partait le temps** (trace temporaire, retirée ; T Tauri, vue isolée,
machine de dev, par image) : boules recalculées 1,9 ms — empilement 0,6 —
bras et jets 1,2 — envoi des quads 1,7 (dont ~1,4 pour les boules).

**O7a — les boules.** Un nuage ne fait que tourner : rayon, hauteur,
décalages de grappe, taille, teinte et éclat sont fixes, seul l'angle
avance (`ang0 + ω·temps`). `nuage_3d` est scindé — `nuage_fixe` en est la
partie fixe, source unique — et les boules vivent dans un buffer GPU
construit une fois par disque (`disque::nuages_gpu`). Le shader
`nuages.vert.glsl` les fait tourner, les éclaire (`eclairement_boule`,
gardée en règle de test) et pose leurs quads.

- **Un pipeline miniquad brut**, pas un `Material` : `draw_mesh` recopie
  les sommets dans le lot de macroquad à chaque appel, c'était le renvoi à
  supprimer. Il dessine directement dans la cible réduite d'O4, après la
  sphère qui cache.
- **Le côté du plan d'une boule ne change pas en tournant** : rangées
  dessus puis dessous, chaque moitié du disque dessine une tranche ; la
  qualité basse est le début de chaque tranche.
- **L'empilement reste sur le CPU** : il lui faut les positions écran, que
  `Boule::locale` redonne sans hachage.
- Test `les_boules_figees_tournent_comme_avant` : à trois instants, chaque
  boule tombe sur `nuage_3d` + son décalage, du bon côté, dans l'ordre —
  rouge vérifié par trois sabotages (côté inversé, décalage oublié, coupe
  de la qualité basse).

| machine de dev, image | avant | après |
|---|---|---|
| T Tauri, défaut | 6,3 ms | 3,2 ms |
| T Tauri, de près (1,5) | 6,3 ms | 3,2 ms |
| T Tauri, qualité basse | 3,4 ms | 2,5 ms |
| Herbig, défaut | 6,1 ms | 3,0 ms |

**L'image** : captures avant/après (ancienne version compilée à part),
quatre cadrages de la T Tauri en rendu Net : écart moyen 0,3 à 2,6/255
contre 7 à 8 d'une image à la suivante — les couleurs passent en flottant
au lieu de 8 bits, la luminosité moyenne bouge de +0,3/255. Galerie des
disques (rendu Palette) : identique à l'œil, quelques pixels changent de
couleur de palette au seuil.

**O7b — l'empilement sur un échantillon, la mesure seulement au DIAG.**
Après O7a, il restait ~2,4 ms de CPU : bras 1,2 — empilement 0,95 — envoi
des bras 0,3 — mesure du remplissage ~0,2 (trace temporaire, retirée).

- **L'empilement** ne sert qu'à un facteur global, le 98ᵉ centile des cases
  éclairées : une statistique, qui s'estime sur un échantillon. Essai sur
  six vues de la T Tauri (de près, de loin, par la tranche, de dessus) :
  une boule sur huit, de poids huit, redonne le pic à 3 % et l'amorti à
  0,001 près, pour un coût ÷6 ; compter un nuage entier comme une seule
  tache, non — l'amorti tombait à 0,65 au lieu de 0,99 vu de loin.
  Retenu : une boule sur `n / 4000` (au plus 8) — les petits disques des
  galeries restent comptés presque en entier (`echantillon`, test
  `l_empilement_echantillonne_garde_l_amorti`, rouge vérifié : poids
  oublié, échantillon trop maigre).
- **La mesure du remplissage** ne tourne plus qu'au DIAG
  (`mesurer_remplissage`). ⚠️ Le DIAG la paie donc encore : pour le temps
  du jeu, lire PROFIL.

| machine de dev, PROFIL | O7a | O7b |
|---|---|---|
| T Tauri | 346 fps (2,9 ms) | 523 fps (1,9 ms) |
| Herbig | 339 fps | 500 fps |

Image contre O7a : écart moyen 0,2 à 2/255, contre 7 à 8 d'une image à la
suivante.

**O7c — les jets sur le GPU.** Des bras qui restaient (~1,2 ms), le flot
bipolaire pesait 1,0 ms à lui seul (5 384 boules, T Tauri comme Herbig) ;
spirales 0,14 ms, entonnoirs 0,09 à 0,22 ms — laissés au CPU.

- `flot.rs` est scindé, sans changer un bit (empreinte de toutes les boules
  avant/après, quatre instants, trois flots) : `boules_fixes` (part, côté,
  phase de départ, tirages) et `animer` — `boules` en est la composition,
  si bien que les tests du flot éprouvent `animer`. Le shader
  `flot.vert.glsl` en est la copie.
- ⚠️ **Le tirage des plaques des parois a changé** : `h` y prenait le sinus
  de ~650 000, qui ne vaut pas la même chose d'un GPU à l'autre ni sur le
  CPU. `hache_plaque`, sans sinus, les tire pareil des deux côtés ; le
  motif des plaques est tiré autrement, même allure.
- `nuages_gpu` devient générique : buffers de quads, un pipeline par shader
  (nuages, flot), même fragment shader. Le côté du plan d'une boule de flot
  est son lobe.
- Test `le_flot_fige_se_range_par_lobe` (rouge vérifié).

| machine de dev, PROFIL | O7b | O7c |
|---|---|---|
| T Tauri | ~513 fps (1,95 ms) | ~895 fps (1,1 ms) |
| Herbig | 492 fps | 886 fps |

Image contre O7b, cinq cadrages en rendu Net : écart moyen 0,9 à 2,9/255
contre 1,8 à 8 d'une image à la suivante, luminosité +0,2 à +0,7/255
(couleurs en flottant). Tour `DIAG=tour` complet, 76 cas, sans erreur.

**Bilan d'O7** (machine de dev, en jeu) : T Tauri **158 → ~895 fps**
(6,3 → 1,1 ms), dont O7a ×2, O7b ×1,5, O7c ×1,7. Restent sur le CPU :
spirales et entonnoirs (~0,3 ms), l'empilement échantillonné (~0,15 ms).
Le gain sur la cible se lira au banc d'essai de fin de chantier.

## O5 : la résolution de rendu et le plafond d'images (2026-11-01)

Deux réglages de plus dans l'écran des paramètres (`reglages.json`, relu
sans perte depuis les fichiers d'avant) :

- **RÉSOLUTION 3D** — 100 / 75 / 50 %, en **rendu Net** seulement (grisé
  sinon) : les astres passent par une cible réduite, agrandie avec lissage
  (`rendu.rs`, la même passe que le rendu Pixel). Le rendu Pixel garde sa
  taille de pixel, qui fait son style. Les galeries, outils de travail,
  restent à pleine résolution. Machine de dev, T Tauri, 2560 × 1440,
  PROFIL : **547 → 894 fps** à 50 %.
- **IMAGES** — libre / 60 / 30 par seconde : le jeu attend l'échéance avant
  de présenter l'image (`attente_plafond`, échéances sur une grille fixe,
  sans dérive ; un gros retard repart d'ici au lieu de rattraper en
  rafale). Mesuré : **30,0 et 60,0 fps** pile. Ignoré sous `PROFIL` et
  `DIAG`, qui mesurent ce que la machine tient.
- La **qualité** reste Haute / Basse : un niveau Moyen n'aurait rien à
  régler aujourd'hui. Le **nombre d'objets** attend la construction de
  Genèse G1, comme prévu.
- Le menu passe à dix lignes : resserré (départ à 24 % de la hauteur,
  écart 8 px) — le test de chevauchement l'avait signalé à 700 px de haut.
- DIAG : une phase « tout, Net a 50 % » pour le banc d'essai ; les phases
  qui imposent le rendu Net le prennent à 100 %.
- Tests : `le_plafond_tient_sa_cadence_sans_deriver` (il a trouvé une
  dérive dans la première version : une image un peu en retard décalait
  toute la grille), `la_resolution_3d_ne_se_regle_qu_en_rendu_net` — rouges
  vérifiés.

## O6 : le shader de l'étoile (2026-11-01)

**Où il coûte.** Au cadrage par défaut, aucune étoile seule ne dépasse
~1 ms par image sur la machine de dev. Plein écran (rayon ~1 200 px,
2560 × 1440, rendu Net), l'étoile ajoute de 1,2 ms (G) à 3 ms (Mira).
Par pixel du disque : 5 `fbm` de 5 octaves (gauchissement ×3, grain,
points brillants), un `fbm` **par tache active** (jusqu'à 8), et sur les
géantes qui bouillonnent deux couches de cellules de 27 voisines.

**Le niveau de détail selon la taille, prévu au plan, est écarté** :
petite, l'étoile ne coûte déjà rien ; de près, tous les octaves se voient,
les retirer changerait l'image. Le levier de près est la résolution de
rendu (O5).

**Deux calculs inutiles retirés, à image identique** (leur contribution
est exactement nulle là où on les saute) :
- le bruit du contour d'une tache, calculé sur toute l'étoile pour chaque
  tache : sauté au-delà de 1,12 fois son rayon (`bord` ne dépasse pas
  1,11) ;
- le bruit des taches d'accrétion (T Tauri, Herbig) : sauté hors des
  calottes polaires, où `choc` vaut 0.

| de près, machine de dev, PROFIL | avant | après |
|---|---|---|
| G (jaune) | 630 fps | 727 fps |
| Supergéante rouge | 360 fps | 409 fps |
| Mira | 252 fps | 269 fps |
| T Tauri, sans nuages | 506 fps | 544 fps |
| Herbig, sans nuages | 472 fps | 514 fps |

**Essayé et retiré** : sauter, dans la recherche des cellules, les
voisines trop loin pour compter (exact aussi) — plus lent (409 → 384 fps
sur la supergéante rouge) : le branchement coûte plus que les sinus
épargnés.

**L'image** : captures avant/après de près (G, T Tauri : identiques, au
compteur de fps près ; supergéante rouge, Mira : l'écart est celui de
deux lancements de la même version, 15 à 45 % des pixels — les éruptions
et l'ébullition ne sont pas rejouées à l'identique d'un lancement à
l'autre).

## O8 : le banc du petit PC (2026-11-02)

**Le petit PC a changé le 2026-10-30**, entre O4 et O7 : mémoire 8 → 16 Go,
pilote Adrenalin à jour, Windows en mode performance. Si les 16 Go sont
passés en double canal, la bande passante de l'iGPU a doublé. Les écarts
ci-dessous mêlent donc **le code et le matériel** ; le banc complémentaire
rejoue la version d'avant O4 (`2244df5`) sur le PC modifié pour les séparer.

Premier banc, tous les lots livrés, 1920 × 1080, rendu Palette, `DIAG=1`,
image en ms (la phase « tout » prise dans sa reprise « bis ») :

| T Tauri | avant O4, ancien PC | aujourd'hui |
|---|---|---|
| défaut, tout | 70,1 | **7,3** |
| défaut, rendu Net | 86,1 | 10,4 |
| de près (1,5), tout | 78,4 | **9,2** |
| de près, rendu Net | 91,5 | 20,2 |
| de près, Net à 50 % | — | 9,0 |
| de près, voile seul en Net | 21 | 12,0 |

La Herbig suit la T Tauri à 10 % près. Le tour (`DIAG=tour`, 76 cas) :

| Cas | 2026-10-28 (1536 × 864) | aujourd'hui (1920 × 1080) |
|---|---|---|
| isolée T Tauri | 70,9 ms | 7,6 ms |
| système solaire | 63,8 | 9,6 |
| système T Tauri | 32,2 | 5,2 |
| starmap | 8,5 | 2,3 |
| galerie des planètes | 11,4 | 12,1 |

- **L'objectif du chantier est atteint sur ce banc** : 30 fps au moins
  partout ; le pire cas du tour, la **graine 2**, tient 63 fps.
- **Ce qui reste lourd** : la graine 2, limitée par le processeur (11,1 ms
  de CPU sur 15,9) ; la galerie des planètes (12,1 ms, GPU : 2,4 de CPU) ;
  le **rendu Net plein écran de près**, 49 fps, dont 12 ms pour l'étoile
  seule — Net à 50 % le ramène à 111.
- **La colonne CPU ne suit plus l'image** (voile seul en Net : 0,8 ms de CPU
  pour 12 ms d'image) : avec le nouveau pilote, le processeur n'attend plus
  dans les dessins. L'écart image/CPU redevient lisible sur la cible.
- ⚠️ La toute première phase du premier lancement a fait 36,3 ms contre 7,3
  pour sa reprise : la chauffe de 4 s ne suffit pas au **premier lancement
  après compilation** (le pilote compile ses shaders). Les lancements
  suivants n'en souffrent pas.
- **Le banc ne couvrait pas** : les étoiles lourdes de près (seules la
  T Tauri et la Herbig l'ont été), le tour en rendu Net, les systèmes zoomés
  (`FOCUS` n'existe que pour le système solaire). Le banc complémentaire
  couvre les deux premiers.
