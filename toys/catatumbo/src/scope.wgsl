// Catatumbo on an oscilloscope: the storm drawn as traces of light, as Mary Ellen Bute drew her films
// on a cathode-ray tube's face (Abstronic, New York, 1952; scope.rs has the method and its sources).
// The prelude (toy-kit film.rs) declares p, src, lin, vs_main and hash. p.a = (exposure; 0; 0; 0).
//
// `fs_surface`: the storm's light, one texel a cell, lifted (a square root) so the dark lake and the
// clouds have levels to trace.
// `fs_main`: the tube's face as filmed: a thin trace along each of LEVELS levels of the sky's and
// lake's light, a beam that is calligraphic, drawn along lines, not scanned; each level through its own
// colour filter, each trace's glow in the emulsion round it. The bolts are not traced from the picture:
// lib.rs draws them over the trace, each one stroke of the beam turned up.

const LEVELS: f32 = 5.0;

fn light(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.30, 0.55, 0.15)); }

@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let c = floor(fc.xy);
  var sum = 0.0;
  for (var j = 0; j < 2; j++) {
    for (var i = 0; i < 2; i++) {
      let q = (c + (vec2<f32>(f32(i), f32(j)) + 0.5) / 2.0) * p.cell;
      sum += light(textureLoad(src, vec2<i32>(clamp(q, vec2<f32>(0.0), p.size - 1.0)), 0).rgb);
    }
  }
  let v = sqrt(clamp(sum * 0.25 * p.a.x, 0.0, 1.0));
  return vec4<f32>(v, 0.0, 0.0, 1.0);
}

fn level_at(q: vec2<f32>) -> f32 {
  return textureSampleLevel(src, lin, q / p.size, 0.0).r;
}

// the filter each level was printed through: the lake's levels blue and green, the clouds' violet and
// rose, the bolt's amber, then white where the beam burns
fn tint(k: f32) -> vec3<f32> {
  let u = k / LEVELS;
  let a = vec3<f32>(0.20, 0.45, 1.00);
  let b = vec3<f32>(0.15, 0.95, 0.55);
  let c = vec3<f32>(0.75, 0.35, 1.00);
  let d = vec3<f32>(1.00, 0.40, 0.55);
  let e = vec3<f32>(1.00, 0.80, 0.35);
  if (u < 0.25) { return mix(a, b, u / 0.25); }
  if (u < 0.5) { return mix(b, c, (u - 0.25) / 0.25); }
  if (u < 0.75) { return mix(c, d, (u - 0.5) / 0.25); }
  return mix(d, e, (u - 0.75) / 0.25);
}

@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let px = p.size.x / 1179.0;
  let h = p.cell * 0.75;
  let v = level_at(fc.xy);
  let gx = (level_at(fc.xy + vec2<f32>(h, 0.0)) - level_at(fc.xy - vec2<f32>(h, 0.0))) / (2.0 * h);
  let gy = (level_at(fc.xy + vec2<f32>(0.0, h)) - level_at(fc.xy - vec2<f32>(0.0, h))) / (2.0 * h);
  let g = max(length(vec2<f32>(gx, gy)) * LEVELS, 1e-4);
  // the clouds and the lake: a few levels of their light, each one thin trace
  let x = v * LEVELS;
  let k = round(x);
  let d = abs(x - k) / g;
  // where the light hardly changes the beam would sweep a whole field: a trace fades there, as a
  // slow sweep does
  let flat = smoothstep(0.003, 0.010, g / LEVELS);
  let lit = step(0.5, k);
  let core = exp(-d * d / (0.8 * px * px));
  let glow = exp(-d / (5.0 * px)) * 0.18;
  let trace = (core + glow) * flat * lit;
  // the tube's face: faint phosphor grain, and the film's grain over everything
  let grain = 0.9 + 0.2 * hash(floor(fc.xy * 0.5) + p.drawing * 0.37);
  let base = vec3<f32>(0.010, 0.012, 0.022) * (0.8 + 0.4 * hash(floor(fc.xy / 3.0) + p.drawing));
  return vec4<f32>(base + (tint(k) * trace * 1.4) * grain, 1.0);
}
