# Conception — Génération de planètes telluriques v2

> Document de conception, phase avant code. Objectif : des rendus de planètes
> aboutis, couvrant tout le spectre du catalogue (tempéré, désert, glacé, lave,
> stérile). Le catalogue lui-même sera revu **après** que cette brique soit fixée.

---

## 1. Diagnostic de l'existant

Le shader actuel (`src/shaders/planete.frag.glsl`, branche tellurique) est 100 %
procédural par pixel sur un impostor. Chaque feature est un champ de bruit
**indépendant** : altitude `h`, montagnes `rg`, humidité `moist`, rivières `rv`,
dunes, cratères… simplement superposés.

Le cas des rivières illustre le problème structurel :

```glsl
// planete.frag.glsl — état actuel
float rv = fbm(p * 1.5 + 50.0);                     // champ SANS lien avec h
float chan = 1.0 - smoothstep(0.0, 0.05, abs(rv - 0.5));
float riv = chan * smoothstep(0.55, 0.18, lh);      // seul lien : atténué en altitude
```

Les rivières sont l'isoligne 0.5 d'un bruit qui ignore l'altitude : elles
traversent les crêtes, ne convergent pas, n'ont ni affluents ni embouchures.

Un terrain crédible est une **chaîne causale** : altitude → vallées → écoulement
→ rivières → humidité → végétation. Or un fragment shader ne connaît pas ses
voisins : l'eau "coule de proche en proche", c'est un phénomène global,
impossible à calculer par pixel. D'où la décision ci-dessous.

## 2. Architecture retenue (option B, hybride)

Deux options étaient sur la table :

- **A — tout corriger dans le shader** : corréler les champs (rivières dans les
  fonds de vallée du même bruit ridged, biomes par température/humidité).
  Pas cher, mais rivières seulement *plausibles* : pas d'affluents, pas de lacs.
- **B — précalculer la géographie** : simuler réellement l'écoulement sur une
  grille CPU, stocker le résultat en textures. Vraies rivières avec affluents,
  lacs, deltas, érosion.

**Décision : B**, coût jugé acceptable (~50-100 ms par planète, une seule fois,
~1,5 Mo de texture par planète). Le shader n'est pas jeté : il garde tout
l'habillage. Répartition des rôles :

| | Précalcul CPU (Rust, 1× par planète) | Shader (par pixel, chaque frame) |
|---|---|---|
| Produit | altitude érodée, flux d'eau, humidité | couleur finale |
| Contenu | continents, chaînes, vallées, rivières, lacs | biomes (temp × humidité), détail haute fréquence, normale/éclairage, nuages, calottes, villes, biolum… |
| Source | `Apparence` + seed | textures + uniforms `Apparence` |

Les features existantes du shader (dunes, mesa, pics, cratères, voile, eyeball,
villes…) **continuent de fonctionner** par-dessus la nouvelle géographie.

## 3. Pipeline de génération (5 étapes)

```
Apparence + seed
      │
      ▼
[1] Grille cube-sphere (6 × 256²)
      │
      ▼
[2] Altitude de base        fbm + domain warping + ridged (comme le shader,
      │                     mais les crêtes sont intégrées DANS h)
      ▼
[3] Érosion                 hydraulique (gouttes) ∝ eau liquide
      │                     + thermique (éboulis), toujours active
      ▼
[4] Hydrologie              priority-flood (lacs) puis flux D8 (rivières)
      │
      ▼
[5] Bake textures           atlas RGBA8 : altitude 16 bits + flux + humidité
      │
      ▼
   Shader (habillage)
```

### 3.1 Universalité : un seul pipeline pour tout le catalogue

