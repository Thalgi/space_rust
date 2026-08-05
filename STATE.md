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

État vérifié : **411 tests verts**, **`cargo clippy` : 0 erreur**.

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

## La vue de départ : le système solaire

La `Skymap` ouvre désormais sur **le preset solaire**, et non plus sur une
graine procédurale : depuis qu'elle porte l'interface de jeu, elle doit ouvrir
sur un lieu connu — on y reconnaît les planètes, on y juge les distances. `G`
tire toujours un système au hasard.

Remis au niveau au passage :

- **Anneaux d'Uranus** (monobande bleu ciel) et **de Neptune** (arcs), ajoutés
  au **catalogue** et non au preset : la galerie et la vue système lisent la
  même apparence.
- **Quatre familles de petits corps** au lieu de deux — ceinture principale,
  Kuiper, **disque épars** (orbites excentriques et inclinées) et **nuage de
  Oort** (coquille sphérique, pas un disque). Toutes existaient déjà dans
  `DisqueConfig` ; le preset n'en utilisait que la moitié.
- **L'ISS en orbite terrestre** (`src/engin.rs`) : le pont entre les deux
  moitiés du projet — la station est assemblée par `vaisseau::preset_iss`, cuite
  en maillage, puis mise en orbite comme une lune. Nouvelle
  `Categorie::Engin`.

⚠️ **L'échelle de l'ISS est fausse, et c'est inévitable.** À l'échelle réelle
elle ferait 9 × 10⁻⁶ unité de monde — mille fois moins qu'un pixel. Les facteurs
sont **nommés** (`ORBITE_ISS`, `ECHELLE_ISS`) plutôt que noyés dans un calcul :
un mensonge d'échelle assumé vaut mieux qu'un mensonge implicite.

## Paramètres du jeu

Écran **PARAMETRES** depuis l'accueil (`src/ecran/parametres.rs`), modèle dans
`src/reglages.rs`. Six réglages, et la place est faite pour les suivants : les
entrées sont une **liste**, en ajouter une tient en une ligne.

Un bouton **QUITTER** est posé sous PARAMETRES sur l'accueil. Il sort de la
boucle de jeu (`return`) plutôt que d'appeler `process::exit`, pour que miniquad
ferme sa fenêtre proprement. ⚠️ Sa géométrie est **calculée en ligne dans
`accueil.rs` et n'est donc pas testée** — comme tout le reste de cet écran ; le
non-recouvrement avec PARAMETRES tient à la construction (52 px d'écart pour
40 px de haut).

⚠️ **Deux modes d'affichage, pas trois.** macroquad n'expose qu'un
`set_fullscreen(bool)`, et miniquad l'implémente sur Windows en passant la
fenêtre en `WS_POPUP` à la taille de l'écran (`native/windows.rs`) — c'est
**déjà** un plein écran sans bordure. Il n'existe pas de mode exclusif
(changement de mode vidéo). Proposer « plein écran » et « sans bordure »
séparément donnerait deux boutons au comportement identique.

Tailles : 9 entrées en 4:3, 16:10 et 16:9, de 1024 × 768 à 1920 × 1200.
**Pas de 4K**, comme demandé — et le rendu est en impostors plein écran, donc le
coût monte comme le nombre de pixels.

**Dette D-PARAM-1** : les réglages ne sont **pas sauvegardés** — ils repartent
en fenêtré 1280 × 720, rendu net, à chaque lancement. `genese/persistance.rs`
sait déjà écrire du JSON dans le dossier du jeu ; c'est là que ça se brancherait.

### Rendu pixel art (PARAMETRES → RENDU / PALETTE / TRAMAGE)

Trois états de rendu : `NET`, `PIXEL ART` (gros pixels, ce qui existait déjà sous
la touche **P**), `PIXEL ART + PALETTE` (quantification CIELAB). Trois réglages
s'ajoutent en mode palette, grisés sinon : **palette**, **tramage** et
**saturation**. Conception complète, corrections apportées au guide d'origine et
dettes : [`docs/conception/pixel_art.md`](docs/conception/pixel_art.md).

**Ajouter une palette** : déposer un `.hex` (format Lospec, un hexa par ligne)
dans [`assets/palettes/`](assets/palettes/) — ramassé au démarrage, aucun code à
toucher. Trois palettes intégrées : Resurrect 64, Sweetie 16, PICO-8.

