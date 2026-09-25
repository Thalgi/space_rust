//! Socle de rendu commun aux corps « impostor » (1 quad face-caméra + 1 shader
//! qui reconstruit une sphère). Mutualise la géométrie du quad, le vertex shader
//! (`shaders/impostor.vert.glsl`) et la pose des uniforms partagés par tous les
//! corps. Chaque type ajoute ensuite ses propres uniforms + son fragment shader.

use crate::astre::CameraInfo;
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::cell::RefCell;

/// Vertex shader partagé : billboard dont le disque est mis à l'échelle par `disc`.
pub const VERT_IMPOSTOR: &str = include_str!("shaders/impostor.vert.glsl");

/// Source d'un shader, avec hot-reload : tente de lire `src/shaders/<nom>` sur le
/// disque (chemin figé à la compilation via CARGO_MANIFEST_DIR, donc indépendant du
/// répertoire courant) ; à défaut, retombe sur la version embarquée `defaut`.
/// En dev on édite le .glsl et on recharge (touche R) sans recompiler ; un binaire
/// distribué sans les sources utilise simplement la version embarquée.
pub fn source(nom: &str, defaut: &'static str) -> String {
    let chemin = format!("{}/src/shaders/{}", env!("CARGO_MANIFEST_DIR"), nom);
    std::fs::read_to_string(&chemin).unwrap_or_else(|_| defaut.to_string())
}

/// Empile un quad face-caméra centré sur le point **monde** `centre`, de
/// demi-taille `taille`.
///
/// ⚠️ **C'est la fonction qui convertit, pas l'appelant.** Les sommets partent
/// au GPU : ils doivent être exprimés dans le repère de rendu
/// (`CameraInfo::rel`). Prendre `cam` entier plutôt que `right`/`up` séparés
/// rend l'oubli impossible — un quad laissé en coordonnées héliocentriques
/// absolues partirait à 460 unités hors champ, et rien côté Rust ne le dirait.
/// Voir `docs/conception/repere_flottant.md`.
pub fn push_quad(
    verts: &mut Vec<Vertex>,
    inds: &mut Vec<u16>,
    cam: &CameraInfo,
    centre: Vec3,
    taille: f32,
    couleur: Color,
) {
    let centre = cam.rel(centre);
    let i0 = verts.len() as u16;
    let r = cam.right * taille;
    let u = cam.up * taille;
    verts.push(Vertex::new2(centre - r - u, vec2(0.0, 0.0), couleur));
    verts.push(Vertex::new2(centre + r - u, vec2(1.0, 0.0), couleur));
    verts.push(Vertex::new2(centre + r + u, vec2(1.0, 1.0), couleur));
    verts.push(Vertex::new2(centre - r + u, vec2(0.0, 1.0), couleur));
    inds.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
}

/// Un point d'un ruban : sa position **monde**, sa demi-largeur et sa couleur.
pub type PointRuban = (Vec3, f32, Color);