La géographie se calcule pareil pour tous les climats ; c'est son
**interprétation** qui change. Deux boutons pilotent le pipeline (fixés par
la logique climatique de `apparence_tellurique()`, comme aujourd'hui) :
l'intensité d'érosion hydraulique et le niveau d'eau.

| Climat | Érosion hydraulique | Le canal "flux" devient | Les dépressions deviennent |
|---|---|---|---|
| Tempéré | forte (vallées en V, deltas) | rivières + berges végétales | lacs |
| Désertique | faible (traces fossiles) | oueds secs, canyons | salines, playas |
| Glacé | moyenne, gouttes "lourdes" (vallées larges) | glaciers / rien | lacs gelés |
| Lave | quasi nulle | coulées incandescentes (`riv_lave`) | mers de lave |
| Stérile | nulle | ignoré | bassins (cratères par-dessus) |

L'érosion **thermique** (éboulis) reste active partout : c'est elle qui évite
l'aspect "bruit plastique" sur les mondes sans eau. Optimisation possible :
sauter l'érosion si `voile > 0.9` (Vénus/Titan, sol invisible).

## 4. La grille cube-sphere

### 4.1 Pourquoi pas une équirectangulaire ?

L'équirect pince les pôles (texels dégénérés, érosion faussée). Le cube-sphere
projette 6 faces carrées sur la sphère : distorsion quasi uniforme, pas de
singularité polaire.

### 4.2 Mapping texel ↔ sphère (warp équi-angulaire)

Sans correction, un texel au centre d'une face couvre ~5× la surface d'un texel
de coin. Le warp `tan` ramène l'écart à ~1,3×.

```rust
/// Face f : normale + 2 tangentes. N = résolution (256).
const FACES: [(Vec3, Vec3, Vec3); 6] = [
    // (normale,          right,             up)
    (Vec3::X,  Vec3::NEG_Z, Vec3::Y), // +X
    (Vec3::NEG_X, Vec3::Z,  Vec3::Y), // -X
    (Vec3::Y,  Vec3::X,  Vec3::NEG_Z), // +Y
    (Vec3::NEG_Y, Vec3::X, Vec3::Z),  // -Y
    (Vec3::Z,  Vec3::X,  Vec3::Y),    // +Z
    (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y), // -Z
];

/// Texel (face, x, y) -> point sur la sphère unité.
fn texel_vers_sphere(face: usize, x: usize, y: usize, n: usize) -> Vec3 {
    let a = (2.0 * (x as f32 + 0.5) / n as f32 - 1.0) * std::f32::consts::FRAC_PI_4;
    let b = (2.0 * (y as f32 + 0.5) / n as f32 - 1.0) * std::f32::consts::FRAC_PI_4;
    let (nrm, r, u) = FACES[face];
    (nrm + r * a.tan() + u * b.tan()).normalize()
}

/// Point sphère -> (face, x, y) : face = axe dominant, puis atan inverse.
fn sphere_vers_texel(d: Vec3, n: usize) -> (usize, usize, usize) {
    let ax = d.abs();
    let face = if ax.x >= ax.y && ax.x >= ax.z { if d.x > 0.0 { 0 } else { 1 } }
          else if ax.y >= ax.z              { if d.y > 0.0 { 2 } else { 3 } }
          else                              { if d.z > 0.0 { 4 } else { 5 } };
    let (nrm, r, u) = FACES[face];
    let inv = 1.0 / d.dot(nrm);
    let a = (d.dot(r) * inv).atan() / std::f32::consts::FRAC_PI_4; // [-1,1]
    let b = (d.dot(u) * inv).atan() / std::f32::consts::FRAC_PI_4;
    let x = (((a + 1.0) * 0.5) * n as f32).min(n as f32 - 1.0) as usize;
    let y = (((b + 1.0) * 0.5) * n as f32).min(n as f32 - 1.0) as usize;
    (face, x, y)
}
```

### 4.3 Stockage et voisinage

```rust
struct Terrain {
    n: usize,          // 256
    h: Vec<f32>,       // altitude,  longueur 6*n*n
    flux: Vec<f32>,    // accumulation d'écoulement
    hum: Vec<f32>,     // humidité
}
// index = face * n*n + y * n + x
```

Deux mécanismes de voisinage selon l'algorithme :

1. **Gouttes d'érosion** : déplacement en 3D sur la sphère, re-projection via
   `sphere_vers_texel` à chaque pas → *aucune* gestion de couture.
2. **Priority-flood / D8** : besoin des 8 voisins exacts. Dans la face :
   trivial. Sur un bord : table de 24 entrées (6 faces × 4 arêtes) qui dit sur
   quelle arête de quelle face on continue, et si l'axe est inversé.

```rust
/// (face, arête N/S/E/O) -> (face voisine, arête d'arrivée, axe inversé ?)
/// 24 entrées écrites à la main une fois, testées par un aller-retour.
const ARETES: [[(usize, usize, bool); 4]; 6] = [ /* ... */ ];

fn voisin(face: usize, x: i32, y: i32, n: i32) -> (usize, usize, usize) {
    if x >= 0 && x < n && y >= 0 && y < n {
        return (face, x as usize, y as usize); // cas courant, intra-face
    }
    // sinon : consulter ARETES, transformer (x, y) dans le repère d'arrivée
    /* ... */
}
```

Les 8 coins du cube ont 7 voisins au lieu de 8 — priority-flood et D8 le
tolèrent sans cas particulier.

**Test de validation clé** (à écrire dès le module grille) : pour chaque texel
de bord, `voisin()` puis `voisin()` en sens inverse doit revenir au point de
départ ; et `texel_vers_sphere ∘ sphere_vers_texel = identité`.

## 5. Étapes de génération

### 5.1 Altitude de base

Même recette que le shader actuel (cohérence visuelle), évaluée sur CPU :

```rust
fn altitude_base(d: Vec3, ap: &Apparence, rng_seed: f32) -> f32 {
    let sd = vec3(rng_seed, rng_seed * 1.7, rng_seed * 0.3);
    let freq = frequence_du_motif(ap.eau_motif);   // taille des masses (0..3)
    let p = d * freq + sd;
    let q = vec3(fbm(p + 1.3), fbm(p + 7.2), fbm(p + 3.4)); // domain warping
    let mut h = fbm(p + 1.9 * q);
    // NOUVEAU : les crêtes ridged font partie de h (le shader actuel ne les
    // met que dans la couleur) -> l'érosion et l'eau les "voient".
    let rg = 1.0 - (2.0 * fbm(p * 2.2 + 9.0) - 1.0).abs();
    h + ap.relief * 0.35 * rg * smoothstep(0.45, 0.75, h)
}
```

### 5.2 Érosion hydraulique par gouttes (méthode "droplet", cf. Sebastian Lague)

~50 000 gouttes. Chacune : naît en un point aléatoire, descend le gradient,
arrache du sédiment quand elle accélère, le dépose quand elle ralentit,
s'évapore. Le terrain "bruit" devient un terrain "sculpté" : vallées en V,
piémonts adoucis, deltas.

```rust
struct ParamsErosion {
    nb_gouttes: u32,     // ∝ intensité climatique (0 pour stérile)
    inertie: f32,        // 0.05 ; plus haut = vallées plus larges (glaciaire)
    capacite: f32,       // sédiment max ∝ vitesse * pente
    erosion: f32,        // 0.3  : fraction arrachée
    depot: f32,          // 0.3  : fraction déposée
    evaporation: f32,    // 0.02 ; plus haut = rivières courtes (désert)
    pas_max: u32,        // 64
}

fn eroder(t: &mut Terrain, p: &ParamsErosion, rng: &mut Rng) {
    for _ in 0..p.nb_gouttes {
        let mut pos = point_aleatoire_sphere(rng); // 3D !
        let (mut vel, mut eau, mut sediment) = (Vec3::ZERO, 1.0f32, 0.0f32);
        for _ in 0..p.pas_max {
            let (grad, h) = gradient_local(t, pos);   // via sphere_vers_texel
            vel = vel * p.inertie - grad * (1.0 - p.inertie);
            pos = (pos + vel.normalize_or_zero() * PAS).normalize(); // reste sur la sphère
            let dh = hauteur(t, pos) - h;
            let cap = (-dh).max(0.01) * vel.length() * eau * p.capacite;
            if sediment > cap || dh > 0.0 {
                depose(t, pos, (sediment - cap).max(0.0) * p.depot);
            } else {
                arrache(t, pos, ((cap - sediment) * p.erosion).min(-dh));
            }
            eau *= 1.0 - p.evaporation;
            if eau < 0.01 { break; }
        }
    }
}
```

`arrache`/`depose` répartissent sur les 4 texels voisins (bilinéaire inverse)
pour éviter les trous en pointe.

### 5.3 Érosion thermique (éboulis) — toujours active

Quelques passes : si la pente entre deux voisins dépasse un angle critique
(le *talus*), on transfère de la matière du haut vers le bas. ~5 lignes,
crucial pour les mondes secs/stériles.

### 5.4 Hydrologie

1. **Priority-flood** : remplit chaque dépression jusqu'à son point de
   déversement → un niveau d'eau cohérent par cuvette (futurs lacs/salines/mers
   de lave). Algorithme à file de priorité, O(n log n), quelques ms.
