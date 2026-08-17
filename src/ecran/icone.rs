//! **Icônes d'astre**, pour le sélecteur de la vue système : chaque planète
//! s'y reconnaît à son vrai visage — pas une pastille de couleur unie — mais
//! en miniature, et à un coût qu'une colonne de plusieurs dizaines d'entrées
//! peut se permettre.
//!
//! # Pourquoi un cache, à la différence de la vignette
//!
//! [`super::vignette::Vignette`] rend **un seul** astre, celui du panneau
//! ouvert, et le refait chaque frame — un choix assumé là-bas (une seule
//! cible, aucune invalidation à guetter). Ici, chaque ligne du sélecteur a la
//! sienne : les refaire toutes, chaque frame, coûterait un rendu 3D complet
//! par astre et par image. Comme l'apparence d'un astre ne change pas d'une
//! frame à l'autre (seule sa position orbitale bouge, pas son visage),
//! l'icône se rend **une fois** et se garde en texture.
//!
//! Le cache vit avec l'écran, pas globalement : changer de système
//! reconstruit une `Skymap`, donc un nouveau cache — rien à invalider à la
//! main.
//!
//! # Fond transparent, contour d'un pixel
//!
//! La cible se lit **une fois rendue**, en pixels (`Texture2D::get_texture_data`),
//! pour deux raisons à la fois : détecter un rendu vide ([`vide`]) et tracer
//! un contour d'un pixel autour de la silhouette ([`tracer_contour`]), pour
//! qu'elle se détache du fond de ligne quel qu'il soit (au repos ou survolé)
//! plutôt que de porter son propre carré sombre. Le fond s'efface donc en
//! **transparent** (alpha 0), pas dans une couleur opaque — la silhouette
//! peinte par-dessus ressort à l'alpha plein, sans anti-crénelage
//! (`sample_count: 1`), ce qui donne une frontière nette.
//!
//! Le résultat de la lecture sert aussi de source à la texture mise en
//! cache : elle est reconstruite depuis les pixels traités
//! (`Texture2D::from_image`), une texture **indépendante** de la cible de
//! rendu qui a servi à la produire. Les cibles, elles, sont réutilisées d'un
//! rendu à l'autre (comme [`super::vignette::Vignette`]) plutôt que
//! recréées.
//!
//! # Un anneau déborde, il ne se comprime pas
//!
//! Un astre à anneau (Saturne, typiquement) se cadre comme n'importe quel
//! autre : `recul = rayon_du_corps × RECUL`, **jamais** ajusté pour faire
//! tenir l'anneau — la planète doit occuper la même place à l'écran que
//! toutes les autres, anneau ou pas.
//!
//! Mais à `recul` et champ de vision fixes, un anneau plus large que le
//! champ serait simplement **coupé**, invisible au-delà du bord de la cible
//! — ni comprimé, ni débordant, juste absent. Pour qu'il déborde
//! *visiblement* du carré de l'icône plutôt que d'être invisible ou
//! comprimé dedans, le champ de vision s'élargit ([`FOVY_GAZEUSE_DEG`]) pour
//! le capter, et la **résolution** de la cible grandit dans le même rapport
//! ([`cote_gazeuse`]) — la planète occupe alors le même nombre de pixels
//! dans les deux cas, seul l'anneau, capté par le surplus de champ, ajoute
//! des pixels. Au dessin ([`echelle_dessin`]), cette texture plus grande se
//! dessine à une échelle proportionnellement plus grande elle aussi, plutôt
//! que compressée dans la case standard — c'est ce qui fait déborder
//! l'anneau à l'écran.

use crate::astre::CameraInfo;
use crate::systeme::Systeme;
use macroquad::prelude::*;
use std::collections::HashMap;

/// Côté de la cible de rendu standard, en pixels. Petit — c'est une icône de
/// sélecteur, pas un portrait de panneau — mais pas trop fin non plus : la
/// mise à l'échelle du sélecteur (voir `selecteur::ECHELLE_LIGNE`) l'agrandit
/// à l'affichage, et une cible trop fine y donnerait une image molle.
const COTE: u32 = 48;

/// Champ de vision standard, en degrés — même valeur que
/// [`super::vignette::Vignette`].
const FOVY_DEG: f32 = 45.0;

