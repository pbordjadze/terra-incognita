// The clouds' fragment shader, as text for sky.js (which draws a full-screen triangle with it).
window.CLOUDS_FRAG = `#version 300 es
// The clouds, one pixel at a time (sky.js draws a full-screen triangle with this).
// App/TerraIncognita/Clouds/Clouds.metal is the same, line for line, in Metal (the native
// prototype): change both together.
//
// Measured: how often each place is under cloud this month, and which kinds of cloud make up its
// sky (sky.png, from ISCCP-H). Drawn: particular clouds, noise thresholded so that each place's
// drawn cover is its measured cover; cyclones wound into the westerlies; everything carried by
// the usual winds at its height. Five layers, low to high: cumulus; stratocumulus and stratus;
// altostratus and nimbostratus; the thin clouds aloft (altocumulus, cirrus); cirrostratus and
// deep convection. Each is lit from the north-west as the globe's relief is (or by a sun, for a
// globe that has one), and casts a soft shadow; coming closer, the reader passes down through them,
// the highest first.
precision highp float;
precision highp int;

uniform vec2 uSize;        // canvas pixels
uniform float uPx;         // canvas pixels per point
uniform vec3 uDisc;        // the globe: centre x, y and radius, points (y down)
uniform mat3 uRot;         // world from view
uniform float uZoom;
uniform float uReading;    // the reading scale (1 on a phone, 1.2 on the 13-inch iPad)
uniform float uDays;       // simulated days
uniform vec3 uMonth;       // this month (0 = January), the next, how far between
uniform sampler2D uSky;
uniform sampler2D uLand;    // 1 on land (the globe's own coast)
uniform highp sampler3D uNoise;
uniform vec4 uSun;         // xyz the sun (world, towards it); w 1 to light by it, 0 the map's north-west
uniform vec4 uSeaShadow;   // rgb, and how strong on the sea
uniform vec4 uLandShadow;  // rgb, and how strong on land
uniform vec4 uTone;        // x the clouds' opacity, y how far the night side darkens them
uniform vec4 uCyclone[20];  // the cyclones (cyclones(), below)
out vec4 outColor;

const float PI = 3.14159265;

// ---- noise: pcg3d-hashed gradient noise, quintic (sky.js's tile of it); fbm with a rotation
// between octaves. One octave's sd, as the tile is sampled (0.185), is measured by
// tools/clouds/calibrate.py, which makes the same tile in numpy: so scored, the fbm is
// close to a standard normal, and a cover becomes a threshold.

uvec3 pcg3d(uvec3 v) {
  v = v * 1664525u + 1013904223u;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  v ^= v >> 16u;
  v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
  return v;
}
vec3 hash3(ivec3 c) { return vec3(pcg3d(uvec3(c))) * (1.0 / 4294967295.0); }

// one tile of the noise, 16 cells a side, is drawn once by sky.js into a 3D texture (six samples
// a cell; the hardware's trilinear filter between them): a lookup an octave, not eight hashes
float gnoise(vec3 p) { return texture(uNoise, p * (1.0 / 16.0)).r * 2.0 - 1.0; }

const mat3 OCT = mat3(0.00, -0.80, -0.60, 0.80, 0.36, -0.48, 0.60, -0.48, 0.64);

// fbm as a standard score (x) and how much of its variation is resolved (y): an octave finer than
// about three pixels ('foot': a pixel, in the first octave's lattice cells) is left at its mean, so
// the limb doesn't shimmer, and what it would have added becomes an even haze
vec2 fbm(vec3 p, int octaves, float foot) {
  float s = 0.0, a = 1.0, kept = 0.0, all = 0.0;
  for (int k = 0; k < 7; k++) {
    if (k >= octaves) break;
    float keep = 1.0 - smoothstep(0.3, 0.6, foot);
    if (keep > 0.0) s += a * keep * gnoise(p + 17.13 * float(k));
    kept += a * a * keep * keep; all += a * a;
    a *= 0.5; foot *= 2.03;
    p = OCT * p * 2.03;
  }
  return vec2(s / max(0.185 * sqrt(kept), 1e-4), sqrt(kept / all));
}

float cdf(float z) { return 1.0 / (1.0 + exp(-1.702 * z)); }   // the normal's, near enough

// ---- the measured sky

vec3 skyAt(vec3 q, float block) {
  float lon = atan(q.x, q.z), lat = asin(clamp(q.y, -1.0, 1.0));
  vec2 uv = vec2((lon / PI + 1.0) * 180.0, (0.5 - lat / PI) * 180.0);
  vec2 t0 = vec2(mod(uMonth.x, 4.0) * 362.0 + 1.0, (floor(uMonth.x / 4.0) + block) * 182.0 + 1.0) + uv;
  vec2 t1 = vec2(mod(uMonth.y, 4.0) * 362.0 + 1.0, (floor(uMonth.y / 4.0) + block) * 182.0 + 1.0) + uv;
  vec2 size = vec2(1448.0, 1092.0);
  return mix(texture(uSky, t0 / size).rgb, texture(uSky, t1 / size).rgb, uMonth.z);
}

// ---- winds, and the cyclones in the westerlies

vec3 turn(vec3 p, float a) { float c = cos(a), s = sin(a); return vec3(c * p.x - s * p.z, p.y, s * p.x + c * p.z); }

vec3 rotateAbout(vec3 p, vec3 k, float a) {   // Rodrigues
  float c = cos(a), s = sin(a);
  return p * c + cross(k, p) * s + k * dot(k, p) * (1.0 - c);
}

// Each hemisphere keeps five cyclones in the westerlies, each six days long: born somewhere between
// 38 and 60 degrees, winding up (anticlockwise in the north, clockwise in the south) and letting go.
// They live in the westerlies' turning frame, so they travel east with it. Invented, as all the
// particular weather here is; where cyclones run is where the measured cloud lets them show. Where
// each is and how hard it turns are the same for every pixel, so the CPU works them out each frame
// (sky.js's cyclones): two vec4s each, its centre and size, then its turn (signed: which way) and
// the cosine of its reach.
vec3 cyclones(vec3 p, float strength) {
  if (strength <= 0.0) return p;
  for (int k = 0; k < 10; k++) {
    vec4 at = uCyclone[2 * k], turning = uCyclone[2 * k + 1];
    float reach = dot(p, at.xyz);
    if (turning.x == 0.0 || reach < turning.y) continue;
    float d = acos(clamp(reach, -1.0, 1.0)), size = at.w;
    // drawn turning anticlockwise (north): what is here came from clockwise of here
    p = rotateAbout(p, at.xyz, strength * turning.x * exp(-d * d / (size * size)));
  }
  return p;
}

struct Layer {
  float cover; float freq; vec3 stretch; float warp; int octaves; float seed; vec2 belts; float spin;
  float from; float a; float b; float h; float shadow; float lift; float alpha; float wisp; vec3 lit; vec3 shade;
};

// a layer's noise where its wind has carried it to p: warped by a slower, wider noise (so its shapes
// flow rather than blob), the warp itself changing slowly as the days pass
vec2 shape(Layer l, vec3 p, int octaves, float foot) {
  if (l.warp <= 0.0) return fbm(p * l.stretch * l.freq + l.seed, min(octaves, l.octaves), foot);
  vec3 slow = vec3(l.seed, uDays * 0.15, 0.0);
  float wf = l.freq * 0.35;
  vec3 w = vec3(gnoise(p * wf + slow), gnoise(p * wf + slow + 31.7), gnoise(p * wf + slow + 57.1));
  return fbm((p + (l.warp / l.freq) * w / 0.195) * l.stretch * l.freq + l.seed, min(octaves, l.octaves), foot);
}

// a layer's noise at q, carried by its belts' winds (the easterly trades; the westerlies, which
// carry the polar caps too, where the record is least sure: radians a day), each belt turning as
// one and blended where they meet, over the subtropical highs, so no shear piles up
vec2 field(Layer l, vec3 q, int octaves, float foot) {
  float lat = abs(asin(clamp(q.y, -1.0, 1.0))) * 180.0 / PI;
  float wt = 1.0 - smoothstep(22.0, 34.0, lat), wm = 1.0 - wt;
  vec2 s = vec2(0.0); float n = 0.0;
  if (wt > 0.001) { s += wt * shape(l, turn(q, -l.belts.x * uDays), octaves, foot); n += wt * wt; }
  if (wm > 0.001) { s += wm * shape(l, cyclones(turn(q, -l.belts.y * uDays), l.spin), octaves, foot); n += wm * wm; }
  return vec2(s.x / sqrt(n), s.y);
}

// world from view, on a shell 'h' above the ground, the view grown by 'm' about the centre
bool shell(vec2 v, float m, float h, out vec3 q, out float facing) {
  v /= m;
  float rc = 1.0 + h, r2 = dot(v, v);
  if (r2 >= rc * rc) return false;
  float z = sqrt(rc * rc - r2);
  q = normalize(uRot * vec3(v, z));
  facing = z / rc;
  return true;
}

// towards the light along the ground at q: the map's north-west, or a sun's direction
vec3 towardLight(vec3 q) {
  if (uSun.w > 0.5) {
    vec3 t = uSun.xyz - q * dot(q, uSun.xyz);
    if (dot(t, t) > 1e-8) return normalize(t);
  }
  vec3 e = vec3(q.z, 0.0, -q.x);
  e = dot(e, e) > 1e-8 ? normalize(e) : vec3(1.0, 0.0, 0.0);
  return normalize(cross(q, e) - e);
}

// passing through a layer: from zoom 'a' it grows about the screen's centre (it is nearer the eye
// than the ground), and by 'b' it has gone by
float growth(float a, float b) { return 1.0 / (1.0 - 0.9 * pow(smoothstep(a, b, uZoom), 1.6)); }
float present(float a, float b) { return 1.0 - smoothstep(b - 0.4 * (b - a), b, uZoom); }

void over(inout vec4 acc, vec3 col, float a) { acc = vec4(col * a, a) + acc * (1.0 - a); }

void main() {
  vec2 pt = vec2(gl_FragCoord.x, uSize.y - gl_FragCoord.y) / uPx;
  float R = uDisc.z;
  vec2 v = vec2(pt.x - uDisc.x, uDisc.y - pt.y) / R;
  float r2 = dot(v, v);
  if (r2 > 1.08) { outColor = vec4(0.0); return; }
  float px = 1.0 / (uPx * R);                               // a pixel, in radians, at the centre

  // the measured sky over the ground here
  vec3 g = normalize(uRot * vec3(v, sqrt(max(1.0 - r2, 0.0))));
  vec3 A = skyAt(g, 0.0), B = skyAt(g, 3.0);
  float C = clamp(A.r, 0.0, 0.995), lc = log(1.0 - C);
  float sLow = A.g, sMid = A.b, sHigh = B.r, sDeep = B.g, sCu = B.b;
  float sThin = max(1.0 - sLow - sMid - sHigh - sDeep - sCu, 0.0);
  float deep = sDeep / max(sHigh + sDeep, 1e-3);            // towers and anvils rather than veils
  // each layer's cover, so that together they cover what was measured: 1 - (1 - C)^share
  #define COVER(s) (1.0 - exp(lc * (s)))

  vec3 white = vec3(1.0, 0.995, 0.982), grey = vec3(0.72, 0.75, 0.82);
  Layer L[5];
  // cover; noise frequency (cells a radian), stretch, warp, octaves, seed; winds (trades,
  // westerlies: radians a day); cyclones' grip. Shown from zoom 'from'; passed from 'a' to 'b';
  // height (radii); shadow and its offset (points); opacity; wisps (filaments rather than shapes)
  L[0] = Layer(COVER(sCu), 40.0, vec3(1.0), 0.0, 3, 11.0, vec2(-0.11, 0.17), 0.3,
               1.8, 6.0, 16.0, 0.002, 0.08, 4.0, 0.72, 0.0, white, grey);
  L[1] = Layer(COVER(sLow), 4.5, vec3(0.85, 1.2, 0.85), 0.5, 7, 23.0, vec2(-0.10, 0.19), 0.8,
               0.0, 2.8, 5.2, 0.004, 0.14, 4.0, 0.80, 0.0, white, grey);
  L[2] = Layer(COVER(sMid), 2.6, vec3(0.6, 1.7, 0.6), 0.5, 7, 37.0, vec2(-0.05, 0.31), 1.2,
               0.0, 2.0, 4.0, 0.007, 0.12, 6.5, 0.78, 0.0, vec3(0.99, 0.988, 0.98), grey);
  L[3] = Layer(COVER(sThin), 5.0, vec3(0.35, 2.6, 0.35), 0.5, 7, 41.0, vec2(-0.06, 0.46), 0.8,
               0.0, 1.6, 3.2, 0.010, 0.0, 9.0, 0.42, 1.0, white, grey);
  L[4] = Layer(COVER(sHigh + sDeep), mix(3.0, 7.0, deep), mix(vec3(0.45, 2.2, 0.45), vec3(1.0), deep), 0.5, 7, 53.0,
               vec2(-0.06, 0.46), 0.6,
               0.0, 1.5, 3.2, 0.012, mix(0.06, 0.22, deep), 10.0, mix(0.55, 0.92, deep), 0.0, vec3(1.0), grey);

  vec4 acc = vec4(0.0);

  // shadows on the ground, under every layer, cast away from the light; soft
  if (r2 < 1.0) {
    float lit = 1.0;
    vec3 toL = towardLight(g);
    for (int i = 0; i < 5; i++) {
      Layer l = L[i];
      // (a layer's shadow goes as it passes: below it, the ground is in its even shade, not patches)
      float on = present(l.a, l.b) * (l.from == 0.0 ? 1.0 : smoothstep(l.from, l.from + 1.0, uZoom));
      if (l.cover < 0.01 || l.shadow <= 0.0 || on < 0.01) continue;
      vec3 s = normalize(g + l.lift * uReading * sqrt(uZoom) / R * toL);
      vec2 f = field(l, s, 4, px * l.freq * 1.5);
      float edge = 1.0 - l.cover;
      float d = mix(l.cover, smoothstep(edge - 0.06, edge + 0.12, cdf(f.x)), f.y);
      lit *= 1.0 - on * l.shadow * d * (1.0 - 0.6 * l.cover);
    }
    // a near-black over the ground, darkening it as a multiply would (CloudsLook), on sea or land
    vec2 uv = vec2(atan(g.x, g.z) / PI + 1.0, 1.0 - asin(clamp(g.y, -1.0, 1.0)) * 2.0 / PI) * 0.5;
    float land = texture(uLand, uv).r;
    float strength = uTone.x * mix(uSeaShadow.w, uLandShadow.w, land);
    over(acc, mix(uSeaShadow.rgb, uLandShadow.rgb, land), (1.0 - lit) * strength);
  }

  // the layers, low to high, lit from the north-west (or the sun)
  for (int i = 0; i < 5; i++) {
    Layer l = L[i];
    if (l.cover < 0.01) continue;
    float shown = present(l.a, l.b) * (l.from == 0.0 ? 1.0 : smoothstep(l.from, l.from + 1.0, uZoom));
    if (shown < 0.004) continue;
    float m = growth(l.a, l.b);
    vec3 q; float facing;
    if (!shell(v, m, l.h, q, facing)) continue;
    float foot = px * l.freq / (m * max(facing, 0.08));
    vec2 f = field(l, q, 6, foot);
    float u = cdf(f.x), edge = 1.0 - l.cover, t = u - edge;  // t > 0 inside
    vec2 grad = vec2(dFdx(u), dFdy(u));
    float slope = max(length(grad), 1e-5);
    vec2 lv = normalize((transpose(uRot) * towardLight(q)).xy + 1e-6);
    float d;
    vec3 col;
    if (l.wisp > 0.0) {
      // thin cloud aloft: where it is, fine filaments along the wind, soft
      float covered = mix(l.cover, smoothstep(-0.18, 0.18, t), f.y);
      vec3 qs = turn(q, -l.belts.y * uDays) * vec3(1.0, 4.0, 1.0) * 26.0 + l.seed;
      float thread = 1.0 - abs(gnoise(qs) + 0.5 * gnoise(OCT * qs * 2.03)) * 2.6;
      d = covered * smoothstep(0.15, 0.85, thread) * (1.0 - smoothstep(0.35, 0.6, px * 104.0 / (m * max(facing, 0.08))));
      col = l.lit;
    } else {
      // as from space: thin and grey at the edges, thick and white within, the tops lit like relief
      float feather = max(slope * 1.2, i == 0 ? 0.035 : 0.07);   // cumulus: crisper
      d = mix(l.cover, smoothstep(-feather, feather, t), f.y);
      if (d < 0.002) continue;
      float body = smoothstep(0.0, 0.3, t);
      vec2 rg = vec2(dFdx(f.x), dFdy(f.x));
      float bumps = i == 0 ? 6.0 : 2.2;                       // cumulus are all tops
      float relief = clamp(dot(rg, lv) * bumps / uPx, -1.0, 1.0) * f.y;   // > 0 faces away
      col = mix(l.lit, l.shade, clamp(0.34 * (1.0 - body) + 0.34 * max(relief, 0.0), 0.0, 1.0));
      col = mix(col, vec3(1.0), 0.6 * max(-relief, 0.0) * body);
    }
    // by a sun, the night side's clouds darken (a globe without one has no night)
    if (uSun.w > 0.5) col *= mix(1.0 - uTone.y, 1.0, smoothstep(-0.12, 0.08, dot(q, uSun.xyz)));
    float a = uTone.x * l.alpha * d * shown * smoothstep(0.0, 0.12, facing);
    over(acc, col, a);
  }
  outColor = acc;
}
`;
