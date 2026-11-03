// The mold close in: each agent drawn where it is, to the fraction of a cell, as a short streak of
// light along the way it last stepped (a dot when it held still), a fixed width in points. Seen from
// afar the body's field draws it (mold.wgsl); as the reader comes closer the field gives way to these,
// so a tube stays a thin vein of streaming light at any zoom instead of swelling into the field's
// cells, and the network gains detail as it is approached. (Agents in transit are not drawn: they
// are not yet part of the mold's body.)

struct Agent { p: vec2f, h: f32, s: u32, home: u32, moved: u32 };

struct Params { grid: vec4u, mv: vec4f, tr: vec4f, misc: vec4f, ex: vec4f, feeds: array<vec4f, 32> };
// a: x how much shows, y the gain, z the streaks' share (by zoom), w the grid's rows; ages: x each
// flake's seconds since it was laid
struct Look { a: vec4f, ages: array<vec4f, 32>, ripples: array<vec4f, 8> };   // ripples: lon, lat, seconds since, 1 on land

@group(1) @binding(0) var<storage, read> agents: array<Agent>;
@group(1) @binding(1) var<uniform> look: Look;
@group(1) @binding(2) var<uniform> P: Params;

const NONE: u32 = 0xffffffffu;
const STREAK = vec3f(0.62, 1.0, 0.78);   // the foxfire of mold.wgsl, a little whiter at a point

struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) along: f32,      // 0..1 tail to head
    @location(1) across: f32,     // pixels from the streak's middle line
    @location(2) half: f32,       // its half width in pixels
    @location(3) shade: f32,
};

fn world(p: vec2f) -> vec3f {
    return vec3f(cos(p.y) * sin(p.x), sin(p.y), cos(p.y) * cos(p.x));
}

// a world point in pixels on the target, and how far it faces the reader
fn screen(w: vec3f) -> vec3f {
    let v = vec3f(dot(U.rot0.xyz, w), dot(U.rot1.xyz, w), dot(U.rot2.xyz, w));
    let pt = vec2f(U.disc.x + v.x * U.disc.z, U.disc.y - v.y * U.disc.z);
    return vec3f(pt * U.frame.z, v.z);
}

// how much brighter the mold is by a flake it is feeding on (mold.wgsl's the same)
fn fed(w: vec3f) -> f32 {
    var b = 1.0;
    let nf = u32(P.misc.y);
    for (var i = 0u; i < nf; i++) {
        let fd = P.feeds[i];
        let f = vec3f(cos(fd.y) * sin(fd.x), sin(fd.y), cos(fd.y) * cos(fd.x));
        let cells = acos(clamp(dot(f, w), -1.0, 1.0)) / (PI / look.a.w);
        b += 1.0 * exp(-cells / 14.0) * smoothstep(0.0, 3.0, look.ages[i].x);
    }
    return b;
}

@vertex fn vs_streak(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VsOut {
    var o: VsOut;
    o.pos = vec4f(2.0, 2.0, 0.0, 1.0);   // (off the target unless drawn)
    let a = agents[iid];
    if (a.home == NONE || a.moved == 0u || look.a.z <= 0.0) { return o; }
    let c = PI / look.a.w;
    // the tail: a step behind along the heading (the way it came), or the point itself held still
    let len = select(0.25, 1.15, a.moved == 1u);
    let back = vec2f(a.p.x - len * c * cos(a.h) / max(cos(a.p.y), 0.05), a.p.y - len * c * sin(a.h));
    let h = screen(world(a.p));
    let t = screen(world(back));
    if (h.z < 0.02) { return o; }
    let ppp = U.frame.z;
    let half = 0.4 * ppp * U.frame.w;              // the width: about 1.1 reading points
    var d = h.xy - t.xy;
    let l = length(d);
    let dir = select(vec2f(1.0, 0.0), d / max(l, 1e-4), l > 1e-3);
    let nrm = vec2f(-dir.y, dir.x);
    let ext = half + 1.0;                            // a pixel more, for the edge's smoothing
    // a quad round the segment, its ends rounded by the extension
    let corner = array<vec2f, 6>(vec2f(0.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0), vec2f(0.0, -1.0), vec2f(1.0, 1.0), vec2f(0.0, 1.0));
    let k = corner[vid];
    let p = mix(t.xy - dir * ext, h.xy + dir * ext, k.x) + nrm * ext * k.y;
    o.pos = vec4f(p.x / U.frame.x * 2.0 - 1.0, 1.0 - p.y / U.frame.y * 2.0, 0.0, 1.0);
    o.along = k.x;
    o.across = ext * k.y;
    o.half = half;
    o.shade = smoothstep(0.02, 0.25, h.z) * (0.85 + 0.3 * hash13(vec3f(f32(iid) * 0.37, 1.7, 3.1))) * fed(world(a.p));
    return o;
}

@fragment fn fs_streak(in: VsOut) -> @location(0) vec4f {
    let cover = 1.0 - smoothstep(in.half - 0.5, in.half + 0.5, abs(in.across));
    // brighter at the head, where the agent is: a streak of light running the way it goes
    let run = 0.45 + 0.55 * smoothstep(0.0, 1.0, in.along);
    let a = cover * run * in.shade * look.a.z * look.a.x * 0.15;
    return vec4f(STREAK * a, 0.0);
}
