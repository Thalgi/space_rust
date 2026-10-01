# La lentille gravitationnelle — recherche

> 2026-10-01. Le lot 8.2 a été livré puis jugé : « ça rend mal ». Recherche
> demandée, menée sur la littérature et sur les implémentations temps réel
> existantes, puis chiffrée sur nos propres constantes.
>
> **Conclusion courte : la formule était fausse et la géométrie trop petite.**
> Les deux défauts sont mesurés ci-dessous, et aucun n'est une question de
> dosage.

---

## 1. La formule que j'avais employée n'est pas celle d'un trou noir

J'ai pris la **lentille mince** `β = θ − θ_E²/θ`, l'équation d'une masse
ponctuelle avec une source à distance finie. Elle produit bien un anneau, mais
elle décrit un autre objet : elle ignore la sphère de photons, donc l'ombre, donc
l'empilement d'images à son bord — c'est-à-dire tout ce qui fait reconnaître un
trou noir.

La littérature donne **deux régimes**, et les implémentations temps réel
sérieuses emploient les deux :

| Régime | Formule | Domaine |
|---|---|---|
| **Champ faible** | `α = 2 r_s / b` | `b ≫ b_c` |
| **Déviation forte** (Darwin, précisée par Bozza) | `α = −π + ln[216(7 − 4√3) / b′]`, avec `b′ = 1 − b_c/b` | `b → b_c⁺` |

Le **paramètre d'impact critique** vaut `b_c = 3√3 m = 2,598 r_s` — *exactement*
le rayon d'ombre que le chantier emploie déjà depuis la recherche sur le disque
d'accrétion. Les deux conventions se rejoignent sans rien convertir : **dans nos
unités, `b_c = 1`**, et `r_s = 0,385`.

Nos constantes deviennent donc, mesurées :

- champ faible : `α = 0,7698 / b`
- déviation forte : `α = −0,4002 − ln(1 − 1/b)` (car `ln[216(7−4√3)] = 2,7414`)
- **croisement des deux : `b = 1,764`**

| `b` (rayons d'ombre) | `α` | en degrés |
|---|---|---|
| 1,02 | 3,53 | **202°** |
| 1,10 | 2,00 | 114° |
| 1,50 | 0,70 | 40° |
| 2,00 | 0,38 | 22° |
| 3,40 | 0,23 | **13°** |
| 10 | 0,077 | 4,4° |
| 40 | 0,019 | 1,1° |

La divergence logarithmique au bord de l'ombre est **ce qui fait l'image** :
l'anneau d'Einstein et l'empilement d'images en sortent tout seuls. La lentille
mince ne les produit pas.

## 2. Et le quad était beaucoup trop petit

`α` décroît en `1/b` — très lentement. Mesuré sur notre cadrage (1440 p,
demi-champ 22,5°, un pixel ≈ 0,00055 rad) :

> **La déviation ne tombe sous un pixel qu'à `b > 1411` rayons d'ombre.**

Mon quad s'arrêtait à **3,4**, où la déviation vaut encore **13°**. L'atténuation
que j'y avais mise pour « raccorder le ciel » n'adoucissait donc pas un détail :
elle **écrasait l'essentiel de l'effet** sur toute la moitié extérieure du quad.

⚠️ **Mais la conclusion n'est pas « il faut un quad de 1400 rayons ».** Sur un
champ d'étoiles sans structure, un décalage **rigide** du ciel est indétectable —
rien ne permet de dire qu'une étoile n'est pas à sa place. Ce qui se voit est le
**gradient** de la déviation, c'est-à-dire l'étirement.

Gradient en champ faible : `dα/db = −0,7698 / b²`. Rapporté à la largeur
angulaire d'un pixel (`Δb = 0,0055` rayon d'ombre pour une ombre de ~100 px) :

| `b` | étirement par pixel |
|---|---|
| 2 | 1,9 px |
| 3 | 0,85 px |
| 5 | 0,31 px |
| 10 | 0,08 px |

**L'étirement devient sub-pixel vers `b ≈ 6`.** Un quad de **12 rayons d'ombre**
laisse donc toute la distorsion visible à l'intérieur, et l'atténuation du reste
sur son tiers extérieur introduit un faux gradient de l'ordre de **0,16 px par
pixel** — invisible, et **vérifiable par un test**.

C'est la vraie réponse : pas « local ou global », mais **assez grand pour
contenir le gradient**, ce qui est douze fois plus que ce que j'avais pris.

## 3. Ce qu'il faut corriger, par ordre

| # | Correction | Pourquoi |
|---|---|---|
| **L1** | **Déviation à deux régimes** : `max(0,7698/b, −0,4002 − ln(1−1/b))` | La lentille mince ne décrit pas un trou noir. C'est la divergence logarithmique qui donne l'anneau |
| **L2** | **Direction échantillonnée `φ = θ + α`**, et non `β = θ − θ_E²/θ` | `φ` dépasse π près de l'ombre : le ciel s'y enroule, et les images d'ordre supérieur apparaissent gratuitement |
| **L3** | **Quad à 12 rayons d'ombre** au lieu de 3,4 | À 3,4 la déviation vaut encore 13° : le quad coupait l'effet en son milieu |
| **L4** | **Atténuation sur le tiers extérieur seulement**, et justifiée au pixel | Le faux gradient qu'elle introduit doit rester sub-pixel — ça se calcule, donc ça se teste |
| **L5** | **`b = rr` directement** : le paramètre d'impact en rayons d'ombre **est** la coordonnée du quad | `b/b_c = θ/θ_s`, et notre `rr` vaut déjà `θ/θ_s`. Aucune conversion, donc aucune occasion de se tromper d'unité — celle qui a donné l'impostor carré |

## 4. Ce qui restera limité, et qu'il ne faut pas espérer corriger ainsi

- **La résolution de la cubemap.** Une face de 256 texels couvre 90°, soit
  0,35° par texel, contre 0,031° pour un pixel d'écran : elle est **onze fois
  plus grossière**. Près de l'anneau, où la lentille grossit énormément, les
  étoiles s'étaleront en taches. Monter à 1024 coûterait 24 Mo et ne ferait que
  repousser le problème d'un facteur quatre.
- **La densité du ciel.** 700 à 900 étoiles sur toute la sphère : l'anneau en
  montrera quelques-unes, pas une couronne. Densifier le champ densifierait le
  ciel **partout** — c'est un choix d'aspect global, pas un réglage de lentille,
  et il ne doit pas être pris en douce ici.
- **Les planètes ne sont pas déviées**, décision déjà prise et inchangée : une
  cubemap encode des directions à l'infini.

## 5. Ce qui se teste, et ce qui ne se teste pas

**Se testent** — et c'est ce que « bien capitonné » veut dire ici :

- les deux régimes de `α` pris séparément (asymptotes, signe, monotonie) ;
- leur **croisement** à `b = 1,764`, et la continuité au raccord ;
- la divergence au bord de l'ombre : `α(1,02) > 3 rad` ;
- la décroissance en `1/b` au loin : `α(40) ≈ 2 α(80)` ;
- le **faux gradient** de l'atténuation, en pixels, sous le seuil d'un pixel ;
- que `b` et la coordonnée du quad soient la **même** grandeur (c'est la faute
  de l'impostor carré, en unités).

**Ne se testent pas** : ce qui part au GPU. La formule vit en Rust **et** en
GLSL — c'est une duplication assumée, du même ordre que `PORTEE` dans le champ
magnétique, et elle se garde comme elle : par des tests de littéraux qui
comparent les deux textes.
