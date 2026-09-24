# Conception — Un seul champ magnétique pour la surface, les taches et les arches

> Étude de faisabilité, **2026-09-24**, écrite avant tout code et n'en produisant
> aucun. Elle prolonge [`astres_v2.md`](astres_v2.md) (§ 1 et § 2) sur le défaut
> signalé à l'œil : les taches ont l'air **posées** sur l'étoile, et les boucles
> de plasma n'ont aucun rapport visible avec elles. Règle du chantier
> ([`../methode.md`](../methode.md)) : **constater d'abord**, `fichier:ligne`
> à l'appui.

---

## 0. Ce qui a été vérifié — et une correction au cadrage

Le cadrage annonçait « trois sources indépendantes », dont un **champ magnétique
interne au shader de surface, vers la ligne 97**. Relu ligne à ligne : il n'y en
a pas. `soleil.frag.glsl:94-122` calcule la normale, la rotation propre, le
décalage de graine et la granulation — rien de magnétique. Le seul champ
magnétique du fichier est la branche **magnétar** (l. 179-187) : des arcs en
`r = L sin²θ` dans la **couronne**, sur un axe vertical câblé en dur, qui ne
touchent ni la surface ni les taches.

Ce ne sont donc pas trois sources qui divergent, ce sont **deux sources et un
trou** — rien à réconcilier, un champ à **créer**. Corollaire : aucune retouche
du bruit existant ne fera émerger une tache, puisque rien dans ce bruit ne sait
qu'une tache existe.

## 1. État des lieux du couplage

| Fait | Où il se décide | Ce que le voisin en sait |
|---|---|---|
| Granulation, campfires | `soleil.frag.glsl:109-122` (`fbm(d·11 + dec + warp·1,2)`) | Le CPU n'a **aucune** fonction qui l'évalue. Seul canal : `graine` (`mod.rs:60`). |
| Contour irrégulier d'une tache | `soleil.frag.glsl:141` (`bord = 0,78 + 0,34·fbm(d·9 + s.xyz·7 + dec)`) | Décidé au GPU, invisible au CPU : celui-ci croit ses taches rondes. |
| Ombre + pénombre, flare | `soleil.frag.glsl:145-148, 156-160` | — |
| Position, taille, durée d'une tache | `eruptions.rs:188-195` : `dir_aleatoire()` + `gen_range` | Aucun rapport avec la granulation, ni avec quoi que ce soit d'autre. |
| Tache porteuse d'une arche | `eruptions.rs:77-94`, `133-136` | **Acquis du lot 3** : le centre est une vraie tache active. |
| Orientation des pieds | `eruptions.rs:146-151` : tangente **tirée au sort** autour du centre | Rien ne dit dans quel sens l'arche devrait enjamber la tache. |
| Forme et rendu d'une arche | `rendu.rs:197-206` (Bézier), `104-147` (49 billboards additifs) | Second material, second pipeline (`materiau.rs:97-114`). |
| Transport CPU → GPU | `rendu.rs:30-45, 61` → `spots[8]` ; rayon **signé** (`eruptions.rs:98-111`) | **Le seul canal** entre le processeur et la surface. |

**Le diagnostic tient en une ligne.** Le seul fait partagé par les trois systèmes
est *la direction d'une tache* : un `vec3` et un rayon. Le shader ne sait ni
pourquoi elle est là ni comment elle est orientée ; le processeur ignore à quoi
ressemble la surface qu'il tache. L'« autocollant », c'est ça.

**Budget actuel.** `uniforms_soleil()` (`materiau.rs:57-69`) : 5 uniforms communs
+ `teinte`, `couronne`, `couronne_irreg`, `couronne_type`, `zone`, `graine`
+ `spots[8]`, sur **2 pipelines** (`PASSES`, `materiau.rs:22`) — plafond moteur
32 (`../suivi/technique.md`). Pour situer : le shader de planète en déclare
**71** uniforms, dont `vortex[8]`, `vortex2[8]`, `gaz_pal[8]`, deux tableaux de
lumières et **deux samplers** (`planete/materiau.rs:339-368`), et il tourne. Le
plafond qui mord ici est celui des **pipelines**, pas des uniforms.

## 2. Piste 1 — un champ précalculé sur processeur, passé en texture

### 2.1 `planete/terrain.rs` — l'atlas cube-sphere

