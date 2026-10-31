#version 100
// Le flot bipolaire sur le GPU (Opti-v2 O7c, `disque::nuages_gpu`) : la copie
// de `flot::animer`. Chaque sommet porte une boule fixe (`flot::BouleFixe`) ;
// le shader la fait couler le long de sa trajectoire et pose son quad.
// ⚠️ Toute retouche du flot se fait des DEUX côtés : `flot.rs` (que ses tests
// éprouvent) et ici.
precision highp float;

attribute vec4 boule;   // famille (0 jet, 1 cône, 2 nœud, 3 paroi), côté ±1, s0, —
attribute vec4 tirage;  // k[0], k[1], k[2], —
attribute vec2 coin;

varying vec2 uv;
varying vec4 vcol;

uniform mat4 mvp;
uniform vec3 centre;
uniform vec3 axe_u;
uniform vec3 axe_n;
uniform vec3 axe_v;
uniform vec3 cam_right;
uniform vec3 cam_up;
uniform float temps;
uniform vec4 lobe_haut;  // `flot::lobe` du dessus : longueur, ouverture, jet, cône
uniform vec4 lobe_bas;   // et du dessous
uniform vec2 parois;     // éclat des parois, dessus et dessous
uniform vec4 flot;       // rayon de l'astre, finesse, asymétrie, forme
uniform float longueur;  // la longueur du flot, avant les lobes (la précession)

#define TAU 6.2831853
#define DEPART_PAROIS 0.15

// `flot::rayon_cavite`
float rayon_cavite(float l, float ouverture, float forme, float z) {
    float o = ouverture > 0.0 ? ouverture : 1.0;
    z = min(abs(z), l);
    float parabole = 0.45 * sqrt(l * z);
    float bulbe = 0.38 * l * sqrt(max(sin(3.14159265 * z / l), 0.0));
    return o * (parabole + (bulbe - parabole) * forme);
}

// `flot::hache_plaque`
float hache_plaque(float c) {
    float p = fract(c * 0.1031);
    p *= p + 33.33;
    p *= p + p;
    return fract(p);
}

void main() {
    float cote = boule.y;
    vec4 lb = cote > 0.0 ? lobe_haut : lobe_bas;
    float l = lb.x;
    float ra = flot.x;
    float finesse = flot.y;
    float forme = flot.w;
    vec3 p;
    float taille;
    vec3 c;
    float a;
    if (boule.x < 0.5) {
        // Le jet.
        float s = fract(boule.z + temps * 0.25);
        float z = ra + s * (l - ra);
        float ecart = 0.012 * l * (0.3 + s);
        float an = tirage.x * TAU;
        p = vec3(cos(an) * ecart * tirage.y, cote * z, sin(an) * ecart * tirage.y);
        taille = 0.012 * l;
        c = vec3(0.85, 0.9, 1.0);
        a = lb.z * 0.4 * sqrt(1.0 - s);
    } else if (boule.x < 1.5) {
        // J1 — le cône.
        float s = fract(boule.z + temps * 0.04);
        float z = s * l;
        float rho = rayon_cavite(l, lb.y, forme, z) * 0.9 * sqrt(tirage.x);
        float an = tirage.y * TAU;
        p = vec3(cos(an) * rho, cote * z, sin(an) * rho);
        c = mix(vec3(1.0, 0.7, 0.45), vec3(0.55, 0.35, 0.22), s);
        float monte = min(s / 0.12, 1.0);
        taille = 0.025 * l + 0.05 * z;
        a = lb.w * 0.32 * (1.0 - s) * monte * monte;
    } else if (boule.x < 2.5) {
        // J2 — un nœud.
        float s = fract(boule.z + temps * 0.05);
        float z = ra * 2.0 + s * l;
        float rk = 0.05 * l * (0.5 + s);
        float fondu = (1.0 - s) * min(s * 8.0, 1.0);
        float al = tirage.x * 1.1;
        float phi = tirage.y * TAU;
        p = vec3(sin(al) * cos(phi) * rk, cote * (z - (1.0 - cos(al)) * rk * 1.6), sin(al) * sin(phi) * rk);
        float apex = 1.0 - al / 1.1;
        taille = rk * 0.4;
        c = mix(vec3(1.0, 0.45, 0.42), vec3(1.0, 0.85, 0.8), apex * 0.6);
        a = fondu * (0.45 + 0.55 * apex);
    } else {
        // J3 / J5 — une paroi.
        float epaisseur = 0.18 * max(1.0 - finesse, 0.05);
        float cote_eclaire = 0.7 + (cote > 0.0 ? 0.0 : 2.4);
        float s = pow(fract(boule.z + temps * 0.01), 1.5);
        float depart = DEPART_PAROIS * (1.0 - forme);
        float z = (depart + s * (1.0 - depart)) * l;
        float phi = tirage.x * TAU;
        float rho = rayon_cavite(l, lb.y, forme, z) * (1.0 + epaisseur * (tirage.y - 0.5));
        float monte = forme > 0.5 ? 1.0 : min(s / 0.3, 1.0);
        float lum = monte * monte * (3.0 - 2.0 * monte) * pow(1.0 - s, 0.7);
        if (finesse > 0.0) {
            float face = pow(0.5 + 0.5 * cos(phi - cote_eclaire), 3.0);
            float plaque = clamp((hache_plaque(floor(phi * 6.0) + floor(s * 10.0) * 13.0 + tirage.z) - 0.35) / 0.65, 0.0, 1.0);
            lum *= face * plaque;
            if (lum < 0.02) {
                // Éteinte : un quad réduit à un point, hors champ — rien à remplir.
                gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
                uv = vec2(0.0);
                vcol = vec4(0.0);
                return;
            }
        }
        p = vec3(cos(phi) * rho, cote * z, sin(phi) * rho);
        taille = l * (0.018 + 0.024 * (1.0 - finesse)) * (0.4 + s);
        c = vec3(1.0, 0.85, 0.65);
        a = (cote > 0.0 ? parois.x : parois.y) * (0.6 + 0.6 * finesse) * lum;
    }
    // La précession courbe le flot en S.
    if (flot.z > 0.0) {
        float y = abs(p.y) / longueur;
        p.x += cote * flot.z * 0.12 * longueur * y * y;
    }
    vec3 rel = axe_u * p.x + axe_n * p.y + axe_v * p.z;
    vcol = vec4(c * a, a);
    uv = coin * 0.5 + 0.5;
    gl_Position = mvp * vec4(centre + rel + (cam_right * coin.x + cam_up * coin.y) * taille, 1.0);
}
