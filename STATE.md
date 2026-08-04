# STATE — où en est le chantier

> Point de reprise, **2026-08-04**. À lire en premier dans une nouvelle session.
> Ce fichier dit **où on en est et quoi faire ensuite**, rien de plus : le
> détail de chaque étape (ce qui a été trouvé, les red-checks, les mesures)
> vit dans les journaux de [`docs/suivi/`](docs/suivi/), et le *pourquoi* dans
> [`docs/conception/`](docs/conception/).

## Commits

Dernier commit : `f843f9c` « Safe commit before GUI ».

Consigne debout : *« no commit i'll say when commit is needed »* — ne jamais
commiter de soi-même. **Mais signaler les points de commit** : l'utilisateur
commite aux jalons qu'on lui indique, depuis qu'un `git checkout --` a détruit
du travail non commité (`docs/suivi/stations.md` §F.8).

⚠️ **Ne jamais employer `git checkout --`, `git restore` ou `git reset --hard`**
sur un fichier contenant du travail non commité. Pour annuler ses propres
modifications : ré-éditer les lignes, ou sauvegarder le fichier dans le
scratchpad **avant** de le toucher (red-checks compris).

État vérifié : **339 tests verts** (`cargo test --release`, ~0,8 s),
**`cargo clippy` compile** (0 erreur).

## Chantier courant : l'interface de jeu

**Les six étapes I.0 à I.5 sont faites** — reste à juger le tout à l'écran, et à
solder les bouche-trous ci-dessous.

Conception écrite le 2026-08-04 :
**[`docs/conception/interface.md`](docs/conception/interface.md)**. À lire avant
d'écrire une ligne. C'est la première interface **de jeu** du projet — tout ce
qui existe aujourd'hui est de l'outillage de développement.

Écran visé : la **Skymap** (vue système), pas la Starmap.

| # | Étape | État |
|---|---|---|
| I.0 | Chargement des sprites (atlas, `FilterMode::Nearest`, repli) | ✅ |
| I.1 | Noms d'astres (presets seulement ; numérotation orbitale en repli) | ✅ |
| I.2 | Sélecteur de planètes à gauche, rétractable | ✅ |
| I.3 | Panneau de planète au clic | ✅ |
| I.4 | Barre de ressources + nom du système | ✅ |
| I.5 | Agrégation `sur_ui` (une seule porte) | ✅ faite au fil des étapes |

