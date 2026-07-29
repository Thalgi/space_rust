# Suivi — Stations spatiales

> Fusion de deux anciens documents : la passation du chantier générateur
> (`generateur_etat_des_lieux.md`) et l'audit de référence de l'ISS
> (`iss_reference.md`). Conception :
> [`docs/conception/stations.md`](../conception/stations.md).

---

## Priorités immédiates (2026-07-29) — à corriger avant tout le reste

Constat d'une revue de code sans complaisance sur l'état actuel du dépôt
(travail en cours non commité). Ordre = urgence, pas ordre de fichier.

1. ✅ **Committer l'état actuel avant de toucher à autre chose.** Fait : le
   gros du travail non commité (suppression des anciens `vaisseau/*.rs`,
   `peintre.rs`/`maillage.rs`, réorg des docs) est passé en deux commits
   (`d87e046`, `136c7e1`), plus un commit de checkpoint (`3c16df2`) pour la
   Partie E elle-même. Point de reprise en place avant le découpage.
2. ⏳ **`cargo clippy` ne compile toujours pas.** `src/planete/terrain.rs:154`
   déclenche toujours `clippy::approx_constant` (deny-by-default) sur
   `0.318_309_9` au lieu de `std::f32::consts::FRAC_1_PI` — le commit
   « cargo clippy fix » (`136c7e1`) a réduit les avertissements (63 → ~45)
   mais **pas** corrigé cette erreur bloquante ni le point 5. Toujours
   trivial, toujours pas fait.
3. 🔨 **`composant.rs` (toujours 1 seul fichier, 5 fonctions dispatch
   géantes)** : découpage en 12 modules conçu en
   [`conception/stations.md`](../conception/stations.md) Partie E.2, **pas
   encore commencé** — E.3 (composite, ci-dessous) est passé devant sur
   décision utilisateur. **Audit de tests fait avant de commencer le
   chantier composant** (2026-07-29) : 9 des 19 variantes n'avaient aucun
   test (`Charpente`, `RadiateurMega`, `Motrice`, `BlocMoteur`, `Reservoir`,
   `MoteurAntimatiere`, `Coiffe`, `ReacteurAntimatiere`, `TreillisHexagone`)
   — 9 tests de fumée ajoutés (121 → 130 tests).
4. ✅ **Composite `Composant::SousEnsemble` (Partie E.3)** : fait
   (2026-07-29). `Chantier::figer` gèle un sous-arbre en brique réutilisable ;
   le trait `Peintre` a gagné `empiler_transforme`/`depiler_transforme`
   (nécessaire, non anticipé dans la conception initiale) ; `Composant`/
   `Piece` ont perdu `Copy` (120 sites cassés, absorbés via des signatures
   empruntées + `cargo fix` + nettoyage — détail en Partie E.3). 130 → **135
   tests**.
5. **Fichiers morts** : `src/ecran/briques.rs` (107 lignes) et
   `src/ecran/vaisseaux.rs` (93 lignes) ne sont plus déclarés dans
   `ecran/mod.rs` depuis la réorganisation du menu — ne compilent plus dans
   le binaire, jamais supprimés. Suppression triviale, toujours pas faite.
