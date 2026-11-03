// The night, with the fireflies in its lights. The kit's ground by night (toy-kit globe/ground.wgsl's
// N_ palette, its night branch alone, copied Nov 3, 2026), with one change: each lit pixel's light
// follows the firefly nearest it (the flash field, field.wgsl), resting low between flashes and
// rising past its measured brightness at one, its colour leaning from sodium toward a firefly's
// yellow-green as it does. Nothing is drawn that the lights aren't: the fireflies are the cities.

@group(0) @binding(0) var<uniform> U: View;
@group(1) @binding(0) var geoTex: texture_2d<f32>;
@group(1) @binding(1) var terrainTex: texture_2d<f32>;
@group(1) @binding(2) var floorTex: texture_2d<f32>;
@group(1) @binding(3) var iceTex: texture_2d_array<f32>;
@group(1) @binding(4) var landA: texture_2d<f32>;
@group(1) @binding(5) var landB: texture_2d<f32>;
@group(1) @binding(6) var linearS: sampler;
@group(1) @binding(7) var lightsTex: texture_2d<f32>;
@group(1) @binding(8) var fieldTex: texture_2d<f32>;

struct Look { a: vec4f };   // x the rest level, y the flash's gain, z the firefly tint, w the bloom
@group(1) @binding(9) var<uniform> L: Look;

