// The floats (and the ring that thickens into one): one instanced quad a float, drawn on the screen
// in reading points from the palette (palette.json; PALETTE.md says why). A float is a marker: its
// radius goes from 4.5 to 7 points between zoom 1 and 3 and is then held; it never grows with the
// water; each lamp is a light in glass of its own by its id (R25, lamp_kind at the foot). Under a resting finger (beat < 1) the ring draws in and the glass forms, without a lamp; the
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
    @location(12) id: f32,         // the lamp's id (R25): its glass, its flame, its rhythm
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
    @location(12) @interpolate(flat) id: f32,
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
    let reach = max(max(18.0 * rp() * (1.0 - f.beat) + r, mix(2.6, 3.6, f.head) * r), select(0.0, 2.3 * r + 2.0 * rp(), f.mode > 0.5));
    // (room for the kept star at the shoulder, R22)
    let reach_k = max(reach, select(0.0, 1.3 * r + max(0.85 * r, 5.0 * rp()) + 2.0 * rp(), f.kept > 0.5));
    let c = corner(vi % 6u);
    var o: FloatOut;
    o.local = c * reach_k;
    o.pos = clip(p.xy + o.local);
    // (past the limb gone; toward it fading by its facing, as the maybes do, never clipped at a line)
    if (p.z < 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.speck = f.speck;
    o.head = f.head;
    o.id = f.id;
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
    // (R22) a lamp chosen as its cloud stays its glass, drawn over its cloud (which thins under it):
    // as one maybe among its own it was lost in them, and the eye lost which lamp the cloud is
    let speck = select(i.speck, 0.0, i.mqk.x > 1.5);
    if (speck > 0.0) { glass = glass * (1.0 - speck) + as_maybe(i) * speck; }
    // (R21) a lamp not chosen while another is steps back: fainter, so the chosen read at a glance;
    // (R22) and it has no rim then (glass_float), so only a chosen lamp is ringed
    // (a kept lamp steps back less: it is yours)
    glass = glass * (1.0 - 0.45 * i.mqk.y * (1.0 - 0.5 * i.mqk.z));
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
    let rim = 0.95 * rp();
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

    // (R25) the lamp: a light in glass of its own (lamp_kind), its light breathing on the water
    let hue = lamp_of(i.hue, n);
    let l = lamp_kind(i.id);
    let fl = flicker(l);
    let br = lamp_breath(l);
    let breath = 0.85 + 0.15 * sin(U.time.x * 1.7);
    let live = 0.82 + 0.12 * br + 0.06 * fl;
    let p = i.local / r;
    // (R21, Engineering) the glows fade to nothing inside the quad's least reach: cut off at the
    // quad's edge they showed its square on the night sea
    let edge = mix(2.6, 3.6, i.head) * r;
    let win = 1.0 - smoothstep(0.7 * edge, 0.98 * edge, d);
    // its light on the water: a pool round it, breathing, and below it the light's broken path on the
    // swell, a few bright slivers that shimmer as the water moves (stronger by night)
    acc = over(acc, hue, win * i.head * 0.30 * breath * exp(-d * d / (2.6 * 2.6 * r * r)));
    acc = over(acc, hue, win * lit_a(n) * i.lit * exp(-d * d / (1.9 * 1.9 * r * r)));
    acc = over(acc, hue, win * lamp_glow_a(n) * live * exp(-d * d / (1.35 * 1.35 * r * r)));
    let below = p.y - 0.85;
    if (below > 0.0) {
        let row = i.local.y / max(0.30 * r, 1.6 * rp());
        let ripple = sin(row * 6.2832 - U.time.x * 2.4 * l.rate + l.phase + 1.3 * sin(p.x * 3.0 + U.time.x * 1.1));
        let sliver = smoothstep(0.15, 0.85, ripple);
        let wide = 0.32 + 0.20 * below;
        let across = exp(-sq((p.x - 0.08 * sin(row * 1.7 + l.phase)) / wide));
        let fall = smoothstep(0.0, 0.25, below) * exp(-below / 0.85);
        acc = over(acc, mix(mix(hue, vec3<f32>(1.0), 0.55), mix(hue, lamp_core(n), 0.35), n), win * mix(0.90, 0.80, n) * live * sliver * across * fall);
    }
    // (day) a faint shadow where it sits in the water, so it floats rather than lies on the screen
    let sd_sh = glass_sd(p - vec2<f32>(0.10, 0.16), l.shape) * r;
    acc = over(acc, vec3<f32>(0.03, 0.10, 0.16), (1.0 - n) * 0.22 * (1.0 - i.mqk.y) * (1.0 - smoothstep(-0.2 * r, 0.25 * r, sd_sh)));
    // a cool hue's pale ring round the glass (R21), so a blue or purple lamp stands off the sea's blue
    let sdp = glass_sd(p, l.shape) * r;
    acc = over(acc, vec3<f32>(0.96, 0.98, 1.0), coolness(i.hue) * (1.0 - n) * 0.85 * step(i.mqk.x, 0.5) * (1.0 - i.mqk.y) * cover(abs(sdp - 0.9 * rp()) - 0.6 * rp()));
    // the lantern's cap and bail, the dome's finial: dark metal by day, the rim's pale by night
    let metal = mix(vec3<f32>(0.13, 0.17, 0.20), vec3<f32>(0.62, 0.72, 0.76), n);
    if (l.shape == 2) {
        let cq = abs(p - vec2<f32>(0.0, -0.98)) - vec2<f32>(0.42 - 0.08, 0.13 - 0.08);
        let cap = (length(max(cq, vec2<f32>(0.0))) + min(max(cq.x, cq.y), 0.0) - 0.08) * r;
        acc = over(acc, metal, (1.0 - 0.6 * i.mqk.y) * cover(cap));
        let bq = p - vec2<f32>(0.0, -1.10);
        let bail = (abs(length(bq) - 0.24) * r) - 0.55 * rp();
        acc = over(acc, metal, (1.0 - 0.6 * i.mqk.y) * step(bq.y, 0.02) * cover(bail));
    }
    if (l.shape == 3) {
        let fq = p - vec2<f32>(0.0, -1.24);
        acc = over(acc, metal, (1.0 - 0.6 * i.mqk.y) * cover(length(fq) * r - max(0.11 * r, 1.0 * rp())));
    }
    // the glass, lit from within: its body tinted by its light, brightest round the flame
    let gb = glass_body(n);
    let fc = p - vec2<f32>(0.0, 0.15);
    // (small, far out, the hue carries the glass: it is how a lamp is told from its neighbour)
    let small = 1.0 - smoothstep(15.0, 24.0, r);
    let inner = exp(-dot(fc, fc) / mix(0.75, 0.95, small)) * live;
    let clear = mix(mix(vec3<f32>(0.93, 0.97, 0.97), hue, mix(0.58, 0.68, small)), mix(gb.rgb, hue, 0.85), n);
    let tint = mix(clear, mix(hue, lamp_core(n), 0.30), inner);
    let body_a = mix(cover(sdp), 1.0 - smoothstep(-0.45 * r, 0.05 * r, sdp), i.mqk.y);
    acc = over(acc, tint, mix(0.85, 0.80, n) * body_a);
    // what the glass shows of itself: the rope round a float, a cut float's table, a lantern's frame
    let rope = mix(vec3<f32>(0.42, 0.30, 0.16), vec3<f32>(0.55, 0.45, 0.32), n);
    let inside = cover(sdp + 0.6 * rp());
    if (l.shape == 0) {
        // a rope round its waist, an equator seen from a little above, tied in a knot at its crown
        let ex = p.x;
        let eq = p.y - (l.tilt * 0.5 * ex + 0.34 * sqrt(max(1.0 - ex * ex, 0.0)) + 0.05);
        let rw = max(0.045 * r, 0.5 * rp());
        acc = over(acc, rope, 0.75 * inside * (1.0 - 0.6 * i.mqk.y) * cover(abs(eq) * r - rw));
        let kq = p - vec2<f32>(0.0, -1.02);
        acc = over(acc, rope, (1.0 - 0.6 * i.mqk.y) * cover(length(kq * vec2<f32>(1.0, 1.4)) * r - max(0.14 * r, 1.2 * rp())));
    }
    if (l.shape == 1) {
        acc = over(acc, mix(hue, vec3<f32>(1.0), 0.6), 0.45 * inside * cover(abs(octa(p, 0.55)) * r - 0.45 * rp()));
    }
    if (l.shape == 2) {
        let bars = min(abs(p.x - 0.40), abs(p.x + 0.40)) * r - 0.5 * rp();
        acc = over(acc, metal, 0.55 * inside * (1.0 - 0.6 * i.mqk.y) * step(p.y, 0.62) * cover(bars));
        // and its foot, a dark band
        acc = over(acc, metal, inside * (1.0 - 0.6 * i.mqk.y) * cover((p.y - 0.80) * -r - 0.0));
    }
    if (l.shape == 3) {
        let rib = abs(p.y - 0.08) * r - 0.45 * rp();
        acc = over(acc, mix(hue, vec3<f32>(1.0), 0.5), 0.40 * inside * cover(rib));
    }
    // the flame, its heart, its tip leaning as it sways
    let soft = clamp(0.9 * rp() / (l.flame_w * r), 0.12, 0.45);
    let fm = flame(p, l, fl, soft);
    acc = over(acc, hue, fm.x * live);
    acc = over(acc, mix(lamp_core(n), vec3<f32>(1.0), 0.4), fm.y);
    // the rim, dashed where the sea's current is the sheet's guess and no drifter measured it (Truth's
    // D3: nothing claims the current where no drifter went), and the sun's highlight upper left
    var dash = 1.0;
    if (i.measured < 0.5) {
        let ang = atan2(i.local.y, i.local.x);
        dash = step(0.0, sin(ang * 8.0));
    }
    // (R22) a lamp stepped back while another is chosen has no dark rim: every rim read as a ring,
    // and a ring is the sign of chosen; it is a lamp in soft glass, its edge the glass's own fade
    let rimmed = 1.0 - i.mqk.y;
    // (by night the rim takes a little of its light, so the lamp is its colour, not a pale ring)
    acc = over(acc, mix(glass_rim(n), hue, 0.35 * n), rimmed * dash * cover(abs(sdp + rim * 0.5) - rim * 0.5));
    let h = i.local - vec2<f32>(-0.36 * r, -0.42 * r);
    let hs = select(vec2<f32>(1.2, 2.4), vec2<f32>(2.6, 0.9), l.shape == 2);
    acc = over(acc, vec3<f32>(1.0), glass_light(n) * 0.85 * inside * exp(-(sq(h.x * hs.x) + sq(h.y * hs.y)) / (0.16 * r * r)));
    // and the glint of a clip's head (R20): a bright point on the glass with a small four-pointed
    // sparkle, and a faint second light low right where the water lights the glass
    let gp = i.local - vec2<f32>(-0.34 * r, -0.40 * r);
    let star = exp(-abs(gp.x) / (0.05 * r)) * exp(-abs(gp.y) / (0.45 * r)) + exp(-abs(gp.y) / (0.05 * r)) * exp(-abs(gp.x) / (0.45 * r));
    acc = over(acc, vec3<f32>(1.0), i.head * clamp(0.9 * exp(-dot(gp, gp) / (0.012 * r * r)) + 0.55 * star * (0.8 + 0.2 * breath), 0.0, 1.0));
    let low = i.local - vec2<f32>(0.40 * r, 0.42 * r);
    acc = over(acc, mix(vec3<f32>(1.0), hue, 0.4), i.head * 0.35 * inside * exp(-dot(low, low) / (0.03 * r * r)));
    // held: a thin ring out at 1.55 r, the maybes' own
    // (in its lamp's colour, R8: a lamp chosen as itself is ringed by its own light)
    acc = over(acc, mix(glass_rim(n), i.hue, 0.65), 0.6 * i.held * step(i.mqk.x, 0.5) * cover(abs(d - 1.55 * r) - 0.35 * rp()));
    return acc;
}

