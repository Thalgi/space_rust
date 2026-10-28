#version 100
precision highp float;

// Quantification vers une palette fixe, en CIELAB, avec tramage ordonne.
//
// Applique au BLIT de la cible basse resolution, pas a la scene : la couche
// pixelisee est deja rendue, on ne fait ici que remplacer chaque couleur.
//
// La recherche du plus proche est PRECALCULEE (`palette::table`, Opti-v2) :
// une texture de 64 x 64 x 64 couleurs, rangee en atlas de 8 x 8 tranches de
// bleu. Le shader n'y fait plus qu'une lecture, au lieu de 182 distances Lab.
//
// Etapes, dans cet ordre :
//   1. saturation, puis ecretage des hautes lumieres (sinon le halo du
//      speculaire devient blanc), puis gamma
//   2. lecture de la table (la quantification elle-meme)

varying lowp vec2 uv;
varying lowp vec4 color;

uniform sampler2D Texture;
// La couleur de palette retenue pour chaque case de la grille (`palette::table`).
uniform sampler2D table;
// `palette::TABLE_NIVEAUX` et `TABLE_TRANCHES` : changer l'un, c'est changer l'autre.
#define NIVEAUX 64.0
#define TRANCHES 8.0

// Taille de la cible basse resolution, pour indexer le tramage sur la grille
// des GROS pixels et non sur celle de l'ecran.
uniform vec2 taille_cible;

// Correction gamma appliquee AVANT la recherche : elle change la clarte, donc
// les entrees de palette retenues. 1 = neutre.
uniform float gamma;

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

    // Gamma AVANT la quantification : corriger apres ne ferait que deplacer des
    // couleurs deja choisies hors de la palette.
    c = clamp(c, 0.0, 1.0);
    if (abs(gamma - 1.0) > 0.001) {
        c = pow(c, vec3(1.0 / gamma));
    }

    // Adressage : miroir exact de `palette::texel`.
    vec3 q = floor(c * (NIVEAUX - 1.0) + 0.5);
    vec2 t = vec2(mod(q.b, TRANCHES), floor(q.b / TRANCHES)) * NIVEAUX + q.rg + 0.5;
    vec3 meilleure = texture2D(table, t / (NIVEAUX * TRANCHES)).rgb;

    // L'alpha est conserve tel quel : c'est lui qui laisse voir le fond
    // stellaire net a travers le vide de la cible.
    gl_FragColor = vec4(meilleure, src.a);
}
