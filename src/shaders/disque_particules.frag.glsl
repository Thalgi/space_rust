#version 100
// Micro-impostor de débris : silhouette PATATOÏDE (lobes basse fréquence par
// graine) + éclairage de pseudo-sphère bosselée. Chaque caillou a du volume,
// un terminateur et des facettes — sans géométrie supplémentaire.
precision highp float;

varying vec4 vcol;
varying float vside;
varying vec2 vp;      // -1..1 dans le quad
varying vec3 vlum;    // direction lumière en base billboard
varying vec3 vforme;  // (graine, irrégularité, taille écran px)

uniform float moitie; // -1 arrière seulement, +1 avant, 0 tout

float hash(float x) {
    return fract(sin(x * 12.9898) * 43758.5453);
}

void main() {
    if (moitie > 0.5 && vside < 0.0) discard;
    if (moitie < -0.5 && vside >= 0.0) discard;

    vec3 col = vcol.rgb;
    float g   = vforme.x;
    float irr = vforme.y;
    float px  = vforme.z;

    // En dessous de ~2.5 px la forme n'existe pas : point carré simple
    // (le discard découperait l'unique fragment -> scintillement).
    if (px > 2.5) {
        float ang = atan(vp.y, vp.x);
        // Rayon patatoïde : 3 lobes doux, phases par graine, amplitude = irr.
        float rad = 0.78
            + irr * 0.16 * sin(ang * 2.0 + g * 37.0)
            + irr * 0.10 * sin(ang * 3.0 + g * 71.0)
            + irr * 0.06 * sin(ang * 5.0 + g * 113.0);
        rad = max(rad, 0.2);
        float rr = length(vp) * 0.87 / rad; // 0.87 = marge du quad (1/1.15)
        if (rr > 1.0) discard;

        // Pseudo-sphère : normale reconstituée + bosses (dérivée des lobes).
        float z = sqrt(max(1.0 - rr * rr, 0.0));
        float bosse = irr * 0.35 * cos(ang * 3.0 + g * 71.0)
                    + irr * 0.2 * cos(ang * 5.0 + g * 113.0);
        vec3 n = normalize(vec3(vp * 0.87 / rad + bosse * 0.3, z));

        // Lambert + plancher ambiant : face nuit lisible en silhouette.
        float d = 0.3 + 0.7 * clamp(dot(n, normalize(vlum)), 0.0, 1.0);
        // Facettes : petits aplats de ton par secteur angulaire.
        float facette = 0.88 + 0.24 * hash(floor(ang * 4.0 + rr * 3.0) + g * 251.0);
        col *= d * facette;
    } else {
        // Point simple : angle de phase plat (croissant des ceintures
        // lointaines, vlum.z = dot(to_light, to_cam)).
        col *= 0.35 + 0.65 * clamp(0.5 + 0.5 * vlum.z, 0.0, 1.0);
    }

    gl_FragColor = vec4(col, 1.0);
}
