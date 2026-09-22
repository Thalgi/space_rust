use super::{Calques, Systeme};

/// **Teintes des quatre calques de lecture.** Nommées ensemble parce que ce qui
/// compte n'est pas chacune prise à part mais qu'elles se **distinguent entre
/// elles** : quatre réseaux de traits superposés sur le même fond noir, dont deux
/// confondus, et on ne sait plus ce qu'on regarde. Un test tient l'écart
/// (`les_calques_ne_se_confondent_pas`).
pub(super) const COL_PLANETES: Color = Color::new(0.13, 0.78, 0.75, 0.75); // sarcelle
pub(super) const COL_ETOILES: Color = Color::new(0.85, 0.80, 0.55, 0.32); // paille, discret
pub(super) const COL_LUNES: Color = Color::new(0.62, 0.55, 0.90, 0.38); // violet pâle
pub(super) const COL_ZONE: Color = Color::new(0.30, 0.90, 0.40, 0.45); // vert habitable
/// Aperçu de trajet (Lot D) — ambre, pour trancher net des quatre teintes
/// ci-dessus (sarcelle/paille/violet/vert) sur le même fond noir.
const COL_PREVISUALISATION: Color = Color::new(1.0, 0.75, 0.20, 0.85);
use crate::astre::{CameraInfo, Categorie, Foyer};
use macroquad::prelude::*;

impl Systeme {
    /// Lumière principale du système : étoile hôte, sinon lumière de secours
    /// (planète isolée). Renvoie (position, couleur).
    pub(crate) fn lumiere_principale(&self) -> (Vec3, Vec3) {
        let etoile = self
            .astres
            .iter()
            .find(|a| a.categorie() == Categorie::Etoile);
        let secours = self.lumiere_secours();
        let light = etoile
            .map(|a| a.corps().position)
            .or(secours.map(|(p, _)| p))
            .unwrap_or(Vec3::ZERO);
        let light_color = etoile
            .and_then(|a| a.lumiere())
            .or(secours.map(|(_, c)| c))
            .unwrap_or(Vec3::ONE);
        (light, light_color)
    }

    /// Décor « net » (jamais pixelisé) : zones habitables + trajectoires.
    /// Dessiné en pleine résolution, sous les corps célestes.
    /// **Autour de quoi l'astre `i` tourne**, s'il tourne autour de quelque chose.
    ///
    /// Une lune tient à sa planète par `parent` ; une planète tient à son étoile
    /// par son `foyer`. Les deux disent la même chose — « ce corps est en orbite
    /// autour de celui-là » — et les confondre ici évite d'avoir à s'en souvenir
    /// partout ailleurs. Un foyer barycentrique ne désigne aucun astre : rien.
    pub(super) fn attache(&self, i: usize) -> Option<usize> {
        let a = self.astres.get(i)?;
        if let Some(p) = a.parent() {
            return Some(p);
        }
        match a.foyer() {
            Some(Foyer::Etoile(s)) => Some(s),
            _ => None,
        }
    }