/// Champ de vision élargi, pour un astre à anneau — voir la note de module
/// « Un anneau déborde, il ne se comprime pas ».
///
/// Cadré sur l'anneau de **Saturne** (2,3 rayons, avec une petite marge),
/// pas sur le maximum théorique du générateur aléatoire (jusqu'à 2,5) : à
/// 90°, ce dernier avait la préséance et l'icône entière (donc son
/// débordement) était ~20 % plus grosse qu'il ne fallait pour Saturne, la
/// seule gazeuse à anneau réellement en jeu dans le préréglage solaire — et
/// le signalement (« l'anneau est trop gros ») la visait précisément.
/// Conséquence assumée : un anneau généré au maximum théorique (2,5) perd
/// un peu de son bord externe. `un_anneau_plus_large_que_saturne_deborde_du_cadre`
/// documente et mesure cette perte plutôt que de la laisser passer inaperçue.
const FOVY_GAZEUSE_DEG: f32 = 76.0;

/// Recul de la caméra, en rayons de l'astre — mêmes valeurs que
/// [`super::vignette::Vignette`] : c'est le même genre de portrait, en plus
/// petit, et il n'y a pas de raison qu'il se cadre autrement. **Fixe pour
/// tout astre**, anneau ou pas — voir la note de module.
const RECUL: f32 = 3.0;
const ELEVATION: f32 = 0.22;

/// Élévation de la caméra pour un astre à anneau — **nulle** : vue
/// exactement de profil, dans le plan même de l'anneau. Voir
/// [`cadre_camera`].
///
/// Historique : demandée à 15°, puis réduite de 30 % (10,5°) — même
/// l'ellipse mise à l'horizontale (`cadre_camera` aligne son grand axe),
/// les deux ouvraient encore assez l'anneau pour qu'il paraisse « penché
/// vers l'avant ». À 0°, `cadre_camera` fait de `haut` la normale de
/// l'anneau elle-même et de `direction` un vecteur pris **dans** son plan :
/// l'anneau se réduit à un trait horizontal, sans ouverture du tout.
///
/// Uniforme pour tout astre à anneau, **pas un cas particulier pour
/// Saturne** : elle est la seule gazeuse annelée du préréglage solaire
/// aujourd'hui, mais l'angle doit rester juste pour toute autre qui
/// s'ajouterait, sans retouche par planète.
fn elevation_anneau_rad() -> f32 {
    0.0
}

/// Fond de la cible : transparent, pas une couleur — voir la note de module
/// « Fond transparent, contour d'un pixel ».
const TRANSPARENT: Color = Color::new(0.0, 0.0, 0.0, 0.0);

/// Couleur du contour d'un pixel tracé autour de la silhouette. Sombre :
/// une planète claire (Uranus, Pluton) s'y détache mieux que sur un contour
/// blanc, et ça reste net aussi bien au repos (fond nuit) que survolé (fond
/// cyan translucide).
const CONTOUR: Color = BLACK;

/// Au-delà de ce nombre d'essais vides pour le **même** astre, on abandonne.
/// Large : les tout premiers astres rendus, juste après le chargement d'un
/// système, peuvent ressortir vides plusieurs frames de suite — pas un
/// échec définitif, juste du contenu (terrain, etc.) encore en cours de
/// génération ailleurs dans le jeu. Sans un plafond fini, un astre qui
/// échouerait vraiment pour de bon bloquerait quand même le peuplement de
/// **tous** ceux qui le suivent dans la colonne — `completer` ne rend
/// jamais qu'un seul astre à la fois, et retente toujours le premier
/// manquant en premier.
const ESSAIS_MAX: u16 = 240;

/// Ce qu'il faut pour rendre un astre dans sa cible : distance de caméra,
/// plan proche, et dimensions de la cible elle-même.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cadrage {
    recul: f32,
    z_near: f32,
    cote: u32,
    fovy_deg: f32,
}

/// Plan proche de la caméra, pour un recul `recul`.
///
/// Le plan proche par défaut de macroquad (0,01) est pensé pour l'échelle du
/// monde, pas pour ce cadrage à quelques rayons du corps : à l'échelle vraie
/// du système (`genese::presets`, `ECHELLE_CORPS`), un nain comme Pluton a un
/// `recul` bien en deçà de 0,01 — la sphère entière tombait alors devant le
/// plan proche et rien ne se dessinait, quel que soit le nombre de
/// tentatives. Mesuré : à `ECHELLE_CORPS` courant, Pluton (0,1868 rayon
/// terrestre) a un `recul` d'environ 0,0057, near et far face toutes deux
/// sous 0,01 — la sphère entière était hors champ. Proportionnel à `recul`,
/// le plan proche renvoyé ici reste toujours largement plus près que le
/// corps, quelle que soit son échelle.
fn z_near_pour(recul: f32) -> f32 {
    (recul * 0.1).max(1e-6)
}

