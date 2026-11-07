// The hillside seen from above in the afternoon sun: the slope's soil and dry grass, a flat rock for
// each territory, the boulders, and on each rock its lizard (a side-blotched lizard: a sandy, blotched
// back, and the throat, orange, blue or yellow, shown at the chin and the flanks as a male shows it
// off). Far out a lizard is a speck of its throat's colour.
//
// cells: one texel a rock (x: shown + 4*was + 16*look + 1024*ground (1 outcrop, 2 shrub), y: heading, z: seconds since it changed
// hands, w: where the newcomer ran in from, (fx+4) + 9*(fy+4)).

struct U {
  size: vec2<f32>,      // target, pixels
  center: vec2<f32>,    // world point at the centre, cells
  scale: f32,           // pixels a cell
  time: f32,
  ppp: f32,             // pixels a point
  night: f32,
  grid: vec2<f32>,      // the hill's size, cells
  _p: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var cells: texture_2d<f32>;
@group(0) @binding(2) var relief: texture_2d<f32>;     // r: outcrops' height, g: shrubs' (blurred masks)
@group(0) @binding(3) var lin: sampler;
fn rel(w: vec2<f32>) -> vec4<f32> { return textureSampleLevel(relief, lin, w / u.grid, 0.0); }

struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}

fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn hash22(p: vec2<f32>) -> vec2<f32> { return vec2(hash2(p), hash2(p + vec2(17.3, 41.9))); }
fn noise2(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p); let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), w.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), w.x), w.y);
}
fn fbm(p: vec2<f32>) -> f32 {
  var s = 0.0; var a = 0.5; var q = p;
  for (var k = 0; k < 4; k++) { s += a * noise2(q); q = q * 2.03 + vec2(1.7, 9.2); a *= 0.5; }
  return s;
}

fn wrapc(c: vec2<i32>) -> vec2<i32> {
  let g = vec2<i32>(u.grid);
  return ((c % g) + g) % g;
}

fn cell(c: vec2<i32>) -> vec4<f32> { return textureLoad(cells, wrapc(c), 0); }

fn throat(k: i32) -> vec3<f32> {
  if (k == 1) { return vec3(0.98, 0.46, 0.10); }
  if (k == 2) { return vec3(0.16, 0.42, 0.98); }
  return vec3(1.0, 0.90, 0.22);
}

// a rock's tint: its holder's colour (alpha 1), none on a boulder
fn tint(x: f32) -> vec4<f32> {
  let k = i32(x) % 4;
  if (k == 0) { return vec4(0.0); }
  return vec4(throat(k), 1.0);
}
// one lizard out on its rock for every BLOCK x BLOCK territories; a lizard's length (territories)
const BLOCK: f32 = 1.0;
const LEN: f32 = 0.7;

struct Field { tint: vec4<f32>, out_m: f32, shrub_m: f32, out_g: vec2<f32>, shrub_g: vec2<f32> };
fn field(w: vec2<f32>) -> Field {
  let wb = w - 0.5;
  let b0 = vec2<i32>(floor(wb));
  let f = fract(wb);
  let c00 = cell(b0).x; let c10 = cell(b0 + vec2(1, 0)).x;
  let c01 = cell(b0 + vec2(0, 1)).x; let c11 = cell(b0 + vec2(1, 1)).x;
  var o: Field;
  o.tint = mix(mix(tint(c00), tint(c10), f.x), mix(tint(c01), tint(c11), f.x), f.y);
  let o00 = f32(i32(c00) / 1024 == 1); let o10 = f32(i32(c10) / 1024 == 1);
  let o01 = f32(i32(c01) / 1024 == 1); let o11 = f32(i32(c11) / 1024 == 1);
  o.out_m = mix(mix(o00, o10, f.x), mix(o01, o11, f.x), f.y);
  o.out_g = vec2(mix(o10 - o00, o11 - o01, f.y), mix(o01 - o00, o11 - o10, f.x));
  let s00 = f32(i32(c00) / 1024 == 2); let s10 = f32(i32(c10) / 1024 == 2);
  let s01 = f32(i32(c01) / 1024 == 2); let s11 = f32(i32(c11) / 1024 == 2);
  o.shrub_m = mix(mix(s00, s10, f.x), mix(s01, s11, f.x), f.y);
  o.shrub_g = vec2(mix(s10 - s00, s11 - s01, f.y), mix(s01 - s00, s11 - s10, f.x));
  return o;
}

