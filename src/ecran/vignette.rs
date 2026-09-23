//! **Vignette d'astre** : le portrait qui coiffe le panneau de droite.
//!
//! Le schéma d'interface montre la planète cliquée, rendue, en haut de l'encart
//! (`docs/conception/interface.md` §1.2, ⓔ). C'était la dette **D-INT-3** — un
//! disque de couleur unie en tenait lieu.
//!
//! # Pourquoi une cible de rendu, et non un viewport
//!
//! La galerie dessine ses planètes dans des **viewports** découpés dans l'écran
//! (`ecran/galerie.rs`), et c'est le bon outil là-bas : elle ne rend qu'elles.
//! Ici, la scène du système est **déjà dessinée**, profondeur comprise. Un
//! viewport poserait la vignette dans ce tampon-là, et l'astre se ferait
//! découper par ce qui traîne devant lui dans le monde — un anneau, une lune,
//! parfois rien du tout selon l'angle de caméra.
//!
//! Une cible séparée a son propre tampon de profondeur, effacé à chaque
//! rendu : la vignette ne peut donc pas se faire mordre. Elle se recolle
//! ensuite comme une simple texture.
//!
//! # Une capture, gardée en cache — pas un rendu à chaque frame
//!
//! C'est une vue simplifiée, un portrait, pas une mise à jour temps réel : la
//! demande était de **capturer une fois** l'astre, trois quarts éclairés, et
//! de garder cette image tant qu'elle reste valable — plutôt que de refaire
//! le rendu 3D complet à chaque frame tant que le panneau reste ouvert
//! (l'ancien choix, documenté ici même par le passé, et jugé trop coûteux à
//! l'usage).
//!
//! Le cache suit le patron de [`super::icone::IconesAstres`] : une texture
//! par astre déjà capturé, une `RenderTarget` **scratch** réutilisée d'un
//! rendu à l'autre plutôt que recréée (voir `STATE.md`, chantier I.6 : une
//! `RenderTarget` dont seule la texture survit se fait supprimer par le
//! moteur à la frame suivante), et la même garde contre un rendu vide.
//!
//! Contrairement à `icone.rs`, une seule entrée est demandée par appel (un
//! seul panneau peut être ouvert), donc pas de compteur d'échecs partagé à
//! parcourir ni de peuplement progressif à orchestrer sur plusieurs
//! astres — juste : déjà en cache ? sinon, prêt à être capturé ? sinon,
//! rien à dessiner cette frame, on retentera à la suivante.
//!
//! ## Invalidation
//!
//! Vidé exactement là où [`super::icone::IconesAstres`] l'est déjà
//! (`ecran/skymap_v2.rs` : nouvelle graine `G`, hot-reload `R`, changement de
//! preset) — même raison : le cache est indexé par position dans
//! `sys.astres`, qui ne désigne plus le même corps une fois le système
//! reconstruit.
//!
//! Une capture ne se garde que si l'astre est **prêt**
//! ([`Systeme::rendu_pret_de`]) : une tellurique dont le terrain n'a pas fini
//! de générer en tâche de fond retombe sur un repli le temps de sa
//! génération, et capturer ce repli le figerait pour de bon — c'est
//! exactement le bug déjà rencontré sur les icônes du sélecteur (`STATE.md`,
//! I.6 bis, « l'icône de la Terre figeait un repli de terrain pas encore
//! généré »). Tant que l'astre n'est pas prêt, rien n'est mis en cache et le
//! prochain appel retente.
//!
//! ## Ce que la capture fige
//!
//! Compromis assumé, à l'usage de l'utilisateur : la rotation propre de
//! l'astre, sa position orbitale et son éclairage au moment de la capture
//! sont figés dans l'image mise en cache. Le portrait ne suit donc plus le
//! jour/nuit ni le déplacement de la planète pendant que le panneau reste
//! ouvert — vu que c'est une vue simplifiée, l'utilisateur a jugé ce
//! figement acceptable face au coût d'un rendu 3D complet chaque frame.
//!
//! # Le corps n'est pas déplacé
//!
//! On approche la **caméra** de l'astre là où il est, plutôt que de le poser à
//! l'origine : sa position sert à son propre dessin (terminateur, éclairage
//! multi-étoiles, anneaux orientés). Le bouger pour le portrait le montrerait
//! éclairé autrement que dans la vue.
//!
//! # Cadrage « trois quarts éclairés »
//!
//! La fraction éclairée visible d'une sphère, vue depuis une direction à
//! l'angle `θ` de la direction astre→étoile, vaut `(1 + cos θ) / 2`
//! (`θ = 0` : plein soleil, disque entier éclairé ; `θ = 180°` : contre-jour,
//! disque noir). Pour trois quarts éclairés, `cos θ = 0,5`, soit `θ = 60°` —
//! voir [`direction_camera_trois_quarts`], qui place la caméra à exactement
//! cet angle, par rotation déterministe autour d'un axe perpendiculaire à la
//! lumière (pas de placement « du côté éclairé » à l'aveugle, ni de choix
//! aléatoire).

