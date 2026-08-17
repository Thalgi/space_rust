# Journal — Trajectoires : vaisseaux et stations mobiles

> Le *pourquoi* et les décisions de gameplay sont dans
> [`conception/trajectoires.md`](../conception/trajectoires.md) (le chantier
> lui-même, §4 pour les sept décisions tranchées le 2026-08-14/15) et
> [`conception/concept_jeu.md`](../conception/concept_jeu.md) (la vision RTS
> globale qui les motive). Ce fichier raconte l'avancement réel, lot par lot
> — créé à l'ouverture du Lot A, pas avant (convention du projet).

---

## Lot A — Statistiques de vol (2026-08-15)

**Fait** : `src/vaisseau/vol.rs`, nouveau module. Implémente la décision 2 de
`trajectoires.md` §4 (statistiques de vol en hybride : base par classe +
modificateurs des pièces VAB), scope réduit conformément à `trajectoires.md`
§6 (Lot A « plus léger que l'ancien Lot A à simulation complète »).

**`ClasseVaisseau`** : un seul variant, `Starship`, **volontairement**. Ni
l'ISV (sa mécanique de vol est un événement distinct hors périmètre —
décision 6), ni Jupitris/le cargo sans nom (aucun design assemblé à mesurer
— `concept_jeu.md` §7) n'y figurent. Ajouter une classe suppose un design
réel à mesurer, pas une extrapolation.

**`StatsVol { vitesse, portee, capacite_fret }`** : `capacite_fret` reste
fixe à la base de la classe — aucune pièce ne la modifie aujourd'hui, faute
de brique de soute générique pour le Starship (`NacelleCargo`/
`RatelierCargo` sont propres à l'ISV, `cargo.rs`). Documenté dans le code
comme simplification explicite, pas un oubli.

**Modificateurs, deux seulement** :
- Un moteur principal (`Composant::Raptor`, ou `Composant::Propulseur` **si**
  `variante.axial()`) ajoute à la vitesse. Une grappe RCS (`GrappeRcs`,
  `Hypergolique`, `PlasmaPulse` — non-axiales) n'ajoute rien : c'est de
  l'attitude, pas de la propulsion principale.
- Un `Composant::Reservoir` allonge la portée, proportionnellement à sa
  longueur.

**Choix de conception** : `modificateur_vitesse`/`modificateur_portee` ne
sont **pas** des méthodes exhaustives sur `Composant` (contrairement à
`Composant::cout`, qui doit compter les ~44 variantes sans en manquer une).
Ce sont des fonctions locales à `vol.rs`, avec un `_ => 0.0` assumé : ce
calcul ne concerne que la propulsion/les réservoirs, et ignorer un panneau
solaire ou un radiateur n'est pas un oubli qui pourrait passer inaperçu de
la même façon qu'un budget qui compterait mal. Évite aussi de toucher au
grand `match` de `composant/mod.rs`, partagé par tout le reste du
catalogue — cohérent avec la consigne du projet de ne pas élargir le rayon
d'un chantier dans du code partagé sans besoin réel.

**Chiffres de base** (`BASE_VITESSE_STARSHIP = 4.0`, `BASE_PORTEE_STARSHIP =
40.0`, `BASE_FRET_STARSHIP = 20.0`, `MODIF_VITESSE_MOTEUR = 0.6`,
`MODIF_PORTEE_PAR_LONGUEUR_RESERVOIR = 3.0`) : **provisoires**, un
bouche-trou nommé comme `engin::ECHELLE_ISS` — à recalibrer sur un premier
trajet joué (Lot B/C), pas avant.

**5 tests, tous red-checkés individuellement** (sabotage → échec confirmé
avec le bon message → restauration) :
- `sans_piece_pertinente_les_stats_sont_la_base_de_la_classe`
- `un_reservoir_allonge_la_portee_sans_changer_la_vitesse`
- `un_raptor_augmente_la_vitesse`
- `un_propulseur_axial_augmente_la_vitesse`
- `une_grappe_rcs_najoute_rien_a_la_vitesse` (celui qui garde vraiment le
  garde-fou `variante.axial()` — sabotage confirmé : sans lui, une grappe
  RCS ajoutait aussi de la vitesse)

**`#![allow(dead_code)]` en tête de module** : `calculer` n'est appelé par
personne côté binaire aujourd'hui — c'est le Lot B qui le consommera pour
donner une vitesse/portée à un vaisseau en transit. `dead_code` ne sait pas
qu'un test l'exerce déjà (`methode.md`) ; l'`allow` documente que c'est
attendu, pas un manque.

**État vérifié** : 686 tests verts (681 + 5), `cargo clippy --release
--all-targets` à 0 avertissement.

**Prochaine étape** : Lot B (`trajectoires.md` §6) — mode de mouvement
mobile pour un `Astre`, qui consommera `vol::calculer` pour savoir à quelle
vitesse un Starship assemblé parcourt l'arc vers sa destination.

## Lot B, première tranche — prédiction et géométrie de l'arc (2026-08-15)

**Fait, mais pas tout le Lot B** : la prédiction de position et la géométrie
du trajet, toutes deux pures et testées. **Ce qui manque encore pour clore
le Lot** : un `Astre` mobile réel qui suit une `Trajectoire` frame après
frame (voir « Reste à faire » en bas).

### Une découverte en cours de route : le mode N-corps

`trajectoires.md` §5 s'appuyait sur « toute cible sur rails a une position
future connue en forme fermée ». En creusant `systeme/mod.rs`, ça tient
**seulement** en `ModePhysique::SurRails` (le défaut) — un bouton de menu
existant (`self.menu.phys_rails`) peut faire basculer tout le système en
`NCorps` (intégration gravitationnelle), où une planète n'a plus de position
analytique. **Portée du Lot B assumée** : la prédiction n'est correcte qu'en
`SurRails` ; sous `NCorps`, elle diverge de la position réellement simulée.
Dette nouvelle, à trancher plus tard (`trajectoires.md` §7, ajoutée) — pas
bloquante ici parce que `SurRails` est le mode par défaut et le seul déjà
éprouvé par le reste du jeu.

Bonne nouvelle en contrepartie : `crate::orbite::Orbite::position(t)` **est
déjà** une évaluation pure `f(t)` (aucune intégration, aucun état caché) —
la prémisse tient, exactement comme espéré, tant qu'on reste sur rails.

### `Astre::position_future` — la prédiction, composée récursivement

Nouvelle méthode de trait (`astre.rs`), symétrique de `maj_rail` mais en
lecture seule et projetable dans le futur : `fn position_future(&self,
foyer_predit: Vec3, t: f64) -> Option<Vec3>`. Implémentée dans
`planete/mod.rs` en une ligne (`self.orbite_kep.map(|o| foyer_predit +
o.position(t))`) — même formule que `maj_rail`, donc pas de second calcul à
tenir d'accord (**une source par fait**) : les deux appellent la même
`Orbite::position`.

**Scope assumé, pas un oubli** : `orbite_kep` n'est posé que pour une
planète « sur rails » — une **lune** (même struct `Planete`, mais motorisée
par `orbiter_autour`/`l_angle`, pas `orbite_kep`) rend `None`. Pas de test
dédié dans `planete/mod.rs` : le fichier entier est dépourvu de tests
(`Planete::new` construit de vrais `Material` GPU, donc rien ne s'y teste
hors contexte graphique) — le red-check porte sur la **composition**
(`Systeme`), pas sur cette délégation d'une ligne.

`Systeme` gagne `temps()` (horloge courante, même base que la prédiction) et
`position_future_de(idx, t)` : résout le foyer de l'astre (l'étoile hôte à
sa position **actuelle**, pas prédite — approximation assumée pour un
système à étoile fixe, dette pour un système multiple où l'étoile bouge)
puis délègue à `Astre::position_future`. **3 tests**, tous red-checkés via
un `CorpsEssai` de test étendu d'un champ `decalage_futur` (évite d'avoir à
construire un vrai `Planete`/`Orbite` juste pour tester la composition) :
- `position_future_de_compose_le_bon_foyer` — sabotage (forcer le foyer à
  `Vec3::ZERO` au lieu de résoudre `Foyer::Etoile`) confirmé rouge.
