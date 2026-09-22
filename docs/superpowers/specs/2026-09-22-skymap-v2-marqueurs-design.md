# Skymap V2 — marqueurs de planètes et lunes

> Design, 2026-09-22. Écrit avant tout code. Approuvé par l'utilisateur (architecture
> validée en conversation avant l'écriture de ce document).

## 1. Le problème

Dans la Skymap actuelle ([src/ecran/skymap.rs](../../../src/ecran/skymap.rs)), les
corps sont rendus à leur échelle **physique réelle** (dette assumée, voir
`STATE.md`) : au cadrage par défaut, une planète tellurique fait quelques
millièmes d'unité de rayon — souvent bien sous le pixel. Deux conséquences :

1. **Les planètes sont invisibles en vue globale du système.** Rien ne dit où
   elles sont ni ne permet de cliquer dessus — seule la colonne de gauche
   (sélecteur en arbre) permet de les atteindre.
2. **Les lunes ont le même problème, en pire** : encore plus petites, et leurs
   orbites ne s'affichent aujourd'hui qu'en tout-ou-rien via une bascule
   globale (le double bouton « RING / MOONS », `src/menu/dessin.rs` ~ligne
   42), jamais liée à une sélection.

## 2. Objectif

Une nouvelle vue **Skymap V2**, séparée de la V1, qui ajoute :

1. Un **marqueur triangulaire** en permanence sur la position projetée à
   l'écran de chaque planète du système. Cliquer le marqueur sélectionne la
   planète (même comportement que cliquer le corps lui-même aujourd'hui :
   focus caméra + ouverture de la fiche).
2. Quand une planète (ou une de ses lunes) est sélectionnée : l'**anneau
   d'orbite de chacune de ses lunes** apparaît, plus un **marqueur
   triangulaire identique** sur chaque lune. Rien de tout ça n'est visible
   hors sélection.

V1 reste **intouchée** : c'est un fork complet, pour pouvoir revenir dessus
sans rien défaire si le résultat ne convient pas.

## 3. Portée

- Le **système de marqueurs est générique** : rien dans son cœur (§4.2) ne
  présume du type d'astre. Cette itération ne l'**alimente** qu'avec deux
  familles — planètes et lunes — mais la même mécanique (projection,
  hit-test, dessin) doit pouvoir servir demain à un vaisseau, une station ou
  une étoile sans y toucher, seulement en lui passant une autre liste
  d'indices. Voir §9 pour ce qui n'est délibérément pas câblé maintenant.
- Concerne, pour cette itération, uniquement les astres de catégorie
  **Planète** (racines, sans parent) et les astres de catégorie **Lune**
  enfants directs d'une planète. ⚠️ Corrigé en préparant le plan : la
  première version prenait « tout enfant direct, sans filtre de catégorie »,
  or l'ISS (`EnginOrbital`, `engin.rs`) a la Terre pour `parent` — elle
  aurait reçu un marqueur, contre le périmètre. `Systeme::lunes_de` reste
  générique (tous les enfants directs) ; c'est la **sélection d'indices** de
  `marqueur.rs` qui filtre `Categorie::Lune`.
- Les calques existants (orbites de planètes en ellipse, zone habitable,
  orbites stellaires) restent inchangés dans leur comportement.

## 4. Architecture

### 4.1 Fork complet de l'écran

`src/ecran/skymap.rs` est copié vers un nouveau `src/ecran/skymap_v2.rs`
(struct `SkymapV2`), identique à `Skymap` sauf les ajouts listés ci-dessous.
Pas de tronc commun extrait : la duplication est acceptée délibérément pour
que V2 reste un module jetable d'un bloc.

Fichiers touchés :

