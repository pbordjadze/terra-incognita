// Battle Ink's drawing (painting-company row 206). Four passes:
//   cut   every unit one tapered burin cut, an arc along its army's current, into the ink sheet
//         (R Rome, G Carthage), its swell running the way the current goes;
//   seg   the finger's strokes (B), the real day's ghost (A) and the page's words (R) into the same sheet;
//   dot   the paper's memory: where the currents ran and where men fell, into a sheet of the world;
//   page  the paper, Cannae's relief and the Aufidus engraved, the stains, and the two inks laid on
//         it, each pixel one ink: where the armies meet the inks fold into marbled chevrons rather
//         than greying.

struct U {
  screen: vec4f,  // width px, height px, px a point, px a world unit
  off: vec4f,     // world origin on screen (px), seconds, night 0..1
  world: vec4f,   // the world sheets' rect: x0, y0, width, height (units)
  ground: vec4f,  // the ground's rect: x0, y0, width, height (units)
  misc: vec4f,    // reduce motion, the page's fade-in, the cuts' width scale, the page's drying (0..1)
};
@group(0) @binding(0) var<uniform> u: U;

fn to_clip(px: vec2f) -> vec4f {
  return vec4f(px.x / u.screen.x * 2.0 - 1.0, 1.0 - px.y / u.screen.y * 2.0, 0.0, 1.0);
}
fn w2px(w: vec2f) -> vec2f { return u.off.xy + w * u.screen.w; }

// ---------------------------------------------------------------- cut
struct CutOut {
  @builtin(position) pos: vec4f,
  @location(0) across: f32,    // px from the cut's spine, signed
  @location(1) hw: f32,        // its half width here, px
  @location(2) @interpolate(flat) side: f32,
  @location(3) @interpolate(flat) alpha: f32,
};

const SEGS: u32 = 8u;

@vertex
fn vs_cut(@builtin(vertex_index) vi: u32, @location(0) a: vec4f, @location(1) b: vec4f, @location(2) c: vec4f) -> CutOut {
  let k = vi / 2u;
  let sgn = select(-1.0, 1.0, (vi & 1u) == 1u);
  let t = f32(k) / f32(SEGS);
  let len = b.x;
  let s = (t - 0.5) * len;
  let kap = b.z;
  let d = normalize(a.zw);
  let n = vec2f(-d.y, d.x);
  // along an arc of curvature kap through the unit, tangent d
  var along: f32;
  var side_off: f32;
  if (abs(kap) < 1e-4) { along = s; side_off = 0.0; } else {
    along = sin(kap * s) / kap;
    side_off = (1.0 - cos(kap * s)) / kap;
  }
  let pw = a.xy + d * along + n * side_off;
  let ang = kap * s;
  let tan = d * cos(ang) + n * sin(ang);
  let nn = vec2f(-tan.y, tan.x);
  // tapered at both ends, swelling where the current's pulse is
  let swell = 0.55 + 0.45 * cos(6.2831853 * (t - b.w));
  // a cut's width grows a little with the view's scale, so a closer view is not hairlines
  let w_pt = b.y * u.misc.z * pow(max(sin(3.14159265 * t), 0.0), 0.7) * swell;
  let hw = max(w_pt * u.screen.z * 0.5, 0.0);
  let ext = hw + 1.0;
  var o: CutOut;
  o.pos = to_clip(w2px(pw) + nn * sgn * ext);
  o.across = sgn * ext;
  o.hw = hw;
  o.side = c.x;
  o.alpha = c.y;
  return o;
}

@fragment
fn fs_cut(i: CutOut) -> @location(0) vec4f {
  // a line thinner than a pixel is drawn a pixel wide and fainter
  let hw = max(i.hw, 0.5);
  let cov = clamp(hw + 0.5 - abs(i.across), 0.0, 1.0) * min(1.0, i.hw * 2.0) * i.alpha;
  if (i.side < 0.5) { return vec4f(cov, 0.0, 0.0, 0.0); }
  return vec4f(0.0, cov, 0.0, 0.0);
}

// ---------------------------------------------------------------- seg
struct SegOut {
  @builtin(position) pos: vec4f,
  @location(0) px: vec2f,
  @location(1) @interpolate(flat) a: vec2f,
  @location(2) @interpolate(flat) b: vec2f,
  @location(3) @interpolate(flat) s: vec2f,     // along-length at a and b (units)
  @location(4) @interpolate(flat) wk: vec4f,    // width pt, kind, alpha, pulse flow
  @location(5) @interpolate(flat) fill: f32,    // the ghost's dashes closed (0..1)
};

