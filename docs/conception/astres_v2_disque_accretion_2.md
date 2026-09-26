# L'aspect du disque d'accrétion — seconde recherche

> 2026-09-26. Le sous-lot 5.2 a donné la silhouette ; l'aspect ne convainc
> pas (capture à l'appui : de gros lobes bruns granuleux, un anneau de
> photons tracé au compas). Recherche web demandée, menée par deux agents —
> **référence visuelle** d'un côté, **techniques de rendu temps réel** de
> l'autre — plus un audit de notre propre code et de notre catalogue.
>
> Ce document remplace les intuitions par des chiffres. Il contient une
> **décision à prendre**, §5, qu'aucune mesure ne tranche.

---

## 1. Ce que notre code fait, mesuré

Sept défauts, tous vérifiés dans le source ou calculés à partir de nos
propres constantes. Aucun n'est une question de goût.

### 1.1 Le pli est géométriquement inversé

`disque_accretion.vert.glsl` déplace chaque sommet de
`pli · k · assiette · length(rel)`. Le bord externe, à 3 rayons, est donc
soulevé de **3,45 rayons** : ce qui devait se resserrer autour de l'astre
s'en éloigne le plus. D'où les lobes.

La littérature est nette : l'image lentillée de la face arrière forme un
**arc fin à rayon apparent quasi constant**, *indépendant de l'angle de
vue*. Elle **se comprime** vers l'ombre.

### 1.2 Le bruit ne peut pas produire de filaments

```glsl
vec2 p = vec2(cos(a2), sin(a2)) * (rr / r_ratio) * turb_freq.x;
```

Ce mapping est **isotrope par construction** : il échantillonne un cercle en
espace cartésien. Aucun réglage de `turb_freq` n'en sortira des filaments —
c'est une propriété du mapping, pas un dosage. Et l'échelle est indexée sur
le rayon **normalisé du disque**, alors que la physique la donne en fraction
du rayon **local** (5 à 10 % pour un disque mince).

Les deux sources indépendantes trouvées par la recherche disent la même
chose : **anisotropie radiale/azimutale de 8:1 à 20:1**, et une spirale
logarithmique — sans quoi les arcs restent concentriques au lieu de
s'enrouler.

### 1.3 Le brun est arithmétique

Au bord externe, `pow(2.143, -1.6) = 0,29` : on est à **71 % sur la teinte
froide** `(1.0, 0.45, 0.12)`, que le bruit assombrit encore. Le brun n'est
pas un choix, c'est le produit de deux constantes.

### 1.4 La teinte chaude est ordonnée à l'envers

Notre `chaud = (0.95, 0.98, 1.0)` a **B ≥ R**. Sous blending additif, le
clipping par canal l'emmène vers le **cyan**. Ordonnée `R > G > B`, la même
mécanique produit gratuitement la progression orange → jaune → **blanc** en
montant l'intensité — le tone mapping qu'on ne peut pas s'offrir autrement.

### 1.5 Le beaming est sous-dosé d'un facteur ~5

Nous appliquons `1 + doppler·(v·tocam)·rr^-0.5`, soit un rapport
avant/arrière mesuré de **≈ 4,9**. La loi est `I ∝ D³` avec
`D = 1/(1 − β cos θ)` : à β = 0,3 le rapport vaut **6,4**, à 0,35 il monte à
**8-10**. Et c'est l'asymétrie qui dit « disque d'accrétion » plutôt
qu'« anneau ».

### 1.6 Sans bloom, un disque fin se lit rouge et terne

Constat cité tel quel par un projet universitaire : avant bloom, « le disque
paraît plutôt fin et dominé par une couleur rouge » ; après, « un blanc
aveuglant ». Nous n'avons **pas de post-traitement global** — mais le bloom
se falsifie en géométrie (§3, R5).

### 1.7 La géométrie est calibrée au mauvais rayon

