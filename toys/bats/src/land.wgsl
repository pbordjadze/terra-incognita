// The dusk over the Hill Country north of San Antonio, seen from a rise above Bracken Cave: the sky
// by the elevation and bearing of each pixel's ray, the low hills on the horizon with their trees, the
// scrub between, and the cave's sinkhole. A backdrop (toy-kit backdrop.rs): u.p0 = (focal length in
// pixels, the camera's pitch up in radians, dusk 0..1, the eye's height in metres); u.p1 = (the cave's
// distance, its radius, its offset to the right, 0).

const PI: f32 = 3.14159265;

fn ray(px: vec2<f32>) -> vec3<f32> {
  let f = u.p0.x;
  let th = u.p0.y;
  let c = vec3<f32>((px.x - u.size.x * 0.5) / f, -(px.y - u.size.y * 0.5) / f, 1.0);
  let y = c.y * cos(th) + c.z * sin(th);
  let z = -c.y * sin(th) + c.z * cos(th);
  return normalize(vec3<f32>(c.x, y, z));
}

fn fbm(p: vec2<f32>) -> f32 {
  var s = 0.0;
  var a = 0.5;
  var q = p;
  for (var i = 0; i < 5; i = i + 1) {
    s = s + a * noise2(q);
    q = q * 2.03 + vec2<f32>(17.1, 9.3);
    a = a * 0.5;
  }
  return s;
}

// The sky toward bearing `az` (radians, 0 straight ahead, positive to the right) at elevation `el`.
fn sky(el: f32, az: f32, dusk: f32) -> vec3<f32> {
  // the sun went down a little left of ahead, and sinks as dusk deepens
  let sun_az = -0.55;
  let d_az = az - sun_az;
  let toward = exp(-d_az * d_az * 1.4);
  let h = max(el, 0.0);
  let zen_early = vec3<f32>(0.13, 0.20, 0.42);
  let zen_late = vec3<f32>(0.04, 0.05, 0.14);
  let zen = mix(zen_early, zen_late, dusk);
  let band_early = vec3<f32>(1.00, 0.66, 0.38);
  let band_late = vec3<f32>(0.80, 0.36, 0.24);
  let band = mix(band_early, band_late, dusk);
  let rose = mix(vec3<f32>(0.86, 0.56, 0.58), vec3<f32>(0.42, 0.26, 0.38), dusk);
  // the glow low in the west, the rose above it, the blue overhead
  let low = exp(-h * mix(9.0, 13.0, dusk));
  let mid = exp(-h * 2.4);
  var c = mix(zen, rose, mid * (0.55 + 0.25 * toward));
  c = mix(c, band, low * (0.35 + 0.65 * toward));
  // the opposite side: the Earth's shadow, a dusky blue band rising behind
  let back = exp(-pow(abs(d_az) - PI, 2.0) * 0.6);
  c = mix(c, c * vec3<f32>(0.70, 0.72, 0.92), back * exp(-h * 6.0) * 0.5);
  // a few thin clouds, lit from beneath
  let cl = fbm(vec2<f32>(az * 3.0, h * 26.0 - u.time * 0.003) * vec2<f32>(1.0, 0.35));
  let thin = smoothstep(0.55, 0.78, cl) * exp(-h * 5.0) * smoothstep(0.0, 0.03, h);
  let lit = mix(vec3<f32>(1.0, 0.62, 0.48), vec3<f32>(0.62, 0.30, 0.36), dusk) * (0.6 + 0.6 * toward);
  c = mix(c, lit, thin * 0.55);
  // the first stars, once it is dark enough
  let sp = vec2<f32>(az, el) * 260.0;
  let st = hash2(floor(sp));
  let star = step(0.9965, st) * smoothstep(0.55, 0.95, dusk) * smoothstep(0.05, 0.4, h);
  c = c + vec3<f32>(star * 0.5);
  return c;
}

