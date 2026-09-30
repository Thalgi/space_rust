# Journal — chantier Astres V2

Journal du chantier décrit par
[`conception/astres_v2.md`](../conception/astres_v2.md), lot par lot.

**Où se trouve le travail** : branche `chantier-astres`, dans le worktree
`.claude/worktrees/astres-v2`. **Rien n'est fusionné dans `main`**, rien
n'est poussé. 32 commits d'avance sur `main` au 2026-09-26.

**État au 2026-09-30** : 855 tests verts, `cargo clippy --all-targets` à zéro
avertissement. Les sous-lots **5.3 validé** et **5.4 jugé « très
satisfaisant »**. Le **5.5 est livré** et attend son jugement : sept presets
au catalogue, et le choix entre cibles reste ouvert.

---

## Où en est chaque lot

| Lot | État | Commit |
|---|---|---|
| 0 — masse dérivée de la luminosité | fait | |
| 1 — graine par étoile, mondes glacés | fait | |
| 2 — plancher en pixels du plasma | fait | |
| 3 — flares, arches ancrées, activité par type | fait | |
| 3 bis — arches espacées, taches non rondes | fait | |
| **3.1** — le champ magnétique | fait | `8761ca7` |
| **3.2** — taches sur les maxima, en groupes | fait | `ca6c1bb` |
| **3.3** — pieds sur deux polarités, répulsion | fait, **+ 4 correctifs** | `672f578`, `7ee33a6`, `f6ea783`, `9478798`, `1d78aac` |
| **3.4** — le shader lit le champ | fait | `f6e05f2` |
| **3.5** — le ruban, l'ancrage des pieds | fait, **+ 2 correctifs** | `2840050`, `c4edc55`, `bd677da` |
| **4** — le trou noir atteignable au tirage | fait | `4160e49` |
| **5** — le disque d'accrétion a un plan | fait | `f43691a` |
| **5.1** — shader dédié | fait | `8c2cec6` |
| **5.2** — le pli | fait | `f4a0e2d` |
| **5.3** — l'aspect, corrigé sur mesures | **fait et validé à l'écran** | `c32e3bf` + 7 correctifs |
| **5.4** — la cible Interstellar, à comparer | **fait**, jugée « très satisfaisante » | |
| **5.5** — la cible froide déclinée en appétits, plus la braise | **fait**, **à juger à l'écran** | |
| 6 à 13 | à faire | |

Deux correctifs hors lot : le disque manquant de T Tauri (`3e2eb99`) et le
`crash.log` sorti du dépôt.

---

## Ce qu'il faut savoir pour reprendre

### Le prochain pas : regarder les sept, puis trancher

Le 5.4 a été jugé « très satisfaisant », et le 5.5 en a tiré les variantes
demandées. **Ce qu'il reste à faire est un jugement, pas un codage.** Les sept
presets sont visibles ensemble en galerie des étoiles, en galerie des disques
et en vue isolée (touche `3`), sous les libellés « chaud / Interstellar /
braise » × « endormi / actif / vorace ».

⚠️ **Ne rien brancher avant le verdict.** Aucune cible froide n'est au tirage :
si l'une l'emporte, ce sont les constantes de `chaude()` qui bougeront, pas un
cas de plus.

**Les deux axes, depuis le 5.5.** C'est la structure à comprendre avant de
toucher à quoi que ce soit ici :

- **l'appétit** (`nom`) — combien l'astre mange. Trois valeurs, et c'est le
  seul axe que le tirage connaît (`appetits()`, source unique dont `regime()`
  puise).
- **la cible** (`cible`) — de quoi ça a l'air. `refroidie()` applique une
  teinte à **n'importe quel** appétit, et le catalogue est le produit des deux.

Décliner une cible de plus coûte **une ligne**. C'est ce que le 5.5 est venu
vérifier, et ça a marché du premier coup.

⚠️ **Absolu ou proportionnel : la distinction porte tout.** Ce que la cible
impose est absolu (teinte, gradient retiré, plafond du beaming, minceur) ; ce à
travers quoi l'appétit doit continuer de se voir est proportionnel (éclat,
turbulence, rotation, halo, cellules, spirale). Passer l'éclat en absolu
rendrait les trois appétits froids identiques — trois cellules pour un seul
objet. Gardé par `l_appetit_se_voit_encore_une_fois_refroidi`.

Les rapports proportionnels sont ceux **mesurés** entre `chaude()` et la cible
validée du 5.4, arrondis : la cible telle qu'elle a été jugée en ressort à
moins de 0,5 % (éclat 1,104 pour 1,1, turbulence 0,247 pour 0,25, rotation
0,605 pour 0,6). Le reste est exact. C'était la contrainte de l'exercice — un
découpage ne doit pas redéfiner ce qui a déjà été validé à l'écran.

