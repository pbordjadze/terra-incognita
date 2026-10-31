// The ground: the whole Earth as satellites saw it this month, in Terra's light key (Globe.metal's
// G_ palette, ported plainly; Visual takes it from here). One fragment a pixel inverts the orthographic
// projection and samples the map's distance fields, so the coast stays a constant width at any zoom.

@group(0) @binding(0) var<uniform> U: View;
@group(1) @binding(0) var geoTex: texture_2d<f32>;
@group(1) @binding(1) var terrainTex: texture_2d<f32>;
@group(1) @binding(2) var floorTex: texture_2d<f32>;
@group(1) @binding(3) var iceTex: texture_2d_array<f32>;
@group(1) @binding(4) var landA: texture_2d<f32>;
@group(1) @binding(5) var landB: texture_2d<f32>;
@group(1) @binding(6) var linearS: sampler;     // wrap east-west, clamp north-south, mips

const G_SHELF = vec3f(211.0, 240.0, 238.0) / 255.0;
const G_OPEN = vec3f(194.0, 228.0, 244.0) / 255.0;
const G_DEEP = vec3f(169.0, 210.0, 236.0) / 255.0;
const G_COAST = vec3f(94.0, 156.0, 192.0) / 255.0;
const G_ICE = vec3f(241.0, 245.0, 248.0) / 255.0;
const G_ATMOS = vec3f(220.0, 238.0, 251.0) / 255.0;
const PAPER = vec3f(244.0, 241.0, 234.0) / 255.0;

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let pix = in.position.xy;
    let R = U.disc.z * U.frame.z;                        // the radius in pixels
    let scale = U.frame.z * U.frame.w;                   // pixels a reading point
    let zoom = U.disc.w;
    let G = ground_at(U, pix);
    let rr = sqrt(G.r2);
    let z = G.v.z;
    // the page: paper with a little grain; the ground's air, a thin bright haze just outside the limb
    let grain = vnoise(pix / 1.7) * 0.5 + vnoise(pix / 9.0) * 0.5;
    var page = PAPER * (0.985 + 0.03 * grain);
    let ringpx = (rr - 1.0) * R;
    page = mix(page, G_ATMOS, 0.85 * exp(-max(ringpx, 0.0) / 3.5) * select(0.0, 1.0, rr >= 1.0));
    let fwRing = fwidth(ringpx);
    // (every derivative at the top level, before any pixel leaves)
    let gr = uv_grad(G.uv);
    let g = textureSampleGrad(geoTex, linearS, G.uv, gr.dx, gr.dy);
    let t = textureSampleGrad(terrainTex, linearS, G.uv, gr.dx, gr.dy);
    let fine = (g.r * 255.0 - 128.0) / 8.0;
    let coarse = (g.g * 255.0 - 128.0) * 2.0;
    let d = select(fine, coarse, abs(coarse) > 14.0);
    let hill = (t.r * 255.0 - 128.0) / 127.0;
    let height = t.g * 255.0 - 128.0;
    let fwD = fwidth(d);
    let landa = smoothstep(-0.5, 0.5, d / max(fwD, 1e-5));
    let k = clamp(zoom, 1.0, 8.0);
    let landCol = mix(textureSampleGrad(landA, linearS, G.uv, gr.dx, gr.dy).rgb,
                      textureSampleGrad(landB, linearS, G.uv, gr.dx, gr.dy).rgb, U.time.w);
    let fl = (textureSampleGrad(floorTex, linearS, G.uv, gr.dx, gr.dy).r * 255.0 - 128.0) / 127.0;
    let ice = mix(textureSampleLevel(iceTex, linearS, G.uv, i32(U.time.y), 0.0).r,
                  textureSampleLevel(iceTex, linearS, G.uv, i32(U.time.z), 0.0).r, U.time.w);
    if (rr >= 1.0) { return vec4f(page, 1.0); }

    // the land: the month's picture, shaded from the north-west
    let gland = landCol * (1.0 + 0.20 * hill);
    // the sea: shelves pale, the open ocean, the abyss, the floor's relief shaded faintly
    let depthM = -min(height, 0.0) * 87.0;
    var gsea = mix(G_SHELF, G_OPEN, smoothstep(60.0, 900.0, depthM));
    gsea = mix(gsea, G_DEEP, smoothstep(2500.0, 6000.0, depthM));
    gsea *= 1.0 + 0.04 * fl;
    // the month's usual sea ice, its edge broken a little (drawn, not measured)
    var jag = 0.5;
    if (ice > 0.02) { jag = vnoise3(G.g * 220.0 + vec3f(17.3, 5.1, 9.7)) * 0.6 + vnoise3(G.g * 900.0 + vec3f(17.3, 5.1, 9.7)) * 0.4; }
    gsea = mix(gsea, G_ICE * (0.97 + 0.04 * jag), smoothstep(0.30, 0.55, ice + 0.25 * (jag - 0.5)));
    var col = mix(gsea, gland, landa);
    // the coast, lighter and finer than a map's, fading where both sides are white
    let pale = smoothstep(0.90, 0.97, (gland.r + gland.g + gland.b) / 3.0);
    col = mix(col, G_COAST, (0.55 - 0.35 * pale) * stroke(d, 0.6 * scale * min(pow(k, 0.3), 1.5), fwD));
    // roundness, and the air at the limb
    col *= 1.0 - 0.05 * (1.0 - z) * (1.0 - z);
    col = mix(col, G_ATMOS, 0.55 * pow(1.0 - z, 4.0));
    // the rim, against the page
    let edge = smoothstep(-0.5, 0.5, -ringpx / max(fwRing, 1e-5));
    return vec4f(mix(page, col, edge), 1.0);
}
