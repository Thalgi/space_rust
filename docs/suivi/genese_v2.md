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
| G1 | Les fondations : hasard, plan, planifier, instancier, Skymap V3 | ✅ 2026-11-05 |
| G2 | Banc statistique, outils de mesure, coût par image V2 / V3 | ✅ 2026-11-05, petit PC reporté |
| **G3** | **L'échelle vraie et le réglage d'affichage** | **en cours** |
| G4 | Un seul modèle de température, habitabilité par bandes | |
| G5 | Les étoiles : âge, anomalies, table de rareté | |
| G6 | Planètes et lunes : catalogue, espacement, naines, noms | |
| G7 | Ceintures | |
| G8 | La fiche, les ressources | |
| G9 | Lagrange, Grecs et Troyens | |
| G10 | Les étoiles proches | |
| G11 | Étalonnage, la V3 devient principale | |

**Commits du chantier** : `da0a091` (G1.1), `c812051` (G1.2), `7b166f1`
(G1.3), `c054c6f` (G1.4), `abc7366` (G1.5), `635d555` (clôture de G1), `2b6ef58` (G2.1), `ffd19fa` (G2.2), `19646a3` (clôture de G2), `c49fe15` (G3.1), G3.2 (dernier commit).

---

## G1 — les fondations (ouvert et clos le 2026-11-05)

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
| **G1.5** | Skymap V3 (copie de la V2), catégorie SKYMAP de l'accueil (V1 gelée, V2, V3) | ✅ `abc7366` |
| ~~G1.6~~ | L'écriture du plan sur disque | **reporté** à la sauvegarde de partie (voir le bilan) |

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
- ~~La place d'une lune corrigée par rapport à la V1~~ : **affirmation fausse**,
  corrigée en G2.1. Le calcul est le même que celui de la V1.
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

### G1.5 — la Skymap V3 (2026-11-05, `abc7366`)

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
  `allow` ciblé sur `PlanSysteme::corps` (lu par les tests, puis par la
  sauvegarde de partie et G9).

**Écarts.**

- **Les outils de mesure de la V2 ne sont pas recopiés** (`sur_astre`,
  `cadrer`, `sur_graine` : `SYSTEME`, `FOCUS`, `DIAG=tour`) : rien ne les
  appelait en V3. **G2 les rebranchera** pour mesurer la V3 contre la V2.
- **La sauvegarde du plan n'est pas dans ce lot** : elle demande une interface
  (sauver, recharger) que la V2 n'a pas (voir le bilan de G1).
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

**Jugé à l'écran par l'utilisateur** : *« je vois des vues comparables »* —
la V3 montre des systèmes de la genèse V2 comparables à ceux de la V2, ce que
G1 devait garantir. Pas de remarque sur les libellés de la colonne SKYMAP.

**Vérifié** : 1 011 tests verts (+ 13 de `mhd`), clippy 0.

---

### Bilan de G1 (clos le 2026-11-05)

**Livré.** La genèse a changé de forme sans changer ce qu'on voit :
`planifier(graine)` rend un plan de données pures, `instancier(&plan)` le
construit, la Skymap V3 le montre à côté des V1 et V2. L'utilisateur juge les
vues comparables. 1 011 tests verts, dont 19 nouveaux, plus l'essai de fumée.

**Reporté : l'écriture du plan sur disque (G1.6, décision de l'utilisateur).**
La sauvegarde de systèmes avait été retirée du menu le 2026-10-19 ; la remettre
n'aurait servi qu'à sauver des plans à refaire après G3 (les unités changent).
Le plan reste **sérialisable**, gardé par `le_plan_revient_intact_de_json`.
L'écriture viendra avec la sauvegarde de partie.

**Dettes laissées, assumées** :

