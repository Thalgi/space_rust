# Conception — Géantes gazeuses V2 (surface)

> Document de conception, phase avant code. Périmètre : la **surface nuageuse**
> (bandes, vortex, pôles, palette, animation). Les anneaux, l'éclairage/atmosphère
> et le scattering restent V1 — chantiers ultérieurs (voir § 9). Précalcul CPU
> léger autorisé (textures 1D par seed, dans l'esprit de l'atlas des telluriques).

---

## 1. Diagnostic critique de l'existant

La V1 (branche `type_p ∈ ]0.5, 1.5[` de `planete.frag.glsl`, ~250 lignes) a coché
presque toute la bucketlist : double-offset, curl warp cheap, grande tache à
rotation différentielle locale, sillage, brume polaire Worley, hexagone, limb
darkening, thermique, aurores, brume. Le résultat est honorable **sur les presets
jupitériens** — et c'est précisément le problème : le shader a une signature
Jupiter câblée en dur, et tout le reste du catalogue la subit.

### 1.1 Rotation en bloc : pas de cisaillement zonal

LE mouvement caractéristique d'une géante — les bandes qui glissent les unes
contre les autres — n'existe pas. `main()` applique une rotation **rigide**
(Rodrigues, `time * 0.01`) et l'animation de surface se réduit à la dérive du
bruit (`+ t` dans les fbm). Aucune bande n'a de vitesse propre. La planète
tourne comme une boule de billard peinte.

### 1.2 Mille-feuille de `mix()` aux couleurs codées en dur

La branche gazeuse enchaîne ~25 `mix()` successifs dont la moitié injecte des
teintes absolues :

```glsl
vec3 choco  = belt * vec3(0.62, 0.5, 0.46);            // chocolat
vec3 saumon = mix(belt, vec3(0.92, 0.62, 0.5), 0.55);  // saumon
vec3 ocre   = mix(belt, vec3(0.88, 0.62, 0.36), 0.55); // ocre
base = mix(base, vec3(1.0, 0.98, 0.92), eqf * zonemask * 0.45); // ivoire
base = mix(base, vec3(0.97, 0.85, 0.71), streak * 0.32);        // saumon clair
```

Conséquences : une Neptune bleu profond reçoit des filaments **saumon** ; une
naine brune quasi noire reçoit un équateur **ivoire propre** ; les classes
Sudarsky se distinguent par la couleur de base mais pas par la matière. Le
contrat des trois couleurs (`couleur`/`couleur2`/`couleur3`) est noyé — `couleur`
(censée être le « ton moyen ») ne sert plus que d'accent de turbulence à 40 %.

### 1.3 Structure zonale pauvre et binaire

`jet_profil` est un booléen qui plaque UN profil symétrique (EZ claire + NEB/SEB) :

```glsl
band = mix(band, 0.82, (1.0 - smoothstep(0.05, 0.18, al)) * 0.55);
float neb = smoothstep(0.18, 0.23, al) * (1.0 - smoothstep(0.34, 0.42, al));
```

Pas d'asymétrie nord/sud, pas de multi-ceintures aux largeurs variées, pas de
contrôle du **nombre** de bandes (`band_scale` est une fréquence de fbm, pas un
compte lisible). Le double-offset (dec1/dec2) casse bien la périodicité mais
produit une répartition de bandes subie, jamais composée.

### 1.4 Vortex : deux poids, deux mesures

La grande tache a un vrai traitement (projection tangente, whirlpool, sillage)
mais sa spirale interne est un `sin()` pur — effet « sillon de vinyle » :

```glsl
float spiral = 0.5 + 0.5 * sin(spot_ang * 2.0 + spot_r * 11.0 - t * 6.0);
```

Tout le reste des tempêtes (`tempetes`, ovales blancs) n'est que du **seuillage
de fbm** : des blobs sans rotation, sans œil, sans structure. Ironie : les
telluriques ont un `champ_cyclones()` complet (spirale log, Coriolis, advection
du fond, dérive) — les géantes, qui sont LE monde des vortex, n'en profitent pas.
Et il n'y a qu'UNE tache paramétrable par planète.

