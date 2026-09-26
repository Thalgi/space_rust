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
uniform float couronne_type;   // 0 = halo, 1 = jets bipolaires (pulsar), 2 = vent stellaire (WR)
uniform vec4 spots[8]; // xyz = direction (repère surface), w = rayon effectif
uniform float zone;    // 0 = disque de l'étoile (opaque), 1 = couronne (translucide)
uniform float graine;  // graine de surface, propre à chaque étoile

// **Le champ magnétique de l'étoile** (sous-lot 3.4) : xyz = direction d'un
// pôle (repère surface), w = force SIGNÉE, le signe étant la polarité.
// C'est la MÊME source que celle où naissent les taches et où s'ancrent les
// pieds d'arche (`soleil/champ.rs`). Jusqu'ici la surface l'ignorait : la
// tache était peinte par-dessus une granulation qui ne savait rien d'elle,
// d'où l'effet d'autocollant.
uniform vec4 poles[6];
// ⚠️ Ces deux valeurs sont **dupliquées depuis `soleil/champ.rs`** —
// `NB_POLES` (taille du tableau ci-dessus) et `PORTEE`. Rien dans le langage
// ne les tient d'accord : deux tests le font (`materiau.rs`).
#define NB_POLES 6
#define PORTEE 0.42

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

    // DEUX PASSES, une seule source.
    //
    // Le disque de l'étoile (rr <= 1) est OPAQUE : il doit écrire la profondeur,
    // sinon un corps passé derrière lui se dessinerait par-dessus. La couronne
    // (rr > 1) est TRANSLUCIDE : elle ne doit surtout PAS en écrire — sinon son
    // quad, plat et face caméra, pose un plan de profondeur jusqu'à 2,1 rayons
    // et rejette tout ce qui passe derrière. Sur un disque protosolaire, dont la
    // matière commence à 0,1 UA, ce plan coupait le voile le long de son
    // intersection avec le plan du disque : une **barre sombre en travers de
    // l'étoile**.
    //
    // GLSL ES 100 ne sait pas couper l'écriture de profondeur par fragment, d'où
    // deux pipelines (`zone`) sur le même shader. Même remède que pour les
    // planètes : corps opaque d'un côté, halo sans profondeur de l'autre.
    if (zone > 0.5 && rr <= 1.0) discard; // passe couronne
    if (zone < 0.5 && rr > 1.0) discard;  // passe corps

    if (couronne_type > 4.5) {
        // ===== TROU NOIR (stylisé, sans lentille gravitationnelle) =====
        //
        // ⚠️ **Le disque d'accrétion n'est plus peint ici.** C'était une
        // ellipse en coordonnées ÉCRAN, d'inclinaison câblée : elle
        // présentait éternellement la même face à la caméra, on ne pouvait
        // ni en faire le tour ni la voir par la tranche, et elle ignorait
        // l'inclinaison du système. C'est devenu un vrai `Disque`, avec son
        // plan (`soleil::avec_trou_noir`), dessiné en deux moitiés autour de
        // cet impostor. Ne restent ici que les deux choses qui appartiennent
        // vraiment à l'horizon.
        float photon = smoothstep(0.10, 0.0, abs(rr - 1.08)); // anneau de photons
        float horizon = smoothstep(1.03, 0.97, rr);           // 1 dans le trou
        vec3 col = vec3(1.0, 0.85, 0.6) * photon;
        // Horizon noir, opaque : c'est lui qui masque la moitié arrière du
        // disque, dessinée juste avant.
        col = mix(col, vec3(0.0), horizon);
        gl_FragColor = vec4(col, clamp(max(photon, horizon), 0.0, 1.0));
        return;
    }

    if (rr <= 1.0) {
        float z = sqrt(max(0.0, 1.0 - rr * rr));
        vec3 n = normalize(v_q.x * cam_right + v_q.y * cam_up + z * to_cam);

        // Rotation propre.
        float a = time * 0.12;
        float ca = cos(a); float sa = sin(a);
        vec3 d = vec3(n.x * ca - n.z * sa, n.y, n.x * sa + n.z * ca);

        // Décalage propre à cette étoile : le bruit ci-dessous ne dépend que
        // de la DIRECTION, donc sans lui toutes les étoiles portent le même
        // motif de granulation, au pixel près.
        vec3 dec = vec3(graine, graine * 1.7, graine * 0.3);

        // --- Le champ magnétique, ici ---
        // Somme de noyaux en cloche, un par pôle : exactement ce que calcule
        // `champ::champ` côté CPU. `bs` en donne l'intensité signée, `bv` la
        // direction dominante — de quoi coucher la granulation dans le sens
        // des lignes de champ.
        // ⚠️ L'intensité utilisée ici est la somme des |contributions|, et NON
        // la valeur absolue de la somme signée que calcule `champ::champ`.
        // Les deux polarités d'une région s'annulent entre elles : la somme
        // signée passe par zéro **sur la ligne neutre**, c'est-à-dire
        // précisément entre les deux taches, là où le champ est fort et
        // couché à l'horizontale. S'y fier éteindrait l'effet au seul endroit
        // où il compte. La signature reste `bv`, qui donne la direction.
        float bf = 0.0;
        vec3 bv = vec3(0.0);
        for (int i = 0; i < NB_POLES; i++) {
            vec4 p = poles[i];
            float ang = acos(clamp(dot(d, p.xyz), -1.0, 1.0));
            float t = ang / PORTEE;
            float k = p.w * exp(-t * t);
            bf += abs(k);
            bv += p.xyz * k;
        }
        // Composante TANGENTE : la direction que suivent les lignes de champ
        // à la surface. Le `step` évite de normaliser un vecteur nul au
        // centre d'une région, là où les contributions s'annulent.
        vec3 bt = bv - d * dot(bv, d);
        float btl = length(bt);
        bt = bt / max(btl, 1e-4) * step(1e-4, btl);

        // Convection fine et bouillonnante (domain warping animé, rapide).
        vec3 w = d * 5.0 + dec + vec3(0.0, time * 0.22, time * 0.13);
        vec3 warp = vec3(fbm(w), fbm(w + 5.2), fbm(w + 9.1)) - 0.5;
        // ⚠️ La granulation s'ÉTIRE le long du champ : on comprime la
        // direction d'échantillonnage dans le sens de `bt`, ce qui allonge
        // les granules perpendiculairement — le signe visuel de l'émergence,
        // et ce qui fait qu'une tache a l'air de sortir de la surface au lieu
        // d'y être collée. Nul là où le champ est faible : la granulation
        // reste isotrope sur le reste de l'étoile.
        float etir = 0.6 * smoothstep(0.25, 1.1, bf);
        vec3 da = d - bt * (dot(d, bt) * etir);
        float gran = fbm(da * 11.0 + dec + warp * 1.2 + vec3(0.0, time * 0.30, 0.0));

        // Palette dérivée de la couleur de l'étoile : sombre -> teinte -> chaud (vers blanc).
        vec3 cLow = teinte * 0.45;
        vec3 cMid = teinte;
        vec3 cHot = mix(teinte, vec3(1.0), 0.6);
        vec3 col = mix(cLow, cMid, smoothstep(0.30, 0.55, gran));
        col = mix(col, cHot, smoothstep(0.55, 0.78, gran));

        // Petits points brillants (campfires).
        float camp = fbm(d * 16.0 + dec + 50.0);
        col += smoothstep(0.66, 0.72, camp) * teinte * 0.5;

        // Un champ fort bloque la convection bien AVANT de creuser une tache :
        // c'est le voile sombre qui entoure un groupe. Sans lui, la tache
        // s'arrête net sur une surface qui l'ignore — la moitié de l'effet
        // d'autocollant. Reste léger : l'assombrissement franc, c'est la
        // tache elle-même, plus bas.
        col *= mix(1.0, 0.74, smoothstep(0.35, 1.25, bf));

        // Taches (w > 0, assombrissement) ET flares (w < 0, sursaut lumineux)
        // partagent le même tableau `spots` -- le signe de `w` les distingue
        // (voir `eruptions::encoder_flare`), pas un second uniform.
        float dark = 0.0;   // ombre : le cœur noir de la tache
        float pen = 0.0;    // pénombre : la couronne filamenteuse autour
        float flare = 0.0;
        for (int i = 0; i < 8; i++) {
            vec4 s = spots[i];
            float rayon = abs(s.w);
            if (rayon <= 0.0) { continue; }
            float ang = acos(clamp(dot(d, s.xyz), -1.0, 1.0));
            if (s.w > 0.0) {
                // ⚠️ Un seuil sur l'angle seul donne un **disque parfait** —
                // c'est ce qu'on voyait, et aucune tache réelle n'est ronde.
                // On module donc le rayon par un bruit pris AUTOUR de la
                // tache : le contour devient irrégulier, sans coûter un
                // uniform ni une passe.
                float bord = 0.78 + 0.34 * fbm(d * 9.0 + s.xyz * 7.0 + dec);
                // L'ombre (sombre, nette) et sa pénombre (plus claire,
                // filamenteuse) : deux seuils sur le même champ, comme sur
                // une vraie tache.
                dark += 1.0 - smoothstep(rayon * 0.30 * bord, rayon * 0.62 * bord, ang);
                pen += 1.0 - smoothstep(rayon * 0.60 * bord, rayon * bord, ang);
            } else {
                flare += 1.0 - smoothstep(rayon * 0.4, rayon, ang);
            }
        }
        dark = clamp(dark, 0.0, 1.0);
        pen = clamp(pen, 0.0, 1.0);
        flare = clamp(flare, 0.0, 1.0);
        // Pénombre d'abord, ombre par-dessus : la seconde est incluse dans la
        // première, et doit l'emporter là où elles se recouvrent.
        col *= mix(1.0, 0.62, pen);
        col *= mix(1.0, 0.30, dark);
        // Flare : sursaut lumineux localisé -> éclaircit vers un blanc chaud,
        // au lieu d'assombrir. C'est l'exact inverse de la tache ci-dessus.
        col = mix(col, vec3(1.0, 0.95, 0.85) + teinte * 0.3, flare * 0.85);

        // Bord renforcé (limb brightening) -> anneau lumineux.
        float rim = smoothstep(0.45, 0.0, z);
        col += rim * teinte * 0.55;

        gl_FragColor = vec4(col, 1.0);
    } else if (couronne_type > 0.5 && couronne_type < 1.5) {
        // JETS bipolaires (pulsar / protoétoile) le long de l'axe vertical (v_q.y).
        float ax = abs(v_q.x);
        float ay = abs(v_q.y);
        float width = 0.10 + 0.12 * (ay - 1.0);              // cône qui s'évase
        float core = 1.0 - smoothstep(0.0, width, ax);
        float along = smoothstep(couronne, 1.0, ay) * step(1.0, ay); // le long du jet
        float flow = fbm(vec3(v_q.x * 9.0, v_q.y * 3.5 - sign(v_q.y) * time * 2.4, time * 0.2));
        float jet = core * along * (0.45 + 0.9 * flow);
        vec3 jetcol = mix(teinte, vec3(0.6, 0.8, 1.0), 0.55); // jets bleutés énergétiques
        gl_FragColor = vec4(jetcol, clamp(jet, 0.0, 1.0) * 0.9);
    } else if (couronne_type > 3.5) {
        // MAGNETAR : arcs de champ magnétique DIPOLAIRE (boucles brillantes, violettes).
        float g = clamp(1.0 - (rr - 1.0) / (couronne - 1.0), 0.0, 1.0);
        float sinth = abs(v_q.x) / max(rr, 0.001);            // colatitude (axe vertical)
        float L = rr / max(sinth * sinth, 0.05);              // paramètre de ligne de champ (r = L sin²θ)
        float lines = abs(fract(L * 0.55 + time * 0.08) - 0.5) * 2.0; // 0 au cœur d'une boucle
        float arc = smoothstep(0.16, 0.0, lines) * g;
        arc *= 0.55 + 0.45 * fbm(vec3(v_q * 5.0, time * 0.5)); // scintillement
        vec3 mcol = mix(teinte, vec3(0.72, 0.5, 1.0), 0.6);   // violet magnétique
        gl_FragColor = vec4(mcol, clamp(arc, 0.0, 1.0) * 0.85);
    } else if (couronne_type > 2.5) {
        // PULSAR : faisceau bipolaire qui TOURNE sur lui-même (effet phare) + flash périodique.
        float spin = time * 1.1;
        float cs = cos(spin), sn = sin(spin);
        vec2 q = vec2(v_q.x * cs - v_q.y * sn, v_q.x * sn + v_q.y * cs);
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