| Dette | Où | Se solde |
|---|---|---|
| `Soleil::new` et `Disque::new` tirent dans le flux global, amorcé avant l'appel | `instancier.rs` (`ponytail:`) | si ces objets reçoivent leur `Alea` |
| Apparences copiées de la V1 | `genese_v2/apparences.rs` | G4, G6 (catalogue) |
| Écran copié de la V2 (934 lignes) | `ecran/skymap_v3.rs` | jamais tant que la V2 reste gardée (§7.13) |
| Systèmes écrits à la main encore ceux de la V1 | `skymap_v3.rs` | G10 |
| Outils de mesure absents de la V3 | `skymap_v3.rs` | ✅ soldée en G2.2 |
| Particules divisées par deux en qualité basse, sans mesure | `instancier.rs` | G2.3 (petit PC) |
| L'assemblage d'`instancier` sans test automatique : il lui faut une fenêtre | `instancier.rs` | essai de fumée à rejouer à chaque lot qui y touche |

---

## G2 — le banc et la mesure (ouvert et clos le 2026-11-05)

**Fond.** G1 affirme que la genèse V2 reprend les règles de la V1, et
l'utilisateur juge les vues comparables à l'œil. G2 le **mesure**, et fixe le
coût par image (critère du §7.7) avant que G3 ne change l'échelle.

| Sous-lot | Contenu | État |
|---|---|---|
| **G2.1** | Le banc statistique : les deux genèses sur des milliers de graines (`BANC=<n>`) | ✅ `2b6ef58` |
| **G2.2** | Les outils de mesure dans la V3 : un réglage choisit la Skymap mesurée par `SYSTEME`, `FOCUS`, `DIAG=tour` ; mesure sur la machine de dev | ✅ `ffd19fa` |
| ~~G2.3~~ | Le coût par image V3 contre V2 sur le petit PC | **reporté** : un passage après G3, un après G7 |

---

### G2.1 — le banc statistique (2026-11-05, `2b6ef58`)

**Fond.** Comparer deux genèses dont les graines ne donnent pas les mêmes
systèmes ne peut se faire qu'en statistiques (§7.12).

**Forme.** `src/genese_v2/banc.rs`, lancé par `BANC=<n> cargo run --release`
(`main.rs`), qui imprime le rapport et quitte :

- **le même code mesure les deux genèses, sur les systèmes construits** : un
  écart au rapport vient des genèses, pas de la mesure. D'où un banc jeu lancé :
  construire un système charge des matériaux GPU ;
- `Bilan` compte par section (étoiles par système, type d'étoile, classe des
  planètes et des lunes, lunes par planète, comètes, disques) et fait des
  moyennes (rayons, périastres, orbites des lunes) ;
- `rapport` met les deux côte à côte, sur l'**union** des valeurs vues, et marque
  `*` un écart à plus de 3 écarts-types (test de deux proportions).

**Résultat, 5 000 graines par genèse** : **aucun écart significatif** sur toutes
les parts. Quelques lignes (V1 / V2) :

| Grandeur | V1 | V2 |
|---|---:|---:|
| Systèmes à 1 / 2 / 3 / 4 étoiles | 55,0 / 28,0 / 11,0 / 6,0 % | 55,1 / 28,1 / 10,8 / 6,0 % |
| Planètes habitables | 18,5 % | 18,4 % |
| Gazeuses sans lune | 34,5 % | 34,8 % |
| Périastre de la 1re planète | 0,635 UA | 0,633 UA |
| Périastre de la dernière planète | 4,463 UA | 4,479 UA |
| Orbite d'une lune, simples | 6,63 | 6,55 |
| Orbite d'une lune, multiples | 5,04 | 4,94 |

**Le banc sait voir** : deux règles de la V2 sabotées (lunes des gazeuses 1–2 au
lieu de 1–3 ; seuil de type 0,72 au lieu de 0,80), il marque sept lignes, dont
les classes de planètes décalées de moins de 2 points. Sans ce contrôle, son
« aucun écart » ne prouvait rien.

**Ce que le banc a corrigé** :

- ⚠️ **Une affirmation fausse de G1.3.** J'y annonçais une correction de la V1 :
  elle aurait mesuré la place des lunes depuis le centre du repère. **C'est faux** :
  `Systeme::ajouter` ne déplace rien, la planète est encore à `a(1−e)` de son
  foyer quand la V1 pose ses lunes. Les deux calculs sont identiques, et le banc
  le confirme (5,04 contre 4,94). Lu dans le code sans être mesuré.
