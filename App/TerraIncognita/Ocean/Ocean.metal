// The ocean, one pixel at a time: a full-screen triangle over the globe (OceanLayer.swift), in a
// layer of its own between the globe's drawing and the clouds.
//
// Measured: how the sea's surface usually moves in the globe's month (the NOAA Global Drifter
// Program's climatology: OceanCurrents.png and OceanStream.png, tools/ocean/build.py). Drawn from it:
// the streamlines, the contours of the month's stream function, which are the water's paths and lie
// closer where it runs faster; light running along them, carried at a pace that follows the current
// (much faster than the water: a drawn pace, not to scale). Drawn, not measured: the sheen of the
// sky on the water, its glitter close in, the surf at the coasts, and the rings where it is touched.
// The globe's own maps say where the sea is (its coast, Globe/geo.png) and where the month's ice
// lies (ground/ice-MM.png).
//
// tools/ocean/web/shader.js is the same, line for line, in GLSL (its desktop twin, which
// tools/ocean/look.cjs draws over the globe's preview): change both together.
#include <metal_stdlib>
using namespace metal;

struct OceanUniforms {
    float4 frame;       // the drawable's width and height (pixels), pixels a point, the reading scale
    float4 disc;        // the globe's centre x, y and radius (points, y down), the zoom
    float4 rot0;        // world from view, by columns
    float4 rot1;
    float4 rot2;
    float4 time;        // the sea's own seconds (for the touches' rings); this month (0 = January), the
                        // next, how far between
    float4 tone;        // x how much of it shows; y 1 for motion, 0 under Reduce Motion; z the drawn
                        // motion's seconds (held under Reduce Motion)
    float4 ripple[8];   // four touches: centre (world) and when (the sea's seconds), then strength
};

constant float OCEAN_PI = 3.14159265;

// ---- noise: the clouds' tile of gradient noise (16 cells a side, wrapping round; cloudsNoiseTile
// makes it)

constexpr sampler oceanWrap(filter::linear, address::repeat);
constexpr sampler oceanHeld(filter::linear, address::clamp_to_edge);
constexpr sampler oceanEastWest(filter::linear, s_address::repeat, t_address::clamp_to_edge);

static float gnoise(texture3d<float> tile, float3 p) { return tile.sample(oceanWrap, p * (1.0 / 16.0)).r * 2.0 - 1.0; }

static float hash13(float3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return fract((p.x + p.y) * p.z);
}

// ---- the measured sea

// the month's cell of a sheet: its tile's origin, in texels
static float2 tileOf(float m) { return float2(fmod(m, 4.0) * 722.0, floor(m / 4.0) * 362.0); }

// eastward and northward current (m/s) and how measured, for month m at uv
static float3 currentAt(texture2d<float> currents, float m, float2 uv) {
    float2 st = (tileOf(m) + 1.0 + uv * float2(720.0, 360.0)) / float2(2888.0, 1086.0);
    float3 c = currents.sample(oceanHeld, st).rgb;
    float2 s = (c.rg * 255.0 - 128.0) / 127.0;
    return float3(sign(s) * s * s * 2.0, c.b);
}

