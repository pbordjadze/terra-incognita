// Aisling's Forest: the painting engine the five levels share.
//
// A plate is 1638×3545 px. Shapes are built in design units of 3 plate px (a 546×1182-unit plate; at rest the
// phone shows its middle 390×844 units), as Path2D, and painted into one canvas: washes composited per pixel so
// they mottle, granulate and pool at their edges like watercolour, pen hatching clipped inside shapes, inked
// edges drawn twice, leaves and crowns filled from small tiles of mottled colour.
//
// The view. The exporter makes a pass plate by magnifying this page with a CSS transform on <html>. A canvas
// magnified that way would only be blown up, so the page watches for the transform and draws the layer again
// through it (setView): the same picture, at the magnified resolution. Everything that decides what is drawn
// (every random draw, every inside test, every noise value that steers a choice) is taken in plate space, so a
// pass plate draws the same leaves, trees and strokes as the plate it magnifies, only sharper. Only the paper's
// grain is in screen pixels.
'use strict';
const PW = 1638, PH = 3545, S = 3, W = PW / S, H = PH / S;

// ---------- the view: device px = (plate px - VX) * VS; plate px = units * S
let VS = 1, VX = 0, VY = 0, K = S, TX = 0, TY = 0;
function setView(s, x0, y0) { VS = s; VX = x0; VY = y0; K = S * s; TX = -x0 * s; TY = -y0 * s; viewFields(); }
const toDev = (u, v) => [u * K + TX, v * K + TY];

// ---------- seeded randomness and noise
function mulberry32(a) {
  return function () { a |= 0; a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; };
}
let rnd = mulberry32(1);
const seedRng = n => { rnd = mulberry32(n); };
const R = (a, b) => a + (b - a) * rnd();
const pick = arr => arr[Math.floor(rnd() * arr.length)];
function hash2(i, j, s) {
  let h = Math.imul(i | 0, 374761393) ^ Math.imul(j | 0, 668265263) ^ Math.imul((s | 0) + 1, 1442695041);
  h = Math.imul(h ^ (h >>> 13), 1274126177); h ^= h >>> 16; return (h >>> 0) / 4294967296;
}
function n2(x, y, s = 0) { // smooth value noise for vector work, 0..1
  const i = Math.floor(x), j = Math.floor(y); let fx = x - i, fy = y - j;
  fx = fx * fx * (3 - 2 * fx); fy = fy * fy * (3 - 2 * fy);
  const a = hash2(i, j, s), b = hash2(i + 1, j, s), c = hash2(i, j + 1, s), d = hash2(i + 1, j + 1, s);
  return a + (b - a) * fx + (c - a) * fy + (a - b - c + d) * fx * fy;
}
const fbm2 = (x, y, s = 0) => 0.55 * n2(x, y, s) + 0.3 * n2(x * 2.1, y * 2.1, s + 3) + 0.15 * n2(x * 4.3, y * 4.3, s + 5);

// A value-noise field, w×h samples, a smoothstep-interpolated lattice of `cell` samples.
function field(cell, seed, w, h, wrap = false) {
  const gw = wrap ? Math.round(w / cell) : Math.ceil(w / cell) + 2, gh = wrap ? Math.round(h / cell) : Math.ceil(h / cell) + 2;
  const g = new Float32Array(gw * gh), r = mulberry32(seed);
  for (let i = 0; i < g.length; i++) g[i] = r();
  const cx = wrap ? gw / w : 1 / cell, cy = wrap ? gh / h : 1 / cell;
  const out = new Float32Array(w * h), ix0 = new Int32Array(w), ix1 = new Int32Array(w), fx = new Float32Array(w);
  for (let x = 0; x < w; x++) { const u = x * cx, i = Math.floor(u); let f = u - i; fx[x] = f * f * (3 - 2 * f); ix0[x] = wrap ? i % gw : i; ix1[x] = wrap ? (i + 1) % gw : i + 1; }
  let o = 0;
  for (let y = 0; y < h; y++) {
    const v = y * cy, j = Math.floor(v); let fy = v - j; fy = fy * fy * (3 - 2 * fy);
    const r0 = (wrap ? j % gh : j) * gw, r1 = (wrap ? (j + 1) % gh : j + 1) * gw;
    for (let x = 0; x < w; x++) {
      const a = g[r0 + ix0[x]], b = g[r0 + ix1[x]], c = g[r1 + ix0[x]], d = g[r1 + ix1[x]];
      const t = a + (b - a) * fx[x], u = c + (d - c) * fx[x]; out[o++] = t + (u - t) * fy;
    }
  }
  return out;
}
function mixFields(parts) {
  const out = new Float32Array(parts[0][0].length);
  for (const [f, w] of parts) for (let i = 0; i < out.length; i++) out[i] += f[i] * w;
  return out;
}
// Plate-space fields (one sample per plate px): the big mottling, the medium blooms.
const FB1p = mixFields([[field(170, 11, PW, PH), 0.55], [field(64, 12, PW, PH), 0.3], [field(24, 13, PW, PH), 0.15]]);
const FB2p = mixFields([[field(120, 21, PW, PH), 0.5], [field(48, 22, PW, PH), 0.35], [field(17, 23, PW, PH), 0.15]]);
const FMp = mixFields([[field(20, 31, PW, PH), 0.6], [field(8, 32, PW, PH), 0.4]]);
// The paper's grain, in device px whatever the view, soft enough to keep JPEGs small.
const FG = mixFields([[field(3.2, 41, PW, PH), 0.55], [field(7, 42, PW, PH), 0.45]]);
let FB1 = FB1p, FB2 = FB2p, FM = FMp; // the plate fields as this view sees them, one sample per device px
function resample(F) {
  if (VS === 1 && VX === 0 && VY === 0) return F;
  const out = new Float32Array(PW * PH);
  for (let j = 0; j < PH; j++) {
    const Y = Math.min(PH - 1.001, Math.max(0, VY + (j + 0.5) / VS - 0.5)), y0 = Math.floor(Y), fy = Y - y0;
    for (let i = 0; i < PW; i++) {
      const X = Math.min(PW - 1.001, Math.max(0, VX + (i + 0.5) / VS - 0.5)), x0 = Math.floor(X), fx = X - x0, o = y0 * PW + x0;
      const a = F[o] + (F[o + 1] - F[o]) * fx, b = F[o + PW] + (F[o + PW + 1] - F[o + PW]) * fx;
      out[j * PW + i] = a + (b - a) * fy;
    }
  }
  return out;
}
function viewFields() { FB1 = resample(FB1p); FB2 = resample(FB2p); FM = resample(FMp); }
// A plate field's value at a point (units), the same in every view: for choices.
const fieldP = (F, x, y) => { const X = Math.round(x * S), Y = Math.round(y * S); return X >= 0 && Y >= 0 && X < PW && Y < PH ? F[Y * PW + X] : 0.5; };

// ---------- colour
const hex = h => [parseInt(h.slice(1, 3), 16) / 255, parseInt(h.slice(3, 5), 16) / 255, parseInt(h.slice(5, 7), 16) / 255];
const rgba = (h, a) => { const c = hex(h); return `rgba(${c[0] * 255 | 0},${c[1] * 255 | 0},${c[2] * 255 | 0},${a})`; };
const lerp3 = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
const smooth = (a, b, x) => { const t = Math.max(0, Math.min(1, (x - a) / (b - a))); return t * t * (3 - 2 * t); };
const P = {
  paper: '#f0e5c4', ink: '#2a1a0d', umber: '#4b2f17', sepia: '#6b4423',
  rust: '#a9461b', rust2: '#c4602a', ochre: '#d29a34', ochre2: '#e4b952', straw: '#ecd58c',
  chart: '#c5cf45', chart2: '#dde27c', lime: '#b5c93a', olive: '#7f8a2a', olive2: '#5d6620', moss: '#90a234', deep: '#3a3f15',
  agaric: '#cf2f1c', fungus: '#e5764a', fungus2: '#f2a070', fox: '#9a4cae', fox2: '#c88bd4', white: '#fbf7ea',
  water: '#e4e8c8', water2: '#b3c9a6', water3: '#6e9387', jade: '#5f8a7a', lakeInk: '#3b6457',
};

