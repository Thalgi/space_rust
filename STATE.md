# STATE — où en est le chantier

> Point de reprise, **2026-08-02**. À lire en premier dans une nouvelle session.
> Ce fichier dit **où on en est et quoi faire ensuite**, rien de plus : le
> détail de chaque étape (ce qui a été trouvé, les red-checks, les mesures)
> vit dans [`docs/suivi/assembleur.md`](docs/suivi/assembleur.md), et le
> *pourquoi* dans [`docs/conception/assembleur.md`](docs/conception/assembleur.md).

## ⚠️ Rien n'est commité

Dernier commit : `32300e9` « Vaisseau : ISV complet avec VFX ».

**Tout ce qui suit est dans l'arbre de travail, non commité** — consigne
debout de l'utilisateur : *« no commit i'll say when commit is needed »*.
Ne pas commiter sans qu'il le demande.

État vérifié : **267 tests verts** (`cargo test --release`, ~0,8 s),
**57 avertissements**, build release propre, **`cargo clippy` compile**
(75 avertissements, 0 erreur).

## ⏸ Assembleur en pause

Mis en pause par l'utilisateur le **2026-08-02**, Lot 3 clos et Lot 4 conçu
mais pas commencé. Pour reprendre : lire `docs/conception/assembleur.md` §10,
puis attaquer L4.1.

## Où on en est

L'ISV est **clos** (asset DONE, `docs/suivi/stations.md` §C.29). Le chantier
courant est l'éditeur d'assemblage interactif façon VAB.