**Rien n'est refusé en silence.** Un `.hex` que le jeu n'a pas pu charger est
listé **à l'écran**, dans PARAMETRES, en ambre, avec son nom de fichier et la
raison (`palette::rejets`). C'est la réponse à la classe d'erreur qui a coûté le
plus cher cette session : deux palettes déposées n'apparaissaient jamais au menu
et la seule trace partait dans une console que personne ne lit.

La décision (quoi dire, combien en détailler) vit dans `lignes_de_rejet`, séparée
du dessin et **testée** ; la lecture disque (`lire_dossier`) ne décide de rien, et
le tri (`trier`) est pur — c'est ce qui rend l'ensemble testable sans contexte
graphique.

⚠️ **Plafond : 256 couleurs** (`palette::MAX`), relevé depuis 64 — Lospec 2000
(182) et AllStars (128) étaient **rejetées puis ignorées**, avec pour seule trace
une ligne de console. Le coût de rendu monte avec le nombre de couleurs : la
recherche parcourt toute la palette **par pixel**.

⚠️ **Une palette d'artiste n'est pas une rampe de dégradé.** Mesuré sur
Resurrect 64 : un dégradé de gris ne tombe que sur **8 couleurs**, avec une
marche de L=49 à L=69, et tout ce qui dépasse 89 % s'écrase sur le blanc. Trois
défauts constatés à l'écran en découlent, chacun avec son remède mesuré :

| Défaut vu | Cause mesurée | Remède |
|---|---|---|
| Bandes qui basculent d'un bloc | marche de 0,18 en gris | **tramage de Bayer**, `FORT` = 0,18 |
| Reflet en aplat blanc | spéculaire additif > 1,0 | **écrêtage** (cœur blanc, halo non) |
| Couleurs ternes, océans gris-violet | les **neutres** de la palette attirent les couleurs peu saturées | **saturation** ×1,9 à luminance constante (+53 % de chroma) |

