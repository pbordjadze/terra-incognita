// The curtain of giant honeybees under the cliff (comb.rs), face on: the overhang along the top, the comb
// hanging from it covered in bees, heads up; a flip lifts the abdomen toward the eye, so it shortens and
// shows its light side. The hornet hovers in front.
// A field (toy-kit field.rs): per cell (abdomen up 0..1, a bee 0/1, flash, -).
// u.p0 = (CELL cm, the hornet's x, y cm, its heading); u.p1 = (comb centre x, top, half width, depth).

fn rot(v: vec2<f32>, a: f32) -> vec2<f32> {
  let c = cos(a); let s = sin(a);
  return vec2<f32>(c * v.x + s * v.y, -s * v.x + c * v.y);
}

// Drawn as Larry Cuba drew *Calculated Movements* (1985) on a Datamax UV-1 in Zgrass (SOURCES.md): a raster
// of 320 by 201 pixels at two bits a pixel, so four colours, "white, black, light grey and dark gray", solid
// areas rather than dots of light, and figures like ribbons "against a middle gray background". Each pixel
// of that raster is one sample of the scene, no smoothing: 320 across the phone, square, as many down as
// the screen holds.
const C_BLACK = vec3<f32>(0.03, 0.03, 0.03);
const C_DARK = vec3<f32>(0.33, 0.33, 0.33);
const C_LIGHT = vec3<f32>(0.62, 0.62, 0.62);
const C_WHITE = vec3<f32>(0.94, 0.94, 0.94);
const ACROSS = 320.0;

// The bees of two rows are one ribbon across the comb, a stretch of it a bee. A hanging bee is the ribbon
// flat to the eye, its dark face; a flip turns that stretch over, narrowing to its edge and opening again
// on its white face; between bees the ribbon's turn runs smoothly on, as a twisted ribbon's does.
// 0 none, 2 dark, 3 white.
fn ribbon(w: vec2<f32>) -> i32 {
  let cell = u.p0.x;
  // a ribbon to every two rows of bees, sagging and slowly swaying along its length
  let band = 2.0 * cell;
  let x0 = w.x / cell;
  let lift = 0.16 * sin(x0 * 0.21 + u.time * 0.5 + w.y / band * 1.7) + 0.08 * sin(x0 * 0.53 - u.time * 0.8);
  let yb = w.y / band - lift;
  let kb = floor(yb);
  let ky = 2.0 * kb;
  let row = (kb % 2.0) * 0.5;
  let x = x0 - row;
  if (x < 0.0 || ky < 0.0 || x >= u.grid.x || ky + 1.0 >= u.grid.y + 1.0) { return 0; }
  // sampled between the bees' centres along the row, so the turn is continuous; the two rows' bees
  // turn it together
  let f0 = fld(vec2<f32>(x, ky + 0.5));
  let f1 = fld(vec2<f32>(x, ky + 1.5));
  if (max(f0.y, f1.y) < 0.5) { return 0; }
  let up = max(max(f0.x, f0.z * 0.8) * step(0.5, f0.y), max(f1.x, f1.z * 0.8) * step(0.5, f1.y));
  let turn = clamp(up, 0.0, 1.0) * 3.14159;
  // a small notch where one bee meets the next, so a band at rest is still bees
  let notch = 0.07 * (1.0 - smoothstep(0.0, 0.18, abs(fract(x) - 0.5) - 0.32));
  let half = (0.30 - notch) * abs(cos(turn)) + 0.03;
  let fy = fract(yb) - 0.5;
  if (abs(fy) > half) { return 0; }
  if (turn > 1.5708) { return 3; }
  return 2;
}

// The hornet, a ribbon of three white segments with black bands, its wings two light strokes.
fn wasp(world: vec2<f32>) -> i32 {
  // drawn twice its size, a figure the raster can hold
  let q = rot(world - u.p0.yz, u.p0.w) / 4.0;
  let beat = 0.65 + 0.35 * sin(u.time * 90.0);
  for (var sd = -1.0; sd <= 1.0; sd = sd + 2.0) {
    let w = rot(q - vec2<f32>(0.12 * sd, -0.05), 1.1 * beat * sd) - vec2<f32>(0.0, 0.42);
    if (abs(w.x) < 0.07 && abs(w.y) < 0.42) { return 1; }
  }
  let segs = array<vec3<f32>, 3>(vec3<f32>(-0.4, 0.17, 0.13), vec3<f32>(-0.08, 0.2, 0.2), vec3<f32>(0.45, 0.22, 0.48));
  for (var i = 0; i < 3; i = i + 1) {
    let sg = segs[i];
    let d = q - vec2<f32>(0.0, sg.x);
    if (abs(d.x) < sg.y && abs(d.y) < sg.z) {
      if (i == 2 && fract((q.y - 0.1) * 3.2) < 0.5) { return 4; }
      return 3;
    }
  }
  return 0;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let cell = u.p0.x;
  // the raster's pixel, and the one point of the scene it samples
  let pix = u.size.x / ACROSS;
  let rp = (floor(px / pix) + 0.5) * pix;
  let w = world + (rp - px) / u.scale;
  // the ground: middle grey; the overhang black along the top
  var c = 1;
  let shp = u.p1;
  let b = ribbon(w);
  if (b != 0) { c = b; }
  // the overhang: its lip stepped, as a program draws a rock, in solid black
  let step_x = floor(w.x / 4.0);
  let lip = shp.y + (hash2(vec2<f32>(step_x, 1.0)) - 0.5) * 2.5 + 0.8;
  if (w.y < lip) { c = 0; }
  let h = wasp(w);
  if (h == 1) { c = 1; }
  if (h == 3) { c = 3; }
  if (h == 4) { c = 0; }
  var col = C_LIGHT;
  if (c == 0) { col = C_BLACK; }
  if (c == 2) { col = C_DARK; }
  if (c == 3) { col = C_WHITE; }
  return vec4<f32>(col, 1.0);
}
