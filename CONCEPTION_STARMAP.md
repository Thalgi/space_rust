# Conception — Starmap (vue galactique)

> Chantier ouvert le 2026-07-04. **Conception d'abord, code après.**
> Vue la plus haute du jeu, au-dessus de la Skymap.

Légende bucket list : `[ ]` à faire · `[x]` fait · `[~]` partiel

---

## 1. Intention (le croquis)

Une carte **rétro** du voisinage stellaire :

- Une **grille en perspective diagonale** posée à plat = le **plan galactique** (le « sol »).
- Chaque **étoile flotte** au-dessus ou en-dessous de ce plan, à une **hauteur** qui
  correspond à sa **vraie altitude z** hors du plan.
- Une **tige pointillée verticale** relie chaque étoile à son **pied** (sa projection (x, y)
  sur la grille) → on lit d'un coup d'œil *où* elle est sur le plan et *à quelle hauteur*.
- Objets spéciaux : un **trou noir** avec disque d'accrétion (en haut à droite du croquis).
  ⚠️ **Non prioritaire** : simple idée du croquis, reportée à plus tard (cf. §8).

C'est la vue de navigation : on y choisit une étoile, on **zoome dedans → Skymap**
(le système complet, vue existante).

---

## 2. Place dans l'architecture

Nouveau mode, frère des autres écrans (`src/ecran/`) :

```
Accueil ──> Starmap (galaxie)  ──clic étoile──>  Skymap (système)  ──> Objet / Galeries…
```

- Nouveau bouton d'accueil « STARMAP - VOISINAGE STELLAIRE » (`Cible::Starmap`) et branche
  dans `main.rs` (aiguilleur `Etat`).
- La Starmap **précède** la Skymap : sélectionner une étoile construit la Skymap de cette
  étoile (preset ou graine, cf. §7).
- Réutilise le socle Minitel : `police::texte` (texte rétro), `ui::minitel_panel`/`minitel_ligne`
  (panneaux/boutons), palette cyan sur fond bleu nuit, filtre pixel optionnel (`P`).

---

## 3. Modèle de coordonnées

Repère **galactique local**, **Soleil à l'origine**. Chaque étoile a une position 3D issue
de ses coordonnées galactiques réelles `(l, b, d)` :

```
x = d · cos(b) · cos(l)      // le long du plan
y = d · cos(b) · sin(l)      // le long du plan
z = d · sin(b)               // HAUTEUR hors du plan  (la "tige")
```

- `d` : distance au Soleil (années-lumière).
- `l` : longitude galactique (direction dans le plan → **où** sur la grille).
- `b` : latitude galactique (au-dessus / en-dessous du plan → **signe et longueur** de la tige).

**Le pied** d'une étoile = point `(x, y, 0)` sur la grille. **L'étoile** = point `(x, y, z)`.
La tige pointillée relie les deux.

### ⚠️ Exagération verticale (décision clé)

Dans le voisinage solaire, **z est minuscule** devant l'étalement horizontal : le disque
galactique fait ~1000 al d'épaisseur, mais nos étoiles sont à ≤ ~12 al. Les vraies altitudes
(quelques al au plus) seraient écrasées sur la grille → aucun relief visible.

→ On applique un **facteur d'exagération vertical** `K_z` (réglable, ~3 à 6) :
`hauteur_écran = z · K_z`. On garde le **signe** et l'**ordre relatif** (physiquement honnête),
on amplifie juste la lecture. Idéalement `K_z` est un uniform hot-reloadable (touche R) pour
régler le rendu sans recompiler, dans l'esprit du reste du projet.

> Alternative discutée : normaliser `z` par un `z_max` du jeu de données. Plus simple mais
> perd l'échelle absolue. On part sur `K_z` fixe (plus lisible et cohérent entre cartes).

---

## 4. Le voisinage stellaire (jeu de données de départ)

Contenu demandé : **le Soleil + les étoiles environnantes** (Proxima, Tau Ceti…).
Table curée d'étoiles proches. Distances = valeurs fiables ; `(l, b)` **approximatifs, à
figer depuis un catalogue** (Gliese / RECONS / SIMBAD) avant de coder les positions.

