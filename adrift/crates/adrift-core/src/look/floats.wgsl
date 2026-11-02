// The floats (and the ring that thickens into one): one instanced quad a float, drawn on the screen
// in reading points from the palette (palette.json; PALETTE.md says why). A float is a marker: its
// radius goes from 4.5 to 7 points between zoom 1 and 3 and is then held; it never grows with the
// water. Under a resting finger (beat < 1) the ring draws in and the glass forms, without a lamp; the
// float the app puts at the beat arrives with its lamp lit. Premultiplied alpha, over the sea.

struct FloatIn {
    @location(0) g: vec3<f32>,      // the float's position, a unit vector
    @location(1) beat: f32,         // 0..1 while the finger rests and the ring thickens; 1 a float
    @location(2) lit: f32,          // 1 while it is what you just did (stays lit), fading in time
    @location(3) held: f32,         // 1 while held (its maybes out)
    @location(4) night: f32,        // 1 by night (the night palette)
    @location(5) measured: f32,     // how measured the sea is where it rides (0..1): under a half the rim is dashed
    @location(6) hue: vec3<f32>,    // the lamp's colour (amber the first, then each its own)
    @location(7) speck: f32,        // 0 a glass float, 1 one maybe among its cloud's (R6), between as it blooms
    @location(8) head: f32,         // 1 in a clip (R20): the line's head, a larger glass float with a glint and a glow
    @location(9) mode: f32,         // how it is chosen (R21): 0 not, 1 as itself, 2 as its cloud
    @location(10) quiet: f32,       // 1 not chosen while another is: it steps back
    @location(11) kept: f32,        // kept for good: a gold bead at its shoulder
    @location(12) pad: f32,
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
    @location(7) measured: f32,
    @location(8) hue: vec3<f32>,
    @location(9) speck: f32,
    @location(10) head: f32,
    @location(11) @interpolate(flat) mqk: vec3<f32>,   // mode, quiet, kept
};

fn float_radius() -> f32 {
    let t = clamp((U.disc.w - 1.0) / 2.0, 0.0, 1.0);
    return (4.5 + 2.5 * t) * rp();
}