// ---------- canvases (all PW×PH device px)
let cv, ctx;
function scratch() { const c = document.createElement('canvas'); c.width = PW; c.height = PH; return [c, c.getContext('2d', { willReadFrequently: true })]; }
let MCV, MX, BCV, BX, HCV, HX, RCV, RX, QCV, QX;
function initCanvases() {
  cv = document.createElement('canvas'); cv.width = PW; cv.height = PH; cv.id = 'plate';
  Object.assign(cv.style, { position: 'absolute', left: '0px', top: '0px', width: PW + 'px', height: PH + 'px', display: 'block' });
  document.body.appendChild(cv);
  ctx = cv.getContext('2d', { willReadFrequently: true });
  [MCV, MX] = scratch(); [BCV, BX] = scratch(); [HCV, HX] = scratch(); [RCV, RX] = scratch(); [QCV, QX] = scratch();
}
function base(c) { c.setTransform(K, 0, 0, K, TX, TY); }
function reset(c) { c.setTransform(1, 0, 0, 1, 0, 0); c.globalCompositeOperation = 'source-over'; c.globalAlpha = 1; c.filter = 'none'; c.clearRect(0, 0, PW, PH); base(c); }
function css(c) { base(c); c.globalCompositeOperation = 'source-over'; c.globalAlpha = 1; c.filter = 'none'; }
const blurPx = u => `blur(${Math.max(0.01, u * K)}px)`;

// A mask of a shape in plate space, for inside tests that must not depend on the view.
function maskOf(shape) {
  QX.setTransform(1, 0, 0, 1, 0, 0); QX.globalCompositeOperation = 'source-over'; QX.globalAlpha = 1; QX.clearRect(0, 0, PW, PH);
  QX.setTransform(S, 0, 0, S, 0, 0); QX.fillStyle = '#fff'; QX.strokeStyle = '#fff'; fillShape(QX, shape);
  const d = QX.getImageData(0, 0, PW, PH).data, a = new Uint8Array(PW * PH);
  for (let i = 0; i < a.length; i++) a[i] = d[i * 4 + 3] > 128 ? 1 : 0;
  return (x, y) => { const X = Math.round(x * S), Y = Math.round(y * S); return X >= 0 && Y >= 0 && X < PW && Y < PH && a[Y * PW + X] === 1; };
}

// ---------- paths: a recorder for d3.geoPath that wobbles and densifies, so edges look drawn by hand
class Rec {
  constructor(o = {}) { this.s = []; this.amp = o.amp || 0; this.sc = o.sc || 18; this.seed = o.seed || 0; this.step = o.step || Math.max(0.8, this.sc / 6); this.last = null; this.start = null; this.r = o.r || 2; }
  d(x, y) {
    if (!this.amp) return [x, y];
    return [x + this.amp * (n2(x / this.sc, y / this.sc, this.seed) - 0.5) * 2, y + this.amp * (n2(x / this.sc + 31.7, y / this.sc + 11.3, this.seed + 7) - 0.5) * 2];
  }
  moveTo(x, y) { const p = this.d(x, y); this.s.push('M', p[0].toFixed(3), ',', p[1].toFixed(3)); this.last = [x, y]; this.start = [x, y]; }
  lineTo(x, y) {
    const near = (px, py) => px > -80 && py > -80 && px < W + 80 && py < H + 80;
    if (this.amp && this.last && (near(x, y) || near(this.last[0], this.last[1]))) {
      const dx = x - this.last[0], dy = y - this.last[1], n = Math.min(4000, Math.floor(Math.hypot(dx, dy) / this.step));
      for (let k = 1; k <= n; k++) { const t = k / (n + 1), p = this.d(this.last[0] + dx * t, this.last[1] + dy * t); this.s.push('L', p[0].toFixed(3), ',', p[1].toFixed(3)); }
    }
    const p = this.d(x, y); this.s.push('L', p[0].toFixed(3), ',', p[1].toFixed(3)); this.last = [x, y];
  }
  closePath() { if (this.start) this.lineTo(this.start[0], this.start[1]); this.s.push('Z'); }
  arc(x, y, r) { for (let k = 0; k <= 12; k++) { const a = k / 12 * Math.PI * 2; (k ? this.lineTo : this.moveTo).call(this, x + r * Math.cos(a), y + r * Math.sin(a)); } }
  path() { return new Path2D(this.s.join('')); }
}
function geoP(proj, obj, o = {}) { const r = new Rec(o); d3.geoPath(proj, r).pointRadius(r.r)(obj); return r.path(); }
function polyP(pts, o = {}, close = true) {
  const r = new Rec(o); pts.forEach((p, i) => i ? r.lineTo(p[0], p[1]) : r.moveTo(p[0], p[1])); if (close) r.closePath(); return r.path();
}
function smoothPts(pts, iter = 2, closed = false) { // Chaikin
  let p = pts;
  for (let k = 0; k < iter; k++) {
    const q = closed ? [] : [p[0]], n = closed ? p.length : p.length - 1;
    for (let i = 0; i < n; i++) {
      const a = p[i], b = p[(i + 1) % p.length];
      q.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25], [a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
    }
    if (!closed) q.push(p[p.length - 1]);
    p = q;
  }
  return p;
}
function fillShape(c, shape) { if (typeof shape === 'function') shape(c); else c.fill(shape, 'evenodd'); }
function linePath(pts) { const p = new Path2D(); pts.forEach((q, i) => i ? p.lineTo(q[0], q[1]) : p.moveTo(q[0], q[1])); return p; }
function offsetPts(pts, d) {
  return pts.map((p, i) => {
    const a = pts[Math.max(0, i - 1)], b = pts[Math.min(pts.length - 1, i + 1)], L = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1;
    return [p[0] - (b[1] - a[1]) / L * d, p[1] + (b[0] - a[0]) / L * d];
  });
}
const offsetPath = (pts, d) => linePath(offsetPts(pts, d));
function bboxOf(pts, pad = 4) { const a = [1e9, 1e9, -1e9, -1e9]; pts.forEach(([x, y]) => { a[0] = Math.min(a[0], x); a[1] = Math.min(a[1], y); a[2] = Math.max(a[2], x); a[3] = Math.max(a[3], y); }); return [a[0] - pad, a[1] - pad, a[2] + pad, a[3] + pad]; }

