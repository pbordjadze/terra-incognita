// Christmas Island's floor from above (crabs.rs): the rainforest's leaf litter and moss, a road across
// with the grate of an underpass, the limestone terrace and the sea's edge. World is metres from the top
// left. u.p0 = (daylight 0..1, sun 0..1, road top, road bottom); u.p1 = (shore, sea, gap's left, gap's right).

const LITTER = vec3<f32>(0.20, 0.15, 0.09);
const MOSS = vec3<f32>(0.13, 0.22, 0.08);
const ASPHALT = vec3<f32>(0.24, 0.24, 0.25);
const STONE = vec3<f32>(0.62, 0.60, 0.55);
const DEEP = vec3<f32>(0.02, 0.20, 0.32);
const SHALLOW = vec3<f32>(0.10, 0.48, 0.52);

fn forest(world: vec2<f32>) -> vec3<f32> {
  let big = noise2(world * 0.35);
  let leaves = noise2(world * 6.0) * 0.6 + noise2(world * 17.0) * 0.4;
  var c = mix(LITTER, MOSS, smoothstep(0.35, 0.75, big));
  c = c * (0.75 + 0.5 * leaves);
  // a fallen leaf here and there, paler
  let g = floor(world * 3.0);
  let h = hash2(g);
  let d = length(fract(world * 3.0) - vec2<f32>(hash2(g + 1.7), hash2(g + 3.3)));
  if (h > 0.7 && d < 0.18) { c = mix(c, vec3<f32>(0.45, 0.36, 0.18), 0.6); }
  return c;
}

fn road(world: vec2<f32>) -> vec3<f32> {
  var c = ASPHALT * (0.9 + 0.2 * noise2(world * 9.0));
  let mid = (u.p0.z + u.p0.w) * 0.5;
  let dash = step(0.5, fract(world.x * 0.33));
  if (abs(world.y - mid) < 0.05 && dash > 0.5) { c = vec3<f32>(0.85, 0.80, 0.55); }
  // the underpass's grate, letting the light into the tunnel
  if (world.x > u.p1.z && world.x < u.p1.w) {
    let bar = step(0.5, fract(world.x * 12.0));
    c = mix(vec3<f32>(0.05, 0.05, 0.05), vec3<f32>(0.42, 0.42, 0.44), bar);
  }
  return c;
}

fn shore(world: vec2<f32>) -> vec3<f32> {
  let pits = noise2(world * 4.0) * 0.6 + noise2(world * 13.0) * 0.4;
  return STONE * (0.6 + 0.6 * pits);
}

fn sea(world: vec2<f32>, t: f32) -> vec3<f32> {
  let d = world.y - u.p1.y;
  var c = mix(SHALLOW, DEEP, smoothstep(0.0, 2.0, d));
  let wave = sin(world.y * 5.0 - t * 1.5 + noise2(world * 0.8) * 4.0);
  c = c + vec3<f32>(0.08) * smoothstep(0.85, 1.0, wave);
  // the foam's edge, running in and out
  let edge = u.p1.y + 0.12 * sin(t * 0.9 + world.x * 0.7) + 0.08 * noise2(vec2<f32>(world.x * 2.0, t));
  return mix(vec3<f32>(0.92, 0.95, 0.95), c, smoothstep(edge, edge + 0.18, world.y));
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  var c: vec3<f32>;
  let edge = u.p0.z;
  if (world.y > u.p1.y - 0.15) {
    c = sea(world, u.time);
    if (world.y < u.p1.y + 0.05) { c = mix(shore(world), c, smoothstep(u.p1.y - 0.15, u.p1.y + 0.05, world.y)); }
  } else if (world.y > u.p1.x) {
    let jag = noise2(vec2<f32>(world.x * 1.5, 0.0)) * 0.5;
    c = select(forest(world), shore(world), world.y > u.p1.x + jag);
  } else if (world.y > edge && world.y < u.p0.w) {
    c = road(world);
    let kerb = min(world.y - edge, u.p0.w - world.y);
    c = mix(vec3<f32>(0.55, 0.55, 0.52), c, smoothstep(0.03, 0.08, kerb));
  } else {
    c = forest(world);
  }
  // the light: dim at night, warm and bright when the sun is high
  let day = u.p0.x;
  let lit = mix(vec3<f32>(0.35, 0.38, 0.55), vec3<f32>(1.08, 1.04, 0.95), day);
  return vec4<f32>(c * lit, 1.0);
}
