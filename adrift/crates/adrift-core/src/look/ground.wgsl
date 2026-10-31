// The ground: the whole Earth as satellites saw it this month, in Terra's light key (Globe.metal's
// G_ palette, ported plainly; Visual takes it from here). One fragment a pixel inverts the orthographic
// projection and samples the map's distance fields, so the coast stays a constant width at any zoom.
// By night (U.pad.x, DEMANDS.md D4) the N_ palette: the land a dark slate shaded by the month's picture,
// the sea darker and the abyss darkest, the ice a pale grey-blue, the coast a faint line, the air's glow
// at the limb, the page near-black; the cities' lights come with their sheet (Terra's night.jpg).

@group(0) @binding(0) var<uniform> U: View;
@group(1) @binding(0) var geoTex: texture_2d<f32>;
@group(1) @binding(1) var terrainTex: texture_2d<f32>;
@group(1) @binding(2) var floorTex: texture_2d<f32>;
@group(1) @binding(3) var iceTex: texture_2d_array<f32>;
@group(1) @binding(4) var landA: texture_2d<f32>;
@group(1) @binding(5) var landB: texture_2d<f32>;
@group(1) @binding(6) var linearS: sampler;     // wrap east-west, clamp north-south, mips
@group(1) @binding(7) var lightsTex: texture_2d<f32>;   // the Earth's lights at night (U.pad.x is the night)

const G_SHELF = vec3f(211.0, 240.0, 238.0) / 255.0;
const G_OPEN = vec3f(194.0, 228.0, 244.0) / 255.0;
const G_DEEP = vec3f(169.0, 210.0, 236.0) / 255.0;
const G_COAST = vec3f(94.0, 156.0, 192.0) / 255.0;
const G_ICE = vec3f(241.0, 245.0, 248.0) / 255.0;
const G_ATMOS = vec3f(220.0, 238.0, 251.0) / 255.0;
const PAPER = vec3f(244.0, 241.0, 234.0) / 255.0;
const N_LAND = vec3f(30.0, 36.0, 48.0) / 255.0;
const N_SEA = vec3f(16.0, 24.0, 40.0) / 255.0;
const N_DEEP = vec3f(9.0, 16.0, 30.0) / 255.0;
const N_ICE = vec3f(68.0, 78.0, 96.0) / 255.0;
const N_COAST = vec3f(60.0, 88.0, 116.0) / 255.0;
const N_ATMOS = vec3f(92.0, 130.0, 184.0) / 255.0;
const N_PAGE = vec3f(14.0, 16.0, 22.0) / 255.0;
const N_LIGHT = vec3f(255.0, 214.0, 134.0) / 255.0;   // sodium gold
const N_CORE = vec3f(255.0, 244.0, 220.0) / 255.0;    // a city's core, whitening

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let pix = in.position.xy;
    let R = U.disc.z * U.frame.z;                        // the radius in pixels
    let scale = U.frame.z * U.frame.w;                   // pixels a reading point
    let zoom = U.disc.w;
    let G = ground_at(U, pix);
    let rr = sqrt(G.r2);
    let z = G.v.z;
    // the page: paper with a little grain; the ground's air, a thin bright haze just outside the limb
    let night = U.pad.x;
    let grain = vnoise(pix / 1.7) * 0.5 + vnoise(pix / 9.0) * 0.5;
    var page = mix(PAPER * (0.985 + 0.03 * grain), N_PAGE * (0.97 + 0.06 * grain), night);
    let ringpx = (rr - 1.0) * R;
    page = mix(page, mix(G_ATMOS, N_ATMOS, night), mix(0.85, 0.7, night) * exp(-max(ringpx, 0.0) / mix(3.5, 5.0, night)) * select(0.0, 1.0, rr >= 1.0));
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
    // the Earth's lights (NASA's Black Marble, Terra's sheet), sampled before any pixel leaves
    let lightsRaw = textureSampleGrad(lightsTex, linearS, G.uv, gr.dx, gr.dy).r;
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
    if (night > 0.0) {
        // the night: the land a slate shaded by the picture's own light and the hills, the sea by depth,
        // the ice grey-blue, the coast faint; a sphere by night, darker at the limb, the air's glow at it
        let lum = dot(landCol, vec3f(0.30, 0.59, 0.11));
        var nland = N_LAND * (0.75 + 0.5 * lum) * (1.0 + 0.25 * hill);
        // the measured lights over the drawn land, lifted a little (a village's glow is faint in the
        // record), in sodium gold whitening at a city's core, each shimmering slowly by the drawn
        // motion's clock as lights seen from far do: a noise in the world and in time, never a flicker
        let shimmer = vnoise3(G.g * 1400.0 + vec3f(U.tone.z * 0.45, U.tone.z * 0.31, 7.9));
        let lights = pow(lightsRaw, 0.75) * (0.86 + 0.28 * shimmer) * smoothstep(0.45, 1.0, night);
        nland = mix(nland, N_LIGHT, smoothstep(0.0, 0.85, lights));
        nland = mix(nland, N_CORE, 0.7 * smoothstep(0.7, 1.0, lights));
        var nsea = mix(N_SEA, N_DEEP, smoothstep(2500.0, 6000.0, depthM)) * (1.0 + 0.06 * fl);
        nsea = mix(nsea, N_ICE * (0.97 + 0.04 * jag), smoothstep(0.30, 0.55, ice + 0.25 * (jag - 0.5)));
        var ncol = mix(nsea, nland, landa);
        ncol = mix(ncol, N_COAST, 0.5 * stroke(d, 0.6 * scale * min(pow(k, 0.3), 1.5), fwD));
        ncol *= 1.0 - 0.35 * pow(1.0 - z, 2.0);
        ncol = mix(ncol, N_ATMOS, 0.45 * pow(1.0 - z, 5.0));
        col = mix(col, ncol, night);
    }
    // the rim, against the page
    let edge = smoothstep(-0.5, 0.5, -ringpx / max(fwRing, 1e-5));
    return vec4f(mix(page, col, edge), 1.0);
}
