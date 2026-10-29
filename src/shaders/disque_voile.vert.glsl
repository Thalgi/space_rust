#version 100
// Voile d'un champ de débris : l'annulus est en coordonnées LOCALES (plan Y,
// centre 0) ; l'orientation (axes) et la translation (centre) sont des
// uniforms — le mesh ne change jamais.
precision highp float;

attribute vec3 position;  // local : (x, 0, z)
attribute vec2 texcoord;  // (t radial 0..1, ang 0..1)
attribute vec4 color0;

varying vec2 vtc;
varying vec3 vpos; // position relative au centre (monde) -> ombre planétaire

uniform mat4 Model;
uniform mat4 Projection;
uniform vec3 centre;
uniform vec3 axe_u;
uniform vec3 axe_n;
uniform vec3 axe_v;
// Le voile en une passe (Opti-v2 O3) : plus d'une couche, l'annulus se pose
// sur la couche du côté de la caméra (`couche`, décalage en part du rayon) —
// tout rayon qui atteint une couche la traverse d'abord — et s'élargit de
// `marge` (monde) de chaque côté : vu de biais, il la traverse loin du point
// où il atteint les autres.
uniform float couche;
uniform float nb_couches;
uniform float interne;
uniform float externe;
uniform float marge;

void main() {
    vtc = texcoord;
    vec3 p = position;
    if (nb_couches > 1.5) {
        float r = mix(max(interne - marge, 0.0), externe + marge, texcoord.x);
        p.xz *= r / max(length(position.xz), 1e-6);
        p.y = couche * r;
    }
    vec3 rel = axe_u * p.x + axe_n * p.y + axe_v * p.z;
    vpos = rel;
    gl_Position = Projection * Model * vec4(centre + rel, 1.0);
}
