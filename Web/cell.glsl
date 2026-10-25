#version 300 es
// Terra Incognita: one fragment per character cell. It inverts the map projection at the cell,
// samples the world (distance fields of coast, borders, rivers and ranges; forest, desert, ice,
// elevation, night lights, sea depth), decides what the cell shows and picks its character.
// Strokes take their character from the line's direction through the cell (- _ ¯ | / \);
// shapes are engraved: an outline where they end, hatching on their lit side; solid accents and
// shaded relief take a density ramp measured from the font. render.js injects the defines below.
precision highp float;
precision highp int;
//%DEFINES%

uniform sampler2D uGeo, uGeo2, uAux, uAux2;
uniform vec2 uGrid, uCell, uView, uCam;
uniform float uScale, uDelta, uTime, uDetail;
uniform int uProj;
uniform vec2 uHemi;
uniform vec2 uHemiC;           // the eastern hemisphere's centre (lon, lat); the western one is its antipode
uniform float uOvalLon;
uniform vec2 uGlobe;
uniform vec3 uPal[NPAL];
uniform float uDark, uIllum, uRelief, uLabels, uKellsAll;
uniform vec4 uKells;            // lon, lat, radius (screen px), amount
uniform vec4 uSun;              // unit vector to the sun (x towards 0°E, y towards 90°E, z north), night strength
uniform float uSunAz;           // light direction for relief, radians
uniform vec4 uKnock[NKNOCK]; uniform int uKnockN;
uniform vec4 uSpr[NSPR]; uniform vec4 uSprP[NSPR]; uniform int uSprN;
uniform vec4 uRose[NROSE]; uniform int uRoseN;
uniform highp usampler2D uArtTex;
uniform vec4 uArt[NART]; uniform vec2 uArtA[NART];   // per drawing: first row, width, height, frames; anchor
uniform vec4 uAS[NASPR]; uniform int uASN;             // per creature on screen: anchor (px), drawing, frame
uniform vec4 uMark[NMARK]; uniform int uMarkN;
uniform vec4 uReveal;           // x progress 0..1, yz origin (screen px), w spread (px)
uniform vec2 uStars;            // parallax offset of the sky (px)

layout(location = 0) out vec4 oFg;
layout(location = 1) out vec4 oBg;

const float PI = 3.14159265;
vec2 SP; int CX, CY;
int G; vec3 FG; float I; vec3 BG;
float LIT;                      // 0 dark .. 1 lit, for ramps
void put(int g, vec3 c, float i) { G = g; FG = c; I = i; }
vec3 P_(int i) { return uPal[i]; }

// ------------------------------------------------------------------ noise
float h12(vec2 p) { vec3 p3 = fract(vec3(p.xyx) * 0.1031); p3 += dot(p3, p3.yzx + 33.33); return fract((p3.x + p3.y) * p3.z); }
float vnoise(vec2 p) {
  vec2 i = floor(p), f = fract(p), u = f * f * (3.0 - 2.0 * f);
  return mix(mix(h12(i), h12(i + vec2(1, 0)), u.x), mix(h12(i + vec2(0, 1)), h12(i + vec2(1, 1)), u.x), u.y);
}
float fbm(vec2 p) { return 0.5 * vnoise(p) + 0.25 * vnoise(p * 2.03 + 17.1) + 0.125 * vnoise(p * 4.1 + 3.3); }
float wrapPI(float a) { return a - 2.0 * PI * floor((a + PI) / (2.0 * PI)); }

// ------------------------------------------------------------------ characters
int ramp(float v) { return RAMP[clamp(int(v * float(NRAMP - 1) + 0.5), 0, NRAMP - 1)]; }
// A stroke through the cell: d is the signed distance (px) from the cell's centre to the line, n
// its unit normal (screen, y down). -1 if the line misses this cell.
int strokeG(float d, vec2 n) {
  float e = abs(n.x) * uCell.x * 0.5 + abs(n.y) * uCell.y * 0.5;
  if (abs(d) > e) return -1;
  vec2 t = vec2(-n.y, n.x);
  float ang = degrees(atan(abs(t.y), abs(t.x)));
  if (ang < 24.0) {
    float oy = -d * n.y;
    return oy > uCell.y * 0.17 ? G_US : (oy < -uCell.y * 0.17 ? G_MACRON : G_DASH);
  }
  if (ang > 70.0) return G_PIPE;
  return (t.x * t.y < 0.0) ? G_SLASH : G_BSLASH;
}
struct F { float d; vec2 n; };
F field(float f0, float fx, float fy) {
  vec2 g = vec2(fx - f0, fy - f0) / uDelta;
  float L = max(length(g), 1e-6);
  return F(f0 / L, g / L);
}

// Fields sampled from the textures: the gradient is taken in degrees, a few texels apart (so
// 8-bit steps don't jitter it), then carried to the screen through the projection's Jacobian.
vec2 Jx, Jy;        // degrees of (lon, lat) per screen px, along x and along y
float E;            // the sampling step, in degrees
vec2 wrapd(vec2 d) { d.x -= 360.0 * floor((d.x + 180.0) / 360.0); return d; }
F fieldLL(float f0, float fE, float fN) {
  vec2 gll = vec2(fE - f0, fN - f0) / E;
  vec2 gs = vec2(dot(Jx, gll), dot(Jy, gll));
  float L = max(length(gs), 1e-6);
  return F(f0 / L, gs / L);
}

// ------------------------------------------------------------------ the map's projections
vec2 uvOf(vec2 ll) { return vec2((ll.x + 180.0) / 360.0, (90.0 - ll.y) / 180.0); }
vec2 inv(vec2 P, int h, out float rho, out vec3 nrm) {
  nrm = vec3(0.0, 0.0, 1.0);
  if (uProj == 0) {            // two hemispheres, azimuthal equidistant (oblique when zoomed in)
    vec2 q = P - (h == 0 ? -uHemi : uHemi);
    rho = length(q);
    vec2 C = h == 1 ? uHemiC : vec2(uHemiC.x + 180.0, -uHemiC.y);
    float X = q.x * 1.5707963, Y = -q.y * 1.5707963, c = min(rho, 1.9) * 1.5707963;
    float p1 = radians(C.y), sc = sin(c), cc = cos(c);
    float lat = asin(clamp(cc * sin(p1) + (c > 1e-6 ? Y * sc * cos(p1) / c : 0.0), -1.0, 1.0));
    float lon = C.x + degrees(atan(X * sc, c * cos(p1) * cc - Y * sin(p1) * sc));
    return vec2(lon, degrees(lat));
  } else if (uProj == 1) {     // the Ortelius-style oval: Hammer
    float X = P.x * 1.4142136, Y = -P.y * 1.4142136;
    rho = sqrt(P.x * P.x * 0.25 + P.y * P.y);
    float z = sqrt(max(1.0 - X * X / 16.0 - Y * Y / 4.0, 0.0));
    return vec2(uOvalLon + degrees(2.0 * atan(z * X, 2.0 * (2.0 * z * z - 1.0))), degrees(asin(clamp(z * Y, -1.0, 1.0))));
  }
  float x = P.x, y = -P.y;     // the globe: orthographic
  rho = length(vec2(x, y));
  float r = min(rho, 0.99999), c = asin(r), sc = sin(c), cc = cos(c), p0 = radians(uGlobe.y);
  float lat = asin(clamp(cc * sin(p0) + (rho > 1e-6 ? y * sc * cos(p0) / rho : 0.0), -1.0, 1.0));
  float lon = uGlobe.x + degrees(atan(x * sc, max(rho, 1e-6) * cc * cos(p0) - y * sc * sin(p0)));
  nrm = vec3(x, y, sqrt(max(0.0, 1.0 - r * r)));
  return vec2(lon, degrees(lat));
}
vec2 fwd(vec2 ll) {             // lon/lat -> plane, for points sampled inside the shader
  float lam = radians(ll.x), phi = radians(ll.y);
  if (uProj == 0) {
    float p1 = radians(uHemiC.y), dl = lam - radians(uHemiC.x);
    float cE = sin(p1) * sin(phi) + cos(p1) * cos(phi) * cos(dl);
    int h = cE >= 0.0 ? 1 : 0;
    if (h == 0) { p1 = -p1; dl = lam - radians(uHemiC.x + 180.0); }
    float cc = clamp(sin(p1) * sin(phi) + cos(p1) * cos(phi) * cos(dl), -1.0, 1.0), c = acos(cc), k = c < 1e-5 ? 1.0 : c / sin(c);
    vec2 q = vec2(k * cos(phi) * sin(dl), -k * (cos(p1) * sin(phi) - sin(p1) * cos(phi) * cos(dl))) / 1.5707963;
    return q + (h == 0 ? -uHemi : uHemi);
  } else if (uProj == 1) {
    float dl = radians(mod(ll.x - uOvalLon + 540.0, 360.0) - 180.0);
    float s = sqrt(1.0 + cos(phi) * cos(dl * 0.5));
    return vec2(2.0 * cos(phi) * sin(dl * 0.5) / s, -sin(phi) / s);
  }
  float dl = lam - radians(uGlobe.x), p0 = radians(uGlobe.y);
  return vec2(cos(phi) * sin(dl), -(cos(p0) * sin(phi) - sin(p0) * cos(phi) * cos(dl)));
}
vec2 toScreen(vec2 P) { return (P - uCam) * uScale + 0.5 * uView; }

