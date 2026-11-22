// The Qiantang at Yanguan from the seawall, painted in ink-wash (`ink`, below; Te Wei's Shanghai films,
// SOURCES.md). A field (toy-kit field.rs) whose grid is the river's profile and the scene's numbers:
//   row 0, 512 cells: the level along the river (m) at Z = ZMIN + (ZMAX - ZMIN) u², as (the level the
//     water ahead of the front would have there, the level behind it, the current, 0);
//   row 1: k0 (level ahead at the front, level behind it, Froude number, the bore's strength 0..1),
//     k1 (the face's length, the depth ahead, the whelps' wavelength, the roller 0..1),
//     k2 (moon x, moon y, moon radius in pixels, moon age 0..1), k3 (lowest level, highest level, wet, 0).
// u.p0 = camera (x, y, z, pitch); u.p1 = (yaw, tan of half the height in view, front's Z, fade to paper).
// Metres: X across from the wall's edge, Y up from mean sea, Z down the river (view.rs).

const ZMIN: f32 = -400.0;
const ZMAX: f32 = 9000.0;
const WALL: f32 = 7.0;
const BREADTH: f32 = 2800.0;

fn par(k: i32) -> vec4<f32> { return fld(vec2<f32>(f32(k) + 0.5, 1.5)); }

fn prof(z: f32) -> vec4<f32> {
  let u = sqrt(clamp((z - ZMIN) / (ZMAX - ZMIN), 0.0, 1.0));
  return fld(vec2<f32>(clamp(u * 512.0, 0.5, 511.5), 0.5));
}

fn bow(x: f32, zf: f32) -> f32 {
  let a = clamp(x / BREADTH, 0.0, 1.2);
  return 90.0 * a * a + 8.0 * sin(x * 0.011 + zf * 0.004);
}

// the water's surface: the level ahead and behind joined across the bore's face, the whelps behind it
struct Water { eta: f32, s: f32, f: f32 };
fn water(x: f32, z: f32) -> Water {
  let k0 = par(0);
  let k1 = par(1);
  let p = prof(z);
  let zf = u.p1.z;
  let s = z - (zf + bow(x, zf));
  var o: Water;
  o.s = s;
  if (k0.w < 0.01) { o.eta = p.x; o.f = 0.0; return o; }
  let l = max(k1.x, 0.5);
  let f = smoothstep(-l, 0.0, s);
  let dh = k0.y - k0.x;
  // the whelps: an undular bore's train of waves behind its front, gone as it breaks
  let lam = max(k1.z, 4.0);
  let und = (1.0 - k1.w) * 0.42 + 0.12;
  let whelp = select(0.0, dh * und * sin(6.2832 * s / lam) * exp(-s / (5.0 * lam)), s > 0.0);
  // the roller: a breaking bore's front curls a little above the level behind it
  let roll = k1.w * dh * 0.28 * exp(-pow((s + l * 0.15) / (l * 0.5 + 0.6), 2.0));
  o.eta = mix(p.x, p.y, f) + whelp + roll;
  o.f = f;
  return o;
}

fn fbm(p: vec2<f32>) -> f32 {
  return noise2(p) * 0.5 + noise2(p * 2.03 + 3.1) * 0.3 + noise2(p * 4.1 + 7.7) * 0.2;
}

fn ground(x: f32) -> f32 { return select(max(WALL - x, -3.0), WALL, x < 0.0); }

