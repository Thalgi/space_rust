#version 100
precision highp float;
varying vec2 v_q;
uniform float time;
uniform vec3 cam_right;
uniform vec3 cam_up;
uniform vec3 to_cam;
uniform vec3 centre;
uniform float rayon;
uniform vec3 lumiere;
uniform vec3 couleur;
uniform vec3 couleur2;
uniform vec3 couleur3;
uniform vec3 light_color;
uniform float type_p;
uniform float eau;
uniform vec3 tache_dir;
uniform float tache_w;
uniform vec3 tache_col;
uniform vec3 axe;
uniform float band_scale;
uniform float warp_amt;
uniform float seed;
uniform float poly_cotes; // 0 = pas de vortex polaire, sinon nb de côtés
uniform float cyclones_pol;   // géantes : amas de cyclones aux pôles (0 = aucun)
uniform float thermique;      // géantes chaudes : émission thermique nocturne (0 = aucune)
uniform vec3 thermique_couleur; // teinte de l'émission thermique
uniform float tempetes;       // géantes : densité de petites tempêtes (ovales) (0 = aucune)
uniform float aurore;         // géantes : aurores polaires émissives (0 = aucune)
uniform vec3 aurore_couleur;  // teinte des aurores
uniform float brume;          // géantes : voile de brume qui adoucit les bandes (sub-Neptune)
uniform vec3 brume_couleur;   // teinte de la brume
uniform vec3 g_pole;          // géantes : teinte des régions polaires (dégradé latitudinal)
uniform float jet_profil;     // géantes : profil latitudinal type Jupiter (EZ + NEB/SEB) (0 = aucun)
uniform float tache_type;     // 0 = tache rouge (GRS), 1 = tache sombre (Neptune)
uniform vec3 atmo;        // halo atmosphérique (0 = aucun)
uniform float lave;       // monde de lave : fissures incandescentes (0 = aucun)
uniform float eau_motif;  // topologie de l'eau : 0 océan global,1 continents,2 mers,3 marais
uniform float grad_lat;   // contraste de biome équateur->pôle (0 = uniforme)
uniform float calotte;    // latitude (0..1) de début de banquise (1 = aucune)
uniform vec3 veg_couleur; // teinte de la végétation
uniform float veg_couv;   // couverture végétale (0 = aucune, sol nu)
uniform float rivieres;   // densité de rivières sur les terres (0 = aucune)
uniform float nuages;     // densité de la couche nuageuse (0 = ciel clair)
uniform vec3 nuages_couleur; // teinte des nuages (blanc, gris orage, sable...)
uniform float nuages_type; // 0 = classique, 1 = tempête sombre, 2 = cyclone spiralé
uniform float relief;     // amplitude des montagnes (0 = plat, 1 = chaînes marquées)
uniform float dunes;      // ondulations de dunes (ergs) sur les terres sèches (0 = aucune)
uniform float mesa;       // plateaux étagés + falaises + strates (0 = aucun)
uniform float pics;       // pics/aiguilles de glace (0 = aucun)
uniform float recifs;     // récifs/atolls turquoise sur les hauts-fonds (0 = aucun)
uniform float basalt;     // orgues basaltiques (cellules) sur les terres (0 = aucun)
uniform float voile;      // voile atmosphérique opaque qui cache le sol (Vénus/Titan)
uniform vec3 voile_couleur; // teinte du voile
uniform float crateres;   // cratères d'impact (mondes sans air) (0 = aucun)
uniform float eyeball;    // verrouillage de marée (0 = aucun)
uniform float eye_glace;  // angle solaire (1..-1) sous lequel la calotte gèle
uniform float eye_lave;   // 1 = zone subsolaire en lave/obsidienne
uniform float eye_ring;   // 1 = anneau de forêt au terminateur
uniform float cryo;       // cryovolcanisme : fractures cyan émissives (0 = aucun)
uniform float biolum;     // bioluminescence : lueur verte côté nuit (0 = aucun)
uniform float riv_lave;   // rivières de lave (incandescentes) au lieu d'eau (0 = eau)
uniform float villes;     // 1 = lumières de villes côté nuit, 0 = non colonisé
uniform sampler2D terrain; // atlas cube-sphere précalculé : R+G=altitude, B=flux, A=humidité
uniform float niveau_mer;  // niveau de la mer (quantile précalculé, -1 = pas d'océan)
uniform float atlas_n;     // résolution d'une face de l'atlas (texels)

// Distance signée à un polygone régulier à `n` côtés (négatif à l'intérieur).
float poly_dist(vec2 p, float r, float n) {
    float ang = atan(p.y, p.x);
    float seg = 6.2831853 / n;
    float a = mod(ang, seg) - seg * 0.5;
    return length(p) * cos(a) - r;
}

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

// Bruit cellulaire (Worley F1) : distance au point le plus proche d'une grille
// jitterée -> cellules (colonnes de basalte, écailles...).
float worley(vec3 p) {
    vec3 ip = floor(p);
    vec3 fp = fract(p);
    float d = 1.0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            for (int z = -1; z <= 1; z++) {
                vec3 g = vec3(float(x), float(y), float(z));
                vec3 o = vec3(hash(ip + g), hash(ip + g + 19.0), hash(ip + g + 41.0));
                d = min(d, length(g + o - fp));
            }
        }
    }
    return d;
}

// Direction sphère -> uv dans l'atlas cube-sphere 3×2 (gouttière de 1 texel).
// Doit rester le miroir exact de `planete/terrain.rs` (table FACES + warp tan).
vec2 dir_vers_atlas(vec3 d) {
    vec3 a = abs(d);
    float face; vec2 uv; float inv;
    if (a.x >= a.y && a.x >= a.z) {
        face = d.x >= 0.0 ? 0.0 : 1.0; inv = 1.0 / a.x;
        uv = vec2(d.x >= 0.0 ? -d.z : d.z, d.y) * inv;
    } else if (a.y >= a.z) {
        face = d.y >= 0.0 ? 2.0 : 3.0; inv = 1.0 / a.y;
        uv = vec2(d.x, d.y >= 0.0 ? -d.z : d.z) * inv;
    } else {
        face = d.z >= 0.0 ? 4.0 : 5.0; inv = 1.0 / a.z;
        uv = vec2(d.z >= 0.0 ? d.x : -d.x, d.y) * inv;
    }
    uv = atan(uv) * 1.2732395; // ×4/π : warp équi-angulaire inverse -> [-1,1]
    vec2 cell = vec2(mod(face, 3.0), floor(face / 3.0 + 0.001));
    float cote = atlas_n + 2.0;
    return (cell * cote + 1.0 + (uv * 0.5 + 0.5) * atlas_n) / vec2(cote * 3.0, cote * 2.0);
}

