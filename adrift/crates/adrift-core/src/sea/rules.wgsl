// The GPU mirror of `Rules::step` (rules.rs): the same sheets, the same hash and draws, the same
// arithmetic, so a float's maybes by the thousand land where the CPU would put them (to the
// rounding of a fused multiply-add; the sheets are read with textureLoad and this file's own
// bilinear, never the sampler's fixed-point weights). Change rules.rs and this together.
//
// Bindings (group 0): the sheets as 8-bit textures, exactly the PNGs' bytes, and the step's
// uniform; a storage buffer of paths. One invocation advances one path (k = id + 1) by one step.

struct Params {
    day: f32,          // days since 2000-01-01 00:00 UTC, as f32 (its bits seed the draws)
    dt: f32,           // days
    leeway: f32,       // of the 10 m wind
    ice_hold: f32,     // concentration that holds
    wander: f32,       // 1: the measured diffusivity
    shore: f32,        // pixels off the shore a held float sits
    seed_lo: u32,
    seed_hi: u32,
    count: u32,        // paths
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct Path {
    lon: f32,
    lat: f32,
    held: u32,         // 0 free, 1 coast, 2 ice
    measured: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var currents: texture_2d<f32>;   // rgba8unorm, 2888 x 1086
@group(0) @binding(2) var wind: texture_2d<f32>;       // rgba8unorm, 1448 x 546
@group(0) @binding(3) var eddy: texture_2d<f32>;       // rgba8unorm, 722 x 362
@group(0) @binding(4) var ice: texture_2d<f32>;        // r8unorm, 5768 x 2166
@group(0) @binding(5) var coast: texture_2d<f32>;      // rg8unorm, 4096 x 2048
@group(0) @binding(6) var<storage, read_write> paths: array<Path>;

const EARTH_RADIUS: f32 = 6371000.0;
const SECONDS_A_DAY: f32 = 86400.0;
const PI: f32 = 3.14159265358979;
const VMAX: f32 = 2.0;
const WMAX: f32 = 16.0;
const KMAX: f32 = 40000.0;
const COAST_PX: f32 = 360.0 / 4096.0;

// --- the hash and the draws (rules.rs: mix, hash_seed, uniform (draw here), gaussian) ---

fn mix(v: u32) -> u32 {
    var x = v;
    x ^= x >> 16u;
    x *= 0x7feb352du;
    x ^= x >> 15u;
    x *= 0x846ca68bu;
    x ^= x >> 16u;
    return x;
}

fn hash_seed(seed_lo: u32, seed_hi: u32, k: u32, day_bits: u32) -> u32 {
    var h = mix(seed_lo ^ 0x9e3779b9u);
    h = mix(h ^ seed_hi);
    h = mix(h ^ k);
    return mix(h ^ day_bits);
}

fn draw(h: u32, i: u32) -> f32 {
    return f32(mix(h ^ (i * 0x9e3779b9u)) >> 8u) / 16777216.0;
}

fn gaussian(h: u32, i: u32) -> f32 {
    var s = 0.0;
    for (var j = 0u; j < 12u; j++) {
        s += draw(h, i + j);
    }
    return s - 6.0;
}

// --- the sheets (sheets.rs) ---

fn wrap(lon: f32) -> f32 {
    return (lon + 180.0) - 360.0 * floor((lon + 180.0) / 360.0) - 180.0;
}

struct Months { a: u32, b: u32, t: f32 };

fn months(day: f32) -> Months {
    let year = day - 365.25 * floor(day / 365.25);
    let m = (year - 15.2) / 30.4375;
    let m0 = floor(m);
    let a = u32(i32(m0) - 12 * i32(floor(m0 / 12.0)));
    return Months(a, (a + 1u) % 12u, m - m0);
}

// A tiled sheet's byte value (0..255) for channel c of month m at (lon, lat), bilinear.
fn tiled(tex: texture_2d<f32>, cells_w: f32, cells_h: f32, m: u32, c: u32, lon: f32, lat: f32) -> f32 {
    let fx = ((lon + 180.0) - 360.0 * floor((lon + 180.0) / 360.0)) / 360.0 * cells_w - 0.5;
    let fy = clamp((90.0 - lat) / 180.0 * cells_h - 0.5, -1.0, cells_h);
    let x0 = floor(fx);
    let y0 = floor(fy);
    let tx = fx - x0;
    let ty = fy - y0;
    let tw = i32(cells_w) + 2;
    let th = i32(cells_h) + 2;
    let ox = i32(m % 4u) * tw + 1;
    let oy = i32(m / 4u) * th + 1;
    let xa = ox + i32(x0);
    let ya = oy + i32(y0);
    let v00 = textureLoad(tex, vec2<i32>(xa, ya), 0)[c] * 255.0;
    let v10 = textureLoad(tex, vec2<i32>(xa + 1, ya), 0)[c] * 255.0;
    let v01 = textureLoad(tex, vec2<i32>(xa, ya + 1), 0)[c] * 255.0;
    let v11 = textureLoad(tex, vec2<i32>(xa + 1, ya + 1), 0)[c] * 255.0;
    let top = v00 * (1.0 - tx) + v10 * tx;
    let bottom = v01 * (1.0 - tx) + v11 * tx;
    return top * (1.0 - ty) + bottom * ty;
}

fn decode_current(b: f32) -> f32 {
    let s = (b - 128.0) / 127.0;
    return sign(s) * s * s * VMAX;
}

// (u, v, measured)
fn current_at(day: f32, lon: f32, lat: f32) -> vec3<f32> {
    let mo = months(day);
    let u = decode_current(tiled(currents, 720.0, 360.0, mo.a, 0u, lon, lat)) * (1.0 - mo.t)
          + decode_current(tiled(currents, 720.0, 360.0, mo.b, 0u, lon, lat)) * mo.t;
    let v = decode_current(tiled(currents, 720.0, 360.0, mo.a, 1u, lon, lat)) * (1.0 - mo.t)
          + decode_current(tiled(currents, 720.0, 360.0, mo.b, 1u, lon, lat)) * mo.t;
    let m = tiled(currents, 720.0, 360.0, mo.a, 2u, lon, lat) * (1.0 - mo.t)
          + tiled(currents, 720.0, 360.0, mo.b, 2u, lon, lat) * mo.t;
    return vec3<f32>(u, v, m / 255.0);
}

fn wind_at(day: f32, lon: f32, lat: f32) -> vec2<f32> {
    let mo = months(day);
    let u = tiled(wind, 360.0, 180.0, mo.a, 0u, lon, lat) * (1.0 - mo.t) + tiled(wind, 360.0, 180.0, mo.b, 0u, lon, lat) * mo.t;
    let v = tiled(wind, 360.0, 180.0, mo.a, 1u, lon, lat) * (1.0 - mo.t) + tiled(wind, 360.0, 180.0, mo.b, 1u, lon, lat) * mo.t;
    return vec2<f32>((u - 128.0) / 127.0 * WMAX, (v - 128.0) / 127.0 * WMAX);
}

// (K zonal, K meridional, measured)
fn eddy_at(lon: f32, lat: f32) -> vec3<f32> {
    let su = tiled(eddy, 720.0, 360.0, 0u, 0u, lon, lat) / 255.0;
    let sv = tiled(eddy, 720.0, 360.0, 0u, 1u, lon, lat) / 255.0;
    return vec3<f32>(su * su * KMAX, sv * sv * KMAX, tiled(eddy, 720.0, 360.0, 0u, 2u, lon, lat) / 255.0);
}

fn ice_at(day: f32, lon: f32, lat: f32) -> f32 {
    let mo = months(day);
    return (tiled(ice, 1440.0, 720.0, mo.a, 0u, lon, lat) * (1.0 - mo.t)
          + tiled(ice, 1440.0, 720.0, mo.b, 0u, lon, lat) * mo.t) / 255.0;
}

// The signed distance to the shore in the coast sheet's pixels (no tiles, no border).
fn coast_distance(lon: f32, lat: f32) -> f32 {
    let w = 4096.0;
    let h = 2048.0;
    let fx = ((lon + 180.0) - 360.0 * floor((lon + 180.0) / 360.0)) / 360.0 * w - 0.5;
    let fy = clamp((90.0 - lat) / 180.0 * h - 0.5, 0.0, h - 1.0);
    let x0f = floor(fx);
    let y0f = floor(fy);
    let tx = fx - x0f;
    let ty = fy - y0f;
    let x0 = (i32(x0f) + 4096) % 4096;
    let x1 = (x0 + 1) % 4096;
    let y0 = i32(y0f);
    let y1 = min(y0 + 1, 2047);
    let p00 = textureLoad(coast, vec2<i32>(x0, y0), 0).rg * 255.0;
    let p10 = textureLoad(coast, vec2<i32>(x1, y0), 0).rg * 255.0;
    let p01 = textureLoad(coast, vec2<i32>(x0, y1), 0).rg * 255.0;
    let p11 = textureLoad(coast, vec2<i32>(x1, y1), 0).rg * 255.0;
    let top = p00 * (1.0 - tx) + p10 * tx;
    let bottom = p01 * (1.0 - tx) + p11 * tx;
    let v = top * (1.0 - ty) + bottom * ty;
    let fine = (v.x - 128.0) / 8.0;
    if (abs(fine) < 15.0) {
        return fine;
    }
    return (v.y - 128.0) * 2.0;
}

// --- the step (rules.rs: step, ashore) ---

struct Shore { lon: f32, lat: f32, held: u32 };

fn ashore(lon: f32, lat: f32, nlon: f32, nlat: f32) -> Shore {
    let d = coast_distance(nlon, nlat);
    if (d < -params.shore) {
        return Shore(nlon, nlat, 0u);
    }
    let gx = (coast_distance(nlon + COAST_PX, nlat) - coast_distance(nlon - COAST_PX, nlat)) / 2.0;
    let gy = (coast_distance(nlon, nlat - COAST_PX) - coast_distance(nlon, nlat + COAST_PX)) / 2.0;
    let g = sqrt(gx * gx + gy * gy);
    if (g > 1e-3) {
        let back = (d + params.shore + 0.5) / g;
        let blon = wrap(nlon - back * gx * COAST_PX);
        let blat = clamp(nlat - back * gy * COAST_PX, -89.5, 89.5);
        if (coast_distance(blon, blat) < -params.shore * 0.5) {
            return Shore(blon, blat, 1u);
        }
    }
    return Shore(lon, lat, 1u);
}

fn step(lon: f32, lat: f32, k: u32) -> Path {
    let day = params.day;
    let dt = params.dt;
    let cur = current_at(day, lon, lat);
    let measured = cur.z;
    if (ice_at(day, lon, lat) >= params.ice_hold) {
        return Path(lon, lat, 2u, measured);
    }
    let w = wind_at(day, lon, lat);
    let kk = eddy_at(lon, lat);
    let secs = dt * SECONDS_A_DAY;
    let h = hash_seed(params.seed_lo, params.seed_hi, k, bitcast<u32>(day));
    let gx = gaussian(h, 0u);
    let gy = gaussian(h, 12u);
    let dx = (cur.x + params.leeway * w.x) * secs + gx * sqrt(2.0 * kk.x * secs) * params.wander;
    let dy = (cur.y + params.leeway * w.y) * secs + gy * sqrt(2.0 * kk.y * secs) * params.wander;
    let coslat = max(cos(lat * PI / 180.0), 0.01);
    let dlon = dx / (EARTH_RADIUS * coslat) * 180.0 / PI;
    let dlat = dy / EARTH_RADIUS * 180.0 / PI;
    let nlon = wrap(lon + dlon);
    let nlat = clamp(lat + dlat, -89.5, 89.5);
    if (ice_at(day, nlon, nlat) >= params.ice_hold) {
        return Path(lon, lat, 2u, measured);
    }
    let s = ashore(lon, lat, nlon, nlat);
    return Path(s.lon, s.lat, s.held, measured);
}

@compute @workgroup_size(64)
fn maybes(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.count) {
        return;
    }
    let p = paths[i];
    paths[i] = step(p.lon, p.lat, i + 1u);
}
