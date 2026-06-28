# Conception — Variations de planètes telluriques

Base de réflexion pour générer le grand nombre de planètes listées dans
`PLANET_BUCKETLIST.md` (≈130 variantes nommées), à coût quasi nul, en s'appuyant sur
le socle de rendu déjà en place (impostor + table déclarative d'uniforms + hot-reload).

## 0. Principe directeur

**On ne fait pas 130 shaders ni 130 textures.** Une planète tellurique = **un point
dans un espace de paramètres**. Chaque variante nommée du catalogue (Sakura, Dune,
Cryovolcano, Mesa…) n'est qu'un **preset** : un jeu de valeurs sur ces axes. Le shader
`planete.frag.glsl` lit ces paramètres (uniforms) et compose la surface.

Conséquence : ajouter une variante = ajouter un preset (données), pas du code. Ajouter
une *capacité visuelle nouvelle* (ex. orgues basaltiques) = 1 uniform dans la table
déclarative + 1 branche dans le shader, réglable en hot-reload.

## 1. Référence

Planetary Diversity (Stellaris) — `https://steamcommunity.com/sharedfiles/filedetails/?id=819148835`.
Ossature reprise : 3 groupes climatiques **Humide / Sec / Froid**, chacun en 3
sous-familles, plus **Extrêmes**, **Exotiques/composition**, **Gaia/Superhabitables**,
**Verrouillées par marée** et **Grottes**. Le catalogue complet vit dans
`PLANET_BUCKETLIST.md` ; ce document décrit comment le produire.

## Bilan d'avancement (à date)

Le modèle paramétrique est en place et **30 presets** sont visibles en galerie.

**Axes implémentés** (uniforms dans la table déclarative de `planete/materiau.rs`,
réglables en hot-reload) : `eau` + `eau_motif` (océan/continents/mers/marais),
`couleur/2/3`, `veg_couleur`+`veg_couv`, `rivieres`, `grad_lat` (dégradé latitudinal),
`calotte` (banquise texturée à bord irrégulier), `nuages`+`nuages_couleur`, `relief`
(montagnes ridged + ombrage de pente + neige de sommet), `dunes`, `mesa` (plateaux/
strates), `pics` (glace), `recifs`, `basalt` (Worley), `lave`, `seed` (géographie unique).
Surface bâtie sur un champ d'altitude par **domain warping** + étagement.

**Restant** : voile atmosphérique opaque (Vénus/Titan), verrouillage de marée (eyeball),
cryovolcan/bioluminescence émissifs, reflet spéculaire océan, rotation visible, mondes
soufre/Titan, combinaison de deux features, et bascule du générateur sur le catalogue.

## 2. Les axes du modèle (ce qu'une planète doit pouvoir exprimer)

Revu pour couvrir les besoins du catalogue. `*` = déjà présent dans `Apparence`.

1. **Groupe climatique** — dérivé de la température d'équilibre (déjà calculée) +
   couverture d'eau. Pilote les plages par défaut de tous les autres axes.
2. **Eau** `eau`* — 0 (aride) → 1 (monde-océan). + sous-type : océan profond, lacs
   épars, mers intérieures, hauts-fonds turquoise (récifs/atolls).
3. **Palette** `couleur/couleur2/couleur3`* — roche, végétation/sable, eau/glace.
4. **Teinte de végétation** — hue dédiée pour les biomes : vert (Forest), violet
   (Retinal), rose (Sakura/Pink Algae), ambre (Carotene/Amber), fluo (Cryflora/Biolumen),
   blanc (Lichen/Salt/Travertine). → 1 paramètre hue + 1 densité de couverture.
5. **Motif / relief de surface** — un *type de feature* procédural (enum) + amplitude :
   dunes, mesas/canyons, orgues basaltiques, pics de glace, dunes de glace, récifs,
   terrasses, strates colorées (Striped/Sodalite), plaines lisses.
6. **Couche nuageuse** — densité + vitesse + couleur (Thunderstorm/Storm/Fog/Dust Storm).
7. **Atmosphère / voile** `atmo`* — couleur + densité du halo ; voile opaque qui cache
   le sol (Vénus jaune, Titan orange, Fog blanc).
8. **Calottes polaires** — latitude de la banquise, fonction de la température.
9. **Émissif** — `lave`* (fissures), + cryovolcans (points), bioluminescence + lumières
   de villes côté nuit (déjà pour océans).
10. **Verrouillage de marée** — direction du soleil **fixe** : face chaude / face gelée /
    anneau habitable au terminateur (eyeball).
11. **Rotation propre** `axe`* + vitesse (visible).
12. **Modificateur rare** — variante (R) : pousse un axe à l'extrême ou active une feature.

## 3. Comment chaque famille se règle

| Famille            | Eau     | Palette / teinte            | Feature dominante         | Nuages / voile        |
|--------------------|---------|-----------------------------|---------------------------|-----------------------|
| Humide/Continental | 0.4–0.8 | végétation (vert→violet…)   | plaines + relief doux     | modérés               |
| Humide/Océan       | 0.8–1.0 | bleu + îles                 | récifs / orgues / îles    | modérés à brumeux     |
| Humide/Tropical    | 0.5–0.9 | vert vif, parfois fluo      | jungle, lagons            | denses (orages)       |
| Sec/Désert         | 0–0.1   | sable (ocre/rouille/bleu)   | dunes                     | clairs / tempête pous.|
| Sec/Aride          | 0–0.2   | roche, strates colorées     | mesas / canyons / strates | brouillard sec        |
| Sec/Savane         | 0.1–0.3 | jaune-vert, ambre           | plaines herbeuses         | clairs                |
| Froid/Arctique     | 0–0.3   | blanc-bleu                  | banquise, pics de glace   | variables             |
| Froid/Toundra      | 0–0.3   | roche + lichen + glace      | basalte, cryovolcans      | variables             |
| Froid/Alpin        | 0.1–0.4 | vert sombre + neige         | montagnes, fjords         | brume                 |
| Extrême/Lave       | 0       | croûte sombre               | fissures (émissif) ✔      | —                     |
| Extrême/Étuve      | 0       | sol caché                   | —                         | voile CO2 jaune opaque|
| Exotique/Fer       | 0       | gris métallique             | cratères                  | aucune                |
| Exotique/Soufre    | 0       | jaune-orange (Io)           | panaches volcaniques      | fines                 |
| Exotique/Titan     | lacs    | brun-orange                 | lacs d'hydrocarbures      | voile orange opaque   |
| Eyeball (marée)    | var.    | gradient chaud→gelé         | anneau au terminateur     | bande nuageuse        |

## 4. Paliers de faisabilité (pour prioriser)

- **T1 — palette + eau + nuages + calottes** : couvre la **majorité** du catalogue
  (Forest, Sakura, Moss, Steppe, Mediterranean, Snow, Boreal, Pink Algae, Carotene…).
  Quasi gratuit : juste des paramètres de couleur/couverture/nuages.
- **T2 — features de surface procédurales** : dunes, mesas/canyons, orgues basaltiques,
  pics/dunes de glace, récifs/atolls, terrasses, strates. Un générateur de feature dans
  le shader (sélection par enum) + relief par perturbation de normale.
- **T3 — spéciaux** : eyeball (verrouillage marée), voile opaque (Vénus/Titan), émissif
  (lave ✔, cryovolcan, bioluminescence), reflet spéculaire océan.
- **Flavor / label seulement** : Cave Worlds (souterrain), Geoglyph, Megaflora, Aerial
  (cosses flottantes), Termite — invisibles ou presque depuis l'orbite. On les rend comme
  leur **famille parente** + une teinte/marqueur ; pas de shader dédié.

## 5. Impact sur le code

- [x] `genese/apparences.rs::apparence_tellurique(temp)` refondu en **groupe climatique**
  (Lave/Étuve/Sec/Humide/Froid/Gelé) qui tire les paramètres dans des plages propres.
- [x] `Apparence` + **table déclarative** de `planete/materiau.rs` étendues ; chaque ajout
  d'uniform = 1 ligne table + 1 builder `avec_*` + 1 branche `planete.frag.glsl` (hot-reload).
  Restent à câbler : `voile` (atmo opaque), `eyeball`+`sun_dir`, `cryo`.
- [ ] **Table de presets nommés** : aujourd'hui les presets vivent dans
  `genese::catalogue_telluriques()` (galerie). À terme, le **générateur** (skymap/objet)
  devrait y piocher, et un **sélecteur** en mode Objet permettrait de viser une variante.

## 6. Plan d'implémentation (étapes cheap, hot-reloadables)

1. [x] **Groupes climatiques propres** : Humide/Sec/Froid/Gelé/Étuve, palettes, eau,
   calottes + dégradé latitudinal.
2. [x] **Teinte de végétation + couverture** → Sakura, Retinal, Carotene, Mousse…
3. [x] **Couche nuageuse animée** → Brumeux, Orageux, Tempête de poussière.
4. [x] **Calottes texturées** (bord irrégulier + crevasses) + **rivières** + relief de surface.
5. [x] **Features de surface** : montagnes, dunes, mesas/strates, pics de glace, récifs,
   orgues basaltiques (un type à la fois pour l'instant).
6. [ ] **Spéciaux** (T3) : eyeball (verrouillage marée), voile opaque (Vénus/Titan),
   cryovolcan, reflet océan.
7. [ ] **Exotiques** : soufre/Io, hydrocarbures/Titan (fer & carbone déjà en presets).
8. [ ] **Sélecteur de preset** en mode Objet + **bascule du générateur** sur le catalogue.
9. [ ] **Combinaison de deux features** (Dune Forest, Geothermal…) + **perf** du shader
   (mutualiser les fbm si besoin).

## 7. Questions ouvertes

- **Sélecteur de variante nommée** en mode Objet (au-delà de G/1/2) : à faire ou pas ? (étape 8)
- **Features combinées** : aujourd'hui **une seule** feature à la fois ; passer à deux
  (Dune Forest, Geothermal) est prévu (étape 9) — à confirmer.
- **Mondes flavor** (Cave/Geoglyph/Megaflora) : gardés dans le catalogue pour la complétude
  mais rendus comme leur famille parente (pas de shader dédié).
- **Perf** : la surface tellurique fait ~15 échantillons de bruit/pixel (+ Worley si basalt).
  OK en galerie/skymap (petites planètes) ; à surveiller en plein écran mode Objet.