const SUN: vec2<f32> = vec2<f32>(-0.55, -0.83);   // toward the sun, on the screen (upper left)

// The ground under a world point (cells): soil and grass between the rocks, each rock's slab.
fn ground(w: vec2<f32>, pxc: f32) -> vec3<f32> {
  let soil = mix(vec3(0.60, 0.51, 0.38), vec3(0.72, 0.62, 0.46), fbm(w * 0.35));
  var col = soil * (0.9 + 0.2 * noise2(w * 3.1));
  // dry grass: short strokes of pale straw, combed downhill
  let g = noise2(vec2(w.x * 5.0, w.y * 1.3)) * noise2(w * 2.2 + 4.0);
  col = mix(col, vec3(0.66, 0.60, 0.46), smoothstep(0.35, 0.6, g) * 0.4);
  // loose stones and slabs scattered over the slope (not one a territory: a cellular noise of its own)
  let sc = 1.9;
  let sp = w * sc;
  let si = floor(sp);
  var best = 9.0; var lit = 0.0; var tone = 0.0;
  for (var dy = -1; dy <= 1; dy++) {
    for (var dx = -1; dx <= 1; dx++) {
      let c = si + vec2(f32(dx), f32(dy));
      let h = hash22(c * 1.37 + 5.0);
      if (h.x > 0.45) { continue; }
      let o = c + 0.5 + (h - 0.5) * 0.7;
      let r = 0.18 + 0.32 * h.y * h.y;
      let q = sp - o;
      let wob = 0.05 * sin(atan2(q.y, q.x) * (3.0 + floor(h.y * 3.0)) + h.x * 9.0);
      let d = (length(q * vec2(1.0, 1.0 + 0.4 * (h.x - 0.3))) - r - wob) / sc;
      if (d < best) { best = d; lit = dot(normalize(q + 1e-4), SUN); tone = h.y; }
    }
  }
  let aa = 1.0 / pxc;
  let sh = smoothstep(0.25, -0.02, best - 0.12);
  col *= 1.0 - 0.25 * sh;
  let slab = mix(vec3(0.58, 0.54, 0.48), vec3(0.70, 0.66, 0.58), tone) * (0.93 + 0.1 * noise2(w * 6.0 + tone * 9.0));
  let rim = clamp(-best / 0.25, 0.0, 1.0);
  let shade = mix(0.80 + 0.2 * lit, 1.0, rim * 0.7);
  col = mix(col, slab * shade, smoothstep(aa, -aa, best));
  // lichen
  let li = smoothstep(0.7, 0.8, noise2(w * 4.0 + 31.0)) * smoothstep(0.0, -0.05, best);
  col = mix(col, vec3(0.86, 0.70, 0.30), li * 0.5);
  // the open grass between the rocky patches: summer-gold, combed by the wind, in tufts
  let gw = w + (vec2(noise2(w * 0.35), noise2(w * 0.35 + 9.1)) - 0.5) * 2.4;
  let gm = rel(gw).b + 0.3 * (noise2(w * 0.9) - 0.5) + 0.15 * (noise2(w * 3.0) - 0.5);
  let blade = noise2(vec2(w.x * 9.0 + w.y * 2.0, w.y * 1.4)) * 0.6 + noise2(w * 4.0 + 7.0) * 0.4;
  let straw = mix(vec3(0.62, 0.50, 0.30), vec3(0.90, 0.78, 0.50), blade) * (0.9 + 0.15 * fbm(w * 0.5 + 3.0));
  let tuft = smoothstep(0.55, 0.75, noise2(w * 2.6 + 19.0));
  col = mix(col, straw * (1.0 - 0.25 * tuft) + vec3(0.06, 0.05, 0.02) * tuft, smoothstep(0.42, 0.42 + 1.5 / pxc + 0.05, gm));
  return col;
}

// a capsule's distance
fn seg(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, r0: f32, r1: f32) -> f32 {
  let pa = p - a; let ba = b - a;
  let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba * h) - mix(r0, r1, h);
}

struct Liz { d: f32, col: vec3<f32>, shadow: f32 };

