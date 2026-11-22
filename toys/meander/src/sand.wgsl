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
  let forest = 0.34 + 0.50 * smoothstep(0.40, 0.14, l);
  return mix(mix(forest, 0.26, lake), 0.0, warm);
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
  var d = deep(q);
  // the rim: what a finger clears it heaps beside the cleared edge
  let h = p.cell * 1.2;
  let around = (deep(q + vec2<f32>(h, 0.0)) + deep(q - vec2<f32>(h, 0.0)) +
                deep(q + vec2<f32>(0.0, h)) + deep(q - vec2<f32>(0.0, h))) * 0.25;
  let far = (deep(q + vec2<f32>(3.0 * h, 0.0)) + deep(q - vec2<f32>(3.0 * h, 0.0)) +
             deep(q + vec2<f32>(0.0, 3.0 * h)) + deep(q - vec2<f32>(0.0, 3.0 * h))) * 0.25;
  d += max(d - far, 0.0) * 1.4;
  // the grains: each a speck the light passes or doesn't; thin sand lets the lamp through between
  // them, deep sand barely at all; drifts of finer and coarser sand
  let g = hash(floor(q / (2.2 * px)));
  let clump = noise(q / (14.0 * px)) * 0.5 + noise(q / (55.0 * px)) * 0.5;
  let t = d * (0.75 + 0.5 * clump);
  // a grain's chance of stopping the light, as the sand deepens
  let blocked = smoothstep(g * 0.85, g * 0.85 + 0.22, t * 1.15);
  var through = mix(1.0, 0.0, blocked);
  // even where the glass is bare a few grains are left behind
  through *= 1.0 - step(0.985, g) * 0.6 * smoothstep(0.0, 0.05, 0.02 + t);
  // the lamp under the glass: hot in the middle, falling off to the corners
  let r = length((fc.xy / p.size - vec2<f32>(0.5, 0.5)) * vec2<f32>(0.8, 1.0));
  let lamp = 1.0 - 0.45 * r * r;
  // black-and-white film: the print's toe and shoulder, its grain
  let v = clamp(through * lamp, 0.0, 1.0);
  let print = smoothstep(0.02, 1.05, v) + (hash(floor(fc.xy / (1.5 * px)) + p.drawing) - 0.5) * 0.05;
  let tone = vec3<f32>(print) * vec3<f32>(1.0, 0.985, 0.96) + vec3<f32>(0.015);
  return vec4<f32>(tone, 1.0);
}
