# Suivi — Pièges techniques du moteur (macroquad / miniquad / GPU)

> Les défauts qui ne tiennent à aucun chantier en particulier mais au **moteur**,
> et qui se paient une fois par projet si on ne les note pas.
> Point de reprise : [`../../STATE.md`](../../STATE.md).
>
> Extrait de `STATE.md` le 2026-08-10.

---

## Le rappel court

| Piège | Règle qui en sort |
|---|---|
| 32 pipelines au maximum (`MAX_PIPELINES`) | tout `load_material` passe par un **cache** `thread_local` |
| un clone de `Material` partage le pipeline | `set_uniform` écrit dans le **pipeline**, pas dans le clone → vider le lot entre deux réglages |
| macroquad ne double-face aucune géométrie | une face vue des deux côtés s'émet **deux fois**, enroulement inversé |
| l'uniform `Projection` est la matrice **vue-projection** | voir [`ceintures.md`](ceintures.md) § P.2 octies |
| un uniform déclaré mais non lu vaut **zéro**, en silence | tests de correspondance uniforms ↔ GLSL |
| `fwidth`/`dFdx`/`dFdy` **ne sont pas** dans GLSL ES 100 core | ne jamais y adosser un anti-crénelage — voir ci-dessous |
| il n'y a **pas** de post-traitement global disponible | pas de bloom, pas de grab-pass : ce qui s'en approche se falsifie en géométrie |
| en caméra 3D, le material **par défaut** de macroquad **écrit la profondeur** | tout ce qui est translucide passe par un material `depth_write: false` — voir ci-dessous |
| un ruban oriente sa largeur sur la tangente **projetée à l'écran** | vu dans son axe, il se croise en rayons : pour une lueur, des **disques face caméra** — voir ci-dessous |

---

## ⚠️ Piège : le material par défaut écrit la profondeur (2026-10-19)

Trouvé sur les queues de comète. En caméra 3D, macroquad dessine `draw_mesh`
avec son pipeline `TRIANGLES_DEPTH_PIPELINE` : `depth_test: LessOrEqual`
**et `depth_write: true`** (`quad_gl.rs`, lu dans les sources 0.4.15). Un
maillage translucide dessiné ainsi inscrit dans le tampon de profondeur
**tout son tracé, parties invisibles comprises** — une vitre que rien ne
montre et qui rejette tout ce qui se dessine ensuite derrière elle.

Les queues de comète l'ont révélé : la queue de poussière, tracée la première,
masquait la queue d'ions partout où elles se recouvrent — près du noyau, si
bien que le trait bleu « n'apparaissait qu'après la traînée de débris »,
comme l'a décrit l'utilisateur (*« comme un genre de sticker »*).

**Règle** : rien de translucide sur le material par défaut. Tous les autres
objets translucides du dépôt avaient déjà le leur (`depth_write: false` dans
`fond`, `planete/materiau`, `soleil/materiau`, `disque/voile`, `panache`,
`accretion`) ; la queue était l'exception. Elle emprunte désormais celui du
**plasma solaire** (additif, teste la profondeur sans l'écrire), sans
pipeline de plus — `le_plasma_teste_la_profondeur_sans_l_ecrire` le garde,
puisque deux usagers en dépendent.

## ⚠️ Piège : un ruban vu dans son axe (2026-10-19)

`impostor::push_ruban` oriente la largeur sur la tangente de la courbe
**projetée dans le plan écran**. Quand la courbe pointe vers la caméra ou à
l'opposé, cette projection tend vers zéro : chaque point prend une
orientation de bruit d'arrondi, les quads voisins se croisent, et on voit
*« plein de rayons qui se croisent »*. Le garde-fou existant (ne jamais
retourner l'orientation d'un point au suivant) ne couvre que le changement de
signe, pas le bruit.

Pour une **arche** de plasma, vue surtout de côté, le ruban reste le bon
choix. Pour une **queue de comète**, qu'on voit sous tous les angles et
parfois dans l'axe, il ne l'est pas : même juste, une bande vue par la
tranche ne fait pas une lueur ronde. Elle est passée en **chapelet de disques
flous face caméra** (`comete::nuage`) : un disque n'a pas d'orientation, rien
ne peut se croiser. Le recouvrement est compensé dans l'éclat (le halo
`(1 − d)²` intègre à `2r/3` sur un diamètre).

---

## P.2 ter — « Pipelines amount exceeded » : un plafond, et une fuite ancienne

Signalé par l'écran au lancement de la **galerie de débris**, après P.1. La
coquille atmosphérique n'est pas la cause — elle est la goutte.

**Le plafond.** macroquad n'accepte que **32 pipelines** (`quad_gl.rs`,
`MAX_PIPELINES`), et `load_material` en consomme un à chaque appel. Au-delà :
`panic!("Pipelines amount exceeded")`, sans rien pour situer la fuite.

**La fuite.** `Disque::new` appelait `material_particules()` et
`voile::material_voile()` **directement** — un à deux pipelines GPU **par
disque**. La galerie de débris en construit douze d'un coup : jusqu'à 24
pipelines pour les seuls disques. `planete` et `soleil` avaient reçu le cache
qui évite ça ; `disque` ne l'avait jamais eu. Le +1 de la passe atmosphère a
fait franchir le seuil.

