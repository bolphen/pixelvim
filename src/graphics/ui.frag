#version 100
precision lowp float;

varying vec2 uv;
uniform sampler2D texture;
uniform vec4 fg;
uniform vec4 bg;

void main() {
    vec4 color = texture2D(texture, uv);
    if (color.g == 0.) {
        gl_FragColor = bg;
    } else {
        gl_FragColor = fg;
    }
}