| | Notre valeur | Physique |
|---|---|---|
| Rayon de l'ombre | **1,0** (= le rayon de l'astre) | **2,6 r_s** |
| Bord interne du disque | 1,4 (1,4 × l'ombre) | **1,155 × l'ombre** |
| Anneau brillant | 1,08 | 1,05 × l'ombre |

Dessiner l'ombre au rayon de Schwarzschild — erreur que la recherche
signale comme fréquente — la rend **2,6 fois trop petite**, et le disque
devient un anneau lointain autour d'une petite bille. C'est exactement ce
que montre la capture.

### 1.8 Et l'anneau brillant n'est pas celui qu'on croit

**La correction la plus contre-intuitive de cette recherche.** L'anneau de
photons proprement dit ne gagne qu'un renforcement **logarithmique** de
brillance : il est observationnellement négligeable. Ce qu'on voit est
l'**anneau de lentille** — une image démagnifiée de la face arrière du
disque, superposée à l'émission directe, **2 à 3 fois plus brillante**
qu'elle à cet endroit.

Conséquence pratique : la bonne chose à dessiner n'est pas un cercle
uniforme, mais une **copie fine et surbrillante du disque**, portant sa
couleur, son bruit et son asymétrie Doppler. Notre cercle orange tracé au
compas est le mauvais objet.

---

## 2. Ce que notre catalogue nous apprend — y compris en négatif

Le voile des anneaux planétaires porte une technique que j'avais d'abord
cru bonne à reprendre : `dens *= 0.86 + 0.14 * hash(floor(t * 130.0))`, des
**stries radiales fines** qui font lire un anneau comme feuilleté.

⚠️ **C'est précisément ce qu'il ne faut pas faire ici.** Le profil radial
d'un disque d'accrétion est **monotone** : zéro anneau concentrique, zéro
division, zéro annelet. Les stries radiales sont l'un des treize traits qui
distinguent Saturne d'un disque d'accrétion. J'allais importer la signature
de l'objet dont on cherche à se démarquer.

Ce que le catalogue apporte vraiment :

- le **plancher de continuum** (`mix(1.0, ...)`) qui évite l'aspect
  fourrure — à protéger en passant en filamenteux ;
- le **cisaillement képlérien** en `r^1,5`, déjà correct chez nous ;
- le pipeline de voile, réutilisable tel quel pour le **faux bloom** (§3,
  R5) sans en créer un nouveau.

---

## 3. Les corrections, par rapport gain/coût

Ordre de priorité issu de la recherche technique. Les cinq premières
coûtent **presque rien**.

| # | Correction | Coût |
|---|---|---|
| **R1** | **Bruit anisotrope** : échantillonner en (radial, azimutal) avec un rapport de cellule 8:1 à 20:1, hash périodique sur l'axe angulaire pour la couture, et décalage en **spirale logarithmique** `ang − w·log(rr)` | 1 `log` + 1 `mod` |
| **R2** | **Beaming en D³** au lieu du terme linéaire : rapport 8-10:1 | 1 division |
| **R3** | **Teinte chaude R > G > B** et intensité > 1 laissée passer : le clipping par canal fabrique le cœur blanc | 0 |
| **R4** | **Faux bloom géométrique** : un annulus large, faible, sans bruit, en additif — réutilise le pipeline du voile | ~600 sommets |
| **R5** | **Dédoubler l'exposant** : raide pour la couleur, doux (0,4-0,8) pour l'opacité. Le bord interne écrase sans effacer le bord externe | 1 `pow` |
| **R6** | **Calibrer la géométrie** : ombre à 2,6 r_s, bord interne à 1,155 × l'ombre, bords **asymétriques** (interne net ≈ 0,015, externe mou ≈ 0,3) | 0 |
| **R7** | **Corriger le pli** : comprimer vers un rayon apparent constant au lieu de dilater | 0 |
| **R8** | **Anneau de lentille** et non anneau de photons : une copie fine, démagnifiée, surbrillante du disque, avec son bruit et son Doppler | ~256 sommets |
| **R9** | **Fausse épaisseur** : 2-3 copies de l'annulus à ±h, amplitude divisée d'autant | ×2-3 sommets |
| **R10** | **Borner la phase** képlérienne (`mod` avant la division) : évite l'enroulement infini et la quantification du hash | 1 `mod` |
| **R11** | **Anti-crénelage sans dérivées** : `fwidth` n'est **pas** dans GLSL ES 100 core. Passer la taille projetée en uniform depuis le CPU | 1 uniform |

**Hors de portée, et pourquoi** : le bloom véritable, le grab-pass et le
raymarching géodésique exigent tous une **cible de rendu** ou une copie du
framebuffer — ce que l'architecture n'a pas. La lentille du **fond stellaire**
(déviation analytique + échantillonnage du champ d'étoiles procédural)
serait accessible, mais c'est le lot 8, pas celui-ci.

⚠️ **Deux pièges nommés par les auteurs de ces shaders**, qui nous
concernent directement : le **burn-out additif** (des couches superposées
finissent en blanc plat qui détruit la silhouette — notre `part_arc` en
protège déjà une), et le fait que **deux transparents qui s'intersectent ne
se trient pas** — d'où notre découpe en moitiés, qui se trouve être la
parade recommandée.

---

## 4. Deux avertissements de méthode, hérités de la recherche

- **Ne pas copier l'image EHT comme photographie.** C'est une reconstruction
  limitée par un lobe instrumental ; son anneau épais et flou est en partie
  de l'instrument, et les trois nœuds de Sgr A\* sont peut-être des
  artefacts d'échantillonnage.
- **Ne pas copier Gargantua comme référence physique.** Ses deux traits les
  plus mémorables — symétrie gauche/droite et teinte orangée — sont
  exactement ce que Thorne et DNEG ont **retiré ou fabriqué**. C'est un
  excellent modèle de *lisibilité*, un mauvais modèle de *réalisme*.

---

## 5. La décision qu'aucune mesure ne tranche

Le papier d'Interstellar est explicite : avec le beaming réaliste, le disque
devient « exceedingly lopsided, with the hole's shadow barely discernible,
was obviously unacceptable ». **Un disque physiquement juste rend l'ombre du
trou noir illisible.** Le trait le plus spectaculaire du réel détruit le
trait le plus reconnaissable.

Et la couleur suit le même arbitrage : un disque d'accrétion stellaire actif
est à ~1,2 × 10⁷ K, donc **blanc-bleu saturant en blanc**. L'orange de
Gargantua vient d'un disque **froid, non accrétant**, spécifié à 4500 K.

Trois cibles cohérentes, il faut en choisir une :

1. **Physique** — blanc-bleu, asymétrie 8-10:1, ombre à 2,6 r_s. Spectaculaire
   et juste ; l'ombre risque d'être mangée par le côté brillant.
2. **Interstellar** — orangé chaud, asymétrie réduite, ombre nette et
   centrée. Lisible, iconique, et assumé comme un disque froid.
3. **Intermédiaire** — blanc-bleu, mais beaming plafonné à ~4:1 pour garder
   l'ombre lisible. Ni tout à fait juste, ni tout à fait iconique.

Ma recommandation : **3**, avec les constantes exposées pour glisser vers 1
ou 2 à l'œil. Mais c'est un choix esthétique, et il appartient à
l'utilisateur.

---

## 6. Ce qui ne se jugera qu'à l'œil

L'amplitude du faux bloom, le dosage du beaming dans la fourchette choisie,
le rapport d'aspect exact des cellules de bruit, la force du pli. Rien de
tout cela ne se déduit.

**Se testent, et doivent l'être** : la calibration géométrique (rapports
ombre / bord interne / anneau), l'ordre des canaux de la teinte chaude, le
fait que le pli comprime au lieu de dilater, la correspondance
uniforms↔GLSL de tout nouvel uniform, et le budget de sommets.
