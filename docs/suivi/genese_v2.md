# Suivi — Chantier Genèse V2

> Journal du chantier, écrit pour **une revue globale** : chaque lot se lit
> seul, sur le même gabarit — **le fond** (pourquoi, ce que ça change), **la
> forme** (fichiers, structure, choix), **les écarts** (avec la V1 ou la
> conception), **les tests** (et le sabotage qui fait rougir chacun),
> **ce qui reste**. Conception — décisions de G0 et découpage G0 à G11 :
> [`../conception/genese_v2.md`](../conception/genese_v2.md).
>
> Branche `chantier-genese-v2`, worktree `.claude/worktrees/astres-v2`.

---

## Vue d'ensemble

**Le but du chantier** : une genèse qui produit des systèmes à l'**échelle
vraie**, cohérents (âge, ceintures, lunes espacées, Lagrange), dont chaque
corps porte une **fiche** que l'économie lira, et dont le **plan** se
sauvegarde. Elle vit dans `src/genese_v2/`, à côté de la genèse V1
(`src/genese/`), **gelée** : les Skymaps V1 et V2 tournent sur la V1, la V3
sur la V2 (conception §7.12, §7.13).

| Lot | Contenu | État |
|---|---|---|
| G0 | Les décisions | ✅ 2026-11-05 (conception §7) |
| **G1** | **Les fondations : hasard, plan, planifier, instancier, Skymap V3** | **en cours** |
| G2 | Banc statistique, coût par image V2 / V3 | |
| G3 | L'échelle vraie et le réglage d'affichage | |
| G4 | Un seul modèle de température, habitabilité par bandes | |
| G5 | Les étoiles : âge, anomalies, table de rareté | |
| G6 | Planètes et lunes : catalogue, espacement, naines, noms | |
| G7 | Ceintures | |
| G8 | La fiche, les ressources | |
| G9 | Lagrange, Grecs et Troyens | |
| G10 | Les étoiles proches | |
| G11 | Étalonnage, la V3 devient principale | |

**Commits du chantier** : `da0a091` (G1.1), `c812051` (G1.2), `7b166f1`
(G1.3), `c054c6f` (G1.4), G1.5 (ce lot).

---

## G1 — les fondations (ouvert le 2026-11-05)

**Fond.** Avant toute règle nouvelle, la genèse change de **forme** : un
**plan** de données pures, tiré de flux de hasard indépendants, puis sa
**construction**. Rien ne doit changer à l'écran — G1 refait les règles de la
V1. Ce qu'on y gagne : une genèse **testable** sans fenêtre, un plan
**sauvegardable** (§7.11), et le point d'accroche du réglage de qualité
(Opti-v2 O5 : la construction décide combien on dessine).

**Inventaire préalable** — ce que G1 doit maîtriser :

- Tout tire dans le **flux global de macroquad** (`gen_range`) : 55 tirages
  dans `genese/mod.rs`, 95 dans `genese/apparences.rs`, plus le tirage de
  l'étoile (`etoile.rs`). Rien ne se teste sans contexte graphique.
- **Deux constructeurs tirent en interne** : `Disque::new` (les particules) et
  `Soleil::new` (graine et minuteries des taches, éruptions, flares).
- **Des tirages ont lieu pendant la construction** : l'orientation de chaque
  orbite (`poser_sur_rails`, `deployer_noeud`), la place d'une lune (qui lit
  la position de sa planète).
