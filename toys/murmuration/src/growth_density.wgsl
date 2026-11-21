// The Growth look's first pass: every part of the scene is a meta-ball or meta-ellipsoid (Osaka's
// density primitives: a centre, radii and a weight), and this pass adds up their fields on the
// screen. The shading pass (growth.wgsl) takes the surface where the sum crosses the threshold, so
// parts that come close melt into one skin, as in Kawaguchi's Growth films.
//
// Each primitive is drawn as a quad round its projected ellipse. Its field is (1 - r²)² inside the
// quad, times its weight, and the channels sum, in order: the field; the field times the part's
// thickness in pixels, for the normals; the field times its material; the field times "you".

struct U {
    right: vec4f,   // .w focal px
    up: vec4f,      // .w width px
    fwd: vec4f,     // .w height px
    eye: vec4f,     // the camera's place, .w: the smallest half-thickness drawn (px)
};

@group(0) @binding(0) var<uniform> u: U;

struct In {
    @location(0) c: vec4f,    // centre (world), .w: the third half-axis's length (m)
    @location(1) a: vec4f,    // a half-axis (world, m), .w: weight
    @location(2) b: vec4f,    // a second half-axis, .w: material
    @location(3) t: vec4f,    // .x you (0..1)
};

struct VOut {
    @builtin(position) pos: vec4f,
    @location(0) l: vec2f,
    @location(1) k: vec4f,    // weight, thickness px, material, you
};

// The quad reaches past the visible edge: with the threshold at a fifth of the peak, a lone
// part's surface sits at 0.74 of the quad, so the quad is that much larger than the part. A low
// threshold keeps the melting close: a bird's parts melt into one body, neighbours only when
// they nearly touch.
const REACH = 1.34;

fn project(p: vec3f) -> vec3f {
    let rel = p - u.eye.xyz;
    let z = dot(rel, u.fwd.xyz);
    let f = u.right.w;
    return vec3f(dot(rel, u.right.xyz) * f / max(z, 0.01), dot(rel, u.up.xyz) * f / max(z, 0.01), z);
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, b: In) -> VOut {
    var o: VOut;
    let s = project(b.c.xyz);
    if (s.z < 0.3 || b.a.w <= 0.0) {
        o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
        return o;
    }
    // The ellipsoid's outline on the screen: its three half-axes projected about its centre, and
    // the ellipse they span (the sum of their outer products), by its two principal axes.
    let tz = normalize(cross(b.a.xyz, b.b.xyz) + vec3f(0.0, 1e-7, 0.0)) * b.c.w;
    let ea = project(b.c.xyz + b.a.xyz).xy - s.xy;
    let eb = project(b.c.xyz + b.b.xyz).xy - s.xy;
    let et = project(b.c.xyz + tz).xy - s.xy;
    let xx = ea.x * ea.x + eb.x * eb.x + et.x * et.x;
    let yy = ea.y * ea.y + eb.y * eb.y + et.y * et.y;
    let xy = ea.x * ea.y + eb.x * eb.y + et.x * et.y;
    let half_tr = 0.5 * (xx + yy);
    let disc = sqrt(max(0.25 * (xx - yy) * (xx - yy) + xy * xy, 0.0));
    var major = sqrt(max(half_tr + disc, 0.0));
    var minor = sqrt(max(half_tr - disc, 0.0));
    // A round outline has no principal direction (and atan2(0, 0) is undefined): any will do.
    let round = abs(xy) + abs(xx - yy) < 1e-9;
    let ang = select(0.5 * atan2(2.0 * xy, xx - yy), 0.0, round);
    let dir = vec2f(cos(ang), sin(ang));
    // Too thin to draw: drawn at the smallest size, weighted by how much of that it really covers,
    // never so little that a lone far bird drops below the surface.
    var w = b.a.w;
    let m0 = u.eye.w;
    if (minor < m0) {
        let area = major * minor;
        minor = m0;
        major = max(major, m0);
        w = w * clamp(area / (major * minor), 0.4, 1.0);
    }
    var corner = array<vec2f, 6>(
        vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0),
        vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0),
    );
    let l = corner[vi];
    let side = vec2f(-dir.y, dir.x);
    let p = s.xy + dir * l.x * REACH * major + side * l.y * REACH * minor;
    o.pos = vec4f(p.x / (0.5 * u.up.w), p.y / (0.5 * u.fwd.w), 0.5, 1.0);
    o.l = l;
    o.k = vec4f(w, minor, b.b.w, b.t.x);
    return o;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4f {
    let r2 = dot(in.l, in.l);
    if (r2 >= 1.0) { discard; }
    let q = 1.0 - r2;
    let d = q * q * in.k.x;
    return vec4f(d, d * in.k.y, d * in.k.z, d * in.k.w);
}