### 1.5 Pôles envahissants

```glsl
float polef = smoothstep(0.32, 0.72, la);
base = mix(base, polcol, polef * 0.95);
```

La calotte feutrée s'engage dès 19° de latitude et écrase 95 % du signal :
presque **la moitié de chaque hémisphère** perd ses bandes. La transition est un
fondu de couleur, pas un changement de régime (dans la réalité, les bandes se
compriment et se turbulisent avant de céder aux cyclones). En prime, la
projection polaire est plane (`dot(d, pe1/pe2)`) → Worley distordu loin du pôle.

### 1.6 Incohérences jour/nuit

L'émission thermique nocturne module par `sin(lat * band_scale)` alors que les
bandes visibles sont en double-offset fbm : la structure de nuit **ne correspond
pas** à celle de jour. L'aurore est un anneau néon fixe (`smoothstep` sur la
latitude), sans lien avec un axe magnétique.

### 1.7 Brume = lavage global

`base = mix(base, brume_couleur, brume)` : uniforme, sans dépendance à la
latitude ni au limbe. Les sub-Neptunes sont des géantes délavées, pas des mondes
voilés.

### 1.8 Coût par pixel et absence de LOD

Comptage sur le chemin gazeux complet : q1(3) + q2(3) + turb + fine + swirl +
curl(4) + dec1 + dec2 + marb + fil + flake + wisp + ov + st + st2 + ond + arms +
finsp ≈ **26-30 fbm** × 5 octaves ≈ **130-150 vnoise/pixel/frame**, en
`highp`, GLSL ES 100. La galerie paie plein tarif sur des sphères de 100 px, où
`fine` (fréq. ×13) ne fait qu'aliaser/scintiller.

### 1.9 Variété perçue < variété paramétrée

13 presets + palette HSV aléatoire corrects sur le papier, mais le hardcode
(§ 1.2) et la structure unique (§ 1.3) compriment tout vers le même rendu. Une
classe III « sans nuages » garde toutes ses marbrures ; une classe I d'ammoniac
ressemble à une Jupiter pâle.

---

## 2. Principes V2

- **P1 — Squelette CPU, chair shader.** La *structure* (jets, latitudes et
  largeurs de bandes, turbulence de cisaillement) vient d'un **profil zonal 1D
  précalculé** par seed. Le bruit ne fait plus que l'habillage (volutes, grain,
  frontières ondulées).
- **P2 — Zéro couleur en dur.** Toutes les teintes dérivées de
  `couleur`/`couleur2`/`couleur3` par opérations relatives (assombrir,
  désaturer, décaler la teinte). Rôles redéfinis et documentés.
- **P3 — Un seul système de vortex** pour la grande tache, les taches sombres,
  les ovales blancs et les cyclones : des slots paramétrés, pas des seuils de fbm.
- **P4 — Le mouvement est différentiel.** L'advection zonale par `u(φ)` remplace
  la dérive de bruit comme source principale d'animation.
- **P5 — Budget.** Plafond ~15 fbm/pixel (moitié de la V1), échantillons
  mutualisés, hautes fréquences atténuées selon la taille apparente.

---

## 3. Le profil zonal (texture 1D, CPU)

Une texture **256×1 RGBA** générée par seed côté Rust (module `planete/zonal.rs`,
même philosophie que `terrain.rs` mais trivial : < 1 ms, 1 Ko). Axe = latitude
(φ de −90° à +90°). Canaux :

| Canal | Contenu | Remplace |
|---|---|---|
| R | `u(φ)` : vitesse zonale signée (jets est/ouest alternés, jet équatorial dominant), encodée 0..1 autour de 0.5 | — (nouveau : rotation différentielle) |
| G | `b(φ)` : type de bande 0 = belt sombre .. 1 = zone claire | `dec1` basse fréquence + `jet_profil` |
| B | `s(φ)` : cisaillement local (∝ \|du/dφ\|, renormalisé) | `shear` (proxy actuel sur dec1) |
| A | réservé (altitude de brume / nuages, plus tard) | — |

