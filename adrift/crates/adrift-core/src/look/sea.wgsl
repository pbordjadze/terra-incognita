// The sea, one pixel at a time, over the ground: Terra's Ocean.metal ported (its comments say what is
// measured and what is drawn; tools/ocean/SOURCES.md). The streamlines are the contours of the month's
// stream function, light runs along them the way the water goes; close in, glitter and surf; rings
// where it is touched. Visual's from R1: the sea at rest (the light settles while the date is stopped).

@group(0) @binding(0) var<uniform> U: View;
@group(1) @binding(0) var currents: texture_2d<f32>;
@group(1) @binding(1) var stream: texture_2d<f32>;
@group(1) @binding(2) var geoTex: texture_2d<f32>;
@group(1) @binding(3) var iceTex: texture_2d_array<f32>;
@group(1) @binding(4) var tile: texture_3d<f32>;
@group(1) @binding(5) var oceanHeld: sampler;        // clamp
@group(1) @binding(6) var oceanEastWest: sampler;    // wrap east-west, clamp north-south
@group(1) @binding(7) var oceanWrap: sampler;        // repeat, for the noise tile
struct SeaUniforms { ripple: array<vec4f, 8> };      // four touches: centre (world) and when, then strength
@group(2) @binding(0) var<uniform> S: SeaUniforms;

fn gnoise(p: vec3f) -> f32 { return textureSampleLevel(tile, oceanWrap, p * (1.0 / 16.0), 0.0).r * 2.0 - 1.0; }

// the month's cell of a sheet: its tile's origin, in texels
fn tileOf(m: f32) -> vec2f { return vec2f((m % 4.0) * 722.0, floor(m / 4.0) * 362.0); }

// eastward and northward current (m/s) and how measured, for month m at uv
fn currentAt(m: f32, uv: vec2f) -> vec3f {
    let st = (tileOf(m) + 1.0 + uv * vec2f(720.0, 360.0)) / vec2f(2888.0, 1086.0);
    let c = textureSampleLevel(currents, oceanHeld, st, 0.0).rgb;
    let s = (c.rg * 255.0 - 128.0) / 127.0;
    return vec3f(sign(s) * s * s * 2.0, c.b);
}

