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
