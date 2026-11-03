# Suivi — Dette architecturale de `vaisseau/`

> Formalise le plan issu de l'audit critique du 2026-08-31 (`src/vaisseau/`,
> `src/vaisseau/composant/`, `src/vaisseau/chantier.rs`). Trois chantiers
> indépendants, mécaniques d'abord (découpage de fichiers, zéro changement de
> comportement), structurels ensuite seulement si le mécanique ne suffit pas.
> Point de reprise : [`../../STATE.md`](../../STATE.md).

---

## Constat de départ

Le refacto « fichiers ≤ ~100 lignes » (`bucketlist_globale.md` §7) a réussi
partout **sauf** dans `vaisseau/` :

| Fichier | Lignes | Défaut principal |
|---|---:|---|
| `vaisseau/generateur.rs` | 6626 | Mélange le moteur de génération procédurale, ~60 presets figés sans rapport entre eux, et 3 blocs `#[cfg(test)]` interleavés au milieu du code de prod |
| `vaisseau/composant/mod.rs` | 4077 | Enum + 5 gros `match` de dispatch dupliqués (~40 branches chacun) + catalogue UI, trois responsabilités dans un seul fichier |
| `vaisseau/chantier.rs` | 2126 | Pas encore instrumenté — inventaire à faire avant de trancher |

Duplication annexe repérée dans le même audit, hors périmètre des chantiers
ci-dessous mais à garder en tête : `reservoir::ports`/`habitat::ports`/
`cargo::ports`/`equipage::ports` recopient le même corps de 4 lignes alors
que `composant/commun.rs` affiche la doctrine inverse (« ce qui serait
dupliqué ailleurs vit ici »).

---

## Principes directeurs des trois chantiers

- **Mécanique d'abord.** Un découpage en fichiers est un cut/paste : zéro
  changement de comportement, donc zéro red-check nécessaire — la seule
  preuve exigée est `cargo test` identique avant/après (même nombre de tests
  verts) et `cargo clippy --all-targets` toujours à 0 avertissement.
- **Un commit par étape vérifiable**, jamais un gros diff monolithique —
  cohérent avec « brique d'abord, assemblage ensuite » (`methode.md`).
