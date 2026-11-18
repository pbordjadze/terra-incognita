// A drop of marsh water under a dark-field microscope: the slide black, the drop's water a faint haze, its
// edge (the meniscus) a ring of scattered light, and still silt specks on the glass. A backdrop (toy-kit
// backdrop.rs); px in pixels; u.p0 = (drop centre x, y, half-width, pixels a point), u.p1 = (the field's
// strength across the glass 0..1, which lights the haze a little; the half-height).

fn speck(q: vec2<f32>, cellsz: f32, keep: f32) -> f32 {
  let g = floor(q / cellsz);
  let h = hash2(g);
  if (h > keep) { return 0.0; }
  let o = vec2<f32>(hash2(g + 7.1), hash2(g + 3.7)) * cellsz;
  let d = length(q - g * cellsz - o);
  let r = 0.6 + 1.6 * hash2(g + 1.3);
  return exp(-d * d / (r * r)) * (0.3 + 0.7 * hash2(g + 9.2));
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let c = u.p0.xy;
  let r = u.p0.z;
  let k = u.p0.w;
  let q = (px - c) / k;              // points from the drop's centre
  // the drop is an oval, half-width r and half-height u.p1.y (pixels): rr its radius scaled to the width
  let rr = length((px - c) / vec2<f32>(r, u.p1.y)) * r;
  let inside = smoothstep(r + 1.5 * k, r - 1.5 * k, rr);
  // the water: a faint haze, brighter toward the edge where the meniscus curves the light
  let haze = (0.015 + 0.03 * noise2(q * 0.02 + vec2<f32>(u.time * 0.01, 0.0)) + 0.04 * pow(rr / r, 6.0)) * inside;
  var col = vec3<f32>(0.32, 0.42, 0.55) * haze * (1.0 + 0.4 * u.p1.x);
  // the meniscus: a bright thin ring, a softer glow inside it
  let edge = (rr - r) / k;
  col += vec3<f32>(0.55, 0.62, 0.7) * (exp(-edge * edge / 2.5) * 0.55 + exp(-pow(edge + 5.0, 2.0) / 40.0) * 0.12);
  // silt on the glass
  let s = speck(q, 9.0, 0.22) + speck(q + 40.0, 23.0, 0.35) * 1.4;
  col += vec3<f32>(0.6, 0.52, 0.4) * s * 0.35 * inside;
  return vec4<f32>(col, 1.0);
}
