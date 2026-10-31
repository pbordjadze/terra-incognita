// The rules on the GPU: the mirror of Rules::step (rules.rs), run by gpu/maybes.rs over thousands of
// seeds at once for a held float's maybes. Truth keeps this and rules.rs the same, step for step:
// the same (place, day, dt, seed, k) gives the same answer on both. R0 (Engineering): the mean
// current alone, held by the coast and the ice; every k is the same until the eddy wander comes.
//
// Bindings (group 1) are the sea's sheets as sea.wgsl reads them: currents, geo, the ice's layers.

fn rules_tile(m: f32) -> vec2f { return vec2f((m % 4.0) * 722.0, floor(m / 4.0) * 362.0); }

// eastward and northward current (m/s) and how measured, for month m at a place (degrees)
fn rules_current_month(m: f32, at: vec2f) -> vec3f {
    let uv = vec2f(fract((at.x + 180.0) / 360.0), (90.0 - at.y) / 180.0);
    let st = (rules_tile(m) + 1.0 + uv * vec2f(720.0, 360.0)) / vec2f(2888.0, 1086.0);
    let c = textureSampleLevel(rCurrents, rHeld, st, 0.0).rgb;
    let s = (c.rg * 255.0 - 128.0) / 127.0;
    return vec3f(sign(s) * s * s * 2.0, c.b);
}

// the date's months: this one, the next, how far between (model.rs Clock::months)
fn rules_months(day: f32) -> vec3f {
    let yd = day - 365.25 * floor(day / 365.25);
    let m = yd / 365.25 * 12.0 - 0.5;
    let mm = m - 12.0 * floor(m / 12.0);
    let a = floor(mm);
    return vec3f(a % 12.0, (a + 1.0) % 12.0, mm - a);
}

fn rules_current(day: f32, at: vec2f) -> vec2f {
    let mo = rules_months(day);
    let a = rules_current_month(mo.x, at);
    let b = rules_current_month(mo.y, at);
    return mix(a.xy, b.xy, mo.z);
}

fn rules_is_land(at: vec2f) -> bool {
    let uv = vec2f(fract((at.x + 180.0) / 360.0), clamp((90.0 - at.y) / 180.0, 0.0, 1.0));
    let g = textureSampleLevel(rGeo, rEastWest, uv, 0.0);
    let fine = (g.r * 255.0 - 128.0) / 8.0;
    let coarse = (g.g * 255.0 - 128.0) * 2.0;
    let d = select(fine, coarse, abs(coarse) > 14.0);
    return d > 0.0;
}

fn rules_ice(day: f32, at: vec2f) -> f32 {
    let uv = vec2f(fract((at.x + 180.0) / 360.0), clamp((90.0 - at.y) / 180.0, 0.0, 1.0));
    let mo = rules_months(day);
    let a = textureSampleLevel(rIce, rEastWest, uv, i32(mo.x), 0.0).r;
    let b = textureSampleLevel(rIce, rEastWest, uv, i32(mo.y), 0.0).r;
    return mix(a, b, mo.z);
}

// a place moved by a velocity (m/s) for so many days, along the ground
fn rules_advance(at: vec2f, v: vec2f, dt_days: f32) -> vec2f {
    let s = dt_days * 86400.0 / 6371000.0;
    let dlat = v.y * s;
    let c = max(cos(radians(at.y)), 0.02);
    let dlon = v.x * s / c;
    return vec2f(at.x + degrees(dlon), clamp(at.y + degrees(dlat), -89.0, 89.0));
}

fn rules_normalize(at: vec2f) -> vec2f {
    var lon = at.x + 180.0;
    lon = lon - 360.0 * floor(lon / 360.0) - 180.0;
    return vec2f(lon, at.y);
}

// one step: the new place and what holds it (0 free, 1 the coast, 2 the ice)
fn rules_step(at: vec2f, day: f32, dt: f32, seed: u32, k: u32) -> vec3f {
    if (rules_is_land(at)) { return vec3f(at, 1.0); }
    if (rules_ice(day, at) >= 0.45) { return vec3f(at, 2.0); }
    let v0 = rules_current(day, at);
    let trial = rules_advance(at, v0, dt);
    let v1 = rules_current(day, trial);
    let next = rules_normalize(rules_advance(at, (v0 + v1) * 0.5, dt));
    if (rules_is_land(next)) { return vec3f(at, 1.0); }
    return vec3f(next, 0.0);
}
