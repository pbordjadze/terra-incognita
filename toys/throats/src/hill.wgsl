// The hillside as John Whitney filmed *Permutations* (UCLA, 1968): "architectures of color dots that
// develop pattern while displaying a kinetic rhythm", drawn "entirely off the black-and-white monitor" of
// an IBM 2250, the colour "added by editing with an optical printer" (SOURCES.md). Each rock is a dot on
// black at its place in the lattice, the lattice swaying on slow waves; its holder's throat is the colour
// the printer gives that dot's pass. A newcomer's dot swells in; the loser's shrinks away as a ring. Close
// in, each dot opens into a rosette that turns as its lizard turns. Outcrops and shrubs are left black.
//
// cells: one texel a rock (x: shown + 4*was + 16*look + 1024*ground (1 outcrop, 2 shrub), y: heading, z: seconds since it changed
// hands, w: where the newcomer ran in from, (fx+4) + 9*(fy+4)).

struct U {
  size: vec2<f32>,      // target, pixels
  center: vec2<f32>,    // world point at the centre, cells
  scale: f32,           // pixels a cell
  time: f32,
  ppp: f32,             // pixels a point
  night: f32,
  grid: vec2<f32>,      // the hill's size, cells
  _p: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var cells: texture_2d<f32>;
@group(0) @binding(2) var relief: texture_2d<f32>;     // r: outcrops' height, g: shrubs' (blurred masks)
@group(0) @binding(3) var lin: sampler;
fn rel(w: vec2<f32>) -> vec4<f32> { return textureSampleLevel(relief, lin, w / u.grid, 0.0); }

struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}

fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn hash22(p: vec2<f32>) -> vec2<f32> { return vec2(hash2(p), hash2(p + vec2(17.3, 41.9))); }
fn wrapc(c: vec2<i32>) -> vec2<i32> {
  let g = vec2<i32>(u.grid);
  return ((c % g) + g) % g;
}

fn cell(c: vec2<i32>) -> vec4<f32> { return textureLoad(cells, wrapc(c), 0); }


// the printer's colour for each throat's pass, as the toy has always named them
fn throat(k: i32) -> vec3<f32> {
  if (k == 1) { return vec3(1.0, 0.42, 0.06); }
  if (k == 2) { return vec3(0.18, 0.45, 1.0); }
  return vec3(1.0, 0.92, 0.20);
}

fn ease(x: f32) -> f32 { let c = clamp(x, 0.0, 1.0); return c * c * (3.0 - 2.0 * c); }

// A phosphor spot of radius r (cells) at distance d, px pixels a cell: a hard core and a little bloom.
fn spot(d: f32, r: f32, px: f32) -> f32 {
  let rr = max(r, 0.7 / px);
  let core = smoothstep(rr + 0.8 / px, rr - 0.8 / px, d);
  return core + 0.25 * exp(-max(d - rr, 0.0) * px / 2.5) * smoothstep(0.0, 1.5, rr * px);
}

@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let px = i.uv * u.size;
  // the film runs at 24 frames a second
  let frame = floor(u.time * 24.0);
  let t = frame / 24.0;
  let pxc = u.scale;
  var col = vec3<f32>(0.0);
  // each colour is its own pass through the printer, and each pass sits a little differently
  for (var k = 1; k <= 3; k++) {
    let off = (hash22(vec2<f32>(frame, f32(k) * 7.3)) - 0.5) * 0.9;
    let w = u.center + (px + off - u.size * 0.5) / u.scale;
    let ci = vec2<i32>(floor(w));
    var a = 0.0;
    for (var dy = -1; dy <= 1; dy++) {
      for (var dx = -1; dx <= 1; dx++) {
        let cc = ci + vec2(dx, dy);
        let c = cell(cc);
        let packed = i32(c.x);
        if (packed / 1024 != 0) { continue; }
        let held = packed % 4;
        let was = (packed / 4) % 4;
        if (held != k && was != k) { continue; }
        let bf = vec2<f32>(wrapc(cc));
        // the lattice, swaying on two slow waves
        let sway = vec2(sin(t * 0.55 + bf.y * 0.23), cos(t * 0.47 + bf.x * 0.19)) * 0.16;
        let at = vec2<f32>(cc) + 0.5 + sway;
        let q = w - at;
        let d = length(q);
        let since = c.z;
        // a pulse runs out across the field now and then, every dot it passes a little larger
        let beat = 0.5 + 0.5 * sin(t * 1.3 - length(bf - u.grid * 0.5) * 0.11);
        var r = 0.17 + 0.05 * beat;
        if (held == k) {
          // a newcomer swells in, past its size and back
          let g = ease(since / 0.5);
          r *= g * (1.0 + 0.35 * sin(3.14159 * clamp(since / 0.9, 0.0, 1.0)));
          if (pxc > 26.0) {
            // close in: a rosette, its six dots turned as the lizard turns
            let open = smoothstep(26.0, 40.0, pxc);
            var ro = spot(d, mix(r, r * 0.38, open), pxc);
            for (var j = 0; j < 6; j++) {
              let an = c.y + f32(j) * 1.0472;
              let sp = vec2(cos(an), sin(an)) * r * open;
              ro = max(ro, spot(length(q - sp), r * 0.24, pxc) * open);
            }
            a = max(a, ro);
          } else {
            a = max(a, spot(d, r, pxc));
          }
        } else if (since < 0.9) {
          // the one that lost: its dot opens into a ring and fades
          let f = ease(since / 0.9);
          let rr = 0.2 * (1.0 + 0.8 * f);
          let ring = smoothstep(1.2 / pxc, 0.0, abs(d - rr) - 0.03);
          a = max(a, ring * (1.0 - f));
        }
      }
    }
    col += throat(k) * a;
  }
  // the print's grain, on the light only: the black stays black
  let g = hash2(floor(px / 1.5) + vec2<f32>(frame * 0.37, frame * 0.11)) - 0.5;
  col *= 1.0 + g * 0.18;
  return vec4(clamp(col, vec3(0.0), vec3(1.0)), 1.0);
}