    /// `cam` : position de l'œil — les traces en points y règlent leur
    /// espacement (voir `dessiner_points`).
    pub fn draw_orbites(&self, c: Calques, cam: Vec3) {
        let Calques {
            planetes: orbites_planetes,
            etoiles: orbites_etoiles,
            lunes: orbites_lunes,
            zone: montrer_zone,
            focus,
            previsualisation,
        } = c;
        // Opacité d'une trace appartenant à `idx`, selon l'astre focalisé.
        let opacite = |idx: usize| {
            let Some(f) = focus else { return 1.0 };
            let cat = |i: usize| self.astres.get(i).map(|a| a.categorie());
            if trace_en_avant(f, idx, |i| self.attache(i), cat) { 1.0 } else { ATTENUATION }
        };
        let (light, _) = self.lumiere_principale();

        // Zone habitable — multi-étoile :
        //  - CIRCUMSTELLAIRE (type S) : la HZ propre de chaque étoile, centrée sur elle.
        //  - CIRCUMBINAIRE (type P) : la HZ de la luminosité TOTALE autour du barycentre,
        //    dessinée seulement si elle tombe nettement hors des orbites stellaires.
        if montrer_zone {
            let vert = COL_ZONE;
            let mut l_tot = 0.0f32;
            let mut nb_et = 0usize;
            let mut r_max = 0.0f32;
            for a in self.astres.iter() {
                if a.categorie() != Categorie::Etoile {
                    continue;
                }
                if let Some((interne, externe)) = a.zone_viable() {
                    let c = a.corps().position;
                    dessiner_cercle(c, interne, vert);
                    dessiner_cercle(c, externe, vert);
                }
                if let Some(l) = a.luminosite() {
                    l_tot += l;
                }
                nb_et += 1;
                r_max = r_max.max(a.corps().position.length());
            }
            if nb_et >= 2 {
                let (i, o) = crate::etoile::zone_habitable(l_tot);
                let (iw, ow) = (i * crate::etoile::UA, o * crate::etoile::UA);
                if iw > 2.0 * r_max {
                    let cyan = Color::new(0.3, 0.85, 0.85, 0.4);
                    dessiner_cercle(Vec3::ZERO, iw, cyan);
                    dessiner_cercle(Vec3::ZERO, ow, cyan);
                }
            }
        }

        // Trajectoires des PLANÈTES (ellipses pleines), centrées sur LEUR foyer
        // (étoile hôte S-type, ou barycentre P-type).
        if orbites_planetes {
            for (idx, a) in self.astres.iter().enumerate() {
                let pts = a.orbite();
                if pts.is_empty() {
                    continue;
                }
                let centre = match a.foyer() {
                    Some(Foyer::Etoile(i)) => {
                        self.astres.get(i).map_or(Vec3::ZERO, |s| s.corps().position)
                    }
                    Some(Foyer::Barycentre) => Vec3::ZERO,
                    None => light,
                };
                dessiner_points(pts, centre, attenue(COL_PLANETES, opacite(idx)), cam);
            }
        }

        // Orbites des LUNES : centrées sur leur planète, qui bouge — d'où un
        // repère relatif, contrairement aux trajectoires planétaires.
        if orbites_lunes {
            for (idx, a) in self.astres.iter().enumerate() {
                let (Some(p), pts) = (a.parent(), a.orbite_lune()) else {
                    continue;
                };
                if pts.is_empty() {
                    continue;
                }
                let Some(centre) = self.astres.get(p).map(|x| x.corps().position) else {
                    continue;
                };
                dessiner_points(pts, centre, attenue(COL_LUNES, opacite(idx)), cam);
            }
        }

        // Orbites des ÉTOILES (systèmes multiples) : pointillés, teinte stellaire.
        if orbites_etoiles {
            if let Some(arbre) = &self.arbre {
                // Les polylignes de l'arbre stellaire ne portent pas d'index
                // d'astre : impossible de les lier une à une. On garde donc
                // TOUTES les orbites stellaires quand c'est une étoile qu'on
                // regarde, et on les efface sinon — approximation assumée.
                let k = match focus.and_then(|f| self.astres.get(f)) {
                    Some(a) if a.categorie() == Categorie::Etoile => 1.0,
                    Some(_) => ATTENUATION,
                    None => 1.0,
                };
                for poly in arbre.orbites_etoiles(self.temps) {
                    dessiner_pointilles(&poly, attenue(COL_ETOILES, k));
                }
            }
        }

        // Aperçu de trajet (Lot D) : arc OUVERT, jamais bouclé sur lui-même —
        // contrairement aux orbites ci-dessus, `dessiner_points` bouclerait
        // le dernier point sur le premier (`% n`), reliant l'arrivée au
        // départ à tort.
        if let Some(poly) = previsualisation {
            dessiner_points_ouvert(&poly, COL_PREVISUALISATION, cam);
        }
    }

