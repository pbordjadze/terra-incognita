struct U { size: vec2<f32>, centre: vec2<f32>, scale: f32, now: f32, side: f32, ppp: f32, river: f32, p0: f32, p1: f32, p2: f32 };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var plain: texture_2d<f32>;
@group(0) @binding(2) var dep: texture_2d<f32>;
@group(0) @binding(3) var smp: sampler;
@group(0) @binding(4) var mask: texture_2d<f32>;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}
fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn vnoise(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p); let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), w.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), w.x), w.y);
}
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let px = i.uv * u.size;
  let world = u.centre + (px - u.size * 0.5) / u.scale;
  let st = world / u.side + 0.5;
  var c = textureSampleLevel(plain, smp, st, 0.0).rgb;
  // the photograph a touch clearer: lifted out of the haze, the greens a little richer
  let l = dot(c, vec3(0.3, 0.59, 0.11));
  c = clamp((c - 0.06) * 1.18, vec3(0.0), vec3(1.0));
  c = mix(vec3(l), c, 1.15);
  // where the river has been: bare sand, then cane, then the forest again
  // nine taps a cell apart: the 25 m grid's cells blurred into a soft edge
  let cellw = 25.0 / u.side;
  var dg = vec2<f32>(0.0);
  for (var j = -1; j <= 1; j++) {
    for (var k = -1; k <= 1; k++) {
      let wgt = select(0.5, 1.0, j == 0 && k == 0);
      dg += textureSampleLevel(dep, smp, st + vec2(f32(j), f32(k)) * cellw * 0.9, 0.0).rg * wgt;
    }
  }
  dg /= 5.0;
  let near = smoothstep(0.05, 0.85, dg.y);
  let d = select(-2.0e9, dg.x / max(dg.y, 1e-3), dg.y > 0.02);
  let age = u.now - d;
  // (the banks' phase and its derivative before the branch: browsers' WGSL takes derivatives only in
  // uniform control flow)
  let per = 22.0;
  let ph = d / per;
  let fw = max(fwidth(ph), 1e-4);
  if (dg.y > 0.02 && age >= 0.0) {
    // where the river has been, its old banks: a thin pale arc every ten years (lines of equal age,
    // the scroll bars a growing bend leaves on its inside), fading as the forest closes over them;
    // a narrow fringe of bare sand at the newest
    let line = smoothstep(0.0, 3.0 * fw + 0.08, abs(fract(ph) - 0.5) * 2.0 - (1.0 - 4.0 * fw));
    let grain = vnoise(world / 30.0);
    let fade = 1.0 - smoothstep(50.0, 200.0, age + grain * 20.0);
    // where the ridges crowd together (a bend's tip, a neck) they would fan into a mesh: let them go
    let calm = 1.0 - smoothstep(0.06, 0.2, fw);
    let arc = clamp(line, 0.0, 1.0) * fade * near * calm * 0.08;
    c = mix(c, vec3(0.78, 0.80, 0.62), arc);
    // just swept: wet silt, darker than the forest's edge, drying within a few years
    // just left by the water: a point bar of pale sand on the inside of the bend, greening in a decade
    let sand = (1.0 - smoothstep(1.0, 10.0, age + grain * 3.0)) * near;
    c = mix(c, vec3(0.72, 0.66, 0.52), sand * 0.5);
    c = c * (1.0 + 0.06 * near * fade);
  }
  // the water, from the mask the river and its lakes were painted into (r the river's water, g where
  // a bend shoals it, b a lake's darkness): drawn into the photograph rather than laid on it, with
  // the photograph's own grain in it, a wet edge, and the faster water down the middle a shade lighter
  let m = textureSampleLevel(mask, smp, i.uv, 0.0);
  let grain = l - 0.25;
  let o = vec2<f32>(u.river * 0.22) / u.size;
  // how far inside the channel: the mask averaged a third of a width around (smooth, no ridges)
  var mid = 0.0;
  mid += textureSampleLevel(mask, smp, i.uv + vec2(o.x, 0.0), 0.0).r;
  mid += textureSampleLevel(mask, smp, i.uv - vec2(o.x, 0.0), 0.0).r;
  mid += textureSampleLevel(mask, smp, i.uv + vec2(0.0, o.y), 0.0).r;
  mid += textureSampleLevel(mask, smp, i.uv - vec2(0.0, o.y), 0.0).r;
  mid = smoothstep(0.5, 1.0, mid * 0.25);
  c = mix(c, vec3(0.13, 0.17, 0.24) * (1.0 + grain * 0.8), clamp(m.b, 0.0, 1.0) * 0.9);
  c = mix(c, vec3(0.42, 0.39, 0.29), smoothstep(0.0, 0.5, m.r) * 0.3);
  // the deep water down the middle a shade darker, the shoals at the bends paler
  let water = mix(vec3(0.68, 0.56, 0.42), vec3(0.78, 0.67, 0.52), clamp(m.g * 1.4, 0.0, 1.0)) * (1.0 - 0.1 * mid * mid) + vec3(0.5, 0.46, 0.36) * smoothstep(0.62, 1.0, m.g);
  // the banks a shade darker (wet, shadowed by the trees), the water a little clear over them
  let bank = smoothstep(0.1, 0.55, m.r) * (1.0 - mid);
  c = mix(c, water * (1.0 + grain * 0.5) * (1.0 - 0.16 * bank), smoothstep(0.1, 1.0, m.r) * 0.86);
  return vec4<f32>(c, 1.0);
}
