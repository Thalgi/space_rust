#version 100
// Profil procédural du voile (voir CONCEPTION_CEINTURES.md §2.3). Les 5 styles
// de l'ex-anneau V1 sont des jeux d'uniforms (presets DisqueConfig) : plateau,
// bandes/lacunes signées, granulation cellulaire, arcs, émissif, rotation
// différentielle képlérienne.
precision highp float;

varying vec2 vtc;    // (t radial 0..1, ang 0..1)
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
uniform float nuages;        // prototype A (lot 12) : amas bouillonnants, 0 = aucun
// **Le voile en une passe** (Opti-v2 O3) : plus d'une couche, chaque pixel
// du plan suit son rayon de vue jusqu'à chaque couche et les compose lui-même.
uniform float nb_couches;    // 1 = le voile à plat, lu sur le maillage
uniform vec4 couches[8];     // par couche : (alpha, décalage h/r, graine, —)
uniform vec3 cam_rel;        // caméra, depuis le centre
uniform vec3 tocam;          // vers la caméra -> moitiés avant/arrière
uniform vec3 axe_u;
uniform vec3 axe_n;
uniform vec3 axe_v;
uniform float interne;
uniform float externe;

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

// **Cellules 2D** (prototype A, lot 12) : la recette de l'ébullition des
// géantes rouges (`soleil.frag.glsl`), ramenée au plan du disque. Distance au
// centre le plus proche (x), au second (y), identité du plus proche (z).
vec3 cellules2(vec2 p, float t) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    float f1 = 8.0;
    float f2 = 8.0;
    float id = 0.0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            vec2 g = vec2(float(x), float(y));
            vec2 h = vec2(hash21(i + g), hash21(i + g + 17.13));
            vec2 r = g + 0.5 + 0.38 * sin(t + 6.2831 * h) - f;
            float dd = dot(r, r);
            if (dd < f1) {
                f2 = f1;
                f1 = dd;
                id = h.x;
            } else if (dd < f2) {
                f2 = dd;
            }
        }
    }
    return vec3(sqrt(f1), sqrt(f2), id);
}

// **Un amas de poussière qui vit** : il se forme, gonfle, se dissipe, et
// recommence — chacun à son rythme. Cycle refermé (comme les bulles des
// géantes) : pas de saut au recommencement.
float amas(vec2 p, float t) {
    vec3 c = cellules2(p, t * 0.2);
    float vie = fract(t * 0.04 + c.z * 13.7);
    float gonfle = smoothstep(0.0, 0.55, vie) * (1.0 - smoothstep(0.65, 1.0, vie));
    float coeur = 1.0 - smoothstep(0.0, mix(0.15, 0.6, gonfle), c.x);
    float interieur = smoothstep(0.0, 0.25, c.y - c.x);
    return coeur * gonfle * interieur;
}

float gauss(float x, float c, float w) {
    float d = (x - c) / w;
    return exp(-d * d);
}

// **Une couche du voile** au point `pos` (depuis le centre) : couleur déjà
// bornée à [0, 1] — ce que la cible en ferait — et alpha, 0 si rien.
// `t` radial 0..1, `a0` angle 0..1 au repos, `sd` graine, `alpha_c` opacité.
vec4 voile(float t, float a0, vec3 pos, float sd, float alpha_c) {
    // Rendu en deux moitiés autour du corps (planètes annelées).
    float vd = dot(pos, tocam);
    if (moitie > 0.5 && vd < 0.0) return vec4(0.0);
    if (moitie < -0.5 && vd >= 0.0) return vec4(0.0);

    // Rotation différentielle : Kepler ~ r^-1.5 (l'intérieur file, le bord traîne).
    float rr = mix(1.0, r_ratio, t);
    float ang = fract(a0 - rotation * time / (6.2831853 * pow(rr, 1.5)));

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
            float ph = a0 * 6.2831853 - lacune_phase[i];
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
            float ck = fract(0.08 + hash(sd + fk * 7.31) * 0.12 + fk * 0.09);
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
        vec2 pn = p * gran_freq.x + sd;
        float nse = vnoise(pn);
        nse = mix(nse, vnoise(pn * 2.7 + 13.1), gran_freq.y * 0.5);
        float m = smoothstep(gran_seuil - 0.18, gran_seuil + 0.22, nse) * 1.3;
        dens *= mix(1.0, m, granulation);
    }

    // **Prototype A** (lot 12) : des amas de poussière qui se forment et se
    // dissipent. Échantillonnés sur l'angle animé, comme la granulation : ils
    // tournent ET se cisaillent avec la rotation différentielle.
    if (nuages > 0.001) {
        float angn = ang * 6.2831853;
        vec2 pa = vec2(cos(angn), sin(angn)) * (rr / r_ratio) * 6.0 + sd;
        float h = amas(pa, time) + 0.5 * amas(pa * 2.3 + 7.7, time * 1.3);
        dens *= mix(1.0, 0.3 + 1.6 * h, nuages);
    }

    // Stries radiales fines (texture d'anneau).
    dens *= 0.86 + 0.14 * hash(floor(t * 130.0) + sd);

    float a = clamp(dens, 0.0, 1.0) * alpha_c;
    if (a < 0.004) return vec4(0.0);

    // Couleur : gradient radial + variation de bande + émissif interne.
    vec3 col = mix(couleur2, couleur, t);
    col *= 0.8 + 0.25 * hash(floor(t * 47.0) + sd * 1.7);

    // Éclairage : teinte de l'étoile + face jour/nuit du plan de l'anneau.
    col *= clamp(lum_couleur, 0.0, 1.5) * face_lum;

    // Ombre du corps central (planète annelée) : cylindre projeté le long de
    // la lumière, bord doux (pénombre). LE détail qui vend un Saturne.
    if (ombre_rayon > 0.001) {
        float along = dot(pos, dir_lumiere);
        if (along > 0.0) {
            float perp = length(pos - dir_lumiere * along);
            float ombre = smoothstep(ombre_rayon * 0.8, ombre_rayon * 1.08, perp);
            col *= mix(0.16, 1.0, ombre);
        }
    }

    // L'émissif (disques proto*) ne subit ni face nuit ni ombre : il émet.
    col += emissif * exp(-t * 2.6) * couleur2;

    return vec4(clamp(col, 0.0, 1.0), clamp(a, 0.0, 1.0));
}

