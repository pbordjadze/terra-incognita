#!/usr/bin/env node
// Draws a pack's pictures from a sheet, so every pack's art is code, rebuilt and never hand-edited:
// sprite frames (an animal swimming, a sail filling, a fire), doorway pictures and pictures that lie on
// the globe (a sea, a shelf). Headless Chromium draws (the same canvas the desktop preview uses), at
// three device pixels a point by default, so a sprite drawn for size [w, h] is crisp on a phone. The one
// art drawer: every pack's art goes through it (docs/tools.md › Art).
//
//   node tools/packart/render.cjs <sheet> <pack dir> [--only name,name] [--scale 3] [--contact out.png] [--with data.js]
//
// A sheet is one of two kinds.
//
// A canvas sheet (`.js`) is a browser script that sets `window.SHEET = { pictures: [...] }`, each picture
//   { name: 'whale', width: 46, height: 20,         points, as the pack will show it
//     frames: 8,                                    optional: whale-1.png … whale-8.png
//     scale: 3,                                     optional: device pixels a point (default --scale)
//     shrink: 'posterize',                          optional: a big ground picture rewritten with its colours held
//                                                   to 64 levels a channel by tools/packart/shrink.py (a quarter of the bytes)
//     draw(ctx, w, h, frame, frames) { ... } }      a 2D canvas, already scaled to points
// `tools/packart/brush.js` is loaded first: small helpers every sheet may use (window.Brush). --with loads a
// data script before the sheet (a builder's `window.X = {...}`: the night crossing's terrain), so a sheet
// drawn from built data stays code, not a file with the data pasted in. --only draws the pictures of those names.
//
// An SVG sheet (`.cjs`) is a Node module, for art built from parameters in Node (the herds' animals, the
// boto), that exports `sheet({ only, scale, out })` returning (or resolving to)
//   { pictures: [{ file: 'art/x-1.png',                 the PNG, relative to the pack dir
//                  width, height, scale,                points, and device pixels a point
//                  svg: '<svg width=… height=…>…</svg>', markup sized width × scale by height × scale px
//                  contact: false }],                   optional: leave it off the contact sheet
//     done() { ... } }                                  optional: after drawing (an index the pack reads, in `out`)
// Each SVG is drawn as Chromium draws a page's SVG and saved with its transparency; a file named twice
// is drawn once. --only is passed to the sheet, which says what it means (the herds: kinds).
//
// --contact out.png lays every picture drawn (at half its pixels) on one page, for looking at them
// together.
'use strict';
const fs = require('fs'), path = require('path');
const { chromium } = require('playwright');

function usage(message) {
  console.error((message ? message + '\n' : '') + 'usage: node tools/packart/render.cjs <sheet> <pack dir> [--only a,b] [--scale 3] [--contact out.png] [--with data.js]');
  process.exit(2);
}

/** A canvas sheet's pictures, drawn in the sheet's own page. Returns the files written. */
async function canvasSheet(browser, sheet, out, only, scale, withs) {
  const brush = fs.readFileSync(path.join(__dirname, 'brush.js'), 'utf8');
  const source = fs.readFileSync(sheet, 'utf8');
  const page = await browser.newPage();
  page.on('pageerror', (e) => { console.error('sheet error: ' + e.message); process.exitCode = 1; });
  await page.setContent('<!doctype html><html><body></body></html>');
  await page.addScriptTag({ content: brush });
  for (const w of withs) await page.addScriptTag({ content: fs.readFileSync(w, 'utf8') });
  await page.addScriptTag({ content: source });
  const names = await page.evaluate(() => (window.SHEET.pictures || []).map((p) => p.name));
  const written = [];
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
    const shrink = await page.evaluate((name) => window.SHEET.pictures.find((q) => q.name === name).shrink || null, name);
    for (const f of files) {
      fs.writeFileSync(path.join(out, f.file), Buffer.from(f.data, 'base64'));
      written.push({ file: f.file });
    }
    if (shrink === 'posterize') {
      const r = require('child_process').spawnSync('python3', [path.join(__dirname, 'shrink.py'), ...files.map((f) => path.join(out, f.file))], { stdio: 'inherit' });
      if (r.status !== 0) { console.error(`shrink.py failed for ${name}`); process.exitCode = 1; }
    }
    console.log(`${name}: ${files.length} picture${files.length === 1 ? '' : 's'}`);
  }
  await page.close();
  return written;
}

/** An SVG sheet's pictures, each drawn as a page of its own and screenshotted. Returns the files written. */
async function svgSheet(browser, sheet, out, only, scale) {
  const made = await require(path.resolve(sheet)).sheet({ only: only && [...only], scale, out });
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  const written = [], seen = new Set();
  for (const p of made.pictures) {
    if (seen.has(p.file)) continue;
    seen.add(p.file);
    const s = p.scale || scale, target = path.join(out, p.file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    await page.setViewportSize({ width: Math.ceil(p.width * s), height: Math.ceil(p.height * s) });
    await page.setContent(`<html><body style="margin:0;background:transparent">${p.svg}</body></html>`);
    await page.locator('svg').screenshot({ path: target, omitBackground: true });
    written.push({ file: p.file, contact: p.contact });
  }
  await page.close();
  if (made.done) await made.done();
  console.log(`${written.length} picture${written.length === 1 ? '' : 's'} in ${path.relative(process.cwd(), out) || '.'}/`);
  return written;
}

/** Every picture drawn, at half its pixels, on one page. */
async function contactSheet(browser, out, written, file) {
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  const cells = written.filter((w) => w.contact !== false).map((w) => {
    const data = fs.readFileSync(path.join(out, w.file));
    const width = data.readUInt32BE(16), height = data.readUInt32BE(20);   // the PNG's header
    return `<img src="data:image/png;base64,${data.toString('base64')}" style="width:${width / 2}px;height:${height / 2}px">`;
  });
  await page.setViewportSize({ width: 2416, height: 800 });
  await page.setContent(`<html><body style="margin:0;background:#E8DFC8;display:flex;flex-wrap:wrap;align-items:flex-start;gap:6px;padding:8px;width:2400px">${cells.join('')}</body></html>`);
  await page.screenshot({ path: file, fullPage: true });
  await page.close();
  console.log(`contact sheet: ${file}`);
}

async function main(argv) {
  let sheet, out, only = null, scale = 3, contact = null; const withs = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--only') only = new Set(argv[++i].split(','));
    else if (a === '--scale') scale = +argv[++i];
    else if (a === '--contact') contact = argv[++i];
    else if (a === '--with') withs.push(argv[++i]);
    else if (!sheet) sheet = a;
    else if (!out) out = a;
    else usage('unexpected ' + a);
  }
  if (!sheet || !out) usage();
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ args: ['--no-sandbox'] });
  try {
    const written = sheet.endsWith('.cjs') ? await svgSheet(browser, sheet, out, only, scale)
                                           : await canvasSheet(browser, sheet, out, only, scale, withs);
    if (contact) await contactSheet(browser, out, written, contact);
  } finally {
    await browser.close();
  }
}

if (require.main === module) main(process.argv.slice(2)).catch((e) => { console.error(e); process.exit(1); });
module.exports = { main };
