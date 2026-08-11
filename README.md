# Solar Game

Bac à sable spatial en Rust / macroquad : systèmes stellaires procéduraux rendus
par shaders, et un éditeur d'assemblage de vaisseaux façon VAB.

```bash
cargo run
```

Première compilation un peu longue (macroquad récupère ses dépendances), ensuite
c'est rapide. Aucun asset externe n'est requis — tout est procédural.

---

## Ce qu'il y a dedans

Le jeu s'ouvre sur un **accueil** qui aiguille vers treize écrans, en deux
familles.

### Astres

| Écran | Ce qu'on y voit |
|---|---|
| **Skymap** | Un système complet : étoile(s), planètes, lunes, ceintures, zone habitable. Orbites képlériennes, jusqu'à 4 sources de lumière (binaires, trinaires, quadruples). |
| **Objet céleste** | Un corps isolé, pour travailler son rendu. |
| **Starmap** | Le voisinage stellaire, en projection oblique ; on zoome vers un système. |
| **Galeries** | Quatre catalogues côte à côte : telluriques (~130 presets), géantes gazeuses, ceintures & disques, étoiles. |

Les planètes telluriques sont construites par un pipeline complet — grille
cube-sphere, érosion, hydrologie D8, biomes, calottes — puis rendues en impostor
avec atmosphère en coquille, couche de nuages en repère propre, ombre d'anneau et
reflet spéculaire de l'océan. Les géantes ont leur profil zonal, leurs vortex et
leurs pôles. Les étoiles ont taches, granulation, boucles d'éruption et trois
types de couronne (halo, jets de pulsar, vent de Wolf-Rayet).

Un **filtre pixel art** optionnel (touche **X**) sous-échantillonne et quantifie
vers une palette de 64 couleurs.

### Vaisseaux et stations

| Écran | Ce qu'on y voit |
|---|---|
| **Briques** | Le catalogue des composants, un par un. |
| **Petites stations** | ISS, Mir, Tiangong reproduites. |
| **Générateur procédural** | Stations engendrées par grammaire, à graine. |
| **Mégastructures** | Anneaux, tores de Stanford, complexe Elysium. |
| **Vaisseaux** | ISV Venture Star, Starship. |
| **Assembleur (VAB)** | **On y construit un vaisseau à la main.** |

---

## L'assembleur

C'est la partie la plus récente, et elle marche de bout en bout : on choisit une
pièce, on vise un port, on pose ; on sélectionne, on tourne, on déplace, on
retire ; on annule, on sauve, on recharge.

Le modèle est un **graphe de ports** : une pièce ne se trouve pas quelque part,
elle est *accouplée* à un port, et sa pose en est déduite. C'est ce qui fait
exister la compatibilité genre/profil, l'anti-collision, le sous-arbre d'un
retrait, et un format de sauvegarde qui ne sérialise **aucune matrice** — une
recette est la liste ordonnée des poses, qu'il suffit de rejouer.

### Commandes

| | |
|---|---|
| clic palette | prend la pièce en main — ou la pose à l'origine si le chantier est vide |
| clic sur un port **vert** | pose |
| **R** / **Maj+R** | tourne la pièce de 15° autour de son axe d'accostage |
| **Tab** | change de port de montage (donc d'orientation) |
| clic sur une pièce | la sélectionne, avec son sous-arbre |
| **Suppr** / **M** | retire / déplace la sélection |
| **Échap** | repose la pièce en main, puis quitte |
| **Z** / **Y** | annuler / refaire |
| **1**–**9**, **F5**, **F9** | emplacement de sauvegarde, écrire, relire |
| **P** / **E** / **X** | ports · enveloppes de collision · filtre pixel |

Les ports se colorent : **vert** ça passe, **rouge** compatible mais encombré,
**éteint** incompatible. Viser un port rouge trace l'enveloppe fautive, le
segment de plus courte approche, et écrit de combien il manque.

Les vaisseaux sauvegardés vont dans `vaisseaux/` (ignoré par git).

---

## Conventions du dépôt

- **Le code est commenté en français**, y compris les shaders, et les
  commentaires disent *pourquoi* plutôt que *quoi*.
- **Chaque test est « red-checké »** : on casse délibérément ce qu'il garde, on
  vérifie qu'il rougit, on rétablit. Un test qui reste vert sous sabotage ne
  garde rien — le journal en compte plusieurs, y compris des tests jetés pour
  cette raison.
- **Pas de test de rendu** (couleurs, disposition) : ce sont des arbitrages qui
  se jugent à l'œil. Tout ce qui *se décide* sort donc du code de dessin, où il
  devient testable.
- `cargo clippy --all-targets` doit rester à **zéro avertissement**.

État vérifié : **622 tests verts**, 0 avertissement clippy.

---

## Documentation

`STATE.md` à la racine est le **journal de bord** : ce qui a été fait, pourquoi,
et ce que chaque bug a appris. C'est le premier fichier à lire pour reprendre le
travail.

`docs/` contient le reste, en trois familles — voir
[`docs/README.md`](docs/README.md) :

- **`conception/`** — le *pourquoi* : intentions, architecture, décisions ;
- **`suivi/`** — le *où on en est* : journaux, catalogues, bilans d'étape ;
- **`reference/`** — des fichiers **générés**, reconstruits et comparés par un
  test, jamais édités à la main.
