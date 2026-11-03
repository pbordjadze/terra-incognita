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
