// Moon Road's night, in Paint by Moonlight's colours: the plum sky, the low moon, and Lake Superior
// as a numbered template of facets (lake.rs) on the app's dark paper, each painted in a moonlight paint
// where the moon has lit it, so the road reads as light.
// A field (toy-kit field.rs): per cell (painted, how lately lit, seed across, seed out + 2 a paint past the first).
// u.p0 = (horizon y px, focal px a radian, eye along the shore m, the moon's elevation),
// u.p1 = (DU, DV, the first row's index V0, eye height m).

const PAPER = vec3<f32>(0.105, 0.080, 0.145);
const LINE = vec3<f32>(0.33, 0.28, 0.40);
const NUMERAL = vec3<f32>(0.47, 0.42, 0.54);
const MOONLIGHT = vec3<f32>(0.985, 0.915, 0.72);
const FLASH = vec3<f32>(1.0, 0.975, 0.90);

fn paint_of(k: i32) -> vec3<f32> {
  // the lake's three moonlight paints: a marshmallow, a candle wax, a pale lilac
  // (one family, near in value: the road reads as one light)
  if (k == 0) { return vec3<f32>(0.93, 0.90, 0.80); }
  if (k == 1) { return vec3<f32>(0.92, 0.84, 0.67); }
  return vec3<f32>(0.84, 0.81, 0.89);
}

fn seg(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
  let pa = p - a; let ba = b - a;
  let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba * h);
}

// A numeral 1, 2 or 3 drawn as strokes in a box a unit tall (y down), its distance from p.
fn numeral(p: vec2<f32>, k: i32) -> f32 {
  var d = 1e5;
  if (k == 1) {
    d = min(d, seg(p, vec2(0.0, -0.5), vec2(0.0, 0.5)));
    d = min(d, seg(p, vec2(0.0, -0.5), vec2(-0.17, -0.34)));
    d = min(d, seg(p, vec2(-0.16, 0.5), vec2(0.16, 0.5)));
  } else if (k == 2) {
    d = min(d, seg(p, vec2(-0.22, -0.30), vec2(-0.12, -0.45)));
    d = min(d, seg(p, vec2(-0.12, -0.45), vec2(0.04, -0.5)));
    d = min(d, seg(p, vec2(0.04, -0.5), vec2(0.19, -0.43)));
    d = min(d, seg(p, vec2(0.19, -0.43), vec2(0.24, -0.26)));
    d = min(d, seg(p, vec2(0.24, -0.26), vec2(0.16, -0.06)));
    d = min(d, seg(p, vec2(0.16, -0.06), vec2(-0.24, 0.48)));
    d = min(d, seg(p, vec2(-0.24, 0.49), vec2(0.26, 0.49)));
  } else {
    d = min(d, seg(p, vec2(-0.22, -0.40), vec2(-0.06, -0.5)));
    d = min(d, seg(p, vec2(-0.06, -0.5), vec2(0.13, -0.47)));
    d = min(d, seg(p, vec2(0.13, -0.47), vec2(0.21, -0.31)));
    d = min(d, seg(p, vec2(0.21, -0.31), vec2(0.12, -0.12)));
    d = min(d, seg(p, vec2(0.12, -0.12), vec2(-0.06, -0.05)));
    d = min(d, seg(p, vec2(-0.06, -0.05), vec2(0.15, 0.01)));
    d = min(d, seg(p, vec2(0.15, 0.01), vec2(0.24, 0.20)));
    d = min(d, seg(p, vec2(0.24, 0.20), vec2(0.17, 0.41)));
    d = min(d, seg(p, vec2(0.17, 0.41), vec2(-0.02, 0.5)));
    d = min(d, seg(p, vec2(-0.02, 0.5), vec2(-0.24, 0.42)));
  }
  return d;
}

