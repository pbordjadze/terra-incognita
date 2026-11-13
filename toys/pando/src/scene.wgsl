// Pando's backdrop, one fragment for every scale (toy-kit backdrop.rs): `world` is the scene's own
// pixel, so a dive's zoom is the kit's camera. u.p0.x is the scale: 0 the grove (its sky, ridges and
// the floor between the stems), 1 one stem (the grove behind, out of focus), 2 one leaf (the blade
// itself, twisting on its stalk), 3 the leaf's skin under the lens (its cells; the pores are painted
// over them).
//   p0 = (scale, horizon px, twist, swing)   p1 = (ppp, a, b, c)
//   leaf: (a, b) the blade's base, c its length (px); skin: (a, b) the finger's shade, c its depth.

fn sky(px: vec2<f32>, horizon: f32) -> vec3<f32> {
  let h = u.size.y;
  let y = px.y / h;
  let hz = horizon / h;
  let top = vec3<f32>(0.18, 0.25, 0.42);
  let mid = vec3<f32>(0.78, 0.62, 0.55);
  let low = vec3<f32>(0.98, 0.78, 0.52);
  var c = mix(top, mid, smoothstep(0.0, hz * 0.75, y));
  c = mix(c, low, smoothstep(hz * 0.55, hz, y));
  // the sun just down behind the ridge on the left
  let d = length((px - vec2<f32>(-0.05 * u.size.x, horizon * 0.93)) / vec2<f32>(u.size.x, h * 0.55));
  c += vec3<f32>(0.35, 0.22, 0.08) * exp(-d * d * 6.0);
  return c;
}

fn ridge(x: f32, k: f32) -> f32 {
  return 0.5 * noise2(vec2<f32>(x * 0.004 + k * 7.0, k)) + 0.3 * noise2(vec2<f32>(x * 0.013 + k * 3.0, k + 2.0)) + 0.2 * noise2(vec2<f32>(x * 0.05, k + 5.0));
}

fn grove(px: vec2<f32>) -> vec3<f32> {
  let horizon = u.p0.y;
  let h = u.size.y;
  var c = sky(px, horizon);
  // three ridges of the Fishlake Plateau, farthest palest; dark firs on the nearest
  let cols = array<vec3<f32>, 3>(vec3<f32>(0.55, 0.47, 0.55), vec3<f32>(0.36, 0.34, 0.38), vec3<f32>(0.20, 0.22, 0.21));
  for (var k = 0; k < 3; k++) {
    let kf = f32(k);
    let top = horizon - h * (0.07 - 0.015 * kf) * ridge(px.x, kf);
    let firs = select(0.0, h * 0.006 * noise2(vec2<f32>(px.x * 0.35, 9.0)), k == 2);
    if (px.y > top - firs) {
      c = cols[k];
    }
  }
  if (px.y > horizon) {
    // the floor between the stems: shade, fallen gold, deeper as it comes near
    let near = clamp((px.y - horizon) / (h - horizon), 0.0, 1.0);
    let floor = mix(vec3<f32>(0.62, 0.45, 0.26), vec3<f32>(0.20, 0.14, 0.08), pow(near, 0.5));
    let speck = noise2(px * 0.02) * 0.5 + noise2(px * 0.07) * 0.5;
    c = floor * (0.8 + 0.4 * speck);
    // far off, the crowns run together into one gold
    c = mix(c, vec3<f32>(0.92, 0.70, 0.36), exp(-(px.y - horizon) / (h * 0.06)) * 0.9);
  }
  return c;
}

