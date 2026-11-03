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
    ears: array<vec4f, 4>,    // the hydrophones (unit vectors), how far down each is (0 none)
    whale: array<vec4f, 2>,   // each whale's place now: the land's shadow is cast from it
    misc: vec4f,              // x the hydrophone's reach (radians)
};
@group(1) @binding(2) var<uniform> RS: Rings;

// How open the sea is from `p` to the whale at `w`, 0..1: a march along the great circle, each step
// as long as the distance to the nearest coast (geo.png's coarse field), noting how deep into land it
// goes, read four times coarser than the map (its mip 2) so a coast's wiggles and the islands blur
// away and neighbouring rays agree. A peninsula or a coast stands in the way (Baja is a hundred kilometres across, its middle four
// map pixels from either shore); an island a few tens of kilometres wide (under one) does not (sound bends round it), so no wedge of shadow is cast by one.
// The shadow's edge is soft: how deep the march went fades it. (sea.rs is the same.)
fn open_sea(p: vec3f, w: vec3f) -> f32 {
    let c = clamp(dot(p, w), -1.0, 1.0);
    let theta = acos(c);
    if (theta < 1e-5) { return 1.0; }
    let t = normalize(w - p * c);
    var s = 0.0;
    var deepest = 0.0;
    // the singer's own bay casts no shadow: the land within about fifty kilometres of it is not
    // asked (its headlands would cut the rings into rays)
    let end = theta - 5.0 * (2.0 * PI / 4096.0);
    for (var i = 0; i < 32; i++) {
        if (s >= end) { break; }
        let q = p * cos(s) + t * sin(s);
        let lat = asin(clamp(q.y, -1.0, 1.0));
        let uv = vec2f(atan2(q.x, q.z) / (2.0 * PI) + 0.5, 0.5 - lat / PI);
        let g = textureSampleLevel(geoTex, linearS, uv, 2.0);
        let fine = (g.r * 255.0 - 128.0) / 8.0;
        let coarse = (g.g * 255.0 - 128.0) * 2.0;
        let d = select(fine, coarse, abs(coarse) > 14.0);
        deepest = max(deepest, d);
        if (deepest > 2.5) { break; }
        s += max(abs(d), 1.5) * (2.0 * PI / 4096.0) * max(cos(lat), 0.2) * 0.8;
    }
    return 1.0 - smoothstep(0.5, 2.2, deepest);
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
    // the open sea calmed under an even night: the ground's depth bands and the seams of its data
    // show as flat panels close in; the veil leaves the shelf and the towns' glow along the coast
    let veil = mix(0.72, 0.88, smoothstep(-14.0, -40.0, d)) * smoothstep(-3.0, -14.0, d) * sea;
    let veilCol = vec3f(13.0, 20.0, 35.0) / 255.0 * (0.97 + 0.06 * vnoise3(G.g * 60.0));
    let night = U.pad.x;
    var light = 0.0;
    var lcol = vec3f(0.0);
    var dark = 0.0;
    let n = i32(RS.n.x);
    // each whale's shadow behind the land, worked out once a pixel and only where its rings are
    var open = vec2f(-1.0, -1.0);
    // the hydrophones: the rings brighten as they pass over each
    var earBoost = 1.0;
    for (var e = 0; e < 4; e++) {
        let ear = RS.ears[e];
        if (ear.w <= 0.0) { continue; }
        let earD = acos(clamp(dot(G.g, ear.xyz), -1.0, 1.0));
        earBoost += 2.2 * ear.w * exp(-sq(earD / max(RS.misc.x, 1e-5)));
    }
    for (var i = 0; i < n; i++) {
        let a = RS.at[i];
        let l = RS.look[i];
        let c = clamp(dot(G.g, a.xyz), -1.0, 1.0);
        let dist = acos(c);
        let x = (dist - a.w) / l.w;
        if (x < -4.5 || x > 3.0) { continue; }
        let k = i32(l.z + 0.5);
        if (open[k] < 0.0) { open[k] = open_sea(G.g, RS.whale[k].xyz); }
        if (open[k] <= 0.001) { continue; }
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
        let w = l.x * crest * earBoost * open[k];
        light += w;
        lcol += tint * w;
        dark += l.x * trough * 0.5 * open[k];
    }
    if (light <= 0.0005 && dark <= 0.0005) { return vec4f(veilCol * veil, veil); }
    let la = clamp(light, 0.0, 1.0) * mix(0.85, 1.0, night) * sea;
    let col = lcol / max(light, 1e-4);
    let da = clamp(dark, 0.0, 1.0) * mix(0.10, 0.22, night) * sea * (1.0 - la);
    let darkCol = mix(vec3f(0.20, 0.36, 0.52), vec3f(0.0, 0.01, 0.03), night);
    // premultiplied: the crest's light, then the trough's shade under it
    let base = vec4f(veilCol * veil, veil);
    let top = vec4f(col * la + darkCol * da, la + da);
    return top + base * (1.0 - top.a);
}
