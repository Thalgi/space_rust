# Suivi — Assembleur de véhicules

> Journal de bord du chantier décrit dans
> [`conception/assembleur.md`](../conception/assembleur.md). Une section par
> étape, écrite **au fil de l'implémentation** : ce qui a été fait, ce qui a été
> mesuré, ce qui a résisté.
>
> Rappel de la méthode retenue (héritée du chantier ISV,
> [`stations.md`](stations.md) §C.29) : **tout test ajouté est red-checké** —
> on casse volontairement ce que l'assertion prétend garder, on vérifie qu'elle
> rougit, on remet. Un test qui reste vert quand on brise sa cible ne mesure pas
> ce qu'il annonce, et l'ISV a montré que c'est le mode de défaillance dominant
> de ce projet (six cas sur six).

---

## Tableau de bord

**Lot 1 — combler ce qui manque au code actuel** (indépendant de l'assembleur)

| # | Étape | État |
|---|---|---|
| L1.1 | Catalogue de briques : source unique + test d'énumération | ✅ |
| L1.2 | Toute variante cuit une géométrie finie et non vide | ✅ |
| L1.3 | `generer_est_deterministe` réécrit sur la géométrie | ✅ |
| L1.4 | Accord `rayon_local` / `englobant_local` + exception `Panache` | ✅ (avec dette ouverte) |
| L1.5 | `complexite_influe_sur_le_nombre_de_pieces` : `>=` → `>` | ✅ |
| L1.6 | Enveloppes de collision en **capsules** (au lieu de sphères) | ✅ |
| L1.7 | Overlay de débogage : enveloppes (**E**) et fils numérotés (**F**) | ✅ |
| L1.8 | Mesureur automatique par tranchage + catalogue `docs/reference/fils.md` | ✅ |

**§9 — le boudin** (indépendant des deux lots, voir `conception/assembleur.md`
§9) : enveloppe rectangle pour les plaques, appliquée aux boucliers de l'ISV. ✅

**Lot 2 — l'assembleur**. Voir
[`conception/assembleur.md`](../conception/assembleur.md) §7.

| # | Étape | État |
|---|---|---|
| L2.1 | Identifiants stables de ports libres | ✅ |
| L2.2 | `retirer` + propriété d'aller-retour | ✅ |
| L2.3 | Undo/redo + propriété de séquence, par graine | ✅ |
| L2.4 | Palette + les deux partitions | ✅ |
| L2.5 | Sérialisation + aller-retour géométrique | ✅ |

**Lot 3 — compléter le modèle pour l'écran**. Trois manques relevés en
confrontant §8 au `Chantier` livré (voir `conception/assembleur.md` §7.1).
Additifs, en lecture seule, testables sans vue.

| # | Étape | État |
|---|---|---|
| L3.1 | `pose_prevue` — la source unique du fantôme (§8.3) | ✅ |
| L3.2 | `sous_arbre` public — surlignage de la sélection (§8.3) | — |
| L3.3 | Désignation : port et pièce sous le curseur | — |

**Lot 4 — l'écran** (§8.2–8.4) : découpage à arrêter **à la fin du Lot 3**,
une fois le modèle réellement complet. **Lot 5 — ce que seul l'écran
permet** : overlay §8.5, sauvegarde disque, arbitrage L1.4, composites
(`figer`). Détail et justification du découpage : `conception/assembleur.md`
§7.1.

**État de départ** (commit `32300e9`, 2026-07-31) : 186 tests verts en 0,07 s
(`cargo test --release`), 37 avertissements `dead_code` sur le binaire, arbre
propre.

---

## Questions ouvertes

*(Les points qui demandent un arbitrage et qui bloquent, ou qui engagent une
décision qu'on ne peut pas défaire seul. Vidé au fur et à mesure.)*

*(vide)*

---

## Journal

### L1.1 Catalogue de briques : quatre sources ramenées à une (2026-07-31)

**Le problème** (`conception/assembleur.md` §5.1) : la même information vivait à
quatre endroits indépendants dans `ecran/station.rs` — le compte
(`Categorie::Briques => 27`), les bras d'un `match i` de 27 lignes, trois
constantes d'indice (`BRIQUE_EQUIPAGE = 20`, `BRIQUE_RADIATEUR = 6`,
`MEGA_ISV = [1]`) qui disaient quels boutons activer, et un quatrième `match`
qui disait quelle épine portait l'ISV complet. Plus un **attrape-tout `_ =>`**
au bout, qui absorbait silencieusement toute entrée en trop.

**La correction n'est pas défensive, elle est structurelle.** Ajouter des tests
sur ces quatre sources aurait vérifié qu'elles sont d'accord ; on a préféré
faire qu'elles ne puissent plus être en désaccord. Le nouveau
`src/ecran/catalogue.rs` tient une table d'`Item { libelle, fabrique }`, et
**les capacités se déduisent de la fabrique** :

| Avant | Après |
|---|---|
| `Categorie::Briques => 27` | `BRIQUES.len()` |
| `match i { 6 => demo_radiateur_mega(regime), … }` | une entrée de table |
| `const BRIQUE_RADIATEUR: usize = 6` | `matches!(fabrique, Fabrique::Regime(_))` |
| `const BRIQUE_EQUIPAGE: usize = 20` | `matches!(fabrique, Fabrique::Repli(_))` |
| `const MEGA_ISV: [usize; 1] = [1]` | `Fabrique::Isv(_)` |
| `match idx { 1 => Some(Epine::Hexagonale) }` | `Fabrique::Isv(e) => Some(e)` |
| `_ => (demo_chantier(), …)` | *(supprimé — plus d'indice hors table)* |

Un item bâti par `Fabrique::Regime` a une propulsion à allumer **parce qu'il lit
le régime**, pas parce qu'une constante le dit ailleurs. Il n'y a plus rien à
tenir d'accord. Les trois constantes d'indice ont disparu, et
`epine_courante` / `rotation_possible` / `allumage_possible` sont devenues des
une-ligne qui interrogent la table.

**Huit tests ajoutés**, six dans `catalogue.rs`, deux dans `station.rs` :

| Test | Ce qu'il garde |
|---|---|
| `chaque_item_batit_une_station_non_vide` | chaque entrée de chaque table produit des pièces, **aux trois réglages** (repos, mi-course, plein) |
| `les_libelles_sont_distincts` | deux entrées de menu identiques = le symptôme exact de l'attrape-tout |
| `les_capacites_se_deduisent_de_la_fabrique` | remplace les trois constantes d'indice, en nommant les items par leur libellé |
| `une_capacite_annoncee_change_vraiment_la_geometrie` | un item qui annonce le repli ou l'allumage doit **bouger** quand le réglage bouge |
| `seul_litem_a_deux_moities_porte_une_epine` | moitié tournante et épine vont ensemble |
| `les_deux_moities_de_lisv_partagent_leur_epine` | la correction de §C.10, rendue vérifiable |
| `un_tour_de_cyclage_visite_chaque_item_une_fois` | le modulo de la touche **D** est une bijection |
| `seul_le_generateur_nest_pas_catalogue` | le `max(1)` de `nb()` et la branche paramétrique de `rebatir` |

**Deux tests plutôt qu'un sur les capacités, et c'est délibéré.**
`les_capacites_se_deduisent_de_la_fabrique` vérifie *quels* items annoncent quoi ;
`une_capacite_annoncee_change_vraiment_la_geometrie` vérifie que l'annonce
**a un effet**. Le second est formulé à l'envers de la déduction — on n'écrit pas
« `Repli(_)` ⟹ `rotation()` », ce qui reviendrait à recopier le `matches!` (piège
n° 1 du catalogue §C.29 : un test qui récite l'implémentation). On exerce le
constructeur aux deux bouts de course et on compare la géométrie.

#### Red-check : 8 sur 8

Chaque test cassé volontairement sur sa propre cible, vérifié rouge, remis :

| Sabotage | Rougit |
|---|---|
| une entrée qui bâtit `EtatStation::Vide` | `chaque_item_batit_une_station_non_vide` seul |
| `Fabrique::Repli(f) => f(0.0)` (réglage ignoré) | `une_capacite_annoncee_change_vraiment_la_geometrie` seul |
| section d'équipage bâtie avec `Epine::default()` | `les_deux_moities_de_lisv_partagent_leur_epine` seul |
| `rotation()` étendu à `Fabrique::Brique(_)` | les **deux** tests de capacité |
| deux libellés identiques | `les_libelles_sont_distincts` seul |
| une moitié tournante sur un item sans épine | `seul_litem_a_deux_moities_porte_une_epine` seul |
| deux derniers indices repliés sur le même item | `un_tour_de_cyclage_visite_chaque_item_une_fois` seul |
| une table donnée au générateur | `seul_le_generateur_nest_pas_catalogue` seul |

Le quatrième est le plus instructif : étendre `rotation()` à une fabrique qui ne
lit pas le repli fait rougir les deux tests de capacité, **pour deux raisons
différentes** — la liste des items est fausse *et* l'annonce n'a pas d'effet.
C'est ce qui confirme qu'ils ne se recouvrent pas.

Le troisième reproduit exactement le bug de §C.10 (les deux moitiés de l'ISV
bâties avec des épines différentes, 3,2 % d'écart, collier qui mord dans la
flèche) et le test le prend.

**Mesures** : 186 → **194 tests**, 0,08 s. 37 avertissements, inchangé —
`catalogue::TOUTES` (la liste des trois tables, qui n'existe que pour les
balayages de tests) est `#[cfg(test)]`, sans quoi elle en ajoutait un.

### L1.2 Balayage de toutes les variantes : santé de la géométrie (2026-07-31)

**Le problème** (`conception/assembleur.md` §5.5) : `dessiner()` n'était exercé
que pour 8 briques de classe C, et seulement en « ne panique pas ». Aucun test
ne balayait les 31 variantes. Or un `NaN` glissé dans une cote se propage
silencieusement jusqu'à faire **disparaître un lot entier** à l'écran —
macroquad ne dit rien — et un indice hors bornes est un plantage GPU, pas une
erreur Rust.

**La difficulté n'est pas d'écrire le balayage, c'est de garantir qu'il balaie
tout.** `Composant` n'a pas de `TOUS` : il faut construire un échantillon par
variante, et une `Vec` d'échantillons **se compile parfaitement en en oubliant
une** — le test passerait alors au vert en la ratant, ce qui est exactement le
mode de défaillance recensé en §C.29.

D'où le dispositif retenu : une **chaîne**, pas une liste.

```rust
fn suivante(c: &Composant) -> Option<Composant> {
    match c {
        Composant::ModuleAxial { .. } => Composant::Noeud { .. },
        // ... 31 bras, exhaustifs
        Composant::SousEnsemble { .. } => return None,
    }
}
```

Le `match` est exhaustif : **ajouter une variante à `Composant` casse la
compilation ici**, et la seule façon de réparer est de lui donner un échantillon
et de l'insérer dans la chaîne. La couverture n'est plus une discipline, c'est
une erreur de compilation.

**Trois tests ajoutés** (`composant/mod.rs`) :

| Test | Ce qu'il garde |
|---|---|
| `la_chaine_dechantillons_visite_chaque_variante_une_fois` | la chaîne est bien recousue : ni doublon, ni cycle, ni saut |
| `toute_variante_cuit_une_geometrie_saine` | positions finies, indices dans les bornes, triangles complets |
| `toute_variante_dessine_sauf_le_panache` | tout dessine — **sauf** le panache, et c'est exigé à l'endroit |

**Le verrou de valeur assumé.** `la_chaine…` contient un
`assert_eq!(ech.len(), 31)`, c'est-à-dire précisément ce que §2 famille D du
document de conception recommande d'éviter. Il est gardé, pour une raison qui
n'est pas datée : *le compilateur garantit que chaque variante a un bras, pas
qu'elle est **atteinte***. Un bras qui pointe par-dessus son voisin
(`Antenne => Caisson`, sautant `Adaptateur`) compile sans broncher et rend une
variante invisible aux deux autres tests. Rien d'autre que le compte ne le dit.
Qu'il rougisse à l'ajout d'une variante est **voulu** : le développeur est déjà
dans ce fichier (le `match` ne compile plus sans lui), et bumper le nombre est
la confirmation qu'il a recousu la chaîne au lieu de la court-circuiter.

**L'exception du panache est une assertion, pas une omission.** `Panache` ne
dessine rien : c'est une décision (§C.28 — un jet de plasma n'a pas de
silhouette, il est rendu en additif par `ecran::panache`). Écrire le test comme
« tout dessine sauf ceux de la liste d'exceptions » aurait fait de cette liste
une nouvelle source à tenir. Il est écrit à l'endroit : on **exige** que le
panache reste vide. Lui redonner de la géométrie le ferait dessiner deux fois,
en volume opaque par-dessus le ruban — l'aspect « tube de plastique » qui avait
justement été rejeté.

#### Red-check : 7 sur 7

| Sabotage | Rougit |
|---|---|
| `Motrice { echelle: f32::NAN }` | `…geometrie_saine` : « Motrice lot 0 sommet 0 : position non finie » |
| un lien qui saute `Adaptateur` | `la_chaine…` : « left: 30, right: 31 » |
| un lien qui repart en arrière (`Charniere => Noeud`) | `la_chaine…` : « la chaîne boucle » |
| `panache::dessiner` remis à dessiner un cône | `…sauf_le_panache` : « le panache ne doit rien dessiner » |
| `Charniere` qui cesse de dessiner | `…sauf_le_panache` : « Charniere ne dessine rien » |
| un indice `u16::MAX` poussé dans un lot | `…geometrie_saine` : « indice 65535 hors des 420 sommets » |
| un indice valide en trop (triangle incomplet) | `…geometrie_saine` : « triangles incomplets » |

Les deux derniers passent par `Batisseur::terminer`, sabotée le temps du
contrôle : c'est la seule façon d'atteindre ces deux assertions, aucune variante
ne produisant naturellement d'indice fautif.

**Mesures** : 194 → **197 tests**, 0,08 s. Aucune variante n'a échoué au
premier passage — les trois tests sont nés verts, ce qui est le résultat
attendu : ils ne corrigent rien, ils **empêchent**.

### L1.3 + L1.5 Deux assertions du générateur qui ne garantissaient rien (2026-07-31)

Traitées ensemble : même fichier, même famille de défaut — une assertion trop
lâche sous un nom qui promet beaucoup.

#### `generer_est_deterministe`

```rust
// avant
assert_eq!(nb(&generer(&p)), nb(&generer(&p)));   // le NOMBRE de pièces
```

Le déterminisme du générateur est ce sur quoi repose l'idée même de « graine »,
et bientôt la sauvegarde d'un assemblage (§6.4 : rejouer une liste de poses doit
reproduire la géométrie **au sommet près**). Ce test comparait deux compteurs :
deux stations entièrement différentes passaient au vert dès qu'elles avaient le
même nombre de pièces.

Réécrit du plus grossier au plus fin — nombre de pièces, puis chaque pièce
(composant **et** transformée), puis les sommets cuits — et balayé sur
4 graines × 3 styles × 4 complexités au lieu d'un seul jeu de paramètres.

**La démonstration.** On a saboté `Chantier::racine` pour qu'elle décale la
station d'un poil de plus à chaque appel (compteur atomique). Toute la station
suit, donc le **nombre de pièces ne bouge pas** : c'est un générateur qui rend
un résultat différent à chaque appel, ce que le test est censé interdire.

| Test | Sous ce sabotage |
|---|---|
| l'ancien (`nb == nb`) | **vert** ✅ — remonté et exécuté pour le vérifier, pas déduit |
| le nouveau | rouge : « graine 0 / HISTORIQUE / cplx 1 : pièce 0 mal placée » |

C'est le résultat qui justifie tout le lot : un test peut être vert pendant que
la propriété qu'il nomme est fausse de la façon la plus flagrante possible.

#### `complexite_influe_sur_le_nombre_de_pieces`

```rust
assert!(grande >= petite);   // l'égalité satisfait « influe »
```

Un test nommé « la complexité influe » que « la complexité n'influe pas »
satisfait. Resserré en `>`, et balayé sur 20 graines × 3 styles.

**Avant de figer le `>` strict**, on a mesuré s'il était tenable : 600
combinaisons (200 graines × 3 styles), **zéro contre-exemple**, écart le plus
serré **33 pièces**. Ce n'est donc pas une inégalité qu'on frôle — resserrer
n'introduit aucune fragilité.

**Red-check** : budget rendu insensible à la complexité (`* 250.0` → `* 0.0`) →
rouge, « graine 3 : cplx 4 → 15 pièces, cplx 1 → 17 ». Au passage, une
observation : sans le budget, la complexité *inverse* la tendance (elle décale
aussi le flux du RNG). Elle n'agit donc pas seulement par le nombre de pièces
qu'elle autorise.

**Mesures** : 197 tests, 0,08 s → **0,44 s** — les deux balayages (48 et 60
combinaisons, chacune générant deux stations complètes) sont désormais les
tests les plus lourds du projet. Reste négligeable.

### L1.4 Les rayons déclarés ne contiennent pas les pièces (2026-07-31)

**L'étape qui a trouvé quelque chose.** Les quatre autres n'ont fait que poser
des filets sur du code correct ; celle-ci a mis au jour un défaut franc et une
dette systémique.

#### Ce qu'on cherchait

Deux mesures d'encombrement coexistent, pour deux usages :

- `rayon_local()` → cadrage caméra (`Station::rayon` compose
  `centre().length() + rayon_local()`) ;
- `englobant_local()` → anti-collision de `Chantier::poser`, depuis **son
  propre centre**, qui peut être décalé (un propulseur, une coiffe se déploient
  d'un seul côté de leur montage).

Rien ne vérifiait qu'elles **contiennent** la pièce. Sous-estimer, c'est une
caméra qui coupe et une collision qui ment.

#### Ce qu'on a mesuré — et il fallait mesurer avant d'asserter

Balayage des 31 variantes (la chaîne de L1.2), hors-tout réel contre rayon
déclaré, **et** proportion de sommets qui sortent :

| Pièce | cadrage | collision | sommets dehors |
|---|---|---|---|
| `Charpente` | **×1,70** | ×1,70 | 44 % |
| `ChargeUtile` | ×0,57 | **×1,37** | 81 % |
| `TreillisHexagone` | ×1,28 | ×1,28 | 50 % |
| `Coiffe` | ×0,74 | ×1,25 | 30 % |
| `Antenne` | ×0,85 | ×1,20 | 16 % |
| `CollierRotatif` | ×1,09 | ×1,09 | 67 % |
| … | | | |

**20 variantes sur 30 débordent**, et pas d'un cheveu : 81 % des sommets d'une
`ChargeUtile` sortent de sa sphère de collision, 67 % de ceux d'un
`CollierRotatif` sortent des deux.

La lecture honnête : ces deux fonctions se sont écrites comme des **tailles
nominales** (« le gabarit de la pièce », ce qu'un catalogue afficherait) et non
comme des **volumes englobants**. Rien n'a jamais cassé parce que le générateur
s'accommode d'une pose refusée ou acceptée à tort — il réessaie ailleurs. Un
humain qui vient de cliquer, non (§5.3 du document de conception).

#### Le défaut franc : `Charpente` ignorait sa propre aiguille — corrigé

`Charpente { aiguille: true }` déclarait `longueur * 0.5` = 10,0 en s'étendant
à 17,0. L'anneau hexagonal et sa jupe pendent **sous** la base du cône, et le
calcul du rayon n'en tenait aucun compte.

Ce n'est pas un oubli anonyme : sa jumelle `CharpenteHexa` le fait depuis
toujours, et le commentaire le dit — « **tour du pied comprise** : elle pend
sous la base, donc c'est elle qui fixe l'extension quand l'aiguille est posée ».
Quelqu'un a vu le problème sur la variante hexagonale, l'a corrigé là, et n'est
pas revenu sur la carrée.

Corrigé en extrayant `treillis::charpente_pied(grand, aiguille) -> (axial,
radial)`, **lue par le dessin et par le rayon** — une seule source, comme
partout ailleurs dans ce lot. Aucune géométrie ne change : seule l'enveloppe
déclarée. Les 199 tests passent, y compris les 49 tests de décision de l'ISV.

#### Ce qui a été livré, et ce qui reste

Deux tests :

| Test | Ce qu'il garde |
|---|---|
| `les_rayons_declares_contiennent_la_piece` | les deux rayons contiennent la pièce, **à `MARGE_RAYON = 1,40` près** |
| `laiguille_de_la_charpente_compte_dans_son_rayon` | la carrée se compare à elle-même, armée vs nue — indépendant des cotes |

⚠️ **`MARGE_RAYON = 1,40` est une dette, pas une cote de conception**, et le
commentaire du code le dit. L'invariant juste est 1,0. La borne à 1,40 fait ce
qu'elle peut : elle empêche que ça **empire** (une nouvelle pièce déclarant la
moitié de sa taille serait prise) mais laisse passer les 19 débordements
actuels.

**Red-check : 2 sur 2.**

| Sabotage | Rougit |
|---|---|
| ancienne formule de `Charpente` remise | « Charpente : cadrage — hors-tout 16.99 pour un rayon_local de 10.00 (×1.70) » **et** le test de l'aiguille |
| `coiffe_englobant` divisé par deux | « Coiffe : collision — hors-tout 1.19 pour un englobant de 0.50 (×2.38) » |

Le second vérifie la branche **collision** séparément : `Charpente` échoue
d'abord sur le cadrage, et une seule sabotage n'aurait donc pas prouvé que la
seconde assertion fonctionne.

#### ❓ Question ouverte pour l'utilisateur

Aligner les 19 variantes restantes **n'est pas une retouche** : les formules
touchées reculent la caméra sur les vues concernées et resserrent la collision,
donc changent les stations générées. Deux lectures possibles, et le choix
appartient à l'utilisateur :

- **(A) ce sont des enveloppes** → corriger les ~19 formules, une par une,
  chacune vérifiée à l'écran. Chantier réel, mais l'assembleur hérite alors
  d'une collision qui ne ment pas ;
- **(B) ce sont des tailles nominales** → laisser les formules et appliquer une
  marge **au point d'usage** (`Chantier::collision`, `Station::rayon`). Un seul
  endroit à changer, rien ne bouge à l'écran, mais la marge est un facteur
  global là où les débordements vont de ×1,0 à ×1,37.

---

## Bilan du Lot 1 (2026-07-31)

| | avant | après |
|---|---:|---:|
| Tests | 186 | **199** |
| Durée (`cargo test --release`) | 0,07 s | 0,45 s |
| Avertissements | 37 | 37 |
| Fichiers sans aucun test | `ecran/` (2 500 lignes) | — |

**13 tests ajoutés, 2 réécrits, 1 défaut de code corrigé, 4 sources
dupliquées supprimées.**

Ce que le lot a réellement produit, au-delà du compteur :

1. **Une source par fait.** Les trois constantes d'indice du catalogue, le
   compte de briques, l'attrape-tout et le `match` d'épine ont disparu au profit
   d'une table où les capacités se **déduisent** de la fabrique. Rien à tenir
   d'accord, donc rien qui puisse diverger.
2. **La couverture des variantes est devenue une erreur de compilation.** La
   chaîne d'échantillons de L1.2 fait qu'ajouter une variante à `Composant`
   casse le build tant qu'elle n'a pas d'échantillon.
3. **Une preuve, pas une conviction.** L'ancien `generer_est_deterministe` a été
   remonté et exécuté sous un générateur volontairement non déterministe : il
   passait au vert. C'est la justification de tout le lot.
4. **Un défaut trouvé et corrigé** (`Charpente` ignorait son aiguille : ×1,70)
   et **une dette systémique mise au jour** (20 variantes sur 30 débordent leur
   rayon déclaré), dont l'arbitrage revient à l'utilisateur.

**Red-check : 17 sabotages, 17 rougissements attendus, aucun test passif.**
Chacun est consigné dans la section de son étape.

**Ce que le lot n'a pas fait**, volontairement : aucun test de rendu (couleurs,
disposition, position d'un panneau) et aucun nouveau verrou de valeur, sauf le
compte de la chaîne de L1.2 et `MARGE_RAYON`, tous deux justifiés sur place.

### L1.6 Les enveloppes de collision passent en capsules (2026-08-01)

**Décision utilisateur**, prise sur la mesure de L1.4 : les enveloppes changent
de **forme**, pas seulement de taille. Motif retenu — « on aura besoin d'ajouter
de nouveaux composants pour meubler l'assemblage, donc autant poser le socle
maintenant ».

#### Pourquoi la sphère ne pouvait pas rester

Une sphère est un bon englobant pour une pièce **ramassée**, et un très mauvais
pour une pièce **allongée**. Le radiateur méga fait 30 de long sur 8 de large :
la plus petite sphère qui le contient a un rayon de 15, donc réserve 15 de vide
sur ses flancs, là où la pièce n'a que 4 d'épaisseur. Le générateur s'en
accommodait — une pose refusée, il réessaie ailleurs. Un humain qui vient de
cliquer, non.

#### La primitive : capsule, et la sphère en est le cas dégénéré

`src/vaisseau/enveloppe.rs` — `Enveloppe { a, b, rayon }`, l'ensemble des points
à distance ≤ `rayon` du segment `[a, b]`. `a == b` donne une sphère.

Trois propriétés qui ont décidé du choix :

1. **Une seule primitive.** Pas d'énumération de formes, pas de branchement dans
   le test de collision, et une pièce compacte n'est pas pénalisée.
2. **Elle survit exactement aux transformées du projet.** Poses (rotation +
   translation) et symétries (réflexion) préservent les distances : on
   transforme les deux bouts et on **garde le rayon**. Une AABB ne survivrait pas
   à une rotation ; une OBB demanderait le théorème des axes séparateurs, quinze
   axes là où la capsule en demande zéro.
3. **Le critère de collision garde sa forme** — distance des *axes* contre somme
   des rayons, à `FACTEUR_COLLISION` près — donc la tolérance d'adjacence de
   docking conserve son sens.

Aucune capsule ne contient exactement une boîte : il faut choisir **où** elle
déborde. Réglage retenu (`Enveloppe::axe`) : l'axe couvre toute la longueur, le
rayon vaut la demi-épaisseur, et le débord est une calotte **au bout** — pas en
travers. L'autre réglage possible déborde moins en longueur mais 41 % plus large,
et les voisins se posent de côté, pas dans l'axe.

#### Le gain, mesuré

| Pièce | sphère | capsule | rapport |
|---|---:|---:|---:|
| `RadiateurMega` | r = 16,5 | r = 4,1 | **×4,0** |
| `Treillis` | r = 4,5 | r = 0,8 | **×5,6** |
| `CharpenteHexa` | r = 24,8 | r = 4,7 | **×5,3** |
| `Radiateur` | r = 2,3 | r = 0,5 | **×4,6** |

Converties : poutres, les deux charpentes, radiateurs, panneaux, caissons,
habitat, nacelles cargo, module d'équipage, bardage thermique. **Laissées en
sphère** : nœuds, antennes, coiffes, adaptateurs, réservoirs — et les **plaques
de bouclier**, qui sont des disques : une capsule couchée sur leur axe serait
pire, la sphère est leur englobant minimal.

#### Deux défauts que le changement a fait remonter

**1. Les paires miroir n'étaient jamais groupées.** `cle_surface` indexait les
faces ±Y/±Z sur `pos.x` **signé** : un port à x = +12,4 et son jumeau à −12,4
tombaient dans deux groupes distincts, servis indépendamment. Corrigé en `|x|`.

Le défaut était **latent depuis toujours** et invisible : avec des englobants
sphériques, les deux côtés étaient refusés *ensemble* par symétrie. Il a fallu
une collision plus fine pour qu'un seul côté passe et que ça se voie.

**2. Un contrôle de pose sans pose.** `Chantier::peut_poser(hote, comp, montage)`
— mêmes contrôles que `poser`, sans rien poser. Ajouté en essayant de rendre les
groupes atomiques ; conservé même après avoir renoncé à l'atomicité, parce que
c'est **exactement** ce que la palette de l'éditeur demandera (§6.5 : savoir si
un clic aboutirait avant de l'avoir fait). Lot 2 arrivé en avance.

#### L'atomicité des groupes : essayée, puis écartée

Une version atomique a été implémentée — vérifier que *tous* les membres d'un
groupe passent avant d'en poser un seul — au motif qu'une demi-paire donne une
station bancale. Elle tient la symétrie mais **perd des pièces** : un groupe
entier saute dès qu'un seul de ses ports est encombré, et une station de 16
modules retombait à 6 radiateurs au lieu de 8.

**Arbitrage rendu (utilisateur, 2026-08-01) : on ne cherche pas la symétrie
partout.** Elle vaut pour la barre de l'ISS (bâbord/tribord) ; sur la coque, des
équipements dépareillés sont la règle sur les vraies stations. La grammaire sert
donc ce qui tient.

#### Tests

11 tests dans `enveloppe.rs`, dont un **contrôle en force brute** : la distance
segment↔segment exacte doit valoir le minimum échantillonné sur 400 × 400 points,
sur quatre configurations (croisées, sécantes, parallèles superposées, parallèles
disjointes). Écrit exprès de façon indépendante pour ne pas récrire la formule
testée — et il a pris deux erreurs d'arithmétique **dans les tests eux-mêmes**
avant de valider le code.

Dans `chantier.rs`, la paire qui dit le changement :

| Test | Ce qu'il garde |
|---|---|
| `collision_rejette_recouvrement` | deux ailes larges à 90° se recouvrent **vraiment** près de la racine → refusées |
| `deux_ailes_perpendiculaires_ne_se_genent_pas` | deux ailes **minces** à 90° ne se touchent pas → acceptées |

⚠️ Le premier testait autrefois des ailes minces perpendiculaires et attendait un
refus. C'était un **faux positif**, et c'est précisément ce refus injustifié qui
a motivé tout ce chantier. Le second existe pour que la correction soit gardée
explicitement, et pas seulement constatée.

#### Dette laissée

Le plancher de proportion des radiateurs de `silhouette_generee_converge` est
passé de 0,40 à 0,35. Le regroupement par `|x|` divise par deux le nombre de
tirages de `fabrique_appendice`, donc **décale le flux du RNG** : toutes les
stations changent sans que la grammaire ait empiré. Mesuré sur les 72
combinaisons du test : **un seul cas** passe sous 0,40 (Mir c=2 graine 4, à
0,357). Le vrai ISS est à 0,88 — un plancher à 0,35 attrape toujours une station
qui ne refroidit visiblement rien.

**Mesures** : 199 → **211 tests**, 0,78 s. 37 → **39 avertissements** : les
méthodes d'`Enveloppe` et `peut_poser` ne servent pour l'instant qu'aux tests
(elles serviront à l'overlay de débogage et à la palette).

### L1.7 Overlay : enveloppes (E) et fils numérotés (F) (2026-08-01)

Conception préalable : `conception/assembleur.md` §8 — l'écran d'assemblage
entier, arrêté avant d'écrire une ligne, sur deux choix rendus par l'utilisateur
(**pièce d'abord** façon KSP, **bac à sable** sans plafond de coût).

**Deux bascules, deux questions différentes.**

`E` — les enveloppes de collision en fil de fer. Il ne s'agit pas de « voir les
capsules » : l'overlay répond à *pourquoi ce port refuse-t-il ma pièce ?* Un
port rouge sans explication est un bug du point de vue de l'utilisateur, même
quand le refus est correct. D'où `conflit()`, qui trace le **segment de plus
courte approche** entre la pièce proposée et ce qui la refuse.

⚠️ `FACTEUR_COLLISION` est devenu `pub(crate)` : l'overlay doit appliquer le
critère **exact** du modèle. Avec sa propre marge, il expliquerait un conflit que
`Chantier` n'a jamais prononcé — pire que pas d'overlay.

`F` — chaque fil de charpente porte son numéro, **posé dans une coupure du
fil**. Un chiffre flottant près d'un enchevêtrement de barres n'appartient
visiblement à aucune ; le trait est donc coupé et le chiffre occupe le trou,
comme une cote en dessin technique et pour la même raison.

**Le relevé est une troisième sortie de `Peintre`** (`vaisseau::inventaire`),
aux côtés d'`Immediat` et de `Batisseur`. Conséquence : les numéros **ne peuvent
pas** désigner une autre géométrie que celle affichée, puisqu'ils sortent du même
code. Une table écrite à la main aurait dérivé à la première retouche — comme les
quatre sources du catalogue de briques (§5.1).

### L1.8 Mesureur automatique : trancher les fils, pas le maillage (2026-08-01)

**Idée de l'utilisateur**, et elle règle le vrai goulot : jusqu'ici chaque
composant se jugeait à l'œil, un par un.

#### Le point de conception

Le mesureur tranche les **fils**, jamais le maillage cuit. C'est la correction
d'une erreur commise **trois fois** dans ce projet (§C.13, §C.29) : un maillage
cuit n'a de sommets qu'aux frontières de facettes, donc une tranche prise à
mi-portée d'un cylindre revient **vide** et la mesure lit zéro sans rien
signaler.

Un fil est un segment analytique doté d'un rayon : échantillonner **le long d'un
segment droit** est exact, là qu'échantillonner un maillage facetté ne l'est pas.
Le test `aucune_tranche_dun_cylindre_plein_nest_vide` garde précisément ça.

#### Ce qu'il rend, par composant

| Mesure | Ce qu'elle décide |
|---|---|
| profil tranché (silhouette ASCII) | où la pièce enfle ou se creuse |
| **serrage** = besoin / déclaré | `> 1` l'enveloppe ne contient pas la pièce ; `< 1` elle réserve du vide |
| **élancement** = long / diamètre | au-dessus de ~1,5, la capsule s'impose sur la sphère (L1.6) |

Tout est versé dans **`docs/reference/fils.md`**, régénéré et comparé par test :
31 composants, 1080 fils, et un relevé par pièce. Un composant *futur* est mesuré
sans qu'on y pense — c'est là qu'est l'accélération.

#### ⚠️ Ce que le relevé ne prouve pas encore

Le premier passage rend **22 `DEBORDE` sur 30**, ce qui ne veut pas dire 22
défauts. Un `Fil` ne porte **qu'un** rayon pour tout son segment : sur un cône
c'est le plus grand des deux bouts appliqué sur toute la longueur, sur une caisse
la demi-diagonale de sa section. Le serrage est donc **pessimiste** sur les
pièces faites de cônes et de caisses.

Ce qui reste fiable, et qui est déjà exploitable :

- le **classement** — qui déborde le plus ;
- les cas **`lache`** (`Radiateur` 0,62, `ModuleHabitat` 0,61) : aucune
  approximation ne rend une enveloppe *trop grande*, donc ces deux-là réservent
  vraiment du vide ;
- les pièces faites de **cylindres seuls**, où le rayon est exact — `Treillis`
  0,98, `ModuleAxial` 0,97, `RadiateurMega` 0,96 : les capsules de L1.6 sont bien
  ajustées, ce qu'aucun test ne disait jusqu'ici.

**Fait le 2026-08-01** : le `Fil` porte un rayon **par extrémité**, le cône est
donc exact. Deux autres corrections ont été **essayées puis annulées** — et
l'annulation est le résultat intéressant :

| Tentative | Pourquoi annulée |
|---|---|
| voile couverte par une capsule médiane au lieu de sa diagonale | fait bondir le besoin du `RadiateurMega` de ×1,07 à ×1,72 |
| maille brute créditée du rayon de son sommet le plus éloigné | idem, et pour une raison identifiée : **double-comptage** (un point pris au milieu de la diagonale se voit prêter la distance d'un coin) |

Les deux contredisaient `les_rayons_declares_contiennent_la_piece`, qui mesure la
même grandeur sur les sommets cuits et reste vert. **Deux mesures de la même
chose qui se contredisent, c'est le défaut que ce chantier traque** (§C.29) :
tant qu'on ne sait pas laquelle a tort, on garde la version conservatrice.

Ce que l'épisode a appris : le serrage est une **borne supérieure**, pas une
cote. Il vaut `distance à l'axe de l'enveloppe + rayon du fil`, exact pour un
cylindre aligné, majorant dès que le fil est de biais. La mesure exacte de
contenance reste le test sur sommets cuits ; les deux se complètent — le test dit
*si* ça déborde, le relevé dit *où* et *de combien au plus*.

**Reste à trancher** : une plaque plate n'est pas une capsule, et aucun couple
(axe, rayon) ne la borne sans gaspiller. Il faudrait un genre de fil de plus.

**Mesures** : 214 → **223 tests**, 0,71 s.

### §9 Le boudin : l'enveloppe rectangle pour les plaques (2026-08-01)

**Idée de l'utilisateur**, notée en conception le jour même : le point resté
ouvert en L1.8 (« une plaque plate n'est ni une sphère ni une capsule »)
demande un troisième noyau, pas un troisième cas particulier.

#### Le principe

`Enveloppe` portait implicitement un noyau **segment** (`a`, `b`, `a == b`
pour un point). Il porte maintenant un `Noyau` explicite à deux variantes,
`Segment` et `Rectangle { centre, eu, ev, hu, hv }` — le rectangle gonflé
d'un rayon, pour les pièces plates. La formule de collision ne change pas :
`Enveloppe::ecart` reste « distance des noyaux moins somme des rayons »,
quelle que soit la forme des deux noyaux en présence
(`conception/assembleur.md` §9.3).

Ce qu'il a fallu écrire, dans `src/vaisseau/enveloppe.rs` :

- `distance_point_rectangle` — trivial, la distance à une boîte est séparable
  par axe.
- `distance_segment_rectangle` — exact par la même famille d'idée que
  `distance_segments` : projetées dans le plan du rectangle, les coordonnées
  `(s, t)` du point courant du segment et son décalage hors-plan `w` sont
  toutes trois **affines** en le paramètre du segment, donc le carré de la
  distance est une quadratique **par morceaux** (les morceaux se recollent
  aux franchissements de bordure), et chaque morceau a un minimum en forme
  close.
- `distance_rectangles` — le point délicat, tranché avec l'utilisateur
  (« on doit prendre en compte l'orientation libre, et le général exact nous
  permet de limiter la dette, propose des solutions hybrides »). L'énumération
  naïve (les 16 paires bord↔bord) rate deux cas réels : une plaque plus
  petite imbriquée dans une plus grande en plans parallèles (aucun bord ne
  croise l'autre), et deux plaques qui se transpercent obliquement sans que
  leurs bords ne se croisent. La solution retenue exploite le théorème de
  l'hyperplan séparateur : pour deux convexes disjoints, le point le plus
  proche d'*au moins un* des deux rectangles est sur son bord. Balayer les
  quatre bords de chaque rectangle contre la face **entière** de l'autre (via
  `distance_segment_rectangle`, qui résout déjà « point de ce segment contre
  tout le rectangle », intérieur compris) couvre donc tous les cas — y compris
  les deux ratés par l'énumération naïve — sans test d'intersection séparé.
  Huit appels, aucun cas particulier.

#### Rectifié en cours de route par le mesureur

`petit_englobant`/`grand_englobant` (boucliers) construisaient d'abord le
boudin avec `demi_epaisseur(rayon)` (l'altitude des deux **peaux**, 0,015 ×
rayon) comme demi-épaisseur — repris tel quel de l'ancien commentaire, sans
vérifier. Erreur : les deux ports de la plaque sont aux **bouts du moyeu**
(`rayon × MOYEU_DEMI`, 0,09 × rayon), pas sur la peau — c'est lui qui domine
l'étendue axiale réelle, six fois plus loin. Une plaque avec ce réglage aurait
reproduit exactement le défaut de L1.4 (l'enveloppe ne contient pas la pièce),
cette fois sur la pièce même qui motive le chantier.

Corrigé en dérivant l'épaisseur du boudin des **mêmes constantes** que le
dessin (`MOYEU_DEMI`, et pour le petit bouclier `PEAU` + `NERVURE`, dont la
racine de la nervure principale dépasse légèrement le moyeu côté arrière,
son ossature étant décalée en Z) — une source par fait, comme `charpente_pied`
en L1.4. Les demi-étendues en plan (`hu`, `hv`) sont lues sur le **contour déjà
construit** (`mesure_xy`, le pendant en rectangle de `mesure`), pas
recalculées à la main : la grande plaque, étirée, gagne un vrai rectangle et
non plus un carré isotrope — le boudin cesse de gaspiller en largeur ce qu'il
gagne en épaisseur.

#### Ce que ça change sur l'ISV

| Pièce | sphère (avant) | boudin (après) |
|---|---:|---:|
| `BouclierGrand` (rayon 12, élancement 1,3) | rayon ≈ 12 dans **toutes** les directions | ≈ 0,7 × 1,3 en plan, ≈ 0,10 en épaisseur |

C'est le facteur qui décide si l'empilement à quatre plaques sur le mât de
l'ISV tient sous une collision honnête — le test
`le_boudin_serre_une_plaque_bien_mieux_quune_sphere` le chiffre : la sphère
équivalente réserve plus de dix fois plus de rayon en épaisseur.

#### Consommateurs adaptés

- `ecran/enveloppes.rs` : `fil()` dispatché en deux tracés (`fil_capsule`,
  inchangé, et `fil_boudin`, un pavé aplati — pas les coins arrondis exacts
  du vrai boudin, un repère de débogage n'a pas besoin de l'exactitude que la
  collision exige). `plus_proches` généralisé à un paramètre `(u, v)` au lieu
  d'un seul `t` : convexe côté par côté sur les deux noyaux, la descente à pas
  décroissant tient sans changer de nature.
- `vaisseau/mesure.rs` : l'axe de tranchage d'une plaque devient sa
  **normale** (l'épaisseur, seule direction à mesurer), et le rayon « radial »
  se lit contre le noyau réel (`distance_point_rectangle`) au lieu du seul
  segment — ce module n'avait jamais eu que deux formes à connaître, il en a
  maintenant trois par la même voie.
- `composant/mod.rs`, `inventaire.rs`, `chantier.rs` : inchangés dans leur
  logique — ils passaient déjà par les méthodes de `Enveloppe`
  (`distance_axes`, `ecart`, `profondeur`, `contient`), jamais par les champs
  `a`/`b` à nu, sauf les deux tests qui les lisaient directement et qui ont dû
  être réécrits contre `Noyau`.

#### Red-check : 5 sabotages, 5 rougissements

| Sabotage | Rougit |
|---|---|
| second balayage de `distance_rectangles` désactivé (retour à l'énumération naïve bord↔bord) | `distance_rectangles_vaut_le_minimum_echantillonne` et `une_petite_plaque_imbriquee_dans_une_grande_est_a_distance_nulle` |
| `branche()` ignore les bornes de la boîte (le sommet de la quadratique par morceaux dégénère en repli sur le bas du morceau) | rien, au premier essai — les six cas choisis avaient tous leur vrai minimum sur une bordure de morceau, par construction. Cas ajouté exprès (minimum au bout du segment, dans une zone où l'excès est actif de bout en bout) : rougit alors, exact 5,74 contre 1,73 attendu — gardé en permanence, il comble un vrai trou de couverture |
| épaisseur du boudin de la grande plaque ramenée à `demi_epaisseur` (peau) au lieu de `MOYEU_DEMI` (moyeu) | `les_rayons_declares_contiennent_la_piece` et `une_plaque_de_bouclier_reste_mince_et_centree` |
| `transformee` d'un rectangle transforme `eu` comme un point (`transform_point3`) au lieu d'un vecteur | `une_transformee_rigide_deplace_le_boudin_sans_le_deformer` |
| `JEU_BOUDIN` ramené à 0 | rien — confirme que la borne analytique (`MOYEU_DEMI`, `debord_nervure`) est déjà juste à l'égalité près, le jeu est une marge de sécurité et non une béquille |

Le deuxième sabotage est le plus instructif : il a fallu **ajouter** un cas
adversarial au test en force brute existant pour qu'il morde, alors que
l'algorithme était réellement faux sous ce sabotage. Les six cas d'origine
avaient tous leur minimum sur une bordure de morceau — un angle mort du choix
des cas, pas de la méthode.

**Mesures** : 223 → **233 tests**, ~1,1 s. 42 → **45 avertissements** — les
méthodes du noyau rectangle ne servent pour l'instant qu'aux tests, même
constat qu'en L1.6 pour la capsule.

### L2.1 Identifiants stables de ports libres (2026-08-01)

**Premier pas du Lot 2**, et un préalable délibéré : `conception/assembleur.md`
§7 le place en tête parce que le rétrofit après coup toucherait chaque
appelant — ce qui s'est confirmé (30 sites touchés, tous dans le générateur).

#### Le défaut (§6.1)

`Chantier::poser` prenait un **indice** dans `libres()` et consommait le port
par `swap_remove`, qui **déplace un élément quelconque** sur la position
libérée. Sans conséquence pour le générateur — il choisit un port et pose
aussitôt, dans la même expression, jamais à cheval sur deux appels. Pour une
UI qui tient un port survolé d'une image à l'autre : rédhibitoire. Le
symptôme est une pose au mauvais endroit, **intermittente**, la pire
catégorie de bug à diagnostiquer.

#### La correction : un compteur monotone, jamais un recalcul

`PortLibre` porte désormais un `id: u64`, distribué par `Chantier::prochain_id`
— un compteur qui ne décroît ni ne se remet à zéro. `poser`/`peut_poser`
prennent cet `id` (plus un indice) ; en interne, `Chantier::trouver(id)` fait
la traduction vers la position actuelle avant le `swap_remove`, qui reste
inchangé — c'est sa **visibilité aux appelants** qui disparaît, pas le
mécanisme lui-même. `compatibles` rend désormais des `id`, pas des positions.

#### Le vrai travail : les 30 appelants du générateur

`chantier.rs` a changé en une heure ; l'essentiel du chantier a été de
retrouver, dans `generateur.rs` et `montage.rs`, chaque `.position(...)` qui
nourrissait un `poser` et de le remplacer par un `.find(...).map(|p| p.id)`.
Le compilateur a servi de check-list exhaustive : après le changement de
signature, les 26 erreurs de type ont désigné **chaque** site un par un — plus
sûr qu'un grep, qui aurait pu en manquer un (un des pièges du catalogue
`suivi/stations.md` §C.29).

Trois fonctions nommées ont changé de type de retour (`Option<usize>` →
`Option<u64>`) : `port_vers`, `index_port`, `port_le_plus_haut`. Leur
commentaire documentait déjà, à la main, la même prudence que l'`id` rend
maintenant automatique — `index_port` par exemple : « les index se décalent à
chaque pose, on ne peut pas les mémoriser d'une itération sur l'autre ». Le
générateur avait donc **déjà** le bon réflexe (ne jamais garder une position
d'un appel à l'autre, toujours re-résoudre par contenu — genre, origine,
position monde) ; L2.1 rend ce réflexe inutile plutôt que de le remplacer.

#### Les trois tests demandés par la conception, et deux qui ont dû être renforcés

Écrits dans l'ordre du §6.1 :

| Test | Ce qu'il garde |
|---|---|
| `lidentifiant_dun_port_libre_survit_a_la_pose_ailleurs` | un `id` désigne le même port après un `swap_remove` qui l'a fait migrer de position |
| `lidentifiant_dun_port_consomme_ne_se_recycle_jamais` | tous les `id` libres restent deux à deux distincts, à chaque étape d'une longue séquence de poses |
| `poser_sur_un_identifiant_perime_echoue_proprement` | un `id` déjà consommé est refusé — sans panique, sans toucher `pieces`/`libres` |

**Deux ont raté leur premier red-check**, et c'est le résultat le plus utile
de l'étape :

- `lidentifiant_dun_port_libre_survit_a_la_pose_ailleurs` : la première version
  se contentait de relire `ch.libres()` après coup pour vérifier que l'`id`
  suivi y figurait encore — ce qui inspecte la **structure de données**, pas
  le **chemin de code** que `poser` emprunte réellement. Sous un sabotage qui
  fait interpréter l'`id` comme une position brute (le bug d'origine, tel
  quel), le test restait vert : `trouver` n'était jamais appelé par
  l'assertion. Corrigé en **reposant réellement** sur l'`id` suivi et en
  comparant la transformée obtenue à celle attendue (via `accoupler`) — cette
  fois le sabotage rougit.
- `poser_sur_un_identifiant_perime_echoue_proprement` : la première version
  retentait la pose avec le **même composant** que la pose d'origine. Sous un
  sabotage qui replie `trouver` sur la position 0 quand l'`id` est introuvable
  (au lieu de `None`), le port replié s'est trouvé être d'un genre
  incompatible avec ce composant précis — le test restait vert **pour la
  mauvaise raison** (rejet par incompatibilité de genre, pas par `id`
  introuvable). Corrigé en sondant le port réellement présent en position 0 et
  en choisissant un composant compatible **avec lui** : un repli buggé
  réussirait alors pour de vrai, et se trahirait par les assertions sur
  `nb_pieces`/`libres().len()`.

Le troisième défaut (`lidentifiant_dun_port_consomme_ne_se_recycle_jamais`,
version initiale) traquait un seul `id` choisi au hasard plutôt que
l'invariant général — un recyclage qui serait tombé sur une **autre** valeur
serait passé inaperçu. Réécrit pour vérifier l'unicité de **tous** les `id`
libres à chaque étape, ce qui l'a fait rougir du premier coup sous le
sabotage correspondant (compteur dérivé de `libres.len()` au lieu d'un
compteur dédié).

**La leçon, la même qu'en §9** : un test qui inspecte l'état plutôt que
d'emprunter le chemin réel, ou qui fixe arbitrairement une des variables d'un
scénario, peut rester vert sous un sabotage qui casse exactement ce qu'il
prétend garder. Le red-check n'est utile que si on répare le test dès qu'il
ne rougit pas — pas si on l'admet comme « suffisamment proche ».

#### Red-check final : 5 sabotages, 5 rougissements

| Sabotage | Rougit |
|---|---|
| `trouver` interprète l'`id` comme une position brute | `lidentifiant_dun_port_libre_survit_a_la_pose_ailleurs` |
| id distribué comme `libres.len()` (recyclable) au lieu du compteur dédié | `lidentifiant_dun_port_consomme_ne_se_recycle_jamais` |
| `trouver` replie sur la position 0 quand l'`id` est introuvable | `poser_sur_un_identifiant_perime_echoue_proprement`, et `poser_refuse_hote_hors_bornes` (existant) |

Les trois autres sabotages listés ci-dessus correspondent aux versions
initiales, plus faibles, des deux premiers tests — comptés une fois chacun
dans les « 5 » (2 ratés + corrigés, 1 bon du premier coup, plus le troisième
sabotage sur `poser_sur_un_identifiant_perime_echoue_proprement`, raté puis
corrigé).

#### Ce qui n'a pas bougé

`Chantier::peut_poser` a changé de signature (même `trouver` en interne) mais
n'a **aucun test dédié**, avant comme après — dette préexistante (notée en
L1.6 : « ne sert pour l'instant qu'aux tests… de la palette », qui n'existe
pas encore). Son chemin de lookup est le même que `poser`, donc couvert
indirectement par les red-checks ci-dessus, mais pas par un test qui
l'exercerait lui, nommément. À reprendre quand L2.4 (palette) lui donnera un
vrai consommateur.

**Mesures** : 233 → **236 tests**, ~0,8 s. 45 avertissements, inchangé.

### L2.2 `retirer` et la propriété d'aller-retour (2026-08-01)

`poser` existait, pas son inverse. §6.2 posait trois questions (le sous-arbre,
le port hôte, le budget) et l'invariant qui les résume : poser puis retirer
doit ramener le chantier à un état indiscernable de l'état initial.

#### Le vrai chantier n'était pas `retirer` lui-même, mais ce qu'il révèle

`PortLibre::origine` était un indice brut dans `pieces`. Tant que `pieces` ne
pouvait que grandir (aucun retrait), cet indice restait valide indéfiniment.
`retirer` fait disparaître des pièces **au milieu de la séquence** — le même
problème que L2.1 avait résolu pour les ports libres, mais pour les pièces
cette fois, et il se serait propagé silencieusement si rien n'avait changé :
un `Vec::retain` déplace toutes les pièces suivantes vers une position
différente, et chaque `PortLibre::origine` qui les désignait se serait mis à
pointer sur la mauvaise pièce.

**Corrigé avant d'écrire `retirer`, pas après** : `PortLibre::origine` devient
un `id` stable (`u64`), comme `PortLibre::id` en L2.1. Chaque pièce du
chantier (`Entree { id, piece, hote }`, privé — `Piece` publié par `Station`
reste inchangé) porte en plus son propre `id` stable et le `PortLibre` **exact**
qu'elle a consommé en se posant (`hote: Option<PortLibre>`, `None` pour la
racine) — c'est cette valeur, gardée verbatim, que `retirer` re-publie dans
`libres` (avec un `id` neuf) quand la pièce disparaît.

Conséquence en cascade, comme en L2.1 : `Chantier::piece` prend un `id` au
lieu d'un indice, et ~15 sites dans `generateur.rs` qui comparaient des
`PortLibre::origine` à un indice de pièce sont passés en `u64`. Un site valait
la peine d'être corrigé pour de bon plutôt que retypé : `compte()` balayait
`(0..ch.nb_pieces())` puis appelait `ch.piece(i)` pour chaque `i` — une
hypothèse de **densité** (aucun trou dans les indices) restée vraie tant que
rien n'était jamais retiré, et qui serait devenue silencieusement fausse dès
qu'un `retirer` aurait laissé des trous. Remplacé par `Chantier::pieces()`,
un itérateur sur les pièces **vivantes** — la bonne primitive existait déjà
pour `retirer`, `compte()` n'avait qu'à s'en servir.

Autre piège du même genre, désamorcé avant qu'il morde : plusieurs endroits du
générateur retrouvaient « la pièce que je viens de poser » via
`ch.nb_pieces() - 1`. Numériquement encore juste tant qu'aucun `retirer`
n'était appelé (les deux compteurs avancent ensemble), mais une coïncidence,
pas une garantie — et silencieuse le jour où elle cesse de tenir. Remplacé
par `Chantier::derniere_piece() -> Option<u64>`, qui lit directement la
dernière entrée plutôt que de déduire son numéro.

#### `retirer` lui-même

Le sous-arbre se calcule en **une seule passe** : `pieces` reste dans l'ordre
de pose (`retain` ne réordonne jamais), donc l'hôte d'une pièce y figure
toujours avant elle — pas besoin d'un point fixe, une pièce rejoint
l'ensemble à retirer dès que son `hote.origine` y est déjà. Le reste suit
dans l'ordre : rembourser (`Budget::rembourser`, nouveau, symétrique de
`depenser`), retirer les pièces du sous-arbre, retirer les ports qu'elles
exposaient, restituer le port hôte sous un `id` neuf.

#### Les trois tests demandés, plus la propriété qui les résume

| Test | Ce qu'il garde |
|---|---|
| `retirer_une_branche_emporte_ses_enfants` | racine → A → B, `retirer(A)` fait disparaître A **et** B |
| `retirer_libere_le_port_qui_portait_la_piece` | même géométrie qu'avant, `id` neuf — l'ancien reste périmé (§6.1) |
| `retirer_rembourse_exactement_le_cout_pose` | le remboursement couvre A **et** B, pas seulement la pièce nommée |
| `poser_puis_retirer_ramene_a_un_etat_indiscernable` | même nombre de pièces, même budget, mêmes ports libres (en multiset, `id` mis à part) |

Contrairement à L2.1, les quatre sont passés au vert **du premier coup** —
attribuable à la méthode qui avait fait défaut la fois précédente : chaque
test repose sur l'API réelle (`retirer`, `piece`, `libres`) plutôt que
d'inspecter une structure interne, et sonde l'état effectivement présent
plutôt que d'en fixer une partie à l'avance.

**Le test qui a le mieux gagné sa place** : le red-check a sabotagé le
nettoyage des ports orphelins (les ports exposés par le sous-arbre retiré,
laissés dans `libres` au lieu d'être supprimés). Aucun des trois tests nommés
ne l'a détecté — chacun ne regarde qu'une facette (le compte de pièces, le
port hôte précis, le budget). Seule
`poser_puis_retirer_ramene_a_un_etat_indiscernable` a rougi. C'est
exactement ce que §6.2 annonçait : *« elle vaut plus que les trois tests
séparés, parce qu'elle reste vraie quand on ajoute un champ que les trois
autres ignoreraient »* — ici, ce n'est pas un champ ajouté après coup mais un
défaut du premier jet, mais le mécanisme qui l'a pris est le même.

#### Red-check : 6 sabotages, 6 rougissements

| Sabotage | Rougit |
|---|---|
| pas de propagation au sous-arbre | `retirer_une_branche_emporte_ses_enfants`, `retirer_rembourse_exactement_le_cout_pose` |
| port hôte jamais restitué | `retirer_libere_le_port_qui_portait_la_piece`, `poser_puis_retirer_ramene_a_un_etat_indiscernable` |
| pas de remboursement | `retirer_rembourse_exactement_le_cout_pose`, `poser_puis_retirer_ramene_a_un_etat_indiscernable` |
| ports orphelins du sous-arbre non nettoyés | `poser_puis_retirer_ramene_a_un_etat_indiscernable` **seul** |
| repli sur la première pièce si l'`id` est inconnu | `retirer_sur_un_identifiant_inconnu_echoue_proprement` |
| port restitué avec l'**ancien** `id` (recyclé) au lieu d'un neuf | `retirer_libere_le_port_qui_portait_la_piece` **seul** — le round-trip, lui, ignore l'`id` par construction et reste vert à raison |

Le dernier sabotage confirme un point de conception plutôt qu'un bug : la
propriété d'aller-retour compare volontairement les ports libres **en
ignorant l'`id`** (§6.1 : un `id` consommé n'est jamais recyclé, donc l'état
« après » ne peut littéralement pas avoir le même `id` que l'état « avant »).
C'est pour ça qu'un test séparé et nommé reste nécessaire pour cet invariant
précis — la propriété générale ne peut pas, par construction, le couvrir.

#### Ce qui n'a pas bougé

`Chantier::peut_poser` reste sans test dédié (dette relevée en L2.1,
inchangée). `Chantier::figer` continue de fonctionner à travers `retirer` :
un chantier partiellement démantelé peut toujours se geler en `SousEnsemble`
— aucun test ne l'exerçait avant, aucun ne l'exerce après, et ce n'était pas
l'objet de cette étape.

**Mesures** : 236 → **241 tests**, ~0,9 s. 45 → **46 avertissements** —
`retirer` rejoint `peut_poser`/`compatibles`/`figer` dans le même constat
qu'en L1.6 et L2.1 : sans consommateur avant L2.3/L2.4.

### L2.3 Annuler/refaire, et un trou trouvé dans le test qui les garde (2026-08-01)

§6.3 pose une propriété à l'échelle de la session : *n* opérations puis *n*
annulations ramènent à l'état initial, *n* refaits ramènent à l'état d'après
les *n* opérations. Et une consigne de méthode : l'exercer par une séquence
**pseudo-aléatoire rejouable par graine**, pas des cas choisis à la main —
« c'est précisément ce qui a laissé passer les six mesures fausses de l'ISV
(§3) ».

#### Pourquoi annuler `retirer` ne s'inverse pas « à la main »

`poser` s'annule par un `retirer`, mais l'inverse ne tient pas : `retirer`
peut faire disparaître tout un sous-arbre (L2.2), et le reposer ne
recréerait ni les mêmes pièces ni la même arborescence — sans compter que
l'anti-collision ou le budget pourraient refuser la deuxième fois ce qu'ils
avaient accepté la première. Plutôt que d'inverser chaque opération une par
une, `Chantier` capture un **instantané complet** (`Instantane`, la même
forme que l'état mutable de `Chantier` lui-même) juste avant chaque
mutation, et `annuler`/`refaire` ne font que le restituer tel quel — deux
piles (`pile_annuler`, `pile_refaire`), aucune logique d'inversion à écrire
ni à faire confiance.

**Opt-in** (`Chantier::avec_historique()`), pas permanent : le générateur
pose des centaines de fois par station et n'annule jamais rien, or chaque
capture clone `pieces` et `libres`. Sans l'opt-in, il paierait cette copie à
chaque pose pour un historique dont il ne se sert jamais. Le point de capture
est aussi choisi tard exprès — juste avant la **première** mutation d'une
opération, une fois tous les contrôles de refus déjà passés — pour ne pas
cloner un état qu'une pose encore susceptible d'échouer (budget insuffisant)
jetterait de toute façon.

#### Le test a d'abord semblé bon, et ne l'était pas

Premier jet : une empreinte de l'état avant les *n* opérations, une autre
après, *n* annulations puis comparaison à la première, *n* refaits puis
comparaison à la seconde. Passait au vert du premier coup — et un red-check
délibéré sur le point de capture de `retirer` (l'instantané pris **après**
les mutations au lieu d'avant) est resté vert lui aussi, sur les 30 graines.

**La raison, une fois trouvée, est structurelle et pas anecdotique** :
`restaurer` **écrase** tout l'état en un bloc, il n'applique pas une
différence. Après *n* annulations, l'état final ne dépend donc que du
**dernier** dépilement (le tout premier `avant` empilé, forcément une pose —
`retirer` ne peut pas être la toute première opération, il n'y a encore que
la racine) ; chaque dépilement intermédiaire écrase le précédent sans que sa
propre exactitude compte pour le résultat final. Un instantané corrompu au
milieu de la pile est donc invisible à un test qui ne regarde que les deux
bouts de la séquence — il faut vérifier **chaque étape**, pas seulement le
départ et l'arrivée.

Corrigé en gardant une empreinte après **chaque** opération réussie
(`etapes: Vec<Empreinte>`), puis en comparant l'état à l'empreinte
correspondante après **chaque** annulation et **chaque** refait, pas
seulement à la fin. Sous ce test renforcé, le même sabotage rougit
immédiatement — à la 16ᵉ annulation sur la graine 0, un budget et une pièce
qui n'auraient pas dû être là.

Le parallèle avec §9 et L2.1 est le même à chaque fois, sous une forme
différente : un test qui semble exercer une propriété générale peut la
rater par la façon dont il est construit, pas par ce qu'il prétend vérifier.
Ici, ce n'était pas un cas mal choisi (§9) ni une variable figée (L2.1),
mais une granularité d'observation trop grossière pour ce que `restaurer`
rend structurellement invisible ailleurs.

#### Les tests

| Test | Ce qu'il garde |
|---|---|
| `annuler_defait_la_derniere_pose` / `refaire_refait_la_pose_defaite` | mécanique de base, budget actif (sans lui, un `restaurer` qui oublierait le budget passerait inaperçu — encore un enseignement du red-check, voir plus bas) |
| `annuler_et_refaire_sans_historique_echouent_toujours` | sans `avec_historique()`, toujours `false`, jamais de panique |
| `une_nouvelle_operation_invalide_la_pile_de_refaire` | invariant classique d'un historique annuler/refaire, non listé par §6.3 mais indispensable |
| `n_operations_puis_n_annulations_ramenent_a_letat_initial` | la propriété elle-même, 30 graines, granularité par étape |

#### Red-check : 5 sabotages, 5 rougissements (dont un a fait muscler deux tests)

| Sabotage | Rougit |
|---|---|
| `annuler` n'empile pas sur `pile_refaire` | `refaire_refait_la_pose_defaite`, `une_nouvelle_operation_invalide_la_pile_de_refaire`, la propriété |
| `enregistrer` ne vide jamais `pile_refaire` | `une_nouvelle_operation_invalide_la_pile_de_refaire` **seul** — la propriété ne l'aurait pas vu, elle n'entrelace jamais une pose neuve entre un annuler et un refaire |
| `avant_mutation` ignore l'opt-in, capture toujours | `annuler_et_refaire_sans_historique_echouent_toujours` |
| `restaurer` oublie le budget | `n_operations_puis_n_annulations_ramenent_a_letat_initial`, puis (après correction) `annuler_defait_la_derniere_pose` — raté la première fois : le test tournait sans `avec_budget`, donc `budget` valait `None` avant comme après, rien à perdre |
| instantané de `retirer` pris après ses mutations au lieu d'avant | seulement la version **renforcée** de la propriété (voir ci-dessus) |

Le quatrième et le cinquième sont les deux enseignements réels de l'étape :
un test round-trip qui ne compare que les extrémités peut être aveugle aux
mêmes fautes qu'un scénario mal choisi ou une variable figée — la forme du
défaut change, la parade est toujours la même, muscler le test dès qu'un
sabotage ne mord pas.

**Mesures** : 241 → **246 tests**, ~0,9 s. 46 → **47 avertissements** —
`annuler`/`refaire`/`avec_historique` (et par ricochet `Budget::rembourser`,
appelé seulement depuis un `retirer` lui-même sans consommateur binaire)
rejoignent le même constat qu'en L1.6/L2.1/L2.2.

### L2.4 La palette : la duale de `compatibles` (2026-08-01)

§6.5 : `Chantier::compatibles(&comp, montage_idx)` part d'un composant et rend
les ports qui l'accepteraient. Il manquait l'inverse — « énumérer tous les
composants posables sur CE port libre » — plus deux partitions : chaque
variante dans **une** catégorie de palette, et palette/`compatibles` d'accord
partout.

#### Une source, pas une deuxième chaîne

La palette a besoin d'un échantillon par variante — exactement ce que la
chaîne `suivante`/`echantillons` de L1.2 fournissait déjà, mais enfermée dans
`#[cfg(test)]`, où seuls les tests de balayage et le générateur de
`docs/reference/fils.md` la lisaient. Plutôt que d'écrire une seconde liste de
31 échantillons pour la palette — la duplication exacte qui a produit le
doublon d'indice de §5.1 — `suivante`/`sous_ensemble_echantillon`/
`echantillons` ont déménagé dans le corps du module, `pub`, hors des tests.
Rien ne change dans leur contenu ; `echantillons()` sert maintenant **trois**
consommateurs (balayage de couverture, `fils.md`, palette) au lieu d'un.

`posables(genre, profil) -> Vec<(Composant, usize)>` filtre ces 31
échantillons par le même prédicat que `Chantier::compatibles`
(`GenrePort::compatible` × `Profil::compatible`), juste appliqué dans l'autre
sens — sur les ports du composant plutôt que sur les ports libres du chantier.

#### Les dix catégories, et pourquoi ce découpage-là

`Categorie` (`Structure`, `Habitat`, `Energie`, `Communication`, `Cargo`,
`Propulsion`, `Bouclier`, `Composite`, `PoseeAMain`, `Effet`) reprend le
découpage déjà présent dans le code — un fichier de famille par groupe de
variantes (`treillis.rs`, `propulsion.rs`, `cargo.rs`…) — plutôt que d'inventer
une taxonomie neuve à côté. `PoseeAMain` (`TreillisHexagone`,
`BouclierThermique`, `Charniere`) et `Effet` (`Panache`) regroupent les
variantes dont `ports()` rend `[]` : structurellement invisibles à `posables`,
quel que soit le port visé, puisqu'aucun de leurs ports ne peut jamais être
compatible avec rien. Pas un cas à part à coder — une conséquence de la même
formule.

⚠️ Trouvé en écrivant le test d'accord : `GenrePort::PoutreBout` n'est posé sur
**aucun** composant du jeu actuel (`grep` à vide dans tout `composant/`) —
variante d'enum mort, sans rapport avec L2.4, non touché ici.

#### Le test qui a raté, deux fois, avant de tenir

Le premier jet comparait palette et `compatibles` **par indice de montage
exact** : pour un hôte, un port, une cible, un indice `m`, `posables`
liste-t-elle `(cible, m)` exactement quand `compatibles(cible, m)` accepte ce
port ? Rouge dès le premier essai, sur un cas révélateur : `ModuleAxial` a
deux écoutilles axiales symétriques (montage 0 et 1) ; `posables` — qui ne
retient que la **première** trouvée par `.position()` — ne proposait que
l'indice 0, alors que `compatibles` acceptait aussi l'indice 1. Le test
« divergeait » sur une différence qui n'en est pas une du point de vue du
joueur : les deux ports mènent au même résultat, la palette n'a besoin de
promettre qu'*un* indice qui marche, pas tous.

Corrigé en comparant à la granularité que §6.5 demande réellement — « toute
variante proposée… y est effectivement posable » (la **variante**, pas un
indice précis) : `posables` inclut-elle la cible **IFF** `compatibles` accepte
**au moins un** de ses montages sur ce port. Sous cette forme, le test a
immédiatement rougi sur les deux sabotages suivants, et est resté vert sur le
comportement réel.

#### Red-check : 3 sabotages, 3 rougissements

| Sabotage | Rougit |
|---|---|
| `categorie` détourne `Antenne` vers `Structure` (catégorie `Communication` vidée) | `toute_categorie_de_palette_est_atteinte` |
| `posables` compare par égalité stricte (`==`) au lieu de `.compatible()` | `palette_et_compatibles_saccordent` — révèle au passage que `ModuleAxial`/`ModuleRadial` sont mutuellement compatibles, pas juste réflexifs |
| `posables` propose tout, sans regarder le port visé | `palette_et_compatibles_saccordent` (sur-inclusion, pas seulement sous-inclusion) |

Les deux premiers essais ratés du test d'accord (ci-dessus) ne sont pas
comptés comme des sabotages : ils ont rougi sous le comportement **correct**
du code, preuve que c'est le test qui avait tort, pas `posables` — la
distinction qui compte pour savoir quoi corriger.

**Mesures** : 246 → **248 tests**, ~0,7 s. 47 → **54 avertissements** —
`Categorie`, `categorie`, `posables`, et la chaîne d'échantillons promue
(`suivante`, `sous_ensemble_echantillon`, `echantillons`) rejoignent le même
constat que le reste du Lot 2 : sans consommateur avant l'écran de palette
(hors modèle, donc hors scope ici — §6.6).

### L2.5 Sérialisation et aller-retour géométrique (2026-08-01)

**Dernière étape du Lot 2** (§6.4) : un format de sauvegarde pour `Chantier`,
et la propriété qui en fait foi — rejouer une sauvegarde doit reproduire la
géométrie **au sommet près**, pas seulement « le même nombre de pièces ».

#### La question tranchée avant d'écrire le format

`Composant::SousEnsemble` porte des `Piece` **déjà cuites** (`Mat4` figées à
`figer()`) : un assemblage qui en contient un ne peut pas se rejouer depuis la
seule liste des poses de haut niveau. Deux issues possibles, posées dans les
« Questions ouvertes » : sérialiser la *recette* du sous-ensemble (ses propres
poses, rejouées à la désérialisation) ou ses *pièces* cuites directement.
Demandé à l'utilisateur, qui a renvoyé la question technique plutôt que de
trancher à l'aveugle : *« laquelle des deux solutions est la meilleure d'un
point de vue performance et évolutivité ? »*

**Pièces cuites, retenu, pour deux raisons :**

1. **Performance** — un `SousEnsemble` existe précisément pour être **réutilisé**
   (§C.29 le préfigurait pour l'ISV). Rejouer sa recette à chaque
   désérialisation referait, pour chaque instance réutilisée, tout le travail
   de pose déjà validé une fois à la construction — collisions comprises,
   O(n²)-ish. Les pièces cuites se désérialisent en O(n), sans repasser par
   aucun contrôle.
2. **Évolutivité** — §6.4 le dit explicitement : la sauvegarde dépend du
   **déterminisme** de la construction, pas de sa stabilité dans le temps.
   Ce projet a un historique démontré de formules de géométrie et de marges
   de collision retouchées après coup (L1.4, §9, L1.6…). Une recette rejouée
   est vulnérable à ces retouches futures — une vieille sauvegarde pourrait
   se reconstruire différemment, ou plus du tout. Des pièces cuites sont
   immunisées : elles portent la géométrie telle qu'observée au moment de la
   sauvegarde, pas une promesse de la retrouver.

#### Le format : une recette indépendante des `id` de session

`Etape { composant, hote: Option<(usize, usize)>, montage }` — une pose
addresse son hôte par **`(indice de la pièce dans la séquence, indice local
du port dans `ports()` de son hôte)`**, pas par un `PortLibre::id`. Un `id` de
session (L2.1) n'a de sens que **pendant** cette session ; une sauvegarde doit
survivre à la fermeture du jeu. D'où un nouveau champ, `PortLibre::indice:
usize` — la position du port dans la liste `ports()` de son composant hôte,
posée une fois pour toutes par `ajouter_libres` et jamais recalculée ailleurs.

`Chantier::recette()` exporte, `Chantier::depuis_recette()` rejoue — sans
budget (§6.4 : la recette rejoue des choix déjà validés, le budget est une
règle de construction, pas une propriété de la géométrie qui en résulte) et
en rendant `None` proprement (pas de panique) si une étape échoue : recette
tronquée, corrompue, ou une pose qui passait alors ne passe plus.

Conséquence en cascade sur `Entree.hote`, comme en L2.1/L2.2 : il fallait
retrouver le `montage_idx` utilisé par une pose pour reconstruire `Etape`,
alors que `Chantier` ne le gardait nulle part — devenu
`Option<(PortLibre, usize)>` (le port hôte exact **et** l'indice de montage
choisi).

Le reste de l'assiette : `serde`/`serde_json` (déjà présents), la feature
`"rc"` de `serde` (`Rc<DonneesSousEnsemble>`), et `glam` ajouté en dépendance
**directe** juste pour activer sa feature `"serde"` — il est déjà tiré par
macroquad à la même version, Cargo unifie les features sur l'instance
partagée plutôt que d'en créer une seconde copie. `Serialize`/`Deserialize`
dérivés sur toute la fermeture transitive de `Composant` (~15 types).

#### Le red-check a de nouveau pris deux tests sur le même défaut que L2.1

Quatre tests : l'aller-retour géométrique (racine + deux poses, dont un port
retrouvé par genre plutôt que par position), le même avec un `SousEnsemble`
imbriqué posé dedans, le même via un aller-retour JSON complet
(`serde_json::to_string`/`from_str`), et le rejet propre d'une recette
corrompue (hôte pointant sur un indice hors bornes).

**Sabotage** : `depuis_recette` retrouve un port hôte en ignorant
`port_idx`/`indice` (prend le premier port libre de l'hôte, quel qu'il soit).
Sur 3 tests concernés, **un seul** a rougi. Les deux autres utilisaient tous
deux `ch.libres()[0].id` pour choisir leur port — coïncidant exactement avec
ce que le sabotage rend aussi (« le premier venu ») ; ils ne pouvaient donc
pas distinguer le code correct du code cassé. Corrigé en durcissant les deux
scénarios pour viser délibérément un port **qui n'est pas le premier** — le
test du sous-ensemble choisit `libres()[3]` sur un nœud à six sorties
symétriques en genre mais distinctes en orientation (un exprès commentaire
l'explique dans le code), le test JSON pose sur un port `Surface` retrouvé par
`.find()` plutôt que sur l'écoutille axiale en position 0. Sous cette forme,
les trois tests rougissent bien sous le même sabotage.

**Exactement le même défaut qu'en L2.1** (§L2.1 ci-dessus, deux tests sur
trois ratés au premier essai) : un scénario qui prend « le premier disponible »
ne peut jamais faire la différence avec un code qui, à tort, fait pareil. La
leçon se répète parce qu'elle n'est pas propre à `Chantier::trouver` — elle
tient à toute logique de sélection dont le premier élément est aussi, par
construction du scénario de test, le bon.

#### Red-check final : 4 sabotages, 4 rougissements (dont un durcissement)

| Sabotage | Rougit |
|---|---|
| `recette()` exporte toujours `montage = 0` | 3 tests sur 4 (pas le test de recette corrompue, orthogonal) |
| `depuis_recette` ignore `port_idx`/`indice`, prend le premier port de l'hôte | 1 test sur 3 concernés au premier essai ; les 3 après durcissement (voir ci-dessus) |
| `depuis_recette` ignore un `piece_idx` hors bornes (`unwrap_or(&0)` au lieu de `?`) | `depuis_recette_sur_une_recette_corrompue_echoue_proprement` |
| `recette()` adresse toujours l'hôte à l'indice de séquence 0 | seulement le test à trois pièces (`recette_puis_depuis_recette_reproduit_la_geometrie_cuite`) — les trois autres n'ont que deux pièces, l'hôte y est **réellement** à l'indice 0, ils ne peuvent structurellement pas voir ce défaut ; pas une faiblesse à corriger |

**Mesures** : 248 → **252 tests**, ~1,0 s. 54 avertissements, inchangé en
façade — `Etape`/`Recette`/`Entree.hote` rejoignent la même dette que
`recette`/`depuis_recette` (sans consommateur avant un écran de sauvegarde,
hors scope modèle), mais le compte net ne bouge pas : les `derive(Serialize)`
ajoutés sur ~15 types font lire, par le code généré, plusieurs champs que le
compilateur classait jusqu'ici « jamais lus » — une bascule qui absorbe les
nouveaux avertissements plutôt qu'un signe qu'il n'y en a pas.

---

## Bilan du Lot 2 (2026-08-01)

Les cinq étapes de `conception/assembleur.md` §7 sont closes : identifiants
stables (L2.1), retrait avec aller-retour (L2.2), undo/redo par instantané
(L2.3), palette duale de `compatibles` (L2.4), sérialisation avec aller-retour
géométrique (L2.5). Le modèle du `Chantier` interactif est complet ; il n'a
encore **aucun consommateur côté écran** — chaque étape l'a relevé en dette,
pas en défaut : L2.6+ (hors du périmètre défini par §7) reviendrait à câbler
un écran d'assemblage sur ce modèle.

**23 sabotages sur le Lot 2 (5+6+5+3+4), tous rougis au final.** La même
leçon méthodologique est revenue **quatre fois sous des formes différentes**
(§9, L2.1, L2.3, L2.5) : un test qui choisit « le premier disponible », fixe
une variable au lieu de la sonder, ou ne compare que les deux bouts d'une
séquence, peut rester vert sous un code cassé qui fait, par coïncidence, la
même chose que le code correct dans ce scénario précis. Le red-check ne vaut
que si le test est renforcé dès qu'il ne mord pas — jamais admis « suffisamment
proche ».

### L3.1 `pose_prevue` : la source unique du fantôme (2026-08-01)

Premier des trois manques relevés en confrontant §8 au modèle livré
(`conception/assembleur.md` §7.1). §8.3 exige que le fantôme soit dessiné à la
pose **exacte** qu'aurait la pièce au clic — « il n'y a qu'une façon de le
garantir : demander la pose au même code que la pose réelle ». Or `poser`
calculait cette transformée en interne sans jamais la publier.

#### Une méthode de plus, mais surtout un calcul de moins

L'ajout évident aurait été d'écrire `pose_prevue` à côté de `poser`, avec son
propre `accoupler`, et un test pour les tenir d'accord. C'est le doublon
habituel de ce projet, avec un test en guise de béquille.

Retenu à la place : extraire `Chantier::corps_prevu(hote, comp, montage_idx)`
— le port de montage retenu et le repère monde qui en découle — et le faire
appeler par **les trois** méthodes qui posent la même question sous trois
formes : `poser` (*pose*), `peut_poser` (*pourrait-on*), `pose_prevue` (*où*).
Il n'y a plus rien à tenir d'accord, donc plus rien qui puisse diverger. Les
deux méthodes existantes ont maigri au passage.

`pose_prevue` rend une `Piece` complète (transformée **et** composant), pas une
`Mat4` : c'est ce que la vue dessine, et surtout ce qui rend le test comparable
au résultat réel par simple égalité.

#### Le contrat : la géométrie, pas l'acceptation

`pose_prevue` répond `Some` dès que la pose est **définie** — le port existe,
`montage_idx` désigne un port du composant — sans regarder compatibilité,
collision ni budget. C'est délibéré, et c'est §8.5 qui l'impose : l'overlay
trace le segment de plus courte approche entre le fantôme et la pièce qui le
**refuse**. Un fantôme qui n'existerait que pour les poses acceptées rendrait
impossible le seul tracé qui compte — celui qui explique un refus. Le *où* est
dans `pose_prevue`, le *si* dans `peut_poser` : deux questions, deux méthodes.

#### Red-check : 3 sabotages, et un résultat qui méritait d'être noté

| Sabotage | Rougit |
|---|---|
| `corps_prevu` inverse les arguments d'`accoupler` | 8 tests, dont `pose_prevue_repond_meme_quand_la_pose_serait_refusee` — mais **pas** le test d'accord (voir ci-dessous) |
| `pose_prevue` « recalcule » la pose de son côté (fantôme posé sur le port hôte, sans tenir compte du port de montage) | `pose_prevue_annonce_exactement_la_pose_que_poser_produit` **seul**, sur 255 tests |
| `pose_prevue` conditionnée à `peut_poser` (le repli « serviable » qui casserait §8.5) | `pose_prevue_repond_meme_quand_la_pose_serait_refusee` **seul** |

**Le premier sabotage ne fait pas rougir le test d'accord, et c'est correct** —
il casse `pose_prevue` et `poser` **de la même façon**, puisqu'ils partagent
désormais le calcul : les deux continuent donc de s'accorder, sur une
géométrie fausse que huit autres tests attrapent. C'est la contrepartie exacte
du single-sourcing : le test d'accord ne garde pas la justesse du calcul, il
garde son **unicité**. Les deux sabotages suivants le confirment par l'autre
bout — chacun n'est vu que par le test qui porte précisément ce contrat, et le
second par **aucun** des 254 autres tests du projet : rien d'autre ne regarde
`pose_prevue`, donc rien d'autre ne verrait un fantôme qui ment.

C'est la même leçon qu'en L2.2 (« seule la propriété d'aller-retour l'a
prise »), sous une forme neuve : il faut savoir *quel* mode de défaillance
chaque test garde, sous peine d'en croire un capable de ce qu'il ne fait pas.

**Mesures** : 252 → **255 tests**, ~0,9 s. 54 avertissements, inchangé.
