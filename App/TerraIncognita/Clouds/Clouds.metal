// The clouds, one pixel at a time: a full-screen triangle over the globe (CloudsLayer.swift), in a
// layer of its own between the globe's drawing and its names.
//
// Measured: how often each place is under cloud this month, and which kinds of cloud make up its
// sky (CloudsSky.png, from ISCCP-H: tools/playground/clouds/build.py). Drawn: particular clouds,
// noise thresholded so that each place's drawn cover is its measured cover; cyclones wound into
// the westerlies; everything carried by the usual winds at its height. Five layers, low to high:
// cumulus; stratocumulus and stratus; altostratus and nimbostratus; the thin clouds aloft
// (altocumulus, cirrus); cirrostratus and deep convection. Each is lit from the north-west as the
// globe's relief is (or by a sun, for a globe that has one), and casts a soft shadow; coming
// closer, the reader passes down through them, the highest first.
//
// Prototypes/clouds-web/shader.js is the same, line for line, in GLSL (its desktop twin, which
// tools/playground/clouds/look.cjs and film.cjs draw over the globe's preview): change both together.
#include <metal_stdlib>
using namespace metal;

struct CloudsUniforms {
    float4 frame;       // the drawable's width and height (pixels), pixels a point, the reading scale
    float4 disc;        // the globe's centre x, y and radius (points, y down), the zoom
    float4 rot0;        // world from view, by columns
    float4 rot1;
    float4 rot2;
    float4 time;        // simulated days; this month (0 = January), the next, how far between
    float4 sun;         // xyz the sun (world, towards it); w 1 to light by it, 0 the map's north-west
    float4 seaShadow;   // rgb, and how strong on the sea
    float4 landShadow;  // rgb, and how strong on land
    float4 tone;        // x the clouds' opacity, y how far the night side darkens them
};

constant float PI = 3.14159265;

// ---- noise: pcg3d-hashed gradient noise, quintic (cloudsNoiseTile's tile of it); fbm with a
// rotation between octaves. One octave's sd, as the tile is sampled (0.185), is measured by
// tools/playground/clouds/calibrate.py, which makes the same tile in numpy: so scored, the fbm is
// close to a standard normal, and a cover becomes a threshold.

static uint3 pcg3d(uint3 v) {
    v = v * 1664525u + 1013904223u;
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    v ^= v >> 16u;
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    return v;
}
static float3 hash3(int3 c) { return float3(pcg3d(uint3(c))) * (1.0 / 4294967295.0); }

// One tile of the noise, 16 cells a side wrapping round, six samples a cell: made once, on the GPU,
// when the clouds start; the fragment samples it trilinearly, a lookup an octave, not eight hashes.
kernel void cloudsNoiseTile(texture3d<float, access::write> tile [[texture(0)]],
                            uint3 gid [[thread_position_in_grid]]) {
    const uint n = 96;
    if (gid.x >= n || gid.y >= n || gid.z >= n) return;
    float3 p = (float3(gid) + 0.5) / 6.0;
    float3 i = floor(p), f = p - i;
    float3 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    float v = 0.0;
    for (int c = 0; c < 8; c++) {
        int3 d = int3(c & 1, (c >> 1) & 1, c >> 2);
        int3 cell = (int3(i) + d) % 16;
        float3 g = hash3(cell) * 2.0 - 1.0;
        float w = (d.x == 1 ? u.x : 1.0 - u.x) * (d.y == 1 ? u.y : 1.0 - u.y) * (d.z == 1 ? u.z : 1.0 - u.z);
        v += w * dot(g, f - float3(d));
    }
    tile.write(float4(clamp(v * 0.5 + 0.5, 0.0, 1.0)), gid);
}

constexpr sampler wrapping(filter::linear, address::repeat);
constexpr sampler held(filter::linear, address::clamp_to_edge);
constexpr sampler eastWest(filter::linear, s_address::repeat, t_address::clamp_to_edge);

static float gnoise(texture3d<float> tile, float3 p) { return tile.sample(wrapping, p * (1.0 / 16.0)).r * 2.0 - 1.0; }

