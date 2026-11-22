// Frost: the window, every pixel. Outside, Oymyakon's winter night (a clear sky, the snow, the larches
// on the far bank, all drawn); on the glass the ice the automaton grew (frost.wgsl's field), drawn in
// line as Peter Foldès drew on the NRC's key-frame system (keys.rs, SOURCES.md): the frost's edge and
// the contours of its age, each frame the drawing between the last two key drawings.

struct Look {
  size: vec2<f32>,   // pixels
  ppp: f32,
  time: f32,
  pane: vec2<f32>,   // points
  fs: f32,           // field texels a point
  field_w: f32,
  field_h: f32,
  key: f32,     // how far between the two key drawings
  frame: f32,   // points
  bar: f32,     // the crossbar's middle, points from the top
};

@group(0) @binding(0) var<uniform> L: Look;
@group(0) @binding(1) var<storage, read> key_a: array<f32>;
@group(0) @binding(2) var<storage, read> key_b: array<f32>;
// the cabin's lamp, always lit
const LAMP = 1.0;
// pixels a point
var<private> LP: f32 = 3.0;

struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO;
  o.pos = vec4<f32>(p, 0.0, 1.0);
  o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
  return o;
}

fn hash2(p: vec2<f32>) -> f32 {
  return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
fn noise(p: vec2<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), w.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), w.x), w.y);
}

fn texel(x: i32, y: i32) -> f32 {
  let cx = clamp(x, 0, i32(L.field_w) - 1);
  let cy = clamp(y, 0, i32(L.field_h) - 1);
  // the in-between: the computer's drawing between the two key drawings
  let i = cy * i32(L.field_w) + cx;
  return mix(key_a[i], key_b[i], L.key);
}

// the field between its texels
fn ice_at(p: vec2<f32>) -> f32 {
  let f = p * L.fs - 0.5;
  let i = vec2<i32>(floor(f));
  let t = f - floor(f);
  let a = texel(i.x, i.y);
  let b = texel(i.x + 1, i.y);
  let c = texel(i.x, i.y + 1);
  let d = texel(i.x + 1, i.y + 1);
  return mix(mix(a, b, t.x), mix(c, d, t.x), t.y);
}

// the night outside, at a point of the pane (points)
fn night(p: vec2<f32>) -> vec3<f32> {
  let h = L.pane.y;
  let y = p.y / h;
  let horizon = 0.74 + 0.012 * sin(p.x * 0.011) + 0.008 * sin(p.x * 0.037 + 1.0);
  // the sky: deep blue, a little lighter toward the snow
  var c = mix(vec3<f32>(0.012, 0.022, 0.060), vec3<f32>(0.050, 0.080, 0.150), smoothstep(0.0, horizon, y));
  // the moon's glow, low over the far bank
  c += vec3<f32>(0.05, 0.06, 0.08) * exp(-pow(length(vec2<f32>((p.x / L.pane.x - 0.72) * 1.4, y - horizon + 0.05)) * 3.0, 2.0));
  // stars: a few hundred, each its own brightness, breathing slowly in the cold air
  let g = p / 9.0;
  let cell = floor(g);
  let r = hash2(cell);
  if r > 0.86 && y < horizon - 0.02 {
    let o = vec2<f32>(hash2(cell + 3.1), hash2(cell + 7.7));
    let d = length((g - cell - o) * 9.0);
    let tw = 0.65 + 0.35 * sin(L.time * (0.6 + 1.3 * hash2(cell + 1.3)) + r * 40.0);
    let m = pow((r - 0.86) / 0.14, 3.0);
    c += vec3<f32>(0.75, 0.82, 1.0) * m * tw * exp(-d * d * 1.6) * 0.9;
  }
  // the snow on the river's flat, faintly lit by the stars, darker toward the glass
  if y > horizon {
    let k = (y - horizon) / (1.0 - horizon);
    c = mix(vec3<f32>(0.120, 0.140, 0.190), vec3<f32>(0.060, 0.070, 0.100), k);
    c *= 0.9 + 0.1 * noise(vec2<f32>(p.x * 0.02, k * 6.0));
  }
  // larches on the far bank: thin dark spires along the horizon
  let col = floor(p.x / 7.0);
  let rh = hash2(vec2<f32>(col, 2.0));
  let tree_h = (0.012 + 0.05 * rh * rh) * h;
  let cx = (col + 0.5 + 0.3 * (hash2(vec2<f32>(col, 5.0)) - 0.5)) * 7.0;
  let top = horizon * h - tree_h;
  if rh > 0.25 && p.y > top && p.y < horizon * h + 2.0 {
    let w = (p.y - top) / tree_h * 2.6 * (0.6 + 0.4 * noise(vec2<f32>(p.x * 0.5, p.y * 0.9)));
    if abs(p.x - cx) < w {
      c = mix(c, vec3<f32>(0.006, 0.009, 0.016), 0.92);
    }
  }
  return c;
}

