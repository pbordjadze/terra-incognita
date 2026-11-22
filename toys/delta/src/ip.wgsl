// The Sandin Image Processor (ip.rs). fs_surface: the black-and-white camera's signal at each picture
// element. fs_main: the video as played: the lines' time-base wobble, the Amplitude Classifier's eight
// levels each keyed to a colour, the Differentiator's horizontal edges, chroma that lags the signal, the
// raster's lines and the tape's snow. p.a = (seconds, motion on 1 / off 0).

@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  // the camera: its tube's blur over the picture element, luminance only
  let c = fc.xy * p.cell;
  var v = 0.0;
  for (var j = 0; j < 3; j++) {
    for (var i = 0; i < 3; i++) {
      let s = textureLoad(src, vec2<i32>(c + (vec2<f32>(f32(i), f32(j)) - 1.0) * p.cell * 0.4), 0).rgb;
      v += dot(s, vec3<f32>(0.30, 0.59, 0.11));
    }
  }
  return vec4<f32>(v / 9.0, 0.0, 0.0, 1.0);
}

fn signal(q: vec2<f32>) -> f32 {
  return textureSampleLevel(src, lin, q / p.size, 0.0).r;
}

// The keys: the colour wired to each of the eight levels of the inverted signal (the processor's
// inverter before the classifier, so the dark water is the bright end). The dry sand gets the three
// lowest, kept dark and quiet; the wet grass a deep violet; the water's wet fringe magenta, then cyan,
// and white where it runs deep; the darkest of all, the trees on their islands, a dark green.
fn key(n: i32) -> vec3<f32> {
  switch (n) {
    case 0: { return vec3<f32>(0.01, 0.01, 0.03); }
    case 1: { return vec3<f32>(0.03, 0.02, 0.10); }
    case 2: { return vec3<f32>(0.06, 0.03, 0.18); }
    case 3: { return vec3<f32>(0.25, 0.05, 0.45); }
    case 4: { return vec3<f32>(0.95, 0.10, 0.60); }
    case 5: { return vec3<f32>(0.10, 0.90, 0.95); }
    case 6: { return vec3<f32>(0.92, 0.98, 1.00); }
    default: { return vec3<f32>(0.05, 0.30, 0.12); }
  }
}

// The classifier's level for a signal: inverted, the gain spanning the land's range; the threshold
// drifts a little, as a knob does.
fn level(s: f32) -> i32 {
  let drift = 0.03 * sin(p.a.x * 0.13) * p.a.y;
  return i32(floor(clamp((0.78 + drift - s) / 0.50 * 8.0, 0.0, 7.0)));
}

@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let t = p.a.x;
  let line = floor(fc.y / p.cell);
  // the time base: each line a little early or late, a slow wave down the picture
  let wob = (sin(line * 0.21 + t * 1.7) * 0.35 + (hash(vec2<f32>(line, floor(t * 30.0))) - 0.5) * 0.3) * p.cell * p.a.y;
  let q = vec2<f32>(fc.x + wob, (line + 0.5) * p.cell);
  // the signal along the line, its bandwidth a few elements
  let h = p.cell;
  let s = signal(q) * 0.5 + (signal(q - vec2<f32>(h, 0.0)) + signal(q + vec2<f32>(h, 0.0))) * 0.25;
  var n = level(s);
  // a second keyer on level 4: magenta only where the water is within a few elements (the flood's wet
  // fringe); elsewhere that level takes the grass's violet
  if (n == 4) {
    var wet = false;
    for (var j = 0; j < 8; j++) {
      let a = f32(j) * 0.785;
      if (level(signal(q + vec2<f32>(cos(a), sin(a)) * h * 3.0)) >= 5) { wet = true; }
    }
    if (!wet) { n = 3; }
  }
  // the colour encoder's chroma lags the signal by an element or two and is softer
  var nc = level(signal(q - vec2<f32>(1.6 * h, 0.0)));
  if (nc == 4 && n != 4) { nc = n; }
  var col = mix(key(n), key(nc), 0.35);
  // the differentiator: a rise of the (inverted) signal along the line lights white, on the horizontal edges only
  let d = signal(q - vec2<f32>(h * 1.5, 0.0)) - s;
  col += vec3<f32>(1.0) * smoothstep(0.03, 0.10, d) * 0.55;
  // the raster: lines with dark between them
  let y = fract(fc.y / p.cell);
  col *= 0.62 + 0.38 * sin(y * 3.14159);
  // snow on the tape
  let snow = hash(vec2<f32>(floor(fc.x / 2.0), fc.y) + vec2<f32>(t * 61.0, t * 17.0) * p.a.y) - 0.5;
  col += snow * 0.07;
  return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