// How a lamp is chosen, at a glance (R21; R23 one ring for both): chosen, as itself or as its cloud, a
// solid ring of its hue close round the glass (its cloud is the scatter of maybes behind it); kept, a small gold bead at its shoulder (a jewel, not a ring, so it never reads as chosen).
// Each on a pale rim by day so a cool hue holds against the sea. `held` is the marks' strength.
fn chosen_marks(i: FloatOut) -> vec4<f32> {
    let n = i.night;
    let d = length(i.local);
    let r = i.r;
    let a = clamp(i.held, 0.0, 1.0);
    let hue = lamp_of(i.hue, n);
    let pale = vec3<f32>(0.97, 0.985, 1.0);
    var acc = vec4<f32>(0.0);
    if (i.mqk.x > 0.5) {
        // (R23) one sign for chosen: as itself or as its cloud, the same light ring of its hue close
        // round the glass (a cloud's dotted halo was a second grammar and read as noise among its
        // maybes); a lamp chosen as its cloud is told only by the scatter of maybes behind it
        let rr = 1.7 * r;
        let w = 0.95 * rp();
        acc = over(acc, pale, (1.0 - n) * 0.8 * a * cover(abs(d - rr) - (w + 0.9 * rp())));
        acc = over(acc, hue, a * cover(abs(d - rr) - w));
    }
    if (i.mqk.z > 0.5) {
        // (R22) kept: a gold star at the shoulder, large enough to read at arm's length (a bead was a
        // dot among dots); a star is no ring and no dot, so it never reads as chosen. Four points, one
        // up, on a dark edge by day and its own soft light by night
        let c = vec2<f32>(0.9, -0.9) * r;
        let q = i.local - c;
        let sr = max(0.85 * r, 5.0 * rp());
        let sd = star4(q, sr);
        acc = over(acc, vec3<f32>(0.20, 0.14, 0.03), (1.0 - n) * 0.75 * cover(sd - 1.1 * rp()));
        acc = over(acc, vec3<f32>(0.98, 0.80, 0.30), n * 0.35 * exp(-dot(q, q) / (0.5 * sr * sr)));
        acc = over(acc, vec3<f32>(0.93, 0.74, 0.22), cover(sd));
        let h = q - vec2<f32>(-0.12, -0.12) * sr;
        acc = over(acc, vec3<f32>(1.0, 0.97, 0.85), 0.85 * exp(-dot(h, h) / (0.03 * sr * sr)));
    }
    return acc;
}

