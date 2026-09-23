//! **Orbites de lunes en trait large** (Skymap V2) : la polyligne d'orbite
//! est projetée à l'écran, puis tracée en 2D au motif « mediumdashdot ».
//!
//! ⚠️ **Pourquoi en 2D et pas avec les autres calques.** `draw_line_3d` de
//! macroquad n'a **pas** d'épaisseur : il rend toujours un pixel, quelle que
//! soit la distance. Les calques de `systeme/rendu.rs` (semis de points)
//! vivent avec cette contrainte ; un trait trois fois plus large ne peut donc
//! pas s'obtenir là-bas. `draw_line`, lui, prend une épaisseur — mais c'est
//! du dessin écran, donc une passe interface.
//!
//! Conséquence assumée : ce trait ne passe pas le test de profondeur, il se
//! voit même quand la lune est derrière sa planète. Sur un anneau d'orbite
//! que l'on n'affiche **que** pour l'astre sélectionné, c'est ce qu'on veut —
//! il sert à dire où chercher la lune.
//!
//! ⚠️ **La phase du motif est ancrée en MONDE, jamais en écran.** Une
//! première version accumulait la phase en longueur **projetée à l'écran**.
//! Mesuré : un seul pixel de glisser souris décalait la phase de 8,1 px
//! (37 % du cycle de 22 px) ; une frame de temps qui passe, 10,6 px (48 %).
//! La perspective déforme la longueur projetée dès que la caméra ou le
//! parent bouge, et ce décalage scintillait à l'œil (strobe des tirets).
//! Le contre-exemple du dépôt qui ne strobe pas : `dessiner_points`
//! (`src/systeme/rendu.rs`) ancre ses marques sur des **index fixes** de la
//! polyligne monde, jamais sur une mesure écran.
//!
//! Le remède ([`tirets_monde`]) : découper l'orbite en tirets le long de son
//! **abscisse curviligne monde**, qui ne dépend ni de la caméra ni du temps
//! — seule la TAILLE du cycle (combien d'unités monde pour un tiret) suit la
//! distance caméra → centre, quantifiée en puissance de deux
//! ([`cycle_monde`]) pour ne changer que par paliers pendant un zoom.
//! Conséquence assumée : la longueur d'un tiret À L'ÉCRAN n'est plus
//! rigoureusement constante — elle varie avec la perspective le long de
//! l'orbite, et par paliers avec le zoom. C'est le prix de l'absence de
//! strobe, à juger à l'œil.
//!
//! Comme `marqueur`, ce qui **décide** ([`tirets_monde`], [`cycle_monde`],
//! [`segments_motif_monde`]) est pur et testé ; seul le tracé touche
//! macroquad (`docs/methode.md` : pas de test de rendu).

use crate::astre::{CameraInfo, Categorie};
use crate::systeme::Systeme;
use macroquad::prelude::*;

/// Épaisseur du trait, en pixels — trois fois le trait d'un pixel des autres
/// calques, qui ne se lisait pas une fois l'orbite réduite à quelques dizaines
/// de pixels.
pub const EPAISSEUR_PX: f32 = 3.0;

/// Teinte des orbites de lunes en V2 : le violet pâle des calques
/// (`systeme::rendu::COL_LUNES`), remonté en opacité — un trait large et
/// transparent bave, là où un trait fin se lisait encore.
pub const COL_LUNES: Color = Color::new(0.62, 0.55, 0.90, 0.85);

/// Le motif **mediumdashdot**, en proportions d'un [`CYCLE`] : trait long,
/// trou, point, trou — répété. `true` = on pose de l'encre.
///
/// Ce ne sont plus des pixels écran : ces quatre valeurs ne sont plus lues
/// que via leur ratio à `CYCLE` (12/22, 4/22, 2/22, 4/22), pour dimensionner
/// un cycle exprimé en unités MONDE ([`cycle_monde`]).
const MOTIF: [(f32, bool); 4] = [(12.0, true), (4.0, false), (2.0, true), (4.0, false)];

