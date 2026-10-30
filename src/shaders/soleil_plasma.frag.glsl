#version 100
precision highp float;
varying vec2 uv;
varying vec4 vcol;
uniform sampler2D Texture;
void main() {
    // UV décalées de 2 : la composition d'une cible de nuages réduite
    // (`disque::basse_res`), déjà prémultipliée — lue telle quelle.
    if (uv.x > 1.5) {
        gl_FragColor = texture2D(Texture, uv - 2.0);
        return;
    }
    float a = texture2D(Texture, uv).a * vcol.a;
    gl_FragColor = vec4(vcol.rgb * a, a); // prémultiplié -> additif sans noir
}
