#version 100
// Particules de champ de débris (voir CONCEPTION_CEINTURES.md §2.2).
// Le sommet ne porte PAS une position mais des éléments orbitaux :
//   position = (phi, incl, r)  — plan orbital + rayon
//   texcoord = (coin + graine, taille) — coin 0..3 en partie entière
//   color0   = teinte RGB + IRRÉGULARITÉ dans l'alpha (pipeline opaque)
// Le shader reconstruit l'orbite képlérienne et le billboard face caméra ;
// le fragment shader sculpte ensuite un « patatoïde » éclairé (micro-impostor).
precision highp float;

attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying vec4 vcol;
varying float vside;  // côté caméra (+) / opposé (-) par rapport au centre
varying vec2 vp;      // coordonnées locales du quad (-1..1)
varying vec3 vlum;    // direction de la lumière en base billboard
varying vec3 vforme;  // (graine, irrégularité, taille écran px)

uniform mat4 Model;
uniform mat4 Projection;
uniform float time;
uniform float gm;       // G * masse du parent -> omega = sqrt(gm / r^3)
uniform float ecc_max;  // excentricité max (0 = orbites circulaires)
uniform vec3 centre;    // position monde du parent (étoile, planète)
uniform vec3 axe_u;     // base du plan du disque (axe_n = normale)
uniform vec3 axe_n;
uniform vec3 axe_v;
uniform vec3 cam_right;
uniform vec3 cam_up;
uniform float viewport_h; // hauteur (px) du viewport de rendu
uniform float px_min;     // demi-taille écran minimale d'une particule
uniform vec3 light_pos;   // étoile primaire (éclairage + ombre)
uniform vec3 cam_pos;     // position caméra
uniform float ombre_rayon; // rayon du corps central (0 = pas d'ombre)

// Seuil de SILHOUETTE, en DEMI-taille écran (px). Au-delà, le caillou reçoit un
// contour et un éclairage de pseudo-sphère ; en deçà, c'est un point carré.
//
// ⚠️ **C'est une demi-taille.** `taille`, `taille_px`, `px_min` et ce seuil sont
// tous des DEMI-tailles : un caillou à 1.25 fait **2,5 px de côté** à l'écran.
// L'ancien seuil valait 2.5 — soit des carrés plats de 5 px de côté, encore
// grossis de 15 % par la marge des lobes, jusqu'à ~5,75 px. C'est ce qui donnait
// des « gros carrés » dès que la caméra approchait ou passait au-dessus du plan,
// là où beaucoup de cailloux tombent d'un coup dans la même tranche de taille.
#define SEUIL_FORME 1.25

float hash(float x) {
    return fract(sin(x * 12.9898) * 43758.5453);
}

