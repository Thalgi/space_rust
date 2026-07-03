# Bilan des travaux — génération de planètes v2

> Reprise du chantier ailleurs : ce document dit CE QUI A ÉTÉ FAIT, OÙ, et
> COMMENT s'en servir. Le POURQUOI (conception, algorithmes, décisions) est
> dans `conception_planete_v2.md` — les renvois « § n » pointent dedans.

## 1. Résumé exécutif

Le rendu des planètes telluriques est passé d'un shader 100 % procédural par
pixel (champs de bruit indépendants, rivières incohérentes) à un pipeline
hybride : **géographie précalculée sur CPU** (grille cube-sphere, érosion,
hydrologie, volcanisme → atlas de textures) + **habillage shader** (biomes,
détail fin, éclairage, couches émissives). Les 8 étapes prévues en conception
sont **toutes implémentées et testées** (11 tests unitaires verts). La
génération est asynchrone (pas de gel d'UI), déterministe par graine, et
outillée : bench intégré, captures de non-régression, filtre pixel.

## 2. Fichiers touchés et nature des changements

| Fichier | État | Contenu |
|---|---|---|
| `src/planete/terrain.rs` | **NOUVEAU** (~1000 lignes + tests) | Tout le pipeline CPU : grille cube-sphere équi-angulaire (mapping texel↔sphère, voisinage par re-projection 3D — pas de table d'arêtes), bruit (port du GLSL), érosion thermique + hydraulique (gouttes 3D), volcans (cônes/caldeiras/chaleur), hydrologie (priority-flood → drainage gratuit → flux log → humidité par rang), bake atlas RGBA8 3×2 avec gouttière, RNG déterministe (SplitMix64), budget de jobs concurrents, stats temps réel, bench complet. |
| `src/shaders/planete.frag.glsl` | refonte branche tellurique | Lecture atlas (`dir_vers_atlas`, altitude 16 bits R+G), niveau de mer quantile, rivières/lacs par canal flux, **régime hydrologique** (pas d'atmo → pas d'eau ; sec → salines/oueds ; lave), glace banquise/terrestre avec veines gelées, biolum qui suit la géographie, coulées de lave émissives pulsantes, normale perturbée, ombres de nuages, température locale (lat+altitude). Branches gazeuse/glacée intactes. |
| `src/planete/materiau.rs` | modifié | Texture `terrain` dans le material + uniforms `niveau_mer`/`atlas_n` ; texture 1×1 de secours (placeholder + corps non telluriques). |
| `src/planete/mod.rs` | modifié | `Planete` porte `terrain_tex`/`terrain_job` : génération **asynchrone au premier draw** (thread + budget global 2 jobs), upload GPU à la fin du job. Accesseurs `terrain_pret()`, `apparence()`. |
| `src/ecran/galerie.rs` | modifié | Scroll doux (cible + lissage exponentiel + accrochage pixel), **P** filtre pixel (render target demi-résolution, plus proche voisin, textes nets), **C** captures PNG de non-régression, **B** bench, overlay FPS/génération/état du bench. |
| `Cargo.toml` | modifié | `[profile.dev] opt-level = 1` (le bruit CPU est inutilisable en debug pur). |
| `conception_planete_v2.md` | vivant | Conception + état d'avancement par étape (§ 8) + revue du catalogue (§ 12) + non-régression (§ 13). |

Zéro dépendance ajoutée (threads std, pas de rayon ; RNG maison).

## 3. Mode d'emploi (galerie des telluriques)

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

## 4. Leviers de réglage (tous dans `terrain.rs` sauf mention)

| Levier | Valeur | Effet |
|---|---|---|
| `QUALITE` (dans `params_depuis_apparence`) | 0.15 | Gouttes d'érosion par texel. Levier perf/beauté n° 1. Si relief trop lissé → 0.20. |
| `N_ATLAS` | 256 | Résolution par face. 512 = ×4 temps et mémoire (mesuré 7-10 s/planète : pas viable sans autre optimisation). |
| `MAX_JOBS` | 2 | Générations simultanées (galerie). |
| `erosion` / `pas_max` / `evaporation` | 0.38 / 48 / 0.02 | Taux et durée de vie des gouttes (calibrés avec QUALITE 0.15). |
| `debit` volcans (dans `generer_chrono`) | 0.5 % des texels | Force des coulées. |
| `PIX` (`galerie.rs`) | 2 | Échelle du filtre pixel (aussi dans `rendu.rs` pour les autres vues). |
| Seuil rivières (shader) | `mix(0.78, 0.50, rivieres)` | Densité du réseau visible, piloté par le preset. |

## 5. Performances

- **Bench utilisateur 2026-07-02** (8 cœurs, release, avant optimisation) :
  médiane 1061 ms, max 2040 ms, érosion ~62 % du coût, bruit ~430 ms non scalé.
- **Optimisations appliquées ensuite** (§ 8.7) : bruit en bandes sur tous les
  cœurs + warp 3 octaves ; érosion −40 % de gouttes compensée + moitié moins
  de lectures par pas. VM 2 cœurs : 1,54 s → 0,85 s. **À re-bencher** (B) —
  attendu ~450-550 ms de médiane sur 8 cœurs.
- Rendu GPU : la lecture d'atlas est MOINS chère que l'ancien calcul (5+
  octaves de fbm remplacées par ~4 lectures de texture par pixel).
- Mémoire GPU : ~1,5 Mo/planète tellurique (atlas 774×516 RGBA8).

## 6. Tests et validation

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

## 7. Pièges connus / vigilance

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

## 8. Reste à faire / prochains chantiers

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