- **L'API publique de `vaisseau::` ne bouge pas.** `vaisseau/mod.rs` réexporte
  aujourd'hui les presets par leur nom (`pub use generateur::{generer,
  preset_iss, preset_starship, ...}`) : tant que le nouveau module
  `generateur` réexporte les mêmes noms, ce bloc n'a pas besoin d'être touché.
- **Vérifier les call-graphs avant de couper**, pas les deviner : plusieurs
  helpers privés de `generateur.rs` (`tournee`, `pivoter`, `verser`,
  `verser_a_echelle`) ressemblent à des utilitaires partagés entre presets
  plutôt qu'exclusifs à un seul — à trancher avec
  `mcp__serena__find_referencing_symbols` avant de choisir leur fichier
  d'atterrissage, pas en devinant sur la position dans le fichier.
- **Mettre à jour ce document à chaque étape** (case cochée + note courte),
  consigne debout du projet.

---

## Chantier 1 — `generateur.rs` → moteur + presets

- [x] **1.0 Baseline** (2026-08-31) : **719 tests verts**, `cargo clippy
  --all-targets` **0 avertissement** — identique à l'état déjà consigné dans
  `STATE.md`, pas de dérive depuis.
- [x] **1.1 Isoler le moteur réel** (2026-08-31). `generateur/moteur.rs`
  porte `generer()` + son call-graph réel, vérifié un par un par
  `find_referencing_symbols` plutôt que fait confiance à la liste ci-dessus :
  `Rng`, `Style`, `Ossature`, `ParamsStation`, `compte*`,
  `greffer_propulsion`, `module`, `port_vers`, `coeur_iss`, `coeur_mir`,
  `greffer_structure_puissance`, `poser_jonction_et_bras`, `index_port`,
  `port_le_plus_haut`, `corridor_libre`, `sur_pressurise`, `brancher`,
  `terminer_extremites`, `fabrique_appendice`, `cle_surface`,
  `habiller_surface`. **Surprise du call-graph** : `hab`, `hub6` et
  `poser_sur` — que la liste ci-dessus plaçait dans le moteur — ne sont
  appelés par `generer()` nulle part ; ce sont des helpers de presets
  (`hab`/`poser_sur` exclusifs à `preset_iss`, `hub6` partagé entre familles).
  Inversement `nez_docking`, physiquement au milieu des helpers ISS, **est**
  appelé par `terminer_extremites` (donc par le moteur) en plus de deux
  presets Mir/Tiangong — il reste dans `moteur.rs`, `pub(crate)`.
- [x] **1.2 Créer `vaisseau/generateur/presets/`** (2026-08-31) — voir Journal
  pour le détail des familles réelles, qui divergent par endroits de la liste
  ci-dessous (vérifiées, pas devinées). Éclaté les ~60
  `preset_*`/`demo_*` par famille, chaque preset avec ses helpers privés
  **exclusifs** dans le même fichier (vérifiés par `find_referencing_symbols`,
  pas supposés) :
  - `iss.rs` — `preset_iss`, `preset_comsat`, `preset_sonde` + leurs helpers
    (`hab_l`, `nez_docking`, `paire_ailes`, `vaisseau_amarre`, `sur_face`, à
    confirmer un par un).
  - `mir_tiangong.rs` — `preset_mir`, `preset_tiangong`.
  - `starship.rs` — `preset_starship`, `demo_raptor`, `demo_anneaux` +
    `starship_moteurs`, `starship_volets`, `starship_bardage`,
    `starship_hublots`, `starship_debout`, `starship_cul`.
  - `elysium.rs` — `preset_elysium` + `elysium_rayon_second_anneau`,
    `second_anneau_elysium`, `jante_elysium`.
  - `stanford.rs` — `preset_stanford` (test only), `preset_stanford_moyeu`,
    `preset_stanford_anneau` + `pilier_polaire`, `charge_polaire_basse`,
    `hauteur_moyeu`, `stanford_rayon_tore`, `elysium_rayon_jante`,
    `pied_de_bras`, `moyeu_hexagonal`, `moyeu_elysium`, `bras_et_tunnels`,
    `habitats_accouples`.
  - `isv.rs` — `preset_isv`, `preset_isv_hexa`, `preset_isv_fixe`, `isv()`,
    `preset_isv_equipage`, `preset_isv_moteur` + `poser_bloc_moteur`.
  - `demos.rs` — le reste des `demo_*` (charpente, cargo, bouclier*,
    équipage, réservoir, moteur antimatière) + leurs helpers
    (`hab_rayon_mini`, `hab_couronne`, `hab_attache`,
    `poser_grappe_habitat`, `axe_pivot`, `poser_equipage`,
    `poser_tete_bouclier`, `bouclier_thermique`, `demo_habitat_isv`).
  - `commun.rs` — utilitaires partagés entre familles s'ils s'avèrent
    réellement partagés (`tournee`, `pivoter`, `verser`,
    `verser_a_echelle`, `demo_panneaux_mega`, `preset_anneau`,
    `demo_radiateur_mega`, `demo_moteur_antimatiere` si aucune famille
    évidente ne les réclame).
- [x] **1.3 Reloger les tests** (2026-08-31). Des 3 blocs `#[cfg(test)]`
  interleavés, deux (`preset_stanford`, la constante `HAB_ATTACHE`) étaient
  déjà à côté de leur fonction une fois la famille regroupée — rien à
  bouger. Le troisième (`verser`, wrapper 1 ligne autour de
  `verser_a_echelle`) a suivi son unique appelant `preset_stanford` dans
  `stanford.rs`, son corps qualifiant l'appel vers `commun::verser_a_echelle`
  (seul changement : le chemin, pas la logique). **Jugement** : le gros
  module de tests (2500+ lignes, cf. Journal) reste groupé dans
  `generateur/tests.rs` plutôt que réparti par preset — le découper à la
  main sur un fichier de cette taille, croisant des dizaines de familles,
  risquait plus de mal classer un test que d'aider à le retrouver.
- [x] **1.4 `generateur/mod.rs`** (2026-08-31) déclare les sous-modules et
  réexporte en bloc (`pub use moteur::*; pub use presets::*;`, même schéma
  dans `presets/mod.rs`) — `vaisseau/mod.rs` **n'a reçu aucune modification**
  (vérifié par `git diff`).
- [x] **1.5 Vérification finale** (2026-08-31) : `cargo build` et
  `cargo clippy --all-targets` propres, `cargo test` → **719 passed; 0
  failed** (identique à 1.0). Fichiers ≤ ~500 lignes pour la moitié des
  nouveaux fichiers (`mir_tiangong.rs` 184, `commun.rs` 185, `elysium.rs`
  228, `iss.rs` 287, `stanford.rs` 423) ; `starship.rs` (532), `isv.rs`
  (650), `demos.rs` (794), `moteur.rs` (884) et `tests.rs` (2522) dépassent
  la cible souple — voir Journal, ce sont surtout de gros commentaires
  explicatifs déplacés tels quels, pas de la logique qu'on aurait pu couper
  plus fin sans mentir sur les familles réelles.
- [x] **1.6 Mis à jour** `docs/suivi/stations.md` et `bucketlist_globale.md`
  §7 (2026-08-31) : `vaisseau/` rejoint la discipline « fichiers courts ».

## Chantier 2 — `composant/mod.rs` → types / dispatch / catalogue