2. **Flux D8** : on trie les texels par altitude décroissante ; chacun ajoute
   son flux (1 + amont) à son voisin le plus bas. Résultat : réseau de drainage
   complet, affluents et confluences compris.

```rust
fn flux_d8(t: &mut Terrain) {
    let mut ordre: Vec<u32> = (0..t.h.len() as u32).collect();
    ordre.sort_unstable_by(|a, b| t.h[*b as usize].total_cmp(&t.h[*a as usize]));
    for i in ordre {
        t.flux[i as usize] += 1.0;
        if let Some(j) = voisin_le_plus_bas(t, i) {   // via la table ARETES
            t.flux[j] += t.flux[i as usize];
        }
    }
}
// rivière au texel i  <=>  flux[i] > seuil ; largeur ∝ log(flux)
```

3. **Humidité** : distance à l'eau (mer, lacs, rivières) + bruit grande
   échelle → alimente les biomes côté shader.

### 5.5 Bake : atlas 2D avec gouttière

macroquad ne expose que `Texture2D` → les 6 faces sont packées dans **un**
atlas 3×2. Autour de chaque face, une **gouttière de 1 texel** recopiée des
faces voisines : l'interpolation bilinéaire du GPU reste correcte au passage
des arêtes (sinon couture visible).

```
Atlas ((N+2)*3) × ((N+2)*2)          1 texel RGBA8 :
┌─────┬─────┬─────┐                  R = altitude, octet fort ┐
│ +X  │ -X  │ +Y  │                  G = altitude, octet faible┘ 16 bits
├─────┼─────┼─────┤                  B = flux d'eau (échelle log)
│ -Y  │ +Z  │ -Z  │                  A = humidité
└─────┴─────┴─────┘
```

Décodage : `h = (R*256 + G) / 257 / 255` → pas de marches d'escalier
(contrainte GLSL 100 / WebGL1 : pas de texture flottante garantie).

## 6. Côté shader (ce qui change)

La branche tellurique de `planete.frag.glsl` remplace le calcul de `h`,
`moist`, `rv` par une lecture d'atlas :

