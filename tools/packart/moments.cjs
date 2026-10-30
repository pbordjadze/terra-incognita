#!/usr/bin/env node
// Photographs a pack in the desktop preview at moments, so what a script does over time (a sea coming
// in, a sail crossing, a fire spreading, a herd on the move) can be seen without a Mac: the pack is started
// past its doorway, the camera put or flown where asked, and screenshots taken at the moments given, with
// touches and the date between, each with the pack's state printed beside it. The one pack photographer
// (it took in tools/playground/explore.cjs, painting-company row 183).
//
//   node tools/packart/moments.cjs <id> <out dir> '<plan>' [--device iphone|ipad] [--root <dir>] [--no-idle]
//   plan: a JSON list of steps, in order (times in seconds):
//     { "at": [lon, lat, zoom] }               put the camera there (no flight)
//     { "fly": [lon, lat, zoom, seconds] }     fly there, as a pack's flyTo would, and wait for it to land
//     { "wait": seconds }                      let time pass (the date plays on meanwhile)
//     { "touch": [lon, lat] }                  a tap at that place on the globe
//     { "tap": "first-1" }                     a tap on the touchable thing whose description has this in it
//                                              (a sprite's id or label, a layer's id: what check.cjs prints)
//     { "drag": [[lon, lat], [lon, lat]] }     a one-finger drag from one place to another
//     { "touchScreen": [x, y] }                a tap at that point of the screen
//     { "date": 0.3 }                          a touch on the date's strip, 0 its start to 1 its end (it snaps
//                                              to a tick within a fingertip)
//     { "play": false }                        pause (or play) the date
//     { "open": true }                         the chip's Open
//     { "shot": "name" }                       a screenshot, <out dir>/<name>.png, and the date, its speed, the
//                                              sprites, the camera and the caption printed
//   e.g. '[{"at":[30.5,29.4,5]},{"wait":8},{"shot":"flooded"},{"wait":50},{"shot":"ebbing"}]'
//        '[{"wait":10},{"play":false},{"fly":[-165,64,4.5,1]},{"date":0.34},{"shot":"bering"},{"tap":"first-1"}]'
// --no-idle keeps the globe from turning by itself however long the plan waits. The pack's and the host's
// warnings and errors are printed at the end; exit 1 on a page error or an error logged.
'use strict';
const { chromium } = require('playwright');
const fs = require('node:fs'), path = require('node:path');
const { start } = require('../playground/serve.cjs');

const SIZES = { iphone: { width: 393, height: 852 }, ipad: { width: 1032, height: 1376 } };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const USAGE = 'usage: node tools/packart/moments.cjs <id> <out dir> \'<plan json>\' [--device ipad|iphone] [--root <dir>] [--no-idle]';

async function main(argv) {
  let id, out, plan, device = 'ipad', idle = true; const roots = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--device') device = argv[++i];
    else if (a === '--root') roots.push(argv[++i]);
    else if (a === '--no-idle') idle = false;
    else if (!id) id = a; else if (!out) out = a; else if (!plan) plan = JSON.parse(a);
  }
  if (!id || !out || !plan || !SIZES[device]) { console.error(USAGE); process.exit(2); }
  const known = new Set(['at', 'fly', 'wait', 'touch', 'tap', 'drag', 'touchScreen', 'date', 'play', 'open', 'shot']);
  for (const step of plan) {
    const bad = Object.keys(step).filter((k) => !known.has(k));
    if (bad.length) { console.error(`a step has ${bad.join(', ')}, which the photographer doesn't know: ${JSON.stringify(step)}\n${USAGE}`); process.exit(2); }
  }
  fs.mkdirSync(out, { recursive: true });
  const server = await start({ port: 0, roots });
  const browser = await chromium.launch({ args: ['--no-sandbox'] });
  try {
    const page = await browser.newPage({ viewport: SIZES[device], deviceScaleFactor: 2 });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto(`http://127.0.0.1:${server.port}/preview/${id}?device=${device}&start=1&bare=1${idle ? '' : '&idleAfter=100000'}`);
    await page.waitForFunction(() => window.__playground && window.__playground.ready, null, { timeout: 30000 });
    await page.waitForFunction(() => window.__playground.started, null, { timeout: 30000 }).catch(() => console.error('the pack did not start'));
    await sleep(500);
    const project = (lon, lat) => page.evaluate(([a, b]) => window.__playground.project(a, b), [lon, lat]);
    for (const step of plan) {
      if (step.at) {
        await page.evaluate(([lon, lat, zoom]) => window.__playground.flyTo(lon, lat, zoom, 0), step.at);
        await sleep(400);
      } else if (step.fly) {
        await page.evaluate((f) => window.__playground.flyTo(f[0], f[1], f[2], f[3] || 0), step.fly);
        await sleep((step.fly[3] || 0) * 1000 + 300);
      } else if (step.wait) {
        await sleep(step.wait * 1000);
      } else if (step.touch) {
        const p = await project(step.touch[0], step.touch[1]);
        await page.mouse.click(p.x, p.y);
        await sleep(300);
      } else if (step.tap) {
        const t = (await page.evaluate(() => window.__playground.touchables())).find((x) => x.what.includes(step.tap));
        if (t) await page.mouse.click(t.x, t.y);
        console.log(t ? 'touched ' + t.what : 'nothing touchable like "' + step.tap + '" in view');
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
      } else if (step.open) {
        await page.evaluate(() => window.__playground.open());
        await sleep(600);
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
        await page.evaluate(() => window.__playground.redraw());
        await sleep(150);
        await page.screenshot({ path: path.join(out, step.shot + '.png') });
        const st = await page.evaluate(() => {
          const s = window.__playground.state(), c = window.__playground.camera();
          return { date: s.time && s.time.label, speed: s.time && s.time.speed, sprites: s.sprites.length,
                   camera: [+c.lon.toFixed(2), +c.lat.toFixed(2), +c.zoom.toFixed(2)], caption: s.caption,
                   clock: +window.__playground.clock().toFixed(1) };
        });
        console.log(`${step.shot}.png at ${st.clock.toFixed(1)} s ${JSON.stringify({ date: st.date, speed: st.speed, sprites: st.sprites, camera: st.camera, caption: st.caption })}`);
      }
    }
    const logs = await page.evaluate(() => window.__playground.log.filter((l) => l.level !== 'info').map((l) => l.source + ' ' + l.level + ': ' + l.message));
    if (logs.length) console.log(logs.join('\n'));
    if (logs.some((l) => / error: /.test(l))) errors.push('the pack or the host logged an error');
    if (errors.length) { console.error('errors:\n  ' + errors.join('\n  ')); process.exitCode = 1; }
  } finally {
    await browser.close();
    server.close();
  }
}
main(process.argv.slice(2)).catch((e) => { console.error(e); process.exit(1); });
