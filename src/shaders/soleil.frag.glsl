#version 100
precision highp float;
varying vec2 v_q;
uniform float time;
uniform vec3 cam_right;
uniform vec3 cam_up;
uniform vec3 to_cam;
uniform vec3 teinte;   // couleur de l'étoile (selon son type spectral)
uniform float couronne;        // extension de la couronne (× rayon)
uniform float couronne_irreg;  // irrégularité (rayons/spicules)
uniform float couronne_type;   // 0=halo 1=jets 2=vent(WR) 3=pulsar 4=magnetar 5=trou noir
uniform vec3 axe;              // axe du pôle (monde) -> ancrage des jets/arcs hors caméra
uniform float gran_scale;      // taille des cellules de granulation (1 = défaut, >1 = fines)
uniform float gran_contraste;  // contraste de la convection (1 = défaut, <1 = lisse)
uniform vec4 spots[8]; // xyz = direction (repère surface), w = rayon effectif

float hash(vec3 p) {
    p = fract(p * 0.3183099 + 0.1);
    p *= 17.0;
    return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
}
float vnoise(vec3 x) {
    vec3 i = floor(x);
    vec3 f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(mix(hash(i + vec3(0.0,0.0,0.0)), hash(i + vec3(1.0,0.0,0.0)), f.x),
                   mix(hash(i + vec3(0.0,1.0,0.0)), hash(i + vec3(1.0,1.0,0.0)), f.x), f.y),
               mix(mix(hash(i + vec3(0.0,0.0,1.0)), hash(i + vec3(1.0,0.0,1.0)), f.x),
                   mix(hash(i + vec3(0.0,1.0,1.0)), hash(i + vec3(1.0,1.0,1.0)), f.x), f.y), f.z);
}
float fbm(vec3 p) {
    float v = 0.0;
    float a = 0.5;
    for (int k = 0; k < 5; k++) {
        v += a * vnoise(p);
        p *= 2.0;
        a *= 0.5;
    }
    return v;
}

