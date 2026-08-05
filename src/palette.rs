//! **Palettes de quantification** et la colorimétrie qui va avec.
//!
//! Le rendu pixel art tient en deux gestes distincts :
//!
//! 1. **Descendre en résolution** puis remonter au plus proche voisin — c'est
//!    [`crate::ecran::pixel`], et ça ne donne que de gros pixels ;
//! 2. **Contraindre les couleurs à une palette fixe** — c'est ici. Sans ça,
//!    l'éclairage 3D produit des dégradés RGB continus : de la 3D basse
//!    résolution, pas du pixel art.
//!
//! # Pourquoi CIELAB
//!
//! Chercher la couleur la plus proche **en RGB** revient à mesurer dans un
//! espace dont les axes ne pèsent pas ce que l'œil perçoit. CIELAB est construit
//! pour que la distance euclidienne approche l'écart *perçu*. Ce n'est pas un
//! principe : `le_choix_lab_differe_du_choix_rgb` vérifie que les deux méthodes
//! choisissent réellement différemment.
//!
//! La palette est convertie en LAB **une fois, sur le CPU**. Le shader ne
//! convertit que le pixel courant — une conversion par pixel au lieu de N+1.
//!
//! # ⚠️ Une palette d'artiste n'est pas une rampe de dégradé
//!
//! Mesuré sur Resurrect 64 : un dégradé de gris ne tombe que sur **8 couleurs**,
//! et deux marches sont brutales — 47 % → 51 % d'entrée fait sauter la clarté de
//! L=49 à L=69, et tout ce qui dépasse 89 % s'écrase sur `#ffffff` (au-dessus de
//! L=70, la palette n'a que **deux** couleurs quasi neutres).
//!
//! C'est la cause de deux défauts visibles à l'écran :
//!
//! - **les bandes qui basculent d'un coup** quand le terminateur d'une planète
//!   se déplace : une région entière change de couleur en même temps ;
//! - **le reflet spéculaire qui bave** : le halo autour du point chaud franchit
//!   le seuil du blanc pur d'un seul tenant, et devient un aplat.
//!
//! Ces palettes ne sont pas fautives — elles sont faites pour qu'un dessinateur
//! choisisse ses teintes, pas pour quantifier un ombrage continu. Les deux
//! remèdes sont dans le shader : le **tramage ordonné** ([`matrice_bayer`]), qui
//! rend les teintes intermédiaires en mélangeant spatialement deux entrées
//! voisines, et l'**écrêtage des hautes lumières** ([`ECRETAGE_SEUIL`]), qui
//! empêche le halo d'atteindre le blanc.
//!
//! # Ajouter une palette
//!
//! Deux façons, sans toucher au reste du code :
//!
//! - **un fichier** `assets/palettes/<nom>.hex` — un hexadécimal par ligne,
//!   c'est le format d'export de Lospec ; il est ramassé au démarrage ;
//! - **une constante** ici, ajoutée à [`INTEGREES`] — toujours disponible, même
//!   sans le dossier d'assets.

use macroquad::prelude::*;
use std::sync::OnceLock;

/// Plafond du tableau d'uniformes du shader.
///
/// Le shader déclare `vec3 palette_lab[MAX]` : une palette plus longue serait
/// tronquée en silence. `aucune_palette_ne_depasse_le_plafond` l'interdit. Le
/// relever se fait ici **et** dans `palette.frag.glsl` (`#define MAX`).
///
/// **256 et non 64.** Le premier plafond était calé sur Resurrect 64, mais les
/// palettes publiées vont bien au-delà : Lospec 2000 en compte 182, AllStars 128.
/// Elles étaient **rejetées puis ignorées**, avec pour seule trace une ligne de
/// console que personne ne lit — d'où des palettes déposées qui n'apparaissaient
/// jamais au menu.
///
/// Coût : 2 × 256 `vec3` d'uniformes, soit 1 536 composantes. Le minimum garanti
/// par GL de bureau est très au-dessus ; ce serait à repenser (palette en
/// texture) pour une cible WebGL/mobile stricte.
///
/// ⚠️ Coût **par pixel** aussi : la recherche parcourt `nb_couleurs` entrées. Une
/// palette de 182 coûte presque trois fois une de 64.
pub const MAX: usize = 256;

/// Côté de la matrice de tramage. 8 × 8 est le compromis habituel : assez de
/// niveaux (64) pour traverser une marche de palette, assez petit pour rester
/// invisible en motif.
pub const COTE_TRAME: usize = 8;

/// Au-dessus de ce niveau, les hautes lumières sont comprimées au lieu de
/// monter tout droit vers le blanc.
///
/// Le spéculaire des océans est **additif** (`planete.frag.glsl` : `col +=
/// ... * spec`) et dépasse donc 1.0. Sans compression, tout le halo au-dessus de
/// 0,89 s'écrase sur `#ffffff` d'un seul tenant et forme un aplat bien plus
/// large que le point chaud réel.
pub const ECRETAGE_SEUIL: f32 = 0.72;

/// Saturation appliquée aux **hautes lumières**, quel que soit le réglage.
///
/// # Pourquoi désaturer le haut
///
/// Un reflet spéculaire est achromatique dans la réalité. Sans ce garde-fou, le
/// halo du reflet sur un océan reste bleuté, et comme la palette n'a que des
/// entrées **chromatiques** vers L≈82-91 (`#8fd3ff`, `#8ff8e2`), il tombe sur un
/// cyan franc : c'est l'anneau cyan visible autour du point chaud.
///
/// En dessous de 1, les hautes lumières sont ramenées vers le neutre et
/// retombent sur `#c7dcd0` / `#ffffff`.
pub const SAT_HAUTES: f32 = 0.5;

