# Noises & patterns pour géantes gazeuses — recherche

Synthèse des techniques utilisées par les générateurs de géantes gazeuses, avec ce
qu'on fait déjà (`planete.frag.glsl`, branche `type_p > 0.5`) et les pistes à tester.

## 1. Domain warping (Inigo Quilez) — ✅ en place
Échantillonner le bruit *à une position elle-même déplissée par du bruit* :
`fbm(p + k·fbm(p + k·fbm(p)))`. Donne les volutes/tourbillons. On l'a en 2 niveaux
(`q1`, `q2`) avec advection temporelle (`+ t`). Réf : iquilezles.org/articles/warp.

## 2. Étirement latitudinal des coordonnées — ✅ en place
Pour transformer le bruit isotrope en **bandes**, on compresse l'axe zonal avant de
sampler (`seededSamplePoint.y *= 2.5` chez Barth ; chez nous `dh = dlat + (dd-dlat)*0.5`).
Les nuages s'allongent le long des jets → bandes. C'est la clé du look « rayé ».

## 3. Modèle de bandes à DOUBLE OFFSET — ✅ en place
Au lieu de bandes en `sin(latitude)` (périodiques, régulières), Barthélemy utilise
**deux décisions de couleur** décalées par le warping :
```
colorDecision1 = fbm(latitude + warping, seed)
colorDecision2 = fbm(latitude - warping, seed)
color = mix(c1, dark, smoothstep(0.4,0.6, colorDecision1))
color = mix(color, c2, smoothstep(0.2,0.8, colorDecision2))
```
→ bandes **organiques, non périodiques**, largeurs variables. Pourrait remplacer/épauler
notre `sin(bc)` pour casser la régularité résiduelle. Le `smoothstep` resserre les
transitions (bandes nettes).

## 4. Curl noise / advection de champ (Gaseous Giganticus) — ✅ version cheap en place
La référence pour Jupiter : on advecte des particules le long d'un **champ de vecteurs
sans divergence** (le *curl* d'un bruit : `curl = (∂n/∂y, -∂n/∂x)`). Les particules
tracent des filaments et vortex ultra-réalistes. Coûteux (simulation / render-to-texture).
Notre turbulence advectée (`q2 + t`) en est une approximation cheap. Piste cheap :
calculer un curl 2D à partir des gradients de `fbm` et l'ajouter au warp pour des
tourbillons plus « fluides » (moins de blobs étirés). Réfs : Parallel Cascades (curl
flow Unity), smbc/Gaseous Giganticus (github).

## 5. Worley / cellular noise — ✅ en place (pôles)
Cellules de Voronoï → amas de cyclones. On l'utilise aux pôles (`wpole`) et pour
`cyclones_pol`. Peut aussi servir à des poches de tempêtes éparses dans les bandes.

## 6. Ridged noise — 🔜 à tester (filaments)
`ridged = 1 - |2·fbm - 1|` (replie le bruit) → crêtes fines et nettes. Idéal pour les
**filaments brillants** dans les zones et les festons. Plus marqué que le `fbm` simple.

## 7. Palette HSV à teintes complémentaires — ✅ en place (génération aléatoire)
Pour `apparence_gazeuse()` (géantes random), tirer la teinte en **HSV** plutôt qu'en RGB :
- teinte1 = normal(µ, σ) ; teinte2 = teinte1 + 180° (complémentaire) ; sombre = teinte libre, V bas.
Donne des combinaisons harmonieuses au lieu de couleurs random parfois laides.
Box-Muller pour une distribution normale (contrôle de la variété).

## 8. Détails déjà couverts chez nous
Festons bleu-gris (cisaillement), marbrures chocolat/saumon/ocre (belts), zones
laiteuses (flocons ammoniac), tache rouge intégrée + sillage, brume polaire (g_pole),
limb darkening + désaturation, profil de jets type Jupiter (`jet_profil` : EZ + NEB/SEB).

## Priorités proposées
1. **Double-offset (§3)** — casse la régularité des bandes, peu de code.
2. **Ridged filaments (§6)** — relief fin dans zones/festons.
3. **Curl warp cheap (§4)** — tourbillons plus fluides.
4. **HSV palette (§7)** — variété des géantes générées aléatoirement.

---

## Post-scriptum V2 (juillet 2026)

La V2 (CONCEPTION_GAZEUSES_V2.md) a rebattu ces cartes :
- Le **double-offset (§3)** est remplacé pour la structure par le **profil zonal
  1D précalculé** (`zonal.rs`) — les bandes viennent d'une somme de gaussiennes
  de jets + vorticité, le fbm ne fait plus qu'onduler les frontières (dec2).
- Le **curl warp (§4)** est conservé, et l'advection « cheap » est devenue une
  **vraie rotation différentielle** par u(φ) — la référence Gaseous Giganticus
  est approchée par transport zonal réel plutôt que par simulation.
- La **palette HSV (§7)** est en place, poussée plus loin : 8 teintes dérivées
  CPU (`palette.rs`) + archétypes structurels dans `apparence_gazeuse()`.
- Le **ridged (§6)** n'a finalement pas été utilisé (les filaments passent par
  des seuils multiples sur 2 fbm partagés) — reste une piste si besoin.
- Le Worley (§5) est passé en **projection azimutale** aux pôles.
