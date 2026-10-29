#!/usr/bin/env node
// Holds the packs' shared script (Prototypes/_shared/shared.js) to the helpers it replaced, bit for bit: the
// great-circle `along`, `arc` and `unit` the lead line, the monsoon and the whales each wrote, the herds'
// `slerp`, and `frames`, run on random places (and the same place twice). Their old text is kept here, so a
// change to the shared script that would move anything a pack draws fails.
//   node tools/playground/shared_check.cjs
'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm');
const ctx = {};
vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../../Prototypes/_shared/shared.js'), 'utf8'), ctx);
const S = ctx.Shared;
const D = Math.PI / 180;
// ---- what the packs wrote, as they wrote it (lead line, monsoon, whales; the herds' slerp)
function unit(p) { var la = p[1] * D, lo = p[0] * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; }
function arc(a, b) { var u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))); }
function along(a, b, t) {
  var u = unit(a), v = unit(b), w = arc(a, b);
  if (w < 1e-9) return a.slice();
  var sa = Math.sin((1 - t) * w) / Math.sin(w), sb = Math.sin(t * w) / Math.sin(w);
  var p = [u[0] * sa + v[0] * sb, u[1] * sa + v[1] * sb, u[2] * sa + v[2] * sb];
  return [Math.atan2(p[0], p[2]) / D, Math.asin(Math.max(-1, Math.min(1, p[1]))) / D];
}
function herdUnit(lon, lat) { return [Math.cos(lat * D) * Math.sin(lon * D), Math.sin(lat * D), Math.cos(lat * D) * Math.cos(lon * D)]; }
function slerp(p, q, f) {
  var a = herdUnit(p[0], p[1]), b = herdUnit(q[0], q[1]), d = Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2]));
  var om = Math.acos(d);
  if (om < 1e-9) return [p[0], p[1]];
  var ka = Math.sin((1 - f) * om) / Math.sin(om), kb = Math.sin(f * om) / Math.sin(om);
  var v = [a[0] * ka + b[0] * kb, a[1] * ka + b[1] * kb, a[2] * ka + b[2] * kb];
  return [Math.atan2(v[0], v[2]) / D, Math.asin(Math.max(-1, Math.min(1, v[1]))) / D];
}
function degrees(p, q) { var a = herdUnit(p[0], p[1]), b = herdUnit(q[0], q[1]); return Math.acos(Math.max(-1, Math.min(1, a[0] * b[0] + a[1] * b[1] + a[2] * b[2]))) / D; }
function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }

let seed = 183, failures = 0, n = 0;
const rand = () => ((seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648);
const place = () => [rand() * 360 - 180, rand() * 180 - 90];
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
function expect(what, a, b) { n++; if (!same(a, b)) { failures++; if (failures < 10) console.error(`FAIL ${what}: ${JSON.stringify(a)} != ${JSON.stringify(b)}`); } }
for (let i = 0; i < 20000; i++) {
  const a = place(), b = i % 50 === 0 ? a.slice() : place(), t = i % 7 === 0 ? [0, 1, 0.5][i % 3] : rand();
  expect('along', S.along(a, b, t), along(a, b, t));
  expect('slerp', S.along(a.concat([-1e6]), b.concat([2e6]), t), slerp(a.concat([-1e6]), b.concat([2e6]), t));
  expect('arc', S.arc(a, b), arc(a, b));
  expect('degrees', S.arc(a, b) / D, degrees(a, b));
  expect('unit', S.unit(a), unit(a));
}
expect('frames', S.frames('whale', 8), frames('whale', 8));
console.log(failures ? `FAIL: ${failures} of ${n}` : `PASS: ${n} comparisons, bit for bit`);
process.exitCode = failures ? 1 : 0;
