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
   orbites ne s'affichent aujourd'hui qu'en tout-ou-rien via un bouton global
   (`ORBITES LUNES`), jamais liées à une sélection.

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
| [src/systeme/rendu.rs](../../../src/systeme/rendu.rs) | modifié | Nouvelle méthode `Systeme::draw_orbites_lunes_de(parent, col, cam)` (§7), appelée par `draw_orbites` |
| [src/systeme/mod.rs](../../../src/systeme/mod.rs) (`Calques`) | modifié | Champ `lunes_de: Option<usize>` (§6.4) |
| [src/ecran/skymap.rs](../../../src/ecran/skymap.rs) | **une ligne** | `lunes_de: None` dans le littéral `Calques` — mécanique, comportement V1 inchangé |
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
- **Couleur** : la teinte propre de l'astre (`Systeme::teinte_de(idx)`, la
  même source que la pastille du sélecteur — cohérence visuelle, une
  retouche d'apparence se voit aussitôt aux deux endroits). Repli sur le cyan
  d'interface (`Color::new(0.0, 0.85, 0.85, 1.0)`) si `teinte_de` rend `None`.
- **Survol** (curseur à moins de 14 px, §6.1) : contour blanc 1 px + alpha
  plein (1.0). Hors survol, alpha 0.85 (le marqueur reste lisible mais ne
  concurrence pas le corps rendu).
- **Sélectionné** (`idx == fiche_ouverte`) : cercle fin (rayon 8 px, blanc,
  50 % d'opacité) centré sur le centre du triangle — 8 et non 6 : les coins
  de la base sont à 6,4 px du centre, un cercle de 6 les couperait —, en plus de l'effet de
  survol s'il s'y ajoute. Les deux états sont indépendants (un marqueur peut
  être sélectionné sans être survolé, et réciproquement).
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

Les orbites sont dessinées **dans** `RenduStandard::rendre` ([src/rendu.rs](../../../src/rendu.rs)),
sous la caméra 3D et avant les corps : un appel séparé après `rendre` n'aurait
plus de caméra 3D active et passerait par-dessus les corps. Le ciblage passe
donc par `Calques`, qui gagne un champ :

```rust
/// Orbites des lunes de CETTE planète seulement, à pleine opacité —
/// indépendamment de `lunes` (le calque global). `None` : rien de ciblé.
pub lunes_de: Option<usize>,
```

`draw_orbites` appelle `self.draw_orbites_lunes_de(parent, COL_LUNES, cam)`
quand `lunes_de` est `Some(parent)`. V2 passe toujours `lunes: false` et
`lunes_de: planete_active(&self.sys, self.fiche_ouverte)`.

⚠️ **Seule retouche de V1** : `Calques` est construit par littéral dans
`skymap.rs` — il y gagne une ligne `lunes_de: None`, purement mécanique, qui
laisse son comportement inchangé.

## 7. `Systeme::draw_orbites_lunes_de` (nouveau, `systeme/rendu.rs`)

Réutilise directement `dessiner_points` (déjà privée dans ce fichier, aucun
changement de visibilité nécessaire) :

```rust
/// Dessine l'orbite de chaque astre enfant direct de `parent`, à pleine
/// opacité — pas d'atténuation façon `draw_orbites` : appelée seulement
/// quand `parent` (ou une de ses lunes) est sélectionné, donc jamais en
/// concurrence visuelle avec d'autres traces.
pub fn draw_orbites_lunes_de(&self, parent: usize, col: Color, cam: Vec3) {
    let Some(centre) = self.astres.get(parent).map(|a| a.corps().position) else { return };
    for a in &self.astres {
        if a.parent() != Some(parent) { continue; }
        let pts = a.orbite_lune();
        if pts.is_empty() { continue; }
        dessiner_points(pts, centre, col, cam);
    }
}
```

`COL_LUNES` déjà défini dans ce fichier (violet pâle) — réutilisé tel quel.

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
