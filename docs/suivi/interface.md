# Suivi — L'interface de jeu

> Journal du chantier conçu dans
> [`conception/interface.md`](../conception/interface.md) : la première
> interface **de jeu** du projet, sur la vue système (`Skymap`). Tout ce qui
> existait avant — menu graine, presets, hot-reload — est de l'outillage de
> développement.
>
> Point de reprise : [`STATE.md`](../../STATE.md).

---

## Tableau de bord

| Étape | Contenu | État |
|---|---|---|
| **I.0** | Chargement des sprites (`FilterMode::Nearest`, repli silencieux) | ✅ |
| **I.1** | Noms d'astres : presets nommés, numérotation orbitale en repli | ✅ |
| **I.2** | Sélecteur d'astres à gauche, rétractable | ✅ |
| **I.3** | Panneau d'astre au clic, habitabilité **déduite** | ✅ |
| **I.4** | Barre de ressources (14 sur deux lignes) + nom du système | ✅ |
| **I.5** | Agrégation `sur_ui` : une seule porte | ✅ |
| **I.6** | Sélecteur : icônes rendues (au lieu de pastilles), contour, débordement des anneaux | ✅ |

**Bouche-trous** : 4 des 5 soldés. Reste **D-INT-2**, l'économie — chantier de
conception à part entière (dette **D-INT-2**, tableau dans [`../../STATE.md`](../../STATE.md)).

**28 tests ajoutés, tous red-checkés** (avant I.6 — voir son propre décompte : 12
tests dans `ecran/icone.rs`).

---

## Ce que le schéma demandait, et ce qu'il a fallu inventer

Le schéma manuscrit (2026-08-04) montrait cinq zones : barre de ressources, nom
du système, sélecteur de planètes rétractable, la vue, et un panneau qui
n'apparaît qu'au clic. Deux d'entre elles n'étaient **pas** du travail
d'affichage :

1. **Les astres n'avaient pas de nom.** `Astre` exposait `categorie()`,
   `corps()`, `position()` — jamais de nom. Dans `genese/presets.rs`, les vrais
   noms vivaient en **commentaire** (`// Phobos`, `// Titan`) et les chaînes
   passées à `preset_tellurique(…)` nomment une **apparence**, pas un corps :
   « Lune » sert à notre Lune *et* à Ganymède.
2. **Il n'existait aucune économie.** Ni monnaie, ni minerai, ni recherche. La
   barre n'affichait pas un état existant : l'état n'existait pas.

Prendre l'un ou l'autre pour de la mise en page aurait été la faute coûteuse de
ce chantier ; d'où la section §2 de la conception, écrite avant toute ligne.

---

## I.0 — Les sprites (2026-08-04)

`assets/sprites/` contenait déjà **14 icônes 16 × 16 RGBA**, fournies par
l'utilisateur. Le schéma n'en montrait que cinq : la barre dessinée était un
**sous-ensemble** du vocabulaire, pas son inventaire.

La symétrie brut → raffiné y est inscrite avant même qu'une économie existe :
`raw_ore`→`metal`, `raw_rare_ore`→`rare_metal`, `raw_food`→`processed_food`.
Trois chaînes de raffinage, données par les dessins.

**Décision : il n'y a pas de monnaie.** Le schéma notait un compteur `$`, seul
de la barre sans sprite. La réponse n'a pas été d'en dessiner un quinzième :
**l'énergie tient le rôle de la monnaie**. Rien ne s'achète avec un nombre
abstrait ; tout se paie en quelque chose qui se produit et se stocke. Les
quatorze sprites sont donc le vocabulaire complet, sans reste.

**Trois contraintes que le pixel art impose**, toutes visibles immédiatement :

1. **Plus proche voisin obligatoire.** macroquad filtre en linéaire par défaut ;
   du 16 × 16 agrandi en linéaire devient une bouillie.
2. **Échelles entières seulement.** Un facteur 1,5 fait tomber un pixel source
   sur une frontière et donne des traits d'épaisseur inégale dans une image qui
   n'en a que de 1 px. `taille_ecran` descend donc au **palier** inférieur.
3. **Les sprites sont saturés, le reste de l'interface ne l'est pas.** Tout le
   projet est en Minitel cyan sur fond sombre. Ces icônes dominent le bandeau —
   ce n'est pas forcément un défaut pour une barre de ressources, mais c'est un
   parti pris qui engage la suite.

Le chargement suit `police.rs`, qui avait déjà résolu le problème : une fois au
démarrage, `thread_local`, **repli silencieux**. Un sprite manquant laisse un
emplacement vide, il ne fait pas tomber le jeu.

**Un test qui vaut d'être signalé** : `aucun_sprite_du_dossier_nest_orphelin`
interdit qu'un PNG traîne dans le dossier sans qu'une `Ressource` le réclame.
Sans lui, un sprite ajouté resterait invisible au jeu sans que rien ne le dise —
le défaut exact que la colonne d'items avait corrigé côté composants
(`suivi/stations.md` §F.7). Red-checké en **déposant un vrai fichier** dans le
dossier, seule façon de vérifier ce qu'il prétend.

---

## I.1 — Les noms d'astres (2026-08-04)

**Décision : noms sur les presets seulement.** Les systèmes procéduraux
retombent sur la numérotation orbitale. Un générateur de noms est un chantier à
lui seul, et un mauvais nom procédural se remarque bien plus qu'un chiffre.

Le nom vit sur **`CorpsBase`**, pas dans une table parallèle à `Systeme::astres` :
deux vecteurs à tenir alignés, c'est un désaccord qui attend, et `ajouter`
devrait pousser dans les deux.

**Le repli n'est pas un pis-aller, c'est la convention astronomique.** Une
planète sans nom est « III » ; la deuxième lune de la troisième planète est
« III-2 » ; si le parent est nommé, ses lunes suivent — « Jupiter-1 ». Le rang
se compte **par distance**, jamais par ordre d'ajout : c'est ce qui distingue une
désignation d'un index déguisé.

### Une limite de test, à connaître

**Aucun test ne peut construire un système.** `genese` tire ses aléas par
`macroquad::rand`, qui exige le contexte graphique (`THREAD_ID.is_some()`) :
hors boucle de rendu, tout `construire_systeme` panique. Aucun test du dépôt
n'en bâtit, et ce n'est pas un oubli.

La numérotation se teste donc de bout en bout contre un **corps d'essai** posé à
la main (`CorpsEssai`, un `Astre` minimal qui ne dessine rien et ne tire aucun
aléa). Mais que le preset solaire dise bien « Mercure » ne se vérifie **qu'à
l'écran**. Même limite que §6.6 sur le rendu : ce qui ne se teste pas doit au
moins être dit.

---

## I.2 — Le sélecteur (2026-08-04)

**`ecran::liste` était déjà le sélecteur.** Écrit pour la colonne de la vue
composants, rien en lui ne connaît les composants : rectangle borné à un dixième
de l'écran, hauteur de ligne adaptative, item sous le curseur, curseur sur la
colonne. Réutilisé, et étendu du repli plutôt que réécrit.

Trois décisions :

1. **Les ceintures ne se listent pas.** `Systeme::pick` les ignore déjà : une
   entrée de ceinture serait cliquable dans la colonne et jamais dans la vue.
2. **Les lunes se listent, en retrait.** Les exclure cacherait 16 des 26 corps du
   preset solaire ; les mettre à plat ferait perdre à quelle planète elles
   appartiennent.
3. **Repliée, la colonne rend la main à la caméra.** Seul le bouton reste actif —
   sinon replier ne libérerait pas la vue et n'aurait aucun intérêt. Le bouton ne
   bouge pas d'un état à l'autre : il faut pouvoir rouvrir sans le chercher.

---

## I.3 — Le panneau d'astre (2026-08-04)

**L'habitabilité se déduit, ne se stocke pas.** Le verdict vient de la distance
orbitale comparée à `etoile::zone_habitable(L)`, où `L` est la luminosité
**cumulée** de toutes les étoiles — la même zone combinée que le rendu trace
déjà pour les binaires. Une seule source : le panneau et les cercles verts ne
peuvent pas diverger. Un booléen `habitable` posé sur la planète aurait été une
seconde source pour un fait que la géométrie décide.

**Une lune hérite de la distance de sa planète.** Sa propre orbite fait quelques
centièmes d'UA ; mesurée depuis l'étoile, elle serait jugée brûlante. Europe est
habitable ou non exactement comme Jupiter l'est.

**Décision : le clic dans le vide referme.** Ni croix, ni Échap — Échap est déjà
« retour à l'accueil », et lui donner un second rôle selon qu'un panneau est
ouvert serait un état caché. Le clic dans le vide est déjà le geste qui
désélectionne.

⚠️ **Le panneau garde un index, pas un astre.** `G`, `R` ou le chargement d'un
preset reconstruisent le système : un index périmé désignerait un autre corps,
ou sortirait du tableau. Le panneau se referme dès que son index dépasse le
nombre d'astres.

### Un test qui avait tort, et pas le code

`les_etoiles_multiples_additionnent_leur_lumiere` a d'abord été écrit avec 0,3
par étoile. Il a rougi — mais à `L = 0,6` le bord externe tombe à **0,89 UA**,
donc 1 UA reste trop froid même à deux : le scénario n'encadrait jamais la borne
qu'il prétendait tester. Repris à 0,5 chacune (bord externe à 0,81 UA seule,
zone couvrant 1 UA à deux), et le test **vérifie sa propre prémisse** avant de
jouer — sans quoi un changement de formule le rendrait vert et vide.

---

## I.4 / I.5 — La barre, et la porte unique (2026-08-04)

**Quatorze compteurs sur deux lignes**, et les deux lignes ne sont pas qu'un
moyen de les faire tenir : elles donnent une **grille de sept colonnes**, où
chaque produit se pose sous sa matière première. Lire une colonne de haut en
bas, c'est lire une chaîne de raffinage. Un test l'impose, si bien que
réordonner la liste ne peut pas casser l'alignement en silence.

**Un seul `abreger`** pour les quatorze : les ordres de grandeur vont de l'unité
au million, et deux compteurs qui formateraient chacun de leur côté finiraient
par écrire la même quantité de deux façons.