```glsl
// direction sphère -> uv atlas (≈ 10 lignes, GLSL 100)
vec2 dir_vers_atlas(vec3 d) {
    vec3 a = abs(d);
    float face; vec2 uv; float inv;
    if (a.x >= a.y && a.x >= a.z)      { face = d.x > 0.0 ? 0.0 : 1.0; inv = 1.0/a.x; uv = vec2(d.x>0.0?-d.z:d.z, d.y)*inv; }
    else if (a.y >= a.z)               { face = d.y > 0.0 ? 2.0 : 3.0; inv = 1.0/a.y; uv = vec2(d.x, d.y>0.0?-d.z:d.z)*inv; }
    else                               { face = d.z > 0.0 ? 4.0 : 5.0; inv = 1.0/a.z; uv = vec2(d.z>0.0?d.x:-d.x, d.y)*inv; }
    uv = atan(uv) * (4.0 / 3.14159265);            // warp équi-angle inverse, [-1,1]
    vec2 cell = vec2(mod(face, 3.0), floor(face / 3.0));
    // (N+2) = taille de face avec gouttière ; +1.0 saute la gouttière
    return (cell * (N + 2.0) + 1.0 + (uv * 0.5 + 0.5) * N) / vec2((N+2.0)*3.0, (N+2.0)*2.0);
}

vec4 t = texture2D(terrain, dir_vers_atlas(d));
float h    = (t.r * 255.0 * 256.0 + t.g * 255.0) / 65535.0;
float flux = t.b;      // interprété selon le climat (rivière/oued/glacier/lave)
float hum  = t.a;
```

Le shader **garde** et améliore :

- **Biomes** par lookup (température = latitude + altitude, humidité lue) —
  diagramme de Whittaker — au lieu des mix ad hoc actuels.
- **Détail haute fréquence** procédural par-dessus (256²/face est flou en plein
  écran ; le fbm fin actuel reste).
- **Normale perturbée** : gradient de `h` par différences finies dans 2
  directions tangentes → les versants réagissent à la position réelle du
  soleil (le `shade` actuel est une dérivée fixe). Le plus gros gain de
  "fini" visuel.
- **Ombres des nuages** au sol : rééchantillonner la couche nuage décalée vers
  la lumière, assombrir (~3 lignes, gros effet).
- Nuages, calottes, villes, biolum, eyeball, voile, anneaux : inchangés.

Les branches gazeuse et glacée du shader ne bougent pas.

## 7. Budgets et contraintes

| Poste | Valeur visée |
|---|---|
| Résolution | 6 × 256² (~400 k texels) ; 512² si le plein écran l'exige |
| Génération (1× par planète) | heightmap ~10 ms + érosion 30-80 ms + hydro ~10 ms ≈ **50-100 ms** |
| Mémoire GPU | ~1,5 Mo / planète (atlas RGBA8) |
| Rendu | *moins* cher qu'avant (1 lecture texture vs 5+ octaves fbm) |
| Déterminisme | RNG dédié par planète, seedé (pas le RNG global macroquad — sinon tout tirage ajouté décale tous les systèmes) |
| GLSL 100 / WebGL1 | pas de float texture → altitude 16 bits sur R+G |

## 8. Décisions actées / restantes

Actées :

- Option **B** (précalcul), hybride avec le shader.
- Projection **cube-sphere** équi-angulaire, atlas 3×2 + gouttière.
- Érosion par **gouttes** (pas le modèle "pipes").
- Un seul pipeline pour tous les climats ; 2 boutons (érosion, niveau d'eau)
  pilotés par la logique température existante.
- 256²/face en première intention.
- Textures = géographie ; shader = habillage ; features existantes préservées.

Restantes (ordre de conception) :

1. ~~Grille cube-sphere~~ (conçue, § 4) → à valider par les tests aller-retour.
2. ~~Érosion : paramètres par climat~~ (conçue, § 9).
3. ~~Interprétation du flux côté shader~~ (conçue, § 10).
4. ~~Biomes (temp × humidité)~~ (conçue, § 11).
5. Hors périmètre pour l'instant (noté pour plus tard) : moment de génération
   (bloquant vs paresseux), mise à jour du catalogue, vue vaisseaux/sondes.

**La conception est complète.** Ordre d'implémentation (état d'avancement) :

