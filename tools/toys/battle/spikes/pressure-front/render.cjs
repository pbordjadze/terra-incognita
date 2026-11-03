// render.cjs: plays the pressure-front sketch headless with a scripted finger and films it.
//
//   node render.cjs [good|bad|both] [--quick]       (from this folder; needs playwright, ffmpeg)
//
// Loads index.html?manual at iPad size (1376x1032 CSS px, deviceScaleFactor 2), steps the world
// frame by frame, draws the plan's strokes and holds with synthesised pointer events (one finger),
// and writes out/<plan>-<n>.png moments, out/<plan>.mp4 (10 s, 30 fps) and out/<plan>-contact.png
// (a 3x3 sheet). --quick writes only the moments and prints the armies' state as it goes.
// Then `python3 shrink.py` brings the PNGs down to 1376 px and 256 colours.
// Plans are in grid cells (160x120 over the ground; 8.6 CSS px a cell on the iPad).
const { chromium } = require('playwright');
const fs = require('fs'), path = require('path'), { execSync } = require('child_process');

const CELL = 8.6, FRAMES = 300;
// Carthage's plan, as Hannibal laid it: the horse by the river rides round behind Rome, the
// African horns fold in, the crescent's centre is held (a held finger) while it gives ground.
const PLANS = {
  good: [
    { at: 4, dur: 34, pts: [[31, 63], [29, 52], [30, 38], [33, 25], [38, 13], [55, 9], [80, 9], [100, 12], [110, 18]] },
    { at: 44, dur: 18, pts: [[43, 64], [38, 54], [36, 44], [41, 35]] },
    { at: 66, dur: 18, pts: [[103, 64], [109, 54], [111, 44], [105, 35]] },
    { at: 110, hold: 60, pts: [[73, 58]], find: 1 }, // pressed on the crescent's centre, wherever it now stands
  ],
  // a different plan: everything straight at Rome, the thin centre too
  bad: [
    { at: 4, dur: 14, pts: [[73, 52], [73, 42], [73, 32]] },
    { at: 50, dur: 12, pts: [[43, 64], [43, 52], [43, 40]] },
    { at: 66, dur: 12, pts: [[103, 64], [103, 52], [103, 40]] },
    { at: 82, dur: 12, pts: [[31, 63], [31, 50], [32, 36]] },
  ],
};
for (const k in PLANS) for (const s of PLANS[k]) s.pts = s.pts.map(([x, y]) => [x, y + 14]); // the field sits 14 cells down
const MOMENTS = [70, 170, 299];

function along(pts, u) { // point at fraction u of a polyline's length
  const L = []; let tot = 0;
  for (let i = 0; i + 1 < pts.length; i++) { const l = Math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1]); L.push(l); tot += l; }
  let d = u * tot;
  for (let i = 0; i < L.length; i++) { if (d <= L[i] || i === L.length - 1) { const t = L[i] ? Math.min(1, d / L[i]) : 0; return [pts[i][0] + (pts[i + 1][0] - pts[i][0]) * t, pts[i][1] + (pts[i + 1][1] - pts[i][1]) * t]; } d -= L[i]; }
  return pts[pts.length - 1];
}

// the canvas is the whole screen (no chrome), and reading it is ~20x faster than a page screenshot
async function grab(page, file, type) {
  const url = await page.evaluate(t => document.getElementById('c').toDataURL(t, 0.92), type);
  fs.writeFileSync(file, Buffer.from(url.split(',')[1], 'base64'));
}

async function play(browser, name, quick) {
  const out = path.join(__dirname, 'out'), tmp = path.join(out, '_frames_' + name);
  fs.mkdirSync(out, { recursive: true });
  if (!quick) { fs.rmSync(tmp, { recursive: true, force: true }); fs.mkdirSync(tmp); }
  const page = await browser.newPage({ viewport: { width: 1376, height: 1032 }, deviceScaleFactor: 2 });
  await page.goto('file://' + path.join(__dirname, 'index.html') + '?manual');
  const plan = PLANS[name];
  let moment = 1;
  for (let f = 0; f < FRAMES; f++) {
    for (const s of plan) {
      const px = p => [p[0] * CELL, p[1] * CELL];
      if (s.hold) {
        if (f === s.at) {
          let p = s.pts[0];
          if (s.find) p = await page.evaluate(([x0, y0]) => { // the densest Carthaginian cell near that column
            const K = __world.armies[1]; let best = [x0, y0], bv = 0;
            for (let y = y0 - 25; y < y0 + 25; y++) for (let x = x0 - 3; x <= x0 + 3; x++) { const v = K.rho[y * 160 + x]; if (v > bv) { bv = v; best = [x + 0.5, y + 0.5]; } }
            return best; }, p.map(Math.round));
          await page.mouse.move(...px(p)); await page.mouse.down();
        }
        if (f === s.at + 12 && quick) console.log('pins', JSON.stringify(await page.evaluate(() => __world.armies.map(A => A.pins.length))));
        if (f === s.at + s.hold) await page.mouse.up();
      } else if (f >= s.at && f <= s.at + s.dur) {
        const u = (f - s.at) / s.dur, p = px(along(s.pts, u * u * (3 - 2 * u)));
        if (f === s.at) { await page.mouse.move(...p); await page.mouse.down(); }
        else await page.mouse.move(...p, { steps: 3 });
        if (f === s.at + s.dur) await page.mouse.up();
      }
    }
    await page.evaluate(() => window.__advance(1));
    if (MOMENTS.includes(f)) {
      await grab(page, path.join(out, `${name}-${moment++}.png`), 'image/png');
      if (quick) console.log(name, f, JSON.stringify(await page.evaluate(() => window.__stats())));
    }
    if (!quick) await grab(page, path.join(tmp, String(f).padStart(4, '0') + '.jpg'), 'image/jpeg');
  }
  console.log(name, 'end', JSON.stringify(await page.evaluate(() => window.__stats())));
  await page.close();
  if (quick) return;
  execSync(`ffmpeg -y -loglevel error -framerate 30 -i ${tmp}/%04d.jpg -vf scale=1376:-2 -c:v libx264 -pix_fmt yuv420p -crf 26 -preset slow ${out}/${name}.mp4`);
  execSync(`ffmpeg -y -loglevel error -framerate 30 -i ${tmp}/%04d.jpg -vf "select='not(mod(n\\,33))',scale=458:-1,tile=3x3" -frames:v 1 ${out}/${name}-contact.png`);
  fs.rmSync(tmp, { recursive: true, force: true });
}

(async () => {
  const which = process.argv[2] && !process.argv[2].startsWith('--') ? process.argv[2] : 'both';
  const quick = process.argv.includes('--quick');
  const browser = await chromium.launch({ args: ['--disable-accelerated-2d-canvas'] }); // canvas 2D only
  for (const n of which === 'both' ? ['good', 'bad'] : [which]) await play(browser, n, quick);
  await browser.close();
})();
