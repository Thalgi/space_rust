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
- [~] **Étoile à flares** (naine M active) : preset galerie présent ; sursauts/CME pas encore (cf. recherche).
- [ ] **Naine blanche pulsante (ZZ Ceti)** : oscillation rapide de luminosité.
- [~] **Trou noir** (accrétion) : preset galerie présent (`couronne_type 5` = horizon noir + anneau
  de photons + disque d'accrétion incliné, Doppler + spirale, **stylisé**). Reste : lentille
  gravitationnelle (arc du disque au-dessus de l'horizon) — le vrai gros morceau. *(2026-07-04)*
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

**Constaté puis tranché** (2026-08-09, § P.3 et P.3 bis de STATE.md). L'hypothèse
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