/// Bande de luminance sur laquelle la saturation passe du réglage à
/// [`SAT_HAUTES`]. En dessous, le gain s'applique en plein ; au-dessus, plus du
/// tout.
///
/// La bande démarre assez bas (0,50) pour que le **halo** d'un reflet, qui vit
/// vers 0,75-0,85 de luminance, soit déjà bien désaturé : à 0,55-0,90 il gardait
/// encore un gain de 0,81 et restait cyan.
pub const SAT_ROLLOFF: (f32, f32) = (0.50, 0.82);

/// À quel point on comprime au-dessus du seuil. 0 = pas d'écrêtage, 1 = plafond
/// dur au seuil.
///
/// À 0,30, une entrée de 1,0 ressort à 0,80 (donc sous le blanc), tandis qu'un
/// point chaud à 1,5 ressort à 0,95 : **le cœur du reflet reste blanc, son halo
/// ne l'est plus**. C'est le but — réduire l'aplat, pas supprimer le reflet.
pub const ECRETAGE_FORCE: f32 = 0.30;

// ---------------------------------------------------------------- palettes

/// Les palettes livrées avec le jeu : `(nom, couleurs)`.
///
/// En ajouter une tient en une ligne. Le nom est ce qu'affiche le menu.
#[allow(clippy::type_complexity)]
pub const INTEGREES: &[(&str, &[&str])] = &[
    ("RESURRECT 64", &RESURRECT_64),
    ("SWEETIE 16", &SWEETIE_16),
    ("PICO-8", &PICO_8),
];

/// Resurrect 64 — 64 teintes, très chromatique.
const RESURRECT_64: [&str; 64] = [
    "#2e222f", "#3e3546", "#625565", "#966c6c", "#ab947a", "#694f62", "#7f708a", "#9babb2",
    "#c7dcd0", "#ffffff", "#6e2727", "#b33831", "#ea4f36", "#f57d4a", "#ae2334", "#e83b3b",
    "#fb6b1d", "#f79617", "#f9c22b", "#7a3045", "#9e4539", "#cd683d", "#e6904e", "#fbb954",
    "#4c3e24", "#676633", "#a2a947", "#d5e04b", "#fbff86", "#165a4c", "#239063", "#1ebc73",
    "#91db69", "#cddf6c", "#313638", "#374e4a", "#547e64", "#92a984", "#b2ba90", "#0b5e65",
    "#0b8a8f", "#0eaf9b", "#30e1b9", "#8ff8e2", "#323353", "#484a77", "#4d65b4", "#4d9be6",
    "#8fd3ff", "#45293f", "#6b3e75", "#905ea9", "#a884f3", "#eaaded", "#753c54", "#a24b6f",
    "#cf657f", "#ed8099", "#831c5d", "#c32454", "#f04f78", "#f68181", "#fca790", "#fdcbb0",
];

/// Sweetie 16 — 16 teintes. Sert surtout à vérifier qu'une **petite** palette
/// traverse toute la chaîne (le shader s'arrête à `nb_couleurs`, pas à `MAX`).
const SWEETIE_16: [&str; 16] = [
    "#1a1c2c", "#5d275d", "#b13e53", "#ef7d57", "#ffcd75", "#a7f070", "#38b764", "#257179",
    "#29366f", "#3b5dc9", "#41a6f6", "#73eff7", "#f4f4f4", "#94b0c2", "#566c86", "#333c57",
];

/// PICO-8 — les 16 couleurs de la console fantôme.
const PICO_8: [&str; 16] = [
    "#000000", "#1d2b53", "#7e2553", "#008751", "#ab5236", "#5f574f", "#c2c3c7", "#fff1e8",
    "#ff004d", "#ffa300", "#ffec27", "#00e436", "#29adff", "#83769c", "#ff77a8", "#ffccaa",
];

/// Une palette prête à servir : les couleurs, et leur version CIELAB.
#[derive(Debug)]
pub struct Palette {
    pub nom: String,
    pub rgb: Vec<Vec3>,
    /// Même ordre que [`Self::rgb`] — c'est sur ceci que se mesurent les
    /// distances. Les deux tableaux partent ensemble vers le shader.
    pub lab: Vec<Vec3>,
}

impl Palette {
    /// Construit depuis une liste d'hexadécimaux. `Err` avec l'entrée fautive :
    /// une palette à moitié lue vaudrait moins qu'un message clair.
    pub fn depuis_hex(nom: &str, hex: &[&str]) -> Result<Palette, String> {
        let mut rgb = Vec::with_capacity(hex.len());
        for (i, h) in hex.iter().enumerate() {
            let c = hex_vers_rgb(h).ok_or_else(|| format!("{nom} : couleur {i} illisible ({h})"))?;
            rgb.push(c);
        }
        Self::depuis_couleurs(nom, rgb)
    }

    /// Construit depuis le **texte d'un fichier `.hex`** : un hexadécimal par
    /// ligne, tel que Lospec l'exporte. Les lignes vides et les commentaires
    /// (`;` ou `//`) sont ignorés ; le `#` de tête est toléré.
    pub fn depuis_texte(nom: &str, texte: &str) -> Result<Palette, String> {
        let mut rgb = Vec::new();
        for (n, ligne) in texte.lines().enumerate() {
            let l = ligne.trim();
            if l.is_empty() || l.starts_with(';') || l.starts_with("//") {
                continue;
            }
            let c = hex_vers_rgb(l)
                .ok_or_else(|| format!("{nom} : ligne {} illisible ({l})", n + 1))?;
            rgb.push(c);
        }
        Self::depuis_couleurs(nom, rgb)
    }

