#!/usr/bin/env node
// Render the currents spike: moments, a film and a contact sheet.
//
//   node render.cjs [work dir]            (work dir defaults to ./out/work, deleted afterwards)
//   QUICK=1 node render.cjs   (moments only, fast)
//   FRAMES=360 MOMENTS=20,110,190,250,320,359 node render.cjs
//
// Needs playwright (Chromium) and ffmpeg. Loads index.html?manual at iPad size (1376x1032 CSS px,
// deviceScaleFactor 2), steps the simulation one film frame at a time (window.advance), and plays a
// finger's plan with synthesised pointer events: Hasdrubal's ride round the Roman rear, the two
// African wings turning inward, and one tap. Writes out/moment-<frame>.png (CSS-pixel size),
// out/film.mp4 (30 fps) and out/contact.png (3x3).
const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');
const { execFileSync } = require('child_process');

const here = path.resolve(process.env.HERE || __dirname); // HERE: when piped in (node - < render.cjs)
const out = path.join(here, 'out');
const work = process.argv[2] || path.join(out, 'work');
const FRAMES = +(process.env.FRAMES || 360);
const QUICK = !!process.env.QUICK; // moments only, no film
const MOMENTS = (process.env.MOMENTS || '24,110,190,250,320,359').split(',').map(Number);

// the plan, in the ground's cells (256x192; up is north-east, the river on the left)
const PLAN = [
  { f0: 62, f1: 112, pts: [[62, 92], [59, 78], [59, 62], [67, 46], [78, 32], [96, 22], [122, 19], [150, 23], [170, 32], [181, 46], [178, 62]] },
  { f0: 150, f1: 170, pts: [[71, 112], [72, 97], [76, 84], [85, 74], [97, 67]] },
  { f0: 176, f1: 196, pts: [[149, 112], [148, 97], [144, 84], [135, 74], [123, 67]] },
  { tap: 268, at: [112, 50] },
];

function along(pts, t) {
  const seg = [];
  let L = 0;
  for (let i = 0; i + 1 < pts.length; i++) { const l = Math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1]); seg.push(l); L += l; }
  let d = t * L;
  for (let i = 0; i < seg.length; i++) {
    if (d <= seg[i] || i === seg.length - 1) { const u = Math.min(1, d / seg[i]); return [pts[i][0] + (pts[i + 1][0] - pts[i][0]) * u, pts[i][1] + (pts[i + 1][1] - pts[i][1]) * u]; }
    d -= seg[i];
  }
}

(async () => {
  fs.mkdirSync(work, { recursive: true });
  const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-accelerated-2d-canvas', '--disable-gpu-compositing'] });
  const page = await browser.newPage({ viewport: { width: 1376, height: 1032 }, deviceScaleFactor: 2 });
  page.on('console', m => console.log('page:', m.text()));
  page.on('pageerror', e => console.log('page error:', e.message));
  page.on('crash', () => console.log('page crashed'));
  await page.goto('file://' + path.join(here, 'index.html') + '?manual');
  await page.waitForFunction(() => typeof window.advance === 'function');
  const scr = async (x, y) => page.evaluate(([x, y]) => window.toWorldScreen(x, y), [x, y]);
  const t0 = Date.now();
  for (let f = 0; f < FRAMES; f++) {
    for (const s of PLAN) {
      if (s.tap === f) { const [x, y] = await scr(...s.at); await page.mouse.move(x, y); await page.mouse.down(); await page.mouse.up(); }
      if (s.pts && f >= s.f0 && f <= s.f1) {
        const [x, y] = await scr(...along(s.pts, (f - s.f0) / (s.f1 - s.f0)));
        if (f === s.f0) { await page.mouse.move(x, y); await page.mouse.down(); }
        else await page.mouse.move(x, y, { steps: 3 });
        if (f === s.f1) await page.mouse.up();
      }
    }
    const isMoment = MOMENTS.includes(f);
    await page.evaluate((nd) => window.advance(1, nd), QUICK && !isMoment);
    if (QUICK && !isMoment) continue;
    const name = path.join(work, `f${String(f).padStart(4, '0')}.jpg`);
    await page.screenshot({ path: name, type: 'jpeg', quality: 88 });
    if (MOMENTS.includes(f)) {
      const big = path.join(work, `m${f}.png`);
      await page.screenshot({ path: big });
      execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-i', big, '-vf', 'scale=1376:-1:flags=lanczos', path.join(out, `moment-${String(f).padStart(3, '0')}.png`)]);
      console.log('moment', f, await page.evaluate(() => window.stats()));
    }
    if (f % 30 === 0) console.log('frame', f, ((Date.now() - t0) / 1000).toFixed(0) + 's');
  }
  await browser.close();
  if (QUICK) { console.log('done (quick)'); return; }
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', '30', '-i', path.join(work, 'f%04d.jpg'),
    '-vf', 'scale=1376:-2:flags=lanczos', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '24', '-preset', 'slow',
    path.join(out, 'film.mp4')]);
  const picks = Array.from({ length: 9 }, (_, i) => Math.round(i * (FRAMES - 1) / 8));
  const inputs = picks.flatMap(p => ['-i', path.join(work, `f${String(p).padStart(4, '0')}.jpg`)]);
  const scale = picks.map((_, i) => `[${i}]scale=688:-1[s${i}]`).join(';');
  const rows = [0, 1, 2].map(r => `[s${r * 3}][s${r * 3 + 1}][s${r * 3 + 2}]hstack=3[r${r}]`).join(';');
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', ...inputs, '-filter_complex', `${scale};${rows};[r0][r1][r2]vstack=3`, path.join(out, 'contact.png')]);
  if (!process.argv[2]) fs.rmSync(work, { recursive: true, force: true });
  console.log('done', ((Date.now() - t0) / 1000).toFixed(0) + 's');
})();