- **Une borne de test fausse** : `les_comptes_suivent_les_regles_de_la_v1` bornait
  les multiples à 8 planètes, le banc en voit 9. Le test passait par chance sur
  ses 2 000 graines. Le maximum réel est de 12 (trois zones de quatre planètes).

**Limites.** Les **moyennes** ne passent pas au test d'écart : elles se lisent à
l'œil. Le banc compare les parts, pas les distributions de distance. Il ne juge
pas si une règle est bonne, seulement si les deux genèses l'appliquent pareil :
un tiers des gazeuses sans lune, c'est la règle de la V1, que §7.3 remplacera.

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `le_type_se_lit_dans_le_libelle` | le type sans sa pollution ; `None` en multiple | pollution gardée |
| `seul_un_vrai_ecart_est_signale` | le bruit ne marque pas, un vrai écart si | seuil à 0,1 σ ; seuil à 100 σ |
| `une_valeur_d_une_seule_genese_se_voit_au_rapport` | l'union des valeurs | valeurs lues d'un seul côté |
| `les_comptes_…` (borne corrigée) | au plus 12 planètes en multiple | cinq à huit planètes par zone |

**Vérifié** : 1 014 tests verts (+ 13 de `mhd`), clippy 0.

---

### G2.2 — les outils de mesure dans la V3 (2026-11-05, `ffd19fa`)

**Fond.** Le critère du §7.7 (la V3 ne coûte pas plus cher que la V2) demande de
mesurer les deux dans les mêmes conditions. Les outils de mesure ne visaient
que la V2.

**Forme.** `main.rs` et `ecran/skymap_v3.rs` (`sur_astre`, `cadrer`, `sur_graine`
recopiés de la V2) :

| Réglage | Effet |
|---|---|
| `SKYMAP=v3` | `SYSTEME`, `FOCUS`, `DIAG=tour`, `DIAG=systemes` visent la V3 |
| `DIAG=systemes` | le tour réduit : **60 graines réparties** et le système solaire, ~214 s |
| `QUALITE=basse` | la qualité posée **avant** la construction : les particules de la V3 s'y décident, et les phases de `DIAG` changent la qualité trop tard |

Le tour imprime `moyenne des graines : image, erreur type, CPU`.

**Le premier protocole était faux, et il flattait la V3.** Le premier passage
(moyenne de toutes les lignes « systeme X » et de 8 graines) donnait la V3 **15 à
20 % plus rapide**, sans cause possible puisque le banc voit les mêmes systèmes
(G2.1). Deux défauts :

- les 28 lignes « systeme X » partagent **la graine 1**, donc en V3 presque le
  même tirage de planètes : ce n'étaient pas 28 échantillons, mais à peu près un ;
- les graines 1 à 8 sont toutes à une étoile en V1 (`graine % 100`), pas en V3.

D'où les 60 graines réparties (`graines_du_tour`), qui couvrent les genres de la
V1, et une moyenne limitée aux graines.

**Mesures, machine de dev, 2 560 × 1 440, dans l'ordre où elles ont tourné** :

| # | Skymap | Qualité | Image (ms) | Erreur type | CPU (ms) |
|---:|---|---|---:|---:|---:|
| 1 | V2 | haute | 1,305 | 0,036 | 0,636 |
| 2 | V3 | haute | 1,311 | 0,031 | 0,631 |
| 3 | V3 | basse | 1,148 | 0,028 | 0,550 |
| 4 | V2 | haute | 0,999 | 0,035 | 0,486 |
| 5 | V3 | haute | 0,935 | 0,024 | 0,412 |
| 6 | V3 | basse | 1,261 | 0,031 | 0,814 |

**Ce qu'on peut en dire** :

- **La dérive de la machine domine.** La même V2 sur les mêmes graines passe de
  1,305 à 0,999 ms (médiane 0,72 par graine). Elle varie même dans un tour : le
  dernier tiers du tour 4 remonte à 1,10 ms contre 0,95 au début. Cause non
  établie (état de la machine). Elle dépasse largement l'erreur type, qui ne
  mesure que la dispersion entre graines.
- **La V3 ne coûte pas plus cher que la V2 ici** : tours voisins, 1,305 / 1,311
  et 0,999 / 0,935.