/// Empile un **ruban** — une bande continue le long d'une polyligne, dont la
/// largeur et la couleur varient d'un point à l'autre.
///
/// C'est la fonction sœur de [`push_quad`] pour tout ce qui est *filaire* :
/// un chapelet de quads face-caméra donne de la continuité **radiale** (le
/// halo de chaque point) mais aucune continuité **longitudinale** — on
/// devine les pastilles une à une le long d'une arche de plasma. Un ruban
/// interpole largeur et couleur aux jointures, donc ne montre pas de couture.
///
/// La largeur est portée par la normale à la courbe **projetée dans le plan
/// écran**, ce que `push_quad` ne sait pas faire : lui pose un quad centré,
/// carré et d'une seule taille.
///
/// Les UV traversent la texture par son **milieu** (`u = 0,5`), du bord
/// inférieur au bord supérieur : sur la texture de halo radiale déjà
/// utilisée, ça donne exactement le profil transversal d'un tube — vif au
/// centre, éteint sur les bords — sans texture ni matériau de plus.
///
/// Moins de deux points : rien à tracer.
pub fn push_ruban(
    verts: &mut Vec<Vertex>,
    inds: &mut Vec<u16>,
    cam: &CameraInfo,
    points: &[PointRuban],
) {
    if points.len() < 2 {
        return;
    }
    let i0 = verts.len() as u16;
    for (i, (pos, demi, couleur)) in points.iter().enumerate() {
        // Tangente par différence centrée : c'est la direction du ruban ici.
        let avant = points[i.saturating_sub(1)].0;
        let apres = points[(i + 1).min(points.len() - 1)].0;
        // Projetée dans le plan de l'écran, puis tournée d'un quart de tour :
        // la largeur s'ouvre perpendiculairement à la course apparente. Le
        // repli couvre le cas où la courbe fuit droit vers la caméra, où la
        // tangente projetée est nulle.
        let t = apres - avant;
        let plan = vec2(t.dot(cam.right), t.dot(cam.up)).normalize_or(Vec2::X);
        let n = (cam.right * -plan.y + cam.up * plan.x) * *demi;
        let centre = cam.rel(*pos);
        verts.push(Vertex::new2(centre - n, vec2(0.5, 0.0), *couleur));
        verts.push(Vertex::new2(centre + n, vec2(0.5, 1.0), *couleur));
    }
    for k in 0..points.len() as u16 - 1 {
        let (a, b) = (i0 + k * 2, i0 + k * 2 + 2);
        inds.extend_from_slice(&[a, a + 1, b + 1, a, b + 1, b]);
    }
}

#[cfg(test)]
mod tests_ruban {
    use super::*;

    fn cam() -> CameraInfo {
        CameraInfo {
            pos: vec3(0.0, 0.0, 10.0),
            origine: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            forward: -Vec3::Z,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        }
    }

    fn ruban(points: &[PointRuban]) -> (Vec<Vertex>, Vec<u16>) {
        let (mut v, mut i) = (Vec::new(), Vec::new());
        push_ruban(&mut v, &mut i, &cam(), points);
        (v, i)
    }

    /// **Deux sommets par point, et deux triangles par segment** — la bande
    /// est continue, sans couture ni sommet dupliqué entre segments. C'est ce
    /// qui la rend aussi **moins chère** que le chapelet de quads qu'elle
    /// remplace : quatre sommets par point là-bas, deux ici.
    #[test]
    fn la_bande_est_continue_et_moins_chere_qu_un_chapelet() {
        let p: Vec<PointRuban> =
            (0..5).map(|k| (vec3(k as f32, 0.0, 0.0), 0.2, WHITE)).collect();
        let (v, i) = ruban(&p);
        assert_eq!(v.len(), 10, "il faut deux sommets par point");
        assert_eq!(i.len(), 4 * 6, "quatre segments, deux triangles chacun");
        // Moins de sommets qu'un quad par point.
        assert!(v.len() < p.len() * 4);
        // Tous les indices pointent dans le tableau de sommets.
        assert!(i.iter().all(|k| (*k as usize) < v.len()), "index hors sommets");
    }

    /// **La largeur s'ouvre perpendiculairement à la course apparente.** Un
    /// ruban dont la largeur suivrait la tangente serait invisible de face ;
    /// c'est toute la différence avec un quad centré, qui n'a pas d'orientation
    /// à respecter.
    #[test]
    fn la_largeur_s_ouvre_en_travers_du_trajet() {
        // Trajet horizontal à l'écran : la bande doit s'ouvrir verticalement.
        let p = [
            (vec3(-1.0, 0.0, 0.0), 0.5, WHITE),
            (vec3(0.0, 0.0, 0.0), 0.5, WHITE),
            (vec3(1.0, 0.0, 0.0), 0.5, WHITE),
        ];
        let (v, _) = ruban(&p);
        let (bas, haut) = (v[2].position, v[3].position);
        assert!((bas.x).abs() < 1e-5 && (haut.x).abs() < 1e-5, "la bande s'ouvre dans le sens du trajet");
        assert!((haut.y - bas.y).abs() > 0.9, "la bande n'a pas d'épaisseur");
    }

