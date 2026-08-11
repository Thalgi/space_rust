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

