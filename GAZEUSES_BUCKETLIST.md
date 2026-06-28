# Gas Giant Bucket List

Catalogue et chantiers pour les géantes gazeuses. Même approche que les telluriques :
un modèle paramétrique (uniforms hot-reloadables) + un catalogue de presets nommés,
visualisés dans la **Galerie - Géantes gazeuses** (4e bouton de l'accueil).

Légende : `[ ]` à faire · `[x]` fait · `[~]` partiel · `(R)` rare

---

## Bilan (à date)

Galerie gazeuse en place avec un **catalogue de départ (~13 presets)**. Le shader gazeux
existant gère déjà : **bandes** étirées + domain warping, **grande tache** (vortex façon
tache rouge), **ovales blancs** épars, **vortex polaire polygonal** (hexagone de Saturne),
**anneaux** (2 passes), halo atmosphérique, graine unique par planète.

Paramètres actuels (`Apparence`) : `couleur/2/3` (ceintures/zone/clair), `band_scale`,
`warp_amt`, `seed`, `poly_cotes` (vortex polaire), `tache_*`, `anneau_*`, `axe`, `atmo`.

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
- [ ] **Scintillement de bandes** animé plus marqué (advection façon curl-noise).

---

## Partie C — Idées / à trier

- [ ] Géante « rayée » extrême (contraste de bandes très fort).
- [ ] Géante jeune chaude qui rougeoie (proto-géante).
- [ ] Géante à anneau de débris récent (anneau brillant, irrégulier).
- [ ]
