# Conception — Repère flottant : le tremblement à fort zoom

> Étude d'impact, **2026-09-23**. Écrite **avant tout code**, et elle n'en
> produit aucun. Elle répond à une question posée telle quelle : le
> tremblement d'une lune de Saturne à ×25 000 « risque d'impacter les stations
> spatiales ou autres objets en orbite fixée, à plus long terme » — **combien
> coûte le chantier avant de le lancer ?**
>
> Le défaut est déjà diagnostiqué et mesuré ; §1 rappelle la loi qui le
> gouverne, recalée. §2 inventorie qui lit une position absolue, §3 chiffre le
> seuil de visibilité **par famille d'objets** — c'est là qu'est la réponse —,
> §4 compare quatre remèdes, §5 découpe celui qui est recommandé, §6 dit ce
> qui ne se vérifiera qu'à l'œil, §7 tranche, §8 isole ce que l'utilisateur
> doit décider lui-même.
>
> **§9, ajouté le 2026-09-24, dit ce qui a été fait** — l'option (a) livrée en
> un seul commit —, ce qui reste faux, et pourquoi deux des six lots du §5 se
> sont révélés inutiles. Les §1 à §8 restent tels qu'écrits avant le code.

---

## 1. La loi, recalée

Le mécanisme est acquis : `Camera::construire` pose
`pos = target - forward * dist` (`src/camera.rs:182`) avec `target` de l'ordre
de la distance héliocentrique et `dist` pouvant descendre à `DIST_MIN = 0,01`
(`src/camera.rs:14`). Soustraire un petit décalage d'une grande valeur en
`f32` perd des bits, et la perte varie d'une orientation à l'autre : c'est un
tremblement, pas une dérive.

Mesuré ici en rejouant l'arithmétique exacte de `construire` sur 3 589
couples (phase orbitale × orientation), comparée à la même chose en `f64` —
l'erreur de crête sur le vecteur cible → caméra vaut, très proprement :

> **err ≈ ½ · ULP(R) = R · 2⁻²⁴**, où *R* est la distance héliocentrique en
> unités monde (moyenne : la moitié de ça).

À Saturne (R = 460 u) : ULP = 3,05 × 10⁻⁵, crête mesurée **1,50 × 10⁻⁵**,
moyenne 7,4 × 10⁻⁶. L'enquête relevait 1,2 × 10⁻⁵ — entre les deux, cohérent.

Conversion en pixels, avec le même demi-fov de 22,5° que partout ailleurs
(`systeme/rendu.rs:323`, `impostor.rs:88`) :

> **gigue (px) = err × Z × h × 9,43 × 10⁻⁴** — soit `err × Z × 0,66` dans la
> fenêtre du jeu (h = 700, `main.rs:43`), `× 1,32` sous `CAPTURE` (h = 1 400).

⚠️ **Les chiffres de l'enquête valent pour la fenêtre de `CAPTURE`.** 0,73 px
à ×50 000 correspond à h ≈ 1 400 ; à 700, le même défaut vaut 0,37 px. Le
tremblement observé en jeu est donc **deux fois plus petit** que ce que le
relevé laisse croire — et deux fois plus gros si on joue un jour en 1440p.
C'est le premier point à trancher (§8).

## 2. Qui lit une position absolue

Trois familles de consommateurs, et elles ne se cassent pas de la même façon.

**A. Le producteur unique.** `CorpsBase.position` (`src/astre.rs:33`) est la
seule source ; `Systeme::position` (`src/systeme/mod.rs:474`) et
`CameraInfo.pos` (`src/astre.rs:65`) en sont les deux robinets. Tout le reste
en découle. La propagation écrit dedans : `maj_rail` (`planete/mod.rs:246`),
`orbiter_autour` (`planete/mod.rs:241`, `engin.rs:232`), `suivre_trajectoire`
(`transit.rs:95`), `stellaire.rs:128`.

