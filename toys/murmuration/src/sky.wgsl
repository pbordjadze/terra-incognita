// The sky and the land under it, per pixel: the view's ray read against the table of the real sky
// (sky.rs), the sun's disc, a dark country of reeds and hedges at the foot, and the eye's adaptation.

struct U {
    right: vec4f,      // camera basis (world), .w: focal length in pixels
    up: vec4f,         // .w: screen width px
    fwd: vec4f,        // .w: screen height px
    sun: vec4f,        // sun direction (world, unit), .w: sun azimuth (radians, from +z toward +x)
    sun_col: vec4f,    // the sun's disc (transmitted), .w: exposure
    ground: vec4f,     // the land's colour (lit), .w: the time
    horizon: vec4f,    // the horizon's radiance toward the sun's side, .w: camera height (m)
    horizon2: vec4f,   // the horizon's radiance opposite the sun
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var table: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VOut { @builtin(position) pos: vec4f, };

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VOut {
    var p = array<vec2f, 3>(vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0));
    var o: VOut;
    o.pos = vec4f(p[i], 0.0, 1.0);
    return o;
}

const PI = 3.14159265;
const EL_MIN = -2.0;

// The view's azimuth from the sun's, 0..PI, measured directly (no wrap to go wrong).
fn sun_angle(dir: vec3f) -> f32 {
    let h = normalize(vec2f(dir.x, dir.z) + vec2f(1e-6, 0.0));
    let s = vec2f(sin(u.sun.w), cos(u.sun.w));
    return acos(clamp(dot(h, s), -1.0, 1.0));
}

fn sky(dir: vec3f) -> vec3f {
    let el = degrees(asin(clamp(dir.y, -1.0, 1.0)));
    let daz = sun_angle(dir);
    let v = sqrt(clamp((el - EL_MIN) / (90.0 - EL_MIN), 0.0, 1.0));
    return textureSampleLevel(table, samp, vec2f(daz / PI, v), 0.0).rgb;
}

fn hash(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(12.9898, 78.233))) * 43758.5453);
}

fn vnoise(x: f32) -> f32 {
    let i = floor(x);
    let f = fract(x);
    let a = hash(vec2f(i, 3.1));
    let b = hash(vec2f(i + 1.0, 3.1));
    return mix(a, b, f * f * (3.0 - 2.0 * f));
}

// The skyline's height (degrees) at an azimuth (radians): hedgerows, a few trees, a far low hill.
fn skyline(az: f32) -> f32 {
    let a = az * 57.2958;
    let hill = 0.35 * vnoise(a * 0.02) + 0.25 * vnoise(a * 0.05 + 7.0);
    let hedge = 0.18 * vnoise(a * 0.6) + 0.10 * vnoise(a * 2.3);
    let tree = 0.45 * smoothstep(0.55, 1.0, vnoise(a * 0.5 + 11.0)) + 0.3 * smoothstep(0.6, 1.0, vnoise(a * 1.7 + 3.0));
    return 0.08 + hill + hedge + tree;
}

