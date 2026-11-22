// Nazaré traced the way Max traced Carla's Island (max.rs): each pixel's ray marched against the sea's
// analytic height field, reflected at the sea, its reflection tested roughly against the land, and one byte
// written: the reflected ray's elevation for the sea, the sky's for the sky, the normal's for the land.
//
// field: three blocks side by side, each the floor's cells (lib.rs › texels):
//   A [cos, sin of the arriving train's phase, its height / H_FULL, ground]  ground: sea 0.43 × sqrt(depth / 800);
//                                                                             land 0.5 + 0.5 × height / LAND_M
//   B [cos, sin of the turned-back train's phase, its height / H_FULL, depth / 60]
//   C [cos, sin of the sets' phase, 0, 0]
// u.up.w and u.size.xy: the short swell's height (m) and wavenumber vector (rad/m), drawn here
struct U {
  eye: vec4<f32>, fwd: vec4<f32>, right: vec4<f32>, up: vec4<f32>, sun: vec4<f32>,
  phase: vec4<f32>, grid: vec4<f32>, size: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var field: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

const TAU: f32 = 6.2831853;
const LAND_M: f32 = 200.0;
const BREAK: f32 = 1.0;
// the wind's chop: how many waves, each one's slope (amplitude × wavenumber), the way the wind blows (rad
// from east: the evening's northwesterly, toward the south-southeast)
const WIND_WAVES: i32 = 14;
const WIND_SLOPE: f32 = 0.022;
const WIND_TO: f32 = -1.2;

struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}

struct Sea {
  z: f32,       // the surface (m above the still sea)
  land: f32,    // 1 on land
  ground: f32,  // the land's height (m), or the sea's depth (m)
  near: f32,    // how near the waves are to breaking (0..)
  crest: f32,   // how near a crest this is (1 on it)
  front: f32,   // 1 on a wave's front face, just ahead of its crest
  behind: f32,  // 1 just behind a crest, where a broken wave's white water rolls
};

fn blocks(q: vec2<f32>) -> array<vec4<f32>, 3> {
  let cells = u.grid.xy;
  let uv = clamp(q / (u.grid.z * cells), vec2(0.0), vec2(1.0));
  let half_texel = 0.5 / (3.0 * cells.x);
  let ux = clamp(uv.x / 3.0, half_texel, 1.0 / 3.0 - half_texel);
  let a = textureSampleLevel(field, samp, vec2(ux, uv.y), 0.0);
  let b = textureSampleLevel(field, samp, vec2(ux + 1.0 / 3.0, uv.y), 0.0);
  let c = textureSampleLevel(field, samp, vec2(ux + 2.0 / 3.0, uv.y), 0.0);
  return array<vec4<f32>, 3>(a, b, c);
}

// The blocks smoothed (a cubic B-spline from four bilinear taps): the field's cells are 50 m, and a crest's
// slope read from straight bilinear steps cell by cell, which up close shows as facets. The taps are
// filtered here, not by the sampler: a sampler's weights are a few bits, so across a wave's face (well under
// a cell) its phase would step, streaking the face.
fn texel(x: i32, y: i32, block: i32) -> vec4<f32> {
  let cells = vec2<i32>(u.grid.xy);
  let xx = clamp(x, 0, cells.x - 1) + block * cells.x;
  return textureLoad(field, vec2(xx, clamp(y, 0, cells.y - 1)), 0);
}

fn tap(c: vec2<f32>, wt: f32) -> array<vec4<f32>, 3> {
  let p = c - vec2(0.5);
  let i = vec2<i32>(floor(p));
  let f = p - floor(p);
  var out: array<vec4<f32>, 3>;
  for (var b = 0; b < 3; b++) {
    let top = mix(texel(i.x, i.y, b), texel(i.x + 1, i.y, b), f.x);
    let bot = mix(texel(i.x, i.y + 1, b), texel(i.x + 1, i.y + 1, b), f.x);
    out[b] = mix(top, bot, f.y) * wt;
  }
  return out;
}

fn blocks_smooth(q: vec2<f32>) -> array<vec4<f32>, 3> {
  let c = q / u.grid.z - vec2(0.5);
  let i = floor(c);
  let f = c - i;
  let w0 = (1.0 - f) * (1.0 - f) * (1.0 - f) / 6.0;
  let w1 = (4.0 - 6.0 * f * f + 3.0 * f * f * f) / 6.0;
  let w2 = (1.0 + 3.0 * f + 3.0 * f * f - 3.0 * f * f * f) / 6.0;
  let w3 = f * f * f / 6.0;
  let g0 = w0 + w1;
  let g1 = w2 + w3;
  let h0 = i - vec2(1.0) + w1 / g0 + vec2(0.5);
  let h1 = i + vec2(1.0) + w3 / g1 + vec2(0.5);
  let s00 = tap(vec2(h0.x, h0.y), g0.x * g0.y);
  let s10 = tap(vec2(h1.x, h0.y), g1.x * g0.y);
  let s01 = tap(vec2(h0.x, h1.y), g0.x * g1.y);
  let s11 = tap(vec2(h1.x, h1.y), g1.x * g1.y);
  var out: array<vec4<f32>, 3>;
  for (var j = 0; j < 3; j++) { out[j] = s00[j] + s10[j] + s01[j] + s11[j]; }
  return out;
}