**La braise**, quatrième teinte : ~3000 K, `chaud (1,0 / 0,58 / 0,28)`. Son
aura suit **gratuitement** — le shader prend la couleur du halo dans
`mix(froid, chaud, 0.6)` — et seule sa *présence* se règle (× 1,45), sans quoi
la variante ne se distinguerait que par un détail de bord.

**Les trois écarts de la cible froide, et rien d'autre.** Tous assumés comme
non physiques : ce sont exactement les retraits qu'a faits le film.

| | appétit « actif » | une fois refroidi | Pourquoi |
|---|---|---|---|
| teinte chaude | `(1.0, 0.97, 0.93)` | `(1.0, 0.80, 0.56)` | 4500 K, la valeur que Thorne a spécifiée pour Gargantua — **absolu** |
| `exposant_couleur` | 2,2 | **0,15** | température **constante en rayon** : un disque froid n'accrète pas, il ne s'échauffe pas vers le centre — **absolu** |
| `eclat` | 1,6 | **× 0,69** → 1,10 | sans gradient, `chaleur` vaut ~1 **partout** : l'éclat frappe le disque entier et 1,6 y ferait un aplat saturé. **Proportionnel**, pour que l'appétit se voie encore |
| `beaming_max` | 4,0 | **1,2** | « exceedingly lopsided, with the hole's shadow barely discernible » — **absolu**, c'est ce que la cible retire |
| `beta` | 0,35 | **0,35** | **inchangé** : la vitesse orbitale tient au rayon, pas à la température |
| bornes, pli, anneau | — | **identiques** | sinon on ne compare plus deux aspects mais deux objets |

Le dernier point est la condition de l'exercice, et il a sa garde
(`refroidir_ne_touche_pas_a_la_geometrie`, qui balaie les trois appétits et les
deux cibles froides). Le `beta` inchangé en est le corollaire honnête : c'est
le **plafond** qui porte le retrait du beaming, ce qui le désigne pour ce qu'il
est — un choix d'image, pas une autre physique.

**Aucune cible froide n'est au tirage.** `regime()` ne tire que des appétits
**chauds** : une cible n'est pas un état d'astre, c'est une réponse concurrente
à la question de l'aspect. Les ajouter changerait tous les trous noirs à graine
égale. Une garde tient les deux moitiés : présentes au catalogue (sinon elles
n'existent pas — le piège qui tenait le trou noir hors du jeu), absentes du
tirage.

### Ce qui reste ouvert, et que personne ne garde

- **Le GLSL ne compile qu'au lancement.** Aucun test du dépôt ne peut le
  vérifier. C'est arrivé une fois au 5.3 — une variable employée dix lignes
  avant sa déclaration, panic au démarrage. `verif::usage_avant_declaration`
  garde désormais cette classe d'erreur **dans le corps de `main`**, mais
  elle seule : un type faux, un appel inexistant passeraient encore. Une
  vraie garde demanderait `glslangValidator` en dépendance de test — chantier
  à part, non engagé.
- **Convention de silhouette** : nous prenons l'ombre comme unité (le noir
  remplit l'anneau, convention « épaisse », celle de l'image EHT). La
  convention « mince » — noir à 0,55 × le rayon de l'anneau, anneau tracé
  **par-dessus** la lumière du disque — reste possible et n'a pas été
  essayée. Les mélanger est incohérent (recherche 2, critère 6).
- **Échelle du disque protosolaire de T Tauri** : 16 rayons stellaires, soit
  0,28 UA, contre 6 UA dans la galerie des disques et ~100 UA en réalité.
  L'utilisateur l'a trouvé grand ; il est en fait vingt fois plus petit que
  celui de la galerie. Laissé tel quel, `RAYON_PROTOSOLAIRE` est isolé.
- **Incohérence connue** : à longue distance l'étoile est agrandie par le
  plancher en pixels (`PX_MIN`) alors que le disque garde sa taille
  physique — le disque passe donc sous l'horizon dessiné. Sans conséquence
  visible à cette échelle, mais réel.
- **Fins de ligne** : `core.autocrlf=true` normalise en LF ce que le dépôt
  stockait en CRLF, d'où des diffs gonflés sur les fichiers touchés. C'est
  une incohérence préexistante ; la trancher demande un `.gitattributes`,
  décision de l'utilisateur.

---

## Ce que le chantier a appris

Les leçons qui valent au-delà de leur lot. Chacune a coûté un aller-retour.

### Une absence ne se corrige pas par un réglage

Trois fois, le défaut signalé n'était pas un défaut de dosage mais une
**chose qui n'existait pas** :

- l'effet « autocollant » des taches venait de ce que **rien ne disait où le
  champ était fort** (3.1) ;
- le trou noir était injoignable parce que **personne ne produisait son mode
  de couronne et personne ne l'attendait** — son rendu existait pourtant
  depuis le début (lot 4) ;