void main() {
    float rr = length(v_q);
    if (rr > couronne) discard;

    if (couronne_type > 4.5) {
        // TROU NOIR : horizon des evenements + disque d'accretion incline.
        // Stylise (pas de vrai ray-tracing) : rim chaud en guise de lentille,
        // disque aplati selon l'inclinaison de l'axe, turbulence fbm + asymetrie
        // Doppler (cote qui approche plus brillant). Cout = 1 quad + 1 fbm, comme
        // les autres couronnes.
        if (rr <= 1.0) {
            float rim = smoothstep(0.78, 1.0, rr);
            vec3 ring = mix(teinte, vec3(1.0), 0.7);
            gl_FragColor = vec4(ring * rim * 1.4, 1.0);
        } else {
            vec2 pax = vec2(dot(axe, cam_right), dot(axe, cam_up));
            float fs = length(pax);                        // 0 = pole face camera (disque rond), 1 = de profil
            vec2 adir = fs > 0.001 ? pax / fs : vec2(0.0, 1.0);
            vec2 pdir = vec2(-adir.y, adir.x);
            float along = dot(v_q, adir);                  // le long du pole projete -> comprime
            float perp = dot(v_q, pdir);                    // grand axe du disque -> inchange
            float flat = mix(1.0, 0.16, fs);                // aplatissement selon l'inclinaison
            vec2 dq = vec2(along / max(flat, 0.05), perp);
            float dr = length(dq);
            float ang = atan(dq.y, dq.x);
            float inner = 1.05;
            float outer = couronne;
            float band = smoothstep(inner, inner + 0.22, dr) * (1.0 - smoothstep(outer * 0.65, outer, dr));
            float spin = time * 0.6;
            float turb = fbm(vec3(cos(ang - spin) * dr * 2.2, sin(ang - spin) * dr * 2.2, dr * 1.4 + time * 0.1));
            float t = clamp((dr - inner) / max(outer - inner, 0.01), 0.0, 1.0);
            vec3 cHot = mix(teinte, vec3(1.0), 0.65);
            vec3 cMid = teinte;
            vec3 cCold = teinte * 0.35;
            vec3 disccol = mix(cHot, cMid, smoothstep(0.0, 0.45, t));
            disccol = mix(disccol, cCold, smoothstep(0.45, 1.0, t));
            // Doppler beaming : nul de face (fs=0), maximal de profil (fs=1) ; cote
            // qui approche (un bord du grand axe) plus brillant que celui qui s'eloigne.
            float beam = mix(0.35, 1.8, smoothstep(-1.0, 1.0, sin(ang)));
            float dop = mix(1.0, beam, fs);
            float bright = band * (0.55 + 0.65 * turb) * dop;
            gl_FragColor = vec4(disccol, clamp(bright, 0.0, 1.0));
        }
    } else if (rr <= 1.0) {
        float z = sqrt(max(0.0, 1.0 - rr * rr));
        vec3 n = normalize(v_q.x * cam_right + v_q.y * cam_up + z * to_cam);

        // Rotation propre.
        float a = time * 0.12;
        float ca = cos(a); float sa = sin(a);
        vec3 d = vec3(n.x * ca - n.z * sa, n.y, n.x * sa + n.z * ca);

        // Convection : la taille des cellules dépend du type (gran_scale).
        // Naines = cellules fines/serrées ; supergéantes = grosses cellules molles.
        float gs = max(gran_scale, 0.2);
        vec3 w = d * 5.0 * gs + vec3(0.0, time * 0.22, time * 0.13);
        vec3 warp = vec3(fbm(w), fbm(w + 5.2), fbm(w + 9.1)) - 0.5;
        float gran = fbm(d * 11.0 * gs + warp * 1.2 + vec3(0.0, time * 0.30, 0.0));

        // Palette dérivée de la couleur de l'étoile : sombre -> teinte -> chaud (vers blanc).
        vec3 cLow = teinte * 0.45;
        vec3 cMid = teinte;
        vec3 cHot = mix(teinte, vec3(1.0), 0.6);
        // Contraste de la convection : étroit = net (étoiles actives), large = lisse (chaudes).
        float spread = 0.22 / max(gran_contraste, 0.3);
        vec3 col = mix(cLow, cMid, smoothstep(0.5 - spread, 0.5, gran));
        col = mix(col, cHot, smoothstep(0.5, 0.5 + spread, gran));

        // Petits points brillants (campfires), à la même échelle.
        float camp = fbm(d * 16.0 * gs + 50.0);
        col += smoothstep(0.66, 0.72, camp) * teinte * 0.5 * gran_contraste;

        // Taches actives (venant du CPU) : assombrissement local.
        float dark = 0.0;
        for (int i = 0; i < 8; i++) {
            vec4 s = spots[i];
            if (s.w > 0.0) {
                float ang = acos(clamp(dot(d, s.xyz), -1.0, 1.0));
                dark += 1.0 - smoothstep(s.w * 0.4, s.w, ang);
            }
        }
        dark = clamp(dark, 0.0, 1.0);
        col *= mix(1.0, 0.30, dark);

        // Bord renforcé (limb brightening) -> anneau lumineux.
        float rim = smoothstep(0.45, 0.0, z);
        col += rim * teinte * 0.55;

        gl_FragColor = vec4(col, 1.0);
    } else if (couronne_type > 0.5 && couronne_type < 1.5) {
        // JETS bipolaires (étoile à neutrons / protoétoile) ancrés sur l'axe du pôle PROJETÉ.
        vec2 pax = vec2(dot(axe, cam_right), dot(axe, cam_up));
        float fs = length(pax);                              // 0 = pôle vers caméra, 1 = de profil
        vec2 adir = fs > 0.001 ? pax / fs : vec2(0.0, 1.0);
        vec2 pdir = vec2(-adir.y, adir.x);
        float s = dot(v_q, adir);                            // le long de l'axe
        float perp = dot(v_q, pdir);                         // perpendiculaire
        float ax = abs(perp);
        float ay = abs(s);
        float width = 0.10 + 0.12 * (ay - 1.0);              // cône qui s'évase
        float core = 1.0 - smoothstep(0.0, width, ax);
        float along = smoothstep(couronne, 1.0, ay) * step(1.0, ay);
        float flow = fbm(vec3(perp * 9.0, s * 3.5 - sign(s) * time * 2.4, time * 0.2));
        float jet = core * along * (0.45 + 0.9 * flow);
        vec3 jetcol = mix(teinte, vec3(0.6, 0.8, 1.0), 0.55);
        gl_FragColor = vec4(jetcol, clamp(jet, 0.0, 1.0) * 0.9 * fs); // foreshorten si pôle de face
    } else if (couronne_type > 3.5) {
        // MAGNETAR : arcs de champ magnétique DIPOLAIRE (boucles brillantes, violettes), ancrés.
        vec2 pax = vec2(dot(axe, cam_right), dot(axe, cam_up));
        float fs = length(pax);
        vec2 adir = fs > 0.001 ? pax / fs : vec2(0.0, 1.0);
        vec2 pdir = vec2(-adir.y, adir.x);
        float perp = dot(v_q, pdir);                          // distance à l'axe du pôle
        float g = clamp(1.0 - (rr - 1.0) / (couronne - 1.0), 0.0, 1.0);
        float sinth = abs(perp) / max(rr, 0.001);             // colatitude depuis l'axe projeté
        float L = rr / max(sinth * sinth, 0.05);              // paramètre de ligne de champ (r = L sin²θ)
        float lines = abs(fract(L * 0.55 + time * 0.08) - 0.5) * 2.0; // 0 au cœur d'une boucle
        float arc = smoothstep(0.16, 0.0, lines) * g;
        arc *= 0.55 + 0.45 * fbm(vec3(v_q * 5.0, time * 0.5)); // scintillement
        vec3 mcol = mix(teinte, vec3(0.72, 0.5, 1.0), 0.6);   // violet magnétique
        gl_FragColor = vec4(mcol, clamp(arc, 0.0, 1.0) * 0.85);
    } else if (couronne_type > 2.5) {
        // PULSAR : faisceau bipolaire qui TOURNE sur lui-même (phare), ancré sur l'axe projeté.
        vec2 pax = vec2(dot(axe, cam_right), dot(axe, cam_up));
        float fs = length(pax);
        vec2 adir = fs > 0.001 ? pax / fs : vec2(0.0, 1.0);
        vec2 pdir = vec2(-adir.y, adir.x);
        float s0 = dot(v_q, adir);
        float perp0 = dot(v_q, pdir);
        float spin = time * 1.1;
        float cs = cos(spin), sn = sin(spin);
        vec2 q = vec2(perp0 * cs - s0 * sn, perp0 * sn + s0 * cs);
        float ax = abs(q.x);
        float ay = abs(q.y);
        float width = 0.08 + 0.10 * (ay - 1.0);
        float core = 1.0 - smoothstep(0.0, width, ax);
        float along = smoothstep(couronne, 1.0, ay) * step(1.0, ay);
        float flow = fbm(vec3(q.x * 9.0, q.y * 3.5 - sign(q.y) * time * 3.2, time * 0.2));
        float pulse = 0.55 + 0.45 * sin(time * 7.0); // flash du phare
        float jet = core * along * (0.45 + 0.9 * flow) * pulse;
        vec3 jetcol = mix(teinte, vec3(0.6, 0.85, 1.0), 0.6);
        gl_FragColor = vec4(jetcol, clamp(jet, 0.0, 1.0) * 0.95);
    } else if (couronne_type > 1.5) {
        // VENT stellaire (Wolf-Rayet / supergéante bleue) : enveloppe épaisse, en expansion,
        // grumeleuse (clumps qui s'éloignent radialement) -> très différent du halo lisse.
        float g = clamp(1.0 - (rr - 1.0) / (couronne - 1.0), 0.0, 1.0);
        float ang = atan(v_q.y, v_q.x);
        // Turbulence advectée radialement (le vent souffle vers l'extérieur).
        float turb = fbm(vec3(cos(ang) * 3.2, sin(ang) * 3.2, rr * 1.6 - time * 0.5) + 20.0);
        float clump = fbm(vec3(v_q * 3.2, time * 0.15) + 40.0);
        float w = pow(g, 0.85) * (0.3 + 1.2 * turb) * (0.4 + 0.9 * clump);
        // Coquilles concentriques (ondes de choc du vent).
        w *= 0.7 + 0.3 * sin(rr * 9.0 - time * 1.2);
        vec3 windcol = mix(teinte, vec3(0.55, 0.5, 1.0), 0.45); // bleu-violet
        gl_FragColor = vec4(windcol, clamp(w, 0.0, 1.0) * 0.85);
    } else {
        // Couronne : décroît de r=1 à r=couronne ; forme modulée par couronne_irreg.
        float g = 1.0 - (rr - 1.0) / (couronne - 1.0);
        g = clamp(g, 0.0, 1.0);
        float ang = atan(v_q.y, v_q.x);
        float ray = fbm(vec3(cos(ang) * 3.0, sin(ang) * 3.0, time * 0.05) + 10.0);
        float spik = mix(1.0, 0.35 + 1.3 * ray, couronne_irreg);
        g = pow(g, 1.7) * spik;
        gl_FragColor = vec4(teinte, clamp(g, 0.0, 1.0) * 0.6);
    }
}
