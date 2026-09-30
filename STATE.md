# STATE — où en est le chantier

> Point de reprise, **2026-08-14**. À lire en premier dans une nouvelle session,
> **en entier** : il tient en deux cents lignes, et c'est délibéré.
>
> Ce fichier dit **où on en est et quoi faire ensuite**, rien de plus. Le
> *récit* de chaque étape — ce qui a été trouvé, les mesures, les red-checks —
> vit dans les journaux de [`docs/suivi/`](docs/suivi/) ; le *pourquoi* de
> chaque système dans [`docs/conception/`](docs/conception/) ; la *discipline*
> dans [`docs/methode.md`](docs/methode.md).
>
> ⚠️ **Il avait dérivé** jusqu'à 1 968 lignes en devenant le journal de tous les
> chantiers à la fois — donc illisible au démarrage, précisément quand on en a
> besoin. Vidé le 2026-08-10 : rien n'a été perdu, tout a été **déplacé** (voir
> « Où trouver quoi »). **Ne pas recommencer** : un fait qui se raconte va dans
> un journal, pas ici.

## Reprendre ici (état au 2026-09-26)

**Chantier en cours : Astres V2.** Branche `chantier-astres`, worktree
`.claude/worktrees/astres-v2`. **32 commits d'avance sur `main`, rien de
fusionné, rien de poussé.**

**Vérifié (2026-09-30)** : **855 tests verts** (867 avec le binaire `mhd`),
`cargo clippy --all-targets` à **0 avertissement**.

**Le lot 5 est clos** (2026-09-30). Ses **sept presets de trou noir sont au
tirage** : l'utilisateur a préféré garder la variété plutôt que choisir une
cible unique.

Le 5.5 a séparé deux axes qui étaient confondus — l'**appétit** (`nom` :
endormi / actif / vorace) et la **cible** (`cible` : chaud / Interstellar /
braise). Le catalogue est leur **produit**. Le tirage pondère un appétit
(0,35 / 0,45 / 0,20) puis **partage ce poids entre ses cibles** : ajouter une
teinte change la variété, jamais la fréquence d'un appétit.

**Le lot 6 est livré**, **à juger à l'écran** : supergéante rouge, sous-naine
sdB, étoile carbonée et **naine brune** au tirage.

- La carbonée n'était pas de la simple donnée : en corps noir elle serait
  *moins* rouge qu'une naine M froide, d'où un **filtre d'absorption
  moléculaire** (`filtre_teinte`).
- La table du tirage est sortie dans `etoile::ordinaire(p, q)` — elle vivait
  dans une chaîne de `if` qu'aucun test ne pouvait atteindre, comme le trou noir
  avant le lot 4.
- La **naine brune** est le seul type qui n'est **pas une photosphère** : elle
  se rend en géante gazeuse. `etoile::est_naine_brune` le dit, et **tout
  bâtisseur de corps doit le consulter** (trois le font).
- La **vue isolée** (touche `3`) fait maintenant défiler aussi les types
  ordinaires. ⚠️ Elle ne juge pas les **tailles** — `dist_pour` cadre chaque
  corps pour remplir l'écran ; le rayon se lit en chiffres dans le debug.
- La **planche des étoiles** porte les **quatre classes** de naine brune (M, L,
  T, Y). Ses cellules sont désormais des `Box<dyn Astre>` : le type était plus
  étroit que l'usage, la boucle n'appelant que `update` et `draw`.

⚠️ Une naine brune au centre d'un système joue un **rôle** stellaire, mais ce
n'est **pas** la quasi-étoile du lot 11 (enveloppe de milliers de masses
solaires chauffée par accrétion, large comme une UA). Le lot 11 reste entier.

**Le lot 7 est livré**, **à juger à l'écran** : la **pulsation**, donc céphéide,
Mira et naine blanche pulsante (ZZ Ceti) d'un coup.

- `etoile::pulsation(nom) -> (amplitude, période)`. La luminosité n'a **pas** de
  réglage à elle : `L ∝ R²` à température constante, elle suit le carré — donc
  la variation d'éclat se voit **à toute distance**.
- ⚠️ La pulsation **n'est pas un mode de couronne** : `Variante::appliquer` prend
  désormais le **nom** du type en plus du mode, et `Feuille` porte ce nom. Sans
  ça, il aurait fallu l'écrire dans cinq endroits.