// A four-pointed star's signed distance in pixels (approximate): its points `sr` from the centre on
// the axes, its waist a third of that, the sides drawn in a little so it reads as a star, not a diamond.
fn star4(q: vec2<f32>, sr: f32) -> f32 {
    let a = abs(q);
    // the boundary between two neighbouring points, in polar form: a curve from (sr, 0) to (0, sr)
    // through the waist; r(theta) = sr * w / (|cos|^k + |sin|^k)^(1/k) style astroid
    let ang = atan2(a.y, a.x);
    let c = cos(ang); let s = sin(ang);
    let k = 0.62;
    let rb = sr / pow(pow(c, k) + pow(s, k), 1.0 / k);
    return length(q) - rb;
}

// ---- R25: each lamp a thing of its own, all of one family: a light in glass on the water. The id
// picks the glass (a round float tied with a rope, a cut float, a lantern with its cap and bail, an
// onion-domed lamp), the flame's height and width, how it flickers and sways, and the slow breath of
// its light, so two lamps side by side are two individuals and breathe out of step.
struct Lamp {
    shape: i32,
    flame_w: f32,
    flame_h: f32,
    rate: f32,
    phase: f32,
    period: f32,
    tilt: f32,
    pad: f32,
};

fn hid(id: f32, k: f32) -> f32 { return fract(sin(id * 12.9898 + k * 78.233) * 43758.5453); }