use crate::astre::CameraInfo;
use crate::systeme::Systeme;
use macroquad::prelude::*;
use std::collections::HashMap;

/// Côté de la cible, en pixels. Fixe et généreux : la vignette est ensuite mise
/// à l'échelle du panneau, et repartir d'une cible plus fine que l'affichage
/// donnerait une image molle sur un grand écran.
pub const COTE: u32 = 192;

/// Recul de la caméra, en rayons de l'astre. 3 R cadre le corps en laissant un
/// peu de vide : plus près il déborde, plus loin il se perd dans la case.
const RECUL: f32 = 3.0;
/// Recul pour un corps à anneau, en rayons **d'anneau** : sinon l'anneau sort
/// du cadre et la planète a l'air posée sur deux traits.
const RECUL_ANNEAU: f32 = 2.6;

/// Angle, en degrés, entre la direction de la caméra et la direction de la
/// lumière — voir la note de module « Cadrage trois quarts éclairés ».
const ANGLE_TROIS_QUARTS_DEG: f32 = 60.0;

/// Fond de la cible — opaque (pas transparent comme `icone.rs`, qui en a
/// besoin pour tracer un contour) : la vignette se dessine seule dans son
/// encart, sans silhouette à détacher d'un fond de liste. Sert aussi de
/// référence à [`vide`] : un rendu qui n'a peint qu'un fond dans cette
/// teinte n'a en réalité rien capturé.
const FOND: Color = Color::new(0.01, 0.02, 0.05, 1.0);

/// Au-delà de ce nombre d'essais vides pour le **même** astre, on abandonne
/// et rien ne se dessine — même raison que `icone::ESSAIS_MAX` : un
/// second changement de caméra 3D vers une cible hors écran, dans la même
/// frame (ici : un rendu d'icône du sélecteur suivi du rendu de la
/// vignette), ressort vide, mesuré à l'écran. Un plafond fini évite qu'un
/// astre qui échouerait vraiment pour de bon bloque le panneau indéfiniment.
const ESSAIS_MAX: u16 = 240;

/// Direction unitaire astre→caméra pour un cadrage trois quarts éclairés :
/// l'angle avec la direction astre→étoile vaut exactement
/// [`ANGLE_TROIS_QUARTS_DEG`]. **Fonction pure**, sans aucun contexte
/// graphique (pas de caméra, pas de cible de rendu) — c'est la seule partie
/// du cadrage qui se teste sans macroquad en jeu.
///
/// La rotation se fait autour d'un axe perpendiculaire à la lumière choisi
/// de façon déterministe : la projection de l'axe monde `Y` sur le plan
/// perpendiculaire à la lumière (même idée que `icone::cadre_camera` pour
/// l'anneau, qui projette la normale de l'anneau sur le plan de l'écran) —
/// avec un secours sur `Z` dans le seul cas où cette projection s'annule
/// (lumière déjà verticale). Ce choix penche la caméra vers le "haut" du
/// monde autant que le permet la contrainte des 60°, plutôt qu'un axe
/// perpendiculaire choisi au hasard qui pourrait présenter l'astre de
/// travers.
///
/// Cas dégénéré : si `position_etoile` est confondue avec `position_astre`
/// (direction de lumière indéfinie), replie sur `Vec3::Z` comme direction de
/// lumière de référence — déterministe, jamais de vecteur nul en sortie.
pub fn direction_camera_trois_quarts(position_astre: Vec3, position_etoile: Vec3) -> Vec3 {
    let vers_lumiere = (position_etoile - position_astre).normalize_or_zero();
    let vers_lumiere = if vers_lumiere == Vec3::ZERO { Vec3::Z } else { vers_lumiere };

    // Axe perpendiculaire à la lumière, projection de Y — voir la note de
    // fonction. Secours en Z si cette projection s'annule (lumière verticale) :
    // Z ne peut pas être, lui aussi, parallèle à la lumière dans ce cas
    // précis (Y et Z sont orthogonaux entre eux).
    let perp = {
        let p = Vec3::Y - Vec3::Y.dot(vers_lumiere) * vers_lumiere;
        if p.length_squared() < 1e-8 {
            (Vec3::Z - Vec3::Z.dot(vers_lumiere) * vers_lumiere).normalize_or_zero()
        } else {
            p.normalize()
        }
    };

    let theta = ANGLE_TROIS_QUARTS_DEG.to_radians();
    (vers_lumiere * theta.cos() + perp * theta.sin()).normalize_or_zero()
}

/// La texture ne montre-t-elle que le fond, sans qu'aucun pixel n'y déroge ?
/// Échantillonnée plutôt que lue en entier, comme `icone::vide` — un pixel
/// par bloc de 4 suffit à trancher pour une cible de cette taille.
///
/// Sur l'écart au **fond connu** (`FOND`), pas sur l'alpha : la cible de la
/// vignette est opaque (voir la note de module), l'alpha ne dit donc rien
/// d'un rendu manqué ici.
fn vide(img: &Image) -> bool {
    let proche_du_fond = |c: Color| {
        (c.r - FOND.r).abs() < 0.02 && (c.g - FOND.g).abs() < 0.02 && (c.b - FOND.b).abs() < 0.02
    };
    for y in (0..img.height).step_by(4) {
        for x in (0..img.width).step_by(4) {
            if !proche_du_fond(img.get_pixel(x as u32, y as u32)) {
                return false;
            }
        }
    }
    true
}

/// Cache de vignettes, une texture par astre déjà capturé — voir la note de
/// module.
pub struct Vignette {
    cache: HashMap<usize, Texture2D>,
    /// Compte des rendus ressortis vides, par astre — voir [`ESSAIS_MAX`].
    echecs: HashMap<usize, u16>,
    /// Cible de rendu, réutilisée d'une capture à l'autre.
    scratch: Option<RenderTarget>,
}

impl Vignette {
    pub fn new() -> Self {
        Self { cache: HashMap::new(), echecs: HashMap::new(), scratch: None }
    }