    fn depuis_couleurs(nom: &str, rgb: Vec<Vec3>) -> Result<Palette, String> {
        if rgb.len() < 2 {
            return Err(format!("{nom} : {} couleur(s), il en faut au moins 2", rgb.len()));
        }
        if rgb.len() > MAX {
            return Err(format!("{nom} : {} couleurs, le shader en tient {MAX}", rgb.len()));
        }
        let lab = rgb.iter().map(|c| rgb_vers_lab(*c)).collect();
        Ok(Palette { nom: nom.to_string(), rgb, lab })
    }

    pub fn nb(&self) -> usize {
        self.rgb.len()
    }

    /// Les couleurs complétées jusqu'à [`MAX`], prêtes pour l'uniforme.
    ///
    /// Le remplissage répète la dernière couleur : le shader s'arrête à
    /// `nb_couleurs`, mais un tableau à moitié nul serait un piège si cette
    /// borne venait à sauter.
    pub fn tableau(&self, source: &[Vec3]) -> [Vec3; MAX] {
        let mut t = [*source.last().unwrap_or(&Vec3::ZERO); MAX];
        t[..source.len()].copy_from_slice(source);
        t
    }
}

/// Un fichier d'`assets/palettes/` que le jeu n'a pas pu charger.
///
/// **Conservé, pas seulement journalisé.** Une palette déposée qui n'apparaît
/// jamais au menu, sans que rien à l'écran ne le dise, est le défaut qui a coûté
/// le plus cher : `lospec-2000.hex` (182 couleurs) et `allstars.hex` (128)
/// étaient refusées par un plafond de 64 et la seule trace partait dans une
/// console que personne ne lit.
#[derive(Debug, Clone)]
pub struct Rejet {
    pub fichier: String,
    pub raison: String,
}

struct Registre {
    palettes: Vec<Palette>,
    rejets: Vec<Rejet>,
}

fn registre() -> &'static Registre {
    static CACHE: OnceLock<Registre> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut palettes: Vec<Palette> = INTEGREES
            .iter()
            .map(|(nom, hex)| {
                Palette::depuis_hex(nom, hex).unwrap_or_else(|e| panic!("palette intégrée : {e}"))
            })
            .collect();
        let (trouvees, rejets) = trier(lire_dossier());
        palettes.extend(trouvees);
        Registre { palettes, rejets }
    })
}

/// Toutes les palettes disponibles : les intégrées, puis celles trouvées dans
/// `assets/palettes/`. Construites une seule fois.
pub fn toutes() -> &'static [Palette] {
    &registre().palettes
}

/// Les fichiers d'`assets/palettes/` qu'on n'a pas pu charger, avec la raison.
/// L'écran des paramètres les affiche.
pub fn rejets() -> &'static [Rejet] {
    &registre().rejets
}

/// Trie des fichiers `(nom, contenu)` en palettes et en rejets.
///
/// **Pur** — c'est ce qui le rend testable : la lecture disque est isolée dans
/// [`lire_dossier`], qui n'a aucune décision à prendre.
fn trier(fichiers: Vec<(String, Result<String, String>)>) -> (Vec<Palette>, Vec<Rejet>) {
    let mut ok = Vec::new();
    let mut ko = Vec::new();
    for (fichier, contenu) in fichiers {
        // Le nom affiché vient du fichier, en majuscules.
        let nom = fichier.trim_end_matches(".hex").to_uppercase();
        let issue = contenu.and_then(|t| Palette::depuis_texte(&nom, &t));
        match issue {
            Ok(p) => ok.push(p),
            // Un fichier fautif n'empêche **jamais** les autres de charger.
            Err(raison) => ko.push(Rejet { fichier, raison }),
        }
    }
    (ok, ko)
}

/// La palette d'indice `i`, en bouclant — l'indice vient d'un réglage, et le
/// nombre de palettes dépend du dossier d'assets.
pub fn palette(i: usize) -> &'static Palette {
    let t = toutes();
    &t[i % t.len()]
}

/// Lit `assets/palettes/*.hex` et rend les couples `(nom de fichier, contenu)`.
///
/// **Ne décide de rien** : le tri en palettes et rejets est fait par [`trier`],
/// qui est pur et donc testable. Un dossier absent est le cas normal.
fn lire_dossier() -> Vec<(String, Result<String, String>)> {
    let mut v = Vec::new();
    let Ok(entrees) = std::fs::read_dir("assets/palettes") else {
        return v;
    };
    let mut chemins: Vec<_> = entrees.flatten().map(|e| e.path()).collect();
    chemins.sort(); // ordre stable d'un lancement à l'autre
    for c in chemins {
        if c.extension().and_then(|e| e.to_str()) != Some("hex") {
            continue;
        }
        let fichier = c.file_name().and_then(|n| n.to_str()).unwrap_or("?").to_string();
        v.push((fichier, std::fs::read_to_string(&c).map_err(|e| e.to_string())));
    }
    v
}

// ------------------------------------------------------------ colorimétrie

/// Décode `#rrggbb` en composantes 0..1. `None` si ce n'est pas un hexadécimal
/// à six chiffres — mieux vaut un message qu'une couleur silencieusement noire.
pub fn hex_vers_rgb(hex: &str) -> Option<Vec3> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let comp = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok().map(|v| v as f32 / 255.0);
    Some(vec3(comp(0)?, comp(2)?, comp(4)?))
}

