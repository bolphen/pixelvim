#version 100
precision lowp float;

varying vec2 uv;
varying vec4 fg;
varying vec4 bg;
uniform sampler2D texture;

void main() {
    vec4 color = texture2D(texture, uv);
    if (color.g == 0.) {
        gl_FragColor = bg;
    } else {
        gl_FragColor = fg;
    }
}
