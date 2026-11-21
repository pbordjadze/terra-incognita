// The barchan field from high above, a low sun from upwind (it turns with the wind, so the slip faces
// stay in shade), painted in ink-wash (`ink_wash`, below): the dunes' backs a pale wash, a dry-brush
// brink, the slip faces and their shadows a dark wash, the reg between bare paper. A field (toy-kit field.rs): `world` is in cells; per cell
// (height in cells, softened, 0, 0, 0); u.p0 = (wind x, wind y, 0, 0). From this high, the ripples are too fine to see.

// the shape: a slab or two of sand in transit lies flat on the reg (no relief of its own)
fn height(c: vec2<f32>) -> f32 { return max(fld(c).x - 0.3, 0.0); }
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let c = world;
  let h = height(c);
  let e = 0.8;
  let hx = (height(c + vec2<f32>(e, 0.0)) - height(c - vec2<f32>(e, 0.0))) / (2.0 * e);
  let hy = (height(c + vec2<f32>(0.0, e)) - height(c - vec2<f32>(0.0, e))) / (2.0 * e);
  let n = normalize(vec3<f32>(-hx, -hy, 1.0));
  // the sun low from upwind and a little to one side, so the slip faces always sit in shade
  let wind = normalize(u.p0.xy + vec2<f32>(1e-4, 0.0));
  let toward = -vec2<f32>(wind.x * 0.87 - wind.y * 0.5, wind.x * 0.5 + wind.y * 0.87);
  let sun = normalize(vec3<f32>(toward * 0.88, 0.46));
  let lit = max(dot(n, sun), 0.0);
  // its shadow: walk toward the sun over the heights
  // its shadow, soft-edged: walk toward the sun over the heights, keeping the closest call
  var shade = 1.0;
  let dir = normalize(sun.xy);
  let rise = sun.z / length(sun.xy);
  for (var i = 1; i < 32; i = i + 1) {
    let d = f32(i) * 1.1;
    let over = height(c + dir * d) - (h + d * rise);
    shade = min(shade, clamp(1.0 - over * 2.5 / (0.4 + d * 0.08), 0.0, 1.0));
  }
  let sand = smoothstep(0.02, 0.25, h);
  // the height softened over a few cells, for the washes
  var hs = 0.0;
  for (var j = -1; j <= 1; j = j + 1) {
    for (var i = -1; i <= 1; i = i + 1) {
      hs = hs + height(c + vec2<f32>(f32(i), f32(j)) * 1.1);
    }
  }
  return vec4<f32>(ink_wash(px, h, hs / 9.0, sand, lit / sun.z, shade), 1.0);
}

// The field painted as Shanghai ink-wash animation (Te Wei and Qian Jiajun's films, 1960 on; SOURCES.md):
// ink on warm paper, the dunes' backs a pale wash, their slip faces and shadows a dark one laid over it,
// each wash drying with a darker edge where its water stopped, the brinks a dry brush, and the
// pigment settling into the paper's tooth.
fn ink_wash(px: vec2<f32>, h: f32, hs: f32, sand: f32, direct: f32, shade: f32) -> vec3<f32> {
  // the paper: warm, mottled, with long fibres two ways
  let m = noise2(px * 0.004) * 0.6 + noise2(px * 0.017) * 0.4;
  let r1 = vec2<f32>(px.x * 0.87 + px.y * 0.5, px.y * 0.87 - px.x * 0.5);
  let r2 = vec2<f32>(px.x * 0.34 - px.y * 0.94, px.y * 0.34 + px.x * 0.94);
  let f1 = noise2(vec2<f32>(r1.x * 0.45, r1.y * 0.006)) * noise2(r1 * 0.01 + 5.0);
  let f2 = noise2(vec2<f32>(r2.x * 0.45, r2.y * 0.006) + 7.0) * noise2(r2 * 0.01 + 9.0);
  let fibre = smoothstep(0.62, 0.8, max(f1, f2));
  let paper = vec3<f32>(0.935, 0.905, 0.835) * (0.97 + 0.05 * m) - fibre * 0.012;
  // the washes, laid by height, not by light: a pale one on each dune's back, deepening toward the
  // crest; over it a dark one where the sun does not reach, thickest high on the slip face under the
  // brink and thinning down it and out across its shadow. (`hs` is the height softened over a few
  // cells, so the wash is one sweep, not the sand's every bump.)
  let back = smoothstep(0.0, 0.3, h) * (0.12 + 0.30 * pow(smoothstep(0.3, 8.0, hs), 1.3));
  let dark_x = clamp((1.0 - shade) * 1.1 + (0.45 - direct) * 0.8, 0.0, 1.0) * smoothstep(0.0, 0.08, h);
  let lee = smoothstep(0.15, 0.6, dark_x);
  // darkest at the crest, bleeding paler down the face and out over its shadow; tall dunes take the
  // heavier load, small ones stay ghost-pale
  let dark = lee * (0.06 + 0.85 * smoothstep(0.6, 6.0, h) * smoothstep(0.3, 1.0, dark_x));
  // where the dark wash's water stopped, its pigment pooled a little; the pale one bleeds out soft
  let pool = clamp((abs(dpdx(lee)) + abs(dpdy(lee))) * 0.6, 0.0, 0.1);
  // wet-into-wet: the washes bloom and thin across the sheet
  let wet = 0.8 + 0.4 * noise2(px * 0.009 + 3.0);
  var ink = 1.0 - (1.0 - back * wet) * (1.0 - dark * wet);
  ink = ink + pool * (1.0 - ink);
  // the brinks: a dry brush along the top of each slip face, broken where the brush ran dry
  let brink = clamp((abs(dpdx(shade)) + abs(dpdy(shade))) * 2.0, 0.0, 1.0) * smoothstep(1.0, 3.0, hs);
  let dry = smoothstep(0.35, 0.6, noise2(vec2<f32>(px.x * 0.9, px.y * 0.12)) * 0.6 + noise2(px * 0.05) * 0.4);
  ink = max(ink, brink * dry * 0.8);
  // the pigment settles into the paper's tooth
  ink *= 0.86 + 0.28 * noise2(px * 0.55);
  let col = mix(paper, vec3<f32>(0.075, 0.075, 0.085), clamp(ink, 0.0, 1.0));
  return seal(px, col);
}

// The painter's seal, low on the left (clear of the corner's mark), in vermilion: a square of strokes cut through it.
fn seal(px: vec2<f32>, col: vec3<f32>) -> vec3<f32> {
  let side = u.size.x * 0.08;
  let o = vec2<f32>(u.size.x * 0.08, u.size.y - u.size.x * 0.36);
  let q = (px - o) / side;
  if (q.x < 0.0 || q.y < 0.0 || q.x > 1.0 || q.y > 1.0) { return col; }
  let lines = abs(fract(q.x * 3.0 - 0.5) - 0.5) < 0.07 || abs(q.y - 0.5) < 0.05;
  let cut = lines && q.x > 0.16 && q.x < 0.84 && q.y > 0.16 && q.y < 0.84;
  let worn = noise2(px * 0.4) < 0.18 || noise2(px * 0.05) * (1.0 - min(min(q.x, q.y), min(1.0 - q.x, 1.0 - q.y)) * 6.0) > 0.6;
  if (cut || worn) { return col; }
  return mix(col, vec3<f32>(0.80, 0.14, 0.09), 0.93);
}
