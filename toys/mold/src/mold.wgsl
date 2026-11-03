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
@group(1) @binding(3) var<uniform> look: vec4f;   // x how much of the mold shows, y gain, z the feed's flash

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

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let G = ground_at(U, in.position.xy);
    if (G.r2 >= 1.0) { return vec4f(0.0); }
    let W = i32(P.grid.x); let H = i32(P.grid.y);
    let v = bilinear(0u, G.uv, W, H);
    // the glow: the coarse field read at four offsets round the pixel, a soft pool
    let o = 1.5 / f32(W / 4);
    let gw = 0.25 * (bilinear(1u, G.uv + vec2f(o, 0.0), W / 4, H / 4) + bilinear(1u, G.uv - vec2f(o, 0.0), W / 4, H / 4)
        + bilinear(1u, G.uv + vec2f(0.0, o), W / 4, H / 4) + bilinear(1u, G.uv - vec2f(0.0, o), W / 4, H / 4));
    let k = look.y;
    // a soft shoulder, so where the mold lies thick it stays a body of light, never a white blot
    let core = v * k / (1.0 + v * k);
    let halo = gw * k / (1.0 + gw * k);
    var col = MOLD * (0.9 * core + 0.28 * halo);
    col = mix(col, TUBE * 0.9, smoothstep(0.55, 0.95, core) * 0.35);
    // the flakes: a small pale heap each, a breath brighter when just laid
    let nf = u32(P.misc.y);
    for (var i = 0u; i < nf; i++) {
        let fd = P.feeds[i];
        let w = vec3f(cos(fd.y) * sin(fd.x), sin(fd.y), cos(fd.y) * cos(fd.x));
        let ang = acos(clamp(dot(w, G.g), -1.0, 1.0));
        let px = ang * U.disc.z * U.frame.z;                 // pixels from the flake
        let r = 2.6 * U.frame.z * U.frame.w;
        col += OAT * (0.45 * (1.0 - smoothstep(r * 0.4, r, px)) + 0.12 * exp(-px / (2.5 * r)));
    }
    // the shuttle streaming of a living Physarum: its body pulses, slow waves along the tubes
    let tt = U.tone.z;
    let wave = 0.5 + 0.5 * sin(tt * 1.6 - 9.0 * vnoise3(G.g * 40.0) - 5.0 * vnoise3(G.g * 160.0 + 3.0));
    col *= 0.8 + 0.35 * wave;
    let face = smoothstep(0.0, 0.25, G.v.z);
    return vec4f(col * face * look.x, 0.0);
}