fn cam_ray(px: vec2<f32>) -> vec3<f32> {
  let half = u.size.y * 0.5;
  let n = vec2<f32>((px.x - u.size.x * 0.5) / half, (half - px.y) / half) * u.p1.y;
  let d = normalize(vec3<f32>(n.x, n.y, 1.0));
  let cp = cos(u.p0.w); let sp = sin(u.p0.w);
  let d1 = vec3<f32>(d.x, d.y * cp - d.z * sp, d.y * sp + d.z * cp);
  let cy = cos(u.p1.x); let sy = sin(u.p1.x);
  return vec3<f32>(d1.x * cy + d1.z * sy, d1.y, -d1.x * sy + d1.z * cy);
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let o = u.p0.xyz;
  let d = cam_ray(px);
  let k3 = par(3);
  let paper = paper_at(px);
  var ink = 0.0;
  var dist = 1e6;
  // the sky: bare paper, a faint wash gathering low over the far shore, and the moon
  ink = 0.05 * smoothstep(-0.02, 0.08, 0.12 - d.y) * (0.7 + 0.6 * noise2(px * 0.003));
  ink = max(ink, moon(px));
  // the far shore: a pale low wash of land and trees along the horizon, straight across
  let tb = (BREADTH - o.x) / max(d.x, 1e-4);
  if (d.x > 0.0) {
    let pb = o + d * tb;
    let trees = 3.0 + 9.0 * noise2(vec2<f32>(pb.z * 0.004, 0.5)) + 4.0 * noise2(vec2<f32>(pb.z * 0.03, 2.0));
    if (pb.y < trees && pb.y > -1.0) {
      dist = tb;
      ink = 0.16 + 0.08 * noise2(px * vec2<f32>(0.02, 0.2));
    }
  }
  // the promenade, and the revetment sloping from its edge into the river
  var tg = 1e6;
  var gp = vec3<f32>(0.0);
  let tp = (WALL - o.y) / d.y;
  if (d.y < 0.0 && tp > 0.0) {
    let pp = o + d * tp;
    if (pp.x < 0.0) { tg = tp; gp = pp; }
  }
  let den = d.x + d.y;
  if (tg > 1e5 && abs(den) > 1e-5) {
    let tr = (WALL - o.x - o.y) / den;
    let pr = o + d * tr;
    if (tr > 0.0 && pr.x >= 0.0 && pr.x < 10.0) { tg = tr; gp = pr; }
  }
  // the water: marched between its highest and lowest levels, then pinned down by halving
  var tw = 1e6;
  var wp = vec3<f32>(0.0);
  if (d.y < 0.0) {
    let t0 = max((k3.y - o.y) / d.y, 0.0);
    let t1 = min((k3.x - o.y) / d.y, 14000.0);
    var a = t0;
    var hit = false;
    for (var i = 1; i <= 40; i = i + 1) {
      let f = f32(i) / 40.0;
      let t = mix(t0, t1, f * f * 0.5 + f * 0.5);
      let p = o + d * t;
      if (p.y < water(p.x, p.z).eta) {
        var lo = a; var hi = t;
        for (var j = 0; j < 7; j = j + 1) {
          let m = 0.5 * (lo + hi);
          let q = o + d * m;
          if (q.y < water(q.x, q.z).eta) { hi = m; } else { lo = m; }
        }
        tw = hi; hit = true;
        break;
      }
      a = t;
    }
    if (!hit && t1 >= 13999.0) { tw = t1; }
    if (hit || t1 >= 13999.0) { wp = o + d * tw; }
  }
  if (tg < tw && tg < dist) {
    dist = tg;
    ink = stone(px, gp, tg, k3.z);
  } else if (tw < dist) {
    dist = tw;
    if (wp.x > 0.0 && wp.x < BREADTH) {
      ink = river_ink(px, wp, d, tw);
    }
  }
  // far things fade into the paper's haze
  if (dist < 1e5) { ink *= exp(-dist / 9000.0); }
  ink *= 1.0 - u.p1.w;
  ink *= 0.88 + 0.24 * noise2(px * 0.55);
  let col = mix(paper, vec3<f32>(0.075, 0.075, 0.085), clamp(ink, 0.0, 1.0));
  return vec4<f32>(seal(px, col), 1.0);
}