- **Le gain de la qualité basse n'est pas mesuré** : une fois plus bas que son
  voisin (1,148 contre 1,311), une fois plus haut (1,261 contre 0,935). À ~1 ms
  par image, la machine de dev ne tranche pas. Le petit PC le dira.

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `le_tour_des_systemes_ne_garde_que_les_systemes` | les graines et le système solaire, rien d'autre | la starmap dans le tour réduit |
| `les_graines_du_tour_couvrent_les_genres_de_la_v1` | parts proches de 55 / 28 / 11 / 6 % | graines qui se suivent (1 à 60) |
| `la_moyenne_ne_compte_que_les_graines` | ni les lignes « systeme X » ni les autres ; l'erreur type | lignes « systeme » comptées ; écart-type au lieu de l'erreur type |

**Reste.** `SKYMAP`, `DIAG=systemes` et `QUALITE` ne se testent qu'en lançant le jeu
(ils ont servi aux six tours ci-dessus).

**Vérifié** : 1 017 tests verts (+ 13 de `mhd`), clippy 0.

### Clôture de G2 (2026-11-05)

**La mesure sur le petit PC est reportée, décision de l'utilisateur.** G1 présente
peu de risque (mêmes systèmes au banc, pas de surcoût sur la machine de dev) ; le
critère du §7.7 porte sur la V3 finale, et le vrai risque de coût arrive avec les
lots qui ajoutent des objets. **Deux passages prévus : après G3, après G7.**

**Le protocole, à lancer à la racine du projet** (six tours alternés, ~22 min) :

```powershell
cargo build --release
foreach ($t in 'v2','v3','v3b','v2','v3','v3b') {
    $env:DIAG = 'systemes'
    $env:SKYMAP = $t.Substring(0,2)
    if ($t -eq 'v3b') { $env:QUALITE = 'basse' } else { Remove-Item Env:QUALITE -ErrorAction SilentlyContinue }
    .\target\release\solar_game.exe | Select-String 'TOUR :|moyenne des graines' | Tee-Object -FilePath g23.txt -Append
}
```

**Contrepartie acceptée** : si la V3 dépasse la V2 après G3, mesurer aussi `ffd19fa`
pour séparer G1 de G3. D'ici là, l'absence de surcoût n'est établie que sur la
machine de dev, et le gain de la qualité basse reste sans mesure.

---

## G3 — l'échelle (ouvert le 2026-11-06)

**Fond.** La genèse produit enfin des systèmes à l'**échelle vraie**, et le
grossissement devient une affaire d'affichage (conception §7.1, §7.14). C'est le
lot le plus risqué après G1 : il touche les 29 types d'étoiles et ce que voit le
joueur.

**Inventaire préalable** :

- **L'éclairage d'une planète ne dépend pas de sa distance** : il vaut
  `couleur × luminosité`. Une luminosité vraie ne peut donc jamais aller au shader :
  la luminosité de rendu reste celle d'aujourd'hui.
- **Le système solaire tourne déjà à distances vraies**, corps ×5 : le moteur connaît
  ce régime. Les étoiles générées font, elles, ~9 fois leur taille vraie (G : 2 unités),
  et une Terre générée 300 fois la sienne (0,6 unité).
- **`√L` avec une luminosité vraie sort du jeu** : Kuiper à 68 000 unités pour une B, à
  675 000 pour une O, quand la caméra s'arrête à 30 000 (§7.14).

| Sous-lot | Contenu | État |
|---|---|---|
| **G3.1** | La table physique des 29 types, avec ses sources | ✅ `c49fe15` |
| **G3.2** | `planifier` sur les vraies valeurs ; le banc doit montrer des écarts voulus | ✅ |
| **G3.3** | `echelle.json` et `instancier` : la vérité dans le plan, grossissement et compression à l'affichage | |
| **G3.4** | Les contrôles : précision des positions, durée des trajets, cadrage, jugement à l'écran ; passage sur le petit PC | |

---

### G3.1 — les vraies valeurs des étoiles (2026-11-06, `c49fe15`)

**Fond.** Les calculs (température, étalement, Kepler) vont lire de vraies valeurs. Les
luminosités du jeu sont comprimées (une O vaut 4 Soleils au lieu de ~100 000) : elles
restent pour le rendu seulement.

