#version 100
precision lowp float;

attribute vec3 in_pos;
attribute vec2 in_uv;

varying vec2 uv;

void main() {
    gl_Position = vec4(in_pos, 1.0);
    uv = in_uv;
}
