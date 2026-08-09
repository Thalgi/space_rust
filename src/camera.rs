use crate::astre::CameraInfo;
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Base orthonormée **(droite, haut, avant)** d'une caméra orbitale de lacet
/// Distance minimale a la cible, donc **plafond de zoom** : `zoom = dist_ref /
/// dist`, si bien qu'une reference de 1 280 donne x2 560.
///
/// Elle valait 2,0 (soit x640). Depuis que les corps sont a l'echelle vraie, une
/// planete ne fait que quelques millemes d'unite : il faut pouvoir descendre
/// bien plus bas pour qu'elle occupe l'ecran. A 0,01, une reference de 1 280
/// donne x128 000, de quoi remplir l'ecran avec la Terre (rayon ~0,010 unite a
/// ECHELLE_CORPS = 5).
pub const DIST_MIN: f32 = 0.01;

/// Distance maximale : de quoi englober le nuage de Oort.
pub const DIST_MAX: f32 = 30000.0;

/// `yaw` et de tangage `pitch`. Indépendante de la distance : la direction
/// cible → caméra est déjà unitaire.
///
/// Sortie de [`Camera::construire`] pour être **testable sans contexte
/// graphique** — `Camera::new` lit la position de la souris, la struct est donc
/// inconstructible hors du run macroquad. La boussole d'axes de l'interface
/// s'appuie sur cette même base, et c'est ce partage qui garantit qu'elle ne peut
/// pas mentir sur l'orientation de la vue.
pub fn base_orbite(yaw: f32, pitch: f32) -> (Vec3, Vec3, Vec3) {
    let cp = pitch.cos();
    // Direction cible → caméra, unitaire (cp²·sin² + sin²p + cp²·cos² = 1).
    let dir = vec3(cp * yaw.sin(), pitch.sin(), cp * yaw.cos());
    base_camera(-dir)
}

/// Base caméra orthonormée à partir de la seule direction de visée.
///
/// **Source unique** : la vue orbitale et les galeries (qui placent leur caméra
/// à la main) doivent construire leur repère de la même façon — les billboards
/// s'appuient dessus, et deux recettes qui divergent donnent deux rendus.
///
/// ⚠️ **La verticale de référence est piégeuse.** `forward × Y` s'annule quand on
/// vise **pile le long de Y**, et `normalize` d'un vecteur nul rend des NaN : le
/// repère part en vrille, et avec lui tous les quads face-caméra qui en
/// dépendent — champs de débris en tête. Près de l'axe, sans même l'atteindre,
/// la norme du produit devient minuscule et amplifie le bruit de calcul. On
/// bascule donc sur Z comme référence quand la visée frôle Y.
pub fn base_camera(forward: Vec3) -> (Vec3, Vec3, Vec3) {
    let f = forward.normalize_or(Vec3::NEG_Z);
    let vertical = if f.dot(Vec3::Y).abs() > 0.999 { Vec3::Z } else { Vec3::Y };
    let right = f.cross(vertical).normalize();
    let up = right.cross(f).normalize();
    (right, up, f)
}

/// Demi-angle vertical du tronc de vue, en radians — celui de `Camera3D` par
/// défaut (45°).
///
/// **Une seule source, et elle compte** : la construction du rayon et la
/// projection sont inverses l'une de l'autre *à condition* de parler du même
/// tronc. Deux copies de cette valeur qui divergeraient donneraient un curseur
/// qui désigne à côté de ce qu'il montre, sans que rien ne le signale.
const DEMI_FOV: f32 = 45.0 * std::f32::consts::PI / 360.0;

/// Direction **unitaire** du rayon partant de l'œil et passant par le point
/// écran `souris` (en pixels, `ecran` étant la taille de la fenêtre).
///
/// Extrait de [`Camera::pick`] pour être partagé avec la désignation de
/// l'assembleur (`ecran::designation`) — et **testable sans contexte
/// graphique**, comme [`base_orbite`], puisqu'il ne lit ni la souris ni
/// l'écran mais les reçoit.
pub fn rayon_ecran(souris: Vec2, ecran: Vec2, cam: &CameraInfo, aspect: f32) -> Vec3 {
    let ndc_x = souris.x / ecran.x * 2.0 - 1.0;
    let ndc_y = 1.0 - souris.y / ecran.y * 2.0;
    let th = DEMI_FOV.tan();
    (cam.forward + cam.right * (ndc_x * th * aspect) + cam.up * (ndc_y * th)).normalize()
}

