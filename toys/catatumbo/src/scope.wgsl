// Catatumbo on an oscilloscope: the storm drawn as traces of light, as Mary Ellen Bute drew her films
// on a cathode-ray tube's face (Abstronic, New York, 1952; scope.rs has the method and its sources).
// The prelude (toy-kit film.rs) declares p, src, lin, vs_main and hash. p.a = (exposure; 0; 0; 0).
//
// `fs_surface`: the storm's light, one texel a cell, lifted (a square root) so the dark lake and the
// clouds have levels to trace.
// `fs_main`: the tube's face as filmed: a thin trace along each of LEVELS levels of that light, a beam
// that is calligraphic, drawn along lines, not scanned; each level through its own colour filter, the
// films printed over one another (adding light), the brightest levels white-hot, each trace's glow in
// the emulsion round it.

const LEVELS: f32 = 9.0;

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
  let h = p.cell * 0.75;
  let v = level_at(fc.xy);
  let gx = (level_at(fc.xy + vec2<f32>(h, 0.0)) - level_at(fc.xy - vec2<f32>(h, 0.0))) / (2.0 * h);
  let gy = (level_at(fc.xy + vec2<f32>(0.0, h)) - level_at(fc.xy - vec2<f32>(0.0, h))) / (2.0 * h);
  let g = max(length(vec2<f32>(gx, gy)) * LEVELS, 1e-4);
  // distance in pixels to the nearest level's line
  let x = v * LEVELS;
  let k = round(x);
  let d = abs(x - k) / g;
  let px = p.size.x / 1179.0;
  // the beam: a hot thin core and its glow; where the light hardly changes (a level drawn flat)
  // the beam would sweep a whole field, so a trace fades there, as a slow sweep does
  let core = exp(-d * d / (0.9 * px * px));
  let glow = exp(-d / (6.0 * px)) * 0.22;
  let flat = smoothstep(0.0008, 0.004, g / LEVELS);
  var on = (core + glow) * flat * step(0.5, k);
  // the brightest levels: the beam turned up, white at the core
  let hot = smoothstep(LEVELS * 0.55, LEVELS * 0.9, k);
  let col = mix(tint(k), vec3<f32>(1.0, 0.97, 0.9), hot * core);
  // where the light is brightest (the bolt itself) the beam rides the ridge, one unbroken stroke,
  // rather than tracing rings round it
  let ridge = smoothstep(0.90, 0.985, v);
  // the tube's face: faint phosphor grain, and the film's grain over everything
  let grain = 0.9 + 0.2 * hash(floor(fc.xy * 0.5) + p.drawing * 0.37);
  let base = vec3<f32>(0.010, 0.012, 0.022) * (0.8 + 0.4 * hash(floor(fc.xy / 3.0) + p.drawing));
  let beam = vec3<f32>(1.0, 0.96, 0.88) * ridge * 1.3;
  return vec4<f32>(base + (col * on * (0.55 + 0.9 * hot) + beam) * grain, 1.0);
}
