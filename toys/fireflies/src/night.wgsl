// The night, with the fireflies in its lights: the fireflies' own night, not the kit's (round 2: the
// kit's slate map, its coasts and lakes stroked, read as a map between flashes). The ground is near
// black, land a breath warmer than the sea, no line drawn anywhere but a faint air at the limb; the
// measured lights (NASA's Black Marble) are the picture, warm and softly bloomed as the photographs
// from orbit show them, so the cities are beautiful while still. Each lit pixel follows the firefly
// nearest it (the flash field, field.wgsl): resting a little low between flashes, rising past its
// measured brightness at one, its colour and its bloom leaning toward a firefly's yellow-green.

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

// a: x the rest level, y the flash's gain, z the firefly tint, w the bloom; b: x the sim's seconds,
// y the ring's width (radians); rings: per ring, (where, when it left) and (speed in radians a second,
// its life in seconds, 1 if in use, unused)
struct Look { a: vec4f, b: vec4f, rings: array<vec4f, 8> };
@group(1) @binding(9) var<uniform> L: Look;

const LAND = vec3f(11.0, 11.0, 14.0) / 255.0;
const SEA = vec3f(5.0, 7.0, 12.0) / 255.0;
const PAGE = vec3f(3.0, 4.0, 7.0) / 255.0;
const AIR = vec3f(40.0, 62.0, 104.0) / 255.0;        // the thin air at the limb
const EMBER = vec3f(150.0, 52.0, 12.0) / 255.0;      // a village, a deep sodium ember
const AMBER = vec3f(255.0, 150.0, 60.0) / 255.0;     // a town
const GOLD = vec3f(255.0, 212.0, 140.0) / 255.0;     // a city
const CORE = vec3f(255.0, 246.0, 226.0) / 255.0;     // a city's heart, whitening
const GLOW = vec3f(255.0, 128.0, 40.0) / 255.0;      // the light round a city, in the air
// a firefly's light: Photinus' and Pteroptyx's yellow-green (their peaks near 560 nm)
const FLY = vec3f(190.0, 255.0, 90.0) / 255.0;
const FLY_CORE = vec3f(240.0, 255.0, 200.0) / 255.0;

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let pix = in.position.xy;
    let R = U.disc.z * U.frame.z;
    let G = ground_at(U, pix);
    let rr = sqrt(G.r2);
    let z = G.v.z;
    let ringpx = (rr - 1.0) * R;
    let fwRing = fwidth(ringpx);
    // the page: near black, the air a faint glow just outside the limb
    var page = PAGE + AIR * 0.35 * exp(-max(ringpx, 0.0) / (4.0 * U.frame.z)) * select(0.0, 1.0, rr >= 1.0);
    let gr = uv_grad(G.uv);
    let g = textureSampleGrad(geoTex, linearS, G.uv, gr.dx, gr.dy);
    let coarse = (g.g * 255.0 - 128.0) * 2.0;
    // land or sea, softly (no coast is drawn: the land is only a breath warmer than the water)
    let landa = smoothstep(-3.0, 3.0, coarse);
    let lightsRaw = textureSampleGrad(lightsTex, linearS, G.uv, gr.dx, gr.dy).r;
    let ldims = vec2f(textureDimensions(lightsTex));
    let lod0 = log2(max(max(length(gr.dx * ldims), length(gr.dy * ldims)), 1e-4));
    let halo = textureSampleLevel(lightsTex, linearS, G.uv, max(lod0 + 2.0, 0.0)).r;
    let near = textureSampleLevel(lightsTex, linearS, G.uv, max(lod0 + 3.5, 0.0)).r;
    let wide = textureSampleLevel(lightsTex, linearS, G.uv, max(lod0 + 5.5, 0.0)).r;
    // the fireflies here: their glow, blended by nearness (r / g), where any are
    let fs = textureSampleLevel(fieldTex, linearS, G.uv, 0.0);
    let cover = smoothstep(0.0, 0.12, fs.g);
    let glow = mix(0.2, fs.r / max(fs.g, 1e-4), cover);
    let level = L.a.x + L.a.y * glow;
    if (rr >= 1.0) { return vec4f(page, 1.0); }

    var col = mix(SEA, LAND, landa);
    // the measured lights, each at its firefly's level, with a slow shimmer as lights seen from far
    let tt = U.tone.z;
    let shimmer = vnoise3(G.g * 1400.0 + vec3f(tt * 0.45, tt * 0.31, 7.9));
    let lit = 1.25 * pow(lightsRaw, 0.7) * (0.9 + 0.2 * shimmer);
    let l = lit * level;
    let tint = L.a.z * smoothstep(0.3, 1.0, glow) * cover;
    // a light's colour by how bright it is: ember, amber, gold, a whitening heart
    var lc = mix(EMBER, AMBER, smoothstep(0.05, 0.35, l));
    lc = mix(lc, GOLD, smoothstep(0.35, 0.8, l));
    lc = mix(lc, CORE, smoothstep(0.8, 1.6, l));
    lc = mix(lc, mix(FLY, FLY_CORE, smoothstep(0.9, 1.8, l)), tint);
    col = col + lc * min(l, 1.6) * 0.95;
    // the bloom round them, as a long exposure from orbit sees it, rising and falling with them: the
    // pool a town sits in, the warm air over a lit region, spilling over its coast
    let airc = mix(GLOW, FLY, tint);
    col = col + airc * (0.55 * pow(halo, 1.0) + 0.40 * pow(near, 1.3) + 0.22 * pow(wide, 1.8)) * level;
    // a flash's own bloom: the light a swarm in step throws into the night round it
    let flash = smoothstep(0.35, 1.0, glow) * cover * L.a.w;
    col = col + FLY * (0.45 * pow(near, 1.0) + 0.45 * pow(wide, 1.2)) * flash;
    // a finger's flash, spreading from where it touched as light, not a line: a soft wavefront with a
    // long fading wake behind it, faint in the dark air over the sea, lifting the city lights it passes
    // over (they flash as it reaches them: sim.rs touch)
    var wave = 0.0;
    var spot = 0.0;
    for (var k = 0; k < 4; k++) {
        let p = L.rings[2 * k];
        let q = L.rings[2 * k + 1];
        if (q.z > 0.5) {
            let age = L.b.x - p.w;
            if (age >= 0.0 && age < q.y) {
                let ang = acos(clamp(dot(G.g, p.xyz), -1.0, 1.0));
                let dlt = ang - q.x * age;
                let w = L.b.y * (1.0 + 0.5 * age);
                let front = select(exp(dlt / (5.0 * w)), exp(-sq(dlt / w)), dlt > 0.0);
                let fade = (1.0 - smoothstep(0.0, q.y, age)) * smoothstep(0.0, 0.1, age);
                wave = wave + front * fade / (1.0 + 2.0 * q.x * age);
                spot = spot + exp(-sq(ang / (4.0 * L.b.y))) * exp(-age / 0.4);
            }
        }
    }
    col = col + FLY * min(wave, 1.2) * (0.045 + 1.1 * lit);
    col = col + FLY_CORE * 0.12 * min(spot, 1.0);
    // a soft shoulder, so a bright heart whitens instead of clipping to a flat block
    col = vec3f(1.0) - exp(-col * 1.15);
    // roundness: darker toward the limb, and the thin air there
    col *= 1.0 - 0.45 * pow(1.0 - z, 2.0);
    col = col + AIR * 0.30 * pow(1.0 - z, 6.0);
    let edge = smoothstep(-0.5, 0.5, -ringpx / max(fwRing, 1e-5));
    return vec4f(mix(page, col, edge), 1.0);
}
