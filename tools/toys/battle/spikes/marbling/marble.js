// Marbling: drops of pigment on a tray of size, moved only by exact, ink-preserving maps (after Jaffer,
// "Mathematical Marbling", and Lu, Jaffer et al. 2012). Every drop is a closed polygon; a new drop pushes
// every older boundary out by p -> c + (p-c)*sqrt(1 + r^2/|p-c|^2) (area-exact), and a stylus step moves
// every boundary by p -> p + v*g(|p-s|). Polygons are drawn oldest first: a later drop always lies "on top"
// because it occupies the space it opened. Edges are refined where a map stretches them.
'use strict';

const W = 1376, H = 1032;

// The armies, in their order of battle at Cannae (turned ground: the river on the left, Romans above,
// facing down; Carthaginians below, facing up). Colours are an ebru palette.
const C = {
  madder: [168, 46, 40], ochre: [222, 168, 70], rose: [196, 104, 116], cream: [238, 226, 200],
  indigo: [38, 58, 112], pale: [150, 172, 202], linen: [236, 230, 214], purple: [92, 44, 98],
  green: [44, 74, 58], sage: [128, 150, 120], sumi: [26, 25, 30], grey: [120, 118, 122],
  gold: [196, 138, 40], umber: [110, 70, 36],
  buff: [206, 190, 158], mist: [176, 186, 188], fawn: [196, 168, 140],
};

function armies(rng) {
  const drops = []; // {x,y,rings:[[r2,col],...], t}
  const jit = (a) => (rng() - 0.5) * a;
  const site = (x, y, rings, t) => drops.push({ x: x + jit(5), y: y + jit(5), rings, t: t + jit(0.25) });
  // Roman legions: the deep centre, a block of maniples, madder with an ochre heart.
  const cx = 900;
  for (let r = 0; r < 4; r++) for (let c = 0; c < 10; c++) {
    const x = cx + (c - 4.5) * 50 + (r % 2 ? 12 : -12), y = 205 + r * 50;
    site(x, y, [[360, C.madder], [220, C.ochre], [130, C.madder], [40, C.ochre]], 0.9 + r * 0.18 + c * 0.05);
  }
  // Roman horse by the river (their right), allied horse on their left.
  for (let i = 0; i < 6; i++) site(515 + (i % 2) * 40, 225 + Math.floor(i / 2) * 42, [[230, C.rose], [110, C.cream], [40, C.rose]], 0.3 + i * 0.07);
  for (let i = 0; i < 6; i++) site(1245 + (i % 2) * 40, 225 + Math.floor(i / 2) * 42, [[230, C.rose], [110, C.cream], [40, C.rose]], 0.4 + i * 0.07);
  // Hannibal's crescent, bowed toward the Romans: Gauls and Spaniards in alternate companies.
  for (let k = 0; k < 2; k++) for (let i = 0; i < 11; i++) {
    const a = (i - 5) / 5, x = cx + a * 245, y = 590 + k * 54 - 95 * (1 - a * a);
    const gaul = (i + k) % 2 === 0;
    site(x, y, gaul ? [[330, C.indigo], [190, C.pale], [100, C.indigo]] : [[330, C.purple], [210, C.linen], [70, C.purple]], 2.1 + i * 0.05 + k * 0.25);
  }
  // African heavy infantry on both wings, held back.
  for (const side of [-1, 1]) for (let r = 0; r < 3; r++) for (let c = 0; c < 3; c++) {
    site(cx + side * (300 + c * 44) - side * 22, 590 + r * 44, [[260, C.green], [150, C.sage], [70, C.green]], 2.9 + r * 0.12 + c * 0.06);
  }
  // Hasdrubal's heavy horse by the river; the Numidians on the far wing.
  for (let i = 0; i < 8; i++) site(445 + (i % 2) * 42, 560 + Math.floor(i / 2) * 42, [[250, C.sumi], [120, C.grey], [40, C.sumi]], 3.5 + i * 0.06);
  for (let i = 0; i < 6; i++) site(1290 + (i % 2) * 42, 580 + Math.floor(i / 2) * 42, [[200, C.gold], [100, C.umber], [40, C.gold]], 3.7 + i * 0.06);
  return drops;
}

