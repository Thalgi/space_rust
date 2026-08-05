# Bucket list — générateur de systèmes solaires

Principe directeur : **chaque astre reste « 1 quad + 1 shader » (faible coût)**, et
toute la génération est **déterministe par graine (seed)** → un même nombre redonne
exactement le même système (reproductible, partageable).

> Légende : `[x]` fait, `[ ]` à faire, `[~]` **partiellement** fait (le détail suit
> sur la ligne). Relu et recalé le **2026-08-04**.

---

## 1. Étoile aléatoire (fondation)

- [x] Tirer un **type spectral** O B A F G K M (poids biaisés pour la variété).
- [x] Dériver du type : **température**, **rayon**, **luminosité**. (masse grav. encore fixe)
- [x] **Température → couleur de corps noir** (RGB) : approx. Tanner Helland.
- [x] Le shader du soleil prend la **couleur en paramètre** (`teinte`).
- [x] La **luminosité** pilote l'éclairage (couleur) des planètes. (zone habitable : à venir)
- [x] **Graine déterministe** (`srand`) + touche **G** pour régénérer un système.
- [ ] Variantes : naine rouge, géante rouge, naine blanche (tailles très différentes).

## 2. Planètes variées

- [x] **Telluriques** (petites, rocheuses, denses) vs **gazeuses** (grosses, bandées)
      vs **glacées** (lointaines, claires).
- [x] Shader planète étendu : **bandes** horizontales (gazeuses), océans/continents +
      calottes (telluriques), taches bleutées (glacées).
