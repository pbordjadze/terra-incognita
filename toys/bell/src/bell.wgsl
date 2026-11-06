// The struck Earth: the globe as the ground pass drew it, moved by its ringing. For each strike the
// radial motion is sum_l c_l P_l(x·e) (the CPU gives each degree's c_l as it swings and fades) plus
// the surface wave's ring running out from it and round the world; the surface rises and falls with
// it (its silhouette too: 0S2 squashes the Earth like a ball), lit from the upper left as it tilts.

struct Bells {
    info: vec4f,              // strikes, the motion's scale, the silhouette on (0 with Reduce Motion), the light's strength
    s: array<vec4f, 48>,      // a strike in six: its place; its ring (angle, height, width); c_0..c_15
};

@group(1) @binding(0) var src: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
@group(1) @binding(2) var<uniform> B: Bells;

// the motion at a world point g: x the height, yzw its gradient (world)
var<private> ring_light: f32;

fn field(g: vec3f) -> vec4f {
    ring_light = 0.0;
    var u = 0.0;
    var grad = vec3f(0.0);
    let n = i32(B.info.x);
    for (var k = 0; k < n; k++) {
        let e = B.s[6 * k].xyz;
        let ring = B.s[6 * k + 1];
        let x = clamp(dot(g, e), -1.0, 1.0);
        var p0 = 1.0; var p1 = x;
        var d0 = 0.0; var d1 = 1.0;
        var su = B.s[6 * k + 2].x;
        var sd = 0.0;
        for (var l = 2; l <= 12; l++) {
            let lf = f32(l);
            let p2 = ((2.0 * lf - 1.0) * x * p1 - (lf - 1.0) * p0) / lf;
            let d2 = d0 + (2.0 * lf - 1.0) * p1;
            let c = B.s[6 * k + 2 + l / 4][l % 4];
            su += c * p2;
            sd += c * d2;
            p0 = p1; p1 = p2; d0 = d1; d1 = d2;
        }
        // the ring: out to the antipode, back, out again (R1, R2, ...), gathering where it focuses
        let gam = acos(x);
        var a = ring.x % (2.0 * PI);
        a = select(a, 2.0 * PI - a, a > PI);
        let dd = (gam - a) / ring.z;
        let sg = sqrt(max(1.0 - x * x, 0.0));
        let focus = inverseSqrt(sg + 0.12) * 0.35;
        let r = ring.y * focus * exp(-dd * dd);
        su += r;
        ring_light += r;
        // d(ring)/dx = d/dγ · dγ/dx, dγ/dx = -1/sin γ
        sd += (-2.0 * dd / ring.z * r) * (-1.0 / max(sg, 0.05));
        u += su;
        grad += sd * (e - x * g);
    }
    return vec4f(u, grad);
}

@group(0) @binding(0) var<uniform> U: View;

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let pt = in.position.xy / U.frame.z;
    let R = U.disc.z;
    let v2 = vec2f(pt.x - U.disc.x, U.disc.y - pt.y) / R;
    let r = length(v2);
    var v = vec3f(v2 / max(r, 1e-6), 0.0);
    if (r < 1.0) { v = vec3f(v2, sqrt(1.0 - r * r)); }
    let g = normalize(U.rot0.xyz * v.x + U.rot1.xyz * v.y + U.rot2.xyz * v.z);
    let F = field(g);
    let amp = B.info.y;
    let u = F.x * amp;
    let scale = 1.0 + clamp(u, -0.05, 0.05) * B.info.z;
    let q2 = v2 / scale;
    let at = vec2f(U.disc.x + q2.x * R, U.disc.y - q2.y * R) * U.frame.z;
    var col = textureSampleLevel(src, samp, at / U.frame.xy, 0.0).rgb;
    // on the moved globe: lit as it tilts, a little warmer where it rises, cooler where it sinks
    let rr = length(q2);
    let fw = max(fwidth(rr), 1e-4);
    let on = 1.0 - smoothstep(1.0 - fw, 1.0, rr);
    let gv = vec3f(dot(U.rot0.xyz, F.yzw), dot(U.rot1.xyz, F.yzw), dot(U.rot2.xyz, F.yzw)) * amp;
    let L = normalize(vec3f(-0.55, 0.65, 0.5));
    let tilt = -dot(gv - v * dot(gv, v), L);
    // soft-limited: a faint swell still reads, a fresh strike never goes black
    let s = tanh(B.info.w * tilt);
    let warm = tanh(u * 30.0);
    // the swells as a soft tint, never darker than a shadow on land; the surface wave as light
    var lit = col * (1.0 + 0.14 * min(s, 0.0)) + vec3f(1.0, 0.97, 0.9) * 0.18 * max(s, 0.0);
    lit = lit * mix(vec3f(1.0), select(vec3f(0.95, 0.99, 1.05), vec3f(1.05, 1.01, 0.95), warm > 0.0), abs(warm) * 0.6);
    // the surface wave: a ring of warm gold laid over the ground, not light added to it, so it reads on the
    // pale day globe as on the night one (painting-company row 246: white on pale blue read as nothing)
    let rl = tanh(max(ring_light, 0.0) * 6.0);
    lit = mix(lit, vec3f(0.98, 0.66, 0.24), 0.62 * rl) + vec3f(1.0, 0.95, 0.82) * 0.12 * rl;
    col = mix(col, lit, on);
    return vec4f(col, 1.0);
}
