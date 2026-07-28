# Conception — Champs de débris unifiés (ceintures, anneaux, disques)

> Document de conception, phase avant code. Périmètre : un **système unique et
> paramétrique** qui absorbe `src/ceinture/` (astéroïdes/Kuiper) ET
> `src/planete/anneau.rs` (anneaux planétaires), et couvre en plus : disques
> protoplanétaires, disques protosolaires, disque épars et nuage de Oort.
> Contrainte forte : **impact performance minimal** — idéalement meilleur que
> l'existant. Le disque d'accrétion du trou noir (`soleil`, couronne_type 5)
> reste hors périmètre mais le modèle doit pouvoir le reprendre plus tard.

---

## 1. Diagnostic critique de l'existant

Trois systèmes disjoints font aujourd'hui « des cailloux qui tournent » :

### 1.1 `ceinture/` : tout le travail au mauvais endroit (CPU, chaque frame)

`Ceinture` stocke 900–1400 `Asteroide` avec orbite circulaire analytique. Deux
boucles par frame :

- `update()` : `angle += omega * dt` pour chaque corps — un état mutable pour
  une quantité **purement dérivable** (`angle = angle0 + omega * t`).
- `dessiner()` : pour CHAQUE astéroïde, calcul trigonométrique de la position,
  billboarding manuel (`cam.right`/`cam.up`), reconstruction complète de
  `verts`/`inds`, flush tous les 400 quads (~6 draw calls et ~9 200 sommets
  recalculés par frame pour le système par défaut).

C'est exactement le travail d'un vertex shader. Le CPU paie chaque frame ce qui
pourrait être payé **une fois à la création**.

### 1.2 `planete/anneau.rs` : l'alpha dans la géométrie

Le mesh est statique (bien), mais le profil radial/angulaire est *cuit dans les
sommets* : 44 × 120 = 5 280 quads potentiels pour encoder ce qu'un fragment
shader calcule en quelques lignes. Conséquences :

- résolution des lacunes plafonnée par la grille (Encke = 1 bande gaussienne
  approximée sur 44 pas radiaux) ;
- **aucune animation** possible (rotation képlérienne, scintillement) sans
  reconstruire le mesh ;
- shader trivial (passthrough couleur) : pas d'éclairage, pas de face
  jour/nuit, pas d'ombre planétaire — l'anneau est plat visuellement ;
- cinq styles (`profil()`) codés dans un `match` CPU, non composables (on ne
  peut pas avoir « Saturne + arcs »).

### 1.3 Duplication conceptuelle

`asteroides`/`kuiper` (ceinture), style 1 « ceinture granuleuse » et style 3
« débris récents » (anneau), disque d'accrétion (soleil) : trois écritures du
même objet — *un champ de matière en orbite képlérienne autour d'un centre*.
Le preset galerie « Anneau ceinture d'astéroïdes » est un anneau plat qui
*imite* une ceinture de particules, faute de système commun.

---

## 2. Modèle unifié : le `Disque`

Un **champ de débris** = un centre (étoile, barycentre ou planète), un repère
orbital, et la superposition d'au plus deux couches complémentaires :

| Couche | Technique | Sert pour |
|---|---|---|
| **Voile** | annulus statique low-poly + **fragment shader procédural** | anneaux denses (Saturne, Uranus, arcs), gaz/poussière des disques proto*, poussière zodiacale |
| **Particules** | quads statiques + **vertex shader orbital** (billboard GPU) | astéroïdes, Kuiper, disque épars, Oort, planétésimaux, gros blocs d'anneaux |

Les deux couches partagent la même config géométrique et le même repère. Un
type d'amas = un preset qui active l'une, l'autre, ou les deux.

```
Ceinture principale   = particules seules
Kuiper / disque épars = particules seules (épaisseur / excentricité ↑)
Nuage de Oort         = particules seules (sphéricité = 1)
Anneau de Saturne     = voile dense (+ option : particules proches)
Anneau d'Uranus       = voile fin
Arcs de Neptune       = voile à arcs
Débris récents        = particules clumpées + voile ténu
Disque protoplanétaire= voile épais à cavités + particules (planétésimaux)
Disque protosolaire   = voile continu jusqu'au centre, émissif
```

### 2.1 Principe de performance n° 1 : géométrie immuable, temps en uniform

