# Suivi — Chantier Genèse V2

> Journal du chantier, lot par lot. Conception (le pourquoi, les décisions de
> G0, le découpage G0 à G11) : [`../conception/genese_v2.md`](../conception/genese_v2.md).
> Branche `chantier-genese-v2`, worktree `.claude/worktrees/astres-v2`.

---

## G1 — les fondations (ouvert le 2026-11-05)

Rien ne doit changer à l'écran : G1 refait les règles de la genèse V1 sous une
autre forme, un plan de données pures puis sa construction.

**Inventaire préalable** (ce que G1 doit maîtriser) :

- Tout tire dans le **flux global de macroquad** (`gen_range`) : 55 tirages
  dans `genese/mod.rs`, 95 dans `genese/apparences.rs`, plus le tirage de
  l'étoile (`etoile.rs`). Rien ne se teste sans contexte graphique.
- **Deux constructeurs tirent en interne** : `Disque::new` (les particules) et
  `Soleil::new` (graine et minuteries des taches, éruptions, flares).
- **`Apparence` n'est pas sérialisable** (`Copy`, un `&'static str` de
  preset) : le plan stockera de quoi la refaire (type, température, graine),
  pas l'apparence elle-même.
- `glam` a déjà sa feature `serde` : les `Vec3` du plan se sérialisent.

**Découpage de G1** :

| Sous-lot | Contenu | État |
|---|---|---|
| **G1.1** | Le hasard en flux nommés (`genese_v2::alea`) | ✅ |
| **G1.2** | Le plan : types sérialisables (étoiles en arbre, planètes, lunes, ceintures, comètes, grand disque), identifiant = place dans le plan, aller-retour JSON | ✅ |
| **G1.3** | `planifier(graine)` : les règles de la V1 relues dans les flux nommés, apparences comprises ; testable sans contexte graphique | |
| **G1.4** | `instancier(&plan)` : construit le `Systeme` ; les constructeurs qui tirent en interne sont amorcés par la graine de leur objet ; les nombres de rendu (particules) décidés ici, jamais dans le plan | |
| **G1.5** | Skymap V3 (copie de la V2) et catégorie SKYMAP de l'accueil (V1 gelée, V2, V3) ; sauvegarde du plan | |

### G1.1 — le hasard en flux nommés (2026-11-05)

`src/genese_v2/alea.rs` : `Alea::flux(graine, nom, rang)` dérive un flux de
la graine, d'un nom et d'un rang, sans aucun état partagé ; `enfant` en dérive
un sous-flux (une lune dans celui de sa planète) sans consommer le parent.
Générateur SplitMix64, sans dépendance. Tirages : `unite`, `entre`, `entier`
(`[a, b[` comme `gen_range`), `chance`.

**Cinq tests**, chacun cassé par sabotage avant d'être admis :

| Test | Sabotage qui le fait rougir |
|---|---|
| `le_flux_rejoue_des_valeurs_figees` | hachage du nom modifié |
| `la_graine_le_nom_et_le_rang_donnent_chacun_un_autre_flux` | rang ignoré |
| `tirer_dans_un_flux_ne_deplace_pas_les_autres` | un compteur global mêlé à la dérivation |
| `un_enfant_ne_consomme_pas_son_parent` | rang de l'enfant ignoré |
| `les_tirages_sont_bornes_et_uniformes` | `unite` sur `[0, 0,5[` ; `entier` qui déborde |

Les **valeurs figées** sont celles du générateur au premier lancement : elles
gardent la reproductibilité d'une version à l'autre, pas la justesse du
mélange — c'est le test d'uniformité qui juge celui-ci.

Le module porte un `#![allow(dead_code)]` : rien ne le consomme encore hors des
tests. Il tombe en G1.5.

**Vérifié** : 997 tests verts (+ 13 de `mhd`), clippy 0.

### G1.2 — le plan, des données pures (2026-11-05)

`src/genese_v2/plan.rs` : tout ce que la genèse V1 produit, en données
sérialisables (`serde`), sans objet graphique.

| Type | Ce qu'il porte |
|---|---|
| `PlanSysteme` | la graine, l'arbre des étoiles, les corps, les disques |
| `NoeudEtoiles` | une étoile, ou une paire (séparation, excentricité, inclinaison, phase, **azimut** du périastre — tiré au déploiement par la V1) |
| `EtoilePlan` | nom du type, température, rayon, luminosité, mode de couronne, masse |
| `CorpsPlan` | nature, orbite, rayon, masse |
| `Nature` | planète ou lune (avec leur `RecetteApparence`), comète |
| `RecetteApparence` | type de corps, température d'entrée, graine |
| `OrbitePlan` | sur rails autour d'un foyer (UA) ; ou lune autour d'un `IdCorps` (unités du monde) |
| `DisquePlan` | ceinture, Kuiper ou grand disque : rayons, masse centrale, graine |
| `IdCorps` | la place du corps dans `PlanSysteme::corps` |

**Choix faits** :

- **Une source par fait** : la couleur d'une étoile, sa variante, le rémanent,
  le cas de la naine brune se **déduisent** du type ; ils ne sont pas stockés.
  Les sillons du grand disque se relisent sur les orbites des planètes.
- **L'apparence n'est pas stockée** (non sérialisable, 95 tirages) : une
  recette de trois valeurs la refait. La température qu'elle porte est
  l'**entrée de la recette**, figée au tirage ; la température d'équilibre que
  lit le jeu se recalcule (§7.4).
- **Miroirs plutôt que `serde` partout** : `TypeCorps` pour `TypePlanete`,
  `FoyerPlan` pour `Foyer` ; les types de la V1 gelée ne bougent pas.
- **Les étoiles n'ont pas d'`IdCorps`** : un foyer les désigne par leur rang de
  feuille dans l'ordre de déploiement, comme la V1. À revoir si Lagrange (G9)
  veut un identifiant commun.
- **Unités reprises de la V1** : UA autour d'un foyer, unités du monde pour
  l'orbite d'une lune. G3 refera ces conventions.
- **Pas de nom propre** : les systèmes générés n'en ont pas ; les systèmes
  écrits à la main (G10) l'ajouteront.

**Trois tests**, chacun cassé par sabotage :

| Test | Sabotage qui le fait rougir |
|---|---|
| `le_plan_revient_intact_de_json` | `#[serde(skip)]` sur l'azimut d'une orbite |
| `un_identifiant_designe_toujours_le_meme_corps` | un corps ajouté en tête de liste |
| `aucun_nombre_de_rendu_dans_le_plan` | un champ `nb_particules` dans `DisquePlan` (ajouté aussi au test pour compiler : la garde seule devait juger) |

**Vérifié** : 1 000 tests verts (+ 13 de `mhd`), clippy 0.
