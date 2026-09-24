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

/// Un marqueur : l'astre qu'il désigne, et **la pointe de son triangle** à
/// l'écran (px) — le centre projeté de l'astre, remonté de son rayon
/// apparent plus [`ECART_PX`] (voir `decalage_px`).
///
/// C'est bien la pointe, et pas le centre de l'astre, qui est stockée : le
/// survol et le clic visent ce qu'on VOIT. Cliquer le disque lui-même reste
/// possible par le pick 3D, qui n'a pas besoin de marqueur.
///
/// Rien de propre à un type d'astre : un marqueur de planète et un marqueur
/// de lune sont exactement la même donnée.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marqueur {
    pub idx: usize,
    pub ecran: Vec2,
}

/// **Écart entre le bord apparent de l'astre et la pointe du triangle**, en
/// pixels. Assez pour que la pointe ne morde pas le disque, assez peu pour
/// qu'on lise encore « ce triangle désigne CE corps ».
pub const ECART_PX: f32 = 6.0;

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
            let centre = sys.position(idx);
            let p = crate::camera::projeter_ecran(centre, ecran, cam, aspect)?;
            // Le cadrage se juge sur le CENTRE de l'astre, pas sur la pointe
            // décalée : c'est l'astre qui est à l'écran ou non.
            if !dans_cadre(p, ecran) {
                return None;
            }
            let decalage = decalage_px(sys, idx, centre, p, cam, ecran, aspect);
            Some(Marqueur { idx, ecran: vec2(p.x, p.y - decalage) })
        })
        .collect()
}

/// **De combien la pointe du triangle monte au-dessus du centre de l'astre.**
///
/// Un décalage fixe en pixels se retrouverait DANS le disque dès qu'on
/// approche : une planète qui remplit l'écran garderait sa pointe en plein
/// milieu. On mesure donc le rayon apparent (un point du monde écarté du
/// rayon de l'astre le long de la verticale caméra, projeté à son tour) et
/// on ajoute [`ECART_PX`]. De loin, le rayon apparent est nul et il ne
/// reste que l'écart.
fn decalage_px(
    sys: &Systeme,
    idx: usize,
    centre: Vec3,
    centre_ecran: Vec2,
    cam: &CameraInfo,
    ecran: Vec2,
    aspect: f32,
) -> f32 {
    let rayon = sys.rayon_de(idx).unwrap_or(0.0);
    let rayon_px = crate::camera::projeter_ecran(centre + cam.up * rayon, ecran, cam, aspect)
        .map_or(0.0, |haut| (centre_ecran.y - haut.y).abs());
    rayon_px + ECART_PX
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

/// Triangle plein, base 8 px, hauteur 10 px, pointe en bas sur
/// `Marqueur::ecran` — donc au-dessus de l'astre, dégagée de son disque.
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
        // `unwrap_or` ne se déclenche jamais en pratique : chaque `m.idx`
        // vient de `marqueurs_pour`, qui filtre déjà `idx < sys.nb_astres()`
        // avant de construire le marqueur.
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
            origine: Vec3::ZERO,
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
        // Même abscisse que l'astre, mais AU-DESSUS de lui (y écran croît vers
        // le bas) : la pointe ne se pose pas sur le centre.
        assert!((ms[0].ecran.x - ECRAN.x * 0.5).abs() < 0.5, "décalé horizontalement : {:?}", ms[0].ecran);
        assert!(ms[0].ecran.y < ECRAN.y * 0.5 - ECART_PX, "la pointe n'est pas remontée : {:?}", ms[0].ecran);
    }

    /// **La pointe monte avec le disque de l'astre.** Un décalage fixe en
    /// pixels se retrouverait à l'intérieur d'une planète qu'on approche : ce
    /// qui compte est le rayon APPARENT, plus l'écart.
    ///
    /// Deux corps de rayons différents à la même distance, donc un seul
    /// paramètre change entre les deux mesures.
    #[test]
    fn la_pointe_se_pose_au_dessus_du_disque_apparent() {
        let cam = camera();
        let mesurer = |rayon: f32| {
            let mut sys = Systeme::new();
            sys.ajouter(Box::new(Corps {
                base: CorpsBase::new(Vec3::ZERO, 1.0, rayon),
                cat: Categorie::Planete,
                parent: None,
            }));
            // Hauteur de la pointe au-dessus du centre projeté de l'astre.
            ECRAN.y * 0.5 - marqueurs_pour(&sys, [0], &cam, ECRAN, ASPECT)[0].ecran.y
        };

        // Un corps ponctuel : il ne reste que l'écart.
        assert!((mesurer(0.0) - ECART_PX).abs() < 0.5, "corps ponctuel : {}", mesurer(0.0));

        // Rayon apparent d'un corps d'une unité à 20 unités de distance :
        // 0.5 · hauteur_écran / (tan(22,5°) · dist) = 350 / (0,4142 · 20) ≈ 42,2 px.
        let attendu = 0.5 * ECRAN.y / (22.5_f32.to_radians().tan() * 20.0) + ECART_PX;
        assert!((mesurer(1.0) - attendu).abs() < 1.0, "rayon 1 : {} au lieu de {attendu}", mesurer(1.0));

        // Et un corps deux fois plus gros monte deux fois plus haut, l'écart mis à part.
        assert!(
            ((mesurer(2.0) - ECART_PX) - 2.0 * (mesurer(1.0) - ECART_PX)).abs() < 1.0,
            "le décalage ne suit pas le rayon : {} puis {}",
            mesurer(1.0),
            mesurer(2.0)
        );
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
