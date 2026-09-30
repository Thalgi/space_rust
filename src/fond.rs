use crate::astre::CameraInfo;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

const RAYON: f32 = 3000.0; // distance des étoiles (sous le far plane, au-delà des planètes)
const LOT: usize = 400;

/// Champ d'étoiles lointaines : billboards sur une sphère centrée sur la caméra
/// (on regarde depuis l'intérieur). Rendues sans test de profondeur, en premier,
/// donc toujours en arrière-plan quelle que soit la distance des objets.
/// Côté d'une face de la cubemap du fond, en texels. Six faces de 256² pèsent
/// 1,5 Mo — payé une fois, jamais par frame.
const COTE_CUBE: usize = 256;

pub struct Fond {
    etoiles: Vec<(Vec3, f32, Color)>, // direction, taille, couleur
    mat: Material,
    verts: Vec<Vertex>,
    inds: Vec<u16>,
    /// **Le même ciel, sous une forme qu'un shader sait lire** (lot 8).
    ///
    /// ⚠️ Gravée depuis `etoiles`, jamais retirée au hasard : c'est toute la
    /// question. Chaque vue tire son propre champ (400 à 900 étoiles selon
    /// l'écran), donc une cubemap construite à part montrerait **d'autres
    /// étoiles** que celles qu'on voit — et la lentille d'un trou noir
    /// afficherait dans son anneau un ciel qui n'existe nulle part.
    ///
    /// Construite à la demande : la plupart des vues n'ont pas de trou noir et
    /// n'ont donc aucune raison de la payer.
    cube: Option<Texture2D>,
}

/// **Téléverse les six faces en une vraie cubemap.**
///
/// ⚠️ macroquad n'expose que des `Texture2D`, mais ce n'est qu'une enveloppe
/// autour d'un identifiant miniquad : `Texture2D::from_miniquad_texture` permet
/// d'y ranger une cubemap créée par en dessous. Et miniquad lie la texture avec
/// **sa propre cible** au moment du dessin (`gl.rs`, `apply_bindings_from_slice`),
/// si bien qu'un `samplerCube` dans le shader reçoit bien un `GL_TEXTURE_CUBE_MAP`.
/// C'est ce qui rend le lot 8 possible sans copie d'écran, comme
/// `suivi/technique.md` l'avait noté.
fn televerser_cubemap(faces: &[Vec<u8>], cote: usize) -> Texture2D {
    use macroquad::miniquad::{
        FilterMode, TextureAccess, TextureFormat, TextureKind, TextureParams, TextureSource,
        TextureWrap,
    };
    // `TextureSource::Array` attend `[face][niveau de mip][octets]`. Un seul
    // niveau ici : pas de mipmaps, le ciel n'est jamais vu de loin.
    let niveaux: Vec<[&[u8]; 1]> = faces.iter().map(|f| [f.as_slice()]).collect();
    let src: Vec<&[&[u8]]> = niveaux.iter().map(|n| &n[..]).collect();
    let gl = unsafe { macroquad::window::get_internal_gl() };
    let id = gl.quad_context.new_texture(
        TextureAccess::Static,
        TextureSource::Array(&src),
        TextureParams {
            kind: TextureKind::CubeMap,
            format: TextureFormat::RGBA8,
            wrap: TextureWrap::Clamp,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            width: cote as u32,
            height: cote as u32,
            ..Default::default()
        },
    );
    Texture2D::from_miniquad_texture(id)
}

