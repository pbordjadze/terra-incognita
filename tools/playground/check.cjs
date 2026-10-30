'use strict';
// Runs a runtime-3 pack in headless Chromium as the desktop preview hosts it (tools/playground/serve.cjs), and fails
// on what the phone would trip over: a script error, a terra.log at level error, a contract the host refuses, a
// request that is not local, a pack that never starts. Screenshots go to build/playground/<id>/<device>-*.png:
// the doorway (if any), the anchor, sprites after a few seconds, closer zooms where something shows only up close,
// and what touching the first touchable thing does.
//   node tools/playground/check.cjs <id> [--device iphone|ipad] [--wikipedia] [--root <dir>]...
//   node tools/playground/check.cjs --all [--device iphone|ipad] [--root <dir>]...
//   node tools/playground/check.cjs --living [<id> <id>] [--device iphone|ipad] [--root <dir>]...
// --root adds a folder of pack folders (a pack being made outside Prototypes/). --wikipedia runs it with Wikipedia's
// words on (the reader's switch: the pack's wikipedia.json where its paragraphs were), its screenshots named so.
// Exit code 1 if any check failed.
// --living checks the living globe (docs/runtime3.md › Many at once): every current pack's doorway on the one
// globe, two packs (the first two with doorways, or the two named) started by touching their doorways, run
// side by side, the second in front; a touch on the first's things brings it back to the front; ending the one
// in front leaves the other exactly as it was. Screenshots go to build/playground/_living/.
const { chromium } = require('playwright');
const fs = require('node:fs'), path = require('node:path');
const { start, listPacks, RUNTIMES } = require('./serve.cjs');

const ROOT = path.resolve(__dirname, '../..');
const LOCAL = new Set(['127.0.0.1', 'localhost']);
const SIZES = { iphone: { width: 393, height: 852 }, ipad: { width: 1032, height: 1376 } };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function usage(message) {
  console.error((message ? message + '\n' : '') + 'usage: node tools/playground/check.cjs <id> | --all  [--device iphone|ipad] [--root <dir>]...');
  process.exit(2);
}

// Wikipedia's words on (--wikipedia): the screenshots are named apart, so both sets of a pack's can stand side by side
let WIKIPEDIA = false;
const shotPrefix = (device) => device + '-' + (WIKIPEDIA ? 'wikipedia-' : '');

function parseArgs(argv) {
  const out = { ids: [], all: false, living: false, device: 'iphone', roots: [] };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--all') out.all = true;
    else if (a === '--living') out.living = true;
    else if (a === '--device') out.device = argv[++i];
    else if (a === '--wikipedia') out.wikipedia = true;
    else if (a === '--root') out.roots.push(argv[++i]);
    else if (a === '-h' || a === '--help') usage();
    else if (a.startsWith('--')) usage('unknown option ' + a);
    else out.ids.push(a);
  }
  if (out.device === 'phone') out.device = 'iphone';
  if (!SIZES[out.device]) usage(`--device is iphone or ipad (got ${out.device})`);
  for (const r of out.roots) if (!r || !fs.existsSync(r)) usage(`--root ${r || ''}: no such folder`);
  if (!out.all && !out.living && !out.ids.length) usage('which pack?');
  if (out.living && out.ids.length && out.ids.length !== 2) usage('--living takes two packs, or none (the first two with doorways)');
  return out;
}

