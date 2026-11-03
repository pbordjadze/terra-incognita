// The mold drawn on the night globe: its body (the cells its agents hold, eased in time), as a living
// light over the ground (added, never covering it): the tubes a pale foxfire, whitening where they
// run thick, in a soft glow of their own; and the flakes a finger fed, small oat-pale heaps.

struct Params {
    grid: vec4u,
    mv: vec4f,
    tr: vec4f,
    misc: vec4f,
    ex: vec4f,
    feeds: array<vec4f, 32>,
};

@group(1) @binding(0) var<uniform> P: Params;
@group(1) @binding(1) var<storage, read> body: array<f32>;
@group(1) @binding(2) var<storage, read> glow: array<f32>;
// a: x how much shows, y the gain, z the streaks' share (by zoom), w the grid's rows; ages: x each
// flake's seconds since it was laid
struct Look { a: vec4f, ages: array<vec4f, 32> };
@group(1) @binding(3) var<uniform> look: Look;

const MOLD = vec3f(0.55, 1.0, 0.72);    // foxfire
const TUBE = vec3f(0.86, 1.0, 0.80);    // a thick tube's heart
const OAT = vec3f(1.0, 0.93, 0.78);

fn texel(buf: u32, x: i32, y: i32, W: i32, H: i32) -> f32 {
    let xx = ((x % W) + W) % W;
    let yy = clamp(y, 0, H - 1);
    if (buf == 0u) { return body[yy * W + xx]; }
    return glow[yy * W + xx];
}

fn bilinear(buf: u32, uv: vec2f, W: i32, H: i32) -> f32 {
    let f = uv * vec2f(f32(W), f32(H)) - 0.5;
    let i = vec2i(floor(f));
    let t = f - floor(f);
    let a = texel(buf, i.x, i.y, W, H);
    let b = texel(buf, i.x + 1, i.y, W, H);
    let c = texel(buf, i.x, i.y + 1, W, H);
    let d = texel(buf, i.x + 1, i.y + 1, W, H);
    return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

// the body read smoothly (a cubic B-spline over 4 x 4 cells): close in its contours are round and
// continuous, never the grid's staircase
fn bspline_w(t: f32) -> vec4f {
    let t2 = t * t; let t3 = t2 * t;
    return vec4f((1.0 - t) * (1.0 - t) * (1.0 - t), 3.0 * t3 - 6.0 * t2 + 4.0, -3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0, t3) / 6.0;
}
fn smooth_body(uv: vec2f, W: i32, H: i32) -> f32 {
    let f = uv * vec2f(f32(W), f32(H)) - 0.5;
    let i = vec2i(floor(f));
    let t = f - floor(f);
    let wx = bspline_w(t.x); let wy = bspline_w(t.y);
    var s = 0.0;
    for (var y = 0; y < 4; y++) {
        var row = 0.0;
        for (var x = 0; x < 4; x++) { row += wx[x] * texel(0u, i.x + x - 1, i.y + y - 1, W, H); }
        s += wy[y] * row;
    }
    return s;
}

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let G = ground_at(U, in.position.xy);
    if (G.r2 >= 1.0) { return vec4f(0.0); }
    let W = i32(P.grid.x); let H = i32(P.grid.y);
    let v = bilinear(0u, G.uv, W, H);
    // the glow: the coarse field read at four offsets round the pixel, a soft pool
    let o = 1.5 / f32(W / 4);
    let gw = 0.25 * (bilinear(1u, G.uv + vec2f(o, 0.0), W / 4, H / 4) + bilinear(1u, G.uv - vec2f(o, 0.0), W / 4, H / 4)
        + bilinear(1u, G.uv + vec2f(0.0, o), W / 4, H / 4) + bilinear(1u, G.uv - vec2f(0.0, o), W / 4, H / 4));
    let k = look.a.y;
    let share = look.a.z;
    // a soft shoulder, so where the mold lies thick it stays a body of light, never a white blot
    let core = v * k / (1.0 + v * k);
    let halo = gw * k / (1.0 + gw * k);
    // close in, the field's core gives way to the agents' own streaks (streaks.wgsl); its glow stays
    var col = MOLD * (0.9 * core * (1.0 - 0.9 * share) + 0.28 * halo * (1.0 - 0.4 * share));
    col = mix(col, TUBE * 0.9, smoothstep(0.55, 0.95, core) * 0.35 * (1.0 - share));
    // close in: the tubes as bodies of their own, the smooth field's band above a threshold, its edge a
    // pixel wide whatever the zoom, a brighter heart where the field runs high
    let sv = smooth_body(G.uv, W, H);
    let fw = max(fwidth(sv), 1e-4);
    if (share > 0.0) {
        let inside = (sv - 3.2) / fw;                     // pixels inside the tube's edge
        let edge = clamp(inside + 0.5, 0.0, 1.0);
        let rim = exp(-max(inside, 0.0) / (1.2 * U.frame.z)) * edge;   // its membrane, a bright line
        let heart = smoothstep(5.5, 12.0, sv);
        let tube = MOLD * (0.10 + 0.28 * heart) * edge + MOLD * 0.42 * rim + TUBE * 0.18 * heart * heart * edge;
        col = col * (1.0 - 0.6 * share) + tube * share;
    }
    // the flakes: a small pale heap each; the mold by one it is feeding on brightens as it gathers there,
    // and a faint ring goes out once from a flake just laid
    let nf = u32(P.misc.y);
    var fedb = 1.0;
    var flakes = vec3f(0.0);
    for (var i = 0u; i < nf; i++) {
        let fd = P.feeds[i];
        let age = look.ages[i].x;
        let w = vec3f(cos(fd.y) * sin(fd.x), sin(fd.y), cos(fd.y) * cos(fd.x));
        let ang = acos(clamp(dot(w, G.g), -1.0, 1.0));
        let px = ang * U.disc.z * U.frame.z;                 // pixels from the flake
        let r = 2.6 * U.frame.z * U.frame.w;
        flakes += OAT * (0.45 * (1.0 - smoothstep(r * 0.4, r, px)) + 0.12 * exp(-px / (2.5 * r)));
        fedb += 1.0 * exp(-(ang / (PI / look.a.w)) / 14.0) * smoothstep(0.0, 3.0, age);
        let ring = age * 70.0 * U.frame.z * U.frame.w;      // 70 reading points a second
        let fade = (1.0 - smoothstep(0.0, 1.4, age));
        flakes += OAT * 0.22 * fade * exp(-sq((px - ring) / (2.0 * U.frame.z)));
    }
    col = col * fedb + flakes;
    // the shuttle streaming of a living Physarum: its body pulses, slow waves along the tubes
    let tt = U.tone.z;
    let wave = 0.5 + 0.5 * sin(tt * 1.6 - 9.0 * vnoise3(G.g * 40.0) - 5.0 * vnoise3(G.g * 160.0 + 3.0));
    col *= 0.8 + 0.35 * wave;
    let face = smoothstep(0.0, 0.25, G.v.z);
    return vec4f(col * face * look.a.x, 0.0);
}
