# Étude — comment sortir les boucles de plasma du « croissant »

> Question posée après le lot 3 bis (`57be998`) : moins d'arches vivantes et
> ancrage réparti sur plusieurs taches n'ont pas suffi. Comparaison des
> techniques de rendu d'une arche, hors régulation temporelle déjà en place
> (`MAX_BOUCLES`, `CADENCE_MIN`, `tache_porteuse`).

## Le constat de départ

Une boucle (`Boucle` dans `eruptions.rs`) est une Bézier quadratique entre
deux pieds. `rendu.rs` la trace avec **48 billboards** posés le long de la
courbe (`point_boucle`), texturés d'un halo radial, en blending **additif**,
sans test de profondeur entre eux. C'est la cause mécanique du « croissant » :
deux billboards qui se recouvrent n'occultent rien, ils **s'additionnent**.
Sur une arche seule, l'œil lit un dégradé continu (voulu — un tube n'a pas
d'arêtes). Dès que deux ou trois arches partagent un voisinage — ce que
l'ancrage sur les taches encourage, puisqu'elles y sont attirées — leurs
halos se recouvrent aussi **entre arches**, et la somme sature en une nappe
blanche sans texture individuelle : plus de boucles, un croissant.

Le plancher en pixels (`plancher_plasma`, lot 3 bis) a réglé un problème
différent — la disparition sous le pixel — pas celui-ci. Il agit sur *une*
arche ; le croissant est un problème *entre* arches.

Échelle de référence (`docs/suivi/etoiles.md`, P.3) : au cadrage par défaut
une étoile fait ~2 px de rayon, sa surface n'existe qu'en dessous de ~100
unités (~2 UA), et une boucle à `r × 0,05` demande un Soleil de 20 px de
rayon pour atteindre le pixel et 60 px pour se lire comme une arche — plus
large que l'orbite de Mercure. **Les boucles sont, et restent, une
récompense d'approche** : toute technique se juge à cette distance-là.

---

## 1. Le chapelet de billboards (existant) — ce qu'on peut encore en tirer

