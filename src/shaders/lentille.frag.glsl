#version 100
// Lentille gravitationnelle **locale** d'un trou noir (lot 8.2).
//
// Passe à part, et dessinée **avant** tout le reste de l'astre — avant même la
// moitié arrière du disque d'accrétion. La raison est le blending : elle rend un
// ciel **opaque**, qui doit *remplacer* le fond stellaire déjà dessiné et non s'y
// ajouter. Posée plus tard, elle effacerait l'arc du pli qui enjambe l'horizon.
//
// ⚠️ **Ce qu'elle dévie est le fond stellaire, et lui seul.** Une cubemap encode
// des directions à l'infini : le champ d'étoiles est à 3000 unités, donc
// assimilable, mais une planète à 40 unités a de la parallaxe — sa direction
// apparente dépend d'où est l'œil — et aucun échantillonnage par direction seule
// ne peut la représenter. Une planète qui passe derrière l'horizon n'est donc ni
// déviée ni reprise dans l'anneau. Parti pris assumé, décidé avec l'utilisateur
// (`docs/suivi/astres_v2.md`, lot 8) ; la seule alternative correcte serait du
// raymarching géodésique par pixel.
precision highp float;

varying vec2 v_q; // coordonnée du quad : |v_q| = 1 au bord de l'ombre

uniform vec3 cam_right;
uniform vec3 cam_up;
uniform vec3 to_cam;
uniform float couronne;    // étendue du quad, en rayons de l'ombre
uniform float angle_ombre; // rayon angulaire de l'ombre, en radians
uniform float einstein;    // rayon d'Einstein, en rayons de l'ombre
uniform samplerCube fond;  // le ciel de CETTE vue (`fond::Fond::cubemap`)

void main() {
    float rr = length(v_q);
    if (rr > couronne) discard;

    // L'ombre : noir opaque. C'est elle qui occulte le fond stellaire déjà
    // dessiné — sans quoi les étoiles se verraient à travers l'horizon.
    if (rr <= 1.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }

    vec2 u = v_q / max(rr, 1e-4);
    float theta = rr * angle_ombre; // angle depuis le centre de l'ombre

    // **Lentille mince.** Un objet dont la position non déviée est `beta`
    // apparaît en `theta`, avec `beta = theta - theta_E^2 / theta`. On rend donc
    // l'inverse : pour chaque pixel à `theta`, on va chercher le ciel à `beta`.
    //
    // Un **anneau d'Einstein** en sort tout seul : à `theta = theta_E`, `beta`
    // vaut 0, donc tout ce pourtour montre le point exactement derrière le trou
    // noir. Et `beta` négatif — plus près que l'anneau — donne l'**image
    // secondaire**, de l'autre côté du centre : c'est physiquement juste, et
    // gratuit ici puisque `sin` change de signe.
    //
    // ⚠️ **La déviation s'éteint au bord du quad.** Le quad est un objet de
    // rendu, pas un objet du monde : sans extinction, le ciel dévié s'arrêterait
    // net sur une frontière invisible, et on verrait un disque de ciel décalé
    // posé sur le ciel réel. Avec elle, `beta` rejoint `theta` au bord, donc le
    // ciel échantillonné y **est** le ciel réel, et le raccord ne se voit pas.
    // C'est aussi ce qui rend la convention de cubemap vérifiable à l'œil : si
    // elle était fautive, le bord du quad trancherait.
    float att = 1.0 - smoothstep(couronne * 0.5, couronne * 0.95, rr);
    float e = einstein * angle_ombre;
    float beta = theta - (e * e / max(theta, 1e-6)) * att;

    // La direction de visée, tournée de `beta` depuis l'axe du trou noir dans le
    // plan que porte l'azimut du pixel. `-to_cam` regarde vers l'astre.
    vec3 lat = u.x * cam_right + u.y * cam_up;
    vec3 dir = normalize(-to_cam * cos(beta) + lat * sin(beta));

    gl_FragColor = vec4(textureCube(fond, dir).rgb, 1.0);
}
