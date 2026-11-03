// The birds: each a starling's silhouette in three dimensions (a body and two pointed wings in the
// plane its lift leans), so a flock that banks shows more wing and darkens, as a real one does (the
// dark bands that run through a murmuration are birds rolling together). Far birds smaller than a
// couple of pixels are drawn at that size with their coverage as alpha, so they never shimmer.

struct U {
    right: vec4f,   // .w focal px
    up: vec4f,      // .w width px
    fwd: vec4f,     // .w height px
    eye: vec4f,     // camera position, .w: minimum size in px
    ink: vec4f,     // the birds' colour (display), .w: unused
    haze: vec4f,    // the air's colour (display), .w: haze distance (m)
};

@group(0) @binding(0) var<uniform> u: U;

struct In {
    @location(0) pos: vec4f,   // xyz, w: wingbeat phase
    @location(1) fwd: vec4f,   // xyz unit, w: size
    @location(2) lift: vec4f,  // xyz unit, w: flap amount
};

struct VOut {
    @builtin(position) pos: vec4f,
    @location(0) col: vec4f,
};

// The silhouette in the bird's frame (x right, y lift, z forward), metres: a starling seen from
// below, its short square tail, its pointed triangular wings bent at the wrist, the hand beating
// more than the arm. 13 triangles, 39 vertices.
fn shape(i: u32, beat: f32) -> vec3f {
    let ya = beat * 0.035;
    let yt = beat * 0.095;
    let tz = -0.05 - 0.015 * abs(beat);
    var p = array<vec3f, 14>(
        vec3f(0.0, 0.0, 0.105),                                     // 0 bill
        vec3f(-0.017, 0.0, 0.07), vec3f(0.017, 0.0, 0.07),          // 1 2 neck
        vec3f(-0.026, 0.0, 0.02), vec3f(0.026, 0.0, 0.02),          // 3 4 shoulders
        vec3f(-0.019, 0.0, -0.045), vec3f(0.019, 0.0, -0.045),      // 5 6 hips
        vec3f(-0.030, 0.0, -0.105), vec3f(0.030, 0.0, -0.105),      // 7 8 tail
        vec3f(0.020, 0.0, 0.048),                                   // 9 wing root, leading
        vec3f(0.020, 0.0, -0.022),                                  // 10 wing root, trailing
        vec3f(0.100, ya, 0.036),                                    // 11 wrist
        vec3f(0.092, ya, -0.030),                                   // 12 elbow, trailing
        vec3f(0.190, yt, tz),                                       // 13 tip
    );
    var idx = array<u32, 39>(
        0u, 1u, 2u,  1u, 3u, 4u,  1u, 4u, 2u,  3u, 5u, 6u,  3u, 6u, 4u,  5u, 7u, 8u,  5u, 8u, 6u,
        9u, 10u, 11u,  10u, 12u, 11u,  11u, 12u, 13u,
        9u, 10u, 11u,  10u, 12u, 11u,  11u, 12u, 13u,
    );
    var v = p[idx[i]];
    // The second wing is the first, mirrored.
    if (i >= 30u) { v.x = -v.x; }
    return v;
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, b: In) -> VOut {
    var o: VOut;
    let f = u.right.w;
    let fwd = normalize(b.fwd.xyz);
    let side = normalize(cross(b.lift.xyz, fwd));   // the bird's right... or left: symmetric
    let up = cross(fwd, side);
    let beat = sin(b.pos.w) * b.lift.w + (1.0 - b.lift.w) * 0.12;
    let rel0 = b.pos.xyz - u.eye.xyz;
    let z0 = dot(rel0, u.fwd.xyz);
    // How big it would be on screen: a wingspan of 0.38 m at its distance.
    let span = 0.38 * b.fwd.w * f / max(z0, 0.05);
    let grow = max(1.0, u.eye.w / max(span, 1e-4));
    let s = shape(vi, beat) * b.fwd.w * grow;
    let w = b.pos.xyz + side * s.x + up * s.y + fwd * s.z;
    let rel = w - u.eye.xyz;
    let x = dot(rel, u.right.xyz);
    let y = dot(rel, u.up.xyz);
    let z = dot(rel, u.fwd.xyz);
    o.pos = vec4f(x * f / (0.5 * u.up.w), y * f / (0.5 * u.fwd.w), z - 0.08, z);
    let dist = length(rel0);
    // Coverage kept when grown.
    var a = min(1.0, 2.2 / (grow * grow));
    // Nothing ever reaches the eye: a bird nearer than a couple of metres in front of it, or beside
    // or behind it, fades and then is not drawn at all (its wings would sweep the glass).
    a = a * step(2.2, z0) * (1.0 - step(0.4, span / u.up.w));
    if (a <= 0.002) {
        o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
        o.col = vec4f(0.0);
        return o;
    }
    let haze = 1.0 - exp(-dist / u.haze.w);
    let c = mix(u.ink.rgb, u.haze.rgb, haze);
    o.col = vec4f(c * a, a);
    return o;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4f {
    return in.col;
}