**B. Ce qui compose une position monde pour le GPU** — casse net si on ne
touche pas au site, silencieusement si on en oublie un :

| Site | Ce qu'il reçoit | Unités |
|---|---|---|
| `camera.rs:94` `projeter_ecran` | `p - cam.pos` | monde, les deux |
| `camera.rs:182` `construire` | `target`, puis `Camera3D.position/target` | monde |
| `systeme/mod.rs:482` `pick` | `origine` (= `cam.pos`), `centre` de chaque astre | monde |
| `systeme/rendu.rs:126/143` `dessiner_points` | `centre` + polyligne relative + `cam` | monde |
| `systeme/rendu.rs:170` `dessiner_points_ouvert` | polyligne **absolue** (`Trajectoire::position`) | monde |
| `systeme/rendu.rs:426/431` pointillés, cercles | points absolus → `draw_line_3d` | monde |
| `systeme/rendu.rs:28/194/225` | `light_pos`, `lights_pos[4]` | monde |
| `planete/mod.rs:310` `draw` | `c = base.position` → `push_quad` | monde |
| `planete/materiau.rs:202/207/213` | uniformes `centre`, `lumiere`, `lights_pos[4]` | monde, f32 |
| `planete/materiau.rs:154` `px_rayon` | `(cam.pos - c).length()` | monde |
| `soleil/rendu.rs:13/88/111` | `c`, boucles et éjections en monde, `cam.pos` | monde |
| `soleil/mod.rs:175` `rayon_dessine` | `cam_pos - base.position` | monde |
| `disque/mod.rs:229/237/238` | uniformes `centre`, `light_pos`, `cam_pos` | monde, f32 |
| `disque/voile.rs:91` | uniforme `centre` | monde, f32 |
| `disque/mod.rs:213` `position_lacune` | `base.position + …` | monde |
| `engin.rs:243` / `transit.rs:102` | translation de la `Mat4` de modèle | monde |
| `fond.rs:53` | `cam.pos + dir * RAYON` | monde |
| `ecran/marqueur.rs:74` | `projeter_ecran(centre, …)` | monde |
| `ecran/trace_orbite.rs:225/282` | `cam.pos.distance(centre)`, `centre + a` | monde |
| `ecran/vignette.rs` (`dessiner`) | `pos = centre + direction * recul` | monde |
| `ecran/icone.rs` (`rendre`) | idem | monde |

