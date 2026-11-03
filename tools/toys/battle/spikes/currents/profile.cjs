#!/usr/bin/env node
// Time the sketch's step and draw in headless Chromium: node profile.cjs
const { chromium } = require('playwright');
const path = require('path');
(async () => {
  const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--disable-accelerated-2d-canvas', '--disable-gpu-compositing'] });
  const page = await browser.newPage({ viewport: { width: 1376, height: 1032 }, deviceScaleFactor: 2 });
  page.on('pageerror', e => console.log('page error:', e.message));
  await page.goto('file://' + path.join(__dirname, 'index.html') + '?manual');
  await page.waitForFunction(() => typeof window.advance === 'function');
  console.log(await page.evaluate(() => window.profile ? window.profile(10) : 'no profile hook'));
  let t = Date.now(); await page.screenshot({ path: '/dev/null', type: 'jpeg' }); console.log('jpeg shot', Date.now() - t, 'ms');
  await browser.close();
})();