/// Longueur d'un cycle complet du motif, **visée à l'écran** en pixels, à la
/// distance caméra → centre de l'orbite. Sert aussi de dénominateur aux
/// fractions de [`MOTIF`].
const CYCLE: f32 = 22.0;

/// Demi-angle vertical du tronc de vue, en radians (22,5°, FOV verticale de
/// 45°) — **même valeur** que `camera::DEMI_FOV` et le `22.5_f32.to_radians()`
/// de `systeme::rendu::px_par_unite`.
///
/// ⚠️ Troisième copie de cette constante dans le dépôt, pas une régression :
/// `camera::DEMI_FOV` est privée à son module, et `rendu.rs` la duplique déjà
/// en dur pour la même raison. Un partage propre demanderait de la rendre
/// publique ailleurs ; ce n'est pas l'objet de cette correction.
const DEMI_FOV: f32 = 22.5 * std::f32::consts::PI / 180.0;

/// Combien de pixels couvre une unité monde à la distance `dist` — même
/// calcul que `systeme::rendu::px_par_unite`, dupliqué ici pour la même
/// raison qu'expliqué sur [`DEMI_FOV`].
fn px_par_unite(ecran_y: f32, dist: f32) -> f32 {
    ecran_y * 0.5 / (DEMI_FOV.tan() * dist.max(1e-3))
}

/// **Longueur d'un cycle de motif, en unités MONDE**, quantifiée en
/// puissance de deux.
///
/// C'est tout l'ancrage de la correction : `dist` et `perimetre` ne
/// dépendent ni de la direction de la caméra ni du temps qui passe pour une
/// orbite figée dans son plan — seuls le zoom (`dist`) et l'orbite elle-même
/// (`perimetre`) influent sur la taille d'un tiret. Un glisser souris qui ne
/// change que le lacet, à distance égale, rend donc exactement le même
/// cycle, et par suite exactement les mêmes tirets en monde
/// ([`segments_motif_monde`]) — voir le test `la_phase_est_invariante_a_la_camera`.
///
/// ⚠️ **Quantifiée en puissance de deux, pas la valeur brute.** Sans ça,
/// `cycle_monde` varierait en continu pendant un zoom, et le motif
/// re-glisserait en continu à l'écran — le même strobe que la correction
/// supprime, mais déclenché par le zoom plutôt que par le lacet. Par paliers
/// de ×2, le motif ne se recale que rarement, et franchement (un bond net
/// plutôt qu'un fondu continu) — voir `cycle_monde_ne_bouge_que_par_paliers_de_x2`.
///
/// ⚠️ **Plancher `perimetre / 2000`, le vrai garde-fou mémoire.** Ce tracé a
/// déjà fait tomber le jeu par épuisement mémoire (une arête ÉCRAN de 10⁷ px
/// produisait 909 091 segments). Ancrer la phase en monde élimine ce risque
/// côté écran — le périmètre d'une orbite ne dépend pas du zoom — mais sans
/// plancher, une caméra collée au centre (`dist` → 0) ferait tendre le cycle
/// brut vers 0 et le nombre de cycles vers l'infini. Le plancher borne à
/// 2000 cycles par orbite, quels que soient le zoom et la distance — voir
/// `une_orbite_geante_ne_coute_que_ce_qui_se_voit`.
fn cycle_monde(dist: f32, ecran_y: f32, perimetre: f32) -> f32 {
    let brut = CYCLE / px_par_unite(ecran_y, dist);
    let quantifie = 2.0_f32.powf(brut.log2().round());
    quantifie.max(perimetre / 2000.0)
}

