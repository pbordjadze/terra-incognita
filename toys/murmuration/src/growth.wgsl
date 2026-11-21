// The Growth look's second pass, appended to sky.wgsl (its uniform, table and sampler): the skin
// where the summed field (growth_density.wgsl) crosses the threshold, shaded as Kawaguchi's 1986
// paper says Growth I and II were: each part's own colour, a highlight, and the colour of a
// texture-mapped ball reflected in the surface. Here that ball is the real dusk (sky.rs's table),
// painted on it in bands of elevation, so the forms carry the sunset in stripes. On black.

@group(1) @binding(0) var dens: texture_2d<f32>;
// The near birds' field: each bird summed within itself, birds kept apart from each other (max).
@group(1) @binding(1) var solid: texture_2d<f32>;

const T = 0.2;

// The skin's height above the screen (px): a dome over each part, as a ball's would be.
// The field between its texels, weighted exactly (a sampler's filter weights are too coarse for
// the slopes taken from it: they draw rings on the skins).
fn field_of(t: texture_2d<f32>, uv: vec2f) -> vec4f {
    let ts = vec2f(textureDimensions(t));
    let p = uv * ts - 0.5;
    let i = vec2i(floor(p));
    let f = p - floor(p);
    let m = vec2i(ts) - 1;
    let a = textureLoad(t, clamp(i, vec2i(0), m), 0);
    let b = textureLoad(t, clamp(i + vec2i(1, 0), vec2i(0), m), 0);
    let c = textureLoad(t, clamp(i + vec2i(0, 1), vec2i(0), m), 0);
    let d = textureLoad(t, clamp(i + vec2i(1, 1), vec2i(0), m), 0);
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// The two fields as one: wherever the near birds' is the stronger, theirs.
fn field(uv: vec2f) -> vec4f {
    let a = field_of(dens, uv);
    let b = field_of(solid, uv);
    return select(a, b, b.x > a.x);
}

fn height(uv: vec2f) -> f32 {
    let d = field(uv);
    let k = clamp((d.x - T) / (1.0 - T), 0.0, 12.0);
    return sqrt(k) * d.y / max(d.x, 1e-4);
}

// The ball round the forms: the dusk in bands of 12 degrees, a little richer than the sky, and
// below the horizon the land's dark half.
fn ball(r: vec3f) -> vec3f {
    let el = degrees(asin(clamp(r.y, -1.0, 1.0)));
    let q = (floor(el / 12.0) + 0.5) * 12.0;
    // The ball is painted from the dusk on the sun's side, whatever way the surface faces: warm
    // low, violet high, as his striped balls were painted.
    let h = normalize(vec2f(u.sun.x, u.sun.z) + vec2f(1e-5, 0.0));
    let a = radians(max(q, 0.5));
    var c = sky(vec3f(h.x * cos(a), sin(a), h.y * cos(a)));
    if (el < 0.0) {
        c = mix(u.ground.rgb * 0.6, c * 0.08, 0.5 + 0.5 * sin(q * 0.4));
    }
    // Alternate bands lighter and darker, as the striped ball's paint was.
    c *= 0.5 + 0.5 * step(0.5, fract(q / 24.0));
    let l = dot(c, vec3f(0.2126, 0.7152, 0.0722));
    return max(mix(vec3f(l), c, 2.2), vec3f(0.0));
}

// Each part's surface attributes (Kawaguchi's rgb, diffuse, reflection, glossiness), by material:
// 0 a bird, 1 a reed's stalk, 2 a joint, 3 a seed head (the flower); and your own bird.
fn albedo(m: f32, you: f32) -> vec3f {
    let bird = vec3f(0.05, 0.07, 0.5);
    let stalk = vec3f(0.05, 0.12, 0.5);
    let joint = vec3f(0.62, 0.05, 0.3);
    let head = vec3f(0.8, 0.5, 0.08);
    var c = mix(bird, stalk, clamp(m, 0.0, 1.0));
    c = mix(c, joint, clamp(m - 1.0, 0.0, 1.0));
    c = mix(c, head, clamp(m - 2.0, 0.0, 1.0));
    return mix(c, vec3f(0.85, 0.12, 0.04), clamp(you, 0.0, 1.0));
}

// The sun: a lamp-ball at its real place, setting.
fn sun_ball(dir: vec3f) -> vec4f {
    let rad = radians(1.5);
    let cs = dot(dir, u.sun.xyz);
    let s = sqrt(max(1.0 - cs * cs, 0.0)) / sin(rad);
    let aa = fwidth(s) + 1e-4;
    let a = (1.0 - smoothstep(1.0 - aa, 1.0, s)) * step(0.0, cs);
    let el = degrees(asin(clamp(u.sun.y, -1.0, 1.0)));
    let up = smoothstep(-1.6, 0.4, el);
    let face = sqrt(max(1.0 - s * s, 0.0));
    let c = u.sun_col.rgb * (12.0 + 30.0 * face) + vec3f(4.0, 1.5, 0.3) * (1.0 - face);
    return vec4f(tone(c), a * up);
}

// Behind everything, black, but for the sun's halo and a low glow of the dusk on the horizon
// under it, in the same bands as the ball, fading away from the sun's side.
fn backdrop(dir0: vec3f) -> vec3f {
    // Below the horizon, the Levels' flooded fields hold the same glow, mirrored, in stripes that
    // close up toward the horizon as water seen at a graze does.
    let below = dir0.y < 0.0;
    let dir = vec3f(dir0.x, abs(dir0.y), dir0.z);
    let el = degrees(asin(clamp(dir.y, -1.0, 1.0)));
    // Stripes finer than a couple of pixels blur to their mean, or they would shimmer (the slope
    // is taken outside the branch, where every pixel takes it).
    let x = 0.12 / max(dir.y, 1e-3);
    let fine = smoothstep(0.15, 0.45, fwidth(x));
    var water = 1.0;
    if (below) {
        water = 0.45 * mix(step(0.45, fract(x)), 0.55, fine) * (1.0 - smoothstep(0.0, 6.0, el));
    }
    let side = max(dot(normalize(vec2f(dir.x, dir.z) + vec2f(1e-5, 0.0)), normalize(vec2f(u.sun.x, u.sun.z) + vec2f(1e-5, 0.0))), 0.0);
    let q = (floor(max(el, 0.0) / 2.0) + 0.5) * 2.0;
    let h = normalize(vec2f(u.sun.x, u.sun.z) + vec2f(1e-5, 0.0));
    let a = radians(q);
    let band = sky(vec3f(h.x * cos(a), sin(a), h.y * cos(a)));
    let glow = band * 0.35 * exp(-q / 4.0) * pow(side, 3.0) * step(0.0, el);
    let ang = degrees(acos(clamp(dot(dir, u.sun.xyz), -1.0, 1.0)));
    let up = smoothstep(-2.5, 0.5, degrees(asin(clamp(u.sun.y, -1.0, 1.0))));
    let halo = u.sun_col.rgb * 3.0 * exp(-ang / 2.5) * up * step(0.0, el);
    return tone((glow + halo) * water);
}

@fragment
fn fs_growth(in: VOut) -> @location(0) vec4f {
    let f = u.right.w;
    let wh = vec2f(u.up.w, u.fwd.w);
    let uv = in.pos.xy / wh;
    let px = vec2f(in.pos.x - 0.5 * wh.x, 0.5 * wh.y - in.pos.y);
    let dir = normalize(u.fwd.xyz * f + u.right.xyz * px.x + u.up.xyz * px.y);
    let sb = sun_ball(dir);
    let back = mix(backdrop(dir), sb.rgb, sb.a);
    let d = field(uv);
    let e = max(fwidth(d.x) * 0.75, 0.01);
    let a = smoothstep(T - e, T + e, d.x);
    if (a <= 0.0) {
        return vec4f(back, 1.0);
    }
    // The skin's normal from the slope of its height, a texel of the field either side.
    // The field is kept at half precision, so on a big skin the slope is taken over a wider span
    // (an eighth of its thickness) or its steps would show as rings.
    let ts = vec2f(textureDimensions(dens));
    let span = max(1.5, 0.12 * d.y / max(d.x, 1e-4) * ts.x / wh.x);
    let dx = vec2f(span / ts.x, 0.0);
    let dy = vec2f(0.0, span / ts.y);
    let step_px = span * wh / ts;
    let gx = (height(uv + dx) - height(uv - dx)) / (2.0 * step_px.x);
    let gy = -(height(uv + dy) - height(uv - dy)) / (2.0 * step_px.y);
    let n = normalize(-u.right.xyz * gx - u.up.xyz * gy - u.fwd.xyz);
    let m = d.z / max(d.x, 1e-4);
    let you = d.w / max(d.x, 1e-4);
    let base = albedo(m, you);
    let sun = u.sun.xyz;
    let r = reflect(dir, n);
    let zen = sky(vec3f(0.0, 1.0, 0.0));
    let zl = dot(zen, vec3f(0.2126, 0.7152, 0.0722));
    // The studio's key light, as his scenes had: above and to the left of the eye, so every form
    // carries its highlight whatever side the sun is on.
    let key = normalize(-u.fwd.xyz * 0.7 + u.up.xyz * 0.6 - u.right.xyz * 0.4);
    let lamb = max(dot(n, sun), 0.0);
    let klamb = max(dot(n, key), 0.0);
    let fres = 0.35 + 0.65 * pow(1.0 - max(dot(-dir, n), 0.0), 2.0);
    let gloss = select(30.0, 70.0, m > 2.5);
    let spec = pow(max(dot(r, key), 0.0), gloss) + 0.6 * pow(max(dot(r, sun), 0.0), gloss) * step(0.0, sun.y + 0.05);
    var lit = base * zl * (u.sun_col.rgb * lamb * 6.0 + vec3f(5.0) * klamb + 1.5);
    // Small forms (a far or edge-on bird, a few pixels thick) keep their colour and lose their
    // glints, which at that size would only sparkle; the shine is for forms big enough to hold it.
    let big = smoothstep(2.0, 9.0, d.y / max(d.x, 1e-4));
    lit += ball(r) * fres * mix(2.2, 1.0, step(2.5, m)) * mix(0.55, 1.0, big);
    lit += vec3f(1.0, 0.97, 0.9) * spec * zl * 40.0 * big;
    let col = tone(lit);
    return vec4f(mix(back, col, a), 1.0);
}