// **Où le rayon de vue croise une couche** : la couche décalée de `c` part
// du rayon est un cône, hauteur = c × distance à l'axe. `o` l'origine du
// rayon, `d` sa direction (unitaire), depuis le centre ; le point dans `x`.
// Vu du côté de la caméra, un rayon ne croise chaque couche qu'une fois —
// sauf à raser le disque (sous ~3°), là où les couches sont effacées.
bool croiser(vec3 o, vec3 d, float c, out vec3 x) {
    float on = dot(o, axe_n);
    float dn = dot(d, axe_n);
    if (abs(c) < 1e-6) {
        if (abs(dn) < 1e-6) return false;
        float s = -on / dn;
        x = o + d * s;
        return s > 0.0;
    }
    vec3 op = o - axe_n * on;
    vec3 dp = d - axe_n * dn;
    float c2 = c * c;
    float qa = dn * dn - c2 * dot(dp, dp);
    float qb = 2.0 * (on * dn - c2 * dot(op, dp));
    float qc = on * on - c2 * dot(op, op);
    float s = -1.0;
    if (abs(qa) < 1e-9) {
        if (abs(qb) > 1e-9) s = -qc / qb;
    } else {
        float disc = qb * qb - 4.0 * qa * qc;
        if (disc < 0.0) return false;
        float r = sqrt(disc);
        float s1 = (-qb - r) / (2.0 * qa);
        float s2 = (-qb + r) / (2.0 * qa);
        // La plus proche des racines devant la caméra, du bon côté du plan
        // (l'équation élevée au carré admet aussi le cône opposé).
        float lo = min(s1, s2);
        float hi = max(s1, s2);
        if (lo > 0.0 && (on + lo * dn) * c >= 0.0) s = lo;
        else if (hi > 0.0 && (on + hi * dn) * c >= 0.0) s = hi;
    }
    if (s <= 0.0) return false;
    x = o + d * s;
    return true;
}

void main() {
    if (nb_couches < 1.5) {
        vec4 v = voile(vtc.x, vtc.y, vpos, seed, alpha);
        if (v.a <= 0.0) discard;
        gl_FragColor = v;
        return;
    }
    // Les couches composées ici dans l'ordre où elles se dessinaient, une
    // passe chacune : la k-ième PAR-DESSUS les précédentes. `c` + `reste` ×
    // fond est ce que donnaient les passes successives.
    vec3 d = normalize(vpos - cam_rel);
    vec3 c = vec3(0.0);
    float reste = 1.0;
    for (int k = 0; k < 8; k++) {
        if (float(k) >= nb_couches) break;
        vec4 p = couches[k];
        if (p.x <= 0.0) continue;
        vec3 x;
        if (!croiser(cam_rel, d, p.y, x)) continue;
        vec2 q = vec2(dot(x, axe_u), dot(x, axe_v));
        float t = (length(q) - interne) / (externe - interne);
        if (t < 0.0 || t > 1.0) continue;
        vec4 v = voile(t, fract(atan(q.y, q.x) / 6.2831853), x, p.z, p.x);
        c = v.a * v.rgb + (1.0 - v.a) * c;
        reste *= 1.0 - v.a;
    }
    if (reste > 0.996) discard;
    gl_FragColor = vec4(c / (1.0 - reste), 1.0 - reste);
}
