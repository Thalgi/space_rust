#version 100
// Voile d'un champ de débris : l'annulus est en coordonnées LOCALES (plan Y,
// centre 0) ; l'orientation (axes) et la translation (centre) sont des
// uniforms — le mesh ne change jamais. `vdot` sert au rendu en deux moitiés
// autour du corps de la planète (discard dans le fragment shader).
precision highp float;

attribute vec3 position;  // local : (x, 0, z)
attribute vec2 texcoord;  // (t radial 0..1, ang 0..1)
attribute vec4 color0;

varying vec2 vtc;
varying float vdot;
varying vec3 vpos; // position relative au centre (monde) -> ombre planétaire

uniform mat4 Model;
uniform mat4 Projection;
uniform vec3 centre;
uniform vec3 axe_u;
uniform vec3 axe_n;
uniform vec3 axe_v;
uniform vec3 tocam;
// Prototype B (lot 12) : décalage de cette couche le long de la normale, en
// part du rayon local — le disque s'évase, plus épais au large. 0 = à plat.
uniform float couche;

void main() {
    vtc = texcoord;
    vec3 rel = axe_u * position.x + axe_n * position.y + axe_v * position.z;
    rel += axe_n * (couche * length(position.xz));
    vdot = dot(rel, tocam);
    vpos = rel;
    gl_Position = Projection * Model * vec4(centre + rel, 1.0);
}
