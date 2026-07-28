# Index de la documentation

Deux familles transverses, chacune un fichier par sujet :

- **`conception/`** — le *pourquoi* : intentions, architecture, décisions de
  design. Écrit avant ou pendant le code, n'est pas censé changer à chaque
  session.
- **`suivi/`** — le *où on en est* : bilans, catalogues de presets (bucket
  lists), passations de chantier, audits de référence. Change souvent, se
  relit en début de session pour reprendre un chantier.

Un sujet peut n'avoir qu'un des deux (ex. les ceintures n'ont pas de suivi
dédié ; les étoiles n'ont pas de conception dédiée — leur bucket list *est*
la trace de ce qui existe).

---

## conception/

| Fichier | Sujet |
|---|---|
| [`stations.md`](conception/stations.md) | Stations spatiales procédurales : modèle de ports, composants, budget/unités/symétrie, raccordement ports↔assemblage, classes de stations (ISS → O'Neill). Fusion de 4 anciens docs. |
| [`planetes.md`](conception/planetes.md) | Planètes telluriques : pipeline de rendu (grille cube-sphere, érosion, hydrologie, biomes) + modèle de variantes/catalogue. Fusion de 2 anciens docs. |
| [`geantes_gazeuses.md`](conception/geantes_gazeuses.md) | Géantes gazeuses : surface V2 (profil zonal, palette, vortex, pôles) + recherche noises/patterns. Fusion de 2 anciens docs. |
| [`ceintures.md`](conception/ceintures.md) | Champs de débris unifiés : ceintures d'astéroïdes, anneaux planétaires, disques proto*, nuage de Oort — un seul modèle `Disque` paramétrique. |
| [`starmap.md`](conception/starmap.md) | Vue galactique (voisinage stellaire, projection oblique rétro, zoom vers la Skymap). |
| [`systemes_multiples.md`](conception/systemes_multiples.md) | Systèmes à plusieurs étoiles (binaires/trinaires/quadruples), modèle orbital analytique unifié (étoiles + planètes + lunes « sur rails »). |

## suivi/

| Fichier | Sujet |
|---|---|
| [`stations.md`](suivi/stations.md) | Passation du chantier générateur de stations (état des lieux, bugs résolus) + audit de référence du preset ISS. Fusion de 2 anciens docs. |
| [`planetes.md`](suivi/planetes.md) | Bilan de la refonte v2 du rendu tellurique (fichiers touchés, perfs, tests) + catalogue des ~126 presets. Fusion de 2 anciens docs. |
| [`geantes_gazeuses.md`](suivi/geantes_gazeuses.md) | Catalogue des géantes gazeuses (archétypes, features, presets) et bilan de la V2. |
| [`etoiles.md`](suivi/etoiles.md) | Catalogue de la galerie des étoiles (types, rendu, chantiers restants). |
| [`bucketlist_globale.md`](suivi/bucketlist_globale.md) | Bucket list historique du projet dans son ensemble (fondations : étoile aléatoire, orbites, ceinture, refactos successifs). |

---

## Note

Les fichiers fusionnés gardent leurs sections d'origine sous des titres
« Partie A / B / (C / D) », dans l'ordre de lecture conseillé. Le code
(commentaires `//!`/`///`) référence ces documents sous la forme
`docs/conception/<sujet>.md, Partie X §N` — la lettre de partie désigne le
document d'origine, le numéro de section n'a pas changé.
