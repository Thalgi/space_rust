# Conception — fondations du générateur de stations

Compagnon de [`stations_procedurales.md`](stations_procedurales.md). Ce document
ne réécrit pas le modèle de ports (déjà validé) : il pose les **3 garde-fous
transverses** demandés — modèle d'état (1), plafond de coût flottant (7),
standard d'unités (8) — plus la synthèse de la phase de recherche. Fil rouge :
**KISS et coût de rendu maîtrisé**.

---

## 0. Ce que dit la recherche

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

## 1. Modèle d'état (point 1) — le plus léger possible

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

## 2. Standard d'unités (point 8)

But : chaque composant dimensionné dans une **unité commune**, proportions
cohérentes, emboîtement garanti — sans réglage au cas par cas.

### 2.1 Unité de base
Une seule constante :

```rust
const U: f32 = 1.0; // rayon du module « standard » = 1 U
```

**Toute** dimension s'écrit `n * U`. Changer `U` rescale toute station d'un coup.

### 2.2 Profils (diamètres discrets, façon KSP)

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

### 2.3 Proportions dérivées
Pour rester réaliste automatiquement, dériver les longueurs du diamètre plutôt
que de les fixer en absolu :
- longueur d'un module = **1,5 à 4 × diamètre** ;
- panneau solaire : largeur ≈ diamètre du module porteur, longueur = k × largeur ;
- treillis : demi-section = 0,5 à 1 × diamètre.

**Règle** : un composant ne fixe jamais une taille absolue arbitraire ; il
l'exprime en `U` ou relativement au **profil de son port**. Les proportions
restent homogènes sans effort.

---

## 3. Plafond de coût flottant (point 7)

But : borner la complexité d'une station pour protéger le budget de rendu, et
servir au cadrage caméra / anti-collision.

### 3.1 Budget de coût (le principal)
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

### 3.2 Rayon maximal (le secondaire)

```rust
struct Station { pieces: Vec<Piece>, rayon: f32 } // sphère englobante
```

Calculé **une fois** à la génération (distance max pièce ↔ centre). Sert à :
cadrer la caméra (comme `demi_dim` pour les maquettes actuelles), rejeter un
enfant qui dépasserait `rayon_max`, alimenter le filet anti-collision (§7 du doc
existant).

---

## 4. Symétrie (point 4, validé)

Reprendre KSP : **deux opérations seulement**.

```rust
enum Symetrie { Miroir, Radiale(u8) } // Radiale(n) = n copies autour de l'axe
```

Portée par les groupes de ports (`groupe_symetrie`, §3.6 existant). Le générateur
place les enfants d'un groupe symétrique **en une passe**. Indispensable au look
ISS (miroir gauche/droite des ailes) et aux nœuds type Mir (radiale).

---

## 5. Ordre d'implémentation

Ces fondations s'insèrent **avant** les étapes 6–7 du doc existant :

1. `U` + `Profil` + proportions (§2) — trivial, débloque tout le reste.
2. `EtatStation = Vide | Prete` + `Station` immuable (§1).
3. `cout()` par variante + `Budget` + `rayon` (§3).
4. `Symetrie` (§4), au moment du générateur (étape 7 du doc existant).

Reconstruire ISS / Mir **« en données »** (étape 6, point 5) : **plus tard**, une
fois le lot de variantes rempli.

---

## Sources
- [KSP — attachment nodes & symétrie (General Discussions)](https://steamcommunity.com/app/220200/discussions/0/1743358239843828618/)
- [KSP — tailles de pièces / form factors (Steam)](https://steamcommunity.com/app/220200/discussions/0/364042703862870924/)
- [KSP 2 — Size categories (modding wiki)](https://modding.kerbal.wiki/Size_Category)
- [KSP — coût CPU par nombre de pièces (forum)](https://forum.kerbalspaceprogram.com/topic/163317-how-to-improve-fps-with-high-part-count-crafts/)
- [SpaceshipGenerator — grammaire d'extrusion + symétrie (GitHub)](https://github.com/a1studmuffin/SpaceshipGenerator)
