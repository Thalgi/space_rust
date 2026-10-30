#version 100
// Les boules des nuages (`disque::nuages_gpu`) : le halo teinté du plasma
// (`soleil_plasma.frag.glsl`), prémultiplié, pour un blending additif.
precision mediump float;
varying vec2 uv;
varying vec4 vcol;
uniform sampler2D Texture;
void main() {
    float a = texture2D(Texture, uv).a * vcol.a;
    gl_FragColor = vec4(vcol.rgb * a, a);
}
