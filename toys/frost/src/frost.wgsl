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
  slots: array<Slot, 8>,
};

struct Op {
  slot: u32,
  kind: u32,          // 1 seed the slot, 2 melt a disc
  _a: u32,
  _b: u32,
  at: vec2<f32>,      // points
  radius: f32,
  _c: f32,
};

@group(0) @binding(0) var<uniform> G: Globals;
@group(0) @binding(1) var<storage, read> src: array<f32>;
@group(0) @binding(2) var<storage, read_write> dst: array<f32>;
@group(0) @binding(3) var<storage, read_write> mask: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> stats: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read_write> field: array<f32>;
@group(0) @binding(6) var<uniform> O: Op;

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
  return G.bar <= 0.0 || abs(p.y - G.bar) > 0.5 * f;
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
    if owner != EMPTY && owner != G.slots[k].id {
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
  // (the glass two cells round its ice: another crystal's arms keep that far off, a clear seam)
  let m0 = vec2<i32>(floor(p / G.cell)) - vec2<i32>(2, 2);
  for (var dy = 0; dy < 5; dy++) {
    for (var dx = 0; dx < 5; dx++) {
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
  if distance(p, O.at) <= melt_radius(p) {
    dst[at(k, q, r)] = beta_at(p);
  }
}

// a fingertip's print is not a circle: its edge wanders with the angle
fn melt_radius(p: vec2<f32>) -> f32 {
  let a = atan2(p.y - O.at.y, p.x - O.at.x);
  let w = 0.16 * sin(3.0 * a + O._c) + 0.10 * sin(5.0 * a + 2.0 * O._c) + 0.07 * sin(8.0 * a + 3.1 * O._c);
  return O.radius * (1.0 + w);
}

@compute @workgroup_size(8, 8, 1)
fn melt_pane(@builtin(global_invocation_id) gid: vec3<u32>) {
  if (O.kind & 2u) == 0u {
    return;
  }
  // over the field's texels near the disc
  let lo = vec2<i32>(floor((O.at - 1.4 * O.radius - 2.0) * G.fs));
  let f = lo + vec2<i32>(gid.xy);
  if f.x < 0 || f.y < 0 || f.x >= i32(G.field_w) || f.y >= i32(G.field_h) {
    return;
  }
  let p = (vec2<f32>(f) + 0.5) / G.fs;
  let d = distance(p, O.at);
  let rr = melt_radius(p);
  if d <= rr {
    // a film of meltwater (pane.wgsl draws field values between 0.3 and 0.95 as wet glass), thinner
    // toward the middle where the finger pressed longest
    field[f.y * i32(G.field_w) + f.x] = 0.55 + 0.35 * smoothstep(0.5 * rr, rr, d);
    let m = mask_index(p);
    if m >= 0 {
      atomicStore(&mask[m], EMPTY);
    }
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
  for (var k = 0u; k < 8u; k++) {
    if G.slots[k].live != 0u && G.slots[k].id == owner {
      field[gid.y * G.field_w + gid.x] = sample(k, p);
      return;
    }
  }
}
