// Night Voyage: a night-sea palette from the 2009 animated film, carried from the whole world down to
// High Street. Deep ultramarine water worked like watercolour, pale-blue shallows edged in white chalk
// and dotted, dark teal land with scratchy pen hatching, chalk line work, a few glowing things, and one
// warm accent: the red sail of a ship whose voyage crosses the scales.
//
// How it draws. Each layer is painted into a float RGB buffer at the plate's full size: watercolour
// washes from seeded noise fields (blooms, mottle, dried tide lines, dry brush, granulation, paper),
// then pen hatching, chalk, glows and lettering drawn on a canvas and composited in. Distances from
// shores come from a Euclidean distance transform (the shallows bands, the dotted shore lines, the
// hill bands, the direction of the hatching). Everything is seeded, so a layer draws the same pixels
// every time.
//
// Units. Drawing is in units of 3 plate pixels (the plate is 546 x 1181.7 units); the screen at rest
// shows the middle 388 x 844 units.
//
// Pass plates. The exporter makes them by magnifying the page about the closer plate's centre with a
// CSS transform on the root element. A canvas would only be blown up, so the page watches for that
// transform and, in the same task, redraws the layer at the magnified scale at full resolution and
// takes the transform off: pass plates stay as sharp as the plates themselves.
'use strict';
(function () {
const W = Kit.W, H = Kit.H, S = 3, CW = W / S, CH = H / S, N = W * H;
const { level: LEVEL, layer: LAYER } = Kit.params();

// ---------------------------------------------------------------- the levels, their scales and layers
const SCALES = [W * 0.306, Kit.km(2300), Kit.km(620), Kit.km(52), Kit.km(1.3)];
const IDS = ['world', 'america', 'ohio', 'columbus', 'shortnorth'];
const lensU = k => d3.geoOrthographic().rotate([-Kit.TARGET[0], -Kit.TARGET[1]]).translate([CW / 2, CH / 2])
  .scale(SCALES[k] / S).clipAngle(90).precision(0.1);
const KELLS = [-6.88, 53.73], CBUS = [-82.99, 39.96];
// where the red sail is at each scale: [lon, lat], mast height in units, facing (1 east, -1 west)
const SHIP = [
  { at: d3.geoInterpolate(KELLS, CBUS)(0.34), h: 19, dir: -1 },
  { at: [-80.6, 42.33], h: 15, dir: -1 },
  { at: [-83.17, 40.34], h: 17, dir: -1 },
  { at: [-83.0211, 39.9852], h: 11, dir: 1 },
  null,
];
const shipFrame = k => {
  const s = SHIP[k]; if (!s) return null;
  const [x, y] = lensU(k)(s.at), h = s.h;
  return { x, y, frame: [x - h * 0.95, y - h * 1.2, h * 1.9, h * 1.5].map(v => Math.round(v * S)) };
};
const CAPTIONS = [
  { eyebrow: 'I · The World', title: 'One lit stone in the night', text: 'Far out, the Earth is a single orb among spiral stars.' },
  { eyebrow: 'II · North America', title: 'The five sweet-water seas', text: 'The Great Lakes hold the light; Ohio lies below Erie.' },
  { eyebrow: 'III · Ohio', title: 'Between the lake and the river', text: 'Erie above, the river below, Columbus between.' },
  { eyebrow: 'IV · Columbus', title: 'The city in its ring', text: 'The Outerbelt is its wall; two rivers meet.' },
  { eyebrow: 'V · Short North', title: 'Lanterns over High Street', text: 'Arches of light down the long street north.' },
];
function layersOf(k) {
  const L = [{ name: 'ground', opaque: true }];
  if (k === 0) L.push({ name: 'near', depth: 0.1 });
  if (k >= 1) L.push({ name: 'lights', depth: 0, motion: { kind: 'breathe', min: 0.72, max: 1, period: 8, phase: 0.15 * k } });
  L.push({ name: 'labels', depth: 0.04 });
  const sf = shipFrame(k);
  if (sf) L.push({ name: 'ship', depth: 0.02, frame: sf.frame,
    motion: { kind: 'sway', angle: 2.4, period: 7, phase: 0.2 * k, anchor: [Math.round(sf.x * S), Math.round(sf.y * S)] } });
  if (k === 4) L.push({ name: 'near', depth: 0.12 });
  return L;
}
window.Plates = {
  levels: 5,
  style: {
    fonts: ['fonts/UncialAntiqua-Regular.ttf', 'fonts/Alegreya-Regular.ttf', 'fonts/Alegreya-Italic.ttf'],
    eyebrow: { font: 'UncialAntiqua-Regular', size: 12, color: '#e0565e', tracking: 2.6, uppercase: true },
    title: { font: 'Alegreya-Italic', size: 27, color: '#f4ead0' },
    text: { font: 'Alegreya-Regular', size: 16.5, color: '#b9c7e8' },
    scrim: '#050718e6', accent: '#e0565e',
  },
  info(level) {
    const k = level - 1;
    return { id: IDS[k], caption: CAPTIONS[k], layers: layersOf(k), child: k < 4 ? Kit.childRect(Kit.lens(SCALES[k]), Kit.lens(SCALES[k + 1])) : null };
  },
};
if (LAYER === 'none') return;

// ---------------------------------------------------------------- magnification (pass plates)
let MS = 1, K = S, OX = 0, OY = 0;   // device px per unit, and the device position of unit (0, 0)
function setMag(s, cx, cy) { MS = s; K = S * s; OX = W / 2 - cx * s; OY = H / 2 - cy * s; }
const ux = i => ((i % W) - OX) / K, uy = i => (((i / W) | 0) - OY) / K;

// ---------------------------------------------------------------- randomness and noise
function rng(seed) {
  let a = seed >>> 0;
  return () => { a = (a + 0x6D2B79F5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
function hashI(i) { let h = Math.imul(i ^ 0x9E3779B9, 0x85EBCA6B); h ^= h >>> 13; h = Math.imul(h, 0xC2B2AE35); h ^= h >>> 16; return (h >>> 0) / 4294967296; }
const smooth = (a, b, x) => { const t = Math.min(1, Math.max(0, (x - a) / (b - a))); return t * t * (3 - 2 * t); };
const clamp01 = x => x < 0 ? 0 : x > 1 ? 1 : x;
const lerp = (a, b, t) => a + (b - a) * t;
const hex = h => [parseInt(h.slice(1, 3), 16) / 255, parseInt(h.slice(3, 5), 16) / 255, parseInt(h.slice(5, 7), 16) / 255];
const rgba = (h, a) => { const c = hex(h); return `rgba(${Math.round(c[0] * 255)},${Math.round(c[1] * 255)},${Math.round(c[2] * 255)},${a})`; };

const TBL = (() => { const r = rng(7771); const t = new Float32Array(65536); for (let i = 0; i < 65536; i++) t[i] = r(); return t; })();
function vn(x, y) {
  const ix = Math.floor(x), iy = Math.floor(y), fx = x - ix, fy = y - iy;
  const u = fx * fx * (3 - 2 * fx), v = fy * fy * (3 - 2 * fy);
  const x0 = ix & 255, x1 = (ix + 1) & 255, y0 = (iy & 255) << 8, y1 = ((iy + 1) & 255) << 8;
  const a = TBL[y0 + x0], b = TBL[y0 + x1], c = TBL[y1 + x0], d = TBL[y1 + x1];
  return a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v;
}
function fbm(x, y, oct) { let s = 0, amp = 1, tot = 0; for (let o = 0; o < oct; o++) { s += amp * vn(x + o * 31.7, y + o * 17.3); tot += amp; x *= 2.03; y *= 2.03; amp *= 0.5; } return s / tot; }
// A noise field over the plate plus a margin of EXT device px (layers sample it at different offsets),
// computed on a grid every `div` px, bilinearly upsampled and normalised to about 0..1.
const EXT = 320, FW = W + EXT, FH = H + EXT;
function field(div, cell, oct, seed, warp = 0, stretchX = 1, ext = EXT) {
  const w = W + ext, h = H + ext, gw = Math.ceil(w / div) + 2, gh = Math.ceil(h / div) + 2, g = new Float32Array(gw * gh);
  const ox = seed * 13.17, oy = seed * 7.71;
  let sum = 0, sum2 = 0;
  for (let y = 0; y < gh; y++) for (let x = 0; x < gw; x++) {
    let u = x * div / cell / stretchX + ox, v = y * div / cell + oy;
    if (warp) { u += warp * (fbm(u * 0.5 + 5.2, v * 0.5 + 1.3, 3) - 0.5); v += warp * (fbm(u * 0.5 + 9.7, v * 0.5 + 4.1, 3) - 0.5); }
    const n = fbm(u, v, oct); g[y * gw + x] = n; sum += n; sum2 += n * n;
  }
  const m = sum / g.length, sd = Math.sqrt(Math.max(1e-9, sum2 / g.length - m * m));
  for (let i = 0; i < g.length; i++) g[i] = clamp01(0.5 + (g[i] - m) / sd * 0.2);
  const f = new Float32Array(w * h);
  if (div === 1) { for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) f[y * w + x] = g[y * gw + x]; return f; }
  for (let y = 0; y < h; y++) {
    const gy = y / div, y0 = Math.floor(gy), ty = gy - y0, r0 = y0 * gw, r1 = r0 + gw;
    for (let x = 0; x < w; x++) {
      const gx = x / div, x0 = Math.floor(gx), tx = gx - x0;
      const a = g[r0 + x0], b = g[r0 + x0 + 1], c = g[r1 + x0], d = g[r1 + x0 + 1];
      f[y * w + x] = (a + (b - a) * tx) * (1 - ty) + (c + (d - c) * tx) * ty;
    }
  }
  return f;
}
let BLOOM, MOT, TIDE, BRUSH, GRAIN, PAPER;
function makeFields() {
  BLOOM = field(4, 420, 5, 1, 1.5);    // big soft blooms: sampled in plate units, so they zoom with the map
  MOT = field(2, 64, 4, 2, 0.9);       // mottling
  TIDE = field(4, 240, 4, 3, 1.3);     // where washes dried: thin darker tide lines
  BRUSH = field(2, 22, 3, 6, 0.6, 5);  // dry-brush streaks, stretched sideways
  GRAIN = field(1, 2.1, 2, 4, 0, 1, 0);     // granulation, chalk breakage
  PAPER = field(1, 4.5, 3, 5, 0, 4, 0);     // paper fibres
}
const fi = i => ((i / W) | 0) * FW + (i % W);   // a device index into a field with margin
const bloomAt = i => { const bx = Math.min(FW - 1, Math.max(0, Math.round(ux(i) * S))), by = Math.min(FH - 1, Math.max(0, Math.round(uy(i) * S))); return BLOOM[by * FW + bx]; };

// ---------------------------------------------------------------- canvases and the paint buffer
function mk(w = W, h = H) { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; }
const OUT = mk(); document.body.appendChild(OUT);
const out = OUT.getContext('2d');
const SCR = mk(), sctx = SCR.getContext('2d', { willReadFrequently: true });
function scratch() {
  sctx.setTransform(1, 0, 0, 1, 0, 0); sctx.globalCompositeOperation = 'source-over'; sctx.globalAlpha = 1;
  sctx.filter = 'none'; sctx.shadowBlur = 0; sctx.shadowColor = 'transparent'; sctx.setLineDash([]);
  sctx.clearRect(0, 0, W, H); sctx.setTransform(K, 0, 0, K, OX, OY); return sctx;
}
function alphaOf(c) { const d = c.getContext('2d').getImageData(0, 0, W, H).data; const a = new Float32Array(N); for (let i = 0; i < N; i++) a[i] = d[i * 4 + 3] / 255; return a; }
function maskFrom(draw) { const c = scratch(); c.fillStyle = '#fff'; c.strokeStyle = '#fff'; draw(c); return alphaOf(SCR); }
// blur radius in units, as a canvas filter (device px)
const blurU = r => `blur(${Math.max(0.5, r * K)}px)`;

let IMG = null;   // RGB, 0..1; a transparent layer also keeps ALPHA
let ALPHA = null;
function begin(opaque) { IMG = new Float32Array(N * 3); ALPHA = opaque ? null : new Float32Array(N); }
// Composite the scratch canvas into the buffer; fx(i) scales the alpha per pixel.
function composite(mode = 'normal', op = 1, fx = null) {
  const d = sctx.getImageData(0, 0, W, H).data;
  for (let i = 0, p = 0, k = 0; i < N; i++, p += 4, k += 3) {
    const a0 = d[p + 3]; if (a0 === 0) continue;
    let a = a0 / 255 * op; if (fx) { a *= fx(i); if (a <= 0) continue; }
    const r = d[p] / 255, g = d[p + 1] / 255, b = d[p + 2] / 255;
    const R = IMG[k], G = IMG[k + 1], B = IMG[k + 2];
    let tr = r, tg = g, tb = b;
    if (mode === 'screen') { tr = 1 - (1 - R) * (1 - r); tg = 1 - (1 - G) * (1 - g); tb = 1 - (1 - B) * (1 - b); }
    if (ALPHA) {
      // a transparent layer: plain "over" with alpha (screen behaves as over here)
      const A = ALPHA[i], oa = a + A * (1 - a);
      if (oa > 0) { IMG[k] = (r * a + R * A * (1 - a)) / oa; IMG[k + 1] = (g * a + G * A * (1 - a)) / oa; IMG[k + 2] = (b * a + B * A * (1 - a)) / oa; }
      ALPHA[i] = oa; continue;
    }
    IMG[k] = R + (tr - R) * a; IMG[k + 1] = G + (tg - G) * a; IMG[k + 2] = B + (tb - B) * a;
  }
}
const chalky = (lo = 0.28, hi = 0.62) => i => smooth(lo, hi, GRAIN[i] * 0.75 + MOT[fi(i)] * 0.25 + 0.06);
const within = m => i => m[i];
// A watercolour wash into the buffer, through a mask (0..1 per pixel).
function wash(mask, P) {
  const [c0, c1, c2] = P.c.map(hex), off = P.off || 0, dx = (off * 37) % EXT, dy = (off * 101) % EXT;
  const bloom = P.bloom ?? 1, mottle = P.mottle ?? 0.5, tide = P.tide ?? 0.12, grain = P.grain ?? 0.14, specks = P.specks ?? 0.003, brush = P.brush ?? 0;
  for (let y = 0, i = 0; y < H; y++) {
    const row = (y + dy) * FW + dx;
    for (let x = 0; x < W; x++, i++) {
      const a = mask ? mask[i] : 1; if (a <= 0.003) continue;
      const j = row + x;
      let v = 0.5 + (bloomAt(i) - 0.5) * bloom * 1.6 + (MOT[j] - 0.5) * mottle * 1.4 + (BRUSH[j] - 0.5) * brush * 1.4;
      if (P.lift) v += P.lift(i);
      v = clamp01(v);
      let r, g, b;
      if (v < 0.5) { const t = v * 2; r = c0[0] + (c1[0] - c0[0]) * t; g = c0[1] + (c1[1] - c0[1]) * t; b = c0[2] + (c1[2] - c0[2]) * t; }
      else { const t = v * 2 - 1; r = c1[0] + (c2[0] - c1[0]) * t; g = c1[1] + (c2[1] - c1[1]) * t; b = c1[2] + (c2[2] - c1[2]) * t; }
      let k = 1;
      if (tide) { const t = Math.abs(TIDE[j] - 0.5); k *= 1 - (1 - smooth(0.003, 0.016, t)) * tide; }
      k *= 1 + (GRAIN[(i + off * 7919) % N] - 0.5) * grain * 2;
      if (specks && hashI(i + off * 131) > 1 - specks) k *= 0.62;
      if (P.edge) k *= 1 - (1 - smooth(0, P.edgeW * K, P.edge[i])) * P.edgeK;
      r *= k; g *= k; b *= k;
      const q = i * 3;
      IMG[q] += (r - IMG[q]) * a; IMG[q + 1] += (g - IMG[q + 1]) * a; IMG[q + 2] += (b - IMG[q + 2]) * a;
    }
  }
}
// Euclidean distance transform (Felzenszwalb and Huttenlocher): device px to the nearest feature pixel.
function edt(feature) {
  const INF = 1e20, D = new Float32Array(N), n = Math.max(W, H);
  const f = new Float64Array(n), d = new Float64Array(n), v = new Int32Array(n), z = new Float64Array(n + 1);
  function dt(len) {
    let k = 0; v[0] = 0; z[0] = -INF; z[1] = INF;
    for (let q = 1; q < len; q++) {
      let s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k]);
      while (s <= z[k]) { k--; s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k]); }
      k++; v[k] = q; z[k] = s; z[k + 1] = INF;
    }
    k = 0;
    for (let q = 0; q < len; q++) { while (z[k + 1] < q) k++; d[q] = (q - v[k]) * (q - v[k]) + f[v[k]]; }
  }
  for (let x = 0; x < W; x++) { for (let y = 0; y < H; y++) f[y] = feature(y * W + x) ? 0 : INF; dt(H); for (let y = 0; y < H; y++) D[y * W + x] = d[y]; }
  for (let y = 0; y < H; y++) { for (let x = 0; x < W; x++) f[x] = D[y * W + x]; dt(W); for (let x = 0; x < W; x++) D[y * W + x] = Math.sqrt(d[x]); }
  return D;
}
// Level lines of a device-px field at `value` units, as polylines in units (sampled every 3 device px).
function isolines(F, value) {
  const G = 3, gw = Math.floor(W / G), gh = Math.floor(H / G), g = new Float64Array(gw * gh);
  for (let y = 0; y < gh; y++) for (let x = 0; x < gw; x++) g[y * gw + x] = F[(y * G + 1) * W + x * G + 1];
  const mp = d3.contours().size([gw, gh]).thresholds([value * K])(g)[0];
  const rings = [];
  for (const poly of mp.coordinates) for (const ring of poly) rings.push(ring.map(([x, y]) => [(x * G - OX) / K, (y * G - OY) / K]));
  return rings;
}
function along(pts, step, phase = 0) {
  const o = []; let carry = phase;
  for (let i = 1; i < pts.length; i++) {
    const [x0, y0] = pts[i - 1], [x1, y1] = pts[i], L = Math.hypot(x1 - x0, y1 - y0);
    if (!L) continue;
    let t = carry;
    while (t <= L) { o.push([x0 + (x1 - x0) * t / L, y0 + (y1 - y0) * t / L, Math.atan2(y1 - y0, x1 - x0)]); t += step; }
    carry = t - L;
  }
  return o;
}
function polyLength(pts) { let L = 0; for (let i = 1; i < pts.length; i++) L += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); return L; }
// values of a device-px array, or a field with margin, at a point in units
const at = (A, x, y) => A[Math.min(H - 1, Math.max(0, (y * K + OY) | 0)) * W + Math.min(W - 1, Math.max(0, (x * K + OX) | 0))];
const FA = (F, x, y) => F[Math.min(FH - 1, Math.max(0, (y * K + OY) | 0)) * FW + Math.min(FW - 1, Math.max(0, (x * K + OX) | 0))];
// the direction along the level lines of a distance field at a point: strokes run with the edge
function tangent(Dist, x, y) {
  const e = 2.5;
  const gx = at(Dist, x + e, y) - at(Dist, x - e, y), gy = at(Dist, x, y + e) - at(Dist, x, y - e);
  return Math.atan2(gy, gx) + Math.PI / 2;
}
// the projected points of a line feature, as polylines in units
function linePts(gpath, f) {
  const pts = [];
  gpath({ moveTo: (x, y) => { pts.push(null); pts.push([x, y]); }, lineTo: (x, y) => pts.push([x, y]), closePath() {}, beginPath() {}, arc() {} })(f);
  const segs = []; let cur = [];
  for (const q of pts) { if (!q) { if (cur.length > 1) segs.push(cur); cur = []; } else cur.push(q); }
  if (cur.length > 1) segs.push(cur);
  return segs;
}
// is a unit point on the plate (with a margin)? loops skip work far off a magnified pass plate
const onPlate = (x, y, m = 20) => { const X = x * K + OX, Y = y * K + OY, M = m * K; return X > -M && X < W + M && Y > -M && Y < H + M; };

