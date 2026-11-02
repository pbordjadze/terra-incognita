// Strokes (R10): one continuous line through a path's points, a tapering ribbon in its lamp's hue on
// a dark edge by day (its own light by night). The wake is one drawn line, not beads: each segment is
// an instanced quad that keeps only its own piece of the line, cut at the bisector of the turn at each
// end, so neighbouring pieces meet exactly (a round join, R16) and never lie twice over each other; a fading line
// has no darker knots at its joints. A true end has a round cap. The same strokes draw the globe's
// wakes (points on the unit sphere, placed and hidden at the rim as every mark is) and the screen's
// own marks (points in points: the panel's journeys and lamps, the record control's ring). Drawn twice,
// every segment's edge then every segment, so a path crossing another keeps its colour.

struct StrokeIn {
    @location(0) prev: vec4<f32>,   // xyz the point before a (a itself at a cap), w a's width (reading points)
    @location(1) a: vec4<f32>,      // xyz, w a's alpha
    @location(2) b: vec4<f32>,      // xyz, w b's width
    @location(3) next: vec4<f32>,   // xyz the point after b (b itself at a cap), w b's alpha
    @location(4) hue: vec4<f32>,    // the line's hue; w 1 for points on the screen (x, y in points), 0 on the globe
    @location(5) flags: vec4<f32>,  // x a round cap at a, y at b, z night, w hollow (the sea was the sheet's guess)
};

struct WakeUniform {
    layer: f32,         // 0 this draw lays the edges, 1 the lines
    airy: f32,          // 1 a wind line (R21): no dark edge, paler, its edges feathered
    _p1: f32,
    _p2: f32,
};
@group(1) @binding(0) var<uniform> W: WakeUniform;

struct StrokeOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) px: vec2<f32>,
    @location(1) @interpolate(flat) ab: vec4<f32>,      // a and b in pixels
    @location(2) @interpolate(flat) m: vec4<f32>,       // the cut at a (its normal), the cut at b
    @location(3) @interpolate(flat) hw: vec4<f32>,      // half widths at a and b in pixels, alphas at a and b
    @location(4) @interpolate(flat) hue: vec4<f32>,     // rgb, night
    @location(5) @interpolate(flat) flags: vec4<f32>,   // caps at a and b, hollow, pixels a reading point
};

fn stroke_px(g: vec3<f32>, screen: bool) -> vec3<f32> {
    if (screen) { return vec3<f32>(g.xy * U.frame.z, 1.0); }
    let p = place(g);
    return p.xyz;
}

fn dir_or(v: vec2<f32>, fallback: vec2<f32>) -> vec2<f32> {
    let l = length(v);
    if (l > 1e-3) { return v / l; }
    return fallback;
}

@vertex
fn vs(@builtin(vertex_index) vi: u32, s: StrokeIn) -> StrokeOut {
    let screen = s.hue.w > 0.5;
    let P = stroke_px(s.prev.xyz, screen);
    let A = stroke_px(s.a.xyz, screen);
    let B = stroke_px(s.b.xyz, screen);
    let N = stroke_px(s.next.xyz, screen);
    // on the globe the line thins toward the whole Earth, as the beads did, so a month's wake is not a slug
    let k = select(by_zoom(0.55, 2.5), 1.0, screen);
    let ha = 0.5 * s.prev.w * rp() * k;
    let hb = 0.5 * s.b.w * rp() * k;
    let d = dir_or(B.xy - A.xy, vec2<f32>(1.0, 0.0));
    let n = vec2<f32>(-d.y, d.x);
    let dp = dir_or(A.xy - P.xy, d);
    let dn = dir_or(N.xy - B.xy, d);
    let ma = dir_or(dp + d, d);
    let mb = dir_or(d + dn, d);
    let h = max(ha, hb) + 0.45 * U.frame.z + 1.5;
        // the quad reaches a half width past each end: every join is round (R16), never a miter's tip
    let ea = h + 1.0;
    let eb = h + 1.0;
    let c = corner(vi % 6u);
    var at = select(B.xy + d * eb, A.xy - d * ea, c.x < 0.0);
    at = at + n * c.y * h;
    var o: StrokeOut;
    o.pos = clip(at);
    if (!screen && (A.z < 0.3 || B.z < 0.3)) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
    o.px = at;
    o.ab = vec4<f32>(A.xy, B.xy);
    o.m = vec4<f32>(ma, mb);
    o.hw = vec4<f32>(ha, hb, s.a.w, s.next.w);
    o.hue = vec4<f32>(s.hue.rgb, s.flags.z);
    o.flags = vec4<f32>(s.flags.x, s.flags.y, s.flags.w, rp() * k);
    return o;
}

@fragment
fn fs(i: StrokeOut) -> @location(0) vec4<f32> {
    let A = i.ab.xy;
    let B = i.ab.zw;
    let len = length(B - A);
    let d = dir_or(B - A, vec2<f32>(1.0, 0.0));
    let n = vec2<f32>(-d.y, d.x);
    let p = i.px;
    let t = dot(p - A, d);
    // only this segment's own piece: between the cut at a (kept on its side) and the cut at b (left
    // to the next); a true end is round instead
    if (i.flags.x < 0.5 && dot(p - A, i.m.xy) < 0.0) { return vec4<f32>(0.0); }
    if (i.flags.y < 0.5 && dot(p - B, i.m.zw) >= 0.0) { return vec4<f32>(0.0); }
    // past either end the distance is to the end itself: a round cap at a true end, and at a join
    // the outer side of the turn rounded (R16: a miter there drew an arrowhead where a line turns
    // back on itself); the cuts above still give each pixel to one segment only
    var dist = abs(dot(p - A, n));
    if (t < 0.0) { dist = length(p - A); }
    if (t > len) { dist = length(p - B); }
    let f = clamp(t / max(len, 1e-3), 0.0, 1.0);
    let hw = mix(i.hw.x, i.hw.y, f);
    let al = mix(i.hw.z, i.hw.w, f);
    let night = i.hue.w;
    var acc = vec4<f32>(0.0);
    if (W.airy > 0.5) {
        // a wind line (R21): a light stroke of its hue, no hard edge, its own light by
        // night, feathered so it reads as air and not as a wake cut in the water
        if (W.layer < 0.5) { return vec4<f32>(0.0); }
        let soft = 1.0 - smoothstep(hw - 0.6 * U.frame.z, hw + 0.9 * U.frame.z, dist);
        // (by day a breath of the hue darkened under it, so a light line still reads on the light sea)
        acc = over(acc, i.hue.rgb * 0.5, (1.0 - night) * 0.35 * al * (1.0 - smoothstep(hw, hw + 1.6 * U.frame.z, dist)));
        return over(acc, mix(i.hue.rgb * 0.92, mix(i.hue.rgb, vec3<f32>(1.0), 0.15), night), al * soft);
    }
    if (W.layer < 0.5) {
        // the edge: the hue darkened, by day only
        acc = over(acc, i.hue.rgb * 0.42, (1.0 - night) * al * al * cover(dist - (hw + 0.45 * U.frame.z)));
    } else {
        acc = over(acc, mix(i.hue.rgb * 0.95, i.hue.rgb, night), al * cover(dist - hw));
        // hollow where the sea was a guess: a pale heart down the line
        let pale = mix(vec3<f32>(0.761, 0.894, 0.957), vec3<f32>(0.063, 0.094, 0.157), night);
        acc = over(acc, pale, i.flags.z * al * cover(dist - max(hw - 0.9 * i.flags.w, 0.0)) * step(0.9 * i.flags.w, hw));
    }
    return acc;
}
