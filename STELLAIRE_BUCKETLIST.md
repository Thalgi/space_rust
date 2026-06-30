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
- [x] **Protoétoile / T Tauri** : preset galerie + jets bipolaires.

## Étoiles particulières (le « un peu spécial »)

- [x] **Étoile à neutrons** : jets de matière bipolaires **fixes** (`couronne_type 1`).
- [x] **Pulsar** : faisceau bipolaire qui **tourne** (effet phare) + flash périodique (`couronne_type 3`).
- [x] **Wolf-Rayet** : **vent stellaire** turbulent en expansion, bleu-violet (`couronne_type 2`).
- [x] **Étoile carbonée (C)** : preset galerie (rouge sombre profond).
- [ ] **Variable céphéide / Mira** : **pulsation** (rayon + luminosité qui oscillent).
- [x] **Étoile à flares** (naine M active) : flash impulsif + deux rubans qui s'écartent + arcade
      post-flare + CME (`avec_flares()`, câblé sur le preset galerie via `taux_flare`).
- [ ] **Naine blanche pulsante (ZZ Ceti)** : oscillation rapide de luminosité.
- [x] **Trou noir** (accrétion) : `couronne_type 5` — horizon noir + rim chaud stylisé
      (lentille gravitationnelle approchée, pas de vrai ray-tracing) + disque incliné
      (aplati selon l'axe, turbulence fbm, asymétrie Doppler). `avec_trou_noir()`,
      2 presets galerie (stellaire / supermassif). Reste : jets relativistes, intégration
      au tirage aléatoire skymap (masse/gravité/éclairage à décider).
- [x] **Magnétar** : arcs de champ magnétique dipolaire brillants, violets (`couronne_type 4`).

---

## Rendu à développer

- [x] **Couronne → JET de matière** (étoile à neutrons/protoétoile) : deux jets bipolaires le long
      de l'axe (cônes évasés + turbulence fbm advectée vers l'extérieur). Mode `couronne_type`
      (0 = halo, 1 = jets fixes, 2 = vent WR, 3 = jets tournants pulsar, 4 = arcs magnétar).
      Builders `avec_jets()` / `avec_vent()` / `avec_pulsar()` / `avec_magnetar()`.
- [x] **Éruptions solaires (flares / CME)** : `struct Flare` (eruptions.rs) ancré sur la plus
      grosse tache (reconnexion). Phases dérivées de `age` côté rendu : **flash** impulsif
      blanc-bleu (montée ~50 ms, décroissance exp) + embrasement global léger ; **deux rubans**
      Hα roses qui s'écartent de la ligne d'inversion ; **arcade** de boucles post-flare qui
      s'élève entre les rubans ; **CME** = bulle de plasma douce (halo + cœur + front de choc,
      plus d'anneau de points) qui se détache et s'estompe avec la distance. Activé sur naines M
      / T Tauri, **câblé galerie + skymap + objet** (`ProfilEtoile.flares`).
- [x] **Proéminences en nappes (loft)** : arches rendues en rubans triangulés (profil doux via
      l'alpha radial du halo, uv vertical 0→1), ancrées dans le **plan de la boucle** -> pas de
      vrillage quand la caméra tourne ; l'éjection (`Rupture`) évase la nappe.
- [x] **Granulation paramétrée** : `gran_scale` (taille des cellules) + `gran_contraste`,
      dérivés du rayon (fines pour naines, grosses pour géantes) et de la couleur (net pour
      froides, lisse pour chaudes). Override `avec_granulation()` (géantes de la galerie).
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
4. [x] **Flares & CME** : flash + rubans + arcade post-flare + CME (`avec_flares()`, preset galerie).
5. [ ] **Pulsation** (céphéides/Mira) + **granulation paramétrée** + **taux d'activité** par type.
6. [ ] Intégrer ces types au tirage `ProfilEtoile::aleatoire` (skymap) + axe de rotation 3D des jets.
7. [~] Gros morceaux à part : **trou noir** (fait, version stylisée bon marché — voir
   ci-dessus), **naine blanche pulsante** (reste à faire).
