// The sand table's physics: a height field of sand (Cannae's relief, exaggerated) that the finger
// grooves and that slumps to its angle of repose; two armies of coloured grain (red ochre for Rome,
// pale lime for Carthage's foot, black seed for Hasdrubal's horse) as particles that slide downhill
// into the grooves, are carried along them, press on one another, kick up tracks, and churn where
// they meet. Crude by design: it only has to make the look happen.
(function () {
  const W = 512, H = 384, N = W * H;
  const FW = 128, FH = 96, FS = 4;            // flow grid (grooves' direction)
  const TALUS = 0.62;                          // angle of repose, height per cell
  const base = new Float32Array(N), wet = new Float32Array(N), blur = new Float32Array(N);
  const dens = [new Float32Array(N), new Float32Array(N), new Float32Array(N)];
  const stain = [new Float32Array(N), new Float32Array(N), new Float32Array(N)];
  const churn = new Float32Array(N), tmp = new Float32Array(N);
  const flowX = new Float32Array(FW * FH), flowY = new Float32Array(FW * FH), flowW = new Float32Array(FW * FH);
  let seed = 12345;
  const rnd = () => {                         // mulberry32: an LCG's pairs fall on lines
    let t = (seed = (seed + 0x6D2B79F5) | 0); t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
  const gauss = () => { let u = 0; for (let i = 0; i < 4; i++) u += rnd(); return (u - 2) * 0.87; };

  // ---- the ground -------------------------------------------------------------------------
  const R = window.RELIEF;
  function reliefAt(x, y) {                   // metres, bilinear, x,y in sim cells
    const fx = Math.min(R.w - 1.001, Math.max(0, x / W * R.w - 0.5)), fy = Math.min(R.h - 1.001, Math.max(0, y / H * R.h - 0.5));
    const ix = fx | 0, iy = fy | 0, ax = fx - ix, ay = fy - iy, m = R.m;
    return (m[iy * R.w + ix] * (1 - ax) + m[iy * R.w + ix + 1] * ax) * (1 - ay) +
           (m[(iy + 1) * R.w + ix] * (1 - ax) + m[(iy + 1) * R.w + ix + 1] * ax) * ay;
  }
  let mn = 1e9, mx = -1e9;
  for (let i = 0; i < R.m.length; i++) { mn = Math.min(mn, R.m[i]); mx = Math.max(mx, R.m[i]); }
  const RELIEF_H = 16;                        // ~100 m of relief as 16 cells (~2.6x exaggeration)
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++)
    base[y * W + x] = 3 + (reliefAt(x, y) - mn) / (mx - mn) * RELIEF_H;

  // the Aufidus: the valley's lowest line, column by column, smoothed, given back its meanders
  // The plain here slopes gently north to the sea, so the lowest line isn't the channel at this
  // resolution: the river is laid along the foot of the right-bank terrace (Canne's spur), read off
  // the relief by eye, as the Ofanto runs below it today.
  const KN = [[-20, 250], [0, 253], [171, 286], [256, 268], [341, 214], [427, 156], [512, 108], [532, 100]];
  const ry = new Float32Array(W);
  for (let x = 0; x < W; x++) {
    let k = 1; while (KN[k][0] < x) k++;
    const a = (x - KN[k - 1][0]) / (KN[k][0] - KN[k - 1][0]);
    ry[x] = KN[k - 1][1] + (KN[k][1] - KN[k - 1][1]) * a;
  }
  for (let k = 0; k < 6; k++) { const c = ry.slice(); for (let x = 0; x < W; x++) { let s = 0, n = 0; for (let d = -10; d <= 10; d++) { const xx = x + d; if (xx >= 0 && xx < W) { s += c[xx]; n++; } } ry[x] = s / n; } }
  for (let x = 0; x < W; x++) ry[x] += 7 * Math.sin(x * 0.045 + 1.3) + 3 * Math.sin(x * 0.11 + 0.4);
  const river = [];
  for (let x = -10; x <= W + 10; x += 2) river.push([x, ry[Math.max(0, Math.min(W - 1, x))]]);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    let d = 1e9;
    for (let k = Math.max(0, ((x + 10) >> 1) - 30); k < Math.min(river.length, ((x + 10) >> 1) + 30); k++) {
      const dx = x - river[k][0], dy = y - river[k][1]; d = Math.min(d, dx * dx + dy * dy);
    }
    d = Math.sqrt(d);
    const i = y * W + x;
    base[i] -= 2.2 * Math.exp(-(d * d) / (2 * 3.2 * 3.2)) + 0.6 * Math.exp(-(d * d) / (2 * 9 * 9));
    wet[i] = Math.min(1, Math.max(0, 1.25 - d / 4.2)) + 0.45 * Math.exp(-(d * d) / (2 * 10 * 10));
  }
  // a little hand-smoothed unevenness, then let it settle
  for (let i = 0; i < N; i++) base[i] += (rnd() - 0.5) * 0.08;
  for (let k = 0; k < 40; k++) slump(base, 0.5);

  function boxBlur(a, r) {
    for (let y = 0; y < H; y++) {             // rows
      let s = 0; const o = y * W;
      for (let x = -r; x <= r; x++) s += a[o + Math.max(0, Math.min(W - 1, x))];
      for (let x = 0; x < W; x++) {
        tmp[o + x] = s / (2 * r + 1);
        s += a[o + Math.min(W - 1, x + r + 1)] - a[o + Math.max(0, x - r)];
      }
    }
    for (let x = 0; x < W; x++) {             // columns
      let s = 0;
      for (let y = -r; y <= r; y++) s += tmp[Math.max(0, Math.min(H - 1, y)) * W + x];
      for (let y = 0; y < H; y++) {
        a[y * W + x] = s / (2 * r + 1);
        s += tmp[Math.min(H - 1, y + r + 1) * W + x] - tmp[Math.max(0, y - r) * W + x];
      }
    }
  }
  function slump(a, rate) {                   // relax every slope steeper than the angle of repose
    for (let y = 0; y < H - 1; y++) for (let x = 0; x < W - 1; x++) {
      const i = y * W + x;
      let d = a[i] - a[i + 1];
      if (d > TALUS) { const m = (d - TALUS) * 0.25 * rate; a[i] -= m; a[i + 1] += m; }
      else if (d < -TALUS) { const m = (-d - TALUS) * 0.25 * rate; a[i] += m; a[i + 1] -= m; }
      d = a[i] - a[i + W];
      if (d > TALUS) { const m = (d - TALUS) * 0.25 * rate; a[i] -= m; a[i + W] += m; }
      else if (d < -TALUS) { const m = (-d - TALUS) * 0.25 * rate; a[i] += m; a[i + W] -= m; }
    }
  }
  const at = (a, x, y) => {
    x = Math.max(0, Math.min(W - 1.001, x)); y = Math.max(0, Math.min(H - 1.001, y));
    const ix = x | 0, iy = y | 0, ax = x - ix, ay = y - iy, i = iy * W + ix;
    return (a[i] * (1 - ax) + a[i + 1] * ax) * (1 - ay) + (a[i + W] * (1 - ax) + a[i + W + 1] * ax) * ay;
  };

  // ---- the armies -------------------------------------------------------------------------
  // 2 August 216 BC: the Romans deep in the centre, the river on their right; Hannibal's crescent
  // bulging toward them, the Africans held back on both wings, Hasdrubal's horse by the river.
  const MAXG = 14000;
  const gx = new Float32Array(MAXG), gy = new Float32Array(MAXG), vx = new Float32Array(MAXG), vy = new Float32Array(MAXG);
  const mat = new Uint8Array(MAXG), side = new Uint8Array(MAXG), roused = new Uint8Array(MAXG), fallen = new Uint8Array(MAXG);
  let G = 0;
  // the field: on the valley floor, the river along the Romans' right
  // Romans on the plain north of the river line, facing down the valley; the river on their right.
  const cx0 = 262, rdx = ry[cx0 + 20] - ry[cx0 - 20], rl = Math.hypot(40, rdx);
  const F = [40 / rl, rdx / rl];              // Roman forward: down the valley, along the river
  const Rt = [-F[1], F[0]];                   // Roman right: toward the river
  const C = [cx0, ry[cx0] - 104];
  const K = 1.5;                              // the armies' scale on the table
  function add(m, s, x, y) { if (G >= MAXG) return; gx[G] = x; gy[G] = y; vx[G] = vy[G] = 0; mat[G] = m; side[G] = s; roused[G] = s === 0 ? 1 : 0; G++; }
  function block(m, s, cxl, cyl, wR, dF, n) { // centre in (right, forward) coords from C
    for (let k = 0; k < n; k++) {
      const a = (rnd() - 0.5) * wR + gauss() * 1.2, b = (rnd() - 0.5) * dF + gauss() * 1.2;
      const r = (cxl + a) * K, f = (cyl + b) * K;
      add(m, s, C[0] + Rt[0] * r + F[0] * f, C[1] + Rt[1] * r + F[1] * f);
    }
  }
  block(0, 0, 0, -32, 74, 30, 5200);          // the legions, deep
  block(0, 0, 42, -28, 10, 14, 380);          // Roman horse, by the river
  block(0, 0, -50, -28, 14, 16, 520);         // allied horse
  for (let k = 0; k < 3200; k++) {            // the crescent, bowed toward Rome
    const t = (rnd() - 0.5) * 2, r = t * 40 * K, f = (14 - 9 * t * t + gauss() * 3) * K;
    add(1, 1, C[0] + Rt[0] * r + F[0] * f, C[1] + Rt[1] * r + F[1] * f);
  }
  block(1, 1, 48, 6, 10, 18, 700);            // Africans, river side
  block(1, 1, -48, 6, 10, 18, 700);           // Africans, far side
  block(2, 1, 52, 10, 11, 12, 900);           // Hasdrubal's horse by the river
  block(2, 1, -66, 10, 12, 12, 420);          // Numidians

  // ---- the finger -------------------------------------------------------------------------
  let last = null;
  function carve(x, y, tx, ty) {
    const r = 4.2, depth = 0.9;
    let vol = 0;
    const x0 = Math.max(1, (x - 3 * r) | 0), x1 = Math.min(W - 2, (x + 3 * r) | 0);
    const y0 = Math.max(1, (y - 3 * r) | 0), y1 = Math.min(H - 2, (y + 3 * r) | 0);
    const ring = [];
    let rw = 0;
    for (let j = y0; j <= y1; j++) for (let i = x0; i <= x1; i++) {
      const d = Math.hypot(i - x, j - y) / r, k = j * W + i;
      if (d < 1) { const dz = depth * (1 - d * d) * 0.35; base[k] -= dz; vol += dz; churn[k] = Math.min(1, churn[k] + 0.2); }
      else if (d < 2.1) {                     // the displaced sand heaps up on either side, mostly
        const s = Math.abs((i - x) * ty - (j - y) * tx) / (Math.hypot(i - x, j - y) + 1e-6);
        const w = Math.exp(-((d - 1.35) ** 2) / 0.09) * (0.35 + 0.65 * s); ring.push(k, w); rw += w;
      }
    }
    for (let q = 0; q < ring.length; q += 2) base[ring[q]] += vol * ring[q + 1] / rw;
    // the groove's direction, for the grain it will carry
    const fx = x / FS, fy = y / FS;
    for (let j = Math.max(0, (fy - 3) | 0); j <= Math.min(FH - 1, (fy + 3) | 0); j++)
      for (let i = Math.max(0, (fx - 3) | 0); i <= Math.min(FW - 1, (fx + 3) | 0); i++) {
        const d = Math.hypot(i + 0.5 - fx, j + 0.5 - fy), w = Math.max(0, 1 - d / 2.2);
        if (w <= 0) continue;
        const k = j * FW + i, a = Math.min(1, w * 0.8);
        flowX[k] = flowX[k] * (1 - a) + tx * a; flowY[k] = flowY[k] * (1 - a) + ty * a;
        flowW[k] = Math.max(flowW[k], w);
      }
    // grain under the finger is ploughed aside
    for (let g = 0; g < G; g++) {
      const dx = gx[g] - x, dy = gy[g] - y, d = Math.hypot(dx, dy);
      if (d < r * 1.3) { const s = (dx * ty - dy * tx) >= 0 ? 1 : -1; vx[g] += ty * s * 6 + tx * 3; vy[g] += -tx * s * 6 + ty * 3; }
    }
  }
  function pointer(type, x, y) {              // x, y in cells
    if (type === 'down') { last = [x, y, false]; return; }
    if (!last) return;
    if (type === 'move') {
      const dx = x - last[0], dy = y - last[1], d = Math.hypot(dx, dy);
      if (d < 0.6) return;
      const tx = dx / d, ty = dy / d, steps = Math.ceil(d / 0.8);
      for (let s = 1; s <= steps; s++) carve(last[0] + dx * s / steps, last[1] + dy * s / steps, tx, ty);
      last = [x, y, true];
    } else if (type === 'up') {
      if (!last[2]) dimple(x, y);
      last = null;
    }
  }
  function dimple(x, y) {                     // a fingertip pressed in: a little crater, grains scatter
    for (let j = Math.max(1, (y - 9) | 0); j < Math.min(H - 1, y + 9); j++) for (let i = Math.max(1, (x - 9) | 0); i < Math.min(W - 1, x + 9); i++) {
      const d = Math.hypot(i - x, j - y) / 4.5, k = j * W + i;
      base[k] += d < 1 ? -1.1 * (1 - d * d) : 0.55 * Math.exp(-((d - 1.3) ** 2) / 0.08);
    }
    for (let g = 0; g < G; g++) {
      const dx = gx[g] - x, dy = gy[g] - y, d = Math.hypot(dx, dy);
      if (d < 14) { const s = (14 - d) * 1.6 / (d + 1); vx[g] += dx * s; vy[g] += dy * s; roused[g] = 1; }
    }
  }

  // ---- a step -----------------------------------------------------------------------------
  let time = 0;
  const cen = [[0, 0], [0, 0]];
  function step(dt) {
    time += dt;
    for (let m = 0; m < 3; m++) dens[m].fill(0);
    cen[0] = [0, 0, 0]; cen[1] = [0, 0, 0];
    for (let g = 0; g < G; g++) {
      const x = Math.max(0, Math.min(W - 1.001, gx[g])), y = Math.max(0, Math.min(H - 1.001, gy[g]));
      const ix = x | 0, iy = y | 0, ax = x - ix, ay = y - iy, i = iy * W + ix, d = dens[mat[g]];
      d[i] += (1 - ax) * (1 - ay); d[i + 1] += ax * (1 - ay); d[i + W] += (1 - ax) * ay; d[i + W + 1] += ax * ay;
      const c = cen[side[g]]; c[0] += x; c[1] += y; c[2]++;
    }
    for (let s = 0; s < 2; s++) { cen[s][0] /= cen[s][2]; cen[s][1] /= cen[s][2]; }
    for (let m = 0; m < 3; m++) boxBlur(dens[m], 1);
    const D0 = dens[0];
    for (let g = 0; g < G; g++) {
      if (fallen[g]) { vx[g] *= 0.8; vy[g] *= 0.8; gx[g] += vx[g] * dt; gy[g] += vy[g] * dt; continue; }
      const x = gx[g], y = gy[g], s = side[g];
      let ax = 0, ay = 0;
      // downhill on the sand: into the grooves
      const hx = at(base, x + 1, y) - at(base, x - 1, y), hy = at(base, x, y + 1) - at(base, x, y - 1);
      ax -= hx * 9; ay -= hy * 9;
      // but not into the river: grain keeps to the dry bank
      const wx = at(wet, x + 1.5, y) - at(wet, x - 1.5, y), wy = at(wet, x, y + 1.5) - at(wet, x, y - 1.5);
      ax -= wx * 160; ay -= wy * 160;
      // press of the crowd
      const tot = (xx, yy) => at(dens[0], xx, yy) + at(dens[1], xx, yy) + at(dens[2], xx, yy);
      const px = tot(x + 1.5, y) - tot(x - 1.5, y), py = tot(x, y + 1.5) - tot(x, y - 1.5);
      ax -= px * 5; ay -= py * 5;
      // carried along a groove
      const fi = Math.min(FW - 1, Math.max(0, (x / FS) | 0)), fj = Math.min(FH - 1, Math.max(0, (y / FS) | 0)), fk = fj * FW + fi;
      const fw = flowW[fk];
      if (fw > 0.15) {
        const fl = Math.hypot(flowX[fk], flowY[fk]) + 1e-6;
        const fp = mat[g] === 2 ? 120 : 60; ax += flowX[fk] / fl * fp * fw; ay += flowY[fk] / fl * fp * fw; roused[g] = 1;
      } else if (roused[g]) {                 // otherwise toward the enemy
        const e = cen[1 - s], dx = e[0] - x, dy = e[1] - y, d = Math.hypot(dx, dy) + 1e-6;
        const sp = s === 0 ? 2.2 : 3.5;
        ax += dx / d * sp; ay += dy / d * sp;
      }
      // where they meet
      const own = s === 0 ? at(D0, x, y) : at(dens[1], x, y) + at(dens[2], x, y);
      const foe = s === 0 ? at(dens[1], x, y) + at(dens[2], x, y) : at(D0, x, y);
      if (foe > 0.12) {
        ax += gauss() * 30; ay += gauss() * 30;
        vx[g] *= 0.86; vy[g] *= 0.86;
        const i = (y | 0) * W + (x | 0);
        if (i >= 0 && i < N) {
          churn[i] = Math.min(1, churn[i] + 0.05);
          stain[0][i] = Math.min(1, stain[0][i] + 0.01); stain[mat[g] === 0 ? 1 : mat[g]][i] = Math.min(1, stain[mat[g] === 0 ? 1 : mat[g]][i] + 0.01);
          if (rnd() < 0.08) { base[i] -= 0.03; base[i + (rnd() < 0.5 ? 1 : W)] += 0.03; }
        }
        if (rnd() < 0.0035 * foe / (own + 0.2) * (s === 0 ? 1.5 : 0.8)) fallen[g] = 1;
      }
      vx[g] = vx[g] * 0.9 + ax * dt; vy[g] = vy[g] * 0.9 + ay * dt;
      const sp = Math.hypot(vx[g], vy[g]), cap = mat[g] === 2 ? 40 : 16;
      if (sp > cap) { vx[g] *= cap / sp; vy[g] *= cap / sp; }
      gx[g] = Math.max(1, Math.min(W - 2, x + vx[g] * dt)); gy[g] = Math.max(1, Math.min(H - 2, y + vy[g] * dt));
      // tracks: the moving grain kicks the sand aside and leaves a little of its colour
      if (sp > 3) {
        const i = (gy[g] | 0) * W + (gx[g] | 0), k = 0.0009 * Math.min(sp, 15);
        base[i] -= k; const nx = (-vy[g] / sp * 1.6) | 0, ny = (vx[g] / sp * 1.6) | 0;
        base[i + ny * W + nx] += k * 0.5; base[i - ny * W - nx] += k * 0.5;
        stain[mat[g]][i] = Math.min(0.6, stain[mat[g]][i] + 0.004);
      }
    }
    slump(base, 1); slump(base, 1);
    for (let i = 0; i < N; i++) churn[i] *= 0.995;
  }

  // ---- what the globe-less renderer reads ---------------------------------------------------
  const out0 = new Float32Array(N * 4), out1 = new Float32Array(N * 4);
  const VOL = 2.8;                           // a grain's heap height
  function pack() {
    let hmax = 0;
    for (let i = 0; i < N; i++) {
      const a = dens[0][i], b = dens[1][i], c = dens[2][i], t = a + b + c;
      const h = base[i] + t * VOL;
      blur[i] = h; hmax = Math.max(hmax, h);
      const cov = Math.min(1, t * 1.6);
      const sa = stain[0][i], sb = stain[1][i], sc = stain[2][i], st = Math.min(1, sa + sb + sc);
      const k = t > 1e-4 ? cov / t : 0, ks = (1 - cov) * (st > 1e-4 ? st / (sa + sb + sc) : 0);
      out0[i * 4] = h; out0[i * 4 + 1] = a * k + sa * ks; out0[i * 4 + 2] = b * k + sb * ks; out0[i * 4 + 3] = c * k + sc * ks;
      out1[i * 4] = wet[i]; out1[i * 4 + 2] = t * VOL; out1[i * 4 + 3] = churn[i];
    }
    boxBlur(blur, 7);
    for (let i = 0; i < N; i++) out1[i * 4 + 1] = blur[i];
    return { out0, out1, hmax };
  }

  window.SIM = { K, W, H, step, pack, pointer, get time() { return time; }, river, C, F, Rt, cen };
})();
