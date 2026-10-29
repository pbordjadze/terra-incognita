#!/usr/bin/env node
// Draws a pack's pictures from a sheet, so every pack's art is code, rebuilt and never hand-edited:
// sprite frames (an animal swimming, a sail filling, a fire), doorway pictures and pictures that lie on
// the globe (a sea, a shelf). Headless Chromium draws (the same canvas the desktop preview uses), at
// three device pixels a point by default, so a sprite drawn for size [w, h] is crisp on a phone. The one
// art drawer: every pack's art goes through it (docs/tools.md › Art).
//
//   node tools/packart/render.cjs <sheet> <pack dir> [--only name,name] [--scale 3] [--contact out.png]
//
// A sheet is one of two kinds.
//
// A canvas sheet (`.js`) is a browser script that sets `window.SHEET = { pictures: [...] }`, each picture
//   { name: 'whale', width: 46, height: 20,         points, as the pack will show it
//     frames: 8,                                    optional: whale-1.png … whale-8.png
//     scale: 3,                                     optional: device pixels a point (default --scale)
//     draw(ctx, w, h, frame, frames) { ... } }      a 2D canvas, already scaled to points
// `tools/packart/brush.js` is loaded first: small helpers every sheet may use (window.Brush).
// --only draws the pictures of those names.
//
// An SVG sheet (`.cjs`) is a Node module, for art built from parameters in Node (the herds' animals, the
// boto), that exports `sheet({ only, scale, out })` returning (or resolving to)
//   { pictures: [{ file: 'art/x-1.png',                 the PNG, relative to the pack dir
//                  width, height, scale,                points, and device pixels a point
//                  svg: '<svg width=… height=…>…</svg>', markup sized width × scale by height × scale px
//                  contact: false,                      optional: leave it off the contact sheet
//                  sheet: 'art/early.png' }],           optional: a frame of that sprite sheet, not a file
//     done(names) { ... } }                             optional: after drawing (an index the pack reads, in `out`)
// Each SVG is drawn as Chromium draws a page's SVG and saved with its transparency; a file named twice
// is drawn once. --only is passed to the sheet, which says what it means (the herds: kinds).
//
// Sprite sheets (runtime 6, docs/runtime3.md › Sprites): the pictures given one `sheet` are laid into
// that one PNG, pixel for pixel (tools/packart/png.cjs, never through a canvas), two transparent pixels
// apart, at most 4096 a side, and `done(names)` hears each one's name in the sheet,
// `names['art/x-1.png'] === 'art/early.png#0,0,375,334'` (x, y, width, height in the sheet's pixels),
// which is what the pack's looks name. A frame is the same pixels it would be as a file of its own.
//
// --contact out.png lays every picture drawn (at half its pixels) on one page, for looking at them
// together.
'use strict';
const fs = require('fs'), path = require('path');
const { chromium } = require('playwright');
const png = require('./png.cjs');

const SHEET_SIDE = 4096, SHEET_GAP = 2;

/** Lays frames ({ file, width, height }, pixels) on shelves in the narrowest sheet no taller than it is
    allowed, tallest first; returns { width, height, places: Map(file → [x, y]) }. */
function packSheet(frames, name) {
  const order = frames.slice().sort((a, b) => b.height - a.height || b.width - a.width || (a.file < b.file ? -1 : 1));
  const area = frames.reduce((s, f) => s + (f.width + SHEET_GAP) * (f.height + SHEET_GAP), 0);
  const widest = Math.max(...frames.map((f) => f.width));
  let best = null;
  for (let width = Math.max(widest, 64); width <= SHEET_SIDE; width += 64) {
    if (width * width < area / 4) continue;
    const places = new Map();
    let x = 0, y = 0, shelf = 0;
    for (const f of order) {
      if (x + f.width > width) { x = 0; y += shelf + SHEET_GAP; shelf = 0; }
      places.set(f.file, [x, y]);
      x += f.width + SHEET_GAP;
      shelf = Math.max(shelf, f.height);
    }
    const height = y + shelf;
    if (height > SHEET_SIDE) continue;
    if (!best || width * height < best.width * best.height) best = { width, height, places };
  }
  if (!best) throw new Error(`sheet ${name}: its frames don't fit in ${SHEET_SIDE} × ${SHEET_SIDE} pixels; give it two sheets`);
  return best;
}

