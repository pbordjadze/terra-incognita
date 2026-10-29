#!/usr/bin/env node
// Draws a pack's pictures from a sheet of canvas drawings, so every pack's art is code, rebuilt and
// never hand-edited: sprite frames (an animal swimming, a sail filling, a fire), doorway pictures and
// pictures that lie on the globe (a sea, a shelf). Headless Chromium draws (the same canvas the
// desktop preview uses), at three device pixels a point, so a sprite drawn for size [w, h] is crisp
// on a phone.
//
//   node tools/packart/render.cjs <sheet.js> <out dir> [--only name,name] [--scale 3]
//
// A sheet is a browser script that sets `window.SHEET = { pictures: [...] }`, each picture
//   { name: 'whale', width: 46, height: 20,         points, as the pack will show it
//     frames: 8,                                    optional: whale-1.png … whale-8.png
//     scale: 3,                                     optional: device pixels a point (default 3)
//     draw(ctx, w, h, frame, frames) { ... } }      a 2D canvas, already scaled to points
// `tools/packart/brush.js` is loaded first: small helpers every sheet may use (window.Brush).
'use strict';
const fs = require('fs'), path = require('path');
const { chromium } = require('playwright');

function usage(message) {
  console.error((message ? message + '\n' : '') + 'usage: node tools/packart/render.cjs <sheet.js> <out dir> [--only a,b] [--scale 3]');
  process.exit(2);
}

async function main(argv) {
  let sheet, out, only = null, scale = 3;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--only') only = new Set(argv[++i].split(','));
    else if (a === '--scale') scale = +argv[++i];
    else if (!sheet) sheet = a;
    else if (!out) out = a;
    else usage('unexpected ' + a);
  }
  if (!sheet || !out) usage();
  fs.mkdirSync(out, { recursive: true });
  const brush = fs.readFileSync(path.join(__dirname, 'brush.js'), 'utf8');
  const source = fs.readFileSync(sheet, 'utf8');
  const browser = await chromium.launch({ args: ['--no-sandbox'] });
  try {
    const page = await browser.newPage();
    page.on('pageerror', (e) => { console.error('sheet error: ' + e.message); process.exitCode = 1; });
    await page.setContent('<!doctype html><html><body></body></html>');
    await page.addScriptTag({ content: brush });
    await page.addScriptTag({ content: source });
    const names = await page.evaluate(() => (window.SHEET.pictures || []).map((p) => p.name));
    for (const name of names) {
      if (only && !only.has(name)) continue;
      const files = await page.evaluate(async ([name, scale]) => {
        const p = window.SHEET.pictures.find((q) => q.name === name);
        const s = p.scale || scale, frames = p.frames || 0, n = Math.max(1, frames);
        const out = [];
        for (let i = 0; i < n; i++) {
          const canvas = document.createElement('canvas');
          canvas.width = Math.round(p.width * s); canvas.height = Math.round(p.height * s);
          const ctx = canvas.getContext('2d');
          ctx.scale(s, s);
          ctx.lineCap = 'round'; ctx.lineJoin = 'round';
          p.draw(ctx, p.width, p.height, i, n);
          out.push({ file: frames ? `${name}-${i + 1}.png` : `${name}.png`, data: canvas.toDataURL('image/png').split(',')[1] });
        }
        return out;
      }, [name, scale]);
      for (const f of files) fs.writeFileSync(path.join(out, f.file), Buffer.from(f.data, 'base64'));
      console.log(`${name}: ${files.length} picture${files.length === 1 ? '' : 's'}`);
    }
  } finally {
    await browser.close();
  }
}

main(process.argv.slice(2)).catch((e) => { console.error(e); process.exit(1); });
