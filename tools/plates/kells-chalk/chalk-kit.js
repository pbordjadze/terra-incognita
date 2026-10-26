/* Chalk and Slate: the drawing kit. The map is drawn the way the abbey courtyard is drawn in the
   film: chalk-white line on blue-grey slate washes, plans with elevations standing in them, and
   two accents kept rare (gold, and a red sail).

   Units are points: a plate is 546 x 1181.7 pt at 3 px per pt (Kit.W x Kit.H pixels), and at rest
   the phone shows its middle 390 x 844 pt. Every random choice is seeded, so each layer of a level
   draws from the same geometry and a plate draws the same every time.

   Pass plates: the exporter magnifies the page (a CSS transform on the root) to make them. A canvas
   would only blur, so the page watches for that transform and paints again natively at the
   magnification (levels.js): VIEW below is that magnification. Geometry, line widths and lettering
   scale with it; the slate's fine grain and the chalk's tooth stay screen-sized, the way the board
   looks at any distance.                                                                         */
'use strict';
const DPR = 3, DW = Kit.W, DH = Kit.H, W = DW / DPR, H = DH / DPR, TAU = Math.PI * 2;
let VIEW = { s: 1, cx: W / 2, cy: H / 2 };          // magnification about (cx, cy), in plate points

// ---------------------------------------------------------------- colour
const INK = {
  chalk: '#eef0ea', gold: '#dcb45c', red: '#c94a3c',
  water: '#0c151d', land: '#6c8092', lit: '#a5b5c3',
  deep: [15, 22, 30], mid: [36, 50, 63], hi: [70, 88, 104],
};

// ---------------------------------------------------------------- seeded randomness and noise
function mulberry(seed) {
  return () => { seed |= 0; seed = seed + 0x6D2B79F5 | 0; let t = Math.imul(seed ^ seed >>> 15, 1 | seed); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; };
}
// Two streams: `rand` places things (trees, hills, curls), `srand` shakes the hand (stroke wobble).
// Layers other than the ground draw no strokes of the ground, so placement must not depend on them.
let rand = mulberry(1), srand = mulberry(2);
function resetRandom(level) { rand = mulberry(1000 + level * 7919); srand = mulberry(5000 + level * 104729); }
const rr = (a, b) => a + (b - a) * rand();
const sr = (a, b) => a + (b - a) * srand();
const LN = 256, LM = 255, LAT = new Float32Array(LN * LN);
{ const r = mulberry(4242); for (let i = 0; i < LAT.length; i++) LAT[i] = r(); }
function vn(x, y) {
  const xi = Math.floor(x), yi = Math.floor(y);
  let fx = x - xi, fy = y - yi;
  fx = fx * fx * (3 - 2 * fx); fy = fy * fy * (3 - 2 * fy);
  const x0 = xi & LM, x1 = (xi + 1) & LM, y0 = (yi & LM) * LN, y1 = ((yi + 1) & LM) * LN;
  const a = LAT[y0 + x0], b = LAT[y0 + x1], c = LAT[y1 + x0], d = LAT[y1 + x1];
  return a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy;
}
const RC = [], RS = [];
for (let i = 0; i < 8; i++) { const a = 0.61 + i * 1.13; RC.push(Math.cos(a)); RS.push(Math.sin(a)); }
function fbm(x, y, oct) {
  let s = 0, amp = 0.5, n = 0;
  for (let i = 0; i < oct; i++) {
    s += amp * vn(x * RC[i] - y * RS[i] + 31.7 * i, x * RS[i] + y * RC[i] + 11.3 * i);
    n += amp; x *= 2.07; y *= 2.07; amp *= 0.5;
  }
  return s / n;
}
const clamp = (v, a = 0, b = 1) => v < a ? a : v > b ? b : v;
const smooth = (a, b, v) => { const t = clamp((v - a) / (b - a)); return t * t * (3 - 2 * t); };