**Rien n'est recalculé côté CPU après la création.** Le buffer de sommets est
construit une fois ; l'animation entière passe par `u_time` (+ le repère caméra
pour le billboarding). `update()` devient un no-op. La position orbitale est
dérivée dans le vertex shader :

```
theta  = angle0 + omega * u_time        // omega = sqrt(GM / r³) — Kepler gratuit
pos    = a1 * (r * cos(theta)) + q * (r * sin(theta))
sommet = pos + cam_right * coin.x + cam_up * coin.y
```

La rotation différentielle (le cœur visuel d'un disque : l'intérieur tourne
plus vite) est un **sous-produit gratuit** de `omega(r)` — là où la V1 devait
la simuler par particule en CPU.

Nuance honnête : `draw_mesh` de macroquad re-streame les sommets vers le GPU à
chaque frame (batching immédiat). Le gain n'est donc pas « zéro coût » mais
« memcpy d'un buffer figé » au lieu de « trig + billboard + rebuild par corps ».
Si un jour ça ne suffit plus (dizaines de milliers de particules), l'issue de
secours est un VBO persistant via miniquad directement — l'architecture shader
proposée ici y est déjà compatible, seul l'upload change. À ne PAS faire en
phase 1 (YAGNI).

### 2.2 Principe n° 2 : encoder l'orbite dans les canaux existants

