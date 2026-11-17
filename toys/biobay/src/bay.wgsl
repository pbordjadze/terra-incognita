// Mosquito Bay at night, looking down over a kayak's side (bay.rs): the dark water in Paint by Moonlight's
// night paper, the mangroves' prop roots, the stars' reflections, and the dinoflagellates' blue light where
// the water has been stirred, as a haze and as the single cells' sparks. With the moon up, the water takes
// its glints in Paint by Moonlight's moonlight paints, and the blue light all but drowns.
// A field (toy-kit field.rs): per cell (light, charge, stir, -).
// u.p0 = (the moon 0..1, CELL cm, -, -); world is in cm from the top left.

const NIGHT = vec3<f32>(0.058, 0.036, 0.082);
const MOONLIT = vec3<f32>(0.20, 0.17, 0.25);
const ROOT = vec3<f32>(0.010, 0.008, 0.014);
const SPARK = vec3<f32>(0.62, 0.94, 1.0);
const HAZE = vec3<f32>(0.04, 0.40, 0.95);

fn paint_of(h: f32) -> vec3<f32> {
  // Paint by Moonlight's moonlight paints: a marshmallow, a candle wax, a pale lilac
  if (h < 0.34) { return vec3<f32>(0.93, 0.90, 0.80); }
  if (h < 0.67) { return vec3<f32>(0.92, 0.84, 0.67); }
  return vec3<f32>(0.84, 0.81, 0.89);
}

// The stars over the bay, seen in the water, shivering with the ripples.
fn star_glints(world: vec2<f32>, t: f32) -> f32 {
  let g = world / 2.6;
  let c = floor(g);
  let h = hash2(c);
  if (h < 0.80) { return 0.0; }
  let wob = vec2<f32>(noise2(world * 0.35 + vec2<f32>(t * 0.35, 0.0)), noise2(world * 0.35 + vec2<f32>(4.0, t * 0.3))) - 0.5;
  let sp = vec2<f32>(hash2(c + 3.1), hash2(c + 7.7)) + wob * 0.25;
  let d = length(fract(g) - sp) * 2.6;
  let b = pow(hash2(c + 1.3), 4.0);
  return b * exp(-d * d / 0.012) * (0.6 + 0.4 * sin(t * (1.0 + 3.0 * h) + h * 40.0));
}

// The moon's glints on the ripples: sparse points that catch it where a ripple's slope is right, flickering
// as the ripples run, each a dab of a moonlight paint.
fn moon_glints(world: vec2<f32>, t: f32) -> vec3<f32> {
  let s = 0.55;
  let g = world / s;
  let c = floor(g);
  // the ripples' slope comes right in long bands that run across the water
  let band = noise2(world * vec2<f32>(0.22, 0.6) + vec2<f32>(t * 0.35, t * 0.9));
  let beat = floor(t * 7.0 + hash2(c + 4.4) * 7.0);
  let on = hash2(c + vec2<f32>(beat * 0.53, beat * 0.29));
  let k = smoothstep(0.86, 0.97, on * (0.55 + 0.6 * band));
  let sp = vec2<f32>(hash2(c + 3.1), hash2(c + 7.7)) * 0.7 + 0.15;
  let d = length(fract(g) - sp) * s;
  let r = 0.05 + 0.05 * hash2(c + 1.9);
  return paint_of(hash2(c + 6.1)) * k * exp(-d * d / (r * r));
}

// The single cells lighting: a spark each, where the light is, twinkling as they flash and go out.
fn sparks(world: vec2<f32>, light: f32, t: f32) -> f32 {
  let s = 0.19;
  let g = world / s;
  let c = floor(g);
  let sp = vec2<f32>(hash2(c + 3.1), hash2(c + 7.7)) * 0.7 + 0.15;
  let d = length(fract(g) - sp) * s;
  let beat = floor(t * 9.0 + hash2(c + 5.5) * 9.0);
  let on = hash2(c + vec2<f32>(beat * 0.37, beat * 0.71));
  let lit = 1.0 - exp(-light * 0.5);
  let flash = smoothstep(1.0 - lit, 1.0 - lit + 0.08, on);
  let r = 0.04 + 0.035 * hash2(c + 9.9);
  return flash * exp(-d * d / (r * r)) * (0.6 + 0.8 * hash2(c + 2.2));
}

