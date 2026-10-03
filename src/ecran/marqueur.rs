//! **Marqueurs d'astres** : un petit repère à taille écran fixe, planté sur
//! la position projetée d'un astre — pour voir et cliquer un corps qui, à
//! l'échelle vraie, fait moins d'un pixel (Skymap V2,
//! `docs/superpowers/specs/2026-09-22-skymap-v2-marqueurs-design.md`).
//!
//! **Le cœur est générique** : `marqueurs_pour`, `plus_proche` et `dessiner`
//! ne manipulent que des indices d'astres, sans rien savoir de leur
//! catégorie. Seules les fonctions de **sélection d'indices** en décident —
//! aujourd'hui astres de racine (planètes, comètes) et lunes. Marquer demain un vaisseau, une
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
pub const ECART_PX: f32 = 16.0;

/// Rayon de détection du survol et du clic, en pixels écran.
///
/// Plus large que le repère (8 × 10 px) : on doit pouvoir le cliquer sans
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

/// **De combien la pointe du repère monte au-dessus du centre de l'astre.**
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

/// Indices des astres marqués **à la racine** : ceux qui n'orbitent pas
/// autour d'un autre corps marqué, et qu'on doit pouvoir trouver sans rien
/// avoir sélectionné d'abord.
///
/// ⚠️ **Deux fois déjà, un filtre par catégorie a oublié la comète** : une
/// fois sur les rails (`systeme/mod.rs`), une fois ici, où elle n'avait
/// aucun marqueur et restait donc invisible. La liste est explicite parce
/// qu'il n'y a rien à interroger — « mérite un repère de premier plan »
/// n'est pas une capacité, c'est un choix. Mais
/// [`chaque_categorie_dit_si_elle_est_marquee_a_la_racine`] les balaie
/// toutes : une catégorie nouvelle y apparaît en défaut, et il faut la
/// trancher pour que les tests repassent.
fn marque_a_la_racine(cat: Categorie) -> bool {
    matches!(cat, Categorie::Planete | Categorie::Comete)
}

/// Le « sans parent » est une ceinture de sécurité : aujourd'hui une
/// `Planete` qui a un parent se déclare `Lune` (`planete/mod.rs`), mais un
/// corps qui recevrait à la fois un marqueur de racine et un de lune serait
/// désigné deux fois.
fn indices_racines(sys: &Systeme) -> impl Iterator<Item = usize> + '_ {
    (0..sys.nb_astres()).filter(|&i| {
        sys.categorie_de(i).is_some_and(marque_a_la_racine) && sys.parent_de(i).is_none()
    })
}

