'use strict';
// A short film of the clouds pack over the app's own globe (as look.cjs, frame by frame), for
// judging its motion without a phone. Two films:
//   descent  the whole Earth at the app's pace (a day a minute), then down through the clouds to
//            the Namib coast, and a moment under them
//   week     a week of weather at the whole-Earth scale in twenty seconds (21x the app's pace)
//   node tools/playground/clouds/film.cjs <descent|week> [--device phone|ipad] [--month 10] [--fps n] [--scale n]
// Frames and the film go to build/playground/clouds/film/. Needs ffmpeg. Reads only this machine.
const { chromium } = require('playwright');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs'), path = require('node:path');
const { start } = require('../serve.cjs');

const ROOT = path.resolve(__dirname, '../../..');
const COMPOSE = path.join(__dirname, 'compose.py');
const SIZES = { phone: [402, 874], ipad: [1032, 1376] };
const ease = (t) => t * t * (3 - 2 * t);
const lerp = (a, b, t) => a + (b - a) * t;

// keyframes: seconds, then the camera and the simulated day; between them, eased
const FILMS = {
  descent: [   // at 20 frames a second, 1.5x
    { t: 0, lon: 18, lat: 6, zoom: 1, days: 0.40 },
    { t: 6, lon: 15, lat: 2, zoom: 1, days: 0.40 + 6 / 60 },
    { t: 15, lon: 13, lat: -21, zoom: 8, days: 0.40 + 15 / 60 },
    { t: 19, lon: 13, lat: -21, zoom: 8, days: 0.40 + 19 / 60 },
  ],
  week: [     // at 12 frames a second, 1x: the whole Earth needs fewer pixels, and it is long
    { t: 0, lon: 20, lat: -5, zoom: 1, days: 0 },
    { t: 20, lon: 20, lat: -5, zoom: 1, days: 7, linear: true },
  ],
};

function at(keys, t) {
  let i = 0;
  while (i < keys.length - 2 && t > keys[i + 1].t) i++;
  const a = keys[i], b = keys[i + 1];
  const u = Math.min(Math.max((t - a.t) / (b.t - a.t), 0), 1);
  const e = b.linear ? u : ease(u);
  // zoom eases in its logarithm, so the descent's pace feels even
  return {
    lon: lerp(a.lon, b.lon, e), lat: lerp(a.lat, b.lat, e),
    zoom: Math.exp(lerp(Math.log(a.zoom), Math.log(b.zoom), e)),
    days: lerp(a.days, b.days, u),
  };
}

function option(args, name, fallback) {
  const i = args.indexOf(name);
  return i >= 0 ? args.splice(i, 2)[1] : fallback;
}

(async () => {
  const args = process.argv.slice(2);
  const device = option(args, '--device', 'phone');
  const month = option(args, '--month', String(new Date().getMonth() + 1));
  const fpsArg = option(args, '--fps', null), scaleArg = option(args, '--scale', null);
  const name = args[0] || 'descent';
  const fps = +(fpsArg || (name === 'week' ? 12 : 20));
  const scale = +(scaleArg || (name === 'week' ? 1 : 1.5));
  const keys = FILMS[name];
  if (!keys) throw new Error(`no film called ${name} (${Object.keys(FILMS).join(', ')})`);
  const [w, h] = SIZES[device];
  const dir = path.join(ROOT, 'build', 'playground', 'clouds', 'film', `${device}-${name}`);
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });

  const server = await start({ port: 0 });
  const browser = await chromium.launch({ args: ['--enable-unsafe-swiftshader', '--use-angle=swiftshader', '--ignore-gpu-blocklist'] });
  const jobs = [];
  try {
    const context = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: scale });
    const page = await context.newPage();
    await page.goto(`${server.url}/pack/clouds-web/overlay.html?mode=overlay&month=${month}&days=0`);
    await page.waitForSelector('html[data-clouds="ready"]', { timeout: 60000 });
    const n = Math.round(keys[keys.length - 1].t * fps);
    for (let f = 0; f <= n; f++) {
      const c = at(keys, f / fps);
      const cam = JSON.parse(execFileSync('python3', [COMPOSE, 'camera', `${w}x${h}`, c.lon, c.lat, c.zoom].map(String)));
      await page.evaluate(([cam, days]) => {
        window.__clouds.hold(days);
        terra._receive({ type: 'camera', camera: cam });
        window.__clouds.draw();
      }, [cam, c.days]);
      const overlay = path.join(dir, `o-${String(f).padStart(4, '0')}.png`);
      await page.screenshot({ path: overlay, omitBackground: true });
      jobs.push({ w, h, lon: c.lon, lat: c.lat, zoom: c.zoom, scale, overlay, out: path.join(dir, `c-${String(f).padStart(4, '0')}.png`) });
      if (f % 50 === 0) console.error(`overlay ${f}/${n}`);
    }
  } finally {
    await browser.close();
    server.close();
  }
  const listing = path.join(dir, 'frames.json');
  fs.writeFileSync(listing, JSON.stringify(jobs));
  console.error('composing over the globe');
  execFileSync('python3', [COMPOSE, 'frames', listing], { stdio: 'inherit' });
  const film = path.join(ROOT, 'build', 'playground', 'clouds', 'film', `${device}-${name}.mp4`);
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(fps), '-i', path.join(dir, 'c-%04d.png'),
    '-vf', 'scale=trunc(iw/2)*2:trunc(ih/2)*2', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '20', film]);
  console.log(path.relative(ROOT, film));
})().catch((e) => { console.error(e); process.exit(1); });
