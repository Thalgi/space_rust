# Conception — L'interface de jeu (vue système)

> Ce document conçoit l'interface dessinée sur le schéma du **2026-08-04** :
> barre de ressources, nom du système, sélecteur de planètes rétractable,
> panneau de planète au clic. C'est la première interface **de jeu** du projet —
> tout ce qui existe aujourd'hui est de l'outillage de développement.
>
> Écran visé : la **Skymap** ([`src/ecran/skymap.rs`](../../src/ecran/skymap.rs)),
> pas la Starmap. Voir §1.1.
>
> **État au 2026-08-04 : les six étapes I.0 à I.5 sont faites**, et quatre des
> cinq bouche-trous soldés. Journal détaillé (décisions, red-checks, tests
> faibles trouvés) : [`suivi/interface.md`](../suivi/interface.md).

---

## 1. Lecture du schéma

### 1.1 De quel écran parle-t-on

Le schéma est titré « INTERFACE STARMAP », mais son contenu — « Système Sol »,
des orbites concentriques autour d'une étoile, une liste de planètes nommées —
est celui de la **vue système**, que le code appelle `Skymap`. La `Starmap` est
la vue **galactique** au-dessus (voisinage stellaire, une bille par étoile).

C'est donc la `Skymap` qui reçoit cette interface. La distinction n'est pas
byzantine : les deux écrans ont des états, des caméras et des unités
différents, et l'interface conçue ici parle de planètes, ce que la Starmap n'a
pas.

### 1.2 Les cinq zones

```
┌──────────────────────────────────────────────────────────────┐
│ ⓐ  $ 50K  🛢 5K  △ 1K  🥫 100  ⬡ 20                          │
│ ⓑ  Système Sol                                               │
│ ┌──┐                                              ┌────────┐ │
│ │ⓒ│                                              │   ⓔ    │ │
│ │○ │                    ⓓ                        │ ●●●●   │ │
│ │○ │             (étoile, orbites,               │ PLANÈTE│ │
│ │○ │              planètes, lunes)               │ Habit. │ │
│ │○ │                                              │        │ │
│ │« │                                              └────────┘ │
│ └──┘                                                         │
└──────────────────────────────────────────────────────────────┘
```

| | Zone | Nature | Visible |
|---|---|---|---|
| ⓐ | **Barre de ressources** | Bandeau horizontal, coin haut gauche | toujours |
| ⓑ | **Nom du système** | Une ligne sous la barre | toujours |
| ⓒ | **Sélecteur de planètes** | Colonne étroite, bord gauche, **rétractable** (`«`) | toujours, sauf repliée |
| ⓓ | **La vue système** | Ce qui existe déjà | toujours |
| ⓔ | **Panneau de planète** | Encart droit | **au clic sur une planète** |

Le coin haut droit du schéma (« RAW RSC / REFINED ONE », les icônes ▲ ⬡ ⬢ H₂)
n'est **pas** un élément d'interface : c'est la légende manuscrite qui explique
le vocabulaire d'icônes. Elle ne se dessine pas, elle se lit ici.

---

## 2. Ce que le code sait déjà faire, et ce qui manque

Cette section existe pour éviter la faute la plus coûteuse de ce chantier :
prendre pour un travail d'affichage ce qui est en réalité un manque de
**modèle**. Deux des cinq zones sont dans ce cas.

### 2.1 Acquis, réutilisable tel quel

| Besoin | Ce qui répond | Où |
|---|---|---|
| Cliquer une planète | `Camera::pick` → `Systeme::pick(origine, dir)` | `camera.rs`, `systeme/mod.rs:143` |
| Empêcher l'interface de faire pivoter la caméra | `cam.input_orbite(sur_ui)` | `skymap.rs:145` |
| Panneaux, cadres, lignes survolables | `minitel_panel`, `minitel_ligne` | `ui.rs` |
| Texte mesuré (troncature juste) | `police::texte`, `police::mesure` | `police.rs` |
| Colonne d'items cliquable, largeur bornée | `ecran::liste` | `ecran/liste.rs` |
| Habitabilité | `etoile::zone_habitable(luminosite) -> (f32, f32)` en UA | `etoile.rs:98` |
| Position, rayon, catégorie d'un astre | `CorpsBase`, `Astre::categorie()` | `astre.rs` |

Deux de ces lignes méritent d'être soulignées.

**`ecran::liste` est déjà le sélecteur ⓒ.** Il a été écrit pour la colonne de la
vue composants, mais rien en lui ne connaît les composants : il calcule un
rectangle de colonne borné à un dixième de l'écran, une hauteur de ligne
adaptative, l'item sous le curseur, et il sait dire si le curseur est sur la
colonne. C'est exactement le contrat de ⓒ. **À réutiliser, pas à réécrire** —
et s'il manque quelque chose (le bouton `«`), c'est à lui qu'on l'ajoute.

**« PLANÈTE HABITABLE » se déduit, ne se stocke pas.** `zone_habitable` rend les
bornes en UA à partir de la luminosité ; comparer le demi-grand axe de la
planète à ces bornes donne la réponse. Un booléen `habitable` posé sur la
planète serait une **seconde source** pour un fait que la géométrie décide déjà,
et c'est la faute qui a produit presque toutes les erreurs de ce projet
(`suivi/stations.md` §C.29, leçon 3).

### 2.2 Manquant — et ce n'est pas de l'interface

#### a) Les astres n'ont pas de nom

`Astre` expose `categorie()`, `corps()`, `position()` — jamais de nom.
`CorpsBase` porte `position`, `vitesse`, `masse`, `rayon`. Dans `genese/mod.rs`,
les noms (`push("Mercure", …)`) servent **uniquement** à choisir une apparence,
puis sont jetés.

Le sélecteur ⓒ affiche « Sol, Mercure, Vénus » : il lui faut un nom par astre.

**Décision (2026-08-04) : nom sur les presets seulement.** Les systèmes écrits à
la main (solaire, Tau Ceti, Alpha Centauri, Avatar…) portent leurs vrais noms ;
les systèmes procéduraux retombent sur la **numérotation orbitale** (I, II,
III…). Conséquence assumée : le sélecteur d'un système engendré affichera
« III » et non un nom inventé. C'est le bon compromis — un générateur de noms
est un chantier à lui seul, et un mauvais nom procédural se remarque bien plus
qu'un chiffre.