@vertex
fn vs_seg(@builtin(vertex_index) vi: u32, @location(0) ab: vec4f, @location(1) sw: vec4f, @location(2) ka: vec4f) -> SegOut {
  let a = w2px(ab.xy);
  let b = w2px(ab.zw);
  var d = b - a;
  let l = length(d);
  if (l < 1e-3) { d = vec2f(1.0, 0.0); } else { d = d / l; }
  let n = vec2f(-d.y, d.x);
  let r = sw.z * u.screen.z * 0.5 * 2.2 + 2.0;   // room for the widest pulse
  let corners = array<vec2f, 4>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0), vec2f(1.0, 1.0));
  let q = corners[vi];
  let base = select(a, b, q.x > 0.0);
  let p = base + d * q.x * r + n * q.y * r;
  var o: SegOut;
  o.pos = to_clip(p);
  o.px = p;
  o.a = a;
  o.b = b;
  o.s = sw.xy;
  o.wk = vec4f(sw.z, ka.x, ka.y, sw.w);
  o.fill = ka.z;
  return o;
}

@fragment
fn fs_seg(i: SegOut) -> @location(0) vec4f {
  let ab = i.b - i.a;
  let l2 = max(dot(ab, ab), 1e-6);
  let h = clamp(dot(i.px - i.a, ab) / l2, 0.0, 1.0);
  let dist = length(i.px - i.a - ab * h);
  let s = mix(i.s.x, i.s.y, h);
  var w = i.wk.x;
  var alpha = i.wk.z;
  if (i.wk.y < 0.5) {
    // the finger's ink: swellings run down it the way it was drawn, like a pulse in a vein
    let flow = select(i.wk.w, 0.0, u.misc.x > 0.5);
    let ph = fract((s - u.off.z * flow) / 34.0);
    let pulse = pow(max(0.0, cos(ph * 6.2831853)), 6.0);
    let taper = min(1.0, 0.35 + s / 60.0);
    w = (w + 1.1 * pulse * select(1.0, 0.0, flow == 0.0)) * taper;
  } else {
    // the real day's ghost: a dashed graphite underdrawing
    // the real day: fine dots, an engraver's stipple (kind 3: an arrowhead, solid)
    if (i.wk.y < 2.5) {
      let ph = fract(s / 4.5);
      alpha = alpha * (1.0 - smoothstep(0.1, 0.16, abs(ph - 0.5)));
    }
  }
  let hw = max(w * u.screen.z * 0.5, 0.5);
  let cov = clamp(hw + 0.5 - dist, 0.0, 1.0) * min(1.0, w * u.screen.z) * alpha;
  if (i.wk.y < 0.5) { return vec4f(0.0, 0.0, cov, 0.0); }
  // (kind 4: the page's words, engraved in Rome's ink)
  if (i.wk.y > 3.5) { return vec4f(cov, 0.0, 0.0, 0.0); }
  return vec4f(0.0, 0.0, 0.0, cov);
}

// ---------------------------------------------------------------- dot
struct DotOut {
  @builtin(position) pos: vec4f,
  @location(0) q: vec2f,
  @location(1) @interpolate(flat) v: vec4f,
};

@vertex
fn vs_dot(@builtin(vertex_index) vi: u32, @location(0) pr: vec4f, @location(1) v: vec4f) -> DotOut {
  let corners = array<vec2f, 4>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0), vec2f(1.0, 1.0));
  let q = corners[vi];
  let w = pr.xy + q * pr.z;
  let uv = (w - u.world.xy) / u.world.zw;
  var o: DotOut;
  o.pos = vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
  o.q = q;
  o.v = v;
  return o;
}

@fragment
fn fs_dot(i: DotOut) -> @location(0) vec4f {
  let r2 = dot(i.q, i.q);
  return i.v * exp(-r2 * 3.0) * step(r2, 1.0);
}

// ---------------------------------------------------------------- page
@group(1) @binding(0) var ground_tex: texture_2d<f32>;
@group(1) @binding(1) var ink_tex: texture_2d<f32>;
@group(1) @binding(2) var stain_tex: texture_2d<f32>;
@group(1) @binding(3) var dens_tex: texture_2d<f32>;
@group(1) @binding(4) var lin: sampler;

