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
3. ✅ **`composant.rs` découpé (Partie E.2)** — fait le 2026-07-29.
   `src/vaisseau/composant.rs` (3 316 lignes) est devenu
   `src/vaisseau/composant/`, **15 fichiers** dont aucun ne dépasse 475
   lignes. Les cinq fonctions de dispatch, qui totalisaient **1 087 lignes**,
   en font **212** : un bras d'une ligne par variante, tout le corps vivant
   dans le module de sa famille.

   | fonction | avant | après |
   |---|---|---|
   | `dessiner` | 661 | **36** |
   | `ports` | 257 | **33** |
   | `rayon_local` | 72 | **54** |
   | `cout` | 49 | **48** |
   | `englobant_local` | 48 | **41** |

   Familles : `commun` (palette + cotes partagées), `module_axial`, `noeud`,
   `panneau_solaire`, `treillis` (poutre + charpente + hexagone), `radiateur`
   (station + méga), `antenne`, `adaptateur` (+ coiffe), `caisson` (+ charge
   utile), `propulsion`, `antimatiere`, `reservoir`, `cargo`, `habitat`.

   **Aucun changement de comportement** : déplacement de code seul, les 147
   tests passent à chaque étape (une famille = une étape, validée avant la
   suivante). L'audit de couverture fait en amont (9 variantes sans test,
   121 → 130) est ce qui a rendu l'opération sûre.

   *Leçons de mécanique* : découper par plages de lignes est traître — trois
   erreurs (bras coupé une ligne trop court, doc de l'enum emportée avec les
   constantes, délégations interverties entre `ports` et `dessiner` parce que
   plusieurs fonctions portent des bras au **motif identique**). Toutes
   rattrapées par le compilateur ou les tests, aucune silencieuse. L'outil de
   coupe a fini brace-aware **et** contraint à chercher le bras *dans la
   fonction visée*.
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

Une fois les points 2/5/6 traités,
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
§2. `cargo test` passe (153 tests au 2026-07-30).

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

### C.16 Boucliers de tête refaits d'après schéma : petite plaque + trois grandes (2026-07-30)

La pile Whipple de §C.8 est **supprimée** — `Composant::BouclierDebris`, son
module, sa vitrine et ses trois tests avec. Elle était une déduction correcte de
la physique mais pas la forme voulue. Le schéma remis à la main en donne une
autre, et c'est celle-là qui est faite.

**Ce que dit le schéma.** La tête porte **quatre** boucliers : un **petit**
d'abord, puis **trois grands identiques** derrière, tous perpendiculaires à
l'axe et manifestement **enfilés sur un même mât** (le trait de l'épine
traverse les quatre sur la vue de profil). Le petit est un hexagone régulier,
dessiné **sur ses deux faces** parce qu'elles diffèrent : une face nervurée
(*face 1*) et une face **striée** en rayons serrés (*face 2*). Le grand est le
même hexagone **étiré**, annoté « *mirror surface on both sides with blue
tint* ».

**Deux composants, pas un seul réglable.** `BouclierPetit { profil, rayon }` et
`BouclierGrand { profil, rayon, elancement }`. Ils partagent tout leur code de
géométrie, mais ce ne sont pas la même pièce : le petit est une **pièce de
structure** (deux faces différentes, un dos et un endroit), le grand est un
**miroir** (deux faces de travail, aucun dos). Un booléen `miroir` aurait
économisé une branche du `match` et perdu cette distinction.

**Étirement.** La cote qui fait la forme est l'orientation de l'hexagone, pas le
facteur. `contour()` place le sommet 0 sur **+Y** (hexagone *pointe en haut*) et
étire selon Y : on obtient une pointe en haut, une en bas, et **deux longs bords
parallèles** de part et d'autre — la silhouette du schéma. Étirer un hexagone
pointe *sur le côté* aurait donné un dessus plat et six arêtes toutes
différentes. `ELANCEMENT` = **1,30** (voir la correction en fin de section).

**Le moyeu est percé, et les ports sont deux.** Contrairement à toute autre
pièce du vaisseau, une plaque ne se **boute** pas : elle s'**enfile**. D'où un
`tube` et non un cylindre au centre, et un port axial à chaque bout du moyeu.
C'est aussi pourquoi son englobant est **centré sur l'origine** — une plaque est
symétrique de part et d'autre de son plan, là où tout le reste est monté par un
bout.

**Nervures coniques.** Épaisses au moyeu, effilées à la jante (`EFFILEMENT`
0,22) : c'est la répartition d'un longeron en flexion, et c'est ce qui donne, vu
par la tranche, le **profil en nœud papillon** que le schéma dessine sur la vue
d'assemblage. Sur les grandes plaques elles sont **centrées sur le plan** (les
deux faces sont des faces de travail) ; sur la petite elles sont **décalées
derrière** la peau arrière, sinon elles traverseraient la face striée et la
brouilleraient. Douze nervures et non six : une fois la plaque étirée, les deux
longs bords sont bien plus longs que les quatre autres et rien ne les tiendrait
en leur milieu.

**« Miroir » dans un rendu à plat.** Le pipeline ne fait pas d'éclairage — une
couleur par sommet, c'est tout. Une réflexion ne peut donc pas se dire ; seules
une **valeur haute** et une **teinte franchement froide** le peuvent, à l'opposé
de l'alu neutre de toute la structure. Reste la face arrière, un cran plus
sourde que l'avant — sans quoi on ne sait plus laquelle des deux on regarde.

*(Premier jet : `FACETTES`, six coefficients de valeur appliqués aux six
secteurs, au motif qu'une plaque d'une seule couleur serait une tache sans
forme. Faux ici — voir §C.17. Le facettage n'a survécu que sur le dos du petit
bouclier, qui n'a pas de motif pour le structurer.)*

🐛 **Le défaut qui ne se voit pas : une nappe cousue à l'envers.** macroquad ne
double-face pas les triangles, donc une peau au mauvais enroulement **disparaît
purement et simplement**. Rien ne le signale : ni le compilateur, ni les tests
de gabarit (l'englobant, l'épaisseur, l'élancement restent tous justes — les
sommets sont là, ce sont les triangles qui regardent ailleurs). D'où
`les_deux_peaux_dune_plaque_regardent_chacune_dehors`, qui isole les triangles
de nappe **par leur cote** — les deux peaux sont les seules surfaces
rigoureusement planes de la pièce, cônes et tubes n'ont jamais leurs trois
sommets à la même altitude — et vérifie le signe de leur normale. Il compte
aussi les triangles trouvés : un test qui n'en trouverait aucun passerait vert
sans avoir rien mesuré, ce qui est exactement le piège attrapé trois fois en
§C.13–C.15.

**Vitrines.** Briques 25 → **26 items**. La n° 21 montre la petite plaque **côte
à côte avec elle-même retournée** — la seule mise en scène qui laisse juger les
deux faces d'un même point de vue, comme le schéma le fait — plus une vue par la
tranche. La n° 22 montre la grande de face, de profil, et les **trois** enfilées
sur un mât pour juger l'espacement.

Détail heureux et vérifié à la mesure : le mât de démonstration (`Treillis` P0
triangulaire) fait **0,290** de rayon transversal pour un alésage de moyeu de
**0,302**. Il passe, de justesse et proprement.

**Reste en suspens** : à quel bout du vaisseau la tête se monte (la
contradiction §C.2 / §C.4–C.6 est intacte), et le mât lui-même, qui n'existe
pas encore comme pièce.

### C.17 Grande plaque : le motif du schéma, et pourquoi elle lisait comme une gemme (2026-07-30)

Retour d'écran : la brique 21 (petit bouclier) est validée telle quelle ; la 22
« *est emerald-shaped, donc ne correspond pas au schéma* », avec le schéma de la
grande plaque renvoyé seul et l'instruction de **reproduire le motif de chaque
face**. Deux causes, et la principale n'était pas la silhouette.

**1. Le facettage faisait le dessin d'une pierre taillée.** J'avais appliqué à la
grande plaque les six coefficients `FACETTES` — six triangles de valeurs
différentes rayonnant d'un moyeu. C'est, trait pour trait, la façon dont on
dessine une gemme facettée. Le raisonnement d'origine (« à plat, une plaque d'un
seul ton est une tache sans forme ») était juste pour le **dos du petit
bouclier**, qui n'a rien d'autre pour se structurer, et faux pour la grande, qui
porte un motif de nervures. Un miroir est **uni** ; ce sont les nervures posées
dessus qui lui donnent sa forme.

`les_faces_dun_grand_bouclier_sont_des_miroirs_unis` compte les tons distincts
sur les triangles de nappe (isolés par leur cote, comme au test d'enroulement) et
exige **exactement deux** : un par face. Vérifié rouge en remettant `FACETTES` —
*« 7 tons sur les nappes : la plaque est facettée comme une gemme »*. Aucun test
de forme n'aurait dit ça : la silhouette, l'épaisseur et l'élancement étaient
tous justes.

**2. Le motif n'était pas celui du schéma.** J'avais repris l'armature du petit
bouclier — douze nervures et une **ceinture** faisant le tour. Le schéma en
montre une autre, et `motif_grand()` la reproduit :

- **huit rayons** partant du moyeu : les six sommets, plus le **milieu des deux
  longs bords**. Ce sont les seules arêtes assez longues pour demander un appui à
  mi-course, et le schéma ne dessine effectivement rien vers les quatre obliques ;
- **deux cordes horizontales**, `V1–V5` et `V2–V4`, qui détachent les deux
  pointes en triangles francs. Elles joignent des sommets de **même ordonnée**,
  donc elles tombent d'elles-mêmes à `±0,5·r·élancement` : aucune cote à régler,
  et elles suivent l'élancement sans qu'on y touche.

Douze cellules en tout : deux par pointe, huit autour du moyeu. Nervures amincies
de 0,055 à **0,030** du rayon — sur le schéma ce sont des *traits* posés sur le
miroir, pas les membrures apparentes d'une pièce de structure.

**3. Élancement 1,75 → 1,30.** Mesuré sur la photo : ≈ 298 px de haut pour 220 de
large, soit un rapport de **1,35**. Pour un hexagone pointe en haut ce rapport
vaut `2e/√3`, d'où `e ≈ 1,17` ; arrondi à 1,30 parce que le cliché est pris de
biais et raccourcit le grand axe. L'ancienne valeur donnait un rapport de 2,02 —
franchement plus long que le dessin, ce qui accentuait encore la lecture
« pierre taillée en long ».

Le test d'élancement a été refait à cette occasion : il comparait `grand` à
`petit × 1,5`, un seuil arbitraire qui aurait simplement viré au rouge sur un
changement légitime. Il vérifie maintenant deux choses distinctes — que
l'étirement **arrive jusqu'à la géométrie** (`grand/petit ≈ ELANCEMENT`), et que
la constante reste dans la fourchette relevée sur le schéma (1,15 à 1,50).

⚠️ *Leçon transférable* : « il faut de la variation de valeur, sinon c'est une
tache » est vrai d'une surface **nue** et faux d'une surface **structurée**.
Appliquer la recette sans regarder ce que la pièce porte déjà, c'est ajouter un
second système de lecture qui contredit le premier — ici, des facettes de gemme
par-dessus une armature de miroir.

### C.18 Méplats aux deux pointes de la grande plaque (2026-07-30)

Confirmation demandée en fin de §C.17 : les deux schémas dessinent bien un
**méplat** aux deux pointes de la grande plaque, en haut comme en bas, « *pas
large* ». Ajouté.

**Rognage, pas appendice.** `contour()` prend maintenant une fraction `tab` et
coupe chaque pointe à cette fraction du chemin vers ses deux voisines. Le bord
droit obtenu fait alors exactement `tab × largeur` — aucune cote à régler, et le
méplat suit l'élancement tout seul. Un onglet **posé en plus** de la pointe
aurait demandé sa propre longueur et sa propre largeur, et se serait désaccordé
au premier changement de proportion.

`TAB` posé à 0,16 puis **divisé par deux à 0,08** au vu de l'écran — un douzième
de la largeur. C'est la seule arête de la plaque qui coure parallèlement à **Z**
dans la vue Briques (les longs bords sont selon Y, quelle que soit la pose des
plaques dans la vitrine), donc à la fois la plus facile à désigner et la plus
voyante quand elle est de trop. Le plancher du test (5 % de la largeur) laisse
encore de la marge, mais plus beaucoup : en dessous le méplat cesse d'être
visible et la pointe redevient franche.

Le contour d'une grande plaque compte donc **huit** sommets et non six, et
`motif_grand()` y renvoie par index. `contour_grand()` serre `TAB` dans
`[0,02 ; 0,6]` : un méplat nul rendrait l'hexagone à six points et les index
sortiraient du tableau. Le rognage est une **propriété de la pièce**, pas un
réglage annulable — vérifié en essayant `TAB = 0`, qui partait en index hors
bornes avant le serrage.

**Deux entrées de mesure au lieu d'une.** `rayon_local`/`englobant` prenaient
`(rayon, elancement)` et la petite plaque passait `elancement = 1`. Ça ne suffit
plus : le méplat change le rayon hors-tout, et une petite plaque mesurée *avec*
le méplat des grandes verrait son englobant cesser de la contenir. La mesure
prend maintenant le **contour déjà construit**, chaque plaque passant le sien —
un paramètre `tab` de plus aurait été un paramètre de plus à oublier.

🐛 **Le méplat a cassé un test, et la façon dont il l'a cassé est instructive.**
`la_grande_plaque_est_elancee_la_petite_reguliere` comparait le rapport
hauteur/largeur de la grande à celui de la petite, et attendait l'élancement.
Faux dès qu'un rognage existe : il raccourcit la hauteur de ≈ `TAB/2`, donc le
rapport mesuré tombe à `0,91 × élancement`. Le test aurait viré au rouge sur un
changement parfaitement légitime.

Corrigé en mesurant **d'une grande plaque à l'autre** — `grand(ELANCEMENT) /
grand(1,0)` — où le méplat, identique des deux côtés, se simplifie. C'est le même
principe que le déport de propulsion en §C.15 : comparer deux choses qui ne
partagent pas les mêmes conditions donne un chiffre qui ne veut rien dire.
Vérifié : le test reste vert à `TAB = 0,02` comme à `0,16`, ce qui est
exactement la propriété recherchée.

`les_pointes_dune_grande_plaque_sont_rognees_dun_meplat` borne le méplat **des
deux côtés** (5 % à 30 % de la largeur), parce qu'il y a deux façons opposées de
le rater et qu'aucune ne fait de bruit : pas de méplat du tout, ou un si large
que la plaque devient un tonneau. Il ne mesure que les sommets de **nappe** —
jante et nervures sont des cylindres dont les couronnes déborderaient du contour
et élargiraient artificiellement le méplat. Vérifié rouge dans les deux sens
(*« la pointe est restée franche »* à 0,02 ; *« un bout coupé »* à 0,45).

### C.19 Épaules remontées : les longs bords divisés par deux (2026-07-30)

Deux retouches de silhouette demandées à l'écran, désignées par l'axe auquel
chaque arête est parallèle dans la vue Briques — une nomenclature que la boussole
d'axes de §C.11 rend enfin praticable, et qui décrit la plaque sans ambiguïté :
les **méplats sont selon Z**, les **longs bords selon Y**, les quatre obliques
selon ni l'un ni l'autre.

**1. Méplat (∥ Z) élargi.** 0,16 jugé trop épais, 0,08 trop maigre, **0,12**
retenu. Trois passes sur une cote purement visuelle, ce qui est normal — mais
c'est bien pour ça que la valeur porte son historique en commentaire.

**2. Longs bords (∥ Y) divisés par deux.** Ils étaient une conséquence de
l'hexagone régulier : les épaules à mi-hauteur donnent des bords longs de
`0,5 × hauteur`. Les raccourcir **sans écraser la plaque** demandait de sortir
cette hauteur d'épaule de la géométrie de l'hexagone, d'où `EPAULE = 0,25`.

⚠️ Le piège était de le faire en réduisant l'élancement. Un long bord vaut
`rayon × élancement` : diviser l'élancement par deux le divise bien par deux —
et divise la hauteur totale par deux avec lui, ce qui donnerait une plaque plus
**large que haute** (0,75 de rapport), à l'opposé du schéma. La demande portait
sur *une arête*, pas sur la plaque. En remontant les épaules, la hauteur et la
largeur ne bougent pas d'un pouce : ce sont les quatre obliques qui s'allongent
d'autant.

`contour()` prend donc maintenant `epaule` en plus, et `EPAULE_REGULIER` (0,5)
nomme la valeur qui redonne l'hexagone régulier — celle que garde le petit
bouclier, dont la brique est validée et qu'on ne touche pas. Généralisation
stricte : à 0,5 la géométrie produite est identique au sommet près.

`les_longs_bords_dune_grande_plaque_sont_reduits_de_moitie` mesure la longueur du
bord **rapportée à la hauteur**, parce que c'est ce rapport qui décrit la
silhouette et non une longueur absolue qui suivrait le rayon. Borné des deux
côtés : au-dessus de 0,40 le raccourcissement n'a pas eu lieu, en dessous de 0,12
les épaules se rejoignent et la plaque n'est plus qu'un losange — elle perd les
deux bords parallèles qui font toute sa forme. Vérifié rouge dans les deux sens
(0,532 à `EPAULE = 0,5`, 0,045 à 0,04).

### C.20 Plaque rétrécie de 20 %, barres transversales retirées (2026-07-30)

**1. Les deux barres transversales sont supprimées.** Elles venaient du schéma et
elles y ont leur place — mais le schéma dessine une plaque aux épaules à
mi-hauteur. Depuis §C.19 les épaules sont remontées à 0,25, si bien que les deux
barres, qui les joignent, passaient désormais à quelques dixièmes du moyeu : elles
encombraient le centre au lieu de le structurer. Le motif se réduit donc à ses
**huit rayons**, tous ancrés au moyeu, et plus rien ne traverse la plaque.

C'est le genre de conséquence qu'on ne voit pas venir en changeant une cote : la
barre n'a pas changé, c'est ce à quoi elle s'accroche qui s'est déplacé. Noté
dans le commentaire de `motif_grand()`, avec la raison — sinon quelqu'un les
remettra en se réclamant du schéma, à juste titre.

**2. Largeur réduite de 20 %** — `ETROITESSE = 0,80`, appliqué à la **seule**
largeur.

⚠️ La façon évidente de rétrécir une plaque est de raboter son rayon. Elle est
fausse ici, et pas d'un cheveu : le rayon commande aussi le **moyeu**, dont
l'alésage (0,302) ne laisse que **0,012** de jeu au mât (0,290, mesuré en §C.16).
Le réduire de 20 % l'aurait fait passer à 0,242, soit le mât **au travers du
moyeu** — un défaut qui ne se serait vu qu'à l'assemblage, longtemps après.
Largeur et hauteur se règlent donc maintenant sur `contour()`, qui prend ses
**deux demi-cotes directement** au lieu d'un rayon assorti de facteurs ; le rayon
garde son rôle là où il représente vraiment un gabarit (moyeu, alésage, section
des nervures).

`la_grande_plaque_est_retrecie_en_largeur_seule` compare la grande à la petite
**au même rayon** : à rayon égal, la petite donne la largeur pleine et la grande
la largeur rabotée, donc leur rapport isole exactement le facteur. Et il vérifie
en plus que **l'alésage n'a pas bougé**, parce que c'est là qu'était le vrai
risque et qu'une simple mesure de largeur n'aurait rien dit de la façon dont elle
a été obtenue. Vérifié rouge des deux côtés : *« largeur rabotée d'un facteur
1,000 au lieu de 0,800 »* en retirant le facteur, *« alésage 0,173 contre 0,216 :
le moyeu a suivi le rétrécissement, le mât commun ne passera plus »* en le
faisant passer par le rayon.

### C.21 Tête de bouclier assemblée sur l'ISV (2026-07-30)

Les quatre plaques sont posées sur le vaisseau. Vaisseau final : **X −45,2 →
123,0** (168,2 de long, contre 137 avant la tête), rayon max 12,6, **43 pièces,
coût 683**.

**À quel bout : tranché.** La question traînait depuis §C.8, où nos propres notes
se contredisaient — §C.2 plaçait les plaques « en avant du vaisseau », donc côté
**moteurs** puisque l'ISV est un tracteur ; §C.4 et §C.6 les mettaient sur le
**haut d'épine libre**. Le schéma d'assemblage tranche : les quatre plaques sont
dessinées **à l'opposé des radiateurs**, après toute la charge utile. C'est aussi
le seul bout dégagé — côté moteurs il aurait fallu composer avec les tuyères et
des ailes de rayon 12,6. `la_tete_de_bouclier_coiffe_le_bout_oppose_aux_moteurs`
fige la réponse, y compris l'ordre petite → grandes.

**Gabarit.** Grande plaque `rayon = 10` → **26 de haut** pour 13,9 de large, à
comparer aux 25,2 de diamètre du vaisseau aux radiateurs : la tête fait la
largeur du vaisseau, ce que montre le schéma. Petite plaque `rayon = 5,5` → 11 de
haut, soit 42 % de la grande. Écart de 8 entre plaques, tête longue de 28.

**Orientation.** Les plaques sont perpendiculaires à l'axe, et leur grand axe est
mis dans le **plan des ailes radiateur** par un quart de tour supplémentaire.
Sans lui, les deux extrémités du vaisseau ne se liraient pas dans la même vue de
profil : la tête paraîtrait plate de trois quarts au moment même où les ailes
sont de face.

**C'est la petite plaque qui dimensionne le mât**, pas les grandes. Son alésage
vaut `rayon × MOYEU × ALESAGE`, le plus étroit de la pile ; le mât est donc un
`Treillis` **P0** (section transversale mesurée 0,290). *(Cotes du moyeu revues
depuis — voir §C.25.)* Le cran
au-dessus fait 0,580 et ne passerait pas — vérifié rouge, *« mât de 0,580 pour un
alésage de 0,396 : il n'enfile pas la petite plaque, il la traverse »*. Et c'est
bien un défaut **muet** : un mât trop gros ressort de l'autre côté du moyeu sans
que rien ne l'arrête, et la pile a l'air enfilée alors qu'elle est empalée. D'où
`le_mat_de_tete_passe_le_plus_petit_alesage`, qui mesure les deux sur la
géométrie cuite et borne aussi par le bas (un mât qui flotte dans son alésage ne
porte visiblement rien).

**Raccord conique au sommet d'épine.** La flèche finit à **0,9** de rayon et le
mât fait **0,29** : bout à bout, la section chutait d'un facteur trois d'un coup
et la tête avait l'air rapportée plutôt que portée. Un `Adaptateur` P1 → P0 de
2,4 de long ramène ça à deux marches franches — mesuré 0,9 → **1,15** (léger
débord sur la flèche, jamais une face coplanaire) → **0,50**, après quoi le mât
sort de son col. C'est aussi ce qui borne la position de la première plaque par
le bas : le col fait 0,5 de rayon, plus que l'alésage de 0,396, donc une plaque
posée avant la fin du raccord s'empalerait dessus.

🐛 *Piège repris à l'identique* : en mesurant le raccord j'ai d'abord trouvé 0,27
là où j'attendais ~0,8, et cru la pièce absente. Elle était là — c'est la
**mesure** qui était fausse : un cône cuit n'a de sommets qu'à ses **deux bouts**,
donc toute tranche prise au milieu est vide. Exactement le piège noté en §C.13
sur les cylindres. Repris la mesure sur les anneaux de sommets, et le raccord
était bien conforme.

**Vue Megastructures réduite à l'ISV hexagonal** (4 → 3 items). L'épine carrée a
servi à valider l'hexagonale par comparaison (vue Briques n° 23) et n'a plus à
occuper une vitrine. ⚠️ **Aucun composant n'est supprimé** : `Epine::Carree`,
`Composant::Charpente` et tout ce qu'ils entraînent restent en place,
`preset_isv()` la construit encore et des tests s'en servent. Seule la vitrine
disparaît.

**Reste sur l'ISV** : les 2 navettes TAV (candidat idéal au composite
`SousEnsemble`), et une relecture des proportions d'ensemble maintenant que le
vaisseau est complet.

