#version 100
// Disque d'accrétion (sous-lots 5.1 à 5.3). Annulus du voile, vertex shader
// propre (`disque_accretion.vert.glsl`), et ici tout ce qui fait qu'un
// disque d'accrétion n'est pas un anneau de matière éclairé du dehors.
//
// Les valeurs et les corrections viennent de
// `docs/conception/astres_v2_disque_accretion_2.md`, qui les chiffre.
precision highp float;

varying vec2 vtc;    // (t radial 0..1, ang 0..1)
varying float vdot;  // dot(position relative, tocam) -> moitiés avant/arrière
varying vec3 vpos;   // position **non pliée** dans le plan du disque
varying float vpli;  // part de pli appliquée ici (0 = à plat)

uniform float time;
uniform float seed;
uniform float moitie;      // -1 arrière seulement, +1 avant, 0 tout
uniform float alpha;       // opacité maximale
uniform vec3 chaud;        // bord interne — ordonné R > G > B
uniform vec3 froid;        // bord externe
uniform float eclat;       // intensité au bord interne (peut dépasser 1)
uniform float exp_couleur; // dureté du profil de couleur
uniform float exp_opacite; // dureté du profil d'opacité (plus douce)
uniform float beta;        // vitesse orbitale au bord interne (fraction de c)
uniform float beaming_max; // plafond du rapport de brillance entre les côtés
uniform vec2 cellules;     // (radiales, azimutales — entier)
uniform float spirale;     // enroulement du bruit, rad par e-fold de rayon
uniform float turbulence;
uniform float rotation;
uniform float r_ratio;     // externe / interne
uniform float bord_int;
uniform float bord_ext;
uniform vec3 axe_n;
uniform vec3 tocam;
uniform float pli;
uniform float pli_signe;
uniform float couche;      // part d'une nappe empilée : 1 = seule, 1/n si n nappes
uniform float halo;        // > 0 : cette passe est le faux bloom, sans bruit
uniform float anneau;      // > 0 : cette passe est l'anneau de lentille

