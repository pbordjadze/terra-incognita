// The GPU mirror of `AirRules::step` (wind.rs): things on the wind, by the thousand. The same
// sheets, hash, lattices and arithmetic, so a maybe lands where the CPU would put it (to the
// rounding of a fused multiply-add). Change wind.rs and this together.
//
// Bindings (group 0): the step's uniform; the wind, gusts and coast sheets as 8-bit textures (the
// PNGs' bytes, read with textureLoad and this file's own bilinear); a storage buffer of paths
// (`GpuPath`, the sea's layout). One invocation advances one path (k = id + 1) by one step.

struct Params {
    day: f32,
    dt: f32,
    carry: f32,        // of the 10 m wind
    gust: f32,         // 1: the measured weather
    fall: f32,         // per day
    washout: f32,      // per day per mm/day
    seed_lo: u32,
    seed_hi: u32,
    count: u32,
    base: u32,
    _pad1: u32,
    _pad2: u32,
};

struct Path {
    lon: f32,
    lat: f32,
    held: u32,         // 0 in the air, 1 down on land, 2 down on the sea
    rain: f32,         // the month's mean rain where it last was (mm/day; for the look)
    gust_u: f32,       // the gust it carries (m/s)
    gust_v: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var wind: texture_2d<f32>;            // rgba8unorm, 1448 x 546
@group(0) @binding(2) var gusts: texture_2d<f32>;           // rgba8unorm, 1448 x 546
@group(0) @binding(3) var coast: texture_2d<f32>;           // rgba8unorm, 4096 x 2048 (Terra's geo.png)
@group(0) @binding(4) var<storage, read_write> paths: array<Path>;

const EARTH_RADIUS: f32 = 6371000.0;
const SECONDS_A_DAY: f32 = 86400.0;
const PI: f32 = 3.14159265358979;
const WMAX: f32 = 16.0;
const SMAX: f32 = 12.0;
const TMAX: f32 = 5.0;
const PMAX: f32 = 25.0;
const CELL_X: f32 = 15.0;
const CELL_Y: f32 = 8.0;
const RAIN_CELL: f32 = 2.5;
const AIR_SPAN: u32 = 8u;
const GUST_STREAM: u32 = 0x411700u;
const RAIN_STREAM: u32 = 0x413b00u;

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

fn bytes_gaussian(h: u32) -> f32 {
    let s = (h & 255u) + ((h >> 8u) & 255u) + ((h >> 16u) & 255u) + (h >> 24u);
    return ((f32(s) + 2.0) / 256.0 - 2.0) * 1.7320508;
}

fn air_field(k: u32, cell: i32, lon: f32, lat: f32) -> vec2<f32> {
    let n = i32(360.0 / CELL_X);
    let fx = (lon + 180.0) / CELL_X;
    let fy = (lat + 90.0) / CELL_Y;
    let x0f = floor(fx);
    let y0f = floor(fy);
    let tx = fx - x0f;
    let ty = fy - y0f;
    let base = mix(mix(k ^ 0xa1e50b0eu) ^ bitcast<u32>(cell));
    let x0 = i32(x0f);
    let y0 = i32(y0f);
    var g = vec2<f32>(0.0, 0.0);
    for (var c = 0; c < 4; c++) {
        let dx = c & 1;
        let dy = c >> 1u;
        let w = select(1.0 - tx, tx, dx == 1) * select(1.0 - ty, ty, dy == 1);
        let ix = u32((((x0 + dx) % n) + n) % n);
        let h = mix(mix(base ^ ix) ^ bitcast<u32>(y0 + dy));
        g.x += w * bytes_gaussian(h);
        g.y += w * bytes_gaussian(mix(h ^ 0x68e31da4u));
    }
    let a = (1.0 - tx) * (1.0 - tx) + tx * tx;
    let b = (1.0 - ty) * (1.0 - ty) + ty * ty;
    return g * (1.0 / sqrt(a * b));
}

struct Span { first: i32, count: u32 };

fn air_span(day: f32, dt: f32) -> Span {
    let first = i32(ceil((day - 0.125) * 4.0));
    let end = i32(ceil((day + dt - 0.125) * 4.0));
    if (end > first) {
        return Span(first, min(u32(end - first), AIR_SPAN));
    }
    return Span(i32(floor(day * 4.0)), 1u);
}

fn air_kick(k: u32, day: f32, dt: f32, lon: f32, lat: f32) -> vec2<f32> {
    let s = air_span(day, dt);
    var g = vec2<f32>(0.0, 0.0);
    for (var i = 0u; i < s.count; i++) {
        g += air_field(k, s.first + i32(i), lon, lat);
    }
    return g * (1.0 / sqrt(f32(s.count)));
}

fn air_rain(k: u32, day: f32, dt: f32, lon: f32, lat: f32, rate: f32) -> bool {
    let s = air_span(day, dt);
    let p = 1.0 - exp(-rate * dt / f32(s.count));
    let n = i32(360.0 / RAIN_CELL);
    let ix = u32(((i32(floor((lon + 180.0) / RAIN_CELL + 0.5)) % n) + n) % n);
    let iy = u32(i32(floor((lat + 90.0) / RAIN_CELL + 0.5)));
    for (var i = 0u; i < s.count; i++) {
        let h = mix(mix(mix(mix((k ^ RAIN_STREAM) ^ 0xa1e50b0eu) ^ bitcast<u32>(s.first + i32(i))) ^ ix) ^ iy);
        if (f32(h >> 8u) / 16777216.0 < p) {
            return true;
        }
    }
    return false;
}

fn wrap(lon: f32) -> f32 {
    return (lon + 180.0) - 360.0 * floor((lon + 180.0) / 360.0) - 180.0;
}

struct Months { a: u32, b: u32, t: f32 };

fn months(day: f32) -> Months {
    let year = day - 365.25 * floor(day / 365.25);
    let m = (year - 15.21875) / 30.4375;
    let m0 = floor(m);
    let a = u32(i32(m0) - 12 * i32(floor(m0 / 12.0)));
    return Months(a, (a + 1u) % 12u, m - m0);
}

// A 360 x 180 tiled sheet's four channels (0..255) of month m at (lon, lat), bilinear.
fn tiled4(tex: texture_2d<f32>, m: u32, lon: f32, lat: f32) -> vec4<f32> {
    let cells_w = 360.0;
    let cells_h = 180.0;
    let fx = ((lon + 180.0) - 360.0 * floor((lon + 180.0) / 360.0)) / 360.0 * cells_w - 0.5;
    let fy = clamp((90.0 - lat) / 180.0 * cells_h - 0.5, -1.0, cells_h);
    let x0 = floor(fx);
    let y0 = floor(fy);
    let tx = fx - x0;
    let ty = fy - y0;
    let ox = i32(m % 4u) * 362 + 1;
    let oy = i32(m / 4u) * 182 + 1;
    let xa = ox + i32(x0);
    let ya = oy + i32(y0);
    let v00 = textureLoad(tex, vec2<i32>(xa, ya), 0) * 255.0;
    let v10 = textureLoad(tex, vec2<i32>(xa + 1, ya), 0) * 255.0;
    let v01 = textureLoad(tex, vec2<i32>(xa, ya + 1), 0) * 255.0;
    let v11 = textureLoad(tex, vec2<i32>(xa + 1, ya + 1), 0) * 255.0;
    let top = v00 * (1.0 - tx) + v10 * tx;
    let bottom = v01 * (1.0 - tx) + v11 * tx;
    return top * (1.0 - ty) + bottom * ty;
}

fn wind_at(day: f32, lon: f32, lat: f32) -> vec2<f32> {
    let mo = months(day);
    let w = tiled4(wind, mo.a, lon, lat) * (1.0 - mo.t) + tiled4(wind, mo.b, lon, lat) * mo.t;
    return vec2<f32>((w.x - 128.0) / 127.0 * WMAX, (w.y - 128.0) / 127.0 * WMAX);
}

struct Weather { sigma: vec2<f32>, time: f32, rain: f32 };

fn decode_weather(p: vec4<f32>) -> Weather {
    let s = p / 255.0;
    return Weather(vec2<f32>(s.x * s.x * SMAX, s.y * s.y * SMAX), s.z * TMAX, s.w * s.w * PMAX);
}

fn weather_at(day: f32, lon: f32, lat: f32) -> Weather {
    let mo = months(day);
    let a = decode_weather(tiled4(gusts, mo.a, lon, lat));
    let b = decode_weather(tiled4(gusts, mo.b, lon, lat));
    return Weather(a.sigma * (1.0 - mo.t) + b.sigma * mo.t, a.time * (1.0 - mo.t) + b.time * mo.t, a.rain * (1.0 - mo.t) + b.rain * mo.t);
}

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

fn step(p: Path, k: u32) -> Path {
    if (p.held != 0u) {
        return p;
    }
    let day = params.day;
    let dt = params.dt;
    let lon = p.lon;
    let lat = p.lat;
    let wv = wind_at(day, lon, lat);
    let w = weather_at(day, lon, lat);
    let secs = dt * SECONDS_A_DAY;
    let t = max(w.time, 0.25) * SECONDS_A_DAY;
    let al = exp(-secs / t);
    let kick = sqrt(1.0 - al * al) * params.gust;
    let g = air_kick(k ^ GUST_STREAM, day, dt, lon, lat);
    let gu = p.gust_u * al + g.x * w.sigma.x * kick;
    let gv = p.gust_v * al + g.y * w.sigma.y * kick;
    let dx = (params.carry * wv.x + gu) * secs;
    let dy = (params.carry * wv.y + gv) * secs;
    let coslat = max(cos(lat * PI / 180.0), 0.01);
    let nlon = wrap(lon + dx / (EARTH_RADIUS * coslat) * 180.0 / PI);
    let nlat = clamp(lat + dy / EARTH_RADIUS * 180.0 / PI, -89.5, 89.5);
    let fall = draw(hash_seed(params.seed_lo, params.seed_hi, k, bitcast<u32>(day)), 0u) < 1.0 - exp(-params.fall * dt);
    let down = air_rain(k, day, dt, lon, lat, params.washout * w.rain) || fall;
    if (!down) {
        return Path(nlon, nlat, 0u, w.rain, gu, gv);
    }
    let held = select(2u, 1u, coast_distance(nlon, nlat) > 0.0);
    return Path(nlon, nlat, held, w.rain, 0.0, 0.0);
}

@compute @workgroup_size(64)
fn maybes(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.count) {
        return;
    }
    let j = params.base + i;
    paths[j] = step(paths[j], i + 1u);
}
