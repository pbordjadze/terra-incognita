'use strict';
// A web twin's frame as its WebGL drew it, for tools/metal/layers.py's `twin`: the uniforms its program
// was given and the pixels it wrote (premultiplied, as read back), at one camera.
//   node tools/metal/twin.cjs <job.json>
// The job: { page, ready, query, width, height, scale, camera, hold: { fn, value }, draw, out }.
// Writes <out>.json (the canvas's size and every active uniform) and <out>.rgba (its pixels, top row
// first). Reads only this machine.
const { chromium } = require('playwright');
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

(async () => {
  const job = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  const browser = await chromium.launch({ args: ['--allow-file-access-from-files', '--enable-unsafe-swiftshader', '--use-angle=swiftshader', '--ignore-gpu-blocklist'] });
  try {
    const context = await browser.newContext({ viewport: { width: job.width, height: job.height }, deviceScaleFactor: job.scale });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto(`${pathToFileURL(path.resolve(job.page))}?${job.query}`);
    await page.waitForSelector(job.ready, { timeout: 120000 });
    const frame = await page.evaluate((j) => {
      terra._receive({ type: 'camera', camera: j.camera });
      if (j.hold) window[j.hold.fn].hold(j.hold.value);
      window[j.draw].draw();
      // the same context the twin drew with: its program and its pixels, before they are composited
      const canvas = document.querySelector('canvas');
      const gl = canvas.getContext('webgl2');
      const program = gl.getParameter(gl.CURRENT_PROGRAM);
      const uniforms = {};
      for (let i = 0, n = gl.getProgramParameter(program, gl.ACTIVE_UNIFORMS); i < n; i++) {
        const info = gl.getActiveUniform(program, i);
        const base = info.name.replace(/\[0\]$/, '');
        const one = (name) => {
          const v = gl.getUniform(program, gl.getUniformLocation(program, name));
          return v && v.length !== undefined ? Array.from(v) : v;
        };
        uniforms[base] = info.size > 1 ? Array.from({ length: info.size }, (_, k) => one(`${base}[${k}]`)) : one(info.name);
      }
      const w = canvas.width, h = canvas.height, px = new Uint8Array(w * h * 4);
      gl.readPixels(0, 0, w, h, gl.RGBA, gl.UNSIGNED_BYTE, px);
      // (GL's rows run up: turned to run down, as the drawable's do)
      const rows = new Uint8Array(w * h * 4);
      for (let y = 0; y < h; y++) rows.set(px.subarray((h - 1 - y) * w * 4, (h - y) * w * 4), y * w * 4);
      let s = '';
      for (let i = 0; i < rows.length; i += 0x8000) s += String.fromCharCode.apply(null, rows.subarray(i, i + 0x8000));
      return { width: w, height: h, uniforms, pixels: btoa(s) };
    }, job);
    fs.writeFileSync(`${job.out}.rgba`, Buffer.from(frame.pixels, 'base64'));
    delete frame.pixels;
    fs.writeFileSync(`${job.out}.json`, JSON.stringify(frame));
    if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
  } finally {
    await browser.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });
