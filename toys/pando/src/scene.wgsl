// Pando's backdrop, one fragment for every scale (toy-kit backdrop.rs): `world` is the scene's own
// pixel, so a dive's zoom is the kit's camera. u.p0.x is the scale: 0 the grove (its sky, ridges and
// the floor between the stems), 1 one stem (the grove behind, out of focus), 2 one leaf (the blade
// itself, twisting on its stalk), 3 the leaf's skin under the lens (its cells; the pores are painted
// over them).
//   p0 = (scale, horizon px, twist, swing)   p1 = (ppp, a, b, c)
//   leaf: (a, b) the blade's base, c its length (px); skin: (a, b) the finger's shade, c its depth.


fn ridge(x: f32, k: f32) -> f32 {
  return 0.5 * noise2(vec2<f32>(x * 0.004 + k * 7.0, k)) + 0.3 * noise2(vec2<f32>(x * 0.013 + k * 3.0, k + 2.0)) + 0.2 * noise2(vec2<f32>(x * 0.05, k + 5.0));
}



// An aspen leaf's outline: nearly round, a short sharp tip, the base squared off, fine round teeth.
// `q` in leaf lengths from the blade's middle, y toward the tip.
fn leaf_r(q: vec2<f32>) -> f32 {
  let t = atan2(q.x, q.y);
  var r = 0.5 * (1.0 + 0.10 * cos(t)) * (1.0 - 0.06 * cos(2.0 * t));
  r *= 1.0 + 0.30 * exp(-(t / 0.16) * (t / 0.16));
  let b = (abs(t) - 3.14159) / 0.5;
  r *= 1.0 - 0.14 * exp(-b * b);
  let sides = smoothstep(0.5, 1.2, abs(t)) * (1.0 - smoothstep(2.6, 3.0, abs(t)));
  r *= 1.0 + 0.016 * sides * (0.5 + 0.5 * cos(t * 42.0));
  return r;
}

fn seg(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba * h);
}

// The veins: the midrib, five pairs of side veins curving toward the tip, and the fine net.
fn veins(q: vec2<f32>) -> f32 {
  var v = exp(-pow(abs(q.x) / 0.007, 2.0)) * step(0.0, q.y + 0.5) * step(q.y, 0.48);
  for (var i = 0; i < 5; i++) {
    let fi = f32(i);
    let y0 = -0.47 + fi * 0.16;
    let s = 0.40 - fi * 0.06;
    let a = vec2<f32>(0.0, y0);
    let ang = 0.95 - fi * 0.10;
    let m = a + vec2<f32>(sin(ang), cos(ang)) * s * 0.55;
    let e = m + vec2<f32>(sin(ang * 0.45), cos(ang * 0.45)) * s * 0.5;
    let x = vec2<f32>(abs(q.x), q.y);
    let d = min(seg(x, a, m), seg(x, m, e));
    v = max(v, exp(-pow(d / 0.0045, 2.0)) * 0.8);
  }
  // the net between them
  let g = q * 46.0;
  let n = abs(noise2(g) - noise2(g + vec2<f32>(5.2, 1.3)));
  v = max(v, (1.0 - smoothstep(0.0, 0.06, n)) * 0.25);
  return v;
}



// The plotter's ink on its paper (plotter.rs): `line` is how much of a pen line covers this pixel.
const PAPER: vec3<f32> = vec3<f32>(0.96, 0.95, 0.91);
const INK: vec3<f32> = vec3<f32>(0.08, 0.08, 0.10);
fn pen(d: f32) -> f32 {
  // d: distance from the line's middle, pixels; the pen's width in points
  let w = 0.55 * u.p1.x * 0.5;
  return smoothstep(w + 0.75, w - 0.75, abs(d));
}

