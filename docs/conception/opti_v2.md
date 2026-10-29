# Chantier-Opti-v2 — la dette de performance d'Astres V2

> **Ouvert le 2026-10-26**, à la clôture du chantier Astres V2 : *« cartographier
> l'intégralité de la dette du chantier Astres V2 sur l'optimisation, dans le but
> de faire tourner le jeu à 30 fps minimum sur un PC low-end (premier APU Ryzen
> portable) »*. Ce document est la **carte** : ce qui coûte, combien, pourquoi,
> et dans quel ordre s'y prendre. Rien n'est encore optimisé.

## 1. La cible, et ce qu'elle veut dire sur la machine de développement

**Cible** : un des premiers APU Ryzen portables — Ryzen 5 2500U / Ryzen 7 2700U
(Zen 1, 4 cœurs / 8 fils, 2 à 3,6 GHz), GPU intégré Vega 8 ou 10 (~1,1 à 1,7
TFLOPS), mémoire **partagée** avec le processeur. **Objectif : 30 fps au « 1 %
bas »**, pas seulement en moyenne — c'est le bafouillement qui se voit.

**Machine de développement** : Ryzen 7 3800X, RTX 3090.

| | Rapport estimé (cible plus lente de…) | Budget d'une image à 30 fps sur la cible → sur la machine de dev |
|---|---|---|
| Processeur (un fil) | ×2 à ×2,5 (Zen 1 portable contre Zen 2 de bureau) | 33 ms → **~14 ms** |
| Carte graphique | **×25 à ×30** (iGPU contre RTX 3090) | 33 ms → **~1,1 ms** |

⚠️ Ces rapports sont des **estimations** (puissances publiées), pas des
mesures. Ils disent surtout une chose : **un coût GPU invisible ici peut
suffire à lui seul à faire tomber la cible sous 30 fps.** Le chantier devra
finir par une mesure sur la vraie machine (§ 6).

## 2. Les mesures (2026-10-26, machine de dev, synchro verticale coupée)

