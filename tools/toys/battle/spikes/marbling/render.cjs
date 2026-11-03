#!/usr/bin/env node
// Plays the marbling sketch headless and keeps its moments, a film and a contact sheet.
// Run from anywhere: node tools/toys/battle/spikes/marbling/render.cjs [--quick]
// Needs playwright (chromium) and ffmpeg; writes out/m1..m6.png (iPad 1376x1032 @2x), out/film.mp4,
// out/contact.png (all PNGs paletted to stay small; --shrink-only redoes just that). --quick keeps only the moments (no film frames); --film-only only the film.
'use strict';
const http = require('http'), fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const { chromium } = require('playwright');
const DIR = __dirname, OUT = path.join(DIR, 'out'), FR = path.join(OUT, 'frames');
const quick = process.argv.includes('--quick');

const types = { '.html': 'text/html', '.js': 'text/javascript', '.png': 'image/png' };
const server = http.createServer((q, r) => {
  const f = path.join(DIR, decodeURIComponent(q.url.split('?')[0]));
  if (!f.startsWith(DIR) || !fs.existsSync(f)) { r.writeHead(404); return r.end(); }
  r.writeHead(200, { 'content-type': types[path.extname(f)] || 'application/octet-stream' }); fs.createReadStream(f).pipe(r);
});

// the plan, in sketch pixels (the ground is turned: river on the left, Romans above facing down)
const spiral = [];
for (let i = 0; i <= 300; i++) { const u = i / 300, a = Math.PI * 0.5 + u * Math.PI * 2 * 3.0, r = 320 - 300 * Math.pow(u, 0.75); spiral.push([900 + Math.cos(a) * r * 1.2, 400 + Math.sin(a) * r * 0.9]); }
const strokes = [
  { name: 'hasdrubal', pts: [[475, 740], [480, 500], [470, 290], [515, 115], [800, 58], [1100, 66], [1290, 160], [1320, 330]], speed: 24 },
  { name: 'centre', pts: [[900, 185], [900, 540]], speed: 14 },
  { name: 'africans-l', pts: [[585, 660], [690, 480], [770, 400]], speed: 16 },
  { name: 'africans-r', pts: [[1225, 660], [1130, 480], [1050, 400]], speed: 16 },
  { name: 'encircle', pts: spiral, speed: 22, hold: 1.9 },
];

function resample(pts, step) {
  const out = [pts[0]];
  let carry = 0;
  for (let i = 1; i < pts.length; i++) {
    const [ax, ay] = pts[i - 1], [bx, by] = pts[i], L = Math.hypot(bx - ax, by - ay);
    let d = step - carry;
    while (d <= L) { out.push([ax + (bx - ax) * d / L, ay + (by - ay) * d / L]); d += step; }
    carry = L - (d - step);
  }
  out.push(pts[pts.length - 1]);
  return out;
}

// keep out/ small: every PNG paletted (256 colours, dithered); the early moments at 1x, the last two at 2x
function shrink() {
  for (const f of fs.readdirSync(OUT).filter((f) => f.endsWith('.png'))) {
    const src = path.join(OUT, f), tmp = path.join(OUT, 'tmp-' + f);
    const full = /m5|m6/.test(f), scale = full ? '' : (f === 'contact.png' ? 'scale=1376:-2,' : 'scale=1376:-2,');
    execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-i', src, '-vf', `${scale}split[a][b];[a]palettegen=stats_mode=full[p];[b][p]paletteuse=dither=sierra2_4a`, tmp]);
    fs.renameSync(tmp, src);
  }
}
if (process.argv.includes('--shrink-only')) { shrink(); process.exit(0); }

(async () => {
  await new Promise((res) => server.listen(0, res));
  const port = server.address().port;
  fs.mkdirSync(FR, { recursive: true });
  for (const f of fs.readdirSync(FR)) fs.unlinkSync(path.join(FR, f));
  const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  // The same plan twice (the sketch is deterministic): at 1x every frame for the film, at 2x only the
  // moments (a 2x frame costs seconds in SwiftShader).
  const play = async (dsf, film) => {
    const page = await browser.newPage({ viewport: { width: 1376, height: 1032 }, deviceScaleFactor: dsf });
    page.on('pageerror', (e) => console.log('pageerror:', e.message));
    await page.goto(`http://localhost:${port}/index.html?manual`);
    await page.waitForFunction(() => window.ready === true);
    let n = 0;
    const tick = async (dt) => {
      await page.evaluate(([d, f]) => window.tick(d, f), [dt, film]);
      if (film) await page.screenshot({ path: path.join(FR, `f${String(n++).padStart(4, '0')}.jpg`), type: 'jpeg', quality: 90 });
    };
    const moment = async (name) => { if (film) return; await page.evaluate(() => window.render()); await page.screenshot({ path: path.join(OUT, name + '.png'), timeout: 120000 }); console.log('moment', name); };
    const DT = 1 / 24;
    for (let i = 0; i < 8; i++) await tick(DT);
    await moment('m1-tray');
    await page.mouse.click(930, 500); // the first touch: the brush is flicked
    for (let i = 0; i < 80; i++) { await tick(1 / 16); if (i === 22) await moment('m2-drops-falling'); }
    for (let i = 0; i < 4; i++) await tick(DT);
    await moment('m3-order-of-battle');
    for (const s of strokes) {
      const pts = resample(s.pts, s.speed);
      await page.mouse.move(pts[0][0], pts[0][1]); await page.mouse.down();
      for (const p of pts.slice(1)) { await page.mouse.move(p[0], p[1]); await tick(DT); }
      if (s.name === 'encircle') await moment('m5-bouquet');
      if (s.hold) { for (let t = 0; t < s.hold; t += DT) await tick(DT); }
      else { await page.mouse.up(); for (let i = 0; i < 2; i++) await tick(DT); }
      if (s.name === 'africans-r') await moment('m4-collision');
    }
    await page.mouse.up();
    for (let i = 0; i < 30; i++) await tick(DT);
    await moment('m6-print');
    console.log(film ? 'film frames' : 'moments', n, 'vertices', await page.evaluate(() => window.S.m.vertexCount()));
    await page.close();
  };
  if (!process.argv.includes('--film-only')) await play(2, false);
  if (!quick) await play(1, true);
  await browser.close(); server.close();
  if (quick) return;
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-framerate', '24', '-i', path.join(FR, 'f%04d.jpg'),
    '-vf', 'scale=1376:-2,setpts=0.6*PTS', '-r', '30', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '29', '-preset', 'slow', path.join(OUT, 'film.mp4')]);
  const frames = fs.readdirSync(FR).sort(); const pick = [];
  for (let i = 0; i < 9; i++) pick.push(path.join(FR, frames[Math.round(i * (frames.length - 1) / 8)]));
  const inputs = []; pick.forEach((f) => inputs.push('-i', f));
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', ...inputs, '-filter_complex',
    pick.map((_, i) => `[${i}]scale=688:516[s${i}]`).join(';') + ';' + pick.map((_, i) => `[s${i}]`).join('') + 'xstack=inputs=9:layout=0_0|w0_0|w0+w1_0|0_h0|w0_h0|w0+w1_h0|0_h0+h3|w0_h0+h3|w0+w1_h0+h3',
    path.join(OUT, 'contact.png')]);
  fs.rmSync(FR, { recursive: true });
  shrink();
  console.log('film and contact sheet written');
})();