// ---------------------------------------------------------------- the night palette
const PAL = {
  sky: ['#060a2a', '#101a55', '#1f2d78'],
  sea: ['#111c5e', '#1d3089', '#3350b2'],
  land: ['#091821', '#12303a', '#22494e'],
  landDim: ['#08131a', '#0f2026', '#1b3132'],
  ice: ['#7d93c2', '#a9bde4', '#d4e0f4'],
  pale: '#a9c3ee', paleMid: '#7c9de0', paleDeep: '#5878c4', white: '#f1f5ff',
  ink: '#04080d', chalk: '#e9eef9', cream: '#f6ecca', sail: '#c8283a', hull: '#3a1422',
};

// ---------------------------------------------------------------- strokes and motifs
// A pen stroke: a thin lens from p0 to p1, bowed a little, tapering at both ends.
function penStroke(c, x0, y0, x1, y1, w, bow) {
  const mx = (x0 + x1) / 2, my = (y0 + y1) / 2, dx = x1 - x0, dy = y1 - y0, L = Math.hypot(dx, dy) || 1;
  const nx = -dy / L, ny = dx / L, cx = mx + nx * bow, cy = my + ny * bow;
  c.moveTo(x0, y0);
  c.quadraticCurveTo(cx + nx * w, cy + ny * w, x1, y1);
  c.quadraticCurveTo(cx - nx * w, cy - ny * w, x0, y0);
}
// Hatching: patches of short parallel strokes on a jittered grid, kept where inside(x, y) holds.
function hatch(c, o) {
  const r = rng(o.seed || 1), cell = o.cell || 9;
  for (let gy = -cell; gy < CH + cell; gy += cell) for (let gx = -cell; gx < CW + cell; gx += cell) {
    if (r() > (o.density ?? 0.5)) continue;
    const x = gx + r() * cell, y = gy + r() * cell;
    const rest = [r(), r(), r(), r(), r(), r()];
    if (!onPlate(x, y)) { for (let k = 0; k < 40; k++) r(); continue; }
    const dens = o.where ? o.where(x, y) : 1; if (dens <= 0 || rest[0] > dens) { for (let k = 0; k < 40; k++) r(); continue; }
    const th = o.angle(x, y) + (rest[1] - 0.5) * (o.jitter ?? 0.25);
    const n = (o.nmin ?? 3) + Math.floor(rest[2] * ((o.nmax ?? 6) - (o.nmin ?? 3) + 1));
    const len = (o.len ?? 7) * (0.7 + rest[3] * 0.6), sp = (o.space ?? 1.4) * (0.85 + rest[4] * 0.3);
    const ca = Math.cos(th), sa = Math.sin(th);
    c.beginPath();
    for (let k = 0; k < 8; k++) {
      const q = [r(), r(), r(), r(), r()];
      if (k >= n) continue;
      const off = (k - (n - 1) / 2) * sp, l2 = len * (0.65 + q[0] * 0.5) * (1 - Math.abs(k - (n - 1) / 2) / n * 0.6);
      const px = x - sa * off + ca * (q[1] - 0.5) * 1.2, py = y + ca * off + sa * (q[2] - 0.5) * 1.2;
      const x0 = px - ca * l2 / 2, y0 = py - sa * l2 / 2, x1 = px + ca * l2 / 2, y1 = py + sa * l2 / 2;
      if (!o.inside(x0, y0) || !o.inside(x1, y1)) continue;
      penStroke(c, x0, y0, x1, y1, (o.w ?? 0.28) * (0.7 + q[3] * 0.6), (q[4] - 0.5) * len * 0.12);
    }
    c.fillStyle = o.color; c.globalAlpha = (o.alpha ?? 0.5) * (0.6 + rest[5] * 0.4); c.fill();
  }
  c.globalAlpha = 1;
}
function stipple(c, n, seed, where, color, rmin, rmax) {
  const r = rng(seed); c.fillStyle = color;
  for (let k = 0; k < n; k++) {
    const x = r() * CW, y = r() * CH, q = r(), rr = r();
    if (!onPlate(x, y)) continue;
    const p = where(x, y); if (p <= 0 || q > p) continue;
    c.beginPath(); c.arc(x, y, rmin + rr * (rmax - rmin), 0, Math.PI * 2); c.fill();
  }
}
// A spiral star: an open curl of about one and a half turns.
function spiralStar(c, x, y, r, rot, lw, alpha, turns = 1.5) {
  c.beginPath();
  const steps = Math.max(18, Math.round(r * 10));
  for (let s = 0; s <= steps; s++) {
    const t = s / steps, th = rot + t * turns * Math.PI * 2, rr = r * (1 - t * 0.86);
    const px = x + Math.cos(th) * rr, py = y + Math.sin(th) * rr;
    s ? c.lineTo(px, py) : c.moveTo(px, py);
  }
  c.lineWidth = lw; c.lineCap = 'round'; c.lineJoin = 'round'; c.strokeStyle = `rgba(241,245,255,${alpha})`; c.stroke();
}
function glow(c, x, y, r, color, a) {
  const g = c.createRadialGradient(x, y, 0, x, y, r);
  g.addColorStop(0, rgba(color, a)); g.addColorStop(0.35, rgba(color, a * 0.45)); g.addColorStop(1, rgba(color, 0));
  c.fillStyle = g; c.beginPath(); c.arc(x, y, r, 0, Math.PI * 2); c.fill();
}
// The red-sailed ship of the night sea, sitting on y; h is the mast's height.
function ship(c, x, y, h, dir = 1) {
  c.save(); c.translate(x, y); c.scale(dir, 1);
  const hw = h * 0.62;
  c.beginPath();
  c.moveTo(-hw, -h * 0.30);
  c.quadraticCurveTo(-hw * 0.82, h * 0.02, -hw * 0.45, h * 0.05);
  c.lineTo(hw * 0.5, h * 0.05);
  c.quadraticCurveTo(hw * 0.86, h * 0.02, hw * 1.02, -h * 0.34);
  c.quadraticCurveTo(hw * 0.75, -h * 0.12, hw * 0.4, -h * 0.1);
  c.lineTo(-hw * 0.45, -h * 0.1);
  c.quadraticCurveTo(-hw * 0.75, -h * 0.1, -hw, -h * 0.30);
  c.fillStyle = PAL.hull; c.fill();
  c.strokeStyle = 'rgba(241,245,255,0.55)'; c.lineWidth = Math.max(0.35, h * 0.02); c.stroke();
  c.fillStyle = 'rgba(236,90,99,0.9)';
  for (let k = -3; k <= 3; k++) { c.beginPath(); c.arc(k * hw * 0.17, -h * 0.03, Math.max(0.3, h * 0.022), 0, Math.PI * 2); c.fill(); }
  c.strokeStyle = '#120a10'; c.lineWidth = Math.max(0.4, h * 0.035);
  c.beginPath(); c.moveTo(0, -h * 0.08); c.lineTo(0, -h * 1.02); c.stroke();
  const sx0 = -h * 0.36, sx1 = h * 0.4, sy0 = -h * 0.95, sy1 = -h * 0.24;
  c.beginPath(); c.moveTo(sx0, sy0); c.lineTo(sx1, sy0 + h * 0.02); c.lineTo(sx1 + h * 0.03, sy1); c.lineTo(sx0 - h * 0.03, sy1 - h * 0.01); c.closePath();
  const sg = c.createLinearGradient(0, sy0, 0, sy1);
  sg.addColorStop(0, '#5a1424'); sg.addColorStop(0.28, PAL.sail); sg.addColorStop(1, '#d83a48');
  c.fillStyle = sg; c.fill();
  c.save(); c.clip();
  const r = rng(91);
  for (let k = 0; k < 26; k++) {
    const xx = sx0 + (sx1 - sx0) * r(), yy = sy0 + (sy1 - sy0) * (0.15 + r() * 0.6);
    c.fillStyle = r() < 0.5 ? 'rgba(255,170,170,0.35)' : 'rgba(70,10,25,0.35)';
    c.fillRect(xx, yy, Math.max(0.3, h * 0.02), h * (0.08 + r() * 0.22));
  }
  c.restore();
  // a little white spray at the bow
  c.fillStyle = 'rgba(241,245,255,0.8)';
  for (let k = 0; k < 7; k++) { c.beginPath(); c.arc(hw * (1.05 + 0.12 * Math.cos(k)), -h * (0.02 + 0.05 * k / 7), Math.max(0.25, h * 0.018), 0, Math.PI * 2); c.fill(); }
  c.restore();
}
// Lettering in insular capitals, with a soft dark halo so it sits cleanly on any ground.
function label(c, text, x, y, size, o = {}) {
  c.save();
  c.font = `${size}px 'Uncial Antiqua'`; c.textAlign = o.align || 'center'; c.textBaseline = 'middle';
  c.letterSpacing = `${(o.track ?? 0.16) * size}px`;
  c.translate(x, y); if (o.rotate) c.rotate(o.rotate);
  c.shadowColor = o.halo || 'rgba(4,7,26,0.9)'; c.shadowBlur = size * 0.6 * K;
  c.fillStyle = o.color || PAL.chalk; c.globalAlpha = o.alpha ?? 1;
  c.fillText(text, 0, 0); c.shadowBlur = size * 0.25 * K; c.fillText(text, 0, 0);
  c.shadowBlur = 0; c.fillText(text, 0, 0);
  c.restore();
}
// Lettering along a path (unit points), centred at fraction `at` of its length.
function pathLabel(c, text, pts, size, o = {}) {
  c.save();
  c.font = `${size}px 'Uncial Antiqua'`; c.textBaseline = 'middle'; c.textAlign = 'center';
  const track = (o.track ?? 0.2) * size;
  const widths = [...text].map(ch => c.measureText(ch).width + track);
  const total = widths.reduce((a, b) => a + b, 0) - track;
  const L = polyLength(pts); let s = L * (o.at ?? 0.5) - total / 2;
  const pos = d => { let acc = 0; for (let i = 1; i < pts.length; i++) { const l = Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); if (acc + l >= d) { const t = (d - acc) / l; return [lerp(pts[i - 1][0], pts[i][0], t), lerp(pts[i - 1][1], pts[i][1], t)]; } acc += l; } const n = pts.length - 1; return [pts[n][0], pts[n][1]]; };
  [...text].forEach((ch, i) => {
    const mid = s + (widths[i] - track) / 2, [px, py] = pos(mid), a0 = pos(mid - 2), a1 = pos(mid + 2);
    c.save(); c.translate(px, py); c.rotate(Math.atan2(a1[1] - a0[1], a1[0] - a0[0])); c.translate(0, o.dy || 0);
    c.shadowColor = o.halo || 'rgba(4,7,26,0.9)'; c.shadowBlur = size * 0.6 * K; c.fillStyle = o.color || PAL.chalk; c.globalAlpha = o.alpha ?? 1;
    c.fillText(ch, 0, 0); c.shadowBlur = 0; c.fillText(ch, 0, 0); c.restore();
    s += widths[i];
  });
  c.restore();
}
// White spatter, thicker where the blooms are lighter.
function spatter(c, n, seed, ok, o = {}) {
  const r = rng(seed);
  for (let k = 0; k < n; k++) {
    const x = r() * CW, y = r() * CH, q = [r(), r(), r(), r(), r()];
    if (!onPlate(x, y) || !ok(x, y)) continue;
    const b = BLOOM[Math.min(FH - 1, Math.max(0, Math.round(y * S))) * FW + Math.min(FW - 1, Math.max(0, Math.round(x * S)))];
    if (q[0] > 0.25 + b * (o.cluster ?? 1.2)) continue;
    const big = q[1] < (o.big ?? 0.04), rad = big ? 0.5 + q[2] * 0.7 : 0.18 + q[2] * 0.32;
    c.fillStyle = `rgba(236,242,255,${(big ? 0.55 : 0.35) + q[3] * 0.45})`;
    c.beginPath(); c.arc(x, y, rad, 0, Math.PI * 2); c.fill();
  }
}
function stars(c, n, seed, ok, o = {}) {
  const r = rng(seed), got = [];
  for (let k = 0; k < n * 60 && got.length < n; k++) {
    const x = (o.x0 ?? 10) + r() * ((o.x1 ?? CW - 10) - (o.x0 ?? 10)), y = (o.y0 ?? 10) + r() * ((o.y1 ?? CH - 10) - (o.y0 ?? 10)), q = [r(), r(), r(), r()];
    if (!ok(x, y)) continue;
    if (got.some(([gx, gy]) => Math.hypot(gx - x, gy - y) < (o.apart ?? 34))) continue;
    got.push([x, y]);
    const big = q[0] < (o.bigShare ?? 0.2), rad = (big ? 3.0 + q[1] * 1.3 : 1.6 + q[1] * 1.0) * (o.scale ?? 1);
    if (big) glow(c, x, y, rad * 4.5, '#c9d8ff', 0.18);
    spiralStar(c, x, y, rad, q[2] * Math.PI * 2, big ? 0.7 : 0.55, big ? 0.95 : 0.82);
  }
}
function light(c, x, y, r, a = 0.55) { glow(c, x, y, r, PAL.cream, a); c.fillStyle = '#fff7de'; c.beginPath(); c.arc(x, y, Math.max(0.6, r * 0.12), 0, Math.PI * 2); c.fill(); }
function route(c, pts, step = 3, r = 0.45, a = 0.9) { c.fillStyle = `rgba(246,236,202,${a})`; for (const [x, y] of along(pts, step, 0)) { c.beginPath(); c.arc(x, y, r, 0, Math.PI * 2); c.fill(); } }
function iris(c, x, y, r, a = 0.9) { c.save(); c.setLineDash([0.1, 2.5]); c.lineCap = 'round'; c.lineWidth = 0.95; c.strokeStyle = `rgba(233,238,249,${a})`; c.beginPath(); c.arc(x, y, r, 0, Math.PI * 2); c.stroke(); c.restore(); }
// A triple spiral as cut at Newgrange, in chalk lines and dots, in a dotted ring.
function triskele(c, x, y, R, rot, o = {}) {
  const lw = o.lw ?? 0.7, a = o.alpha ?? 0.85, hand = o.hand ?? 1;
  c.save(); c.translate(x, y); c.lineCap = 'round'; c.lineJoin = 'round';
  for (let k = 0; k < 3; k++) {
    const th = rot + k * Math.PI * 2 / 3, cx = 0.5 * R * Math.cos(th), cy = 0.5 * R * Math.sin(th);
    const arm = (inset, t0, t1) => {
      const pts = [];
      for (let s = 0; s <= 90; s++) {
        const t = t0 + (t1 - t0) * s / 90, phi = th + Math.PI + hand * t * Math.PI * 3.1;
        const r = 0.5 * R * (1 - 0.86 * t) - inset * (t > 0.02 ? 1 : t / 0.02);
        pts.push([cx + Math.cos(phi) * r, cy + Math.sin(phi) * r]);
      }
      return pts;
    };
    c.beginPath(); arm(0, 0, 1).forEach((p, i) => i ? c.lineTo(p[0], p[1]) : c.moveTo(p[0], p[1]));
    c.strokeStyle = `rgba(236,242,255,${a})`; c.lineWidth = lw; c.stroke();
    c.fillStyle = `rgba(236,242,255,${a * 0.9})`;
    for (const [px, py] of along(arm(R * 0.11, 0.06, 0.62), o.dot ?? 2.3, 0.4)) { c.beginPath(); c.arc(px, py, lw * 0.55, 0, Math.PI * 2); c.fill(); }
  }
  c.setLineDash([0.1, 2.6]); c.lineWidth = lw * 1.2; c.strokeStyle = `rgba(236,242,255,${a * 0.75})`;
  c.beginPath(); c.arc(0, 0, R * 0.97, 0, Math.PI * 2); c.stroke(); c.setLineDash([]);
  c.restore();
}
// Sweeping bands, as the film draws hills: offset curves along a spine (unit points), wider in the
// middle and faded out at both ends, every other band washed a little lighter.
function bands(spine, o) {
  const pts = [], line = d3.line().curve(d3.curveCatmullRom.alpha(0.5));
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'path'); svg.setAttribute('d', line(spine));
  const L = svg.getTotalLength(), n = Math.max(8, Math.round(L / 2));
  for (let k = 0; k <= n; k++) { const p = svg.getPointAtLength(L * k / n); pts.push([p.x, p.y]); }
  const nor = pts.map((p, k) => { const a = pts[Math.max(0, k - 1)], b = pts[Math.min(n, k + 1)], dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy) || 1; return [-dy / l, dx / l]; });
  const env = k => { const t = k / n; return 0.7 + 0.3 * smooth(0, o.taper ?? 0.22, t) * smooth(1, 1 - (o.taper ?? 0.22), t); };
  const off = (j, k) => { const w = (o.count / 2) * o.space * env(k), d = (j / o.count - 0.5) * 2 * w; const wob = (vn(k * 0.05, j * 3.1) - 0.5) * o.space * 0.5; return [pts[k][0] + nor[k][0] * (d + wob), pts[k][1] + nor[k][1] * (d + wob)]; };
  const curves = []; for (let j = 0; j <= o.count; j++) { const cv = []; for (let k = 0; k <= n; k++) cv.push(off(j, k)); curves.push(cv); }
  const e0 = pts[0], e1 = pts[n], fl = (o.fadeEnds ?? 0.16) * L;
  const endFade = i => { const x = ux(i), y = uy(i); return smooth(0, fl, Math.hypot(x - e0[0], y - e0[1])) * smooth(0, fl, Math.hypot(x - e1[0], y - e1[1])); };
  let c = scratch();
  for (let j = 0; j < o.count; j += 2) {
    c.beginPath(); curves[j].forEach((q, k) => k ? c.lineTo(q[0], q[1]) : c.moveTo(q[0], q[1]));
    for (let k = n; k >= 0; k--) c.lineTo(curves[j + 1][k][0], curves[j + 1][k][1]);
    c.closePath(); c.fillStyle = rgba(o.fill, o.fillAlpha ?? 0.16); c.fill();
  }
  composite('normal', 1, i => (o.mask ? o.mask[i] : 1) * endFade(i));
  c = scratch();
  c.lineCap = 'round'; c.lineJoin = 'round'; c.lineWidth = o.lw ?? 0.45; c.strokeStyle = `rgba(228,236,250,${o.lineAlpha ?? 0.45})`;
  for (const cv of curves) { c.beginPath(); cv.forEach((q, k) => k ? c.lineTo(q[0], q[1]) : c.moveTo(q[0], q[1])); c.stroke(); }
  composite('normal', 1, i => (o.mask ? o.mask[i] : 1) * chalky(0.15, 0.5)(i) * endFade(i));
}
// A round tower standing upright in the plan, as the film stands the abbey's tower in its courtyard.
function tower(c, x, y, h, o = {}) {
  const w = h * 0.16, capH = h * 0.24;
  c.save(); c.translate(x, y); c.lineJoin = 'round';
  c.fillStyle = 'rgba(2,4,14,0.35)'; c.beginPath(); c.ellipse(w * 0.9, 0, w * 1.6, w * 0.45, 0, 0, Math.PI * 2); c.fill();
  c.beginPath(); c.moveTo(-w / 2, 0); c.lineTo(-w * 0.42, -h + capH); c.lineTo(0, -h); c.lineTo(w * 0.42, -h + capH); c.lineTo(w / 2, 0); c.closePath();
  const g = c.createLinearGradient(-w / 2, 0, w / 2, 0); g.addColorStop(0, '#4a5f86'); g.addColorStop(0.55, '#26365a'); g.addColorStop(1, '#141d36');
  c.fillStyle = g; c.fill();
  c.strokeStyle = `rgba(236,242,255,${o.edge ?? 0.85})`; c.lineWidth = Math.max(0.4, h * 0.03); c.stroke();
  c.beginPath(); c.moveTo(-w * 0.42, -h + capH); c.lineTo(w * 0.42, -h + capH); c.stroke();
  c.fillStyle = '#ffe9b0'; c.fillRect(-w * 0.12, -h + capH + h * 0.08, w * 0.24, h * 0.07);
  if (h > 20) { c.fillRect(-w * 0.1, -h * 0.42, w * 0.2, h * 0.05); c.fillRect(-w * 0.1, -h * 0.2, w * 0.2, h * 0.05); }
  c.restore();
}
function walledPlace(c, x, y, r) {
  c.save(); c.lineCap = 'round';
  c.strokeStyle = 'rgba(238,243,255,0.9)'; c.lineWidth = 0.6;
  c.beginPath(); c.arc(x, y, r, 0, Math.PI * 2); c.stroke();
  c.lineWidth = 0.4; c.beginPath(); c.arc(x, y, r + 2.1, 0, Math.PI * 2); c.stroke();
  c.setLineDash([0.1, 1.9]); c.lineWidth = 0.75; c.beginPath(); c.arc(x, y, r + 1.05, 0, Math.PI * 2); c.stroke();
  c.restore();
}
// Round crowns of trees: a dark disc, a moonlit rim on the upper left, a shadow, a curl inside.
function crown(c, x, y, r, o = {}) {
  c.fillStyle = `rgba(2,5,12,${o.shadow ?? 0.45})`; c.beginPath(); c.ellipse(x + r * 0.35, y + r * 0.4, r * 1.02, r * 0.92, 0, 0, Math.PI * 2); c.fill();
  const g = c.createRadialGradient(x - r * 0.35, y - r * 0.4, r * 0.1, x, y, r);
  g.addColorStop(0, o.lit || '#2f5a50'); g.addColorStop(1, o.dark || '#0c1f22');
  c.fillStyle = g; c.beginPath(); c.arc(x, y, r, 0, Math.PI * 2); c.fill();
  c.strokeStyle = `rgba(226,238,246,${o.rim ?? 0.6})`; c.lineWidth = Math.max(0.25, r * 0.12); c.lineCap = 'round';
  c.beginPath(); c.arc(x, y, r * 0.92, Math.PI * 0.95, Math.PI * 1.65); c.stroke();
  if (r > 2.6 && o.curl !== false) spiralStar(c, x + r * 0.05, y + r * 0.05, r * 0.55, o.rot ?? 0, Math.max(0.22, r * 0.07), 0.26, 1.2);
}
// Shallows: a pale band along every shore on the water side, its outer edge wavering and touched with chalk.
function shallows(waterM, dWater, o = {}) {
  const pale = hex(o.color || PAL.paleMid), white = hex(PAL.white), ch = chalky(0.2, 0.6), w0 = o.w ?? 1.9, a0 = o.alpha ?? 0.6, e0 = o.edge ?? 0.5;
  for (let i = 0; i < N; i++) {
    if (waterM[i] <= 0 || dWater[i] > (w0 * 3 + 4) * K) continue;
    const d = dWater[i] / K, wob = w0 + (MOT[fi(i)] - 0.5) * w0 * 0.95;
    const band = (1 - smooth(wob * 0.5, wob, d)) * a0;
    const edgeL = (1 - smooth(0.0, 0.42, Math.abs(d - wob - 0.2))) * e0 * ch(i);
    const q = i * 3, a = waterM[i];
    for (let k = 0; k < 3; k++) { IMG[q + k] += (pale[k] - IMG[q + k]) * band * a; IMG[q + k] += (white[k] - IMG[q + k]) * edgeL * a; }
  }
}
// Dotted chalk out on the water at a distance from the shore, and fainter ripple lines beyond.
function shoreLines(waterM, dWater, o = {}) {
  const c = scratch();
  c.fillStyle = `rgba(241,245,255,${o.dotAlpha ?? 0.85})`;
  for (const ring of isolines(dWater, o.dots ?? 4.4)) {
    if (polyLength(ring) < (o.minLen ?? 20)) continue;
    for (const [x, y] of along(ring, o.step ?? 2.7, 1)) { c.beginPath(); c.arc(x, y, o.r ?? 0.38, 0, Math.PI * 2); c.fill(); }
  }
  for (const [dist, al] of (o.ripples ?? [[8.5, 0.32]])) {
    c.strokeStyle = rgba(PAL.pale, al); c.lineWidth = 0.45;
    for (const ring of isolines(dWater, dist)) {
      if (polyLength(ring) < (o.minLen ?? 20) * 2) continue;
      c.beginPath(); ring.forEach((p, k) => k ? c.lineTo(p[0], p[1]) : c.moveTo(p[0], p[1])); c.stroke();
    }
  }
  composite('normal', 1, within(waterM));
}
// Paper grain and a gentle darkening towards the plate's outer ring (in units of the unmagnified plate).
function paper(vig = 0.45) {
  for (let i = 0; i < N; i++) {
    const dx = (ux(i) - CW / 2) / (CW * 0.71), dy = (uy(i) - CH / 2) / (CH * 0.71);
    const v = 1 - vig * smooth(0.25, 0.85, Math.sqrt(dx * dx * 1.1 + dy * dy));
    const p = (0.96 + PAPER[i] * 0.06 + (GRAIN[i] - 0.5) * 0.04) * v;
    const q = i * 3; IMG[q] *= p; IMG[q + 1] *= p; IMG[q + 2] *= p;
  }
}
function present() {
  const id = out.createImageData(W, H), d = id.data;
  for (let i = 0, p = 0, q = 0; i < N; i++, p += 4, q += 3) {
    d[p] = Math.round(clamp01(IMG[q]) * 255); d[p + 1] = Math.round(clamp01(IMG[q + 1]) * 255); d[p + 2] = Math.round(clamp01(IMG[q + 2]) * 255);
    d[p + 3] = ALPHA ? Math.round(clamp01(ALPHA[i]) * 255) : 255;
  }
  out.putImageData(id, 0, 0);
}
// A transparent layer drawn straight onto the output canvas (labels, the ship, glows).
function vectorLayer(draw) {
  out.setTransform(1, 0, 0, 1, 0, 0); out.clearRect(0, 0, W, H); out.setTransform(K, 0, 0, K, OX, OY);
  draw(out);
  out.setTransform(1, 0, 0, 1, 0, 0);
}

