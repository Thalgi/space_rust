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
// Éclairage multi-source (systèmes à plusieurs étoiles). L'indice 0 = étoile
// primaire (= lumiere/light_color) ; les entrées inutilisées ont une couleur nulle.
uniform vec3 lights_pos[4];
uniform vec3 lights_color[4];
uniform float type_p;
uniform float eau;
uniform vec3 tache_col;
// Vortex unifiés (phase 4, vortex.rs) : 8 slots. vortex[i].xyz = direction du
// centre, .w = type + rayon angulaire (type = floor : 0 GRS, 1 sombre,
// 2 ovale blanc, 3 barge, 4 chapelet ; inactif si fract ≈ 0).
// vortex2[i] : x = dérive (u du jet local), y = spin, z = index du slot.
uniform vec4 vortex[8];
uniform vec4 vortex2[8];
uniform vec3 axe;
uniform float warp_amt;
uniform float seed;
uniform float poly_cotes; // 0 = pas de vortex polaire, sinon nb de côtés
uniform float cyclones_pol;   // géantes : amas de cyclones aux pôles (0 = aucun)
uniform float thermique;      // géantes chaudes : émission thermique nocturne (0 = aucune)
uniform vec3 thermique_couleur; // teinte de l'émission thermique
uniform float aurore;         // géantes : aurores polaires émissives (0 = aucune)
uniform vec3 aurore_couleur;  // teinte des aurores
uniform float brume;          // géantes : voile de brume qui adoucit les bandes (sub-Neptune)
uniform vec3 brume_couleur;   // teinte de la brume
uniform vec3 g_pole;          // géantes : teinte des régions polaires (dégradé latitudinal)
// Palette paramétrique des gazeuses (dérivée CPU des couleurs du preset, cf.
// planete/palette.rs) : 0 fond de zone, 1 clair (flocons/ovales/équateur),
// 2 filaments sombres, 3 filaments chauds, 4 collier/sillage, 5 ceinture hôte
// de la tache, 6 bord de tache, 7 cellules polaires claires.
uniform vec3 gaz_pal[8];
// Profil zonal 1D précalculé (gazeuses, zonal.rs) : indexé par sin(latitude)
// (uv.x = dot(d, axe)*0.5+0.5). R = u(φ) vitesse de jet signée (0.5 = 0),
// G = b(φ) type de bande (0 belt .. 1 zone), B = s(φ) cisaillement 0..1.
uniform sampler2D zonal;
uniform float pole_lat;   // sin(latitude) où commence le régime polaire (zonal.rs)
uniform float px_rayon;   // rayon apparent en pixels -> LOD du micro-détail (phase 6)
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
uniform float cyclones_nb; // proportion (0..1) des emplacements de cyclones actifs
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

