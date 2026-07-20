#version 100
precision highp float;
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying vec2 v_uv;
varying vec4 v_col;
varying vec3 v_wpos;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    vec4 w = Model * vec4(position, 1.0);
    v_wpos = w.xyz;
    v_uv = texcoord;
    v_col = color0 / 255.0;
    gl_Position = Projection * w;
}
