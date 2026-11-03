// The waves on the sea: the simulated height (sea.rs) on the globe's grid, drawn only as light: a
// crest's face toward the light catches it, its back goes a little deeper blue, its height glows
// faintly. Nothing of the floor is drawn; it shows in how the light moves.

struct Waves { info: vec4f };  // x the light's gain, y the glow, z the texel's width, w its height (uv)

@group(0) @binding(0) var<uniform> U: View;
@group(1) @binding(0) var eta: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
@group(1) @binding(2) var<uniform> S: Waves;

fn at(uv: vec2f) -> f32 { return textureSampleLevel(eta, samp, uv, 0.0).r; }

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let G = ground_at(U, in.position.xy);
    if (G.r2 >= 1.0) { return vec4f(0.0); }
    let du = S.info.z; let dv = S.info.w;
    let h = at(G.uv);
    let ex = at(G.uv + vec2f(du, 0.0)) - at(G.uv - vec2f(du, 0.0));
    let ny = at(G.uv - vec2f(0.0, dv)) - at(G.uv + vec2f(0.0, dv));
    let dl = 2.0 * PI * du * 2.0;   // radians across the difference
    let dp = PI * dv * 2.0;
    let cl = cos(G.lat);
    let east = vec3f(cos(G.lon), 0.0, -sin(G.lon));
    let north = vec3f(-sin(G.lat) * sin(G.lon), cl, -sin(G.lat) * cos(G.lon));
    let grad = east * (ex / dl) / max(cl, 0.2) + north * (ny / dp);
    let gv = vec3f(dot(U.rot0.xyz, grad), dot(U.rot1.xyz, grad), dot(U.rot2.xyz, grad));
    let L = normalize(vec3f(-0.55, 0.65, 0.5));
    let tilt = -dot(gv - G.v * dot(gv, G.v), L);
    let s = tanh(S.info.x * tilt);
    let rim = smoothstep(0.0, 0.12, G.v.z);
    let b = max(s, 0.0) * 0.8 * rim;
    let d = max(-s, 0.0) * 0.55 * rim;
    let glow = tanh(abs(h) * S.info.y) * 0.28 * rim;
    let light = vec3f(1.0, 0.99, 0.94);
    let deep = vec3f(0.10, 0.22, 0.42);
    let a = clamp(b + d + glow, 0.0, 0.85);
    return vec4f(light * (b + glow) + deep * d, a);
}
