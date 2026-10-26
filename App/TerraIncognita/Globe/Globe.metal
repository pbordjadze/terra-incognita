#include <metal_stdlib>
using namespace metal;

// The globe and the page around it, one fragment per pixel: the orthographic projection inverted,
// the map's distance fields sampled, the lines drawn a constant width on screen. tools/globe/preview.py
// is the same, line for line (its palette and constants), and draws it without a phone.

struct GlobeUniforms {
    float4 u0;          // drawable size (px), the globe's centre (px)
    float4 u1;          // radius (px), pixels per point, zoom, the gold ring's strength
    float4 u2;          // the selected country (1 + its index; 0: none), its highlight
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
                               texture2d<float> idsTex [[texture(2)]]) {
    constexpr sampler linearS(s_address::repeat, t_address::clamp_to_edge, filter::linear, mip_filter::linear);
    constexpr sampler nearestS(s_address::repeat, t_address::clamp_to_edge, filter::nearest);
    float2 pix = in.position.xy;
    float2 size = U.u0.xy, centre = U.u0.zw;
    float R = U.u1.x, scale = U.u1.y, zoom = U.u1.z, ringOn = U.u1.w;
    float selected = U.u2.x, highlight = U.u2.y;
    float x = (pix.x - centre.x) / R, y = (centre.y - pix.y) / R;
    float rr = sqrt(x * x + y * y);

    // the page: paper with a little grain and a soft vignette, the globe's shadow, the gold ring
    float grain = vnoise(pix / 1.7) * 0.5 + vnoise(pix / 9) * 0.5;
    float3 page = PAPER * (0.985 + 0.03 * grain);
    float vig = length(float2(pix.x / size.x - 0.5, pix.y / size.y - 0.48));
    page *= 1 - 0.07 * smoothstep(0.35, 0.8, vig);
    float sh = rr > 0.97 ? exp(-pow(max(length(float2(x, y + 0.035)) - 0.985, 0.0), 2.0) / 0.0035) : 0;
    page = mix(page, SHADOW, 0.16 * sh);
    float ringpx = (rr - 1) * R / scale;                // points outside the globe's edge
    float fwRing = fwidth(ringpx);
    if (ringOn > 0.001) {                               // (the same for every pixel)
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
    // (an unsigned distance sampled between texels never quite reaches 0 on its line)
    float border = max(g.b * 255 / 8 - 0.45, 0.0);
    float hill = (t.r * 255 - 128) / 127;
    float height = t.g * 255 - 128;
    float river = max(t.b * 255 / 16 - 0.45, 0.0);
    float fwD = fwidth(d), fwCoarse = fwidth(coarse), fwBorder = fwidth(border), fwRiver = fwidth(river);
    float landa = smoothstep(-0.5, 0.5, d / max(fwD, 1e-5));
    float k = clamp(zoom, 1.0, 8.0);

    // the sea: deeper is a little bluer, with a faint watercolour mottle and lines along the coasts
    float depth = pow(clamp(-min(height, 0.0) * 87 / 5500, 0.0, 1.0), 0.6);
    float3 sea = mix(OCEAN_SHALLOW, OCEAN_DEEP, depth);
    float mott = vnoise(float2(uv.x * 90, uv.y * 45)) * 0.6 + vnoise(float2(uv.x * 260, uv.y * 130)) * 0.4;
    sea *= 0.985 + 0.03 * mott;
    float offshore = coarse < -2 ? 1.0 : 0.0;
    sea = mix(sea, COAST, 0.15 * stroke(coarse + 6, 0.7 * scale, fwCoarse) * offshore);
    sea = mix(sea, COAST, 0.08 * stroke(coarse + 13, 0.7 * scale, fwCoarse) * offshore);
    // the land: cream, warmer up high, the relief shaded from the north-west
    float hm = max(height, 0.0) * 50;
    float3 land = mix(LAND, HIGHLAND, 0.85 * smoothstep(250.0, 3200.0, hm));
    land *= 1 + 0.30 * hill;
    if (selected > 0.5 && highlight > 0.001) {
        // the country's share of the four texels around this point, read exactly and blended, so
        // its edge is smooth however close the camera
        float4 q = idsTex.gather(nearestS, uv) * 255;
        float2 f = fract(uv * float2(float(idsTex.get_width()), float(idsTex.get_height())) - 0.5);
        float4 is = float4(abs(q - selected) < 0.5);     // gather: (0,1), (1,1), (1,0), (0,0)
        float m = mix(mix(is.w, is.z, f.x), mix(is.x, is.y, f.x), f.y);
        land = mix(land, LOCATOR * (1 + 0.12 * hill), 0.94 * m * highlight);
    }
    land = mix(land, RIVER, 0.75 * stroke(river, 0.65 * scale, fwRiver) * smoothstep(1.2, 2.2, k) * (river < 8 ? 1.0 : 0.0));
    land = mix(land, BORDER, 0.8 * stroke(border, 0.7 * scale, fwBorder) * (border < 16 ? 1.0 : 0.0));
    float3 col = mix(sea, land, landa);
    col = mix(col, COAST, stroke(d, 0.9 * scale * min(pow(k, 0.25), 1.4), fwD));
    // the graticule, every 15 degrees
    float glat = deg(lat), glon = deg(lon);
    float fwlat = fwidth(glat);
    float fwlon = min(fwidth(glon), fwidth(floormod(glon, 360)));   // the antimeridian's jump, or 0's
    float gl = max(stroke(floormod(glat + 7.5, 15) - 7.5, 0.6 * scale, fwlat),
                   stroke(floormod(glon + 7.5, 15) - 7.5, 0.6 * scale, fwlon) * (abs(glat) < 75 ? 1.0 : 0.0));
    col = mix(col, COAST, gl * (landa > 0.5 ? 0.10 : 0.16));
    // roundness: the limb a little shaded; the rim drawn
    col *= 1 - 0.07 * (1 - z) * (1 - z);
    col = mix(col, COAST, stroke((rr - 1) * R, 1.2 * scale, fwRing * scale));
    float edge = smoothstep(1 + 0.5 / R, 1 - 0.5 / R, rr);
    return float4(rr < 1 + 2 / R ? mix(page, col, edge) : page, 1);
}
