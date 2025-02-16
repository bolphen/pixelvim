#version 100
precision lowp float;

varying vec2 uv;
uniform vec4 color1;
uniform vec4 color2;
uniform vec2 size;
uniform vec2 offset;

void main() {
    float x = mod(floor((uv.x - offset.x) * size.x / 8.), 2.);
    float y = mod(floor((uv.y - offset.y) * size.y / 8.), 2.);
    float t = x * y + (1. - x) * (1. - y);
    gl_FragColor = t * color1 + (1. - t) * color2;
}
