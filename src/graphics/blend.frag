#version 100
precision lowp float;

varying vec2 uv;
uniform sampler2D render_target;
uniform sampler2D texture;

void main() {
    vec4 dst = texture2D(render_target, uv);
    vec4 src = texture2D(texture, uv);
    float alpha = dst.a * (1. - src.a) + src.a;
    if (alpha < 1. / 255.) {
        gl_FragColor = vec4(0.);
    } else {
        gl_FragColor = vec4(dst.rgb + (src.rgb - dst.rgb) / alpha, 1.) * src.a;
    }
}