/// Côté de la cible pour un astre à anneau — plus grand que [`COTE`], dans
/// le **même rapport** que [`FOVY_GAZEUSE_DEG`] élargit le champ par rapport
/// à [`FOVY_DEG`], pour qu'un corps cadré au même `recul` occupe le même
/// nombre de pixels dans les deux cas (voir la note de module et le test
/// `une_planete_a_anneau_garde_la_meme_taille_quune_sans`). Dérivé du
/// rapport des tangentes des deux demi-champs, pas mesuré à l'œil : recalculé
/// automatiquement si `FOVY_DEG`/`FOVY_GAZEUSE_DEG` changent.
fn cote_gazeuse() -> u32 {
    let theta_standard = (FOVY_DEG / 2.0).to_radians();
    let theta_gazeuse = (FOVY_GAZEUSE_DEG / 2.0).to_radians();
    (COTE as f32 * theta_gazeuse.tan() / theta_standard.tan()).round() as u32
}

/// Cadrage pour un astre de rayon `rayon` et d'étendue visuelle `etendue`
/// (≥ `rayon` ; > `rayon` pour un corps à anneau — voir
/// [`crate::astre::Astre::etendue_visuelle`]).
///
/// **Extrait pour être testable sans contexte graphique** : c'est la seule
/// partie de [`IconesAstres::rendre`] qui ne touche ni caméra ni astre réel.
fn cadrage(rayon: f32, etendue: f32) -> Cadrage {
    let recul = rayon * RECUL;
    let z_near = z_near_pour(recul);
    if etendue > rayon * 1.05 {
        Cadrage { recul, z_near, cote: cote_gazeuse(), fovy_deg: FOVY_GAZEUSE_DEG }
    } else {
        Cadrage { recul, z_near, cote: COTE, fovy_deg: FOVY_DEG }
    }
}

/// Direction horizontale d'approche de la caméra, et élévation, pour un
/// astre de normale d'anneau `normale_anneau` (`None` s'il n'en a pas) —
/// dans ce dernier cas, vue de son côté éclairé (`vers_lumiere`), comme
/// avant.
///
/// Renvoie `(direction, haut)` : `direction`, une fois multipliée par
/// `recul`, donne le décalage caméra depuis le centre de l'astre ; `haut`
/// est le vecteur « haut » de la caméra (`Camera3D::up`).
///
/// **Avec un anneau**, les deux se choisissent à partir de son inclinaison
/// propre (voir [`Astre::normale_anneau`]), pas de la lumière (arbitraire,
/// dictée par la position orbitale du moment) : l'approche se fait depuis un
/// vecteur `e1` du plan de l'anneau, relevé vers sa normale `n` de l'angle
/// d'élévation choisi — et `haut` est la projection de `n` sur le plan de
/// l'écran, pour que le **grand axe de l'ellipse tombe exactement à
/// l'horizontale**, quelle que soit l'inclinaison réelle de l'anneau. Sans
/// ce second réglage, l'ellipse pouvait apparaître à un angle quelconque
/// (le haut de la caméra restant l'axe Y du monde, sans rapport avec le
/// plan de l'anneau) — demandé : un anneau horizontal à l'écran, pas
/// penché.
///
/// [`Astre::normale_anneau`]: crate::astre::Astre::normale_anneau
fn cadre_camera(vers_lumiere: Vec3, normale_anneau: Option<Vec3>) -> (Vec3, Vec3) {
    match normale_anneau {
        Some(n) => {
            let n = n.normalize_or_zero();
            let n = if n == Vec3::ZERO { Vec3::Y } else { n };
            // Un vecteur du plan de l'anneau (perpendiculaire à sa normale) —
            // Z de secours si `n` est déjà (quasi) vertical, seul cas où
            // Y × n s'annule.
            let e1 = {
                let c = Vec3::Y.cross(n);
                if c.length_squared() < 1e-8 { Vec3::Z.cross(n).normalize_or_zero() } else { c.normalize() }
            };
            // Le côté éclairé quand les deux approches restent également
            // valables (l'anneau reste horizontal dans les deux cas) :
            // sinon l'astre lui-même pourrait se présenter dans l'ombre.
            let e1 = if e1.dot(vers_lumiere) < 0.0 { -e1 } else { e1 };
            let a = elevation_anneau_rad();
            let direction = e1 * a.cos() + n * a.sin();
            let haut = (n * a.cos() - e1 * a.sin()).normalize_or_zero();
            (direction, haut)
        }
        None => {
            let azimut = if vers_lumiere == Vec3::ZERO { Vec3::Z } else { vers_lumiere };
            (azimut + Vec3::Y * ELEVATION, Vec3::Y)
        }
    }
}