### C.22 Structure de l'ISV validée (2026-07-30)

Validé à l'écran. La **structure** du vaisseau est close : plus rien de portant
n'est à ajouter, et les cotes ci-dessous font référence pour la suite.

| | |
|---|---|
| Longueur hors-tout | **168,2** (X −45,2 → 123,0) |
| Rayon max | **12,6** (ailes radiateur) |
| Pièces / coût | **43 / 683** |
| Sommet d'épine | 91,7 |
| Tête de bouclier | 95 → 123 |

Enchaînement des sections, des tuyères vers la tête : propulsion et ailes
radiateur → pied d'épine en pavillon hexagonal → épine nue → 3 rangées de fret en
triforce → habitat principal (3 modules) → section d'équipage rotative → raccord
conique → tête de bouclier (1 petite plaque + 3 grandes sur mât). C'est l'ordre
du vrai vaisseau : les moteurs à un bout, ce qui est habité aussi loin d'eux que
possible, et le blindage encore au-delà.

**Ce qui a tenu, sur toute la durée du chantier.** Trois décisions méritent
d'être notées parce qu'elles ont chacune évité une reprise complète :

- **brique d'abord, assemblage ensuite.** Aucune pièce n'a été posée sur le
  vaisseau avant d'avoir été jugée seule dans la vue Briques. Les allers-retours
  sur les boucliers (§C.16 → §C.20 : facettage, méplats, épaules, largeur) se
  sont tous faits sur la brique, sans jamais toucher au vaisseau ;
- **le gabarit d'épine comme source unique.** `Epine::hors_tout()` et les
  `fn(epine) -> rayon` qui en dépendent : c'est ce qui a permis de passer l'épine
  de carrée à hexagonale sans recaler une seule cote de charge utile à la main ;
- **mesurer plutôt que déduire.** Presque toutes les erreurs de ce chantier
  étaient des **tests qui mesuraient la mauvaise chose** — pièces comparées à des
  hauteurs différentes (§C.15), tranche prise là où un maillage cuit n'a pas de
  sommets (§C.13, §C.21), seuil recalculé au lieu d'être lu (§C.14). Aucune
  n'était une erreur de géométrie. Le réflexe qui les a toutes attrapées : après
  avoir écrit une assertion, **casser exprès ce qu'elle surveille** et vérifier
  qu'elle vire au rouge.

**Reste** : 2 navettes TAV (candidat idéal au composite `SousEnsemble`), puis une
relecture des proportions d'ensemble. Détails de surface facultatifs (tunnel
pressurisé, bouclier thermique d'épine) — à ne faire que s'ils se voient à la
silhouette.

### C.23 Relecture des proportions d'ensemble (2026-07-31)

Mesuré section par section sur le vaisseau assemblé, plutôt que jugé à l'œil.

| Bloc | de | à | long | part |
|---|---|---|---|---|
| Propulsion (ailes, nacelles, cuves, moteurs) | −45,2 | −1,3 | 43,9 | 26 % |
| **Épine nue** | −1,3 | 51,3 | **52,6** | **31 %** |
| Fret (3 rangées) | 51,3 | 64,8 | 13,5 | 8 % |
| Habitat | 71,6 | 79,6 | 8,0 | 5 % |
| Équipage | 84,5 | 85,9 | 1,4 | 1 % |
| Raccord + mât | 85,9 | 94,5 | 8,6 | 5 % |
| Tête de bouclier | 94,5 | 119,9 | 25,4 | 15 % |

**Verdict : l'équilibre est bon.** Le tronçon d'épine **nu** est le plus long
segment du vaisseau (31 %, et plus long que toute la charge utile), la
propulsion tient un quart, la tête un sixième. C'est la répartition de la
référence, et rien n'appelait de redécoupage.

**Un seul écart réel : le fret.** Le vrai ISV porte **4 rangées de 4 modules** ;
nous en avions 3, pour 13,5 de long contre 8,0 d'habitat — un rapport de 1,7 là
où le fret devrait franchement dominer, l'ISV étant avant tout un cargo.
`FRET_RANGEES` passe à **4** : fret à 18,3 (rapport 2,3), et l'espace mort entre
le fret et l'habitat tombe de 6,8 à 2,0 par la même occasion. **Aucune cote n'est
retouchée** — une rangée est ajoutée, et `FRET_DEBUT_Y` était déjà écrit pour que
le bloc grandisse vers le haut.

**Trois rapports verrouillés**, chacun encodant une décision et pas une valeur :

1. **l'épine nue** dépasse le quart de la longueur *et* la charge utile entière —
   c'est une poutre en **tension**, et ce qui le dit est sa longueur à vide ;
2. **le fret domine l'habitat** (× 1,8 au moins) — sinon le vaisseau cesse de
   lire comme un cargo ;
3. **l'élancement** reste au-dessus de 6 diamètres — en deçà la silhouette
   s'épaissit et l'ISV ressemble à une station.

Bornes larges à dessein : elles n'imposent pas une silhouette, elles interdisent
de la perdre par accumulation de retouches dont aucune n'est fautive isolément.

🐛 **Le troisième red-check a trouvé un vrai défaut.** En vérifiant que
l'assertion d'élancement virait bien au rouge, j'ai divisé `EPINE_LONGUEUR` par
deux — et le test est resté vert. Cause : `BOUCLIER_DEBUT_Y` valait **95,0 en
clair**. Raccourcir l'épine ne raccourcissait donc pas le vaisseau : la tête
restait plantée à sa cote absolue, séparée du sommet d'épine par un vide. Le
défaut n'était visible d'aucune vue tant qu'on ne touchait pas à l'épine, et
c'est précisément la classe de bug que le gabarit d'épine
(`Epine::hors_tout`, §C.10) évite pour toute la charge utile — la tête y avait
échappé. Elle se déduit maintenant du sommet d'épine, et l'élancement vire au
rouge comme attendu (*« élancement 5,0 »* à demi-épine).

C'est le troisième cas du chantier où **le red-check trouve un défaut du code et
non du test**. Le réflexe vaut d'être gardé : casser exprès ce qu'une assertion
surveille dit autant sur le code que sur l'assertion.

