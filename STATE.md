# STATE — où en est le chantier

> Point de reprise, **2026-08-04**. À lire en premier dans une nouvelle session.
> Ce fichier dit **où on en est et quoi faire ensuite**, rien de plus : le
> détail de chaque étape (ce qui a été trouvé, les red-checks, les mesures)
> vit dans les journaux de [`docs/suivi/`](docs/suivi/), et le *pourquoi* dans
> [`docs/conception/`](docs/conception/).

## Commits

Dernier commit : `f843f9c` « Safe commit before GUI ».

Consigne debout : *« no commit i'll say when commit is needed »* — ne jamais
commiter de soi-même. **Mais signaler les points de commit** : l'utilisateur
commite aux jalons qu'on lui indique, depuis qu'un `git checkout --` a détruit
du travail non commité (`docs/suivi/stations.md` §F.8).

⚠️ **Ne jamais employer `git checkout --`, `git restore` ou `git reset --hard`**
sur un fichier contenant du travail non commité. Pour annuler ses propres
modifications : ré-éditer les lignes, ou sauvegarder le fichier dans le
scratchpad **avant** de le toucher (red-checks compris).

État vérifié : **522 tests verts**, **`cargo clippy --all-targets` : 0 avertissement**.

## Chantier courant : finir les planètes et la vue système

**Décidé le 2026-08-06, avant D-INT-2** : l'économie attendra que la vue système
soit finie. On ne conçoit pas une économie par-dessus un décor qu'on va encore
remuer.