@vertex
fn vs_page(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4f {
  let x = f32((vi << 1u) & 2u);
  let y = f32(vi & 2u);
  return vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

fn hash(p: vec2f) -> f32 {
  var q = fract(p * vec2f(123.34, 456.21));
  q = q + dot(q, q + 45.32);
  return fract(q.x * q.y);
}
fn vnoise(p: vec2f) -> f32 {
  let i = floor(p);
  var f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2f(1.0, 0.0)), f.x), mix(hash(i + vec2f(0.0, 1.0)), hash(i + vec2f(1.0, 1.0)), f.x), f.y);
}
fn gsample(w: vec2f) -> vec2f {
  return textureSampleLevel(ground_tex, lin, (w - u.ground.xy) / u.ground.zw, 0.0).xy;
}
fn wsample_d(w: vec2f) -> vec4f {
  return textureSampleLevel(dens_tex, lin, (w - u.world.xy) / u.world.zw, 0.0);
}
// a line of constant width (pt) where f crosses a multiple of step
fn iso(f: f32, fw: f32, width_pt: f32) -> f32 {
  let d = abs(fract(f + 0.5) - 0.5) / max(fw, 1e-5);
  let hw = max(width_pt * u.screen.z * 0.5, 0.5);
  return clamp(hw + 0.5 - d, 0.0, 1.0) * min(1.0, width_pt * u.screen.z);
}