**Ce qui reste** (dans l'ordre décidé) : les 2 navettes TAV, puis les détails de
surface retenus — **bouclier thermique d'épine**, **montée en température des
radiateurs**, **allumage du panache antimatière**. Les deux derniers ne sont pas
de la géométrie mais de l'**état** : ils supposent une notion de régime moteur
que le modèle n'a pas encore.

### C.24 Bouclier thermique d'épine : bardage d'écailles (2026-07-31)

Premier des trois détails de surface retenus, et le plus simple des trois parce
que c'est le seul qui soit purement géométrique.

**Des écailles, et pas une tôle.** La demande était « une forme d'écailles sur
l'épine, pas très épaisse », et elle tombe juste pour une raison qui n'est pas
esthétique : une paroi continue de plusieurs dizaines d'unités soumise au
gradient d'une tuyère se déforme, et une paroi encastrée qui se déforme casse ou
arrache ses fixations. Le bardage **imbriqué** résout exactement ça — chaque
plaque est petite, tenue par un seul bord, et **libre de se dilater** sous celle
qui la recouvre. C'est la logique des tuiles de navette.

⚠️ **Le sens du recouvrement n'est pas libre.** La chaleur vient de la base :
chaque écaille recouvre donc la **suivante en s'éloignant des moteurs**, si bien
que le flux glisse d'une plaque à l'autre sans jamais rencontrer une tranche de
face. Monté à l'envers, chaque joint offrirait une arête au rayonnement — le
raisonnement d'un toit posé dans le sens de la pluie.

**Cotes.** `SAILLIE = 0,13` du rayon (« pas très épais » : le bardage habille
l'épine, il ne la double pas), `RECOUVREMENT = 0,35` du pas, lèvre sombre sur
22 % de l'écaille. Sur le vaisseau : rayon 1,25 contre 1,15 d'épine hors-tout,
de X = 12 à X = 48, 30 rangs.

> ⚠️ **Emprise et manchon droit revus le lendemain** — voir §C.25. Le bardage
> était posé de X = 12 à 48, sur le tronçon nu, alors qu'il doit être au droit
> des moteurs ; et il est passé de droit à **évasé** pour épouser l'épine là où
> elle s'ouvre. Les cotes d'écaille ci-dessus (saillie, recouvrement, lèvre)
> n'ont pas bougé.

🐛 **Mon premier test mesurait la mauvaise propriété.** Il comptait les
**retombées de rayon** le long de l'axe, en tenant que « ça monte puis ça
redescend à chaque rang » distingue un bardage d'un manchon conique. C'est vrai,
mais insuffisant : mis à `RECOUVREMENT = 0` — écailles bout à bout, aucun
recouvrement, donc plus aucune fonction — le test **reste vert**, parce que le
profil dentelé, lui, ne change pas. Silhouette identique, protection nulle.

Le test vérifie maintenant les deux, et la seconde est celle qui compte : le bord
**libre** d'un rang doit être axialement **au-delà** du bord **plaqué** du rang
suivant, d'au moins 0,2 pas. Bords plaqués et bords libres sont séparés par leur
rayon (nu / maximal), ce qui évite d'avoir à reconnaître les rangs un par un.
Vérifié rouge à recouvrement nul.

C'est la quatrième fois du chantier qu'une assertion mesure un **corollaire** de
ce qu'elle prétend garder plutôt que la chose elle-même. Le corollaire est
toujours plus facile à mesurer — c'est bien pour ça qu'on l'écrit sans y penser.

**Vitrine.** Briques 26 → **27 items** ; n° 23, avec un tronçon court grossi pour
juger une écaille seule, et le manchon **enfilé sur un bout d'épine** — le seul
moyen de vérifier ce qui compte, à savoir qu'il plaque sur la poutre au lieu de
flotter autour.

**Vaisseau** : 44 pièces, coût 689. Les proportions ne bougent pas — c'est un
revêtement, il n'ajoute ni masse visible ni encombrement, et
`les_proportions_densemble_de_lisv_tiennent` reste vert sans retouche.

### C.25 Bardage thermique déplacé au droit des moteurs (2026-07-31)

Grief à l'écran : le bardage était **au mauvais endroit**. Il courait de X = 12 à
48, c'est-à-dire sur le long tronçon nu, loin des tuyères qui s'arrêtent à −1,3.
Un bouclier thermique se met **là où est la chaleur**.

**Nouvelle emprise : X = −9 → +4**, treize unités au droit des moteurs
(−9,8 → −1,3), qui les dépassent de cinq et s'arrêtent là. Le long tronçon nu
redevient nu, ce qui est doublement juste : il n'a rien à parer, et un bardage
l'aurait fait lire comme une gaine technique.

**Conséquence : le bardage devient évasé.** C'est tout l'intérêt du déplacement
et c'était le vrai travail. Au droit des moteurs, l'épine n'est pas droite — elle
s'ouvre vers son pied, de 2,81 à 1,28 de rayon sur l'emprise couverte. Un manchon
droit y serait traversé à un bout et flotterait à l'autre. La section suit donc
la **même loi en puissance** que le treillis : `bout + (pied − bout)·(1 − t)^c`.

Trois cotes mesurées sur l'épine assemblée et non calculées : `pied = 3,50`,
`bout = 1,70`, `courbure = 1,5`. Jeu au fil de l'emprise : **+0,12 à +0,25**,
positif partout et **au pire cas** (voir ci-dessous).

🐛 **Le premier essai partait de X = −12 et flottait de 0,93.** Cause mesurée :
entre −12 et −10,5 le rayon de l'épine tombe de 3,75 à 2,85 **d'un coup**, puis
reprend une décroissance douce. Aucune loi en puissance ne suit ce décrochement.
En reculant le départ à −9, le profil est régulier — et l'exposant ajusté tombe à
**2,0 ± 0,05 aux trois tranches du milieu**, c'est-à-dire que le profil de
l'épine *est* une loi en puissance sur cette portion. Ça n'allait pas de soi :
la loi du treillis est écrite depuis la base de la charpente, dix unités plus
bas, avec le pied en pavillon par-dessus.

🐛 **Et le deuxième essai pinçait encore, alors que le test était vert.** Griefs
successifs à l'écran : « ça accroche légèrement l'épine ». Le test comparait les
**circonradius** des deux pièces et trouvait un jeu positif partout. Il mesurait
la mauvaise chose, pour une raison qui ne saute pas aux yeux :

> Les deux hexagones — celui du bardage et celui du treillis — ne sont calés sur
> **aucune orientation commune**. Le bardage a son repère propre, l'épine tient
> le sien de `repere(axe)`. Dans le pire cas ils sont décalés d'un demi-pas, et
> c'est alors le **milieu de facette** du bardage (rayon **inscrit**, 0,866 × le
> circonradius) qui passe au droit d'un **longeron** de l'épine.

Comparer deux circonradius, c'est donc comparer les deux points les plus
*éloignés* de l'axe alors que le contact se joue entre le point le plus rentrant
de l'un et le plus saillant de l'autre. Le test compare maintenant
`circonradius_bardage × 0,866` au circonradius de l'épine, et les cotes sont
dimensionnées là-dessus — d'où le facteur 1,155 sur les rayons relevés. C'est le
prix de ne pas dépendre d'un calage angulaire que rien ne garantit.

Deux autres corrections de mesure au passage, toutes deux du même genre :

- **le bardage était pris par ses sommets**, lèvres comprises. Une tranche qui ne
  contient qu'une lèvre — relevée par construction — lit un rayon bien trop grand
  et fait croire à un bardage qui flotte (0,41 et 0,45 mesurés là où il ne
  flottait pas). Il est maintenant pris par sa **section de calcul**, la surface
  qui se plaque réellement sur la poutre ;
- **l'épine était prise tranche par tranche**, et certaines tranches ne coupent
  que des diagonales : elles lisent un rayon anormalement bas (1,62 là où la
  voisine donne 1,84). Le rayon est maintenant pris sur une **fenêtre d'une
  baie**.

`le_bardage_thermique_epouse_lepine` borne des deux côtés — trop serré l'épine
ressort au travers, trop lâche le bardage flotte — et rien de tout ça ne se
déduit du composant seul : c'est un ajustement entre **deux** pièces, vérifiable
seulement assemblé. Vérifié rouge dans les trois sens : *« pincement de 0,21 »*
et *« de 0,10 »* en revenant aux rayons d'avant, *« flotte de 1,14 »* en gonflant
le pied.

**Moyeu des plaques de tête resserré, et l'alésage a dû suivre.** Le rayon de
moyeu est passé de 0,16 à **0,09** du rayon de plaque. Effet de bord non voulu :
à alésage constant (0,45), le trou tombait à 0,223 pour un mât qui en fait
0,290 — la petite plaque était **empalée** au lieu d'être enfilée.
`le_mat_de_tete_passe_le_plus_petit_alesage` l'a signalé immédiatement, ce qui
est exactement ce qu'on lui demande : le défaut est invisible, un mât trop gros
ressortant de l'autre côté du moyeu sans que rien ne l'arrête.

Corrigé du bon côté — `ALESAGE` porté à **0,75**, trou à 0,371, le mât repasse
avec 0,08 de marge. Le moyeu devient une **bague** (paroi 0,124 pour un extérieur
de 0,495) plutôt qu'un disque percé, ce qui est d'ailleurs plus juste pour une
pièce qu'on enfile. ⚠️ Réagir en annulant le resserrement aurait été le mauvais
réflexe : le test signale une **incompatibilité entre deux cotes**, il ne désigne
pas laquelle des deux est en tort.

### C.26 Chauffe des radiateurs, au bouton (2026-07-31)

Deuxième détail de surface. Un bouton **RADIATEURS: FROID / CHAUD** dans la vue,
et les ailes montent au rouge puis à l'orange en trois secondes et demie.

**Ce n'est pas un réglage d'affichage, c'est de la géométrie.** Les couleurs
vivent dans les **sommets** du maillage cuit : chauffer une aile veut dire la
recuire. D'où `chaleur: f32` sur `Composant::RadiateurMega` et sur
`preset_isv_fixe`, plutôt qu'un drapeau dans la vue. C'est la même nature que le
repli, et pas celle de la rotation — qui, elle, n'est qu'une matrice.

**Où passe la chaleur : seulement le gris.** Panneau, tubes calorifiques et rails
de bord chauffent ; la colonne vertébrale et le réservoir restent noirs. Ce ne
sont pas des surfaces radiantes mais des organes internes, et les voir rougir
ferait mentir la pièce.

**Gris → rouge sombre → orange**, et pas gris → orange. Un métal qui chauffe
**rougit d'abord** (point de Draper, ~525 °C) ; sauter cette étape donne un
radiateur peint plutôt que chaud. C'est aussi ce qui rend le début de la montée
lisible : un mélange direct vers l'orange éclaircit tout de suite et le seuil
disparaît. Trois secondes et demie de transition, plus lent que le repli — une
masse chauffe lentement, et c'est ce que la durée doit dire.

⚠️ **Le dégradé n'est pas un effet, c'est la fonction de la pièce.** Un radiateur
est exactement l'objet qui se refroidit sur sa longueur : le fluide entre chaud à
la racine et ressort tiède à la pointe. Une aile qui rougirait uniformément ne
dirait pas qu'elle radie, elle dirait qu'elle est peinte. La chaleur locale perd
donc 55 % de la racine à la pointe. Conséquence de dessin : le panneau, qui était
un quadrilatère d'une seule couleur, est découpé en **sept bandes** — un
quadrilatère ne peut pas porter de dégradé.

**`charger()` scindé en `rebatir()` + `cadrer()`.** La montée refait la géométrie
à chaque frame, et `charger()` recadrait la caméra : le zoom de l'utilisateur
aurait été annulé en continu pendant toute l'animation. On ne recadre plus qu'au
**changement d'item**, seul moment où le gabarit change vraiment.

Le recuit porte ici sur la moitié **fixe** du vaisseau (les ailes sont sur
l'ossature, elles ne tournent pas), donc sur tout le maillage. Coût assumé et
**borné** : la montée dure trois secondes et s'arrête, là où le commentaire de
§C.7 visait une rotation recuite, qui tournerait indéfiniment.

🐛 **Compter une proportion ne disait rien.** Mon premier test vérifiait que
« moins de 95 % des sommets sont chauds », en tenant que le reste serait le noir.
Il virait au rouge à **98 %** — non parce que le noir avait rougi, mais parce que
les tubes calorifiques pèsent à eux seuls presque tous les sommets de l'aile et
que la colonne noire disparaît dans l'arrondi. Le test suit maintenant les
sommets **un par un** : les noirs sont repérés à froid, et comme la chauffe ne
déplace rien, le sommet `i` est le même dans les deux versions. Vérifié rouge en
passant le réservoir dans `chauffer()` — *« sommet 3570 : la colonne vertébrale
a rougi »*.

`le_radiateur_est_plus_chaud_a_sa_racine_qu_a_sa_pointe` garde le dégradé,
vérifié rouge à refroidissement nul (*« racine 220 contre pointe 220 »*).

Bouton actif sur **deux** vues : l'ISV complet et la brique n° 6 du radiateur
méga — c'est là, l'aile présentée en grand, qu'on juge le dégradé de près.
### C.27 Panache d'antimatière, et un seul régime moteur (2026-07-31)

Le bouton **RADIATEURS: FROID/CHAUD** devient **PROPULSION: ALLUMEE/ETEINTE**, et
pilote désormais les deux : les ailes rougissent *et* les tuyères crachent.

**Un seul nombre, deux manifestations.** `regime` (0 à 1) remplace `chaleur` dans
la vue et se propage à `preset_isv_fixe`. Ce n'est pas de la simplification : les
radiateurs chauffent **parce que** les moteurs poussent. Deux réglages séparés
auraient permis un vaisseau qui pousse sans évacuer sa chaleur, ce qui n'existe
pas — et c'est exactement la « notion de régime moteur » que §C.22 annonçait
manquante.

> ⚠️ **Le rendu a changé le lendemain** — voir §C.28. Le panache n'est plus de
> la géométrie pleine mais un **ruban en additif**, comme les jets de pulsar. Le
> raisonnement de forme ci-dessous reste valable ; c'est la façon de le peindre
> qui était fausse.

**Ce n'est pas une flamme, et la forme le dit.** Trois conséquences, aucune
décorative :

- **pas de disques de Mach.** Ils demandent une pression ambiante pour
  recomprimer le jet ; dans le vide il n'y en a pas. Un panache perlé serait le
  dessin d'un moteur-fusée atmosphérique ;
- **détente libre et lente** — `t^1,45`, donc serré au col puis s'ouvrant, comme
  un plasma que le champ magnétique lâche progressivement. Un cône droit
  s'ouvrirait dès la sortie, ce qui est le dessin d'une tuyère sans confinement ;
- **il s'éteint en refroidissant**, pas en s'effaçant. Le rendu n'a pas de
  transparence : la disparition passe par la **valeur**, blanc-bleu → bleu →
  violet → magenta sombre → noir du fond. C'est d'ailleurs juste, un plasma qui
  se détend perd sa température de couleur exactement comme ça.

**Longueur : 336, soit deux longueurs de vaisseau**, comme demandé. Un jet de
pions relativistes n'a rien qui l'arrête, et sa portée dit ce que cette
propulsion a d'inhabituel. À l'allumage il **pousse** depuis la tuyère (15 % de
sa portée au ralenti) plutôt que d'apparaître d'un bloc : c'est la seule façon de
voir un allumage et non un interrupteur.

⚠️ **Le panache est un effet, pas une pièce** : coût nul, `rayon_local` nul,
englobant nul. Renvoyer sa vraie longueur ferait reculer la caméra de deux
longueurs de vaisseau au moment précis où l'on veut regarder le vaisseau. Et
moteur coupé il n'est **pas dessiné du tout** — un panache éteint n'est pas un
panache noir, ce serait un masque sur les étoiles.

**Le braquage de 5° prend enfin son sens.** Il a été décidé bien avant qu'il y
ait un panache à regarder, au motif que « le moteur ne tire plus dans la
station ». Mesuré maintenant : les jets partent de (−1,25 ; ±11,6) selon
(+0,996 ; ±0,087) — vers **+X**, c'est-à-dire le long du vaisseau remorqué, et
5° vers l'extérieur. C'est la configuration tracteur, et sans le braquage la
charge utile baignerait dans le plasma.

`le_panache_ne_leche_pas_la_charge_utile` mesure le jeu entre chaque jet et
chaque pièce **remorquée** (fret, habitat, équipage, plaques de tête —
l'ossature de propulsion est exclue, le jet en sort). Jeu minimal **3,93**, borné
à 1,0. C'est un rapport entre **trois** choses — l'angle des tuyères, l'évasement
du jet, le gabarit de ce qui est derrière — dont aucune ne se voit sur le
composant seul. Vérifié rouge des deux façons de le casser : bout à 40
(*« passe à 0,5 »*) et évasement à 0,5, c'est-à-dire un jet qui s'ouvre dès la
sortie (*« passe à −4,0 »*).
### C.28 Le panache repris comme un jet de pulsar (2026-07-31)

Grief à l'écran : le panache « ne donne pas le résultat voulu ». Diagnostic
juste du premier coup — *« regarde comment on a fait l'éjection polaire du
pulsar »*. Le vaisseau en avait déjà un, et il marche.

**La cause n'était pas la forme, c'était la matière.** Le profil, la longueur, la
rampe de couleur, le braquage : tout ça restait bon. Ce qui clochait, c'est que
je l'avais dessiné en **cônes pleins**, avec le `Peintre` comme tout le reste du
vaisseau. Or un jet de plasma **n'a pas de silhouette** — et ce sont exactement
les qualités d'un solide qui le trahissaient : une arête nette, une face opaque,
un bord franc sur le fond étoilé. D'où le tube de plastique planté dans la
tuyère.

**Le procédé du pulsar** (`shaders/soleil.frag.glsl`, branche
`couronne_type ∈ ]0,5 ; 1,5[`) fait tout l'inverse : un quad **face-caméra**, un
fragment shader qui reconstruit une **densité**, et un blending **additif**. Il
n'y a plus de surface, seulement une concentration : là où elle est faible, les
étoiles passent au travers. Repris tel quel, avec ce qu'un jet de tuyère demande
en plus — un **ruban** suivant l'axe plutôt qu'un disque, et un profil
d'évasement le long de sa longueur.

Trois choix hérités du pulsar, et chacun corrige un symptôme précis :

- **profil en cœur** en travers du ruban (`(1 − x²)^2,2`) : sans lui le ruban a
  une arête, et on retombe sur le tube ;
- **turbulence qui file vers le bout** (`fbm` avec un terme en `−time` sur l'axe
  du jet) : c'est ce qui distingue un jet d'un fuseau peint — la matière doit
  visiblement partir ;
- **prémultiplié en additif** : la composante noire n'ajoute rien, donc le bord
  du ruban s'éteint au lieu de se découper.

**Le composant ne dessine plus rien.** `panache::dessiner` est vide, et le dit :
`Composant::Panache` ne sert plus qu'à **porter la pose** — où est la tuyère, où
va le jet, à quelles cotes — dans l'assemblage. C'est exactement ce qu'un
composant sait faire et qu'un effet d'écran ne saurait pas : le panache hérite
ainsi de la transformée du bloc moteur, braquage de 5° compris, sans qu'on ait à
recalculer quoi que ce soit. Le rendu, lui, vit dans `ecran/panache.rs` avec son
material.

⚠️ **`depth_write: false`.** Le jet est un milieu, pas une surface : l'écrire
dans le Z-buffer masquerait le vaisseau et les étoiles derrière lui. Et il est
dessiné **après** la coque mais **dans** la passe pixelisée — sans quoi il
flotterait en net par-dessus un vaisseau pixelisé.

**Ce qui ne change pas** : le braquage, l'évasement `t^1,45`, la longueur de deux
vaisseaux, la rampe de température, et `le_panache_ne_leche_pas_la_charge_utile`,
qui mesure des cotes et non des pixels — il reste vert sans retouche. C'est
d'ailleurs le signe que la séparation est propre : changer la façon de peindre
n'a rien changé à la géométrie du problème.

**Largeur divisée par deux** dans la foulée, jugée à l'écran : bout 22 → **11**,
col 0,30 → **0,15** de la taille du moteur. Le col passe ainsi sous le rayon des
anneaux de stabilisation, ce qui est juste plutôt que gênant — une tuyère
magnétique **pince** le faisceau plus étroit que l'ouverture qui le laisse
passer. Jeu à la charge utile : 3,93 → **6,22**.

⚠️ Le test garde le fait **physique** (le jet ne baigne pas ce qui est remorqué),
pas la largeur du jour : à 6,2 de marge il ne rattraperait pas un doublement de
la cote. C'est délibéré — la largeur se juge à l'œil, le contact se calcule.
Vérifié rouge à bout = 45 (*« passe à −0,4 »*).
### C.29 ISV — asset CLOS (2026-07-31)

Validé à l'écran, propulsion allumée. **Le vaisseau est fini** : plus rien à
poser, plus rien à régler. Les navettes TAV sont explicitement **hors périmètre**
et ne bloquent pas la clôture.

| | éteint | allumé |
|---|---|---|
| Pièces | 43 | 45 |
| Coût | 725 | 725 |
| Longueur | 168,2 (X −45,2 → 123,0) | + panaches sur 336 |
| Rayon max | 12,6 | 12,6 |
| Sommets cuits | 110 008 en 204 lots | idem |

*(Les deux panaches comptent comme pièces mais ne produisent aucun sommet : ils
sont rendus à part, en additif.)*

Découpage en trois maillages, un par **degré de liberté** — le principe posé en
§C.7 et qui a tenu jusqu'au bout :

| | pièces | sommets | ce qui le fait recuire |
|---|---|---|---|
| Coque fixe | 40 | 102 130 | le régime moteur (chauffe + panache) |
| Section d'équipage | 7 | 7 878 | le repli |
| Panaches | — | — | rien : rendus par l'écran, pas cuits |

La rotation, elle, ne recuit **rien** : c'est une matrice.

**186 tests**, dont une trentaine propres à l'ISV. Aucun n'a été écrit après coup
pour décrire ce qui existait : chacun garde une **décision** — l'ordre des
sections, le braquage des tuyères, l'alésage qui doit laisser passer le mât, le
sens de recouvrement des écailles, le fait que le fret domine l'habitat.

#### Ce que ce chantier a appris, et qui vaut au-delà de l'ISV

1. **Brique d'abord, assemblage ensuite.** Aucune pièce n'est allée sur le
   vaisseau avant d'avoir été jugée seule. Les cinq allers-retours sur les
   boucliers de tête (§C.16 → §C.20) se sont tous faits sur la brique.
2. **Une cote qui se règle contre une autre pièce ne tombe pas sur une grille.**
   Vu trois fois : le collier d'équipage (§C.8), le rayon de fret, la largeur des
   grandes plaques. À chaque fois, sortir la cote de `Profil` était la réponse.
3. **Une source unique par famille de cotes.** `Epine::hors_tout()` a permis de
   passer l'épine de carrée à hexagonale sans recaler une seule cote de charge
   utile. Le seul endroit qui y avait échappé — la position de la tête — s'est
   fait attraper par un red-check (§C.23).
4. **Presque toutes les erreurs ont été des tests qui mesuraient autre chose que
   ce qu'ils prétendaient.** Jamais des erreurs de géométrie. Le catalogue :
   pièces comparées à des hauteurs différentes (§C.15), tranche prise là où un
   maillage cuit n'a pas de sommets (§C.13, §C.21), seuil recalculé au lieu
   d'être lu (§C.14), corollaire mesuré à la place de la chose (§C.24),
   circonradius comparés quand c'est le rayon inscrit qui touche (§C.25),
   proportion comptée quand 98 % des sommets sont dans une seule famille (§C.26).
5. **Le red-check est le seul filet.** Casser exprès ce qu'une assertion
   surveille. Il a trouvé trois défauts du **code** et non du test.
6. **La physique donne la forme, et il faut la suivre jusqu'au bout.** Les
   écailles se recouvrent dans le sens du flux, le radiateur refroidit vers sa
   pointe, le panache n'a pas de disques de Mach dans le vide, les tuyères sont
   braquées pour ne pas baigner ce qu'elles remorquent. Chaque fois qu'un choix a
   été fait « parce que c'est joli », il a fallu le refaire.
7. **Tout n'est pas de la géométrie.** Le panache l'a montré : un jet de plasma
   n'a pas de silhouette, et le peindre en solide le trahit quoi qu'on fasse
   (§C.28). Certains objets sont des **milieux** et se rendent en additif.

#### Dettes laissées ouvertes

- `cargo clippy` ne compile toujours pas (`src/planete/terrain.rs:154`) — dette
  d'avant ce chantier, sans rapport avec l'ISV ;
- 37 avertissements `dead_code` sur le binaire, tous antérieurs.
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

1. ✅ **Section charge utile — faite.** Fret et habitat principal (2026-07-29),
   modules d'équipage rotatifs (2026-07-30). Il ne reste que les **2 navettes
   TAV** amarrées.
2. ✅ **Boucliers de tête** — faits et posés (2026-07-30, §C.16 à §C.21). Ce
   ne sont pas les « plaques planes » prévues ici : le schéma remis à la main
   demandait une petite plaque nervurée et trois grandes plaques miroir,
   enfilées sur un mât. La note d'origine sous-estimait franchement la pièce.
3. ✅ **Proportions** — relecture faite le 2026-07-31 (§C.23) : équilibre
   d'ensemble conforme, une 4ᵉ rangée de fret ajoutée, trois rapports verrouillés
   par `les_proportions_densemble_de_lisv_tiennent`.
4. ✅ **Détails de surface** — bouclier thermique d'épine (§C.24–C.25), chauffe
   des radiateurs et **panache d'antimatière** (§C.26–C.27). Le tunnel
   pressurisé est abandonné : il ne se verrait pas à la silhouette.
5. ✅ **Test sur `preset_isv`** — fait avec le fret :
   `isv_porte_son_fret_a_loppose_des_moteurs` verrouille la disposition
   **tracteur** (fret nettement à l'opposé des moteurs) et le fait que les
   rangées sont **enfilées sur l'axe**, pas déportées sur un flanc.

### C.4 Ordre de travail proposé

Méthode retenue et qui marche : **la brique d'abord dans la vue BRIQUES**, on
valide sa forme à l'écran, et **seulement ensuite** on l'assemble sur l'ISV
(vue **MEGASTRUCTURES**, item « ISV — PROPULSION + FRET + HABITAT + EQUIPAGE »).

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
   `EPINE_SOMMET_Y = 76,4`, soit ~19 unités libres. L'habitat en prend 59→67 et
   la section d'équipage 69→73 ; il reste le bout (73→76,4) et tout le bas pour
   le bouclier antidébris.
3. ✅ **Section d'équipage rotative** (2026-07-30) — traverse + 2 modules, sur
   l'épine **au-dessus de l'habitat** (`EQUIPAGE_CENTRE_Y = 71`). Trois briques
   neuves dans `composant/equipage.rs` :
   - `ModuleEquipage` — nacelle **cylindrique**, et c'est voulu : sous rotation,
     une section triangulaire ferait changer l'inclinaison du « bas » le long de
     la paroi. Plancher = calotte bombée au bout **+Z** (vers l'extérieur, là où
     pousse la force centrifuge), couronne de hublots juste au-dessus ;
   - `CollierRotatif` — le tambour qui ceinture l'épine. Son alésage a été
     **resserré sous** le gabarit de la flèche depuis (§C.7) : la structure le
     traverse, au lieu de flotter dans un jour visible ;
   - `Charniere` — chape à deux joues, axe traversant, et **vérin télescopique**
     qui s'allonge réellement avec le repli.

   *Deux pièges corrigés* : (a) des bagues de roulement descendant jusqu'à
   l'alésage partagent leur paroi intérieure avec celle du tambour → z-fighting
   plein l'alésage ; ce sont des **frettes extérieures**, le vide central reste
   net ; (b) la rotation avait d'abord été appliquée autour de **Y** alors que la
   démo posait le collier le long de Z — l'axe dépend du repère de la vue, d'où
   `VueStation::axe_rotation()`.

   **Créneau de montage**, verrouillé par
   `la_section_dequipage_se_glisse_entre_lhabitat_et_le_bout_de_lepine` : le
   collier va de Y 84,2 à 86,2, entre le sommet de l'habitat (79,6) et celui de
   l'épine (`EPINE_SOMMET_Y × ISV_ECHELLE` = 91,7). Trop bas il tournerait dans
   les modules fixes, trop haut autour de rien — deux bornes invisibles sans faire
   pivoter la caméra. ⚠️ Ces Y sont **à l'échelle du vaisseau** : les centres sont
   multipliés par `ISV_ECHELLE`, les longueurs de composants non (§C.7).

   **Rendu en deux maillages** : `preset_isv_fixe()` (la coque) et
   `preset_isv_equipage(repli)` (ce qui tourne). La rotation est alors une simple
   matrice modèle poussée sur la seconde moitié, au lieu de recuire tout le
   vaisseau à chaque frame. `preset_isv()` reste le vaisseau d'un seul tenant,
   utilisé par les tests. Le **repli**, lui, déplace des sommets et impose bien
   une reconstruction — mais `recuire_repli()` ne refait que la moitié tournante.

   Les deux boutons (rotation, repli) sont actifs sur la brique **et** sur l'ISV
   complet, grisés ailleurs. Le repli porte un sens : replié en transit, déployé
   en orbite (`EtatEquipage`).
4. ⛔ **Navettes TAV** — **hors périmètre**, décidé le 2026-07-31. Elles
   restent le candidat idéal au composite `SousEnsemble` (Partie E.3) le jour où
   elles reviendront à la feuille de route, mais l'ISV est clos sans elles.
5. ✅ **Boucliers de tête** — briques faites **et posées** (2026-07-30).
   `Composant::BouclierPetit` + `Composant::BouclierGrand`, vues Briques n° 21 et
   22, montés sur l'ISV au bout opposé aux moteurs (§C.16, §C.21). La pile
   Whipple de §C.8 est **abandonnée**, et la question « à quel bout ? » qu'elle
   laissait ouverte est **tranchée**.
6. ✅ **Relecture des proportions d'ensemble** — faite le 2026-07-31 (§C.23).

### C.7 Section d'équipage divisée par deux, et collier enfoncé dans l'épine (2026-07-30)

Deux griefs à l'écran, tous deux sur la section d'équipage : elle **écrasait la
silhouette** (au point de sembler chevaucher les panaches de propulsion), et son
collier avait l'air d'une **chaussette enfilée sur l'épine** — pas d'une pièce
montée dessus.

**1. Gabarit divisé par deux.**

| | avant | après |
|---|---|---|
| `EQUIPAGE_LONG` (module) | 7,0 | **3,5** |
| `EQUIPAGE_BRAS` (envergure) | 9,0 | **5,5** |
| `EQUIPAGE_COLLIER` (longueur) | 4,0 | **2,0** |
| `EQUIPAGE_CHARNIERE` | 0,62 | **0,31** |
| profil du module et du bras | P1 (1,0) | **P0 (0,5)** |
| demi-envergure hors-tout | 16,0 | **9,0** |

Les rayons passent par un **cran de `Profil`** (P1 → P0) et non par un facteur :
`Profil` est discret, c'est le seul moyen de diviser une section par deux sans
sortir la mise à l'échelle géométrique.

⚠️ **La demi-envergure tombe à 9 et non à 8** : le rayon du collier (P2 = 2,0)
ne suit pas la réduction, parce qu'il est imposé par l'épine qu'il doit
envelopper et non par les proportions de la section. Il consomme une constante
de 2 unités sur l'envergure. Même logique que pour le fret — les cotes calées
sur l'épine ne se retouchent pas avec le reste.

**2. Le collier avale l'épine** — `EQUIPAGE_ALESAGE` : `EPINE × 1,35` (1,51) →
`EPINE × 0,45` (**0,50**). L'alésage passe donc **sous** le hors-tout de la
flèche : les membrures traversent la paroi du tambour et il ne subsiste aucun
jour entre les deux. On ne voit plus que les surfaces extérieures du collier.

C'est un **arbitrage rendu contre le mécanisme** : un vrai palier demanderait
justement le jeu qu'on vient de supprimer. Mais ce jeu se voit, et ce qu'il donne
à lire est faux — une pièce posée autour de la flèche au lieu d'être solidaire
d'elle. Le test `le_collier_dequipage_tourne_librement_autour_de_lepine`
affirmait l'ancienne règle ; il est remplacé par
`le_collier_dequipage_enveloppe_lepine_sans_jour`, qui dit **l'inverse** et le
signale en commentaire, pour que personne ne « répare » le sens de ces assertions
sans avoir regardé la vue. Les deux bornes restent serrées : sous l'épine pour
l'alésage, au-dessus pour la jaquette (sinon les longerons ressortent à travers).

**Le chevauchement des panaches n'existait pas.** Mesuré sur les pièces cuites :
la coque va de X −25,2 à 75,6 (rayon ≤ 11,2, les ailes radiateur), la section
d'équipage est à X 85,2 (rayon ≤ 9,0). Les tuyères sont donc à **~110 unités**
d'elle, à l'autre bout du vaisseau : ce que l'œil lisait comme une intersection
était le cercle balayé par les nacelles se superposant aux moteurs **en
projection** seulement. La réduction reste la bonne correction — la section
dominait bel et bien la silhouette — mais elle ne corrigeait pas une collision.

🐛 *Erreur trouvée en mesurant* : le test de créneau de montage écrit la veille
comparait un `*_CENTRE_Y` **brut** à une demi-longueur de composant. Or les
centres sont multipliés par `ISV_ECHELLE` à la pose et les longueurs non — le
test raisonnait en unités mélangées. Il se trouvait conservateur (il annonçait
3 unités de jour là où il y en a 4,6), donc aucun faux vert, mais il aurait
dérivé au prochain changement d'`ISV_ECHELLE`. Corrigé, et il mord toujours des
deux côtés (vérifié rouge à `EQUIPAGE_CENTRE_Y` = 64 et 77).

### C.8 Collier réduit de 30 %, et bouclier antidébris en brique (2026-07-30)

**1. Collier de rotation : 2,0 → 1,4 de rayon** (−30 %), longueur 2,0 → 1,4. Le
tambour dominait encore le centre de la section après le halvage de §C.7.

Le rayon du collier **a cessé d'être un cran de `Profil`** : il est devenu un
`rayon: f32` sur `Composant::CollierRotatif`, exactement comme le `rayon` de
`RatelierCargo`. Raison de fond — c'est une cote qui se règle contre la structure
traversante, et l'échelle `Profil` (0,5 / 1 / 2 / 3) n'a **rien entre 1,0 et
2,0** : le cran du dessous passe *sous* le hors-tout de la flèche (1,12), donc
les longerons ressortiraient à travers la jaquette. `profil` ne sert plus qu'à
déclarer les ports. C'est la deuxième fois que ce besoin apparaît sur l'ISV : à
l'échelle vaisseau, les cotes calées sur une autre pièce ne tombent pas sur la
grille des profils.

À 1,4 il ne reste que **0,28 de marge** au-dessus de l'épine — la borne est
gardée par `le_collier_dequipage_enveloppe_lepine_sans_jour` (vérifié rouge à
1,05). Effet de bord voulu : rétrécir le collier **allonge le bras** d'autant
(`bras_long = EQUIPAGE_BRAS − r_collier`), donc la demi-envergure ne bouge pas.
C'est `EQUIPAGE_BRAS` qui tient la silhouette, pas le tambour.

**2. Bouclier antidébris (IDPS) — brique faite, pas encore posée.**

> ⚠️ **Cette brique a été remplacée le 2026-07-30 même** — voir §C.16. La pile
> Whipple de plaques cambrées décrite ici n'existe plus dans le code ; ce qui
> suit ne vaut plus que comme trace du raisonnement, dont la partie *physique*
> (l'espacement est le blindage) reste vraie et a été reprise telle quelle.

`Composant::BouclierDebris { profil, rayon, couches, ecart }`, dans le nouveau
`composant/bouclier.rs` ; vue **Briques n° 21** (une barrière seule pour juger la
cambrure, puis la pile complète). Briques : 22 → **23 items**.

C'est un **bouclier Whipple**, et la forme découle entièrement de ça : à 0,7 c un
grain de poussière n'est pas un impact mais une détonation, et aucune plaque
monolithique n'y résiste. Donc — barrière de tête **sacrificielle** (elle vaporise
le grain au lieu de l'arrêter), barrières suivantes **espacées** qui encaissent un
nuage déjà dilué. ⚠️ **L'espacement *est* le blindage** : c'est lui qui laisse au
plasma la place de s'étaler (~100 m sur le vrai ISV). Une pile resserrée n'est
plus un Whipple mais un feuilleté, qui ne pare rien — d'où
`le_bouclier_reste_un_whipple_etage`, qui garde le rapport écart/plaque.

Deux conséquences de forme qui ne sont pas décoratives :
- **plaques cambrées** en pyramide très plate, apex vers l'avant : aucune facette
  perpendiculaire au flux, donc impacts tous obliques (trajet plus long dans la
  matière, ricochet favorisé). C'est aussi ce qui donne l'aspect « anguleux »
  décrit en §C.2 plutôt qu'un disque ;
- **barrières arrière plus larges** que la tête (+10 % par étage) : le cône de
  débris s'ouvre en s'éloignant d'elle, c'est donc en remontant vers le vaisseau
  qu'il faut de la surface. La pile s'évase vers la coque, pas vers l'avant.

🐛 *Piège repris à l'identique, attrapé par les tests* : la barrière arrière posée
à ras du plan de montage avait son cordon de jante **à cheval sur l'interface**,
donc à moitié dans la pièce d'en face — troisième récidive du motif après les
collerettes de fret et les ferrures d'habitat. D'où `talon()`, un recul de
0,10 rayon que l'embase occupe exactement, et la section des membrures prise sur
le rayon **de base** (pas sur la plaque évasée) pour que ce talon soit calculable
une fois pour toutes. Vérifié rouge avec `talon = 0`.

❓ **Question ouverte avant de l'assembler : à quel bout ?** Nos propres notes se
contredisent, et il faut trancher à l'écran :
- §C.2 décrit les plaques « **en avant du vaisseau** » et la charge utile
  « **arrière** » — l'ISV est un **tracteur** (moteurs devant, charge remorquée,
  panaches inclinés de 5° pour ne pas tirer dedans, cf.
  `isv_porte_son_fret_a_loppose_des_moteurs`). L'avant serait donc le **bout
  moteurs** (X ≈ −25) ;
- mais §C.4 et §C.6 annoncent le bouclier sur le **haut d'épine libre**, à
  l'opposé (X ≈ 92).

Les deux ne peuvent pas être vrais. À décider avec la vue Briques validée en
main, sachant qu'un bouclier au bout moteurs devra composer avec les tuyères et
les ailes radiateur (rayon 11,2), alors que le haut d'épine est dégagé.

### C.9 Épine hexagonale — variante candidate, non assemblée (2026-07-30)

Deux griefs sur l'épine carrée actuelle : elle **se voit mal de loin sous le
filtre pixel**, et sa section carrée est devenue la **dernière forme isolée** du
vaisseau (tout ce qu'elle porte est hexagonal ou triangulaire — cadre de
propulsion, montures d'habitat, sections onigiri, bras d'équipage).

⚠️ **Rien n'est remplacé.** `preset_isv` utilise toujours
`Composant::Charpente`. La variante vit à côté, sous
`Composant::CharpenteHexa`, et se juge en **vue Briques n° 22**, qui aligne les
quatre spécimens : carrée nue, hexagonale nue, carrée avec cadre, hexagonale avec
cadre. Briques : 23 → **24 items**.

**Le calcul qui justifie la forme.** La largeur apparente d'un polygone régulier
de circonradius `R` oscille avec l'angle de vue entre `2R·cos(π/n)` (vu de face)
et `2R` (vu par un sommet) :

| | mini | maxi | rapport |
|---|---|---|---|
| carré (n=4) | 1,414 R | 2 R | **1,41** |
| hexagone (n=6) | 1,732 R | 2 R | **1,15** |

Le circonradius est **repris tel quel** de la version carrée (`sg·√2`, la
distance à ses coins, d'où le `√2` dans `hexa_rayons`). Conséquence : l'épine
hexagonale a exactement le **même encombrement maximal** que l'ancienne — elle ne
grossit pas — mais elle est **22 % plus large dans son pire angle**. Et c'est le
pire angle qui décide : sous le filtre pixel, un montant qui tombe sous le pixel
sur trois quarts des orientations clignote au lieu de se dessiner. Un hexagone
garde une épaisseur quasi constante d'où qu'on le regarde.

Mesuré numériquement (balayage de 3600 angles) et non déduit, par
`la_section_hexagonale_est_plus_constante_que_la_carree` : gain 1,2247 =
`√3/√2` = `pieces::HEXA_GAIN_SILHOUETTE`, encombrement maximal identique à 10⁻².

**Le pied : cadre plat basculé de 90°, devenu une tour.** Premier essai raté, et
la correction vaut d'être notée parce qu'elle supprime du code au lieu d'en
ajouter.

`pieces::treillis_hexagone` produit un hexagone **couché** : son plan *contient*
l'axe de la poutre. Il se présente donc **de travers** à une poutre axiale, et
j'avais raccordé les deux par une jupe vrillée d'un quart de tour — les deux
longerons de l'axe X droits, les quatre autres en torsion. J'ai décrit cette
vrille comme « une vraie baie de torsion » ; à l'écran elle ne se lisait pas comme
une baie mais comme un accostage manqué, et le grief était juste.

**Basculé de 90°**, l'hexagone a sa section **perpendiculaire à l'axe**, donc
parallèle à celle du cône : il n'y a plus rien à raccorder. La tour
(`pieces::tour_hexagonale`, prisme hexagonal de 3 étages, hauteur
2,4 circonradius) **prolonge** le cône au même rayon, et les six longerons
descendent tout droit. Toute la jupe vrillée a disparu.

⚠️ L'accostage est exact **par construction, pas par réglage** : les sommets de la
tour et la section du cône sortent de la **même** `hexa_section`, avec les mêmes
axes et le même rayon `rg`. Il n'y a aucune valeur à faire coïncider, donc rien
qui puisse dériver. Corollaire : la tour ne dessine **pas** de ceinture à son
niveau 0 — le cadre de base du cône y est déjà, et deux anneaux coplanaires,
c'est du z-fighting garanti (le motif habituel du projet).

Le pied a perdu au passage la propriété d'être **plus large** que l'épine : la
tour fait exactement le rayon de base du cône, là où l'ancien cadre valait
`2·sg`. Les blocs de propulsion viendront donc se poser sur le **plancher** de la
tour (trois cordes en étoile entre sommets opposés) plutôt que sur un cadre
débordant. À revoir si la propulsion a besoin de plus d'emprise.

`la_tour_du_pied_prolonge_le_cone_sans_se_rétrecir` mesure les rayons cuits de
part et d'autre de la jonction et en bas de tour : la tour doit partir du rayon du
cône et ne pas se rétrécir (c'est un prisme, pas un second cône). Vérifié rouge en
donnant à la tour 1,35 fois le rayon du cône.

### C.10 Second ISV complet, en épine hexagonale (2026-07-30)

Plutôt que de basculer `preset_isv`, **les deux vaisseaux coexistent** :
`Megastructures` compte désormais 4 items (3 → 4), dont deux ISV complets — item 1
« EPINE CARREE », item 2 « EPINE HEXAGONALE ». Même mécanique pour les deux : rendu
en deux maillages, rotation et repli de la section d'équipage.

⚠️ **Les deux presets partagent tout le code sauf une ligne.** `isv(epine,
avec_equipage)` est le seul constructeur ; `preset_isv()` et `preset_isv_hexa()`
n'en sont que deux appels. C'est délibéré : à la moindre duplication la
comparaison ne voudrait plus rien dire, puisqu'un écart pourrait venir d'une dérive
entre deux copies au lieu de la section d'épine.

**Le gabarit se propage au lieu d'être écrit en dur.** C'était le vrai piège
annoncé en §C.9. `EPINE_FLECHE` vaut `0,5·√2 + 0,225` pour le carré ; l'hexagone
donne `0,5·√2 + 0,2546`. Même forme, et ce n'est pas un hasard — le circonradius
hexagonal est **repris des coins du carré**, donc les sommets sont à la même
distance de l'axe, et seule l'épaisseur des longerons diffère (`0,12·1,5·√2` contre
`0,15·1,5`). L'épine hexagonale est donc **3,2 % plus large hors-tout**.

3,2 % paraît négligeable ; c'est exactement l'ordre de grandeur qui a replanté la
charge utile dans la structure en §C.6. Toutes les cotes dérivées sont donc
devenues des `const fn` du gabarit :

| | avant | après |
|---|---|---|
| `FRET_RAYON` | `const` sur `EPINE` | `const fn fret_rayon(epine)` |
| `HAB_RAYON` | `const` sur `EPINE` | `const fn hab_rayon(epine)` |
| `EQUIPAGE_ALESAGE` | `const` sur `EPINE` | `const fn equipage_alesage(epine)` |

Les constantes historiques survivent comme **appels** de ces fonctions
(`const FRET_RAYON: f32 = fret_rayon(EPINE);`), si bien que le calcul n'existe
qu'une fois et que le premier ISV est garanti inchangé —
`le_gabarit_carre_est_inchange_par_lajout_de_lhexagonal` le vérifie.

**Trois niveaux de test, parce que deux ne suffisaient pas :**
1. `la_charge_utile_suit_le_gabarit_de_lepine` **boucle maintenant sur les deux
   variantes** — ajouter une épine sans recaler les couronnes devient impossible ;
2. `le_gabarit_carre_est_inchange_par_lajout_de_lhexagonal` épingle les cotes
   historiques et vérifie que l'écart hexagonal tombe bien dans les ~3 % ;
3. `le_second_isv_recale_vraiment_sa_charge_utile` vérifie le **câblage**, pas les
   formules : il **construit** les deux presets, y retrouve le râtelier de fret et
   le collier, et compare leurs cotes réelles. Les points 1 et 2 valident des
   calculs justes ; celui-ci attrape le cas où le calcul juste est appliqué au
   mauvais gabarit — littéralement le bug de §C.6. Vérifié rouge en recâblant le
   fret sur `FRET_RAYON` : *« couronne de fret non recalée (2,1452 vs 2,1452) »*.

**Marge la plus serrée à surveiller** : la jaquette du collier
(`EQUIPAGE_COLLIER_RAYON = 1,4`) doit dépasser la flèche, sinon les longerons
ressortent à travers. L'épine carrée laissait 0,28 ; l'hexagonale ne laisse plus
que **0,246**. C'est désormais dans la boucle de test.

**Reste à faire** : ajouter de la structure sur la propulsion, maintenant que le
pied est une tour et non plus un cadre débordant (§C.9) — c'est la prochaine étape
demandée. Et trancher, à terme, laquelle des deux épines garder : rien n'oblige à
le faire tant que les deux presets vivent côte à côte.

### C.11 Pied en pavillon : le cône s'épanouit jusqu'à la propulsion (2026-07-30)

Demandé sur schéma (« spine wanted state ») : la tour du pied (§C.9) est refusée,
**le cône doit continuer de s'ouvrir** jusqu'à une large embouchure hexagonale, qui
sera l'interface avec la propulsion.

⚠️ **La tour est conservée**, comme demandé : `PiedHexa` remplace le `aiguille:
bool` de `CharpenteHexa` et vaut `Aucun` / `Tour` / `Pavillon`. Nouvelle brique de
comparaison en **vue Briques n° 23** (« PIED TOUR vs PIED PAVILLON »), même cône,
seul le pied change. Briques : 24 → **25 items**. Les deux presets ISV restent en
pied `Tour` — rien n'est monté sur le vaisseau.

**L'embouchure**, et la lecture du schéma. Le bord est un **simple hexagone**, fermé
par une jante plus forte que les ceintures intermédiaires. Le second hexagone
intérieur et ses six panneaux radiaux, présents au premier jet, ont été **retirés**
(2026-07-30) : jugés inutiles à l'écran, ils encombraient une embouchure qui doit
rester ouverte sur les tuyères.

Le schéma annote les six côtés `A`…`F` avec deux égalités :
`A = B = D = E` et `C = F`. C'est **la** contrainte utile, parce qu'un hexagone
régulier n'a qu'une seule famille d'arêtes — six côtés égaux. Obtenir exactement
4 + 2 impose donc que la section soit **écrasée sur un axe** :

- les deux sommets portés par X ne bougent pas, donc les deux arêtes
  perpendiculaires à Y (`C` en bas, `F` en haut) **gardent la longueur du
  rayon** ;
- les quatre arêtes obliques (`A`, `B`, `D`, `E`) raccourcissent, à égalité entre
  elles par symétrie.

D'où `PAVILLON_ETIREMENT`. À 1 l'hexagone redeviendrait régulier et les deux
familles se confondraient — la contrainte du schéma serait satisfaite mais vide de
sens. Aucune primitive nouvelle n'a été nécessaire : `hexa_section` reçoit un
vecteur `h` déjà mis à l'échelle, ce qui écrase la section sans toucher aux
sommets portés par `d`.

**Écrasement resserré à 0,55 (était 0,82) — silhouette « taille émeraude ».** Le
premier réglage rapprochait trop peu les deux grands côtés : leur rapport aux
biseaux n'était que de **1,15**, et la section lisait comme un hexagone vaguement
irrégulier plutôt que comme une pierre taillée. Ce n'est donc pas la *différence*
entre les deux familles qui fait la forme, mais leur **contraste** :

| écrasement | largeur / hauteur | grand côté / biseau | |
|---|---|---|---|
| 0,82 | 1,41 | 1,15 | les familles se distinguent à peine |
| **0,55** | **2,10** | **1,45** | retenu |
| 0,45 | 2,57 | 1,58 | l'hexagone commence à s'aplatir en losange |

Les deux grands côtés valent toujours exactement le rayon, quel que soit
l'écrasement : ils sont portés par X, que l'écrasement ne touche pas. C'est donc la
**hauteur** seule qui bouge.

`lembouchure_a_quatre_aretes_obliques_et_deux_droites` verrouille maintenant le
contraste (1,35 < rapport < 1,9) et l'allongement (1,8 < largeur/hauteur < 2,8),
pas seulement l'inégalité des familles — sans quoi l'ancien 0,82 resterait vert.
Vérifié rouge **des deux côtés** : à 0,82 (trop peu contrasté) et à 0,40
(*« largeur/hauteur = 2,89, hors du gabarit émeraude visé »*).

**Fût de couronnement** (2026-07-30). Une tour droite coiffe l'embouchure : c'est
elle qui portera la propulsion, au-delà de la corolle. D'abord posée en **virole
courte** (0,8 rayon de cône) le temps de juger la forme, puis **hauteur ×6**
(`PAVILLON_TOUR_HAUTEUR` 4,8, 6 niveaux) : ce n'est plus une bague d'interface mais
un vrai fût, à peu près aussi haut que l'embouchure est large (10,2 contre 8,9).

Le test de gabarit a suivi ce changement d'intention : il exigeait « plus courte que
la moitié de l'embouchure », ce qui disait désormais **l'inverse** de ce qu'on veut.
Il a été **remplacé** — élancement du fût entre 0,7 et 1,6 fois la largeur
d'embouchure, et hauteur sous 35 % de l'épine — et non simplement assoupli.

⚠️ **Elle se pose sur une section écrasée, pas sur un hexagone régulier.**
`pieces::tour_hexagonale` a donc gagné un paramètre `etirement` : la tour du *pied*
le laisse à 1 (elle prolonge la base du cône, qui est régulière), celle du pavillon
reprend `PAVILLON_ETIREMENT` (elle prolonge l'embouchure). La réutiliser telle
quelle aurait recréé, à l'autre bout, le désaccord tout juste corrigé au col — deux
sommets alignés sur X, quatre obliques décalés.

`la_tour_du_pavillon_reprend_la_section_de_lembouchure` mesure les étendues X et Y
de part et d'autre du plan d'embouchure : la tour doit être écrasée comme elle, du
même rayon, et rester courte. Vérifié rouge à `etirement = 1` et à
`TOUR_HAUTEUR = 3` (*« tour haute de 6,364 pour une embouchure large de 9,979 : ce
n'est plus une virole »*).

🐛 *Piège de mesure rencontré ici* : un cylindre cuit ne porte de sommets **qu'à
ses deux bouts**. Une tranche prise au milieu d'une baie est donc vide, et la mesure
y rendait `NaN` — silencieusement comparé, un `NaN` fait passer bien des
assertions. Il faut échantillonner **aux niveaux** du treillis.

**Réglages du pavillon** (les deux premiers en rayons de base de cône) :
`PAVILLON_OUVERTURE` 2,1 · `PAVILLON_HAUTEUR` 2,0 · `PAVILLON_ETIREMENT` 0,55 ·
`PAVILLON_ETAGES` 3 · `PAVILLON_TOUR_HAUTEUR` 0,8 · `PAVILLON_TOUR_ETAGES` 2.
Flancs **droits** et non en courbe de cloche, le tracé du schéma étant rectiligne.

🐛 **L'accostage au cône était faux, et je l'avais annoncé exact.** J'avais repris
tel quel l'argument de la tour (§C.9) — « le col part de `rg`, les sommets sortent
de la même `hexa_section` » — sans voir qu'il **cesse de tenir dès qu'il y a un
écrasement** : le col était écrasé de 0,55 alors que la base du cône est un hexagone
**régulier**. Les deux sommets portés par X coïncidaient (l'écrasement ne les
touche pas), mais les **quatre obliques** tombaient à un autre Y. D'où le raccord
qui se voyait sur les grands côtés.

Correction : **l'écrasement est progressif**. Il vaut 1 au col — donc section
régulière, superposable à la base du cône — et n'atteint `PAVILLON_ETIREMENT`
qu'à l'embouchure (`pieces::etirement_progressif`). La section *morphe* le long de
la corolle, de l'hexagone régulier vers la pierre taillée. Effet secondaire
heureux : la transition se lit mieux qu'un écrasement uniforme.

⚠️ **Le test censé couvrir l'accostage ne pouvait pas voir ce bug.**
`le_pavillon_souvre_au_lieu_de_prolonger_droit` ne compare que des **rayons**, or un
écrasement selon Y laisse le rayon maximal **inchangé** (les sommets portés par X
sont intacts). Vérifié : en remettant le bug, ce test reste vert. D'où
`le_col_du_pavillon_epouse_la_section_du_cone`, qui mesure les étendues X et Y
**séparément** et compare leur rapport à celui d'un hexagone régulier (2/√3).

🐛 *Second défaut, dans ce nouveau test* : mesurée sur la charpente complète, la
tranche du col contient aussi le **cadre de base du cône**, dont l'épaisseur déborde
sous le plan de jonction. C'est lui qu'on mesurait — régulier — et le test restait
vert avec un col écrasé. Il mesure donc le pavillon **seul**, primitive appelée
directement. Confirmé rouge des deux façons : formule contournée *et* géométrie
écrasée à la main (*« au col, largeur/hauteur = 1,936 au lieu de 1,155 »*).

**Deux tests, chacun sur une affirmation du schéma :**
- `le_pavillon_souvre_au_lieu_de_prolonger_droit` — mesure les rayons cuits au col
  et au bord : le col doit coïncider avec la base du cône, le bord dépasser de
  moitié celui de la tour, et l'englobant suivre l'ouverture **radiale** (le
  pavillon déborde en rayon, pas seulement en longueur, d'où
  `charpente_hexa_pied_rayon`). Vérifié rouge à `OUVERTURE = 1,05` ;
- `lembouchure_a_quatre_aretes_obliques_et_deux_droites` — vérifie les deux
  familles, qu'elles sont distinctes, **et** le gabarit émeraude (contraste et
  allongement, cf. ci-dessus). Vérifié rouge à `ETIREMENT = 1,0` : *« les deux
  familles se confondent (3.0000 vs 3.0000) »*.

🐛 *Défaut de test attrapé en le vérifiant rouge* : la première version recopiait
`0,82` dans le test au lieu de lire `PAVILLON_ETIREMENT`. Elle restait donc verte
alors que la pièce livrée avait perdu son écrasement — un test qui se vérifie
lui-même. La constante est passée `pub(super)` et le test lit la vraie valeur.

### C.12 Le pavillon monté sur l'ISV hexagonal (2026-07-30)

`Epine::Hexagonale` construit désormais sa charpente en `PiedHexa::Pavillon`.
C'est le **seul** écart supplémentaire entre les deux ISV : l'épine carrée garde son
cadre couché, et tout le reste du code est partagé.

**Mesuré sur les sommets cuits**, pour savoir ce qu'on regarde :

| | épine carrée | épine hexagonale |
|---|---|---|
| la charpente descend jusqu'à | X = −17,3 | **X = −26,9** |
| rayon max du pied (X < −12) | 5,1 | **6,0** |
| bout du modèle (tuyères) | X = −44,0 | X = −44,0 |

✅ **Aucune interpénétration.** Le fût occupe X ∈ [−9,1 ; −26,9] à un rayon ≤ 6,0,
tandis que les deux ensembles moteurs pendent à un rayon de **11,2** de l'axe : le
fût passe **entre** eux, avec ~5 unités de dégagement. Il descend 9,6 unités plus
bas que l'ancien pied carré sans rien toucher.

⚠️ **Mais le fût ne porte encore rien.** Les moteurs restent accrochés aux ailes
radiateur, comme du temps du cadre couché — le fût est donc structurellement
**inerte** pour l'instant. Le reposer sur l'embouchure reste à faire, et c'est à ce
moment que se posera la question du dégagement des tuyères (le fût descend à −26,9,
les tuyères jusqu'à −44).

### C.13 Dégagements de la propulsion et des cuves (2026-07-30)

Deux recouvrements relevés à l'écran sur l'ISV hexagonal, tous deux corrigés en
**découplant des cotes qui avaient été confondues**.

**1. La corolle traversait les ailes radiateur.** Le déport latéral de la
propulsion (`PROPULSION_DEPORT`) passe de 6,5 à **9,5**.

⚠️ Il a fallu **mesurer** plutôt que déduire : le rayon interne de l'aile n'est pas
`déport − largeur/2`, parce que le collecteur du radiateur rentre plus près de l'axe
que la pointe des ailettes. Relevé sur la géométrie cuite :

| déport | rayon interne de l'aile | rayon du pied | jeu |
|---|---|---|---|
| 6,5 (d'origine) | 3,14 | 5,99 | **−2,85** |
| 8,5 | 5,54 | 5,99 | −0,45 |
| 9,0 | 6,14 | 5,99 | +0,15 |
| 9,5 | 6,74 | 5,99 | +0,75 |

L'estimation analytique donnait « +2 suffit » (pointe d'ailette à 5,86 contre un
pied à 5,0) ; à l'échelle du vaisseau elle laissait encore 0,45 de recouvrement.
`la_propulsion_degage_le_pied_de_lepine` mesure donc les deux rayons sur le preset
**construit**, et non des cotes. Vérifié rouge au déport d'origine.

### C.14 Barres du fût affinées, propulsion resserrée (2026-07-30)

**1. Les barres du fût étaient calées sur la mauvaise section.**
`tour_hexagonale` dimensionnait ses montants à `rayon × 0,12`, `rayon` étant celui
de la tour — donc, pour le fût du pavillon, celui de l'**embouchure**, 2,1 fois la
base du cône. Ses barres faisaient 0,53 de rayon là où les longerons de la corolle,
juste au-dessus, en font 0,25 : le fût paraissait bien plus lourd que ce qu'il
prolonge.

Le principe manquant est maintenant nommé (`pieces::LONGERON`) : **un longeron ne
s'épaissit pas parce que la section s'évase**. Le cône hexagonal et le pavillon le
respectaient déjà (tous deux prennent la section de leur base) ; la tour non.
`tour_hexagonale` reçoit désormais son épaisseur en paramètre, ceinture et diagonale
s'en déduisant par les rapports d'origine (0,75 et 0,583). La tour du **pied** est
inchangée au bit près — sa section *est* `rg` — et celle du pavillon passe de 0,53
à **0,25**, en continuité exacte avec les longerons de la corolle.

Effet de bord utile : le rayon dessiné du pied tombe de **5,99 à 5,65**.

**2. Propulsion resserrée** : `PROPULSION_DEPORT` 9,5 → 8,5 → **6,5**, jusqu'à ce
qu'elle se **pose sur** l'épine au lieu de la longer.

### C.15 La propulsion posée sur l'épine, et une mesure qui mentait (2026-07-30)

**Avancée vers les tuyères.** `PROPULSION_AVANCE` = 1,0 décale propulsion **et**
cuves le long de l'épine, côté tuyères (sens choisi en arbitrage).

🐛 **Le test d'engagement mesurait deux pièces à des hauteurs différentes.** Il
comparait le rayon **minimal** de l'aile — pris sur toute sa longueur — au rayon
**maximal** du pied. Or l'aile plonge bien plus près de l'axe *sous* le fût qu'au
droit de celui-ci. Il annonçait donc un engagement de **−1,31** là où, dans la
tranche réellement partagée, il n'y avait que **−0,04** : les deux pièces se
frôlaient sans se toucher.

Corrigé : les deux mesures sont restreintes au **recouvrement axial** des pièces, et
son absence est en soi un échec. Le test a immédiatement révélé la vraie valeur.

⚠️ **Ce que la correction a mis au jour : la fenêtre est minuscule.** L'aile ne
croise le fût que sur **1,9 unité** (avance 0), au ras de sa pointe — tout le reste
de la propulsion pend sous la structure. Avancer vers les tuyères **consomme** cette
fenêtre :

| avance | recouvrement axial | engagement au droit du fût |
|---|---|---|
| 0,0 | 1,9 | −1,24 |
| **1,0** (retenu) | **0,7** | **−1,24** |
| 1,5 | 0,1 | mesure sans objet |
| 2,0+ | néant | l'aile est passée sous le fût |

**1,0 est donc à peu près le maximum utile** : au-delà, la propulsion quitte
l'épine et l'engagement latéral ne sert plus à rien. Le test refuse désormais un
recouvrement inférieur à 0,3, parce qu'un liseré ne porte que quelques sommets de
bord : à 1,5 le « jeu » mesuré bascule à +1,34, l'inverse de la réalité — une mesure
sur trop peu de matière est pire qu'aucune mesure.

**Déport final 6,5** → engagement de **−1,24** au droit du fût. Trois bornes
vérifiées rouges : déport 9,5 (*« engagement de 2,36 »*), avance 1,5
(*« recouvrement axial de 0,09 »*), avance 3,0 (*« −1,71 »*).

⚠️ Ce test a maintenant changé de sens **deux fois** — dégagement exigé, puis contact
toléré, puis engagement exigé — chaque fois délibérément. Le commentaire en tête
retrace les trois états pour qu'on ne « corrige » pas le sens des assertions sans
avoir regardé la vue.

**Le fond du problème reste entier** : la propulsion pend *à côté* de l'épine et n'y
touche que par la tranche. La reposer franchement sur l'embouchure — plutôt que de
la faire mordre par le flanc — est le vrai correctif, et il ouvrira à nouveau toute
la latitude axiale.

**2. Les deux cuves de carburant d'un même côté s'interpénétraient de 1,30.** Leur
écart était `2·ap`, celui des **plaques hexagonales** — deux cotes sans rapport,
confondues, et la note « léger chevauchement accepté » sous-estimait franchement le
défaut. L'écart des cuves se déduit maintenant de **leur propre rayon** :
`2·res_r + RESERVOIR_JEU` (0,7), soit 7,20 au lieu de 5,20. Les plaques gardent leur
`2·ap` et continuent de se toucher bord à bord.

`les_cuves_de_carburant_ne_se_traversent_pas` verrouille les deux moitiés du
raisonnement : que le nouvel écart dégage les sphères, **et** que l'ancien ne le
faisait pas — sans quoi le découplage n'aurait pas lieu d'être.

### Sources
- [Interstellar Vehicle — Avatar Wiki](https://james-camerons-avatar.fandom.com/wiki/Interstellar_Vehicle)
- [ISV Venture Star — Grokipedia](https://grokipedia.com/page/ISV_Venture_Star)
- [Interstellar voyages with the Venture Star — State of Flux](https://kimbody1535.wordpress.com/2013/04/10/interstellar-voyages-with-the-venture-star-a-look-at-the-best-part-of-avatar/)
- [ISV Venture Star — NamuWiki](https://en.namu.wiki/w/ISV%20%EB%B2%A4%EC%B2%98%20%EC%8A%A4%ED%83%80)

---

## Partie F — Mégastructures et vaisseaux (journal, 2026-08-03 → 2026-08-04)

> Suite de la Partie C, qui s'arrêtait à la clôture de l'ISV (§C.29). Cette
> partie couvre les deux mégastructures (tore de Stanford, complexe Elysium),
> les panneaux solaires d'échelle mégastructure, la séparation
> vaisseaux/stations au menu, et le début du Starship.
> Conception : [`conception/stations.md`](../conception/stations.md) Partie D et
> **Partie D bis**.

### F.1 Deux mégastructures, un moyeu commun (2026-08-03)

Bâties en recyclant l'épine de l'ISV, comme demandé. Le moyeu est un **prisme
hexagonal** — deux `TreillisHexagone` reliés par des montants — et trois bras à
120° dans son plan.

**Trois choix mesurés, pas supposés :**

1. **Le prisme plutôt que l'anneau plat.** Sa hauteur vaut le **côté** de
   l'hexagone, ce qui rend ses six faces latérales carrées : une base de bras
   s'y pose sans rattrapage.
2. **Bras en `Charpente` carrée, pas `CharpenteHexa`.** Ce n'est pas un repli.
   La base carrée mesure `2·sg`, soit exactement le côté de l'hexagone. La
   variante hexagonale reprend le circonradius du carré (×√2), ce qui la rend
   **22,5 % trop large entre plats** (3,67 contre 3,00) : elle déborderait la
   face au lieu de s'y asseoir.
3. **Sur les faces, pas sur les sommets.** Un hexagone a six faces ; trois bras
   à 120° en prennent une sur deux, et une face est un appui plan quand un
   sommet est une arête.

Longueurs, **dérivées et jamais recopiées** — le jour où l'épine du vaisseau
bouge, les bras suivent (leçon de `Epine::hors_tout`, §C.5) :

| | formule | valeur |
|---|---|---|
| `EPINE_LONGUEUR` | — | 84,0 |
| `ELYSIUM_BRAS` | `EPINE_LONGUEUR × 0,5` | 42,0 |
| `STANFORD_BRAS` | `ELYSIUM_BRAS × 0,5` | 21,0 |

**Un défaut latent trouvé au passage** : `TreillisHexagone::liaison` extrudait
ses montants le long de **+Z**, c'est-à-dire *dans* le plan de l'hexagone, au
lieu de +Y qui en est la normale. Le commentaire disait « prisme reliant », le
code faisait un peigne plat. Jamais vu parce qu'aucun appelant n'avait jamais
passé `liaison > 0`. Corrigé, et l'échantillon de la chaîne L1.2 passé à
`liaison: 3.0` pour que le chemin soit désormais **exercé**.

⚠️ Les bras ne sont pas reliés entre eux. La conception prévoit des membrures
courbes joignant les longerons d'un bras à l'autre, et **aucune primitive du
projet ne trace une poutre à axe courbe** (`treillis_conique` courbe sa
*section*, pas son axe). C'est une brique à écrire, donc une étape à part.

### F.2 Le tore de Stanford, première primitive paramétrique (2026-08-03)

Le tore boucle au bout des bras courts. Son rayon majeur est la distance de
l'axe à leur pointe (**24,35**) ; sa section vaut la **hauteur du moyeu**
(3,0), donc un rayon mineur de 1,5 — le prisme et le tube se lisent au même
gabarit.

C'est le premier composant du projet dont la surface est **paramétrique** et
non assemblée de primitives. Sa section se découpe en trois bandes, et le
découpage est **une seule source** que le dessin et les tests lisent :

| bande | étendue | contenu |
|---|---|---|
| tuiles | ±90° autour de l'extérieur | bardage d'écailles hexagonales |
| épaulements | 2 × 50° | coque lisse |
| fenêtre | ±40° autour du moyeu | vitrage |

Un test vérifie que les trois bandes **pavent la section entière** — somme
égale à 2π, sans trou ni recouvrement.

**Le bardage suit la taille de tuile, pas le rayon.** Un anneau deux fois plus
grand porte deux fois plus de tuiles de la même taille, jamais les mêmes tuiles
deux fois plus grandes. Au gabarit actuel : **4 rangs × 123 colonnes = 492
tuiles**.

⚠️ **Un test qui ne tenait que dans un régime.** La première version mesurait
le nombre de **sommets du tore entier** pour vérifier ce rapport. Elle marchait
tant que les tuiles pesaient l'essentiel du maillage, et a cessé de mordre dès
qu'on les a agrandies : les bandes lisses et la menuiserie, à compte fixe,
diluaient le rapport à 1,58 au lieu de 2. Un proxy qui ne tient que dans un
régime n'est pas une mesure — le test interroge désormais la grille elle-même.

**Épaisseur et joint des tuiles sont absolus, pas des fractions du pas.** Ils
l'étaient, et c'était un piège : doubler le pas doublait la saillie, alors
qu'un panneau plus large n'est pas un panneau plus épais. Exprimés en dur, ils
ne bougent plus quand on retaille le bardage — et il n'y a plus rien à
« re-régler en conséquence » quand l'utilisateur demande de doubler la taille
des tuiles, ce qui fut exactement la demande suivante.

Le vitrage porte des **meneaux** (un vitrage courbe de 150 unités ne se
fabrique pas d'un tenant, et leur pas donne son échelle apparente à tout
l'anneau), des **lèvres** aux deux bords (la discontinuité la plus sollicitée
de la section : coque pleine d'un côté, verre de l'autre), et **pas de fenêtre
aux trois jonctions**, où les bras reprennent l'effort. Les jonctions sont
calées sur les mêmes angles que les bras — une seule source, sinon le vitrage
se retrouverait là où arrive la charge.

### F.3 Ce que coûte une mégastructure (mesuré, 2026-08-04)

| | pièces | coût | sommets | lots | triangles | étendue (U) | rayon |
|---|---|---|---|---|---|---|---|
| Tore de Stanford | 29 | 230 | 104 658 | 154 | 114 152 | 52,0 × 18,7 × 52,0 | 25,8 |
| Complexe Elysium | 49 | 379 | 52 248 | 125 | 99 520 | 79,3 × 28,0 × 68,8 | 45,5 |
| *(rappel ISV)* | *45* | *725* | *110 008* | *204* | — | *168,2 de long* | *12,6* |

Le fait à retenir : **Elysium a 69 % de pièces en plus que Stanford et deux
fois moins de sommets**. Le tore, à lui seul, pèse plus que les 28 autres
pièces réunies. Une surface paramétrique et un assemblage de primitives ne se
comparent pas au nombre de pièces — et le budget d'une mégastructure se juge en
sommets, pas en briques.

Cela confirme le mur d'échelle relevé en conception : ~2,85 pièces et
~2 850 sommets par unité de rayon d'anneau. L'Elysium « vrai » du document
(1,8 km d'anneau) pèserait ~1 140 pièces contre 45 pour l'ISV. Les
mégastructures à taille réelle demanderont autre chose que l'assemblage pièce à
pièce.

### F.4 Le moyeu, et deux fois la grille discrète de `Profil` (2026-08-04)

Chaque bras porte un tunnel de modules vers le centre ; le centre porte deux
habitats `Coeur` gris, accouplés **par leur plus grande face**, parallèles à Y.
Le tore tourne autour de Y, au bouton.

**`Profil` est une grille discrète, et elle a bloqué deux fois :**

1. **Habitats en P2, pas P3**, alors que P3 serait le triplement littéral du P1
   d'origine. Le tambour d'un `Coeur` vaut `rayon × 1,16`, soit **3,48** en P3 —
   au-delà du bord extérieur des barres du moyeu (**3,35**). L'habitat avalerait
   l'hexagone au lieu d'être ceinturé par lui. En P2 le tambour vaut 2,32 et se
   loge entre les barres (1,85 → 3,35). C'est le cas de figure de §C.8 : une
   cote qui se règle **contre une autre pièce** ne tombe pas sur la grille — ici
   on a préféré rester sur la grille et s'arrêter au cran qui tient.
2. **Réduction de 33 % de l'habitat de Stanford**, demandée par l'utilisateur.
   Premier essai : ne raccourcir que `longueur`. Retour de l'utilisateur —
   « je ne sais pas ce qui a été modifié mais ce n'était peut-être pas la bonne
   chose ». Il avait raison : le diamètre vient de `Profil`, où rien ne vaut
   0,67 × P2, si bien que réduire la seule longueur **allongeait** la silhouette
   au lieu de la rapetisser. Corrigé en composant une mise à l'échelle
   géométrique (`verser_a_echelle`) dans les transformées cuites — la même
   réponse que `ISV_ECHELLE` sur l'ossature.

**Les deux centres ont été séparés** à la demande de l'utilisateur : Stanford
et Elysium ne sont pas destinés à se ressembler, et factoriser un centre unique
paramétré aurait rendu chaque divergence à venir plus coûteuse que la
duplication qu'elle évite. Ce qui reste partagé (`moyeu_hexagonal`,
`bras_et_tunnels`) l'est parce que c'est **encore** identique, pas parce que ça
doit le rester.

**Un test qui confondait deux choses** : `une_capacite_annoncee_change_vraiment_la_geometrie`
prétendait tester `rotation()` en déplaçant `repli` — or un anneau tourne sans
se replier. Les deux capacités ont été séparées. Et `axe_rotation()`, qui était
indexé sur la **catégorie de menu** (`Megastructures => ISV_AXE`), a été déplacé
sur l'item : une seconde source, fausse dès que deux mégastructures tournent
autour d'axes différents — ce qui venait précisément d'arriver.

### F.5 Panneaux solaires d'échelle mégastructure (2026-08-04)

Quatre familles, avec suivi solaire à deux axes (azimut autour du mât,
inclinaison autour de l'axe de l'aile) : ferme modulaire à cassettes,
concentrateur symétrique, rubans tendus (héliogyre), nappe plissée déployable.

Gabarit d'essai : **16 × 6 U ≈ 36 × 13,5 m**, la taille d'un vrai SAW de l'ISS
— et non celle du `PanneauSolaire` du parc, qui en fait moins de la moitié.

**Le premier jet a été jeté en entier.** Retour de l'utilisateur : « on dirait
un copier-coller des panneaux solaires déjà existants ». Il avait raison, et la
cause était de méthode : les quatre variantes avaient été dessinées en partant
de **formes**. La reprise est partie des **contraintes d'ingénierie** — comment
on tend une nappe de 36 m, comment on la replie, comment on concentre la
lumière, ce qui casse en premier — et les formes en ont découlé.

Le berceau se dessine **hors** du cardan : c'est la partie qui ne suit pas le
soleil, et l'inclure dans la transformée l'aurait fait tourner avec l'aile.

### F.6 Vaisseaux séparés des stations, et le Starship (2026-08-04)

Nouvelle vue au menu, « vaisseaux & supervaisseaux », qui accueille l'ISV et
les deux cibles à venir : l'**Endurance** (Interstellar) et le **Starship**
(SpaceX). Anatomies et ordre de travail en `conception/stations.md` Partie D bis.

**Le Starship tombe pile sur la grille** — 9 m de diamètre = 4,0 U = P2 — ce
qui en fait la bonne première cible. Cotes réelles converties à 1 U ≈ 2,25 m :

| | réel | en U |
|---|---|---|
| Hauteur hors-tout | 50 m | 22,22 |
| Flèche de l'ogive | 11 m | 4,89 |
| Fût | 39 m | 17,33 |
| Diamètre | 9 m | 4,0 (P2) |

Mesuré sur le maillage cuit : **4,1 × 4,1 × 22,2 U**, soit 9,2 × 50 m. La coque
est **une seule pièce** de 28 440 sommets en 59 lots.

**L'ogive est tangente**, et le choix est géométrique et non stylistique :
`ρ = (R²+L²)/2R`, `y(x) = √(ρ²−(L−x)²) + R − ρ`. C'est le seul profil qui
raccorde un cylindre avec une **dérivée nulle**, donc sans arête visible à la
jonction. Le test le mesure sur la **pente** et non sur un rayon : le rayon de
raccord vaut R pour toutes les formes de nez du monde, donc l'y comparer ne
distingue pas une ogive d'un cône. La pente, elle, les sépare — un cône garde
`R/L = 0,4` jusqu'au bout.

Le Raptor a été bâti **en vitrine avant d'être posé**, à la demande de
l'utilisateur — leçon 1 de §C.29. Deux déclinaisons, dont les cotes viennent du
rapport de détente (`r_col = r_e/√ε`, ε = 40 et 90) :

| | sortie | hauteur | col/sortie |
|---|---|---|---|
| RSL (atmosphérique) | 1,30 m | 3,10 m | 0,158 |
| RVac (vide) | 2,40 m | 4,60 m | 0,105 |

Le profil de cloche va en `t^0,55` : ouverture rapide au col puis
aplatissement, ce qui **est** la forme d'une tuyère de détente — un cône, lui,
s'ouvre linéairement. Le test le mesure à mi-hauteur.

**Deux débordements de la même famille**, trouvés par le balayage
d'enveloppes : les cordons de soudure de la coque (×1,004 + section) et les
cannelures du Raptor (×1,012) sont **posés sur** la paroi, donc ils dépassent
du rayon nominal. Une capsule calée sur `profil.rayon()` laissait des sommets
dehors. Réglé par un `rayon_hors_tout()` dans chaque module, source unique du
dessin **et** de l'enveloppe.

⚠️ **Et une capsule ne ferme pas à plat.** Caler le demi-axe à
`demi_hauteur − rayon` laisse la **couronne du culot** dehors, là où le fût
s'arrête net : mesuré, **0,85 hors d'une enveloppe de 2,03**. Le demi-axe doit
valoir `hypot(demi_hauteur, rayon)` — ce que `rayon_local` calcule déjà, et
c'est l'idiome de `ModuleAxial`.

### F.7 La colonne d'items, déduite de l'enum (2026-08-04)

La vue composants ne changeait d'item qu'en **cyclant** : pour atteindre le
27ᵉ des 28, vingt-sept pressions, et aucun moyen de savoir ce qui existe sans
faire le tour. Une colonne à gauche, **bornée à un dixième de la largeur**,
rend le catalogue visible et chaque entrée atteignable en un clic.

La demande de l'utilisateur était explicite : « ne pas avoir à ajouter une
nouvelle entrée à la liste des boutons à chaque fois ». Les boutons se
**déduisent** donc de `Composant::nom()`, un `match` exhaustif : ajouter une
variante à l'enum casse la compilation tant qu'elle n'a pas son nom, et son
bouton arrive avec elle. La couverture de la colonne est une propriété du
**compilateur**, pas une discipline à se rappeler.

Les libellés sont abrégés à ce qui précède le deux-points — « PANNEAUX :
5 VARIANTES » devient « PANNEAUX ». Pas de moteur d'expressions régulières
pour ça : `split_once(':')` **est** la règle, en une ligne et sans dépendance.

**Abréger rapproche**, et c'est le risque propre à l'étape : deux entrées
distinctes peuvent se réduire au même bouton, ce qui ne se verrait qu'à
l'écran, sur deux lignes jumelles. Un test l'interdit sur toutes les tables. Il
a immédiatement servi : le libellé curaté « RADIATEUR MEGA » entrait en
collision avec le `RadiateurMega` engendré — réglé par un marqueur `· ` sur les
entrées engendrées, ce qui a demandé de passer `Item.libelle` en `Cow`.

**Deux tests faibles trouvés par red-check** : celui de la largeur se comparait
à `l × PART_LARGEUR`, c'est-à-dire à **sa propre constante** — réciter la
formule, et suivre docilement n'importe quelle valeur qu'on lui donne ; il
compare désormais à `l / 10.0` littéral. Celui de la couverture sondait
`engendrees()` directement au lieu de la liste effective.

⚠️ **Un défaut que les tests ne pouvaient pas attraper** : la colonne était
dessinée pendant la phase d'entrée, et le `clear_background(BLACK)` de la passe
3D l'effaçait aussitôt. Elle était là chaque frame, quelques microsecondes.
Le dessin est passé dans la passe interface ; seul le calcul d'exclusion caméra
reste tôt, n'ayant rien à effacer. C'est exactement la classe de défaut que
§6.6 (pas de test de rendu) laisse remonter à l'écran et non en CI.

### F.8 Incident : du travail non commité détruit (2026-08-04)

Un `git checkout -- src/vaisseau/composant/mod.rs`, lancé pour annuler quatre
lignes de mesure temporaire, a ramené tout le fichier au dernier commit et
emporté avec lui le câblage de `CoqueOgive` et `Raptor`, `Composant::nom()`, et
les entrées correspondantes de la chaîne d'échantillons. Aucun filet : la
consigne debout est de ne pas commiter, et rien ne l'avait été depuis des
heures.

Reconstruit à partir des cinq erreurs de compilation, qui pointaient chaque
manque. Les modules `coque.rs` et `raptor.rs` eux-mêmes n'étaient pas touchés.
Sept tests de coque et de moteur ont été réécrits et **tous red-checkés**.

Deux choses n'ont pas pu être restituées à l'identique, et le choix est assumé
plutôt que deviné :

- **`Composant::nom()` rend le nom de famille** (« COQUE OGIVE », « RAPTOR »),
  pas famille + variante : dans une colonne de 100 px la variante est du bruit,
  et la vitrine la fait déjà défiler.
- **La formule d'enveloppe** a été retrouvée par la mesure, pas par mémoire —
  voir le débordement de 0,85 en §F.6.

**Conséquence de méthode** : l'utilisateur commite désormais aux points signalés.
Ne jamais employer `git checkout --` / `git restore` / `git reset --hard` sur un
fichier contenant du travail non commité ; pour annuler ses propres
modifications, ré-éditer les lignes, ou sauvegarder le fichier dans le
scratchpad **avant** de le toucher.

### Sources
- [Stanford torus — Wikipedia](https://en.wikipedia.org/wiki/Stanford_torus)

### F.7 Starship — la grappe de six Raptor (2026-08-05)

Étape 2/5 de §D.5. Les six moteurs sont posés, en deux couronnes de trois.

**Les rayons de couronne viennent du réel**, comme le reste de l'engin :

| | réel (rayon) | ici |
|---|---:|---:|
| 3 RSL (atmosphériques) | 1,10 m | 0,489 U |
| 3 RVac (vide) | 3,10 m | 1,378 U |

**Une couronne de trois EST une symétrie radiale.** `Symetrie::Radiale(3)` pose
le premier moteur et fabrique les deux autres ; recopier trois angles à la main
laisserait une couronne devenir irrégulière sans que rien ne le dise. Le module
`symetrie.rs`, écrit d'avance pour le Lot 4, **entre donc en service ici** — et
son `#![allow(dead_code)]` de module a été **resserré sur les deux seuls
éléments encore en attente** (`Miroir`, `nb_copies`), pour que le reste
redevienne sensible au code mort.

**Le placement est sorti du preset** (`starship_moteurs`) pour être *mesurable* :
un placement se vérifie sans contexte graphique. Sept tests, tous red-checkés :
rien ne dépasse de la jupe, rien ne s'interpénètre, les têtes affleurent le cul,
chaque couronne est régulière, les deux sont déphasées, et le preset les pose
vraiment.

⚠️ **Un sabotage m'a corrigé.** J'avais annoncé « 0,057 U de jeu entre les
couronnes », et élargir la couronne RSL aurait dû faire rougir la collision.
C'est resté **vert** — et le test avait raison. Ces 0,057 U sont l'écart
**radial entre deux anneaux**, pas la distance entre deux moteurs : décalées de
60°, les couronnes laissent 1,19 U entre un RSL et son RVac voisin pour 0,83
requis, et l'équation montre qu'un RSL ne peut *jamais* rejoindre un RVac en
s'écartant sur son propre rayon (discriminant négatif). Le vrai risque de
contact est **à l'intérieur** d'une couronne. Le commentaire disait une chose
fausse au sujet d'une géométrie juste ; il a été corrigé.

**Décalage de 60°** entre les deux couronnes : alignés, les RVac se logeraient
pile derrière les RSL et le cul ne montrerait que trois directions au lieu de
six. C'est le seul des sept tests qui porte sur l'aspect et non sur la mécanique.


### F.8 Starship — jupe moteur et quatre volets (2026-08-06)

Étapes 3 et 4 de §D.5, plus une correction de fond sur la hauteur.

#### ⚠️ Les 50 m n'étaient pas les 50 m

En posant les moteurs (F.7), je les avais accrochés **sous** une coque déjà
haute de 50 m : le vaisseau mesurait 54 m. Une fusée se mesure jusqu'au **plan
de sortie de ses tuyères**, pas jusqu'au bas de son réservoir.

Corrigé en faisant de ce plan l'origine de toutes les cotes verticales
(`starship_cul`). La jupe et la coque se **partagent** les 50 m au lieu de
s'additionner : `STARSHIP_FUT = HAUTEUR − NEZ − JUPE_HAUT`. Le test
`le_starship_mesure_toujours_cinquante_metres` tient l'addition, et rougit si on
remet la jupe sous la coque.

#### La jupe moteur

Nouvelle brique `JupeMoteur` : une virole **ouverte aux deux bouts**, à double
paroi (sans quoi on verrait à travers par l'arrière, le rendu ne faisant pas de
faces doubles). Ouverte, et c'est le point — un cylindre plein cacherait les six
cloches, autant ne pas les avoir modelées.

Sa cote basse est **déduite, pas choisie** : elle doit couvrir au moins
`JUPE_COUVERTURE` (2/3) de *chaque* moteur, et c'est le plus **court** qui
contraint — à bord de jupe donné, c'est lui qui ressort le premier. D'où
RSL couvert à exactement 2/3, RVac à 77 %.

⚠️ Les moteurs sont bornés par la **paroi intérieure** de la jupe, pas par son
rayon nominal : la paroi rentre de 2 %, et un RVac passe à quelques centimètres.
Le test lit `jupe_rayon_interieur` à la source.

#### Les quatre volets

Nouvelle brique `Volet`, la seule pièce du vaisseau qui **bouge**.

**Ce n'est pas une aile** : un volet de Starship *traîne*, il ne porte pas.
L'engin tombe à plat, ventre au vent, et les volets modulent leur traînée pour
tenir l'assiette. D'où l'épaisseur de caisson là où une aile serait fine — un
profil d'aile lirait comme un avion.

Deux courts en haut (4,5 × 5 m à 41 m du cul), deux longs en bas (7,5 × 8 m à
8 m). **L'écartement fait la silhouette** : les rapprocher donnerait une croix.

⚠️ **Les volets étaient posés à plat, dans le mauvais sens** — vu à l'écran,
pas par un test. Le volet est dessiné **corde sur `X`, épaisseur sur `Z`** ;
avec la seule rotation d'azimut, la corde s'enroulait donc *autour* de la coque
et l'épaisseur se couchait le long du vaisseau. Corrigé par une pré-rotation de
**−90° autour de Y** (l'axe d'envergure, donc le volet continue de pointer hors
de la coque) : `R_z(azimut−90°) · R_y(−90°)`.

⚠️ **Aucun test ne pouvait le voir**, et c'est la leçon. Celui de la hauteur
mesurait `pos.z ± corde/2` **sans regarder la rotation** : il supposait
l'orientation au lieu de la lire, donc il passait au vert quelle qu'elle soit.
Il mesure désormais l'étendue réelle, coins tournés compris, et un test neuf
(`la_corde_des_volets_suit_laxe_du_vaisseau`) tient les trois axes.

⚠️ **Une paire de volets est un MIROIR, pas une rotation.** Deuxième défaut vu
à l'écran : le volet du côté `−Y` sortait retourné, montrant son inox là où
l'autre montre ses tuiles. La rotation d'azimut seule retourne le panneau ; il
faut un **demi-tour supplémentaire sur le second de la paire**, autour de son
axe d'envergure. `les_faces_tuilees_dune_paire_regardent_du_meme_cote` tient
désormais l'invariant — et rougit aussi bien sans demi-tour qu'avec un demi-tour
sur les deux.

⚠️ **J'avais épinglé le mauvais invariant.** Le test précédent exigeait
`corde.z > 0.999` — signé — ce qui était arbitraire : en miroir, les deux cordes
pointent vers des bouts opposés du vaisseau, et c'est normal. Ce qui doit
s'accorder, c'est la **face tuilée**. L'axialité est redevenue non signée, et la
vraie garde est passée là où elle a un sens.

ℹ️ **Le côté *global* du ventre n'est pas encore épinglé** : retourner les
quatre volets ensemble passe au vert, faute d'une référence de ventre. C'est le
bouclier thermique (étape 5/5) qui la fournira.

⚠️ **Le signe compte, et `abs()` le perdait.** Première version du test : `+90°`
passait au vert comme `−90°`, puisque la corde suit l'axe dans les deux cas —
mais la face **tuilée** bascule alors de l'autre côté tangentiel, ce qui se voit.
Assertion signée, et les trois sabotages rougissent : ancienne rotation, `+90`
au lieu de `−90`, et ordre de composition inversé.

⚠️ **L'enveloppe du volet était fausse** et un test existant l'a dit tout de
suite (`les_rayons_declares_contiennent_la_piece` : « déborde de 1,43 hors d'une
enveloppe de rayon 1,00 »). J'avais déclaré une capsule **centrée**, alors qu'un
volet part de sa charnière et s'écarte — et qu'il pivote hors du plan avec le
braquage. Elle lit désormais son bout à la source (`volet_bout`), la même
fonction que le dessin, si bien qu'un changement de braquage ne peut plus la
laisser en arrière.

Cinq sabotages red-checkés, dont deux qui comptent : **volets tournés à
l'envers** (ils rentreraient dans le réservoir, invisible sur un rendu de trois
quarts) et **paire non opposée**.

#### Deux verrous d'inventaire ont fait leur travail

Ajouter deux variantes a fait rougir `la_chaine_dechantillons_visite_chaque_variante_une_fois`
(35 → 37) et `le_catalogue_des_fils_est_a_jour`. C'est **voulu** : le compilateur
garantit qu'une variante a un bras, pas qu'elle est atteinte par la chaîne. Les
deux ont été mis à jour, pas contournés.



### F.9 Starship — bardage de tuiles, et la cote des volets avant (2026-08-06)

Étape 5/5 de §D.5.

#### Les volets avant mordaient sur l'ogive

Leur cote venait du réel — 41 m — mais le raccord fût/ogive est à
`HAUTEUR − NEZ` = 17,3 U (39 m). Le volet avant, corde comprise, montait donc à
19,3 U : **deux unités dans le nez**, où la coque se rétrécit déjà et où sa
charnière, posée au rayon du fût, flottait au-dessus du vide. Visible de profil.

La cote est désormais **déduite** — `HAUTEUR − NEZ − corde/2 − jeu` — donc 35,6 m,
et elle suivra toute retouche de l'ogive au lieu de la contredire.

#### Le bardage de tuiles

Nouvelle brique `BardageTuiles` : un manchon **partiel**, sur un arc d'un peu
plus de 180°, posé sur le fût. Un Starship ne rentre pas nez en avant — il tombe
**à plat, ventre au vent** — et c'est ce contraste noir/inox sur un seul côté
qui fait reconnaître l'engin, bien plus que sa silhouette.

Il couvre **exactement le fût** : ni la jupe, ni l'ogive (un manchon cylindrique
flotterait sur un cône). Les tuiles décollent de 1,2 % du rayon, sinon elles se
battraient avec la coque dans le tampon de profondeur.

#### ⚠️ Le ventre était écrit DEUX fois — il n'y avait pas de bouton

Vu à l'écran : les quatre volets et le bardage tuilaient bien du même côté…
mais du **mauvais**, tous ensemble. Et il n'y avait aucun moyen simple de les
retourner, parce que le côté était codé à deux endroits — en dur (`+X`) dans
`bardage_dessiner`, et dans le **signe** du quart de tour des volets.

Les tests ne voyaient que leur **accord**, jamais leur orientation d'ensemble :
un engin qui présente son dos à la caméra passait au vert.

Corrigé par `STARSHIP_VENTRE`, **une seule constante d'azimut**. Le bardage s'y
oriente, et le signe du quart de tour des volets s'en déduit
(`-FRAC_PI_2 * VENTRE.cos()`). Le test compare désormais les deux à **la
constante**, pas l'un à l'autre — et un second test interdit de poser le ventre
sur une charnière (le bardage passerait sous les volets et la nappe serait
coupée en deux).

C'est la troisième fois de la session qu'un test vérifie une cohérence interne
au lieu de la chose elle-même : masses recopiées, corde supposée axiale, ventre
auto-référent. Le motif est toujours le même — **comparer deux copies plutôt
qu'une copie à sa source**.

#### Les volets sont **droits** au repos, et le commentaire disait l'inverse

Ils étaient posés à `braquage = 0,55`, soit 41° — quatre gouvernes penchées vers
le ventre en permanence, ce qui donne une silhouette affaissée. Position de
repos remise à **zéro** : c'est celle de l'engin au sol et en montée, les
gouvernes ne se braquent qu'à la rentrée.

⚠️ **La documentation de `braquage` était à l'envers.** Elle annonçait
« 0 = replié contre la coque, 1 = déployé » alors que le code fait le contraire :
0 laisse le volet **droit**, 1 le braque à fond. Le commentaire a menti pendant
toute la pose des volets sans qu'aucun test ne s'en aperçoive — parce qu'aucun
ne mesurait ce que le paramètre *fait*.

Deux tests neufs le tiennent maintenant : le sens (0 → droit, 1 → penché vers le
ventre) et, surtout, que **c'est une rotation et non un glissement** — le bout
reste à l'envergure de sa charnière quel que soit le braquage. Ce dernier
invariant rougit sur un sabotage que le premier ne voit pas.

#### La nappe monte jusqu'à la pointe

L'ogive est tuilée elle aussi : le bardage suit le fût **puis** la courbe
d'ogive, en relisant `ogive()` — la fonction de la coque — au lieu de
réinventer le profil. S'arrêter au raccord laissait un nez d'inox nu, c'est-à-dire
exactement la partie qui prend le plasma en premier.

#### La lacune du 4/5 se referme

À l'étape précédente j'avais noté que retourner **les quatre volets ensemble**
passait au vert, faute d'une référence de ventre. Le bardage la fournit : il
regarde `+X` par construction, et `le_bardage_et_les_volets_tuilent_le_meme_cote`
rougit désormais aussi bien si on retourne les volets que si on tourne le
bardage. Une brique qui ferme un trou de test laissé ouvert par la précédente.

`+X` est **fixe et non paramétrable**, délibérément : le ventre est une
direction unique, et un azimut réglable inviterait à poser deux boucliers qui ne
regardent pas au même endroit.



### F.10 Starship — debout, et les hublots (2026-08-06)

Le Starship est **terminé** (5/5 de §D.5).

#### Le vaisseau se montre debout

Toutes les pièces du parc se bâtissent le long de `+Z`, et le Starship ne fait
pas exception — c'est dans ce repère que ses cotes se lisent. Mais une fusée se
**regarde debout** : `starship_debout()` fait un quart de tour autour de `X` pour
amener l'axe sur `+Y`, moteurs en bas.

⚠️ **Appliquée à l'assemblage seulement.** Tourner la géométrie à la source
obligerait chaque cote et chaque test à raisonner en `Y` pour un gain nul : le
vaisseau n'en serait pas plus juste, seulement plus difficile à lire.

#### Les hublots — sur le nez, à l'opposé du bouclier

Deux petits vitrages encadrés, côte à côte, **sur l'ogive** et **côté dos**.

Premier jet posé côté ventre et sur le fût : les deux étaient faux, et pour des
raisons physiques, pas esthétiques. Sur le cylindre, ce seraient des hublots de
**réservoir** — l'équipage regarde par le nez. Et côté ventre, ce serait une
baie vitrée percée dans le bouclier thermique, à l'endroit exact qui prend le
plasma.

⚠️ **Sur une ogive, un vitrage plat ne peut pas être radial.** Il se pose au
rayon de la **courbe** (1,865 U et non 2,000), et sa normale s'**incline vers la
pointe** en suivant la pente locale. Les deux sont relus sur `ogive_rayon`, la
fonction que la coque dessine — pas approchés.

C'est la seule teinte froide d'un engin qui n'est qu'inox et tuiles, et **la
seule chose qui donne une échelle humaine** à une coque de 50 m.

⚠️ **Une tolérance trop large a rendu un test aveugle.** Je bornais l'écart de
rayon par `HUBLOT_ECART` (0,62) pour absorber le décalage tangentiel des deux
hublots — mais l'erreur à détecter ne vaut que 0,14, si bien que poser le
vitrage au rayon du fût passait au **vert**. Corrigé en projetant sur l'azimut :
le décalage tangentiel lui est perpendiculaire, donc la composante radiale est
exacte et la tolérance retombe à 1e-3.

La **hauteur** sur le nez reste un choix et n'est pas épinglée — mais sa limite
physique l'est : vers 90 % de la flèche, la peau ne fait plus que 0,44 U pour
une demi-paire de 0,53, et les hublots s'enrouleraient autour de la pointe.

#### ⚠️ Encore un test qui comparait des origines à des étendues

Premier jet du test « debout » : *la pièce la plus basse est un moteur, la plus
haute est la coque*. La seconde moitié est fausse — le repère de la coque est en
son **milieu**, si bien qu'un hublot se pose plus haut que lui sans rien
dépasser. Le test a rougi tout de suite, ce qui est la bonne nouvelle.

Il n'affirme plus que ce qu'une origine peut dire : **ce qui est en bas** (un
Raptor), et que l'engin est plus de trois fois plus haut que large. Quatre
sabotages rougissent : pas de rotation, rotation inverse (nez en bas), hublots
regardant l'intérieur du réservoir, hublots du mauvais côté.


### Sources

- [Starship — SpaceX](https://www.spacex.com/vehicles/starship/)
- [Raptor (rocket engine) — Wikipedia](https://en.wikipedia.org/wiki/Raptor_(rocket_engine))

---

## Deux briques neuves : miroir primaire et module en coin d'arc (2026-08-09)

Demandees par l'utilisateur apres la cloture de l'assembleur : *« on va
travailler sur des composants pour l'Endurance et le tore de Stanford »*, puis
*« d'abord un disque a 110 % de la taille du tore »*.

### Miroir primaire (`composant/miroir.rs`)

**110 % du rayon de l'anneau** (`rayon_pour_anneau`, `DEBORD_ANNEAU = 1.10`) : au
gabarit exact, la jante resterait dans son ombre au moindre desalignement.

C'est la piece iconique du tore de Stanford, et elle manquait : le preset n'avait
que le moyeu, les bras, les habitats et l'anneau. **Sans miroir, rien n'explique
d'ou vient la lumiere dans un tore vitre** — or c'est tout le principe de
l'objet.

- Membrane en bande de quadrilateres (`Peintre::triangles`), pas en eventail :
  un eventail depuis le centre donne des triangles etires au bord, et gere mal
  le cas perce. La bande entre `trou` et `rayon` reste reguliere et couvre les
  trois variantes sans branche separee.
- **Legerement creuse**, en parabole : plate au bord, creusee au centre. Un creux
  uniforme donnerait un cone, pas une membrane tendue.
- Trois variantes : `Primaire` (plein), `Anneau` (perce, laisse passer l'axe du
  moyeu), `Petales` (segmente).
- **Le cout suit la surface**, pas le rayon : doubler le rayon quadruple la
  membrane a tendre. Un cout lineaire ferait passer un miroir geant pour deux
  fois un petit.
- Pose sur `preset_stanford` en variante `Anneau`, a 0,75 rayon au-dessus du plan
  de l'anneau, face vers le bas.

⚠️ **Il expose un port, contrairement au tore.** `Composant::Tore` rend `vec![]` :
il est range en « pose a la main » et **l'assembleur ne peut donc pas le
proposer**. Le miroir, lui, se monte — c'est ce qui le rend constructible dans le
VAB, et une piece de cette portee ne sert a rien si on ne peut pas l'accrocher.

### Module en coin d'arc (`composant/module_arc.rs`)

L'etape 4 de l'ordre de travail (section D.5) : la brique de l'Endurance, jugee
seule avant tout assemblage.

Une section de caisson **a faces planes** balayee le long d'un arc, en facettes
plates. ⚠️ **Ce n'est deliberement pas un `Tore`** : la section de l'Endurance
est facettee, pas de revolution — reutiliser le tore lui oterait sa silhouette.

Deux ports de bout, tangents a l'anneau : les chainer fait tourner chaque suivant
de `arc`, et douze coins de 30° ferment le cercle. Quatre familles
(`Habitat`, `Soute`, `PosteRanger`, `PosteLander`), comme le demande la section D.3.

⚠️ **Le graphe de ports est un arbre : l'anneau ne se referme pas en cycle.** Le
douzieme module *touche* le premier geometriquement, mais rien ne les soude. Sans
consequence pour le rendu ni pour la sauvegarde ; ca en aurait une le jour ou un
retrait au milieu devrait casser l'anneau en deux.

### ⚠️ Le red-check a demoli un test, et l'enseignement vaut d'etre garde

`douze_modules_chaines_referment_l_anneau` semblait la garde evidente de la
brique. **Deplacer le port de fin a `0,9 x arc` l'a laisse vert.**

La raison est de la geometrie pure : la position du corps s'obtient en sommant
douze fois le meme vecteur, tourne a chaque pas d'un douzieme de tour — et **la
somme d'un vecteur *quelconque* sur les douze multiples de 30° est nulle par
symetrie**. N'importe quelle position de port ferme donc l'anneau, du moment que
la *rotation* vaut bien 2π/12. Le test ne prouvait que la rotation.

D'ou `les_ports_sont_tangents_a_l_anneau`, qui lie enfin position et
orientation : un port d'anneau regarde le long de la tangente, donc sa direction
avant est perpendiculaire au rayon qui le porte. Le sabotage rougit aussitot.

Et une inversion **globale** du signe reste invisible aux deux — la chaine tourne
a l'envers mais reste coherente avec elle-meme. C'est
`les_rayons_declares_contiennent_la_piece`, anterieur et qui confronte la
geometrie *dessinee* au rayon declare, qui attrape ce cas la.

C'est la quatrieme fois de ce chantier qu'un test verifie un invariant sans
prouver l'effet.

### Limite connue : la palette ne liste pas les sous-variantes

`echantillons()` ne visite qu'un echantillon **par variante de `Composant`**, pas
par sous-variante — meme limite que `VarianteCaisson` et toutes les autres
familles. On ne peut donc pas choisir « ARC SOUTE » a l'ecran, seulement la
famille par defaut. C'est une vraie lacune pour l'Endurance, dont l'interet est
justement l'alternance des quatre familles autour de l'anneau.

### Mesures

579 → **592 tests**, 0 avertissement clippy. La chaine d'echantillons passe de 39
a 41 variantes, et `docs/reference/fils.md` a ete regenere.

**6 sabotages** : signe du repere inverse, port de fin deplace, une seule
facette, miroir sous-dimensionne, cout lineaire, creux uniforme.

### La colonne de composants defile (2026-08-09)

Signale a l'usage : *« il y a tellement de choses que je n'arrive plus a lire le
contenu de chaque item »*.

**Le diagnostic.** `liste::hauteur_ligne` divise la place disponible par le
nombre d'items, borne a `LIGNE_MIN = 16`. Tant que la liste tenait, c'etait
juste. Passe une quarantaine d'entrees — seuil que le catalogue vient de
franchir, notamment avec les deux briques du jour — **deux choses arrivaient
ensemble** : les lignes tombaient au minimum, donc le texte a la taille minimale
(illisible), **et** le surplus etait coupe net par un `break`, sans que rien ne
signale qu'il existait.

**Le correctif** vit dans `liste.rs`, ou il se teste (section 10.9 : ce qui se
decide sort du dessin) : hauteur de ligne **fixe** (`LIGNE_DEFIL = 24`),
decalage borne, ascenseur qui n'apparait que s'il sert, et un `item_sous_curseur`
qui tient compte du decalage. La molette defile quand le curseur est sur la
colonne — `sur_la_liste` retirait deja a la camera le glisser *et* le zoom, donc
la roue etait libre : aucune nouvelle regle de priorite a inventer.

La propriete qui porte tout : **on clique sur ce qu'on voit**. `ligne_defilee` et
`item_sous_curseur_defile` sont deux calculs separes qui doivent parler du meme
ecran ; le test les confronte a cinq decalages differents.

⚠️ **Un sabotage a d'abord glisse entre les mailles.** Retirer le plancher de
taille du curseur d'ascenseur ne rougissait rien : le plancher ne mord qu'au-dela
d'un millier d'items (a 24 px de ligne sur ~700 px de colonne), et le test
s'arretait a mille. Il monte maintenant a cinq mille, avec la note que c'est une
garde **defensive**, hors d'echelle des catalogues reels. Sans ce `n`, le test
faisait semblant.

**6 sabotages** : signe du decalage dans le test de collision, course trop
courte, ascenseur permanent, plancher de curseur retire, bornage supprime,
hauteur de ligne re-ecrasee. **592 → 598 tests.**

⚠️ Seule la vue **composants** defile pour l'instant. `galerie_disques` et
`skymap` utilisent la meme colonne et souffriraient du meme defaut si leurs
listes s'allongeaient — l'API est la, il ne reste qu'a la brancher.

### Le miroir n'avait qu'une face (2026-08-09)

Signale a l'oeil : *« l'interieur et l'exterieur du miroir devraient avoir une
texture de miroir, ou une teinte bleu sombre »*.

**Le diagnostic.** La membrane etait un maillage a **un seul enroulement**, et
`vaisseau::panneau` documente deja la raison : *« macroquad ne double-face pas
les parallelogrammes, donc on le redessine avec les aretes inversees »*. J'avais
la primitive `triangles` sous les yeux sans faire le rapprochement — toute une
face du miroir etait donc **invisible**, et l'on voyait au travers en passant
derriere.

**Deux correctifs :**

1. **Chaque bande est emise deux fois**, la seconde a enroulement inverse. La
   face avant porte le reflet, la face arriere un **bleu sombre** (0.10, 0.15,
   0.26) : c'est le dos d'un miroir, pas un second reflecteur, mais il doit se
   voir.
2. **Un degrade radial** remplace la couleur plate. Il n'y a ni texture ni
   normale dans ce parc de composants : une couleur unie donne un jeton uni, et
   un miroir uni ne ressemble a rien. Cinq bandes concentriques, du bleu profond
   au centre au blanc-bleu au bord — ce que renvoie un reflecteur : le ciel au
   milieu, la lumiere rasante sur la jante.

**Le test compte ce qui sort du peintre**, plutot que de relire le code : un
peintre d'essai enregistre les triangles, et l'on verifie que chacun a bien son
jumeau de normale opposee. C'est ce qui rougit si quelqu'un retire un jour la
seconde emission.

**2 sabotages** : face arriere supprimee, degrade aplati. **598 → 600 tests.**

### Le tore de Stanford se desolidarise (2026-08-09)

Trois demandes d'un coup, a l'oeil : *« le miroir ne devrait pas planer parallele
au tore, il devrait etre incline vers le soleil ; seul le tore devrait tourner ;
je veux un pilier d'habitat qui s'etire en haut et en bas de la partie centrale,
avec un module a chaque pole »*.

**1. Le miroir est incline a 45°.** A plat, il ne renvoyait rien : la lumiere
arrive **perpendiculairement a l'axe de rotation**, et un disque parallele a
l'anneau la reflechit d'ou elle vient. 45° est le seul angle qui la renvoie *le
long* de l'axe, donc vers l'anneau — c'est la geometrie meme du tore de Stanford,
et la raison pour laquelle toutes ses illustrations montrent un miroir de biais.
Le test encadre l'angle des deux cotes : ni parallele a l'anneau (le defaut
d'origine), ni couche dans son plan (il ne le verrait plus).

**2. Le moyeu ne tourne plus.** Un tore de Stanford ne tourne pas d'un bloc :
l'anneau tourne — c'est lui qui fait la gravite —, mais le moyeu est
**desolidarise** (« despun ») pour qu'on puisse s'y amarrer, et le miroir doit
garder le soleil dans le meme axe. Solidaire de l'anneau, il ne renverrait la
lumiere qu'une fraction du temps.

`preset_stanford` se scinde donc en `preset_stanford_moyeu` (moyeu, pilier,
poles, miroir) et `preset_stanford_anneau` (bras, habitats, tore), branches par
une nouvelle `Fabrique::Despun` — le mecanisme de moitie tournante existait deja
pour l'ISV, il ne restait qu'a l'employer. La vue d'un seul tenant reste, en
`#[cfg(test)]`, et se compose des deux moities plutot que de les rebatir : c'est
ce qui garantit que la vue animee et la vue statique montrent le meme objet, et
un test verifie que les deux moities sont **disjointes et completes**.

**3. Le pilier polaire.** Une colonne le long de l'axe qui traverse le moyeu et
sort des deux cotes, coiffee d'un noeud d'amarrage et d'une antenne a chaque
pole. C'est la zone de gravite nulle du tore, et elle ne tourne pas. Symetrique
par construction — une boucle sur `[1, -1]` pose les deux moities avec le meme
code, plutot qu'un haut et un bas ecrits separement qui finiraient par diverger.

### ⚠️ Un test qui encodait une coincidence

`seul_litem_a_deux_moities_porte_une_epine` exigeait l'**equivalence** entre
« a une moitie tournante » et « porte une epine ». C'etait vrai — parce que l'ISV
etait alors le **seul** item dissocie, donc les deux formules designaient le meme
item. Le tore de Stanford, desolidarise a son tour, n'a evidemment pas d'epine :
le test a rougi pour une raison qui n'etait pas la sienne.

Le sens qui porte quelque chose est unidirectionnel : l'epine regle l'alesage du
collier de la section tournante, donc un item qui en porte une **doit** avoir
cette section. La reciproque ne dit rien. Le test verifie desormais
l'implication, **et** qu'il existe au moins un item dissocie sans epine — sans
quoi l'equivalence redeviendrait vraie par hasard et personne ne le saurait.

`Fabrique::Rotatif` est supprimee : le tore en etait le seul usage, et une
variante d'enum sans constructeur est du code mort.

**3 sabotages** : miroir remis a plat, pilier d'un seul cote, miroir bascule du
cote tournant. **600 → 603 tests.**

### Trois reglages a l'oeil sur le tore (2026-08-09)

Le genre de choses qu'aucun test ne dit, et que seul l'ecran tranche.

- **Noeud de pole : quatre sorties, pas six.** Le noeud n'est pas tourne, donc
  ses ports axiaux (±Z) et radiaux (±X) tombent tous dans le plan horizontal et
  forment un collier d'amarrage propre. Les deux sorties de plus d'un noeud a six
  pointent le long de ±Y — l'une **dans** le pilier, l'autre en plein dans
  l'antenne de pole. C'est ce qui n'allait pas.
- **Miroir a 30°, et non 45°.** La geometrie pure demande 45° : c'est l'angle qui
  renvoie *exactement* le long de l'axe une lumiere qui arrive
  perpendiculairement. Mais a 45° le disque se presente trop de profil et sa
  surface disparait sous les angles de camera usuels. 30° est un **arbitrage de
  lecture, pas d'optique** — assez incline pour se lire de biais, assez ouvert
  pour montrer son reflet.
- **Miroir 50 % plus large** (110 % → 165 % du rayon de l'anneau). Consequence
  directe du point precedent : une fois incline, un disque ne presente a l'anneau
  que sa **projection**. La cote qui compte est ce qu'il couvre, pas son rayon
  nu.

### ⚠️ Un verrou de valeur retire au passage

`le_miroir_deborde_l_anneau_qu_il_eclaire` exigeait **exactement** 110 %. Passer
a 165 % l'a fait rougir — pour rien : le debord est un arbitrage visuel, et
§6.6 ecarte explicitement les verrous de valeur, qui *« avaient une raison datee,
pas une raison permanente »*. Un tel test rougit a chaque retouche sans rien
apprendre a personne, et pousse a le mettre a jour machinalement — c'est-a-dire a
ne plus le lire.

Il **encadre** desormais : le miroir deborde (sinon la jante reste dans l'ombre),
sans etre demesure (au-dela du triple, ce n'est plus un reflecteur mais un
couvercle), et le facteur est le meme a toute echelle — c'est une proportion, pas
une taille. Meme traitement pour l'inclinaison, qui se compare a la constante
plutot qu'a un nombre reecrit dans le test.

### Le pilier polaire, deuxieme passe (2026-08-09)

- **Miroir trois fois plus loin** (0,75 → 2,25 rayon d'anneau). De pres il
  coiffait le tore comme un couvercle et l'ecrasait ; recule, il redevient une
  piece distincte, tenue a distance. ⚠️ **C'est desormais lui qui fixe le rayon
  englobant**, donc le recul de la camera : le tore occupe mecaniquement moins
  de place a l'ecran au cadrage automatique.
- **Coeur d'habitat aux poles, plus de noeud.** Ni six sorties ni quatre : les
  deux ont ete essayes et ecartes a l'oeil. Un noeud est une **croix**, et une
  croix au sommet d'une colonne se lit comme un embranchement — or il n'y a rien
  a embrancher la-haut. Le tube du `CoeurGris` prolonge le pilier au lieu de le
  contredire, et c'est **la piece meme qu'emploie le complexe Elysium** pour ses
  habitats : un seul vocabulaire d'un bout a l'autre de la famille.

### ⚠️ Le tore avait deja des habitats axiaux, et ils tournaient

En posant le pilier je n'avais pas vu que `habitats_accouples` place **deja**
une paire de coeurs sur l'axe, au centre — mon pilier s'installait donc dans une
region deja occupee. Trois tests l'ont dit d'un coup, en comptant quatre
`CoeurGris` la ou ils en attendaient deux.

Et ces habitats-la etaient du **cote tournant**, par accident de decoupage : ils
voisinaient les bras dans le code d'origine, pas dans l'espace. Le nom du test
qui les garde le disait pourtant depuis toujours —
`les_habitats_du_moyeu_sont_paralleles_a_laxe`. Ils sont passes du cote fixe, ce
que demandait aussi la consigne « seul le tore doit tourner ». Sans consequence
visible (ce sont des corps de revolution sur leur propre axe), mais le decoupage
dit maintenant ce qu'il fait.

**Trois tests distinguaient les habitats par leur type** ; ils en distinguent
desormais **quatre par leur distance a l'origine** (`coeurs_tries`) : deux
accouples au moyeu, deux aux poles. Meme composant, meme variante — seule la
position les separe, et s'appuyer sur l'ordre de pose aurait ete s'appuyer sur un
detail d'ecriture.

### ⚠️ Un verrou de valeur retire

`le_miroir_deborde_l_anneau_qu_il_eclaire` exigeait **exactement** 110 %. Le
passage a 165 % l'a fait rougir pour rien : le debord est un arbitrage visuel, et
§6.6 ecarte les verrous de valeur, qui *« avaient une raison datee, pas une
raison permanente »*. Il **encadre** desormais — le miroir deborde, sans etre
demesure, et le facteur est le meme a toute echelle.

**603 tests**, 0 avertissement clippy.

### ⚠️ Le pilier polaire etait a moitie enfoui (2026-08-09)

Signale a l'oeil : *« les coeurs d'habitat de chaque cote semblent recouverts par
un autre fut d'habitat »*. Une **sonde** l'a chiffre au lieu de le deviner :

```
  y=  1.500  r= 3.000  MODULE AXIAL   <- segment de pilier
  y=  4.500  r= 7.250  MODULE AXIAL   <- habitat axial (fut 0..9)
  y=  4.500  r= 3.000  MODULE AXIAL   <- segment, au meme endroit
  y=  7.500  r= 3.000  MODULE AXIAL   <- segment, toujours dedans
```

Les **trois segments de fut par cote** couvraient exactement la meme tranche
(y de 0 a 9) que l'habitat axial deja pose, dont le rayon est plus du double.
Six pieces dessinees pour rien, et **invisibles** : aucun test ne les voyait,
puisque chacune etait individuellement bien formee.

Le fut du pilier, c'est donc l'habitat axial lui-meme ; `pilier_polaire` ne pose
plus que ses **coiffes**, au-dela. Le moyeu passe de 15 pieces a 7.

**Une garde nouvelle** : `le_pilier_n_enfouit_aucune_de_ses_pieces` verifie que
les modules axiaux se succedent le long de l'axe **sans se recouvrir**. C'est
exactement le defaut qui vient d'etre corrige, et il n'avait aucune chance
d'etre attrape autrement — chaque piece etait juste, c'est leur *ensemble* qui ne
l'etait pas. Red-check : reposer un segment enfoui rougit aussitot.

**Antennes de pole retirees.** Une parabole en bout ajoutait un detail d'echelle
incompatible avec une piece longue de plusieurs unites.

**Moyeu du miroir : un anneau, pas un point.** Le mat central et ses trois
haubans flottaient au milieu du trou de la variante percee, rattaches a rien de
visible — la piece paraissait tenue par un piquet. Une **couronne de membrures**
au bord du trou dit ce qui se passe vraiment : la membrane est tendue sur un
anneau, et c'est l'anneau qui porte.

**604 tests**, 0 avertissement clippy.

### Le decoupage tournant/fixe, revu (2026-08-09)

Demande : *« tout l'anneau avec l'ossature centrale tourne avec le reste ; les
deux gros modules coeur sont la partie qui ne tourne pas »*.

| Moitie | Contenu |
|---|---|
| **Tourne** | ossature hexagonale centrale, bras et tunnels, tore, coiffes de pole |
| **Fixe** | les deux gros coeurs d'habitat axiaux, et le miroir |

C'est ainsi qu'un axe « despun » fonctionne : le manege tourne **autour** du
moyeu, pas l'inverse. L'ossature porte l'anneau — la laisser immobile pendant
que sa jante tourne n'aurait rien a quoi se raccrocher.

⚠️ **Le miroir reste du cote fixe**, et ce n'est pas negociable : il doit garder
le soleil dans le meme axe. Solidaire de l'anneau, il ne renverrait la lumiere
qu'une fraction du temps — ce qui rendrait vaine l'inclinaison a 30° reglee juste
avant. La demande ne nommait que les deux coeurs ; c'est la seule piece que j'ai
gardee fixe sans qu'on me le dise, et la raison est ci-dessus.

Les coiffes de pole tournent, elles : posees **sur** l'axe de rotation, ce sont
des corps de revolution, donc sans effet visible — et ca evite une troisieme
moitie pour deux pieces.

### ⚠️ Deux tests filtraient sur le mauvais critere

La colonne d'habitat est desormais **a cheval sur les deux moities**, et deux
tests qui n'interrogeaient qu'une seule moitie ne voyaient plus qu'un bout. Ils
lisent la vue d'un seul tenant — c'est exactement ce pour quoi elle existe.

Et `le_pilier_n_enfouit_aucune_de_ses_pieces` filtrait sur le **type** de
composant : or les tunnels des bras sont des `ModuleAxial` eux aussi, mais
couches a l'horizontale. Leur etendue en Y est nulle, et les meler a la colonne
faisait s'entrelacer une douzaine d'intervalles degeneres. C'est
l'**orientation** qui definit la colonne, pas le type — le test filtre desormais
sur l'axe.

Nouvelle garde dans `les_deux_moities_du_tore_sont_disjointes_et_completes` : les
coeurs axiaux du cote fixe, l'ossature du cote tournant. **2 sabotages**, dont
celui qui deplace les coeurs et rougit six tests d'un coup.

**604 tests**, 0 avertissement clippy.

### La fleche polaire (2026-08-09)

Un **second coeur d'habitat** a chaque pole, au-dela de la coiffe, etire cinq
fois (25 unites) et **deux fois plus fin**.

Le profil vient de la grille, pas d'un facteur ecrit a la main : `FLECHE_PROFIL`
est **P0** la ou l'habitat est P2, soit le **quart** du diametre. Halvee une
premiere fois (P1), puis encore : a P1 la fleche restait un fut, et ce qu'il faut
ici est une **tige**. P0 est le plus fin que porte la grille, donc le bout de la
course — descendre plus bas demanderait un profil neuf. La colonne
s'effile donc en trois paliers — gros coeur, coiffe, fleche — au lieu de filer
d'un seul gabarit sur toute sa longueur, ce qui la ferait lire comme un tube et
non comme un mat.

**Toute la colonne axiale est desormais immobile** : coeurs, coiffes et fleches.
Les coiffes tournaient un temps — sans effet visible, ce sont des corps de
revolution poses sur l'axe de rotation — mais les ranger du cote fixe dit ce
qu'elles sont : la partie de la station ou l'on ne pese rien. Une garde le
verifie explicitement (aucun `CoeurGris` du cote tournant), et c'est
precisement celle qui manquait quand les coiffes y etaient encore.

Profil de la colonne, du centre vers le haut :

```
  y=  4.50  coeur accouple   (0..9,  P2)
  y= 11.50  coiffe           (9..14, P2)
  y= 26.50  fleche           (14..39, P0 — une tige)
  y= 54.78  miroir
```

⚠️ **Le miroir n'a pas bouge**, sur consigne explicite. Il reste a 2,25 rayons
d'anneau, et la fleche s'arrete a 39 : il subsiste donc un ecart d'une quinzaine
d'unites entre le sommet de la colonne et le disque. A l'oeil de trancher si ca
se voit — le miroir fait 40 de rayon, ce qui compense peut-etre.

**604 tests**, 0 avertissement clippy.

### Deux miroirs, et une colonne doublee (2026-08-09)

- **Un second miroir** de l'autre cote de l'anneau. Un seul laissait une moitie
  du tore eclairee et l'autre non, alors qu'il est vitre sur tout son pourtour.
- **Colonne deux fois plus longue** (`POLE_ETIREMENT` 5 → 10) : la fleche passe
  de 25 a 50 unites.
- **Miroirs rapproches d'un tiers** (2,25 → 1,5 rayon d'anneau) : deux disques
  aussi lointains ne se lisaient plus comme appartenant au meme objet.

### ⚠️ Le piege du second miroir, et le test qui l'a trouve

Les deux miroirs doivent regarder **le meme soleil**. Chacun part de la normale
qui fait face a l'anneau — −Y en haut, +Y en bas — puis s'incline vers le
soleil ; comme ils partent de cotes opposes, il faut les pencher **en sens
opposes**. Du meme signe, l'un des deux regarderait a l'oppose et ne renverrait
rien.

D'ou une constante `SOLEIL`, qui n'existe que pour ca : l'eclairage de la vue
suit la camera, il n'y a pas de vrai soleil a viser, et c'est **l'accord entre
les deux miroirs** qui compte, pas l'azimut choisi. Le test exige un produit
scalaire positif avec elle pour les deux, plus une normale qui redescend vers le
plan de l'anneau.

Il a servi tout de suite : **le signe de `vers_anneau` etait inverse**, et le
miroir du haut regardait a l'oppose du soleil (n·soleil = −0,5). Un defaut que
l'oeil aurait peut-etre laisse passer, l'eclairage suivant la camera.

Et le correctif ne s'est pas arrete au signe. L'axe du penchement se **deduit**
desormais de `SOLEIL` (`n × soleil`) au lieu d'etre un `rotation_z` de signe
choisi a la main — un axe calcule ne peut pas se tromper de cote, quel que soit
le pole et quelle que soit la direction donnee au soleil. Verifie dans les deux
sens : deplacer `SOLEIL` a un autre azimut laisse le test **vert** (les miroirs
suivent), figer l'axe le fait **rougir**.

### ⚠️ Deux assertions qui decrivaient un ordre de grandeur

`le_pilier_polaire_est_symetrique_et_coiffe` verifiait qu'« il y a quelque chose
au-dessus du pole » — vrai tant que le miroir etait l'element le plus haut. La
fleche doublee culmine desormais au-dessus de lui : les assertions ne decrivaient
pas une propriete mais la geometrie du moment. Remplacees par ce qui reste vrai —
les extremites de la colonne sont les points extremes de la station sur son axe.

⚠️ **A regarder** : la colonne monte a 64, les miroirs sont a 36,5. Les disques
sont donc traverses par la fleche a peu pres au tiers de sa hauteur. C'est
peut-etre exactement l'effet voulu (les miroirs portes par le mat), peut-etre pas.

**604 tests**, 0 avertissement clippy.

### Deux briques neuves : grappe de reservoirs et hangar (2026-08-09)

**`GrappeReservoirs`** — trois cuves spheriques par rangee, en triangle autour
de l'axe, sur plusieurs rangees empilees. Le parc n'avait que le reservoir
**unitaire** : une soute d'ergols se construisait cuve par cuve, autant de pieces
et autant de ports consommes, et aucune operation ne traitait la grappe comme un
tout. C'est desormais **une** piece — elle se pose, se deplace et se retire d'un
bloc.

Trois par rangee est le plus petit groupe qui **enserre un axe** : deux
laisseraient l'ensemble plat, quatre gaspillent le volume entre les cuves. Le
rayon de disposition se **calcule** pour que deux voisines soient tangentes
(`d = r / sin(π/n)`), au lieu d'etre un nombre trouve a tatons : le jour ou le
nombre par rangee change, la tangence suit.

**`Hangar`** — un fut d'habitat prolonge d'une **baie parallelepipedique**. Le
parc savait faire des cylindres et des caissons techniques, mais rien qui
**abrite** : pas de volume creux avec une porte par ou sort un engin. Un hangar
tout cylindrique perdrait ses coins (un engin est plus large que haut) ; une
boite seule ne se raccorde a rien, tout le reste du parc etant de revolution.
D'ou les deux — le fut donne l'accroche et le vocabulaire, la baie donne le
volume et la porte. Deux variantes, ouverte et fermee.

### ⚠️ Un sabotage vert, et le commentaire qu'il a corrige

Le decalage d'une demi-rangee entre rangees successives etait presente comme
« serrant le volume ». **Faux** : les rangees etant espacees d'un diametre,
l'empilement en colonnes garde les cuves tangentes tout autant, et supprimer le
decalage ne rougit **aucun** test. C'est un choix de lecture, rien de plus — le
commentaire le dit maintenant, et aucune garde ne le protege. On ne teste pas un
gout.

### Red-check : 6 sabotages

| Sabotage | Rougit |
|---|---|
| disposition approximative au lieu du calcul de tangence | la tangence |
| rangees espacees d'un rayon | l'empilement **et** l'enveloppe globale |
| decalage de rangee supprime | **rien** — voir ci-dessus |
| baie plus etroite que le fut | le debord de baie |
| port de toit tourne vers l'avant | l'ecoutille et le toit |
| cout independant de la profondeur de baie | le cout au volume |

**604 → 615 tests**, 0 avertissement clippy. La chaine d'echantillons passe de 41
a 43 variantes, `docs/reference/fils.md` regenere.

### ⚠️ La grappe etait surchargee (2026-08-09)

Juge a l'oeil : *« la grappe de reservoirs a l'air tres bizarre avec les choses
ajoutees, compare au reservoir spherique d'origine »*.

J'avais ajoute une **ame centrale**, des **colliers** vers l'axe et un
**cerclage** par rangee — de la structure censee « expliquer ce qui relie les
cuves ». A l'ecran, ca brouillait tout : la grappe ne ressemblait plus au
reservoir unitaire dont elle est le pluriel, mais a un assemblage indistinct.

Le traitement est desormais **exactement** celui de `reservoir.rs` : sous-couche
sombre pour que les joints ressortent, puis icosphere facettee a la **meme
subdivision** (3, et non 2 comme je l'avais mise — un ecart qui suffisait deja a
distinguer les deux pieces sans raison). Deux pieces de la meme famille doivent
se peindre pareil, sinon leur parente ne se voit pas.

**La lecon, et elle revient** : ajouter de la structure pour « expliquer » une
piece la rend souvent moins lisible, pas plus. C'est le meme reflexe qui avait
plante un mat au centre du miroir, et le meme oeil qui l'a corrige.

Une garde compte ce que recoit le peintre — **deux spheres par cuve, et rien
d'autre**. Toute structure rajoutee emet des cylindres, et le compte s'en
apercoit. Red-check : remettre un seul cylindre rougit.

**616 tests**, 0 avertissement clippy.

### La baie du hangar passe a l'hexagone (2026-08-10)

Juge a l'oeil : *« le hangar a besoin d'un bon relooking : la face du
parallelepipede tournee vers Z doit devenir un hexagone, dont le cote parallele
a x est double »*.

La baie etait un parallelepipede. Deux defauts, l'un de vocabulaire, l'autre de
geometrie :

1. **Elle ne rimait avec rien.** L'hexagone est partout dans le parc — treillis,
   tours, pales, ceinture des nacelles. La boite a angles droits etait la seule
   forme du vaisseau sans echo, et elle se lisait comme un colis arrime.
2. **Le raccord ne tenait pas.** Le tronc de cone qui evasait le fut vers la
   baie visait la **demi-largeur** de celle-ci ; a `1,55·r` contre une
   demi-hauteur de `1,05·r`, il **depassait de la silhouette** en haut et en
   bas. Defaut present depuis le premier jet, jamais vu : aucun test ne
   comparait les deux cotes.

L'hexagone corrige les deux. Le raccord est desormais **l'hexagone lui-meme**,
reduit par [`echelle_pied`] jusqu'a s'inscrire exactement dans le cercle du fut
— une homothetie de la section ne peut pas deborder de la section.

#### La section

Une seule cote maitresse, `demi_hauteur = 1,05·r` (inchangee) ; tout le reste
**derive** de la geometrie de l'hexagone, aucun facteur ecrit a la main :

| Cote | Formule | A P2 (`r = 2`) |
|---|---|---|
| cote oblique | `demi_hauteur · 2/√3` | 2,42 |
| demi-plat (cote ∥ x) | `cote · ALLONGE/2` | 2,42 |
| demi-largeur | `demi_plat + cote/2` | 3,64 |

`ALLONGE = 2` est la demande : les deux cotes paralleles a x valent deux fois
chacun des quatre obliques. A 1 on aurait l'hexagone regulier — un tuyau a six
pans, aussi haut que large, qui n'abriterait rien de mieux qu'un cylindre.
**C'est ce facteur, et lui seul, qui fait la difference entre une section de
coque et une baie.** La demi-largeur passe de `1,55·r` a `1,82·r` : la baie a
gagne en largeur ce que l'allongement demandait, a hauteur constante.

Aucune primitive du `Peintre` n'a de section non circulaire : le prisme sort en
triangles bruts, comme `pieces::prisme_onigiri`. L'evasement est **noye** de 3 %
dans la baie et retire de 1 % en echelle, sans quoi sa face haute et la face
basse de la baie se disputeraient le plan `z = fut` et clignoteraient l'une a
travers l'autre — meme piege, meme remede, que les collerettes de
`nacelle_cargo`.

#### Ce que l'hexagone change ailleurs

- **Les rails de toit** etaient poses a `demi_largeur·0,62`. Sur un hexagone, au
  dela du demi-plat on est **sur la pente** : ils sont reindexes sur le plat.
- **L'encadrement de porte** etait quatre barres droites — bon pour un
  rectangle. C'est un jonc suivant les six aretes de la bouche.
- **L'enveloppe** prenait la demi-diagonale de la boite. L'hexagone n'a pas de
  coin en `(demi_largeur, demi_hauteur)` : reserver ce coin-la interdisait de
  poser une piece dans un **vide reel**. Elle prend le cercle circonscrit.
- **Le cout** suivait le volume de la boite. Il suit l'aire de l'hexagone, 17 %
  plus petite : on ne facture pas un vide qu'on n'abrite pas.

#### Red-check : 3 sabotages, 3 rouges

| Sabotage | Rougit |
|---|---|
| `demi_plat` a `ALLONGE·0,4` au lieu de `·0,5` | l'hexagone allonge |
| `echelle_pied` indexee sur la demi-hauteur | le pied de l'evasement |
| aire de section = celle de la boite englobante | le cout au volume |

Le test d'hexagone se mesure **contre `ALLONGE`**, pas contre `2.0` : changer la
constante doit changer la forme sans rougir, changer la formule doit rougir.
C'est le contraire d'un verrou de valeur (§6.6).

**616 → 618 tests**, 0 avertissement clippy, `docs/reference/fils.md` regenere
(la baie passe de 12 a 18 fils : un prisme hexagonal a six aretes de plus qu'une
boite).

### Charge utile du mât polaire bas (2026-08-10)

Demande : *« sur la colonne du bas du tore, ajouter l'habitat de l'ISV dans la
même disposition en triple, et dessous, avec un peu d'espace, la grappe de
réservoirs »*.

Les deux charges partagent la **même figure** — trois volumes en couronne autour
d'un axe vide, où passe le mât. Ce n'est pas une coïncidence dont on profite :
c'est ce qui fait qu'elles se lisent comme deux étages d'une même pile.

#### ⚠️ Au jeu nominal, les trois modules se croisaient

Mesuré avant de poser, et c'est ce qui a sauvé le montage. La couronne
d'habitat de l'ISV se cale sur le gabarit de l'épine : `épine + jeu + inscrit`.
Sur la flèche polaire, qui est en **P0** (rayon 0,5 contre 1,12 pour l'épine),
ça donne un rayon de 1,97 — or il en faut **2,07** pour que trois sections
onigiri P2 ne s'interpénètrent pas. Les trois fûts se seraient croisés, et
l'assemblage serait resté « valide ».

Le calcul qui l'établit **existait déjà** : `la_charge_utile_suit_le_gabarit_de
_lepine` le faisait pour *vérifier* le montage de l'ISV. Il **décide**
désormais (`hab_rayon_mini`), ce qui est sa place (§10.9) — un chiffre qui
contrôle une pose ne doit pas vivre uniquement dans un test.

La couronne retient donc **la plus large des deux contraintes**, et c'est la
**ferrure** qui s'allonge pour rejoindre le mât, jamais la couronne qui se
rapproche. Sur l'ISV le jeu commande toujours et rien ne bouge : couronne 2,589
et ferrure 0,500, aux valeurs historiques — tenu par
`la_couronne_de_lisv_est_inchangee`, sans quoi la généralisation aurait déplacé
l'habitat du vaisseau au passage.

| | ISV (épine 1,12) | mât polaire (P0, 0,5) |
|---|---|---|
| couronne | 2,589 (le jeu) | 2,318 (l'encombrement) |
| ferrure | 0,500 | 1,196 |

#### Ancrage par le bout, pas par le haut

Toute la pile se mesure **depuis le bout de la flèche** et remonte : cuves d'
abord (pied à −64, exactement la pointe du mât), puis l'écart de mât nu, puis
l'habitat. Ancrée par le haut, la dernière rangée de cuves serait passée dans le
vide dès qu'une cote bouge — et personne ne l'aurait vu. Ce qui doit rester vrai,
c'est que la pile **finit sur le mât**.

Disposition obtenue : habitat en −40,4..−48,4 (couronne 2,32, hors-tout 4,32),
mât nu sur 6, cuves en −54,4..−64,0 (hors-tout 3,45). La pile **s'affine en
descendant**, comme le mât lui-même sous les cœurs ; l'inverse ferait une massue.

C'est la seule chose qui **rompt la symétrie des deux pôles**, et c'est voulu :
le haut porte l'amarrage, le bas porte ce qui se consomme. Une station symétrique
jusqu'au bout n'a ni haut ni bas.

#### ⚠️ Marge de 1,8 sous le miroir bas — à voir à l'œil

Les disques font 40 unités de rayon et sont inclinés à 30° : ce qu'on ajoute sur
la colonne passe forcément dans leur voisinage. Aucune pièce n'est **à cheval**
sur un miroir (garde `la_charge_polaire_ne_traverse_aucun_miroir`), mais le
module d'habitat le plus proche n'est qu'à **1,8** du plan du disque, et toute la
charge est **derrière** le miroir bas — donc masquée depuis les points de vue
situés au-dessus de l'anneau. À juger à l'écran ; se corrige en réduisant
`POLE_ECART`, qui fait descendre la charge.

#### Red-check : 4 sabotages

| Sabotage | Rougit |
|---|---|
| cuves ancrées 5 plus haut que le bout | l'enfilage sur le mât |
| couronne sans le plancher d'encombrement | le croisement des modules |
| plancher porté à 8 fois le jeu | la couronne de l'ISV |
| charge remontée dans le plan du miroir (écart 14) | la traversée de miroir |

⚠️ **Le premier jet du 4ᵉ sabotage n'a rien rougi** (écart 20) : la charge
franchissait bien le plan du miroir mais restait à 1,3 de dégagement. Un
sabotage qui ne rougit pas ne prouve pas que la garde est faible — encore
faut-il qu'il produise vraiment le défaut visé. Refait à 14, il rougit.

**618 → 622 tests**, 0 avertissement clippy.
