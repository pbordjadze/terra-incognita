// The songs on the sea: each ring a moment of a whale's song (its loudness, its pitch), spreading over
// the water from where the whale is. A ring is a circle on the sphere about its whale, so it bends
// round the globe as sound spreads in the sea; the land stops it. A crest of light with a soft trough
// behind it, as a swell catches the sky; the higher the note, the paler and finer, the lower, the
// deeper and wider. Drawn, not measured: the songs are real, the rings are how this toy shows them.

@group(0) @binding(0) var<uniform> U: View;
@group(1) @binding(0) var geoTex: texture_2d<f32>;
@group(1) @binding(1) var linearS: sampler;

struct Rings {
    at: array<vec4f, 64>,     // the whale's place (unit vector) and the ring's radius (radians)
    look: array<vec4f, 64>,   // loudness 0..1, pitch 0..1, which whale (0 humpback, 1 blue), width (radians)
    n: vec4f,                 // x how many
    ear: vec4f,               // the hydrophone (unit vector), how far down it is (0 none)
    whale: array<vec4f, 2>,   // each whale's place now: the land's shadow is cast from it
    misc: vec4f,              // x the hydrophone's reach (radians)
};
@group(1) @binding(2) var<uniform> RS: Rings;

// Whether the sea is open from `p` to the whale at `w`: a march along the great circle, each step as
// long as the distance to the nearest coast (geo.png's coarse field), stopping on land. Sound in the
// sea does not cross a peninsula, so a ring is not drawn in the bay behind one. (sea.rs is the same.)
fn open_sea(p: vec3f, w: vec3f) -> f32 {
    let c = clamp(dot(p, w), -1.0, 1.0);
    let theta = acos(c);
    if (theta < 1e-5) { return 1.0; }
    let t = normalize(w - p * c);
    var s = 0.0;
    for (var i = 0; i < 28; i++) {
        if (s >= theta) { return 1.0; }
        let q = p * cos(s) + t * sin(s);
        let lat = asin(clamp(q.y, -1.0, 1.0));
        let uv = vec2f(atan2(q.x, q.z) / (2.0 * PI) + 0.5, 0.5 - lat / PI);
        let g = textureSampleLevel(geoTex, linearS, uv, 1.0);
        let fine = (g.r * 255.0 - 128.0) / 8.0;
        let coarse = (g.g * 255.0 - 128.0) * 2.0;
        let d = select(fine, coarse, abs(coarse) > 14.0);
        if (d > 0.5 && s > 0.002) { return 0.0; }
        s += max(-d, 1.5) * (2.0 * PI / 4096.0) * max(cos(lat), 0.2) * 0.8;
    }
    return 1.0;
}

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let pix = in.position.xy;
    let G = ground_at(U, pix);
    let gr = uv_grad(G.uv);
    let g = textureSampleGrad(geoTex, linearS, G.uv, gr.dx, gr.dy);
    let fine = (g.r * 255.0 - 128.0) / 8.0;
    let coarse = (g.g * 255.0 - 128.0) * 2.0;
    let d = select(fine, coarse, abs(coarse) > 14.0);
    let fwD = fwidth(d);
    if (G.r2 >= 1.0) { return vec4f(0.0); }
    // the sea, and a little way back from the coast so a ring fades before it reaches the shore
    let sea = 1.0 - smoothstep(-2.0, 0.0, d);
    if (sea <= 0.0) { return vec4f(0.0); }
    let night = U.pad.x;
    var light = 0.0;
    var lcol = vec3f(0.0);
    var dark = 0.0;
    let n = i32(RS.n.x);
    // each whale's shadow behind the land, worked out once a pixel and only where its rings are
    var open = vec2f(-1.0, -1.0);
    // the hydrophone: the rings brighten as they pass over it
    let earD = acos(clamp(dot(G.g, RS.ear.xyz), -1.0, 1.0));
    let earBoost = 1.0 + 2.2 * RS.ear.w * exp(-sq(earD / max(RS.misc.x, 1e-5)));
    for (var i = 0; i < n; i++) {
        let a = RS.at[i];
        let l = RS.look[i];
        let c = clamp(dot(G.g, a.xyz), -1.0, 1.0);
        let dist = acos(c);
        let x = (dist - a.w) / l.w;
        if (x < -4.5 || x > 3.0) { continue; }
        let k = i32(l.z + 0.5);
        if (open[k] < 0.0) { open[k] = open_sea(G.g, RS.whale[k].xyz); }
        if (open[k] <= 0.0) { continue; }
        let crest = exp(-x * x);
        let trough = exp(-(x + 2.0) * (x + 2.0) * 0.6);
        // the humpback's rings warm white going gold as the note climbs; the blue whale's a cold
        // deep blue going pale: each whale's colour, so two songs crossing stay two
        var tint: vec3f;
        if (l.z < 0.5) {
            tint = mix(vec3f(1.0, 0.98, 0.93), vec3f(1.0, 0.86, 0.55), smoothstep(0.45, 0.85, l.y));
        } else {
            tint = mix(vec3f(0.30, 0.45, 0.95), vec3f(0.62, 0.80, 1.0), smoothstep(0.2, 0.6, l.y));
        }
        let w = l.x * crest * earBoost;
        light += w;
        lcol += tint * w;
        dark += l.x * trough * 0.5;
    }
    if (light <= 0.0005 && dark <= 0.0005) { return vec4f(0.0); }
    let la = clamp(light, 0.0, 1.0) * mix(0.85, 1.0, night) * sea;
    let col = lcol / max(light, 1e-4);
    let da = clamp(dark, 0.0, 1.0) * mix(0.10, 0.22, night) * sea * (1.0 - la);
    let darkCol = mix(vec3f(0.20, 0.36, 0.52), vec3f(0.0, 0.01, 0.03), night);
    // premultiplied: the crest's light, then the trough's shade under it
    return vec4f(col * la + darkCol * da, la + da);
}
