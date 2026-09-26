#version 100
// Vertex shader du disque d'accrétion (sous-lot 5.2) : le **pli**.
//
// Le disque reste l'annulus plat du voile, en coordonnées locales et figé ;
// c'est ici qu'on le déforme. Un vertex shader à part plutôt qu'un uniform de
// plus sur celui du voile : les anneaux planétaires n'ont rien à faire d'un
// pli, et le voile a déjà assez de vocabulaire qui ne le concerne pas.
// Vertex et fragment forment un seul pipeline — ça ne coûte rien de plus.
//
// ## Ce que le pli imite
//
// Autour d'un trou noir, la lumière de la face ARRIÈRE du disque ne file pas
// droit : elle contourne l'astre. Cette face, physiquement cachée derrière
// lui, apparaît donc AU-DESSUS de lui — et sa face inférieure au-dessous.
// C'est cette silhouette, l'anneau qui enjambe l'astre, que tout le monde a
// en tête ; sans elle un disque incliné n'est qu'un anneau de Saturne autour
// d'une boule noire.
//
// ⚠️ **C'est un trucage, pas une lentille.** On ne dévie rien : on relève la
// moitié arrière du maillage. Ça tient tant qu'on regarde le disque depuis
// l'extérieur ; vu dans son plan ou de très près, la supercherie se voit. La
// vraie déviation est un chantier de rendu à part (`astres_v2.md` §7).
precision highp float;

attribute vec3 position;  // local : (x, 0, z)
attribute vec2 texcoord;  // (t radial 0..1, ang 0..1)
attribute vec4 color0;

varying vec2 vtc;
varying float vdot;
varying vec3 vpos;
varying float vpli;  // part de pli appliquée ici (0 = à plat)

uniform mat4 Model;
uniform mat4 Projection;
uniform vec3 centre;
uniform vec3 axe_u;
uniform vec3 axe_n;
uniform vec3 axe_v;
uniform vec3 tocam;
uniform float pli;        // amplitude du pli (0 = disque plat)
uniform float pli_signe;  // +1 = l'arc du dessus, -1 = celui du dessous

void main() {
    vtc = texcoord;
    vec3 rel = axe_u * position.x + axe_n * position.y + axe_v * position.z;
    vdot = dot(rel, tocam);

    float k = 0.0;
    if (pli > 0.001) {
        // Le « haut » du disque tel qu'il apparaît à l'écran : la normale du
        // plan, débarrassée de sa part dirigée vers la caméra. Sa longueur
        // dit à quel point on voit le disque par la tranche — nulle quand on
        // le regarde de face, et il n'y a alors **rien derrière l'astre** à
        // faire passer par-dessus.
        vec3 perp = axe_n - tocam * dot(axe_n, tocam);
        float assiette = length(perp);
        vec3 haut = perp / max(assiette, 1e-4);

        // 1 droit derrière l'astre, 0 sur les côtés : le pli s'éteint là où
        // la matière n'a jamais été cachée, ce qui **raccorde** l'arc au
        // disque plat sans couture.
        k = smoothstep(-0.15, 0.95, -dot(normalize(rel), tocam));
        rel += haut * (pli * pli_signe * k * assiette * length(rel));
    }
    vpli = k;

    vpos = rel;
    gl_Position = Projection * Model * vec4(centre + rel, 1.0);
}