| Lot | Contenu | État |
|---|---|---|
| **Lot 1** (L1.1–L1.8) | Assainir les fondations : catalogue à source unique, balayage des 31 variantes, déterminisme, rayons déclarés, **capsules**, overlays E/F, mesureur + `docs/reference/fils.md` | ✅ |
| **§9** « le boudin » | Enveloppe **rectangle** pour les plaques (`Noyau::Rectangle`), distances exactes, appliquée aux boucliers ISV (~×10 sur l'épaisseur réservée) | ✅ |
| **Lot 2** (L2.1–L2.5) | Le modèle du `Chantier` interactif : identifiants stables, `retirer`, undo/redo, palette (`posables`/`Categorie`), sérialisation (`recette`/`depuis_recette`) | ✅ |
| **Lot 3** (L3.1–L3.3) | Compléter le modèle pour l'écran : `pose_prevue` (fantôme), `sous_arbre` (surlignage), désignation port/pièce | ✅ |
| **Lot 4** (L4.1–L4.5) | **L'écran d'assemblage** (§8.2–8.4) | ⏸ à faire |
| **Lot 5** | Sauvegarde/chargement disque (L5.1), overlay §8.5 (L5.2) | à faire |
| **Lot 6** | Arbitrage L1.4 (L6.1), composites `figer` (L6.2), `PoutreBout` (L6.3) | à faire |

Le modèle répond maintenant aux quatre questions de §8 — où se poserait la
pièce, ce que Suppr emporte, quel port et quelle pièce sont sous le curseur —
et **aucun écran ne le consomme encore**. C'est l'objet du Lot 4.

Régénérer le catalogue de fils : `FILS=1 cargo test --release le_catalogue_des_fils`.

## À faire — Lot 4, l'écran d'assemblage

**Toute la conception est en `docs/conception/assembleur.md` §10**, qui tranche
ce que §8 laissait ouvert. À lire avant d'écrire une ligne.

| # | Étape |
|---|---|
| L4.1 | Squelette : entrée au menu, zones (§8.2), caméra, boussole, bandeau |
| L4.2 | Palette : catégories, entrées, **grisage** (§10.2) |
| L4.3 | Clic : discrimination clic/glissé (§10.4), désignation (L3.3), pose |
| L4.4 | Fantôme, trois couleurs + **cache** (§10.3), cycle du montage (§10.5) |
| L4.5 | Sélection, surlignage, Suppr, undo/redo au bandeau |

Premier geste de L4.1 : réexporter `Chantier` depuis `vaisseau` (laissé de
côté en L3.3, faute de consommateur).

Les quatre décisions de §10 qui comptent le plus :

- **chantier vide** : cliquer une pièce la pose comme racine, sans viser de
  port — sinon le premier clic ne fait rien sans que rien ne l'explique (§10.1) ;
- **`posables` sert à griser la palette**, pas à la remplir : relever les
  couples `(genre, profil)` **distincts** des ports libres et faire l'union —
  le coût cesse alors de croître avec la station (§10.2) ;
- **l'état vert/rouge des ports se met en cache** : il ne dépend que du
  chantier et de la pièce en main, pas du curseur (seul le fantôme suit la
  souris). Toute mutation passe par **une seule porte** qui invalide (§10.3) ;
- **clic ≠ glissé** : même bouton que l'orbite caméra, donc un seuil en
  pixels, sans seuil de durée (§10.4).

⚠️ Le Lot 4 est le **premier lot majoritairement non testé** (§6.6 : pas de
test de rendu). La discipline change de forme : pousser hors du code de dessin
tout ce qui **se décide**, vers des requêtes red-checkables. Ce qui reste non
testé ne doit être que *où le rectangle se pose*, jamais *ce qu'il signifie*.
Quatre choses sortent et se testent sur ce lot : grisage (L4.2), clic/glissé
(L4.3), invalidation du cache (L4.4), et la désignation (déjà faite, L3.3).

## Décisions prises (ne pas les rouvrir sans raison)

- **Pièce d'abord** (façon KSP) : palette permanente à gauche, ports compatibles
  qui s'allument. Pas « port d'abord ».
- **Bac à sable libre** : pas de plafond de coût.
- **Capsules** (et rectangles pour les plaques) plutôt que sphères — et **pas**
  d'AABB/OBB.
- **Symétrie non recherchée partout** : la grammaire pose ce qui tient, les
  groupes ne sont pas atomiques.
- **`SousEnsemble` se sérialise en pièces cuites**, pas en recette rejouée
  (performance + immunité aux retouches futures de géométrie).
- **Pas de roulis libre** autour du port : `accoupler` le fixe, et l'ajouter
  toucherait `Etape`, donc le format de sauvegarde (§10.6).

## Questions ouvertes

- **Sortir de l'assembleur perd le vaisseau** (§10.7b) : `main.rs` reconstruit
  un `Accueil` et détruit la vue, donc Échap efface le travail en cours. Trois
  issues — garder la vue vivante dans `Etat` (une ligne, mais le vaisseau
  survit alors à un aller-retour au menu sans que rien ne le dise), demander
  confirmation (le projet n'en a nulle part), ou assumer la perte jusqu'à la
  sauvegarde du Lot 5. **À trancher avant L4.1.**

## Méthode (ce qui a marché, à ne pas perdre)

- **Red-check systématique** : tout test ajouté est cassé volontairement sur sa
  cible pour vérifier qu'il rougit. 56 sabotages à ce jour (17 sur le Lot 1,
  5 sur §9, 23 sur le Lot 2, 11 sur le Lot 3), tous rougis au
  final. C'est le seul filet qui ait trouvé de vrais défauts. Trois formes du
  même piège sont revenues — un test qui **inspecte l'état** au lieu
  d'emprunter le vrai chemin de code, qui **fixe une variable** du scénario au
  lieu de la sonder (typiquement `libres()[0]`, revenu en L2.1, L2.5 puis
  L3.3), ou qui ne compare que **les deux bouts** d'une séquence (L2.3). La
  parade est toujours la même : muscler le test dès qu'un sabotage ne mord pas,
  jamais l'admettre « suffisamment proche ».
- ⚠️ **Un test rouge ne veut pas dire que le code a tort** (L2.4) : vérifier
  lequel des deux se trompe avant de corriger.
- ⚠️ **Une propriété d'accord entre deux appelants d'un même calcul ne garde
  pas ce calcul — elle garde son unicité** (Lot 3). Dès qu'on met en facteur
  (`corps_prevu`, `sous_arbre`, `DEMI_FOV`), il faut un **second** test qui
  pinne la valeur elle-même.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent
  d'une valeur qui en a deux (`docs/suivi/stations.md` §C.29).
- **Mesurer avant d'affirmer.** Deux corrections du mesureur ont été annulées
  parce qu'elles contredisaient un test existant sur la même grandeur.
- **Mettre à jour les `.md`** à chaque étape — consigne debout de l'utilisateur.

## Dettes préexistantes, sans rapport

- **Le hash CPU/GPU** (traité le 2026-08-02) : `cargo clippy` ne compilait pas
  à cause d'`approx_constant` (refusé par défaut) sur le multiplicateur du hash
  de `terrain.rs`. **Suivre le conseil de clippy aurait introduit un défaut** —
  `FRAC_1_PI` décale la valeur d'**1 ULP**, et le `fract` final de `hash`
  amplifie ça en un bruit entièrement différent : la géographie CPU cesserait
  de correspondre au shader, sans aucun signal. Constante nommée
  (`DECORRELATION_HASH`) et `allow` porté dessus avec l'explication.
  **Dette restante** : la même valeur vit dans 4 fichiers (1 Rust + 3 GLSL) et
  rien ne les tient d'accord → `docs/suivi/planetes.md` §7.
- Les 75 avertissements clippy restants ne contiennent **aucun défaut réel** :
  ~54 sont les `dead_code` ci-dessous, le reste est du style
  (`too_many_arguments`, `doc_lazy_continuation`, `needless_range_loop`…).
- Avertissements `dead_code` de fond : les méthodes du noyau rectangle (L1.6),
  la moitié de l'API de `Chantier` (`retirer`/`annuler`/`refaire`/`peut_poser`/
  `compatibles`/`figer`/`recette`/`depuis_recette`/`pose_prevue`/`sous_arbre`/
  `piece_sous_rayon`), `Categorie`/`posables`, et `ecran::designation` — tout
  attend l'écran du Lot 4.
- Dette de **L1.4** : ~19 variantes sous-déclarent leur `rayon_local` (cadrage
  caméra ; mesuré : 2 vues sur 35 bougeraient, de 1 % et 2 %).
  `MARGE_RAYON = 1.40` marque la dette dans le code. **Se solde en Lot 6**,
  une fois l'overlay de L5.2 capable de la montrer à l'œil.
- Dette de **L1.6** : `peut_poser` n'a aucun test dédié, reporté trois fois
  faute de consommateur réel. Son vrai consommateur est le code couleur de
  §8.4 — donc **Lot 4**.
- `GenrePort::PoutreBout` n'est posé sur aucun composant actuel (trouvé en
  L2.4) — variante d'enum morte, **Lot 6**.