fn sky(px: vec2<f32>) -> vec3<f32> {
  let hy = u.p0.x;
  let f = u.p0.y;
  let t = clamp((hy - px.y) / hy, 0.0, 1.0);
  // the plum night, deeper overhead
  var col = mix(vec3<f32>(0.30, 0.17, 0.335), vec3<f32>(0.095, 0.042, 0.135), pow(t, 0.55));
  let moon = vec2<f32>(u.size.x * 0.5, hy - u.p0.w * f);
  let r = 0.016 * f;
  let dm = length(px - moon);
  // its glow on the haze, wide and faint, then close
  col = col + vec3<f32>(0.42, 0.33, 0.36) * exp(-dm / (u.size.y * 0.11)) * 0.55;
  col = col + vec3<f32>(0.55, 0.48, 0.45) * exp(-dm / (r * 2.2)) * 0.35;
  // a few long wisps low over the water
  let w = noise2(vec2<f32>(px.x / u.size.x * 3.0 + u.time * 0.004, px.y / u.size.y * 34.0));
  col = col + vec3<f32>(0.03, 0.02, 0.028) * smoothstep(0.78, 0.97, w) * smoothstep(0.55, 0.9, 1.0 - t) * (1.0 + 2.0 * exp(-dm / (u.size.y * 0.2)));
  // stars, fainter toward the horizon's haze and the moon
  let g = px / (u.scale * 3.4);
  let cell = floor(g);
  let h = hash2(cell);
  if (h > 0.93) {
    let sp = vec2<f32>(hash2(cell + 3.1), hash2(cell + 7.7));
    let ds = length(fract(g) - sp) * u.scale * 3.4;
    let b = pow(hash2(cell + 1.3), 3.0) * smoothstep(0.08, 0.5, t) * smoothstep(r * 4.0, r * 14.0, dm);
    let tw = 0.75 + 0.25 * sin(u.time * (1.0 + 3.0 * h) + h * 40.0);
    col = col + vec3<f32>(0.98, 0.93, 0.85) * b * tw * exp(-ds * ds / (u.scale * u.scale * 0.55));
  }
  // the moon: a low one, warm, its seas faint
  let disc = 1.0 - smoothstep(r - u.scale * 0.6, r + u.scale * 0.6, dm);
  let q = (px - moon) / r;
  let seas = noise2(q * 2.3 + 4.0) * 0.6 + noise2(q * 5.0) * 0.4;
  let mc = MOONLIGHT * (1.0 - 0.07 * smoothstep(0.45, 0.75, seas));
  col = mix(col, mc, disc);
  // the far shore of the bay, a low dark point to the right
  let xs = px.x / u.size.x;
  let rise = smoothstep(0.60, 0.70, xs);
  let ridge = hy - u.scale * rise * (3.0 + 9.0 * smoothstep(0.70, 0.86, xs) - 2.5 * smoothstep(0.88, 1.0, xs) + 1.2 * noise2(vec2<f32>(xs * 40.0, 1.0)));
  if (px.y > ridge && rise > 0.0) {
    col = vec3<f32>(0.085, 0.045, 0.105);
  }
  return col;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let hy = u.p0.x;
  let f = u.p0.y;
  if (px.y < hy + 0.5 * u.scale) {
    return vec4<f32>(sky(px), 1.0);
  }
  let du = u.p1.x; let dv = u.p1.y; let v0 = u.p1.z; let eye = u.p1.w;
  let ang = (px.y - hy) / f;
  let d = eye / tan(ang);
  let x = u.p0.z + (px.x - u.size.x * 0.5) / f * d;
  // the row out (absolute: v = ln D / dv); a row's cells are du * its own distance wide, in metres
  let v = log(d) / dv;
  // how tall a cell is here, in pixels: ~(y - horizon) * DV
  let cellpx = (px.y - hy) * dv;
  // the haze far out: the paper and its paint go lavender-grey toward the horizon
  let haze = exp(-(px.y - hy) / (u.scale * 26.0));
  let hazec = vec3<f32>(0.27, 0.16, 0.31);
  let row = v - v0;
  var col = PAPER;
  if (row < f32(u.grid.y) - 1.0 && cellpx > 0.9) {
    // the nearest seed, then the distance to its cell's edge (the nearest bisector with a
    // neighbour), in screen pixels from this pixel to the seed
    var best = 1e9;
    var bc = vec4<f32>(0.0); var bv = 0.0; var bid = vec2<f32>(0.0); var pb = vec2<f32>(0.0);
    var seeds: array<vec2<f32>, 15>;
    var n = 0;
    for (var j = -2; j <= 2; j = j + 1) {
      let r = floor(v) + f32(j);
      let cr = r - v0;
      if (cr < 0.0 || cr >= f32(u.grid.y)) { continue; }
      let w = du * exp((r + 0.5) * dv);
      let c0 = floor(x / w);
      for (var i = -1; i <= 1; i = i + 1) {
        let c = c0 + f32(i);
        let fc = fld(vec2<f32>(c, cr) + 0.5);
        let k = floor(fc.w / 2.0 + 0.001);
        let sv = r + fc.w - 2.0 * k;
        // this pixel from the seed, in pixels
        let ps = vec2<f32>((x - (c + fc.z) * w) / d * f, (v - sv) * cellpx);
        seeds[n] = ps; n = n + 1;
        let l = dot(ps, ps);
        if (l < best) { best = l; bc = fc; bv = sv; bid = vec2<f32>(c, r); pb = ps; }
      }
    }
    var edge = 1e9;
    for (var s = 0; s < n; s = s + 1) {
      let ps = seeds[s];
      let ab = pb - ps;
      let ll = dot(ab, ab);
      if (ll > 1e-4) {
        // the bisector of the two seeds, as seen from this pixel
        edge = min(edge, dot((pb + ps) * 0.5, -ab) / sqrt(ll));
      }
    }
    edge = max(edge, 0.0);
    let painted = bc.x;
    let lit = bc.y;
    let k = i32(floor(bc.w / 2.0 + 0.001));
    let paint = paint_of(k);
    // a newly painted cell flashes, then settles into its paint
    // a painted cell settles to a soft paint; the moon in it lately lifts it toward the flash, so the
    // live road outshines the trail it leaves
    let settled = mix(paint, vec3<f32>(0.66, 0.62, 0.80), 0.55) * 0.66;
    col = select(PAPER, mix(settled, paint, smoothstep(0.0, 0.6, lit)), painted > 0.01);
    col = mix(col, FLASH, smoothstep(0.55, 1.0, lit));
    // the template's line between cells: thin, grey on paper, faint on paint; gone where cells are tiny
    let lw = 0.42 * u.scale;
    let la = (1.0 - smoothstep(lw * 0.6, lw * 1.4, edge)) * smoothstep(3.0, 14.0, cellpx) * mix(0.55, 0.16, painted);
    col = mix(col, LINE, la);
    // the paint's number, in about half the unpainted cells big enough to hold it (judged at the
    // cell's own seed, so a numeral is never cut by the row it straddles)
    let cw = du * f;
    let seedpx = atan(eye / exp(bv * dv)) * f * dv;
    let numh = min(7.0 * u.scale, seedpx * 0.55);
    if (n > 0 && painted < 0.5 && numh > 3.4 * u.scale && cw > numh * 1.2 && hash2(bid * 1.37) < 0.42) {
      // (out is up the screen)
      let q = vec2<f32>(pb.x, -pb.y) / numh;
      let nd = numeral(q, k + 1) * numh;
      let ink = 1.0 - smoothstep(0.38 * u.scale, 0.9 * u.scale, nd);
      col = mix(col, NUMERAL, ink * 0.85 * (1.0 - painted * 2.0));
    }
  } else {
    // too far for the template's lines: the road's glitter alone, on the haze
    let a = abs(px.x - u.size.x * 0.5) / f;
    let road = exp(-a * a / (2.0 * 0.004 * 0.004)) * (0.4 + 0.6 * noise2(vec2<f32>(px.x * 0.5 / u.scale, px.y * 2.0 / u.scale + u.time * 3.0)));
    col = mix(PAPER, MOONLIGHT, clamp(road, 0.0, 1.0));
  }
  // (the haze dims the dark water more than the light on it)
  let lum = dot(col, vec3<f32>(0.3, 0.5, 0.2));
  col = mix(col, hazec, haze * 0.75 * (1.0 - smoothstep(0.3, 0.8, lum)));
  // the paper's grain
  col = col * (0.97 + 0.06 * noise2(px / (u.scale * 0.9)));
  return vec4<f32>(col, 1.0);
}
