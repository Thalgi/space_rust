# Bucket list — Galerie des étoiles

Même approche que les planètes/géantes : un modèle paramétrique (uniforms hot-reloadables)
+ un catalogue de presets nommés, visualisés dans une **Galerie des étoiles**.

Légende : `[ ]` à faire · `[x]` fait · `[~]` partiel

---

## Ce qu'on a déjà

**Types** (`etoile.rs::ProfilEtoile::aleatoire`) :
- [x] Séquence principale **O B A F G K M** (couleur corps noir, rayon/lumi croissants).
- [x] **Géante rouge** (énorme, froide, lumineuse).
- [x] **Naine blanche** (minuscule, très chaude, peu lumineuse).

**Rendu** (`soleil.frag.glsl` + `soleil/`) :
- [x] **Granulation** : domain warping animé (convection bouillonnante).
- [x] **Campfires** (points brillants épars).
- [x] **Taches** actives (assombrissement local, venant du CPU, max 8).
- [x] **Limb brightening** (anneau lumineux au bord).
- [x] **Couronne** : décroissance radiale + **spicules** (`couronne_irreg`), large/irrégulière si lumineuse.
- [x] **Éruptions/boucles** (plasma additif), taille/forme de couronne selon le type.

Paramètres actuels : `teinte` (couleur), `couronne`, `couronne_irreg`, `spots[8]`. Pas de
notion de « type » stocké dans `Soleil` (juste couleur/lumi/rayon).

---

## Types standard manquants

- [x] **Supergéante rouge** (Bételgeuse) : preset galerie (colossale, froide, lumineuse).
- [x] **Supergéante bleue** (Rigel) : preset galerie + vent stellaire (`couronne_type 2`).
- [x] **Sous-naine** (subdwarf) : preset galerie (petite/chaude).
- [ ] **Naine brune** (déjà côté gazeuses) : à relier ? (chevauchement géante/étoile).
- [x] **Protoétoile / T Tauri** : preset galerie + jets bipolaires, **et son disque
  protosolaire** depuis le 2026-09-26 (mode de couronne 6, `Variante::Proto`). Il partageait
  le mode 1 avec l'étoile à neutrons, qui n'a que faire d'un berceau de planètes.

## Étoiles particulières (le « un peu spécial »)