fn lamp_kind(id: f32) -> Lamp {
    var l: Lamp;
    let k = max(i32(round(id)), 1) - 1;
    // (neighbours by id, and ids four apart, never share a glass)
    let shapes = array<i32, 8>(0, 1, 2, 3, 1, 0, 3, 2);
    l.shape = shapes[k % 8];
    l.flame_w = mix(0.30, 0.40, hid(id, 1.0));
    l.flame_h = mix(0.85, 1.10, hid(id, 2.0));
    l.rate = mix(0.75, 1.35, hid(id, 3.0));
    l.phase = 6.2832 * hid(id, 4.0);
    l.period = mix(2.6, 4.8, hid(id, 5.0));
    l.tilt = mix(-0.30, 0.30, hid(id, 6.0));
    l.pad = 0.0;
    return l;
}

// The glass's signed distance in the float's radii (p in radii, y down), by its shape.
fn octa(p: vec2<f32>, s: f32) -> f32 {
    let q = abs(p);
    return max(max(q.x, q.y), (q.x + q.y) * 0.7071) - s;
}

fn glass_sd(p: vec2<f32>, shape: i32) -> f32 {
    if (shape == 1) {
        return octa(p, 0.97);
    }
    if (shape == 2) {
        // a lantern: a rounded box a little taller than wide
        let q = abs(p) - vec2<f32>(0.74 - 0.30, 0.90 - 0.30);
        return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - 0.30;
    }
    if (shape == 3) {
        // an onion dome: round below, drawn up to a point
        let yc = -p.y + 0.10;
        let t = max(yc, 0.0) * 0.74;
        let x = p.x * (1.0 + 2.6 * t * t);
        return (length(vec2<f32>(x, select(yc, t, yc > 0.0))) - 0.94) * 0.85;
    }
    return length(p) - 1.0;
}