    /// **Les UV traversent la texture par son milieu**, ce qui donne le profil
    /// transversal d'un tube sur la texture de halo radiale déjà utilisée —
    /// vif au centre, éteint aux bords, sans texture ni matériau de plus.
    #[test]
    fn les_uv_prennent_la_coupe_du_halo() {
        let p = [(Vec3::ZERO, 0.3, WHITE), (Vec3::X, 0.3, WHITE)];
        let (v, _) = ruban(&p);
        for s in &v {
            assert_eq!(s.uv.x, 0.5, "l'UV quitte l'axe médian de la texture");
        }
        assert_eq!((v[0].uv.y, v[1].uv.y), (0.0, 1.0), "la coupe ne traverse pas");
    }

    /// **Moins de deux points ne trace rien — pas même un sommet.**
    ///
    /// Un ruban est coupé chaque fois qu'il passe derrière l'étoile : les
    /// brins d'un seul point sont donc courants, pas théoriques. ⚠️ Vérifier
    /// les seuls indices ne suffit pas — sans sommets, il n'y a pas de
    /// triangle, mais deux **sommets orphelins** par brin s'accumuleraient
    /// dans le mesh à chaque frame.
    #[test]
    fn un_brin_trop_court_ne_trace_rien() {
        for brin in [vec![], vec![(Vec3::ZERO, 0.3, WHITE)]] {
            let (v, i) = ruban(&brin);
            assert!(i.is_empty(), "un brin de {} point a produit des triangles", brin.len());
            assert!(v.is_empty(), "un brin de {} point a laissé des sommets", brin.len());
        }
    }

    /// **Les sommets partent en coordonnées de rendu**, comme ceux de
    /// [`push_quad`] : un ruban laissé en absolu partirait hors champ à
    /// l'échelle héliocentrique (`repere_flottant.md`).
    #[test]
    fn les_sommets_passent_par_le_repere_de_rendu() {
        let mut c = cam();
        c.origine = vec3(100.0, 0.0, 0.0);
        let (mut v, mut i) = (Vec::new(), Vec::new());
        push_ruban(&mut v, &mut i, &c, &[(vec3(100.0, 0.0, 0.0), 0.1, WHITE), (vec3(101.0, 0.0, 0.0), 0.1, WHITE)]);
        assert!(v[0].position.x.abs() < 1e-4, "sommet resté en absolu : {:?}", v[0].position);
    }
}

/// Uniforms communs à tous les corps impostor : temps, repère caméra, et l'échelle
/// `disc` du disque. `disc` doit valoir la même demi-taille que celle du quad.
pub fn uniforms_standard(mat: &Material, cam: &CameraInfo, temps: f32, disc: f32) {
    mat.set_uniform("time", temps);
    mat.set_uniform("cam_right", (cam.right.x, cam.right.y, cam.right.z));
    mat.set_uniform("cam_up", (cam.up.x, cam.up.y, cam.up.z));
    let to_cam = -cam.forward;
    mat.set_uniform("to_cam", (to_cam.x, to_cam.y, to_cam.z));
    mat.set_uniform("disc", disc);
}

// ---------------------------------------------------------------------------
// Échelle apparente
// ---------------------------------------------------------------------------

// Hauteur (px) du viewport de rendu courant. **Une seule cellule**, alors que
// `planete` et `disque` en tenaient chacun la leur : il fallait les régler en
// cascade, et un appelant qui en oubliait une donnait à ce module-là le LOD du
// plein écran dans une cellule de galerie (c'est le cas de `galerie.rs`, qui ne
// réglait que `planete`). L'étoile en a besoin à son tour, et hors du chemin de
// dessin — `Systeme::pick` doit connaître la taille RÉELLEMENT dessinée.
// 0 = plein écran.
thread_local! {
    static VIEWPORT_H: RefCell<f32> = const { RefCell::new(0.0) };
}

/// Hauteur du viewport de rendu courant (galerie : hauteur de cellule,
/// éventuellement divisée par le facteur pixel). 0 = plein écran.
pub fn set_viewport_h(h: f32) {
    VIEWPORT_H.with(|v| *v.borrow_mut() = h);
}

/// Hauteur effective : la valeur posée, ou l'écran entier à défaut.
pub fn viewport_h() -> f32 {
    let h = VIEWPORT_H.with(|v| *v.borrow());
    if h > 0.0 {
        h
    } else {
        screen_height()
    }
}

