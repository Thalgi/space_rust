# Conception — briques à variantes pour stations procédurales

Document de travail. **But immédiat** : constituer un *bon lot de variantes* pour
chaque type de brique (structure, habitat, nœud, panneau solaire, radiateur,
antenne/parabole, appendices), afin que des stations type ISS générées
procéduralement aient de la **variété visuelle** tout en restant low-poly,
réalistes et esthétiques.

**But final** : un générateur `Station::generer(seed, params)` qui assemble ces
variantes selon une grammaire (voir §5).

---

## 0. État (2026-07-19)

Le **modèle de ports** (Étape 1) est en place et testé — voir
[`stations_raccordement.md`](stations_raccordement.md). Composants déjà
implémentés dans `src/vaisseau/composant.rs` (enum `Composant`) :

- `ModuleAxial` — cylindre pressurisé (collerettes de docking) en 6 variantes
  d'habitat : `Standard`, `Dore`, `Hublots`, `Labo`, `Gonflable`, `Coupole` ;
- `Noeud` — hub sphérique multi-ports, 4 dispositions : `Quatre` (croix plane),
  `Six` (croix 3D), `T` (plan XZ), `Tetra` (tétraèdre) ;
- `PanneauSolaire` — 5 variantes : `RigideUS`, `RusseBleu`, `RollOut`,
  `Futuriste`, `Hexagonal` (tuiles hexagonales espacées, en maillage) ;
- `Treillis` — poutre-ossature, 2 styles (`Carre`, `Triangulaire`) × gabarits
  (profil), **avec ports hôtes `Surface`** répartis sur la longueur ;
- `Radiateur` — 7 variantes : `PanneauSimple`, `AccordeonATCS`, `PivotantTRRJ`,
  `Caloducs`, `Deroulable`, `Corps` (6 technos réelles) + `Gouttelettes` (LDR,
  exotique) ;
- `Antenne` — 6 variantes : `ParaboleGG`, `ParaboleOffset`, `Cornets`, `Fouet`,
  `ReseauPhase`, `Helice`.

**Montage factorisé** : tous les appendices (panneau, radiateur, antenne) se
montent par le **même genre générique `Surface`** ; un port hôte `Surface` en
accepte donc n'importe lequel, le `profil` gérant la taille. Les genres
`MontageAile`/`MontageRadiateur` ont été supprimés. **Ports hôtes `Surface`
partout** : sur le treillis (paires ±X), sur le **module** (±X, ±Y radiaux) et
sur le **nœud** (faces principales libres) — stations type Mir possibles.

**Constructeur `Chantier`** (`src/vaisseau/chantier.rs`) — le fondement du
générateur : il suit les **ports hôtes libres**. `racine(comp)`, puis
`poser(hote_idx, comp, montage_idx)` qui vérifie compatibilité + budget +
**anti-collision** (sphères englobantes géométriques, hôte direct exempté), le
port consommé et les ports de l'enfant libérés. `compatibles()` liste où poser.

**Générateur** `generer(&ParamsStation)` (`src/vaisseau/generateur.rs`) : RNG
déterministe (splitmix64), `Style` (Historique/Russe/Futuriste), `Ossature`
(Iss/Mir, ou tirée à la graine), grammaire par-dessus le `Chantier` (ossature →
armatures en treillis → habillage par axe : panneaux ±X, radiateurs ±Y, antennes
±Z). Vue **STATION** : démo 0 = générateur (1-4 complexité, O ossature, G graine,
S style), démos 1-2 = **presets ISS/Mir**, puis les vitrines de composants.

`preset_iss()` est une **reproduction ISS assemblée à la main** (référence
d'après la vue éclatée NASA) : hub central, poutre intégrée transverse (arrays
outboard, radiateurs inboard), segment US ramifié (Node1→Lab→Node2 + Columbus/
Kibo + grappe Node3/Cupola), segment russe (FGB→SM + petits arrays + nœud MRM).