const N_LAND = vec3f(30.0, 36.0, 48.0) / 255.0;
const N_SEA = vec3f(16.0, 24.0, 40.0) / 255.0;
const N_DEEP = vec3f(9.0, 16.0, 30.0) / 255.0;
const N_ICE = vec3f(68.0, 78.0, 96.0) / 255.0;
const N_COAST = vec3f(60.0, 88.0, 116.0) / 255.0;
const N_ATMOS = vec3f(92.0, 130.0, 184.0) / 255.0;
const N_PAGE = vec3f(14.0, 16.0, 22.0) / 255.0;
const N_LIGHT = vec3f(255.0, 214.0, 134.0) / 255.0;   // sodium gold
const N_CORE = vec3f(255.0, 244.0, 220.0) / 255.0;    // a city's core, whitening
const N_HALO = vec3f(255.0, 146.0, 58.0) / 255.0;     // the pool of light round a city
const N_AIR = vec3f(196.0, 120.0, 72.0) / 255.0;      // the wide glow over a lit region
// a firefly's light: Photinus' and Pteroptyx's yellow-green (their peaks near 560 nm)
const FLY = vec3f(214.0, 255.0, 128.0) / 255.0;
const FLY_CORE = vec3f(244.0, 255.0, 214.0) / 255.0;

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let pix = in.position.xy;
    let R = U.disc.z * U.frame.z;
    let scale = U.frame.z * U.frame.w;
    let zoom = U.disc.w;
    let G = ground_at(U, pix);
    let rr = sqrt(G.r2);
    let z = G.v.z;
    let grain = vnoise(pix / 1.7) * 0.5 + vnoise(pix / 9.0) * 0.5;
    var page = N_PAGE * (0.97 + 0.06 * grain);
    let ringpx = (rr - 1.0) * R;
    page = mix(page, N_ATMOS, 0.7 * exp(-max(ringpx, 0.0) / 5.0) * select(0.0, 1.0, rr >= 1.0));
    let fwRing = fwidth(ringpx);
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
    let landCol = mix(textureSampleLevel(landA, linearS, G.uv, 2.0).rgb, textureSampleLevel(landB, linearS, G.uv, 2.0).rgb, U.time.w);
    let fl = (textureSampleGrad(floorTex, linearS, G.uv, gr.dx, gr.dy).r * 255.0 - 128.0) / 127.0;
    let ice = mix(textureSampleLevel(iceTex, linearS, G.uv, i32(U.time.y), 0.0).r,
                  textureSampleLevel(iceTex, linearS, G.uv, i32(U.time.z), 0.0).r, U.time.w);
    let lightsRaw = textureSampleGrad(lightsTex, linearS, G.uv, gr.dx, gr.dy).r;
    let ldims = vec2f(textureDimensions(lightsTex));
    let lod0 = log2(max(max(length(gr.dx * ldims), length(gr.dy * ldims)), 1e-4));
    let halo = textureSampleLevel(lightsTex, linearS, G.uv, max(lod0 + 2.5, 0.0)).r;
    let wide = textureSampleLevel(lightsTex, linearS, G.uv, max(lod0 + 4.5, 0.0)).r;
    // the fireflies here: their glow, blended by nearness (r / g), where any are
    let fs = textureSampleLevel(fieldTex, linearS, G.uv, 0.0);
    let cover = smoothstep(0.0, 0.12, fs.g);
    let glow = mix(0.15, fs.r / max(fs.g, 1e-4), cover);
    let level = L.a.x + L.a.y * glow;
    if (rr >= 1.0) { return vec4f(page, 1.0); }

    let depthM = -min(height, 0.0) * 87.0;
    var jag = 0.5;
    if (ice > 0.02) { jag = vnoise3(G.g * 220.0 + vec3f(17.3, 5.1, 9.7)) * 0.6 + vnoise3(G.g * 900.0 + vec3f(17.3, 5.1, 9.7)) * 0.4; }
    let lum = dot(landCol, vec3f(0.30, 0.59, 0.11));
    var nland = N_LAND * (0.62 + 0.4 * lum) * (1.0 + 0.25 * hill);
    // the measured lights, each at its firefly's level (a slow shimmer kept from the kit's night)
    let tt = U.tone.z;
    let shimmer = vnoise3(G.g * 1400.0 + vec3f(tt * 0.45, tt * 0.31, 7.9));
    var lights = pow(lightsRaw, 0.75) * (0.86 + 0.28 * shimmer) * level;
    let tint = L.a.z * smoothstep(0.25, 0.9, glow) * cover;
    let ember = mix(mix(N_HALO * 0.95, N_LIGHT, smoothstep(0.12, 0.6, lights)), FLY, tint);
    nland = mix(nland, ember, smoothstep(0.0, 0.85, lights));
    nland = mix(nland, mix(N_CORE, FLY_CORE, tint), 0.7 * smoothstep(0.75, 1.3, lights));
    var nsea = mix(N_SEA, N_DEEP, smoothstep(2500.0, 6000.0, depthM)) * (1.0 + 0.06 * fl);
    nsea = mix(nsea, N_ICE * (0.97 + 0.04 * jag), smoothstep(0.30, 0.55, ice + 0.25 * (jag - 0.5)));
    var ncol = mix(nsea, nland, landa);
    // the glow round them, rising and falling with them: the wide one over a lit region, the near
    // one the pool each town sits in, spilling a little over its coast
    let airCol = mix(N_AIR, FLY * 0.8, tint);
    let haloCol = mix(N_HALO, FLY, tint);
    ncol = ncol + airCol * 0.26 * pow(wide, 0.9) * level;
    ncol = ncol + haloCol * 0.5 * pow(halo, 0.9) * level * mix(0.55, 1.0, landa);
    // a flash's bloom: the light a swarm in step throws into the air round it, only while it flashes
    let flash = smoothstep(0.35, 1.0, glow) * cover * L.a.w;
    ncol = ncol + FLY * (0.55 * pow(halo, 0.8) + 0.35 * pow(wide, 0.8)) * flash;
    ncol = mix(ncol, N_COAST, 0.5 * stroke(d, 0.6 * scale * min(pow(k, 0.3), 1.5), fwD));
    ncol *= 1.0 - 0.35 * pow(1.0 - z, 2.0);
    ncol = mix(ncol, N_ATMOS, 0.45 * pow(1.0 - z, 5.0));
    let edge = smoothstep(-0.5, 0.5, -ringpx / max(fwRing, 1e-5));
    return vec4f(mix(page, ncol, edge), 1.0);
}
