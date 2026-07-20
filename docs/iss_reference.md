# Référence ISS — inventaire, fusions et chevauchements

But : servir de **vérité terrain** pour le preset `preset_iss` et, ensuite, pour
rapprocher le générateur. Basé sur la vue éclatée NASA + recherche (voir Sources).

> Convention repère (comme dans le code) : **+Z fore, −Z aft, +Y zénith (vers la
> poutre), −Y nadir (vers la Terre), ±X bâbord/tribord**. La **poutre est au
> zénith** du segment habité, reliée par Z1/S0 — elle **ne traverse pas** les
> modules.

---

## 1. Inventaire réel

### 1.1 Poutre intégrée (ITS) — une seule barre bâbord↔tribord, au zénith
Ordre : `P6 P5 P4 P3 P1  S0  S1 S3 S4 S5 S6`. Reliée au segment habité par **Z1**
(sur le zénith d'Unity) et **S0** (sur le zénith de Destiny).
- **Arrays solaires (SAW)** : 4 paires = **8 ailes**, aux **extrémités** :
  P4/P6 (bâbord) et S4/S6 (tribord). Elles tournent dans le plan orbital (BGA).
- **Radiateurs HRS** : 3 panneaux blancs sur **S1** et **P1** (inboard), déployés
  vers le **nadir** (perpendiculaires aux arrays). + radiateurs photovoltaïques
  (PVR) sur P4/S4.

### 1.2 Segment US (USOS) — sous la poutre (nadir), aligné fore↔aft
- **Unity (Node 1)** : hub. Zénith→Z1(poutre) ; bâbord→Tranquility ; tribord→sas
  Quest ; aft→segment russe (via PMA-1) ; fore→Destiny ; nadir→PMM(Leonardo).
- **Destiny (US Lab)** : fore d'Unity ; zénith→S0(poutre).
- **Harmony (Node 2)** : fore de Destiny ; tribord→Columbus ; bâbord→Kibō ;
  fore+zénith→PMA-2/3 + IDA (docking) ; nadir→cargo.
- **Tranquility (Node 3)** : bâbord d'Unity ; nadir→**Cupola** ; porte aussi
  **PMM**, **BEAM**, **Bishop**.
- **Columbus** (labo ESA) : tribord de Harmony.
- **Kibō (JEM)** : bâbord de Harmony = **PM** (labo) + **ELM-PS** (logistique au
  zénith) + **EF** (plateforme exposée) + **Bartolomeo**.
- **Quest** (sas) : tribord d'Unity.

### 1.3 Segment russe (ROS) — dans le prolongement aft
- **Zarya (FGB)** → **Zvezda (SM)** (arrays russes sur les deux).
- **Poisk (MRM2)** : zénith de Zvezda. **Rassvet (MRM1)** : nadir de Zarya.
- **Nauka (MLM)** : nadir de Zvezda → **Prichal** (nœud sphérique) au nadir.

---

## 2. Mapping actuel → composants (et ce qui est **fusionné**)

| Élément réel | Preset actuel | Problème |
|---|---|---|
| S0 + Z1 + Unity | **un seul `Noeud` (hub)** | 3 éléments **fusionnés** ; la poutre traverse le cœur au lieu d'être au zénith |
| Poutre P6..S6 | 2 demi-treillis sur ±X du hub | OK en silhouette, mais jonction = fusion S0/Z1 |
| Kibō (PM+ELM+EF+Bartolomeo) | **1 module Hublots** | 4 éléments fusionnés en 1 |
| Node3 + Cupola + PMM + BEAM + Bishop | **1 nœud nu + 1 Cupola** | Tranquility manquant (nœud nu), PMM/BEAM/Bishop manquants |
| PMA-2/3 + IDA (docking fore Node2) | **1 module habitat « av »** | adaptateur de docking **manquant** (mis un module à la place) |
| Quest (sas) | absent | **manquant** |
| MRM1/MRM2 (radiaux) | 3 modules terminaux sur un nœud | agencement radial faux |

**Constat « modules doubles fusionnés »** : plusieurs modules du preset
représentent **plusieurs modules réels** à la fois (hub = S0/Z1/Unity ; Kibō
seul ; Node3 grappe) → d'où l'impression de modules qui se chevauchent/se
confondent.

---

## 3. Modules/pièces théoriques à créer — **état phase 2**

1. [x] **Adaptateur de docking (PMA/IDA)** — créé : `Composant::Adaptateur`
   (tronc de cône, deux écoutilles axiales de profils différents).
2. [x] **Adaptateur de profil (P1↔P2)** — **même composant** `Adaptateur`.
3. [~] **Boom court « Z1 »** — réutilise un **treillis court** (pas de composant
   dédié) ; le décalage au zénith se fera dans le preset (phase 3).
4. [x] **Sas (Quest)** — créé : `VarianteModule::Sas` (écoutille EVA + main
   courante).
5. [~] **ELM-PS / EF / Bartolomeo** — réutilisent des **modules courts** /
   appendices `Surface`.
6. [~] **PMM / BEAM / Bishop** — BEAM = `Gonflable` (déjà là) ; PMM = module
   court ; Bishop = petit `Sas`.

Nouveaux composants ajoutés en phase 2 : **`Adaptateur`** + variante **`Sas`**.
Le reste réutilise l'existant.

---

## 4. Chevauchements, échelles et orientations à corriger

- **Poutre vs cœur** : aujourd'hui la poutre traverse le hub (fusion S0/Z1) →
  chevauchement visuel poutre/modules au centre. **Fix** : poutre décalée au
  zénith via un boom Z1, ne traversant pas les modules.
- **Radiateurs** : actuellement déployés en **±Z** (vers les deux empilements) →
  chevauchent le stack. Réel : radiateurs vers le **nadir (−Y)** uniquement,
  **perpendiculaires** aux arrays. **Fix** : arrays sur un axe (plan orbital),
  radiateurs sur l'axe nadir — jamais le même axe, jamais vers les modules.
- **Arrays vs radiateurs** : séparer les axes (arrays et radiateurs sur des
  directions orthogonales) pour supprimer les recouvrements sur la poutre.
- **Échelles** : vérifier l'espacement des ports `Surface` de la poutre vs la
  taille des arrays (longueur 6.5) pour qu'elles ne se touchent pas entre bandes
  voisines ; ajuster `TREILLIS_PAS_AILE` / longueurs si besoin.

---

## 5. Plan (dans l'ordre)

1. **(ce doc)** inventaire + fusions + chevauchements. ✅
2. **Créer** les modules/pièces manquants (§3). ✅ `Adaptateur` + `Sas` ajoutés ;
   le reste réutilise l'existant.
3. **Refaire `preset_iss`** fidèle à §1. ✅ Poutre déportée au **zénith via boom
   Z1** (fini le chevauchement poutre/cœur) ; nez **PMA/IDA** (`Adaptateur`) à
   l'avant US ; **Sas Quest**, **PMM** et **radiateurs nadir** sur le cœur ;
   arrays aux bouts / radiateurs inboard sur la poutre. *Limite connue* : les
   gros radiateurs de poutre restent fore/aft (la poutre n'offre des ports
   `Surface` que sur ±X_local) → le vrai nadir n'est rendu que par les
   radiateurs montés sur les modules. Piste : ajouter des ports `Surface` ±Y à la
   poutre (bénéficierait aussi au générateur).
4. **Adapter le générateur** à partir des règles dégagées (topologie décalée via
   boom, zonage arrays/radiateurs, axes orthogonaux, adaptateurs, symétrie).
   ← **phase 4**.

---

## 6. Passe de rigueur (audit du preset)

Positions monde de chaque pièce tracées pour vérifier (a) la couverture de
l'inventaire, (b) l'absence de chevauchement imposant un nouveau composant.

### Couverture de l'inventaire
**Présent** : Z1, S0 (nœud), poutre P/S (2 demi), 8 arrays, radiateurs HRS (poutre
+ nadir modules), Unity, Destiny, Harmony, Tranquility, Cupola, Columbus, Kibō,
Quest (Sas), **PMM**, **BEAM** (Gonflable, ajouté à cette passe), nez PMA/IDA
(Adaptateur), Zarya, Zvezda + arrays russes, nœud russe + 3 MRM (Poisk/Rassvet/
Nauka+Prichal).

**Omissions assumées** (micro-modules / sous-parties, hors objectif silhouette
low-poly) : Bishop, ELM-PS & EF & Bartolomeo (sous-parties de Kibō représentées
par un seul module), PMA-1/3 & IDA distincts (un seul nez adapter). **Aucune** ne
requiert un nouveau composant (réutilisation de l'existant si un jour souhaité).

### Chevauchements
- **Collision trouvée & corrigée** : PMM sur le port nadir du cœur (hub −Y)
  intersectait la **Cupola** (qui se rabat sous le cœur ≈ (0,−2.6,+0.2)). Fix :
  PMM déplacé sur **hub −X** (bâbord, dégagé). Erreur de placement — **pas** un
  besoin de composant.
- Autres proximités (Node3/Destiny, BEAM/Destiny) = voisins diagonaux avec jeu
  (≥ 0.4 U), sans intersection.
- Les radiateurs nadir sur modules sont posés par `appendice_sur_module` qui
  **cible la face par direction monde** → jamais de radiateur mal orienté (au
  pire aucun placé), donc pas de collision par mauvaise orientation.

**Conclusion** : aucun chevauchement du preset n'implique la création d'un
nouveau composant. Le seul manque *structurel* connu reste les ports `Surface`
±Y de poutre (radiateurs de poutre vraiment nadir) — amélioration, non blocage.

### 6 bis. Overlay de numéros + repli de chaîne corrigé
- **Overlay** : touche **N** dans la vue STATION → chaque pièce affiche son
  **index d'assemblage** (projection écran), pour pointer les pièces à corriger.
- **Bug trouvé** (via les numéros + calcul des positions monde) : la chaîne US
  se **repliait** sur elle-même (Destiny et « av » au même point, Harmony sur le
  FGB russe). Cause : les nœuds **basculent** (demi-tour) à l'accouplement, donc
  docker par *index* de port (« −Z ») ne pointe pas vers −Z monde.
- **Fix** : `porter_vers(hote, dir_monde, …)` docke sur le port dont l'avant
  **monde** vise `dir` (aft/fore/nadir/zénith/bâbord/tribord). Chaîne US, russe et
  cœur réécrits ainsi. Vérifié hors-Rust (réplique Python de l'accouplement) :
  **plus aucune paire de modules/nœuds < 2.6 U**. Toujours **aucun** nouveau
  composant requis — c'était un défaut de chaînage.

---

## Sources
- [Integrated Truss Structure — Wikipedia](https://en.wikipedia.org/wiki/Integrated_Truss_Structure)
- [Integrated Truss Structure — NASA](https://www.nasa.gov/international-space-station/integrated-truss-structure/)
- [Harmony (Node 2) — Wikipedia](https://en.wikipedia.org/wiki/Harmony_(ISS_module))
- [Tranquility (Node 3) — Wikipedia](https://en.wikipedia.org/wiki/Tranquility_(ISS_module))
- [Unity (Node 1) — Wikipedia](https://en.wikipedia.org/wiki/Unity_(ISS_module))
- [US Orbital Segment — Wikipedia](https://en.wikipedia.org/wiki/United_States_Orbital_Segment)