// A lizard at `o` facing `a` (radians), its length `len` (cells), in a pixel `q` (cells), throat `k`,
// its look `look` (0–63), `bob` its display (0–1).
fn lizard(p: vec2<f32>, o: vec2<f32>, a: f32, len: f32, k: i32, look: f32, bob: f32, sway: f32) -> Liz {
  let ca = cos(a); let sa = sin(a);
  let r = p - o;
  // local: x forward, y to its left, in lengths
  var q = vec2(dot(r, vec2(ca, sa)), dot(r, vec2(-sa, ca))) / len;
  // the body bends a little as it runs
  q.y += sway * 0.10 * sin(q.x * 6.0 + look);
  let head = length((q - vec2(0.33, 0.0)) / vec2(0.13, 0.095)) - 1.0;
  let dh = head * 0.095;
  let body = seg(q, vec2(-0.12, 0.0), vec2(0.22, 0.0), 0.11, 0.095);
  let tail = seg(q, vec2(-0.14, 0.0), vec2(-0.62, 0.06 * sway + 0.04 * sin(look)), 0.075, 0.01);
  let fl = seg(q, vec2(0.15, 0.0), vec2(0.24, 0.20), 0.032, 0.022);
  let fr = seg(q, vec2(0.15, 0.0), vec2(0.08, -0.20), 0.032, 0.022);
  let hl = seg(q, vec2(-0.08, 0.0), vec2(-0.20, 0.20), 0.036, 0.022);
  let hr = seg(q, vec2(-0.08, 0.0), vec2(-0.01, -0.20), 0.036, 0.022);
  let legs = min(min(fl, fr), min(hl, hr));
  let d = min(min(min(dh, body), tail), legs);
  let t = throat(k);
  // the back: sandy brown with two rows of dark blotches, its sides washed with his colour
  let back = mix(vec3(0.40, 0.33, 0.25), vec3(0.52, 0.43, 0.32), fract(look * 0.61));
  let side = smoothstep(0.02, 0.09, abs(q.y));
  var col = mix(back, t, 0.45 + 0.5 * side);
  let bl = step(0.5, fract(q.x * 8.0 + look * 0.13)) * smoothstep(0.05, 0.015, abs(abs(q.y) - 0.04)) * step(-0.45, q.x) * step(q.x, 0.2);
  col = mix(col, vec3(0.20, 0.15, 0.10), bl * 0.7);
  // the throat: chin and the shoulders glow, brighter in a push-up
  let chin = smoothstep(0.02, -0.04, head) * step(0.25, q.x);
  let shoulder = smoothstep(0.12, 0.0, length(q - vec2(0.18, 0.0)) - 0.06);
  col = mix(col, t * (1.05 + 0.25 * bob), clamp(max(chin * 0.9, shoulder * 0.8) + 0.2 * bob, 0.0, 1.0));
  // the eyes
  let eye = length(vec2(q.x - 0.37, abs(q.y) - 0.06)) - 0.018;
  col = mix(col, vec3(0.05, 0.04, 0.03), smoothstep(0.006, -0.006, eye));
  // the light on its round back
  col *= 0.85 + 0.3 * clamp(1.0 - abs(q.y) / 0.12, 0.0, 1.0);
  var out: Liz;
  out.d = d * len;
  out.col = col;
  // shadow: the body pushed away from the sun
  let sv = -SUN * 0.10;
  let qs = q + vec2(dot(sv, vec2(ca, sa)), dot(sv, vec2(-sa, ca)));
  let ds = min(min(length((qs - vec2(0.33, 0.0)) / vec2(0.13, 0.095)) * 0.095 - 0.095, seg(qs, vec2(-0.12, 0.0), vec2(0.22, 0.0), 0.11, 0.095)), seg(qs, vec2(-0.14, 0.0), vec2(-0.62, 0.04), 0.075, 0.01));
  out.shadow = ds * len;
  return out;
}

fn ease(x: f32) -> f32 { let c = clamp(x, 0.0, 1.0); return c * c * (3.0 - 2.0 * c); }

