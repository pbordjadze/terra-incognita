// The beach at low tide, from above, in centimetres (`world`): wave ripples left by the last tide, the
// trenches the crabs have sifted (the ripples gone), a finger's furrows, the wet, and the swash when the
// tide comes. A field (toy-kit field.rs): the grid's cells
// are 0.25 cm; per cell (worked, wet, furrow, 0). u.p0 = (the water's edge in cm or -1, the beach's
// height in cm, the cell in cm, 0).

// Drawn as beads on a dark ground, after Ishu Patel's *Bead Game* (beads.rs; SOURCES.md): the sand is not
// painted but laid in beads, one where a ripple's crest stands, none where the crabs have sifted it away or
// a finger has drawn, darker and bluer where it is wet; the swash is a sheet of turquoise and white beads.

const GROUND = vec3<f32>(0.035, 0.03, 0.03);
const SAND_BEAD = vec3<f32>(0.72, 0.60, 0.42);
const SEA_BEAD = vec3<f32>(0.10, 0.62, 0.66);
const FOAM_BEAD = vec3<f32>(0.92, 0.94, 0.92);
// beads this far apart, centimetres
const PITCH = 0.34;

// The bead lattice (rows offset by half, as beads pack): this point's bead centre and its id.
fn lattice(world: vec2<f32>) -> vec4<f32> {
  let rh = PITCH * 0.866;
  let row = floor(world.y / rh);
  let off = (row % 2.0) * 0.5;
  let col = floor(world.x / PITCH - off);
  let c = vec2<f32>((col + 0.5 + off) * PITCH, (row + 0.5) * rh);
  var best = vec4<f32>(c, col, row);
  var bd = length(world - c);
  for (var dy = -1.0; dy <= 1.0; dy = dy + 2.0) {
    let r2 = row + dy;
    let o2 = (abs(r2) % 2.0) * 0.5;
    for (var dx = 0.0; dx <= 1.0; dx = dx + 1.0) {
      let c2col = floor(world.x / PITCH - o2) + dx - 0.0;
      let c2 = vec2<f32>((c2col + 0.5 + o2) * PITCH, (r2 + 0.5) * rh);
      let d2 = length(world - c2);
      if (d2 < bd) { bd = d2; best = vec4<f32>(c2, c2col, r2); }
    }
  }
  return best;
}

// A glass bead of radius r at offset q (cm): its body, the shade, the threading hole, the light.
fn shade_bead(q: vec2<f32>, r: f32, col: vec3<f32>, aa: f32) -> vec4<f32> {
  let d = length(q);
  let cover = 1.0 - smoothstep(r - aa, r + aa, d);
  if (cover <= 0.0) { return vec4<f32>(0.0); }
  let lit = clamp(1.0 - length(q / r + vec2<f32>(0.35, 0.35)) * 0.7, 0.0, 1.0);
  var c = col * (0.45 + 0.75 * lit);
  c = mix(c, vec3<f32>(0.0), 1.0 - smoothstep(r * 0.2 - aa, r * 0.2 + aa, d));
  let spec = 1.0 - smoothstep(r * 0.1, r * 0.22, length(q + vec2<f32>(r * 0.42, r * 0.42)));
  c = mix(c, vec3<f32>(0.95), spec * 0.85);
  return vec4<f32>(c, cover);
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let cellw = u.p0.z;
  let aa = 0.8 / u.scale;
  let l = lattice(world);
  let bc = l.xy;
  let id = l.zw;
  let c = bc / cellw;
  let f = fld(c);
  let worked = f.x;
  let wet = f.y;
  // the last tide's ripples, ~1.2 cm apart, wavering, strong in patches
  let wv = bc.y * 4.6 + 3.2 * noise2(bc * 0.13) + 1.4 * noise2(bc * 0.41 + 7.0) + 0.6 * sin(bc.x * 0.21);
  let patchy = 0.35 + 0.65 * smoothstep(0.3, 0.8, noise2(mat2x2<f32>(0.8, 0.6, -0.6, 0.8) * bc * 0.09));
  let crest = smoothstep(0.35, 0.9, cos(wv) * 0.5 + 0.5) * patchy;
  let h = hash2(id);
  // a bead where a crest stands on untouched sand; none on sifted sand or in a furrow
  var show = step(h, crest * 0.95) * (1.0 - step(0.35, worked)) * (1.0 - step(0.3, f.z));
  var col = SAND_BEAD * (0.8 + 0.4 * hash2(id + 3.1));
  col = mix(col, col * vec3<f32>(0.55, 0.62, 0.72), smoothstep(0.3, 1.0, wet));
  // the swash: a sheet of sea beads, white along its edge
  let front = u.p0.x;
  if (front > -50.0 && bc.y > front - 1.2) {
    let edge = front + 0.6 * sin(bc.x * 0.35 + u.time * 1.3) + 0.8 * noise2(vec2<f32>(bc.x * 0.15, u.time * 0.4));
    if (bc.y > edge) {
      let foam = exp(-(bc.y - edge) * 1.1) * smoothstep(0.3, 0.7, noise2(bc * 1.7 + vec2<f32>(0.0, u.time)));
      // the sea's beads laid loose, darker and lighter, gaps between
      show = step(0.35, h);
      col = mix(SEA_BEAD * (0.55 + 0.8 * hash2(id + 7.7)) + vec3<f32>(0.0, 0.0, 0.25) * step(0.7, hash2(id + 2.2)), FOAM_BEAD, step(0.45, foam));
    }
  }
  var out = GROUND;
  if (show > 0.5) {
    let b = shade_bead(world - bc, PITCH * 0.42, col, aa);
    out = mix(out, b.rgb, b.a);
  }
  return vec4<f32>(out, 1.0);
}
