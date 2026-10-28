# Index de la documentation

Trois familles transverses, plus un fichier à part :

- **`conception/`** — le *pourquoi* : intentions, architecture, décisions de
  design. Écrit avant ou pendant le code, n'est pas censé changer à chaque
  session.
- **`suivi/`** — le *où on en est* : bilans, catalogues de presets (bucket
  lists), passations de chantier, audits de référence, journaux étape par
  étape. Change souvent, se relit en début de session pour reprendre un
  chantier.
- **`reference/`** — des **fichiers générés**, reconstruits et comparés par
  un test (jamais édités à la main). Un seul pour l'instant.
- **[`methode.md`](methode.md)** — la *discipline* : red-check, pièges de tests,
  règles apprises à leurs dépens. Transverse à tous les chantiers, **à lire
  avant d'écrire un test**.

Un sujet peut n'avoir qu'une des deux premières familles (les étoiles n'ont pas
de conception dédiée — leur bucket list *est* la trace de ce qui existe).

> **Réorganisation du 2026-08-10.** `STATE.md` avait dérivé jusqu'à 1 968 lignes
> en devenant le journal de tous les chantiers à la fois. Il est revenu à son
> rôle — état et suite, deux cents lignes — et son contenu a été **réparti** dans
> les journaux ci-dessous. Quatre fichiers sont nés de cette passe :
> [`suivi/ceintures.md`](suivi/ceintures.md),
> [`suivi/technique.md`](suivi/technique.md),
> [`suivi/revues.md`](suivi/revues.md) et [`methode.md`](methode.md).

---

## conception/

