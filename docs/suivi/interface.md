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

**Bouche-trous** : 4 des 5 soldés. Reste **D-INT-2**, l'économie — chantier de
conception à part entière (tableau dans `STATE.md`).

**28 tests ajoutés, tous red-checkés.**

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

## Ce qui reste

- **D-INT-2, l'économie** : production, consommation, coûts, recherche. Les
  quatorze quantités sont figées à la main. La barre a été écrite pour pouvoir
  l'attendre — elle affiche une `Tresorerie`, d'où qu'elle vienne.
- **Le menu de développement** (`src/menu/`) cohabite avec la nouvelle interface.
  La question de sa place se posera.
- **Le parti pris esthétique** : sprites saturés contre Minitel monochrome. À
  juger à l'usage.
