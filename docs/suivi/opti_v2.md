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

**Suite : O2**, l'empilement réparé (7,2 ms par calcul, désormais une seule
fois par image — le plus gros poste restant).
