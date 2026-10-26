// From map data to stitches: the counted grid's cells, distance fields for the water lines, and the
// freeform stitches laid along projected lines and shapes. Everything is seeded, so a plate and its
// magnified pass plates stitch exactly the same threads.
'use strict';
const Geo = (() => {
  const W = Kit.W, H = Kit.H;
  // a small seeded random source
  function rng(seed) {
    let a = seed >>> 0;
    return () => {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  // the projected lines of a GeoJSON object, in plate px: [[x, y], ...] with .closed on rings
  function lines(proj, geo) {
    const out = [];
    let cur = null;
    const ctx = {
      beginPath() {}, moveTo(x, y) { cur = [[x, y]]; out.push(cur); }, lineTo(x, y) { cur.push([x, y]); },
      closePath() { if (cur) { cur.closed = true; if (cur.length) cur.push(cur[0].slice()); } }, arc() {},
    };
    d3.geoPath(proj, ctx)(geo);
    return out.filter(l => l.length > 1);
  }

  // A counted grid over the plate: cell c, origin at a hole of the cloth.
  function grid(c, ox, oy, extra = 0) {
    const i0 = Math.floor((-extra - ox) / c), j0 = Math.floor((-extra - oy) / c);
    const gox = ox + i0 * c, goy = oy + j0 * c;
    const ncol = Math.ceil((W + extra - gox) / c), nrow = Math.ceil((H + extra - goy) / c);
    return { c, ox: gox, oy: goy, ncol, nrow, data: new Uint8Array(ncol * nrow) };
  }

  // How much of each cell a shape covers (0..16), from a canvas drawn 4 samples to a cell side.
  function coverage(G, draw) {
    const S = 4, cv = document.createElement('canvas');
    cv.width = G.ncol * S; cv.height = G.nrow * S;
    const ctx = cv.getContext('2d');
    ctx.setTransform(S / G.c, 0, 0, S / G.c, -G.ox * S / G.c, -G.oy * S / G.c);
    ctx.fillStyle = '#fff'; ctx.strokeStyle = '#fff';
    draw(ctx);
    const px = ctx.getImageData(0, 0, cv.width, cv.height).data;
    const cov = new Float32Array(G.ncol * G.nrow);
    for (let y = 0; y < cv.height; y++) {
      const j = (y / S) | 0;
      for (let x = 0; x < cv.width; x++) cov[j * G.ncol + ((x / S) | 0)] += px[(y * cv.width + x) * 4 + 3] / 255;
    }
    return cov;
  }
  // fills cells at least `min` (of 16) covered by the features with colour col (or col(cellIndex))
  function fill(G, proj, geo, col, min = 8) {
    const cov = coverage(G, ctx => { ctx.beginPath(); d3.geoPath(proj, ctx)(geo); ctx.fill(); });
    for (let k = 0; k < cov.length; k++) if (cov[k] >= min) G.data[k] = typeof col === 'function' ? col(k) : col;
    return cov;
  }

  // A mask of the plate at `res` plate px per sample (1 inside), drawn by draw(ctx) in plate px.
  function mask(res, draw) {
    const w = Math.ceil(W / res), h = Math.ceil(H / res);
    const cv = document.createElement('canvas'); cv.width = w; cv.height = h;
    const ctx = cv.getContext('2d');
    ctx.setTransform(1 / res, 0, 0, 1 / res, 0, 0);
    ctx.fillStyle = '#fff'; ctx.strokeStyle = '#fff';
    draw(ctx);
    const px = ctx.getImageData(0, 0, w, h).data, m = new Uint8Array(w * h);
    for (let k = 0; k < m.length; k++) m[k] = px[k * 4 + 3] > 127 ? 1 : 0;
    return { w, h, res, m };
  }

  // distance (in samples) from every sample to the nearest masked one (Felzenszwalb and Huttenlocher)
  function edt(M) {
    const { w, h, m } = M, INF = 1e20, g = new Float64Array(w * h);
    for (let k = 0; k < g.length; k++) g[k] = m[k] ? 0 : INF;
    const n = Math.max(w, h), f = new Float64Array(n), d = new Float64Array(n), v = new Int32Array(n), z = new Float64Array(n + 1);
    const pass = (len) => {
      let k = 0; v[0] = 0; z[0] = -INF; z[1] = INF;
      for (let q = 1; q < len; q++) {
        let s;
        while (true) {
          s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k]);
          if (s <= z[k]) { k--; continue; }
          break;
        }
        k++; v[k] = q; z[k] = s; z[k + 1] = INF;
      }
      k = 0;
      for (let q = 0; q < len; q++) { while (z[k + 1] < q) k++; d[q] = (q - v[k]) * (q - v[k]) + f[v[k]]; }
    };
    for (let x = 0; x < w; x++) { for (let y = 0; y < h; y++) f[y] = g[y * w + x]; pass(h); for (let y = 0; y < h; y++) g[y * w + x] = d[y]; }
    for (let y = 0; y < h; y++) { for (let x = 0; x < w; x++) f[x] = g[y * w + x]; pass(w); for (let x = 0; x < w; x++) g[y * w + x] = d[x]; }
    const out = new Float32Array(w * h);
    for (let k = 0; k < out.length; k++) out[k] = Math.sqrt(g[k]);
    return out;
  }

  function blur(field, w, h, r) {
    const tmp = new Float32Array(field.length), out = new Float32Array(field.length), n = 2 * r + 1;
    for (let y = 0; y < h; y++) {
      let s = 0;
      for (let x = -r; x <= r; x++) s += field[y * w + Math.min(w - 1, Math.max(0, x))];
      for (let x = 0; x < w; x++) {
        tmp[y * w + x] = s / n;
        s += field[y * w + Math.min(w - 1, x + r + 1)] - field[y * w + Math.max(0, x - r)];
      }
    }
    for (let x = 0; x < w; x++) {
      let s = 0;
      for (let y = -r; y <= r; y++) s += tmp[Math.min(h - 1, Math.max(0, y)) * w + x];
      for (let y = 0; y < h; y++) {
        out[y * w + x] = s / n;
        s += tmp[Math.min(h - 1, y + r + 1) * w + x] - tmp[Math.max(0, y - r) * w + x];
      }
    }
    return out;
  }

  // the lines where a field crosses each threshold, in plate px, cut where they run along the edge
  function isolines(field, M, thresholds, keep = () => true) {
    const { w, h, res } = M;
    const out = [];
    for (const t of thresholds) {
      const [geom] = d3.contours().size([w, h]).thresholds([t])(field);
      const lv = [];
      for (const poly of geom.coordinates) for (const ring of poly) {
        let cur = [];
        for (const [x, y] of ring) {
          const edge = x <= 0.5 || y <= 0.5 || x >= w - 0.5 || y >= h - 0.5;
          const p = [x * res, y * res];
          if (edge || !keep(p)) { if (cur.length > 1) lv.push(cur); cur = []; } else cur.push(p);
        }
        if (cur.length > 1) lv.push(cur);
      }
      out.push(lv);
    }
    return out;
  }

  // ---- polylines -------------------------------------------------------------------------------------
  function length(l) { let s = 0; for (let k = 1; k < l.length; k++) s += Math.hypot(l[k][0] - l[k - 1][0], l[k][1] - l[k - 1][1]); return s; }
  // points every `step` along the line (the last one at its end)
  function resample(l, step) {
    const out = [l[0].slice()];
    let carry = 0;
    for (let k = 1; k < l.length; k++) {
      const [x0, y0] = l[k - 1], [x1, y1] = l[k], d = Math.hypot(x1 - x0, y1 - y0);
      let t = step - carry;
      while (t <= d) { out.push([x0 + (x1 - x0) * t / d, y0 + (y1 - y0) * t / d]); t += step; }
      carry = d - (t - step);
    }
    const last = l[l.length - 1], p = out[out.length - 1];
    if (Math.hypot(last[0] - p[0], last[1] - p[1]) > step * 0.35) out.push(last.slice());
    return out;
  }
  function chaikin(l, n = 2, closed = false) {
    for (let r = 0; r < n; r++) {
      const o = closed ? [] : [l[0]];
      const m = closed ? l.length : l.length - 1;
      for (let k = 0; k < m; k++) {
        const a = l[k], b = l[(k + 1) % l.length];
        o.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25], [a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
      }
      if (!closed) o.push(l[l.length - 1]);
      l = o;
    }
    return l;
  }
  // Douglas–Peucker
  function simplify(l, tol) {
    if (l.length < 3) return l;
    const keep = new Uint8Array(l.length); keep[0] = keep[l.length - 1] = 1;
    const st = [[0, l.length - 1]];
    while (st.length) {
      const [a, b] = st.pop();
      let md = 0, mi = -1;
      const [ax, ay] = l[a], [bx, by] = l[b], dx = bx - ax, dy = by - ay, L = Math.hypot(dx, dy);
      for (let k = a + 1; k < b; k++) {
        // a closed ring's ends coincide: measure from the point itself
        const d = L < 1e-6 ? Math.hypot(l[k][0] - ax, l[k][1] - ay) : Math.abs((l[k][0] - ax) * dy - (l[k][1] - ay) * dx) / L;
        if (d > md) { md = d; mi = k; }
      }
      if (md > tol) { keep[mi] = 1; st.push([a, mi], [mi, b]); }
    }
    return l.filter((_, k) => keep[k]);
  }
  const onPlate = (l, pad = 0) => l.some(([x, y]) => x > -pad && y > -pad && x < W + pad && y < H + pad);

  // ---- stitches along lines --------------------------------------------------------------------------
  // backstitch: stitches end to end, sharing holes
  function backstitch(S, l, o) {
    const r = rng(o.seed || 1);
    const pts = resample(l, o.len);
    for (let k = 1; k < pts.length; k++) {
      const j = o.jitter || 0;
      const a = pts[k - 1], b = pts[k];
      S.push(a[0] + (r() - 0.5) * j, a[1] + (r() - 0.5) * j, b[0] + (r() - 0.5) * j, b[1] + (r() - 0.5) * j, o.hw, o.col, 0,
        { tier: o.tier, taper: o.taper ?? 0.55, mode: o.mode, shadow: o.shadow });
    }
  }
  // running stitch: a stitch, a gap
  function running(S, l, o) {
    const r = rng(o.seed || 1);
    const L = length(l);
    if (L < o.len) return;
    let pos = (o.phase ?? r()) * (o.len + o.gap);
    const at = mkAt(l);
    while (pos + o.len * 0.6 < L) {
      const len = o.len * (1 + ((r() - 0.5) * (o.vary ?? 0.2)));
      const a = at(pos), b = at(Math.min(L, pos + len));
      const w = (r() - 0.5) * (o.wobble ?? 0.6);
      S.push(a[0], a[1] + w, b[0], b[1] + w, o.hw, o.col, 0, { tier: o.tier, taper: o.taper ?? 0.6, mode: o.mode, shadow: o.shadow });
      pos += len + o.gap * (1 + (r() - 0.5) * 0.3);
    }
  }
  function mkAt(l) {
    const cum = [0];
    for (let k = 1; k < l.length; k++) cum.push(cum[k - 1] + Math.hypot(l[k][0] - l[k - 1][0], l[k][1] - l[k - 1][1]));
    let k = 1;
    return s => {
      if (s < cum[k - 1]) k = 1;
      while (k < l.length - 1 && cum[k] < s) k++;
      const d = cum[k] - cum[k - 1] || 1e-9, t = Math.max(0, Math.min(1, (s - cum[k - 1]) / d));
      return [l[k - 1][0] + (l[k][0] - l[k - 1][0]) * t, l[k - 1][1] + (l[k][1] - l[k - 1][1]) * t];
    };
  }
  // couching: a laid thread along the line, held by small stitches across it
  function couch(S, l, o) {
    const pts = resample(l, Math.max(2, o.hw * 1.6));
    let s0 = 0;
    for (let k = 1; k < pts.length; k++) {
      const a = pts[k - 1], b = pts[k];
      S.push(a[0], a[1], b[0], b[1], o.hw, o.col, 1, { s0, phase: o.phase ?? 0.3, tier: o.tier, shadow: o.shadow });
      s0 += Math.hypot(b[0] - a[0], b[1] - a[1]);
    }
    if (!o.tack) return;
    const at = mkAt(pts), L = length(pts);
    for (let s = (o.tackPhase ?? 0.5) * o.every; s < L; s += o.every) {
      const p = at(s), q = at(Math.min(L, s + 1)), dx = q[0] - p[0], dy = q[1] - p[1], d = Math.hypot(dx, dy) || 1;
      const nx = -dy / d, ny = dx / d, h = o.hw * 1.45, sk = o.hw * 0.5;
      S.push(p[0] - nx * h - dx / d * sk, p[1] - ny * h - dy / d * sk, p[0] + nx * h + dx / d * sk, p[1] + ny * h + dy / d * sk,
        o.tackHw || o.hw * 0.38, o.tack, 0, { tier: (o.tier || 0) + 1, taper: 0.4 });
    }
  }
  // stem stitch: overlapping slanted stitches, a rope-like line (vines, stems)
  function stem(S, l, o) {
    const pts = resample(l, o.len / 2);
    for (let k = 2; k < pts.length; k += 1) {
      const a = pts[k - 2], b = pts[k], dx = b[0] - a[0], dy = b[1] - a[1], d = Math.hypot(dx, dy) || 1;
      const nx = -dy / d * o.hw * 0.7, ny = dx / d * o.hw * 0.7;
      S.push(a[0] - nx, a[1] - ny, b[0] + nx, b[1] + ny, o.hw, o.col, 0, { tier: o.tier, taper: 0.45, mode: 1 });
    }
  }
  // satin: parallel stitches across a shape at an angle; long spans break into staggered rows
  function satin(S, rings, o) {
    const ang = o.angle || 0, ca = Math.cos(ang), sa = Math.sin(ang);
    // rotate the shape so the stitches run along x
    const rot = rings.map(r => r.map(([x, y]) => [x * ca + y * sa, -x * sa + y * ca]));
    let y0 = Infinity, y1 = -Infinity;
    for (const r of rot) for (const p of r) { y0 = Math.min(y0, p[1]); y1 = Math.max(y1, p[1]); }
    const r = rng(o.seed || 3);
    let row = 0;
    for (let y = y0 + o.spacing / 2; y < y1; y += o.spacing, row++) {
      const xs = [];
      for (const ring of rot) for (let k = 1; k < ring.length; k++) {
        const [ax, ay] = ring[k - 1], [bx, by] = ring[k];
        if ((ay <= y) !== (by <= y)) xs.push(ax + (y - ay) / (by - ay) * (bx - ax));
      }
      xs.sort((a, b) => a - b);
      for (let k = 0; k + 1 < xs.length; k += 2) {
        let a = xs[k] + (o.inset || 0), b = xs[k + 1] - (o.inset || 0);
        if (b - a < (o.min || 2)) continue;
        // long spans: brick rows, the joins staggered from one row to the next
        const max = o.maxLen || 1e9, n = Math.ceil((b - a) / max), seg = (b - a) / n;
        let off = n > 1 ? ((row % 3) / 3 - 0.33) * seg : 0;
        let x = a;
        const cuts = [a];
        for (let q = 1; q < n; q++) cuts.push(a + q * seg + off * (0.8 + r() * 0.4));
        cuts.push(b);
        for (let q = 1; q < cuts.length; q++) {
          const xa = cuts[q - 1] + (q > 1 ? 0.6 : 0), xb = cuts[q] - (q < cuts.length - 1 ? 0.6 : 0);
          if (xb - xa < 1) continue;
          const j = o.jitter || 0;
          const pa = [xa + (r() - 0.5) * j, y + (r() - 0.5) * j * 0.3], pb = [xb + (r() - 0.5) * j, y + (r() - 0.5) * j * 0.3];
          const wa = [pa[0] * ca - pa[1] * sa, pa[0] * sa + pa[1] * ca], wb = [pb[0] * ca - pb[1] * sa, pb[0] * sa + pb[1] * ca];
          const col = typeof o.col === 'function' ? o.col(wa, wb, r) : o.col;
          S.push(wa[0], wa[1], wb[0], wb[1],
            o.hw, col, 0, { tier: o.tier, taper: o.taper ?? 0.35, mode: o.mode ?? 1, shadow: o.shadow ?? 0.7 });
        }
        x = b;
      }
    }
  }
  function knot(S, x, y, r, col, o = {}) { S.push(x, y, x, y, r, col, 2, { tier: o.tier ?? 1, phase: o.phase, shadow: o.shadow }); }

  // ---- charts ------------------------------------------------------------------------------------------
  // An ASCII chart stamped into a grid at cell (i0, j0): each character names a colour in the legend,
  // '.' and ' ' are empty.
  function stamp(G, chart, legend, i0, j0, opts = {}) {
    const rows = typeof chart === 'string' ? chart.replace(/^\n+|\n+$/g, '').split('\n') : chart;
    rows.forEach((row, j) => {
      for (let i = 0; i < row.length; i++) {
        const ch = row[opts.mirror ? row.length - 1 - i : i];
        const col = legend[ch];
        if (!col) continue;
        const ii = i0 + i, jj = j0 + j;
        if (ii < 0 || jj < 0 || ii >= G.ncol || jj >= G.nrow) continue;
        G.data[jj * G.ncol + ii] = col;
      }
    });
    return { w: Math.max(...rows.map(r => r.length)), h: rows.length };
  }
  function chartSize(chart) {
    const rows = chart.replace(/^\n+|\n+$/g, '').split('\n');
    return { w: Math.max(...rows.map(r => r.length)), h: rows.length };
  }
  // the cell of a plate point
  const cellAt = (G, x, y) => [Math.floor((x - G.ox) / G.c), Math.floor((y - G.oy) / G.c)];

  return { rng, lines, grid, coverage, fill, mask, edt, blur, isolines, length, resample, chaikin, simplify, onPlate,
    backstitch, running, couch, stem, satin, knot, stamp, chartSize, cellAt, mkAt };
})();
window.Geo = Geo;