// The river painted: calm water a few pale dry strokes, the bore's face a dark wash darkest under its
// crest, its roller bare paper with a ragged inked edge, the churned flood behind it broken patches.
fn river_ink(px: vec2<f32>, p: vec3<f32>, d: vec3<f32>, t: f32) -> f32 {
  let k0 = par(0);
  let k1 = par(1);
  let w = water(p.x, p.z);
  let e = max(0.25, t * 0.004);
  let dz = (water(p.x, p.z + e).eta - water(p.x, p.z - e).eta) / (2.0 * e);
  let dh = max(k0.y - k0.x, 0.01);
  let bore = k0.w;
  // calm water: long horizontal strokes, closer together far off
  let sc = u.size.y / 2556.0;
  let band = noise2(vec2<f32>(px.x * 0.006 / sc + noise2(px * 0.004 / sc) * 3.0, px.y * 0.45 / sc + noise2(px * 0.02 / sc) * 1.5));
  let streak = smoothstep(0.74, 0.86, band) * (0.12 + 0.1 * smoothstep(2000.0, 200.0, t)) * smoothstep(0.3, 0.6, noise2(px * 0.003 / sc));
  var ink = 0.05 + streak;
  // the current behind the front: the strokes thicken and run
  let cur = abs(prof(p.z).z);
  ink += 0.06 * smoothstep(0.5, 3.0, cur) * smoothstep(0.6, 0.85, noise2(vec2<f32>(px.x * 0.01, px.y * 0.06 + u.time * 0.8)));
  // the face, anything of the water that leans toward us
  let face = smoothstep(0.02, 0.2, dz) * bore;
  let up = clamp((w.eta - k0.x) / dh, 0.0, 1.0);
  let streaks = 0.85 + 0.2 * noise2(vec2<f32>(px.x * 0.12 / sc, px.y * 0.01 / sc));
  ink = max(ink, face * (0.62 + 0.33 * up) * streaks);
  // the roller and the churned water behind a breaking bore: paper left bare, inked at its edges
  let rl = k1.w * bore;
  let l = max(k1.x, 0.5);
  // the roller sits on the crest and just behind it
  let near_crest = exp(-pow((w.s - l * 0.15) / (l * 0.45 + 0.6 + t * 0.0015), 2.0));
  let churn = select(0.0, exp(-w.s / 700.0), w.s > 0.0) * max(rl, 0.25 * bore);
  // foam lies in streaks along the front, stretched across the river
  let foam_n = fbm(vec2<f32>(p.x * 0.02, p.z * 0.12 - u.time * 0.05)) * 0.8 + noise2(px * 0.02 / sc) * 0.2;
  let foam = max(near_crest * rl * smoothstep(0.3, 0.42, foam_n + 0.12), churn * smoothstep(0.5, 0.6, foam_n));
  // the foam's edge, where the noise crosses its threshold: the brush's ragged line round the white
  let rim = max(near_crest * rl * (1.0 - smoothstep(0.0, 0.03, abs(foam_n + 0.12 - 0.36))), churn * (1.0 - smoothstep(0.0, 0.02, abs(foam_n - 0.55))));
  ink = mix(ink, 0.0, clamp(foam * 1.2, 0.0, 1.0));
  // between the foam the churned water is darker, the risen sea
  ink = max(ink, churn * (1.0 - foam) * (0.5 + 0.15 * noise2(px * 0.01 / sc)));
  let dry = smoothstep(0.4, 0.65, noise2(vec2<f32>(px.x * 0.05, px.y * 0.12) / sc));
  ink = max(ink, rim * dry * 0.9);
  // the white line itself, seen from afar: the crest a thin bare stroke with a dark lip under it
  let lip = exp(-pow((w.s + l * 0.6) / (l * 0.3 + t * 0.0015), 2.0)) * bore * smoothstep(0.3, 1.0, dh);
  ink = max(ink, lip * 0.7 * dry);
  return ink;
}

// The seawall: the promenade's flagstones and the revetment's courses in a light dry brush; stone left wet
// darker where the water has been.
fn stone(px: vec2<f32>, p: vec3<f32>, t: f32, wet: f32) -> f32 {
  var ink = 0.1 + 0.06 * noise2(px * 0.02);
  if (p.x < 0.0) {
    let j = min(abs(fract(p.z / 2.4) - 0.5), abs(fract(p.x / 1.8 + floor(p.z / 2.4) * 0.5) - 0.5));
    ink += 0.25 * (1.0 - smoothstep(0.0, 0.03 + t * 0.0012, j)) * smoothstep(0.3, 0.6, noise2(px * 0.08));
    ink += wet * 0.25 * smoothstep(-4.0, -0.3, p.x) * smoothstep(0.35, 0.6, noise2(vec2<f32>(p.x * 0.6, p.z * 0.3)));
  } else {
    // the revetment's fish-scale courses: arcs of stone along the slope
    let along = p.z / 1.2;
    let course = (WALL - p.y) / 0.5;
    let arc = abs(fract(course + 0.25 * cos(6.2832 * fract(along + floor(course) * 0.5))) - 0.5);
    ink += 0.16 * (1.0 - smoothstep(0.0, 0.03 + t * 0.001, arc)) * smoothstep(0.4, 0.65, noise2(px * 0.03));
    ink = 0.06 + ink * 0.7 + 0.16 * smoothstep(WALL - 3.0, 0.0, p.y);
    ink += wet * 0.15;
    // where the water meets the stones: a dark wet band just above it, and the line itself, so you can count
    // the courses the tide swallows
    let lv = water(p.x, p.z).eta;
    let above = p.y - lv;
    ink += 0.22 * (1.0 - smoothstep(0.0, 0.7, above)) * (0.7 + 0.3 * noise2(px * 0.05));
    ink = max(ink, 0.75 * (1.0 - smoothstep(0.0, 0.08 + t * 0.002, above)));
  }
  // the edge of the wall: a firm stroke
  let edge = 1.0 - smoothstep(0.0, 0.06 + t * 0.002, abs(p.x));
  return max(ink, edge * 0.85);
}

