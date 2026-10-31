// The maybes: a held float's other possible selves, thousands of pale amber specks where each would
// be now, each with one dark grain by day so the cloud has texture and is read as a cloud, never as a
// tint; by night at a third, or they out-shine the cities (CRITICS.md R0). The positions come from
// the GPU (gpu/maybes.rs: lon, lat in degrees, held, k; sea/rules.wgsl moved them); one point-sized
// quad a speck, 0.75 reading points, scaled by M.bloom as they bloom out and fold back.

struct Maybe { @location(0) p: vec2f, @location(1) held: u32 };   // lon, lat (degrees), what holds it (Truth's Path)

struct Maybes {
    origin: vec4<f32>,  // the held float's place (a unit vector), where the specks bloom from
    hue: vec4<f32>,     // its lamp's hue: the specks are a pale wash of it
    bloom: f32,         // 0..1 as they bloom out of the float and fold back
    night: f32,
    _p0: f32,
    _p1: f32,
};
@group(1) @binding(0) var<uniform> M: Maybes;

struct SpeckOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) r: f32,
    @location(2) held: f32,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) k: u32, m: Maybe) -> SpeckOut {
    let la = radians(m.p.y);
    let lo = radians(m.p.x);
    // from the float's place out to its own, eased, so blooming out means out (and folding back, back)
    let own = vec3<f32>(cos(la) * sin(lo), sin(la), cos(la) * cos(lo));
    let b = M.bloom * M.bloom * (3.0 - 2.0 * M.bloom);
    let g = normalize(mix(M.origin.xyz, own, b));
    let p = place(g);
    // at globe scale the cloud must read at a glance: each self is a soft kernel seven reading points
    // across, and five hundred of them lie over one another into a field of the lamp's hue (dense where
    // the sea would most likely have carried it); close in they shrink to the specks of a dusting
    let field = 1.0 - smoothstep(1.3, 2.4, U.disc.w);
    let r = mix(0.75, 7.0, field) * rp() * min(M.bloom * 2.0, 1.0);
    let c = corner(vi % 6u);
    var o: SpeckOut;
    o.local = c * (r + 0.5 * U.frame.z);
    o.pos = clip(p.xy + o.local);
    if (p.z < 0.3 || M.bloom <= 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.r = r;
    o.held = f32(m.held);
    return o;
}

@fragment
fn fs(i: SpeckOut) -> @location(0) vec4<f32> {
    let n = M.night;
    let d = length(i.local);
    var acc = vec4<f32>(0.0);
    // a pale wash of the lamp's hue (a self aground is not marked: truth underneath); at globe scale a
    // soft kernel at a low alpha, so the field is the sum of many and glows where they crowd
    let field = 1.0 - smoothstep(1.3, 2.4, U.disc.w);
    let speck = mix(mix(M.hue.rgb, vec3<f32>(1.0), mix(0.72, 0.35, field)), mix(M.hue.rgb, vec3<f32>(1.0), mix(0.55, 0.15, field)), n);
    let soft = exp(-d * d / (0.45 * i.r * i.r));
    // (a low alpha a kernel, so the field is a density: pale where one self passed, the lamp's own
    // colour where many did)
    let a = mix(mix(0.80, 0.30, n), 0.045, field) * min(M.bloom * 2.0, 1.0);
    acc = over(acc, speck, a * mix(cover(d - i.r), soft, field));
    // the grain that gives the cloud texture from afar, gone close in where it read as ringed dots
    let grain = (1.0 - smoothstep(3.5, 5.5, U.disc.w)) * 0.5 * (1.0 - n) * min(M.bloom * 2.0, 1.0);
    acc = over(acc, M.hue.rgb * 0.55, grain * cover(d - 0.35 * i.r));
    return acc;
}
