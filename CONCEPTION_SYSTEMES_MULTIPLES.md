# Conception — Systèmes à plusieurs étoiles (binaires, trinaires…)

> Chantier ouvert le 2026-07-04. **Conception d'abord, code après.**
> Objectif : générer et rendre des systèmes à 2, 3, 4+ étoiles, physiquement plausibles.
>
> **Révision 2026-07-04 (décision structurante).** Ce chantier **fusionne** avec le modèle
> orbital : le **défaut** est **tout sur rails** (étoiles + planètes + lunes en orbites de Kepler
> analytiques dans un arbre hiérarchique). Motif : un jeu incrémental d'expansion ne doit
> **jamais** voir son système se détruire par émergence chaotique ; il faut des orbites théoriques
> stables, déterministes, et un temps que l'on peut accélérer/sauter (« on-rails / coniques
> raccordées » à la Kepler Space Program). Le multi-étoiles devient une simple conséquence
> (une étoile = un nœud de plus).
>
> **Le N-corps n'est pas supprimé** : il devient un **mode optionnel** (bac à sable émergent)
> activable par un **bouton** dans les options, qui n'affecte **que les planètes** (les étoiles
> et lunes restent toujours analytiques). Voir §4bis.

Légende bucket list : `[ ]` à faire · `[x]` fait · `[~]` partiel

---

## 1. Intention & vocabulaire

Aujourd'hui un système = **une** étoile à l'origine + des planètes. On veut des systèmes
**multiples**, comme la majorité des étoiles massives réelles.

- **Barycentre** : centre de masse commun. Les étoiles orbitent *lui*, pas l'une l'autre.
- **Binaire / trinaire / quaternaire** : 2 / 3 / 4 étoiles liées.
- **Hiérarchique** : les systèmes stables sont emboîtés — une paire serrée + un compagnon
  lointain, jamais 3 étoiles à distances comparables (instable). Ex. réel : **Alpha Centauri**
  = A+B serrées (~23 UA) + **Proxima** très loin (~13 000 UA).
- **Orbite d'une planète** :
  - **Type S (circumstellaire)** : autour d'**une seule** étoile de la paire (le compagnon est
    lointain). Stable si la planète est **assez proche** de son étoile.
  - **Type P (circumbinaire / « Tatooine »)** : autour de **tout le couple** (du barycentre).
    Stable si la planète est **assez loin** du couple. Ex. réel : **Kepler-16b**.

---

## 2. État actuel & ce qui bloque

Trois verrous dans le moteur (tous mono-étoile) :

1. **Étoiles figées** : `systeme/gravite.rs` n'intègre que les corps `Planete` ; les `Etoile`
   restent à l'origine (`if a.categorie() != Planete { continue }`). → Impossible d'avoir des
   étoiles qui orbitent un barycentre.
2. **Une seule lumière** : `CameraInfo` porte un unique `light_pos`/`light_color`
   (`systeme/rendu.rs` prend la **première** étoile). Le shader `planete.frag.glsl` a un seul
   `uniform vec3 lumiere` + `light_color`. → Pas d'éclairage multi-source.
3. **Orbites & zone habitable mono-foyer** : les orbites et la HZ sont tracées **relativement à
   l'unique étoile** (`light + pts[i]`). `genese::ajouter_planete` calcule la vitesse pour une
   orbite autour de `MASSE_ETOILE` **à l'origine**. → Aucun foyer alternatif (étoile hôte /
   barycentre).

**+ un 4e point, révélé par le gameplay** : les planètes sont aujourd'hui en **N-corps**
(`gravite.rs`, leapfrog), donc sujettes à dérive/chaos numérique sur de longues parties → un jeu
incrémental ne peut pas se permettre un système qui se détruit. La solution (§4) fait d'une pierre
deux coups : passer **tout** en analytique règle à la fois le mouvement des étoiles *et* la
stabilité des planètes.

Points d'appui existants (à réutiliser) :
- Le **pattern des lunes** : `parent()` + `orbiter_autour(centre, dt)` → orbite **analytique**
  autour d'un centre mobile, hors N-corps. C'est exactement le mécanisme qu'il faut pour faire
  bouger les étoiles secondaires sans casser la stabilité.