/** The living globe: several packs at once, by touch, on /atlas. */
async function checkLiving(browser, server, ids, device) {
  const size = SIZES[device], outDir = path.join(ROOT, 'build', 'playground', '_living');
  fs.mkdirSync(outDir, { recursive: true });
  for (const f of fs.readdirSync(outDir)) if (f.startsWith(shotPrefix(device)) && f.endsWith('.png')) fs.rmSync(path.join(outDir, f));
  const failures = [], warnings = [], shots = [], steps = [], nonLocal = [];
  const fail = (m) => { if (!failures.includes(m)) failures.push(m); };
  const warn = (m) => { if (!warnings.includes(m)) warnings.push(m); };
  const started = Date.now();
  let logs = [];
  const context = await browser.newContext({ viewport: size, deviceScaleFactor: 2 });
  await context.route((url) => !LOCAL.has(url.hostname) && !['data:', 'blob:', 'about:'].includes(url.protocol), (route) => {
    nonLocal.push(route.request().url());
    route.abort();
  });
  const page = await context.newPage();
  page.on('pageerror', (e) => fail('preview page error: ' + e.message));
  page.on('console', (m) => { if (m.type() === 'error') fail('console error: ' + m.text()); });
  page.on('response', (r) => { if (r.status() >= 400 && !r.url().endsWith('/favicon.ico')) fail(`${r.status()} for ${r.url().replace(server.url, '')}`); });
  const pg = (fn, arg) => page.evaluate(fn, arg);
  const shot = async (name) => {
    const file = `${shotPrefix(device)}${name}.png`;
    await page.screenshot({ path: path.join(outDir, file) });
    shots.push(path.relative(ROOT, path.join(outDir, file)));
    steps.push(`+${((Date.now() - started) / 1000).toFixed(1)}s screenshot ${file}`);
  };
  const settle = async (ms, what) => {
    try { await page.waitForFunction(() => window.__playground.settled(), null, { timeout: ms || 8000 }); }
    catch (e) { warn(`${what || 'the preview'} did not settle (a flight or a picture still loading)`); }
    const f = await pg(() => { window.__playground.redraw(); return window.__playground.frame; });
    await page.waitForFunction((f0) => window.__playground.frame >= f0 + 2, f, { timeout: 2000 }).catch(() => {});
  };
  // the globe over a doorway, close enough for its picture itself (its last look) to show, then a touch on it
  const enter = async (id) => {
    const d = (await pg(() => window.__playground.doorways())).find((x) => x.pack === id);
    if (!d) { fail(`${id} has no doorway on the living globe`); return false; }
    // close enough for its picture itself (the look that needs the most zoom), not its glint from afar
    const close = d.looks.reduce((a, b) => (b.minZoom > a.minZoom ? b : a), d.looks[0]);
    await pg((a) => window.__playground.flyTo(a.lon, a.lat, a.z, 0), { lon: d.lon, lat: d.lat, z: Math.min(8, Math.max(1, close.minZoom + 0.25)) });
    await settle(4000, `the doorway of ${id}`);
    await sleep(300);
    const now = (await pg(() => window.__playground.doorways())).find((x) => x.pack === id);
    if (!now || !now.visible) { fail(`${id}'s doorway is not drawn at its own place and zoom`); return false; }
    await page.mouse.click(now.x, now.y);
    steps.push(`touched the doorway "${now.label}" (${id})`);
    try {
      await page.waitForFunction((pid) => (window.__playground.stages().find((s) => s.id === pid) || {}).started, id, { timeout: 10000 });
    } catch (e) { fail(`${id} never started from its doorway`); return false; }
    await settle(8000, id);
    await sleep(300);
    return true;
  };
  const stagesNow = () => pg(() => window.__playground.stages());
  const counts = (s) => s && `${s.geo} geo, ${s.images} pictures, ${s.sprites} sprites`;
  try {
    await page.goto(`${server.url}/atlas?device=${device}&bare=1&idleAfter=100000${WIKIPEDIA ? '&wikipedia=1' : ''}`);
    await page.waitForFunction(() => window.__playground && window.__playground.ready, null, { timeout: 15000 });
    const doors = await pg(() => window.__playground.doorways());
    steps.push(`${doors.length} doorways on the globe: ${doors.map((d) => d.pack).join(', ')}`);
    if (!ids.length) ids = doors.slice(0, 2).map((d) => d.pack);
    if (ids.length < 2) { fail('the living globe needs two packs with doorways'); throw new Error('stopped'); }
    const [a, b] = ids;
    await settle(4000, 'the living globe');
    await shot('doorways');

    if (!(await enter(a))) throw new Error('stopped');
    let st = await stagesNow();
    const A1 = st.find((s) => s.id === a);
    steps.push(`${a} runs: ${counts(A1)}; entry ${A1 && A1.entry}`);
    if (!A1 || A1.entry !== 'doorway') fail(`${a} should know it was entered by its doorway (got ${A1 && A1.entry})`);
    await shot('one');

    if (!(await enter(b))) throw new Error('stopped');
    st = await stagesNow();
    const A2 = st.find((s) => s.id === a), B2 = st.find((s) => s.id === b);
    if (!A2) fail(`starting ${b} ended ${a}: the stage is shared`);
    if (!B2 || !B2.front) fail(`${b}, just started, should be in front`);
    if (A2 && A2.front) fail(`${a} should be behind ${b}`);
    if (A2 && counts(A2) !== counts(A1)) fail(`${a}'s things changed when ${b} started (${counts(A1)} → ${counts(A2)})`);
    steps.push(`${b} runs in front: ${counts(B2)}; ${a} behind: ${counts(A2)}`);
    await shot('two');

    // a touch on something of the first (or its row) brings it to the front; the second stays
    const front1 = await pg(() => window.__playground.front());
    const touchable = (await pg(() => window.__playground.touchables())).find((t) => t.stage === a);
    if (touchable) {
      await page.mouse.click(touchable.x, touchable.y);
      await sleep(400);
      steps.push(`touched ${touchable.what} of ${a}`);
    } else {
      await pg((pid) => window.__playground.bringToFront(pid), a);
      steps.push(`nothing touchable of ${a} in view: brought to the front as a workbench row would`);
    }
    const front2 = await pg(() => window.__playground.front());
    if (front1 !== b || front2 !== a) fail(`the front should pass from ${b} to ${a} by touch (was ${front1}, then ${front2})`);
    st = await stagesNow();
    if (!st.find((s) => s.id === b)) fail(`${b} should still run behind ${a}`);
    await settle(4000, 'after the touch');
    await shot('front-again');

    // ending the one in front leaves the other exactly as it was
    const before = st.find((s) => s.id === b);
    await pg(() => window.__playground.end(window.__playground.front()));
    await sleep(300);
    st = await stagesNow();
    if (st.find((s) => s.id === a)) fail(`✕ should end ${a}, the one in front`);
    const after = st.find((s) => s.id === b);
    if (!after) fail(`ending ${a} should leave ${b} running`);
    else if (counts(after) !== counts(before) || !after.front) fail(`${b} should be as it was, in front (${counts(before)} → ${counts(after)}, front ${after.front})`);
    steps.push(`ended ${a}: ${b} ${after ? 'in front, ' + counts(after) : 'gone'}`);
    const door = (await pg(() => window.__playground.doorways())).find((d) => d.pack === a);
    if (!door || door.running) fail(`${a}'s doorway should be back on the globe once it has ended`);
    await settle(4000, 'after ending');
    await shot('one-left');
    await pg(() => window.__playground.endAll());
    await sleep(300);
    if ((await stagesNow()).length) fail('ending all should leave nothing running');
    const back = await pg(() => window.__playground.doorways());
    if (back.some((d) => d.running)) fail('every doorway should be back on the globe');
    steps.push('ended all: every doorway back');
  } catch (e) {
    if (e.message !== 'stopped') fail('check stopped: ' + e.message.split('\n')[0]);
  }
  try { logs = await pg(() => window.__playground.log); } catch (e) { /* the page is gone */ }
  for (const l of logs) if (l.level === 'error') fail(`${l.source === 'script' ? 'script' : l.source === 'host' ? 'host refused' : l.source}: ${l.message}`);
  for (const l of logs) if (l.level === 'warn') warn(`${l.source}: ${l.message}`);
  for (const u of nonLocal) fail('request to a non-local host: ' + u);
  await context.close();
  const result = { id: 'the living globe (' + ids.join(' + ') + ')', folder: '_living', device, passed: failures.length === 0, seconds: +((Date.now() - started) / 1000).toFixed(1),
                   failures, warnings, screenshots: shots, steps, logs: logs.slice(-300).map((l) => Object.assign({}, l, { t: +l.t.toFixed(2) })) };
  fs.writeFileSync(path.join(outDir, `check-${device}.json`), JSON.stringify(result, null, 2) + '\n');
  return result;
}

