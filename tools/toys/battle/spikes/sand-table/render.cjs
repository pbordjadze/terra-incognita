// Sand table: plays the scripted plan headless and photographs it.
// Run:  node render.cjs [outDir=out] [--frames 300] [--dpr 2] [--film-dpr 1] [--only-moments]
// Needs playwright (require('playwright')) and ffmpeg. Loads index.html at iPad size (1376x1032 CSS
// px), steps the simulation by hand (window.__manual), synthesises pointer events for the finger's
// strokes and a tap, saves moment-*.jpg (and a 1:1 PNG detail) at --dpr, the film's frames at --film-dpr, then film.mp4
// (30 fps) and contact.png (3x3) with ffmpeg.
const { chromium } = require('playwright');
const path = require('path'), fs = require('fs'), { execSync } = require('child_process');
const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(k); return i >= 0 ? args[i + 1] : d; };
const out = path.resolve(__dirname, args[0] && !args[0].startsWith('--') ? args[0] : 'out');
const FRAMES = +opt('--frames', 300), DPR = +opt('--dpr', 2), FDPR = +opt('--film-dpr', 1);
const ONLY = args.includes('--only-moments');
const SUB = 2, DT = 1 / 30;                  // two simulation steps a frame: 20 s of sand in 10 s

// The plan, in the Roman frame (right toward the river, forward down the valley), cells from C.
const strokes = [
  { at: 25, dur: 55, pts: [[52, 12], [57, -8], [60, -36], [52, -60], [26, -72], [-8, -74], [-40, -66], [-58, -50]] }, // Hasdrubal round the rear
  { at: 95, dur: 30, pts: [[48, 12], [47, 0], [41, -14], [31, -24]] },          // the Africans turn inward, river side
  { at: 132, dur: 30, pts: [[-48, 12], [-47, 0], [-41, -14], [-31, -24]] },     // and on the far side
];
const taps = [{ at: 175, pt: [-66, 10] }];    // a fingertip among the Numidians
const moments = { 18: 'still', 58: 'drawing', 190: 'pour', 299: 'closed', 245: 'clash' };

function chaikin(p) {                         // a finger's line, not a ruler's
  for (let k = 0; k < 3; k++) { const q = [p[0]]; for (let i = 0; i < p.length - 1; i++) { const a = p[i], b = p[i + 1]; q.push([a[0] * .75 + b[0] * .25, a[1] * .75 + b[1] * .25], [a[0] * .25 + b[0] * .75, a[1] * .25 + b[1] * .75]); } q.push(p[p.length - 1]); p = q; }
  return p;
}
for (const s of strokes) s.pts = chaikin(s.pts);
function interp(pts, u) {                     // along the polyline by arc length
  const L = [0]; for (let i = 1; i < pts.length; i++) L.push(L[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const s = u * L[L.length - 1]; let i = 1; while (i < L.length - 1 && L[i] < s) i++;
  const a = (s - L[i - 1]) / (L[i] - L[i - 1] || 1);
  return [pts[i - 1][0] + (pts[i][0] - pts[i - 1][0]) * a, pts[i - 1][1] + (pts[i][1] - pts[i - 1][1]) * a];
}

(async () => {
  fs.mkdirSync(out, { recursive: true });
  const fr = path.join(out, 'frames'); fs.rmSync(fr, { recursive: true, force: true }); fs.mkdirSync(fr);
  const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const ctx = await browser.newContext({ viewport: { width: 1376, height: 1032 }, deviceScaleFactor: DPR });
  const page = await ctx.newPage();
  page.on('console', m => console.log('page:', m.text())); page.on('pageerror', e => console.log('error:', e.message));
  await page.addInitScript(() => { window.__manual = true; });
  await page.goto('file://' + path.join(__dirname, 'index.html'));
  await page.waitForFunction(() => window.DRAW);
  const ev = (type, rf) => page.evaluate(([type, r, f]) => {
    const S = window.SIM; r *= S.K; f *= S.K; const x = S.C[0] + S.Rt[0] * r + S.F[0] * f, y = S.C[1] + S.Rt[1] * r + S.F[1] * f;
    const cx = x / S.W * innerWidth, cy = y / S.H * innerHeight;
    document.getElementById('c').dispatchEvent(new PointerEvent('pointer' + type, { clientX: cx, clientY: cy, pointerId: 1, pointerType: 'touch', isPrimary: true, bubbles: true }));
  }, [type, rf[0], rf[1]]);
  const t0 = Date.now();
  for (let f = 0; f < FRAMES; f++) {
    for (const s of strokes) {
      if (f === s.at) await ev('down', s.pts[0]);
      if (f > s.at && f <= s.at + s.dur) {
        // several moves a frame: a finger reports at 120 Hz
        for (let k = 3; k >= 0; k--) { const u = Math.min(1, (f - s.at - k / 4) / s.dur); if (u > 0) await ev('move', interp(s.pts, u)); }
      }
      if (f === s.at + s.dur + 1) await ev('up', s.pts[s.pts.length - 1]);
    }
    for (const t of taps) { if (f === t.at) { await ev('down', t.pt); await ev('up', t.pt); } }
    await page.evaluate(([n, dt]) => { for (let i = 0; i < n; i++) window.SIM.step(dt); }, [SUB, DT]);
    const isMoment = moments[f] !== undefined;
    if (ONLY && !isMoment) continue;
    await page.evaluate(() => window.DRAW());
    if (isMoment) {                           // full-size grain is noise to PNG (13 MB): JPEG at 1x, and one 2x PNG detail
      await page.screenshot({ path: path.join(out, `moment-${String(f).padStart(3, '0')}-${moments[f]}.jpg`), type: 'jpeg', quality: 85, scale: 'css' });
      if (f === 245) await page.screenshot({ path: path.join(out, 'detail-245-grain.png'), clip: { x: 600, y: 200, width: 320, height: 240 } });
    }
    if (!ONLY) await page.screenshot({ path: path.join(fr, `f${String(f).padStart(4, "0")}.jpg`), scale: FDPR === DPR ? 'device' : 'css', type: 'jpeg', quality: 92 });
    if (f % 30 === 0) console.log('frame', f, ((Date.now() - t0) / 1000).toFixed(1) + 's');
  }
  await browser.close();
  if (!ONLY) {
    execSync(`ffmpeg -y -loglevel error -framerate 30 -i ${fr}/f%04d.jpg -vf "scale=1100:-2" -c:v libx264 -pix_fmt yuv420p -crf 30 -preset slow ${out}/film.mp4`);
    execSync(`ffmpeg -y -loglevel error -framerate 30 -i ${fr}/f%04d.jpg -vf "select='not(mod(n\\,${Math.floor(FRAMES / 9)}))',scale=456:-2,tile=3x3:padding=4" -frames:v 1 ${out}/contact.png`);
    fs.rmSync(fr, { recursive: true, force: true });
  }
  console.log('done', ((Date.now() - t0) / 1000).toFixed(1) + 's');
})();