/** Writes each sprite sheet from its frames' PNGs; returns the frames' names in their sheets. */
function writeSheets(out, sheets) {
  const names = {};
  for (const [sheet, frames] of sheets) {
    const pictures = frames.map((f) => Object.assign(png.read(f.data), { file: f.file }));
    const { width, height, places } = packSheet(pictures, sheet);
    const data = Buffer.alloc(width * height * 4);
    for (const p of pictures) {
      const [x, y] = places.get(p.file);
      for (let row = 0; row < p.height; row++) p.data.copy(data, ((y + row) * width + x) * 4, row * p.width * 4, (row + 1) * p.width * 4);
      names[p.file] = `${sheet}#${x},${y},${p.width},${p.height}`;
    }
    const target = path.join(out, sheet);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, png.write({ width, height, data }));
    console.log(`sheet ${sheet}: ${pictures.length} frames, ${width} × ${height}`);
  }
  return names;
}

function usage(message) {
  console.error((message ? message + '\n' : '') + 'usage: node tools/packart/render.cjs <sheet> <pack dir> [--only a,b] [--scale 3] [--contact out.png]');
  process.exit(2);
}

/** A canvas sheet's pictures, drawn in the sheet's own page. Returns the files written. */
async function canvasSheet(browser, sheet, out, only, scale) {
  const brush = fs.readFileSync(path.join(__dirname, 'brush.js'), 'utf8');
  const source = fs.readFileSync(sheet, 'utf8');
  const page = await browser.newPage();
  page.on('pageerror', (e) => { console.error('sheet error: ' + e.message); process.exitCode = 1; });
  await page.setContent('<!doctype html><html><body></body></html>');
  await page.addScriptTag({ content: brush });
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
    for (const f of files) {
      fs.writeFileSync(path.join(out, f.file), Buffer.from(f.data, 'base64'));
      written.push({ file: f.file });
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
  const written = [], seen = new Set(), sheets = new Map();
  for (const p of made.pictures) {
    if (seen.has(p.file)) continue;
    seen.add(p.file);
    const s = p.scale || scale, target = path.join(out, p.file);
    await page.setViewportSize({ width: Math.ceil(p.width * s), height: Math.ceil(p.height * s) });
    await page.setContent(`<html><body style="margin:0;background:transparent">${p.svg}</body></html>`);
    const data = await page.locator('svg').screenshot({ omitBackground: true });
    if (p.sheet) {
      if (!sheets.has(p.sheet)) sheets.set(p.sheet, []);
      sheets.get(p.sheet).push({ file: p.file, data });
    } else {
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, data);
    }
    written.push({ file: p.file, data, contact: p.contact });
  }
  await page.close();
  const names = writeSheets(out, sheets);
  if (made.done) await made.done(names);
  console.log(`${written.length} picture${written.length === 1 ? '' : 's'} in ${path.relative(process.cwd(), out) || '.'}/`);
  return written;
}

/** Every picture drawn, at half its pixels, on one page. */
async function contactSheet(browser, out, written, file) {
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  const cells = written.filter((w) => w.contact !== false).map((w) => {
    const data = w.data || fs.readFileSync(path.join(out, w.file));
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
  let sheet, out, only = null, scale = 3, contact = null;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--only') only = new Set(argv[++i].split(','));
    else if (a === '--scale') scale = +argv[++i];
    else if (a === '--contact') contact = argv[++i];
    else if (!sheet) sheet = a;
    else if (!out) out = a;
    else usage('unexpected ' + a);
  }
  if (!sheet || !out) usage();
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ args: ['--no-sandbox'] });
  try {
    const written = sheet.endsWith('.cjs') ? await svgSheet(browser, sheet, out, only, scale)
                                           : await canvasSheet(browser, sheet, out, only, scale);
    if (contact) await contactSheet(browser, out, written, contact);
  } finally {
    await browser.close();
  }
}

if (require.main === module) main(process.argv.slice(2)).catch((e) => { console.error(e); process.exit(1); });
module.exports = { main, packSheet };