@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let px = i.uv * u.size;
  let w = u.center + (px - u.size * 0.5) / u.scale;   // cells
  let pxc = u.scale;
  var col = ground(w, pxc);
  let ci = vec2<i32>(floor(w));
  let f0 = field(w);
  // the outcrops and shrubs: their shadows, down and to the right, then themselves
  let rs = rel(w + SUN * 0.9);
  col *= 1.0 - 0.38 * smoothstep(0.3, 0.5, max(rs.r, rs.g * 1.1) + 0.1 * (noise2(w * 2.0) - 0.5));
  let r0 = rel(w);
  let e = 0.35;
  let gx = rel(w + vec2(e, 0.0)).r - rel(w - vec2(e, 0.0)).r;
  let gy = rel(w + vec2(0.0, e)).r - rel(w - vec2(0.0, e)).r;
  let om = r0.r + 0.12 * (fbm(w * 0.8) - 0.5);
  if (om > 0.3) {
    // granite: pale rounded domes lit from the upper left, grainy, a few joints, lichen
    let n = normalize(vec3(-gx * 2.2, -gy * 2.2, 1.0));
    let lit = clamp(dot(n, normalize(vec3(SUN, 0.9))), 0.0, 1.0);
    var g = mix(vec3(0.50, 0.48, 0.45), vec3(0.88, 0.86, 0.80), lit) * (0.92 + 0.12 * noise2(w * 9.0)) * (0.94 + 0.08 * fbm(w * 2.0));
    let joint = smoothstep(0.035, 0.0, abs(noise2(w * 0.45 + 3.0) - 0.5)) * smoothstep(0.45, 0.6, om);
    g *= 1.0 - 0.3 * joint;
    g = mix(g, vec3(0.80, 0.66, 0.30), smoothstep(0.66, 0.74, noise2(w * 1.3 + 11.0)) * 0.4);
    col = mix(col, g, smoothstep(0.3, 0.3 + 1.5 / pxc + 0.02, om));
  }
  let sm = r0.g + 0.3 * (noise2(w * 2.2) - 0.5);
  if (sm > 0.25) {
    // shrubs: dusty sage-green clumps
    let leaf = noise2(w * 6.0) * 0.6 + noise2(w * 14.0) * 0.4;
    let sx = rel(w + vec2(e, 0.0)).g - rel(w - vec2(e, 0.0)).g;
    let sy = rel(w + vec2(0.0, e)).g - rel(w - vec2(0.0, e)).g;
    let lit = clamp(0.5 - (sx * SUN.x + sy * SUN.y) * 2.5, 0.0, 1.0);
    let g = mix(vec3(0.20, 0.24, 0.15), vec3(0.55, 0.60, 0.44), clamp(lit * 0.6 + leaf * 0.55, 0.0, 1.0));
    col = mix(col, g, smoothstep(0.25, 0.25 + 1.5 / pxc + 0.03, sm));
  }
  let aa = 1.2 / pxc;

  // the lizards: one out on its own rock for every BLOCK x BLOCK territories, wearing the colour
  // of the territory its rock is in
  let lpx = LEN * pxc;                       // a lizard's length, pixels
  let near = smoothstep(9.0, 15.0, lpx);
  let specks = 1.0 - smoothstep(12.0, 18.0, lpx);
  {
    var shadow = 0.0;
    var lcol = vec3(0.0); var la = 0.0;
    let bi = vec2<i32>(floor(w / BLOCK));
    for (var dy = -1; dy <= 1; dy++) {
      for (var dx = -1; dx <= 1; dx++) {
        let bc = bi + vec2(dx, dy);
        let bf = vec2<f32>(bc);
        let hb = hash22(bf * 0.731 + 3.1);
        let rock = (bf + 0.5 + (hb - 0.5) * 0.75) * BLOCK;
        let cc = vec2<i32>(floor(rock));
        let c = cell(cc);
        let packed = i32(c.x);
        if (packed / 1024 != 0) { continue; }
        // far out a lizard is a speck of its colour where it sits
        if (specks > 0.0) {
          let kk = packed % 4;
          let lk = f32((packed / 16) % 64);
          let sp = rock + (hash22(bf + lk * 0.013) - 0.5) * LEN * 0.25;
          let sd = length(w - sp) - 0.36;
          let a = smoothstep(0.8 / pxc, -0.8 / pxc, sd) * specks;
          col = mix(col, throat(kk), a * 0.95);
          if (near <= 0.0) { continue; }
        }
        // its rock: a flat sunny slab
        let ang = hb.x * 6.283;
        let rot = mat2x2<f32>(cos(ang), sin(ang), -sin(ang), cos(ang));
        let rq = rot * (w - rock) * vec2(1.0, 1.25 + 0.3 * hb.y);
        let rr = LEN * (0.40 + 0.1 * hb.y);
        let rd = length(rq) - rr - rr * 0.22 * (noise2(rq * 1.1 / LEN * 3.0 + hb * 17.0) - 0.5);
        let rsh = length(rot * (w - rock + SUN * 0.22 * LEN) * vec2(1.0, 1.25 + 0.3 * hb.y)) - rr;
        col *= 1.0 - 0.32 * smoothstep(0.4, -0.2, rsh);
        let edge = smoothstep(-0.45 * rr, 0.0, rd);
        let rlit = clamp(0.7 - dot(normalize(w - rock + 1e-4), SUN) * 0.35 * edge, 0.0, 1.0);
        var slab = mix(vec3(0.48, 0.45, 0.41), vec3(0.68, 0.64, 0.58), rlit) * (0.9 + 0.14 * noise2((w - rock) * 2.5 + hb * 9.0));
        slab = mix(slab, vec3(0.78, 0.64, 0.32), smoothstep(0.68, 0.76, noise2((w - rock) * 1.4 + hb * 5.0)) * 0.5);
        col = mix(col, slab, smoothstep(aa, -aa, rd));
        let k = packed % 4;
        let was = (packed / 4) % 4;
        let look = f32((packed / 16) % 64);
        let since = c.z;
        let fi = i32(c.w);
        let came = vec2(f32(fi % 9 - 4), f32(fi / 9 - 4));
        let home = rock + (hash22(bf + look * 0.013) - 0.5) * LEN * 0.25;
        let len = LEN * (0.9 + 0.2 * fract(look * 0.29));
        let run = ease(since / 0.6);
        let inv = normalize(came + vec2(1e-4, 0.0)) * BLOCK * 0.8;
        // the one leaving, running off away from the newcomer
        if (was != 0 && was != k && since < 0.8) {
          let away = -normalize(inv + vec2(1e-4, 0.0));
          let gp = home + away * ease(since / 0.8) * BLOCK * 0.9;
          let lz = lizard(w, gp, atan2(away.y, away.x), len, was, look + 7.0, 0.0, 1.0);
          let fade = 1.0 - ease(since / 0.8);
          shadow = max(shadow, smoothstep(aa, -aa, lz.shadow) * 0.35 * fade);
          let al = smoothstep(aa, -aa, lz.d) * fade;
          lcol = mix(lcol, lz.col, al); la = max(la, al);
        }
        if (k != 0) {
          let pos = home + inv * (1.0 - run);
          let idle = 0.3 * sin(u.time * (0.25 + 0.2 * fract(look * 0.7)) + look);
          let a = c.y + idle * run;
          // the push-up: now and then a lizard flashes its throat; a newcomer at once
          let bob = max(smoothstep(0.85, 1.0, sin(u.time * 0.9 + look * 1.7)), 1.0 - ease(since / 1.4));
          let lz = lizard(w, pos, a, len, k, look, bob, 1.0 - run);
          shadow = max(shadow, smoothstep(aa, -aa, lz.shadow) * 0.45);
          let al = smoothstep(aa, -aa, lz.d);
          lcol = mix(lcol, lz.col, al); la = max(la, al);
        }
      }
    }
    col *= 1.0 - shadow * near;
    col = mix(col, lcol, la * near);
  }
  // the hillside's own folds in the afternoon sun: warm where they face it, cool in their shade
  let hs = 0.055;
  let h0 = fbm(w * hs);
  let hx = fbm((w + vec2(1.5, 0.0)) * hs) - h0;
  let hy = fbm((w + vec2(0.0, 1.5)) * hs) - h0;
  let face = clamp(0.5 - (hx * SUN.x + hy * SUN.y) * 14.0, 0.0, 1.0);
  col *= mix(vec3(0.90, 0.92, 0.98), vec3(1.06, 1.02, 0.95), face);
  return vec4(col, 1.0);
}
