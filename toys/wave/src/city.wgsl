// Beyond the stadium: Mexico City at night, its streets a grid of orange lights to the horizon, and
// the sky over it lit from below. A backdrop (toy-kit backdrop.rs); u.p0 = (focal length in pixels,
// tilt down in radians, eye height, eye z), u.p1 = (the bowl's outer half-width, half-length,
// corner radius, 0).

fn ray(px: vec2<f32>) -> vec3<f32> {
  let f = u.p0.x;
  let th = u.p0.y;
  let c = vec3<f32>((px.x - u.size.x * 0.5) / f, -(px.y - u.size.y * 0.5) / f, 1.0);
  // camera to world: up = (0, cos, sin), forward = (0, -sin, cos)
  let y = c.y * cos(th) - c.z * sin(th);
  let z = c.y * sin(th) + c.z * cos(th);
  return normalize(vec3<f32>(c.x, y, z));
}

fn rrect(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - half + vec2<f32>(r);
  return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let d = ray(px);
  let eye = vec3<f32>(0.0, u.p0.z, u.p0.w);
  // the sky: deep blue-black, warm with the city's light toward the horizon
  let h = max(d.y, 0.0);
  var c = mix(vec3<f32>(0.16, 0.09, 0.07), vec3<f32>(0.010, 0.012, 0.030), smoothstep(0.0, 0.35, h));
  if (d.y < 0.0) {
    let t = eye.y / -d.y;
    let g = eye.xz + d.xz * t;
    // the stadium's own dark roof and walls round the bowl
    let e = rrect(g, u.p1.xy, u.p1.z);
    // streets: a grid, a little turned, with avenues every so often
    let q = vec2<f32>(g.x * 0.96 + g.y * 0.28, -g.x * 0.28 + g.y * 0.96);
    let block = 60.0;
    let cell = q / block;
    // distance to the nearest street, metres (streets run between the blocks)
    let fq = (0.5 - abs(fract(cell) - 0.5)) * block;
    let street = min(fq.x, fq.y);
    let w = max(fwidth(street), 0.3);
    // the street's glow, and its lamps every 25 m along it
    let along = select(q.x, q.y, fq.x < fq.y);
    let lamp = 1.0 - smoothstep(0.0, 2.2 * w + 1.0, length(vec2<f32>(street, (0.5 - abs(fract(along / 25.0) - 0.5)) * 25.0)));
    let glow = exp(-street / (2.0 + w));
    let lit = 0.35 + 0.9 * hash2(floor(cell) + vec2<f32>(3.0, 7.0));
    // the avenues: every few streets wider and brighter
    let ix = round(cell.x);
    let iy = round(cell.y);
    let ax = exp(-fq.x / (2.5 + w)) * step(abs(ix - 6.0 * round(ix / 6.0)), 0.1);
    let ay = exp(-fq.y / (2.5 + w)) * step(abs(iy - 5.0 * round(iy / 5.0)), 0.1);
    let wide = max(ax, ay);
    // and the blocks themselves: windows, signs, courtyards, a scatter of points, some white
    let spot = floor(g / 7.0);
    let hs = hash2(spot);
    let pin = step(0.72, hs) * (1.0 - smoothstep(0.0, 0.5 + 1.5 * w, length(fract(g / 7.0) - 0.5 - 0.3 * (vec2<f32>(hash2(spot + vec2<f32>(1.0, 0.0)), hash2(spot + vec2<f32>(0.0, 1.0))) - 0.5)) * 7.0));
    var lamps = (lamp * 1.0 + glow * 0.06) * lit + wide * 0.12 + pin * 0.6;
    // far away the lights merge into a glow
    let far = 1.0 - exp(-t / 1800.0);
    var ground = vec3<f32>(0.014, 0.011, 0.012) + vec3<f32>(1.0, 0.55, 0.20) * lamps * 0.45;
    ground = mix(ground, vec3<f32>(0.11, 0.06, 0.03), far * 0.8);
    // the stadium's dark shell round the stands
    ground = mix(ground, vec3<f32>(0.012, 0.012, 0.016), 1.0 - smoothstep(0.0, 25.0, e));
    c = ground;
  }
  return vec4<f32>(c, 1.0);
}