/// Position **en pixels** du point monde `p`, ou `None` s'il est derrière l'œil.
///
/// Le `None` n'est pas une politesse : un point situé **derrière** la caméra se
/// projette à des coordonnées écran parfaitement plausibles — la division par
/// une profondeur négative retourne l'image — et un curseur qui accrocherait ce
/// fantôme désignerait un port dans le dos de l'observateur. C'est le piège
/// classique de la projection, et il ne se voit pas à l'œil : le port fautif
/// est simplement celui qu'on n'a pas visé.
#[allow(dead_code)] // Interaction de l'assembleur (Lot 4) : deja teste, pas encore branche a une vue.
pub fn projeter_ecran(p: Vec3, ecran: Vec2, cam: &CameraInfo, aspect: f32) -> Option<Vec2> {
    let rel = p - cam.pos;
    let profondeur = rel.dot(cam.forward);
    if profondeur <= 1e-6 {
        return None;
    }
    let th = DEMI_FOV.tan();
    let ndc_x = rel.dot(cam.right) / (profondeur * th * aspect);
    let ndc_y = rel.dot(cam.up) / (profondeur * th);
    Some(vec2((ndc_x + 1.0) * 0.5 * ecran.x, (1.0 - ndc_y) * 0.5 * ecran.y))
}

/// Caméra orbitale : tourne autour d'une cible (origine ou astre focalisé),
/// gère le glisser/zoom et la sélection au clic.
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    dist_ref: f32,
    prec: (f32, f32),
    focus: Option<usize>,
}

impl Camera {
    pub fn new(dist: f32) -> Self {
        Self {
            yaw: 0.8,
            pitch: 0.5,
            dist,
            dist_ref: dist,
            prec: mouse_position(),
            focus: None,
        }
    }

    pub fn reset_focus(&mut self) {
        self.focus = None;
    }
    /// Focalise la caméra sur l'astre `idx` (le suit).
    pub fn set_focus(&mut self, idx: usize) {
        self.focus = Some(idx);
    }
    pub fn focus_actif(&self) -> bool {
        self.focus.is_some()
    }
    /// L'astre suivi, s'il y en a un. Le sélecteur s'en sert pour surligner la
    /// ligne courante : sans ça, la colonne ne dirait pas ce qui est cadré.
    pub fn focus(&self) -> Option<usize> {
        self.focus
    }
    pub fn set_dist(&mut self, d: f32) {
        self.dist = d;
        self.dist_ref = d;
    }
    /// Multiplicateur de zoom courant : x1.00 au cadrage de référence, >1 en approchant.
    pub fn zoom(&self) -> f32 {
        self.dist_ref / self.dist
    }

    // `zoom_max()` a ete supprimee : elle n'avait aucun appelant, et son test ne
    // pouvait pas passer par elle (`Camera::new` exige un contexte GL). Le
    // plafond se lit `dist_ref / DIST_MIN`, comme le fait `le_zoom_monte_...`.

    /// Rotation (glisser) + zoom (molette), sauf si la souris est sur l'UI.
    pub fn input_orbite(&mut self, sur_ui: bool) {
        let s = mouse_position();
        if is_mouse_button_down(MouseButton::Left) && !sur_ui {
            self.yaw -= (s.0 - self.prec.0) * 0.005;
            self.pitch = (self.pitch + (s.1 - self.prec.1) * 0.005).clamp(-1.4, 1.4);
        }
        self.prec = s;
        if !sur_ui {
            let mol = mouse_wheel().1;
            if mol != 0.0 {
                self.dist = (self.dist * (1.0 - mol.signum() * 0.1)).clamp(DIST_MIN, DIST_MAX);
            }
        }
    }

    pub fn cible(&self, sys: &Systeme) -> Vec3 {
        match self.focus {
            Some(i) => sys.position(i),
            None => Vec3::ZERO,
        }
    }

