// Shared by every drawing shader (look/): the view's geometry and Terra's helpers. Each pass's WGSL
// is the View struct (gpu/view.rs), this, then its own text.

const PI: f32 = 3.14159265;

struct FsIn { @builtin(position) position: vec4f };

@vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4f {
    let p = vec2f(select(-1.0, 3.0, id == 1u), select(-1.0, 3.0, id == 2u));
    return vec4f(p, 0.0, 1.0);
}

fn hash2(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash13(pin: vec3f) -> f32 {
    var p = fract(pin * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return fract((p.x + p.y) * p.z);
}

fn vnoise(p: vec2f) -> f32 {
    let i = floor(p); let f = p - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i); let b = hash2(i + vec2f(1.0, 0.0)); let c = hash2(i + vec2f(0.0, 1.0)); let d = hash2(i + vec2f(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// value noise in three dimensions: on the sphere it has no seam and no pole
fn vnoise3(p: vec3f) -> f32 {
    let i = floor(p); let f = p - i;
    let u = f * f * (3.0 - 2.0 * f);
    var n = 0.0;
    for (var dz = 0; dz <= 1; dz++) {
        for (var dy = 0; dy <= 1; dy++) {
            for (var dx = 0; dx <= 1; dx++) {
                let c = i + vec3f(f32(dx), f32(dy), f32(dz));
                let h = hash2(vec2f(hash2(c.xy) * 157.0 + c.z * 1.37, c.z * 0.71 + hash2(c.yx) * 89.0));
                n += h * select(1.0 - u.x, u.x, dx == 1) * select(1.0 - u.y, u.y, dy == 1) * select(1.0 - u.z, u.z, dz == 1);
            }
        }
    }
    return n;
}

// coverage of a line `width` pixels wide along d == 0, fw being d's change per pixel
fn stroke(d: f32, width: f32, fw: f32) -> f32 {
    return 1.0 - smoothstep(width / 2.0 - 0.5, width / 2.0 + 0.5, abs(d) / max(fw, 1e-5));
}

fn sq(x: f32) -> f32 { return x * x; }

// the pass's colour over what it has so far (premultiplied)
fn over(acc: vec4f, col: vec3f, a: f32) -> vec4f { return vec4f(col * a, a) + acc * (1.0 - a); }

// ---- the view: the point of the globe under a pixel, and where it is on the map

struct Ground {
    v: vec3f,       // the view-space point on the unit sphere (z 0 at the rim)
    g: vec3f,       // the world point
    uv: vec2f,      // on the equirectangular map
    r2: f32,        // the squared distance from the disc's centre, in radii
    lat: f32,
    lon: f32,
};

fn ground_at(U: View, pix: vec2f) -> Ground {
    let pt = pix / U.frame.z;
    let R = U.disc.z;
    let v2 = vec2f(pt.x - U.disc.x, U.disc.y - pt.y) / R;
    let r2 = dot(v2, v2);
    let z = sqrt(max(1.0 - r2, 0.0));
    let v = vec3f(v2, z);
    let g = normalize(U.rot0.xyz * v.x + U.rot1.xyz * v.y + U.rot2.xyz * v.z);
    let lat = asin(clamp(g.y, -1.0, 1.0));
    let lon = atan2(g.x, g.z);
    var out: Ground;
    out.v = v; out.g = g; out.r2 = r2; out.lat = lat; out.lon = lon;
    out.uv = vec2f(lon / (2.0 * PI) + 0.5, 0.5 - lat / PI);
    return out;
}

// the map wraps at the antimeridian: each derivative from whichever of u or u + 1/2 doesn't jump
// there, or the sampler would read the smallest mip along that meridian
struct Grad { dx: vec2f, dy: vec2f };
fn uv_grad(uv: vec2f) -> Grad {
    let uv2 = vec2f(fract(uv.x + 0.5), uv.y);
    let dx1 = dpdx(uv); let dy1 = dpdy(uv); let dx2 = dpdx(uv2); let dy2 = dpdy(uv2);
    var g: Grad;
    g.dx = vec2f(select(dx2.x, dx1.x, abs(dx1.x) < abs(dx2.x)), dx1.y);
    g.dy = vec2f(select(dy2.x, dy1.x, abs(dy1.x) < abs(dy2.x)), dy1.y);
    return g;
}