| Étoile              | d (al) | l (°) ~ | b (°) ~ | z (haut/bas)   | Destination in-game            |
|---------------------|-------:|--------:|--------:|----------------|--------------------------------|
| **Soleil**          |   0.0  |    —    |    —    | plan (origine) | Preset **Solaire**             |
| **Proxima Centauri**|  4.25  |   313   |   −1.9  | ~plan (bas)    | (système Proxima — à définir)  |
| **Alpha Centauri A/B** | 4.37|   316   |   −0.7  | ~plan (bas)    | Preset **Avatar** (Polyphemus) |
| **Barnard**         |  5.96  |    31   |  +14    | au-dessus      | graine procédurale             |
| **Wolf 359**        |  7.86  |   244   |  +56    | haut au-dessus | graine procédurale             |
| **Lalande 21185**   |  8.31  |   185   |  +65    | haut au-dessus | graine procédurale             |
| **Sirius A/B**      |  8.60  |   227   |   −8.9  | sous le plan   | graine procédurale             |
| **Epsilon Eridani** | 10.5   |   196   |  −48    | bas sous plan  | graine procédurale             |
| **Tau Ceti**        | 11.9   |   173   |  −73    | très bas       | Preset **TauCeti**             |
| **Epsilon Indi**    | 11.9   |   336   |  −48    | bas sous plan  | graine procédurale             |

Observations utiles pour le rendu :
- **Alpha/Proxima Centauri** quasi dans le plan → tiges courtes. **Tau Ceti** ≈ « sous les pieds »
  du Soleil (b ≈ −73°) → longue tige vers le bas. **Wolf 359 / Lalande** haut au-dessus.
  → Beau contraste de hauteurs, fidèle au croquis, **sans inventer** : c'est la vraie géométrie.
- Les **binaires** (Alpha Cen A/B, Sirius A/B) : une seule entrée sur la carte (un point),
  détail dans la Skymap. Cf. §8.

> Extension naturelle plus tard : compléter avec le reste des ~50 étoiles < 15 al, puis
> un fond procédural par graine (mix presets + procédural), sans changer le modèle.

---

## 5. Rendu rétro

Tout au trait, palette Minitel (cyan/vert/violet sur bleu nuit), esprit filaire.

**La grille (le plan)**
- [ ] Maillage régulier de lignes (ex. 1 case = 2 al) sur une emprise ~14 × 14 al centrée sur
      le Soleil, projeté en oblique → l'aspect « diagonale » du croquis.
- [ ] Lignes cyan, atténuation vers l'horizon (fade en profondeur) pour la profondeur rétro.
- [ ] Repère central discret sur le Soleil (origine).

**Les tiges**
- [ ] Segment **pointillé** vertical du pied `(x,y,0)` à l'étoile `(x,y,z·K_z)`.
- [ ] Pointillé plus dense/atténué en bas (ancrage au sol) ; couleur = teinte de l'étoile désaturée.

**Les étoiles (glyphes)**
- [ ] Petit disque + halo, **couleur corps noir** selon le type (réutilise `etoile::couleur_corps_noir`).
- [ ] Taille du glyphe ∝ luminosité/type (repère visuel, pas la hauteur).
- [ ] Nom en dessous en police Minitel (togglable pour éviter la surcharge).
- [ ] Surbrillance au survol (halo inversé façon télétexte, comme `minitel_ligne`).

**Cas spécial trou noir** (cf. §8)
- [ ] Glyphe dédié : anneau sombre + disque d'accrétion (ellipse brillante) + tige qui plonge.

---

## 6. Projection & caméra (choix technique)

Deux options ; recommandation = **A** pour le look rétro net.

**A. Projection oblique/dimétrique fixe (CPU 2D)** — *recommandé*
- Un `project(x, y, z) -> Vec2` maison (oblique : `écran = origine + x·ex + y·ey + z·(0,−1)·K_z`).
- Lignes 1 px nettes, pas d'anti-alias 3D, contrôle total du style « papier quadrillé ».
- Pan + zoom simples (décalage/échelle 2D). Pas de perspective → très lisible, très Minitel.
- Tri en profondeur pour dessiner grille → tiges → étoiles dans le bon ordre.

**B. Vraie caméra 3D (réutiliser `camera::Camera` + `Camera3D`)**
- Grille = mesh, étoiles = billboards (comme les impostors). Orbite/parallaxe « gratuite ».
- Plus riche mais moins « à plat rétro », et plus lourd. À garder pour une évolution.

> On code **A** d'abord (fidèle au croquis, cheap), on garde **B** en réserve si on veut
> l'orbite 3D plus tard. Le modèle de coordonnées (§3) est identique dans les deux cas.

---

## 7. Interaction (sélection → zoom Skymap)

- [ ] **Survol** : glyphe/étiquette en surbrillance, panneau Minitel discret (nom, type, distance, z).
- [ ] **Clic** : sélection → transition **zoom** vers la **Skymap** de cette étoile.
- Correspondance étoile → système :
  - Étoile **avec preset** (Soleil→Solaire, Tau Ceti→TauCeti, Alpha Cen→Avatar) : ouvre le preset.
  - Étoile **sans preset** : `construire_systeme(graine)` avec une **graine dérivée de l'identité**
    de l'étoile (stable → même étoile ⇒ même système à chaque visite).