/// Un marqueur par astre de racine. Ne fait que choisir les indices.
pub fn marqueurs_racines(sys: &Systeme, cam: &CameraInfo, ecran: Vec2, aspect: f32) -> Vec<Marqueur> {
    marqueurs_pour(sys, indices_racines(sys), cam, ecran, aspect)
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

/// Boîte du marqueur : base 8 px, hauteur 10 px, **pointe en bas** sur
/// `Marqueur::ecran` — donc au-dessus de l'astre, dégagée de son disque.
/// Les deux formes tiennent dans la même boîte, pour que [`RAYON_HIT_PX`]
/// vaille pour l'une comme pour l'autre.
const DEMI_BASE_PX: f32 = 4.0;
const HAUTEUR_PX: f32 = 10.0;

/// **Le contour du marqueur, pointe en premier**, en pixels écran.
///
/// Une **comète** reçoit un losange, tout le reste un triangle : la forme
/// dit le type d'astre là où la teinte dit déjà la sélection
/// (`teinte_marqueur`), sans rien ajouter à lire. Le losange partage la
/// pointe et la boîte du triangle ; il écarte seulement ses flancs à
/// mi-hauteur et se referme sur un sommet haut.
///
/// Pur et rendu en données : le dessin n'a plus qu'à relier les points.
fn contour(cat: Categorie, pointe: Vec2) -> Vec<Vec2> {
    let (dx, h) = (DEMI_BASE_PX, HAUTEUR_PX);
    match cat {
        Categorie::Comete => vec![
            pointe,
            vec2(pointe.x - dx, pointe.y - h * 0.5),
            vec2(pointe.x, pointe.y - h),
            vec2(pointe.x + dx, pointe.y - h * 0.5),
        ],
        _ => vec![pointe, vec2(pointe.x - dx, pointe.y - h), vec2(pointe.x + dx, pointe.y - h)],
    }
}

/// **Les quatre teintes de flèche**, en niveaux de gris : la couleur propre
/// de l'astre est réservée à sa pastille dans la colonne, la carte se lit en
/// clair/sombre.
///
/// Deux échelons pour la planète (clair → blanc), deux pour la lune (sombre
/// → clair) : la **sélection** monte d'un cran, et la lune reste toujours en
/// dessous de sa planète. Un marqueur sélectionné se repère donc à sa
/// clarté, sans second objet à lire.
fn teinte_marqueur(est_lune: bool, selectionne: bool) -> Color {
    let v = match (est_lune, selectionne) {
        (false, true) => 1.00,  // planète sélectionnée : blanc
        (false, false) => 0.78, // planète : gris clair
        (true, true) => 0.78,   // lune sélectionnée : gris clair
        (true, false) => 0.52,  // lune : gris
    };
    Color::new(v, v, v, 1.0)
}

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
        let mut c = teinte_marqueur(cat == Categorie::Lune, selectionne == Some(m.idx));
        let est_survole = survole == Some(m.idx);
        c.a = if est_survole { 1.0 } else { 0.85 };

        let pts = contour(cat, m.ecran);
        // Éventail depuis la pointe : trois points font un triangle, quatre
        // un losange, sans un cas de dessin par forme.
        for f in pts.windows(2).skip(1) {
            draw_triangle(pts[0], f[0], f[1], c);
        }
        if est_survole {
            for i in 0..pts.len() {
                let b = pts[(i + 1) % pts.len()];
                draw_line(pts[i].x, pts[i].y, b.x, b.y, 1.0, WHITE);
            }
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

    // **Astres de racine seulement**, et **lunes seulement** : ni l'étoile,
    // ni l'engin posté autour de la planète (le cas de l'ISS autour de la
    // Terre), ni une planète qui aurait un parent. La **comète**, elle, est
    // bien de la fête — sans quoi elle n'a aucun repère et reste invisible.
    #[test]
    fn la_selection_ne_garde_que_les_racines_et_les_lunes() {
        let mut sys = Systeme::new();
        let etoile = poser(&mut sys, Categorie::Etoile, Vec3::ZERO, None);
        let terre = poser(&mut sys, Categorie::Planete, vec3(1.0, 0.0, 0.0), None);
        let lune = poser(&mut sys, Categorie::Lune, vec3(1.2, 0.0, 0.0), Some(terre));
        let iss = poser(&mut sys, Categorie::Engin, vec3(1.1, 0.0, 0.0), Some(terre));
        let planete_enfant = poser(&mut sys, Categorie::Planete, vec3(1.4, 0.0, 0.0), Some(terre));
        let mars = poser(&mut sys, Categorie::Planete, vec3(-2.0, 0.0, 0.0), None);
        let halley = poser(&mut sys, Categorie::Comete, vec3(-3.0, 1.0, 0.0), None);

        let idx = |v: Vec<Marqueur>| v.into_iter().map(|m| m.idx).collect::<Vec<_>>();
        let racines = idx(marqueurs_racines(&sys, &camera(), ECRAN, ASPECT));
        assert_eq!(racines, vec![terre, mars, halley], "étoile {etoile}, lune, ISS et planète-enfant exclues");
        assert!(!racines.contains(&planete_enfant));

        let lunes = idx(marqueurs_lunes(&sys, terre, &camera(), ECRAN, ASPECT));
        assert_eq!(lunes, vec![lune], "l'ISS ({iss}) n'est pas une lune");
        assert!(marqueurs_lunes(&sys, mars, &camera(), ECRAN, ASPECT).is_empty());
    }

    /// **Chaque catégorie tranche**, pour qu'une nouvelle venue ne puisse pas
    /// se glisser muette dans la carte comme la comète l'a fait. Balayage
    /// exhaustif : ajouter un variant à `Categorie` fait rougir ce test tant
    /// qu'on n'a pas dit, ici, s'il porte un repère de premier plan.
    #[test]
    fn chaque_categorie_dit_si_elle_est_marquee_a_la_racine() {
        for cat in [
            Categorie::Etoile,
            Categorie::Planete,
            Categorie::Lune,
            Categorie::Asteroide,
            Categorie::Comete,
            Categorie::Engin,
        ] {
            let attendu = match cat {
                // Les deux seuls corps qu'on cherche sans rien avoir
                // sélectionné d'abord. La lune passe par sa planète,
                // l'étoile se voit toute seule, l'astéroïde et l'engin
                // relèvent d'autres itérations.
                Categorie::Planete | Categorie::Comete => true,
                Categorie::Etoile | Categorie::Lune | Categorie::Asteroide | Categorie::Engin => false,
            };
            assert_eq!(marque_a_la_racine(cat), attendu, "{cat:?}");
        }
    }

    /// **Le losange de la comète et le triangle des autres**, même pointe,
    /// même boîte. La boîte compte : [`RAYON_HIT_PX`] est réglé sur elle, et
    /// une forme qui en sortirait se cliquerait moins bien que ce qu'on voit.
    #[test]
    fn le_contour_dit_la_categorie_sans_sortir_de_la_boite() {
        let pointe = vec2(200.0, 300.0);
        let losange = contour(Categorie::Comete, pointe);
        assert_eq!(losange.len(), 4, "la comète a quatre sommets : {losange:?}");

        for cat in [Categorie::Planete, Categorie::Lune, Categorie::Etoile, Categorie::Engin] {
            assert_eq!(contour(cat, pointe).len(), 3, "{cat:?} garde le triangle");
        }

        for cat in [Categorie::Comete, Categorie::Planete] {
            let pts = contour(cat, pointe);
            assert_eq!(pts[0], pointe, "{cat:?} : la pointe vient en premier");
            for p in &pts {
                assert!((p.x - pointe.x).abs() <= DEMI_BASE_PX + 0.01, "{cat:?} déborde en x : {p:?}");
                // y écran croît vers le bas : la forme monte au-dessus de la pointe.
                assert!(
                    p.y <= pointe.y + 0.01 && p.y >= pointe.y - HAUTEUR_PX - 0.01,
                    "{cat:?} déborde en y : {p:?}"
                );
            }
        }

        // Ce qui fait un losange et non un triangle pointe en bas : des
        // flancs écartés À MI-HAUTEUR, et un sommet qui se referme en haut.
        assert_eq!(losange[1], vec2(pointe.x - DEMI_BASE_PX, pointe.y - HAUTEUR_PX * 0.5));
        assert_eq!(losange[3], vec2(pointe.x + DEMI_BASE_PX, pointe.y - HAUTEUR_PX * 0.5));
        assert_eq!(losange[2], vec2(pointe.x, pointe.y - HAUTEUR_PX), "le sommet haut est sur l'axe");
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
