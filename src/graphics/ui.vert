#version 100
precision lowp float;

attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec4 in_fg;
attribute vec4 in_bg;

varying vec2 uv;
varying vec4 fg;
varying vec4 bg;

void main() {
    gl_Position = vec4(in_pos, 1.0);
    uv = in_uv;
    fg = in_fg;
    bg = in_bg;
}