/// **Face de cubemap et coordonnées `(u, v)` d'une direction.**
///
/// Convention OpenGL, dans l'ordre `+X, -X, +Y, -Y, +Z, -Z`. ⚠️ Elle n'est pas
/// intuitive — deux axes sont retournés — et s'en écarter ne casse rien de
/// visible tant que le ciel est **seul** : on ne voit pas qu'un ciel d'étoiles
/// aléatoires est miroité. Ça ne se verrait que dans l'anneau du trou noir, où
/// le ciel dévié ne raccorderait pas le ciel autour. D'où [`direction_de`], son
/// inverse exact, et la garde d'aller-retour.
pub(crate) fn face_et_uv(dir: Vec3) -> (usize, f32, f32) {
    let a = dir.abs();
    let (face, sc, tc, ma) = if a.x >= a.y && a.x >= a.z {
        if dir.x > 0.0 {
            (0, -dir.z, -dir.y, a.x)
        } else {
            (1, dir.z, -dir.y, a.x)
        }
    } else if a.y >= a.z {
        if dir.y > 0.0 {
            (2, dir.x, dir.z, a.y)
        } else {
            (3, dir.x, -dir.z, a.y)
        }
    } else if dir.z > 0.0 {
        (4, dir.x, -dir.y, a.z)
    } else {
        (5, -dir.x, -dir.y, a.z)
    };
    let m = ma.max(1e-9);
    (face, (sc / m + 1.0) * 0.5, (tc / m + 1.0) * 0.5)
}

/// **L'inverse exact de [`face_et_uv`]**, et c'est cette exactitude qui se teste.
// ⚠️ N'existe **que** pour la garde d'aller-retour : c'est elle qui vérifie la
// convention de cubemap, et rien en production n'a besoin de l'inverse — le
// shader échantillonne par direction, pas par face et coordonnées.
#[cfg(test)]
pub(crate) fn direction_de(face: usize, u: f32, v: f32) -> Vec3 {
    let (sc, tc) = (u * 2.0 - 1.0, v * 2.0 - 1.0);
    let d = match face {
        0 => vec3(1.0, -tc, -sc),
        1 => vec3(-1.0, -tc, sc),
        2 => vec3(sc, 1.0, tc),
        3 => vec3(sc, -1.0, -tc),
        4 => vec3(sc, -tc, 1.0),
        _ => vec3(-sc, -tc, -1.0),
    };
    d.normalize()
}

/// **Grave le champ d'étoiles dans les six faces.** Fonction pure : elle rend
/// les octets, elle ne parle pas au GPU — c'est ce qui la rend testable.
///
/// ⚠️ **Additif et écrêté.** Deux étoiles qui tombent sur le même texel
/// s'ajoutent au lieu que la seconde efface la première : à 900 étoiles sur
/// 393 216 texels la collision est rare, mais l'écrasement ferait disparaître
/// une étoile du ciel dévié sans en retirer une du ciel dessiné.
pub(crate) fn graver_faces(etoiles: &[(Vec3, f32, Color)], cote: usize) -> Vec<Vec<u8>> {
    let mut faces = vec![vec![0u8; cote * cote * 4]; 6];
    for (dir, taille, col) in etoiles {
        let (f, u, v) = face_et_uv(*dir);
        let cx = (u * cote as f32 - 0.5).round() as i32;
        let cy = (v * cote as f32 - 0.5).round() as i32;
        // ⚠️ Une étoile du ciel est **plus petite qu'un texel** : à 3000 unités
        // son billboard couvre ~0,0016 rad, contre ~0,0061 pour un texel d'une
        // face de 256. Un seul texel, donc, et deux pour les plus grosses —
        // les étaler davantage ferait un ciel de taches dans l'anneau.
        let r = if *taille > 3.0 { 1 } else { 0 };
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x >= cote as i32 || y >= cote as i32 {
                    continue;
                }
                let i = (y as usize * cote + x as usize) * 4;
                for (k, c) in [col.r, col.g, col.b].iter().enumerate() {
                    faces[f][i + k] = faces[f][i + k].saturating_add((c * 255.0) as u8);
                }
                faces[f][i + 3] = 255;
            }
        }
    }
    faces
}

impl Fond {
    pub fn new(n: usize) -> Self {
        let mut etoiles = Vec::with_capacity(n);
        for _ in 0..n {
            let z: f32 = gen_range(-1.0, 1.0);
            let a: f32 = gen_range(0.0, std::f32::consts::TAU);
            let r = (1.0 - z * z).sqrt();
            let dir = vec3(r * a.cos(), z, r * a.sin());
            let lum: f32 = gen_range(0.35, 1.0);
            let t: f32 = gen_range(0.0, 1.0);
            let col = Color::new(lum * (0.85 + 0.15 * t), lum * 0.95, lum * (1.0 - 0.15 * t), 1.0);
            let taille: f32 = gen_range(1.2, 4.0);
            etoiles.push((dir, taille, col));
        }
        Self {
            etoiles,
            mat: material_etoiles(),
            verts: Vec::new(),
            inds: Vec::new(),
            cube: None,
        }
    }