    /// Corps célestes (étoiles, planètes, lunes…) avec éclairage multi-source.
    /// C'est la seule couche pixelisée quand le filtre rétro est actif.
    /// Repère caméra **complété de l'éclairage** du système : étoile primaire
    /// plus jusqu'à quatre sources.
    ///
    /// Sorti de `draw_corps` pour que la vignette du panneau d'astre s'en serve
    /// aussi. Une seconde version du calcul montrerait le portrait sous une
    /// autre lumière que la vue, ce qui est précisément ce qu'un portrait ne
    /// doit pas faire.
    pub(crate) fn eclairage(&self, cam: CameraInfo) -> CameraInfo {
        let (light, light_color) = self.lumiere_principale();
        let mut lights_pos = [Vec3::ZERO; 4];
        let mut lights_color = [Vec3::ZERO; 4];
        let mut nb = 0usize;
        for a in self.astres.iter() {
            if nb >= 4 {
                break;
            }
            if a.categorie() == Categorie::Etoile {
                if let Some(col) = a.lumiere() {
                    lights_pos[nb] = a.corps().position;
                    lights_color[nb] = col;
                    nb += 1;
                }
            }
        }
        if nb == 0 {
            lights_pos[0] = light;
            lights_color[0] = light_color;
        }
        let mut c = cam;
        c.light_pos = light;
        c.light_color = light_color;
        c.lights_pos = lights_pos;
        c.lights_color = lights_color;
        c
    }

    pub fn draw_corps(&mut self, cam: &CameraInfo) {
        let (light, light_color) = self.lumiere_principale();

        // Éclairage multi-source : jusqu'à 4 étoiles ([0] = primaire = light).
        let mut lights_pos = [Vec3::ZERO; 4];
        let mut lights_color = [Vec3::ZERO; 4];
        let mut nb = 0usize;
        for a in self.astres.iter() {
            if nb >= 4 {
                break;
            }
            if a.categorie() == Categorie::Etoile {
                if let Some(col) = a.lumiere() {
                    lights_pos[nb] = a.corps().position;
                    lights_color[nb] = col;
                    nb += 1;
                }
            }
        }
        if nb == 0 {
            // Aucune étoile : lumière de secours en source unique.
            lights_pos[0] = light;
            lights_color[0] = light_color;
        }

        let mut c = *cam;
        c.light_pos = light;
        c.light_color = light_color;
        c.lights_pos = lights_pos;
        c.lights_color = lights_color;
        for a in &mut self.astres {
            a.draw(&c);
        }
    }
}

/// Ce qui reste d'une trace **non liée** à l'astre focalisé : un dixième.
///
/// Effacée plutôt que supprimée — le reste du système doit rester lisible en
/// filigrane, sinon désigner une planète donne l'impression que les autres ont
/// disparu.
const ATTENUATION: f32 = 0.10;

/// Une trace appartenant à `idx` est-elle **liée** à l'astre focalisé `focus` ?
///
/// Lié veut dire : lui-même, ce autour de quoi il tourne, ou ce qui tourne
/// autour de lui. Désigner une étoile garde donc **toutes les trajectoires de
/// ses planètes** — sans quoi on ne voit plus le système du tout ; désigner une
/// planète garde sa trace et celles de ses lunes ; désigner une lune garde la
/// sienne et celle de sa planète.
///
/// ⚠️ **`attache` n'est pas `Astre::parent`.** `parent` n'est renseigné que pour
/// les lunes ; une planète tient à son étoile par son `foyer`. La première
/// version ne regardait que `parent`, si bien que désigner une étoile effaçait
/// toutes les planètes — exactement ce qu'on voulait éviter. La règle porte donc
/// sur « autour de quoi ce corps tourne », les deux cas confondus.
///
/// `attache` est passée en argument plutôt que lue sur le système : la règle est
/// une question d'appartenance, pas de rendu, et se teste sans écran.
fn liee_au_focus(focus: usize, idx: usize, attache: impl Fn(usize) -> Option<usize>) -> bool {
    idx == focus || attache(idx) == Some(focus) || attache(focus) == Some(idx)
}

