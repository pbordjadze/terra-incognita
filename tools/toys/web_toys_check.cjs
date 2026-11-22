#!/usr/bin/env node
// Opens every toy of a Terra web build in Chromium's WebGPU (Dawn and Tint, stricter than naga: e.g. a
// derivative, fwidth or textureSample, in non-uniform control flow is refused there and the toy draws
// nothing) and reports each toy's shader and validation errors, page errors and a screenshot.
//
//     bash tools/toys/web_build.sh /tmp/web && python3 -m http.server -d /tmp/web 8765 &
//     xvfb-run -a node tools/toys/web_toys_check.cjs http://127.0.0.1:8765/ /tmp/web-shots [toy id ...]
//
// With no ids it reads them from toys/terra/src/doors/. Runs headed under xvfb: headless Chromium's
// SwiftShader loses the device at the globe's ground pass (Nov 22, 2026). Software rendering: slow, and a
// heavy toy may lose its device or time out here when it is fine on a real GPU; the shader errors are the
// point. Exits 1 if any toy had a shader or validation error. Needs the pre-installed Playwright.
'use strict';
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
const FLAGS = ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-vulkan=swiftshader', '--use-angle=swiftshader'];

function ids() {
  const dir = path.join(__dirname, '../../toys/terra/src/doors');
  return fs.readdirSync(dir).flatMap(f => [...fs.readFileSync(path.join(dir, f), 'utf8').matchAll(/id: "([a-z]+)"/g)].map(m => m[1]));
}

(async () => {
  const [url, dir, ...given] = process.argv.slice(2);
  if (!url || !dir) { console.error('usage: web_toys_check.cjs <url> <out-dir> [toy id ...]'); process.exit(2); }
  fs.mkdirSync(dir, { recursive: true });
  const b = await chromium.launch({ args: FLAGS, headless: false });
  let failed = 0;
  for (const id of given.length ? given : ids()) {
    const p = await b.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
    const bad = new Set();
    let shader = false; // (or a page that never started)
    p.on('pageerror', e => bad.add('pageerror: ' + String(e).split('\n')[0]));
    p.on('console', m => {
      const t = m.text();
      if (/WGSL|Invalid|validation|panicked|lost/i.test(t) && !/previous error/.test(t)) {
        bad.add(t.split('\n').slice(0, 2).join(' | ').slice(0, 240));
        if (/WGSL|validation/i.test(t)) shader = true;
      }
    });
    let state = 'opened';
    try {
      await p.goto(url + '#' + id);
      await p.waitForFunction(() => getComputedStyle(document.getElementById('loading')).opacity === '0', null, { timeout: 120000 });
      // a page that never started (no WebGPU, or the library stopped) says so in #status: a failure, not "clean"
      const status = await p.evaluate(() => { const s = document.getElementById('status'); return s.style.display === 'block' ? s.textContent : ''; });
      if (status) { bad.add('status: ' + status); shader = true; throw new Error('the page did not start'); }
      // the link raises the toy's card a second in; its button then opens it (some toys' links open them
      // straight away, so a card that never rose is only noted)
      const open = () => p.evaluate(() => { const b = [...document.querySelectorAll('#marks button')].find(b => /^Open /.test(b.getAttribute('aria-label') || '')); if (b) b.click(); return !!b; });
      let opened = false;
      for (let t = 0; t < 30 && !(opened = await open()); t++) await p.waitForTimeout(1000);
      if (!opened) state = 'card never rose';
      await p.waitForTimeout(10000);
      await p.screenshot({ path: path.join(dir, id + '.png'), timeout: 120000 });
    } catch (e) { state = 'failed: ' + e.message.split('\n')[0]; }
    if (shader) failed++;
    console.log(id, state, bad.size ? 'ERRORS:' : 'clean');
    for (const x of bad) console.log('   ', x);
    await p.close();
  }
  await b.close();
  process.exit(failed ? 1 : 0);
})();
