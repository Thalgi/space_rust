# Suivi — Ceintures, anneaux et débris

> Journal du chantier **P.2** : les disques de débris (ceintures d'astéroïdes,
> anneaux planétaires, nuage de Oort) et leur galerie.
> Conception : [`../conception/ceintures.md`](../conception/ceintures.md).
> Point de reprise : [`../../STATE.md`](../../STATE.md).
>
> Ce document a été **extrait de `STATE.md` le 2026-08-10**, qui avait fini par
> tenir lieu de journal pour tous les chantiers à la fois.

---

## Ce que ce chantier a appris, en trois lignes

1. **Un shader se sonde, il ne se devine pas** — et une sonde ne change qu'une
   chose à la fois (§ P.2 quinquies).
2. **`Projection` de macroquad n'est pas la projection** mais la matrice
   vue-projection combinée (§ P.2 octies) : trois défauts en descendent.
3. En virgule flottante, **une différence de deux quantités presque égales ne
   vaut rien** : transformer une *direction*, ne pas soustraire deux *points*
   (§ P.2 decies).

---

## P.2 — Ceintures : mesuré (2026-08-08). ⚠️ **Il n'y a rien à optimiser là.**

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

## P.2 bis — Voile des anneaux : l'instrument, faute de pouvoir mesurer

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

## P.2 quater — La barre sombre en travers des étoiles protosolaires

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

## P.2 quinquies — La couture des disques proto* : trouvée par sondes, pas par déduction

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

## P.2 sexies — Les débris en « gros carrés » vus de dessus

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

## P.2 septies — Le repère caméra dégénère sur l'axe Y (latent, corrigé)

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

## P.2 octies — ⭐ La cause : `Projection` n'est pas la projection

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

## P.2 nonies — Colonne de cas dans la galerie des ceintures

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

## P.2 decies — ⚠️ Régression de ma correction : annulation catastrophique

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

## P.2 quater bis — Débris d'anneau : tellurique et géante accordées, puis affinées

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

## P.2 ter — Les débris n'avaient pas l'aspect pixel art : le blit était recopié

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

