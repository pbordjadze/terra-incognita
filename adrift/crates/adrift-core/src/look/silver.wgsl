// The nearest real drifter that passed where the held float is: its track as a silver wire on a dark
// line by day (its own light by night), one instanced quad a segment with round joins, thinning
// toward the whole Earth with the beads. Drawn twice, every segment's dark line then every segment's
// wire (S.layer), or a segment's line would cut the wire of the one before it at the join.

struct SegIn {
    @location(0) a: vec3<f32>,
    @location(1) b: vec3<f32>,
    @location(2) t: f32,            // 0..1 along the track (the end is where it was last seen)
};

struct SegOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,  // pixels: x along the segment from its start, y across
    @location(1) len: f32,
    @location(2) w: f32,
};

struct Silver {
    rise: f32,          // 0..1 as it rises among the maybes and sinks back
    night: f32,
    layer: f32,         // 0 this draw lays the dark lines, 1 the wires
    _pad: f32,
};
@group(1) @binding(0) var<uniform> S: Silver;

@vertex
fn vs(@builtin(vertex_index) vi: u32, s: SegIn) -> SegOut {
    let pa = place(s.a);
    let pb = place(s.b);
    let k = by_zoom(0.5, 2.5);
    let w = 2.0 * rp() * k;                                   // the dark line's width; the wire is 0.9
    let d = pb.xy - pa.xy;
    let len = max(length(d), 1e-3);
    let along = d / len;
    let across = vec2<f32>(-along.y, along.x);
    let c = corner(vi % 6u);
    let x = mix(-w, len + w, c.x * 0.5 + 0.5);
    let y = c.y * w;
    var o: SegOut;
    o.local = vec2<f32>(x, y);
    o.pos = clip(pa.xy + along * x + across * y);
    if (min(pa.z, pb.z) < 0.3 || S.rise <= 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.len = len;
    o.w = w;
    return o;
}

@fragment
fn fs(i: SegOut) -> @location(0) vec4<f32> {
    let n = S.night;
    let t = clamp(i.local.x, 0.0, i.len);
    let d = length(i.local - vec2<f32>(t, 0.0));
    var acc = vec4<f32>(0.0);
    let edge = vec3<f32>(0.290, 0.337, 0.388);
    let wire = mix(vec3<f32>(0.867, 0.890, 0.914), vec3<f32>(0.863, 0.890, 0.918), n);
    if (S.layer < 0.5) {
        acc = over(acc, edge, (1.0 - n) * S.rise * cover(d - i.w * 0.5));
    } else {
        acc = over(acc, wire, S.rise * cover(d - i.w * 0.225));
    }
    return acc;
}