/// Somme des longueurs entre points consécutifs. Appelée sur une polyligne
/// déjà **fermée** (dernier point == premier), elle rend donc le périmètre
/// complet de l'orbite.
fn perimetre(points_fermes: &[Vec3]) -> f32 {
    points_fermes.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Portion **visible** d'une arête, en abscisses curvilignes `[t0, t1]` le
/// long de `dir` depuis `a` — `None` si l'arête ne touche pas l'écran.
///
/// Découpe de Liang-Barsky contre le rectangle de l'écran, élargi de
/// l'épaisseur du trait pour qu'un trait rasant le bord ne disparaisse pas.
fn portion_visible(a: Vec2, dir: Vec2, longueur: f32, ecran: Vec2) -> Option<(f32, f32)> {
    let marge = EPAISSEUR_PX;
    let (mut t0, mut t1) = (0.0_f32, longueur);
    for (p, q) in [
        (-dir.x, a.x + marge),
        (dir.x, ecran.x + marge - a.x),
        (-dir.y, a.y + marge),
        (dir.y, ecran.y + marge - a.y),
    ] {
        if p.abs() <= f32::EPSILON {
            // Arête parallèle à ce bord : dedans ou dehors, mais pas coupée.
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = t0.max(r);
        } else {
            t1 = t1.min(r);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((t0, t1))
}

/// Découpe une polyligne **monde** (déjà fermée, dernier point == premier)
/// selon [`MOTIF`] mis à l'échelle de `cycle`, et rend les segments à
/// **encrer** — en coordonnées monde, sans rien savoir de la caméra ni de
/// l'écran.
///
/// Le motif se poursuit d'un segment de polyligne au suivant (il ne repart
/// pas de zéro à chaque sommet) : sans ça, une orbite échantillonnée fin
/// donnerait un trait plein, et une orbite échantillonnée grossièrement un
/// motif à la longueur variable.
///
/// Le travail est borné par construction : le nombre de tirets est de
/// l'ordre de `perimetre(points) / cycle`, et l'appelant plafonne `cycle` à
/// `perimetre / 2000` au minimum ([`cycle_monde`]) — jamais plus de
/// quelques milliers d'itérations, quel que soit le zoom.
fn segments_motif_monde(points: &[Vec3], cycle: f32) -> Vec<(Vec3, Vec3)> {
    let mut out = Vec::new();
    if points.len() < 2 || cycle <= f32::EPSILON {
        return out;
    }
    // Fraction de MOTIF mise à l'échelle de `cycle`. `.max(f32::EPSILON)` :
    // garde défensive contre un `cycle` si minuscule que la fraction
    // arrondirait à 0 en f32, ce qui bloquerait la boucle ci-dessous
    // (`pas` resterait nul sans jamais avancer `parcouru`). En pratique
    // `cycle_monde` ne produit jamais une valeur aussi petite (plancher
    // `perimetre / 2000`), mais cette fonction reste appelable directement
    // par les tests avec un `cycle` arbitraire.
    let echelle = |i: usize| (MOTIF[i].0 / CYCLE * cycle).max(f32::EPSILON);
    let mut phase = 0usize;
    let mut reste = echelle(0);
    for paire in points.windows(2) {
        let (a, b) = (paire[0], paire[1]);
        let longueur = a.distance(b);
        // Deux points confondus (une orbite peut en porter) : on saute, sinon
        // `dir` vaudrait 0/0 = NaN.
        if longueur <= f32::EPSILON {
            continue;
        }
        let dir = (b - a) / longueur;
        let mut parcouru = 0.0;
        while parcouru < longueur {
            let pas = reste.min(longueur - parcouru);
            if MOTIF[phase].1 {
                out.push((a + dir * parcouru, a + dir * (parcouru + pas)));
            }
            parcouru += pas;
            reste -= pas;
            if reste <= f32::EPSILON {
                phase = (phase + 1) % MOTIF.len();
                reste = echelle(phase);
            }
        }
    }
    out
}

/// Décompose l'orbite monde `pts` (**relative à son parent**, non fermée) en
/// tirets à encrer, phase ancrée sur l'abscisse curviligne monde.
///
/// C'est la partie « décide » du tracé : pure, sans macroquad de dessin,
/// testable directement avec une [`CameraInfo`] construite à la main
/// (`camera::base_orbite`). `cam` ne sert qu'à mesurer `dist`, la distance
/// caméra → `centre` (le parent, pas un point de l'orbite — voir le
/// commentaire de module).
fn tirets_monde(pts: &[Vec3], centre: Vec3, cam: &CameraInfo, ecran_y: f32) -> Vec<(Vec3, Vec3)> {
    if pts.len() < 2 {
        return Vec::new();
    }
    let fermee: Vec<Vec3> = pts.iter().copied().chain(std::iter::once(pts[0])).collect();
    let dist = cam.pos.distance(centre);
    let cycle = cycle_monde(dist, ecran_y, perimetre(&fermee));
    segments_motif_monde(&fermee, cycle)
}

/// Projette et rogne à l'écran les tirets monde de [`tirets_monde`] — la
/// partie « dessine » du tracé, elle aussi pure (aucun appel macroquad de
/// rendu, seulement `camera::projeter_ecran` et [`portion_visible`], déjà
/// testables sans contexte graphique).
///
/// Un tiret dont une extrémité est derrière la caméra est purement
/// abandonné plutôt que rogné au plan proche : c'est la façon dont, ici,
/// « un point derrière la caméra coupe la polyligne ». Les tirets sont
/// courts (une fraction de [`cycle_monde`]) ; en perdre un pile à l'horizon
/// de la caméra ne se voit pas — contrairement à l'ancienne polyligne brute,
/// dont les arêtes pouvaient couvrir une bonne part de l'orbite.
fn segments_ecran(
    monde: &[(Vec3, Vec3)],
    centre: Vec3,
    cam: &CameraInfo,
    ecran: Vec2,
    aspect: f32,
) -> Vec<(Vec2, Vec2)> {
    let mut out = Vec::new();
    for &(a, b) in monde {
        let (Some(pa), Some(pb)) = (
            crate::camera::projeter_ecran(centre + a, ecran, cam, aspect),
            crate::camera::projeter_ecran(centre + b, ecran, cam, aspect),
        ) else {
            continue;
        };
        let longueur = pa.distance(pb);
        if longueur <= f32::EPSILON {
            continue;
        }
        let dir = (pb - pa) / longueur;
        if let Some((t0, t1)) = portion_visible(pa, dir, longueur, ecran) {
            out.push((pa + dir * t0, pa + dir * t1));
        }
    }
    out
}

/// Trace l'orbite de chaque **lune** de `parent`. Passe interface.
///
/// La polyligne d'orbite est relative au parent (`Astre::orbite_lune`), qui
/// bouge : [`tirets_monde`] travaille sur les coordonnées relatives (les
/// longueurs n'en dépendent pas), et [`segments_ecran`] ajoute la position
/// courante du parent avant de projeter. Une orbite est fermée, d'où le
/// retour au premier point (fait par `tirets_monde`).
pub fn dessiner_orbites_lunes(
    sys: &Systeme,
    parent: usize,
    cam: &CameraInfo,
    ecran: Vec2,
    aspect: f32,
) {
    let centre = sys.position(parent);
    for idx in sys.lunes_de(parent) {
        if sys.categorie_de(idx) != Some(Categorie::Lune) {
            continue;
        }
        let pts = sys.orbite_lune_de(idx);
        let monde = tirets_monde(pts, centre, cam, ecran.y);
        for (a, b) in segments_ecran(&monde, centre, cam, ecran, aspect) {
            draw_line(a.x, a.y, b.x, b.y, EPAISSEUR_PX, COL_LUNES);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Part encrée d'un cycle (12 + 2 sur 22).
    const ENCRE: f32 = 14.0;
    /// Écran d'essai. Les polylignes des tests de motif tiennent dedans :
    /// c'est voulu, elles mesurent le motif, pas le découpage.
    const ECRAN: Vec2 = Vec2::new(1000.0, 700.0);

    fn longueur3(segments: &[(Vec3, Vec3)]) -> f32 {
        segments.iter().map(|(a, b)| a.distance(*b)).sum()
    }

    /// Caméra orbitale construite à la main, comme `Camera::construire` mais
    /// sans le contexte graphique qu'exige `Camera::new`.
    fn cam_test(yaw: f32, pitch: f32, dist: f32, cible: Vec3) -> CameraInfo {
        let (right, up, forward) = crate::camera::base_orbite(yaw, pitch);
        CameraInfo {
            pos: cible - forward * dist,
            right,
            up,
            forward,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        }
    }

    /// Orbite circulaire dans le plan XZ, comme `Planete::en_lune` en fabrique
    /// une (256 points, `src/planete/mod.rs`).
    fn orbite_circulaire(rayon: f32, n: usize) -> Vec<Vec3> {
        (0..n)
            .map(|i| {
                let t = i as f32 / n as f32 * std::f32::consts::TAU;
                vec3(rayon * t.cos(), 0.0, rayon * t.sin())
            })
            .collect()
    }

    /// **Le motif encre la bonne fraction du trait.** C'est ce qui fait lire
    /// « trait-point » plutôt que « trait plein » ou « pointillé ».
    #[test]
    fn le_motif_encre_la_fraction_attendue() {
        // Dix cycles exactement, en ligne droite.
        let l = CYCLE * 10.0;
        let segments = segments_motif_monde(&[vec3(0.0, 0.0, 0.0), vec3(l, 0.0, 0.0)], CYCLE);
        let attendu = ENCRE * 10.0;
        assert!(
            (longueur3(&segments) - attendu).abs() < 0.01,
            "{} encré sur {l}, attendu {attendu}",
            longueur3(&segments)
        );
        // Et ce sont bien deux traits par cycle (le long, puis le point).
        assert_eq!(segments.len(), 20, "un trait et un point par cycle");
    }

    /// **Le motif ne repart pas de zéro à chaque sommet.** Une orbite
    /// échantillonnée fin sortirait sinon en trait plein : chaque segment
    /// commencerait par le trait long et n'irait jamais jusqu'au trou.
    #[test]
    fn le_motif_se_poursuit_d_un_sommet_a_l_autre() {
        let l = CYCLE * 4.0;
        let entier = segments_motif_monde(&[vec3(0.0, 0.0, 0.0), vec3(l, 0.0, 0.0)], CYCLE);
        // La même droite, mais découpée en 16 petits segments colinéaires.
        let decoupee: Vec<Vec3> =
            (0..=16).map(|i| vec3(l * i as f32 / 16.0, 0.0, 0.0)).collect();
        let morcelee = segments_motif_monde(&decoupee, CYCLE);
        assert!(
            (longueur3(&entier) - longueur3(&morcelee)).abs() < 0.01,
            "{} encré en un trait contre {} en seize",
            longueur3(&entier),
            longueur3(&morcelee)
        );
    }

    /// Le premier trait démarre **au début** de la polyligne : une orbite ne
    /// doit pas commencer par un trou, sinon deux lunes voisines paraissent
    /// décalées l'une de l'autre.
    #[test]
    fn le_motif_commence_par_de_l_encre() {
        let segments =
            segments_motif_monde(&[vec3(10.0, 10.0, 0.0), vec3(60.0, 10.0, 0.0)], CYCLE);
        assert_eq!(segments[0].0, vec3(10.0, 10.0, 0.0));
        assert!(
            (segments[0].1.x - 22.0).abs() < 0.01,
            "premier trait de 12 px : {:?}",
            segments[0]
        );
    }

    /// **Rien n'est encré hors de l'écran.** Le découpage doit vraiment
    /// couper, pas seulement border le nombre de segments.
    #[test]
    fn aucun_segment_ne_sort_de_l_ecran() {
        // Caméra qui regarde droit vers +Z (yaw = pitch = 0) : profondeur
        // constante pour tout point du plan z = 0, donc la projection y est
        // affine — pratique pour fabriquer une diagonale monde qui
        // corresponde à une diagonale écran traversant l'image de part en
        // part, en débordant très largement des deux côtés.
        let cam = cam_test(0.0, 0.0, 844.87, Vec3::ZERO);
        let l = 50_000.0;
        let monde = vec![(vec3(-l, -l, 0.0), vec3(l, l, 0.0))];
        let segments = segments_ecran(&monde, Vec3::ZERO, &cam, ECRAN, 1.0);
        assert!(!segments.is_empty(), "la diagonale traverse l'écran, elle doit s'encrer");
        let marge = EPAISSEUR_PX + 0.01;
        for (a, b) in segments {
            for p in [a, b] {
                assert!(
                    p.x >= -marge && p.x <= ECRAN.x + marge && p.y >= -marge && p.y <= ECRAN.y + marge,
                    "segment encré hors écran : {p:?}"
                );
            }
        }
    }

    /// **Une orbite énorme ne doit pas coûter un travail énorme, même vue de
    /// très (très) près.**
    ///
    /// Avant la correction, une arête ÉCRAN de 10⁷ px produisait 909 091
    /// segments (orbite de 96 arêtes, zoom ×25000) : le tracé épuisait la
    /// mémoire et le jeu tombait. La phase étant maintenant ancrée en monde,
    /// le périmètre parcouru ne dépend plus du zoom — mais une caméra
    /// collée au centre (`dist` → 0) pourrait redonner un cycle minuscule et
    /// donc un nombre de cycles énorme sans le plancher `perimetre / 2000`
    /// de `cycle_monde`. C'est ce plancher que ce test garde.
    #[test]
    fn une_orbite_geante_ne_coute_que_ce_qui_se_voit() {
        let orbite = orbite_circulaire(1.0e6, 256);
        let fermee: Vec<Vec3> = orbite.iter().copied().chain(std::iter::once(orbite[0])).collect();
        let per = perimetre(&fermee);
        // Caméra quasi collée au centre : zoom extrême.
        let cycle = cycle_monde(1e-6, ECRAN.y, per);
        let segments = segments_motif_monde(&fermee, cycle);
        // Au plus 2000 cycles (plancher perimetre/2000) × 2 tirets encrés par
        // cycle, plus au plus un tiret de marge par arête de la polyligne
        // (256 ici) : un sommet peut tomber au milieu d'un tiret et le
        // scinder en deux.
        assert!(
            segments.len() <= 2 * 2000 + 256,
            "{} segments pour un périmètre de {per} (plancher perimetre/2000 attendu)",
            segments.len()
        );
    }

    /// **`cycle_monde` ne descend jamais sous `perimetre / 2000`.** C'est le
    /// garde-fou mémoire : sans lui, une caméra collée au centre de l'orbite
    /// ferait tendre le cycle vers 0 et le nombre de tirets vers l'infini.
    #[test]
    fn cycle_monde_ne_descend_jamais_sous_perimetre_sur_2000() {
        let per = 500.0;
        let plancher = per / 2000.0;
        for dist in [1e-6, 1e-3, 1.0] {
            let c = cycle_monde(dist, ECRAN.y, per);
            assert!(c >= plancher - 1e-6, "{c} sous le plancher {plancher} à dist={dist}");
        }
    }

    /// **`cycle_monde` est soit au plancher, soit une puissance de deux
    /// exacte** — jamais une valeur brute intermédiaire.
    #[test]
    fn cycle_monde_est_une_puissance_de_deux_ou_le_plancher() {
        let per = 1000.0;
        let plancher = per / 2000.0;
        for dist in [0.001, 0.1, 1.0, 10.0, 1000.0, 1_000_000.0] {
            let c = cycle_monde(dist, ECRAN.y, per);
            if (c - plancher).abs() > plancher * 1e-6 {
                let log2 = c.log2();
                assert!(
                    (log2 - log2.round()).abs() < 1e-4,
                    "cycle {c} n'est pas une puissance de deux (dist={dist})"
                );
            }
        }
    }

    /// **`cycle_monde` ne bouge que par paliers de ×2** en balayant le zoom
    /// (`dist`) : jamais de valeur intermédiaire entre deux puissances de
    /// deux consécutives. Sans ça, `cycle_monde` varierait en continu
    /// pendant un zoom et le motif re-glisserait en continu à l'écran — le
    /// même strobe que corrige cette version, mais déclenché par le zoom.
    ///
    /// Le périmètre est choisi assez petit pour que le plancher
    /// (`perimetre / 2000`) ne joue jamais aux distances balayées (le cycle
    /// brut, lui, reste de l'ordre de 10⁻⁵ à 10³ sur ce balayage) : un saut
    /// depuis ou vers le plancher n'a aucune raison d'être un rapport ×2, ce
    /// n'est pas ce que ce test garde (c'est le rôle de
    /// `cycle_monde_ne_descend_jamais_sous_perimetre_sur_2000`).
    #[test]
    fn cycle_monde_ne_bouge_que_par_paliers_de_x2() {
        let per = 1.0e-6;
        let mut prec: Option<f32> = None;
        let mut d = 0.001_f32;
        while d < 100_000.0 {
            let c = cycle_monde(d, ECRAN.y, per);
            if let Some(p) = prec {
                if (c - p).abs() > 1e-6 {
                    let ratio = c / p;
                    assert!(
                        (ratio - 2.0).abs() < 1e-3 || (ratio - 0.5).abs() < 1e-3,
                        "saut non binaire : {p} -> {c} (ratio {ratio}) à dist={d}"
                    );
                }
            }
            prec = Some(c);
            d *= 1.01;
        }
    }

    /// **Le test qui garde la correction : la phase est invariante à la
    /// caméra.**
    ///
    /// Deux caméras à 0,005 rad de lacet d'écart — la sensibilité réelle
    /// d'UN pixel de glisser souris (`Camera::input_orbite`, facteur 0.005).
    /// Avant la correction, ce seul glisser décalait la phase de 37 % du
    /// cycle (mesuré) : c'est le strobe que corrige cette version.
    ///
    /// `tirets_monde` ne dépend de la caméra qu'au travers de `dist`
    /// (caméra → centre), qui est **exactement** le même pour les deux
    /// caméras ici (seul le lacet change, pas la distance) : les tirets
    /// décidés en monde doivent donc être identiques — on compare en
    /// paramètre MONDE (les coordonnées des tirets), jamais en pixels.
    #[test]
    fn la_phase_est_invariante_a_la_camera() {
        let orbite = orbite_circulaire(37.0, 256);
        let dist = 777.0;
        let cam_a = cam_test(0.3, 0.2, dist, Vec3::ZERO);
        let cam_b = cam_test(0.3 + 0.005, 0.2, dist, Vec3::ZERO);

        let tirets_a = tirets_monde(&orbite, Vec3::ZERO, &cam_a, ECRAN.y);
        let tirets_b = tirets_monde(&orbite, Vec3::ZERO, &cam_b, ECRAN.y);

        assert_eq!(
            tirets_a.len(),
            tirets_b.len(),
            "nombre de tirets différent entre deux caméras à 0,005 rad d'écart"
        );
        for ((a0, a1), (b0, b1)) in tirets_a.iter().zip(tirets_b.iter()) {
            assert!(a0.distance(*b0) < 1e-3, "tiret décalé en monde (début) : {a0:?} vs {b0:?}");
            assert!(a1.distance(*b1) < 1e-3, "tiret décalé en monde (fin) : {a1:?} vs {b1:?}");
        }
    }
}