/// Faut-il garder la trace de `idx` en pleine opacité, l'astre `focus` étant
/// désigné ?
///
/// Deux raisons de la garder, et il faut les deux :
///
/// 1. le **lien d'orbite** ([`liee_au_focus`]) — planète et ses lunes, lune et sa
///    planète ;
/// 2. **une étoile désignée met en avant tout son système planétaire.**
///
/// ⚠️ Le point 2 n'est pas une redite du point 1. Le lien fin passe par le foyer,
/// or `genese::ajouter_planete` pose TOUTES les planètes sur `Foyer::Barycentre`
/// — l'étoile étant à l'origine, barycentre et étoile se confondent, mais
/// `attache` n'y voit aucun astre. Sans cette seconde règle, désigner l'étoile
/// effaçait le système entier : plus rien pour le lire. Les lunes, elles,
/// s'effacent — elles sont un cran plus bas et nombreuses.
fn trace_en_avant(
    focus: usize,
    idx: usize,
    attache: impl Fn(usize) -> Option<usize>,
    categorie: impl Fn(usize) -> Option<Categorie>,
) -> bool {
    liee_au_focus(focus, idx, attache)
        || (categorie(focus) == Some(Categorie::Etoile)
            && categorie(idx) == Some(Categorie::Planete))
}

/// Applique un facteur d'opacité à une teinte de calque.
fn attenue(c: Color, k: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * k)
}

/// **Espacement visé entre deux points d'une trace, en pixels écran.**
///
/// C'est ce qui fait lire « cercle de points » plutôt que « trait » ou
/// « poignée de marques » : un semis régulier, quelle que soit la distance.
const ESPACEMENT_PX: f32 = 22.0;

/// **Longueur d'un point, en pixels écran.**
///
/// En PIXELS et non en fraction de l'espacement : les deux réglages deviennent
/// indépendants, si bien qu'espacer les points ne les allonge plus. C'est aussi
/// la seule façon de dire « des petits points » et que ça reste vrai à tous les
/// zooms — `draw_line_3d` rend un pixel de large, donc 3 px de long donne bien
/// un point et non un tiret.
const LONGUEUR_POINT_PX: f32 = 3.0;

/// Nombre de pixels que couvre une unité monde à la distance `dist`.
/// Même petit angle que partout ailleurs (fov vertical de 45°).
fn px_par_unite(dist: f32) -> f32 {
    let demi_fov = 22.5_f32.to_radians();
    screen_height() * 0.5 / (demi_fov.tan() * dist.max(1e-3))
}

/// Combien d'échantillons sauter pour approcher [`ESPACEMENT_PX`] à l'écran.
///
/// `ecart_px` : distance écran entre deux échantillons consécutifs.
///
/// Deux bornes, et chacune répare un travers observé :
/// - **jamais 0**, sinon la boucle de tracé n'avance plus ;
/// - **jamais plus du quart** des échantillons, faute de quoi une orbite vue de
///   très loin se réduirait à trois marques et ne dirait plus qu'elle est ronde.
fn pas_points(ecart_px: f32, n: usize) -> usize {
    let plafond = (n as f32 * 0.25).max(1.0);
    (ESPACEMENT_PX / ecart_px.max(1e-6)).round().clamp(1.0, plafond) as usize
}