**Manques du générateur pour atteindre la fidélité ISS** (constatés en comparant
au preset) :
- **topologie en croix décalée** : la poutre est *transverse* au cœur habité (via
  un boom court type Z1), pas une épine ; le générateur ne pose pas encore la
  poutre transverse dédiée ;
- **attache mi-poutre** : le treillis n'a de ports qu'aux bouts ; impossible d'y
  accrocher un module en son *milieu* (l'ISS s'y connecte via Z1) ;
- **zonage des appendices** : arrays aux extrémités, radiateurs inboard — règle à
  porter dans le générateur ;
- **adaptateur de profil** (P1↔P2) pour les jonctions cœur↔grosse poutre ;
- **symétrie bâbord/tribord structurée** (le générateur reste stochastique).

Reste (détaillé aux §4–5) : combler ces manques ; composants optionnels
(appendices dockés Soyouz/cargo, styles de nœud/treillis) ; atelier à deux axes ;
styles/palettes.

---

## 1. Rappel : ce qui existe déjà

- Briques factorisées dans `src/vaisseau/pieces.rs` : `treillis`, `module`,
  `pale_solaire`, `paire_ailes`, `radiateur` (paramétrées, orientables).
- Primitives orientées dans `mod.rs` : `cylindre`, `cone`, `parabole`, `voile`,
  `panneau`.
- Atelier de visualisation `ecran/briques.rs` (menu « BRIQUES »), flèches
  haut/bas pour changer de brique.
- ISS de référence reconstruite à partir de ces briques (calibrage).

Il manque : **plusieurs variantes par type**, et un moyen de les parcourir.

---

## 2. Modèle de variante (à implémenter en premier)

Chaque type de brique devient un **enum de variantes** + des **paramètres
continus**. Une brique concrète = `(Type, Variante, Params, Palette)`.

```rust
// Exemple pour les panneaux solaires.
enum VariantePanneau {
    RigideUS,     // ambre, 2 lés rigides (P4/P6…)
    RusseBleu,    // bleu, plus court
    RollOut,      // iROSA, étroit et foncé, posé sur un rigide
    Futuriste,    // cyan
}

struct ParamsPanneau { longueur: f32, largeur: f32, cellules: usize, ecart: f32 }
```

Règle : **une seule fonction de dessin par type**, qui `match` sur la variante.
Les variantes partagent le maximum de code (les lés, le cadre, les nervures
restent factorisés).

---

## 3. Points d'accroche (le cœur de l'accouplement)

> **État : implémenté** dans `src/vaisseau/port.rs` (`Repere`, `Port`,
> `GenrePort`, `accoupler`), couvert par 13 tests (`cargo test port`). Les 5 cas
> limites — coïncidence des positions, opposition des avants, verrouillage du
> roulis, robustesse en chaîne, garde-fou de compatibilité — sont validés.
> Reste à poser le **trait `Composant`** (`ports()` + `dessiner()`, Étape 2).
> Écart avec le brouillon ci-dessous : le champ `diametre` a été remplacé par un
> **`profil: Profil`** (enum discret P0..P3, cf. `unites.rs`), plus sûr qu'un
> flottant pour le « snap » et la compatibilité.

Idée retenue : **chaque composant expose des points d'accroche** (ports)
orientés. Un composant s'assemble en « clipsant » un de ses ports sur un port
libre d'un composant déjà posé. C'est le modèle d'attache par nœuds (façon
Kerbal Space Program) — il rend triviales les stations qui se ramifient (nœuds,
modules radiaux, panneaux le long d'une poutre) et garantit des jonctions
propres, sans positions codées en dur.

### 3.1 Ce qu'est un port

Un port n'est **pas un simple point** : c'est un **repère orienté** local au
composant.

- `pos` : où est le port sur le composant.
- `direction` : le sens d'accouplement **sortant** (vers l'extérieur). Deux
  ports s'apparient quand leurs directions sont **opposées** (face à face).
