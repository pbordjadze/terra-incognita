'use strict';
// Pictures of the ocean (its web twin, tools/ocean/web/) over the app's own globe in the ground style,
// names on top, for judging the look: tools/clouds/compose.py draws the globe and the names, headless
// Chromium the ocean between them.
//   node tools/ocean/look.cjs [--device phone|ipad] [--month 10] [--time 12.5] [--frames 3] [--still]
//                             [--touch lon,lat,age] [view ...]
// A view is name:lon,lat,zoom (default: a set from the whole Earth down to a coast). --frames n draws
// each view at n moments 0.4 s apart (to see the motion); --touch rings the water there, that many
// seconds after the touch. Pictures go to build/ocean/look/. Reads only this machine.
const { chromium } = require('playwright');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs'), path = require('node:path');
const { pathToFileURL } = require('node:url');

const ROOT = path.resolve(__dirname, '../..');
const COMPOSE = path.join(ROOT, 'tools', 'clouds', 'compose.py');
const SIZES = { phone: [402, 874], ipad: [1032, 1376] };
const VIEWS = [
  'earth:-40,25,1', 'pacific:-160,10,1', 'gulf:-68,36,2.6', 'agulhas:24,-38,2.6',
  'stream:-72,35,5', 'shore:-9.3,38.8,8',
];

function option(args, name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 ? args.splice(i, 2)[1] : fallback;
}

(async () => {
  const args = process.argv.slice(2);
  const device = option(args, '--device', 'phone');
  const month = option(args, '--month', '10');
  const time = +option(args, '--time', '12');
  const frames = +option(args, '--frames', '1');
  const scale = +option(args, '--scale', '2');
  const touch = option(args, '--touch', null);
  const still = args.includes('--still') ? (args.splice(args.indexOf('--still'), 1), true) : false;
  const [w, h] = SIZES[device];
  const views = (args.length ? args : VIEWS).map((v) => {
    const [name, at] = v.split(':');
    const [lon, lat, zoom] = at.split(',').map(Number);
    return { name, lon, lat, zoom };
  });
  const out = path.join(ROOT, 'build', 'ocean', 'look');
  fs.mkdirSync(out, { recursive: true });
  const env = { ...process.env, CLOUDS_GROUND: String(month) };

  const browser = await chromium.launch({ args: ['--allow-file-access-from-files', '--enable-unsafe-swiftshader', '--use-angle=swiftshader', '--ignore-gpu-blocklist'] });
  try {
    const context = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: scale });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto(`${pathToFileURL(path.join(__dirname, 'web', 'preview.html'))}?month=${month}${still ? '&still=1' : ''}`);
    await page.waitForSelector('html[data-ocean="ready"]', { timeout: 120000 });
    for (const v of views) {
      const cam = JSON.parse(execFileSync('python3', [COMPOSE, 'camera', `${w}x${h}`, v.lon, v.lat, v.zoom].map(String)));
      for (let f = 0; f < frames; f++) {
        const s = time + (+process.env.OCEAN_STEP || 0.4) * f;
        await page.evaluate(([c, s, touch]) => {
          window.__ocean.hold(s);
          if (touch) { const [lon, lat, age] = touch.split(',').map(Number); window.__ocean.touch(lon, lat, s - age, 0); }
          terra._receive({ type: 'camera', camera: c });
          window.__ocean.draw();
        }, [cam, s, touch]);
        const tag = frames > 1 ? `-${f}` : '';
        const overlay = path.join(out, `${device}-${v.name}${tag}-overlay.png`);
        await page.screenshot({ path: overlay, omitBackground: true });
        const file = path.join(out, `${device}-${v.name}${tag}.png`);
        execFileSync('python3', [COMPOSE, 'compose', `${w}x${h}`, v.lon, v.lat, v.zoom, scale, overlay, file].map(String), { env });
        console.log(path.relative(ROOT, file));
      }
    }
    if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
  } finally {
    await browser.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });
