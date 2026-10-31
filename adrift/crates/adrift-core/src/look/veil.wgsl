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
    if (Vl.strength <= 0.0) { return vec4<f32>(0.0); }
    let p = place(Vl.centre.xyz);
    let d = length(i.pos.xy - p.xy) / rp();
    let window = smoothstep(Vl.radius, Vl.radius + Vl.soft, d);
    let shade = mix(vec3<f32>(0.78, 0.86, 0.92), vec3<f32>(0.02, 0.03, 0.06), Vl.night);
    let amount = mix(0.18, 0.5, Vl.night) * Vl.strength * window;
    return vec4<f32>(shade * amount, amount);
}