- le bruit du disque ne pouvait **pas** produire de filaments : son mapping
  était isotrope par construction, et aucun réglage n'en serait sorti (5.3).

### Deux endroits qui décident du même fait divergent toujours

Quatre occurrences, dont trois trouvées par l'utilisateur à l'écran :

- le **rayon dessiné** d'une tache (`taille × intensité` au rendu, `taille`
  seul à l'ancrage) → les pieds d'arche tombaient hors de leur tache ;
- `PORTEE` du champ, dupliquée en GLSL → deux tests de littéraux ajoutés ;
- la chaîne **mode → constructeur**, recopiée dans la galerie des étoiles →
  T Tauri sans disque, et un mode 6 qui y aurait donné un trou noir ;
- le vocabulaire d'accrétion versé dans le `DisqueConfig` des anneaux de
  Saturne → sorti dans un type à part au 5.3.
- la **liste des régimes**, recopiée **quatre fois** — galerie des disques,
  galerie des étoiles, vue isolée, et une garde. Elle allait resservir au
  5.4 : ajouter un preset à trois de ces quatre endroits ne se serait vu
  qu'à l'écran. `Accretion::catalogue()` est désormais la seule liste.

**Et sa cause profonde, trouvée au 5.5 : deux faits dans un seul champ.** Un
preset mêlait *combien l'astre mange* et *de quoi ça a l'air*. Tant qu'il n'y
avait qu'une cible, ça ne se voyait pas. À la seconde, décliner l'une par
l'autre revenait à réécrire les trois écarts d'appétit pour chaque teinte — et
le jour où l'utilisateur retouche l'endormi, une seule des deux familles
bougerait. **La duplication n'était pas dans le code, elle était dans le
type.** Séparer les deux axes a ramené la troisième cible à une ligne.

**Et sa variante sournoise : un fait déduit au lieu d'être porté.** Le nom
d'un régime était *reconstitué* — une ceinture ⇒ vorace, un halo élevé ⇒
endormi, sinon actif. La déduction tenait pour trois presets qui
s'ordonnaient. Le quatrième n'a aucun trait qui le trahisse : il se serait
nommé « actif », et la galerie aurait affiché deux cellules du même nom.
C'est un champ, maintenant.

### Un sabotage doit être vérifié, pas seulement lu

**Trois fois** un test est resté vert à tort :

- le sabotage n'avait **pas eu lieu** (l'indentation avait changé après une
  extraction) — le vert ne voulait rien dire ;
- les entrées du test **évitaient le cas limite** par chance : un zigzag
  dont la *différence centrée* annulait l'alternance, et trois glissements
  qui rataient tous les 5,6 % d'ancrages refusés ;
- le test ne vérifiait **que les indices**, pas les sommets : un brin d'un
  point laissait deux sommets orphelins par frame.

Quand j'écris les entrées à la main, je les écris trop gentilles.

Deux autres pièges du red-check, rencontrés depuis :

- **saboter le test au lieu du code.** Retirer un nom de la *liste
  d'assertions* ne prouve rien ; c'est la production qu'on casse.
- **un sabotage resté en place.** La commande qui devait le défaire a échoué,
  et l'arbre a été annoncé propre à tort. C'est un test qui l'a rattrapé au
  coup suivant. **Relire avant d'annoncer**, ne pas se fier à la commande.
- **restaurer par copie remonte le temps.** `Copy-Item` donne au fichier
  restauré la date de **la sauvegarde**, donc plus ancienne que les
  artefacts du build saboté. Cargo compare les dates : il conclut que rien
  n'a changé et **relance le binaire saboté**. Au 5.4 ça s'est vu dans le
  bon sens — deux échecs sur un arbre pourtant sain, et le fichier disait
  bien la bonne valeur. Dans l'autre sens, c'est un sabotage qui passe au
  vert sans qu'on sache pourquoi. Le contenu a raison contre le test :
  vérifier le fichier, puis remettre sa date à l'heure avant de conclure.

### Mesurer désigne le coupable ; supposer coûte deux tours

Sur la pointe au sommet des arches, deux correctifs successifs ont visé à
côté. Le premier chiffre mesuré — **41** de rapport entre les pas
d'échantillonnage — a immédiatement désigné la vraie cause, et a montré que
ma première correction *aggravait* la géométrie tout en améliorant
l'échantillonnage.

### Ce qui ne se teste pas doit être nommé

Le chantier a accumulé des réglages qu'aucune assertion n'arbitre :
`PART_COURTES`, `MAX_ARCHES`/`MAX_COURTES`, `MARGE_ARCHES`, `EVASEMENT`,
`ANCRAGE`, `RAPPORT_TUBE`, `pli`, `beaming_max`, `halo`. Tous portent un
commentaire qui dit **entre quels deux défauts** ils arbitrent — c'est la
seule forme de garde possible.