// decoders, mirroring tools/build_textures.py (distances in texels of the texture they live in)
float decCoast(vec4 g) { float coarse = (g.g * 255.0 - 128.0) * 2.0; return abs(coarse) < 14.0 ? (g.r * 255.0 - 128.0) / 8.0 : coarse; }
float decBorder(vec4 g) { return g.b * 255.0 / 8.0; }
float decRiver(vec4 g) { return g.a * 255.0 / 16.0; }
float decSigned4(float v) { return (v * 255.0 - 128.0) / 4.0; }
float decEire(vec4 g) { return (g.b * 255.0 - 128.0) / 2.0; }

// ------------------------------------------------------------------ shapes
float sdCircle(vec2 p, float r) { return length(p) - r; }
float sdBox(vec2 p, vec2 b) { vec2 d = abs(p) - b; return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0); }
float sdSeg(vec2 p, vec2 a, vec2 b) { vec2 pa = p - a, ba = b - a; float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0); return length(pa - ba * h); }
float sdTri(vec2 p, vec2 p0, vec2 p1, vec2 p2) {
  vec2 e0 = p1 - p0, e1 = p2 - p1, e2 = p0 - p2, v0 = p - p0, v1 = p - p1, v2 = p - p2;
  vec2 pq0 = v0 - e0 * clamp(dot(v0, e0) / dot(e0, e0), 0.0, 1.0);
  vec2 pq1 = v1 - e1 * clamp(dot(v1, e1) / dot(e1, e1), 0.0, 1.0);
  vec2 pq2 = v2 - e2 * clamp(dot(v2, e2) / dot(e2, e2), 0.0, 1.0);
  float s = sign(e0.x * e2.y - e0.y * e2.x);
  vec2 d = min(min(vec2(dot(pq0, pq0), s * (v0.x * e0.y - v0.y * e0.x)), vec2(dot(pq1, pq1), s * (v1.x * e1.y - v1.y * e1.x))), vec2(dot(pq2, pq2), s * (v2.x * e2.y - v2.y * e2.x)));
  return -sqrt(d.x) * sign(d.y);
}
float sdEll(vec2 p, vec2 r) { float k = length(p / r); return (k - 1.0) * min(r.x, r.y); }
// (distance, material, light) of the nearest part
vec3 U(vec3 a, vec3 b) { return a.x < b.x ? a : b; }

// materials
#define M_BODY 1.0
#define M_SPIKE 2.0
#define M_EYE 3.0
#define M_WHALE 4.0
#define M_SPRAY 5.0
#define M_FIRE 6.0
#define M_HULL 7.0
#define M_SAIL 8.0
#define M_CROSS 9.0
#define M_FLAG 10.0
#define M_GOLD 11.0
#define M_VERM 12.0
#define M_VERD 13.0
#define M_GHOST 14.0
#define M_MOON 15.0
#define M_SMOKE 16.0
#define M_CLOUD 17.0
#define M_RIBBON 18.0
#define M_STONE 19.0
#define M_MOSAIC 20.0
#define M_DRAGON 21.0
#define M_HOLE 22.0
bool solidMat(float m) { return m == M_EYE || m == M_FIRE || m == M_CROSS || m == M_FLAG || m == M_MOSAIC || m == M_MOON || (m == M_GOLD && uIllum > 0.5); }
vec3 matCol(float m, float lit) {
  vec3 a, b;
  if (m == M_BODY) { a = P_(P_VERD); b = P_(P_VERDHI); }
  else if (m == M_SPIKE || m == M_VERM || m == M_CROSS || m == M_FLAG || m == M_DRAGON) { a = P_(P_VERM); b = P_(P_VERMHI); }
  else if (m == M_EYE || m == M_GOLD) { a = P_(P_GOLD); b = P_(P_GOLDHI); }
  else if (m == M_WHALE) { a = P_(P_WHALE); b = P_(P_WHALEHI); }
  else if (m == M_SPRAY || m == M_GHOST) { a = P_(P_COAST); b = P_(P_SPRAY); }
  else if (m == M_FIRE) { a = P_(P_FIRE); b = P_(P_GOLDHI); }
  else if (m == M_HULL) { a = P_(P_HULL); b = P_(P_MOUNTHI); }
  else if (m == M_SAIL) { a = P_(P_VELLUM); b = P_(P_SAIL); }
  else if (m == M_VERD) { a = P_(P_VERD); b = P_(P_VERDHI); }
  else if (m == M_MOON) { a = P_(P_MOON); b = P_(P_GOLDHI); }
  else if (m == M_SMOKE || m == M_CLOUD) { a = P_(P_CLOUD); b = P_(P_CLOUDHI); }
  else if (m == M_RIBBON) { a = P_(P_GOLD); b = P_(P_GOLDHI); }
  else if (m == M_STONE) { a = P_(P_VELLUM); b = P_(P_VELLUMHI); }
  else if (m == M_MOSAIC) { int k = int(h12(vec2(CX, CY) + floor(uTime * 2.0)) * 4.0); a = b = k == 0 ? P_(P_GOLDHI) : k == 1 ? P_(P_LAPIS) : k == 2 ? P_(P_VERDHI) : P_(P_VERMHI); }
  else { a = P_(P_VELLUM); b = P_(P_VELLUMHI); }
  return mix(a, b, clamp(lit, 0.0, 1.0));
}

