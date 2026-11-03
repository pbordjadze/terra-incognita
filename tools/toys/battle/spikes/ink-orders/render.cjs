#!/usr/bin/env node
// Ink orders: play a plan with a synthesised finger in headless Chromium and keep what it shows.
//
//   node render.cjs good            # film of the good plan -> out/good-*.png, out/good.mp4, out/good-contact.png
//   node render.cjs bad             # the bad plan
//   node render.cjs good --probe    # no film: the census over time and six moments at 1x (PROBE_DIR=...)
//   (node probe.cjs runs the rules alone, without a browser, for tuning)
//   node render.cjs all             # both films, then out/film.mp4 (good, then bad) and out/contact.png
//   node render.cjs join            # only the last step, from out/good.mp4 and out/bad.mp4 already made
//
// Needs playwright (require('playwright')) and ffmpeg. iPad landscape: 1376x1032 CSS px at 2x.
'use strict';
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const HERE = __dirname, OUT = path.join(HERE, 'out');

const { PLANS, FRAMES, schedule } = require('./plans.cjs');   // the strokes, and the film's clock
const FPS = 30, DX = 170;   // plans are in field px; the view shows the field shifted right by DX (draw.js)

async function play(name, probe) {
  const browser = await chromium.launch({ args: ['--disable-accelerated-2d-canvas'] });  // canvas 2D only: far faster screenshots
  const page = await browser.newPage({ viewport: { width: 1376, height: 1032 }, deviceScaleFactor: probe ? 1 : 2 });
  page.on('pageerror', e => console.error('page:', e.message));
  await page.goto('file://' + path.join(HERE, 'index.html') + '?manual&film');
  const dir = path.join(OUT, '.frames-' + name); fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
  const moments = new Set([8, 70, 130, 190, 240, 299]);
  const frame = async (f, steps) => {
    await page.evaluate(n => window.__tick(n), steps);
    const png = path.join(dir, String(f).padStart(4, '0') + '.png');
    if (!probe) await page.screenshot({ path: png });
    if (moments.has(f)) { if (probe) await page.screenshot({ path: path.join(process.env.PROBE_DIR || OUT, `probe-${name}-${f}.png`) }); else fs.copyFileSync(png, path.join(OUT, `${name}-${String(f).padStart(3, '0')}.png`)); }
    if (f % 30 === 0) console.log(name, 'frame', f, JSON.stringify(await page.evaluate(() => window.__census())));
  };
  // the finger draws each stroke with the battle running gently under it; then the battle runs on
  for (const a of schedule(PLANS[name])) {
    if (a.t === 'down') { await page.mouse.move(a.x + DX, a.y); await page.mouse.down(); }
    else if (a.t === 'move') await page.mouse.move(a.x + DX, a.y);
    else if (a.t === 'up') await page.mouse.up();
    else if (a.t === 'start') await page.evaluate(() => window.SIM.start());
    else await frame(a.f, a.n);
  }
  await browser.close();
  if (probe) return;
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(FPS), '-i', path.join(dir, '%04d.png'), '-vf', 'scale=1376:1032', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '26', '-preset', 'slow', path.join(OUT, name + '.mp4')]);
  const picks = [8, 45, 80, 110, 140, 170, 210, 250, 299].map(k => path.join(dir, String(k).padStart(4, '0') + '.png'));
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', ...picks.flatMap(p => ['-i', p]), '-filter_complex', picks.map((_, k) => `[${k}]scale=688:516[s${k}]`).join(';') + ';' + picks.map((_, k) => `[s${k}]`).join('') + 'xstack=inputs=9:layout=0_0|w0_0|w0+w1_0|0_h0|w0_h0|w0+w1_h0|0_h0+h1|w0_h0+h1|w0+w1_h0+h1', path.join(OUT, name + '-contact.png')]);
  fs.rmSync(dir, { recursive: true, force: true });
}

(async () => {
  const name = process.argv[2] || 'good', probe = process.argv.includes('--probe');
  fs.mkdirSync(OUT, { recursive: true });
  const names = ['good', 'bad'];
  if (name !== 'join') for (const n of (name === 'all' ? names : [name])) await play(n, probe);
  if (name === 'all' || name === 'join') {
    const list = path.join(OUT, '.list.txt'); fs.writeFileSync(list, names.map(n => `file '${path.join(OUT, n + '.mp4')}'`).join('\n'));
    execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-f', 'concat', '-safe', '0', '-i', list, '-c', 'copy', path.join(OUT, 'film.mp4')]);
    fs.rmSync(list); fs.copyFileSync(path.join(OUT, 'good-contact.png'), path.join(OUT, 'contact.png'));
  }
})();
