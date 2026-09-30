# Journal — chantier Astres V2

Journal du chantier décrit par
[`conception/astres_v2.md`](../conception/astres_v2.md), lot par lot.

**Où se trouve le travail** : branche `chantier-astres`, dans le worktree
`.claude/worktrees/astres-v2`. **Rien n'est fusionné dans `main`**, rien
n'est poussé. 32 commits d'avance sur `main` au 2026-09-26.

**État au 2026-09-30** : 864 tests verts, clippy à zéro. **Le lot 5 est clos**
(ses sept presets de trou noir sont au tirage) et le **6.1 est livré** :
supergéante rouge, sous-naine sdB et étoile carbonée au tirage. **Le 6.2 —
la naine brune comme centre d'un système — reste à faire.**

Deux dettes restent, petites et consignées plus bas : le disque passe sous
l'horizon dessiné à très longue distance (`PX_MIN`), et la convention de
silhouette « mince » n'a jamais été essayée.

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
| **5.4** — la cible Interstellar, à comparer | fait, jugée « très satisfaisante » | `689b8f4` |
| **5.5** — la cible froide déclinée en appétits, plus la braise | fait | `689b8f4` |
| **5 — clos** : les sept presets au tirage | **fait et tranché** | |
| **6.1** — supergéante rouge, sous-naine, carbonée au tirage | **fait**, à juger à l'écran | |
| **6.2** — la naine brune comme **centre** d'un système | à faire | |
| 7 à 13 | à faire | |

Deux correctifs hors lot : le disque manquant de T Tauri (`3e2eb99`) et le
`crash.log` sorti du dépôt.

---

## Ce qu'il faut savoir pour reprendre

### Le lot 6, découpé en deux (2026-09-30)

**6.1 — les trois types « donnée seulement », et ce qui n'en était pas.** La
fiche du §3 les donne tous les trois pour de la donnée pure, « le rendu normal
leur convient ». **C'est faux pour l'étoile carbonée**, et mesuré : dérivée du
corps noir comme les autres, elle sort à `(1,00 / 0,68 / 0,40)` à 2900 K, quand
une naine M à 2400 K est à `(1,00 / 0,61 / 0,24)`. Elle serait donc **moins
rouge** que ce dont elle doit se distinguer — une étiquette, pas un type, ce que
le filtre de la §3 bis interdit.

D'où `filtre_teinte`, un multiplicateur sur la couleur de corps noir. Et c'est
la physique, pas un correctif : une carbonée est une photosphère froide vue **à
travers** ses bandes de C₂ et CN, qui mangent le bleu puis le vert. Avec le
filtre : `(1,00 / 0,375 / 0,08)`, la chose la plus rouge du catalogue.

**La sous-naine sdB a le problème inverse** : sa couleur est celle d'une naine
blanche — `(0,62 / 0,74 / 1,00)` contre `(0,67 / 0,78 / 1,00)`. Aucun réglage de
teinte ne les séparera. Ce qui la distingue est son **éclat**, cinq à vingt fois
supérieur : cœur d'hélium en fusion contre braise qui refroidit. Ça se lit au
halo, et la zone habitable part au loin. Une garde dit explicitement que les
teintes ne les séparent pas — si un jour elles le faisaient, le commentaire
serait faux et le test le dira.

⚠️ **Le vrai contenu du 6.1 est ailleurs : la table du tirage n'était pas
testable.** Les types ordinaires vivaient dans la chaîne de `if` d'`aleatoire`,
qui appelle `gen_range` — donc hors d'atteinte de tout test, **exactement**
comme le trou noir l'était avant le lot 4. Les particulières avaient reçu leur
garde à ce moment-là ; les ordinaires, non. La table est sortie dans
`ordinaire(p, q)` sur le modèle de `particuliere(p)`, `q` donnant la place dans
la plage de rayon et de luminosité — sans lui, une table de constantes figées
ferait de toutes les géantes rouges des jumelles, soit le défaut que le lot 1 a
corrigé.

