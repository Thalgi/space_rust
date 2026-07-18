#version 100
// Profil procédural du voile (voir CONCEPTION_CEINTURES.md §2.3). Les 5 styles
// de l'ex-anneau V1 sont des jeux d'uniforms (presets DisqueConfig) : plateau,
// bandes/lacunes signées, granulation cellulaire, arcs, émissif, rotation
// différentielle képlérienne.
precision highp float;

varying vec2 vtc;    // (t radial 0..1, ang 0..1)
varying float vdot;  // dot(position relative, tocam) -> moitiés avant/arrière
varying vec3 vpos;   // position relative au centre (monde)

uniform float time;
uniform float seed;
uniform float moitie;        // -1 arrière seulement, +1 avant, 0 tout
uniform float alpha;         // opacité maximale
uniform vec3 couleur;        // bord externe
uniform vec3 couleur2;       // bord interne
uniform float plateau;       // densité de base (1 = annulus plein)
uniform float alpha_interne; // facteur d'alpha au bord interne (anneau C)
uniform float bord;          // largeur des bords doux
uniform float granulation;   // 0 lisse .. 1 granuleux épars
uniform float gran_seuil;
uniform vec2 gran_freq;      // (fréquence spatiale, poids de l'octave fine 0..1)
uniform float arcs;          // 0 anneau complet .. 1 arcs isolés
uniform float emissif;       // lueur chaude interne (disques proto*)
uniform float rotation;      // vitesse angulaire au bord interne (rad/s)
uniform float r_ratio;       // externe / interne (rotation différentielle)
uniform vec4 bandes[4];      // (t, demi-largeur, profondeur signée, ondulation)
uniform vec4 lacune_phase;   // phase orbitale du corps de chaque lacune (rad)
uniform vec3 dir_lumiere;    // direction de propagation de la lumière au centre
uniform float face_lum;      // 1 = face éclairée du plan, 0.55 = face nuit
uniform float ombre_rayon;   // rayon du corps central (0 = pas d'ombre)
uniform vec3 lum_couleur;    // couleur * intensité de l'étoile primaire

float hash(float x) {
    return fract(sin(x * 12.9898) * 43758.5453);
}

float hash21(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

// Bruit de valeur 2D lissé : granulation ISOTROPE en espace disque (les
// cellules polaires brutes donnaient un damier géant au bord externe).
float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float gauss(float x, float c, float w) {
    float d = (x - c) / w;
    return exp(-d * d);
}

void main() {
    // Rendu en deux moitiés autour du corps (planètes annelées).
    if (moitie > 0.5 && vdot < 0.0) discard;
    if (moitie < -0.5 && vdot >= 0.0) discard;

    float t = vtc.x;
    // Rotation différentielle : Kepler ~ r^-1.5 (l'intérieur file, le bord traîne).
    float rr = mix(1.0, r_ratio, t);
    float ang = fract(vtc.y - rotation * time / (6.2831853 * pow(rr, 1.5)));

    // Densité de base : plateau borné par des bords doux + atténuation interne.
    float env = smoothstep(0.0, bord, t) * (1.0 - smoothstep(1.0 - bord, 1.0, t));
    float dens = plateau * mix(alpha_interne, 1.0, smoothstep(0.0, 0.25, t));

    // Bandes signées : lacune (z > 0, creusée par une lune/proto-planète) ou
    // surdensité (z < 0, bande brillante). Une lacune VIVANTE (w > 0) ondule :
    // festons sur les bords, figés dans le référentiel du corps perturbateur
    // (sin(K*(angle - phase)) -> les vagues voyagent avec la lune, comme les
    // festons de Daphnis, pas avec la matière de l'anneau).
    for (int i = 0; i < 4; i++) {
        vec4 b = bandes[i];
        if (b.y <= 0.0001) continue;
        float largeur = b.y;
        if (b.z > 0.0 && b.w > 0.001) {
            float ph = vtc.y * 6.2831853 - lacune_phase[i];
            largeur *= 1.0 + 0.55 * b.w * sin(18.0 * ph);
            // Crête de matière soulevée juste au bord (sillage vertical écrasé
            // en surbrillance) : renforce la lisibilité des festons.
            float bordf = gauss(t, b.x, largeur * 1.6) - gauss(t, b.x, largeur);
            dens += 0.25 * b.w * bordf * (0.5 + 0.5 * sin(18.0 * ph + 1.6));
        }
        float g = gauss(t, b.x, largeur);
        if (b.z > 0.0) {
            dens *= 1.0 - b.z * g;
        } else {
            dens += (-b.z) * g;
        }
    }
    dens *= env;

    // Arcs angulaires (Neptune) : 3 arcs groupés, semés par la graine.
    if (arcs > 0.001) {
        float acc = 0.0;
        for (int k = 0; k < 3; k++) {
            float fk = float(k);
            float ck = fract(0.08 + hash(seed + fk * 7.31) * 0.12 + fk * 0.09);
            float d = abs(ang - ck);
            d = min(d, 1.0 - d);
            acc += gauss(d, 0.0, 0.022);
        }
        dens *= mix(1.0, min(acc, 1.0) + 0.07, arcs);
    }

    // Granulation : bruit de valeur en coordonnées LOCALES du disque
    // (isotrope quel que soit le rayon). Échantillonné sur l'angle animé ->
    // la matière tourne ET se cisaille avec la rotation différentielle.
    if (granulation > 0.001) {
        float angw = ang * 6.2831853;
        vec2 p = vec2(cos(angw), sin(angw)) * (rr / r_ratio); // bord externe = rayon 1
        vec2 pn = p * gran_freq.x + seed;
        float nse = vnoise(pn);
        nse = mix(nse, vnoise(pn * 2.7 + 13.1), gran_freq.y * 0.5);
        float m = smoothstep(gran_seuil - 0.18, gran_seuil + 0.22, nse) * 1.3;
        dens *= mix(1.0, m, granulation);
    }

    // Stries radiales fines (texture d'anneau).
    dens *= 0.86 + 0.14 * hash(floor(t * 130.0) + seed);

    float a = clamp(dens, 0.0, 1.0) * alpha;
    if (a < 0.004) discard;

    // Couleur : gradient radial + variation de bande + émissif interne.
    vec3 col = mix(couleur2, couleur, t);
    col *= 0.8 + 0.25 * hash(floor(t * 47.0) + seed * 1.7);

    // Éclairage : teinte de l'étoile + face jour/nuit du plan de l'anneau.
    col *= clamp(lum_couleur, 0.0, 1.5) * face_lum;

    // Ombre du corps central (planète annelée) : cylindre projeté le long de
    // la lumière, bord doux (pénombre). LE détail qui vend un Saturne.
    if (ombre_rayon > 0.001) {
        float along = dot(vpos, dir_lumiere);
        if (along > 0.0) {
            float perp = length(vpos - dir_lumiere * along);
            float ombre = smoothstep(ombre_rayon * 0.8, ombre_rayon * 1.08, perp);
            col *= mix(0.16, 1.0, ombre);
        }
    }

    // L'émissif (disques proto*) ne subit ni face nuit ni ombre : il émet.
    col += emissif * exp(-t * 2.6) * couleur2;

    gl_FragColor = vec4(col, a);
}
