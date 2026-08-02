# Index de la documentation

Trois familles transverses :

- **`conception/`** — le *pourquoi* : intentions, architecture, décisions de
  design. Écrit avant ou pendant le code, n'est pas censé changer à chaque
  session.
- **`suivi/`** — le *où on en est* : bilans, catalogues de presets (bucket
  lists), passations de chantier, audits de référence, journaux étape par
  étape. Change souvent, se relit en début de session pour reprendre un
  chantier.
- **`reference/`** — des **fichiers générés**, reconstruits et comparés par
  un test (jamais édités à la main). Un seul pour l'instant.

Un sujet peut n'avoir qu'un des deux premiers (ex. les ceintures n'ont pas de
suivi dédié ; les étoiles n'ont pas de conception dédiée — leur bucket list
*est* la trace de ce qui existe).

---

## conception/

| Fichier | Sujet |
|---|---|
| [`stations.md`](conception/stations.md) | Stations spatiales procédurales : modèle de ports, composants, budget/unités/symétrie, raccordement ports↔assemblage, classes de stations (ISS → O'Neill), **+ Partie E : refonte de `composant.rs` et composite `SousEnsemble`** (vers un futur éditeur façon VAB). Fusion de 4 anciens docs. |
| [`assembleur.md`](conception/assembleur.md) | Suite de la Partie E.4 ci-dessus : éditeur d'assemblage interactif façon VAB. Audit du parc de tests existant (§1–§4), ce qui manque au code actuel (§5, Lot 1), ce qu'il faut à l'éditeur (§6–§8, Lot 2), l'enveloppe rectangle des plaques (§9, « le boudin »), et **l'écran en détail + le découpage des lots 3 à 6 (§10)**. |
| [`planetes.md`](conception/planetes.md) | Planètes telluriques : pipeline de rendu (grille cube-sphere, érosion, hydrologie, biomes) + modèle de variantes/catalogue. Fusion de 2 anciens docs. |
| [`geantes_gazeuses.md`](conception/geantes_gazeuses.md) | Géantes gazeuses : surface V2 (profil zonal, palette, vortex, pôles) + recherche noises/patterns. Fusion de 2 anciens docs. |
| [`ceintures.md`](conception/ceintures.md) | Champs de débris unifiés : ceintures d'astéroïdes, anneaux planétaires, disques proto*, nuage de Oort — un seul modèle `Disque` paramétrique. |
| [`starmap.md`](conception/starmap.md) | Vue galactique (voisinage stellaire, projection oblique rétro, zoom vers la Skymap). |
| [`systemes_multiples.md`](conception/systemes_multiples.md) | Systèmes à plusieurs étoiles (binaires/trinaires/quadruples), modèle orbital analytique unifié (étoiles + planètes + lunes « sur rails »). |

## suivi/

| Fichier | Sujet |
|---|---|
| [`stations.md`](suivi/stations.md) | **Priorités immédiates en tête** (à corriger avant tout), puis passation du chantier générateur de stations (état des lieux, bugs résolus), audit de référence du preset ISS, et **état/manques de l'ISV Venture Star** (Partie C). Fusion de 2 anciens docs. |
| [`assembleur.md`](suivi/assembleur.md) | Journal du chantier `conception/assembleur.md`, étape par étape : tableau de bord (Lot 1 fait, §9 « le boudin » fait, Lot 2 pas commencé), questions ouvertes, journal détaillé avec red-check de chaque étape. À relire en premier pour reprendre ce chantier — voir aussi [`STATE.md`](../STATE.md) à la racine. |
| [`planetes.md`](suivi/planetes.md) | Bilan de la refonte v2 du rendu tellurique (fichiers touchés, perfs, tests) + catalogue des ~126 presets. Fusion de 2 anciens docs. |
| [`geantes_gazeuses.md`](suivi/geantes_gazeuses.md) | Catalogue des géantes gazeuses (archétypes, features, presets) et bilan de la V2. |
| [`etoiles.md`](suivi/etoiles.md) | Catalogue de la galerie des étoiles (types, rendu, chantiers restants). |
| [`bucketlist_globale.md`](suivi/bucketlist_globale.md) | Bucket list historique du projet dans son ensemble (fondations : étoile aléatoire, orbites, ceinture, refactos successifs). |

## reference/

| Fichier | Sujet |
|---|---|
| [`fils.md`](reference/fils.md) | Catalogue généré des 31 variantes de composants du vaisseau : fils numérotés (mêmes numéros qu'à l'écran, touche **F**), profil tranché, serrage de l'enveloppe. Régénérer avec `FILS=1 cargo test --release le_catalogue_des_fils`. |

---

## Note

Les fichiers fusionnés gardent leurs sections d'origine sous des titres
« Partie A / B / (C / D) », dans l'ordre de lecture conseillé. Le code
(commentaires `//!`/`///`) référence ces documents sous la forme
`docs/conception/<sujet>.md, Partie X §N` — la lettre de partie désigne le
document d'origine, le numéro de section n'a pas changé.