/// Échelle à laquelle dessiner une icône, connaissant la largeur (en pixels)
/// de sa texture native — voir la note de module « Un anneau déborde, il ne
/// se comprime pas ». 1 pour une cible standard ; davantage pour une cible
/// de gazeuse, plus grande. Le dessin doit multiplier sa taille cible par
/// cette échelle : sinon une cible plus grande (plus de champ capté, pour
/// loger l'anneau) se retrouve compressée dans la même case, annulant tout
/// le débordement voulu.
pub fn echelle_dessin(largeur_texture: f32) -> f32 {
    largeur_texture / COTE as f32
}

/// La texture ne montre-t-elle que du transparent, sans qu'aucun pixel n'y
/// déroge ? Échantillonnée plutôt que lue en entier — un pixel par bloc de 4
/// suffit à trancher, et une icône de cette taille n'a de toute façon pas
/// besoin d'un test pixel par pixel.
///
/// Sur l'**alpha**, pas sur une distance de couleur au fond : l'ancienne
/// version comparait chaque pixel à un fond opaque, et un astre sombre à
/// faible contraste (Mercure, Pluton) pouvait tomber sous le seuil sans
/// qu'il n'y ait rien d'anormal à son rendu — un faux « vide ». Un fond
/// transparent n'a pas cette ambiguïté : soit un pixel a été peint (alpha
/// plein), soit non (alpha nul), quelle que soit la couleur du corps.
fn vide(img: &Image) -> bool {
    for y in (0..img.height).step_by(4) {
        for x in (0..img.width).step_by(4) {
            if img.get_pixel(x as u32, y as u32).a > 0.5 {
                return false;
            }
        }
    }
    true
}

/// Trace un contour d'**un pixel** de `couleur` autour de la silhouette
/// opaque de `img` : tout pixel transparent qui touche (8-connexité) un
/// pixel opaque devient `couleur`, à alpha plein.
///
/// Fonctionne sur l'alpha déjà peint par [`IconesAstres::rendre`] (fond
/// transparent, astre opaque) — voir la note de module. Les pixels déjà
/// opaques ne bougent pas : seul le halo d'un pixel autour de la silhouette
/// change, la planète elle-même est intacte.
///
/// Une passe en deux temps (lire l'original, écrire une liste, puis
/// appliquer) : modifier `img` pendant qu'on la lit ferait grossir le
/// contour de proche en proche au lieu de rester à un seul pixel.
fn tracer_contour(img: &mut Image, couleur: Color) {
    let (w, h) = (img.width as i32, img.height as i32);
    let opaque = |img: &Image, x: i32, y: i32| -> bool {
        x >= 0 && y >= 0 && x < w && y < h && img.get_pixel(x as u32, y as u32).a > 0.5
    };
    const VOISINS: [(i32, i32); 8] =
        [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)];
    let mut a_tracer = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if opaque(img, x, y) {
                continue;
            }
            if VOISINS.iter().any(|(dx, dy)| opaque(img, x + dx, y + dy)) {
                a_tracer.push((x as u32, y as u32));
            }
        }
    }
    for (x, y) in a_tracer {
        img.set_pixel(x, y, couleur);
    }
}

/// Cache d'icônes, une texture par astre déjà rencontré.
pub struct IconesAstres {
    cache: HashMap<usize, Texture2D>,
    /// Compte des rendus ressortis vides, par astre — voir [`ESSAIS_MAX`].
    echecs: HashMap<usize, u16>,
    /// Cible de rendu standard, réutilisée d'un astre sans anneau à l'autre.
    scratch: Option<RenderTarget>,
    /// Cible de rendu pour un astre à anneau — plus grande (voir
    /// [`cote_gazeuse`]), donc distincte : une `RenderTarget` macroquad a une
    /// taille fixe, et personne d'autre n'a besoin de ce format.
    scratch_gazeuse: Option<RenderTarget>,
}

impl IconesAstres {
    pub fn new() -> Self {
        Self { cache: HashMap::new(), echecs: HashMap::new(), scratch: None, scratch_gazeuse: None }
    }

    /// Icône déjà en cache pour l'astre `idx`, **sans** en déclencher le
    /// rendu — c'est [`Self::completer`] qui remplit le cache. À appeler
    /// depuis le dessin, qui ne doit rien faire d'autre que lire.
    pub fn prete(&self, idx: usize) -> Option<&Texture2D> {
        self.cache.get(&idx)
    }