function mulberry(a) {
  return () => { a |= 0; a = (a + 0x6D2B79F5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

class Marble {
  constructor(seed = 7) {
    this.polys = []; // {pts: Float64Array-ish array [x0,y0,x1,y1...], col}
    this.pending = []; // drops waiting to fall
    this.growing = []; // {cx,cy,left (r2 still to add), rate}
    this.t = 0;
    this.rng = mulberry(seed);
    this.maxSeg = 2.2; this.minSeg = 0.5; this.armyScale = 1.9;
  }
  sprinkle() {
    // the marbler flicks the brush: first a pale ground of fine spatters over the whole tray, then every
    // army's drops fall, in order, a little at random
    this.t0 = this.t;
    const R = this.rng;
    for (let i = 0; i < 900; i++) {
      const col = R() < 0.55 ? C.buff : (R() < 0.6 ? C.mist : C.fawn);
      this.pending.push({ x: R() * W, y: R() * H, r2: 14 + R() * R() * 170, col, at: this.t + R() * 0.7 });
    }
    for (const d of armies(this.rng)) {
      let tt = d.t;
      for (const [r2, col] of d.rings) { this.pending.push({ x: d.x, y: d.y, r2: r2 * this.armyScale, col, at: this.t + tt }); tt += 0.16 + this.rng() * 0.08; }
    }
    this.pending.sort((a, b) => a.at - b.at);
  }
  // apply a point map f to every polygon, refining stretched edges by mapping their originals' midpoints
  apply(f, moveCentres = true) {
    const maxS2 = this.maxSeg * this.maxSeg, minS2 = this.minSeg * this.minSeg;
    for (const P of this.polys) {
      const src = P.pts, n = src.length / 2, out = [];
      const m = [0, 0];
      let fx0, fy0;
      f(src[0], src[1], m); fx0 = m[0]; fy0 = m[1];
      let ax = src[0], ay = src[1], fax = fx0, fay = fy0;
      for (let i = 0; i < n; i++) {
        const j = (i + 1) % n, bx = src[2 * j], by = src[2 * j + 1];
        let fbx, fby;
        if (j === 0) { fbx = fx0; fby = fy0; } else { f(bx, by, m); fbx = m[0]; fby = m[1]; }
        out.push(fax, fay);
        const d2 = (fbx - fax) ** 2 + (fby - fay) ** 2;
        if (d2 > maxS2) {
          const k = Math.min(8, Math.ceil(Math.sqrt(d2 / maxS2)));
          for (let s = 1; s < k; s++) { const u = s / k; f(ax + (bx - ax) * u, ay + (by - ay) * u, m); out.push(m[0], m[1]); }
        }
        ax = bx; ay = by; fax = fbx; fay = fby;
      }
      // thin out crowded vertices
      if (out.length > 32) {
        const th = [out[0], out[1]];
        for (let i = 2; i < out.length; i += 2) {
          const lx = th[th.length - 2], ly = th[th.length - 1];
          if ((out[i] - lx) ** 2 + (out[i + 1] - ly) ** 2 >= minS2) th.push(out[i], out[i + 1]);
        }
        P.pts = th.length >= 16 ? th : out;
      } else P.pts = out;
    }
    // a spreading drop's centre is a point of the fluid too: other maps carry it
    if (moveCentres) { const m = [0, 0]; for (const g of this.growing) { f(g.cx, g.cy, m); g.cx = m[0]; g.cy = m[1]; } }
  }
  dropMap(cx, cy, r2) {
    this.apply((x, y, o) => {
      const dx = x - cx, dy = y - cy, d2 = dx * dx + dy * dy + 1e-9, s = Math.sqrt(1 + r2 / d2);
      o[0] = cx + dx * s; o[1] = cy + dy * s;
    });
  }
  // a stylus step from (x0,y0) to (x1,y1): fluid near the tip goes with it, falling off with distance
  stylus(x0, y0, x1, y1, lam = 40) {
    const len = Math.hypot(x1 - x0, y1 - y0);
    const n = Math.max(1, Math.ceil(len / 3));
    for (let k = 0; k < n; k++) {
      const sx = x0 + (x1 - x0) * (k + 0.5) / n, sy = y0 + (y1 - y0) * (k + 0.5) / n;
      const vx = (x1 - x0) / n, vy = (y1 - y0) / n;
      this.apply((x, y, o) => {
        const d = Math.hypot(x - sx, y - sy), g = (lam / (d + lam)) * Math.exp(-d / (2.5 * lam));
        o[0] = x + vx * g; o[1] = y + vy * g;
      });
    }
  }
  step(dt) {
    this.t += dt;
    while (this.pending.length && this.pending[0].at <= this.t) {
      const d = this.pending.shift();
      // a new drop starts as a speck and opens: its own circle is mapped with everything else
      const N = 48, r0 = 0.6, pts = [];
      for (let i = 0; i < N; i++) { const a = (i / N) * Math.PI * 2; pts.push(d.x + Math.cos(a) * r0, d.y + Math.sin(a) * r0); }
      this.polys.push({ pts, col: d.col, r2: d.r2 });
      this.growing.push({ cx: d.x, cy: d.y, left: d.r2, rate: d.r2 / 0.5 });
    }
    if (this.growing.length) {
      // spreading slows as the film thins: each frame opens a fraction of what is left (all at once)
      // composed in order; each drop's centre is where the drops before it (this frame) carried it
      const k = 1 - Math.exp(-dt * 9), gs = this.growing.map((g) => { const a = g.left < 2 ? g.left : g.left * k; g.left -= a; return [g.cx, g.cy, a]; });
      const one = (x, y, j0, j1) => {
        for (let j = j0; j < j1; j++) {
          const [cx, cy, r2] = gs[j], dx = x - cx, dy = y - cy, d2 = dx * dx + dy * dy + 1e-9, s = Math.sqrt(1 + r2 / d2);
          x = cx + dx * s; y = cy + dy * s;
        }
        return [x, y];
      };
      for (let i = 0; i < gs.length; i++) { const c = one(gs[i][0], gs[i][1], 0, i); gs[i][0] = c[0]; gs[i][1] = c[1]; }
      this.apply((x, y, o) => { const c = one(x, y, 0, gs.length); o[0] = c[0]; o[1] = c[1]; }, false);
      this.growing.forEach((g, i) => { const c = one(gs[i][0], gs[i][1], i + 1, gs.length); g.cx = c[0]; g.cy = c[1]; });
      this.growing = this.growing.filter((g) => g.left > 0);
    }
  }
  vertexCount() { let n = 0; for (const p of this.polys) n += p.pts.length / 2; return n; }
  draw(ctx, scale) {
    ctx.setTransform(scale, 0, 0, scale, 0, 0);
    ctx.clearRect(0, 0, W, H);
    for (const P of this.polys) {
      const p = P.pts; ctx.beginPath(); ctx.moveTo(p[0], p[1]);
      for (let i = 2; i < p.length; i += 2) ctx.lineTo(p[i], p[i + 1]);
      ctx.closePath();
      const [r, g, b] = P.col;
      // the dispersant's thin pale line where a drop pushed the colour before it
      ctx.strokeStyle = `rgba(${(r + 246) / 2 | 0},${(g + 238) / 2 | 0},${(b + 220) / 2 | 0},0.55)`;
      ctx.lineWidth = 1.1; ctx.lineJoin = 'round'; if (P.r2 > 200) ctx.stroke();
      ctx.fillStyle = `rgb(${r},${g},${b})`;
      ctx.fill();
    }
  }
}

window.Marble = Marble; window.MW = W; window.MH = H;
