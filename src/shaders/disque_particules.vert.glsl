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
    vec4 clip = Projection * Model * vec4(pos, 1.0);
    float px_par_unite = Projection[1][1] * viewport_h * 0.5 / max(clip.w, 0.0001);
    float taille_px = taille * px_par_unite;
    if (taille_px < px_min && taille_px > 0.0) {
        taille *= px_min / taille_px;
        float fade = clamp(taille_px / px_min, 0.3, 1.0);
        vcol.rgb *= fade;
    }

    // Lumière en base billboard (right, up, vers caméra) : le fragment shader
    // éclaire la pseudo-sphère avec, sans repasser par le monde.
    vec3 to_light = normalize(light_pos - pos);
    vec3 to_cam = normalize(cam_pos - pos);
    vlum = vec3(dot(to_light, cam_right), dot(to_light, cam_up), dot(to_light, to_cam));
    vforme = vec3(graine, irr, max(taille_px, px_min));

    // Billboard : quad plein (marge 1.15 pour les lobes du patatoïde) ; la
    // silhouette est sculptée par le fragment shader, plus par les coins.
    vec2 base = vec2(
        (coin > 0.5 && coin < 2.5) ? 1.0 : -1.0,
        (coin > 1.5) ? 1.0 : -1.0
    );
    vp = base;
    pos += cam_right * (base.x * taille * 1.15) + cam_up * (base.y * taille * 1.15);

    gl_Position = Projection * Model * vec4(pos, 1.0);
}