// The grove drawn: the horizon, the three ridges' lines (the firs' ragged one nearest).
fn grove_lines(px: vec2<f32>) -> f32 {
  let horizon = u.p0.y;
  let h = u.size.y;
  var l = pen(px.y - horizon);
  for (var k = 0; k < 3; k++) {
    let kf = f32(k);
    let x0 = px.x;
    let top = horizon - h * (0.07 - 0.015 * kf) * ridge(x0, kf);
    let firs = select(0.0, h * 0.006 * noise2(vec2<f32>(x0 * 0.35, 9.0)), k == 2);
    // the line's distance measured down the page, steepened where the profile is
    let dx = 1.5;
    let t1 = horizon - h * (0.07 - 0.015 * kf) * ridge(x0 + dx, kf) - select(0.0, h * 0.006 * noise2(vec2<f32>((x0 + dx) * 0.35, 9.0)), k == 2);
    let slope = (t1 - (top - firs)) / dx;
    let d = (px.y - (top - firs)) / sqrt(1.0 + slope * slope);
    if (px.y < horizon + 1.0) { l = max(l, pen(d)); }
  }
  return l;
}

// The leaf drawn: its edge, the midrib and the side veins (the fine net is left out).
fn leaf_lines(px: vec2<f32>) -> f32 {
  let base = u.p1.yz;
  let len = u.p1.w;
  let twist = u.p0.z;
  let swing = u.p0.w;
  var d = px - base;
  let cs = cos(-swing);
  let sn = sin(-swing);
  d = vec2<f32>(d.x * cs - d.y * sn, d.x * sn + d.y * cs);
  let face = max(abs(cos(twist)), 0.03);
  var q = vec2<f32>(d.x / face, d.y) / len;
  q = vec2<f32>(q.x, q.y - 0.36);
  let r = leaf_r(q);
  let edge = (r - length(q)) * len * face;
  var l = pen(edge);
  if (edge > 0.0) {
    let v = veins(q);
    l = max(l, smoothstep(0.62, 0.78, v) * step(0.5, v));
  }
  return l;
}

// The skin drawn: the walls between its cells, and a finger's shade over them.
fn skin_lines(px: vec2<f32>) -> f32 {
  let ppp = u.p1.x;
  let cell = 52.0 * ppp;
  let w = px + vec2<f32>(noise2(px / (cell * 0.45)) - 0.5, noise2(px / (cell * 0.45) + vec2<f32>(7.0, 3.0)) - 0.5) * cell * 0.55;
  let g = w / cell;
  let i = floor(g);
  var d1 = 9.0;
  var d2 = 9.0;
  var s1 = vec2<f32>(0.0);
  var s2 = vec2<f32>(9.0);
  for (var y = -1; y <= 1; y++) {
    for (var x = -1; x <= 1; x++) {
      let o = vec2<f32>(f32(x), f32(y));
      let s = i + o + vec2<f32>(hash2(i + o), hash2(i + o + vec2<f32>(3.1, 7.7))) * 0.8 + 0.1;
      let dd = length(g - s);
      if (dd < d1) {
        d2 = d1;
        s2 = s1;
        d1 = dd;
        s1 = s;
      } else if (dd < d2) {
        d2 = dd;
        s2 = s;
      }
    }
  }
  // the true distance to the wall between the two nearest (their bisector), so the pen keeps its width
  // where walls meet
  let wall = (d2 * d2 - d1 * d1) / (2.0 * max(length(s2 - s1), 1e-4));
  var l = pen(wall * cell);
  // the finger's shade, hatched as a plotter shades: diagonal strokes, crossed where it is deepest
  let dsh = (px - u.p1.yz) / (130.0 * ppp);
  let sh = u.p1.w * exp(-dot(dsh, dsh));
  let gap = 7.0 * ppp;
  let h1 = abs(fract((px.x + px.y) / gap) - 0.5) * gap;
  let h2 = abs(fract((px.x - px.y) / gap) - 0.5) * gap;
  l = max(l, pen(h1) * step(0.25, sh));
  l = max(l, pen(h2) * step(0.6, sh));
  return l;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let mode = u.p0.x;
  var l = 0.0;
  if (mode < 0.5) {
    l = grove_lines(world);
  } else if (mode < 1.5) {
    // one stem: the grove behind it is out of the drawing
    l = 0.0;
  } else if (mode < 2.5) {
    l = leaf_lines(world);
  } else {
    l = skin_lines(world);
  }
  return vec4<f32>(mix(PAPER, INK, l), 1.0);
}