// ---------------------------------------------------------------- data
const D = name => Kit.json(`../data/${name}`);
const fc = features => ({ type: 'FeatureCollection', features });

// ================================================================ I · the world
async function loadWorld() {
  const [landT, countriesT, lakesFC, riversFC] = await Promise.all([D('land-50m.json'), D('countries-50m.json'), D('america-lakes.geojson'), D('america-rivers.geojson')]);
  const land0 = topojson.feature(landT, landT.objects.land), polys = [];
  for (const f of land0.features) for (const pc of (f.geometry.type === 'Polygon' ? [f.geometry.coordinates] : f.geometry.coordinates)) {
    const g = { type: 'Polygon', coordinates: pc }; if (d3.geoArea(g) > 0.0004) polys.push({ type: 'Feature', geometry: g });
  }
  const countries = topojson.feature(countriesT, countriesT.objects.countries);
  return { land: fc(polys), ice: fc(countries.features.filter(f => f.id === '304')),
    lakes: fc(lakesFC.features.filter(f => d3.geoArea(f) > 0.00018)),
    rivers: fc(riversFC.features.filter(f => ['Mississippi', 'Missouri', 'Ohio', 'Saint Lawrence', 'Mackenzie', 'Yukon', 'Rio Grande', 'Arkansas', 'Nelson', 'Saskatchewan', 'Columbia', 'Colorado'].includes(f.properties.name))) };
}
function drawWorld(layer, d) {
  const proj = lensU(0), gp = c => d3.geoPath(proj, c), OXu = CW / 2, OYu = CH / 2, OR = proj.scale();
  const offOrb = (x, y, m) => Math.hypot(x - OXu, y - OYu) > OR + m;
  if (layer === 'near') {
    begin(false);
    let c = scratch(); spatter(c, 3200, 23, (x, y) => offOrb(x, y, 24), { big: 0.05 }); composite();
    c = scratch(); stars(c, 26, 33, (x, y) => offOrb(x, y, 40), { apart: 46, bigShare: 0.24 }); composite();
    return present();
  }
  if (layer === 'labels') {
    // the voyage and the next frame, in chalk floating just above the orb
    return vectorLayer(c => {
      const ip = d3.geoInterpolate(KELLS, CBUS), pts = [];
      for (let t = 0; t <= 1.0001; t += 0.01) { const p = proj(ip(t)); if (p) pts.push(p); }
      route(c, pts, 3.0, 0.5, 0.9);
      const pk = proj(KELLS); glow(c, pk[0], pk[1], 6, PAL.cream, 0.4); c.fillStyle = 'rgba(246,236,202,0.95)'; c.beginPath(); c.arc(pk[0], pk[1], 1, 0, Math.PI * 2); c.fill();
      c.save(); const ir = new Path2D(d3.geoPath(proj)(d3.geoCircle().center(CBUS).radius(11)())); c.setLineDash([0.1, 2.5]); c.lineCap = 'round'; c.lineWidth = 0.95; c.strokeStyle = 'rgba(233,238,249,0.9)'; c.stroke(ir); c.restore();
    });
  }
  if (layer === 'lights') return vectorLayer(() => {});
  if (layer === 'ship') return vectorLayer(c => { const [x, y] = proj(SHIP[0].at); ship(c, x, y + 1, SHIP[0].h, SHIP[0].dir); });
  // ground
  begin(true);
  wash(null, { c: PAL.sky, bloom: 0.5, mottle: 0.4, brush: 0.2, tide: 0.1, grain: 0.11, specks: 0.002,
    lift: i => { const x = ux(i), y = uy(i); const q = (x * 0.55 + y * -0.84 + 290) / 80; return 0.12 * Math.exp(-q * q); } });
  const orb = maskFrom(c => { c.beginPath(); c.arc(OXu, OYu, OR, 0, Math.PI * 2); c.fill(); });
  const landM = maskFrom(c => { c.beginPath(); gp(c)(d.land); c.fill(); c.globalCompositeOperation = 'destination-out'; c.beginPath(); gp(c)(d.lakes); c.fill(); });
  for (let i = 0; i < N; i++) landM[i] *= orb[i];
  const seaM = new Float32Array(N); for (let i = 0; i < N; i++) seaM[i] = orb[i] * (1 - landM[i]);
  wash(orb, { c: PAL.sea, bloom: 0.75, mottle: 0.5, brush: 0.25, tide: 0.16, grain: 0.12, off: 7,
    lift: i => -0.16 * ((ux(i) - OXu) * 0.6 + (uy(i) - OYu) * 0.8) / OR });
  const dSea = edt(i => landM[i] > 0.5), dLand = edt(i => landM[i] <= 0.5);
  shallows(seaM, dSea, { w: 1.9, alpha: 0.6 });
  const iceM = maskFrom(c => { c.beginPath(); gp(c)(d.ice); c.fill(); });
  for (let i = 0; i < N; i++) iceM[i] *= landM[i];
  wash(landM, { c: PAL.land, bloom: 0.4, mottle: 0.4, brush: 0.3, tide: 0.22, grain: 0.2, specks: 0.005, off: 13, edge: dLand, edgeW: 5, edgeK: 0.25,
    lift: i => -0.12 * ((ux(i) - OXu) * 0.6 + (uy(i) - OYu) * 0.8) / OR });
  wash(iceM, { c: PAL.ice, bloom: 0.7, mottle: 0.5, tide: 0.12, grain: 0.12, specks: 0.002, off: 19 });
  // pen hatching along the coasts, a few moonlit strokes, chalk ridges down the great ranges
  const inLand = (x, y) => at(landM, x, y) > 0.5 && at(iceM, x, y) < 0.3;
  let c = scratch();
  hatch(c, { seed: 11, cell: 5.5, density: 0.9, inside: inLand, angle: (x, y) => tangent(dLand, x, y), jitter: 0.3,
    where: (x, y) => Math.max(Math.exp(-at(dLand, x, y) / K / 5) * 0.95, 0.12), len: 6, space: 1.05, nmin: 3, nmax: 6, w: 0.3, color: PAL.ink, alpha: 0.8 });
  composite();
  c = scratch();
  hatch(c, { seed: 12, cell: 10, density: 0.5, inside: inLand, angle: (x, y) => tangent(dLand, x, y) + 0.5, jitter: 0.4,
    where: () => 0.3, len: 5, space: 1.25, nmin: 2, nmax: 3, w: 0.22, color: '#9cc3bd', alpha: 0.42 });
  composite('normal', 1, chalky(0.15, 0.5));
  c = scratch();
  hatch(c, { seed: 13, cell: 6, density: 0.7, inside: (x, y) => at(iceM, x, y) > 0.5, angle: () => 0.15, len: 7, space: 1.6, nmin: 2, nmax: 4, w: 0.22, color: '#5d74a8', alpha: 0.5 });
  composite();
  const ranges = [
    [[-136, 62], [-128, 57], [-122, 52], [-116, 48], [-112, 44.5], [-108, 40], [-106, 35.5], [-105, 31]],
    [[-74, 10], [-77, 3], [-78.5, -4], [-75.5, -12], [-70, -17], [-67.5, -23], [-69.5, -31], [-70.5, -38], [-72, -46]],
    [[-85.6, 34.6], [-82.3, 36.9], [-79.4, 39.1], [-76.9, 40.9], [-74.6, 42.2], [-72, 44.5], [-69.5, 46]],
  ];
  c = scratch();
  {
    const r = rng(15);
    for (const rg of ranges) {
      const pts = rg.map(p => proj(p)).filter(Boolean);
      for (const [x, y, a] of along(pts, 2.6, 0)) for (let j = -2; j <= 2; j++) {
        if (r() < 0.35) continue;
        const ox = x - Math.sin(a) * j * 2.1 + (r() - 0.5), oy = y + Math.cos(a) * j * 2.1 + (r() - 0.5);
        if (!inLand(ox, oy)) continue;
        const l = 2.2 + r() * 1.6; c.beginPath(); penStroke(c, ox - Math.cos(a) * l, oy - Math.sin(a) * l, ox + Math.cos(a) * l, oy + Math.sin(a) * l, 0.3, (r() - 0.5) * 0.6);
        c.fillStyle = 'rgba(228,236,250,0.8)'; c.fill();
      }
    }
  }
  composite('normal', 0.85, chalky(0.2, 0.5));
  // the great rivers, fine pale threads
  c = scratch(); c.beginPath(); gp(c)(d.rivers); c.strokeStyle = rgba(PAL.pale, 0.75); c.lineWidth = 0.5; c.lineCap = 'round'; c.lineJoin = 'round'; c.stroke();
  composite('normal', 1, i => landM[i] * (1 - iceM[i]));
  // coasts: a dark ink line of varying weight, dotted chalk out in the water, a faint ripple
  c = scratch(); c.beginPath(); gp(c)(d.land); gp(c)(d.lakes); c.strokeStyle = 'rgba(3,6,12,0.9)'; c.lineWidth = 0.8; c.lineJoin = 'round'; c.stroke();
  composite('normal', 1, i => orb[i] * (0.55 + 0.45 * smooth(0.35, 0.65, MOT[fi(i)])));
  shoreLines(seaM, dSea, { dots: 4.4, step: 2.7, minLen: 20, ripples: [[8.5, 0.32]] });
  // the sea's emblem: a chalk triple spiral where the Atlantic gyre turns
  c = scratch(); { const p = proj([-43, 31]); if (p) triskele(c, p[0], p[1], 22, -0.35, { lw: 0.7, alpha: 0.8 }); }
  composite('normal', 1, i => seaM[i] * chalky(0.15, 0.5)(i));
  // a light at Columbus, where the film ends
  c = scratch(); { const p = proj(CBUS); glow(c, p[0], p[1], 10, PAL.cream, 0.6); c.fillStyle = '#fff7de'; c.beginPath(); c.arc(p[0], p[1], 1.2, 0, Math.PI * 2); c.fill(); }
  composite();
  // the orb's edge: a breath of pale air outside, a chalk line on the rim, rings round it as round a courtyard
  for (let i = 0; i < N; i++) {
    const r1 = Math.hypot(ux(i) - OXu, uy(i) - OYu) / OR; if (r1 < 0.9 || r1 > 1.3) continue;
    const t = Math.pow(smooth(0.9, 1.0, r1), 2) * 0.16 * orb[i] + Math.exp(-Math.pow((r1 - 1) / 0.05, 2)) * 0.2 * (1 - orb[i]);
    const q = i * 3; IMG[q] += (0.62 - IMG[q]) * t; IMG[q + 1] += (0.74 - IMG[q + 1]) * t; IMG[q + 2] += (0.98 - IMG[q + 2]) * t;
  }
  c = scratch();
  c.lineWidth = 0.75; c.strokeStyle = 'rgba(236,242,255,0.8)'; c.beginPath(); c.arc(OXu, OYu, OR - 0.2, 0, Math.PI * 2); c.stroke();
  c.lineCap = 'round'; c.strokeStyle = 'rgba(233,238,249,0.75)';
  c.setLineDash([0.1, 3.3]); c.lineWidth = 1.05; c.beginPath(); c.arc(OXu, OYu, OR + 10, 0, Math.PI * 2); c.stroke();
  c.setLineDash([]); c.lineWidth = 0.45; c.strokeStyle = 'rgba(233,238,249,0.45)'; c.beginPath(); c.arc(OXu, OYu, OR + 15, 0, Math.PI * 2); c.stroke();
  composite('normal', 1, chalky(0.18, 0.5));
  // far stars and fine spatter on the ground; the nearer ones are on their own layer
  c = scratch(); spatter(c, 2600, 21, (x, y) => offOrb(x, y, 4), { big: 0.02 }); composite('normal', 0.8);
  c = scratch(); stars(c, 16, 35, (x, y) => offOrb(x, y, 26), { apart: 40, bigShare: 0.05, scale: 0.75 }); composite('normal', 0.75);
  paper(0.4);
  present();
}

