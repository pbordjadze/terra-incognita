// The Okavango from the air in the flood season: the Kalahari's pale grey-buff sand and khaki grass, the
// floodplain bright green with reeds and grass standing in shallow water, the deeper channels and lagoons
// dark with the sky in them, and islands, a dark ring of trees round a pale salt crust. A field (toy-kit
// field.rs): `world` in cells; per cell (water depth m, height above the fan's plane m, green 0..1, the
// island's rise m); u.p0 = (sun x, sun y, islands on, the summer rains 0..1).

fn at(c: vec2<f32>) -> vec4<f32> { return fld(clamp(c, vec2<f32>(0.5), u.grid - vec2<f32>(0.5))); }

fn fbm(p: vec2<f32>) -> f32 {
  return noise2(p) * 0.5 + noise2(p * 2.03 + 7.1) * 0.25 + noise2(p * 4.1 + 3.3) * 0.15 + noise2(p * 8.3 + 1.9) * 0.1;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  // the edges of water and green wander inside a cell, as real ones do
  let c = world + (vec2<f32>(fbm(world * 0.7), fbm(world * 0.7 + 17.0)) - 0.5) * 1.6;
  let f = at(c);
  let depth = f.x;
  let green = f.z;
  let mound = f.w * u.p0.z;
  // the ground's light: rises shaded from the low sun at the upper left
  let e = 0.75;
  let hx = (at(c + vec2<f32>(e, 0.0)).y - at(c - vec2<f32>(e, 0.0)).y) / (2.0 * e);
  let hy = (at(c + vec2<f32>(0.0, e)).y - at(c - vec2<f32>(0.0, e)).y) / (2.0 * e);
  let sun = normalize(u.p0.xy + vec2<f32>(1e-4, 0.0));
  let lit = clamp(1.0 - 0.3 * dot(vec2<f32>(hx, hy), sun), 0.75, 1.2);
  // the dry land: grey-buff sand, khaki grass and darker scrub in broad patches
  let big = fbm(world * 0.035);
  let mid = fbm(world * 0.13 + 5.0);
  let grain = fbm(world * 1.7);
  let sand = vec3<f32>(0.80, 0.76, 0.66);
  let grass = vec3<f32>(0.68, 0.63, 0.46);
  let scrub = vec3<f32>(0.47, 0.47, 0.34);
  var ground = mix(sand, grass, smoothstep(0.35, 0.65, big));
  ground = mix(ground, scrub, smoothstep(0.55, 0.75, mid) * 0.6);
  ground = ground * (0.9 + 0.18 * grain);
  // bush: soft dark clumps, thicker toward the wet ground
  let bush = smoothstep(0.64 - 0.1 * green, 0.84 - 0.1 * green, fbm(world * 0.6 + 11.0));
  ground = mix(ground, vec3<f32>(0.36, 0.38, 0.24), bush * 0.4);
  // the floodplain after the water: grass greening where it has lain, staying green a while
  let lush = vec3<f32>(0.47, 0.56, 0.30) * (0.9 + 0.2 * grain);
  ground = mix(ground, lush, smoothstep(0.1, 0.9, green) * 0.6);
  // the summer rains green the dry land everywhere, the grass first, the sand last
  let rained = vec3<f32>(0.52, 0.60, 0.33) * (0.9 + 0.2 * grain);
  ground = mix(ground, rained, u.p0.w * (0.35 + 0.35 * smoothstep(0.3, 0.6, big)));
  // islands: trees in broken clumps round the rise, a pale crust in the middle of the biggest
  let clumps = smoothstep(0.38, 0.62, fbm(world * 1.6 + 23.0));
  let trees = smoothstep(0.25, 0.6, mound) * mix(0.35, 1.0, clumps);
  let canopy = vec3<f32>(0.15, 0.25, 0.11) * (0.6 + 0.7 * fbm(world * 3.4));
  ground = mix(ground, canopy, trees);
  let crust = smoothstep(1.9, 2.5, mound) * smoothstep(0.3, 0.6, fbm(world * 2.1 + 31.0));
  ground = mix(ground, vec3<f32>(0.76, 0.73, 0.64) * (0.92 + 0.1 * grain), crust * 0.75);
  var col = ground * lit;
  // the flood: water, the sky in it darker toward the bottom of the screen, rippled, glinting
  let wet = smoothstep(0.012, 0.04, depth);
  let sky = mix(vec3<f32>(0.42, 0.58, 0.62), vec3<f32>(0.24, 0.38, 0.44), clamp(px.y / u.size.y, 0.0, 1.0));
  let ripple = fbm(world * 1.3 + vec2<f32>(u.time * 0.05, u.time * 0.02));
  // clear and shallow: the drowned grass shows through, darker and greener the deeper it lies
  let under = mix(col, vec3<f32>(0.20, 0.30, 0.18), 0.5);
  let clear = mix(0.55, 0.15, smoothstep(0.05, 0.8, depth));
  var water = mix(sky * (0.85 + 0.3 * ripple) * mix(1.0, 0.7, smoothstep(0.1, 0.8, depth)), under, clear);
  let glint = smoothstep(0.62, 0.9, fbm(world * 0.9 + vec2<f32>(u.time * 0.08, -u.time * 0.03)));
  water += vec3<f32>(0.35, 0.42, 0.45) * glint * 0.5 * smoothstep(0.05, 0.15, depth);
  // reeds and grass standing up through the shallows, in beds, thinning as the water deepens
  let beds = smoothstep(0.42, 0.62, fbm(world * 1.1 + 2.0)) * (1.0 - smoothstep(0.05, 0.22, depth));
  let blades = 0.75 + 0.5 * fbm(world * 4.3 + 6.0);
  let reeds = vec3<f32>(0.33, 0.50, 0.18) * blades;
  water = mix(water, reeds, beds * 0.85);
  // the reed fringe: a thin bright green band where the flood meets dry ground
  let fringe = wet * (1.0 - smoothstep(0.03, 0.07, depth));
  water = mix(water, vec3<f32>(0.40, 0.58, 0.20) * blades, fringe * 0.7);
  col = mix(col, water, wet);
  return vec4<f32>(col, 1.0);
}
