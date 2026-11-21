// The Wave drawn as a BEFLIX film (Ken Knowlton, Bell Labs, 1963; beflix.rs has the method and its
// sources). Two passes over the stadium as drawn:
//
// `fs_surface`: the surface, one texel a cell, each holding one of eight values (0..7) as BEFLIX's
// did: the cell's light, read from the drawing beneath it.
// `fs_main`: each cell drawn as the character for its value, as the SC-4020 drew a cell through its
// charactron mask with a defocused beam: soft dots of light on black film, a halo where the film
// took more light than it could hold, grain, the gate's weave, and in some shots the colour the
// optical printer dyed in afterwards.

// The prelude (toy-kit film.rs) declares p, src, lin, vs_main and hash; p.a = (shot: 0 black and white,
// 1 and 2 the optical printer's colours; weave: 1, or 0 with Reduce Motion).

// ---- the surface ----
@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let c = floor(fc.xy);
  var sum = 0.0;
  var hi = 0.0;
  for (var j = 0; j < 3; j++) {
    for (var i = 0; i < 3; i++) {
      // a little past the cell's own edges, so a far wave, a thread of light, still fills whole cells
      let q = (c + (vec2<f32>(f32(i), f32(j)) + 0.5) / 3.0 * 1.8 - 0.4) * p.cell;
      let s = textureLoad(src, vec2<i32>(clamp(q, vec2<f32>(0.0), p.size - 1.0)), 0).rgb;
      let l = dot(s, vec3<f32>(0.3, 0.55, 0.15));
      sum += l;
      hi = max(hi, l);
    }
  }
  // the night's few lights carry the picture: lifted hard, so a seated crowd is the low greys and a
  // standing one the top of the scale
  var l = mix(sum / 9.0, hi, 0.3);
  // the night: a few points of light in the black, as the films' starfields were typed
  let star = hash(c + vec2<f32>(0.37, 0.11));
  if (l < 0.02 && star > 0.994) {
    l = 0.06 + 0.1 * hash(c + vec2<f32>(floor(p.drawing / 6.0), 3.3));
  }
  let v = clamp(floor(pow(max(l - 0.035, 0.0) * 1.9, 0.7) * 7.0 + 0.5), 0.0, 7.0);
  return vec4<f32>(v / 7.0, 0.0, 0.0, 1.0);
}

// ---- the film ----
// The eight characters, 5 dots by 7, a row a word (the top row first, the left dot the high bit):
// none . : + X # @ and the full cell (the beam left on), each holding more light than the last.
const GLYPHS = array<u32, 56>(
  0u, 0u, 0u, 0u, 0u, 0u, 0u,
  0u, 0u, 0u, 0u, 0u, 0u, 4u,
  0u, 0u, 4u, 0u, 0u, 4u, 0u,
  0u, 4u, 4u, 31u, 4u, 4u, 0u,
  17u, 17u, 10u, 4u, 10u, 17u, 17u,
  10u, 10u, 31u, 10u, 31u, 10u, 10u,
  31u, 17u, 23u, 21u, 23u, 16u, 31u,
  31u, 31u, 31u, 31u, 31u, 31u, 31u,
);

fn dot_on(g: i32, d: vec2<i32>) -> f32 {
  if (d.x < 0 || d.x > 4 || d.y < 0 || d.y > 6) { return 0.0; }
  return f32((GLYPHS[g * 7 + d.y] >> u32(4 - d.x)) & 1u);
}

fn value(c: vec2<i32>) -> i32 {
  let cc = clamp(c, vec2<i32>(0), vec2<i32>(p.grid) - 1);
  return i32(textureLoad(src, cc, 0).r * 7.0 + 0.5);
}

@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let d = p.drawing;
  // the gate's weave: the whole frame a little off, a different way each drawing
  let wv = (vec2<f32>(hash(vec2<f32>(fract(d * 0.0173) * 61.0, 1.3)), hash(vec2<f32>(fract(d * 0.0291) * 53.0, 7.1))) - 0.5) * p.cell * 0.45 * p.a.y;
  let px = fc.xy + wv;
  let cf = px / p.cell;
  let c = vec2<i32>(floor(cf));
  let g = value(c);
  // the cell's dots, soft: the beam was defocused, so each dot is a little round blur
  let q = (cf - floor(cf)) * vec2<f32>(6.0, 8.0) - vec2<f32>(0.5, 0.5);
  let n = vec2<i32>(floor(q + 0.5));
  var ink = 0.0;
  for (var j = -1; j <= 1; j++) {
    for (var i = -1; i <= 1; i++) {
      let k = n + vec2<i32>(i, j);
      let r = length(q - vec2<f32>(k));
      ink += dot_on(g, k) * exp(-r * r * 2.6);
    }
  }
  // the halo: where the film was flooded, light spread past its cell
  let halo = textureSampleLevel(src, lin, cf / p.grid, 0.0).r;
  // the full cell is the beam left on: solid, not dots
  ink = select(ink, 1.0, g == 7);
  var lum = clamp(ink, 0.0, 1.0) * 0.95 + halo * halo * 0.2;
  // the grain, new each drawing
  // (the drawing folded small first: a large argument to sin loses its precision on the phone)
  lum += (hash(floor(fc.xy) + vec2<f32>(fract(d * 0.1373) * 97.0, fract(d * 0.0311) * 89.0)) - 0.5) * 0.07;
  lum = clamp(lum, 0.0, 1.0);
  var rgb = vec3<f32>(lum) * vec3<f32>(1.0, 0.97, 0.9);
  if (p.a.x > 0.5) {
    // dyed in afterwards, one colour for the dim and another for the bright
    let k = clamp(lum * 1.4, 0.0, 1.0);
    let a = select(vec3<f32>(0.95, 0.15, 0.55), vec3<f32>(1.0, 0.30, 0.08), p.a.x > 1.5);
    let b = select(vec3<f32>(0.30, 0.95, 1.00), vec3<f32>(1.0, 0.92, 0.35), p.a.x > 1.5);
    rgb = lum * mix(a, b, k) * 1.2 + vec3<f32>(0.025, 0.0, 0.04);
  }
  return vec4<f32>(rgb + vec3<f32>(0.018, 0.016, 0.016), 1.0);
}
