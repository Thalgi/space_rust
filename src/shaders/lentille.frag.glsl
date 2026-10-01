#version 100
// Lentille gravitationnelle d'un trou noir (lot 8.2, refondu le 2026-10-01).
//
// Passe à part, dessinée **avant** tout le reste de l'astre — avant même la
// moitié arrière du disque d'accrétion. La raison est le blending : elle rend un
// ciel **opaque**, qui doit *remplacer* le fond stellaire déjà dessiné et non s'y
// ajouter. Posée plus tard, elle effacerait l'arc du pli qui enjambe l'horizon.
//
// ⚠️ **Ce qu'elle dévie est le fond stellaire, et lui seul.** Une cubemap encode
// des directions à l'infini : le champ d'étoiles est à 3000 unités, donc
// assimilable, mais une planète à 40 unités a de la parallaxe — sa direction
// apparente dépend d'où est l'œil — et aucun échantillonnage par direction seule
// ne peut la représenter. Parti pris assumé, décidé avec l'utilisateur.
precision highp float;

varying vec2 v_q; // coordonnée du quad : |v_q| = 1 au bord de l'ombre

uniform vec3 cam_right;
uniform vec3 cam_up;
uniform vec3 to_cam;
uniform float couronne;    // étendue du quad, en rayons de l'ombre
uniform float angle_ombre; // rayon angulaire de l'ombre, en radians
uniform float dev_faible;  // 2 r_s, numérateur du champ faible
uniform float dev_forte;   // -pi + ln[216(7-4*sqrt(3))]
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

    // ⚠️ **Le paramètre d'impact EST la coordonnée du quad.** `b/b_c = θ/θ_s`,
    // et `rr` vaut déjà `θ/θ_s` puisque `|v_q| = 1` tombe sur le bord de
    // l'ombre, qui est `b_c`. Aucune conversion, donc aucune occasion de se
    // tromper d'unité — celle qui a donné l'impostor carré.
    float b = rr;

    // **Déviation à deux régimes** (`astres_v2_lentille.md` §1). Le champ
    // faible `2 r_s / b` est exact au loin ; la déviation forte de Darwin
    // **diverge** au bord de l'ombre, et c'est elle qui fait l'image — l'anneau
    // d'Einstein et l'empilement des images d'ordre supérieur en sortent seuls.
    // Les deux se croisent à b = 1,764 ; on prend le plus grand, ce qui reste
    // continu puisqu'ils se coupent.
    //
    // ⚠️ Les deux constantes sont **dupliquées** depuis `soleil/materiau.rs`,
    // qui les dérive de `b_c = 3√3/2 r_s`. Un test de littéraux compare les deux
    // textes, comme pour `PORTEE` du champ magnétique.
    float alpha = max(dev_faible / b, dev_forte - log(1.0 - 1.0 / b));

    // ⚠️ **Atténuation sur le tiers extérieur seulement.** Le quad est un objet
    // de rendu, pas un objet du monde : sans elle, le ciel dévié s'arrêterait net
    // sur une frontière invisible. Le faux gradient qu'elle introduit vaut
    // environ 0,16 px par pixel à 12 rayons — sub-pixel, donc invisible, et
    // c'est un test qui le vérifie, pas une impression.
    alpha *= 1.0 - smoothstep(couronne * 0.66, couronne, rr);

    // **La direction du ciel est à `θ + α` du centre de l'ombre.** Au loin
    // `α → 0` et l'on retrouve la direction de visée ; au bord de l'ombre `α`
    // dépasse π et le ciel s'y **enroule**, ce qui produit l'anneau puis les
    // images d'ordre supérieur. C'est tout l'effet, et il ne coûte qu'un cosinus.
    float phi = rr * angle_ombre + alpha;

    vec2 u = v_q / max(rr, 1e-4);
    vec3 lat = u.x * cam_right + u.y * cam_up;
    vec3 dir = normalize(-to_cam * cos(phi) + lat * sin(phi));

    gl_FragColor = vec4(textureCube(fond, dir).rgb, 1.0);
}
