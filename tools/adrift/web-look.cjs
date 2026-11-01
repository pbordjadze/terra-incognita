// Opens a web build in headless Chromium with WebGPU on the software driver, taps like a phone
// (iPhone-sized, touch), and screenshots it: a look at the wasm build without a device.
//
//     python3 -m http.server -d adrift/build/web 8765 &
//     node tools/adrift/web-look.cjs http://localhost:8765/ <out-dir> '<plan json>'
//
// The plan: [{"wait": ms} | {"tap": [x, y]} | {"press": [x, y, ms]} | {"shot": "name"}], points.
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [url, out, planJson] = process.argv.slice(2);
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ args: ['--enable-unsafe-webgpu', '--enable-features=Vulkan,WebGPU', '--use-vulkan=swiftshader', '--use-angle=vulkan', '--ignore-gpu-blocklist'] });
  const page = await browser.newPage({ viewport: { width: 393, height: 852 }, deviceScaleFactor: 2, hasTouch: true, isMobile: true });
  page.on('console', m => console.log('console:', m.text()));
  page.on('pageerror', e => console.log('pageerror:', e.message));
  await page.goto(url);
  const plan = JSON.parse(planJson || '[{"wait": 8000}, {"shot": "open"}]');
  const t0 = Date.now();
  for (const s of plan) {
    if (s.wait) await page.waitForTimeout(s.wait);
    if (s.tap) await page.touchscreen.tap(s.tap[0], s.tap[1]);
    if (s.press) {
      const c = await page.$('canvas');
      await c.dispatchEvent('pointerdown', { pointerId: 9, pointerType: 'touch', isPrimary: true, clientX: s.press[0], clientY: s.press[1], bubbles: true });
      await page.waitForTimeout(s.press[2]);
      await c.dispatchEvent('pointerup', { pointerId: 9, pointerType: 'touch', isPrimary: true, clientX: s.press[0], clientY: s.press[1], bubbles: true });
    }
    if (s.shot) { await page.screenshot({ path: `${out}/${s.shot}.png` }); console.log(((Date.now() - t0) / 1000).toFixed(1) + 's', s.shot); }
  }
  console.log('status:', await page.evaluate(() => document.getElementById('status')?.textContent));
  await browser.close();
})();