// ---------------------------------------------------------------- layers
function viewTransform(g) {
  const s = VIEW.s * DPR;
  g.setTransform(s, 0, 0, s, DPR * (W / 2 - VIEW.s * VIEW.cx), DPR * (H / 2 - VIEW.s * VIEW.cy));
}
function layer() {
  const c = document.createElement('canvas'); c.width = DW; c.height = DH;
  const g = c.getContext('2d'); viewTransform(g);
  g.lineCap = 'round'; g.lineJoin = 'round';
  return g;
}
function put(dst, src, op = 'source-over', a = 1) {
  dst.save(); dst.setTransform(1, 0, 0, 1, 0, 0); dst.globalCompositeOperation = op; dst.globalAlpha = a;
  dst.drawImage(src.canvas || src, 0, 0); dst.restore();
}
const blurPx = pt => `blur(${pt * DPR * VIEW.s}px)`;
// A picture the size of the plate's canvas: fn(X, Y, px, py, out), with (X, Y) the canvas pixel and
// (px, py) the plate point under it.
function texture(fn) {
  const c = document.createElement('canvas'); c.width = DW; c.height = DH;
  const g = c.getContext('2d'), img = g.createImageData(DW, DH), d = img.data, out = [0, 0, 0, 0];
  const k = 1 / (DPR * VIEW.s), ox = VIEW.cx - W / 2 / VIEW.s, oy = VIEW.cy - H / 2 / VIEW.s;
  for (let Y = 0, i = 0; Y < DH; Y++) {
    const py = oy + Y * k;
    for (let X = 0; X < DW; X++, i += 4) {
      fn(X, Y, ox + X * k, py, out); d[i] = out[0]; d[i + 1] = out[1]; d[i + 2] = out[2]; d[i + 3] = out[3];
    }
  }
  g.putImageData(img, 0, 0);
  return c;
}
// The slate's tooth (where chalk skips), uneven wash coverage, and breaks for dotted rings, made once
// per paint since they depend on the magnification.
const TEX = {};
function resetTextures() { for (const k of Object.keys(TEX)) delete TEX[k]; }
function tex(name) {
  if (TEX[name]) return TEX[name];
  if (name === 'tooth') TEX.tooth = texture((X, Y, px, py, o) => {
    const f = vn(X / 1.45 + 300, Y / 1.45), m = vn(X / 6.5, Y / 6.5 + 500), p = fbm(px / 70 + 40, py / 70, 2);
    const v = f * 0.58 + m * 0.42 - (p - 0.5) * 0.5;
    o[0] = o[1] = o[2] = 0; o[3] = 255 * clamp((v - 0.5) * 3.2, 0, 0.92);
  });
  if (name === 'mottle') TEX.mottle = texture((X, Y, px, py, o) => {
    const m = fbm(px / 26 + 90, py / 26, 3), f = vn(X / 2.2, Y / 2.2 + 900);
    o[0] = o[1] = o[2] = 255; o[3] = 255 * clamp(0.42 + (m - 0.5) * 1.5 + (f - 0.5) * 0.14, 0.12, 1);
  });
  if (name === 'break') TEX.break = texture((X, Y, px, py, o) => {
    o[0] = o[1] = o[2] = 0; o[3] = 255 * smooth(0.5, 0.62, vn(px / 4.2 + 70, py / 4.2 + 70));
  });
  return TEX[name];
}
// The slate: mottled washes, watercolour tide lines, a faint vertical grain, and fine grain kept low
// (it is what a JPEG pays for).
function slate(g, { deep = INK.deep, mid = INK.mid, hi = INK.hi, streak = 0.03, wash = 0.8, grain = 0.11 } = {}) {
  put(g, texture((X, Y, px, py, o) => {
    const m = fbm(px / 150, py / 150 + 20, 4);
    const b = fbm(px / 17 + 50, py / 17, 3);
    const gr = vn(X / 1.6, Y / 1.6 + 77);
    const s = vn(px / 2.6 + 9, py / 120);
    const tide = Math.exp(-(((m - 0.53) / 0.011) ** 2)) + 0.6 * Math.exp(-(((m - 0.41) / 0.009) ** 2));
    let v = 0.5 + (m - 0.5) * 2.1 * wash + (b - 0.5) * 0.42 + (gr - 0.5) * grain + (s - 0.5) * streak * 2 - tide * 0.07;
    v = clamp(v);
    const t = v < 0.5 ? v * 2 : (v - 0.5) * 2, A = v < 0.5 ? deep : mid, B = v < 0.5 ? mid : hi;
    o[0] = A[0] + (B[0] - A[0]) * t; o[1] = A[1] + (B[1] - A[1]) * t; o[2] = A[2] + (B[2] - A[2]) * t; o[3] = 255;
  }));
}