- `Soleil::new(position, rayon, couleur, luminosite)` accepte déjà une **position** ≠ origine et
  expose `lumiere()` (= couleur·lumi) et `zone_viable()` (depuis `etoile::zone_habitable`).

---

## 3. Modèle de données : arbre hiérarchique

Un système multiple = **un arbre de nœuds orbitaux** (chaque nœud = une orbite de Kepler entre
deux « sous-systèmes » autour de leur barycentre commun) :

```
Nœud = Orbite(élements) { gauche: Corps|Nœud, droite: Corps|Nœud }
Corps feuille = une étoile (Soleil)

Binaire :        (A · B)
Triple hiérarch.: ((A · B) · C)          ← Alpha Centauri
Quadruple :      ((A · B) · (C · D))     ← ex. réels (2+2)
```

Éléments d'orbite d'un nœud : `a` (demi-grand axe), `e` (excentricité), `incl`, `phase`,
masses des deux côtés → le barycentre et les deux orbites en découlent (rapport des distances
= inverse du rapport des masses).

- Représentation Rust proposée : `enum NoeudStellaire { Etoile(ProfilEtoile), Paire(Box<Orbite>) }`
  où `Orbite { a, e, incl, phase, gauche, droite }`. Sérialisable comme les presets.
- L'arbre est **déplié** à la construction en une liste de `Soleil` positionnés + leurs
  orbites analytiques (cf. §4).

---

## 4. Modèle orbital : tout « sur rails » (analytique)

**Décision structurante** : *aucun* corps n'est simulé en N-corps. Étoiles, planètes et lunes
sont **tous** placés sur des **orbites de Kepler fermées, évaluées analytiquement**
(position = f(t)) dans le même **arbre hiérarchique**. Modèle « on-rails / coniques raccordées »
de Kepler Space Program : les corps sont sur rails, jamais éjectés, jamais en collision numérique.

Pourquoi (et pourquoi ça remplace le N-corps des planètes) :
- **Robustesse** : une ellipse exacte ne se déstabilise jamais. Pas d'émergence chaotique qui
  détruit le système sur une longue partie (exigence du jeu incrémental).
- **Déterminisme** : même graine + même `t` → mêmes positions, toujours. Indispensable pour
  sauvegarde/chargement, événements planifiés, « reviens dans 2 h, ta colonie est où tu l'as
  laissée ».
- **Contrôle du temps** : `f(t)` permet de **scrubber** le temps librement — pause, ×1000, saut
  instantané à `t+100 ans` (mécaniques idle/incrémentales). Le N-corps doit intégrer chaque pas.
- **Plannabilité** : fenêtres de transfert, dates d'arrivée = réponses en forme close (gameplay
  d'expansion).
- **Unification & perf** : un seul modèle pour étoiles/planètes/lunes ; O(n) trig/frame au lieu
  de O(n²) accélérations × sous-pas. Le N-corps (`gravite.rs`) **n'est pas supprimé** mais
  **conservé comme mode optionnel** pour les planètes (§4bis).

Coût honnête :
- On perd les perturbations « réelles » (résonances, migration, assistance gravitationnelle).
  Non désiré ici. Pour un soupçon de vie, on **scripte** une lente précession (Ω/ω qui tournent
  doucement) — sans jamais rallumer le N-corps.
