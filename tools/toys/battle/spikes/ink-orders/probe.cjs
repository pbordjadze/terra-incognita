#!/usr/bin/env node
// Ink orders: run the simulation without a browser, to tune the rules fast.
//   node probe.cjs [good|bad|none ...]
// prints, every 30 film frames, each side's [formed, fleeing, fallen or gone] (Rome first, then Carthage).
// Plays the same plans on the same clock as render.cjs (plans.cjs).
'use strict';
const fs = require('fs'), vm = require('vm'), path = require('path');
const { PLANS, schedule } = require('./plans.cjs');

function run(name) {
  const window = {}; const ctx = vm.createContext({ window, Math, Float32Array, Int16Array, Int32Array, Uint8Array, Uint16Array, console });
  for (const f of ['terrain.js', 'sim.js']) vm.runInContext(fs.readFileSync(path.join(__dirname, f), 'utf8'), ctx);
  const S = window.SIM; S.setRiver(window.TERRAIN.river); S.deploy();
  const out = []; let k = -1;
  for (const a of schedule(PLANS[name])) {
    if (a.t === 'down') { if (!S.started) S.start(); k = S.beginStroke(a.x, a.y, 0); }
    else if (a.t === 'move') S.extendStroke(k, a.x, a.y);
    else if (a.t === 'up') S.endStroke(k);
    else if (a.t === 'start') S.start();
    else { for (let i = 0; i < a.n; i++) S.step(1 / 30); if (a.f % 30 === 0 && a.f) out.push(a.f + JSON.stringify(S.census()).replace(/\],\[/, '|')); if (process.env.KINDS && a.f % 30 === 0) { const c = [0, 0, 0, 0, 0, 0, 0]; for (let i = 0; i < S.N; i++) if (S.state[i] === 0) c[S.kind[i]]++; console.log('   ', a.f, 'formed by kind', c.join(' ')); } }
  }
  console.log(name.padEnd(5), out.join(' '));
  // who broke, and what broke them: [rear, surround, panic, blows, outnumbered] per kind
  const w = {}; for (let i = 0; i < S.N; i++) if (S.why[i]) { const k = S.kind[i]; (w[k] = w[k] || [0, 0, 0, 0, 0])[S.why[i] - 1]++; }
  console.log('      broke by kind:', JSON.stringify(w));
}
for (const n of (process.argv.length > 2 ? process.argv.slice(2) : ['good', 'bad'])) run(n);