- **`Apparence` n'est pas sérialisable** (`Copy`, un `&'static str` de
  preset) ; et ses fonctions de tirage rendent aussi le **rayon** et la
  **masse**, dont dépend le placement (garde-fou anti-collision).
- `glam` a déjà sa feature `serde` : les `Vec3` du plan se sérialisent.

| Sous-lot | Contenu | État |
|---|---|---|
| **G1.1** | Le hasard en flux nommés (`genese_v2::alea`) | ✅ `da0a091` |
| **G1.2** | Le plan : types sérialisables, identifiant = place dans le plan | ✅ `c812051` |
| **G1.3** | `planifier(graine)` : les règles de la V1 relues dans les flux nommés, apparences comprises | ✅ `7b166f1` |
| **G1.4** | `instancier(&plan)` : construit le `Systeme` ; constructeurs qui tirent en interne amorcés par la graine de leur objet ; nombres de rendu décidés ici | ✅ `c054c6f` |
| **G1.5** | Skymap V3 (copie de la V2), catégorie SKYMAP de l'accueil (V1 gelée, V2, V3) | ✅ |
| **G1.6** | La sauvegarde du plan : sauver et recharger un système dans la V3 | |

---

### G1.1 — le hasard en flux nommés (2026-11-05, `da0a091`)

**Fond.** La V1 tire tout dans **une seule file** de hasard : un tirage
ajouté n'importe où décale tout ce qui suit (le lot 12 a dû poser son grand
disque *sans tirage* pour cette raison), et cette file n'existe qu'avec une
fenêtre ouverte, donc rien ne se teste. Ici, chaque partie d'un système a
**sa propre file**, dérivée de la graine, d'un nom et d'un rang : tirer dans
les comètes ne déplace plus les planètes.

**Forme.** `src/genese_v2/alea.rs`, une structure `Alea` :

- `Alea::flux(graine, nom, rang)` : la graine mélangée, puis chaque octet du
  nom, puis le rang — aucun état partagé entre flux ;
- `enfant(nom, rang)` : un sous-flux (une lune dans sa planète) **sans
  consommer** de tirage du parent ;
- tirages `suivant` (64 bits), `unite` ([0, 1[, 24 bits), `entre`, `entier`
  (`[a, b[`, comme `gen_range`), `chance`.

Générateur **SplitMix64** : une vingtaine de lignes, sans dépendance, calcul
entier pur, donc identique sur toutes les machines.

**Écarts / limites.** `entier` passe par un modulo : biais de l'ordre de
10⁻¹⁸ sur de petits intervalles, sans effet. Deux flux de même nom et rang
sont identiques — les noms sont tenus dans un tableau en tête de
`planifier.rs` (G1.3).

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `le_flux_rejoue_des_valeurs_figees` | les sorties ne changent jamais d'une version à l'autre | hachage du nom modifié |
| `la_graine_le_nom_et_le_rang_donnent_chacun_un_autre_flux` | chaque ingrédient compte | rang ignoré |
| `tirer_dans_un_flux_ne_deplace_pas_les_autres` | l'indépendance, cœur du lot | un compteur global mêlé à la dérivation |
| `un_enfant_ne_consomme_pas_son_parent` | ajouter une lune ne touche pas sa planète | rang de l'enfant ignoré |
| `les_tirages_sont_bornes_et_uniformes` | bornes, moyenne, histogramme sur 20 000 tirages | `unite` sur [0 ; 0,5[ ; `entier` qui déborde |

Les valeurs figées sont celles du premier lancement : elles gardent la
reproductibilité, pas la qualité du mélange — c'est le test d'uniformité qui
juge celle-ci.

**Reste.** `#![allow(dead_code)]` sur le module tant que rien ne le consomme
hors des tests : tombe en G1.5.

**Vérifié** : 997 tests verts (+ 13 de `mhd`), clippy 0.

---

### G1.2 — le plan, des données pures (2026-11-05, `c812051`)

**Fond.** Le plan est **ce que la genèse décide**, séparé de **ce qu'on
dessine** : il se sauvegarde (un système découvert ne change plus, §7.11), se
lit sans rien construire (la starmap, §5.4), et ne contient aucun nombre de
rendu (la qualité ne change pas le système).

**Forme.** `src/genese_v2/plan.rs` (réalisé par un agent, revérifié) :

| Type | Ce qu'il porte |
|---|---|
| `PlanSysteme` | la graine, l'arbre des étoiles, les corps, les disques |
| `NoeudEtoiles` | une étoile, ou une paire (séparation, excentricité, inclinaison, phase, **azimut** du périastre) |
| `EtoilePlan` | nom du type, température, rayon, luminosité, mode de couronne, masse |
| `CorpsPlan` | nature, orbite, rayon, masse |
| `Nature` | planète ou lune (avec leur `RecetteApparence`), comète |
| `RecetteApparence` | type de corps, température d'entrée, graine |
| `OrbitePlan` | sur rails autour d'un foyer (UA, avec azimut) ; ou lune autour d'un `IdCorps` (unités du monde) |
| `DisquePlan` | ceinture, Kuiper ou grand disque : rayons, masse centrale, graine |
| `IdCorps` | la place du corps dans `PlanSysteme::corps` — l'identifiant définitif |

**Choix.**

- **Une source par fait** : couleur, variante, rémanent et cas de la naine
  brune se **déduisent** du type ; les sillons du grand disque se relisent
  sur les orbites des planètes.
- **L'apparence n'est pas stockée** : une recette de trois valeurs la refait.
  Sa température est l'**entrée de la recette**, figée au tirage ; la
  température d'équilibre que lit le jeu se recalcule (§7.4).
- **Miroirs plutôt que `serde` partout** : `TypeCorps`, `FoyerPlan` ; les
  types de la V1 gelée ne bougent pas.

**Écarts / limites.**

- Les étoiles n'ont pas d'`IdCorps` : un foyer les désigne par leur rang de
  feuille. À revoir si Lagrange (G9) veut un identifiant commun.
- Unités de la V1 (UA autour d'un foyer, unités du monde pour une lune) : G3
  les refera.
- Pas de nom propre : les systèmes écrits à la main (G10) l'ajouteront.

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `le_plan_revient_intact_de_json` | aucun champ perdu à la sauvegarde | `#[serde(skip)]` sur l'azimut d'une orbite |
| `un_identifiant_designe_toujours_le_meme_corps` | un ajout ne déplace pas les identifiants | un corps ajouté en tête de liste |
| `aucun_nombre_de_rendu_dans_le_plan` | §5.4 | un champ `nb_particules` dans `DisquePlan` (ajouté aussi au test pour compiler : la garde seule devait juger) |

**Vérifié** : 1 000 tests verts (+ 13 de `mhd`), clippy 0.

---

### G1.3 — planifier (2026-11-05, `7b166f1`)

**Fond.** La genèse devient une **fonction pure** : `planifier(graine)` rend
le plan d'un système, sans fenêtre, sans état global. Pour la première fois
dans le projet, on peut **bâtir des milliers de systèmes dans un test** et
vérifier leurs règles. Les règles restent celles de la V1, tirage pour
tirage ; seule la source du hasard change.

**Forme.**

- `src/genese_v2/planifier.rs` :
  - `planifier(graine)` : le genre (55 % simple, puis binaire, trinaire,
    quadruple), puis `simple` ou `multiple` ;
  - `planifier_autour(graine, type)` : une étoile d'un type choisi (menu
    « choisir astre », starmap) ;
  - `simple` : étoile, échelle, Titius-Bode avec garde-fou anti-collision,
    lunes, ceinture, grand disque (étoile jeune), Kuiper, comètes ;
  - `multiple` : l'arbre stellaire et ses zones stables (Holman-Wiegert),
    puis `peupler_zone` ;
  - un **tableau des noms de flux** en tête du fichier (`genre`, `etoile`,
    `arbre`, `echelle`, `zone` ↳ `planete` ↳ `lune`, `ceinture`, `kuiper`,
    `grand_disque`, `cometes` ↳ `comete`).
- `src/genese_v2/apparences.rs` : **copie** de `genese::apparences` avec un
  `Alea` à la place de `gen_range` (la V1 est gelée, et G4/G6 remplaceront ces
  apparences par le catalogue). `planete(recette)` rend `(rayon, masse,
  apparence)` ; `lune(recette)` l'apparence d'une lune.
- Réutilisé de la V1, sans le modifier : `echelle_systeme`, `premiere_orbite`,
  `masse_luminosite`, `orbite_comete`, les masses des cas particuliers,
  `stabilite`. Deux visibilités élargies pour y accéder : `genese::taille` et
  `etoile::PART_PARTICULIERES` passent `pub(crate)`.

**Choix.**

- **Le rayon et la masse sont tirés dans le flux de la recette**, puis
  stockés : la construction refera l'apparence avec la même graine et
  retrouvera le même corps (`la_recette_refait_le_meme_corps`).
- **Les tirages de construction remontent dans le plan** : l'azimut de chaque
  orbite et de chaque paire, la place et le sens de chaque lune.
- **Chaque disque a son flux** (`ceinture`, `kuiper`, `grand_disque`) : le
  grand disque, qui n'existe que pour les étoiles jeunes, ne décale plus
  Kuiper — le piège du lot 12 disparaît.

**Écarts avec la V1** (voulus, à juger en G1.5 à l'écran) :

- **Les mêmes graines ne donnent pas les mêmes systèmes** : le hasard est
  autre. On compare à type d'étoile égal et en statistiques (§7.12).
- **Le genre se tire** dans un flux (`genre`) au lieu de `graine % 100`.
- **La place d'une lune** se calcule sur la distance de sa planète **à son
  foyer**, `a·(1−e)`. La V1 mesurait la distance au **centre du repère** : pour
  une planète autour d'une étoile d'un multiple, cette distance comptait
  l'écart de l'étoile, et ses lunes partaient trop loin. Les systèmes simples
  ne changent pas.
- **Le cadrage par défaut d'un multiple** (`definir_vue`) n'est pas dans le
  plan : c'est de l'affichage, G1.4 le déduira des planètes.

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `la_meme_graine_rejoue_le_meme_plan` | déterminisme | un compteur caché dans le flux d'échelle |
| `la_recette_refait_le_meme_corps` | le corps construit sera le corps tiré | recette stockée et corps tirés de graines différentes |
| `les_comptes_suivent_les_regles_de_la_v1` | 3–6 planètes, 1–3 comètes, deux ceintures (simple) ; rien de tout ça en multiple ; ≤ 3 lunes, ≤ 1 hors géante | jusqu'à 7 planètes |
| `les_planetes_voisines_ne_se_chevauchent_pas` | le garde-fou, et qu'il **mord** vraiment | garde-fou retiré ; garde-fou élargi (il ne mord plus jamais au plus juste) |
| `les_lunes_restent_dans_le_domaine_de_leur_planete` | entre Roche et ~14 % de la distance à l'étoile | lunes repoussées d'une demi-bande |
| `un_systeme_sur_deux_environ_a_plusieurs_etoiles` | 40–50 % de multiples sur 2 000 graines | seuil des simples à 35 % |
| `planifier_autour_impose_le_type` | le type demandé est le type tiré ; type inconnu → `None` | type ignoré |

**Reste.** Le plan existe, rien ne le construit : G1.4.

**Vérifié** : 1 007 tests verts (+ 13 de `mhd`), clippy 0.

---

### G1.4 — instancier (2026-11-05, `c054c6f`)

**Fond.** Le plan devient un `Systeme` que l'écran sait dessiner. La règle
de §5.4 tient : **aucun tirage de genèse à la construction** — tout ce qui
fait le système est dans le plan — et **les nombres de rendu se décident
ici**, pas dans le plan. C'est le point d'accroche que la part « nombre
d'objets » d'Opti-v2 O5 attendait.

**Forme.** `src/genese_v2/instancier.rs`, `instancier(&plan) -> (Systeme,
String)` (le libellé, comme la V1) :

- **calculs purs, testés** : `orbite_rails` (Kepler, départ au périastre dans
  la direction de l'azimut), `config_disque` (les particules selon la
  qualité), `vue` (le cadrage d'un multiple) ;
- **assemblage** : l'étoile seule (ou la naine brune en géante au centre),
  l'arbre stellaire déployé avec l'azimut du plan, puis planètes et lunes,
  disques, comètes — l'ordre de la V1.

**Choix.**

- **Les constructeurs qui tirent en interne sont amorcés** (`srand`) :
  `Soleil::new` par une graine dérivée du plan (flux `soleil`, k),
  `Disque::new` par la graine du disque. Déterministe, donc rejoué à
  l'identique ; marqué `ponytail:` — disparaît si ces objets reçoivent un
  jour leur `Alea`.
- **Le profil d'une étoile se refait depuis son type** : nom statique et
  couleur relus dans le catalogue d'`etoile.rs` (une source par fait).
- **Particules selon la qualité** : 900 (ceinture) et 1 400 (Kuiper) en
  haute, **la moitié en basse** ; le grand disque aussi. Le disque, lui, ne
  change pas. Changer la qualité en cours de partie ne reconstruit pas un
  système déjà ouvert.

**Écarts avec la V1.**

- **Le cadrage d'un multiple** se déduit des planètes du plan (l'apoastre le
  plus lointain autour de la première étoile qui en porte), là où la V1
  lisait la borne de la zone stable, absente du plan.
- **La qualité basse divise les particules par deux** : nouveau, à mesurer
  en G2 (coût par image sur le petit PC).
- `camera::DEMI_FOV` passe `pub(crate)` : le test du cadrage lit le champ de
  la caméra à sa source.

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `un_corps_part_de_son_periastre_dans_la_direction_de_son_azimut` | distance `a(1−e)`, direction de l'azimut, pente de l'inclinaison | azimut décalé ; inclinaison ignorée |
| `la_qualite_change_les_particules_pas_le_disque` | rayons identiques, particules divisées en basse | qualité sans effet |
| `le_cadrage_vise_l_etoile_qui_porte_des_planetes` | l'apoastre le plus lointain tient dans le champ de la caméra | cadrage six fois trop serré |

⚠️ **Un test trop faible, renforcé.** La première version du test de cadrage
vérifiait seulement que les planètes étaient plus près que la caméra : le
sabotage « cadrage six fois trop serré » restait vert, le plancher de 150
unités suffisant à le satisfaire. Il vérifie maintenant que l'apoastre tient
dans le **champ de vision** — et il rougit.

**Ce qui ne se teste pas ici.** L'assemblage crée des objets qui chargent des
matériaux GPU : il ne tourne pas sans fenêtre. Il se vérifie à l'écran, dans
la Skymap V3 (G1.5).

**Vérifié** : 1 010 tests verts (+ 13 de `mhd`), clippy 0.

---

### G1.5 — la Skymap V3 (2026-11-05)

**Fond.** La genèse V2 devient **visible** : un écran la montre, à côté des
Skymaps V1 et V2, pour comparer (§7.12, §7.13). C'est aussi la première
vérification de l'assemblage d'`instancier`, qui ne tourne qu'avec une
fenêtre.

**Forme.**

- `src/ecran/skymap_v3.rs` : **copie** de `skymap_v2.rs`. Deux fonctions
  locales, `construire_systeme` et `construire_systeme_autour`, passent par
  la genèse V2 (planifier puis instancier) sous les mêmes noms que la V1 : le
  reste de l'écran est celui de la V2, ligne pour ligne. Le libellé des
  systèmes générés porte **`[V3]`**. Les systèmes écrits à la main (Soleil,
  Tau Ceti, Alpha Centauri…) restent ceux de la V1 jusqu'à G10.
- **Accueil** : une colonne **[ SKYMAP ]** à gauche, étroite (180 px) — **V1
  GELEE**, **V2 MARQUEURS**, **V3 GENESE V2** —, les deux entrées Skymap
  retirées du bloc des astres ; trois colonnes centrées (`colonnes`), 980 px
  dans la fenêtre de 1 000.
- `main.rs` : `Cible::SkymapV3`, `Etat::SkymapV3`.
- `genese_v2` : l'`#![allow(dead_code)]` du module **tombe** ; reste un
  `allow` ciblé sur `PlanSysteme::corps` (lu par les tests, puis par G1.6 et
  G9).

**Écarts.**

- **Les outils de mesure de la V2 ne sont pas recopiés** (`sur_astre`,
  `cadrer`, `sur_graine` : `SYSTEME`, `FOCUS`, `DIAG=tour`) : rien ne les
  appelait en V3. **G2 les rebranchera** pour mesurer la V3 contre la V2.
- **La sauvegarde du plan passe en G1.6** : elle demande une interface
  (sauver, recharger) que la V2 n'a pas, et G1.5 se limite à voir la V3.
- La V3 s'ouvre, comme la V2, sur le système solaire (preset V1) : un système
  de la genèse V2 s'obtient par ALEATOIRE ou CHOISIR ASTRE.

**Essai de fumée** (temporaire, non commité) : le jeu lancé avec un drapeau
qui construit **400 systèmes de la genèse V2 dans la vraie fenêtre**, puis
les cinq types particuliers (géante rouge, naine blanche, pulsar, naine brune
au centre, trou noir) : **aucune erreur**, 4 226 astres en 0,57 s.
`main.rs` restauré par copie, vérifié identique.

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `les_trois_colonnes_tiennent_dans_la_fenetre_par_defaut` | trois colonnes dans 1 000 px, sans chevauchement | écart de 120 px ; écart négatif |

⚠️ **Une première version vérifiait aussi la largeur des libellés**, avec une
police estimée à 12 px par caractère : elle refusait un libellé existant
(« PETITES STATIONS… ») qui tient pourtant. La largeur d'un libellé dépend
de la police, qui ne se mesure qu'avec une fenêtre : elle se juge à l'œil,
plutôt que contre une valeur supposée.

**À vérifier à l'écran** (par l'utilisateur) : les libellés de la colonne
SKYMAP tiennent dans leurs boutons ; la V3 montre des systèmes de la genèse
V2 (`[V3]` dans le bandeau) comparables à ceux de la V2.

**Vérifié** : 1 011 tests verts (+ 13 de `mhd`), clippy 0.