- **Format** : atlas 3×2 faces, `N_ATLAS = 128` texels par face (l. 17),
  **gouttière de 1 texel** par re-projection du voisin (l. 957-990) pour que
  l'interpolation bilinéaire du GPU soit juste au passage des arêtes. RGBA8 :
  **altitude sur 16 bits répartie sur R+G** (l. 978-981), décodée par
  `altitude_atlas` (`planete.frag.glsl:173-176`) — déjà la réponse au « pas de
  texture flottante garantie ». **Coût** : 390×260×4 ≈ **0,4 Mo par planète** ;
  génération ~1 s (médiane 1 061 ms, érosion ~62 % — `../suivi/planetes.md`),
  donc **asynchrone** (`planete/mod.rs:318`, budget global de 2 jobs).
- **Transposé à l'étoile** : un champ magnétique n'a besoin ni d'érosion ni
  d'hydrologie — quelques octaves de bruit et un terme dipolaire. À **32²/face**
  (6 144 texels) l'atlas pèse ~**28 ko** et se calcule en ~5 ms mono-cœur ; à
  64²/face, ~102 ko et ~25 ms. ⚠️ La galerie en construit **21 d'un coup**
  (`galerie_etoiles.rs:7-33`) : 32² passe en synchrone (~100 ms de saccade),
  64² exige l'asynchrone de `terrain`.
- **Côté GPU le portage est déjà écrit** : `dir_vers_atlas`
  (`planete.frag.glsl:153-170`), dont le commentaire avoue le prix — « doit
  rester le miroir exact de `planete/terrain.rs` ». Nuance qui compte pour le
  § 3 : c'est de la **géométrie** (table de faces, `atan`), pas un hash — 1 ULP
  y décale d'un centième de texel, il ne change rien.

### 2.2 `planete/zonal.rs` — le vrai modèle, et le moins cher

Texture **256×1 RGBA8**, trois champs sur trois canaux, générée en **< 1 ms,
synchrone** (`planete/mod.rs:99-101`). Ce qui s'en transpose n'est pas la forme
1D — un champ stellaire est 2D sur la sphère — mais la **discipline** : la
texture n'est que l'*encodage* d'un `Profil` (l. 40-58) qui reste en Rust, avec
`u_at` / `b_at` comme accesseurs. Le **processeur interroge son propre champ**,
ce qui permet à `vortex.rs:84-96` d'y ancrer ses taches. Transposé : un maximum
local de `|B|` donne une tache, deux lobes opposés donnent les pieds d'une arche
— l'orientation cesse d'être tirée au sort (`eruptions.rs:147`).

Autre acquis : ces champs sont **testables sans GPU** — ni `zonal.rs:182-200` ni
`terrain.rs:1255-1276` n'appellent `Texture2D::from_rgba8`, ils testent le
`Profil` et l'atlas en `Vec<u8>`. `methode.md` appliqué.

### 2.3 Deux pièges hérités, et ce que la piste ferme

- **Texture par astre + material partagé** (`../suivi/technique.md`, D-GPU-1) :
  `set_texture` écrit dans le pipeline, pas dans le clone — la dernière étoile
  dessinée imposerait son champ aux autres. Remède en place (`rendu.rs:74-76`),
  mais le `set_texture` doit tomber **entre deux vidages**, et rien ne le garde.
- **Hot-reload** : `galerie_etoiles.rs:72-74` ne fait que `vider_cache_materials()`
  — il ne reconstruit **pas** les étoiles, contrairement à `galerie.rs:115-118`
  qui appelle `construire()`. Une texture portée par le `Soleil` **survit donc à
  R** : on itère sur le `.glsl` sans repayer la génération ; contrepartie,
  toucher au **générateur Rust** demande un redémarrage.
- **Ce que ça ferme** : un sampler de plus, à déclarer dans `textures:`
  (`planete/materiau.rs:378`) ; une résolution qui plafonne le détail, donc un
  bruit shader par-dessus. On ne supprime pas le bruit GPU, on l'**ancre**.

## 3. Piste 2 — la même formule évaluée des deux côtés

C'est exactement `terrain::hash` (`terrain.rs:168-175`) et la dette consignée en
`../suivi/planetes.md:111-140` : `DECORRELATION_HASH = 0,318_309_9` vit dans
**quatre fichiers** (`terrain.rs`, `planete.frag.glsl`, `panache.frag.glsl`,
`soleil.frag.glsl`), **rien ne les tient d'accord**, et la remplacer par
`FRAC_1_PI` la décalerait d'1 ULP — que le `fract` final amplifie en **bruit
entièrement différent**. Le garde-fou est un commentaire ; aucun test n'est
possible, comparer les deux implémentations exigeant un contexte GPU.