// the stream function (m/s radians) for month m at uv: four texels, their bytes joined, then mixed
fn streamTexel(t: vec2i) -> f32 {
    let b = floor(textureLoad(stream, t, 0).rg * 255.0 + 0.5);
    return (b.x * 256.0 + b.y) / 65535.0 * 0.4 - 0.2;
}
fn streamAt(m: f32, uv: vec2f) -> f32 {
    let p = uv * vec2f(720.0, 360.0) + 0.5;
    let i = floor(p); let f = p - i;
    let t = vec2i(tileOf(m) + i);
    let a = streamTexel(t); let b = streamTexel(t + vec2i(1, 0));
    let c = streamTexel(t + vec2i(0, 1)); let d = streamTexel(t + vec2i(1, 1));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// the distance in pixels from a contour of q (one a unit; fw is fwidth(q)), and how much of the set
// shows: all of it, unless its lines would crowd closer than a few pixels
fn contour(q: f32, fw: f32) -> vec2f {
    return vec2f(abs(q - floor(q + 0.5)) / max(fw, 1e-6), 1.0 - smoothstep(0.16, 0.36, fw));
}

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let uPx = U.frame.z;
    let R = U.disc.z; let zoom = U.disc.w; let reading = U.frame.w; let t = U.time.x; let tm = U.tone.z;
    let G = ground_at(U, in.position.xy);
    let v = G.v.xy; let z = G.v.z; let g = G.g; let lat = G.lat; let uv = G.uv;
    let r2 = G.r2;

    // where the sea is: the globe's coast (land positive, in its texels), the month's ice
    let geo = textureSampleLevel(geoTex, oceanEastWest, uv, 0.0);
    let fine = (geo.r * 255.0 - 128.0) / 8.0; let coarse = (geo.g * 255.0 - 128.0) * 2.0;
    let d = select(coarse, fine, abs(coarse) < 15.0);
    let fwD = fwidth(d);
    let sea = 1.0 - smoothstep(-0.5, 0.5, d / max(fwD, 1e-5));
    let shore = -fine * (PI / 2048.0) * mix(1.0, cos(lat), 0.5) * R;
    let iceC = mix(textureSampleLevel(iceTex, oceanEastWest, uv, i32(U.time.y), 0.0).r,
                   textureSampleLevel(iceTex, oceanEastWest, uv, i32(U.time.z), 0.0).r, U.time.w);
    let ice = smoothstep(0.30, 0.55, iceC);

    // the month's water: its current and stream function, between this month and the next
    let c0 = currentAt(U.time.y, uv); let c1 = currentAt(U.time.z, uv);
    let cur = mix(c0, c1, U.time.w);
    let psi = mix(streamAt(U.time.y, uv), streamAt(U.time.z, uv), U.time.w);
    let speed = length(cur.xy); let measured = cur.z;
    let east = normalize(vec3f(g.z, 0.0, -g.x) + vec3f(1e-6, 0.0, 0.0));
    let north = cross(g, east);
    let flow = east * cur.x + north * cur.y;
    let slope = east * cur.y - north * cur.x;
    let rot = mat3x3f(U.rot0.xyz, U.rot1.xyz, U.rot2.xyz);
    let ax = rot * vec3f(1.0, 0.0, -v.x / max(z, 0.05)); let ay = rot * vec3f(0.0, 1.0, -v.y / max(z, 0.05));
    let fwPsi = max(length(vec2f(dot(slope, ax), dot(slope, ay))), 1e-9) / (R * uPx);

    if (r2 >= 1.0 || sea <= 0.0) { return vec4f(0.0); }
    let water = sea * (1.0 - ice) * smoothstep(0.0, 0.12, z) * U.tone.x;
    if (water <= 0.0) { return vec4f(0.0); }
    var acc = vec4f(0.0);

    // -- the sheen: the sky's light on the water, from the upper left, soft and broad
    let toLight = normalize(vec3f(-0.42, 0.55, 0.72));
    let n = vec3f(v, z);
    let h = normalize(toLight + vec3f(0.0, 0.0, 1.0));
    let sheen = pow(max(dot(n, h), 0.0), 18.0) * (1.0 - smoothstep(1.5, 3.0, zoom));
    acc = over(acc, vec3f(1.0), 0.10 * sheen * water * (1.0 - U.pad.x));

    // -- the streamlines, in octaves fixed to the globe and blended between
    let gap = 9.0 * reading; let ref_ = 0.5;
    let level = log2(gap * ref_ / R);
    let k = floor(level); let blend = level - k;
    let delta = exp2(k);
    let sparse = contour(psi / (2.0 * delta), fwPsi / (2.0 * delta));
    var dense = contour(psi / delta, fwPsi / delta);
    dense.y *= smoothstep(0.0, 0.6, 1.0 - blend);
    let lineW = 0.75 * uPx * reading; let haloW = 4.0 * uPx * reading;
    let core = max(sparse.y * (1.0 - smoothstep(lineW * 0.5, lineW * 0.5 + 1.0, sparse.x)),
                   dense.y * (1.0 - smoothstep(lineW * 0.5, lineW * 0.5 + 1.0, dense.x)));
    let halo = max(sparse.y * exp(-sparse.x * sparse.x / (haloW * haloW)), dense.y * exp(-dense.x * dense.x / (haloW * haloW)));
    let running = smoothstep(0.04, 0.40, speed) * measured;
    acc = over(acc, mix(vec3f(0.20, 0.42, 0.62), vec3f(0.62, 0.74, 0.86), U.pad.x), mix(0.22, 0.30, U.pad.x) * core * running * water);

    // -- light running along them: noise carried by the current, each bright place trailing a tail
    // upstream, so a still picture shows which way the water goes
    let pulse = 46.0 * reading;
    let plevel = log2(R / pulse);
    let pk = floor(plevel); let pb = plevel - pk;
    let pace = 26.0 * reading / R;
    let along = flow / max(speed, 1e-4);
    var glow = 0.0; var w2 = 0.0;
    for (var o = 0; o < 2; o++) {
        let F = exp2(pk + f32(o));
        let wo = select(pb, 1.0 - pb, o == 0);
        for (var ph = 0; ph < 2; ph++) {
            let cyc = tm / 5.0 + 0.5 * f32(ph);
            let age = fract(cyc);
            let wp = 1.0 - abs(2.0 * age - 1.0);
            let p = (g - flow * pace * age * 5.0) * F + 37.0 * floor(cyc) + 11.0 * f32(o);
            var tail = 0.0;
            for (var s = 0; s < 3; s++) { tail += gnoise(p + along * (0.22 * f32(s))) * (1.0 - 0.3 * f32(s)); }
            glow += wo * wp * tail / 2.1;
            w2 += wo * wo * wp * wp;
        }
    }
    glow = smoothstep(0.3, 1.5, glow / (0.185 * 0.86 * sqrt(w2))) * running * mix(0.45, 1.0, smoothstep(1.0, 2.2, zoom));
    // the sea at rest: while the date is stopped the light holds where it is (tm is held) and settles to
    // half its strength over a beat (U.tone.w eases 0..1 as the date stops: DEMANDS.md D3), so a still
    // frame reads as light lying along the lines, not a flash caught mid-air
    glow *= mix(1.0, 0.5, U.tone.w);
    acc = over(acc, vec3f(0.96, 0.99, 1.0), (0.24 * halo + 0.6 * core) * glow * water);

    // -- close in, the wind on the water: patches of glitter drifting slowly
    let near = smoothstep(2.6, 4.2, zoom) * U.tone.y * (1.0 - U.tone.w);
    if (near > 0.0) {
        let gusts = smoothstep(-0.4, 1.1, gnoise(g * R / (420.0 * reading) + vec3f(0.0, tm * 0.02, tm * 0.03)) / 0.185);
        let cell = 15.0 * reading;
        let glevel = log2(R / cell);
        let gk = floor(glevel); let gb = glevel - gk;
        var glint = 0.0;
        for (var o = 0; o < 2; o++) {
            let F = exp2(gk + f32(o));
            let wo = select(gb, 1.0 - gb, o == 0);
            let c = floor(g * F);
            let id = hash13(c + 0.37);
            let at = normalize(c + 0.25 + 0.5 * vec3f(hash13(c + 1.7), hash13(c + 5.3), hash13(c + 9.1)));
            let off = (v - (transpose(rot) * at).xy) * R / reading;
            let period = 2.5 + 3.5 * hash13(c + 13.9);
            let flash = pow(max(sin(6.2832 * (tm / period + id)), 0.0), 32.0);
            let there = step(id, 0.5 * gusts);
            let star = exp(-dot(off, off) / 0.45) + 0.28 * exp(-off.x * off.x / 3.5 - off.y * off.y / 0.1)
                     + 0.28 * exp(-off.y * off.y / 3.5 - off.x * off.x / 0.1);
            glint += wo * there * flash * star;
        }
        // (quieted: by day a little, by night to a trace, so the water's glitter never draws the eye from a
        // float's lamp; the land's lights are what lives by night)
        acc = over(acc, vec3f(1.0), min(glint, 1.0) * mix(0.6, 0.12, U.pad.x) * near * water * smoothstep(0.0, 6.0, shore));
    }

    // -- close in, the surf: swells coming in to the coast, a set at a time, and a line of foam
    let surfing = smoothstep(3.2, 5.0, zoom);
    if (surfing > 0.0 && shore < 24.0 * reading && fine > -15.5) {
        let reach = 18.0 * reading; let spacing = 5.5 * reading;
        let swellSet = gnoise(g * R / (220.0 * reading) + vec3f(tm * 0.05)) / 0.185;
        let arriving = tm / 3.4 + 0.5 * swellSet;
        let phase = shore / spacing + arriving;
        let swell = exp(-sq((fract(phase) - 0.5) * spacing / (0.55 * reading)));
        let fade = (1.0 - smoothstep(0.3 * reach, reach, shore)) * smoothstep(1.5, 3.5, shore);
        let breaking = 0.5 + 0.5 * cos(6.2832 * arriving);
        let lace = 0.6 + 0.4 * smoothstep(-0.8, 0.8, gnoise(g * R / (5.0 * reading)) / 0.185);
        let foam = exp(-sq(max(shore - 0.3 * reading, 0.0) / ((0.8 + 1.1 * breaking) * reading))) * lace;
        // (by night the foam is a line, not a glare)
        acc = over(acc, vec3f(1.0), (0.28 * swell * fade + 0.8 * foam) * surfing * water * mix(1.0, 0.35, U.pad.x));
    }

    // -- touched: rings spreading on the water from each touch
    for (var i = 0; i < 4; i++) {
        let at = S.ripple[2 * i]; let how = S.ripple[2 * i + 1];
        let age = t - at.w;
        if (how.x <= 0.0 || age < 0.0 || age > 5.0) { continue; }
        let dPt = acos(clamp(dot(g, at.xyz), -1.0, 1.0)) * R;
        let r = 34.0 * reading * pow(mix(0.8, age, U.tone.y), 0.75);
        var light = 0.0; var shade = 0.0;
        for (var j = 0; j < 3; j++) {
            let x = (dPt - (r - f32(j) * 12.0 * reading)) / reading;
            let a = (1.0 - 0.28 * f32(j)) * step(0.0, r - f32(j) * 12.0 * reading);
            light += a * exp(-sq((x + 1.4) / 2.0));
            shade += a * exp(-sq((x - 2.6) / 2.4));
        }
        let strength = how.x * exp(-age / 1.5) * smoothstep(0.0, 0.12, age) / (1.0 + r / (90.0 * reading));
        acc = over(acc, vec3f(0.13, 0.32, 0.50), 0.20 * shade * strength * water);
        acc = over(acc, vec3f(1.0), 0.65 * light * strength * water);
    }
    return acc;
}
