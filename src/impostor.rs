//! Socle de rendu commun aux corps « impostor » (1 quad face-caméra + 1 shader
//! qui reconstruit une sphère). Mutualise la géométrie du quad, le vertex shader
//! (`shaders/impostor.vert.glsl`) et la pose des uniforms partagés par tous les
//! corps. Chaque type ajoute ensuite ses propres uniforms + son fragment shader.

use crate::astre::CameraInfo;
use macroquad::models::Vertex;
use macroquad::prelude::*;

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

/// Empile un quad face-caméra centré sur `centre`, de demi-taille `taille`.
pub fn push_quad(
    verts: &mut Vec<Vertex>,
    inds: &mut Vec<u16>,
    centre: Vec3,
    right: Vec3,
    up: Vec3,
    taille: f32,
    couleur: Color,
) {
    let i0 = verts.len() as u16;
    let r = right * taille;
    let u = up * taille;
    verts.push(Vertex::new2(centre - r - u, vec2(0.0, 0.0), couleur));
    verts.push(Vertex::new2(centre + r - u, vec2(1.0, 0.0), couleur));
    verts.push(Vertex::new2(centre + r + u, vec2(1.0, 1.0), couleur));
    verts.push(Vertex::new2(centre - r + u, vec2(0.0, 1.0), couleur));
    inds.extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);
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
