// Bats on a pinscreen (Alexandre Alexeieff and Claire Parker, Paris, 1933; pinscreen.rs has the
// method and its sources). Two passes over the dusk as drawn; the prelude (toy-kit film.rs) declares
// p, src, lin, vs_main and hash. p.a = (exposure: how much the dark sky is lifted, so the bats stay
// darker than it; flicker: 1, or 0 with Reduce Motion; 0; 0).
//
// `fs_surface`: one texel a pin, holding how far out it is pushed (0 flush, 1 all the way): the
// darker the picture beneath it, the further.
// `fs_main`: the board, white, lit from the upper left at a raking angle; each pin throws a shadow
// down and to the right as long as it stands out, the shadows of neighbours running together into
// greys; a pin half out catches the lamp on its head.

const LONGEST: f32 = 4.6;   // a pin pushed all the way out: its shadow's length, in pins
const DIR: vec2<f32> = vec2<f32>(0.894, 0.447);   // the shadows' way across the board (down-right)

fn light(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.30, 0.55, 0.15)); }

// ---- the pins ----
@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let c = floor(fc.xy);
  var sum = 0.0;
  var lo = 1.0;
  for (var j = 0; j < 3; j++) {
    for (var i = 0; i < 3; i++) {
      // a little past the pin's own square, so a bat a pixel wide still pushes its pin
      // (odd rows sit half a pin to the right: read beneath where the pin stands)
      let row = vec2<f32>(0.5 * (c.y - 2.0 * floor(c.y * 0.5)), 0.0);
      let q = (c + row + (vec2<f32>(f32(i), f32(j)) + 0.5) / 3.0 * 2.6 - 0.8) * p.cell;
      let l = light(textureLoad(src, vec2<i32>(clamp(q, vec2<f32>(0.0), p.size - 1.0)), 0).rgb);
      sum += l;
      lo = min(lo, l);
    }
  }
  // the board's tones: most of the dusk deep, the light only where the sun went down
  // and the animator's lamp, pooled low over the cave where the sun went down, the rest of the board deep
  let uv = c * p.cell / p.size;
  let lamp = 0.32 + 1.0 * exp(-(pow((uv.x - 0.3) / 0.55, 2.0) + pow((uv.y - 0.78) / 0.24, 2.0)));
  // (rolled off softly toward flush, so the glow has no edge)
  let x = mix(sum / 9.0, lo, 0.55) * p.a.x * lamp;
  let l = pow(1.0 - exp(-x * 2.6), 1.5);
  // a little unevenness: no two pins pushed quite alike
  let out = clamp(pow(1.0 - l, 0.8) + (hash(c * 0.37 + 3.1) - 0.5) * 0.10, 0.0, 1.0);
  return vec4<f32>(out, 0.0, 0.0, 1.0);
}

// where pin (i, j) stands, in pins: rows offset by half, as the screens' were
fn pin_at(i: f32, j: f32) -> vec2<f32> {
  return vec2<f32>(i + 0.5 + 0.5 * (j - 2.0 * floor(j * 0.5)), j + 0.5);
}

fn height(i: f32, j: f32) -> f32 {
  let g = vec2<i32>(clamp(vec2<f32>(i, j), vec2<f32>(0.0), p.grid - 1.0));
  return textureLoad(src, g, 0).r;
}

// ---- the board ----
@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let q = fc.xy / p.cell;          // where this pixel is, in pins
  let base = floor(q);
  var shade = 0.0;
  var head = 0.0;
  for (var dj = -4; dj <= 0; dj++) {
    for (var di = -5; di <= 1; di++) {
      let i = base.x + f32(di);
      let j = base.y + f32(dj);
      let h = height(i, j);
      let v = q - pin_at(i, j);
      // each pin leans a little its own way, so the shadows never weave into cloth
      let tilt = (hash(vec2<f32>(i, j) * 0.71) - 0.5) * 0.35;
      let dir = normalize(DIR + vec2<f32>(-DIR.y, DIR.x) * tilt);
      let along = dot(v, dir);
      let across = v.x * dir.y - v.y * dir.x;
      let len = h * LONGEST;
      // the shadow: a thin dark blade from the pin's foot, softening and paling toward its end
      // in the darks the blades widen and run together into velvet
      let w = 0.24 + 0.16 * clamp(along / LONGEST, 0.0, 1.0) + 0.35 * h * h;
      let on = smoothstep(-0.15, 0.1, along) * (1.0 - smoothstep(len - 0.5, len + 0.2, along));
      shade += on * exp(-across * across / (w * w)) * (0.62 - 0.06 * along) * step(0.03, h);
      // the head: a point of light on a pin half out
      let r2 = dot(v, v);
      head = max(head, exp(-r2 * 30.0) * clamp(1.0 - abs(h - 0.5) * 3.0, 0.0, 1.0));
    }
  }
  // seen from the camera's distance the blades run together into a smooth tone; the pins show
  // where the tone changes, at the edges of things
  let hs = textureSampleLevel(src, lin, q / p.grid, 0.0).r;
  let edge = clamp(length(vec2<f32>(dpdx(hs), dpdy(hs))) * 6.0, 0.0, 1.0);
  let smooth_lum = 0.94 * exp(-hs * hs * 3.4);
  var lum = mix(smooth_lum, 0.94 * exp(-shade * 1.5), 0.3 + 0.6 * edge);
  lum += head * 0.30 * (1.0 - lum);
  // the lamp falls off across the board, from its upper left
  let uv = fc.xy / p.size;
  lum *= 1.0 - 0.20 * (pow(uv.x - 0.15, 2.0) + pow(uv.y - 0.3, 2.0));
  // the camera's film: its grain, and the lamp's small flicker from one exposure to the next
  let d = p.drawing;
  lum *= 1.0 + p.a.y * (hash(vec2<f32>(fract(d * 0.0173) * 61.0, 4.7)) - 0.5) * 0.04;
  lum += (hash(floor(fc.xy) + vec2<f32>(fract(d * 0.1373) * 97.0, fract(d * 0.0311) * 89.0)) - 0.5) * 0.05;
  lum = pow(clamp(lum, 0.0, 1.0), 1.35);
  // a silver print: black and white
  let col = mix(vec3<f32>(0.005, 0.005, 0.007), vec3<f32>(0.96, 0.955, 0.94), lum);
  return vec4<f32>(col, 1.0);
}