    /// Rend **au plus une** icône manquante parmi `indices`, et l'ajoute au
    /// cache si le rendu a vraiment peint quelque chose.
    ///
    /// ⚠️ Une seule par appel, et c'est voulu : un **second** changement de
    /// caméra 3D vers une cible hors écran, dans la **même** frame, ressort
    /// vide — mesuré à l'écran, pas supposé. Les icônes manquantes se
    /// rattrapent donc sur plusieurs frames plutôt que toutes d'un coup — un
    /// effet de peuplement progressif, pas un défaut caché.
    ///
    /// ⚠️ **Un rendu isolé peut, lui aussi, ressortir vide** — mesuré sur des
    /// astres différents d'un lancement à l'autre (jamais le même deux fois),
    /// donc pas propre à un astre précis : une chauffe de pipeline ou de
    /// tampon de profondeur côté moteur, supposée faute de pouvoir
    /// l'inspecter plus bas que macroquad. Comme l'astre en cause varie, un
    /// nombre fixe de coups jetés au démarrage ne le couvrirait pas de façon
    /// fiable — **`rendre` est donc vérifié** ([`vide`], quelques pixels lus
    /// sur la texture) et, s'il n'a rien peint, retenté tout seul à un
    /// prochain appel plutôt que mis en cache tel quel.
    ///
    /// ⚠️ **Une tellurique dont le terrain n'a pas fini de générer** (job
    /// asynchrone à budget limité, voir [`crate::astre::Astre::rendu_pret`])
    /// est sautée sans compter contre `ESSAIS_MAX` : son relief retombe sur
    /// un repli tant que le vrai terrain n'est pas prêt, et ce repli n'est
    /// **pas vide** (`vide` ne l'aurait donc jamais rattrapé) — il figerait
    /// pour de bon un rendu qui n'a rien à voir avec le preset (c'était le
    /// bug derrière une Terre bien plus verte que bleue dans le sélecteur).
    pub fn completer(&mut self, sys: &mut Systeme, indices: impl Iterator<Item = usize>) {
        for idx in indices {
            if self.cache.contains_key(&idx) {
                continue;
            }
            if self.echecs.get(&idx).copied().unwrap_or(0) >= ESSAIS_MAX {
                continue; // abandonné pour cet astre : le repli (pastille) reste
            }
            if !sys.rendu_pret_de(idx) {
                continue; // terrain encore en génération : retenté sans limite
            }
            if let Some(mut img) = self.rendre(sys, idx) {
                if vide(&img) {
                    *self.echecs.entry(idx).or_insert(0) += 1;
                } else {
                    tracer_contour(&mut img, CONTOUR);
                    self.cache.insert(idx, Texture2D::from_image(&img));
                }
            }
            return; // une seule par appel — voir l'avertissement ci-dessus
        }
    }

    /// Rend l'astre `idx` dans la cible réutilisable adaptée (standard ou
    /// gazeuse), et en relit les pixels — l'appelant décide s'ils sont
    /// vides, trace le contour, et bâtit la texture mise en cache à partir
    /// du résultat (voir la note de module).
    fn rendre(&mut self, sys: &mut Systeme, idx: usize) -> Option<Image> {
        let rayon = sys.rayon_de(idx)?.max(1e-4);
        let etendue = sys.rayon_visuel_de(idx).unwrap_or(rayon).max(rayon);
        let c = cadrage(rayon, etendue);

        // `depth: true` — sans tampon de profondeur, les faces arrière d'une
        // sphère passeraient devant les faces avant.
        let cible = if c.cote == COTE { &mut self.scratch } else { &mut self.scratch_gazeuse };
        let rt = cible.get_or_insert_with(|| {
            let rt = render_target_ex(c.cote, c.cote, RenderTargetParams { sample_count: 1, depth: true });
            rt.texture.set_filter(FilterMode::Linear);
            rt
        });

        let centre = sys.position(idx);
        // Caméra du côté éclairé, un peu en hauteur — même raison qu'à la
        // vignette : de face contre la lumière on ne verrait qu'un croissant.
        // Sauf pour un astre à anneau, dont la direction ET le "haut" se
        // choisissent plutôt selon son inclinaison propre — voir `cadre_camera`.
        let vers_lumiere = (sys.position_lumiere() - centre).normalize_or_zero();
        let (direction, haut) = cadre_camera(vers_lumiere, sys.normale_anneau_de(idx));
        let pos = centre + direction * c.recul;

        let cam3d = Camera3D {
            position: pos,
            target: centre,
            up: haut,
            fovy: c.fovy_deg.to_radians(),
            aspect: Some(1.0),
            render_target: Some(rt.clone()),
            z_near: c.z_near,
            ..Default::default()
        };
        set_camera(&cam3d);
        clear_background(TRANSPARENT);

        // `haut` (pas `Vec3::Y`) : pour un astre à anneau, le "haut" réel de
        // la caméra n'est plus l'axe du monde — recalculer la base depuis
        // `Vec3::Y` désaccorderait `right`/`up` de l'orientation réellement
        // rendue.
        let forward = (centre - pos).normalize_or_zero();
        let right = forward.cross(haut).normalize_or_zero();
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

        Some(rt.texture.get_texture_data())
    }
}

