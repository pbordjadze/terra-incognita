// The beach at low tide, from above, in centimetres (`world`): pale Andaman sand, wave ripples left by
// the last tide, the trenches the crabs have sifted (drier, lighter, the ripples gone), a finger's
// furrows, the wet, and the swash when the tide comes. A field (toy-kit field.rs): the grid's cells
// are 0.25 cm; per cell (worked, wet, furrow, 0). u.p0 = (the water's edge in cm or -1, the beach's
// height in cm, the cell in cm, 0).

fn grain(p: vec2<f32>) -> f32 {
  return noise2(p * 38.0) * 0.5 + noise2(p * 11.0) * 0.35 + noise2(p * 3.1) * 0.15;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let cellw = u.p0.z;
  let c = world / cellw;
  // four taps, a cell apart: soft edges on a coarse grid
  let f = (fld(c + vec2<f32>(0.5, 0.5)) + fld(c + vec2<f32>(-0.5, 0.5)) + fld(c + vec2<f32>(0.5, -0.5)) + fld(c + vec2<f32>(-0.5, -0.5))) * 0.25;
  let worked = f.x;
  let wet = f.y;
  // the furrow's slope, for its light (the sun from the upper left)
  let e = 0.9;
  let gx = fld(c + vec2<f32>(e, 0.0)).z - fld(c - vec2<f32>(e, 0.0)).z;
  let gy = fld(c + vec2<f32>(0.0, e)).z - fld(c - vec2<f32>(0.0, e)).z;
  // wave ripples, ~1.2 cm apart, wavering; gone where the crabs have sifted
  let wv = world.y * 4.6 + 3.2 * noise2(world * 0.13) + 1.4 * noise2(world * 0.41 + 7.0) + 0.6 * sin(world.x * 0.21);
  // (strong in patches, faint between, as the last tide left them)
  let patchy = smoothstep(0.3, 0.8, noise2(mat2x2<f32>(0.8, 0.6, -0.6, 0.8) * world * 0.09));
  let rslope = cos(wv) * (1.0 - worked) * patchy;
  var col = vec3<f32>(0.86, 0.80, 0.69);
  let g = grain(world);
  col = col * (0.90 + 0.16 * g);
  // dark mineral specks and bits of shell
  // (round, each in its own little cell)
  let q9 = world * 9.0;
  let speck = step(0.985, hash2(floor(q9))) * (1.0 - smoothstep(0.18, 0.32, length(fract(q9) - 0.5)));
  col = mix(col, vec3<f32>(0.38, 0.34, 0.30), speck * 0.7);
  let q6 = world * 6.0;
  let shell = step(0.992, hash2(floor(q6) + 9.0)) * (1.0 - smoothstep(0.2, 0.36, length(fract(q6) - 0.5)));
  col = mix(col, vec3<f32>(0.97, 0.95, 0.90), shell * 0.6);
  // light on the ripples and the furrow's sides
  col = col * (1.0 + 0.035 * rslope * (1.0 - wet * 0.5) - (gx + gy) * 0.22);
  col = col * (1.0 - 0.16 * smoothstep(0.2, 1.0, fld(c).z));
  // the sifted sand: drier, a little lighter, rougher
  col = mix(col, col * vec3<f32>(1.05, 1.04, 1.02) * (0.96 + 0.08 * noise2(world * 22.0)), worked * 0.85);
  // wet sand: darker and cooler, with a sheen
  let wk = smoothstep(0.3, 1.0, wet) * (1.0 - worked * 0.6);
  col = mix(col, col * vec3<f32>(0.80, 0.76, 0.70), wk);
  let rs = mat2x2<f32>(0.8, 0.6, -0.6, 0.8) * world;
  let sheen = noise2(rs * 0.35 + vec2<f32>(u.time * 0.03, 0.0)) * 0.65 + noise2(rs * 1.1 + 4.0) * 0.35;
  col = col + vec3<f32>(0.05, 0.06, 0.07) * wk * smoothstep(0.35, 0.95, sheen);
  // just-soaked sand shines
  col = col + vec3<f32>(0.05, 0.05, 0.05) * smoothstep(0.85, 1.0, wet) * (1.0 - worked);
  // the swash: thin water over the sand, foam at its edge
  let front = u.p0.x;
  if (front > -50.0 && world.y > front - 1.2) {
    let depth = clamp((world.y - front) / 10.0, 0.0, 1.0);
    let edge = front + 0.6 * sin(world.x * 0.35 + u.time * 1.3) + 0.8 * noise2(vec2<f32>(world.x * 0.15, u.time * 0.4));
    if (world.y > edge) {
      let sea = vec3<f32>(0.40, 0.63, 0.62);
      let rw = mat2x2<f32>(0.8, 0.6, -0.6, 0.8) * world;
      // thin and clear at the edge, the sand under it plain, greener as it deepens
      col = mix(col * vec3<f32>(0.82, 0.88, 0.88), sea, 0.03 + 0.14 * depth);
      // the sky's glints on its little waves
      let gl = noise2(rw * 2.3 + vec2<f32>(u.time * 0.9, u.time * 0.5)) * noise2(world * 3.1 - vec2<f32>(0.0, u.time * 0.7));
      col = col + vec3<f32>(0.06, 0.07, 0.07) * smoothstep(0.3, 0.8, gl);
      let caustic = noise2(rw * 0.9 + vec2<f32>(u.time * 0.6, -u.time * 0.4)) * noise2(world * 1.3 - vec2<f32>(u.time * 0.3, 0.0));
      col = col + vec3<f32>(0.10, 0.12, 0.12) * smoothstep(0.15, 0.55, caustic) * (1.0 - depth);
      let foam = exp(-(world.y - edge) * 1.1) * smoothstep(0.25, 0.8, noise2(rw * 1.7 + vec2<f32>(0.0, u.time)) * 0.6 + noise2(world * 4.3 + 3.0) * 0.25 + noise2(rw * 11.0) * 0.15);
      col = mix(col, vec3<f32>(0.97, 0.98, 0.97), clamp(foam * 1.4, 0.0, 0.9));
    }
  }
  return vec4<f32>(col, 1.0);
}
