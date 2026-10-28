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