### 3.1 Génération (Rust)

Somme de gaussiennes posées par le seed : un jet équatorial large (amplitude et
signe paramétrés), puis N paires de jets alternés vers les pôles, amplitudes
décroissantes, latitudes jitterées, **asymétrie nord/sud** (jitter indépendant
par hémisphère). `b(φ)` s'en déduit : zone claire là où `du/dφ` est anticyclonique,
belt sombre ailleurs — c'est physiquement le bon couplage et ça garantit que
festons et turbulence tombent **exactement** aux frontières de jets.

Paramètres `Apparence` (remplacent/complètent l'existant) :

| Param | Sens | Notes |
|---|---|---|
| `nb_bandes` | nombre de paires de jets (2..9) | remplace le rôle « fréquence » de `band_scale` |
| `jets_force` | amplitude de `u` (0 = rotation rigide) | 0 pour Uranus calme |
| `zonal_asym` | asymétrie N/S (0..1) | |
| `zonal_flou` | adoucissement des frontières | classes voilées |
| `jet_profil` | **supprimé** | devient un jeu de gaussiennes préconfiguré (preset « Jupiter ») |

Hot-reload : la texture se régénère quand un param change (comme l'atlas).

### 3.2 Consommation (shader)

```glsl
float phi = asin(clamp(dot(d, k), -1.0, 1.0));      // latitude vraie
vec4 zp  = texture2D(zonal, vec2(phi * 0.3183 + 0.5, 0.5));
float u   = (zp.r - 0.5) * 2.0;                      // vitesse du jet local
// ADVECTION DIFFÉRENTIELLE : rotation autour de k d'un angle u*t, par pixel.
float aw = u * time * vit_zonale;                    // Rodrigues, 1 rotation
vec3 dz  = d * cos(aw) + cross(k, d) * sin(aw) + k * dot(k, d) * (1.0 - cos(aw));
```

Tout l'échantillonnage de surface se fait ensuite sur `dz` : les bandes
**cisaillent réellement** entre elles, les volutes s'étirent aux frontières sans
aucun terme croissant dans le bruit (pas d'enroulement infini : le warp fbm
reste borné, seul le transport est linéaire en temps). Le double-offset survit
avec un rôle réduit : onduler les frontières de `b(φ)`, amplitude ~⅓ de la V1.

---

## 4. Palette paramétrique

Rôles contractuels des trois couleurs (à documenter dans `apparence.rs`) :

- `couleur2` = **belt** (ceintures sombres) ;
- `couleur3` = **zone** (bandes claires) ;
- `couleur` = **accent** (cœurs de tempêtes, filaments chauds, traînées).

Les ~10 teintes dérivées (filament clair/sombre, floconné, collier, sillage,
cœur de vortex, équateur, pôle-olive…) sont calculées **côté CPU** par
opérations HSV relatives sur ces trois entrées, et passées en un tableau
d'uniforms `vec3 gaz_pal[8]` :

```
pal[0] = eclaircir(zone, 0.3)          // floconné laiteux (ex-ivoire)
pal[1] = assombrir(belt, 0.4)          // filaments sombres (ex-chocolat)
pal[2] = vers(belt, accent, 0.5)       // filaments chauds (ex-saumon/ocre)
pal[3] = desaturer(refroidir(belt))    // festons froids (ex-bleu-gris)
pal[4] = eclaircir(zone, 0.5)          // collier / sillage / ovales blancs
pal[5] = accent boosté                 // cœur de grande tache
pal[6] = g_pole ; pal[7] = olive(g_pole)
```

Avantage : le shader raccourcit (plus de constantes), le hot-reload teste des
palettes entières, et une géante bleue a des filaments bleus. Les presets V1
sont reproduits en choisissant les mêmes teintes via les dérivations.

---

## 5. Vortex unifiés

Un `champ_vortex_gazeux()` sur le modèle de `champ_cyclones()` tellurique,
adapté aux anticyclones de géantes. **8 slots**, chacun : direction (hash du
seed), rayon, spin, dérive, et un **type** :

| Type | Rendu | Remplace |
|---|---|---|
| 0 — GRS | ovale chaud : cœur `pal[5]` calme, anneau de haute vitesse, collier `pal[4]`, sillage turbulent sous le vent | grande tache V1 |
| 1 — sombre | ovale sombre fondu, compagnons blancs dans le sillage | tache sombre V1 |
| 2 — ovale blanc | petit, dense, œil discret | ovales fbm + `tempetes` clairs |
| 3 — brun | cyclone sombre allongé (barge jovienne) | `tempetes` sombres |

Décisions structurantes :

- **Slot 0 = la tache du preset** (`tache_dir/taille/couleur/type` conservés) →
  compatibilité totale des presets nommés.
- `tempetes` devient la **densité d'activation des slots 1..7** (0 = slot 0
  seul), les seuils de fbm actuels disparaissent.
- Structure interne : bruit échantillonné en coordonnées polaires **log-spiralées**
  (`fbm(vec3(ang + swirl(r), r * k, seed))`) au lieu du `sin()` — bras
  irréguliers, plus de vinyle.
- Chaque vortex **advecte le fond** (rotation différentielle bornée, reprise de
  l'anti-autocollant tellurique) et **dérive le long de son jet** : sa vitesse
  de dérive = `u(φ_vortex)` lue dans le profil zonal. Cohérence totale
  structure/mouvement.
- Les vortex se placent de préférence aux latitudes de zones (`b(φ)` élevé pour
  les ovales clairs) / de belts (types sombres) — lu au moment du hash CPU si on
  génère les slots côté Rust (option retenue : **slots générés CPU**, passés en
  uniforms `vec4 vortex[8]` (xyz = dir, w = rayon signé par type) — évite 8×3
  hash par pixel).

---

## 6. Pôles V2

- **Emprise réduite** : engage à \|φ\| ≈ 62° (après la dernière paire de jets du
  profil zonal — la borne est calculée CPU et passée en uniform), pleine à 85°.
- **Transition structurelle, pas un fondu** : sur la zone 55°-70°, `nb_bandes`
  effectif augmente (bandes compressées, lecture directe de `b(φ)` qui se
  resserre par construction) et la turbulence monte (canal `s(φ)`), PUIS le
  régime cyclonique prend le relais.
- **Projection azimutale correcte** : coordonnées `(ρ, θ)` avec
  `ρ = acos(|dot(d,k)|)` — supprime la distorsion Worley de la projection plane.
- **Un seul système polaire** : vortex central + anneau de cyclones Worley
  (config Juno). `poly_cotes` reste et dessine le jet périphérique en polygone
  (Saturne) — même code, le polygone est le contour de l'anneau de cyclones.
- Teintes : `pal[6]/pal[7]`, plus aucune constante.

---

## 6 bis. Cas particuliers réels : couverture et omissions (revue)

Inventaire des phénomènes documentés (Voyager/Cassini/Juno/Hubble), avec la
décision V2. Règle de tri : on garde ce qui est **spatial et statique** (un état
visuel qu'un preset peut porter), on omet ce qui est **temporel** (événements,
cycles pluriannuels — nos planètes n'ont pas d'évolution longue) ou invisible.

### Couvert par la V2

| Phénomène réel | Couvert par |
|---|---|
| Grande Tache Rouge (anticyclone séculaire) | vortex type 0, slot 0 |
| Oval BA « Red Spot Jr. » (anticyclone moyen rougi) | type 0, rayon réduit |
| Ovales blancs épars | type 2 |
| Barges brunes (cyclones allongés de la NEB) | type 3 |
| Grande Tache Sombre (Neptune) + compagnons blancs (« Scooter ») | type 1 |
| Taches sombres d'Uranus (rares) | type 1, `tempetes` faible |
| Hexagone de Saturne — **pôle nord seulement** dans la réalité | `poly_cotes`, déjà nord-only : on garde |
| Amas de cyclones polaires de Jupiter (8 au nord, 5-6 au sud) | § 6 ; le **nombre diffère par hémisphère** (hash par pôle, gratuit) |
| Cyclone polaire unique centré (Saturne, Uranus, Neptune) | § 6, anneau désactivé |
| Festons / hot spots 5 µm (plumes sombres au bord de l'EZ) | turbulence liée à `s(φ)` |
| SEB « affaiblie » (état pâle) | statiquement via `zonal_flou` / amplitude de `b(φ)` |

### Ajouts retenus (peu coûteux, forte identité)

- **Chapelet d'ovales (« string of pearls »)** : N petits ovales blancs
  régulièrement espacés **sur la même latitude de jet** (la STB de Jupiter).
  Un type de slot supplémentaire (type 4) qui dépose ses N instances le long
  d'un jet du profil zonal → très bon rapport identité/coût. → Phase 4.
- **Grande Tache Blanche (Saturne)** : la vraie est un événement (~30 ans) qui
  encercle la planète ; on en garde un **instantané statique rare** — tête de
  tempête brillante + traîne turbulente sur toute la longitude d'un jet.
  Variante rare de `apparence_gazeuse()` + un preset. → Phase 7, priorité basse.

### Omis (assumé)

| Phénomène | Raison |
|---|---|
| Cycle d'apparition/disparition de la GTB, revival de la SEB | événements pluriannuels — pas d'évolution temporelle longue dans le jeu |
| Dérive en latitude et dissolution des taches sombres de Neptune | idem (temporel) ; la position par seed en donne la variété |
| Tempêtes convectives transitoires (Neptune 2017, panaches méthane brillants) | transitoires ; les ovales blancs en tiennent lieu |
| Éclairs côté nuit | gadget émissif, à noter comme idée post-V2 |
| Grande Tache Froide (thermosphère aurorale de Jupiter), mushballs d'ammoniac | invisibles en lumière visible |
| Hexagone aux **deux** pôles | la réalité dit un seul ; en garder un seul renforce le réalisme sans rien coûter |

---

## 7. Cohérences incluses (petites, mais visibles)

- **Thermique nocturne structurée** : `surface()` exporte `band` (via `out`) et
  l'émission module par la vraie structure — la nuit devient le négatif du jour.
- **Brume dépendante de la latitude et de la profondeur** : pondérée par
  `mix(1.0, 0.6, |b(φ) - 0.5| * 2.0)` (les belts percent un peu) + un fbm très
  basse fréquence → voile inégal, plus « atmosphérique », toujours pas de
  scattering (hors périmètre).
- **LOD** : uniform `px_rayon` (rayon apparent en pixels, connu de la galerie et
  du mode objet). `fine`, `flake` et le grain des vortex s'estompent sous
  ~140 px → moins d'aliasing en galerie, gros gain perf là où ça ne se voit pas.

---

## 8. Budget fbm (avant/après)

| Poste | V1 | V2 |
|---|---|---|
| Warp (q1, q2, curl) | 10 | 7 (curl réduit à 2 taps, q1 réutilisé) |
| Bandes (dec1, dec2, turb, swirl, fine) | 5 | 3 (texture zonale remplace dec1 + profil ; fine sous LOD) |
| Marbrures (marb, fil, flake, wisp, ov) | 5 | 2 échantillons partagés, seuils multiples |
| Tempêtes (st, st2) | 2 | 0 (slots vortex) |
| Tache (arms, finsp, ond) | 3 | 2 |
| Pôles / divers | 3 | 2 |
| **Total** | **~28** | **~15** (+1 lecture de texture 1D) |

---

## 9. Hors périmètre V2 (acté, pour mémoire)

- **Anneaux V2** : ombres croisées planète↔anneau (le manque visuel n°1 du
  rendu global), éclairage de la face nuit, anti-crénelage radial — chantier
  dédié suivant. La texture 1D de profil radial d'anneau (même mécanique que le
  profil zonal) en sera la clé : ce doc la prépare mais ne l'implémente pas.
- Scattering/terminateur, halo dépendant de la phase, aurores en ovale magnétique.
- Ombres de lunes sur les bandes.
- Simulation par particules type Gaseous Giganticus (render-to-texture) : le
  profil zonal + curl cheap en donne 80 % pour 5 % du coût.

---

## 9 bis. Décisions actées

- **Cisaillement stylisé, visible** : le glissement entre bandes se perçoit en
  quelques secondes en mode objet. `vit_zonale` reste un uniform global (pas
  par preset) réglé pour ça ; non-réalisme assumé.
- **Non-régression « esprit, pas pixel »** : Jupiter doit rester reconnaissable,
  mais un nouveau look meilleur est assumé. Les captures phase 0 servent à
  juger les diffs, pas à les verrouiller.
- **Pas de migration de sauvegardes** : `band_scale` → `nb_bandes` casse le
  format d'apparence, les systèmes/presets sauvegardés se régénèrent. Aucun
  code de compat en phase 2.

---

## 10. Étapes de travaux

Chaque phase compile, se hot-reload, et se valide sur la galerie avant la
suivante.

### Phase 0 — Filet de non-régression
- [x] Outillage : la touche **C** de la galerie lance désormais une session
      multi-frames qui fait défiler la grille et exporte **toutes** les
      cellules dans un seul dossier `captures/<ts>_seed<N>_<gaz|tell>_<jour|nuit>/`
      (avant : cellules visibles uniquement, un dossier par pression).
- [x] La galerie gazeuse ajoute 3 cellules **« Aleatoire 1..3 »** tirées via
      `apparence_gazeuse()` à seed fixé (mêmes tirages pour une même graine G).
- [ ] Prendre les captures de référence : galerie gazeuse, **C** en éclairage
      jour puis **C** en nuit (graine par défaut, seed 1).

### Phase 1 — Palette paramétrique
- [x] Dérivations côté Rust (`planete/palette.rs`), uniform `gaz_pal[8]`
      (poussé dans `materiau.rs`, déclaré dans le .glsl).
- [x] Remplacer les constantes couleur de la branche gazeuse : zone/ivoire,
      SEB brique, ZTS, chocolat, saumon, ocre, flocons, ovales, équateur,
      traînées, bord/collier/sillage de tache, cellules polaires olive.
      Conservés à dessein (opérations RELATIVES, pas des teintes absolues) :
      festons `base * vec3(0.68,0.76,0.85)` (refroidissement multiplicatif),
      tempêtes sombres `belt * 0.65`, nudges quasi neutres des pôles (+0.03).
- [ ] Retoucher les presets si la validation visuelle le demande.
- Validation : presets jupitériens ≈ mêmes tons ; Neptune/naine brune/carbone
  perdent leurs teintes saumon/ivoire parasites. À vérifier en galerie (C,
  comparer au dossier phase 0).

### Phase 2 — Profil zonal
- [x] `planete/zonal.rs` : génération 256×1 RGBA par seed (gaussiennes, vorticité
      -> type de bande, flou paramétré). Texture liée en 2e unité (`zonal`),
      indexée par sin(latitude) — pas d'asin shader.
- [x] Shader : `b(φ)` remplace dec1 + `jet_profil` (bloc supprimé) ; `s(φ)`
      remplace le proxy `shear` ; la SEB hôte est désormais conditionnée à la
      tache rouge (`tache_type < 0.5`), plus au profil. Thermique nocturne
      structurée par `b(φ)` (anticipé de la phase 6, une ligne).
- [x] Nouveaux params `nb_bandes` (paires de jets), `jets_force`, `zonal_asym`,
      `zonal_flou` ; `band_scale`/`jet_profil` supprimés partout (26 presets
      convertis, génération aléatoire recalibrée). Builders `avec_jets`,
      `avec_zonal_flou`.
- [x] Anti-moucheté (retour phase 1) : grain global `0.96+0.12·fine` ->
      `0.98+0.05·fine`, contribution de `fine` aux bandes 0.16 -> 0.10.
- Validation : nombre de bandes pilotable et lisible, asymétrie N/S visible,
  festons concentrés aux frontières de jets, moucheté résorbé.

### Phase 3 — Rotation différentielle
- [x] Advection `dzn` par `u(φ)` (§ 3.2) : rotation Rodrigues par pixel, angle
      `u(φ)·time·0.025` (stylisé, § 9 bis) ; tout l'échantillonnage de bandes
      passe par `dzn`. Rotation exacte : aucun enroulement cumulatif.
- [x] La grande tache reste ANCRÉE au repère rigide : le flot advecté cisaille
      autour d'elle (physique GRS — le vortex roule entre deux jets opposés).
- [x] Dérive de bruit V1 réduite à une micro-turbulence (`tn = time·0.010`
      dans curl et q2 ; `t` reste pour l'animation interne des vortex).
- Validation : en mode objet, les bandes glissent visiblement les unes contre
  les autres ; pas d'artefact d'enroulement après 10 min.

### Phase 4 — Vortex unifiés
- [x] Génération CPU des slots (`planete/vortex.rs` -> uniforms `vortex[8]` +
      `vortex2[8]`), slot 0 = tache preset ; latitude choisie selon la bande
      (zones pour ovales/chapelets, belts pour sombres/barges, via le profil
      zonal partagé) ; dérive = u(φ) du jet local (le vortex ride son flot).
- [x] Boucle de vortex dans le shader : 5 types (GRS, sombre, ovale blanc,
      barge allongée, chapelet de perles § 6 bis), torsion du fond BORNÉE par
      slot (aspiration anti-vinyle), bord rongé par un bruit partagé, rendu du
      slot dominant. `tache_dir/tache_w/tache_type/tempetes` retirés du shader
      (le CPU les consomme).
- [x] Supprimés : ovales-seuils de fbm, bloc `tempetes` (st/st2, -2 fbm),
      spirale `sin()` de la GRS (bras = fbm pur en coordonnées log-spirales),
      whirlpool à terme croissant (remplacé par la torsion bornée).
- [ ] Reporté phase 5 : nombre de cyclones polaires différent par hémisphère
      (avec la refonte des pôles).
- Validation : GRS avec bras irréguliers ; ovales blancs avec vraie rotation ;
  une géante à `tempetes` élevé montre des vortex distincts, pas une bouillie.

### Phase 5 — Pôles V2
- [x] Emprise recalculée : uniform `pole_lat` = sin(latitude) après la dernière
      paire de jets (calculé dans `generer_zonal`) — la calotte engage vers
      62-67° au lieu de 19° (!), les bandes montent enfin haut.
- [x] UN SEUL système polaire (remplace les 3 blocs V1 : fond feutré à 95 %,
      hexagone tamponné, worley cyclones) : projection azimutale correcte
      (ρ = angle au pôle, θ = longitude sur dzn), fond feutré Worley + grain
      partagé, anneau de cyclones config Juno (N différent par hémisphère,
      5-8, dérive lente opposée, bras fbm log-spiral + œil), vortex central
      sombre à bras fbm.
- [x] Hexagone désautocollanté : rendu DANS le régime polaire (teintes du
      pôle), pôle nord seulement, bord ondulé par le bruit + eddies Worley,
      rotation lente. Le `sin()` du vortex central V1 a disparu.
- Validation : les bandes montent jusqu'à ~62° ; Saturne garde son hexagone
  (vivant) ; vue pole-on (Uranus) intéressante au lieu d'une calotte plate ;
  Jupiter : anneau de cyclones visible en vue polaire, différent N/S.

### Phase 6 — Cohérences + budget
- [x] Thermique structurée par `b(φ)` (fait dès la phase 2).
- [x] Brume inégale : fbm très basse fréquence + les belts percent le voile
      (`0.75 + 0.25·b(φ)`) — les sub-Neptunes sont voilées, plus délavées.
- [x] LOD `px_rayon` : rayon apparent calculé CPU (distance caméra × hauteur
      de viewport — la galerie déclare sa hauteur de cellule via
      `set_viewport_h`) ; sous ~120 px, `fine`/`fil`/`flake`/`finsp` fondent
      vers leur valeur neutre et leurs fbm ne sont plus évalués.
- [x] Budget : ~17 fbm plein détail (~14 sous LOD galerie) contre ~28 en V1 ;
      2 worley + 2-3 fbm supplémentaires dans la calotte polaire seulement.
- [x] Recalibrage Neptune (anticipé de la phase 7, retour visuel) : `c3` bleu
      moyen saturé au lieu de quasi-blanc, jets 0.55, `zonal_flou` 0.3 —
      corps bleu à bandes discrètes au lieu d'une « Jupiter blanche inversée ».
- Validation : côté nuit d'une classe V = bandes en négatif ; galerie fluide
  et moins scintillante ; Neptune bleue ; diff des captures phase 0 assumé.

### Phase 7 — Recalibrage du catalogue
- [x] Presets : les ~27 ont tous leur structure (`avec_jets`/`avec_zonal_flou`
      par archétype — Jupiter 1.0, Uranus 0.15 + flou, classes I/II douces et
      voilées, IV/V et naines brunes contrastées, Neptune recalibrée bleue…).
- [x] `apparence_gazeuse()` : 4 archétypes STRUCTURELS (classique 30 % / glace
      28 % / chaude 30 % / lisse « classe III » 12 %) — chacun tire ses
      (nb_bandes, jets, flou, warp, tempêtes) ; les glaces gardent un c3
      saturé (leçon Neptune), les lisses n'ont ni vortex ni tache.
- [x] Variante rare « Grande Tache Blanche » statique (§ 6 bis) :
      `avec_tache_blanche` (tache_type 2 → slot 0 ovale blanc massif) +
      tempêtes max ; preset rare « Tempete planetaire (GTB) » + ~6 % des
      géantes classiques aléatoires.
- [x] Mise à jour `GAZEUSES_BUCKETLIST.md` (bilan V2, items cochés) et
      `NOISES_GAZEUSES.md` (post-scriptum V2 : ce qui a servi, ce qui a changé).

---

### Phase 4 bis — Passe anti-autocollant sur la tache (retour visuel)

Verdict de validation : la tache faisait encore autocollant. Causes trouvées et
corrigées :

- **Les bandes ne se courbaient pas autour d'elle** : la torsion des vortex
  s'appliquait aux textures (via `dd`) mais la lecture du profil zonal `b(φ)`
  se faisait à la latitude du PIXEL. Corrigé : `zp` est lu à `dot(dd, k)`
  (latitude tordue) -> les bandes se déforment visiblement autour des vortex.
- **Collier-halo** : l'anneau crème était uniforme (signature d'autocollant).
  Corrigé : modulé par le champ de flot `ov` -> chapelet de nuages irrégulier.
  Idem pour le sillage (traîne déchiquetée).
- **Intérieur découplé** : la tache ignorait la luminance des bandes locales.
  Corrigé : `spotc *= 0.86 + 0.28·bandc` + opacité dégressive vers le bord
  -> elle appartient à sa ceinture.
- Torsion renforcée (portée 2.4 -> 3.0 rayons, force 1.2 -> 1.6) et bord plus
  rongé (0.34 -> 0.45).

---

## 11 bis. État final

Les 7 phases sont livrées (juillet 2026). Périmètre tenu : surface seule.
Chantiers suivants notés : **anneaux V2** (ombres croisées — § 9), éclairage/
terminateur, ombres de lunes, éclairs nocturnes (§ 6 bis).

---

## 11. Références

- Profils de vents zonaux de Jupiter/Saturne (Voyager/Cassini) — alternance de
  jets, asymétries N/S : la base du § 3.
- I. Quilez, *domain warping* — conservé pour la chair.
- Gaseous Giganticus (S. M. Birrell) — l'advection par champ sans divergence,
  approchée ici par transport zonal réel + curl cheap.
- Juno (JunoCam) : configuration des cyclones polaires en anneau (§ 6).
- `conception_planete_v2.md` — le précédent « squelette CPU / chair shader »
  qui a fait ses preuves sur les telluriques.
