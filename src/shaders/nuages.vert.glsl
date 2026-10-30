#version 100
// Les boules des nuages, figées sur le GPU (Opti-v2 O7, `disque::nuages_gpu`).
// Chaque sommet porte ce qui ne change pas ; le shader fait tourner le nuage,
// l'éclaire et pose le quad face caméra — exactement ce que faisait le CPU
// (`calculer_nuages`, `eclairement_boule`, `impostor::push_quad`).
precision highp float;

attribute vec4 orbite;  // r, hauteur, angle à t = 0, vitesse angulaire
attribute vec4 decal;   // décalage dans la grappe (repère u, n, v), demi-taille
attribute vec4 couleur; // teinte, alpha avant l'amorti
attribute vec2 coin;    // ±1, ±1

varying vec2 uv;
varying vec4 vcol;

uniform mat4 mvp;
uniform vec3 centre;    // centre du disque, repère de rendu
uniform vec3 axe_u;
uniform vec3 axe_n;
uniform vec3 axe_v;
uniform vec3 cam_right;
uniform vec3 cam_up;
uniform float temps;
uniform float amorti;   // `attenuation_globale`, un seul facteur par image

void main() {
    float ang = orbite.z + orbite.w * temps;
    vec3 rel = axe_u * (cos(ang) * orbite.x) + axe_n * orbite.y + axe_v * (sin(ang) * orbite.x);
    vec3 dw = axe_u * decal.x + axe_n * decal.y + axe_v * decal.z;
    // `eclairement_boule` : la face tournée vers l'étoile, au centre du disque.
    float cote = 0.0;
    if (dot(dw, dw) > 0.0 && dot(rel, rel) > 0.0) {
        cote = dot(normalize(dw), normalize(-rel));
    }
    float lum = 0.35 + 0.65 * (0.5 + 0.5 * cote);
    float a = couleur.a * amorti;
    vcol = vec4(couleur.rgb * lum * a, a);
    uv = coin * 0.5 + 0.5;
    vec3 pos = centre + rel + dw + (cam_right * coin.x + cam_up * coin.y) * decal.w;
    gl_Position = mvp * vec4(pos, 1.0);
}
