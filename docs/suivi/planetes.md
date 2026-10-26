# Suivi — Planètes telluriques

> Fusion de deux anciens documents : le bilan de la refonte v2 du rendu
> (`bilan_travaux.md`) et le catalogue de presets (`PLANET_BUCKETLIST.md`).
> Conception : [`docs/conception/planetes.md`](../conception/planetes.md).

---

## Chantier planètes : le type et l'habitabilité (branche `chantier-planetes`, 2026-10-27)

*« Rajouter un indicateur de type et d'habitabilité sur les planètes gazeuses et
rocheuses »*, huit types, chacun son logo (`assets/sprites/planet_type_*.png`,
16 × 16, fournis par l'utilisateur ; noms régularisés — une espace et une
coquille corrigées).

| Type | Règle (`planete::classe::classer`) |
|---|---|
| Naine brune | gazeuse marquée `naine_brune` (catalogue : nom en « Naine brune ») |
| Géante gazeuse annelée | gazeuse à anneaux |
| Géante gazeuse | les autres gazeuses |
| Inhabitable | rocheuse, lave (au-delà de 600 K à la genèse) |
| Hostile | rocheuse sous un voile épais (étuve, comme Vénus) |
| Dangereuse | glacée, ou banquise quasi globale (gelée) |
| Habitable | océans (eau ≥ 0,4) **et** végétation (≥ 0,4) |
| Colonisable | tout le reste : désert, toundra, roche nue |

**Une seule règle, lue sur l'apparence** : la genèse la fabrique d'après la
température d'équilibre, donc elle vaut partout — système généré, galerie (sans
étoile ni distance), lunes. Les comètes n'en ont pas (*« elles auront leur
logo »*). Affiché dans la **fiche de la skymap V2** (icône + libellé coloré,
sous la catégorie) et dans la **galerie des planètes** (icône devant le nom).
`Astre::classe_planete`, `Systeme::classe_planete_de`, `Fiche::classe`,
`sprites::dessiner_classe`. La ligne de zone de la fiche passe de
« HABITABLE » à **« ZONE TEMPEREE »** : elle ne dit que la distance à l'étoile.

⚠️ Les deux peuvent se contredire : une planète du système G de la graine 1
est **habitable** (océans, végétation) mais **trop chaude** selon la zone — la
genèse tire son climat d'une température d'équilibre, la zone d'une
luminosité ; deux modèles, à réconcilier avec la refonte de la genèse.

Gardes : `chaque_monde_tombe_dans_son_type` (red-check : lave ignorée),
`les_gazeuses_se_rangent`, `chaque_type_a_son_icone_sur_le_disque`, et le
garde du dossier des sprites étendu aux types. Captures : `GALERIE`,
`SYSTEME=<type> FICHE=<rang>` (nouveaux, pour les captures).

## Partie A — Bilan des travaux (rendu v2)


> Reprise du chantier ailleurs : ce document dit CE QUI A ÉTÉ FAIT, OÙ, et
> COMMENT s'en servir. Le POURQUOI (conception, algorithmes, décisions) est
> dans [`conception/planetes.md`](../conception/planetes.md) Partie A —
> les renvois « § n » pointent dedans.

### 1. Résumé exécutif

