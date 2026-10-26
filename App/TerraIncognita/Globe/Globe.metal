#include <metal_stdlib>
using namespace metal;

// The globe and the page around it, one fragment per pixel: the orthographic projection inverted,
// the map's distance fields sampled, the lines drawn a constant width on screen. tools/globe/preview.py
// is the same, line for line (its palette and constants), and draws it without a phone.

struct GlobeUniforms {
    float4 u0;          // drawable size (px), the globe's centre (px)
    float4 u1;          // radius (px), pixels per point, zoom, the gold ring's strength
    float4 u2;          // the selected country (1 + its index; 0: none), its highlight, the style
                        // (0 Wikipedia's colours, 1 the Persian; between them while it changes)
    float4 r0, r1, r2;  // world from view, by rows
};

struct GlobeVertex { float4 position [[position]]; };

vertex GlobeVertex globe_vertex(uint vid [[vertex_id]]) {
    // one triangle covering the screen
    float2 p = float2(float((vid << 1) & 2), float(vid & 2));
    GlobeVertex o;
    o.position = float4(p * 2 - 1, 0, 1);
    return o;
}

constant float3 PAPER = float3(0xF6, 0xF2, 0xE7) / 255;
constant float3 OCEAN_SHALLOW = float3(0xDD, 0xF3, 0xFD) / 255;
constant float3 OCEAN_DEEP = float3(0xBC, 0xE3, 0xF6) / 255;
constant float3 COAST = float3(0x1C, 0x7A, 0xB0) / 255;
constant float3 LAND = float3(0xF5, 0xEE, 0xD8) / 255;
constant float3 HIGHLAND = float3(0xE4, 0xD3, 0xAE) / 255;
constant float3 BORDER = float3(0x8C, 0x84, 0x78) / 255;
constant float3 RIVER = float3(0x4E, 0x9C, 0xCB) / 255;
constant float3 LOCATOR = float3(0xC1, 0x28, 0x38) / 255;
constant float3 GOLD = float3(0xB0, 0x8A, 0x3E) / 255;
constant float3 SHADOW = float3(0x6E, 0x63, 0x52) / 255;

// The Persian style: a study of the illuminated opening and the landscapes of LACMA's Khamsa of
// Nizami (M.85.237.16, Tabriz, 1529), cleaned of the varnish's yellow. The globe is the quiet text
// panel; its frame is the manuscript's: gold, pink, green and lapis rulings around a lapis band.
constant float3 P_PAPER = float3(0xF3, 0xE9, 0xD2) / 255;
constant float3 P_SEA = float3(0xE7, 0xEB, 0xF1) / 255;
constant float3 P_SEA_DEEP = float3(0xD3, 0xDA, 0xE9) / 255;
constant float3 P_COAST = float3(0x4A, 0x57, 0x94) / 255;
constant float3 P_MEADOW = float3(0xEC, 0xE7, 0xCE) / 255;     // lowland: the meadows' pale sage, on cream
constant float3 P_UPLAND = float3(0xDC, 0xDC, 0xC0) / 255;
constant float3 P_ROCK = float3(0xD5, 0xC6, 0xD0) / 255;       // highland: the lavender rocks
constant float3 P_PEAK = float3(0xED, 0xE5, 0xEA) / 255;
constant float3 P_SHADE = float3(0x8C, 0x7C, 0x9C) / 255;
constant float3 P_BORDER = float3(0xA8, 0x63, 0x5F) / 255;
constant float3 P_RIVER = float3(0x5C, 0x6E, 0xAE) / 255;
constant float3 P_LAPIS = float3(0x4E, 0x5B, 0x9B) / 255;
constant float3 P_GOLD = float3(0xA8, 0x89, 0x4F) / 255;
constant float3 P_PINK = float3(0xD5, 0x9A, 0x86) / 255;
constant float3 P_GREEN = float3(0x6F, 0x9A, 0x7E) / 255;
constant float3 P_INK = float3(0x2A, 0x21, 0x19) / 255;
constant float3 P_SEL = float3(0xD6, 0xBE, 0x7E) / 255;
constant float3 P_SEL_EDGE = float3(0xC0, 0x70, 0x5E) / 255;
constant float SEL_ALPHA = 0.85;
// The frame, outward from the globe's edge, in units of its radius (so it grows with the globe):
// the ornamental band (Globe/band.png, its repeats round), and the blue-ink finials standing on a
// ruled circle on the paper (Globe/fleuron.png: their base, height and number)
constant float P_BAND_IN = 1.0155, P_BAND_OUT = 1.0705, P_BAND_N = 20;
constant float P_FL_BASE = 1.1025, P_FL_TALL = 0.045, P_FL_N = 48;

