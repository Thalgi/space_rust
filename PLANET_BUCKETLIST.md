# Planet Bucket List

Catalogue des planètes telluriques. Voir `CONCEPTION_PLANETES.md` pour le modèle paramétrique.
`[x]` = un preset existe dans la **Galerie** (ou est généré aléatoirement).

Légende : `[ ]` à faire · `[x]` fait · `(R)` rare · `(F)` flavor (rendu comme la famille parente)

---

## Bilan — CATALOGUE COMPLET ✅

**~126 presets tellurique** en galerie (3 passes). Toutes les familles de Planetary
Diversity sont couvertes, plus les non-habitables et les cas spéciaux.

**Axes / features implémentés** (uniforms, hot-reload R) : altitude par domain warping +
**étagement de biome** (prairie → forêt → roche → neige), `eau`+`eau_motif`, `veg`,
`rivieres` (+`riv_lave`), `grad_lat`, `calotte` (glace fracturée, bord déchiqueté), `nuages`
**2 couches** + `nuages_type` (classique/tempête/cyclone), `relief` (montagnes), `dunes`,
`mesa`, `pics`, `recifs`, `basalt`, `crateres`, `voile` (Vénus/Titan), `eyeball` (marée),
`cryo`, `biolum`, `lave`, **reflet spéculaire océan**, `seed` (géographie unique).

**Reste (optionnel, non bloquant)** :

- [ ] Rotation à **vitesse variable** par planète (la rotation existe déjà, vitesse uniforme).
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

## Humide › Continental
- [x] Megaflora (R) · Retinal · Forest · Petrified (R) · Lake · Tepid · Mushroom · Sakura ·
  Carotene · Moss · Marsh

## Humide › Océan
- [x] Reef (R) · Cascadian · Swamp · Archipelago (R) · Crag · Fog · Kelp · Columnar ·
  Barnacle · Pink Algae · Tidepool
- [x] **Ocean pur** (aucune terre émergée, eau = 1.0) · **Hycean** (océan global sous
  atmosphère de vapeur épaisse) · **Ocean de magma** (mer de roche fondue, émissif la nuit)

## Humide › Tropical
- [x] Geothermal (R) · Atoll · Mangrove · Bioluminescent (R) · Tepui · Cenote · Fungal ·
  Lilypad · Thunderstorm · Aerial · Obsidian

## Sec › Désert
- [x] Salt (R) · Oasis · Outback · Aquifer (R) · Dune · Coastal · Fungi · Cactus ·
  Dust Storm · Ironsand · Sodalite

## Sec › Aride
- [x] Coral (R) · Mesa · Mediterranean · Primal (R) · Fog Desert · Badlands · Succulent ·
  Amethyst · Superbloom · Striped · Opal

## Sec › Savane
- [x] Baobab (R) · Scrubland · Steppe · Geoglyph (R,F) · Pampa · Veldt · Acacia · Heath ·
  Bushveld · Termite (F) · Amber

## Froid › Arctique
- [x] Storm (R) · Cold Desert · Antarctic · Iceberg (R) · Glacial · Aeolian · Ice Spike ·
  Ice Dunes · Supraglacial · Crevasse · Ferrosprings

## Froid › Toundra
- [x] Cryflora (R) · Bog · Mud · Lichen (R) · Mycelium* · Basalt · Tuya · Treeline ·
  Travertine · Cryovolcano · Peatland   (*Mycelium ≈ Fungal/Bog)

## Froid › Alpin
- [x] Glaciovolcanic (R) · Boreal · Highland · Lanthanide (R) · Snow · Dune Forest · Fjord ·
  Taiga · Ravine · Blossom · Craton

## Volcanique
- [x] Ash

## Gaia & Superhabitables
- [x] Dry Gaia · Cold Gaia · Wet/Dry/Cold Superhabitable

## Verrouillées par marée & Mondes-grottes
- [x] Wet/Dry/Cold Tidally Locked (Eyeball) · Wet/Dry/Cold Cave (F)

## Non-habitables
- [x] Lune · Diamant · Chthonien · Subglaciaire · Vénus · Titan · Fer/Mercure · Carbone ·
  Ash · Io (soufre)