1. ~~`planete/terrain.rs` : grille cube-sphere + voisinage + tests~~ **FAIT**.
   Implémentation : le voisinage inter-faces se fait par re-projection 3D
   (au lieu de la table d'arêtes prévue) — zéro cas particulier, testé.
2. ~~Altitude de base + bake atlas + lecture shader~~ **FAIT**. Génération
   parallèle (1 thread/face), niveau de mer par quantile branché, génération
   paresseuse au premier draw (`Planete::draw`), gouttière validée par test.
   ⚠ Depuis cette étape, les RIVIÈRES lisent le canal flux (vide tant que
   l'étape 4 n'est pas faite) : elles sont invisibles temporairement.
3. ~~Érosion thermique puis hydraulique~~ **FAIT**. Ordre : thermique ×2
   (stabilise le bruit), hydraulique (gouttes 3D sur la sphère, RNG SplitMix64
   déterministe), thermique ×1 (gomme les pointes). Paramètres dérivés de
   l'Apparence (§ 9.3) : `intensite = f(eau, lave, voile)`, vallées en U si
   `calotte < 0.35`, oueds courts si `eau < 0.15`, talus abaissé si dunes.
   `QUALITE = 0.25` goutte/texel (curseur perf/beauté dans `terrain.rs`).
4. ~~Priority-flood + D8 + encodage flux~~ **FAIT** — les rivières sont de
   retour, avec affluents, lacs et humidité par distance à l'eau. Détail
   d'implémentation : sur une sphère fermée, le flood part du minimum global
   (= l'océan, jamais rempli) et le pointeur d'inondation DONNE la direction
   de drainage -> pas de D8 séparé, les lacs sont traversés jusqu'à leur
   exutoire. Humidité : BFS multi-source depuis l'eau (décroissance ~40
   texels) + bruit, puis rang 0..1 ; le shader seuil la végétation à
   `1 - veg_couv` (couverture garantie, § 11.2 bis).
5. ~~Volcanisme (§ 11 bis)~~ **FAIT**. Déclencheur : `lave`, `cryo` ou
   `riv_lave` ; 2 + 10×intensité édifices (cône patiné + caldeira), semés
   AVANT l'érosion. La caldeira devient lac (de lave) via priority-flood et
   déborde par le point bas du rebord -> coulée dans la vraie vallée. Chaleur
   -> humidité si `cryo` (anneau de forêt de fonte, rang § 11.2 bis). Au
   shader : `regime_lave = max(riv_lave, lave > 0.3)` -> les mondes de lave
   ont des coulées incandescentes même sans `riv_lave` explicite.
6. ~~Shader : finitions~~ **FAIT** (validé glslang, GLSL ES 100) :
   - **Normale perturbée** : gradient d'altitude lu dans l'atlas (2 tangentes)
     -> les versants réagissent à la vraie position du soleil ; remplace
     l'ancien `shade` à direction fixe. L'eau reste lisse, l'amplitude suit
     `relief`.
   - **Ombres portées des nuages** : le champ nuageux est rééchantillonné
     décalé vers le soleil, le sol s'assombrit sous les nuages.
   - **Température locale** : calotte et refroidissement seuillent
     `froid = latitude + altitude·relief` -> la neige descend sur les
     montagnes, plus seulement aux pôles.
7. ~~Outillage de benchmark~~ **FAIT** — mesures à réaliser sur machine cible :
   - **Touche B** (galerie telluriques) : bench complet en tâche de fond —
     tout le catalogue en 256² avec détail par étape (bruit / volcans /
     érosion / hydro / bake), stats (min/médiane/moyenne/max), top 5 lents,
     échantillon 512². Rapport écrit dans `bench_terrain.txt` + console.
   - **Overlay permanent** en bas de la galerie : FPS, filtre pixel ON/off,
     nb de terrains générés, dernier temps, temps moyen.
   - Répartition typique du coût (VM 2 cœurs, monde tempéré 256²) : érosion
     ~62 %, bruit ~26 %, hydrologie ~11 %, bake <1 %. Le levier n° 1 est donc
     `QUALITE` (gouttes/texel) dans `params_depuis_apparence`.
   - 512² ≈ 4× le temps du 256² et 4× la mémoire GPU (~6 Mo/planète).
   - ⚠ Bencher en RELEASE (`cargo run --release`) : le rapport indique le
     profil utilisé.
   - **Bench du 2026-07-02 (8 cœurs, release)** : médiane 1061 ms, max 2040 ms
     (Barnacle), érosion dominante (~62 %), bruit ~430 ms non scalé.
   - **Optimisations appliquées suite au bench** : bruit découpé en bandes de
     lignes sur tous les cœurs (`available_parallelism`, plus seulement 6
     threads) + warp à 3 octaves (−30 % de bruit) ; érosion : `hauteur_bi`
     pour le point d'arrivée (2× moins de lectures/pas), `QUALITE` 0.25→0.15
     avec taux d'érosion 0.30→0.38 (creusement préservé), 48 pas max.
     Mesure VM 2 cœurs : monde tempéré 1,54 s → 0,85 s (−45 %) ; attendu
     mieux encore sur 8 cœurs (le bruit scale enfin). À re-bencher (touche B)
     et comparer les captures C avant/après (l'érosion est un peu plus douce).

Bonus galerie : **P** = filtre pixel ON/OFF (phase 3D rendue en demi-résolution
upscalée en plus proche voisin, textes nets — préfigure le style pixel final),
**C** = captures de non-régression, **B** = bench.
8. ~~Mini-chantier « anti-autocollant » (glace + bioluminescence)~~ **FAIT** :
   - **Glace** : la calotte distingue BANQUISE (liseré bleuté le long des
     côtes -> le trait de côte reste lisible, vieille banquise au large, plus
     de reflet spéculaire d'eau libre) et GLACE TERRESTRE (neige sur les
     hauteurs, langues glaciaires bleutées dans les vallées, rivières/lacs
     gelés en veines de glace vive qui suivent le réseau d'écoulement).
   - **Bioluminescence** : la lueur nocturne suit la géographie — forêts
     (seuil d'humidité = celui de la végétation), fleuves/lacs (canal flux),
     plancton le long des côtes — au lieu d'un bruit plaqué indépendant.
   - **Lave** : coulées et lacs de lave désormais ÉMISSIFS (une rivière de
     lave n'est jamais sombre la nuit), pulsation lente, suivent le canal
     flux. Sous une calotte, la lueur transperce les veines de glace ->
     effet « lave sous glace fracturée » (Glaciovolcanic, Crevasse). Les
     fissures diffuses (`lave`) restent une couche géologique séparée.

## 9. Paramétrage climatique de l'érosion (conçu)

### 9.1 Niveau de la mer par quantile

Le shader actuel utilise un seuil fixe (`sea = mix(0.36, 0.60, eau)`) sur un
bruit dont la distribution varie → couverture d'eau imprévisible. Sur CPU on a
l'histogramme complet :

```rust
/// eau = 0.7 -> exactement 70 % de la surface sous l'eau.
fn niveau_mer(t: &Terrain, eau: f32) -> f32 {
    let mut tri: Vec<f32> = t.h.clone();
    tri.sort_unstable_by(f32::total_cmp);
    tri[((tri.len() - 1) as f32 * eau) as usize]
}
```

Même mécanisme réutilisable pour la latitude de calotte ou la limite forêt/roche.

### 9.2 Encodage des lacs

Après priority-flood, pour chaque texel de cuvette sous le niveau de
déversement : `h = niveau_de_remplissage` (plan d'eau plat) et `flux = 1.0`
(valeur saturée). Règle unique côté shader :

- `flux ≈ 1.0` → eau **stagnante** : lac / saline / lac gelé / mer de lave
  selon le régime climatique ;
- `seuil < flux < ~0.9` → cours d'eau, largeur ∝ `log(flux)` ;
- `flux < seuil` → rien (le canal sert quand même à l'humidité).

### 9.3 Deux boutons, paramètres dérivés

La logique climatique de `apparence_tellurique()` (bandes de température) fixe
seulement :

- `intensite_hydro` ∈ [0, 1] — combien d'eau a sculpté ce monde ;
- `regime` — comment interpréter l'eau : `Liquide`, `Gele`, `Lave`, `Aucun`.

Tout le reste se dérive :

| Bande | intensite_hydro | inertie | évaporation | régime → rendu du flux |
|---|---|---|---|---|
| Tempéré 255-350 K | 0.8-1.0 | 0.05 (vallées en V) | 0.01 (fleuves longs) | rivières + berges végétales |
| Sec 350-450 K | 0.15-0.3 | 0.05 | 0.08 (oueds courts) | oueds secs, salines |
| Étuve 450-600 K | 0-0.1 | — | — | rien |
| Lave > 600 K | 0 | — | — | coulées (`riv_lave`), mers de lave |
| Froid 180-255 K | 0.4-0.6 | 0.3 (vallées en U) | 0.02 | rivières + glaciers |
| Gelé < 180 K | 0.1-0.25 | 0.5 | 0.03 | glaciers, lacs gelés |

```rust
fn params_erosion(intensite: f32, regime: Regime, dunes: f32, n_texels: usize) -> ParamsErosion {
    ParamsErosion {
        nb_gouttes: (QUALITE * intensite * n_texels as f32) as u32, // QUALITE ≈ 0.15-0.4
        inertie: match regime { Regime::Gele => 0.4, Regime::Liquide => 0.05, _ => 0.05 },
        evaporation: /* table ci-dessus */,
        ..ParamsErosion::DEFAUT
    }
}
```

- **Érosion thermique** : active partout, intensité fixe ; angle de talus
  abaissé si `dunes > 0` (le sable s'éboule avant la roche).
- `QUALITE` (gouttes par texel) est LE curseur perf/beauté, à calibrer au
  benchmark une fois le pipeline codé.
- Optimisation : sauter l'érosion hydraulique si `voile > 0.9` (sol invisible).

## 10. Interprétation du flux côté shader (conçu)

### 10.1 Encodage du canal B (au bake)

Le flux D8 brut varie de 1 (crête) à ~n_texels (embouchure d'un grand fleuve) :
une échelle log est indispensable.

```rust
// Bake : normalisation log. Les lacs (§ 9.2) ont déjà flux = saturé.
let b = if lac { 1.0 } else { (1.0 + flux).ln() / (1.0 + flux_max).ln() * 0.9 };
// -> cours d'eau dans [0, 0.9], eau stagnante = 1.0 : jamais d'ambiguïté.
```

### 10.2 Lecture (une seule règle, quatre rendus)

```glsl
float fx  = t.b;                          // flux normalisé lu dans l'atlas
float lac = smoothstep(0.93, 0.97, fx);   // eau stagnante
// Seuil de rivière piloté par l'uniform `rivieres` existant (contrôle artistique
// conservé) : rivieres = 1 -> réseau dense, 0.3 -> grands fleuves seulement.
float seuil = mix(0.75, 0.45, rivieres);
float canal = smoothstep(seuil, seuil + 0.08, fx);  // bord doux = largeur ∝ log(flux)
```

La **largeur** vient gratuitement : `fx` monte vers l'aval, donc la bande
au-dessus du seuil s'élargit vers l'embouchure ; l'interpolation bilinéaire
donne des bords lisses malgré la grille 256².

Rendu selon `regime` (uniform, remplace l'actuel `riv_lave`) :

| régime | canal (cours d'eau) | lac (stagnant) | spéculaire |
|---|---|---|---|
| Liquide | `couleur3` assombrie ∝ fx | lac : `couleur3` profonde | oui (`wet`) |
| Sec | lit sombre à sec, pas d'eau | saline / playa claire | non |
| Gelé | glacier blanc-bleu + crevasses (fbm fin) | lac gelé (glace texturée § existant) | léger |
| Lave | reprend le rendu émissif `riv_lave` actuel | mer de lave émissive | non |

### 10.3 Ce qu'on ne code PAS

- **Berges végétales** : aucun cas particulier — le canal A (humidité) est déjà
  plus élevé près de l'eau (§ 5.4), la végétation suit toute seule via les
  biomes (§ 11).
- **Deltas / confluences** : déjà dans la géométrie du flux D8.
- **Anti-grille** : léger warp procédural haute fréquence de la direction
  d'échantillonnage (`d += fbm_fin * 0.002`) pour casser l'alignement des
  texels en très gros plan. Une ligne.

## 11. Biomes par température × humidité (conçu)

### 11.1 Les deux axes

```glsl
// Température locale : base climatique - latitude - altitude.
// temp_norm = 0 (glacial) .. 1 (brûlant) ; uniform dérivé de temp_equilibre().
float tloc = temp_norm - grad_lat * lat * 0.5 - h * 0.35;   // lapse rate
float hum  = t.a;                                            // canal humidité
```

`calotte` devient un seuil sur `tloc` (et plus sur la latitude seule) → la
neige descend naturellement en altitude, les calottes suivent le climat.

### 11.2 La table ne fixe pas des couleurs, mais des POIDS de palette

Crucial pour le catalogue : une planète peut être violette. Le lookup
(tloc × hum) renvoie des poids de mélange entre les couleurs de `Apparence`,
jamais des couleurs absolues :

```
        hum ->  0            0.3           0.6           1
tloc
 1 (chaud)     désert c1     savane        forêt dense   marécage
               +dunes        veg×0.7 sèche veg_couleur   veg sombre + eau
 0.6           steppe        prairie       forêt         forêt humide
 0.3           toundra c2    toundra+veg   taïga veg×0.6 tourbière
 0 (froid)     roche gelée   glace         glace         banquise
```

```glsl
// Implémentation : pas une texture de lookup, 4-5 mix ordonnés (GLSL 100) :
vec3 sol   = mix(couleur2, couleur, smoothstep(0.0, 0.6, lh));   // roche/sol nu
vec3 vege  = veg_couleur * mix(1.2, 0.55, hum);                  // prairie -> forêt sombre
float veg  = veg_couv * smoothstep(0.25, 0.55, hum) * smoothstep(0.05, 0.3, tloc);
vec3 terre = mix(sol, vege, veg);
terre      = mix(terre, glace, smoothstep(0.12, 0.02, tloc));    // gel par température
```

Les features de style (`dunes`, `mesa`, `basalt`, `pics`…) restent des couches
par-dessus, inchangées — elles se contentent maintenant d'un masque plus juste
(ex. dunes seulement si `hum < 0.3`).

### 11.2 bis Couverture végétale garantie (normalisation par quantile)

Piège identifié : sur un monde sec à végétation (Steppe, Scrubland, "Dune
Forest"… `eau ≈ 0.05`, `veg_couv ≈ 0.5`), une humidité purement physique est
basse partout → zéro végétation alors que le preset en demande 50 %.

Correctif — même principe que le niveau de mer (§ 9.1) : au bake, l'humidité
est **normalisée par quantile** sur la planète. `veg_couv` redevient une
couverture *garantie* : la végétation occupe la fraction `veg_couv` la plus
humide de la surface, où qu'elle soit en absolu.

```rust
// Bake du canal A : rang du texel dans l'histogramme d'humidité (0..1),
// et non la valeur brute. hum = 0.8 signifie « plus humide que 80 % de la
// planète », pas « humide dans l'absolu ».
```

Conséquence heureuse sur les mondes mixtes (ex. "Dune Forest") : dunes et
forêt ne se chevauchent plus au hasard de deux bruits — l'humidité les
**répartit spatialement**. Forêts-galeries le long des fleuves, oasis autour
des lacs, ergs dans l'intérieur sec ; sur un désert, les broussailles se
placent dans les creux et lits d'oueds, pas au sommet des dunes. Le régime
sémantique (l'eau est-elle liquide, gelée, de la lave ?) reste piloté par
`regime`, la répartition par le rang d'humidité.

### 11.3 Ce que ça remplace dans le shader actuel

- `moist = fbm(...)` (bruit indépendant) → canal A précalculé.
- Végétation par `lat` brute → par `tloc` (latitude + altitude + climat).
- Calotte par latitude bruitée → par température (le bruit de côte déchiquetée
  est conservé, appliqué à `tloc`).

## 11 bis. Volcanisme (conçu, à implémenter après l'étape 4)

Constat : lave et cryo sont aujourd'hui des couches émissives du shader — aucun
édifice volcanique dans `h`, pas de coulées, pas de fonte locale.

Design (le volcanisme est une feature de GÉOMÉTRIE -> opérateur de bake, § 12) :

1. **Édifices** : K volcans semés par le RNG déterministe, estampés dans `h`
   (cône + caldeira) AVANT la passe d'érosion légère -> patinés, pas neufs.
   Déclencheur : `lave > 0` ou `cryo > 0`, densité ∝ intensité.
2. **Coulées** : le flux D8 est É MIS DEPUIS LES SOMMETS (en plus ou à la place
   de la pluie uniforme selon le régime) -> les coulées suivent les vraies
   vallées. Le régime `Lave` les rend incandescentes (rendu § 10 existant).
3. **Chaleur = humidité** (cryovolcanique) : autour d'un volcan la glace fond
   -> eau liquide -> on écrit `hum = max(hum, chaleur(dist))` au bake. Zéro
   canal supplémentaire, et les biomes (§ 11) produisent AUTOMATIQUEMENT :
   roche nue au cratère, anneau de forêt/marais de fonte là où chaud + humide,
   glace au-delà. (Le cas « volcans qui font fondre la glace, forêt là où les
   étendues le permettent ».)
4. **Shader** : quasi rien — biomes inchangés, `cryo` émissif conservé pour
   les fractures.

## 12. Couverture du catalogue — revue de cas

Stress test de la conception contre les presets existants.

| Cas | Verdict | Détail |
|---|---|---|
| Récifs / Atoll | couvert, amélioré | Récif = couche de style sur hauts-fonds (inchangé), mais la profondeur devient réelle : l'érosion dépose des sédiments aux côtes → plateaux continentaux et lagons cohérents autour des îles. |
| Archipel | couvert | `eau ≈ 0.88` par quantile → seuls les sommets émergent ; les îles sont les crêtes des chaînes érodées, alignées en arcs comme les vrais archipels. |
| Eyeball | couvert, orthogonal | Le masque (glace nocturne, anneau, lave subsolaire) se calcule sur `dot(n, L)` APRÈS le terrain, comme aujourd'hui. Option : ne rendre le flux en rivières que dans l'anneau tempéré. |
| Mesa | couvert, gain majeur | Le terrassement migre au bake, appliqué à `h` AVANT l'érosion → l'érosion creuse les plateaux, le D8 y coule des rivières = vrais canyons avec fleuve au fond. Strates colorées : restent au shader. |
| Méditerranéenne | couvert | Mers intérieures (`eau_motif`) + quantile → mers fermées ; l'humidité se concentre sur leurs pourtours → végétation côtière, intérieur sec. |
| Terre | couvert, cas vitrine | Bonus émergent : déserts continentaux automatiques (humidité = distance à l'eau → intérieur des grands continents sec, type Gobi/Sahara). |
| Cryovolcanique | couvert, inchangé | `cryo` = couche émissive dans `main()`, indépendante du terrain. Option future : fractures calées sur le flux (coulées cryo dans les vallées). |
| Bioluminescente | couvert + upgrade | Fonctionne tel quel ; option très rentable : moduler `biolum` par humidité/flux → côtes et réseaux fluviaux luminescents la nuit (plancton). |
| Dune Forest / mondes secs à végétation | couvert via § 11.2 bis | Répartition spatiale par rang d'humidité : forêts-galeries le long des fleuves, ergs dans l'intérieur. |

**Motif général** : les features de *géométrie* (mesa, éventuellement pics)
gagnent à devenir des opérateurs sur `h` au bake, avant l'érosion. Les
features de *surface/émissives* (récifs, cryo, biolum, eyeball, villes)
restent des couches shader et profitent automatiquement des nouveaux canaux
(profondeur réelle, humidité, flux).

## 13. Non-régression visuelle (implémenté)

Leçon de l'étape 4 : la montée de version a fait apparaître de l'eau sur des
mondes qui n'en ont pas (lacs du priority-flood rendus en eau sur la Lune…).
Deux garde-fous :

1. **Régime hydrologique branché dans le shader** (§ 10.2, réalisé) :
   - pas d'atmosphère NI de voile (`atmo + voile ≈ 0`) → aucun liquide, le
     flux est ignoré (Lune, Mercure, Carbone…) ;
   - air mais `eau ≈ 0` → salines/playas claires dans les cuvettes, oueds
     (lits à sec sombres) pour les rivières — jamais d'eau bleue ni de reflet ;
   - `riv_lave` → coulées/mers de lave ; `eau > 0` → régime aquatique normal.
   Le voile compte comme atmosphère (Titan garde ses lacs sous la brume).

2. **Captures de référence** : touche **P** dans la galerie → chaque cellule
   visible (au rendu définitif, pas les placeholders) est exportée en PNG dans
   `captures/<horodatage>_seed<graine>/<preset>.png`. Workflow de montée de
   version : capturer AVANT la modif, capturer APRÈS, comparer les dossiers
   (même graine = mêmes planètes, génération déterministe). Les captures sont
   la « mémoire de l'état de l'art » des types de planètes.

## 14. Références

- Inigo Quilez — *Domain warping* (déjà utilisé dans le shader actuel)
- Sebastian Lague — *Coding Adventure: Hydraulic Erosion* (modèle de gouttes)
- Amit Patel (Red Blob Games) — *Polygonal Map Generation* (rivières par graphe)
- Barnes, Lehman, Mulla — *Priority-flood* (remplissage de dépressions)
- Diagramme de Whittaker (biomes par température × précipitations)
