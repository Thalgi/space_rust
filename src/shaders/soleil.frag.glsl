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
uniform float ebullition; // 0 = granulation fine ; > 0 = cellules qui bouillonnent (essai)
uniform float ebul_style; // 0 = peint à plat, 1 = relief par l'éclairage, 2 = contour irrégulier
// Éclat de surface : flare et pulsation, déjà compensés de la taille dessinée
// (`soleil::eclat_surfacique`). 1 au repos. Sans lui, ni le sursaut ni la
// pulsation n'atteignaient l'image : ils n'éclairaient que les planètes.
uniform float eclat;
// Surface propre au type (lot 10, essai) : x = échelle du grain (11 = celle
// d'origine), y = contraste du grain (1 = d'origine, 0 = surface lisse),
// z = assombrissement du bord (0 = le bord ÉCLAIRCI d'origine).
uniform vec3 surface;
// Réglages de l'ébullition (lot 10, essai) : x = finesse des cellules (1 =
// d'origine, 2 = deux fois plus petites), y = clarté minimale du bouillonnement
// (0,5 = d'origine ; plus haut, le fond des cellules et l'ombre du relief
// s'éclaircissent).
uniform vec2 ebul_reglage;
// Lueur du cœur (lot 11, essai) : la quasi-étoile est chauffée par un trou noir
// en son centre, pas par la fusion. 0 pour toutes les autres étoiles.
uniform float coeur;
// Inclinaison de l'axe de rotation (lot 12) : quaternion qui amène le
// pôle nord de la surface (+Y) sur l'axe réel de l'étoile — celui de son
// disque et de ses jets. Identité (0,0,0,1) pour toutes les autres.
uniform vec4 axe_rot;
// Taches d'accrétion polaires (lot 12) : le gaz du disque tombe sur
// l'étoile le long de son champ magnétique, près des pôles. 0 = aucune.
uniform float accretion_pol;
// Axe magnétique en coordonnées du monde (lot 12) : nul = les pôles de rotation.
uniform vec3 axe_magnetique;
// Lot 12 (étoile Be) : rayon polaire = 1 − aplatissement fois l'équatorial.
uniform float aplatissement;

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

vec3 hash3(vec3 p) {
    return vec3(hash(p), hash(p + 17.13), hash(p + 31.71));
}

// **Cellules de convection** (essai d'ébullition) : distance au centre de la
// cellule la plus proche (x), à la seconde (y), identité de la plus proche (z).
// Les centres dérivent lentement : une cellule roule, elle n'est pas posée.
vec3 cellules(vec3 p, float t) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    float f1 = 8.0;
    float f2 = 8.0;
    float id = 0.0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            for (int z = -1; z <= 1; z++) {
                vec3 g = vec3(float(x), float(y), float(z));
                vec3 h = hash3(i + g);
                vec3 r = g + 0.5 + 0.38 * sin(t + 6.2831 * h) - f;
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
    }
    return vec3(sqrt(f1), sqrt(f2), id);
}

// **Chaleur d'une couche de bulles** : 0 = sillon froid, 1 = bulle qui crève.
// Chaque cellule vit à son rythme (son identité décale sa phase) : elle gonfle
// lentement, éclate d'un coup, refroidit, recommence — une casserole, pas une
// respiration : deux cellules voisines ne sont jamais au même moment.
float bulles(vec3 p, float t, float vitesse) {
    vec3 c = cellules(p, t * 0.25);
    float vie = fract(t * vitesse + c.z * 13.7);
    // ⚠️ Un cycle qui se REFERME : à vie = 1, tout est revenu à l'état de
    // vie = 0. Le premier jet finissait refroidi, sombre, et `fract` le
    // remettait à neuf d'un coup : les plaques grises s'effaçaient d'une
    // image à l'autre.
    float gonfle = smoothstep(0.0, 0.62, vie) * (1.0 - smoothstep(0.62, 0.80, vie));
    float eclat = smoothstep(0.60, 0.68, vie) * (1.0 - smoothstep(0.68, 0.85, vie));
    float refroid = smoothstep(0.70, 0.85, vie) * (1.0 - smoothstep(0.88, 1.0, vie));
    // Le dôme : une tache claire qui grossit au centre de la cellule.
    float dome = 1.0 - smoothstep(0.0, mix(0.18, 0.62, gonfle), c.x);
    // ⚠️ Ce qui est propre à UNE cellule s'éteint à sa frontière : c'est là
    // que la cellule la plus proche change, et sa phase avec elle. Sans ce
    // fondu, la frontière tranchait net entre deux états.
    float interieur = smoothstep(0.0, 0.22, c.y - c.x);
    // Le sillon : sombre là où deux cellules se touchent.
    float sillon = 1.0 - smoothstep(0.0, 0.14, c.y - c.x);
    float propre = dome * (0.30 + 0.35 * gonfle) + eclat * 0.55 - refroid * 0.18;
    return clamp(0.40 + propre * interieur - sillon * 0.28, 0.0, 1.0);
}

