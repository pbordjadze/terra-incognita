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
  let wv = world.y * 5.2 + 1.6 * noise2(world * 0.18) + 0.6 * sin(world.x * 0.21);
  let ripple = sin(wv) * (1.0 - worked) * 0.5;
  let rslope = cos(wv) * (1.0 - worked);
  var col = vec3<f32>(0.86, 0.80, 0.69);
  let g = grain(world);
  col = col * (0.90 + 0.16 * g);
  // dark mineral specks and bits of shell
  let sp = hash2(floor(world * 9.0));
  col = mix(col, vec3<f32>(0.38, 0.34, 0.30), step(0.985, sp) * 0.7);
  col = mix(col, vec3<f32>(0.97, 0.95, 0.90), step(0.992, hash2(floor(world * 6.0) + 9.0)) * 0.6);
  // light on the ripples and the furrow's sides
  col = col * (1.0 + 0.06 * rslope * (1.0 - wet * 0.5) - (gx + gy) * 0.22);
  col = col * (1.0 - 0.16 * smoothstep(0.2, 1.0, fld(c).z));
  // the sifted sand: drier, a little lighter, rougher
  col = mix(col, col * vec3<f32>(1.05, 1.04, 1.02) * (0.96 + 0.08 * noise2(world * 22.0)), worked * 0.85);
  // wet sand: darker and cooler, with a sheen
  let wk = smoothstep(0.3, 1.0, wet) * (1.0 - worked * 0.6);
  col = mix(col, col * vec3<f32>(0.80, 0.76, 0.70), wk);
  col = col + vec3<f32>(0.07, 0.08, 0.09) * wk * smoothstep(0.6, 0.9, noise2(world * 0.7 + vec2<f32>(u.time * 0.05, 0.0)));
  // the swash: thin water over the sand, foam at its edge
  let front = u.p0.x;
  if (front > -50.0 && world.y > front - 1.2) {
    let depth = clamp((world.y - front) / 10.0, 0.0, 1.0);
    let edge = front + 0.6 * sin(world.x * 0.35 + u.time * 1.3) + 0.8 * noise2(vec2<f32>(world.x * 0.15, u.time * 0.4));
    if (world.y > edge) {
      let sea = vec3<f32>(0.40, 0.63, 0.62);
      col = mix(col * vec3<f32>(0.78, 0.84, 0.84), sea, 0.10 + 0.35 * depth);
      let caustic = noise2(world * 0.9 + vec2<f32>(u.time * 0.6, -u.time * 0.4)) * noise2(world * 1.3 - vec2<f32>(u.time * 0.3, 0.0));
      col = col + vec3<f32>(0.10, 0.12, 0.12) * smoothstep(0.25, 0.5, caustic) * (1.0 - depth);
      let foam = exp(-(world.y - edge) * 1.1) * smoothstep(0.3, 0.75, noise2(world * 1.7 + vec2<f32>(0.0, u.time)) * 0.7 + noise2(world * 5.0) * 0.3);
      col = mix(col, vec3<f32>(0.97, 0.98, 0.97), clamp(foam * 1.4, 0.0, 0.9));
    }
  }
  return vec4<f32>(col, 1.0);
}