// ---------------------------------------------------------------- geometry
function capture(proj, geo) {   // GeoJSON projected into polylines {pts, closed}, in plate points
  const out = []; let cur = null;
  d3.geoPath(proj, {
    moveTo(x, y) { cur = { pts: [[x, y]], closed: false }; out.push(cur); },
    lineTo(x, y) { cur.pts.push([x, y]); },
    closePath() { if (cur) cur.closed = true; },
    arc() {}, beginPath() {},
  })(geo);
  return out.filter(r => r.pts.length > 1);
}
const FC = features => ({ type: 'FeatureCollection', features });
function toPath(lines, close = true) {
  const p = new Path2D();
  for (const l of lines) {
    const pts = l.pts || l;
    p.moveTo(pts[0][0], pts[0][1]);
    for (let i = 1; i < pts.length; i++) p.lineTo(pts[i][0], pts[i][1]);
    if (close && (l.closed !== false)) p.closePath();
  }
  return p;
}
function frameRect() { const p = new Path2D(); p.rect(-40, -40, W + 80, H + 80); return p; }
function unionPath(...paths) { const p = new Path2D(); for (const q of paths) p.addPath(q); return p; }
const ringArea = r => { let a = 0; for (let i = 0, n = r.length; i < n; i++) { const p = r[i], q = r[(i + 1) % n]; a += p[0] * q[1] - q[0] * p[1]; } return a / 2; };
function centroidOf(pts) { let x = 0, y = 0; for (const p of pts) { x += p[0]; y += p[1]; } return [x / pts.length, y / pts.length]; }
function polyLen(p) { let s = 0; for (let i = 1; i < p.length; i++) s += Math.hypot(p[i][0] - p[i - 1][0], p[i][1] - p[i - 1][1]); return s; }
// A path rasterised once at point resolution, for cheap inside-tests.
const MASKS = new Map();
function inPath(path, x, y) {
  let m = MASKS.get(path);
  if (!m) {
    const c = document.createElement('canvas'); c.width = Math.ceil(W); c.height = Math.ceil(H);
    const g = c.getContext('2d'); g.fillStyle = '#fff'; g.fill(path, 'evenodd');
    const d = g.getImageData(0, 0, c.width, c.height).data, a = new Uint8Array(c.width * c.height);
    for (let i = 0; i < a.length; i++) a[i] = d[i * 4 + 3] > 127 ? 1 : 0;
    m = { w: c.width, h: c.height, a }; MASKS.set(path, m);
  }
  const X = Math.round(x), Y = Math.round(y);
  return X >= 0 && Y >= 0 && X < m.w && Y < m.h && m.a[Y * m.w + X] === 1;
}
function resample(pts, step, closed) {
  const src = closed ? pts.concat([pts[0]]) : pts, out = [src[0]];
  let carry = 0;
  for (let i = 1; i < src.length; i++) {
    const [x0, y0] = src[i - 1], [x1, y1] = src[i];
    const L = Math.hypot(x1 - x0, y1 - y0);
    let t = step - carry;
    while (t <= L) { out.push([x0 + (x1 - x0) * t / L, y0 + (y1 - y0) * t / L]); t += step; }
    carry = L - (t - step);
  }
  const last = src[src.length - 1], lp = out[out.length - 1];
  if (Math.hypot(last[0] - lp[0], last[1] - lp[1]) > step * 0.3) out.push(last);
  return out;
}
function wobble(pts, amp, wl, row) {   // a hand's wobble: each point pushed along the normal by slow noise
  const n = pts.length; if (n < 3 || amp <= 0) return pts;
  const out = new Array(n); let s = 0;
  for (let i = 0; i < n; i++) {
    if (i) s += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
    const a = pts[Math.max(0, i - 1)], b = pts[Math.min(n - 1, i + 1)];
    let nx = -(b[1] - a[1]), ny = b[0] - a[0]; const l = Math.hypot(nx, ny) || 1; nx /= l; ny /= l;
    const k = (vn(s / wl, row) - 0.5) * 2 * amp;
    out[i] = [pts[i][0] + nx * k, pts[i][1] + ny * k];
  }
  return out;
}
function strokePts(g, pts, closed) {
  g.beginPath(); g.moveTo(pts[0][0], pts[0][1]);
  for (let i = 1; i < pts.length; i++) g.lineTo(pts[i][0], pts[i][1]);
  if (closed) g.closePath();
  g.stroke();
}
// The points of every piece of a line that pass a test, in order along it (data splits rivers and
// roads at junctions), for setting lettering along them.
function runAlong(lines, test) {
  const pts = lines.flatMap(l => resample(l.pts || l, 2, false).filter(test)); if (pts.length < 2) return pts;
  const xs = pts.map(p => p[0]), ys = pts.map(p => p[1]);
  const byX = Math.max(...xs) - Math.min(...xs) >= Math.max(...ys) - Math.min(...ys);
  const sorted = pts.sort((a, b) => byX ? a[0] - b[0] : a[1] - b[1]), out = [sorted[0]];
  for (const p of sorted) { const q = out[out.length - 1]; if (Math.hypot(p[0] - q[0], p[1] - q[1]) > 1.2) out.push(p); }
  return out;
}
function smoothLine(pts, k = 3) {
  return pts.map((p, i) => {
    let x = 0, y = 0, n = 0;
    for (let j = Math.max(0, i - k); j <= Math.min(pts.length - 1, i + k); j++) { x += pts[j][0]; y += pts[j][1]; n++; }
    return [x / n, y / n];
  });
}
function poisson(n, test, minD, tries = 30) {
  const pts = [];
  for (let k = 0; k < n * tries && pts.length < n; k++) {
    const p = test(); if (!p) continue;
    if (pts.every(q => Math.hypot(q[0] - p[0], q[1] - p[1]) >= minD)) pts.push(p);
  }
  return pts;
}