// ---------- watercolour wash, composited per pixel (multiply: transparent pigment on paper)
// o: c (1–3 hex colours), op, mottle, edge (pooling at the wet edge), blur (its width, units), gran, bloom,
//    fb (which big field), cut (shapes taken out of the mask), over (opaque body colour instead of multiply),
//    cbuf {w,h,data} (a colour per unit instead of c), bbox [x0,y0,x1,y1] in units.
function wash(shape, o = {}) {
  const op = o.op ?? 0.8, mottle = o.mottle ?? 0.35, edgeK = o.edge ?? 0.8, blur = o.blur ?? 3, gran = o.gran ?? 0.3, bloom = o.bloom ?? 0.2;
  const fb = o.fb ?? 0.5, C = (o.c || [P.ochre]).map(hex), c2amt = o.c2amt ?? 0.6, over = !!o.over;
  reset(MX); MX.fillStyle = '#fff'; MX.strokeStyle = '#fff'; fillShape(MX, shape);
  if (o.cut) for (const s of [].concat(o.cut)) {
    if (typeof s === 'function') { // drawn on its own sheet first, so its own compositing can't touch the mask
      reset(BX); BX.fillStyle = '#fff'; BX.strokeStyle = '#fff'; s(BX); css(BX);
      MX.setTransform(1, 0, 0, 1, 0, 0); MX.globalCompositeOperation = 'destination-out'; MX.drawImage(BCV, 0, 0); css(MX);
    } else { MX.globalCompositeOperation = 'destination-out'; MX.fill(s, 'evenodd'); MX.globalCompositeOperation = 'source-over'; }
  }
  reset(BX); BX.setTransform(1, 0, 0, 1, 0, 0); BX.filter = blurPx(blur); BX.drawImage(MCV, 0, 0); BX.filter = 'none';
  const bb = o.bbox || [-10, -10, W + 10, H + 10], [dx0, dy0] = toDev(bb[0], bb[1]), [dx1, dy1] = toDev(bb[2], bb[3]);
  const x0 = Math.max(0, Math.floor(dx0)), y0 = Math.max(0, Math.floor(dy0)), x1 = Math.min(PW, Math.ceil(dx1)), y1 = Math.min(PH, Math.ceil(dy1));
  const bw = x1 - x0, bh = y1 - y0; if (bw <= 0 || bh <= 0) return;
  const m = MX.getImageData(x0, y0, bw, bh).data, bl = BX.getImageData(x0, y0, bw, bh).data;
  const img = ctx.getImageData(x0, y0, bw, bh), d = img.data;
  const c0 = C[0], c1 = C[1] || C[0], c2 = C[2], cb = o.cbuf, cvar = o.cvar ?? 0.18;
  for (let y = 0; y < bh; y++) {
    const gy = y + y0, vy = Math.min(cb ? cb.h - 1.001 : 0, Math.max(0, (gy + 0.5 - TY) / K - 0.5)), cy0 = vy | 0, fy = vy - cy0;
    for (let x = 0; x < bw; x++) {
      const k = (y * bw + x) * 4, a8 = m[k + 3]; if (!a8) continue;
      const a = a8 / 255, b = bl[k + 3] / 255, i = gy * PW + x + x0;
      let e = a - b; e = e < 0 ? 0 : e * 2; e *= e;
      const nb = FB1[i] * fb + FB2[i] * (1 - fb), md = FM[i], g = FG[i];
      const bd = md - 0.6;
      let dens = op * a * (1 + mottle * (nb - 0.5) * 2.2 + edgeK * e + gran * (g - 0.5) * 2 + bloom * Math.exp(-bd * bd * 900));
      dens = dens < 0 ? 0 : dens > 1 ? 1 : dens;
      let cr, cg, cbv;
      if (cb) { // the colour buffer, read bilinearly so it stays smooth magnified
        const vx = Math.min(cb.w - 1.001, Math.max(0, (x + x0 + 0.5 - TX) / K - 0.5)), cx0 = vx | 0, fx = vx - cx0;
        const o00 = (cy0 * cb.w + cx0) * 3, o10 = o00 + 3, o01 = o00 + cb.w * 3, o11 = o01 + 3, D = cb.data;
        const t = (nb - 0.5) * cvar, w00 = (1 - fx) * (1 - fy), w10 = fx * (1 - fy), w01 = (1 - fx) * fy, w11 = fx * fy;
        cr = (D[o00] * w00 + D[o10] * w10 + D[o01] * w01 + D[o11] * w11) * (1 + t);
        cg = (D[o00 + 1] * w00 + D[o10 + 1] * w10 + D[o01 + 1] * w01 + D[o11 + 1] * w11) * (1 + t);
        cbv = (D[o00 + 2] * w00 + D[o10 + 2] * w10 + D[o01 + 2] * w01 + D[o11 + 2] * w11) * (1 + t * 0.6);
      } else {
        const t = smooth(0.32, 0.68, nb);
        cr = c0[0] + (c1[0] - c0[0]) * t; cg = c0[1] + (c1[1] - c0[1]) * t; cbv = c0[2] + (c1[2] - c0[2]) * t;
        if (c2) { const t2 = smooth(0.5, 0.75, md) * c2amt; cr += (c2[0] - cr) * t2; cg += (c2[1] - cg) * t2; cbv += (c2[2] - cbv) * t2; }
      }
      if (over) {
        const a0 = d[k + 3] / 255;
        if (a0 >= 1) { d[k] = d[k] * (1 - dens) + cr * 255 * dens; d[k + 1] = d[k + 1] * (1 - dens) + cg * 255 * dens; d[k + 2] = d[k + 2] * (1 - dens) + cbv * 255 * dens; }
        else { // on a transparent sheet (a sprite): pigment over whatever is already there
          const a1 = a0 + dens * (1 - a0), w0 = a0 * (1 - dens) / (a1 || 1), w1 = dens / (a1 || 1);
          d[k] = d[k] * w0 + cr * 255 * w1; d[k + 1] = d[k + 1] * w0 + cg * 255 * w1; d[k + 2] = d[k + 2] * w0 + cbv * 255 * w1; d[k + 3] = a1 * 255;
        }
      } else {
        d[k] *= 1 - dens * (1 - cr); d[k + 1] *= 1 - dens * (1 - cg); d[k + 2] *= 1 - dens * (1 - cbv);
      }
    }
  }
  ctx.putImageData(img, x0, y0);
}

// ---------- pen hatching: families of short, slightly bowed strokes, kept inside a region
// o: ang (deg), sp (spacing), len [min,max], gap [min,max], w, col, alpha, jit, bbox, feather, comp, bow
function hatch(region, o = {}) {
  reset(RX); RX.fillStyle = '#fff'; RX.strokeStyle = '#fff'; fillShape(RX, region);
  if (o.feather) { reset(BX); BX.setTransform(1, 0, 0, 1, 0, 0); BX.filter = blurPx(o.feather); BX.drawImage(RCV, 0, 0); BX.filter = 'none'; RX.setTransform(1, 0, 0, 1, 0, 0); RX.clearRect(0, 0, PW, PH); RX.drawImage(BCV, 0, 0); }
  reset(HX);
  const bb = o.bbox || [0, 0, W, H], cx = (bb[0] + bb[2]) / 2, cy = (bb[1] + bb[3]) / 2, rad = Math.hypot(bb[2] - bb[0], bb[3] - bb[1]) / 2 + 4;
  const a = (o.ang ?? 35) * Math.PI / 180, ux = Math.cos(a), uy = Math.sin(a), vx = -uy, vy = ux;
  const sp = o.sp ?? 2.4, len = o.len || [3, 9], gap = o.gap || [0.8, 3.5], w = o.w ?? 0.42, jit = o.jit ?? 0.35, bow = o.bow ?? 0.5;
  HX.strokeStyle = o.col || P.ink; HX.lineCap = 'round';
  for (let t = -rad; t < rad; t += sp * R(0.75, 1.25)) {
    let s = -rad + R(0, len[1]);
    while (s < rad) {
      const L = R(len[0], len[1]), j = R(-jit, jit);
      const px = cx + ux * s + vx * (t + j), py = cy + uy * s + vy * (t + j);
      const qx = px + ux * L + vx * R(-jit, jit) * 0.8, qy = py + uy * L + vy * R(-jit, jit) * 0.8;
      const mx = (px + qx) / 2, my = (py + qy) / 2;
      const ga = (o.alpha ?? 0.7) * R(0.55, 1), lw = w * R(0.65, 1.3), b1 = R(-bow, bow), b2 = R(-bow, bow);
      if (mx > bb[0] - 2 && mx < bb[2] + 2 && my > bb[1] - 2 && my < bb[3] + 2) {
        HX.globalAlpha = ga; HX.lineWidth = lw;
        HX.beginPath(); HX.moveTo(px, py); HX.quadraticCurveTo(mx + vx * b1, my + vy * b2, qx, qy); HX.stroke();
      }
      s += L + R(gap[0], gap[1]);
    }
  }
  HX.globalAlpha = 1; HX.setTransform(1, 0, 0, 1, 0, 0); HX.globalCompositeOperation = 'destination-in'; HX.drawImage(RCV, 0, 0);
  ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.globalCompositeOperation = o.comp || 'multiply'; ctx.globalAlpha = o.op ?? 1; ctx.drawImage(HCV, 0, 0); css(ctx);
}
// The band of a shape on its shadow side: the shape minus itself shifted toward the light.
const shadowBand = (shape, dx, dy) => c => { fillShape(c, shape); c.globalCompositeOperation = 'destination-out'; c.translate(dx, dy); fillShape(c, shape); c.translate(-dx, -dy); c.globalCompositeOperation = 'source-over'; };
// The inside rim of a shape, `w` wide.
const rimBand = (shape, w) => c => { c.save(); fillShape(c, shape); c.globalCompositeOperation = 'destination-in'; c.lineWidth = w * 2; c.lineJoin = 'round'; c.stroke(shape); c.restore(); };

// ---------- ink: a pen line drawn twice, slightly apart, so it reads as scratchy and of varying weight
function ink(path, o = {}) {
  const w = o.w ?? 0.7, col = o.col || P.ink, al = o.alpha ?? 0.85;
  const dash2 = o.dash2 || [R(6, 14), R(1, 4), R(3, 9), R(2, 6)], jx = R(-0.35, 0.35), jy = R(-0.35, 0.35);
  ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round'; ctx.strokeStyle = col;
  if (o.comp) ctx.globalCompositeOperation = o.comp;
  ctx.globalAlpha = al; ctx.lineWidth = w; if (o.dash) ctx.setLineDash(o.dash); ctx.stroke(path);
  if (o.passes !== 1) { ctx.setLineDash(dash2); ctx.globalAlpha = al * 0.55; ctx.lineWidth = w * 0.7; ctx.translate(jx, jy); ctx.stroke(path); }
  ctx.restore();
}
// A line drawn onto a sheet, kept inside or outside a region, then laid on the canvas.
function strokeMasked(drawFn, keep, o = {}) {
  reset(RX); RX.strokeStyle = '#fff'; RX.fillStyle = '#fff'; drawFn(RX);
  if (keep) { reset(BX); BX.fillStyle = '#fff'; BX.strokeStyle = '#fff'; keep(BX); RX.setTransform(1, 0, 0, 1, 0, 0); RX.globalCompositeOperation = o.out ? 'destination-out' : 'destination-in'; RX.drawImage(BCV, 0, 0); }
  RX.setTransform(1, 0, 0, 1, 0, 0); RX.globalCompositeOperation = 'source-in'; RX.fillStyle = o.col || P.ink; RX.fillRect(0, 0, PW, PH);
  ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.globalCompositeOperation = o.comp || 'source-over'; ctx.globalAlpha = o.alpha ?? 1; ctx.drawImage(RCV, 0, 0); css(ctx);
}

