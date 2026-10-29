#!/usr/bin/env node
// Photographs a pack in the desktop preview at moments, so what a script does over time (a sea coming
// in, a sail crossing, a fire spreading) can be seen without a Mac: the pack is started past its
// doorway, the camera put where asked, and screenshots taken at the seconds given, with touches between.
//
//   node tools/packart/moments.cjs <id> <out dir> '<plan>' [--device iphone|ipad] [--root <dir>]
//   plan: a JSON list of steps, in order:
//     { "at": [lon, lat, zoom] }        put the camera there (no flight)
//     { "wait": seconds }               let time pass
//     { "touch": [lon, lat] }           a tap at that place on the globe
//     { "drag": [[lon, lat], [lon, lat]] }  a one-finger drag from one place to another
//     { "touchScreen": [x, y] }         a tap at that point of the screen
//     { "date": 0.3 }                   a touch on the date's strip, 0 its start to 1 its end
//     { "play": false }                 pause (or play) the date
//     { "shot": "name" }                a screenshot, <out dir>/<name>.png
//   e.g. '[{"at":[30.5,29.4,5]},{"wait":8},{"shot":"flooded"},{"wait":50},{"shot":"ebbing"}]'
'use strict';
const { chromium } = require('playwright');
const fs = require('node:fs'), path = require('node:path');
const { start } = require('../playground/serve.cjs');

const SIZES = { iphone: { width: 393, height: 852 }, ipad: { width: 1032, height: 1376 } };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function main(argv) {
  let id, out, plan, device = 'ipad'; const roots = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--device') device = argv[++i];
    else if (a === '--root') roots.push(argv[++i]);
    else if (!id) id = a; else if (!out) out = a; else if (!plan) plan = JSON.parse(a);
  }
  if (!id || !out || !plan) { console.error('usage: node tools/packart/moments.cjs <id> <out dir> <plan json> [--device ipad|iphone]'); process.exit(2); }
  fs.mkdirSync(out, { recursive: true });
  const server = await start({ port: 0, roots });
  const browser = await chromium.launch({ args: ['--no-sandbox'] });
  try {
    const page = await browser.newPage({ viewport: SIZES[device], deviceScaleFactor: 2 });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto(`http://127.0.0.1:${server.port}/preview/${id}?device=${device}&start=1&bare=1`);
    await page.waitForFunction(() => window.__playground && window.__playground.ready, null, { timeout: 30000 });
    await page.waitForFunction(() => window.__playground.started, null, { timeout: 30000 }).catch(() => console.error('the pack did not start'));
    await sleep(500);
    const project = (lon, lat) => page.evaluate(([a, b]) => window.__playground.project(a, b), [lon, lat]);
    for (const step of plan) {
      if (step.at) {
        await page.evaluate(([lon, lat, zoom]) => window.__playground.flyTo(lon, lat, zoom, 0), step.at);
        await sleep(400);
      } else if (step.wait) {
        await sleep(step.wait * 1000);
      } else if (step.touch) {
        const p = await project(step.touch[0], step.touch[1]);
        await page.mouse.click(p.x, p.y);
        await sleep(300);
      } else if (step.date != null) {
        // a touch on the date's strip at a fraction of its length (0 its start, 1 its end), as a reader scrubs
        const box = await page.evaluate(() => { const r = document.getElementById('date-strip').getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height }; });
        await page.mouse.click(box.x + box.w * step.date, box.y + box.h / 2);
        await sleep(300);
      } else if (step.play != null) {
        const label = await page.evaluate(() => document.getElementById('date-play').getAttribute('aria-label'));
        if ((label === 'Pause') !== step.play) await page.click('#date-play');
        await sleep(200);
      } else if (step.drag) {
        // a one-finger drag from one place on the globe to another, in a few steps
        const a = await project(step.drag[0][0], step.drag[0][1]), b = await project(step.drag[1][0], step.drag[1][1]);
        await page.mouse.move(a.x, a.y); await page.mouse.down();
        for (let i = 1; i <= 12; i++) { await page.mouse.move(a.x + (b.x - a.x) * i / 12, a.y + (b.y - a.y) * i / 12); await sleep(40); }
        await page.mouse.up();
        await sleep(300);
      } else if (step.touchScreen) {
        await page.mouse.click(step.touchScreen[0], step.touchScreen[1]);
        await sleep(300);
      } else if (step.shot) {
        // (a few frames first: a sprite leaving its dates fades out over half a second of drawn frames, and
        // the preview draws none while nothing moves, so without them a ghost of it would be in the shot)
        for (let i = 0; i < 8; i++) { await page.evaluate(() => window.__playground.redraw()); await sleep(80); }
        await page.screenshot({ path: path.join(out, step.shot + '.png') });
        console.log(`${step.shot}.png at ${(await page.evaluate(() => window.__playground.clock())).toFixed(1)} s`);
      }
    }
    if (errors.length) { console.error('errors:\n  ' + errors.join('\n  ')); process.exitCode = 1; }
  } finally {
    await browser.close();
    server.close();
  }
}
main(process.argv.slice(2)).catch((e) => { console.error(e); process.exit(1); });