- [ ] **Échap** : retour Accueil. Réutilise le pattern `frame() -> bool`/`Option<Cible>`.
- Transition zoom : d'abord un simple *cut* (comme les autres écrans) ; animation d'entrée
  (dézoom sur l'étoile) = polish ultérieur.

---

## 8. Cas spéciaux

- **Trou noir (disque d'accrétion)** — ⚠️ **NON PRIORITAIRE** : c'était une idée du croquis,
  pas un besoin immédiat. Reporté. Déjà listé comme gros morceau dans `STELLAIRE_BUCKETLIST.md`.
  Le jour où on le fait : **glyphe stylisé** suffisant sur la Starmap (anneau + disque), pas de
  rendu physique (lentille) ici ; objet fictif/scénarisé épinglé (aucun trou noir réel < 15 al).
- **Systèmes binaires** (Alpha Cen A/B, Sirius A/B) : **un seul point** sur la carte ; la dualité
  se révèle dans la Skymap. Éventuel petit marqueur « double ».
- **Proxima** vs **Alpha Cen** : physiquement liées et quasi superposées (4.25 vs 4.37 al) →
  risque de chevauchement des glyphes. Prévoir un léger décalage d'affichage / regroupement.

---

## 9. Découpage en modules (proposé)

Dans l'esprit « fichiers courts ≤ ~150 lignes » du projet :

```
src/ecran/starmap.rs      // écran : état, input, frame() -> bool  (aiguillage, sélection)
src/starmap/mod.rs        // modèle : Etoile { nom, l, b, d, type, destination }, jeu de données
src/starmap/projection.rs // project(x,y,z)->Vec2, pan/zoom, K_z
src/starmap/rendu.rs      // grille, tiges pointillées, glyphes étoiles, glyphe trou noir
```

- Le **jeu de données** (§4) vit dans `starmap/mod.rs` (table en dur au départ, comme les presets).
- `destination` d'une étoile = enum `{ Preset(...), Graine(u64) }` → alimente le zoom (§7).

---

## 10. Plan d'implémentation

1. [x] **Squelette écran** : `Cible::Starmap` + bouton accueil + `Etat::Starmap` dans `main`,
   `frame()` qui clear + Échap retour. *(2026-07-04)*
2. [x] **Modèle + données** : struct `Etoile`, table du voisinage (§4), conversion `(l,b,d)->(x,y,z)`.
   *(`src/starmap/mod.rs`)*
3. [x] **Projection oblique** (§6.A) + `K_z` : `project()`. *(pan/zoom = polish §9, pas encore fait)*
4. [x] **Grille** : rendue en **nuage de points** (choix utilisateur) au lieu de lignes pleines.
5. [x] **Tiges + glyphes** : pointillés, disque/halo couleur corps noir, noms Minitel (togglables `N`).
6. [x] **Survol + sélection** : pick de l'étoile la plus proche du curseur (`viser`), surbrillance +
   panneau info Minitel (nom, type spectral, distance, hauteur z). *(2026-07-04)*
7. [x] **Zoom → Skymap** : clic -> `SortieStarmap::Systeme(dest)` -> `Skymap::depuis_destination`
   (preset nommé via `appliquer`, ou graine). Transition = *cut*. *(2026-07-04)*
8. [x] Gestion **binaires/chevauchement** (§8) *(2026-07-04)* : champ `Etoile.double` -> bille
   compagne (Alpha Cen, Sirius) + mention « (binaire) » au panneau ; champ `Etoile.decalage`
   (px, affichage seul) pour séparer Proxima ↔ Alpha Cen quasi superposées. *(Trou noir : reporté.)*
   Bonus : **billes rendues en pixel art** (`disque_pixel`/`pixel`/`crochets`, `TAILLE_PIXEL`).
9. [ ] Polish : `K_z` hot-reload (R), filtre pixel global (P), pan/zoom, animation d'entrée, atténuations.

---

## 11. Questions ouvertes

- **`K_z`** : valeur de départ (3 ? 5 ?) et unité de la grille (1 case = 2 al ?).
- **Trou noir** : purement fictif épinglé, ou réservé à un chantier « objets exotiques » ?
- **Proxima** : lui donner son propre système (procédural) ou la regrouper avec Alpha Cen ?
- **Caméra** : rester en oblique fixe (A) ou prévoir tout de suite l'orbite 3D (B) ?
- **Étendue** : s'arrêter à ~10 étoiles nommées, ou viser toutes les < 15 al dès le départ ?