constant float3x3 OCT = float3x3(float3(0.00, -0.80, -0.60), float3(0.80, 0.36, -0.48), float3(0.60, -0.48, 0.64));

// fbm as a standard score (x) and how much of its variation is resolved (y): an octave finer than
// about three pixels ('foot': a pixel, in the first octave's lattice cells) is left at its mean, so
// the limb doesn't shimmer, and what it would have added becomes an even haze
static float2 fbm(texture3d<float> tile, float3 p, int octaves, float foot) {
    float s = 0.0, a = 1.0, kept = 0.0, all = 0.0;
    for (int k = 0; k < 7; k++) {
        if (k >= octaves) break;
        float keep = 1.0 - smoothstep(0.3, 0.6, foot);
        if (keep > 0.0) s += a * keep * gnoise(tile, p + 17.13 * float(k));
        kept += a * a * keep * keep; all += a * a;
        a *= 0.5; foot *= 2.03;
        p = OCT * p * 2.03;
    }
    return float2(s / max(0.185 * sqrt(kept), 1e-4), sqrt(kept / all));
}

static float cdf(float z) { return 1.0 / (1.0 + exp(-1.702 * z)); }   // the normal's, near enough

// ---- the measured sky

static float3 skyAt(texture2d<float> sky, float4 time, float3 q, float block) {
    float lon = atan2(q.x, q.z), lat = asin(clamp(q.y, -1.0, 1.0));
    float2 uv = float2((lon / PI + 1.0) * 180.0, (0.5 - lat / PI) * 180.0);
    float2 t0 = float2(fmod(time.y, 4.0) * 362.0 + 1.0, (floor(time.y / 4.0) + block) * 182.0 + 1.0) + uv;
    float2 t1 = float2(fmod(time.z, 4.0) * 362.0 + 1.0, (floor(time.z / 4.0) + block) * 182.0 + 1.0) + uv;
    float2 size = float2(1448.0, 1092.0);
    return mix(sky.sample(held, t0 / size).rgb, sky.sample(held, t1 / size).rgb, time.w);
}

// ---- winds, and the cyclones in the westerlies

static float3 turn(float3 p, float a) { float c = cos(a), s = sin(a); return float3(c * p.x - s * p.z, p.y, s * p.x + c * p.z); }

static float3 rotateAbout(float3 p, float3 k, float a) {   // Rodrigues
    float c = cos(a), s = sin(a);
    return p * c + cross(k, p) * s + k * dot(k, p) * (1.0 - c);
}

// Each hemisphere keeps five cyclones in the westerlies, each six days long: born somewhere between
// 38 and 60 degrees, winding up (anticlockwise in the north, clockwise in the south) and letting go.
// They live in the westerlies' turning frame, so they travel east with it. Invented, as all the
// particular weather here is; where cyclones run is where the measured cloud lets them show.
static float3 cyclones(float3 p, float strength, float days) {
    if (strength <= 0.0) return p;
    for (int k = 0; k < 10; k++) {
        float hemi = k < 5 ? 1.0 : -1.0;
        float life = 6.0;
        float t = days / life + float(k) * 0.2 + (hemi < 0.0 ? 0.1 : 0.0);
        float n = floor(t), age = t - n;
        float3 r = hash3(int3(k, int(n), 977));
        float lat = hemi * (38.0 + 22.0 * r.y) * PI / 180.0, lon = 2.0 * PI * r.x;
        float3 c = float3(cos(lat) * sin(lon), sin(lat), cos(lat) * cos(lon));
        float d = acos(clamp(dot(p, c), -1.0, 1.0));
        float size = 0.09 + 0.07 * r.z;
        if (d > 3.0 * size) continue;
        float wind = 2.6 * smoothstep(0.0, 0.55, age) * (1.0 - smoothstep(0.78, 1.0, age));
        // drawn turning anticlockwise (north): what is here came from clockwise of here
        p = rotateAbout(p, c, -hemi * strength * wind * exp(-d * d / (size * size)));
    }
    return p;
}

struct Layer {
    float cover; float freq; float3 stretch; float warp; int octaves; float seed; float2 belts; float spin;
    float from; float a; float b; float h; float shadow; float lift; float alpha; float wisp; float3 lit; float3 shade;
};