// ---- the ornaments, in local units (y down)
vec3 rose(vec2 u) {
  vec3 r = vec3(1e5, 0.0, 0.0);
  for (int j = 0; j < 16; j++) {
    bool lng = j < 8;
    float a = lng ? float(j) * PI / 4.0 : PI / 8.0 + float(j - 8) * PI / 4.0;
    float L = lng ? ((j % 2) == 1 ? 0.78 : 1.0) : 0.52, W = lng ? ((j % 2) == 1 ? 0.1 : 0.13) : 0.075, b = L * 0.2;
    vec2 dir = vec2(sin(a), -cos(a)), per = vec2(cos(a), sin(a));
    vec2 q = vec2(dot(u, dir), dot(u, per));
    float m = lng ? ((j % 2) == 1 ? M_VERM : M_GOLD) : M_VERD;
    float lit = sdTri(q, vec2(0.0), vec2(b, W), vec2(L, 0.0)), dark = sdTri(q, vec2(0.0), vec2(b, -W), vec2(L, 0.0));
    r = U(r, vec3(lit, m, 0.95));
    r = U(r, vec3(dark, m, 0.2));
  }
  r = U(r, vec3(sdCircle(u, 0.07), M_GOLD, 1.0));
  float ring = abs(length(u) - 1.02) - 0.028;
  float ang = atan(u.x, -u.y);
  float tick = max(abs(length(u) - 1.06) - 0.04, abs(fract(ang / (PI / 16.0) + 0.5) - 0.5) * (PI / 16.0) * length(u) - 0.01);
  r = U(r, vec3(min(ring, tick), M_GOLD, 0.8));
  vec2 f = u - vec2(0.0, -1.17);
  float fleur = min(min(sdBox(f, vec2(0.05, 0.12)), sdEll(f - vec2(0.0, -0.03), vec2(0.09, 0.15))), min(sdCircle(f - vec2(0.085, -0.04), 0.06), sdCircle(f - vec2(-0.085, -0.04), 0.06)));
  r = U(r, vec3(fleur, M_GOLD, 0.95));
  return r;
}
vec3 moonShape(vec2 u, float ph) {
  float d = sdCircle(u, 1.0);
  float z = sqrt(max(0.0, 1.0 - dot(u, u)));
  float lit = u.x * sin(2.0 * PI * ph) + z * (-cos(2.0 * PI * ph));
  float crater = fbm(u * 2.2 + vec2(4.0, 7.0));
  return vec3(d, M_MOON, lit > 0.0 ? clamp(0.45 + 0.6 * sqrt(lit) - (crater - 0.45) * 0.8, 0.0, 1.0) : 0.05);
}
vec3 ribbon(vec2 u, float hr) {
  // a banderole: the band, its folded tails behind and lower, with swallowtail notches
  float yc = 0.22 * hr * sin(u.x * PI);
  float band = max(abs(u.y - yc) - hr * 0.5, abs(u.x) - 1.0);
  vec3 r = vec3(band, M_RIBBON, 0.85);
  float ax = abs(u.x);
  float y0 = 0.22 * hr * sin(sign(u.x) * 0.95 * PI) + hr * 0.5;
  float tail = max(max(abs(u.y - y0) - hr * 0.5, abs(ax - 1.09) - 0.23), (ax - 1.32) + (0.14 - abs(u.y - y0) / hr * 0.28));
  tail = max(tail, -band);
  return U(r, vec3(tail, M_RIBBON, 0.45));
}
vec3 windHead(vec2 u, float t) {
  vec3 r = vec3(1e5, 0.0, 0.0);
  float c = min(min(sdCircle(u - vec2(-1.2, 0.0), 1.0), sdCircle(u - vec2(-0.3, -0.55), 0.75)), min(sdCircle(u - vec2(-0.35, 0.6), 0.7), sdCircle(u - vec2(0.2, 0.05), 0.55)));
  c = min(c, min(sdCircle(u - vec2(-2.0, -0.4), 0.7), sdCircle(u - vec2(-2.0, 0.45), 0.65)));
  r = U(r, vec3(c, M_CLOUD, clamp(0.4 - u.y * 0.35 - u.x * 0.1, 0.0, 1.0)));
  r = U(r, vec3(min(min(sdCircle(u - vec2(-0.45, -0.22), 0.1), sdCircle(u - vec2(-0.45, 0.28), 0.1)), sdCircle(u - vec2(0.42, 0.05), 0.15)), M_HOLE, 0.0));
  return r;
}
vec3 tower(vec2 u) {
  vec3 r = vec3(sdBox(u - vec2(0.0, -1.0), vec2(0.28, 1.0)), M_STONE, u.x < 0.0 ? 0.9 : 0.35);
  r = U(r, vec3(sdTri(u, vec2(-0.34, -2.0), vec2(0.34, -2.0), vec2(0.0, -2.6)), M_VERM, u.x < 0.0 ? 0.9 : 0.4));
  r = U(r, vec3(min(sdBox(u - vec2(0.0, -1.7), vec2(0.06, 0.1)), sdBox(u - vec2(0.0, -0.25), vec2(0.08, 0.2))), M_HOLE, 0.0));
  return r;
}
vec3 glint(vec2 u, float t, float seed, float m) {
  float pulse = 0.55 + 0.45 * pow(sin(t * 1.4 + seed * 6.28), 2.0);
  vec2 a = abs(u) / (0.75 + 0.5 * pulse);
  float star = min(max(a.x - 0.16 * (1.0 - a.y), a.y - 1.0), max(a.y - 0.16 * (1.0 - a.x / 0.75), a.x - 0.75));
  star = min(star, length(a) - 0.18);
  vec3 r = vec3(star, m, clamp(pulse + 0.2 - length(a) * 0.4, 0.0, 1.0));
  float ring = abs(length(u) - (1.0 + 0.6 * fract(t * 0.35 + seed))) - 0.04;
  return U(r, vec3(ring + 0.9 * fract(t * 0.35 + seed) * 0.12, M_GHOST, 0.9));
}
vec3 phantom(vec2 u, float t) {
  float d = abs(sdCircle(u, 0.97)) - 0.06;
  float chan = abs(u.y - 0.06 * sin(u.x * 6.0)) - 0.13;
  d = max(d, -chan);
  float shore = max(abs(abs(u.y - 0.06 * sin(u.x * 6.0)) - 0.18) - 0.05, length(u) - 0.97);
  d = min(d, shore);
  float dash = sin(atan(u.y, u.x) * 18.0) > -0.3 ? 0.0 : 1.0;
  return vec3(d + dash, M_GHOST, 0.55 + 0.45 * sin(t * 0.7));
}
vec3 sprite(int type, vec2 u, vec4 p, float t) {
  if (type == 5) return rose(u);
  if (type == 6) return moonShape(u, p.y);
  if (type == 7) return ribbon(u, p.y);
  if (type == 8) return windHead(u, t);
  if (type == 9) return glint(u, t, p.y, p.z > 1.5 ? M_MOSAIC : (p.z > 0.5 ? M_GHOST : M_GOLD));
  if (type == 10) return phantom(u, t);
  if (type == 11) return tower(u);
  if (type == 12) return vec3(min(sdEll(u, vec2(0.8, 0.45)), sdBox(u - vec2(0.0, -1.0), vec2(0.1, 0.6))), M_STONE, 0.7);
  return vec3(1e5, 0.0, 0.0);
}
float spriteExtent(int type) {
  if (type == 5) return 1.4;
  if (type == 7) return 1.6;
  if (type == 8) return 3.2;
  if (type == 11) return 2.8;
  if (type == 9) return 2.0;
  return 1.3;
}
// Draw sprite i over the cell, engraved. Returns true if the sprite covers the cell.
bool drawSprite(int i, out bool covered) {
  covered = false;
  vec4 s = uSpr[i], p = uSprP[i];
  int type = int(s.w);
  float dir = p.x == 0.0 ? 1.0 : p.x;
  vec2 cen = s.xy, d0 = (SP - cen) / s.z;
  vec2 u = vec2(d0.x * dir, d0.y);
  float ext = spriteExtent(type);
  if (abs(u.x) > ext + 0.5 || abs(u.y) > ext + 0.5) return false;
  float t = uTime + p.w * 13.0;
  vec3 a = sprite(type, u, p, t);
  float cellU = max(uCell.x, uCell.y) / s.z;
  if (a.x > cellU) return false;
  vec3 ax = sprite(type, u + vec2(dir * uDelta / s.z, 0.0), p, t), ay = sprite(type, u + vec2(0.0, uDelta / s.z), p, t);
  F f = field(a.x * s.z, ax.x * s.z, ay.x * s.z);
  f.n.x *= dir;
  float m = a.y, lit = a.z;
  if (m == M_HOLE) { covered = a.x < 0.0; if (covered) { put(G_SP, FG, 0.0); } return covered; }
  if (solidMat(m) && a.x < 0.0) {
    put(ramp(clamp(0.45 + 0.55 * lit, 0.0, 1.0)), matCol(m, lit), 1.15);
    covered = true; return true;
  }
  int g = strokeG(f.d, f.n);
  if (g >= 0) { put(g, matCol(m, max(lit, 0.7)), 1.05); covered = true; return true; }
  if (a.x < 0.0) {
    // inside: shaded in a ramp of characters by its light, as an engraver would model it
    if (m == M_GHOST || m == M_SMOKE) { put(G_SP, FG, 0.0); covered = true; return true; }
    float l = uDark > 0.5 ? lit : 1.0 - lit;
    put(ramp(clamp(0.18 + 0.62 * l, 0.0, 1.0)), matCol(m, lit), 0.85 + 0.3 * l);
    covered = true; return true;
  }
  return false;
}

