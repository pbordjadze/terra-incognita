// Everyone else's lamps (Together, painting-company row 199): a small warm point on the night sea, a
// soft halo round it, breathing slowly as a candle does, each at its own pace. A lamp just set blooms
// in; two that have just met glow toward each other, the halo drawn out along the line between them.
// A lamp is a marker: its size is in reading points and grows a little with the zoom, never with the
// water. Premultiplied alpha, over the sea.

struct LampIn {
    @location(0) g: vec3<f32>,
    @location(1) alpha: f32,
    @location(2) toward: vec3<f32>,
    @location(3) glow: f32,
    @location(4) warm: f32,
    @location(5) arrival: f32,
    @location(6) phase: f32,
    @location(7) pad: f32,
};

struct LampOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,  // pixels from the lamp
    @location(1) off: vec2<f32>,    // pixels to the half-way point toward the other
    @location(2) r: f32,            // the core's radius, pixels
    @location(3) alpha: f32,
    @location(4) glow: f32,
    @location(5) warm: f32,
    @location(6) arrival: f32,
    @location(7) phase: f32,
};

fn core_radius() -> f32 {
    // 0.9 points at the whole Earth, 1.8 by zoom 4
    return mix(0.9, 1.8, smoothstep(1.0, 4.0, U.disc.w)) * rp();
}

@vertex
fn vs(@builtin(vertex_index) vi: u32, l: LampIn) -> LampOut {
    let p = place(l.g);
    let q = place(l.toward);
    let r = core_radius();
    var off = (q.xy - p.xy) * 0.5;
    let room = 26.0 * rp();
    if (length(off) > room) { off = normalize(off) * room; }
    if (l.glow <= 0.0) { off = vec2<f32>(0.0); }
    let reach = 9.0 * r + length(off) + 6.0 * rp() * max(l.glow, l.arrival);
    let c = corner(vi % 6u);
    var o: LampOut;
    o.local = c * reach;
    o.pos = clip(p.xy + o.local);
    let facing = smoothstep(0.02, 0.3, p.z);
    if (p.z < 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.off = off;
    o.r = r;
    o.alpha = l.alpha * facing;
    o.glow = l.glow;
    o.warm = l.warm;
    o.arrival = l.arrival;
    o.phase = l.phase;
    return o;
}

// The distance from x to the segment from 0 to b.
fn to_segment(x: vec2<f32>, b: vec2<f32>) -> f32 {
    let bb = max(dot(b, b), 1e-4);
    let t = clamp(dot(x, b) / bb, 0.0, 1.0);
    return length(x - b * t);
}

@fragment
fn fs(i: LampOut) -> @location(0) vec4<f32> {
    let d = length(i.local);
    let r = i.r;
    // a candle's colour, each its own between amber and a paler flame
    let hue = mix(vec3<f32>(0.976, 0.627, 0.239), vec3<f32>(1.0, 0.808, 0.553), i.warm);
    let core = vec3<f32>(1.0, 0.953, 0.847);
    // breathing: slow, small, each at its own pace
    let t = U.time.x;
    let breath = 0.84 + 0.16 * sin(t * (0.9 + 0.5 * i.warm) + i.phase) * sin(t * 0.37 + i.phase * 1.7);
    let a = i.alpha * breath;
    var acc = vec4<f32>(0.0);
    // the light on the water
    let halo_r = 3.6 * r;
    acc = over(acc, hue, 0.22 * a * exp(-d * d / (halo_r * halo_r)));
    // a meeting: a glow drawn toward the other, brighter at the start, fading
    if (i.glow > 0.0) {
        let g = i.glow * i.glow * (3.0 - 2.0 * i.glow);
        let s = to_segment(i.local, i.off);
        let w = 3.2 * r + 4.0 * rp() * g;
        acc = over(acc, hue, 0.55 * g * i.alpha * exp(-s * s / (w * w)));
        acc = over(acc, core, 0.35 * g * i.alpha * exp(-d * d / (1.8 * r * 1.8 * r)));
    }
    // arriving: a ring opens once and the light swells in
    if (i.arrival > 0.0) {
        let k = 1.0 - i.arrival;
        let rr = mix(1.5 * r, 9.0 * r, k);
        acc = over(acc, hue, 0.45 * i.arrival * i.alpha * cover(abs(d - rr) - 0.4 * rp()));
    }
    // the flame
    acc = over(acc, hue, a * cover(d - r));
    acc = over(acc, core, a * 0.85 * cover(d - 0.5 * r));
    return acc;
}
