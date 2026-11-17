// The NamibRand from a drone, low morning sun: red Namib sand, golden tussocks of Stipagrostis, and the
// bare discs in them, each ringed by taller grass. A field (toy-kit field.rs): `world` is in cells; per cell
// (grass as a share of the uniform sward's, the finger's shower, soil water, 0); u.p0 = (sun x, sun y, 0, 0).

fn grass(c: vec2<f32>) -> f32 { return fld(c).x; }
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let c = world;
  let f = fld(c);
  let g = f.x;
  let wet = clamp(f.y * 0.6, 0.0, 1.0);
  // tussocks: the grass is clumps, more of them (and taller) where the field says more grass
  let fine = noise2(world * 2.3) * 0.6 + noise2(world * 5.1) * 0.4;
  let cover = smoothstep(0.55, 1.0, g);
  let tuft = smoothstep(0.55, 0.45, fine - cover * 0.75 + 0.25);
  let tall = clamp(g - 0.9, 0.0, 1.0);
  // the ground: red sand with a few scales of mottle, darker where the shower wet it
  let m = noise2(world * 0.07) * 0.5 + noise2(world * 0.31) * 0.3 + noise2(world * 1.7) * 0.2;
  var sand = vec3<f32>(0.80, 0.42, 0.24) * (0.88 + 0.2 * m);
  sand = mix(sand, sand * vec3<f32>(0.62, 0.55, 0.55), wet);
  // the grass: straw gold, paler where tall, greening where it has just rained
  var straw = mix(vec3<f32>(0.80, 0.66, 0.40), vec3<f32>(0.93, 0.82, 0.56), tall);
  straw = straw * (0.85 + 0.25 * noise2(world * 0.9));
  straw = mix(straw, vec3<f32>(0.55, 0.62, 0.30), wet * 0.7);
  // sun from the upper left: the tussocks' (and the taller ring's) shadows fall to the lower right
  let sun = normalize(u.p0.xy + vec2<f32>(1e-4, 0.0));
  let e = 0.9;
  let gx = (grass(c + vec2<f32>(e, 0.0)) - grass(c - vec2<f32>(e, 0.0))) / (2.0 * e);
  let gy = (grass(c + vec2<f32>(0.0, e)) - grass(c - vec2<f32>(0.0, e))) / (2.0 * e);
  let lit = clamp(1.0 + 0.9 * dot(vec2<f32>(gx, gy), sun), 0.6, 1.25);
  let behind = noise2((world + sun * 0.35) * 2.3) * 0.6 + noise2((world + sun * 0.35) * 5.1) * 0.4;
  let shadowed = smoothstep(0.55, 0.45, behind - cover * 0.75 + 0.25) * (1.0 - tuft);
  let ground = sand * (1.0 - 0.35 * shadowed * cover);
  let col = mix(ground, straw * (0.9 + 0.2 * fine), tuft * (0.45 + 0.55 * cover)) * lit;
  return vec4<f32>(col, 1.0);
}