| Fichier | Nature | Contenu |
|---|---|---|
| `src/ecran/skymap_v2.rs` | **nouveau** | Copie de `skymap.rs` + intégration du module marqueur (§4.2, §5, §6) |
| `src/ecran/marqueur.rs` | **nouveau** | Calcul des marqueurs, hit-test, dessin (§4.2) |
| [src/ecran/mod.rs](../../../src/ecran/mod.rs) | modifié | `mod skymap_v2;` + `pub use skymap_v2::SkymapV2;` |
| [src/ecran/accueil.rs](../../../src/ecran/accueil.rs) | modifié | Entrée `("SKYMAP V2 - MARQUEURS", Cible::SkymapV2)` dans `BLOC_ASTRES`, nouvelle variante `Cible::SkymapV2` |
| [src/main.rs](../../../src/main.rs) | modifié | Variante `Etat::SkymapV2(Box<SkymapV2>)`, branchement dans l'aiguilleur (import, arm du `match` sur `Cible`, arm du `match` sur `Etat`) |
| [src/systeme/mod.rs](../../../src/systeme/mod.rs) | modifié | Nouvelle méthode `Systeme::lunes_de(parent) -> Vec<usize>` (§6.2) |
| `src/ecran/trace_orbite.rs` | **nouveau** | Orbites de lunes en trait large pointillé-trait, tracées en 2D (§6.4, §7) |
| [src/systeme/mod.rs](../../../src/systeme/mod.rs) | modifié | Accesseur `Systeme::orbite_lune_de(idx) -> &[Vec3]` (§7) |
| ~~`Calques::lunes_de`, `draw_orbites_lunes_de`, la ligne de `skymap.rs`~~ | **retirés** | Remplacés par le tracé 2D — V1 finit strictement intouchée (§6.4) |
| [src/camera.rs](../../../src/camera.rs) | modifié | Retrait de `#[allow(dead_code)]` sur `projeter_ecran` (premier appelant réel) |

### 4.2 Module `ecran/marqueur.rs`

Toute la **décision** (position écran, quel marqueur est sous le curseur) est
dans des fonctions pures, testables sans contexte graphique — dans l'esprit de
`docs/methode.md` (« ce qui se décide sort du code de dessin »). Seule la
fonction de dessin touche macroquad.

```rust
/// Un marqueur : l'astre qu'il désigne, sa position projetée à l'écran.
///
/// Ne porte rien de spécifique à un type d'astre : un marqueur de planète,
/// de lune, ou (plus tard) de vaisseau, de station ou d'étoile est
/// EXACTEMENT la même donnée. C'est `marqueurs_pour` (générique) et ses
/// appelants (§3 : seulement planètes/lunes pour l'instant) qui décident
/// quels astres en reçoivent un.
pub struct Marqueur {
    pub idx: usize,
    pub ecran: Vec2,
}

/// **Cœur générique** : un marqueur par astre de `indices`, projeté à
/// l'écran — sans aucune hypothèse sur ce que ces indices désignent.
///
/// Deux astres sont exclus, et c'est la SEULE logique de filtrage ici :
/// - **derrière la caméra** (`projeter_ecran` rend `None`) ;
/// - **hors du cadre** (point projeté en dehors de `[0, ecran.x] ×
///   [0, ecran.y]`) — exclu **à la source**, avant d'entrer dans le `Vec`,
///   plutôt que filtré plus tard au dessin ou au hit-test : sur un système
///   chargé (des dizaines d'astres), un marqueur hors champ ne doit coûter
///   ni un test de survol ni un appel de dessin par frame. C'est la demande
///   « disparaît d'une manière à économiser de l'énergie ».
pub fn marqueurs_pour(
    sys: &Systeme,
    indices: impl IntoIterator<Item = usize>,
    cam: &CameraInfo,
    ecran: Vec2,
    aspect: f32,
) -> Vec<Marqueur>;

