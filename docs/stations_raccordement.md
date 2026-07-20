# Conception — raccordement ports ↔ assemblage (Étape 2)

Troisième doc stations, à lire après [`stations_procedurales.md`](stations_procedurales.md)
(plan directeur, modèle de ports) et [`stations_fondations.md`](stations_fondations.md)
(état, budget, unités, symétrie). Il ne réexplique pas ces briques : il pose **le
chaînon qui les relie** — le trait/enum `Composant`, l'évolution de `Piece`, et la
« cuisson » des transformées. C'est l'**Étape 2** du plan directeur.

Fil rouge inchangé : **KISS**, coût de rendu maîtrisé, invariants tenus *par
construction* plutôt que surveillés.

---

## 0. État de départ (ce qui existe déjà)

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

## 1. Décisions actées

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

---

## 2. Architecture à deux couches (le cœur)

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

### Pourquoi `Mat4` cuite et pas `Repere` dans `Piece`

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

## 3. Cible de types

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

## 4. Sous-étapes ordonnées (chacune se valide seule)

> **État (2026-07-16) : Étape 2 close (2a→2f faits).** Raccordement complet
> validé : les composants s'assemblent par ports, se cuisent en `Mat4`, se
> dessinent (écran « STATION », bouton du menu). `cargo test` couvre `composant`,
> `assemblage` et `montage`. Premiers composants réels construits par-dessus :
> `ModuleAxial`, `Noeud` (4 dispositions) et `PanneauSolaire` (5 variantes) — voir
> `docs/stations_procedurales.md` §0 pour la suite.
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
(orange) et **haut** (vert) ; touche **P** dans la vue STATION. La vue cycle aussi
plusieurs démos (touche **D**).
Petit tracé des ports d'un composant (flèche `avant`, repère `haut`) dans
l'atelier `ecran/briques.rs`. Facultatif mais accélère la mise au point de 2a–2d.

---

## 5. Hors scope (rappel — Étapes 3+)

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

## 6. Limites & extensions futures (mégastructures)

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