// the stream function (m/s radians) for month m at uv: four texels, their bytes joined, then mixed
static float streamTexel(texture2d<float> stream, int2 t) {
    float2 b = floor(stream.read(uint2(t)).rg * 255.0 + 0.5);
    return (b.x * 256.0 + b.y) / 65535.0 * 0.4 - 0.2;
}
static float streamAt(texture2d<float> stream, float m, float2 uv) {
    float2 p = uv * float2(720.0, 360.0) + 0.5;         // (the border's texel, less half a texel)
    float2 i = floor(p), f = p - i;
    int2 t = int2(tileOf(m) + i);
    float a = streamTexel(stream, t), b = streamTexel(stream, t + int2(1, 0));
    float c = streamTexel(stream, t + int2(0, 1)), d = streamTexel(stream, t + int2(1, 1));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// ---- drawing

static void over(thread float4 &acc, float3 col, float a) { acc = float4(col * a, a) + acc * (1.0 - a); }

static float sq(float x) { return x * x; }

// the distance in pixels from a contour of q (one a unit; 'fw' is fwidth(q)), and how much of the set
// shows: all of it, unless its lines would crowd closer than a few pixels
static float2 contour(float q, float fw) {
    return float2(abs(q - floor(q + 0.5)) / max(fw, 1e-6), 1.0 - smoothstep(0.16, 0.36, fw));
}

struct OceanVertex { float4 position [[position]]; };

vertex OceanVertex oceanVertex(uint id [[vertex_id]]) {
    float2 p = float2(id == 1 ? 3.0 : -1.0, id == 2 ? 3.0 : -1.0);
    OceanVertex out;
    out.position = float4(p, 0.0, 1.0);
    return out;
}

fragment float4 oceanFragment(OceanVertex in [[stage_in]],
                              constant OceanUniforms &U [[buffer(0)]],
                              texture2d<float> currents [[texture(0)]],
                              texture2d<float> stream [[texture(1)]],
                              texture2d<float> geoTex [[texture(2)]],
                              texture2d<float> iceTex [[texture(3)]],
                              texture3d<float> tile [[texture(4)]]) {
    // (Metal's pixels run down from the top, as points do)
    float2 pt = in.position.xy / U.frame.z;
    float uPx = U.frame.z;
    // (the drawn motion's clock, which Reduce Motion holds; a touch's ring keeps time regardless)
    float R = U.disc.z, zoom = U.disc.w, reading = U.frame.w, t = U.time.x, tm = U.tone.z;
    float2 v = float2(pt.x - U.disc.x, U.disc.y - pt.y) / R;
    float r2 = dot(v, v);
    // (derivatives are taken before any pixel leaves, so neighbours agree)
    float z = sqrt(max(1.0 - r2, 0.0));
    float3x3 rot = float3x3(U.rot0.xyz, U.rot1.xyz, U.rot2.xyz);
    float3 g = normalize(rot * float3(v, z));
    float lat = asin(clamp(g.y, -1.0, 1.0)), lon = atan2(g.x, g.z);
    float2 uv = float2(lon / (2.0 * OCEAN_PI) + 0.5, 0.5 - lat / OCEAN_PI);

    // where the sea is: the globe's coast (land positive, in its texels), the month's ice
    float4 geo = geoTex.sample(oceanEastWest, uv);
    float fine = (geo.r * 255.0 - 128.0) / 8.0, coarse = (geo.g * 255.0 - 128.0) * 2.0;
    float d = abs(coarse) < 15.0 ? fine : coarse;
    float fwD = fwidth(d);
    float sea = 1.0 - smoothstep(-0.5, 0.5, d / max(fwD, 1e-5));
    // points out to sea from the coast, near it (the fine distance, its texels a little longer
    // north-south than east-west on the ground)
    float shore = -fine * (OCEAN_PI / 2048.0) * mix(1.0, cos(lat), 0.5) * R;
    float ice = smoothstep(0.30, 0.55, iceTex.sample(oceanEastWest, uv).r);

    // the month's water: its current and stream function, between this month and the next
    float3 c0 = currentAt(currents, U.time.y, uv), c1 = currentAt(currents, U.time.z, uv);
    float3 cur = mix(c0, c1, U.time.w);
    float psi = mix(streamAt(stream, U.time.y, uv), streamAt(stream, U.time.z, uv), U.time.w);
    float speed = length(cur.xy), measured = cur.z;
    float3 east = normalize(float3(g.z, 0.0, -g.x) + float3(1e-6, 0.0, 0.0));
    float3 north = cross(g, east);
    float3 flow = east * cur.x + north * cur.y;          // m/s, along the ground
    // the stream function's slope on the screen, a pixel's worth: from the current, which is
    // continuous (the stream function's own bilinear slopes jump at every cell), projected exactly
    float3 slope = east * cur.y - north * cur.x;          // per radian along the ground
    float3 ax = rot * float3(1.0, 0.0, -v.x / max(z, 0.05)), ay = rot * float3(0.0, 1.0, -v.y / max(z, 0.05));
    float fwPsi = max(length(float2(dot(slope, ax), dot(slope, ay))), 1e-9) / (R * uPx);

    if (r2 >= 1.0 || sea <= 0.0) return float4(0.0);
    float water = sea * (1.0 - ice) * smoothstep(0.0, 0.12, z) * U.tone.x;
    if (water <= 0.0) return float4(0.0);
    float4 acc = float4(0.0);

    // -- the sheen: the sky's light on the water, from the upper left, soft and broad
    float3 toLight = normalize(float3(-0.42, 0.55, 0.72));
    float3 n = float3(v, z);                              // the surface's normal, in the view
    float3 h = normalize(toLight + float3(0.0, 0.0, 1.0));
    float sheen = pow(max(dot(n, h), 0.0), 18.0) * (1.0 - smoothstep(1.5, 3.0, zoom));
    over(acc, float3(1.0), 0.10 * sheen * water);

    // -- the streamlines: the stream function's contours, about 'gap' points apart where the water
    // runs at 'ref', in octaves fixed to the globe (so they stay put as it zooms) and blended between;
    // each line's distance, and a halo round it for the light to travel in
    float gap = 9.0 * reading, ref = 0.5;
    float level = log2(gap * ref / R);
    float k = floor(level), blend = level - k;
    float delta = exp2(k);
    float2 sparse = contour(psi / (2.0 * delta), fwPsi / (2.0 * delta));
    float2 dense = contour(psi / delta, fwPsi / delta);
    dense.y *= smoothstep(0.0, 0.6, 1.0 - blend);
    float lineW = 0.75 * uPx * reading, haloW = 4.0 * uPx * reading;
    float core = max(sparse.y * (1.0 - smoothstep(lineW * 0.5, lineW * 0.5 + 1.0, sparse.x)),
                     dense.y * (1.0 - smoothstep(lineW * 0.5, lineW * 0.5 + 1.0, dense.x)));
    float halo = max(sparse.y * exp(-sparse.x * sparse.x / (haloW * haloW)), dense.y * exp(-dense.x * dense.x / (haloW * haloW)));
    float running = smoothstep(0.04, 0.40, speed) * measured;
    over(acc, float3(0.20, 0.42, 0.62), 0.22 * core * running * water);

    // -- light running along them: noise carried by the current (two phases, each starting afresh
    // as the other is at its height), fixed to the globe in two octaves about 'pulse' points across;
    // each bright place trails a tail upstream, so a still picture shows which way the water goes
    float pulse = 46.0 * reading;
    float plevel = log2(R / pulse);
    float pk = floor(plevel), pb = plevel - pk;
    float pace = 26.0 * reading / R;                     // radians a second for a metre a second
    float3 along = flow / max(speed, 1e-4);
    float glow = 0.0, w2 = 0.0;
    for (int o = 0; o < 2; o++) {
        float F = exp2(pk + float(o));
        float wo = o == 0 ? 1.0 - pb : pb;
        for (int ph = 0; ph < 2; ph++) {
            float cyc = tm / 5.0 + 0.5 * float(ph);
            float age = fract(cyc);
            float wp = 1.0 - abs(2.0 * age - 1.0);
            float3 p = (g - flow * pace * age * 5.0) * F + 37.0 * floor(cyc) + 11.0 * float(o);
            float tail = 0.0;
            for (int s = 0; s < 3; s++) tail += gnoise(tile, p + along * (0.22 * float(s))) * (1.0 - 0.3 * float(s));
            glow += wo * wp * tail / 2.1;
            w2 += wo * wo * wp * wp;
        }
    }
    // (as a standard score: mixed, the samples vary less than one does, whose deviation is 0.185, and
    // the tail's three less still; quieter at the whole Earth)
    glow = smoothstep(0.3, 1.5, glow / (0.185 * 0.86 * sqrt(w2))) * running * mix(0.45, 1.0, smoothstep(1.0, 2.2, zoom));
    over(acc, float3(0.96, 0.99, 1.0), (0.24 * halo + 0.6 * core) * glow * water);

    // -- close in, the wind on the water: patches of glitter drifting slowly, each glint a moment's
    // flash at a place of its own (a small four-pointed star on the screen), in cells about 'cell'
    // points across fixed to the globe; none held still
    float near = smoothstep(2.6, 4.2, zoom) * U.tone.y;
    if (near > 0.0) {
        float gusts = smoothstep(-0.4, 1.1, gnoise(tile, g * R / (420.0 * reading) + float3(0.0, tm * 0.02, tm * 0.03)) / 0.185);
        float cell = 15.0 * reading;
        float glevel = log2(R / cell);
        float gk = floor(glevel), gb = glevel - gk;
        float glint = 0.0;
        for (int o = 0; o < 2; o++) {
            float F = exp2(gk + float(o));
            float wo = o == 0 ? 1.0 - gb : gb;
            float3 c = floor(g * F);
            float id = hash13(c + 0.37);
            float3 at = normalize(c + 0.25 + 0.5 * float3(hash13(c + 1.7), hash13(c + 5.3), hash13(c + 9.1)));
            float2 off = (v - (transpose(rot) * at).xy) * R / reading;   // points from the glint, on screen
            float period = 2.5 + 3.5 * hash13(c + 13.9);
            float flash = pow(max(sin(6.2832 * (tm / period + id)), 0.0), 32.0);
            float there = step(id, 0.5 * gusts);
            float star = exp(-dot(off, off) / 0.45) + 0.28 * exp(-off.x * off.x / 3.5 - off.y * off.y / 0.1)
                       + 0.28 * exp(-off.y * off.y / 3.5 - off.x * off.x / 0.1);
            glint += wo * there * flash * star;
        }
        over(acc, float3(1.0), min(glint, 1.0) * 0.95 * near * water * smoothstep(0.0, 6.0, shore));
    }

    // -- close in, the surf: swells coming in to the coast, a set at a time, and a line of foam where
    // they break, within about 'reach' points of the shore
    float surfing = smoothstep(3.2, 5.0, zoom);
    if (surfing > 0.0 && shore < 24.0 * reading && fine > -15.5) {
        float reach = 18.0 * reading, spacing = 5.5 * reading;
        float set = gnoise(tile, g * R / (220.0 * reading) + float3(tm * 0.05)) / 0.185;
        float arriving = tm / 3.4 + 0.5 * set;
        float phase = shore / spacing + arriving;
        float swell = exp(-sq((fract(phase) - 0.5) * spacing / (0.55 * reading)));
        float fade = (1.0 - smoothstep(0.3 * reach, reach, shore)) * smoothstep(1.5, 3.5, shore);
        float breaking = 0.5 + 0.5 * cos(6.2832 * arriving);
        float lace = 0.6 + 0.4 * smoothstep(-0.8, 0.8, gnoise(tile, g * R / (5.0 * reading)) / 0.185);
        float foam = exp(-sq(max(shore - 0.3 * reading, 0.0) / ((0.8 + 1.1 * breaking) * reading))) * lace;
        over(acc, float3(1.0), (0.28 * swell * fade + 0.8 * foam) * surfing * water);
    }

    // -- touched: rings spreading on the water from each touch, a crest of light and its trough (held
    // still, a ring that fades where it is)
    for (int i = 0; i < 4; i++) {
        float4 at = U.ripple[2 * i], how = U.ripple[2 * i + 1];
        float age = t - at.w;
        if (how.x <= 0.0 || age < 0.0 || age > 5.0) continue;
        float dPt = acos(clamp(dot(g, at.xyz), -1.0, 1.0)) * R;
        float r = 34.0 * reading * pow(mix(0.8, age, U.tone.y), 0.75);
        float light = 0.0, shade = 0.0;
        for (int j = 0; j < 3; j++) {
            float x = (dPt - (r - float(j) * 12.0 * reading)) / reading;
            float a = (1.0 - 0.28 * float(j)) * step(0.0, r - float(j) * 12.0 * reading);
            light += a * exp(-sq((x + 1.4) / 2.0));
            shade += a * exp(-sq((x - 2.6) / 2.4));
        }
        float strength = how.x * exp(-age / 1.5) * smoothstep(0.0, 0.12, age) / (1.0 + r / (90.0 * reading));
        over(acc, float3(0.13, 0.32, 0.50), 0.20 * shade * strength * water);
        over(acc, float3(1.0), 0.65 * light * strength * water);
    }

    return acc;
}