- `position_future_de_retombe_sur_lorigine_sans_etoile_hote`
- `position_future_de_est_absente_sans_orbite_connue`

### `systeme::trajectoire::Trajectoire` — l'arc, décoratif mais garanti

Nouveau module. Pas de coniques raccordées ni d'équation de Kepler pour le
trajet lui-même (décision assumée, cohérente avec la décision 2 — pas de
simulation complète) : une **Bézier quadratique** entre départ et arrivée
**prédite**, avec un point de contrôle décalé perpendiculairement à la
droite. Le rendez-vous est garanti **par construction** — `u=0` rend
`depart`, `u=1` rend `arrivee`, exactement, quel que soit le bombement.

Le bombement reste **dans le plan orbital** (perpendiculaire à sa normale),
pas hors de lui — `bombement_dir = droite.cross(normale_plan)`, repli sur un
axe arbitraire si colinéaire (même parade que `ecran::icone::cadre_camera` :
`Y.cross`, secours `Z.cross`). Durée déduite de la distance et de la
vitesse de croisière (`vaisseau::vol::StatsVol::vitesse`).

**9 tests.** 7 red-checkés individuellement, tous confirmés (clamp de
`progres` retiré → deux tests d'extrapolation rougissent ; `FRACTION_
BOMBEMENT` mis à 0 → les deux tests de bombement rougissent ; formule de
durée privée de la division par `vitesse` → seul le test de durée rougit ;
`terminee` passé en comparaison stricte → seul son test rougit). **Un test
n'a pas rougi malgré trois sabotages tentés** :
`un_trajet_de_longueur_nulle_ne_panique_pas` (départ == arrivée). La garde
explicite de `perpendiculaire` (`if d == Vec3::ZERO`) s'est révélée
redondante contre le premier sabotage — mais retirer la garde **et**
remplacer le `normalize_or_zero` final par un `normalize` nu n'a *pas*
non plus fait rougir le test : `d` reste un `Vec3::ZERO` propre (pas de NaN)
à chaque étape tant que l'entrée est un vrai zéro exact, et le dernier
`normalize()` sur un vecteur nul exact ne semble pas produire de NaN
observable dans ce cas précis (raison exacte non élucidée côté `glam`, pas
creusée davantage — retour sur investissement jugé trop bas). **Décision** :
garder la garde explicite malgré la redondance apparente — la retirer se
serait appuyé sur un comportement de `glam` non pleinement compris plutôt
que sur une garantie lisible dans le code. Consigné ici plutôt que passé
sous silence, conformément à `methode.md` (« un sabotage qui ne rougit pas
ne prouve pas que la garde est faible »).

**État vérifié** : 698 tests verts (686 + 3 + 9), `cargo clippy --release
--all-targets` à 0 avertissement. `#![allow(dead_code)]` sur `trajectoire.rs`
et `#[allow(dead_code)]` sur `Astre::position_future`/`Systeme::
position_future_de`/`Systeme::temps` : rien n'est encore câblé au binaire.

### Reste à faire pour clore le Lot B (état avant la tranche suivante)

- Un type `Astre` mobile réel (le pendant d'`EnginOrbital` pour un vaisseau
  en transit) qui tient une `Trajectoire` et appelle `.position(t)` à
  chaque frame au lieu d'`orbiter_autour` — rien de tel n'existe encore.
- Le câblage dans `Systeme::update` : aujourd'hui rien n'avance un mobile,
  la boucle ne connaît que `orbiter_autour`/`maj_rail`.
- Le point de départ d'un trajet (aujourd'hui `Trajectoire::nouvelle` prend
  un `depart: Vec3` déjà donné — d'où vient-il en jeu : position d'un
  Starship posé sur un astre ? Pas encore de réponse, question du Lot C).