Les cinq shaders concernés reçoivent des **points monde en `f32`** :
`planete.frag.glsl:8/10/17` (`centre`, `lumiere`, `lights_pos`),
`atmosphere.frag.glsl:34/36/38` (idem), `disque_particules.vert.glsl:85`
(`pos = centre + …`, l'orbite est reconstruite **dans le shader**),
`disque_voile.vert.glsl:18` (`centre`), `station.frag.glsl:9` (`cam_pos`,
comparé à `v_wpos` issu de la matrice `Model`, `station.vert.glsl:13`).
`soleil.frag.glsl`, `soleil_plasma`, `fond`, `impostor.vert`, `panache` et
`palette` ne manipulent que des **directions** : exempts.

**C. Ce qui lit une position sans la rendre** — jamais cassé par un repère
relatif : `ecran/selecteur.rs:94` (tri par distance), `ecran/fiche.rs:152`
(distance en UA affichée), `systeme/mod.rs:104` (rayon du système),
`genese/mod.rs:455`. Et deux espaces entièrement à part, hors sujet :
`starmap/` (parsecs, autre repère) et l'assembleur `vaisseau/` +
`ecran/designation.rs`, `galerie*.rs`, `briques.rs` (repères locaux, corps à
l'origine, `eclairage::avec` n'est appelé que là).

⚠️ **Le maillage d'une station n'est pas un problème** — contre-intuitif, et
c'est la bonne nouvelle de cet inventaire. `MaillageStation` est cuit en
sommets **relatifs à son origine** et posé par **une seule**
`push_model_matrix` (`vaisseau/maillage.rs:341`, `engin.rs:240`) : les
milliers de sommets d'une mégastructure ne voient jamais une coordonnée
héliocentrique, seule la translation de la `Mat4` en porte une. **Un nombre
à changer par site**, pas un maillage à retoucher.

## 3. Le seuil de visibilité, par famille

Deux grandeurs se combinent, et c'est la clé : la gigue est fixée par *R* et
par le zoom, mais le zoom **auquel on regarde vraiment un objet** est fixé par
son rayon dessiné. Le rayon de son orbite propre — `ORBITE_ISS`, l'orbite
d'une lune — n'entre **pas** dans le calcul.

Zoom auquel la gigue atteint ½ px (≈ 1 px crête à crête), fenêtre de jeu :

| Hôte | R (u) | ULP(R) | Z(½ px) | Plafond `DIST_MIN` |
|---|---|---|---|---|
| Terre, 1 UA | 48 | 3,8 × 10⁻⁶ | ×398 000 | inatteignable |
| Mars, 1,52 UA | 73 | 7,6 × 10⁻⁶ | ×206 000 | inatteignable |
| Jupiter, 5,2 UA | 250 | 1,5 × 10⁻⁵ | ×101 000 | atteint de justesse |
| Saturne, 9,58 UA | 460 | 3,1 × 10⁻⁵ | **×51 000** | largement atteint |
| Neptune, 30 UA | 1 443 | 1,2 × 10⁻⁴ | **×12 800** | largement atteint |
| Kuiper, 40 UA | 1 920 | 1,2 × 10⁻⁴ | ×14 000 | largement atteint |
| Oort, 2 000 UA | 96 000 | 7,8 × 10⁻³ | ×200 | toujours |

Et l'exposition réelle, **à cadrage égal** : gigue au zoom qui donne à l'objet
200 px de diamètre (fenêtre `CAPTURE`, à diviser par deux en jeu) :

| Objet | rayon dessiné (u) | zoom | gigue |
|---|---|---|---|
| Saturne, son anneau | 9,3 × 10⁻² / 2,1 × 10⁻¹ | ×816 / ×371 | 0,02 / 0,01 px |
| **ISS, Starship postés @Terre** | 3,07 × 10⁻³ | ×24 850 | **0,06 px** |
| Station postée @Saturne | 2,8 × 10⁻² | ×2 719 | 0,05 px |
| Io | 2,9 × 10⁻³ | ×26 080 | 0,25 px |
| **Titan** (le cas signalé) | 4,1 × 10⁻³ | ×18 440 | **0,37 px** |
| Caillou de Kuiper | 1,2 × 10⁻² | ×6 350 | 0,45 px |
| Station postée @Neptune | 1,2 × 10⁻² | ×6 430 | 0,51 px |
| Phobos | 1,8 × 10⁻⁵ | plafond (3 px de rayon !) | 0,62 px |
| Encelade | 4,0 × 10⁻⁴ | plafond (68 px de rayon) | 2,53 px |
| **Triton** | 2,2 × 10⁻³ | ×35 100 | **2,74 px** |
| Caillou de Oort | 1,2 × 10⁻² | ×6 350 | 31 px |

**Réponse à la question posée : une station en orbite basse terrestre est six
fois MOINS exposée qu'une lune de Saturne** (0,06 px contre 0,37). Deux
raisons cumulées, aucune n'étant celle qu'on attend :

1. *R* vaut 48 au lieu de 460 — un facteur 8 sur l'ULP, gratuit ;
2. l'engin est dessiné **exagéré** (`ECHELLE_ISS = 0,30` du rayon planétaire,
   `engin.rs:36`, mensonge d'échelle assumé) : à 3,07 × 10⁻³ unité il est
   **sept fois plus gros qu'Encelade** et presque aussi gros que Titan. Il
   demande donc *moins* de zoom, pas plus.

Le rayon d'orbite minuscule d'`ORBITE_ISS` ne joue aucun rôle. Et comme
l'envergure d'un engin posté est dérivée du rayon de son hôte, une station
autour de Saturne reste à 0,05 px : **les engins postés sont, aujourd'hui, la
famille la moins menacée.** Ce qui menace vraiment, par ordre : Triton, puis
Encelade, puis tout ce qui sera posé au-delà de 20 UA — or la décision 4 de
[`trajectoires.md`](trajectoires.md) prévoit explicitement des habitats en
orbite de géante gazeuse et aux points de Lagrange. **Une station à Neptune
est déjà à 0,51 px.** C'est ça, le « plus long terme » de la question.

Note pour la branche en cours : `ecran/trace_orbite.rs` trace les orbites de
lunes en 2D via `projeter_ecran`, le chemin le plus exposé du tableau B.

## 4. Les quatre options

**(a) Repère relatif à la caméra, au rendu seul.** `CameraInfo` gagne une
origine (la cible), `construire` la pose, `pos` devient `-forward*dist`, et
tout point monde passe par une unique fonction `rel(p) = p - origine` avant
d'atteindre le GPU. *Pourquoi ça marche :* `p` et `origine` sont dans un
facteur 2 l'un de l'autre, donc leur soustraction `f32` est **exacte**
(Sterbenz) ; l'objet focalisé se rend à l'origine, immobile par construction.
*Fichiers :* ~20 Rust, 5 shaders, 14 sites de construction de `CameraInfo`
dans 11 fichiers. *Sur les shaders :* **rien à réécrire** — les uniformes
restent des `f32`, on leur passe simplement de petits nombres. C'est le
point. *Ce qui reste faux :* les objets **non focalisés** gardent la
quantification de leur position stockée (½ ULP de *R*), qui produit non plus
un tremblement mais un **escalier** régulier à mesure que la lune avance —
3,05 × 10⁻⁵ unité de marche à Saturne, soit 0,5 px à ×25 000. Bien moins
visible qu'une gigue erratique, mais réel, et **non chiffré à l'œil**.
*Tests :* les ~24 qui fabriquent une `CameraInfo` à la main (`marqueur.rs`,
`designation.rs`, `trace_orbite.rs`) ajoutent un champ ; aucun ne disparaît.

**(b) Positions en `f64`.** `CorpsBase.position: DVec3`, propagation en
`f64` de bout en bout. *Fichiers :* ~25–30 (`astre.rs`, `orbite.rs`,
`planete/`, `engin.rs`, `transit.rs`, `systeme/`, `stellaire.rs`,
`disque/`, `genese/`), plus chaque frontière `Vec3` ↔ `DVec3`. Les tests qui
comparent des positions à l'identique (`transit.rs:141/154`, `engin.rs:374`,
les 12 de `trajectoire.rs`) changent tous de type. *Effet sur les shaders :*
**aucun — et c'est exactement le problème.** Le GPU continue de recevoir des
`f32` absolus via `Projection * Model` : convertir une `DVec3` de 460 en
`f32` la requantifie à 3 × 10⁻⁵ juste avant l'appel de dessin. ⚠️ **`f64`
seul ne change rien à l'écran.** Il corrige la simulation (l'escalier du
point (a)), pas le rendu. Deux fois le coût de (a) pour aucun gain visible.