    /// Dessine la vignette de l'astre `idx` dans `zone`, en la capturant
    /// d'abord si elle n'est pas encore en cache. Ne fait rien si l'astre est
    /// invalide, pas encore prêt, ou n'a rien à montrer cette frame (capture
    /// en attente de retentative — voir [`Self::texture`]).
    pub fn dessiner(&mut self, sys: &mut Systeme, idx: usize, zone: Rect) {
        let Some(tex) = self.texture(sys, idx) else { return };

        // La texture d'une cible de rendu est **retournée** verticalement par
        // rapport à l'écran (origine GL en bas à gauche) : sans `flip_y`, la
        // planète apparaîtrait la tête en bas, ce qui ne se voit que sur les
        // corps à anneau incliné.
        draw_texture_ex(
            tex,
            zone.x,
            zone.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(zone.w, zone.h)),
                flip_y: true,
                ..Default::default()
            },
        );
    }

    /// Texture en cache pour `idx`, capturée à la volée si besoin (au plus
    /// une capture par appel). `None` si rien n'est encore disponible : astre
    /// invalide, terrain pas prêt ([`Systeme::rendu_pret_de`]), ou nombre
    /// d'essais vides épuisé ([`ESSAIS_MAX`]).
    fn texture(&mut self, sys: &mut Systeme, idx: usize) -> Option<&Texture2D> {
        if !self.cache.contains_key(&idx)
            && self.echecs.get(&idx).copied().unwrap_or(0) < ESSAIS_MAX
            && sys.rendu_pret_de(idx)
        {
            if let Some(img) = self.rendre(sys, idx) {
                if vide(&img) {
                    *self.echecs.entry(idx).or_insert(0) += 1;
                } else {
                    self.cache.insert(idx, Texture2D::from_image(&img));
                }
            }
        }
        self.cache.get(&idx)
    }

    /// Capture l'astre `idx` dans la cible scratch, et en relit les pixels —
    /// l'appelant décide s'ils sont vides et bâtit la texture mise en cache.
    fn rendre(&mut self, sys: &mut Systeme, idx: usize) -> Option<Image> {
        let rayon = sys.rayon_de(idx)?.max(1e-4);

        let cible = self.scratch.get_or_insert_with(|| {
            // `depth: true` — sans tampon de profondeur, les faces arrière d'une
            // sphère passent devant les faces avant.
            let rt = render_target_ex(COTE, COTE, RenderTargetParams { sample_count: 1, depth: true });
            rt.texture.set_filter(FilterMode::Linear);
            rt
        });

        let centre = sys.position(idx);
        let etendue = sys.rayon_visuel_de(idx).unwrap_or(rayon).max(rayon);
        let recul = if etendue > rayon * 1.05 { etendue * RECUL_ANNEAU } else { rayon * RECUL };

        // Cadrage trois quarts éclairés — voir la note de module et
        // `direction_camera_trois_quarts`.
        let direction = direction_camera_trois_quarts(centre, sys.position_lumiere());
        let pos = centre + direction * recul;

        let cam3d = Camera3D {
            position: pos,
            target: centre,
            up: Vec3::Y,
            fovy: 45.0_f32.to_radians(),
            aspect: Some(1.0),
            render_target: Some(cible.clone()),
            // Voir `icone.rs::rendre` : le plan proche par défaut (0.01) est
            // pensé pour l'échelle du monde, pas pour ce cadrage à `recul` —
            // un nain comme Pluton a un `recul` bien en deçà, et sa sphère
            // entière tomberait devant le plan proche sans ce correctif.
            z_near: (recul * 0.1).max(1e-6),
            ..Default::default()
        };
        set_camera(&cam3d);
        clear_background(FOND);

        let forward = (centre - pos).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let cam = CameraInfo {
            pos,
            right,
            up,
            forward,
            light_pos: Vec3::ZERO,
            light_color: Vec3::ONE,
            lights_pos: [Vec3::ZERO; 4],
            lights_color: [Vec3::ZERO; 4],
        };
        sys.dessiner_astre(idx, &cam);
        set_default_camera();

        Some(cible.texture.get_texture_data())
    }
}

impl Default for Vignette {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
// `la_cible_est_carree_et_assez_fine` affirme quelque chose sur une
// CONSTANTE, et clippy y voit une condition toujours vraie. Elle l'est —
// aujourd'hui. Le jour où quelqu'un abaisse la constante, l'assertion devient
// fausse et le test tombe : c'est précisément son rôle, garder la valeur ET
// dire pourquoi elle compte.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;

    // La cible est carrée et l'aspect vaut 1 : une cible rectangulaire
    // écraserait la planète en ovale, ce qui ne se remarque pas tout de suite.
    #[test]
    fn la_cible_est_carree_et_assez_fine() {
        assert!(COTE >= 128, "une vignette de {COTE} px serait molle sur un grand écran");
    }