Honnêtement, elle offrirait quand même : zéro mémoire, zéro génération,
résolution infinie, hot-reload transparent, et un champ interrogeable **en tout
point** côté CPU. C'est séduisant. **Verdict** : raisonnable **si et
seulement si** la formule partagée est courte
et **géométrique** (quelques dipôles : `dot`, `acos`, une somme à borne fixe) —
un ULP y déplace un lobe d'un millionième de radian. Elle répète l'erreur connue
dès qu'elle contient un `hash` ou un `fbm`. Or une formule géométrique courte
tirée par graine, **c'est la piste 3** : la piste 2 n'a pas d'existence propre —
soit elle dégénère en 3, soit elle ajoute un cinquième fichier à la dette.

## 4. Piste 3 — un champ dipolaire paramétrique en uniforms

Le précédent le plus proche est `planete/vortex.rs`, et il enseigne trois choses.

1. **Son hash `h01` (l. 24-27) n'a pas de jumeau GLSL**, et ça n'a aucune
   importance : ce qui traverse la frontière, ce sont des **nombres**
   (`vortex[8]`, `vortex2[8]`), pas une formule. Seule construction CPU/GPU du
   dépôt **sans dette**.
2. **Le placement est un concours contre le champ** (l. 84-96) : six latitudes
   candidates, notées par `b_at`, la meilleure gagne — littéralement le
   mécanisme d'ancrage qui manque aux taches stellaires.
3. **L'encodage fait tenir deux faits dans un float** : `type + rayon`, décodé
   par `floor`/`fract` (`planete.frag.glsl:294-299`). Le soleil connaît déjà le
   procédé — le **signe** de `spots[i].w` distingue tache et flare
   (`eruptions.rs:98-111`).

**Transposition.** Un module `soleil/champ.rs` tire N pôles magnétiques depuis
`graine` : direction, **polarité signée**, force, rayon. Uniform `poles[N]` en
`vec4` — `xyz` = direction, `w` = force **signée**, la polarité tient donc sans
second tableau. Le shader évalue `B(d) = Σ force_i · noyau(angle(d, dir_i))` dans
une boucle **à borne fixe**, comme `spots` (`soleil.frag.glsl:130`) et
`champ_cyclones` (`planete.frag.glsl:189`) — GLSL ES 100 compatible. En
découlent : la surface s'assombrit là où `|B|` est fort (la tache **est** le
champ, elle n'est plus posée dessus) ; la granulation s'**étire le long des
lignes** près d'un pôle — le signe visuel de l'émergence, absent aujourd'hui ;
les pieds d'une arche sont deux pôles de **signe opposé**, choisis côté CPU
comme `vortex.rs` choisit sa latitude.

**Coûts.** N = 6 → 24 floats, sur un shader qui en déclare une douzaine (§ 1) ;
**aucun pipeline de plus** (mêmes deux passes) ; zéro texture, zéro sampler, zéro
génération, zéro asynchrone ; pas de piège D-GPU-1 ; hot-reload gratuit, les
uniforms étant repoussés à chaque frame (`rendu.rs:53-61`).

**Ce que ça ferme.** Pas de structure **fine** : un dipôle donne des lobes
lisses, les filaments restent peints au bruit — mais un bruit **modulé** par le
champ. N est plafonné par la lisibilité avant de l'être par les uniforms :
`MAX_BOUCLES = 4` (`eruptions.rs:11-14`) a tranché qu'au-delà, c'est un croissant.

## 5. Comparaison

| | **P1 · texture** | **P2 · formule partagée** | **P3 · uniforms** |
|---|---|---|---|
| Fichiers ajoutés | 1 Rust + sampler GLSL | 1 Rust + 1 bloc GLSL jumeau | 1 Rust + 1 bloc GLSL |
| Mémoire / étoile | 28 ko (32²) à 102 ko (64²) | 0 | 0 |
| Génération / étoile | ~5 à 25 ms ; ×21 en galerie | 0 | ~0 (quelques hash) |
| Uniforms (pipelines inchangés) | +2 (`atlas_n`, sampler) | +N | +N `vec4` (N ≈ 6) |
| Piège D-GPU-1 | **oui**, non testable | non | non |
| Hot-reload (R) | survit (pas de reconstruction) | transparent | transparent |
| CPU peut interroger | **oui** (le `Profil` reste) | **oui** (partout) | **oui** (les pôles) |
| Dette CPU/GPU | géométrique (tolérable) | **hash — l'erreur connue** | **aucune** |
| Résolution du champ | plafonnée par les texels | infinie | lisse par construction |

## 6. Recommandation

**La piste 3.** Un champ de quelques pôles tirés par graine, poussé en uniforms,
dont découlent l'assombrissement, l'étirement de la granulation, le placement des
taches et les pieds des arches — la seule des trois qui donne l'ancrage **sans
rien coûter** et **sans dette CPU/GPU**, et la seule qui ne se paie rien si on
l'abandonne. Pourquoi pas la 1 d'abord : un sampler, un atlas, un piège de
plomberie non testable et une génération par étoile, pour un gain — l'ancrage —
que la 3 donne déjà ; elle ne devient nécessaire que si le champ doit être
*structuré* (réseau supergranulaire, cellules à grande échelle), ce que personne
n'a demandé et qui ne se lira pas sous ~20 px de rayon (`astres_v2.md` § 0).
Pourquoi pas la 2 : § 3.

