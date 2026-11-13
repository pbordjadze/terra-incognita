// Catatumbo's night: the storm's cloud deck over Lake Maracaibo, lit from inside by the flashes, the
// far shore with its stilt houses' lamps, and the lake giving the sky back, broken by ripples. A
// backdrop (toy-kit backdrop.rs): u.size the target in pixels, u.scale pixels a point, u.time.
// u.p0 = (flash, flash x, flash y, horizon y), u.p1 = (glow of the flickers, x, y, rain): pixels.

fn fbm(p0: vec2<f32>) -> f32 {
  var p = p0; var a = 0.5; var s = 0.0;
  for (var i = 0; i < 5; i = i + 1) { s = s + a * noise2(p); p = p * 2.03 + vec2<f32>(17.1, 9.2); a = a * 0.5; }
  return s;
}

// The sky at a pixel above the horizon (y < horizon): cloud, its light, and the flashes in it.
fn sky(px: vec2<f32>) -> vec3<f32> {
  let hy = u.p0.w;
  let v = clamp(px.y / hy, 0.0, 1.0);
  // perspective: the deck overhead is near and large, toward the horizon far and thin
  let depth = 1.0 / max(1.08 - v, 0.06);
  let q = vec2<f32>((px.x - u.size.x * 0.5) / u.size.x * 2.2 * depth, depth * 1.6);
  let drift = vec2<f32>(u.time * 0.012, 0.0);
  let dens = smoothstep(0.32, 0.78, fbm(q * 1.3 + drift)) * (1.0 - smoothstep(0.86, 1.0, v) * 0.6);
  let base = mix(vec3<f32>(0.016, 0.018, 0.045), vec3<f32>(0.060, 0.045, 0.090), v);
  var col = base + dens * vec3<f32>(0.030, 0.028, 0.050);
  // the flash: lit from inside, the thick of the cloud lit most
  let fl = u.p0.x * 1.6 * exp(-length(px - u.p0.yz) / (u.size.y * 0.32));
  let fk = u.p1.x * exp(-length(px - u.p1.yz) / (u.size.y * 0.16));
  let light = fl + fk;
  let tint = vec3<f32>(0.78, 0.66, 1.0);
  col = col + tint * light * (0.12 + 1.15 * dens);
  // the whole deck takes a little of every flash
  col = col + tint * u.p0.x * (0.10 + 0.25 * dens);
  return col;
}

// The far shore: a low dark line with stilt houses and their lamps.
fn shore(px: vec2<f32>, hy: f32) -> vec4<f32> {
  let s = u.scale;
  let x = px.x / s;
  let h = (1.2 + 2.2 * noise2(vec2<f32>(x * 0.02, 3.0)) + 1.6 * noise2(vec2<f32>(x * 0.11, 7.0))) * s;
  // the houses: little boxes on stilts, in clumps
  let cell = floor(x / 7.0);
  let clump = step(0.62, noise2(vec2<f32>(x * 0.035, 1.0)));
  let has = clump * step(0.35, hash2(vec2<f32>(cell, 2.0)));
  let fx = fract(x / 7.0);
  let house = has * step(0.15, fx) * step(fx, 0.85);
  let roof = h + house * 3.4 * s;
  let top = hy - roof;
  if (px.y < top || px.y > hy) { return vec4<f32>(0.0); }
  var col = vec3<f32>(0.004, 0.004, 0.008);
  // a lamp in a window, warm
  let lit = house * step(0.55, hash2(vec2<f32>(cell, 5.0)));
  let wy = hy - h - 1.7 * s;
  let lamp = lit * exp(-pow(length(vec2<f32>((fx - 0.5) * 7.0 * s, px.y - wy)) / (0.7 * s), 2.0));
  col = col + vec3<f32>(1.0, 0.62, 0.25) * lamp * 1.4;
  return vec4<f32>(col, 1.0);
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let hy = u.p0.w;
  if (px.y < hy) {
    var col = sky(px);
    let sh = shore(px, hy);
    col = mix(col, sh.rgb, sh.a);
    return vec4<f32>(col, 1.0);
  }
  // the lake: the sky mirrored about the horizon, broken by ripples that are finer far away
  let below = px.y - hy;
  let near = below / (u.size.y - hy);
  let freq = 0.9 / max(near, 0.02);
  let t = u.time;
  let wob = (noise2(vec2<f32>(px.x / u.scale * 0.04, below / u.scale * 0.5 * freq * 0.05 - t * 0.6)) - 0.5);
  let mx = px.x + wob * u.scale * (3.0 + 30.0 * near);
  let my = hy - below * (1.0 + 0.25 * wob);
  var col = sky(vec2<f32>(mx, my)) * (0.30 + 0.25 * (1.0 - near));
  let sh = shore(vec2<f32>(mx, hy - (below * 1.0)), hy);
  col = mix(col, sh.rgb * 0.7, sh.a * 0.8);
  // fine glints of the flash on the wavelets
  let glint = smoothstep(0.82, 1.0, noise2(vec2<f32>(px.x / u.scale * 0.5, below / u.scale * 2.0 * freq * 0.1 + t * 0.4)));
  col = col + vec3<f32>(0.78, 0.66, 1.0) * glint * (u.p0.x + u.p1.x * 0.3) * 0.10;
  // rain on the water: tiny rings coming and going
  let rc = floor(px / (u.scale * 9.0));
  let rf = fract(px / (u.scale * 9.0)) - 0.5;
  let ph = fract(t * 0.7 + hash2(rc));
  let ring = exp(-pow((length(rf * vec2<f32>(1.0, 2.6)) - ph * 0.45) * 18.0, 2.0)) * (1.0 - ph) * step(0.6, hash2(rc + 3.0));
  col = col + vec3<f32>(0.10, 0.10, 0.16) * ring * u.p1.w * near * (0.2 + u.p0.x);
  return vec4<f32>(col, 1.0);
}