**Trois décisions déjà prises** (§2.2, §4.1) : ressources en **affichage seul**
(`Tresorerie` figée, pas d'économie), noms d'astres **sur les presets
seulement**, clic sur une planète **centre la caméra** comme aujourd'hui.

**Cinq questions ouvertes** en §5.1 — l'icône des crédits (aucun sprite),
quelles ressources dans la barre (14 sprites existent, 5 tiennent), où vit le
nom, comment se ferme le panneau, les lunes dans le sélecteur.

Deux acquis à ne pas réécrire : **`ecran::liste` *est* le sélecteur** (colonne
bornée, hauteur de ligne adaptative, item sous le curseur — rien en lui ne
connaît les composants), et **l'habitabilité se déduit** de
`etoile::zone_habitable` au lieu de se stocker.

## Chantiers en pause

| Chantier | Où on en est | Reprendre par |
|---|---|---|
| **Assembleur** (Lot 4) | Lots 1–3 clos, Lot 4 conçu, pas commencé | `conception/assembleur.md` §10, puis L4.1 |
| **Starship** (2/5) | Coque et Raptor faits et validés | Poser les 6 moteurs (3 RSL sur 0,489 U, 3 RVac sur 1,378 U), puis volets, bouclier, hublot |
| **Endurance** | Rien | `conception/stations.md` Partie D bis |

## L'assembleur, en résumé

L'ISV est **clos** (`docs/suivi/stations.md` §C.29). Les mégastructures et le
Starship sont journalisés en **Partie F** du même fichier.

| Lot | Contenu | État |
|---|---|---|
| **Lot 1** (L1.1–L1.8) | Fondations : catalogue à source unique, balayage des variantes, déterminisme, rayons déclarés, **capsules**, overlays, mesureur | ✅ |
| **§9** « le boudin » | Enveloppe **rectangle** pour les plaques, distances exactes | ✅ |
| **Lot 2** (L2.1–L2.5) | Modèle du `Chantier` : identifiants stables, `retirer`, undo/redo, palette, sérialisation | ✅ |
| **Lot 3** (L3.1–L3.3) | `pose_prevue` (fantôme), `sous_arbre`, désignation port/pièce | ✅ |
| **Lot 4** (L4.1–L4.5) | **L'écran d'assemblage** | ⏸ conçu, pas commencé |
| **Lot 5** | Sauvegarde disque (L5.1), overlay §8.5 (L5.2) | à faire |
| **Lot 6** | Arbitrage L1.4 (L6.1), composites `figer` (L6.2), `PoutreBout` (L6.3) | à faire |

Le modèle répond aux quatre questions de §8 et **aucun écran ne le consomme
encore**. Premier geste de L4.1 : réexporter `Chantier` depuis `vaisseau`.

⚠️ Le Lot 4 est le **premier lot majoritairement non testé** (§6.6 : pas de test
de rendu) — comme l'interface de jeu. La discipline change de forme : pousser
hors du code de dessin tout ce qui **se décide**, vers des requêtes
red-checkables. Ce qui reste non testé ne doit être que *où le rectangle se
pose*, jamais *ce qu'il signifie*.

Régénérer le catalogue de fils : `FILS=1 cargo test --release le_catalogue_des_fils`.

## Décisions prises (ne pas les rouvrir sans raison)

- **Pièce d'abord** (façon KSP) : palette permanente à gauche, ports compatibles
  qui s'allument. Pas « port d'abord ».
- **Bac à sable libre** : pas de plafond de coût.
- **Capsules** (et rectangles pour les plaques) plutôt que sphères — et **pas**
  d'AABB/OBB.
- **`SousEnsemble` se sérialise en pièces cuites**, pas en recette rejouée.
- **Pas de roulis libre** autour du port (§10.6).
- **Les centres de Stanford et d'Elysium sont séparés** : ils ne sont pas
  destinés à se ressembler (§F.4).
- **`Composant::nom()` rend la famille**, pas famille + variante : la colonne
  fait 100 px, et la vitrine fait déjà défiler les variantes.

## Questions ouvertes

- **Sortir de l'assembleur perd le vaisseau** (§10.7b) : Échap efface le travail
  en cours. **À trancher avant L4.1.**
- **Les mégastructures à taille réelle** : ~2,85 pièces et ~2 850 sommets par
  unité de rayon d'anneau, soit ~1 140 pièces pour l'Elysium de 1,8 km contre 45
  pour l'ISV (§F.3). L'assemblage pièce à pièce ne passera pas à l'échelle.
- **Membrures à axe courbe** : aucune primitive du projet n'en trace, et les
  bras des mégastructures en ont besoin pour être reliés entre eux (§F.1).

## Méthode (ce qui a marché, à ne pas perdre)

- **Red-check systématique** : tout test ajouté est cassé volontairement sur sa
  cible pour vérifier qu'il rougit. **Sauvegarder le fichier dans le scratchpad
  avant de le saboter**, et le restaurer par copie — jamais par git.
  Trois formes du même piège reviennent : un test qui **inspecte l'état** au
  lieu d'emprunter le vrai chemin de code, qui **fixe une variable** du scénario
  au lieu de la sonder (`libres()[0]`), ou qui se compare à **sa propre
  constante** (§F.7). La parade est toujours la même : muscler le test dès qu'un
  sabotage ne mord pas, jamais l'admettre « suffisamment proche ».
- ⚠️ **Un test rouge ne veut pas dire que le code a tort** (L2.4).
- ⚠️ **Un proxy qui ne tient que dans un régime n'est pas une mesure** (§F.2).
- ⚠️ **Une propriété d'accord entre deux appelants d'un même calcul ne garde pas
  ce calcul — elle garde son unicité.** Il faut un **second** test qui pinne la
  valeur.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent
  d'une valeur qui en a deux (`docs/suivi/stations.md` §C.29).
- **Une cote qui se règle contre une autre pièce ne tombe pas sur `Profil`** —
  vu quatre fois. Réponse : mise à l'échelle **géométrique**, ou rester sur la
  grille et s'arrêter au cran qui tient (§F.4).
- **Brique d'abord, assemblage ensuite.** Jugée seule avant d'être posée.
- **Mesurer avant d'affirmer.**
- **Mettre à jour les `.md`** à chaque étape — consigne debout de l'utilisateur.

## Dette des bouche-trous de l'interface

Consigne de l'utilisateur (2026-08-04) : **poser un bouche-trou** pour ce qui
n'est pas encore fait, et **inscrire ici** ce qu'il reste à faire. Un
bouche-trou non listé est un mensonge à l'écran — il a l'air fini.

| # | Bouche-trou | Où | Ce qu'il faut à la place |
|---|---|---|---|
| **D-INT-1** | **Pastille de couleur unie** au lieu d'une vignette d'astre. La teinte ne dit que la catégorie (étoile / planète / lune) : deux planètes se ressemblent. | `ecran/selecteur.rs::teinte` | Une miniature rendue de l'astre — le schéma en montre une. Suppose un rendu hors-écran vers une texture, et un cache (une par astre, pas une par frame). |
| **D-INT-2** | **`Tresorerie` figée** : les quatorze quantités sont écrites à la main, rien ne les fait bouger. | à venir, étape I.4 | L'économie : production, consommation, coûts, recherche. C'est un chantier de conception à part entière, et la barre a été écrite pour pouvoir l'attendre (`conception/interface.md` §2.2b). |
| **D-INT-3** | **Vignette du panneau** : disque uni, comme la pastille de la colonne. Le panneau lui-même est fait (nom, type, distance, rayon, habitabilité déduite). | `ecran/skymap.rs::fiche_dessiner` | Même besoin que D-INT-1 : un rendu miniature de l'astre. Les deux se règlent ensemble. |
| **D-INT-4** | ~~Nom du système~~ — **soldée** : dérivé de l'étoile hôte (`Systeme::nom_systeme`). Repli sur le libellé de génération pour un système engendré, dont les étoiles n'ont pas de nom propre. | — | — |
| **D-INT-5** | ~~Noms de presets non gardés~~ — **soldée** : `ajouter_planete`/`ajouter_planete_autour` exigent un `Option<&'static str>`, comme `ajouter_lune_preset`. Chaque appelant doit dire `Some("Terre")` ou `None`. Deux tests lisent la **source** pour interdire le retour du `sys.nommer` après coup — seul moyen, faute de pouvoir bâtir un système en test. | — | — |

## Dettes préexistantes

- **Le hash CPU/GPU** (traité le 2026-08-02) : `cargo clippy` ne compilait pas à
  cause d'`approx_constant` sur le multiplicateur du hash de `terrain.rs`.
  **Suivre le conseil de clippy aurait introduit un défaut** — `FRAC_1_PI`
  décale la valeur d'**1 ULP**, et le `fract` final amplifie ça en un bruit
  entièrement différent : la géographie CPU cesserait de correspondre au shader,
  sans aucun signal. Constante nommée + `allow` avec l'explication.
  **Dette restante** : la même valeur vit dans 4 fichiers (1 Rust + 3 GLSL) et
  rien ne les tient d'accord → `docs/suivi/planetes.md` §7.
- Les 86 avertissements clippy ne contiennent **aucun défaut réel** : l'essentiel
  est du `dead_code` en attente d'un consommateur (l'API de `Chantier`,
  `ecran::designation`, les `nom()` de variantes utilisés en test seulement), le
  reste est du style.
- **`src/ecran/briques.rs` et `src/ecran/vaisseaux.rs` sont commités mais pas
  déclarés** dans `ecran/mod.rs` : ils ne compilent pas. À supprimer ou à
  rebrancher.
- Dette de **L1.4** : ~19 variantes sous-déclarent leur `rayon_local` (cadrage
  caméra ; 2 vues sur 35 bougeraient, de 1 % et 2 %). `MARGE_RAYON = 1.40`
  marque la dette dans le code. **Se solde en Lot 6.**
- Dette de **L1.6** : `peut_poser` n'a aucun test dédié, faute de consommateur.
  Son vrai consommateur est le code couleur de §8.4 — donc **Lot 4**.
- `GenrePort::PoutreBout` n'est posé sur aucun composant — variante morte,
  **Lot 6**.