// ================================================================ II · North America
async function loadAmerica() {
  const [landT, land10, lakesFC, riversFC, statesFC, placesFC, ohioPlaces] = await Promise.all([D('land-50m.json'), D('america-land.geojson'), D('america-lakes.geojson'),
    D('america-rivers.geojson'), D('america-states.geojson'), D('america-places.geojson'), D('ohio-places.geojson')]);
  return { land50: topojson.feature(landT, landT.objects.land), land10, lakesFC, riversFC, statesFC, placesFC, ohioPlaces };
}
function drawAmerica(layer, d) {
  const proj = lensU(1), gp = c => d3.geoPath(proj, c), pt = ll => proj(ll);
  const areaOf = f => Math.abs(d3.geoPath(proj).area(f));
  const lakes = fc(d.lakesFC.features.filter(f => areaOf(f) > 3)), bigLakes = fc(lakes.features.filter(f => areaOf(f) > 60));
  const ohio = d.statesFC.features.find(f => f.properties.name === 'Ohio');
  const cbus = d.ohioPlaces.features.find(f => f.properties.name === 'Columbus').geometry.coordinates;
  if (layer === 'labels') return vectorLayer(c => {
    const voyage = [[-61, 47.3], [-64.5, 49.2], [-68.5, 48.8], [-71.2, 46.8], [-72.6, 46.2], [-74.2, 45.3], [-75.8, 44.4], [-76.9, 43.75], [-78.5, 43.55], [-79.25, 43.2],
      [-79.4, 42.85], [-80.2, 42.45], [-81.3, 41.95], [-82.4, 41.62], [-82.85, 41.2], [-83.05, 40.6], cbus].map(pt);
    route(c, voyage, 3.0, 0.5, 0.85);
    const oc = pt([-82.7, 40.2]); iris(c, oc[0], oc[1], 60);
    const Lb = (t, ll, size, o) => { const [x, y] = pt(ll); label(c, t, x, y, size, o); };
    Lb('Superior', [-87.6, 47.65], 9, { track: 0.22 });
    Lb('Michigan', [-87.05, 43.6], 8, { rotate: -1.48, track: 0.2 });
    Lb('Huron', [-82.3, 44.9], 8.5, { rotate: -0.62, track: 0.22 });
    Lb('Erie', [-82.05, 41.92], 7.5, { rotate: -0.4, track: 0.24 });
    Lb('Ontario', [-77.9, 43.66], 7.5, { rotate: -0.1, track: 0.22 });
    Lb('Ohio', [-82.75, 40.42], 13, { color: PAL.cream, track: 0.3 });
    Lb('Atlantic', [-72.4, 37.6], 9.5, { color: PAL.pale, track: 0.34, rotate: -0.62 });
  });
  if (layer === 'lights') return vectorLayer(c => {
    const places = [...d.placesFC.features, ...d.ohioPlaces.features.filter(f => ['Cleveland', 'Cincinnati', 'Toledo', 'Dayton', 'Pittsburgh', 'Detroit', 'Indianapolis', 'Louisville'].includes(f.properties.name))];
    for (const f of places) { const p = pt(f.geometry.coordinates); if (!p || f.properties.name === 'Columbus') continue; light(c, p[0], p[1], 5 + Math.min(7, Math.sqrt(f.properties.pop / 1e6) * 2), 0.45); }
    const p = pt(cbus); light(c, p[0], p[1], 16, 0.75);
  });
  if (layer === 'ship') return vectorLayer(c => { const [x, y] = pt(SHIP[1].at); ship(c, x, y + 1.5, SHIP[1].h, SHIP[1].dir); });
  begin(true);
  wash(null, { c: PAL.sea, bloom: 0.75, mottle: 0.5, brush: 0.25, tide: 0.16, grain: 0.12, off: 7 });
  const landM = maskFrom(c => {
    c.beginPath(); gp(c)(d.land50); c.fill(); c.beginPath(); gp(c)(d.land10); c.fill();
    c.globalCompositeOperation = 'destination-out'; c.beginPath(); gp(c)(lakes); c.fill();
  });
  const seaM = new Float32Array(N); for (let i = 0; i < N; i++) seaM[i] = 1 - landM[i];
  const dSea = edt(i => landM[i] > 0.5), dLand = edt(i => landM[i] <= 0.5);
  shallows(seaM, dSea, { w: 2.6, alpha: 0.62 });
  { const small = maskFrom(c => { c.beginPath(); gp(c)(fc(lakes.features.filter(f => areaOf(f) <= 60))); c.fill(); });
    const pm = hex(PAL.paleMid); for (let i = 0; i < N; i++) { const a = small[i] * 0.75; if (a > 0) { const q = i * 3; for (let k = 0; k < 3; k++) IMG[q + k] += (pm[k] - IMG[q + k]) * a; } } }
  const ohioM = maskFrom(c => { c.beginPath(); gp(c)(ohio); c.fill(); });
  for (let i = 0; i < N; i++) ohioM[i] *= landM[i];
  // the north of the lakes is forest and lies darker; the land is lighter round the lakes; Ohio moonlit
  const NORTH = gridOf((x, y) => { const ll = proj.invert([x, y]); return ll ? smooth(45.0, 46.4, ll[1]) : 0; });
  const north = i => NORTH[i];
  wash(landM, { c: PAL.land, bloom: 0.45, mottle: 0.4, brush: 0.3, tide: 0.22, grain: 0.2, specks: 0.005, off: 13, edge: dLand, edgeW: 6, edgeK: 0.25,
    lift: i => ohioM[i] * 0.12 + 0.15 * Math.exp(-dSea[i] / K / 34) - 0.08 * north(i) });
  // the Appalachians: sweeping bands along the ridge line, from Tennessee to the Hudson
  const spine = [[-85.6, 34.6], [-84.1, 35.8], [-82.3, 36.9], [-80.7, 37.9], [-79.4, 39.1], [-78.2, 40.2], [-76.9, 40.9], [-75.6, 41.5], [-74.6, 42.2], [-74.1, 43.2]].map(pt);
  bands(spine, { count: 11, space: 5.4, fill: '#6a9c96', fillAlpha: 0.15, lineAlpha: 0.32, lw: 0.45, taper: 0.3, fadeEnds: 0.28, mask: landM });
  // farmland from the Mississippi to Ohio: a faint quilt of hatched fields, as in the next plate
  const inLand = (x, y) => at(landM, x, y) > 0.5;
  const field = (x, y) => { const ll = proj.invert([x, y]); if (!ll) return 0; return smooth(36.6, 37.6, ll[1]) * smooth(43.6, 42.6, ll[1]) * smooth(-96, -94.5, ll[0]) * smooth(-80.2, -81.4, ll[0]); };
  let c = scratch();
  {
    const r = rng(41), cell = 9;
    for (let gy = 0; gy < CH; gy += cell) for (let gx = 0; gx < CW; gx += cell) {
      const x0 = gx + (r() - 0.5) * 1.5, y0 = gy + (r() - 0.5) * 1.5, q = [r(), r(), r(), r()];
      const cx = x0 + cell / 2, cy = y0 + cell / 2;
      if (!onPlate(cx, cy) || !inLand(cx, cy) || q[0] > 0.55 * field(cx, cy) || at(dLand, cx, cy) < 4 * K) continue;
      const horiz = q[1] < 0.5, sp = 1.4 + q[2] * 0.4, m = 1.6, ww = cell - m * 2;
      c.beginPath();
      for (let t = m; t < cell - m; t += sp) {
        if (horiz) penStroke(c, x0 + m, y0 + t, x0 + m + ww, y0 + t, 0.22, 0.1);
        else penStroke(c, x0 + t, y0 + m, x0 + t, y0 + m + ww, 0.22, 0.1);
      }
      c.fillStyle = q[3] < 0.65 ? 'rgba(160,205,196,0.5)' : 'rgba(3,8,12,0.6)'; c.fill();
    }
  }
  composite('normal', 1, chalky(0.12, 0.45));
  // hatching: dark ink along the shores, a few moonlit strokes inland
  c = scratch();
  hatch(c, { seed: 21, cell: 5.5, density: 0.9, inside: inLand, angle: (x, y) => tangent(dLand, x, y), jitter: 0.3,
    where: (x, y) => Math.max(Math.exp(-at(dLand, x, y) / K / 6) * 0.95, 0.12) * (1 - 0.5 * field(x, y)), len: 6.5, space: 1.1, nmin: 3, nmax: 6, w: 0.3, color: PAL.ink, alpha: 0.78 });
  composite();
  // the forest north of the lakes: the film's dark round crowns, drawn at continent size
  const forestM = new Float32Array(N); for (let i = 0; i < N; i++) forestM[i] = landM[i] > 0.5 ? north(i) : 0;
  c = scratch();
  {
    const r = rng(43), got = [];
    for (let k = 0; k < 26000; k++) {
      const x = r() * CW, y = r() * CH, rad = 2.4 + r() * 1.8, q = r();
      if (!onPlate(x, y) || at(forestM, x, y) < 0.05 || q > at(forestM, x, y) || at(dLand, x, y) < (rad + 1.5) * K) continue;
      if (got.some(([gx, gy, gr]) => Math.abs(gx - x) < 9 && Math.abs(gy - y) < 9 && Math.hypot(gx - x, gy - y) < (gr + rad) * 0.95)) continue;
      got.push([x, y, rad]);
    }
    got.sort((a, b) => a[1] - b[1]);
    for (const [x, y, rad] of got) crown(c, x, y, rad, { lit: '#24463f', dark: '#08161a', rim: 0.32, shadow: 0.35, curl: false });
  }
  composite('normal', 0.9);
  // shores: ink round the coast and the big lakes, dotted chalk out on the water, ripples beyond
  c = scratch(); c.beginPath(); gp(c)(d.land10); gp(c)(bigLakes); c.strokeStyle = 'rgba(3,6,12,0.9)'; c.lineWidth = 0.9; c.lineJoin = 'round'; c.stroke();
  composite('normal', 1, i => 0.55 + 0.45 * smooth(0.35, 0.65, MOT[fi(i)]));
  shoreLines(seaM, dSea, { dots: 5.0, step: 2.8, minLen: 30, ripples: [[10, 0.3], [16, 0.15]] });
  // rivers: the St Lawrence as the film's water, the rest as pale threads
  const RIV = ['Mississippi', 'Missouri', 'Ohio', 'Tennessee', 'Arkansas', 'Wabash', 'Illinois', 'Hudson', 'Potomac', 'Susquehanna', 'Allegheny', 'Monongahela', 'Ottawa', 'Cumberland', 'Kentucky', 'Kanawha', 'Scioto', 'Muskingum'];
  c = scratch(); c.lineCap = 'round'; c.lineJoin = 'round';
  for (const f of d.riversFC.features) {
    const n = f.properties.name; if (!RIV.includes(n)) continue;
    c.beginPath(); gp(c)(f); c.strokeStyle = rgba(PAL.paleMid, 0.85); c.lineWidth = ['Mississippi', 'Ohio', 'Missouri'].includes(n) ? 1.4 : 0.85; c.stroke();
    c.strokeStyle = 'rgba(241,245,255,0.5)'; c.lineWidth = 0.32; c.stroke();
  }
  const stl = fc(d.riversFC.features.filter(f => f.properties.name === 'Saint Lawrence'));
  c.beginPath(); gp(c)(stl); c.strokeStyle = 'rgba(241,245,255,0.85)'; c.lineWidth = 3.4; c.stroke(); c.strokeStyle = rgba(PAL.paleMid, 1); c.lineWidth = 2.5; c.stroke(); c.strokeStyle = rgba(PAL.paleDeep, 0.6); c.lineWidth = 0.9; c.stroke();
  composite('normal', 1, within(landM));
  c = scratch(); c.fillStyle = 'rgba(241,245,255,0.85)';
  for (const f of stl.features) for (const sg of linePts(gp, f)) for (const [x, y] of along(sg, 3.0, 1)) { c.beginPath(); c.arc(x, y, 0.4, 0, Math.PI * 2); c.fill(); }
  composite('normal', 1, within(landM));
  // borders: faint dashed chalk between the states, Ohio's own a fine solid line
  c = scratch(); c.beginPath(); for (const f of d.statesFC.features) if (f.properties.name !== 'Ohio') gp(c)(f);
  c.strokeStyle = 'rgba(214,226,246,0.3)'; c.lineWidth = 0.42; c.setLineDash([1.8, 1.6]); c.stroke(); c.setLineDash([]);
  composite('normal', 1, within(landM));
  c = scratch(); c.beginPath(); gp(c)(ohio); c.strokeStyle = 'rgba(236,242,255,0.7)'; c.lineWidth = 0.7; c.lineJoin = 'round'; c.stroke();
  composite('normal', 1, i => landM[i] * chalky(0.15, 0.5)(i));
  c = scratch(); spatter(c, 2600, 27, (x, y) => at(seaM, x, y) > 0.9 && at(dSea, x, y) > 12 * K, { big: 0.03 }); composite('normal', 0.8);
  c = scratch(); stars(c, 9, 29, (x, y) => at(dSea, x, y) > 18 * K && at(seaM, x, y) > 0.9, { apart: 40 }); composite();
  paper(0.45);
  present();
}
// A function of position evaluated on a coarse grid (every 8 device px) and read back bilinearly.
function gridOf(fn) {
  const G = 8, gw = Math.ceil(W / G) + 1, gh = Math.ceil(H / G) + 1, g = new Float32Array(gw * gh);
  for (let y = 0; y < gh; y++) for (let x = 0; x < gw; x++) g[y * gw + x] = fn(((x * G) - OX) / K, ((y * G) - OY) / K);
  const f = new Float32Array(N);
  for (let y = 0; y < H; y++) {
    const gy = y / G, y0 = Math.floor(gy), ty = gy - y0;
    for (let x = 0; x < W; x++) {
      const gx = x / G, x0 = Math.floor(gx), tx = gx - x0, a = g[y0 * gw + x0], b = g[y0 * gw + x0 + 1], c = g[(y0 + 1) * gw + x0], d = g[(y0 + 1) * gw + x0 + 1];
      f[y * W + x] = (a + (b - a) * tx) * (1 - ty) + (c + (d - c) * tx) * ty;
    }
  }
  return f;
}