**(c) Hybride : stockage `f64`, rendu relatif.** La réponse de manuel, bonne à
terme. Coût = (a) + (b), ~40 fichiers ; justifiée seulement si l'escalier
résiduel de (a) se voit — ce qui n'est pas su.

**(d) Ne rien faire, borner le zoom.** Porter `DIST_MIN` de 0,01 à 0,04
ramène le plafond de ×128 000 à ×32 000 : Saturne repasse sous le seuil, mais
Neptune (×12 800) et Kuiper non, et **Triton devient inobservable de près**.
Une variante honnête ferait dépendre le plancher de *R*. *Fichiers :* 1
(`camera.rs`), plus le site qui connaît la cible ; 2 tests à relire
(`le_zoom_monte_jusqua_2560`, `le_plancher_de_distance_reste_positif`, qui
pinnent la constante **et disent pourquoi elle compte**). Coût quasi nul,
mais c'est une perte de capacité là où le jeu veut poser des habitats.

## 5. Découpage de l'option (a)

Six lots, chacun livrant quelque chose de visible ou de vérifiable.

| Lot | Contenu | Ce qu'il livre |
|---|---|---|
| **1** | Un test pur qui rejoue l'arithmétique de `construire` en `f32` et affirme `err ≈ ½·ULP(R)`, plus la table des seuils du §3. Aucun changement de rendu. | Le chiffre qui dira si les lots suivants ont marché. Vert au `cargo test`. |
| **2** | `CameraInfo` gagne `origine`, `construire` la pose, `rel(p)` naît dans `camera.rs`. Les 14 sites de construction posent `Vec3::ZERO`. | **Comportement bit à bit inchangé.** La suite passe sans rien d'autre que ces 14 initialisations. |
| **3** | Le chemin 2D : `projeter_ecran` passe par `rel()`. Couvre `marqueur.rs`, `trace_orbite.rs`, `designation.rs`. | Le triangle de marqueur et l'orbite de lune cessent de trembler **avant** le corps — l'écart entre les deux est la preuve. Entièrement testable (§6). |
| **4** | Les corps impostor : `planete/mod.rs`, `planete/materiau.rs`, `soleil/rendu.rs`, `impostor.rs` + `planete.frag.glsl`, `atmosphere.frag.glsl`. | **La lune cesse de trembler.** C'est le lot qui répond à la demande. |
| **5** | Engins et maillages : la translation des `Mat4` d'`engin.rs:243` et `transit.rs:102`, `cam_pos` de `station.frag.glsl`. | Le moins cher des six malgré les apparences (§2, note C). |
| **6** | Champs de débris, anneaux, fond, vignettes : `disque/mod.rs`, `disque/voile.rs`, `fond.rs:53` (qui **se simplifie** : `dir*RAYON` suffit), `vignette.rs`, `icone.rs`. | Les particules : `centre` relatif suffit, **aucune ligne de GLSL réécrite**. Les vignettes portaient le même défaut, jamais diagnostiqué. |

Ordre non négociable : 1 avant tout (mesurer avant de soigner), puis 2 (qui ne change rien et rend les suivants mécaniques).

## 6. Ce qui ne sera pas testable

`docs/methode.md` pose la règle : **pas de test de rendu**. Rien dans la suite
ne compile un shader ni n'ouvre de fenêtre — `Camera::new` exige le contexte
GL (`camera.rs:153`).

**Ce qui l'est quand même**, et c'est plus que prévu : l'arithmétique pure se
teste comme `pas_points` ou `liee_au_focus` le sont déjà. `rayon_ecran` et
`projeter_ecran` ont **été extraits exprès pour être testables sans contexte
graphique** (`camera.rs:78/93`) — donc le lot 3 est couvert de bout en bout.
Le bon test n'est pas « absolu et relatif donnent le même pixel » (ce serait
comparer une copie à une copie, `methode.md`) : c'est **comparer les deux à
une référence `f64`** et exiger que le relatif s'en approche strictement mieux
dès que *R* dépasse quelques dizaines d'unités. Un red-check naturel : poser
l'origine à `Vec3::ZERO` doit faire rougir le test.

**Ce qui ne l'est pas**, et il faut le dire net :

- Qu'un uniforme ait bien été rendu relatif. Aucun harnais GPU : un oubli ne
  produit ni erreur de compilation ni test rouge, seulement une planète
  éclairée du mauvais côté.
- **Que la lune ait cessé de trembler.** `CAPTURE=<f.png>` (`main.rs:98`)
  produit **une** image après quelques frames de chauffe ; un tremblement est
  par définition une différence entre deux frames, qu'une image ne montre pas.
  Le harnais ouvre de plus la Skymap au cadrage par défaut, pas un zoom donné.
  `PROFIL=` ne mesure que des fps.

