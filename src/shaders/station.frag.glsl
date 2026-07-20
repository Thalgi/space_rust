#version 100
#extension GL_OES_standard_derivatives : enable
precision highp float;
varying vec2 v_uv;
varying vec4 v_col;
varying vec3 v_wpos;
uniform sampler2D Texture;
uniform vec3 lum_dir;  // direction VERS la lumière (normalisée côté CPU)
uniform vec3 cam_pos;  // position caméra (monde) pour le spéculaire

void main() {
    vec4 base = v_col * texture2D(Texture, v_uv);

    // Normale de facette par dérivées écran : éclaire toutes les primitives
    // (cylindres, cubes, mailles) sans données de normales dans les sommets.
    vec3 dx = dFdx(v_wpos);
    vec3 dy = dFdy(v_wpos);
    vec3 n = cross(dx, dy);
    float ln = length(n);

    // Lignes/fils : dérivées dégénérées -> pas d'éclairage directionnel.
    if (ln < 1e-6) {
        gl_FragColor = base;
        return;
    }
    n /= ln;
    // Normale orientée vers la caméra (les panneaux sont double-face).
    vec3 vers_cam = normalize(cam_pos - v_wpos);
    if (dot(n, vers_cam) < 0.0) { n = -n; }

    // Clé + contre-jour froid + ambiance : plage ~[0.25, 1.15].
    float diff = max(dot(n, lum_dir), 0.0);
    float fill = max(dot(n, -lum_dir), 0.0) * 0.18;
    float shade = 0.30 + 0.78 * diff + fill;

    // Spéculaire discret (métal/alu) : Blinn-Phong large.
    vec3 h = normalize(lum_dir + vers_cam);
    float spec = pow(max(dot(n, h), 0.0), 24.0) * 0.35;

    vec3 rgb = base.rgb * shade + vec3(spec);
    gl_FragColor = vec4(rgb, base.a);
}
