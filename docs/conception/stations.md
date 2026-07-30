# Conception — Stations spatiales

> Fusion de quatre anciens documents de conception, dans l'ordre de lecture
> conseillé : le plan directeur (`stations_procedurales.md`), les fondations
> transverses (`stations_fondations.md`), le raccordement ports↔assemblage
> (`stations_raccordement.md`), et la feuille de route des classes de station
> (`classes_stations.md`). Plus une **Partie E** (2026-07-29) : refonte du
> système de composants (découpage de `composant.rs` + composite
> `SousEnsemble`), en préparation d'un futur éditeur d'assemblage façon VAB
> (Kerbal Space Program). Suivi/passation :
> [`docs/suivi/stations.md`](../suivi/stations.md) — priorités immédiates en
> tête de ce document.

---

## Partie A — Plan directeur : briques à variantes pour stations procédurales


Document de travail. **But immédiat** : constituer un *bon lot de variantes* pour
chaque type de brique (structure, habitat, nœud, panneau solaire, radiateur,
antenne/parabole, appendices), afin que des stations type ISS générées
procéduralement aient de la **variété visuelle** tout en restant low-poly,
réalistes et esthétiques.

**But final** : un générateur `Station::generer(seed, params)` qui assemble ces
variantes selon une grammaire (voir §5).

---

### 0. État (2026-07-28)

Le **modèle de ports** (Étape 1) est en place et testé — voir la Partie C
ci-dessous. Composants déjà implémentés dans `src/vaisseau/composant.rs`
(enum `Composant`) :

