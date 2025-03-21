#version 100
precision lowp float;

varying vec2 uv;
uniform sampler2D texture;
uniform vec4 color;
uniform vec2 size;
uniform float time;

float tex(float x, float y) {
    return texture2D(texture, vec2(x, y)).a;
}

void main() {
    float x = uv.x;
    float y = uv.y;
    vec2 d = 0.5 / size;
    float c = tex(x, y);
    float n  = tex(x      , y - d.y);
    float ne = tex(x + d.x, y - d.y);
    float e  = tex(x + d.x, y      );
    float se = tex(x + d.x, y + d.y);
    float s  = tex(x      , y + d.y);
    float sw = tex(x - d.x, y + d.y);
    float w  = tex(x - d.x, y      );
    float nw = tex(x - d.x, y - d.y);
    float t = mod(floor((uv.x / d.x + uv.y / d.y) / 10. + time * 2.), 2.);
    bool interior = false;
    if (c == 0.) {
        if (n == 1. || ne == 1. || e == 1. || se == 1.
         || s == 1. || sw == 1. || w == 1. || nw == 1.) {
            c = 1.;
        }
    } else {
        d *= 2.;
        if (!(x < d.x || x > 1. - d.x || y < d.y || y > 1. - d.y
         || n == 0. || ne == 0. || e == 0. || se == 0.
         || s == 0. || sw == 0. || w == 0. || nw == 0.)) {
            interior = true;
        }
    }
    if (interior) {
        gl_FragColor = color;
    } else {
        if (c == 1.) {
            gl_FragColor = vec4(vec3(t), c);
        } else {
            gl_FragColor = vec4(0.);
        }
    }
}
