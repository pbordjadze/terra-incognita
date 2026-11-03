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
// below. The body a fan round its middle (bill, round head, the shoulders, the short square tail,
// spread wider in a turn); each wing a strip from the root out to the pointed tip, its leading edge
// curving forward to the wrist and back to the tip, the hand beating more than the arm.
// 30 triangles, 90 vertices.
fn shape(i: u32, beat: f32, spread: f32) -> vec3f {
    let y1 = beat * 0.02;
    let y2 = beat * 0.045;
    let y3 = beat * 0.07;
    let y4 = beat * 0.095;
    let tz = -0.04 - 0.012 * abs(beat);
    let tw = 0.026 * spread;
    var p = array<vec3f, 26>(
        // body outline, clockwise from the bill (0..15), then its middle (16)
        vec3f(0.0, 0.0, 0.118), vec3f(0.012, 0.0, 0.098), vec3f(0.017, 0.0, 0.07), vec3f(0.026, 0.0, 0.025),
        vec3f(0.025, 0.0, -0.02), vec3f(0.017, 0.0, -0.05), vec3f(0.014, 0.0, -0.066), vec3f(tw, 0.0, -0.112),
        vec3f(0.0, 0.0, -0.104), vec3f(-tw, 0.0, -0.112), vec3f(-0.014, 0.0, -0.066), vec3f(-0.017, 0.0, -0.05),
        vec3f(-0.025, 0.0, -0.02), vec3f(-0.026, 0.0, 0.025), vec3f(-0.017, 0.0, 0.07), vec3f(-0.012, 0.0, 0.098),
        vec3f(0.0, 0.0, 0.01),
        // the right wing: leading edge root, arm, wrist, hand; the tip; trailing edge hand, arm, root
        vec3f(0.02, 0.0, 0.05), vec3f(0.065, y1, 0.06), vec3f(0.105, y2, 0.052), vec3f(0.15, y3, 0.022),
        vec3f(0.195, y4, tz),
        vec3f(0.148, y3, -0.034), vec3f(0.1, y2, -0.04), vec3f(0.058, y1, -0.036), vec3f(0.02, 0.0, -0.026),
    );
    var idx = array<u32, 90>(
        16u, 0u, 1u,  16u, 1u, 2u,  16u, 2u, 3u,  16u, 3u, 4u,  16u, 4u, 5u,  16u, 5u, 6u,
        16u, 6u, 7u,  16u, 7u, 8u,  16u, 8u, 9u,  16u, 9u, 10u,  16u, 10u, 11u,  16u, 11u, 12u,
        16u, 12u, 13u,  16u, 13u, 14u,  16u, 14u, 15u,  16u, 15u, 0u,
        17u, 25u, 18u,  25u, 24u, 18u,  18u, 24u, 19u,  24u, 23u, 19u,  19u, 23u, 20u,  23u, 22u, 20u,  20u, 22u, 21u,
        17u, 25u, 18u,  25u, 24u, 18u,  18u, 24u, 19u,  24u, 23u, 19u,  19u, 23u, 20u,  23u, 22u, 20u,  20u, 22u, 21u,
    );
    var v = p[idx[i]];
    // The left wing is the right one, mirrored.
    if (i >= 69u) { v.x = -v.x; }
    return v;
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32, b: In) -> VOut {
    var o: VOut;
    let f = u.right.w;
    let fwd = normalize(b.fwd.xyz);
    // Each bird a little its own: a roll of its own about its heading, and its own way of holding
    // its wings in a glide (level, raised, or swept down).
    let h1 = fract(sin(f32(ii) * 12.9898) * 43758.5453);
    let h2 = fract(sin(f32(ii) * 78.233) * 24634.6345);
    let side0 = normalize(cross(b.lift.xyz, fwd));
    let up0 = cross(fwd, side0);
    let me = ii == u32(u.ink.w);
    var roll = (h1 - 0.5) * 0.7 + 0.25 * sin(b.pos.w * 0.13 + h2 * 6.28);
    if (me) { roll = 0.0; }
    let side = side0 * cos(roll) + up0 * sin(roll);
    let up = cross(fwd, side);
    let beat = sin(b.pos.w) * b.lift.w + (1.0 - b.lift.w) * (h2 * 0.5 - 0.15);
    let rel0 = b.pos.xyz - u.eye.xyz;
    let z0 = dot(rel0, u.fwd.xyz);
    // How big it would be on screen: a wingspan of 0.38 m at its distance.
    let span = 0.38 * b.fwd.w * f / max(z0, 0.05);
    let grow = max(1.0, u.eye.w / max(span, 1e-4));
    // A turning bird fans its tail (the lift leaning off the vertical is the turn).
    let spread = 1.0 + 2.2 * clamp(1.0 - b.lift.y, 0.0, 0.5);
    var s = shape(vi, beat, spread) * b.fwd.w * grow;
    // Far birds a little fuller in the body and wing (their chord), so a distant flock reads dark,
    // as the eye sees it, not as fine stipple.
    let full = clamp(sqrt(28.0 * u.eye.w / 3.0 / max(span, 1e-3)), 1.0, 1.9);
    s = vec3f(s.x, s.y * full, s.z * full);
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
    // (Your own bird is always drawn: it is the one that rides just ahead of the eye.) Others never
    // come nearer than 5 m nor grow past an eighth of the glass, so yours is the largest, nearest.
    if (!me) {
        a = a * step(5.0, z0) * (1.0 - step(0.12, span / u.up.w));
    }
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
