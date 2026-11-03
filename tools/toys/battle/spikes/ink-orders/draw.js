// Ink orders: the paper, the ground, the ghost of the real day, the ink and the armies.
'use strict';
(function () {
  const S = window.SIM, T = window.TERRAIN, W = S.W, H = S.H;
  const q = new URLSearchParams(location.search);
  const manual = q.has('manual'), film = q.has('film');
  const cv = document.getElementById('c'), ctx = cv.getContext('2d');
  let dpr = 1, scale = 1, ox = 0, oy = 0;
  const DX = 170, X0 = T.x0;            // the view sits the battle a little right of centre, the far bank showing

  function resize() {
    dpr = window.devicePixelRatio || 1;
    cv.width = Math.round(innerWidth * dpr); cv.height = Math.round(innerHeight * dpr);
    scale = Math.max(innerWidth / W, innerHeight / H); ox = (innerWidth - W * scale) / 2 + DX * scale; oy = (innerHeight - H * scale) / 2;
    paper = makePaper(); stains = makeLayer(); stainsDone = 0;
  }
  function makeLayer() { const c = document.createElement('canvas'); c.width = Math.ceil((W - X0) * 2); c.height = Math.ceil(H * 2); const x = c.getContext('2d'); x.scale(2, 2); x.translate(-X0, 0); return c; }

  // ---------- the paper and the ground, drawn once ----------
  const PAPER = '#efe8d6', CONTOUR = 'rgba(120,96,64,', RIVER = '#5d7184';
  let paper, stains, stainsDone = 0;
  const river = T.river; S.setRiver(river);

  function makePaper() {
    const c = makeLayer(), g = c.getContext('2d');
    g.fillStyle = PAPER; g.fillRect(X0, 0, W - X0, H);
    // grain: a few thousand faint flecks and a soft unevenness, like laid paper
    let s = 7; const r = () => ((s = (s * 16807) % 2147483647) / 2147483647);
    for (let k = 0; k < 60; k++) { const x = X0 + r() * (W - X0), y = r() * H, rad = 120 + r() * 300; const gr = g.createRadialGradient(x, y, 0, x, y, rad); gr.addColorStop(0, `rgba(${r() < 0.75 ? '255,251,240' : '222,208,178'},${r() < 0.75 ? 0.06 : 0.03})`); gr.addColorStop(1, 'rgba(0,0,0,0)'); g.fillStyle = gr; g.fillRect(x - rad, y - rad, rad * 2, rad * 2); }
    for (let k = 0; k < 9000; k++) { g.fillStyle = `rgba(110,90,60,${0.03 + r() * 0.05})`; g.fillRect(X0 + r() * (W - X0), r() * H, 0.6 + r(), 0.6); }
    // contours of the real ground every 3 m, every 15 m a little firmer (marching squares)
    const { w, h, cell, elev } = T, E = (i, j) => elev[j * w + i];
    for (let lv = 12; lv < 100; lv += 3) {
      const major = lv % 15 === 0;
      g.strokeStyle = CONTOUR + (major ? 0.26 : 0.11) + ')'; g.lineWidth = major ? 0.8 : 0.5; g.beginPath();
      for (let j = 0; j < h - 1; j++) for (let i = 0; i < w - 1; i++) {
        const a = E(i, j), b = E(i + 1, j), c2 = E(i + 1, j + 1), d = E(i, j + 1);
        const pts = [];
        const edge = (v0, v1, x0, y0, x1, y1) => { if ((v0 < lv) !== (v1 < lv)) { const t = (lv - v0) / (v1 - v0); pts.push([(x0 + (x1 - x0) * t) * cell + X0, (y0 + (y1 - y0) * t) * cell]); } };
        edge(a, b, i, j, i + 1, j); edge(b, c2, i + 1, j, i + 1, j + 1); edge(c2, d, i + 1, j + 1, i, j + 1); edge(d, a, i, j + 1, i, j);
        for (let k = 0; k + 1 < pts.length; k += 2) { g.moveTo(pts[k][0], pts[k][1]); g.lineTo(pts[k + 1][0], pts[k + 1][1]); }
      }
      g.stroke();
    }
    // hachures on the slope up to Cannae's ridge: short strokes down the fall line
    g.strokeStyle = 'rgba(110,86,56,0.05)'; g.lineWidth = 0.6; g.beginPath();
    for (let j = 1; j < h - 1; j++) for (let i = 1; i < w - 1; i++) {
      const gx = (E(i + 1, j) - E(i - 1, j)) / (2 * cell), gy = (E(i, j + 1) - E(i, j - 1)) / (2 * cell), m = Math.hypot(gx, gy);
      if (m < 0.012 || r() > Math.min(1, m * 14)) continue;
      const x = i * cell + X0 + (r() - 0.5) * cell, y = j * cell + (r() - 0.5) * cell, L = 3 + Math.min(6, m * 60);
      g.moveTo(x, y); g.lineTo(x - gx / m * L, y - gy / m * L);
    }
    g.stroke();
    // the Aufidus: a pale wash, its banks in a thin grey-blue ink
    const line = (off, wd, col) => { g.strokeStyle = col; g.lineWidth = wd; g.lineJoin = g.lineCap = 'round'; g.beginPath(); river.forEach((p, k) => { const x = p[0] + off + Math.sin(p[1] * 0.031) * 2.2; k ? g.lineTo(x, p[1]) : g.moveTo(x, p[1]); }); g.stroke(); };
    line(0, 22, 'rgba(160,184,196,0.22)'); line(0, 11, 'rgba(150,176,190,0.30)');
    line(-4.5, 0.9, RIVER); line(4.5, 0.9, RIVER);
    // the ghost of the day itself: the real movements, faint graphite underdrawing
    ghost(g);
    return c;
  }

  function ghost(g) {
    g.save(); g.strokeStyle = 'rgba(70,70,74,0.20)'; g.fillStyle = 'rgba(70,70,74,0.20)'; g.lineWidth = 1.1; g.setLineDash([5, 5]); g.lineCap = 'round';
    const path = (pts, arrow) => {
      g.beginPath(); g.moveTo(pts[0][0], pts[0][1]);
      for (let k = 1; k < pts.length - 1; k++) { const m = [(pts[k][0] + pts[k + 1][0]) / 2, (pts[k][1] + pts[k + 1][1]) / 2]; g.quadraticCurveTo(pts[k][0], pts[k][1], m[0], m[1]); }
      const e = pts[pts.length - 1]; g.lineTo(e[0], e[1]); g.stroke();
      if (arrow) { const p = pts[pts.length - 2], a = Math.atan2(e[1] - p[1], e[0] - p[0]); g.save(); g.setLineDash([]); g.beginPath(); g.moveTo(e[0], e[1]); g.lineTo(e[0] - 8 * Math.cos(a - 0.4), e[1] - 8 * Math.sin(a - 0.4)); g.moveTo(e[0], e[1]); g.lineTo(e[0] - 8 * Math.cos(a + 0.4), e[1] - 8 * Math.sin(a + 0.4)); g.stroke(); g.restore(); }
    };
    path([[262, 610], [250, 470], [272, 385], [292, 262], [380, 196], [520, 180], [650, 196], [720, 262], [716, 340]], true); // Hasdrubal's ride
    path([[338, 600], [330, 520], [348, 452], [392, 410]], true);   // the Africans turn in
    path([[700, 600], [706, 520], [690, 452], [646, 410]], true);
    path([[520, 260], [520, 470]], true);                            // the Roman press
    // the crescent as it stood, and as it was bent back
    const arc = (bow, y0) => { const pts = []; for (let c = 0; c <= 12; c++) { const u = c / 12 * 2 - 1; pts.push([365 + c / 12 * 300, y0 - bow * Math.cos(u * Math.PI / 2)]); } path(pts, false); };
    arc(46, 540); arc(-40, 560);
    g.restore();
  }

  // ---------- ink and armies, every frame ----------
  const INK = '#2a1c12', ROME = [28, 38, 62], CARTH = [126, 44, 24];
  let clock = 0;

  function drawInk() {
    for (const st of S.strokes) {
      const p = st.pts; if (p.length < 2) continue;
      const age = clock - st.born;
      // the channel: a wash as wide as its reach, spreading as the ink soaks in
      ctx.lineJoin = ctx.lineCap = 'round';
      ctx.strokeStyle = 'rgba(120,84,48,0.045)'; ctx.lineWidth = S.REACH * 1.6 * Math.min(1, 0.4 + age * 0.9);
      ctx.beginPath(); p.forEach((v, k) => k ? ctx.lineTo(v[0], v[1]) : ctx.moveTo(v[0], v[1])); ctx.stroke();
      // the stroke: a brush's line, swelling a little where the hand slowed
      // the stroke, alive: swellings of ink run down it the way it was drawn, like a pulse in a vein;
      // thin where the brush set down, full where it lifted
      const flow = 40, gap = 34;
      ctx.strokeStyle = INK;
      for (let k = 1; k < p.length; k++) {
        const a = p[k - 1], b = p[k], sm = (a[2] + b[2]) / 2;
        const ph = ((sm - clock * flow) % gap + gap) % gap / gap, pulse = Math.pow(Math.max(0, Math.cos(ph * Math.PI * 2)), 6);
        const taper = Math.min(1, 0.35 + sm / 60);
        ctx.globalAlpha = 0.62 + 0.3 * pulse; ctx.lineWidth = (1.5 + 2.4 * pulse) * taper + 0.5 * Math.sin(sm * 0.04);
        ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke();
      }
      ctx.globalAlpha = 1;
      // where it ends, the ink pools: the order's "hold here"
      if (st.done) { const e = p[p.length - 1]; ctx.fillStyle = INK; ctx.globalAlpha = 0.75; ctx.beginPath(); ctx.ellipse(e[0], e[1], 3.6, 3.1, e[2] * 0.01, 0, Math.PI * 2); ctx.fill(); ctx.globalAlpha = 1; }
    }
  }

  function drawUnits() {
    const N = S.N, { px, py, fx, fy, kind, state, onInk, fight } = S;
    // stains first: every fallen unit leaves a mark on the paper for good
    const sg = stains.getContext('2d');
    for (; stainsDone < S.deaths.length; stainsDone++) {
      const [x, y, s] = S.deaths[stainsDone], c = s ? CARTH : ROME;
      sg.fillStyle = `rgba(${c[0]},${c[1]},${c[2]},0.13)`; sg.beginPath(); sg.arc(x, y, 2.4, 0, Math.PI * 2); sg.fill();
    }
    ctx.drawImage(stains, X0, 0, W - X0, H);
    ctx.lineCap = 'round';
    for (let side = 0; side < 2; side++) for (let pass = 0; pass < 3; pass++) {
      const c = side ? CARTH : ROME;
      // pass 0: the fled, faint; 1: formed; 2: carried by ink or fighting, a little bolder
      ctx.strokeStyle = `rgba(${c[0]},${c[1]},${c[2]},${pass === 0 ? 0.45 : pass === 1 ? 0.82 : 0.95})`;
      ctx.lineWidth = pass === 0 ? 1.4 : pass === 1 ? 1.5 : 1.8;
      ctx.beginPath();
      for (let i = 0; i < N; i++) {
        const K = S.KIND[kind[i]]; if (K.side !== side || state[i] >= 2) continue;
        const p = state[i] === 1 ? 0 : (onInk[i] || fight[i]) ? 2 : 1; if (p !== pass) continue;
        const L = K.cav ? 3.4 : 2.1, x = px[i], y = py[i];
        if (p === 0) { ctx.moveTo(x - 0.5, y); ctx.lineTo(x + 0.5, y); continue; }
        // a tick across the way it faces (a man's shoulders), horse a longer stroke along it
        if (K.cav) { ctx.moveTo(x - fx[i] * L, y - fy[i] * L); ctx.lineTo(x + fx[i] * L, y + fy[i] * L); }
        else { ctx.moveTo(x - fy[i] * L, y + fx[i] * L); ctx.lineTo(x + fy[i] * L, y - fx[i] * L); }
      }
      ctx.stroke();
    }
  }

  let finger = null;
  function render() {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = PAPER; ctx.fillRect(0, 0, innerWidth, innerHeight);
    ctx.setTransform(dpr * scale, 0, 0, dpr * scale, dpr * ox, dpr * oy);
    ctx.drawImage(paper, X0, 0, W - X0, H);
    drawInk();
    drawUnits();
    if (film && finger) { ctx.fillStyle = 'rgba(60,50,40,0.10)'; ctx.beginPath(); ctx.arc(finger[0], finger[1], 20, 0, Math.PI * 2); ctx.fill(); }
  }

  // ---------- the finger ----------
  let cur = -1;
  const toSim = e => [(e.clientX - ox) / scale, (e.clientY - oy) / scale];
  cv.addEventListener('pointerdown', e => { const p = toSim(e); if (!S.started) S.start(); cur = S.beginStroke(p[0], p[1], clock); finger = p; cv.setPointerCapture(e.pointerId); });
  cv.addEventListener('pointermove', e => { if (cur < 0) return; const p = toSim(e); S.extendStroke(cur, p[0], p[1]); finger = p; });
  const up = () => { if (cur >= 0) { S.endStroke(cur); if (S.strokes[cur].len < 8) S.strokes[cur].pts.length = 1; } cur = -1; finger = null; };
  cv.addEventListener('pointerup', up); cv.addEventListener('pointercancel', up);

  const DT = 1 / 30;
  function tick(steps) { for (let k = 0; k < steps; k++) { S.step(DT); } clock += DT * Math.max(1, steps * 0.5); render(); }
  window.__tick = n => tick(n);
  window.__census = () => S.census();
  addEventListener('resize', () => { resize(); render(); });
  S.deploy(); resize(); render();
  if (!manual) { const loop = () => { tick(2); requestAnimationFrame(loop); }; requestAnimationFrame(loop); }
})();