**Le décalage du haut est transmis, jamais recopié.** La barre et la colonne
occupent toutes deux le coin haut gauche ; la colonne demande à la barre la place
qu'elle prend (`bandeau::hauteur_occupee`). Deux constantes à tenir d'accord se
seraient recouvertes sans que rien ne le dise. La ligne d'outillage (graine, FPS,
raccourcis) est passée en bas pour la même raison.

**Une seule porte pour la souris** : `sur_ui` agrège menu, bandeau, colonne et
panneau ; caméra et picking la consultent tous les deux.

---

## Les bouche-trous, et comment ils ont été soldés

Consigne de l'utilisateur : poser un bouche-trou pour ce qui n'est pas fait, et
**inscrire ce qu'il reste à faire**. Un bouche-trou non listé est un mensonge à
l'écran — il a l'air fini.

### D-INT-5 — le nom d'une planète de preset (soldée)

`ajouter_planete` et `ajouter_planete_autour` exigent désormais un
`Option<&'static str>`. Chaque appelant doit dire `Some("Terre")` ou `None` :
l'oubli ne compile plus. C'est le dispositif déjà en place pour
`ajouter_lune_preset`, où le nom entre avec le corps parce qu'une lune peut être
**refusée** (limite de Roche, sphère de Hill trop serrée).

**Rendre le paramètre obligatoire a trouvé onze corps** dont le nom était déjà
écrit quelque part mais jamais dans le programme : le preset Alpha Centauri
déclare `oceanus`, `aphrodite`, `gaea`, `ares`, `zeus`, `cronus`, `poseidon`,
`coeus` ; le preset Avatar a `poly` sous un commentaire « Polyphemus : géante
gazeuse bleu-vert dans la zone habitable » et une apparence littéralement nommée
`"Polyphemus (Avatar)"` ; Pandora était construite en ligne, sans nom. Tous
recopiés de là où l'auteur les avait mis — aucun inventé. Quinze planètes
réellement anonymes gardent `None`.

⚠️ Faute de pouvoir bâtir un système en test, deux tests **lisent la source** de
`presets.rs` : l'un interdit le retour du `sys.nommer` après coup, l'autre
vérifie que les presets nomment encore. C'est un procédé qu'on ne se permet
qu'ici, et uniquement pour **interdire un motif**, jamais pour vérifier un
comportement.

### D-INT-1 — la pastille du sélecteur (soldée)

Une couleur par **catégorie** faisait de toutes les planètes la même pastille
bleue. Elle vient maintenant du corps : `Astre::teinte()`, tirée de l'apparence
réelle — terres et océan mélangés par la part d'eau pour une tellurique, bandes
moyennées pour une gazeuse. Mars lit rouille, un monde-océan lit bleu, et
retoucher une apparence se voit aussitôt dans la colonne.

Ces couleurs sont des **albédos de surface**, faits pour être éclairés : posés
tels quels sur fond nuit, les plus mats donneraient un disque presque noir. Elles
sont relevées vers un plancher **en conservant le rapport des canaux**, si bien
qu'une planète rousse reste rousse au lieu de virer au gris.

La pastille reste un disque, et c'est voulu : à 6–14 px de diamètre, un rendu de
planète serait de la bouillie.

### D-INT-3 — la vignette du panneau (soldée)

Là, la place le permet : l'astre est **réellement rendu**
(`ecran/vignette.rs`), dans une cible 192 × 192.

**Une cible de rendu, et non un viewport.** La galerie découpe des viewports
dans l'écran, et c'est le bon outil là-bas puisqu'elle ne rend qu'eux. Ici la
scène du système est **déjà dessinée, profondeur comprise** : un viewport
poserait la vignette dans ce tampon-là, et l'astre se ferait découper par ce qui
traîne devant lui — un anneau, une lune, parfois rien selon l'angle. Une cible
séparée a son propre tampon de profondeur, effacé à chaque rendu.

**Le corps n'est pas déplacé** : on approche la caméra. Sa position sert à son
propre dessin (terminateur, éclairage multi-étoiles, anneaux orientés) ; le poser
à l'origine le montrerait éclairé autrement que dans la vue. La caméra se place
du **côté éclairé** — de face contre la lumière on ne verrait qu'un croissant,
à contre-jour un disque noir — et recule davantage pour un corps à anneau.

### Un test faible, trouvé par red-check

`la_pastille_retombe_sur_la_categorie` est **resté vert** quand on a saboté la
lecture de la teinte : tous les corps d'essai étaient sans apparence, donc les
deux chemins rendaient la même couleur. Il ne visitait qu'une branche. Le corps
d'essai porte maintenant une teinte, et le test est scindé — l'un prouve qu'une
planète rouge et une bleue se distinguent **et** diffèrent de la couleur de
catégorie, l'autre garde le repli.

---

## Trois recouvrements avec l'outillage (2026-08-04)

Signalés à l'écran par l'utilisateur, une fois la vue système ouverte sur le
solaire. Aucun n'était visible en test : la mise en page du jeu et celle de
l'outillage avaient été écrites séparément, chacune en dur.

1. **Les bascules ORB / ZONE / PHYS étaient dans la barre de ressources** —
   dessinées à `y = 34` depuis toujours, c'est-à-dire au milieu d'un bandeau
   qui n'existait pas quand elles ont été posées.
2. **Le panneau d'astre recouvrait MENU / RETOUR**, tous deux en haut à droite.
3. **MENU rattrapait la barre** sur un écran étroit.

**Correction : le haut au jeu, le bas à l'atelier** (`conception/interface.md`
§1.3). Le panneau d'astre descend **en bas à droite** — le seul coin que rien
d'autre ne réclame. Tout l'outillage se regroupe dans une bande de 56 px en bas,
à droite de la colonne, dont la géométrie est **transmise** au menu par
`bandeau::strip_outils` plutôt que recalculée de son côté.

Les bascules s'y **resserrent** quand la place manque : à 640 px de large il
reste 560 px pour 587 px de boutons, et la dernière sortait de l'écran.

**Cinq tests de mise en page**, tous red-checkés — ils vérifient qu'aucune des
zones n'en recouvre une autre, à quatre tailles d'écran. C'est le genre de
défaut qui ne se voyait qu'à l'œil ; il est désormais gardé.

## Passe de lisibilité (2026-08-04, retours d'écran)

Quatre demandes après la première mise à l'écran.

### Le sélecteur devient un arbre, et perd ses lunes

Le preset solaire compte **16 lunes pour 9 planètes** : listées, elles noyaient
la structure du système, qui est justement ce que cette colonne sert à lire.
Elles disparaissent, ainsi que les ceintures (déjà exclues).

Surtout, la liste était **plate** : dans un système multiple, rien ne disait
quelle planète orbite quelle étoile. Elle est maintenant un arbre — étoiles à la
racine, corps dessous, avec un trait `├─` / `└─` à gauche.

**La hiérarchie vient du foyer, pas de la distance** : une planète de type S se
range sous son étoile hôte (`Foyer::Etoile`), une circumbinaire sous une racine
« BARYCENTRE » qui n'est pas un astre et ne se sélectionne pas. À **une seule
étoile**, tout se range sous elle : un barycentre n'apprendrait rien.

