// The mold (Jones 2010, "Characteristics of pattern formation and evolution in approximations of
// Physarum transport networks"): many small agents, each sensing the chemoattractant ahead of it at
// three sensors, turning toward the strongest, stepping forward and depositing; the field diffuses
// and decays. Food sources (the Earth's lights at night, and the flakes a finger feeds) put
// attractant into the same field, so the agents gather on them and the paths between them thicken
// into tubes: a transport network, as the real mold made between oat flakes laid as Tokyo's towns
// (Tero et al. 2010).
//
// The field lies on an equirectangular grid (W x H cells, the globe's maps' layout); an agent moves
// on the sphere, its heading in its own east-north frame, so a step east is longer in longitude
// toward the poles. It cannot go to sea (ground < 0): a step that would is refused and it turns.

// home: the cell the agent holds (one agent a cell, Jones's occupancy), or NONE
struct Agent { p: vec2f, h: f32, s: u32, home: u32, _pad: u32 };
const NONE: u32 = 0xffffffffu;

struct Params {
    grid: vec4u,     // W, H, agents, the step's number
    mv: vec4f,       // sensor angle, sensor offset (cells), rotation angle, step (cells)
    tr: vec4f,       // deposit, decay, diffusion, food gain
    misc: vec4f,     // respawn chance a step, feeds, sea's sensed value, the body's easing a step
    ex: vec4f,       // x the chance a crowded agent lets go a step
    feeds: array<vec4f, 32>,   // lon, lat (radians), strength, radius (cells)
};

@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(2) var<storage, read> trail_in: array<f32>;
@group(0) @binding(3) var<storage, read_write> trail_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> body: array<f32>;      // the mold itself, seen: its cells held, eased in time
@group(0) @binding(5) var<storage, read> ground: array<f32>;
@group(0) @binding(6) var<storage, read> spawn: array<vec2f>;   // lit places, by brightness
@group(0) @binding(7) var<storage, read_write> glow: array<f32>;
@group(0) @binding(8) var<storage, read_write> occupied: array<atomic<u32>>;

const PI: f32 = 3.14159265;
const GLOW: u32 = 4u;        // the glow's cells are GLOW x GLOW of the field's

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}
fn rand(s: ptr<function, u32>) -> f32 {
    *s = pcg(*s);
    return f32(*s >> 8u) / 16777216.0;
}

fn cell(lon: f32, lat: f32) -> u32 {
    let W = P.grid.x; let H = P.grid.y;
    let fx = (lon / (2.0 * PI) + 0.5) * f32(W);
    let fy = (0.5 - lat / PI) * f32(H);
    let x = u32(i32(floor(fx)) % i32(W) + i32(W)) % W;
    let y = u32(clamp(i32(floor(fy)), 0, i32(H) - 1));
    return y * W + x;
}

// a place `d` cells along heading `h` from (lon, lat)
fn ahead(p: vec2f, h: f32, d: f32) -> vec2f {
    let c = PI / f32(P.grid.y);      // radians a cell
    let lat = clamp(p.y + d * c * sin(h), -1.5, 1.5);
    let lon = p.x + d * c * cos(h) / max(cos(p.y), 0.05);
    return vec2f(lon - 2.0 * PI * floor((lon + PI) / (2.0 * PI)), lat);
}

fn sense(p: vec2f, h: f32) -> f32 {
    let q = ahead(p, h, P.mv.y);
    let i = cell(q.x, q.y);
    return select(trail_in[i], P.misc.z, ground[i] < 0.0);
}