- Le piège annoncé (la cible de clic) n'en était pas un : la pulsation passe par
  `rayon_dessine`, donc elle bouge la cible **avec** le disque peint, et les 40 %
  de marge de `Systeme::pick` absorbent le reste.
- **Amplitudes réelles** pour la céphéide (10 %) et Mira (20 %) ; ZZ Ceti est
  exagérée et c'est assumé, comme `PX_MIN`. Les **périodes** sont compressées.

**Le lot 8 est engagé, et tranché** : lentille **locale**, fond échantillonné
depuis une **cubemap gravée du champ d'étoiles existant** (décision de
l'utilisateur, 2026-09-30).

**8.1 fait** : `Fond` grave son propre ciel en cubemap (`face_et_uv`,
`direction_de`, `graver_faces`), verrouillé par trois gardes. ⚠️ **Pas encore
consommé** — ses items portent un `allow(dead_code)` qui le dit. Le découpage est
délibéré : la convention de cubemap ne se débogue pas à l'œil (un ciel d'étoiles
aléatoires miroité ressemble à un ciel d'étoiles aléatoires), donc elle est
verrouillée avant qu'une ligne de GLSL en dépende.

**Le prochain pas : 8.2**, la déviation dans le shader. La formule, la liste des
branchements et une **étape de validation** (sampler sans dévier : le quad élargi
doit devenir invisible) sont écrites dans
[`docs/suivi/astres_v2.md`](docs/suivi/astres_v2.md), section « lot 8 ».

⚠️ Limite de fond, pas de budget : une cubemap encode des directions **à
l'infini**. Une planète qui passe derrière le trou noir ne sera ni déviée ni
reprise dans l'anneau — elle a de la parallaxe, le champ d'étoiles non.

Deux dettes du lot 5, petites et non bloquantes : le disque passe sous
l'horizon dessiné à très longue distance (`PX_MIN`), et la convention de
silhouette « mince » n'a jamais été essayée.

→ Tout le détail, lot par lot, dans
[`docs/suivi/astres_v2.md`](docs/suivi/astres_v2.md) — c'est lui qu'il faut
lire, pas les commits.

**Commits** : un par lot ou sous-lot, sur la branche de chantier, une fois
les tests verts et clippy à zéro — la consigne a été révisée le 2026-09-26,
voir « Commits » plus bas. La **fusion** vers `main` reste une décision de
l'utilisateur.

---

## Ce qui précédait (état au 2026-08-16)

**Vérifié** : **734 tests verts**, `cargo clippy --all-targets` à **0
avertissement**.

**Commité et poussé** (`c53bab2` « Chantier trajectoires : lots A-D, icônes
du sélecteur (I.6), retrait N-corps », sur `origin/main`) : chantier I.6
(icônes du sélecteur) et lots A/B/C/D du chantier trajectoires (voir
ci-dessous) — trente fichiers touchés au total. Détail exact dans le commit
lui-même (`git show --stat c53bab2`).

### Ce qui a été fait juste avant l'ouverture du chantier trajectoires

⚠️ Malgré le titre d'origine, **ce n'est plus la dernière session** : quatre
lots du chantier trajectoires (A/B/C/D, voir plus bas) ont été faits depuis.
Gardé tel quel — travail réel, toujours pertinent — mais pour « qu'est-ce qui
vient d'être fait », c'est la section trajectoires ci-dessous qu'il faut lire.

