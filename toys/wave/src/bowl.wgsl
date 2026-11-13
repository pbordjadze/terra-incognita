// The stands and the pitch. Every seat a person, every person a phone held up as a light; standing,
// the light rises and brightens. The stands are a mesh whose uv is (seat, row); the state of every
// seat is a texture of U x V (r: how far up, 0..1; g: the person's own light, 0 none .. 1 bright; b: a
// colour of their own).

struct C {
  eye: vec3<f32>, tilt: f32,
  size: vec2<f32>, f: f32, time: f32,
  grid: vec2<f32>, ppp: f32, _p: f32,
};
@group(0) @binding(0) var<uniform> cam: C;
@group(0) @binding(1) var seats: texture_2d<f32>;

struct VI { @location(0) pos: vec3<f32>, @location(1) uv: vec2<f32> };
struct VO { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) world: vec3<f32> };

@vertex fn vs_main(v: VI) -> VO {
  let s = sin(cam.tilt);
  let c = cos(cam.tilt);
  let d = v.pos - cam.eye;
  let x = d.x;
  let y = d.y * c + d.z * s;
  let z = -d.y * s + d.z * c;
  var o: VO;
  o.clip = vec4<f32>(cam.f * x * 2.0 / cam.size.x, cam.f * y * 2.0 / cam.size.y, z * 0.5, z);
  o.uv = v.uv;
  o.world = v.pos;
  return o;
}

fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }

fn pitch(w: vec3<f32>) -> vec3<f32> {
  // a dark pitch under dimmed floodlights, mown in stripes, its white lines faint
  let stripe = step(0.5, fract(w.z / 10.0));
  var g = mix(vec3<f32>(0.030, 0.075, 0.040), vec3<f32>(0.036, 0.088, 0.046), stripe);
  let p = abs(w.xz);
  let inside = step(p.x, 34.0) * step(p.y, 52.5);
  var line = 0.0;
  line = max(line, (1.0 - smoothstep(0.0, 0.18, abs(p.x - 34.0))) * step(p.y, 52.5));
  line = max(line, (1.0 - smoothstep(0.0, 0.18, abs(p.y - 52.5))) * step(p.x, 34.0));
  line = max(line, (1.0 - smoothstep(0.0, 0.18, abs(w.z))) * step(p.x, 34.0));
  line = max(line, 1.0 - smoothstep(0.0, 0.18, abs(length(w.xz) - 9.15)));
  line = max(line, (1.0 - smoothstep(0.0, 0.18, abs(p.x - 20.16))) * step(36.0, p.y) * step(p.y, 52.5));
  line = max(line, (1.0 - smoothstep(0.0, 0.18, abs(p.y - 36.0))) * step(p.x, 20.16));
  g = mix(g, vec3<f32>(0.20, 0.24, 0.22), line * 0.6);
  let surround = vec3<f32>(0.035, 0.040, 0.045);
  return mix(surround, g, inside);
}

@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  if (i.uv.x < 0.0) {
    return vec4<f32>(pitch(i.world), 1.0);
  }
  let seat = i.uv;
  let id = vec2<i32>(i32(floor(seat.x)) % i32(cam.grid.x), clamp(i32(floor(seat.y)), 0, i32(cam.grid.y) - 1));
  let s = textureLoad(seats, id, 0);
  let up = s.r;
  let has = s.g;
  // the concrete and the people, dark; a concourse every so many rows
  var c = vec3<f32>(0.040, 0.036, 0.044) + vec3<f32>(0.03, 0.012, 0.010) * s.b;
  let aisle = step(abs(seat.y - 20.0), 0.35) + step(abs(seat.y - 40.0), 0.35);
  c = mix(c, vec3<f32>(0.012, 0.012, 0.016), clamp(aisle, 0.0, 1.0));
  // the phone's light: a soft dot, raised and brighter as the person stands
  let px = fwidth(seat);
  let size = max(px.x, px.y);
  var f = fract(seat) - vec2<f32>(0.5, 0.5);
  f.y = f.y - 0.32 * up;
  let r = length(f / vec2<f32>(1.0, 1.0));
  let rad = 0.16 + 0.10 * up;
  // where a seat is smaller than a pixel, its light is spread over it
  let sharp = 1.0 - smoothstep(rad - size * 0.6, rad + size * 0.6, r);
  // (a pixel over several seats takes the light of the one it falls on, so the stands glitter)
  let dot = mix(sharp, 0.5, smoothstep(0.35, 1.2, size));
  let hs = hash(vec2<f32>(f32(id.x), f32(id.y)));
  // a light of its own: most a phone's torch, warm white; some a screen, cool; a few coloured
  var warm = mix(vec3<f32>(1.0, 0.86, 0.66), vec3<f32>(0.72, 0.84, 1.0), smoothstep(0.55, 0.75, s.b));
  warm = mix(warm, vec3<f32>(0.55, 1.0, 0.6), step(0.93, s.b));
  warm = mix(warm, vec3<f32>(1.0, 0.45, 0.45), step(0.97, s.b));
  // each twinkles at its own pace: a phone turned toward the camera flashes, then turns away
  let ph = 0.5 + 0.5 * sin(cam.time * (0.6 + 2.4 * hs) + hs * 61.0);
  let p4 = ph * ph * ph * ph;
  let tw = p4 * p4;
  let bright = has * (0.10 + 0.25 * hs + 1.6 * tw + 3.4 * up);
  // a standing person shows their shirt: green, white, red, or dark, rising with them
  let k = fract(hs * 7.31);
  var shirt = vec3<f32>(0.10, 0.10, 0.11);
  shirt = select(shirt, vec3<f32>(0.06, 0.30, 0.14), k < 0.35);
  shirt = select(shirt, vec3<f32>(0.42, 0.42, 0.40), k >= 0.35 && k < 0.55);
  shirt = select(shirt, vec3<f32>(0.40, 0.07, 0.07), k >= 0.55 && k < 0.7);
  c = c + shirt * up * 0.9;
  c = c + warm * dot * bright;
  // a standing crowd glows: the light of the raised phones on the people round them, along the row
  var halo = 0.0;
  for (var k = -6; k <= 6; k = k + 1) {
    let n = textureLoad(seats, vec2<i32>((id.x + k + i32(cam.grid.x)) % i32(cam.grid.x), id.y), 0);
    halo = halo + n.r * n.g * exp(-f32(k * k) / 10.0);
  }
  c = c + warm * halo * 0.14;
    return vec4<f32>(c, 1.0);
}
