// Plays the ink-vellum spike headless and photographs it.
//
//   node tools/toys/battle/spikes/ink-vellum/render.cjs [out dir] [--quick]
//
// iPad landscape (1376x1032 CSS px, deviceScaleFactor 2) in headless Chromium on SwiftShader. The
// clock is ours (?manual=1): every film frame dispatches the finger's pointer events for that moment,
// steps the simulation STEPS times and renders. It writes the moments (PNG), film.mp4 (~10 s, 24 fps,
// iPad CSS size) and contact.png (3x3) into the out dir (default: out/ beside this file).
// The committed out/ moments are these downscaled to CSS size and quantised (to stay under ~8 MB).
// --quick writes only the moments (no film); --until <s> stops the plan early (then settles). Needs playwright and ffmpeg.
const { chromium } = require('playwright');
const path = require('path'), fs = require('fs'), { execFileSync } = require('child_process');

const HERE = __dirname;
const OUT = path.resolve(process.argv.slice(2).find((a, i, all) => !a.startsWith('--') && all[i - 1] !== '--until') || path.join(HERE, 'out'));
const QUICK = process.argv.includes('--quick');
const UNTIL = process.argv.includes('--until') ? +process.argv[process.argv.indexOf('--until') + 1] : 1e9; // stop early (seconds)
const W = 1376, H = 1032, FPS = 24, SECONDS = 10, STEPS = 4;

// The plan, in page fractions (x right, y down). Times in seconds.
const plan = [
  // Hasdrubal's horse: from the Carthaginian left, up the Aufidus past the Roman horse,
  // round the Roman rear, down onto the allied horse
  { t0: 0.6, t1: 2.8, pts: [[0.378, 0.62], [0.36, 0.52], [0.352, 0.40], [0.36, 0.26], [0.42, 0.17], [0.55, 0.14], [0.68, 0.16], [0.75, 0.22], [0.735, 0.30]] },
  // the Roman centre presses into the crescent
  { t0: 3.1, t1: 4.0, pts: [[0.55, 0.33], [0.553, 0.42], [0.555, 0.50]] },
  // the Africans turn inward on both flanks
  { t0: 4.3, t1: 5.2, pts: [[0.40, 0.585], [0.415, 0.50], [0.44, 0.42], [0.47, 0.36]] },
  { t0: 5.4, t1: 6.3, pts: [[0.70, 0.585], [0.69, 0.50], [0.665, 0.42], [0.635, 0.36]] },
  // drops of water where they meet
  { t0: 7.0, tap: [0.555, 0.45] },
  { t0: 7.6, tap: [0.46, 0.40] },
  { t0: 8.2, tap: [0.645, 0.40] },
];
const moments = { 0.3: '1-vellum-pools', 2.2: '2-hasdrubal-ride', 4.9: '3-centre-and-wings', 8.6: '4-blooms' };

function along(pts, f) { // point at fraction f of a polyline's length
  const L = []; let tot = 0;
  for (let i = 1; i < pts.length; i++) { const l = Math.hypot((pts[i][0] - pts[i - 1][0]) * 1.333, pts[i][1] - pts[i - 1][1]); L.push(l); tot += l; }
  let d = f * tot;
  for (let i = 0; i < L.length; i++) { if (d <= L[i]) { const k = d / L[i]; return [pts[i][0] + (pts[i + 1][0] - pts[i][0]) * k, pts[i][1] + (pts[i + 1][1] - pts[i][1]) * k]; } d -= L[i]; }
  return pts[pts.length - 1];
}
function ease(t) { return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2; }