- [x] **Étoile à neutrons** : jets de matière bipolaires **fixes** (`couronne_type 1`).
- [x] **Pulsar** : faisceau bipolaire qui **tourne** (effet phare) + flash périodique (`couronne_type 3`).
- [x] **Wolf-Rayet** : **vent stellaire** turbulent en expansion, bleu-violet (`couronne_type 2`).
- [x] **Étoile carbonée (C)** : preset galerie (rouge sombre profond).
- [ ] **Variable céphéide / Mira** : **pulsation** (rayon + luminosité qui oscillent).
- [~] **Étoile à flares** (naine M active) : preset galerie présent ; sursauts/CME pas encore (cf. recherche).
- [ ] **Naine blanche pulsante (ZZ Ceti)** : oscillation rapide de luminosité.
- [~] **Trou noir** (accrétion) : preset galerie présent (`couronne_type 5` = horizon noir + anneau
  de photons + disque d'accrétion incliné, Doppler + spirale, **stylisé**). Reste : lentille
  gravitationnelle (arc du disque au-dessus de l'horizon) — le vrai gros morceau. *(2026-07-04)*
  - **2026-09-26** : il est désormais **rencontrable en explorant** (lot 4) — il n'était
    produit par aucun tirage, et le `match` du mode de couronne n'avait pas de bras `5` :
    l'objet existait mais personne ne pouvait tomber dessus. ~1 % des étoiles, masse propre
    (la relation masse-luminosité ne s'applique pas), pas de zone habitable.
  - Le disque d'accrétion est devenu un **vrai `Disque`** avec son plan (lot 5), puis son
    **shader dédié** et son **pli** (5.1-5.3) — la moitié arrière relevée par-dessus et
    par-dessous l'astre, qui imite l'arc lentillé sans dévier un seul rayon. L'arc *du fond
    stellaire*, lui, reste à faire (lot 8) — et la recherche a montré qu'il est **accessible**
    sans copie d'écran, les cubemaps étant core en GLSL ES 100 ([`technique.md`](technique.md)).
  - ⚠️ Aspect **non validé à l'écran** au moment de l'arrêt. Journal :
    [`astres_v2.md`](astres_v2.md).
- [x] **Magnétar** : arcs de champ magnétique dipolaire brillants, violets (`couronne_type 4`).

---

## Rendu à développer

- [x] **Couronne → JET de matière** (étoile à neutrons/protoétoile) : deux jets bipolaires le long
      de l'axe (cônes évasés + turbulence fbm advectée vers l'extérieur). Mode `couronne_type`
      (0 = halo, 1 = jets fixes, 2 = vent WR, 3 = jets tournants pulsar, 4 = arcs magnétar).
      Builders `avec_jets()` / `avec_vent()` / `avec_pulsar()` / `avec_magnetar()`.
- [ ] **Éruptions solaires (flares / CME)** — RECHERCHE À FAIRE : aujourd'hui on a des
      proéminences en arches (`Boucle`) + taches. Manque le **flare** proprement dit : sursaut
      lumineux soudain (pic d'éclat localisé près d'une tache) et **éjection de masse coronale**
      (bulle de plasma qui se détache et s'éloigne). À rechercher : déclenchement (reconnexion
      magnétique au-dessus des taches), forme (ruban + boucle post-flare), animation, couleur.
      Plus fréquent sur naines M actives / T Tauri / étoiles jeunes.
- [ ] **Granulation paramétrée** : échelle/contraste selon le type (fine pour naines,
      grossière pour supergéantes), `gran_scale`/`gran_contraste`.
- [ ] **Pulsation** : rayon et luminosité modulés par le temps (`pulse_amp`, `pulse_freq`).
- [ ] **Taux d'activité** : densité de taches/éruptions selon le type (naines M & jeunes = +).
- [x] **Vent stellaire** (WR/supergéantes bleues) : couronne épaisse turbulente colorée (`couronne_type 2`).
- [x] **Arcs magnétiques** (magnétar) : boucles de champ dipolaire brillantes (violet) autour de
      l'étoile, scintillantes (`couronne_type 4`).
- [x] **Faisceau de pulsar** : 2 cônes lumineux qui tournent + flash (`couronne_type 3`).
- [ ] **Limb darkening** vrai (vs brightening actuel) pour certaines (atmosphères étendues).

## Recherche — patterns/noise (réf. ci-dessous)

- **Granulation** = convection → bruit 3D animé (déjà : fbm + domain warping). Varier l'échelle
  par type donne déjà beaucoup de variété (fine/serrée vs grosses cellules lentes).
- **Taches** = assombrissement local (déjà). Plus nombreuses/grandes pour étoiles actives.
- **Couronne** réelle peu visible (noyée par l'éclat) ; stylisée OK. Pour WR/supergéantes,
  l'épaissir + la rendre turbulente (fbm angulaire au lieu de simples spicules).
- **Jets relativistes** (pulsar / fusion d'étoiles à neutrons / GRB) : faisceaux bipolaires
  collimatés le long de l'axe de rotation — d'où le mode « jets » à coder.
- **Pulsation** (céphéides/Mira) : oscillation lente rayon+luminosité.

Sources :
- Ben Podgursky — Procedural star rendering (granules = simplex noise 3D, corona, sunspots).
- IndieDB — Procedural Star Rendering (granulation par bruit GPU).
- Jets relativistes post-fusion d'étoiles à neutrons (modèles GRB).

---

## Plan d'implémentation proposé

1. [x] **Galerie des étoiles** (visualiseur) : catalogue de 19 presets + écran en grille, bouton
   d'accueil + aiguillage `main`.
2. [x] **Couronne paramétrée** : `couronne_type` (halo / jets / vent / pulsar / magnétar).
3. [x] **Couronne → jets** (étoile à neutrons/protoétoile) + **pulsar** + **vent WR** + **magnétar**.
4. [ ] **Flares & CME** (RECHERCHE) : sursaut lumineux + éjection de masse coronale.
5. [ ] **Pulsation** (céphéides/Mira) + **granulation paramétrée** + **taux d'activité** par type.
6. [ ] Intégrer ces types au tirage `ProfilEtoile::aleatoire` (skymap) + axe de rotation 3D des jets.
7. [ ] Gros morceaux à part : **trou noir** (disque + lentille), **naine blanche pulsante**.

## ⚠️ Éjections et motifs de surface : ils existent déjà (2026-08-06)

Relevé **avant** d'ouvrir le chantier, parce que la demande était « les étoiles
manquent d'éjections coronales et de motifs de surface » :

| Supposé manquant | Ce que le code contient |
|---|---|
| Éjections coronales | `soleil/eruptions.rs` — `Boucle` ancrée par deux pieds, cycle **Montée → Tenue → Rupture → Retombée**, corde de flux (`apex_vel`), rendue en billboards additifs par `soleil/rendu.rs` |
| Taches | `Tache` avec durée de vie et intensité, poussées dans `spots[8]` |
| Motif de surface | granulation `fbm` (`soleil.frag.glsl` l. 87) + champ magnétique (l. 97) |
| Couronne | trois types : halo, jets bipolaires (pulsar), vent stellaire (WR) |

Et `Astre::update` est bien appelé en vue système (`systeme/mod.rs:393`) : la
simulation tourne.

**Le défaut n'est donc pas l'absence, mais le fait que ça n'atteint pas l'œil.**
Hypothèse la plus probable : l'**échelle**. À distances réelles, le Soleil fait
1,1 U de rayon quand Neptune orbite à 1 440 U — boucles et granulation tombent
sous le pixel dans la vue système.

C'est le **cinquième** défaut de cette forme dans le projet (calotte, villes,
atmo de Titan, texture zonale, ventre du Starship) : *la donnée est juste, c'est
la plomberie ou l'échelle qui ne suit pas.*

**Ordre de travail** : constater d'abord (galerie d'étoiles **contre** vue
système), puis décider — LOD sur `px_rayon` comme les géantes gazeuses, ou
exagération d'échelle assumée. **Ne pas réécrire ce qui existe.**

**Constaté puis tranché** (2026-08-09 ; mesures et arbitrage en bas de ce fichier,
« Chantier P.3 — le détail »). L’hypothèse
était juste, et rien n'a été réécrit. Deux causes distinctes :

1. **L'horloge** — vrai défaut, corrigé : `Systeme::update` passait le
   multiplicateur de vitesse ORBITAL à l'animation propre des astres. À ×16, une
   protubérance réglée pour vivre 8 s tenait dans une demi-seconde. Deux horloges
   séparées (`dt`, `DtAnimation`).
2. **L'échelle** — pas un défaut, de la physique : 1,14 px de rayon au cadrage par
   défaut. Plancher en pixels `soleil::PX_MIN = 8`, qui s'efface en approchant.
   ⚠️ **Il ne rend pas les éruptions visibles** et ne le peut pas : leurs boucles
   sont dessinées à `r × 0,05`, il faudrait 20 px de rayon stellaire pour qu'elles
   atteignent le pixel et 60 pour qu'elles se lisent — soit un Soleil plus large
   que l'orbite de Mercure. Elles sont, et restent, une récompense d'approche.

Reste un manque possible et non vérifié : la **variété d'une étoile à l'autre**
— toutes partagent-elles la même graine de granulation ? À regarder en galerie.

---

# Chantier P.3 — le détail (2026-08-09)

> Extrait de [`STATE.md`](../../STATE.md) le 2026-08-10. Le constat qui a ouvert
> le chantier est plus haut dans ce fichier ; voici les mesures et l'arbitrage.

## P.3 — Étoiles : deux causes, chiffrées. Une corrigée, une à décider

Rappel du point de départ : **le code existe** (`soleil/eruptions.rs` — boucles
Montée→Tenue→Rupture→Retombée, taches, granulation `fbm`, trois couronnes) et
`Astre::update` **est** appelé. Le défaut n'était donc pas l'absence. Deux
mécanismes distincts, tous deux réels, à deux endroits de la chaîne.

### 1. L'échelle — ce n'est pas un bug, c'est de la physique

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

### P.3 bis — Tranché : plancher à 8 px (2026-08-09)

**La mesure a changé la question.** Un plancher ne peut pas livrer ce qui avait
lancé le chantier :

| ce qu'on veut voir | rayon nécessaire |
|---|---|
| disque rond, teinte, assombrissement du limbe | ~4 px |
| taches (`taille ≈ 0,2 r`) lisibles comme des marbrures | ~10 px |
| **boucles d'éruption** (dessinées à `r × 0,05`) | **20 px pour atteindre LE pixel, 60 pour se lire comme une arche** |

Or au cadrage par défaut l'orbite de Mercure fait **19 px de rayon**. Un plancher
assez grand pour montrer les éruptions ferait un Soleil **plus large que toute
l'orbite de Mercure**. Cette option est morte : le plancher achète « ça se lit
comme une étoile », pas « ça se lit comme une étoile ACTIVE ». Les protubérances
restent la récompense de l'approche.

`soleil::PX_MIN = 8.0`. Conséquence mesurée : le Soleil dessiné fait **7,90
unités, soit 42 % du rayon de l'orbite de Mercure** (18,72). Les corps étaient
**déjà** grossis d'un facteur fixe 5 (`ECHELLE_CORPS`, distances laissées
vraies) : le plancher ne crée pas l'exagération, il la rend dépendante de la
distance — et il **s'efface** dès que la taille vraie dépasse 8 px, sans saut (le
facteur vaut exactement 1 à la traversée).

