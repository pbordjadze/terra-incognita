// The wake: one bead a day, amber on a dark edge by day (its own light by night), newest largest,
// fading and shrinking with age like a comet's tail; bunched where the float idled in an eddy, spread
// where a jet carried it. Instanced quads, one a bead, in reading points, scaled down toward the
// whole Earth so forty-five beads don't pile into a slug (CRITICS.md R0).

struct BeadIn {
    @location(0) g: vec3<f32>,      // where the float was, that day
    @location(1) age: f32,          // 0 the newest bead .. 1 the oldest shown
    @location(2) night: f32,
};

struct BeadOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) r: f32,
    @location(2) a: f32,
    @location(3) night: f32,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, b: BeadIn) -> BeadOut {
    let p = place(b.g);
    let k = by_zoom(0.45, 2.5);
    let r = mix(2.2, 1.2, b.age) * rp() * k;
    let c = corner(vi % 6u);
    var o: BeadOut;
    o.local = c * (r + 1.0 * rp());
    o.pos = clip(p.xy + o.local);
    if (p.z < 0.3) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.r = r;
    o.a = pow(1.0 - b.age, 1.4) * 0.95 + 0.05;
    o.night = b.night;
    return o;
}

@fragment
fn fs(i: BeadOut) -> @location(0) vec4<f32> {
    let n = i.night;
    let d = length(i.local);
    var acc = vec4<f32>(0.0);
    let edge = vec3<f32>(0.549, 0.310, 0.063);                      // bead_edge, by day only
    let bead = mix(vec3<f32>(0.910, 0.584, 0.173), vec3<f32>(0.941, 0.627, 0.251), n);
    acc = over(acc, edge, (1.0 - n) * i.a * cover(d - (i.r + 0.45 * V.frame.z)));
    acc = over(acc, bead, i.a * cover(d - i.r));
    return acc;
}