impl Default for IconesAstres {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // **L'élévation d'un anneau est nulle** — comparée à la valeur
    // littérale, pas à sa propre formule : se mesurer contre
    // `elevation_anneau_rad()` reviendrait à réciter le code, et le test
    // suivrait n'importe quelle valeur qu'on lui donnerait sans jamais
    // rougir.
    #[test]
    fn lelevation_dun_anneau_est_nulle() {
        assert_eq!(elevation_anneau_rad(), 0.0);
    }

    // **Conséquence directe d'une élévation nulle** : la caméra vise
    // exactement de profil — `direction` tombe **dans** le plan de l'anneau
    // (perpendiculaire à sa normale) et `haut` devient la normale
    // elle-même. C'est ce qui réduit l'anneau à un trait horizontal, sans
    // aucune ouverture.
    #[test]
    fn a_elevation_nulle_la_direction_tombe_dans_le_plan_de_lanneau() {
        let normale = vec3(0.15, 0.936, 0.32);
        let (direction, haut) = cadre_camera(Vec3::Z, Some(normale));
        assert!(direction.dot(normale.normalize()).abs() < 1e-5, "direction hors du plan de l'anneau");
        assert!((haut - normale.normalize()).length() < 1e-5, "haut {haut} devrait être la normale elle-même");
    }

    // **Le plan proche ne doit jamais mordre la sphère** — c'était le bug de
    // Pluton : à l'échelle vraie du système, un corps assez petit avait un
    // `recul` sous le plan proche par défaut (0,01), et sa sphère entière
    // tombait hors champ, systématiquement, sur toutes les tentatives.
    #[test]
    fn le_plan_proche_reste_sous_la_face_proche_du_corps() {
        // De Pluton (le plus petit corps du preset solaire, en unités de
        // monde réelles) au Soleil : le plan proche doit rester devant la
        // sphère à toutes les échelles, pas seulement les grandes.
        for rayon in [0.0019_f32, 0.0039, 0.0102, 0.0934, 1.1161] {
            let c = cadrage(rayon, rayon);
            let face_proche = c.recul - rayon;
            assert!(
                c.z_near < face_proche,
                "rayon={rayon} : plan proche {} devant la face du corps {face_proche}",
                c.z_near
            );
        }
    }

    // Même chose pour un corps à anneau — la garde ne doit pas dépendre du
    // chemin emprunté pour l'obtenir.
    #[test]
    fn le_plan_proche_reste_sous_la_face_proche_dun_corps_a_anneau() {
        let rayon = 0.0934_f32; // Saturne
        let etendue = 0.2149; // anneau
        let c = cadrage(rayon, etendue);
        assert!(c.z_near < c.recul - rayon, "plan proche {} devant la face du corps {}", c.z_near, c.recul - rayon);
    }

    // **Régression directe** : les vraies valeurs mesurées de Pluton, qui
    // rendaient une sphère entièrement derrière le plan proche par défaut
    // (near_face ET far_face sous 0,01) avant ce correctif.
    #[test]
    fn pluton_a_lechelle_reelle_ne_tombe_plus_hors_champ() {
        let rayon = 0.0019093_f32;
        let c = cadrage(rayon, rayon);
        assert!(c.recul - rayon > c.z_near, "la sphère de Pluton tombe toujours devant le plan proche");
    }

    // **Une planète à anneau se cadre comme une sans anneau** — même
    // `recul`, à rayon égal : c'était le point de la demande, une gazeuse ne
    // doit pas peser plus ou moins lourd dans la colonne selon qu'elle a un
    // anneau ou non.
    #[test]
    fn une_planete_a_anneau_se_cadre_comme_une_sans_anneau_a_rayon_egal() {
        let rayon = 0.0934_f32; // Saturne
        let sans_anneau = cadrage(rayon, rayon);
        let avec_anneau = cadrage(rayon, 0.2149); // anneau à 2,3 rayons
        assert_eq!(sans_anneau.recul, avec_anneau.recul, "le recul ne doit pas dépendre de l'anneau");
    }

    // **La cible d'un astre à anneau est plus grande**, pour capter l'anneau
    // sans reculer la caméra — voir la note de module.
    #[test]
    fn la_cible_dun_astre_a_anneau_est_plus_grande() {
        let rayon = 0.0934_f32;
        let sans_anneau = cadrage(rayon, rayon);
        let avec_anneau = cadrage(rayon, 0.2149);
        assert_eq!(sans_anneau.cote, COTE);
        assert!(avec_anneau.cote > COTE, "la cible d'un anneau devrait être plus grande que {COTE}");
    }