- Il faut un **solveur de Kepler** (`M = E − e·sin E` pour l'anomalie excentrique) : standard,
  ~3 itérations de Newton, trivial. Pour e≈0, quasi-circulaire direct.

---

## 4bis. Mode physique (option activable)

Le joueur peut basculer les **planètes** entre deux modes via un **bouton** dans les options ;
le libellé du bouton **affiche le mode courant** (pattern des toggles `ORBITES: ON/OFF` déjà en
place, `ui::minitel_ligne`) :

- **`ORBITES PHYS.: SUR RAILS`** (défaut) — planètes analytiques, stables, déterministes.
- **`ORBITES PHYS.: N-CORPS`** — planètes intégrées (moteur `gravite.rs`), dynamique émergente
  « bac à sable » (résonances, dérive, éjections possibles — assumé).

Règles :
- **Ne concerne QUE les planètes.** Étoiles et lunes restent **toujours** analytiques (stabilité
  des multiples). En N-corps, les planètes ressentent les étoiles (sur rails, éventuellement
  mobiles) + les autres planètes — ≈ le moteur actuel, avec des étoiles animées.
- **Hand-off** (l'analytique est la « vérité ») :
  - *Sur-rails → N-corps* : on **injecte** à chaque planète la **vitesse exacte** issue de son
    orbite de Kepler (état vis-viva à `t`), puis intégration libre.
  - *N-corps → sur-rails* : on **resnappe** sur la position/vitesse canonique (éléments avancés
    à `t`). Le bac à sable est un « et si ? » réinitialisable. *(Option future : conserver la
    dérive en re-dérivant les éléments osculateurs — pas au départ.)*
- **Déterminisme** : garanti seulement en mode sur rails. Le mode N-corps est explicitement
  hors de cette garantie (d'où l'étiquette claire au bouton).

`gravite.rs` est donc **conservé**, simplement **piloté par le mode** (défaut = off).

---

## 5. L'arbre unifié (étoiles, planètes, lunes)

Chaque corps est un **nœud enfant** d'un **foyer** (son centre d'orbite), avec ses éléments de
Kepler. Le même mécanisme sert à tout :

- **Étoiles** : enfants du barycentre du système (ou d'un sous-barycentre pour un couple serré).
- **Planète type S (circumstellaire)** : foyer = **une étoile hôte** (mobile). Stable si proche (§8).
- **Planète type P (circumbinaire)** : foyer = **barycentre stellaire**, masse centrale = **somme**.
- **Lune** : foyer = une planète (déjà le cas via `en_lune`).

À chaque frame, on parcourt l'arbre **racine → feuilles** : chaque nœud lit la position de son
foyer (calculée juste avant) puis évalue sa propre position analytique `f(t)`. Les lunes font
déjà exactement ça ; on généralise.

Conséquences moteur :
- `astre.rs` : généraliser le mécanisme lune (`parent()`/`orbiter_autour`) en un **foyer**
  d'orbite pour **tout** corps (index du centre : barycentre, étoile hôte, ou planète).
- `genese::ajouter_planete` : ne stocke plus `pos+vel` pour le N-corps, mais des **éléments
  d'orbite** (a, e, incl, phase, Ω, ω) + un foyer ; la position est évaluée par `update`.
- `systeme/gravite.rs` : **conservé**, mais **piloté par le mode physique** (§4bis) ; inactif sur
  les planètes en mode « sur rails » (défaut).
- Tracé d'orbite : chaque corps trace son ellipse autour de **son foyer** (résout le point 3 de §2).

---

## 6. Éclairage multi-source

Étendre l'éclairage à **N sources** (N petit : 2–4 étoiles).

- `CameraInfo` : remplacer `light_pos`/`light_color` par des **tableaux** `lights_pos[N]`,
  `lights_color[N]`, `n_lights`.
- Shader `planete.frag.glsl` : boucler sur les lumières, **sommer** les contributions diffuses
  (chaque étoile a sa direction `L` et sa couleur). Le terme ambiant reste unique.
- `systeme/rendu.rs` : collecter **toutes** les étoiles (pas la première) → remplir les tableaux.
- **MVP possible** : garder 1 lumière = l'étoile **dominante** (plus proche/plus lumineuse) pour
  débloquer vite, puis passer à la somme. Décision §12.
- Polish ultérieur : double **ombrage/pénombre** (deux ombres colorées), scintillement des
  couleurs mêlées au terminateur (l'esthétique « double coucher de soleil »).

---

## 7. Zone habitable multi-étoile

- **Type P (circumbinaire)** : HZ autour du **barycentre**, calculée sur la **luminosité totale**
  (somme). Anneau unique, comme aujourd'hui mais centré barycentre + lumi cumulée.
- **Type S** : HZ autour de **chaque** étoile (sa propre lumi), éventuellement resserrée par le
  flux du compagnon. Deux anneaux.
- Réutilise `etoile::zone_habitable(lumi)` ; l'entrée devient la lumi pertinente (hôte ou somme).

---

## 8. Stabilité (générer du plausible)

Critères de **Holman & Wiegert (1999)** pour ne pas générer d'orbites qui « n'existeraient pas » :

- **Type S** (planète autour d'une étoile) : stable si `a_planète ≲ a_crit`, avec
  `a_crit ≈ 0.1–0.4 × a_binaire` (dépend du rapport de masse et de l'excentricité).
- **Type P** (planète autour du couple) : stable si `a_planète ≳ a_crit`, avec
  `a_crit ≈ 2–4 × a_binaire`.
- **Hiérarchie** : ratio des demi-grands axes emboîtés **≳ 3–5** (sinon triple instable).

→ La génération place les planètes dans les **fenêtres stables** ; sinon on rejette/retire.
Pas besoin d'une vraie analyse de stabilité, juste ces garde-fous.

---

## 9. Types & exemples réels (catalogue de départ)

Lien direct avec la Starmap ([[chantier-starmap]]) — le voisinage réel *est* plein de multiples :

| Système            | Structure                         | Planètes    | Note                              |
|--------------------|-----------------------------------|-------------|-----------------------------------|
| **Alpha Centauri** | ((A G2V · B K1V) · Proxima M5.5)  | S-type      | Triple hiérarchique. A = preset **Avatar/Polyphemus**. |
| **Sirius**         | (A A1V · B naine blanche)         | —           | Étoile + vestige. Double contrasté. |
| **Système P-type** | (A · B) serré + planète lointaine | P-type      | « Tatooine » (cf. Kepler-16b).    |
| **Quadruple 2+2**  | ((A · B) · (C · D))               | S ou P      | Deux couples liés.                |

- Le preset **Avatar** existant place Polyphemus autour d'« Alpha Centauri A » **seule** →
  évolution naturelle : en faire un vrai binaire A+B (Pandora en S-type autour de A).
- Les presets se prêtent bien à des systèmes **scénarisés** (arbre écrit à la main) ; la
  génération procédurale viendra tirer des arbres aléatoires plausibles (§8).

---

## 10. Découpage en modules (proposé)

```
src/etoile/ (ou systeme/)  →  NoeudStellaire, Orbite, dépliage arbre -> Vec<Soleil positionnés>
src/systeme/stellaire.rs   →  propagation analytique des orbites d'étoiles (barycentres, t)
```
Modifs ciblées dans l'existant :
- `astre.rs` : `CameraInfo` → tableaux de lumières.
- `systeme/rendu.rs` : collecte N étoiles ; orbites/HZ par foyer.
- `systeme/mod.rs`/`gravite.rs` : appliquer l'orbite analytique aux `Soleil` secondaires.
- `genese/mod.rs` : `ajouter_planete` généralisé (centre/masse/foyer) + helper `ajouter_binaire`.
- `planete.frag.glsl` + `planete/materiau.rs` : uniforms de lumières multiples.
- `planete/mod.rs` : champ `foyer` (hôte/barycentre) pour le tracé d'orbite.

---

## 11. Plan d'implémentation

0. [~] **Socle orbital analytique** (le gros morceau) — *implémenté 2026-07-04, à compiler/tester* :
   `src/orbite.rs` (Kepler + solveur), `Planete.orbite_kep` + `maj_rail`/`amorcer_ncorps` (trait
   `Astre`), `Systeme` porte `temps: f64` + `mode: ModePhysique` (défaut `SurRails`),
   `ajouter_planete` construit une `Orbite`. `gravite.rs` conservé (branché en mode N-corps).
   Foyer = étoile à l'origine (mono-étoile) ; foyer mobile = étape 1.
   [~] **Bouton mode physique** (§4bis) : toggle `PHYS: SUR RAILS ⇄ N-CORPS` dans la barre
   d'options du menu skymap, câblé à `Systeme::regler_mode` (hand-off vis-viva au changement).
1. [~] **Étoiles mobiles** — *implémenté 2026-07-04, à compiler/tester* : `Orbite::avec_n` (n
   partagé), `Soleil.orbite_kep` + `avec_orbite` + `maj_rail`, `Systeme::update` place les étoiles
   analytiquement autour du barycentre (origine) **dans les deux modes**. Helper
   `genese::ajouter_binaire` (deux étoiles opposées, barycentre à l'origine) + preset
   `construire_preset_binaire` (« BINAIRE A+B » dans le menu skymap). Foyer des planètes = étoile
   primaire (S-type par défaut) ; foyer par planète (P-type) = étape suivante.
2. [~] **Modèle d'arbre** — *implémenté 2026-07-04, à compiler/tester* : `src/stellaire.rs`
   (`Noeud` = Étoile|Paire, `Feuille` {rayon,couleur,lumi,masse,variante}, `Variante`,
   `ArbreStellaire` déployé + `evaluer(t)` récursif racine→feuilles = composition des orbites).
   `genese::deployer_arbre`/`deployer_noeud` ; `Systeme.arbre` évalué dans `update` (remplace le
   chemin plat de l'étape 1, retiré de `Soleil`). Presets `binaire`/`trinaire ((A·B)·C)`/
   `quadruple 2+2 ((A·B)·(C·D))` + entrées menu. Masses par feuille → barycentres corrects ;
   variantes d'astres (jets/vent/pulsar/magnétar) supportées (WR utilisée dans le quadruple).
3. [~] **Éclairage multi-source** — *implémenté 2026-07-04, à compiler/tester* : `CameraInfo`
   gagne `lights_pos[4]`/`lights_color[4]` ; `systeme/rendu.rs` collecte jusqu'à 4 étoiles
   (indice 0 = primaire) ; `planete/materiau.rs` déclare/pose les uniforms tableau
   (`set_uniform_array`, comme `spots[8]`) ; `planete.frag.glsl` **somme** le diffus des N étoiles
   (primaire = diffus bosselé via `light_color` ; compagnons = diffus géométrique `lights_*[1..3]`).
   Single-étoile strictement inchangé (compagnons couleur nulle). Spéculaire/terminateur/villes
   restent sur la primaire (approx. assumée). Le preset **BINAIRE** reçoit une planète (type S)
   pour voir l'effet à deux soleils.
4. [~] **Planètes S-type / P-type** — *implémenté 2026-07-04, à compiler/tester* : enum
   `astre::Foyer { Barycentre, Etoile(idx) }` + `Planete.foyer` + `Astre::foyer()` ;
   `genese::ajouter_planete_autour(foyer, masse_centrale, …)` (`ajouter_planete` = wrapper
   `Barycentre`+`MASSE_ETOILE`). `Systeme::update`/`regler_mode` résolvent le foyer par planète
   (snapshot des positions) ; le tracé d'orbite est centré sur le foyer. Preset BINAIRE = démo
   type S (autour de A = `Etoile(0)`) **et** type P (circumbinaire, `Barycentre`, 18 UA).
5. [~] **Zone habitable** — *implémenté 2026-07-04, à compiler/tester* : circumstellaire (HZ propre
   de chaque étoile, verte, centrée sur elle) + circumbinaire (HZ de la lumi TOTALE, cyan, autour
   du barycentre) — cette dernière n'est tracée que si elle tombe hors des orbites stellaires
   (`iw > 2·r_max`), sinon trompeuse. `Astre::luminosite()` ajouté ; rendu dans `systeme/rendu.rs`.
6. [x] **Garde-fous de stabilité** (Holman-Wiegert) — *implémenté 2026-07-04* : `src/stabilite.rs`
   (`a_crit_p` circumbinaire min, `a_crit_s` circumstellaire max, `rapport_masse`). La génération
   `arbre_et_plan` s'appuie dessus : séparation externe = `a_crit_p(interne)×1.6-2.6` (hiérarchie
   stable), planètes P à `a_crit_p(externe)×1.25`. Distances de base recalées. `a_crit_s` prêt
   pour la génération S-type.
7. [~] **Presets scénarisés** : Alpha Centauri A+B (+ Pandora S-type) **et** Proxima Centauri
   — *implémenté 2026-07-04, à compiler/tester*. Deux presets menu séparés (Proxima à >10 000 UA,
   incadrable avec ACA/ACB) suivant le canon *Avatar* (Pandorapedia) : `construire_preset_alpha_centauri`
   = binaire ACA(G2V,Etoile 0)·ACB(K1V,Etoile 1), a≈23 UA e≈0.52 ; ACA a Odyssey/Ulysses (glace)
   + Oceanus + **Polyphemus**(+Pandora,Dante,Hadès) + Coeus(+Dionysos,Bacchus) ; ACB a Vulcain→
   Poséidon (miroir du Solaire). `construire_preset_proxima` = naine rouge M (L relevée 0.06 pour
   rendu) + géante gazeuse chaude + 2 rocheuses (dont une eyeball). Câblés menu skymap
   (ActionMenu/Source/labels). Reste : le P-type « Tatooine » dédié. **cargo check non fait**
   (pas de toolchain Rust dans l'env cowork) → vérif signatures manuelle.
7bis. [~] **Séparations réalistes + S/P coexistants** — *implémenté 2026-07-04* : séparations
   log-uniformes (~Raghavan 2010, pic ~40 UA, 1-60 UA) ; `arbre_et_plan` renvoie une **liste** de
   zones stables qui coexistent — S autour de A, S autour de B, P circumbinaire (P borné aux
   séparations où le rayon critique reste raisonnable). Binaire large → système type S « Avatar »
   autour de la primaire (Alpha Cen A stable jusqu'à ~2.8 UA). Vue par défaut : focus l'étoile hôte
   sur sa zone planétaire (`Systeme::vue` + `Camera::set_focus`) ; zoom élargi (clamp → 30000).
8. [~] **Génération procédurale** — *implémenté 2026-07-04, à compiler/tester* : `construire_systeme`
   tire la multiplicité depuis `seed % 100` (~55 % simple, 28 % binaire, 11 % trinaire, 6 % quadruple)
   → `construire_multiple` (arbre `arbre_et_plan` + planètes circumbinaires type P dans la zone
   stable). `feuille_alea` (masse ∝ L^0.286, variantes d'astres). Cadrage caméra auto via
   `Systeme::rayon_englobant()` sur G / ALEATOIRE / Charger. Mono-étoile inchangé (RNG intacte).
9. [ ] Polish : double ombre/pénombre, double coucher de soleil, marqueur « multiple » sur la Starmap.

---

## 12. Décisions (2026-07-04)

- **Modèle orbital** → **analytique « sur rails » par défaut** (étoiles + planètes + lunes en
  Kepler hiérarchique). Motif : stabilité garantie, déterminisme, temps accélérable — exigences
  du jeu incrémental. Les 2 chantiers (multi-étoiles + orbites) **fusionnent** en ce socle
  (étape 0, le gros morceau).
- **N-corps** → **mode optionnel** (bouton dans les options, libellé = mode courant), **planètes
  uniquement** ; étoiles/lunes toujours analytiques. `gravite.rs` conservé. Voir §4bis.
- **Éclairage** → **somme des N lumières** (coût par-pixel négligeable vs le reste du shader).
- **Étoiles** → **orbite analytique hiérarchique** (déterministe, stable, cheap ; réutilise le
  mécanisme des lunes). N-corps sur les étoiles = écarté (instable + non déterministe). Les
  planètes restent en N-corps autour des étoiles mobiles.
- **Portée** → **arbre hiérarchique complet dès le départ** (binaire/triple/quadruple = même type
  de nœud, pas de cas particuliers). Perf non-sujet : 1–3 nœuds, quelques trig/frame.
- **Nombre max de lumières shader** → **N = 4** ✅ *confirmé* (couvre jusqu'aux quadruples ;
  `uniform vec3 lights_pos[4]` + `n_lights` réel ; slots inutilisés gratuits, boucle bornée à
  `n_lights`).
- **Avatar** → upgrade en vrai binaire A+B **après le socle** (étapes 1–6). Le preset n'est qu'un
  consommateur du socle : trivial une fois celui-ci en place.
