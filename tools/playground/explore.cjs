#!/usr/bin/env node
// Plays a pack in the desktop preview from a plan, and screenshots it with its state: for looking at a
// pack's moments (a date, a place, a zoom, a touch) without a Mac, beyond what check.cjs checks.
//
//   node tools/playground/explore.cjs <id> <out-dir> [--device iphone|ipad] '<plan>'
//
// The plan is JSON, a list of steps done in order; each may have any of:
//   { "wait": ms }                       wait (the date plays on meanwhile)
//   { "fly": [lon, lat, zoom, seconds] } move the camera, as the reader would
//   { "strip": 0.4 }                     touch the date's strip that far along (it snaps to a tick within 10 points)
//   { "play": 1 }                        press play or pause
//   { "tap": "first-1" }                 touch the touchable thing whose description includes this (a sprite's id, a label)
//   { "open": 1 }                        the chip's Open
//   { "shot": "name" }                   save <out-dir>/<name>.png and print the date, its speed, the sprites and camera
// e.g. '[{"wait":10000},{"play":1},{"fly":[-165,64,4.5]},{"strip":0.34,"shot":"bering"}]'
// Warnings and errors the pack or the host logged are printed at the end.
'use strict';
const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');
const { start } = require('./serve.cjs');

const SIZES = { iphone: { width: 393, height: 852 }, ipad: { width: 1032, height: 1376 } };

async function main() {
  const args = process.argv.slice(2);
  const di = args.indexOf('--device');
  const device = di >= 0 ? args.splice(di, 2)[1] : 'iphone';
  const [id, out, planText] = args;
  if (!id || !out || !SIZES[device]) {
    console.error('usage: node tools/playground/explore.cjs <id> <out-dir> [--device iphone|ipad] \'<plan JSON>\'');
    process.exit(2);
  }
  const plan = JSON.parse(planText || '[]');
  fs.mkdirSync(out, { recursive: true });
  const server = await start({ port: 0, roots: [], packsDir: null });
  const browser = await chromium.launch();
  try {
    const context = await browser.newContext({ viewport: SIZES[device], deviceScaleFactor: 2 });
    const page = await context.newPage();
    await page.goto(`${server.url}/preview/${id}?device=${device}&start=1&bare=1&idleAfter=100000`);
    await page.waitForFunction(() => window.__playground && window.__playground.ready, null, { timeout: 20000 });
    const pg = (fn, a) => page.evaluate(fn, a);
    for (const step of plan) {
      if (step.wait) await page.waitForTimeout(step.wait);
      if (step.fly) {
        await pg((f) => window.__playground.flyTo(f[0], f[1], f[2], f[3] || 0), step.fly);
        await page.waitForTimeout((step.fly[3] || 0) * 1000 + 300);
      }
      if (step.strip != null) {
        const box = await page.locator('#date-strip').boundingBox();
        if (box) await page.mouse.click(box.x + box.width * step.strip, box.y + box.height / 2);
        await page.waitForTimeout(400);
      }
      if (step.play) { await page.click('#date-play'); await page.waitForTimeout(200); }
      if (step.tap) {
        const t = (await pg(() => window.__playground.touchables())).find((x) => x.what.includes(step.tap));
        if (t) await page.mouse.click(t.x, t.y);
        console.log(t ? 'touched ' + t.what : 'nothing touchable like "' + step.tap + '" in view');
        await page.waitForTimeout(1800);
      }
      if (step.open) { await pg(() => window.__playground.open()); await page.waitForTimeout(600); }
      if (step.shot) {
        const st = await pg(() => {
          const s = window.__playground.state(), c = window.__playground.camera();
          return { date: s.time && s.time.label, speed: s.time && s.time.speed, sprites: s.sprites.length,
                   camera: [+c.lon.toFixed(2), +c.lat.toFixed(2), +c.zoom.toFixed(2)], caption: s.caption };
        });
        console.log(step.shot, JSON.stringify(st));
        await page.screenshot({ path: path.join(out, step.shot + '.png') });
      }
    }
    const logs = await pg(() => window.__playground.log.filter((l) => l.level !== 'info').map((l) => l.source + ' ' + l.level + ': ' + l.message));
    if (logs.length) console.log(logs.join('\n'));
  } finally {
    await browser.close();
    await server.close();
  }
}

main().catch((e) => { console.error(e); process.exit(1); });
