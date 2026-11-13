// The sea ice of Atka Bay on a winter's day near noon, seen from above: wind-carved snow, drift
// streaming over it, the low light of a sun just under the horizon, and a little of the aurora's green
// in it. A backdrop (toy-kit backdrop.rs): world in metres; u.p0 = (the wind's direction x, y, its
// speed, the light 0..1); u.p1 = (the huddle's middle x, y, its radius, 0).

fn fbm(p: vec2<f32>) -> f32 {
  var s = 0.0;
  var a = 0.5;
  var q = p;
  for (var i = 0; i < 4; i = i + 1) {
    s = s + a * noise2(q);
    q = q * 2.07 + vec2<f32>(13.1, 7.7);
    a = a * 0.5;
  }
  return s;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let w = normalize(u.p0.xy);
  let side = vec2<f32>(-w.y, w.x);
  let speed = u.p0.z;
  let light = u.p0.w;
  // in the wind's frame: along it and across it
  let q = vec2<f32>(dot(world, w), dot(world, side));
  // sastrugi: ridges the wind has carved, long along it, laid down over the weeks (not this wind)
  let old = vec2<f32>(world.x * 0.8 + world.y * 0.6, -world.x * 0.6 + world.y * 0.8);
  let ridge = fbm(old * vec2<f32>(0.25, 1.3));
  let fine = fbm(world * 7.0);
  var c = mix(vec3<f32>(0.76, 0.80, 0.90), vec3<f32>(0.90, 0.91, 0.96), ridge);
  c = c * (0.92 + 0.12 * fine);
  // the ridges' lee sides a little shaded by the low light from the north
  let sh = fbm(old * vec2<f32>(0.25, 1.3) + vec2<f32>(0.0, 0.08));
  // the low sun from the north warms the ridges' faces; their backs stay blue
  let face = clamp((ridge - sh) * 9.0, -1.0, 1.0);
  c = c * mix(vec3<f32>(0.88, 0.91, 1.0), vec3<f32>(1.04, 0.98, 0.94), face * 0.5 + 0.5);
  // drift: snow streaming low over the surface with the wind
  let t = u.time;
  let d1 = fbm(vec2<f32>(q.x * 0.9 - t * speed * 0.55, q.y * 7.0));
  let d2 = fbm(vec2<f32>(q.x * 1.7 - t * speed * 0.8, q.y * 13.0 + 3.0));
  let drift = smoothstep(0.5, 0.8, d1) * 0.6 + smoothstep(0.55, 0.85, d2) * 0.4;
  // each streak of drift with its faint blue shadow just beside it
  let d3 = fbm(vec2<f32>(q.x * 0.9 - t * speed * 0.55, q.y * 7.0 - 0.35));
  let shade = smoothstep(0.5, 0.8, d3) * (1.0 - smoothstep(0.5, 0.8, d1));
  let gust = clamp(speed / 8.0, 0.25, 1.4);
  c = c * (1.0 - shade * 0.10 * gust);
  c = mix(c, vec3<f32>(0.98, 0.98, 1.0), drift * gust * 0.5);
  // where the huddle has stood: the snow trodden and stained
  let m = u.p1.xy;
  let r = u.p1.z;
  let under = smoothstep(r + 0.6, r - 0.3, length(world - m));
  c = mix(c, c * vec3<f32>(0.80, 0.78, 0.76), under * 0.5);
  // the light: a dim noon twilight, blue, the aurora's green moving slowly over it
  let aur = fbm(vec2<f32>(world.x * 0.04 + t * 0.006, world.y * 0.02 - t * 0.004));
  let green = vec3<f32>(0.75, 1.08, 0.88);
  c = c * mix(vec3<f32>(1.0), green, smoothstep(0.45, 0.75, aur) * 0.45);
  // the light: rose from the sun just over the northern horizon, blue in the shade
  c = c * mix(vec3<f32>(0.42, 0.50, 0.70), vec3<f32>(0.97, 0.90, 0.92), light);
  // a soft vignette, the edges of the world dimmer
  let uv = px / u.size - vec2<f32>(0.5);
  c = c * (1.0 - 0.35 * dot(uv, uv));
  return vec4<f32>(c, 1.0);
}