@fragment
fn fs_page(@builtin(position) fc: vec4f) -> @location(0) vec4f {
  let night = u.off.w;
  let px = fc.xy;
  let w = (px - u.off.xy) / u.screen.w;
  let g = gsample(w);
  let elev = g.x;
  let rd = g.y;

  // the paper: laid, a faint unevenness and fibres
  let day_paper = vec3f(0.937, 0.906, 0.839);
  let night_paper = vec3f(0.105, 0.088, 0.070);
  var paper = mix(day_paper, night_paper, night);
  let blot = vnoise(px / (u.screen.z * 90.0)) * 0.6 + vnoise(px / (u.screen.z * 23.0)) * 0.4;
  let fib = vnoise(vec2f(px.x / (u.screen.z * 1.2), px.y / (u.screen.z * 9.0)) + vnoise(px / (u.screen.z * 30.0)) * 4.0);
  paper = paper * (0.975 + 0.04 * blot) - vec3f(0.018) * smoothstep(0.62, 0.9, fib) * (1.0 - night * 0.6);
  var col = paper;

  // hill shade, very quiet: light from the screen's upper left
  let e = 2.0;
  let gx = gsample(w + vec2f(e, 0.0)).x - gsample(w - vec2f(e, 0.0)).x;
  let gy = gsample(w + vec2f(0.0, e)).x - gsample(w - vec2f(0.0, e)).x;
  let shade = clamp((gx + gy) * 0.5 * 2.2, -0.6, 1.0);
  let sepia = mix(vec3f(0.40, 0.31, 0.20), vec3f(0.58, 0.50, 0.40), night);
  col = mix(col, sepia, max(shade, 0.0) * 0.16 + max(-shade, 0.0) * 0.04 * (1.0 - night));

  // contours every metre, every fifth firmer: on the plain the ground's breath, on the ridge hatching
  let fe = elev;
  let fw = fwidth(fe);
  let minor = iso(fe, fw, 0.45) * (1.0 - smoothstep(0.35, 0.9, fw));
  let major = iso(fe / 5.0, fw / 5.0, 0.6);
  col = mix(col, sepia, max(minor * 0.11, major * 0.22) * mix(1.0, 0.9, night));

  // the Aufidus: a verdigris wash, its banks engraved, water lines echoing them outward
  let water = mix(vec3f(0.30, 0.45, 0.42), vec3f(0.36, 0.46, 0.44), night);
  let ard = abs(rd);
  let bank = 6.5;
  col = mix(col, mix(water, col, 0.55), (1.0 - smoothstep(bank - 0.5, bank + 0.5, ard)) * 0.5);
  let rfw = max(fwidth(rd), 1e-4);
  let bank_line = clamp(max(0.45 * u.screen.z, 0.5) + 0.5 - abs(ard - bank) / rfw, 0.0, 1.0);
  col = mix(col, water * 0.75, bank_line * 0.8);
  for (var k = 1; k < 6; k = k + 1) {
    let off = bank + f32(k) * (1.7 + f32(k) * 0.45);
    let ln = clamp(max(0.35 * u.screen.z, 0.5) + 0.5 - abs(ard - off) / rfw, 0.0, 1.0);
    col = mix(col, water * 0.8, ln * 0.55 / f32(k));
  }
  // and a few lines inside the stream, the current's
  let inner = clamp(max(0.3 * u.screen.z, 0.5) + 0.5 - abs(ard - 2.6) / rfw, 0.0, 1.0) * step(ard, bank);
  col = mix(col, water * 0.8, inner * 0.35);

  // the inks
  let rome = mix(vec3f(0.149, 0.180, 0.275), vec3f(0.55, 0.68, 0.92), night);
  let carth = mix(vec3f(0.502, 0.133, 0.361), vec3f(0.80, 0.46, 0.54), night);
  let quill = mix(vec3f(0.26, 0.18, 0.12), vec3f(0.60, 0.48, 0.34), night);
  // the real day in sanguine, quieter than either army's ink, visible on the night paper too
  let graphite = mix(vec3f(0.62, 0.32, 0.22), vec3f(0.86, 0.52, 0.40), night);

  // the paper's memory: where each current ran, where men fell
  let st = textureSampleLevel(stain_tex, lin, (w - u.world.xy) / u.world.zw, 0.0);
  col = mix(col, rome, (1.0 - exp(-st.r * 2.5)) * 0.15);
  col = mix(col, carth, (1.0 - exp(-st.g * 2.5)) * 0.17);
  col = mix(col, quill, (1.0 - exp(-st.b * 3.0)) * 0.25);

  let ink = textureLoad(ink_tex, vec2i(px), 0);
  col = mix(col, graphite, ink.a * 0.85);
  // the plan: the quill's ink while drawn, drying into Carthage's purple, the army's own orders
  col = mix(col, mix(quill, carth * mix(1.0, 0.85, 1.0 - night), u.misc.w), ink.b * 0.9);

  // where the two armies meet, the inks fold: each pixel takes one, chosen by which army is the
  // stronger here, combed into feathered chevrons along the front that drift as the lines shear
  let dn = wsample_d(w);
  let dr = dn.r;
  let dc = dn.g;
  let zone = smoothstep(0.04, 0.22, min(dr, dc));
  let bal = (dr - dc) / (dr + dc + 1e-3);
  let h = 3.0;
  let dx = wsample_d(w + vec2f(h, 0.0));
  let dxm = wsample_d(w - vec2f(h, 0.0));
  let dy = wsample_d(w + vec2f(0.0, h));
  let dym = wsample_d(w - vec2f(0.0, h));
  let gb = vec2f((dx.r - dx.g) - (dxm.r - dxm.g), (dy.r - dy.g) - (dym.r - dym.g));
  var nrm = vec2f(0.0, 1.0);
  if (length(gb) > 1e-4) { nrm = normalize(gb); }
  let along = vec2f(-nrm.y, nrm.x);
  let drift = select(u.off.z * 0.35, 0.0, u.misc.x > 0.5);
  let sa = dot(w, along) / 7.0 + drift;
  let chevron = abs(fract(sa) - 0.5) * 2.0 - 0.5;
  let feather = 0.15 * sin(dot(w, nrm) * 0.9 + sa * 6.2831853);
  let m = bal + zone * (1.1 * chevron + feather);
  let mw = max(fwidth(m), 1e-3);
  let pick_r = smoothstep(-mw, mw, m);
  let both = step(0.003, ink.r) * step(0.003, ink.g);
  let wr = mix(1.0, pick_r, max(zone, both));
  let wc = mix(1.0, 1.0 - pick_r, max(zone, both));
  // a thin rim where the inks meet, as the pigment gathers at a marbled edge
  let rim = (1.0 - smoothstep(0.0, 1.5 * mw, abs(m))) * zone * max(ink.r, ink.g);
  let fade = u.misc.y;
  col = mix(col, rome, clamp(ink.r * wr * 0.94 * fade, 0.0, 1.0));
  col = mix(col, carth, clamp(ink.g * wc * 0.94 * fade, 0.0, 1.0));
  col = mix(col, mix(rome, carth, 0.5) * 0.7, rim * 0.25 * fade);
  return vec4f(col, 1.0);
}