### Coût en lots

- **A — le champ seul, testé** (Rust, aucun rendu). `soleil/champ.rs` : `Champ`
  dérivé de `graine`, `b_at(dir) -> f32` signé, `lobes_opposes(tirage)`. Tests
  sans GPU : déterminisme à graine fixée, somme des polarités ≈ 0 (pas de
  monopôle), deux graines donnent deux champs, `b_at` a des maxima **séparés**.
  ~150 lignes + tests. *Brique d'abord, assemblage ensuite.*
- **B — les taches naissent du champ.** `eruptions.rs:189` : `dir_aleatoire()`
  → un maximum local de `|b_at|` encore libre, la taille suivant la force du
  lobe. **Rien ne change à l'écran** tant que le shader ne lit pas le champ :
  c'est voulu, ça isole la régression.
- **C — les pieds sont deux polarités opposées.** `declencher_boucle`
  (`eruptions.rs:133-151`) : la tangente tirée au sort devient l'axe joignant
  les deux lobes ; l'arche enjambe le changement de signe (testable).
  **Premier effet visible.**
- **D — le shader lit le champ.** `poles[N]` déclaré (`materiau.rs:57-69` +
  `soleil.frag.glsl`), boucle à borne fixe ; `spots[8]` **reste** — il porte
  l'âge et l'intensité, que le champ ignore — mais son assombrissement se module
  par `|B|`, et la granulation s'étire le long des lignes. Le test uniforms↔GLSL
  (`materiau.rs:134-139`) rougira seul si la déclaration manque.
- **E (optionnel) — la couronne.** La branche magnétar (l. 179-187) calcule déjà
  des arcs en `r = L sin²θ` sur un axe câblé : lui donner les vrais pôles fait
  converger les deux moitiés du fichier.

A + B + C tiennent dans une session ; D dans une autre (c'est du shader, ça
s'itère à l'œil) ; E est un bonus.

## 7. Ce qui reste incertain

- **Le nombre de pôles.** Trop peu : deux taches immobiles. Trop : une bouillie.
  `MAX_BOUCLES` a appris que cette borne est esthétique, pas technique.
- **Le noyau.** `1/(1+k·θ²)`, gaussien, `cos^n` ? Il décide si les lobes ont un
  bord franc — le défaut des « taches rondes » déjà corrigé
  (`soleil.frag.glsl:136-141`) — ou diffus au point que rien ne se voie.
- **Le champ doit-il vivre ?** Figé, les taches reviennent toujours aux six mêmes
  endroits pendant toute la session. Une dérive lente (rotation différentielle :
  pôles équatoriaux plus rapides) coûte un float et règle ça — hors lot A.
- ⚠️ **Dériver de `graine`, jamais tirer.** `Soleil::new` (`mod.rs:116`) puise
  dans le flux RNG global semé par `genese` (`genese/mod.rs:59`) : ajouter des
  `gen_range` **décalerait tout ce qui est tiré après**, et le système entier
  changerait à graine égale. `temps_initial` (`mod.rs:104-108`) a tranché —
  « cette étoile est unique » est *un* fait, il n'a droit qu'à une source.
- **La piste 1 reste ouverte en succession, pas en rejet** : le lot A la prépare,
  un `Champ` interrogeable étant exactement ce qu'on bakerait.

## 8. Ce qui ne se vérifiera qu'à l'œil

`methode.md` : le rendu ne se teste pas. Ne se jugeront qu'à l'écran — que la
tache **émerge** au lieu d'être posée (affaire d'étirement de la granulation à
son bord, qu'aucune assertion ne mesure) ; que l'arche parte **visiblement** de
la tache (le CPU peut prouver que les pieds sont sur les bons lobes, pas que ça
se lise à 20 px) ; le nombre de pôles et la forme du noyau (§ 7) ; et l'échelle
— `astres_v2.md` § 0, rien de tout ceci n'existe sous ~10 px de rayon. Au
cadrage par défaut (4 px, `PX_MIN`) le chantier est **invisible**, et c'est
normal : il vise la galerie, la vue objet et l'approche.

Se testent, et **doivent** l'être : le champ (déterminisme, absence de monopôle,
séparation des lobes), le choix de la tache, celui des pieds, la correspondance
uniforms↔GLSL — tout le reste est du dessin.