- `haut` : référence de roulis, pour un accouplement totalement contraint (sinon
  ambiguïté de rotation autour de l'axe). On peut aussi laisser un roulis
  aléatoire/paramétré pour varier.
- `genre` : type de connexion (compatibilité, voir §3.3).
- `profil` : taille nominale discrète (`P0..P3`, cf. `unites.rs`) — évite
  d'accoupler un module de 4 m sur un port de 0,5 m, et sert au « snap ». (Choisi
  plutôt qu'un `diametre: f32` : la compatibilité devient une égalité d'enum,
  sans cas limite numérique.)

```rust
// Tel qu'implémenté dans src/vaisseau/port.rs :
struct Repere { pos: Vec3, rot: Quat } // avant = rot*Z, haut = rot*Y

enum GenrePort {
    ModuleAxial,   // hatch/CBM en bout de module
    ModuleRadial,  // face d'un nœud
    PoutreBout,    // extrémité de treillis
    Surface,       // montage d'appendice GÉNÉRIQUE : panneau, radiateur,
                   // antenne, capteur (factorisé — un port hôte les accepte tous)
}

struct Port { repere: Repere, genre: GenrePort, profil: Profil } // profil P0..P3

// À poser en Étape 2 :
trait Composant {
    fn ports(&self) -> Vec<Port>; // dans le repère local
    fn dessiner(&self);           // dans le repère local
}
```

### 3.2 Port de montage vs ports hôtes

Un composant a **un port de « montage »** (celui par lequel il se rattache à son
parent) et **0..n ports « hôtes »** libres (où viennent ses enfants). En
pratique c'est la même liste : on marque simplement le port consommé comme
occupé. Un composant peut donc être relié à **1 ou n structures** — exactement
ce qu'on veut.

### 3.3 Compatibilité

On n'accouple que des ports de **genres compatibles** (table de compatibilité) :
un appendice (panneau/radiateur/antenne) se monte sur `Surface`, un module sur
`ModuleAxial`/`Radial`, etc.
Le générateur ne pioche que dans les ports libres compatibles.

### 3.4 Calcul d'accouplement

Attacher un enfant (port de montage `pm`, local) sur un port hôte `ph` déjà en
coordonnées monde :

```rust
// On veut : enfant.avant == -hote.avant, et les positions des ports coïncident.
fn accoupler(ph: Repere, pm: Repere) -> Repere {
    let face_a_face = ph.rot * Quat::from_rotation_y(PI); // demi-tour autour du "haut"
    let rot = face_a_face * pm.rot.inverse();
    let pos = ph.pos - rot * pm.pos;
    Repere { pos, rot } // transformée monde de l'enfant
}
```

Le rendu applique ensuite ce `Repere` via `push_model_matrix` avant d'appeler
`dessiner()`. (Détail : le demi-tour se fait autour de l'axe *haut* pour que les
« avant » s'opposent tout en gardant les *haut* alignés ; le roulis fin se règle
avec la référence `haut` du port.)

### 3.5 Deux familles d'hôtes

- **Ports discrets** : hatch de module, faces d'un nœud → liste finie. *(à faire
  en premier)*
- **Rails continus** : bord d'une poutre où l'on peut monter panneaux et
  radiateurs à **n'importe quel décalage** → un « rail » qui génère des ports à
  la demande. *(étape ultérieure)*

### 3.6 Symétrie

