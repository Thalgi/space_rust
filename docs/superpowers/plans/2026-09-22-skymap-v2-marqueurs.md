# Skymap V2 — marqueurs de planètes et lunes : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Une nouvelle vue Skymap V2 où chaque planète porte un marqueur triangulaire cliquable, et où la sélection d'une planète affiche les orbites et les marqueurs de ses lunes.

**Architecture:** Fork complet de `src/ecran/skymap.rs` vers `src/ecran/skymap_v2.rs` (V1 ne gagne qu'une ligne mécanique). Nouveau module `src/ecran/marqueur.rs` dont le cœur (`marqueurs_pour`, `plus_proche`, `dessiner`) est générique sur des indices d'astres ; seules deux fonctions de sélection d'indices (planètes racines, lunes d'une planète) l'alimentent. Les orbites des lunes ciblées passent par un nouveau champ `Calques::lunes_de`, parce que le tracé d'orbites vit dans la passe 3D de `RenduStandard::rendre`.

**Tech Stack:** Rust 2021, macroquad (glam `Vec2`/`Vec3`), `cargo test`, `cargo clippy --all-targets`.

**Spec:** [`docs/superpowers/specs/2026-09-22-skymap-v2-marqueurs-design.md`](../specs/2026-09-22-skymap-v2-marqueurs-design.md)

## Global Constraints

- ~~**Ne jamais commiter.** Consigne debout du projet (`STATE.md`, « Commits ») : « no commit i'll say when commit is needed ». Chaque tâche se termine par un **point de commit à signaler** à l'utilisateur, jamais par un `git commit`.~~ ⚠️ **Caduc depuis le 2026-09-26** : le travail se fait désormais sur une branche de chantier, où un commit ne vaut plus engagement mais **point de repli**. La consigne est maintenant « un commit par lot » (`STATE.md`, « Commits »). Barré plutôt que supprimé — ce plan est exécuté et fusionné, c'est une archive.
- **Ne jamais employer `git checkout --`, `git restore` ou `git reset --hard`** sur un fichier (`STATE.md`) : le dépôt contient du travail non commité de l'utilisateur (`src/vaisseau/generateur/`, docs modifiées). Ne pas y toucher.
- **Red-check systématique** (`docs/methode.md`) : chaque test neuf est cassé volontairement sur sa cible, on vérifie qu'il rougit, on rétablit. **Copier le fichier dans le scratchpad avant de le saboter, restaurer par copie — jamais par git.** Scratchpad : `C:\Users\thalgi\AppData\Local\Temp\claude\c--Users-thalgi-Desktop-perso-space-rust\f7e3003b-6e7d-4eca-8161-a725472eb82d\scratchpad`.
- **Code et commentaires en français**, commentaires qui disent *pourquoi*.
- **Pas de test de rendu** (couleurs, disposition) — seules les décisions sont testées.
- `cargo clippy --all-targets` doit finir à **0 avertissement** à la fin de la Tâche 3 (des avertissements `dead_code` transitoires sont attendus à la fin de la Tâche 2, voir sa note).
- Aucun test ne peut construire un vrai système (`genese` exige le contexte graphique) : les tests posent leurs propres corps d'essai via `Systeme::new()` + `Systeme::ajouter`.
- Constantes du spec, à recopier telles quelles : triangle **base 8 px, hauteur 10 px**, pointe en bas sur le point projeté ; `RAYON_HIT_PX = 14.0` ; alpha **0.85** hors survol, **1.0** au survol ; contour blanc 1 px au survol ; cercle de sélection **rayon 8 px**, blanc 50 %, centré sur le centre du triangle.

---

## Carte des fichiers

| Fichier | Responsabilité |
|---|---|
| `src/systeme/mod.rs` | `Systeme::lunes_de` (générique : tous les enfants directs) ; champ `Calques::lunes_de` |
| `src/systeme/rendu.rs` | `Systeme::draw_orbites_lunes_de`, appelée par `draw_orbites` |
| `src/ecran/skymap.rs` (V1) | **une ligne** : `lunes_de: None` dans le littéral `Calques` |
| `src/ecran/marqueur.rs` (nouveau) | `Marqueur`, cœur générique, sélection planètes/lunes, `plus_proche`, `planete_active`, `dessiner` |
| `src/ecran/skymap_v2.rs` (nouveau) | Copie de V1 + branchement des marqueurs |
| `src/ecran/mod.rs`, `src/ecran/accueil.rs`, `src/main.rs` | Navigation vers V2 |
| `src/camera.rs` | Retrait de l'`#[allow(dead_code)]` périmé sur `projeter_ecran` |
| `STATE.md`, `docs/README.md`, `docs/suivi/interface.md` | Journal |

---

### Task 1 : `Systeme::lunes_de`, `Calques::lunes_de`, tracé ciblé des orbites de lunes

**Files:**
- Modify: `src/systeme/mod.rs` (struct `Calques` l. 32-59 ; `impl Systeme`, après `nb_lunes` l. 442-444 ; module `tests_designation`)
- Modify: `src/systeme/rendu.rs` (`draw_orbites` l. 59-172 ; nouvelle méthode après `draw_orbites`)
- Modify: `src/ecran/skymap.rs` l. 317-324 (littéral `Calques`)

**Interfaces:**
- Produces: `pub fn Systeme::lunes_de(&self, parent: usize) -> Vec<usize>` — indices des astres dont `parent() == Some(parent)`, dans l'ordre de `self.astres`, **sans filtre de catégorie** ; vide pour un index invalide.
- Produces: champ `pub lunes_de: Option<usize>` sur `crate::systeme::Calques`.
- Produces: `pub fn Systeme::draw_orbites_lunes_de(&self, parent: usize, col: Color, cam: Vec3)`.

- [ ] **Step 1 : Écrire le test qui échoue**

Dans `src/systeme/mod.rs`, module `tests_designation`, ajouter après le test `deux_astres_coorbitaux_ne_se_confondent_pas` :

