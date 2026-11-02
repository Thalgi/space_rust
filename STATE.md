# STATE — où en est le chantier

> Point de reprise, **2026-11-02**. À lire en premier dans une nouvelle session,
> **en entier** : il tient en deux cents lignes, et c'est délibéré.
>
> Ce fichier dit **où on en est et quoi faire ensuite**, rien de plus. Le
> *récit* de chaque étape — ce qui a été trouvé, les mesures, les red-checks —
> vit dans les journaux de [`docs/suivi/`](docs/suivi/) ; le *pourquoi* de
> chaque système dans [`docs/conception/`](docs/conception/) ; la *discipline*
> dans [`docs/methode.md`](docs/methode.md).
>
> ⚠️ **Il a dérivé deux fois** : 1 968 lignes le 2026-08-10, 670 le 2026-11-02,
> chaque fois en devenant le journal des chantiers en cours. Vidé les deux
> fois sans rien perdre : tout est dans les journaux (voir « Où trouver
> quoi »). **Ne pas recommencer** : un fait qui se raconte va dans un journal ;
> ici, une ligne et un lien.

## Reprendre ici (état au 2026-11-02)

**Chantier en cours : Opti-v2**, en clôture. Branche `chantier-opti-v2`,
worktree `.claude/worktrees/astres-v2` (on y travaille : depuis le dossier
principal, la branche ne se bascule pas). **Poussée** sur `origin`, à jour.
`main` (`5af973a`, poussé) est un ancêtre de la branche : la fusion sera une
avance rapide.

**Vérifié (2026-11-01, O6)** : **992 tests verts**, `cargo clippy
--all-targets` à **0 avertissement**.