// a layer's noise where its wind has carried it to p: warped by a slower, wider noise (so its shapes
// flow rather than blob), the warp itself changing slowly as the days pass
static float2 shape(texture3d<float> tile, thread const Layer &l, float3 p, int octaves, float foot, float days) {
    if (l.warp <= 0.0) return fbm(tile, p * l.stretch * l.freq + l.seed, min(octaves, l.octaves), foot);
    float3 slow = float3(l.seed, days * 0.15, 0.0);
    float wf = l.freq * 0.35;
    float3 w = float3(gnoise(tile, p * wf + slow), gnoise(tile, p * wf + slow + 31.7), gnoise(tile, p * wf + slow + 57.1));
    return fbm(tile, (p + (l.warp / l.freq) * w / 0.195) * l.stretch * l.freq + l.seed, min(octaves, l.octaves), foot);
}

// a layer's noise at q, carried by its belts' winds (the easterly trades; the westerlies, which
// carry the polar caps too, where the record is least sure: radians a day), each belt turning as
// one and blended where they meet, over the subtropical highs, so no shear piles up
static float2 field(texture3d<float> tile, thread const Layer &l, float3 q, int octaves, float foot, float days) {
    float lat = abs(asin(clamp(q.y, -1.0, 1.0))) * 180.0 / PI;
    float wt = 1.0 - smoothstep(22.0, 34.0, lat), wm = 1.0 - wt;
    float2 s = float2(0.0); float n = 0.0;
    if (wt > 0.001) { s += wt * shape(tile, l, turn(q, -l.belts.x * days), octaves, foot, days); n += wt * wt; }
    if (wm > 0.001) { s += wm * shape(tile, l, cyclones(turn(q, -l.belts.y * days), l.spin, days), octaves, foot, days); n += wm * wm; }
    return float2(s.x / sqrt(n), s.y);
}

// world from view, on a shell 'h' above the ground, the view grown by 'm' about the centre
static bool shell(float3x3 rot, float2 v, float m, float h, thread float3 &q, thread float &facing) {
    v /= m;
    float rc = 1.0 + h, r2 = dot(v, v);
    if (r2 >= rc * rc) return false;
    float z = sqrt(rc * rc - r2);
    q = normalize(rot * float3(v, z));
    facing = z / rc;
    return true;
}

// towards the light along the ground at q: the map's north-west, or a sun's direction
static float3 towardLight(float3 q, float4 sun) {
    if (sun.w > 0.5) {
        float3 t = sun.xyz - q * dot(q, sun.xyz);
        if (dot(t, t) > 1e-8) return normalize(t);
    }
    float3 e = float3(q.z, 0.0, -q.x);
    e = dot(e, e) > 1e-8 ? normalize(e) : float3(1.0, 0.0, 0.0);
    return normalize(cross(q, e) - e);
}

// passing through a layer: from zoom 'a' it grows about the screen's centre (it is nearer the eye
// than the ground), and by 'b' it has gone by
static float growth(float zoom, float a, float b) { return 1.0 / (1.0 - 0.9 * pow(smoothstep(a, b, zoom), 1.6)); }
static float present(float zoom, float a, float b) { return 1.0 - smoothstep(b - 0.4 * (b - a), b, zoom); }

static void over(thread float4 &acc, float3 col, float a) { acc = float4(col * a, a) + acc * (1.0 - a); }

static float cover(float lc, float share) { return 1.0 - exp(lc * share); }

struct CloudsVertex { float4 position [[position]]; };

vertex CloudsVertex cloudsVertex(uint id [[vertex_id]]) {
    float2 p = float2(id == 1 ? 3.0 : -1.0, id == 2 ? 3.0 : -1.0);
    CloudsVertex out;
    out.position = float4(p, 0.0, 1.0);
    return out;
}

