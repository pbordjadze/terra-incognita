// The maybes: a held float's other possible selves, thousands of pale amber specks where each would
// be now, each with one dark grain by day so the cloud has texture and is read as a cloud, never as a
// tint; by night at a third, or they out-shine the cities (CRITICS.md R0). The positions come from
// the GPU (gpu/maybes.rs: lon, lat in degrees, held, k; sea/rules.wgsl moved them); one point-sized
// quad a speck, 0.75 reading points, scaled by M.bloom as they bloom out and fold back.

struct Maybe { @location(0) p: vec2f, @location(1) held: u32 };   // lon, lat (degrees), what holds it (Truth's Path)

struct Maybes {
    origin: vec4<f32>,  // the held float's place (a unit vector), where the specks bloom from
    hue: vec4<f32>,     // its lamp's hue: the specks are a pale wash of it
    bloom: f32,         // 0..1 as they bloom out of the float and fold back
    night: f32,
    density: f32,       // 512 over the count drawn, so the field reads the same whatever Truth's budget
    quiet: f32,         // 0 the cloud in front, more for the others out with it (R6: several at once)
};
@group(1) @binding(0) var<uniform> M: Maybes;

struct SpeckOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) r: f32,
    @location(2) held: f32,
    @location(3) facing: f32,
};

@vertex
fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) k: u32, m: Maybe) -> SpeckOut {
    let la = radians(m.p.y);
    let lo = radians(m.p.x);
    // from the float's place out to its own, eased, so blooming out means out (and folding back, back)
    let own = vec3<f32>(cos(la) * sin(lo), sin(la), cos(la) * cos(lo));
    let b = M.bloom * M.bloom * (3.0 - 2.0 * M.bloom);
    let g = normalize(mix(M.origin.xyz, own, b));
    let p = place(g);
    // at globe scale the cloud must read at a glance: each self is a soft kernel seven reading points
    // across, and five hundred of them lie over one another into a field of the lamp's hue (dense where
    // the sea would most likely have carried it); close in they shrink to the specks of a dusting
    let field = 1.0 - smoothstep(1.3, 2.4, U.disc.w);
    // (by night close in a speck is a little larger: a ring of glass with its lamp's spark in it, R6)
    // (toward the limb the field's kernels shrink with the sea's foreshortening, or a cloud at the rim
    // would bulge past it, R6)
    let edge = mix(0.35, 1.0, smoothstep(0.0, 0.5, p.z));
    let r = mix(mix(0.75, 1.0, M.night), 7.0 * edge, field) * rp() * min(M.bloom * 2.0, 1.0);
    let c = corner(vi % 6u);
    var o: SpeckOut;
    o.local = c * (r + 0.5 * U.frame.z);
    o.pos = clip(p.xy + o.local);
    // (past the limb gone; toward it fading, never clipped at a line: the rim is as smooth as the rest)
    if (p.z < 0.0 || M.bloom <= 0.0) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.facing = p.z;
    o.r = r;
    o.held = f32(m.held);
    return o;
}

@fragment
fn fs(i: SpeckOut) -> @location(0) vec4<f32> {
    let n = M.night;
    let d = length(i.local);
    var acc = vec4<f32>(0.0);
    // at the rim: fading by facing, the field's alpha thinned as the sea is foreshortened (the selves
    // crowd on the screen there though not on the sea), and nothing past the globe's edge
    let rim_px = length(i.pos.xy - U.disc.xy * U.frame.z) - U.disc.z * U.frame.z;
    let rim = smoothstep(0.02, 0.32, i.facing) * cover(rim_px) * mix(1.0, clamp(i.facing * 1.6, 0.25, 1.0), 1.0 - smoothstep(1.3, 2.4, U.disc.w));
    let shown = min(M.bloom * 2.0, 1.0) * rim * (1.0 - M.quiet);
    // a pale wash of the lamp's hue (a self aground is not marked: truth underneath); at globe scale a
    // soft kernel at a low alpha, so the field is the sum of many and glows where they crowd
    let field = 1.0 - smoothstep(1.3, 2.4, U.disc.w);
    // by night the maybes are of the sea, never of the land (R6: a maybe must never read as a town):
    // the lamp's hue cooled toward the glass's own pale, as light in water is, while the towns are
    // warm embers in pools of their own glow
    let cool = vec3<f32>(0.70, 0.90, 1.0);
    let night_hue = mix(M.hue.rgb, cool, 0.62);
    let speck = mix(mix(M.hue.rgb, vec3<f32>(1.0), mix(0.72, 0.35, field)), mix(night_hue, vec3<f32>(1.0), mix(0.25, 0.10, field)), n);
    let soft = exp(-d * d / (0.45 * i.r * i.r));
    // (a low alpha a kernel, so the field is a density: pale where one self passed, the lamp's own
    // colour where many did)
    let a = mix(mix(0.80, 0.30, n), 0.045 * pow(M.density, 0.75), field) * shown;
    if (n > 0.5 && field < 0.999) {
        // close in by night: each maybe a tiny float, a ring of its glass with the lamp's spark at its
        // heart, so the eye reads a thing on the water and not a light on the land
        let close = 1.0 - field;
        let ring = cover(abs(d - 0.72 * i.r) - 0.2 * i.r);
        acc = over(acc, night_hue, 0.5 * shown * close * ring);
        acc = over(acc, M.hue.rgb, 0.6 * shown * close * cover(d - 0.24 * i.r));
        acc = over(acc, speck, a * field * soft);
        return acc;
    }
    acc = over(acc, speck, a * mix(cover(d - i.r), soft, field));
    // the grain that gives the cloud texture from afar, gone close in where it read as ringed dots
    // (from afar a grain a speck's own size, not a third of the kernel: a third of seven points over
    // two thousand selves was a solid brown, R6)
    let grain = (1.0 - smoothstep(3.5, 5.5, U.disc.w)) * mix(0.5, 0.3, field) * (1.0 - n) * shown;
    acc = over(acc, M.hue.rgb * 0.55, grain * cover(d - mix(0.35 * i.r, 0.3 * rp(), field)));
    return acc;
}