// Champ de cyclones tropicaux (telluriques) : plusieurs vortex COMPACTS répartis
// sur le globe, chacun avec œil dégagé + mur de l'œil dense + bras en SPIRALE
// LOGARITHMIQUE (θ ∝ ln r -> enroulement plus serré au centre), sens de rotation
// donné par l'hémisphère (Coriolis : antihoraire au nord, horaire au sud).
// ANTI-AUTOCOLLANT : en plus de la densité 0..1 retournée, la fonction
// - tord `dn` (direction d'échantillonnage des nuages de fond) en rotation
//   différentielle autour de chaque vortex -> la couverture existante est
//   ASPIRÉE en spirale (bandes d'alimentation), même mécanique que la grande
//   tache des géantes ;
// - accumule `clair` (0..1) : dégagement de l'œil, pour trouer le fond.
// Les centres DÉRIVENT lentement vers l'ouest (rotation autour de l'axe k,
// sens opposé par hémisphère). GLSL ES 100 (pas de continue/break).
float champ_cyclones(vec3 d, vec3 k, float t, inout vec3 dn, inout float clair) {
    float acc = 0.0;
    for (int i = 0; i < 6; i++) {
        float fi = float(i);
        // Centre pseudo-aléatoire, distribution ~uniforme sur la sphère.
        float h0 = hash(vec3(seed + fi * 11.3, 3.1, 7.7));
        float h1 = hash(vec3(seed + fi * 4.7, 9.2, 1.3));
        float h2 = hash(vec3(seed + fi * 2.9, 5.5, 8.1));
        float present = step(1.0 - cyclones_nb, h2);    // quantité pilotée par preset
        float z = 2.0 * h0 - 1.0;
        float ph = 6.2831853 * h1;
        float rho = sqrt(max(0.0, 1.0 - z * z));
        vec3 c = vec3(rho * cos(ph), z, rho * sin(ph));
        float spin = (dot(c, k) > 0.0) ? 1.0 : -1.0;    // Coriolis
        // Dérive lente autour de l'axe (Rodrigues) : conserve la latitude.
        float aw0 = spin * (0.015 + 0.02 * h1) * t;
        float cw = cos(aw0); float sw = sin(aw0);
        c = c * cw + cross(k, c) * sw + k * dot(k, c) * (1.0 - cw);
        // Cyclones surtout aux moyennes latitudes/tropiques (pas pile sur l'axe).
        float lat = abs(dot(c, k));
        present *= 1.0 - smoothstep(0.72, 0.95, lat);
        // Rayon angulaire : gros et lisible, mais RÉTRÉCIT quand ils sont
        // nombreux (sinon un monde-tempête sature en une bouillie de spirales).
        float R = (0.24 + 0.18 * h2) * (1.15 - 0.45 * cyclones_nb);
        float cd = dot(d, c);
        float front = smoothstep(0.15, 0.45, cd);       // face visible du centre (doux)
        // Coordonnées tangentes locales (projection gnomonique autour de c).
        vec3 up = abs(c.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
        vec3 e1 = normalize(cross(c, up));
        vec3 e2 = cross(c, e1);
        vec2 lp = vec2(dot(d, e1), dot(d, e2)) / max(cd, 1e-3);
        float r = length(lp) / R;                       // 0 = centre, 1 = bord nominal
        float ang = atan(lp.y, lp.x);
        float rot = t * (0.25 + 0.25 * h0);
        // ADVECTION du fond : rotation différentielle BORNÉE. Surtout PAS de
        // terme croissant avec le temps ici : le fond s'enroulerait à l'infini
        // -> anneaux concentriques (sillons de vinyle) à la périphérie. Le
        // mouvement vient de la dérive propre du fbm de fond (t1/t2) qui
        // TRAVERSE ce champ de torsion fixe.
        float pull = present * front * smoothstep(2.4, 0.5, r);
        float aw = spin * pull * 1.3 / (r + 0.5);
        float ca2 = cos(aw); float sa2 = sin(aw);
        vec2 q = vec2(lp.x * ca2 - lp.y * sa2, lp.x * sa2 + lp.y * ca2) - lp;
        dn += (e1 * q.x + e2 * q.y) * R * max(cd, 0.0);
        // Bras spiraux logarithmiques + rotation différentielle animée.
        float sp = 0.5 + 0.5 * sin(2.0 * ang + spin * 7.0 * log(r + 0.12) - spin * rot * 4.0);
        sp = pow(sp, 0.55);                             // bras ÉPAIS (sinon effet fil/emoji)
        sp *= 0.75 + 0.25 * fbm(d * 14.0 + fi + seed);  // grain fin le long des bras
        float eye = smoothstep(0.05, 0.12, r);          // œil central dégagé
        float env = smoothstep(1.0, 0.25, r);           // enveloppe : dense au cœur, fond au bord
        float wall = smoothstep(0.08, 0.13, r) * smoothstep(0.26, 0.15, r); // mur de l'œil
        float dens = max(sp * env * eye, wall * env * 0.95);
        acc = max(acc, dens * present * front);
        // Œil dégagé : troue aussi la couverture de fond.
        clair = max(clair, present * front * smoothstep(0.14, 0.05, r));
    }
    dn = normalize(dn);
    return clamp(acc, 0.0, 1.0);
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
        float t = time * 0.025;  // animation des vortex (tache, polygone...)
        float tn = time * 0.010; // dérive RÉSIDUELLE du bruit (le transport vient de l'advection)

        // ADVECTION DIFFÉRENTIELLE (phase 3, § 3.2) : chaque latitude tourne à
        // la vitesse de son jet u(φ) (canal R du profil zonal) -> les bandes
        // glissent réellement les unes contre les autres. Rotation exacte
        // autour de k : aucun enroulement cumulatif du bruit. La grande tache
        // reste ancrée au repère rigide : le flot cisaille AUTOUR d'elle,
        // comme la vraie GRS entre ses deux jets.
        float uz = (texture2D(zonal, vec2(dk * 0.5 + 0.5, 0.5)).r - 0.5) * 2.0;
        float az = uz * time * 0.025; // stylisé : cisaillement visible en quelques secondes
        float caz = cos(az); float saz = sin(az);
        vec3 dzn = d * caz + cross(k, d) * saz + k * dk * (1.0 - caz);

        // ---- CHAMP DE VORTEX UNIFIÉ (phase 4, § 5) : 8 slots CPU. ----
        // Chaque vortex TORD le fond advecté (aspiration bornée, anti-vinyle :
        // aucun terme croissant, c'est le fond qui traverse le champ) et le
        // slot DOMINANT au pixel garde ses coordonnées polaires pour le rendu.
        // Le slot 0 est la tache du preset ; tous dérivent le long de leur jet
        // (même horloge que l'advection -> ils RIDENT le flot).
        float spot_amt = 0.0;
        float spot_r = 2.0;    // rayon normalisé dans le vortex dominant
        float spot_ang = 0.0;  // angle polaire local
        float spot_type = -1.0;
        float spot_spin = 1.0;
        float spot_id = 0.0;
        float wake = 0.0;      // sillage turbulent (GRS seulement)
        vec3 dd = dzn;         // direction d'échantillonnage advectée + torsions
        float edgn = fbm(d * 6.0 + sd + 5.0) - 0.5; // bruit de bord PARTAGÉ par les slots
        for (int i = 0; i < 8; i++) {
            float vw = vortex[i].w;
            float vray = fract(vw);
            if (vray > 0.005) {
                float vtype = floor(vw + 0.001);
                // Dérive le long du jet local (u lu par le CPU dans le profil).
                vec3 c0 = vortex[i].xyz;
                float ad = vortex2[i].x * time * 0.025;
                float cad = cos(ad); float sad = sin(ad);
                vec3 c = c0 * cad + cross(k, c0) * sad + k * dot(k, c0) * (1.0 - cad);
                float cd = dot(d, c);
                if (cd > 0.25) {
                    float spin = vortex2[i].y;
                    vec3 e1 = normalize(cross(k, c) + vec3(1e-5, 0.0, 0.0)); // est local
                    vec3 e2 = normalize(cross(c, e1));                       // nord local
                    vec2 lp = vec2(dot(d, e1), dot(d, e2)) / max(cd, 0.3);
                    float front = smoothstep(0.25, 0.5, cd);
                    // Torsion du fond : rotation différentielle BORNÉE, portée
                    // élargie et plus musclée -> les filaments voisins s'enroulent
                    // visiblement autour du vortex (anti-autocollant).
                    float rt = length(lp) / vray;
                    float pull = front * smoothstep(3.0, 0.55, rt);
                    float awv = spin * pull * 1.6 / (rt + 0.5);
                    float ca2 = cos(awv); float sa2 = sin(awv);
                    vec2 qr = vec2(lp.x * ca2 - lp.y * sa2, lp.x * sa2 + lp.y * ca2) - lp;
                    dd += (e1 * qr.x + e2 * qr.y) * vray * max(cd, 0.0) * 1.3;
                    // Géométrie par type : ovale est-ouest, barge très allongée.
                    float ax = (vtype > 2.5 && vtype < 3.5) ? 0.38 : 0.6;
                    vec2 lq = vec2(lp.x * ax, lp.y);
                    float re = vray;
                    float inarc = 1.0;
                    if (vtype > 3.5) {
                        // Chapelet (« string of pearls ») : perles régulières le long du jet.
                        float arc = vray * 5.0;   // demi-étendue de l'arc
                        float pas = vray * 1.35;  // espacement des perles
                        inarc = 1.0 - smoothstep(arc * 0.7, arc, abs(lp.x));
                        lq = vec2((fract(lp.x / pas + 0.5) - 0.5) * pas, lp.y);
                        re = vray * 0.3;          // rayon d'une perle
                    }
                    float r = length(lq) / re;
                    // Bord rongé par la turbulence -> le vortex se FOND dans les bandes.
                    float rn = r + edgn * 0.45;
                    float amt = (1.0 - smoothstep(0.5, 1.0, rn)) * front * inarc;
                    if (amt > spot_amt) {
                        spot_amt = amt;
                        spot_r = r;
                        spot_ang = atan(lq.y, lq.x);
                        spot_type = vtype;
                        spot_spin = spin;
                        spot_id = vortex2[i].z;
                    }
                    // Sillage : GRS uniquement, flanc ouest, hors du corps.
                    if (vtype < 0.5) {
                        float wlon = lq.x / vray;
                        float wlat = lp.y / vray;
                        wake = max(wake, smoothstep(0.0, -0.5, wlon)
                             * (1.0 - smoothstep(0.0, 1.3, abs(wlat)))
                             * smoothstep(0.9, 1.15, r) * (1.0 - smoothstep(2.4, 2.9, r)));
                    }
                }
            }
        }
        dd = normalize(dd);

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
            float cx = fbm(dh * 3.0 + ta * e + sd + tn) - fbm(dh * 3.0 - ta * e + sd + tn);
            float cy = fbm(dh * 3.0 + tb * e + sd + tn) - fbm(dh * 3.0 - tb * e + sd + tn);
            dh += (ta * cy - tb * cx) * (0.5 + 0.35 * warp_amt); // rotation 90° = curl
        }

        // Domain warping multi-octave (2 niveaux) -> tourbillons ; le temps = advection.
        vec3 q1 = vec3(fbm(dh * 2.4 + sd), fbm(dh * 2.4 + sd + 5.2), fbm(dh * 2.4 + sd + 9.1)) - 0.5;
        vec3 q2 = vec3(fbm(dh * 2.4 + 2.6 * q1 + sd + tn),
                       fbm(dh * 2.4 + 2.6 * q1 + sd + 7.3 - tn),
                       fbm(dh * 2.4 + 2.6 * q1 + sd + 2.8)) - 0.5;
        float turb = fbm(dh * 4.0 + 2.4 * q2 + sd);
        // LOD (phase 6) : sous ~120 px de rayon apparent, le micro-détail
        // s'estompe -> anti-scintillement (galerie) et économies de fbm.
        float lod = smoothstep(30.0, 120.0, px_rayon);
        float fine = 0.5; // valeur neutre quand le détail est éteint
        if (lod > 0.02) {
            fine = mix(0.5, fbm(dh * 13.0 + 3.0 * q2 + sd + 80.0), lod);
        }
        float swirl = fbm(dh * 6.0 + 3.0 * q2 + sd + 12.0) - 0.5; // octave de micro-tourbillons/filaments

        // PROFIL ZONAL (texture 1D précalculée, CONCEPTION_GAZEUSES_V2 § 3) :
        // la STRUCTURE (latitudes/largeurs des bandes, cisaillement) vient du
        // CPU ; le bruit n'a plus qu'à ONDULER les frontières (warp) et poser
        // des sous-bandes fines. Remplace dec1 + jet_profil de la V1.
        float warp = ((turb - 0.5) * 0.7 + swirl * 0.4) * (0.6 + 0.3 * warp_amt)
                   + (turb - 0.5) * wake * 1.5; // ondulation des frontières + boost sillage
        // ANTI-AUTOCOLLANT : la latitude de lecture vient de la direction
        // TORDUE par les vortex (dot(dd,k)), pas du pixel -> les bandes se
        // COURBENT autour de la tache au lieu de passer derrière en ligne droite.
        float dkv = dot(dd, k);
        vec4 zp = texture2D(zonal, vec2(clamp(dkv + warp * 0.16, -0.99, 0.99) * 0.5 + 0.5, 0.5));
        float shear = zp.b;                    // cisaillement réel : festons aux flancs des jets
        float dec2 = fbm(vec3(dk * 7.5 - warp, sd.z + 4.0, sd.x)); // sous-bandes fines
        float band = zp.g;
        band = clamp(band + (smoothstep(0.42, 0.58, dec2) - 0.5) * 0.35
                          + (fine - 0.5) * 0.10, 0.0, 1.0); // sous-bandes + grain (réduit : anti-moucheté)

        // Couleurs : ceinture sombre (belt) <-> zone claire ; courbe en S -> contraste marqué.
        // Toutes les teintes de détail viennent de gaz_pal (palette paramétrique).
        vec3 zone = gaz_pal[0]; // fond des zones claires
        vec3 belt = couleur2;   // ceintures sombres
        float bandc = smoothstep(0.12, 0.88, band);
        bandc = smoothstep(0.0, 1.0, bandc);                   // courbe en S -> zones plus claires, belts plus sombres
        vec3 base = mix(belt, zone, bandc);
        float beltmask = 1.0 - smoothstep(0.30, 0.58, band); // dans les ceintures
        float zonemask = smoothstep(0.55, 0.85, band);       // dans les zones
        // Ceinture hôte de la Grande Tache (SEB) : ceinture continue teintée de
        // la tache (slot 0 de type GRS seulement ; les sombres n'en ont pas).
        if (fract(vortex[0].w) > 0.005 && floor(vortex[0].w + 0.001) < 0.5) {
            float slat = dot(vortex[0].xyz, k);              // latitude de la tache
            float seb = 1.0 - smoothstep(0.04, 0.18, abs(dk - slat));
            seb *= 0.6 + 0.4 * beltmask;                     // surtout dans la ceinture, mais continue
            base = mix(base, gaz_pal[5], seb * 0.7);         // ceinture hôte (teinte de la tache)
            // Zone Tempérée Sud : fine bande claire qui ondule juste sous la Tache.
            float ond = (fbm(dd * 5.0 + sd + 22.0) - 0.5) * 0.05;
            float zts = 1.0 - smoothstep(0.0, 0.045, abs(dk - (slat - 0.135) + ond));
            base = mix(base, gaz_pal[1], zts * 0.55 * (1.0 - spot_amt));
        }

        // Bandes sombres MARBRÉES : filaments sombres + chauds (bruit étiré longitudinal).
        float marb = fbm(dd * 8.0 + vec3(turb * 3.0, 0.0, 0.0) + sd + 50.0);
        base = mix(base, gaz_pal[2], beltmask * smoothstep(0.30, 0.05, marb) * 0.45);
        base = mix(base, gaz_pal[3], beltmask * smoothstep(0.62, 0.85, marb) * 0.4);
        // Filaments internes plus clairs -> casse l'effet « bloc » uni.
        float fil = 0.5;
        if (lod > 0.02) {
            fil = mix(0.5, fbm(dd * 11.0 + vec3(turb * 4.0, 0.0, 0.0) + sd + 55.0), lod);
        }
        base = mix(base, gaz_pal[3], beltmask * smoothstep(0.55, 0.82, fil) * 0.45);
        base = mix(base, couleur, smoothstep(0.40, 0.62, turb) * 0.4 * (0.4 + 0.6 * shear));

        // Bandes claires LAITEUSES / floconneuses (cristaux d'ammoniac).
        float flake = 0.5;
        if (lod > 0.02) {
            flake = mix(0.5, fbm(dd * 14.0 + 2.0 * q2 + sd + 33.0), lod);
        }
        base = mix(base, mix(zone, gaz_pal[1], 0.6), zonemask * smoothstep(0.5, 0.82, flake) * 0.4);

        // Festons / volutes / micro-tourbillons aux frontières (cisaillement élevé).
        float wisp = smoothstep(0.6, 0.86, fbm(dd * 7.0 + 4.0 * q2 + sd + 15.0));
        wisp = max(wisp, wake * smoothstep(0.45, 0.7, fine)); // chaos du sillage à gauche de la tache
        base = mix(base, mix(zone, gaz_pal[1], 0.65), wisp * 0.35 * (shear + wake));
        // Festons bleu-gris (crochets sombres caractéristiques aux bords des ceintures).
        base = mix(base, base * vec3(0.68, 0.76, 0.85), wisp * (shear + wake) * 0.28);

        // Champ doux pour les traînées équatoriales (les ovales et tempêtes
        // sont désormais de VRAIS vortex en slots, plus des seuils de bruit).
        float ov = fbm(dd * 5.0 + 2.0 * q2 + sd + 30.0);
        // Micro-détail global continu, DISCRET (trop fort = aspect moucheté).
        base *= 0.98 + 0.05 * fine;
        // Ombrage subtil entre bandes (relief des nuages).
        base *= 1.0 + clamp((turb - 0.5) * 0.7, -0.2, 0.2);

        // (Pôles V2 : le régime polaire complet est rendu APRÈS les vortex,
        //  voir le bloc « PÔLES V2 » plus bas — plus de calotte envahissante.)
        float la = abs(dk);
        // Équateur : zone claire PROPRE + fines traînées chaudes (pas de beige sale).
        float eqf = (1.0 - smoothstep(0.0, 0.5, la));
        base = mix(base, gaz_pal[1], eqf * zonemask * 0.45);                     // clair propre
        float streak = smoothstep(0.55, 0.72, ov) * eqf * zonemask;
        base = mix(base, mix(gaz_pal[1], gaz_pal[3], 0.5), streak * 0.32);       // traînées chaudes claires

        // ---- RENDU DU VORTEX DOMINANT (phase 4) : intégré aux bandes. ----
        if (spot_amt > 0.0) {
            float finsp = 0.5; // grain haute résolution (sous LOD)
            if (lod > 0.02) {
                finsp = mix(0.5, fbm(dd * 20.0 + sd + 40.0), lod);
            }
            if (spot_type < 0.5) {
                // GRS : bras en spirale log par FBM PUR (fini le sinus « vinyle »).
                float swl = spot_spin * 1.9 * log(spot_r + 0.15);
                float pang = spot_ang + swl - t * 2.2 * spot_spin;
                float arms = fbm(vec3(pang * 1.1, spot_r * 4.5, sd.y + 50.0 + spot_id));
                vec3 coeur = tache_col * 1.35;                       // cœur vif (teinte du preset)
                vec3 spotc = mix(coeur, gaz_pal[6], smoothstep(0.0, 0.8, spot_r));
                spotc *= (0.72 + 0.5 * arms) * (0.92 + 0.16 * finsp);
                // Cœur calme et profond (faible vorticité au centre).
                spotc = mix(spotc, tache_col * 0.8, smoothstep(0.3, 0.0, spot_r) * 0.5);
                // Anneau de HAUTE VITESSE à 70-85 % du rayon : liseré vif.
                float velring = smoothstep(0.58, 0.72, spot_r) * (1.0 - smoothstep(0.82, 0.95, spot_r));
                spotc = mix(spotc, spotc * 1.4, velring * 0.6);
                // ANTI-AUTOCOLLANT : la luminance des bandes locales transparaît
                // dans la tache (elle appartient à SA ceinture, pas posée dessus).
                spotc *= 0.86 + 0.28 * bandc;
                base = mix(base, spotc, spot_amt * (0.8 + 0.12 * smoothstep(0.6, 0.2, spot_r)));
                // Collier clair : chapelet de nuages IRRÉGULIER (modulé par le
                // flot), pas un halo uniforme.
                float collar = smoothstep(0.78, 1.0, spot_r) * (1.0 - smoothstep(1.0, 1.3, spot_r));
                collar *= 0.35 + 0.75 * smoothstep(0.35, 0.75, ov);
                base = mix(base, gaz_pal[4], collar * 0.85);
            } else if (spot_type < 1.5) {
                // Tache sombre (GDS) : ovale sombre fondu, sans collier (les
                // compagnons blancs viennent du sillage/festons alentour).
                vec3 tcol = spot_id < 0.5 ? tache_col : couleur2 * 0.45;
                vec3 coeur = tcol * 0.55;
                vec3 bordd = mix(tcol, base * 0.7, 0.5);
                vec3 spotc = mix(coeur, bordd, smoothstep(0.0, 0.92, spot_r));
                spotc *= 0.85 + 0.2 * finsp;
                base = mix(base, spotc, spot_amt * 0.85);
            } else if (spot_type < 2.5) {
                // Ovale blanc : anticyclone compact brillant à cœur calme,
                // assis dans sa bande par un fin liseré d'ombre.
                float swl2 = spot_spin * 1.5 * log(spot_r + 0.2);
                float arms2 = fbm(vec3((spot_ang + swl2 - t * 2.5 * spot_spin) * 1.3,
                                       spot_r * 5.0, sd.x + 70.0 + spot_id));
                vec3 spotc = mix(gaz_pal[1], vec3(1.0), 0.25) * (0.9 + 0.18 * arms2);
                spotc = mix(spotc, base * 0.72, smoothstep(0.75, 1.0, spot_r) * 0.5);
                base = mix(base, spotc, spot_amt * 0.85);
            } else if (spot_type < 3.5) {
                // Barge brune : cyclone allongé sombre, filaments internes.
                float fil2 = fbm(vec3(spot_ang * 0.8 + spot_r * 2.0,
                                      spot_r * 6.0 - t * 1.5, sd.z + 90.0 + spot_id));
                vec3 spotc = gaz_pal[2] * 0.85;
                spotc = mix(spotc, gaz_pal[3] * 0.8, smoothstep(0.55, 0.85, fil2) * 0.5);
                spotc *= 0.9 + 0.14 * finsp;
                spotc = mix(spotc, gaz_pal[1] * 0.9, smoothstep(0.7, 1.0, spot_r) * 0.25);
                base = mix(base, spotc, spot_amt * 0.8);
            } else {
                // Perle de chapelet : petit ovale blanc simple.
                vec3 spotc = gaz_pal[1] * (0.92 + 0.14 * finsp);
                spotc = mix(spotc, base * 0.75, smoothstep(0.7, 1.0, spot_r) * 0.4);
                base = mix(base, spotc, spot_amt * 0.8);
            }
        }
        // Sillage clair sur le flanc ouest : traîne DÉCHIQUETÉE par le flot
        // (modulée par le champ ov), pas une bande crème uniforme.
        if (wake > 0.0) {
            float wmod = 0.3 + 0.55 * smoothstep(0.3, 0.8, ov);
            base = mix(base, gaz_pal[4], wake * wmod * (1.0 - spot_amt));
        }
        // ---- PÔLES V2 (phase 5, § 6) : UN SEUL système polaire. ----
        // Emprise RÉDUITE : engage après la dernière paire de jets (pole_lat,
        // borne calculée par le CPU depuis le profil zonal), pleine vers ~80°.
        // La montée de turbulence en amont vient de s(φ) (phase 2) : la
        // transition est structurelle, pas un fondu envahissant.
        float polef = smoothstep(pole_lat, min(pole_lat + 0.10, 0.995), la);
        if (polef > 0.0) {
            float hemi = dk >= 0.0 ? 1.0 : -1.0;
            vec3 pref = abs(k.y) < 0.9 ? vec3(0.0, 1.0, 0.0) : vec3(1.0, 0.0, 0.0);
            vec3 pe1 = normalize(cross(k, pref));
            vec3 pe2 = cross(k, pe1);
            // PROJECTION AZIMUTALE correcte : ρ = angle au pôle (fini la
            // distorsion Worley de la projection plane), θ = longitude.
            // dzn -> la calotte suit l'advection résiduelle (u→0 au pôle).
            float rho = acos(clamp(la, 0.0, 1.0));
            float theta = atan(dot(dzn, pe2) * hemi, dot(dzn, pe1));
            vec2 pq = vec2(rho * cos(theta), rho * sin(theta));

            // 1) Fond feutré : cellules Worley azimutales, désaturé.
            float wpole = worley(vec3(pq * 5.5, hemi * 3.0) + sd + 60.0);
            vec3 polcol = mix(g_pole, gaz_pal[7], smoothstep(0.2, 0.6, wpole));
            polcol = mix(polcol, polcol * 0.86, smoothstep(0.34, 0.1, wpole) * 0.4);
            float lum = dot(polcol, vec3(0.33));
            polcol = mix(polcol, vec3(lum), 0.16);
            polcol *= 0.97 + 0.06 * fine; // même grain que le reste -> pas d'aplat

            // 2) Anneau de cyclones (config Juno) : N cyclones autour du vortex
            //    central, N DIFFÉRENT par hémisphère, rotation lente opposée.
            if (cyclones_pol > 0.5) {
                float ncyc = 5.0 + mod(floor(seed * 13.7) + max(hemi, 0.0) * 3.0, 4.0);
                float rr0 = 0.30;                   // rayon (rad) de l'anneau
                float seg = 6.2831853 / ncyc;
                float thr = theta - hemi * t * 0.5; // l'anneau dérive lentement
                float aj = (floor(thr / seg) + 0.5) * seg + hemi * t * 0.5;
                vec2 cc = vec2(rr0 * cos(aj), rr0 * sin(aj));
                vec2 lpc = (pq - cc) / 0.15;        // rayon d'un cyclone ~0.15 rad
                float rc2 = length(lpc);
                // Bras spiralés fbm + œil sombre, bord rongé par le bruit partagé.
                float acy = atan(lpc.y, lpc.x);
                float armc = fbm(vec3((acy + hemi * 2.2 * log(rc2 + 0.2) - hemi * t * 3.0) * 1.2,
                                      rc2 * 3.5, sd.x + 44.0 + aj));
                float cycm = 1.0 - smoothstep(0.55, 1.0, rc2 + edgn * 0.3);
                polcol = mix(polcol, mix(g_pole * 0.8, gaz_pal[7] * 1.12, armc), cycm * 0.8);
                polcol = mix(polcol, g_pole * 0.7, smoothstep(0.18, 0.0, rc2) * 0.6);
            }

            // 3) Vortex central : tourbillon sombre à bras fbm (pas de sinus).
            float rcen = rho / 0.15;
            if (rcen < 1.6) {
                float acen = atan(pq.y, pq.x);
                float armz = fbm(vec3((acen + hemi * 2.4 * log(rcen + 0.18) - hemi * t * 2.0) * 1.2,
                                      rcen * 3.0, sd.y + 61.0));
                float cenm = 1.0 - smoothstep(0.5, 1.3, rcen + edgn * 0.25);
                polcol = mix(polcol, g_pole * (0.62 + 0.3 * armz), cenm * 0.75);
            }

            // 4) Polygone (hexagone de Saturne) : le CONTOUR du jet polaire,
            //    pôle nord seulement (comme le vrai). Bord ondulé par le bruit,
            //    pincé d'eddies Worley, rotation lente -> il ÉMERGE du régime
            //    polaire au lieu d'être tamponné dessus.
            if (poly_cotes > 2.5 && hemi > 0.0) {
                float rot = t * 0.25;
                vec2 ph = vec2(pq.x * cos(rot) - pq.y * sin(rot),
                               pq.x * sin(rot) + pq.y * cos(rot)) * 1.55;
                float hd = poly_dist(ph, 0.33, poly_cotes);
                hd += (fbm(vec3(ph * 5.0, sd.z + 21.0)) - 0.5) * 0.045; // bord VIVANT
                float dedans = smoothstep(0.0, -0.06, hd);
                float bordp = smoothstep(0.05, 0.0, abs(hd));
                polcol = mix(polcol, polcol * 0.68, dedans * 0.55);
                float edd = worley(vec3(ph * 4.0, hemi * 5.0) + sd + 33.0);
                float lisere = bordp * (0.6 + 0.6 * smoothstep(0.45, 0.18, edd));
                polcol = mix(polcol, polcol * 1.45 + vec3(0.05), lisere);
            }

            base = mix(base, polcol, polef);
        }
        // Voile de brume INÉGAL (phase 6) : variation très basse fréquence et
        // ceintures qui percent légèrement -> monde voilé, pas délavé.
        if (brume > 0.0) {
            float bv = brume * (0.8 + 0.2 * fbm(dzn * 1.6 + sd + 120.0));
            bv *= 0.75 + 0.25 * zp.g; // les belts (sombres, profondes) percent un peu
            base = mix(base, brume_couleur, clamp(bv, 0.0, 1.0));
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
            // Cyclones (nuages_type 2) : calculés AVANT le fond, car ils tordent
            // la direction d'échantillonnage -> la couverture est aspirée dedans.
            vec3 dnu = d;
            float cyc = 0.0;
            float oeil = 0.0;
            if (nuages_type > 1.5) { cyc = champ_cyclones(d, k, time, dnu, oeil); }
            float c1 = fbm(dnu * 2.2 + sd + vec3(t1, 0.0, t1 * 0.7));
            float c2 = fbm(dnu * 4.8 + sd + vec3(t2, 0.0, -t2 * 0.6));
            float cov = c1 * 0.65 + c2 * 0.35;
            // EYEBALL : capuchon de tempête permanent au-dessus du point
            // subsolaire (la zone chaude évapore en continu), bord effiloché.
            if (eyeball > 0.0) {
                float sub = smoothstep(0.5, 0.92, dot(d, ld));
                cov = max(cov, sub * (0.55 + 0.30 * fbm(d * 6.0 + sd + vec3(time * 0.02, 0.0, 0.0))));
            }
            float seuil_bas = 0.50;
            float seuil_haut = 0.78;
            vec3 ccol = nuages_couleur;

            if (nuages_type > 0.5 && nuages_type < 1.5) {
                // Tempête : couverture dense, cœurs sombres, fort contraste.
                seuil_bas = 0.34;
                seuil_haut = 0.60;
                ccol = mix(nuages_couleur, nuages_couleur * 0.35, smoothstep(0.62, 0.88, cov));
            } else if (nuages_type > 1.5) {
                // Cyclones : le fond garde sa PLEINE densité (c'est LUI qui
                // spirale, via l'advection) ; le vortex s'y fond en RENFORÇANT
                // la couverture locale, l'œil troue tout.
                cov = max(cov, mix(cov, 1.0, cyc * 0.85)) * (1.0 - oeil);
                ccol = mix(nuages_couleur, vec3(1.0), 0.35 * cyc); // murs plus blancs
                seuil_bas = 0.42;
                seuil_haut = 0.72;
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
        // GLACE TEXTURÉE (pas un aplat blanc) : mince et translucide près du
        // bord (la surface transparaît -> anneau de "slush"), épaisse et
        // bleutée vers la nuit profonde, parcourue de fractures.
        float fx = fbm(d * 5.0 + 31.0);
        float veine = 1.0 - abs(2.0 * fx - 1.0);
        vec3 gcol = mix(vec3(0.87, 0.91, 0.97), vec3(0.58, 0.72, 0.9), smoothstep(0.0, -0.75, f));
        gcol = mix(gcol, vec3(0.5, 0.67, 0.88), smoothstep(0.7, 0.95, veine) * 0.55); // fractures bleutées
        vec3 mince = mix(albedo, gcol, 0.55);              // glace mince : la surface dessous
        gcol = mix(mince, gcol, smoothstep(0.10, -0.30, fr)); // épaississement vers la nuit
        albedo = mix(albedo, gcol, ice * eyeball);
        wet *= 1.0 - ice * eyeball; // l'eau gelée ne fait plus de reflet spéculaire
        if (eye_ring > 0.5) {
            // Anneau de vie au terminateur : bord rongé par le bruit, teinte du preset.
            float rb = (fbm(d * 4.0 + 17.0) - 0.5) * 0.22;
            float ring = smoothstep(0.30, 0.12, abs(f - 0.05) + rb) * (1.0 - ice);
            albedo = mix(albedo, veg_couleur * 0.85, ring * 0.8 * eyeball);
        }
        if (eye_lave > 0.5) {
            // Zone subsolaire : obsidienne variée de plaques de croûte, bord bruité.
            float ncr = fbm(d * 7.0 + 55.0);
            eye_hot = smoothstep(0.45, 0.82, f + (ncr - 0.5) * 0.2) * eyeball;
            vec3 obs = mix(vec3(0.05, 0.045, 0.06), vec3(0.16, 0.10, 0.08), smoothstep(0.4, 0.8, ncr));
            albedo = mix(albedo, obs, eye_hot * 0.85);
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
    // Éclairage diffus MULTI-SOURCE : l'étoile primaire garde le diffus bosselé par
    // le relief (diffb, via light_color) ; les compagnons (indices 1..3) ajoutent un
    // diffus géométrique. En système à une seule étoile, lights_color[1..3] = 0 ->
    // strictement identique à avant.
    vec3 lit = vec3(0.35) + light_color * (0.65 * diffb);
    for (int i = 1; i < 4; i++) {
        vec3 Li = normalize(lights_pos[i] - surf);
        lit += lights_color[i] * (0.65 * max(dot(n, Li), 0.0));
    }
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
    // structurée par le PROFIL ZONAL réel (les belts, moins nuageuses, rayonnent
    // plus) -> la nuit est le négatif du jour, mêmes latitudes de bandes.
    if (thermique > 0.0) {
        float nuit = 1.0 - smoothstep(-0.1, 0.35, diff);
        float gb = 0.55 + 0.55 * (1.0 - texture2D(zonal, vec2(dot(d, k) * 0.5 + 0.5, 0.5)).g);
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