float hash21(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

// Bruit de valeur **périodique sur x**. La périodicité est ce qui permet
// d'échantillonner en coordonnées (radial, azimutal) sans couture au
// raccord de l'angle : le hash se referme sur `periode` cellules.
float vnoise_cyl(vec2 p, float periode) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float i0 = mod(i.x, periode);
    float i1 = mod(i.x + 1.0, periode);
    float a = hash21(vec2(i0, i.y));
    float b = hash21(vec2(i1, i.y));
    float c = hash21(vec2(i0, i.y + 1.0));
    float d = hash21(vec2(i1, i.y + 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

void main() {
    if (moitie * vdot < 0.0) { discard; }

    // Le second arc du pli ne vit que là où il a vraiment soulevé la
    // matière : ailleurs, les deux copies se superposeraient exactement et
    // additionneraient leur éclat le long du raccord.
    float part_arc = 1.0;
    if (pli > 0.001 && pli_signe < 0.0) {
        part_arc = smoothstep(0.12, 0.55, vpli);
        if (part_arc < 0.004) { discard; }
    }

    float t = clamp(vtc.x, 0.0, 1.0);
    float rr = mix(1.0, r_ratio, t);

    // **Rotation képlérienne**, phase bornée. Non bornée, l'enroulement
    // finit par produire des rayures infiniment fines, et l'argument du hash
    // grandit au point de se quantifier.
    float tour = 6.2831853;
    float phase = mod(rotation * time, tour) / pow(rr, 1.5);
    float ang = fract(vtc.y - phase / tour);

    // **Profil radial, dédoublé.** La couleur suit un exposant raide (le
    // bord interne doit écraser le reste), l'opacité un exposant doux — un
    // seul exposant raide éteindrait le bord externe au point de le perdre.
    float chaleur = pow(rr, -exp_couleur);
    float densite = pow(rr, -exp_opacite);

    // **Bords asymétriques** : l'ISCO est une coupure franche, le bord
    // externe une dilution. Un fondu symétrique donne un anneau mou.
    densite *= smoothstep(0.0, max(bord_int, 0.001), t);
    densite *= 1.0 - smoothstep(1.0 - max(bord_ext, 0.001), 1.0, t);

    // **Turbulence anisotrope.** ⚠️ L'ancien mapping échantillonnait un
    // cercle en espace cartésien : isotrope par construction, donc incapable
    // de produire autre chose que des bulles, quel que soit le réglage. Ici
    // on échantillonne en (azimut, rayon) avec des cellules très allongées,
    // et on décale l'azimut en **spirale logarithmique** — sans quoi les
    // structures restent concentriques au lieu de s'enrouler.
    if (turbulence > 0.001 && halo < 0.001) {
        float periode = max(floor(cellules.y), 1.0);
        float ang_s = ang * periode - spirale * log(rr);
        vec2 p = vec2(ang_s, t * cellules.x);
        float n = vnoise_cyl(p, periode);
        n = mix(n, vnoise_cyl(p * 2.0 + 7.3 + seed, periode * 2.0), 0.45);
        // Crêtes affûtées : des filaments, pas des taches molles. Le
        // plancher garde un continuum — sans lui, c'est de la fourrure.
        n = 1.0 - abs(2.0 * n - 1.0);
        densite *= mix(1.0, 0.35 + 1.25 * n * n, turbulence);
    }

    // **Beaming Doppler en D³**, plafonné. La loi relativiste donne un
    // rapport de 8 à 10 entre les deux côtés — assez pour que l'ombre du
    // trou noir disparaisse, ce qui a fait retirer l'effet du film
    // Interstellar. Le plafond garde l'asymétrie sans perdre la silhouette.
    float faisceau = 1.0;
    float decalage = 0.0;
    if (beta > 0.001) {
        // ⚠️ `vpos` est la position **avant le pli**. Le beaming dépend de
        // la vitesse orbitale, donc de l'endroit où la matière se trouve
        // *vraiment* — pas de l'endroit où le pli l'a peinte. Calculé sur la
        // position pliée, le côté brillant se retrouvait posé au mauvais
        // endroit sur les arcs, et l'asymétrie cessait de correspondre au
        // point de vue : signalé à l'écran.
        vec3 radial = normalize(vpos - axe_n * dot(vpos, axe_n));
        vec3 vitesse = cross(axe_n, radial);
        float cos_t = dot(vitesse, tocam);
        float b = beta * pow(rr, -0.5); // l'intérieur file le plus vite
        float d = 1.0 / max(1.0 - b * cos_t, 0.05);
        faisceau = clamp(d * d * d, 1.0 / beaming_max, beaming_max);
        decalage = clamp((faisceau - 1.0) * 0.25, -0.5, 0.5);
    }

    // Couleur : on laisse l'intensité dépasser 1. Le clipping par canal fait
    // office de tone mapping — le rouge sature d'abord, puis le vert, puis
    // le bleu, et le cœur vire au blanc. C'est pour ça que `chaud` est
    // ordonné R > G > B.
    vec3 col = mix(froid, chaud, clamp(chaleur, 0.0, 1.0));
    col.b *= 1.0 + decalage;       // s'approche -> bleuit
    col.r *= 1.0 - decalage * 0.5; // s'éloigne -> rougit
    // ⚠️ **`couche` ne s'applique QU'À L'ALPHA.** Ce qui se dépose dans le
    // framebuffer vaut `col × a` : l'appliquer des deux côtés le divisait
    // par n **au carré**, soit neuf fois trop sombre à trois nappes. C'est
    // pour ça que le cœur ne saturait jamais en blanc.
    col *= eclat * chaleur * faisceau;

    float a = clamp(densite, 0.0, 1.0) * alpha * part_arc * couche;
    // (l'énergie déposée est `col × a` : diviser `a` par le nombre de nappes
    // suffit à ce que l'empilement rende exactement une nappe pleine)
    a *= clamp(faisceau, 0.3, 2.0);

    // Le faux bloom : large, faible, lisse. Il ne porte ni bruit ni bord
    // franc — c'est une lueur, pas de la matière.
    if (halo > 0.001) {
        a = clamp(densite, 0.0, 1.0) * halo * clamp(faisceau, 0.5, 1.6);
        col = mix(froid, chaud, 0.6) * eclat;
    }
    // L'anneau de lentille porte la couleur, le bruit et l'asymétrie du
    // disque — c'est une image de lui, pas un liseré décoratif.
    if (anneau > 0.001) {
        a *= anneau;
    }

    if (a < 0.003) { discard; }
    gl_FragColor = vec4(max(col, vec3(0.0)), clamp(a, 0.0, 1.0));
}