Deux fausses pistes écartées par la mesure, à ne pas re-explorer : la recherche
CIELAB garde déjà 85 % de la chroma (pondérer L n'y change rien), et le tramage
ne désature pas.

⚠️ L'écrêtage n'agit **que dans la passe pixel art** : le spéculaire de
`planete.frag.glsl` n'est pas touché, le modifier changerait le rendu `NET` et
donc la galerie, les presets et les captures de non-régression.

Le mode est **global** (`reglages::etat_rendu`), pas par vue : les touches P et
le menu pilotent le même état, et P ne change que le mode (ni la palette ni le
tramage). `ecran/pixel.rs` est devenu la **source unique du blit** — les quatre
vues y passent, ainsi que le facteur `PIX` et la création de cible qui étaient
recopiés trois fois.

⚠️ **Seule la couche 3D est quantifiée.** Fond stellaire, orbites et textes
restent nets, comme c'était déjà la règle.

⚠️ **D-PIX-1** : le miroir CPU testable (`palette.rs`) et le shader
(`palette.frag.glsl`) sont deux écritures du même algorithme ; aucun test ne
compile de GLSL, donc rien ne garantit qu'ils restent d'accord.

## Le système solaire et le catalogue

Le preset solaire (`genese/presets.rs`) tire **déjà** ses apparences du
catalogue de la galerie (`preset_tellurique` / `preset_gazeuse`, qui **paniquent**
sur un nom inconnu — il n'y a pas de repli silencieux). Deux corrections le
2026-08-05 :

**1. Le catalogue est devenu déterministe** — c'était la cause racine.

`catalogue_telluriques()` / `catalogue_gazeuses()` tiraient une graine **et** une
taille au sort à chaque construction. Conséquence : la « Terre » de la galerie
n'était pas une référence, mais une planète différente à chaque ouverture de
l'écran. Le système solaire ne pouvait donc **pas** lui ressembler — il n'y avait
rien à quoi ressembler.

Graine et taille sont maintenant déduites du **nom du preset**
(`genese::graine_de_nom`, FNV-1a, et `ClasseTaille::rayon_pour`). Le brassage n'a
pas disparu : il a changé de place, dans la galerie, où la touche **G**
incrémente une `variation`. À `variation == 0`, le décalage est nul — **la vue
par défaut de la galerie EST le catalogue canonique**, ce qui est ce à quoi le
système solaire s'aligne.

Les corps à preset **unique** prennent donc la graine du catalogue et sont
identiques à leur vignette de galerie. `fige(…, "<corps>")` ne subsiste que là où
un preset est **réutilisé** (Callisto et Obéron tirent tous deux « Lune »), sans
quoi ils seraient rigoureusement jumeaux.

**Gain de côté** : sans `gen_range`, le catalogue **se teste enfin**. Cinq tests
neufs, dont « chaque preset demandé par les systèmes scénarisés existe » — jusqu'ici
garanti par un seul `panic!` au lancement, écran noir à la clé.

**2. Appariements revus**, pour cesser de recycler le même preset :

| Corps | Avant | Après | Pourquoi |
|---|---|---|---|
| Ganymède | `Lune` | `Crevasse` | ses sillons (sulci) |
| Callisto | `Carbone` | `Lune` | le corps le plus cratérisé connu |
| Ariel | `Boule de neige` | `Supraglacial` | terrains fracturés brillants |
| Pluton | `Boule de neige` | `Ice Dunes` | dunes vues par New Horizons |

⚠️ **Fixtures recopiées** : les listes de noms des tests (`DEMANDES_TELLURIQUES`,
`DEMANDES_GAZEUSES`, `CORPS`) sont recopiées de `presets.rs`. Un nom ajouté là-haut
sans l'être ici est **moins couvert**, il ne devient pas faux.

⚠️ **Piège évité de justesse** : `chaque_taille_reste_dans_les_bornes_de_sa_classe`
déduisait d'abord ses bornes de `rayon_pour`, la fonction même qu'il teste —
l'attendu bougeait avec elle et le test ne pouvait pas échouer. Il lit maintenant
`bornes_terrestres()` à la source. C'est le mode de défaillance récurrent du
projet : *un test qui mesure autre chose que ce que son nom annonce*.

⚠️ **Bug de LOD corrigé** (`rendu.rs`) : `planete::set_viewport_h` n'était réglé
**que par la galerie**. Dans la vue système, `px_rayon` retombait sur
`screen_height()` — donc en mode pixel art, les planètes étaient ombrées avec le
détail d'un plein écran alors qu'elles sont dessinées dans une cible deux fois
plus petite. Les débris avaient déjà leur correction, pas les planètes.

## Cohérence du catalogue de planètes

Passe du 2026-08-05, déclenchée par « Mercure a de la glace ». **Trois défauts,
dont deux dans le shader** — la donnée, elle, était déjà cohérente.

| Défaut | Où | Cause |
|---|---|---|
| Banquise sur des mondes sans climat (Mercure, Lune, Carbone, Vénus) | `planete.frag.glsl` | `calotte = 1` était censé dire « aucune », mais le seuil est perturbé par un bruit de ±0,30 et `froid` atteint déjà 1 au pôle : `smoothstep(1, 1.05, 1.5)` rendait 1 |
| Lumières de ville sur les cailloux morts | `apparence.rs` + shader | `villes` vaut **1 par défaut** et le shader n'excluait que lave et voile — aucune condition d'air ou d'eau |
| (faux positif) Rivières sans eau sur `Crevasse` | — | ce sont des **coulées de lave** (`riv_lave`) : c'est le test qui était faux |

`calotte >= 1` est maintenant un **sentinelle exact** dans le shader, et
« ni air ni eau ⇒ pas de villes » est appliqué **dans la donnée** (le `push` du
catalogue), là où ça se teste.

Six tests de cohérence neufs, tous red-checkés. Ils tiennent des invariants, pas
des valeurs : *pas de climat ⇒ pas de banquise*, *un monde annoncé gelé porte
vraiment de la glace*, *un monde habitable garde ses villes* (sans quoi une règle
trop large les éteindrait partout sans qu'on le voie), *une rivière coule d'eau
ou de lave*.

⚠️ Ces tests ne sont possibles que **depuis que le catalogue est déterministe** :
avant, il exigeait le contexte graphique.

## Revue de code du 2026-08-05 — liste à traiter

Établi en passant tout le catalogue de planètes au crible et en dépouillant
`cargo clippy`. **Par ordre d'urgence.**

### P0 — défauts réels

| # | Quoi | État |
|---|---|---|
| 1 | Banquise sur les mondes sans climat (`calotte = 1` n'était pas un sentinelle exact) | ✅ corrigé |
| 2 | Villes sur les cailloux morts (`villes = 1` par défaut, shader sans condition d'air) | ✅ corrigé |
| 3 | **Titan sans atmosphère** : voile orange épais mais `atmo = 0`, donc aucun halo de limbe | ✅ corrigé |
| 4 | Géantes gazeuses avec `villes = 1` et `relief = 0,35` — inertes (gardées par `type_p`), latentes | ✅ corrigé |
| 5 | `large_enum_variant` sur `ecran/objet.rs:14` — `Apparence` (376 o) contre 48 o pour la variante étoile : toute copie de `Specimen` traînait les 376 o. Boxée. | ✅ corrigé |

### P1 — le bruit qui cache les vrais défauts

C'est **la leçon de la session** : la palette Lospec était refusée avec une ligne
de console, invisible au milieu du bruit. Tant que `clippy` sort 117
avertissements, le 118ᵉ ne se voit pas.

| # | Quoi | Volume | État |
|---|---|---|---|
| 6 | Imports inutilisés | 9 | ✅ |
| 7 | Code mort (méthodes, fonctions, constantes jamais utilisées), surtout dans `vaisseau/` | **55** | ⏳ |
| 8 | `assertions_on_constants` — **légitimes** : elles gardent une constante, l'abaisser fait échouer le test. `#[allow]` commenté sur les 4 modules de test concernés | 7 | ✅ |
| 9 | `field_reassign_with_default` (tests), `needless_range_loop`, `useless_conversion`, `needless_update` | ~15 | ⏳ |

**Avertissements : 117 → 105.**

⚠️ **Le code mort n'est PAS un artefact de test.** Mesuré : 55 éléments morts
pour la cible binaire, 57 avec les tests — donc il est bien inutilisé, pas
seulement « utilisé ailleurs ». Le supprimer demande du **jugement**, pas un
passage automatique : une partie est de l'API posée d'avance pour l'assembleur
(Lot 4), et l'effacer coûterait plus cher que le bruit qu'elle fait. À trier
élément par élément, pas en masse.

⚠️ **Trois réexportations étaient utilisées par les seuls tests** (`mesurer`,
`silhouette`, `a_des_mailles`, plus `hexagone_ceinture`) : appelées par chemin
complet depuis `composant/mod.rs`, donc invisibles au binaire. Elles sont
désormais `#[cfg(test)]` plutôt que supprimées — les retirer cassait la
compilation des tests.

### P2 — dettes déjà inscrites

D-PARAM-1 (réglages non sauvegardés), D-PIX-1/3/4/5, `briques.rs`/`vaisseaux.rs`
non déclarés dans `ecran/mod.rs`, hash CPU/GPU dupliqué dans 4 fichiers.

### Ce que la revue a appris

**Deux de mes trois règles candidates étaient trop strictes, et le catalogue avait
raison.** Les rivières de `Crevasse` sont des coulées de **lave** (`riv_lave`), et
la végétation de `Lichen` et `Fungi` prend son humidité dans l'**air**, pas dans
un océan. Ce sont des cas limites délibérés. Les tests encodent désormais la
règle juste — *une rivière coule d'eau ou de lave*, *la végétation veut de l'eau
ou une atmosphère* — et chacun vérifie que **le cas limite existe encore**, sans
quoi la règle deviendrait vide sans que rien ne le dise.

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

**Quatre sur cinq sont soldés.** Il ne reste que D-INT-2, l'économie, qui est un
chantier de conception à part entière.

| # | Bouche-trou | Où | Ce qu'il faut à la place |
|---|---|---|---|
| **D-INT-1** | ~~Pastille de catégorie~~ — **soldée** : la pastille prend la **teinte réelle** du corps (`Astre::teinte`, tirée de l'apparence), relevée pour rester visible sur fond nuit. Une retouche d'apparence se voit aussitôt dans la colonne. Reste un disque, et c'est voulu : à 6–14 px de diamètre, un rendu de planète serait de la bouillie. | — | — |
| **D-INT-2** | **`Tresorerie` figée** : les quatorze quantités sont écrites à la main, rien ne les fait bouger. | `ecran/bandeau.rs::Tresorerie` | L'économie : production, consommation, coûts, recherche. Chantier de **conception** à part entière ; la barre a été écrite pour pouvoir l'attendre (`conception/interface.md` §2.2b). |
| **D-INT-3** | ~~Vignette du panneau~~ — **soldée** : l'astre est **réellement rendu** (`ecran/vignette.rs`) dans une cible 192×192 avec son propre tampon de profondeur, sous l'éclairage du système, caméra placée du côté éclairé et reculée pour laisser tenir un anneau. | — | — |
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