// The flame's breath and flicker (-1..1, slow, a few incommensurate sines: alive, never noisy).
fn flicker(l: Lamp) -> f32 {
    let t = U.time.x * l.rate;
    return 0.5 * sin(t * 5.3 + l.phase) + 0.3 * sin(t * 8.9 + 2.0 * l.phase) + 0.2 * sin(t * 14.1 + 0.5 * l.phase);
}

fn lamp_breath(l: Lamp) -> f32 {
    return 0.5 + 0.5 * sin(U.time.x * 6.2832 / l.period + l.phase);
}

// The flame: 1 inside, soft at its edge; `core` its white heart. p in radii, y down; its base low in
// the glass, its tip leaning as it sways.
fn flame(p: vec2<f32>, l: Lamp, f: f32, soft: f32) -> vec2<f32> {
    let w = l.flame_w * (1.0 - 0.06 * f);
    let h = l.flame_h * (1.0 + 0.10 * f);
    let base = select(0.50, 0.58, l.shape == 2);
    let v = base - p.y;                 // height above the flame's foot, radii
    let u = clamp((v - w) / max(h - w, 0.01), 0.0, 1.0);
    let sway = (0.06 * sin(U.time.x * l.rate * 3.1 + l.phase) + 0.03 * f) * u * u;
    let x = (p.x - sway) / (w * max(1.0 - 0.80 * u, 0.05));
    let y = (v - w) / select(w, h - w, v > w);
    let m = length(vec2<f32>(x, y));
    let body = 1.0 - smoothstep(1.0 - soft, 1.0 + soft, m);
    let yc = (v - 0.8 * w) / select(w, 0.7 * (h - w), v > 0.8 * w);
    let core = 1.0 - smoothstep(0.25, 0.75, length(vec2<f32>(x * 1.6, yc * 1.25)));
    return vec2<f32>(body, core);
}