⚠️ **Le dossier principal porte du travail non commité** qui n'est pas à
nous : le découpage de `src/vaisseau/generateur.rs` en `generateur/`,
`docs/suivi/dette_vaisseau.md`, et des ajouts à `docs/README.md`,
`docs/suivi/bucketlist_globale.md`, `docs/suivi/stations.md`. **N'y rien
toucher.** À la fusion : sauvegarder ces fichiers dans le scratchpad, en
tirer un patch (`git diff`), les remettre à l'état commité, `git merge
--ff-only`, réappliquer le patch (`git apply --3way`), puis `git restore
--staged` pour les laisser non commités.

### Ce qui a été fait dans la dernière session

| Lot | Résultat | Journal |
|---|---|---|
| **O4** | nuages et bras dessinés dans une cible à 1/2 par axe | [`suivi/opti_v2.md`](docs/suivi/opti_v2.md) |
| **O7** | boules des nuages et jets figés dans des buffers GPU, animés par le shader ; empilement sur un échantillon — T Tauri 158 → ~895 fps (dev) | idem |
| **O5** | RÉSOLUTION 3D (100/75/50 %, rendu Net) et IMAGES (libre/60/30) dans les paramètres | idem |
| **O6** | deux calculs inutiles retirés du shader de l'étoile, à image identique | idem |
| **O8**, premier banc | petit PC modifié (16 Go, pilote à jour) : T Tauri 70 → 7,3 ms par image, pire cas du tour 63 fps (graine 2) | idem, « O8 » |

### À faire ensuite, par ordre

1. **Clore Opti-v2** : dépouiller le banc complémentaire du petit PC (étoiles
   de près, tour en rendu Net, la version d'avant O4 sur le PC modifié pour
   séparer le gain du code de celui du matériel), écrire le bilan d'O8 dans le
   journal, puis **fusionner vers `main`** (procédure ci-dessus) — décision de
   l'utilisateur.
2. **Choisir le chantier suivant** sur ce banc. Pistes relevées : la graine 2
   (limitée par le processeur, 11 ms), le rendu Net plein écran de près
   (l'étoile seule ~12 ms de GPU sur le petit PC), la galerie des planètes.
3. **Genèse V2** : trancher les décisions de G0
   ([`conception/genese_v2.md`](docs/conception/genese_v2.md)), puis G1 — qui
   débloque aussi la part « nombre d'objets » de la qualité (O5).
4. **Planètes** : régler finement `assets/habitabilite.json` avec la genèse
   ([`suivi/planetes.md`](docs/suivi/planetes.md)).
5. **Astres V2** : la **passe de relecture** demandée (*« observer qu'on n'a
   rien loupé »*), jamais faite.
6. **Trajectoires, lot E** : l'arrivée (amarrage, consommation à la
   colonisation) ; un vaisseau arrivé reste immobile.
7. Plus ancien, toujours ouvert : **lister les sous-variantes** dans la palette
   (bloque l'Endurance, étape 5) ; **assembleur** lots 6, 8 et 9 ; **D-INT-2,
   l'économie** ; **D-PIX-3/4** (contours, taille de pixel).

### Dettes ouvertes repérées en chemin

- ⚠️ **Crash intermittent** (texture 32768², `miniquad gl.rs:181`), vu le
  2026-10-03 et le 2026-10-20. **À la prochaine occurrence, la pile
  d'appels** (`$env:RUST_BACKTRACE=1; cargo run --release`) — ne rien corriger
  avant. [`suivi/astres_v2.md`](docs/suivi/astres_v2.md), « Ce qui reste
  ouvert ».
- Un engin vu de près (ISS, Starship) repasse par `draw_mesh`, qui recopie ses
  sommets à chaque image. Elysium : 20 fps en vue dédiée. Même remède, des
  tampons GPU persistants ([`suivi/stations.md`](docs/suivi/stations.md)).
- Le GLSL ne se compile qu'au lancement : aucun test ne le vérifie au-delà de
  `verif::usage_avant_declaration`.
- Le **défilement** n'est câblé que dans la vue des composants ; la colonne de
  la skymap a le même défaut.
- `src/ecran/briques.rs` et `src/ecran/vaisseaux.rs` sont commités mais non
  déclarés : ils ne compilent pas. À supprimer ou à rebrancher.
- Lot 5 d'Astres V2 : le disque passe sous l'horizon dessiné à très longue
  distance (`PX_MIN`) ; la convention de silhouette « mince » jamais essayée.

## Commits

Dernier commit : `453ee73` « Opti-v2 O6 : le shader de l'etoile, deux calculs
inutiles retires ». `git log` fait foi pour le reste.

**Consigne debout (révisée le 2026-09-26) : un commit par lot ou sous-lot**,
sur la branche de chantier, une fois les tests verts et clippy à zéro. Un
commit n'engage rien, il crée un point de repli. La **fusion** vers `main` et
le **push** restent des décisions de l'utilisateur.

⚠️ **Ne jamais employer `git checkout --`, `git restore` ou `git reset --hard`**
sur un fichier contenant du travail non commité (`docs/suivi/stations.md`
§F.8). Sauvegarder dans le scratchpad **avant** de toucher. ⚠️ **Jamais de
`git stash` nu** : la pile est partagée entre les worktrees.

## Les chantiers

| Chantier | Où on en est | Journal · conception |
|---|---|---|
| **Opti-v2** | O1 à O7, O9, O10 livrés ; O8 (banc du petit PC) en cours | [`suivi/opti_v2.md`](docs/suivi/opti_v2.md) · [`conception/opti_v2.md`](docs/conception/opti_v2.md) |
| **Genèse V2** | carte faite (lots G0 à G8), rien de codé ; G0 = décisions à trancher | — · [`conception/genese_v2.md`](docs/conception/genese_v2.md) |
| **Planètes** (type, habitabilité) | P1 à P3 livrés, habitabilité des presets ajustée à la main ; réglage fin avec la genèse | [`suivi/planetes.md`](docs/suivi/planetes.md) · [`conception/planetes.md`](docs/conception/planetes.md) |
| **Astres V2** | **clos** (2026-10-26) : lots 0 à 15 livrés et validés, le 8 abandonné ; reste la passe de relecture | [`suivi/astres_v2.md`](docs/suivi/astres_v2.md) · [`conception/astres_v2.md`](docs/conception/astres_v2.md) |
| **Trajectoires** | lots A à D faits (un Starship vole en jeu) ; lot E à faire | [`suivi/trajectoires.md`](docs/suivi/trajectoires.md) · [`conception/trajectoires.md`](docs/conception/trajectoires.md) |
| **Interface de jeu** | I.0 à I.6 faites ; Skymap V2 (marqueurs) fusionnée, à côté de la V1 — choix entre les deux jamais tranché | [`suivi/interface.md`](docs/suivi/interface.md) · [`conception/interface.md`](docs/conception/interface.md) |
| **Assembleur (VAB)** | lots 1 à 5 clos, rotation faite ; restent 6, 8, 9 | [`suivi/assembleur.md`](docs/suivi/assembleur.md) · [`conception/assembleur.md`](docs/conception/assembleur.md) |
| **Stations et vaisseaux** | ISV, Starship, Stanford, Elysium (chantier initial) clos — d'Elysium, la rotation au bouton et le hangar fermé jamais vus à l'écran ; **Endurance bloquée** par la palette | [`suivi/stations.md`](docs/suivi/stations.md) · [`conception/stations.md`](docs/conception/stations.md) |
| **Dette de `vaisseau/`** | chantier 1 (découpage de `generateur.rs`) fait **dans le dossier principal, non commité** ; chantiers 2 et 3 à faire | `docs/suivi/dette_vaisseau.md` (dossier principal) |
| **Planètes et vue système** (P.1 à P.4) | ✅ clos (2026-08) ; le voile des anneaux de près, mesuré par Opti-v2 (~12 ms sur le petit PC, Saturne plein écran), pas réduit | [`suivi/planetes.md`](docs/suivi/planetes.md), [`suivi/ceintures.md`](docs/suivi/ceintures.md), [`suivi/etoiles.md`](docs/suivi/etoiles.md) |

## Où trouver quoi

| Vous cherchez | C'est dans |
|---|---|
| **La discipline** : red-check, pièges de tests, règles apprises | [`docs/methode.md`](docs/methode.md) |
| L'index commenté de toute la documentation | [`docs/README.md`](docs/README.md) |
| Pièges moteur : pipelines, materials partagés, GPU | [`docs/suivi/technique.md`](docs/suivi/technique.md) |
| Revues de code | [`docs/suivi/revues.md`](docs/suivi/revues.md) |
| La liste globale des fonctionnalités | [`docs/suivi/bucketlist_globale.md`](docs/suivi/bucketlist_globale.md) |
| La vision RTS globale | [`docs/conception/concept_jeu.md`](docs/conception/concept_jeu.md) |
| Les mesures de performance (`PROFIL`, `DIAG`, `DIAG=tour`, `DEFILE`, `SYSTEME`, `FOCUS`) | [`docs/suivi/opti_v2.md`](docs/suivi/opti_v2.md), « Méthode de mesure » et « Le tour de diagnostic » |
| Régénérer le catalogue de fils : `FILS=1 cargo test --release le_catalogue_des_fils` | [`docs/reference/fils.md`](docs/reference/fils.md) |

## Décisions prises (ne pas les rouvrir sans raison)

- **Assembleur** : pièce d'abord, bac à sable sans plafond de coût, capsules
  plutôt que sphères, `SousEnsemble` sérialisé en pièces cuites, pas de roulis
  libre autour du port ([`conception/assembleur.md`](docs/conception/assembleur.md)).
- Les centres de **Stanford et d'Elysium** sont séparés (§F.4).
- **`Composant::nom()` rend la famille**, pas famille + variante.
- **Tout sur rails** : le mode N-corps a été retiré du jeu (2026-08-16).
- **Menu V2** : la sauvegarde des systèmes est mise de côté (le code reste,
  la V1 s'en sert) ; CHOISIR ASTRE bâtit un système autour du type choisi.
- **Prototypes** : les essais visuels vivent à la touche 4 de la vue isolée
  (`ecran/prototypes.rs`), pas dans les galeries ; retirés une fois validés.
- **Lentille gravitationnelle abandonnée** (lot 8) : ne pas la reprendre sans
  lire [`conception/astres_v2_lentille.md`](docs/conception/astres_v2_lentille.md).
- **Nuages des disques à 1/2 par axe** (O4) ; le niveau de détail du shader
  d'étoile selon la taille écarté (O6).

## Questions ouvertes

- **Les mégastructures à taille réelle** : ~1 140 pièces pour l'Elysium de
  1,8 km contre 45 pour l'ISV (§F.3). L'assemblage pièce à pièce ne passera
  pas à l'échelle.
- **Membrures à axe courbe** : aucune primitive n'en trace, les bras des
  mégastructures en ont besoin (§F.1).
- **Étoiles variables à l'œil**, type par type : seul l'éclat de la céphéide
  atteint l'image (conception astres §6 decies).
- **Fins de ligne** : `core.autocrlf=true` contre des fichiers stockés en
  CRLF ; un `.gitattributes` trancherait (décision de l'utilisateur).

## Méthode

Dans **[`docs/methode.md`](docs/methode.md)**. Les quatre règles qu'on oublie
le plus souvent :

- **Red-check systématique** : casser volontairement ce que le test garde,
  vérifier qu'il rougit, rétablir. **Sauvegarder le fichier dans le scratchpad
  avant de le saboter**, et le restaurer par copie — jamais par git.
- ⚠️ **Un sabotage qui ne rougit pas n'accuse pas la garde** : vérifier d'abord
  qu'il produisait vraiment le défaut visé.
- **Une source par fait.** Presque toutes les erreurs de ce projet viennent
  d'une valeur qui en a deux.
- **Mesurer avant d'affirmer**, et **mettre à jour les `.md` à chaque étape**.

## Dette des bouche-trous de l'interface

Consigne de l'utilisateur (2026-08-04) : **poser un bouche-trou** pour ce qui
n'est pas encore fait, et **l'inscrire ici**. Un bouche-trou non listé est un
mensonge à l'écran — il a l'air fini. Les quatre soldés (D-INT-1, 3, 4, 5) sont
racontés dans [`suivi/interface.md`](docs/suivi/interface.md).

| # | Bouche-trou | Où | Ce qu'il faut à la place |
|---|---|---|---|
| **D-INT-2** | **`Tresorerie` figée** : les quatorze quantités sont écrites à la main. | `ecran/bandeau.rs::Tresorerie` | L'économie : production, consommation, coûts, recherche. Chantier de **conception** à part entière (`conception/interface.md` §2.2b). |

## Dettes préexistantes

- **Le hash CPU/GPU** : la même constante vit dans 4 fichiers (1 Rust + 3
  GLSL), rien ne les tient d'accord → [`suivi/planetes.md`](docs/suivi/planetes.md) §7.
- Dette de **L1.4** : ~19 variantes sous-déclarent leur `rayon_local`
  (`MARGE_RAYON = 1.40` la marque). **Se solde au lot 6** de l'assembleur.
- Dette de **L1.6** : `peut_poser` n'a aucun test dédié, faute de consommateur.
- `GenrePort::PoutreBout` n'est posé sur aucun composant — variante morte,
  lot 6.