Forme : `nom: Option<&'static str>` sur `CorpsBase` (ou un champ parallèle dans
`Systeme` ; à trancher à l'implémentation, §5.1). `None` ⇒ numérotation.

#### b) Il n'existe aucune économie

Le dépôt ne connaît ni énergie, ni minerai, ni métal, ni métal rare, ni
hydrogène, ni nourriture, ni recherche. La barre ⓐ n'affiche pas un état
existant : **l'état n'existe pas**.

**Décision (2026-08-04) : affichage seul, valeurs figées.** On écrit la barre,
ses icônes et sa mise en page ; les quantités viennent d'une structure
`Tresorerie` remplie à la main. Production, consommation, coûts et recherche
restent à concevoir plus tard.

C'est délibérément le petit bout : la question ouverte de ⓐ n'est pas
« combien de minerai ai-je ? » mais « **est-ce lisible ?** » — quatorze compteurs
dans un bandeau, à toutes les largeurs d'écran, sans que ça déborde ni que ça
ressemble à un tableur. Cette question-là se juge à l'écran, et se juge sans
économie derrière. L'économie viendra remplir une structure dont la forme aura
déjà été validée par l'œil.

---

## 3. Les ressources

### 3.1 Le vocabulaire réel : quatorze sprites, pas sept

`assets/sprites/` contient **14 icônes**, toutes en **16 × 16 RGBA**. Le schéma
n'en montre qu'une poignée : la barre dessinée est un **sous-ensemble** du
vocabulaire, pas son inventaire.

| Fichier | Ce qu'on y voit | Famille | Sur le schéma |
|---|---|---|---|
| `raw_ore.png` | caillou gris sombre | brute | ✅ (△) |
| `raw_rare_ore.png` | caillou sombre marqué **R** | brute | — |
| `raw_food.png` | gerbe de blé dorée | brute | — |
| `hydrogen.png` | **H₂** sur deux ballons rouges | brute | ✅ (H₂) |
| `metal.png` | lingot gris | raffinée | ✅ (⬡) |
| `rare_metal.png` | lingot turquoise marqué **R**, étincelles | raffinée | ✅ (⬢) |
| `processed_food.png` | conserve, pomme verte visible | raffinée | ✅ (🥫) |
| `construction_material.png` | poutrelles et tuyau | raffinée | — |
| `superstructure.png` | bloc structurel gris et rouge | raffinée | — |
| `antimater.png` | atome violet cerclé d'une orbite | exotique | — |
| `energy.png` | éclair bleu | flux | — |
| `research.png` | loupe à lentille bleue, manche rose | abstraite | ✅ (🔍) |
| `population_number.png` | groupe de silhouettes turquoise | abstraite | — |
| `robot.png` | tête de robot à antennes | abstraite | — |

**Décision (2026-08-04) : il n'y a pas de monnaie.** Le schéma notait un
compteur `$`, seul de la barre à n'avoir aucun sprite. La réponse n'était pas
d'en dessiner un quinzième : **l'énergie tient le rôle de la monnaie** dans ce
jeu. C'est le genre de décision qui vaut bien plus qu'une icône — elle dit que
rien ne s'achète avec un nombre abstrait ; tout se paie en quelque chose qui se
produit, se stocke et se dépense pour de bon.

Conséquence directe : les quatorze sprites **sont** le vocabulaire complet, sans
reste. Aucun asset à produire, et pas de compteur sans icône.

La symétrie brut → raffiné est nette et porte du sens : `raw_ore` → `metal`,
`raw_rare_ore` → `rare_metal`, `raw_food` → `processed_food`. Trois chaînes de
raffinage sont donc déjà **inscrites dans les icônes**, avant même qu'une
économie existe. L'interface a intérêt à rendre ces couples lisibles — les
ranger côte à côte, ou les faire se suivre.

### 3.2 Ce que les sprites imposent au rendu

Trois contraintes techniques, toutes faciles à manquer et visibles
immédiatement à l'écran :

1. **Filtrage au plus proche voisin, obligatoire.** macroquad filtre en linéaire
   par défaut ; du pixel art 16 × 16 agrandi en linéaire devient une bouillie.
   Il faut `set_filter(FilterMode::Nearest)` sur chaque texture, une fois au
   chargement.
2. **Échelles entières seulement** — 16, 32, 48 px. Un facteur 1,5 fait tomber
   un pixel source sur une frontière et produit des lignes d'épaisseur inégale
   dans une image qui n'en a que de 1 px. Conséquence : la barre **ne peut pas**
   dimensionner ses icônes proportionnellement à la hauteur d'écran ; elle
   choisit un palier.
3. **Les sprites sont saturés, le reste de l'interface ne l'est pas.** Tout le
   projet est en Minitel — cyan sur fond sombre, un seul ton. Ces icônes sont
   colorées et contrastées. Elles vont **dominer** le bandeau. Ce n'est pas
   forcément un défaut (une barre de ressources doit se repérer d'un coup
   d'œil), mais c'est un parti pris esthétique qui engage tout le reste de
   l'interface de jeu, et il vaut mieux le prendre sciemment en le regardant à
   l'écran qu'en le découvrant.

Le chargement suit le modèle de `police.rs`, qui a déjà résolu exactement ce
problème pour la police Minitel : chargement **une seule fois** au démarrage,
stockage en `thread_local` (macroquad est mono-thread), et **repli silencieux**
si le fichier manque. Un sprite absent ne doit pas faire tomber le jeu.

### 3.3 Format des quantités

Les ordres de grandeur diffèrent de plusieurs décades — 500 K de crédits contre
20 unités de métal, sur le schéma même. L'abrégé (`50K`, `5M`) est donc
obligatoire, et il doit passer par **une seule fonction** : deux compteurs qui
formatent chacun de leur côté finiront par afficher la même quantité de deux
façons. C'est testable de bout en bout, et ce sera le premier test de I.4.

### 3.4 La barre : quatorze compteurs sur deux lignes

**Décision (2026-08-04) : les quatorze, sur deux lignes.** Le schéma n'en
montrait que cinq ; tout est affiché en permanence.

Deux lignes ne sont pas qu'un moyen de faire tenir quatorze compteurs — elles
donnent une **grille de sept colonnes**, et une grille permet d'aligner
verticalement ce qui va ensemble. La disposition proposée s'en sert pour poser
chaque produit **sous** sa matière première :

| | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|
| **haut** | minerai | minerai rare | nourriture brute | hydrogène | antimatière | énergie | recherche |
| **bas** | métal | métal rare | nourriture transf. | mat. de construction | superstructure | population | robots |

- **Colonnes 1 à 3** : les trois chaînes de raffinage, exactement alignées.
  Lire une colonne de haut en bas, c'est lire la chaîne.
- **Colonnes 4 et 5** : intrants au-dessus, choses bâties en dessous.
  L'alignement y est plus lâche et l'assume.
- **Colonnes 6 et 7** : les flux au-dessus (énergie — donc la monnaie —,
  recherche), la main-d'œuvre en dessous (population, robots).

Cette disposition est une **proposition à juger à l'écran**, pas un acquis. Ce
qui est acquis, c'est que la barre affiche une liste **ordonnée et groupée**
qu'on peut réarranger sans toucher au code de dessin.

### 1.3 Le partage haut / bas

Trois recouvrements sont apparus dès que l'interface de jeu s'est posée sur un
écran qui portait déjà l'outillage de développement (bascules d'orbites, de
zone habitable, de physique, menu de presets) :

1. les bascules étaient dessinées à `y = 34` — **au milieu** de la barre de
   ressources, qui va de 8 à 76 ;
2. le panneau d'astre, en haut à droite, recouvrait les boutons MENU / RETOUR ;
3. sur un écran étroit, MENU rattrapait la barre de ressources (à 640 px la
   barre va jusqu'à `x = 632`, le bouton commençait à 518).

**Règle posée : le haut appartient au jeu, le bas à l'atelier.**

| bande | contenu |
|---|---|
| Haut gauche | barre de ressources, nom du système |
| Toute la gauche | sélecteur d'astres |
| **Bas droite** | panneau de l'astre sélectionné |
| **Bande basse** (56 px) | bascules, MENU / RETOUR, ligne d'état (graine, FPS, raccourcis) |

La bande basse démarre **à droite de la colonne** et sa géométrie est une source
unique (`bandeau::strip_outils`) : le menu la **reçoit** au lieu de la
recalculer, comme la colonne reçoit déjà la hauteur du bandeau. Les bascules s'y
**resserrent** quand elle est trop courte — à 640 px il reste 560 px pour 587 px
de boutons, et sans ce facteur la dernière sortait de l'écran.

---

## 4. Comportements

### 4.1 Clic sur une planète

**Décision (2026-08-04) : centrer, comme aujourd'hui.** `Camera::pick` centre
déjà la caméra sur le corps cliqué ; le panneau ⓔ s'ouvre **en plus**. Aucun
changement au comportement existant.

Conséquence à assumer : sélectionner *est* cadrer. On ne peut donc pas comparer
deux planètes sans déplacer la vue. Si cela gêne à l'usage, la séparation des
deux gestes est un changement local (le panneau garde son index, la caméra ne
le suit plus) — mais on ne le fait pas par anticipation.

**Décision (2026-08-04) : le clic dans le vide referme.** Ni croix, ni Échap.
Échap est déjà « retour à l'accueil » sur cet écran, et lui confier un second
rôle selon qu'un panneau est ouvert créerait un état caché : la même touche
ferait deux choses sans que rien ne le dise. Le clic dans le vide, lui, est déjà
le geste qui désélectionne — `Systeme::pick` rend `None`, et le panneau suit.

⚠️ **Le panneau garde un index, pas un astre.** `G`, `R` ou le chargement d'un
preset reconstruisent le système : un index périmé désignerait alors un autre
corps, ou sortirait du tableau. Le panneau se referme donc dès que son index
dépasse le nombre d'astres.

### 4.2 L'interface mange les clics

Toute zone d'interface doit être exclue de la caméra **et** du picking, sinon
cliquer un item du sélecteur ferait aussi pivoter la vue derrière, ou
désélectionnerait la planète qu'on vient de choisir. `skymap.rs` a déjà la
mécanique (`sur_ui`) ; il faut y **agréger** les nouvelles zones :

```
sur_ui = menu.zone_cliquable | barre_ressources | selecteur | panneau_planete
```

⚠️ Une seule porte. Le jour où une zone oublie de s'y déclarer, le défaut est
invisible en test et pénible à l'écran — c'est exactement ce qui est arrivé à la
colonne de la vue composants, dessinée avant le `clear_background` qui l'effaçait.

### 4.3 Le sélecteur se replie

Le `«` du schéma replie la colonne. Replié, il doit rester un moyen de la
rouvrir (un `»` au même endroit). L'état de repli appartient à l'écran, pas à
`ecran::liste`, qui reste un calculateur sans mémoire.

---

## 5. Découpage en étapes

Chaque étape se juge **seule** à l'écran avant la suivante — la leçon 1 du
chantier ISV. L'ordre va du plus structurant au plus cosmétique, de sorte
qu'une pause à n'importe quel palier laisse un écran cohérent.

| # | Étape | Contenu | Testable ? |
|---|---|---|---|
| **I.0** | **Chargement des sprites** | Module d'atlas sur le modèle de `police.rs` : chargement unique, `FilterMode::Nearest`, repli si absent | ✅ le repli |
| **I.1** | **Noms d'astres** | `nom` sur les presets, numérotation orbitale en repli | ✅ la numérotation ; ❌ les noms de presets (voir ci-dessous) |
| **I.2** | **Sélecteur ⓒ** | Colonne réutilisant `ecran::liste`, pastilles, repli `«`/`»`, clic → sélection | ✅ le calcul ; ❌ le dessin |
| **I.3** | **Panneau ⓔ** | Encart droit au clic : nom, type, distance, rayon, habitabilité **déduite**, fermeture | ✅ l'habitabilité et le placement |
| **I.4** | **Barre ⓐ + nom du système ⓑ** | `Tresorerie` figée, format abrégé unique, sprites 16×16 au plus proche voisin, mise en page bornée | ✅ le format et la mise en page |

**Toutes faites au 2026-08-04.** Le décalage du haut (hauteur du bandeau) est
**transmis** à la colonne par `bandeau::hauteur_occupee`, jamais recopié : deux
constantes à tenir d'accord se seraient recouvertes en silence.
| **I.5** | **Agrégation `sur_ui`** | Une seule porte pour toutes les zones | ✅ entièrement |

I.5 est listé en dernier parce qu'il se **vérifie** en dernier, mais chaque
étape déclare sa zone au fur et à mesure — on ne laisse pas une zone non
déclarée entre deux étapes.

### 5.1 Questions ouvertes à trancher avant de coder

1. ~~Où vit le nom~~ — **tranché** : champ `nom: Option<&'static str>` sur
   `CorpsBase`. Une table parallèle à `Systeme::astres` serait deux vecteurs à
   tenir alignés, donc un désaccord qui attend, et `ajouter` devrait pousser
   dans les deux.
2. **Que devient le menu existant** (`src/menu/`) une fois cette interface en
   place ? Il est aujourd'hui l'unique interface de la Skymap et relève de
   l'outillage de développement (graine, presets, shaders). Il peut cohabiter,
   mais la question de sa place se posera.
3. ~~Les lunes dans le sélecteur~~ — **tranché** : listées, **en retrait** sous
   leur planète. Les exclure cacherait 16 des 26 corps du preset solaire ; les
   mettre à plat ferait perdre à quelle planète elles appartiennent.

### 5.1 bis Une limite de test à connaître

**Aucun test ne peut construire un système.** `genese` tire ses nombres
aléatoires par `macroquad::rand`, qui exige le contexte graphique
(`THREAD_ID.is_some()`) : hors boucle de rendu, tout appel à
`construire_systeme` ou `construire_preset_*` panique. Aucun test du dépôt n'en
bâtit, et ce n'est pas un oubli.

Conséquence pour I.1 : la **numérotation** se teste de bout en bout, contre un
corps d'essai posé à la main. Mais le fait que le preset solaire porte bien
« Mercure », « Titan », « Triton » ne se vérifie **qu'à l'écran**. C'est la même
limite que §6.6 sur le rendu — ce qui ne se teste pas doit au moins être dit.

Ce qui protège quand même : `ajouter_lune_preset` **exige** un nom dans sa
signature, donc une lune de preset ne peut pas être ajoutée sans. Les planètes,
elles, restent nommées par un appel séparé et pourraient être oubliées.

### 5.2 Ce qui n'est **pas** dans ce chantier

Production et consommation de ressources, arbre de recherche, coûts de
construction, colonisation, tout lien entre les stations/vaisseaux déjà
modélisés et cette interface. La barre affiche un état ; personne ne le fait
encore bouger.

---

## Sources

- Schéma manuscrit de l'utilisateur, 2026-08-04 (« INTERFACE STARMAP »).
- Décisions prises en conception le 2026-08-04 : affichage seul pour les
  ressources, noms sur les presets uniquement, clic = centrer comme aujourd'hui.