// ---------- paint tiles: small seamless squares of mottled colour, for leaves, trees, hills and roofs
const TILE = 360;
function makeTile(cols, o = {}) {
  const c = document.createElement('canvas'); c.width = c.height = TILE; const x = c.getContext('2d');
  const seed = Math.floor(rnd() * 1e6);
  const f1 = field(TILE / 4, seed, TILE, TILE, true), f2 = field(TILE / 12, seed + 1, TILE, TILE, true), f3 = field(TILE / 60, seed + 2, TILE, TILE, true), f4 = field(TILE / 120, seed + 3, TILE, TILE, true);
  const img = x.createImageData(TILE, TILE), d = img.data, C = cols.map(hex);
  for (let i = 0; i < TILE * TILE; i++) {
    const nb = f1[i] * 0.6 + f2[i] * 0.4, md = f3[i], g = f4[i];
    let col = lerp3(C[0], C[1], smooth(0.3, 0.7, nb));
    if (C[2]) col = lerp3(col, C[2], smooth(0.58, 0.8, md) * (o.c2 ?? 0.7));
    const bd = md - 0.55, k = 1 - (o.bloom ?? 0.12) * Math.exp(-bd * bd * 700) - (g - 0.5) * (o.gran ?? 0.07);
    d[i * 4] = col[0] * 255 * k; d[i * 4 + 1] = col[1] * 255 * k; d[i * 4 + 2] = col[2] * 255 * k; d[i * 4 + 3] = 255;
  }
  x.putImageData(img, 0, 0);
  if (o.blot) { // soft darker blooms, the mottling of a wet wash
    x.globalCompositeOperation = 'multiply';
    for (let n = 0; n < o.blot; n++) {
      const px = R(0, TILE), py = R(0, TILE), r = R(8, 26);
      for (const [dx, dy] of [[0, 0], [TILE, 0], [-TILE, 0], [0, TILE], [0, -TILE]]) {
        const g = x.createRadialGradient(px + dx, py + dy, r * 0.2, px + dx, py + dy, r); g.addColorStop(0, rgba(o.blotCol || '#8a7a30', 0.0)); g.addColorStop(0.75, rgba(o.blotCol || '#8a7a30', 0.32)); g.addColorStop(1, rgba(o.blotCol || '#8a7a30', 0));
        x.fillStyle = g; x.beginPath(); x.arc(px + dx, py + dy, r, 0, 7); x.fill();
      }
    }
    x.globalCompositeOperation = 'source-over';
  }
  if (o.speck) { // dark freckles, like the film's leaves
    for (let n = 0; n < o.speck; n++) {
      x.fillStyle = rgba(o.speckCol || P.deep, R(0.25, 0.7)); const px = R(0, TILE), py = R(0, TILE), r = R(0.6, 2.4) * (rnd() < 0.1 ? 2 : 1);
      for (const [dx, dy] of [[0, 0], [TILE, 0], [-TILE, 0], [0, TILE], [0, -TILE]]) { x.beginPath(); x.arc(px + dx, py + dy, r, 0, 7); x.fill(); }
    }
  }
  return c;
}
function pattern(tile, ox = R(0, TILE), oy = R(0, TILE), rot = 0) {
  const p = ctx.createPattern(tile, 'repeat');
  p.setTransform(new DOMMatrix().rotate(rot).translate(ox, oy).scale(1 / S));
  return p;
}
const tiles = {};
function initTiles() {
  seedRng(4242);
  tiles.ochre = makeTile([P.ochre2, P.ochre, '#b9842c'], { speck: 70, blot: 14 });
  tiles.chart = makeTile(['#d3d266', '#b8bb48', '#8c8d32'], { speck: 60, blot: 14 });
  tiles.khaki = makeTile(['#c2b65a', '#a49b40', '#76712a'], { speck: 70, blot: 16 });
  tiles.olive = makeTile(['#9aa040', '#7c8430', '#5d6322'], { speck: 70, blot: 14, blotCol: '#4a4a18' });
  tiles.rust = makeTile([P.rust2, '#b05222', P.ochre], { speck: 50, speckCol: P.umber, blot: 10, blotCol: '#7a3012' });
  tiles.straw = makeTile(['#efd98e', '#e2bf5c', '#c9b04c'], { speck: 45, blot: 12 });
  tiles.deep = makeTile([P.olive2, P.deep, P.moss], { speck: 20, speckCol: P.ink });
  tiles.lime = makeTile([P.chart2, P.lime, P.moss], { speck: 40, blot: 10 });
  tiles.roof = makeTile(['#c79a6a', '#b4865a', '#a87a50'], { speck: 8, speckCol: P.umber });
  tiles.roof2 = makeTile(['#bba586', '#a8927a', '#c8b292'], { speck: 8, speckCol: P.sepia });
  tiles.roof3 = makeTile(['#c07a4c', '#ad683e', '#c98a5a'], { speck: 8, speckCol: P.umber });
  tiles.brick = makeTile(['#b46a48', '#9c573a', '#c47c56'], { speck: 10, speckCol: P.umber });
  tiles.slate = makeTile(['#9aa09a', '#868c88', '#aab0a8'], { speck: 8, speckCol: P.ink });
  tiles.stone = makeTile(['#d8d0b4', '#c4bc9e', '#e2dcc4'], { speck: 10, speckCol: P.sepia });
  tiles.white = makeTile(['#fbf7ea', '#f0ead6', '#fffdf4'], { speck: 0, bloom: 0.04, gran: 0.03 });
}
const leafTiles = () => [tiles.ochre, tiles.ochre, tiles.ochre, tiles.khaki, tiles.khaki, tiles.chart, tiles.olive, tiles.olive, tiles.straw, tiles.straw, tiles.rust];