    // **Le point de tout le mécanisme** : à `recul` égal (même rayon, avec
    // ou sans anneau), la planète occupe le même nombre de pixels dans les
    // deux cadrages — champ élargi et résolution agrandie se compensent
    // exactement. Réplique la projection utilisée par `rendre` : rayon
    // apparent en pixels = (rayon / (recul × tan(demi-champ))) × (côté / 2).
    #[test]
    fn une_planete_a_anneau_garde_la_meme_taille_quune_sans() {
        fn rayon_apparent_px(rayon: f32, c: Cadrage) -> f32 {
            let theta = (c.fovy_deg / 2.0).to_radians();
            (rayon / (c.recul * theta.tan())) * (c.cote as f32 / 2.0)
        }
        let rayon = 0.0934_f32;
        let sans_anneau = cadrage(rayon, rayon);
        let avec_anneau = cadrage(rayon, 0.2149);
        let px_sans = rayon_apparent_px(rayon, sans_anneau);
        let px_avec = rayon_apparent_px(rayon, avec_anneau);
        assert!(
            (px_sans - px_avec).abs() < 0.5,
            "la planète ne fait pas la même taille en pixels : {px_sans} vs {px_avec}"
        );
    }

    // **Le point de la demande** : l'anneau doit apparaître **horizontal** à
    // l'écran, quelle que soit son inclinaison réelle. C'est exactement dire
    // que `haut` est la projection de la normale de l'anneau sur le plan de
    // l'écran (perpendiculaire à la direction de vue) — la définition même
    // de « le grand axe de l'ellipse tombe à l'horizontale ». Valeurs
    // réelles de Saturne (~21° hors de l'axe Y).
    #[test]
    fn lanneau_incline_apparait_horizontal_a_lecran() {
        let normale = vec3(0.15, 0.936, 0.32);
        let (direction, haut) = cadre_camera(Vec3::Z, Some(normale));
        let forward = (-direction).normalize_or_zero();
        let projection_normale = (normale - normale.dot(forward) * forward).normalize_or_zero();
        assert!(
            (haut - projection_normale).length() < 1e-4,
            "haut {haut} devrait être la projection de la normale {projection_normale}"
        );
        // Et c'est un vecteur "haut" valide : perpendiculaire à la vue.
        assert!(haut.dot(direction).abs() < 1e-4, "haut pas perpendiculaire à la direction de vue");
    }

    // **Sans inclinaison horizontale, la direction retombe sur une valeur
    // par défaut** plutôt que sur un vecteur nul (qui donnerait une caméra
    // confondue avec sa cible) — et `haut` reste un vecteur valide.
    #[test]
    fn cadre_camera_retombe_sur_une_valeur_par_defaut_sans_inclinaison() {
        let (direction, haut) = cadre_camera(Vec3::Z, Some(Vec3::Y));
        assert!((direction.length() - 1.0).abs() < 1e-4);
        assert!((haut.length() - 1.0).abs() < 1e-4);
    }

    // **Avec un anneau, l'approche privilégie le côté éclairé** quand les
    // deux sont également valables (l'anneau reste horizontal dans les deux
    // cas) — sinon l'astre pourrait se présenter dans l'ombre sans raison.
    // La lumière vient exactement de `-e1` (le premier choix, avant
    // correction) : sans le correctif, `direction` s'en écarterait.
    #[test]
    fn un_anneau_privilegie_le_cote_eclaire() {
        let normale = vec3(0.15, 0.936, 0.32);
        let e1_initial = Vec3::Y.cross(normale).normalize();
        let vers_lumiere = -e1_initial;
        let (direction, _) = cadre_camera(vers_lumiere, Some(normale));
        assert!(direction.dot(vers_lumiere) > 0.5, "l'approche devrait pencher vers la lumière : {direction}");
    }

    // **Sans anneau, le cadrage suit la lumière** comme avant ce correctif —
    // seuls les astres à anneau changent de comportement.
    #[test]
    fn sans_anneau_le_cadrage_suit_la_lumiere_et_lelevation_reste_standard() {
        let lum = vec3(0.6, 0.0, 0.8);
        let (direction, haut) = cadre_camera(lum, None);
        assert_eq!(direction, lum + Vec3::Y * ELEVATION);
        assert_eq!(haut, Vec3::Y);
    }

