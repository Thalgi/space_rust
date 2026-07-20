#version 100
precision highp float;
varying vec2 uv;
varying vec4 vcol;
uniform sampler2D Texture;
void main() {
    float a = texture2D(Texture, uv).a * vcol.a;
    gl_FragColor = vec4(vcol.rgb * a, a); // prémultiplié -> additif sans noir
}