**Un trou structurel trouvé en chemin** : rien n'empêchait de poser une planète
**dans** son étoile. Le garde-fou anti-collision de `instancier` ne compare que
les planètes entre elles. Ça n'avait pas de conséquence — le plus gros astre
était une géante de 7 unités et l'échelle du système suit `√L` — mais c'était
une coïncidence. La supergéante monte à 12, et la quasi-étoile du lot 11 est
annoncée large comme une orbite. D'où `premiere_orbite`, posé une fois.

**6.2 — la naine brune comme centre d'un système.** Pas de la donnée : elle est
déjà **mieux rendue que la plupart des étoiles** (quatre presets gazeux soignés),
mais côté planète. La mettre au centre demande une échelle de système propre
(`√L` la réduirait à rien, sa luminosité est de l'ordre de 10⁻⁵), une masse
propre comme le trou noir (`masse_luminosite` la coucherait au plancher), et
que la lumière principale du système vienne d'un corps qui n'est pas de
catégorie `Etoile`. Le chemin existe — `lumiere_secours`, pour la planète
isolée — mais il n'a jamais servi de source principale.

### Le lot 5 est clos. Tranché le 2026-09-30 : on garde les sept

**Décision de l'utilisateur.** La comparaison entre cibles ne débouche pas sur
un choix : *« pour garder de la variation je suis preneur de garder les 7 cas
de représentation de trou noir, ça rajoute de la diversité »*.

⚠️ **Conséquence, et elle n'est pas documentaire** : garder les sept veut dire
les **brancher sur le tirage**. Quatre d'entre eux n'existaient que dans les
vues de travail ; les laisser là aurait rendu la décision sans effet sur le
jeu. `regime()` puise donc désormais dans le catalogue entier.

**Le poids d'un appétit se partage entre ses cibles**, et c'est le point à ne
pas perdre : la décision ajoute de la *variété*, pas de la *fréquence*. Un trou
noir vorace reste aussi rare qu'avant — 20 % — il a simplement deux visages
possibles. Sans ce partage, décliner une teinte de plus rendrait mécaniquement
le vorace plus courant, et un état de crise cesserait d'en être un. Deux gardes
le tiennent : les sept sont atteignables, et les parts par appétit restent
0,35 / 0,45 / 0,20.

⚠️ **À graine égale, un trou noir existant change d'aspect.** C'est inhérent à
la décision. En revanche rien ne se décale en cascade : le preset est **dérivé**
de la graine, il ne consomme pas le flux aléatoire.

**Les deux axes, depuis le 5.5.** C'est la structure à comprendre avant de
toucher à quoi que ce soit ici :

- **l'appétit** (`nom`) — combien l'astre mange. Trois valeurs, et c'est le
  seul axe que le tirage connaît (`appetits()`, source unique dont `regime()`
  puise).
- **la cible** (`cible`) — de quoi ça a l'air. `refroidie()` applique une
  teinte à **n'importe quel** appétit, et le catalogue est le produit des deux.

Décliner une cible de plus coûte **une ligne**. C'est ce que le 5.5 est venu
vérifier, et ça a marché du premier coup.

Le tirage, lui, ne connaît que les **appétits** : il pondère un appétit, puis
partage ce poids entre les cibles qui le portent. Ajouter une teinte reste donc
une ligne, même du côté du jeu.

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

**Les sept cibles sont au tirage** depuis la décision du 2026-09-30. Ce
paragraphe disait l'inverse jusque-là — les cibles froides étaient des
candidates en attente de verdict, volontairement tenues hors du jeu pour ne pas
changer les trous noirs existants avant qu'on ait choisi. Le choix a été de ne
pas choisir.

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
- **vérifier que le motif a mordu, systématiquement.** Au 2026-09-30 un
  sabotage a raté parce que le motif supposait des `CRLF` là où le fichier a
  des `LF`. Le script l'a dit — il teste que le motif existe avant de
  remplacer — et sans ce garde-fou j'aurais lu un vert comme une preuve.
  C'est la même leçon que l'indentation changée, et elle est revenue.
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
