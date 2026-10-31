// The maybes: thousands of pale amber specks where the held float might be now (Truth's sampler
// writes their positions; rules.wgsl moves them), each with one dark grain by day so the cloud has
// texture and is read as a cloud, never as a tint. Point-sized instanced quads: a speck is 0.75
// reading points, three pixels on the phone, and costs what a point sprite costs. By night at
// half strength, or they out-shine the cities (CRITICS.md R0).

struct SpeckIn {
    @location(0) g: vec3<f32>,
};

struct SpeckOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) r: f32,
};

struct Maybes {
    bloom: f32,         // 0..1 as they bloom out of the float and fold back
    night: f32,
    _pad: vec2<f32>,
};
@group(1) @binding(0) var<uniform> M: Maybes;

@vertex
fn vs(@builtin(vertex_index) vi: u32, s: SpeckIn) -> SpeckOut {
    let p = place(s.g);
    let r = 0.75 * rp() * M.bloom;
    let c = corner(vi % 6u);
    var o: SpeckOut;
    o.local = c * (r + 0.5 * V.frame.z);
    o.pos = clip(p.xy + o.local);
    if (p.z < 0.3 || M.bloom <= 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.r = r;
    return o;
}

@fragment
fn fs(i: SpeckOut) -> @location(0) vec4<f32> {
    let n = M.night;
    let d = length(i.local);
    var acc = vec4<f32>(0.0);
    let speck = mix(vec3<f32>(0.984, 0.914, 0.800), vec3<f32>(0.914, 0.827, 0.682), n);
    let a = mix(0.80, 0.30, n) * M.bloom;
    acc = over(acc, speck, a * cover(d - i.r));
    acc = over(acc, vec3<f32>(0.557, 0.341, 0.071), 0.5 * (1.0 - n) * M.bloom * cover(d - 0.35 * i.r));
    return acc;
}