// ================================================================ III · Ohio
async function loadOhio() {
  const [land10, aLakes, aStates, oLakes, oStates, riversFC, placesFC, roadsFC, cbusRoads] = await Promise.all([D('america-land.geojson'), D('america-lakes.geojson'),
    D('america-states.geojson'), D('ohio-lakes.geojson'), D('ohio-states.geojson'), D('ohio-rivers.geojson'), D('ohio-places.geojson'), D('ohio-roads.geojson'), D('columbus-roads.geojson')]);
  const ring = fc(cbusRoads.features.filter(f => f.properties.class === 'interstate' && ['Outerbelt', 'I- 270', 'Jack Nicklaus Fwy'].includes(f.properties.name)));
  return { land10, aLakes, aStates, oLakes, oStates, riversFC, placesFC, roadsFC, ring };
}
function drawOhio(layer, d) {
  const proj = lensU(2), gp = c => d3.geoPath(proj, c), pt = ll => proj(ll);
  const ohio = d.oStates.features.find(f => f.properties.name === 'Ohio');
  const place = n => d.placesFC.features.find(f => f.properties.name === n).geometry.coordinates;
  // the finer Ohio files inside their box (-88.5, 35)..(-77, 45.5), the coarser America files outside
  const BX = [-88.5, 35, -77, 45.5];
  const box = []; const edge = (a, b, n) => { for (let k = 0; k <= n; k++) box.push(pt([lerp(a[0], b[0], k / n), lerp(a[1], b[1], k / n)])); };
  edge([BX[0], BX[3]], [BX[2], BX[3]], 40); edge([BX[2], BX[3]], [BX[2], BX[1]], 40); edge([BX[2], BX[1]], [BX[0], BX[1]], 40); edge([BX[0], BX[1]], [BX[0], BX[3]], 40);
  const outsideBox = c => { c.beginPath(); c.rect(-200, -200, CW + 400, CH + 400); const b = box.filter(Boolean); c.moveTo(b[0][0], b[0][1]); for (const q of b.slice(1)) c.lineTo(q[0], q[1]); c.closePath(); c.clip('evenodd'); };
  const onEdge = ([x, y]) => Math.abs(x - BX[0]) < 1e-3 || Math.abs(x - BX[2]) < 1e-3 || Math.abs(y - BX[1]) < 1e-3 || Math.abs(y - BX[3]) < 1e-3;
  const borders = (c, f) => {
    const polys = f.geometry.type === 'Polygon' ? [f.geometry.coordinates] : f.geometry.coordinates;
    for (const pc of polys) for (const rg of pc) for (let k = 1; k < rg.length; k++) {
      if (onEdge(rg[k - 1]) && onEdge(rg[k])) continue;
      const a = pt(rg[k - 1]), b = pt(rg[k]); if (!a || !b) continue; c.moveTo(a[0], a[1]); c.lineTo(b[0], b[1]);
    }
  };
  const cb = pt(place('Columbus'));
  if (layer === 'labels') return vectorLayer(c => {
    const sandusky = [[-82.55, 41.55], [-82.8, 41.47], [-83.05, 41.38], [-83.15, 41.15], [-83.25, 40.86], [-83.2, 40.55], [-83.179, 40.3523], [-83.1434, 40.2696], [-83.14, 40.21], [-83.08, 40.12]].map(pt);
    route(c, sandusky, 3.2, 0.55, 0.9);
    iris(c, cb[0] - 1, cb[1] - 1, 27);
    const Lb = (t, ll, size, o) => { const [x, y] = pt(ll); label(c, t, x, y, size, o); };
    Lb('Lake Erie', [-81.25, 42.22], 12, { track: 0.3, rotate: -0.36 });
    Lb('Columbus', [-82.99, 39.66], 10.5, { color: PAL.cream, track: 0.2 });
    Lb('Cleveland', [-81.45, 41.29], 8.5, { color: PAL.cream, track: 0.18 });
    Lb('Cincinnati', [-84.42, 39.37], 8.5, { color: PAL.cream, track: 0.18 });
    Lb('Toledo', [-83.56, 41.53], 6.5, { track: 0.2, alpha: 0.85 });
    Lb('Dayton', [-84.2, 39.64], 6.5, { track: 0.2, alpha: 0.85 });
    Lb('Michigan', [-84.55, 42.25], 7.5, { color: PAL.pale, track: 0.36, alpha: 0.7 });
    Lb('Indiana', [-85.25, 39.9], 7.5, { color: PAL.pale, track: 0.36, alpha: 0.7, rotate: -1.5708 });
    Lb('Kentucky', [-83.5, 38.15], 7.5, { color: PAL.pale, track: 0.36, alpha: 0.7 });
    Lb('West Virginia', [-80.75, 38.75], 7, { color: PAL.pale, track: 0.3, alpha: 0.7, rotate: -0.95 });
    Lb('Pennsylvania', [-80.25, 41.1], 7, { color: PAL.pale, track: 0.3, alpha: 0.7, rotate: -1.5708 });
    Lb('Ontario', [-81.6, 42.75], 7.5, { color: PAL.pale, track: 0.36, alpha: 0.7, rotate: -0.25 });
    pathLabel(c, 'Ohio River', [[-84.2, 38.84], [-84.036, 38.779], [-83.871, 38.726], [-83.707, 38.683], [-83.543, 38.653], [-83.379, 38.639], [-83.214, 38.645], [-83.05, 38.673]].map(pt), 8, { dy: 10, track: 0.26, color: PAL.pale });
  });
  if (layer === 'lights') return vectorLayer(c => {
    for (const f of d.placesFC.features) {
      const n = f.properties.name; if (['Columbus', 'Cleveland', 'Cincinnati'].includes(n)) continue;
      const p = pt(f.geometry.coordinates); if (!p) continue; light(c, p[0], p[1], 3.5 + Math.min(5, Math.sqrt(f.properties.pop / 1e5) * 0.8), 0.42);
    }
    for (const n of ['Cleveland', 'Cincinnati']) { const p = pt(place(n)); glow(c, p[0], p[1], 17, PAL.cream, 0.42); }
    glow(c, cb[0], cb[1], 34, PAL.cream, 0.55);
  });
  if (layer === 'ship') return vectorLayer(c => { const [x, y] = pt(SHIP[2].at); ship(c, x, y + 2, SHIP[2].h, SHIP[2].dir); });
  begin(true);
  wash(null, { c: PAL.sea, bloom: 0.75, mottle: 0.5, brush: 0.3, tide: 0.16, grain: 0.12, off: 7 });
  const landM = maskFrom(c => {
    c.beginPath(); gp(c)(d.land10); c.fill(); c.beginPath(); gp(c)(d.oStates); c.fill();
    c.globalCompositeOperation = 'destination-out';
    c.save(); outsideBox(c); c.beginPath(); gp(c)(d.aLakes); c.fill(); c.restore();
    c.beginPath(); gp(c)(d.oLakes); c.fill();
  });
  const seaM = new Float32Array(N); for (let i = 0; i < N; i++) seaM[i] = 1 - landM[i];
  const ohioM = maskFrom(c => { c.beginPath(); gp(c)(ohio); c.fill(); });
  for (let i = 0; i < N; i++) ohioM[i] *= landM[i];
  const dSea = edt(i => landM[i] > 0.5), dLand = edt(i => landM[i] <= 0.5);
  shallows(seaM, dSea, { w: 3.2, alpha: 0.62 });
  wash(landM, { c: PAL.landDim, bloom: 0.45, mottle: 0.4, brush: 0.3, tide: 0.2, grain: 0.2, specks: 0.005, off: 13, edge: dLand, edgeW: 6, edgeK: 0.2 });
  wash(ohioM, { c: ['#12303a', '#1e4650', '#33625f'], bloom: 0.45, mottle: 0.45, brush: 0.25, tide: 0.22, grain: 0.2, specks: 0.005, off: 31, edge: dLand, edgeW: 6, edgeK: 0.2,
    lift: i => 0.1 * Math.exp(-dSea[i] / K / 40) });
  // the glacier's edge splits Ohio: flat till plains to the north and west, hills to the south-east
  const glacial = [[-80.52, 40.62], [-81.25, 40.78], [-81.9, 40.62], [-82.25, 40.25], [-82.55, 39.65], [-83.1, 39.1], [-83.75, 38.95], [-84.4, 39.05], [-84.82, 39.12]].map(pt);
  const gEnd = glacial[glacial.length - 1];
  const hillsM = maskFrom(c => { c.beginPath(); c.moveTo(glacial[0][0], glacial[0][1]); for (const p of glacial) c.lineTo(p[0], p[1]); c.lineTo(gEnd[0], CH + 200); c.lineTo(CW + 200, CH + 200); c.lineTo(CW + 200, glacial[0][1]); c.closePath(); c.fill(); });
  const plainsO = new Float32Array(N), hillsO = new Float32Array(N);
  for (let i = 0; i < N; i++) { hillsO[i] = ohioM[i] * hillsM[i]; plainsO[i] = ohioM[i] * (1 - hillsM[i]); }
  let c = scratch();
  {
    const r = rng(41), cell = 14;
    for (let gy = 0; gy < CH; gy += cell) for (let gx = 0; gx < CW; gx += cell) {
      const x0 = gx + (r() - 0.5) * 2, y0 = gy + (r() - 0.5) * 2, q = [r(), r(), r(), r()], cx = x0 + cell / 2, cy = y0 + cell / 2;
      const jit = []; for (let k = 0; k < 12; k++) jit.push([r(), r()]);
      if (!onPlate(cx, cy) || at(plainsO, cx, cy) < 0.5 || q[0] < 0.5) continue;
      const horiz = q[1] < 0.5, sp = 1.6 + q[2] * 0.5, m = 2.2, ww = cell - m * 2;
      c.beginPath();
      let k = 0;
      for (let t = m; t < cell - m; t += sp, k++) {
        const [j0, j1] = jit[k % 12];
        if (horiz) penStroke(c, x0 + m, y0 + t, x0 + m + ww, y0 + t + (j0 - 0.5) * 0.4, 0.28, (j1 - 0.5) * 0.5);
        else penStroke(c, x0 + t, y0 + m, x0 + t + (j0 - 0.5) * 0.4, y0 + m + ww, 0.28, (j1 - 0.5) * 0.5);
      }
      c.fillStyle = q[3] < 0.65 ? 'rgba(160,205,196,0.62)' : 'rgba(3,8,12,0.7)'; c.fill();
    }
  }
  composite('normal', 1, i => plainsO[i] * chalky(0.12, 0.45)(i));
  // hills: sweeping bands, the level lines of distance from the glacier's edge
  const gl = maskFrom(cc => { cc.beginPath(); cc.moveTo(glacial[0][0], glacial[0][1]); for (const q of glacial) cc.lineTo(q[0], q[1]); cc.lineWidth = 1; cc.stroke(); });
  const dGl = edt(i => gl[i] > 0.3);
  const BW0 = 7.5;
  for (let i = 0; i < N; i++) dGl[i] = dGl[i] / K + (MOT[fi(i)] - 0.5) * 5 + (bloomAt(i) - 0.5) * 8;
  {
    const lite = hex('#4f8580');
    for (let i = 0; i < N; i++) {
      const a = hillsO[i]; if (a <= 0) continue;
      const t = dGl[i] / BW0, f = (Math.floor(t) % 2) ? 0.16 * smooth(0, 0.18, t % 1) * smooth(1, 0.82, t % 1) : 0;
      const q = i * 3; for (let k = 0; k < 3; k++) IMG[q + k] += (lite[k] - IMG[q + k]) * f * a;
    }
  }
  c = scratch();
  {
    const dd = new Float32Array(N); for (let i = 0; i < N; i++) dd[i] = dGl[i] * K;
    c.strokeStyle = 'rgba(226,236,250,0.42)'; c.lineWidth = 0.48; c.lineJoin = 'round';
    for (let k = 1; k < 26; k++) for (const rg of isolines(dd, k * BW0)) {
      if (polyLength(rg) < 30) continue;
      c.beginPath(); rg.forEach((q, j) => j ? c.lineTo(q[0], q[1]) : c.moveTo(q[0], q[1])); c.stroke();
    }
  }
  composite('normal', 1, i => hillsO[i] * chalky(0.12, 0.45)(i) * smooth(0.5, 3, dGl[i]));
  c = scratch();
  hatch(c, { seed: 44, cell: 7, density: 0.8, inside: (x, y) => at(hillsO, x, y) > 0.5, angle: (x, y) => 0.65 + (FA(MOT, x, y) - 0.5), jitter: 0.3,
    where: () => 0.55, len: 4.5, space: 1.0, nmin: 3, nmax: 5, w: 0.26, color: PAL.ink, alpha: 0.6 });
  composite();
  c = scratch();
  hatch(c, { seed: 45, cell: 6, density: 0.85, inside: (x, y) => at(landM, x, y) > 0.5 && at(ohioM, x, y) < 0.5, angle: (x, y) => tangent(dLand, x, y), jitter: 0.3,
    where: (x, y) => Math.max(Math.exp(-at(dLand, x, y) / K / 6), 0.18), len: 6, space: 1.1, nmin: 3, nmax: 5, w: 0.28, color: PAL.ink, alpha: 0.7 });
  composite();
  // shores
  c = scratch(); c.strokeStyle = 'rgba(3,6,12,0.9)'; c.lineWidth = 0.95; c.lineJoin = 'round';
  c.save(); outsideBox(c); c.beginPath(); gp(c)(d.aLakes); c.stroke(); c.restore();
  c.beginPath(); gp(c)(d.oLakes); c.stroke();
  composite('normal', 1, i => (0.55 + 0.45 * smooth(0.35, 0.65, MOT[fi(i)])) * (dSea[i] < 3 * K || dLand[i] < 3 * K ? 1 : 0));
  shoreLines(seaM, dSea, { dots: 6.0, step: 2.9, r: 0.42, minLen: 30, ripples: [[11.5, 0.32], [18, 0.18], [25, 0.1]] });
  // rivers: the Ohio as a ribbon of the film's water, the rest as pale threads
  c = scratch();
  for (const f of d.riversFC.features) {
    const n = f.properties.name; if (n === 'Ohio' || n === 'St. Clair' || n === 'Niagara') continue;
    c.beginPath(); gp(c)(f); c.lineCap = 'round'; c.lineJoin = 'round';
    c.strokeStyle = rgba(PAL.paleMid, 0.9); c.lineWidth = ['Scioto', 'Muskingum', 'Kanawha', 'Allegheny', 'Monongahela', 'Wabash', 'Kentucky'].includes(n) ? 1.3 : 1.0; c.stroke();
    c.strokeStyle = 'rgba(241,245,255,0.5)'; c.lineWidth = 0.35; c.stroke();
  }
  const ohioR = fc(d.riversFC.features.filter(f => f.properties.name === 'Ohio'));
  c.save(); c.lineCap = 'round'; c.lineJoin = 'round'; c.beginPath(); gp(c)(ohioR);
  c.strokeStyle = 'rgba(241,245,255,0.85)'; c.lineWidth = 4.2; c.stroke(); c.strokeStyle = rgba(PAL.paleMid, 1); c.lineWidth = 3.2; c.stroke(); c.strokeStyle = rgba(PAL.paleDeep, 0.6); c.lineWidth = 1.1; c.stroke(); c.restore();
  composite('normal', 1, within(landM));
  c = scratch(); c.fillStyle = 'rgba(241,245,255,0.85)';
  for (const f of ohioR.features) for (const sg of linePts(gp, f)) for (const [x, y] of along(sg, 3.2, 1)) { c.beginPath(); c.arc(x, y, 0.42, 0, Math.PI * 2); c.fill(); }
  composite('normal', 1, within(landM));
  // the old roads between the towns, faint; borders
  c = scratch(); c.beginPath(); gp(c)(d.roadsFC); c.strokeStyle = 'rgba(232,222,190,0.22)'; c.lineWidth = 0.5; c.setLineDash([1.2, 1.6]); c.stroke(); c.setLineDash([]);
  composite('normal', 1, within(landM));
  c = scratch(); c.strokeStyle = 'rgba(214,226,246,0.3)'; c.lineWidth = 0.45; c.setLineDash([2, 1.8]);
  c.beginPath(); for (const f of d.oStates.features) if (f.properties.name !== 'Ohio') borders(c, f); c.stroke();
  c.save(); outsideBox(c); c.beginPath(); for (const f of d.aStates.features) gp(c)(f); c.stroke(); c.restore(); c.setLineDash([]);
  c.beginPath(); borders(c, ohio); c.strokeStyle = 'rgba(236,242,255,0.8)'; c.lineWidth = 0.75; c.lineJoin = 'round'; c.stroke();
  composite('normal', 1, i => landM[i] * chalky(0.15, 0.5)(i));
  // the towns: Cleveland and Cincinnati walled in plan; Columbus walled by its real ring road, with its tower
  c = scratch();
  for (const n of ['Cleveland', 'Cincinnati']) { const p = pt(place(n)); walledPlace(c, p[0], p[1], 7.5); }
  c.beginPath(); gp(c)(d.ring); c.lineCap = 'round'; c.lineJoin = 'round';
  c.strokeStyle = 'rgba(238,243,255,0.95)'; c.lineWidth = 2.2; c.stroke();
  c.strokeStyle = 'rgba(30,50,80,0.9)'; c.lineWidth = 1.1; c.stroke();
  c.setLineDash([0.1, 1.7]); c.strokeStyle = 'rgba(246,236,202,0.95)'; c.lineWidth = 0.8; c.stroke(); c.setLineDash([]);
  tower(c, cb[0], cb[1] + 4, 15);
  composite();
  c = scratch(); spatter(c, 1800, 47, (x, y) => at(seaM, x, y) > 0.9 && at(dSea, x, y) > 9 * K, { big: 0.03 }); composite('normal', 0.8);
  c = scratch(); stars(c, 6, 49, (x, y) => at(dSea, x, y) > 15 * K && at(seaM, x, y) > 0.9, { apart: 36, scale: 1.1 }); composite();
  paper(0.45);
  present();
}