// ---------------------------------------------------------------- strokes
// One chalk line: two passes of the hand, the second lighter and a little off the first.
function chalk(g, pts, o = {}) {
  const { w = 1, a = 0.9, amp = 0.45, wl = 16, passes = 2, closed = false, step = 1.5, color = INK.chalk, rng = srand } = o;
  if (!g || pts.length < 2) { if (passes) for (let k = 0; k < passes; k++) rng(); return; }
  const base = resample(pts, step, closed);
  g.strokeStyle = color;
  for (let k = 0; k < passes; k++) {
    const q = wobble(base, amp * (k ? 1.35 : 1), wl * (k ? 0.8 : 1), rng() * 200);
    g.globalAlpha = a * (k ? 0.5 : 1); g.lineWidth = w * (k ? 0.65 : 1);
    strokePts(g, q, closed);
  }
  g.globalAlpha = 1;
}
function chalkLines(g, lines, o = {}) { for (const l of lines) chalk(g, l.pts || l, { ...o, closed: o.closed ?? !!l.closed }); }
function circlePts(cx, cy, r, n = 0, a0 = 0, a1 = TAU, ry = r) {
  n = n || Math.max(12, Math.round(Math.abs(a1 - a0) * Math.max(r, ry) / 1.5));
  const out = []; for (let i = 0; i <= n; i++) { const a = a0 + (a1 - a0) * i / n; out.push([cx + Math.cos(a) * r, cy + Math.sin(a) * ry]); }
  return out;
}
// A hand-drawn circle (or ellipse) overshoots a little where it closes.
function handCircle(g, cx, cy, r, o = {}) {
  const rng = o.rng || srand, a0 = rng() * TAU, over = o.over ?? 0.35;
  chalk(g, circlePts(cx, cy, r, 0, a0, a0 + TAU + over, o.ry ?? r), { amp: 0.35, wl: 22, ...o, closed: false });
}
function spiral(cx, cy, r, turns = 2.2, a0 = 0, dir = 1) {
  const out = [], n = Math.round(turns * 28);
  for (let i = 0; i <= n; i++) { const t = i / n, a = a0 + dir * t * turns * TAU, rad = r * (1 - t * 0.92); out.push([cx + Math.cos(a) * rad, cy + Math.sin(a) * rad]); }
  return out;
}
function erodeWith(g, t, a = 1) {
  g.save(); g.setTransform(1, 0, 0, 1, 0, 0); g.globalCompositeOperation = 'destination-out'; g.globalAlpha = a; g.drawImage(t, 0, 0); g.restore();
}
function wash(g, path, color, a, o = {}) {
  const t = layer();
  t.fillStyle = color; t.fill(path, 'evenodd');
  put(t, tex('mottle'), 'destination-in');
  put(g, t, 'source-over', a);
}
// Rubbed chalk: a soft white glow, a finger dragged through chalk, clipped to a shape.
function rub(g, path, clip, w, a, blur) {
  g.save(); if (clip) g.clip(clip, 'evenodd');
  g.filter = blurPx(blur); g.globalAlpha = a; g.strokeStyle = INK.chalk; g.lineWidth = w; g.stroke(path);
  g.restore();
}
function hatch(g, clip, o = {}) {
  const { angle = -0.78, gap = 3.4, w = 0.5, a = 0.25, box = [0, 0, W, H], broken = 0.35, amp = 0.35 } = o;
  g.save(); g.clip(clip, 'evenodd');
  const [x0, y0, x1, y1] = box, cx = (x0 + x1) / 2, cy = (y0 + y1) / 2, R = Math.hypot(x1 - x0, y1 - y0) / 2;
  const ca = Math.cos(angle), sa = Math.sin(angle);
  for (let d = -R; d <= R; d += gap * sr(0.8, 1.2)) {
    let t = -R;
    while (t < R) {
      const len = sr(6, 26), skip = srand() < broken ? sr(3, 12) : sr(0.5, 2);
      const p0 = [cx + ca * t - sa * d, cy + sa * t + ca * d], p1 = [cx + ca * (t + len) - sa * d, cy + sa * (t + len) + ca * d];
      chalk(g, [p0, p1], { w: w * sr(0.7, 1.2), a: a * sr(0.6, 1.1), amp, wl: 9, passes: 1, step: 2 });
      t += len + skip;
    }
  }
  g.restore();
}
// Ripple lines offset from an edge, on one side of it: the edge stroked wide, its middle erased.
function offsetRing(path, d, lw, keepClip, eraseFill) {
  const g = layer();
  g.strokeStyle = '#fff'; g.lineWidth = 2 * d + lw; g.stroke(path);
  g.globalCompositeOperation = 'destination-out'; g.lineWidth = Math.max(0.1, 2 * d - lw); g.stroke(path);
  if (eraseFill) g.fill(eraseFill, 'evenodd');
  if (keepClip) { g.globalCompositeOperation = 'destination-in'; g.fill(keepClip, 'evenodd'); }
  return g;
}
function plainLines(g, lines, w, a) {   // many thin lines at once, no wobble (a city's lanes)
  g.save(); g.lineWidth = w; g.strokeStyle = INK.chalk; g.globalAlpha = a; g.beginPath();
  for (const l of lines) { const p = l.pts; g.moveTo(p[0][0], p[0][1]); for (let i = 1; i < p.length; i++) g.lineTo(p[i][0], p[i][1]); }
  g.stroke(); g.restore();
}
function dotted(g, lines, o = {}) {
  const { gap = 2.8, w = 1, a = 0.35 } = o;
  g.save(); g.setLineDash([0.1, gap]); g.lineWidth = w; g.strokeStyle = INK.chalk; g.globalAlpha = a;
  for (const l of lines) strokePts(g, wobble(resample(l.pts || l, 1.5, l.closed), 0.25, 14, srand() * 100), l.closed);
  g.restore();
}
function dashed(g, lines, o = {}) {
  const { dash = [3, 2.6], w = 0.7, a = 0.35 } = o;
  g.save(); g.setLineDash(dash); g.lineWidth = w; g.strokeStyle = INK.chalk; g.globalAlpha = a;
  for (const l of lines) strokePts(g, wobble(resample(l.pts || l, 1.5, false), 0.3, 14, srand() * 100), false);
  g.restore();
}
// A double-lined river or road: a wide chalk stroke with its middle wiped out.
function doubleLine(g, lines, outer, inner, o = {}) {
  const t = layer(), amp = o.amp ?? 0.3;
  const ws = lines.map(l => wobble(resample(l.pts || l, 1.5, false), amp, o.wl ?? 18, srand() * 100));
  t.strokeStyle = INK.chalk; t.lineWidth = outer; for (const q of ws) strokePts(t, q, false);
  t.globalCompositeOperation = 'destination-out'; t.lineWidth = inner; for (const q of ws) strokePts(t, q, false);
  put(g, t, 'source-over', o.a ?? 0.9);
  return ws;
}

