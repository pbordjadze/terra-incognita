// Huddle as a printout: the huddle typed by a line printer, a sheet a drawing, as Kitty (Koshechka,
// Moscow State University, 1968) was printed on the BESM-4's ACPU-128 and photographed sheet by sheet
// (printer.rs has the method and its sources). The prelude (toy-kit film.rs) declares p, src, lin,
// vs_main and hash; printer_type.wgsl (appended) the type. p.a = (sheet jitter: 1, or 0 with Reduce
// Motion; 0; 0; 0).
//
// `fs_surface`: one texel a character place (the grid's first rows only: a place is ROW times as tall
// as it is wide), holding what is struck there: 0 nothing, 1..TYPE_N one character (lightest first),
// above that the darkest struck twice, a second character over the first.
// `fs_main`: the sheet: paper, the type inked through a worn ribbon, each character hammered a little
// early or late on the drum so it stands a little high or low, the whole sheet laid a little askew
// under the camera.

const ROW: f32 = 1.6;          // a printer's place: 10 to the inch across, 6 lines to the inch down
const PAPER: vec3<f32> = vec3<f32>(0.925, 0.910, 0.865);
const INKC: vec3<f32> = vec3<f32>(0.13, 0.12, 0.20);

fn light(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.30, 0.55, 0.15)); }

fn sheet_shift() -> vec2<f32> {
  let s = p.drawing;
  return (vec2<f32>(hash(vec2<f32>(s, 1.7)), hash(vec2<f32>(s, 9.3))) - 0.5) * 3.0 * p.a.x;
}

// ---- what is struck at each place ----
@fragment fn fs_surface(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let c = floor(fc.xy);
  let cw = p.cell;
  let ch = p.cell * ROW;
  if (c.y * ch > p.size.y + ch) { return vec4<f32>(0.0); }
  var sum = 0.0;
  var lo = 1.0;
  for (var j = 0; j < 4; j++) {
    for (var i = 0; i < 4; i++) {
      let q = (c + (vec2<f32>(f32(i), f32(j)) + 0.5) / 4.0) * vec2<f32>(cw, ch);
      let l = light(textureLoad(src, vec2<i32>(clamp(q, vec2<f32>(0.0), p.size - 1.0)), 0).rgb);
      sum += l;
      lo = min(lo, l);
    }
  }
  // and the light round the place, a bird's width away: a bird's back stands out lighter than the
  // dark between birds, so the program sharpens the difference and each bird prints as a body
  var round = 0.0;
  for (var k = 0; k < 8; k++) {
    let a = f32(k) * 0.785;
    let q = (c + 0.5) * vec2<f32>(cw, ch) + vec2<f32>(cos(a), sin(a)) * cw * 1.6;
    round += light(textureLoad(src, vec2<i32>(clamp(q, vec2<f32>(0.0), p.size - 1.0)), 0).rgb);
  }
  // a bird anywhere in the place is struck there: the darkest point counts most
  let l0 = mix(sum / 16.0, lo, 0.35);
  let l = l0 + (l0 - round / 8.0) * 1.6 * smoothstep(0.45, 0.25, l0);
  // the ice prints as a few light marks, the birds solid; the snow, lighter than the ice, leaves gaps
  let d = pow(clamp((0.70 - l) / 0.64, 0.0, 1.0), 1.4);
  let n = f32(TYPE_N);
  // a little noise in the choosing, so a tone is a mix of two characters, as the program's would be
  let k = floor(d * (n + 2.0) + hash(c * 0.73 + 0.5) * 0.9);
  return vec4<f32>(clamp(k, 0.0, n + 1.0) / 255.0, 0.0, 0.0, 1.0);
}

fn type_px(g: i32, x: i32, y: i32) -> f32 {
  if (x < 0 || y < 0 || x >= TYPE_W || y >= TYPE_H) { return 0.0; }
  let i = y * TYPE_W + x;
  let w = TYPE[g * TYPE_WORDS + i / 8];
  return f32((w >> (4u * u32(i % 8))) & 15u) / 15.0;
}

// glyph g at uv (0..1 across its box), filtered
fn type_at(g: i32, uv: vec2<f32>) -> f32 {
  let q = uv * vec2<f32>(f32(TYPE_W), f32(TYPE_H)) - 0.5;
  let b = floor(q);
  let f = q - b;
  let x = i32(b.x);
  let y = i32(b.y);
  let a = mix(type_px(g, x, y), type_px(g, x + 1, y), f.x);
  let c = mix(type_px(g, x, y + 1), type_px(g, x + 1, y + 1), f.x);
  return mix(a, c, f.y);
}

// ---- the sheet ----
@fragment fn fs_main(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let cw = p.cell;
  let ch = p.cell * ROW;
  let xy = fc.xy + sheet_shift();
  let col = floor(xy.x / cw);
  // each character on the drum strikes a little early or late, the same in a column all down a line
  var ink = 0.0;
  for (var dj = -1; dj <= 1; dj++) {
    let line = floor(xy.y / ch) + f32(dj);
    let fly = (hash(vec2<f32>(col * 1.3, line * 0.17 + 4.0)) - 0.5) * 0.22 * ch;
    let cell = vec2<i32>(i32(col), i32(line));
    if (cell.x < 0 || cell.y < 0 || f32(cell.x) >= p.grid.x || f32(cell.y) >= p.grid.y) { continue; }
    let k = i32(round(textureLoad(src, cell, 0).r * 255.0));
    if (k == 0) { continue; }
    // the type's box in its place: a gap between letters and between lines
    let local = vec2<f32>(xy.x - col * cw, xy.y - line * ch - fly);
    let box = vec2<f32>(cw * 0.86, ch * 0.80);
    let uv = (local - vec2<f32>(cw * 0.07, ch * 0.10)) / box;
    if (any(uv < vec2<f32>(-0.1)) || any(uv > vec2<f32>(1.1))) { continue; }
    let n = TYPE_N;
    var v = 0.0;
    if (k <= n) { v = type_at(k - 1, uv); }
    if (k > n) {
      // struck twice: the line printed again without a feed, a second character over the first
      v = max(type_at(n - 2, uv), type_at(n - 1, uv + vec2<f32>(0.05, 0.0)));
    }
    // the ribbon: worn unevenly along its length, and its weave in every stroke
    let ribbon = 0.72 + 0.28 * hash(vec2<f32>(line * 0.31, floor(col / 9.0) + p.drawing * 0.07));
    let weave = 0.80 + 0.20 * hash(floor(xy * 0.9));
    ink = max(ink, smoothstep(0.10, 0.55, v) * ribbon * weave);
  }
  // the paper: its fibre, and the sheet's own brightness under the lamp
  let fibre = (hash(floor(xy * 0.5) + 7.0) - 0.5) * 0.025;
  let lamp = 1.0 - 0.06 * length((fc.xy / p.size - vec2<f32>(0.5, 0.45)) * vec2<f32>(1.0, 0.8));
  let paper = PAPER * (lamp + fibre + (hash(vec2<f32>(p.drawing, 3.0)) - 0.5) * 0.02 * p.a.x);
  return vec4<f32>(mix(paper, INKC, ink * 0.92), 1.0);
}
