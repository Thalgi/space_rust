#version 100
// Disque d'accrétion (sous-lot 6.1). Partage le vertex shader du voile --
// l'annulus, son orientation et sa translation sont exactement les mêmes --
// mais rien d'autre : un disque d'accrétion n'est pas un anneau de matière
// éclairé du dehors, c'est du gaz qui rayonne parce qu'il brûle.
//
// Ce qu'il fait et que le voile ne faisait pas :
//   - un profil de température en PUISSANCE (T ~ r^-3/4, Shakura-Sunyaev),
//     là où le voile interpolait linéairement deux teintes ;
//   - un beaming Doppler qui décale aussi la COULEUR, pas seulement l'éclat ;
//   - une turbulence cisaillée par la rotation képlérienne.
//
// Ce qu'il ne fait PAS : la silhouette (le disque reste un anneau plat, plié
// au sous-lot 6.2), et toute forme de lentille gravitationnelle.
precision highp float;

varying vec2 vtc;    // (t radial 0..1, ang 0..1)
varying float vdot;  // dot(position relative, tocam) -> moitiés avant/arrière
varying vec3 vpos;   // position relative au centre (monde)
varying float vpli;  // part de pli appliquée ici (0 = à plat)

uniform float time;
uniform float seed;
uniform float moitie;      // -1 arrière seulement, +1 avant, 0 tout
uniform float alpha;       // opacité maximale
uniform vec3 chaud;        // teinte du bord interne (le plus chaud)
uniform vec3 froid;        // teinte du bord externe
uniform float exposant;    // dureté du profil de température (0 = plat)
uniform float turbulence;  // 0 lisse .. 1 filamenteux
uniform vec2 turb_freq;    // (fréquence spatiale, poids de l'octave fine)
uniform float rotation;    // vitesse angulaire visuelle au bord interne
uniform float r_ratio;     // externe / interne -> rotation différentielle
uniform float doppler;     // force du beaming (0 = aucun)
uniform float bord;        // largeur des bords doux (fraction radiale)
uniform vec3 axe_n;        // normale du plan
uniform vec3 tocam;        // du centre vers la caméra
uniform float pli;         // amplitude du pli (0 = disque plat)
uniform float pli_signe;   // +1 = arc du dessus, -1 = celui du dessous

float hash21(vec2 p) {
    return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

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

void main() {
    // Moitié avant / arrière : l'astre s'intercale entre les deux passes.
    if (moitie * vdot < 0.0) { discard; }

    // **Le second arc ne vit que là où le pli l'a vraiment soulevé.** Sur les
    // côtés le pli s'annule, et les deux copies de la moitié arrière se
    // superposent exactement — en additif, une bande deux fois trop brillante
    // le long de la ligne de raccord. Celle du dessus, elle, se raccorde au
    // disque plat : c'est la même matière, elle doit rester continue.
    float part_arc = 1.0;
    if (pli > 0.001 && pli_signe < 0.0) {
        part_arc = smoothstep(0.12, 0.55, vpli);
        if (part_arc < 0.004) { discard; }
    }

    float t = clamp(vtc.x, 0.0, 1.0);        // 0 = bord interne, 1 = externe
    float rr = mix(1.0, r_ratio, t);         // rayon en unités du bord interne

    // **Rotation képlérienne différentielle** : l'intérieur tourne bien plus
    // vite que l'extérieur (omega ~ r^-3/2). C'est ce qui étire la matière en
    // filaments au lieu de la laisser en paquets.
    float ang = fract(vtc.y - rotation * time / pow(rr, 1.5));

    // **Profil de température.** T ~ r^-3/4 et l'émission suit T^4, soit une
    // chute en r^-3 : le bord interne doit ECRASER le reste. `exposant` en
    // règle la dureté -- la valeur physique (3.0) éteint si complètement le
    // bord externe qu'il disparaît, et on perd le disque.
    float chaleur = pow(rr, -exposant);

    // Turbulence, cisaillée avec la rotation : on échantillonne en espace
    // disque (cartésien) pour éviter le damier polaire du bord externe.
    float dens = 1.0;
    if (turbulence > 0.001) {
        float a2 = ang * 6.2831853;
        vec2 p = vec2(cos(a2), sin(a2)) * (rr / r_ratio) * turb_freq.x;
        float n = vnoise(p + seed);
        n = mix(n, vnoise(p * 2.7 + seed * 1.7), turb_freq.y);
        dens = mix(1.0, 0.45 + 1.1 * n, turbulence);
    }

    // Bords doux : sans eux, l'annulus se termine sur une arête franche.
    float b = max(bord, 0.001);
    dens *= smoothstep(0.0, b, t) * (1.0 - smoothstep(1.0 - b, 1.0, t));

    // **Beaming Doppler.** Le côté qui vient vers nous est plus brillant ET
    // plus bleu ; celui qui s'éloigne, plus sombre et plus rouge. Le voile
    // partagé n'en faisait que la moitié (l'éclat). C'est l'asymétrie qui
    // distingue un disque d'accrétion d'un anneau -- sur M87*, elle est le
    // trait le plus visible de l'image.
    float faisceau = 1.0;
    float bleuissement = 0.0;
    if (doppler > 0.001) {
        vec3 radial = normalize(vpos - axe_n * dot(vpos, axe_n));
        vec3 vitesse = cross(axe_n, radial);
        float vers_nous = dot(vitesse, tocam);
        // La matière interne tourne le plus vite : le beaming y est le plus
        // fort. Même dépendance que la rotation, en plus doux.
        float rapide = pow(rr, -0.5);
        faisceau = max(0.0, 1.0 + doppler * vers_nous * rapide);
        bleuissement = clamp(doppler * vers_nous * rapide * 0.5, -0.6, 0.6);
    }

    vec3 col = mix(froid, chaud, clamp(chaleur, 0.0, 1.0));
    // Décalage de couleur : vers le bleu si la matière s'approche, vers le
    // rouge si elle fuit. Appliqué sur les canaux extrêmes seulement.
    col.b *= 1.0 + bleuissement;
    col.r *= 1.0 - bleuissement * 0.6;
    col *= faisceau;

    float a = clamp(dens * chaleur, 0.0, 1.0) * alpha * part_arc;
    a *= clamp(faisceau, 0.35, 1.8);
    if (a < 0.004) { discard; }

    gl_FragColor = vec4(max(col, vec3(0.0)), clamp(a, 0.0, 1.0));
}