**Forme.** `src/genese_v2/physique.rs` :

- `plages(nom)` : rayon et masse en unités solaires, pour chacun des 29 types ;
- `stefan_boltzmann(R, T)` et `T_SOLEIL` (5 772 K, valeur de l'UAI) ;
- `vraies(nom, q, T)` : les valeurs d'une étoile tirée, `q` donnant sa place dans ses
  plages, en interpolation géométrique.

**Le choix qui structure la table : la luminosité se déduit**, `L = R²(T/T☉)⁴`. Le jeu
tire déjà la température : une colonne de luminosité aurait été un second avis sur le
même fait. Seuls les objets sans photosphère en portent une (trou noir, pulsar, étoile à
neutrons, magnétar, quasi-étoile).

**Sources, et ce qu'elles valent** :

| Types | Source | Fiabilité |
|---|---|---|
| M, K, G, F, A, B, O | [table de Mamajek](https://www.pas.rochester.edu/~emamajek/EEM_dwarf_UBVIJHK_colors_Teff.txt) (Pecaut & Mamajek 2013, tenue à jour), bornes de chaque classe | **forte** ; l'auteur dit ses masses provisoires |
| Géante rouge | [Wikipédia](https://en.wikipedia.org/wiki/Red_giant) : 0,3–8 M☉, jusqu'à ~200 R☉ et ~3 000 L☉ | ordre de grandeur |
| Supergéante rouge | [HyperPhysics](https://hyperphysics.gsu.edu/hbase/Astro/redsup.html), V354 Cep, μ Cep | ordre de grandeur |
| Céphéide | [Wikipédia](https://en.wikipedia.org/wiki/Classical_Cepheid_variable) : 4–20 M☉, 1 000–50 000 L☉ | ordre de grandeur |
| Mira, carbonée | T Cep, S Cep, R Lep, [revue des Miras](https://arxiv.org/pdf/1207.4094) | étoiles nommées |
| Wolf-Rayet | [Wikipédia](https://en.wikipedia.org/wiki/Wolf%E2%80%93Rayet_star), tables WN et WC | **rayon effectif** (voir plus bas) |
| Sous-naine sdB | [Wikipédia](https://en.wikipedia.org/wiki/Subdwarf_B_star) : ~0,5 M☉, 0,15–0,25 R☉ | ordre de grandeur |
| Naines blanches | [Wikipédia](https://en.wikipedia.org/wiki/White_dwarf) : 0,5–0,7 M☉, 0,8–2 % R☉ | bonne |
| Naine brune | 13–80 masses de Jupiter, rayon ~Jupiter, [exemple à 1/8 000 L☉](https://universetoday.com/107141/when-is-a-star-not-a-star/amp) | ordre de grandeur |
| Étoile à neutrons | [9–12 km, 1–2 M☉](https://arxiv.org/pdf/1901.01673) | bonne pour R et M |
| Quasi-étoile | [Wikipédia](https://en.wikipedia.org/wiki/Quasi-star) : 38–189 UA, ≥ 1 000 M☉ | **hypothétique** ; garde sa taille de jeu (G3.2) |

**Valeurs non sourcées, posées à l'ordre de grandeur** (`// ~` dans le code) : rayon de
l'hypergéante et de la supergéante bleue (les pages ne donnent que leur luminosité), toute
l'étoile Be (sa page ne donne aucun chiffre), rayon et bornes de T Tauri et Herbig, masse
du trou noir, luminosités des objets compacts et de la quasi-étoile.

**Écarts et limites.**

- **Wolf-Rayet : rayon effectif.** Leur gabarit dit 38 000–45 000 K, moins chaud que
  le réel : le rayon publié (0,7–8,7 R☉) donnerait trop peu de lumière. On lui donne
  8–20 R☉, pour retrouver la luminosité publiée, qui est ce que lisent les calculs.
- **Le rayon et la température sont tirés séparément** : une O peut être à la fois petite
  et très chaude. La luminosité reste plausible, sa dispersion dans une classe est un
  peu large.
- **La quasi-étoile** faisait 38 à 189 UA dans cette table : G3.2 lui rend sa taille de jeu.

**Tests** :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `chaque_type_du_tirage_a_ses_valeurs` | les 29 types, plages valides | un type retiré |
| `la_loi_retrouve_la_sequence_principale_publiee` | la loi contre neuf lignes de Mamajek | loi en T³ ; T☉ fausse |
| `le_soleil_tient_dans_sa_classe` | 1 R☉, 1 M☉, 1 L☉ | classe G déplacée |
| `les_classes_de_la_sequence_principale_montent` | de M à O, sans chevauchement notable | classe K élargie |
| `les_luminosites_publiees_sont_atteignables` | chaque luminosité publiée, à ×1,5 près, dans ce que le type produit | rayon de céphéide réduit ; rayon mini de supergéante doublé |
| `la_place_dans_la_plage_est_geometrique` | interpolation ; une luminosité explicite ignore T | interpolation linéaire ; luminosité explicite ignorée |

⚠️ **Un test rouge, et la table n'avait pas tort.** Bételgeuse à 10 000 L☉ pour 370 R☉ ne
vérifie pas la loi à sa propre température : c'est un chiffre arrondi. Plutôt que d'ajuster
un rayon pour faire passer le test, celui-ci admet ×1,5, sous l'erreur qu'il vise (un rayon
faux d'un facteur 2 change L d'un facteur 4).

**Reste.** Module sans consommateur hors des tests (`allow(dead_code)`) : tombe en G3.2.

**Vérifié** : 1 023 tests verts (+ 13 de `mhd`), clippy 0.

---

### G3.2 — planifier sur les vraies valeurs (2026-11-06)

**Fond.** Le plan porte désormais la **vérité physique** : distances en UA, étoiles en
unités solaires, planètes et lunes en unités terrestres. Les températures, l'étalement,
les orbites se calculent dessus. L'écran garde sa luminosité de rendu et grossit le reste.
**C'est le premier lot où la V3 cesse d'imiter la V2.**

**Forme.**

- `plan.rs` : `EtoilePlan` gagne `q`, sa place dans son type. `rayon`, `luminosite`, `masse`
  sont les vraies valeurs, **figées dans le plan** : un système découvert ne change pas si
  la table bouge (§7.11).
- `planifier.rs` :
  - `etalement(L)` = `√L` entre **0,05 et 5** (§7.14) ; la première orbite passe au-delà de
    la **vraie** surface de l'étoile ;
  - `sphere_de_hill` : les lunes se placent entre Roche et **le tiers de la sphère de
    Hill**, avec la période de Kepler du système solaire (`omega_lune`) ;
  - en multiple : `sep_ecartee` (deux étoiles restent à deux fois la somme de leurs vrais
    rayons), `a_min_s` et `zone_s_possible` (une zone commence au-delà de la surface).
- `apparences.rs` : rayon et masse en unités terrestres, la masse déduite du rayon.
- `instancier.rs` : la conversion vers l'écran, en dur jusqu'à G3.3. `rayon_corps` (×5 comme
  le système solaire), `rayon_affiche` (compression en puissance 0,6 pour les étoiles, taille
  de rendu pour les objets compacts), `rendu` (luminosité et activité lues dans le gabarit).
- `skymap_v3.rs` : le cadrage par défaut descend à 30 unités au lieu de 350.

**Ce que l'échelle vraie a obligé à changer**, au-delà des nombres :

- **Plus de glace chaude** : la règle par distance relative aurait posé des mondes de
  glace à 900 K autour d'une O. Au-delà de 250 K, un monde de glace devient rocheux.
- **Le garde-fou anti-collision est retiré** : mis à l'échelle, il ne servait plus jamais
  (son test le disait : `serrees` restait à 0). Les rayons sont négligeables devant l'écart
  entre orbites. La propriété reste gardée par un test.
- **Le grand disque** : son bord interne suit le petit disque de l'étoile, donc la taille
  **affichée**. C'est `instancier` qui le calcule et qui décide s'il a la place ; le plan
  n'en porte que le bord externe.
- **La quasi-étoile garde sa taille et sa masse de jeu** (1 UA, 20 masses) : hypothétique, et
  son rayon théorique (38–189 UA) dépasserait à lui seul le plafond. **Proposition à valider
  par l'utilisateur.**

**Banc sur 5 000 graines** : les écarts avec la V1 sont maintenant **voulus** :

| Grandeur | V1 | V2 | Pourquoi |
|---|---:|---:|---|
| Planètes inhabitables | 2,2 % | 12,7 % | les étoiles chaudes brûlent leurs mondes |
| Planètes habitables | 18,5 % | 14,0 % | idem |
| Planètes dangereuses | 18,4 % | 22,8 % | étoiles faibles, mondes froids |
| Gazeuses sans lune | 34,5 % | 2,2 % | la vraie sphère de Hill est bien plus large que la règle des 14 % |
| Multiples sans planète | 0 % | 0,6 % | une géante près de sa compagne n'a pas de zone |
| Rayon moyen d'une planète (unités) | 0,898 | 0,029 | ×5 au lieu de ~×300 |
| Périastre de la 1re planète | 0,635 UA | 1,103 UA | étoiles brillantes plus étalées, sous le plafond |

**Vu à l'écran, sur des captures** (`SKYMAP=v3 SYSTEME=… CAPTURE=…`, G, O, naine rouge,
supergéante) : le rendu est propre, les icônes vont dans le bon sens (mondes tempérés autour
de la G, lave autour de la O et de la supergéante). **Défauts relevés** :

- ⚠️ **L'anneau de zone habitable et la ligne de zone de la fiche sont faux en V3** : ils
  lisent la luminosité de rendu (`Soleil::zone_viable`, `systeme/rendu.rs`,
  `ecran/fiche.rs`). À corriger en premier en G3.3.
- **Le cadrage par défaut vise les comètes** : leur apoastre occupe l'écran, le système
  intérieur est un point. Comme en V2, mais plus marqué. À juger en G3.4.
- **Toutes les étoiles d'un même type de graine se ressemblent à ce cadrage** : même graine,
  donc même tirage de planètes, à l'échelle près. C'est normal (voir G2.2).

**Tests** (les tests de G1.3 sont repris sur les vraies unités) :

| Test | Ce qu'il garde | Sabotage qui le fait rougir |
|---|---|---|
| `tout_systeme_tient_dans_le_champ_de_la_camera` | apoastres et disques sous `DIST_MAX`, multiples compris | plafond retiré |
| `le_plafond_laisse_les_etoiles_chaudes_sans_monde_tempere` | > 350 K autour d'une O ; étalement ~1 pour une G ; les bornes | plafond à 40 ; température sur une luminosité comprimée |
| `les_vraies_valeurs_ne_produisent_rien_d_absurde` | pas de glace chaude, de planète dans son étoile, de compagne dans une géante | chacun des trois |
| `les_lunes_restent_dans_le_domaine_de_leur_planete` | entre Roche et la sphère de Hill | lunes à 3 sphères |
| `la_sphere_de_hill_retrouve_celles_du_systeme_solaire` | Terre 0,01 UA, Jupiter 0,355 UA (valeurs publiées) | formule fausse |
| `les_planetes_voisines_ne_se_chevauchent_pas` | la propriété, sans le garde-fou | orbites collées |
| `un_corps_a_le_rayon_du_systeme_solaire` | une Terre vaut celle du preset ; la comète garde le sien | sans grossissement ; comète grossie |
| `la_taille_des_etoiles_a_l_ecran` | Soleil, supergéante ~1 UA, naine blanche visible, compacts | sans compression ; quasi-étoile compressée |
| `le_rendu_d_une_etoile_ne_lit_pas_ses_vraies_valeurs` | luminosité et activité de rendu inchangées | vraie luminosité au rendu ; activité sur le rayon affiché |
| `l_etoile_a_l_ecran_ne_recouvre_pas_ses_planetes` | rayon affiché < périastre | étoile affichée trop grosse |

**Reste.** G3.3 : la zone habitable vraie, puis `echelle.json` et la compression des
distances. G3.4 : les contrôles et le jugement à l'écran.

**Vérifié** : 1 031 tests verts (+ 13 de `mhd`), clippy 0, banc de 5 000 graines et captures
sans erreur.
