# Un disque d'accrétion qui se lise comme tel

> Recherche préalable, 2026-09-26. Le lot 5 a donné au disque d'accrétion un
> **plan** — il s'incline, on peut en faire le tour, il se voit par la
> tranche. Le résultat ne convainc pas. Hypothèse de l'utilisateur : il
> faudrait un **effet dédié**, du même ordre que le chantier « ceintures &
> disques », plutôt qu'un réglage de l'existant. Ce document vérifie cette
> hypothèse, et dit ce que coûterait chaque manière d'y répondre.
>
> Contrainte posée : du réalisme, **sans forcer les performances**.

---

## 0. Ce qui est mesuré, et ce qui est su

Distinguer les deux évite de bâtir sur une impression.

**Lu dans le code** (`disque/voile.rs`, `shaders/disque_voile.*`) :

- Le voile est un **annulus plat**, `N_RADIAL = 10` bandes × `N_SEG = 96`
  secteurs, soit **960 quads / 3 840 sommets**, figé une fois pour toutes ;
  l'orientation passe en uniforms. Dessiné en deux moitiés → **7 680
  sommets/frame** par disque. À comparer aux 25 600 sommets d'un vaisseau et
  aux 104 658 d'un tore de Stanford (`astres_v2_boucles.md` §3) : **le disque
  ne pèse rien**.
- `position.y` vaut **toujours 0** : c'est une feuille sans épaisseur.
- Le fragment shader porte déjà la **rotation képlérienne différentielle**
  (`pow(rr, 1.5)`, l. 71-72) et **cisaille** la granulation avec elle
  (l. 119-122). Ce n'est donc pas un anneau figé.
- Il porte **27 uniforms**, dont `lacunes`, `arcs`, `ringlets`,
  `ombre_rayon` — le vocabulaire des **anneaux planétaires**.
- **13 `load_material` dans tout le dépôt**, tous derrière un cache, pour un
  plafond moteur de **32 pipelines** (`suivi/technique.md` P.2 ter). Un
  pipeline de plus est abordable ; dix ne le seraient pas.

**Su de l'astrophysique**, et non vérifiable ici :

- Disque mince standard (Shakura-Sunyaev) : `T(r) ∝ r^(-3/4)`. Le bord
  interne n'est pas « un peu plus chaud », il est **beaucoup** plus chaud, et
  la luminosité de surface suit `T⁴`.
- Trou noir de Schwarzschild : dernière orbite stable à **3 rayons**, sphère
  des photons à 1,5, et l'**ombre apparente** — ce qu'un observateur lointain
  voit comme le disque noir — fait ≈ **2,6 rayons**, donc plus grande que
  l'horizon lui-même.
- **Beaming Doppler** : le flux observé varie comme `δ^(3+α)`, `δ` étant le
  facteur Doppler. Aux vitesses du bord interne, le côté qui s'approche peut
  être **plusieurs fois** plus brillant que celui qui s'éloigne — c'est
  l'asymétrie criante de l'image de M87\* par l'EHT, et de Gargantua.
- L'image iconique d'un disque vu de biais ne doit pas sa forme à sa
  géométrie mais à la **déviation des rayons lumineux** : la face arrière du
  disque, physiquement *derrière* le trou, apparaît **au-dessus** de lui, et
  sa face inférieure **au-dessous**. L'anneau se referme sur l'astre.

---

## 1. Pourquoi le résultat actuel ne convainc pas

Quatre causes, de la plus décisive à la plus cosmétique. Cet ordre est un
**jugement**, pas une mesure.

### 1.1 Il manque la seule chose que tout le monde a en tête

Sans déviation des rayons, un disque incliné **est** un anneau incliné. La
silhouette obtenue est celle de Saturne : une ellipse qui passe devant et
derrière une boule. Or personne n'a cette image en tête en pensant « trou
noir » — on a celle d'un **anneau qui enjambe l'astre**, passant par-dessus
et par-dessous.

C'est, à mon sens, l'essentiel de l'écart. Aucun réglage de couleur, de
turbulence ou de beaming ne le comble : c'est une affaire de **silhouette**.

### 1.2 Le vocabulaire est celui d'un anneau de Saturne

Le preset `accretion` est un `DisqueConfig` comme les autres. Il hérite de
`lacunes`, `arcs`, `ringlets`, `ombre_rayon` — qui n'ont aucun sens ici — et
ne dispose d'aucun des termes qui feraient un disque d'accrétion : profil de
température, épaisseur, évasement, assombrissement du bord interne,
décalage de couleur.

L'intuition de l'utilisateur est **juste** : ce n'est pas le même objet, et
le forcer dans le moule des anneaux revient à décrire un feu de forêt avec
le vocabulaire d'une cheminée.

