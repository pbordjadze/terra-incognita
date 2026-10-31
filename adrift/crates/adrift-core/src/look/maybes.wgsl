// The maybes: a held float's other possible selves, thousands of pale amber specks where each would
// be now, each with one dark grain by day so the cloud has texture and is read as a cloud, never as a
// tint; by night at a third, or they out-shine the cities (CRITICS.md R0). The positions come from
// the GPU (gpu/maybes.rs: lon, lat in degrees, held, k; sea/rules.wgsl moved them); one point-sized
// quad a speck, 0.75 reading points, scaled by M.bloom as they bloom out and fold back.

struct Maybe { @location(0) p: vec2f, @location(1) held: u32 };   // lon, lat (degrees), what holds it (Truth's Path)

struct Maybes {
    origin: vec4<f32>,  // the held float's place (a unit vector), where the specks bloom from
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
    let r = 0.75 * rp() * min(M.bloom * 2.0, 1.0);
    let c = corner(vi % 6u);
    var o: SpeckOut;
    o.local = c * (r + 0.5 * U.frame.z);
    o.pos = clip(p.xy + o.local);
    // toward the whole Earth the cloud is a dusting, not a smudge: a share of the specks shows (a
    // quarter at zoom 1, all from zoom 3), chosen by a hash of each one's number so the cloud keeps
    // its shape as the camera comes and goes
    let share = by_zoom(0.25, 3.0);
    let pick = fract(sin(f32(k) * 12.9898 + 78.233) * 43758.5453);
    if (p.z < 0.3 || M.bloom <= 0.0 || pick > share) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.r = r;
    o.held = f32(m.held);
    return o;
}

@fragment
fn fs(i: SpeckOut) -> @location(0) vec4<f32> {
    let n = M.night;
    let d = length(i.local);
    var acc = vec4<f32>(0.0);
    // a self held at a coast or in ice is a little greyer: it stopped
    let speck = mix(mix(vec3<f32>(0.984, 0.914, 0.800), vec3<f32>(0.914, 0.827, 0.682), n), vec3<f32>(0.85, 0.84, 0.80), 0.6 * i.held);
    let a = mix(0.80, 0.30, n) * min(M.bloom * 2.0, 1.0) * by_zoom(0.7, 3.0);
    acc = over(acc, speck, a * cover(d - i.r));
    acc = over(acc, vec3<f32>(0.557, 0.341, 0.071), 0.5 * (1.0 - n) * min(M.bloom * 2.0, 1.0) * cover(d - 0.35 * i.r));
    return acc;
}