    // **Le champ élargi capte l'anneau de Saturne** (2,3 rayons), avec une
    // petite marge — c'est la cible réelle du cadrage, voir la note de
    // `FOVY_GAZEUSE_DEG`. Sans quoi élargir le champ n'aurait servi à rien,
    // l'anneau resterait hors cadre.
    #[test]
    fn le_champ_gazeuse_capte_lanneau_de_saturne() {
        let rayon = 1.0_f32;
        let etendue_saturne = 2.3_f32;
        let c = cadrage(rayon, etendue_saturne * rayon);
        let theta = (c.fovy_deg / 2.0).to_radians();
        let champ_visible = c.recul * theta.tan();
        assert!(
            champ_visible > etendue_saturne,
            "champ trop étroit pour l'anneau de Saturne : {champ_visible} <= {etendue_saturne}"
        );
    }

    // **Un anneau plus large que celui de Saturne déborde du cadre** —
    // conséquence assumée du choix ci-dessus (cadré sur Saturne, pas sur le
    // maximum théorique du générateur aléatoire, 2,5 rayons) : mesurée,
    // documentée, pas découverte plus tard par surprise à l'écran.
    #[test]
    fn un_anneau_plus_large_que_saturne_deborde_du_cadre() {
        let rayon = 1.0_f32;
        let etendue_max_generee = 2.5_f32;
        let c = cadrage(rayon, etendue_max_generee * rayon);
        let theta = (c.fovy_deg / 2.0).to_radians();
        let champ_visible = c.recul * theta.tan();
        assert!(
            champ_visible < etendue_max_generee,
            "le maximum théorique tiendrait finalement dans le cadre : {champ_visible} >= {etendue_max_generee}"
        );
    }

    // **L'échelle de dessin suit la résolution native** : 1 pour une cible
    // standard, proportionnellement plus pour une cible plus grande — sinon
    // une cible agrandie pour loger un anneau se retrouverait compressée
    // dans la même case à l'écran, annulant le débordement voulu.
    #[test]
    fn echelle_dessin_suit_la_resolution_native() {
        assert_eq!(echelle_dessin(COTE as f32), 1.0);
        assert_eq!(echelle_dessin(COTE as f32 * 2.0), 2.0);
    }

    fn image_vide(cote: u16) -> Image {
        Image { bytes: vec![0u8; cote as usize * cote as usize * 4], width: cote, height: cote }
    }

    // **Une image entièrement transparente est vide.**
    #[test]
    fn une_image_transparente_est_vide() {
        assert!(vide(&image_vide(16)));
    }

    // **Un seul pixel opaque suffit à ne plus l'être** — même loin de la
    // grille d'échantillonnage (`step_by(4)`), en plaçant le pixel pile sur
    // un point échantillonné.
    #[test]
    fn un_pixel_opaque_echantillonne_nest_plus_vide() {
        let mut img = image_vide(16);
        img.set_pixel(4, 4, WHITE);
        assert!(!vide(&img));
    }

    // **Le contour n'apparaît qu'autour de la silhouette**, à un pixel de
    // distance — pas plus loin, pas ailleurs sur l'image.
    #[test]
    fn le_contour_entoure_la_silhouette_dun_seul_pixel() {
        let mut img = image_vide(7);
        img.set_pixel(3, 3, RED); // un seul pixel opaque, au centre
        let rouge = img.get_pixel(3, 3); // couleur telle que réellement stockée (octets u8)
        tracer_contour(&mut img, WHITE);

        // Le pixel d'origine n'a pas bougé.
        assert_eq!(img.get_pixel(3, 3), rouge);

        // Les 8 voisins directs sont devenus blancs, à alpha plein.
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let (x, y) = ((3 + dx) as u32, (3 + dy) as u32);
            assert_eq!(img.get_pixel(x, y), WHITE, "voisin ({dx},{dy}) pas tracé");
        }

        // Un pixel à deux cases du centre reste transparent : le contour ne
        // déborde pas au-delà d'un pixel.
        assert_eq!(img.get_pixel(1, 3).a, 0.0, "le contour dépasse un pixel");
        assert_eq!(img.get_pixel(3, 1).a, 0.0, "le contour dépasse un pixel");
    }

    // **Un contour en bord d'image ne panique pas** — les voisins hors
    // limites doivent être traités comme transparents, pas comme un accès
    // invalide.
    #[test]
    fn le_contour_en_bord_dimage_ne_panique_pas() {
        let mut img = image_vide(4);
        img.set_pixel(0, 0, RED);
        let rouge = img.get_pixel(0, 0); // couleur telle que réellement stockée (octets u8)
        tracer_contour(&mut img, WHITE);
        assert_eq!(img.get_pixel(0, 0), rouge);
        assert_eq!(img.get_pixel(1, 0), WHITE);
        assert_eq!(img.get_pixel(0, 1), WHITE);
        assert_eq!(img.get_pixel(1, 1), WHITE);
    }
}
