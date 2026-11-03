// Ink orders: the simulation. Every unit (a few dozen men) obeys only local rules; the ink is a
// field of channels that carries Carthaginian units the way each stroke was drawn.
'use strict';
(function () {
  const W = 1376, H = 1032;

  // seeded randomness, so a plan plays the same every time
  let seed = 216;
  const rand = () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296);

  // ---------- the ink field ----------
  const FC = 8, FW = Math.ceil(W / FC) + 1, FH = Math.ceil(H / FC) + 1;
  const REACH = 26;                       // how far a channel reaches out to take a unit
  const fDist = new Float32Array(FW * FH).fill(1e9);
  const fTx = new Float32Array(FW * FH), fTy = new Float32Array(FW * FH);
  const fPx = new Float32Array(FW * FH), fPy = new Float32Array(FW * FH);
  const fS = new Float32Array(FW * FH), fId = new Int16Array(FW * FH).fill(-1);
  const strokes = [];                     // {pts:[[x,y,s]], len, born}

  function stampSegment(k, a, b) {
    const dx = b[0] - a[0], dy = b[1] - a[1], L = Math.hypot(dx, dy) || 1, tx = dx / L, ty = dy / L;
    const x0 = Math.max(0, Math.floor((Math.min(a[0], b[0]) - REACH) / FC)), x1 = Math.min(FW - 1, Math.ceil((Math.max(a[0], b[0]) + REACH) / FC));
    const y0 = Math.max(0, Math.floor((Math.min(a[1], b[1]) - REACH) / FC)), y1 = Math.min(FH - 1, Math.ceil((Math.max(a[1], b[1]) + REACH) / FC));
    for (let j = y0; j <= y1; j++) for (let i = x0; i <= x1; i++) {
      const cx = i * FC, cy = j * FC;
      let t = ((cx - a[0]) * tx + (cy - a[1]) * ty); t = Math.max(0, Math.min(L, t));
      const px = a[0] + tx * t, py = a[1] + ty * t, d = Math.hypot(cx - px, cy - py);
      const c = j * FW + i;
      // the newer ink lies on top where channels cross
      if (d < REACH && (d < fDist[c] - 0.01 || (fId[c] !== k && fId[c] >= 0 && d < REACH * 0.55))) {
        fDist[c] = d; fTx[c] = tx; fTy[c] = ty; fPx[c] = px; fPy[c] = py; fS[c] = a[2] + t; fId[c] = k;
      }
    }
  }
  function beginStroke(x, y, t) { strokes.push({ pts: [[x, y, 0]], len: 0, born: t, done: false }); return strokes.length - 1; }
  function extendStroke(k, x, y) {
    const st = strokes[k], p = st.pts[st.pts.length - 1], d = Math.hypot(x - p[0], y - p[1]);
    if (d < 5) return;
    // a little smoothing, as a brush would
    const q = [p[0] * 0.35 + x * 0.65, p[1] * 0.35 + y * 0.65]; const dd = Math.hypot(q[0] - p[0], q[1] - p[1]);
    const n = [q[0], q[1], st.len + dd]; st.len += dd; st.pts.push(n); stampSegment(k, p, n);
  }
  function endStroke(k) { strokes[k].done = true; }
  function inkAt(x, y) {
    const i = Math.round(x / FC), j = Math.round(y / FC);
    if (i < 0 || j < 0 || i >= FW || j >= FH) return null;
    const c = j * FW + i; if (fId[c] < 0) return null;
    const dx = fPx[c] - x, dy = fPy[c] - y, d = Math.hypot(dx, dy);
    if (d > REACH) return null;
    const st = strokes[fId[c]], left = st.len - fS[c];
    return { tx: fTx[c], ty: fTy[c], dx, dy, d, left, k: fId[c] };
  }

  // ---------- the armies ----------
  // kinds: 0 Roman legion, 1 Roman horse, 2 allied horse, 3 Gauls & Spaniards, 4 Africans, 5 Hasdrubal's horse, 6 Numidians
  const KIND = [
    { side: 0, cav: 0, speed: 13, atk: 0.11, hp: 1.0, morale: 1.3 },
    { side: 0, cav: 1, speed: 34, atk: 0.20, hp: 1.6, morale: 0.8 },
    { side: 0, cav: 1, speed: 34, atk: 0.18, hp: 1.5, morale: 0.8 },
    { side: 1, cav: 0, speed: 14, atk: 0.12, hp: 1.5, morale: 1.6 },
    { side: 1, cav: 0, speed: 15, atk: 0.15, hp: 1.2, morale: 1.4 },
    { side: 1, cav: 1, speed: 40, atk: 0.26, hp: 1.8, morale: 1.4 },
    { side: 1, cav: 1, speed: 44, atk: 0.14, hp: 1.2, morale: 1.0 },
  ];
  let N = 0; const MAX = 4000;
  const px = new Float32Array(MAX), py = new Float32Array(MAX), vx = new Float32Array(MAX), vy = new Float32Array(MAX);
  const fx = new Float32Array(MAX), fy = new Float32Array(MAX);   // facing
  const hp = new Float32Array(MAX), mor = new Float32Array(MAX), kind = new Uint8Array(MAX), grp = new Uint16Array(MAX);
  const state = new Uint8Array(MAX);      // 0 formed, 1 fleeing, 2 dead, 3 gone off the field
  const why = new Uint8Array(MAX), loosed = new Uint8Array(MAX), onInk = new Uint8Array(MAX), fight = new Uint8Array(MAX), hit = new Float32Array(MAX);
  const deaths = [], groupInk = new Int16Array(64).fill(-1);                      // stains to lay on the paper: [x,y,side]

  function add(k, x, y, g) {
    const i = N++; px[i] = x + (rand() - 0.5) * 1.2; py[i] = y + (rand() - 0.5) * 1.2; vx[i] = vy[i] = 0;
    const s = KIND[k].side; fx[i] = 0; fy[i] = s ? -1 : 1;
    hp[i] = KIND[k].hp; mor[i] = KIND[k].morale * (0.85 + rand() * 0.3); kind[i] = k; grp[i] = g; state[i] = 0;
  }
  function block(k, g, x0, y0, cols, rows, sp) { for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) add(k, x0 + c * sp, y0 + r * sp, g); }

  function deploy() {
    let g = 0;
    // Romans, facing down the screen. Maniples in three lines, checkered; depth over breadth.
    const mx0 = 352, mw = 5, sp = 4.6, gap = 7, n = 12, mpx = mw * sp + gap;
    for (let m = 0; m < n; m++) {
      const x = mx0 + m * mpx;
      block(0, g, x, 330, mw, 10, sp);                       // hastati
      if (m < n - 1) block(0, g, x + mpx / 2, 278, mw, 11, sp);  // principes, in the gaps
      block(0, g, x, 236, mw, 7, sp);                        // triarii
    }
    g++;
    block(1, g++, 262, 352, 5, 14, 5.2);                    // Roman horse, by the river
    block(2, g++, 724, 352, 8, 15, 5.2);                    // allied horse, the open left
    // Hannibal's crescent, bowed toward the Romans
    for (let r = 0; r < 8; r++) for (let c = 0; c < 66; c++) {
      const u = (c / 65) * 2 - 1, x = 365 + c * 4.6 + (r % 2) * 2.3;
      add(3, x, 540 - 46 * Math.cos(u * Math.PI / 2) + r * 4.6, g);
    }
    g++;
    block(4, g++, 318, 560, 7, 23, 4.6);                    // Africans, left
    block(4, g++, 678, 560, 7, 23, 4.6);                    // Africans, right
    block(5, g++, 256, 560, 7, 34, 5.2);                    // Hasdrubal's horse, by the river
    block(6, g++, 750, 570, 10, 15, 5.4);                   // Numidians
  }

  // ---------- the rules ----------
  const GC = 16, GW = Math.ceil(W / GC) + 2, GH = Math.ceil(H / GC) + 2;
  let head = new Int32Array(GW * GH), next = new Int32Array(MAX);
  const CG = 40, CW = Math.ceil(W / CG), CH = Math.ceil(H / CG), coarse = new Int32Array(CW * CH * 2);
  function bin() {
    head.fill(-1); coarse.fill(-1);
    for (let i = 0; i < N; i++) if (state[i] === 0) {
      const cx = Math.floor(px[i] / CG), cy = Math.floor(py[i] / CG);
      if (cx >= 0 && cy >= 0 && cx < CW && cy < CH) coarse[(cy * CW + cx) * 2 + KIND[kind[i]].side] = i;
    }
    for (let i = 0; i < N; i++) if (state[i] < 2) {
      const c = (Math.floor(py[i] / GC) + 1) * GW + Math.floor(px[i] / GC) + 1;
      if (c >= 0 && c < head.length) { next[i] = head[c]; head[c] = i; } else next[i] = -1;
    }
  }
  let started = false, time = 0;
  const RCX = 520;                         // where the Roman commanders press: the centre

  function step(dt) {
    if (!started) return;
    time += dt; bin();
    const dmg = new Float32Array(N);
    for (let i = 0; i < N; i++) {
      if (state[i] >= 2) continue;
      const K = KIND[kind[i]], side = K.side, cav = K.cav;
      const x = px[i], y = py[i], gx = Math.floor(x / GC) + 1, gy = Math.floor(y / GC) + 1;
      // look round: nearest enemy, threats by direction, friends' motion, crowding
      let ne = -1, nd = 1e9, ax = 0, ay = 0, an = 0, fn = 0, en = 0, rx = 0, ry = 0, rear = 0, flee = 0;
      const look = cav ? 3 : 2;
      for (let jy = gy - look; jy <= gy + look; jy++) for (let jx = gx - look; jx <= gx + look; jx++) {
        if (jx < 0 || jy < 0 || jx >= GW || jy >= GH) continue;
        for (let j = head[jy * GW + jx]; j >= 0; j = next[j]) {
          if (j === i) continue;
          const dx = px[j] - x, dy = py[j] - y, d2 = dx * dx + dy * dy;
          if (KIND[kind[j]].side === side) {
            if (d2 < 196) {
              if (state[j] === 1) flee++; else if (d2 < 300) fn++;
              else if (grp[j] === grp[i]) { ax += vx[j]; ay += vy[j]; an++; }
            }
          } else if (state[j] !== 1 || state[i] === 1) {
            if (d2 < nd) { nd = d2; ne = j; }
            if (d2 < 300) {
              const d = Math.sqrt(d2) || 1; rx += dx / d; ry += dy / d; en++;
              if ((dx * fx[i] + dy * fy[i]) / d < -0.6) rear++;
            }
          }
        }
      }
      if (ne < 0 && state[i] === 0) {
        // nobody at hand: a coarser look for the nearest formed enemy further off
        const R = cav ? (side ? (loosed[i] ? 8 : 2) : 4) : (side ? (loosed[i] ? 5 : 1) : 1), cx = Math.floor(x / CG), cy = Math.floor(y / CG), o = 1 - side;
        for (let jy = cy - R; jy <= cy + R; jy++) for (let jx = cx - R; jx <= cx + R; jx++) {
          if (jx < 0 || jy < 0 || jx >= CW || jy >= CH) continue;
          const j = coarse[(jy * CW + jx) * 2 + o]; if (j < 0) continue;
          const d2 = (px[j] - x) ** 2 + (py[j] - y) ** 2; if (d2 < nd) { nd = d2; ne = j; }
        }
      }
      nd = Math.sqrt(nd);
      let tx = 0, ty = 0, sp = K.speed; onInk[i] = 0; fight[i] = 0;

      if (state[i] === 1) {
        // fleeing: away from the enemies at hand, toward home
        let hx = rx ? -rx : 0, hy = ry ? -ry : 0, hl = Math.hypot(hx, hy);
        if (hl > 0) { hx /= hl; hy /= hl; }
        tx = hx * 0.8; ty = hy * 0.8 + (side ? 0.6 : -0.6); sp *= cav ? 1.1 : 0.9;
      } else {
        const contact = ne >= 0 && nd < (cav ? 8 : 6.4);
        const ink = side === 1 ? inkAt(x, y) : null;
        if (contact) {
          fight[i] = 1; const e = ne;
          tx = (px[e] - x) / nd * 0.5; ty = (py[e] - y) / nd * 0.5;
          if (ink) { tx += ink.tx * 0.6; ty += ink.ty * 0.6; }  // the ink keeps pressing them on
          // Lanchester at the point of contact: blows land on the nearest foe, harder from the flank or rear
          const ek = KIND[kind[e]]; const fdot = ((x - px[e]) * fx[e] + (y - py[e]) * fy[e]) / nd;
          let a = K.atk * (fdot < -0.3 ? 2.2 : fdot < 0.3 ? 1.5 : 1) * (state[e] === 1 ? 3 : 1);
          if (cav && !ek.cav) a *= 0.8;
          dmg[e] += a * (0.6 + rand() * 0.8) * dt;
        } else if (ink && ink.left > 6 && !(ne >= 0 && nd < (cav ? 22 : 12))) {
          // carried: the stroke's direction, and drawn back toward its line
          onInk[i] = 1; const pull = Math.min(1, ink.d / REACH) * 0.9;
          const fade = Math.min(1, ink.left / 30);
          tx = ink.tx * fade + ink.dx / (ink.d || 1) * pull; ty = ink.ty * fade + ink.dy / (ink.d || 1) * pull;
          sp *= 1.15; groupInk[grp[i]] = ink.k; loosed[i] = 1;
        } else if (ne >= 0 && nd < (side ? (loosed[i] ? (cav ? 320 : 200) : (cav ? 60 : 26)) : (cav ? 140 : 40))) {
          // the enemy near enough draws them on; men the ink has carried are loosed to seek further
          tx = (px[ne] - x) / nd; ty = (py[ne] - y) / nd;
        } else if (side === 0) {
          // the consuls' intent: advance, and press toward the centre
          tx = (RCX - x) * (cav ? 0 : 0.004); ty = 1;
          if (cav) { ty = 0.6; tx = 0; }
        } else if (groupInk[grp[i]] >= 0 && !loosed[i]) {
          // a body some of whose men the ink has taken goes to the channel too, as Tsuro's pieces do
          const st = strokes[groupInk[grp[i]]], p = st.pts; let bd = 1e9, bx = 0, by = 0;
          for (let k = 0; k < p.length; k += 2) { const d = (p[k][0] - x) ** 2 + (p[k][1] - y) ** 2; if (d < bd && st.len - p[k][2] > 20) { bd = d; bx = p[k][0]; by = p[k][1]; } }
          if (bd < 1e9) { tx = bx - x; ty = by - y; } else if (an > 0) { tx = ax / an / K.speed; ty = ay / an / K.speed; }
        } else if (an > 0) {
          // keep with the friends at hand who are moving
          tx = ax / an / K.speed; ty = ay / an / K.speed;
        }
        // morale: an enemy behind, or on both sides, breaks men; friends running are catching
        const surround = en >= 3 && Math.hypot(rx, ry) / en < 0.45;
        let dm = (rear >= 3 ? 0.1 + rear * 0.07 : 0) + (surround ? 0.5 : 0) + Math.min(5, flee) * 0.006 + hit[i] / dt * 0.1 + Math.max(0, en - fn - 2) * 0.012;
        if (!en && !flee) dm = -0.03;
        mor[i] = Math.min(K.morale * 1.15, mor[i] - dm * dt);
        if (mor[i] < 0) {
          state[i] = 1;
          const terms = [rear >= 3 ? 0.1 + rear * 0.07 : 0, surround ? 0.5 : 0, Math.min(5, flee) * 0.006, hit[i] / dt * 0.1, Math.max(0, en - fn - 2) * 0.012];
          why[i] = 1 + terms.indexOf(Math.max(...terms));
        }
      }
      // steer
      const tl = Math.hypot(tx, ty); let dvx = 0, dvy = 0;
      if (tl > 0.02) { const m = Math.min(1, tl); dvx = tx / tl * sp * m; dvy = ty / tl * sp * m; }
      const acc = cav ? 2.5 : 3.5; vx[i] += (dvx - vx[i]) * Math.min(1, acc * dt); vy[i] += (dvy - vy[i]) * Math.min(1, acc * dt);
      // face slowly toward the way they go or the foe they fight
      let wx = vx[i], wy = vy[i];
      if (fight[i] && ne >= 0) { wx = px[ne] - x; wy = py[ne] - y; }
      const wl = Math.hypot(wx, wy);
      if (wl > 2) { const r = Math.min(1, (fight[i] ? 2.5 : 1.2) * dt); fx[i] += (wx / wl - fx[i]) * r; fy[i] += (wy / wl - fy[i]) * r; const fl = Math.hypot(fx[i], fy[i]) || 1; fx[i] /= fl; fy[i] /= fl; }
    }
    for (let i = 0; i < N; i++) {
      if (state[i] >= 2) continue;
      hit[i] = dmg[i]; hp[i] -= dmg[i];
      if (hp[i] <= 0) { state[i] = 2; deaths.push([px[i], py[i], KIND[kind[i]].side]); continue; }
      px[i] += vx[i] * dt; py[i] += vy[i] * dt;
      if (px[i] < -10 || px[i] > W + 10 || py[i] < -10 || py[i] > H + 10) state[i] = 3;
    }
    // bodies take room: overlap is pushed apart, so depth presses (the deep Roman column shoves)
    bin();
    for (let it = 0; it < 2; it++) for (let i = 0; i < N; i++) {
      if (state[i] >= 2) continue;
      const ri = KIND[kind[i]].cav ? 2.9 : 2.25, gx = Math.floor(px[i] / GC) + 1, gy = Math.floor(py[i] / GC) + 1;
      for (let jy = gy - 1; jy <= gy + 1; jy++) for (let jx = gx - 1; jx <= gx + 1; jx++) {
        if (jx < 0 || jy < 0 || jx >= GW || jy >= GH) continue;
        for (let j = head[jy * GW + jx]; j >= 0; j = next[j]) {
          if (j <= i || state[j] >= 2) continue;
          const rr = ri + (KIND[kind[j]].cav ? 2.9 : 2.25);
          const dx = px[j] - px[i], dy = py[j] - py[i], d2 = dx * dx + dy * dy;
          if (d2 < rr * rr && d2 > 1e-6) {
            const d = Math.sqrt(d2), o = (rr - d) * 0.5 / d * 0.8;
            px[i] -= dx * o; py[i] -= dy * o; px[j] += dx * o; py[j] += dy * o;
          }
        }
      }
    }
    // the river is not crossed in battle
    for (let i = 0; i < N; i++) if (state[i] < 2) { const rx = riverX(py[i]); if (px[i] < rx + 6) { px[i] = rx + 6; vx[i] = Math.max(0, vx[i]); } }
  }

  let riverPts = [];
  function riverX(y) {
    const r = riverPts; if (!r.length) return -99;
    for (let k = 1; k < r.length; k++) if ((r[k - 1][1] - y) * (r[k][1] - y) <= 0) {
      const t = (y - r[k - 1][1]) / ((r[k][1] - r[k - 1][1]) || 1); return r[k - 1][0] + t * (r[k][0] - r[k - 1][0]);
    }
    return r[0][0];
  }

  function census() {
    const c = [[0, 0, 0], [0, 0, 0]];          // per side: formed, fleeing, fallen
    for (let i = 0; i < N; i++) { const s = KIND[kind[i]].side; c[s][state[i] === 0 ? 0 : state[i] === 1 ? 1 : 2]++; }
    return c;
  }

  window.SIM = {
    W, H, REACH, KIND, strokes, deaths, deploy, step, beginStroke, extendStroke, endStroke, inkAt, census, riverX,
    setRiver: r => { riverPts = r.slice().sort((a, b) => a[1] - b[1]); },
    start: () => { started = true; }, get started() { return started; }, get time() { return time; },
    get N() { return N; }, why, px, py, vx, vy, fx, fy, kind, state, onInk, fight, mor,
  };
})();