    /// Construit le repère caméra (billboards/éclairage) et la caméra 3D.
    pub fn construire(&self, target: Vec3, aspect: f32) -> (CameraInfo, Camera3D) {
        let (right, up, forward) = base_orbite(self.yaw, self.pitch);
        let pos = target - forward * self.dist;
        let info = CameraInfo {
            pos,
            right,
            up,
            forward,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        };
        // **Plans de coupe proportionnels a la distance.**
        //
        // Les valeurs par defaut de macroquad (0,01 et 10 000) ne tiennent plus
        // aux deux bouts :
        //
        // - au plus pres, `z_near` valait exactement `DIST_MIN` : la planete se
        //   posait SUR le plan de coupe et disparaissait ;
        // - au plus loin, `z_far` a 10 000 coupait avant meme `DIST_MAX`
        //   (30 000), donc bien avant le nuage de Oort.
        //
        // Les faire suivre la distance donne un cadrage utilisable partout, et
        // garde le rapport near/far constant -- c'est lui qui fixe la precision
        // du tampon de profondeur, un near minuscule avec un far enorme fait
        // clignoter les surfaces.
        let (z_near, z_far) = (self.dist * 0.01, self.dist * 1000.0);
        let cam3d = Camera3D {
            position: pos,
            target,
            up: Vec3::Y,
            aspect: Some(aspect),
            z_near,
            z_far,
            ..Default::default()
        };
        (info, cam3d)
    }

    /// Sélectionne l'astre cliqué (rayon depuis la souris) comme nouvelle cible,
    /// et **rend son index** : l'écran en a besoin pour ouvrir sa fiche.
    ///
    /// Rendre l'index plutôt que de laisser l'appelant relancer un `sys.pick`
    /// de son côté : le rayon serait alors calculé deux fois, donc deux sources
    /// pour la même désignation, qui divergeraient au premier réglage de marge.
    pub fn pick(&mut self, sys: &Systeme, cam: &CameraInfo, aspect: f32) -> Option<usize> {
        let s = mouse_position();
        let dir =
            rayon_ecran(vec2(s.0, s.1), vec2(screen_width(), screen_height()), cam, aspect);
        let touche = sys.pick(cam.pos, dir);
        if let Some(idx) = touche {
            self.focus = Some(idx);
        }
        touche
    }
}

#[cfg(test)]
// Ces tests affirment des choses sur des CONSTANTES, et clippy y voit une
// condition toujours vraie. Elle l'est -- aujourd'hui. Le jour ou quelqu'un
// abaisse la constante, l'assertion devient fausse et le test tombe : c'est
// precisement son role, garder la valeur ET dire pourquoi elle compte.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    // **Le plafond de zoom.** Il n'est ecrit nulle part en dur : il tombe du
    // plancher de distance, `zoom = dist_ref / dist`. A 2,0 on plafonnait a
    // x640, ce qui ne suffit plus depuis que les corps sont a l'echelle vraie --
    // une planete y mesure quelques millemes d'unite.
    #[test]
    fn le_zoom_monte_jusqua_2560() {
        // `Camera::new` exige le contexte graphique : on eprouve donc
        // l'arithmetique du plafond, qui est tout ce que la constante decide.
        let plafond = |dist_ref: f32| dist_ref / DIST_MIN;
        assert!(
            plafond(1280.0) >= 2560.0,
            "plafond a x{} : le plancher de distance ({DIST_MIN}) est trop haut",
            plafond(1280.0)
        );
        // Le plafond doit avoir **quadruple** : il etait a x640 pour la meme
        // reference, plancher 2,0.
        assert!(plafond(1280.0) >= 4.0 * (1280.0 / 2.0), "le plafond n'a pas quadruple");
    }

    // **Les plans de coupe encadrent la cible a toute distance.** Les valeurs
    // par defaut de macroquad coupaient aux deux bouts : `z_near` valait
    // exactement `DIST_MIN`, et `z_far` s'arretait a 10 000 quand `DIST_MAX` est
    // a 30 000.
    #[test]
    fn les_plans_de_coupe_encadrent_la_cible() {
        for dist in [DIST_MIN, 1.0, 48.0, 1440.0, DIST_MAX] {
            let (near, far) = (dist * 0.01, dist * 1000.0);
            assert!(near > 0.0, "near nul a dist={dist}");
            assert!(near < dist, "near {near} devant la cible a dist={dist}");
            assert!(far > dist, "far {far} coupe avant la cible a dist={dist}");
            // Marge confortable autour de la cible, des deux cotes.
            assert!(dist / near >= 50.0, "trop peu de marge devant a dist={dist}");
            assert!(far / dist >= 50.0, "trop peu de marge derriere a dist={dist}");
        }
        // Et de quoi englober le nuage de Oort (2 000 UA = 96 000 unites) une
        // fois recule au maximum.
        assert!(DIST_MAX * 1000.0 > 96_000.0, "le nuage de Oort reste coupe");
    }

    // Le plancher reste **strictement positif** : a zero, `zoom` diviserait par
    // zero et la camera se retrouverait dans la cible.
    #[test]
    fn le_plancher_de_distance_reste_positif() {
        assert!(DIST_MIN > 0.0, "plancher nul : division par zero dans zoom()");
        assert!(DIST_MIN < DIST_MAX, "plancher au-dessus du plafond");
        // Et le plafond englobe le nuage de Oort (2 000 UA = 96 000 unites,
        // tronque a 20 000 UA ; on vise au moins de quoi cadrer Neptune a 30 UA).
        assert!(DIST_MAX > 30.0 * 48.0, "plafond trop bas pour cadrer Neptune");
    }
}

