'use strict';
// Real browser checks for Prototypes/time-atlas, through the playground's injected runtime.
// node tools/playground/time-atlas/check.cjs [--engine chromium|webkit|all]
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium, webkit } = require('playwright');
const { start } = require('../serve.cjs');

const ROOT = path.resolve(__dirname, '../../..');
const OUT = path.join(ROOT, 'build/time-atlas-report');
const sizes = [
  { name: 'phone', width: 390, height: 844 },
  { name: 'ipad-landscape', width: 1366, height: 1024 },
  { name: 'ipad-portrait', width: 1024, height: 1366 },
];
const index = process.argv.indexOf('--engine');
const engineChoice = index < 0 ? 'all' : process.argv[index + 1];
if (!['all', 'chromium', 'webkit'].includes(engineChoice)) {
  throw new Error('--engine must be chromium, webkit, or all');
}
fs.mkdirSync(OUT, { recursive: true });
const report = {
  passed: false,
  tests: [],
  failures: [],
  screenshots: [],
  requests: [],
  notes: [
    'Chromium pinch checks use CDP native touch input, not synthetic PointerEvents.',
    'WebKit pinch checks dispatch PointerEvents with a temporary pointer-capture shim; they verify handler mathematics, not physical-device multitouch delivery.',
    'Simulator/browser checks do not establish physical iPad or iPhone performance.',
  ],
};
const state = page => page.evaluate(() => window.timeAtlasState());
const settle = page => page.evaluate(() => new Promise(resolve =>
  requestAnimationFrame(() => requestAnimationFrame(resolve))));
const settings = (page, next) => page.evaluate(settings => {
  window.terra._receive({ type: 'settings', settings });
}, next);
const setYear = async (page, year) => {
  await page.locator('#year-slider').evaluate((el, value) => {
    el.value = value;
    el.dispatchEvent(new Event('input', { bubbles: true }));
  }, String(year));
  await page.waitForFunction(y => window.timeAtlasState().year === y, year);
  await settle(page);
};
const pixels = page => page.locator('#map').evaluate(canvas => {
  const { data } = canvas.getContext('2d').getImageData(0, 0, canvas.width, canvas.height);
  let hash = 2166136261 >>> 0, warm = 0, total = 0;
  // Sample actual painted pixels, including RGB channels; do not infer rendering from state.
  for (let i = 0; i < data.length; i += 16) {
    hash = Math.imul(hash ^ data[i], 16777619) >>> 0;
    hash = Math.imul(hash ^ data[i + 1], 16777619) >>> 0;
    hash = Math.imul(hash ^ data[i + 2], 16777619) >>> 0;
    if (data[i] > data[i + 1] + 8 && data[i + 1] > data[i + 2] + 8 && data[i + 3] > 200) warm++;
    total++;
  }
  return { hash, warm, total, width: canvas.width, height: canvas.height };
});
async function mouseDrag(page, locator, from, to) {
  const box = await locator.boundingBox();
  assert(box && box.width > 0 && box.height > 0, 'gesture surface has a visible box');
  const a = { x: box.x + box.width * from[0], y: box.y + box.height * from[1] };
  const b = { x: box.x + box.width * to[0], y: box.y + box.height * to[1] };
  await page.mouse.move(a.x, a.y);
  await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 8 });
  await page.mouse.up();
  await settle(page);
}
async function pinch(page, context, engine) {
  const box = await page.locator('#ruler').boundingBox();
  assert(box && box.width > 100 && box.height > 30, 'timeline is large enough for two fingers');
  const center = box.x + box.width / 2;
  const y = box.y + box.height * .45;
  const start = Math.min(48, box.width * .14), end = Math.min(125, box.width * .35);
  if (engine === 'chromium') {
    const cdp = await context.newCDPSession(page);
    const points = spread => [
      { x: center - spread, y, id: 11, radiusX: 5, radiusY: 5, force: 1 },
      { x: center + spread, y, id: 12, radiusX: 5, radiusY: 5, force: 1 },
    ];
    try {
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: points(start) });
      for (let i = 1; i <= 6; i++) {
        await cdp.send('Input.dispatchTouchEvent', {
          type: 'touchMove', touchPoints: points(start + (end - start) * i / 6),
        });
      }
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    } finally {
      await cdp.detach();
    }
  } else {
    await page.locator('#ruler').evaluate((canvas, values) => {
      const [center, y, start, end] = values;
      // Synthetic PointerEvents cannot acquire a real browser pointer capture.
      const old = {
        set: canvas.setPointerCapture, release: canvas.releasePointerCapture, has: canvas.hasPointerCapture,
      };
      canvas.setPointerCapture = () => {};
      canvas.releasePointerCapture = () => {};
      canvas.hasPointerCapture = () => false;
      const send = (type, id, x) => canvas.dispatchEvent(new PointerEvent(type, {
        pointerId: id, pointerType: 'touch', isPrimary: id === 11,
        clientX: x, clientY: y, bubbles: true, cancelable: true, buttons: type === 'pointerup' ? 0 : 1,
      }));
      try {
        send('pointerdown', 11, center - start);
        send('pointerdown', 12, center + start);
        for (let i = 1; i <= 6; i++) {
          const spread = start + (end - start) * i / 6;
          send('pointermove', 11, center - spread);
          send('pointermove', 12, center + spread);
        }
        send('pointerup', 11, center - end);
        send('pointerup', 12, center + end);
      } finally {
        canvas.setPointerCapture = old.set;
        canvas.releasePointerCapture = old.release;
        canvas.hasPointerCapture = old.has;
      }
    }, [center, y, start, end]);
  }
  await settle(page);
}

