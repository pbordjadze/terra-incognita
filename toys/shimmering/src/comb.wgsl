// The curtain of giant honeybees under the cliff (comb.rs), face on, in the evening: the overhang's dark
// rock along the top, the comb hanging from it covered in bees, heads up, overlapping like shingles, each
// dark amber with gold bands; a flip lifts the abdomen toward the eye, so it shortens and catches the light,
// and the wings open and glint. The hornet hovers in front, its shadow on the bees.
// A field (toy-kit field.rs): per cell (abdomen up 0..1, a bee 0/1, flash, -).
// u.p0 = (CELL cm, the hornet's x, y cm, its heading); u.p1 = (comb centre x, top, half width, depth).

const SKY = vec3<f32>(0.10, 0.12, 0.09);
const ROCK = vec3<f32>(0.11, 0.09, 0.075);
const WAX = vec3<f32>(0.42, 0.30, 0.13);
const BODY = vec3<f32>(0.10, 0.055, 0.025);
const AMBER = vec3<f32>(0.80, 0.48, 0.12);
const GOLD = vec3<f32>(1.0, 0.80, 0.36);
const FUR = vec3<f32>(0.45, 0.32, 0.14);

fn rot(v: vec2<f32>, a: f32) -> vec2<f32> {
  let c = cos(a); let s = sin(a);
  return vec2<f32>(c * v.x + s * v.y, -s * v.x + c * v.y);
}

// soft-edged ellipse coverage of q, radii r, edge e (cm)
fn ell(q: vec2<f32>, r: vec2<f32>, e: f32) -> f32 {
  let d = (length(q / r) - 1.0) * min(r.x, r.y);
  return 1.0 - smoothstep(-e, e, d);
}

// One bee at its cell `k`: colour and coverage at `world`.
fn bee(world: vec2<f32>, k: vec2<f32>, e: f32) -> vec4<f32> {
  let f = fld(k + 0.5);
  if (f.y < 0.5) { return vec4<f32>(0.0); }
  let cell = u.p0.x;
  let h = hash2(k);
  let row = (k.y % 2.0) * 0.5;
  let c = (k + vec2<f32>(0.5 + row - 0.25 + (hash2(k + 3.3) - 0.5) * 0.35, 0.5 + (hash2(k + 5.1) - 0.5) * 0.3)) * cell;
  let ang = (h - 0.5) * 0.6;
  let s = cell * 1.25;
  let q = rot(world - c, ang) / s;
  let up = f.x;
  // the abdomen: hanging down, banded; lifted, foreshortened toward the thorax
  let ac = vec2<f32>(0.0, mix(0.34, 0.12, up));
  let ar = vec2<f32>(mix(0.25, 0.29, up), mix(0.44, 0.26, up));
  let ab = ell(q - ac, ar, e / s);
  let band = smoothstep(0.25, 0.6, sin((q.y - ac.y) / ar.y * 7.0 + 1.0) * 0.5 + 0.5);
  let side = clamp(1.0 - abs(q.x) / ar.x, 0.0, 1.0);
  var acol = mix(BODY, AMBER * (0.6 + 0.4 * h), band) * (0.55 + 0.6 * side);
  // lifted, its golden underside and the tip catch the evening light
  acol = mix(acol, GOLD * (0.8 + 0.5 * side), up * (0.55 + 0.4 * band)) + GOLD * f.z * 0.6 * side;
  let th = ell(q - vec2<f32>(0.0, -0.12), vec2<f32>(0.21, 0.2), e / s);
  let hd = ell(q - vec2<f32>(0.0, -0.38), vec2<f32>(0.15, 0.12), e / s);
  // the wings, folded over the abdomen; opened out and glinting in a flip
  let wa = 0.25 + up * 0.75;
  let wl = ell(rot(q - vec2<f32>(-0.08, 0.05), -wa) - vec2<f32>(0.0, 0.26), vec2<f32>(0.09, 0.3), e / s);
  let wr = ell(rot(q - vec2<f32>(0.08, 0.05), wa) - vec2<f32>(0.0, 0.26), vec2<f32>(0.09, 0.3), e / s);
  let wing = max(wl, wr);
  var col = acol;
  var a = ab;
  let fur = 0.7 + 0.6 * hash2(floor(q * 18.0) + k);
  col = mix(col, FUR * fur * (0.8 + 0.4 * up), th);
  a = max(a, th);
  col = mix(col, BODY * 0.6, hd);
  a = max(a, hd);
  let glint = vec3<f32>(0.55, 0.55, 0.5) * (0.25 + 1.1 * up) + GOLD * f.z * 0.4;
  col = mix(col, col * 0.6 + glint, wing * (0.35 + 0.35 * up));
  a = max(a, wing * 0.6);
  // the shadow it casts on the bees under it, lower and to the left
  let sh = ell(q - ac - vec2<f32>(-0.06, 0.12 + 0.1 * up), ar * 1.05, 0.12);
  let s2 = sh * 0.45 * (1.0 - a);
  let alpha = a + s2;
  return vec4<f32>(col * a / max(alpha, 1e-4), alpha);
}

fn hornet(world: vec2<f32>, e: f32) -> vec4<f32> {
  let p = u.p0.yz;
  let q = rot(world - p, u.p0.w) / 2.0;
  let ab = ell(q - vec2<f32>(0.0, 0.45), vec2<f32>(0.24, 0.5), e / 2.0);
  let bands = step(0.5, fract((q.y - 0.1) * 3.2));
  var col = mix(vec3<f32>(0.08, 0.04, 0.02), vec3<f32>(0.85, 0.55, 0.08), bands * step(0.25, q.y));
  col = mix(col, vec3<f32>(0.85, 0.6, 0.12), step(0.75, q.y));
  var a = ab;
  let th = ell(q - vec2<f32>(0.0, -0.08), vec2<f32>(0.22, 0.24), e / 2.0);
  col = mix(col, vec3<f32>(0.22, 0.10, 0.04), th);
  a = max(a, th);
  let hd = ell(q - vec2<f32>(0.0, -0.4), vec2<f32>(0.2, 0.15), e / 2.0);
  col = mix(col, vec3<f32>(0.88, 0.52, 0.12), hd);
  a = max(a, hd);
  // the wings, a blur in their beat
  let beat = 0.65 + 0.35 * sin(u.time * 90.0);
  let wl = ell(rot(q - vec2<f32>(-0.12, -0.05), -1.1 * beat) - vec2<f32>(0.0, 0.42), vec2<f32>(0.16, 0.45), 0.05);
  let wr = ell(rot(q - vec2<f32>(0.12, -0.05), 1.1 * beat) - vec2<f32>(0.0, 0.42), vec2<f32>(0.16, 0.45), 0.05);
  let wing = max(wl, wr) * 0.35;
  col = mix(col, vec3<f32>(0.75, 0.75, 0.7), wing * (1.0 - a));
  a = max(a, wing);
  return vec4<f32>(col, a);
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let cell = u.p0.x;
  let e = 1.2 / u.scale;
  let t = u.time;
  // behind: the forest below the cliff, dim and out of focus
  let blur = noise2(world * 0.05 + vec2<f32>(t * 0.01, 0.0)) * 0.7 + noise2(world * 0.13) * 0.3;
  var col = SKY * (0.6 + 0.8 * blur) + vec3<f32>(0.25, 0.2, 0.08) * smoothstep(0.6, 0.95, noise2(world * 0.09)) * 0.25;
  // the comb's wax, where it shows at the curtain's edge
  let shp = u.p1;
  let qq = (world - vec2<f32>(shp.x, shp.y)) / shp.zw;
  let inside = 1.0 - smoothstep(0.97, 1.02, length(qq));
  col = mix(col, WAX * (0.5 + 0.3 * noise2(world * 2.0)) * 0.5, inside * step(shp.y, world.y));
  // the bees, upper rows first, the lower overlapping them
  let k0 = floor(world / cell);
  for (var dy = -1; dy <= 1; dy = dy + 1) {
    for (var dx = -1; dx <= 1; dx = dx + 1) {
      let k = k0 + vec2<f32>(f32(dx), f32(dy));
      if (k.x < 0.0 || k.y < 0.0 || k.x >= u.grid.x || k.y >= u.grid.y) { continue; }
      let b = bee(world, k, e);
      col = mix(col, b.rgb, b.a);
    }
  }
  // the shimmer's glow: the light off a wave's lifted bodies and wings
  let c = world / cell;
  var glow = fld(c).z * 0.5;
  for (var i = 0; i < 6; i = i + 1) {
    let a = f32(i) * 1.0472;
    glow = glow + fld(c + vec2<f32>(cos(a), sin(a)) * 1.6).z * 0.08;
  }
  col = col + GOLD * glow * 0.22;
  // the overhang: dark rock along the top, a lit lip
  let lip = shp.y + (noise2(vec2<f32>(world.x * 0.15, 0.0)) - 0.5) * 3.0 + 0.8;
  let rock = 1.0 - smoothstep(lip - 0.2, lip + 0.2, world.y);
  let grain = noise2(world * vec2<f32>(0.3, 1.2)) * 0.6 + noise2(world * 1.7) * 0.4;
  col = mix(col, ROCK * (0.6 + 0.7 * grain) + vec3<f32>(0.2, 0.15, 0.08) * smoothstep(lip - 1.5, lip, world.y) * 0.5, rock);
  // the overhang's shade on the top of the comb
  col = col * (0.7 + 0.5 * smoothstep(lip, lip + 18.0, world.y));
  // the hornet's shadow on the bees, then the hornet
  let hs = ell(world - u.p0.yz - vec2<f32>(-1.4, 2.2), vec2<f32>(1.0, 1.4), 0.9);
  col = col * (1.0 - hs * 0.45 * inside);
  let hn = hornet(world, e);
  col = mix(col, hn.rgb, hn.a);
  let q = px / u.size - 0.5;
  col = col * (1.0 - 0.45 * dot(q, q));
  return vec4<f32>(col / (1.0 + col * 0.15), 1.0);
}