// One train's shape at its phase u (radians from its crest; negative ahead of it): a cosine at sea, its front
// half squeezed and its trough ahead flattened as it nears breaking (lean 0..1), its crest sharpened
// (Stokes' second order).
fn shape(u0: f32, lean: f32, ka: f32) -> vec3<f32> {
  let v = u0 - TAU * floor((u0 + 3.14159265) / TAU);
  let wf = 3.14159265 * (1.0 - 0.8 * lean);
  // the face concave as a breaking wave's: steepest under the lip, easing into the trough
  let x = clamp(-v / wf, 0.0, 1.0);
  let front = mix(cos(x * 3.14159265), 2.0 * pow(1.0 - x, 2.2) - 1.0, lean);
  let e = select(front, cos(v), v >= 0.0);
  let front_face = select(0.0, 1.0 - clamp(-v / wf, 0.0, 1.0), v < 0.0);
  let behind = select(0.0, 1.0 - clamp(v / 1.9, 0.0, 1.0), v >= 0.0);
  // near breaking the crest pitches forward: its top runs a little ahead of the face below it
  let lip = 0.35 * lean * lean * exp(-pow((v + 0.12 * wf) / (0.08 * wf + 0.02), 2.0));
  return vec3(e + lip + 0.5 * ka * (2.0 * e * e - 1.0), front_face, behind);
}

fn sea_at(q: vec2<f32>, foot: vec2<f32>) -> Sea {
  return sea_from(blocks(q), q, foot);
}

fn sea_smooth(q: vec2<f32>, foot: vec2<f32>) -> Sea {
  return sea_from(blocks_smooth(q), q, foot);
}

// How much of a wave one pixel can carry: all of it while each wave covers four pixels or more on screen, none
// under two (Nyquist), where its crests would only alias into bands. On screen a wave's crests repeat at its
// length foreshortened by the grazing angle, until the view is lower than the waves are steep: then each
// hides the next one's back, and what repeats on screen is each one's face, as tall as the wave. `foot`: a
// pixel's width at this distance (m), the sine of the view's grazing angle.
fn kept(foot: vec2<f32>, wavelength: f32, height: f32) -> f32 {
  let covers = max(wavelength * max(foot.y, 0.003), height);
  return 1.0 - smoothstep(0.25, 0.5, foot.x / max(covers, 1e-4));
}

fn foot_at(t: f32, graze: f32) -> vec2<f32> {
  return vec2(t * u.phase.w, abs(graze));
}

fn sea_from(k: array<vec4<f32>, 3>, q: vec2<f32>, foot: vec2<f32>) -> Sea {
  var s: Sea;
  s.land = smoothstep(0.47, 0.53, k[0].a);
  let depth = select(800.0 * pow(min(k[0].a, 0.43) / 0.43, 2.0), 60.0 * k[1].a, k[1].a < 0.98);
  s.ground = select(depth, (k[0].a - 0.5) * 2.0 * LAND_M, s.land > 0.5);
  // the sets: a group of big waves every so often, at the group speed
  let g = atan2(k[2].g - 0.5, k[2].r - 0.5) / TAU;
  let sets = 0.45 + 0.55 * pow(0.5 + 0.5 * cos(TAU * (g - u.phase.y)), 6.0);
  // each train's phase now and its height (crest to trough, m) in this set, kept only where a pixel can
  // carry its crests (the local wavelength, shortened over the shallows)
  let d = max(depth, 0.5);
  let kk = u.phase.z / max(tanh(u.phase.z * d), 0.05);
  let ph1 = atan2(k[0].g - 0.5, k[0].r - 0.5);
  let ph2 = atan2(k[1].g - 0.5, k[1].r - 0.5);
  let h1 = k[0].b * u.grid.w * sets * kept(foot, TAU / kk, k[0].b * u.grid.w * sets);
  let h2 = k[1].b * u.grid.w * sets * kept(foot, TAU / kk, k[1].b * u.grid.w * sets);
  s.near = (h1 + h2) / (BREAK * d);
  let lean = clamp((s.near - 0.4) / 0.4, 0.0, 1.0);
  let w1 = shape(TAU * u.phase.x - ph1, lean, min(kk * h1 * 0.5, 0.4));
  let w2 = shape(TAU * u.phase.x - ph2, lean, min(kk * h2 * 0.5, 0.4));
  var z = 0.5 * h1 * w1.x + 0.5 * h2 * w2.x;
  // the short swell the field can't carry: straight crests at the open sea's spacing, kept as the trains are
  let short_fade = kept(foot, TAU / max(length(u.size.xy), 1e-4), u.up.w * sets);
  z += 0.5 * u.up.w * sets * short_fade * cos(TAU * u.phase.x - dot(u.size.xy, q));
  s.crest = max(pow(max(w1.x, -1.0) * 0.5 + 0.5, 6.0) * step(2.0, h1), pow(max(w2.x, -1.0) * 0.5 + 0.5, 6.0) * step(2.0, h2));
  s.front = max(w1.y, w2.y);
  s.behind = max(w1.z * step(2.0, h1), w2.z * step(2.0, h2));
  s.z = mix(z, s.ground, s.land);
  return s;
}