// ================================================================ IV · Columbus
async function loadColumbus() {
  const [roadsFC, waterFC, streamsFC] = await Promise.all([D('columbus-roads.geojson'), D('columbus-water.geojson'), D('columbus-streams.geojson')]);
  return { roadsFC, waterFC, streamsFC };
}
function drawColumbus(layer, d) {
  const proj = lensU(3), gp = c => d3.geoPath(proj, c), gpn = () => d3.geoPath(proj), pt = ll => proj(ll);
  const RC = pt([-82.985, 39.99]), RR = Math.abs(pt([-82.80, 39.99])[0] - pt([-83.17, 39.99])[0]) / 2;
  const bTop = pt([-83, 40.42])[1], bBot = pt([-83, 39.55])[1];
  const fadeAt = (x, y) => (1 - smooth(1.04, 1.55, Math.hypot(x - RC[0], (y - RC[1]) * 1.02) / RR)) * smooth(bTop, bTop + 40, y) * smooth(bBot, bBot - 40, y);
  const ringNames = ['Outerbelt', 'I- 270', 'Jack Nicklaus Fwy'];
  const R = d.roadsFC.features;
  const ring = fc(R.filter(f => f.properties.class === 'interstate' && ringNames.includes(f.properties.name)));
  const spokes = fc(R.filter(f => f.properties.class === 'interstate' && !ringNames.includes(f.properties.name)));
  const majors = fc(R.filter(f => f.properties.class === 'major')), locals = fc(R.filter(f => f.properties.class === 'local'));
  const dt = pt([-83.0006, 39.9612]), sn = pt(Kit.TARGET);
  if (layer === 'labels') return vectorLayer(c => {
    // the voyage: down the Scioto past downtown, then up the Olentangy to its mooring by Short North
    const v = [[-83.108, 40.075], [-83.103, 40.05], [-83.088, 40.02], [-83.075, 39.99], [-83.045, 39.966], [-83.03, 39.961], [-83.021, 39.965], [-83.0205, 39.975], [-83.0211, 39.982]].map(pt);
    route(c, v, 3.0, 0.5, 0.85);
    iris(c, sn[0], sn[1], 13, 0.95);
    const Lb = (t, ll, size, o) => { const [x, y] = pt(ll); label(c, t, x, y, size, o); };
    Lb('Scioto', [-83.035, 39.885], 8.5, { color: PAL.pale, track: 0.28, rotate: -1.33 });
    Lb('Olentangy', [-83.012, 40.075], 7.5, { color: PAL.pale, track: 0.24, rotate: -1.5 });
    Lb('Short North', [Kit.TARGET[0] + 0.004, Kit.TARGET[1] + 0.0125], 7, { color: PAL.cream, track: 0.2 });
    Lb('Downtown', [-82.985, 39.951], 7, { color: PAL.cream, track: 0.2, alpha: 0.9 });
    const arc = []; for (let k = 0; k <= 40; k++) { const t = Math.PI * (1.2 + 0.6 * k / 40); arc.push([RC[0] + Math.cos(t) * (RR + 13), RC[1] + Math.sin(t) * (RR * 0.95 + 13)]); }
    pathLabel(c, 'The Outerbelt', arc, 8, { color: PAL.chalk, track: 0.3, alpha: 0.85 });
  });
  if (layer === 'lights') return vectorLayer(c => {
    c.save(); c.filter = blurU(2); c.beginPath(); gp(c)(majors); c.strokeStyle = rgba(PAL.cream, 0.3); c.lineWidth = 2.2; c.stroke(); c.restore();
    glow(c, dt[0], dt[1], 46, PAL.cream, 0.5); glow(c, dt[0], dt[1], 16, PAL.cream, 0.45);
    glow(c, sn[0], sn[1], 9, PAL.cream, 0.4);
  });
  if (layer === 'ship') {
    if (MS > 6) return vectorLayer(() => {});   // past this the ship would sweep by as a giant
    return vectorLayer(c => { const [x, y] = pt(SHIP[3].at); ship(c, x, y + 2, SHIP[3].h, SHIP[3].dir); });
  }
  const FADE = new Float32Array(N); for (let i = 0; i < N; i++) FADE[i] = fadeAt(ux(i), uy(i));
  const named = /Scioto|Olentangy|Alum|Walnut|Blacklick|Darby|Hoover/;
  const water = fc(d.waterFC.features.filter(f => named.test(f.properties.name) || Math.abs(gpn().area(f)) > 5));
  const mainNamed = maskFrom(c => { c.beginPath(); gp(c)(fc(d.waterFC.features.filter(f => /Scioto|Olentangy/.test(f.properties.name)))); gp(c)(fc(d.streamsFC.features.filter(f => /Scioto|Olentangy/.test(f.properties.name)))); c.lineWidth = 1; c.stroke(); c.fill(); });
  const dMain = edt(i => mainNamed[i] > 0.4);
  const main = fc(water.features.filter(f => { if (/Scioto|Olentangy/.test(f.properties.name)) return true; const q = gpn().centroid(f); return isFinite(q[0]) && at(dMain, q[0], q[1]) < 9 * K && !/Hoover|Walnut|Alum|Blacklick|Darby/.test(f.properties.name); }));
  const mainStreams = fc(d.streamsFC.features.filter(f => /Scioto|Olentangy/.test(f.properties.name)));
  begin(true);
  wash(null, { c: ['#060b1e', '#0b1628', '#142436'], bloom: 0.45, mottle: 0.4, brush: 0.3, tide: 0.2, grain: 0.2, specks: 0.005, off: 13 });
  wash(FADE, { c: ['#10283a', '#1b3f4a', '#2e5a5a'], bloom: 0.5, mottle: 0.45, brush: 0.25, tide: 0.22, grain: 0.2, specks: 0.005, off: 41 });
  const waterM = maskFrom(c => { c.beginPath(); gp(c)(water); c.fill(); c.lineWidth = 1.6; c.lineJoin = 'round'; c.stroke(); c.beginPath(); gp(c)(d.streamsFC); c.lineWidth = 0.9; c.stroke(); });
  const dW = edt(i => waterM[i] > 0.4);
  let c = scratch();
  hatch(c, { seed: 51, cell: 6, density: 0.85, inside: (x, y) => at(waterM, x, y) < 0.2, angle: (x, y) => tangent(dW, x, y), jitter: 0.35,
    where: (x, y) => (0.12 + 0.85 * Math.exp(-at(dW, x, y) / K / 7)) * (0.15 + 0.85 * fadeAt(x, y)), len: 5.5, space: 1.05, nmin: 3, nmax: 5, w: 0.26, color: PAL.ink, alpha: 0.7 });
  composite();
  c = scratch(); c.beginPath(); gp(c)(locals); c.strokeStyle = 'rgba(214,226,246,0.2)'; c.lineWidth = 0.3; c.stroke();
  composite('normal', 1, i => FADE[i] * FADE[i]);
  c = scratch(); c.beginPath(); gp(c)(majors); c.strokeStyle = rgba(PAL.cream, 0.6); c.lineWidth = 0.5; c.lineCap = 'round'; c.stroke();
  composite('normal', 1, i => 0.1 + 0.9 * FADE[i]);
  c = scratch();
  {
    const r = rng(53); let n = 0;
    for (let k = 0; k < 40000 && n < 1800; k++) {
      const x = r() * CW, y = r() * CH, q = r(), rad = 0.9 + r() * 0.9, rot = r() * 6;
      if (!onPlate(x, y)) continue;
      const dd = at(dW, x, y) / K;
      if (dd < 1.4 || dd > 7 || q > 0.9 * Math.exp(-(dd - 1.4) / 2.6) * fadeAt(x, y)) continue;
      crown(c, x, y, rad, { rim: 0.45, rot }); n++;
    }
  }
  composite();
  c = scratch(); c.lineJoin = 'round'; c.lineCap = 'round';
  c.beginPath(); gp(c)(d.streamsFC); c.strokeStyle = rgba(PAL.paleMid, 0.6); c.lineWidth = 0.55; c.stroke();
  c.beginPath(); gp(c)(water); c.strokeStyle = rgba(PAL.paleMid, 0.8); c.lineWidth = 0.9; c.stroke(); c.fillStyle = rgba(PAL.paleMid, 0.9); c.fill();
  c.beginPath(); gp(c)(main); gp(c)(mainStreams); c.strokeStyle = 'rgba(241,245,255,0.9)'; c.lineWidth = 2.7; c.stroke(); c.strokeStyle = rgba(PAL.paleMid, 1); c.lineWidth = 1.9; c.stroke();
  c.beginPath(); gp(c)(main); c.fillStyle = rgba(PAL.paleMid, 1); c.fill(); c.strokeStyle = rgba(PAL.pale, 0.7); c.lineWidth = 0.5; c.stroke();
  composite('normal', 1, i => 0.15 + 0.85 * FADE[i]);
  const mainM = maskFrom(cc => { cc.beginPath(); gp(cc)(main); cc.fill(); cc.beginPath(); gp(cc)(main); gp(cc)(mainStreams); cc.lineWidth = 2.6; cc.lineJoin = 'round'; cc.stroke(); });
  const dMainW = edt(i => mainM[i] > 0.4);
  c = scratch(); c.fillStyle = 'rgba(241,245,255,0.75)';
  for (const rg of isolines(dMainW, 2.6)) { if (polyLength(rg) < 40) continue; for (const [x, y] of along(rg, 3.0, 1)) { if (!onPlate(x, y, 2) || fadeAt(x, y) < 0.4) continue; c.beginPath(); c.arc(x, y, 0.36, 0, Math.PI * 2); c.fill(); } }
  composite('normal', 0.8);
  c = scratch(); c.beginPath(); gp(c)(spokes); c.lineCap = 'round'; c.lineJoin = 'round';
  c.strokeStyle = 'rgba(236,242,255,0.75)'; c.lineWidth = 1.3; c.stroke(); c.strokeStyle = 'rgba(20,38,60,0.9)'; c.lineWidth = 0.5; c.stroke();
  composite('normal', 1, i => (0.15 + 0.85 * FADE[i]) * chalky(0.15, 0.5)(i));
  c = scratch(); c.beginPath(); gp(c)(ring); c.lineCap = 'round'; c.lineJoin = 'round';
  c.strokeStyle = 'rgba(240,244,255,0.95)'; c.lineWidth = 5.2; c.stroke();
  c.strokeStyle = 'rgba(16,30,52,1)'; c.lineWidth = 3.6; c.stroke();
  c.setLineDash([0.1, 2.6]); c.strokeStyle = 'rgba(246,236,202,0.95)'; c.lineWidth = 1.15; c.stroke(); c.setLineDash([]);
  composite('normal', 1, chalky(0.1, 0.42));
  c = scratch(); c.filter = blurU(3.3); c.beginPath(); gp(c)(ring); c.strokeStyle = rgba(PAL.pale, 0.35); c.lineWidth = 7; c.stroke(); c.filter = 'none';
  composite('screen', 0.5);
  c = scratch(); tower(c, dt[0], dt[1] + 3, 46, { edge: 0.9 }); composite();
  paper(0.45);
  present();
}

