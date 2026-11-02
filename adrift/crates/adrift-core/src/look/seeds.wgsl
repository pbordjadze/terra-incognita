// The things on the wind (R21): a down seed, its pappus seen from above, one instanced quad each, in
// reading points like the floats (its radius 5 to 7.5 points between zoom 1 and 3, then held). It is
// gossamer, not glass: fine filaments of its hue fanned round a small dark seed, turning slowly, over
// its own soft shadow on the ground, set off down and right (the sun is upper left, as on the glass)
// by how high it rides, so it reads as carried by the air, not floating on the water. Coming down,
// the shadow meets it and the fan folds a little; then it fades to a mark: a small seed on land, a
// ring spreading once on the sea. By night its filaments carry their own light and it casts none.
// Premultiplied alpha, over the sea.

struct SeedIn {
    @location(0) g: vec3<f32>,
    @location(1) beat: f32,         // 0..1 while a finger rests and the ring gathers into a seed; 1 a seed
    @location(2) hue: vec3<f32>,
    @location(3) night: f32,
    @location(4) lift: f32,         // 1 aloft, 0 down: the shadow's offset
    @location(5) down: f32,         // 0 flying, 1 down on land, 2 down on the sea
    @location(6) after: f32,        // 0..1 through the ending: to a mark
    @location(7) spin: f32,         // its own phase
    @location(8) lit: f32,
    @location(9) held: f32,
    @location(10) head: f32,        // 1 in a clip: the line's head
    @location(11) pad: f32,
};

struct SeedOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) r: f32,
    @location(2) @interpolate(flat) hue: vec3<f32>,
    @location(3) @interpolate(flat) a: vec4<f32>,   // beat, night, lift, down
    @location(4) @interpolate(flat) b: vec4<f32>,   // after, spin, lit, held
    @location(5) @interpolate(flat) c: vec2<f32>,   // head, facing
};

fn seed_radius() -> f32 {
    let t = clamp((U.disc.w - 1.0) / 2.0, 0.0, 1.0);
    return (5.0 + 2.5 * t) * rp();
}

// The shadow's offset in pixels for a lift (down and right, from the sun upper left).
fn shadow_off(lift: f32, r: f32) -> vec2<f32> {
    return lift * vec2<f32>(0.85, 1.15) * r;
}