**Rendu** : une nappe de points lumineux le long de la courbe, lisible comme
tube *tant qu'une seule arche est visible à la fois*. Le défaut n'est pas
dans la géométrie d'une arche mais dans l'**empilement additif entre
arches** et dans l'absence de hiérarchie visuelle le long d'une même arche
(48 points identiques en taille et en teinte, sauf l'onde de `flow`).

**Coût** : 48 quads × ≤4 arches vivantes = ≤192 quads/frame, plus le cœur de
rupture (1 quad par arche en état `Rupture`). Un seul `flush` tous les 200
quads (`QUADS_PAR_LOT`) — donc au plus 1 à 2 `draw_mesh` pour tout le plasma
d'une étoile. Rien à ce budget qui menace le plafond de pipelines ou le
temps GPU ; le levier n'est pas le coût, il est purement visuel.

**Ce qui reste à en tirer sans changer de technique :**

- **Épaisseur variable le long de l'arche** : `taille_l` est constante sur
  les 48 points. Un tube de plasma est plus fin aux pieds et renfle vers
  l'apex — un profil en `sin(π·t)^k` (k~0,6) évoque un tube sans coût
  supplémentaire, juste un facteur multiplicatif par point.
- **Atténuation aux pieds** : fondre l'alpha vers 0 aux deux extrémités
  (`t` proche de 0 ou 1) au lieu de couper net évite l'arête qui trahit un
  chapelet, et prépare l'ancrage visuel (§5).
- **Ordre de tracé** : sans effet ici — en additif la somme ne dépend pas
  de l'ordre (ne vaudrait que pour une passe non additive).
- **Teinte qui suit la température** : `lum` module déjà vers le
  blanc-jaune. On peut teinter différemment pieds (plus rouge/froid) et
  apex (plus blanc/chaud) — dégradé le long de `t`, gratuit en CPU.
- **Le vrai levier reste la densité spatiale**, pas la présentation d'une
  arche seule : **espacer les points d'ancrage entre arches simultanées**
  (répulsion minimale entre `centre` de boucles nées proches dans le
  temps) réduirait le recouvrement additif bien plus qu'aucun réglage de
  billboard — complément direct au lot 3 bis, qui a réparti sur les taches
  mais pas entre arches sur une même tache.

**Ce que ça casse** : rien — ce sont des variations de paramètres dans une
fonction déjà en place. Risque nul sur le plafond de pipelines/uniforms.

---

## 2. Le ruban orienté (un quad étiré le long de la courbe)

**Principe** : remplacer les 48 billboards face-caméra par une bande
continue — une polyligne de N segments, chacun un quad dont la largeur
suit la normale à la courbe **projetée dans le plan écran**, avec un
dégradé de largeur et d'alpha intégré aux UV plutôt qu'à 48 draws séparés.

**Rendu** : supprime la texture de halo répétée 48 fois (le « chapelet de
pastilles » qu'on devine même sur une arche isolée) au profit d'un contour
continu, sans discontinuité entre segments si largeur et alpha sont
interpolés aux jointures — la **continuité longitudinale** qui manque le
plus au rendu actuel, où le billboard-brochette n'a que de la continuité
radiale (le halo de chaque point).

**Coût** : un ruban à 16-24 segments suffit à une Bézier quadratique —
~32-48 sommets et un seul quad-strip par arche, contre 48 quads (192
sommets) aujourd'hui. **Moins cher en sommets**, budget de `draw_mesh`
inchangé (même mécanisme de lot que `flush`).

**Faisabilité avec `impostor`** : `push_quad` ne convient pas tel quel — il
pose un quad *centré*, face caméra, de taille uniforme. Un ruban a besoin
d'une normale par segment (perpendiculaire à la tangente de la courbe et à
la direction caméra) et d'une largeur variable : une fonction sœur de
`push_quad`, calcul CPU simple comme l'actuel `point_boucle`, pas de
vertex shader dédié. Le matériau `mat_plasma` (même texture de halo, UV le
long du ruban plutôt qu'en tuile radiale) reste identique : **zéro
pipeline supplémentaire**.

**Occultation et scintillement** : `cache_par_sphere` teste déjà **par
point** — un test par sommet du ruban donne la même granularité que les
48 billboards, pas de régression. Le scintillement tient au plancher en
pixels (`plancher_plasma`), qui recalcule la taille par frame quelle que
soit la géométrie — risque identique à l'existant, ni pire ni meilleur.

**Ce que ça casse** : le code de tracé de `rendu.rs` (boucle `for b in
&self.boucles`) est à réécrire — pas un patch mineur, une nouvelle fonction
de génération de mesh. Le cœur de rupture (billboard unique à `t=0.5`,
inchangé) reste compatible tel quel. Aucun risque sur les plafonds
(technique de rendu 2D en mesh dynamique, pas de matériau nouveau).

**Verdict** : c'est le palier le plus rentable — moins de sommets que
l'existant, continuité visuelle nettement meilleure, aucun nouveau
pipeline, complexité d'implémentation raisonnable (une fonction de
génération de ruban plus une intégration dans la boucle de dessin).

---

## 3. Le tube de flux (vraie géométrie tubulaire)

**Principe** : un maillage cylindrique classique autour de la courbe —
anneaux de sommets (6 à 8 côtés) le long de N segments, avec normales pour
un éclairage de surface plutôt qu'un dégradé de halo.

**Sommets** : un anneau à 8 côtés et 16 segments de longueur fait
`8 × 17 = 136` sommets par arche (contre 32-48 pour le ruban, 192 pour le
chapelet actuel) — ~544 pour 4 arches, rien face aux 25 600 sommets des
ceintures (`docs/suivi/ceintures.md`, jugés « du bruit pour un GPU ») ni
aux 28 440 d'une coque de station (`docs/suivi/stations.md`, §C.7) ou aux
104 658 d'un tore de Stanford. **Le budget en sommets n'est objectivement
pas le problème** — même à 8 arches, le tube resterait sous le millier.

**Ce qui n'est pas gratuit :**

- **Pas de double face** (`docs/suivi/technique.md`) : un tube vu de
  l'intérieur (l'arche passe devant la caméra, ou la caméra est à
  l'intérieur d'une grande éjection) montrerait des faces retournées
  invisibles, sauf à émettre les deux enroulements — double le nombre de
  triangles, ou à billboarder l'axe transverse (ce qui le ramène en fait à
  un ruban orienté avec un profil, pas un vrai solide).
- **Un matériau/pipeline dédié** : réutiliser `mat_plasma` en UV
  longitudinal reste possible (pas de nouveau pipeline), mais un éclairage
  de surface façon `disque_voile.frag.glsl` ajouterait de la complexité
  GLSL sans bénéfice net — le plasma n'a pas de surface physique à
  éclairer, il est translucide et additif par nature.
- **Le blending additif reste posé au-dessus.** Deux tubes qui se
  recouvrent en écran s'additionnent toujours en couleur — la géométrie 3D
  n'apporte d'occultation qu'*avec* un depth test, ce qui contredit le
  rendu additif actuel (`draw_mesh` sans z-write pour la couronne/le
  plasma). Passer le plasma en test de profondeur changerait aussi
  comment il se compose avec la couronne — effet à juger à l'écran, pas
  déductible sur plan.

