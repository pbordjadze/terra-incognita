'use strict';
// Loads a prototype in headless Chromium, as the preview harness hosts it, and fails on what a phone
// would trip over: a page error, a console error, a terra.log error, a request or response that is not
// local, a page that never says ready. Screenshots go to build/playground/<id>/.
//   node tools/playground/check.cjs <id> [--device phone|ipad]
const { chromium } = require('playwright');
const fs = require('node:fs'), path = require('node:path');
const { start } = require('./serve.cjs');

const ROOT = path.resolve(__dirname, '../..');
const LOCAL = new Set(['127.0.0.1', 'localhost']);
const SIZES = { phone: { viewport: { width: 1000, height: 1000 }, w: 390, h: 844 }, ipad: { viewport: { width: 1500, height: 1500 }, w: 1024, h: 1366 } };
const ID_RE = /^[a-z0-9](?:[a-z0-9-]{0,38}[a-z0-9])$/;

function usage(message) {
  console.error((message ? message + '\n' : '') + 'usage: node tools/playground/check.cjs <id> [--device phone|ipad]');
  process.exit(2);
}

(async () => {
  const args = process.argv.slice(2);
  const di = args.indexOf('--device');
  const device = di >= 0 ? args.splice(di, 2)[1] : 'phone';
  if (!SIZES[device]) usage(`--device must be phone or ipad (got ${device})`);
  const id = args[0];
  if (!id || !ID_RE.test(id)) usage(id ? `"${id}" is not a prototype id` : 'which prototype?');
  const manifestPath = path.join(ROOT, 'Prototypes', id, 'prototype.json');
  if (!fs.existsSync(manifestPath)) usage(`Prototypes/${id}/prototype.json does not exist`);
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));

  const outDir = path.join(ROOT, 'build', 'playground', id);
  fs.mkdirSync(outDir, { recursive: true });
  for (const f of fs.readdirSync(outDir)) if (f.startsWith(device + '-') && f.endsWith('.png')) fs.rmSync(path.join(outDir, f));

  const failures = [], warnings = [], shots = [], steps = [], requests = [], nonLocal = [];
  const fail = (m) => { if (!failures.includes(m)) failures.push(m); };
  const warn = (m) => { if (!warnings.includes(m)) warnings.push(m); };
  const server = await start({ port: 0 });
  const browser = await chromium.launch(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {});
  let logs = [];

  try {
    const size = SIZES[device];
    const context = await browser.newContext({ viewport: size.viewport, deviceScaleFactor: 2 });
    // nothing leaves this machine, whatever the pack asks for
    await context.route((url) => !LOCAL.has(url.hostname) && !['data:', 'blob:', 'about:'].includes(url.protocol), (route) => {
      nonLocal.push(route.request().url());
      route.abort();
    });
    const page = await context.newPage();
    page.on('pageerror', (e) => fail('page error: ' + e.message));
    page.on('console', (m) => { if (m.type() === 'error') fail('console error: ' + m.text()); });
    page.on('request', (r) => requests.push(r.url()));
    page.on('response', (r) => {
      if (r.status() >= 400 && !r.url().endsWith('/favicon.ico')) fail(`${r.status()} for ${r.url().replace(server.url, '')}`);
    });

    const shot = async (name) => {
      const file = `${device}-${name}.png`;
      await page.locator('#device').screenshot({ path: path.join(outDir, file) });
      shots.push(path.relative(ROOT, path.join(outDir, file)));
      steps.push('screenshot ' + file);
    };
    const waitReady = async (what) => {
      try {
        await page.waitForFunction((w) => window.__playground.ready[w], what, { timeout: 8000 });
        steps.push(`${what} ready`);
      } catch (e) {
        fail(`the ${what} never said ready (does the page load, and does it finish loading?)`);
        throw new Error('stopped: ' + what + ' not ready');
      }
    };
    // the Back button's zone, and sideways overflow, inside the screen's frame
    const inspectScreen = async (label) => {
      const frame = page.frames().find((f) => f.url().includes(`/pack/${id}/`) && f.url().includes('mode=screen'));
      if (!frame) return;
      const found = await frame.evaluate(() => {
        const root = document.documentElement, out = { overflow: root.scrollWidth > root.clientWidth + 1, cover: [] };
        for (const el of document.querySelectorAll('body *')) {
          const r = el.getBoundingClientRect();
          if (r.width === 0 || r.height === 0 || r.left >= 56 || r.top >= 56 || r.right <= 0 || r.bottom <= 0) continue;
          const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
          if (own || /^(BUTTON|A|INPUT|SELECT|TEXTAREA|IMG|SVG|CANVAS|VIDEO)$/i.test(el.tagName)) out.cover.push(el.tagName.toLowerCase() + (el.id ? '#' + el.id : ''));
        }
        return out;
      });
      if (found.overflow) warn(`${label}: the page scrolls sideways`);
      if (found.cover.length) warn(`${label}: ${found.cover.join(', ')} sit in the top-left 56 pt, where the app's Back button is`);
    };

    await page.goto(`${server.url}/preview/${id}?device=${device}`);
    await page.waitForFunction(() => window.__playground);
    await page.waitForTimeout(400);   // the land data and the first frame

    try {
      if (manifest.presentation === 'globe') {
        if (manifest.overlay) await waitReady('overlay');
        await page.waitForTimeout(900);
        await shot('globe');
        if (manifest.taps && manifest.overlay) {
          const box = await page.locator('#stage').boundingBox();
          await page.mouse.click(box.x + box.width / 2 + 30, box.y + box.height / 2 - 20);
          await page.waitForTimeout(450);
          await shot('globe-tap');
          steps.push('tapped the globe');
        }
        const opening = (manifest.markers || []).find((m) => m.opens);
        if (opening) {
          const target = page.locator('.marker').nth(manifest.markers.indexOf(opening));
          if (await target.isVisible()) {
            await target.click();
            steps.push(`opened the screen from marker "${opening.id}"`);
          } else {
            await page.evaluate((m) => window.__playground.openScreen({ marker: m }), opening.id);
            warn(`the opening marker "${opening.id}" is not visible at the starting camera; the screen was opened directly`);
          }
        } else if (manifest.entry) {
          await page.evaluate(() => window.__playground.openScreen({}));   // reached by terra.open() in the app
          steps.push('opened the screen directly');
        }
      }
      if (manifest.presentation === 'screen' || await page.locator('iframe.screen').count()) {
        await waitReady('screen');
        await page.waitForTimeout(700);   // a fade-in finishes
        await shot('screen');
        await inspectScreen('text size 1');
        await page.evaluate(() => window.__playground.setSettings({ textScale: 2 }));
        await page.waitForTimeout(400);
        await shot('screen-large');
        await inspectScreen('text size 2');
        await page.evaluate(() => window.__playground.setSettings({ textScale: 3.2 }));
        await page.waitForTimeout(300);
        await inspectScreen('text size 3.2');
        await page.evaluate(() => window.__playground.setSettings({ textScale: 1, reduceMotion: true }));
        await page.waitForTimeout(300);
        steps.push('reduce motion on, text size 3.2 and back');
        await page.evaluate(() => window.__playground.closeScreen());
        steps.push('closed the screen');
      } else if (manifest.presentation === 'globe') {
        warn('this prototype has no screen to open (no marker with "opens": true)');
      }
    } catch (e) {
      if (!/^stopped:/.test(e.message)) fail('check stopped: ' + e.message);
    }

    logs = await page.evaluate(() => window.__playground.log);
    for (const l of logs) if (l.level === 'error') fail(`terra.log error from ${l.source}: ${l.message}`);
    for (const l of logs) if (l.level === 'warn') warn(`${l.source}: ${l.message}`);
    for (const u of nonLocal) fail('request to a non-local host: ' + u);
    for (const u of requests) {
      let host = ''; try { host = new URL(u).hostname; } catch (e) { /* data: */ }
      if (host && !LOCAL.has(host)) fail('request to a non-local host: ' + u);
    }
    await context.close();
  } finally {
    await browser.close();
    await server.close();
  }

  const result = { id, device, passed: failures.length === 0, failures, warnings, screenshots: shots, steps, requests: requests.length, logs };
  fs.writeFileSync(path.join(outDir, 'check.json'), JSON.stringify(result, null, 2) + '\n');
  for (const w of warnings) console.log('warn: ' + w);
  for (const f of failures) console.error('FAIL: ' + f);
  for (const l of logs) console.log(`  log ${l.source} ${l.level}: ${l.message}`);
  console.log(`${result.passed ? 'PASS' : 'FAIL'}: ${id} on ${device}, ${shots.length} screenshots in ${path.relative(ROOT, outDir)}/ (${warnings.length} warnings)`);
  process.exit(result.passed ? 0 : 1);
})().catch((e) => { console.error(e); process.exit(1); });