/// A hash without sine (whose precision fails at a screen's coordinates in 32 bits).
static float hash2(float2 p) {
    float3 p3 = fract(float3(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

static float vnoise(float2 p) {
    float2 i = floor(p), f = p - i;
    float2 u = f * f * (3 - 2 * f);
    float a = hash2(i), b = hash2(i + float2(1, 0)), c = hash2(i + float2(0, 1)), d = hash2(i + float2(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Floored modulo, as Python's %.
static float floormod(float x, float y) { return x - y * floor(x / y); }

static float deg(float r) { return r * 57.29577951308232; }

/// Distance to the nearest of one jittered point per unit cell.
static float worley(float2 p) {
    float2 i = floor(p);
    float best = 9;
    for (int oy = -1; oy <= 1; oy++) {
        for (int ox = -1; ox <= 1; ox++) {
            float2 c = i + float2(ox, oy);
            float2 j = float2(hash2(c), hash2(float2(c.y + 17, c.x + 31)));
            best = min(best, length(c + j - p));
        }
    }
    return best;
}

/// Coverage of the ring between radii a and b (in the globe's radii; R its radius in pixels).
static float ring(float rr, float a, float b, float R) {
    return smoothstep(a - 0.5 / R, a + 0.5 / R, rr) * (1 - smoothstep(b - 0.5 / R, b + 0.5 / R, rr));
}

/// How steeply a field changes per texel of the map, from its change per pixel (gs) and the map's
/// (tx, ty: texels per pixel along the screen's x and y): a distance changes about 1 a texel, the
/// jump where a line's sign flips about 16.
static float perTexel(float2 gs, float2 tx, float2 ty) {
    float det = tx.x * ty.y - tx.y * ty.x;
    if (abs(det) < 1e-8) return 1e9;
    float2 gt = float2(ty.y * gs.x - tx.y * gs.y, -ty.x * gs.x + tx.x * gs.y) / det;
    return length(gt);
}

/// Coverage of a line `width` pixels wide along d == 0, fw being d's change per pixel. (Every fwidth
/// is taken at the top level, outside any branch: derivatives are undefined where neighbouring
/// pixels part ways.)
static float stroke(float d, float width, float fw) {
    return 1 - smoothstep(width / 2 - 0.5, width / 2 + 0.5, abs(d) / max(fw, 1e-5));
}

fragment float4 globe_fragment(GlobeVertex in [[stage_in]],
                               constant GlobeUniforms &U [[buffer(0)]],
                               texture2d<float> geoTex [[texture(0)]],
                               texture2d<float> terrainTex [[texture(1)]],
                               texture2d<float> idsTex [[texture(2)]],
                               texture2d<float> bandTex [[texture(3)]],
                               texture2d<float> fleuronTex [[texture(4)]]) {
    constexpr sampler linearS(s_address::repeat, t_address::clamp_to_edge, filter::linear, mip_filter::linear);
    constexpr sampler nearestS(s_address::repeat, t_address::clamp_to_edge, filter::nearest);
    constexpr sampler spriteS(address::clamp_to_zero, filter::linear, mip_filter::linear);
    float2 pix = in.position.xy;
    float2 size = U.u0.xy, centre = U.u0.zw;
    float R = U.u1.x, scale = U.u1.y, zoom = U.u1.z, ringOn = U.u1.w;
    float selected = U.u2.x, highlight = U.u2.y, style = U.u2.z;
    float x = (pix.x - centre.x) / R, y = (centre.y - pix.y) / R;
    float rr = sqrt(x * x + y * y);

    // the page: paper with a little grain and a soft vignette, the globe's shadow, the gold ring
    float grain = vnoise(pix / 1.7) * 0.5 + vnoise(pix / 9) * 0.5;
    float3 page = PAPER * (0.985 + 0.03 * grain);
    float fwRing = fwidth((rr - 1) * R / scale);
    // the Persian frame's ornaments are sampled round the globe; their derivatives come from the
    // angle's (which jumps at the top, where it wraps: take them from the half-turned angle there)
    float theta = floormod(atan2(x, y), 2 * M_PI_F);   // clockwise from the top
    float theta2 = fract(theta / (2 * M_PI_F) + 0.5) * 2 * M_PI_F;
    float tdx = abs(dfdx(theta)) < abs(dfdx(theta2)) ? dfdx(theta) : dfdx(theta2);
    float tdy = abs(dfdy(theta)) < abs(dfdy(theta2)) ? dfdy(theta) : dfdy(theta2);
    float rdx = dfdx(rr), rdy = dfdy(rr);
    float3 ppage = 0;
    if (style > 0.001) {                                // (the same for every pixel)
        ppage = P_PAPER * (0.985 + 0.03 * grain);
        float pvig = length(float2(pix.x / size.x - 0.5, pix.y / size.y - 0.48));
        ppage *= 1 - 0.06 * smoothstep(0.35, 0.8, pvig);
        float psh = rr > 1.08 ? exp(-pow(max(length(float2(x, y + 0.03)) - 1.10, 0.0), 2.0) / 0.003) : 0;
        ppage = mix(ppage, SHADOW, 0.10 * psh);
        // the rulings of a manuscript's frame (jadval), the band between them, then on the paper a
        // ruled circle with blue-ink finials standing on it, as round a medallion
        ppage = mix(ppage, P_INK, ring(rr, 1.000, 1.0035, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.0035, 1.0115, R));
        ppage = mix(ppage, P_PINK, ring(rr, 1.0115, 1.0155, R));
        ppage = mix(ppage, P_PINK, ring(rr, 1.0705, 1.0745, R));
        ppage = mix(ppage, P_GREEN, ring(rr, 1.0745, 1.0805, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.0805, 1.0905, R));
        ppage = mix(ppage, P_INK, ring(rr, 1.0905, 1.093, R));
        ppage = mix(ppage, P_LAPIS, ring(rr, 1.100, 1.1025, R));
        float turn = 2 * M_PI_F;
        float bw = P_BAND_OUT - P_BAND_IN;
        float2 buv = float2(fract(theta / turn * P_BAND_N), (rr - P_BAND_IN) / bw);
        float4 band = bandTex.sample(linearS, buv, gradient2d(float2(tdx / turn * P_BAND_N, rdx / bw),
                                                              float2(tdy / turn * P_BAND_N, rdy / bw)));
        ppage = mix(ppage, band.rgb, ring(rr, P_BAND_IN, P_BAND_OUT, R) * band.a);
        float side = P_FL_TALL * 256 / 224;             // the sprite's content is 224 of its 256 px
        float span = turn * P_FL_BASE / P_FL_N / side;  // a finial's share of the circle, in sprite widths
        float2 fuv = float2(0.5 + (fract(theta / turn * P_FL_N) - 0.5) * span, (240 - (rr - P_FL_BASE) / side * 256) / 256);
        float4 fl = fleuronTex.sample(spriteS, fuv, gradient2d(float2(tdx / turn * P_FL_N * span, -rdx / side),
                                                               float2(tdy / turn * P_FL_N * span, -rdy / side)));
        ppage = ppage * (1 - fl.a) + fl.rgb;            // (premultiplied on upload)
    }
    float vig = length(float2(pix.x / size.x - 0.5, pix.y / size.y - 0.48));
    page *= 1 - 0.07 * smoothstep(0.35, 0.8, vig);
    float sh = rr > 0.97 ? exp(-pow(max(length(float2(x, y + 0.035)) - 0.985, 0.0), 2.0) / 0.0035) : 0;
    page = mix(page, SHADOW, 0.16 * sh);
    float ringpx = (rr - 1) * R / scale;                // points outside the globe's edge
    if (ringOn > 0.001 && style < 0.999) {              // (the same for every pixel)
        float gold = max(stroke(ringpx - 6.5, 1.1 * scale, fwRing), stroke(ringpx - 10.0, 0.7 * scale, fwRing));
        float ang = floormod(deg(atan2(x, y)), 360);
        float tick = (ringpx > 7 && ringpx < 9.6) ? stroke(floormod(ang + 5, 10) - 5, 0.9 * scale, deg(1 / max(rr * R, 1.0))) : 0;
        float dots = 0;
        for (int q = 0; q < 4; q++) {                   // the four quarters: three little dots, Kells-fashion
            for (int k = -1; k <= 1; k++) {
                float t = (90.0 * q + 1.5 * k) / 57.29577951308232;
                float2 at = float2(centre.x + sin(t) * (R + 8.25 * scale), centre.y - cos(t) * (R + 8.25 * scale));
                dots = max(dots, 1 - smoothstep(1.1 * scale, 1.9 * scale, length(at - pix)));
            }
        }
        page = mix(page, GOLD, clamp(max(max(gold, tick * 0.8), dots), 0.0, 1.0) * ringOn);
    }
    page = mix(page, ppage, style);
    // the globe: the point of the sphere under this pixel, and where it is on the map (worked out for
    // the page's pixels too, so no pixel leaves its neighbours before the derivatives are taken)
    float z = sqrt(clamp(1 - rr * rr, 0.0, 1.0));
    float3 v = float3(x, y, z);
    float3 w = float3(dot(U.r0.xyz, v), dot(U.r1.xyz, v), dot(U.r2.xyz, v));
    float lat = asin(clamp(w.y, -1.0, 1.0)), lon = atan2(w.x, w.z);
    float2 uv = float2((lon / M_PI_F + 1) / 2, 0.5 - lat / M_PI_F);
    // the map wraps at the antimeridian: take each derivative from whichever of u or u + 1/2 doesn't
    // jump there, or the sampler would read the smallest mip along that meridian
    float2 uv2 = float2(fract(uv.x + 0.5), uv.y);
    float2 dx1 = dfdx(uv), dy1 = dfdy(uv), dx2 = dfdx(uv2), dy2 = dfdy(uv2);
    float2 dx = float2(abs(dx1.x) < abs(dx2.x) ? dx1.x : dx2.x, dx1.y);
    float2 dy = float2(abs(dy1.x) < abs(dy2.x) ? dy1.x : dy2.x, dy1.y);
    float4 g = geoTex.sample(linearS, uv, gradient2d(dx, dy));
    float4 t = terrainTex.sample(linearS, uv, gradient2d(dx, dy));
    float fine = (g.r * 255 - 128) / 8;
    float coarse = (g.g * 255 - 128) * 2;
    float d = abs(coarse) > 14 ? coarse : fine;
    float border = (g.b * 255 - 128) / 16;
    float hill = (t.r * 255 - 128) / 127;
    float height = t.g * 255 - 128;
    float river = (t.b * 255 - 128) / 16;
    float fwD = fwidth(d), fwFine = fwidth(fine), fwBorder = fwidth(border), fwRiver = fwidth(river);
    // a line's distance changes about a texel a texel; where its sign flips (past a line's end,
    // between lines) it jumps: no line there
    float2 tx = dx * float2(4096, 2048), ty = dy * float2(4096, 2048);
    float borderOn = perTexel(float2(dfdx(border), dfdy(border)), tx, ty) < 1.5 && abs(border) < 7 ? 1.0 : 0.0;
    float riverOn = perTexel(float2(dfdx(river), dfdy(river)), tx, ty) < 1.5 && abs(river) < 7 ? 1.0 : 0.0;
    float landa = smoothstep(-0.5, 0.5, d / max(fwD, 1e-5));
    float k = clamp(zoom, 1.0, 8.0);

    float depth = pow(clamp(-min(height, 0.0) * 87 / 5500, 0.0, 1.0), 0.6);
    float mott = vnoise(float2(uv.x * 90, uv.y * 45)) * 0.6 + vnoise(float2(uv.x * 260, uv.y * 130)) * 0.4;
    float hm = max(height, 0.0) * 50;
    float offshore = fine < -2 && fine > -15.5 ? 1.0 : 0.0;
    // the graticule, every 15 degrees
    float glat = deg(lat), glon = deg(lon);
    float fwlat = fwidth(glat);
    float fwlon = min(fwidth(glon), fwidth(floormod(glon, 360)));   // the antimeridian's jump, or 0's
    float gl = max(stroke(floormod(glat + 7.5, 15) - 7.5, 0.6 * scale, fwlat),
                   stroke(floormod(glon + 7.5, 15) - 7.5, 0.6 * scale, fwlon) * (abs(glat) < 75 ? 1.0 : 0.0));
    // the selected country's share of the four texels around this point, read exactly and blended,
    // so its edge is smooth however close the camera
    float m = 0;
    if (selected > 0.5 && highlight > 0.001) {
        float4 q = idsTex.gather(nearestS, uv) * 255;
        float2 f = fract(uv * float2(float(idsTex.get_width()), float(idsTex.get_height())) - 0.5);
        float4 is = float4(abs(q - selected) < 0.5);     // gather: (0,1), (1,1), (1,0), (0,0)
        m = mix(mix(is.w, is.z, f.x), mix(is.x, is.y, f.x), f.y) * highlight;
    }

    float3 col = 0;
    if (style < 0.999) {
        // the sea: deeper is a little bluer, with a faint watercolour mottle and lines along the coasts
        float3 sea = mix(OCEAN_SHALLOW, OCEAN_DEEP, depth);
        sea *= 0.985 + 0.03 * mott;
        sea = mix(sea, COAST, 0.15 * stroke(fine + 6, 0.7 * scale, fwFine) * offshore);
        sea = mix(sea, COAST, 0.08 * stroke(fine + 13, 0.7 * scale, fwFine) * offshore);
        // the land: cream, warmer up high, the relief shaded from the north-west
        float3 land = mix(LAND, HIGHLAND, 0.85 * smoothstep(250.0, 3200.0, hm));
        land *= 1 + 0.30 * hill;
        land = mix(land, RIVER, 0.75 * stroke(river, 0.65 * scale, fwRiver) * smoothstep(1.2, 2.2, k) * riverOn);
        // the selected country over its rivers, as a locator map has it; its borders drawn on top
        land = mix(land, LOCATOR * (1 + 0.12 * hill), 0.94 * m);
        land = mix(land, BORDER, 0.8 * stroke(border, 0.7 * scale, fwBorder) * borderOn);
        col = mix(sea, land, landa);
        col = mix(col, COAST, stroke(d, 0.9 * scale * min(pow(k, 0.25), 1.4), fwD));
        col = mix(col, COAST, gl * (landa > 0.5 ? 0.10 : 0.16));
        // roundness: the limb a little shaded; the rim drawn
        col *= 1 - 0.07 * (1 - z) * (1 - z);
        col = mix(col, COAST, stroke((rr - 1) * R, 1.2 * scale, fwRing * scale));
    }
    if (style > 0.001) {                                // (the same for every pixel)
        // the sea: a pale lapis wash, its coasts ruled like a panel when looked at closer, a gold hair
        // close in and a lapis one further out
        float3 sea = mix(P_SEA, P_SEA_DEEP, depth);
        sea *= 0.985 + 0.03 * mott;
        float near = smoothstep(1.4, 2.6, k);
        sea = mix(sea, P_GOLD, 0.6 * near * stroke(fine + 4.5, 0.6 * scale, fwFine) * (fine < -1.5 && fine > -8 ? 1.0 : 0.0));
        sea = mix(sea, P_COAST, 0.14 * near * stroke(fine + 11, 0.6 * scale, fwFine) * (fine < -8 && fine > -15.5 ? 1.0 : 0.0));
        // the land: the meadows' sage low, the rocks' lavender high, their shadows violet
        float3 land = mix(P_MEADOW, P_UPLAND, smoothstep(200.0, 1100.0, hm));
        land = mix(land, P_ROCK, smoothstep(900.0, 2600.0, hm));
        land = mix(land, P_PEAK, 0.7 * smoothstep(5000.0, 6300.0, hm));
        land = mix(land, P_SHADE, 0.36 * clamp(-hill, 0.0, 1.0));
        land = mix(land, float3(1), 0.30 * clamp(hill, 0.0, 1.0));
        // closer in, the heights' contours every 600 m, lobed as the painters drew rocks (the
        // height plus a little cellular swell), pale like the rocks' highlights, a violet hair below
        if (k > 1.6) {                                  // (the same for every pixel)
            float hc = hm / 600 + 0.3 * worley(uv * float2(4096, 2048) / 4.5);
            float fwHc = fwidth(hc);
            float rocky = smoothstep(800.0, 1300.0, hm) * smoothstep(1.6, 2.6, k);
            land = mix(land, P_PEAK, 0.7 * stroke(fract(hc + 0.5) - 0.5, 0.55 * scale, fwHc) * rocky);
            land = mix(land, P_SHADE, 0.22 * stroke(fract(hc + 0.42) - 0.5, 0.55 * scale, fwHc) * rocky);
        }
        land = mix(land, P_RIVER, 0.75 * stroke(river, 0.65 * scale, fwRiver) * smoothstep(1.2, 2.2, k) * riverOn);
        // the chosen country gilded, like the opening's cartouche, and edged in its pink
        float3 gild = P_SEL * (1 + 0.20 * hill + 0.05 * (vnoise(uv * float2(2400, 1200)) - 0.5));
        land = mix(land, gild, SEL_ALPHA * m);
        land = mix(land, P_BORDER, 0.75 * stroke(border, 0.7 * scale, fwBorder) * borderOn);
        land = mix(land, P_SEL_EDGE, stroke(border, 1.5 * scale, fwBorder) * borderOn * (m > 0.04 * highlight ? highlight : 0.0));
        float3 pcol = mix(sea, land, landa);
        pcol = mix(pcol, P_COAST, stroke(d, 0.75 * scale * min(pow(k, 0.3), 1.3), fwD));
        pcol = mix(pcol, P_GOLD, gl * (landa > 0.5 ? 0.14 : 0.22));
        pcol *= 1 - 0.06 * (1 - z) * (1 - z);
        col = mix(col, pcol, style);
    }
    float edge = 1 - smoothstep(1 - 0.5 / R, 1 + 0.5 / R, rr);
    return float4(rr < 1 + 2 / R ? mix(page, col, edge) : page, 1);
}