// ================================================================ V · Short North
async function loadShortNorth() {
  const [roadsFC, bldFC, greenFC, treesFC, railFC] = await Promise.all(['roads', 'buildings', 'green', 'trees', 'rail'].map(n => D(`shortnorth-${n}.geojson`)));
  return { roadsFC, bldFC, greenFC, treesFC, railFC };
}
function drawShortNorth(layer, d) {
  const proj = lensU(4), gp = c => d3.geoPath(proj, c), pt = ll => proj(ll);
  const cls = k => fc(d.roadsFC.features.filter(f => k.includes(f.properties.class)));
  const high = fc(d.roadsFC.features.filter(f => f.properties.name === 'North High Street' && ['primary', 'secondary', 'tertiary'].includes(f.properties.class)));
  const mainR = cls(['primary', 'secondary', 'tertiary', 'primary_link', 'secondary_link']), resR = cls(['residential', 'unclassified', 'living_street']);
  const svcR = cls(['service']), footR = cls(['footway', 'path', 'cycleway', 'pedestrian', 'steps']), motorR = cls(['motorway', 'motorway_link']);
  const byKind = k => fc(d.greenFC.features.filter(f => k.includes(f.properties.kind)));
  const parks = byKind(['park', 'garden', 'playground', 'pitch']), grass = byKind(['grass']), woods = byKind(['forest', 'wood', 'scrub']);
  // High Street's centre line, south to north, for the arches
  const highLine = (() => {
    const segs = []; for (const f of high.features) for (const sg of linePts(gp, f)) segs.push(sg);
    const pts = segs.flat().sort((a, b) => b[1] - a[1]);
    const outp = []; for (let y = CH + 20; y > -20; y -= 4) { const near = pts.filter(p => Math.abs(p[1] - y) < 6); if (near.length) outp.push([near.reduce((s, p) => s + p[0], 0) / near.length, y]); }
    return outp;
  })();
  const archY0 = pt([-83.0035, 39.9726])[1], archY1 = pt([-83.0035, 39.9886])[1];
  const arches = [];
  { const n = 17; for (let k = 0; k < n; k++) { const y = lerp(archY0, archY1, k / (n - 1)); let best = null; for (const p of highLine) if (!best || Math.abs(p[1] - y) < Math.abs(best[1] - y)) best = p; if (best) arches.push([best[0], y]); } }
  const highDir = (() => { const a = highLine[0], b = highLine[highLine.length - 1]; return Math.atan2(b[1] - a[1], b[0] - a[0]); })();
  if (layer === 'labels') return vectorLayer(c => {
    // High Street's name along the street, north of the arches; the villages either side, faint
    const top = highLine.filter(p => p[1] < archY1 - 6 && p[1] > archY1 - 150);
    if (top.length > 2) pathLabel(c, 'High Street', top.map(([x, y]) => [x - 11, y]), 9, { track: 0.28, color: PAL.cream });
    label(c, 'Victorian Village', pt([-83.0095, 39.9835])[0], pt([-83.0095, 39.9835])[1], 8.5, { color: PAL.pale, track: 0.28, alpha: 0.75 });
    label(c, 'Italian Village', pt([-82.9985, 39.979])[0], pt([-82.9985, 39.979])[1], 8.5, { color: PAL.pale, track: 0.28, alpha: 0.75 });
    label(c, '© OpenStreetMap contributors', CW / 2 + 186, CH / 2 + 412, 5.5, { color: '#b7c5e6', track: 0.02, alpha: 0.6, align: 'right' });
  });
  if (layer === 'lights') return vectorLayer(c => {
    // the arches over High Street, drawn upright as the film stands its towers in a plan: each a curve
    // of small lamps from kerb to kerb, a soft light pooled under it
    const ca = Math.cos(highDir), sa = Math.sin(highDir);   // along the street (pointing north is -y)
    for (const [x, y] of arches) {
      glow(c, x, y, 13, PAL.cream, 0.32);
      const half = 6.6, rise = 7.5;
      // across the street (perpendicular to its line), rising "up" the page
      const px = -sa, py = ca;   // unit across
      const ptsA = []; for (let s = 0; s <= 24; s++) { const t = s / 24, u = (t - 0.5) * 2 * half, v = -Math.sqrt(Math.max(0, 1 - (u / half) ** 2)) * rise; ptsA.push([x + px * u, y + py * u + v]); }
      c.beginPath(); ptsA.forEach((p, k) => k ? c.lineTo(p[0], p[1]) : c.moveTo(p[0], p[1]));
      c.strokeStyle = 'rgba(246,236,202,0.45)'; c.lineWidth = 0.5; c.stroke();
      for (let s = 0; s <= 24; s += 2) { const [qx, qy] = ptsA[s]; glow(c, qx, qy, 1.4, '#fff3cf', 0.6); c.fillStyle = '#fffaf0'; c.beginPath(); c.arc(qx, qy, 0.32, 0, Math.PI * 2); c.fill(); }
    }
    // lamps along the side streets, and High Street's own glow
    c.save(); c.filter = blurU(3); c.lineCap = 'round'; c.beginPath(); gp(c)(high); c.strokeStyle = rgba(PAL.cream, 0.42); c.lineWidth = 7; c.stroke(); c.restore();
    for (const f of [...mainR.features, ...resR.features]) {
      const main = mainR.features.includes(f);
      if (f.properties.name === 'North High Street') continue;
      for (const sg of linePts(gp, f)) for (const [x, y] of along(sg, main ? 16 : 26, main ? 5 : 11)) {
        if (!onPlate(x, y, 4)) continue;
        glow(c, x, y, main ? 6 : 5, PAL.cream, main ? 0.3 : 0.22); c.fillStyle = 'rgba(255,246,220,0.9)'; c.beginPath(); c.arc(x, y, main ? 0.45 : 0.35, 0, Math.PI * 2); c.fill();
      }
    }
  });
  if (layer === 'near') {
    // a frame of dark round crowns at the plate's edges, nearer the eye, so the street sits deep below
    begin(false);
    const c = scratch(), r = rng(97), got = [];
    for (let k = 0; k < 4000 && got.length < 46; k++) {
      const x = r() * CW, y = r() * CH, rad = 14 + r() * 16;
      const ex = Math.min(x - 70, CW - 70 - x), ey = Math.min(y - 150, CH - 150 - y);
      const out = Math.min(ex / 60, ey / 120);   // < 0 outside the resting screen's inner area
      if (out > -0.2 || (y > CH * 0.62 && out > -0.6)) continue;
      if (got.some(([gx, gy, gr]) => Math.hypot(gx - x, gy - y) < (gr + rad) * 0.7)) continue;
      got.push([x, y, rad]);
    }
    for (const [x, y, rad] of got.sort((a, b) => a[2] - b[2])) crown(c, x, y, rad, { lit: '#1a3533', dark: '#040b0e', rim: 0.35, shadow: 0.55, rot: x });
    composite();
    return present();
  }
  begin(true);
  wash(null, { c: ['#080f1c', '#0e1a2a', '#17293a'], bloom: 0.45, mottle: 0.4, brush: 0.2, tide: 0.18, grain: 0.2, specks: 0.005, off: 61 });
  const parkM = maskFrom(c => { c.beginPath(); gp(c)(parks); gp(c)(grass); gp(c)(woods); c.fill(); });
  wash(parkM, { c: ['#0e2a2e', '#18403f', '#2b5c55'], bloom: 0.45, mottle: 0.4, brush: 0.2, tide: 0.18, grain: 0.18, off: 63 });
  const bldM = maskFrom(c => { c.beginPath(); gp(c)(d.bldFC); c.fill(); });
  const roadM = maskFrom(c => { c.lineCap = 'round'; c.lineJoin = 'round'; c.beginPath(); gp(c)(mainR); gp(c)(motorR); c.lineWidth = 5; c.stroke(); c.beginPath(); gp(c)(resR); c.lineWidth = 3.6; c.stroke(); c.beginPath(); gp(c)(svcR); c.lineWidth = 1.6; c.stroke(); });
  // hatching in the yards and parks
  let c = scratch();
  hatch(c, { seed: 71, cell: 6, density: 0.7, inside: (x, y) => at(bldM, x, y) < 0.3 && at(roadM, x, y) < 0.3, angle: (x, y) => 0.7 + (FA(MOT, x, y) - 0.5) * 2.4, jitter: 0.3,
    where: () => 0.45, len: 4.5, space: 1.0, nmin: 2, nmax: 5, w: 0.25, color: PAL.ink, alpha: 0.55 });
  composite();
  // the freeway in its cut, edged in chalk; the railway as a chalk ladder
  c = scratch(); c.lineCap = 'round'; c.lineJoin = 'round';
  c.beginPath(); gp(c)(motorR); c.strokeStyle = 'rgba(200,214,240,0.5)'; c.lineWidth = 7.4; c.stroke(); c.strokeStyle = '#0b1322'; c.lineWidth = 6.2; c.stroke();
  composite('normal', 1, chalky(0.12, 0.45));
  c = scratch(); c.beginPath(); gp(c)(d.railFC); c.strokeStyle = 'rgba(214,226,246,0.55)'; c.lineWidth = 0.45; c.stroke();
  c.setLineDash([0.4, 1.6]); c.lineWidth = 2.2; c.strokeStyle = 'rgba(214,226,246,0.35)'; c.stroke(); c.setLineDash([]);
  composite('normal', 1, chalky(0.15, 0.5));
  // lanes: dark surfaces washed with lamplight, High Street the brightest
  c = scratch(); c.lineCap = 'round'; c.lineJoin = 'round';
  c.beginPath(); gp(c)(svcR); c.strokeStyle = 'rgba(30,40,62,0.75)'; c.lineWidth = 1.4; c.stroke();
  c.beginPath(); gp(c)(resR); c.strokeStyle = '#1e283f'; c.lineWidth = 3.2; c.stroke();
  c.beginPath(); gp(c)(mainR); c.strokeStyle = '#242e48'; c.lineWidth = 4.4; c.stroke();
  c.beginPath(); gp(c)(high); c.strokeStyle = '#2a3450'; c.lineWidth = 6; c.stroke();
  composite();
  c = scratch(); c.lineCap = 'round'; c.lineJoin = 'round'; c.filter = blurU(3);
  c.beginPath(); gp(c)(resR); c.strokeStyle = rgba(PAL.cream, 0.32); c.lineWidth = 3; c.stroke();
  c.beginPath(); gp(c)(mainR); c.strokeStyle = rgba(PAL.cream, 0.55); c.lineWidth = 4.4; c.stroke();
  c.filter = 'none';
  composite('screen', 0.75);
  c = scratch(); c.lineCap = 'round'; c.lineJoin = 'round';
  c.beginPath(); gp(c)(resR); c.strokeStyle = rgba(PAL.cream, 0.24); c.lineWidth = 0.45; c.stroke();
  c.beginPath(); gp(c)(mainR); c.strokeStyle = rgba(PAL.cream, 0.42); c.lineWidth = 0.6; c.stroke();
  c.beginPath(); gp(c)(high); c.strokeStyle = rgba(PAL.cream, 0.7); c.lineWidth = 0.8; c.stroke();
  composite();
  // buildings: dark roofs with a shadow, a faint moonlit edge, the ridge along the long side
  c = scratch(); c.save(); c.translate(1.2, 1.4); c.beginPath(); gp(c)(d.bldFC); c.fillStyle = 'rgba(2,5,12,0.55)'; c.fill(); c.restore();
  composite();
  c = scratch();
  c.beginPath(); gp(c)(d.bldFC); c.fillStyle = '#121828'; c.fill();
  c.strokeStyle = 'rgba(190,206,236,0.24)'; c.lineWidth = 0.3; c.stroke();
  for (const f of d.bldFC.features) {
    const rg = f.geometry.coordinates[0].map(pt).filter(Boolean); if (rg.length < 4) continue;
    if (!onPlate(rg[0][0], rg[0][1], 10)) continue;
    let best = 0, th = 0;
    for (let k = 1; k < rg.length; k++) { const dx = rg[k][0] - rg[k - 1][0], dy = rg[k][1] - rg[k - 1][1], l = dx * dx + dy * dy; if (l > best) { best = l; th = Math.atan2(dy, dx); } }
    const uxv = Math.cos(th), uyv = Math.sin(th);
    let u0 = 1e9, u1 = -1e9, v0 = 1e9, v1 = -1e9;
    for (const [x, y] of rg) { const u = x * uxv + y * uyv, v = -x * uyv + y * uxv; u0 = Math.min(u0, u); u1 = Math.max(u1, u); v0 = Math.min(v0, v); v1 = Math.max(v1, v); }
    if ((u1 - u0) * (v1 - v0) > 260 || u1 - u0 < 2) continue;
    const vm = (v0 + v1) / 2, inset = Math.min((v1 - v0) / 2, (u1 - u0) * 0.3);
    const X = (u, v) => [u * uxv - v * uyv, u * uyv + v * uxv];
    const [x0, y0] = X(u0 + inset, vm), [x1, y1] = X(u1 - inset, vm), p1 = X(u0, v0), p2 = X(u1, v0);
    c.beginPath(); c.moveTo(p1[0], p1[1]); c.lineTo(p2[0], p2[1]); c.lineTo(x1, y1); c.lineTo(x0, y0); c.closePath();
    c.fillStyle = (-uyv - uxv) > 0 ? 'rgba(80,98,140,0.26)' : 'rgba(0,0,0,0.15)'; c.fill();
    c.beginPath(); c.moveTo(x0, y0); c.lineTo(x1, y1); c.strokeStyle = 'rgba(226,234,250,0.36)'; c.lineWidth = 0.28; c.stroke();
  }
  composite();
  // trees: the mapped ones (the rows along the streets), the parks' woods filled with crowns
  const crowns = [];
  for (const f of d.treesFC.features) { const p = pt(f.geometry.coordinates); if (p && onPlate(p[0], p[1], 6)) crowns.push([p[0], p[1], 2.3 + hashI(crowns.length + 7) * 1.5]); }
  const woodM = maskFrom(cc => { cc.beginPath(); gp(cc)(woods); cc.fill(); });
  { const r = rng(73); for (let k = 0; k < 20000; k++) { const x = r() * CW, y = r() * CH, rad = 2.4 + r() * 1.4; if (!onPlate(x, y) || at(woodM, x, y) < 0.5) continue; if (crowns.some(([cx, cy, cr]) => Math.abs(cx - x) < 6 && Math.abs(cy - y) < 6 && Math.hypot(cx - x, cy - y) < (cr + rad) * 0.62)) continue; crowns.push([x, y, rad]); } }
  c = scratch(); crowns.sort((a, b) => a[1] - b[1]);
  for (const [x, y, rad] of crowns) crown(c, x, y, rad, { rot: x * 0.7 + y, lit: '#2d5a4c', dark: '#0a1d1f', rim: 0.5 });
  composite();
  // footpaths: dotted chalk, in the parks and across the plazas only (the pavements stay quiet)
  c = scratch(); c.beginPath(); gp(c)(footR); c.setLineDash([0.1, 1.6]); c.lineCap = 'round'; c.strokeStyle = 'rgba(230,238,250,0.5)'; c.lineWidth = 0.55; c.stroke();
  composite('normal', 1, i => parkM[i] * (1 - bldM[i]));
  paper(0.5);
  present();
}

