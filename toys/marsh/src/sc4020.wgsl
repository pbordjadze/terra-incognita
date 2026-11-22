// The SC 4020's film (sc4020.rs). fs_surface: the tube face's light at each addressable position.
// fs_main: the frame as projected: shifted in the gate by the drawing's jitter, the lines' halation, the
// black-and-white stock's curve and grain. p.a.x: jitter on (1) or off (0).

@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  // the brightest of the pixels under this position: a vector is never lost between positions
  let c = vec2<i32>(fc.xy * p.cell);
  let n = max(i32(ceil(p.cell)), 1);
  var v = 0.0;
  for (var j = 0; j < n; j++) {
    for (var i = 0; i < n; i++) {
      let s = textureLoad(src, c + vec2<i32>(i, j), 0).rgb;
      v = max(v, dot(s, vec3<f32>(0.3, 0.5, 0.2)));
    }
  }
  return vec4<f32>(v, 0.0, 0.0, 1.0);
}

fn face(q: vec2<f32>) -> f32 {
  return textureSampleLevel(src, lin, q / p.size, 0.0).r;
}

@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let d = p.drawing;
  // the frame in the gate: off by a little, differently every frame (no pin registration)
  let off = (vec2<f32>(hash(vec2<f32>(d, 1.7)), hash(vec2<f32>(d, 8.3))) - 0.5) * p.cell * 2.4 * p.a.x;
  let q = fc.xy + off;
  let line = face(q);
  // halation: light spread in the emulsion round each bright line
  var halo = 0.0;
  for (var i = 0; i < 8; i++) {
    let a = f32(i) * 0.785 + 0.39;
    halo += face(q + vec2<f32>(cos(a), sin(a)) * p.cell * 1.6);
  }
  halo /= 8.0;
  // the stock: a hard toe (the black stays black), a shoulder the brightest lines run into
  let e = line * 1.35 + halo * 0.25;
  var v = 1.0 - exp(-e * 2.2);
  // grain, a new clump every frame, finer than a line
  let g = hash(floor(fc.xy / (p.cell * 1.3)) + vec2<f32>(d * 0.37, d * 0.11)) - 0.5;
  v = clamp(v + g * (0.05 + 0.10 * v), 0.0, 1.0);
  // the base: never quite black; the lens a little darker to the corners
  let r = length(fc.xy / p.size - vec2<f32>(0.5, 0.5));
  v = 0.018 + v * (1.0 - 0.35 * r * r);
  return vec4<f32>(vec3<f32>(v) * vec3<f32>(0.97, 0.98, 1.0), 1.0);
}