// ---------- the forest's things, each drawn by hand at (x, y) in units
function leafOutline(len, wid, serr) {
  const pts = [], N = 16;
  const hw = u => wid * 0.5 * Math.pow(Math.sin(Math.PI * Math.pow(u, 0.72)), 0.9);
  for (let i = 0; i <= N; i++) { const u = i / N; pts.push([u * len, -hw(u) * (serr && i % 2 && i < N - 1 ? 1.1 : 1)]); }
  for (let i = N - 1; i > 0; i--) { const u = i / N; pts.push([u * len, hw(u) * (serr && i % 2 && i < N - 1 ? 1.1 : 1)]); }
  return { pts, hw };
}
function leaf(x, y, ang, len, wid, tile, o = {}) {
  const { pts, hw } = leafOutline(len, wid, o.serr ?? true);
  const path = new Path2D(); pts.forEach((p, i) => i ? path.lineTo(p[0], p[1]) : path.moveTo(p[0], p[1])); path.closePath();
  const pr = [R(0, TILE), R(0, TILE), R(0, 360)];
  ctx.save(); ctx.translate(x, y); ctx.rotate(ang);
  if (o.shadow !== false) { ctx.save(); ctx.translate(1.2, 1.6); ctx.fillStyle = `rgba(30,26,8,${o.shA ?? 0.28})`; ctx.filter = blurPx(1.2); ctx.fill(path); ctx.restore(); }
  ctx.fillStyle = pattern(tile, pr[0], pr[1], pr[2]); ctx.fill(path);
  ctx.save(); ctx.clip(path);
  ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = `rgba(90,80,20,${o.fold ?? 0.22})`; ctx.fillRect(0, -wid, len, wid);
  const gr = ctx.createLinearGradient(0, 0, len, 0); gr.addColorStop(0, 'rgba(60,50,10,0.25)'); gr.addColorStop(0.5, 'rgba(255,255,255,0)'); gr.addColorStop(1, 'rgba(60,40,10,0.18)');
  ctx.fillStyle = gr; ctx.fillRect(0, -wid, len, wid * 2);
  ctx.restore();
  ctx.strokeStyle = o.inkCol || P.ink; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  ctx.globalAlpha = o.inkA ?? 0.85; ctx.lineWidth = o.w ?? Math.max(0.32, len * 0.0125); ctx.stroke(path);
  ctx.lineWidth *= 0.9; ctx.beginPath(); ctx.moveTo(-len * 0.08, 0); ctx.quadraticCurveTo(len * 0.5, wid * 0.04, len * 0.93, 0); ctx.stroke();
  if (o.veins !== false) {
    ctx.lineWidth *= 0.7; ctx.globalAlpha *= 0.7;
    const nv = o.nv || Math.max(3, Math.round(len / 7));
    for (let k = 1; k <= nv; k++) {
      const u = 0.1 + 0.78 * k / (nv + 0.5);
      for (const sgn of [-1, 1]) { ctx.beginPath(); ctx.moveTo(u * len, 0); ctx.quadraticCurveTo((u + 0.06) * len, sgn * hw(u) * 0.45, (u + 0.13) * len, sgn * hw(Math.min(0.98, u + 0.13)) * 0.86); ctx.stroke(); }
    }
  }
  ctx.restore();
}
function agaric(x, y, s, o = {}) { // a fly agaric, in elevation; s is about the cap's width
  const nd = o.dots ?? 7, dots = []; for (let k = 0; k < nd; k++) dots.push([R(0.25, 0.8), R(0.035, 0.06), R(0.03, 0.045)]);
  ctx.save(); ctx.translate(x, y); ctx.lineJoin = 'round'; ctx.lineCap = 'round';
  ctx.fillStyle = 'rgba(30,26,8,0.25)'; ctx.beginPath(); ctx.ellipse(s * 0.1, 0, s * 0.42, s * 0.09, 0, 0, 7); ctx.fill();
  const sh = s * (o.tall ?? 0.75), sw = s * 0.16;
  const stem = new Path2D(); stem.moveTo(-sw, 0); stem.bezierCurveTo(-sw * 0.8, -sh * 0.5, -sw * 0.6, -sh * 0.8, -sw * 0.55, -sh); stem.lineTo(sw * 0.55, -sh); stem.bezierCurveTo(sw * 0.6, -sh * 0.8, sw * 0.8, -sh * 0.5, sw, 0); stem.closePath();
  ctx.fillStyle = P.white; ctx.fill(stem); ctx.fillStyle = 'rgba(150,130,90,0.25)'; ctx.fillRect(sw * 0.1, -sh, sw, sh);
  ctx.strokeStyle = P.ink; ctx.lineWidth = Math.max(0.3, s * 0.035); ctx.stroke(stem);
  ctx.beginPath(); ctx.ellipse(0, -sh * 0.62, sw * 0.95, sw * 0.28, 0, 0, Math.PI); ctx.stroke();
  const cw = s / 2, ch = s * (o.dome ?? 0.42), cap = new Path2D();
  cap.moveTo(-cw, -sh + s * 0.04); cap.bezierCurveTo(-cw * 0.95, -sh - ch * 1.25, cw * 0.95, -sh - ch * 1.25, cw, -sh + s * 0.04);
  cap.quadraticCurveTo(0, -sh - s * 0.05, -cw, -sh + s * 0.04); cap.closePath();
  const gr = ctx.createRadialGradient(-cw * 0.3, -sh - ch * 0.7, s * 0.05, 0, -sh - ch * 0.3, s * 0.65);
  gr.addColorStop(0, '#e9503a'); gr.addColorStop(0.6, P.agaric); gr.addColorStop(1, '#8e1f12');
  ctx.fillStyle = gr; ctx.fill(cap); ctx.stroke(cap);
  ctx.save(); ctx.clip(cap); ctx.fillStyle = P.white;
  dots.forEach(([v, rx, ry], k) => { const u = (k + 0.5) / nd * 1.8 - 0.9; ctx.globalAlpha = 0.95; ctx.beginPath(); ctx.ellipse(u * cw, -sh - ch * v * (1 - u * u * 0.6), s * rx, s * ry, 0, 0, 7); ctx.fill(); });
  ctx.restore(); ctx.restore();
}
function bracket(x, y, w, ang = 0) { // a bracket fungus shelf
  ctx.save(); ctx.translate(x, y); ctx.rotate(ang);
  const p = new Path2D(); p.moveTo(-w / 2, 0); p.bezierCurveTo(-w * 0.45, -w * 0.22, w * 0.45, -w * 0.24, w / 2, -w * 0.02); p.bezierCurveTo(w * 0.3, w * 0.07, -w * 0.3, w * 0.08, -w / 2, 0); p.closePath();
  ctx.fillStyle = 'rgba(40,20,8,0.25)'; ctx.save(); ctx.translate(0.8, 1.6); ctx.fill(p); ctx.restore();
  const gr = ctx.createLinearGradient(0, -w * 0.2, 0, w * 0.06); gr.addColorStop(0, P.fungus2); gr.addColorStop(0.55, P.fungus); gr.addColorStop(1, '#b4502c');
  ctx.fillStyle = gr; ctx.fill(p);
  ctx.strokeStyle = P.ink; ctx.lineWidth = Math.max(0.35, w * 0.025); ctx.stroke(p);
  ctx.save(); ctx.clip(p); ctx.lineWidth *= 0.6; ctx.globalAlpha = 0.7;
  for (let k = -6; k <= 6; k++) { const u = k / 6.5 * w * 0.46; ctx.beginPath(); ctx.moveTo(u, -w * 0.02); ctx.lineTo(u * 1.04, w * 0.07); ctx.stroke(); }
  ctx.globalAlpha = 0.5; ctx.beginPath(); ctx.moveTo(-w * 0.42, -w * 0.07); ctx.bezierCurveTo(-w * 0.2, -w * 0.14, w * 0.2, -w * 0.15, w * 0.42, -w * 0.06); ctx.stroke();
  ctx.restore(); ctx.restore();
}
function foxglove(x, y, h, o = {}) { // a foxglove spike, its bells hanging along one side
  ctx.save(); ctx.translate(x, y); ctx.lineCap = 'round';
  const lean = o.lean ?? 0;
  const at = t => [lean * h * t * t, -h * t];
  ctx.strokeStyle = '#4f5a1c'; ctx.lineWidth = Math.max(0.6, h * 0.018);
  ctx.beginPath(); for (let k = 0; k <= 20; k++) { const p = at(k / 20); k ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1]); } ctx.stroke();
  for (let k = 0; k < 3; k++) { const s = k % 2 ? 1 : -1; leaf(s * h * 0.04, -h * 0.02 * k, (s > 0 ? -0.35 : Math.PI + 0.35) + R(-0.2, 0.2), h * R(0.22, 0.3), h * 0.075, tiles.olive, { shadow: false, nv: 4 }); }
  const nb = o.bells ?? 16, side = o.side ?? 1;
  for (let k = nb - 1; k >= 0; k--) {
    const t = 0.3 + 0.66 * k / nb, p = at(t), L = h * (0.1 - 0.05 * k / nb), bud = k > nb * 0.76, sd = (k % 4 === 1 ? -1 : 1) * side;
    const sp = [R(-0.16, 0.18), R(0.96, 1.02), R(-0.16, 0.18), R(0.96, 1.02), R(-0.16, 0.18), R(0.96, 1.02)];
    ctx.save(); ctx.translate(p[0] + sd * L * 0.12, p[1]); ctx.rotate(-sd * (0.42 + 0.12 * ((k * 5) % 3)));
    if (bud) {
      ctx.fillStyle = k > nb * 0.88 ? '#94a548' : '#b886c0'; ctx.strokeStyle = 'rgba(42,26,13,0.5)'; ctx.lineWidth = 0.25;
      ctx.beginPath(); ctx.ellipse(0, L * 0.3, L * 0.17, L * 0.3, 0, 0, 7); ctx.fill(); ctx.stroke();
    } else {
      const b = new Path2D();
      b.moveTo(-L * 0.1, 0); b.bezierCurveTo(-L * 0.16, L * 0.35, -L * 0.3, L * 0.7, -L * 0.3, L * 0.95);
      b.quadraticCurveTo(-L * 0.02, L * 1.06, L * 0.33, L * 1.04); b.bezierCurveTo(L * 0.3, L * 0.7, L * 0.16, L * 0.35, L * 0.1, 0); b.closePath();
      const gr = ctx.createLinearGradient(-L * 0.3, 0, L * 0.3, 0); gr.addColorStop(0, '#dcb0e0'); gr.addColorStop(0.5, '#ae64bf'); gr.addColorStop(1, '#723585');
      ctx.fillStyle = gr; ctx.fill(b); ctx.strokeStyle = 'rgba(42,26,13,0.55)'; ctx.lineWidth = Math.max(0.22, L * 0.028); ctx.stroke(b);
      ctx.fillStyle = '#f1d9ec'; ctx.beginPath(); ctx.ellipse(L * 0.02, L * 1.0, L * 0.27, L * 0.06, 0.05, 0, 7); ctx.fill();
      ctx.fillStyle = '#5a1f66'; for (let q = 0; q < 3; q++) { ctx.beginPath(); ctx.arc(sp[q * 2] * L, L * sp[q * 2 + 1], L * 0.022, 0, 7); ctx.fill(); }
    }
    ctx.restore();
  }
  for (let k = 0; k < 4; k++) { const p = at(0.98 + k * 0.006); ctx.fillStyle = '#7d9a35'; ctx.beginPath(); ctx.ellipse(p[0], p[1] - k * h * 0.012, h * 0.012, h * 0.02, 0, 0, 7); ctx.fill(); }
  ctx.restore();
}
function flower(x, y, r, o = {}) { // a small white flower
  const rot = R(0, 6.3), n = o.petals || 6;
  ctx.save(); ctx.translate(x, y); ctx.rotate(rot);
  for (let k = 0; k < n; k++) {
    ctx.rotate(Math.PI * 2 / n); ctx.fillStyle = o.col || P.white; ctx.beginPath(); ctx.ellipse(r * 0.55, 0, r * 0.5, r * 0.26, 0, 0, 7); ctx.fill();
    ctx.strokeStyle = 'rgba(42,26,13,0.45)'; ctx.lineWidth = Math.min(0.25, r * 0.12); ctx.stroke();
  }
  ctx.fillStyle = '#e4b33a'; ctx.beginPath(); ctx.arc(0, 0, r * 0.28, 0, 7); ctx.fill();
  ctx.restore();
}
function fern(x, y, len, ang, o = {}) { // a frond: a curving rib with paired leaflets
  ctx.save(); ctx.translate(x, y); ctx.rotate(ang); ctx.lineCap = 'round';
  const curl = o.curl ?? 0.35, N = o.n ?? 18;
  const pts = []; for (let k = 0; k <= 40; k++) { const t = k / 40; pts.push([t * len, Math.sin(t * 1.4) * curl * len * t * 0.5]); }
  ctx.strokeStyle = o.rib || '#55621f'; ctx.lineWidth = Math.max(0.4, len * 0.012);
  ctx.beginPath(); pts.forEach((p, i) => i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1])); ctx.stroke();
  ctx.fillStyle = o.col || '#86a332';
  for (let k = 2; k < N; k++) {
    const t = k / N, i = Math.round(t * 40), p = pts[i], q = pts[Math.min(40, i + 1)], a = Math.atan2(q[1] - p[1], q[0] - p[0]);
    const l = len * 0.16 * Math.sin(Math.PI * Math.pow(t, 0.7)) + len * 0.02;
    for (const s of [-1, 1]) {
      ctx.save(); ctx.translate(p[0], p[1]); ctx.rotate(a + s * 1.05);
      ctx.beginPath(); ctx.moveTo(0, 0); ctx.quadraticCurveTo(l * 0.5, -l * 0.22, l, 0); ctx.quadraticCurveTo(l * 0.5, l * 0.22, 0, 0); ctx.fill();
      ctx.strokeStyle = 'rgba(42,40,10,0.55)'; ctx.lineWidth = 0.25; ctx.stroke(); ctx.restore();
    }
  }
  ctx.restore();
}
function twig(pts, w) { ctx.save(); ctx.strokeStyle = P.umber; ctx.lineCap = 'round'; ctx.lineWidth = w; ctx.beginPath(); smoothPts(pts, 2).forEach((p, i) => i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1])); ctx.stroke(); ctx.restore(); }

