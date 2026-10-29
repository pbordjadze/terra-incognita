'use strict';
// Drives A gathering of water, alive (Prototypes/gathering-of-water-alive) in the desktop preview as a reader
// would, and fails where it doesn't do what it is for. Screenshots go to build/playground/gathering-of-water-alive/check-*.png.
//   node tools/water/alive_check.cjs [--device iphone|ipad] [--reduce-motion]
// What it checks, with the real mouse (no state injected):
//   - a plain drag turns the globe and lights nothing (the hold scheme: a pan is never taken for a touch);
//   - a tap on a river lights it, and the light runs on (its way down grows) to the sea;
//   - a resting pointer, then a draw across the rivers, lights many at once, all growing together; lifting it
//     lights what gathers above them;
//   - the Amazon's lit water, at its mouth, runs on into the sea (the sea's lines and the plume's picture);
//   - close in on a lit large river, a boto surfaces (a sprite) and a tap on it names it at the foot;
//   - the foot carries no numbers; Open shows them (What is lit);
//   - a tap away from the rivers lets everything go, and ending leaves nothing behind.
const { chromium } = require('playwright');
const fs = require('node:fs'), path = require('node:path');
const { start } = require('../playground/serve.cjs');

const ROOT = path.resolve(__dirname, '../..');
const ID = 'gathering-of-water-alive';
const SIZES = { iphone: { width: 393, height: 852 }, ipad: { width: 1032, height: 1376 } };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const args = process.argv.slice(2);
const device = args.includes('--device') ? args[args.indexOf('--device') + 1] : 'iphone';
const reduce = args.includes('--reduce-motion');