**Le remède** : template en `thread_local`, cloné ensuite (`mat_particules`,
`mat_voile`) — douze disques coûtent deux pipelines au lieu de vingt-quatre.

⚠️ **Et sa contrepartie, qui aurait produit un second bug silencieux.** Un clone
de `Material` est un `Arc` sur le **même** pipeline : `set_uniform` écrit dans le
pipeline, pas dans le clone. Sans vidage de lot, le **dernier** disque dessiné
imposerait ses réglages (centre, axes, `gm`, moitié) à tous les précédents —
anneaux plaqués au mauvais endroit. D'où `vider_lot()` dans `draw_moitie`, même
remède que pour les corps planétaires. Le commentaire de `planete/materiau.rs`
affirmait le contraire (« un clone a ses propres uniforms ») ; il est corrigé.

**Tests** : `les_materials_ne_sont_construits_qu_une_fois` tient l'invariant réel
— chaque constructeur brut n'est appelé qu'**une** fois dans la crate, depuis le
cache. Red-check : remettre le material par disque le fait rougir.

⚠️ **Une première version de ce test ne servait à rien.** Elle comptait les
shaders du projet et restait **verte** quand on remettait délibérément un
material par disque : elle ne mesurait pas ce qu'elle prétendait garder. Pire,
sa version suivante était **rouge dès le départ** (elle cherchait l'appel dans le
fichier de définition, pas dans celui de l'appel) — et un test toujours rouge ne
prouve rien de ses red-checks. Leçon, à ajouter aux précédentes : **vérifier que
la base est verte AVANT de saboter**, sinon le red-check ne teste que lui-même.

## ⚠️ Piège : matériau partagé + lots de dessin (D-GPU-1)

**Toutes les planètes partagent un seul matériau** : `mat_corps()` rend des clones
du même template, et les clones partagent le pipeline. `mat.set_texture(...)`
n'attache donc **pas** une texture à *ce* dessin — elle l'écrit dans le pipeline,
et macroquad ne l'applique qu'au **vidage du lot**.

Conséquence : sans vidage explicite, *le dernier corps dessiné impose ses
textures à tous les précédents*. Dans le système solaire, le dernier est une
tellurique, donc `zonal = tex_vide()` (canal vert à 0) → `band = 0` →
`base = belt` → **les quatre géantes sortaient d'un brun uni**.

La galerie y échappait **par accident** : elle change de viewport à chaque
cellule, et chaque `set_camera` vide le lot. C'est pourquoi le défaut ne se
voyait que dans la vue système — et pourquoi le chercher dans les données ne
pouvait pas aboutir (le profil zonal CPU était parfait, c'est testé).

Corrigé par un `get_internal_gl().flush()` après chaque `draw_mesh` de planète.

⚠️ **Le piège reste ouvert pour toute texture par objet.** `terrain` court le même
risque sur les telluriques ; il n'est masqué que parce qu'elles partagent souvent
`tex_vide()`. Toute nouvelle texture par astre doit être posée **entre deux
vidages**.

⚠️ **Non testable** : c'est de l'état GPU, invisible à `cargo test`. Le garde
honnête serait une comparaison d'images rendues, que le projet n'a pas. C'est le
**quatrième** défaut de la session où une valeur juste n'atteignait pas le shader
(calotte, villes, atmo de Titan, texture zonale) : la donnée était bonne, la
plomberie non.

⚠️ **Coût** : le vidage est fait à chaque planète, donc plus d'appels de dessin
qu'avant. Invisible aux effectifs actuels ; si la vue système rame un jour, ne
vider que sur **changement** de textures plutôt qu'inconditionnellement.



---

## Trois faits sur GLSL ES 100, établis par la recherche du 2026-09-26

Rassemblés en cherchant comment rendre un disque d'accrétion
([`conception/astres_v2_disque_accretion_2.md`](../conception/astres_v2_disque_accretion_2.md)),
mais ils ne tiennent à aucun chantier.

- ⚠️ **Les dérivées ne sont pas core.** `fwidth`, `dFdx`, `dFdy` sont des
  fonctions **optionnelles** de GLSL ES 100 : elles exigent
  `#extension GL_OES_standard_derivatives : enable`, et l'extension n'existe
  que là où le contexte l'expose. **Ne pas y adosser l'anti-crénelage d'un
  bord.** La parade portable : faire calculer la taille projetée par le CPU
  et la passer en uniform.
- **Pas de contrôle de LOD en fragment** (`texture2DLodEXT` est une
  extension). Une octave fine de bruit ne peut donc pas être filtrée par
  mip : il faut la fondre analytiquement, sinon elle scintille en vue
  rasante.
- **Les cubemaps, elles, sont core** — et miniquad 0.4.10 expose bien
  `TextureKind::CubeMap`. C'est ce qui rendra faisable la **lentille
  gravitationnelle du fond stellaire** (lot 8 du chantier astres) sans
  copie d'écran : dévier analytiquement la direction de vue, puis
  échantillonner le fond. Noté ici parce que c'est le genre de possibilité
  qu'on croit fermée par défaut.

**Décompte des pipelines au 2026-09-26** : **11 sites de chargement** sur les
32 du plafond, après l'ajout du shader d'accrétion. Le test
`impostor::tests::le_nombre_de_shaders_reste_sous_le_plafond` sonne à chaque
arrivée — c'est le moment de vérifier que le nouveau venu est bien mis en
cache.