| # | Chantier | Nature | État |
|---|---|---|---|
| **P.1** | **Atmosphère planétaire** | vrai manque, confirmé dans le code | ✅ code fait, **rendu à valider à l'œil** |
| **P.2** | **Ceintures : finition + optimisation** | mesurer d'abord | ⏳ |
| **P.3** | **Étoiles : éjections et motifs de surface** | ⚠️ **existent déjà** — enquête avant travaux | ⏳ |
| **P.4** | Finitions planétaires (rotation propre, nuages, ombres d'anneaux) | liste ancienne | ⏳ |

### P.1 — Atmosphère : fait (2026-08-08)

**Le diagnostic.** `planete.frag.glsl` jetait tout ce qui dépasse la sphère
(`if (rr > 1.0) discard;`), alors que l'impostor est un disque de `DISC` :
**la marge autour de chaque corps ne servait à rien**. Ce que le shader
appelait « halo atmosphérique » n'était qu'un `pow(1 - z, 3)` appliqué **sur la
surface** — un limbe qui s'éclaircit, et rien au-delà de la silhouette. C'est
pourtant le débordement qui fait lire une atmosphère.

**Ce qui a été fait.** Une passe séparée, `src/shaders/atmosphere.frag.glsl`,
dessinée sur le même quad juste après le corps, additive et **sans écriture de
profondeur** (la moitié avant d'un anneau doit continuer à passer devant).

Le modèle est une coquille creuse entre le sol (rayon 1) et le sommet de l'air
(rayon `disc`). Pour un rayon visuel à la distance `b` du centre on mesure la
**corde** parcourue dans la coquille — la quantité d'air traversée :

| Où | Corde | Effet |
|---|---|---|
| `b < 1` (devant le corps) | `√(disc²−b²) − √(1−b²)` — le sol coupe le rayon | voile ténu sur la surface |
| `b > 1` (à côté du corps) | `2·√(disc²−b²)` — la coquille est traversée deux fois | halo qui déborde, ~98 % au limbe |

Le **facteur deux au franchissement de la silhouette n'est pas un artefact** :
c'est exactement ce qui fait du limbe un liseré fin et lumineux plutôt qu'un
dégradé mou. S'y ajoutent la raréfaction en altitude (hauteur d'échelle = 35 %
de la coquille), l'opacité Beer-Lambert, un terminateur qui **déborde sur la
nuit** (crépuscule : la haute couche voit encore l'étoile), l'éclairage
multi-source comme le corps, et la **diffusion vers l'avant** — à contre-jour
l'arc du limbe s'embrase.

Densité et teinte viennent de `atmo` **et** de `voile` : le halo de Vénus est
jaune et épais, pas bleu. L'ancien `rim` a été **supprimé** du corps — deux
modèles pour la même chose se seraient additionnés.

**Trois duplications tuées au passage** (la même erreur que les masses, la
corde du volet, le ventre du Starship) :

- `r * 1.05` était écrit en dur dans `mod.rs` **à côté** de `DISC = 1.05` privé
  dans `materiau.rs` : le quad et l'uniform `disc` pouvaient diverger en silence.
  `DISC` est maintenant `pub(super)` et lu aux deux endroits — ce qui a permis
  d'épaissir la coquille en changeant **un seul chiffre**.
- la formule `px_rayon` était recopiée dans `appliquer_uniforms` ; elle est
  désormais une fonction, réutilisée par le seuil de visibilité.
- les listes d'uniforms des deux materials sont devenues des fonctions
  (`uniforms_corps`, `uniforms_atmo`), ce qui **rend testable** leur
  correspondance avec le `.glsl`.

**Coût maîtrisé** : la passe est sautée si le corps n'a pas d'air, ou si la
coquille tient dans moins de `ATMO_PX_MIN = 0.75` px — en vue système, des
dizaines de corps de quelques pixels ne paient rien.

**Tests** (6 ajoutés, tous red-checkés) : correspondance uniforms ↔ GLSL pour
les **deux** shaders (un uniform lu par le shader mais non déclaré vaut
silencieusement zéro : aucun message, l'effet disparaît), marge `DISC`,
les deux branches du seuil de visibilité, et `px_rayon` contre le viewport et
la distance.

**Épaisseur portée de 5 % à 10 % (`DISC = 1.10`)** après avoir vu le rendu : à
5 % le liseré était juste mais illisible. Profil réel, aux constantes du shader
(`CONTRASTE = 1.6`, `K = 4`, `ECHELLE = 0.35`) :

| `b` | 0 | 0,9 | 0,99 | **1,0** | 1,02 | 1,04 | 1,06 | 1,08 |
|---|---|---|---|---|---|---|---|---|
| Terre (dens. 0,97) | 11 % | 28 % | 55 % | **98 %** | 84 % | 57 % | 29 % | 11 % |
| Mars avant (dens. 0,094) | 1 % | 3 % | 7 % | **31 %** | 16 % | 8 % | 3 % | 1 % |
| Mars après (dens. 0,378) | 4 % | 12 % | 26 % | **78 %** | 45 % | 28 % | 12 % | 4 % |

⚠️ Deux choses que ce tableau dit et qu'il faut garder en tête :

- le voile de 11 % au **centre** du disque est la contrepartie physique d'une
  atmosphère deux fois plus épaisse (on regarde à travers de l'air même à la
  verticale). S'il délave, c'est `CONTRASTE` qu'il faut monter, pas `DISC` ;
- **l'épaisseur n'éclaircit pas** : elle élargit la bande. Un corps à air ténu
  reste pâle parce que son plafond est sa **densité**. C'est ce qui a fait
  ressortir le cas Mars (corrigé ci-dessous) — les deux réglages sont
  indépendants : `DISC` pour la largeur, `atmo`/`voile` pour l'éclat.

⚠️ **Reste à valider à l'œil.** Un shader ne se compile qu'au lancement, et
aucun validateur GLSL n'est installé ici. `ECHELLE`, `CONTRASTE` et `K` sont des
constantes du `.glsl` : rechargeables à chaud (touche R) sans recompiler.
`DISC` est du Rust — le changer demande une recompilation.

### P.2 ter — « Pipelines amount exceeded » : un plafond, et une fuite ancienne

Signalé par l'écran au lancement de la **galerie de débris**, après P.1. La
coquille atmosphérique n'est pas la cause — elle est la goutte.

**Le plafond.** macroquad n'accepte que **32 pipelines** (`quad_gl.rs`,
`MAX_PIPELINES`), et `load_material` en consomme un à chaque appel. Au-delà :
`panic!("Pipelines amount exceeded")`, sans rien pour situer la fuite.

**La fuite.** `Disque::new` appelait `material_particules()` et
`voile::material_voile()` **directement** — un à deux pipelines GPU **par
disque**. La galerie de débris en construit douze d'un coup : jusqu'à 24
pipelines pour les seuls disques. `planete` et `soleil` avaient reçu le cache
qui évite ça ; `disque` ne l'avait jamais eu. Le +1 de la passe atmosphère a
fait franchir le seuil.

**Le remède** : template en `thread_local`, cloné ensuite (`mat_particules`,
`mat_voile`) — douze disques coûtent deux pipelines au lieu de vingt-quatre.

⚠️ **Et sa contrepartie, qui aurait produit un second bug silencieux.** Un clone
de `Material` est un `Arc` sur le **même** pipeline : `set_uniform` écrit dans le
pipeline, pas dans le clone. Sans vidage de lot, le **dernier** disque dessiné
imposerait ses réglages (centre, axes, `gm`, moitié) à tous les précédents —
anneaux plaqués au mauvais endroit. D'où `vider_lot()` dans `draw_moitie`, même
remède que pour les corps planétaires. Le commentaire de `planete/materiau.rs`
affirmait le contraire (« un clone a ses propres uniforms ») ; il est corrigé.

**Tests** : `les_materials_ne_sont_construits_qu_une_fois` tient l'invariant réel
— chaque constructeur brut n'est appelé qu'**une** fois dans la crate, depuis le
cache. Red-check : remettre le material par disque le fait rougir.

⚠️ **Une première version de ce test ne servait à rien.** Elle comptait les
shaders du projet et restait **verte** quand on remettait délibérément un
material par disque : elle ne mesurait pas ce qu'elle prétendait garder. Pire,
sa version suivante était **rouge dès le départ** (elle cherchait l'appel dans le
fichier de définition, pas dans celui de l'appel) — et un test toujours rouge ne
prouve rien de ses red-checks. Leçon, à ajouter aux précédentes : **vérifier que
la base est verte AVANT de saboter**, sinon le red-check ne teste que lui-même.

### P.2 quater — La barre sombre en travers des étoiles protosolaires

Signalée à l'écran sur les cas **protoplanétaire** et **protosolaire** de la
galerie de débris. **Bug ancien**, sans rapport avec la coquille atmosphérique :
il n'était visible que là, et cette vue plantait au lancement jusqu'ici.

**Le mécanisme.** Le soleil est un quad plat face caméra qui va jusqu'à
`couronne` × rayon (jusqu'à 2,1). Son material écrivait la profondeur **sur
toute cette étendue**, couronne translucide comprise. Il posait donc un **plan de
profondeur** perpendiculaire à la vue. Le voile du disque, dessiné après et
testé en profondeur, était rejeté partout où il passait derrière ce plan.

L'intersection d'un plan face caméra avec le plan incliné d'un disque est **une
droite passant par le centre** — la barre. Elle ne se voit que dans l'emprise du
quad du soleil, d'où l'aspect « barre au milieu, en travers de l'étoile ». Et
seulement sur les disques proto* : leur matière commence à 0,1 et 0,3 UA, dans
l'emprise de la couronne, quand les autres ceintures commencent à 2,2 UA.

Le même plan **rognait aussi toute planète passant derrière une étoile** en vue
système : le défaut était général, la galerie l'a seulement rendu visible.

**Le remède** : deux passes sur le même shader, aiguillées par un uniform `zone`
(GLSL ES 100 ne sait pas couper l'écriture de profondeur par fragment).

| Passe | Région | Profondeur |
|---|---|---|
| disque de l'étoile | `rr <= 1`, opaque | **écrite** — il doit occulter |
| couronne | `rr > 1`, translucide | **non** — elle ne doit rien occulter |

Les deux sont décrites par une seule donnée, `materiau::PASSES`, d'où sortent à
la fois les materials et l'ordre de dessin. C'est **le même remède que pour la
coquille atmosphérique** : corps opaque d'un côté, halo sans profondeur de
l'autre. Le soleil faisait précisément ce que la planète évitait déjà.

⚠️ **Et une troisième leçon sur les red-checks.** Mes deux premiers tests de
cette passe lisaient le **texte source** (`include_str!("materiau.rs")`) pour
vérifier que `depth_write` suivait bien le paramètre. Ils sont restés **verts**
sous sabotage : la ligne d'assertion contient elle-même la chaîne cherchée, donc
le test se trouvait lui-même. C'est la **troisième** fois de la session qu'un
test s'auto-satisfait par auto-référence (déjà : le compteur de `load_material`
qui comptait ses propres mentions, puis le même piège sur `sites_load_material`).

Règle à retenir : **un test qui cherche une chaîne dans le source ne doit jamais
contenir cette chaîne**, et plus généralement, préférer une **donnée** (`PASSES`)
à du texte. Les tests actuels lisent `PASSES` ; six sabotages, six rouges.

### P.2 quinquies — La couture des disques proto* : trouvée par sondes, pas par déduction

Une file de petits carrés sombres le long de l'axe horizontal, « au hasard des
deux côtés ». **Trois diagnostics faux avant le bon**, tous posés en raisonnant
sur une capture d'écran. Ce qui a fini par trancher : instrumenter les shaders
(rechargeables à chaud) et demander une observation à chaque coup.

**Ce que les sondes ont dit** (`#define DIAG` dans les deux shaders du disque) :

| Sonde | Observation | Conclusion |
|---|---|---|
| cailloux en magenta | trou magenta sur la ligne | ⚠️ **fausse piste** : le voile était en vert OPAQUE dans la même sonde, il recouvrait les cailloux. Deux variables changées à la fois. |
| découpe désactivée | trou disparu | ⚠️ non concluant : sans découpe, chaque caillou est aussi dessiné dans la passe avant, donc APRÈS le voile |
| découpe active, couleur par passe | **ni trou ni jaune ; le magenta scintille sur la ligne** | ✅ les cailloux sont tous dessinés. Ce n'est pas un trou. |
| vue système | tout en jaune | ✅ elle dessine `moitie = 0` — **une passe, aucune couture** |

**Le mécanisme.** Dans chaque moitié, l'ordre est *cailloux puis voile de cette
moitié*. Un caillou de la moitié AVANT dont le quad déborde à l'écran sur la
moitié ARRIÈRE est donc peint **après** que le voile arrière soit déjà posé, et
le voile avant ne couvre pas ce côté-là. Il reste **nu** : opaque et sombre sur
le voile clair du disque interne. Les cailloux traversant la couture au fil de
leur orbite, la ligne scintille et change de côté — ce que la troisième sonde a
montré directement.

C'est un problème d'**ordre de peinture**, ce qui explique qu'abaisser
`depth_write` n'y ait rien changé (essayé, annulé).

**Le remède : supprimer la découpe.** Trois faits convergent —

- la vue système dessine déjà `moitie = 0` et **n'a aucune couture** ;
- la justification de la découpe (« fiable même sans depth buffer en mode
  pixel ») est **caduque** : la cible de rendu est créée avec `depth: true` ;
- la découpe **dégénère vue de dessus** — son plan devient parallèle au disque,
  et `vside` ne mesure plus « avant/arrière » mais la hauteur au-dessus du plan
  médian, quantité quasi nulle pour un disque mince. C'est ce que l'œil signalait
  par « ça fait bizarre vu d'en haut ».

La galerie dessine donc l'astre d'abord (son disque écrit la profondeur, sa
couronne non — cf. P.2 quater), puis le champ en **une passe**.

⚠️ **Leçon de méthode, la plus chère de la session.** Trois corrections posées
sur trois hypothèses non vérifiées, dont une (`vside` recalculé après le
décalage billboard) était un **no-op géométrique** : le quad est perpendiculaire
à la direction de vue, donc parallèle au plan de coupe — il ne peut pas le
traverser. Le coût : quatre allers-retours. Et la première sonde changeait
**deux variables à la fois**, ce qui a produit une conclusion fausse de plus.
Règle : *un shader se sonde, il ne se devine pas* — et **une sonde ne change
qu'une chose**.

### P.2 sexies — Les débris en « gros carrés » vus de dessus

Signalé après la couture : de face au plan du disque, les cailloux paraissent
grossir et deviennent de vilains carrés. Deux défauts de LOD, tous deux réels.

**1. Tout est en DEMI-taille, et la doc disait le contraire.** `taille`,
`taille_px`, `px_min` et le seuil de silhouette sont des demi-tailles. Un caillou
à `px_min = 1.5` fait donc **3 px de côté**, pas 1,5 — et la marge de lobes le
portait à **3,45**. Le commentaire de `set_px_min` promettait « 1 pixel de la
grille » pour 1.15 : faux d'un facteur ~2,3. Le plus petit débris possible était
deux fois trop gros, partout, depuis toujours.

**2. La marge des lobes s'appliquait aux points carrés.** Le ×1.15 loge les lobes
du patatoïde. Un caillou trop petit pour être sculpté n'en a pas : la marge ne
faisait que le grossir de 15 % pour rien.

**3. Le seuil de silhouette était à 2.5 — en demi-taille, soit 5 px de côté.**
Tout ce qui est en dessous sort en **carré plat**. Ramené à 1.25 (2,5 px de côté).

**Pourquoi de dessus, précisément.** Vu par la tranche, les cailloux s'étalent sur
une grande plage de distances et se répartissent entre les deux branches du LOD —
seuls quelques-uns sont des carrés. Vu de face, ils sont tous à **peu près à la
même distance** : la population entière bascule d'un coup dans la même tranche de
taille. Ce n'est donc pas qu'ils grossissent, c'est que le carré devient
**uniforme et donc voyant**. Hypothèse cohérente avec les trois défauts ci-dessus,
à confirmer à l'œil.

Le seuil vit désormais dans le seul vertex shader (`SEUIL_FORME`), et son signe
transporte la décision jusqu'au fragment shader : les deux étages ne peuvent plus
diverger. Réglable à chaud.

### P.2 ter — Les débris n'avaient pas l'aspect pixel art : le blit était recopié

**Trouvé en lisant, pas en devinant.** `ecran::pixel` existe pour tenir trois
choses en un seul endroit — facteur de réduction, cible basse résolution, et le
**blit**, qui est là où s'applique la quantification de palette. Son en-tête dit
même qu'il est « partagé par quatre vues ».

La galerie des ceintures était une **cinquième**, jamais branchée : son propre
`const PIX = 2`, son propre `render_target_ex`, et un `draw_texture_ex` nu à la
place du blit. Elle pixelisait donc **sans jamais quantifier**. Or c'est la
quantification qui produit les à-plats — `ModeRendu::Palette` le dit dans sa
propre doc : « ce qui produit les à-plats, **donc le pixel art proprement dit** ».
D'où des débris en dégradés RGB continus quand les planètes, elles, passaient
bien par la palette.

**Corrigé en supprimant la copie** : `pixel::dimensions`, `pixel::nouvelle_cible`,
`pixel::PIX`, `pixel::blit`. Au passage, la vue perd son interrupteur local au
profit du **réglage global** à trois états (NET → PIXEL ART → PIXEL ART +
PALETTE), comme la galerie des planètes ; l'état est affiché en toutes lettres au
lieu d'un « pixel: ON » qui ne distinguait pas les deux derniers.

**Garde-fou** `qui_pixelise_passe_par_le_blit_partage` : toute vue qui lit le mode
pixel doit finir sa passe par le blit du module. ⚠️ Première version **trop
large** — elle interdisait tout dessin de texture et accusait la vignette
d'astre, qui pose une miniature dans un panneau sans rapport avec le filtre. La
règle porte donc sur le **couplage réel** (lire le mode ⇒ blitter par le module),
pas sur l'appel de dessin. Deux red-checks.

**Ce défaut ne se voit pas en lisant la vue fautive** : tout y semble normal. Il
ne se voit qu'en **comparant deux vues** — d'où le test, qui fait exactement
cette comparaison.

### P.4 — Finitions planétaires : deux items déjà faits, deux vrais manques

**Vérifié avant d'ouvrir le chantier**, comme pour P.3 — et la liste était à
moitié périmée :

| item | état réel |
|---|---|
| ombres portées des nuages au sol | **déjà fait** (`planete.frag.glsl` l. 840-847) |
| inclinaison d'axe | **déjà fait** (`axe`, par planète) |
| **vitesse** de rotation | ✗ codée en dur : `time * 0.01`, la même pour les neuf |
| ombre de l'anneau **sur la planète** | ✗ seule la réciproque existait |

#### Rotation propre : neuf planètes, une seule cadence

`rot_vitesse` par planète, alimentée par `ROTATIONS_JOURS` — les **vraies**
périodes sidérales, la Terre pour référence (un tour en ~10 min réelles). Jupiter
boucle en 10 h et sort donc 2,4 fois plus vite ; Vénus et Uranus tournent **à
l'envers**, ce qui ne coûte qu'un signe et se voit.

⚠️ **Bornée**, et c'est un choix : à l'échelle littérale, Vénus mettrait 42 h de
jeu pour un tour — indiscernable d'une planète figée, donc lu comme un bug plutôt
que comme un fait. Plancher à 1/40 de la Terre, plafond à ×4.

La table est posée dans `fige()`, là où les autres données réelles d'un corps
atterrissent déjà (rayons, masses) — pas une nouvelle voie d'alimentation.

#### Ombre de l'anneau sur la planète

La réciproque existait (la planète assombrit son anneau, via `ombre_rayon`), pas
celle-ci — pourtant la plus visible des deux : une large bande sombre en travers
des nuages de Saturne, dont la position raconte la saison.

Géométrie exacte, en quatre lignes. En rayons planétaires, le point de surface
est `n` (norme 1) ; on remonte le rayon lumineux jusqu'au plan de l'anneau :

```
dot(n + t·L, rn) = 0   ->   t = -dot(n, rn) / dot(L, rn)
```

`t > 0` dit que l'anneau est bien **entre** la surface et l'étoile ; reste à voir
si le point percé tombe dans la couronne. Le cas rasant (`|dot(L, rn)| < 0.02`)
est écarté : c'est l'équinoxe, l'anneau ne projette qu'un fil. La géométrie de
l'anneau vivait déjà dans `Apparence` — elle n'atteignait simplement pas ce
shader.

#### Un test nouveau, né d'un red-check resté vert

Saboter le shader pour revenir à la cadence codée en dur laissait tout vert : la
correspondance uniforms ↔ GLSL ne compare que des **listes de déclarations**, pas
les usages. Un uniform déclaré des deux côtés et ignoré est pourtant un réglage
qui ne fait rien — *la donnée est juste, la plomberie ne suit pas*, le travers le
plus tenace de ce projet.

D'où `uniforms_inutilises` : tout uniform déclaré doit être **lu** quelque part.
Comparaison d'identifiants entiers, pour qu'`atmo` ne passe pas pour lu à cause
d'`atmo_densite`.

⚠️ **Portée mesurée, pas supposée** : il attrape « déclaré et jamais mentionné »,
pas « mentionné dans du code mort » — débrancher le bloc de l'ombre d'anneau le
laisse vert. Le distinguer demanderait d'analyser l'atteignabilité du GLSL, hors
de proportion.

Deuxième red-check vert corrigé au passage : le garde-fou « période nulle »
était **redondant** — `clamp` ramène l'infini au plafond. Redondant mais pas
équivalent : sans lui, une période absurde donnait la planète la **plus rapide**.
L'assertion « fini et positif » ne distinguait pas les deux ; elle nomme
désormais la valeur attendue.

### P.4 bis — Nuages en couche propre, et le glint qui survit au terminateur

#### Une couche, pas une peinture sur le sol

Les nuages étaient échantillonnés dans le repère du **sol**. Ils dérivaient un
peu (décalages de temps sur le bruit) mais restaient soudés au relief : un
continent gardait éternellement la même couverture. Ce n'était pas une couche,
c'était une texture de plus.

Ils ont désormais leur **propre repère**, qui super-tourne. Deux termes, et il
faut les deux :

- un facteur **multiplicatif** (×1,35), qui garde le sens de rotation — une
  planète rétrograde a une atmosphère rétrograde ;
- une dérive **additive**, sans quoi un rotateur très lent aurait des nuages
  aussi figés que son sol, et la couche cesserait de se lire comme une couche.

Le cas d'école est Vénus : son atmosphère boucle en 4 jours quand sa surface en
met 243, soixante fois plus vite.

**L'ombre portée suit.** Elle échantillonne le même champ, décalé vers le soleil
— donc dans le repère des nuages, avec la lumière exprimée dans **ce** repère.
Mélanger les deux ferait dériver l'ombre loin de son nuage à mesure que les
repères s'écartent, et ils s'écartent d'un tour complet.

#### Pas d'air, pas de nuages

Condition ajoutée dans le shader (`atmo + voile`), et **les géantes gazeuses ne
passent jamais là** : leur branche rend la main bien plus haut dans `surface()`.
Vérifié avant d'ajouter quoi que ce soit, plutôt que supposé.

L'invariant existait déjà côté **catalogue** (`la_vie_et_les_nuages_ont_leurs_conditions`).
Il manquait pour les mondes **tirés** — or la skymap n'utilise pas les presets,
elle tire avec `apparence_tellurique`. Un caillou sans air qui en sortirait
nuageux n'était attrapé par rien, et le garde-fou du shader l'aurait rendu
**invisible** au lieu de le corriger. D'où
`les_mondes_tires_n_ont_pas_de_nuages_sans_air`, qui balaie cinq graines sur tout
le spectre de températures.

#### Le reflet de l'océan éteint là où il est le plus beau

Il était multiplié par `diff`, le diffus lambertien : le glint s'éteignait donc
exactement au terminateur. C'est l'inverse de la physique — un reflet spéculaire
est **d'autant plus fort que l'angle est rasant** (Fresnel), et les images
satellite les plus spectaculaires du glint terrestre sont prises près du
terminateur.

L'extinction est désormais posée sur la **géométrie** (pleine puissance jusqu'au
terminateur, éteinte peu après) plus un renfort de Fresnel au rasant. Un reflet
qui déborderait franchement sur la nuit serait, lui, une lampe.

⚠️ **Ce qui est testé, et ce qui ne l'est pas.** La règle « pas de nuages sans
air » est tenue côté **données** (catalogue et générateur). Le comportement du
**shader** ne l'est pas : les deux sabotages — retirer la condition d'air,
recoller les nuages au repère du sol — restent verts. `uniforms_inutilises` ne
voit que « déclaré et jamais mentionné », et ces uniforms restent mentionnés
ailleurs. Vérifier ça demanderait un banc de rendu GPU ; en attendant, cette
moitié tient à la lecture et à l'œil.

### Interface — Bascules de calques en « doubles carrés »

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

### Interface — Trajectoires planétaires en points sarcelle

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

### Interface — Désigner un corps efface les traces qui ne le concernent pas

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

### P.3 — Étoiles : deux causes, chiffrées. Une corrigée, une à décider

Rappel du point de départ : **le code existe** (`soleil/eruptions.rs` — boucles
Montée→Tenue→Rupture→Retombée, taches, granulation `fbm`, trois couronnes) et
`Astre::update` **est** appelé. Le défaut n'était donc pas l'absence. Deux
mécanismes distincts, tous deux réels, à deux endroits de la chaîne.

#### 1. L'échelle — ce n'est pas un bug, c'est de la physique

`R_SOLEIL = 1,116` unité monde. Rayon apparent, fov 45°, écran de 1080 :

| caméra | soleil (px de rayon) | boucle (`r × 0,05`) | cœur d'éjection (`r × 0,16`) |
|---|---|---|---|
| **1280** (défaut skymap) | **1,14** | 0,06 | 0,18 |
| 480 | 3,03 | 0,15 | 0,49 |
| 100 | 14,6 | 0,73 | 2,33 |
| 20 | 72,8 | 3,64 | 11,6 |
| 5 (focus étoile) | 291 | 14,6 | 46,6 |

**Au cadrage par défaut de la vue système, l'étoile fait deux pixels de large.**
Aucune granulation, aucune boucle ne peut y être visible — et c'est juste : une
étoile *est* un point à cette distance. Il faut descendre sous ~100 unités
(≈ 2 UA) pour que la surface commence à exister.

⚠️ **Décision à prendre, et elle est de conception, pas de correction** : soit on
assume que le détail stellaire ne se voit qu'en approchant, soit on exagère le
rayon des étoiles en vue système comme les jeux le font couramment. Je ne tranche
pas — c'est un parti pris visuel.

#### 2. L'horloge — ça, c'était un vrai défaut, corrigé

`Systeme::update` passait **la même cadence** aux orbites et à l'animation propre
des astres. Or le multiplicateur de vitesse (×0,125 à ×16 en vue système, jusqu'à
×64 en galerie) existe pour faire tourner les **orbites** : une période de Kepler
se compte en années.

Les rythmes d'éruption, eux, sont réglés à la main comme un **spectacle** : une
tache naît en 1,5 s, vit 8 à 16 s, meurt en 2 s ; une boucle monte en ~2 s. À ×16
tout cela tient dans une demi-seconde — on ne lit plus une protubérance, on lit du
bruit. **Et il faut accélérer le temps pour voir bouger quoi que ce soit d'autre**,
donc les deux réglages se combattaient.

`Systeme::update(dt, dt_anim)` sépare les deux. La pause fige toujours tout.

⚠️ **Le test seul ne suffisait pas, et le red-check l'a montré.** Il vérifie le
contrat de `update` (l'animation reçoit `dt_anim`, la pause la fige), mais il est
resté **vert** quand on a saboté le *câblage* de la vue système — un test ne voit
pas comment chaque appelant branche ses arguments. Les deux horloges étant des
secondes en `f32`, les intervertir compilait sans un mot : exactement le défaut
d'origine.

D'où `DtAnimation(f32)`, un type distinct. Le sabotage qui passait inaperçu
**ne compile plus**. Leçon : quand deux paramètres de même type ne veulent pas
dire la même chose, c'est au **type** de le dire, pas à un test de l'espérer.

### P.2 decies — ⚠️ Régression de ma correction : annulation catastrophique

Les débris **les plus lointains** enflaient brusquement, au hasard, sans arrêt.
**Défaut introduit par la correction de P.2 octies**, pas un défaut ancien.

Ma première version mesurait la taille projetée en **soustrayant deux points
projetés** :

```glsl
float ndc_c = clip_c.y / max(clip_c.w, 1e-4);
float ndc_h = clip_h.y / max(clip_h.w, 1e-4);   // centre décalé de `taille`
float taille_px = abs(ndc_h - ndc_c) * viewport_h * 0.5;
```

Sur le nuage de Oort (20 000 UA, cailloux de 0,012 unité), ces deux NDC
coïncident sur les **~7 chiffres significatifs** d'un f32 : leur différence n'est
plus que du bruit d'arrondi. `px_min / taille_px` divisait donc par du bruit —
facteur d'agrandissement aléatoire, retiré à chaque frame, d'où le scintillement
en taille. **Annulation catastrophique**, cas d'école.

**Le remède : mesurer une ÉCHELLE, pas une différence de positions.** On projette
`cam_up` comme une **direction** d'une unité monde (`w = 0`) et on applique la
règle du quotient en une seule expression :

```glsl
vec4 dclip = Projection * Model * vec4(cam_up, 0.0);
float k = abs(dclip.y * clip_c.w - clip_c.y * dclip.w) / w2 * viewport_h * 0.5;
float taille_px = taille * k;
if (taille_px < px_min) { taille = px_min / k; }   // pas de rapport de petits nombres
```

`k` ne dépend plus de `taille` : ses termes restent d'ordre 1 quelle que soit la
distance, et rien ne se soustrait de presque égal. Le clamp pose ensuite la
taille voulue **directement** (`px_min / k`) au lieu de la multiplier par un
rapport de deux petits nombres.

⚠️ **Deux leçons, et la seconde est la vraie.**

1. En virgule flottante, **une différence de deux quantités presque égales ne
   vaut rien**. Là où on veut une dérivée, transformer une *direction* plutôt que
   soustraire deux *points*.
2. **Corriger un défaut d'échelle en en introduisant un autre**, c'est le
   troisième aller-retour de la session sur ce même clamp sub-pixel (matrice
   vue-projection, demi-tailles, puis précision). Ce bout de code touche à
   quatre ordres de grandeur — de l'anneau planétaire au nuage de Oort — et
   mérite d'être éprouvé **aux deux bouts** avant d'être déclaré bon.

### P.2 quater bis — Débris d'anneau : tellurique et géante accordées, puis affinées

**Pourquoi les deux vitrines ne se ressemblaient pas**, alors qu'elles partagent
la même config (`debris_planetaire`) et le même cadrage relatif :

| | rayon | grain apparent |
|---|---|---|
| « Carbone » (tellurique) | 0,451 | **× 2,7** |
| « Neptune » (géante) | 1,202 | référence |

`taille_min`/`taille_max` valaient 0,015 et 0,09 en **unités monde absolues**,
quand tout le reste de la géométrie s'exprime en rayons planétaires — bords de
l'anneau (`r × 1,9` à `r × 3,4`) et jusqu'au recul de la caméra (`ext × 2`). La
taille à l'écran vaut donc `taille / rayon` : elle variait comme l'**inverse** du
rayon de la planète. Petite planète, gros cailloux.

Rien ne le signalait : les deux cas se lisent identiques dans le code. Il fallait
comparer les deux vitrines pour le voir — encore un défaut invisible à la lecture.

**Corrigé** : le grain est une fraction de la **largeur de l'anneau**
(`DEBRIS_MIN`/`DEBRIS_MAX`), donc identique quelle que soit la planète. Puis
**réduit de 40 %** sur demande, les deux vitrines une fois accordées.

**Tests** : `le_grain_des_debris_ne_depend_pas_de_la_taille_de_la_planete` compare
le grain apparent des deux rayons réels du catalogue (écart < 1 %), et
`les_debris_restent_fins_devant_l_anneau` borne le plus gros caillou à 5 % de la
largeur. Quatre red-checks — retour à l'absolu, une seule des deux tailles mise à
l'échelle, grain grossier, min/max inversés.

### P.2 bis — Voile des anneaux : l'instrument, faute de pouvoir mesurer

Le seul coût GPU plausible restant des disques est le **remplissage de pixels du
voile** : 960 quads, mais potentiellement plein écran, et chaque pixel paie 155
lignes de bruit procédural, deux fois (avant + arrière).

Je ne peux pas prendre la mesure — le jeu demande une fenêtre. Ce que je peux
faire, et qui vaut mieux qu'un ressenti : **rendre la mesure sans ambiguïté**.
Touche **V** en vue système et en galerie des ceintures : voile on/off, état
affiché à côté du compteur de FPS.

Comparer « caméra serrée sur Saturne » à « caméra au large » **n'isolait rien** :
approcher change aussi le coût du corps, de son atmosphère, de ses lunes.
Éteindre le voile **à cadrage identique** ne change qu'une seule chose, et
l'écart de FPS lui est alors entièrement imputable.

Protocole : vue système, cadrer Saturne anneau plein écran, noter les FPS, **V**,
noter à nouveau. Puis reculer et refaire. Un écart net et croissant avec la
surface couverte désigne le fill rate ; pas d'écart, et P.2 est close.

Le voile est **allumé par défaut**, et un test le tient (`les_voiles_sont_allumes_par_defaut`) :
un interrupteur de diagnostic laissé à « off » ferait disparaître les anneaux du
jeu entier, et ça ne se remarquerait que bien plus tard.

### P.2 nonies — Colonne de cas dans la galerie des ceintures

Treize cas, une molette : atteindre le dernier demandait treize crans. La vue a
maintenant la **colonne de gauche** — un clic par cas, et le catalogue visible
d'un coup d'œil.

**Rien de neuf n'a été écrit pour ça.** `ecran::liste` existait déjà (colonne
bornée à un dixième, hauteur de ligne adaptative, item sous le curseur,
troncature mesurée à la vraie police, bouton de repli), et c'est sa **troisième**
vue après les composants et la vue système.

Une seule chose lui manquait, ajoutée **au module** et pas à la vue :
`bord_droit(ecran, repliee)`, l'abscisse à partir de laquelle dessiner sans
passer sous la colonne. Une galerie occupe tout l'écran : son viewport 3D doit
rétrécir d'autant, sinon la colonne recouvre une bande de l'image au lieu de la
côtoyer. Repliée, elle rend toute la largeur.

Test `le_bord_droit_libere_la_place_de_la_colonne` : la place laissée couvre
toute la colonne, ne dépasse pas sa part, et tombe à zéro une fois repliée.
Trois red-checks (marge retirée, repli sans effet, marge démesurée).

### P.2 octies — ⭐ La cause : `Projection` n'est pas la projection

**Le symptôme dit tout, une fois bien énoncé** : « les PETITS débris ENFLENT
quand la caméra approche de l'axe Y ». Enfler, c'est le clamp sub-pixel qui
s'emballe — pas un problème de forme, un problème de **division**.

```glsl
float px_par_unite = Projection[1][1] * viewport_h * 0.5 / max(clip.w, 0.0001);
float taille_px = taille * px_par_unite;
if (taille_px < px_min && taille_px > 0.0) {
    taille *= px_min / taille_px;   // -> l'infini quand taille_px -> 0
```

**L'uniform `Projection` de macroquad n'est pas la projection.** Vérifié dans la
source (`camera.rs`, `Camera3D::matrix`) : c'est
`Mat4::perspective_rh_gl(...) * Mat4::look_at_rh(...)`, la matrice **vue-projection
combinée**, poussée telle quelle sous ce nom (`quad_gl.rs:780`). Lire
`Projection[1][1]` comme un `1/tan(fovy/2)` ne vaut donc que pour **une**
orientation de caméra.

En visant le long de Y, la vue envoie le Y du monde sur la **profondeur** : ce
coefficient tend vers zéro, `taille_px` avec lui, et le facteur de correction
`px_min / taille_px` part à l'infini. Les petits débris enflaient en gros carrés
— **et seulement près de cet axe**, ce qui explique qu'on ne l'ait jamais vu de
trois-quarts.

**Le remède : mesurer au lieu de déduire.** On projette le centre du caillou et
un point décalé de `taille` selon `cam_up`, et on lit l'écart en NDC vertical
(`cam_up` tombe sur la verticale de l'écran par construction, et NDC y couvre
`viewport_h` pixels sur [-1, 1]). Aucune hypothèse sur la disposition de la
matrice, donc aucune orientation privilégiée.

**Garde-fou** : `aucun_shader_ne_lit_un_coefficient_de_la_matrice_vue_projection`
(`impostor.rs`) balaie `src/shaders/` et refuse tout accès par élément à cette
matrice. Red-check : remettre l'ancienne formule le fait rougir.

⚠️ **Ce que ça dit sur la méthode.** Le symptôme exact — « les PETITS enflent »
— désignait le coupable en une ligne. Les formulations précédentes (« gros
carrés », « ça grossit ») m'ont fait chercher du côté de la FORME et du LOD, où
j'ai trouvé de vrais défauts (demi-tailles, marge des lobes, seuil) mais pas
celui-là. **Demander « qu'est-ce qui change, exactement ? » aurait coûté un
aller-retour au lieu de quatre.**

### P.2 septies — Le repère caméra dégénère sur l'axe Y (latent, corrigé)

Cherché après « le bug n'apparaît que caméra proche de l'axe Y » — Y étant la
normale des disques. Trouvé un vrai défaut latent, **mais qui n'explique
probablement pas le symptôme** (voir la réserve plus bas).

`let right = forward.cross(Vec3::Y).normalize();` — recopié **quatre fois**
(`camera.rs`, `galerie.rs`, `galerie_disques.rs`, `galerie_etoiles.rs`). Le
produit vectoriel s'annule quand la visée est colinéaire à Y, et `normalize`
d'un vecteur nul rend des **NaN**. Tout ce qui fait face à la caméra s'appuie sur
ce repère : un NaN et les quads des champs de débris partent en vrille — un
défaut qui ne se voit QUE près de cet axe, jamais en trois-quarts.

Une seule fonction désormais, `camera::base_camera`, qui bascule sur Z comme
verticale de référence quand la visée frôle Y. Quatre red-checks (ancienne
recette, seuil jamais franchi, visée nulle, `up` non orthogonalisé).

⚠️ **Réserve honnête.** La vue orbitale **bornait déjà** son tangage à ±1,4 rad
(80°), où `|forward × Y| = cos(1,4) = 0,17` : le calcul y était encore bien
conditionné. La dégénérescence n'était donc atteignable que par les galeries, qui
placent leur caméra à la main **sans borne**. C'est une correction juste et une
duplication de moins, pas nécessairement LA cause de ce qui se voit à l'écran.

### P.1 bis — Le catalogue n'a pas été retouché, et `atmo` a changé de sens

Recensement des **161 entrées** (132 telluriques + 29 gazeuses) fait le
2026-08-08, après la coquille. Le catalogue n'est pas un continuum mais une
**échelle à 4 niveaux**, trois constantes partagées couvrant les telluriques :

| `\|atmo\|` | monde | densité obtenue |
|---|---|---|
| 1.074 (`bleu`) | 46 humides | 0,97 |
| 0.283 (`voile`) | 41 glacés | 0,25 |
| 0.141 (`sec`) | 33 secs | 0,13 |
| 0.000 | 9 sans air | aucune coquille |

Plus ~7 valeurs propres (Vénus 0.276, Io 0.248, Titan 0.313, Amber 0.330, magma
0.398, Hycean 0.587). Gazeuses : 0.037 à 0.527, **aucune à zéro**.

**Première correction : Mars** (2026-08-08, vue à l'écran). Elle valait `* 0.10`,
soit `|atmo| = 0.105` — **la plus faible du catalogue hors zéro, sous même la
constante des mondes secs** — et sortait sans atmosphère visible. Ce chiffre
réglait l'ancien halo, peint *sur* la surface : le monter aurait délavé le
contraste clair/sombre (oxyde de fer contre basalte) qui fait lire la planète.
La coquille, elle, vit surtout **dehors** — au centre du disque elle ne laisse
que ~4 %. La raison de le garder bas a donc disparu avec l'ancien modèle.

Portée à `* 0.40` (densité 0,378). Ce n'est **pas** le rapport des pressions :
Mars fait 0,6 % de la Terre, à ce compte elle serait nue. C'est ce que Mars
*montre* — une brume chargée de poussière, franchement visible au limbe depuis
l'orbite, dont la hauteur d'échelle vaut **2,4 fois celle de la Terre rapportée
au rayon**. Mince en densité, haute en géométrie. Test
`mars_a_une_brume_visible_mais_tenue` : il **encadre** Mars entre « Badlands »
(monde sec générique) et « Terre », deux entrées écrites indépendamment d'elle,
plutôt que de recopier la formule de densité qui vit dans le GLSL.

**`atmo` voulait dire « teinte × éclat du liseré » — un bouton de rendu.** Il
pilote maintenant la densité d'une vraie coquille. Les valeurs ont été choisies
sous l'ancien sens.

**Vénus le prouve** : `|atmo| = 0.276`, parmi les plus basses du catalogue, pour
l'atmosphère la plus épaisse du système solaire. C'était **juste** avant (un
liseré fort par-dessus un voile déjà opaque rendait mal, donc on l'a baissé) ; ça
ne l'est plus. Elle ne s'en sort aujourd'hui que parce que la densité intègre
`voile` (0,25 + 0,74 = 0,99). Titan pareil.

**Celle que le voile ne rattrape pas : Hycean.** Océan global sous une enveloppe
épaisse de vapeur (modelé sur K2-18b), elle tombe à 0,71 — **sous la Terre à
0,97**. Elle devrait être la plus dense du catalogue.

Des 9 zéros, quatre sont justes par définition (Lune, Fer/Mercure, Diamant,
Chthonien — « atmosphère arrachée » est le sens même du mot). **Ash
(volcanique)** est douteuse : des cendres en suspension supposent de l'air.
Primal et Lave se discutent.

⚠️ **Ne rien retoucher avant d'avoir regardé.** Ces valeurs réglaient un autre
effet ; les refaire au raisonnement serait deviner deux fois de suite.

Le défaut de structure, si le rendu le confirme : **un seul nombre porte deux
sens** — l'éclat de la teinte et la quantité d'air. Vénus demande *sombre et
épais*, ce que `atmo` seul ne sait pas dire. La correction propre est un champ
`atmo_densite` distinct, dérivé par défaut du groupe climatique pour que
l'essentiel des 161 entrées ne bouge pas. Édition à l'échelle du catalogue :
signalée, pas engagée.

### P.2 — Ceintures : mesuré (2026-08-08). ⚠️ **Il n'y a rien à optimiser là.**

**La mesure d'abord, comme annoncé** — et elle contredit ce que ce document
disait. Test `budget_de_dessin_des_ceintures_solaires` (`disque/mod.rs`) : il
appelle les **vrais** constructeurs de maillage sur les **vraies** configs du
préset (`genese::presets::ceintures_solaires`, devenue source unique), et compte.

| Famille | particules | lots (= `draw_mesh`) |
|---|---|---|
| Astéroïdes | 900 | 3 |
| Kuiper | 2 000 | 5 |
| Épars | 900 | 3 |
| Oort | 2 600 | 7 |
| **Total** | **6 400** | **18** |

**18 appels de dessin et 25 600 sommets par frame.** C'est du bruit pour
n'importe quel GPU. Les buffers sont figés (`construire_lots`), l'orbite est
calculée dans le vertex shader (`omega = sqrt(gm/r³)`) : rien ne remonte au CPU.

**Ce que ce document affirmait était faux.** Il disait « le vrai gisement est le
culling, le nuage de Oort surtout, dessiné même à 1 UA ». Le nuage de Oort coûte
**7 appels de dessin et 10 400 sommets**. Le culler ne rendrait rien de
mesurable, et il faudrait payer un test de visibilité par frame pour ça. La
raison de l'erreur : « 2 600 corps » sonne gros, et le chiffre n'avait jamais
été rapporté à ce qu'un GPU avale. C'est exactement ce que « mesurer d'abord »
devait éviter — et l'avoir écrit sans mesurer a failli coûter une optimisation
inutile.

**Où le coût peut réellement être**, et qui ne se compte pas en appels de dessin :

- **Le remplissage de pixels des voiles.** Un anneau planétaire n'a que 960
  quads (10 bandes radiales × 96 segments), mais il peut couvrir tout l'écran,
  et chaque pixel paie `disque_voile.frag.glsl` — **155 lignes de bruit
  procédural**, deux fois (moitié avant + moitié arrière). Saturne vue de près
  est le cas à surveiller, pas le nuage de Oort.
- Les quatre ceintures solaires, elles, **n'ont aucun voile** (le test le
  vérifie) : ce sont des nuages de cailloux, pas des anneaux.

**Prochaine mesure, et elle demande l'écran** : FPS en vue système, caméra
serrée sur Saturne (anneau plein cadre) contre caméra au large. Un écart net
désigne le fill rate du voile ; pas d'écart, et P.2 se réduit à la finition
visuelle. Je ne peux pas la prendre — le jeu a besoin d'une fenêtre.

### P.3 — ⚠️ Les éjections stellaires existent déjà

**Vérifié dans le code avant d'ouvrir le chantier**, et c'est important :

| Ce qu'on croyait manquant | Réalité |
|---|---|
| Éjections coronales | `soleil/eruptions.rs` : `Boucle` avec cycle complet **Montée → Tenue → Rupture → Retombée**, corde de flux (`apex_vel`), rendue en billboards additifs (`soleil/rendu.rs`) |
| Taches | `Tache` avec durée de vie, poussées dans `spots[8]` |
| Motif de surface | granulation `fbm` (`soleil.frag.glsl` l. 87), champ magnétique (l. 97) |
| Couronne | trois types : halo, jets bipolaires, vent stellaire |

Et `Astre::update` **est bien appelé** en vue système (`systeme/mod.rs:393`).

**Donc le défaut n'est pas l'absence : c'est que ça n'arrive pas à l'écran.**
L'hypothèse la plus probable est l'**échelle** — à distances réelles le Soleil
fait 1,1 U de rayon quand Neptune orbite à 1 440 U, si bien que boucles et
granulation tombent sous le pixel. C'est le **cinquième** défaut de ce type dans
le projet (calotte, villes, atmo de Titan, texture zonale, ventre) : *la donnée
est juste, la plomberie ou l'échelle ne suit pas*.

**Travail** : d'abord **constater** (galerie d'étoiles contre vue système), puis
décider — LOD sur `px_rayon` comme les géantes, ou exagération d'échelle assumée.
Ne **pas** réécrire ce qui existe.

Reste ensuite un vrai manque possible : la **variété d'une étoile à l'autre**
(même graine de granulation partout ?), à vérifier en galerie.

**479 tests, 0 avertissement clippy.**

### Ce qui vient d'être fait (2026-08-05 → 06)

| Chantier | Résultat |
|---|---|
| **Revue P1** | 117 → **0** avertissement. 90 éléments morts triés, dont **77 exercés par les tests** — la mesure précédente était fausse. A révélé un vrai défaut : `MASSES_TERRESTRES` était documentée « source unique » et **personne ne la lisait**. |
| **Réglages persistés** | `reglages.json`, GAMMA au curseur, tramage retiré. |
| **Zoom** | Plancher à `0,01` (×128 000), plans de coupe suivant la distance. |
| **Texte** | Deux défauts distincts : glyphes absents de la police (157 caractères), puis **tuiles noires** dues au repaquetage de l'atlas. |
| **Starship 1/5 → 5/5** | Coque, 6 Raptor, jupe moteur, 4 volets, bardage de tuiles, hublots, exposé debout. |

## L'interface de jeu (I.0 à I.5 — faites)

Conception écrite le 2026-08-04 :
**[`docs/conception/interface.md`](docs/conception/interface.md)**. À lire avant
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
dettes : [`docs/conception/pixel_art.md`](docs/conception/pixel_art.md).

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

## Le système solaire et le catalogue

Le preset solaire (`genese/presets.rs`) tire **déjà** ses apparences du
catalogue de la galerie (`preset_tellurique` / `preset_gazeuse`, qui **paniquent**
sur un nom inconnu — il n'y a pas de repli silencieux). Deux corrections le
2026-08-05 :

**1. Le catalogue est devenu déterministe** — c'était la cause racine.

`catalogue_telluriques()` / `catalogue_gazeuses()` tiraient une graine **et** une
taille au sort à chaque construction. Conséquence : la « Terre » de la galerie
n'était pas une référence, mais une planète différente à chaque ouverture de
l'écran. Le système solaire ne pouvait donc **pas** lui ressembler — il n'y avait
rien à quoi ressembler.

Graine et taille sont maintenant déduites du **nom du preset**
(`genese::graine_de_nom`, FNV-1a, et `ClasseTaille::rayon_pour`). Le brassage n'a
pas disparu : il a changé de place, dans la galerie, où la touche **G**
incrémente une `variation`. À `variation == 0`, le décalage est nul — **la vue
par défaut de la galerie EST le catalogue canonique**, ce qui est ce à quoi le
système solaire s'aligne.

Les corps à preset **unique** prennent donc la graine du catalogue et sont
identiques à leur vignette de galerie. `fige(…, "<corps>")` ne subsiste que là où
un preset est **réutilisé** (Callisto et Obéron tirent tous deux « Lune »), sans
quoi ils seraient rigoureusement jumeaux.

**Gain de côté** : sans `gen_range`, le catalogue **se teste enfin**. Cinq tests
neufs, dont « chaque preset demandé par les systèmes scénarisés existe » — jusqu'ici
garanti par un seul `panic!` au lancement, écran noir à la clé.

**2. Appariements revus**, pour cesser de recycler le même preset :

| Corps | Avant | Après | Pourquoi |
|---|---|---|---|
| Ganymède | `Lune` | `Crevasse` | ses sillons (sulci) |
| Callisto | `Carbone` | `Lune` | le corps le plus cratérisé connu |
| Ariel | `Boule de neige` | `Supraglacial` | terrains fracturés brillants |
| Pluton | `Boule de neige` | `Ice Dunes` | dunes vues par New Horizons |

⚠️ **Fixtures recopiées** : les listes de noms des tests (`DEMANDES_TELLURIQUES`,
`DEMANDES_GAZEUSES`, `CORPS`) sont recopiées de `presets.rs`. Un nom ajouté là-haut
sans l'être ici est **moins couvert**, il ne devient pas faux.

⚠️ **Piège évité de justesse** : `chaque_taille_reste_dans_les_bornes_de_sa_classe`
déduisait d'abord ses bornes de `rayon_pour`, la fonction même qu'il teste —
l'attendu bougeait avec elle et le test ne pouvait pas échouer. Il lit maintenant
`bornes_terrestres()` à la source. C'est le mode de défaillance récurrent du
projet : *un test qui mesure autre chose que ce que son nom annonce*.

⚠️ **Bug de LOD corrigé** (`rendu.rs`) : `planete::set_viewport_h` n'était réglé
**que par la galerie**. Dans la vue système, `px_rayon` retombait sur
`screen_height()` — donc en mode pixel art, les planètes étaient ombrées avec le
détail d'un plein écran alors qu'elles sont dessinées dans une cible deux fois
plus petite. Les débris avaient déjà leur correction, pas les planètes.

## Cohérence du catalogue de planètes

Passe du 2026-08-05, déclenchée par « Mercure a de la glace ». **Trois défauts,
dont deux dans le shader** — la donnée, elle, était déjà cohérente.

| Défaut | Où | Cause |
|---|---|---|
| Banquise sur des mondes sans climat (Mercure, Lune, Carbone, Vénus) | `planete.frag.glsl` | `calotte = 1` était censé dire « aucune », mais le seuil est perturbé par un bruit de ±0,30 et `froid` atteint déjà 1 au pôle : `smoothstep(1, 1.05, 1.5)` rendait 1 |
| Lumières de ville sur les cailloux morts | `apparence.rs` + shader | `villes` vaut **1 par défaut** et le shader n'excluait que lave et voile — aucune condition d'air ou d'eau |
| (faux positif) Rivières sans eau sur `Crevasse` | — | ce sont des **coulées de lave** (`riv_lave`) : c'est le test qui était faux |

`calotte >= 1` est maintenant un **sentinelle exact** dans le shader, et
« ni air ni eau ⇒ pas de villes » est appliqué **dans la donnée** (le `push` du
catalogue), là où ça se teste.

Six tests de cohérence neufs, tous red-checkés. Ils tiennent des invariants, pas
des valeurs : *pas de climat ⇒ pas de banquise*, *un monde annoncé gelé porte
vraiment de la glace*, *un monde habitable garde ses villes* (sans quoi une règle
trop large les éteindrait partout sans qu'on le voie), *une rivière coule d'eau
ou de lave*.

⚠️ Ces tests ne sont possibles que **depuis que le catalogue est déterministe** :
avant, il exigeait le contexte graphique.

## Revue de code du 2026-08-05 — liste à traiter

Établi en passant tout le catalogue de planètes au crible et en dépouillant
`cargo clippy`. **Par ordre d'urgence.**

### P0 — défauts réels

| # | Quoi | État |
|---|---|---|
| 1 | Banquise sur les mondes sans climat (`calotte = 1` n'était pas un sentinelle exact) | ✅ corrigé |
| 2 | Villes sur les cailloux morts (`villes = 1` par défaut, shader sans condition d'air) | ✅ corrigé |
| 3 | **Titan sans atmosphère** : voile orange épais mais `atmo = 0`, donc aucun halo de limbe | ✅ corrigé |
| 4 | Géantes gazeuses avec `villes = 1` et `relief = 0,35` — inertes (gardées par `type_p`), latentes | ✅ corrigé |
| 5 | `large_enum_variant` sur `ecran/objet.rs:14` — `Apparence` (376 o) contre 48 o pour la variante étoile : toute copie de `Specimen` traînait les 376 o. Boxée. | ✅ corrigé |

### P1 — le bruit qui cache les vrais défauts ✅ TERMINÉ

C'était **la leçon de la session** : la palette Lospec était refusée avec une
ligne de console, invisible au milieu du bruit. Tant que `clippy` sortait 117
avertissements, le 118ᵉ ne se voyait pas.

**117 → 0.** `cargo clippy --all-targets` est propre.

| # | Quoi | Volume | État |
|---|---|---|---|
| 6 | Imports inutilisés | 9 | ✅ |
| 7 | Code mort | **59** | ✅ trié |
| 8 | `assertions_on_constants` — légitimes, elles gardent une constante | 12 | ✅ `#[allow]` commenté sur les 6 modules de test |
| 9 | Style : `field_reassign_with_default`, `needless_range_loop`, `doc_lazy_continuation`, `too_many_arguments`, `type_complexity`, `useless_conversion`, `needless_update`, `clone_on_copy`, `manual_checked_ops`, `single_char_add_str`, `wrong_self_convention`, `unused_*` | ~50 | ✅ |

#### ⚠️ La mesure précédente était fausse

STATE.md disait : « 55 éléments morts pour la cible binaire, 57 avec les tests —
donc il est bien inutilisé ». **Non.** En comparant vraiment les deux cibles :

```
morts pour le binaire ................ 90
morts pour le binaire ET les tests ... 13
```

**77 des 90 sont exercés par les tests** — ils ne sont pas morts, ils sont
*écrits avant leur consommateur*. `dead_code` n'a aucune notion de « utilisé par
les tests » : il compte sur la cible binaire, et tout ce que seuls les tests
appellent y ressort mort. La conclusion inverse aurait mené à supprimer du code
testé et qui marche.

#### Ce qui a été fait, par groupe

- **`chantier.rs` et `symetrie.rs`** : `#![allow(dead_code)]` **au module**, avec
  la raison en tête de fichier (l'écran d'assemblage du Lot 4 est le
  consommateur). 27 éléments.
- **Le reste** : `#[allow(dead_code)]` **par élément**, chacun avec sa raison sur
  la même ligne — catalogue de l'assembleur, économie du chantier (D-INT-2),
  interaction/désignation, mesures de géométrie tenues par leurs tests.
- **Supprimé** : `Camera::zoom_max()`, sans aucun appelant, et intestable
  (`Camera::new` exige un contexte GL). Ajoutée la session d'avant, jamais
  branchée.
- **`Palette::depuis_hex`** passée en `#[cfg(test)]` : le jeu lit des fichiers.

L'autorisation est **volontairement locale**. Contrôle rouge : une fonction morte
neuve ajoutée dans `orbite.rs` sort **un** avertissement, seul et visible. C'est
tout l'objet de la manœuvre.

#### 🔴 Un vrai défaut trouvé sous le bruit : `MASSES_TERRESTRES`

La table des masses planétaires se documentait comme la **source unique** — et
**personne ne la lisait**. Les masses étaient écrites à la main à **trois**
endroits : la table, chaque appel `poser(...)` du preset, et `parent_de` dans les
tests. C'est exactement le défaut que la table existait pour empêcher, et le test
de Kepler ne pouvait pas le voir : il vérifiait **sa propre copie**.

Corrigé par `masse_monde(nom)` — le pendant de `rayon_monde` — que le preset et
les tests appellent tous les deux. Contrôle rouge : mettre Jupiter à 31,78 au
lieu de 317,8 fait tomber `les_periodes_de_lunes_suivent_kepler` (Lune/Io = 4,91
au lieu de 15,44). Avant, cette même corruption passait au vert.

C'est la justification de la revue en une ligne : **le défaut était visible
depuis le début, noyé dans 117 avertissements.**

#### Deux choix assumés

- **`too_many_arguments` (7 sites) : autorisés, pas refactorés.** Ce sont des
  listes cohérentes — éléments orbitaux, uniforms de shader, dimensions d'une
  poutre. Un type `ElementsOrbitaux` serait plus propre et touche **tous** les
  presets : c'est un chantier à part, pas un à-côté de passe de nettoyage.
- **Ce qui reste candidat à la suppression** (mort dans les deux cibles, gardé
  faute d'une décision qui n'est pas la mienne) : `police::mesure`,
  `sprites::presente`, `Systeme::mode`, `Apparence::avec_eyeball` (remplacée par
  `avec_eyeball_zones`), `Apparence::avec_cryo`, `generateur::preset_isv`
  (remplacée par `preset_isv_fixe` + `preset_isv_equipage`), `vaisseau::voile` et
  `Peintre::voile`. Chacune porte sa raison en commentaire.

### P2 — dettes déjà inscrites

~~D-PARAM-1~~ (soldée), D-PIX-1/3/4 (D-PIX-5 sans objet), `briques.rs`/`vaisseaux.rs`
non déclarés dans `ecran/mod.rs`, hash CPU/GPU dupliqué dans 4 fichiers.

### Ce que la revue a appris

**Deux de mes trois règles candidates étaient trop strictes, et le catalogue avait
raison.** Les rivières de `Crevasse` sont des coulées de **lave** (`riv_lave`), et
la végétation de `Lichen` et `Fungi` prend son humidité dans l'**air**, pas dans
un océan. Ce sont des cas limites délibérés. Les tests encodent désormais la
règle juste — *une rivière coule d'eau ou de lave*, *la végétation veut de l'eau
ou une atmosphère* — et chacun vérifie que **le cas limite existe encore**, sans
quoi la règle deviendrait vide sans que rien ne le dise.

## ⚠️ Piège : matériau partagé + lots de dessin (D-GPU-1)

**Toutes les planètes partagent un seul matériau** : `mat_corps()` rend des clones
du même template, et les clones partagent le pipeline. `mat.set_texture(...)`
n'attache donc **pas** une texture à *ce* dessin — elle l'écrit dans le pipeline,
et macroquad ne l'applique qu'au **vidage du lot**.

Conséquence : sans vidage explicite, *le dernier corps dessiné impose ses
textures à tous les précédents*. Dans le système solaire, le dernier est une
tellurique, donc `zonal = tex_vide()` (canal vert à 0) → `band = 0` →
`base = belt` → **les quatre géantes sortaient d'un brun uni**.

La galerie y échappait **par accident** : elle change de viewport à chaque
cellule, et chaque `set_camera` vide le lot. C'est pourquoi le défaut ne se
voyait que dans la vue système — et pourquoi le chercher dans les données ne
pouvait pas aboutir (le profil zonal CPU était parfait, c'est testé).

Corrigé par un `get_internal_gl().flush()` après chaque `draw_mesh` de planète.

⚠️ **Le piège reste ouvert pour toute texture par objet.** `terrain` court le même
risque sur les telluriques ; il n'est masqué que parce qu'elles partagent souvent
`tex_vide()`. Toute nouvelle texture par astre doit être posée **entre deux
vidages**.

⚠️ **Non testable** : c'est de l'état GPU, invisible à `cargo test`. Le garde
honnête serait une comparaison d'images rendues, que le projet n'a pas. C'est le
**quatrième** défaut de la session où une valeur juste n'atteignait pas le shader
(calotte, villes, atmo de Titan, texture zonale) : la donnée était bonne, la
plomberie non.

⚠️ **Coût** : le vidage est fait à chaque planète, donc plus d'appels de dessin
qu'avant. Invisible aux effectifs actuels ; si la vue système rame un jour, ne
vider que sur **changement** de textures plutôt qu'inconditionnellement.

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

## Chantiers en pause

| Chantier | Où on en est | Reprendre par |
|---|---|---|
| **Assembleur** (Lot 4) | Lots 1–3 clos, Lot 4 conçu, pas commencé | `conception/assembleur.md` §10, puis L4.1 |
| **Starship** ✅ **5/5** | Coque, 6 Raptor, jupe, 4 volets droits, bardage de tuiles (fût **et** ogive) et 2 hublots. Exposé **debout**, moteurs en bas. | Vaisseau **mobile** (transferts entre astres) |
| **Endurance** | Rien | `conception/stations.md` Partie D bis |

## L'assembleur, en résumé

L'ISV est **clos** (`docs/suivi/stations.md` §C.29). Les mégastructures et le
Starship sont journalisés en **Partie F** du même fichier.

| Lot | Contenu | État |
|---|---|---|
| **Lot 1** (L1.1–L1.8) | Fondations : catalogue à source unique, balayage des variantes, déterminisme, rayons déclarés, **capsules**, overlays, mesureur | ✅ |
| **§9** « le boudin » | Enveloppe **rectangle** pour les plaques, distances exactes | ✅ |
| **Lot 2** (L2.1–L2.5) | Modèle du `Chantier` : identifiants stables, `retirer`, undo/redo, palette, sérialisation | ✅ |
| **Lot 3** (L3.1–L3.3) | `pose_prevue` (fantôme), `sous_arbre`, désignation port/pièce | ✅ |
| **Lot 4** (L4.1–L4.5) | **L'écran d'assemblage** | ⏸ conçu, pas commencé |
| **Lot 5** | Sauvegarde disque (L5.1), overlay §8.5 (L5.2) | à faire |
| **Lot 6** | Arbitrage L1.4 (L6.1), composites `figer` (L6.2), `PoutreBout` (L6.3) | à faire |

Le modèle répond aux quatre questions de §8 et **aucun écran ne le consomme
encore**. Premier geste de L4.1 : réexporter `Chantier` depuis `vaisseau`.

⚠️ Le Lot 4 est le **premier lot majoritairement non testé** (§6.6 : pas de test
de rendu) — comme l'interface de jeu. La discipline change de forme : pousser
hors du code de dessin tout ce qui **se décide**, vers des requêtes
red-checkables. Ce qui reste non testé ne doit être que *où le rectangle se
pose*, jamais *ce qu'il signifie*.

Régénérer le catalogue de fils : `FILS=1 cargo test --release le_catalogue_des_fils`.

## Décisions prises (ne pas les rouvrir sans raison)

- **Pièce d'abord** (façon KSP) : palette permanente à gauche, ports compatibles
  qui s'allument. Pas « port d'abord ».
- **Bac à sable libre** : pas de plafond de coût.
- **Capsules** (et rectangles pour les plaques) plutôt que sphères — et **pas**
  d'AABB/OBB.
- **`SousEnsemble` se sérialise en pièces cuites**, pas en recette rejouée.
- **Pas de roulis libre** autour du port (§10.6).
- **Les centres de Stanford et d'Elysium sont séparés** : ils ne sont pas
  destinés à se ressembler (§F.4).
- **`Composant::nom()` rend la famille**, pas famille + variante : la colonne
  fait 100 px, et la vitrine fait déjà défiler les variantes.

## Questions ouvertes

- **Sortir de l'assembleur perd le vaisseau** (§10.7b) : Échap efface le travail
  en cours. **À trancher avant L4.1.**
- **Les mégastructures à taille réelle** : ~2,85 pièces et ~2 850 sommets par
  unité de rayon d'anneau, soit ~1 140 pièces pour l'Elysium de 1,8 km contre 45
  pour l'ISV (§F.3). L'assemblage pièce à pièce ne passera pas à l'échelle.
- **Membrures à axe courbe** : aucune primitive du projet n'en trace, et les
  bras des mégastructures en ont besoin pour être reliés entre eux (§F.1).

## Méthode (ce qui a marché, à ne pas perdre)

- **Red-check systématique** : tout test ajouté est cassé volontairement sur sa
  cible pour vérifier qu'il rougit. **Sauvegarder le fichier dans le scratchpad
  avant de le saboter**, et le restaurer par copie — jamais par git.
  Trois formes du même piège reviennent : un test qui **inspecte l'état** au
  lieu d'emprunter le vrai chemin de code, qui **fixe une variable** du scénario
  au lieu de la sonder (`libres()[0]`), ou qui se compare à **sa propre
  constante** (§F.7). La parade est toujours la même : muscler le test dès qu'un
  sabotage ne mord pas, jamais l'admettre « suffisamment proche ».
- ⚠️ **Un test rouge ne veut pas dire que le code a tort** (L2.4).
- ⚠️ **Un proxy qui ne tient que dans un régime n'est pas une mesure** (§F.2).
- ⚠️ **Une propriété d'accord entre deux appelants d'un même calcul ne garde pas
  ce calcul — elle garde son unicité.** Il faut un **second** test qui pinne la
  valeur.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent
  d'une valeur qui en a deux (`docs/suivi/stations.md` §C.29).
- **Une cote qui se règle contre une autre pièce ne tombe pas sur `Profil`** —
  vu quatre fois. Réponse : mise à l'échelle **géométrique**, ou rester sur la
  grille et s'arrêter au cran qui tient (§F.4).
- **Brique d'abord, assemblage ensuite.** Jugée seule avant d'être posée.
- ⚠️ **La tolérance d'un test doit être plus petite que l'erreur qu'il cherche.**
  Vu le 2026-08-06 : le placement d'un hublot était borné par l'écart tangentiel
  de la paire (0,62) alors que l'erreur à détecter valait 0,14 — poser le vitrage
  au mauvais rayon passait au vert. La parade : mesurer sur un axe où le bruit
  n'existe pas (ici, la projection sur l'azimut), pas élargir la borne.
- ⚠️ **Comparer une copie à une copie ne prouve rien.** Quatre fois dans la même
  session : masses recopiées dans le test, corde *supposée* axiale au lieu d'être
  lue, ventre auto-référent (bardage comparé aux volets, jamais à une référence),
  commentaire de `braquage` que rien ne vérifiait. Toujours comparer à **la
  source**.
- ⚠️ **`dead_code` ne sait pas ce qu'est un test.** Il compte sur la cible
  binaire : tout ce que seuls les tests appellent y ressort mort. Mesuré le
  2026-08-05 — 90 morts pour le binaire, **13** pour les deux cibles. Conclure
  sans faire la différence mène à supprimer du code testé et qui marche.
- **Mesurer avant d'affirmer.**
- **Mettre à jour les `.md`** à chaque étape — consigne debout de l'utilisateur.

## Dette des bouche-trous de l'interface

Consigne de l'utilisateur (2026-08-04) : **poser un bouche-trou** pour ce qui
n'est pas encore fait, et **inscrire ici** ce qu'il reste à faire. Un
bouche-trou non listé est un mensonge à l'écran — il a l'air fini.

**Quatre sur cinq sont soldés.** Il ne reste que D-INT-2, l'économie, qui est un
chantier de conception à part entière.

| # | Bouche-trou | Où | Ce qu'il faut à la place |
|---|---|---|---|
| **D-INT-1** | ~~Pastille de catégorie~~ — **soldée** : la pastille prend la **teinte réelle** du corps (`Astre::teinte`, tirée de l'apparence), relevée pour rester visible sur fond nuit. Une retouche d'apparence se voit aussitôt dans la colonne. Reste un disque, et c'est voulu : à 6–14 px de diamètre, un rendu de planète serait de la bouillie. | — | — |
| **D-INT-2** | **`Tresorerie` figée** : les quatorze quantités sont écrites à la main, rien ne les fait bouger. | `ecran/bandeau.rs::Tresorerie` | L'économie : production, consommation, coûts, recherche. Chantier de **conception** à part entière ; la barre a été écrite pour pouvoir l'attendre (`conception/interface.md` §2.2b). |
| **D-INT-3** | ~~Vignette du panneau~~ — **soldée** : l'astre est **réellement rendu** (`ecran/vignette.rs`) dans une cible 192×192 avec son propre tampon de profondeur, sous l'éclairage du système, caméra placée du côté éclairé et reculée pour laisser tenir un anneau. | — | — |
| **D-INT-4** | ~~Nom du système~~ — **soldée** : dérivé de l'étoile hôte (`Systeme::nom_systeme`). Repli sur le libellé de génération pour un système engendré, dont les étoiles n'ont pas de nom propre. | — | — |
| **D-INT-5** | ~~Noms de presets non gardés~~ — **soldée** : `ajouter_planete`/`ajouter_planete_autour` exigent un `Option<&'static str>`, comme `ajouter_lune_preset`. Chaque appelant doit dire `Some("Terre")` ou `None`. Deux tests lisent la **source** pour interdire le retour du `sys.nommer` après coup — seul moyen, faute de pouvoir bâtir un système en test. | — | — |

## Dettes préexistantes

- **Le hash CPU/GPU** (traité le 2026-08-02) : `cargo clippy` ne compilait pas à
  cause d'`approx_constant` sur le multiplicateur du hash de `terrain.rs`.
  **Suivre le conseil de clippy aurait introduit un défaut** — `FRAC_1_PI`
  décale la valeur d'**1 ULP**, et le `fract` final amplifie ça en un bruit
  entièrement différent : la géographie CPU cesserait de correspondre au shader,
  sans aucun signal. Constante nommée + `allow` avec l'explication.
  **Dette restante** : la même valeur vit dans 4 fichiers (1 Rust + 3 GLSL) et
  rien ne les tient d'accord → `docs/suivi/planetes.md` §7.
- ⚠️ **Cette entrée disait « les 86 avertissements clippy ne contiennent aucun
  défaut réel ». C'était faux**, et la revue P1 (2026-08-05) l'a montré :
  `MASSES_TERRESTRES` se documentait comme la source unique des masses
  planétaires et **personne ne la lisait** — les masses étaient recopiées à trois
  endroits, et le test de Kepler vérifiait *sa propre copie*. Le défaut était
  visible depuis le début, noyé dans le bruit. **`clippy` est à 0 aujourd'hui**,
  et c'est ce qui rend le prochain visible.
- **`src/ecran/briques.rs` et `src/ecran/vaisseaux.rs` sont commités mais pas
  déclarés** dans `ecran/mod.rs` : ils ne compilent pas. À supprimer ou à
  rebrancher.
- Dette de **L1.4** : ~19 variantes sous-déclarent leur `rayon_local` (cadrage
  caméra ; 2 vues sur 35 bougeraient, de 1 % et 2 %). `MARGE_RAYON = 1.40`
  marque la dette dans le code. **Se solde en Lot 6.**
- Dette de **L1.6** : `peut_poser` n'a aucun test dédié, faute de consommateur.
  Son vrai consommateur est le code couleur de §8.4 — donc **Lot 4**.
- `GenrePort::PoutreBout` n'est posé sur aucun composant — variante morte,
  **Lot 6**.