**Verdict** : soutenable au budget de sommets, mais le gain sur le défaut
signalé (le croissant vient de l'**additif entre arches**, pas de la forme
en coupe d'une arche) est marginal par rapport au coût d'implémentation —
nouvelle génération de maillage, gestion de la face simple, décision sur
le depth test. Le ruban (§2) capture l'essentiel de la continuité visuelle
pour une fraction du travail.

---

## 4. Le rendu volumétrique / raymarching dans l'impostor

**Hors de portée, et il faut le dire sans détour.** Ce moteur tourne en
**GLSL ES 100 / WebGL1** (`docs/suivi/technique.md`) : pas de garantie de
boucles à borne dynamique déroulables, pas de `textureLod` fiable, pas de
compute shader pour précalculer un bruit volumétrique, `precision highp
float` non garanti sur tout matériel ES 100. Un raymarching de plasma
correct (dizaines d'échantillons par fragment, occlusion douce,
absorption/émission) suppose un budget d'instructions par fragment que ES
100 n'offre pas de façon portable. Le projet a déjà un précédent : le
shader de granulation (`fbm`) existe mais **tombe sous le pixel** à toute
distance utile (`docs/suivi/etoiles.md`) — un raymarching coûterait bien
plus cher par fragment pour un résultat visible à la même distance
d'approche où le ruban suffit déjà. **À ne pas retenir**, ni maintenant ni
plus tard sur ce moteur — un changement de cible (WebGL2/GLSL ES 300)
serait un préalable, pas un réglage.

---

## 5. L'ancrage visuel — planter le pied, pas le poser devant

Indépendant du choix de §1-3 : ce qui fait qu'un pied de boucle *semble*
sortir de la surface plutôt que flotter devant tient à trois leviers, tous
peu coûteux :

- **Halo au point d'ancrage** : un billboard de halo, plus large et plus
  doux que le corps de l'arche, posé exactement sur `sa`/`sb` (déjà connus,
  déjà projetés en `vers_monde`). Un seul quad de plus par pied — 2 par
  arche, négligeable au budget de `QUADS_PAR_LOT`.
- **Occultation partielle par la sphère** : `cache_par_sphere` masque déjà
  un point *entièrement* caché derrière l'étoile. Pour l'ancrage, l'effet
  recherché est plus subtil — qu'une **fraction** du premier segment se
  fonde progressivement dans la surface au lieu de s'arrêter net
  au-dessus, via l'atténuation d'alpha du §1, combinée à un halo qui
  déborde légèrement sur le disque : le pied se lit « sous » la surface
  sans test de profondeur, par composition plutôt que par géométrie.
- **Continuité de teinte avec la tache** : la couleur du pied devrait
  converger vers la teinte de la tache porteuse plutôt que rester le
  jaune-blanc uniforme du plasma — un `Vec3` de plus sur `Boucle`, fixé à
  la naissance à partir de la tache que `tache_porteuse` a déjà choisie,
  utilisé en fondu sur les `t` proches des extrémités.

**Ce qui ne sera jugeable qu'à l'œil** : l'amplitude du fondu d'ancrage
(trop fort, le pied disparaît ; trop faible, l'arche reste « posée
devant »), le rayon du halo relatif à la tache, et l'étendue de la
convergence de teinte. Rien ne se déduit d'un calcul — ces seuils se
posent comme le plancher en pixels (`PLASMA_PX_MIN`, `soleil::PX_MIN`) :
une première valeur raisonnable, corrigée au jugé.

---

## Recommandation, en lots

**Lot A — retouches sans changement de technique** (§1, hors répulsion) :
profil d'épaisseur, fondu d'alpha aux pieds, teinte dérivant vers la tache
porteuse. Coût faible, zéro risque, mais **ne touche pas la cause
principale du croissant** (l'additif entre arches).

**Lot B — espacement spatial entre arches simultanées** : à la naissance
d'une boucle, répulsion minimale par rapport aux `centre` des boucles déjà
vivantes sur la même tache (au-delà de la pondération du lot 3 bis).
Complément direct du diagnostic : le croissant naît du recouvrement de
halos proches. À faire **avant** ou avec le lot C, sinon un ruban plus
propre saturera quand même si deux arches naissent au même endroit.

**Lot C — passage au ruban orienté** (§2) : remplace le chapelet de 48
billboards par un ruban à largeur/alpha variables. Moins de sommets que
l'existant, continuité longitudinale nettement supérieure, aucun nouveau
pipeline ni matériau — le seul changement de technique qui se justifie au
vu du budget et du rendu. Nouvelle fonction de génération de mesh dans
`rendu.rs` ; `eruptions.rs` reste inchangé.

**Lot D — ancrage visuel** (§5) : halo aux pieds + convergence de teinte.
Se greffe sur A ou C ; à faire après que la forme (lot C) est stabilisée.

**Écarté** : le tube de flux (§3, soutenable en sommets mais gain
insuffisant face au ruban) et le raymarching/volumétrique (§4, hors de
portée de GLSL ES 100 sur ce moteur).

**Ce qui ne se juge qu'à l'écran, en approche (~2 UA), jamais sur plan** :
l'équilibre épaisseur/alpha du ruban, le rayon de répulsion du lot B (trop
large et les arches désertent les taches, trop faible et le croissant
persiste), l'intensité et le rayon du halo d'ancrage, l'étendue de la
convergence de teinte. Chacun de ces réglages est un nombre à poser puis
corriger au jugé, comme le reste des plafonds en pixels déjà dans ce
fichier — aucun ne se déduit d'un calcul de sommets ou de draw calls.
