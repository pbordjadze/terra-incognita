#include <metal_stdlib>
using namespace metal;

// The globe and the page around it, one fragment per pixel: the orthographic projection inverted,
// the map's distance fields sampled, the lines drawn a constant width on screen. tools/globe/preview.py
// is the same, line for line (its palette and constants), and draws it without a phone.

struct GlobeUniforms {
    float4 u0;          // drawable size (px), the globe's centre (px)
    float4 u1;          // radius (px), pixels per reading point, zoom, the gold ring's strength
    float4 u2;          // the selected country (1 + its index; 0: none), its highlight, the style
                        // (0 Wikipedia's colours, 1 the Persian; between them while it changes),
                        // the page's frame (how far it has come in)
    float4 r0, r1, r2;  // world from view, by rows
    float4 u3;          // the safe area's insets (px): top, bottom, left, right
    float4 u4;          // x: illustrative prehistoric view suppresses modern political boundaries;
                        // y: how far the drawing has crossed to the ground style (0 Wikipedia's, 1 the ground)
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

// The ground style (tools/globe/ground.py: the land is baked, in the atlas's light key, from NASA's Blue
// Marble for the month): the sea's tints by depth, its lines and its ice, the air at the limb.
constant float3 G_SHELF = float3(0xD3, 0xF0, 0xEE) / 255;      // under 60 m: shallow water's turquoise
constant float3 G_OPEN = float3(0xC2, 0xE4, 0xF4) / 255;       // the open ocean
constant float3 G_DEEP = float3(0xA9, 0xD2, 0xEC) / 255;       // past 6000 m
constant float3 G_COAST = float3(0x5E, 0x9C, 0xC0) / 255;
constant float3 G_RIVER = float3(0x6F, 0xAE, 0xD3) / 255;
constant float3 G_ICE = float3(0xF1, 0xF5, 0xF8) / 255;
constant float3 G_ATMOS = float3(0xDC, 0xEE, 0xFB) / 255;
constant float3 G_CHOSEN = float3(0x9E, 0x1F, 0x2C) / 255;     // the chosen country's edge: the locator's red, deeper

// The Persian style, every colour traced (CLAUDE.md › The Persian style's sources). The manuscript's
// colours are sampled from LACMA's scans of the Khamsa of Nizami (M.85.237.16, Tabriz, 1529),
// white-balanced so its lead-white pigment is neutral; the map's tints take their lightness from
// Wikipedia's map conventions (WikiProject Maps) and their hue from the manuscript. The golds alone
// are lifted from the scan's (+0.10 OKLab lightness, chroma x1.3, same hue): a scan shows shell gold
// unlit, olive, and gold is meant to be seen under light.
constant float3 P_PAPER = float3(0xF2, 0xE4, 0xC8) / 255;      // the margin's paper
constant float3 P_SEA = float3(0xE1, 0xEF, 0xFF) / 255;        // Wikipedia's shallowest sea, in the light-blue ruling's hue
constant float3 P_SEA_DEEP = float3(0xC1, 0xD6, 0xF0) / 255;   // its fourth depth, in the same light blue (lapis let down with white)
constant float3 P_COAST = float3(0x6F, 0x69, 0x95) / 255;      // Wikipedia's coasts and rivers, in the margin's blue ink
constant float3 P_MEADOW = float3(0xDD, 0xE7, 0xD9) / 255;     // Wikipedia's land, in the landscapes' meadow
constant float3 P_ROCK = float3(0xED, 0xDE, 0xEC) / 255;       // the same, in their lavender rocks
constant float3 P_PEAK = float3(0xF7, 0xF1, 0xFF) / 255;       // Wikipedia's highest tint, in the rocks' highlights
constant float3 P_SHADE = float3(0xA2, 0x98, 0xA7) / 255;      // the rocks' shadowed strokes
constant float3 P_WHITE = float3(0xF5, 0xF5, 0xF5) / 255;      // lead white: the rocks' highlights
constant float3 P_BORDER = float3(0x82, 0x59, 0x47) / 255;     // Wikipedia's borders, in the pink ruling's hue
constant float3 P_LAPIS = float3(0x5F, 0x6C, 0xA7) / 255;
constant float3 P_GOLD = float3(0xC5, 0xAA, 0x6E) / 255;        // (gold lifted from the scan's: see below)
constant float3 P_PINK = float3(0xD1, 0xA3, 0x8F) / 255;
constant float3 P_GREEN = float3(0x79, 0xA1, 0x8B) / 255;
constant float3 P_LIGHT_BLUE = float3(0xA2, 0xB7, 0xD0) / 255;
constant float3 P_INK = float3(0x25, 0x1F, 0x14) / 255;        // the black of the rulings
constant float3 P_SEL = float3(0xC2, 0xA8, 0x62) / 255;        // the headpiece cartouche's gold, lifted
constant float SEL_ALPHA = 0.85;
// The frame, outward from the globe's edge in units of its radius (so it grows with the globe), in
// the order and proportions measured across the opening spread's frame, from its text panel out:
// pink, gold, light blue, the braid (Globe/braid.png, its repeats round), gold, green, gold, pink,
// gold, lapis, gold, black, the border band (Globe/band.png), black, gold, pink; then on the paper a
// ruled line in the margin's ink with its finials standing on it (Globe/fleuron.png: their base,
// height and number)
constant float P_BRAID_IN = 1.0091, P_BRAID_OUT = 1.0307, P_BRAID_N = 74;
constant float P_BAND_IN = 1.0650, P_BAND_OUT = 1.1150, P_BAND_N = 30;
constant float P_FL_BASE = 1.1383, P_FL_TALL = 0.045, P_FL_N = 48;
// The page's frame, when the globe is looked at closer than its own frame allows: the map becomes
// the text panel, framed as the spread frames its panels, by their rulings and braid, on a margin of
// paper at the screen's edges. In reading points out from the panel: pink, gold, light blue, the
// braid (8 points deep, its tile 4:1), gold, green, gold, pink, in the measured proportions.
constant float PAGE_MARGIN = 6, PAGE_DEPTH = 16.81, PAGE_BRAID_IN = 3.39, PAGE_BRAID_OUT = 11.39;

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

/// Value noise in three dimensions: on the sphere it has no seam and no pole.
static float vnoise3(float3 p) {
    float3 i = floor(p), f = p - i;
    float3 u = f * f * (3 - 2 * f);
    float n = 0;
    for (int dz = 0; dz <= 1; dz++) {
        for (int dy = 0; dy <= 1; dy++) {
            for (int dx = 0; dx <= 1; dx++) {
                float3 c = i + float3(float(dx), float(dy), float(dz));
                float h = hash2(float2(hash2(c.xy) * 157.0 + c.z * 1.37, c.z * 0.71 + hash2(c.yx) * 89.0));
                n += h * (dx ? u.x : 1 - u.x) * (dy ? u.y : 1 - u.y) * (dz ? u.z : 1 - u.z);
            }
        }
    }
    return n;
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

/// Coverage of the ring between radii a and b (in the globe's radii; R its radius in pixels); one
/// thinner than 0.8 px is drawn 0.8 px wide and as much fainter, so a ruling never vanishes.
static float ring(float rr, float a, float b, float R) {
    float w = (b - a) * R, k = 1;
    if (w < 0.8) { float c = (a + b) / 2; a = c - 0.4 / R; b = c + 0.4 / R; k = w / 0.8; }
    return k * smoothstep(a - 0.5 / R, a + 0.5 / R, rr) * (1 - smoothstep(b - 0.5 / R, b + 0.5 / R, rr));
}

/// Coverage of a band of the page's frame from `a` to `b` reading points out from the panel, `e`
/// being how far in from the frame's outer edge a pixel is; never thinner than 0.8 px (`ppp`: pixels
/// per reading point).
static float pageBand(float e, float a, float b, float ppp) {
    float lo = PAGE_DEPTH - b, hi = PAGE_DEPTH - a, k = 1;
    if ((hi - lo) * ppp < 0.8) { float c = (lo + hi) / 2; k = (hi - lo) * ppp / 0.8; lo = c - 0.4 / ppp; hi = c + 0.4 / ppp; }
    return k * clamp(min(e - lo, hi - e) * ppp + 0.5, 0.0, 1.0);
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
                               texture2d<float> fleuronTex [[texture(4)]],
                               texture2d<float> braidTex [[texture(5)]],
                               texture2d<float> groundTex [[texture(6)]],
                               texture2d<float> iceTex [[texture(7)]],
                               texture2d<float> floorTex [[texture(8)]]) {
    constexpr sampler linearS(s_address::repeat, t_address::clamp_to_edge, filter::linear, mip_filter::linear);
    constexpr sampler nearestS(s_address::repeat, t_address::clamp_to_edge, filter::nearest);
    constexpr sampler spriteS(address::clamp_to_zero, filter::linear, mip_filter::linear);
    float2 pix = in.position.xy;
    float2 size = U.u0.xy, centre = U.u0.zw;
    float R = U.u1.x, scale = U.u1.y, zoom = U.u1.z, ringOn = U.u1.w;
    float selected = U.u2.x, highlight = U.u2.y, style = U.u2.z, pageOn = U.u2.w;
    float ground = U.u4.y;
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
    float tdx1 = dfdx(theta), tdx2 = dfdx(theta2), tdy1 = dfdy(theta), tdy2 = dfdy(theta2);
    float tdx = abs(tdx1) < abs(tdx2) ? tdx1 : tdx2;
    float tdy = abs(tdy1) < abs(tdy2) ? tdy1 : tdy2;
    float rdx = dfdx(rr), rdy = dfdy(rr);
    float3 ppage = 0;
    if (style > 0.001) {                                // (the same for every pixel)
        ppage = P_PAPER * (0.985 + 0.03 * grain);       // (the paper's own texture; no shadow, no vignette)
        ppage = mix(ppage, P_PINK, ring(rr, 1.0000, 1.0018, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.0018, 1.0055, R));
        ppage = mix(ppage, P_LIGHT_BLUE, ring(rr, 1.0055, 1.0091, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.0307, 1.0314, R));
        ppage = mix(ppage, P_GREEN, ring(rr, 1.0314, 1.0373, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.0373, 1.0416, R));
        ppage = mix(ppage, P_PINK, ring(rr, 1.0416, 1.0453, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.0453, 1.0471, R));
        ppage = mix(ppage, P_LAPIS, ring(rr, 1.0471, 1.0486, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.0486, 1.0635, R));
        ppage = mix(ppage, P_INK, ring(rr, 1.0635, 1.0650, R));
        ppage = mix(ppage, P_INK, ring(rr, 1.1150, 1.1161, R));
        ppage = mix(ppage, P_GOLD, ring(rr, 1.1161, 1.1274, R));
        ppage = mix(ppage, P_PINK, ring(rr, 1.1274, 1.1303, R));
        ppage = mix(ppage, P_COAST, ring(rr, 1.1376, 1.1383, R));
        float turn = 2 * M_PI_F;
        float ww = P_BRAID_OUT - P_BRAID_IN;
        float2 wuv = float2(fract(theta / turn * P_BRAID_N), (rr - P_BRAID_IN) / ww);
        float4 braid = braidTex.sample(linearS, wuv, gradient2d(float2(tdx / turn * P_BRAID_N, rdx / ww),
                                                               float2(tdy / turn * P_BRAID_N, rdy / ww)));
        ppage = mix(ppage, braid.rgb, ring(rr, P_BRAID_IN, P_BRAID_OUT, R) * braid.a);
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
    if (ground > 0.001) {                               // (the same for every pixel)
        // the ground style's air: a thin bright haze just outside the limb, as it is seen edge on
        page = mix(page, G_ATMOS, 0.85 * ground * exp(-max(ringpx, 0.0) / 3.5) * (rr >= 1 ? 1.0 : 0.0));
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
    borderOn *= 1 - U.u4.x;
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
        if (ground > 0.001) {                           // (the same for every pixel)
            // the ground: the month's land as satellites saw it, in the atlas's light key, shaded
            // from the north-west; rivers from the regional scale; borders a pencil line from about
            // x2 to x3; the chosen country's red let down so the ground shows through it
            float3 gland = groundTex.sample(linearS, uv, gradient2d(dx, dy)).rgb * (1 + 0.20 * hill);
            gland = mix(gland, G_RIVER, 0.7 * stroke(river, 0.6 * scale, fwRiver) * smoothstep(1.4, 2.4, k) * riverOn);
            gland = mix(gland, LOCATOR * (1 + 0.12 * hill), 0.75 * m);
            gland = mix(gland, BORDER, 0.55 * smoothstep(2.0, 3.2, zoom) * stroke(border, 0.6 * scale, fwBorder) * borderOn);
            // the chosen country's land borders a firm line, over its fill's edge (the country map's
            // texels show as steps close in where the pencil borders are too faint to cover them)
            gland = mix(gland, G_CHOSEN, stroke(border, 1.6 * scale, fwBorder) * borderOn * (m > 0.04 * highlight ? highlight : 0.0));
            // the sea: shelves pale, the open ocean, the abyss, the floor's relief shaded faintly
            float depthM = -min(height, 0.0) * 87;
            float3 gsea = mix(G_SHELF, G_OPEN, smoothstep(60.0, 900.0, depthM));
            gsea = mix(gsea, G_DEEP, smoothstep(2500.0, 6000.0, depthM));
            float fl = (floorTex.sample(linearS, uv, gradient2d(dx, dy)).r * 255 - 128) / 127;
            gsea *= 1 + 0.04 * fl;
            // the month's usual sea ice, its edge broken a little (drawn, not measured)
            float ice = iceTex.sample(linearS, uv, gradient2d(dx, dy)).r;
            float jag = 0.5;
            if (ice > 0.02) jag = vnoise3(w * 220 + float3(17.3, 5.1, 9.7)) * 0.6 + vnoise3(w * 900 + float3(17.3, 5.1, 9.7)) * 0.4;
            gsea = mix(gsea, G_ICE * (0.97 + 0.04 * jag), smoothstep(0.30, 0.55, ice + 0.25 * (jag - 0.5)));
            float3 g = mix(gsea, gland, landa);
            // the coast lighter and finer than Wikipedia's, fading where both sides are white
            float pale = smoothstep(0.90, 0.97, (gland.r + gland.g + gland.b) / 3);
            g = mix(g, G_COAST, (0.55 - 0.35 * pale) * stroke(d, 0.6 * scale * min(pow(k, 0.3), 1.5), fwD));
            // roundness, and the air at the limb
            g *= 1 - 0.05 * (1 - z) * (1 - z);
            g = mix(g, G_ATMOS, 0.55 * pow(1 - z, 4.0));
            col = mix(col, g, ground);
        }
    }
    if (style > 0.001) {                                // (the same for every pixel)
        // the sea: a pale lapis wash, its coasts ruled like a panel when looked at closer, a gold hair
        // close in and a lapis one further out
        float3 sea = mix(P_SEA, P_SEA_DEEP, depth);     // (flat washes, as the manuscript's colours are)
        float near = smoothstep(1.4, 2.6, k);
        sea = mix(sea, P_GOLD, 0.6 * near * stroke(fine + 4.5, 0.6 * scale, fwFine) * (fine < -1.5 && fine > -8 ? 1.0 : 0.0));
        sea = mix(sea, P_LIGHT_BLUE, 0.5 * near * stroke(fine + 11, 0.6 * scale, fwFine) * (fine < -8 && fine > -15.5 ? 1.0 : 0.0));
        // the land: the meadows' sage low, the rocks' lavender high, their shadows violet
        float3 land = mix(P_MEADOW, P_ROCK, smoothstep(900.0, 2600.0, hm));
        land = mix(land, P_PEAK, 0.7 * smoothstep(5000.0, 6300.0, hm));
        land = mix(land, P_SHADE, 0.36 * clamp(-hill, 0.0, 1.0));
        land = mix(land, P_WHITE, 0.30 * clamp(hill, 0.0, 1.0));
        // closer in, the heights' contours every 600 m, lobed as the painters drew rocks (the
        // height plus a little cellular swell), pale like the rocks' highlights, a violet hair below
        if (k > 1.6) {                                  // (the same for every pixel)
            float hc = hm / 600 + 0.3 * worley(uv * float2(4096, 2048) / 4.5);
            float fwHc = fwidth(hc);
            float rocky = smoothstep(800.0, 1300.0, hm) * smoothstep(1.6, 2.6, k);
            land = mix(land, P_WHITE, 0.7 * stroke(fract(hc + 0.5) - 0.5, 0.55 * scale, fwHc) * rocky);
            land = mix(land, P_SHADE, 0.22 * stroke(fract(hc + 0.42) - 0.5, 0.55 * scale, fwHc) * rocky);
        }
        land = mix(land, P_COAST, 0.75 * stroke(river, 0.65 * scale, fwRiver) * smoothstep(1.2, 2.2, k) * riverOn);
        // the chosen country gilded in the cartouche's matte gold, its speckle and all, and edged in
        // its pink
        float3 gild = P_SEL * (1 + 0.20 * hill + 0.05 * (vnoise(uv * float2(2400, 1200)) - 0.5));
        land = mix(land, gild, SEL_ALPHA * m);
        land = mix(land, P_BORDER, 0.75 * stroke(border, 0.7 * scale, fwBorder) * borderOn);
        land = mix(land, P_PINK, stroke(border, 1.5 * scale, fwBorder) * borderOn * (m > 0.04 * highlight ? highlight : 0.0));
        float3 pcol = mix(sea, land, landa);
        pcol = mix(pcol, P_COAST, stroke(d, 0.75 * scale * min(pow(k, 0.3), 1.3), fwD));
        pcol = mix(pcol, P_GOLD, gl * (landa > 0.5 ? 0.14 : 0.22));
        pcol *= 1 - 0.06 * (1 - z) * (1 - z);
        col = mix(col, pcol, style);
    }
    float edge = 1 - smoothstep(1 - 0.5 / R, 1 + 0.5 / R, rr);
    float3 outc = rr < 1 + 2 / R ? mix(page, col, edge) : page;
    if (pageOn > 0.001) {                               // (the same for every pixel)
        // e: reading points in from the frame's outer edge, to the nearest of the four edges; the
        // braid runs along that edge (its gradients worked out, not taken: the edges meet in mitres)
        float outer = PAGE_MARGIN * scale;
        float dl = pix.x - U.u3.z - outer, dr = size.x - U.u3.w - outer - pix.x;
        float dt = pix.y - U.u3.x - outer, db = size.y - U.u3.y - outer - pix.y;
        float e = min(min(dl, dr), min(dt, db)) / scale;
        bool horizontal = min(dt, db) <= min(dl, dr);
        float side = horizontal ? (dt < db ? 1.0 : -1.0) : (dl < dr ? 1.0 : -1.0);   // which way e grows
        outc = mix(outc, P_PAPER * (0.985 + 0.03 * grain), pageOn * (1 - clamp(e * scale + 0.5, 0.0, 1.0)));
        outc = mix(outc, P_PINK, pageOn * pageBand(e, 0, 0.68, scale));
        outc = mix(outc, P_GOLD, pageOn * pageBand(e, 0.68, 2.03, scale));
        outc = mix(outc, P_LIGHT_BLUE, pageOn * pageBand(e, 2.03, 3.39, scale));
        outc = mix(outc, P_GOLD, pageOn * pageBand(e, 11.39, 11.66, scale));
        outc = mix(outc, P_GREEN, pageOn * pageBand(e, 11.66, 13.83, scale));
        outc = mix(outc, P_GOLD, pageOn * pageBand(e, 13.83, 15.46, scale));
        outc = mix(outc, P_PINK, pageOn * pageBand(e, 15.46, 16.81, scale));
        float tile = 8 * 4 * scale, depth = PAGE_BRAID_OUT - PAGE_BRAID_IN;
        float2 wuv = float2(fract((horizontal ? pix.x : pix.y) / tile), (PAGE_DEPTH - PAGE_BRAID_IN - e) / depth);
        float2 gx = float2(horizontal ? 1 / tile : 0, horizontal ? 0 : -side / (depth * scale));
        float2 gy = float2(horizontal ? 0 : 1 / tile, horizontal ? -side / (depth * scale) : 0);
        float4 braid = braidTex.sample(linearS, float2(wuv.x, clamp(wuv.y, 0.0, 1.0)), gradient2d(gx, gy));
        outc = mix(outc, braid.rgb, pageOn * pageBand(e, PAGE_BRAID_IN, PAGE_BRAID_OUT, scale) * braid.a);
    }
    return float4(outc, 1);
}
