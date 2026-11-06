// Frost: Reiter's hexagonal automaton for a snow crystal (C. A. Reiter, Chaos, Solitons & Fractals 23,
// 2005), one grid a crystal, each at its own angle on the pane (SOURCES.md). The numpy reference is
// tools/toys/frost/reiter.py; change both together.
//
// A grid is N x N cells in axial hex coordinates (q, r), the seed at the middle; a cell's six
// neighbours are (q±1, r), (q, r±1), (q+1, r-1), (q-1, r+1); on the pane a cell is at
// centre + cell * rotate(q + r/2, r sqrt(3)/2). s is the cell's water: s >= 1 is ice.
// A step: a cell that is ice or touches ice is receptive and takes gamma from the air in front of the
// glass; the rest's water diffuses (receptive cells count 0); beyond the growing region the air holds
// beta, the pane's humidity where the cell is. A cell on another crystal's ice, or off the glass, is a
// sink (0): the crystals meet at a seam.

const N: i32 = 512;
const HALF: i32 = 256;
const MARGIN: i32 = 28;
const EMPTY: u32 = 0xffffffffu;
// glass held by a finger's frozen print (no crystal's: the crystals grow round it)
const HOAR: u32 = 0xfffffff0u;

struct Slot {
  center: vec2<f32>,
  rot: vec2<f32>,
  id: u32,
  live: u32,
  _a: f32,
  _b: f32,
};

struct Globals {
  pane: vec2<f32>,      // points
  cell: f32,            // points a cell (and a mask texel)
  fs: f32,              // field texels a point
  mask_w: u32,
  mask_h: u32,
  field_w: u32,
  field_h: u32,
  beta_top: f32,
  beta_bottom: f32,
  gamma_top: f32,
  gamma_bottom: f32,
  eps: f32,
  alpha: f32,
  frame: f32,          // the window frame's width, points
  bar: f32,            // the crossbar's middle, points from the top (0: none)
  fringe: f32,         // how far frost creeps back into a melt from its edge, points (on average)
  relax: f32,          // how much shallower every glaze gets this frame (the holes heal), points
  _g1: f32,
  _g2: f32,
  slots: array<Slot, 12>,
};

struct Op {
  slot: u32,
  kind: u32,          // 1 seed the slot, 2 melt along the finger's way, 4 a moist print along it,
                      // 8 probe the ice under _d, 16 the stove's warmth (clear within radius of at)
  _a: u32,
  _b: u32,
  at: vec2<f32>,      // the finger's way this frame, from at to to (points)
  radius: f32,
  _c: f32,            // the print's wobble (a phase)
  to: vec2<f32>,
  _d: vec2<f32>,
};

@group(0) @binding(0) var<uniform> G: Globals;
@group(0) @binding(1) var<storage, read> src: array<f32>;
@group(0) @binding(2) var<storage, read_write> dst: array<f32>;
@group(0) @binding(3) var<storage, read_write> mask: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> stats: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read_write> field: array<f32>;
@group(0) @binding(6) var<uniform> O: Op;
// the glaze: where a melt froze again smooth, how far inside its edge (points, as f32 bits; 0 is glass
// frost can catch on). Frost grows into it only a fringe deep (fringe_at): a wipe stays clear.
@group(0) @binding(7) var<storage, read_write> glaze: array<atomic<u32>>;

fn hexdist(q: i32, r: i32) -> i32 {
  return (abs(q) + abs(r) + abs(q + r)) / 2;
}

// where a cell of slot k is on the pane, points
fn pane_of(k: u32, q: i32, r: i32) -> vec2<f32> {
  let sl = G.slots[k];
  let x = (f32(q) + 0.5 * f32(r)) * G.cell;
  let y = f32(r) * 0.8660254 * G.cell;
  return sl.center + vec2<f32>(x * sl.rot.x - y * sl.rot.y, x * sl.rot.y + y * sl.rot.x);
}