// A hill in elevation (the film's flat false perspective): a dome on its plan, banded inside with sweeping lines,
// hatched on its shadow side. (x, y) is the middle of its foot.
function hill(x, y, w, h, tile, o = {}) {
  const sk = o.skew ?? R(-0.12, 0.12), pat = pattern(tile);
  const dome = (sc, dy) => { const p = new Path2D(), ww = w * sc, hh = h * sc; p.moveTo(x - ww / 2, y + dy); p.bezierCurveTo(x - ww * 0.36, y + dy - hh * 1.2, x + ww * (sk + 0.02) - ww * 0.18, y + dy - hh * 1.36, x + ww * sk, y + dy - hh * 1.33 * 0.75); p.bezierCurveTo(x + ww * (sk + 0.2), y + dy - hh * 1.25, x + ww * 0.38, y + dy - hh * 1.05, x + ww / 2, y + dy); return p; };
  const outer = dome(1, 0), body = new Path2D(outer); body.closePath();
  ctx.save();
  ctx.fillStyle = 'rgba(40,30,8,0.2)'; ctx.beginPath(); ctx.ellipse(x + w * 0.12, y + 0.3, w * 0.55, h * 0.16, 0, 0, 7); ctx.fill();
  ctx.fillStyle = pat; ctx.fill(body);
  ctx.save(); ctx.clip(body);
  ctx.globalCompositeOperation = 'multiply';
  const gr = ctx.createLinearGradient(x - w / 2, 0, x + w / 2, 0); gr.addColorStop(0, 'rgba(255,255,255,0)'); gr.addColorStop(0.55, 'rgba(255,255,255,0)'); gr.addColorStop(1, 'rgba(110,60,20,0.38)');
  ctx.fillStyle = gr; ctx.fillRect(x - w, y - h * 2, w * 2, h * 2);
  ctx.globalCompositeOperation = 'source-over';
  ctx.strokeStyle = o.bandCol || P.umber; ctx.lineCap = 'round';
  const nb = o.bands ?? 3;
  for (let k = 1; k <= nb; k++) { const sc = 1 - k * (0.8 / (nb + 1)); ctx.globalAlpha = 0.55; ctx.lineWidth = Math.max(0.25, w * 0.012); ctx.stroke(dome(sc, 0)); }
  ctx.globalAlpha = 0.55; ctx.lineWidth = Math.max(0.22, w * 0.009);
  const nh = o.hatch ?? Math.round(w / 2.2);
  for (let k = 0; k < nh; k++) { const t = 0.55 + 0.42 * k / Math.max(1, nh), hx = x - w / 2 + w * t; ctx.beginPath(); ctx.moveTo(hx, y - h * (1.05 - t * 0.9) * 0.95); ctx.lineTo(hx - w * 0.05, y); ctx.stroke(); }
  ctx.restore();
  ctx.strokeStyle = P.ink; ctx.globalAlpha = o.inkA ?? 0.8; ctx.lineWidth = Math.max(0.35, w * 0.02); ctx.lineCap = 'round'; ctx.stroke(outer);
  ctx.restore();
}
// Hills in staggered rows along a spine (units), smaller toward its ends, drawn back to front.
function hillChain(pts, o = {}) {
  pts = smoothPts(pts, 3);
  const cum = [0]; for (let i = 1; i < pts.length; i++) cum.push(cum[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const total = cum[cum.length - 1], list = [], w0 = o.w ?? 20, h0 = o.h ?? 9, rows = o.rows ?? 2, step = w0 * (o.step ?? 0.62);
  const at = s => { let i = 1; while (i < pts.length - 1 && cum[i] < s) i++; const t = (s - cum[i - 1]) / (cum[i] - cum[i - 1] || 1), a = pts[i - 1], b = pts[i], L = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1; return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, -(b[1] - a[1]) / L, (b[0] - a[0]) / L]; };
  for (let r = 0; r < rows; r++) for (let s = (r % 2) * step * 0.5 + R(0, step * 0.3); s < total; s += step * R(0.8, 1.15)) {
    const e = Math.pow(Math.sin(Math.PI * s / total), 0.6), [x, y, nx, ny] = at(s), d = (r - (rows - 1) / 2) * h0 * (o.rowGap ?? 1.0) * (0.5 + 0.5 * e);
    const skip = rnd() < 0.5, jx = R(-1.5, 1.5), jy = R(-1, 1), sw = R(0.85, 1.15), sh = R(0.85, 1.2);
    if (e < 0.25 && skip) continue;
    list.push([x + nx * d + jx, y + ny * d + jy, w0 * (0.55 + 0.45 * e) * sw, h0 * (0.5 + 0.5 * e) * sh]);
  }
  list.sort((a, b) => a[1] - b[1]);
  const tl = o.tiles || [tiles.ochre, tiles.rust, tiles.straw, tiles.ochre, tiles.khaki];
  for (const [x, y, w, h] of list) hill(x, y, w, h, pick(tl), o);
  return list;
}
// A round crown seen from above: a scalloped disc of mottled colour, its shadow cast on the ground, darker on its
// shadow side, inked in a broken line; now and then a spiral in the foliage, as the film draws it.
function crown(x, y, r, tile, o = {}) {
  const lobes = o.lobes ?? Math.max(6, Math.round(r * 1.6)), ph = R(0, 6.28), pts = [];
  for (let k = 0; k <= lobes * 4; k++) { const a = k / (lobes * 4) * 6.2832, rr = r * (1 + 0.09 * Math.abs(Math.sin(a * lobes / 2 + ph)) - 0.05); pts.push([x + rr * Math.cos(a), y + rr * Math.sin(a)]); }
  const p = linePath(pts); p.closePath();
  const pat = pattern(tile), dash = [r * R(2, 4), r * R(0.2, 0.6), r * R(1, 2), r * R(0.1, 0.4)], sdir = rnd() < 0.5 ? 1 : -1;
  ctx.save();
  if (o.shadow !== false) { ctx.save(); ctx.translate(r * 0.38, r * 0.46); ctx.fillStyle = `rgba(35,32,8,${o.shA ?? 0.3})`; ctx.fill(p); ctx.restore(); }
  ctx.fillStyle = pat; ctx.fill(p);
  ctx.save(); ctx.clip(p); ctx.globalCompositeOperation = 'multiply';
  const g = ctx.createRadialGradient(x - r * 0.4, y - r * 0.45, r * 0.1, x, y, r * 1.15); g.addColorStop(0, 'rgba(255,255,230,0)'); g.addColorStop(0.6, 'rgba(160,150,80,0.12)'); g.addColorStop(1, 'rgba(70,62,20,0.5)');
  ctx.fillStyle = g; ctx.fillRect(x - r * 1.2, y - r * 1.2, r * 2.4, r * 2.4);
  ctx.restore();
  if (o.leaves && r > 5) { // leaves drawn one by one around the rim, where it counts
    const nl = Math.round(r * 2.1);
    for (let k = 0; k < nl; k++) { const a = k / nl * 6.283 + R(-0.1, 0.1), d = r * R(0.62, 0.88), L = r * R(0.32, 0.42), ra = R(-0.4, 0.4);
      leaf(x + d * Math.cos(a) - Math.cos(a) * L * 0.5, y + d * Math.sin(a) - Math.sin(a) * L * 0.5, a + ra, L, L * 0.5, tile, { shadow: false, veins: L > 5, nv: 3, inkA: 0.55, w: 0.28, fold: 0.18 }); }
  }
  ctx.strokeStyle = P.ink; ctx.lineCap = 'round'; ctx.globalAlpha = o.inkA ?? 0.5; ctx.lineWidth = o.w ?? Math.max(0.3, r * 0.07);
  ctx.setLineDash(dash); ctx.stroke(p); ctx.setLineDash([]);
  if (o.spiral) { ctx.globalAlpha *= 0.75; ctx.lineWidth *= 0.8; ctx.stroke(linePath(spiralPts(x - r * 0.08, y - r * 0.05, r * 0.06, r * 0.5, 1.7, ph, sdir))); }
  ctx.restore();
}
// A small round tree in elevation, the continent's grammar for a wood: an egg of crown on a stroke of trunk.
function roundTree(x, y, r, tile, o = {}) {
  const pat = pattern(tile);
  ctx.save(); ctx.translate(x, y);
  ctx.fillStyle = 'rgba(40,36,10,0.22)'; ctx.beginPath(); ctx.ellipse(r * 0.5, r * 0.15, r * 0.9, r * 0.35, 0, 0, 7); ctx.fill();
  ctx.strokeStyle = P.umber; ctx.lineWidth = Math.max(0.25, r * 0.14); ctx.beginPath(); ctx.moveTo(0, 0); ctx.lineTo(0, -r * 0.9); ctx.stroke();
  const p = new Path2D(); p.ellipse(0, -r * 1.55, r * 0.85, r * (o.tall ?? 1.05), 0, 0, 7);
  ctx.fillStyle = pat; ctx.fill(p);
  ctx.save(); ctx.clip(p); ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = 'rgba(70,64,20,0.35)'; ctx.fillRect(r * 0.05, -r * 3, r, r * 3); ctx.restore();
  ctx.strokeStyle = P.ink; ctx.globalAlpha = o.inkA ?? 0.7; ctx.lineWidth = Math.max(0.22, r * 0.12); ctx.stroke(p);
  ctx.restore();
}
// A spruce: the film's tall vertical.
function spruce(x, y, h, tile) {
  const pat = pattern(tile);
  ctx.save(); ctx.translate(x, y);
  ctx.strokeStyle = P.umber; ctx.lineWidth = Math.max(0.25, h * 0.04); ctx.beginPath(); ctx.moveTo(0, 0); ctx.lineTo(0, -h * 0.2); ctx.stroke();
  const p = new Path2D(); p.moveTo(0, -h); p.quadraticCurveTo(h * 0.2, -h * 0.5, h * 0.17, -h * 0.18); p.quadraticCurveTo(0, -h * 0.12, -h * 0.17, -h * 0.18); p.quadraticCurveTo(-h * 0.2, -h * 0.5, 0, -h); p.closePath();
  ctx.fillStyle = pat; ctx.fill(p);
  ctx.save(); ctx.clip(p); ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = 'rgba(50,50,15,0.4)'; ctx.fillRect(0, -h, h, h); ctx.restore();
  ctx.strokeStyle = P.ink; ctx.globalAlpha = 0.65; ctx.lineWidth = Math.max(0.22, h * 0.035); ctx.stroke(p);
  ctx.restore();
}
// A round tower in elevation, standing up out of a plan.
function roundTower(x, y, w, h, o = {}) {
  ctx.save();
  ctx.fillStyle = 'rgba(40,30,8,0.28)'; ctx.beginPath(); ctx.ellipse(x + w * 0.5, y + 0.3, w * 0.9, w * 0.32, 0, 0, 7); ctx.fill();
  const t = new Path2D(); t.moveTo(x - w / 2, y); t.lineTo(x - w * 0.42, y - h); t.lineTo(x, y - h - w * (o.cap ?? 1.1)); t.lineTo(x + w * 0.42, y - h); t.lineTo(x + w / 2, y); t.closePath();
  const gr = ctx.createLinearGradient(x - w / 2, 0, x + w / 2, 0); gr.addColorStop(0, o.lit || '#e2dcc0'); gr.addColorStop(0.55, o.mid || '#b3ad94'); gr.addColorStop(1, o.dark || '#7c7764');
  ctx.fillStyle = gr; ctx.fill(t); ctx.strokeStyle = P.ink; ctx.lineWidth = Math.max(0.3, w * 0.11); ctx.stroke(t);
  ctx.lineWidth = Math.max(0.2, w * 0.06); ctx.beginPath(); ctx.moveTo(x - w * 0.42, y - h); ctx.lineTo(x + w * 0.42, y - h); ctx.stroke();
  ctx.fillStyle = P.ink; for (let k = 0; k < (o.windows ?? 2); k++) ctx.fillRect(x - w * 0.08, y - h * (0.8 - k * 0.3), w * 0.16, w * 0.32);
  if (o.capCol) { const c = new Path2D(); c.moveTo(x - w * 0.42, y - h); c.lineTo(x, y - h - w * (o.cap ?? 1.1)); c.lineTo(x + w * 0.42, y - h); c.closePath(); ctx.fillStyle = o.capCol; ctx.fill(c); ctx.stroke(c); }
  ctx.restore();
}

// ---------- spirals
// An Archimedean spiral from radius r1 (outside) in to r0, `turns` turns, starting at angle a0, dir ±1.
function spiralPts(cx, cy, r0, r1, turns, a0 = 0, dir = 1, sq = 1) {
  const pts = [], N = Math.max(40, Math.round(turns * 60));
  for (let k = 0; k <= N; k++) { const t = k / N, r = r1 + (r0 - r1) * t, a = a0 + dir * t * turns * Math.PI * 2; pts.push([cx + r * Math.cos(a), cy + r * Math.sin(a) * sq]); }
  return pts;
}
// A spiral carved like Newgrange's stones: a groove of darker pigment, pen strokes across its shadow side, a lit lip.
function carve(pts, o = {}) {
  const w = o.w ?? 3, path = linePath(pts);
  wash(c => { c.lineWidth = w; c.lineCap = 'round'; c.lineJoin = 'round'; c.stroke(path); }, { c: o.c || [P.sepia, P.rust], op: o.op ?? 0.55, edge: 1.4, blur: w * 0.35, mottle: 0.3, bbox: o.bbox });
  ctx.save(); ctx.strokeStyle = o.ink || P.ink; ctx.lineCap = 'round';
  const step = o.step ?? Math.max(0.9, w * 0.42);
  let acc = 0;
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1], b = pts[i], L = Math.hypot(b[0] - a[0], b[1] - a[1]); if (!L) continue; acc += L;
    while (acc > step) {
      acc -= step; const t = 1 - acc / L, px = a[0] + (b[0] - a[0]) * t, py = a[1] + (b[1] - a[1]) * t;
      const nx = -(b[1] - a[1]) / L, ny = (b[0] - a[0]) / L, s = o.side ?? 1, ga = (o.alpha ?? 0.6) * R(0.5, 1), lw = (o.hw ?? 0.35) * R(0.7, 1.3), q = R(0.0, 0.25);
      ctx.globalAlpha = ga; ctx.lineWidth = lw;
      ctx.beginPath(); ctx.moveTo(px + nx * s * w * 0.5, py + ny * s * w * 0.5); ctx.lineTo(px - nx * s * w * q, py - ny * s * w * q); ctx.stroke();
    }
  }
  ctx.globalAlpha = o.edgeA ?? 0.8; ctx.lineWidth = o.lw ?? 0.45;
  ctx.stroke(offsetPath(pts, (o.side ?? 1) * w * 0.5));
  if (o.lip !== false) { ctx.strokeStyle = o.lipCol || 'rgba(255,248,220,0.75)'; ctx.globalAlpha = 0.8; ctx.lineWidth = o.lipW ?? 0.6; ctx.stroke(offsetPath(pts, -(o.side ?? 1) * w * 0.62)); }
  ctx.restore();
}
// A Newgrange double spiral: two spirals wound in opposite senses, joined by an S.
function doubleSpiral(cx, cy, r, turns, ang = 0, sq = 1) {
  const c1 = [cx - r * Math.cos(ang), cy - r * Math.sin(ang) * sq], c2 = [cx + r * Math.cos(ang), cy + r * Math.sin(ang) * sq];
  const A = spiralPts(c1[0], c1[1], r * 0.08, r * 0.95, turns, ang - Math.PI / 2, 1, sq).reverse();
  const B = spiralPts(c2[0], c2[1], r * 0.08, r * 0.95, turns, ang + Math.PI / 2, 1, sq);
  return A.concat(B);
}
// A spiral of ink with a row of white dots beside it: the film's way of edging a wave.
function dottedSpiral(pts, ptsDots, o = {}) {
  ink(linePath(pts), { w: o.w ?? 0.7, col: o.col || P.jade, alpha: o.alpha ?? 0.7, passes: 1 });
  if (ptsDots) dots(ptsDots, o);
}
function dots(pts, o = {}) {
  ctx.save(); ctx.setLineDash([0.1, o.gap ?? 3.2]); ctx.lineCap = 'round'; ctx.strokeStyle = o.dotCol || 'rgba(255,253,240,0.95)'; ctx.lineWidth = o.dotW ?? 1.1;
  ctx.stroke(linePath(pts)); ctx.restore();
}