    // Un corps à anneau doit être vu de **plus loin**, sinon l'anneau sort du
    // cadre — c'est la raison d'être des deux constantes de recul.
    #[test]
    fn un_corps_a_anneau_se_cadre_de_plus_loin() {
        let rayon = 1.0_f32;
        let sans = rayon * RECUL;
        // Un anneau de Saturne s'étend à ~2,3 rayons.
        let avec = (rayon * 2.3) * RECUL_ANNEAU;
        assert!(avec > sans, "l'anneau tiendrait hors du cadre : {avec} contre {sans}");
    }

    /// Angle, en degrés, entre `direction_camera_trois_quarts(astre, etoile)`
    /// et la direction astre→étoile — sert à toutes les vérifications
    /// « trois quarts éclairés » ci-dessous.
    fn angle_avec_lumiere_deg(astre: Vec3, etoile: Vec3) -> f32 {
        let direction = direction_camera_trois_quarts(astre, etoile);
        let vers_lumiere = (etoile - astre).normalize_or_zero();
        let vers_lumiere = if vers_lumiere == Vec3::ZERO { Vec3::Z } else { vers_lumiere };
        direction.dot(vers_lumiere).clamp(-1.0, 1.0).acos().to_degrees()
    }

    // **Le point de la demande** : quelle que soit la géométrie, la caméra se
    // place à exactement 60° de la direction de la lumière — trois quarts de
    // la sphère éclairés visibles, (1 + cos 60°) / 2 = 0,75.
    #[test]
    fn la_camera_se_place_a_60_degres_de_la_lumiere() {
        let cas: [(Vec3, Vec3); 5] = [
            (Vec3::ZERO, vec3(10.0, 0.0, 0.0)),
            (vec3(5.0, 0.0, 0.0), Vec3::ZERO),
            (vec3(3.0, -2.0, 7.0), vec3(-40.0, 12.0, -8.0)),
            (Vec3::ZERO, Vec3::Y * 100.0), // lumière verticale : bascule sur le secours Z
            (vec3(1.0, 2.0, 3.0), vec3(1.0, 2.0, 3.0)), // étoile confondue avec l'astre
        ];
        for (astre, etoile) in cas {
            let angle = angle_avec_lumiere_deg(astre, etoile);
            assert!(
                (angle - ANGLE_TROIS_QUARTS_DEG).abs() < 1e-2,
                "astre={astre:?} etoile={etoile:?} : angle {angle}° au lieu de {ANGLE_TROIS_QUARTS_DEG}°"
            );
        }
    }

    // **Toujours un vecteur unitaire** — sinon `recul` ne cadrerait plus le
    // corps à la bonne distance, et la garde du plan proche ne tiendrait plus.
    #[test]
    fn la_direction_est_toujours_unitaire() {
        let cas: [(Vec3, Vec3); 3] = [
            (Vec3::ZERO, vec3(10.0, 0.0, 0.0)),
            (vec3(3.0, -2.0, 7.0), vec3(-40.0, 12.0, -8.0)),
            (Vec3::ZERO, Vec3::ZERO),
        ];
        for (astre, etoile) in cas {
            let d = direction_camera_trois_quarts(astre, etoile);
            assert!((d.length() - 1.0).abs() < 1e-4, "direction non unitaire pour {astre:?}/{etoile:?} : {d:?}");
        }
    }

    // **Astre et étoile confondus** (direction de lumière indéfinie) : replie
    // sur une direction déterministe, jamais sur un vecteur nul qui rendrait
    // la caméra confondue avec sa cible.
    #[test]
    fn astre_et_etoile_confondus_ne_donne_jamais_une_direction_nulle() {
        let d = direction_camera_trois_quarts(vec3(2.0, 2.0, 2.0), vec3(2.0, 2.0, 2.0));
        assert!(d.length() > 1e-4, "direction nulle pour astre/étoile confondus");
        // Déterministe : rejouer le même cas donne exactement le même résultat.
        let d2 = direction_camera_trois_quarts(vec3(2.0, 2.0, 2.0), vec3(2.0, 2.0, 2.0));
        assert_eq!(d, d2);
    }
}