macroquad n'offre que trois canaux par sommet (`Vertex::new2`) : `position:
Vec3`, `uv: Vec2`, `color: [u8;4]`. Ça suffit, en cessant de stocker la
*position* pour stocker les *éléments orbitaux* :

| Canal | Contenu |
|---|---|
| `position` | `(phi, incl, r)` — longitude du plan orbital, inclinaison, rayon |
| `uv` | `(coin_id ∈ {0,1,2,3} + graine ∈ [0,1[ , taille)` — partie entière/fractionnaire |
| `color` | teinte RGB de la particule + alpha |

Le vertex shader reconstruit `a1 = (cos phi, 0, sin phi)`,
`q = a2·cos(incl) + Y·sin(incl)`, `angle0 = hash(graine) * TAU`, l'irrégularité
des coins (`hash(graine + coin_id)` — les « cailloux » actuels), et
l'excentricité optionnelle `e = hash(graine · 7.3) * u_ecc_max` avec
l'approximation d'ellipse `r(θ) = a(1-e²)/(1+e·cos(θ-θ_p))` (vitesse angulaire
uniforme : approximation visuellement suffisante, cohérente avec l'analytique
circulaire actuel — on ne résout pas Kepler dans un shader pour des cailloux).

Le repère du disque (normale, éventuel basculement) passe en uniforms : les
mêmes sommets servent quel que soit le plan.

### 2.3 Principe n° 3 : le voile est un shader, pas une grille

L'annulus du voile descend à **8 anneaux × 48 segments = 384 quads** (contre
5 280), uniquement pour épouser la courbure. Tout le contenu part dans le
fragment shader, dans le style de la maison (socle impostor, table d'uniforms,
hot-reload R) :

- coordonnée radiale `t` et angulaire `ang` en varying ;
- **profil radial** : somme de bandes gaussiennes + lacunes paramétrées en
  uniforms (jusqu'à ~4 lacunes : centre, largeur, profondeur) — Cassini et
  Encke deviennent des paramètres, plus des constantes ;
- **modulation angulaire** : arcs (Neptune) = enveloppes gaussiennes sur `ang`,
  granulation = bruit cellulaire 2D `(t, ang)` — remplace les `hash(cell)` CPU ;
- **animation** : `ang' = ang - omega(t) * u_time` → le voile tourne
  différentiellement, granulation comprise. Impossible avec l'ancienne
  géométrie cuite ;
- **émission** : gradient radial émissif (proto-disques : intérieur chaud
  blanc-orangé → bord froid rougeâtre) ;
- coût : ~un fbm 2D léger par fragment, sur une surface écran généralement
  faible. Comparable au shader du moindre impostor existant.

Les cinq styles de `profil()` deviennent des **jeux de valeurs d'uniforms**
(donc composables et interpolables), pas des branches.

### 2.4 Géométrie généralisée : du disque plat à la coquille de Oort

Deux paramètres étendent le modèle au-delà du disque :

- `epaisseur` : dispersion des inclinaisons (existant, conservé) ;
- `spherite ∈ [0,1]` : 0 = inclinaisons dans `±epaisseur` (disque) ;
  1 = distribution isotrope sur la sphère (`incl = asin(2u-1)`, `phi` uniforme).
  Kuiper ≈ 0 avec epaisseur 0.28 ; disque épars ≈ 0.15 avec excentricités
  fortes ; **Oort = 1.0**.

Le nuage de Oort a en plus un régime LOD propre : à sa distance, les particules
sont sous le pixel → rendu en **points** (quads d'une taille écran minimale,
clampée dans le vertex shader), très clairsemés, alpha faible. C'est une
ambiance, pas une structure.

---

## 3. Paramétrage

### 3.1 `DisqueConfig` (remplace `CeintureConfig` + les champs `anneau_*` d'`Apparence`)

```rust
pub struct DisqueConfig {
    // — Repère —
    pub normale: Vec3,        // plan du disque
    pub r_in: f32,            // unités monde
    pub r_out: f32,
    pub gm: f32,              // G * masse du parent (vitesses képlériennes)

    // — Distribution —
    pub epaisseur: f32,       // inclinaison max (rad)
    pub spherite: f32,        // 0 disque … 1 coquille (Oort)
    pub ecc_max: f32,         // excentricité max (disque épars)
    pub profil_radial: f32,   // exposant densité(r) : 0 uniforme, >0 concentré interne
    pub clumping: f32,        // 0 uniforme … 1 amas (débris récents)
    pub graine: f32,

    // — Couche particules (nb = 0 → désactivée) —
    pub nb: usize,
    pub taille_min: f32,
    pub taille_max: f32,      // distribution u⁴ conservée (gros rares)
    pub couleur: Vec3,        // + variation par particule dans le shader

    // — Couche voile (voile_alpha = 0 → désactivée) —
    pub voile_alpha: f32,
    pub voile_couleur: Vec3,
    pub voile_couleur2: Vec3, // 2e teinte (bandes internes/externes)
    pub bandes: f32,          // fréquence des bandes radiales
    pub granulation: f32,     // bruit cellulaire (0 lisse, 1 granuleux)
    pub lacunes: [Vec4; 4],   // (centre t, largeur, profondeur, ondulation bord) ; largeur 0 = inactif
    pub arcs: f32,            // 0 anneau complet … 1 arcs isolés (Neptune)
    pub emissif: f32,         // gradient chaud interne (proto-disques)
    pub rotation: f32,        // vitesse visuelle du voile (facteur sur omega(t))
}
```

Constructeurs presets (mêmes signatures d'appel qu'aujourd'hui pour limiter la
casse dans `genese`) : `asteroides()`, `kuiper()`, `disque_epars()`, `oort()`,
`anneau_saturne()`, `anneau_uranus()`, `anneau_arcs()`, `debris_recent()`,
`protoplanetaire()`, `protosolaire()`.

### 3.2 Lacunes sculptées par des corps réels

Une lacune n'est pas forcément décorative : dans les vrais systèmes, c'est un
**corps en orbite qui la creuse** — lune bergère dans les anneaux d'une
gazeuse (Pan dans Encke, Daphnis dans Keeler), proto-planète dans son sillon
de disque protoplanétaire (PDS 70b). Le modèle le prend en charge à deux
niveaux, sans rien coûter au rendu :

- **Couplage à la création (genese).** Quand une lune orbite dans l'étendue
  radiale d'un disque (`r_in < r_lune < r_out`), genese ouvre une lacune
  centrée sur `t = (r_lune - r_in)/(r_out - r_in)`, de largeur croissante avec
  la taille du corps. Même règle pour les jeunes planètes d'un disque
  protoplanétaire : on place d'abord les proto-planètes (petits corps sombres
  ou rougeoyants, rendus normalement comme astres), puis les sillons du voile
  s'alignent dessus. La lacune reste un simple uniform — le fragment shader
  ignore tout du corps, seule la génération garantit la cohérence. Corollaire :
  la couche particules respecte aussi les lacunes (rejet à la création des
  particules tirées dans une lacune profonde — coût nul à l'exécution).
- **Bords vivants (composante `ondulation` de la lacune).** Une lune bergère
  laisse des vagues sur les bords de sa lacune (les festons de Daphnis). Le
  4e paramètre module les bords en fragment shader :
  `bord(ang) = largeur · (1 + ondulation · sin(k·ang + phase))` avec
  `phase = angle orbital du corps` passé en uniform (un float par lacune,
  recalculé trivialement CPU depuis `angle0 + omega·t` — le seul couplage
  runtime, négligeable). L'ondulation est plus marquée en aval du corps,
  s'amortit en s'éloignant — lisible même sans voir la lune.

Cas dégradé assumé : une lacune peut rester purement décorative (Cassini,
creusée par une résonance et non un corps embarqué) — `ondulation = 0`, pas de
corps associé.

### 3.3 Les deux modes d'attache

- **Autour d'une étoile / du barycentre** : `Disque` implémente `Astre` comme
  l'actuelle `Ceinture` (masse nulle, catégorie `Asteroide`), ajouté par
  `genese`.
- **Autour d'une planète** : la planète possède un `Option<Disque>` à la place
  des champs `anneau_*` bruts. `Apparence` garde des builders de même nom
  (`avec_anneau_saturne(...)` etc.) qui remplissent une `DisqueConfig` — la
  galerie et les presets existants compilent sans changement. Le disque suit
  la position du parent via l'uniform `u_centre` (déjà le cas : quads relatifs
  au centre).

### 3.4 Ce que deviennent les types demandés

| Type | Recette |
|---|---|
| **Ceinture d'astéroïdes** (débris plus ou moins grands) | particules, `taille_max` et distribution ajustables, `clumping` léger possible (familles de collision) |
| **Ceinture de débris planétaire** (gazeuse ou tellurique) | particules autour d'une planète, `gm` de la planète ; + voile ténu si poussière ; `debris_recent()` = clumping fort + teintes chaudes |
| **Disque protoplanétaire** | voile épais (`voile_alpha` ↑, `emissif` ↑, granulation) + 2–3 `lacunes` larges alignées sur des **proto-planètes réellement placées dans leurs sillons** (§ 3.2, façon HL Tauri / PDS 70) + particules planétésimaux clairsemées dans les zones denses |
| **Disque protosolaire** | voile continu de `r_in ≈ 0` au bord, `emissif` fort au centre, aucune lacune, granulation turbulente ; se marie avec la couronne « jets bipolaires » (protoétoile) existante de `soleil` |
| **Disque épars** | particules, `ecc_max ≈ 0.5`, `epaisseur` forte |
| **Nuage de Oort** | particules-points, `spherite = 1`, alpha faible, `r_in/r_out` très grands |

Anneaux planétaires actuels : les styles 0–4 de `anneau.rs` se retrouvent tous
(Saturne = bandes + 2 lacunes ; Uranus = 1 bande fine ; Neptune = `arcs` ;
granuleux/débris = `granulation`/`clumping`). Parité visuelle exigée avant de
supprimer l'ancien code (captures de non-régression, l'outillage existe).

