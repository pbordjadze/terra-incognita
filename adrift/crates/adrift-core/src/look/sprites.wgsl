// The things on the sea: floats, their wakes' beads, the maybes, the touch's ring: each an instanced
// quad on the screen, drawn as a signed-distance shape (R0's plain forms; Visual designs them).

@group(0) @binding(0) var<uniform> U: View;

struct Instance {
    @location(0) at: vec2f,      // screen position, points
    @location(1) size: f32,      // radius, points
    @location(2) kind: u32,      // 0 float, 1 bead, 2 ring, 3 maybe, 4 drifter
    @location(3) color: vec4f,   // premultiplied, with its alpha
    @location(4) t: f32,         // the shape's own parameter (a ring's growth, a float's lamp)
};

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) local: vec2f,
    @location(1) color: vec4f,
    @location(2) @interpolate(flat) kind: u32,
    @location(3) size: f32,
    @location(4) t: f32,
};

@vertex fn vs_sprite(@builtin(vertex_index) vid: u32, inst: Instance) -> VsOut {
    let corner = vec2f(f32(vid & 1u) * 2.0 - 1.0, f32((vid >> 1u) & 1u) * 2.0 - 1.0);
    let pad = 1.5;      // points of room for the edge's softness and the lamp's glow
    let r = inst.size + pad + select(0.0, inst.size * 1.5, inst.kind == 0u);
    let pt = inst.at + corner * r;
    let ndc = vec2f(pt.x / (U.frame.x / U.frame.z) * 2.0 - 1.0, 1.0 - pt.y / (U.frame.y / U.frame.z) * 2.0);
    var out: VsOut;
    out.position = vec4f(ndc, 0.0, 1.0);
    out.local = corner * r;
    out.color = inst.color;
    out.kind = inst.kind;
    out.size = inst.size;
    out.t = inst.t;
    return out;
}

@fragment fn fs_sprite(in: VsOut) -> @location(0) vec4f {
    let d = length(in.local);
    let px = 1.0 / U.frame.z;                     // a pixel, in points
    var a = 0.0;
    var col = in.color.rgb;
    if (in.kind == 0u) {
        // a glass float: a clear disc with a dark rim, lamp amber at its heart, a warm glow about it
        let body = 1.0 - smoothstep(in.size - px, in.size + px, d);
        let rim = smoothstep(in.size - 2.0 * px - 0.6, in.size - 0.6, d) * body;
        let lamp = exp(-sq(d / (0.45 * in.size)));
        let glow = exp(-sq(max(d - in.size, 0.0) / (0.9 * in.size))) * (1.0 - body) * 0.35 * in.t;
        let glass = vec3f(0.78, 0.86, 0.88);
        let amber = vec3f(1.0, 0.68, 0.22);
        var c = mix(glass, amber, lamp);
        c = mix(c, vec3f(0.22, 0.20, 0.16), 0.55 * rim);
        let alpha = body * mix(0.55, 1.0, lamp) + glow;
        return vec4f(mix(c, amber, glow / max(alpha, 1e-4) * 0.8) * alpha, alpha) * in.color.a;
    } else if (in.kind == 2u) {
        // a ring: a thin circle of light at the radius, as the sea's own but drawn once, on the screen
        let w = max(1.5 * px, 0.6);
        a = (1.0 - smoothstep(0.0, w, abs(d - in.size))) * in.color.a;
    } else {
        // a bead, a maybe, a drifter: a soft dot
        a = (1.0 - smoothstep(in.size - px, in.size + px, d)) * in.color.a;
    }
    return vec4f(col * a, a);
}