- [x] Paramètres procéduraux par planète : rayon, masse, 2 couleurs, phase orbitale.
      (inclinaison d'axe + vitesse de rotation : à venir)
- [x] **Nombre variable** (3–6), placement géométrique (type Titius-Bode), type selon distance.

## 3. Orbites réalistes

- [x] Orbites **elliptiques** (excentricité) au lieu de cercles parfaits.
- [x] **Inclinaisons** légères → systèmes non plats.
- [x] Tracé d'**ellipse** au lieu du cercle actuel.
- [x] Init au périhélie (position + vitesse) — orbite képlérienne stable.

## 4. Ceinture d'astéroïdes

- [x] **900** petits corps en billboards **batchés** (lots de 400) — pas un astre par caillou.
- [x] Anneau entre ~2 et ~4 UA, dispersion radiale + verticale (légère inclinaison).
- [x] Gravité **simplifiée** : orbites analytiques indépendantes, masse nulle (n'influence rien).

## 5. Ce qui manquerait encore (nice-to-have)

- [x] **Lunes** : orbite analytique autour de leur planète (suivent la planète), 3 max par planète.
- [x] **Anneaux** des géantes (Saturne larges/inclinés, Uranus quasi verticaux) + occlusion correcte (2 passes).
- [ ] **Comètes** : orbite très excentrique + **queue** pointant à l'opposé de l'étoile.
      (`Categorie::Comete` existe, mais aucune comète n'est encore engendrée.)
- [x] **Disque épars** et **nuage de Oort** (coquille sphérique) dans le preset solaire —
      quatre familles de petits corps au total.
- [x] **Anneaux d'Uranus et de Neptune** au catalogue (la galerie et la vue système
      lisent la même apparence).
- [x] **Atmosphères** : halo/rim lumineux (bleu sur mondes à océans, voile léger sinon, halo sur gazeuses/glacées).
- [x] **Fond étoilé** : champ d'étoiles lointaines autour de la caméra (effet infini).
- [x] **Contrôle du temps** : Espace = pause, Haut/Bas = accélérer/ralentir (×0.125–×16).
- [x] **Zone habitable** affichée (anneaux verts) + style des telluriques par température d'équilibre.
- [x] **Systèmes binaires** (deux étoiles) — joli mais complique les orbites.

## 5 bis. Réalisé en plus (hors liste initiale)

- [x] **Ceinture de Kuiper** distincte (glacée, dispersée) en plus de la principale.
- [x] **Bandes des gazeuses** retravaillées (bruit étiré + domain warping), grande tache tourbillonnante, hexagone de Saturne.
- [x] **Caméra focalisable** : clic sur un astre pour le centrer + suivre, bouton RETOUR.
- [x] **Filtre pixel** (rendu basse-déf upscalé).
- [x] **UI style Minitel** complète : menu, presets, toggles affichage.
- [x] **Presets sauvegardés en JSON** (nommables) chargés au lancement, bouton quitter qui sauve.
- [x] **Preset système solaire** (→ Pluton) et **preset Tau Ceti** (recherche réelle).
- [x] **Toggles** trajectoires / zone habitable indépendants.
- [x] **Couronnes de soleil variables** (taille + forme/spicules selon le type d'étoile).
- [x] **Vortex polaires polygonaux** aléatoires + ovales blancs + graine par gazeuse.
- [x] **Refonte complète des géantes gazeuses** (galerie ~27 presets) : niveau Jupiter (profil de jets EZ/NEB/SEB, zones laiteuses, ceintures marbrées, brume polaire cyclonique, festons, limb darkening), Grande Tache (cœur calme + anneau de vitesse 70-85 % + spirale + collier/sillage crème), tache sombre (Neptune), hexagone (œil central + eddies), bandes organiques (double-offset fbm), curl-noise. Voir
[`suivi/geantes_gazeuses.md`](geantes_gazeuses.md) +
[`conception/geantes_gazeuses.md`](../conception/geantes_gazeuses.md).
- [x] **Tous les types de géantes** : Sudarsky I-V, Jupiter chaud, méthane, soufre, naine brune (+ L/T/Y), hélium, Neptune chaud, carbone, proto-géante, rayée extrême.
- [x] **Anneaux variés** (`anneau_style`) : Saturne (lacunes Cassini/Encke), Uranus monobande bleu ciel, Neptune arcs, débris, ceinture granuleuse. Caméra galerie reculée pour les cadrer.
- [x] **Génération aléatoire des géantes** refondue (`apparence_gazeuse`) : palette HSV à teinte complémentaire, biais chaud/glacé, tache rouge ou sombre, profil de jets, brume polaire, tempêtes/cyclones, anneaux à style aléatoire. Utilisée par la skymap (`construire_systeme`) et le mode objet.
- [x] **Polyphemus (Avatar)** + presets du système solaire (skymap) remis au niveau de la galerie.

## 6. Technique / robustesse

- [ ] **RNG à graine** (crate `rand` + `rand_chacha`, ou xorshift maison) pour la reproductibilité.
- [ ] Trait/méthode commune de **génération** : chaque type d'astre sait se créer aléatoirement.
- [~] **Infos de l'astre** faites — panneau de droite au clic (nom, type, distance, rayon,
      habitabilité déduite) : [`conception/interface.md`](../conception/interface.md) §I.3.
      Champ graine éditable : toujours à faire.
- [x] Gestion correcte de la **transparence/profondeur** des anneaux (rendu en 2 passes).
- [x] **Material partagé** (un seul pipeline par type, cloné) -> corrige « Pipelines amount exceeded », gros gain init/mémoire GPU.
- [ ] **LOD / culling** si la ceinture d'astéroïdes devient lourde.

## 7. Refacto en cours (objectif : fichiers ≤ ~100 lignes)

- [x] **UI séparée** (`ui.rs`) + **contrôleur de rendu** interchangeable (`rendu.rs`, trait `Rendu`).
- [x] **Caméra** isolée (`camera.rs`) + **menu** isolé (`menu.rs`).
- [x] **`genese/`** découpé (`persistance`, `apparences`, `presets`, `mod`).
- [x] **Sortir les shaders GLSL** de `soleil.rs`, `planete.rs`, `fond.rs` en fichiers `src/shaders/*.glsl` (`include_str!`). planète 685→476, soleil 664→535.
- [x] Scinder `planete/` (apparence / anneau / materiau / mod) et `soleil/` (eruptions / materiau / rendu / mod).
- [x] Alléger `systeme/` (mod/gravite/rendu), `ceinture/` (mod/config/rendu), `menu/` (mod/dessin) — fichiers ~50–110 lignes. `etoile.rs` à 102.
- [x] **Socle de rendu impostor** (`impostor.rs`) : `push_quad` mutualisé + vertex shader sphère partagé (`shaders/impostor.vert.glsl`) + uniforms communs. Planète & soleil dessus.
- [x] **Table déclarative des uniforms planète** : source unique (nom + type + lecture `Apparence`) -> descripteurs ET setters. Ajouter un paramètre visuel = une ligne.
- [x] **Hot-reload des shaders** : lecture des `.glsl` au runtime (CARGO_MANIFEST_DIR, fallback `include_str!`), touche **R** = vide le cache de materials + reconstruit le système courant. Édition GLSL sans recompiler.
- [x] **Deux modes** (`ecran/` : accueil, skymap, objet) : écran d'accueil à 2 boutons, vue système complète, et vue d'un astre isolé (soleil/planète aléatoire) pour travailler le rendu d'un seul corps. `main` = aiguilleur.
- [ ] **Trait commun de génération** : chaque type d'astre sait se créer aléatoirement (`Genere`).
- [ ] **RNG à graine dédié** (`rand`/`rand_chacha`) plutôt que la RNG globale de macroquad.
- [~] **Infos de l'astre** faites (panneau de droite, étape I.3). Champ graine éditable :
      toujours à faire.

## 8. Aspect des planètes — manques

- [ ] **Telluriques** : vraie carte de relief (height/normal map procédurale) pour ombrer montagnes/cratères.
- [x] **Mondes de lave** (>600 K) : croûte sombre + fissures incandescentes émissives (brillent de nuit).
- [ ] **Couche de nuages** séparée et animée sur les telluriques (et ombre portée au sol).
- [x] **Côté nuit** : lumières de villes (amas dorés sur les terres, face non éclairée) sur les mondes à océans.
- [ ] **Côté nuit** : reflet spéculaire de l'océan au terminateur.
- [x] **Gazeuses** : advection par champ de flux façon curl-noise (champ sans divergence) + bandes organiques (double-offset fbm).
- [x] **Géantes de glace** (Uranus/Neptune) distinctes : Uranus voilée + anneau monobande bleu ciel, Neptune contrastée navy→cyan + Grande Tache Sombre.
- [ ] **Anneaux** : ombre de la planète sur l'anneau + ombre de l'anneau sur la planète.
- [ ] **Rotation propre** : axe d'inclinaison + vitesse de rotation par planète.
- [ ] **Calottes/biomes** liés à la température (banquise étendue si froid, désert si chaud).

## 9. Types d'étoiles — manques

- [x] **Naine rouge (M)**, **naine blanche** (minuscule, très chaude/bleutée), **géante rouge** (énorme, froide) — variantes au tirage.
- [~] **Supergéante bleue** faite. Densités/masses cohérentes : toujours à faire
      (`MASSE_ETOILE` reste fixe à 1000, cf. ligne suivante).
- [ ] **Masse gravitationnelle dépendante du type** (actuellement fixe à 1000).
- [ ] **Activité variable** : taux d'éruptions/taches plus élevé pour les naines M et étoiles jeunes.
- [ ] **Pulsation/variabilité** (étoiles variables) : luminosité qui oscille légèrement.
- [~] **Classes spéciales** : pulsar, magnétar, étoile à neutrons **faits** (`etoile.rs`, et
      `est_remnant()` leur retire la zone habitable). Wolf-Rayet : toujours à faire.

## 10. Systèmes binaires & trinaires — ✅ fait

Conception : [`conception/systemes_multiples.md`](../conception/systemes_multiples.md).
Presets `binaire`, `trinaire`, `quadruple`, plus Alpha Centauri (A+B réels).

- [x] **Modèle gravitationnel** : option (b) retenue — **orbites analytiques** dans un
      `ArbreStellaire` hiérarchique, évalué en `f(t)`. Plus stable qu'un N-corps sur des
      étoiles, et rejouable à l'identique.
- [x] **P-type et S-type** : `Foyer::Barycentre` (circumbinaire) contre `Foyer::Etoile(idx)`
      (circumstellaire). Chaque planète déclare le sien.
- [x] **Éclairage à plusieurs sources** : jusqu'à **4** (`CameraInfo::lights_pos/lights_color`),
      double terminateur compris.
- [x] **Zone habitable composite** : somme des luminosités (`zone_habitable(l_tot)`), tracée
      autour du barycentre quand elle tombe hors des orbites stellaires, et la HZ propre de
      chaque étoile sinon.
- [x] **`Systeme` généralisé** : plus de « la » source de lumière ni de centre unique.

---

## 10 bis. Engins en orbite — ✅ premier jet

- [x] **`Categorie::Engin`** : une station assemblée dans `vaisseau/` peut être mise en
      orbite dans un système (`src/engin.rs`), avec orbite analytique comme une lune.
- [x] **L'ISS autour de la Terre** dans le preset solaire — le premier pont entre le
      constructeur de stations et le générateur de systèmes.
- [ ] ⚠️ **L'échelle est exagérée** et le restera tant qu'il n'y aura pas de vue « orbite
      basse » : à l'échelle réelle l'ISS fait 9 × 10⁻⁶ unité de monde. Facteurs nommés
      (`ORBITE_ISS`, `ECHELLE_ISS`).
- [ ] Vaisseaux **mobiles** (transferts entre astres), et l'ISV / le Starship en orbite.

## 10 ter. Paramètres du jeu — ✅ premier jet

- [x] **Écran PARAMETRES** depuis l'accueil, modèle testable (`src/reglages.rs`) séparé de
      l'écran (`src/ecran/parametres.rs`).
- [x] **Mode d'affichage** : fenêtré / plein écran sans bordure. ⚠️ Pas de mode **exclusif** :
      macroquad n'expose qu'un `set_fullscreen(bool)`, et miniquad l'implémente déjà en
      `WS_POPUP` plein écran — « plein écran » et « sans bordure » seraient le même bouton.
- [x] **Taille de fenêtre** : 9 entrées en 4:3, 16:10 et 16:9, jusqu'à 1920 × 1200. Pas de 4K.
      Grisée en plein écran, où c'est l'écran qui décide.
- [x] **Pipeline de rendu** : `NET` / `PIXEL ART` / `PIXEL ART + PALETTE 64` — voir §10 quater.
      Plus palette, tramage et saturation, grisés hors du mode palette.
- [x] **Bouton QUITTER** sur l'accueil, sous PARAMETRES. Sort de la boucle de jeu, sans
      `process::exit`, pour que miniquad ferme sa fenêtre proprement.
- [ ] **Sauvegarde des réglages** (dette D-PARAM-1) : ils repartent aux valeurs par défaut
      à chaque lancement. `genese/persistance.rs` sait déjà écrire du JSON.
- [ ] Autres réglages à venir : volume, langue, taille des pixels (dette D-PIX-4 : `PIX = 2`
      est figé).

## 10 quater. Rendu pixel art — ✅ premier jet

Conception : [`conception/pixel_art.md`](../conception/pixel_art.md).

Le sous-échantillonnage existait depuis longtemps (touche P) ; ce qui manquait
était la **quantification vers une palette fixe**, sans quoi l'éclairage 3D
produit des dégradés continus — de la 3D basse résolution, pas du pixel art.

- [x] **Palettes** (`src/palette.rs`), l'hexadécimal comme source unique. Trois intégrées
      (Resurrect 64, Sweetie 16, PICO-8) **de longueurs différentes**, plus tout `.hex` déposé
      dans `assets/palettes/` — format Lospec, ramassé au démarrage, aucun code à toucher.
- [x] **Conversion sRGB → CIELAB** côté CPU, une fois par couleur : le shader ne convertit
      que le pixel courant, une fois au lieu de N+1.
- [x] **Quantification GPU** (`src/shaders/palette.frag.glsl`), distance au carré, `discard`
      des pixels transparents (l'espace est majoritairement vide).
- [x] **Tramage ordonné de Bayer 8×8** — le remède au basculement de bandes. Réglable
      (NON / LEGER / FORT), `FORT` par défaut car calé sur la pire marche mesurée (0,18).
- [x] **Écrêtage des hautes lumières** — le remède au reflet spéculaire en aplat blanc.
      Le cœur du reflet reste blanc, son halo ne l'est plus.
- [x] **Saturation avant quantification** — le remède aux couleurs ternes (océans gris-violet).
      À luminance constante, ×1,9 par défaut : +53 % de chroma en sortie, mesuré. Le gain
      retombe dans les hautes lumières, ce qui supprime aussi l'anneau cyan du reflet.
- [x] **Blit unifié** : `ecran/pixel.rs` devient la source unique du blit, du facteur `PIX` et
      de la création de cible — c'était recopié dans trois vues.
- [x] **Réglage global** : les touches P et le menu pilotent le même état ; les trois booléens
      `pixelise` séparés ont disparu.
- [ ] **D-PIX-1** : le miroir CPU testable et le shader sont deux écritures du même algorithme ;
      aucun test ne compile de GLSL. La surface de divergence a grandi (écrêtage, tramage).
- [ ] **D-PIX-3** : pas de contour (passe Sobel / profondeur).
- [ ] **D-PIX-4** : `PIX = 2` figé — une ligne de menu de plus.
- [ ] **D-PIX-5** : tramage en espace écran, donc fixe pendant que les objets bougent.
- [ ] **Quantifier toute l'interface** : aujourd'hui seule la couche 3D l'est, les textes et le
      fond stellaire restent nets. Décision assumée, pas un oubli.

## 10 quinquies. Catalogue reproductible — ✅ fait

Le catalogue tirait graine **et** taille au sort à chaque construction : sa
« Terre » était une planète différente à chaque ouverture de l'écran, donc une
référence à laquelle rien ne pouvait se comparer.

- [x] **Graine et taille déduites du nom** du preset (`genese::graine_de_nom`,
      `ClasseTaille::rayon_pour`). Plus aucun `gen_range` dans les deux catalogues.
- [x] **Le brassage passe à la galerie** : `variation` incrémentée par G, décalage
      **nul au départ** — la vue par défaut EST le catalogue canonique.
- [x] **Le système solaire s'y aligne** : les corps à preset unique sont identiques
      à leur vignette de galerie ; `fige()` ne reste que sur les presets réutilisés.
- [x] **Le catalogue se teste enfin** (il exigeait le contexte graphique) : 5 tests,
      dont « chaque preset demandé existe », jusqu'ici garanti par un seul `panic!`.
- [x] **Bug de LOD** : `planete::set_viewport_h` n'était réglé que par la galerie ;
      en mode pixel art, les planètes étaient ombrées avec le détail d'un plein écran.
- [x] **Plafond de palette 64 → 256** : Lospec 2000 (182) et AllStars (128) étaient
      **rejetées puis ignorées**, avec pour seule trace une ligne de console.
- [x] **Plus de rejet silencieux** (lot E) : les `.hex` refusés sont listés à l'écran
      dans PARAMETRES, nom de fichier et raison. La lecture disque ne décide de rien,
      le tri est pur, la mise en forme est testée — 5 tests red-checkés.

## 11. Interface de jeu (vue système) — ✅ premier jet

Conception : [`conception/interface.md`](../conception/interface.md).
Journal : [`suivi/interface.md`](interface.md).

C'est la **première interface de jeu** du projet : tout ce qui existait avant
(menu graine, presets, hot-reload) est de l'outillage de développement.

- [x] **Barre de ressources** : les 14 sprites 16×16 en grille 7×2, chaque produit sous sa
      matière première. Filtrage au plus proche voisin, échelles entières seulement.
- [x] **Nom du système**, dérivé de l'étoile hôte.
- [x] **Sélecteur d'astres** à gauche, rétractable, un dixième de la largeur au plus,
      en **arbre** : étoiles à la racine, planètes dessous selon leur foyer (type S sous
      leur étoile, circumbinaires sous un barycentre). Lunes et ceintures exclues.
- [x] **Panneau d'astre** au clic : nom, type, distance, rayon, **habitabilité déduite**
      de la luminosité cumulée et de la distance orbitale.
- [x] **Vignette rendue** de l'astre dans le panneau (cible de rendu dédiée, avec son
      propre tampon de profondeur).
- [x] **Noms propres** sur les corps des presets ; numérotation orbitale (I, II, III…, et
      « III-2 » pour une lune) partout ailleurs.
- [ ] **L'économie** : production, consommation, coûts, recherche. Les quantités affichées
      sont **figées** (dette D-INT-2 dans [`STATE.md`](../../STATE.md)). C'est un chantier
      de conception à part entière — les 14 ressources et leurs 3 chaînes de raffinage
      (minerai→métal, minerai rare→métal rare, nourriture brute→transformée) sont déjà
      inscrites dans les sprites.
- [ ] **Pas de monnaie** : décision de conception — l'**énergie** en tient lieu. Rien ne
      s'achète avec un nombre abstrait.

---

## Ordre conseillé

1. **RNG à graine + étoile aléatoire** (type → couleur). Fondation, gros effet visuel immédiat.
2. **Planètes telluriques / gazeuses** (shader étendu + génération paramétrée).
3. **Orbites elliptiques + inclinaisons**.
4. **Ceinture d'astéroïdes** (mesh unique).

5. **Lunes, anneaux, comètes, atmosphères**.
6. **Polish** : fond étoilé, UI graine, zone habitable, contrôle du temps.