// ---------------------------------------------------------------- lettering
// A label: drawn on the labels layer, and the chalk under it wiped from the ground so it reads.
function chalkText(g, text, x, y, o = {}) {
  const { size = 12, font = 'italic', family = "'IM Fell English'", spacing = 0, align = 'center', a = 0.85, rot = 0, color = INK.chalk, halo = 0, clear = false } = o;
  g.save(); g.translate(x, y); g.rotate(rot);
  g.font = `${font} ${size}px ${family}`; g.letterSpacing = `${spacing}px`; g.textAlign = align; g.textBaseline = 'middle';
  if (clear) {
    g.globalCompositeOperation = 'destination-out'; g.filter = blurPx(halo * 0.6);
    g.lineWidth = halo; g.strokeStyle = '#000'; g.strokeText(text, 0, 0); g.fillText(text, 0, 0);
  } else { g.globalAlpha = a; g.fillStyle = color; g.fillText(text, 0, 0); }
  g.restore();
}
// Letters set one by one along a polyline, centred at fraction `at` of its length.
function textOnPath(g, text, pts, o = {}) {
  const { size = 11, font = 'italic', family = "'IM Fell English'", spacing = 1, a = 0.8, offset = 0, at = 0.5, color = INK.chalk, halo = 0, clear = false } = o;
  g.save();
  g.font = `${font} ${size}px ${family}`; g.letterSpacing = '0px'; g.textBaseline = 'middle'; g.textAlign = 'center';
  const widths = [...text].map(ch => g.measureText(ch).width + spacing);
  const total = widths.reduce((s, w) => s + w, 0) - spacing;
  const seg = []; let L = 0;
  for (let i = 1; i < pts.length; i++) { const l = Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); seg.push(l); L += l; }
  let s = L * at - total / 2;
  const posAt = s0 => {
    let acc = 0;
    for (let i = 0; i < seg.length; i++) {
      if (acc + seg[i] >= s0 || i === seg.length - 1) {
        const t = clamp((s0 - acc) / (seg[i] || 1)), p = pts[i], q = pts[i + 1];
        return [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, Math.atan2(q[1] - p[1], q[0] - p[0])];
      }
      acc += seg[i];
    }
  };
  [...text].forEach((ch, i) => {
    const [x, y, ang] = posAt(s + widths[i] / 2);
    g.save(); g.translate(x, y); g.rotate(ang); g.translate(0, offset);
    if (clear) { g.globalCompositeOperation = 'destination-out'; g.filter = blurPx(halo * 0.6); g.lineWidth = halo; g.strokeStyle = '#000'; g.strokeText(ch, 0, 0); g.fillText(ch, 0, 0); }
    else { g.globalAlpha = a; g.fillStyle = color; g.fillText(ch, 0, 0); }
    g.restore();
    s += widths[i];
  });
  g.restore();
}

