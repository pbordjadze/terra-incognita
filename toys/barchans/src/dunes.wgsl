// The barchan field from high above, a low sun from upwind (it turns with the wind, so the slip faces
// stay in shade): bright sand on the dunes' windward backs, a sharp brink, the slip faces in shade and
// their shadows on the pale reg between. A field (toy-kit field.rs): `world` is in cells; per cell
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
  // the reg: gravel and hard ground, pale grey-tan, mottled at a few scales
  let g = noise2(world * 0.045) * 0.5 + noise2(world * 0.17) * 0.3 + noise2(world * 0.9) * 0.2;
  let reg = vec3<f32>(0.72, 0.64, 0.54) * (0.90 + 0.16 * g);
  // the sand: bright, warmer where it is deep
  let sc = mix(vec3<f32>(0.97, 0.86, 0.68), vec3<f32>(0.98, 0.80, 0.58), clamp(h / 8.0, 0.0, 1.0));
  let base = mix(reg, sc, sand);
  let sky = vec3<f32>(0.46, 0.54, 0.70);
  let warm = vec3<f32>(1.0, 0.95, 0.86);
  let direct = lit / sun.z * shade;
  let col = base * (warm * direct * 0.72 + sky * 0.50);
  return vec4<f32>(col, 1.0);
}