// The moon, by day pale: its lit part bare paper inside a dry ring, its dark part a faint wash; a halo.
fn moon(px: vec2<f32>) -> f32 {
  let k2 = par(2);
  let r = k2.z;
  let q = (px - k2.xy) / r;
  let l = length(q);
  let halo = 0.05 * smoothstep(3.5, 1.0, l) * (0.7 + 0.6 * noise2(px * 0.01));
  if (l > 1.0) { return halo + 0.5 * (1.0 - smoothstep(0.0, 1.8 / r, l - 1.0)) * smoothstep(0.35, 0.6, noise2(px * 0.3)); }
  // the terminator: the lit fraction by age (0 new, 0.5 full), lit from the right while waxing
  let ph = cos(6.2832 * k2.w);
  let side = select(-1.0, 1.0, k2.w < 0.5);
  let xs = q.x * side;
  let edge = ph * sqrt(max(1.0 - q.y * q.y, 0.0));
  let lit = smoothstep(-0.04, 0.04, xs - edge);
  return mix(0.32, 0.0, lit) + 0.06 * noise2(px * 0.2);
}

// the paper: warm, mottled, with long fibres two ways
fn paper_at(px: vec2<f32>) -> vec3<f32> {
  let m = noise2(px * 0.004) * 0.6 + noise2(px * 0.017) * 0.4;
  let r1 = vec2<f32>(px.x * 0.87 + px.y * 0.5, px.y * 0.87 - px.x * 0.5);
  let r2 = vec2<f32>(px.x * 0.34 - px.y * 0.94, px.y * 0.34 + px.x * 0.94);
  let f1 = noise2(vec2<f32>(r1.x * 0.45, r1.y * 0.006)) * noise2(r1 * 0.01 + 5.0);
  let f2 = noise2(vec2<f32>(r2.x * 0.45, r2.y * 0.006) + 7.0) * noise2(r2 * 0.01 + 9.0);
  let fibre = smoothstep(0.62, 0.8, max(f1, f2));
  return vec3<f32>(0.935, 0.905, 0.835) * (0.97 + 0.05 * m) - fibre * 0.012;
}

// The painter's seal, low on the right, in vermilion: a square of strokes cut through it.
fn seal(px: vec2<f32>, col: vec3<f32>) -> vec3<f32> {
  let side = u.size.x * 0.08;
  let o = vec2<f32>(u.size.x * 0.84, u.size.y - u.size.x * 0.4);
  let q = (px - o) / side;
  if (q.x < 0.0 || q.y < 0.0 || q.x > 1.0 || q.y > 1.0) { return col; }
  let lines = abs(fract(q.y * 3.0 - 0.5) - 0.5) < 0.07 || abs(q.x - 0.5) < 0.05;
  let cut = lines && q.x > 0.16 && q.x < 0.84 && q.y > 0.16 && q.y < 0.84;
  let worn = noise2(px * 0.4) < 0.18 || noise2(px * 0.05) * (1.0 - min(min(q.x, q.y), min(1.0 - q.x, 1.0 - q.y)) * 6.0) > 0.6;
  if (cut || worn) { return col; }
  return mix(col, vec3<f32>(0.80, 0.14, 0.09), 0.93 * (1.0 - u.p1.w));
}