/// Trace une polyligne fermée en **points**, à espacement écran ~constant.
///
/// Un semis à pas fixe dans la polyligne ne peut pas marcher : la même orbite
/// couvre quelques pixels vue de loin et plusieurs écrans vue de près. À pas
/// fixe, on obtient soit un trait plein (les points se recouvrent), soit deux
/// marques perdues sur l'écran. On mesure donc l'espacement **à l'écran** et on
/// saute ce qu'il faut d'échantillons — d'où une réserve d'échantillons haute à
/// la source, qui ne sert que de près.
///
/// La distance retenue est celle du point le **plus proche** de la caméra, pas
/// celle du foyer : vue de l'intérieur d'une grande orbite, l'arc proche est à
/// des ordres de grandeur du foyer, et c'est lui qu'on regarde.
fn dessiner_points(poly: &[Vec3], centre: Vec3, col: Color, cam: Vec3) {
    let n = poly.len();
    if n == 0 {
        return;
    }
    // Balayage grossier : un échantillon sur 16 suffit à trouver l'arc proche,
    // et évite un parcours complet par orbite et par frame.
    let mut proche = f32::MAX;
    let mut i = 0;
    while i < n {
        proche = proche.min((centre + poly[i] - cam).length_squared());
        i += 16;
    }
    let px = px_par_unite(proche.max(0.0).sqrt());

    // Espacement d'un échantillon à l'autre, en pixels, puis le pas qui approche
    // le mieux l'espacement visé.
    let pas = pas_points((poly[1 % n] - poly[0]).length() * px, n);

    // Demi-longueur du point, en unités monde : la taille écran voulue divisée
    // par l'échelle. Les points de l'arc lointain sortent donc plus courts —
    // c'est la perspective, et c'est juste.
    let demi_monde = LONGUEUR_POINT_PX / px.max(1e-6) * 0.5;
    let mut i = 0;
    while i < n {
        let a = centre + poly[i];
        let b = centre + poly[(i + pas) % n];
        let demi = (b - a).normalize_or_zero() * demi_monde;
        draw_line_3d(a - demi, a + demi, col);
        i += pas;
    }
}

/// Trace une polyligne **ouverte** en points, à espacement écran ~constant —
/// même technique que [`dessiner_points`], adaptée à un arc à sens unique.
///
/// Deux écarts avec [`dessiner_points`] : pas de `centre` (la trajectoire
/// rend déjà des coordonnées monde absolues, voir `Trajectoire::position`),
/// et surtout pas de `% n` — un arc ouvert dont le dernier segment boucle sur
/// le premier point relierait à tort l'arrivée au départ.
fn dessiner_points_ouvert(poly: &[Vec3], col: Color, cam: Vec3) {
    let n = poly.len();
    if n < 2 {
        return;
    }
    let mut proche = f32::MAX;
    let mut i = 0;
    while i < n {
        proche = proche.min((poly[i] - cam).length_squared());
        i += 16;
    }
    let px = px_par_unite(proche.max(0.0).sqrt());
    let pas = pas_points((poly[1] - poly[0]).length() * px, n);
    let demi_monde = LONGUEUR_POINT_PX / px.max(1e-6) * 0.5;
    let mut i = 0;
    while i + 1 < n {
        let j = (i + pas).min(n - 1);
        let a = poly[i];
        let b = poly[j];
        let demi = (b - a).normalize_or_zero() * demi_monde;
        draw_line_3d(a - demi, a + demi, col);
        i += pas;
    }
}

/// Trace une polyligne fermée en pointillés fins : on ne dessine qu'une courte
/// fraction (`FRAC`) de chaque segment -> tirets courts, espacés régulièrement.
fn dessiner_pointilles(poly: &[Vec3], col: Color) {
    const FRAC: f32 = 0.4; // longueur du tiret (fraction du segment) ; le reste = vide
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        draw_line_3d(a, a + (b - a) * FRAC, col);
    }
}

/// Trace un cercle dans le plan xz (orbite ou zone habitable).
fn dessiner_cercle(centre: Vec3, r: f32, col: Color) {
    let n = 96;
    let mut prec = centre + vec3(r, 0.0, 0.0);
    for i in 1..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let p = centre + vec3(r * a.cos(), 0.0, r * a.sin());
        draw_line_3d(prec, p, col);
        prec = p;
    }
}

#[cfg(test)]
mod tests_calques {
    use super::*;

    fn lab(c: Color) -> Vec3 {
        crate::palette::rgb_vers_lab(vec3(c.r, c.g, c.b))
    }