/// Rayon apparent d'un corps, en pixels — approximation petit angle pour un fov
/// de ~45° : `px ≈ r/dist · h / (2·tan(22,5°))`. Pilote tous les LOD.
pub fn px_rayon(r: f32, dist: f32, vh: f32) -> f32 {
    r / dist.max(1e-3) * vh * 1.2
}

/// Rayon **à dessiner** pour qu'un corps couvre au moins `px_min` pixels.
///
/// N'agrandit jamais que ce qui est trop petit, et **rend la main** dès que la
/// taille vraie dépasse le plancher : à la traversée, le facteur vaut exactement
/// 1, donc aucun saut — seule la vitesse de croissance a un coude.
///
/// Le piège du LOD des débris (`disque_particules.vert.glsl`) ne s'applique pas
/// ici : là-bas la taille se calculait dans le vertex shader à partir de la
/// matrice `Projection`, qui chez macroquad est la vue-projection COMBINÉE — un
/// coefficient qui s'annule en visant l'axe Y, d'où des débris infinis. Ici il
/// n'y a pas de matrice, seulement une distance.
pub fn rayon_plancher(r: f32, dist: f32, vh: f32, px_min: f32) -> f32 {
    let px = px_rayon(r, dist, vh);
    if px <= 0.0 {
        return r; // corps de rayon nul : rien à planchéier
    }
    r * (px_min / px).max(1.0)
}

/// Plafond de pipelines de macroquad (`quad_gl.rs`, `MAX_PIPELINES`). Chaque
/// `load_material` en consomme un ; le dépassement **panique** au lancement,
/// avec « Pipelines amount exceeded » et rien d'autre pour situer la fuite.
#[cfg(test)]
const MAX_PIPELINES: usize = 32;

/// Descripteurs des uniforms communs (à concaténer à ceux propres au corps).
pub fn uniforms_communs() -> Vec<UniformDesc> {
    vec![
        UniformDesc::new("time", UniformType::Float1),
        UniformDesc::new("cam_right", UniformType::Float3),
        UniformDesc::new("cam_up", UniformType::Float3),
        UniformDesc::new("to_cam", UniformType::Float3),
        UniformDesc::new("disc", UniformType::Float1),
    ]
}

/// Outillage de test partagé par les modules qui chargent un shader.
#[cfg(test)]
pub mod verif {
    use macroquad::prelude::UniformDesc;
    use std::collections::BTreeSet;

    /// Uniforms que macroquad fournit d'office : présents dans le `.glsl`, ils
    /// n'ont pas à être déclarés côté Rust.
    const IMPLICITES: &[&str] = &["Model", "Projection", "_Time"];

    /// Noms des uniforms **lus par le shader**, extraits des sources GLSL (vertex
    /// et fragment). Les textures (`sampler2D`) sont déclarées à part, dans
    /// `MaterialParams.textures`.
    pub fn uniforms_glsl(sources: &[&str]) -> BTreeSet<String> {
        let mut noms = BTreeSet::new();
        for src in sources {
            for ligne in src.lines() {
                let Some(reste) = ligne.trim().strip_prefix("uniform ") else {
                    continue;
                };
                let mut mots = reste.split(';').next().unwrap_or("").split_whitespace();
                let (Some(type_), Some(nom)) = (mots.next(), mots.next()) else {
                    continue;
                };
                if type_ == "sampler2D" {
                    continue;
                }
                let nom = nom.split('[').next().unwrap_or(nom);
                if !IMPLICITES.contains(&nom) {
                    noms.insert(nom.to_string());
                }
            }
        }
        noms
    }

    /// Uniforms **déclarés dans le GLSL mais jamais lus** par le code du shader.
    ///
    /// Un uniform déclaré et ignoré est un réglage qui ne fait rien : la valeur
    /// part bien du Rust, arrive bien au GPU, et n'a aucun effet. C'est le
    /// travers le plus tenace de ce projet — la donnée est juste, la plomberie
    /// ne suit pas — et il ne se voit à la lecture ni d'un côté ni de l'autre.
    ///
    /// Une occurrence hors de sa ligne de déclaration et hors commentaires suffit
    /// à le dire utilisé.
    pub fn uniforms_inutilises(sources: &[&str]) -> BTreeSet<String> {
        let mut morts = BTreeSet::new();
        for src in sources {
            for nom in uniforms_glsl(&[src]) {
                let lu = src.lines().any(|l| {
                    let t = l.trim();
                    !t.starts_with("//") && !t.starts_with("uniform ") && contient_identifiant(l, &nom)
                });
                if !lu {
                    morts.insert(nom);
                }
            }
        }
        morts
    }