// Altitude 16 bits packée sur R (octet fort) + G (octet faible).
float altitude_atlas(vec4 t) {
    return (t.r * 255.0 * 256.0 + t.g * 255.0) / 65535.0;
}

vec3 surface(vec3 d, vec3 k, vec3 ld, out float wet) {
    wet = 0.0; // surface d'eau (pour le reflet spéculaire) ; mise à 1 sur l'océan
    if (type_p > 1.5) {
        // Glacée.
        float n = fbm(d * 4.0);
        vec3 base = mix(couleur, couleur2, n);
        return mix(base, vec3(1.0), smoothstep(0.55, 0.85, n) * 0.5);
    } else if (type_p > 0.5) {
        // Géante gazeuse : jets latitudinaux + TURBULENCE ADVECTÉE (curl-like, animée).
        // Les bandes sont une fonction de la latitude, perturbée par une turbulence
        // multi-octave qui dérive dans le temps -> festons, volutes et tourbillons vivants.
        float dk = dot(d, k);
        vec3 sd = vec3(seed, seed * 1.7, seed * 0.3);
        float t = time * 0.025;

        // Grande tache (vortex) : calculée d'abord pour spiraler le flot autour d'elle.
        float spot_amt = 0.0;
        float spot_r = 2.0;   // rayon normalisé dans la tache (0 = cœur)
        float spot_ang = 0.0; // angle autour du centre de la tache
        float wake = 0.0;     // sillage turbulent sur le flanc ouest de la tache
        vec3 dd = d; // direction d'échantillonnage, spiralée près de la tache
        if (tache_w > 0.0 && dot(d, tache_dir) > 0.0) {
            vec3 se1 = normalize(cross(tache_dir, k));
            vec3 se2 = cross(tache_dir, se1);
            vec2 sq = vec2(dot(d, se1), dot(d, se2));
            sq.x *= 0.6; // ovale
            float rr = length(sq) / tache_w;
            spot_r = rr;
            spot_ang = atan(sq.y, sq.x);
            // Bord légèrement irrégulier (intégré aux turbulences) mais corps bien opaque.
            float rn = rr + (fbm(d * 6.0 + sd + 5.0) - 0.5) * 0.18;
            spot_amt = 1.0 - smoothstep(0.55, 1.0, rn); // plein jusqu'à 55 % du rayon -> contours opaques
            // Sillage : flanc ouest (sq.x<0), traîne latitudinale, juste hors de la tache.
            float wlon = sq.x / tache_w;
            float wlat = sq.y / tache_w;
            wake = smoothstep(0.0, -0.5, wlon) * (1.0 - smoothstep(0.0, 1.3, abs(wlat)))
                 * smoothstep(0.9, 1.15, rr) * (1.0 - smoothstep(2.4, 2.9, rr));
            float ang = 1.2 / (rr + 0.3) + t * 4.0; // rotation DIFFÉRENTIELLE (whirlpool) -> enroulement
            float ca2 = cos(ang), sa2 = sin(ang);
            vec2 qr = vec2(sq.x * ca2 - sq.y * sa2, sq.x * sa2 + sq.y * ca2);
            dd = normalize(d + (se1 * (qr.x - sq.x) + se2 * (qr.y - sq.y)) * tache_w * 1.3);
        }

        // Échantillonnage ÉTIRÉ HORIZONTALEMENT : on compresse la composante zonale (est-ouest)
        // -> les tourbillons s'allongent le long des jets, plus de « pointes » verticales abruptes.
        vec3 dlat = k * dot(dd, k);            // composante latitudinale
        vec3 dh = dlat + (dd - dlat) * 0.5;    // zonal compressé -> bruit étiré

        // CURL NOISE (champ de vecteurs SANS DIVERGENCE, approx Gaseous Giganticus) : on déplace
        // l'échantillonnage perpendiculairement au gradient du bruit -> tourbillons fluides, non
        // étirés. Donne plus de variété de flot que le seul domain warping.
        {
            float e = 0.06;
            vec3 ta = normalize(cross(k, dh) + vec3(1e-4, 0.0, 0.0)); // tangente est-ouest
            vec3 tb = cross(dh, ta);                                  // tangente nord-sud
            float cx = fbm(dh * 3.0 + ta * e + sd + t) - fbm(dh * 3.0 - ta * e + sd + t);
            float cy = fbm(dh * 3.0 + tb * e + sd + t) - fbm(dh * 3.0 - tb * e + sd + t);
            dh += (ta * cy - tb * cx) * (0.5 + 0.35 * warp_amt); // rotation 90° = curl
        }

        // Domain warping multi-octave (2 niveaux) -> tourbillons ; le temps = advection.
        vec3 q1 = vec3(fbm(dh * 2.4 + sd), fbm(dh * 2.4 + sd + 5.2), fbm(dh * 2.4 + sd + 9.1)) - 0.5;
        vec3 q2 = vec3(fbm(dh * 2.4 + 2.6 * q1 + sd + t),
                       fbm(dh * 2.4 + 2.6 * q1 + sd + 7.3 - t),
                       fbm(dh * 2.4 + 2.6 * q1 + sd + 2.8)) - 0.5;
        float turb = fbm(dh * 4.0 + 2.4 * q2 + sd);
        float fine = fbm(dh * 13.0 + 3.0 * q2 + sd + 80.0); // détail haute fréquence (anti basse-déf)
        float swirl = fbm(dh * 6.0 + 3.0 * q2 + sd + 12.0) - 0.5; // octave de micro-tourbillons/filaments

        // BANDES ORGANIQUES (double-offset, façon Cosmos Journeyer) : au lieu de sin(latitude)
        // (périodique, régulier), deux fbm échantillonnés le long de la latitude et décalés par
        // le warping -> bandes aux largeurs variables, non périodiques, frontières ondulées.
        float bscale = band_scale * 0.3;
        float warp = ((turb - 0.5) * 0.7 + swirl * 0.4) * (0.6 + 0.3 * warp_amt)
                   + (turb - 0.5) * wake * 1.5; // ondulation des frontières + boost sillage
        float dec1 = fbm(vec3(dk * bscale + warp, sd.x, sd.y));            // décision principale
        float dec2 = fbm(vec3(dk * bscale * 1.9 - warp, sd.z + 4.0, sd.x)); // sous-bandes (offset opposé)
        // Cisaillement = proximité d'une frontière de bande (dec1 ~ 0.5) -> festons/turbulence là.
        float shear = 1.0 - smoothstep(0.0, 0.22, abs(dec1 - 0.5));
        float band = smoothstep(0.36, 0.64, dec1);
        band = clamp(band + (smoothstep(0.42, 0.58, dec2) - 0.5) * 0.5
                          + (fine - 0.5) * 0.16, 0.0, 1.0); // sous-bandes fines + grain

        // PROFIL TYPE JUPITER (jet_profil > 0) : large Zone Équatoriale claire bordée des
        // ceintures sombres NEB/SEB, par-dessus les bandes procédurales -> structure reconnaissable.
        if (jet_profil > 0.5) {
            float al = abs(dk);
            band = mix(band, 0.82, (1.0 - smoothstep(0.05, 0.18, al)) * 0.55);        // EZ claire (plafonnée, pas surexposée)
            float neb = smoothstep(0.18, 0.23, al) * (1.0 - smoothstep(0.34, 0.42, al));
            band = mix(band, 0.0, neb * 0.6);                                          // NEB/SEB sombres, larges
        }

        // Couleurs : ceinture sombre (belt) <-> zone claire ; courbe en S -> contraste marqué.
        vec3 zone = mix(couleur3, vec3(1.0, 0.98, 0.93), 0.3); // zones crème/ivoire éclatantes
        vec3 belt = couleur2;                                  // brique / ocre (ceintures sombres)
        float bandc = smoothstep(0.12, 0.88, band);
        bandc = smoothstep(0.0, 1.0, bandc);                   // courbe en S -> zones plus claires, belts plus sombres
        vec3 base = mix(belt, zone, bandc);
        float beltmask = 1.0 - smoothstep(0.30, 0.58, band); // dans les ceintures
        float zonemask = smoothstep(0.55, 0.85, band);       // dans les zones
        // Ceinture hôte de la Grande Tache (SEB) : grosse ceinture brun-rouge brique continue.
        if (jet_profil > 0.5 && tache_w > 0.0) {
            float slat = dot(tache_dir, k);                  // latitude de la tache
            float seb = 1.0 - smoothstep(0.04, 0.18, abs(dk - slat));
            seb *= 0.6 + 0.4 * beltmask;                     // surtout dans la ceinture, mais continue
            base = mix(base, vec3(0.66, 0.29, 0.16), seb * 0.7); // brun-rouge brique intense
            // Zone Tempérée Sud : fine bande ivoire qui ondule juste sous la Tache.
            float ond = (fbm(dd * 5.0 + sd + 22.0) - 0.5) * 0.05;
            float zts = 1.0 - smoothstep(0.0, 0.045, abs(dk - (slat - 0.135) + ond));
            base = mix(base, vec3(0.98, 0.96, 0.9), zts * 0.55 * (1.0 - spot_amt));
        }

        // Bandes sombres MARBRÉES : filaments chocolat + saumon (bruit étiré longitudinal).
        float marb = fbm(dd * 8.0 + vec3(turb * 3.0, 0.0, 0.0) + sd + 50.0);
        vec3 choco  = belt * vec3(0.62, 0.5, 0.46);                 // filaments chocolat
        vec3 saumon = mix(belt, vec3(0.92, 0.62, 0.5), 0.55);       // filaments saumon
        base = mix(base, choco,  beltmask * smoothstep(0.30, 0.05, marb) * 0.45);
        base = mix(base, saumon, beltmask * smoothstep(0.62, 0.85, marb) * 0.4);
        // Filaments internes plus clairs (ocre/brique) -> casse l'effet « bloc » uni.
        vec3 ocre = mix(belt, vec3(0.88, 0.62, 0.36), 0.55);
        float fil = fbm(dd * 11.0 + vec3(turb * 4.0, 0.0, 0.0) + sd + 55.0);
        base = mix(base, ocre, beltmask * smoothstep(0.55, 0.82, fil) * 0.45);
        base = mix(base, couleur, smoothstep(0.40, 0.62, turb) * 0.4 * (0.4 + 0.6 * shear));

        // Bandes claires LAITEUSES / floconneuses (cristaux d'ammoniac).
        float flake = fbm(dd * 14.0 + 2.0 * q2 + sd + 33.0);
        base = mix(base, mix(zone, vec3(1.0), 0.35), zonemask * smoothstep(0.5, 0.82, flake) * 0.4);

        // Festons / volutes / micro-tourbillons aux frontières (cisaillement élevé).
        float wisp = smoothstep(0.6, 0.86, fbm(dd * 7.0 + 4.0 * q2 + sd + 15.0));
        wisp = max(wisp, wake * smoothstep(0.45, 0.7, fine)); // chaos du sillage à gauche de la tache
        base = mix(base, mix(zone, vec3(1.0), 0.4), wisp * 0.35 * (shear + wake));
        // Festons bleu-gris (crochets sombres caractéristiques aux bords des ceintures).
        base = mix(base, base * vec3(0.68, 0.76, 0.85), wisp * (shear + wake) * 0.28);

        // Petites tempêtes (ovales blancs) advectées.
        float ov = fbm(dd * 5.0 + 2.0 * q2 + sd + 30.0);
        base = mix(base, mix(zone, vec3(1.0), 0.6), smoothstep(0.80, 0.90, ov) * 0.5);
        // Champ de tempêtes multiples : ovales clairs + cyclones sombres épars dans les bandes.
        if (tempetes > 0.0) {
            float st = fbm(dd * 6.5 + 3.0 * q2 + sd + 70.0);
            float clair = smoothstep(0.74, 0.80, st) * (1.0 - smoothstep(0.86, 0.92, st));
            base = mix(base, mix(zone, vec3(1.0), 0.7), clair * tempetes * 0.7);
            float st2 = fbm(dd * 5.5 + 2.0 * q1 + sd + 90.0);
            float sombre = smoothstep(0.78, 0.84, st2) * (1.0 - smoothstep(0.9, 0.95, st2));
            base = mix(base, belt * 0.65, sombre * tempetes * 0.5);
        }
        // Micro-détail global continu (élimine l'aspect basse résolution).
        base *= 0.96 + 0.12 * fine;
        // Ombrage subtil entre bandes (relief des nuages).
        base *= 1.0 + clamp((turb - 0.5) * 0.7, -0.2, 0.2);

        // PÔLES : dégradé brumeux bleu-gris / olive sombre, structuré en cyclones (Worley), pas en bandes.
        float la = abs(dk);
        float polef = smoothstep(0.32, 0.72, la); // engage à latitude moyenne-haute -> région polaire visible bleutée
        if (polef > 0.0) {
            vec3 pref = abs(k.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
            vec3 pe1 = normalize(cross(k, pref));
            vec3 pe2 = cross(k, pe1);
            vec2 pp = vec2(dot(d, pe1), dot(d, pe2)) * 5.0;
            float wpole = worley(vec3(pp, dk * 3.0) + sd + 60.0);
            vec3 olive = g_pole * vec3(0.9, 0.94, 0.74);                  // olive plus clair
            vec3 polcol = mix(g_pole, olive, smoothstep(0.2, 0.6, wpole));
            polcol = mix(polcol, polcol * 0.86, smoothstep(0.34, 0.1, wpole) * 0.4); // cœurs de cyclones discrets
            float lum = dot(polcol, vec3(0.33));
            polcol = mix(polcol, vec3(lum), 0.16);                        // légère désaturation feutrée
            base = mix(base, polcol, polef * 0.95);                       // calotte brumeuse dominante
        }
        // Équateur : zone ivoire PROPRE + fines traînées ocre/saumon claires (pas de beige sale).
        float eqf = (1.0 - smoothstep(0.0, 0.5, la));
        base = mix(base, vec3(1.0, 0.98, 0.92), eqf * zonemask * 0.45);          // ivoire propre
        float streak = smoothstep(0.55, 0.72, ov) * eqf * zonemask;
        base = mix(base, vec3(0.97, 0.85, 0.71), streak * 0.32);                 // traînées ocre/saumon claires

        // GRANDE TACHE : cyclone fluide INTÉGRÉ -> cœur orange-brique vif, bords beige rosé, collier crème.
        if (spot_amt > 0.0) {
            float spiral = 0.5 + 0.5 * sin(spot_ang * 2.0 + spot_r * 11.0 - t * 6.0);
            float finsp = fbm(dd * 20.0 + sd + 40.0);            // grain haute résolution
            if (tache_type < 0.5) {
                // Tache rouge (GRS) : VORTEX en coordonnées polaires distordues (technique whirlpool).
                // On enroule l'angle par 1/(r+s) -> bras en SPIRALE LOGARITHMIQUE (rotation différentielle,
                // plus serrée près du cœur). Du bruit échantillonné le long de la spirale = filaments.
                float swirl = 1.7 / (spot_r + 0.22);
                float pang = spot_ang + swirl - t * 2.5;
                float arms = fbm(vec3(pang * 1.3, spot_r * 5.0, sd.y + 50.0));
                arms = mix(0.5 + 0.5 * sin(pang * 3.0), arms, 0.6); // bandes spiralées + irrégularité
                vec3 coeur = tache_col * 1.28 + vec3(0.07, 0.01, 0.0);   // orange-brique vif
                vec3 bordr = mix(tache_col, vec3(0.96, 0.78, 0.7), 0.8); // beige rosé
                vec3 spotc = mix(coeur, bordr, smoothstep(0.0, 0.8, spot_r));
                spotc *= (0.74 + 0.42 * arms) * (0.92 + 0.16 * finsp); // bras spiralés + grain fin
                // Cœur calme et rouge profond (faible vorticité au centre).
                spotc = mix(spotc, tache_col * 0.8, smoothstep(0.3, 0.0, spot_r) * 0.5);
                // Anneau de HAUTE VITESSE à 70-85 % du rayon (pic de vorticité) : liseré vif.
                float velring = smoothstep(0.58, 0.72, spot_r) * (1.0 - smoothstep(0.82, 0.95, spot_r));
                spotc = mix(spotc, spotc * 1.32 + vec3(0.07, 0.02, 0.0), velring * 0.6);
                base = mix(base, spotc, spot_amt * 0.92);
                // Collier blanc/crème isolant.
                float collar = smoothstep(0.78, 1.0, spot_r) * (1.0 - smoothstep(1.0, 1.3, spot_r));
                base = mix(base, vec3(0.98, 0.94, 0.86), collar * 0.8);
            } else {
                // Tache sombre (Grande Tache Sombre de Neptune) : ovale très sombre, bords fondus,
                // sans collier crème (les nuages blancs compagnons viennent du sillage).
                vec3 coeur = tache_col * 0.55;
                vec3 bordd = mix(tache_col, base * 0.7, 0.5);
                vec3 spotc = mix(coeur, bordd, smoothstep(0.0, 0.92, spot_r));
                spotc *= 0.85 + 0.18 * finsp + 0.12 * spiral;
                base = mix(base, spotc, spot_amt * 0.85);
            }
        }
        // Sillage crème NET sur le flanc gauche (ouest) -> détache la tache du fond.
        if (wake > 0.0) {
            base = mix(base, vec3(0.97, 0.94, 0.87), wake * 0.5 * (1.0 - spot_amt));
        }
        // Vortex polaire polygonal (hexagone de Saturne, ou pentagone/octogone), au pôle nord.
        if (poly_cotes > 2.5 && dk > 0.55) {
            float cap = smoothstep(0.55, 0.72, dk);
            vec3 ref = abs(k.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
            vec3 e1 = normalize(cross(k, ref));
            vec3 e2 = cross(k, e1);
            vec2 pp = vec2(dot(d, e1), dot(d, e2)) * 1.6;
            float hd = poly_dist(pp, 0.33, poly_cotes);
            float bord = smoothstep(0.05, 0.0, abs(hd));
            float dedans = smoothstep(0.0, -0.06, hd);
            base = mix(base, base * 0.65, dedans * 0.6 * cap);
            // Eddies le long du jet-stream qui pincent le polygone (cellules Worley sur le bord).
            float edd = worley(vec3(pp * 4.0, dk * 5.0) + sd + 33.0);
            float lisere = bord * cap * (0.6 + 0.6 * smoothstep(0.45, 0.18, edd));
            base = mix(base, base * 1.45 + vec3(0.05), lisere);
            // Vortex polaire central (œil) qui stabilise l'hexagone : petit tourbillon sombre spiralé.
            float rc = length(pp);
            float eye = smoothstep(0.14, 0.0, rc);
            float swirl = 0.5 + 0.5 * sin(atan(pp.y, pp.x) * 2.0 + rc * 26.0 - t * 5.0);
            base = mix(base, base * (0.55 + 0.25 * swirl) + vec3(0.03, 0.02, 0.04), eye * cap * 0.7);
        }
        // Cyclones polaires : amas de tourbillons aux deux pôles (cellules Worley).
        if (cyclones_pol > 0.5) {
            float cap2 = smoothstep(0.58, 0.82, abs(dk));
            vec3 ref2 = abs(k.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
            vec3 ce1 = normalize(cross(k, ref2));
            vec3 ce2 = cross(k, ce1);
            vec2 cp = vec2(dot(d, ce1), dot(d, ce2)) * 5.5;
            float w = worley(vec3(cp, dk * 4.0) + sd + 60.0);
            base = mix(base, base * 0.8, smoothstep(0.42, 0.12, w) * cap2 * 0.35);   // cœurs sombres
            base = mix(base, base * 1.22 + vec3(0.03), smoothstep(0.34, 0.46, w) * cap2 * 0.4); // bords clairs
        }
        // Voile de brume : adoucit/efface les bandes (sub-Neptunes, hot Jupiters voilés).
        if (brume > 0.0) {
            base = mix(base, brume_couleur, brume);
        }
        return base;
    } else {
        // Tellurique. Latitude : 0 à l'équateur, 1 aux pôles (par rapport à l'axe).
        float lat = abs(dot(d, k));

        // GÉOGRAPHIE PRÉCALCULÉE (atlas cube-sphere, cf. conception_planete_v2.md) :
        // altitude 16 bits, flux d'écoulement et humidité viennent du CPU.
        // `p`/`sd` restent pour les features de style haute fréquence (dunes,
        // mesa, glace, cratères...) qui demeurent procédurales.
        float freq = eau_motif < 0.5 ? 1.6 : (eau_motif < 1.5 ? 2.4 : (eau_motif < 2.5 ? 1.5 : 4.5));
        vec3 sd = vec3(seed, seed * 1.7, seed * 0.3);
        vec3 p = d * freq + sd;
        vec4 geo = texture2D(terrain, dir_vers_atlas(d));
        float h = altitude_atlas(geo);       // altitude 0..1 (érodée à terme)
        float fluxr = geo.b;                 // flux d'eau (rivières/lacs, § 10)
        float moist = geo.a;                 // humidité grande échelle

        // Glace texturée (banquise + sommets) : plaques, réseau de fractures bleutées
        // profondes et éclats brillants (sastrugi) -> banquise vivante, pas un aplat.
        float ig = fbm(p * 3.5 + 60.0);
        float crack = smoothstep(0.80, 0.97, 1.0 - abs(2.0 * fbm(p * 6.0 + 70.0) - 1.0));
        float spark = smoothstep(0.86, 0.98, fbm(p * 12.0 + 80.0));
        vec3 glace = mix(vec3(0.80, 0.87, 0.97), vec3(0.99, 1.0, 1.0), ig);
        glace = mix(glace, vec3(0.5, 0.66, 0.85), crack * 0.6);  // crevasses profondes
        glace += vec3(0.05, 0.06, 0.07) * spark;                 // éclats de neige

        float sea = niveau_mer;              // quantile précalculé : couverture EXACTE (§ 9.1)
        vec3 base;
        if (eau > 0.001 && h < sea) {
            // Océan : sombre au large, plus clair (turquoise) sur les hauts-fonds côtiers.
            float prof = clamp((sea - h) / max(sea, 0.001), 0.0, 1.0);
            vec3 cotier = mix(couleur3, vec3(0.55, 0.85, 0.85), 0.35);
            base = mix(cotier, couleur3 * 0.5, prof);
            wet = 1.0; // océan -> reflet spéculaire dans main()
            // Récifs / atolls : taches turquoise vives sur les hauts-fonds.
            if (recifs > 0.0) {
                float rf = fbm(p * 4.0 + 100.0);
                float reef = smoothstep(0.55, 0.72, rf) * (1.0 - smoothstep(0.0, 0.4, prof));
                base = mix(base, vec3(0.45, 0.92, 0.85), reef * recifs);
            }
        } else {
            // Terre : étagement par altitude (côte -> plaine -> roche -> pic).
            float lh = (h - sea) / max(1.0 - sea, 0.001); // 0 au littoral .. 1 au sommet
            // Montagnes : bruit « ridged » (crêtes nettes) qui s'intensifie en altitude.
            float rg = 1.0 - abs(2.0 * fbm(p * 2.2 + 9.0) - 1.0);
            float mont = relief * smoothstep(0.30, 0.78, lh) * rg;

            vec3 rock = mix(couleur2, couleur, smoothstep(0.0, 0.55, lh));
            // Liseré de plage juste au-dessus du niveau de la mer.
            rock = mix(mix(couleur, vec3(0.85, 0.8, 0.6), 0.4), rock, smoothstep(0.0, 0.05, lh));
            // Végétation étagée : prairie claire en plaine -> forêt sombre vers les reliefs,
            // puis roche nue plus haut. Donne des bandes de biome contrastées.
            vec3 prairie = veg_couleur * 1.2 + vec3(0.06, 0.09, 0.0);
            vec3 foret = veg_couleur * 0.65;
            vec3 vcol = mix(prairie, foret, smoothstep(0.10, 0.42, lh));
            // `moist` est un RANG 0..1 (§ 11.2 bis) : seuil à 1-veg_couv ->
            // la végétation couvre la fraction demandée, sur les zones les
            // PLUS humides (berges, côtes, cuvettes) -> forêts-galeries.
            float sv = 1.0 - veg_couv;
            float veg = smoothstep(0.0, 0.2, veg_couv)
                      * smoothstep(0.60, 0.40, lh)
                      * smoothstep(sv - 0.10, sv + 0.06, moist)
                      * (1.0 - lat * 0.55) * (1.0 - mont);
            vec3 land = mix(rock, vcol, veg);
            // Dunes : ondulations parallèles déformées par le bruit (ergs), en plaine.
            if (dunes > 0.0) {
                float s = dot(d, k) * 26.0 + fbm(p * 1.3 + 80.0) * 9.0;
                float dm = dunes * (1.0 - smoothstep(0.35, 0.65, lh)) * (1.0 - veg);
                land = mix(land, couleur * 1.08, dm * 0.55); // zone sableuse distincte
                land *= 1.0 + 0.16 * sin(s) * dm;            // ondulations
            }
            // Mesas / canyons : altitude étagée (plateaux plats), strates colorées,
            // falaises assombries au bas de chaque palier.
            if (mesa > 0.0) {
                float steps = 6.0;
                float terr = floor(lh * steps) / steps;
                float frac = fract(lh * steps);
                vec3 strata = mix(couleur2, couleur, smoothstep(0.0, 0.55, terr));
                strata *= 0.88 + 0.24 * fract(terr * 6.3 + 0.5); // bandes alternées
                land = mix(land, strata, mesa);
                land *= 1.0 - 0.28 * smoothstep(0.14, 0.0, frac) * mesa; // falaises
            }
            // Pics de glace : fines aiguilles brillantes (ridged haute fréquence).
            if (pics > 0.0) {
                float sp = 1.0 - abs(2.0 * fbm(p * 6.0 + 90.0) - 1.0);
                float spike = smoothstep(0.86, 0.99, sp);
                land *= 1.0 - 0.18 * smoothstep(0.60, 0.86, sp) * (1.0 - spike) * pics; // ombre de base
                land = mix(land, vec3(0.95, 0.97, 1.0), spike * pics);
            }
            // Orgues basaltiques : cellules sombres (colonnes) à bords clairs, en plaine.
            if (basalt > 0.0) {
                float w = worley(d * 14.0 + sd);
                vec3 bas = mix(couleur2 * 0.55, couleur2 * 1.15, smoothstep(0.0, 0.18, w));
                land = mix(land, bas, basalt * (1.0 - smoothstep(0.45, 0.7, lh)));
            }
            // Cratères d'impact (mondes sans air) : fond sombre + bourrelet clair (Worley).
            if (crateres > 0.0) {
                float c = worley(p * 7.0 + 200.0);
                float fond = 1.0 - smoothstep(0.0, 0.18, c);
                float rim = smoothstep(0.15, 0.21, c) * (1.0 - smoothstep(0.21, 0.30, c));
                land *= 1.0 - 0.40 * fond * crateres;
                land += couleur * 0.20 * rim * crateres;
            }
            // Pics rocheux dénudés + assombrissement des crêtes (volume).
            land = mix(land, mix(couleur2, vec3(0.55), 0.4), smoothstep(0.62, 0.85, lh));
            land = mix(land, land * 0.72 + vec3(0.05), mont * 0.5);
            // Neige de sommet (descend plus bas aux hautes latitudes).
            float snow = smoothstep(0.62, 0.85, lh + mont * 0.4) * (0.35 + 0.65 * lat) * relief;
            land = mix(land, glace, clamp(snow, 0.0, 1.0));

            // RÉGIME hydrologique (§ 10.2) : le flux précalculé s'interprète.
            // Pas d'atmosphère ni de voile -> pas de liquide, JAMAIS (la Lune
            // ne doit pas avoir de flaques) ; air mais pas d'eau -> salines et
            // lits à sec ; riv_lave -> lave.
            float a_air = step(0.02, atmo.r + atmo.g + atmo.b + voile);
            float a_eau = step(0.0015, eau);
            // Régime lave : riv_lave explicite OU monde de lave (coulées § 11 bis).
            float regime_lave = max(riv_lave, step(0.3, lave));
            // Rivières : réseau d'écoulement PRÉCALCULÉ (flux D8, § 10).
            // Seuil piloté par `rivieres` -> contrôle artistique conservé ;
            // la largeur croît vers l'aval (le flux monte, la bande s'élargit).
            if ((rivieres > 0.0 || regime_lave > 0.5) && (a_air > 0.5 || regime_lave > 0.5)) {
                float seuil = mix(0.78, 0.50, max(rivieres, regime_lave * 0.7));
                float riv = smoothstep(seuil, seuil + 0.08, fluxr) * (1.0 - smoothstep(0.90, 0.94, fluxr));
                if (regime_lave > 0.5) {
                    land = mix(land, vec3(0.9, 0.35, 0.08), riv);       // coulée de lave
                    land = mix(land, vec3(1.0, 0.85, 0.4), riv * riv * 0.7); // cœur incandescent
                } else if (a_eau > 0.5) {
                    land = mix(land, couleur3 * 0.9, riv);
                    land = mix(land, veg_couleur, riv * 0.35 * veg_couv); // berges verdoyantes
                } else {
                    land = mix(land, couleur2 * 0.72, riv * 0.8); // oued : lit à sec sombre
                }
            }
            // Eau stagnante (lacs/mers de lave/salines, § 9.2) : flux saturé.
            float lac = smoothstep(0.93, 0.965, fluxr) * max(a_air, regime_lave);
            if (lac > 0.0) {
                if (regime_lave > 0.5) {
                    land = mix(land, vec3(0.95, 0.4, 0.1), lac);
                } else if (a_eau > 0.5) {
                    land = mix(land, couleur3 * 0.6, lac);
                    wet = max(wet, lac);
                } else {
                    // Saline / playa : croûte minérale claire, mate.
                    land = mix(land, mix(couleur, vec3(0.93, 0.9, 0.84), 0.7), lac);
                }
            }
            base = land;
            // (L'ombrage de pente est désormais fait par la NORMALE PERTURBÉE
            // dans main() : les versants réagissent à la vraie position du soleil.)
        }

        // TEMPÉRATURE locale (§ 11) : latitude + refroidissement en ALTITUDE
        // -> la neige descend sur les montagnes, les calottes suivent le climat.
        float froid = lat + max(h - max(sea, 0.0), 0.0) * 0.5 * relief;
        base = mix(base, mix(base, vec3(0.78, 0.83, 0.90), 0.7), froid * grad_lat);
        // Calottes : seuil sur la "température froide" perturbée par du bruit
        // multi-échelle -> côte de glace déchiquetée, jamais une ligne droite.
        float bord = froid
                   + (fbm(p * 1.3 + 40.0) - 0.5) * 0.42
                   + (fbm(p * 3.5 + 55.0) - 0.5) * 0.18;
        float gel = smoothstep(calotte, calotte + 0.05, bord);
        if (gel > 0.0) {
            // La glace N'EST PAS un autocollant : elle lit l'eau et le relief.
            vec3 gcol;
            if (eau > 0.001 && h < sea) {
                // BANQUISE : plaques et fractures (texture `glace`), liseré
                // bleuté le long des côtes -> le trait de côte reste lisible,
                // vieille banquise bleutée au large.
                float pr = clamp((sea - h) / max(sea, 0.001), 0.0, 1.0);
                gcol = mix(glace, vec3(0.62, 0.75, 0.9), smoothstep(0.25, 0.0, pr) * 0.5);
                gcol = mix(gcol, glace * vec3(0.82, 0.9, 1.03), smoothstep(0.5, 1.0, pr) * 0.3);
            } else {
                // GLACE TERRESTRE : neige éclatante sur les hauteurs, langues
                // glaciaires bleutées dans les vallées -> le relief transparaît.
                float lh2 = (h - max(sea, 0.0)) / max(1.0 - max(sea, 0.0), 0.001);
                gcol = mix(vec3(0.62, 0.74, 0.9), glace, smoothstep(0.10, 0.48, lh2));
                // Rivières et lacs GELÉS : veines de glace vive qui suivent le
                // réseau d'écoulement -> la géographie transparaît sous la calotte.
                float veine = max(smoothstep(0.55, 0.75, fluxr), smoothstep(0.93, 0.965, fluxr));
                gcol = mix(gcol, vec3(0.52, 0.7, 0.9), veine * 0.65);
            }
            base = mix(base, gcol, gel);
            wet *= 1.0 - gel; // la glace est solide : plus de reflet d'eau libre
        }

        // Couche de nuages : un bruit qui dérive au-dessus de la surface (par-dessus tout).
        if (nuages > 0.0) {
            // Deux couches qui dérivent à des vitesses/échelles différentes -> ciel vivant.
            float t1 = time * 0.015;
            float t2 = time * 0.032;
            float c1 = fbm(d * 2.2 + sd + vec3(t1, 0.0, t1 * 0.7));
            float c2 = fbm(d * 4.8 + sd + vec3(t2, 0.0, -t2 * 0.6));
            float cov = c1 * 0.65 + c2 * 0.35;
            float seuil_bas = 0.50;
            float seuil_haut = 0.78;
            vec3 ccol = nuages_couleur;

            if (nuages_type > 0.5 && nuages_type < 1.5) {
                // Tempête : couverture dense, cœurs sombres, fort contraste.
                seuil_bas = 0.34;
                seuil_haut = 0.60;
                ccol = mix(nuages_couleur, nuages_couleur * 0.35, smoothstep(0.62, 0.88, cov));
            } else if (nuages_type > 1.5) {
                // Cyclone : bras spiraux autour d'un centre, œil dégagé.
                vec3 cc = normalize(vec3(sin(seed * 1.7), 0.35, cos(seed)));
                vec3 e1 = normalize(cross(cc, vec3(0.0, 1.0, 0.0)) + vec3(1e-4));
                vec3 e2 = cross(cc, e1);
                vec2 qd = vec2(dot(d, e1), dot(d, e2));
                float rr2 = length(qd);
                float ang = atan(qd.y, qd.x);
                float dome = max(dot(d, cc), 0.0);
                float spiral = 0.5 + 0.5 * sin(ang * 2.0 + rr2 * 16.0 - time * 0.6);
                float eye = smoothstep(0.05, 0.14, rr2); // œil central dégagé
                cov = mix(cov, spiral * eye, smoothstep(0.25, 0.85, dome));
                seuil_bas = 0.40;
                seuil_haut = 0.70;
            }
            // OMBRES PORTÉES des nuages au sol : même champ échantillonné
            // décalé vers le soleil -> le sol s'assombrit sous les nuages.
            vec3 lt = ld - d * dot(ld, d);
            vec3 dsh = normalize(d + normalize(lt + vec3(1e-5)) * 0.05);
            float o1 = fbm(dsh * 2.2 + sd + vec3(t1, 0.0, t1 * 0.7));
            float o2 = fbm(dsh * 4.8 + sd + vec3(t2, 0.0, -t2 * 0.6));
            float ombre = smoothstep(seuil_bas, seuil_haut, o1 * 0.65 + o2 * 0.35);
            base *= 1.0 - 0.30 * ombre * nuages;

            float c = smoothstep(seuil_bas, seuil_haut, cov);
            base = mix(base, ccol, c * nuages);
        }
        // Voile atmosphérique opaque (Vénus, Titan) : cache le sol, fines bandes mouvantes.
        if (voile > 0.0) {
            float vn = fbm(d * 3.0 + sd + vec3(time * 0.01, 0.0, 0.0));
            base = mix(base, voile_couleur * (0.85 + 0.3 * vn), voile);
        }
        return base;
    }
}

void main() {
    float rr = length(v_q);
    if (rr > 1.0) discard;

    float z = sqrt(max(0.0, 1.0 - rr * rr));
    vec3 n = normalize(v_q.x * cam_right + v_q.y * cam_up + z * to_cam);

    vec3 surf = centre + n * rayon;
    vec3 L = normalize(lumiere - surf);
    float diff = max(dot(n, L), 0.0);

    // Rotation propre autour de l'axe (Rodrigues) -> bandes/pôles suivent l'inclinaison.
    vec3 k = normalize(axe);
    float a = time * 0.01;
    float ca = cos(a); float sa = sin(a);
    vec3 d = n * ca + cross(k, n) * sa + k * dot(k, n) * (1.0 - ca);

    // Lumière exprimée dans le repère TOURNÉ de la surface (même rotation que
    // n -> d) : nécessaire aux ombres de nuages calculées dans surface().
    vec3 ld = L * ca + cross(k, L) * sa + k * dot(k, L) * (1.0 - ca);
    float wet;
    vec3 albedo = surface(d, k, ld, wet);
    // Verrouillage de marée (eyeball) : la surface de base fait le "jour" ; on ajoute une
    // calotte glaciaire irrégulière côté nuit, un anneau de forêt au terminateur (option),
    // et une zone lave/obsidienne au point subsolaire (option, émissif plus bas).
    float eye_hot = 0.0;
    if (eyeball > 0.0) {
        float f = dot(n, L); // 1 = subsolaire, -1 = antisolaire
        float fr = f + (fbm(d * 1.6 + 90.0) - 0.5) * 0.5; // bord de glace irrégulier
        float ice = smoothstep(eye_glace + 0.06, eye_glace - 0.10, fr);
        albedo = mix(albedo, vec3(0.9, 0.94, 0.99), ice * eyeball);
        if (eye_ring > 0.5) {
            float ring = smoothstep(0.30, 0.12, abs(f - 0.05)) * (1.0 - ice);
            albedo = mix(albedo, vec3(0.14, 0.4, 0.17), ring * 0.75 * eyeball);
        }
        if (eye_lave > 0.5) {
            eye_hot = smoothstep(0.45, 0.82, f) * eyeball;
            albedo = mix(albedo, vec3(0.05, 0.045, 0.06), eye_hot * 0.85); // obsidienne
        }
    }
    // NORMALE PERTURBÉE (telluriques) : le gradient d'altitude bosselle la
    // sphère -> les versants accrochent la lumière selon la position réelle
    // du soleil. L'eau (wet) reste lisse.
    float diffb = diff;
    if (type_p < 0.5) {
        vec3 tb1 = normalize(cross(k, d) + vec3(1e-4));
        vec3 tb2 = cross(d, tb1);
        float eb = 0.012;
        float hb0 = altitude_atlas(texture2D(terrain, dir_vers_atlas(d)));
        float hb1 = altitude_atlas(texture2D(terrain, dir_vers_atlas(normalize(d + tb1 * eb))));
        float hb2 = altitude_atlas(texture2D(terrain, dir_vers_atlas(normalize(d + tb2 * eb))));
        float amp = relief * 5.0 * (1.0 - wet);
        vec3 db = normalize(d - tb1 * (hb1 - hb0) * amp - tb2 * (hb2 - hb0) * amp);
        // Retour au repère monde : rotation inverse (angle -a autour de k).
        vec3 nb = db * ca - cross(k, db) * sa + k * dot(k, db) * (1.0 - ca);
        diffb = max(dot(nb, L), 0.0);
    }
    vec3 lit = vec3(0.35) + light_color * (0.65 * diffb);
    vec3 col = albedo * lit;
    // Assombrissement du limbe (géantes gazeuses) : trajet atmosphérique vers les bords.
    if (type_p > 0.5 && type_p < 1.5) {
        float edge = mix(0.22, 1.0, smoothstep(0.0, 0.9, z)); // dégradé noir fort sur TOUT le contour
        col *= edge;
        float lum = dot(col, vec3(0.299, 0.587, 0.114));
        col = mix(vec3(lum), col, mix(0.5, 1.0, z)); // désaturation progressive vers les bords
    }
    // Reflet spéculaire du soleil sur l'eau (glint océanique, côté jour).
    if (wet > 0.0) {
        vec3 refl = reflect(-L, n);
        float spec = pow(max(dot(refl, to_cam), 0.0), 60.0);
        col += vec3(1.0, 0.97, 0.9) * spec * wet * diff;
    }
    // Mondes de lave : réseau de fissures incandescentes, émissif (brille de nuit).
    if (lave > 0.0) {
        // Varié par graine : échelle des fissures, finesse, teinte, + réseau fin secondaire.
        vec3 sdl = vec3(seed * 1.3, seed * 0.7, seed * 2.1);
        float fr = 4.0 + 6.0 * fract(seed * 0.123);
        float n = fbm(d * fr + sdl);
        float glow = pow(1.0 - abs(n - 0.5) * 2.0, 4.0 + 5.0 * fract(seed * 0.37));
        float n2 = fbm(d * fr * 2.3 + sdl + 7.0);
        glow += 0.4 * pow(1.0 - abs(n2 - 0.5) * 2.0, 9.0); // réseau fin
        vec3 col_lave = mix(vec3(1.0, 0.32, 0.04), vec3(1.0, 0.7, 0.18), fract(seed * 0.5));
        col += col_lave * glow * lave;
    }
    // COULÉES ET LACS DE LAVE ÉMISSIFS (§ 11 bis) : le réseau d'écoulement
    // brille la nuit (une rivière de lave n'est jamais sombre), avec une
    // pulsation lente -> volcanisme vivant. Suit le canal flux, pas un bruit.
    if (max(riv_lave, step(0.3, lave)) > 0.5 && type_p < 0.5) {
        vec4 g3 = texture2D(terrain, dir_vers_atlas(d));
        float sl = mix(0.78, 0.50, max(rivieres, 0.7));
        float coulee = smoothstep(sl, sl + 0.08, g3.b) * (1.0 - smoothstep(0.90, 0.94, g3.b));
        float lacl = smoothstep(0.93, 0.965, g3.b);
        float puls = 0.8 + 0.2 * sin(time * 0.7 + fbm(d * 3.0 + vec3(seed)) * 9.0);
        col += vec3(1.0, 0.42, 0.08) * (coulee * 0.85 + lacl * 0.7) * puls;
    }
    // Eyeball : coulées de lave incandescentes dans la zone subsolaire.
    if (eye_hot > 0.0) {
        float nh = fbm(d * 6.0 + 3.0);
        float glow = pow(1.0 - abs(nh - 0.5) * 2.0, 5.0);
        col += vec3(1.0, 0.4, 0.08) * glow * eye_hot;
    }
    // Cryovolcanisme : fractures cyan émissives (type Encelade/Triton).
    if (cryo > 0.0) {
        float v = 1.0 - abs(2.0 * fbm(d * 5.0 + 12.0) - 1.0);
        col += vec3(0.3, 0.7, 1.0) * smoothstep(0.82, 0.98, v) * cryo;
    }
    // Bioluminescence : la lueur SUIT LA GÉOGRAPHIE (§ 12) — forêts (humidité),
    // fleuves et côtes deviennent des réseaux de lumière côté nuit, au lieu de
    // taches plaquées au hasard.
    if (biolum > 0.0 && type_p < 0.5) {
        float nuit = 1.0 - smoothstep(0.0, 0.25, diff);
        vec4 g2 = texture2D(terrain, dir_vers_atlas(d));
        float h2 = altitude_atlas(g2);
        float grain = smoothstep(0.35, 0.75, fbm(d * 7.0 + 22.0)); // texture organique
        float veg2 = smoothstep(1.0 - veg_couv - 0.10, 1.0 - veg_couv + 0.06, g2.a)
                   * step(niveau_mer, h2) * grain;                  // forêts luminescentes
        float riv2 = smoothstep(0.55, 0.70, g2.b);                  // fleuves/lacs de lumière
        float cote = smoothstep(0.035, 0.0, abs(h2 - niveau_mer)) * step(0.0015, eau); // plancton côtier
        float lueur = max(max(veg2 * 0.55, riv2 * 0.9), cote * 0.8);
        col += vec3(0.2, 0.9, 0.55) * lueur * nuit * biolum;
    }
    // Lumières de villes (colonisation) : sur toute tellurique habitable (pas lave/voile),
    // propres à chaque planète (graine), sur la terre (1 - wet), côté nuit, regroupées en
    // réseaux : densité régionale (basse fréq) x grain fin de villes (haute fréq).
    if (villes > 0.01 && type_p < 0.5 && lave < 0.5 && voile < 0.5) {
        // villes : 0..1 monte l'intensité (demi-paliers), >1 étend la couverture.
        float force = clamp(villes, 0.0, 1.0);
        float ext = clamp(villes - 1.0, 0.0, 3.0);
        vec3 sdc = vec3(seed, seed * 1.7, seed * 0.3);
        float nuit = 1.0 - smoothstep(-0.05, 0.25, dot(n, L));
        float land = 1.0 - wet;
        float pop = smoothstep(0.50 - ext * 0.12, 0.78 - ext * 0.06, fbm(d * 3.5 + sdc + 5.0));
        float grain = smoothstep(0.60 - ext * 0.085, 0.80 - ext * 0.04, fbm(d * 26.0 + sdc + 50.0));
        float city = land * nuit * pop * grain;
        vec3 teinte = mix(vec3(1.0, 0.85, 0.55), vec3(0.75, 0.88, 1.0), fbm(d * 8.0 + sdc));
        col += teinte * city * (1.2 + ext * 0.15) * force;
    }
    // Émission thermique (géantes chaudes : classes IV/V, naine brune) : lueur côté nuit,
    // structurée par les bandes (zones plus chaudes).
    if (thermique > 0.0) {
        float nuit = 1.0 - smoothstep(-0.1, 0.35, diff);
        float gb = 0.65 + 0.35 * sin(dot(d, k) * band_scale);
        col += thermique_couleur * (nuit * thermique * gb);
    }
    // Aurores polaires (géantes) : anneaux émissifs scintillants autour des pôles,
    // plus visibles côté nuit.
    if (aurore > 0.0) {
        float la = abs(dot(d, k));
        float ring = smoothstep(0.78, 0.85, la) * (1.0 - smoothstep(0.9, 0.97, la));
        float flick = 0.55 + 0.45 * fbm(d * 9.0 + vec3(seed) + time * 0.3);
        float nuit = 1.0 - smoothstep(0.0, 0.22, diff); // strictement côté nuit
        col += aurore_couleur * (ring * flick * nuit * aurore);
    }
    // Halo atmosphérique : brille sur le limbe, surtout côté jour.
    float rim = pow(1.0 - z, 3.0);
    col += atmo * rim * (0.35 + 0.65 * diff);
    gl_FragColor = vec4(col, 1.0);
}