```rust
    // **`lunes_de` rend les enfants directs, et seulement eux.** Le filtre
    // porte sur le PARENT, pas sur la catégorie : c'est ce qui le rend
    // générique (une station ou un vaisseau posté autour d'une planète en est
    // aussi l'enfant). Ce sont les appelants qui filtrent par catégorie.
    #[test]
    fn lunes_de_rend_les_enfants_directs_et_rien_d_autre() {
        let mut sys = Systeme::new();
        let etoile = CorpsEssai::poser(&mut sys, Categorie::Etoile, 0.0, None);
        let terre = CorpsEssai::poser(&mut sys, Categorie::Planete, 100.0, None);
        let mars = CorpsEssai::poser(&mut sys, Categorie::Planete, 150.0, None);
        let lune = CorpsEssai::poser(&mut sys, Categorie::Lune, 101.0, Some(terre));
        let phobos = CorpsEssai::poser(&mut sys, Categorie::Lune, 151.0, Some(mars));
        let deimos = CorpsEssai::poser(&mut sys, Categorie::Lune, 152.0, Some(mars));
        // Un engin posté autour de Mars en est aussi l'enfant direct.
        let station = CorpsEssai::poser(&mut sys, Categorie::Engin, 153.0, Some(mars));

        assert_eq!(sys.lunes_de(terre), vec![lune]);
        assert_eq!(sys.lunes_de(mars), vec![phobos, deimos, station]);
        assert!(sys.lunes_de(etoile).is_empty(), "une planète tient à son étoile par son foyer, pas par parent");
        assert!(sys.lunes_de(lune).is_empty(), "une lune sans satellite");
        assert!(sys.lunes_de(999).is_empty(), "index invalide : liste vide, pas de panique");
    }
```

- [ ] **Step 2 : Lancer le test, vérifier qu'il échoue**

Run: `cargo test lunes_de_rend_les_enfants_directs`
Expected: échec de compilation, `no method named 'lunes_de' found for struct 'Systeme'`.

- [ ] **Step 3 : Implémenter `lunes_de`**

Dans `src/systeme/mod.rs`, juste après `pub fn nb_lunes(...)` (l. 442-444) :

```rust
    /// **Indices des enfants directs** de `parent` : tout astre dont
    /// `parent()` le désigne, dans l'ordre de `self.astres`.
    ///
    /// Générique à dessein : pas de filtre de catégorie. Une lune en est un
    /// enfant, mais aussi un engin posté autour de la planète (l'ISS autour de
    /// la Terre). C'est à l'appelant de choisir ce qu'il garde — les marqueurs
    /// de la Skymap V2 ne gardent que les lunes.
    pub fn lunes_de(&self, parent: usize) -> Vec<usize> {
        self.astres
            .iter()
            .enumerate()
            .filter(|(_, a)| a.parent() == Some(parent))
            .map(|(i, _)| i)
            .collect()
    }
```

- [ ] **Step 4 : Lancer le test, vérifier qu'il passe**

Run: `cargo test lunes_de_rend_les_enfants_directs`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5 : Red-check**

Copier `src/systeme/mod.rs` dans le scratchpad (`mod.rs.avant_redcheck`). Deux sabotages, un à la fois, chacun restauré par copie avant le suivant :
1. remplacer le filtre par `a.categorie() == Categorie::Lune && a.parent() == Some(parent)` → attendu **rouge** (la station manque dans la liste de Mars) ;
2. remplacer le filtre par `a.parent().is_some()` → attendu **rouge** (la Terre reçoit les lunes de Mars).

Run après chaque sabotage : `cargo test lunes_de_rend_les_enfants_directs` → FAIL. Restaurer le fichier par copie depuis le scratchpad, relancer → PASS.

- [ ] **Step 6 : Ajouter le champ `Calques::lunes_de`**

Dans `src/systeme/mod.rs`, struct `Calques`, après le champ `lunes` (l. 38-39) :

```rust
    /// Orbites des lunes de **cette** planète seulement, à pleine opacité —
    /// indépendamment de [`Self::lunes`], le calque global tout-ou-rien.
    ///
    /// C'est ce que la Skymap V2 affiche quand une planète (ou une de ses
    /// lunes) est sélectionnée. `None` : rien de ciblé.
    pub lunes_de: Option<usize>,
```

- [ ] **Step 7 : Ajouter `lunes_de: None` au littéral de V1**

Dans `src/ecran/skymap.rs`, l. 317-324, le littéral devient :

```rust
                crate::systeme::Calques {
                    planetes: self.menu.orbites,
                    etoiles: self.menu.orbites_etoiles,
                    lunes: self.menu.orbites_lunes,
                    lunes_de: None,
                    zone: self.menu.zone,
                    focus: self.cam.focus(),
                    previsualisation,
                },
```

C'est la **seule** retouche de V1 de tout le plan : elle laisse son comportement strictement inchangé.

- [ ] **Step 8 : Implémenter `draw_orbites_lunes_de` et l'appeler depuis `draw_orbites`**

Dans `src/systeme/rendu.rs`, dans `draw_orbites`, le déstructurage (l. 60-67) devient :

```rust
        let Calques {
            planetes: orbites_planetes,
            etoiles: orbites_etoiles,
            lunes: orbites_lunes,
            lunes_de,
            zone: montrer_zone,
            focus,
            previsualisation,
        } = c;
```

Puis, juste après le bloc `if orbites_lunes { ... }` (qui se termine l. 145), ajouter :

```rust
        // Orbites des lunes d'UNE planète (Skymap V2) : pleine opacité, sans
        // passer par l'atténuation du focus — elles ne s'affichent que quand
        // cette planète est sélectionnée, donc toujours en avant.
        if let Some(parent) = lunes_de {
            self.draw_orbites_lunes_de(parent, COL_LUNES, cam);
        }
```

Et ajouter la méthode dans le même `impl Systeme`, juste après la fin de `draw_orbites` :

```rust
    /// Trace l'orbite de chaque enfant direct de `parent`, centrée sur lui.
    ///
    /// Même semis de points que les autres calques (`dessiner_points`), donc
    /// même espacement écran. Seuls les corps qui **ont** une orbite de lune
    /// (`orbite_lune` non vide) dessinent quelque chose : un engin posté n'en
    /// porte pas, il ne trace donc rien ici.
    pub fn draw_orbites_lunes_de(&self, parent: usize, col: Color, cam: Vec3) {
        let Some(centre) = self.astres.get(parent).map(|a| a.corps().position) else {
            return;
        };
        for a in &self.astres {
            if a.parent() != Some(parent) {
                continue;
            }
            let pts = a.orbite_lune();
            if pts.is_empty() {
                continue;
            }
            dessiner_points(pts, centre, col, cam);
        }
    }
```

- [ ] **Step 9 : Vérifier que tout compile et que la suite reste verte**