// Les deux couches de bulles : les grosses, et les petites qui crèvent dessus.
float bouillon(vec3 pb) {
    return mix(bulles(pb, time, 0.05), bulles(pb * 2.7 + 11.0, time * 1.6, 0.09), 0.30);
}

// Rotation d'un vecteur par un quaternion unitaire.
vec3 qrot(vec4 q, vec3 v) {
    return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}

// **L'ellipse d'une étoile aplatie, ramenée au cercle.** L'axe de rotation
// (`axe_rot` appliqué à +Y) se projette sur l'écran ; vu de côté, la
// silhouette est écrasée le long de cette projection de `1 − aplatissement`,
// vu par le pôle elle reste ronde. On étire donc d'autant le long de l'axe
// projeté : un point du limbe aplati retombe sur le cercle unité.
vec2 aplatir(vec2 q) {
    if (aplatissement <= 0.0) return q;
    vec3 ax = qrot(axe_rot, vec3(0.0, 1.0, 0.0));
    vec2 pa = vec2(dot(ax, cam_right), dot(ax, cam_up));
    float s = length(pa);
    if (s < 1e-4) return q;
    vec2 u = pa / s;
    float p = 1.0 - aplatissement;
    float k = sqrt(max(1.0 - s * s * (1.0 - p * p), 1e-4));
    return q + u * dot(q, u) * (1.0 / k - 1.0);
}

// Rotation propre : du repère monde au repère de la surface, qui tourne.
// ⚠️ D'abord **redresser** l'axe (le conjugué de `axe_rot`), puis la rotation
// propre autour de +Y. C'est l'inverse exact de `vers_monde` côté CPU, qui
// place taches et arches : les deux doivent rester accordés, sinon une arche
// se dessinerait ailleurs que sa tache.
vec3 tourner(vec3 n) {
    n = qrot(vec4(-axe_rot.xyz, axe_rot.w), n);
    float a = time * 0.12;
    float ca = cos(a); float sa = sin(a);
    return vec3(n.x * ca - n.z * sa, n.y, n.x * sa + n.z * ca);
}