### 1.3 Le gradient thermique est linéaire, la réalité est en puissance

Le voile fait `mix(couleur2, couleur, t)` : une interpolation **linéaire** du
bord interne au bord externe. Avec `T ∝ r^(-3/4)` et une émission en `T⁴`, la
luminosité de surface s'effondre comme `r^(-3)` — le bord interne devrait
**écraser** le reste, et le bord externe être presque éteint. Un dégradé
linéaire donne une galette uniformément lumineuse, qui se lit comme un
anneau de matière éclairé de l'extérieur, et non comme du gaz qui rayonne
parce qu'il brûle.

### 1.4 Le beaming ne change que l'éclat, pas la couleur

Le lot 5 multiplie la luminosité par `1 + doppler·(v·tocam)`. C'est la moitié
de l'effet : le décalage vers le bleu du côté qui s'approche, vers le rouge
de celui qui s'éloigne, manque. Et l'asymétrie réelle est bien plus violente
que ce que `doppler = 0,8` produit.

---

## 2. Piste A — enrichir le voile partagé

Ajouter au `DisqueConfig` commun les termes manquants : exposant de
température, évasement, décalage de couleur.

**Ce que ça donne** : les points 1.3 et 1.4 traités, 1.1 et 1.2 intacts.

**Coût** : trois ou quatre uniforms de plus sur un shader qui en porte déjà
27, et **aucun pipeline**. Le moins cher de tous.

**Ce que ça ferme** : rien, mais ça **alourdit le voile commun** — chaque
anneau de Saturne du jeu traînerait désormais des uniforms d'accrétion qu'il
laisse à zéro. C'est la trajectoire qui a mené aux 27 uniforms actuels, et
elle a une fin.

**Verdict** : utile, insuffisant seul, et il faut savoir où l'on s'arrête.

---

## 3. Piste B — un shader dédié

Un second fragment shader sur **le même mesh d'annulus** et la même
mécanique d'uniforms, avec son propre vocabulaire : profil de température en
puissance, évasement, turbulence cisaillée, beaming coloré, assombrissement
au bord interne.

**Ce que ça donne** : 1.2, 1.3 et 1.4 traités proprement. 1.1 toujours pas.

**Coût** : **un pipeline** (14 sur 32) et un fichier `.glsl`. Le mesh, la
géométrie, le batching, le rendu en deux moitiés : tout est réutilisé tel
quel. Zéro sommet de plus.

**Ce que ça ferme** : deux shaders de disque à maintenir. Mais ils divergent
déjà dans les faits — la moitié des uniforms du voile ne sert pas à
l'accrétion, et l'inverse deviendrait vrai.

⚠️ **À faire avec sa garde.** Le voile n'avait **aucun** test
uniforms↔GLSL avant le lot 5 ; un second shader sans ce test reproduirait la
même dette. `uniforms_voile_correspondent_au_shader` est le modèle.

**Verdict** : c'est la réponse directe à l'hypothèse de l'utilisateur, et
elle est bon marché. Mais elle ne fait pas l'image.

---

## 4. Piste C — l'anneau qui enjambe l'astre

C'est le point 1.1, et donc le cœur du sujet. Deux façons.

### C1 — déformation en espace **objet**, dans le vertex shader

Au lieu de lensing, on **plie le maillage** : la moitié du disque située
derrière l'astre est relevée le long de l'axe vertical de l'écran, de sorte
qu'elle apparaisse au-dessus de l'horizon, et une seconde copie au-dessous.
La déformation se calcule par sommet à partir de `tocam` — déjà un uniform du
voile (`disque_voile.vert.glsl` l. 22) — donc **le mesh reste figé**.

**Ce que ça donne** : la silhouette. Un anneau qui se referme sur l'astre,
qui suit la caméra quand on tourne autour, et qui s'aplatit correctement
quand on revient dans le plan du disque.

**Coût** : une poignée d'opérations par sommet sur 3 840 sommets, plus
**une passe de plus** pour la copie relevée — soit ~11 500 sommets/frame par
trou noir au lieu de 7 680. Toujours moins de la moitié d'un vaisseau.
**Aucun pipeline, aucune texture, aucun ray tracing.**

⚠️ **Ce que ça exige** : `N_RADIAL = 10` est trop grossier pour un pli. Un
annulus d'accrétion aurait besoin de ~24 bandes radiales — 2 300 quads, soit
9 200 sommets. Reste négligeable, mais ce n'est plus le mesh partagé : il
faut le construire à part.

**Ce que ça ferme** : c'est un **trucage**, et il ne tient que tant qu'on
reste hors du disque. Vu depuis l'intérieur du plan, ou de très près, la
supercherie se voit. Il ne produit pas non plus l'anneau de photons
secondaire (l'image d'ordre 2), qui restera peint.