Les **engins restent listés** (l'ISS sous la Terre) : ce sont les objets du
joueur, pas du décor. À dire si ce n'est pas le bon arbitrage.

### La barre collée en haut, icônes doublées

`HAUT = 0` : c'est un bandeau d'état, pas une fenêtre — un jour d'écart le
faisait flotter.

Les icônes passent de 16 à **32 px**. Le réglage n'est pas où on l'attend :
`sprites::taille_ecran` ne rend que des **multiples entiers de 16**, donc c'est
la *hauteur de ligne* qui commande le palier. À 22 px de ligne les sprites
tombaient à ×1 ; il a fallu passer la ligne à 38 pour atteindre ×2, et élargir
les cases (88 px de plancher) pour loger l'icône **plus** le nombre. Un test
pinne le palier : sans lui, retoucher `LIGNE` ferait retomber les sprites à 16
sans que la barre ait l'air d'avoir changé.

### Deux tests qui mesuraient autre chose que leur nom

1. `pas_de_barycentre_dans_un_systeme_a_une_etoile` vérifiait « aucune entrée non
   sélectionnable » et « aucun libellé BARYCENTRE ». Les deux restaient vraies
   quand les planètes cessaient de trouver leur étoile : elles devenaient des
   racines *sélectionnables*, sans barycentre pour autant. **Compter les racines**
   capture la propriété — il doit y en avoir exactement une, et c'est l'étoile.
2. Un sabotage de la garde `if multiple` ne pouvait **pas** rougir : la liste des
   circumbinaires ne peut être non vide que dans un système multiple, donc la
   garde était une **branche morte** qu'un lecteur croit vivante. Retirée, et
   l'invariant est désormais tenu par un test plutôt que par un raisonnement.

## Ce qui reste

- **D-INT-2, l'économie** : production, consommation, coûts, recherche. Les
  quatorze quantités sont figées à la main. La barre a été écrite pour pouvoir
  l'attendre — elle affiche une `Tresorerie`, d'où qu'elle vienne.
- **Le menu de développement** (`src/menu/`) cohabite avec la nouvelle interface.
  La question de sa place se posera.
- **Le parti pris esthétique** : sprites saturés contre Minitel monochrome. À
  juger à l'usage.

---

# Partie B — Calques, lisibilité, police et paramètres

## Récapitulatif I.0–I.5, et ce qui restait ouvert

Conception écrite le 2026-08-04 :
**[`docs/conception/interface.md`](../conception/interface.md)**. À lire avant
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


> Extrait de [`STATE.md`](../../STATE.md) le 2026-08-10.

## Interface — Bascules de calques en « doubles carrés »

Quatre bascules à deux étages, un mot par case, l'état porté par la **couleur du
fond** au lieu d'un « : ON » qu'il fallait lire :

| | RING / PLANET | RING / STAR | RING / MOONS | ZONE / HABITABLE |
|---|---|---|---|---|

**Les orbites de lunes n'existaient pas.** Les lunes sont créées avec un
`orbite` vide, donc la boucle des trajectoires les sautait — et les dessiner
avec celle des planètes les aurait centrées sur le **barycentre du système**
plutôt que sur leur planète, qui bouge. D'où une trace à part, `orbite_lune`, en
coordonnées relatives au parent, **précalculée** dans `en_lune` (elle ne dépend
que des éléments orbitaux) et construite à partir du **même** couple de vecteurs
`(a1, q)` que `orbiter_autour` : deux formules dériveraient en silence.

Désactivées par défaut — elles sont nombreuses et encombrent vite.

**Couleurs, et pourquoi elles ne sont pas libres.** Rouge contre vert est le
pire couple pour un deutéranope ou un protanope (~8 % des hommes) : les deux
teintes se confondent. Ce qui les sépare quand la teinte ne le fait pas, c'est la
**luminance** — un rouge sombre et un vert néon tiennent à plus de 4,5 de rapport
de contraste, lisible sans percevoir aucune des deux couleurs. Le vert est tiré
vers le jaune, où un protanope le voit encore clair.

Deux tests le tiennent (`les_deux_etats_se_distinguent_sans_la_teinte`,
`le_texte_contraste_avec_son_fond`) : une retouche esthétique qui rapprocherait
les luminances casserait l'accessibilité **sans que rien ne se voie** pour un œil
trichromate. C'est exactement le genre de régression qu'un test doit attraper.

**Deux autres choses au passage :**

- `draw_orbites` prenait trois booléens de suite, et il en fallait un quatrième.
  Quatre `bool` positionnels du même type s'intervertissent sans un mot du
  compilateur — le défaut qu'on venait de corriger sur les horloges. Ils sont
  devenus `Calques { planetes, etoiles, lunes, zone }`, nommés à l'appel.
- La taille de police des libellés descend jusqu'à ce que le **plus large** des
  deux mots tienne, mesuré avec la vraie police. La règle de mesure est passée en
  argument pour que le calcul se teste **sans fenêtre** (`police::mesure` exige un
  contexte graphique).

PHYS garde sa bascule à une ligne : ce n'est pas un calque mais un mode de
simulation, et il dit donc son état en toutes lettres.

## Interface — Trajectoires planétaires en points sarcelle

Le trait continu est devenu un **semis de points** : un court segment centré sur
l'échantillon, et non posé à son départ — sinon chaque marque se décale vers
l'avant et l'espacement se lit de travers.

**Un pas fixe ne pouvait pas marcher, et la première version l'a montré.** Une
même orbite couvre quelques pixels vue de loin et plusieurs écrans vue de près :
à pas constant on obtient, selon le bout, un trait plein ou deux marques perdues
sur l'écran. C'était le défaut signalé — « pas assez de points pour visualiser le
cercle de près ».

Deux corrections, et il fallait les deux :

1. **La réserve d'échantillons** (`Orbite::N_TRACE`) passe de 96 à **512**. Elle
   plafonne la densité : le tracé peut en sauter, jamais en inventer. Elle ne
   sert que de près, et coûte ~6 Ko par orbite. Les lunes suivent (72 → 256).
2. **Le pas s'adapte** : `dessiner_points` mesure l'espacement **à l'écran** et
   saute ce qu'il faut pour viser 22 px entre deux points. La distance retenue
   est celle de l'arc le **plus proche**, pas celle du foyer — vue de l'intérieur
   d'une grande orbite, l'arc proche est à des ordres de grandeur du foyer, et
   c'est lui qu'on regarde.

`draw_line_3d` rend un pixel de large quelle que soit la distance : un segment
très court se lit donc comme un point à tous les zooms utiles, et ne s'allonge en
tiret que de très près.

Les **lunes** ont suivi (mêmes points, violet pâle) : deux traces de même nature
dessinées différemment auraient été une incohérence gratuite.

**Les quatre teintes sont désormais nommées et testées ensemble.** Ce qui compte
n'est pas chacune prise à part mais qu'elles se distinguent **entre elles** :
quatre réseaux superposés sur le même fond noir, dont deux confondus, et on ne
sait plus ce qu'on regarde.

| calque | teinte |
|---|---|
| trajectoires planétaires | sarcelle |
| orbites stellaires | paille (tirets) |
| orbites lunaires | violet pâle |
| zone habitable | vert |

`les_calques_ne_se_confondent_pas` mesure les écarts deux à deux **en Lab**, où
la distance approche la différence perçue — pas en RGB, où deux couleurs
numériquement éloignées peuvent se ressembler. Et
`les_calques_restent_des_traits_de_fond` borne l'alpha : un calque est un fond de
carte, pas le sujet.

Le pas adaptatif a ses tests aussi : espacement obtenu proche de la cible sur
toute la plage de zoom, pas **jamais nul** (la boucle de tracé n'avancerait plus)
et **jamais au-delà du quart** des échantillons (il ne resterait plus un cercle).
⚠️ Le premier jeu de red-checks a laissé passer le retour à 96 échantillons —
rien ne gardait ce nombre. D'où `N_TRACE`, nommée et bornée par une assertion de
compilation, avec le raisonnement : c'est un plancher, pas une copie de la
valeur.

## Interface — Désigner un corps efface les traces qui ne le concernent pas

Sur un système garni, une douzaine de cercles superposés empêchent de voir lequel
appartient au corps qu'on vient de désigner. Un clic met donc à **10 %** tout ce
qui ne lui est pas lié.

**« Lié » veut dire : lui-même, ce autour de quoi il tourne, ou ce qui tourne
autour de lui.**

- désigner une **étoile** garde **toutes les trajectoires de ses planètes** —
  sans quoi on ne voit plus le système du tout ;
- désigner une planète garde sa trace **et** les orbites de ses lunes ;
- désigner une lune garde la sienne **et** celle de sa planète ;
- la planète sœur, elle, s'efface : c'est tout l'objet du filtre.

⚠️ **Deux erreurs successives sur cette règle, et la seconde n'a été trouvée
qu'à l'écran.**

1. La première version ne regardait que `Astre::parent` — renseigné pour les
   seules **lunes**. D'où `Systeme::attache`, qui réunit `parent` et `foyer` sous
   une idée : « ce corps tourne autour de celui-là ».
2. **Insuffisant, et désigner l'étoile effaçait toujours tout.** Le lien fin
   passe par le foyer, or `genese::ajouter_planete` — que le préset solaire
   utilise pour **toutes** ses planètes — les pose sur `Foyer::Barycentre`.
   L'étoile étant à l'origine, barycentre et étoile se confondent à l'œil, mais
   `attache` n'y voit aucun astre.

D'où une **seconde règle, assumée comme telle** : une étoile désignée met en
avant tout son système planétaire, indépendamment du foyer. Les lunes restent en
retrait — un cran plus bas, et nombreuses.

Effacé et non supprimé : le reste du système doit rester lisible en filigrane,
sinon désigner une planète donne l'impression que les autres ont disparu. Et
l'atténuation ne touche **que l'alpha** — une trace effacée qui changerait de
teinte se lirait comme un autre calque.

La règle est une question de **parenté**, pas de rendu : `liee_au_focus` reçoit
la fonction parent en argument et se teste sans écran, sur un arbre d'essai
(étoile, deux planètes, deux lunes).

⚠️ **Une approximation assumée** : les orbites stellaires viennent de l'arbre
sous forme de polylignes **sans index d'astre**, impossible à lier une à une. On
les garde donc entières quand c'est une étoile qu'on regarde, et on les efface
sinon.

⚠️ **Et le test de la première version est resté vert pendant que la
fonctionnalité était cassée.** Il modélisait `parent` comme rendant déjà
l'étoile d'une planète — ce que le vrai système ne fait pas. Il vérifiait donc
mon hypothèse, pas le système. Le double d'essai n'avait même pas de `foyer` :
la distinction ne pouvait pas être exercée.

Corrigé en donnant un `foyer` au double et en testant `attache` sur de vrais
astres. Le sabotage qui compte — revenir à `parent` seul — rougit maintenant.

Red-checks : filtre supprimé, enfants délaissés, parent délaissé, atténuation
nulle, imperceptible, ou qui déteint sur la teinte ; puis `attache` réduite à
`parent`, `attache` ignorant `parent`, et foyer barycentrique pris pour un astre.

**Taille des points** : elle se règle désormais en **pixels écran**
(`LONGUEUR_POINT_PX = 3`) et non plus en fraction de l'espacement. Les deux
réglages deviennent indépendants — espacer les points ne les allonge plus — et
« des petits points » reste vrai à tous les zooms. Les points de l'arc lointain
sortent plus courts : c'est la perspective, et c'est juste.

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

## ⚠️ La police ne couvre que 157 caractères

`assets/minitel.ttf` fait 12 Ko. Relevé sur sa table `cmap` : il porte les
accents **minuscules** (à â ä ç è é ê ë î ï ô ö ù û ü) — l'essentiel du look
Minitel — mais **pas leurs majuscules**, ni la ponctuation typographique
(`…` `—` `«` `»` `·` `×`). Un caractère sans glyphe ne se dessine pas : l'écran
montre un blanc, ce que l'œil lit comme un texte corrompu.

Deux cas se produisaient pour de bon, tous deux dans la **vue système** :

1. `skymap.rs` titre par `to_uppercase()`. « Système généré » devient
   « SYSTÈME GÉNÉRÉ » — `È` et `É` disparaissent.
2. `liste::tronquer` ajoute une **ellipse** `…` à tout libellé trop long, donc à
   toutes les entrées coupées du sélecteur d'astres.

Corrigé par `police::assainir`, appliquée au **point de passage unique** de tout
le texte du jeu. La table des remplacements est relevée sur la police, pas
devinée. Les accents minuscules sont **laissés intacts** : la police les a, et
les aplatir abîmerait le rendu là où il est juste.

⚠️ **Ce n'était pas une régression de la revue P1** — le défaut est aussi vieux
que la police. Vérifié : les seules retouches de la passe sur `skymap.rs` et
`selecteur.rs` étaient des retraits de `mut`.

⚠️ **`texte` et `mesure` doivent traiter la chaîne à l'identique.** Sinon
`tronquer` mesure une ellipse d'un caractère et en dessine trois : il déborde de
la colonne qu'il vient d'ajuster. Le désaccord **n'est pas testable** — `mesure`
exige un contexte GL — il est donc rendu difficile à écrire : les deux fonctions
ne sont plus que des enveloppes de `preparer`, qui est seule à décider, et qui
est testée. C'est le sabotage qui l'a imposé : « `mesure` n'assainit plus »
passait au vert.

## ⚠️ Carrés noirs à la place du texte (atlas de police)

**Symptôme** : en ouvrant la fiche d'un astre dans la vue système, *tous* les
caractères déjà dessinés dans la frame sortent en rectangles noirs pleins.

**Cause, dans macroquad** (`src/text/atlas.rs`, `Atlas::texture`) : les glyphes
vivent dans un atlas qui grandit à la demande, et quand il grandit macroquad
**détruit l'ancienne texture** pour en créer une plus grande :

```rust
ctx.delete_texture(self.texture);
self.texture = ctx.new_texture_from_rgba8(...);
```

Or macroquad ne dessine pas tout de suite : il **empile**. Tout le texte déjà
empilé dans la frame pointe alors sur une texture détruite — et sort en noir.

Le déclencheur était la fiche : c'est le seul endroit de la vue système à
demander les tailles de glyphe **10 et 12**. Premier emploi ⇒ l'atlas grandit
⇒ toute la frame noircit. D'où « ça noircit quand je clique sur une planète ».

#### ⚠️ Ce n'est pas la taille de l'atlas, c'est son **repaquetage**

Première correction insuffisante. Le vrai mécanisme est pire que la simple
recréation de texture : quand l'atlas déborde, macroquad **vide et repaquette
tous les glyphes** à de nouvelles positions (`Atlas::cache_sprite`) —

```rust
let sprites = self.sprites.drain().collect::<Vec<_>>();
self.image = Image::gen_image_color(new_width, new_height, ...);
for (key, sprite) in sprites { self.cache_sprite(key, image); }
```

— si bien que les quads **déjà empilés dans la frame** gardent des coordonnées
de texture qui ne désignent plus rien. D'où des tuiles noires sur *tout* le
texte de la frame, et pas seulement sur le caractère fautif.

**Correction définitive** : `precharger()`, appelée juste après le chargement de
la police et **avant la première frame**. Elle grave tout l'alphabet pour toutes
les tailles de `TAILLES_PRECHARGEES` (6..=26, le jeu en demande 8 à 22). Tout
débordement a donc lieu hors frame, là où il ne peut rien casser.

⚠️ **Un défaut de ma première correction** : `graver` insérait la taille dans
`TAILLES_GRAVEES` **avant** de vérifier que la police était chargée. Police
absente, la taille était réputée gravée pour toujours et plus rien ne la
rattrapait — précisément le cas du repli sur la police par défaut de macroquad,
qui a son propre atlas jamais préparé. Corrigé : on ne marque qu'après avoir
gravé.

#### La taille qui manquait : 5

`station.rs` bornait la taille de sa liste d'items à **9,0** (`clamp(9.0, 14.0)`).
Or `preparer` rend `round(9,0 × 0,6) = 5`, et la plage préchargée commençait à
**6**. La vue des vaisseaux — celle-là même où l'on regarde le Starship —
gravait donc la taille 5 **en pleine frame**, l'atlas se repaquetait, et tout le
texte de la frame passait en noir.

Une seule vue, une seule taille, tout le texte.

**Corrigé à la racine** : `police::TAILLE_MIN` / `TAILLE_MAX` sont déclarées
**dans `police`**, et ce sont les écrans qui s'y bornent — au lieu de trois
nombres posés à l'œil dans trois fichiers, dont un seul suffisait à rouvrir le
trou. La plage préchargée (4..=24) les couvre, et le test les vérifie *toutes
les deux* au lieu des seules tailles littérales.

⚠️ **Le test rougit sur l'ancienne plage `6..=26`** : c'est la garantie qu'il
attrape le défaut réel, et pas seulement qu'il décrit le correctif.

⚠️ **Fausse piste écartée en chemin** : je soupçonnais le `flush()` de
`planete::draw` de dessiner le texte en attente avec le matériau de planète.
`quad_gl.rs` dit le contraire — chaque `DrawCall` **mémorise son propre
pipeline** au moment de l'empilement (`geometry`, ligne 886) et `draw` le rejoue
tel quel. Un vidage ne peut pas appliquer le mauvais shader.

⚠️ **Autre piste écartée** : le binaire périmé. L'exe était bien postérieur à la
correction et contenait les symboles de `precharger` — donc le préchargement
tournait, et ne suffisait pas.

⚠️ **Piste laissée pour la prochaine fois.** Je n'ai pas pu prouver quel appel
déclenchait encore le repaquetage : l'analyse statique dit que `graver` couvre
tout. `graver` **écrit donc maintenant dans la console** quand une taille est
gravée en pleine frame, avec son numéro. Si les tuiles noires reviennent, cette
ligne nomme la coupable ; le test `les_tailles_du_jeu_sont_toutes_prechargees`
tient l'autre bout.

⚠️ Vérifié et **écarté** : le DPI. macroquad calcule `ceil(taille × dpi_scale)`
avant de mettre en cache (`text.rs`), ce qui aurait décalé toutes mes gravures —
mais `window_conf` ne demande pas `high_dpi`, donc `window_scale` reste à 1, et
l'écran est à 96 ppp. Fausse piste, mais elle valait la vérification.

**Première correction (conservée)** : `police::graver`, appelée depuis `texte` **et** `mesure`, à la
première demande de chaque taille. Elle vide le lot (plus rien n'attend), puis
grave **tout l'alphabet** d'un coup pour cette taille. L'atlas ne grandit donc
plus jamais en cours de frame, et une taille inventée par un futur appelant est
prise en charge d'elle-même.

⚠️ **`assainir` et `graver` forment une paire.** `graver` prépare `ALPHABET` et
rien d'autre ; `assainir` garantit que rien d'autre n'en sort — tout caractère
inconnu (€, ✓, un emoji) devient `?`. Si l'une des deux lâche, l'atlas
regrandit et les carrés noirs reviennent. C'est ce que tient
`assainir_ne_sort_jamais_de_lalphabet`, red-checké dans les deux sens.

⚠️ **Le défaut n'était pas dans la police mais dans le moment.** Les glyphes
manquants (section précédente) et les carrés noirs se ressemblent à l'œil et
n'ont rien à voir : les premiers touchent *certains* caractères, les seconds
*tous* ceux de la frame. C'est cette différence qui a désigné l'atlas.

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

**D-PARAM-1 soldée** : les réglages sont **sauvegardés** dans `reglages.json`,
à côté du jeu, et relus au démarrage.

La sauvegarde vit dans `appliquer()` — le **point de passage unique** de tout
changement. Répartie dans les six actions du menu, elle finirait par en oublier
une.

⚠️ **Tout échec de lecture retombe sur `default()`** : fichier absent, JSON
cassé, champ ajouté depuis la dernière version. Un réglage perdu n'est rien ; un
jeu qui refuse de démarrer pour un fichier de confort serait absurde. `assainir`
borne ensuite ce qui a été relu — le fichier est éditable à la main, et un gamma
à zéro ou un indice de palette hors liste donneraient un écran noir sans rien
expliquer.

**GAMMA au curseur** (0,5 → 2,5), appliqué **avant** la quantification : corriger
après ne ferait que déplacer des couleurs déjà choisies hors de la palette. Le
calcul (`gamma_depuis_x` / `x_depuis_gamma`) est séparé du dessin, donc testé.

⚠️ **Le tramage a été retiré** le 2026-08-05, sur jugement visuel : le motif se
voyait trop. C'était le remède aux **marches de la palette** (basculement de
bandes entières décrit plus bas) — elles redeviennent donc franches. La
saturation et l'écrêtage restent.

**La ligne PALETTE se grise à une seule palette installée.** Cycler la
ramènerait sur elle-même : le libellé ne bougerait pas, ce qui se lit exactement
comme un bouton cassé. La règle est dans `palette_reglable(rendu, nb_palettes)`,
à qui le nombre est **donné** — pas lu dans `palette::toutes()`.

⚠️ **C'est le sabotage qui a imposé cette forme.** Écrite d'abord en lisant le
global, la règle n'était testable que dans le cas installé : « griser toujours »
passait au **vert**, la branche « on rallume à partir de deux » n'ayant aucune
garde. En paramétrant, les quatre sabotages (griser toujours, ignorer le nombre,
ignorer le mode, seuil à 0) rougissent tous.

### Rendu pixel art (PARAMETRES → RENDU / PALETTE / TRAMAGE)

Trois états de rendu : `NET`, `PIXEL ART` (gros pixels, ce qui existait déjà sous
la touche **P**), `PIXEL ART + PALETTE` (quantification CIELAB). Trois réglages
s'ajoutent en mode palette, grisés sinon : **palette**, **gamma** et
**saturation**. Conception complète, corrections apportées au guide d'origine et
dettes : [`../conception/pixel_art.md`](../conception/pixel_art.md).

**Ajouter une palette** : déposer un `.hex` (format Lospec, un hexa par ligne)
dans [`assets/palettes/`](assets/palettes/) — ramassé au démarrage, aucun code à
toucher. **Une seule embarquée : Lospec 2000** — les trois premières (Resurrect 64,
Sweetie 16, PICO-8) ont été retirées le 2026-08-05.

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
Mesuré alors sur Resurrect 64 : un dégradé de gris ne tombait que sur **8 couleurs**, avec une
marche de L=49 à L=69, et tout ce qui dépasse 89 % s'écrase sur le blanc. Trois
défauts constatés à l'écran en découlent, chacun avec son remède mesuré :

| Défaut vu | Cause mesurée | Remède |
|---|---|---|
| Bandes qui basculent d'un bloc | marche de 0,18 en gris | ~~tramage de Bayer~~ — **retiré** le 2026-08-05 (motif trop visible). Les marches redeviennent franches : dette assumée. |
| Reflet en aplat blanc | spéculaire additif > 1,0 | **écrêtage** (cœur blanc, halo non) |
| Couleurs ternes, océans gris-violet | les **neutres** de la palette attirent les couleurs peu saturées | **saturation** ×1,9 à luminance constante (+53 % de chroma) |

Deux fausses pistes écartées par la mesure, à ne pas re-explorer : la recherche
CIELAB garde déjà 85 % de la chroma (pondérer L n'y change rien), et le tramage
ne désaturait pas — mesuré du temps où il existait.

⚠️ L'écrêtage n'agit **que dans la passe pixel art** : le spéculaire de
`planete.frag.glsl` n'est pas touché, le modifier changerait le rendu `NET` et
donc la galerie, les presets et les captures de non-régression.

Le mode est **global** (`reglages::etat_rendu`), pas par vue : les touches P et
le menu pilotent le même état, et P ne change que le mode (ni la palette, ni le
gamma, ni la saturation). `ecran/pixel.rs` est devenu la **source unique du blit** — les quatre
vues y passent, ainsi que le facteur `PIX` et la création de cible qui étaient
recopiés trois fois.

⚠️ **Seule la couche 3D est quantifiée.** Fond stellaire, orbites et textes
restent nets, comme c'était déjà la règle.

⚠️ **D-PIX-1** : le miroir CPU testable (`palette.rs`) et le shader
(`palette.frag.glsl`) sont deux écritures du même algorithme ; aucun test ne
compile de GLSL, donc rien ne garantit qu'ils restent d'accord.

## Echelle du système solaire

**Les distances étaient déjà réelles** : `a_monde = a_au * UA` avec `UA = 48`,
linéaire, sans compression, et le preset porte les vraies valeurs (0,39 → 39,5 UA).
Il n'y avait rien à corriger là.

**Échelle vraie depuis le 2026-08-05 : plus aucune exagération.** `R_TERRE` vaut
`4,2588e-5 × UA`, soit **0,002 unité** — le vrai rayon terrestre. Le Soleil suit
(109,2 rayons terrestres au lieu des **mille** qu'il valait). Les lunes aussi :
elles étaient exagérées *à part*, la Lune à 0,44 rayon terrestre au lieu de 0,27,
et Ganymède sortait plus petite qu'elle alors qu'elle la dépasse de moitié.

⚠️ **Conséquence assumée** : les planètes sont minuscules. Il faut approcher
vraiment pour qu'une planète fasse quelques pixels, et depuis l'orbite de Neptune
le système intérieur est un point. C'est le système solaire.

**Les lunes ont désormais leurs vraies orbites** (table `LUNES` : demi-grands
axes réels en km). Le placement procédural les mettait six fois trop loin — notre
Lune vers 0,8 unité au lieu de 0,123, soit 60,3 rayons terrestres.

**Les périodes suivent Kepler avec la masse du parent** (`MASSES_TERRESTRES`).
C'est indispensable : Io et la Lune sont à peu près à la même distance de leur
planète, mais Io boucle en 1,8 jour contre 27,3 — parce que Jupiter pèse 318
Terres. Sans les masses, toutes les lunes tourneraient à la même vitesse. La
constante est calée sur la période de notre Lune ; vérification indépendante :
Io ressort à 1,76 jour (vrai : 1,769). Triton est **rétrograde**.

⚠️ **Deux pièges trouvés en chemin.** `ajouter_lune_preset` prenait une taille
**relative** (`rayon_planete * taille_rel`) : lui passer un rayon absolu rendait
les lunes 2 000 fois trop petites. Et `poser_lune` forçait `.max(0.05)` sur le
rayon, soit **vingt-quatre fois la Terre** à l'échelle vraie — toutes les lunes
sortaient en boules géantes. `ajouter_lune_reelle` n'a ni l'un ni l'autre :
Phobos a le droit d'être invisible.

### Bouton d'échelle

`ECHELLE_CORPS` (dans `presets.rs`) multiplie les rayons du Soleil, des planètes
et des lunes **sans toucher aux distances**. À `1.0` = échelle vraie. Le monter
est le compromis habituel pour rendre un système lisible, et c'est désormais un
seul endroit.

**Réglé à ×5.** L'échelle magnifie chaque planète **avec son cortège** :
`distance_lune` la multiplie aussi. Seules les distances au Soleil restent
vraies.

⚠️ **Pourquoi les orbites de lunes suivent l'échelle.** Elles ne la suivaient
pas d'abord, et ×5 a mis **Phobos à l'intérieur de Mars** : il n'orbite qu'à
2,76 rayons martiens, ce qui plafonnait `ECHELLE_CORPS` à **1,15** — autant dire
aucun réglage. J'avais estimé le plafond à ×25 en regardant la Lune (60 rayons
terrestres) au lieu de la lune la **plus serrée**. C'est le test qui l'a dit.

### Zoom et plans de coupe

`DIST_MIN` (dans `camera.rs`) est passé de `2.0` à **`0.01`** : le plafond de
zoom monte de ×640 à **×128 000** pour un cadrage de référence de 1280. À
`ECHELLE_CORPS = 5`, la Terre mesure 0,010 unité — sans ce plancher, on ne
pouvait pas l'approcher assez pour qu'elle remplisse l'écran.

⚠️ **Baisser le plancher seul n'aurait rien donné.** La caméra prenait
`..Default::default()`, donc les valeurs de macroquad : `z_near = 0.01`,
`z_far = 10000`. Les deux tombaient à côté :

- `z_near` valait **exactement** le nouveau plancher → au zoom maximal la
  planète se posait sur le plan de coupe et disparaissait ;
- `z_far` coupait à 10 000 alors que `DIST_MAX` vaut 30 000 et que le nuage de
  Oort est à 96 000.

Les deux suivent maintenant la distance : `z_near = dist × 0.01`,
`z_far = dist × 1000`. Le **rapport** near/far reste donc constant — un near
minuscule avec un far énorme est ce qui fait clignoter les surfaces.
`les_plans_de_coupe_encadrent_la_cible` tient les deux marges.

### Palette

**Une seule, Lospec 2000**, embarquée par `include_str!` depuis
`assets/palettes/sources/lospec-2000.hex`. Embarquée et non simplement déposée :
il en faut **au moins une** même sans dossier d'assets, sinon le menu n'a rien à
proposer et `palette(0)` rien à rendre.

Les trois anciennes intégrées (Resurrect 64, Sweetie 16, PICO-8) sont retirées.
Les `.hex` déposés ont été **déplacés** dans `assets/palettes/sources/` (pas
supprimés) : ce sous-dossier n'est pas scanné, sinon Lospec 2000 serait chargée
deux fois et apparaîtrait en double. Déposer un `.hex` dans
`assets/palettes/` en ajoute toujours une.

**Ce qui était faux avant ça, ce sont les rapports de rayons.** Écrits à la main dans chaque appel :
Jupiter valait 3,1 fois la Terre au lieu de **11**, Pluton près de trois fois trop
gros. Les rapports étaient incohérents et rien ne le disait — c'est
vraisemblablement ce qui faisait que le système ne se lisait pas.

Ils viennent maintenant d'une table unique, `RAYONS_TERRESTRES` (valeurs UAI), via
`rayon_monde(nom)`. Le facteur d'exagération reste (`R_TERRE = 0,55` ; à l'échelle
vraie la Terre ferait un demi-millième de pixel) mais il est **commun**, donc les
rapports sont exacts.

⚠️ **Piège évité** : `ajouter_lune_preset` reçoit le rayon de la planète pour
calculer la limite de Roche, et ces rayons étaient **recopiés en dur** (`1.7`
pour Jupiter). Grossir Jupiter sans les toucher aurait posé ses lunes **dans** la
planète. Tout passe désormais par `rayon_monde`.

**Ceintures aux distances réelles** : disque épars 48 → 1 000 UA (Sedna monte à
937), nuage de Oort **2 000 → 20 000 UA** au lieu de 300 → 480. Le bord interne
est désormais le vrai ; le nuage externe (jusqu'à ~100 000 UA) reste tronqué.
⚠️ Conséquence assumée : il faut un zoom bien plus large pour apercevoir Oort.

⚠️ **Les rayons des lunes restent exagérés** et sans rapport exact : Phobos
mesure 11 km, soit 0,001 unité — invisible. C'est le même mensonge assumé que
l'échelle de l'ISS.

⚠️ **Limite de test** : `construire_preset_solaire` exige le contexte graphique.
Les tests prouvent que la table est juste, pas que le preset s'en sert — un rayon
réécrit à la main les laissait tous verts. D'où
`le_preset_solaire_puise_ses_rayons_dans_la_table`, qui inspecte le **texte
source** (`include_str!`). Inhabituel, mais c'est le seul garde possible ici.

---

## I.6 — Sélecteur : icônes rendues, contour, débordement des anneaux (2026-08-14)

### La demande

Remplacer la pastille de couleur unie du sélecteur par un vrai portrait
rendu de chaque astre, en cache (`ecran/icone.rs`, nouveau) — et pour les
gazeuses, régler trois défauts remontés à l'œil après une première passe :

1. **Le contour d'un pixel autour de la silhouette est trop clair** (blanc) —
   à assombrir.
2. **Les gazeuses ne se lisent pas à la même taille.** Une passe précédente
   les cadrait sur le rayon de la plus grande d'entre elles (Jupiter) pour
   leur donner une taille **relative** vraie — ce n'est pas ce qui était
   demandé : Jupiter, Saturne, Uranus et Neptune doivent occuper la **même**
   place à l'écran, comme n'importe quelle autre planète du sélecteur.
3. **Les anneaux sont coupés au lieu de déborder.** Le cadrage d'origine
   reculait la caméra pour faire tenir l'anneau dans les 48×48 px de la
   cible — exactement l'inverse de la demande : l'anneau doit dépasser le
   carré de l'icône, pas s'y comprimer.

### Ce qui a changé de conception

**Contour** : `CONTOUR` passe de `WHITE` à `BLACK` dans `ecran/icone.rs` —
un contour sombre se détache aussi bien d'une planète claire que d'une
sombre, et reste net sur la ligne survolée (fond cyan translucide) comme au
repos (fond nuit).

**Taille des gazeuses** : retour au cadrage **par corps**, celui de toujours
— `recul = rayon_du_corps × RECUL`, sans référence partagée. La référence
partagée (`Systeme::rayon_gazeuse_reference`, ajoutée à l'itération
précédente) est retirée : elle n'a plus de raison d'être une fois chaque
gazeuse cadrée sur elle-même.

**Anneaux qui débordent — le vrai problème.** Faire déborder un anneau d'une
cible de rendu à **résolution et champ de vision fixes** est contradictoire
avec « la planète garde la même taille » : à `recul` fixe, élargir le champ
pour loger l'anneau réduit forcément la taille apparente de la planète dans
le même cadre (elle occupe une fraction plus petite d'un champ plus large).
La solution n'est pas un compromis sur ces deux points à la fois, mais un
troisième axe : la **résolution de la cible**.

Pour un astre à anneau, la cible de rendu s'agrandit dans le **même rapport**
que le champ de vision s'élargit (`cote_gazeuse`, dérivé du rapport des
tangentes des deux demi-FOV — pas mesuré à l'œil, recalculé automatiquement
si `FOVY_DEG`/`FOVY_GAZEUSE_DEG` changent). La planète occupe alors
exactement le **même nombre de pixels** dans les deux cas (même `rayon`,
même `recul`, la résolution compense l'élargissement du champ) — seul
l'anneau, qui n'existait pas dans le champ étroit, apparaît, capté par le
surplus de champ.

Reste à faire déborder ce **surcroît de pixels** au-delà du carré de
l'icône à l'écran : le dessin (`skymap.rs::selecteur_dessiner`) étirait
jusqu'ici **toute** texture dans la même case `rayon_icone() × 2`, quelle
que soit sa résolution native — une cible plus grande s'y retrouvait donc
**compressée**, annulant tout le débordement. Corrigé en dessinant à une
échelle **proportionnelle à la résolution native de la texture**
(`icone::echelle_dessin`) plutôt qu'à une case de taille fixe : une texture
deux fois plus large se dessine deux fois plus grande, sans toucher à
l'échelle réelle de la planète qu'elle contient.

**Pourquoi une cible dédiée, plus grande, plutôt qu'un second passage.** Deux
tailles de cible de rendu coexistent maintenant (`scratch` 48×48 pour tout
le reste, `scratch_gazeuse` ~116×116 pour un corps à anneau) plutôt qu'une
seule cible redimensionnée à la volée : une `RenderTarget` macroquad est
créée à taille fixe (`render_target_ex(largeur, hauteur, …)`), et personne
n'a besoin d'un troisième format — Jupiter (sans anneau dans ce preset) et
toute tellurique restent sur la cible standard.

### Comment le vérifier

`cargo test`/`clippy` ne disent rien de la lecture d'un pixel — capture
d'écran obligatoire (`CAPTURE=<chemin.png> ./target/release/solar_game.exe`,
harnais dans `main.rs`), **agrandie avant recadrage** (Pillow, dans le
scratchpad) : à 1× l'icône fait 20–50 px à l'écran, illisible pour juger un
contour d'un seul pixel ou un débordement d'anneau au bord d'une case.
`window_conf` double la fenêtre sous `CAPTURE` (2000×1400, sans effet sur le
jeu normal) pour partir d'une image déjà plus grande.

### Résultat, vérifié à l'écran

Capture agrandie (Pillow ×3 à ×6, dans le scratchpad) puis lue directement :

- **Contour** : `BLACK` se détache nettement de chaque planète, claire
  (Uranus, Pluton) ou sombre (Mercure), au repos comme survolée.
- **Jupiter, Saturne, Uranus et Neptune occupent désormais exactement la
  même place à l'écran** — comparées côte à côte dans la même capture, les
  quatre disques font la même taille, anneau ou non.
- **L'anneau de Saturne déborde bel et bien** du carré de son icône,
  visiblement, jusque dans la ligne de Jupiter au-dessus — l'effet demandé,
  pas une approximation.
- Les telluriques (Mercure, Venus, Terre, Mars, Pluton) gardent leur taille à
  moitié d'une gazeuse, fond transparent compris — aucune régression sur ce
  qui marchait déjà.
- `cote_gazeuse()` calcule **116** à `FOVY_GAZEUSE_DEG = 90°` (contre 48 en
  standard) : conforme à la valeur dérivée à la main pendant la conception
  (`48 × tan 45° / tan 22,5° ≈ 115,9`).

**12 tests dans `ecran/icone.rs`** (tous red-checkés, y compris les deux
nouveaux qui répliquent la formule de projection pour garder la taille de
planète et la capture d'anneau), plus la suppression de
`Systeme::rayon_gazeuse_reference` (et ses deux tests) — la référence
partagée de l'itération précédente n'a plus de raison d'être une fois chaque
corps cadré sur lui-même. **661 tests verts, `clippy --all-targets` à 0
avertissement.**

### I.6 bis — Anneau moins dominant, tabulation, panneau animé, vraie Terre (2026-08-14)

Quatre retouches demandées après un premier jugement à l'écran de I.6.

**Anneau de Saturne, toujours trop gros.** Deux leviers, pas un seul :

1. **L'azimut de caméra ne suit plus la lumière** pour un astre à anneau — il
   se place perpendiculairement à la projection horizontale de l'inclinaison
   propre de l'anneau (`Astre::normale_anneau`, nouveau, exposé depuis
   `Apparence.anneau_normal`). Sans ça, l'azimut restait dicté par la
   position orbitale du moment : Saturne pouvait se présenter presque de
   face (anneau ouvert en grand cercle) selon l'heure, un hasard sans
   rapport avec ce qui se lit le mieux en 48 px. L'élévation suit un angle
   **fixe** de 15° (demandé), remplaçant l'angle générique des corps sans
   anneau pour ce cas précis.
2. **`FOVY_GAZEUSE_DEG` cadré sur l'anneau de Saturne (2,3 rayons), pas sur
   le maximum théorique du générateur aléatoire (2,5)** — 90° → 76°. Une
   gazeuse au maximum théorique perd un peu de son bord externe
   (`un_anneau_plus_large_que_saturne_deborde_du_cadre` le mesure) ; c'est
   assumé, Saturne est la seule gazeuse à anneau du préréglage solaire et
   c'est elle qui était jugée.

Débordement réduit, mesurable (~22 % de cible en moins), mais **pas
supprimé** : la demande précédente (« same size as jupiter » + « rings
overflow, i want it to ») borne d'en bas ce qu'on peut retirer sans revenir
sur ces deux points.

**Tabulation.** `selecteur::INDENT_TAB` (20 px) décale icône + arbre + nom
d'un bloc pour toute entrée de profondeur > 0 — comme un fichier texte, la
colonne se lit d'un coup d'œil comme une liste **sous** le Soleil. Retour
en arrière assumé sur I.6 (« le sélecteur n'a jamais plus de deux niveaux,
l'indentation ne distinguait rien » — vrai fonctionnellement, faux à l'œil).

**Panneau de droite animé.** `fiche::avancer_ouverture` (progression 0↔1 à
vitesse constante dans le temps, un quart de seconde) et `ease_in_out`
(smoothstep) pilotent un `decalage_horizontal` : le panneau glisse depuis
hors écran plutôt que d'apparaître d'un coup. `Skymap` garde
`fiche_affiche` distinct de `fiche_ouverte` — ce dernier redevient `None`
dès la fermeture demandée, mais le panneau doit continuer à montrer son
contenu pendant qu'il glisse encore hors champ. 6 tests, tous red-checkés ;
non vérifiable par capture statique (c'est une animation), la logique est
testée, le point de repos (immobile) confirmé à l'écran.

**La Terre.** Signalée trop verte dans son icône. Piste explorée d'abord —
et **fausse** : `eau` monté 0,62 → 0,71 → 0,78 (bien au-delà de la vraie
fraction d'océan) sans changement visible, y compris sous un azimut de
caméra complètement différent (testé, jeté). La vraie cause : le terrain
d'une tellurique se génère **de façon asynchrone** (job en tâche de fond,
budget de générations simultanées limité — `Planete::terrain_pret`,
préexistant, déjà utilisé pour ne pas figer un repli dans les captures de
non-régression de la galerie). L'icône du sélecteur ne le consultait pas :
elle pouvait mettre en cache un rendu pris **pendant** la génération, sur un
repli qui n'a rien à voir avec le preset — indépendamment de la valeur
d'`eau`. Nouveau `Astre::rendu_pret` (défaut `true`, `Planete` délègue à
`terrain_pret()`) consulté dans `IconesAstres::completer` avant tout rendu ;
un astre pas prêt est sauté **sans** compter contre `ESSAIS_MAX` — l'attente
n'a pas de plafond, contrairement à un rendu vide. `eau` repose à 0,71 (la
vraie fraction terrestre, un correctif légitime en soi, mais qui n'était pas
la cause du défaut).

**16 tests dans `ecran/icone.rs`, 6 dans `ecran/fiche.rs`** (tous
red-checkés). **676 tests verts, `clippy --all-targets` à 0 avertissement.**

### I.6 ter — Anneau à plat, bouton de repli agrandi, colonne « liquide » (2026-08-14)

Trois retouches de plus après un nouveau jugement à l'œil de I.6 bis.

**L'anneau de Saturne, cadré à l'horizontale plutôt qu'à l'azimut de la
lumière.** `azimut_et_elevation` devenait `cadre_camera`, et change de
signature : `(direction, elevation: f32)` → `(direction, haut: Vec3)`. Pour
un astre à anneau, `haut` n'est plus jamais `Vec3::Y` — c'est la projection
de la normale de l'anneau sur le plan perpendiculaire à la direction de vue
(`n - (n·fwd) fwd`, normalisée). Conséquence géométrique directe : le grand
axe de l'ellipse projetée est **forcé horizontal à l'écran**, quelle que
soit l'inclinaison réelle de l'anneau ou l'angle sous lequel Saturne est
prise ce jour-là dans son orbite. Avant, seul l'azimut (I.6 bis) suivait
l'inclinaison de l'anneau ; `haut` restait l'axe du monde, donc l'ellipse
pouvait sortir de travers.

**L'anneau se lisait encore « penché vers l'avant » une fois à l'horizontale**
— l'ellipse ne tournait plus de travers, mais gardait une ouverture verticale
(l'angle d'élévation de la caméra, qui restait celui des corps sans anneau).
Deux options proposées par l'utilisateur : réduire de 30 % *seulement* pour
Saturne, ou dériver la caméra directement des paramètres de l'anneau pour que
ça généralise. La seconde a été prise — `cadre_camera` construit désormais
une base orthonormée dans le plan de l'anneau lui-même (`e1 =
normalize(Y × n)`, replié sur `Z × n` si `n` est déjà vertical) et exprime
`direction`/`haut` par un angle d'élévation **paramétrique**
(`elevation_anneau_rad()`) : `direction = e1 cos a + n sin a`, `haut = n cos a
− e1 sin a`. Pas de cas particulier Saturne — l'angle est uniforme pour tout
astre à anneau, aujourd'hui la seule gazeuse annelée du préréglage solaire
mais pas censée le rester.

**Puis, sur demande explicite, l'angle a été mis à zéro.** À `a = 0`,
`direction` tombe **dans** le plan de l'anneau et `haut` devient la normale
de l'anneau elle-même : l'anneau se réduit à un trait horizontal, ouverture
nulle. Deux tests le pinnent : `lelevation_dun_anneau_est_nulle` (la
constante) et `a_elevation_nulle_la_direction_tombe_dans_le_plan_de_lanneau`
(la conséquence géométrique, pas seulement la valeur). Vérifié à l'écran
(capture + recadrage) : l'anneau traverse l'équateur de Saturne en une
ligne fine, sans déborder sur la ligne de Jupiter au-dessus.

⚠️ `CameraInfo` (utilisé par l'éclairage du shader) prend désormais `right`
et `up` dérivés du vrai `haut` de la caméra (`forward.cross(haut)`, etc.), pas
de `Vec3::Y` en dur — sinon l'éclairage aurait divergé de l'orientation
réellement rendue dès que `haut` s'écarte de l'axe du monde.

**Bouton de repli (◄) doublé.** `selecteur::ECHELLE_BOUTON_REPLI = 2.0`,
propre au sélecteur (pas au bouton similaire de la vitrine des briques).
`liste.rs` gagne des variantes `_bouton`/`_echelle_bouton` prenant le
facteur en paramètre (`bouton_repli_echelle`,
`colonne_items_depuis_echelle_bouton`, `zone_active_echelle_bouton`) ; les
anciennes fonctions restent en enveloppes à `1.0` **seulement** si un autre
appelant les utilise encore — `zone_active_echelle` n'en avait plus,
**supprimée** plutôt que gardée morte. Les trois usages (dessin, test de
clic, réservation d'espace des items) passent tous par le même facteur : un
bouton agrandi sans hit-test ni réservation d'espace assortis casserait
silencieusement les clics ou ferait chevaucher la liste sur ses bords.

**Fond et bordure « liquides ».** Toujours le thème terminal/minitel, pas un
autre : fond nuit conservé, juste la texture du panneau. `Skymap::
colonne_fond_liquide` remplace le rectangle plat à bordure cyan par 16
bandes horizontales semi-transparentes (alpha 0,85 → 0,60 de haut en bas,
léger tint cyan qui s'estompe avec la profondeur) plus une ligne de 2 px
très claire tout en haut (le « ménisque »). Bordure passée du cyan plein à
un **gris à 30 % d'opacité**, sur demande explicite (« I want to delete the
border color... for testing ») — un premier jet, pas un réglage final : pas
de shader/blur, juste des rectangles empilés, macroquad ne donnant rien de
plus simple sous la main ici.

**681 tests verts, `clippy --all-targets` à 0 avertissement.**

## Skymap V2 — marqueurs de planètes et lunes (2026-09-22)

**Le problème.** Dans la Skymap actuelle, les corps sont rendus à leur
échelle **physique réelle** (dette assumée, voir `STATE.md`) : au cadrage par
défaut, une planète tellurique fait quelques millièmes d'unité de rayon —
souvent bien sous le pixel. Rien ne dit où sont les planètes en vue globale
du système, ni ne permet de cliquer dessus (seule la colonne de gauche, en
arbre, le permettait). Les lunes ont le même problème en pire, et leurs
orbites n'existaient qu'en tout-ou-rien via la bascule globale « RING /
MOONS » (`src/menu/dessin.rs`), jamais liées à une sélection.

**Le choix du fork.** Plutôt que de retoucher la Skymap existante,
`src/ecran/skymap.rs` est copié tel quel vers `src/ecran/skymap_v2.rs`
(`SkymapV2`) — V1 reste intouchée, accessible depuis l'accueil à côté de V2
(« SKYMAP V2 - MARQUEURS »), pour pouvoir juger le résultat à l'œil sans rien
défaire s'il ne convient pas, et revenir en arrière sans coût si besoin.
Une seule ligne de V1 a bougé, purement mécanique : le littéral `Calques` de
`skymap.rs` gagne `lunes_de: None` (nouveau champ du calque, comportement V1
strictement inchangé).

**Le cœur générique.** `src/ecran/marqueur.rs` (nouveau) ne connaît que des
indices d'astres, jamais une catégorie : `marqueurs_pour` projette et filtre
(hors cadre, derrière la caméra) n'importe quelle liste d'indices,
`plus_proche` fait le hit-test, `dessiner` trace le triangle. Cette
itération ne l'alimente qu'avec deux familles — planètes racines et lunes
(`marqueurs_planetes`, `marqueurs_lunes`) — délibérément : étoiles,
vaisseaux/engins (dont l'ISS), stations et ceintures n'ont aujourd'hui
**aucun appelant** qui leur construirait un marqueur (spec §9). Les y
brancher plus tard est un ajout (une fonction de sélection d'indices de
plus, façon `marqueurs_planetes`), pas une reprise d'architecture.

**Quatre corrections au spec** (trois faites en préparant le plan, avant tout
code ; la quatrième constatée en relisant l'implémentation) :
1. La sélection qui pilote les lunes visibles doit lire `self.fiche_ouverte`
   et non `fiche_affiche` — ce dernier garde le dernier astre montré pendant
   que le panneau glisse hors champ et ne redevient jamais `None` ; s'en
   servir aurait laissé les lunes affichées indéfiniment après une
   désélection.
2. Le tracé des orbites de lunes ciblées passe par un nouveau champ de
   `Calques` (`lunes_de: Option<usize>`), pas par un appel séparé après
   `rendre` : le tracé des orbites vit **dans** la passe 3D
   (`RenduStandard::rendre`), sous la caméra ; un appel après coup n'aurait
   plus de caméra 3D active et serait passé par-dessus les corps.
3. La sélection d'indices pour les lunes filtre `Categorie::Lune` — pas
   « tout enfant direct » comme une première version du spec le prenait :
   l'ISS (`EnginOrbital`, `src/engin.rs`) a la Terre pour parent et aurait
   sinon reçu un marqueur, hors périmètre. `Systeme::lunes_de` reste
   générique (tous les enfants directs, même famille que `nb_lunes`) ; c'est
   `marqueur.rs` qui filtre à l'usage.
4. Le spec (§5 « Couleur ») disait que le marqueur reprend directement
   `Systeme::teinte_de(idx)`. Le code appelle en fait
   `selecteur::teinte_astre(sys, idx, cat)` — la même fonction que la
   pastille du sélecteur, qui éclaircit `teinte_de` (`relever`) et retombe
   sur une teinte par catégorie, pas sur le cyan d'interface, si `teinte_de`
   rend `None` : mieux vaut appeler directement la fonction du sélecteur que
   reproduire sa logique à côté, avec le risque de diverger plus tard.

**Tests et red-checks.** `Systeme::lunes_de` : 1 test
(`lunes_de_rend_les_enfants_directs_et_rien_d_autre`), 2 sabotages
red-checkés (filtre remplacé par une condition de catégorie ; filtre réduit
à « a un parent ») — les deux rouges comme attendu. `ecran/marqueur.rs` :
7 tests (bord du cadre inclus/exclu, rejet hors-champ et derrière la caméra,
sélection planètes racines/lunes avec exclusion de l'ISS, plus-proche avec
départage par ordre de liste et rayon-bord inclus, planète active sur les
trois branches), 8 sabotages red-checkés — tous rouges au premier essai,
aucun test faible détecté, aucun muscle-test nécessaire. `cargo clippy
--all-targets` à 0 avertissement une fois le branchement complet (les
`dead_code` transitoires des tâches 1 et 2, le temps que `SkymapV2` et
`Systeme::draw_orbites_lunes_de` aient un vrai appelant, ont disparu en
tâche 3).

**Une déviation, à la compilation.** Le spec calculait `lunes_de:
marqueur::planete_active(&self.sys, self.fiche_ouverte)` à l'intérieur même
de l'appel `self.rendu.rendre(..., &mut self.sys, Calques { ... })` —
refusé par l'emprunteur (E0502) : `&mut self.sys` et le `&self.sys` porté
par le champ `lunes_de` du littéral sont deux emprunts vivants simultanés
dans le même appel. Calculé dans une variable locale juste avant l'appel à
la place — même valeur, seule la forme change.

**Vérification à l'écran (Tâche 3, Step 14) : non faite.** Aucun accès
souris/fenêtre interactif n'était disponible dans l'environnement
d'implémentation, et le harnais `CAPTURE` n'ouvre que la Skymap V1. **Non
vérifié à l'écran, à faire par l'utilisateur** :
1. Au cadrage par défaut, un triangle sur chaque planète du système solaire,
   de la teinte de sa pastille dans la colonne.
2. Survol d'un triangle : contour blanc. Clic : la caméra se centre sur la
   planète et la fiche s'ouvre.
3. Terre sélectionnée : l'orbite de la Lune apparaît (violet pâle) avec un
   triangle sur la Lune ; pas de triangle sur l'ISS. Jupiter sélectionnée :
   ses lunes et leurs orbites.
4. Clic sur une lune de Jupiter : les triangles des autres lunes de Jupiter
   restent.
5. Clic dans le vide : orbites et triangles des lunes disparaissent (la
   fiche se referme).
6. Faire pivoter la caméra pour qu'une planète sorte du champ : son triangle
   disparaît.
7. La bascule « RING / MOONS » du menu ne fait plus rien en V2 (voulu, spec
   §6.4). Elle continue de changer d'aspect allumée/éteinte au clic (l'état
   `self.menu.orbites_lunes` bouge toujours) — c'est seulement son effet sur
   le rendu qui est coupé en V2 (`Calques::lunes` fixé à `false`, voir
   `skymap_v2.rs`) ; à juger à l'écran par l'utilisateur : une bascule qui
   s'allume sans rien changer peut sembler cassée même si c'est voulu.
8. Accueil : le bouton QUITTER reste entièrement visible sous PARAMETRES (le
   bloc de gauche a gagné une ligne).
9. SKYMAP (V1) : inchangée — « RING / MOONS » y fonctionne toujours.

**727 tests verts** (739 avec le binaire `mhd`) au moment de ce chantier, **`cargo clippy
--all-targets` à 0 avertissement.** Commits : `16199fa` (Tâche 1 —
`Systeme::lunes_de`, `Calques::lunes_de`, tracé ciblé des orbites de lunes),
`a2482f7` (Tâche 2 — cœur de `ecran/marqueur.rs`), `6990661` (Tâche 3 —
dessin des marqueurs et écran `SkymapV2` branché).


## Skymap V2 — retours visuels du 2026-09-22

Deux demandes après avoir vu la V2 tourner, et chacune a buté sur une
contrainte du moteur qu'il valait mieux constater que contourner.

### Le triangle se posait au centre de la planète

Demandé : « le triangle pointe sur le centre de la planète ; quand on zoome
il devrait être offset au-dessus avec un léger écart. »

Un décalage **fixe** en pixels ne règle rien : il reste au milieu du disque
dès qu'on approche. La pointe remonte donc de `rayon apparent + 6 px`, et le
rayon apparent se **mesure** — on projette un point du monde écarté du rayon
de l'astre le long de la verticale caméra, et on lit l'écart en pixels. De
loin, ce rayon est nul et il ne reste que l'écart : le rendu lointain ne
bouge pas. `Marqueur::ecran` porte désormais **la pointe** et non le centre,
si bien que le survol et le clic visent ce qu'on voit ; cliquer le disque
lui-même continue de marcher par le pick 3D.

Test `la_pointe_se_pose_au_dessus_du_disque_apparent` : corps ponctuel
(écart seul), rayon 1 à 20 unités (≈ 42 px de rayon apparent, calculé à la
main dans le test), et un corps deux fois plus gros qui monte deux fois plus
haut. Red-check : décalage ramené à l'écart fixe → rouge.

### Les orbites de lunes en semis de points

Demandé : « une ligne trois fois plus large au style mediumdashdot. »

⚠️ **`draw_line_3d` de macroquad n'a pas d'épaisseur** — il rend toujours un
pixel. Tout le calque d'orbites de `systeme/rendu.rs` vit avec cette
contrainte ; un trait large y est donc impossible, et ce n'est pas un
réglage qui manquait. Le tracé est passé en **2D, en passe interface**
(`ecran/trace_orbite.rs`) : la polyligne est projetée à l'écran, découpée au
motif mediumdashdot (trait 12 px, trou 4, point 2, trou 4) et tracée par
`draw_line`, qui prend une épaisseur.

Ce qui en découle, et qui vaut mieux que le détour initial : le champ
`Calques::lunes_de` et `Systeme::draw_orbites_lunes_de` (ajoutés au chantier
précédent) **n'ont plus d'appelant et sont retirés** — donc la ligne
`lunes_de: None` de `skymap.rs` disparaît avec eux, et **V1 finit
strictement intouchée**. Prix payé : ce trait ne passe pas le test de
profondeur, il se voit même quand la lune passe derrière sa planète. Assumé
— il n'apparaît que pour l'astre sélectionné.

Deux propriétés décident du rendu, et sont testées : le motif **se poursuit
d'un sommet au suivant** (sinon une orbite échantillonnée fin sort en trait
plein — c'est la longueur parcourue à l'écran qui compte, pas le nombre de
points), et il **commence par de l'encre** (sinon deux lunes voisines
paraissent décalées). Trois red-checks, tous mordants : motif rendu plein,
motif remis à zéro à chaque sommet, motif démarré sur un trou.

### ⚠️ Un quatrième test écrit puis jeté

`une_polyligne_degeneree_ne_produit_rien` affirmait garder contre deux
points confondus — « longueur nulle, et pas de boucle infinie ». **Aucun
sabotage ne le faisait rougir** : retirer la garde de longueur nulle, ou
celle de `points.len() < 2`, le laissait vert. La raison est que
`windows(2)` ne rend rien sous deux points, et que `parcouru < longueur` est
faux d'emblée à longueur nulle — la boucle infinie que je croyais prévenir
ne pouvait pas exister. Le test décrivait des propriétés vraies que rien
dans le code ne pouvait casser. Jeté, avec la note qui explique pourquoi à
l'endroit où il vivait. La garde de longueur nulle, elle, est **gardée**
comme défense contre un `dir` à NaN si la boucle est un jour réécrite — et
son commentaire dit désormais qu'elle est défensive, pas nécessaire.

**731 tests verts** (743 avec le binaire `mhd`), clippy à 0 avertissement.
La vérification à l'écran des neuf points ci-dessus **reste à faire**.

## ⚠️ Le crash à ×25000 sur une lune de Saturne (2026-09-22)

Signalé en jeu, tout de suite après le passage des orbites de lunes en trait
large : zoomer sur une lune de Saturne faisait tomber le jeu. **Défaut
introduit par ce tracé**, pas un défaut ancien.

**La cause, mesurée et non devinée.** `segments_motif` découpait la
polyligne **sur toute sa longueur projetée**, au pas du motif (22 px). Or à
fort zoom, une arête d'orbite se projette énorme : test à l'appui, une seule
arête de 10⁷ px produisait **909 091 segments** — pour une orbite de 96
arêtes, et à chaque frame. Le `Vec` grossissait jusqu'à épuiser la mémoire.

Le semis de points de V1 (`systeme/rendu.rs`) n'a jamais pu faire ça : son
pas est un **nombre d'échantillons à sauter**, borné par la taille de la
polyligne. Le mien était une **longueur en pixels**, bornée par rien. Le
même calque, deux unités de travail, et une seule tient au zoom.

**Le remède est le bon, pas le petit** : le travail suit désormais la
longueur **visible**. Chaque arête est découpée au rectangle de l'écran
(Liang-Barsky), et la part invisible n'est pas parcourue — elle est sautée
au modulo du cycle (`avancer`, en temps constant), ce qui garde le motif en
phase. Borner bêtement le nombre de segments aurait masqué le symptôme en
tronquant l'orbite.

Trois tests neufs, tous red-checkés : le travail reste borné sur une orbite
géante (celui qui reproduit le crash), rien n'est encré hors écran, et le
saut au modulo laisse le motif exactement où le parcours l'aurait laissé.

⚠️ **Un sabotage qui ne mord pas, et il avait raison** : remplacer
`l %= CYCLE` par `l %= CYCLE * 2.0` laisse tout vert — l'écart vaut un cycle
entier, que la boucle suivante consomme. C'est équivalent, donc le test ne
peut pas le voir. La vraie cible était l'avance de phase (`l -= reste`
retiré), et là ça rougit.

**734 tests verts** (746 avec `mhd`), clippy à 0.

## Skymap V2 — les tirets d'orbite de lune scintillaient (2026-09-23)

Signalé à l'œil : le trait mediumdashdot des orbites de lunes papillote dès
qu'on bouge la caméra ou que le temps passe, même sans zoomer. Gênant à
regarder — pas un crash, mais un défaut visuel net.

**La cause, mesurée.** `segments_motif` accumulait la phase du motif en
longueur **projetée à l'écran** (`avancer`). Cette longueur n'est pas
invariante : la perspective la déforme dès que la caméra ou la planète
parente bouge. Mesuré : un seul pixel de glisser souris décalait la phase de
8,1 px, soit **37 % du cycle de 22 px** ; une seule frame de temps qui passe,
10,6 px (48 %). D'où le scintillement — le motif « respire » à chaque
mouvement, même minime. Le contre-exemple du dépôt qui ne strobe pas :
`dessiner_points` (`systeme/rendu.rs`) ancre ses marques sur des **index
fixes** de la polyligne monde, jamais sur une mesure écran.

**Le remède : ancrer la phase sur l'abscisse curviligne du monde.** L'orbite
d'une lune (`Systeme::orbite_lune_de`) est une polyligne fixe dans le monde ;
la longueur d'arc qu'on y parcourt ne dépend ni de la caméra ni du temps.
Seule la **taille** d'un tiret (`cycle_monde`, combien d'unités monde pour un
cycle de 22 px à l'écran) suit le zoom — via la distance caméra → parent, au
même petit angle que le reste du dépôt (demi-FOV 22,5°). Le découpage en
tirets (`segments_motif_monde`) se fait donc entièrement en monde, sans
rien savoir de la caméra ; seule la projection finale (`segments_ecran`)
touche l'écran, et rogne comme avant (Liang-Barsky, marge de l'épaisseur du
trait).

⚠️ **`cycle_monde` est quantifié en puissance de deux** (`2^round(log2(x))`).
Sans ça, il varie en continu pendant un zoom et le glissement revient — côté
zoom cette fois. Par paliers de ×2, le motif ne se recale que rarement, et
franchement. Un second garde-fou, `perimetre / 2000`, plafonne le nombre de
cycles par orbite : c'est lui qui reprend le rôle de l'ancien `avancer` pour
tenir la contrainte mémoire (le crash à ×25000 documenté juste au-dessus) —
mais il n'est presque plus jamais nécessaire, puisque le travail suit
désormais le périmètre de l'orbite (fixe), plus le zoom écran (variable).

**Compromis assumé, à juger à l'œil** : la longueur d'un tiret À L'ÉCRAN
n'est plus rigoureusement constante. Elle varie un peu avec la perspective
le long de l'orbite (un tiret loin de l'axe de visée se projette plus court
qu'un tiret proche), et par paliers avec le zoom (le motif se recale net,
pas en fondu, à chaque franchissement de puissance de deux). C'est le prix
de l'absence de strobe.

Neuf tests, tous purs (`camera::base_orbite` pour fabriquer une `CameraInfo`
sans contexte graphique) : le test qui garde la correction compare, en
paramètre MONDE (jamais en pixels), les tirets décidés par deux caméras à
0,005 rad de lacet d'écart — la sensibilité réelle d'un pixel de glisser —
et les exige identiques. Red-check : une perturbation smooth et modeste de
`dist` (le genre qu'une régression discrète introduirait) reste **absorbée
par la quantification** et ne fait pas rougir le test — attendu, c'est le
but du palier de ×2, vérifié en l'amplifiant jusqu'à confirmer que le test
réagit bien à un écart réel. La réintroduction **fidèle** du mécanisme
d'origine (phase accumulée en longueur écran) fait rougir immédiatement
(187 tirets contre 186 entre les deux caméras). Les autres red-checks
mordent tous du premier coup, dont celui du garde-fou mémoire : retirer le
plancher `perimetre / 2000` sur l'orbite géante du test fait planter le
process par épuisement mémoire (103 Go alloués) — la même signature que le
crash à ×25000 documenté plus haut, avant même le message d'assertion.

**749 tests verts** (737 + 12 `mhd`), clippy à 0 avertissement.
