// The whales themselves and the way they came: each a small quad on the screen whose fragment draws a
// shape. A humpback seen from above (the long white flippers, a third of its length, are the
// humpback's own), a blue whale (long and slender, small flippers, mottled pale blue-grey), and the
// dots of each one's wake over the last weeks. The flukes beat as it swims.

@group(0) @binding(0) var<uniform> U: View;

struct Mark {
    @location(0) at: vec4f,     // centre x, y (points), size (points), heading (radians, screen, y down)
    @location(1) look: vec4f,   // kind (0 humpback, 1 blue, 2 a dot of the wake), alpha, the swim's phase, glow
};

struct VOut {
    @builtin(position) pos: vec4f,
    @location(0) q: vec2f,      // in body lengths, forward +x
    @location(1) @interpolate(flat) look: vec4f,
    @location(2) @interpolate(flat) size: f32,
};

@vertex fn vs_mark(@builtin(vertex_index) vi: u32, m: Mark) -> VOut {
    var corners = array<vec2f, 6>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, -1.0), vec2f(1.0, 1.0), vec2f(-1.0, 1.0));
    let c = corners[vi];
    let half = m.at.z * 0.75;
    let cs = cos(m.at.w);
    let sn = sin(m.at.w);
    // the quad turned with the body: local (forward, side) to screen points
    let off = vec2f(c.x * cs - c.y * sn, c.x * sn + c.y * cs) * half;
    let pt = m.at.xy + off;
    let px = pt * U.frame.z;
    var o: VOut;
    o.pos = vec4f(px.x / U.frame.x * 2.0 - 1.0, 1.0 - px.y / U.frame.y * 2.0, 0.0, 1.0);
    o.q = c * 0.75;
    o.look = m.look;
    o.size = m.at.z;
    return o;
}

fn ell(p: vec2f, r: vec2f) -> f32 {
    // a fair signed distance to an ellipse (exact enough at these sizes)
    let k0 = length(p / r);
    let k1 = length(p / (r * r));
    return k0 * (k0 - 1.0) / max(k1, 1e-5);
}

fn rot(p: vec2f, a: f32) -> vec2f {
    let c = cos(a); let s = sin(a);
    return vec2f(c * p.x + s * p.y, -s * p.x + c * p.y);
}

// a limb: an ellipse of length `len` and width `wid` from `root`, pointing at angle `a`
fn limb(p: vec2f, root: vec2f, a: f32, len: f32, wid: f32) -> f32 {
    let dir = vec2f(cos(a), sin(a));
    let q = rot(p - root - dir * len * 0.5, a);
    return ell(q, vec2f(len * 0.5, wid * 0.5));
}

struct Shape { body: f32, fin: f32 };

fn whale(p0: vec2f, kind: f32, phase: f32) -> Shape {
    // the tail beats: the body behind the middle bends, more toward the flukes
    var p = p0;
    let beat = sin(phase) * 0.12;
    let bend = smoothstep(0.0, -0.55, p.x);
    p = vec2f(p.x, p.y - beat * bend * bend * 0.25);
    var s: Shape;
    if (kind < 0.5) {
        // the humpback: stout, a broad head, its tail stock narrowing
        let taper = mix(0.42, 1.0, smoothstep(-0.48, 0.05, p.x));
        let b = min(ell(vec2f(p.x - 0.02, p.y / taper), vec2f(0.47, 0.105)), length(p - vec2f(0.30, 0.0)) - 0.095);
        let f1 = limb(p, vec2f(0.16, 0.07), 2.25, 0.30, 0.07);
        let f2 = limb(p, vec2f(0.16, -0.07), -2.25, 0.30, 0.07);
        let tail = vec2f(-0.43, -beat * 0.25);
        let k1 = limb(p, tail, 1.95 + beat, 0.18, 0.065);
        let k2 = limb(p, tail, -1.95 + beat, 0.18, 0.065);
        s.body = min(min(b, k1), k2);
        s.fin = min(f1, f2);
    } else {
        // the blue whale: long and slender, a flat broad head, small flippers far forward
        let taper = mix(0.40, 1.0, smoothstep(-0.5, 0.15, p.x));
        // the broad flat U of the head
        let b = min(ell(vec2f(p.x, p.y / taper), vec2f(0.5, 0.068)), ell(p - vec2f(0.36, 0.0), vec2f(0.13, 0.066)));
        let f1 = limb(p, vec2f(0.22, 0.045), 2.25, 0.12, 0.028);
        let f2 = limb(p, vec2f(0.22, -0.045), -2.25, 0.12, 0.028);
        let tail = vec2f(-0.47, -beat * 0.25);
        let k1 = limb(p, tail, 1.9 + beat, 0.12, 0.045);
        let k2 = limb(p, tail, -1.9 + beat, 0.12, 0.045);
        s.body = min(min(b, min(k1, k2)), min(f1, f2));
        s.fin = 1.0;
    }
    return s;
}

@fragment fn fs_mark(v: VOut) -> @location(0) vec4f {
    let kind = v.look.x;
    let alpha = v.look.y;
    let night = U.pad.x;
    let px = 1.0 / max(v.size * U.frame.z, 1.0);       // a pixel in body lengths
    if (kind > 1.5) {
        // a dot of the wake
        let r = length(v.q);
        let a = (1.0 - smoothstep(0.30 - px, 0.30 + px, r)) * alpha;
        let col = mix(vec3f(0.22, 0.36, 0.48), vec3f(0.80, 0.88, 0.96), night);
        return vec4f(col * a, a);
    }
    let s = whale(v.q, kind, v.look.z);
    let d = min(s.body, s.fin);
    // the water over it: a soft shadow round the body, then the body, then the humpback's white fins
    let shadow = (1.0 - smoothstep(0.0, 0.10, d)) * 0.28;
    let inside = 1.0 - smoothstep(-px, px, d);
    let finIn = 1.0 - smoothstep(-px, px, s.fin);
    var body: vec3f;
    if (kind < 0.5) {
        body = mix(vec3f(0.17, 0.24, 0.31), vec3f(0.30, 0.37, 0.44), night);
        // the back a little lighter along its ridge
        body = body * (1.0 + 0.25 * smoothstep(0.08, 0.0, abs(v.q.y)));
    } else {
        // mottled pale blue-grey, as the blue whale is under the water
        let mott = vnoise(v.q * 38.0 + vec2f(3.1, 7.7));
        body = mix(vec3f(0.36, 0.52, 0.66), vec3f(0.55, 0.70, 0.84), night) * (0.9 + 0.2 * mott);
    }
    let fin = mix(vec3f(0.90, 0.95, 0.95), vec3f(0.96, 0.98, 1.0), night);
    var col = mix(body, fin, finIn);
    // the one followed glows a little in the water
    let glow = v.look.w * (1.0 - smoothstep(0.0, 0.35, d)) * 0.35 * (1.0 - inside);
    let shadeCol = mix(vec3f(0.10, 0.20, 0.30), vec3f(0.0), night);
    let glowCol = mix(vec3f(1.0, 1.0, 1.0), vec3f(0.7, 0.85, 1.0), night);
    let a = inside * alpha;
    let sa = shadow * (1.0 - inside) * alpha * (1.0 - v.look.w * 0.6);
    let ga = glow * alpha;
    return vec4f(col * a + shadeCol * sa + glowCol * ga, a + sa + ga);
}
