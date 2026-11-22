// Meander in sand on a lightbox, as Caroline Leaf made Sand, or Peter and the Wolf (Harvard, 1969;
// sand.rs has the method and its sources): beach sand spread on glass lit from below, pushed about by
// a finger a frame at a time, the camera above, black-and-white film. The prelude (toy-kit film.rs)
// declares p, src, lin, vs_main and hash. p.a = (boil: 1, or 0 with Reduce Motion; 0; 0; 0).
//
// `fs_surface`: how deep the sand lies, one texel a cell: the river is cleared to the glass, the lakes
// and old courses swept thin, the forest left deep, deeper where it is darker.
// `fs_main`: the light through it: the lamp under the glass, dimmed by the sand's depth grain by grain,
// the sand a finger pushed aside heaped in a dark rim along every cleared edge.

fn light(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.30, 0.55, 0.15)); }

fn depth_of(c: vec3<f32>) -> f32 {
  let l = light(c);
  // the river's water and its banks of wet sand (warm and light): cleared to the glass
  let warm = smoothstep(0.02, 0.12, c.r - c.b) * smoothstep(0.35, 0.55, l);
  // lakes (blue over the green): swept thin
  let lake = smoothstep(0.0, 0.06, c.b - c.g);
  // the forest: deep, a little less where it is lighter (old scrolls of the river, young growth)
  let forest = 0.17 + 0.48 * smoothstep(0.40, 0.12, l);
  return mix(mix(forest, 0.12, lake), 0.035, warm);
}

@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let c = floor(fc.xy);
  var sum = 0.0;
  for (var j = 0; j < 2; j++) {
    for (var i = 0; i < 2; i++) {
      let q = (c + (vec2<f32>(f32(i), f32(j)) + 0.5) / 2.0) * p.cell;
      sum += depth_of(textureLoad(src, vec2<i32>(clamp(q, vec2<f32>(0.0), p.size - 1.0)), 0).rgb);
    }
  }
  return vec4<f32>(sum * 0.25, 0.0, 0.0, 1.0);
}

fn deep(q: vec2<f32>) -> f32 { return textureSampleLevel(src, lin, q / p.size, 0.0).r; }

fn noise(q: vec2<f32>) -> f32 {
  let i = floor(q);
  let f = q - i;
  let u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
             mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let px = p.size.x / 1179.0;
  // a frame's sand never lies quite where the last frame's did: edges pushed again by hand
  let boil = (vec2<f32>(noise(fc.xy / (40.0 * px) + p.drawing * 1.7),
                        noise(fc.xy / (40.0 * px) + p.drawing * 2.3 + 9.0)) - 0.5) * 5.0 * px * p.a.x;
  let q = fc.xy + boil;
  // the cleared edge crumbles: sand slumps back into a channel unevenly, coarse and fine
  let crumb = (noise(q / (7.0 * px)) - 0.5) * 0.35 + (noise(q / (2.5 * px)) - 0.5) * 0.2;
  var d = max(deep(q) + crumb * smoothstep(0.02, 0.25, deep(q)) * (1.0 - smoothstep(0.5, 0.9, deep(q))), 0.0);
  // the banks: what a finger clears it heaps along the cleared edge, a ridge darker than the sand
  // behind it
  let h = p.cell * 1.5;
  var near = 0.0;
  for (var i = 0; i < 6; i++) {
    let a = f32(i) * 1.047;
    near += deep(q + vec2<f32>(cos(a), sin(a)) * h * 2.0);
  }
  near /= 6.0;
  let bank = max(d - near, 0.0) * 1.6;
  // in a cleared furrow a film of sand is left, thicker toward the walls and in drifts along it, so the
  // light is brightest where the finger pressed hardest
  let furrow = 1.0 - smoothstep(0.0, 0.12, deep(q));
  let film = (0.015 + 0.06 * noise(q / (34.0 * px)) + 0.03 * noise(q / (9.0 * px))) * furrow
           + smoothstep(0.02, 0.10, deep(q)) * 0.05;
  let t = d + bank + film;
  // the light through it: thin sand glows grey, deep sand goes black, grain by grain (each grain a
  // little denser or thinner, drifts of finer and coarser sand)
  let grain = hash(floor(q / (2.0 * px)));
  let clump = noise(q / (16.0 * px)) * 0.5 + noise(q / (60.0 * px)) * 0.5;
  let thick = t * (0.7 + 0.6 * clump) * (0.75 + 0.5 * grain);
  var through = exp(-thick * 4.5);
  // the lamp's light bleeds a little past every cleared edge into the sand's first grains
  var bleed = 0.0;
  for (var i = 0; i < 8; i++) {
    let a = f32(i) * 0.785 + 0.2;
    bleed += exp(-deep(q + vec2<f32>(cos(a), sin(a)) * 9.0 * px) * 6.0);
  }
  through = max(through, bleed / 8.0 * 0.35 * (0.6 + 0.4 * grain));
  // the lamp under the glass: hot in the middle, falling off to the corners
  let r = length((fc.xy / p.size - vec2<f32>(0.5, 0.5)) * vec2<f32>(0.8, 1.0));
  let lamp = 1.0 - 0.40 * r * r;
  // black-and-white film: the print's toe and shoulder, its grain
  let v = clamp(through * lamp, 0.0, 1.0);
  let print = smoothstep(0.0, 1.0, pow(v, 0.9)) + (hash(floor(fc.xy / (1.5 * px)) + p.drawing) - 0.5) * 0.04;
  let tone = vec3<f32>(print) * vec3<f32>(1.0, 0.985, 0.96) + vec3<f32>(0.012);
  return vec4<f32>(tone, 1.0);
}