(async () => {
  const server = await start({ port: 0 });
  const browser = await chromium.launch();
  const out = path.join(ROOT, 'build', 'playground', ID);
  fs.mkdirSync(out, { recursive: true });
  const failures = [], notes = [];
  const fail = (m) => { failures.push(m); console.log('FAIL ' + m); };
  const note = (m) => { notes.push(m); console.log('  ' + m); };
  const page = await (await browser.newContext({ viewport: SIZES[device], deviceScaleFactor: 2 })).newPage();
  page.on('pageerror', (e) => fail('page error: ' + e.message));
  page.on('console', (m) => { if (m.type() === 'error') fail('console error: ' + m.text()); });
  const pg = (fn, a) => page.evaluate(fn, a);
  const tag = (reduce ? 'still-' : '') + device;
  const shot = (name) => page.screenshot({ path: path.join(out, `check-${tag}-${name}.png`) });
  const state = () => pg(() => window.__playground.state());
  const features = (s, id) => { const L = s.geo.find((g) => g.id === id); return L ? L.features : 0; };
  const lit = (s) => features(s, 'trace-1') + features(s, 'trace-2') + features(s, 'trace-3');
  const vertices = (s, ids) => s.geo.filter((g) => ids.includes(g.id)).reduce((a, g) => a + g.vertices, 0);
  const TRACES = ['trace-1', 'trace-2', 'trace-3'];
  const fly = async (lon, lat, zoom) => {
    await pg((a) => window.__playground.flyTo(a[0], a[1], a[2], 0), [lon, lat, zoom]);
    await page.waitForFunction(() => window.__playground.settled(), null, { timeout: 8000 }).catch(() => {});
    await sleep(500);                                  // the script rebuilds what it draws once the camera rests
  };
  const at = (lon, lat) => pg((a) => window.__playground.project(a[0], a[1]), [lon, lat]);

  try {
    await page.goto(`${server.url}/preview/${ID}?device=${device}&bare=1&idleAfter=100000&start=1`);
    await page.waitForFunction(() => window.__playground && window.__playground.ready && window.__playground.started, null, { timeout: 20000 });
    if (reduce) await pg(() => window.__playground.setSettings({ reduceMotion: true }));
    await fly(-60, -6, 2.4);
    await shot('1-idle');
    let s = await state();
    if (s.scheme !== 'hold') fail(`the scheme is "${s.scheme}", not hold`);
    note(`idle caption: "${s.caption.title}" / "${s.caption.text}"`);

    // a plain drag: the globe turns, nothing lights
    const c0 = s.camera;
    const a = await at(-63.9, -8.7), b = await at(-56, -4);   // across the Madeira toward the Tapajós
    await page.mouse.move(a.x, a.y);
    await page.mouse.down();
    await page.mouse.move(b.x, b.y, { steps: 12 });
    await page.mouse.up();
    s = await state();
    const turned = Math.abs(s.camera.lon - c0.lon) + Math.abs(s.camera.lat - c0.lat);
    if (turned < 1) fail(`a plain drag did not turn the globe (moved ${turned.toFixed(2)} degrees)`);
    if (lit(s)) fail(`a plain drag lit ${lit(s)} ways down`);
    note(`plain drag: the globe turned ${turned.toFixed(1)} degrees, nothing lit`);

    // a tap on the Madeira near Porto Velho: lit, then grown to the sea
    await fly(-60, -6, 2.4);
    const madeira = await at(-63.9224, -8.699);
    await page.mouse.click(madeira.x, madeira.y);
    await sleep(reduce ? 300 : 700);
    s = await state();
    const early = vertices(s, TRACES);
    await shot('2-tap-running');
    await sleep(reduce ? 300 : 8500);
    s = await state();
    const late = vertices(s, TRACES);
    note(`tap on the Madeira: the way down ${early} vertices after 0.7 s, ${late} after 9 s; caption "${s.caption.title}" / "${s.caption.text}"`);
    if (!lit(s)) fail('a tap on the Madeira lit nothing');
    if (!reduce && !(late > early)) fail('the light did not run on down the river (its way down did not grow)');
    if (reduce && early !== late) fail('under Reduce Motion the light should arrive at once');
    if (/\d/.test(s.caption.title + s.caption.text)) fail('the foot carries numbers: ' + s.caption.title + ' ' + s.caption.text);
    if (!features(s, 'gathered') && !features(s, 'gathered-great')) fail('nothing gathered above the tap is lit');
    await shot('3-tap-grown');

    // the Amazon's mouth: the sea's part and the plume
    if (!features(s, 'sea')) fail('the Amazon\'s lit water did not run on into the sea');
    s = await state();
    const plume = s.images.find((im) => /^plume/.test(im.id));
    note(`at the mouth: ${features(s, 'sea')} ways into the sea, plume picture ${plume ? plume.image + ' ' + Math.round(plume.alpha * 100) + '%' : 'none'}`);
    if (!plume) fail('no plume picture at the Amazon\'s mouth');
    await fly(-46, 4, 2.6);
    await shot('4-sea');

    // a tap away from every river lets it all go
    await fly(-30, -4, 2.4);
    const away = await at(-30, -4);                     // the open Atlantic
    await page.mouse.click(away.x, away.y);
    await sleep(300);
    s = await state();
    if (lit(s)) fail('a tap away from the rivers left water lit');

    // rest, then draw across many: all lit at once, growing together
    await fly(-62, -6, 3.0);
    const p0 = await at(-69, -2.5), p1 = await at(-57, -10.5);
    await page.mouse.move(p0.x, p0.y);
    await page.mouse.down();
    await sleep(450);                                  // the rest (0.3 s) before the draw
    await page.mouse.move(p1.x, p1.y, { steps: 40 });
    s = await state();
    const crossing = lit(s);
    await sleep(reduce ? 100 : 400);
    await shot('5-draw-running');
    await page.mouse.up();
    await sleep(reduce ? 300 : 9000);
    s = await state();
    note(`draw across: ${crossing} ways down lit while drawing, ${lit(s)} drawn after; tiers ${TRACES.map((t) => features(s, t)).join('/')}; gathered ${features(s, 'gathered')}+${features(s, 'gathered-great')}; caption "${s.caption.title}"`);
    if (crossing < 4) fail(`drawing across the rivers lit only ${crossing} ways down`);
    if (!features(s, 'trace-2') && !features(s, 'trace-3')) fail('no stretch is heavier where several lit rivers run together');
    await shot('6-draw-grown');

    // close in on the lower Madeira: a large river lit there, and a boto surfaces in it
    await fly(-61.2, -5.4, 9);
    const here = await pg(() => { const c = window.__playground.camera(); return { x: c.cx, y: c.cy }; });
    const touchables = await pg(() => window.__playground.touchables());
    const river = touchables.filter((t) => /rivers-(6|7|8|9)/.test(t.what || '')).sort((u, v) => Math.hypot(u.x - here.x, u.y - here.y) - Math.hypot(v.x - here.x, v.y - here.y))[0];
    if (!river) fail('no large river to touch close in');
    else {
      await page.mouse.click(river.x, river.y);
      let boto = null;
      for (let i = 0; i < 30 && !boto; i++) {
        await sleep(300);
        s = await state();
        boto = s.sprites.find((x) => /^boto/.test(x.id) && x.drawn);
      }
      note(`close in (zoom 9): ${boto ? 'a boto surfaced, ' + boto.id + ' at ' + Math.round(boto.drawn.x) + ',' + Math.round(boto.drawn.y) : 'no boto'}`);
      if (!boto) fail('no boto surfaced close in on a lit large river');
      else {
        for (let i = 0; i < 6; i++) { await sleep(reduce ? 50 : 700); await shot('7-boto-' + i); }
        // (it swims: where it is now)
        boto = (await state()).sprites.find((x) => x.id === boto.id);
        await page.mouse.click(boto.drawn.x, boto.drawn.y);
        await sleep(200);
        s = await state();
        note(`touched the boto: caption "${s.caption.title}"`);
        if (!/boto/i.test(s.caption.title)) fail('touching the boto did not name it');
      }
    }

    // Open: the numbers, on demand
    await pg(() => window.__playground.open());
    await sleep(300);
    s = await state();
    const panel = s.panel;
    note(`Open: ${panel ? JSON.stringify(panel).slice(0, 160) : 'no panel'}`);
    await shot('8-panel');

    await pg(() => window.__playground.end());
    await sleep(300);
    s = await pg(() => window.__playground.state('gathering-of-water-alive'));
    if (s.geo.length || s.sprites.length || s.images.length) fail(`ending left ${s.geo.length} layers, ${s.sprites.length} sprites, ${s.images.length} pictures`);
  } catch (e) {
    fail('stopped: ' + (e && e.stack || e));
  }
  await browser.close();
  server.close && server.close();
  console.log(failures.length ? `FAIL (${failures.length})` : 'PASS', `screenshots in ${path.relative(ROOT, out)}/check-${tag}-*.png`);
  process.exit(failures.length ? 1 : 0);
})();