### C2 — déviation en espace **écran**

La vraie déviation : dévier les UV de l'arrière-plan autour de l'astre, comme
le prévoit le lot 8 (« lentille locale »).

**Coût** : une passe de post-traitement locale, une cible de rendu, et le
fond stellaire à échantillonner. C'est un **chantier de rendu** à part
entière, pas un lot d'astre — `astres_v2.md` §7 l'avait déjà écarté du
périmètre pour cette raison.

**Verdict C** : **C1**. Il donne 90 % de l'image pour 3 % du coût de C2, et
il se retire en une ligne s'il déplaît.

---

## 5. Piste D — volumétrique

Raymarcher un disque épais dans le fragment shader. Écartée pour la même
raison que les boucles de plasma (`astres_v2_boucles.md` §4) : **hors de
portée de GLSL ES 100** sur ce moteur, et sans commune mesure avec la
contrainte de performance posée.

---

## 6. Comparaison

| | **A · voile enrichi** | **B · shader dédié** | **C1 · pli objet** | **C2 · lentille écran** |
|---|---|---|---|---|
| Traite 1.1 (silhouette) | non | non | **oui** | oui |
| Traite 1.2 (vocabulaire) | non | **oui** | — | non |
| Traite 1.3/1.4 (physique) | oui | **oui** | — | non |
| Pipelines | 0 | +1 (14/32) | 0 | +1 et une cible de rendu |
| Sommets / frame | inchangé | inchangé | 7 680 → ~11 500 | inchangé |
| Nouveau mesh | non | non | oui (annulus fin) | non |
| Dette ajoutée | uniforms morts sur tous les anneaux | 2ᵉ shader à garder | trucage borné | chantier de rendu |
| Se retire facilement | oui | oui | **oui** | non |

---

## 7. Recommandation, en lots

**B + C1**, dans cet ordre, et **A abandonné** : ce que A apporterait, B le
fait mieux sans alourdir le voile commun.

- **Lot α — le shader dédié** (B). `shaders/disque_accretion.frag.glsl` +
  `DisqueAccretion` (ou un drapeau sur `Disque` qui choisit le material).
  Profil de température en puissance, beaming **coloré**, turbulence
  cisaillée, assombrissement du bord interne. Test uniforms↔GLSL **dès le
  premier commit**. *Rien ne change de silhouette — c'est voulu, ça isole la
  régression.*
- **Lot β — le pli** (C1). Annulus fin dédié, déformation par sommet pilotée
  par `tocam`, seconde passe pour l'image inférieure. **Le lot qui fait
  l'image.**
- **Lot γ (optionnel) — l'ombre**. Le disque noir apparent vaut 2,6 rayons,
  pas 1 : agrandir l'horizon peint et y poser l'anneau de photons au bon
  rayon. Trois lignes de shader, et c'est ce qui ancre l'échelle.

Le disque protosolaire de T Tauri **reste sur le voile partagé** : il n'a ni
beaming, ni bord interne brûlant, ni pli. C'est bien un anneau de matière,
et le vocabulaire des anneaux lui convient.

---

## 8. Ce qui reste incertain

- **L'amplitude du pli.** Trop faible, l'anneau ne se referme pas ; trop
  forte, il devient une soucoupe. Rien ne le déduit — c'est le même genre de
  seuil que `PLASMA_PX_MIN` ou `EVASEMENT` : une valeur posée, corrigée au
  jugé.
- **Le raccord entre le disque plat et son pli.** C'est là que le trucage se
  voit ou non, et je ne sais pas d'avance si une transition douce suffit.
- **L'exposant de température.** `-3/4` est la valeur physique, mais
  l'émission en `T⁴` peut rendre le bord externe si sombre qu'il disparaît.
  Il faudra probablement l'adoucir, et **le dire** plutôt que de le faire en
  douce.
- **L'échelle.** Rien de tout ceci ne se lit sous ~10 px de rayon
  (`astres_v2.md` §0). Le chantier vise la galerie, la vue objet et
  l'approche — comme tout le lot 3.

## 9. Ce qui ne se jugera qu'à l'œil

L'amplitude du pli et son raccord, la force du beaming, la dureté du profil
de température, le rayon de l'ombre. Aucun ne se déduit d'un calcul.

**Se testent, et doivent l'être** : la correspondance uniforms↔GLSL du
nouveau shader, le fait que le pli suive bien la caméra (une fonction pure
de `tocam` et de la position locale), que le preset d'accrétion se distingue
d'un anneau, et que l'annulus fin reste dans son budget de sommets.