// the pane's humidity at a height: wetter low on the glass (the warm room's air pools there)
fn wet(p: vec2<f32>) -> f32 {
  return clamp(p.y / G.pane.y, 0.0, 1.0);
}
fn beta_at(p: vec2<f32>) -> f32 {
  let t = wet(p);
  return mix(G.beta_top, G.beta_bottom, t * t * (3.0 - 2.0 * t));
}
fn hash(p: vec2<f32>) -> f32 {
  return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
// smooth value noise: the glass's flaws, a few cells across
fn flaws(x: vec2<f32>) -> f32 {
  let i = floor(x);
  let f = x - i;
  let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), w.x), mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), w.x), w.y);
}
fn gamma_at(p: vec2<f32>) -> f32 {
  let t = wet(p);
  let g = exp(mix(log(G.gamma_top), log(G.gamma_bottom), t));
  // the glass's own flaws, fixed in place: a speck takes a little more, a clean patch less
  let h = flaws(p / (2.5 * G.cell)) * 2.0 - 1.0;
  return g * (1.0 + G.eps * h);
}

// the glass: inside the frame, off the crossbar
fn on_glass(p: vec2<f32>) -> bool {
  let f = G.frame;
  if p.x < f || p.y < f || p.x >= G.pane.x - f || p.y >= G.pane.y - f {
    return false;
  }
  if G.bar > 0.0 && abs(p.y - G.bar) <= 0.5 * f {
    return false;
  }
  // the corner where the two help marks sit stays clear (the lamp's warmth, as it were)
  let a = vec2<f32>(G.pane.x - 70.0, G.pane.y - 34.0);
  let b = vec2<f32>(G.pane.x - 26.0, G.pane.y - 34.0);
  let t = clamp(dot(p - a, b - a) / dot(b - a, b - a), 0.0, 1.0);
  return distance(p, a + t * (b - a)) > 32.0;
}

fn mask_index(p: vec2<f32>) -> i32 {
  if !on_glass(p) {
    return -1;
  }
  let m = vec2<i32>(floor(p / G.cell));
  if m.x < 0 || m.y < 0 || m.x >= i32(G.mask_w) || m.y >= i32(G.mask_h) {
    return -1;
  }
  return m.y * i32(G.mask_w) + m.x;
}

// how deep into a melt the frost may creep back here: ragged, a few points to twice the mean
fn fringe_at(p: vec2<f32>) -> f32 {
  return G.fringe * (0.45 + 1.1 * flaws(p / 7.0) + 0.3 * flaws(p / 2.3));
}

fn glazed(m: i32, p: vec2<f32>) -> bool {
  return bitcast<f32>(atomicLoad(&glaze[m])) > fringe_at(p);
}

fn at(k: u32, q: i32, r: i32) -> u32 {
  return k * u32(N * N) + u32(r + HALF) * u32(N) + u32(q + HALF);
}

// s at a cell (beyond the grid: the air's beta)
fn s_of(k: u32, q: i32, r: i32) -> f32 {
  if abs(q) >= HALF || abs(r) >= HALF {
    return -1.0;
  }
  return src[at(k, q, r)];
}

fn ice(v: f32) -> bool {
  return v >= 1.0;
}

// receptive: ice, or touching ice
fn receptive(k: u32, q: i32, r: i32) -> bool {
  return ice(s_of(k, q, r)) || ice(s_of(k, q + 1, r)) || ice(s_of(k, q - 1, r)) || ice(s_of(k, q, r + 1))
    || ice(s_of(k, q, r - 1)) || ice(s_of(k, q + 1, r - 1)) || ice(s_of(k, q - 1, r + 1));
}

fn u_of(k: u32, q: i32, r: i32) -> f32 {
  if receptive(k, q, r) {
    return 0.0;
  }
  return max(s_of(k, q, r), 0.0);
}