Run: `cargo test`
Expected: tous les tests passent (le total d'avant **+1**). Noter le total obtenu pour la Tâche 4.

Run: `cargo clippy --all-targets`
Expected: 0 avertissement (`lunes_de` n'a pas encore d'appelant hors test, mais `pub` sur un module interne peut déclencher `dead_code` — si clippy le signale, c'est attendu jusqu'à la Tâche 3 : le noter, ne **pas** ajouter d'`allow`).

- [ ] **Step 10 : Point de commit — signaler, ne pas commiter**

Dire à l'utilisateur : « Point de commit possible : `Systeme::lunes_de` + calque ciblé des orbites de lunes (fichiers : `src/systeme/mod.rs`, `src/systeme/rendu.rs`, `src/ecran/skymap.rs` une ligne). »

---

### Task 2 : Le cœur de `ecran/marqueur.rs` (décisions pures, testées)

**Files:**
- Create: `src/ecran/marqueur.rs`
- Modify: `src/ecran/mod.rs` (déclaration du module)

**Interfaces:**
- Consumes: `Systeme::lunes_de(parent) -> Vec<usize>` (Tâche 1) ; `crate::camera::projeter_ecran(p: Vec3, ecran: Vec2, cam: &CameraInfo, aspect: f32) -> Option<Vec2>` (existant) ; `Systeme::categorie_de`, `Systeme::parent_de`, `Systeme::position`, `Systeme::nb_astres` (existants).
- Produces (tous `pub(crate)` via `pub` dans un module `mod marqueur;` privé à `ecran`) :
  - `pub struct Marqueur { pub idx: usize, pub ecran: Vec2 }`
  - `pub const RAYON_HIT_PX: f32 = 14.0;`
  - `pub fn marqueurs_pour(sys: &Systeme, indices: impl IntoIterator<Item = usize>, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur>`
  - `pub fn marqueurs_planetes(sys: &Systeme, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur>`
  - `pub fn marqueurs_lunes(sys: &Systeme, parent: usize, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur>`
  - `pub fn plus_proche(marqueurs: &[Marqueur], souris: Vec2, rayon_px: f32) -> Option<usize>` — rend l'**index d'astre** (`Marqueur::idx`), pas la position dans la liste.
  - `pub fn planete_active(sys: &Systeme, selection: Option<usize>) -> Option<usize>`

⚠️ **Avertissements attendus en fin de tâche** : rien n'appelle encore ces fonctions hors des tests, `cargo clippy --all-targets` signalera du `dead_code` sur la cible binaire. C'est transitoire (la Tâche 3 les branche) — **ne pas** poser d'`#[allow(dead_code)]` (`docs/methode.md` : `dead_code` ne sait pas ce qu'est un test).

- [ ] **Step 1 : Créer le module avec ses tests (qui échouent)**

Créer `src/ecran/marqueur.rs` :

```rust
//! **Marqueurs d'astres** : un petit triangle à taille écran fixe, planté sur
//! la position projetée d'un astre — pour voir et cliquer un corps qui, à
//! l'échelle vraie, fait moins d'un pixel (Skymap V2,
//! `docs/superpowers/specs/2026-09-22-skymap-v2-marqueurs-design.md`).
//!
//! **Le cœur est générique** : `marqueurs_pour`, `plus_proche` et `dessiner`
//! ne manipulent que des indices d'astres, sans rien savoir de leur
//! catégorie. Seules les fonctions de **sélection d'indices** en décident —
//! aujourd'hui planètes racines et lunes. Marquer demain un vaisseau, une
//! station ou une étoile, c'est écrire une fonction de sélection de plus, pas
//! retoucher le cœur.
//!
//! Tout ce qui **décide** est pur et testé ici ; seul `dessiner` touche
//! macroquad (`docs/methode.md` : pas de test de rendu).

use crate::astre::{CameraInfo, Categorie};
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Un marqueur : l'astre qu'il désigne, et où il se projette à l'écran (px).
///
/// Rien de propre à un type d'astre : un marqueur de planète et un marqueur
/// de lune sont exactement la même donnée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marqueur {
    pub idx: usize,
    pub ecran: Vec2,
}

/// Rayon de détection du survol et du clic, en pixels écran.
///
/// Plus large que le triangle (8 × 10 px) : on doit pouvoir le cliquer sans
/// viser au pixel près — même esprit que la marge de `Systeme::pick`.
pub const RAYON_HIT_PX: f32 = 14.0;

/// Le point projeté tombe-t-il dans l'écran ? Bord gauche/haut inclus,
/// bord droit/bas exclu — un pixel d'abscisse `ecran.x` n'existe pas.
fn dans_cadre(p: Vec2, ecran: Vec2) -> bool {
    p.x >= 0.0 && p.x < ecran.x && p.y >= 0.0 && p.y < ecran.y
}

/// **Cœur générique** : un marqueur par astre de `indices`, projeté à l'écran.
///
/// Deux cas exclus, et c'est le seul filtrage ici :
/// - **derrière la caméra** : `projeter_ecran` rend `None` (une profondeur
///   négative retournerait l'image et donnerait des pixels plausibles) ;
/// - **hors du cadre** : rejeté **à la source**, avant d'entrer dans le
///   `Vec`. Un marqueur hors champ ne coûte ainsi ni test de survol ni appel
///   de dessin par frame — la demande « disparaît d'une manière à économiser
///   de l'énergie ».
pub fn marqueurs_pour(
    sys: &Systeme,
    indices: impl IntoIterator<Item = usize>,
    cam: &CameraInfo,
    ecran: Vec2,
    aspect: f32,
) -> Vec<Marqueur> {
    indices
        .into_iter()
        .filter(|&idx| idx < sys.nb_astres())
        .filter_map(|idx| {
            let p = crate::camera::projeter_ecran(sys.position(idx), ecran, cam, aspect)?;
            dans_cadre(p, ecran).then_some(Marqueur { idx, ecran: p })
        })
        .collect()
}

/// Indices des planètes **racines** : catégorie `Planete` et sans parent.
///
/// Le « sans parent » est une ceinture de sécurité : aujourd'hui une
/// `Planete` qui a un parent se déclare `Lune` (`planete/mod.rs`), mais un
/// corps qui recevrait à la fois un marqueur de planète et un de lune serait
/// désigné deux fois.
fn indices_planetes_racines(sys: &Systeme) -> impl Iterator<Item = usize> + '_ {
    (0..sys.nb_astres())
        .filter(|&i| sys.categorie_de(i) == Some(Categorie::Planete) && sys.parent_de(i).is_none())
}

/// Un marqueur par planète racine. Ne fait que choisir les indices.
pub fn marqueurs_planetes(sys: &Systeme, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur> {
    marqueurs_pour(sys, indices_planetes_racines(sys), cam, ecran, aspect)
}

/// Un marqueur par **lune** de `parent`.
///
/// `Systeme::lunes_de` rend tous les enfants directs ; on ne garde que la
/// catégorie `Lune`. Sans ce filtre, l'ISS (enfant de la Terre) recevrait un
/// marqueur — hors du périmètre de cette itération.
pub fn marqueurs_lunes(sys: &Systeme, parent: usize, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur> {
    let lunes = sys
        .lunes_de(parent)
        .into_iter()
        .filter(|&i| sys.categorie_de(i) == Some(Categorie::Lune));
    marqueurs_pour(sys, lunes, cam, ecran, aspect)
}

/// L'astre dont le marqueur est le plus proche de `souris`, à `rayon_px` au
/// plus (bord inclus). Rend l'**index d'astre**, pas la place dans la liste.
///
/// À distance égale, le premier de la liste l'emporte : comparaison stricte,
/// donc déterministe.
pub fn plus_proche(marqueurs: &[Marqueur], souris: Vec2, rayon_px: f32) -> Option<usize> {
    let mut meilleur: Option<(f32, usize)> = None;
    for m in marqueurs {
        let d = m.ecran.distance(souris);
        if d <= rayon_px && meilleur.is_none_or(|(bd, _)| d < bd) {
            meilleur = Some((d, m.idx));
        }
    }
    meilleur.map(|(_, i)| i)
}

/// La planète dont on montre les lunes, selon ce qui est sélectionné.
///
/// Une planète sélectionnée : elle. Une **lune** sélectionnée : sa planète —
/// sans quoi cliquer une lune ferait disparaître les marqueurs de ses sœurs.
/// Tout le reste (étoile, engin, ceinture) : rien.
///
/// Décidé sur la **catégorie**, pas sur `parent_de` : l'ISS a la Terre pour
/// parent, et la sélectionner ne doit pas afficher les lunes de la Terre.
pub fn planete_active(sys: &Systeme, selection: Option<usize>) -> Option<usize> {
    let idx = selection?;
    match sys.categorie_de(idx)? {
        Categorie::Planete => Some(idx),
        Categorie::Lune => sys.parent_de(idx),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astre::{Astre, CorpsBase};
    use crate::camera::base_orbite;

    const ECRAN: Vec2 = Vec2::new(1000.0, 700.0);
    const ASPECT: f32 = 1000.0 / 700.0;

    /// Corps d'essai : une catégorie, une position, un parent éventuel. Ne
    /// dessine rien, ne tire aucun aléatoire (`genese` exige le contexte
    /// graphique, aucun test ne peut bâtir un vrai système).
    struct Corps {
        base: CorpsBase,
        cat: Categorie,
        parent: Option<usize>,
    }

    impl Astre for Corps {
        fn categorie(&self) -> Categorie {
            self.cat
        }
        fn corps(&self) -> &CorpsBase {
            &self.base
        }
        fn corps_mut(&mut self) -> &mut CorpsBase {
            &mut self.base
        }
        fn update(&mut self, _dt: f32) {}
        fn draw(&mut self, _cam: &CameraInfo) {}
        fn parent(&self) -> Option<usize> {
            self.parent
        }
    }

    fn poser(sys: &mut Systeme, cat: Categorie, pos: Vec3, parent: Option<usize>) -> usize {
        sys.ajouter(Box::new(Corps { base: CorpsBase::new(pos, 1.0, 1.0), cat, parent }))
    }

    /// Caméra en (0, 0, 20) qui regarde vers -Z : l'origine tombe au centre de
    /// l'écran, +X part à droite. À 20 unités, le bord droit est à
    /// x ≈ 20 · tan(22,5°) · aspect ≈ 11,8.
    fn camera() -> CameraInfo {
        let (right, up, forward) = base_orbite(0.0, 0.0);
        CameraInfo {
            pos: -forward * 20.0,
            right,
            up,
            forward,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        }
    }

    fn m(idx: usize, x: f32, y: f32) -> Marqueur {
        Marqueur { idx, ecran: vec2(x, y) }
    }

    // **Le cœur générique rejette à la source ce qui ne se voit pas** :
    // derrière la caméra, et hors du cadre. C'est ce qui fait qu'un marqueur
    // hors champ ne coûte rien par la suite.
    #[test]
    fn marqueurs_pour_rejette_derriere_et_hors_cadre() {
        let mut sys = Systeme::new();
        let centre = poser(&mut sys, Categorie::Planete, Vec3::ZERO, None);
        let hors_cadre = poser(&mut sys, Categorie::Planete, vec3(50.0, 0.0, 0.0), None);
        let derriere = poser(&mut sys, Categorie::Planete, vec3(0.0, 0.0, 30.0), None);

        let ms = marqueurs_pour(&sys, [centre, hors_cadre, derriere, 999], &camera(), ECRAN, ASPECT);

        assert_eq!(ms.len(), 1, "seul l'astre au centre doit être marqué : {ms:?}");
        assert_eq!(ms[0].idx, centre);
        assert!(ms[0].ecran.distance(ECRAN * 0.5) < 0.5, "l'origine doit tomber au centre : {:?}", ms[0].ecran);
    }

    // La frontière du cadre, tenue sur le prédicat lui-même : bord
    // gauche/haut inclus, bord droit/bas exclu.
    #[test]
    fn le_cadre_inclut_le_bord_haut_gauche_et_exclut_le_bord_bas_droit() {
        assert!(dans_cadre(vec2(0.0, 0.0), ECRAN));
        assert!(dans_cadre(vec2(999.9, 699.9), ECRAN));
        assert!(!dans_cadre(vec2(1000.0, 10.0), ECRAN));
        assert!(!dans_cadre(vec2(10.0, 700.0), ECRAN));
        assert!(!dans_cadre(vec2(-0.1, 10.0), ECRAN));
        assert!(!dans_cadre(vec2(10.0, -0.1), ECRAN));
    }

    // **Planètes racines seulement**, et **lunes seulement** : ni l'étoile,
    // ni l'engin posté autour de la planète (le cas de l'ISS autour de la
    // Terre), ni une planète qui aurait un parent.
    #[test]
    fn la_selection_ne_garde_que_planetes_racines_et_lunes() {
        let mut sys = Systeme::new();
        let etoile = poser(&mut sys, Categorie::Etoile, Vec3::ZERO, None);
        let terre = poser(&mut sys, Categorie::Planete, vec3(1.0, 0.0, 0.0), None);
        let lune = poser(&mut sys, Categorie::Lune, vec3(1.2, 0.0, 0.0), Some(terre));
        let iss = poser(&mut sys, Categorie::Engin, vec3(1.1, 0.0, 0.0), Some(terre));
        let planete_enfant = poser(&mut sys, Categorie::Planete, vec3(1.4, 0.0, 0.0), Some(terre));
        let mars = poser(&mut sys, Categorie::Planete, vec3(-2.0, 0.0, 0.0), None);

        let idx = |v: Vec<Marqueur>| v.into_iter().map(|m| m.idx).collect::<Vec<_>>();
        let planetes = idx(marqueurs_planetes(&sys, &camera(), ECRAN, ASPECT));
        assert_eq!(planetes, vec![terre, mars], "étoile {etoile}, lune, ISS et planète-enfant exclues");
        assert!(!planetes.contains(&planete_enfant));

        let lunes = idx(marqueurs_lunes(&sys, terre, &camera(), ECRAN, ASPECT));
        assert_eq!(lunes, vec![lune], "l'ISS ({iss}) n'est pas une lune");
        assert!(marqueurs_lunes(&sys, mars, &camera(), ECRAN, ASPECT).is_empty());
    }

    #[test]
    fn plus_proche_prend_le_plus_proche_sous_le_rayon() {
        let ms = [m(1, 100.0, 100.0), m(2, 110.0, 100.0)];
        assert_eq!(plus_proche(&ms, vec2(108.0, 100.0), 14.0), Some(2), "le plus proche gagne, pas le premier");
        assert_eq!(plus_proche(&ms, vec2(102.0, 100.0), 14.0), Some(1));
    }

    #[test]
    fn plus_proche_respecte_le_rayon_bord_inclus() {
        let ms = [m(7, 100.0, 100.0)];
        assert_eq!(plus_proche(&ms, vec2(100.0, 114.0), 14.0), Some(7), "à exactement 14 px : dedans");
        assert_eq!(plus_proche(&ms, vec2(100.0, 115.0), 14.0), None, "à 15 px : dehors");
        assert_eq!(plus_proche(&[], vec2(0.0, 0.0), 14.0), None, "liste vide");
    }

    // À distance égale, le premier de la liste : pas d'ambiguïté possible.
    #[test]
    fn plus_proche_departage_par_l_ordre_de_la_liste() {
        let ms = [m(3, 90.0, 100.0), m(4, 110.0, 100.0)];
        assert_eq!(plus_proche(&ms, vec2(100.0, 100.0), 14.0), Some(3));
    }

    #[test]
    fn la_planete_active_suit_la_selection() {
        let mut sys = Systeme::new();
        let etoile = poser(&mut sys, Categorie::Etoile, Vec3::ZERO, None);
        let terre = poser(&mut sys, Categorie::Planete, vec3(1.0, 0.0, 0.0), None);
        let lune = poser(&mut sys, Categorie::Lune, vec3(1.2, 0.0, 0.0), Some(terre));
        let iss = poser(&mut sys, Categorie::Engin, vec3(1.1, 0.0, 0.0), Some(terre));

        assert_eq!(planete_active(&sys, None), None, "rien de sélectionné");
        assert_eq!(planete_active(&sys, Some(terre)), Some(terre), "la planète elle-même");
        assert_eq!(planete_active(&sys, Some(lune)), Some(terre), "une lune -> sa planète");
        assert_eq!(planete_active(&sys, Some(etoile)), None, "une étoile n'a pas de lunes à montrer");
        assert_eq!(planete_active(&sys, Some(iss)), None, "l'ISS a un parent mais n'est pas une lune");
        assert_eq!(planete_active(&sys, Some(999)), None, "index invalide");
    }
}
```

Dans `src/ecran/mod.rs`, ajouter la ligne `mod marqueur;` entre `mod liste` / `mod galerie` en respectant l'ordre existant — concrètement après la ligne `mod icone;` (l. 14) :

```rust
mod icone;
mod marqueur;
pub(crate) mod liste;
```

Note : ce fichier-ci **est** l'implémentation — le module n'existe pas encore, il n'y a pas d'étape « test qui échoue faute de fonction » séparée qui ait du sens pour un module neuf. Le red-check (Step 3) tient ce rôle.

- [ ] **Step 2 : Lancer les tests**

Run: `cargo test marqueur`
Expected: les 7 tests du module passent (`test result: ok. 7 passed`, plus éventuellement d'autres tests dont le nom contient `marqueur`).

- [ ] **Step 3 : Red-check, un sabotage à la fois**

Copier `src/ecran/marqueur.rs` dans le scratchpad (`marqueur.rs.avant_redcheck`). Pour chaque ligne ci-dessous : appliquer le sabotage, lancer `cargo test marqueur`, constater **le test nommé rouge**, restaurer par copie, relancer vert.

| Sabotage | Test qui doit rougir |
|---|---|
| dans `marqueurs_pour`, remplacer `dans_cadre(p, ecran).then_some(...)` par `Some(Marqueur { idx, ecran: p })` | `marqueurs_pour_rejette_derriere_et_hors_cadre` |
| dans `dans_cadre`, `p.x < ecran.x` → `p.x <= ecran.x` | `le_cadre_inclut_le_bord_haut_gauche_et_exclut_le_bord_bas_droit` |
| dans `marqueurs_lunes`, retirer le `.filter(...)` sur `Categorie::Lune` | `la_selection_ne_garde_que_planetes_racines_et_lunes` |
| dans `indices_planetes_racines`, retirer `&& sys.parent_de(i).is_none()` | `la_selection_ne_garde_que_planetes_racines_et_lunes` |
| dans `plus_proche`, `d < bd` → `d <= bd` | `plus_proche_departage_par_l_ordre_de_la_liste` |
| dans `plus_proche`, `d <= rayon_px` → `d < rayon_px` | `plus_proche_respecte_le_rayon_bord_inclus` |
| dans `plus_proche`, remplacer toute la boucle par `marqueurs.iter().find(\|m\| m.ecran.distance(souris) <= rayon_px).map(\|m\| m.idx)` | `plus_proche_prend_le_plus_proche_sous_le_rayon` |
| dans `planete_active`, remplacer le `match` par `sys.parent_de(idx).or(Some(idx))` | `la_planete_active_suit_la_selection` |

⚠️ `docs/methode.md` : si un sabotage reste vert, vérifier d'abord qu'il produisait **vraiment** le défaut visé avant de conclure que le test est faible ; s'il est faible, le muscler — jamais l'admettre « suffisamment proche ».

- [ ] **Step 4 : Vérifier la suite complète**

Run: `cargo test`
Expected: tout vert, total = total de la Tâche 1 **+ 7**.

Run: `cargo clippy --all-targets`
Expected: seulement des `dead_code` sur les éléments pub de `ecran::marqueur` (et éventuellement `Systeme::lunes_de`) — transitoires, levés en Tâche 3. Aucun autre avertissement.

- [ ] **Step 5 : Point de commit — signaler, ne pas commiter**

« Point de commit possible : cœur du module `ecran/marqueur.rs` + 7 tests red-checkés (fichiers : `src/ecran/marqueur.rs`, `src/ecran/mod.rs`). »

---

### Task 3 : Dessin des marqueurs et écran Skymap V2 branché

**Files:**
- Modify: `src/ecran/marqueur.rs` (ajout de `dessiner`)
- Create: `src/ecran/skymap_v2.rs` (copie de `src/ecran/skymap.rs`, puis retouches ci-dessous)
- Modify: `src/ecran/mod.rs` (l. 26 et l. 38)
- Modify: `src/ecran/accueil.rs` (enum `Cible` l. 6-30, `BLOC_ASTRES` l. 36-44)
- Modify: `src/main.rs` (import l. 27-31, enum `Etat` l. 49-60, `match cible` l. 107-144, `match &mut etat` l. 104-197)
- Modify: `src/camera.rs` l. 93

**Interfaces:**
- Consumes: tout ce que produit la Tâche 2 ; `Calques::lunes_de` (Tâche 1) ; `super::selecteur::teinte_astre(sys: &Systeme, idx: usize, cat: Categorie) -> Color` (existant, `src/ecran/selecteur.rs:237` — la même teinte que la pastille du sélecteur, relevée pour se voir sur fond nuit).
- Produces: `pub fn dessiner(sys: &Systeme, marqueurs: &[Marqueur], survole: Option<usize>, selectionne: Option<usize>)` ; `pub struct SkymapV2` avec `pub fn new() -> Self` et `pub fn frame(&mut self) -> bool` ; `Cible::SkymapV2`.

- [ ] **Step 1 : Ajouter `dessiner` à `marqueur.rs`**

Dans `src/ecran/marqueur.rs`, juste avant `#[cfg(test)]` :

```rust
/// Triangle plein, base 8 px, hauteur 10 px, pointe en bas **sur** le point
/// projeté — une punaise plantée sur l'astre.
const DEMI_BASE_PX: f32 = 4.0;
const HAUTEUR_PX: f32 = 10.0;
/// Cercle de sélection, centré sur le centre du triangle. 8 et non 6 : les
/// coins de la base sont à 6,4 px de ce centre, un cercle de 6 les couperait.
const RAYON_SELECTION_PX: f32 = 8.0;

/// Dessine les marqueurs. **Passe interface uniquement** — ne décide de rien :
/// quels marqueurs, lequel est survolé, lequel est sélectionné, tout arrive
/// déjà tranché.
///
/// Teinte : celle de la pastille du sélecteur (`selecteur::teinte_astre`),
/// pour qu'un même astre ait la même couleur dans la colonne et sur la carte.
pub fn dessiner(sys: &Systeme, marqueurs: &[Marqueur], survole: Option<usize>, selectionne: Option<usize>) {
    for m in marqueurs {
        let cat = sys.categorie_de(m.idx).unwrap_or(Categorie::Planete);
        let mut c = super::selecteur::teinte_astre(sys, m.idx, cat);
        let est_survole = survole == Some(m.idx);
        c.a = if est_survole { 1.0 } else { 0.85 };

        let pointe = m.ecran;
        let gauche = vec2(pointe.x - DEMI_BASE_PX, pointe.y - HAUTEUR_PX);
        let droite = vec2(pointe.x + DEMI_BASE_PX, pointe.y - HAUTEUR_PX);
        draw_triangle(pointe, gauche, droite, c);
        if est_survole {
            draw_triangle_lines(pointe, gauche, droite, 1.0, WHITE);
        }
        if selectionne == Some(m.idx) {
            let centre = vec2(pointe.x, pointe.y - HAUTEUR_PX * 0.5);
            draw_circle_lines(centre.x, centre.y, RAYON_SELECTION_PX, 1.0, Color::new(1.0, 1.0, 1.0, 0.5));
        }
    }
}
```

- [ ] **Step 2 : Copier l'écran V1**

Run (Bash) : `cp src/ecran/skymap.rs src/ecran/skymap_v2.rs`

- [ ] **Step 3 : Renommer et nettoyer la copie**

Dans `src/ecran/skymap_v2.rs` :

1. l. 10 : `use super::{bandeau, fiche, liste, selecteur};` → `use super::{bandeau, fiche, liste, marqueur, selecteur};`
2. l. 13 : **supprimer** `use crate::starmap::Destination;` (seul `depuis_destination` s'en servait, et V2 n'est pas une destination de la Starmap).
3. l. 34-35 : 
   ```rust
   /// **Skymap V2** : la vue système, plus des marqueurs triangulaires sur les
   /// planètes et, quand une planète est sélectionnée, sur ses lunes — avec les
   /// orbites de ces lunes. Fork de `Skymap` (V1 reste intacte), voir
   /// `docs/superpowers/specs/2026-09-22-skymap-v2-marqueurs-design.md`.
   pub struct SkymapV2 {
   ```
4. l. 91 : `impl Skymap {` → `impl SkymapV2 {`
5. **Supprimer** la fonction `depuis_destination` entière (l. 125-144, doc comprise) : sans appelant, elle serait du code mort.

- [ ] **Step 4 : Ajouter la méthode qui rassemble les marqueurs**

Dans `impl SkymapV2`, juste avant `fn tenter_lancement` :

```rust
    /// Les marqueurs de la frame : toutes les planètes, plus les lunes de la
    /// planète active s'il y en a une. Une seule source pour le survol, le
    /// clic et le dessin — deux listes calculées séparément finiraient par ne
    /// plus désigner la même chose.
    fn marqueurs(&self, cam_info: &CameraInfo, aspect: f32) -> Vec<marqueur::Marqueur> {
        let ecran = vec2(screen_width(), screen_height());
        let mut ms = marqueur::marqueurs_planetes(&self.sys, cam_info, ecran, aspect);
        if let Some(p) = marqueur::planete_active(&self.sys, self.fiche_ouverte) {
            ms.extend(marqueur::marqueurs_lunes(&self.sys, p, cam_info, ecran, aspect));
        }
        ms
    }
```

- [ ] **Step 5 : Survol et clic — marqueur d'abord, pick 3D en repli**

Dans `frame()` de `skymap_v2.rs`, remplacer :

```rust
        let (cam_info, cam3d) = self.cam.construire(target, aspect);
        if clic && !sur_ui {
            let pick = self.cam.pick(&self.sys, &cam_info, aspect);
```

par :

```rust
        let (cam_info, cam3d) = self.cam.construire(target, aspect);
        // Survol : le marqueur sous la souris, jamais à travers l'interface.
        let survole = if sur_ui {
            None
        } else {
            marqueur::plus_proche(&self.marqueurs(&cam_info, aspect), m, marqueur::RAYON_HIT_PX)
        };
        if clic && !sur_ui {
            // Un marqueur touché désigne son astre — même suite qu'un pick 3D
            // réussi, `set_focus` compris (c'est ce que fait `Camera::pick`).
            // Sinon, repli sur le pick 3D : étoiles, engins, tout ce qui n'a
            // pas de marqueur reste cliquable comme en V1.
            let pick = match survole {
                Some(idx) => {
                    self.cam.set_focus(idx);
                    Some(idx)
                }
                None => self.cam.pick(&self.sys, &cam_info, aspect),
            };
```

Le reste du bloc (`if let Some(idx_vaisseau) = self.lancement { ... } else { self.fiche_ouverte = pick; }`) est **inchangé**.

- [ ] **Step 6 : L'aperçu de trajet suit la même priorité**

Dans `skymap_v2.rs`, remplacer l'appel :

```rust
        let previsualisation = self.previsualisation_trajectoire(&cam_info, aspect, m);
```

par :

```rust
        let previsualisation = self.previsualisation_trajectoire(&cam_info, aspect, m, survole);
```

et la fonction `previsualisation_trajectoire` entière par :

```rust
    /// Aperçu du trajet en cours d'armement (Lot D,
    /// `docs/conception/trajectoires.md` §6) : l'arc qu'un lancement
    /// immédiat vers l'astre **survolé** produirait. `None` hors armement,
    /// souris dans le vide, ou survol du vaisseau armé lui-même.
    ///
    /// Le survol d'un marqueur l'emporte, comme pour le clic ; sinon repli
    /// sur `Systeme::pick` (`&self`, sans effet de bord) plutôt que
    /// `Camera::pick`, qui déplacerait le focus pour un simple survol.
    fn previsualisation_trajectoire(
        &self,
        cam_info: &CameraInfo,
        aspect: f32,
        m: Vec2,
        survole: Option<usize>,
    ) -> Option<Vec<Vec3>> {
        let idx_vaisseau = self.lancement?;
        let idx_destination = match survole {
            Some(idx) => idx,
            None => {
                let ecran = vec2(screen_width(), screen_height());
                let dir = crate::camera::rayon_ecran(m, ecran, cam_info, aspect);
                self.sys.pick(cam_info.pos, dir)?
            }
        };
        if idx_destination == idx_vaisseau {
            return None;
        }
        Some(self.trajectoire_vers(idx_vaisseau, idx_destination)?.polyligne(48))
    }
```

- [ ] **Step 7 : Calques — lunes globales coupées, lunes ciblées**

Dans `frame()` de `skymap_v2.rs`, le littéral `Calques` (hérité de la Tâche 1) devient :

```rust
                crate::systeme::Calques {
                    planetes: self.menu.orbites,
                    etoiles: self.menu.orbites_etoiles,
                    // Le calque global tout-ou-rien est coupé en V2 : les
                    // orbites de lunes n'apparaissent que pour la planète
                    // sélectionnée (spec §6.4), quel que soit le bouton.
                    lunes: false,
                    lunes_de: marqueur::planete_active(&self.sys, self.fiche_ouverte),
                    zone: self.menu.zone,
                    focus: self.cam.focus(),
                    previsualisation,
                },
```

- [ ] **Step 8 : Dessiner les marqueurs, sous le reste de l'interface**

Dans `frame()` de `skymap_v2.rs`, juste **après** la fermeture de l'appel `self.rendu.rendre(...);` et **avant** `let temps = if self.pause {` :

```rust
        // Marqueurs : passe interface, AVANT bandeau/colonne/fiche, qui les
        // recouvrent naturellement là où ils se superposent. Recalculés ici
        // (et non repris du survol) parce que le clic a pu changer la
        // sélection, donc les lunes à montrer, dans cette même frame.
        let marqueurs = self.marqueurs(&cam_info, aspect);
        marqueur::dessiner(&self.sys, &marqueurs, survole, self.fiche_ouverte);
```

- [ ] **Step 9 : Déclarer et exporter l'écran**

Dans `src/ecran/mod.rs` :
- après `mod skymap;` (l. 26) : `mod skymap_v2;`
- après `pub use skymap::Skymap;` (l. 38) : `pub use skymap_v2::SkymapV2;`

- [ ] **Step 10 : Bouton d'accueil**

Dans `src/ecran/accueil.rs`, enum `Cible`, après `Skymap,` (l. 8) :

```rust
    /// Skymap V2 : marqueurs de planètes et de lunes (fork de `Skymap`).
    SkymapV2,
```

Dans `BLOC_ASTRES`, après la ligne `("SKYMAP - SYSTEME COMPLET", Cible::Skymap),` :

```rust
    ("SKYMAP V2 - MARQUEURS", Cible::SkymapV2),
```

- [ ] **Step 11 : Aiguillage dans `main.rs`**

Dans `src/main.rs` :
1. Import (l. 27-31) : ajouter `SkymapV2` à la liste, après `Skymap,` :
   ```rust
   use ecran::{
       Parametres,
       Accueil, Categorie, Cible, Galerie, GalerieDisques, GalerieEtoiles, Objet, Skymap, SkymapV2,
       SortieStarmap, Starmap, VueAssembleur, VueStation,
   };
   ```
2. Enum `Etat`, après `Skymap(Box<Skymap>),` : `SkymapV2(Box<SkymapV2>),`
3. `match cible`, après `Cible::Skymap => Etat::Skymap(Box::new(Skymap::new())),` :
   ```rust
                        Cible::SkymapV2 => Etat::SkymapV2(Box::new(SkymapV2::new())),
   ```
4. `match &mut etat`, après le bras `Etat::Skymap(s) => { ... }` :
   ```rust
            Etat::SkymapV2(s) => {
                if s.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
   ```

- [ ] **Step 12 : Retirer l'allow périmé de `projeter_ecran`**

Dans `src/camera.rs`, supprimer la ligne 93 :

```rust
#[allow(dead_code)] // Interaction de l'assembleur (Lot 4) : deja teste, pas encore branche a une vue.
```

Elle est fausse depuis longtemps (`ecran/assembleur.rs` et `ecran/designation.rs` l'appellent) et la V2 en est un appelant de plus.

- [ ] **Step 13 : Compilation, tests, clippy**

Run: `cargo test`
Expected: tout vert, même total qu'à la fin de la Tâche 2 (aucun test neuf ici : dessin et câblage d'écran ne se testent pas, convention du projet).

Run: `cargo clippy --all-targets`
Expected: **0 avertissement**. Les `dead_code` transitoires des Tâches 1-2 doivent avoir disparu. S'il en reste un sur `SkymapV2` ou `marqueur`, c'est un oubli de branchement (Steps 4-11), pas un `allow` à poser.

- [ ] **Step 14 : Vérification à l'écran (obligatoire — rien de ce qui précède ne teste le rendu)**

Run: `cargo run --release`, accueil → **SKYMAP V2 - MARQUEURS**. Vérifier, et rapporter à l'utilisateur ce qui a été **vu**, pas supposé :
1. Au cadrage par défaut, un triangle sur chaque planète du système solaire, de la teinte de sa pastille dans la colonne.
2. Survol d'un triangle : contour blanc. Clic : la caméra se centre sur la planète et la fiche s'ouvre.
3. Terre sélectionnée : l'orbite de la Lune apparaît (violet pâle) avec un triangle sur la Lune ; **pas** de triangle sur l'ISS. Jupiter sélectionnée : ses lunes et leurs orbites.
4. Clic sur une lune de Jupiter : les triangles des autres lunes de Jupiter restent.
5. Clic dans le vide : orbites et triangles des lunes disparaissent (la fiche se referme).
6. Faire pivoter la caméra pour qu'une planète sorte du champ : son triangle disparaît.
7. Le bouton ORBITES LUNES du menu ne fait **plus** rien en V2 (voulu, spec §6.4).
8. Accueil : le bouton QUITTER reste entièrement visible sous PARAMETRES (le bloc de gauche a gagné une ligne — à 720 px de haut, il reste ~10 px de marge ; si la fenêtre est plus petite, le signaler).
9. SKYMAP (V1) : inchangée — ORBITES LUNES y fonctionne toujours.

⚠️ Si une vérification n'a pas pu être faite (pas d'accès à la fenêtre), le **dire explicitement** plutôt que de la déclarer réussie.

- [ ] **Step 15 : Point de commit — signaler, ne pas commiter**

« Point de commit possible : Skymap V2 fonctionnelle (fichiers : `src/ecran/marqueur.rs`, `src/ecran/skymap_v2.rs`, `src/ecran/mod.rs`, `src/ecran/accueil.rs`, `src/main.rs`, `src/camera.rs`). »

---

### Task 4 : Journal

**Files:**
- Modify: `STATE.md` (section « Reprendre ici » et compteur de tests)
- Modify: `docs/README.md` (index `suivi/` et « Repères chronologiques »)
- Modify: `docs/suivi/interface.md` (nouvelle entrée en fin de fichier)

Consigne debout du projet : mettre à jour les `.md` à chaque étape, sans faire dériver `STATE.md` (il dit l'état et la suite, le récit va dans le journal).

- [ ] **Step 1 : Entrée de journal dans `docs/suivi/interface.md`**

Ajouter en fin de fichier une section `## Skymap V2 — marqueurs de planètes et lunes (2026-09-22)` qui dit, en prose courte : le problème (corps sous le pixel en vue globale), le choix du fork (V1 conservée, une seule ligne mécanique touchée : `lunes_de: None`), le cœur générique de `ecran/marqueur.rs` et ce qui n'est **pas** branché (étoiles, engins, stations, ceintures — spec §9), les trois corrections faites au spec en préparant le plan (`fiche_ouverte` et non `fiche_affiche` ; orbites via `Calques` parce que le tracé vit dans la passe 3D ; filtre `Categorie::Lune` à cause de l'ISS), les tests ajoutés et leurs red-checks, et **le résultat de la vérification à l'écran** de la Tâche 3 Step 14 (ce qui a été vu, ce qui n'a pas pu l'être).

- [ ] **Step 2 : `STATE.md`**

Dans « Reprendre ici » : remplacer le compteur de tests (719) par le total réel obtenu en Tâche 3 Step 13, et ajouter une ligne en tête de « À faire ensuite » : « **Skymap V2** (marqueurs) : faite, à juger à l'œil contre V1 — garder V2, revenir à V1, ou porter les marqueurs dans V1. Journal : `suivi/interface.md`. Spec : `docs/superpowers/specs/2026-09-22-skymap-v2-marqueurs-design.md`. »

- [ ] **Step 3 : `docs/README.md`**

Dans la table `suivi/`, compléter la ligne `interface.md` par « **+ Skymap V2** (2026-09-22) : marqueurs de planètes et lunes ». Dans « Repères chronologiques », ajouter une ligne `| 2026-09-22 | Skymap V2 : marqueurs de planètes et lunes, fork de la vue système | [suivi/interface.md](suivi/interface.md) |`.

- [ ] **Step 4 : Point de commit final — signaler, ne pas commiter**

« Point de commit possible : journal à jour (`STATE.md`, `docs/README.md`, `docs/suivi/interface.md`) ; le spec et ce plan (`docs/superpowers/`) sont aussi non commités. »
