#version 100
precision lowp float;

varying vec2 uv;
uniform vec4 color;
uniform vec2 size;
uniform vec2 grid_size;

void main() {
    float dx = 0.5 / grid_size.x;
    float dy = 0.5 / grid_size.y;
    float xm = fract(uv.x * size.x / grid_size.x);
    float ym = fract(uv.y * size.y / grid_size.y);
    if (xm < dx || xm >= 1. - dx || ym < dy || ym > 1. - dy) {
        gl_FragColor = color;
    } else {
        gl_FragColor = vec4(0.);
    }
}