/// sRGB (0..1) → CIELAB, illuminant D65.
///
/// Le passage par le RGB **linéaire** n'est pas décoratif : sans la
/// dé-gammatisation, les tons sombres seraient traités comme bien plus clairs
/// qu'ils ne sont et toute la moitié basse de la palette serait mal choisie.
pub fn rgb_vers_lab(c: Vec3) -> Vec3 {
    let lin = |v: f32| {
        if v > 0.04045 {
            ((v + 0.055) / 1.055).powf(2.4)
        } else {
            v / 12.92
        }
    };
    let (r, g, b) = (lin(c.x), lin(c.y), lin(c.z));

    let x = r * 0.4124 + g * 0.3576 + b * 0.1805;
    let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
    let z = r * 0.0193 + g * 0.1192 + b * 0.9505;

    let f = |t: f32| {
        if t > 0.008856 {
            t.powf(1.0 / 3.0)
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x / 0.95047), f(y / 1.0), f(z / 1.08883));
    vec3(116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

// ----------------------------------------------------------------- tramage

/// Matrice de Bayer `COTE_TRAME × COTE_TRAME`, valeurs `0..n²-1`, en
/// ligne par ligne.
///
/// Construction récursive classique : `M₂ₙ = [[4M, 4M+2], [4M+3, 4M+1]]`. Elle
/// répartit les seuils de façon à ce que le motif ne fasse ni ligne ni paquet —
/// c'est ce qui rend le tramage lisible comme une teinte intermédiaire plutôt
/// que comme du bruit.
pub fn matrice_bayer() -> Vec<u32> {
    let mut m = vec![0u32];
    let mut n = 1usize;
    while n < COTE_TRAME {
        let n2 = n * 2;
        let mut r = vec![0u32; n2 * n2];
        for y in 0..n {
            for x in 0..n {
                let v = m[y * n + x] * 4;
                r[y * n2 + x] = v;
                r[y * n2 + x + n] = v + 2;
                r[(y + n) * n2 + x] = v + 3;
                r[(y + n) * n2 + x + n] = v + 1;
            }
        }
        m = r;
        n = n2;
    }
    m
}

/// La matrice de Bayer en octets, pour la texture transmise au shader.
///
/// Une **texture** et non un tableau d'uniformes : GLSL ES 1.00 n'autorise
/// l'indexation d'un tableau d'uniformes que par une constante, et l'indice de
/// tramage se calcule à partir de la position du pixel.
pub fn texture_bayer() -> Vec<u8> {
    matrice_bayer()
        .iter()
        .flat_map(|v| {
            let n = (((*v as f32 + 0.5) / (COTE_TRAME * COTE_TRAME) as f32) * 255.0).round() as u8;
            [n, n, n, 255]
        })
        .collect()
}

// -------------------------------------------------- miroir CPU (tests seuls)

/// Luminance perçue (Rec. 709) — l'axe autour duquel la saturation pivote.
#[cfg(test)]
pub fn luminance(c: Vec3) -> f32 {
    c.dot(vec3(0.2126, 0.7152, 0.0722))
}

/// Le gain réellement appliqué à cette luminance : le réglage en bas,
/// [`SAT_HAUTES`] en haut.
#[cfg(test)]
pub fn gain_sature(y: f32, reglage: f32) -> f32 {
    let (a, b) = SAT_ROLLOFF;
    let t = ((y - a) / (b - a)).clamp(0.0, 1.0);
    let t = t * t * (3.0 - 2.0 * t); // smoothstep
    reglage + (SAT_HAUTES - reglage) * t
}

/// Ravive la chroma **à luminance constante** — miroir CPU du shader.
///
/// Ne borne pas : l'écrêtage qui suit a besoin de voir les valeurs au-dessus de
/// 1, et le shader ne borne qu'au tout dernier moment.
#[cfg(test)]
pub fn saturer(c: Vec3, reglage: f32) -> Vec3 {
    let y = luminance(c);
    Vec3::splat(y) + (c - Vec3::splat(y)) * gain_sature(y, reglage)
}

/// Écrêtage des hautes lumières — **miroir CPU** de ce que fait le shader.
#[cfg(test)]
pub fn ecreter(c: Vec3) -> Vec3 {
    let m = c.x.max(c.y).max(c.z);
    if m <= ECRETAGE_SEUIL {
        return c;
    }
    let vise = ECRETAGE_SEUIL + (m - ECRETAGE_SEUIL) * ECRETAGE_FORCE;
    c * (vise / m)
}

/// Indice de la couleur la plus proche **en CIELAB**.
///
/// Distances **au carré** : la racine est monotone, elle ne change pas le
/// gagnant, et l'économiser vaut N racines par pixel. Le shader fait pareil.
///
/// Réservé aux tests : à l'exécution, c'est le GPU qui cherche.
#[cfg(test)]
pub fn plus_proche(p: &Palette, couleur: Vec3) -> usize {
    let cible = rgb_vers_lab(couleur);
    let mut meilleur = 0;
    let mut min = f32::INFINITY;
    for (i, l) in p.lab.iter().enumerate() {
        let d = (cible - *l).length_squared();
        if d < min {
            min = d;
            meilleur = i;
        }
    }
    meilleur
}

/// La même recherche, **en RGB** — la version naïve, qui n'est utilisée par
/// aucun rendu. Elle sert à `le_choix_lab_differe_du_choix_rgb`.
#[cfg(test)]
pub fn plus_proche_rgb(p: &Palette, couleur: Vec3) -> usize {
    let mut meilleur = 0;
    let mut min = f32::INFINITY;
    for (i, c) in p.rgb.iter().enumerate() {
        let d = (couleur - *c).length_squared();
        if d < min {
            min = d;
            meilleur = i;
        }
    }
    meilleur
}

/// Quantification **sans** tramage.
#[cfg(test)]
pub fn quantifier(p: &Palette, couleur: Vec3) -> Vec3 {
    p.rgb[plus_proche(p, couleur)]
}

/// La chaîne complète, miroir CPU du shader : **saturation, écrêtage, tramage,
/// quantification**, dans cet ordre.
///
/// L'ordre compte. La saturation d'abord, sur la couleur telle que la scène l'a
/// produite ; l'écrêtage ensuite, qui a besoin de voir les dépassements
/// au-dessus de 1 pour distinguer le cœur d'un reflet de son halo ; le tramage
/// en dernier, juste avant la recherche.
#[cfg(test)]
pub fn quantifier_trame(
    p: &Palette,
    couleur: Vec3,
    x: usize,
    y: usize,
    force: f32,
    saturation: f32,
) -> Vec3 {
    let m = matrice_bayer();
    let cellule = m[(y % COTE_TRAME) * COTE_TRAME + (x % COTE_TRAME)];
    let seuil = (cellule as f32 + 0.5) / (COTE_TRAME * COTE_TRAME) as f32 - 0.5;
    let c = ecreter(saturer(couleur, saturation)) + Vec3::splat(seuil * force);
    quantifier(p, c.clamp(Vec3::ZERO, Vec3::ONE))
}

#[cfg(test)]
// Ces tests affirment des choses sur des CONSTANTES, et clippy y voit une
// condition toujours vraie. Elle l'est — aujourd'hui. Le jour où quelqu'un
// abaisse la constante, l'assertion devient fausse et le test tombe : c'est
// précisément son rôle, garder la valeur ET dire pourquoi elle compte.
#[allow(clippy::assertions_on_constants)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn octets(c: Vec3) -> (u8, u8, u8) {
        ((c.x * 255.0).round() as u8, (c.y * 255.0).round() as u8, (c.z * 255.0).round() as u8)
    }

    fn integrees() -> Vec<Palette> {
        INTEGREES.iter().map(|(n, h)| Palette::depuis_hex(n, h).expect(n)).collect()
    }

    // **Un fichier refusé est conservé avec sa raison**, pas seulement ignoré.
    // C'est le défaut qui a coûté le plus cher : deux palettes déposées
    // n'apparaissaient jamais au menu et la seule trace partait en console.
    #[test]
    fn un_fichier_refuse_est_conserve_avec_sa_raison() {
        let trop: String = std::iter::repeat_n("ffffff
", MAX + 1).collect();
        let (ok, ko) = trier(vec![("geante.hex".to_string(), Ok(trop))]);
        assert!(ok.is_empty(), "une palette trop longue a été acceptée");
        assert_eq!(ko.len(), 1);
        // Le **nom du fichier** est là : c'est ce qu'on va aller corriger.
        assert_eq!(ko[0].fichier, "geante.hex");
        // Et la raison dit le plafond, sinon on ne sait pas quoi corriger.
        assert!(ko[0].raison.contains(&MAX.to_string()), "raison muette : {}", ko[0].raison);
    }

    // **Un fichier fautif n'empêche pas les autres de charger.** Sans ça, une
    // seule faute de frappe priverait le jeu de toutes les palettes du dossier.
    #[test]
    fn un_fichier_fautif_nempeche_pas_les_autres() {
        let (ok, ko) = trier(vec![
            ("bon.hex".to_string(), Ok("2e222f
3e3546
625565".to_string())),
            ("casse.hex".to_string(), Ok("pas du tout un hexa".to_string())),
            ("illisible.hex".to_string(), Err("accès refusé".to_string())),
            ("autre.hex".to_string(), Ok("000000
ffffff".to_string())),
        ]);
        assert_eq!(ok.len(), 2, "les palettes valides n'ont pas survécu");
        assert_eq!(ko.len(), 2, "les fautifs ne sont pas tous signalés");
        // L'erreur disque remonte telle quelle, elle ne se déguise pas en
        // erreur de format.
        let disque = ko.iter().find(|r| r.fichier == "illisible.hex").unwrap();
        assert!(disque.raison.contains("refus"), "raison perdue : {}", disque.raison);
    }

    // Le nom affiché vient du fichier, sans son extension.
    #[test]
    fn le_nom_de_la_palette_vient_du_fichier() {
        let (ok, _) = trier(vec![("endesga_32.hex".to_string(), Ok("000000
ffffff".to_string()))]);
        assert_eq!(ok[0].nom, "ENDESGA_32", "nom inattendu : {}", ok[0].nom);
    }

    // **Toutes les palettes intégrées se construisent.** Un hexa mal recopié
    // rendrait la palette entière indisponible au démarrage.
    #[test]
    fn les_palettes_integrees_se_construisent() {
        assert!(!INTEGREES.is_empty(), "aucune palette intégrée");
        for (nom, hex) in INTEGREES {
            let p = Palette::depuis_hex(nom, hex)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(p.nb(), hex.len(), "{nom} : {} couleurs lues sur {}", p.nb(), hex.len());
        }
    }

    // **Aucune ne dépasse le plafond du shader**, et aucune n'est trop courte.
    // Au-delà de MAX, le tableau d'uniformes tronquerait sans rien dire.
    #[test]
    fn aucune_palette_ne_depasse_le_plafond() {
        for p in integrees() {
            assert!(p.nb() >= 2, "{} : {} couleur(s)", p.nom, p.nb());
            assert!(p.nb() <= MAX, "{} : {} couleurs pour un plafond de {MAX}", p.nom, p.nb());
        }
        // Et le constructeur refuse bien ce qui dépasse : sans ça, le plafond
        // ne serait qu'un commentaire.
        let trop: Vec<&str> = std::iter::repeat_n("#ffffff", MAX + 1).collect();
        assert!(Palette::depuis_hex("TROP", &trop).is_err(), "une palette trop longue est passée");
        assert!(Palette::depuis_hex("VIDE", &[]).is_err(), "une palette vide est passée");
    }

    // Les noms servent au menu : **distincts et non vides**, sinon deux entrées
    // deviennent indiscernables.
    #[test]
    fn les_palettes_ont_des_noms_distincts_et_non_vides() {
        let mut vus = HashSet::new();
        for (nom, _) in INTEGREES {
            assert!(!nom.trim().is_empty(), "palette sans nom");
            assert!(vus.insert(*nom), "{nom} en double");
        }
    }

    // Le décodage place les composantes **dans le bon ordre**. Un r/b inversé
    // passerait tous les autres tests : la palette resterait valide, mais fausse.
    #[test]
    fn le_decodage_respecte_lordre_des_composantes() {
        assert_eq!(octets(hex_vers_rgb("#ffffff").unwrap()), (255, 255, 255));
        assert_eq!(octets(hex_vers_rgb("#000000").unwrap()), (0, 0, 0));
        assert_eq!(octets(hex_vers_rgb("#ff8000").unwrap()), (255, 128, 0));
        assert_eq!(octets(hex_vers_rgb("204060").unwrap()), (32, 64, 96));
        for mauvais in ["#12345", "#gggggg", "", "#1234567"] {
            assert!(hex_vers_rgb(mauvais).is_none(), "{mauvais:?} accepté à tort");
        }
    }

    // **Le format `.hex` de Lospec se lit**, c'est le chemin par lequel une
    // palette s'ajoute sans toucher au code.
    #[test]
    fn le_format_hex_de_lospec_se_lit() {
        let texte = "; un commentaire\n2e222f\n#3E3546\r\n\n// autre commentaire\n625565\n";
        let p = Palette::depuis_texte("ESSAI", texte).expect("lecture");
        assert_eq!(p.nb(), 3, "commentaires ou lignes vides comptés comme couleurs");
        assert_eq!(octets(p.rgb[0]), (0x2e, 0x22, 0x2f));
        // La casse ne change rien, et le `#` de tête est toléré.
        assert_eq!(octets(p.rgb[1]), (0x3e, 0x35, 0x46));
        // Une ligne fautive est signalée, avec son numéro — pas ignorée.
        let e = Palette::depuis_texte("ESSAI", "2e222f\nzzz\n625565").unwrap_err();
        assert!(e.contains("ligne 2"), "erreur peu claire : {e}");
    }

    // **Aucun doublon** dans une palette : la seconde entrée ne serait jamais
    // choisie, c'est une case perdue.
    #[test]
    fn aucune_palette_na_de_couleur_en_double() {
        for p in integrees() {
            let mut vues = HashSet::new();
            for (i, c) in p.rgb.iter().enumerate() {
                assert!(vues.insert(octets(*c)), "{} : couleur {i} en double", p.nom);
            }
        }
    }

    // **Les repères connus de CIELAB.** Le gris sRGB 50 % tombe vers L≈53 et non
    // 50 : c'est précisément l'effet de la dé-gammatisation. L'oublier donnerait
    // L≈76.
    #[test]
    fn la_conversion_lab_retrouve_ses_reperes() {
        let blanc = rgb_vers_lab(vec3(1.0, 1.0, 1.0));
        assert!((blanc.x - 100.0).abs() < 0.1, "blanc : L={} au lieu de 100", blanc.x);
        assert!(blanc.y.abs() < 0.1 && blanc.z.abs() < 0.2, "blanc : teinte non nulle {blanc:?}");
        assert!(rgb_vers_lab(Vec3::ZERO).x.abs() < 0.1, "noir : L non nul");
        let gris = rgb_vers_lab(vec3(0.5, 0.5, 0.5));
        assert!((gris.x - 53.4).abs() < 1.0, "gris 50% : L={} (gamma oubliée ?)", gris.x);
        assert!(rgb_vers_lab(vec3(1.0, 0.0, 0.0)).y > 60.0, "le rouge devrait être très +a");
        assert!(rgb_vers_lab(vec3(0.0, 0.0, 1.0)).z < -60.0, "le bleu devrait être très -b");
    }

    // **Quantifier une couleur de la palette la rend telle quelle.** Prouve d'un
    // coup que la recherche trouve le minimum exact et que **chaque** couleur est
    // atteignable — une entrée que rien ne rend serait morte.
    #[test]
    fn quantifier_ne_touche_pas_aux_couleurs_de_la_palette() {
        for p in integrees() {
            for (i, c) in p.rgb.iter().enumerate() {
                assert_eq!(octets(quantifier(&p, *c)), octets(*c), "{} : couleur {i} déplacée", p.nom);
            }
        }
    }

    // **La sortie est toujours dans la palette** — c'est tout l'intérêt : sinon
    // il resterait des dégradés continus.
    #[test]
    fn quantifier_rend_toujours_une_couleur_de_la_palette() {
        for p in integrees() {
            let connues: HashSet<(u8, u8, u8)> = p.rgb.iter().map(|c| octets(*c)).collect();
            let mut sorties = HashSet::new();
            for r in 0..6 {
                for g in 0..6 {
                    for b in 0..6 {
                        let c = vec3(r as f32 / 5.0, g as f32 / 5.0, b as f32 / 5.0);
                        let q = octets(quantifier(&p, c));
                        assert!(connues.contains(&q), "{} : {c:?} sort {q:?}", p.nom);
                        sorties.insert(q);
                    }
                }
            }
            // Une quantification qui renverrait toujours la même couleur
            // passerait l'assertion ci-dessus.
            assert!(sorties.len() > 8, "{} : {} couleurs en sortie", p.nom, sorties.len());
        }
    }

    // **CIELAB ne choisit pas comme RGB.** Justification de tout le module : si
    // les deux s'accordaient partout, la conversion serait un coût pur.
    #[test]
    fn le_choix_lab_differe_du_choix_rgb() {
        let p = &integrees()[0];
        let mut desaccords = 0;
        for r in 0..8 {
            for g in 0..8 {
                for b in 0..8 {
                    let c = vec3(r as f32 / 7.0, g as f32 / 7.0, b as f32 / 7.0);
                    if plus_proche(p, c) != plus_proche_rgb(p, c) {
                        desaccords += 1;
                    }
                }
            }
        }
        assert!(desaccords > 0, "LAB et RGB choisissent toujours pareil");
    }

    // Les deux tableaux transmis au shader se **correspondent index par index** :
    // un décalage ferait choisir sur un critère et afficher sur un autre.
    #[test]
    fn les_deux_tableaux_du_shader_se_correspondent() {
        for p in integrees() {
            assert_eq!(p.rgb.len(), p.lab.len(), "{} : tableaux de tailles différentes", p.nom);
            for i in 0..p.nb() {
                assert_eq!(p.lab[i], rgb_vers_lab(p.rgb[i]), "{} : décalage à {i}", p.nom);
            }
        }
    }

    // Le complément jusqu'à MAX **ne crée pas de couleur** que la recherche
    // pourrait choisir par accident : il répète la dernière.
    #[test]
    fn le_tableau_transmis_est_complete_sans_inventer_de_couleur() {
        let p = &integrees()[1]; // une petite palette (16)
        let t = p.tableau(&p.rgb);
        assert_eq!(t.len(), MAX);
        assert_eq!(&t[..p.nb()], &p.rgb[..], "les vraies couleurs ont bougé");
        for (i, c) in t.iter().enumerate().skip(p.nb()) {
            assert_eq!(*c, *p.rgb.last().unwrap(), "case {i} : remplissage inattendu");
        }
    }

    // **La matrice de Bayer est une permutation** de 0..n²-1 : chaque seuil
    // exactement une fois. Un doublon créerait un motif visible et laisserait des
    // niveaux inatteignables.
    #[test]
    fn la_matrice_de_bayer_est_une_permutation() {
        let m = matrice_bayer();
        assert_eq!(m.len(), COTE_TRAME * COTE_TRAME);
        let mut vus: Vec<u32> = m.clone();
        vus.sort_unstable();
        let attendu: Vec<u32> = (0..(COTE_TRAME * COTE_TRAME) as u32).collect();
        assert_eq!(vus, attendu, "la matrice n'est pas une permutation");
        // Et c'est bien la matrice de Bayer, pas n'importe quelle permutation :
        // sa première ligne est connue.
        assert_eq!(&m[..8], &[0, 32, 8, 40, 2, 34, 10, 42], "ce n'est pas Bayer");
    }

    // **Le tramage restitue des teintes intermédiaires.** C'est le remède au
    // défaut observé : sans lui, un dégradé doux ne donne qu'une poignée
    // d'aplats qui basculent d'un bloc. Le test compare, sur une même rampe, le
    // nombre de couleurs obtenues avec et sans tramage.
    #[test]
    fn le_tramage_restitue_des_teintes_intermediaires() {
        let p = &integrees()[0];
        let mut sans = HashSet::new();
        let mut avec = HashSet::new();
        // Une rampe de gris douce, sur un bloc de 8×8 pixels par échelon.
        for i in 0..64 {
            let v = 0.2 + 0.6 * (i as f32 / 63.0);
            let c = Vec3::splat(v);
            sans.insert(octets(quantifier(p, c)));
            for y in 0..COTE_TRAME {
                for x in 0..COTE_TRAME {
                    // Saturation neutre : ce test-ci ne mesure que le tramage.
                    avec.insert(octets(quantifier_trame(p, c, x, y, 0.18, 1.0)));
                }
            }
        }
        assert!(
            avec.len() > sans.len(),
            "le tramage n'apporte rien : {} couleurs avec, {} sans",
            avec.len(),
            sans.len()
        );
        // Sans tramage, la rampe s'effondre sur très peu d'aplats — c'est la
        // mesure du défaut d'origine.
        assert!(sans.len() <= 8, "la rampe donne déjà {} couleurs", sans.len());
    }

    /// Chroma CIELAB — la « vivacité » d'une couleur.
    fn chroma(c: Vec3) -> f32 {
        let l = rgb_vers_lab(c);
        (l.y * l.y + l.z * l.z).sqrt()
    }

    // **La saturation ravive ce que la quantification ternit.**
    //
    // C'est le défaut le plus visible à l'écran : la Terre sortait grise-violette.
    // Mesuré : une planète voilée par son atmosphère n'a qu'une chroma modérée,
    // et les entrées **neutres** de la palette, voisines en CIELAB, l'emportent.
    // Une forêt voilée (chroma 17,6) tombait sur `#374e4a`, de chroma 9,8 — la
    // moitié perdue.
    #[test]
    fn la_saturation_ravive_les_couleurs_ternes() {
        let p = &integrees()[0];
        // Une rampe océan → côte → terre, telle qu'une atmosphère la voile.
        let voilees: Vec<Vec3> = (0..12)
            .map(|i| {
                let t = i as f32 / 11.0;
                vec3(0.20 + 0.35 * t, 0.28 + 0.25 * t, 0.46 - 0.10 * t)
            })
            .collect();

        let moyenne = |gain: f32| -> f32 {
            voilees.iter().map(|c| chroma(quantifier(p, saturer(*c, gain)))).sum::<f32>()
                / voilees.len() as f32
        };
        let sans = moyenne(1.0);
        let avec = moyenne(1.9);
        assert!(
            avec > sans * 1.25,
            "la saturation n'apporte rien : chroma {sans:.1} sans, {avec:.1} avec"
        );

        // Et le défaut à corriger est bien celui qu'on a vu à l'écran : sans
        // saturation, une partie de la rampe tombe sur des entrées **quasi
        // neutres** — le gris-violet qui remplaçait les océans.
        //
        // ⚠️ Ce n'est PAS un appauvrissement de la chroma moyenne : mesurée, elle
        // monte même un peu (15,2 en entrée → 18,9 en sortie), parce que les
        // couleurs qui tombent juste sont, elles, plus franches que l'entrée
        // voilée. Le défaut est la dispersion, pas la moyenne.
        let neutres = |gain: f32| {
            voilees.iter().filter(|c| chroma(quantifier(p, saturer(**c, gain))) < 12.0).count()
        };
        assert!(neutres(1.0) > 0, "aucune couleur ne tombe sur un neutre : rien à corriger");
        assert!(
            neutres(1.9) < neutres(1.0),
            "la saturation ne sort pas des neutres : {} → {}",
            neutres(1.0),
            neutres(1.9)
        );
    }

    // **La saturation ne déplace pas la luminance.** Elle doit raviver la
    // couleur, pas éclaircir ni assombrir l'image — sinon elle défait l'ombrage.
    #[test]
    fn la_saturation_conserve_la_luminance() {
        for c in [vec3(0.2, 0.35, 0.6), vec3(0.7, 0.5, 0.2), vec3(0.1, 0.1, 0.1)] {
            for gain in [1.0, 1.45, 1.9] {
                let avant = luminance(c);
                let apres = luminance(saturer(c, gain));
                assert!((avant - apres).abs() < 1e-4, "{c:?} × {gain} : {avant} → {apres}");
            }
        }
        // Un gris reste gris : rien à raviver sur l'axe neutre.
        let gris = saturer(Vec3::splat(0.4), 1.9);
        assert!((gris.x - gris.y).abs() < 1e-5 && (gris.y - gris.z).abs() < 1e-5, "{gris:?}");
    }

    // **Les hautes lumières sont désaturées, quel que soit le réglage.** Un
    // reflet est achromatique ; sans ce garde-fou, son halo tombe sur les entrées
    // cyan de la palette (`#8fd3ff`, `#8ff8e2`) et forme un anneau coloré.
    #[test]
    fn les_hautes_lumieres_sont_ramenees_vers_le_neutre() {
        // En bas de l'échelle, le réglage s'applique en plein.
        assert!((gain_sature(0.2, 1.9) - 1.9).abs() < 1e-5, "le gain n'agit pas sur les tons bas");
        // En haut, il est remplacé par SAT_HAUTES — et c'est bien une réduction.
        assert!((gain_sature(0.95, 1.9) - SAT_HAUTES).abs() < 1e-5, "les hautes lumières échappent");
        assert!(SAT_HAUTES < 1.0, "SAT_HAUTES ne désature pas");
        // Le passage est monotone : pas de rebond au milieu de la bande.
        let mut precedent = f32::INFINITY;
        for i in 0..=20 {
            let g = gain_sature(i as f32 / 20.0, 1.9);
            assert!(g <= precedent + 1e-6, "le gain remonte à y={}", i as f32 / 20.0);
            precedent = g;
        }
        // Effet concret : un halo bleuté vif perd la moitié de sa chroma.
        let halo = vec3(0.55, 0.85, 0.95);
        assert!(
            chroma(saturer(halo, 1.9)) < chroma(halo) * 0.7,
            "le halo reste coloré : {} → {}",
            chroma(halo),
            chroma(saturer(halo, 1.9))
        );
    }

    // **L'écrêtage empêche le halo d'atteindre le blanc, sans tuer le reflet.**
    // C'est le second défaut observé : le spéculaire additif dépassait 1,0 et
    // tout le halo s'écrasait d'un coup sur le blanc pur.
    #[test]
    fn lecretage_calme_le_halo_mais_garde_le_coeur_du_reflet() {
        // Sous le seuil, rien ne bouge : l'écrêtage ne doit pas ternir l'image.
        let bas = vec3(0.5, 0.4, 0.3);
        assert_eq!(ecreter(bas), bas, "l'écrêtage touche aux tons moyens");

        // Le halo (autour de 1,0) redescend franchement sous le blanc…
        let halo = ecreter(Vec3::splat(1.0)).x;
        assert!(halo < 0.85, "halo à {halo} : toujours au blanc");
        assert!(halo > ECRETAGE_SEUIL, "halo à {halo} : écrasé sous le seuil");

        // … tandis qu'un point chaud franc reste, lui, quasiment blanc.
        let coeur = ecreter(Vec3::splat(2.0)).x;
        assert!(coeur > 1.0, "cœur du reflet à {coeur} : le reflet a disparu");
        assert!(coeur > halo, "le cœur n'est pas plus clair que le halo");

        // Et la teinte est préservée : on comprime, on ne décolore pas.
        let teinte = ecreter(vec3(1.2, 0.6, 0.3));
        assert!(
            (teinte.y / teinte.x - 0.5).abs() < 1e-4 && (teinte.z / teinte.x - 0.25).abs() < 1e-4,
            "l'écrêtage a changé la teinte : {teinte:?}"
        );
    }
}