**Trois pièges, tous réels, tous traités :**

1. **La cible de clic serait restée sur le rayon physique.** `Systeme::pick`
   lisait `corps().rayon` : l'étoile serait devenue un disque de 8 px qu'il
   faudrait toucher à 1,6 px près. D'où `Astre::rayon_dessine(cam_pos)` — une
   **seule** source de taille, lue par le rendu ET par la sélection.
2. **Les protubérances se seraient détachées.** `point_boucle`, la taille des
   billboards et `cache_par_sphere` prenaient tous `base.rayon`. Un seul oublié
   et les arches flottent à côté d'une sphère d'occultation de la mauvaise
   taille. `dessiner` dérive désormais tout d'un unique `r`.
3. **La garde existante ne voyait rien.** `presets.rs` teste
   `R_SOLEIL < 0.39 · UA · 0.25` — sur la **constante**. Un plancher agrandit au
   moment du dessin et passe devant elle sans la déclencher. Garde ajoutée sur la
   taille effectivement peinte.

#### P.3 bis, révisé : plancher ramené à 4 px (2026-08-14)

Jugé à l'écran, comme prévu par ce même chantier : 8 px faisait un Soleil trop
gros au cadrage par défaut. `soleil::PX_MIN` passe à **4** — le seuil que le
tableau ci-dessus identifiait déjà comme celui où le disque rond, la teinte et
l'assombrissement du limbe se lisent (sans les taches, qui demandaient ~10 px).
Le Soleil dessiné occupe désormais **21 %** du rayon de l'orbite de Mercure
(mesuré ; c'était 42 % à 8 px) — toujours au-dessus du quart que s'autorise la
constante `R_SOLEIL` seule, mais loin sous le seuil de sécurité (la moitié) que
`l_etoile_agrandie_reste_dans_l_orbite_de_mercure` garde. Aucun des trois
pièges ci-dessus n'est concerné : `Astre::rayon_dessine` et la garde sur la
taille peinte lisent `PX_MIN` en direct, rien à retoucher pour eux.

**Une quatrième copie de `VIEWPORT_H` aurait été de trop.** `planete` et `disque`
en tenaient chacun une, réglées en cascade par les appelants — et `galerie.rs` ne
réglait déjà que celle de `planete`, donnant aux anneaux le LOD du plein écran
dans une cellule. L'étoile en a besoin **hors du chemin de dessin** (`pick`), où
personne n'aurait pensé à la poser. Cellule unique dans `impostor`, les deux
setters publics délèguent : aucun site d'appel n'a bougé.

`camera::DIST_DEFAUT` : le cadrage de référence (1 280) était écrit deux fois
dans `skymap`, avec un commentaire promettant qu'elles restaient égales.

### 2. L'horloge — ça, c'était un vrai défaut, corrigé

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

