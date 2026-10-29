#!/usr/bin/env node
// Holds a pack's things on dated ways (runtime 5) to where its script placed them before: runs the pack's
// scripts headless against a stand-in terra twice, the old revision's (from git) and the working tree's,
// paused at each date along its strip, and compares every sprite's place: the old one where the script put
// it (`at`), the new one where the globe would put it on its dated way (great circles between the two
// places whose dates the date lies between, as GlobeMarker and the preview do). Prints the largest gap;
// exit 1 above the tolerance (1e-5°, a metre). At the very date of a jump (two places sharing a date) a
// dated way shows the place before it, which a script may not have: --offset 0.13 steps between such dates.
//   node tools/playground/dated_check.cjs <pack id> [<git revision, default main>] [--step days] [--offset days] [--tolerance degrees]
// (written for painting-company row 183, when the lead line's ship and the monsoon's dhows moved onto dated ways)
'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm'), { execFileSync } = require('child_process');
const ROOT = path.join(__dirname, '../..');
const D = Math.PI / 180;
const unit = (p) => [Math.cos(p[1] * D) * Math.sin(p[0] * D), Math.sin(p[1] * D), Math.cos(p[1] * D) * Math.cos(p[0] * D)];
const arc = (a, b) => { const u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))) / D; };
function slerp(a, b, t) {
  const u = unit(a), v = unit(b), w = arc(a, b) * D;
  if (w < 1e-12) return a;
  const ka = Math.sin((1 - t) * w) / Math.sin(w), kb = Math.sin(t * w) / Math.sin(w), p = [0, 1, 2].map((i) => u[i] * ka + v[i] * kb);
  return [Math.atan2(p[0], p[2]) / D, Math.asin(Math.max(-1, Math.min(1, p[1]))) / D];
}
function onWay(way, date) {          // GlobeMarker.Path.position(atDate:), the preview's datedPlace
  const d = way.dates, c = way.coordinates, n = d.length;
  if (date < d[0] || date > d[n - 1]) return null;
  let i = 1;
  while (i < n - 1 && d[i] < date) i++;
  const span = d[i] - d[i - 1];
  return slerp(c[i - 1], c[i], span > 0 ? (date - d[i - 1]) / span : 1);
}

/** Runs a pack's scripts (name → text) against a stand-in terra; returns { show, setDate(v) → { id: place } }. */
function run(read, manifest) {
  const listeners = {}, sprites = {};
  let show = null;
  const noop = () => {};
  const layer = { set: noop, remove: noop, clear: noop };
  const terra = {
    runtime: 6, pack: { id: manifest.id, revision: 0, origin: 'preview' }, focus: true,
    settings: { reduceMotion: false, textScale: 1, active: true, wikipedia: false },
    entry: { by: 'workbench', lon: null, lat: null, zoom: null },
    camera: { lon: 60, lat: 10, zoom: 1, cx: 200, cy: 400, radius: 180, width: 400, height: 800 },
    on: (type, fn) => { (listeners[type] = listeners[type] || []).push(fn); if (type === 'settings') fn(terra.settings); },
    off: noop, close: noop, flyTo: noop, log: noop, read: (p) => read(p), readJSON: (p) => JSON.parse(read(p)),
    project: () => ({ x: 0, y: 0, facing: 1, visible: true }), unproject: () => null,
    geo: layer, images: layer,
    sprites: { set: (id, o) => { sprites[id] = o; }, remove: (id) => { delete sprites[id]; }, clear: () => { for (const k in sprites) delete sprites[k]; } },
    touch: { scheme: noop }, caption: { set: noop }, globe: { set: noop }, panel: { show: noop, hide: noop },
    time: { show: (o) => { show = o; }, set: noop, play: noop, speed: noop, hide: noop },
  };
  const ctx = { terra, console: { log: noop, warn: noop, error: console.error }, setTimeout: () => 0, clearTimeout: noop, setInterval: () => 0,
                clearInterval: noop, requestAnimationFrame: () => 0, cancelAnimationFrame: noop, performance: { now: () => 0 }, Date, Math, JSON };
  ctx.window = ctx; ctx.globalThis = ctx;
  vm.createContext(ctx);
  for (const s of manifest.scripts) vm.runInContext(read(s), ctx, { filename: s });
  const emit = (type, v) => (listeners[type] || []).forEach((fn) => fn(v));
  return {
    show: () => show,
    emit,
    sprites,
    at(date) {
      emit('time', { value: date, playing: false, by: 'reader' });
      const out = {};
      for (const [id, o] of Object.entries(sprites)) {
        if (o.at) out[id] = o.at;
        else if (o.path && o.path.dates) out[id] = onWay(o.path, date);
      }
      return out;
    },
  };
}

function main(argv) {
  const opts = { step: 1, offset: 0, tolerance: 1e-5 }, rest = [];
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--step') opts.step = +argv[++i];
    else if (argv[i] === '--tolerance') opts.tolerance = +argv[++i];
    else if (argv[i] === '--offset') opts.offset = +argv[++i];
    else rest.push(argv[i]);
  }
  const [id, rev = 'main'] = rest;
  if (!id) { console.error('usage: node tools/playground/dated_check.cjs <pack id> [<git revision>] [--step days] [--tolerance degrees]'); process.exit(2); }
  const dir = path.join('Prototypes', id);
  const nowRead = (p) => fs.readFileSync(path.join(ROOT, dir, p), 'utf8');
  const oldRead = (p) => execFileSync('git', ['-C', ROOT, 'show', `${rev}:${dir}/${p}`], { encoding: 'utf8', maxBuffer: 1 << 28 });
  const before = run(oldRead, JSON.parse(oldRead('prototype.json'))), after = run(nowRead, JSON.parse(nowRead('prototype.json')));
  const show = after.show();
  const lo = show.min != null ? show.min : show.pace[0][0], hi = show.max != null ? show.max : show.pace[show.pace.length - 1][0];
  let worst = { gap: 0 }, compared = 0, missing = 0;
  for (let date = lo + opts.offset; date <= hi; date += opts.step) {
    const a = before.at(date), b = after.at(date);
    for (const sid of Object.keys(a)) {
      if (!(sid in b) || !b[sid]) { missing++; continue; }
      const gap = arc(a[sid], b[sid]);
      compared++;
      if (gap > worst.gap) worst = { gap, id: sid, date, before: a[sid], after: b[sid] };
    }
  }
  console.log(`${id}: ${compared} places compared from ${lo} to ${hi}; largest gap ${worst.gap.toExponential(2)}°` +
              (worst.id ? ` (${worst.id} at ${worst.date})` : '') + (missing ? `; ${missing} not on a way` : ''));
  process.exitCode = worst.gap > opts.tolerance || missing ? 1 : 0;
}
if (require.main === module) main(process.argv.slice(2));
module.exports = { run, onWay, arc };