    /// `nom` apparaît-il dans `ligne` comme un identifiant entier ?
    ///
    /// Une recherche de sous-chaîne dirait que `atmo` est lu par `atmo_densite`,
    /// et masquerait donc l'uniform mort derrière son homonyme plus long.
    fn contient_identifiant(ligne: &str, nom: &str) -> bool {
        let mot = |c: char| c.is_alphanumeric() || c == '_';
        let bytes = ligne.as_bytes();
        ligne.match_indices(nom).any(|(i, _)| {
            let avant = i == 0 || !mot(bytes[i - 1] as char);
            let j = i + nom.len();
            let apres = j >= bytes.len() || !mot(bytes[j] as char);
            avant && apres
        })
    }

    /// Noms des uniforms **déclarés côté Rust**.
    pub fn uniforms_rust(descs: &[UniformDesc]) -> BTreeSet<String> {
        descs.iter().map(|d| d.name.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::MAX_PIPELINES;

    /// Nom de la fonction de chargement, épelé pour que ce fichier-ci ne se
    /// compte pas lui-même : le test parle de `load_material`, en doc comme dans
    /// le motif, et une recherche naïve trouvait ses propres mentions.
    fn motif() -> String {
        format!("{}_{}(", "load", "material")
    }

    /// Toutes les sources Rust du projet : (chemin relatif à `src/`, contenu).
    fn fichiers_rust() -> Vec<(String, String)> {
        fn parcourir(d: &std::path::Path, racine: &std::path::Path, out: &mut Vec<(String, String)>) {
            let Ok(entrees) = std::fs::read_dir(d) else { return };
            for e in entrees.flatten() {
                let p = e.path();
                if p.is_dir() {
                    parcourir(&p, racine, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let rel = p.strip_prefix(racine).unwrap_or(&p);
                    out.push((
                        rel.to_string_lossy().replace('\\', "/"),
                        std::fs::read_to_string(&p).unwrap_or_default(),
                    ));
                }
            }
        }
        let racine = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        let mut v = Vec::new();
        parcourir(racine, racine, &mut v);
        v.sort();
        assert!(!v.is_empty(), "aucune source lue : le parcours de src/ est cassé");
        v
    }

    /// Les appels réels au chargeur de shader, par fichier. Les lignes de
    /// commentaire sont écartées : elles parlent des appels, elles n'en sont pas.
    fn sites_de_chargement() -> Vec<(String, usize)> {
        let motif = motif();
        fichiers_rust()
            .into_iter()
            .filter_map(|(nom, src)| {
                let n: usize = src
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .map(|l| l.matches(&motif).count())
                    .sum();
                (n > 0).then_some((nom, n))
            })
            .collect()
    }

    /// **Sonnette sur l'arrivée d'un shader.**
    ///
    /// macroquad plafonne les pipelines à 32 et panique au-delà. Ce test compte
    /// les **sites d'appel**, ce qui n'est pas tout à fait le nombre de
    /// pipelines : un même site peut en produire plusieurs. `charger_soleil` en
    /// fait deux — disque opaque et couronne translucide, identiques sauf
    /// l'écriture de profondeur.
    ///
    /// ⚠️ Ce test ne dit **rien du cache**. Il est resté vert quand on a
    /// délibérément remis un material par disque : c'est
    /// `les_materials_ne_sont_construits_qu_une_fois` qui garde ça. Ici on veut
    /// seulement être averti qu'un shader est arrivé — moment où il faut
    /// vérifier qu'il est bien créé une fois pour toutes.
    #[test]
    fn le_nombre_de_shaders_reste_sous_le_plafond() {
        let sites = sites_de_chargement();
        let total: usize = sites.iter().map(|(_, n)| n).sum();
        assert_eq!(total, 10, "un shader est arrivé ou parti. Détail : {sites:?}");

        // Décompte réel des pipelines vivants, à tenir à jour avec le précédent :
        //   9 templates en cache — planète (corps, atmosphère), soleil (disque,
        //     couronne, plasma), disque (particules, voile), éclairage, palette ;
        //   6 par instance — un fond étoilé par écran (5) + le panache (1).
        const PIPELINES: usize = 9 + 6;
        const _: () = assert!(
            PIPELINES * 2 <= MAX_PIPELINES,
            "plus de marge sous le plafond de pipelines de macroquad"
        );
    }

    /// **Un shader se charge une fois, pas une fois par objet.**
    ///
    /// C'est LE test du plantage : `Disque::new` appelait `material_particules`
    /// et `material_voile` directement, donc un pipeline GPU par disque. La
    /// galerie de débris en construit douze d'un coup — plafond dépassé,
    /// « Pipelines amount exceeded » au lancement, sans rien pour situer la fuite.
    ///
    /// Le remède est un template mis en cache et cloné (`mat_particules`,
    /// `mat_voile`). Ce test tient l'invariant qui le garantit : **chaque
    /// constructeur brut n'est appelé qu'à un seul endroit**, l'initialisation
    /// du cache. Deux appels, et le material redevient créé par objet.
    #[test]
    fn les_materials_ne_sont_construits_qu_une_fois() {
        // Le comptage porte sur TOUTE la crate : un constructeur est défini dans
        // un fichier et appelé depuis un autre (`voile::material_voile` vit dans
        // `disque/voile.rs`, le cache qui l'appelle est dans `disque/mod.rs`).
        for constructeur in ["material_particules", "material_voile"] {
            let appels: usize = fichiers_rust()
                .iter()
                .map(|(_, src)| {
                    src.lines()
                        .filter(|l| {
                            let t = l.trim_start();
                            !t.starts_with("//") && !t.contains(&format!("fn {constructeur}"))
                        })
                        .map(|l| l.matches(&format!("{constructeur}(")).count())
                        .sum::<usize>()
                })
                .sum();
            assert_eq!(
                appels, 1,
                "`{constructeur}` est appelé {appels} fois dans la crate. Il doit l'être \
                 UNE seule, depuis l'initialisation du cache — sinon c'est un pipeline \
                 GPU par objet, et le plafond de {MAX_PIPELINES} saute."
            );
        }
    }
}

#[cfg(test)]
mod tests_plancher {
    use super::{px_rayon, rayon_plancher};

    /// **Un plancher n'agrandit jamais que ce qui est trop petit.** S'il pouvait
    /// rétrécir, une étoile déjà grosse rapetisserait en approchant — l'inverse
    /// exact de ce qu'on veut, et invisible sur les cas lointains.
    #[test]
    fn le_plancher_ne_retrecit_jamais() {
        for dist in [1.0_f32, 10.0, 100.0, 1_000.0, 100_000.0] {
            let r = rayon_plancher(1.116, dist, 1080.0, 8.0);
            assert!(r >= 1.116, "dist {dist} : {r} < rayon vrai");
        }
        // Corps de rayon nul : pas de division par zéro, pas d'agrandissement.
        assert_eq!(rayon_plancher(0.0, 100.0, 1080.0, 8.0), 0.0);
    }

    /// **Ce que le plancher promet, il le tient** : au-delà de la distance de
    /// bascule, le corps couvre exactement le minimum demandé.
    #[test]
    fn au_dela_du_seuil_la_taille_apparente_est_constante() {
        for dist in [500.0_f32, 1_280.0, 20_000.0] {
            let r = rayon_plancher(1.116, dist, 1080.0, 8.0);
            let px = px_rayon(r, dist, 1080.0);
            assert!((px - 8.0).abs() < 1e-3, "dist {dist} : {px} px au lieu de 8");
        }
    }

    /// **Et il rend la main en approchant.** C'est ce qui distingue un plancher
    /// d'une exagération permanente : passé le seuil, on revient à l'échelle
    /// vraie, sans saut — le facteur vaut exactement 1 à la traversée.
    #[test]
    fn le_plancher_s_efface_de_pres_et_sans_saut() {
        // Seuil : la distance où la taille vraie atteint exactement 8 px.
        let seuil = 1.116 / 8.0 * 1080.0 * 1.2;
        assert!((px_rayon(1.116, seuil, 1080.0) - 8.0).abs() < 1e-3);
        // Plus près : rayon vrai, intact.
        let pres = rayon_plancher(1.116, seuil * 0.5, 1080.0, 8.0);
        assert!((pres - 1.116).abs() < 1e-6, "de près l'étoile doit garder sa taille : {pres}");

        // Continuité au seuil. Elle se mesure en RELATIF : de part et d'autre à
        // ±eps, l'écart de taille doit rester de l'ordre d'eps. Une tolérance
        // absolue ne prouverait rien — un vrai saut la franchirait pour un eps
        // assez petit, alors qu'ici le majorant s'annule avec eps.
        for eps in [1e-2_f32, 1e-3, 1e-4] {
            let avant = rayon_plancher(1.116, seuil * (1.0 - eps), 1080.0, 8.0);
            let apres = rayon_plancher(1.116, seuil * (1.0 + eps), 1080.0, 8.0);
            let saut = (apres - avant).abs() / 1.116;
            assert!(
                saut <= 3.0 * eps,
                "saut relatif {saut} pour un pas de {eps} : la taille n'est pas continue au seuil"
            );
        }
    }

    /// Le plancher suit le **viewport**, pas l'écran : dans une cellule de
    /// galerie deux fois plus courte, il faut deux fois plus de monde pour
    /// couvrir les mêmes pixels.
    #[test]
    fn le_plancher_suit_le_viewport() {
        let plein = rayon_plancher(1.116, 1_280.0, 1080.0, 8.0);
        let cellule = rayon_plancher(1.116, 1_280.0, 540.0, 8.0);
        assert!(
            (cellule - plein * 2.0).abs() < 1e-3,
            "viewport moitié -> rayon double attendu : {cellule} vs {plein}"
        );
    }
}

#[cfg(test)]
mod tests_matrice {
    /// **Aucun shader ne doit lire un ÉLÉMENT de la matrice `Projection`.**
    ///
    /// L'uniform que macroquad fournit sous ce nom n'est pas la projection seule :
    /// c'est `perspective × look_at` (`Camera3D::matrix`), donc la matrice
    /// **vue-projection**. Ses coefficients dépendent de l'orientation de la
    /// caméra, et les lire comme ceux d'une projection pure ne vaut que pour un
    /// angle de vue particulier.
    ///
    /// Le cas vécu : `Projection[1][1]` servait de `1/tan(fovy/2)` pour convertir
    /// des unités monde en pixels. En visant le long de Y, la vue envoie le Y du
    /// monde sur la profondeur, ce coefficient tend vers zéro — et le LOD des
    /// débris, qui divise par lui, faisait enfler les petites particules en gros
    /// carrés. Invisible partout ailleurs.
    ///
    /// Pour une taille à l'écran, **projeter deux points et mesurer l'écart**.
    #[test]
    fn aucun_shader_ne_lit_un_coefficient_de_la_matrice_vue_projection() {
        // Aiguille assemblée pour que ce fichier ne se trouve pas lui-même.
        let aiguille = format!("{}[", "Projection");
        let dossier = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders");
        let mut fautifs = Vec::new();
        let entrees = std::fs::read_dir(dossier).expect("dossier des shaders");
        let mut vus = 0;
        for e in entrees.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "glsl") {
                continue;
            }
            vus += 1;
            let src = std::fs::read_to_string(&p).unwrap_or_default();
            for (n, ligne) in src.lines().enumerate() {
                // Les commentaires ont le droit d'en parler — ils l'expliquent.
                if ligne.trim_start().starts_with("//") {
                    continue;
                }
                if ligne.contains(&aiguille) {
                    fautifs.push(format!("{}:{}", p.file_name().unwrap().to_string_lossy(), n + 1));
                }
            }
        }
        assert!(vus > 0, "aucun shader lu : le parcours du dossier est cassé");
        assert!(
            fautifs.is_empty(),
            "ces shaders lisent un coefficient de la matrice vue-projection : {fautifs:?}"
        );
    }
}