Marquer les ports en **paires miroir** (ex. +Y / −Y d'un nœud, gauche/droite
d'une poutre) : le générateur y place des enfants **appariés**, indispensable au
look ISS. Un `groupe_symetrie: Option<u8>` sur le port suffit.

### 3.7 Coût / garde-fous

- Plus de machinerie en amont qu'un placement codé en dur — mais c'est justement
  ce qui débloque ramification + variété (le but).
- V1 volontairement minimale : `Repere` + `genre` + `occupe`. Le reste
  (diamètre, symétrie, rails) s'ajoute ensuite.
- Les ports ne suffisent pas contre les **chevauchements à distance** (deux
  enfants voisins qui se croisent) : garder une vérification de boîtes
  englobantes en filet de sécurité (§7).

---

## 4. Le lot de variantes visé (cible : 3–5 par type)

### 3.1 Structure (treillis / poutre)

- [x] `Carre` — 4 longerons + cadres/diagonales (barres en cylindres, du volume).
- [x] `Triangulaire` — 3 longerons (plus léger, look « sonde »).
- [ ] `Caisson` — tube/box plein, faces pleines.
- [ ] `AvecRails` — poutre + rail du transporteur mobile (détail ISS).
- Axes : longueur, gabarit (via `profil`) ; ports hôtes `Surface` répartis.

### 3.2 Habitat (module)

- [x] `Standard` — blanc simple.
- [x] `Dore` — teinte or (segment russe).
- [x] `Hublots` — rangée de hublots + mains courantes EVA.
- [x] `Labo` — grande fenêtre + rack externe (type Destiny).
- [x] `Gonflable` — profil bombé (type BEAM).
- [x] `Coupole` — coupole vitrée à un bout (type Cupola).
- Implémenté comme champ `variante` de `ModuleAxial` (couleur + `details()`).
  Axes : `profil`, `longueur`.

### 3.3 Nœud d'amarrage

- [x] `Spherique` — multi-ports (type Mir) : dispositions `Quatre`, `Six`, `T`,
  `Tetra` ; sphère gonflée, bras cylindriques ancrés + collerette par sortie.
- [ ] `Cubique` — nœud US (Unity/Harmony).
- [ ] `AvecCupola` — coupole facettée orientable.
- Axes : disposition/nombre de ports, profil.

### 3.4 Panneau solaire

- [x] `RigideUS` — ambre, 2 lés rigides.
- [x] `RusseBleu` — bleu, plus court.
- [x] `RollOut` — iROSA, bande étroite plus foncée.
- [x] `Futuriste` — cyan, plus large.
- [x] `Hexagonal` — tuiles hexagonales espacées (maillage).
- Axes : longueur, largeur ; couleur/proportions portées par la variante (`style()`).
- Reste : la **paire d'ailes** en données (symétrie miroir sur un port hôte
  `Surface` — le treillis en expose déjà).

### 3.5 Radiateur