// The hills on the horizon: their height (radians above level) at bearing `az`, trees on them.
fn ridge(az: f32) -> f32 {
  let hill = 0.012 + 0.022 * fbm(vec2<f32>(az * 2.2, 3.7)) + 0.008 * sin(az * 3.0 + 1.0);
  // live oaks and junipers: round crowns, in clumps, two rows overlapping
  var trees = 0.0;
  for (var k = 0; k < 2; k = k + 1) {
    let x = az * (150.0 + 70.0 * f32(k)) + f32(k) * 0.37;
    let cell = floor(x);
    let f = fract(x) - 0.5;
    let n = hash2(vec2<f32>(cell, f32(k) + 2.0));
    let crown = sqrt(max(0.0, 0.25 - f * f)) * 2.0;
    let clump = smoothstep(0.35, 0.6, noise2(vec2<f32>(az * 11.0, 4.0 + f32(k))));
    trees = max(trees, 0.0065 * crown * (0.5 + 0.5 * n) * step(0.3, n) * clump);
  }
  return hill + trees;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let d = ray(px);
  let dusk = u.p0.z;
  let eye = u.p0.w;
  let el = asin(clamp(d.y, -1.0, 1.0));
  let az = atan2(d.x, d.z);
  var c = sky(el, az, dusk);
  // the haze toward the horizon
  let haze = sky(0.004, az, dusk);
  let r = ridge(az);
  if (el < r) {
    // the far hills, a silhouette with a little of the sky's light in it
    let far = mix(haze * 0.42, vec3<f32>(0.10, 0.08, 0.10), 0.35 + 0.4 * dusk);
    c = far;
  }
  if (d.y < 0.0) {
    // the ground: where the ray meets it
    let t = eye / -d.y;
    let g = vec2<f32>(d.x, d.z) * t;
    // the near ground rises toward the watcher: we stand on a low rise
    let scrub = fbm(g * 0.09);
    let clumps = smoothstep(0.52, 0.66, fbm(g * 0.045 + vec2<f32>(3.0, 1.0)));
    var ground = mix(vec3<f32>(0.20, 0.17, 0.12), vec3<f32>(0.13, 0.14, 0.09), scrub);
    ground = mix(ground, vec3<f32>(0.05, 0.07, 0.05), clumps);
    // the west's light on it, fading with the dusk
    ground = ground * mix(1.0, 0.45, dusk) * (0.75 + 0.35 * exp(-pow(az + 0.55, 2.0)));
    // the sinkhole: a dark mouth in pale limestone
    let cave = vec2<f32>(u.p1.z, u.p1.x);
    let rc = u.p1.y;
    let q = (g - cave) * vec2<f32>(1.0, 1.25);
    let rr = length(q) + 2.5 * (fbm(q * 0.25) - 0.5);
    let rim = smoothstep(rc * 1.9, rc * 1.1, rr) * (1.0 - smoothstep(rc * 1.05, rc * 0.9, rr)) * (0.35 + 0.65 * fbm(q * 1.3));
    let stone = mix(vec3<f32>(0.62, 0.55, 0.45), vec3<f32>(0.30, 0.25, 0.24), dusk) * (0.7 + 0.5 * fbm(q * 0.6));
    ground = mix(ground, stone, rim * 0.6);
    let mouth = smoothstep(rc * 1.0, rc * 0.82, rr);
    ground = mix(ground, vec3<f32>(0.015, 0.012, 0.015), mouth);
    // farther ground goes into the haze
    let k = 1.0 - exp(-t / 2200.0);
    c = mix(ground, mix(haze * 0.5, c, 0.0), k * 0.85);
    if (el < r && t > 2500.0) {
      c = mix(haze * 0.42, vec3<f32>(0.10, 0.08, 0.10), 0.35 + 0.4 * dusk);
    }
  }
  // near juniper and oak, black against the dusk, framing the foot of the view
  let uv = px / u.size;
  let aspect = u.size.x / u.size.y;
  let xs = uv.x * aspect;
  let left = 0.075 + 0.07 * fbm(vec2<f32>(xs * 14.0, 2.0)) - 0.32 * max(uv.x - 0.12, 0.0)
    + 0.02 * smoothstep(0.5, 0.8, noise2(vec2<f32>(xs * 60.0, 5.0)));
  let right = 0.06 + 0.06 * fbm(vec2<f32>(xs * 12.0, 7.0)) - 0.4 * max(0.86 - uv.x, 0.0)
    + 0.02 * smoothstep(0.5, 0.8, noise2(vec2<f32>(xs * 55.0, 9.0)));
  let grass = 0.022 + 0.012 * noise2(vec2<f32>(xs * 90.0, 1.0)) + 0.01 * fbm(vec2<f32>(xs * 9.0, 3.0));
  let top = max(max(left, right), grass);
  let edge = 1.0 - uv.y - top;
  let leaf = smoothstep(0.0015, -0.0015, edge);
  // a lacy edge: light through the leaves near their tops
  let lace = smoothstep(0.42, 0.62, noise2(px * 0.09)) * smoothstep(-0.02, 0.0, edge);
  let near = vec3<f32>(0.022, 0.022, 0.028);
  c = mix(c, near, leaf * (1.0 - lace * 0.75));
  return vec4<f32>(c, 1.0);
}