---

## 4. Performance : budget et LOD

### 4.1 Comparatif (système par défaut : 900 + 1400 particules, 1 planète annelée)

| | Avant | Après |
|---|---|---|
| CPU / frame | 2 300 × (trig + billboard + push 4 sommets) + boucle `update` | **0** (buffers figés, `u_time` seul) |
| Draw calls | ~7 (ceintures, lots de 400 quads — clamp du batcher macroquad) + 1 (anneau) | inchangés en nombre (même contrainte de batch), mais chaque lot est un memcpy de tranche figée au lieu d'un rebuild |
| Sommets anneau | jusqu'à 21 120 | 1 536 |
| Animation | rotation particules seulement | particules + voile, rotation différentielle |

### 4.2 LOD (règles simples, pas d'usine)

1. **Particule sous-pixel** : taille écran clampée à ~1.5 px dans le vertex
   shader + atténuation d'alpha — pas de scintillement, pas de tri.
2. **Disque lointain** : quand le rayon apparent du disque passe sous un seuil,
   ne dessiner que le voile (les ceintures sans voile gardent leurs particules-
   points : c'est leur seule présence visuelle). Un simple test par frame et
   par disque.
3. **Nombre de particules** : fixé à la création selon le contexte (galerie
   riche, système généré plus sobre). Pas de streaming dynamique.
4. **Culling** : disque entièrement hors champ → skip du draw (test
   sphère-frustum grossier sur `r_out`, comme les autres astres).

### 4.3 Transparence et ordre de rendu

Le voile est alpha-blendé sans depth-write (comme `material_anneau` actuel) ;
les particules restent opaques (depth-write on, aucun tri nécessaire — gros
avantage conservé de la V1). Ordre : opaque d'abord (particules avec le reste
de la scène), voiles ensuite, triés grossièrement par distance caméra
(quelques disques par scène : tri trivial).