(async () => {
  let server;
  try {
    server = await start({ port: 0 });
    for (const [engine, launcher] of [['chromium', chromium], ['webkit', webkit]]) {
      if (engineChoice !== 'all' && engineChoice !== engine) continue;
      let browser;
      try {
        browser = await launcher.launch(engine === 'chromium' && process.env.CHROMIUM_PATH
          ? { executablePath: process.env.CHROMIUM_PATH } : {});
        for (const size of sizes) {
          const label = engine + '/' + size.name;
          const result = { label, passed: false, steps: [], errors: [], screenshots: [] };
          report.tests.push(result);
          const context = await browser.newContext({
            viewport: { width: size.width, height: size.height },
            deviceScaleFactor: 1,
            hasTouch: true,
          });
          const page = await context.newPage();
          let phase = 'load';
          const step = async (name, fn) => {
            phase = name;
            await fn();
            result.steps.push(name);
          };
          const shot = async name => {
            const file = engine + '-' + size.name + '-' + name + '.png';
            await page.screenshot({ path: path.join(OUT, file), fullPage: true });
            const rel = path.relative(ROOT, path.join(OUT, file));
            report.screenshots.push(rel);
            result.screenshots.push(rel);
          };
          page.on('pageerror', error => result.errors.push('pageerror: ' + error.message));
          page.on('console', message => {
            if (message.type() === 'error') result.errors.push('console: ' + message.text());
          });
          page.on('response', response => {
            if (response.status() >= 400) result.errors.push(response.status() + ' ' + response.url());
          });
          await context.route('**/*', async route => {
            const url = new URL(route.request().url());
            report.requests.push({ test: label, url: url.href });
            if (url.origin !== server.url && !['data:', 'blob:', 'about:'].includes(url.protocol)) {
              result.errors.push('external request blocked: ' + url.href);
              await route.abort();
              return;
            }
            if (url.pathname === '/favicon.ico') {
              await route.fulfill({ status: 204 });
              return;
            }
            await route.continue();
          });
          try {
            await step('injected runtime and collapsed opening', async () => {
              await page.goto(server.url + '/pack/time-atlas/index.html?mode=screen', { waitUntil: 'networkidle' });
              await page.waitForFunction(() => window.terra && window.timeAtlasState);
              await settle(page);
              assert.equal(await page.evaluate(() => terra.pack.id), 'time-atlas');
              assert.equal((await state(page)).year, 1938);
              assert.equal(await page.locator('#date-toggle').getAttribute('aria-expanded'), 'false');
              await shot('opening');
            });
            await step('select Poland and verify painted land', async () => {
              await page.locator('[data-subject="poland"]').click();
              await settle(page);
              assert.equal((await state(page)).selection.id, 'poland');
              assert.equal(await page.locator('#date-toggle').getAttribute('aria-expanded'), 'true');
              const image = await pixels(page);
              assert(image.width > 100 && image.height > 100, 'map has real backing pixels');
              assert(image.warm > 200, 'map canvas contains warm land pixels');
              result.landPixels = image.warm;
            });
            await step('1938 and 1945 paint different boundary snapshots', async () => {
              await setYear(page, 1938);
              const a = await pixels(page);
              assert.equal((await state(page)).snapshot, 1938);
              await setYear(page, 1945);
              const b = await pixels(page);
              assert.equal((await state(page)).snapshot, 1945);
              assert.notEqual(a.hash, b.hash, 'actual canvas pixel hash changes between maps');
              result.snapshotHashes = { 1938: a.hash, 1945: b.hash };
              await shot('timeline');
            });
            await step('drag timeline changes date', async () => {
              const before = await state(page);
              await mouseDrag(page, page.locator('#ruler'), [.45, .42], [.65, .42]);
              assert(Math.abs((await state(page)).year - before.year) > .1, 'ruler drag moves time');
            });
            await step('map pan preserves date', async () => {
              const before = await state(page);
              await mouseDrag(page, page.locator('#map'), [.40, .48], [.60, .50]);
              const after = await state(page);
              assert.equal(after.year, before.year);
              assert(Math.abs(after.camera.lon - before.camera.lon) > .01, 'camera pans');
            });
            await step('timeline zoom preserves map camera', async () => {
              const before = await state(page);
              await page.locator('#timeline-tools summary').click();
              await page.locator('#zoom-in').click();
              const after = await state(page);
              assert(after.span < before.span, 'timeline scale becomes narrower');
              assert.deepEqual(after.camera, before.camera, 'map camera is independent of time scale');
              await page.locator('#timeline-tools summary').click();
            });
            await step(engine === 'chromium' ? 'actual CDP two-finger pinch' : 'WebKit pointer pinch handler fallback', async () => {
              const before = await state(page);
              await pinch(page, context, engine);
              const after = await state(page);
              assert(after.span < before.span * .9, 'spreading two fingers narrows the time span');
              assert.deepEqual(after.camera, before.camera, 'pinching time does not zoom the map');
            });
            await step('place stays fixed across historical dates', async () => {
              await page.locator('[data-subject="place"]').click();
              const before = await state(page);
              assert.equal(before.selection.kind, 'place');
              await setYear(page, 1530);
              await setYear(page, 1994);
              const after = await state(page);
              assert.deepEqual(after.selection.point, before.selection.point);
              assert.deepEqual(after.camera, before.camera);
              await setYear(page, 1938);
            });
            await step('inactive runtime stops playback', async () => {
              await settings(page, { active: true, reduceMotion: false, textScale: 1 });
              const before = await state(page);
              await page.locator('#play-toggle').click();
              await page.waitForFunction(y => window.timeAtlasState().playing && window.timeAtlasState().year > y, before.year);
              await settings(page, { active: false });
              const paused = await state(page);
              assert.equal(paused.active, false);
              assert.equal(paused.playing, false);
              await page.waitForTimeout(180);
              assert.equal((await state(page)).year, paused.year, 'inactive clock stays fixed');
            });
            await step('Reduce Motion prevents autoplay', async () => {
              await settings(page, { active: true, reduceMotion: true });
              const before = await state(page);
              await page.locator('#play-toggle').click();
              await page.waitForTimeout(140);
              const after = await state(page);
              assert.equal(after.playing, false);
              assert.equal(after.year, before.year);
            });
            await step('large text has no horizontal overflow and controls are reachable', async () => {
              await settings(page, { active: true, reduceMotion: false, textScale: 3.2 });
              await settle(page);
              const overflow = await page.evaluate(() => ({
                viewport: innerWidth,
                html: document.documentElement.scrollWidth,
                body: document.body.scrollWidth,
                scale: getComputedStyle(document.documentElement).getPropertyValue('--text-scale'),
              }));
              assert(Number(overflow.scale) >= 3.19, 'runtime text scale reaches the prototype');
              assert(overflow.html <= overflow.viewport + 1, JSON.stringify(overflow));
              assert(overflow.body <= overflow.viewport + 1, JSON.stringify(overflow));
              for (const id of ['date-toggle', 'year-slider', 'sources-open', 'map-zoom-in', 'map-zoom-out', 'clear-selection']) {
                const control = page.locator('#' + id);
                await control.scrollIntoViewIfNeeded();
                await control.click({ trial: true, timeout: 3000 });
              }
              const summary = page.locator('#timeline-tools summary');
              await summary.scrollIntoViewIfNeeded();
              if (!await page.locator('#timeline-tools').evaluate(el => el.open)) await summary.click();
              for (const id of ['zoom-in', 'zoom-out', 'previous-snapshot', 'next-snapshot']) {
                const control = page.locator('#' + id);
                await control.scrollIntoViewIfNeeded();
                await control.click({ trial: true, timeout: 3000 });
              }
              await summary.click();
              await shot('large-text');
              await settings(page, { textScale: 1 });
              await settle(page);
            });
            await step('runtime close disposes listeners and clock', async () => {
              await page.evaluate(() => terra._receive({ type: 'close' }));
              const before = await state(page);
              assert.equal(before.disposed, true);
              assert.equal(before.playing, false);
              await page.evaluate(() => {
                const slider = document.getElementById('year-slider');
                slider.value = 1530;
                slider.dispatchEvent(new Event('input', { bubbles: true }));
                document.querySelector('[data-subject="poland"]').click();
                document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
                const map = document.getElementById('map');
                map.dispatchEvent(new PointerEvent('pointerdown', { pointerId: 5, clientX: 100, clientY: 200, bubbles: true }));
                map.dispatchEvent(new PointerEvent('pointermove', { pointerId: 5, clientX: 170, clientY: 220, bubbles: true }));
                map.dispatchEvent(new PointerEvent('pointerup', { pointerId: 5, clientX: 170, clientY: 220, bubbles: true }));
              });
              await page.waitForTimeout(120);
              const after = await state(page);
              assert.equal(after.year, before.year, 'disposed slider and keyboard have no handlers');
              assert.equal(after.span, before.span);
              assert.deepEqual(after.selection, before.selection, 'disposed subject buttons have no handlers');
              assert.deepEqual(after.camera, before.camera, 'disposed map has no handlers');
              assert.equal(after.disposed, true);
            });
            assert.deepEqual(result.errors, [], 'no browser errors or external requests');
            result.passed = true;
          } catch (error) {
            result.errors.push(phase + ': ' + error.message);
            try { result.state = await state(page); } catch (_) {}
            try { await shot('failure'); } catch (_) {}
          } finally {
            await context.close();
          }
          if (!result.passed) report.failures.push({ test: label, errors: result.errors });
        }
      } catch (error) {
        report.failures.push({ test: engine, errors: [error.stack || String(error)] });
      } finally {
        if (browser) await browser.close();
      }
    }
  } catch (error) {
    report.failures.push({ test: 'setup', errors: [error.stack || String(error)] });
  } finally {
    if (server) await server.close();
    report.passed = report.failures.length === 0 && report.tests.length > 0 && report.tests.every(t => t.passed);
    fs.writeFileSync(path.join(OUT, 'results.json'), JSON.stringify(report, null, 2) + '\n');
    console.log(JSON.stringify(report, null, 2));
    process.exitCode = report.passed ? 0 : 1;
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