async function checkPack(browser, server, pack, device) {
  const id = pack.id, size = SIZES[device];
  const outDir = path.join(ROOT, 'build', 'playground', id);
  fs.mkdirSync(outDir, { recursive: true });
  for (const f of fs.readdirSync(outDir)) if (f.startsWith(shotPrefix(device)) && f.endsWith('.png')) fs.rmSync(path.join(outDir, f));
  const failures = [], warnings = [], shots = [], steps = [], nonLocal = [];
  const fail = (m) => { if (!failures.includes(m)) failures.push(m); };
  const warn = (m) => { if (!warnings.includes(m)) warnings.push(m); };
  const started = Date.now();
  let logs = [], state = null, touched = null;

  const context = await browser.newContext({ viewport: size, deviceScaleFactor: 2 });
  // nothing leaves this machine, whatever the pack asks for
  await context.route((url) => !LOCAL.has(url.hostname) && !['data:', 'blob:', 'about:'].includes(url.protocol), (route) => {
    nonLocal.push(route.request().url());
    route.abort();
  });
  const page = await context.newPage();
  page.on('pageerror', (e) => fail('preview page error: ' + e.message));
  page.on('console', (m) => { if (m.type() === 'error') fail('console error: ' + m.text()); });
  page.on('response', (r) => { if (r.status() >= 400 && !r.url().endsWith('/favicon.ico')) fail(`${r.status()} for ${r.url().replace(server.url, '')}`); });
  const pg = (fn, arg) => page.evaluate(fn, arg);
  const shot = async (name) => {
    const file = `${shotPrefix(device)}${name}.png`;
    await page.screenshot({ path: path.join(outDir, file) });
    shots.push(path.relative(ROOT, path.join(outDir, file)));
    steps.push(`+${((Date.now() - started) / 1000).toFixed(1)}s screenshot ${file}`);
  };
  const settle = async (ms, what) => {
    try {
      await page.waitForFunction(() => window.__playground.settled(), null, { timeout: ms || 8000 });
    } catch (e) { warn(`${what || 'the preview'} did not settle (a flight or a picture still loading)`); }
    // two frames drawn after it settled
    const f = await pg(() => { window.__playground.redraw(); return window.__playground.frame; });
    await page.waitForFunction((f0) => window.__playground.frame >= f0 + 2, f, { timeout: 2000 }).catch(() => {});
  };

  try {
    await page.goto(`${server.url}/preview/${id}?device=${device}&bare=1&idleAfter=100000${WIKIPEDIA ? '&wikipedia=1' : ''}`);
    await page.waitForFunction(() => window.__playground && window.__playground.ready, null, { timeout: 15000 });
    steps.push('preview ready');

    const door = await pg(() => window.__playground.doorway());
    if (door) {
      await settle(4000, 'the doorway');
      await shot('doorway');
      const d = await pg(() => window.__playground.doorway());
      if (d.visible) {
        await page.mouse.click(d.x, d.y);
        steps.push(`touched the doorway "${d.label}"`);
      } else {
        warn(`the doorway "${d.label}" is not drawn at its own place and zoom (its look's minZoom/maxZoom, or every/for between showings?): started with Start`);
        await pg(() => window.__playground.start());
      }
    }
    try {
      await page.waitForFunction(() => window.__playground.started || window.__playground.ended, null, { timeout: 10000 });
    } catch (e) { fail('the pack never started (its scripts did not finish loading in 10 s)'); throw new Error('stopped'); }
    if (!(await pg(() => window.__playground.started))) { fail('the pack did not start'); throw new Error('stopped'); }
    steps.push('started');
    await settle(8000, 'the anchor');
    await sleep(250);                                  // what a script does on its first timers
    await settle(4000, 'the anchor');
    await shot('anchor');
    state = await pg(() => window.__playground.state());

    if (state.sprites.length) {
      await sleep(2500);
      await shot('motion');
      const after = await pg(() => window.__playground.state());
      const moved = after.sprites.filter((s, i) => s.moving && s.drawn && state.sprites[i] && state.sprites[i].drawn
        && Math.hypot(s.drawn.x - state.sprites[i].drawn.x, s.drawn.y - state.sprites[i].drawn.y) > 0.5);
      steps.push(`after 2.5 s: ${moved.length} of ${after.sprites.filter((s) => s.moving).length} moving sprites moved on screen`);
      state = after;
    }

    // closer, where something shows only up close (a sprite's look, a picture, a layer with minZoom)
    const anchorZoom = state.camera.zoom;
    const thresholds = [
      ...state.sprites.flatMap((s) => s.looks.map((l) => l.minZoom)),
      ...state.images.map((im) => im.minZoom), ...state.geo.map((L) => L.minZoom),
    ].filter((z) => z > 0 && z <= 8);
    if (thresholds.length) {
      const zooms = [3, 5].filter((z) => z > anchorZoom + 0.2);
      const top = Math.max(...thresholds);
      if (top > 5 && top + 0.5 > anchorZoom + 0.2) zooms.push(Math.min(8, +(top + 0.5).toFixed(2)));
      // where: the first sprite with a close look, or the first picture with a minZoom, or the anchor
      const closeSprite = state.sprites.find((s) => s.looks.some((l) => l.minZoom > 0));
      const closeImage = state.images.find((im) => im.minZoom > 0);
      let focus = { lon: state.camera.lon, lat: state.camera.lat, what: 'the anchor' };
      if (closeSprite) {
        const ll = await pg((sid) => { const s = window.__playground.state().sprites.find((x) => x.id === sid); return s && s.drawn ? window.__playground.unproject(s.drawn.x, s.drawn.y) : null; }, closeSprite.id);
        if (ll) focus = { lon: ll.lon, lat: ll.lat, what: `sprite "${closeSprite.id}"` };
      } else if (closeImage) {
        const [w, s, e, n] = closeImage.bounds;
        focus = { lon: e > w ? (w + e) / 2 : (w + e + 360) / 2, lat: (s + n) / 2, what: `picture "${closeImage.id}"` };
      }
      for (const z of zooms) {
        await pg((a) => window.__playground.flyTo(a.lon, a.lat, a.z, 0), { lon: focus.lon, lat: focus.lat, z });
        await settle(4000, `zoom ${z}`);
        await sleep(400);
        await shot(`zoom-${z}`);
        const s = await pg(() => window.__playground.state());
        const looks = s.sprites.filter((x) => x.drawn).map((x) => `${x.id} look ${x.drawn.look}`).join(', ');
        steps.push(`zoom ${z} on ${focus.what}${looks ? ': ' + looks : ''}${s.images.length ? '; pictures ' + s.images.map((im) => im.id + ' ' + Math.round(im.alpha * 100) + '%').join(', ') : ''}`);
      }
      if (zooms.length) {
        await pg((c) => window.__playground.flyTo(c.lon, c.lat, c.zoom, 0), state.camera);
        await settle(3000, 'back at the anchor');
      }
    }

    // touch the first touchable thing on screen (from further out if none is in view)
    let found = await pg(() => window.__playground.touchables());
    if (!found.length && state.camera.zoom > 1.2) {
      await pg((c) => window.__playground.flyTo(c.lon, c.lat, Math.max(0.8, c.zoom / 2), 0), state.camera);
      await settle(3000, 'further out');
      found = await pg(() => window.__playground.touchables());
    }
    if (found.length) {
      const t = found[0], before = await pg(() => ({ state: window.__playground.state(), touches: window.__playground.touches.length }));
      // (a sprite under way has moved since it was found: tap it where it is now)
      const now = (await pg(() => window.__playground.touchables())).find((u) => JSON.stringify(u.target) === JSON.stringify(t.target)) || t;
      await page.mouse.click(now.x, now.y);
      await sleep(700);
      await settle(3000, 'after the touch');
      await shot('touch');
      const after = await pg(() => ({ state: window.__playground.state(), touches: window.__playground.touches.slice() }));
      const sent = after.touches.slice(before.touches);
      const tap = sent.find((x) => x.phase === 'tap');
      const same = tap && JSON.stringify(tap.target) === JSON.stringify(t.target);
      if (!same) fail(`touching ${t.what} at ${t.x.toFixed(0)}, ${t.y.toFixed(0)} did not reach the pack as a tap on it`);
      const changes = [];
      if (after.state.panel && JSON.stringify(after.state.panel) !== JSON.stringify(before.state.panel)) changes.push(`the panel "${after.state.panel.title}"`);
      if (JSON.stringify(after.state.caption) !== JSON.stringify(before.state.caption)) changes.push(`the caption "${after.state.caption.title}: ${after.state.caption.text}"`);
      if (JSON.stringify(after.state.geo.map((L) => [L.id, L.features])) !== JSON.stringify(before.state.geo.map((L) => [L.id, L.features]))) changes.push('its geography');
      if (JSON.stringify(after.state.sprites.map((s) => s.id)) !== JSON.stringify(before.state.sprites.map((s) => s.id))) changes.push('its sprites');
      touched = { what: t.what, target: t.target, changes };
      steps.push(`tapped ${t.what}: ${changes.length ? 'it showed ' + changes.join(', ') : 'nothing visible changed'}`);
      state = after.state;
    } else {
      warn('nothing touchable on screen at the anchor (or further out): no touch was tried');
    }
  } catch (e) {
    if (e.message !== 'stopped') fail('check stopped: ' + e.message.split('\n')[0]);
  }

  try {
    logs = await pg(() => window.__playground.log);
  } catch (e) { /* the page is gone */ }
  for (const l of logs) if (l.level === 'error') fail(`${l.source === 'script' ? 'script' : l.source === 'host' ? 'host refused' : l.source}: ${l.message}`);
  for (const l of logs) if (l.level === 'warn') warn(`${l.source}: ${l.message}`);
  for (const u of nonLocal) fail('request to a non-local host: ' + u);
  await context.close();

  const result = { id, device, passed: failures.length === 0, seconds: +((Date.now() - started) / 1000).toFixed(1), failures, warnings, screenshots: shots, steps,
                   touched, state: state && { geo: state.geo, images: state.images, sprites: state.sprites, time: state.time, caption: state.caption, panel: state.panel, scheme: state.scheme },
                   logs: logs.slice(-300).map((l) => Object.assign({}, l, { t: +l.t.toFixed(2) })) };
  fs.writeFileSync(path.join(outDir, `check-${device}.json`), JSON.stringify(result, null, 2) + '\n');
  return result;
}

