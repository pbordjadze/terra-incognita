// probe.cjs: runs the pressure-front world in Node, without drawing, for tuning.
//   node probe.cjs [steps] [good|bad|none]
// Prints, every 40 steps, each army's share left, share broken, centre of mass and spread.
// Strokes are laid at once at the plan's start (no finger timing), so it is only a rough guide.
const fs = require('fs'), path = require('path'), vm = require('vm');
const ctx = { window: {}, atob: s => Buffer.from(s, 'base64').toString('binary') };
ctx.window = ctx; vm.createContext(ctx);
for (const f of ['terrain.js', 'sim.js']) vm.runInContext(fs.readFileSync(path.join(__dirname, f), 'utf8'), ctx);
const W = new ctx.PF.World({ seed: 216 }), GW = ctx.PF.GW;
const steps = +(process.argv[2] || 400), plan = process.argv[3] || 'none';
const PLANS = { good: [[[31, 63], [29, 52], [30, 38], [33, 25], [38, 13], [55, 9], [80, 9], [100, 12], [110, 18]], [[43, 64], [38, 54], [36, 44], [41, 35]], [[103, 64], [109, 54], [111, 44], [105, 35]]],
  bad: [[[73, 52], [73, 42], [73, 32]], [[43, 64], [43, 52], [43, 40]], [[103, 64], [103, 52], [103, 40]], [[31, 63], [31, 50], [32, 36]]] };
for (const k in PLANS) PLANS[k] = PLANS[k].map(p => p.map(([x, y]) => [x, y + 14]));
const K = W.armies[1];
if (PLANS[plan]) for (const p of PLANS[plan]) W.addStroke(K, p);
if (plan === 'good') { const p = W.addPin(K, 73, 70); p.held = false; p.released = 40; }
for (let s = 0; s <= steps; s++) {
  if (s % 40 === 0) console.log(s, W.armies.map(A => {
    let m = 0, mx = 0, my = 0, b = 0, top = 0;
    for (let i = 0; i < A.rho.length; i++) { const r = A.rho[i]; top = Math.max(top, r + W.armies[0].rho[i] * (A === W.armies[1]) ); m += r; mx += r * (i % GW); my += r * Math.floor(i / GW); if (r > 1e-4 && A.q[i] / r < 0.5) b += r; }
    return `${A.name} left ${(m / A.start).toFixed(2)} broken ${(b / A.start).toFixed(2)} at ${(mx / m).toFixed(0)},${(my / m).toFixed(0)} max ${top.toFixed(2)}`;
  }).join(' | '));
  if (s % 60 === 0 && process.env.DUMP) dump(s);
  W.tick();
}
// a crude picture (rome red, carthage blue, the front black) as a PPM, 3 px a cell
function dump(s) {
  const GH = ctx.PF.GH, S = 3, w = GW * S, h = GH * S, buf = Buffer.alloc(w * h * 3);
  const [R, C] = W.armies;
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const i = Math.floor(y / S) * GW + Math.floor(x / S), o = (y * w + x) * 3;
    const r = Math.min(1, R.rho[i]), c = Math.min(1, C.rho[i]), f = Math.min(1, W.contact[i]);
    const rb = r > 1e-3 && R.q[i] / R.rho[i] < 0.5, cb = c > 1e-3 && C.q[i] / C.rho[i] < 0.5;
    let pr = 240 - c * 200 - (rb ? 0 : 0), pg = 235 - r * 190 - c * 150, pb = 225 - r * 200;
    if (!W.G.pass[i]) { pr = 120; pg = 160; pb = 200; }
    if (rb || cb) pg += 60;
    buf[o] = Math.max(0, pr * (1 - f)); buf[o + 1] = Math.max(0, Math.min(255, pg) * (1 - f)); buf[o + 2] = Math.max(0, pb * (1 - f));
  }
  fs.writeFileSync(`${process.env.DUMP}-${String(s).padStart(4, '0')}.ppm`, Buffer.concat([Buffer.from(`P6 ${w} ${h} 255\n`), buf]));
}