@compute @workgroup_size(8, 8, 1)
fn grow(@builtin(global_invocation_id) gid: vec3<u32>) {
  let k = gid.z;
  if gid.x >= u32(N) || gid.y >= u32(N) || G.slots[k].live == 0u {
    return;
  }
  let q = i32(gid.x) - HALF;
  let r = i32(gid.y) - HALF;
  let d = hexdist(q, r);
  let reach = i32(atomicLoad(&stats[k * 4u]));
  let i = at(k, q, r);
  if d > reach + MARGIN || d >= HALF - 2 {
    return; // the far air: beta, in both buffers since the seed
  }
  let s = src[i];
  let p = pane_of(k, q, r);
  if !ice(s) {
    // another crystal's ice, or off the glass: a sink
    let m = mask_index(p);
    if m < 0 {
      dst[i] = 0.0;
      return;
    }
    let owner = atomicLoad(&mask[m]);
    if (owner != EMPTY && owner != G.slots[k].id) || glazed(m, p) {
      dst[i] = 0.0;
      return;
    }
  }
  let rec = receptive(k, q, r);
  var u = 0.0;
  var v = 0.0;
  if rec {
    v = s + gamma_at(p);
  } else {
    u = s;
  }
  let mean = (u_of(k, q + 1, r) + u_of(k, q - 1, r) + u_of(k, q, r + 1) + u_of(k, q, r - 1) + u_of(k, q + 1, r - 1)
    + u_of(k, q - 1, r + 1)) / 6.0;
  dst[i] = u + 0.5 * G.alpha * (mean - u) + v;
}

// after a step: newly frozen cells claim their place on the pane (the lowest id wins a tie, so the
// result is the same whatever order the GPU runs them in), widen the region and are counted
@compute @workgroup_size(8, 8, 1)
fn claim(@builtin(global_invocation_id) gid: vec3<u32>) {
  let k = gid.z;
  if gid.x >= u32(N) || gid.y >= u32(N) || G.slots[k].live == 0u {
    return;
  }
  let q = i32(gid.x) - HALF;
  let r = i32(gid.y) - HALF;
  let i = at(k, q, r);
  // (dst is the step's result; src what it was)
  if !(ice(dst[i]) && !ice(src[i])) {
    return;
  }
  let p = pane_of(k, q, r);
  let id = G.slots[k].id;
  // (the glass three cells round its ice: another crystal's arms keep that far off, a clear seam)
  let m0 = vec2<i32>(floor(p / G.cell)) - vec2<i32>(3, 3);
  for (var dy = 0; dy < 7; dy++) {
    for (var dx = 0; dx < 7; dx++) {
      let m = m0 + vec2<i32>(dx, dy);
      if m.x >= 0 && m.y >= 0 && m.x < i32(G.mask_w) && m.y < i32(G.mask_h) {
        atomicMin(&mask[m.y * i32(G.mask_w) + m.x], id);
      }
    }
  }
  atomicMax(&stats[k * 4u], u32(hexdist(q, r)));
  atomicAdd(&stats[k * 4u + 1u], 1u);
}

// one slot made new: the air at the pane's humidity, the seed at the middle (both buffers: binding 1
// is bound read-only, so the pass runs once a parity)
@compute @workgroup_size(8, 8, 1)
fn seed(@builtin(global_invocation_id) gid: vec3<u32>) {
  if gid.x >= u32(N) || gid.y >= u32(N) || (O.kind & 1u) == 0u {
    return;
  }
  let k = O.slot;
  let q = i32(gid.x) - HALF;
  let r = i32(gid.y) - HALF;
  var s = beta_at(pane_of(k, q, r));
  if q == 0 && r == 0 {
    s = 1.0;
  }
  dst[at(k, q, r)] = s;
  if gid.x == 0u && gid.y == 0u {
    atomicStore(&stats[k * 4u], 0u);
    atomicStore(&stats[k * 4u + 1u], 0u);
  }
}

// a warm fingertip: the ice under it gone, the pane's claim with it, each live crystal's cells there
// back to the air's water (run once a parity, like seed)
@compute @workgroup_size(8, 8, 1)
fn melt_cells(@builtin(global_invocation_id) gid: vec3<u32>) {
  let k = gid.z;
  if gid.x >= u32(N) || gid.y >= u32(N) || G.slots[k].live == 0u || (O.kind & 2u) == 0u {
    return;
  }
  let q = i32(gid.x) - HALF;
  let r = i32(gid.y) - HALF;
  let p = pane_of(k, q, r);
  if distance(p, melt_near(p)) <= melt_radius(p) {
    dst[at(k, q, r)] = beta_at(p);
  }
}