| Fichier | Sujet |
|---|---|
| [`concept_jeu.md`](conception/concept_jeu.md) | **2026-08-14.** Chapeau au-dessus de tous les autres : la vision RTS (isolement des ressources, plateau de tuiles, recherche/tech tree, habitats spatiaux, superstructures et leur dépendance à l'énergie, classes de vaisseaux). Sert de référence commune à [`trajectoires.md`](conception/trajectoires.md) et au futur chantier économie (D-INT-2). |
| [`stations.md`](conception/stations.md) | Stations spatiales procédurales : modèle de ports, composants, budget/unités/symétrie, raccordement ports↔assemblage, classes de stations (ISS → O'Neill), **+ Partie E : refonte de `composant.rs` et composite `SousEnsemble`**. ⚠️ §E.4 est **historique** : l'éditeur qu'elle anticipait existe. |
| [`assembleur.md`](conception/assembleur.md) | Suite de la Partie E.4 ci-dessus : éditeur d'assemblage interactif façon VAB. Audit du parc de tests existant (§1–§4), ce qui manque au code actuel (§5, Lot 1), ce qu'il faut à l'éditeur (§6–§8, Lot 2), l'enveloppe rectangle des plaques (§9, « le boudin »), **l'écran en détail + le découpage des lots 3 à 6 (§10)**, et **les trois modes d'édition Move/Rotate/Resize (§11, lots 7 à 9)**. |
| [`planetes.md`](conception/planetes.md) | Planètes telluriques : pipeline de rendu (grille cube-sphere, érosion, hydrologie, biomes) + modèle de variantes/catalogue. Fusion de 2 anciens docs. |
| [`geantes_gazeuses.md`](conception/geantes_gazeuses.md) | Géantes gazeuses : surface V2 (profil zonal, palette, vortex, pôles) + recherche noises/patterns. Fusion de 2 anciens docs. |
| [`ceintures.md`](conception/ceintures.md) | Champs de débris unifiés : ceintures d'astéroïdes, anneaux planétaires, disques proto*, nuage de Oort — un seul modèle `Disque` paramétrique. |
| [`astres_v2.md`](conception/astres_v2.md) | **2026-09-24 → 10-19.** Le chantier **Astres V2** : flares et CME, variation des textures, astres exotiques, trous noirs et ceintures de débris. Lots 0 à 12, plus les sous-lots 3.1-3.5 (le champ magnétique d'où découlent taches et arches), 5.1-5.5 (le disque d'accrétion), 6.1-6.3 (les types au tirage, puis **l'ébullition des géantes rouges**, §6 nonies) et 9.1-9.3 (les comètes). Lot 8 (lentille) abandonné. Journal : [`suivi/astres_v2.md`](../suivi/astres_v2.md). |
| [`astres_v2_lentille.md`](conception/astres_v2_lentille.md) | Pourquoi la lentille gravitationnelle (lot 8) est difficile, chiffres à l'appui. **À lire avant de la reprendre** : elle a été abandonnée le 2026-10-01, la déviation correcte dominant le trou noir au lieu de l'habiller. |
| [`astres_v2_recherche_visuelle.md`](conception/astres_v2_recherche_visuelle.md) | La cible visuelle des régions actives stellaires, et neuf critères d'acceptation. |
| [`astres_v2_champ_partage.md`](conception/astres_v2_champ_partage.md) | D'où faire découler taches **et** arches : trois pistes, et pourquoi un champ en uniforms l'emporte. |
| [`astres_v2_boucles.md`](conception/astres_v2_boucles.md) | Comment tracer une arche qui se lise comme un tube : le ruban orienté, et ce qui a été écarté. |
| [`astres_v2_disque_accretion.md`](conception/astres_v2_disque_accretion.md) | Première recherche sur le disque d'accrétion : la cause décisive est la **silhouette**, pas la matière. |
| [`genese_v2.md`](conception/genese_v2.md) | **2026-10-28.** **Chantier Genèse V2** : la carte avant la refonte — ce que fait la genèse, ce que les autres chantiers attendent d'elle (planètes, trajectoires, Lagrange, stations, économie), les décisions à trancher, les mécanismes manquants (points de Lagrange, hasard en flux nommés, tirage dans le catalogue, habitabilité graduée par la température, banc statistique). Lots G0 à G8 (plus G1 bis), rien de codé ; le découpage plan / construction (G1) est aussi le point d'accroche de la qualité d'Opti-v2 (O5). |
| [`opti_v2.md`](conception/opti_v2.md) | **2026-10-26.** **Chantier-Opti-v2** : la carte de la dette de performance d'Astres V2, pour 30 fps sur un premier APU Ryzen portable. Mesures (vue isolée, vue système, T Tauri décomposée), onze dettes classées, leviers, lots O1 à O8, et la méthode (harnais `PROFIL` sans synchro, `SYSTEME=<type>`). |
| [`astres_v2_disque_accretion_2.md`](conception/astres_v2_disque_accretion_2.md) | **Seconde recherche (web).** Sept défauts **mesurés** de notre rendu, onze corrections classées par gain/coût, et l'arbitrage qu'aucune mesure ne tranche : un disque physiquement juste rend l'ombre du trou noir illisible. |
| [`interface.md`](conception/interface.md) | **L'interface de jeu** (vue système) : barre de ressources, nom du système, sélecteur de planètes rétractable, panneau de planète au clic. Lecture du schéma du 2026-08-04, inventaire des 14 sprites, ce qui manque au modèle (noms d'astres, économie), découpage en étapes I.0–I.5. |
| [`pixel_art.md`](conception/pixel_art.md) | **Rendu pixel art** : sous-échantillonnage + quantification CIELAB vers une palette de 64 couleurs. Reprend le guide d'origine **corrigé** de ce qui ne compilait pas ici, dit ce qui est quantifié et ce qui reste net, et liste les dettes D-PIX-1 à 4. |
| [`starmap.md`](conception/starmap.md) | Vue galactique (voisinage stellaire, projection oblique rétro, zoom vers la Skymap). |
| [`systemes_multiples.md`](conception/systemes_multiples.md) | Systèmes à plusieurs étoiles (binaires/trinaires/quadruples), modèle orbital analytique unifié (étoiles + planètes + lunes « sur rails »). |
| [`trajectoires.md`](conception/trajectoires.md) | **2026-08-14 → 16.** Faire voyager un vaisseau assemblé (ou une station) d'un astre à un autre, au lieu d'un rail fixe. Onze décisions de gameplay tranchées (§4), conséquences techniques (§5), découpage en lots (§6). **Lots A, B, C et D faits** (statistiques de vol, prédiction de position, arc de trajectoire, vaisseau mobile, UI de lancement, aperçu du trajet + caméra qui suit) — un Starship posté vole pour de vrai en jeu, avec un aperçu ambre pendant l'armement. Journal : [`suivi/trajectoires.md`](../suivi/trajectoires.md). Lot E (arrivée) reste à faire. |

## suivi/

| Fichier | Sujet |
|---|---|
| [`stations.md`](suivi/stations.md) | Le plus gros journal du projet. Générateur de stations, audit du preset ISS, **ISV Venture Star** (Partie C), **mégastructures et vaisseaux** (Partie F : tore de Stanford, complexe Elysium, Starship de bout en bout), puis les **composants neufs** de 2026-08-09/10 — miroir primaire, module en coin d'arc, grappe de cuves, hangar à baie hexagonale, charge du mât polaire. |
| [`assembleur.md`](suivi/assembleur.md) | Journal du chantier `conception/assembleur.md`, étape par étape. **Lots 1 à 5 clos** (modèle, écran, déplacement de sous-arbre, sauvegarde disque, overlay des refus) **+ rotation au clavier** ; restent les lots 6 (dette d'enveloppes, composites), 8 (décalage) et 9 (redimensionnement). Red-check de chaque étape, y compris les tests jetés parce qu'ils ne gardaient rien. |
| [`planetes.md`](suivi/planetes.md) | Bilan de la refonte v2 du rendu tellurique + catalogue des ~126 presets, **+ Partie C** (2026-08-10) : atmosphères (P.1 et la scission `atmo`/`atmo_densite`), finitions P.4 (rotation propre, nuages en couche, ombre d'anneau, glint), cohérence du catalogue. |
| [`ceintures.md`](suivi/ceintures.md) | **Nouveau (2026-08-10).** Tout le chantier P.2 : la mesure qui a montré qu'il n'y avait rien à optimiser, la barre sombre des étoiles protosolaires, la couture des disques trouvée **par sondes**, les débris en gros carrés, l'annulation catastrophique, et la découverte que `Projection` n'est pas la projection. |
| [`opti_v2.md`](suivi/opti_v2.md) | **2026-10-28 →.** Journal du **Chantier-Opti-v2** (branche `chantier-opti-v2`) : méthode de mesure, puis lot par lot. **O1 livré** : les nuages calculés une fois par image, T Tauri 38,9 → 76,5 fps en vue isolée. |
| [`astres_v2.md`](suivi/astres_v2.md) | **2026-09-26 → 10-26.** Journal du chantier `conception/astres_v2.md`, lot par lot. **Lots 0 à 7 et 9 à 11 livrés et validés à l'écran** (dont le 6.3, l'ébullition des géantes rouges, et les 9.3-9.5, familles de comètes, queues reprises et trajectoires à part), **lot 8 abandonné**, **lot 12 clos** (T Tauri, Herbig Ae/Be, jets polaires, étoile Be, hypergéante, grand disque posé par la genèse), **lot 13 clos** (rémanents refaits en 3D), **lot 14 clos** (la naine blanche polluée, verte au sodium), **lot 15 clos** (la naine noire) — tous les lots livrés ; reste une passe de relecture et la refonte de la genèse. Où reprendre, ce qui reste ouvert (dont un crash intermittent à attraper par sa pile d'appels), et les leçons du chantier — dont « une absence ne se corrige pas par un réglage » et « un sabotage doit être vérifié, pas seulement lu ». |
| [`etoiles.md`](suivi/etoiles.md) | Catalogue de la galerie des étoiles, **+ le chantier P.3** : les éruptions existaient déjà, les deux causes chiffrées (l'horloge, l'échelle) et l'arbitrage du plancher à 8 px. |
| [`interface.md`](suivi/interface.md) | Journal de **l'interface de jeu** : étapes I.0 à I.5 et les bouche-trous soldés, **+ Partie B** (2026-08-10) : bascules de calques, trajectoires en points, filtre de focus, les deux pièges de police, les paramètres du jeu et l'échelle du système solaire. |
| [`technique.md`](suivi/technique.md) | **Nouveau (2026-08-10).** Les pièges du moteur, ceux qui ne tiennent à aucun chantier : plafond de 32 pipelines, clone de `Material` qui partage son pipeline, géométrie jamais double-face, uniform déclaré-mais-non-lu qui vaut zéro en silence. |
| [`revues.md`](suivi/revues.md) | **Nouveau (2026-08-10).** Les passes de relecture systématique et ce qu'elles ont appris — dont le fait que « 86 avertissements clippy sans défaut réel » était faux, et qu'un vrai défaut y dormait. |
| [`geantes_gazeuses.md`](suivi/geantes_gazeuses.md) | Catalogue des géantes gazeuses (archétypes, features, presets) et bilan de la V2. |
| [`bucketlist_globale.md`](suivi/bucketlist_globale.md) | Bucket list du projet dans son ensemble : ce qui est fait, ce qui reste, section par section. |
| [`trajectoires.md`](suivi/trajectoires.md) | **2026-08-15 → 16.** Journal du chantier `conception/trajectoires.md`, lot par lot. **Lots A, B, C et D** clos — un Starship posté vole pour de vrai depuis un bouton LANCER en jeu, avec aperçu ambre du trajet pendant l'armement et caméra qui suit le vaisseau parti ; l'impasse d'object-safety qui a fait remplacer `Systeme::remplacer` par `Astre::tenter_lancer` ; la découverte sur le mode N-corps (retiré) ; la liste consolidée des chiffres provisoires. |

## reference/

| Fichier | Sujet |
|---|---|
| [`fils.md`](reference/fils.md) | Catalogue généré des variantes de composants du vaisseau : fils numérotés (mêmes numéros qu'à l'écran, touche **F**), profil tranché, serrage de l'enveloppe. Régénérer avec `FILS=1 cargo test --release le_catalogue_des_fils`. |

---

## Note

Les fichiers fusionnés gardent leurs sections d'origine sous des titres
« Partie A / B / (C / D) », dans l'ordre de lecture conseillé. Le code
(commentaires `//!`/`///`) référence ces documents sous la forme
`docs/conception/<sujet>.md, Partie X §N` — la lettre de partie désigne le
document d'origine, le numéro de section n'a pas changé.

---

## Repères chronologiques

Pour situer un chantier dans le temps sans relire les journaux.

| Quand | Ce qui s'est joué | Journal |
|---|---|---|
| 2026-07-29 | Découpage de `composant.rs`, composite `SousEnsemble` | [`conception/stations.md`](conception/stations.md) §E |
| 2026-07-31 → 08-03 | Conception de l'assembleur, lots 1 à 3 | [`suivi/assembleur.md`](suivi/assembleur.md) |
| 2026-08-03 → 06 | Mégastructures, puis **Starship de bout en bout** | [`suivi/stations.md`](suivi/stations.md) Partie F |
| 2026-08-04 | L'interface de jeu, étapes I.0 à I.5 | [`suivi/interface.md`](suivi/interface.md) |
| 2026-08-05 | **Revue de code** : 117 → 0 avertissement, 90 éléments morts triés (dont 77 exercés par les tests), et le vrai défaut qu'ils cachaient | [`suivi/revues.md`](suivi/revues.md) |
| 2026-08-05 → 06 | Réglages persistés, zoom au plancher `0,01`, les **deux** défauts de texte (glyphes manquants puis atlas repaqueté) | [`suivi/interface.md`](suivi/interface.md) Partie B |
| 2026-08-08 | P.1 atmosphères, P.2 mesuré (rien à optimiser) | [`suivi/planetes.md`](suivi/planetes.md), [`suivi/ceintures.md`](suivi/ceintures.md) |
| 2026-08-09 | Scission `atmo`/`atmo_densite`, plancher stellaire à 8 px, **assembleur lots 4–5**, miroir et module d'arc | [`suivi/etoiles.md`](suivi/etoiles.md), [`suivi/assembleur.md`](suivi/assembleur.md), [`suivi/stations.md`](suivi/stations.md) |
| 2026-08-10 | Tore de Stanford retravaillé, grappe de cuves, hangar hexagonal, charge du mât polaire — puis **cette réorganisation** | [`suivi/stations.md`](suivi/stations.md) |
| 2026-08-14 | Icônes du sélecteur (I.6, I.6 bis/ter) — anneau à l'horizontale, bouton ×2, colonne « liquide » | [`suivi/interface.md`](suivi/interface.md) |
| 2026-08-14 → 16 | Vision RTS globale, puis ouverture du chantier trajectoires : onze décisions de gameplay, **Lots A/B/C/D** (statistiques de vol, vaisseau mobile, UI de lancement, aperçu du trajet + caméra qui suit — un Starship vole pour de vrai en jeu), mode N-corps retiré | [`conception/concept_jeu.md`](conception/concept_jeu.md), [`conception/trajectoires.md`](conception/trajectoires.md), [`suivi/trajectoires.md`](suivi/trajectoires.md) |
| 2026-09-24 → 10-03 | **Chantier Astres V2**, branche `chantier-astres` : masse stellaire, graines, champ magnétique et arches, trou noir et son disque d'accrétion, nouveaux types au tirage, pulsation, lentille (abandonnée), comètes | [`conception/astres_v2.md`](conception/astres_v2.md), [`suivi/astres_v2.md`](suivi/astres_v2.md) |
| 2026-10-19 | Reprise après la coupure du 3 octobre ; **lot 6.3** : les géantes rouges bouillonnent (stables / instables), filtre pixel dans la galerie des étoiles | [`suivi/astres_v2.md`](suivi/astres_v2.md) |
| 2026-10-19 → 20 | Menu V2 « choisir astre » ; **lots 6, 7, 9 et 10 clos et validés** : queues de comète reprises (bouffées, nuages, ÷ 4), trajectoires à part ; éclat de la céphéide visible de loin ; chaque type sa surface, bord assombri pour toutes ; défilement des galeries corrigé ; lots futurs 13 à 15 notés | [`suivi/astres_v2.md`](suivi/astres_v2.md), [`conception/astres_v2.md`](conception/astres_v2.md) |
| 2026-10-21 | **Lot 11 clos** : la quasi-étoile au tirage — une UA, pourpre, cœur qui transparaît ; refonte future de la genèse notée | [`suivi/astres_v2.md`](suivi/astres_v2.md) |
| 2026-10-22 → 25 | **Lot 12 clos** : prototypes à la touche 4 ; disques protoplanétaires en nuages 3D (habillages B à I, versés dans la galerie des disques) ; bras de matière avalés par l'étoile (entonnoirs magnétiques, spirales) ; **la T Tauri adopte T1B4, la Herbig Ae/Be entre au tirage** ; amorti global des nuages superposés ; jet polaire standard ; **étoile Be et hypergéante au tirage** ; grand disque posé par la genèse | [`suivi/astres_v2.md`](suivi/astres_v2.md) |
| 2026-10-26 | **Lot 15 clos** : la naine noire au tirage — silhouette, braise ou cristallisée, sans éruptions. Tous les lots du chantier Astres V2 livrés | [`suivi/astres_v2.md`](suivi/astres_v2.md) |
| 2026-10-26 | **Lot 14 clos** : la naine blanche polluée au tirage — verte (WD J1330+6435, première étoile verte, octobre 2026), rose, cyan ou rouge ; l'anneau des débris de ses planètes, l'anneau de gaz chaud, son nom qui dit ce qui la pollue | [`suivi/astres_v2.md`](suivi/astres_v2.md) |
| 2026-10-26 | **Lot 13 clos** : les rémanents refaits en 3D — pulsar blanc à faisceaux courbés, étoile à neutrons violette (ou turquoise et stable), magnétars à boucles de champ ; arches à la teinte de l'astre | [`suivi/astres_v2.md`](suivi/astres_v2.md) |
