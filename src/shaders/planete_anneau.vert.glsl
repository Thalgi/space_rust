#version 100
precision highp float;
attribute vec3 position;
attribute vec4 color0;
varying vec4 vcol;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    vcol = color0 / 255.0;
    gl_Position = Projection * Model * vec4(position, 1.0);
}