Verdict : **le tremblement ne se jugera qu'à l'œil**, à ×25 000 sur Titan,
par l'utilisateur, avant et après. Pour le mécaniser il faudrait un
`CAPTURE_N=<n>` : n images consécutives à horloge de simulation figée, sur un
cadrage nommé, puis différence deux à deux. Chantier d'outillage distinct,
utile à tout défaut inter-frame, jamais un préalable aux six lots.

## 7. Recommandation

**Option (a), le repère relatif au rendu, en six lots.** Pas (b) : `f64` seul
ne change rien à l'écran, puisque le GPU reçoit de toute façon des `f32`
absolus — c'est le résultat le plus utile de cette étude, et il retire une
option qui paraissait évidente. Pas (c) tant que (a) n'a pas montré sa limite.
(d) reste le repli si le chantier est repoussé : une constante dans
`camera.rs`, à assumer comme une perte de capacité.

**Coût :** ~20 fichiers Rust, 5 shaders dont **aucun à réécrire**, 14 sites de
construction de `CameraInfo`, ~24 tests à réajuster sur 752, zéro test
supprimé. Les lots 1 et 2 ne touchent pas au rendu : livrables seuls, sans
risque.

**Risque principal : la régression silencieuse d'éclairage.** `lumiere`,
`lights_pos[4]`, `light_pos` et `cam_pos` sont des positions monde dans cinq
shaders ; en oublier une ne casse rien de visible au `cargo test` — seulement
un terminateur de planète décalé, ou un spéculaire de station venu de nulle
part, six mois plus tard. Garde-fou (`methode.md`, « une source par fait ») :
faire passer **toutes** les positions monde par la seule fonction `rel()`.

## 8. Ce qui reste à trancher

1. **Pour quelle fenêtre corrige-t-on ?** Les chiffres de l'enquête valent
   pour h = 1 400 (`CAPTURE`) ; le jeu tourne à 700 (`main.rs:43`), où le
   défaut est deux fois plus petit — et en 1440p plein écran, deux fois plus
   gros qu'à `CAPTURE`. Le seuil de ½ px du §3 se déplace d'un facteur 4 entre
   ces deux bouts.
2. **Jusqu'où doit-on pouvoir zoomer, et sur quoi ?** Encelade ne dépasse
   jamais 68 px de rayon même au plafond, Phobos plafonne à 3 px. Si ces corps
   ne sont pas censés être approchés, (d) suffit et le chantier n'a pas lieu
   d'être. La question est de gameplay, pas de virgule flottante.
3. **Où vivront les stations ?** Dans le système interne, elles sont six fois
   moins exposées qu'une lune de Saturne et le chantier peut attendre. À
   Neptune ou au-delà — ce que prévoit la décision 4 de `trajectoires.md` —
   il devient requis avant qu'on en pose une.
4. **Étend-on le harnais de capture** (`CAPTURE_N`, §6) pour pouvoir juger
   autrement qu'à l'œil ? Chantier distinct, utile au-delà de ce défaut,
   jamais un préalable.

---

## 9. Ce qui a été fait — 2026-09-24

> Écrit **après** le chantier, qui a tenu en **un seul commit**. L'étude
> ci-dessus reste telle qu'elle a été rédigée : elle a servi à décider, et ses
> chiffres servent encore de référence. Cette section dit ce qui a changé, ce
> qui reste faux, et où le découpage du §5 s'est trompé.

### 9.1 La règle posée

`CameraInfo` a gagné un champ `origine: Vec3` et une méthode
`rel(p) = p - origine` (`src/astre.rs`), **source unique** de la conversion.
La règle tient en deux lignes :

- **`CameraInfo.pos` reste ABSOLU.** `Systeme::pick`, les distances, les LOD
  (`rayon_dessine`, `px_rayon`, le seuil sub-pixel des débris) et
  `projeter_ecran` continuent de le lire tel quel, et marchent à l'identique.