    /// **Le ciel de cette vue, en cubemap**, pour qui doit l'échantillonner dans
    /// une direction quelconque — la lentille d'un trou noir (lot 8).
    ///
    /// Gravée au premier appel depuis les **mêmes** étoiles que celles qui sont
    /// dessinées, puis conservée.
    pub fn cubemap(&mut self) -> Texture2D {
        if let Some(t) = &self.cube {
            return t.clone();
        }
        let faces = graver_faces(&self.etoiles, COTE_CUBE);
        let t = televerser_cubemap(&faces, COTE_CUBE);
        self.cube = Some(t.clone());
        t
    }

    /// Recharge le material depuis les .glsl (hot-reload) sans toucher au champ d'étoiles.
    pub fn recharger_material(&mut self) {
        self.mat = material_etoiles();
    }

    pub fn draw(&mut self, cam: &CameraInfo) {
        self.verts.clear();
        self.inds.clear();
        let echelle = RAYON * 0.0004; // petites étoiles (points)
        // Centre de la sphère céleste **au repère de rendu** : l'œil, translaté
        // comme tout ce qui part au GPU. Calculé une fois hors de la boucle —
        // il ne dépend pas de l'étoile. Sans cette conversion, le champ entier
        // resterait posé à la coordonnée héliocentrique de la caméra alors que
        // la caméra 3D, elle, est à l'origine : le ciel se retrouverait à 460
        // unités derrière le dos de l'observateur.
        let oeil = cam.rel(cam.pos);
        gl_use_material(&self.mat);
        let mut q = 0;
        for (dir, taille, col) in &self.etoiles {
            let pos = oeil + *dir * RAYON;
            let s = *taille * echelle;
            let r2 = cam.right * s;
            let u2 = cam.up * s;
            let i0 = self.verts.len() as u16;
            self.verts.push(Vertex::new2(pos - r2 - u2, vec2(0.0, 0.0), *col));
            self.verts.push(Vertex::new2(pos + r2 - u2, vec2(1.0, 0.0), *col));
            self.verts.push(Vertex::new2(pos + r2 + u2, vec2(1.0, 1.0), *col));
            self.verts.push(Vertex::new2(pos - r2 + u2, vec2(0.0, 1.0), *col));
            self.inds
                .extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
            q += 1;
            if q >= LOT {
                flush(&mut self.verts, &mut self.inds);
                q = 0;
            }
        }
        flush(&mut self.verts, &mut self.inds);
        gl_use_default_material();
    }
}

fn material_etoiles() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: &crate::impostor::source("fond.vert.glsl", VERT),
            fragment: &crate::impostor::source("fond.frag.glsl", FRAG),
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::Always, // toujours peint -> arrière-plan
                depth_write: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap()
}

fn flush(verts: &mut Vec<Vertex>, inds: &mut Vec<u16>) {
    if inds.is_empty() {
        return;
    }
    let mesh = Mesh {
        vertices: std::mem::take(verts),
        indices: std::mem::take(inds),
        texture: None,
    };
    draw_mesh(&mesh);
    let mut v = mesh.vertices;
    v.clear();
    let mut i = mesh.indices;
    i.clear();
    *verts = v;
    *inds = i;
}

const VERT: &str = include_str!("shaders/fond.vert.glsl");

const FRAG: &str = include_str!("shaders/fond.frag.glsl");

#[cfg(test)]
mod tests_cubemap {
    use super::*;