function summarize(r) {
  const s = r.state, lines = [];
  lines.push(`${r.passed ? 'PASS' : 'FAIL'} ${r.id} (${r.device}, ${r.seconds} s): ${r.screenshots.length} screenshots in build/playground/${r.folder || r.id}/`);
  if (s) {
    const n = (k, one, many) => `${k} ${k === 1 ? one : many}`;
    const parts = [n(s.geo.length, 'geo layer', 'geo layers') + (s.geo.some((L) => L.touchable) ? ` (${s.geo.filter((L) => L.touchable).length} touchable)` : ''),
      n(s.images.length, 'picture', 'pictures'), n(s.sprites.length, 'sprite', 'sprites')];
    if (s.time) parts.push(`date ${s.time.label}`);
    if (s.caption && (s.caption.title || s.caption.text)) parts.push(`caption "${[s.caption.title, s.caption.text].filter(Boolean).join(': ').slice(0, 90)}"`);
    lines.push('  ' + parts.join(', '));
  }
  if (r.touched) lines.push(`  touched ${r.touched.what}: ${r.touched.changes.length ? r.touched.changes.join(', ') : 'nothing visible changed'}`);
  for (const w of r.warnings) lines.push('  warn: ' + w);
  for (const f of r.failures) lines.push('  FAIL: ' + f);
  return lines.join('\n');
}