fn land_at(q: vec2<f32>) -> f32 {
  let k = blocks(q);
  return select(-1000.0, (k[0].a - 0.5) * 2.0 * LAND_M, k[0].a > 0.5);
}

fn elevation_index(e: f32, base: f32) -> f32 {
  return base + clamp(floor(e / 50.0 * 63.0), 0.0, 63.0);
}

// The wind's chop, in the shading only: a spectrum of small waves (2 to 30 m, slopes of a light wind's sea)
// spread about the wind, each moving at its own deep-water speed. Each is kept only as far as the pixel's
// footprint along its direction can carry it; what the footprint averages away still tilts the sea toward the
// eye, as a rough sea seen low shows the faces turned to it, so the distance greys toward the sky smoothly
// instead of banding. Calmed on a standing face, where it would streak it.
fn wind_sea(q: vec2<f32>, d: vec3<f32>, t: f32, n: vec3<f32>, s: Sea) -> vec3<f32> {
  let run = normalize(d.xy + vec2(1e-6, 0.0));
  let foot = foot_at(t, dot(d, n));
  let lean = clamp((s.near - 0.4) / 0.4, 0.0, 1.0);
  let slope = WIND_SLOPE * (1.0 - 0.85 * lean * s.front);
  var g = vec2(0.0);
  var lost = 0.0;
  for (var i = 0; i < WIND_WAVES; i++) {
    let f = f32(i) / f32(WIND_WAVES - 1);
    let wavelength = 2.0 * pow(15.0, f);
    let kw = TAU / wavelength;
    let ang = WIND_TO + 1.05 * (2.0 * fract(f32(i) * 0.618034 + 0.31) - 1.0);
    let dir = vec2(cos(ang), sin(ang));
    // a wave is foreshortened only as far as it runs along the view: its crests' spacing on screen
    let c = abs(dot(dir, run));
    let graze = 1.0 / length(vec2(c / max(foot.y, 0.003), sqrt(max(1.0 - c * c, 0.0))));
    let w = kept(vec2(foot.x, graze), wavelength, 2.0 * WIND_SLOPE / kw);
    let ph = kw * dot(dir, q) - sqrt(9.81 * kw) * u.eye.w + f32(i) * 2.39996;
    g += w * slope * cos(ph) * dir;
    lost += (1.0 - w) * 0.5 * slope * slope;
  }
  g += 0.8 * sqrt(lost) * run;
  return normalize(vec3(n.xy / n.z - g, 1.0));
}

