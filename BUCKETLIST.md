# Bucket list — générateur de systèmes solaires

Principe directeur : **chaque astre reste « 1 quad + 1 shader » (faible coût)**, et
toute la génération est **déterministe par graine (seed)** → un même nombre redonne
exactement le même système (reproductible, partageable).

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
- [x] **Atmosphères** : halo/rim lumineux (bleu sur mondes à océans, voile léger sinon, halo sur gazeuses/glacées).
- [x] **Fond étoilé** : champ d'étoiles lointaines autour de la caméra (effet infini).
- [x] **Contrôle du temps** : Espace = pause, Haut/Bas = accélérer/ralentir (×0.125–×16).
- [x] **Zone habitable** affichée (anneaux verts) + style des telluriques par température d'équilibre.
- [ ] **Systèmes binaires** (deux étoiles) — joli mais complique les orbites.

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
- [x] **Refonte complète des géantes gazeuses** (galerie ~27 presets) : niveau Jupiter (profil de jets EZ/NEB/SEB, zones laiteuses, ceintures marbrées, brume polaire cyclonique, festons, limb darkening), Grande Tache (cœur calme + anneau de vitesse 70-85 % + spirale + collier/sillage crème), tache sombre (Neptune), hexagone (œil central + eddies), bandes organiques (double-offset fbm), curl-noise. Voir `GAZEUSES_BUCKETLIST.md` + `NOISES_GAZEUSES.md`.
- [x] **Tous les types de géantes** : Sudarsky I-V, Jupiter chaud, méthane, soufre, naine brune (+ L/T/Y), hélium, Neptune chaud, carbone, proto-géante, rayée extrême.
- [x] **Anneaux variés** (`anneau_style`) : Saturne (lacunes Cassini/Encke), Uranus monobande bleu ciel, Neptune arcs, débris, ceinture granuleuse. Caméra galerie reculée pour les cadrer.
- [x] **Génération aléatoire des géantes** refondue (`apparence_gazeuse`) : palette HSV à teinte complémentaire, biais chaud/glacé, tache rouge ou sombre, profil de jets, brume polaire, tempêtes/cyclones, anneaux à style aléatoire. Utilisée par la skymap (`construire_systeme`) et le mode objet.
- [x] **Polyphemus (Avatar)** + presets du système solaire (skymap) remis au niveau de la galerie.

## 6. Technique / robustesse

- [ ] **RNG à graine** (crate `rand` + `rand_chacha`, ou xorshift maison) pour la reproductibilité.
- [ ] Trait/méthode commune de **génération** : chaque type d'astre sait se créer aléatoirement.
- [ ] **UI** : champ graine + bouton « Générer un nouveau système », infos de l'astre survolé.
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
- [ ] **Infos de l'astre survolé** (nom, type, distance) + champ graine éditable dans l'UI.

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
- [ ] **Supergéante** + densités/masses cohérentes (zone habitable très lointaine pour les géantes).
- [ ] **Masse gravitationnelle dépendante du type** (actuellement fixe à 1000).
- [ ] **Activité variable** : taux d'éruptions/taches plus élevé pour les naines M et étoiles jeunes.
- [ ] **Pulsation/variabilité** (étoiles variables) : luminosité qui oscille légèrement.
- [ ] **Classes spéciales** : Wolf-Rayet (vents forts), étoile à neutrons / pulsar (jets).

## 10. Systèmes binaires & trinaires — conception nécessaire

But : générer des systèmes à 2–3 étoiles crédibles. Points de conception à trancher :

- [ ] **Modèle gravitationnel** : aujourd'hui l'étoile est fixe à l'origine. Pour 2–3 étoiles,
      choisir entre (a) intégrer réellement les étoiles en N-corps autour du barycentre,
      ou (b) orbites analytiques pré-calculées des étoiles (plus stable, plus simple).
- [ ] **Types d'orbites planétaires** : **P-type** (circumbinaire, autour des deux étoiles) vs
      **S-type** (autour d'une seule étoile, l'autre lointaine) — règles de stabilité (zones interdites).
- [ ] **Éclairage à plusieurs sources** : le shader des planètes ne gère qu'une lumière ;
      étendre à 2–3 (couleurs/positions), avec ombres douces et double terminateur.
- [ ] **Zone habitable composite** (somme des flux des étoiles) à recalculer et afficher.
- [ ] **Couronnes/couleurs** distinctes par étoile, barycentre visible, repère caméra adapté.
- [ ] **UI** : la « source de lumière » et le « centre » ne sont plus uniques → généraliser `Systeme`.

---

## Ordre conseillé

1. **RNG à graine + étoile aléatoire** (type → couleur). Fondation, gros effet visuel immédiat.
2. **Planètes telluriques / gazeuses** (shader étendu + génération paramétrée).
3. **Orbites elliptiques + inclinaisons**.
4. **Ceinture d'astéroïdes** (mesh unique).

5. **Lunes, anneaux, comètes, atmosphères**.
6. **Polish** : fond étoilé, UI graine, zone habitable, contrôle du temps.
