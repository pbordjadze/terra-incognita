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

// The silhouette in the bird's frame (x right, y lift, z forward), metres; 12 vertices, 4 triangles.
fn shape(i: u32, beat: f32) -> vec3f {
    let tip_y = beat * 0.085;
    let tip_z = -0.035 - 0.02 * abs(beat);
    var v = array<vec3f, 12>(
        // body: nose, the two shoulders, the tail
        vec3f(0.0, 0.0, 0.105), vec3f(-0.024, 0.0, 0.01), vec3f(0.024, 0.0, 0.01),
        vec3f(-0.024, 0.0, 0.01), vec3f(0.024, 0.0, 0.01), vec3f(0.0, 0.0, -0.10),
        // left wing: root front, root back, tip
        vec3f(-0.018, 0.0, 0.045), vec3f(-0.018, 0.0, -0.035), vec3f(-0.19, tip_y, tip_z),
        // right wing
        vec3f(0.018, 0.0, 0.045), vec3f(0.018, 0.0, -0.035), vec3f(0.19, tip_y, tip_z),
    );
    return v[i];
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
    // Coverage kept when grown; close birds rush past rather than fill the glass.
    var a = min(1.0, 1.5 / (grow * grow));
    a = a * smoothstep(1.1, 1.7, dist);
    let haze = 1.0 - exp(-dist / u.haze.w);
    let c = mix(u.ink.rgb, u.haze.rgb, haze);
    o.col = vec4f(c * a, a);
    return o;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4f {
    return in.col;
}