// the nearest point of the finger's way
fn melt_near(p: vec2<f32>) -> vec2<f32> {
  let d = O.to - O.at;
  let l = dot(d, d);
  if l < 1e-6 {
    return O.at;
  }
  return O.at + clamp(dot(p - O.at, d) / l, 0.0, 1.0) * d;
}

// a fingertip's print is not a circle: its edge wanders with the angle
fn melt_radius(p: vec2<f32>) -> f32 {
  let c = melt_near(p);
  let a = atan2(p.y - c.y, p.x - c.x);
  let w = 0.16 * sin(3.0 * a + O._c) + 0.10 * sin(5.0 * a + 2.0 * O._c) + 0.07 * sin(8.0 * a + 3.1 * O._c);
  return O.radius * (1.0 + w);
}

@compute @workgroup_size(8, 8, 1)
fn melt_pane(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (O.kind & 2u) == 0u {
    return;
  }
  // over the field's texels near the disc
  let lo = vec2<i32>(floor((min(O.at, O.to) - 1.4 * O.radius - 2.0) * G.fs));
  let f = lo + vec2<i32>(gid.xy);
  if f.x < 0 || f.y < 0 || f.x >= i32(G.field_w) || f.y >= i32(G.field_h) {
    return;
  }
  let p = (vec2<f32>(f) + 0.5) / G.fs;
  let d = distance(p, melt_near(p));
  let rr = melt_radius(p);
  if d <= rr {
    // a film of meltwater (pane.wgsl draws field values between 0.3 and 0.95 as wet glass), thinner
    // toward the middle where the finger pressed longest
    // (a finger's way melts frame after frame: where it is wet already, the thinner film stays)
    let i = f.y * i32(G.field_w) + f.x;
    let film = 0.55 + 0.35 * smoothstep(0.5 * rr, rr, d);
    let was = field[i];
    field[i] = select(film, min(was, film), was > 0.3 && was < 0.98);
    let m = mask_index(p);
    if m >= 0 {
      atomicStore(&mask[m], EMPTY);
      // the meltwater freezes smooth: how far inside the melt's edge (the deepest any melt went)
      atomicMax(&glaze[m], bitcast<u32>(max(rr - d, 0.0)));
    }
  }
}

// a fingertip on clear glass: its moist print, which frost can catch on again (the glaze given back
// along the way, a little wider than the print) and a faint wet trace where the glass was dry
@compute @workgroup_size(8, 8, 1)
fn pen(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (O.kind & 4u) == 0u {
    return;
  }
  let lo = vec2<i32>(floor((min(O.at, O.to) - 3.2 * O.radius - 2.0 * G.fringe - 2.0) * G.fs));
  let f = lo + vec2<i32>(gid.xy);
  if f.x < 0 || f.y < 0 || f.x >= i32(G.field_w) || f.y >= i32(G.field_h) {
    return;
  }
  let p = (vec2<f32>(f) + 0.5) / G.fs;
  let d = distance(p, melt_near(p));
  let rr = melt_radius(p);
  let m = mask_index(p);
  if m < 0 || d > 3.0 * rr + 2.0 * G.fringe {
    return;
  }
  // (frost may grow feathers out from it a little further than a melt's fringe)
  atomicMin(&glaze[m], bitcast<u32>(max(d - 3.0 * rr, 0.0)));
  let i = f.y * i32(G.field_w) + f.x;
  // the print's middle freezes at once, granular, ragged at its edge
  let core = rr * (0.2 + 0.5 * flaws(p / 1.6));
  if d <= core {
    field[i] = max(field[i], 1.0 + 0.5 * hash(floor(p * 2.7)) * (1.0 - d / rr));
    atomicMin(&mask[m], HOAR);
  } else if d <= rr {
    let was = field[i];
    if was < 0.3 {
      field[i] = 0.62;
    } else if was < 0.98 {
      field[i] = min(was, 0.62);
    }
  }
}