@vertex
fn vs(@builtin(vertex_index) vi: u32, f: FloatIn) -> FloatOut {
    let p = place(f.g);
    // (a clip's head, R20: a size up, so it anchors the eye at 1080 x 1920)
    let r = float_radius() * (1.0 + 0.3 * f.head);
    // room for the ring at the start of the beat (16 points) and the glow (2.6 r)
    let reach = max(max(18.0 * rp() * (1.0 - f.beat) + r, mix(2.4, 3.6, f.head) * r), select(0.0, 2.3 * r + 2.0 * rp(), f.mode > 0.5));
    let c = corner(vi % 6u);
    var o: FloatOut;
    o.local = c * reach;
    o.pos = clip(p.xy + o.local);
    // (past the limb gone; toward it fading by its facing, as the maybes do, never clipped at a line)
    if (p.z < 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.speck = f.speck;
    o.head = f.head;
    o.mqk = vec3<f32>(f.mode, f.quiet, f.kept);
    o.r = r; o.beat = f.beat; o.lit = f.lit; o.held = f.held; o.night = f.night; o.facing = p.z; o.measured = f.measured; o.hue = f.hue;
    return o;
}

// The palette (palette.json: day, then night)
fn glass_rim(n: f32) -> vec3<f32> { return mix(vec3<f32>(0.118, 0.290, 0.345), vec3<f32>(0.725, 0.867, 0.890), n); }
fn glass_body(n: f32) -> vec4<f32> { return mix(vec4<f32>(0.561, 0.816, 0.776, 0.70), vec4<f32>(0.247, 0.478, 0.525, 0.45), n); }
fn glass_light(n: f32) -> f32 { return mix(0.95, 0.55, n); }
fn lamp(n: f32) -> vec3<f32> { return mix(vec3<f32>(0.957, 0.647, 0.165), vec3<f32>(0.965, 0.682, 0.227), n); }
fn lamp_of(h: vec3<f32>, n: f32) -> vec3<f32> { return mix(h, h * 1.04, n); }
fn lamp_core(n: f32) -> vec3<f32> { return mix(vec3<f32>(1.0, 0.914, 0.722), vec3<f32>(1.0, 0.941, 0.784), n); }
fn lamp_glow_a(n: f32) -> f32 { return mix(0.30, 0.40, n); }
fn lit_a(n: f32) -> f32 { return mix(0.22, 0.28, n); }
fn ring_shade(n: f32) -> vec4<f32> { return mix(vec4<f32>(0.129, 0.322, 0.498, 0.35), vec4<f32>(0.0, 0.0, 0.0, 0.0), n); }

@fragment
fn fs(i: FloatOut) -> @location(0) vec4<f32> {
    let rim_fade = smoothstep(0.02, 0.32, i.facing);
    var glass = glass_float(i);
    if (i.speck > 0.0) { glass = glass * (1.0 - i.speck) + as_maybe(i) * i.speck; }
    // (R21) a lamp not chosen while another is steps back: fainter, so the chosen read at a glance
    glass = glass * (1.0 - 0.5 * i.mqk.y);
    let marks = chosen_marks(i);
    return (marks + glass * (1.0 - marks.a)) * rim_fade;
}

// The lamp drawn as one maybe of its own cloud (maybes.wgsl's speck, the same sizes and inks), so the
// cloud's possibilities are all equal and nothing on the globe says which one is "the" lamp (R6).
fn as_maybe(i: FloatOut) -> vec4<f32> {
    let n = i.night;
    let d = length(i.local);
    let field = 1.0 - smoothstep(1.3, 2.4, U.disc.w);
    let r = mix(mix(0.75, 1.0, n), 7.0, field) * rp();
    var acc = vec4<f32>(0.0);
    let night_hue = mix(i.hue, vec3<f32>(0.70, 0.90, 1.0), 0.62);
    if (n > 0.5) {
        let close = 1.0 - field;
        acc = over(acc, night_hue, 0.5 * close * cover(abs(d - 0.72 * r) - 0.2 * r));
        acc = over(acc, i.hue, 0.6 * close * cover(d - 0.24 * r));
        return acc;
    }
    let speck = mix(i.hue, vec3<f32>(1.0), mix(0.72, 0.35, field));
    acc = over(acc, speck, 0.80 * (1.0 - field) * cover(d - r));
    let grain = (1.0 - smoothstep(3.5, 5.5, U.disc.w)) * mix(0.5, 0.3, field);
    acc = over(acc, i.hue * 0.55, grain * cover(d - mix(0.35 * r, 0.3 * rp(), field)));
    return acc;
}

fn glass_float(i: FloatOut) -> vec4<f32> {
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
        // (no lamp during the press: the lamp lights on Put, when the float exists, so the beat is the
        // moment the light comes on and a lift before it only un-forms glass)
        return acc;
    }

    // what you just did stays lit, and the lamp's light on the water
    let hue = lamp_of(i.hue, n);
    // a clip's head (R20): a soft glow of its hue on the water round it, breathing slowly, so the eye
    // finds the line's head at a glance without the glow outshining the line
    let breath = 0.85 + 0.15 * sin(U.time.x * 1.7);
    acc = over(acc, hue, i.head * 0.30 * breath * exp(-d * d / (2.6 * 2.6 * r * r)));
    acc = over(acc, hue, lit_a(n) * i.lit * exp(-d * d / (1.9 * 1.9 * r * r)));
    acc = over(acc, hue, lamp_glow_a(n) * exp(-d * d / (1.25 * 1.25 * r * r)));
    // a cool hue's pale ring round the glass (R21), so a blue or purple lamp stands off the sea's blue
    acc = over(acc, vec3<f32>(0.96, 0.98, 1.0), coolness(i.hue) * (1.0 - n) * 0.85 * step(i.mqk.x, 0.5) * cover(abs(d - (r + 0.9 * rp())) - 0.6 * rp()));
    // the glass, the lamp, its core
    let gb = glass_body(n);
    acc = over(acc, gb.rgb, gb.a * cover(d - r));
    let rl = 0.55 * r;
    acc = over(acc, hue, cover(d - rl));
    acc = over(acc, mix(lamp_core(n), hue, 0.25), pow(clamp(1.0 - d / (rl * 0.72), 0.0, 1.0), 1.6));
    // the rim, dashed where the sea's current is the sheet's guess and no drifter measured it (Truth's
    // D3: nothing claims the current where no drifter went), and the sun's highlight upper left
    var dash = 1.0;
    if (i.measured < 0.5) {
        let ang = atan2(i.local.y, i.local.x);
        dash = step(0.0, sin(ang * 8.0));
    }
    acc = over(acc, glass_rim(n), dash * cover(abs(d - (r - rim * 0.5)) - rim * 0.5));
    let h = i.local - vec2<f32>(-0.36 * r, -0.42 * r);
    acc = over(acc, vec3<f32>(1.0), glass_light(n) * exp(-((h.x * 1.2) * (h.x * 1.2) + (h.y * 2.4) * (h.y * 2.4)) / (0.16 * r * r)));
    // and the glint of a clip's head (R20): the sun's highlight crisper, a bright point on the glass with
    // a small four-pointed sparkle, and a faint second light low right where the water lights the glass
    let gp = i.local - vec2<f32>(-0.34 * r, -0.40 * r);
    let star = exp(-abs(gp.x) / (0.05 * r)) * exp(-abs(gp.y) / (0.45 * r)) + exp(-abs(gp.y) / (0.05 * r)) * exp(-abs(gp.x) / (0.45 * r));
    acc = over(acc, vec3<f32>(1.0), i.head * clamp(0.9 * exp(-dot(gp, gp) / (0.012 * r * r)) + 0.55 * star * (0.8 + 0.2 * breath), 0.0, 1.0));
    let low = i.local - vec2<f32>(0.40 * r, 0.42 * r);
    acc = over(acc, mix(vec3<f32>(1.0), hue, 0.4), i.head * 0.35 * exp(-dot(low, low) / (0.03 * r * r)));
    // held: a thin ring out at 1.55 r, the maybes' own
    // (in its lamp's colour, R8: a lamp chosen as itself is ringed by its own light)
    acc = over(acc, mix(glass_rim(n), i.hue, 0.65), 0.6 * i.held * step(i.mqk.x, 0.5) * cover(abs(d - 1.55 * r) - 0.35 * rp()));
    return acc;
}

