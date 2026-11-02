// The veil: while a lamp is chosen the night deepens everywhere but a soft window round it, so its
// cloud of possibilities is what the eye goes to and the living land steps back without going dark
// (by day a cool, lighter veil does the same). A full-screen pass over the stage, before the marks,
// eased by the cloud's bloom; Visual's own, touching nothing in the ground.

struct Veil {
    centre: vec4<f32>,  // the chosen lamp's place (a unit vector)
    radius: f32,        // the window's radius, reading points
    soft: f32,          // the window's edge, reading points
    strength: f32,      // 0..1 with the bloom
    night: f32,
    panel: vec4<f32>,   // (R22) the panel's ground, x, y, w, h in points
    panel_a: f32,       // its strength, 0 none
    _p0: f32, _p1: f32, _p2: f32,
};
@group(1) @binding(0) var<uniform> Vl: Veil;

struct VeilOut { @builtin(position) pos: vec4<f32> };

@vertex
fn vs(@builtin(vertex_index) vi: u32) -> VeilOut {
    let c = corner(vi % 6u);
    var o: VeilOut;
    o.pos = vec4<f32>(c, 0.0, 1.0);
    return o;
}

@fragment
fn fs(i: VeilOut) -> @location(0) vec4<f32> {
    var acc = vec4<f32>(0.0);
    // (R22) the panel's own ground: a soft pool of the sea's pale (by night its deep) under the row of
    // marks, so they read as marks in a tray of their own and not as more lamps on the land under them.
    // No edge, no frame: a rounded field that feathers out over ten points
    if (Vl.panel_a > 0.0) {
        let pt = i.pos.xy / U.frame.z;
        let c = Vl.panel.xy + 0.5 * Vl.panel.zw;
        let h = 0.5 * Vl.panel.zw;
        let rr = min(h.x, h.y);
        let q = abs(pt - c) - (h - vec2<f32>(rr));
        let sd = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - rr;
        let soft = 7.0 * U.frame.w;
        let k = 1.0 - smoothstep(-soft, soft, sd);
        // (by day the page's own paper, so the tray reads as off the globe even over the white ice, on a
        // breath of shadow just outside it; by night the page's deep)
        let shadow = (1.0 - Vl.night) * 0.10 * Vl.panel_a * (1.0 - smoothstep(0.0, 2.2 * soft, sd)) * smoothstep(-soft, 0.4 * soft, sd);
        acc = vec4<f32>(vec3<f32>(0.20, 0.24, 0.30) * shadow, shadow);
        let ground = mix(vec3<f32>(0.965, 0.955, 0.925), vec3<f32>(0.035, 0.055, 0.10), Vl.night);
        let a = mix(0.93, 0.80, Vl.night) * Vl.panel_a * k;
        acc = vec4<f32>(ground * a, a) + acc * (1.0 - a);
    }
    if (Vl.strength <= 0.0) { return acc; }
    let p = place(Vl.centre.xyz);
    let d = length(i.pos.xy - p.xy) / rp();
    let window = smoothstep(Vl.radius, Vl.radius + Vl.soft, d);
    let shade = mix(vec3<f32>(0.78, 0.86, 0.92), vec3<f32>(0.02, 0.03, 0.06), Vl.night);
    let amount = mix(0.18, 0.5, Vl.night) * Vl.strength * window;
    return vec4<f32>(shade * amount, amount) + acc * (1.0 - amount);
}
