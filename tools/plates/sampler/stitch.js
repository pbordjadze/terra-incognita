// The embroidery, stitched per pixel: linen, counted cross stitch, and freeform stitches (backstitch,
// running stitch, satin, couched thread, French knots), some in gold. Every picture is computed from
// plate coordinates through a view (s, tx, ty), so the same scene drawn magnified is re-stitched with
// larger threads, not blown up: that is how the pass plates show the stitches growing.
//
// A scene is
//   { linen: { P, ox, oy, rgb, seed } | null,     // the ground cloth, threads P plate px apart
//     pal: [{ rgb: [r, g, b], metal }],            // thread colours, 0..1; index 0 is unused
//     grids: [{ c, ox, oy, ncol, nrow, data, hw, seed, shadow }],  // counted cross stitch, cell c
//     segs: Segs }                                 // freeform stitches, drawn in order, by tier
'use strict';
const Stitch = (() => {
  const n3 = (x, y, z) => { const l = Math.hypot(x, y, z); return [x / l, y / l, z / l]; };
  const LIGHT = n3(-0.45, -0.62, 0.64), HALF = n3(LIGHT[0], LIGHT[1], LIGHT[2] + 1);
  const LX = LIGHT[0], LY = LIGHT[1], LZ = LIGHT[2], HX = HALF[0], HY = HALF[1], HZ = HALF[2];
  const SHX = 0.52, SHY = 0.85;            // where shadows fall: down and a little right

  function hash(a, b, c) {
    let h = Math.imul(a | 0, 0x27d4eb2d) ^ Math.imul(b | 0, 0x165667b1) ^ Math.imul(c | 0, 0x2f0b4a13);
    h = Math.imul(h ^ (h >>> 15), 0x85ebca6b);
    h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35);
    return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
  }
  function vnoise(x, y, s) {
    const xi = Math.floor(x), yi = Math.floor(y), fx = x - xi, fy = y - yi;
    const ux = fx * fx * (3 - 2 * fx), uy = fy * fy * (3 - 2 * fy);
    const a = hash(xi, yi, s), b = hash(xi + 1, yi, s), c = hash(xi, yi + 1, s), d = hash(xi + 1, yi + 1, s);
    return a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy;
  }
  const clamp01 = x => x < 0 ? 0 : x > 1 ? 1 : x;
  function rgb(hex) {
    const v = parseInt(hex.slice(1), 16);
    return [((v >> 16) & 255) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255];
  }

  // ---- freeform stitches -------------------------------------------------------------------------
  // kind 0: a stitch between two holes (it narrows where it goes into the cloth); 1: laid thread,
  // continuous along a path (couching); 2: a French knot at (x1, y1), hw its radius.
  const F = 13;
  class Segs {
    constructor() { this.a = new Float32Array(1 << 16); this.n = 0; }
    push(x1, y1, x2, y2, hw, col, kind = 0, o = {}) {
      if (!(hw > 0) || !isFinite(x1 + y1 + x2 + y2)) return;
      if ((this.n + 1) * F > this.a.length) { const b = new Float32Array(this.a.length * 2); b.set(this.a); this.a = b; }
      const a = this.a, i = this.n++ * F;
      a[i] = x1; a[i + 1] = y1; a[i + 2] = x2; a[i + 3] = y2; a[i + 4] = hw; a[i + 5] = col; a[i + 6] = kind;
      a[i + 7] = o.s0 || 0; a[i + 8] = o.phase ?? hash(this.n, 7, 11); a[i + 9] = o.tier || 0;
      a[i + 10] = o.shadow ?? 1; a[i + 11] = o.taper ?? 0.5; a[i + 12] = o.mode || 0;
    }
    get length() { return this.n; }
  }

  // ---- shading -------------------------------------------------------------------------------------
  // One thread lit from the upper left: a twisted two-ply cylinder. n is the position across it
  // (-1..1, towards its normal (-diry, dirx)), s the distance along it in plate px, pitch the twist's
  // length, px the plate px per screen px (detail fades out when it would be smaller than a pixel).
  const out3 = new Float64Array(3);
  // mode 0: a twisted two-ply thread (perle, crewel, gold); mode 1: stranded floss, two strands laid
  // side by side, each lightly twisted, flatter and plumper (cross stitch, satin)
  function shade(r, g, b, metal, n, s, pitch, dirx, diry, px, phase, mode) {
    const flat = mode === 1 ? 0.72 : 1;
    const nn = n * flat, nz = Math.sqrt(Math.max(0, 1 - nn * nn));
    const nx = -diry * nn, ny = dirx * nn;
    let dif = nx * LX + ny * LY + nz * LZ; if (dif < 0) dif = 0;
    let spec = nx * HX + ny * HY + nz * HZ; if (spec < 0) spec = 0;
    let groove = 1, crown = 0.5;
    const pp = pitch / px;
    if (mode === 1) {
      // two strands: a soft valley between them, each strand a little cylinder with its own twist
      const m = n >= 0 ? (n - 0.5) * 2 : (n + 0.5) * 2, bs = Math.sqrt(Math.max(0, 1 - m * m));
      const ws = pitch * 0.22 / px;                      // a strand's width on screen
      const amp = clamp01((ws - 3.2) / 3);
      groove = 1 - amp * 0.28 * (1 - bs * bs);
      crown = bs;
      if (pp > 15) {
        const ph = 2 * (s / pitch + m * 0.30 + phase + (n >= 0 ? 0.5 : 0)), f = ph - Math.floor(ph);
        const st = Math.sin(Math.PI * f), a2 = clamp01((pp - 15) / 8);
        groove *= 1 - a2 * 0.16 * (1 - Math.sqrt(st));
        crown *= 0.6 + 0.4 * st;
      }
      if (pp > 16) {
        // the filaments of the floss, once a stitch is large: fine streaks along each strand
        const k = clamp01((pp - 16) / 30);
        const f1 = vnoise(s / (pitch * 0.9) + phase * 50, n * 22, 3) - 0.5, f2 = vnoise(s / (pitch * 0.5) + phase * 31, n * 47, 4) - 0.5;
        groove *= 1 - k * (0.16 * f1 + 0.10 * f2) * 2;
        crown *= 1 - k * 0.3 * Math.abs(f2) * 2;
      }
    } else if (pp > 2.2) {
      // the ply twist: two plies cross the thread diagonally, a groove between them
      const ph = 2 * (s / pitch + n * 0.42 + phase), f = ph - Math.floor(ph);
      crown = Math.sin(Math.PI * f);
      const amp = clamp01((pp - 2.2) / 3);
      groove = 1 - amp * (metal ? 0.42 : 0.30) * (1 - Math.sqrt(crown));
      if (pp > 14) {
        const k = clamp01((pp - 14) / 20) * 0.10;
        groove *= 1 - k * (vnoise(s / (pitch * 0.09) + phase * 50, n * 7 + f * 3, 3) - 0.5) * 2;
      }
    }
    if (metal) {
      const sp = Math.pow(spec, 9) * (0.45 + 0.55 * crown);
      const k = (0.28 + 0.62 * dif) * groove;
      out3[0] = r * k + sp * 0.95; out3[1] = g * k + sp * 0.85; out3[2] = b * k + sp * 0.55;
    } else {
      const sp = Math.pow(spec, 18) * 0.10 * (0.3 + 0.7 * crown);
      const k = (0.46 + 0.80 * dif) * groove;
      out3[0] = r * k + sp; out3[1] = g * k + sp * 0.97; out3[2] = b * k + sp * 0.9;
    }
  }

  // ---- the cloth -----------------------------------------------------------------------------------
  const lin = new Float64Array(3);
  function linen(L, u, v, px) {
    const P = L.P, gx = (u - L.ox) / P, gy = (v - L.oy) / P;
    const k = Math.floor(gx), m = Math.floor(gy);
    const fu = gx - k - 0.5, fv = gy - m - 0.5;
    const seed = L.seed;
    // each thread its own thickness and tint; slubs thicken it here and there along its length
    const tk = hash(k, 0, seed), tm = hash(m, 1, seed);
    const sk = vnoise(k * 17, gy * 0.21, seed + 2), sm = vnoise(m * 17, gx * 0.21, seed + 3);
    const hwk = 0.37 + 0.05 * (tk - 0.5) + 0.12 * Math.max(0, sk - 0.55);
    const hwm = 0.37 + 0.05 * (tm - 0.5) + 0.12 * Math.max(0, sm - 0.55);
    const aa = px / P;
    const cw = clamp01((hwk - Math.abs(fu)) / aa + 0.5), cf = clamp01((hwm - Math.abs(fv)) / aa + 0.5);
    const warpOver = ((k + m) & 1) === 0;
    let nw = fu / hwk; nw = nw > 1 ? 1 : nw < -1 ? -1 : nw;
    let nf = fv / hwm; nf = nf > 1 ? 1 : nf < -1 ? -1 : nf;
    const arch = Math.cos(Math.PI * (warpOver ? fv : fu));
    let bw = 0.84 + 0.16 * Math.sqrt(1 - nw * nw) - 0.07 * nw;
    let bf = 0.84 + 0.16 * Math.sqrt(1 - nf * nf) - 0.07 * nf;
    if (warpOver) { bw *= 0.93 + 0.07 * arch; bf *= 0.90; } else { bf *= 0.93 + 0.07 * arch; bw *= 0.90; }
    bw *= 0.975 + 0.05 * tk + 0.03 * (sk - 0.5);
    bf *= 0.975 + 0.05 * tm + 0.03 * (sm - 0.5);
    // fibres, once a thread is a few pixels wide
    if (px < 0.7) {
      const amp = clamp01((0.7 - px) * 2.2) * 0.07;
      bw *= 1 + amp * (vnoise(k * 31 + fu * 9, gy * 2.6, seed + 5) - 0.5);
      bf *= 1 + amp * (vnoise(m * 31 + fv * 9, gx * 2.6, seed + 6) - 0.5);
    }
    const hole = 0.74;
    let b;
    if (warpOver) b = (hole + (bf - hole) * cf) * (1 - cw) + bw * cw;
    else b = (hole + (bw - hole) * cw) * (1 - cf) + bf * cf;
    // the cloth's tone drifts a little over its width, as old linen does
    const mot = 1 + (vnoise(u / 420, v / 420, seed + 7) - 0.5) * 0.045 + (vnoise(u / 95, v / 95, seed + 8) - 0.5) * 0.016;
    const grey = (tk - 0.5) * (warpOver ? cw : 0) * 0.03;
    lin[0] = L.rgb[0] * b * mot - grey; lin[1] = L.rgb[1] * b * mot - grey * 0.6; lin[2] = L.rgb[2] * b * mot;
  }

  // ---- counted cross stitch ------------------------------------------------------------------------
  // One stroke of a cell, in cell units: the bottom stroke runs / and the top stroke \, both from hole
  // to hole. Returns its coverage at (a, b), and leaves the across position and along distance.
  let sN = 0, sT = 0;
  const E = 0.07, R2 = Math.SQRT1_2;
  function stroke(top, a, b, hw, jit, aa, soft) {
    let rx, ry, dx, dy;
    if (top) { rx = a - E; ry = b - E; dx = R2; dy = R2; } else { rx = a - E; ry = b - 1 + E; dx = R2; dy = -R2; }
    const len = Math.SQRT2 * (1 - 2 * E);
    const t = rx * dx + ry * dy, d = -rx * dy + ry * dx - jit;
    const tc = t < 0 ? 0 : t > len ? len : t, tn = tc / len;
    const w = hw * (0.52 + 0.48 * Math.sqrt(Math.sin(Math.PI * tn)));
    const et = t - tc, dist = Math.sqrt(et * et + d * d);
    sN = d / w; sT = tc;
    return clamp01((w - dist) / (aa + soft) + 0.5);
  }

  const acc = new Float64Array(4);
  function over(r, g, b, c) {
    const k = 1 - c;
    acc[0] = r * c + acc[0] * k; acc[1] = g * c + acc[1] * k; acc[2] = b * c + acc[2] * k; acc[3] = c + acc[3] * k;
  }
  function darken(k) {
    if (k <= 0) return;
    const m = 1 - k;
    acc[0] *= m; acc[1] *= m; acc[2] *= m; acc[3] = k + acc[3] * m;
  }

  function cellInfo(G, i, j) {
    if (i < 0 || j < 0 || i >= G.ncol || j >= G.nrow) return 0;
    return G.data[j * G.ncol + i];
  }

  function crossGrid(G, pal, u, v, px) {
    const c = G.c, gx = (u - G.ox) / c, gy = (v - G.oy) / c;
    const i = Math.floor(gx), j = Math.floor(gy);
    const aa = px / c, hw = G.hw, seed = G.seed;
    // the shadow the stitches cast, from the cell under the point the light comes past
    const so = hw * 0.55 * (G.shadow ?? 1);
    const sx = gx - so * SHX, sy = gy - so * SHY, si = Math.floor(sx), sj = Math.floor(sy);
    const scol = cellInfo(G, si, sj);
    const own = cellInfo(G, i, j);
    if (!scol && !own) return;
    let shTop = 0;
    if (scol) {
      const a = sx - si, b = sy - sj, soft = hw * 0.55;
      const h1 = hash(si, sj, seed);
      const jb = (h1 - 0.5) * 0.05, jt = (hash(si, sj, seed + 1) - 0.5) * 0.05;
      const cb = stroke(false, a, b, hw, jb, aa, soft);
      shTop = stroke(true, a, b, hw, jt, aa, soft);
      darken(Math.max(cb, shTop) * 0.36 * (G.shadow ?? 1));
    }
    if (!own) return;
    const a = gx - i, b = gy - j;
    const h1 = hash(i, j, seed), h2 = hash(i, j, seed + 1), h3 = hash(i, j, seed + 2);
    const jb = (h1 - 0.5) * 0.05, jt = (h2 - 0.5) * 0.05;
    const P = pal[own], base = P.rgb;
    const lot = 1 + (vnoise(i / 7.3, j / 7.3, seed + 4) - 0.5) * (P.lot ?? 0.11) + (h3 - 0.5) * 0.07;
    const r = base[0] * lot, g = base[1] * lot, bl = base[2] * lot;
    const pitch = hw * c * 4.2;
    const wb = hw * (0.94 + 0.12 * h3), wt = hw * (0.94 + 0.12 * h1);
    const cb = stroke(false, a, b, wb, jb, aa, 0);
    if (cb > 0) {
      shade(r, g, bl, P.metal, sN, sT * c, pitch, R2, -R2, px, h1, 1);
      let k = 1;
      if (shTop > 0 && scol) k = 1 - shTop * 0.30;
      over(out3[0] * k * 0.96, out3[1] * k * 0.96, out3[2] * k * 0.96, cb);
    }
    const ct = stroke(true, a, b, wt, jt, aa, 0);
    if (ct > 0) {
      shade(r, g, bl, P.metal, sN, sT * c, pitch, R2, R2, px, h2, 1);
      over(out3[0], out3[1], out3[2], ct);
    }
  }

  // ---- freeform stitches ---------------------------------------------------------------------------
  function segShadow(a, i, u, v, px) {
    const hw = a[i + 4], kind = a[i + 6];
    const off = hw * 0.85 * a[i + 10];
    const x = u - off * SHX, y = v - off * SHY;
    const soft = hw * 0.8 + px;
    const x1 = a[i], y1 = a[i + 1];
    if (kind === 2) {
      const d = Math.hypot(x - x1, y - y1);
      return clamp01((hw - d) / soft + 0.5);
    }
    const dx = a[i + 2] - x1, dy = a[i + 3] - y1, len = Math.hypot(dx, dy) || 1e-6;
    const ux = dx / len, uy = dy / len, rx = x - x1, ry = y - y1;
    const t = rx * ux + ry * uy, tc = t < 0 ? 0 : t > len ? len : t;
    const d = -rx * uy + ry * ux, et = t - tc;
    const w = kind === 0 ? hw * (1 - a[i + 11] * 0.5 * (1 - Math.sqrt(Math.sin(Math.PI * tc / len)))) : hw;
    return clamp01((w - Math.sqrt(et * et + d * d)) / soft + 0.5);
  }

  function segDraw(a, i, pal, u, v, px) {
    const hw = a[i + 4], kind = a[i + 6], P = pal[a[i + 5]];
    const x1 = a[i], y1 = a[i + 1], phase = a[i + 8];
    const base = P.rgb;
    if (kind === 2) {
      const dx = u - x1, dy = v - y1, d = Math.hypot(dx, dy);
      const c = clamp01((hw - d) / px + 0.5);
      if (c <= 0) return;
      const rr = Math.min(1, d / hw), nz = Math.sqrt(1 - rr * rr);
      const nx = d > 0 ? dx / d * rr : 0, ny = d > 0 ? dy / d * rr : 0;
      let dif = nx * LX + ny * LY + nz * LZ; if (dif < 0) dif = 0;
      let spec = nx * HX + ny * HY + nz * HZ; if (spec < 0) spec = 0;
      // the wraps of the knot: a spiral of thread round its crown
      // the wraps of the knot: loops of thread coiled round its crown, each with its ply
      let groove = 1, lit = 1;
      const ppx = hw / px;
      if (ppx > 1.6) {
        const ang = Math.atan2(dy, dx) / (2 * Math.PI);
        const ph = ang * 4 + rr * 2.6 + phase * 3, f = ph - Math.floor(ph), w = Math.sin(Math.PI * f);
        const a = clamp01((ppx - 1.6) / 3);
        groove = 1 - a * 0.42 * (1 - Math.sqrt(w));
        lit = 1 - a * 0.6 * (1 - w);
        if (ppx > 8) {
          const q = ang * 26 + rr * 9 + phase * 7, g = q - Math.floor(q);
          groove *= 1 - clamp01((ppx - 8) / 10) * 0.12 * (1 - Math.sin(Math.PI * g));
        }
      }
      const k = (0.48 + 0.78 * dif) * groove, sp = (P.metal ? Math.pow(spec, 7) * 0.85 : Math.pow(spec, 14) * 0.18) * lit;
      over(base[0] * k + sp, base[1] * k + sp * 0.92, base[2] * k + sp * 0.75, c);
      return;
    }
    const dx = a[i + 2] - x1, dy = a[i + 3] - y1, len = Math.hypot(dx, dy) || 1e-6;
    const ux = dx / len, uy = dy / len, rx = u - x1, ry = v - y1;
    const t = rx * ux + ry * uy, tc = t < 0 ? 0 : t > len ? len : t;
    const d = -rx * uy + ry * ux, et = t - tc;
    let w = hw, endk = 1;
    if (kind === 0) {
      const tn = tc / len, sn = Math.sqrt(Math.sin(Math.PI * tn));
      w = hw * (1 - a[i + 11] * (1 - sn));
      endk = 0.86 + 0.14 * sn;
    }
    const dist = Math.sqrt(et * et + d * d);
    const c = clamp01((w - dist) / px + 0.5);
    if (c <= 0) return;
    let n = d / w; n = n > 1 ? 1 : n < -1 ? -1 : n;
    shade(base[0], base[1], base[2], P.metal, n, a[i + 7] + tc, hw * (a[i + 12] ? 8.4 : 5.2), ux, uy, px, phase, a[i + 12]);
    over(out3[0] * endk, out3[1] * endk, out3[2] * endk, c);
  }

  // which segments touch which screen tile, by tier
  function bin(segs, view, W, H, T) {
    const a = segs.a, n = segs.n, s = view.s, tw = Math.ceil(W / T), th = Math.ceil(H / T);
    let tiers = 1;
    for (let k = 0; k < n; k++) tiers = Math.max(tiers, a[k * F + 9] + 1);
    const counts = new Int32Array(tw * th * tiers + 1);
    const boxes = new Int32Array(n * 4);
    for (let k = 0; k < n; k++) {
      const i = k * F, hw = a[i + 4], pad = hw * 2.2 + 2 / s;
      const x0 = (Math.min(a[i], a[i + 2]) - pad) * s + view.tx, x1 = (Math.max(a[i], a[i + 2]) + pad) * s + view.tx;
      const y0 = (Math.min(a[i + 1], a[i + 3]) - pad) * s + view.ty, y1 = (Math.max(a[i + 1], a[i + 3]) + pad) * s + view.ty;
      if (x1 < 0 || y1 < 0 || x0 >= W || y0 >= H) { boxes[k * 4] = 1; boxes[k * 4 + 2] = 0; continue; }
      const bx0 = Math.max(0, Math.floor(x0 / T)), bx1 = Math.min(tw - 1, Math.floor(x1 / T));
      const by0 = Math.max(0, Math.floor(y0 / T)), by1 = Math.min(th - 1, Math.floor(y1 / T));
      boxes[k * 4] = bx0; boxes[k * 4 + 1] = by0; boxes[k * 4 + 2] = bx1; boxes[k * 4 + 3] = by1;
      const tier = a[i + 9];
      for (let y = by0; y <= by1; y++) for (let x = bx0; x <= bx1; x++) counts[(y * tw + x) * tiers + tier + 1]++;
    }
    for (let k = 1; k < counts.length; k++) counts[k] += counts[k - 1];
    const start = counts.slice();
    const list = new Int32Array(counts[counts.length - 1]);
    for (let k = 0; k < n; k++) {
      const b = k * 4;
      if (boxes[b] > boxes[b + 2]) continue;
      const tier = a[k * F + 9];
      for (let y = boxes[b + 1]; y <= boxes[b + 3]; y++) for (let x = boxes[b]; x <= boxes[b + 2]; x++) list[start[(y * tw + x) * tiers + tier]++] = k;
    }
    return { tw, th, tiers, counts, list };
  }

  // ---- the picture ---------------------------------------------------------------------------------
  // view: plate (u, v) shows at screen (u * s + tx, v * s + ty). Returns RGBA bytes, W x H.
  function render(scene, view, W, H) {
    const t0 = performance.now();
    const outp = new Uint8ClampedArray(W * H * 4);
    const T = 16, s = view.s, px = 1 / s;
    const segs = scene.segs || new Segs();
    const B = bin(segs, view, W, H, T);
    const a = segs.a, pal = scene.pal, L = scene.linen, grids = scene.grids || [];
    const base = scene.base;
    for (let ty = 0; ty < B.th; ty++) for (let tx = 0; tx < B.tw; tx++) {
      const tb = (ty * B.tw + tx) * B.tiers;
      const anyseg = B.counts[tb + B.tiers] > B.counts[tb];
      // a tile none of whose grids' cells nor segments are near, on a transparent layer, stays empty
      const y0 = ty * T, x0 = tx * T, y1 = Math.min(H, y0 + T), x1 = Math.min(W, x0 + T);
      if (!L && !base && !anyseg && !gridsNear(grids, view, x0, y0, x1, y1)) continue;
      for (let y = y0; y < y1; y++) {
        const v = (y + 0.5 - view.ty) * px;
        for (let x = x0; x < x1; x++) {
          const u = (x + 0.5 - view.tx) * px;
          if (L) { linen(L, u, v, px); acc[0] = lin[0]; acc[1] = lin[1]; acc[2] = lin[2]; acc[3] = 1; }
          else if (base) { acc[0] = base[0]; acc[1] = base[1]; acc[2] = base[2]; acc[3] = 1; }
          else { acc[0] = acc[1] = acc[2] = acc[3] = 0; }
          for (let g = 0; g < grids.length; g++) crossGrid(grids[g], pal, u, v, px);
          if (anyseg) {
            for (let tier = 0; tier < B.tiers; tier++) {
              const from = B.counts[tb + tier], to = B.counts[tb + tier + 1];
              if (from === to) continue;
              let sh = 0;
              for (let q = from; q < to; q++) {
                const i = B.list[q] * F;
                const k = segShadow(a, i, u, v, px) * a[i + 10];
                if (k > sh) sh = k;
              }
              darken(sh * 0.34);
              for (let q = from; q < to; q++) segDraw(a, B.list[q] * F, pal, u, v, px);
            }
          }
          const o = (y * W + x) * 4, al = acc[3];
          if (al <= 0) continue;
          const inv = 255 / al;
          outp[o] = acc[0] * inv; outp[o + 1] = acc[1] * inv; outp[o + 2] = acc[2] * inv; outp[o + 3] = al * 255;
        }
      }
    }
    if (window.STITCH_LOG) console.log(`stitched ${W}x${H} at ${s.toFixed(2)}x, ${segs.n} segments, ${(performance.now() - t0).toFixed(0)} ms`);
    return new ImageData(outp, W, H);
  }

  function gridsNear(grids, view, x0, y0, x1, y1) {
    for (const G of grids) {
      const u0 = (x0 - view.tx) / view.s, u1 = (x1 - view.tx) / view.s, v0 = (y0 - view.ty) / view.s, v1 = (y1 - view.ty) / view.s;
      const i0 = Math.floor((u0 - G.ox) / G.c) - 1, i1 = Math.floor((u1 - G.ox) / G.c) + 1;
      const j0 = Math.floor((v0 - G.oy) / G.c) - 1, j1 = Math.floor((v1 - G.oy) / G.c) + 1;
      if (!G.rowsAny) {
        G.rowsAny = new Uint8Array(G.nrow);
        for (let j = 0; j < G.nrow; j++) for (let i = 0; i < G.ncol; i++) if (G.data[j * G.ncol + i]) { G.rowsAny[j] = 1; break; }
      }
      for (let j = Math.max(0, j0); j <= Math.min(G.nrow - 1, j1); j++) {
        if (!G.rowsAny[j]) continue;
        for (let i = Math.max(0, i0); i <= Math.min(G.ncol - 1, i1); i++) if (G.data[j * G.ncol + i]) return true;
      }
    }
    return false;
  }

  return { Segs, render, rgb, hash, vnoise, clamp01 };
})();
window.Stitch = Stitch;
