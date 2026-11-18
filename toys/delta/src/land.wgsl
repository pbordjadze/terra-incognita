// The Okavango from the air in the flood season: pale Kalahari sand, the floodplain's grass greening where
// water has lain, open channels and lagoons dark with sky in them, and islands ringed by trees. A field
// (toy-kit field.rs): `world` in cells; per cell (water depth m, height above the fan's plane m, green
// 0..1, the island's rise m); u.p0 = (sun x, sun y, islands on, 0).

fn at(c: vec2<f32>) -> vec4<f32> { return fld(clamp(c, vec2<f32>(0.5), u.grid - vec2<f32>(0.5))); }

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  // the edges of water and green wander inside a cell, as real ones do
  let c = world + (vec2<f32>(noise2(world * 0.9), noise2(world * 0.9 + 17.0)) - 0.5) * 1.1;
  let f = at(c);
  let depth = f.x;
  let green = f.z;
  let mound = f.w * u.p0.z;
  // the ground's light: hills shaded from the sun at the upper left
  let e = 0.75;
  let hx = (at(c + vec2<f32>(e, 0.0)).y - at(c - vec2<f32>(e, 0.0)).y) / (2.0 * e);
  let hy = (at(c + vec2<f32>(0.0, e)).y - at(c - vec2<f32>(0.0, e)).y) / (2.0 * e);
  let sun = normalize(u.p0.xy + vec2<f32>(1e-4, 0.0));
  let lit = clamp(1.0 - 0.3 * dot(vec2<f32>(hx, hy), sun), 0.7, 1.25);
  // sand and dry grass
  let m = noise2(world * 0.11) * 0.5 + noise2(world * 0.53) * 0.3 + noise2(world * 2.7) * 0.2;
  var ground = vec3<f32>(0.80, 0.72, 0.56) * (0.86 + 0.22 * m);
  let straw = vec3<f32>(0.70, 0.63, 0.44) * (0.9 + 0.2 * noise2(world * 1.9));
  ground = mix(ground, straw, smoothstep(0.35, 0.7, noise2(world * 0.31)) * 0.6);
  // the floodplain: grass greening where water has lain
  let lush = vec3<f32>(0.33, 0.45, 0.20) * (0.85 + 0.3 * noise2(world * 1.3));
  ground = mix(ground, lush, smoothstep(0.05, 0.8, green));
  // trees: dark crowns, thick on the islands' rims and along the wet margins, scattered elsewhere
  let rim = smoothstep(0.2, 0.7, mound) * (1.0 - smoothstep(1.2, 1.9, mound));
  let crowd = 0.025 + 0.5 * rim + 0.18 * green * (1.0 - smoothstep(0.05, 0.4, depth));
  let tq = world * 3.1;
  let tg = floor(tq);
  let th = hash2(tg);
  let tc = tg + vec2<f32>(hash2(tg + 3.1), hash2(tg + 5.7)) * 0.6 + 0.2;
  let tr = 0.18 + 0.22 * hash2(tg + 1.7);
  let tree = select(0.0, 1.0 - smoothstep(tr * 0.6, tr, length(tq - tc)), th < crowd);
  let shadow = select(0.0, 1.0 - smoothstep(tr * 0.6, tr * 1.1, length(tq - tc - sun * -0.25)), th < crowd);
  ground = ground * (1.0 - 0.35 * shadow);
  ground = mix(ground, vec3<f32>(0.12, 0.20, 0.09) * (0.8 + 0.4 * hash2(tg + 9.0)), tree);
  // the island's top: pale, salted sand in the middle of the bigger ones
  ground = mix(ground, vec3<f32>(0.86, 0.83, 0.74), smoothstep(1.7, 2.3, mound) * 0.45);
  var col = ground * lit;
  // water: flooded grass shows through the shallows; deep channels and lagoons hold the sky
  let shallow = smoothstep(0.03, 0.12, depth);
  let deep = smoothstep(0.25, 1.4, depth);
  let skyc = vec3<f32>(0.15, 0.29, 0.38) + vec3<f32>(0.06, 0.08, 0.09) * noise2(world * 0.4 + vec2<f32>(u.time * 0.03, 0.0));
  let flooded = mix(vec3<f32>(0.30, 0.48, 0.24) * (0.85 + 0.3 * noise2(world * 1.7)), skyc, 0.3);
  col = mix(col, flooded, shallow * 0.85);
  col = mix(col, skyc, deep);
  // a little light on the open water, moving with the breeze
  let glint = pow(noise2(world * 2.2 + vec2<f32>(u.time * 0.12, u.time * 0.05)), 8.0);
  col += vec3<f32>(0.5, 0.55, 0.55) * glint * deep * 0.6;
  // the water's edge: a thin darker line where it laps the sand
  let edge = shallow * (1.0 - smoothstep(0.12, 0.3, depth));
  col = col * (1.0 - 0.15 * edge);
  return vec4<f32>(col, 1.0);
}