Le rendu des planètes telluriques est passé d'un shader 100 % procédural par
pixel (champs de bruit indépendants, rivières incohérentes) à un pipeline
hybride : **géographie précalculée sur CPU** (grille cube-sphere, érosion,
hydrologie, volcanisme → atlas de textures) + **habillage shader** (biomes,
détail fin, éclairage, couches émissives). Les 8 étapes prévues en conception
sont **toutes implémentées et testées** (11 tests unitaires verts). La
génération est asynchrone (pas de gel d'UI), déterministe par graine, et
outillée : bench intégré, captures de non-régression, filtre pixel.

### 2. Fichiers touchés et nature des changements

| Fichier | État | Contenu |
|---|---|---|
| `src/planete/terrain.rs` | **NOUVEAU** (~1000 lignes + tests) | Tout le pipeline CPU : grille cube-sphere équi-angulaire (mapping texel↔sphère, voisinage par re-projection 3D — pas de table d'arêtes), bruit (port du GLSL), érosion thermique + hydraulique (gouttes 3D), volcans (cônes/caldeiras/chaleur), hydrologie (priority-flood → drainage gratuit → flux log → humidité par rang), bake atlas RGBA8 3×2 avec gouttière, RNG déterministe (SplitMix64), budget de jobs concurrents, stats temps réel, bench complet. |
| `src/shaders/planete.frag.glsl` | refonte branche tellurique | Lecture atlas (`dir_vers_atlas`, altitude 16 bits R+G), niveau de mer quantile, rivières/lacs par canal flux, **régime hydrologique** (pas d'atmo → pas d'eau ; sec → salines/oueds ; lave), glace banquise/terrestre avec veines gelées, biolum qui suit la géographie, coulées de lave émissives pulsantes, normale perturbée, ombres de nuages, température locale (lat+altitude). Branches gazeuse/glacée intactes. |
| `src/planete/materiau.rs` | modifié | Texture `terrain` dans le material + uniforms `niveau_mer`/`atlas_n` ; texture 1×1 de secours (placeholder + corps non telluriques). |
| `src/planete/mod.rs` | modifié | `Planete` porte `terrain_tex`/`terrain_job` : génération **asynchrone au premier draw** (thread + budget global 2 jobs), upload GPU à la fin du job. Accesseurs `terrain_pret()`, `apparence()`. |
| `src/ecran/galerie.rs` | modifié | Scroll doux (cible + lissage exponentiel + accrochage pixel), **P** filtre pixel (render target demi-résolution, plus proche voisin, textes nets), **C** captures PNG de non-régression, **B** bench, overlay FPS/génération/état du bench. |
| `Cargo.toml` | modifié | `[profile.dev] opt-level = 1` (le bruit CPU est inutilisable en debug pur). |
| [`conception/planetes.md`](../conception/planetes.md) Partie A | vivant | Conception + état d'avancement par étape (§ 8) + revue du catalogue (§ 12) + non-régression (§ 13). |

Zéro dépendance ajoutée (threads std, pas de rayon ; RNG maison).

### 3. Mode d'emploi (galerie des telluriques)

- Molette : défilement (doux). `G` : nouvelle graine. `R` : hot-reload shaders.
- `P` : filtre **pixel** ON/OFF (préfigure le style final).
- `C` : **capture** les cellules visibles → `captures/<epoch>_seed<N>/<preset>.png`.
  Workflow non-régression : capturer avant modif, après, comparer les dossiers.
  Si les PNG sortent à l'envers : constante `RENVERSEE` dans `galerie.rs`.
- `B` : **bench** en tâche de fond (telluriques seulement, un seul à la fois).
  Progression dans l'overlay du bas ; rapport → `bench_terrain.txt` (chemin
  absolu affiché à la fin ; fichier écrit dès la fin de la passe 256²).
  Toujours bencher en `cargo run --release`.
- Overlay bas d'écran : FPS, état pixel, nb de terrains générés, dernier/moyen.
- Une tellurique s'affiche d'abord en **placeholder uni** (~0,5-1 s) le temps
  que son terrain se génère en fond — comportement normal, pas un bug.

### 4. Leviers de réglage (tous dans `terrain.rs` sauf mention)

| Levier | Valeur | Effet |
|---|---|---|
| `QUALITE` (dans `params_depuis_apparence`) | 0.15 | Gouttes d'érosion par texel. Levier perf/beauté n° 1. Si relief trop lissé → 0.20. |
| `N_ATLAS` | 256 | Résolution par face. 512 = ×4 temps et mémoire (mesuré 7-10 s/planète : pas viable sans autre optimisation). |
| `MAX_JOBS` | 2 | Générations simultanées (galerie). |
| `erosion` / `pas_max` / `evaporation` | 0.38 / 48 / 0.02 | Taux et durée de vie des gouttes (calibrés avec QUALITE 0.15). |
| `debit` volcans (dans `generer_chrono`) | 0.5 % des texels | Force des coulées. |
| `PIX` (`galerie.rs`) | 2 | Échelle du filtre pixel (aussi dans `rendu.rs` pour les autres vues). |
| Seuil rivières (shader) | `mix(0.78, 0.50, rivieres)` | Densité du réseau visible, piloté par le preset. |

### 5. Performances

- **Bench utilisateur 2026-07-02** (8 cœurs, release, avant optimisation) :
  médiane 1061 ms, max 2040 ms, érosion ~62 % du coût, bruit ~430 ms non scalé.
- **Optimisations appliquées ensuite** (§ 8.7) : bruit en bandes sur tous les
  cœurs + warp 3 octaves ; érosion −40 % de gouttes compensée + moitié moins
  de lectures par pas. VM 2 cœurs : 1,54 s → 0,85 s. **À re-bencher** (B) —
  attendu ~450-550 ms de médiane sur 8 cœurs.
- Rendu GPU : la lecture d'atlas est MOINS chère que l'ancien calcul (5+
  octaves de fbm remplacées par ~4 lectures de texture par pixel).
- Mémoire GPU : ~1,5 Mo/planète tellurique (atlas 774×516 RGBA8).

### 6. Tests et validation

- `cargo test` : 11 tests dans `terrain.rs` — aller-retour texel↔sphère
  exhaustif, coutures réciproques, quantile de mer, gouttière d'atlas,
  stabilité érosion (lissage garanti, pas de NaN), lacs/flux d'hydrologie,
  caldeiras/coulées de volcans.
- Le fragment shader passe `glslangValidator -S frag` (GLSL ES 100 / WebGL1).
- Validation visuelle : dossiers `captures/` + presets témoins — Terre
  (rivières, déserts continentaux), Lune/Salines (AUCUNE eau), Cryovolcan
  (volcans + anneaux de fonte + veines gelées), Io/Lave (coulées émissives
  nocturnes), Bioluminescent (réseaux de lumière la nuit), Boule de neige
  (banquise ≠ glace terrestre).

### 7. Pièges connus / vigilance

- **Un incident de troncature de fichier** s'est produit pendant les travaux
  (`src/planete/mod.rs` coupé en fin de fichier, réparé depuis git). Avant de
  committer : `git diff --stat` et vérifier qu'aucun fichier ne finit au
  milieu d'une fonction.
- Les rivières ne se voient que si le preset a `rivieres > 0` (contrôle
  artistique conservé) — sauf régime lave, qui trace ses coulées seul.
- Le canal humidité est un **rang** 0..1 (pas une valeur physique) : tout
  seuillage doit penser « fraction de la planète », cf. § 11.2 bis.
- `apparence_tellurique()` (génération aléatoire des systèmes) n'a PAS été
  retouchée : les seuils climatiques pilotent déjà le pipeline via `eau`,
  `lave`, `cryo`, `calotte`, `dunes`, `voile`.
- La graine du terrain vient de `app.seed` (f32) : deux presets à graine
  identique ont la même géographie (voulu pour la galerie, touche G pour
  varier).

#### Le hash CPU/GPU : une constante dupliquée que clippy veut « corriger »

Relevé le **2026-08-02**, en débloquant `cargo clippy` (qui ne compilait plus).

`terrain::hash` est un **port exact** du `hash` des shaders — c'est ce qui fait
que la géographie précalculée sur CPU correspond à ce que le GPU dessine. Le
multiplicateur `0.318_309_9` ressemble à `1/π`, et
`clippy::approx_constant` — **refusé par défaut**, donc erreur dure et non
avertissement — exigeait de le remplacer par `f32::consts::FRAC_1_PI`.

**Suivre ce conseil aurait introduit un vrai défaut.** Les deux valeurs
diffèrent d'exactement **1 ULP** (`3ea2f984` contre `3ea2f983`) : anodin
n'importe où ailleurs, mais `hash` finit par un `fract`, qui amplifie le
moindre écart en un bruit **entièrement différent**. La géographie CPU aurait
cessé de correspondre au shader, sans que rien ne le signale — et le symptôme
(un relief qui ne colle plus à sa texture) n'aurait pas désigné sa cause.

Corrigé en nommant la constante (`DECORRELATION_HASH`) et en portant l'`allow`
sur elle, avec l'explication : ce n'est pas une approximation de `1/π`, c'est
une constante arbitraire de hachage dont la seule propriété qui compte est
d'être **identique à celle des shaders**.

⚠️ **Dette assumée** : la même valeur vit dans quatre fichiers —
`planete/terrain.rs` et les trois shaders `planete.frag.glsl`,
`panache.frag.glsl`, `soleil.frag.glsl` (`0.3183099`) — et **rien ne les tient
d'accord**. C'est « une source par fait » violé, mais Rust et GLSL ne partagent
aucun texte : il faudrait générer les shaders, ou un test qui compare les deux
implémentations sur un échantillon (impossible sans contexte GPU en test). En
attendant, le commentaire du code est le seul garde-fou. **Toucher à cette
valeur d'un côté oblige à la toucher des trois autres.**

### 8. Reste à faire / prochains chantiers

1. **Re-bench post-optimisation** (B) et calibrage visuel de `QUALITE`
   (captures avant/après).
2. **Catalogue v2** : re-passer les ~130 presets sous le nouveau pipeline
   (l'utilisateur a signalé que certains presets méritent des retouches
   maintenant que l'eau/glace/veg suivent la géographie).
3. **Galerie d'édition des presets** (objectif annoncé) : remplacer la vue
   objet isolée par un éditeur temps réel des paramètres d'`Apparence` avec
   régénération à la volée — le pipeline est prêt (déterministe, ~0,5 s par
   itération, async).
4. **Style pixel final** : le filtre existe (P) ; reste à décider s'il devient
   permanent et à quel `PIX_SCALE`, et si le filtrage de l'atlas doit passer
   en Nearest pour l'esthétique.
5. Optionnel selon besoin : érosion parallèle (si le re-bench déçoit),
   512²/face pour la vue rapprochée, lunes riches, génération aléatoire
   branchée sur le catalogue (§ 8 point 5 de la première analyse).

---

## Partie B — Catalogue (bucket list)


Catalogue des planètes telluriques. Voir [`conception/planetes.md`](../conception/planetes.md)
Partie B pour le modèle paramétrique.
`[x]` = un preset existe dans la **Galerie** (ou est généré aléatoirement).

Légende : `[ ]` à faire · `[x]` fait · `(R)` rare · `(F)` flavor (rendu comme la famille parente)

---

### Bilan — CATALOGUE COMPLET ✅

**~126 presets tellurique** en galerie (3 passes). Toutes les familles de Planetary
Diversity sont couvertes, plus les non-habitables et les cas spéciaux.

**Axes / features implémentés** (uniforms, hot-reload R) : altitude par domain warping +
**étagement de biome** (prairie → forêt → roche → neige), `eau`+`eau_motif`, `veg`,
`rivieres` (+`riv_lave`), `grad_lat`, `calotte` (glace fracturée, bord déchiqueté), `nuages`
**2 couches** + `nuages_type` (classique/tempête/cyclone), `relief` (montagnes), `dunes`,
`mesa`, `pics`, `recifs`, `basalt`, `crateres`, `voile` (Vénus/Titan), `eyeball` (marée),
`cryo`, `biolum`, `lave`, **reflet spéculaire océan**, `seed` (géographie unique).

**Atmosphère** (2026-08-08) : elle n'est plus peinte par le shader du corps. Passe
séparée `shaders/atmosphere.frag.glsl`, additive et sans écriture de profondeur, qui
**déborde de la silhouette** — coquille creuse entre le sol et `disc`, opacité par
longueur de corde, crépuscule au terminateur, embrasement à contre-jour. Teinte prise
sur `atmo` **et** `voile`. Voir `conception/planetes.md` § 14.

**Densité séparée de la teinte** (2026-08-09) : `atmo` est une teinte unitaire,
`atmo_densite` porte la quantité d'air. Les deux tenaient dans un seul `Vec3`, si bien
qu'un halo **terne** ne pouvait s'écrire que comme un halo **mince** — Vénus (92 bars)
sortait deux fois plus fine que la Terre. Quatre entrées corrigées : Vénus 0,28 → 1,45,
Titan 0,31 → 1,20, Hycean 0,59 → 1,30, Ash 0 → 0,32. Le reste du catalogue est
**inchangé au bit près** (`avec_atmo` fait la scission lui-même).

**Reste (optionnel, non bloquant)** :

- [x] Rotation à **vitesse variable** par planète : `ROTATIONS_JOURS` (vraies périodes
      sidérales, Terre = référence), rétrogrades comprises (Vénus, Uranus). Bornée en haut
      et en bas — à l'échelle littérale Vénus paraîtrait figée, ce qui se lit comme un bug.
- [x] **Couche de nuages séparée** : repère propre qui super-tourne (×1,35 + dérive), donc
      les nuages glissent au-dessus du relief au lieu d'y être collés ; ombre portée
      échantillonnée dans le même repère. Conditionnée à la présence d'air (`atmo_densite`/`voile`),
      et hors de portée des géantes gazeuses (leur branche rend la main plus haut).
- [x] **Reflet spéculaire de l'océan au terminateur** : il était éteint par `diff` là où il
      est le plus fort. Extinction géométrique + renfort de Fresnel au rasant.
- [x] **Ombre de l'anneau sur la planète** (la réciproque existait seule) : intersection
      exacte du rayon lumineux avec le plan de l'anneau, cas rasant écarté.
- [ ] Faire **piocher le générateur** (skymap/objet) dans le catalogue de presets nommés.
- [ ] Sélecteur de variante en mode Objet.
- [ ] Passe **planètes gazeuses** (Hot/Cold/Cloudless Gas Giant) — type séparé.
- [ ] Perf : mutualiser les fbm si le plein écran rame.

---

# Partie A — Archétypes (tous faits)

- [x] Humide : Continentale, Monde-océan, Tropicale, Marécage, Récif
- [x] Sec : Désert/dunes, Aride/mesa/badlands, Oasis, Savane/steppe, Sable ferreux
- [x] Froid : Toundra, Arctique, Boule de neige, Cryovolcanique, Alpin
- [x] Extrêmes : Lave, Étuve/Vénus, Eyeball (humide/sec/gelé)
- [x] Exotiques : Fer/Mercure, Carbone/Diamant, Soufre/Io, Hydrocarbures/Titan
- [x] Transverses : nuages (2 couches + types), relief + étagement, calottes texturées,
      lumières de villes, **reflet spéculaire océan**

---

# Partie B — Catalogue (toutes les variantes ont un preset)

### Humide › Continental
- [x] Megaflora (R) · Retinal · Forest · Petrified (R) · Lake · Tepid · Mushroom · Sakura ·
  Carotene · Moss · Marsh

### Humide › Océan
- [x] Reef (R) · Cascadian · Swamp · Archipelago (R) · Crag · Fog · Kelp · Columnar ·
  Barnacle · Pink Algae · Tidepool
- [x] **Ocean pur** (aucune terre émergée, eau = 1.0) · **Hycean** (océan global sous
  atmosphère de vapeur épaisse) · **Ocean de magma** (mer de roche fondue, émissif la nuit)

### Humide › Tropical
- [x] Geothermal (R) · Atoll · Mangrove · Bioluminescent (R) · Tepui · Cenote · Fungal ·
  Lilypad · Thunderstorm · Aerial · Obsidian

### Sec › Désert
- [x] Salt (R) · Oasis · Outback · Aquifer (R) · Dune · Coastal · Fungi · Cactus ·
  Dust Storm · Ironsand · Sodalite

### Sec › Aride
- [x] Coral (R) · Mesa · Mediterranean · Primal (R) · Fog Desert · Badlands · Succulent ·
  Amethyst · Superbloom · Striped · Opal

### Sec › Savane
- [x] Baobab (R) · Scrubland · Steppe · Geoglyph (R,F) · Pampa · Veldt · Acacia · Heath ·
  Bushveld · Termite (F) · Amber

### Froid › Arctique
- [x] Storm (R) · Cold Desert · Antarctic · Iceberg (R) · Glacial · Aeolian · Ice Spike ·
  Ice Dunes · Supraglacial · Crevasse · Ferrosprings

### Froid › Toundra
- [x] Cryflora (R) · Bog · Mud · Lichen (R) · Mycelium* · Basalt · Tuya · Treeline ·
  Travertine · Cryovolcano · Peatland   (*Mycelium ≈ Fungal/Bog)

### Froid › Alpin
- [x] Glaciovolcanic (R) · Boreal · Highland · Lanthanide (R) · Snow · Dune Forest · Fjord ·
  Taiga · Ravine · Blossom · Craton

### Volcanique
- [x] Ash

### Gaia & Superhabitables
- [x] Dry Gaia · Cold Gaia · Wet/Dry/Cold Superhabitable

### Verrouillées par marée & Mondes-grottes
- [x] Wet/Dry/Cold Tidally Locked (Eyeball) · Wet/Dry/Cold Cave (F)

### Non-habitables
- [x] Lune · Diamant · Chthonien · Subglaciaire · Vénus · Titan · Fer/Mercure · Carbone ·
  Ash · Io (soufre)

---

## Passe de cohérence du catalogue (2026-08-05)

Partie de « Mercure a de la glace ». Trois trouvailles, détaillées dans
la Partie C de ce fichier, § « Cohérence du catalogue de planètes » :

1. **`calotte = 1` ne voulait pas dire « aucune banquise ».** Le seuil
   `smoothstep(calotte, calotte + 0.05, bord)` reçoit un `bord` perturbé de
   ±0,30 par du bruit, et `froid` vaut déjà 1 au pôle. Tous les presets qui
   déclarent 1 pour dire « pas de glace » — Mercure, la Lune, Carbone, Vénus —
   sortaient coiffés de blanc. C'est désormais un sentinelle exact.
2. **Les lumières de ville s'allumaient partout.** `villes` vaut 1 par défaut et
   le shader n'excluait que la lave et les mondes voilés : les cailloux morts
   étaient colonisés. La condition « ni air ni eau ⇒ pas de villes » vit
   maintenant dans la donnée, où elle se teste.
3. **Un test à moi était faux** : « pas de rivières sans eau » mordait sur
   `Crevasse`, dont les rivières sont des coulées de **lave** (`riv_lave`). La
   règle correcte est « une rivière coule d'eau **ou de lave** ».

**Ce que la passe n'a PAS trouvé** : la donnée du catalogue était déjà cohérente
sur tous les invariants testés. Les deux vrais défauts étaient dans le rendu, pas
dans les presets — il n'y avait donc rien à « repasser » preset par preset.

---

# Partie C — Atmosphères et finitions (chantiers P.1 et P.4)

> Extrait de [`STATE.md`](../../STATE.md) le 2026-08-10.

## P.1 — Atmosphère : fait (2026-08-08)

**Le diagnostic.** `planete.frag.glsl` jetait tout ce qui dépasse la sphère
(`if (rr > 1.0) discard;`), alors que l'impostor est un disque de `DISC` :
**la marge autour de chaque corps ne servait à rien**. Ce que le shader
appelait « halo atmosphérique » n'était qu'un `pow(1 - z, 3)` appliqué **sur la
surface** — un limbe qui s'éclaircit, et rien au-delà de la silhouette. C'est
pourtant le débordement qui fait lire une atmosphère.

**Ce qui a été fait.** Une passe séparée, `src/shaders/atmosphere.frag.glsl`,
dessinée sur le même quad juste après le corps, additive et **sans écriture de
profondeur** (la moitié avant d'un anneau doit continuer à passer devant).

Le modèle est une coquille creuse entre le sol (rayon 1) et le sommet de l'air
(rayon `disc`). Pour un rayon visuel à la distance `b` du centre on mesure la
**corde** parcourue dans la coquille — la quantité d'air traversée :

| Où | Corde | Effet |
|---|---|---|
| `b < 1` (devant le corps) | `√(disc²−b²) − √(1−b²)` — le sol coupe le rayon | voile ténu sur la surface |
| `b > 1` (à côté du corps) | `2·√(disc²−b²)` — la coquille est traversée deux fois | halo qui déborde, ~98 % au limbe |

Le **facteur deux au franchissement de la silhouette n'est pas un artefact** :
c'est exactement ce qui fait du limbe un liseré fin et lumineux plutôt qu'un
dégradé mou. S'y ajoutent la raréfaction en altitude (hauteur d'échelle = 35 %
de la coquille), l'opacité Beer-Lambert, un terminateur qui **déborde sur la
nuit** (crépuscule : la haute couche voit encore l'étoile), l'éclairage
multi-source comme le corps, et la **diffusion vers l'avant** — à contre-jour
l'arc du limbe s'embrase.

Densité et teinte viennent de `atmo` **et** de `voile` : le halo de Vénus est
jaune et épais, pas bleu. L'ancien `rim` a été **supprimé** du corps — deux
modèles pour la même chose se seraient additionnés.

**Trois duplications tuées au passage** (la même erreur que les masses, la
corde du volet, le ventre du Starship) :

- `r * 1.05` était écrit en dur dans `mod.rs` **à côté** de `DISC = 1.05` privé
  dans `materiau.rs` : le quad et l'uniform `disc` pouvaient diverger en silence.
  `DISC` est maintenant `pub(super)` et lu aux deux endroits — ce qui a permis
  d'épaissir la coquille en changeant **un seul chiffre**.
- la formule `px_rayon` était recopiée dans `appliquer_uniforms` ; elle est
  désormais une fonction, réutilisée par le seuil de visibilité.
- les listes d'uniforms des deux materials sont devenues des fonctions
  (`uniforms_corps`, `uniforms_atmo`), ce qui **rend testable** leur
  correspondance avec le `.glsl`.

**Coût maîtrisé** : la passe est sautée si le corps n'a pas d'air, ou si la
coquille tient dans moins de `ATMO_PX_MIN = 0.75` px — en vue système, des
dizaines de corps de quelques pixels ne paient rien.

**Tests** (6 ajoutés, tous red-checkés) : correspondance uniforms ↔ GLSL pour
les **deux** shaders (un uniform lu par le shader mais non déclaré vaut
silencieusement zéro : aucun message, l'effet disparaît), marge `DISC`,
les deux branches du seuil de visibilité, et `px_rayon` contre le viewport et
la distance.

**Épaisseur portée de 5 % à 10 % (`DISC = 1.10`)** après avoir vu le rendu : à
5 % le liseré était juste mais illisible. Profil réel, aux constantes du shader
(`CONTRASTE = 1.6`, `K = 4`, `ECHELLE = 0.35`) :

| `b` | 0 | 0,9 | 0,99 | **1,0** | 1,02 | 1,04 | 1,06 | 1,08 |
|---|---|---|---|---|---|---|---|---|
| Terre (dens. 0,97) | 11 % | 28 % | 55 % | **98 %** | 84 % | 57 % | 29 % | 11 % |
| Mars avant (dens. 0,094) | 1 % | 3 % | 7 % | **31 %** | 16 % | 8 % | 3 % | 1 % |
| Mars après (dens. 0,378) | 4 % | 12 % | 26 % | **78 %** | 45 % | 28 % | 12 % | 4 % |

⚠️ Deux choses que ce tableau dit et qu'il faut garder en tête :

- le voile de 11 % au **centre** du disque est la contrepartie physique d'une
  atmosphère deux fois plus épaisse (on regarde à travers de l'air même à la
  verticale). S'il délave, c'est `CONTRASTE` qu'il faut monter, pas `DISC` ;
- **l'épaisseur n'éclaircit pas** : elle élargit la bande. Un corps à air ténu
  reste pâle parce que son plafond est sa **densité**. C'est ce qui a fait
  ressortir le cas Mars (corrigé ci-dessous) — les deux réglages sont
  indépendants : `DISC` pour la largeur, `atmo`/`voile` pour l'éclat.

⚠️ **Reste à valider à l'œil.** Un shader ne se compile qu'au lancement, et
aucun validateur GLSL n'est installé ici. `ECHELLE`, `CONTRASTE` et `K` sont des
constantes du `.glsl` : rechargeables à chaud (touche R) sans recompiler.
`DISC` est du Rust — le changer demande une recompilation.

## P.1 bis — Le catalogue n'a pas été retouché, et `atmo` a changé de sens

Recensement des **161 entrées** (132 telluriques + 29 gazeuses) fait le
2026-08-08, après la coquille. Le catalogue n'est pas un continuum mais une
**échelle à 4 niveaux**, trois constantes partagées couvrant les telluriques :

| `\|atmo\|` | monde | densité obtenue |
|---|---|---|
| 1.074 (`bleu`) | 46 humides | 0,97 |
| 0.283 (`voile`) | 41 glacés | 0,25 |
| 0.141 (`sec`) | 33 secs | 0,13 |
| 0.000 | 9 sans air | aucune coquille |

Plus ~7 valeurs propres (Vénus 0.276, Io 0.248, Titan 0.313, Amber 0.330, magma
0.398, Hycean 0.587). Gazeuses : 0.037 à 0.527, **aucune à zéro**.

**Première correction : Mars** (2026-08-08, vue à l'écran). Elle valait `* 0.10`,
soit `|atmo| = 0.105` — **la plus faible du catalogue hors zéro, sous même la
constante des mondes secs** — et sortait sans atmosphère visible. Ce chiffre
réglait l'ancien halo, peint *sur* la surface : le monter aurait délavé le
contraste clair/sombre (oxyde de fer contre basalte) qui fait lire la planète.
La coquille, elle, vit surtout **dehors** — au centre du disque elle ne laisse
que ~4 %. La raison de le garder bas a donc disparu avec l'ancien modèle.

Portée à `* 0.40` (densité 0,378). Ce n'est **pas** le rapport des pressions :
Mars fait 0,6 % de la Terre, à ce compte elle serait nue. C'est ce que Mars
*montre* — une brume chargée de poussière, franchement visible au limbe depuis
l'orbite, dont la hauteur d'échelle vaut **2,4 fois celle de la Terre rapportée
au rayon**. Mince en densité, haute en géométrie. Test
`mars_a_une_brume_visible_mais_tenue` : il **encadre** Mars entre « Badlands »
(monde sec générique) et « Terre », deux entrées écrites indépendamment d'elle,
plutôt que de recopier la formule de densité qui vit dans le GLSL.

**`atmo` voulait dire « teinte × éclat du liseré » — un bouton de rendu.** Il
pilote maintenant la densité d'une vraie coquille. Les valeurs ont été choisies
sous l'ancien sens.

**Vénus le prouve** : `|atmo| = 0.276`, parmi les plus basses du catalogue, pour
l'atmosphère la plus épaisse du système solaire. C'était **juste** avant (un
liseré fort par-dessus un voile déjà opaque rendait mal, donc on l'a baissé) ; ça
ne l'est plus. Elle ne s'en sort aujourd'hui que parce que la densité intègre
`voile` (0,25 + 0,74 = 0,99). Titan pareil.

**Celle que le voile ne rattrape pas : Hycean.** Océan global sous une enveloppe
épaisse de vapeur (modelé sur K2-18b), elle tombe à 0,71 — **sous la Terre à
0,97**. Elle devrait être la plus dense du catalogue.

Des 9 zéros, quatre sont justes par définition (Lune, Fer/Mercure, Diamant,
Chthonien — « atmosphère arrachée » est le sens même du mot). **Ash
(volcanique)** est douteuse : des cendres en suspension supposent de l'air.
Primal et Lave se discutent.

⚠️ **Ne rien retoucher avant d'avoir regardé.** Ces valeurs réglaient un autre
effet ; les refaire au raisonnement serait deviner deux fois de suite.

Le défaut de structure, si le rendu le confirme : **un seul nombre porte deux
sens** — l'éclat de la teinte et la quantité d'air. Vénus demande *sombre et
épais*, ce que `atmo` seul ne sait pas dire.

## P.1 ter — La scission `atmo` / `atmo_densite` (2026-08-09)

Faite. `Apparence::atmo` est désormais une **teinte unitaire** ; `atmo_densite`
porte seule la quantité d'air. Le shader ne calcule plus `length(atmo)` et ne
normalise plus : les deux arrivent séparés.

**La migration est neutre par construction.** `avec_atmo(v)` — l'écriture
historique `teinte × intensité`, que garde la quasi-totalité des 161 entrées —
pose `densite = |v|` et `teinte = v.normalisé`. On retrouve exactement l'ancien
rendu ; test `lecriture_historique_de_latmosphere_est_preservee`. Le nouveau
constructeur `avec_air(teinte, densite)` sert aux entrées corrigées.

Quatre corrections délibérées, toutes le même cas : **épais mais terne**, qui
n'était pas *mal évalué* mais **inécrivable** — un halo sombre ne pouvait se
noter que comme un halo mince.

| monde | avant | après | pourquoi |
|---|---|---|---|
| Vénus | 0.276 | **1.45** | 92 bars ; ne s'en sortait que par le terme `voile` |
| Titan | 0.313 | **1.20** | 1,5 bar, brume à 600 km — la couche la plus haute |
| Hycean | 0.587 | **1.30** | enveloppe épaisse de vapeur : c'est sa définition |
| Ash | 0.000 | **0.32** | des cendres en suspension **sont** de l'air |

Plafond de densité du shader porté de 1.6 à 2.4 (`PLAFOND`) : Vénus et Titan
saturaient tous deux au même halo et cessaient de se distinguer. **Aucun monde
antérieur n'atteignait 1.6** (le plus dense sortait à 0,99) — relever ne change
rien à l'existant.

Trois effets de bord traités :

- `atmo` **n'est plus déclaré** dans `planete.frag.glsl` : le corps ne peint plus
  le halo, il lui suffit de savoir qu'il y a de l'air. Les deux portes
  (hydrologie, nuages) lisaient `atmo.r + atmo.g + atmo.b + voile` — une somme
  qui vaudrait ~1,7 sur un caillou nu maintenant que la teinte est unitaire.
  Elles lisent `atmo_densite + voile`. **C'est `aucun_uniform_du_corps_n_est_ignore`
  qui a attrapé l'uniform devenu mort**, sans que je l'aie vu : le garde a servi.
- `genese::apparences::gazeuse` écrivait `a.atmo = atmo` **en direct**, court-circuitant
  le constructeur. Passée par `avec_atmo`.
- Donner de l'air à Ash lui ouvrait la garde « ni air ni eau → pas de villes » :
  un monde de cendre et de lave se serait mis à briller de lumières urbaines côté
  nuit (le shader ne bloque qu'au-delà de `lave >= 0.5`, et Ash est à 0,3).
  `villes = 0` posé explicitement, avec sa raison.

⚠️ **Un test écrit puis jeté.** La première version de la garde cherchait un
monde « plus dense ET moins lumineux que la Terre » — censé être impossible sous
l'ancien encodage. Elle est partie rouge, et **elle avait tort** : une teinte
ramenée à la norme 1 voit sa luminance dictée par sa *couleur* (le bleu pèse 0,07
en Rec. 709, le jaune 0,93), pas par son épaisseur. Elle mesurait la teinte en
croyant mesurer l'air. Remplacée par la préservation de l'écriture historique,
qui est vérifiable. La note est gardée dans le test.

## P.4 — Finitions planétaires : deux items déjà faits, deux vrais manques

**Vérifié avant d'ouvrir le chantier**, comme pour P.3 — et la liste était à
moitié périmée :

| item | état réel |
|---|---|
| ombres portées des nuages au sol | **déjà fait** (`planete.frag.glsl` l. 840-847) |
| inclinaison d'axe | **déjà fait** (`axe`, par planète) |
| **vitesse** de rotation | ✗ codée en dur : `time * 0.01`, la même pour les neuf |
| ombre de l'anneau **sur la planète** | ✗ seule la réciproque existait |

### Rotation propre : neuf planètes, une seule cadence

`rot_vitesse` par planète, alimentée par `ROTATIONS_JOURS` — les **vraies**
périodes sidérales, la Terre pour référence (un tour en ~10 min réelles). Jupiter
boucle en 10 h et sort donc 2,4 fois plus vite ; Vénus et Uranus tournent **à
l'envers**, ce qui ne coûte qu'un signe et se voit.

⚠️ **Bornée**, et c'est un choix : à l'échelle littérale, Vénus mettrait 42 h de
jeu pour un tour — indiscernable d'une planète figée, donc lu comme un bug plutôt
que comme un fait. Plancher à 1/40 de la Terre, plafond à ×4.

La table est posée dans `fige()`, là où les autres données réelles d'un corps
atterrissent déjà (rayons, masses) — pas une nouvelle voie d'alimentation.

### Ombre de l'anneau sur la planète

La réciproque existait (la planète assombrit son anneau, via `ombre_rayon`), pas
celle-ci — pourtant la plus visible des deux : une large bande sombre en travers
des nuages de Saturne, dont la position raconte la saison.

Géométrie exacte, en quatre lignes. En rayons planétaires, le point de surface
est `n` (norme 1) ; on remonte le rayon lumineux jusqu'au plan de l'anneau :

```
dot(n + t·L, rn) = 0   ->   t = -dot(n, rn) / dot(L, rn)
```

`t > 0` dit que l'anneau est bien **entre** la surface et l'étoile ; reste à voir
si le point percé tombe dans la couronne. Le cas rasant (`|dot(L, rn)| < 0.02`)
est écarté : c'est l'équinoxe, l'anneau ne projette qu'un fil. La géométrie de
l'anneau vivait déjà dans `Apparence` — elle n'atteignait simplement pas ce
shader.

### Un test nouveau, né d'un red-check resté vert

Saboter le shader pour revenir à la cadence codée en dur laissait tout vert : la
correspondance uniforms ↔ GLSL ne compare que des **listes de déclarations**, pas
les usages. Un uniform déclaré des deux côtés et ignoré est pourtant un réglage
qui ne fait rien — *la donnée est juste, la plomberie ne suit pas*, le travers le
plus tenace de ce projet.

D'où `uniforms_inutilises` : tout uniform déclaré doit être **lu** quelque part.
Comparaison d'identifiants entiers, pour qu'`atmo` ne passe pas pour lu à cause
d'`atmo_densite`.

⚠️ **Portée mesurée, pas supposée** : il attrape « déclaré et jamais mentionné »,
pas « mentionné dans du code mort » — débrancher le bloc de l'ombre d'anneau le
laisse vert. Le distinguer demanderait d'analyser l'atteignabilité du GLSL, hors
de proportion.

Deuxième red-check vert corrigé au passage : le garde-fou « période nulle »
était **redondant** — `clamp` ramène l'infini au plafond. Redondant mais pas
équivalent : sans lui, une période absurde donnait la planète la **plus rapide**.
L'assertion « fini et positif » ne distinguait pas les deux ; elle nomme
désormais la valeur attendue.

## P.4 bis — Nuages en couche propre, et le glint qui survit au terminateur

### Une couche, pas une peinture sur le sol

Les nuages étaient échantillonnés dans le repère du **sol**. Ils dérivaient un
peu (décalages de temps sur le bruit) mais restaient soudés au relief : un
continent gardait éternellement la même couverture. Ce n'était pas une couche,
c'était une texture de plus.

Ils ont désormais leur **propre repère**, qui super-tourne. Deux termes, et il
faut les deux :

- un facteur **multiplicatif** (×1,35), qui garde le sens de rotation — une
  planète rétrograde a une atmosphère rétrograde ;
- une dérive **additive**, sans quoi un rotateur très lent aurait des nuages
  aussi figés que son sol, et la couche cesserait de se lire comme une couche.

Le cas d'école est Vénus : son atmosphère boucle en 4 jours quand sa surface en
met 243, soixante fois plus vite.

**L'ombre portée suit.** Elle échantillonne le même champ, décalé vers le soleil
— donc dans le repère des nuages, avec la lumière exprimée dans **ce** repère.
Mélanger les deux ferait dériver l'ombre loin de son nuage à mesure que les
repères s'écartent, et ils s'écartent d'un tour complet.

### Pas d'air, pas de nuages

Condition ajoutée dans le shader (`atmo + voile`), et **les géantes gazeuses ne
passent jamais là** : leur branche rend la main bien plus haut dans `surface()`.
Vérifié avant d'ajouter quoi que ce soit, plutôt que supposé.

L'invariant existait déjà côté **catalogue** (`la_vie_et_les_nuages_ont_leurs_conditions`).
Il manquait pour les mondes **tirés** — or la skymap n'utilise pas les presets,
elle tire avec `apparence_tellurique`. Un caillou sans air qui en sortirait
nuageux n'était attrapé par rien, et le garde-fou du shader l'aurait rendu
**invisible** au lieu de le corriger. D'où
`les_mondes_tires_n_ont_pas_de_nuages_sans_air`, qui balaie cinq graines sur tout
le spectre de températures.

### Le reflet de l'océan éteint là où il est le plus beau

Il était multiplié par `diff`, le diffus lambertien : le glint s'éteignait donc
exactement au terminateur. C'est l'inverse de la physique — un reflet spéculaire
est **d'autant plus fort que l'angle est rasant** (Fresnel), et les images
satellite les plus spectaculaires du glint terrestre sont prises près du
terminateur.

L'extinction est désormais posée sur la **géométrie** (pleine puissance jusqu'au
terminateur, éteinte peu après) plus un renfort de Fresnel au rasant. Un reflet
qui déborderait franchement sur la nuit serait, lui, une lampe.

⚠️ **Ce qui est testé, et ce qui ne l'est pas.** La règle « pas de nuages sans
air » est tenue côté **données** (catalogue et générateur). Le comportement du
**shader** ne l'est pas : les deux sabotages — retirer la condition d'air,
recoller les nuages au repère du sol — restent verts. `uniforms_inutilises` ne
voit que « déclaré et jamais mentionné », et ces uniforms restent mentionnés
ailleurs. Vérifier ça demanderait un banc de rendu GPU ; en attendant, cette
moitié tient à la lecture et à l'œil.

## Le système solaire et le catalogue

Le preset solaire (`genese/presets.rs`) tire **déjà** ses apparences du
catalogue de la galerie (`preset_tellurique` / `preset_gazeuse`, qui **paniquent**
sur un nom inconnu — il n'y a pas de repli silencieux). Deux corrections le
2026-08-05 :

**1. Le catalogue est devenu déterministe** — c'était la cause racine.

`catalogue_telluriques()` / `catalogue_gazeuses()` tiraient une graine **et** une
taille au sort à chaque construction. Conséquence : la « Terre » de la galerie
n'était pas une référence, mais une planète différente à chaque ouverture de
l'écran. Le système solaire ne pouvait donc **pas** lui ressembler — il n'y avait
rien à quoi ressembler.

Graine et taille sont maintenant déduites du **nom du preset**
(`genese::graine_de_nom`, FNV-1a, et `ClasseTaille::rayon_pour`). Le brassage n'a
pas disparu : il a changé de place, dans la galerie, où la touche **G**
incrémente une `variation`. À `variation == 0`, le décalage est nul — **la vue
par défaut de la galerie EST le catalogue canonique**, ce qui est ce à quoi le
système solaire s'aligne.

Les corps à preset **unique** prennent donc la graine du catalogue et sont
identiques à leur vignette de galerie. `fige(…, "<corps>")` ne subsiste que là où
un preset est **réutilisé** (Callisto et Obéron tirent tous deux « Lune »), sans
quoi ils seraient rigoureusement jumeaux.

**Gain de côté** : sans `gen_range`, le catalogue **se teste enfin**. Cinq tests
neufs, dont « chaque preset demandé par les systèmes scénarisés existe » — jusqu'ici
garanti par un seul `panic!` au lancement, écran noir à la clé.

**2. Appariements revus**, pour cesser de recycler le même preset :

| Corps | Avant | Après | Pourquoi |
|---|---|---|---|
| Ganymède | `Lune` | `Crevasse` | ses sillons (sulci) |
| Callisto | `Carbone` | `Lune` | le corps le plus cratérisé connu |
| Ariel | `Boule de neige` | `Supraglacial` | terrains fracturés brillants |
| Pluton | `Boule de neige` | `Ice Dunes` | dunes vues par New Horizons |

⚠️ **Fixtures recopiées** : les listes de noms des tests (`DEMANDES_TELLURIQUES`,
`DEMANDES_GAZEUSES`, `CORPS`) sont recopiées de `presets.rs`. Un nom ajouté là-haut
sans l'être ici est **moins couvert**, il ne devient pas faux.

⚠️ **Piège évité de justesse** : `chaque_taille_reste_dans_les_bornes_de_sa_classe`
déduisait d'abord ses bornes de `rayon_pour`, la fonction même qu'il teste —
l'attendu bougeait avec elle et le test ne pouvait pas échouer. Il lit maintenant
`bornes_terrestres()` à la source. C'est le mode de défaillance récurrent du
projet : *un test qui mesure autre chose que ce que son nom annonce*.

⚠️ **Bug de LOD corrigé** (`rendu.rs`) : `planete::set_viewport_h` n'était réglé
**que par la galerie**. Dans la vue système, `px_rayon` retombait sur
`screen_height()` — donc en mode pixel art, les planètes étaient ombrées avec le
détail d'un plein écran alors qu'elles sont dessinées dans une cible deux fois
plus petite. Les débris avaient déjà leur correction, pas les planètes.

## Cohérence du catalogue de planètes

Passe du 2026-08-05, déclenchée par « Mercure a de la glace ». **Trois défauts,
dont deux dans le shader** — la donnée, elle, était déjà cohérente.

| Défaut | Où | Cause |
|---|---|---|
| Banquise sur des mondes sans climat (Mercure, Lune, Carbone, Vénus) | `planete.frag.glsl` | `calotte = 1` était censé dire « aucune », mais le seuil est perturbé par un bruit de ±0,30 et `froid` atteint déjà 1 au pôle : `smoothstep(1, 1.05, 1.5)` rendait 1 |
| Lumières de ville sur les cailloux morts | `apparence.rs` + shader | `villes` vaut **1 par défaut** et le shader n'excluait que lave et voile — aucune condition d'air ou d'eau |
| (faux positif) Rivières sans eau sur `Crevasse` | — | ce sont des **coulées de lave** (`riv_lave`) : c'est le test qui était faux |

`calotte >= 1` est maintenant un **sentinelle exact** dans le shader, et
« ni air ni eau ⇒ pas de villes » est appliqué **dans la donnée** (le `push` du
catalogue), là où ça se teste.

Six tests de cohérence neufs, tous red-checkés. Ils tiennent des invariants, pas
des valeurs : *pas de climat ⇒ pas de banquise*, *un monde annoncé gelé porte
vraiment de la glace*, *un monde habitable garde ses villes* (sans quoi une règle
trop large les éteindrait partout sans qu'on le voie), *une rivière coule d'eau
ou de lave*.

⚠️ Ces tests ne sont possibles que **depuis que le catalogue est déterministe** :
avant, il exigeait le contexte graphique.