@vertex
fn vs(@builtin(vertex_index) vi: u32, s: SeedIn) -> SeedOut {
    let p = place(s.g);
    let r = seed_radius() * (1.0 + 0.3 * s.head);
    let reach = max(max(18.0 * rp() * (1.0 - s.beat) + r, 3.0 * r), length(shadow_off(s.lift, r)) + 1.6 * r);
    let c = corner(vi % 6u);
    var o: SeedOut;
    o.local = c * reach;
    o.pos = clip(p.xy + o.local);
    if (p.z < 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.r = r;
    o.hue = s.hue;
    o.a = vec4<f32>(s.beat, s.night, s.lift, s.down);
    o.b = vec4<f32>(s.after, s.spin, s.lit, s.held);
    o.c = vec2<f32>(s.head, p.z);
    return o;
}

const TAU_S: f32 = 6.2831853;
const FILAMENTS: f32 = 17.0;

// The pappus at `R` pixels: its glow, its gossamer, its filaments with their tips, its seed; `k` its
// opacity. `ph` the fan's turn.
fn pappus(local: vec2<f32>, R: f32, hue: vec3<f32>, n: f32, ph: f32, lit: f32, k: f32) -> vec4<f32> {
    var acc = vec4<f32>(0.0);
    if (k <= 0.0 || R <= 0.0) { return acc; }
    let d = length(local);
    let w = 0.2 * rp();
    // its light in the air: a soft glow of its hue, brighter by night and when just put
    acc = over(acc, hue, k * (mix(0.16, 0.32, n) + 0.22 * lit) * exp(-d * d / (2.6 * R * R)));
    // the gossamer: a soft puff of light, fullest toward the tips where the down gathers
    let veil = mix(mix(hue, vec3<f32>(1.0), 0.72), mix(hue, vec3<f32>(1.0), 0.30), n);
    acc = over(acc, veil, k * mix(0.30, 0.24, n) * (1.0 - smoothstep(0.75 * R, 1.08 * R, d)) * mix(0.55, 1.0, smoothstep(0.2 * R, 0.85 * R, d)));
    // the nearest filament: straight from the seed, its length its own
    let ang = atan2(local.y, local.x) - ph;
    let j = round(ang * FILAMENTS / TAU_S);
    let rel = ang - j * TAU_S / FILAMENTS;
    let along = d * cos(rel);
    let perp = abs(d * sin(rel));
    let jj = j - FILAMENTS * floor(j / FILAMENTS);
    let flen = R * (0.84 + 0.16 * hash2(vec2<f32>(jj, 7.0)));
    let on = step(0.18 * R, along) * (1.0 - smoothstep(flen - 0.5, flen + 0.5, along));
    let fil = mix(mix(hue, vec3<f32>(1.0), 0.50), mix(hue, vec3<f32>(1.0), 0.40), n);
    // (each filament brightest at its tip, as down catches the light; by day on a faint hair of its hue
    // darkened, so it reads on the light land as on the sea)
    let toward = mix(0.45, 1.0, smoothstep(0.2 * R, flen, along));
    acc = over(acc, hue * 0.40, k * (1.0 - n) * 0.30 * on * cover(perp - (w + 0.3 * U.frame.z)));
    acc = over(acc, fil, k * mix(0.85, 0.95, n) * toward * on * cover(perp - w));
    // the tips: each a soft point of light
    let tip = local - vec2<f32>(cos(ph + j * TAU_S / FILAMENTS), sin(ph + j * TAU_S / FILAMENTS)) * flen;
    acc = over(acc, mix(hue, vec3<f32>(1.0), mix(0.80, 0.50, n)), k * 0.95 * exp(-dot(tip, tip) / (0.22 * rp() * rp())));
    // the seed itself, small and dark by day, its hue's ember by night
    let sr = 0.16 * R;
    acc = over(acc, mix(hue * 0.30, hue * 0.9, n), k * cover(d - (sr + 0.4 * U.frame.z)));
    acc = over(acc, mix(hue * 0.75, mix(hue, vec3<f32>(1.0), 0.45), n), k * cover(d - sr));
    return acc;
}

@fragment
fn fs(i: SeedOut) -> @location(0) vec4<f32> {
    let rim_fade = smoothstep(0.02, 0.32, i.c.y);
    let r = i.r;
    let beat = i.a.x;
    let n = i.a.y;
    let lift = i.a.z;
    let down = i.a.w;
    let after = i.b.x;
    let lit = i.b.z;
    let held = i.b.w;
    let head = i.c.x;
    let d = length(i.local);
    // the fan turns slowly (a full turn in about a minute), each seed from its own phase
    let ph = i.b.y + U.time.x * 0.1;
    var acc = vec4<f32>(0.0);

    if (beat < 1.0) {
        // the ring gathering: drawing in from 16 points as a float's does, and the fan opening in it
        let t = beat;
        let rr = mix(16.0 * rp(), r, t * t);
        acc = over(acc, vec3<f32>(1.0), mix(0.9, 0.7, n) * (1.0 - t) * cover(abs(d - rr) - 0.4 * rp()));
        let k = smoothstep(0.35, 0.9, t);
        let pp = pappus(i.local, r * k, i.hue, n, ph, 0.0, k);
        return (pp + acc * (1.0 - pp.a)) * rim_fade;
    }

    // its shadow on the ground, offset by how high it rides; gone by night; it becomes the mark's
    let so = i.local - shadow_off(lift, r);
    let shadow_a = (1.0 - n) * mix(0.30, 0.20, lift) * (1.0 - 0.6 * after);
    acc = over(acc, vec3<f32>(0.02, 0.05, 0.07), shadow_a * exp(-dot(so, so) / (0.55 * r * r)));
    // a clip's head: a soft breathing glow of its hue round it
    let breath = 0.85 + 0.15 * sin(U.time.x * 1.7);
    acc = over(acc, i.hue, head * 0.26 * breath * exp(-d * d / (7.0 * r * r)));

    // the ending: on the sea a ring spreads once from where it touched and fades
    if (down > 1.5) {
        let e = 1.0 - pow(1.0 - clamp(after, 0.0, 1.0), 2.0);
        let rr = mix(0.7 * r, 2.8 * r, e);
        let wr = mix(0.9, 0.4, e) * rp();
        let ra = 0.75 * pow(1.0 - clamp(after, 0.0, 1.0), 1.4) * step(0.001, after);
        acc = over(acc, mix(mix(i.hue, vec3<f32>(1.0), 0.5), i.hue, n), ra * cover(abs(d - rr) - wr));
    }
    // the mark it leaves: a small seed of its hue, darker on land, lighter afloat
    if (down > 0.5) {
        let m = smoothstep(0.2, 0.9, after);
        let mr = select(0.20, 0.18, down > 1.5) * r;
        let ink = select(mix(i.hue * 0.8, i.hue, n), mix(i.hue, vec3<f32>(1.0), mix(0.3, 0.1, n)), down > 1.5);
        acc = over(acc, i.hue * 0.3, m * (1.0 - n) * 0.7 * cover(d - (mr + 0.5 * U.frame.z)));
        acc = over(acc, ink, m * 0.9 * cover(d - mr));
    }

    // the pappus: folding a little as it settles, fading as the ending runs
    let R = r * mix(0.8, 1.0, lift);
    let k = 1.0 - smoothstep(0.0, 0.85, after);
    let pp = pappus(i.local, R, i.hue, n, ph, lit, k);
    acc = pp + acc * (1.0 - pp.a);
    // held, chosen or followed: a thin ring out at 1.55 r of its hue, as the lamps have
    acc = over(acc, mix(vec3<f32>(1.0), i.hue, 0.65), 0.6 * held * k * cover(abs(d - 1.55 * r) - 0.35 * rp()));
    return acc * rim_fade;
}
