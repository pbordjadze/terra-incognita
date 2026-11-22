// Elkmont engraved on black film, as Norman McLaren made Blinkity Blank (National Film Board of Canada,
// Montreal, 1955; engraved.rs has the method and its sources): nothing but black leader, and light only
// where a frame was scratched through its emulsion, a frame at a time, some frames left blank. The
// prelude (toy-kit film.rs) declares p, src, lin, vs_main and hash. p.a = (lines: how many frames in a
// run carry the woods' lines (0..1); steady: 1 with Reduce Motion; exposure; 0).
//
// `fs_surface`: the woods' light, one texel a cell, lifted (a square root) so the dark trunks have edges.
// `fs_main`: the frame: black leader with its dust; the fireflies' light scratched clean through as
// ragged points; the trunks and ferns as thin scratches along their edges, cut anew each
// frame so they boil, on only some frames (all of them while a burst lights the woods); every
// scratch coloured with a dye laid on unevenly.

fn light(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.30, 0.55, 0.15)); }

@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let c = floor(fc.xy);
  var sum = 0.0;
  var hi = 0.0;
  var green = 0.0;
  for (var j = 0; j < 2; j++) {
    for (var i = 0; i < 2; i++) {
      let q = (c + (vec2<f32>(f32(i), f32(j)) + 0.5) / 2.0) * p.cell;
      let rgb = textureLoad(src, vec2<i32>(clamp(q, vec2<f32>(0.0), p.size - 1.0)), 0).rgb;
      let l = light(rgb);
      sum += l;
      hi = max(hi, l);
      green = max(green, rgb.g - rgb.b);
    }
  }
  var v = sqrt(clamp(mix(sum * 0.25, hi, 0.5) * p.a.z, 0.0, 1.0));
  // a star is not a firefly: a light that isn't yellow-green stays under the fireflies' level, and is
  // scratched only as the woods are
  v = min(v, mix(0.38, 1.0, smoothstep(0.04, 0.12, green)));
  return vec4<f32>(v, 0.0, 0.0, 1.0);
}

fn at(q: vec2<f32>) -> f32 { return textureSampleLevel(src, lin, q / p.size, 0.0).r; }

// a hand's wobble: each frame's scratches cut a little differently
fn wobble(q: vec2<f32>, s: f32) -> vec2<f32> {
  let k = floor(q / (p.cell * 6.0));
  let f = p.drawing * (1.0 - p.a.y);
  return (vec2<f32>(hash(k + f * 1.37), hash(k.yx + f * 2.11 + 5.0)) - 0.5) * s;
}

@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let px = p.size.x / 1179.0;
  let q = fc.xy + wobble(fc.xy, 3.0 * px);
  let v = at(q);
  // the fireflies: a point brighter than the woods round it is scratched clean through, ragged at
  // the edge (a broad glow is not)
  let rag = (hash(floor(q / (1.5 * px)) + p.drawing * (1.0 - p.a.y)) - 0.5) * 0.06;
  // (at two sizes: a far firefly's point, and a near one's disc, solid to its middle)
  var ring = 0.0;
  var wide = 0.0;
  for (var d = 0; d < 8; d++) {
    let a = f32(d) * 0.785;
    ring += at(q + vec2<f32>(cos(a), sin(a)) * p.cell * 5.0);
    wide += at(q + vec2<f32>(cos(a), sin(a)) * p.cell * 12.0);
  }
  let peak = v - min(ring, wide) / 8.0;
  let fly = smoothstep(0.13, 0.15, peak + rag) * step(0.42, v);
  // the woods: thin scratches along the edges of the trunks and ferns, on some frames only
  let h = p.cell * 0.8;
  let gx = at(q + vec2<f32>(h, 0.0)) - at(q - vec2<f32>(h, 0.0));
  let gy = at(q + vec2<f32>(0.0, h)) - at(q - vec2<f32>(0.0, h));
  let edge = smoothstep(0.05, 0.11, length(vec2<f32>(gx, gy)));
  // runs of frames: a run of three, cut or blank as the run's dice say
  let run = floor(p.drawing / 3.0);
  let shown = max(step(1.0 - p.a.x, hash(vec2<f32>(run, 7.7))), p.a.y);
  let lines = edge * max(shown, smoothstep(0.30, 0.5, v)) * (0.45 + 0.4 * smoothstep(0.2, 0.5, v)) * (0.6 + 0.4 * hash(floor(q / (4.0 * px)) + p.drawing));
  // the dyes, brushed on by hand: the fireflies' a yellow-green, the woods' a blue that wanders to violet
  let blot = hash(floor(q / (90.0 * px)) + floor(p.drawing / 2.0) * 0.3);
  let fly_dye = mix(vec3<f32>(0.80, 1.00, 0.30), vec3<f32>(1.00, 0.92, 0.45), blot);
  let wood_dye = mix(vec3<f32>(0.30, 0.55, 1.00), vec3<f32>(0.60, 0.40, 1.00), blot);
  // the hottest middle of a scratch is clear film: the dye thins to white
  let core = smoothstep(0.80, 0.97, v) * fly;
  var c = fly_dye * fly + wood_dye * lines * (1.0 - fly);
  c = mix(c, vec3<f32>(1.0, 1.0, 0.92), core);
  // the leader: black, a few flecks of dust and a hairline scratch that runs down the reel
  let dust = step(0.9993, hash(floor(fc.xy / (3.0 * px)) + p.drawing * 3.1)) * 0.5;
  let lane = hash(vec2<f32>(floor(p.drawing / 24.0), 1.0)) * p.size.x;
  let off = (fc.x - lane) / (1.2 * px); // squared by hand: pow of a negative base is undefined
  let hair = exp(-off * off) * 0.12 * (1.0 - p.a.y);
  let base = vec3<f32>(0.012, 0.011, 0.010);
  return vec4<f32>(base + c + vec3<f32>(dust + hair), 1.0);
}
