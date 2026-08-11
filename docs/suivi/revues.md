# Suivi — Revues de code

> Les passes de relecture systématique, ce qu'elles ont trouvé, et ce qu'elles
> ont appris sur la façon de relire.
> Point de reprise : [`../../STATE.md`](../../STATE.md).
>
> Extrait de `STATE.md` le 2026-08-10.

---

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

### P1 — le bruit qui cache les vrais défauts ✅ TERMINÉ

C'était **la leçon de la session** : la palette Lospec était refusée avec une
ligne de console, invisible au milieu du bruit. Tant que `clippy` sortait 117
avertissements, le 118ᵉ ne se voyait pas.

**117 → 0.** `cargo clippy --all-targets` est propre.

| # | Quoi | Volume | État |
|---|---|---|---|
| 6 | Imports inutilisés | 9 | ✅ |
| 7 | Code mort | **59** | ✅ trié |
| 8 | `assertions_on_constants` — légitimes, elles gardent une constante | 12 | ✅ `#[allow]` commenté sur les 6 modules de test |
| 9 | Style : `field_reassign_with_default`, `needless_range_loop`, `doc_lazy_continuation`, `too_many_arguments`, `type_complexity`, `useless_conversion`, `needless_update`, `clone_on_copy`, `manual_checked_ops`, `single_char_add_str`, `wrong_self_convention`, `unused_*` | ~50 | ✅ |

#### ⚠️ La mesure précédente était fausse

STATE.md disait : « 55 éléments morts pour la cible binaire, 57 avec les tests —
donc il est bien inutilisé ». **Non.** En comparant vraiment les deux cibles :

```
morts pour le binaire ................ 90
morts pour le binaire ET les tests ... 13
```

**77 des 90 sont exercés par les tests** — ils ne sont pas morts, ils sont
*écrits avant leur consommateur*. `dead_code` n'a aucune notion de « utilisé par
les tests » : il compte sur la cible binaire, et tout ce que seuls les tests
appellent y ressort mort. La conclusion inverse aurait mené à supprimer du code
testé et qui marche.

#### Ce qui a été fait, par groupe

- **`chantier.rs` et `symetrie.rs`** : `#![allow(dead_code)]` **au module**, avec
  la raison en tête de fichier (l'écran d'assemblage du Lot 4 est le
  consommateur). 27 éléments.
- **Le reste** : `#[allow(dead_code)]` **par élément**, chacun avec sa raison sur
  la même ligne — catalogue de l'assembleur, économie du chantier (D-INT-2),
  interaction/désignation, mesures de géométrie tenues par leurs tests.
- **Supprimé** : `Camera::zoom_max()`, sans aucun appelant, et intestable
  (`Camera::new` exige un contexte GL). Ajoutée la session d'avant, jamais
  branchée.
- **`Palette::depuis_hex`** passée en `#[cfg(test)]` : le jeu lit des fichiers.

L'autorisation est **volontairement locale**. Contrôle rouge : une fonction morte
neuve ajoutée dans `orbite.rs` sort **un** avertissement, seul et visible. C'est
tout l'objet de la manœuvre.

#### 🔴 Un vrai défaut trouvé sous le bruit : `MASSES_TERRESTRES`

La table des masses planétaires se documentait comme la **source unique** — et
**personne ne la lisait**. Les masses étaient écrites à la main à **trois**
endroits : la table, chaque appel `poser(...)` du preset, et `parent_de` dans les
tests. C'est exactement le défaut que la table existait pour empêcher, et le test
de Kepler ne pouvait pas le voir : il vérifiait **sa propre copie**.

Corrigé par `masse_monde(nom)` — le pendant de `rayon_monde` — que le preset et
les tests appellent tous les deux. Contrôle rouge : mettre Jupiter à 31,78 au
lieu de 317,8 fait tomber `les_periodes_de_lunes_suivent_kepler` (Lune/Io = 4,91
au lieu de 15,44). Avant, cette même corruption passait au vert.

C'est la justification de la revue en une ligne : **le défaut était visible
depuis le début, noyé dans 117 avertissements.**

#### Deux choix assumés

- **`too_many_arguments` (7 sites) : autorisés, pas refactorés.** Ce sont des
  listes cohérentes — éléments orbitaux, uniforms de shader, dimensions d'une
  poutre. Un type `ElementsOrbitaux` serait plus propre et touche **tous** les
  presets : c'est un chantier à part, pas un à-côté de passe de nettoyage.
- **Ce qui reste candidat à la suppression** (mort dans les deux cibles, gardé
  faute d'une décision qui n'est pas la mienne) : `police::mesure`,
  `sprites::presente`, `Systeme::mode`, `Apparence::avec_eyeball` (remplacée par
  `avec_eyeball_zones`), `Apparence::avec_cryo`, `generateur::preset_isv`
  (remplacée par `preset_isv_fixe` + `preset_isv_equipage`), `vaisseau::voile` et
  `Peintre::voile`. Chacune porte sa raison en commentaire.

### P2 — dettes déjà inscrites

~~D-PARAM-1~~ (soldée), D-PIX-1/3/4 (D-PIX-5 sans objet), `briques.rs`/`vaisseaux.rs`
non déclarés dans `ecran/mod.rs`, hash CPU/GPU dupliqué dans 4 fichiers.

### Ce que la revue a appris

**Deux de mes trois règles candidates étaient trop strictes, et le catalogue avait
raison.** Les rivières de `Crevasse` sont des coulées de **lave** (`riv_lave`), et
la végétation de `Lichen` et `Fungi` prend son humidité dans l'**air**, pas dans
un océan. Ce sont des cas limites délibérés. Les tests encodent désormais la
règle juste — *une rivière coule d'eau ou de lave*, *la végétation veut de l'eau
ou une atmosphère* — et chacun vérifie que **le cas limite existe encore**, sans
quoi la règle deviendrait vide sans que rien ne le dise.