// the frame and crossbar: old larch wood, painted once, lit by the lamp; how far into the wood a
// point is (negative on the glass)
fn wood_depth(p: vec2<f32>) -> f32 {
  let f = L.frame;
  let edge = min(min(p.x, p.y), min(L.pane.x - p.x, L.pane.y - p.y));
  var d = f - edge;
  if L.bar > 0.0 {
    d = max(d, 0.5 * f - abs(p.y - L.bar));
  }
  return d;
}

fn wood(p: vec2<f32>, d: f32, lamp: f32) -> vec3<f32> {
  let grain = noise(vec2<f32>(p.x * 0.05, p.y * 0.6)) * 0.5 + noise(vec2<f32>(p.x * 0.3, p.y * 2.0)) * 0.25;
  var c = vec3<f32>(0.20, 0.12, 0.07) * (0.75 + 0.5 * grain);
  // a bevel toward the glass: lighter where it faces the room's lamp
  c *= 0.75 + 0.35 * smoothstep(0.0, 4.0, d);
  c *= 0.55 + 0.9 * lamp;
  return c;
}

@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  LP = L.ppp;
  let p = i.uv * L.pane;
  var c = night(p);
  // the lamp behind the reader, a faint warm reflection low on the glass
  let lamp_at = vec2<f32>(L.pane.x * 0.18, L.pane.y * 1.05);
  let ld = length((p - lamp_at) / L.pane.x);
  c += vec3<f32>(0.060, 0.034, 0.014) * exp(-ld * ld * 2.2) * LAMP;

  // the ice
  let s = ice_at(p);
  // meltwater where a fingertip was: the night clear through it under the faintest film (its edge is
  // the ice's own line, below)
  let wet = smoothstep(0.3, 0.5, s) * (1.0 - smoothstep(0.95, 0.985, s));
  c += vec3<f32>(0.006, 0.007, 0.009) * wet;
  // the ice in line: its edge, and the contours of its age (the spines and ribs froze first, so
  // their contours run along them), one pen weight, chalk-white on the night
  let e = 0.7;
  let gx = ice_at(p + vec2<f32>(e, 0.0)) - ice_at(p - vec2<f32>(e, 0.0));
  let gy = ice_at(p + vec2<f32>(0.0, e)) - ice_at(p - vec2<f32>(0.0, e));
  let grad = max(length(vec2<f32>(gx, gy)) / (2.0 * e), 1e-4);
  let r2 = 2.4;
  let around = 0.25 * (ice_at(p + vec2<f32>(r2, 0.0)) + ice_at(p - vec2<f32>(r2, 0.0)) + ice_at(p + vec2<f32>(0.0, r2)) + ice_at(p - vec2<f32>(0.0, r2)));
  let sm = 0.35 * s + 0.65 * around;
  let px = 1.0 / LP;
  // the pen: about two thirds of a point wide, its edge a pixel soft; where edges crowd closer than
  // the pen (the dense fern near the frame) the line thins rather than filling
  let w = 0.33;
  let edge = (1.0 - smoothstep(w - px, w + px, abs(s - 1.0) / grad)) * (1.0 - 0.6 * smoothstep(1.5, 4.0, grad));
  // the contours of its age, sparse: two levels, and none where they would crowd into a fill (a pen
  // can't draw lines closer than its own width)
  var ages = 0.0;
  let sg = max(length(vec2<f32>(
    ice_at(p + vec2<f32>(r2, 0.0)) - ice_at(p - vec2<f32>(r2, 0.0)),
    ice_at(p + vec2<f32>(0.0, r2)) - ice_at(p - vec2<f32>(0.0, r2)))) * 0.65 / (2.0 * r2), 1e-4);
  for (var k = 0; k < 2; k++) {
    let lv = 1.0 + 0.25 * exp2(f32(k) * 1.6);
    ages = max(ages, 1.0 - smoothstep(w * 0.8 - px, w * 0.8 + px, abs(sm - lv) / sg));
  }
  ages *= 1.0 - smoothstep(0.25, 0.5, sg);
  // the night through the glass, frosted or not: only the line is drawn
  let ink = vec3<f32>(0.90, 0.93, 0.97);
  let warm = exp(-ld * ld * 1.6) * LAMP;
  let chalk = ink * mix(vec3<f32>(1.0), vec3<f32>(1.04, 0.97, 0.90), warm);
  c = mix(c, chalk, max(edge, ages * 0.8 * step(1.0, s)));
  // a breath of condensation low on the glass, before the frost: the faintest grey veil
  c += vec3<f32>(0.018, 0.020, 0.024) * smoothstep(0.5, 1.0, p.y / L.pane.y);
  // the wood over all of it
  let d = wood_depth(p);
  if d > -1.5 {
    let w = wood(p, d, exp(-ld * ld * 1.2) * LAMP);
    // a soft shadow on the glass beside the wood
    if d < 0.0 {
      c *= 0.6 + 0.4 * smoothstep(-1.5, 0.0, -d - 0.0) ;
    } else {
      c = mix(c, w, smoothstep(0.0, 0.8, d));
    }
  }
  return vec4<f32>(c, 1.0);
}