// ---------------------------------------------------------------- drawing a layer, and redrawing it magnified
const LOADERS = [loadWorld, loadAmerica, loadOhio, loadColumbus, loadShortNorth];
const DRAWERS = [drawWorld, drawAmerica, drawOhio, drawColumbus, drawShortNorth];
let DATA = null;
function render() {
  const k = LEVEL - 1;
  DRAWERS[k](LAYER, DATA);
}
async function start() {
  await Promise.all(["20px 'Uncial Antiqua'", '20px Alegreya'].map(f => document.fonts.load(f)));
  makeFields();
  DATA = await LOADERS[LEVEL - 1]();
  setMag(1, W / 2, H / 2);
  render();
  // The exporter magnifies pass plates with a transform on the root element. Redraw at that scale,
  // in this same task (before it screenshots), and take the transform off again.
  const root = document.documentElement;
  let busy = false;
  new MutationObserver(() => {
    if (busy) return;
    const m = /translate\(\s*([-\d.e]+)px,\s*([-\d.e]+)px\)\s*scale\(\s*([-\d.e]+)\s*\)/.exec(root.style.transform || '');
    if (!m) return;
    const tx = +m[1], ty = +m[2], s = +m[3];
    busy = true;
    try {
      setMag(s, (W / 2 - tx) / s, (H / 2 - ty) / s);
      render();
      root.style.transform = 'none';
    } catch (e) { console.error('magnified redraw failed: ' + e.message); }
    busy = false;
  }).observe(root, { attributes: true, attributeFilter: ['style'] });
}
addEventListener('DOMContentLoaded', () => start().then(() => { window.platesReady = true; }, e => { console.error(e.stack); window.platesReady = 'failed: ' + e.message; }));
})();
