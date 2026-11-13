// The barchan field from above, a low morning sun from the east: gold sand on its windward backs,
// the slip faces in shade, long shadows on the grey-brown reg between the dunes, and ripples on the
// sand across the wind. A field (toy-kit field.rs): `world` is in cells; per cell (height in cells,
// 0, 0, 0). u.p0 = (wind x, wind y, 0, 0).

// the shape: a slab or two of sand in transit lies flat on the reg (no relief of its own)
fn height(c: vec2<f32>) -> f32 { return max(fld(c).x - 0.22, 0.0); }
// the same, softened over a cell or so: the slabs' steps are not the sand's shape
fn smooth_h(c: vec2<f32>) -> f32 {
  return (height(c) * 2.0 + height(c + vec2<f32>(1.2, 0.0)) + height(c - vec2<f32>(1.2, 0.0)) + height(c + vec2<f32>(0.0, 1.2)) + height(c - vec2<f32>(0.0, 1.2))) / 6.0;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let c = world;
  let h = height(c);
  let e = 1.3;
  let hx = (smooth_h(c + vec2<f32>(e, 0.0)) - smooth_h(c - vec2<f32>(e, 0.0))) / (2.0 * e);
  let hy = (smooth_h(c + vec2<f32>(0.0, e)) - smooth_h(c - vec2<f32>(0.0, e))) / (2.0 * e);
  let n = normalize(vec3<f32>(-hx, -hy, 1.0));
  // the sun: low in the east (screen right, a little toward the top), 22° up
  let sun = normalize(vec3<f32>(0.86, -0.35, 0.42));
  var lit = max(dot(n, sun), 0.0);
  // its shadow: walk toward the sun over the heights
  var shade = 1.0;
  let step = normalize(sun.xy);
  let rise = sun.z / length(sun.xy);
  for (var i = 1; i < 28; i = i + 1) {
    let d = f32(i) * 1.6;
    let hs = height(c + step * d);
    if (hs > h + d * rise + 0.05) { shade = 0.0; break; }
  }
  let sand = smoothstep(0.03, 0.6, h);
  // thin sand in transit: a faint warm streak on the reg
  let thin = smoothstep(0.05, 0.25, fld(c).x) * (1.0 - sand) * 0.35;
  // the reg: gravel and hard ground, grey-brown, faintly mottled
  let g = noise2(world * 0.11) * 0.6 + noise2(world * 0.53) * 0.4;
  let reg = vec3<f32>(0.56, 0.48, 0.40) * (0.88 + 0.2 * g) * (0.9 + 0.1 * noise2(world * 2.7));
  // the sand: warm gold, deeper where it is thick
  let sc = mix(vec3<f32>(0.90, 0.66, 0.42), vec3<f32>(0.93, 0.62, 0.34), clamp(h / 5.0, 0.0, 1.0));
  // ripples across the wind, ~a cell apart, on the sand only, fading on steep faces
  let wind = normalize(u.p0.xy + vec2<f32>(1e-4, 0.0));
  let along = dot(world, wind);
  let across = dot(world, vec2<f32>(-wind.y, wind.x));
  let rip = sin(along * 4.2 + 1.2 * noise2(vec2<f32>(across * 0.35, along * 0.1))) * (1.0 - smoothstep(0.25, 0.6, length(vec2<f32>(hx, hy))));
  var base = mix(mix(reg, sc, thin), sc, sand);
  base = base * (1.0 + 0.05 * rip * sand);
  let sky = vec3<f32>(0.42, 0.50, 0.66);
  let warm = vec3<f32>(1.0, 0.92, 0.80);
  let flat_lit = dot(vec3<f32>(0.0, 0.0, 1.0), sun);
  // a flat floor reads as itself; the faces toward the sun brighter, away darker, shadows blue
  let direct = lit / flat_lit * shade;
  var col = base * (warm * direct * 0.82 + sky * 0.30);
  return vec4<f32>(col, 1.0);
}