    /// **Les quatre calques doivent rester distinguables entre eux.**
    ///
    /// Ils se superposent sur le même fond noir : trajectoires planétaires,
    /// orbites stellaires, orbites lunaires, zone habitable. Deux teintes trop
    /// proches et on ne sait plus lequel on regarde — un défaut qui ne casse
    /// aucun test de rendu et ne se voit qu'en croisant les quatre à l'écran.
    ///
    /// Mesuré en **Lab**, où la distance approche la différence perçue, et non
    /// en RGB, où deux couleurs éloignées numériquement peuvent se ressembler.
    #[test]
    fn les_calques_ne_se_confondent_pas() {
        let calques = [
            ("planètes", COL_PLANETES),
            ("étoiles", COL_ETOILES),
            ("lunes", COL_LUNES),
            ("zone", COL_ZONE),
        ];
        // 25 : au-delà d'un « on les voit différentes », en deçà d'une exigence
        // qui interdirait toute famille de teintes cohérente.
        const ECART_MIN: f32 = 25.0;
        for (i, (na, ca)) in calques.iter().enumerate() {
            for (nb, cb) in calques.iter().skip(i + 1) {
                let d = (lab(*ca) - lab(*cb)).length();
                assert!(d >= ECART_MIN, "{na} et {nb} se ressemblent trop (ΔLab = {d:.1})");
            }
        }
    }

    /// **Le semis garde un espacement à peu près constant à l'écran.**
    ///
    /// C'est tout l'objet du pas adaptatif : une même orbite couvre quelques
    /// pixels vue de loin et plusieurs écrans vue de près. À pas fixe on
    /// obtient, selon le bout, un trait plein ou deux marques perdues — et
    /// c'était le défaut signalé, « pas assez de points pour voir le cercle ».
    #[test]
    fn le_semis_garde_son_espacement_a_lecran() {
        const N: usize = 512;
        // Échantillons très espacés à l'écran (zoom serré) : on les prend tous.
        assert_eq!(pas_points(60.0, N), 1);
        assert_eq!(pas_points(ESPACEMENT_PX, N), 1);
        // Échantillons serrés (vue lointaine) : on en saute d'autant plus.
        assert!(pas_points(1.0, N) > pas_points(5.0, N));
        // Et l'espacement obtenu reste proche de la cible.
        for ecart in [0.5_f32, 1.0, 3.0, 7.0, 15.0] {
            let obtenu = ecart * pas_points(ecart, N) as f32;
            assert!(
                (obtenu - ESPACEMENT_PX).abs() <= ESPACEMENT_PX * 0.6,
                "écart {ecart} px -> espacement {obtenu:.1} px, loin de {ESPACEMENT_PX}"
            );
        }
    }

    /// Les deux bornes du pas, chacune contre un travers précis.
    #[test]
    fn le_pas_ne_bloque_ni_ne_vide_le_semis() {
        const N: usize = 512;
        // Un pas nul ferait boucler le tracé sans jamais avancer.
        assert!(pas_points(1e9, N) >= 1);
        assert!(pas_points(0.0, N) >= 1);
        // Et de très loin, il reste assez de points pour lire une courbe.
        let pas = pas_points(1e-9, N);
        assert!(pas <= N / 4, "pas de {pas} sur {N} : il ne resterait pas un cercle");
        assert!(N / pas >= 4, "moins de quatre points : plus de cercle du tout");
        // Une polyligne minuscule ne doit pas produire un pas nul non plus.
        assert!(pas_points(1e-9, 2) >= 1);
    }