---

## 5. Éclairage (hooks, pas de sur-conception)

Phase 1 : reprendre l'existant (particules teintées, voile plat) pour la
parité. Mais le passage en shader ouvre, à coût quasi nul :

- **face jour/nuit du voile** : `dot(normale_disque, dir_lumiere)` module la
  luminosité (les anneaux vus du côté non éclairé sont plus sombres) ;
- **terminateur sur les particules** : assombrir selon
  `dot(dir_particule_soleil, cam_forward)` — approximation billboard suffisante ;
- **ombre de la planète sur l'anneau** : projection du cylindre d'ombre en
  fragment shader (un cône assombri au-delà de la planète). C'est LE détail
  qui vend un anneau de Saturne. Prévu en phase 4, uniform `u_dir_soleil` posé
  dès la phase 2 ;
- multi-étoiles : l'uniform lumière devient un tableau court le moment venu
  (chantier systèmes multiples — anneaux V2 « ombres croisées » de la
  bucketlist gazeuses).

---

## 6. Plan d'implémentation (chaque étape compile et se voit)

1. **`disque/` — couche particules GPU.** Nouveau module (config, builder de
   buffer, vertex shader orbital, material dédié via `impostor::source` pour
   le hot-reload). Brancher `asteroides()` et `kuiper()` dessus dans `genese`
   (mêmes rayons/couleurs). Vérifier la parité visuelle, puis supprimer
   `ceinture/`. *Livrable : mêmes ceintures, zéro CPU par frame.*
2. **Couche voile.** Annulus low-poly + fragment shader procédural + table
   d'uniforms. Porter les 5 styles de `profil()` en presets d'uniforms.
   `Planete` bascule sur `Option<Disque>` ; `Apparence` garde ses builders.
   Captures de non-régression sur la galerie annelée, puis suppression
   d'`anneau.rs`. *Livrable : anneaux identiques mais animés (rotation
   différentielle).*
3. **Nouveaux types.** `spherite` + `ecc_max` + points (épars, Oort) ;
   `lacunes`/`emissif`/granulation turbulente (protoplanétaire,
   protosolaire) ; `clumping` (débris récents en particules) ; lacunes
   couplées aux corps (§ 3.2) : lune bergère dans un anneau de gazeuse
   (ondulation des bords), proto-planètes placées dans leurs sillons.
   Entrées galerie pour chacun. *Livrable : les 4 types demandés + épars +
   Oort visibles, avec lunes/proto-planètes dans leurs lacunes.*
4. **Éclairage.** Face jour/nuit du voile, terminateur particules, ombre
   planétaire. *Livrable : anneaux crédibles au terminateur.*
5. **Génération.** `genese` : disques proto* pour les jeunes étoiles (lien
   couronne protoétoile), épars+Oort optionnels selon le système, ceintures de
   débris autour de certaines planètes. Échelle `ech` déjà en place.

Dépendances : 2 après 1 (réutilise material/uniforms), 3–5 indépendantes
entre elles après 2.

---

## 7. Hors périmètre (assumé)

- **Troyens L4/L5, traînées météoritiques** : écartés à la demande — le modèle
  les permettrait plus tard (enveloppe angulaire sur une orbite pour les
  troyens ; `ecc_max` + enveloppe pour les traînées).
- **Disque d'accrétion du trou noir** : reste dans `soleil` (il est stylisé et
  couplé à la lentille à venir). Candidat naturel à une migration vers le
  voile quand la lentille gravitationnelle sera conçue.
- **Interaction physique** (collisions, perturbations par les planètes,
  résonances creusant les lacunes dynamiquement) : les lacunes sont
  paramétriques, pas simulées. Masse nulle conservée.
- **VBO persistant miniquad** : issue de secours documentée (§ 2.1), pas
  implémentée tant que le profil ne le réclame pas.