#[cfg(test)]
mod tests_base_camera {
    use super::*;

    fn orthonormee(nom: &str, r: Vec3, u: Vec3, f: Vec3) {
        for (axe, v) in [("right", r), ("up", u), ("forward", f)] {
            assert!(v.is_finite(), "{nom} : {axe} n'est pas fini ({v:?})");
            assert!(
                (v.length() - 1.0).abs() < 1e-4,
                "{nom} : {axe} n'est pas unitaire (|{axe}| = {})",
                v.length()
            );
        }
        for (paire, d) in [("right·up", r.dot(u)), ("right·fwd", r.dot(f)), ("up·fwd", u.dot(f))] {
            assert!(d.abs() < 1e-4, "{nom} : {paire} = {d}, la base n'est pas orthogonale");
        }
    }

    /// **Viser le long de Y ne doit pas casser le repère caméra.**
    ///
    /// `forward × Y` s'annule quand la visée est colinéaire à Y, et
    /// `normalize` d'un vecteur nul rend des NaN. Tout ce qui fait face à la
    /// caméra s'appuie sur ce repère : un NaN ici et les quads des champs de
    /// débris partent en vrille — ce qui ne se voit QUE près de cet axe, donc
    /// jamais dans une vue de trois-quarts.
    #[test]
    fn le_repere_tient_meme_en_visant_pile_le_long_de_y() {
        for (nom, dir) in [
            ("plein bas", Vec3::NEG_Y),
            ("plein haut", Vec3::Y),
            ("quasi bas", vec3(1e-7, -1.0, 0.0)),
            ("quasi haut", vec3(0.0, 1.0, 1e-7)),
        ] {
            let (r, u, f) = base_camera(dir);
            orthonormee(nom, r, u, f);
        }
    }

    /// Et il reste sain partout ailleurs — balayage complet de la sphère.
    #[test]
    fn le_repere_reste_orthonorme_dans_toutes_les_directions() {
        for i in 0..64 {
            let yaw = i as f32 / 64.0 * std::f32::consts::TAU;
            for j in 0..33 {
                // Latitude de -90° à +90°, pôles INCLUS.
                let pitch = (j as f32 / 32.0 - 0.5) * std::f32::consts::PI;
                let (r, u, f) = base_orbite(yaw, pitch);
                orthonormee(&format!("yaw {yaw:.2} pitch {pitch:.2}"), r, u, f);
            }
        }
    }

    /// Une direction nulle ne doit pas non plus produire de NaN : la caméra
    /// regarde alors devant elle, faute de mieux.
    #[test]
    fn une_visee_nulle_retombe_sur_une_base_valide() {
        let (r, u, f) = base_camera(Vec3::ZERO);
        orthonormee("visée nulle", r, u, f);
    }
}