    /// **Aller-retour exact entre une direction et sa place sur la cubemap.**
    ///
    /// ⚠️ C'est LA garde du lot 8. La convention OpenGL retourne deux axes
    /// selon la face, et s'en écarter ne casse **rien de visible** tant que le
    /// ciel est seul : personne ne voit qu'un champ d'étoiles aléatoires est
    /// miroité. Ça n'apparaîtrait que dans l'anneau du trou noir, où le ciel
    /// dévié ne raccorderait plus le ciel autour — un défaut qu'on mettrait des
    /// heures à attribuer, et qu'on corrigerait en tâtonnant sur des signes.
    #[test]
    fn une_direction_retrouve_sa_place_sur_la_cubemap() {
        // Les six axes, puis un balayage dense de la sphère : les axes tombent
        // pile sur les arêtes, là où le choix de face se joue.
        let mut dirs = vec![
            Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z,
        ];
        for i in 0..40 {
            for j in 0..40 {
                let z = -1.0 + 2.0 * (i as f32 + 0.5) / 40.0;
                let a = std::f32::consts::TAU * (j as f32 + 0.5) / 40.0;
                let r = (1.0 - z * z).max(0.0).sqrt();
                dirs.push(vec3(r * a.cos(), z, r * a.sin()));
            }
        }
        for d in dirs {
            let d = d.normalize();
            let (f, u, v) = face_et_uv(d);
            assert!(f < 6, "face {f} hors des six");
            assert!((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v), "({u}, {v}) hors face");
            let retour = direction_de(f, u, v);
            assert!(
                (retour - d).length() < 1e-4,
                "face {f} : {d:?} revient en {retour:?} — convention de cubemap fautive"
            );
        }
    }

    /// **Chaque face est atteinte**, et par la direction qu'on croit.
    ///
    /// Un aller-retour cohérent le resterait si deux faces étaient permutées
    /// entre elles : la garde ci-dessus ne le verrait pas.
    #[test]
    fn chaque_face_recoit_son_axe() {
        for (axe, attendue) in [
            (Vec3::X, 0), (Vec3::NEG_X, 1), (Vec3::Y, 2),
            (Vec3::NEG_Y, 3), (Vec3::Z, 4), (Vec3::NEG_Z, 5),
        ] {
            let (f, u, v) = face_et_uv(axe);
            assert_eq!(f, attendue, "l'axe {axe:?} tombe sur la face {f}");
            // Un axe vise le CENTRE de sa face.
            assert!((u - 0.5).abs() < 1e-5 && (v - 0.5).abs() < 1e-5, "({u}, {v}) hors du centre");
        }
    }

    /// **Le ciel gravé est celui qu'on dessine, étoile pour étoile.**
    ///
    /// ⚠️ Toute la valeur de la cubemap tient là. Un champ retiré au hasard
    /// pour l'occasion afficherait dans l'anneau des étoiles qui ne sont nulle
    /// part ailleurs — et rien, à l'écran, ne dirait que ce sont les mauvaises.
    #[test]
    fn chaque_etoile_du_champ_est_gravee_dans_sa_face() {
        let cote = 64;
        let etoiles: Vec<(Vec3, f32, Color)> = (0..200)
            .map(|k| {
                let z = -1.0 + 2.0 * (k as f32 + 0.5) / 200.0;
                let a = 2.399_963 * k as f32; // angle d'or : bien réparti
                let r = (1.0 - z * z).max(0.0).sqrt();
                (vec3(r * a.cos(), z, r * a.sin()).normalize(), 2.0, WHITE)
            })
            .collect();
        let faces = graver_faces(&etoiles, cote);
        assert_eq!(faces.len(), 6);
        for (dir, _, _) in &etoiles {
            let (f, u, v) = face_et_uv(*dir);
            let x = (u * cote as f32 - 0.5).round().clamp(0.0, cote as f32 - 1.0) as usize;
            let y = (v * cote as f32 - 0.5).round().clamp(0.0, cote as f32 - 1.0) as usize;
            let i = (y * cote + x) * 4;
            assert!(faces[f][i + 3] > 0, "étoile {dir:?} absente de la face {f}");
        }
        // Et le ciel reste **noir** ailleurs : une gravure qui déborderait
        // éclaircirait le fond au lieu d'y poser des points.
        let allumes: usize = faces.iter().flat_map(|f| f.chunks(4)).filter(|p| p[3] > 0).count();
        assert!(allumes <= etoiles.len() * 9, "{allumes} texels pour {} étoiles", etoiles.len());
        assert!(allumes >= etoiles.len() / 2, "seulement {allumes} texels allumés");
    }
}