void main() {
    // **Aplatie** (l'étoile Be) : on ramène l'ellipse vue à un cercle, en
    // étirant le long de l'axe projeté — tout ce qui suit voit une sphère.
    vec2 vq = aplatir(v_q);
    float rr = length(vq);
    if (rr > couronne) discard;

    // Décalage propre à cette étoile : le bruit ci-dessous ne dépend que
    // de la DIRECTION, donc sans lui toutes les étoiles portent le même
    // motif de granulation, au pixel près.
    vec3 dec = vec3(graine, graine * 1.7, graine * 0.3);
    float ech = mix(5.0, 2.2, ebullition) * ebul_reglage.x;

    // **Contour irrégulier** (essai, `ebul_style` 2) : le bord du disque se
    // soulève là où une grosse bulle passe sur le limbe. ⚠️ Calculé AVANT
    // l'aiguillage des passes, et identique dans les deux : sinon disque et
    // couronne ne se rejoindraient plus et un liseré vide les séparerait.
    float limbe = 1.0;
    if (ebul_style > 1.5) {
        vec3 dl = tourner(normalize(vq.x * cam_right + vq.y * cam_up + 1e-4 * to_cam));
        limbe = 1.0 + 0.10 * (bulles(dl * ech + dec, time, 0.05) - 0.45);
    }

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
    if (zone > 0.5 && rr <= limbe) discard; // passe couronne
    if (zone < 0.5 && rr > limbe) discard;  // passe corps

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
        // ⚠️ **Ce liseré n'est pas l'anneau de photons.** Celui-ci ne gagne
        // qu'un renforcement LOGARITHMIQUE de brillance : il est
        // observationnellement negligeable. Ce qu'on voit est l'anneau de
        // LENTILLE, une image demagnifiee de la face arriere du disque --
        // et c'est le disque lui-meme qui le dessine desormais
        // (`disque_accretion`, passe `anneau`), avec sa couleur, son bruit
        // et son asymetrie. Ne reste ici qu'un liseré discret sur le bord de
        // l'ombre, la ou il n'y a pas de matiere pour le porter.
        //
        // Il valait 1,0 d'eclat et une teinte orangee franche : il se lisait
        // comme un cercle trace au compas, et dominait l'image alors qu'il
        // ne represente que quelques pour cent du flux.
        float photon = smoothstep(0.07, 0.0, abs(rr - 1.05));
        float horizon = smoothstep(1.02, 0.98, rr);  // 1 dans le trou
        vec3 col = vec3(1.0, 0.93, 0.82) * photon * 0.35;
        // Horizon noir, opaque : c'est lui qui masque la moitié arrière du
        // disque, dessinée juste avant.
        col = mix(col, vec3(0.0), horizon);
        gl_FragColor = vec4(col, clamp(max(photon * 0.5, horizon), 0.0, 1.0));
        return;
    }

    if (rr <= limbe) {
        // Sphère de rayon `limbe` dans cette direction (1 hors contour irrégulier).
        float rs = rr / limbe;
        float z = sqrt(max(0.0, 1.0 - rs * rs));
        vec3 n = normalize((vq.x * cam_right + vq.y * cam_up) / limbe + z * to_cam);
        vec3 d = tourner(n);

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
        // L'échelle du grain entraîne celle de son gauchissement : sans elle,
        // un grain fin serait tordu par des tourbillons trop larges pour lui.
        vec3 w = d * (5.0 * surface.x / 11.0) + dec + vec3(0.0, time * 0.22, time * 0.13);
        vec3 warp = vec3(fbm(w), fbm(w + 5.2), fbm(w + 9.1)) - 0.5;
        // ⚠️ La granulation s'ÉTIRE le long du champ : on comprime la
        // direction d'échantillonnage dans le sens de `bt`, ce qui allonge
        // les granules perpendiculairement — le signe visuel de l'émergence,
        // et ce qui fait qu'une tache a l'air de sortir de la surface au lieu
        // d'y être collée. Nul là où le champ est faible : la granulation
        // reste isotrope sur le reste de l'étoile.
        float etir = 0.6 * smoothstep(0.25, 1.1, bf);
        vec3 da = d - bt * (dot(d, bt) * etir);
        float gran = fbm(da * surface.x + dec + warp * 1.2 + vec3(0.0, time * 0.30, 0.0));
        // Contraste : 0 ramène tout le grain à sa valeur moyenne — une étoile
        // chaude n'a pas de convection en surface, donc pas de granulation.
        gran = 0.5 + (gran - 0.5) * surface.y;

        // Palette dérivée de la couleur de l'étoile : sombre -> teinte -> chaud (vers blanc).
        vec3 cLow = teinte * 0.45;
        vec3 cMid = teinte;
        vec3 cHot = mix(teinte, vec3(1.0), 0.6);
        vec3 col = mix(cLow, cMid, smoothstep(0.30, 0.55, gran));
        col = mix(col, cHot, smoothstep(0.55, 0.78, gran));

        // Petits points brillants (campfires).
        float camp = fbm(d * 16.0 + dec + 50.0);
        col += smoothstep(0.66, 0.72, camp) * teinte * 0.5 * surface.y;

        if (ebullition > 0.0) {
            // **Ébullition** (essai) : quelques cellules géantes sur une
            // supergéante, comme Bételgeuse, plus nombreuses sur une géante.
            // Deux couches : les grosses bulles, et de plus petites, plus
            // vives, qui crèvent à leur surface.
            vec3 pb = d * ech + dec + warp * 0.6;
            float h0 = bouillon(pb);
            // Le grain fin reste, en sourdine : sans lui, du plastique.
            float h = h0 + (gran - 0.5) * 0.25;
            vec3 cb = mix(teinte * ebul_reglage.y, teinte * 0.95, smoothstep(0.15, 0.55, h));
            cb = mix(cb, cHot, smoothstep(0.62, 0.95, h));
            col = mix(col, cb, clamp(ebullition * 2.0, 0.0, 1.0));

            if (ebul_style > 0.5 && ebul_style < 1.5) {
                // **Relief par l'éclairage** (essai) : la chaleur sert de
                // hauteur, sa pente incline la normale, et une lumière fixe
                // (haut gauche, côté caméra) éclaire un flanc de chaque dôme
                // et ombre l'autre. Seul compte l'ÉCART à la sphère lisse :
                // l'étoile ne doit pas se retrouver à moitié dans l'ombre.
                vec3 ref = mix(vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), step(0.99, abs(d.y)));
                vec3 t1 = normalize(cross(d, ref));
                vec3 t2 = cross(d, t1);
                float e = 0.015;
                float hx = bouillon(pb + t1 * (e * ech));
                float hy = bouillon(pb + t2 * (e * ech));
                vec3 pente = ((hx - h0) * t1 + (hy - h0) * t2) / e;
                vec3 nb = normalize(d - pente * 0.06);
                vec3 lum = tourner(normalize(-0.55 * cam_right + 0.65 * cam_up + 0.55 * to_cam));
                // L'ombre du relief suit la clarté minimale : à 0,5 (d'origine)
                // elle descend à 0,60.
                col *= clamp(1.0 + 2.0 * (dot(nb, lum) - dot(d, lum)), ebul_reglage.y + 0.10, 1.6);
            }
            // Un bord assombri : la boule a du volume au lieu d'un anneau clair.
            col *= mix(1.0, mix(0.65, 1.0, sqrt(z)), ebullition);
        }

        if (aplatissement > 0.0) {
            // Pôles plus chauds, donc plus clairs ; équateur plus sombre.
            float pole = abs(dot(n, qrot(axe_rot, vec3(0.0, 1.0, 0.0))));
            col *= 1.0 + aplatissement * 1.2 * (pole - 0.5);
        }

        if (accretion_pol > 0.0) {
            // **Les taches d'accrétion** (lot 12) : une jeune étoile
            // avale son disque. Le gaz remonte le long des lignes de son champ
            // et s'écrase près des pôles — de l'axe de rotation, ici, celui
            // d'où partent les jets —, y creusant des taches chaudes,
            // grumeleuses et changeantes. Une couronne autour de chaque pôle.
            // Dipôle incliné : les taches suivent ses pôles, là où les
            // entonnoirs s'écrasent (`Disque::axe_magnetique`).
            float lat = dot(axe_magnetique, axe_magnetique) > 0.5 ? abs(dot(n, axe_magnetique)) : abs(d.y);
            float calotte = smoothstep(0.76, 0.86, lat) * (1.0 - 0.5 * smoothstep(0.95, 1.0, lat));
            float grumeaux = fbm(d * 7.0 + dec + vec3(time * 0.4, 0.0, time * 0.3));
            float choc = calotte * smoothstep(0.42, 0.68, grumeaux);
            col = mix(col, vec3(1.0, 0.96, 0.92), clamp(accretion_pol * choc, 0.0, 1.0));
        }

        if (coeur > 0.0) {
            // **Le cœur qui transparaît** (lot 11, essai) : la chaleur du trou
            // noir central traverse l'enveloppe là où on la voit de face, et
            // s'y perd vers le bord, où la visée traverse plus de matière. Une
            // respiration lente : l'accrétion n'est pas un robinet régulier.
            float face = pow(z, 6.0);
            float souffle = 0.8 + 0.2 * sin(time * 0.6 + graine);
            vec3 chaud = vec3(1.0, 0.82, 0.55);
            col = mix(col, chaud, clamp(coeur * face * souffle, 0.0, 1.0));
        }

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

        if (surface.z > 0.0) {
            // **Assombrissement centre-bord** (lot 10) : au bord, on voit des
            // couches plus hautes et plus froides de la photosphère. Loi
            // linéaire `1 − u(1 − μ)`, μ = z le cosinus de la visée.
            col *= 1.0 - surface.z * (1.0 - z);
        } else {
            // Bord renforcé (limb brightening) -> anneau lumineux.
            float rim = smoothstep(0.45, 0.0, z);
            col += rim * teinte * 0.55 * (1.0 - ebullition);
        }

        gl_FragColor = vec4(col, 1.0);
    } else if (couronne_type > 0.5 && couronne_type < 1.5) {
        // JETS bipolaires (pulsar / protoétoile) le long de l'axe vertical (vq.y).
        float ax = abs(vq.x);
        float ay = abs(vq.y);
        float width = 0.10 + 0.12 * (ay - 1.0);              // cône qui s'évase
        float core = 1.0 - smoothstep(0.0, width, ax);
        float along = smoothstep(couronne, 1.0, ay) * step(1.0, ay); // le long du jet
        float flow = fbm(vec3(vq.x * 9.0, vq.y * 3.5 - sign(vq.y) * time * 2.4, time * 0.2));
        float jet = core * along * (0.45 + 0.9 * flow);
        vec3 jetcol = mix(teinte, vec3(0.6, 0.8, 1.0), 0.55); // jets bleutés énergétiques
        gl_FragColor = vec4(jetcol, clamp(jet, 0.0, 1.0) * 0.9);
    } else if (couronne_type > 3.5) {
        // MAGNETAR : arcs de champ magnétique DIPOLAIRE (boucles brillantes, violettes).
        float g = clamp(1.0 - (rr - 1.0) / (couronne - 1.0), 0.0, 1.0);
        float sinth = abs(vq.x) / max(rr, 0.001);            // colatitude (axe vertical)
        float L = rr / max(sinth * sinth, 0.05);              // paramètre de ligne de champ (r = L sin²θ)
        float lines = abs(fract(L * 0.55 + time * 0.08) - 0.5) * 2.0; // 0 au cœur d'une boucle
        float arc = smoothstep(0.16, 0.0, lines) * g;
        arc *= 0.55 + 0.45 * fbm(vec3(vq * 5.0, time * 0.5)); // scintillement
        vec3 mcol = mix(teinte, vec3(0.72, 0.5, 1.0), 0.6);   // violet magnétique
        gl_FragColor = vec4(mcol, clamp(arc, 0.0, 1.0) * 0.85);
    } else if (couronne_type > 2.5) {
        // PULSAR : faisceau bipolaire qui TOURNE sur lui-même (effet phare) + flash périodique.
        float spin = time * 1.1;
        float cs = cos(spin), sn = sin(spin);
        vec2 q = vec2(vq.x * cs - vq.y * sn, vq.x * sn + vq.y * cs);
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
        float ang = atan(vq.y, vq.x);
        // Turbulence advectée radialement (le vent souffle vers l'extérieur).
        float turb = fbm(vec3(cos(ang) * 3.2, sin(ang) * 3.2, rr * 1.6 - time * 0.5) + 20.0);
        float clump = fbm(vec3(vq * 3.2, time * 0.15) + 40.0);
        float w = pow(g, 0.85) * (0.3 + 1.2 * turb) * (0.4 + 0.9 * clump);
        // Coquilles concentriques (ondes de choc du vent).
        w *= 0.7 + 0.3 * sin(rr * 9.0 - time * 1.2);
        vec3 windcol = mix(teinte, vec3(0.55, 0.5, 1.0), 0.45); // bleu-violet
        gl_FragColor = vec4(windcol, clamp(w, 0.0, 1.0) * 0.85);
    } else {
        // Couronne : décroît de r=1 à r=couronne ; forme modulée par couronne_irreg.
        float g = 1.0 - (rr - limbe) / (couronne - limbe);
        g = clamp(g, 0.0, 1.0);
        float ang = atan(vq.y, vq.x);
        float ray = fbm(vec3(cos(ang) * 3.0, sin(ang) * 3.0, time * 0.05) + 10.0);
        float spik = mix(1.0, 0.35 + 1.3 * ray, couronne_irreg);
        g = pow(g, 1.7) * spik;
        // Ébullition (essai) : la couronne s'effiloche en panaches qui bougent.
        float panache = fbm(vec3(cos(ang) * 2.2, sin(ang) * 2.2, time * 0.18) + 30.0);
        g *= mix(1.0, 0.35 + 1.5 * panache, ebullition);
        gl_FragColor = vec4(teinte, clamp(g, 0.0, 1.0) * 0.6);
    }
    // L'éclat, en dernier et pour toutes les formes : le disque s'éclaire,
    // la couronne s'épaissit. Le trou noir est déjà sorti plus haut.
    if (zone > 0.5) {
        gl_FragColor.a = clamp(gl_FragColor.a * eclat, 0.0, 1.0);
    } else {
        gl_FragColor.rgb *= eclat;
    }
}
