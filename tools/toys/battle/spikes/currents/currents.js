// Currents: Cannae as two inks flowing on an engraved plate.
// Each army is a crowd of parcels whose mean motion is a smoothed flow field; every parcel is drawn
// as one burin cut along its army's streamline through it, so the cuts' density is the army's
// strength and their direction and swell its intent. A finger's stroke lays a channel in the plate
// that the nearest army's current follows (a Tsuro path); a tap is an impulse that pushes the
// current outward. Where the two currents touch they slide past each other in opposite senses, and
// the shear curls into eddies. The ground underneath is real (ground.js, baked by ground.py).
// ?manual: the page does not animate on its own; render.cjs calls window.advance(n).
(() => {
  'use strict';
  const G = window.GROUND, W = G.w, H = G.h;
  const qs = new URLSearchParams(location.search);
  const manual = qs.has('manual');

  // ---------- palette: paper, iron-gall ink (Rome), murex purple (Carthage), verdigris water
  const PAPER = '#efe7d6';
  const INK = [[38, 46, 70], [128, 34, 92]];
  const SEPIA = 'rgba(120,98,66,';
  const WATER = [126, 160, 146];

  // ---------- seeded randomness
  let seed = 2160802;
  const rnd = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296);
  const gauss = () => { let u = 0; for (let i = 0; i < 4; i++) u += rnd(); return (u - 2) * 1.2; };

  // ---------- ground
  const elev = Float32Array.from(G.elev_dm, v => v / 10);
  const blurGrid = (src, w, h, passes) => {
    let a = Float32Array.from(src), b = new Float32Array(a.length);
    for (let p = 0; p < passes; p++) {
      for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
        let s = 0, n = 0;
        for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
          const xx = x + dx, yy = y + dy;
          if (xx >= 0 && yy >= 0 && xx < w && yy < h) { s += a[yy * w + xx]; n++; }
        }
        b[y * w + x] = s / n;
      }
      [a, b] = [b, a];
    }
    return a;
  };
  const elevS = blurGrid(elev, W, H, 3);
  const sampleGrid = (g, w, h, x, y) => {
    x = Math.max(0, Math.min(w - 1.001, x)); y = Math.max(0, Math.min(h - 1.001, y));
    const ix = x | 0, iy = y | 0, fx = x - ix, fy = y - iy, i = iy * w + ix;
    return (g[i] * (1 - fx) + g[i + 1] * fx) * (1 - fy) + (g[i + w] * (1 - fx) + g[i + w + 1] * fx) * fy;
  };
  const slopeAt = (x, y) => {
    const gx = sampleGrid(elevS, W, H, x + 1, y) - sampleGrid(elevS, W, H, x - 1, y);
    const gy = sampleGrid(elevS, W, H, x, y + 1) - sampleGrid(elevS, W, H, x, y - 1);
    return Math.hypot(gx, gy) * 0.5 / G.metresPerCell; // rise over run
  };
  const river = G.riverLine; // [[x, y], ...] every 2 rows
  const riverX = y => {
    const i = Math.max(0, Math.min(river.length - 2, Math.floor(y / 2)));
    const f = Math.max(0, Math.min(1, (y - river[i][1]) / 2));
    return river[i][0] * (1 - f) + river[i + 1][0] * f;
  };

  // ---------- canvas
  const cv = document.getElementById('c');
  const ctx = cv.getContext('2d');
  let dpr = 1, sc = 1, ox = 0, oy = 0, groundCv, memCv, memCtx;
  const toScreen = (x, y) => [ox + x * sc, oy + y * sc];
  const toWorld = (sx, sy) => [(sx * dpr - ox) / sc, (sy * dpr - oy) / sc];

  function resize() {
    dpr = window.devicePixelRatio || 1;
    cv.width = Math.round(innerWidth * dpr); cv.height = Math.round(innerHeight * dpr);
    sc = Math.max(cv.width / W, cv.height / H);
    ox = (cv.width - W * sc) / 2; oy = (cv.height - H * sc) / 2;
    drawGround();
    memCv = document.createElement('canvas'); memCv.width = cv.width; memCv.height = cv.height;
    memCtx = memCv.getContext('2d');
  }

  function drawGround() {
    groundCv = document.createElement('canvas');
    groundCv.width = cv.width; groundCv.height = cv.height;
    const g = groundCv.getContext('2d');
    g.fillStyle = PAPER; g.fillRect(0, 0, cv.width, cv.height);
    // paper: a faint plate tone and fibres
    const save = seed; seed = 77;
    for (let i = 0; i < 9000; i++) {
      const x = rnd() * cv.width, y = rnd() * cv.height, a = rnd() * Math.PI, l = (2 + rnd() * 6) * dpr;
      g.strokeStyle = `rgba(110,90,60,${0.025 + rnd() * 0.03})`; g.lineWidth = 0.5 * dpr;
      g.beginPath(); g.moveTo(x, y); g.lineTo(x + Math.cos(a) * l, y + Math.sin(a) * l); g.stroke();
    }
    // hill shade, very quiet (light from the north-west of the screen)
    const sh = document.createElement('canvas'); sh.width = W; sh.height = H;
    const sx = sh.getContext('2d'), im = sx.createImageData(W, H);
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
      const gx = sampleGrid(elevS, W, H, x + 1, y) - sampleGrid(elevS, W, H, x - 1, y);
      const gy = sampleGrid(elevS, W, H, x, y + 1) - sampleGrid(elevS, W, H, x, y - 1);
      const d = (gx + gy) * 0.5 / G.metresPerCell * 6; // facing away from the light darkens
      const i = (y * W + x) * 4;
      im.data[i] = 96; im.data[i + 1] = 76; im.data[i + 2] = 52;
      im.data[i + 3] = Math.max(0, Math.min(1, d)) * 38;
    }
    sx.putImageData(im, 0, 0);
    g.imageSmoothingEnabled = true; g.drawImage(sh, ox, oy, W * sc, H * sc);
    // contours (marching squares), 1 m apart, every fifth bolder: on the plain they are the
    // ground's breath, on the ridge they crowd into the hatching of the slope
    const lo = Math.floor(Math.min(...elevS)), hi = Math.ceil(Math.max(...elevS));
    for (let lev = lo + 0.5; lev < hi; lev += 1) {
      const major = Math.round(lev - 0.5) % 5 === 0;
      g.strokeStyle = SEPIA + (major ? 0.32 : 0.16) + ')';
      g.lineWidth = (major ? 0.9 : 0.55) * dpr;
      g.beginPath();
      for (let y = 0; y < H - 1; y++) for (let x = 0; x < W - 1; x++) {
        const a = elevS[y * W + x], b = elevS[y * W + x + 1], c = elevS[(y + 1) * W + x + 1], d = elevS[(y + 1) * W + x];
        const idx = (a > lev) | ((b > lev) << 1) | ((c > lev) << 2) | ((d > lev) << 3);
        if (idx === 0 || idx === 15) continue;
        const pts = [];
        const e = (p, q, px, py, qx, qy) => { const t = (lev - p) / (q - p); pts.push([px + (qx - px) * t, py + (qy - py) * t]); };
        if ((a > lev) !== (b > lev)) e(a, b, x, y, x + 1, y);
        if ((b > lev) !== (c > lev)) e(b, c, x + 1, y, x + 1, y + 1);
        if ((c > lev) !== (d > lev)) e(c, d, x + 1, y + 1, x, y + 1);
        if ((d > lev) !== (a > lev)) e(d, a, x, y + 1, x, y);
        for (let k = 0; k + 1 < pts.length; k += 2) {
          const [x0, y0] = toScreen(pts[k][0], pts[k][1]), [x1, y1] = toScreen(pts[k + 1][0], pts[k + 1][1]);
          g.moveTo(x0, y0); g.lineTo(x1, y1);
        }
      }
      g.stroke();
    }
    // the river: a verdigris wash and engraved water lines echoing its banks outward
    const rpath = (off) => {
      g.beginPath();
      for (let i = 0; i < river.length; i++) {
        const [x, y] = river[i];
        const j = Math.min(river.length - 1, i + 1), k = Math.max(0, i - 1);
        let tx = river[j][0] - river[k][0], ty = river[j][1] - river[k][1];
        const l = Math.hypot(tx, ty) || 1; tx /= l; ty /= l;
        const [sx_, sy_] = toScreen(x - ty * off, y + tx * off);
        if (i === 0) g.moveTo(sx_, sy_); else g.lineTo(sx_, sy_);
      }
    };
    g.lineCap = 'round'; g.lineJoin = 'round';
    rpath(0); g.strokeStyle = `rgba(${WATER},0.2)`; g.lineWidth = 2.4 * sc; g.stroke();
    for (const [off, a, w] of [[-1.25, .7, .8], [1.25, .7, .8], [-0.45, .35, .5], [0.45, .35, .5],
      [-2.6, .28, .5], [2.6, .28, .5], [-4.2, .16, .45], [4.2, .16, .45], [-6.2, .08, .4], [6.2, .08, .4]]) {
      rpath(off); g.strokeStyle = `rgba(70,112,100,${a})`; g.lineWidth = w * dpr; g.stroke();
    }
    seed = save;
  }

  // ---------- the armies (the ground truth underneath, never shown as words)
  // Romans at the top facing down the screen (south-west), the river on their right flank (screen
  // left); Hannibal's convex crescent of Gauls and Spaniards, Africans behind its horns, Hasdrubal's
  // heavy horse by the river, the Numidians on the open flank. One parcel ~ 33 men.
  const GROUPS = [
    { id: 'rinf', army: 0, n: 2400, kind: 'inf', rect: [70, 150, 38, 74], speed: 0.32, face: [0, 1] },
    { id: 'rcav', army: 0, n: 150, kind: 'cav', rect: [49, 67, 58, 72], speed: 1.4, target: ['hcav'], face: [0, 1] },
    { id: 'acav', army: 0, n: 200, kind: 'cav', rect: [153, 178, 56, 72], speed: 1.3, target: ['ncav'], face: [0, 1] },
    { id: 'gauls', army: 1, n: 880, kind: 'inf', crescent: [78, 142, 88, 102, 5], speed: 0.12, face: [0, -1] },
    { id: 'afrL', army: 1, n: 260, kind: 'inf', rect: [66, 77, 98, 116], speed: 0.1, face: [0, -1] },
    { id: 'afrR', army: 1, n: 260, kind: 'inf', rect: [143, 154, 98, 116], speed: 0.1, face: [0, -1] },
    { id: 'hcav', army: 1, n: 300, kind: 'cav', rect: [49, 68, 86, 100], speed: 1.5, target: ['rcav', 'acav', 'rinf'], face: [0, -1] },
    { id: 'ncav', army: 1, n: 220, kind: 'cav', rect: [157, 182, 88, 102], speed: 1.5, target: ['acav', 'rinf'], face: [0, -1] },
  ];
  const N = GROUPS.reduce((s, g) => s + g.n, 0);
  const P = {
    x: new Float32Array(N), y: new Float32Array(N), vx: new Float32Array(N), vy: new Float32Array(N),
    dx: new Float32Array(N), dy: new Float32Array(N), a: new Uint8Array(N), g: new Uint8Array(N), ph: new Float32Array(N), alive: new Uint8Array(N), j: new Float32Array(N),
  };
  {
    let i = 0;
    GROUPS.forEach((gr, gi) => {
      gr.alive = gr.n;
      for (let k = 0; k < gr.n; k++, i++) {
        let x, y;
        if (gr.rect) { const [x0, x1, y0, y1] = gr.rect; x = x0 + rnd() * (x1 - x0); y = y0 + rnd() * (y1 - y0); }
        else {
          const [x0, x1, apex, ends, th] = gr.crescent; const u = rnd();
          x = x0 + u * (x1 - x0); const c = (u - 0.5) * 2;
          y = apex + (ends - apex) * c * c + rnd() * th;
        }
        P.x[i] = x; P.y[i] = y; P.a[i] = gr.army; P.g[i] = gi; P.alive[i] = 1;
        P.vx[i] = gr.face[0] * 0.05; P.vy[i] = gr.face[1] * 0.05; P.ph[i] = rnd(); P.j[i] = rnd();
      }
    });
  }

  // ---------- flow fields on a coarse grid
  const FC = 2, FW = W / FC, FH = H / FC, FN = FW * FH;
  const rho = [new Float32Array(FN), new Float32Array(FN)];
  const mx = [new Float32Array(FN), new Float32Array(FN)], my = [new Float32Array(FN), new Float32Array(FN)];
  let rhoB = [rho[0], rho[1]], ux = [mx[0], mx[1]], uy = [my[0], my[1]];
  const gxF = [new Float32Array(FN), new Float32Array(FN)], gyF = [new Float32Array(FN), new Float32Array(FN)];
  const stx = [new Float32Array(FN), new Float32Array(FN)], sty = [new Float32Array(FN), new Float32Array(FN)];
  const fs = (g, x, y) => sampleGrid(g, FW, FH, x / FC - 0.5, y / FC - 0.5);

  function buildFields() {
    for (let a = 0; a < 2; a++) { rho[a].fill(0); mx[a].fill(0); my[a].fill(0); }
    for (let i = 0; i < N; i++) {
      if (!P.alive[i]) continue;
      const cx = Math.min(FW - 1, Math.max(0, (P.x[i] / FC) | 0)), cy = Math.min(FH - 1, Math.max(0, (P.y[i] / FC) | 0));
      const k = cy * FW + cx, a = P.a[i];
      rho[a][k] += 1; mx[a][k] += P.vx[i] + P.dx[i] * 0.6; my[a][k] += P.vy[i] + P.dy[i] * 0.6;
    }
    for (let a = 0; a < 2; a++) {
      rhoB[a] = blurGrid(rho[a], FW, FH, 2);
      const bx = blurGrid(mx[a], FW, FH, 3), by = blurGrid(my[a], FW, FH, 3), br = blurGrid(rho[a], FW, FH, 3);
      ux[a] = new Float32Array(FN); uy[a] = new Float32Array(FN);
      for (let k = 0; k < FN; k++) { ux[a][k] = bx[k] / (br[k] + 0.02); uy[a][k] = by[k] / (br[k] + 0.02); }
      for (let y = 0; y < FH; y++) for (let x = 0; x < FW; x++) {
        const k = y * FW + x, r = rhoB[a];
        gxF[a][k] = ((x < FW - 1 ? r[k + 1] : r[k]) - (x > 0 ? r[k - 1] : r[k])) * 0.5;
        gyF[a][k] = ((y < FH - 1 ? r[k + FW] : r[k]) - (y > 0 ? r[k - FW] : r[k])) * 0.5;
      }
    }
  }

  // ---------- the finger: strokes lay channels, taps push
  const strokes = []; // {a, pts:[[x,y]], t0, live}
  const pulses = []; // {x, y, t0}
  let simT = 0, cur = null;
  function ownerAt(x, y) {
    const r0 = fs(rhoB[0], x, y), r1 = fs(rhoB[1], x, y);
    if (r0 + r1 > 0.05) return r0 > r1 ? 0 : 1;
    let best = 1e9, a = 0;
    for (let i = 0; i < N; i++) if (P.alive[i]) { const d = (P.x[i] - x) ** 2 + (P.y[i] - y) ** 2; if (d < best) { best = d; a = P.a[i]; } }
    return a;
  }
  cv.addEventListener('pointerdown', e => {
    const [x, y] = toWorld(e.clientX, e.clientY);
    cur = { a: ownerAt(x, y), pts: [[x, y]], t0: simT, live: true, down: simT };
    cv.setPointerCapture && cv.setPointerCapture(e.pointerId);
  });
  cv.addEventListener('pointermove', e => {
    if (!cur) return;
    const [x, y] = toWorld(e.clientX, e.clientY), l = cur.pts[cur.pts.length - 1];
    if (Math.hypot(x - l[0], y - l[1]) > 0.8) {
      cur.pts.push([x, y]);
      if (cur.pts.length === 3) strokes.push(cur);
    }
  });
  const up = () => {
    if (!cur) return;
    if (cur.pts.length < 3) { const [x, y] = cur.pts[0]; pulses.push({ x, y, t0: simT }); }
    cur.live = false; cur.t0 = simT; cur = null;
  };
  cv.addEventListener('pointerup', up); cv.addEventListener('pointercancel', up);

  const TAU = 50; // a channel's life (s)
  function buildChannels() {
    for (let a = 0; a < 2; a++) { stx[a].fill(0); sty[a].fill(0); }
    const R = 5;
    for (let s = strokes.length - 1; s >= 0; s--) {
      const st = strokes[s];
      const age = st.live ? 0 : simT - st.t0, str = Math.exp(-age / TAU);
      if (str < 0.03) { strokes.splice(s, 1); continue; }
      const pts = st.pts;
      for (let i = 0; i + 1 < pts.length; i++) {
        const [x0, y0] = pts[i], [x1, y1] = pts[i + 1];
        let tx = x1 - x0, ty = y1 - y0; const l = Math.hypot(tx, ty) || 1; tx /= l; ty /= l;
        const cx = (x0 / FC) | 0, cy = (y0 / FC) | 0;
        for (let dy = -R; dy <= R; dy++) for (let dx = -R; dx <= R; dx++) {
          const gx = cx + dx, gy = cy + dy;
          if (gx < 0 || gy < 0 || gx >= FW || gy >= FH) continue;
          const wx = (gx + 0.5) * FC, wy = (gy + 0.5) * FC;
          const px = wx - x0, py = wy - y0, along = px * tx + py * ty;
          if (along < -1 || along > l + 1) continue;
          const perp = px * -ty + py * tx, d = Math.abs(perp) / (R * FC);
          if (d >= 1) continue;
          const w = str * (1 - d) * (1 - d) * 0.35;
          const k = gy * FW + gx;
          stx[st.a][k] += (tx + ty * perp * 0.12) * w;
          sty[st.a][k] += (ty - tx * perp * 0.12) * w;
        }
      }
    }
  }

  // ---------- one step of the crowd
  const centroid = (gi) => {
    let sx = 0, sy = 0, n = 0;
    for (let i = 0; i < N; i++) if (P.alive[i] && P.g[i] === gi) { sx += P.x[i]; sy += P.y[i]; n++; }
    return n ? [sx / n, sy / n, n] : null;
  };
  let romanC = [110, 60];
  function step(dt) {
    buildFields(); buildChannels();
    const cents = GROUPS.map((g, gi) => centroid(gi));
    const rc = cents[0]; if (rc) romanC = [rc[0], rc[1]];
    for (const gr of GROUPS) gr.alive = 0;
    for (let i = 0; i < N; i++) {
      if (!P.alive[i]) continue;
      const gr = GROUPS[P.g[i]], a = P.a[i], e = 1 - a;
      GROUPS[P.g[i]].alive++;
      const x = P.x[i], y = P.y[i];
      // intent
      let dx = gr.face[0] * gr.speed, dy = gr.face[1] * gr.speed;
      if (gr.kind === 'cav') {
        // the first target still standing (Hasdrubal: the Roman horse, then the allied horse, then the rear)
        let tc = null;
        for (const id of gr.target) {
          const ti = GROUPS.findIndex(g => g.id === id);
          if (cents[ti] && cents[ti][2] > GROUPS[ti].n * (id === 'rinf' ? 0 : 0.65)) { tc = cents[ti]; break; }
        }
        if (!tc) { // the nearest enemy mass
          let best = 1e9;
          GROUPS.forEach((g, gi) => { const c = cents[gi]; if (g.army === e && c) { const d = Math.hypot(c[0] - x, c[1] - y); if (d < best) { best = d; tc = c; } } });
        }
        if (tc) {
          const vx = tc[0] - x, vy = tc[1] - y, l = Math.hypot(vx, vy) || 1;
          const melee = fs(rhoB[e], x, y) > 0.5 ? 0.3 : 1; // in the press, the charge is spent
          dx = vx / l * gr.speed * melee; dy = vy / l * gr.speed * melee;
        }
      }
      // the channel a finger laid
      const cx = fs(stx[a], x, y), cy = fs(sty[a], x, y), cl = Math.hypot(cx, cy);
      if (cl > 0.02) {
        const k = Math.min(1, cl * 2.2), sp = gr.kind === 'cav' ? 3.2 : 1.1;
        dx = dx * (1 - k) + cx / cl * sp * k; dy = dy * (1 - k) + cy / cl * sp * k;
      }
      P.dx[i] = dx; P.dy[i] = dy;
      dx += gauss() * 0.035; dy += gauss() * 0.035;
      // inertia toward intent, then toward the army's current
      let vx = P.vx[i] + (dx - P.vx[i]) * 0.1, vy = P.vy[i] + (dy - P.vy[i]) * 0.1;
      vx += (fs(ux[a], x, y) - vx) * 0.12; vy += (fs(uy[a], x, y) - vy) * 0.12;
      // pressure of one's own (spreads), contact with the enemy (pushes and slides)
      const gox = fs(gxF[a], x, y), goy = fs(gyF[a], x, y);
      const gex = fs(gxF[e], x, y), gey = fs(gyF[e], x, y);
      const re = fs(rhoB[e], x, y), ro = fs(rhoB[a], x, y);
      const pr = gr.kind === 'cav' ? 0.3 : 0.08; vx -= gox * pr; vy -= goy * pr;
      const push = gr.kind === 'cav' ? 1.0 : (a === 0 ? 0.3 : 0.7);
      vx -= gex * push; vy -= gey * push;
      if (re > 0.05) {
        // slide past the enemy: Carthage turns about the Roman mass one way, Rome the other way
        const rx = x - romanC[0], ry = y - romanC[1], rl = Math.hypot(rx, ry) || 1;
        const sgn = a === 1 ? 1 : -0.6, m = Math.min(1.5, re * 0.6);
        vx += -ry / rl * sgn * m * 0.5; vy += rx / rl * sgn * m * 0.5;
        // Rome, held, still presses on
        if (a === 0 && gr.kind === 'inf') vy += 0.05;
      }
      // impulses of taps
      for (const p of pulses) {
        const age = simT - p.t0, ddx = x - p.x, ddy = y - p.y, d = Math.hypot(ddx, ddy) || 1;
        const ring = 4 + age * 9, w = Math.exp(-((d - ring) ** 2) / 30) * Math.exp(-age * 0.45);
        vx += ddx / d * w * 2.5; vy += ddy / d * w * 2.5;
      }
      // the ground: slope slows, the river is a wall
      const sl = slopeAt(x, y); const slow = 1 / (1 + sl * 18);
      let sp = Math.hypot(vx, vy); const cap = (gr.kind === 'cav' ? 3.6 : 1.4) * slow;
      if (sp > cap) { vx *= cap / sp; vy *= cap / sp; }
      let nx = x + vx * dt, ny = y + vy * dt;
      const rx = riverX(ny);
      if ((x - riverX(y)) * (nx - rx) <= 0 || Math.abs(nx - rx) < 1.6) { nx = x + (x > rx ? 0.25 : -0.25) * dt * 4; vx = -vx * 0.3; }
      nx = Math.max(1, Math.min(W - 1, nx)); ny = Math.max(1, Math.min(H - 1, ny));
      P.x[i] = nx; P.y[i] = ny; P.vx[i] = vx; P.vy[i] = vy;
      P.ph[i] = (P.ph[i] + Math.hypot(vx, vy) * dt * 0.35 + dt * 0.08) % 1;
      // attrition: the enemy's weight against one's own, far worse when surrounded
      if (re > 0.08) {
        let enc = 0;
        for (const [ax, ay] of [[4, 0], [-4, 0], [0, 4], [0, -4]]) if (fs(rhoB[e], x + ax, y + ay) > 0.4) enc++;
        const p = 0.2 * dt * re / (ro + re + 0.5) * (1 + enc * enc * 0.6) * (a === 0 ? (gr.kind === 'cav' ? 2.5 : 1.0) : (gr.kind === 'cav' ? 0.35 : 0.25));
        if (rnd() < p) {
          P.alive[i] = 0;
          const [sx, sy] = toScreen(nx, ny);
          memCtx.fillStyle = `rgba(${INK[a]},0.32)`;
          memCtx.fillRect(sx - 0.6 * dpr, sy - 0.6 * dpr, 1.2 * dpr, 1.2 * dpr);
        }
      }
    }
    for (let s = pulses.length - 1; s >= 0; s--) if (simT - pulses[s].t0 > 10) pulses.splice(s, 1);
    // the plate remembers where the currents ran
    if ((frame & 1) === 0) {
      for (let a = 0; a < 2; a++) {
        memCtx.fillStyle = `rgba(${INK[a]},0.012)`; memCtx.beginPath();
        for (let i = 0; i < N; i++) if (P.alive[i] && P.a[i] === a) {
          memCtx.rect(ox + P.x[i] * sc - 0.8 * dpr, oy + P.y[i] * sc - 0.8 * dpr, 1.6 * dpr, 1.6 * dpr);
        }
        memCtx.fill();
      }
    }
    simT += dt;
  }

  // ---------- drawing: one burin cut per parcel, along its army's current
  const trace = new Float32Array(64);
  function cut(i) {
    const a = P.a[i];
    const sp = Math.hypot(P.vx[i], P.vy[i]);
    const len = 2.8 + Math.min(3.2, sp) * 2.6 + P.j[i] * 1.2;
    const n = 8, h = len / (2 * n);
    // trace backward then forward through the army's field
    let x = P.x[i], y = P.y[i];
    const pts = trace; let m = 0;
    const dirAt = (x, y, s) => {
      let u = fs(ux[a], x, y), v = fs(uy[a], x, y);
      u += P.dx[i] * 0.3; v += P.dy[i] * 0.3;
      const l = Math.hypot(u, v);
      if (l < 1e-4) { const f = GROUPS[P.g[i]].face; return [f[0] * s, f[1] * s]; }
      return [u / l * s, v / l * s];
    };
    const back = [];
    for (let k = 0; k < n; k++) { const [u, v] = dirAt(x, y, -h); x += u; y += v; back.push(x, y); }
    for (let k = back.length - 2; k >= 0; k -= 2) { pts[m++] = back[k]; pts[m++] = back[k + 1]; }
    x = P.x[i]; y = P.y[i]; pts[m++] = x; pts[m++] = y;
    for (let k = 0; k < n; k++) { const [u, v] = dirAt(x, y, h); x += u; y += v; pts[m++] = x; pts[m++] = y; }
    const cnt = m / 2;
    // tapered outline: the swell travels along the cut with the parcel's phase
    const ro = fs(rhoB[a], P.x[i], P.y[i]);
    const wmax = (0.42 + 0.5 * Math.min(1, ro / 3.5)) * dpr * (GROUPS[P.g[i]].kind === 'cav' ? 1.15 : 1) * (a === 1 ? 1.2 : 1);
    const L = [], R = [];
    for (let k = 0; k < cnt; k++) {
      const t = k / (cnt - 1);
      const kk = Math.min(cnt - 1, k + 1), kp = Math.max(0, k - 1);
      let tx = pts[kk * 2] - pts[kp * 2], ty = pts[kk * 2 + 1] - pts[kp * 2 + 1];
      const l = Math.hypot(tx, ty) || 1; tx /= l; ty /= l;
      const sw = 0.55 + 0.45 * Math.cos(2 * Math.PI * (t - P.ph[i]));
      const w = wmax * Math.pow(Math.sin(Math.PI * t), 0.7) * sw;
      const [sx, sy] = toScreen(pts[k * 2], pts[k * 2 + 1]);
      L.push(sx - ty * w, sy + tx * w); R.push(sx + ty * w, sy - tx * w);
    }
    if (vn + L.length * 9 > vbuf.length) return;
    for (let k = 0; k + 2 < L.length; k += 2) {
      const ax = L[k], ay = L[k + 1], bx = R[k], by = R[k + 1], cx = L[k + 2], cy = L[k + 3], dx = R[k + 2], dy = R[k + 3];
      vbuf[vn++] = ax; vbuf[vn++] = ay; vbuf[vn++] = a; vbuf[vn++] = bx; vbuf[vn++] = by; vbuf[vn++] = a; vbuf[vn++] = cx; vbuf[vn++] = cy; vbuf[vn++] = a;
      vbuf[vn++] = cx; vbuf[vn++] = cy; vbuf[vn++] = a; vbuf[vn++] = bx; vbuf[vn++] = by; vbuf[vn++] = a; vbuf[vn++] = dx; vbuf[vn++] = dy; vbuf[vn++] = a;
    }
  }

  // the cuts are drawn by WebGL on an offscreen canvas and multiplied into the plate
  const gcv = document.createElement('canvas');
  const gl = gcv.getContext('webgl2', { antialias: true, premultipliedAlpha: true, alpha: true, preserveDrawingBuffer: true });
  const vbuf = new Float32Array(N * 20 * 18); let vn = 0;
  const prog = (() => {
    const sh = (t, src) => { const o = gl.createShader(t); gl.shaderSource(o, src); gl.compileShader(o); if (!gl.getShaderParameter(o, gl.COMPILE_STATUS)) throw gl.getShaderInfoLog(o); return o; };
    const p = gl.createProgram();
    gl.attachShader(p, sh(gl.VERTEX_SHADER, `#version 300 es
      in vec3 v; uniform vec2 res; flat out int arm;
      void main() { arm = int(v.z + 0.5); vec2 c = v.xy / res * 2.0 - 1.0; gl_Position = vec4(c.x, -c.y, 0, 1); }`));
    gl.attachShader(p, sh(gl.FRAGMENT_SHADER, `#version 300 es
      precision mediump float; flat in int arm; uniform vec4 ink[2]; out vec4 o;
      void main() { vec4 c = ink[arm]; o = vec4(c.rgb * c.a, c.a); }`));
    gl.linkProgram(p); return p;
  })();
  const vbo = gl.createBuffer();
  const vao = gl.createVertexArray(); gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
  gl.enableVertexAttribArray(0); gl.bindAttribLocation(prog, 0, 'v');
  gl.vertexAttribPointer(gl.getAttribLocation(prog, 'v'), 3, gl.FLOAT, false, 12, 0);
  function drawCuts() {
    if (gcv.width !== cv.width || gcv.height !== cv.height) { gcv.width = cv.width; gcv.height = cv.height; }
    vn = 0;
    for (let i = 0; i < N; i++) if (P.alive[i]) cut(i);
    gl.viewport(0, 0, gcv.width, gcv.height);
    gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.useProgram(prog);
    gl.uniform2f(gl.getUniformLocation(prog, 'res'), gcv.width, gcv.height);
    gl.uniform4fv(gl.getUniformLocation(prog, 'ink'), [...INK[0].map(c => c / 255), 0.9, ...INK[1].map(c => c / 255), 0.9]);
    gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.bindVertexArray(vao); gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(gl.ARRAY_BUFFER, vbuf.subarray(0, vn), gl.DYNAMIC_DRAW);
    gl.drawArrays(gl.TRIANGLES, 0, vn / 3);
  }

  let frame = 0;
  function draw() {
    ctx.globalCompositeOperation = 'source-over';
    ctx.drawImage(groundCv, 0, 0);
    ctx.globalCompositeOperation = 'multiply';
    ctx.drawImage(memCv, 0, 0);
    // a finger's channel: a burnished groove that fades as the current takes it
    for (const st of strokes) {
      const age = st.live ? 0 : simT - st.t0, al = Math.exp(-age / (TAU * 0.5));
      ctx.beginPath();
      st.pts.forEach(([x, y], k) => { const [sx, sy] = toScreen(x, y); k ? ctx.lineTo(sx, sy) : ctx.moveTo(sx, sy); });
      ctx.globalCompositeOperation = 'source-over';
      ctx.strokeStyle = `rgba(250,246,236,${0.55 * al})`; ctx.lineWidth = 7 * dpr; ctx.lineCap = 'round'; ctx.lineJoin = 'round'; ctx.stroke();
      ctx.globalCompositeOperation = 'multiply';
      ctx.strokeStyle = `rgba(${INK[st.a]},${0.35 * al})`; ctx.lineWidth = 0.6 * dpr; ctx.stroke();
    }
    drawCuts();
    ctx.globalCompositeOperation = 'multiply';
    ctx.drawImage(gcv, 0, 0);
    ctx.globalCompositeOperation = 'source-over';
  }

  const DT = 1 / 30, SPEED = 7;
  function advance(n, noDraw) {
    for (let k = 0; k < n; k++) { step(DT * SPEED * 0.5); step(DT * SPEED * 0.5); frame++; }
    if (!noDraw) draw();
  }
  window.advance = advance;
  window.profile = (n) => {
    let t = performance.now(); for (let k = 0; k < n; k++) step(0.1); const ts = (performance.now() - t) / n;
    t = performance.now(); for (let k = 0; k < n; k++) draw(); const td = (performance.now() - t) / n;
    return `step ${ts.toFixed(1)} ms, draw ${td.toFixed(1)} ms`;
  };
  window.toWorldScreen = (x, y) => [(ox + x * sc) / dpr, (oy + y * sc) / dpr]; // world -> CSS px
  window.stats = () => GROUPS.map(g => g.id + ':' + g.alive).join(' ');

  resize();
  buildFields();
  draw();
  addEventListener('resize', () => { resize(); draw(); });
  if (!manual) {
    let last = performance.now(), acc = 0;
    const loop = (t) => {
      acc += Math.min(0.1, (t - last) / 1000); last = t;
      let n = 0; while (acc >= DT && n < 2) { acc -= DT; step(DT * SPEED * 0.5); step(DT * SPEED * 0.5); frame++; n++; }
      draw(); requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
  }
})();