fn bokeh(px: vec2<f32>, dark: f32) -> vec3<f32> {
  let p = px / u.size.x;
  let t = u.time * 0.03;
  let g = noise2(p * 2.5 + vec2<f32>(t, 0.0)) * 0.6 + noise2(p * 5.0 - vec2<f32>(0.0, t)) * 0.4;
  let sky = smoothstep(0.62, 0.8, noise2(p * 1.8 + vec2<f32>(3.0, t * 0.5)));
  let trunks = smoothstep(0.82, 0.9, noise2(vec2<f32>(p.x * 7.0, 0.5 + p.y * 0.15)));
  var c = mix(vec3<f32>(0.42, 0.30, 0.14), vec3<f32>(0.98, 0.72, 0.28), g);
  c = mix(c, vec3<f32>(0.62, 0.74, 0.90), sky * 0.8);
  c = mix(c, vec3<f32>(0.88, 0.88, 0.80), trunks * 0.6);
  let y = px.y / u.size.y;
  c *= mix(1.0, 0.45, y * dark);
  return c;
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

fn leaf(px: vec2<f32>) -> vec4<f32> {
  let base = u.p1.yz;
  let len = u.p1.w;
  let twist = u.p0.z;
  let swing = u.p0.w;
  var d = px - base;
  let cs = cos(-swing);
  let sn = sin(-swing);
  d = vec2<f32>(d.x * cs - d.y * sn, d.x * sn + d.y * cs);
  let ct = cos(twist);
  let face = max(abs(ct), 0.03);
  var q = vec2<f32>(d.x / face, d.y) / len;
  // the blade's middle a little under half its length below the base, so the base meets the stalk
  q = vec2<f32>(q.x, q.y - 0.36);
  let r = leaf_r(q);
  let edge = r - length(q);
  let aa = 1.5 / (len * face);
  let inside = smoothstep(-aa, aa, edge);
  if (inside <= 0.0) {
    return vec4<f32>(0.0);
  }
  // light through the leaf: the sun behind it and to the upper right
  let toward = clamp(1.25 - length(px - vec2<f32>(u.size.x * 0.75, u.size.y * 0.42)) / (u.size.y * 0.55), 0.35, 1.25);
  let front = ct >= 0.0;
  var col = select(vec3<f32>(0.90, 0.85, 0.62), vec3<f32>(0.99, 0.70, 0.12), front);
  col *= toward * (0.55 + 0.45 * abs(ct));
  // green lingers along the veins near the base
  let v = veins(q);
  let green = exp(-pow(length(q - vec2<f32>(0.0, -0.45)) / 0.35, 2.0)) * 0.35;
  col = mix(col, vec3<f32>(0.45, 0.55, 0.12) * toward, green * (0.4 + v));
  col = mix(col, select(vec3<f32>(0.96, 0.94, 0.80), vec3<f32>(1.0, 0.92, 0.55), front) * toward, v * 0.45);
  // the leaf's own grain
  col *= 0.94 + 0.08 * noise2(q * 90.0) + 0.05 * noise2(q * 17.0);
  // a darker rim where the blade thins to its teeth
  col *= 0.82 + 0.18 * smoothstep(0.0, 0.02, edge);
  return vec4<f32>(col * inside, inside);
}

// The skin of the leaf under the lens: interlocking cells, bright walls, the finger's shade.
fn skin(px: vec2<f32>) -> vec3<f32> {
  let ppp = u.p1.x;
  let cell = 52.0 * ppp;
  let w = px + vec2<f32>(noise2(px / (cell * 0.45)) - 0.5, noise2(px / (cell * 0.45) + vec2<f32>(7.0, 3.0)) - 0.5) * cell * 0.55;
  let g = w / cell;
  let i = floor(g);
  var d1 = 9.0;
  var d2 = 9.0;
  var id = vec2<f32>(0.0);
  for (var y = -1; y <= 1; y++) {
    for (var x = -1; x <= 1; x++) {
      let o = vec2<f32>(f32(x), f32(y));
      let s = i + o + vec2<f32>(hash2(i + o), hash2(i + o + vec2<f32>(3.1, 7.7))) * 0.8 + 0.1;
      let dd = length(g - s);
      if (dd < d1) {
        d2 = d1;
        d1 = dd;
        id = i + o;
      } else if (dd < d2) {
        d2 = dd;
      }
    }
  }
  let wall = 1.0 - smoothstep(0.0, 0.05, d2 - d1);
  let tone = 0.86 + 0.16 * hash2(id + vec2<f32>(1.7, 2.9));
  var c = vec3<f32>(0.93, 0.70, 0.27) * tone;
  // each cell a little domed: lighter in its middle
  c *= 0.84 + 0.2 * smoothstep(0.0, 0.5, d2 - d1);
  c = mix(c, vec3<f32>(1.0, 0.92, 0.66), wall * 0.65);
  // a few oil drops of the autumn leaf's yellow in the cells
  let drop = smoothstep(0.06, 0.03, length(g - id - vec2<f32>(0.5 + 0.2 * hash2(id), 0.5)));
  c = mix(c, vec3<f32>(1.0, 0.82, 0.25), drop * 0.5);
  let sh = u.p1.w * exp(-dot(px - u.p1.yz, px - u.p1.yz) / pow(130.0 * ppp, 2.0));
  return c * (1.0 - 0.62 * sh);
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let mode = u.p0.x;
  var c = vec3<f32>(0.0);
  if (mode < 0.5) {
    c = grove(world);
  } else if (mode < 1.5) {
    c = bokeh(world, 0.4);
  } else if (mode < 2.5) {
    let b = bokeh(world * 0.6, 0.9) * 0.62;
    let l = leaf(world);
    c = b * (1.0 - l.a) + l.rgb;
  } else {
    c = skin(world);
  }
  // a soft darkening at the corners
  let v = (px / u.size - 0.5) * vec2<f32>(1.0, 0.7);
  c *= 1.0 - 0.5 * dot(v, v);
  return vec4<f32>(c, 1.0);
}