// The red mangroves' prop roots, arching down into the water from the fringe along the top of the screen,
// each a column's own (its reach, lean and thickness from its hash): how much of this pixel is root.
fn roots(world: vec2<f32>) -> f32 {
  let colw = 4.2;
  let i = floor(world.x / colw);
  let px = 1.2 / u.scale;
  var cov = 0.0;
  for (var k = -2; k <= 2; k = k + 1) {
    let c = i + f32(k);
    let h = hash2(vec2<f32>(c, 1.7));
    let reach = 7.0 + 20.0 * hash2(vec2<f32>(c, 2.9)) * hash2(vec2<f32>(c, 8.3));
    let lean = (hash2(vec2<f32>(c, 4.1)) - 0.5) * 9.0;
    let uu = clamp(world.y / reach, 0.0, 1.0);
    let x = (c + h) * colw + lean * uu * uu + sin(uu * 5.0 + h * 9.0) * 0.5;
    let r = (0.22 + 0.32 * hash2(vec2<f32>(c, 6.6))) * (1.35 - 0.75 * uu);
    let d = abs(world.x - x);
    let tip = smoothstep(reach + 0.3, reach - 0.4, world.y);
    cov = max(cov, smoothstep(r + px, r - px, d) * tip);
  }
  // the fringe itself, close-packed roots and leaves along the top
  let fringe = 1.4 + 1.6 * noise2(vec2<f32>(world.x * 0.35, 0.5)) + 0.8 * noise2(vec2<f32>(world.x * 1.3, 2.5));
  cov = max(cov, smoothstep(fringe + px, fringe - px, world.y));
  return cov;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let moon = u.p0.x;
  let f = fld(world / u.p0.y + 0.5);
  let light = f.x;
  let root = roots(world);
  let t = u.time;

  // the water: the night's paper, a slow swell of lighter and darker
  let swell = noise2(world * 0.045 + vec2<f32>(t * 0.03, t * 0.02)) * 0.6 + noise2(world * 0.16 - t * 0.05) * 0.4;
  var col = mix(NIGHT, MOONLIT, moon) * (0.8 + 0.4 * swell);
  col = col + vec3<f32>(0.75, 0.78, 0.95) * star_glints(world, t) * (1.0 - moon) * 0.55;
  col = col + moon_glints(world, t) * moon * 1.1;

  // the light: its haze in the water, and the sparks of the single cells; the moon drowns them
  let seen = mix(1.0, 0.16, moon);
  let haze = 1.0 - exp(-light * 0.45);
  col = col + HAZE * haze * 0.5 * seen;
  col = col + SPARK * sparks(world, light, t) * (1.2 + 1.2 * smoothstep(1.0, 5.0, light)) * seen;
  col = col + SPARK * smoothstep(1.5, 6.0, light) * 0.35 * seen;
  // the light's bloom, wider: the water around a bright stroke glows too
  let c = world / u.p0.y + 0.5;
  var wide = 0.0;
  for (var k = 0; k < 8; k = k + 1) {
    let a = f32(k) * 0.785398;
    wide = wide + fld(c + vec2<f32>(cos(a), sin(a)) * 4.0).x;
  }
  col = col + HAZE * (1.0 - exp(-wide / 8.0 * 0.35)) * 0.30 * seen;

  // the prop roots, dark against the water, a rim of moon on them when it is up
  // the roots, dark against the water; the blue light catches their edges, and the moon too when it is up
  let edge = root * (1.0 - root) * 4.0;
  col = mix(col, ROOT, root) + edge * (HAZE * haze * 0.4 * seen + vec3<f32>(0.5, 0.48, 0.6) * moon * 0.06);

  // a little vignette, as the eye in the dark
  let q = px / u.size - 0.5;
  col = col * (1.0 - 0.35 * dot(q, q));
  return vec4<f32>(col / (1.0 + col * 0.2), 1.0);
}
