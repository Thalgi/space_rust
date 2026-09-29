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
uniform float interne;    // rayon du bord interne, l'échelle du pli
uniform float decalage;   // décalage vertical de cette nappe (épaisseur)

void main() {
    vtc = texcoord;
    vec3 rel = axe_u * position.x + axe_n * position.y + axe_v * position.z;
    vdot = dot(rel, tocam);
    // ⚠️ La position **dans le plan du disque**, capturée avant toute
    // déformation. Le fragment shader en tire la vitesse orbitale pour le
    // beaming : celle-ci dépend de l'endroit où la matière est *vraiment*,
    // pas de l'endroit où le pli l'a peinte. Prise après, le côté brillant
    // se retrouvait posé au mauvais endroit sur les arcs — signalé à
    // l'écran.
    vpos = rel;

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
        // ⚠️ **La borne basse est exactement 0.** Le découpage en moitiés se
        // fait sur `vdot = 0` ; si `k` n'y vaut pas zéro, la moitié arrière
        // y est déjà comprimée et relevée alors que la moitié avant y est
        // plate — les deux ne se rejoignent plus, et une **cassure
        // horizontale** apparaît de part et d'autre de l'astre. Elle valait
        // -0,15, ce qui laissait k ≈ 0,05 à la couture : signalé à l'écran.
        k = smoothstep(0.0, 0.95, -dot(normalize(rel), tocam));

        // ⚠️ **L'arc se COMPRIME vers l'astre, il ne s'en éloigne pas.**
        // La première version déplaçait proportionnellement à `length(rel)`
        // : le bord externe, le plus loin, était donc soulevé le plus haut,
        // et le disque se dépliait en deux gros lobes — signalé à l'écran.
        // L'image lentillée de la face arrière forme au contraire un **arc
        // fin à rayon apparent quasi constant**, indépendant de l'angle de
        // vue, calé sur le voisinage de l'ombre. On ramène donc la matière
        // vers un rayon de référence — celui du bord interne — au lieu de
        // l'étaler.
        float r = length(rel);
        vec3 vers = rel / max(r, 1e-4);
        // Vers le rayon de référence, d'autant plus que le point est
        // derrière l'astre : le bord externe est ramené, le bord interne à
        // peine touché.
        float vise = mix(r, interne * 1.25, k * 0.85);
        rel = vers * vise;
        // Puis relevé, d'une hauteur elle aussi calée sur l'échelle de
        // l'ombre et non sur l'étendue du disque.
        // ⚠️ **L'arc doit DÉGAGER l'ombre, sinon il n'existe pas.** Multiplié
        // par `assiette` (le sinus de l'inclinaison), le soulèvement
        // s'effondrait dès qu'on regardait le disque d'un peu haut : l'arc
        // retombait derrière le disque noir, qui le masquait, et il ne
        // restait qu'un liseré autour d'un dôme sombre — signalé à l'écran.
        // `assiette` ne sert plus qu'à **éteindre** le pli quand le disque
        // est vu de face, cas où il n'y a rien derrière l'astre à faire
        // passer par-dessus ; partout ailleurs le pli garde toute sa hauteur.
        float ouvert = smoothstep(0.04, 0.30, assiette);
        rel += haut * (pli * pli_signe * k * ouvert * interne);
    }
    vpli = k;

    // Épaisseur : la nappe est décalée le long de la normale du plan. Un
    // disque d'accrétion fait h/R ~ 0,05 a 0,1 -- il n'est jamais
    // infiniment mince comme un anneau de Saturne, et par la tranche il
    // doit montrer une bande, pas un cheveu.
    rel += axe_n * decalage;

    gl_Position = Projection * Model * vec4(centre + rel, 1.0);
}
