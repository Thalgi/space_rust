#version 100
precision highp float;
attribute vec3 position;
attribute vec2 texcoord;
varying vec2 v_q;
uniform mat4 Model;
uniform mat4 Projection;
uniform float disc; // demi-etendue du disque impostor (1.05 planete, couronne soleil)
void main() {
    v_q = (texcoord * 2.0 - 1.0) * disc;
    gl_Position = Projection * Model * vec4(position, 1.0);
}