6. **`astre::Astre` a une variante trop grosse** (`clippy::large_enum_variant`,
   `Planete { app: Apparence, .. }` → 376 octets pour tout l'enum). Hors
   sujet stations, toujours pas corrigé.

Une fois §E.2 de la conception implémenté et les points 2/5/6 traités,
reprendre le fil du générateur (Partie A ci-dessous) et le chantier
« stockages de carburant » de l'ISV
([`conception/stations.md`](../conception/stations.md) Partie D).

---

## Partie A — Générateur : état des lieux et reprise


Document de passation. Objectif : rendre la **silhouette** des stations générées
conforme à celle des presets faits à la main.

---

### 1. Le problème

`generer()` (dans `src/vaisseau/generateur.rs`) produit des stations dont la
silhouette ne ressemble pas à celle attendue. La cible est le preset ISS
(`preset_iss()`, vue 1) :

> une **longue poutre horizontale** portant **tous** les panneaux solaires, un
> **boom** vertical court, et une **grappe habitée compacte** nettement séparée
> en dessous. Les modules sont tous reliés entre eux et arrimés à cette
> structure.

Formulation de l'utilisateur : *« il y a une barre de structures avec tous les
panneaux solaires et une barre de modules perpendiculaire, séparées du tout ; un
côté un peu organique où les modules doivent tous être reliés ensemble et
arrimés à une structure qui a des panneaux solaires. »*

Griefs successifs constatés à l'écran :
1. structures éparpillées un peu partout au lieu d'une seule barre ;
2. nœuds à 6 sorties en bout de chaîne, ne desservant rien ;
3. plus aucune structure sur une partie des graines ;
4. une seule structure, noyée au milieu de la station ;
5. poutre **vide** (aucun array), panneaux minuscules restés sur les modules ;
6. les deux demi-poutres posées **bout à bout** du même côté.

---

### 2. Contraintes du moteur (à connaître avant de toucher au code)

Ces trois points ont chacun causé un bug. Ils ne sont pas évidents à la lecture.

- **`Chantier::poser` vérifie l'égalité stricte des profils**
  (`hote.profil.compatible(montage.profil)`), contrairement à `montage::poser`
  utilisé par les presets, qui ne vérifie rien. Une poutre `P2` posée sur un
  nœud `P1` est donc **refusée en silence** : `poser` renvoie `false` et le port
  reste libre, prêt à être garni par autre chose. Il faut intercaler un
  `Composant::Adaptateur { grand: P2, petit: P1 }`.
- **`Chantier` consomme ses ports par `swap_remove`**, ce qui **réordonne** la
  liste `libres`. On ne peut donc pas se fier à l'ordre d'insertion (`rposition`
  ≠ « la pièce que je viens de poser »). Sélectionner par **critère
  géométrique** : hauteur, direction, proximité.
- **`accoupler` impose un demi-tour** : après montage, « le port −Z » d'un nœud
  ne pointe pas vers −Z **monde**. Toujours viser une **direction monde**
  (`port_vers`), jamais un index de port. Cf. conception/stations.md, Partie C §1 bis.

---

### 3. Étalon de taille (mesuré)

`preset_iss()` = **49 pièces, coût 371**. La complexité 2 doit valoir une ISS.

```rust
let budget = 120.0 + (c as f32 - 1.0) * 250.0; // 1:120  2:370  3:620  4:870
```

Coûts unitaires utiles (`Composant::cout`) : module 5 · nœud `1 + 2×sorties`
(Six = 13, T = 7) · panneau 6 · treillis `2 + longueur` · adaptateur 3.

Ordre de grandeur de la structure de puissance à c=2 : cœur ≈ 23, boom ≈ 10,
jonction T = 7, 2 adaptateurs = 6, 2 poutres ≈ 34 → **≈ 80**.

---

### 4. Grammaire actuelle

`generer()` enchaîne cinq rôles explicites (approche « grammaire de formes ») :

1. **cœur pressurisé** — `coeur_iss` (épine ±Z) ou `coeur_mir` (grappe radiale).
   Racine toujours `Sorties::Six`, pour garantir une face zénith libre ;
2. **`greffer_structure_puissance`** — boom `P1` (`4.5 + 1.8·c`), jonction
   `Sorties::T` montée par sa tige (index 2), puis pour chaque bras :
   adaptateur `P1→P2` puis poutre `P2` (`8 + 3.5·c`). **Universelle** : posée
   quelle que soit l'ossature ;
3. **`habiller_surface(.., poutres = true)`** — arrays posés **tout de suite**,
   tant que le budget est intact ;
4. **`brancher`** ×c — croissance des modules uniquement (aucun treillis ici),
   sous **plancher de budget** (20 %), avec `corridor_libre` qui interdit la
   pousse vers le zénith pour ne pas noyer le boom ;
5. **`terminer_extremites`** puis **`habiller_surface(.., poutres = false)`**.

Zonage des appendices sur une poutre : par **éloignement du centre** de la
station (`t` normalisé sur l'étendue des ports de cette poutre) — `t > 0.62`
array, `t < 0.34` radiateur, entre les deux caisson d'équipement.

Helpers : `port_vers` (direction monde), `port_le_plus_haut` (genre + profil),
`index_port` (repérage par position), `sur_pressurise`, `corridor_libre`.

---

### 5. Corrections appliquées et validées (session 2026-07-21)

Le lot précédent (adaptateur `P1→P2`, sélection du port `P2` par proximité,
deux passes d'habillage, plancher de budget, arrays agrandis) a été **validé
analytiquement** : projections ASCII des pièces (vue de face et de dessus,
comparées au preset ISS) + dump des positions, sans passer par l'écran. La
structure de puissance sort correcte : deux demi-poutres opposées, panneaux aux
extrémités, radiateurs en pied.

Corrections supplémentaires apportées ensuite :

- **le projet ne compilait pas** : `Chantier::budget_restant` appelait
  `.restant()` sur un `Option<Budget>` (corrigé, `f32::INFINITY` sans budget) ;
- **corridor étanche** : `brancher` ne filtrait le corridor que pour la pousse
  radiale — la **ramification** (`rposition` sans filtre) et le **prolongement
  axial** laissaient les modules remonter jusqu'à toucher la poutre
  (tours M+nez+cargo au zénith). Les trois chemins passent par `corridor_libre`,
  ainsi que `terminer_extremites` ;
- **boom nu** : une « poutre » à habiller est un treillis **P2** ; le boom (P1)
  n'est plus garni (il ressemblait à un mât de sapin de Noël au ras des
  modules) ;
- **jonction nue** : le nœud `T` expose des faces `Surface` que la 2ᵉ passe
  garnissait (radiateur perché au sommet du boom, vu graine 9) — exclu ;
- **plus aucun grand panneau hors poutre** : `cle_surface` ±X donnait des
  arrays plaqués aux modules (grief 5) → radiateurs/antennes alternés, et ~45 %
  des groupes de faces restent nus (les stations réelles ont de la coque vide) ;
- **zonage élargi** : arrays sur la moitié externe de la poutre (`t > 0.5` au
  lieu de 0.62) → deux paires par demi-poutre, comme l'ISS.

**Symétrie de la structure** (demande utilisateur, même session) :

- l'habillage des poutres est groupé par bande d'éloignement **toutes poutres
  confondues** (plus par tronçon) : les deux demi-poutres étant miroir, un seul
  tirage garnit les deux côtés à l'identique ;
- la croissance radiale de `brancher` tire par **paires de faces opposées**
  (même chance, même module pour ±axe d'une même pièce) ;
- la ramification pose le **même module sur les deux faces ±X horizontales** du
  nœud fraîchement posé, au lieu de deux tirages sur des ports arbitraires.

**Quotas d'équipement** (demande utilisateur, même session) : le nombre de
modules n'est plus « tout ce que le budget permet » mais **proportionnel à la
puissance**, façon ISS — et le reste suit l'habitat :

- références chiffrées : preset maison = 13 modules P1 + 6 nœuds pour
  8 arrays, 10 radiateurs ; ISS réelle = 16 éléments pressurisés / 8 ailes /
  14 radiateurs (6 ATCS + 8 PV) / quelques antennes / moteurs à l'arrière de
  Zvezda. D'où `MODULES_PAR_ARRAY = 1.6` et `RADIATEURS_PAR_MODULE = 0.75`
  (constantes en tête de `generateur.rs`, à régler à l'œil) ;
- `generer` compte les ailes posées → quota de modules ; `brancher` boucle
  jusqu'au quota (vérifié pièce à pièce, plus de dépassement massif par passe)
  ou jusqu'au plancher de budget — c'est là que la complexité décide de ce qui
  est **atteignable** ;
- après la grappe : quota de radiateurs (ceux de la poutre comptent) et
  d'antennes (`1 + modules/8`) servis par `habiller_surface`, groupes entiers
  seulement (la symétrie ne casse jamais) ; le reste des faces reste nu ;
- **propulsion** : un moteur principal au bout pressurisé le plus en aval
  (TuyereCloche en Historique/Russe, ionique/Hall/VASIMR en Futuriste) ;
- les **arrays sont servis en premier** dans l'habillage de poutre (tri par
  catégorie) : posés en dernier, l'anti-collision les refusait parfois au
  profit des caissons (graine 7 : 4 ailes au lieu de 8) ;
- conséquence assumée : le budget est un **plafond**, plus une cible — une
  c=2 sort vers 210-240 de coût (l'ISS preset = 371, plus riche en nœuds et
  petits panneaux russes que le générateur).

**Échelle par complexité** (demande utilisateur, même session) :

- **c=4 : structure en H** — tête en croix (nœud Six monté par sa face +Y, ses
  écoutilles axiales restent horizontales) au sommet du boom, une traverse P1
  (5,5) de chaque côté, une **barre complète au bout de chaque traverse** =
  deux barres parallèles, deux fois plus d'ailes, grappe dimensionnée en
  conséquence par les quotas. La traverse est calibrée pour que les deux barres
  passent l'anti-collision (sphères englobantes : demi-poutres de 12 → ~15
  d'écartement). Repli en barre simple si la tête ne se pose pas ;
- `poser_jonction_et_bras` factorisé (T + 2×(adaptateur+demi-poutre)), toutes
  les sélections par **`origine`** — avec deux jonctions à la même hauteur,
  « le port le plus haut » ne désigne plus rien ;
- la grappe ne pousse pas sur les nœuds d'altitude (tête du H, jonctions) :
  filtre `pos.y < 2` dans `brancher` et `terminer_extremites` ;
- **pièces techniques** : les bandes ±X de la grappe alternent
  radiateur/antenne/**caisson** (racks, avionique), caissons contingentés à
  `1 + modules/4` (les caissons de poutre, type ELC, ne comptent pas).

**Garde-fou** : nouveau test `silhouette_generee_converge` (2 ossatures ×
complexités 2-4 × 12 graines ; à c=4 : H = 4 tronçons, 2 de chaque côté de
chaque axe) qui vérifie : exactement deux demi-poutres P2
opposées, grappe entièrement sous la barre, tous les grands panneaux sur la
poutre, boom et jonction nus, **chaque appendice de la barre a son jumeau
miroir** (même type, position réfléchie), et **proportions ISS** (1 à 2,5
modules par aile, 0,4 à 1,2 radiateur par module, ≥ 1 propulsion). C'est le filet contre les `poser` silencieux du
§2. `cargo test` passe (121 tests au 2026-07-28).

Reste à faire : **validation visuelle** (couleurs, proportions, styles Russe et
Futuriste, complexités 1 et 4 — c=1 est presque tout structure, à juger à
l'œil), puis phase 4 de la Partie B ci-dessous (ports `Surface` ±Y sur la poutre,
attache mi-poutre).

---

### 6. Prompt de reprise

> Projet Rust + macroquad (`C:\Users\Daexion\Desktop\space_rust`), jeu qui
> simule des systèmes solaires ; on y construit un générateur procédural de
> stations spatiales. Lis ce document (`docs/suivi/stations.md`, Partie A
> ci-dessus et Partie B ci-dessous) et
> [`docs/conception/stations.md`](../conception/stations.md).
>
> **Tâche** : faire converger la silhouette de `generer()`
> (`src/vaisseau/generateur.rs`) vers celle de `preset_iss()` — une longue
> poutre horizontale portant tous les panneaux, un boom, et une grappe habitée
> compacte séparée en dessous.
>
> **Méthode attendue** : l'utilisateur exécute `cargo test` et
> `cargo run --release` (accueil → « GENERATEUR PROCEDURAL » pour la vue du
> générateur, « PETITES STATIONS » item 0 pour le preset ISS de référence ;
> touche **G** change la graine, **1-4** la complexité, **N** affiche les
> numéros de pièce, **M** bascule maillage cuit / rendu immédiat).
> Il renvoie captures et messages d'erreur. Demande-lui une capture avant de
> conclure qu'un changement fonctionne : plusieurs bugs de ce chantier
> n'étaient visibles **que** sur l'image, `poser` échouant silencieusement.
>
> **Pièges déjà rencontrés** : voir §2. En particulier, tout `ch.poser(...)`
> qui renvoie `false` laisse un port libre que la suite garnira d'autre chose —
> ce qui produit un résultat plausible mais faux. Vérifie les valeurs de retour.
>
> **Contexte utile** : la calibration de taille est réelle (ISS = 371 de coût,
> affiché en bas de la vue), et les presets `preset_iss` / `preset_mir` /
> `preset_tiangong` / `preset_comsat` / `preset_sonde` servent de références de
> style à ne pas casser.
>
> Écris en français, commente le *pourquoi* et non le *quoi*, et signale
> honnêtement ce que tu n'as pas pu vérifier.

---

## Partie B — Référence ISS : inventaire, fusions et chevauchements


But : servir de **vérité terrain** pour le preset `preset_iss` et, ensuite, pour
rapprocher le générateur. Basé sur la vue éclatée NASA + recherche (voir Sources).

> Convention repère (comme dans le code) : **+Z fore, −Z aft, +Y zénith (vers la
> poutre), −Y nadir (vers la Terre), ±X bâbord/tribord**. La **poutre est au
> zénith** du segment habité, reliée par Z1/S0 — elle **ne traverse pas** les
> modules.

---

### 1. Inventaire réel

##### 1.1 Poutre intégrée (ITS) — une seule barre bâbord↔tribord, au zénith
Ordre : `P6 P5 P4 P3 P1  S0  S1 S3 S4 S5 S6`. Reliée au segment habité par **Z1**
(sur le zénith d'Unity) et **S0** (sur le zénith de Destiny).
- **Arrays solaires (SAW)** : 4 paires = **8 ailes**, aux **extrémités** :
  P4/P6 (bâbord) et S4/S6 (tribord). Elles tournent dans le plan orbital (BGA).
- **Radiateurs HRS** : 3 panneaux blancs sur **S1** et **P1** (inboard), déployés
  vers le **nadir** (perpendiculaires aux arrays). + radiateurs photovoltaïques
  (PVR) sur P4/S4.

##### 1.2 Segment US (USOS) — sous la poutre (nadir), aligné fore↔aft
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

##### 1.3 Segment russe (ROS) — dans le prolongement aft
- **Zarya (FGB)** → **Zvezda (SM)** (arrays russes sur les deux).
- **Poisk (MRM2)** : zénith de Zvezda. **Rassvet (MRM1)** : nadir de Zarya.
- **Nauka (MLM)** : nadir de Zvezda → **Prichal** (nœud sphérique) au nadir.

---

### 2. Mapping actuel → composants (et ce qui est **fusionné**)

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

### 3. Modules/pièces théoriques à créer — **état phase 2**

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

### 4. Chevauchements, échelles et orientations à corriger

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

### 5. Plan (dans l'ordre)

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

### 6. Passe de rigueur (audit du preset)

Positions monde de chaque pièce tracées pour vérifier (a) la couverture de
l'inventaire, (b) l'absence de chevauchement imposant un nouveau composant.

##### Couverture de l'inventaire
**Présent** : Z1, S0 (nœud), poutre P/S (2 demi), 8 arrays, radiateurs HRS (poutre
+ nadir modules), Unity, Destiny, Harmony, Tranquility, Cupola, Columbus, Kibō,
Quest (Sas), **PMM**, **BEAM** (Gonflable, ajouté à cette passe), nez PMA/IDA
(Adaptateur), Zarya, Zvezda + arrays russes, nœud russe + 3 MRM (Poisk/Rassvet/
Nauka+Prichal).

**Omissions assumées** (micro-modules / sous-parties, hors objectif silhouette
low-poly) : Bishop, ELM-PS & EF & Bartolomeo (sous-parties de Kibō représentées
par un seul module), PMA-1/3 & IDA distincts (un seul nez adapter). **Aucune** ne
requiert un nouveau composant (réutilisation de l'existant si un jour souhaité).

##### Chevauchements
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

##### 6 bis. Overlay de numéros + repli de chaîne corrigé
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

### Sources
- [Integrated Truss Structure — Wikipedia](https://en.wikipedia.org/wiki/Integrated_Truss_Structure)
- [Integrated Truss Structure — NASA](https://www.nasa.gov/international-space-station/integrated-truss-structure/)
- [Harmony (Node 2) — Wikipedia](https://en.wikipedia.org/wiki/Harmony_(ISS_module))
- [Tranquility (Node 3) — Wikipedia](https://en.wikipedia.org/wiki/Tranquility_(ISS_module))
- [Unity (Node 1) — Wikipedia](https://en.wikipedia.org/wiki/Unity_(ISS_module))
- [US Orbital Segment — Wikipedia](https://en.wikipedia.org/wiki/United_States_Orbital_Segment)

---

## Partie C — ISV Venture Star : état et manques

> Ajouté le 2026-07-29. Chantier **classe C** (cf.
> [`conception/stations.md`](../conception/stations.md) Partie D). Objectif :
> un ISV *inspiré* d'Avatar, pas une copie au rivet près — la silhouette et
> l'articulation des grandes masses priment sur le détail.

### C.1 Ce qui existe aujourd'hui (mesuré, pas supposé)

> **Mesure du 2026-07-29, après fret, habitat et agrandissement de l'ossature** :
> `preset_isv()` produit **30 pièces, coût 520, rayon englobant 83,3**. Le coût
> est monté de 364 (section propulsion seule) à 472 avec le fret, puis 520 avec
> l'habitat — le vaisseau pèse enfin nettement plus qu'une ISS (371), ce qui est
> le minimum attendu pour un interstellaire.
>
> ⚠️ Le rayon affiché **sous-estime** d'environ 9 % depuis que l'ossature est
> agrandie par une matrice d'échelle : `Composant::rayon_local()` ignore
> l'échelle portée par la transformée cuite. Sans conséquence — il ne sert qu'au
> cadrage caméra, qui garde 35 % de marge.
>
> *(Le tableau ci-dessous liste l'état d'avant la charge utile ; les
> 3 `RatelierCargo` et les 3 `ModuleHabitat` s'y ajoutent — voir §C.3 pour ce
> qui reste.)*

`preset_isv()` (`src/vaisseau/generateur.rs`) produisait **24 pièces, coût 364,
rayon englobant 76,4** — soit, en coût, l'équivalent du preset ISS (371) pour
un vaisseau censé être 30× plus long. C'était le signe que **seule la section
propulsion était construite** :

| Pièce | × | Rôle sur le vaisseau |
|---|---|---|
| `Charpente` | 1 | épine dorsale courbe (P3 base → P1 flèche), longueur 84, anneau hexagonal au pied |
| `RadiateurMega` | 2 | les deux grandes voiles radiantes, inclinées de 5° (pointe vers l'intérieur → tuyères vers l'extérieur) |
| `BlocMoteur` | 2 | caisse collecteur de chaque nacelle |
| `ModuleAxial` (Cœur) | 6 | 3 Cœurs par nacelle (rangée étagée) |
| `Coiffe` | 4 | chapes bombées sur les bouts exposés de Cœur 1 & 2 |
| `ReacteurAntimatiere` + `MoteurAntimatiere` | 2 + 2 | le bloc propulsion antimatière au bout de Cœur 3 |
| `Reservoir` | 4 | cuves sphériques d'hydrogène, 2 de chaque côté (±Z) de l'hexagone |
| `TreillisHexagone` | 1 | second anneau hexagonal, bord à bord avec celui du pied |
| `RatelierCargo` | 3 | **(2026-07-29)** rangées de fret en triforce, enfilées sur la flèche de l'épine (Y ≈ 42,5 à 57), à l'opposé des moteurs |
| `ModuleHabitat` | 3 | **(2026-07-29)** habitat principal en couronne autour de l'épine (Y ≈ 59 à 67), boulonné dessus par ses ferrures |

**Correction d'une note périmée** : la conception (Partie D) annonçait encore
« prochain chantier : stockages de carburant ». Ils sont **faits** (les 4
`Reservoir` ci-dessus). Le vrai reste à faire est ailleurs (§C.3).

### C.2 Référence : anatomie du vrai ISV Venture Star

Le vaisseau réel fait **1 646 m** et se lit en trois blocs (sources en fin de
partie). Point de vocabulaire qui prête à confusion : l'ISV est un **tracteur**
— il *tire* sa charge utile. La section propulsion est donc à l'**avant** en
poussée, et la charge utile derrière, au bout d'une longue épine en tension.

1. **Section propulsion (avant)** — deux moteurs hybrides antimatière/fusion
   **écartés de quelques degrés** de l'axe (pour que les panaches d'échappement
   ne touchent pas la structure), leurs **tours de radiateurs** au-dessus, et
   les **cuves sphériques** d'hydrogène cryogénique (isolées « zéro
   évaporation »).
2. **Épine en tension** — treillis en composite de nanotubes de carbone,
   **long et fuselé**, qui transmet la poussée en tirant. Il porte un
   **bouclier thermique** (l'échappement est « plus chaud que le Soleil ») et
   un **tunnel pressurisé** en son cœur, qui relie l'habitat aux navettes.
3. **Section charge utile (arrière)** — quatre sous-ensembles :
   - **cargo** : 4 rangées × 4 modules × 6 nacelles de fret, manipulées par un
     bras robotisé qui les charge sur les navettes ;
   - **2 navettes TAV** (*Valkyrie*) amarrées à des tunnels d'accès ;
   - **habitat principal** (**fixe**) : 3 gros modules (cryovaults + cuves
     amniotiques), en composites **non métalliques** (le métal produirait du
     rayonnement secondaire sous les rayons cosmiques) ;
   - **2 modules d'équipage rotatifs** — **à distinguer** du précédent : aux
     deux bouts d'une **traverse** perpendiculaire, reliés par des bras, ils
     **tournent** pour la gravité artificielle en croisière et se **replient**
     le long de l'axe pendant les phases d'accélération/décélération.
4. **Bouclier antidébris (IDPS)** — plaques planes anguleuses en avant du
   vaisseau, façon bouclier Whipple étagé (barrières séparées par ~100 m).

*Hors périmètre assumé* : la **voile solaire** de 16 km, déployée uniquement
au départ du système solaire — elle n'est pas là dans la configuration
« en orbite de Pandora » que l'on modélise.

### C.3 Manques, par ordre d'impact sur la silhouette

1. 🟠 **Section charge utile — bien avancée.** **Fret et habitat principal
   posés** (2026-07-29) ; il manque :
   - 2 modules d'équipage **rotatifs** sur leur traverse (à ne pas confondre
     avec l'habitat fixe, qui est fait) ;
   - 2 navettes TAV amarrées.
2. 🟠 **Bouclier antidébris (IDPS)** — les plaques planes en tête ; forte
   signature visuelle, brique neuve (mais simple : des `panneau` inclinés).
3. 🟡 **Proportions** — l'épine fait 84 unités pour un rayon englobant de 76 :
   à refaire quand l'habitat et les modules d'équipage seront posés (le fret
   n'a pas bougé le rayon, l'épine commande toujours).
4. 🟡 **Tunnel pressurisé + bouclier thermique** de l'épine — détails de
   surface, à faire seulement s'ils se voient à la silhouette.
5. ✅ **Test sur `preset_isv`** — fait avec le fret :
   `isv_porte_son_fret_a_loppose_des_moteurs` verrouille la disposition
   **tracteur** (fret nettement à l'opposé des moteurs) et le fait que les
   rangées sont **enfilées sur l'axe**, pas déportées sur un flanc.

### C.4 Ordre de travail proposé

Méthode retenue et qui marche : **la brique d'abord dans la vue BRIQUES**, on
valide sa forme à l'écran, et **seulement ensuite** on l'assemble sur l'ISV
(vue **MEGASTRUCTURES**, item « ISV — CHARPENTE + RADIATEURS + FRET »).

1. ✅ **Grappe cargo** (2026-07-29). **Rien à réutiliser** : `Caisson`/
   `ChargeUtile` sont du vocabulaire ISS (porteurs d'ORU, berceaux FRAM,
   poignées EVA) — mauvaise échelle, mauvaise silhouette. Deux briques neuves :
   - `Composant::NacelleCargo` — conteneur long à section **onigiri**
     (triangle à coins congés, côtés **droits**). Le triangle n'est pas
     décoratif : c'est la forme qui tient, et la seule qui s'empaquette sans
     vides autour d'une épine ;
   - `Composant::RatelierCargo` — une rangée. Deux dispositions selon le
     nombre : **triforce** à 3 (même orientation, pointe contre pointe, creux
     triangulaire au milieu, retenue pour l'ISV) et **couronne** à ≥ 4 (coin
     vers l'axe).

   *Deux pièges rencontrés, corrigés, et verrouillés par des tests* : (a) des
   collerettes posées à ras du corps ont leurs faces coplanaires avec ses bouts
   → z-fighting ; il faut les faire déborder **et** s'enfoncer (même remède que
   les `EMBOUT_*` de module) ; (b) des triangles à coins **congés** ne peuvent
   pas se toucher pointe contre pointe comme des coins vifs — ce sont des
   triangles nus gonflés de ρ, il faut écarter les centres de `r(1−f+2f/√3)`,
   sinon ils se traversent de 0,27 ρ.

   **Gabarit validé à l'œil (2026-07-29)** : `FRET_ECHELLE = 0,70` et
   **3 rangées**. Toute la cote du fret tient dans quatre constantes de
   `generateur.rs` — `FRET_RAYON`/`FRET_LONG`/`FRET_PAS` (gabarit de base, ne
   bougent plus), `FRET_ECHELLE` (le seul chiffre à retoucher pour le rapport
   fret/vaisseau) et `FRET_RANGEES`.

   Le bloc est ancré par son **bord bas** (`FRET_DEBUT_Y = 42,5`, côté
   moteurs), pas par son centre : c'est là que finit le tronçon d'épine nu et
   que commence la charge utile. Conséquence utile — ajouter ou retirer une
   rangée allonge ou raccourcit le bloc **vers le haut**, en libérant
   d'autant l'extrémité. C'est ce qui a permis de passer de 4 à 3 rangées sans
   déplacer les deux premières.

   ⚠️ **Plancher d'échelle ≈ 0,67** : le creux central de la triforce vaut
   `rayon − r_nacelle·(0,5 + f/2)` et rétrécit avec l'échelle, alors que
   l'épine ne bouge pas (flèche fine, ~0,93 hors-tout). À 0,70 il reste 0,98 —
   ça passe, mais de peu. En dessous de ~0,67 les conteneurs **traversent**
   l'épine ; il faudrait alors découpler le rayon de la longueur au lieu de
   tout mettre à l'échelle ensemble.
2. ✅ **Habitat principal** — brique faite **et assemblée** (2026-07-29).

   ⚠️ **Distinction à ne pas perdre** : il s'agit de l'habitat **fixe**,
   solidaire de l'épine. Les **modules d'équipage rotatifs** (gravité
   artificielle) sont une **autre brique**, traitée à l'étape 3 — ne pas les
   confondre sous le mot « cryo ».

   `Composant::ModuleHabitat` reprend la **section onigiri** des nacelles de
   fret, en plus gros : c'est ce qui fait tenir la famille visuelle du
   vaisseau. Écarts voulus par rapport à la nacelle : **pas** de collerettes
   sombres ni de rails d'arête (la coque composite reste franche), trois
   **armatures** aux quarts (¼, ½, ¾), et sur **un seul** côté plat des
   **ferrures d'attache** par lesquelles le module se boulonne à l'épine. Le
   champ `spin` désigne ce côté, donc oriente les ferrures ; `attache` en donne
   la portée (0 = module présenté seul).

   🐛 **Armature : deux fois enfoncée dans la coque avant d'être juste.** Le
   même piège, sous deux formes — sur un triangle **congé**, on ne peut pas
   raisonner comme sur un triangle à coins vifs.
   1. *Version triangulaire* : une barre tendue d'un coin au suivant passe à
      `0,52 r`, alors que le côté plat de la coque est repoussé à
      `r·(0,5 + f/2) ≈ 0,61 r` → la barre plongeait à mi-côté, et il ne restait
      de visible que les goussets sphériques d'angle (« un triangle relié par
      des billes »).
   2. *Version hexagonale posée sur les **points de tangence*** (±60° sur l'arc
      de congé) : les longs côtés tombaient juste, mais la **corde courte** en
      travers d'un coin coupe l'arc et rentre de `ρ·(1 − cos 60°) = ρ/2`.

   Corrigé en plaçant les sommets à **±30°** au lieu de ±60° (la corde ne mord
   plus que de `ρ·(1 − cos 30°)`, sept fois moins) **et** en dérivant l'échelle
   de l'armature au lieu de la choisir à l'œil :
   `pieces::onigiri_hex_echelle_mini()` renvoie le facteur en dessous duquel un
   segment replonge, calculé pour les deux contraintes (le long des faces, en
   travers des coins). Le composant prend cette borne + 4 %.

   Conséquence agréable : l'angle des sommets devient un **paramètre de style
   libre** — le changer recalcule automatiquement l'échelle minimale, donc ne
   peut plus réintroduire le défaut. Vérifié : passer de 30° à 60° laisse le
   test au vert (l'armature s'écarte d'elle-même), alors que forcer l'échelle à
   1,0 le fait rougir. Le test échantillonne le contour et teste
   l'appartenance à la section par somme de Minkowski (distance au triangle nu
   ≤ ρ).

   Ferrures : **deux** longerons écartés plutôt qu'un seul central (deux appuis
   courts tiennent mieux qu'un long bras isolé), chacun à **mi-`attache`** ;
   leurs jambes partent des stations d'armature, pour que l'effort passe dans
   les cadres et non dans la coque nue.

   **Réglages validés à l'écran (2026-07-29)** : longueur **−33 %** (12 → 8) ;
   armature **centrale supprimée** (il n'en reste qu'au ¼ et au ¾) et remplacée
   à mi-longueur par une **bande de repérage jaune** plaquée sur la coque —
   seule couleur franche du vaisseau, elle donne l'échelle et casse le
   monochrome, comme les bandes peintes sur les lanceurs. Bande **élargie ×2**,
   et armatures passées en **gris sombre** : j'avais choisi un métal moyen en
   pensant qu'un ton franc salirait le composite clair — l'écran dit l'inverse,
   c'est le contraste qui fait lire les cadres comme de la structure.

   **Posé sur l'ISV** : grappe centrée à `HAB_CENTRE_Y` (× l'échelle de
   l'ossature), juste au-dessus du fret. L'ordre des sections le long de l'épine
   suit celui du vrai vaisseau — moteurs, épine nue, fret, puis l'habitat le
   plus loin possible des tuyères — et c'est désormais **testé**
   (`isv_porte_son_fret_a_loppose_des_moteurs` vérifie aussi `habitat > fret`).
   Le haut de l'épine reste libre pour les modules d'équipage rotatifs et le
   bouclier antidébris.

### C.5 Ossature agrandie de 20 % (2026-07-29)

Demande : **épine et propulsion +20 % en taille**, fret et habitat inchangés en
taille mais **recalés sur le nouveau gabarit**.

**Pourquoi une mise à l'échelle géométrique et pas des constantes** : l'épaisseur
du treillis, le diamètre des modules Cœur et le gabarit des hexagones viennent
de `Profil`, un enum **discret plafonné à P3** — aucune constante ne peut les
étirer de 20 %. L'ossature est donc bâtie dans son propre `Assembleur`, à
l'échelle 1, puis **reversée agrandie** (`verser_a_echelle`, qui compose une
`Mat4::from_scale` dans chaque transformée cuite, comme `pivoter` le fait pour
une rotation). La charge utile est ajoutée après, à sa taille propre.

**Le conflit géométrique, et sa résolution.** Le creux central de la triforce de
fret était calibré au ras de l'épine (0,98 contre 0,93). Élargir l'épine de 20 %
la porte à 1,12 : les conteneurs la traversaient. Comme le fret ne doit pas
grossir, il a fallu **découpler la taille du conteneur du rayon de la
couronne** — `RatelierCargo` gagne un champ `nacelle` (0 = empilement serré,
comportement d'avant ; > 0 = rayon imposé). La couronne s'ouvre donc pour
laisser passer l'épine pendant que le conteneur garde exactement la taille
validée à l'écran.

**Tout se déduit maintenant du gabarit** plutôt que d'être recopié à la main :
`EPINE` (extension hors-tout de la flèche, × l'échelle) commande `FRET_RAYON`,
`HAB_RAYON` et `HAB_ATTACHE`. Changer `ISV_ECHELLE` suffit ; rien d'autre n'est
à re-régler.

**Serrage de la charge utile contre l'épine (2026-07-29)** — deux constantes de
« jeu », une par famille, qui sont les seuls chiffres à toucher :

- `HAB_JEU` : distance du côté plat d'un module d'habitat à la surface de
  l'épine, ramenée de 0,86 à **0,25**. Les ferrures ne franchissent que ce jeu,
  donc elles raccourcissent avec lui. Plancher réel : **0** (contact avec
  l'épine). J'avais d'abord écrit « plancher ≈ 0,07, sinon les modules se
  touchent entre eux » — **faux**, vérifié en poussant le jeu à 0 : ce sont
  toujours l'épine qui bloque, jamais les voisins (le rayon inscrit d'un module,
  1,22, impose déjà une couronne large devant ce qu'il faudrait pour qu'ils se
  croisent). La vérification est maintenant dans le test.
- `FRET_JEU` : **0,02**. Le fret était déjà au ras (0,056) — ce qui commande,
  c'est le **coin** du treillis carré. Le vide qu'on croit voir entre l'épine et
  les conteneurs vient de ses **faces**, en retrait de ~30 % par rapport à ses
  coins (côté à `0,5·k`, coin à `√2·0,5·k`) : on ne peut pas le combler en
  rapprochant le fret, seulement en changeant la section de l'épine.

### C.6 Fret réduit de 20 % (2026-07-29)

`FRET_ECHELLE` : 0,70 → **0,56**. Elle porte désormais sur **les trois** cotes
du fret — longueur de rangée, entraxe et taille de conteneur — pour qu'elles ne
dérivent pas les unes par rapport aux autres ; le rayon de conteneur se déduit
d'une base à l'échelle 1 (`FRET_NACELLE_BASE`) au lieu d'être un absolu à
retoucher en parallèle.

Effets mesurés :

| | avant (0,70) | après (0,56) |
|---|---|---|
| largeur d'un conteneur | 4,13 | **3,30** (−20 %) |
| rayon de couronne | 2,40 | 2,15 |
| bloc de fret (Y) | 51,0 → 68,6 | 51,0 → **65,1** |
| jour entre conteneurs voisins | 0,46 | **0,76** |

⚠️ **Conséquence à connaître : la triforce se desserre.** La couronne ne peut
pas se refermer autant que les conteneurs rétrécissent — son rayon est borné en
bas par le passage de l'épine. Les conteneurs, qui se touchaient par la pointe
au départ, sont maintenant séparés d'un jour de 0,76 pour 3,30 de large (~23 %).
Le motif « triforce » se lit donc de moins en moins à mesure que le fret rapetisse
ou que l'ossature grossit. Pour le retrouver serré il faudrait soit remonter le
fret, soit affiner l'épine — les deux sont un seul chiffre (`FRET_ECHELLE`,
`ISV_ECHELLE`).

Le bloc de fret raccourcissant, il reste ~5,7 unités d'épine nue entre le fret
(fin Y ≈ 65) et l'habitat (début Y ≈ 71) — contre ~2,2 avant. À resserrer via
`HAB_CENTRE_Y` si le vide gêne à l'écran.

🐛 *Erreur commise en chemin* : l'épine élargie, les ferrures d'habitat sont
restées calculées pour l'ancienne — elles finissaient **plantées dans** la
structure (0,93 contre 1,12). D'où le test
`la_charge_utile_suit_le_gabarit_de_lepine`, qui vérifie les trois relations
(creux du fret ≥ épine, longeron de ferrure **exactement sur** l'épine, fût
d'habitat à distance) ; il a été confirmé rouge sur la version fautive.

   Coque en teinte **os, non métallique** : l'habitat du vrai ISV évite le
   métal, qui transformerait les rayons cosmiques en rayonnement secondaire
   dans les couchettes. Les armatures sont en métal **moyen** et non sombre —
   sur du composite clair, un gris franc lit comme une salissure.

   Disposition : 3 modules en couronne, **coin vers l'extérieur** donc côté
   plat (et ferrure) vers l'axe. `poser_grappe_habitat()` est partagée par la
   vue Briques et par le futur assemblage, donc ce qui est validé à l'écran est
   exactement ce qui partira sur le vaisseau.

   **Place déjà dégagée** : le fret s'arrête à Y ≈ 57 et l'épine court jusqu'à
   76,4, soit ~19 unités libres pour l'habitat, les modules d'équipage et le
   bouclier antidébris.
3. **Traverse + 2 modules d'équipage rotatifs** — réutilise `Treillis` pour la
   traverse ; c'est la silhouette la plus reconnaissable après les radiateurs.
4. **Navettes TAV** — 2 petits assemblages amarrés ; candidat idéal au
   composite `SousEnsemble` (Partie E.3) : une navette = une brique figée,
   posée deux fois.
5. **IDPS** — plaques de tête.
6. **Relecture des proportions d'ensemble** une fois tout posé.

### Sources
- [Interstellar Vehicle — Avatar Wiki](https://james-camerons-avatar.fandom.com/wiki/Interstellar_Vehicle)
- [ISV Venture Star — Grokipedia](https://grokipedia.com/page/ISV_Venture_Star)
- [Interstellar voyages with the Venture Star — State of Flux](https://kimbody1535.wordpress.com/2013/04/10/interstellar-voyages-with-the-venture-star-a-look-at-the-best-part-of-avatar/)
- [ISV Venture Star — NamuWiki](https://en.namu.wiki/w/ISV%20%EB%B2%A4%EC%B2%98%20%EC%8A%A4%ED%83%80)