(async () => {
  const args = parseArgs(process.argv.slice(2));
  WIKIPEDIA = !!args.wikipedia;
  const server = await start({ port: 0, roots: args.roots });
  const all = listPacks(server.roots);
  let packs;
  if (args.all || args.living) packs = all;
  else {
    packs = args.ids.map((id) => all.find((p) => p.id === id) || { id, missing: true });
    for (const p of packs) if (p.missing) { await server.close(); usage(`no pack "${p.id}" (no ${p.id}/prototype.json in ${server.roots.map((r) => path.relative(ROOT, r) || r).join(', ')})`); }
  }
  const browser = await chromium.launch(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {});
  const results = [], skipped = [];
  if (args.living) {
    let living;
    try {
      living = await checkLiving(browser, server, args.ids, args.device);
      console.log(summarize(living));
      for (const s of living.steps) console.log('  ' + s);
    } finally {
      await browser.close();
      await server.close();
    }
    process.exit(living && living.passed ? 0 : 1);
  }
  try {
    for (const p of packs) {
      if (p.error) { results.push({ id: p.id, device: args.device, passed: false, failures: [p.error], warnings: [], screenshots: [], seconds: 0 }); continue; }
      if (!RUNTIMES.includes(p.manifest.runtime)) {
        const why = `runtime ${JSON.stringify(p.manifest.runtime)}: not a pack of runtimes ${RUNTIMES[0]} to ${RUNTIMES[RUNTIMES.length - 1]} (docs/runtime3.md)`;
        if (args.all) { skipped.push(`${p.id} (${why})`); continue; }
        results.push({ id: p.id, device: args.device, passed: false, failures: [why], warnings: [], screenshots: [], seconds: 0 });
        continue;
      }
      results.push(await checkPack(browser, server, p, args.device));
    }
  } finally {
    await browser.close();
    await server.close();
  }
  for (const r of results) console.log(summarize(r));
  if (skipped.length) console.log('skipped: ' + skipped.join('; '));
  if (args.all) console.log(`${results.filter((r) => r.passed).length} of ${results.length} packs passed on ${args.device}`);
  process.exit(results.every((r) => r.passed) ? 0 : 1);
})().catch((e) => { console.error(e); process.exit(1); });
