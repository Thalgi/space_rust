#version 100
precision highp float;
varying vec4 vcol;
void main() {
    gl_FragColor = vcol;
}
