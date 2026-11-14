// The Hoh's floor and its mist (a toy-kit field: the grid is the light reaching the floor, a cell of
// forest.rs's light grid each, in .x). Above the horizon, the stand going away into mist: trunks
// fainter the farther, the canopy dark overhead. Below, the moss carpet in clumps, brighter where a
// gap lets the light down, going into the same mist far off.
//   p0 = (horizon px, focal px, eye m, middle px)   p1 = (stand's half-width m, its nearest z m, cell m, mist drift)

const MIST: vec3<f32> = vec3<f32>(0.56, 0.64, 0.55);

fn fogk(z: f32) -> f32 { return 1.0 - exp(-z / 45.0); }

fn light_at(x: f32, z: f32) -> f32 {
  let c = vec2<f32>((x + u.p1.x) / u.p1.z, (z - u.p1.y) / u.p1.z);
  let g = clamp(c, vec2<f32>(0.5), u.grid - vec2<f32>(0.5));
  return fld(g).x;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let horizon = u.p0.x;
  let f = u.p0.y;
  let eye = u.p0.z;
  let cx = u.p0.w;
  let drift = u.p1.w;
  if (px.y < horizon + 1.0) {
    // the mist beyond, brightening upward to the canopy's glow, and trunks going into it
    let up = clamp((horizon - px.y) / horizon, 0.0, 1.0);
    var c = mix(MIST, vec3<f32>(0.70, 0.76, 0.64), smoothstep(0.0, 0.8, up));
    c = mix(c, vec3<f32>(0.30, 0.40, 0.30), smoothstep(0.75, 1.0, up) * 0.6);
    for (var k = 0; k < 3; k++) {
      let kf = f32(k);
      let s = 0.010 + 0.012 * kf;
      let n = noise2(vec2<f32>(px.x * s + kf * 17.0, kf));
      let trunk = smoothstep(0.80 - 0.06 * kf, 0.86 - 0.06 * kf, n);
      let tone = mix(MIST, vec3<f32>(0.32, 0.36, 0.30), 0.18 + 0.16 * kf);
      c = mix(c, tone, trunk * (0.5 + 0.3 * kf));
    }
    // mist moving through, slow
    let m = noise2(vec2<f32>(px.x * 0.003 + drift * 0.05, px.y * 0.006));
    c = mix(c, vec3<f32>(0.78, 0.82, 0.74), m * 0.18 * (1.0 - up * 0.5));
    return vec4<f32>(c, 1.0);
  }
  let dy = px.y - horizon;
  let z = f * eye / dy;
  let x = (px.x - cx) * z / f;
  let w = vec2<f32>(x, z);
  // moss clumps, small and large; litter showing between
  let n1 = noise2(w * 1.6);
  let n2 = noise2(w * 0.45 + vec2<f32>(3.1, 7.7));
  let n3 = noise2(w * 5.0 + vec2<f32>(1.3, 2.9));
  let fine = clamp(fwidth(z) * 0.3, 0.0, 1.0);
  let clump = mix(n1 * 0.6 + n3 * 0.4, 0.5, fine);
  var c = mix(vec3<f32>(0.16, 0.25, 0.08), vec3<f32>(0.30, 0.42, 0.13), clump);
  c = mix(c, vec3<f32>(0.22, 0.17, 0.10), smoothstep(0.62, 0.8, n2) * 0.18);
  // the light let down: the floor brightens yellow-green where the canopy is open
  let l = light_at(x, z);
  c = c * (0.62 + 0.5 * l) + vec3<f32>(0.06, 0.07, 0.01) * smoothstep(0.4, 0.9, l);
  c = mix(c, MIST, fogk(z));
  return vec4<f32>(c, 1.0);
}