// ------------------------------------------------------------------ after the Book of Kells
// The pigments of an Insular gospel book: iron-gall ink, gold, red lead, orpiment, verdigris,
// lapis and a folium purple, on calfskin.
const vec3 K_INK = vec3(0.11, 0.07, 0.035), K_GOLD = vec3(0.80, 0.58, 0.12), K_GOLDHI = vec3(0.98, 0.82, 0.36);
const vec3 K_RED = vec3(0.78, 0.20, 0.13), K_ORP = vec3(0.90, 0.68, 0.16), K_GREEN = vec3(0.16, 0.52, 0.31), K_GREENHI = vec3(0.45, 0.78, 0.38);
const vec3 K_LAPIS = vec3(0.17, 0.30, 0.66), K_PURPLE = vec3(0.45, 0.25, 0.48), K_VELLUM = vec3(0.93, 0.86, 0.71), K_DEEP = vec3(0.05, 0.09, 0.07);
bool kellsDark() { return uKellsAll > 0.5; }
vec3 kOn(vec3 c) { return kellsDark() ? mix(c, vec3(1.0), 0.18) : c; }

// A three-armed spiral (the Insular triskele) around c, radius R: a stroke along its arms.
bool triskele(vec2 c, float R, float spin, vec3 col) {
  vec2 q = SP - c; float r = length(q);
  if (r > R) return false;
  if (r < uCell.x * 0.8) { put(G_BULLET, kOn(K_ORP), 1.25); return true; }
  float th = atan(q.y, q.x), p = max(R / 2.4, uCell.y * 1.55);
  float ph = r / p - 3.0 * th / (2.0 * PI) + spin;
  float f = fract(ph); float s = f < 0.5 ? f : f - 1.0;
  vec2 rh = q / r, tt = vec2(-rh.y, rh.x);
  vec2 grad = rh / p - (3.0 / (2.0 * PI * r)) * tt;
  float L = length(grad);
  int g = strokeG(s / L, grad / L);
  if (g >= 0) { put(g, col, 1.25); return true; }
  // the ring that closes it
  int g2 = strokeG(r - R + uCell.y * 0.4, rh);
  if (g2 >= 0) { put(g2, kellsDark() ? K_GOLD : K_INK, 1.05); return true; }
  return false;
}
// A round-topped tree as the film draws them: a canopy ring with a heart, a straight trunk.
bool kellsTree(vec2 c, float R, vec3 col, vec3 ink) {
  vec2 q = SP - c;
  if (abs(q.x) < uCell.x * 0.5 && q.y > R * 0.85 && q.y < R * 2.0) { put(G_PIPE, ink, 1.0); return true; }
  vec2 e = q * vec2(1.0, 0.9);
  float d = length(e) - R;
  int g = strokeG(d, normalize(e + 1e-5));
  if (g >= 0) { put(g, col, 1.2); return true; }
  if (d < 0.0) { put(length(q) < R * 0.4 ? G_CLUB : G_SP, col, 1.1); return true; }
  return false;
}
// The pigments laid flat: the field colour of the land in the Kells treatments.
vec3 kField(int k) {
  return k == 0 ? vec3(0.13, 0.36, 0.22) : k == 1 ? vec3(0.45, 0.16, 0.10) : k == 2 ? vec3(0.15, 0.20, 0.42) : vec3(0.42, 0.31, 0.08);
}
// The land as a carpet page: a flat field of colour, triskeles on a lattice anchored to the map,
// round trees between them where things grow, a stipple where they don't.
void kellsLand(vec2 P, vec4 a0, vec3 field, vec3 ink, vec3 light) {
  BG = field;
  float spPx = uCell.y * 12.0;
  float lvl = log2(spPx / uScale), s0 = exp2(floor(lvl));
  vec2 g = P / s0, ic = floor(g);
  float hh = h12(ic * 1.31 + 0.7);
  vec2 cen = toScreen((ic + 0.5 + (vec2(h12(ic + 3.1), h12(ic + 5.7)) - 0.5) * 0.2) * s0);
  float R = s0 * uScale * 0.4;
  vec3 cols[3] = vec3[3](K_GOLDHI, light, mix(K_RED, vec3(1.0, 0.6, 0.45), 0.35));
  if (triskele(cen, R, uTime * (hh > 0.5 ? 0.12 : -0.12) + hh * 3.0, cols[int(hh * 3.0) % 3])) return;
  float veg = a0.r;
  if (veg > 0.3) {
    float ts = uCell.y * 3.2;
    vec2 base = uCam * uScale / vec2(ts * 0.9, ts * 1.25);
    vec2 tg = SP / vec2(ts * 0.9, ts * 1.25) + base;
    vec2 ti = floor(tg); float row = mod(ti.y, 2.0);
    vec2 tc = (vec2(ti.x + 0.5 + row * 0.5, ti.y + 0.5) - base) * vec2(ts * 0.9, ts * 1.25);
    if (length(tc - cen) > R + ts * 0.7 && kellsTree(tc, ts * 0.4, h12(ti) < 0.5 ? K_GREENHI : light, ink)) return;
  }
  float k = h12(vec2(CX, CY));
  put(k < 0.18 ? G_DOT : k < 0.24 ? G_MIDDOT : G_SP, light, 0.75);
}
// The sea after Kells: a band of plait along every coast, over and under in turn; open water
// quiet, with a ripple now and then.
void kellsSea(float off, vec3 c1, vec3 c2, vec3 ripple) {
  float bandEnd = uCell.y * 5.0;
  if (off < bandEnd) {
    vec2 o = floor(uCam * uScale / uCell);
    float a = float(CX) + o.x + float(CY) + o.y, b = float(CX) + o.x - float(CY) - o.y;
    int i = int(mod(a, 4.0)), j = int(mod(b, 4.0));
    if (i == 0 || j == 0) {
      bool aOver = int(mod(floor(a / 4.0) + floor(b / 4.0), 2.0)) == 0;
      bool showA = i == 0 && (j != 0 || aOver);
      float fade = clamp((bandEnd - off) / (uCell.y * 1.5), 0.0, 1.0);
      put(showA ? G_SLASH : G_BSLASH, showA ? c1 : c2, 1.1 * fade);
      return;
    }
    put(G_SP, FG, 0.0);
    return;
  }
  float row = floor(SP.y / uCell.y + uCam.y * uScale / uCell.y);
  float u = fract((SP.x + uCam.x * uScale + uTime * 4.0 * (mod(row, 2.0) > 0.5 ? 1.0 : -1.0)) / (uCell.x * 11.0) + h12(vec2(row, 3.0)));
  if (mod(row, 3.0) < 0.5 && u < 0.22) { put(G_TILDE, ripple, 0.8); return; }
  put(G_SP, FG, 0.0);
}
// The coast: a heavy line, and a row of red dots standing just off it.
bool kellsCoast(float coastPx, vec2 n, bool land, vec3 line) {
  int g = strokeG(coastPx, n);
  if (g >= 0) { put(g, line, 1.35); return true; }
  float off = -coastPx;
  if (!land && off > uCell.y * 0.6 && off < uCell.y * 1.5) {
    float along = dot(SP + uCam * uScale, vec2(-n.y, n.x)) / uCell.x;
    put(fract(along * 0.5) < 0.5 ? G_MIDDOT : G_SP, mix(K_RED, vec3(1.0, 0.5, 0.35), kellsDark() ? 0.4 : 0.0), 1.35);
    return true;
  }
  return false;
}
// A two-strand braid in a band between rules, around `cen` at radius R (px), width w.
bool braid(vec2 cen, float R, float w, vec3 c1, vec3 c2, vec3 rule, vec3 ground) {
  float r = length(SP - cen);
  float t = (r - R) / (w * 0.5);
  if (abs(t) > 1.35) return false;
  vec2 rad = normalize(SP - cen), tang = vec2(-rad.y, rad.x);
  BG = ground;
  int gr = strokeG(r - R - sign(t) * w * 0.5, rad);
  if (gr >= 0 && abs(abs(t) - 1.0) < 0.38) { put(gr, rule, 1.25); return true; }
  if (abs(t) > 1.0) { put(G_SP, FG, 0.0); return true; }
  float ang = atan(SP.y - cen.y, SP.x - cen.x), s = ang * R / (w * 1.5);
  float k = floor(s + 0.5), nr = abs(s - k);
  for (int j = 0; j < 2; j++) {
    float sg = j == 0 ? 1.0 : -1.0;
    if (nr < 0.17 && int(mod(k + float(j), 2.0)) == 0) continue;
    float y = sg * 0.62 * sin(s * PI);
    float slope = sg * 0.62 * PI * cos(s * PI) * 0.5 / 1.5;
    int g = strokeG((t - y) * w * 0.5 / sqrt(1.0 + slope * slope), normalize(rad - slope * tang));
    if (g >= 0) { put(g, j == 0 ? c1 : c2, 1.25); return true; }
  }
  put(nr > 0.45 && abs(t) < 0.2 ? G_MIDDOT : G_SP, rule, 1.0);
  return true;
}
// Hibernia's roundel: a parchment medallion over Ireland in a dark enamel ring with a gilt
// braid; Ireland laid in verdigris under gold spirals, Britain in orpiment.
bool hibernia(vec2 P, vec4 a0, float coastPx, vec2 coastN, bool land, float eire) {
  if (uKells.w <= 0.001 || kellsDark()) return false;
  vec2 cen = toScreen(fwd(uKells.xy));
  float R = uKells.z, r = length(SP - cen);
  float open = uKells.w;
  float reach = R * smoothstep(0.0, 0.75, open);
  float w = uCell.y * 3.2;
  if (r > R + w) return false;
  if (r > R - w * 0.5 - uCell.y * 0.5) {
    if (open < 0.6) return false;
    float drawn = smoothstep(0.6, 1.0, open);
    float ang = mod(atan(SP.x - cen.x, -(SP.y - cen.y)) + 2.0 * PI, 2.0 * PI) / (2.0 * PI);
    if (ang > drawn) return false;
    return braid(cen, R, w, K_GOLDHI, mix(K_RED, vec3(1.0), 0.25), K_GOLD, vec3(0.10, 0.06, 0.09));
  }
  if (r > reach) return false;
  BG = K_VELLUM;
  if (kellsCoast(coastPx, coastN, land, K_INK)) return true;
  if (land) {
    if (eire > -1.0) kellsLand(P, a0, mix(K_GREEN, K_VELLUM, 0.12), K_INK, vec3(0.86, 0.95, 0.72));
    else { BG = mix(K_ORP, K_VELLUM, 0.35); float k = h12(vec2(CX, CY)); put(k < 0.25 ? G_DOT : G_SP, K_INK, 0.9); }
  } else kellsSea(-coastPx, K_LAPIS, K_PURPLE, K_LAPIS);
  if (open < 1.0 && reach - r < uCell.y * 1.2) put(ramp(0.8), K_GOLD, 1.3);
  return true;
}

