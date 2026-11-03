// pressure-front: two armies as continua over the ground of Cannae.
//
// Each army is a density field rho (0..1 of a packed formation) on a GWxGH grid. Per step:
//  - will: a potential phi (Dijkstra eikonal, cost = 1/speed) toward the enemy and, far behind it,
//    the army's objective edge; speed falls on slopes and in its own crowd. The player's strokes are
//    channels of will: near a stroke the desire turns along it (and a little toward its centreline),
//    so the ink carries whatever density lies on it, Tsuro-like. A held finger pins: no desire,
//    and the enemy's shove is mostly refused there.
//  - shove: each army is pushed down the gradient of the enemy's widely blurred density, so a deep
//    formation presses harder than a thin one and the front moves toward the thinner side.
//  - cohesion: a pull up the army's own blurred density (surface tension), so a line holds a line.
//  - flow: donor-cell fluxes, capped so no cell holds more than one packed formation; the river
//    and the map's edges pass nothing.
//  - attrition (Lanchester): where the armies touch, each loses in proportion to the enemy's
//    density around it.
//  - morale rides with the density (q = rho*m is advected); enemy on two opposite sides of a cell
//    (pressed front and rear, or both flanks) drains it; broken density stops fighting, loses its
//    cohesion, flees toward its own camp, and dies faster if it can't get there.
(function () {
  const GW = 160, GH = 120, N = GW * GH;
  const DT = 0.2;

  const hyp = (x, y) => Math.sqrt(x * x + y * y);
  function rng(seed) { let s = seed >>> 0; return () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; }; }

  // ---------- ground ----------
  function decodeTerrain() {
    const T = window.TERRAIN, bin = atob(T.b64), n = T.w * T.h, h = new Float32Array(n);
    for (let i = 0; i < n; i++) h[i] = (bin.charCodeAt(2 * i) | (bin.charCodeAt(2 * i + 1) << 8)) / 10;
    return { w: T.w, h: T.h, z: h, m: T.metresPerCell };
  }
  // the river: drawn, not measured (the ancient Aufidus has wandered); it runs up the plain's left
  function riverX(gy) { return 22 + 4.5 * Math.sin(gy / 13 + 0.6) + 2.2 * Math.sin(gy / 5.3 + 2.0) + gy * 0.05; }
  const RIVER_HALF = 1.4;

  function makeGround() {
    const T = decodeTerrain();
    const elev = new Float32Array(N), slow = new Float32Array(N), pass = new Uint8Array(N);
    const sx = T.w / GW;
    for (let y = 0; y < GH; y++) for (let x = 0; x < GW; x++) {
      let s = 0; for (let j = 0; j < sx; j++) for (let i = 0; i < sx; i++) s += T.z[(y * sx + j) * T.w + x * sx + i];
      elev[y * GW + x] = s / (sx * sx);
    }
    const cell = T.m * sx;
    for (let y = 0; y < GH; y++) for (let x = 0; x < GW; x++) {
      const i = y * GW + x;
      const gx = (elev[y * GW + Math.min(GW - 1, x + 1)] - elev[y * GW + Math.max(0, x - 1)]) / (2 * cell);
      const gy = (elev[Math.min(GH - 1, y + 1) * GW + x] - elev[Math.max(0, y - 1) * GW + x]) / (2 * cell);
      const sl = hyp(gx, gy);
      slow[i] = 1 / (1 + 22 * sl);
      const onRiver = Math.abs(x + 0.5 - riverX(y + 0.5)) < RIVER_HALF;
      pass[i] = onRiver || x === 0 || y === 0 || x === GW - 1 || y === GH - 1 ? 0 : 1;
    }
    return { T, elev, slow, pass, cell };
  }

  // ---------- helpers ----------
  const CX = new Int32Array(GW + 40), CY = new Int32Array(GH + 40); // clamped indices, offset 20
  for (let k = 0; k < GW + 40; k++) CX[k] = Math.min(GW - 1, Math.max(0, k - 20));
  for (let k = 0; k < GH + 40; k++) CY[k] = Math.min(GH - 1, Math.max(0, k - 20)) * GW;
  function blur(src, dst, tmp, r) { // separable box, twice (about a gaussian)
    const inv = 1 / (2 * r + 1);
    for (let pass = 0; pass < 2; pass++) {
      const a = pass === 0 ? src : dst;
      for (let y = 0; y < GH; y++) {
        const row = y * GW; let acc = 0;
        for (let x = -r; x <= r; x++) acc += a[row + CX[x + 20]];
        for (let x = 0; x < GW; x++) {
          tmp[row + x] = acc * inv;
          acc += a[row + CX[x + r + 21]] - a[row + CX[x - r + 20]];
        }
      }
      for (let x = 0; x < GW; x++) {
        let acc = 0;
        for (let y = -r; y <= r; y++) acc += tmp[CY[y + 20] + x];
        for (let y = 0; y < GH; y++) {
          dst[y * GW + x] = acc * inv;
          acc += tmp[CY[y + r + 21] + x] - tmp[CY[y - r + 20] + x];
        }
      }
    }
  }

  // binary heap Dijkstra over 8 neighbours
  const DX = [1, -1, 0, 0, 1, 1, -1, -1], DY = [0, 0, 1, -1, 1, -1, 1, -1], DL = [1, 1, 1, 1, Math.SQRT2, Math.SQRT2, Math.SQRT2, Math.SQRT2];
  function dijkstra(phi, seedCost, cost, pass) {
    const heapI = new Int32Array(N * 8), heapV = new Float32Array(N * 8); let hn = 0;
    const push = (i, v) => { let k = hn++; while (k > 0) { const p = (k - 1) >> 1; if (heapV[p] <= v) break; heapI[k] = heapI[p]; heapV[k] = heapV[p]; k = p; } heapI[k] = i; heapV[k] = v; };
    const pop = () => { const i = heapI[0], v = heapV[0]; const li = heapI[--hn], lv = heapV[hn]; let k = 0;
      for (;;) { let c = 2 * k + 1; if (c >= hn) break; if (c + 1 < hn && heapV[c + 1] < heapV[c]) c++; if (heapV[c] >= lv) break; heapI[k] = heapI[c]; heapV[k] = heapV[c]; k = c; }
      heapI[k] = li; heapV[k] = lv; return [i, v]; };
    phi.fill(1e9);
    for (let i = 0; i < N; i++) if (seedCost[i] < 1e8 && pass[i]) { phi[i] = seedCost[i]; push(i, seedCost[i]); }
    while (hn > 0) {
      const [i, v] = pop(); if (v > phi[i]) continue;
      const x = i % GW, y = (i / GW) | 0;
      for (let k = 0; k < 8; k++) {
        const nx = x + DX[k], ny = y + DY[k]; if (nx < 0 || ny < 0 || nx >= GW || ny >= GH) continue;
        const j = ny * GW + nx; if (!pass[j]) continue;
        const nv = v + DL[k] * 0.5 * (cost[i] + cost[j]);
        if (nv < phi[j]) { phi[j] = nv; push(j, nv); }
      }
    }
  }
  function descend(phi, pass, ox, oy) { // steepest-descent unit directions
    for (let y = 1; y < GH - 1; y++) for (let x = 1; x < GW - 1; x++) {
      const i = y * GW + x; let best = 0, bx = 0, by = 0;
      for (let k = 0; k < 8; k++) {
        const j = (y + DY[k]) * GW + x + DX[k]; if (!pass[j]) continue;
        const d = (phi[i] - phi[j]) / DL[k]; if (d > best) { best = d; bx = DX[k] / DL[k]; by = DY[k] / DL[k]; }
      }
      // soften with the central gradient so directions aren't stuck on 8 compass points
      const gx = (Math.min(phi[i + 1], 1e6) - Math.min(phi[i - 1], 1e6)), gy = (Math.min(phi[i + GW], 1e6) - Math.min(phi[i - GW], 1e6));
      const gl = hyp(gx, gy);
      let ux = bx, uy = by;
      if (gl > 1e-6 && gl < 1e5) { ux = 0.5 * bx - 0.5 * gx / gl; uy = 0.5 * by - 0.5 * gy / gl; }
      const l = hyp(ux, uy); ox[i] = l > 1e-6 ? ux / l : 0; oy[i] = l > 1e-6 ? uy / l : 0;
    }
  }

  // ---------- the world ----------
  function World(opts) {
    const G = makeGround();
    const rand = rng(opts && opts.seed || 216);
    const self = this;
    this.G = G; this.GW = GW; this.GH = GH; this.t = 0; this.step = 0;
    const F = () => new Float32Array(N);
    const tmp = F(), tmp2 = F();
    this.dead = F(); // where men fell: the ground remembers
    this.contact = F();
    function army(name, home, kill, speed) {
      return { name, home, kill, speed, rho: F(), q: F(), phi: F(), dx: F(), dy: F(), vx: F(), vy: F(),
        P: F(), C: F(), near: F(), homePhi: F(), hx: F(), hy: F(), sx: F(), sy: F(), sw: F(), pin: F(),
        strokes: [], pins: [], lost: 0, start: 0 };
    }
    const R = army('rome', 'top', 0.12, 1.0), K = army('carthage', 'bottom', 0.32, 1.0);
    R.ahead = 0.6; K.ahead = 0.0;
    R.nerve = 1.0; K.nerve = 0.45; // raw levies; Hannibal's veterans
    this.armies = [R, K];

    // home potentials (for routing) and edge objectives
    for (const A of this.armies) {
      const seed = F().fill(1e9);
      for (let x = 1; x < GW - 1; x++) seed[(A.home === 'top' ? 1 : GH - 2) * GW + x] = 0;
      const c = F(); for (let i = 0; i < N; i++) c[i] = 1 / G.slow[i];
      dijkstra(A.homePhi, seed, c, G.pass); descend(A.homePhi, G.pass, A.hx, A.hy);
    }

    // ---------- deployment (2 August 216 BC, as Polybius and Livy give it) ----------
    const OY = 14; // the whole field a little down the valley, so the battle sits in the middle of the glass
    const put = (A, x0, y0, x1, y1, d) => {
      for (let y = y0 + OY; y < y1 + OY; y++) for (let x = x0; x < x1; x++) { const i = y * GW + x; if (G.pass[i]) A.rho[i] = Math.min(1, A.rho[i] + d * (0.9 + 0.2 * rand())); }
    };
    // Rome: the infantry massed deep in the centre, its horse on the wings (citizens by the river)
    put(R, 38, 15, 106, 34, 0.74);
    put(R, 29, 20, 37, 26, 0.45);    // Roman cavalry, by the river
    put(R, 107, 18, 121, 24, 0.6);  // allied cavalry
    // Carthage: Gauls and Spaniards in a thin crescent bowed toward Rome, Africans deep at its horns,
    // Hasdrubal's heavy horse by the river, the Numidians on the open right
    for (let x = 48; x < 98; x++) {
      const u = (x - 73) / 25, yc = 51 + OY + 9 * u * u;
      for (let y = Math.floor(yc - 2); y < yc + 2.5; y++) { const i = y * GW + x; K.rho[i] = Math.min(1, K.rho[i] + 0.75 * (0.9 + 0.2 * rand())); }
    }
    put(K, 38, 58, 48, 70, 0.88);   // Africans, left horn
    put(K, 98, 58, 108, 70, 0.88);  // Africans, right horn
    put(K, 26, 60, 37, 67, 0.9);   // Hasdrubal's horse
    put(K, 110, 59, 124, 65, 0.65); // Numidians
    for (const A of this.armies) { for (let i = 0; i < N; i++) { A.q[i] = A.rho[i]; A.start += A.rho[i]; } }

    // slowly moving noise for the will: a little randomness
    const noiseA = F(), noiseB = F();
    for (let i = 0; i < N; i++) { noiseA[i] = rand() * 2 - 1; noiseB[i] = rand() * 2 - 1; }
    blur(noiseA, noiseA, tmp, 3); blur(noiseB, noiseB, tmp, 3);

    // ---------- the finger ----------
    this.armyAt = (gx, gy, rad) => {
      let best = null, bv = 0.04; rad = rad || 3;
      for (const A of this.armies) {
        let s = 0;
        for (let y = Math.max(0, Math.floor(gy) - rad); y <= Math.min(GH - 1, Math.floor(gy) + rad); y++)
          for (let x = Math.max(0, Math.floor(gx) - rad); x <= Math.min(GW - 1, Math.floor(gx) + rad); x++) {
            const d2 = (x - gx) ** 2 + (y - gy) ** 2; s = Math.max(s, A.rho[y * GW + x] * Math.exp(-d2 / (rad * rad)));
          }
        if (s > bv) { bv = s; best = A; }
      }
      return best;
    };
    this.addStroke = (A, pts) => { const s = { pts, born: this.t, life: 70 }; A.strokes.push(s); return s; };
    this.addPin = (A, gx, gy) => { const p = { x: gx, y: gy, r: 6, held: true, born: this.t, released: 0 }; A.pins.push(p); return p; };

    function stampWill(A) {
      A.sx.fill(0); A.sy.fill(0); A.sw.fill(0); A.pin.fill(0);
      const W = 4.0;
      A.strokes = A.strokes.filter(s => self.t - s.born < s.life);
      for (const s of A.strokes) {
        const fade = Math.min(1, (s.life - (self.t - s.born)) / 15);
        const p = s.pts;
        // cumulative length so the far end pulls a little harder (the ink is wetter where it's going)
        for (let k = 0; k + 1 < p.length; k++) {
          const ax = p[k][0], ay = p[k][1], bx = p[k + 1][0], by = p[k + 1][1];
          const L = hyp(bx - ax, by - ay); if (L < 1e-3) continue;
          const tx = (bx - ax) / L, ty = (by - ay) / L;
          const x0 = Math.max(1, Math.floor(Math.min(ax, bx) - W * 2)), x1 = Math.min(GW - 2, Math.ceil(Math.max(ax, bx) + W * 2));
          const y0 = Math.max(1, Math.floor(Math.min(ay, by) - W * 2)), y1 = Math.min(GH - 2, Math.ceil(Math.max(ay, by) + W * 2));
          for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
            const px = x + 0.5 - ax, py = y + 0.5 - ay;
            const u = Math.max(0, Math.min(L, px * tx + py * ty));
            const cx = ax + tx * u, cy = ay + ty * u;
            const ex = cx - (x + 0.5), ey = cy - (y + 0.5), d2 = ex * ex + ey * ey;
            const w = Math.exp(-d2 / (W * W)) * fade; if (w < 0.02) continue;
            const i = y * GW + x;
            if (w > A.sw[i]) {
              const d = Math.sqrt(d2) + 1e-6, pull = Math.min(1, d / W) * 0.35;
              A.sx[i] = tx + ex / d * pull; A.sy[i] = ty + ey / d * pull; A.sw[i] = w;
            }
          }
        }
      }
      A.pins = A.pins.filter(p => p.held || self.t - p.released < 30);
      for (const p of A.pins) {
        const fade = p.held ? 1 : Math.max(0, 1 - (self.t - p.released) / 30);
        for (let y = Math.max(1, Math.floor(p.y - p.r * 2)); y < Math.min(GH - 1, p.y + p.r * 2); y++)
          for (let x = Math.max(1, Math.floor(p.x - p.r * 2)); x < Math.min(GW - 1, p.x + p.r * 2); x++) {
            const d2 = (x + 0.5 - p.x) ** 2 + (y + 0.5 - p.y) ** 2, w = Math.exp(-d2 / (p.r * p.r)) * fade;
            const i = y * GW + x; A.pin[i] = Math.max(A.pin[i], w);
          }
      }
    }

    function will(A, E) {
      // goals: the enemy (where it stands), and far behind it the army's objective edge
      const seed = tmp2; seed.fill(1e9);
      for (let i = 0; i < N; i++) if (E.rho[i] > 0.08) seed[i] = 0;
      const row = A.home === 'top' ? GH - 2 : 1;
      for (let x = 1; x < GW - 1; x++) seed[row * GW + x] = Math.min(seed[row * GW + x], 60);
      const cost = tmp; const rho = A.rho;
      for (let i = 0; i < N; i++) cost[i] = 1 / (G.slow[i] * Math.max(0.25, 1 - 0.6 * rho[i]));
      dijkstra(A.phi, seed, cost, G.pass);
      descend(A.phi, G.pass, A.dx, A.dy);
    }

    // ring of eight directions for "pressed from two sides"
    const RING = []; for (let k = 0; k < 8; k++) { const a = k * Math.PI / 4; RING.push([Math.round(Math.cos(a) * 6), Math.round(Math.sin(a) * 6)]); }

    const RINGO = RING.map(([x, y]) => y * GW + x), NB5 = [0, 1, -1, GW, -GW];
    this.tick = function () {
      const [A0, A1] = this.armies;
      const total = F();
      for (let i = 0; i < N; i++) total[i] = A0.rho[i] + A1.rho[i];
      if (this.step % 6 === 0) { will(A0, A1); will(A1, A0); }
      for (const A of this.armies) { stampWill(A); blur(A.rho, A.P, tmp, 4); blur(A.rho, A.C, tmp, 1); }
      for (const [A, E] of [[A0, A1], [A1, A0]]) {
        const ph = this.t * 0.07;
        for (let y = 1; y < GH - 1; y++) for (let x = 1; x < GW - 1; x++) {
          const i = y * GW + x;
          if (A.rho[i] < 1e-4) { A.vx[i] = A.vy[i] = 0; continue; }
          const m = A.q[i] / A.rho[i], broke = Math.min(1, Math.max(0, (0.55 - m) / 0.35));
          // desire, bent by the ink
          // Rome marches on (south, into the plain) more than it turns to the nearest enemy
          let ux = A.dx[i] * (1 - A.ahead), uy = A.dy[i] * (1 - A.ahead) + (A.home === 'top' ? 1 : -1) * A.ahead;
          const n = 0.25; ux += n * (noiseA[i] * Math.cos(ph) + noiseB[i] * Math.sin(ph)); uy += n * (noiseB[i] * Math.cos(ph) - noiseA[i] * Math.sin(ph));
          const w = A.sw[i];
          if (w > 0) { ux = ux * (1 - w) + A.sx[i] * w * 1.6; uy = uy * (1 - w) + A.sy[i] * w * 1.6; }
          let l = hyp(ux, uy) || 1; ux /= l; uy /= l;
          // broken density flees toward its own camp
          ux = ux * (1 - broke) + A.hx[i] * broke; uy = uy * (1 - broke) + A.hy[i] * broke;
          const pin = A.pin[i];
          let crowd = Math.max(0, 1 - total[i]) * 0.6 + 0.4 * Math.max(0, 1 - E.C[i] * 3);
          crowd += (Math.max(crowd, 0.8) - crowd) * w; // the ink charges through
          const sp = A.speed * G.slow[i] * crowd * (1 + 1.4 * w) * (1 - pin) * (1 + 0.6 * broke);
          let vx = ux * sp, vy = uy * sp;
          // shove: down the enemy's broad pressure, refused where pinned
          const sh = 2.6 * (1 - 0.8 * pin) * (1 - 0.6 * w) * (1 + broke) * Math.min(1, A.rho[i] * 4) * Math.min(1, E.C[i] * 5);
          vx -= sh * (E.P[i + 1] - E.P[i - 1]) * 0.5 * 6; vy -= sh * (E.P[i + GW] - E.P[i - GW]) * 0.5 * 6;
          // cohesion: up its own close density (gone when broken)
          const co = 0.22 * (1 - broke) * (1 + 1.5 * pin);
          vx += co * (A.C[i + 1] - A.C[i - 1]) * 0.5 * 4; vy += co * (A.C[i + GW] - A.C[i - GW]) * 0.5 * 4;
          const vl = hyp(vx, vy); if (vl > 3) { vx *= 3 / vl; vy *= 3 / vl; }
          A.vx[i] = vx; A.vy[i] = vy;
        }
      }
      // flow, capped by room
      const out = [F(), F()], fl = [[F(), F(), F(), F()], [F(), F(), F(), F()]]; // +x,-x,+y,-y
      const inflow = F(); const nb = new Int32Array(4), uf = new Float64Array(4);
      for (let a = 0; a < 2; a++) {
        const A = this.armies[a], f = fl[a];
        for (let y = 1; y < GH - 1; y++) for (let x = 1; x < GW - 1; x++) {
          const i = y * GW + x, r = A.rho[i]; if (r < 1e-5) continue;
          nb[0] = i + 1; nb[1] = i - 1; nb[2] = i + GW; nb[3] = i - GW;
          uf[0] = (A.vx[i] + (A.rho[i + 1] > 1e-4 ? A.vx[i + 1] : 0)) * 0.5; uf[1] = -(A.vx[i] + (A.rho[i - 1] > 1e-4 ? A.vx[i - 1] : 0)) * 0.5;
          uf[2] = (A.vy[i] + (A.rho[i + GW] > 1e-4 ? A.vy[i + GW] : 0)) * 0.5; uf[3] = -(A.vy[i] + (A.rho[i - GW] > 1e-4 ? A.vy[i - GW] : 0)) * 0.5;
          let s = 0;
          for (let k = 0; k < 4; k++) { const v = uf[k]; f[k][i] = v > 0 && G.pass[nb[k]] ? r * v * DT : 0; s += f[k][i]; }
          if (s > r * 0.9) { const sc = r * 0.9 / s; for (let k = 0; k < 4; k++) f[k][i] *= sc; s = r * 0.9; }
          out[a][i] = s;
          for (let k = 0; k < 4; k++) inflow[nb[k]] += f[k][i];
        }
      }
      const scale = F();
      for (let i = 0; i < N; i++) {
        const room = Math.max(0, 1 - total[i]);
        scale[i] = inflow[i] > room ? room / inflow[i] : 1;
      }
      for (let a = 0; a < 2; a++) {
        const A = this.armies[a], f = fl[a];
        const nr = Float32Array.from(A.rho), nq = Float32Array.from(A.q);
        for (let y = 1; y < GH - 1; y++) for (let x = 1; x < GW - 1; x++) {
          const i = y * GW + x, r = A.rho[i]; if (r < 1e-5) continue;
          const m = A.q[i] / r; nb[0] = i + 1; nb[1] = i - 1; nb[2] = i + GW; nb[3] = i - GW;
          for (let k = 0; k < 4; k++) {
            const v = f[k][i] * scale[nb[k]]; if (v <= 0) continue;
            nr[i] -= v; nr[nb[k]] += v; nq[i] -= v * m; nq[nb[k]] += v * m;
          }
        }
        A.rho.set(nr); A.q.set(nq);
      }
      // spill: whatever is packed past one formation overflows to its neighbours (each army in
      // proportion to its share), so no cell ever holds more than the ground can
      for (let it = 0; it < 3; it++) {
        for (let i = 0; i < N; i++) total[i] = A0.rho[i] + A1.rho[i];
        for (let y = 1; y < GH - 1; y++) for (let x = 1; x < GW - 1; x++) {
          const i = y * GW + x, ex = total[i] - 1; if (ex <= 0) continue;
          let room = 0;
          for (let k = 0; k < 4; k++) { const j = i + NB5[k + 1]; if (G.pass[j]) room += Math.max(0, 1 - total[j]); }
          if (room <= 1e-6) continue;
          const mv = Math.min(ex, room);
          for (let k = 0; k < 4; k++) {
            const j = i + NB5[k + 1]; if (!G.pass[j]) continue;
            const share = mv * Math.max(0, 1 - total[j]) / room;
            for (const A of this.armies) {
              const f = A.rho[i] / total[i], d = share * f, m = A.rho[i] > 1e-6 ? A.q[i] / A.rho[i] : 1;
              A.rho[i] -= d; A.rho[j] += d; A.q[i] -= d * m; A.q[j] += d * m;
            }
            total[j] += share; total[i] -= share;
          }
        }
      }
      // attrition and morale
      blur(A0.rho, A0.near, tmp, 1); blur(A1.rho, A1.near, tmp, 1);
      for (const A of this.armies) blur(A.rho, A.P, tmp, 3);
      for (const [A, E] of [[A0, A1], [A1, A0]]) {
        for (let y = 1; y < GH - 1; y++) for (let x = 1; x < GW - 1; x++) {
          const i = y * GW + x, r = A.rho[i]; if (r < 1e-5) continue;
          const m = A.q[i] / r, broke = Math.min(1, Math.max(0, (0.55 - m) / 0.35));
          // the enemy that can strike here: its density around, weighted by its own resolve
          let strike = 0;
          for (let k = 0; k < 5; k++) {
            const j = i + NB5[k], er = E.rho[j]; if (er < 1e-4) continue;
            const em = E.q[j] / er; strike += er * Math.min(1, Math.max(0.1, (em - 0.2) / 0.5));
          }
          const loss = Math.min(r * 0.5, E.kill * r * strike * DT * (1 + 2.5 * broke));
          // pressed from two sides at once: enemy pressure on opposite sides of the ring
          let two = 0;
          for (let k = 0; k < 4; k++) {
            const ja = i + RINGO[k], jb = i + RINGO[k + 4];
            const pa = ja >= 0 && ja < N ? E.P[ja] : 0, pb = jb >= 0 && jb < N ? E.P[jb] : 0;
            two = Math.max(two, Math.min(pa, pb));
          }
          const strain = (Math.max(0, two - 0.08) * 1.4 + strike * 0.008) * A.nerve;
          let nm = m - strain * DT + 0.03 * (1 - m) * DT * (strike < 0.02 ? 1 : 0.3);
          nm = Math.max(0, Math.min(1, nm));
          const nr = r - loss; A.rho[i] = nr; A.q[i] = nr * nm;
          A.lost += loss; self.dead[i] += loss; self.contact[i] = self.contact[i] * 0.85 + loss * 12;
        }
      }
      // morale is catching: a little diffusion of resolve through the formation
      for (const A of this.armies) {
        blur(A.q, tmp2, tmp, 1); blur(A.rho, A.P, tmp, 1);
        for (let i = 0; i < N; i++) if (A.rho[i] > 1e-4) {
          const local = A.P[i] > 1e-4 ? tmp2[i] / A.P[i] : 1, m = A.q[i] / A.rho[i];
          A.q[i] = A.rho[i] * (m * 0.8 + local * 0.2);
        }
      }
      this.t += DT; this.step++;
    };
  }
  window.PF = { World, GW, GH, riverX, RIVER_HALF };
})();