// how much of the glass round a point is ice (stats[48], and how many samples were on the glass: [49])
@compute @workgroup_size(8, 8, 1)
fn probe(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (O.kind & 8u) == 0u {
    return;
  }
  if gid.x == 0u && gid.y == 0u {
    let c = mask_index(O._d);
    if c >= 0 {
      atomicStore(&stats[50], atomicLoad(&glaze[c]));
    }
  }
  let o = (vec2<f32>(gid.xy) - 3.5) / 3.5;
  if dot(o, o) > 1.0 {
    return;
  }
  let p = O._d + o * 16.0;
  let f = vec2<i32>(floor(p * G.fs));
  if mask_index(p) < 0 || f.x < 0 || f.y < 0 || f.x >= i32(G.field_w) || f.y >= i32(G.field_h) {
    return;
  }
  atomicAdd(&stats[49], 1u);
  if field[f.y * i32(G.field_w) + f.x] >= 1.0 {
    atomicAdd(&stats[48], 1u);
  }
}

// the stove's warmth: everything within radius of at runs wet and clears, the glaze with it (the
// glass new again, for the frost to come back to); a wet front at the edge
@compute @workgroup_size(8, 8, 1)
fn clear(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (O.kind & 16u) == 0u || gid.x >= G.field_w || gid.y >= G.field_h {
    return;
  }
  let p = (vec2<f32>(gid.xy) + 0.5) / G.fs;
  let wob = 1.0 + 0.08 * sin(atan2(p.y - O.at.y, p.x - O.at.x) * 5.0 + 0.03 * p.x);
  let d = distance(p, O.at) * wob;
  if d > O.radius {
    return;
  }
  let i = gid.y * G.field_w + gid.x;
  let wet = 1.0 - smoothstep(O.radius - 60.0, O.radius - 6.0, d);
  field[i] = select(0.0, mix(0.9, 0.55, wet), d > O.radius - 60.0);
  let m = mask_index(p);
  if m >= 0 {
    atomicStore(&mask[m], EMPTY);
    atomicStore(&glaze[m], 0u);
  }
}

// a slot's s at a point of the pane, read between its cells
fn sample(k: u32, p: vec2<f32>) -> f32 {
  let sl = G.slots[k];
  let d = (p - sl.center) / G.cell;
  let x = d.x * sl.rot.x + d.y * sl.rot.y;
  let y = -d.x * sl.rot.y + d.y * sl.rot.x;
  let r = y / 0.8660254;
  let q = x - 0.5 * r;
  let q0 = floor(q);
  let r0 = floor(r);
  let fq = q - q0;
  let fr = r - r0;
  let qi = i32(q0);
  let ri = i32(r0);
  if abs(qi) >= HALF - 1 || abs(ri) >= HALF - 1 {
    return 0.0;
  }
  let a = src[at(k, qi, ri)];
  let b = src[at(k, qi + 1, ri)];
  let c = src[at(k, qi, ri + 1)];
  let e = src[at(k, qi + 1, ri + 1)];
  return mix(mix(a, b, fq), mix(c, e, fq), fr);
}

// the pane's ice, once a frame: where a live crystal holds the glass, its water read there; where a
// finished one does, what it left
@compute @workgroup_size(8, 8, 1)
fn paint(@builtin(global_invocation_id) gid: vec3<u32>) {
  if gid.x >= G.field_w || gid.y >= G.field_h {
    return;
  }
  let p = (vec2<f32>(gid.xy) + 0.5) / G.fs;
  let m = mask_index(p);
  if m < 0 {
    return;
  }
  let owner = atomicLoad(&mask[m]);
  if owner == EMPTY {
    return;
  }
  for (var k = 0u; k < 12u; k++) {
    if G.slots[k].live != 0u && G.slots[k].id == owner {
      field[gid.y * G.field_w + gid.x] = sample(k, p);
      return;
    }
  }
}

// the holes heal: every glaze a little shallower each frame, so the frost's edge may creep further in
@compute @workgroup_size(8, 8, 1)
fn relax(@builtin(global_invocation_id) gid: vec3<u32>) {
  if gid.x >= G.mask_w || gid.y >= G.mask_h {
    return;
  }
  let i = gid.y * G.mask_w + gid.x;
  let v = bitcast<f32>(atomicLoad(&glaze[i]));
  if v > 0.0 {
    atomicStore(&glaze[i], bitcast<u32>(max(v - G.relax, 0.0)));
  }
}
