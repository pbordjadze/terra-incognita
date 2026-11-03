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
const N_HALO = vec3f(255.0, 146.0, 58.0) / 255.0;     // the pool of light round a city, deeper sodium
const N_AIR = vec3f(196.0, 120.0, 72.0) / 255.0;      // the wide glow over a lit region

fn k0(zoom: f32) -> f32 { return clamp(zoom, 1.0, 8.0); }
fn grainAt(pix: vec2f) -> f32 { return vnoise(pix / 1.7) * 0.5 + vnoise(pix / 9.0) * 0.5; }
// the page: paper with a little grain; the ground's air, a thin bright haze just outside the limb
fn paper(pix: vec2f, rr: f32, ringpx: f32, night: f32) -> vec3f {
    let grain = grainAt(pix);
    let page = mix(PAPER * (0.985 + 0.03 * grain), N_PAGE * (0.97 + 0.06 * grain), night);
    return mix(page, mix(G_ATMOS, N_ATMOS, night), mix(0.85, 0.7, night) * exp(-max(ringpx, 0.0) / mix(3.5, 5.0, night)) * select(0.0, 1.0, rr >= 1.0));
}

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let pix = in.position.xy;
    let R = U.disc.z * U.frame.z;                        // the radius in pixels
    let scale = U.frame.z * U.frame.w;                   // pixels a reading point
    let zoom = U.disc.w;
    let G = ground_at(U, pix);
    let rr = sqrt(G.r2);
    let z = G.v.z;
    let night = U.pad.x;
    // (R24: the grain and the page are worked out only for the pixels that show them, `paper` below)
    let ringpx = (rr - 1.0) * R;
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
    let k = k0(zoom);
    // (R24) the reads below are each made only where what they give can show: the month's picture
    // where there is land or the coast's line (`pale`), the lights by night, the floor where there is
    // sea; the same pixels, fewer reads (an export frame is mostly open water)
    let landa = smoothstep(-0.5, 0.5, d / max(fwD, 1e-5));
    let coastW = stroke(d, 0.6 * scale * min(pow(k0(zoom), 0.3), 1.5), fwD);
    let needLand = landa > 0.0 || coastW > 0.0;
    var landCol = vec3f(0.0);
    var grain = 0.5;
    if (needLand) {
    landCol = mix(textureSampleGrad(landA, linearS, G.uv, gr.dx, gr.dy).rgb,
                      textureSampleGrad(landB, linearS, G.uv, gr.dx, gr.dy).rgb, U.time.w);
    // magnified past its own pixels (a clip close in, R16), the month's picture would show as blocks,
    // and its dark sea pixels as notches along the drawn coast: it eases toward the picture read a few
    // mips coarser (its broad colour) with a fine grain at the screen's own scale, a soft painted
    // ground rather than an enlarged photograph; the coast stays the distance field's
    let adims = vec2f(textureDimensions(landA));
    let alod = log2(max(max(length(gr.dx * adims), length(gr.dy * adims)), 1e-5));
    let soft = mix(textureSampleLevel(landA, linearS, G.uv, 3.0).rgb, textureSampleLevel(landB, linearS, G.uv, 3.0).rgb, U.time.w);
    let magnified = smoothstep(-1.0, -3.0, alod);
    if (magnified > 0.0) { grain = grainAt(pix); }
    landCol = mix(landCol, soft * (0.97 + 0.06 * grain), 0.85 * magnified);
    }
    var fl = 0.0;
    if (landa < 1.0) { fl = (textureSampleGrad(floorTex, linearS, G.uv, gr.dx, gr.dy).r * 255.0 - 128.0) / 127.0; }
    let ice = mix(textureSampleLevel(iceTex, linearS, G.uv, i32(U.time.y), 0.0).r,
                  textureSampleLevel(iceTex, linearS, G.uv, i32(U.time.z), 0.0).r, U.time.w);
    // the Earth's lights (NASA's Black Marble, Terra's sheet), sampled before any pixel leaves
    var lightsRaw = 0.0; var halo = 0.0; var wide = 0.0;
    if (night > 0.0) {
    lightsRaw = textureSampleGrad(lightsTex, linearS, G.uv, gr.dx, gr.dy).r;
    // and their glow, as a long exposure from orbit sees it (R6): the same sheet read a few mips
    // coarser than the pixel's own, a near halo and a wide one, so a city sits in a pool of its own
    // light that spills a little over its coast; drawn from the measured lights, never invented
    let ldims = vec2f(textureDimensions(lightsTex));
    let lod0 = log2(max(max(length(gr.dx * ldims), length(gr.dy * ldims)), 1e-4));
    halo = textureSampleLevel(lightsTex, linearS, G.uv, max(lod0 + 2.5, 0.0)).r;
    wide = textureSampleLevel(lightsTex, linearS, G.uv, max(lod0 + 4.5, 0.0)).r;
    }
    if (rr >= 1.0) { return vec4f(paper(pix, rr, ringpx, night), 1.0); }

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
    col = mix(col, G_COAST, (0.55 - 0.35 * pale) * coastW);
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
        // alive, not flickering: a slow shimmer over whole regions (as Terra's night), a finer and
        // quicker one over streets and suburbs, the whole field breathing over nine seconds, and the
        // brightest cores catching now and then, as lights seen from far do
        let t = U.tone.z;
        // (R24: each noise worked out only where it shows: the slow shimmer where there are lights or
        // their glow, the fine one and the catching where there are lights; elsewhere they're scaled
        // by nothing)
        var shimmer = 0.0;
        if (lightsRaw > 0.0 || wide > 0.0 || halo > 0.0) { shimmer = vnoise3(G.g * 1400.0 + vec3f(t * 0.45, t * 0.31, 7.9)); }
        var fine = 0.0; var catching = 0.0;
        if (lightsRaw > 0.0) {
            fine = vnoise3(G.g * 6000.0 + vec3f(t * 0.9, -t * 0.7, 3.1));
            catching = smoothstep(0.82, 1.0, vnoise3(G.g * 9000.0 + vec3f(-t * 1.3, t * 1.1, 11.0)));
        }
        let breath = 0.96 + 0.04 * sin(t * 0.7);
        // (R22, a clip only: U.pad.y) the fine and quick shimmers and the catching are the bits a
        // re-encode spends and smears; a clip keeps the slow shimmer and calms the rest
        let ex = U.pad.y;
        var lights = pow(lightsRaw, 0.75) * (0.74 + 0.28 * shimmer + 0.24 * mix(fine, 0.5, 0.7 * ex)) * breath * smoothstep(0.45, 1.0, night);
        lights = lights + 0.5 * (1.0 - 0.6 * ex) * catching * smoothstep(0.6, 1.0, lights);
        // close in, flying over a city: each street's light blinks a tiny bit of its own, quick and small
        let close = smoothstep(3.5, 5.5, zoom);
        let cell = floor(G.g * 24000.0);
        let blink = 0.5 + 0.5 * sin(t * (5.0 + 4.0 * hash13(cell)) + 6.2832 * hash13(cell + 1.0));
        lights = lights * (1.0 + (1.0 - ex) * close * 0.12 * (blink - 0.5));
        // a light's colour by how bright it is, as the photographs from orbit show it: a village a
        // deep sodium ember, a town gold, a city's heart whitening
        let ember = mix(N_HALO * 0.95, N_LIGHT, smoothstep(0.12, 0.6, lights));
        nland = mix(nland, ember, smoothstep(0.0, 0.85, lights));
        nland = mix(nland, N_CORE, 0.7 * smoothstep(0.7, 1.0, lights));
        var nsea = mix(N_SEA, N_DEEP, smoothstep(2500.0, 6000.0, depthM)) * (1.0 + 0.06 * fl);
        nsea = mix(nsea, N_ICE * (0.97 + 0.04 * jag), smoothstep(0.30, 0.55, ice + 0.25 * (jag - 0.5)));
        var ncol = mix(nsea, nland, landa);
        // the glow over all, land and the water by it: the wide one a breath of warm air over a lit
        // region, the near one the pool each town sits in, breathing with the field's slow shimmer
        let pool = smoothstep(0.45, 1.0, night) * breath;
        ncol = ncol + N_AIR * 0.26 * pow(wide, 0.9) * pool * (0.85 + 0.3 * shimmer);
        ncol = ncol + N_HALO * 0.5 * pow(halo, 0.9) * pool * (0.8 + 0.4 * shimmer) * mix(0.55, 1.0, landa);
        ncol = mix(ncol, N_COAST, 0.5 * stroke(d, 0.6 * scale * min(pow(k, 0.3), 1.5), fwD));
        ncol *= 1.0 - 0.35 * pow(1.0 - z, 2.0);
        ncol = mix(ncol, N_ATMOS, 0.45 * pow(1.0 - z, 5.0));
        col = mix(col, ncol, night);
    }
    // the rim, against the page
    let edge = smoothstep(-0.5, 0.5, -ringpx / max(fwRing, 1e-5));
    var out = col;
    if (edge < 1.0) { out = mix(paper(pix, rr, ringpx, night), col, edge); }
    // (R22, a clip only) the night's darkest tones lifted out of the near-black a re-encode crushes
    // into blocks (its video black is 16 of 255), and a grain of a part in 255 laid over the whole
    // picture so the deep sea's long gradients don't band: the night still reads as night
    let ex = U.pad.y;
    if (ex > 0.0) {
        let lift = vec3f(0.030, 0.038, 0.056) * night;
        out = out * (1.0 - 0.06 * night) + lift;
        // (fixed to the screen, never changing in time: a moving grain is what an encoder pays most for)
        let tri = hash2(pix) + hash2(pix * 1.37 + 41.0) - 1.0;
        out = out + vec3f(tri * 1.2 / 255.0);
    }
    return vec4f(out, 1.0);
}