void main() {
    // L'alpha du sommet transporte l'IRRÉGULARITÉ de forme (pipeline opaque :
    // le canal est libre). Gros fragments anguleux, petits cailloux ronds.
    float irr = color0.a / 255.0;
    vcol = vec4(color0.rgb / 255.0, 1.0);

    float phi    = position.x;
    float incl   = position.y;
    float r0     = position.z;
    float coin   = floor(texcoord.x);
    float graine = fract(texcoord.x);
    float taille = texcoord.y;

    // CRUCIAL : quantifier la graine pour qu'elle soit BIT-IDENTIQUE entre les
    // 4 coins du quad (sinon hash() amplifie l'écart f32 -> traînées).
    graine = floor(graine * 1024.0 + 0.5) * 0.0009765625;

    // Repère orbital (même construction que l'ex-Ceinture CPU).
    vec3 a1 = vec3(cos(phi), 0.0, sin(phi));
    vec3 a2 = vec3(-sin(phi), 0.0, cos(phi));
    vec3 q  = normalize(a2 * cos(incl) + vec3(0.0, 1.0, 0.0) * sin(incl));

    // Orbite : Kepler analytique. La graine fixe la phase de départ,
    // l'excentricité et le périastre — déterministe, aucun état CPU.
    float omega = sqrt(gm / (r0 * r0 * r0));
    float theta = graine * 6.2831853 + omega * time;
    float e     = hash(graine * 97.7 + phi) * ecc_max;
    float peri  = hash(graine * 61.1 + phi) * 6.2831853;
    float r     = r0 * (1.0 - e * e) / (1.0 + e * cos(theta - peri));

    // Position locale (plan Y) puis passage dans le plan du disque (uniforms).
    vec3 loc = a1 * (r * cos(theta)) + q * (r * sin(theta));
    vec3 pos = centre + axe_u * loc.x + axe_n * loc.y + axe_v * loc.z;

    // Moitié avant/arrière par rapport au corps central : rendu peintre
    // (arrière -> corps -> avant), fiable même sans depth buffer (mode pixel).
    //
    // Une valeur PAR PARTICULE, et ça ne peut pas être autrement : le plan de
    // coupe a pour normale la direction de vue, or le billboard est justement
    // perpendiculaire à cette direction. Le quad est donc **parallèle au plan de
    // coupe** — il ne le traverse jamais, et l'y projeter ne changerait rien.
    // (Essayé : recalculer `vside` après le décalage billboard est un no-op,
    // `cam_right` et `cam_up` étant orthogonaux à la normale du plan.)
    vside = dot(pos - centre, normalize(cam_pos - centre));

    // Ombre du corps central (anneau de débris planétaire) : cylindre projeté.
    if (ombre_rayon > 0.001) {
        vec3 ldir = normalize(centre - light_pos); // propagation au centre
        vec3 d = pos - centre;
        float along = dot(d, ldir);
        if (along > 0.0) {
            float perp = length(d - ldir * along);
            float ombre = smoothstep(ombre_rayon * 0.8, ombre_rayon * 1.08, perp);
            vcol.rgb *= mix(0.16, 1.0, ombre);
        }
    }

    // LOD sub-pixel (CONCEPTION §4.2) : une particule plus petite qu'un pixel
    // disparaît au rasterizer. On borne sa taille ÉCRAN à px_min en compensant
    // par un assombrissement (pipeline opaque : on fond vers le noir).
    //
    // ⚠️ **On MESURE la taille projetée, on ne la déduit pas de la matrice.**
    //
    // L'uniform `Projection` de macroquad n'est PAS la seule projection : c'est
    // `perspective × look_at`, la matrice vue-projection combinée
    // (`Camera3D::matrix`). Lire `Projection[1][1]` comme un `1/tan(fovy/2)` ne
    // vaut donc que pour UNE orientation de caméra. En visant le long de Y, la
    // vue envoie le Y du monde sur la profondeur, ce coefficient tend vers zéro,
    // `taille_px` avec lui — et le facteur de correction `px_min / taille_px`
    // part à l'infini. Les petits débris enflaient en gros carrés, et seulement
    // près de cet axe.
    //
    // On mesure donc l'ÉCHELLE, pas une différence de positions.
    //
    // ⚠️ **Ne jamais soustraire deux points projetés.** La version précédente
    // projetait le centre et un point décalé de `taille`, puis soustrayait leurs
    // NDC. Sur les débris LOINTAINS (nuage de Oort à 20 000 UA, cailloux de
    // 0,012 unité) ces deux NDC coïncident sur les ~7 chiffres significatifs
    // d'un f32 : la soustraction ne rend plus que du bruit d'arrondi. Diviser
    // `px_min` par ce bruit donnait un facteur d'agrandissement ALÉATOIRE,
    // retiré à chaque frame — d'où des astéroïdes lointains qui enflaient
    // brusquement et sans cesse. Annulation catastrophique, cas d'école.
    //
    // Le remède : projeter le vecteur `cam_up` **d'une unité monde** (une
    // direction, `w = 0`), et appliquer la règle du quotient en UNE expression.
    // `k` ne dépend alors plus de `taille` : ses termes restent d'ordre 1 quelle
    // que soit la distance, et rien ne se soustrait de presque égal.
    vec4 clip_c = Projection * Model * vec4(pos, 1.0);
    vec4 dclip = Projection * Model * vec4(cam_up, 0.0);
    float w2 = clip_c.w * clip_c.w;
    // d(y/w) pour un déplacement d'une unité selon cam_up, en pixels.
    float k = abs(dclip.y * clip_c.w - clip_c.y * dclip.w) / max(w2, 1e-12) * viewport_h * 0.5;
    float taille_px = taille * k;
    if (taille_px < px_min && k > 1e-9) {
        // `px_min / k` directement : pas de rapport entre deux petits nombres.
        taille = px_min / k;
        float fade = clamp(taille_px / px_min, 0.3, 1.0);
        vcol.rgb *= fade;
    }

    // Lumière en base billboard (right, up, vers caméra) : le fragment shader
    // éclaire la pseudo-sphère avec, sans repasser par le monde.
    vec3 to_light = normalize(light_pos - pos);
    vec3 to_cam = normalize(cam_pos - pos);
    vlum = vec3(dot(to_light, cam_right), dot(to_light, cam_up), dot(to_light, to_cam));

    // Sculpter, ou pas. Au-dessus du seuil le fragment shader taille un
    // patatoïde ; en dessous il ne reste pas assez de fragments pour qu'un
    // contour veuille dire quoi que ce soit, et le découper ferait scintiller —
    // la particule est alors un simple point carré.
    //
    // Le SIGNE de vforme.z porte la décision jusqu'au fragment shader : le seuil
    // n'est écrit qu'ICI, et les deux étages ne peuvent pas diverger.
    float px = max(taille_px, px_min);
    float sculpte = px > SEUIL_FORME ? 1.0 : -1.0;
    vforme = vec3(graine, irr, px * sculpte);

    // Billboard. La marge 1.15 loge les LOBES du patatoïde : un point carré n'en
    // a pas, la lui appliquer ne faisait que le grossir de 15 % pour rien.
    float marge = sculpte > 0.0 ? 1.15 : 1.0;
    vec2 base = vec2(
        (coin > 0.5 && coin < 2.5) ? 1.0 : -1.0,
        (coin > 1.5) ? 1.0 : -1.0
    );
    vp = base;
    pos += cam_right * (base.x * taille * marge) + cam_up * (base.y * taille * marge);

    gl_Position = Projection * Model * vec4(pos, 1.0);
}