- [ ] **2.0 Baseline** (comme 1.0, après le chantier 1).
- [ ] **2.1** `composant/types.rs` — `enum Composant`, `enum Categorie`.
- [ ] **2.2** `composant/dispatch.rs` — les 5 `impl Composant` de
  comportement (`ports`, `dessiner`, `cout`, `rayon_local`,
  `enveloppe_locale`, la fonction privée `suivante`).
- [ ] **2.3** `composant/catalogue.rs` — `categorie()`, `posables()`,
  `echantillons()`, `sous_ensemble_echantillon()`.
- [ ] **2.4 Vérification finale** (identique à 1.5).
- [ ] **2.5 Option B (à ne juger qu'après 2.1–2.4, chantier séparé)** :
  remplacer les 5 `match` parallèles par un trait `ComportementComposant`
  implémenté par chaque struct de composant, dispatché une seule fois. Ne
  pas l'entamer dans le même commit que le split mécanique — seulement si
  l'ajout d'un nouveau composant redevient pénible en pratique.

## Chantier 3 — `chantier.rs` : inventaire puis découpage

- [ ] **3.1 Inventaire.** Lister toutes les méthodes de `impl Chantier`
  (`get_symbols_overview`) et les grouper par responsabilité apparente
  (pose/branchement, déplacement/undo via `Instantane`/`PlanDeplacement`,
  requêtes en lecture, symétrie). Aucune coupe avant cet inventaire.
- [ ] **3.2 Proposer un découpage** sur la base de 3.1 — pas avant, pas de
  plan a priori ici pour éviter de deviner une structure qui ne correspond
  pas au vrai couplage interne.

---

## Journal

*(rempli au fil des étapes — date, ce qui a été fait, ce que la vérification
a montré)*

- **2026-08-31 — Chantier 1 complet (1.1 à 1.6).** `generateur.rs` (6626
  lignes) éclaté en `generateur/{mod.rs, moteur.rs, tests.rs,
  presets/{mod.rs, commun.rs, iss.rs, mir_tiangong.rs, starship.rs,
  elysium.rs, stanford.rs, isv.rs, demos.rs}}`. Extraction faite par plages
  de lignes exactes (`sed`) après avoir localisé chaque fonction/constante
  via `find_symbol`/`find_referencing_symbols` — aucune retype à la main du
  corps des fonctions, seulement des ajouts de `use` et de qualifications de
  chemin.

  **Vérification finale** : `cargo build` propre, `cargo test` →
  **719 passed; 0 failed** dans le binaire principal (12 passed inchangés
  dans `mhd`), `cargo clippy --all-targets` → **0 avertissement** (vérifié
  après `cargo clean -p solar_game` pour écarter le cache). Chiffres
  identiques à la baseline 1.0 — aucune dérive.

  **Surprises du call-graph** (vérifiées par `find_referencing_symbols`,
  pas devinées sur la position dans l'ancien fichier) :
  - `hab`, `hub6`, `poser_sur` : la liste indicative de 1.1 les plaçait dans
    le moteur. En réalité aucun n'est appelé par `generer()` — `hab` et
    `poser_sur` sont exclusifs à `preset_iss` (→ `iss.rs`), `hub6` est
    partagé entre `iss.rs` et `mir_tiangong.rs` (→ `commun.rs`).
  - `nez_docking` : physiquement au milieu des helpers ISS (donc plutôt
    « preset » à vue d'œil), mais réellement appelé par `terminer_extremites`
    — donc par le moteur — en plus de `preset_mir`/`preset_tiangong`. Resté
    dans `moteur.rs`, `pub(crate)`, importé par `mir_tiangong.rs`.
  - `hab_l`, `paire_ailes`, `vaisseau_amarre` : physiquement juste après
    `preset_iss`, mais **jamais** appelés par lui — exclusifs à
    `preset_mir`/`preset_tiangong`. Déplacés en bloc vers `mir_tiangong.rs`
    plutôt que laissés en `iss.rs` avec un import inverse.
  - `tournee` : la liste des principes directeurs le flaguait comme
    « à vérifier, peut-être partagé ». En réalité exclusif à
    `preset_elysium` — resté dans `elysium.rs`, pas de `commun.rs`.
  - `pivoter` : même chose, exclusif à `isv()`/`preset_isv_equipage` — resté
    dans `isv.rs`.
  - `verser_a_echelle` : lui, réellement partagé (`preset_stanford_moyeu`
    **et** `isv()`) — direction `commun.rs` confirmée.
  - `EPINE` (et toute l'échelle ISV — `ISV_ECHELLE`, `EPINE_SOMMET_Y`,
    `Epine`, le bloc `FRET_*`) : présumé isv-exclusif par sa position, mais
    consommé aussi par `demos.rs` (`hab_rayon`, `equipage_alesage`,
    `poser_grappe_habitat`, `poser_equipage`, `poser_tete_bouclier`,
    `BOUCLIER_DEBUT_Y`). Resté en bloc dans `isv.rs` (son vrai foyer), avec
    import `use super::isv::{EPINE, ISV_ECHELLE, EPINE_SOMMET_Y};` côté
    `demos.rs`. Réciproquement, `isv()` appelle `poser_grappe_habitat`,
    `poser_equipage`, `bouclier_thermique`, `poser_tete_bouclier`,
    `EtatEquipage` — tous dans `demos.rs` : couplage **bidirectionnel**
    isv.rs ↔ demos.rs, réel dans le code d'origine (l'ISV « complet » est
    littéralement l'assemblage des vitrines demos), pas un artefact du
    découpage.
  - `elysium.rs` ↔ `stanford.rs` : même couplage bidirectionnel, déjà
    annoncé dans le commentaire de tête de `preset_stanford` (« les deux
    mégastructures partagent leur moyeu et leurs bras »). Les constantes
    d'échelle (`STANFORD_BRAS`, `STANFORD_ECHELLE`, `ELYSIUM_PROFIL`,
    `HABITAT_PROFIL`, `ELYSIUM_HABITAT`) sont restées dans `elysium.rs`
    (leur position d'origine), les fonctions de moyeu/bras partagées
    (`moyeu_elysium`, `bras_et_tunnels`, `habitats_accouples`,
    `hauteur_moyeu`, `elysium_rayon_jante`) dans `stanford.rs` — chacune
    important le manquant de l'autre.

  **Visibilité** : tout ce qui était `fn`/`const` module-niveau et privé
  dans le monolithe est passé `pub(crate)` mécaniquement (pas de `pub(super)`
  au cas par cas) — sinon ni les fichiers de presets voisins ni
  `generateur/tests.rs` (sibling de `moteur`/`presets`, pas leur enfant) ne
  pouvaient les voir. Les `mod.rs` réexportent en bloc
  (`pub use moteur::*; pub use presets::*;`). Rust plafonne l'effet d'un
  `pub use` à la visibilité réelle de la cible pour tout ce qui compte
  (rien ne devient consommable hors-crate qui ne l'était pas), mais un
  helper `pub(crate)` redevient nommable via `generateur::` /
  `generateur::presets::` par simple ricochet du glob — accepté sciemment
  ici (binaire de jeu, aucun consommateur externe) plutôt que de retaper à
  la main la liste exacte des réexports par famille, terrain où une seule
  faute de frappe aurait été invisible jusqu'au test qui l'aurait percutée.
  `vaisseau/mod.rs` n'a reçu **aucune modification** (`git diff` vide) : les
  noms qu'il importe (`generer, preset_iss, ..., Epine, EtatEquipage,
  Ossature, ParamsStation, Style, ISV_AXE`) étaient déjà `pub` dans le
  monolithe et le sont restés tels quels dans leur nouveau fichier.

  **`super::` devenu `crate::vaisseau::`** : trois références
  (`super::Piece`, `super::Station` dans `verser_a_echelle`/`pivoter`,
  `super::grappe_hauteur` dans `charge_polaire_basse`) visaient
  `vaisseau::X` depuis l'ancien `generateur.rs` ; la profondeur de nesting
  ayant changé (`generateur/presets/famille.rs` au lieu de
  `generateur.rs`), elles ont été réécrites en chemin absolu
  `crate::vaisseau::X` plutôt que recompter les `super::super::` par
  fichier — mécanique, pas un changement de comportement.

  **Judgment call (1.3)** : le module de tests (2500+ lignes, dizaines de
  presets et de constantes croisées) est resté un seul `generateur/tests.rs`
  plutôt que réparti par fichier de preset. Le découper à la main sur un
  fichier de cette taille — sans lecture exhaustive de chaque test pour
  identifier sa vraie cible — risquait de mal classer un test ou de casser
  une dépendance croisée plus que ça n'aidait à la lecture. Seul le wrapper
  `mod tests { use super::*; ... }` a été retiré (le fichier `tests.rs` sert
  lui-même de module, plus besoin de le nester) et `use super::*;` remplacé
  par `use super::moteur::*; use super::presets::*;`.

- **2026-11-03 — Chantier 1 revérifié avant commit.** Resté hors commit deux
  mois ; aucun commit n'a touché `generateur.rs` depuis `a0ebb58`, antérieur
  au découpage. Comparaison ligne à ligne contre l'original : seuls des `use`,
  des `pub(crate)`, sept chemins requalifiés et des en-têtes de module
  diffèrent. Sur `main` à `9868eff` : **992 tests verts** (+ 13 de `mhd`),
  clippy **0**. **Portée** : un découpage pour la lisibilité, **pas une
  refonte** — l'audit a mesuré tailles et call-graphs, pas ce que la refonte
  du générateur demandera.
