#version 100
precision highp float;

// COQUILLE ATMOSPHÉRIQUE
//
// Passe séparée, dessinée juste après le corps (planete.frag.glsl) sur un quad
// impostor identique. Le corps s'arrête net à la silhouette (`discard` au-delà
// de rr = 1) : c'est ICI que se joue tout ce qui la dépasse — le halo qui
// déborde du disque, le liseré du limbe, le crépuscule qui mord sur la nuit.
//
// MODÈLE. Une coquille sphérique creuse entre le sol (rayon 1) et le sommet de
// l'atmosphère (rayon `disc`), en rayons planétaires. Pour un rayon visuel qui
// frappe l'image à la distance `b` du centre, on mesure la longueur de corde
// parcourue DANS la coquille : c'est la quantité d'air traversée, donc la
// lumière diffusée vers l'œil.
//
//   b < 1  le sol coupe le rayon -> seule la moitié AVANT de la coquille
//          compte -> corde courte -> voile ténu par-dessus la surface ;
//   b > 1  le rayon passe à côté de la planète et traverse la coquille DEUX
//          fois -> corde double.
//
// Le facteur deux au franchissement de la silhouette n'est pas un artefact de
// calcul : c'est exactement ce qui fait du limbe d'une planète un liseré fin et
// lumineux, et non un dégradé mou.

varying vec2 v_q;

uniform float time; // posé par impostor::uniforms_standard (inutilisé ici)
uniform vec3 cam_right;
uniform vec3 cam_up;
uniform vec3 to_cam;
uniform float disc; // sommet de l'atmosphère, en rayons planétaires

uniform vec3 centre;
uniform float rayon;
uniform vec3 lumiere;
uniform vec3 light_color;
uniform vec3 lights_pos[4];
uniform vec3 lights_color[4];

uniform vec3 atmo;           // TEINTE de l'air, unitaire (ne dit rien de la quantité)
uniform float atmo_densite;  // QUANTITÉ d'air (0 = pas d'atmosphère)
uniform float voile;         // voile opaque (Vénus, Titan) : épaissit et recolore
uniform vec3 voile_couleur;  // teinte du voile

// Hauteur d'échelle de la coquille, en fraction de son épaisseur : la densité
// décroît d'un facteur e sur cette hauteur. 0.35 -> l'essentiel du halo tient
// dans le tiers inférieur, le reste s'éteint doucement vers le vide.
#define ECHELLE 0.35

// Contraste de l'épaisseur optique. Sans lui, la corde brute peint un voile
// uniforme sur TOUT le disque (on regarde à travers de l'air même au centre) et
// délave la planète. L'exposant écrase le centre et garde le limbe.
#define CONTRASTE 1.6

// Opacité de la coquille à corde pleine (Beer-Lambert : 1 - exp(-K·ép)).
#define K 4.0

// Plafond de densité. Il valait 1.6 quand la densité était la longueur d'un
// vecteur couleur et qu'aucun monde ne l'approchait (le plus dense sortait à
// 0,99). Vénus et Titan, une fois leur épaisseur écrite pour de vrai, passent
// au-dessus : sans marge ils saturaient tous deux au même halo et cessaient de
// se distinguer. Aucun monde antérieur n'atteignait 1.6 -> relever ne change
// rien à l'existant.
#define PLAFOND 2.4

/// Éclairement de la coquille vue depuis la direction `na`, pour une source en
/// `sp`. Le terminateur atmosphérique déborde sur la nuit (crépuscule : la haute
/// couche voit encore l'étoile alors que le sol est déjà dans l'ombre).
float jour(vec3 na, vec3 pos, vec3 sp) {
    return smoothstep(-0.35, 0.15, dot(na, normalize(sp - pos)));
}

void main() {
    float b = length(v_q);
    if (b > disc) discard;

    // Corde traversée dans la coquille, et sa valeur maximale (atteinte au
    // limbe b = 1, où le rayon rase le sol de part en part).
    float dehors = sqrt(max(disc * disc - b * b, 0.0));
    float dedans = b < 1.0 ? sqrt(max(1.0 - b * b, 0.0)) : 0.0;
    float corde = b < 1.0 ? dehors - dedans : 2.0 * dehors;
    float corde_max = 2.0 * sqrt(max(disc * disc - 1.0, 0.0));
    if (corde_max < 1e-4) discard; // coquille d'épaisseur nulle : rien à peindre

    // Altitude minimale atteinte par le rayon -> raréfaction de l'air. Sous la
    // silhouette le rayon touche le sol : densité pleine.
    float alt = max(b, 1.0) - 1.0;
    float rarefaction = exp(-alt / (ECHELLE * (disc - 1.0)));
    float epaisseur = pow(corde / corde_max, CONTRASTE) * rarefaction;

    // Point représentatif du segment traversé : son milieu, ramené sur la
    // sphère -> normale de la coquille, qui sert à l'éclairage.
    float zm = 0.5 * (dedans + dehors);
    vec3 na = normalize(v_q.x * cam_right + v_q.y * cam_up + zm * to_cam);
    vec3 pos = centre + na * (rayon * (1.0 + disc) * 0.5);

    // Éclairage multi-source, comme le corps : primaire + jusqu'à 3 compagnons.
    vec3 lit = light_color * jour(na, pos, lumiere);
    for (int i = 1; i < 4; i++) {
        lit += lights_color[i] * jour(na, pos, lights_pos[i]);
    }

    // Diffusion vers l'avant : à contre-jour (l'étoile derrière la planète), la
    // lumière rase la coquille et la traverse presque sans dévier -> l'arc du
    // limbe s'embrase. C'est la signature d'un corps éclairé par derrière.
    vec3 L = normalize(lumiere - pos);
    float avant = 0.5 - 0.5 * dot(L, to_cam);
    float phase = 1.0 + 1.8 * avant * avant * avant;

    // Densité et teinte. Un voile opaque épaissit l'atmosphère et lui impose sa
    // couleur : le halo de Vénus est jaune, pas bleu.
    //
    // La teinte arrive déjà unitaire : la densité vit dans son propre uniform et
    // n'est plus la longueur d'un vecteur couleur. Le plafond est haut (PLAFOND)
    // parce que les mondes les plus épais du catalogue — Vénus, Titan — dépassent
    // franchement la Terre, ce que l'ancien amalgame ne pouvait pas exprimer.
    if (atmo_densite < 1e-4) discard; // corps sans atmosphère
    vec3 teinte = mix(atmo, voile_couleur, voile * 0.6);
    float densite = clamp(atmo_densite * 0.9 + voile * 0.8, 0.0, PLAFOND);

    float alpha = 1.0 - exp(-K * epaisseur * densite);
    gl_FragColor = vec4(teinte * lit * (alpha * phase), alpha);
}