- **Tout ce qui part au GPU passe par `rel()`** : position et cible de la
  caméra 3D, sommets, centres d'impostor, matrices de modèle, uniformes de
  position.

`Camera::construire` pose `origine = target` et bâtit son `Camera3D` avec
`position: pos - origine`, `target: Vec3::ZERO`. Les autres sites posent
l'origine qui leur convient : `Vec3::ZERO` pour les galeries et pour
l'assembleur (qui appelle déjà `construire(Vec3::ZERO, …)` — comportement
inchangé), et la position de l'astre portraituré pour `ecran/vignette.rs` et
`ecran/icone.rs`, qui portaient le même défaut sans que personne l'ait
diagnostiqué.

**Aucune ligne de GLSL n'a été réécrite**, comme le §4 (a) l'annonçait : les
uniformes étaient déjà des `f32`, on leur passe simplement de petits nombres.

### 9.2 Fichiers convertis

`impostor.rs` (`push_quad` prend désormais `&CameraInfo` et convertit
lui-même — l'oubli devient impossible côté appelant), `planete/mod.rs`,
`planete/materiau.rs` (`centre`, `lumiere`, `lights_pos[4]`, corps ET
coquille atmosphérique), `soleil/rendu.rs` (quad du disque, boucles,
éjections), `disque/mod.rs` (`centre`, `light_pos`, `cam_pos`),
`disque/voile.rs` (`centre`), `engin.rs` et `transit.rs` (la seule
translation de la `Mat4` de modèle), `fond.rs`, `systeme/rendu.rs` (orbites
de planètes et de lunes, pointillés stellaires, cercles de zone habitable,
aperçu de trajet — tous les `draw_line_3d`), `rendu.rs` (`draw_orbites`
reçoit le repère entier), `ecran/vignette.rs`, `ecran/icone.rs`, et les trois
galeries.

`ecran/trace_orbite.rs` **n'a rien eu à changer** : il projette via
`projeter_ecran`, qui soustrait `cam.pos` avant toute division — le chemin 2D
était sain, et le rester est gardé par un test. `ecran/skymap.rs` (V1) est
identique à l'octet près : il profite de la correction par `construire`, sans
une ligne de diff.

`shaders/station.frag.glsl` n'a pas bougé non plus, et ce n'est pas un oubli :
son `cam_pos` n'est posé que par `vaisseau::eclairage::avec`, appelé seulement
par l'assembleur, les briques et l'écran de station — trois vues dont
l'origine de rendu est `Vec3::ZERO`.

### 9.3 Les tests

Deux garde-fous neufs dans `src/camera.rs`, module `tests_precision`, tous
deux branchés sur ce que `Camera::construire` produit **vraiment** (la
`Camera` se fabrique par littéral de struct : `Camera::new` exige un contexte
graphique, pas le littéral) :

- `la_camera_construite_est_relative_mais_garde_une_position_absolue` :
  `info.origine == target`, `cam3d.target == Vec3::ZERO`,
  `cam3d.position.length() ≈ dist`, `info.pos` toujours absolu,
  `info.rel(cible) == Vec3::ZERO`.
- `la_camera_construite_immobilise_lastre_suivi` : la mécanique de
  `saut_max_px` (une cible qui avance de quelques ULP, le pixel où elle
  atterrit), mais la caméra vient de `construire` et le sommet de
  `CameraInfo::rel`. **Zéro pixel exactement**, à Saturne, Neptune et Kuiper,
  aux trois distances les plus serrées.

⚠️ **L'égalité stricte à zéro n'est pas une coquetterie.** Le second sabotage
du red-check — caméra 3D remise en absolu, origine laissée juste — ne produit
que **0,00037 px** de saut, parce que `rel(cible)` vaut exactement zéro et que
seule la pose de l'œil perd des bits. Une tolérance d'un demi-pixel aurait
laissé ce sabotage vert.

Les deux tests d'origine restent verts, et c'était voulu :
`le_rendu_3d_absolu_fait_sauter_lastre_suivi` rejoue **sa propre** matrice
(c'est un fait mathématique sur `f32`, pas une observation de la production) ;
`la_projection_ecran_ne_fait_pas_sauter_lastre_suivi` garde le chemin 2D.
Les fabriques de `CameraInfo` à la main (`marqueur.rs`, `designation.rs`,
`trace_orbite.rs`, `tests_precision`) ont gagné un champ ; aucun test n'a
disparu ni été affaibli. Totaux : **756 tests verts**, `cargo clippy
--all-targets` à zéro avertissement.

### 9.4 Ce qui reste faux

**L'escalier des objets non focalisés**, annoncé au §4 (a) et toujours là. Une
lune qu'on ne suit pas garde la quantification de sa position **stockée**
(½ ULP de *R*, soit 3,05 × 10⁻⁵ unité à Saturne) : ce n'est plus une gigue
erratique, c'est une marche régulière à mesure qu'elle avance, ~0,5 px à
×25 000. Bien moins visible — l'œil pardonne un pas régulier là où il accroche
un tremblement — mais réel, et **toujours non chiffré à l'œil**. Le remède
serait l'option (c), stockage `f64` + rendu relatif ; elle ne se justifie que
si cet escalier se voit, ce qui n'est pas su.

**Rien de ceci ne se vérifie autrement qu'à l'œil** (§6) : aucun harnais GPU,
et `CAPTURE=` ne produit qu'une image quand un tremblement est par définition
une différence entre deux frames. Les cas à regarder, par ordre de risque :
Titan et Triton à fort zoom (le défaut d'origine), le fond stellaire (sphère
recentrée sur l'œil), les anneaux vus par la tranche et les ceintures
(uniformes `centre` / `cam_pos` / `light_pos`), une vue **sans astre
focalisé** (origine = `Vec3::ZERO`, donc chemin nominalement inchangé), les
objets posés à l'origine même, les vignettes et les icônes du sélecteur, et
les écrans de galerie et d'assemblage — qui, eux, ne devaient rien changer du
tout.

### 9.5 Les lots 2 et 3 n'ont pas eu lieu

Le découpage du §5 prévoyait six lots. Le lot 1 (le témoin chiffré) a été
livré seul, comme prévu. Les autres se sont réduits à **un seul commit**, et
deux des six se sont révélés **inutiles** :

- **Lot 2** — « `CameraInfo` gagne `origine`, les sites posent `Vec3::ZERO`,
  comportement bit à bit inchangé » : livrable en théorie, sans intérêt en
  pratique. Il ajoute un champ mort et une méthode que personne n'appelle ;
  le seul moyen de vérifier qu'il n'a rien cassé est de faire passer la suite,
  ce que fait de toute façon le commit qui suit. Une étape qui ne peut rien
  prouver n'est pas un jalon.
- **Lot 3** — « `projeter_ecran` passe par `rel()` » : **à ne surtout pas
  faire.** Le §2 le disait déjà (« `camera.rs:94` : `p - cam.pos`, monde les
  deux ») sans en tirer la conclusion : ce chemin soustrait *déjà* la position
  de la caméra, il est sain, et l'origine s'y annulerait des deux côtés. Y
  toucher n'aurait rien corrigé et aurait risqué de casser marqueurs,
  désignation et orbites de lunes — les seules choses qui marchaient déjà.

Et surtout, le découpage manquait la contrainte qui décide de tout :
**le chantier est atomique**. Dès que la caméra passe en relatif, tout dessin
resté en absolu part à 460 unités hors champ. Livrer les lots 4, 5 et 6
séparément aurait donné deux états intermédiaires **injouables**. Les lots 1
et 2 étaient les seuls détachables ; à partir du lot 4, c'est tout ou rien.
