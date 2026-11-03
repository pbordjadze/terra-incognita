#!/usr/bin/env node
// A screenshot of a WebGPU page from headless Chromium on Linux that is not blank (docs/kit.md).
//
// Headless Chromium hands out a WebGPU adapter with only --enable-unsafe-webgpu, and the page draws,
// but the screenshot comes back blank: the canvas's frames never reach the headless compositor. With
// Vulkan on SwiftShader for both Dawn and ANGLE (the flags in FLAGS, each one needed: found by taking
// them away one at a time on Nov 3, 2026) the canvas is composited and the screenshot shows it. All in
// software: slow (seconds a frame at phone size), but every pixel is the page's own.
//
//     node tools/kit/webgpu_shot.cjs <dir | url> <out.png> [--size 390x844] [--dpr 3] [--wait 8]
//          [--until "<js expression>"] [--shots 2,4,8] [--canvas <selector>]
//
// <dir> is served on a local port (a wasm page can't load from file://). It waits `--wait` seconds, or
// until the expression is true in the page (then 1 s more), and writes <out.png>; with --shots it
// writes <out>-<t>.png at each time. Fails (exit 1) when the page has no WebGPU adapter, throws, or
// the canvas is blank (one colour over its whole area: the symptom this tool exists for), and prints
// the adapter, the console's errors and how long the page took. Needs the pre-installed Playwright.
'use strict';
const { chromium } = require('playwright');
const http = require('http');
const fs = require('fs');
const path = require('path');

const FLAGS = ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-vulkan=swiftshader', '--use-angle=vulkan'];
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm',
  '.json': 'application/json', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.css': 'text/css',
  '.wav': 'audio/wav', '.mp3': 'audio/mpeg', '.svg': 'image/svg+xml' };

function args() {
  const a = process.argv.slice(2), o = { size: [390, 844], dpr: 3, wait: 8, canvas: 'canvas', shots: null, until: null };
  const pos = [];
  for (let i = 0; i < a.length; i++) {
    const k = a[i];
    if (k === '--size') o.size = a[++i].split('x').map(Number);
    else if (k === '--dpr') o.dpr = Number(a[++i]);
    else if (k === '--wait') o.wait = Number(a[++i]);
    else if (k === '--until') o.until = a[++i];
    else if (k === '--shots') o.shots = a[++i].split(',').map(Number);
    else if (k === '--canvas') o.canvas = a[++i];
    else pos.push(k);
  }
  if (pos.length !== 2) { console.error('usage: webgpu_shot.cjs <dir | url> <out.png> [--size WxH] [--dpr n] [--wait s] [--until js] [--shots t,t] [--canvas sel]'); process.exit(2); }
  [o.src, o.out] = pos;
  return o;
}

function serve(dir) {
  return new Promise(resolve => {
    const server = http.createServer((req, res) => {
      const rel = decodeURIComponent(req.url.split('?')[0]).replace(/^\/+/, '') || 'index.html';
      const file = path.resolve(dir, rel);
      if (!file.startsWith(path.resolve(dir)) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) { res.writeHead(404); res.end(); return; }
      res.writeHead(200, { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream' });
      fs.createReadStream(file).pipe(res);
    });
    server.listen(0, '127.0.0.1', () => resolve(server));
  });
}

// Whether the canvas's own pixels (in the screenshot) are all one colour.
async function blank(page, png, sel) {
  return page.evaluate(async ([b64, sel]) => {
    const el = document.querySelector(sel);
    if (!el) return 'no element matches ' + sel;
    const r = el.getBoundingClientRect(), dpr = window.devicePixelRatio;
    const img = new Image(); img.src = 'data:image/png;base64,' + b64; await img.decode();
    const c = document.createElement('canvas'); c.width = img.width; c.height = img.height;
    const g = c.getContext('2d'); g.drawImage(img, 0, 0);
    const x0 = Math.max(0, Math.floor(r.left * dpr)), y0 = Math.max(0, Math.floor(r.top * dpr));
    const w = Math.min(img.width - x0, Math.floor(r.width * dpr)), h = Math.min(img.height - y0, Math.floor(r.height * dpr));
    if (w <= 0 || h <= 0) return 'the canvas is off screen';
    const d = g.getImageData(x0, y0, w, h).data;
    for (let i = 4; i < d.length; i += 4) if (d[i] !== d[0] || d[i + 1] !== d[1] || d[i + 2] !== d[2]) return null;
    return `the canvas is one colour (rgb ${d[0]},${d[1]},${d[2]})`;
  }, [png.toString('base64'), sel]);
}

(async () => {
  const o = args();
  let server = null, url = o.src;
  if (!/^https?:\/\//.test(o.src)) { server = await serve(o.src); url = `http://127.0.0.1:${server.address().port}/`; }
  const browser = await chromium.launch({ args: FLAGS });
  const page = await browser.newPage({ viewport: { width: o.size[0], height: o.size[1] }, deviceScaleFactor: o.dpr });
  const errors = [];
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  page.on('pageerror', e => errors.push(String(e)));
  const t0 = Date.now();
  await page.goto(url);
  const adapter = await page.evaluate(async () => {
    if (!navigator.gpu) return null;
    const a = await navigator.gpu.requestAdapter();
    if (!a) return null;
    const i = a.info || {};
    return [i.vendor, i.architecture, i.description].filter(Boolean).join(' ') || 'unnamed';
  });
  console.log('adapter:', adapter || 'none');
  let fail = adapter ? null : 'no WebGPU adapter';
  const times = o.shots || [o.wait];
  if (o.until && !fail) {
    try { await page.waitForFunction(o.until, null, { timeout: o.wait * 1000 }); await page.waitForTimeout(1000); }
    catch { fail = `"${o.until}" not true after ${o.wait} s`; }
  }
  for (const t of times) {
    if (fail) break;
    if (!o.until) await page.waitForTimeout(Math.max(0, t * 1000 - (Date.now() - t0)));
    const file = o.shots ? o.out.replace(/\.png$/, '') + `-${t}.png` : o.out;
    const png = await page.screenshot({ path: file });
    const b = await blank(page, png, o.canvas);
    console.log(`${file}: ${((Date.now() - t0) / 1000).toFixed(1)} s${b ? ', ' + b : ''}`);
    if (b) fail = b;
  }
  for (const e of errors) console.log('page error:', e);
  if (!fail && errors.length) fail = `${errors.length} page error(s)`;
  await browser.close();
  if (server) server.close();
  if (fail) { console.error('FAIL:', fail); process.exit(1); }
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });
