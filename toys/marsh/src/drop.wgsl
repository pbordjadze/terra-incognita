// A drop of marsh water under a dark-field microscope: the slide black, the drop's water a faint haze, its
// edge (the meniscus) a ring of scattered light, and still debris on the glass: silt specks and a few
// diatoms' glassy shells. A backdrop (toy-kit backdrop.rs); px in pixels; u.p0 = (drop centre x, y,
// radius, pixels a point), u.p1.x = the field's strength across the glass (0..1), lighting the haze a little.

fn speck(q: vec2<f32>, cellsz: f32, keep: f32) -> f32 {
  let g = floor(q / cellsz);
  let h = hash2(g);
  if (h > keep) { return 0.0; }
  let o = vec2<f32>(hash2(g + 7.1), hash2(g + 3.7)) * cellsz;
  let d = length(q - g * cellsz - o);
  let r = 0.6 + 1.6 * hash2(g + 1.3);
  return exp(-d * d / (r * r)) * (0.3 + 0.7 * hash2(g + 9.2));
}

fn diatom(q: vec2<f32>, at: vec2<f32>, ang: f32, len: f32) -> f32 {
  let c = cos(ang); let s = sin(ang);
  let p = q - at;
  let l = vec2<f32>(c * p.x + s * p.y, -s * p.x + c * p.y);
  let e = length(l / vec2<f32>(len, len * 0.24));
  let rim = exp(-pow((e - 1.0) * len * 0.35, 2.0));
  let ribs = 0.5 + 0.5 * cos(l.x / len * 40.0);
  return rim * 0.8 + step(e, 1.0) * ribs * 0.06;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let c = u.p0.xy;
  let r = u.p0.z;
  let k = u.p0.w;
  let q = (px - c) / k;              // points from the drop's centre
  let rr = length(px - c);
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
  // a few diatoms
  var d = 0.0;
  d += diatom(q, vec2<f32>(-92.0, -150.0), 0.6, 26.0);
  d += diatom(q, vec2<f32>(110.0, 60.0), -0.9, 34.0);
  d += diatom(q, vec2<f32>(-40.0, 140.0), 2.1, 20.0);
  d += diatom(q, vec2<f32>(60.0, -40.0), 1.4, 14.0);
  col += vec3<f32>(0.45, 0.62, 0.55) * d * 0.12 * inside;
  return vec4<f32>(col, 1.0);
}