@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let eye = u.eye.xyz;
  let ndc = vec2(i.uv.x * 2.0 - 1.0, 1.0 - i.uv.y * 2.0);
  let d = normalize(u.fwd.xyz + ndc.x * u.right.w * u.right.xyz + ndc.y * u.fwd.w * u.up.xyz);
  let sun = u.sun.xyz;
  var idx = 0.0;
  // the sky: its elevation; the sun's disc on it
  let sky = elevation_index(degrees(asin(clamp(d.z, 0.0, 1.0))), 0.0);
  let sun_disc = dot(d, sun) > 0.99985;
  // march from where the ray is as high as anything can be to where it is under every trough
  let top = 130.0;
  var t0 = 1.0;
  if (eye.z > top && d.z < 0.0) { t0 = (eye.z - top) / -d.z; }
  var t1 = 30000.0;
  if (d.z < -1e-4) { t1 = min(t1, (eye.z + 30.0) / -d.z); }
  if (d.z > 0.0) { t1 = min(t1, (top - eye.z) / d.z); }
  var lo = t0;
  var hi = t1;
  var hit = false;
  let n = 96;
  if (t1 > t0) {
    for (var j = 1; j <= n; j++) {
      let t = t0 * pow(t1 / t0, f32(j) / f32(n));
      let p = eye + d * t;
      var sz = 0.0;
      // up close, filtered exactly (as the shading is), so a thin lip isn't stepped over
      let foot = foot_at(t, d.z);
      if (t < 400.0) { sz = sea_from(tap(p.xy / u.grid.z, 1.0), p.xy, foot).z; } else { sz = sea_at(p.xy, foot).z; }
      if (p.z < sz) { hi = t; hit = true; break; }
      lo = t;
    }
  }
  if (!hit) {
    idx = select(sky, 255.0, sun_disc);
    return vec4(idx / 255.0, 0.0, 0.0, 1.0);
  }
  for (var j = 0; j < 9; j++) {
    let m = 0.5 * (lo + hi);
    let p = eye + d * m;
    if (p.z < sea_smooth(p.xy, foot_at(m, d.z)).z) { hi = m; } else { lo = m; }
  }
  let t = hi;
  let p = eye + d * t;
  let foot = foot_at(t, d.z);
  let s = sea_smooth(p.xy, foot);
  let e = clamp(t * 0.003, 0.4, 25.0);
  let zx = sea_smooth(p.xy + vec2(e, 0.0), foot).z - sea_smooth(p.xy - vec2(e, 0.0), foot).z;
  let zy = sea_smooth(p.xy + vec2(0.0, e), foot).z - sea_smooth(p.xy - vec2(0.0, e), foot).z;
  let swell_n = normalize(vec3(-zx / (2.0 * e), -zy / (2.0 * e), 1.0));
  var nrm = swell_n;
  if (s.land > 0.5) {
    // the land: which ground (sand low, limestone where steep, scrub on top, the town's walls on the
    // headland's south side), then the angle of its normal toward the sun
    let steep = 1.0 - nrm.z;
    var m = 2.0;
    if (s.ground < 6.0) { m = 0.0; } else if (steep > 0.22) { m = 1.0; }
    let to_light = length(p.xy - u.size.zw);
    if (m == 2.0 && p.y > u.size.w + 120.0 && to_light < 1800.0) { m = 3.0; }
    // the floor's 50 m cells are smooth: fields, scrub and rock break the light in patches
    let patches = sin(p.x * 0.09 + 1.3 * sin(p.y * 0.05)) * sin(p.y * 0.11 + 1.1 * sin(p.x * 0.04));
    let lam = clamp(dot(nrm, sun), 0.0, 1.0) * 0.6 + 0.4 * nrm.z + 0.12 * patches;
    idx = 128.0 + m * 16.0 + clamp(floor(lam * 15.99), 0.0, 15.0);
    return vec4(idx / 255.0, 0.0, 0.0, 1.0);
  }
  // the sea: the wind's chop on the swell, then reflect, and see whether the reflection meets the land on its way to the sky
  nrm = wind_sea(p.xy, d, t, swell_n, s);
  let r = reflect(d, nrm);
  let rz = abs(r.z);
  var blocked = false;
  let q0 = p + 2.0 * vec3(r.xy, rz);
  for (var j = 0; j < 7; j++) {
    let tt = 25.0 * pow(2.0, f32(j));
    let q = q0 + tt * vec3(r.xy, rz);
    if (land_at(q.xy) > q.z) { blocked = true; break; }
  }
  let rel = degrees(asin(clamp(rz, 0.0, 1.0)));
  idx = elevation_index(rel, 64.0);
  if (blocked) {
    idx = 192.0 + clamp(floor(rel / 30.0 * 15.0), 0.0, 15.0);
  } else {
    let rs = dot(vec3(r.xy, rz), sun);
    let w = u.sun.w;
    if (rs > w) {
      idx = 224.0 + clamp(floor((rs - w) / (1.0 - w) * 31.99), 0.0, 31.0);
    }
  }
  // white water: the lip where a wave stands near breaking, the face it throws, and what rolls in after
  let breaking = smoothstep(0.7, 0.95, s.near);
  let churn = 0.5 + 0.5 * sin(p.x * 0.11 + p.y * 0.07 + u.eye.w * 0.8) * sin(p.x * 0.05 - p.y * 0.13);
  // the lip only at the face's top: the face itself stays dark, as a standing wall's does
  let lip = smoothstep(0.9, 0.96, s.front);
  let white = breaking * (lip * 1.2 + 0.9 * s.behind * churn);
  if (white > 0.55) {
    let lit = clamp(dot(nrm, sun) * 0.5 + 0.5, 0.0, 1.0);
    idx = 208.0 + clamp(floor(lit * 15.99), 0.0, 15.0);
  }
  return vec4(idx / 255.0, 0.0, 0.0, 1.0);
}
