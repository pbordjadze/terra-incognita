// The ocean, one pixel at a time: a full-screen triangle over the globe, in a layer of its own between
// the globe's drawing and the clouds (App/TerraIncognita/Ocean/Ocean.metal is the same, line for
// line: change both together).
//
// Measured: how the sea's surface usually moves in the globe's month (the NOAA Global Drifter
// Program's climatology: OceanCurrents.png and OceanStream.png, tools/ocean/build.py). Drawn from it:
// the streamlines, the contours of the month's stream function, which are the water's paths and lie
// closer where it runs faster; light running along them, carried at a pace that follows the current
// (much faster than the water: a drawn pace, not to scale). Drawn, not measured: the sheen of the
// sky on the water, its glitter close in, the surf at the coasts, and the rings where it is touched.
// The globe's own maps say where the sea is (its coast, Globe/geo.png) and where the month's ice
// lies (ground/ice-MM.png).
window.OCEAN_FRAG = `#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;
precision highp sampler3D;

uniform vec2 uSize;        // the drawable's width and height (pixels)
uniform float uPx;         // pixels a point
uniform vec3 uDisc;        // the globe's centre x, y and radius (points, y down)
uniform mat3 uRot;         // world from view
uniform float uZoom;
uniform float uReading;    // the reading scale (Globe/type.json)
uniform float uTime;       // the sea's own seconds, for the touches' rings
uniform vec3 uMonth;       // this month (0 = January), the next, how far between
uniform vec4 uTone;        // x how much of it shows; y 1 for motion, 0 under Reduce Motion; z the
                           // drawn motion's seconds (held under Reduce Motion)
uniform vec4 uRipple[8];   // four touches: centre (world) and when (uTime), then strength
uniform sampler2D uCurrents;
uniform sampler2D uStream;
uniform sampler2D uGeo;
uniform sampler2D uIce;
uniform sampler3D uNoise;
out vec4 frag;

const float PI = 3.14159265;

// ---- noise: the clouds' tile of gradient noise (16 cells a side, wrapping round)

float gnoise(vec3 p) { return texture(uNoise, p * (1.0 / 16.0)).r * 2.0 - 1.0; }

float hash13(vec3 p) {
    p = fract(p * 0.1031);
    p += dot(p, p.zyx + 31.32);
    return fract((p.x + p.y) * p.z);
}

// ---- the measured sea

// the month's cell of a sheet: its tile's origin, in texels
vec2 tileOf(float m) { return vec2(mod(m, 4.0) * 722.0, floor(m / 4.0) * 362.0); }

// eastward and northward current (m/s) and how measured, for month m at uv
vec3 currentAt(float m, vec2 uv) {
    vec2 st = (tileOf(m) + 1.0 + uv * vec2(720.0, 360.0)) / vec2(2888.0, 1086.0);
    vec3 c = texture(uCurrents, st).rgb;
    vec2 s = (c.rg * 255.0 - 128.0) / 127.0;
    return vec3(sign(s) * s * s * 2.0, c.b);
}

// the stream function (m/s radians) for month m at uv: four texels, their bytes joined, then mixed
float streamTexel(ivec2 t) {
    vec2 b = floor(texelFetch(uStream, t, 0).rg * 255.0 + 0.5);
    return (b.x * 256.0 + b.y) / 65535.0 * 0.4 - 0.2;
}
float streamAt(float m, vec2 uv) {
    vec2 p = uv * vec2(720.0, 360.0) + 0.5;           // (the border's texel, less half a texel)
    vec2 i = floor(p), f = p - i;
    ivec2 t = ivec2(tileOf(m) + i);
    float a = streamTexel(t), b = streamTexel(t + ivec2(1, 0));
    float c = streamTexel(t + ivec2(0, 1)), d = streamTexel(t + ivec2(1, 1));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// ---- drawing

void over(inout vec4 acc, vec3 col, float a) { acc = vec4(col * a, a) + acc * (1.0 - a); }

float sq(float x) { return x * x; }

// the distance in pixels from a contour of q (one a unit; 'fw' is fwidth(q)), and how much of the set
// shows: all of it, unless its lines would crowd closer than a few pixels
vec2 contour(float q, float fw) {
    return vec2(abs(q - floor(q + 0.5)) / max(fw, 1e-6), 1.0 - smoothstep(0.16, 0.36, fw));
}

void main() {
    vec2 pt = vec2(gl_FragCoord.x, uSize.y - gl_FragCoord.y) / uPx;
    // (the drawn motion's clock, which Reduce Motion holds; a touch's ring keeps time regardless)
    float R = uDisc.z, zoom = uZoom, reading = uReading, t = uTime, tm = uTone.z;
    vec2 v = vec2(pt.x - uDisc.x, uDisc.y - pt.y) / R;
    float r2 = dot(v, v);
    // (derivatives are taken before any pixel leaves, so neighbours agree)
    float z = sqrt(max(1.0 - r2, 0.0));
    vec3 g = normalize(uRot * vec3(v, z));
    float lat = asin(clamp(g.y, -1.0, 1.0)), lon = atan(g.x, g.z);
    vec2 uv = vec2(lon / (2.0 * PI) + 0.5, 0.5 - lat / PI);

    // where the sea is: the globe's coast (land positive, in its texels), the month's ice, the depth
    vec4 geo = texture(uGeo, uv);
    float fine = (geo.r * 255.0 - 128.0) / 8.0, coarse = (geo.g * 255.0 - 128.0) * 2.0;
    float d = abs(coarse) < 15.0 ? fine : coarse;
    float fwD = fwidth(d);
    float sea = 1.0 - smoothstep(-0.5, 0.5, d / max(fwD, 1e-5));
    // points out to sea from the coast, near it (the fine distance, its texels a little longer
    // north-south than east-west on the ground)
    float shore = -fine * (PI / 2048.0) * mix(1.0, cos(lat), 0.5) * R;
    float ice = smoothstep(0.30, 0.55, texture(uIce, uv).r);

    // the month's water: its current and stream function, between this month and the next
    vec3 c0 = currentAt(uMonth.x, uv), c1 = currentAt(uMonth.y, uv);
    vec3 cur = mix(c0, c1, uMonth.z);
    float psi = mix(streamAt(uMonth.x, uv), streamAt(uMonth.y, uv), uMonth.z);
    float speed = length(cur.xy), measured = cur.z;
    vec3 east = normalize(vec3(g.z, 0.0, -g.x) + vec3(1e-6, 0.0, 0.0));
    vec3 north = cross(g, east);
    vec3 flow = east * cur.x + north * cur.y;            // m/s, along the ground
    // the stream function's slope on the screen, a pixel's worth: from the current, which is
    // continuous (the stream function's own bilinear slopes jump at every cell), projected exactly
    vec3 slope = east * cur.y - north * cur.x;            // per radian along the ground
    vec3 ax = uRot * vec3(1.0, 0.0, -v.x / max(z, 0.05)), ay = uRot * vec3(0.0, 1.0, -v.y / max(z, 0.05));
    float fwPsi = max(length(vec2(dot(slope, ax), dot(slope, ay))), 1e-9) / (R * uPx);

    if (r2 >= 1.0 || sea <= 0.0) { frag = vec4(0.0); return; }
    float water = sea * (1.0 - ice) * smoothstep(0.0, 0.12, z) * uTone.x;
    if (water <= 0.0) { frag = vec4(0.0); return; }
    vec4 acc = vec4(0.0);

    // -- the sheen: the sky's light on the water, from the upper left, soft and broad
    vec3 toLight = normalize(vec3(-0.42, 0.55, 0.72));
    vec3 n = vec3(v, z);                                  // the surface's normal, in the view
    vec3 h = normalize(toLight + vec3(0.0, 0.0, 1.0));
    float sheen = pow(max(dot(n, h), 0.0), 18.0) * (1.0 - smoothstep(1.5, 3.0, zoom));
    over(acc, vec3(1.0), 0.10 * sheen * water);

    // -- the streamlines: the stream function's contours, about 'gap' points apart where the water
    // runs at 'ref', in octaves fixed to the globe (so they stay put as it zooms) and blended between;
    // each line's distance, and a halo round it for the light to travel in
    float gap = 9.0 * reading, ref = 0.5;
    float level = log2(gap * ref / R);
    float k = floor(level), blend = level - k;
    float delta = exp2(k);
    vec2 sparse = contour(psi / (2.0 * delta), fwPsi / (2.0 * delta));
    vec2 dense = contour(psi / delta, fwPsi / delta);
    dense.y *= smoothstep(0.0, 0.6, 1.0 - blend);
    float lineW = 0.75 * uPx * reading, haloW = 4.0 * uPx * reading;
    float core = max(sparse.y * (1.0 - smoothstep(lineW * 0.5, lineW * 0.5 + 1.0, sparse.x)),
                     dense.y * (1.0 - smoothstep(lineW * 0.5, lineW * 0.5 + 1.0, dense.x)));
    float halo = max(sparse.y * exp(-sparse.x * sparse.x / (haloW * haloW)), dense.y * exp(-dense.x * dense.x / (haloW * haloW)));
    float running = smoothstep(0.04, 0.40, speed) * measured;
    over(acc, vec3(0.20, 0.42, 0.62), 0.22 * core * running * water);

    // -- light running along them: noise carried by the current (two phases, each starting afresh
    // as the other is at its height), fixed to the globe in two octaves about 'pulse' points across;
    // each bright place trails a tail upstream, so a still picture shows which way the water goes
    float pulse = 46.0 * reading;
    float plevel = log2(R / pulse);
    float pk = floor(plevel), pb = plevel - pk;
    float pace = 26.0 * reading / R;                     // radians a second for a metre a second
    vec3 along = flow / max(speed, 1e-4);
    float glow = 0.0, w2 = 0.0;
    for (int o = 0; o < 2; o++) {
        float F = exp2(pk + float(o));
        float wo = o == 0 ? 1.0 - pb : pb;
        for (int ph = 0; ph < 2; ph++) {
            float cyc = tm / 5.0 + 0.5 * float(ph);
            float age = fract(cyc);
            float wp = 1.0 - abs(2.0 * age - 1.0);
            vec3 p = (g - flow * pace * age * 5.0) * F + 37.0 * floor(cyc) + 11.0 * float(o);
            float tail = 0.0;
            for (int s = 0; s < 3; s++) tail += gnoise(p + along * (0.22 * float(s))) * (1.0 - 0.3 * float(s));
            glow += wo * wp * tail / 2.1;
            w2 += wo * wo * wp * wp;
        }
    }
    // (as a standard score: mixed, the samples vary less than one does, whose deviation is 0.185, and
    // the tail's three less still; quieter at the whole Earth)
    glow = smoothstep(0.3, 1.5, glow / (0.185 * 0.86 * sqrt(w2))) * running * mix(0.45, 1.0, smoothstep(1.0, 2.2, zoom));
    over(acc, vec3(0.96, 0.99, 1.0), (0.24 * halo + 0.6 * core) * glow * water);

    // -- close in, the wind on the water: patches of glitter drifting slowly, each glint a moment's
    // flash at a place of its own (a small four-pointed star on the screen), in cells about 'cell'
    // points across fixed to the globe; none held still
    float near = smoothstep(2.6, 4.2, zoom) * uTone.y;
    if (near > 0.0) {
        float gusts = smoothstep(-0.4, 1.1, gnoise(g * R / (420.0 * reading) + vec3(0.0, tm * 0.02, tm * 0.03)) / 0.185);
        float cell = 15.0 * reading;
        float glevel = log2(R / cell);
        float gk = floor(glevel), gb = glevel - gk;
        float glint = 0.0;
        for (int o = 0; o < 2; o++) {
            float F = exp2(gk + float(o));
            float wo = o == 0 ? 1.0 - gb : gb;
            vec3 c = floor(g * F);
            float id = hash13(c + 0.37);
            vec3 at = normalize(c + 0.25 + 0.5 * vec3(hash13(c + 1.7), hash13(c + 5.3), hash13(c + 9.1)));
            vec2 off = (v - (transpose(uRot) * at).xy) * R / reading;   // points from the glint, on screen
            float period = 2.5 + 3.5 * hash13(c + 13.9);
            float flash = pow(max(sin(6.2832 * (tm / period + id)), 0.0), 32.0);
            float there = step(id, 0.5 * gusts);
            float star = exp(-dot(off, off) / 0.45) + 0.28 * exp(-off.x * off.x / 3.5 - off.y * off.y / 0.1)
                       + 0.28 * exp(-off.y * off.y / 3.5 - off.x * off.x / 0.1);
            glint += wo * there * flash * star;
        }
        over(acc, vec3(1.0), min(glint, 1.0) * 0.95 * near * water * smoothstep(0.0, 6.0, shore));
    }

    // -- close in, the surf: swells coming in to the coast, a set at a time, and a line of foam where
    // they break, within about 'reach' points of the shore
    float surfing = smoothstep(3.2, 5.0, zoom);
    if (surfing > 0.0 && shore < 24.0 * reading && fine > -15.5) {
        float reach = 18.0 * reading, spacing = 5.5 * reading;
        float set = gnoise(g * R / (220.0 * reading) + vec3(tm * 0.05)) / 0.185;
        float arriving = tm / 3.4 + 0.5 * set;
        float phase = shore / spacing + arriving;
        float swell = exp(-sq((fract(phase) - 0.5) * spacing / (0.55 * reading)));
        float fade = (1.0 - smoothstep(0.3 * reach, reach, shore)) * smoothstep(1.5, 3.5, shore);
        float breaking = 0.5 + 0.5 * cos(6.2832 * arriving);
        float lace = 0.6 + 0.4 * smoothstep(-0.8, 0.8, gnoise(g * R / (5.0 * reading)) / 0.185);
        float foam = exp(-sq(max(shore - 0.3 * reading, 0.0) / ((0.8 + 1.1 * breaking) * reading))) * lace;
        over(acc, vec3(1.0), (0.28 * swell * fade + 0.8 * foam) * surfing * water);
    }

    // -- touched: rings spreading on the water from each touch, a crest of light and its trough (held
    // still, a ring that fades where it is)
    for (int i = 0; i < 4; i++) {
        vec4 at = uRipple[2 * i], how = uRipple[2 * i + 1];
        float age = t - at.w;
        if (how.x <= 0.0 || age < 0.0 || age > 5.0) continue;
        float dPt = acos(clamp(dot(g, at.xyz), -1.0, 1.0)) * R;
        float r = 34.0 * reading * pow(mix(0.8, age, uTone.y), 0.75);
        float light = 0.0, shade = 0.0;
        for (int j = 0; j < 3; j++) {
            float x = (dPt - (r - float(j) * 12.0 * reading)) / reading;
            float a = (1.0 - 0.28 * float(j)) * step(0.0, r - float(j) * 12.0 * reading);
            light += a * exp(-sq((x + 1.4) / 2.0));
            shade += a * exp(-sq((x - 2.6) / 2.4));
        }
        float strength = how.x * exp(-age / 1.5) * smoothstep(0.0, 0.12, age) / (1.0 + r / (90.0 * reading));
        over(acc, vec3(0.13, 0.32, 0.50), 0.20 * shade * strength * water);
        over(acc, vec3(1.0), 0.65 * light * strength * water);
    }

    frag = acc;
}
`;