(async () => {
  fs.mkdirSync(OUT, { recursive: true });
  const frames = path.join(OUT, 'frames'); fs.rmSync(frames, { recursive: true, force: true }); fs.mkdirSync(frames);
  const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: 2 });
  page.on('console', m => console.log('page:', m.text()));
  page.on('pageerror', e => console.log('page error:', e.message));
  await page.goto('file://' + path.join(HERE, 'index.html') + '?manual=1');
  await page.waitForFunction(() => window.ink && window.ink.ready, null, { timeout: 120000 });
  const m = page.mouse; const px = p => [p[0] * W, p[1] * H];
  // the plan goes through the page's own placing of the battle (ink.place)
  for (const s of plan) {
    if (s.tap) s.tap = await page.evaluate(p => window.ink.place(...p), s.tap);
    else s.pts = await page.evaluate(ps => ps.map(p => window.ink.place(...p)), s.pts);
  }
  const state = plan.map(() => ({ down: false, done: false }));
  const total = FPS * SECONDS; const t0 = Date.now();
  const mkeys = Object.keys(moments).map(Number);
  await page.evaluate(() => window.ink.setScale(1));   // film frames at CSS size; moments at the device's
  const hires = async file => {
    await page.evaluate(() => { window.ink.setScale(2); window.ink.render(); });
    await page.screenshot({ path: file, timeout: 300000 });
    await page.evaluate(() => { window.ink.setScale(1); window.ink.render(); });
  };
  for (let f = 0; f < total; f++) {
    const t = f / FPS; if (t > UNTIL) break;
    for (let i = 0; i < plan.length; i++) {
      const s = plan[i], st = state[i]; if (st.done || t < s.t0) continue;
      if (s.tap) { await m.move(...px(s.tap)); await m.down(); await m.up(); st.done = true; continue; }
      const k = Math.min(1, (t - s.t0) / (s.t1 - s.t0));
      // a few sub-moves a frame, so the line is smooth
      for (let j = 1; j <= 4; j++) {
        const kk = Math.min(1, (t - 1 / FPS + j / FPS / 4 - s.t0) / (s.t1 - s.t0));
        if (kk < 0) continue;
        await m.move(...px(along(s.pts, ease(kk))));
        if (!st.down) { await m.down(); st.down = true; }
      }
      if (k >= 1) { await m.up(); st.done = true; }
    }
    await page.evaluate(n => { window.ink.step(n); window.ink.render(); }, STEPS);
    const mk = mkeys.find(x => Math.abs(x - t) < 0.5 / FPS);
    if (mk !== undefined) await hires(path.join(OUT, moments[mk] + '.png'));
    if (!QUICK) await page.screenshot({ path: path.join(frames, String(f).padStart(4, '0') + '.jpg'), type: 'jpeg', quality: 88, scale: 'css', timeout: 300000 });
    if (f % 24 === 0) console.log(`frame ${f}/${total}  ${((Date.now() - t0) / 1000).toFixed(0)} s`);
  }
  // let it settle: the page dries into what it leaves behind
  await page.evaluate(() => { window.ink.evap = 0.006; });   // a warm afternoon: it dries faster than the film
  for (let i = 0; i < 8; i++) await page.evaluate(() => { window.ink.step(80); const g = document.getElementById('c').getContext('webgl2');
    g.bindFramebuffer(g.FRAMEBUFFER, null); g.readPixels(0, 0, 1, 1, g.RGBA, g.UNSIGNED_BYTE, new Uint8Array(4)); });
  await hires(path.join(OUT, '5-dried-manuscript.png'));
  await browser.close();
  if (!QUICK) {
    execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', String(FPS), '-i', path.join(frames, '%04d.jpg'),
      '-vf', `scale=${W}:${H}`, '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '24', '-preset', 'slow', path.join(OUT, 'film.mp4')]);
    const pick = [0, 30, 60, 90, 120, 150, 180, 210, 239].map(i => path.join(frames, String(i).padStart(4, '0') + '.jpg'));
    const args = ['-y', '-loglevel', 'error']; pick.forEach(p => args.push('-i', p));
    args.push('-filter_complex', pick.map((_, i) => `[${i}]scale=640:480[s${i}]`).join(';') + ';' + pick.map((_, i) => `[s${i}]`).join('') + 'xstack=inputs=9:layout=0_0|w0_0|w0+w1_0|0_h0|w0_h0|w0+w1_h0|0_h0+h1|w0_h0+h1|w0+w1_h0+h1', path.join(OUT, 'contact.png'));
    execFileSync('ffmpeg', args);
  }
  fs.rmSync(frames, { recursive: true, force: true });
  console.log('done', OUT, ((Date.now() - t0) / 1000).toFixed(0), 's');
})();