    /// **Désigner un corps met en avant ce qui lui est accroché.**
    ///
    /// Sur un système garni, une douzaine de cercles superposés empêchent de
    /// voir lequel appartient au corps qu'on vient de désigner. On garde donc à
    /// pleine opacité sa trace, celle de son parent et celles de ses enfants ;
    /// tout le reste tombe à un dixième.
    ///
    /// Arbre d'essai : 0 étoile, 1 et 2 planètes, 3 et 4 lunes de la planète 1.
    #[test]
    fn le_focus_garde_la_famille_et_efface_le_reste() {
        let parent = |i: usize| match i {
            1 | 2 => Some(0),
            3 | 4 => Some(1),
            _ => None,
        };

        // Focus sur la planète 1 : elle, son étoile, et ses deux lunes.
        for lie in [1, 0, 3, 4] {
            assert!(liee_au_focus(1, lie, parent), "{lie} devrait rester lié à la planète 1");
        }
        // La planète soeur, non : c'est tout l'objet du filtre.
        assert!(!liee_au_focus(1, 2, parent), "la planète soeur ne devrait pas rester");

        // Focus sur une lune : elle et SA planète — on veut voir à quoi elle
        // est accrochée, pas seulement elle.
        assert!(liee_au_focus(3, 3, parent));
        assert!(liee_au_focus(3, 1, parent), "la planète hôte devrait rester");
        // Mais pas sa lune soeur, ni l'étoile (grand-parent), ni l'autre planète.
        for loin in [4, 0, 2] {
            assert!(!liee_au_focus(3, loin, parent), "{loin} ne devrait pas rester");
        }
    }

    /// **Désigner l'étoile montre tout le système planétaire.**
    ///
    /// Le lien fin ne suffit pas : `genese::ajouter_planete` pose les planètes
    /// sur `Foyer::Barycentre`, donc `attache` ne les rattache à aucun astre.
    /// Sans la règle « étoile désignée ⇒ planètes en avant », désigner l'étoile
    /// effaçait tout, et il ne restait rien pour lire le système. C'est
    /// exactement ce qui a été signalé à l'écran.
    #[test]
    fn designer_l_etoile_montre_tout_le_systeme() {
        // Ni parent ni foyer : le cas réel du préset solaire.
        let sans_attache = |_: usize| None;
        let cat = |i: usize| {
            Some(match i {
                0 => Categorie::Etoile,
                1 | 2 => Categorie::Planete,
                _ => Categorie::Lune,
            })
        };
        for planete in [1, 2] {
            assert!(
                trace_en_avant(0, planete, sans_attache, cat),
                "planète {planete} effacée alors que son étoile est désignée"
            );
        }
        // Les lunes, elles, restent en retrait : un cran plus bas, et nombreuses.
        assert!(!trace_en_avant(0, 3, sans_attache, cat));
        // Et l'inverse ne vaut pas : désigner une planète ne rallume pas ses sœurs.
        assert!(!trace_en_avant(1, 2, sans_attache, cat));
    }

    /// L'atténuation efface sans supprimer : le reste du système doit rester
    /// lisible en filigrane, sinon désigner une planète donne l'impression que
    /// les autres ont disparu.
    #[test]
    fn les_traces_effacees_restent_du_filigrane() {
        const _: () = assert!(ATTENUATION > 0.0, "les autres traces disparaîtraient tout à fait");
        const _: () = assert!(ATTENUATION <= 0.15, "l'effacement ne se verrait pas");
        // Et l'atténuation ne touche QUE l'alpha : une trace effacée ne doit pas
        // changer de couleur, sinon on croit à un autre calque.
        let c = attenue(COL_PLANETES, ATTENUATION);
        assert_eq!((c.r, c.g, c.b), (COL_PLANETES.r, COL_PLANETES.g, COL_PLANETES.b));
        assert!((c.a - COL_PLANETES.a * ATTENUATION).abs() < 1e-6);
    }

    /// Les calques sont un **fond de carte**, pas le sujet : aucun ne doit être
    /// opaque, sinon un réseau de traits masque les corps qu'il documente.
    #[test]
    fn les_calques_restent_des_traits_de_fond() {
        for (nom, c) in [
            ("planètes", COL_PLANETES),
            ("étoiles", COL_ETOILES),
            ("lunes", COL_LUNES),
            ("zone", COL_ZONE),
        ] {
            assert!(c.a < 0.85, "{nom} : alpha {} trop opaque pour un calque", c.a);
            assert!(c.a > 0.2, "{nom} : alpha {} invisible", c.a);
        }
    }
}