/// Un marqueur par planète RACINE (catégorie Planète, sans parent).
/// Délègue entièrement à `marqueurs_pour` — ne fait que choisir les indices.
pub fn marqueurs_planetes(sys: &Systeme, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur> {
    marqueurs_pour(sys, indices_planetes_racines(sys), cam, ecran, aspect)
}

/// Un marqueur par LUNE de `parent` : enfants directs (`sys.lunes_de(parent)`)
/// de catégorie `Lune` seulement — l'ISS, enfant de la Terre, est exclue (§3).
pub fn marqueurs_lunes(sys: &Systeme, parent: usize, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur> {
    let lunes = sys.lunes_de(parent).into_iter().filter(|&i| sys.categorie_de(i) == Some(Categorie::Lune));
    marqueurs_pour(sys, lunes, cam, ecran, aspect)
}

/// Le marqueur le plus proche de `souris`, sous le rayon `rayon_px` — `None` si aucun.
/// Égalité de distance : le premier de la liste l'emporte (déterministe, pas d'ambiguïté
/// pratique — les marqueurs ne se superposent jamais exactement).
pub fn plus_proche(marqueurs: &[Marqueur], souris: Vec2, rayon_px: f32) -> Option<usize>;

/// Dessine tous les marqueurs. `survole`/`selectionne` pilotent la mise en valeur (§5).
pub fn dessiner(sys: &Systeme, marqueurs: &[Marqueur], survole: Option<usize>, selectionne: Option<usize>);
```

`indices_planetes_racines` (privée, dans `marqueur.rs`) filtre
`Categorie::Planete && parent().is_none()` — une planète qui aurait elle-même
un parent (cas de bord existant dans `Systeme::designation`, jamais vu en
pratique mais géré par le code actuel) compte comme une lune, pas une
planète racine, et ne doit pas recevoir les deux marqueurs à la fois.

**Pourquoi le cœur générique plutôt que deux fonctions spécialisées d'emblée**
— demande explicite : la seule chose qui doit changer pour marquer une autre
famille d'astres (vaisseau, station, étoile) plus tard est la fonction qui
fournit les indices ; `marqueurs_pour`, `plus_proche` et `dessiner` n'ont
rien à savoir de la catégorie de ce qu'ils manipulent.

## 5. Rendu du marqueur

- **Triangle plein**, taille écran fixe : base 8 px, hauteur 10 px, pointe en
  bas touchant exactement le point projeté (`Marqueur::ecran`), sommet vers
  le haut — comme une punaise plantée sur la position de l'astre.
- **Couleur** : `selecteur::teinte_astre(sys, idx, cat)` — pas un appel direct
  à `Systeme::teinte_de` (correction au spec, voir la quatrième correction
  dans `docs/suivi/interface.md`), mais la **même fonction** que la pastille
  du sélecteur — cohérence visuelle, une retouche d'apparence se voit
  aussitôt aux deux endroits. Elle éclaircit `teinte_de` (`relever`, plancher
  0,42, sans délaver une couleur déjà vive) et retombe sur une teinte par
  catégorie (`teinte_categorie`), pas sur le cyan d'interface, si `teinte_de`
  rend `None`.
- **Survol** (curseur à moins de 14 px, §6.1) : contour blanc 1 px + alpha
  plein (1.0). Hors survol, alpha 0.85 (le marqueur reste lisible mais ne
  concurrence pas le corps rendu).
- **Sélectionné** (`idx == fiche_ouverte`) : cercle fin (rayon 8 px, blanc,
  50 % d'opacité) centré sur le centre du triangle — 8 et non 6 : les coins
  de la base sont à 6,4 px du centre, un cercle de 6 les couperait —, en plus de l'effet de
  survol s'il s'y ajoute. Les deux états sont indépendants (un marqueur peut
  être sélectionné sans être survolé, et réciproquement).
- **Décalage au-dessus de l'astre** (révisé le 2026-09-22, après retour
  visuel : « quand on zoome il devrait être offset au-dessus avec un léger
  écart »). La pointe ne se pose pas sur le centre projeté : elle remonte de
  `rayon apparent + ECART_PX` (6 px). Le rayon apparent se **mesure** (on
  projette un point du monde écarté du rayon de l'astre le long de la
  verticale caméra), il ne se devine pas : un décalage fixe se retrouverait
  au milieu du disque dès qu'on approche. De loin, le rayon apparent est nul
  et il ne reste que l'écart. `Marqueur::ecran` porte donc **la pointe**, pas
  le centre de l'astre — le survol et le clic visent ce qu'on voit ; cliquer
  le disque lui-même reste possible par le pick 3D.
- Dessiné en **passe interface**, juste après `self.rendu.rendre(...)` et
  avant bandeau/sélecteur/fiche — ces derniers, dessinés par-dessus,
  recouvrent naturellement tout marqueur qui tomberait sous leur zone (même
  ordre de calque que le reste de l'UI, pas de cas particulier à gérer).

## 6. Interaction

### 6.1 Rayon de détection

`RAYON_HIT_PX = 14.0` (module `marqueur.rs`) : légèrement plus large que le
triangle (8×10 px) pour rester clicable sans viser au pixel près — dans
l'esprit de la marge déjà prise par `Systeme::pick` (`rayon * 1.4`).

### 6.2 Planète « active » (dont les lunes sont montrées)

```rust
fn planete_active(sys: &Systeme, selection: Option<usize>) -> Option<usize> {
    let idx = selection?;
    match sys.categorie_de(idx)? {
        Categorie::Planete => Some(idx),         // la planète elle-même
        Categorie::Lune => sys.parent_de(idx),   // une lune -> sa planète
        _ => None,                               // étoile, engin (ISS !), ceinture : rien
    }
}
```

Sur la catégorie et non sur `parent_de` : sélectionner l'ISS (enfant de la
Terre) ne doit pas afficher les lunes de la Terre.

`selection` = `self.fiche_ouverte` — **pas** `fiche_affiche`. Corrigé en
préparant le plan : `fiche_affiche` garde le dernier astre montré pour que le
panneau conserve son contenu pendant qu'il glisse hors champ, et ne redevient
**jamais** `None`. S'en servir laisserait les lunes affichées indéfiniment
après une désélection, contre l'exigence « visibles uniquement quand la
planète est sélectionnée ». `fiche_ouverte`, lui, repasse à `None` au clic
dans le vide. Conséquence conservée : sélectionner une lune garde les
marqueurs de ses sœurs visibles (pas de disparition-réapparition en cliquant
d'une lune à l'autre de la même planète).

`Systeme::lunes_de` (nouveau, `systeme/mod.rs`, même famille que `nb_lunes`
déjà présent) :

```rust
/// Indices des astres dont le parent direct est `parent` — les « lunes » au
/// sens large (catégorie non filtrée, voir §3). Ordre : celui de `self.astres`.
pub fn lunes_de(&self, parent: usize) -> Vec<usize> {
    self.astres.iter().enumerate()
        .filter(|(_, a)| a.parent() == Some(parent))
        .map(|(i, _)| i)
        .collect()
}
```

### 6.3 Clic : marqueur d'abord, pick 3D en repli

Dans `SkymapV2::frame()`, remplace le bloc de pick actuel :

```
si clic et pas sur l'UI :
    marqueurs = marqueurs_planetes(...) + (si planete_active, marqueurs_lunes(..., planete_active, ...))
    si let Some(idx) = plus_proche(&marqueurs, souris, RAYON_HIT_PX) :
        // même suite que le pick 3D touché aujourd'hui
        self.cam.set_focus(idx)
        gérer lancement armé / ouverture de fiche (identique à Skymap::frame)
    sinon :
        // repli : comportement V1 inchangé (étoiles, vaisseaux, tout le reste)
        pick = self.cam.pick(&self.sys, &cam_info, aspect)
        ...
```

Le survol (pour l'affichage, §5, et pour l'aperçu de trajet pendant un
armement — `previsualisation_trajectoire`) suit la même priorité : marqueur
d'abord, `sys.pick` en repli.

### 6.4 Orbites des lunes

⚠️ **Révisé le 2026-09-22, après retour visuel** (« je voudrais une ligne
trois fois plus large au style mediumdashdot »). La première version passait
par un champ `Calques::lunes_de` et un semis de points dessiné dans la passe
3D de `RenduStandard::rendre`. Elle est **abandonnée** : `draw_line_3d` de
macroquad n'a pas d'épaisseur — il rend toujours un pixel —, donc un trait
large est hors de portée de cette passe.

Les orbites de lunes se tracent désormais **en 2D, en passe interface**
(§7) : la polyligne est projetée à l'écran, puis découpée au motif
mediumdashdot et tracée avec `draw_line`, qui prend une épaisseur.

Conséquences :

- le champ `Calques::lunes_de` et `Systeme::draw_orbites_lunes_de` sont
  **retirés** — plus rien ne les appelle ;
- **V1 redevient strictement intouchée** : la ligne `lunes_de: None` du
  littéral `Calques` de `skymap.rs` disparaît avec le champ ;
- V2 passe toujours `lunes: false` à `draw_orbites`, et appelle
  `trace_orbite::dessiner_orbites_lunes` quand `planete_active` rend
  `Some(parent)` ;
- le trait ne passe plus le test de profondeur : il se voit même quand la
  lune est derrière sa planète. Assumé — il n'apparaît que pour l'astre
  sélectionné, et sert à dire où chercher la lune.

## 7. `ecran/trace_orbite.rs` (nouveau)

Même partage que `marqueur.rs` : ce qui **décide** est pur et testé, seul le
tracé touche macroquad.

```rust
/// Trait trois fois plus large que le pixel des autres calques.
pub const EPAISSEUR_PX: f32 = 3.0;
/// Violet pâle des calques, remonté en opacité (un trait large et transparent bave).
pub const COL_LUNES: Color = Color::new(0.62, 0.55, 0.90, 0.85);
/// mediumdashdot, en pixels écran : trait, trou, point, trou.
const MOTIF: [(f32, bool); 4] = [(12.0, true), (4.0, false), (2.0, true), (4.0, false)];

/// Découpe une polyligne ÉCRAN selon MOTIF, et rend les segments à encrer.
pub fn segments_motif(points: &[Vec2]) -> Vec<(Vec2, Vec2)>;

/// Trace l'orbite de chaque lune de `parent`. Passe interface.
pub fn dessiner_orbites_lunes(sys: &Systeme, parent: usize, cam: &CameraInfo, ecran: Vec2, aspect: f32);
```

Deux points qui décident du rendu :

- le motif **se poursuit d'un sommet de polyligne au suivant** (il ne repart
  pas de zéro à chaque sommet) : ce qui compte est la longueur parcourue à
  l'écran, pas le nombre d'échantillons de l'orbite. Sans ça, une orbite
  échantillonnée fin sortirait en trait plein ;
- un point **derrière la caméra** coupe la polyligne en deux morceaux, au
  lieu de les relier par un trait fantôme en travers de l'écran.

Nouvel accesseur `Systeme::orbite_lune_de(idx) -> &[Vec3]` (polyligne
relative au parent) : le tracé a besoin des points, pas d'un appel de dessin.

## 8. Tests prévus

Purs, sans contexte graphique — suivent le patron déjà en place dans
`systeme/mod.rs` (`CorpsEssai`) et `camera.rs` (`base_camera`, testé sans
macroquad).

- `marqueur::plus_proche` : le plus proche gagne (plusieurs candidats), rayon
  respecté (un marqueur juste hors rayon ne compte pas), liste vide → `None`,
  égalité de distance → le premier de la liste.
- `Systeme::lunes_de` : rend les bons indices pour un parent donné, liste
  vide pour une planète sans lune, n'inclut pas les petits-enfants (une lune
  d'une lune, cas qui n'existe pas dans le modèle actuel mais que le filtre
  `parent() == Some(parent)` exclut naturellement — pas de test spécifique
  nécessaire, propriété du filtre lui-même).
- `marqueurs_pour` : `projeter_ecran` est déjà pur (`CameraInfo` par valeur)
  → testable avec une caméra et des positions d'essai construites à la main,
  même esprit que `CorpsEssai`. Vérifie l'absence de marqueur pour un astre
  **derrière la caméra** et pour un astre **hors du rectangle écran**
  (projeté mais `x`/`y` en dehors de `[0, ecran.x[`/`[0, ecran.y[`) — les
  deux cas de rejet à la source (§4.2). Un astre juste dans le cadre (bord
  inclus) reçoit bien un marqueur, pour fixer la frontière.
- `marqueurs_planetes`/`marqueurs_lunes` : vérifient uniquement le filtre
  catégorie/parent (`indices_planetes_racines`, `Systeme::lunes_de`) —
  la projection elle-même est déjà couverte par les tests de `marqueurs_pour`
  ci-dessus, pas la peine de la re-tester ici.
- `planete_active` : les trois branches (lune sélectionnée, planète
  sélectionnée, ni l'un ni l'autre).
- Chaque test **red-checké** (méthode.md) avant d'être considéré acquis.
- Pas de test sur `dessiner`/`draw_orbites_lunes_de` (dessin — convention du
  projet, `docs/methode.md` : « pas de test de rendu »).

## 9. Hors périmètre (assumé)

- ⚠️ **Le plus important à noter ici** : le cœur du système de marqueurs
  (`marqueurs_pour`, `plus_proche`, `dessiner`) est écrit **générique**
  dès cette itération (§4.2) — il ne connaît que des indices d'astres, pas
  une catégorie. Mais **seuls les indices de planètes et de lunes lui sont
  passés maintenant** : étoiles, vaisseaux/engins, stations et ceintures
  n'ont aujourd'hui **aucun appelant** qui leur construirait un marqueur.
  Les brancher plus tard est un ajout (une fonction de sélection d'indices
  de plus, façon `marqueurs_planetes`), pas une reprise d'architecture.
- Décollisionnement des marqueurs qui se chevaucheraient à l'écran (planètes
  internes groupées serré au cadrage englobant) : pas traité dans ce premier
  jet — à juger à l'œil une fois construit, ajustable sans revoir
  l'architecture si besoin (ajouterait une passe de répulsion 2D dans
  `dessiner`).
- Indicateur de bord d'écran pour un astre hors-champ (écarté en
  clarification : le besoin exprimé est la visibilité d'un corps déjà à
  l'écran mais sous le pixel, pas un corps hors du champ de vue).
