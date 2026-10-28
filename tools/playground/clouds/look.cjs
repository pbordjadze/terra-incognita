'use strict';
// Pictures of the clouds pack over the app's own globe, names on top, for judging the look:
// compose.py draws the globe and the names, headless Chromium the overlay between them.
//   node tools/playground/clouds/look.cjs [--device phone|ipad] [--month 10] [--days 0.4] [view ...]
// A view is name:lon,lat,zoom (default: a set from the whole Earth down to the ground). Pictures go
// to build/playground/clouds/look/. Reads only this machine.
const { chromium } = require('playwright');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs'), path = require('node:path');
const { start } = require('../serve.cjs');

const ROOT = path.resolve(__dirname, '../../..');
const COMPOSE = path.join(__dirname, 'compose.py');
const SIZES = { phone: [402, 874], ipad: [1032, 1376] };
const VIEWS = [
  'earth:15,8,1', 'pacific:-150,5,1', 'atlantic:-35,35,1', 'storms:60,-50,1.4',
  'namib:12,-18,2.2', 'descent:12,-20,3.4', 'ground:14,-22,5.5', 'close:15,-23,8',
];

function option(args, name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 ? args.splice(i, 2)[1] : fallback;
}

(async () => {
  const args = process.argv.slice(2);
  const device = option(args, '--device', 'phone');
  const month = option(args, '--month', String(new Date().getMonth() + 1));
  const days = option(args, '--days', '0.4');
  const scale = +option(args, '--scale', '2');
  const [w, h] = SIZES[device];
  const views = (args.length ? args : VIEWS).map((v) => {
    const [name, at] = v.split(':');
    const [lon, lat, zoom] = at.split(',').map(Number);
    return { name, lon, lat, zoom };
  });
  const out = path.join(ROOT, 'build', 'playground', 'clouds', 'look');
  fs.mkdirSync(out, { recursive: true });

  const server = await start({ port: 0 });
  const browser = await chromium.launch({ args: ['--enable-unsafe-swiftshader', '--use-angle=swiftshader', '--ignore-gpu-blocklist'] });
  try {
    const context = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: scale });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('console', (m) => { if (m.type() === 'error' || /clouds:/.test(m.text())) errors.push(m.text()); });
    await page.goto(`${server.url}/pack/clouds-web/overlay.html?mode=overlay&month=${month}&days=${days}`);
    await page.waitForSelector('html[data-clouds="ready"]', { timeout: 60000 });
    for (const v of views) {
      const cam = JSON.parse(execFileSync('python3', [COMPOSE, 'camera', `${w}x${h}`, v.lon, v.lat, v.zoom].map(String)));
      await page.evaluate((c) => { terra._receive({ type: 'camera', camera: c }); window.__clouds.draw(); }, cam);
      const overlay = path.join(out, `${device}-${v.name}-overlay.png`);
      await page.screenshot({ path: overlay, omitBackground: true });
      const file = path.join(out, `${device}-${v.name}.png`);
      execFileSync('python3', [COMPOSE, 'compose', `${w}x${h}`, v.lon, v.lat, v.zoom, scale, overlay, file].map(String));
      console.log(path.relative(ROOT, file));
    }
    if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
  } finally {
    await browser.close();
    server.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });
