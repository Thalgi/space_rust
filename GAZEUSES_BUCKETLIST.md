# Gas Giant Bucket List

Catalogue et chantiers pour les géantes gazeuses. Même approche que les telluriques :
un modèle paramétrique (uniforms hot-reloadables) + un catalogue de presets nommés,
visualisés dans la **Galerie - Géantes gazeuses** (4e bouton de l'accueil).

Légende : `[ ]` à faire · `[x]` fait · `[~]` partiel · `(R)` rare

---

## Bilan (à date) — V2 livrée

**V2 (juillet 2026, voir CONCEPTION_GAZEUSES_V2.md)** : profil zonal 1D précalculé
(`zonal.rs` : jets u(φ), bandes b(φ) par vorticité, cisaillement s(φ)), **rotation
différentielle réelle** (advection par u(φ)), **palette 100 % paramétrique**
(`palette.rs`, `gaz_pal[8]`, plus aucune couleur en dur), **vortex unifiés en slots**
(`vortex.rs` : GRS/sombre/ovale blanc/barge/chapelet, dérive le long de leur jet),
**pôles V2** (projection azimutale, anneau de cyclones Juno N≠ par hémisphère,
hexagone intégré au régime polaire), LOD `px_rayon`, brume inégale,
~17 fbm/pixel (vs ~28 en V1). Catalogue : **~27 presets** + 3 tirages aléatoires
en galerie, génération par archétypes structurels (classique/glace/chaude/lisse).

Paramètres (`Apparence`) : `couleur/2/3` (accent/ceintures/zones — contrat
palette), `nb_bandes`, `jets_force`, `zonal_asym`, `zonal_flou`, `warp_amt`,
`seed`, `poly_cotes`, `cyclones_pol`, `tache_*` (type 2 = tête de GTB),
`tempetes` (densité de slots), `brume_*`, `g_pole`, `thermique_*`, `aurore_*`,
`anneau_*`, `axe`, `atmo`. Supprimés : `band_scale`, `jet_profil`.

---

## Partie A — Classification (archétypes)

### Géantes du système solaire
- [x] **Jupiter** — bandes brun-orange + Grande Tache Rouge
- [x] **Saturne** — bandes dorées pâles + hexagone polaire
- [x] **Uranus** — cyan quasi uniforme (géante de glace calme)
- [x] **Neptune** — bleu profond + Grande Tache Sombre

### Classification de Sudarsky (par température)
- [x] **Classe I** — nuages d'ammoniac (froide, < 150 K) : blanc-tan
- [x] **Classe II** — nuages d'eau (~250 K) : blanc brillant, albédo élevé
- [x] **Classe III** — sans nuage (~350–800 K) : azur (diffusion Rayleigh), peu de détails
- [x] **Classe IV** — alcalins (Na/K, ~900–1500 K) : sombre, profond, faible albédo
- [x] **Classe V** — silicates/fer (> 1500 K) : Jupiter chaud très réfléchissant

### Variantes
- [x] **Jupiter chaud** (hot Jupiter inflé, bandes rouges)
- [x] **Géante de méthane** (teinte verte)
- [x] **Géante de soufre** (jaune)
- [x] **Naine brune** (brun-rouge sombre, fortement bandée)
- [x] **Sub-Neptune / mini-Neptune** (plus petite, voilée de brume)
- [x] **Géante d'hélium** (post-évaporation, blanchâtre)
- [x] **Naine brune L / T / Y** (sous-classes : L poussiéreuse rouge, T méthane magenta, Y froide quasi-noire)
- [x] **Neptune chaud** (hot Neptune, bleu-gris érodé voilé)
- [x] **Géante de carbone** (riche en carbone, suie/noir mat)
- [x] **Proto-géante chaude** (jeune, rougeoyante, forte émission thermique)
- [x] **Géante rayée extrême** (contraste de bandes très élevé, profil de jets)

---

## Partie B — Features à développer (le « même travail » que les telluriques)

- [x] **Bandes** : jets latitudinaux + turbulence advectée animée + festons (curl-like).
- [x] **Grande Tache** : tempête détaillée (spirale interne + grain fin + cœur + liseré, flot spiralé autour).
- [x] **Tempêtes multiples** : champ d'ovales clairs + cyclones sombres épars (`tempetes`).
- [x] **Cyclones polaires** : amas de tourbillons aux deux pôles (cellules Worley).
- [x] **Aurores polaires** : anneaux émissifs scintillants aux pôles, côté nuit (`aurore`).
- [x] **Émission thermique nocturne** : Classe IV/V + naine brune rougeoient côté nuit (structurée par les bandes).
- [x] **Couche de brume** (sub-Neptunes) : voile qui adoucit/efface les bandes (`brume`).
- [x] **Tilt d'axe visible** : bandes inclinées via `axe` (Uranus pole-on, Sub-Neptune incliné).
- [x] **Profil latitudinal type Jupiter** (`jet_profil`) : large Zone Équatoriale + ceintures NEB/SEB, SEB brique hôte de la Tache, bande ivoire sous la Tache.
- [x] **Brume polaire** (`g_pole`) : calotte bleu-gris/olive feutrée structurée en cyclones Worley (remplace l'aurore bleue saturée).
- [x] **Bandes détaillées** : zones laiteuses (flocons), ceintures marbrées (filaments chocolat/saumon/ocre), bruit étiré horizontalement (jets zonaux).
- [x] **Grande Tache intégrée** : cœur orange spiralé, bords beige rosé opaques, collier + sillage crème, bord irrégulier fondu dans les turbulences.
- [x] **Limb darkening** : assombrissement + désaturation sur tout le contour (volume 3D).
- [x] **Anneaux variés** (`anneau_style`) : Saturne dense + lacunes Cassini/Encke, Uranus fins/étroits verticaux, Neptune arcs partiels (Adams), débris en amas. Galerie : caméra reculée + léger plongé pour cadrer l'anneau.
- [ ] **Lunes/ombres** : ombre d'une lune projetée sur les bandes (plus tard).
- [x] **Scintillement de bandes** animé : remplacé en mieux par la **rotation
      différentielle** V2 (les bandes glissent réellement, advection u(φ)).
- [x] **Grande Tache Blanche** statique (§ 6 bis V2) : preset rare
      « Tempete planetaire (GTB) » + ~6 % des géantes classiques aléatoires.
- [x] **Chapelet d'ovales** (« string of pearls ») : type de vortex dédié.

---

## Partie C — Idées / à trier

- [x] Géante « rayée » extrême (contraste de bandes très fort) — preset, 9 paires de jets.
- [x] Géante jeune chaude qui rougeoie (proto-géante) — preset.
- [x] Géante à anneau de débris récent (anneau brillant, irrégulier) — preset.
- [ ] Éclairs côté nuit (idée notée en V2 § 6 bis, post-V2).
- [ ] Anneaux V2 : ombres croisées planète↔anneau, éclairage face nuit (chantier suivant).
