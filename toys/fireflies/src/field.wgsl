// The flash field: every firefly a soft disc on an equirectangular sheet, added up as (glow x weight,
// weight), so the night pass reads glow = r / g, each lit pixel following the fireflies nearest it,
// blended smoothly between them. One instance a firefly (and a copy across the antimeridian).

struct Inst {
    @location(0) lonlat: vec2f,   // degrees
    @location(1) glow: f32,
};

struct Out {
    @builtin(position) pos: vec4f,
    @location(0) q: vec2f,
    @location(1) glow: f32,
};

const RADIUS_DEG: f32 = 1.1;

@vertex fn vs_field(@builtin(vertex_index) vi: u32, inst: Inst) -> Out {
    let corner = vec2f(select(-1.0, 1.0, (vi & 1u) == 1u), select(-1.0, 1.0, vi >= 2u));
    let c = max(cos(radians(inst.lonlat.y)), 0.08);
    let lon = inst.lonlat.x + corner.x * RADIUS_DEG / c;
    let lat = inst.lonlat.y + corner.y * RADIUS_DEG;
    // u = lon/360 + 1/2 left to right; v = 1/2 - lat/180 top down (clip y up)
    let x = lon / 180.0;
    let y = lat / 90.0;
    var o: Out;
    o.pos = vec4f(x, y, 0.0, 1.0);
    o.q = corner;
    o.glow = inst.glow;
    return o;
}

@fragment fn fs_field(in: Out) -> @location(0) vec4f {
    let r2 = dot(in.q, in.q);
    if (r2 > 1.0) { discard; }
    let w = exp(-4.0 * r2) - exp(-4.0);
    return vec4f(in.glow * w, w, 0.0, 0.0);
}
