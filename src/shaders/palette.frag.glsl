#version 100
precision highp float;

// Quantification vers une palette fixe, en CIELAB, avec tramage ordonne.
//
// Applique au BLIT de la cible basse resolution, pas a la scene : la couche
// pixelisee est deja rendue, on ne fait ici que remplacer chaque couleur.
//
// `palette_lab` arrive DEJA converti depuis `src/palette.rs` : le shader ne
// convertit que le pixel courant, une fois au lieu de N+1.
//
// Trois etapes, dans cet ordre :
//   1. ecretage des hautes lumieres  (sinon le halo du speculaire devient blanc)
//   2. tramage ordonne de Bayer      (sinon les bandes basculent d'un bloc)
//   3. recherche du plus proche      (la quantification elle-meme)

varying lowp vec2 uv;
varying lowp vec4 color;

uniform sampler2D Texture;
// Matrice de Bayer 8x8. Une TEXTURE et non un tableau d'uniformes : GLSL ES
// 1.00 n'autorise l'indexation d'un tableau d'uniformes que par une constante,
// or l'indice de tramage se calcule depuis la position du pixel.
uniform sampler2D tramage;

#define MAX 256
uniform vec3 palette_lab[MAX];
uniform vec3 palette_rgb[MAX];
// Longueur reelle : les palettes ne font pas toutes MAX couleurs.
uniform float nb_couleurs;

// Taille de la cible basse resolution, pour indexer le tramage sur la grille
// des GROS pixels et non sur celle de l'ecran.
uniform vec2 taille_cible;
uniform float cote_trame;
uniform float force_trame;

uniform float ecretage_seuil;
uniform float ecretage_force;

// Combien on ravive la chroma AVANT de chercher. Sans ca, une planete voilee
// par son atmosphere n'a qu'une chroma moderee et les entrees NEUTRES de la
// palette, voisines en CIELAB, l'emportent : les oceans sortaient gris-violet.
uniform float saturation;
// Saturation des hautes lumieres, toujours < 1 : un reflet est achromatique.
// Sans ce garde-fou, le halo tombe sur les entrees cyan de la palette et forme
// un anneau colore autour du point chaud.
uniform float sat_hautes;
uniform vec2 sat_rolloff;

vec3 rgb2xyz(vec3 c) {
    // sRGB -> lineaire. Sans cette etape, les tons sombres seraient traites
    // comme bien plus clairs qu'ils ne sont.
    c = vec3(
        (c.r > 0.04045) ? pow((c.r + 0.055) / 1.055, 2.4) : c.r / 12.92,
        (c.g > 0.04045) ? pow((c.g + 0.055) / 1.055, 2.4) : c.g / 12.92,
        (c.b > 0.04045) ? pow((c.b + 0.055) / 1.055, 2.4) : c.b / 12.92
    );
    return vec3(
        c.r * 0.4124 + c.g * 0.3576 + c.b * 0.1805,
        c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722,
        c.r * 0.0193 + c.g * 0.1192 + c.b * 0.9505
    );
}

float f_lab(float t) {
    return (t > 0.008856) ? pow(t, 1.0 / 3.0) : (7.787 * t) + (16.0 / 116.0);
}

vec3 rgb2lab(vec3 c) {
    vec3 x = rgb2xyz(c);
    vec3 v = vec3(f_lab(x.x / 0.95047), f_lab(x.y / 1.00000), f_lab(x.z / 1.08883));
    return vec3((116.0 * v.y) - 16.0, 500.0 * (v.x - v.y), 200.0 * (v.y - v.z));
}

// Ravive la chroma A LUMINANCE CONSTANTE : on ecarte du gris de meme clarte,
// donc l'ombrage n'est pas touche. Le gain retombe a `sat_hautes` en haut de
// l'echelle, ou les reflets doivent rester neutres.
//
// Ne borne pas : l'ecretage qui suit doit voir les depassements au-dessus de 1
// pour distinguer le coeur d'un reflet de son halo.
vec3 saturer(vec3 c) {
    float y = dot(c, vec3(0.2126, 0.7152, 0.0722));
    float t = clamp((y - sat_rolloff.x) / (sat_rolloff.y - sat_rolloff.x), 0.0, 1.0);
    t = t * t * (3.0 - 2.0 * t); // smoothstep
    float gain = saturation + (sat_hautes - saturation) * t;
    return vec3(y) + (c - vec3(y)) * gain;
}

// Comprime ce qui depasse le seuil, en gardant la teinte (on divise les trois
// composantes par le meme facteur). Le speculaire des oceans est ADDITIF et
// monte au-dessus de 1.0 : sans ca, tout son halo s'ecrase sur le blanc pur.
vec3 ecreter(vec3 c) {
    float m = max(max(c.r, c.g), c.b);
    if (m <= ecretage_seuil) {
        return c;
    }
    float vise = ecretage_seuil + (m - ecretage_seuil) * ecretage_force;
    return c * (vise / m);
}

void main() {
    vec4 src = texture2D(Texture, uv) * color;

    // La cible est nettoyee en TRANSPARENT et composee par-dessus le decor net :
    // l'essentiel de l'ecran est vide. On l'ecarte avant la boucle, ce qui
    // epargne la recherche sur la plus grande partie des pixels.
    if (src.a < 0.004) {
        discard;
    }

    // L'ordre compte : saturer sur la couleur telle que la scene l'a produite,
    // ecreter ensuite (qui a besoin des depassements), tramer en dernier.
    vec3 c = ecreter(saturer(src.rgb));

    // --- Tramage ordonne ---
    //
    // L'offset se prend sur la grille de la CIBLE : indexe sur les pixels de
    // l'ecran, le motif serait plus fin que les gros pixels et invisible.
    if (force_trame > 0.0) {
        vec2 pc = floor(uv * taille_cible);
        vec2 duv = (mod(pc, cote_trame) + 0.5) / cote_trame;
        float seuil = texture2D(tramage, duv).r - 0.5;
        c += seuil * force_trame;
    }

    vec3 lab = rgb2lab(clamp(c, 0.0, 1.0));

    // ATTENTION : on retient la COULEUR, pas l'indice.
    //
    // `palette_rgb[meilleurIndice]` serait la forme naturelle, mais GLSL ES 1.00
    // n'autorise l'indexation d'un tableau d'uniformes que par une
    // « constant-index-expression » — un indice de boucle en est une, une
    // variable calculee non.
    vec3 meilleure = palette_rgb[0];
    float dmin = 1.0e9;

    for (int i = 0; i < MAX; i++) {
        // Les palettes courtes s'arretent ici ; les cases au-dela sont un
        // remplissage.
        if (float(i) >= nb_couleurs) {
            break;
        }
        vec3 d = lab - palette_lab[i];
        // Distance AU CARRE : la racine est monotone, elle ne change pas le
        // gagnant. `palette.rs` compare de la meme facon.
        float d2 = dot(d, d);
        if (d2 < dmin) {
            dmin = d2;
            meilleure = palette_rgb[i];
        }
    }

    // L'alpha est conserve tel quel : c'est lui qui laisse voir le fond
    // stellaire net a travers le vide de la cible.
    gl_FragColor = vec4(meilleure, src.a);
}