fragment float4 cloudsFragment(CloudsVertex in [[stage_in]],
                               constant CloudsUniforms &U [[buffer(0)]],
                               texture2d<float> sky [[texture(0)]],
                               texture2d<float> landMask [[texture(1)]],
                               texture3d<float> tile [[texture(2)]]) {
    // (Metal's pixels run down from the top, as points do)
    float2 pt = in.position.xy / U.frame.z;
    float R = U.disc.z, zoom = U.disc.w, reading = U.frame.w, days = U.time.x;
    float2 v = float2(pt.x - U.disc.x, U.disc.y - pt.y) / R;
    float r2 = dot(v, v);
    if (r2 > 1.08) return float4(0.0);
    float px = 1.0 / (U.frame.z * R);                         // a pixel, in radians, at the centre
    float3x3 rot = float3x3(U.rot0.xyz, U.rot1.xyz, U.rot2.xyz);

    // the measured sky over the ground here
    float3 g = normalize(rot * float3(v, sqrt(max(1.0 - r2, 0.0))));
    float3 A = skyAt(sky, U.time, g, 0.0), B = skyAt(sky, U.time, g, 3.0);
    float C = clamp(A.r, 0.0, 0.995), lc = log(1.0 - C);
    float sLow = A.g, sMid = A.b, sHigh = B.r, sDeep = B.g, sCu = B.b;
    float sThin = max(1.0 - sLow - sMid - sHigh - sDeep - sCu, 0.0);
    float deep = sDeep / max(sHigh + sDeep, 1e-3);            // towers and anvils rather than veils
    // each layer's cover, so that together they cover what was measured: 1 - (1 - C)^share

    float3 white = float3(1.0, 0.995, 0.982), grey = float3(0.72, 0.75, 0.82);
    Layer L[5];
    // cover; noise frequency (cells a radian), stretch, warp, octaves, seed; winds (trades,
    // westerlies: radians a day); cyclones' grip. Shown from zoom 'from'; passed from 'a' to 'b';
    // height (radii); shadow and its offset (points); opacity; wisps (filaments rather than shapes)
    L[0] = Layer{cover(lc, sCu), 40.0, float3(1.0), 0.0, 3, 11.0, float2(-0.11, 0.17), 0.3,
                 1.8, 6.0, 16.0, 0.002, 0.08, 4.0, 0.72, 0.0, white, grey};
    L[1] = Layer{cover(lc, sLow), 4.5, float3(0.85, 1.2, 0.85), 0.5, 7, 23.0, float2(-0.10, 0.19), 0.8,
                 0.0, 2.8, 5.2, 0.004, 0.14, 4.0, 0.80, 0.0, white, grey};
    L[2] = Layer{cover(lc, sMid), 2.6, float3(0.6, 1.7, 0.6), 0.5, 7, 37.0, float2(-0.05, 0.31), 1.2,
                 0.0, 2.0, 4.0, 0.007, 0.12, 6.5, 0.78, 0.0, float3(0.99, 0.988, 0.98), grey};
    L[3] = Layer{cover(lc, sThin), 5.0, float3(0.35, 2.6, 0.35), 0.5, 7, 41.0, float2(-0.06, 0.46), 0.8,
                 0.0, 1.6, 3.2, 0.010, 0.0, 9.0, 0.42, 1.0, white, grey};
    L[4] = Layer{cover(lc, sHigh + sDeep), mix(3.0, 7.0, deep), mix(float3(0.45, 2.2, 0.45), float3(1.0), deep), 0.5, 7, 53.0,
                 float2(-0.06, 0.46), 0.6,
                 0.0, 1.5, 3.2, 0.012, mix(0.06, 0.22, deep), 10.0, mix(0.55, 0.92, deep), 0.0, float3(1.0), grey};

    float4 acc = float4(0.0);

    // shadows on the ground, under every layer, cast away from the light; soft
    if (r2 < 1.0) {
        float lit = 1.0;
        float3 toL = towardLight(g, U.sun);
        for (int i = 0; i < 5; i++) {
            thread const Layer &l = L[i];
            // (a layer's shadow goes as it passes: below it, the ground is in its even shade, not patches)
            float on = present(zoom, l.a, l.b) * (l.from == 0.0 ? 1.0 : smoothstep(l.from, l.from + 1.0, zoom));
            if (l.cover < 0.01 || l.shadow <= 0.0 || on < 0.01) continue;
            float3 s = normalize(g + l.lift * reading * sqrt(zoom) / R * toL);
            float2 f = field(tile, l, s, 4, px * l.freq * 1.5, days);
            float edge = 1.0 - l.cover;
            float d = mix(l.cover, smoothstep(edge - 0.06, edge + 0.12, cdf(f.x)), f.y);
            lit *= 1.0 - on * l.shadow * d * (1.0 - 0.6 * l.cover);
        }
        // on the sea a cool blue-grey; on land the globe's own shadow colour (its drop shadow's, 6E6352)
        float2 uv = float2(atan2(g.x, g.z) / PI + 1.0, 1.0 - asin(clamp(g.y, -1.0, 1.0)) * 2.0 / PI) * 0.5;
        float land = landMask.sample(eastWest, uv).r;
        float strength = mix(U.seaShadow.w, U.landShadow.w, land);
        over(acc, mix(U.seaShadow.rgb, U.landShadow.rgb, land), (1.0 - lit) * strength);
    }

    // the layers, low to high, lit from the north-west (or the sun)
    for (int i = 0; i < 5; i++) {
        thread const Layer &l = L[i];
        if (l.cover < 0.01) continue;
        float shown = present(zoom, l.a, l.b) * (l.from == 0.0 ? 1.0 : smoothstep(l.from, l.from + 1.0, zoom));
        if (shown < 0.004) continue;
        float m = growth(zoom, l.a, l.b);
        float3 q; float facing;
        if (!shell(rot, v, m, l.h, q, facing)) continue;
        float foot = px * l.freq / (m * max(facing, 0.08));
        float2 f = field(tile, l, q, 6, foot, days);
        float u = cdf(f.x), edge = 1.0 - l.cover, t = u - edge;  // t > 0 inside
        // (the view's y runs up, the pixels' down)
        float2 grad = float2(dfdx(u), -dfdy(u));
        float slope = max(length(grad), 1e-5);
        float2 lv = normalize((transpose(rot) * towardLight(q, U.sun)).xy + 1e-6);
        float d;
        float3 col;
        if (l.wisp > 0.0) {
            // thin cloud aloft: where it is, fine filaments along the wind, soft
            float covered = mix(l.cover, smoothstep(-0.18, 0.18, t), f.y);
            float3 qs = turn(q, -l.belts.y * days) * float3(1.0, 4.0, 1.0) * 26.0 + l.seed;
            float thread_ = 1.0 - abs(gnoise(tile, qs) + 0.5 * gnoise(tile, OCT * qs * 2.03)) * 2.6;
            d = covered * smoothstep(0.15, 0.85, thread_) * (1.0 - smoothstep(0.35, 0.6, px * 104.0 / (m * max(facing, 0.08))));
            col = l.lit;
        } else {
            // as from space: thin and grey at the edges, thick and white within, the tops lit like relief
            float feather = max(slope * 1.2, i == 0 ? 0.035 : 0.07);   // cumulus: crisper
            d = mix(l.cover, smoothstep(-feather, feather, t), f.y);
            if (d < 0.002) continue;
            float body = smoothstep(0.0, 0.3, t);
            float2 rg = float2(dfdx(f.x), -dfdy(f.x));
            float bumps = i == 0 ? 6.0 : 2.2;                       // cumulus are all tops
            float relief = clamp(dot(rg, lv) * bumps / U.frame.z, -1.0, 1.0) * f.y;   // > 0 faces away
            col = mix(l.lit, l.shade, clamp(0.34 * (1.0 - body) + 0.34 * max(relief, 0.0), 0.0, 1.0));
            col = mix(col, float3(1.0), 0.6 * max(-relief, 0.0) * body);
        }
        // by a sun, the night side's clouds darken (a globe without one has no night)
        if (U.sun.w > 0.5) col *= mix(1.0 - U.tone.y, 1.0, smoothstep(-0.12, 0.08, dot(q, U.sun.xyz)));
        float a = U.tone.x * l.alpha * d * shown * smoothstep(0.0, 0.12, facing);
        over(acc, col, a);
    }
    return acc;
}