@compute @workgroup_size(256)
fn step_agents(@builtin(global_invocation_id) gid: vec3u) {
    let i = gid.x;
    if (i >= P.grid.z) { return; }
    var a = agents[i];
    var s = a.s ^ pcg(P.grid.w + i * 2654435761u);
    var p = a.p;
    var h = a.h;
    let sa = P.mv.x; let ra = P.mv.z;
    // one not yet settled walks on, straight but for a wobble, till it finds a free cell: the
    // mold's growing edge, pushing out from where it is crowded
    if (a.home != NONE) {
    let f = sense(p, h);
    let fl = sense(p, h + sa);
    let fr = sense(p, h - sa);
    if (f > fl && f > fr) {
    } else if (f < fl && f < fr) {
        h += select(-ra, ra, rand(&s) < 0.5);
    } else if (fl > fr) {
        h += ra;
    } else if (fr > fl) {
        h -= ra;
    }
    }
    h += (rand(&s) - 0.5) * 0.2;
    var home = a.home;
    let q = ahead(p, h, P.mv.w);
    let qi = cell(q.x, q.y);
    if (ground[qi] < 0.0 || abs(q.y) > 1.35) {
        h = rand(&s) * 2.0 * PI;
    } else if (qi == home) {
        p = q;
    } else if (atomicCompareExchangeWeak(&occupied[qi], 0u, 1u).exchanged) {
        // the cell ahead is free: it moves there, leaving its own
        if (home != NONE) { atomicStore(&occupied[home], 0u); }
        home = qi;
        p = q;
    } else {
        // taken: it stays, and turns
        if (home == NONE) {
            p = q;   // (one not yet settled goes through)
        } else if (rand(&s) < P.ex.x) {
            // crowded in: now and then it lets go and walks out to the edge, so a packed mass thins
            // into tubes and its edge keeps reaching
            atomicStore(&occupied[home], 0u);
            home = NONE;
        } else {
            h = rand(&s) * 2.0 * PI;
        }
    }
    // now and then one starts again at a lit place, so the mold's mass stays with the food
    if (rand(&s) < P.misc.x) {
        let n = arrayLength(&spawn);
        p = spawn[min(u32(rand(&s) * f32(n)), n - 1u)];
        h = rand(&s) * 2.0 * PI;
        if (home != NONE) { atomicStore(&occupied[home], 0u); }
        home = NONE;
    }
    agents[i] = Agent(p, h - 2.0 * PI * floor(h / (2.0 * PI)), s, home, 0u);
}

@compute @workgroup_size(16, 16)
fn diffuse(@builtin(global_invocation_id) gid: vec3u) {
    let W = P.grid.x; let H = P.grid.y;
    if (gid.x >= W || gid.y >= H) { return; }
    let idx = gid.y * W + gid.x;
    let g = ground[idx];
    // an agent deposits in the cell it holds (so a cell takes at most one agent's a step)
    let held = f32(atomicLoad(&occupied[idx]));
    body[idx] = body[idx] * P.misc.w + held;
    let dep = held * P.tr.x;
    if (g < 0.0) { trail_out[idx] = 0.0; return; }
    var sum = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
        let y = u32(clamp(i32(gid.y) + dy, 0, i32(H) - 1));
        for (var dx = -1; dx <= 1; dx++) {
            let x = u32((i32(gid.x) + dx + i32(W)) % i32(W));
            sum += trail_in[y * W + x];
        }
    }
    var v = mix(trail_in[idx], sum / 9.0, P.tr.z) + dep + g * P.tr.w;
    trail_out[idx] = v * P.tr.y;
}

// the flakes a finger fed, each in a box of its own round it (workgroup z the flake): a heap of food
// the size of a town, and the faint smell of it carried far, so the mold finds it from a way off
const REACH: i32 = 96;   // the box's half height, cells
@compute @workgroup_size(16, 16)
fn feed(@builtin(global_invocation_id) gid: vec3u) {
    let W = P.grid.x; let H = P.grid.y;
    let fd = P.feeds[gid.z];
    let c = PI / f32(H);
    let cx = i32(floor((fd.x / (2.0 * PI) + 0.5) * f32(W)));
    let cy = i32(floor((0.5 - fd.y / PI) * f32(H)));
    let dx = i32(gid.x) - 2 * REACH;
    let dy = i32(gid.y) - REACH;
    if (dx >= 2 * REACH || dy >= REACH) { return; }
    let y = cy + dy;
    if (y < 0 || y >= i32(H)) { return; }
    let lat = (0.5 - (f32(y) + 0.5) / f32(H)) * PI;
    let ex = f32(dx) * cos(lat);
    let ey = f32(dy);
    let d2 = ex * ex + ey * ey;
    let r = fd.w;
    let add = fd.z * (exp(-d2 / (r * r)) + 0.1 * exp(-d2 / (36.0 * r * r)));
    if (add < 1e-3) { return; }
    let x = u32(((cx + dx) % i32(W) + i32(W)) % i32(W));
    let idx = u32(y) * W + x;
    if (ground[idx] < 0.0) { return; }
    trail_out[idx] += add;
}

// the mold seen from a little way off: each glow cell the mean of GLOW x GLOW of the body's
@compute @workgroup_size(16, 16)
fn blur(@builtin(global_invocation_id) gid: vec3u) {
    let W = P.grid.x / GLOW; let H = P.grid.y / GLOW;
    if (gid.x >= W || gid.y >= H) { return; }
    var sum = 0.0;
    for (var dy = 0u; dy < GLOW; dy++) {
        for (var dx = 0u; dx < GLOW; dx++) {
            sum += body[(gid.y * GLOW + dy) * P.grid.x + gid.x * GLOW + dx];
        }
    }
    glow[gid.y * W + gid.x] = sum / f32(GLOW * GLOW);
}