- [x] `PanneauSimple` — panneau plat rainuré (body-mounted).
- [x] `AccordeonATCS` — corrugation en zigzag (bank ISS).
- [x] `PivotantTRRJ` — gros joint rotatif visible.
- [x] `Caloducs` — tubes cuivre apparents (loop heat pipe).
- [x] `Deroulable` — gros tambour + bande étroite dorée (roll-out).
- [x] `Corps` — large et court, sombre (body-mounted).
- [x] `Gouttelettes` — **exotique** : rideau de gouttelettes (LDR).
- Chaque variante porte sa couleur/proportions/silhouette. Reste : ports hôtes
  `Surface` (le treillis en expose déjà — panneau/radiateur/antenne s'y clipsent).

### 3.6 Antenne / Parabole

- [x] `ParaboleGG` — grand gain, orientée +Z.
- [x] `ParaboleOffset` — parabole à alimentation décalée, inclinée.
- [x] `Fouet` — fouets omni croisés.
- [x] `ReseauPhase` — plaque plate quadrillée (réseau phasé).
- [x] `Cornets` — grappe de cornets (horns).
- [x] `Helice` — antenne hélicoïdale.
- Monté par un port `Surface` ; axe : `taille`.

### 3.7 Appendices (vaisseaux amarrés)

- [ ] `Soyouz` — vert, petits panneaux.
- [ ] `CargoUS` — Dragon/Cygnus.
- Axes : taille, couleur.

---

## 5. Étapes claires

**Étape 1 — Modèle de points d'accroche** ✅ *fait*
`Repere`, `Port`, `GenrePort` et `accoupler` posés dans `src/vaisseau/port.rs`,
validés par 13 tests (5 cas limites). Reste, avant l'Étape 2, à ajouter le trait
`Composant` (`ports()` + `dessiner()`) et à valider sur deux modules bout à bout
puis un nœud + modules radiaux.

**Étape 2 — Modèle de variante**
Transformer chaque brique de `pieces.rs` en `match` sur un enum de variante +
struct de params, et faire exposer ses `ports()`. Commencer par un seul type
(panneau solaire) de bout en bout.

**Étape 3 — Atelier à deux axes**
Étendre `ecran/briques.rs` : **haut/bas = type de brique**, **gauche/droite =
variante**. Afficher « TYPE — Variante (i/n) » en bas à gauche, et
(option) visualiser les ports (petites flèches). Outil de réglage au cas par cas.

**Étape 4 — Remplir le lot**
Implémenter les variantes listées au §4, une par une, en les vérifiant dans
l'atelier. Objectif : 3–5 variantes par type.

**Étape 5 — Palettes / styles**
Regrouper les couleurs en `Style` (Historique argent+ambre, Russe or+bleu,
Futuriste métal+cyan). Une variante peut être compatible avec un sous-ensemble
de styles.

**Étape 6 — Station en données**
Introduire `Piece { composant, variante, params, style }` reliées par ports, et
l'assemblage en `Vec<Piece>` (transformées calculées par `accoupler`). Réécrire
Mir / ISS / Tiangong comme **données** pour valider que le lot couvre les cas
réels.

**Étape 7 — Générateur**
`Station::generer(seed, params)` : grammaire (§6) qui tire des variantes au
hasard (RNG graine) dans le style choisi, en clipsant sur les ports libres
compatibles, avec les contraintes d'espacement.

**Étape 8 — Cohérence & collisions**
Rails continus (§3.5), boîtes englobantes par pièce, espacement mini entre
groupes de panneaux, symétrie miroir/radiale.

---

## 6. Grammaire d'assemblage (pour l'étape 7)

1. **Épine dorsale** : poutre (type ISS) ou enfilade de modules (type Mir), posée
   en clipsant module sur module par leurs ports axiaux.
2. **Nœuds** tous les N modules ; 0–4 modules radiaux tirés sur les ports
   radiaux libres (symétrie via les paires miroir).
3. **Énergie** : paires d'ailes sur les ports hôtes `Surface` symétriques, `ecart`
   inter-paire garanti (> largeur de pale) pour ne jamais coller.
4. **Thermique** : un radiateur par surface X de panneaux.
5. **Appendices** : antennes/paraboles sur les ports `Surface` libres ; vaisseaux
   sur les ports axiaux terminaux.
6. **Style** : toutes les variantes tirées dans la palette du style choisi.

Paramètres exposés : `seed`, `taille`, `nb_paires_ailes`, `symetrie`, `style`,
`densite_details`.

---

## 7. Garde-fous esthétiques (low-poly réaliste)

- Cellules solaires : couture centrale + nervures ; ambre US / bleu russe.
- Treillis réellement ajouré (longerons + diagonales).
- Modules : anneaux de jonction sombres pour lire les raccords.
- Radiateurs franchement blancs, distincts des panneaux.
- Jamais de sphère pour un élément orienté (paraboles = cônes orientés).
- Variété *dans le style* : deux stations d'un même style doivent différer par
  le choix et le placement des variantes, pas par des couleurs incohérentes.

---

## Sources

- [Integrated Truss Structure — NASA](https://www.nasa.gov/international-space-station/integrated-truss-structure/)
- [Integrated Truss Structure — Wikipedia](https://en.wikipedia.org/wiki/Integrated_Truss_Structure)
- [Electrical system of the International Space Station — Wikipedia](https://en.wikipedia.org/wiki/Electrical_system_of_the_International_Space_Station)