- `ModuleAxial` — cylindre pressurisé (collerettes de docking + **bague
  d'accostage** alu au bout) en 7 variantes d'habitat : `Standard`, `Dore`,
  `Hublots`, `Labo`, `Gonflable`, `Coupole`, `Sas` (écoutille EVA, type Quest) ;
- `Noeud` — hub sphérique multi-ports, 4 dispositions : `Quatre` (croix plane),
  `Six` (croix 3D), `T` (plan XZ), `Tetra` (tétraèdre) ;
- `PanneauSolaire` — 5 variantes : `RigideUS`, `RusseBleu`, `RollOut`,
  `Futuriste`, `Hexagonal` (tuiles hexagonales espacées, en maillage) ;
- `Treillis` — poutre-ossature, 2 styles (`Carre`, `Triangulaire`) × gabarits
  (profil), **avec ports hôtes `Surface`** répartis sur la longueur ;
- `Radiateur` — 8 variantes : `PanneauSimple`, `AccordeonATCS`, `PivotantTRRJ`,
  `Caloducs`, `Deroulable`, `Corps` (6 technos réelles) + `Gouttelettes` (LDR,
  exotique) + `Voile` (grande voile radiante à l'échelle vaisseau, type ISV) ;
- `Antenne` — 6 variantes : `ParaboleGG`, `ParaboleOffset`, `Cornets`, `Fouet`,
  `ReseauPhase`, `Helice` ;
- `Adaptateur` — tronc de cône à **deux écoutilles axiales de profils
  différents** : sert à la fois de **nez de docking** (PMA/IDA) et
  d'**adaptateur de profil** (P1↔P2).

**Ajouts 2026-07-23 (briques classe C — ISV).** Depuis, `ModuleAxial` compte
**10 variantes** (ajout de `GrandGonflable`, `Serre`, `Coeur` étagé). Nouvelles
briques dans `composant.rs` :

- `Charpente` — treillis **courbe à section variable** (P3 base → P1 flèche),
  option `aiguille` (anneau hexagonal en treillis au pied) ; épine de l'ISV.
- `RadiateurMega` — grande aile **en arête de poisson** (boom + ailettes), échelle
  mégastructure ; les voiles radiateurs de l'ISV.
- `Motrice`, `BlocMoteur` — nacelle/bloc moteur (caisse collecteur + rangée
  d'habitats), base de la partie propulsion.
- `Reservoir` — cuve **sphérique** à tuiles, tenue par une **cage tétraédrique**
  de 4 barres ; réservoirs de carburant.
- `Coiffe` — capuchon de nez de module, 3 formes (`Bombee` demi-dôme fermé,
  `Hexagonale` à face plate, `Amarrage` adaptateur d'accostage) ; posée sur
  l'écoutille axiale d'un module (à ras du corps).
- **Bloc propulsion antimatière**, deux briques chaînées : `ReacteurAntimatiere`
  (cuve sombre + 4 bobines EM en cryostat + tuyauterie + injecteur & pièges à
  antiprotons) puis `MoteurAntimatiere` (tuyère : **buse** cylindrique + **cage
  de 2 cercles ouverts sur 4 tiges** ancrées à la buse, cœur d'annihilation).

**Ajouts 2026-07-29 (fret de vaisseau).** Deux briques pour la section charge
utile de l'ISV, **créées de zéro** plutôt que dérivées de `Caisson`/
`ChargeUtile` : ces derniers sont du vocabulaire ISS (porteurs d'ORU, berceaux
FRAM, poignées EVA), à la fois trop petits et de mauvaise silhouette pour du
fret interstellaire.

- `NacelleCargo` — conteneur **long** à section **onigiri** : un triangle à
  coins congés, aux **côtés droits** (trois arcs reliés par trois segments). La
  section triangulaire est structurelle, pas décorative — c'est la forme qui ne
  se déforme pas sous charge, et la seule qui s'empaquette sans vides autour
  d'une épine (des cylindres gaspilleraient tout l'entre-deux).
- `ModuleHabitat` — module d'**habitat principal** (fixe, solidaire de l'épine ;
  **pas** les modules d'équipage rotatifs, qui restent à faire). Même section
  onigiri que la nacelle, en plus gros — c'est ce qui tient la famille visuelle
  du vaisseau — mais coque composite **nue** (ni collerette sombre ni rail
  d'arête), trois **armatures hexagonales** aux quarts, et sur **un seul** côté
  plat deux **ferrures d'attache** orientées par `spin` : le module se boulonne
  à l'épine par ce côté au lieu de flotter à côté.

  *Piège de section, appris en le ratant deux fois* : sur un triangle **congé**,
  aucune intuition de triangle à coins vifs ne tient. Une corde d'un coin au
  suivant ne longe pas le côté plat (`0,52 r` contre `0,61 r`) ; et un hexagone
  posé sur les **points de tangence** rate encore, sa corde courte coupant l'arc
  de congé de `ρ/2`. La forme juste met les sommets à **±30°** sur l'arc, et —
  surtout — **l'écartement se calcule** (`pieces::onigiri_hex_echelle_mini`) au
  lieu de se régler à l'œil : la fonction renvoie l'échelle minimale sans
  recouvrement, pour les deux contraintes (faces et coins). L'angle des sommets
  redevient alors un paramètre de style libre, puisque l'échelle s'y adapte.
- `RatelierCargo` — une **rangée** de fret : couronne de nacelles autour de
  l'axe, écoutilles axiales aux deux bouts pour chaîner les rangées. Deux
  dispositions, dans `grappe_cargo()` (partagée par le dessin **et** par le
  calcul d'encombrement, pour qu'ils ne divergent pas) : **triforce** à 3
  (même orientation, pointe contre pointe, creux triangulaire central où passe
  l'épine) et **couronne** à ≥ 4 (coin vers l'axe, côtés plats face à face).
  Les nacelles sont dessinées **par le râtelier**, comme les ailettes d'un
  `RadiateurMega` : identiques et nombreuses, une pièce chacune ferait exploser
  le compte pour rien.

*Deux invariants géométriques appris à l'écran, désormais testés* : une
collerette posée **à ras** d'un bout partage son plan avec lui et clignote
(z-fighting) — elle doit déborder *et* s'enfoncer ; et deux triangles à coins
**congés** ne se touchent pas pointe contre pointe comme des coins vifs (ce
sont des triangles nus gonflés du rayon de congé ρ), il faut écarter leurs
centres de `r(1 − f + 2f/√3)` sous peine de recouvrement.

Nouvelles **vues Briques** (touche **D**) : réservoir carburant, moteur
antimatière (tuyère + réacteur assemblés), coiffes de modules (3 formes).
Assemblages complets : `preset_isv` et `preset_isv_moteur` portent le bloc
propulsion ISV via `poser_bloc_moteur(.., propulseur)`. **Prochain chantier :
stockages de carburant.**

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
±Z). `generateur.rs` porte aussi tous les **presets** (`preset_iss`,
`preset_mir`, `preset_tiangong`, `preset_comsat`, `preset_sonde`,
`preset_anneau`, `preset_isv`, `preset_isv_moteur`) et les **vitrines de
briques** (`demo_*`) : les anciens fichiers un-vaisseau-par-fichier
(`iss.rs`, `tiangong.rs`, `comsat.rs`, `sonde.rs`, `voyager.rs`, `telescope.rs`,
`gps.rs`, `cubesat.rs`, `navette.rs`, `atterrisseur.rs`, `futur.rs`,
`station.rs`) ont été **supprimés** : plus aucune géométrie codée en dur en
dehors du vocabulaire composants/ports — chaque preset n'est plus qu'un
assemblage écrit avec les mêmes helpers que le générateur.

**Menu** (`ecran/accueil.rs`) : deux blocs de boutons, « Astres & galeries » et
« Stations & mégastructures ». Ce second bloc route vers quatre entrées qui
partagent la même vue (`ecran/station.rs`, enum `Categorie`) mais cyclent
chacune leurs **propres** items à la touche **D** : `Briques` (27 vitrines de
composants), `PetitesStations` (ISS / Mir / Tiangong / comsat / sonde),
`Generateur` (1 item, réglable par G/S/1-4/O), `Megastructures` (anneau / ISV
complet en **épine carrée** / le même en **épine hexagonale** / ISV radiateur+bloc
moteur). Deux **boutons** s'ajoutent sur les vues
qui montrent une section d'équipage (la brique dédiée et l'ISV complet) : mise en
**rotation** et **repli**/déploiement ; ailleurs ils restent visibles mais grisés,
pour que la fonction se voie sans laisser croire qu'elle agit. Touches debug/rendu
communes : **P** gizmos de ports, **N** numéros de pièce (index d'assemblage
projeté à l'écran — sert à désigner une pièce à corriger), **X** filtre pixel,
**M** bascule maillage cuit / rendu immédiat.

**Boussole d'axes** (`ui::boussole_axes`, coin bas-droit de la vue station) : le
repère XYZ du monde projeté en 2D, X rouge / Y vert / Z bleu selon la convention
des logiciels 3D. Projection **orthographique** — un gizmo d'orientation montre
des directions, pas des positions, et une perspective y fausserait la lecture des
angles. Les axes qui **fuient** la caméra sont atténués : c'est ce qui distingue
`+X` de `−X`, dont les projections se superposent. La base vient de
`camera::base_orbite`, la même que celle de l'éclairage, si bien que la boussole ne
peut pas se désynchroniser de la vue. Les boutons de la section d'équipage sont
décalés vers le haut de `BOUSSOLE_BOITE` pour ne pas se disputer le coin.

**Sortie de géométrie abstraite** (`src/vaisseau/peintre.rs` +
`src/vaisseau/maillage.rs`, nouveau) : toutes les briques de dessin
(`pieces.rs`, `composant.rs`) sont **génériques sur un trait `Peintre`**
(`fn treillis<P: Peintre>(p: &mut P, ..)`,
`Composant::dessiner<P: Peintre>(&self, p: &mut P)`) plutôt que d'appeler
macroquad en dur — un seul jeu de fonctions de forme, deux sorties :
`peintre::Immediat` (appels macroquad directs, comportement historique) et
`maillage::Batisseur` (accumulation sommets/indices). La vue station **cuit**
la station courante une fois (`MaillageStation::cuire`, touche **M** pour
comparer aux deux rendus) : un treillis ISS complet, dessiné en immédiat,
pousse près d'un millier de `push_model_matrix` (un draw call par primitive) ;
cuit, il tient en quelques `draw_mesh` (découpés en lots de
≤ 1600 sommets / 2400 indices, la limite du batcher macroquad).
`Station::cout_total()` (somme de `Composant::cout()`) donne la mesure de
complexité affichée à l'écran à côté du nombre de lots/sommets/triangles.

**Pièces mobiles : deux régimes, pas un.** Un maillage cuit est figé, ce qui
oblige à trancher pour chaque mouvement — et la réponse n'est pas la même selon
qu'il *déplace un solide* ou qu'il *déforme la géométrie* :

- **Mouvement rigide** (la section d'équipage qui tourne) → une **matrice modèle**
  poussée au moment du rendu. Le maillage ne bouge pas d'un sommet ; on peut
  tourner à chaque frame pour le prix d'une `Mat4`. Il faut en revanche que ce qui
  tourne soit **cuit séparément** de ce qui reste fixe, sinon la matrice emporte
  tout le vaisseau : d'où `preset_isv_fixe()` + `preset_isv_equipage()` en deux
  maillages, la vue ne poussant la rotation que sur le second.
- **Mouvement articulé** (le repli, qui plie des bras autour de charnières) → il
  n'y a pas de matrice unique qui le décrive, et la géométrie doit être
  **reconstruite**. Le coût est alors réel, donc on ne reconstruit **que la
  partie concernée** (`recuire_repli()`), jamais le vaisseau entier.

Corollaire pour la suite : découper un modèle en maillages suit les **axes de
liberté**, pas les familles de composants. Chaque sous-ensemble qui bougera d'un
bloc mérite son propre maillage ; le reste gagne à être cuit ensemble.

**Couche rendu** (`src/vaisseau/eclairage.rs` + `src/shaders/station.*.glsl`) :
un material unique ombre **toutes** les primitives macroquad via des normales de
facette calculées en **dérivées écran** (lumière clé + contre-jour + spéculaire
alu) — aucune normale requise dans les sommets, aucune géométrie modifiée ;
usage `eclairage::avec(cam_pos, || …)`. Le **filtre pixel** (`src/ecran/pixel.rs`,
`FiltrePixel`) rend la station dans une cible basse résolution en Nearest, avec
le fond stellaire net dessous. Habillage de coque (`composant.rs`) : coutures de
panneaux, bandes **MLI** sur le module doré, **bagues d'accostage** alu au bout
des collerettes (modules, nœuds, adaptateurs).

`preset_iss()` est une **reproduction ISS assemblée à la main** — inventaire
réel, fusions, chevauchements et audit dans le document dédié
[`suivi/stations.md`](../suivi/stations.md) Partie B. Topologie : poutre déportée au **zénith
par un boom Z1** (elle ne traverse plus le cœur), segment **US** aft
(Destiny→Harmony + Columbus/Kibō + nez PMA/IDA) avec grappe Tranquility (Cupola
nadir, BEAM, PMM), segment **russe** fore (Zarya→Zvezda + arrays + nœud MRM),
Sas Quest et radiateurs nadir.

`preset_mir()` reproduit **Mir en configuration finale**, d'après le schéma
d'assemblage et les cotes publiées (fonction `cote(m)` : les longueurs du code
sont les cotes réelles à l'échelle). Cœur DOS-7 (13,13 m) ; **nœud sphérique à
5 ports** à l'avant (1 axial + **4 radiaux** → la « croix ») ; **Kvant-1 à
l'arrière du cœur** (et non au nœud), tonneau court de 5,80 m ; module
d'amarrage navette (Ø 2,20 m) au bout de Kristall ; Soyouz-TM et Progress-M
amarrés aux deux ports axiaux.

Trois détails d'ailes solaires, qui portent beaucoup de la silhouette :
**Kristall et Priroda n'en ont aucune** en configuration finale (celles de
Kristall ont été transférées sur Kvant-1, Priroda tournait sur batteries), et
**Spektr en porte quatre** (deux paires croisées). Les ailes font ~10,6 × 3,9 m,
soit presque la longueur d'un module.

`preset_tiangong()` reproduit la **configuration en T** : cœur Tianhe, nœud avant
dont deux ports **radiaux opposés** portent Wentian et Mengtian, plus un port
axial et un port **nadir** (Shenzhou) ; cargo Tianzhou à l'arrière. Les ailes des
laboratoires sont volontairement bien plus grandes que celles du cœur (27 m
contre 12,6 m en vrai) — c'est une part importante de la silhouette.

**Docking par direction monde** : les nœuds **basculent** (demi-tour) à
l'accouplement — viser « le port −Z » ne pointe donc *pas* vers −Z monde, ce qui
repliait les chaînes sur elles-mêmes (Destiny et le module avant au même point,
Harmony sur le FGB). Le preset docke via `porter_vers(hote, dir_monde, …)`, qui
choisit le port dont l'**avant monde** vise la direction voulue. À réutiliser
dans le générateur.

**Manques du générateur pour atteindre la fidélité ISS** (constatés en comparant
au preset) :
- **topologie décalée** : poutre transverse portée par un **boom** (type Z1) au
  lieu d'une épine — fait dans le preset, pas encore dans le générateur ;
- **attache mi-poutre** : le treillis n'a de ports qu'aux bouts ; impossible d'y
  accrocher un module en son *milieu* ;
- **ports `Surface` ±Y de poutre** : ils n'existent que sur ±X_local, d'où des
  radiateurs de poutre fore/aft au lieu de nadir ;
- **zonage des appendices** : arrays aux extrémités, radiateurs inboard — règle à
  porter dans le générateur ;
- **docking par direction monde** (`porter_vers`) et **symétrie bâbord/tribord
  structurée** (le générateur reste stochastique).

Reste (détaillé aux §4–5) : combler ces manques ; composants optionnels
(appendices dockés Soyouz/cargo, styles de nœud/treillis) ; atelier à deux axes ;
styles/palettes.

---

### 1. Rappel : ce qui existe déjà

- Briques factorisées dans `src/vaisseau/pieces.rs` : `treillis`, `module`,
  `pale_solaire`, `paire_ailes`, `radiateur` (paramétrées, orientables).
- Primitives orientées dans `mod.rs` : `cylindre`, `cone`, `parabole`, `voile`,
  `panneau`.
- Atelier de visualisation : catégorie `Briques` de `ecran/station.rs` (menu
  accueil « BRIQUES - COMPOSANTS »), touche **D** pour changer de brique.
  (Les anciens `ecran/briques.rs`/`ecran/vaisseaux.rs` ne sont plus référencés
  par `ecran/mod.rs` — supplantés par cette vue unifiée.)
- ISS de référence reconstruite à partir de ces briques (calibrage).

Il manque : **plusieurs variantes par type**, et un moyen de les parcourir.

---

### 2. Modèle de variante (à implémenter en premier)

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

### 3. Points d'accroche (le cœur de l'accouplement)

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

##### 3.1 Ce qu'est un port

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

##### 3.2 Port de montage vs ports hôtes

Un composant a **un port de « montage »** (celui par lequel il se rattache à son
parent) et **0..n ports « hôtes »** libres (où viennent ses enfants). En
pratique c'est la même liste : on marque simplement le port consommé comme
occupé. Un composant peut donc être relié à **1 ou n structures** — exactement
ce qu'on veut.

##### 3.3 Compatibilité

On n'accouple que des ports de **genres compatibles** (table de compatibilité) :
un appendice (panneau/radiateur/antenne) se monte sur `Surface`, un module sur
`ModuleAxial`/`Radial`, etc.
Le générateur ne pioche que dans les ports libres compatibles.

##### 3.4 Calcul d'accouplement

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

##### 3.5 Deux familles d'hôtes

- **Ports discrets** : hatch de module, faces d'un nœud → liste finie. *(à faire
  en premier)*
- **Rails continus** : bord d'une poutre où l'on peut monter panneaux et
  radiateurs à **n'importe quel décalage** → un « rail » qui génère des ports à
  la demande. *(étape ultérieure)*

##### 3.6 Symétrie

Marquer les ports en **paires miroir** (ex. +Y / −Y d'un nœud, gauche/droite
d'une poutre) : le générateur y place des enfants **appariés**, indispensable au
look ISS. Un `groupe_symetrie: Option<u8>` sur le port suffit.

##### 3.7 Coût / garde-fous

- Plus de machinerie en amont qu'un placement codé en dur — mais c'est justement
  ce qui débloque ramification + variété (le but).
- V1 volontairement minimale : `Repere` + `genre` + `occupe`. Le reste
  (diamètre, symétrie, rails) s'ajoute ensuite.
- Les ports ne suffisent pas contre les **chevauchements à distance** (deux
  enfants voisins qui se croisent) : garder une vérification de boîtes
  englobantes en filet de sécurité (§7).

---

### 4. Le lot de variantes visé (cible : 3–5 par type)

##### 3.1 Structure (treillis / poutre)

- [x] `Carre` — 4 longerons + cadres/diagonales (barres en cylindres, du volume).
- [x] `Triangulaire` — 3 longerons (plus léger, look « sonde »).
- [ ] `Caisson` — tube/box plein, faces pleines.
- [ ] `AvecRails` — poutre + rail du transporteur mobile (détail ISS).
- Axes : longueur, gabarit (via `profil`) ; ports hôtes `Surface` répartis.

##### 3.2 Habitat (module)

- [x] `Standard` — blanc simple.
- [x] `Dore` — teinte or (segment russe).
- [x] `Hublots` — rangée de hublots + mains courantes EVA.
- [x] `Labo` — grande fenêtre + rack externe (type Destiny).
- [x] `Gonflable` — profil bombé (type BEAM).
- [x] `Coupole` — coupole vitrée à un bout (type Cupola).
- Implémenté comme champ `variante` de `ModuleAxial` (couleur + `details()`).
  Axes : `profil`, `longueur`.

##### 3.3 Nœud d'amarrage

- [x] `Spherique` — multi-ports (type Mir) : dispositions `Quatre`, `Six`, `T`,
  `Tetra` ; sphère gonflée, bras cylindriques ancrés + collerette par sortie.
- [ ] `Cubique` — nœud US (Unity/Harmony).
- [ ] `AvecCupola` — coupole facettée orientable.
- Axes : disposition/nombre de ports, profil.

##### 3.4 Panneau solaire

- [x] `RigideUS` — ambre, 2 lés rigides.
- [x] `RusseBleu` — bleu, plus court.
- [x] `RollOut` — iROSA, bande étroite plus foncée.
- [x] `Futuriste` — cyan, plus large.
- [x] `Hexagonal` — tuiles hexagonales espacées (maillage).
- Axes : longueur, largeur ; couleur/proportions portées par la variante (`style()`).
- Reste : la **paire d'ailes** en données (symétrie miroir sur un port hôte
  `Surface` — le treillis en expose déjà).

##### 3.5 Radiateur

- [x] `PanneauSimple` — panneau plat rainuré (body-mounted).
- [x] `AccordeonATCS` — corrugation en zigzag (bank ISS).
- [x] `PivotantTRRJ` — gros joint rotatif visible.
- [x] `Caloducs` — tubes cuivre apparents (loop heat pipe).
- [x] `Deroulable` — gros tambour + bande étroite dorée (roll-out).
- [x] `Corps` — large et court, sombre (body-mounted).
- [x] `Gouttelettes` — **exotique** : rideau de gouttelettes (LDR).
- [x] `Voile` — grande voile radiante à l'échelle vaisseau (quille + nervures,
  teinte chaude), distincte de `RadiateurMega` (celle-ci reste montée en port
  `Surface` standard, pas un composant autonome).
- Chaque variante porte sa couleur/proportions/silhouette. Reste : ports hôtes
  `Surface` (le treillis en expose déjà — panneau/radiateur/antenne s'y clipsent).

##### 3.6 Antenne / Parabole

- [x] `ParaboleGG` — grand gain, orientée +Z.
- [x] `ParaboleOffset` — parabole à alimentation décalée, inclinée.
- [x] `Fouet` — fouets omni croisés.
- [x] `ReseauPhase` — plaque plate quadrillée (réseau phasé).
- [x] `Cornets` — grappe de cornets (horns).
- [x] `Helice` — antenne hélicoïdale.
- Monté par un port `Surface` ; axe : `taille`.

##### 3.7 Appendices (vaisseaux amarrés)

- [ ] `Soyouz` — vert, petits panneaux.
- [ ] `CargoUS` — Dragon/Cygnus.
- Axes : taille, couleur.

---

### 5. Étapes claires

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
*Réalisé autrement* : pas d'atelier à deux axes séparé — `ecran/briques.rs` n'est
plus référencé (voir §0). La catégorie `Briques` de `ecran/station.rs` couvre le
même besoin en une seule liste de 19 vitrines cyclée par **D**, une par type/
variante notable, avec les gizmos de ports (**P**) déjà en commun avec le reste
de la vue station.

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

### 6. Grammaire d'assemblage (pour l'étape 7)

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

### 7. Garde-fous esthétiques (low-poly réaliste)

- Cellules solaires : couture centrale + nervures ; ambre US / bleu russe.
- Treillis réellement ajouré (longerons + diagonales).
- Modules : anneaux de jonction sombres pour lire les raccords.
- Radiateurs franchement blancs, distincts des panneaux.
- Jamais de sphère pour un élément orienté (paraboles = cônes orientés).
- Variété *dans le style* : deux stations d'un même style doivent différer par
  le choix et le placement des variantes, pas par des couleurs incohérentes.

---

### Sources

- [Integrated Truss Structure — NASA](https://www.nasa.gov/international-space-station/integrated-truss-structure/)
- [Integrated Truss Structure — Wikipedia](https://en.wikipedia.org/wiki/Integrated_Truss_Structure)
- [Electrical system of the International Space Station — Wikipedia](https://en.wikipedia.org/wiki/Electrical_system_of_the_International_Space_Station)

---

## Partie B — Fondations transverses (état, budget, unités, symétrie)


Compagnon de la Partie A ci-dessus. Ce document
ne réécrit pas le modèle de ports (déjà validé) : il pose les **3 garde-fous
transverses** demandés — modèle d'état (1), plafond de coût flottant (7),
standard d'unités (8) — plus la synthèse de la phase de recherche. Fil rouge :
**KISS et coût de rendu maîtrisé**.

---

### 0. Ce que dit la recherche

**Kerbal Space Program**
- Chaque pièce déclare des *attach nodes* nommés (`node_stack_top`,
  `node_stack_bottom`, `node_attach`) = **position + orientation**. C'est
  exactement notre `Port` (§3 du doc existant). La recherche **valide** notre
  choix, rien à changer.
- Symétrie = **miroir** + **radiale à multiplicateur N** ; elle se propage dans
  l'arbre. Assemblage en **arbre, sans boucle**.
- Diamètres **normalisés** en famille discrète (0,625 / 1,25 / 2,5 / 3,75 / 5 m).
  Les pièces ne s'emboîtent proprement que par **profils compatibles**. → fonde
  le point 8.
- Le **coût = nombre de pièces** (physique par pièce, chaque frame, mono-thread →
  ça rame au-delà de ~200 pièces). Chez nous le coût n'est pas la physique mais
  les **draw calls / primitives par frame** (macroquad en mode immédiat). **Même
  leçon** : plafonner le poids de pièces. → fonde le point 7.

**Générateurs procéduraux (grammaires de formes)**
- Axiome + règles de réécriture + opérations de symétrie/répétition. C'est déjà
  notre grammaire (§6 du doc existant).

**Conclusion** : l'architecture est la bonne. Ce qui manque, ce sont les trois
fondations ci-dessous.

---

### 1. Modèle d'état (point 1) — le plus léger possible

Problème : ne **jamais** dessiner une station à moitié construite (rendu tronqué
ou incohérent).

Principe KISS : **séparer génération et rendu**. On ne dessine QUE des stations
immuables et terminées. On n'atteint jamais un état partiel *observable*, car :
- la génération écrit dans un `Vec<Piece>` **local** ;
- on ne publie la station (move dans le slot de rendu) **qu'une fois complète**.

L'état se réduit à :

```rust
enum EtatStation {
    Vide,            // rien à dessiner
    Prete(Station),  // immuable — la seule qu'on dessine
}
```

Le rendu fait un `match` ; seul `Prete` dessine. **Coût par frame : zéro** (un
enum, aucune machinerie, aucune vérification runtime).

Invariant tenu *par construction* (pas par surveillance) :
`Station::generer(seed, params) -> Station` renvoie l'objet **fini**,
transformées déjà cuites, et `Station` **n'est jamais muté** après publication.
Immuabilité = cohérence garantie.

**Croissance future uniquement si besoin** : si un jour la génération passe en
tâche de fond (thread), ajouter une 3e variante `Generation { seed }` qui **ne se
dessine pas**. Tant que la génération reste synchrone et sous la milliseconde,
elle est atomique → `Vide | Prete` suffisent. Ne pas sur-concevoir maintenant.

---

### 2. Standard d'unités (point 8)

But : chaque composant dimensionné dans une **unité commune**, proportions
cohérentes, emboîtement garanti — sans réglage au cas par cas.

##### 2.1 Unité de base
Une seule constante :

```rust
const U: f32 = 1.0; // rayon du module « standard » = 1 U
```

**Toute** dimension s'écrit `n * U`. Changer `U` rescale toute station d'un coup.

##### 2.2 Profils (diamètres discrets, façon KSP)

```rust
enum Profil { P0, P1, P2, P3 } // rayons : 0.5U, 1U, 2U, 3U
```

- **P0** (0,5 U) : sondes, cubesats, appendices.
- **P1** (1 U) : module habitat standard.
- **P2** (2 U) : gros module / nœud.
- **P3** (3 U) : cœur / épine dorsale.

Deux ports ne s'accouplent que s'ils ont le **même profil** (`port.profil`
précise le `diametre` du §3.1 existant). Compatibilité = **égalité d'enum** →
test trivial, jonctions toujours propres, jamais de module de 4 m clipsé sur un
port de 0,5 m.

##### 2.3 Proportions dérivées
Pour rester réaliste automatiquement, dériver les longueurs du diamètre plutôt
que de les fixer en absolu :
- longueur d'un module = **1,5 à 4 × diamètre** ;
- panneau solaire : largeur ≈ diamètre du module porteur, longueur = k × largeur ;
- treillis : demi-section = 0,5 à 1 × diamètre.

**Règle** : un composant ne fixe jamais une taille absolue arbitraire ; il
l'exprime en `U` ou relativement au **profil de son port**. Les proportions
restent homogènes sans effort.

---

### 3. Plafond de coût flottant (point 7)

But : borner la complexité d'une station pour protéger le budget de rendu, et
servir au cadrage caméra / anti-collision.

##### 3.1 Budget de coût (le principal)
Chaque variante déclare un **coût de rendu approximatif** (poids ≈ nombre de
primitives / lignes dessinées) :

```rust
fn cout(&self) -> f32
```

La génération part d'un budget et le dépense :

```rust
struct Budget { restant: f32 }
// à chaque pièce ajoutée : restant -= piece.cout();
// on arrête d'ajouter dès que restant <= 0.
```

**Pourquoi un float et pas un compteur de pièces** : les pièces n'ont pas le même
coût (un segment de treillis nu ≪ une aile solaire nervurée). Le float pondère
correctement → le plafond limite le **coût réel de rendu**, pas un nombre
trompeur. C'est notre équivalent, pondéré, de la limite ~200 pièces de KSP.

Le budget par défaut se calibre pour tenir le framerate sur la station la plus
lourde ; il est exposé dans la grammaire (le paramètre `taille` du §6 existant se
mappe sur un budget).

##### 3.2 Rayon maximal (le secondaire)

```rust
struct Station { pieces: Vec<Piece>, rayon: f32 } // sphère englobante
```

Calculé **une fois** à la génération (distance max pièce ↔ centre). Sert à :
cadrer la caméra (comme `demi_dim` pour les maquettes actuelles), rejeter un
enfant qui dépasserait `rayon_max`, alimenter le filet anti-collision (§7 du doc
existant).

---

### 4. Symétrie (point 4, validé)

Reprendre KSP : **deux opérations seulement**.

```rust
enum Symetrie { Miroir, Radiale(u8) } // Radiale(n) = n copies autour de l'axe
```

Portée par les groupes de ports (`groupe_symetrie`, §3.6 existant). Le générateur
place les enfants d'un groupe symétrique **en une passe**. Indispensable au look
ISS (miroir gauche/droite des ailes) et aux nœuds type Mir (radiale).

---

### 5. Ordre d'implémentation

Ces fondations s'insèrent **avant** les étapes 6–7 du doc existant :

1. `U` + `Profil` + proportions (§2) — trivial, débloque tout le reste.
2. `EtatStation = Vide | Prete` + `Station` immuable (§1).
3. `cout()` par variante + `Budget` + `rayon` (§3).
4. `Symetrie` (§4), au moment du générateur (étape 7 du doc existant).

Reconstruire ISS / Mir **« en données »** (étape 6, point 5) : **plus tard**, une
fois le lot de variantes rempli.

---

### Sources
- [KSP — attachment nodes & symétrie (General Discussions)](https://steamcommunity.com/app/220200/discussions/0/1743358239843828618/)
- [KSP — tailles de pièces / form factors (Steam)](https://steamcommunity.com/app/220200/discussions/0/364042703862870924/)
- [KSP 2 — Size categories (modding wiki)](https://modding.kerbal.wiki/Size_Category)
- [KSP — coût CPU par nombre de pièces (forum)](https://forum.kerbalspaceprogram.com/topic/163317-how-to-improve-fps-with-high-part-count-crafts/)
- [SpaceshipGenerator — grammaire d'extrusion + symétrie (GitHub)](https://github.com/a1studmuffin/SpaceshipGenerator)

---

## Partie C — Raccordement ports ↔ assemblage (Étape 2)


Troisième partie de ce document, à lire après la Partie A
(plan directeur, modèle de ports) et la Partie B
(état, budget, unités, symétrie). Il ne réexplique pas ces briques : il pose **le
chaînon qui les relie** — le trait/enum `Composant`, l'évolution de `Piece`, et la
« cuisson » des transformées. C'est l'**Étape 2** du plan directeur.

Fil rouge inchangé : **KISS**, coût de rendu maîtrisé, invariants tenus *par
construction* plutôt que surveillés.

---

### 0. État de départ (ce qui existe déjà)

| Brique | Fichier | État |
|---|---|---|
| Ports : `Repere`, `Port`, `GenrePort`, `accoupler` | `src/vaisseau/port.rs` | ✅ 13 tests (5 cas limites) |
| Unités : `U`, `Profil` P0–P3, `proportion` | `src/vaisseau/unites.rs` | ✅ |
| État & immuabilité : `EtatStation`, `Assembleur`, `Station` | `src/vaisseau/assemblage.rs` | ✅ |
| Budget & rayon englobant | `src/vaisseau/assemblage.rs` | ✅ |
| Symétrie : `Miroir`, `Radiale(n)` → `Vec<Mat4>` | `src/vaisseau/symetrie.rs` | ✅ |
| Briques de dessin (treillis, module, ailes, radiateur…) | `src/vaisseau/pieces.rs` | ✅ (fonctions libres, **pas** de ports) |

**Manque, dans l'ordre du raccordement :** le `Composant`, une `Piece` qui porte une
transformée cuite, la liaison `accoupler`/`Symetrie` → `Piece`, et un `cout()` par
composant. Tout le reste (variantes riches, styles, générateur, atelier 2 axes) est
**hors scope** de ce doc → Étapes 3+.

---

### 1. Décisions actées

Quatre forks tranchés avant d'écrire une ligne :

1. **Dispatch = enum `Composant` + `match`.** Pas de `Box<dyn>`. KISS, zéro
   allocation, monomorphisé, cohérent avec `TypeEngin` déjà en place. Une seule
   fonction de dessin et une seule d'exposition de ports par composant, qui
   `match` sur la variante.

2. **`Piece.transforme` est une `Mat4` cuite** (pas un `Repere`). Voir §2 pour
   l'architecture à deux couches et l'argument.

3. **Le miroir est natif** grâce à la couche `Mat4` : une réflexion (déterminant
   −1) ne rentre pas dans un `Quat`, donc on n'essaie pas. `symetrie` continue de
   renvoyer des `Mat4`, appliquées à la transformée cuite.

4. **Premier composant validé de bout en bout = le module axial.** Deux modules
   bout-à-bout par leurs ports axiaux : le cas minimal qui exerce
   `accoupler` + `Composant` + `Piece` **sans** symétrie ni variantes riches.
   (C'est le critère de validation de l'Étape 1 du plan directeur.) Le panneau
   solaire (avec sa paire miroir) vient juste après, pour exercer la symétrie.

##### 1 bis. Piège d'assemblage : ne pas chaîner par index de port

`accoupler` met les ports **face à face**, ce qui impose un **demi-tour** à
l'enfant. Conséquence : après montage, « le port −Z » d'un nœud ne pointe plus
vers −Z **monde**. Chaîner en réutilisant des index fixes (`port 1`, `port 0`…)
fait donc **replier la chaîne sur elle-même** — bug réellement rencontré sur
`preset_iss` (deux modules au même point, un segment retombant sur l'autre).

**Règle** : pour chaîner, viser une **direction monde**, pas un index. Cf.
`porter_vers(hote, dir_monde, enfant, montage)` dans `generateur.rs` : il
sélectionne le port dont l'**avant monde** maximise le produit scalaire avec la
direction voulue (aft/fore/nadir/zénith/bâbord/tribord). Même principe pour les
appendices avec `appendice_sur_module` (choix de la face par direction monde).

---

### 2. Architecture à deux couches (le cœur)

L'assemblage et le rendu ne parlent pas le même langage géométrique, et c'est
**voulu** :

**Couche construction — `Repere` / `Quat`.**
C'est là que vivent les ports. `accoupler(hote, montage)` et `Repere::compose`
travaillent en rotation pure : composition exacte, aucune dérive au-delà de
l'arrondi f32, déjà testé. Le chaînage d'un arbre de composants se fait
entièrement ici. **Le miroir n'y est jamais appliqué.**

**Couche cuite — `Mat4`.**
Une fois la place d'un composant résolue en `Repere` monde, on la **cuit** en
`Mat4` (`repere.to_mat4()`) et on la range dans une `Piece`. Le rendu fait
`push_model_matrix(piece.transforme)` puis `composant.dessiner()`.

##### Pourquoi `Mat4` cuite et pas `Repere` dans `Piece`

Le seul argument qui trancherait vers `Repere` serait « un seul type partout ».
Mais le **miroir le casse** : une symétrie Miroir est une réflexion de
déterminant −1, impossible à encoder dans un `Quat`. Avec `Repere` dans `Piece`,
une pièce miroir devient irreprésentable — il faudrait un `miroir: bool` qui, en
plus, ne suffit pas (un miroir à travers un plan quelconque exige le plan), et ce
cas spécial fuit dans le rendu, le rayon englobant et l'anti-collision.

Une `Mat4` encode réflexion + rotation + translation dans un type uniforme. En
prime : `symetrie::transformations` renvoie **déjà** des `Mat4`, et le renderer
veut une `Mat4`. Une copie symétrique n'est alors qu'un produit :

```
transforme_copie_k = symetrie_k * repere_monde.to_mat4()
```

Coût : 64 o/pièce contre 28 — négligeable pour quelques centaines de pièces. On
perd le re-chaînage depuis une pièce cuite, mais on n'en a pas besoin : **le
chaînage est terminé avant la cuisson**. (KSP fait pareil : les pièces miroir
sont de vraies copies chirales.)

---

### 3. Cible de types

Esquisse (les noms/champs se figent en codant ; `params`/`style` restent **hors
scope**, ajoutés aux Étapes 4–5) :

```rust
// src/vaisseau/composant.rs (nouveau)

/// Un composant concret : ce qui sait exposer ses ports et se dessiner.
/// Enum fermé, match — pas de trait objet.
pub enum Composant {
    ModuleAxial { profil: Profil, longueur: f32 },
    // PanneauSolaire { .. }, Treillis { .. }, Noeud { .. } … à venir
}

impl Composant {
    /// Ports dans le repère LOCAL du composant (montage + hôtes libres).
    pub fn ports(&self) -> Vec<Port>;
    /// Dessine dans le repère local (transformée déjà poussée par l'appelant).
    pub fn dessiner(&self);
    /// Coût de rendu ≈ nb de primitives/lignes (pondère le Budget, §3.1 fondations).
    pub fn cout(&self) -> f32;
    /// Rayon englobant local (pour la sphère de Station, remplace Piece.profil).
    pub fn rayon_local(&self) -> f32;
}
```

> **Mise à jour (implémentée depuis) :** `dessiner` a fini **générique sur un
> trait `Peintre`** plutôt que sur une signature figée —
> `pub fn dessiner<P: Peintre>(&self, p: &mut P)` — pour pouvoir, sans dupliquer
> la géométrie, soit dessiner immédiatement (`peintre::Immediat`, comportement
> historique) soit accumuler dans un maillage cuit (`maillage::Batisseur`). Le
> raisonnement et les deux implémentations sont dans `src/vaisseau/peintre.rs`
> et `src/vaisseau/maillage.rs` (voir la Partie A §0). Ça ne
> change rien à l'esquisse ci-dessus côté ports/coût/rayon, seulement à la
> forme de `dessiner`.

`Piece` évolue de `{ position, profil, cout }` vers :

```rust
pub struct Piece {
    pub transforme: Mat4,      // cuite (couche Mat4)
    pub composant: Composant,  // porte cout() et rayon_local()
}
```

- `Station::depuis_pieces` calcule le rayon via
  `translation(transforme).length() + composant.rayon_local()` (au lieu de
  `position.length() + profil.rayon()`).
- `Budget` consomme `composant.cout()` au lieu d'un `f32` fourni à la main
  (le champ `cout` brut de `Piece` disparaît).

> **Note de compat :** cette évolution touche les tests existants de
> `assemblage.rs` (ils construisent des `Piece::new(pos, profil, cout)`). Ils
> seront réécrits en même temps — c'est attendu, pas une régression.

---

### 4. Sous-étapes ordonnées (chacune se valide seule)

> **État (2026-07-16) : Étape 2 close (2a→2f faits).** Raccordement complet
> validé : les composants s'assemblent par ports, se cuisent en `Mat4`, se
> dessinent (écran « STATION », bouton du menu). `cargo test` couvre `composant`,
> `assemblage` et `montage`. Premiers composants réels construits par-dessus :
> `ModuleAxial`, `Noeud` (4 dispositions) et `PanneauSolaire` (5 variantes) — voir
> la Partie A §0 pour la suite.
>
> **Affinage rendu (issu de la validation visuelle) :** `ModuleAxial` dessine son
> corps en **cylindre lisse** (pas via `pieces::module`, dont les anneaux évasés
> 1.06× créaient une large bande sombre au joint), gagne une **collerette de
> docking** (col étroit dépassant à chaque bout ; le port se pose à son extrémité
> → offset visible au joint), et des **embouts** qui coiffent chaque disque de
> bout en **chevauchant** le corps — donc aucune face coplanaire, ce qui supprime
> le z-fighting (cause du « halo » observé). Constantes de forme dans
> `composant.rs` (`COL_*`, `EMBOUT_*`).

**2a — Enum `Composant` minimal.** ✅ *fait*
`composant.rs` avec la seule variante `ModuleAxial` : `ports()` (deux ports
axiaux, avant opposés, sur ±Z aux deux bouts), `dessiner()` (réutilise
`pieces::module`), `cout()`, `rayon_local()`.
*Validation :* test que `ports()` renvoie 2 ports axiaux de profils cohérents et
de `haut` bien orienté.

**2b — `Piece` en `Mat4` + `Composant`.** ✅ *fait*
Faire évoluer `Piece`, adapter `Station::depuis_pieces` (rayon via
`rayon_local`), `Assembleur`, et réécrire les tests d'`assemblage.rs`.
*Validation :* les 19 tests d'assemblage repassent au vert, adaptés au nouveau
`Piece`.

**2c — Cuisson d'un accouplement.** ✅ *fait*
Fonction de glu : à partir du `Repere` monde d'un port hôte et du composant
enfant + l'indice de son port de montage, produire la `Piece` enfant
(`accoupler(...).to_mat4()`).
*Validation :* **deux modules bout-à-bout** (le cas de la décision 4) — poser A à
l'identité, cuire B sur le port axial libre de A, et vérifier en re-décodant les
ports monde que les hatches **coïncident** et sont **face-à-face**.

**2d — Symétrie cuite.** ✅ *fait*
Appliquer `symetrie::transformations` à la transformée cuite d'un composant pour
produire un groupe de `Piece`.
*Validation :* un groupe `Miroir` produit 2 pièces, transformées de déterminants
opposés (la réflexion est bien là) ; un `Radiale(4)` produit 4 pièces à 90°.

**2e — Branchement rendu.** ✅ *fait*
La vue (`ecran/…`) qui affiche une `Station`/`EtatStation` : `match doit_dessiner`,
puis pour chaque `Piece` `push_model_matrix(transforme)` → `composant.dessiner()`.
*Validation :* visuelle — deux modules bout-à-bout à l'écran, jonction propre.

**2f (option, utile au debug) — Visualisation des ports.** ✅ *fait*
`Station::dessiner_ports()` trace pour chaque port une bille + l'axe **avant**
(orange) et **haut** (vert) ; touche **P** dans la vue STATION, dans toutes ses
catégories (`ecran/station.rs`, enum `Categorie` — voir la Partie A
§0). Chaque catégorie cycle ses propres items à la touche **D**.

---

### 5. Hors scope (rappel — Étapes 3+)

- Modèle de **variante** riche (`params`, plusieurs formes par type) — Étape 2 du
  plan directeur au sens « lot de variantes », déclenchée après ce raccordement.
- **Palettes / styles** — Étape 5.
- **Atelier 2 axes** (type/variante) — Étape 3.
- **Générateur** `Station::generer(seed, params)` + grammaire — Étape 7.
- **Rails continus**, anti-collision par boîtes englobantes — Étape 8.

Ce doc s'arrête quand deux modules (puis une paire d'ailes miroir) s'assemblent
par ports, se cuisent en `Mat4`, et se dessinent — la chaîne complète validée sur
le cas minimal.

---

### 6. Limites & extensions futures (mégastructures)

Ce modèle vise les stations **type ISS/Mir** : un assemblage de petits modules
clipsés par ports, en **arbre sans boucle**. Les habitats rotatifs — **tore de
Stanford**, **cylindre d'O'Neill** — sont un **chantier séparé**, pas produit
automatiquement par ce générateur. Deux raisons de fond :

1. **Grande coque courbe ≠ assemblage de briques.** Un tore ou un long cylindre
   est fondamentalement **une seule primitive courbe**, pas une accrétion de
   modules. Nos briques (`pieces.rs`) ne savent pas dessiner une section de tore.

2. **Un anneau est une boucle ; l'assemblage est un arbre.** Un anneau se referme
   sur lui-même — le modèle acyclique ne garantit pas la fermeture. Le plus KISS
   est de dessiner l'anneau **en une primitive paramétrique**, pas de l'assembler
   par segments.

**Ce qui resterait réutilisable** le jour où on s'y attaque : toutes les
fondations (transformée `Mat4` — qui encode aussi la rotation d'habitat —,
budget, `EtatStation`, immuabilité), et surtout le **squelette moyeu + rayons**
(un nœud central + poutres en `Radiale(n)`) qui, lui, colle parfaitement au
modèle de ports. Seul l'anneau/cylindre est l'intrus.

**Ce qu'il faudrait ajouter** (hors de ce doc) : des variantes de `Composant`
**primitives paramétriques** — `Tore { rayon_majeur, rayon_mineur, … }`,
`CylindreOneill { longueur, rayon, … }` — dessinant leur coque courbe (maillage
généré) et exposant des ports pour le moyeu et les rayons ; et, si l'on veut un
anneau *assemblé*, un mécanisme de **fermeture de boucle**. Rien n'interdit ces
mégastructures ; elles demandent simplement ces primitives dédiées en plus.

---

## Partie D — Classes de stations : de l'ISS à l'O'Neill


Feuille de route d'architecture. But : garantir que **chaque brique compose vers
le haut**, de la station modulaire à la mégastructure, sans jamais repartir de
zéro. La règle est qu'une classe supérieure **réutilise** les briques des classes
inférieures et n'ajoute que ce qui lui est propre.

---

### Les couches (rien au-dessus ne réinvente ce qui est en dessous)

- **L0 — primitives** (`peintre`/`maillage`) : cylindre, cône, sphère, cube,
  panneau, treillis. Sortie abstraite → dessin immédiat *ou* maillage cuit.
- **L1 — composants** (vocabulaire à ports) : `ModuleAxial` (10 variantes,
  jusqu'au `GrandGonflable` et `Serre`), `Noeud`, `Treillis`, `Adaptateur`,
  `PanneauSolaire`, `Radiateur`, `Antenne`, `Caisson`, `ChargeUtile`,
  `Propulseur` (3 familles). **Briques classe C (ISV) :** `Charpente` (treillis
  courbe à section variable), `RadiateurMega` (aile en arête de poisson),
  `CharpenteHexa` (la même en section **hexagonale** : variante candidate, plus
  lisible sous filtre pixel car sa largeur apparente ne varie que de 1,15 contre
  1,41, et dont le pied (`PiedHexa`) est soit une **tour** hexagonale coaxiale qui
  prolonge le cône, soit un **pavillon** — une corolle qui continue de s'ouvrir
  jusqu'à un large hexagone ouvert, coiffé d'un **fût** droit qui portera la
  propulsion (monté sur l'ISV hexagonal). Voir
  `suivi/stations.md` §C.9 et §C.11),
  `Motrice`, `BlocMoteur`, `Reservoir`
  (cuve sphérique en cage tétraédrique),
  `Coiffe` (capuchon de nez, 3 formes : bombée / hexagonale à face plate /
  adaptateur d'amarrage) et le **bloc propulsion antimatière** en deux briques
  chaînées — `ReacteurAntimatiere` (cuve sombre + bobines EM + tuyauterie +
  pièges à antiprotons) puis `MoteurAntimatiere` (tuyère : buse + cage de deux
  cercles ouverts sur 4 tiges). S'y ajoutent la **charge utile ISV** —
  `NacelleCargo`/`RatelierCargo` et `ModuleHabitat` (fixe) à section **onigiri** —
  et la **section d'équipage rotative** : `ModuleEquipage` (fût *cylindrique*),
  `CollierRotatif` (tambour, `rayon` libre hors grille `Profil`) et `Charniere`
  (chape + axe + vérin). Enfin les **boucliers de tête**, en deux briques :
  `BouclierPetit` (hexagone régulier, face avant striée / face arrière nervurée)
  et `BouclierGrand` (le même hexagone **étiré**, **épaules remontées** (longs
  bords courts) et **rogné d'un méplat aux deux pointes** — huit sommets, donc —
  **rétréci de 20 % en largeur seule** (le rayon garde le moyeu, qui doit laisser
  passer le mât commun), miroir bleuté **uni** sur ses deux faces, portant huit
  rayons ancrés au moyeu et rien en travers). Enfin `BouclierThermique` — bardage
  d'**écailles imbriquées** sur l'épine, qui recouvrent vers l'avant pour que le
  rayonnement des tuyères ne rencontre jamais de tranche. Toutes deux ont un **moyeu percé et deux ports axiaux** : les quatre
  plaques s'enfilent sur un mât commun au lieu de se bouter l'une l'autre, et
  c'est l'**espacement** entre elles qui blinde, pas leur épaisseur.
- **L2 — assemblages typés** (helpers qui posent des groupes) : `poser_anneau`,
  `greffer_structure_puissance` (boom + poutre + arrays), `paire_ailes`,
  `vaisseau_amarre`, `sur_face`. *À venir* : `poser_epine`, `poser_rayons`,
  `voiles_radiateurs`, `coque_cylindre`.
- **L3 — classes de station** : les cibles ci-dessous, chacune un preset (fait
  main) puis, à terme, une famille du générateur.

---

### Les classes

| Classe | Échelle | Exemples | Construction dominante |
|---|---|---|---|
| **A — engins** | m | comsat, sonde | un bus + appendices |
| **B — stations modulaires** | 10–100 m | ISS, Mir, Tiangong, générateur | cœur + poutre + grappe |
| **C — grands vaisseaux & anneaux** | 100 m–2 km | **ISV**, station à anneau, **Elysium** | épine ou anneau + moyeu |
| **D — mégastructures** | 10–30 km | **cylindre de O'Neill** | coque monolithique en rotation |

La classe D est « au-dessus » : sa coque n'est **pas** faite de modules, mais
elle réutilise l'anneau (ceinture agricole), l'épine (axe) et les radiateurs.

---

### Ce dont chaque cible a besoin (réutilisé vs neuf)

##### ISV Venture Star — classe C (*inspiré*, pas copié)
Grand vaisseau à **épine**. Réutilise : nœuds, modules (grappe habitée),
`Propulseur` (nucléaire-électrique / VASIMR, en gros gabarit), `Adaptateur`,
`poser_anneau` (petit anneau d'habitation optionnel). **Neuf** : épine dorsale
longue (`Charpente`/`Treillis` P3), **voiles radiateurs** (`RadiateurMega`) et le
**bloc propulsion à antimatière** (`ReacteurAntimatiere` + `MoteurAntimatiere`).

**État (2026-07-23) — bloc propulsion ISV fait.** La `Charpente` courbe, les
`RadiateurMega`, le `BlocMoteur` avec sa rangée de trois Cœurs et le propulseur
à antimatière sont assemblés. `preset_isv_moteur` (« RADIATEUR + BLOC MOTEUR »)
et `preset_isv` (« CHARPENTE + RADIATEURS ») portent la **version complète** :
Cœur 1 & 2 coiffés d'une chape bombée, propulseur accroché au bout libre de
Cœur 3 (réacteur monté par sa tête, tuyère sous sa base, poussée vers
l'extérieur ; taille calée pour que le corps du réacteur ait le **même diamètre**
que Cœur 3 au raccord). Réservoirs sphériques de part et d'autre de l'hexagone.
Le tout via le helper `poser_bloc_moteur(.., propulseur: bool)`.

**Mise à jour 2026-07-29** : les stockages de carburant, annoncés ici comme
« prochain chantier », sont **faits** (4 `Reservoir` autour de l'hexagone).
L'état mesuré du preset et le vrai reste à faire — essentiellement **toute la
section charge utile** (cargo, habitat/cryo, modules d'équipage rotatifs,
navettes TAV) — sont dans [`suivi/stations.md`](../suivi/stations.md)
**Partie C**, avec l'anatomie du vaisseau réel et les sources.

##### Elysium — classe C (tore de Stanford, anneau **ouvert** ~1,8 km)
Réutilise **directement** `poser_anneau` mis à l'échelle + `poser_rayons`
(rayons moyeu→jante, en `Treillis`) + moyeu (`Noeud`). **Neuf** : habitat sur la
**jante intérieure** (variante d'orientation de `poser_anneau` : modules
tournés vers l'axe), et une bande de jante continue.

##### Cylindre de O'Neill — classe D (paire contra-rotative)
Réutilise : `poser_anneau` (ceinture agricole), épine (axe central),
radiateurs, capots. **Neuf** : **coque cylindrique rayée** — un grand cylindre à
**6 bandes longitudinales** (3 terres, 3 fenêtres), **capots d'extrémité**
coniques, et **miroirs externes** (grands `panneau` inclinés). C'est la seule
brique vraiment nouvelle de cette classe.

---

### Ordre des briques (ce qu'il reste à faire)

1. ✅ **Anneau** (`poser_anneau`) — topologie fermée, posée par cuisson
   géométrique. Sert au preset roue, à Elysium, et à la ceinture O'Neill.
2. ✅ **Épine dorsale** — `Charpente` (treillis courbe P3→P1, anneau hexagonal en
   pied) porte moteurs, radiateurs et réservoirs.
3. ✅ **Voiles radiateurs** — `RadiateurMega` (aile en arête de poisson) en
   enfilade. Signature de l'ISV.
4. ✅ **assembler l'ISV** (preset, classe C) — **structure validée le
   2026-07-30** : 168 de long, rayon 12,6, 43 pièces, coût 683. Reste les 2
   navettes TAV et une relecture des proportions. Détail ci-dessous.
   **Section propulsion complète**
   (`poser_bloc_moteur` + propulseur antimatière sur Cœur 3, chapes bombées sur
   Cœur 1/2, **réservoirs faits**), **fret posé** (3 rangées en triforce) et
   **habitat principal posé** (3 modules en couronne, boulonnés sur l'épine
   au-delà du fret). Ossature (épine + propulsion) **agrandie de 20 %** par une
   mise à l'échelle géométrique, la charge utile gardant sa taille et se
   recalant sur le nouveau gabarit — 2026-07-29 : 30 pièces, coût 520.
   **Boucliers de tête posés** au bout opposé aux moteurs (petite plaque + 3
   grandes sur un mât, raccord conique au sommet d'épine) : 43 pièces, coût 683,
   168 de long. **Reste : navettes TAV** — détail et ordre de travail en
   [`suivi/stations.md`](../suivi/stations.md) Partie C.
5. **Elysium** — anneau XL + rayons + moyeu + habitat de jante (surtout de la
   réutilisation).
6. **Coque cylindrique rayée** + capots + miroirs (nouveau composant).
7. → **assembler l'O'Neill** (preset, classe D).

À terme, le **générateur** choisit une classe selon un paramètre d'échelle et
enchaîne les mêmes helpers — c'est pour ça qu'on les écrit réutilisables plutôt
que de coder chaque cible en dur.

---

### Sources
- [Stanford torus — Wikipedia](https://en.wikipedia.org/wiki/Stanford_torus)
- [Stanford torus — Elysium Wiki](https://elysiumfilm.fandom.com/wiki/Stanford_torus)
- [O'Neill cylinder — Wikipedia](https://en.wikipedia.org/wiki/O%27Neill_cylinder)
- [O'Neill Cylinder Space Settlement — NSS](https://nss.org/o-neill-cylinder-space-settlement/)

---

## Partie E — Refonte du système de composants (vers un éditeur façon VAB)

> Document de conception (2026-07-29), mis à jour au fil de l'implémentation.
> Deux chantiers, prévus dans l'ordre E.2 puis E.3 mais **faits dans l'ordre
> inverse** (décision utilisateur) : ramener `composant.rs` à une taille
> gérable (§E.1–E.2, **toujours à faire**) et introduire un composant
> **composite** capable d'agréger plusieurs pièces en une seule brique
> réutilisable (§E.3, **fait**). L'éditeur d'assemblage interactif façon VAB
> (Vehicle Assembly Building de Kerbal Space Program) reste **hors scope** :
> §E.4 liste ce dont il aura besoin, pour que la refonte ne lui ferme aucune
> porte, mais sa construction est un chantier **ultérieur**, distinct.

### E.1 Constat : pourquoi `composant.rs` doit être découpé

`src/vaisseau/composant.rs` fait aujourd'hui **2800 lignes** pour un seul
fichier — ~19× l'objectif que le projet s'est fixé lui-même
(`suivi/bucketlist_globale.md` §7, « fichiers ≤ ~100-150 lignes »). Le
problème n'est pas la taille en soi, mais sa cause : **une seule fonction par
capacité, qui `match` sur les 19 variantes de `Composant`** (décision actée en
Partie C §1 — enum fermé, pas de `Box<dyn>`) :

| Fonction | Lignes | Ce qu'elle contient |
|---|---|---|
| `dessiner<P: Peintre>(&self, p: &mut P)` | **624** | la géométrie complète des 19 variantes, un `match` géant |
| `ports(&self) -> Vec<Port>` | **241** | idem pour la liste de ports |
| `cout(&self)` / `rayon_local(&self)` / `englobant_local(&self)` | (le reste) | idem, en plus petit — **5 fonctions au total**, pas 4 (`englobant_local` sert l'anti-collision, cf. Partie C §7) |

Conséquence directe : **ajouter ou corriger une seule variante touche un
fichier de 2800 lignes**, oblige à naviguer 5 `match` différents pour trouver
tous les endroits qui la concernent, et rend les diffs de revue illisibles
(une modification d'un radiateur produit un diff au milieu d'un fichier qui
parle aussi de réacteurs à antimatière). Le nombre de variantes n'a fait que
grandir (2 en Partie C §4 « État 2026-07-16 », 19 aujourd'hui) — la fonction
géante était un choix raisonnable à 2 variantes, plus du tout à 19.

**Audit de couverture de tests (préalable à toute extraction, 2026-07-29)** :
sur les 19 variantes, **9 n'avaient aucun test dédié** — exactement les
briques « classe C / ISV » ajoutées après coup (`Charpente`, `RadiateurMega`,
`Motrice`, `BlocMoteur`, `Reservoir`, `MoteurAntimatiere`, `Coiffe`,
`ReacteurAntimatiere`, `TreillisHexagone`). Neuf tests de fumée ajoutés
(ports : genre/nombre/profils ; `cout`/`rayon_local` figés à des valeurs
concrètes ; `dessiner()` exercé via `maillage::Batisseur` — pas besoin de
contexte GL) avant de commencer le découpage : **130 tests** au lieu de 121,
un verrou de non-régression sur les 9 variantes qui n'en avaient aucun.

**Ce qui ne change pas** : la décision Partie C §1 reste valide.
`Composant` reste un **enum fermé, `Copy`, dispatché par `match`** — pas de
trait objet, zéro allocation, monomorphisé. Le problème est la **granularité
des fichiers**, pas le modèle de dispatch.

### E.2 Découpage en modules par famille — ✅ fait (2026-07-29)

> **État : fait.** `composant.rs` (3 316 lignes) est devenu `composant/`,
> 15 fichiers, aucun au-dessus de 475 lignes ; les cinq dispatch sont passés
> de 1 087 lignes cumulées à 212 (un bras d'une ligne par variante). Le plan
> ci-dessous a été suivi tel quel, à trois familles près, ajoutées depuis sa
> rédaction : `cargo` (nacelle + râtelier), `habitat` et `antimatiere`.
> Mesures et pièges rencontrés : [`suivi/stations.md`](../suivi/stations.md),
> « Priorités immédiates » point 3.

Principe : **une variante (ou une petite famille de variantes proches) = un
fichier**, qui regroupe pour cette famille son enum de style/variante *et* ses
cinq comportements (`ports`, `dessiner`, `cout`, `rayon_local`,
`englobant_local`) — au lieu d'un enum de variante isolé quelque part et son
comportement dispersé dans 5 `match` à 1000 lignes d'écart. `composant/mod.rs`
ne garde que la définition de l'enum `Composant` et 5 `match` **d'une ligne
par bras**, qui délèguent :

```rust
// composant/mod.rs (esquisse)
impl Composant {
    pub fn ports(&self) -> Vec<Port> {
        match self {
            Composant::ModuleAxial { profil, variante, longueur } =>
                module_axial::ports(*profil, *variante, *longueur),
            Composant::Noeud { profil, sorties } => noeud::ports(*profil, *sorties),
            // … une ligne par variante
        }
    }
    // dessiner / cout / rayon_local / englobant_local : même forme
}
```

Regroupement en **familles** (pas 19 fichiers isolés — les briques déjà
apparentées dans la Partie D restent ensemble) :

| Fichier | Variantes regroupées | Pourquoi ensemble |
|---|---|---|
| `commun.rs` | — (constantes + helpers) | `COULEUR`/`SOMBRE`/`BAGUE`/`TRAIT_FIN`, `COL_*`/`EMBOUT_*` (collerette de docking, partagée par module/nœud/adaptateur/coiffe), `faces_principales`, `dessiner_moteur_seul` |
| `module_axial.rs` | `ModuleAxial` + `VarianteModule` (10) | déjà un tout cohérent |
| `noeud.rs` | `Noeud` + `Sorties` + `faces_noeud` | déjà un tout cohérent |
| `panneau_solaire.rs` | `PanneauSolaire` + `VariantePanneau` (5) | déjà un tout cohérent |
| `treillis.rs` | `Treillis` + `TreillisHexagone` + `Charpente` + `StyleTreillis` | même famille « ossature » (Partie D L1/L1-C) |
| `radiateur.rs` | `Radiateur` + `RadiateurMega` + `VarianteRadiateur` (8) | grand radiateur = même brique à l'échelle vaisseau |
| `antenne.rs` | `Antenne` + `VarianteAntenne` (6) | déjà un tout cohérent |
| `adaptateur.rs` | `Adaptateur` + `Coiffe` + `VarianteCoiffe` | même rôle : pièce de raccord/embout en bout de module |
| `caisson.rs` | `Caisson` + `ChargeUtile` + `VarianteCaisson` + `VarianteCharge` | même famille « boîte » |
| `propulsion.rs` | `Propulseur` + `Motrice` + `BlocMoteur` + `FamillePropulsion` + `VariantePropulseur` (9) | même famille propulsion classique |
| `antimatiere.rs` | `ReacteurAntimatiere` + `MoteurAntimatiere` | déjà documentées comme « deux briques chaînées » (Partie D) |
| `reservoir.rs` | `Reservoir` | isolé mais simple |

Douze fichiers de ~100 à ~300 lignes chacun (le plus gros, `module_axial.rs`,
reste sous la barre grâce à ses 10 variantes qui partagent déjà beaucoup de
code) au lieu d'un fichier de 2800 lignes. **Aucun changement de
comportement** : c'est un déplacement de code, pas une réécriture — la
migration se valide avec les **130 tests** existants inchangés (ils testent
le comportement de `Composant::ports/dessiner/cout/rayon_local/
englobant_local`, pas l'endroit où vit le code ; voir l'audit de couverture
en E.1 — les 9 variantes qui n'avaient aucun test en ont désormais un chacune,
donc la migration a un vrai filet sur les 19 variantes, pas 10).

**Ordre de migration conseillé** (chaque étape compile et passe les tests) :
1. Créer `composant/commun.rs`, y déplacer les constantes et helpers partagés.
2. Extraire les familles une par une, **de la plus isolée à la plus
   couplée** : `reservoir.rs` → `antimatiere.rs` → `antenne.rs` →
   `panneau_solaire.rs` → `caisson.rs` → `adaptateur.rs` → `radiateur.rs` →
   `propulsion.rs` → `treillis.rs` → `noeud.rs` → `module_axial.rs` en dernier
   (le plus gros, le plus sûr une fois la mécanique rodée sur les petits).
3. À chaque extraction : couper-coller le bras de `match` concerné des 4
   fonctions vers le nouveau module, vérifier `cargo test`, commit.

### E.3 Le composant composite : `SousEnsemble` — ✅ implémenté (2026-07-29)

> **État : fait**, sans attendre le découpage §E.2 (décision utilisateur —
> l'ordre initialement prévu était E.2 puis E.3 ; en pratique E.3 est arrivé
> en premier). `composant.rs` reste donc un seul fichier pour l'instant ;
> §E.2 (découpage en modules) reste à faire, inchangé par ce qui suit.

Assembler plusieurs composants était déjà le travail de `Chantier`/
`Assembleur` (Partie C), qui produit une `Station` = `Vec<Piece>`. Mais il
n'existait **aucun moyen de traiter un groupe de pièces déjà assemblées comme
une seule brique réutilisable** — impossible de construire « une paire
d'ailes + adaptateur » une fois et de la clipser telle quelle à trois endroits
différents, ou de docker une station entière comme module d'une
mégastructure (pourtant annoncé en Partie D : « chaque classe supérieure
réutilise les briques des classes inférieures »).

**Une 20ᵉ variante de `Composant`, composite** (`src/vaisseau/composant.rs`) :

```rust
/// Sous-ensemble figé : un groupe de pièces déjà assemblées (ports cuits en
/// Mat4, comme dans une Station), traité comme UNE SEULE brique. Pattern
/// Composite : `SousEnsemble` a des ports/cout/rayon_local comme n'importe
/// quel composant, mais son `dessiner` délègue à ses enfants.
Composant::SousEnsemble {
    profil: Profil,                  // profil du port de montage présenté au parent
    donnees: Rc<DonneesSousEnsemble>,
}

pub struct DonneesSousEnsemble {
    pub pieces: Vec<Piece>,          // sous-arbre figé, repère LOCAL au sous-ensemble
    pub ports_exposes: Vec<Port>,    // ports hôtes restés libres, en repère local
    pub cout: f32,                   // précalculé = somme des cout() enfants
    pub rayon: f32,                  // précalculé = rayon englobant du sous-arbre
}
```

Les cinq comportements sont triviaux (c'est tout l'intérêt du Composite) :
`ports()` clone `donnees.ports_exposes` ; `cout()`/`rayon_local()` lisent les
champs précalculés (`O(1)`, important car `Chantier::poser` les appelle à
chaque pose) ; `englobant_local()` = `(Vec3::ZERO, donnees.rayon)`.

**`dessiner(p)` a demandé plus que prévu.** L'idée initiale — « boucle sur
`donnees.pieces`, délègue à `piece.composant.dessiner(p)` » — bute sur un
vrai trou d'abstraction découvert à l'implémentation : les primitives de
`Peintre` (`cylindre`, `cube`, …) dessinent dans un repère qu'un appelant
externe doit avoir déjà positionné (`push_model_matrix` côté GL pour
`Immediat`, `poser_transforme` côté `Batisseur`) — mais **aucune des deux
mécaniques n'était exposée par le trait `Peintre` lui-même**, seulement par
les types concrets. Un composite qui doit dessiner *plusieurs* enfants,
chacun à sa propre place, n'avait donc aucun moyen générique de le faire.
Ajouté au trait (`src/vaisseau/peintre.rs`) :

```rust
fn empiler_transforme(&mut self, m: Mat4); // compose par-dessus l'actif
fn depiler_transforme(&mut self);          // restaure
```

`Immediat` délègue à la pile GL de macroquad (qui compose déjà nativement).
`Batisseur` n'a qu'un champ `transforme` (pas de pile native) : il gagne un
`Vec<Mat4>` interne pour sauvegarder/restaurer. `SousEnsemble::dessiner`
devient alors : pour chaque enfant, `empiler_transforme(piece.transforme)` →
`piece.composant.dessiner(p)` → `depiler_transforme()` — testé explicitement
(`sous_ensemble_dessine_ses_enfants_a_leur_vraie_place`) pour vérifier que la
composition a bien lieu (et non un écrasement, qui aurait perdu la position
du composite lui-même dans son propre parent).

**Fabrication** : `Chantier::figer(self, profil) -> Option<Composant>`
(`src/vaisseau/chantier.rs`) — même idée que `Station::terminer()`
(Partie B §1, « on ne publie qu'une fois complet »), appliquée à un sous-arbre
au lieu de la station entière. `None` si rien n'a été posé (même invariant
que `Station::depuis_pieces`). Pas encore fait : geler un `Assembleur`/une
`Station` déjà publiée (utile pour réutiliser un preset entier comme brique
de mégastructure) — `figer` n'existe aujourd'hui que sur `Chantier`, qui est
le seul des deux à suivre les ports libres.

**Le coût réel de la perte de `Copy` : 120 sites, pas « quelques ».**
`Composant` (et `Piece`, qui l'embarque) a perdu `Copy` (`Rc` ne l'est pas) —
anticipé, mais l'estimation initiale (« quelques `.clone()` ») était fausse
par un ordre de grandeur : la suppression a cassé la compilation à **120
endroits**, presque tous dans les presets à la main (ISS/Mir/Tiangong/ISV de
`generateur.rs`) qui réutilisent une variable `Composant` plusieurs fois
(ex. un nœud avec 4 bras dockés dessus). Décidé **après avoir mesuré ce coût
en conditions réelles** (question posée explicitement) plutôt que de garder
`Copy` via un handle + registre : le registre aurait résolu le problème
immédiat mais en créant un nouveau pour l'éditeur futur — croissance non
bornée sans stratégie de réclamation (une session d'édition longue pose/
annule/repose des centaines de fois, contrairement au générateur qui
construit une fois et s'arrête), sérialisation impossible (un handle `u32`
n'a de sens que dans le process qui l'a créé), et undo/redo qui aurait dû
réinventer le comptage de références que `Rc` offre déjà. `Rc` reste donc la
décision retenue, alignée sur ce que l'éditeur (§E.4) demandera réellement.

**Comment le coût a été absorbé** (pas en modifiant 120 sites à la main) :
- Les fonctions internes qui n'avaient besoin que de **lire** un composant
  (`ports()`, `cout()`, `englobant_local()`) sont passées à `&Composant` —
  `Chantier::{payer, collision, ajouter_libres, compatibles}`,
  `montage::{port_monde, poser, cuire, cuire_symetrie}`,
  `generateur::{poser_sur, arrays_russes, appendice_sur_module, porter_vers,
  sur_face}`. Chacune ne clone qu'**une fois**, en interne, au moment de
  construire un `Piece` qui doit posséder son `Composant` (`comp.clone()`) —
  au lieu que chaque appelant s'en soucie.
- `cargo fix --broken-code` (rustfix) a ensuite appliqué mécaniquement les
  ~115 corrections restantes (emprunts, clones) à chaque site d'appel —
  suggestions `MachineApplicable` du compilateur, jamais de logique nouvelle.
- Une passe de nettoyage a retiré 42 `&x.clone()` redondants que `cargo fix`
  avait posés par prudence (une référence directe `&x` suffisait) : il n'en
  restait que **14 clones réellement nécessaires** dans tout le projet (les
  cas où la même conception se pose à plusieurs endroits dans une boucle).
- Validation : `cargo build`, `cargo build --tests` et `cargo test` (135
  tests, tous verts) après chaque étape — aucune régression de comportement,
  uniquement des changements de représentation (emprunt vs valeur, clone vs
  copie implicite).

### E.4 Ce qu'il faudra à un futur éditeur façon VAB (non fait ici)

Rappel : **ce qui suit n'est pas construit maintenant**. Ça sert seulement à
vérifier que §E.2/E.3 ne ferment aucune porte. Un éditeur d'assemblage
interactif (façon Kerbal Space Program : palette de pièces, clic pour poser,
undo, sauvegarde) aurait besoin, en plus de ce qui existe déjà :

- **Retrait d'une pièce (et de son sous-arbre)** : `Chantier`/`Assembleur`
  savent poser (`poser`) mais pas retirer — l'éditeur doit pouvoir supprimer
  une branche et **libérer** les ports qu'elle occupait. Aujourd'hui
  irréversible par construction (Partie B §1 : « on ne publie qu'une fois
  complet », l'immuabilité de `Station` n'anticipait pas l'édition
  incrémentale interactive).
- **Historique (undo/redo)** : conséquence directe du point précédent — une
  pile d'opérations `poser`/`retirer` réversibles, pas dans le modèle actuel.
- **Métadonnées de palette** : chaque variante a déjà un `nom()` (cf. Partie D
  et les `impl Variante* { pub fn nom(...) }` du §E.2), mais il manque une
  façon d'**énumérer** « tous les composants posables sur CE port libre,
  avec leur nom, pour construire un menu » — aujourd'hui seul le générateur
  sait quoi poser où, rien n'expose la liste aux mêmes fins pour un humain.
- **Sérialisation** : sauvegarder/charger un assemblage utilisateur (format
  simple : liste de `(Composant, port hôte visé)` dans l'ordre de pose —
  suffit à rejouer la construction, pas besoin de sérialiser les `Mat4`
  cuites).
- **`SousEnsemble` (§E.3) est le mécanisme naturel** pour la fonctionnalité
  « sauvegarder cette sélection comme sous-assemblage réutilisable » de KSP —
  c'est une des raisons de l'introduire maintenant plutôt que d'attendre
  l'éditeur : il sert déjà (mégastructures, Partie D) indépendamment de
  l'éditeur, et l'éditeur n'aura qu'à l'exploiter.

Rien de cette liste ne se code avant que §E.2 (découpage) et §E.3 (composite)
soient faits et stables — un éditeur interactif construit sur un
`composant.rs` de 2800 lignes hériterait du même problème en pire (chaque
pièce de la palette référençant un `match` géant).