## Lot B, deuxième tranche — le vaisseau mobile lui-même (2026-08-15)

Les deux premiers points ci-dessus sont clos. Le troisième (d'où vient le
point de départ en jeu) reste ouvert — toujours une question du Lot C.

**`src/transit.rs`, nouveau.** `VaisseauEnTransit` : le pendant
d'`EnginOrbital` (`engin.rs`) pour un astre mobile. Même structure (un
`MaillageStation` cuit, une échelle assemblage → monde, `draw()` identique à
une translation + mise à l'échelle près), mais motorisé par une
`Trajectoire` au lieu d'un rayon d'orbite fixe. `nouveau(etat, trajectoire,
envergure)` place le vaisseau exactement à `trajectoire.position(trajectoire
.depart_a())` — pas à l'origine du monde, pas à l'arrivée. `arrive(t)`
délègue à `Trajectoire::terminee` : le vaisseau reste visible, immobile, une
fois arrivé — rien (Lot E) ne décide encore de son sort à ce moment-là.

**Nouvelle méthode de trait**, `Astre::suivre_trajectoire(&mut self, t:
f64)` (défaut no-op, `astre.rs`) : ni `orbiter_autour` (incrémental, prend
un centre de parent) ni `maj_rail` (a besoin d'un foyer) ne convenaient — un
trajet n'a ni l'un ni l'autre, seulement une heure. `VaisseauEnTransit` la
redéfinit en une ligne : `self.base.position = self.trajectoire
.position(t)`.

**`Systeme::update` gagne une troisième boucle de mouvement**, après celle
des lunes : `for a in &mut self.astres { a.suivre_trajectoire(t); }`, avec
le même `t` absolu déjà calculé en tête de fonction pour `maj_rail`. Coût :
un appel no-op de plus par astre et par frame pour tout ce qui n'est pas en
transit — négligeable, même schéma que la boucle d'animation.

**5 tests, tous red-checkés** :
- `nouveau_place_le_vaisseau_au_point_de_depart` — sabotage (`depart_a()` →
  `arrivee_a()` par erreur) confirmé rouge.
- `suivre_trajectoire_avance_le_long_de_larc` — sabotage (méthode vidée en
  no-op) confirmé rouge.
- `arrive_devient_vrai_a_larrivee_pas_avant` — sabotage (`arrive` renvoie
  toujours `false`) confirmé rouge.
- `nouveau_rend_none_pour_une_station_vide` — gardé sans sabotage forcé :
  passthrough mécanique d'`EtatStation::doit_dessiner`, même contrat
  qu'`EnginOrbital::nouveau`, déjà couvert par construction (`?`).
- `update_appelle_suivre_trajectoire_avec_le_temps_absolu`
  (`systeme/mod.rs`, via `CorpsEssai` étendu d'un mouchard
  `t_trajectoire_recu`) — sabotage (boucle retirée de `update`) confirmé
  rouge ; vérifie aussi que l'horloge **cumule** entre deux appels
  (3.0 puis 5.0), pas un `dt` brut repassé tel quel.

**Pas de test dédié pour `MaillageStation::cuire`** dans ces nouveaux
tests : contrairement à `Planete::new`, `cuire` ne touche pas le GPU (pas de
`Material`), donc les tests de `transit.rs` construisent une vraie
`Station` de test (`vaisseau::demo_station`) sans contexte graphique — plus
proche du réel que les corps d'essai synthétiques de `systeme/mod.rs`.

**`#![allow(dead_code)]` en tête de `transit.rs`** : rien ne construit
encore de `VaisseauEnTransit` en jeu (où démarre un trajet reste une
question ouverte du Lot C).

**État vérifié** : **703 tests verts** (698 + 5), `cargo clippy --release
--all-targets` à 0 avertissement.

### Ce qui reste, pour de vrai, avant que quelque chose vole en jeu

- **Lot C** : une UI qui choisit une destination, construit une
  `Trajectoire` et appelle `Systeme::ajouter(Box::new(VaisseauEnTransit
  ::nouveau(...)))`. **Le départ n'est plus une question ouverte**
  (`trajectoires.md` §4 décision 8, 2026-08-15) : c'est la position courante
  d'un `EnginOrbital` déjà posté (en orbite d'une planète/lune, ou en vol
  stationnaire près d'un habitat/station) — retiré du système, sa position
  lue, une `Trajectoire` construite depuis là. Il manque encore : la
  fonction de conversion elle-même, et l'UI qui la déclenche.
- **Lot D** : rendu de la trajectoire prévue avant le départ (réemploi du
  semis de points), et une caméra qui peut suivre un mobile.
- **Lot E** : que se passe-t-il à `arrive(t) == true` — amarrage, conversion
  **retour** en `EnginOrbital` si la destination est en orbite, consommation
  si c'est une colonisation (décision 7) ? `VaisseauEnTransit` expose
  `arrive()` mais ne fait rien de la réponse pour l'instant.

## Nettoyage — le mode N-corps retiré (2026-08-16)

Sur confirmation de l'utilisateur : `ModePhysique::NCorps` était un test, pas
une fonctionnalité à garder. Retiré entièrement, pas juste désactivé :

- `src/systeme/gravite.rs` supprimé (l'intégrateur leapfrog, `accelerations`).
- `ModePhysique`, le champ `mode` de `Systeme`, `Systeme::mode()`/
  `regler_mode()` supprimés. `Systeme::update` n'a plus qu'un seul chemin
  (sur rails), sans `match`.
- `Astre::amorcer_ncorps` (trait) et son implémentation dans `Planete`
  supprimés — plus de hand-off à amorcer puisqu'il n'y a plus de second mode
  vers lequel basculer.
- Le bouton **PHYS** du menu (`menu/mod.rs`, `menu/dessin.rs`) supprimé :
  `Menu::phys_rails`, sa bascule, son libellé « PHYS: SUR RAILS »/« PHYS:
  N-CORPS ». La bande d'outils passe de 5 bascules à 4 (`LARGEURS`,
  `bascules()`) ; `N_CALQUES` n'avait plus de raison d'exister séparément de
  la longueur du tableau, retiré aussi. Deux tests de mise en page adaptés
  (un assert devenu sans objet — comparer les calques à un bouton PHYS qui
  n'existe plus — supprimé, pas contourné).
- `src/ecran/skymap.rs` : le bloc qui synchronisait `Systeme` avec le bouton
  retiré.
- **Conséquence pour ce chantier** : la découverte notée dans `trajectoires.md`
  §7 (« la prédiction ne tient qu'en `SurRails` ») **disparaît d'elle-même**
  — il n'y a plus qu'un mode, la prémisse du §5 tient sans condition.

**Vérifié** : build et `cargo clippy --release --all-targets` propres du
premier coup après le retrait (aucun avertissement, aucun import orphelin) ;
**703 tests verts**, inchangé (aucun test supprimé, deux adaptés). Contrôle
visuel par capture (`CAPTURE=...png`) : la bande d'outils affiche
correctement les 4 bascules restantes, sans chevauchement.

## Persistance d'un trajet en cours — ce que ça veut dire concrètement

Question posée en `trajectoires.md` §7, demandée en clair : sauvegarder le
jeu (`vaisseaux/`, `reglages.json`, les presets) fige aujourd'hui un
**design** ou un **système généré** — jamais un vaisseau qui serait, à cet
instant précis, à mi-chemin entre deux astres.

**Le problème concret** : `VaisseauEnTransit` tient `depart: Vec3`,
`arrivee: Vec3` et deux dates absolues (`t_depart`, `t_arrivee`), toutes
relatives à l'horloge `Systeme::temps` — qui repart de zéro à chaque relance
du jeu (`Systeme::new()`). Sauvegarder « où en est le vaisseau » suppose de
sauvegarder soit sa position/vitesse à l'instant `T`, soit son
`depart`/`arrivee`/dates **et** une façon de recaler ces dates absolues sur
la nouvelle horloge au chargement — sans ce recalage, un vaisseau sauvegardé
à mi-trajet rejaillirait avec une date d'arrivée qui n'a plus de sens
(`t_arrivee` compté depuis un `t=0` qui n'existe plus).

**Deux options, pas encore choisies** :
1. **Sauvegarder l'état de vol** (position figée + trajectoire recalée sur
   la nouvelle horloge au chargement) — le vaisseau reprend exactement où il
   était. Demande un nouveau format de sauvegarde (aujourd'hui, seuls des
   designs VAB et des systèmes/réglages se sauvegardent, jamais un astre
   « en cours »).
2. **Ne pas sauvegarder les trajets en vol** — au chargement, soit ils sont
   perdus (le vaisseau réapparaît « posté » à son point de départ ou
   d'arrivée), soit la sauvegarde est simplement refusée/avertie tant qu'un
   trajet est en cours. Plus simple, mais frustrant si un trajet dure
   plusieurs sessions de jeu.

Décision 3 (`trajectoires.md` §4 — logistique manuelle d'abord, pas de
routes permanentes) réduit l'urgence : sans routes qui tournent en continu,
un trajet en vol au moment de sauvegarder reste un cas **occasionnel**, pas
la norme. Mais il reste à trancher avant le Lot C si on veut éviter de le
découvrir en codant.

## Placeholders — tous les chiffres provisoires du chantier, en un endroit

Consolidé sur demande (2026-08-16) : plutôt que de les retrouver dispersés
dans le code au moment de les régler. Deux familles — celles qui ont déjà
une valeur provisoire dans le code, celles qui n'ont même pas encore de
représentation.

### Déjà dans le code (à ajuster, pas à inventer)

| Constante | Fichier | Valeur | Ce qu'elle règle |
|---|---|---:|---|
| `BASE_VITESSE_STARSHIP` | `src/vaisseau/vol.rs:44` | 4.0 | Vitesse de croisière de base d'un Starship |
| `BASE_PORTEE_STARSHIP` | `src/vaisseau/vol.rs:45` | 40.0 | Portée de base |
| `BASE_FRET_STARSHIP` | `src/vaisseau/vol.rs:46` | 20.0 | Capacité de fret de base |
| `MODIF_VITESSE_MOTEUR` | `src/vaisseau/vol.rs:51` | 0.6 | Bonus de vitesse par moteur principal (Raptor ou Propulseur axial) |
| `MODIF_PORTEE_PAR_LONGUEUR_RESERVOIR` | `src/vaisseau/vol.rs:54` | 3.0 | Bonus de portée par unité de longueur de réservoir |
| `FRACTION_BOMBEMENT` | `src/systeme/trajectoire.rs:24` | 0.15 | Écart de l'arc par rapport à la ligne droite (fraction de la distance) |
| `ORBITE_STARSHIP` | `src/engin.rs` | 1.85 | Rayon d'orbite du Starship posté (en rayons de sa planète) |
| `ECHELLE_STARSHIP` | `src/engin.rs` | 0.30 | Envergure à l'écran (même exagération que l'ISS) |
| `OMEGA_STARSHIP` | `src/engin.rs` | 0.45 | Vitesse angulaire de l'orbite postée |

Aucun de ces neuf n'a été calibré sur un trajet vu en jeu — ils datent tous
des Lots A/B/C, écrits pour que le code tourne et se teste, pas pour qu'un
trajet ait la bonne « sensation ». À revoir dès qu'un premier trajet
s'affiche vraiment (Lot D). `Systeme::calculer_trajectoire`'s `ITERATIONS =
24` est différent des autres : ce n'est pas un réglage de « sensation », mais
une marge de convergence **mesurée** (voir le journal Lot C ci-dessous) — pas
à ajuster sans refaire la mesure.

### Pas encore dans le code du tout (décision de gameplay avant chiffre)

| Quoi | D'où ça vient | Statut |
|---|---|---|
| Durée avant destruction d'une superstructure sans énergie | `concept_jeu.md` §6, « un mois » | Donnée de départ, pas calibrée |
| Valeur de départ et courbe du % de réutilisabilité (décision 9) | `concept_jeu.md` §7 | Rien — mécanique décrite, aucun chiffre |
| Barème de coût par classe de vaisseau (décision 9) | `concept_jeu.md` §7 | Rien |
| Seuils de PV / dégâts par événement (décision 10) | `concept_jeu.md` §6 bis | Rien — même la liste des événements est incomplète |
| Capacités de fret par classe (Jupitris, cargo sans nom) | `concept_jeu.md` §7 | Sans objet : les designs eux-mêmes n'existent pas encore |
| Tiers de recherche (nombre, coût, ce qu'ils débloquent) | `concept_jeu.md` §4 | Rien — l'arbre technologique lui-même reste à créer |
| Bonus de regroupement du plateau de tuiles | `concept_jeu.md` §3 | Rien |

## Lot C — Un premier vaisseau qui vole vraiment (2026-08-16)

Le vaisseau bouge maintenant à l'écran, depuis un vrai clic. Décision 8
(§4) est pleinement câblée : plus une prédiction sur le papier.

### Une impasse d'object-safety, et ce qu'elle a changé de la conception

Le premier plan (`suivi/trajectoires.md`, Lot B) prévoyait
`EnginOrbital::vers_transit(self, trajectoire) -> Result<VaisseauEnTransit,
Self>` — une méthode **inhérente**, consommant `self` par valeur. Pour
l'appeler depuis l'UI, il fallait un moyen générique de dire « prends
l'astre à cet index, essaie de le convertir, remets le résultat » — donc une
méthode sur le **trait** `Astre`, dispatchée dynamiquement sur `Box<dyn
Astre>`.

Premier essai : `fn lancer(self: Box<Self>, ..) -> Box<dyn Astre>`, avec un
défaut `{ self }` (« ne fait rien »). **Ça ne compile pas** : le corps par
défaut doit coercer `Box<Self> → Box<dyn Astre>` pour un `Self` générique,
ce que Rust refuse sans une borne `Self: Sized` explicite — borne qui,
posée sur la déclaration du trait, **retire la méthode de la vtable** :
elle devient alors impossible à appeler sur un `Box<dyn Astre>`, ce qui est
exactement ce qu'il fallait pouvoir faire. Impasse confirmée par le
compilateur, pas une supposition : voir le message d'erreur exact
(`E0277`, « the size for values of type `Self` cannot be known at
compilation time ») avant d'abandonner la piste.

**Solution retenue** : `fn tenter_lancer(&mut self, trajectoire) ->
Option<Box<dyn Astre>>`. Emprunt mutable plutôt que consommation : le
défaut `{ None }` ne coerce rien, se compile pour n'importe quel `Self`
sans borne, et reste pleinement dans la vtable. `EnginOrbital` redéfinit
`tenter_lancer` en vidant ses propres champs avec `std::mem::take`
(`MaillageStation` a donc gagné un `#[derive(Default)]`, lots vides) plutôt
qu'en consommant `self` — l'engin d'origine, vidé, est de toute façon jeté
par l'appelant juste après. `Systeme::lancer(idx, trajectoire) -> bool`
fait ce jeu de chaises : `Vec::remove(idx)` puis `Vec::insert(idx, ..)`
— **aucun index ne bouge**, malgré le passage par une valeur possédée.

Conséquence pour la conception : `Systeme::remplacer` (Lot B, testé et
documenté comme la pièce manquante pour la décision 8) s'est révélé
**ne pas être la bonne pièce** une fois le problème de possession
entièrement travaillé — il suppose que l'appelant a déjà en main un
`Box<dyn Astre>` construit, ce qui n'est justement pas possible sans passer
par le trait. **Retiré**, pas gardé « au cas où » : ses deux tests
remplacés par ceux de `lancer`. Une leçon de plus pour `methode.md` : une
primitive testée et documentée peut quand même se révéler être la mauvaise
primitive une fois le problème suivant posé — mieux vaut la retirer que la
garder comme dette silencieuse.

### Le reste du backend

- **`Systeme::calculer_trajectoire(destination, depart, t_depart, vitesse,
  normale_plan)`** : résout un point fixe sur la durée (la position
  d'arrivée dépend de la durée, qui dépend de la position d'arrivée).
  **Convergence mesurée, pas supposée** : un test délibérément défavorable
  (cible qui s'éloigne droit devant le vaisseau, à la moitié de sa vitesse)
  a d'abord fait échouer l'implémentation à `ITERATIONS = 4` — l'erreur ne
  se réduit que d'un facteur `vitesse_cible / vitesse_vaisseau` par
  itération dans ce cas, donc très lentement à un ratio de 0,5. Remonté à
  24 (toujours négligeable en coût : chaque itération n'est qu'un appel à
  `position_future_de`), avec la mesure et le raisonnement consignés dans
  le commentaire du code, pas juste le chiffre.
- **`EnginOrbital::nouveau_pilotable`** : comme `nouveau`, plus une classe
  de vaisseau — calcule et garde `(ClasseVaisseau, StatsVol)` une fois pour
  toutes, depuis la `Station` d'origine, avant sa cuisson en maillage.
  `nouveau`/`nouveau_pilotable` partagent désormais un `nouveau_interne`
  privé (une seule source pour la géométrie orbitale).
- **`Astre::pilotable`/`Astre::stats_vol`** : nouvelles méthodes de trait
  (défauts `false`/`None`), redéfinies par `EnginOrbital` — c'est ce qui
  permet à `Systeme::pilotable_de`/`stats_vol_de` de lire ces informations
  à travers `Box<dyn Astre>`, sans downcasting.
- **Un Starship posté**, pour de vrai (2026-08-16) : `genese/presets.rs`
  ajoute un `EnginOrbital::starship_pose` en orbite terrestre, juste après
  l'ISS — le premier vaisseau pilotable qui existe réellement dans une
  partie, pas seulement dans la vitrine des briques ou la galerie. Nommé
  « STARSHIP », `ORBITE_STARSHIP`/`INCLINAISON_STARSHIP` distincts de ceux
  de l'ISS pour ne pas se confondre à l'écran (26,5°, proche du site réel
  de Starbase). Un test de source existant (D-INT-5,
  `aucune_planete_de_preset_nest_nommee_apres_coup`) a dû gagner une
  exception pour `idx_starship`, exactement comme `idx_iss` avant lui —
  pas un défaut, la garde a fait exactement son travail.

### L'interface : un bouton, un mode, pas un écran de plus

Décision 1 (§4) tenue : pas d'écran de planification séparé. Le flux tient
dans le panneau d'astre existant (`fiche.rs`) et le dispatcheur de clic de
`skymap.rs`, sur le même modèle que le `Interaction` de l'assembleur
(`EnMain`/`Repos`) :

1. Cliquer un vaisseau pilotable ouvre sa fiche, qui montre désormais un
   bouton **LANCER** (`fiche::bouton_lancer`, ancré en bas du panneau —
   pas sous la dernière ligne de texte, dont le nombre varie).
2. Cliquer LANCER arme `Skymap::lancement = Some(idx)`. La fiche affiche
   alors « CHOISIR UNE DESTINATION... » à la place du bouton.
3. Le clic suivant (sélecteur **ou** clic 3D — les deux chemins convergent
   dans `Skymap::tenter_lancement`) résout le trajet : position courante
   du vaisseau, horloge `Systeme::temps()`, vitesse de
   `stats_vol_de`, plan par défaut `Vec3::Y`. Réussite → `Systeme::lancer`
   remplace le vaisseau posté par un `VaisseauEnTransit` au même index, qui
   suit sa trajectoire dès la frame suivante (`Systeme::update`, câblé
   depuis le Lot B). Échec silencieux (destination sans position
   prédictible) ou reclic sur le vaisseau lui-même ou clic dans le vide :
   désarme, rien ne bouge.

Aucun nouvel écran, aucune nouvelle passe de rendu : le bouton est inclus
dans la zone déjà couverte par `sur_ui` (le rectangle entier du panneau),
donc pas de zone de clic à ajouter séparément — juste le clic à
interpréter.

### Tests et vérifications

**11 tests de plus** (Lot A+B était à 714 ; +2 `Systeme::lancer` réussite/
échec/bornes en fait 3, +2 `EnginOrbital::tenter_lancer`, +2 fiche.rs
(`pilotable` reporté, géométrie du bouton), le reste en ajustements de
tests existants) — **716 tests verts**, tous red-checkés individuellement
sauf `fiche::bouton_lancer` (géométrie pure, jugée à faible risque). Détail
notable du red-check : le premier sabotage tenté sur `calculer_trajectoire`
(retirer la boucle de raffinement) a immédiatement rougi avec un écart net
(`vaisseau à [0,0,0], cible à [50,0,0]`) — la garde mord bien sur le
rendez-vous, pas seulement sur un signe de vie.

`cargo build --release` et `cargo clippy --release --all-targets` : **0
avertissement du premier coup** après le câblage complet — plus aucun code
mort, chaque nouvelle méthode a enfin un appelant réel.

⚠️ **Ce qui n'a pas été vérifié** : le clic réel, en jeu. `CAPTURE` est un
outil de capture statique (300 frames puis export), pas un simulateur de
clic — la boucle de mise à jour a été confirmée saine (le Starship posté ne
casse rien sur 300 frames), mais la séquence complète LANCER → choisir une
destination → voir le vaisseau partir n'a pas été rejouée à l'écran. La
logique qui **décide** (où, quand, à quelle vitesse) est testée à fond ; ce
qui reste non vérifié est exactement la même limite que celle déjà admise
pour l'assembleur (`STATE.md`, Lot 4 : « ce qui reste non testé ne doit
être que où le rectangle se pose, jamais ce qu'il signifie »).

### Reste pour le chantier

- **Lot E** : que se passe-t-il à `VaisseauEnTransit::arrive() == true` —
  amarrage (conversion retour en `EnginOrbital`, symétrique de
  `tenter_lancer`), consommation si colonisation (décision 7). Rien n'y est
  câblé : un vaisseau arrivé reste visible, immobile, pour toujours.
- Le plan par défaut (`Vec3::Y`) de `tenter_lancement` (et maintenant de
  `previsualisation_trajectoire`, qui appelle la même fonction — voir Lot D
  ci-dessous) est une simplification pas encore questionnée — un système
  dont le plan orbital n'est pas proche de l'écliptique Y donnerait un
  bombement d'arc dans une direction arbitraire. Pas un bug pour le système
  solaire (plan proche de Y), à revoir si un système très incliné devient
  un cas réel.

## Lot D — Aperçu du trajet + caméra qui suit (2026-08-16)

Les deux morceaux du Lot D (§6 de `conception/trajectoires.md`) tenaient sur
des mécaniques déjà en place ailleurs dans le code — ni l'un ni l'autre
n'a demandé de nouvelle notion.

### La caméra suivait déjà, sans le savoir

Hypothèse à vérifier avant d'écrire quoi que ce soit : `Camera::cible(&sys)`
(`src/camera.rs`) fait `sys.position(self.focus?)`, et `Systeme::position`
lit juste `corps().position` — **rien dans cette chaîne ne regarde comment
la position a été calculée**. Une planète la doit à `orbiter_autour`, un
`VaisseauEnTransit` à `suivre_trajectoire` (câblé dans `Systeme::update`
depuis le Lot B) : la caméra ne fait pas la différence, elle relit juste la
position à chaque frame. Confirmé en lisant le code, pas supposé.
Conséquence : suivre un vaisseau en vol ne demandait **aucune** nouvelle
mécanique de caméra, juste un appel à `set_focus` au bon moment — ajouté
dans `Skymap::tenter_lancement`, sur la branche de succès de
`Systeme::lancer` (jamais sur un échec silencieux ou une annulation, qui ne
doivent rien recentrer).

### L'aperçu : adapter le semis de points existant, pas en réinventer un

`systeme/rendu.rs` avait déjà toute la technique qu'il fallait pour un
tracé de points à espacement écran constant (`px_par_unite`, `pas_points`,
`dessiner_points`) — mais taillée pour des polylignes **fermées** : le `(i
+ pas) % n` de `dessiner_points` boucle le dernier point sur le premier,
ce qui relierait à tort l'arrivée d'un trajet à son départ. Reprise en une
nouvelle fonction, `dessiner_points_ouvert`, qui garde la même logique
d'espacement mais s'arrête au dernier point (`(i + pas).min(n - 1)`, boucle
`while i + 1 < n`) plutôt que de boucler.

Côté échantillonnage, `Trajectoire::polyligne(n)` (nouveau, dans
`systeme/trajectoire.rs`) échantillonne `position(t)` en `n` points
uniformément répartis en temps entre `depart_a()` et `arrivee_a()`, `n`
plancher à 2. Plus simple que son équivalent `Orbite::polyligne` :
`Trajectoire::position` rend déjà des coordonnées **monde** absolues, pas
de `centre` à additionner côté appelant (une orbite, elle, rend des points
relatifs à son foyer, qui peut lui-même bouger).

**Où le brancher sans écran de plus** (décision 1, toujours en vigueur) :
`Calques` (le paquet de bascules déjà passé à `draw_orbites` chaque frame)
a gagné un champ `previsualisation: Option<Vec<Vec3>>`. `Skymap` le
remplit via `previsualisation_trajectoire(cam_info, aspect, m)` : tant
qu'un vaisseau est **armé** (`self.lancement.is_some()`), elle relance un
rayon depuis la souris (`crate::camera::rayon_ecran`, la même fonction
que `Camera::pick`) et appelle `Systeme::pick(&self, ...)` directement —
**pas** `Camera::pick`, qui déplace le focus comme effet de bord ; un
simple survol ne doit ni recentrer la vue ni ouvrir de fiche. Si l'astre
survolé donne un trajet valide (même formule que le lancement réel, voir
plus bas), son échantillonnage à 48 points est dessiné en ambre — sinon
rien.

**Une seule formule pour l'aperçu et le vrai lancement.** Le calcul du
trajet (départ, horloge, vitesse, `calculer_trajectoire`) était dupliqué
entre `tenter_lancement` et l'aperçu dans un premier jet ; factorisé en
`Skymap::trajectoire_vers(idx_vaisseau, idx_destination)`, appelée par les
deux. Sans ça, un écart entre les deux formules aurait fait mentir
l'aperçu — montrer un arc, puis lancer un trajet différent au clic —
silencieusement, sans qu'aucun test ne puisse l'attraper (l'aperçu n'a pas
de test de rendu possible).

`Calques` a perdu son `derive(Copy)` (un `Vec` ne l'est pas) — sans
conséquence : la struct n'était de toute façon construite qu'une fois par
frame, jamais dupliquée.

### Tests et vérifications

**3 tests de plus** sur `Trajectoire::polyligne` (nombre de points
demandé, premier/dernier point exacts, plancher à 2 en dessous) — **719
tests verts**, `cargo clippy --release --all-targets` à 0 avertissement.
Red-check individuel des 3 : sabotage du plancher (1 au lieu de 2) et de
l'exactitude du dernier point (`u = i/n` au lieu de `i/(n-1)`) fait rougir
2 des 3 avec le message attendu (le troisième, sur le compte de points à
`n = 12`, n'était pas concerné par ce sabotage précis — attendu, pas un
défaut de la garde).

`dessiner_points_ouvert` elle-même n'a **aucun test** : comme
`dessiner_points`/`dessiner_pointilles` avant elle, c'est une fonction de
dessin pur (`draw_line_3d`), sans branche logique à red-checker — la
logique qui *décide* de l'arc (`Trajectoire::polyligne`,
`Skymap::trajectoire_vers`) est testée ; ce qui reste non testé est
exactement où les points se posent à l'écran, jamais ce qu'ils signifient
(même limite déjà admise pour l'assembleur, `STATE.md`).

⚠️ **Même limite que le Lot C** : `CAPTURE` ne simule pas la souris, donc
ni l'armement (clic LANCER) ni le survol qui déclenche l'aperçu n'ont pu
être rejoués à l'écran — une capture après 300 frames sans interaction
montre le système au repos, aucun trajet armé. Vérifié autrement : lecture
de code pour la caméra (`Camera::cible` ne distingue pas les mécaniques de
mouvement), tests unitaires pour `polyligne`, et build+clippy propres pour
la partie rendu qui ne peut pas se tester autrement.