// ---------------------------------------------------------------- glyphs that stand up in the plan
// A round tree: a crown with a curl of chalk inside, on a short trunk when big enough to carry one.
function tree(g, x, y, r, o = {}) {
  const a = o.a ?? 0.85, trunk = o.trunk ?? (r >= 3), cy = trunk ? y - r * 1.55 : y - r;
  g.save();
  g.globalCompositeOperation = 'destination-out'; g.beginPath(); g.arc(x, cy, r + 0.45, 0, TAU); g.fill();
  if (trunk) g.fillRect(x - 0.7, cy, 1.4, y - cy);
  g.globalCompositeOperation = 'source-over';
  if (o.fill) { g.globalAlpha = o.fill; g.fillStyle = INK.chalk; g.beginPath(); g.arc(x, cy, r, 0, TAU); g.fill(); }
  g.restore();
  if (trunk) chalk(g, [[x, y], [x, cy + r * 0.9]], { w: 0.7, a, amp: 0.1, passes: 1, step: 1 });
  handCircle(g, x, cy, r, { w: o.w ?? 0.72, a, over: 0.3, passes: 1, amp: Math.min(0.35, r * 0.08) });
  if (r > 1.8 && o.curl !== false) chalk(g, spiral(x + r * 0.08, cy + r * 0.05, r * 0.58, r > 3 ? 1.5 : 1.05, srand() * TAU), { w: 0.48, a: a * 0.62, amp: 0.04, passes: 1, step: 0.8 });
}
// A grove: three or four crowns huddled, the back ones first.
function grove(g, x, y, r, o = {}) {
  const n = o.n ?? 3 + (o.k ?? 0) % 2, pts = [];
  for (let i = 0; i < n; i++) { const a = i / n * TAU + (vn(x, y + i) - 0.5) * 0.8; pts.push([x + Math.cos(a) * r * 1.05, y + Math.sin(a) * r * 0.55, r * (0.85 + vn(x + i, y) * 0.27)]); }
  pts.sort((p, q) => p[1] - q[1]).forEach(([px, py, pr]) => tree(g, px, py, pr, { ...o, trunk: false }));
}
// A tall conifer: a narrow spire on a stalk.
function spire(g, x, y, h, o = {}) {
  const a = o.a ?? 0.8, w = h * 0.28;
  chalk(g, [[x, y], [x, y - h * 0.25]], { w: 0.6, a, amp: 0.05, passes: 1, step: 1 });
  chalk(g, [[x - w, y - h * 0.22], [x, y - h], [x + w, y - h * 0.22], [x - w, y - h * 0.22]], { w: 0.65, a, amp: 0.12, passes: 1, step: 1 });
}
// A peak, tall and thin, shaded with hatching on its far side.
function peak(g, x, y, h, o = {}) {
  const a = o.a ?? 0.8, w = h * (o.wide ?? 0.36);
  g.save(); g.globalCompositeOperation = 'destination-out'; g.beginPath(); g.moveTo(x - w, y); g.lineTo(x, y - h); g.lineTo(x + w, y); g.closePath(); g.fill(); g.restore();
  chalk(g, [[x - w, y], [x, y - h], [x + w, y]], { w: o.lw ?? 0.75, a, amp: 0.15, passes: 1, step: 1 });
  const n = Math.max(1, Math.round(h / 2.6));
  for (let i = 1; i <= n; i++) {
    const t = i / (n + 1), px = x + w * t * 0.92, py = y - h * (1 - t);
    chalk(g, [[px, py + 0.4], [x + (px - x) * 0.25, y - 0.2]], { w: 0.45, a: a * 0.55, amp: 0.05, passes: 1, step: 1 });
  }
}
// A banded hill: the film's sweeping bands as a molehill.
function hill(g, x, y, w, h, o = {}) {
  const a = o.a ?? 0.7, bands = o.bands ?? 2;
  g.save(); g.globalCompositeOperation = 'destination-out'; g.beginPath(); g.ellipse(x, y, w / 2, h, 0, Math.PI, TAU); g.closePath(); g.fill(); g.restore();
  for (let b = 0; b <= bands; b++) {
    const k = 1 - b * 0.33;
    const pts = []; for (let i = 0; i <= 24; i++) { const t = Math.PI + Math.PI * i / 24; pts.push([x + Math.cos(t) * w / 2 * k + (1 - k) * w * 0.12, y + Math.sin(t) * h * k]); }
    chalk(g, pts, { w: b ? 0.45 : 0.7, a: a * (b ? 0.6 : 1), amp: 0.12, passes: 1, step: 1 });
  }
}
// A walled place: its ring seen a little from above, the wall standing in short strokes, and for the
// big ones a round tower standing in it, like the abbey's.
function walled(g, x, y, r, o = {}) {
  const a = o.a ?? 0.85, ry = r * 0.66;
  g.save(); g.globalCompositeOperation = 'destination-out'; g.beginPath(); g.ellipse(x, y, r + 1.6, ry + 1.6, 0, 0, TAU); g.fill(); g.restore();
  if (o.rub) { g.save(); g.globalAlpha = o.rub; g.fillStyle = INK.chalk; g.beginPath(); g.ellipse(x, y, r, ry, 0, 0, TAU); g.fill(); g.restore(); }
  const ring = [], a0 = srand() * TAU;
  for (let i = 0; i <= 48; i++) { const t = a0 + (TAU + 0.3) * i / 48; ring.push([x + Math.cos(t) * r, y + Math.sin(t) * ry]); }
  chalk(g, ring, { w: o.w ?? 0.85, a, amp: 0.2, wl: 12, passes: 1 });
  const n = Math.max(7, Math.round(TAU * r / 2.4)), wh = Math.min(1.6, 0.5 + r * 0.16);
  for (let i = 0; i < n; i++) {
    const t = i / n * TAU, px = x + Math.cos(t) * r, py = y + Math.sin(t) * ry;
    chalk(g, [[px, py], [px, py - wh]], { w: 0.55, a: a * 0.85, amp: 0, passes: 1, step: 1 });
  }
  if (o.tower) {
    const th = o.tower, tw = Math.max(1.3, th * 0.17), by = y + ry * 0.25, cap = tw * 2.2;
    g.save(); g.globalCompositeOperation = 'destination-out';
    g.beginPath(); g.moveTo(x - tw - 0.7, by + 0.5); g.lineTo(x - tw - 0.7, by - th); g.lineTo(x, by - th - cap - 0.8); g.lineTo(x + tw + 0.7, by - th); g.lineTo(x + tw + 0.7, by + 0.5); g.closePath(); g.fill(); g.restore();
    g.save(); g.globalAlpha = 0.16; g.fillStyle = INK.chalk; g.fillRect(x - tw, by - th, tw * 2, th); g.restore();
    chalk(g, [[x - tw, by], [x - tw, by - th], [x, by - th - cap], [x + tw, by - th], [x + tw, by]], { w: 0.65, a, amp: 0.05, passes: 1, step: 1 });
    chalk(g, [[x - tw, by - th], [x + tw, by - th]], { w: 0.5, a: a * 0.7, amp: 0, passes: 1, step: 1 });
    g.save(); g.globalAlpha = a * 0.8; g.fillStyle = INK.chalk; g.fillRect(x - 0.35, by - th * 0.62, 0.7, th * 0.16); g.restore();
  } else if (r >= 3) {
    handCircle(g, x, y, r * 0.42, { w: 0.5, a: a * 0.55, passes: 1, over: 0.2, amp: 0.05 });
  } else {
    g.save(); g.globalAlpha = a; g.fillStyle = INK.chalk; g.beginPath(); g.arc(x, y, 0.75, 0, TAU); g.fill(); g.restore();
  }
}
// A small wave: a crest that rolls over into a spiral, dotted behind (the film's night sea).
function curl(g, x, y, s, o = {}) {
  const a = o.a ?? 0.5, dir = o.dir ?? 1;
  const crest = []; for (let i = 0; i <= 10; i++) { const t = i / 10; crest.push([x - dir * s * (1.6 - t * 1.6), y + Math.sin(t * Math.PI * 0.9) * -s * 0.25]); }
  const sp = spiral(x + dir * s * 0.45, y - s * 0.05, s * 0.55, 1.3, Math.PI, -dir).reverse();
  chalk(g, crest.concat(sp.slice(1)), { w: o.w ?? 0.6, a, amp: 0.08, passes: 1, step: 1 });
  if (o.dots) for (let i = 0; i < 3; i++) { g.save(); g.globalAlpha = a * 0.8; g.fillStyle = INK.chalk; g.beginPath(); g.arc(x - dir * s * (1.9 + i * 0.7), y + s * 0.15, 0.45, 0, TAU); g.fill(); g.restore(); }
}
// A rolling wave of the night sea: a long crest that rises, breaks and curls into a spiral, with a
// line of dots riding the water below it.
function seaWave(g, x, y, len, o = {}) {
  const a = o.a ?? 0.5, dir = o.dir ?? 1, hgt = len * 0.16, crest = [];
  for (let i = 0; i <= 24; i++) {
    const t = i / 24, rise = Math.pow(Math.sin(t * Math.PI * 0.62), 1.6);
    crest.push([x - dir * len * (1 - t), y - hgt * rise]);
  }
  const tip = crest[crest.length - 1], r = hgt * 0.95;
  const sp = spiral(tip[0] + dir * r * 0.15, tip[1] + r * 0.9, r, 1.35, -Math.PI / 2, dir).slice(1);
  chalk(g, crest.concat(sp), { w: o.w ?? 0.7, a, amp: 0.1, wl: 20, passes: 1, step: 1 });
  chalk(g, crest.slice(4, 21).map(([px, py]) => [px - dir * 0.6, py + hgt * 0.35]), { w: 0.45, a: a * 0.5, amp: 0.1, passes: 1, step: 1 });
  g.save(); g.fillStyle = INK.chalk;
  for (let i = 0; i < 6; i++) { const t = (i + 0.5) / 6; g.globalAlpha = a * 0.75; g.beginPath(); g.arc(x - dir * len * (1 - t * 0.85), y + hgt * 0.55 + Math.sin(i * 1.7) * 0.3, 0.5, 0, TAU); g.fill(); }
  g.restore();
}
// A tower in elevation, standing on (x, y): a shaft with window lines and its own top.
function tower(g, x, y, w, h, top = 'flat', a = 0.9) {
  const x0 = x - w / 2, x1 = x + w / 2, yt = y - h;
  const sil = [[x0, y], [x0, yt]];
  if (top === 'spire') sil.push([x0 + w * 0.18, yt], [x0 + w * 0.18, yt - h * 0.07], [x0 + w * 0.32, yt - h * 0.07], [x, yt - h * 0.2], [x1 - w * 0.32, yt - h * 0.07], [x1 - w * 0.18, yt - h * 0.07], [x1 - w * 0.18, yt]);
  else if (top === 'dome') sil.push(...circlePts(x, yt, w / 2, 14, Math.PI, TAU).slice(1, -1), [x + 0.5, yt - w / 2], [x, yt - w / 2 - 2.6], [x - 0.5, yt - w / 2]);
  else if (top === 'step') sil.push([x0 + w * 0.2, yt], [x0 + w * 0.2, yt - h * 0.06], [x1 - w * 0.2, yt - h * 0.06], [x1 - w * 0.2, yt]);
  else if (top === 'gable') sil.push([x, yt - w * 0.9]);
  sil.push([x1, yt], [x1, y]);
  g.save(); g.globalCompositeOperation = 'destination-out'; g.beginPath(); sil.forEach((p, i) => i ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1])); g.closePath();
  g.lineWidth = 1.6; g.stroke(); g.fill(); g.restore();
  g.save(); g.globalAlpha = 0.12; g.fillStyle = INK.chalk; g.beginPath(); sil.forEach((p, i) => i ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1])); g.closePath(); g.fill(); g.restore();
  chalk(g, sil, { w: 0.7, a, amp: 0.08, passes: 1, step: 1 });
  const cols = Math.max(1, Math.round(w / 2.2));
  for (let c = 1; c <= cols; c++) {
    const cx = x0 + w * c / (cols + 1);
    chalk(g, [[cx, y - 1.5], [cx, yt + 2]], { w: 0.4, a: a * 0.45, amp: 0.05, passes: 1, step: 1.5 });
  }
}
// A house facing us whichever way its street runs: walls, a roof, a door, a window.
function house(g, x, y, w, v, o = {}) {
  const h = w * (v < 0.22 ? 0.66 : 0.5), rh = w * (0.34 + ((v * 7.3) % 1) * 0.16), hip = ((v * 13.1) % 1) > 0.68;
  const x0 = x - w / 2, x1 = x + w / 2, yt = y - h, yr = yt - rh, oh = 0.55, a = o.a ?? 1;
  const roof = hip ? [[x0 - oh, yt], [x0 + w * 0.26, yr], [x1 - w * 0.26, yr], [x1 + oh, yt]] : [[x0 - oh, yt], [x, yr], [x1 + oh, yt]];
  const sil = [[x0, y], ...roof, [x1, y]];
  g.save(); g.globalCompositeOperation = 'destination-out'; g.beginPath(); sil.forEach((p, i) => i ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1])); g.closePath();
  g.lineWidth = 1.5; g.stroke(); g.fill(); g.restore();
  g.save(); g.globalAlpha = 0.09; g.fillStyle = INK.chalk; g.beginPath(); sil.forEach((p, i) => i ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1])); g.closePath(); g.fill(); g.restore();
  chalk(g, [[x0, y], [x0, yt], [x1, yt], [x1, y], [x0, y]], { w: 0.6, a: 0.74 * a, amp: 0.06, passes: 1, step: 1 });
  chalk(g, roof, { w: 0.65, a: 0.82 * a, amp: 0.06, passes: 1, step: 1 });
  const dx = ((v * 3.7) % 1) < 0.5 ? x0 + w * 0.28 : x1 - w * 0.28, wx = dx < x ? x1 - w * 0.27 : x0 + w * 0.27;
  g.save(); g.fillStyle = INK.chalk; g.globalAlpha = 0.75 * a;
  g.fillRect(dx - 0.45, y - h * 0.58, 0.9, h * 0.58);
  g.fillRect(wx - 0.55, yt + h * 0.28, 1.1, 1.0);
  if (((v * 5.9) % 1) > 0.6) g.fillRect(x1 - w * 0.3, yr + rh * 0.25, 0.9, rh * 0.55);
  g.restore();
}
// A shopfront row in elevation: a flat parapet, a cornice line, upper windows in rows, and a band of
// shop windows at the street.
function shopfront(g, x, y, w, h, v, o = {}) {
  const a = o.a ?? 1, x0 = x - w / 2, x1 = x + w / 2, yt = y - h;
  const sil = [[x0, y], [x0, yt], [x1, yt], [x1, y]];
  g.save(); g.globalCompositeOperation = 'destination-out'; g.fillRect(x0 - 0.6, yt - 0.9, w + 1.2, h + 0.9); g.restore();
  g.save(); g.globalAlpha = 0.1; g.fillStyle = INK.chalk; g.fillRect(x0, yt, w, h); g.restore();
  chalk(g, sil.concat([[x0, y]]), { w: 0.6, a: 0.78 * a, amp: 0.05, passes: 1, step: 1 });
  chalk(g, [[x0 - 0.5, yt + 1.1], [x1 + 0.5, yt + 1.1]], { w: 0.45, a: 0.6 * a, amp: 0, passes: 1, step: 1 });
  const cols = Math.max(2, Math.round(w / 2.6)), rows = Math.max(1, Math.round((h - 4) / 3.4));
  g.save(); g.fillStyle = INK.chalk; g.globalAlpha = 0.62 * a;
  for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) {
    const wx = x0 + w * (c + 0.5) / cols, wy = yt + 2.6 + r * 3.4;
    if (wy > y - 3.6) continue;
    g.fillRect(wx - 0.45, wy, 0.9, 1.4);
  }
  g.globalAlpha = 0.5 * a; g.fillRect(x0 + 0.8, y - 2.2, w - 1.6, 0.5);
  g.restore();
}
