// Panache d'antimatière : ruban face-caméra, rendu en **additif prémultiplié**.
//
// Le même principe que les jets bipolaires de pulsar (`soleil.frag.glsl`), et
// pour la même raison : un jet de plasma n'a pas de silhouette. Un cône de
// géométrie pleine en a une — une arête nette, une face éclairée, un bord franc
// sur le fond — et c'est exactement ce qui le faisait lire comme un tube de
// plastique plutôt que comme du gaz incandescent.
//
// Ici il n'y a plus de surface : la matière est une **densité** qui s'additionne
// au fond. Là où elle est faible, les étoiles passent au travers.
#version 100
precision highp float;
varying vec2 uv;    // x : 0..1 en travers (0,5 = axe) — y : 0 au col, 1 au bout
varying vec4 vcol;  // teinte du plasma à cette hauteur (rampe de température)
uniform float time;
uniform float intensite;

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
    for (int k = 0; k < 4; k++) {
        v += a * vnoise(p);
        p *= 2.0;
        a *= 0.5;
    }
    return v;
}

void main() {
    float x = abs(uv.x * 2.0 - 1.0); // 0 sur l'axe, 1 au bord du ruban
    float t = uv.y;

    // Profil en travers : cœur dense qui s'éteint doucement vers le bord. Sans
    // ce dégradé le ruban aurait une arête, et on retomberait sur le tube.
    float coeur = pow(max(0.0, 1.0 - x * x), 2.2);

    // Écoulement : la turbulence **file vers le bout** (le terme en -time sur
    // l'axe du jet). C'est ce qui distingue un jet d'un fuseau peint — la
    // matière doit visiblement partir.
    float flot = fbm(vec3(uv.x * 5.0, t * 7.0 - time * 2.6, time * 0.25));
    float grain = 0.55 + 0.85 * flot;

    // Extinction le long du jet : le plasma se détend et se refroidit. Le col
    // reste toujours dense, sans quoi le jet paraîtrait détaché de la tuyère.
    float long_ = (1.0 - smoothstep(0.35, 1.0, t));
    float col = 1.0 - smoothstep(0.0, 0.06, t); // surbrillance de sortie

    float a = coeur * grain * long_ * intensite;
    a = clamp(a + col * coeur * 0.6 * intensite, 0.0, 1.0);

    // Prémultiplié : la composante noire n'ajoute rien en additif, donc le bord
    // du ruban disparaît au lieu de se découper sur le fond.
    gl_FragColor = vec4(vcol.rgb * a, a);
}