// ---------- lettering
async function loadFonts() {
  const faces = [['Uncial Antiqua', 'fonts/UncialAntiqua-Regular.ttf'], ['IM Fell English SC', 'fonts/IMFellEnglishSC-Regular.ttf'],
    ['IM Fell English Italic', 'fonts/IMFellEnglish-Italic.ttf'], ['EB Garamond', 'fonts/EBGaramond-Regular.ttf']];
  for (const [name, url] of faces) { const f = new FontFace(name, `url(${url})`); await f.load(); document.fonts.add(f); }
}
function label(text, x, y, o = {}) {
  ctx.save(); ctx.font = o.font || `${o.size || 12}px "Uncial Antiqua"`; ctx.textAlign = o.align || 'center'; ctx.textBaseline = 'middle';
  ctx.letterSpacing = (o.ls ?? 0.08) * (o.size || 12) + 'px';
  ctx.translate(x, y); if (o.ang) ctx.rotate(o.ang);
  if (o.halo !== false) {
    ctx.strokeStyle = o.haloCol || rgba(P.paper, 0.7); ctx.lineJoin = 'round'; ctx.lineWidth = o.haloW ?? 3.2;
    ctx.filter = blurPx(o.haloBlur ?? 1.2); ctx.strokeText(text, 0, 0); ctx.filter = 'none';
  }
  ctx.fillStyle = o.col || P.ink; ctx.globalAlpha = o.alpha ?? 0.92; ctx.fillText(text, 0, 0);
  ctx.restore();
}
function textOnPath(text, pts, o = {}) {
  ctx.save(); ctx.font = o.font || `${o.size || 12}px "Uncial Antiqua"`; ctx.textBaseline = 'middle'; ctx.textAlign = 'center';
  const ls = (o.ls ?? 0.1) * (o.size || 12);
  const cum = [0]; for (let i = 1; i < pts.length; i++) cum.push(cum[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const total = cum[cum.length - 1];
  const widths = [...text].map(ch => ctx.measureText(ch).width + ls), tw = widths.reduce((a, b) => a + b, 0) - ls;
  let s = (o.at ?? 0.5) * total - tw / 2;
  const at = d => { let i = 1; while (i < cum.length - 1 && cum[i] < d) i++; const t = (d - cum[i - 1]) / (cum[i] - cum[i - 1] || 1), a = pts[i - 1], b = pts[i]; return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, Math.atan2(b[1] - a[1], b[0] - a[0])]; };
  [...text].forEach((ch, k) => {
    const mid = s + (widths[k] - ls) / 2, p = at(Math.max(0, Math.min(total, mid)));
    ctx.save(); ctx.translate(p[0], p[1]); ctx.rotate(p[2]);
    if (o.halo !== false) { ctx.strokeStyle = o.haloCol || rgba(P.paper, 0.65); ctx.lineWidth = o.haloW ?? 3; ctx.lineJoin = 'round'; ctx.filter = blurPx(1.1); ctx.strokeText(ch, 0, 0); ctx.filter = 'none'; }
    ctx.fillStyle = o.col || P.ink; ctx.globalAlpha = o.alpha ?? 0.9; ctx.fillText(ch, 0, 0); ctx.restore();
    s += widths[k];
  });
  ctx.restore();
}

// ---------- light and paper
// Dappled light: the shade of a canopy overhead, flecks of sun through it.
function dapple(o = {}) {
  const img = ctx.getImageData(0, 0, PW, PH), d = img.data, sh = hex(o.shade || '#56601e'), k = o.k ?? 0.3, sun = o.sun ?? 0.2;
  for (let i = 0, p = 0; i < PW * PH; i++, p += 4) {
    const n = FB1[i] * 0.65 + FM[i] * 0.35, shade = smooth(0.44, 0.6, 1 - n) * k, fleck = smooth(0.62, 0.72, FB2[i] * 0.8 + FM[i] * 0.2) * sun;
    let r = d[p] * (1 - shade * (1 - sh[0])), g = d[p + 1] * (1 - shade * (1 - sh[1])), b = d[p + 2] * (1 - shade * (1 - sh[2]));
    r += (255 - r) * fleck; g += (246 - g) * fleck; b += (200 - b) * fleck * 0.8;
    d[p] = r; d[p + 1] = g; d[p + 2] = b;
  }
  ctx.putImageData(img, 0, 0);
}
// The paper, last: grain and fibre over the ground, so every layer sits in one sheet.
function paperFinish(o = {}) {
  const img = ctx.getImageData(0, 0, PW, PH), d = img.data, k0 = o.grain ?? 0.045, k1 = o.tooth ?? 0.04;
  for (let i = 0, p = 0; i < PW * PH; i++, p += 4) {
    const f = 1 - k0 * (FG[i] - 0.5) * 2 - k1 * (FM[i] - 0.5) * 2;
    d[p] *= f; d[p + 1] *= f; d[p + 2] *= f * 0.995;
  }
  ctx.putImageData(img, 0, 0);
}
function paperGround(col = P.paper) { css(ctx); ctx.fillStyle = col; ctx.fillRect(-10, -10, W + 20, H + 20); }
async function loadJSON(u) { const r = await fetch(u); if (!r.ok) throw new Error(u + ' ' + r.status); return r.json(); }
async function loadImageData(u) {
  const im = new Image(); im.src = u; await im.decode();
  const c = document.createElement('canvas'); c.width = im.width; c.height = im.height; const x = c.getContext('2d'); x.drawImage(im, 0, 0);
  return { w: im.width, h: im.height, data: x.getImageData(0, 0, im.width, im.height).data };
}
