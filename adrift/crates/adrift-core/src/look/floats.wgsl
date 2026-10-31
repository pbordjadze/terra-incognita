// The floats (and the ring that thickens into one): one instanced quad a float, drawn on the screen
// in reading points from the palette (palette.json; PALETTE.md says why). A float is a marker: its
// radius goes from 4.5 to 7 points between zoom 1 and 3 and is then held; it never grows with the
// water. Premultiplied alpha, over the sea.

struct FloatIn {
    @location(0) g: vec3<f32>,      // the float's position, a unit vector
    @location(1) beat: f32,         // 0..1 while the finger rests and the ring thickens; 1 a float
    @location(2) lit: f32,          // 1 while it is what you just did (stays lit), fading in time
    @location(3) held: f32,         // 1 while held (its maybes out)
    @location(4) night: f32,        // 1 by night (the night palette)
};

struct FloatOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,  // pixels from the centre
    @location(1) r: f32,            // the float's radius, pixels
    @location(2) beat: f32,
    @location(3) lit: f32,
    @location(4) held: f32,
    @location(5) night: f32,
    @location(6) facing: f32,
};

fn float_radius() -> f32 {
    let t = clamp((U.disc.w - 1.0) / 2.0, 0.0, 1.0);
    return (4.5 + 2.5 * t) * rp();
}

@vertex
fn vs(@builtin(vertex_index) vi: u32, f: FloatIn) -> FloatOut {
    let p = place(f.g);
    let r = float_radius();
    // room for the ring at the start of the beat (16 points) and the glow (2.6 r)
    let reach = max(18.0 * rp() * (1.0 - f.beat) + r, 2.8 * r);
    let c = corner(vi % 6u);
    var o: FloatOut;
    o.local = c * reach;
    o.pos = clip(p.xy + o.local);
    if (p.z < 0.3) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }   // (hidden past the limb: clipped)
    o.r = r; o.beat = f.beat; o.lit = f.lit; o.held = f.held; o.night = f.night; o.facing = p.z;
    return o;
}

// The palette (palette.json: day, then night)
fn glass_rim(n: f32) -> vec3<f32> { return mix(vec3<f32>(0.118, 0.290, 0.345), vec3<f32>(0.725, 0.867, 0.890), n); }
fn glass_body(n: f32) -> vec4<f32> { return mix(vec4<f32>(0.561, 0.816, 0.776, 0.70), vec4<f32>(0.247, 0.478, 0.525, 0.45), n); }
fn glass_light(n: f32) -> f32 { return mix(0.95, 0.55, n); }
fn lamp(n: f32) -> vec3<f32> { return mix(vec3<f32>(0.957, 0.647, 0.165), vec3<f32>(0.965, 0.682, 0.227), n); }
fn lamp_core(n: f32) -> vec3<f32> { return mix(vec3<f32>(1.0, 0.914, 0.722), vec3<f32>(1.0, 0.941, 0.784), n); }
fn lamp_glow_a(n: f32) -> f32 { return mix(0.30, 0.40, n); }
fn lit_a(n: f32) -> f32 { return mix(0.22, 0.28, n); }
fn ring_shade(n: f32) -> vec4<f32> { return mix(vec4<f32>(0.129, 0.322, 0.498, 0.35), vec4<f32>(0.0, 0.0, 0.0, 0.0), n); }

@fragment
fn fs(i: FloatOut) -> @location(0) vec4<f32> {
    let n = i.night;
    let d = length(i.local);
    let r = i.r;
    let rim = 1.1 * rp();
    var acc = vec4<f32>(0.0);
    let t = i.beat;

    if (t < 1.0) {
        // the ring that thickens: drawing in from 16 points to the float's radius, thin to the rim's
        // width, the glass forming over the middle of the beat and the lamp lighting at its end
        let rr = mix(16.0 * rp(), r, t * t) * (1.0 - t * t) + r * t * t;
        let w = mix(0.8 * rp(), rim, t);
        let sh = ring_shade(n);
        acc = over(acc, sh.rgb, sh.a * (1.0 - 0.5 * t) * cover(abs(d - (rr + w * 1.2)) - w * 0.7));
        acc = over(acc, vec3<f32>(1.0), mix(0.9, 0.7, n) * (1.0 - 0.3 * t) * cover(abs(d - rr) - w * 0.5));
        let k = clamp((t - 0.4) / 0.4, 0.0, 1.0);
        let gb = glass_body(n);
        acc = over(acc, gb.rgb, gb.a * k * cover(d - (rr - w * 0.5)));
        acc = over(acc, glass_rim(n), k * cover(abs(d - rr) - w * 0.5));
        let kl = clamp((t - 0.5) / 0.5, 0.0, 1.0);          // (the amber from half the beat: CRITICS.md R0)
        acc = over(acc, lamp(n), kl * cover(d - 0.55 * r * kl));
        return acc;
    }

    // what you just did stays lit, and the lamp's light on the water
    acc = over(acc, lamp(n), lit_a(n) * i.lit * exp(-d * d / (2.4 * 2.4 * r * r)));
    acc = over(acc, lamp(n), lamp_glow_a(n) * exp(-d * d / (1.25 * 1.25 * r * r)));
    // the glass, the lamp, its core
    let gb = glass_body(n);
    acc = over(acc, gb.rgb, gb.a * cover(d - r));
    let rl = 0.55 * r;
    acc = over(acc, lamp(n), cover(d - rl));
    acc = over(acc, lamp_core(n), pow(clamp(1.0 - d / (rl * 0.72), 0.0, 1.0), 1.6));
    // the rim, and the sun's highlight upper left (where the sea's sheen is)
    acc = over(acc, glass_rim(n), cover(abs(d - (r - rim * 0.5)) - rim * 0.5));
    let h = i.local - vec2<f32>(-0.36 * r, -0.42 * r);
    acc = over(acc, vec3<f32>(1.0), glass_light(n) * exp(-((h.x * 1.2) * (h.x * 1.2) + (h.y * 2.4) * (h.y * 2.4)) / (0.16 * r * r)));
    // held: a thin ring out at 1.55 r, the maybes' own
    acc = over(acc, glass_rim(n), 0.45 * i.held * cover(abs(d - 1.55 * r) - 0.3 * rp()));
    return acc;
}