// ------------------------------------------------------------------ main
void main() {
  CX = int(gl_FragCoord.x); CY = int(uGrid.y) - 1 - int(gl_FragCoord.y);
  SP = (vec2(CX, CY) + 0.5) * uCell;
  vec2 P = uCam + (SP - 0.5 * uView) / uScale;
  vec2 Px = P + vec2(uDelta / uScale, 0.0), Py = P + vec2(0.0, uDelta / uScale);
  int hemi = (uProj == 0 && length(P - uHemi) < length(P + uHemi)) ? 1 : 0;
  float rho, rhox, rhoy; vec3 nrm, nrmx, nrmy;
  vec2 ll = inv(P, hemi, rho, nrm), llx = inv(Px, hemi, rhox, nrmx), lly = inv(Py, hemi, rhoy, nrmy);
  bool onMap = rho <= 1.0;
  G = G_SP; FG = vec3(0.0); I = 0.0; BG = P_(P_SKY); LIT = 0.0;
  F rim = field(rho - 1.0, rhox - 1.0, rhoy - 1.0);
  Jx = wrapd(llx - ll) / uDelta; Jy = wrapd(lly - ll) / uDelta;
  float hpx = 1.0 / max(length(Jx), 1e-6);                    // screen px per degree here
  E = clamp(length(Jx) * uCell.x * 0.7, 0.13, 4.0);

  // octave coordinates: patterns anchored to the map, switching scale smoothly as it zooms
  float lvl = log2(max(uDetail, 1e-3) * 1.3);
  float o0 = floor(lvl), of = lvl - o0;
  vec2 cellLL0 = floor(ll * exp2(o0)), cellLL1 = floor(ll * exp2(o0 + 1.0));
  float sel = h12(cellLL1 + 7.7);
  vec2 cellLL = sel < of ? cellLL1 : cellLL0;
  float hsh = h12(cellLL);

  if (!onMap) {
    // the night sky behind the sheet, with a little parallax
    vec2 sc = floor((SP + uStars) / uCell);
    float sh = h12(sc * vec2(3.1, 7.7));
    if (sh < 0.03) {
      float tw = 0.5 + 0.5 * sin(uTime * (1.0 + sh * 40.0) + sh * 500.0);
      put(sh < 0.002 ? G_STAR : sh < 0.006 ? G_PLUS : sh < 0.016 ? G_MIDDOT : G_DOT, P_(P_STAR), (0.25 + 0.65 * tw * (sh < 0.006 ? 1.3 : 0.8)) * (uDark > 0.5 ? 1.0 : 0.35));
    }
  } else {
    vec4 g0 = texture(uGeo, uvOf(ll)), gx = texture(uGeo, uvOf(ll + vec2(E, 0.0))), gy = texture(uGeo, uvOf(ll + vec2(0.0, E)));
    vec4 a0 = texture(uAux, uvOf(ll));
    vec4 b0 = texture(uAux2, uvOf(ll));
    vec4 q0 = texture(uGeo2, uvOf(ll));
    float c0 = decCoast(g0);
    F coast = fieldLL(c0, decCoast(gx), decCoast(gy));
    bool land = c0 > 0.0;
    float eire = decEire(q0);
    vec2 mxy = SP;   // screen anchor for patterns that drift
    float night = 0.0;
    if (uSun.w > 0.0) {
      vec3 v = vec3(cos(radians(ll.y)) * cos(radians(ll.x)), cos(radians(ll.y)) * sin(radians(ll.x)), sin(radians(ll.y)));
      night = smoothstep(0.05, -0.12, dot(v, uSun.xyz)) * uSun.w;
    }
    if (!land) {
      BG = P_(P_SEABG);
      float depth = b0.a;
      if (uRelief > 0.5) {
        // the sea by its depth: shelves light, trenches dark, waves catching the light
        float sw = vnoise(ll * 0.8 + vec2(uTime * 0.05, -uTime * 0.03));
        float v = clamp(0.12 + depth * 0.55 + 0.2 * sw, 0.0, 1.0);
        put(ramp(v * 0.55), mix(P_(P_SEA), P_(P_SEAHI), v * 0.6), 0.55 + 0.4 * v);
      } else {
        // staggered wavelets, like the engraved seas of the old charts
        float swell = vnoise(ll * 0.35 + vec2(uTime * 0.05, -uTime * 0.03));
        float row = floor(SP.y / uCell.y + uCam.y * uScale / uCell.y);
        float rowOff = mod(row, 2.0);
        if (rowOff < 0.5) {
          float lw = uCell.x * 9.0;
          float drift = uTime * (mod(floor(row * 0.5), 2.0) > 0.5 ? 3.5 : -3.0);
          float u = fract((SP.x + uCam.x * uScale + drift + h12(vec2(floor(row * 0.5), 11.0)) * lw) / lw);
          if (u < 0.3) {
            bool mid = abs(u - 0.15) < 0.05;
            put(mid && swell > 0.62 ? G_APPROX : G_TILDE, mix(P_(P_SEA), P_(P_SEAHI), swell * 0.35), 0.22 + 0.42 * swell * (mid ? 1.0 : 0.75));
          }
        }
        if (G == G_SP && hsh < 0.02) put(G_DOT, P_(P_SEA), 0.22);
      }
      // moonlight on the water
      float tw = fract(hsh * 91.7 + uTime * 0.21);
      if (tw < 0.006 && night < 0.5) put(tw < 0.002 ? G_STAR : G_PLUS, P_(P_GLINT), 0.9 + 0.6 * (1.0 - tw / 0.006));
      // engraved water-lines along the coast, drifting outward: two far out, four close in
      float off = -coast.d, sp = uCell.y * 0.95, nl = uDetail < 0.9 ? 2.0 : 4.0;
      if (off < sp * (nl + 0.4) && off > sp * 0.35) {
        float k = clamp(floor((off - sp * 0.5) / sp + 0.5), 0.0, nl - 1.0);
        float drift = fract(uTime * 0.18) * sp;
        float dd = off - (sp * 0.5 + k * sp) - drift;
        int g = strokeG(dd, coast.n);
        float fade = clamp(1.15 - off / (sp * (nl + 0.4)), 0.0, 1.0);
        if (g >= 0) put(g, mix(P_(P_SEA), P_(P_COAST), fade), 0.5 + 0.5 * fade);
      }
      // rhumb lines from the compass roses
      for (int i = 0; i < NROSE; i++) {
        if (i >= uRoseN) break;
        vec4 ro = uRose[i];
        if (int(ro.w) != hemi && uProj == 0) continue;
        vec2 v = SP - ro.xy; float rr = length(v);
        if (rr < ro.z * 1.12 || off < uCell.y * 1.2) continue;
        float a = atan(v.x, -v.y), k = floor(a / (PI / 8.0) + 0.5), da = a - k * PI / 8.0;
        float dirA = k * PI / 8.0;
        vec2 n = vec2(cos(dirA), sin(dirA));
        int g = strokeG(rr * sin(da), n);
        if (g >= 0) {
          float fade = clamp(1.15 - rr / (ro.z * 5.5), 0.0, 1.0);
          int cls = int(mod(k, 2.0)) == 0 ? 0 : (int(mod(k, 4.0)) == 1 ? 1 : 2);
          put(g, P_(P_RHUMB0 + cls), (cls == 0 ? 0.42 : 0.32) * fade * (uIllum > 0.5 ? 1.4 : 1.0));
          break;
        }
      }
      // red dots off the coast, in the illuminated ink
      if (uIllum > 0.5 && off > uCell.y * 0.55 && off < uCell.y * 1.45) {
        float along = dot(SP, vec2(-coast.n.y, coast.n.x)) / uCell.x;
        if (fract(along * 0.5) < 0.5) put(G_MIDDOT, P_(P_VERMHI), 1.1);
      }
      if (night > 0.0) { FG *= 1.0 - 0.7 * night; I *= 1.0 - 0.55 * night; BG *= 1.0 - 0.6 * night; }
    } else {
      BG = P_(P_LANDBG);
      float veg = a0.r, arid = a0.g, ice = a0.b, elev = a0.a;
      float desertPoly = decSigned4(q0.a);
      vec2 geoW = ll * 9.0;
      if (uRelief > 0.5) {
        // shaded relief: the GEBCO elevation lit from a moving sun, in a ramp of characters, tinted
        // by height (lowland green, upland tan, high brown, snow)
        float e2 = max(E * 1.5, 0.06);
        float eE = texture(uAux, uvOf(ll + vec2(e2, 0.0))).a, eW = texture(uAux, uvOf(ll - vec2(e2, 0.0))).a;
        float eN = texture(uAux, uvOf(ll + vec2(0.0, e2))).a, eS = texture(uAux, uvOf(ll - vec2(0.0, e2))).a;
        vec2 g = vec2(eE - eW, eN - eS) / (2.0 * e2) * 22.0;
        vec3 n3 = normalize(vec3(-g.x, -g.y, 1.0));
        vec3 L = normalize(vec3(cos(uSunAz), sin(uSunAz), 0.8));
        float rel = dot(n3, L) - L.z;
        float v = clamp(0.3 + rel * 1.5 + elev * 0.9, 0.0, 1.0);
        vec3 tint = elev < 0.06 ? mix(P_(P_GRASS), P_(P_FORESTHI), 0.3) : elev < 0.2 ? mix(P_(P_GRASSHI), P_(P_DESERT), (elev - 0.06) / 0.14)
                  : elev < 0.45 ? mix(P_(P_DESERT), P_(P_MOUNT), (elev - 0.2) / 0.25) : mix(P_(P_MOUNTHI), P_(P_SNOW), clamp((elev - 0.45) / 0.25, 0.0, 1.0));
        if (ice > 0.5) tint = P_(P_ICEHI);
        put(ramp(v), tint * (0.65 + 0.5 * clamp(rel * 2.0 + 0.5, 0.0, 1.0)), 0.7 + 0.5 * v);
      } else if (ice > 0.5) {
        float shm = 0.5 + 0.5 * sin(uTime * 0.8 + hsh * 30.0);
        put(hsh < 0.3 ? G_STAR : hsh < 0.55 ? G_PLUS : hsh < 0.8 ? G_MIDDOT : G_DOT, mix(P_(P_ICE), P_(P_ICEHI), shm), 0.45 + 0.35 * shm);
      } else if (desertPoly > 0.0 || arid > 0.55) {
        float dune = sin(dot(ll, vec2(0.35, -0.9)) * 6.0 + 3.0 * vnoise(ll * 0.6) + uTime * 0.15);
        bool crest = dune > 0.8 && vnoise(ll * vec2(2.0, 1.2) + 7.0) > 0.55;
        put(crest ? G_TILDE : hsh < 0.42 ? G_DOT : hsh < 0.68 ? G_MIDDOT : hsh < 0.86 ? G_COLON : G_QUOTE, mix(P_(P_DESERT), P_(P_DESERTHI), clamp(dune * 0.5 + 0.5, 0.0, 1.0) * 0.7), crest ? 0.95 : 0.6 + 0.25 * clamp(dune * 0.5 + 0.5, 0.0, 1.0));
      } else if (veg > 0.72) {
        float gust = 0.5 + 0.5 * sin(dot(ll, vec2(1.6, -0.7)) * 3.0 - uTime * 1.6 + 2.2 * vnoise(ll * 0.4));
        float clump = vnoise(ll * 2.5 + 3.1);
        if (clump > 0.48 && hsh < 0.78) put(h12(cellLL + 5.0) < 0.62 ? G_CLUB : G_SPADE, mix(P_(P_FOREST), P_(P_FORESTHI), gust * 0.7), 0.6 + 0.5 * gust);
        else { float tl = gust - 0.5; put(tl > 0.3 ? G_SLASH : tl < -0.3 ? G_BSLASH : G_PIPE, P_(P_FOREST), 0.4 + 0.35 * gust); }
      } else if (veg > 0.3) {
        float wv = sin(dot(ll, vec2(1.4, -0.9)) * 3.0 - uTime * 1.6 + 2.5 * vnoise(ll * 0.3));
        float f = clamp(wv * 0.5 + 0.5, 0.0, 1.0);
        put(wv > 0.4 ? G_SLASH : wv < -0.4 ? G_BSLASH : (hsh < 0.45 ? G_PIPE : hsh < 0.7 ? G_COMMA : hsh < 0.85 ? G_QUOTE : G_SEMI), mix(P_(P_GRASS), P_(P_GRASSHI), f * 0.85), 0.4 + 0.45 * f);
      } else {
        put(hsh < 0.3 ? G_DOT : hsh < 0.48 ? G_QUOTE : hsh < 0.6 ? G_BTICK : hsh < 0.78 ? G_COLON : G_MIDDOT, P_(P_VELLUM), 0.5);
      }
      // a wash of colour inside each country's border, as maps were coloured by hand
      float bd = decBorder(g0);
      F border = fieldLL(bd, decBorder(gx), decBorder(gy));
      int wsh = int(texelFetch(uGeo2, ivec2(uvOf(ll) * vec2(2048.0, 1024.0)) % ivec2(2048, 1024), 0).g * 255.0 / 60.0 + 0.5);
      if (wsh >= 1 && wsh <= 4 && border.d < uCell.x * 3.2 && uDetail > 0.5) {
        float f = (1.0 - border.d / (uCell.x * 3.2)) * 0.75;
        FG = mix(FG, P_(P_WASH0 + wsh - 1), f);
        if (G == G_SP && border.d < uCell.x * 1.6) put(G_DOT, FG, 0.4);
        I = max(I, 0.4 + 0.3 * f);
      }
      // rivers and borders
      float rv = decRiver(g0);
      F river = fieldLL(rv, decRiver(gx), decRiver(gy));
      // rivers and borders are unsigned distances: a stroke where the distance falls below half a cell
      if (uDetail > 0.45 && river.d < uCell.x * 0.5) { int g = strokeG(0.0, river.n); if (g >= 0) put(g, P_(P_RIVER), 0.9); }
      if (uDetail > 0.55 && border.d < uCell.x * 0.5 && ((CX + CY) % 3) != 0) { int g = strokeG(0.0, border.n); if (g >= 0) put(g, P_(P_BORDER), 0.8); }
      // mountains drawn as little hills: a lattice anchored to the map, kept where ranges are
      if (uRelief < 0.5) {
        float mpoly = decSigned4(q0.r);
        if (mpoly > -12.0) {
          float iconPx = uCell.x * clamp(4.0 + 2.0 * log2(max(uDetail, 0.25) / 0.7 + 1.0), 4.0, 13.0);
          float stepDeg = iconPx * 0.78 / hpx;
          float sl = log2(stepDeg), s0 = exp2(floor(sl));
          float best = -1e5; vec3 hit = vec3(0.0);
          for (int oy = -1; oy <= 2; oy++) for (int ox = -1; ox <= 1; ox++) {
            vec2 gi = floor(ll / s0) + vec2(float(ox), float(-oy));
            float j1 = h12(gi * 1.37), j2 = h12(gi * 2.71 + 3.3);
            vec2 L = (gi + vec2(0.5 + (j1 - 0.5) * 0.6 + mod(gi.y, 2.0) * 0.25, 0.5 + (j2 - 0.5) * 0.4)) * s0;
            if (decSigned4(texture(uGeo2, uvOf(L)).r) < 0.5 || decCoast(texture(uGeo, uvOf(L))) < 3.0) continue;
            vec2 S = toScreen(fwd(L));
            float w = iconPx * (0.82 + j1 * 0.36), h = w * 0.85 * (0.9 + j2 * 0.35);
            vec2 u = (SP - S) / vec2(w, h);
            if (u.y < -1.05 || u.y > 0.05 || abs(u.x) > 0.6) continue;
            if (S.y > best) {
              float half_ = 0.5 * (u.y + 1.0);
              if (abs(u.x) <= half_ + 0.02) { best = S.y; hit = vec3(u, j2); }
            }
          }
          if (best > -1e4) {
            vec2 u = hit.xy; float half_ = 0.5 * (u.y + 1.0);
            float edge = (half_ - abs(u.x)) * iconPx * 0.86;
            bool snowy = hit.z > 0.7 || elev > 0.35;
            vec3 col = snowy && u.y < -0.62 ? P_(P_SNOW) : P_(P_MOUNTHI);
            vec2 n = normalize(vec2(sign(u.x), 0.5));
            int g = strokeG(edge - uCell.x * 0.45, n);
            if (edge < uCell.x * 0.9) put(u.x < 0.0 ? G_SLASH : G_BSLASH, col, 1.0);
            else if (u.x > 0.04) { float hv = fract((SP.x + SP.y * 0.55) / (uCell.x * 2.3)); put(hv < 0.4 ? G_BSLASH : G_SP, P_(P_MOUNT), 0.6); }
            else put(hsh < 0.25 ? G_DOT : G_SP, P_(P_MOUNT), 0.55);
          }
        }
      }
      if (night > 0.0) {
        FG *= 1.0 - 0.75 * night; I *= 1.0 - 0.6 * night; BG *= 1.0 - 0.6 * night;
        float lights = b0.b;
        if (lights > 0.12 && h12(vec2(CX, CY) + 2.0) < lights * 2.2) put(lights > 0.5 ? G_STAR : lights > 0.28 ? G_PLUS : G_MIDDOT, P_(P_LIGHTS), (0.8 + 0.7 * lights) * night + (1.0 - night) * I);
        if (night > 0.15 && night < 0.6) { FG = mix(FG, P_(P_FIRE), 0.35); }
      }
      // the town marks of capitals
      for (int i = 0; i < NMARK; i++) {
        if (i >= uMarkN) break;
        vec2 d = SP - uMark[i].xy;
        if (abs(d.x) < uCell.x * 0.5 && abs(d.y) < uCell.y * 0.5) { put(G_O, uMark[i].w > 0.5 ? P_(P_GOLDHI) : P_(P_VERMHI), 1.1); break; }
      }
    }
    // graticule and the named parallels
    {
      float dl = abs(mod(ll.x + 15.0, 30.0) - 15.0), dlx = abs(mod(llx.x + 15.0, 30.0) - 15.0), dly = abs(mod(lly.x + 15.0, 30.0) - 15.0);
      float dp = abs(mod(ll.y + 15.0, 30.0) - 15.0), dpx_ = abs(mod(llx.y + 15.0, 30.0) - 15.0), dpy = abs(mod(lly.y + 15.0, 30.0) - 15.0);
      if (abs(ll.y) < 80.0) {
        F m1 = field(dl, dlx, dly), m2 = field(dp, dpx_, dpy);
        int g1 = strokeG(m1.d, m1.n), g2 = strokeG(m2.d, m2.n);
        if ((g1 >= 0 || g2 >= 0) && (G == G_SP || !land) && I < 0.35) put(g1 >= 0 ? g1 : g2, P_(P_GOLD), 0.2 + 0.15 * uIllum);
      }
      float best = 1e5; float bx = 0.0, by = 0.0;
      float pars[5] = float[5](0.0, 23.44, -23.44, 66.56, -66.56);
      for (int i = 0; i < 5; i++) { float d = abs(ll.y - pars[i]); if (d < best) { best = d; bx = abs(llx.y - pars[i]); by = abs(lly.y - pars[i]); } }
      F tp = field(best, bx, by);
      int g = strokeG(tp.d, tp.n);
      if (g >= 0 && fract(SP.x / (uCell.x * 4.4)) < 0.5 && !land) put(g, P_(P_VERM), 0.42);
    }
    // the coast itself
    {
      int g = strokeG(coast.d, coast.n);
      if (g >= 0) put(g, uIllum > 0.5 ? P_(P_INK) : P_(P_VELLUMHI), uIllum > 0.5 ? 1.2 : 0.95);
    }
    if (kellsDark()) {
      // the Kells ink: the whole world as a carpet page, each country laid in its own pigment
      BG = K_DEEP;
      if (!kellsCoast(coast.d, coast.n, land, K_GOLDHI)) {
        if (land) {
          int wk = int(texelFetch(uGeo2, ivec2(uvOf(ll) * vec2(2048.0, 1024.0)) % ivec2(2048, 1024), 0).g * 255.0 / 60.0 + 0.5);
          kellsLand(P, a0, kField(max(wk - 1, 0)) * (a0.b > 0.5 ? 0.6 : 1.0), K_INK, vec3(0.95, 0.88, 0.68));
        } else kellsSea(-coast.d, vec3(0.38, 0.52, 0.95), vec3(0.62, 0.42, 0.72), vec3(0.32, 0.44, 0.8));
      }
    } else hibernia(P, a0, coast.d, coast.n, land, eire);
  }
  // the rim: a double gold rule around each world
  {
    int g = strokeG(rim.d, rim.n);
    if (g >= 0) put(g, P_(P_GOLDHI), 1.0);
    int g2 = strokeG(rim.d - uCell.y * 1.6, rim.n);
    if (g2 >= 0 && !onMap) put(g2, P_(P_GOLD), 0.8);
    // a band of interlace beyond the rule, in the illuminated ink
    if (uIllum > 0.5 && !onMap) {
      float w = uCell.y * 2.2, r0 = uCell.y * 2.6;
      float t = (rim.d - r0 - w * 0.5) / (w * 0.5);
      if (abs(t) < 1.25) {
        int g3 = strokeG(rim.d - r0 - w * (t > 0.0 ? 1.0 : 0.0), rim.n);
        if (g3 >= 0 && abs(abs(t) - 1.0) < 0.3) put(g3, P_(P_GOLD), 1.0);
        else {
          vec2 Pc = uProj == 0 ? (hemi == 0 ? -uHemi : uHemi) : vec2(0.0);
          vec2 cen = toScreen(Pc);
          float ang = atan(SP.y - cen.y, SP.x - cen.x), R = length(SP - cen);
          float s = ang * R / (w * 1.6);
          float k = floor(s + 0.5), nr = abs(s - k);
          for (int j = 0; j < 2; j++) {
            float y = (j == 0 ? 1.0 : -1.0) * 0.62 * sin(s * PI);
            if (nr < 0.16 && int(mod(k + float(j), 2.0)) == 0) continue;
            float slope = (j == 0 ? 1.0 : -1.0) * 0.62 * PI * cos(s * PI) * 0.5 / 1.6;
            vec2 rad = normalize(SP - cen), tang = vec2(-rad.y, rad.x);
            int gg = strokeG((t - y) * w * 0.5 / sqrt(1.0 + slope * slope), normalize(rad - slope * tang));
            if (gg >= 0) { put(gg, j == 0 ? P_(P_GOLDHI) : P_(P_VERMHI), 1.05); break; }
          }
        }
      }
    }
  }
  // the creatures: hand-drawn ASCII art drawn over the map by the lettering pass, on a grid k
  // times coarser; here the map is cleared under them
  for (int i = 0; i < NASPR; i++) {
    if (i >= uASN) break;
    vec4 s = uAS[i];
    int ai = int(s.z), k = int(mod(s.w, 8.0)), fr = int(s.w / 8.0);
    vec4 art = uArt[ai];
    ivec2 ac = ivec2(floor(s.xy / uCell));
    int dx = CX - ac.x + int(uArtA[ai].x) * k, dy = CY - ac.y + int(uArtA[ai].y) * k;
    if (dx < 0 || dy < 0) continue;
    int col = dx / k, row = dy / k;
    if (col >= int(art.y) || row >= int(art.z)) continue;
    uvec4 tx = texelFetch(uArtTex, ivec2(col, int(art.x) + fr * int(art.z) + row), 0);
    if (tx.g > 0u || tx.b > 0u) put(G_SP, FG, 0.0);
  }
  // compass roses and ornaments
  for (int i = 0; i < NSPR; i++) {
    if (i >= uSprN) break;
    bool cov;
    drawSprite(i, cov);
  }
  // the map inking itself in
  if (uReveal.x < 1.0) {
    float d = length(SP - uReveal.yz) / uReveal.w;
    float edge = uReveal.x * 1.6 - d - h12(vec2(CX, CY)) * 0.15;
    if (edge < 0.0) { G = G_SP; I = 0.0; BG = P_(P_SKY); }
    else if (edge < 0.08) { put(ramp(0.9), P_(P_GOLDHI), 1.6); }
  }
  // the words sit on knocked-back ground
  for (int i = 0; i < NKNOCK; i++) {
    if (i >= uKnockN) break;
    vec4 b = uKnock[i];
    if (SP.x > b.x && SP.x < b.z && SP.y > b.y && SP.y < b.w) { I *= 0.3; break; }
  }
  oFg = vec4(FG, clamp(I * 0.5, 0.0, 1.0));
  oBg = vec4(BG, float(G) / 255.0);
}
