// The Hoh's floor and its mist (a toy-kit field: the grid is the light reaching the floor, a cell of
// forest.rs's light grid each, in .x). Above the horizon, the stand going away into mist: trunks
// fainter the farther, the canopy dark overhead. Below, the moss carpet in clumps, brighter where a
// gap lets the light down, going into the same mist far off.
//   p0 = (horizon px, focal px, eye m, middle px)   p1 = (stand's half-width m, its nearest z m, cell m, mist drift)


fn light_at(x: f32, z: f32) -> f32 {
  let c = vec2<f32>((x + u.p1.x) / u.p1.z, (z - u.p1.y) / u.p1.z);
  let g = clamp(c, vec2<f32>(0.5), u.grid - vec2<f32>(0.5));
  return fld(g).x;
}

// Drawn as Bartosch made *L'Idée* (draw.rs, SOURCES.md): behind everything, glass lit from behind with
// lather spread on it ("halos, smoke and fog were made with lather spread on glass plates and lit from
// behind"); the stand going away is cut from the palest paper, the floor from card, each level one tone.
const CARDS = array<f32, 4>(0.012, 0.17, 0.30, 0.47);
fn level(z: f32) -> i32 {
  if (z < 18.0) { return 0; }
  if (z < 30.0) { return 1; }
  if (z < 50.0) { return 2; }
  return 3;
}

// The lather: milky, uneven, brushed on, drifting a little as the glass is moved.
fn lather(px: vec2<f32>, drift: f32) -> f32 {
  let q = px * 0.004 + vec2<f32>(drift * 0.01, 0.0);
  let n = noise2(q) * 0.55 + noise2(q * 2.7 + 5.0) * 0.3 + noise2(q * 9.0 + 1.0) * 0.15;
  return smoothstep(0.35, 0.8, n);
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let horizon = u.p0.x;
  let f = u.p0.y;
  let eye = u.p0.z;
  let cx = u.p0.w;
  let drift = u.p1.w;
  let grain = (hash2(floor(px / 2.0)) - 0.5) * 0.03;
  if (px.y < horizon + 1.0) {
    // the light behind the glass, brightest high up where the canopy opens, the lather over it
    let up = clamp((horizon - px.y) / horizon, 0.0, 1.0);
    var c = mix(0.72, 0.9, smoothstep(0.0, 0.7, up));
    c = c - 0.18 * lather(px, drift) * (1.0 - up * 0.4) + 0.08 * lather(px * 1.7 + 40.0, -drift);
    // far trunks: strips of the palest paper, the level farthest back
    for (var k = 0; k < 2; k++) {
      let kf = f32(k);
      let n = noise2(vec2<f32>(px.x * (0.010 + 0.012 * kf) + kf * 17.0, kf));
      let trunk = step(0.82 - 0.05 * kf, n);
      c = mix(c, CARDS[3] + 0.12 * (1.0 - kf), trunk);
    }
    return vec4<f32>(vec3<f32>(c + grain), 1.0);
  }
  let dy = px.y - horizon;
  let z = f * eye / dy;
  let x = (px.x - cx) * z / f;
  // the floor: card cut to the level its distance puts it on, its moss clumps cut into the edge of the
  // next level up (a lighter paper laid behind showing through)
  let w = vec2<f32>(x, z);
  // the card's edge torn, not ruled
  // the floor is paper, never the black card: the logs, ferns and seedlings are cut from that, in front
  let lv = max(1, level(z + 4.0 * (noise2(vec2<f32>(x * 0.4, z * 0.1)) - 0.5) * (1.0 + z * 0.05)));
  var c = CARDS[lv];
  let clump = noise2(w * 1.6) * 0.6 + noise2(w * 5.0 + vec2<f32>(1.3, 2.9)) * 0.4;
  if (lv > 1 && lv < 3 && clump > 0.76) { c = CARDS[lv + 1]; }
  // where a gap lets light down, lather glows on the glass behind the floor's card: through the paper
  // levels, faintly through the nearest
  let l = light_at(x, z);
  let through = select(1.0, 0.35, lv == 1);
  c = c + 0.28 * through * smoothstep(0.6, 1.0, l) * lather(px * 1.3, drift);
  return vec4<f32>(vec3<f32>(c + grain), 1.0);
}