fn tone(c: vec3f) -> vec3f {
    // The eye's adaptation to the dusk's warm light: a white balance a little toward the sun's colour.
    let x = c * u.sun_col.w * vec3f(0.84, 1.0, 1.3);
    // A filmic shoulder (Reinhard on luminance, hue kept), then sRGB's curve.
    let l = dot(x, vec3f(0.2126, 0.7152, 0.0722));
    let lt = l * (1.0 + l / 9.0) / (1.0 + l);
    var y = x * (lt / max(l, 1e-6));
    y = min(y, vec3f(1.0));
    return pow(y, vec3f(1.0 / 2.2));
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4f {
    let f = u.right.w;
    let px = vec2f(in.pos.x - 0.5 * u.up.w, 0.5 * u.fwd.w - in.pos.y);
    let dir = normalize(u.fwd.xyz * f + u.right.xyz * px.x + u.up.xyz * px.y);
    let el = degrees(asin(clamp(dir.y, -1.0, 1.0)));
    let az = atan2(dir.x, dir.z);
    var col = sky(dir);
    // The sun's disc and the glare round it.
    let cs = dot(dir, u.sun.xyz);
    let disc = smoothstep(0.999962, 0.999972, cs);
    col += u.sun_col.rgb * disc * 400.0;
    // The land: the skyline's silhouette, then the fields toward the eye, hazed with distance.
    let hor = sky(normalize(vec3f(dir.x, 0.007, dir.z)));
    let line = skyline(az);
    let pxdeg = degrees(1.0 / f);
    let land = 1.0 - smoothstep(line - pxdeg, line + pxdeg, el);
    if (land > 0.0) {
        var g: vec3f;
        if (el > 0.0) {
            // The skyline, a few kilometres off: the land's colour hazed most of the way to the sky.
            g = mix(u.ground.rgb, hor, 0.42);
        } else {
            let depression = max(-dir.y, 1e-4);
            let dist = u.horizon.w / depression;
            let haze = 1.0 - exp(-dist / 30000.0);
            g = mix(u.ground.rgb, hor, haze);
        }
        col = mix(col, g, land);
    }
    var out = tone(col);
    // A dither below one step of 8 bits, so the sky's long gradients never band.
    let d = hash(in.pos.xy + vec2f(u.ground.w * 7.0, 0.0)) - hash(in.pos.xy * 1.37 + 3.0);
    out += vec3f(d / 255.0);
    return vec4f(out, 1.0);
}

// The reed bed the watcher stands in: stems and plumes against the sky, in three layers (near,
// mid, far), swaying in a light wind. Placed by the view's azimuth and elevation, so they stay put
// in the world as the eye turns; drawn over the birds (they are nearer), and gone while flying.
fn h2(a: f32, b: f32) -> f32 {
    return fract(sin(a * 127.1 + b * 311.7) * 43758.5453);
}

fn reeds(azd: f32, el: f32, pxdeg: f32, t: f32) -> vec2f {
    var cov = 0.0;
    var depth = 0.0;
    for (var layer = 0; layer < 3; layer++) {
        let l = f32(layer);
        let dens = mix(1.6, 5.0, l / 2.0);           // stems a degree
        let top_max = mix(9.0, 1.2, l / 2.0);       // the tallest tops, degrees above level
        let w = mix(0.11, 0.035, l / 2.0);          // stem width, degrees
        let ps = mix(1.0, 0.3, l / 2.0);            // plume size
        let cell = floor(azd * dens);
        for (var k = -1; k <= 1; k++) {
            let c = cell + f32(k);
            let r1 = h2(c, l + 1.0);
            let r2 = h2(c + 17.0, l + 3.0);
            let r3 = h2(c + 41.0, l + 7.0);
            let base = (c + r1) / dens;
            let top = top_max * (0.3 + 0.7 * r2 * r2) - 1.0;
            let sway = (0.25 + 0.5 * r3) * ps * sin(t * (0.7 + 0.5 * r3) + r3 * 6.283 + base * 0.08);
            // The near layer rises from the foot of the glass; the farther ones stand on the horizon.
            let bottom = select(-0.6 - 0.4 * l, -25.0, layer == 0);
            let s = clamp((el - bottom) / (top - bottom), 0.0, 1.0);
            let ac = base + sway * s * s;
            var c_here = 0.0;
            if (el < top && el > bottom) {
                let d = abs(azd - ac);
                c_here = 1.0 - smoothstep(0.5 * w - 0.5 * pxdeg, 0.5 * w + 0.5 * pxdeg, d);
            }
            // The plume: a drooping, feathered head leaning with the wind.
            let pc = vec2f(base + sway * 1.25, top + 0.75 * ps);
            let q = vec2f(azd - pc.x, el - pc.y);
            let lean = sway * 0.6;
            let qq = vec2f(q.x - lean * q.y, q.y);
            let pw = 0.2 * ps + 0.6 * pxdeg;
            let ph = 0.85 * ps;
            let rr = length(vec2f(qq.x / pw, qq.y / ph));
            let feather = 0.75 + 0.25 * h2(floor(el / max(pxdeg * 3.0, 0.01)), c);
            let pcov = (1.0 - smoothstep(0.75, 1.0, rr)) * feather;
            let here = max(c_here, pcov);
            if (here > cov) {
                cov = here;
                depth = l / 2.0;
            }
        }
    }
    return vec2f(cov, depth);
}

@fragment
fn fs_reeds(in: VOut) -> @location(0) vec4f {
    let strength = u.horizon2.w;
    if (strength <= 0.001) { discard; }
    let f = u.right.w;
    let px = vec2f(in.pos.x - 0.5 * u.up.w, 0.5 * u.fwd.w - in.pos.y);
    let dir = normalize(u.fwd.xyz * f + u.right.xyz * px.x + u.up.xyz * px.y);
    let el = degrees(asin(clamp(dir.y, -1.0, 1.0)));
    if (el > 12.0) { discard; }
    let azd = degrees(atan2(dir.x, dir.z));
    let pxdeg = degrees(1.0 / f);
    let r = reeds(azd, el, pxdeg, u.ground.w);
    if (r.x <= 0.001) { discard; }
    let hor = sky(normalize(vec3f(dir.x, 0.007, dir.z)));
    // Near reeds nearly black; far ones hazed toward the sky behind them.
    let col = tone(mix(u.ground.rgb * 0.5, hor * 0.5, r.y * 0.35 * smoothstep(-0.5, 0.5, el)));
    let a = r.x * strength;
    return vec4f(col * a, a);
}