Harnais : `PROFIL=<libellé>` (10 s d'échantillons, moyenne, 1 % bas, pire
image), avec désormais la **synchro coupée** (sinon tout plafonne à la
fréquence de l'écran), et `DEFILE=<rang>` (vue isolée) ou **`SYSTEME=<type>`**
(vue système, nouveau). Mode de rendu : Palette (celui de l'utilisateur).

### Vue isolée (un astre, caméra au cadrage par défaut du défilé)

| Astre | Moyenne | 1 % bas |
|---|---|---|
| Naine noire | 2 604 | 1 319 |
| Pulsar | 2 159 | 1 128 |
| **G (jaune), référence** | 2 069 | 1 170 |
| Magnétar | 1 971 | 895 |
| Géante rouge | 1 375 | 823 |
| Trou noir | 1 244 | 704 |
| Naine blanche polluée | 818 | 549 |
| Étoile Be | 527 | 402 |
| Hypergéante | 235 | 181 |
| **T Tauri** | **41** | **34** |
| **Herbig Ae/Be** | **37** | **30** |

Sur la machine de dev, **les deux jeunes étoiles sont déjà sous 60 fps.**

### Vue système (le système entier au cadrage par défaut, graine 1)

| Système | Moyenne | 1 % bas |
|---|---|---|
| Naine blanche polluée | 904 | 588 |
| Pulsar | 887 | 604 |
| G (jaune) | 777 | 515 |
| Hypergéante | 727 | 486 |
| Trou noir | 683 | 480 |
| Étoile Be | 611 | 442 |
| Herbig Ae/Be | 443 | 330 |
| **T Tauri** | **376** | **259** |

En vue système, le niveau de détail tient le petit disque (sous 40 px de rayon
apparent, ni nuages ni bras) ; ce qui coûte encore, c'est le **grand disque
protoplanétaire** posé par la genèse (500 grappes, ~3 000 boules), qui, lui,
est assez grand à l'écran pour garder ses nuages.

### La T Tauri, décomposée (vue isolée, en débranchant un effet à la fois)

| Ce qui reste branché | Moyenne |
|---|---|
| tout | 38 |
| tout sauf l'**envoi** des nuages (calcul seul) | 47 |
| tout sauf les **nuages, bras et jet** | 513 |
| ni nuages, ni **voile** | 1 984 |

Et, chronométré dans `dessiner_nuages_3d` (28 800 boules, **par appel** — il y
a **deux appels par image**, un par moitié) :

| Étape | Vue isolée | Vue système (grand disque, 3 000 boules) |
|---|---|---|
| Construire les boules | 2,4 ms | 0,20 ms |
| L'empilement (`Empilement`, amorti global) | **7,2 ms** | 0,37 ms |
| Envoyer les quads (lots de 800) | 2,0 ms | 0,05 ms |
| **Par image (×2)** | **~23 ms** | **~1,2 ms** |

## 3. La carte de la dette

Classée par gravité **sur la cible**. CPU = processeur, GPU = carte graphique.

| # | Dette | Où | Coût mesuré (dev) | Sur la cible (estimé) | Gravité |
|---|---|---|---|---|---|
| D1 | **Les nuages calculés deux fois par image** : `dessiner_nuages_3d` est appelé pour chaque moitié du disque, refait **toutes** les boules et l'empilement, et n'en garde que la moitié | CPU — `disque/mod.rs` | ×2 sur tout D2-D4 | ×2 | 🔴 |
| D2 | **L'empilement dégénéré** : la taille des cases est la *moyenne* des rayons apparents ; caméra **dans** le disque, les boules proches (z → 0) la font exploser → **une seule case** (`cote` 1), et 7 ms quand même (chaque boule balaie son voisinage de cases). En vue système, l'inverse : grille 512² (262 000 cases) remise à zéro à chaque appel | CPU — `Empilement` | 7,2 ms / 0,37 ms par appel | ~32 ms / ~1,6 ms par image | 🔴 |
| D3 | **Les boules reconstruites à chaque image** : position (`hache`, sinus, grappe), taille, teinte, éclairement — alors que tout sauf l'angle orbital est fixe | CPU — `nuage_3d`, `boule_de_grappe` | 2,4 ms par appel | ~11 ms par image | 🟠 |
| D4 | **Les quads envoyés depuis le CPU** à chaque image (`push_quad`, lots de 800) — 28 800 boules + jet (~5 000 par côté) + bras | CPU — `dessiner_nuages_3d` | 2,0 ms par appel | ~9 ms par image | 🟠 |
| D5 | **Le voile en couches** : 7 couches (T Tauri, Herbig ; 3 pour la Be) × 2 moitiés = **14 passes** de l'annulus plein, shader de bruit à chaque fois, sur un disque qui remplit l'écran de près | **GPU** — `disque_voile.frag`, `couche_voile` | ~1,4 ms par image | **~35-40 ms** par image | 🔴 |
| D6 | **Le remplissage additif des nuages de près** : des milliers de grosses boules transparentes superposées couvrent l'écran — la chute sous 40 fps signalée en jeu, tout près d'un disque | **GPU** — nuages, jet, bras | non isolé (masqué par D1-D4 ici) | élevé, de près | 🔴 |
| D7 | **Le shader de l'étoile s'est alourdi** au fil d'Astres V2 : ébullition (bulles, cellules), surface par type, champ magnétique, taches d'accrétion, cœur, aplatissement — calculés sur tout le quad de l'étoile et de sa couronne. Négligeable ici (G : 2 000 fps), mais plein écran de près sur un iGPU | GPU — `soleil.frag.glsl` | < 0,5 ms | 5-15 ms de près | 🟠 |
| D8 | **Allocations par image** : `Vec` de 28 800 boules, `Vec` des bras, du jet, des fragments… recréés à chaque appel | CPU | inclus dans D3-D4 | — | 🟡 |
| D9 | **Le grand disque en vue système** : 3 000 boules + empilement 512², deux fois | CPU | ~1,2 ms par image | ~3 ms | 🟡 |
| D10 | **Les rubans recalculés par image** : arches (`le_long`), faisceaux du pulsar, boucles du magnétar | CPU | négligeable (pulsar 2 159 fps) | faible | 🟢 |
| D11 | **Non mesurés** : queues de comète (`comete::nuage`, `fluctuer`), galeries (dizaines de cases animées à la fois), fond étoilé | CPU / GPU | — | ? | ❔ |

**Ce qui n'est pas de la dette** : le niveau de détail des nuages (§ lot 12)
fonctionne — en vue système le petit disque ne coûte rien ; les particules des
ceintures sont animées par le GPU depuis toujours (P.2 : « rien à optimiser
là ») ; les rémanents du lot 13 et les naines des lots 14-15 sont légers.

## 4. Les leviers

**Sur le CPU (D1-D4, D8, D9)** — le plus gros gain, le plus sûr :
- **Une seule passe de calcul par image** (D1) : construire les boules une
  fois, les ranger par côté du plan, dessiner chaque moitié depuis ce rangement.
  Gain attendu : ×2 sur tout le reste.
- **Réparer l'empilement** (D2) : la *médiane* des rayons apparents plutôt que
  la moyenne (les boules collées à la caméra ne la tirent plus), une grille
  bornée (64² au plus), ou le pic estimé sur un échantillon, ou recalculé une
  image sur N (l'atténuation varie lentement).
- **Des nuages figés sur le GPU** (D3, D4, D8) : la même recette que les
  particules des ceintures, qui sont un maillage construit une fois et animé
  par le shader (`disque_particules.vert` reconstruit l'orbite de Kepler). Les
  boules ne bougent qu'en angle : leur partie fixe peut vivre dans un maillage
  statique, l'angle et l'atténuation passer en uniforms. C'est le levier
  radical — et un vrai chantier (le côté du plan, l'éclairement par boule et
  l'amorti devront passer dans le shader).

**Sur le GPU (D5-D7)** — le plus gros risque sur la cible :
- **Le voile en une passe** (D5) : boucler sur les couches **dans** le fragment
  shader, avec sortie anticipée, au lieu de redessiner l'annulus sept fois ;
  et moins de couches quand le disque est petit à l'écran.
- **Borner le remplissage** (D6) : plafonner la taille écran d'une boule, en
  dessiner moins quand le disque déborde l'écran.
- **Un niveau de détail du shader de l'étoile** (D7) : moins d'octaves de bruit
  et pas d'ébullition sous une taille écran donnée.

**Transverses** :
- **Un réglage de qualité** (Bas / Moyen / Haut) qui pilote nombre de boules,
  couches du voile, octaves, et — surtout — la **résolution de rendu**. Le mode
  Palette rend déjà dans une cible basse résolution : c'est le premier levier
  sur un iGPU, il suffit d'en faire un réglage à part entière.
- **Un plafond d'images** (30 / 60) pour ne pas chauffer une machine portable.
- **Où poser la qualité** (ajouté le 2026-10-28) : les nombres de particules
  des ceintures (900, 1 400) et des disques sont aujourd'hui écrits **dans la
  genèse**, au milieu des tirages — les baisser changerait les systèmes. Le
  lot G1 de [`genese_v2.md`](genese_v2.md) (§5.4) sépare le plan
  (ce qui existe, tiré) de la construction (combien on en dessine, sans tirage) :
  c'est là que O5 réglera la richesse du dessin, à système identique. Même
  découpage, deux gains de plus : la construction à la demande (G1 bis) et
  les plans lus sans rien construire (starmap).

## 5. Les lots proposés, dans l'ordre

| Lot | Contenu | Dette | Gain attendu (T Tauri, vue isolée, dev) |
|---|---|---|---|
| O1 | Calcul unique par image, buffers réutilisés | D1, D8 | 38 → ~75 fps |
| O2 | Empilement réparé (médiane, grille bornée) | D2, D9 | → ~150 fps |
| O3 | Voile en une passe, couches selon la taille écran ✅ | D5 | voile de la T Tauri ÷5 (dev) ; les voiles à une couche (anneaux, ceintures) ne changent pas |
| O4 | Remplissage borné | D6 | la chute de près |
| O5 | Réglage de qualité, résolution de rendu, plafond d'images — la part « nombre d'objets » s'accroche à la construction de Genèse G1 | transverse | la marge sur la cible |
| O6 | LOD du shader de l'étoile | D7 | de près, sur la cible |
| O7 | Nuages figés sur le GPU (si O1-O2 ne suffisent pas) | D3, D4 | le reste du CPU |
| O8 | **Mesure sur la vraie machine** (ou la plus proche disponible) | — | la seule preuve de l'objectif |
| O9 | Engins (ISS, Starship) en point sous 2 px — trouvé par `DIAG=tour` ✅ | — | système solaire 169 → 484 fps (dev) |
| O10 | Quantification de palette en table précalculée ✅ | — | passe Palette ~0,5 ms sur la cible ; les « ~8 ms » étaient un biais de chauffe du DIAG |

Après O1 et O2, le calcul des nuages de la T Tauri tomberait de ~23 ms à ~5 ms
par image sur la machine de dev — soit ~11 ms sur la cible, dans le budget
CPU. **Le GPU resterait le verrou** (O3, O4), et lui ne se juge que sur la
cible.

## 6. La méthode

- **Mesurer avant et après chaque lot**, avec le harnais : `PROFIL=<libellé>`
  et `DEFILE=<rang>` (vue isolée) ou `SYSTEME=<type>` (vue système). Synchro
  coupée sous `PROFIL`.
- **Débrancher un effet à la fois** pour isoler un coût (c'est ainsi qu'ont été
  obtenus les tableaux du § 2 : interrupteurs temporaires, retirés après).
- **Le 1 % bas** est le critère, pas la moyenne.
- **Le GPU ne se juge pas sur une RTX 3090.** Tant que la cible n'est pas
  disponible, un coût GPU se lit au mieux en *rapport* (ce qu'un effet ajoute
  par rapport à l'étoile G de référence), et se confirme sur la vraie machine.