// How a lamp is chosen, at a glance (R21; the panel's marks carry the same signs): as itself, a solid
// ring of its hue close round the glass, its own line; as its cloud, a ring of dots of its hue, its
// maybes; kept, a small gold bead at its shoulder (a jewel, not a ring, so it never reads as chosen).
// Each on a pale rim by day so a cool hue holds against the sea. `held` is the marks' strength.
fn chosen_marks(i: FloatOut) -> vec4<f32> {
    let n = i.night;
    let d = length(i.local);
    let r = i.r;
    let a = clamp(i.held, 0.0, 1.0);
    let hue = lamp_of(i.hue, n);
    let pale = vec3<f32>(0.97, 0.985, 1.0);
    var acc = vec4<f32>(0.0);
    if (i.mqk.x > 0.5 && i.mqk.x < 1.5) {
        let rr = 1.7 * r;
        let w = 0.95 * rp();
        acc = over(acc, pale, (1.0 - n) * 0.8 * a * cover(abs(d - rr) - (w + 0.9 * rp())));
        acc = over(acc, hue, a * cover(abs(d - rr) - w));
    } else if (i.mqk.x > 1.5) {
        let rr = 2.0 * r;
        let ang = atan2(i.local.y, i.local.x) + 1.5707963;
        let k = round(ang * 10.0 / 6.2831853);
        let t = k * 6.2831853 / 10.0 - 1.5707963;
        let q = i.local - vec2<f32>(cos(t), sin(t)) * rr;
        let dr = 1.25 * rp();
        acc = over(acc, pale, (1.0 - n) * 0.85 * a * cover(length(q) - (dr + 0.8 * rp())));
        acc = over(acc, hue, a * cover(length(q) - dr));
    }
    if (i.mqk.z > 0.5) {
        let q = i.local - vec2<f32>(0.92, -0.92) * r;
        let br = 0.36 * r;
        acc = over(acc, vec3<f32>(0.18, 0.13, 0.04), (1.0 - n) * 0.6 * cover(length(q) - (br + 0.5 * rp())));
        acc = over(acc, vec3<f32>(0.788, 0.635, 0.243), cover(length(q) - br));
        let h = q - vec2<f32>(-0.3, -0.3) * br;
        acc = over(acc, vec3<f32>(1.0, 0.95, 0.8), 0.8 * exp(-dot(h, h) / (0.08 * br * br)));
    }
    return acc;
}
