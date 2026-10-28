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