| Chantier | Résultat | Journal |
|---|---|---|
| **I.6 — Icônes du sélecteur** | Pastille de couleur remplacée par un vrai rendu 3D en cache, fond transparent + contour d'un pixel, gazeuses toutes à la même taille (l'anneau déborde au lieu de s'y comprimer), Pluton toujours en dernier malgré son orbite excentrique. Corrige au passage un bug de cycle de vie GPU (une `RenderTarget` dont on ne gardait que la texture voyait celle-ci supprimée à la frame suivante) et un plan de coupe caméra trop loin pour les petits corps (Pluton tombait hors champ) | [`suivi/interface.md`](docs/suivi/interface.md) I.6 |
| **I.6 bis — Anneau, tabulation, panneau animé, Terre** | Azimut de caméra d'un astre à anneau aligné sur sa propre inclinaison (plus l'arbitraire de la position orbitale) + `FOVY_GAZEUSE_DEG` recadré sur Saturne réelle plutôt que le maximum théorique — débordement réduit d'~22 %, pas supprimé (borné par « même taille que Jupiter » + « l'anneau doit déborder »). Tabulation des planètes sous leur étoile. Panneau de droite animé (glissement, ease-in-out). Bug trouvé et corrigé : l'icône de la Terre figeait parfois un repli de terrain **pas encore généré** (génération asynchrone, budget limité) — sans rapport avec la fraction d'eau, qu'un premier réglage avait pourtant fait bouger deux fois en vain | [`suivi/interface.md`](docs/suivi/interface.md) I.6 bis |
| **I.6 ter — Anneau à plat, bouton de repli ×2, colonne « liquide »** | `cadre_camera` (ex-`azimut_et_elevation`) rend `haut` — la projection de la normale de l'anneau sur le plan perpendiculaire à la vue — au lieu de `Vec3::Y` : le grand axe de l'ellipse est désormais **forcé horizontal à l'écran** quel que soit l'astre ou l'instant. Élévation de caméra généralisée en angle paramétrique (`elevation_anneau_rad`, uniforme pour tout astre à anneau, pas un cas Saturne), **mise à zéro** sur demande explicite : l'anneau se réduit à un trait, ouverture nulle. Bouton ◄ doublé (`selecteur::ECHELLE_BOUTON_REPLI = 2.0`), filé dans les trois usages (dessin/clic/réservation d'espace) via des variantes `_bouton` de `liste.rs`. Fond de colonne remplacé par un dégradé de 16 bandes + ménisque haut ; bordure cyan pleine remplacée par du gris 30 % d'opacité (demandé « for testing », pas un réglage final) | [`suivi/interface.md`](docs/suivi/interface.md) I.6 ter |

### À faire ensuite, par ordre

1. **Skymap V2** (marqueurs) : faite, à juger à l'œil contre V1 — garder V2,
   revenir à V1, ou porter les marqueurs dans V1. Journal :
   `suivi/interface.md`. Spec :
   `docs/superpowers/specs/2026-09-22-skymap-v2-marqueurs-design.md`.
2. ⚠️ **Elysium : fps mesuré en jeu, amélioré, encore loin d'être bon.**
   Harnais de profilage ajouté (`PROFIL=<libellé> ./target/release/solar_game.exe`,
   voir `suivi/stations.md`) : le gain du correctif de vitrage (session
   précédente) **mesuré**, pas estimé — insuffisant seul (18,7 fps moyen sur
   Elysium, contre 59 pour la station à anneau de référence, 28,6 pour
   Stanford). Cause n°1 identifiée et chiffrée : les six **bras**
   (`Composant::Charpente`) — 53 928 sommets sur 204 580 (26 %) pour
   6 pièces, `baies` (densité de treillis) jamais réglé pour l'échelle
   d'Elysium (2× celle de Stanford).
   - **Choix fait entre deux corrections** (retoucher la densité des
     treillis — visuel/structurel, propre à Elysium/Stanford — ou baisser
     `SEGMENTS`, le curseur global de tessellation des cylindres) :
     `SEGMENTS` baissé **20 → 12** (`maillage.rs`). Mesuré : 18,7 → **20,0
     fps**. Réel, mais net en dessous de ce qu'annonçait la réduction de
     sommets (~38 % de sommets en moins, ~7 % de fps en plus) — le goulot
     n'est pas surtout le volume de sommets, plutôt un coût **par appel**.
   - **Piste creusée, pas prise** : des tampons GPU **persistants**
     (uploader une fois, ne plus recopier chaque frame) sont possibles en
     principe via `get_internal_gl().quad_context` (accès bas niveau à
     `miniquad::RenderingBackend`, déjà utilisé ailleurs dans ce projet),
     mais `Material`/`QuadGl` (le système d'éclairage) n'expose pas son
     pipeline pour ça — il faudrait un **second chemin de rendu complet**,
     parallèle à `MaillageStation`. Jugé trop gros et trop risqué (un bug de
     shader s'y verrait sur tout le jeu, pas seulement Elysium) pour cette
     session. Détail et refus assumé dans `suivi/stations.md`.
   - ~~le croisement bras/anneau du second anneau~~ — **tranché** : pas de
     raccord dédié nécessaire, ni pour Elysium ni pour Stanford ;
   - la **rotation au bouton** (axe Z) et l'**encadrement du hangar fermé**,
     tous deux ajoutés ce chantier sans les avoir vus tourner/s'afficher ;
   - le **sol et la ville restent volontairement provisoires** (vert plat sans
     texture, parallélépipèdes bruités) — pas un défaut, juste pas habillé.
3. ~~⚠️ Trois choses à juger à l'œil (Stanford/étoiles)~~ — **les trois
   tranchées** (2026-08-14, jugées en jeu) :
   - le **plancher stellaire** : 8 px faisait un Soleil trop gros, ramené à
     **4 px** (`soleil::PX_MIN`) — occupe désormais 21 % du rayon de l'orbite
     de Mercure (42 % avant) ;
   - la **charge du mât polaire** « derrière » le miroir bas et le **mât qui
     perce les disques** : **pas des défauts** — le miroir est troué en son
     centre pour le mât (`VarianteMiroir::trou`), tout se voit à travers.
     `POLE_ECART` inchangé.
   - Détail dans `suivi/etoiles.md` (P.3 bis, révisé) et `suivi/stations.md`
     (« Marge de 1,8 sous le miroir bas »).
4. **Palette : lister les sous-variantes.** `echantillons()` n'en donne qu'une par
   `Composant`, si bien qu'`ARC SOUTE`, les postes d'amarrage et `HANGAR FERME`
   sont **inatteignables à l'écran**. Bloque l'étape 5 de l'Endurance.
5. **Endurance étape 5** : assembler l'anneau à douze avec l'alternance des
   quatre familles. Puis étape 6 : propulsion et réservoirs arrière.
6. **Assembleur Lot 8** (décalage) et **Lot 9** (redimensionnement) ; **Lot 6**
   pour la dette d'enveloppes L1.4, les composites et `PoutreBout`.
7. **D-INT-2, l'économie** — reporté explicitement trois fois. À ouvrir quand la
   vue système sera close.
8. **D-PIX-3/4** (contours Sobel, taille de pixel réglable) et les **comètes**
   (`Categorie::Comete` n'engendre rien).

### Chantier en cours : trajectoires (Lots A, B, C et D faits, 2026-08-15/16)

Faire voyager un vaisseau assemblé (VAB) ou une station déjà en orbite d'un
astre à un autre — le Starship est **fini depuis le 2026-08-06** et n'a
toujours nulle part où aller (§10 bis de la bucket list). Conception :
[`conception/trajectoires.md`](docs/conception/trajectoires.md) (§4, **onze**
décisions de gameplay désormais — sept le 2026-08-14/15 : trajet automatisé
sans plan de manœuvre, statistiques de vol en hybride, rendez-vous avec
cible mobile dès le premier jet — y compris aux cinq points de Lagrange —,
logistique manuelle d'abord, minuteur de superstructure et saut
interstellaire de l'ISV **hors périmètre**, colonisation qui consomme le
vaisseau ; **quatre de plus le 2026-08-15/16** : un vaisseau non en vol est
« posté » via `EnginOrbital` — orbite d'une planète/lune ou vol stationnaire
près d'un habitat/station, résout d'où vient le départ d'un trajet ; coût de
construction non plat + réutilisabilité du Starship croissant avec la
recherche ; stations postées **non pilotables** mais dotées de PV
(dégâts d'éruption solaire/astéroïde, réparation) ; **mode N-corps retiré du
jeu** comme code de test) et
[`conception/concept_jeu.md`](docs/conception/concept_jeu.md) (vision RTS
globale). Journal : [`suivi/trajectoires.md`](docs/suivi/trajectoires.md) —
y compris une liste consolidée de tous les chiffres encore provisoires.

- ✅ **Lot A — Statistiques de vol** : `src/vaisseau/vol.rs`, nouveau.
  `ClasseVaisseau::Starship` (seul, volontairement — ISV et Jupitris hors
  scope, voir le journal), `StatsVol` (vitesse/portée/fret) = base de classe
  + modificateurs des pièces de propulsion (`Raptor`, `Propulseur` axial) et
  réservoir. Chiffres de base provisoires, nommés. 5 tests, tous
  red-checkés.
- ✅ **Lot B — Mobile** : la prédiction (`Astre::position_future`,
  `Systeme::position_future_de`, `Systeme::temps`), la géométrie de l'arc
  (`systeme::trajectoire::Trajectoire`, Bézier quadratique, rendez-vous
  garanti par construction) et le vaisseau mobile lui-même
  (`transit::VaisseauEnTransit`, `Astre::suivre_trajectoire`, câblé dans
  `Systeme::update`). 17 tests, tous red-checkés.
- ✅ **Mode N-corps retiré** (2026-08-16, confirmé « code de test » par
  l'utilisateur) : `src/systeme/gravite.rs` supprimé, `ModePhysique` et tout
  son câblage (`Systeme::mode`/`regler_mode`, `Astre::amorcer_ncorps`, le
  bouton PHYS du menu) retirés — pas juste désactivés. La bande d'outils
  passe de 5 à 4 bascules, vérifié à l'écran. Retire du même coup la
  divergence de prédiction en `NCorps` notée dans le Lot B : il n'y a plus
  qu'un mode, la prémisse de `trajectoires.md` §5 tient sans condition.
- ✅ **Lot C — Un premier vaisseau qui vole vraiment** (2026-08-16) : bouton
  LANCER dans la fiche (`fiche::bouton_lancer`, décision 8) + mode armé
  dans `skymap.rs` (`Skymap::lancement`, `tenter_lancement`) — clic sur le
  vaisseau, clic sur LANCER, clic sur une destination, il part. Backend :
  `Systeme::calculer_trajectoire` (résolution par point fixe du rendez-vous
  avec une cible mobile — convergence **mesurée**, pas supposée : voir le
  journal pour le cas défavorable qui a fait remonter `ITERATIONS` de 4 à
  24), `Systeme::lancer`/`Astre::tenter_lancer` (a remplacé
  `Systeme::remplacer`, qui s'est révélé être la mauvaise primitive —
  retiré, pas gardé). Un vrai Starship pilotable posté en orbite terrestre
  (`genese/presets.rs`). ⚠️ **Le clic réel n'a pas été rejoué à l'écran**
  (`CAPTURE` ne simule pas de clic) — seule la boucle de mise à jour est
  vérifiée saine sur 300 frames ; toute la logique qui décide est testée à
  fond. **716 tests verts**, clippy 0 avertissement.
- ✅ **Lot D — Aperçu du trajet + caméra qui suit** (2026-08-16) :
  `Trajectoire::polyligne(n)` (échantillonnage de l'arc, coordonnées monde
  déjà absolues — pas de `centre` à additionner, contrairement à
  `Orbite::polyligne`), `dessiner_points_ouvert` dans `systeme/rendu.rs`
  (même technique de semis à espacement écran constant que les orbites,
  mais **sans** le `% n` qui boucle une polyligne fermée — un arc à sens
  unique ne doit pas relier son arrivée à son départ). `Calques` porte
  maintenant `previsualisation: Option<Vec<Vec3>>`, alimenté par
  `Skymap::previsualisation_trajectoire` : tant qu'un vaisseau est armé
  (LANCER cliqué), l'astre **survolé** (pas cliqué — `Systeme::pick`,
  `&self`, aucun effet de bord, pas `Camera::pick` qui déplace le focus)
  donne un arc d'essai, dessiné en ambre. **Caméra** : `tenter_lancement`
  appelle `self.cam.set_focus(idx_vaisseau)` après un lancement réussi —
  et rien de plus n'était nécessaire, `Camera::cible` relit
  `Systeme::position(idx)` chaque frame quel que soit ce qui la fait bouger
  (orbite ou trajet), la mécanique de suivi existait déjà. **719 tests
  verts**, clippy 0 avertissement. ⚠️ Même limite que le Lot C : le clic/
  survol réel n'a pas pu être rejoué à l'écran (`CAPTURE` ne simule pas la
  souris) — vérifié par lecture de code (`Camera::cible`/`Systeme::update`)
  et par les tests unitaires de `polyligne`, pas par capture visuelle de
  l'arc lui-même.
- **Suite : Lot E** — arrivée (amarrage, consommation à la colonisation) :
  un vaisseau arrivé reste immobile pour toujours aujourd'hui.

### Dettes ouvertes repérées en chemin

- Le **défilement** n'est câblé que dans la vue des composants ; `galerie_disques`
  et `skymap` partagent la même colonne et ont le même défaut.
- `src/ecran/briques.rs` et `src/ecran/vaisseaux.rs` sont commités mais non
  déclarés : ils ne compilent pas.

## Commits

Dernier commit : `c53bab2` « Chantier trajectoires : lots A-D, icônes du
sélecteur (I.6), retrait N-corps » — **poussé sur `origin/main`**. Avant
lui : `a0ebb58` « torus & elysium complex base complete », `73fb924`
« torus », `fb7d589` « fix » (2026-08-10), `dc0d7bf` « torus ok », `2a042e6`
« P4 ok », `7278aa1` « P.1 a P.3 ok, plus fix ui ».

⚠️ **Périmé depuis.** Skymap V2 a été **fusionné dans `main`**
(`f3a32b8`), et la branche courante est maintenant `chantier-astres`
(worktree `.claude/worktrees/astres-v2`), 32 commits d'avance sur `main`,
**non fusionnée et non poussée**. Les deux lignes ci-dessus décrivent un
`main` déjà dépassé.

**Consigne debout (révisée le 2026-09-26) : commiter à chaque lot, sur la
branche de chantier.**

⚠️ **Elle remplace l'ancienne**, *« no commit i'll say when commit is
needed »*, qui interdisait de commiter de soi-même. Cette règle-là datait
d'une époque où l'on travaillait **directement sur `main`** : un commit y
était un engagement, et l'utilisateur voulait le prononcer lui-même.

Depuis, le travail se fait sur une **branche de chantier dans un worktree**
(`.claude/worktrees/…`). Le raisonnement s'inverse : un commit n'engage plus
rien — il **crée un point de repli**. Si un lot tourne mal, on revient au
commit précédent au lieu de perdre le travail. C'est exactement ce que
cherchait à éviter l'incident d'origine, un `git checkout --` qui avait
détruit du travail non commité (`docs/suivi/stations.md` §F.8) : la réponse
n'était pas de commiter *moins*, c'était de commiter *plus tôt*.

En pratique : **un commit par lot ou sous-lot**, une fois les tests verts et
clippy à zéro. La fusion vers `main`, elle, reste une décision de
l'utilisateur — c'est là qu'est passé l'engagement.

⚠️ **Ne jamais employer `git checkout --`, `git restore` ou `git reset --hard`**
sur un fichier contenant du travail non commité. Pour annuler ses propres
modifications : ré-éditer les lignes, ou sauvegarder le fichier dans le
scratchpad **avant** de le toucher (red-checks compris).

État vérifié au 2026-09-26 : **838 tests verts** (850 avec `mhd`), **`cargo
clippy --all-targets` : 0 avertissement** (branche `chantier-astres`).

Ancien relevé : **734 tests verts** (746 avec le binaire `mhd`), **`cargo clippy
--all-targets` : 0 avertissement** (branche `worktree-skymap-v2-marqueurs`,
voir la ligne « Branche courante » ci-dessus ; 719 était le compte au moment
du Lot D, laissé tel quel dans le récit qui suit).

## Chantier courant : finir les planètes et la vue système

**Décidé le 2026-08-06, avant D-INT-2** : l'économie attendra que la vue système
soit finie. On ne conçoit pas une économie par-dessus un décor qu'on va encore
remuer.

| # | Chantier | Nature | État |
|---|---|---|---|
| **P.1** | **Atmosphère planétaire** | vrai manque, confirmé dans le code | ✅ code fait (teinte/densité séparées le 2026-08-09), **rendu à valider à l'œil** |
| **P.2** | **Ceintures : finition + optimisation** | mesurer d'abord | ✅ pour l'essentiel — reste l'instrument de fill rate (touche **V**), jamais relevé |
| **P.3** | **Étoiles : éjections et motifs de surface** | ⚠️ **existent déjà** — enquête avant travaux | ✅ tranché : plancher à 8 px, **à valider à l'œil** |
| **P.4** | Finitions planétaires (rotation propre, nuages, ombres d'anneaux) | liste ancienne | ✅ |


Le **détail** de chacun — ce qui a été trouvé, les mesures, les red-checks — est
sorti d'ici le 2026-08-10 : voir « Où trouver quoi » juste dessous.

## Où trouver quoi

⚠️ **Ce fichier ne raconte plus rien.** Il dit l'état et la suite ; chaque
chantier a son journal. C'est la correction du travers qui l'avait porté à
1 968 lignes.

| Vous cherchez | C'est dans |
|---|---|
| **La discipline** : red-check, pièges de tests, règles apprises | [`docs/methode.md`](docs/methode.md) |
| Atmosphères (P.1), rotation propre, nuages, ombres d'anneaux (P.4), cohérence du catalogue | [`docs/suivi/planetes.md`](docs/suivi/planetes.md) |
| Ceintures, anneaux, débris, galerie des disques (tout P.2) | [`docs/suivi/ceintures.md`](docs/suivi/ceintures.md) |
| Étoiles : éruptions, échelle, plancher à 8 px (P.3) | [`docs/suivi/etoiles.md`](docs/suivi/etoiles.md) |
| Interface : calques, trajectoires, police, paramètres, échelle | [`docs/suivi/interface.md`](docs/suivi/interface.md) |
| Assembleur (VAB) : lots 1 à 5, rotation, sauvegarde | [`docs/suivi/assembleur.md`](docs/suivi/assembleur.md) |
| Stations, vaisseaux, composants, mégastructures | [`docs/suivi/stations.md`](docs/suivi/stations.md) |
| Pièges moteur : pipelines, materials partagés, GPU | [`docs/suivi/technique.md`](docs/suivi/technique.md) |
| Revues de code | [`docs/suivi/revues.md`](docs/suivi/revues.md) |
| Géantes gazeuses | [`docs/suivi/geantes_gazeuses.md`](docs/suivi/geantes_gazeuses.md) |
| La liste globale des fonctionnalités | [`docs/suivi/bucketlist_globale.md`](docs/suivi/bucketlist_globale.md) |
| Le **pourquoi** de chaque système | [`docs/conception/`](docs/conception/) |
| **Trajectoires** (vaisseaux/stations mobiles) — Lots A/B/C faits, journal | [`docs/suivi/trajectoires.md`](docs/suivi/trajectoires.md) (conception : [`docs/conception/trajectoires.md`](docs/conception/trajectoires.md)) |
| **La vision RTS globale** (isolement des ressources, tuiles, recherche, habitats, superstructures, vaisseaux) | [`docs/conception/concept_jeu.md`](docs/conception/concept_jeu.md) |

## Chantiers en pause

| Chantier | Où on en est | Reprendre par |
|---|---|---|
| **Assembleur** | Lots 4–5 clos ; **rotation (R) faite** sans attendre le gizmo | Lot 8 (décalage) et Lot 9 (redimensionnement) |
| **Starship** ✅ **5/5** | Coque, 6 Raptor, jupe, 4 volets droits, bardage de tuiles (fût **et** ogive) et 2 hublots. Exposé **debout**, moteurs en bas. | Vaisseau **mobile** (transferts entre astres) |
| **Endurance** | **Module en coin d'arc fait** (brique seule, §D.5 étape 4) | ⚠️ **bloqué** : la palette ne liste qu'une sous-variante par composant, donc les quatre familles d'arc sont inatteignables à l'écran. Débloquer `echantillons()` d'abord, puis §D.5 étape 5 |
| **Tore de Stanford** | Miroirs, pilier polaire, moitiés fixe/tournante, charge du mât bas — les trois points à juger à l'œil sont **tranchés** (2026-08-14) | rien de connu ; reprendre si un nouveau défaut se signale |
| **Complexe Elysium** | Chantier **initial clos** 2026-08-14 (anneau complet, sol/ville, second anneau, face à Z + rotation, hangar, rendu optimisé) | ⚠️ points à **juger à l'œil** — voir « Reprendre ici » ; sol/ville restent délibérément provisoires |

## L'assembleur, en résumé

L'ISV est **clos** (`docs/suivi/stations.md` §C.29), Elysium aussi pour son
chantier initial (même fichier, Partie F). Les mégastructures et le
Starship sont journalisés en **Partie F** du même fichier.

| Lot | Contenu | État |
|---|---|---|
| **Lot 1** (L1.1–L1.8) | Fondations : catalogue à source unique, balayage des variantes, déterminisme, rayons déclarés, **capsules**, overlays, mesureur | ✅ |
| **§9** « le boudin » | Enveloppe **rectangle** pour les plaques, distances exactes | ✅ |
| **Lot 2** (L2.1–L2.5) | Modèle du `Chantier` : identifiants stables, `retirer`, undo/redo, palette, sérialisation | ✅ |
| **Lot 3** (L3.1–L3.3) | `pose_prevue` (fantôme), `sous_arbre`, désignation port/pièce | ✅ |
| **Lot 4** (L4.1–L4.6) | **L'écran d'assemblage**, déplacement compris | ✅ |
| **Lot 5** | Sauvegarde disque (L5.1), overlay §8.5 (L5.2) | ✅ |
| **Lot 6** | Arbitrage L1.4 (L6.1), composites `figer` (L6.2), `PoutreBout` (L6.3) | à faire |

Le modèle répond aux quatre questions de §8, et **un écran le consomme enfin** :
`ecran::assembleur::VueAssembleur`. `Chantier` est réexporté, et
`Chantier::apercu` lui donne une station sans le consommer (voir
`suivi/assembleur.md` § L4.1).

**Arbitrage §10.7 (b) rendu** (2026-08-09) : quitter l'écran détruit le
chantier, et on l'assume jusqu'à la sauvegarde du Lot 5. Échap repose d'abord la
pièce en main, et ne quitte qu'au repos.

⚠️ Le Lot 4 est le **premier lot majoritairement non testé** (§6.6 : pas de test
de rendu) — comme l'interface de jeu. La discipline change de forme : pousser
hors du code de dessin tout ce qui **se décide**, vers des requêtes
red-checkables. Ce qui reste non testé ne doit être que *où le rectangle se
pose*, jamais *ce qu'il signifie*.

Régénérer le catalogue de fils : `FILS=1 cargo test --release le_catalogue_des_fils`.

Mesurer le fps réel d'un item des Mégastructures (pas une estimation en
triangles) : `PROFIL="<libellé exact>" ./target/release/solar_game.exe` —
10 s d'échantillonnage, la fenêtre se ferme d'elle-même et imprime le
résultat (`suivi/stations.md`, « Le vrai fps, mesuré en jeu »).

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

- ~~**Sortir de l'assembleur perd le vaisseau**~~ (§10.7b) — **tranchée le
  2026-08-09** : on l'assume, et le Lot 5 (sauvegarde disque) l'a rendue
  inoffensive. Échap repose d'abord la pièce en main.
- **Les mégastructures à taille réelle** : ~2,85 pièces et ~2 850 sommets par
  unité de rayon d'anneau, soit ~1 140 pièces pour l'Elysium de 1,8 km contre 45
  pour l'ISV (§F.3). L'assemblage pièce à pièce ne passera pas à l'échelle.
- **Membrures à axe courbe** : aucune primitive du projet n'en trace, et les
  bras des mégastructures en ont besoin pour être reliés entre eux (§F.1).


## Méthode

Déplacée dans **[`docs/methode.md`](docs/methode.md)** — trop importante pour
rester noyée. Les quatre règles qu'on oublie le plus souvent :

- **Red-check systématique** : casser volontairement ce que le test garde,
  vérifier qu'il rougit, rétablir. **Sauvegarder le fichier dans le scratchpad
  avant de le saboter**, et le restaurer par copie — jamais par git.
- ⚠️ **Un sabotage qui ne rougit pas n'accuse pas la garde** : vérifier d'abord
  qu'il produisait vraiment le défaut visé.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent
  d'une valeur qui en a deux.
- **Mesurer avant d'affirmer**, et **mettre à jour les `.md` à chaque étape**.

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
- ⚠️ **Cette entrée disait « les 86 avertissements clippy ne contiennent aucun
  défaut réel ». C'était faux**, et la revue P1 (2026-08-05) l'a montré :
  `MASSES_TERRESTRES` se documentait comme la source unique des masses
  planétaires et **personne ne la lisait** — les masses étaient recopiées à trois
  endroits, et le test de Kepler vérifiait *sa propre copie*. Le défaut était
  visible depuis le début, noyé dans le bruit. **`clippy` est à 0 aujourd'hui**,
  et c'est ce qui rend le prochain visible.
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
