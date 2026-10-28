'use strict';
// Checks the preview harness's runtime-2 host in headless Chromium: the geography, date, caption and globe
// behaviour a pack asks for through terra.geo / terra.time / terra.caption / terra.globe, drawn and heard as the
// app would. It builds a throwaway runtime-2 pack (build/harness-fixture/geo-fixture/, never in Prototypes/),
// serves it with tools/playground/serve.cjs, and at phone (402x874) and iPad (1032x1376) point sizes:
//   - draws every geo style and reads the canvas's pixels (fills, outlines, dashes, dots, rings, labels with
//     halos, symbols, soundings, offsets, zoom limits, layer order, the far side, the limb),
//   - drives the date strip with the mouse, the keyboard and the play button, and reads what the page heard,
//   - checks the caption, today's politics and `globe.historical`, the idle turn and `globe.still`, the reset.
// Screenshots go to build/harness-check/<device>-*.png and the verdicts to build/harness-check/report.json.
//   node tools/playground/harness_check.cjs [--device phone|ipad]      (default: both)
const { chromium } = require('playwright');
const fs = require('node:fs'), path = require('node:path'), zlib = require('node:zlib');
const { start } = require('./serve.cjs');

const ROOT = path.resolve(__dirname, '../..');
const FIXTURE = path.join(ROOT, 'build', 'harness-fixture');
const OUT = path.join(ROOT, 'build', 'harness-check');
const PACK = 'geo-fixture';
const LOCAL = new Set(['127.0.0.1', 'localhost']);
const DEVICES = {
  phone: { size: [402, 874], viewport: { width: 1100, height: 1100 }, reading: 1.0 },
  ipad: { size: [1032, 1376], viewport: { width: 1700, height: 1700 }, reading: 1.2 },
};
const CAMERA = { lon: 30, lat: 36, zoom: 3 };

// ---- the fixture pack: a runtime-2 overlay that only listens; the check drives terra from outside
function crc32(buf) {
  let c, crc = ~0;
  for (let n = 0; n < buf.length; n++) {
    c = (crc ^ buf[n]) & 0xff;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    crc = (crc >>> 8) ^ c;
  }
  return ~crc >>> 0;
}
function png(w, h, [r, g, b]) {
  const row = Buffer.alloc(1 + 4 * w);
  for (let x = 0; x < w; x++) row.set([r, g, b, 255], 1 + 4 * x);
  const raw = Buffer.concat(Array.from({ length: h }, () => row));
  const chunk = (type, data) => {
    const t = Buffer.concat([Buffer.from(type), data]), len = Buffer.alloc(4), crc = Buffer.alloc(4);
    len.writeUInt32BE(data.length); crc.writeUInt32BE(crc32(t));
    return Buffer.concat([len, t, crc]);
  };
  const head = Buffer.alloc(13);
  head.writeUInt32BE(w, 0); head.writeUInt32BE(h, 4); head.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', head), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}
const SYMBOL_RGB = [46, 139, 87];
function writeFixture() {
  const dir = path.join(FIXTURE, PACK);
  fs.rmSync(FIXTURE, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'prototype.json'), JSON.stringify({
    id: PACK, title: 'Geo fixture', summary: 'A runtime-2 pack for the harness check.', runtime: 2, presentation: 'globe',
    overlay: 'overlay.html', taps: true, shelf: 'current', background: '#F6F2E7', anchor: CAMERA,
  }, null, 2));
  fs.writeFileSync(path.join(dir, 'overlay.html'), `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Fixture overlay</title>
<style>html, body { margin: 0; height: 100%; background: transparent; }</style></head><body>
<script>
  window.heard = []; window.taps = [];
  terra.on('time', function (e) { window.heard.push({ value: e.value, playing: e.playing, by: e.by, t: Date.now() }); });
  terra.on('tap', function (e) { window.taps.push(e); });
</script></body></html>`);
  fs.writeFileSync(path.join(dir, 'symbol.png'), png(20, 20, SYMBOL_RGB));
}

// ---- in the page: reading the canvas the harness draws the globe on (colours are [r, g, b])
function installProbe() {
  const cv = () => document.getElementById('globe');
  const dpr = () => window.devicePixelRatio || 1;
  let before = null;
  const whole = () => { const c = cv(); return c.getContext('2d').getImageData(0, 0, c.width, c.height); };
  const near = (d, i, rgb, tol) => Math.abs(d[i] - rgb[0]) <= tol && Math.abs(d[i + 1] - rgb[1]) <= tol && Math.abs(d[i + 2] - rgb[2]) <= tol && d[i + 3] > 250;
  // the pixels of a box given in points: { d, x0, y0, w, h, k } (device pixels from x0, y0; d is row-major RGBA)
  const grab = (x, y, w, h) => {
    const c = cv(), k = dpr();
    const x0 = Math.max(0, Math.floor(x * k)), y0 = Math.max(0, Math.floor(y * k));
    const x1 = Math.min(c.width, Math.ceil((x + w) * k)), y1 = Math.min(c.height, Math.ceil((y + h) * k));
    const pw = Math.max(0, x1 - x0), ph = Math.max(0, y1 - y0);
    return { d: pw && ph ? c.getContext('2d').getImageData(x0, y0, pw, ph).data : new Uint8ClampedArray(0), x0, y0, w: pw, h: ph, k };
  };
  window.__t = {
    snap() { before = whole(); },
    // how many pixels within r points of (x, y) are within tol of `rgb` laid at `alpha` over what the snapshot had there
    match(x, y, r, rgb, alpha, tol) {
      const g = grab(x - r, y - r, 2 * r + 1 / dpr(), 2 * r + 1 / dpr()), b = before.data, bw = before.width;
      let n = 0;
      for (let py = 0; py < g.h; py++) for (let px = 0; px < g.w; px++) {
        if (Math.hypot((g.x0 + px + 0.5) / g.k - x, (g.y0 + py + 0.5) / g.k - y) > r + 1 / g.k) continue;
        const i = 4 * (py * g.w + px), j = 4 * ((g.y0 + py) * bw + g.x0 + px);
        let ok = g.d[i + 3] > 250;
        for (let c = 0; c < 3 && ok; c++) ok = Math.abs(g.d[i + c] - (b[j + c] * (1 - alpha) + rgb[c] * alpha)) <= tol;
        if (ok) n++;
      }
      return n;
    },
    // pixels in a box (points) near a colour, and where they lie (points, pixel centres)
    scan(x, y, w, h, rgb, tol) {
      const g = grab(x, y, w, h);
      let n = 0, sx = 0, sy = 0, x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9;
      for (let py = 0; py < g.h; py++) for (let px = 0; px < g.w; px++) {
        if (!near(g.d, 4 * (py * g.w + px), rgb, tol)) continue;
        const X = (g.x0 + px + 0.5) / g.k, Y = (g.y0 + py + 0.5) / g.k;
        n++; sx += X; sy += Y; x0 = Math.min(x0, X); y0 = Math.min(y0, Y); x1 = Math.max(x1, X); y1 = Math.max(y1, Y);
      }
      return n ? { n, cx: sx / n, cy: sy / n, x0, y0, x1, y1 } : { n: 0 };
    },
    // pixels in a box (points) that differ from the snapshot by more than 24 in any channel
    changed(x, y, w, h) {
      const g = grab(x, y, w, h), b = before.data, bw = before.width;
      let n = 0;
      for (let py = 0; py < g.h; py++) for (let px = 0; px < g.w; px++) {
        const i = 4 * (py * g.w + px), j = 4 * ((g.y0 + py) * bw + g.x0 + px);
        for (let c = 0; c < 4; c++) if (Math.abs(g.d[i + c] - b[j + c]) > 24) { n++; break; }
      }
      return n;
    },
    hash(x, y, w, h) {
      const g = grab(x, y, w, h);
      let hsh = 2166136261 >>> 0;
      for (let i = 0; i < g.d.length; i++) hsh = Math.imul(hsh ^ g.d[i], 16777619) >>> 0;
      return hsh;
    },
    // pixels with any alpha in the ring of radius r0..r1 points about (cx, cy): a layer must not spill past the limb
    annulus(cx, cy, r0, r1) {
      const g = grab(cx - r1, cy - r1, 2 * r1, 2 * r1);
      let n = 0;
      for (let py = 0; py < g.h; py++) for (let px = 0; px < g.w; px++) {
        const r = Math.hypot((g.x0 + px + 0.5) / g.k - cx, (g.y0 + py + 0.5) / g.k - cy);
        if (r >= r0 && r <= r1 && g.d[4 * (py * g.w + px) + 3] > 0) n++;
      }
      return n;
    },
    // a pixel of a PNG screenshot (base64), for the DOM that is not on the canvas
    async png(b64, points) {
      const img = new Image();
      img.src = 'data:image/png;base64,' + b64;
      await img.decode();
      const c = document.createElement('canvas');
      c.width = img.width; c.height = img.height;
      const g = c.getContext('2d');
      g.drawImage(img, 0, 0);
      return points.map(([x, y]) => Array.from(g.getImageData(Math.round(x * img.width), Math.round(y * img.height), 1, 1).data));
    },
  };
}

// ---- the run
const results = [];
function check(name, ok, detail) {
  results.push({ name, ok: !!ok, detail: ok ? undefined : detail === undefined ? undefined : String(typeof detail === 'string' ? detail : JSON.stringify(detail)) });
  if (!ok) console.error('FAIL: ' + name + (detail === undefined ? '' : ' -- ' + (typeof detail === 'string' ? detail : JSON.stringify(detail))));
}
const near = (a, b, tol) => Math.abs(a - b) <= tol;
// the extent of a scan along an axis, pixel edge to pixel edge (a scan reports the pixels' centres; the screenshots are at 2x)
const wide = (s, axis) => (axis === 'x' ? s.x1 - s.x0 : s.y1 - s.y0) + 0.5;
const rgbNear = (a, b, tol) => a.slice(0, 3).every((v, i) => Math.abs(v - b[i]) <= tol);
const blend = (base, rgb, a) => rgb.map((v, i) => base[i] * (1 - a) + v * a);
const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const INK = hex('#3B4A54'), PAPER = hex('#F6F2E7'), BORDER = hex('#8C8478'), SOUND = hex('#1C7AB0'), SOUND_TEXT = hex('#1C5C8C');
const OCEAN = hex('#C6ECFF'), LAND = hex('#F6E1B9'), NAME_INK = hex('#5F5A51');
const darken = (rgb) => rgb.map((v) => v * 0.7);
const yearText = (v) => { const y = Math.floor(v); return y < 0 ? -y + ' BCE' : String(y); };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function runDevice(browser, server, name, outLog) {
  const dev = DEVICES[name], [W, H] = dev.size, rs = dev.reading;
  const context = await browser.newContext({ viewport: dev.viewport, deviceScaleFactor: 2 });
  const requests = [];
  await context.route((url) => !LOCAL.has(url.hostname) && !['data:', 'blob:', 'about:'].includes(url.protocol), (route) => { requests.push('BLOCKED ' + route.request().url()); route.abort(); });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', (e) => problems.push('page error: ' + e.message));
  page.on('console', (m) => { if (m.type() === 'error') problems.push('console error: ' + m.text()); });
  page.on('request', (r) => requests.push(r.url()));
  page.on('response', (r) => { if (r.status() >= 400) problems.push(`${r.status()} for ${r.url().replace(server.url, '')}`); });
  await page.addInitScript(installProbe);
  const tag = (s) => `[${name}] ${s}`;
  const t = (n, ok, d) => check(tag(n), ok, d);

  // ---- helpers
  const stage = { x: 0, y: 0 };
  const url = (extra) => `${server.url}/preview/${PACK}?device=${name}&size=${W}x${H}&idleAfter=3600${extra || ''}`;
  const overlay = () => page.frames().find((f) => f.url().includes(`/pack/${PACK}/`) && f.url().includes('mode=overlay'));
  const ov = (fn, arg) => overlay().evaluate(fn, arg);
  const pg = (fn, arg) => page.evaluate(fn, arg);
  const frames = (n = 3) => pg((k) => new Promise((res) => { let i = 0; const f = () => (++i >= k ? res() : requestAnimationFrame(f)); requestAnimationFrame(f); }), n);
  let syncN = 0;
  // the page's messages arrive in order: once a log line sent after them has arrived, they all have, then a frame draws
  const sync = async () => {
    const mark = `sync-${name}-${++syncN}`;
    await ov((m) => terra.log(m), mark);
    await page.waitForFunction((m) => window.__playground.log.some((l) => l.message === m), mark);
    await frames();
  };
  const put = async (id, features, options) => { await ov(([i, f, o]) => terra.geo.set(i, f, o), [id, features, options]); await sync(); };
  const proj = (lon, lat) => pg(([a, b]) => terra.project(a, b, window.__playground.camera()), [lon, lat]);
  const probe = (fn, ...args) => pg(([f, a]) => window.__t[f](...a), [fn, args]);
  const pixel = (x, y) => pg(([x0, y0]) => { const c = document.getElementById('globe'), k = window.devicePixelRatio; return Array.from(c.getContext('2d').getImageData(Math.round(x0 * k), Math.round(y0 * k), 1, 1).data); }, [x, y]);
  const reset = async (cam) => {
    await ov(() => { terra.geo.clear(); terra.time.hide(); terra.caption.clear(); terra.globe.set({}); });
    const c = cam || CAMERA;
    await pg(([c0]) => window.__playground.flyTo(c0.lon, c0.lat, c0.zoom, 0), [c]);
    await page.evaluate(() => window.__playground.setSettings({ reduceMotion: false, textScale: 1 }));
    await sync();
  };
  const heard = () => ov(() => window.heard.slice());
  const shot = async (n) => {
    await frames();
    await page.locator('#device').screenshot({ path: path.join(OUT, `${name}-${n}.png`) });
    outLog.push(`${name}-${n}.png`);
  };
  const dot = (x, y, r = 3) => ({ x0: x - r, y0: y - r, w: 2 * r + 1, h: 2 * r + 1 });
  const point = (lon, lat) => ({ type: 'Point', coordinates: [lon, lat] });
  const feat = (geometry, properties) => ({ type: 'Feature', geometry, properties: properties || {} });
  const rect = (lon0, lat0, lon1, lat1) => [[lon0, lat1], [lon1, lat1], [lon1, lat0], [lon0, lat0], [lon0, lat1]];

  // ---- load
  await page.goto(url());
  await page.waitForFunction(() => window.__playground && window.__playground.ready.overlay, null, { timeout: 8000 });
  await pg(() => window.__playground.fonts);
  await page.waitForTimeout(500);
  const box0 = await page.locator('#stage').boundingBox();
  Object.assign(stage, box0);
  t('the device is the requested size', near(box0.width, W, 0.5) && near(box0.height, H, 0.5), box0);
  t('the reading scale follows Globe/type.json (' + rs + ')', near(await pg(() => window.__playground.reading()), rs, 1e-6), await pg(() => window.__playground.reading()));
  t('the globe\'s type is loaded', await pg(() => document.fonts.check('15px "Libertinus Serif"') && document.fonts.check('italic 15px "Libertinus Serif"') && document.fonts.check('600 15px "Libertinus Serif"')));
  t('the page told the harness it is runtime 2 and was heard ready', await pg(() => window.terra.runtime === 2));
  await reset();

  const c0 = await proj(CAMERA.lon, CAMERA.lat);        // the middle of the screen
  t('the camera looks at the middle of the screen', near(c0.x, W / 2, 0.01) && near(c0.y, H / 2, 0.01) && near(c0.facing, 1, 1e-9), c0);

  // ---- area: fill at 28% of the colour, outline 30% darker, in layer order, replaced in place, removed
  {
    await probe('snap');
    const geom = { type: 'Polygon', coordinates: [rect(26, 33, 34, 38)] };
    await put('a', [feat(geom)], { style: 'area' });
    const mid = await proj(30, 35.5), corner = await proj(26, 38);
    const orange = hex('#C17846');
    t('area: the fill is the colour at 28% over the globe', (await probe('match', mid.x, mid.y, 0.6, orange, 0.28, 3)) > 0, await pixel(mid.x, mid.y));
    t('area: the outline is the colour 30% darker, opaque', (await probe('match', corner.x, corner.y, 1.2, darken(orange), 1, 6)) > 0);
    const edge = await proj(30, 38);
    t('area: the outline is 2.2 pt wide (x reading scale)', await (async () => {
      const s = await probe('scan', edge.x - 6, edge.y - 8 * rs, 12, 16 * rs, darken(orange).map(Math.round), 8);
      return s.n > 0 && near(wide(s, 'y'), 2.2 * rs, 1.0);
    })(), await probe('scan', edge.x - 6, edge.y - 8 * rs, 12, 16 * rs, darken(orange).map(Math.round), 8));
    // a hole stays unfilled; a colour option is used
    await put('hole', [feat({ type: 'Polygon', coordinates: [rect(24, 40, 32, 46), rect(26, 42, 30, 44)] })], { style: 'area', color: '#1F7A8C' });
    const ring_ = await proj(25, 43), hole = await proj(28, 43), teal = hex('#1F7A8C');
    t('area: a colour option fills with that colour', (await probe('match', ring_.x, ring_.y, 0.6, teal, 0.28, 3)) > 0);
    t('area: a hole in the polygon is not filled', (await probe('changed', hole.x - 2, hole.y - 2, 5, 5)) === 0, await probe('changed', hole.x - 2, hole.y - 2, 5, 5));
    await shot('area');

    // order: a later id is above an earlier one; setting an id again replaces it where it was
    await ov(() => terra.geo.clear()); await sync();
    await probe('snap');
    const both = { type: 'Polygon', coordinates: [rect(27, 34, 33, 38)] };
    await put('under', [feat(both)], { style: 'area', color: '#C03030' });
    await put('over', [feat(both)], { style: 'area', color: '#3040C0' });
    const red = hex('#C03030'), blue = hex('#3040C0'), green = hex('#30C030'), m2 = await proj(30, 36);
    const under = await pixel(m2.x, m2.y);
    await ov(() => terra.geo.clear()); await sync();
    const clean = await pixel(m2.x, m2.y);
    await put('under', [feat(both)], { style: 'area', color: '#C03030' });
    await put('over', [feat(both)], { style: 'area', color: '#3040C0' });
    const px = await pixel(m2.x, m2.y);
    const redFirst = blend(blend(clean, red, 0.28), blue, 0.28), blueFirst = blend(blend(clean, blue, 0.28), red, 0.28);
    t('order: a later layer draws above an earlier one', rgbNear(px, redFirst, 3) && !rgbNear(px, blueFirst, 3), { px, redFirst, blueFirst });
    await put('under', [feat(both)], { style: 'area', color: '#30C030' });       // set again: replaced, still underneath
    t('order: setting an id again replaces the layer in its place', rgbNear(await pixel(m2.x, m2.y), blend(blend(clean, green, 0.28), blue, 0.28), 3), await pixel(m2.x, m2.y));
    await ov(() => terra.geo.remove('over')); await sync();
    t('order: remove takes a layer away', rgbNear(await pixel(m2.x, m2.y), blend(clean, green, 0.28), 3), await pixel(m2.x, m2.y));
    await ov(() => terra.geo.clear()); await sync();
    t('order: clear takes them all away', rgbNear(await pixel(m2.x, m2.y), clean, 1), { now: await pixel(m2.x, m2.y), clean, under });
  }

  // ---- lines: border solid, ghost and track dashed
  {
    await reset();
    await probe('snap');
    const pts = [[24, 36], [30, 34], [36, 36]];
    await put('line', [feat({ type: 'LineString', coordinates: pts })], { style: 'border' });
    const v = await proj(30, 34);
    t('border: a solid line in #8C8478', (await probe('match', v.x, v.y, 2, BORDER, 1, 6)) > 0);
    const a = await proj(26, 35), b = await proj(34, 35);
    // along the straight stretch between two vertices of a ghost/track
    const along = async (style, dash, alpha) => {
      await ov(() => terra.geo.clear());
      await put('d', [feat({ type: 'LineString', coordinates: [[24, 38], [36, 38]] })], { style });
      const p = await proj(24, 38), q = await proj(36, 38), n = 240;
      const hits = await pg(async ([p0, q0, n0, alpha0, ink]) => {
        let on = 0, off = 0;
        for (let i = 0; i <= n0; i++) {
          const f = i / n0, x = p0.x + (q0.x - p0.x) * f, y = p0.y + (q0.y - p0.y) * f;
          // a small cross: the line is 1.2-1.6 pt wide
          const hit = window.__t.match(x, y, 0.5, ink, alpha0, 10);
          if (hit > 0) on++; else off++;
        }
        return { on, off };
      }, [p, q, n, alpha, INK]);
      return { ...hits, frac: hits.on / (hits.on + hits.off) };
    };
    const g = await along('ghost', [5, 4], 0.8), tr = await along('track', [3, 4], 0.47);
    t('ghost: dashed [5, 4], #3B4A54 at 80%', g.frac > 0.4 && g.frac < 0.72, g);
    t('track: dashed [3, 4], #3B4A54 at 47%', tr.frac > 0.28 && tr.frac < 0.58, tr);
    // a polygon given to a line style is its outline
    await ov(() => terra.geo.clear()); await sync();
    await put('outline', [feat({ type: 'Polygon', coordinates: [rect(26, 33, 34, 37)] })], { style: 'ghost' });
    const corner = await proj(26, 37), centre = await proj(30, 35);
    t('ghost: a polygon is drawn as its outline, not filled', (await probe('changed', centre.x - 4, centre.y - 4, 9, 9)) === 0 && (await probe('changed', corner.x - 8, corner.y - 8, 17, 17)) > 4);
  }

  // ---- points: dot, ring, label, symbol, sounding, offsets
  {
    await reset();
    await probe('snap');
    const P = await proj(30, 36);
    await put('dots', [point(30, 36)], { style: 'dot' });
    t('dot: filled #3B4A54, radius 3.4 pt (x reading scale)', await (async () => {
      const s = await probe('scan', P.x - 8, P.y - 8, 16, 16, INK, 8);
      return s.n > 0 && near(wide(s, 'x'), 6.8 * rs, 1.2) && near(wide(s, 'y'), 6.8 * rs, 1.2) && near(s.cx, P.x, 0.4) && near(s.cy, P.y, 0.4);
    })(), await probe('scan', P.x - 8, P.y - 8, 16, 16, INK, 8));
    await put('dots', [point(30, 36)], { style: 'dot', color: '#C03030' });
    t('dot: options.color', (await probe('scan', P.x - 8, P.y - 8, 16, 16, hex('#C03030'), 8)).n > 0);
    await put('dots', [{ type: 'MultiPoint', coordinates: [[30, 36]] }], { style: 'ring' });
    const centre = await probe('scan', P.x - 1, P.y - 1, 2, 2, PAPER, 8), edge = await probe('scan', P.x - 8, P.y - 8, 16, 16, INK, 10);
    t('ring: hollow, #F6F2E7 inside, #3B4A54 stroke, radius 3.4 pt', centre.n > 0 && edge.n > 0 && near(wide(edge, 'x'), (6.8 + 1.3) * rs, 1.5), { centre, edge });

    // labels: regular, italic, caps; halo; colour; size; overlap; offset
    const text = (id, str, options, lon = 30, lat = 36) => put(id, [feat(point(lon, lat), { label: str })], Object.assign({ style: 'label' }, options));
    await ov(() => terra.geo.clear()); await sync();
    const area = [P.x - 120, P.y - 30, 240, 110];
    await text('l', 'Smyrna', {});
    const plain = await probe('hash', ...area);
    const ink = await probe('scan', ...area, INK, 6), halo = await probe('scan', ...area, PAPER, 4);
    t('label: drawn in #3B4A54 on a #F6F2E7 halo, centred on the point', ink.n > 20 && halo.n > 20 && near(ink.cx, P.x, 3) && near(ink.cy, P.y, 3 * rs + 2), { ink, halo });
    const size15 = wide(ink, 'y');
    await text('l', 'Smyrna', { size: 30 });
    const big = await probe('scan', ...area, INK, 6);
    t('label: options.size (pt, x reading scale)', big.n > 0 && wide(big, 'y') > size15 * 1.6, { size15, big: wide(big, 'y') });
    await text('l', 'Smyrna', { color: '#C03030' });
    t('label: options.color', (await probe('scan', ...area, hex('#C03030'), 6)).n > 20);
    await text('l', 'Smyrna', { font: 'italic' });
    const italic = await probe('hash', ...area);
    await text('l', 'smyrna', { font: 'caps' });
    const capsLower = await probe('hash', ...area);
    await text('l', 'SMYRNA', { font: 'caps' });
    const capsUpper = await probe('hash', ...area);
    await text('l', 'SMYRNA', {});
    const regularUpper = await probe('hash', ...area);
    t('label: italic differs from regular', italic !== plain);
    t('label: caps is uppercase (and semibold, spaced)', capsLower === capsUpper && capsUpper !== regularUpper);
    // earlier features win room: a second label on the same spot is skipped
    await put('l', [feat(point(30, 36), { label: 'Alpha' }), feat(point(30, 36), { label: 'Beta' })], { style: 'label' });
    const two = await probe('hash', ...area);
    await put('l', [feat(point(30, 36), { label: 'Alpha' })], { style: 'label' });
    const one = await probe('hash', ...area);
    t('label: one that overlaps a placed label is skipped (earlier wins)', two === one);
    await put('l', [feat(point(30, 36), { label: 'Alpha' })], { style: 'label' });
    await put('m', [feat(point(30, 36), { label: 'Beta' })], { style: 'label' });
    t('label: the rule holds across layers', (await probe('hash', ...area)) === one);
    await put('m', [feat(point(30, 36), { label: 'Beta' })], { style: 'label', offset: [0, 40] });
    const lower = await probe('scan', P.x - 60, P.y + 40 * rs - 12, 120, 24, INK, 6);
    t('label: options.offset moves it right/down by points x reading scale, and its collision box with it', (await probe('hash', ...area)) !== one && lower.n > 20 && near(lower.cy, P.y + 40 * rs, 3 * rs + 2), lower);
    await ov(() => terra.geo.clear()); await sync();
    await text('l', 'Smyrna', {});
    const home = await probe('scan', ...area, INK, 6);
    await text('l', 'Smyrna', { offset: [30, 20] });
    const moved = await probe('scan', P.x - 120, P.y - 30, 240 + 30 * rs, 60 + 20 * rs, INK, 6);
    t('label: offset [30, 20] shifts the ink by (30, 20) x reading scale', near(moved.cx - home.cx, 30 * rs, 1) && near(moved.cy - home.cy, 20 * rs, 1), { home, moved });

    // not drawn behind the globe
    await reset({ lon: 30, lat: 36, zoom: 1 });
    await text('far', 'Antipodes', {}, -150, -36);
    await put('fardot', [point(-150, -36)], { style: 'dot' });
    const whole = [W / 2 - 169, H / 2 - 169, 338, 338];
    await probe('snap');
    t('points on the far side are not drawn', (await probe('changed', 0, 0, W, H)) === 0);
    await reset();

    // symbols: the pack's image, centred, width x height pt
    await probe('snap');
    await put('s', [point(30, 36)], { style: 'symbol', image: 'symbol.png', width: 24, height: 12 });
    await page.waitForFunction(() => true); await sync();
    const sy = await probe('scan', P.x - 40, P.y - 40, 80, 80, SYMBOL_RGB, 14);
    t('symbol: the pack\'s image, centred, options.width x options.height pt (x reading scale)', sy.n > 0 && near(sy.cx, P.x, 0.6) && near(sy.cy, P.y, 0.6) && near(wide(sy, 'x'), 24 * rs, 2.2) && near(wide(sy, 'y'), 12 * rs, 2.2), sy);
    await put('s', [point(30, 36)], { style: 'symbol', image: 'symbol.png', width: 24, height: 12, offset: [20, -10] });
    const sy2 = await probe('scan', P.x - 40, P.y - 40, 120, 80, SYMBOL_RGB, 14);
    t('symbol: options.offset shifts it by points x reading scale', near(sy2.cx - sy.cx, 20 * rs, 1) && near(sy2.cy - sy.cy, -10 * rs, 1), { sy, sy2 });

    // sounding: a dashed line down properties.length pt, a 16 pt foot, the label to its right
    await ov(() => terra.geo.clear()); await sync();
    await probe('snap');
    await put('snd', [feat(point(30, 36), { length: 60, label: 'Depth 40' })], { style: 'sounding' });
    const len = 60 * rs, bottom = P.y + len;
    const lineArea = await probe('scan', P.x - 3, P.y, 6, len - 4 * rs, SOUND, 12);
    const foot = await probe('scan', P.x - 14 * rs, bottom - 3, 28 * rs, 6, SOUND, 12);
    const txt = await probe('scan', P.x + 10 * rs, bottom - 12 * rs, 120, 24 * rs, SOUND_TEXT, 8);
    // dashed [2, 3]: along the line some pixels are in, some out
    const ys = []; for (let y = P.y + 2; y < bottom - 4; y += 0.5) ys.push(y);
    const dashHits = await pg(([x, list, ink]) => list.map((y) => window.__t.match(x, y, 0.4, ink, 1, 14) > 0), [P.x, ys, SOUND]);
    const dashFrac = dashHits.filter(Boolean).length / dashHits.length;
    t('sounding: a dashed [2, 3] line #1C7AB0 hangs from the point', lineArea.n > 10 && dashFrac > 0.25 && dashFrac < 0.65, { dashFrac, lineArea: lineArea.n });
    t('sounding: the line is properties.length pt (x reading scale) long and ends in a 16 pt foot', foot.n > 0 && near(wide(foot, 'x'), 16 * rs, 2) && near(foot.cy, bottom, 1.5), { foot, bottom });
    t('sounding: properties.label to the right of the foot, #1C5C8C', txt.n > 15 && txt.x0 > P.x + 8 * rs, txt);
    await shot('sounding');
  }

  // ---- zoom limits
  {
    await reset();
    const spots = { min4: [30, 39], max2: [30, 36], both: [30, 33] };
    const mk = (id, lat, o) => put(id, [point(30, lat)], Object.assign({ style: 'dot' }, o));
    await mk('min4', 39, { minZoom: 4 }); await mk('max2', 36, { maxZoom: 2 }); await mk('both', 33, { minZoom: 2, maxZoom: 4 });
    const seen = async () => {
      const out = {};
      for (const [k, [lon, lat]] of Object.entries(spots)) {
        const p = await proj(lon, lat);
        out[k] = (await probe('scan', p.x - 4, p.y - 4, 9, 9, INK, 10)).n > 0;
      }
      return out;
    };
    t('zoom 3: only the layer whose limits include it', JSON.stringify(await seen()) === JSON.stringify({ min4: false, max2: false, both: true }), await seen());
    await pg(() => window.__playground.flyTo(30, 36, 5, 0)); await frames();
    t('zoom 5: only minZoom 4', JSON.stringify(await seen()) === JSON.stringify({ min4: true, max2: false, both: false }), await seen());
    await pg(() => window.__playground.flyTo(30, 36, 1.5, 0)); await frames();
    t('zoom 1.5: only maxZoom 2', JSON.stringify(await seen()) === JSON.stringify({ min4: false, max2: true, both: false }), await seen());
    await reset();
  }

  // ---- the far side and the limb
  {
    await reset({ lon: 30, lat: 36, zoom: 1 });
    const c = await pg(() => window.__playground.camera());
    await probe('snap');
    await put('line', [feat({ type: 'LineString', coordinates: [[30, 36], [90, 36], [150, 36], [-150, 0], [-60, 36], [10, 40]] })], { style: 'border' });
    t('a line that goes behind the globe stops at the limb and comes back', (await probe('annulus', c.cx, c.cy, c.radius + 3, c.radius + 60)) === 0);
    const cr = await probe('scan', c.cx - c.radius - 2, c.cy - c.radius - 2, 2 * c.radius + 4, 2 * c.radius + 4, BORDER, 4);
    t('...and is drawn on the near side', cr.n > 40, cr);
    await put('fill', [feat({ type: 'Polygon', coordinates: [[[30, 36], [120, 36], [120, -30], [30, -30], [30, 36]]] })], { style: 'area', color: '#C03030' });
    t('a polygon that reaches behind the globe is pulled to the limb and does not spill', (await probe('annulus', c.cx, c.cy, c.radius + 3, c.radius + 60)) === 0);
    const inside = await proj(60, 10);
    t('...and is filled where it faces us', inside.facing > 0 && (await probe('scan', inside.x - 3, inside.y - 3, 7, 7, blend(OCEAN, hex('#C03030'), 0.28).map(Math.round), 14)).n > 0);
    await shot('limb');
    await reset();
  }

  // ---- layers move with the camera
  {
    await reset();
    await put('dot', [point(30, 36)], { style: 'dot' });
    await pg(() => window.__playground.flyTo(26, 34, 3, 0)); await frames();
    const p = await proj(30, 36), p0 = await proj(26, 34);
    t('geography follows the camera', (await probe('scan', p.x - 4, p.y - 4, 9, 9, INK, 8)).n > 0 && (await probe('scan', p0.x - 4, p0.y - 4, 9, 9, INK, 8)).n === 0, { p, p0 });
    const before = await pg(() => window.__playground.camera().lon);
    const sbox = await page.locator('#stage').boundingBox();
    await page.mouse.move(sbox.x + 100, sbox.y + 300); await page.mouse.down(); await page.mouse.move(sbox.x + 160, sbox.y + 300, { steps: 5 }); await page.mouse.up();
    await frames();
    const p2 = await proj(30, 36);
    t('...and a drag of the globe', (await pg(() => window.__playground.camera().lon)) !== before && (await probe('scan', p2.x - 4, p2.y - 4, 9, 9, INK, 8)).n > 0, p2);
    await reset();
  }

  // ---- a bad layer is told to the author
  {
    await ov(() => terra.geo.set('bad', [], { style: 'sparkle' })); await sync();
    const errors = await pg(() => window.__playground.log.filter((l) => l.level === 'error' && /options\.style/.test(l.message)).length);
    t('an unknown style is an error in the log', errors === 1, errors);
    await pg(() => { const log = window.__playground.log; for (let i = log.length - 1; i >= 0; i--) if (log[i].level === 'error' && /options\.style/.test(log[i].message)) log.splice(i, 1); });
    t('...and draws nothing', (await pg(() => window.__playground.geo().length)) === 0);
    await ov(() => terra.geo.set('few', [{ type: 'Point', coordinates: [30, 36] }, { type: 'Polygon', coordinates: [] }], { style: 'dot' })); await sync();
    const warned = await pg(() => window.__playground.log.filter((l) => l.level === 'warn' && /skipped/.test(l.message)).length);
    t('geometry a style cannot draw is a warning', warned === 1, warned);
    await pg(() => { const log = window.__playground.log; for (let i = log.length - 1; i >= 0; i--) if (log[i].level === 'warn' && /skipped/.test(log[i].message)) log.splice(i, 1); });
    await reset();
  }

  // ---- the date: year units, BCE, ticks, drag, click, keys, play
  {
    await reset();
    const yearOpts = { unit: 'year', min: -500, max: 1500, value: -500, ticks: [-300, 0, { at: 1000, hollow: true }, { at: 1400 }], playable: true, rate: 200, playing: false };
    await ov((o) => { terra.caption.set({ title: 'Evidence', text: 'What we can see of the harbour at this date.' }); terra.time.show(o); }, yearOpts);
    await sync();
    t('date: the foot is shown', await page.locator('#foot').isVisible());
    t('date: year units show the integer year, BCE when negative', (await page.getByTestId('foot-title').innerText()) === '500 BCE', await page.getByTestId('foot-title').innerText());
    t('date: caption.title is the eyebrow (capitals), caption.text under the date', (await page.getByTestId('foot-eyebrow').innerText()) === 'EVIDENCE' && /harbour/.test(await page.getByTestId('foot-text').innerText()));
    t('date: window.harnessTime() is the value', (await pg(() => window.harnessTime())) === -500);
    t('date: the page heard the first value from the host', JSON.stringify((await heard()).map((e) => [e.value, e.playing, e.by])) === JSON.stringify([[-500, false, 'host']]), await heard());
    t('date: filled and hollow ticks', (await page.locator('#date-strip .tick').count()) === 4 && (await page.locator('#date-strip .tick.hollow').count()) === 1);
    const tickBox = await page.locator('#date-strip .tick').first().boundingBox();
    t('date: a tick is r 2.6 pt', near(tickBox.width, 5.2, 0.3), tickBox);
    t('date: the play button is 44 pt, at the strip\'s left', await (async () => { const b = await page.getByTestId('date-play').boundingBox(), s = await page.getByTestId('date-strip').boundingBox(); return near(b.width, 44, 0.5) && near(b.height, 44, 0.5) && b.x + b.width <= s.x + 1; })());
    const inner = await page.locator('#foot .inner').boundingBox();
    t('date: the foot\'s column is at most 560 pt wide and centred', inner.width <= 560.5 && near(inner.x + inner.width / 2 - stage.x, W / 2, 1), inner);
    t('date: the ground fades up from the paper colour', /rgba\(246, 242, 231, 0\) 0%, rgba\(246, 242, 231, 0\.92\) 33\.3/.test(await pg(() => getComputedStyle(document.getElementById('foot'), '::before').backgroundImage)), await pg(() => getComputedStyle(document.getElementById('foot'), '::before').backgroundImage));
    const png0 = (await page.locator('#device').screenshot()).toString('base64');
    const [corner, high] = await pg(([b]) => window.__t.png(b, [[0.2, 0.99], [0.5, 0.0]]), [png0]);
    t('date: the foot\'s ground is paper at the bottom', rgbNear(corner, PAPER, 14), corner);
    await shot('year');

    const strip = await page.getByTestId('date-strip').boundingBox();
    const fAt = (x) => Math.min(1, Math.max(0, (x - strip.x - 14) / (strip.width - 28)));
    const yAt = (x) => -500 + 2000 * fAt(x);
    const y0 = strip.y + strip.height / 2;
    const x1 = strip.x + strip.width * 0.25, x2 = strip.x + strip.width * 0.75;
    const n0 = (await heard()).length;
    await page.mouse.move(x1, y0); await page.mouse.down(); await page.mouse.move(x2, y0, { steps: 8 });
    const mid = await pg(() => window.harnessTime());
    await page.mouse.up(); await sync();
    const h1 = (await heard()).slice(n0);
    const last = h1[h1.length - 1];
    t('date: dragging the strip sets the value in proportion', near(await pg(() => window.harnessTime()), yAt(x2), 1.0) && mid > yAt(x1), { value: await pg(() => window.harnessTime()), expect: yAt(x2) });
    t('date: the page hears every drag step as "reader"', h1.length >= 6 && h1.every((e) => e.by === 'reader') && near(last.value, yAt(x2), 1.0) && last.playing === false, h1.slice(-2));
    t('date: the title follows the value', (await page.getByTestId('foot-title').innerText()) === yearText(await pg(() => window.harnessTime())));
    const needle = await page.getByTestId('date-needle').boundingBox();
    t('date: the red needle is at the value', near(needle.x + needle.width / 2, strip.x + 14 + fAt(x2) * (strip.width - 28), 1.0) && /rgb\(176, 66, 52\)/.test(await page.getByTestId('date-needle').evaluate((e) => getComputedStyle(e).backgroundColor)), needle);
    await page.mouse.click(strip.x + strip.width * 0.5, y0); await sync();
    t('date: a click sets the value too', near(await pg(() => window.harnessTime()), yAt(strip.x + strip.width * 0.5), 1.0), await pg(() => window.harnessTime()));
    await page.mouse.click(strip.x + 1, y0); await sync();
    t('date: the left end is min', (await pg(() => window.harnessTime())) === -500);
    await page.mouse.click(strip.x + strip.width - 1, y0); await sync();
    t('date: the right end is max', (await pg(() => window.harnessTime())) === 1500 && (await page.getByTestId('foot-title').innerText()) === '1500');
    await page.getByTestId('date-strip').focus(); await page.keyboard.press('Home'); await sync();
    t('date: keys move it (Home)', (await pg(() => window.harnessTime())) === -500);
    await page.keyboard.press('ArrowRight'); await sync();
    t('date: keys move it (arrow: a hundredth of the range)', near(await pg(() => window.harnessTime()), -480, 1e-6) && (await heard()).slice(-1)[0].by === 'reader', await pg(() => window.harnessTime()));

    // the page sets the date: no echo
    const nBefore = (await heard()).length;
    await ov(() => terra.time.set(1200)); await sync();
    t('date: terra.time.set moves the strip and is not echoed to the page', (await pg(() => window.harnessTime())) === 1200 && (await heard()).length === nBefore && (await page.getByTestId('foot-title').innerText()) === '1200');
    await ov(() => terra.time.set(-100)); await sync();

    // play: rate units a second, at most 30 messages a second, 'play'
    await page.getByTestId('date-play').click(); await sync();
    const tPlay = Date.now(), vPlay = await pg(() => window.harnessTime());
    t('date: the play button starts playback', (await pg(() => window.__playground.time().playing)) === true && (await page.getByTestId('date-play').getAttribute('data-playing')) === 'true');
    await sleep(1000);
    const played = (await heard()).filter((e) => e.t >= tPlay && e.by === 'play');
    const span = played.length > 1 ? (played[played.length - 1].t - played[0].t) / 1000 : 0;
    const speed = span > 0 ? (played[played.length - 1].value - played[0].value) / span : 0;
    t('date: playback advances by rate units a second', near(speed, 200, 50), { speed, n: played.length });
    t('date: playback is told to the page as "play", at most 30 times a second', played.length >= 8 && played.length <= 31 && played.every((e) => e.playing === true), { n: played.length });
    await page.getByTestId('date-play').click(); await sync();
    const paused = await pg(() => window.harnessTime()), hp = (await heard()).slice(-1)[0];
    await sleep(300);
    t('date: the button pauses it, as "reader" with playing false', hp.by === 'reader' && hp.playing === false && (await pg(() => window.harnessTime())) === paused && (await page.getByTestId('date-play').getAttribute('data-playing')) === 'false', hp);
    t('date: the value moved on from where it started', paused > vPlay + 80, { vPlay, paused });

    // dragging while it plays takes it from the clock
    await page.getByTestId('date-play').click(); await sleep(150);
    await page.mouse.click(strip.x + strip.width * 0.3, y0); await sync();
    t('date: a hand on the strip stops playback', (await pg(() => window.__playground.time().playing)) === false);

    // to the end: stops at max, last message playing false
    await ov(() => terra.time.set(1450)); await sync();
    await page.getByTestId('date-play').click();
    await page.waitForFunction(() => window.__playground.time().playing === false, null, { timeout: 4000 });
    const end = (await heard()).slice(-1)[0];
    t('date: playing ends at max and says so', end.value === 1500 && end.playing === false && end.by === 'play' && (await pg(() => window.harnessTime())) === 1500, end);
    await page.getByTestId('date-play').click(); await sleep(250);
    t('date: playing again from the end starts over from min', (await pg(() => window.harnessTime())) < -300 + 200, await pg(() => window.harnessTime()));
    await page.getByTestId('date-play').click();

    // playing: true in show starts it unless Reduce Motion is on
    await ov((o) => terra.time.show(Object.assign({}, o, { playing: true, value: 0 })), yearOpts); await sleep(400); await sync();
    t('date: show({ playing: true }) starts playback', (await pg(() => window.__playground.time().playing)) === true && (await pg(() => window.harnessTime())) > 20);
    await pg(() => window.__playground.setSettings({ reduceMotion: true })); await sync();
    t('date: Reduce Motion stops playback and hides the button', (await pg(() => window.__playground.time().playing)) === false && !(await page.getByTestId('date-play').isVisible()));
    await ov((o) => terra.time.show(Object.assign({}, o, { playing: true, value: 0 })), yearOpts); await sleep(300); await sync();
    t('date: under Reduce Motion show({ playing: true }) does not play', (await pg(() => window.__playground.time().playing)) === false && (await pg(() => window.harnessTime())) === 0);
    await ov(() => terra.time.play(true)); await sleep(300); await sync();
    t('date: ...nor does terra.time.play(true)', (await pg(() => window.__playground.time().playing)) === false && (await pg(() => window.harnessTime())) === 0);
    await pg(() => window.__playground.setSettings({ reduceMotion: false })); await sync();

    // text size: the foot follows the reader's
    await pg(() => window.__playground.setSettings({ textScale: 2 })); await frames();
    t('date: the title grows with the reader\'s text size (30 pt x 2)', near(parseFloat(await page.getByTestId('foot-title').evaluate((e) => getComputedStyle(e).fontSize)), 60, 0.01));
    await pg(() => window.__playground.setSettings({ textScale: 1 })); await frames();
    t('date: the title is Libertinus 30 pt, #253440', await page.getByTestId('foot-title').evaluate((e) => { const s = getComputedStyle(e); return s.fontSize === '30px' && /Libertinus/.test(s.fontFamily) && s.color === 'rgb(37, 52, 64)'; }));
    await ov(() => terra.time.hide()); await sync();
    t('date: hide removes the strip and the date, keeps the caption', !(await page.getByTestId('date-strip').isVisible()) && !(await page.getByTestId('foot-title').isVisible()) && (await page.getByTestId('foot-text').isVisible()));
  }

  // ---- the date: day units
  {
    await reset();
    const day = (y, m, d) => Date.UTC(y, m, d) / 864e5;
    await ov(([v]) => terra.time.show({ unit: 'day', min: -800000, max: 100000, value: v, ticks: [v], playable: false }), [day(1873, 1, 26)]); await sync();
    const text = () => page.getByTestId('foot-title').innerText();
    t('date: day units: "26 February 1873"', (await text()) === '26 February 1873', await text());
    t('date: no play button unless playable', !(await page.getByTestId('date-play').isVisible()));
    await ov(([v]) => terra.time.set(v), [0]); await sync();
    t('date: day 0 is 1 January 1970', (await text()) === '1 January 1970', await text());
    await ov(([v]) => terra.time.set(v), [-1]); await sync();
    t('date: day -1 is 31 December 1969', (await text()) === '31 December 1969', await text());
    await ov(([v]) => terra.time.set(v), [day(-43, 2, 15)]); await sync();
    t('date: before the common era, "N BCE"', (await text()) === '15 March 44 BCE', await text());
    t('date: no caption, no eyebrow or words', !(await page.getByTestId('foot-eyebrow').isVisible()) && !(await page.getByTestId('foot-text').isVisible()));
    await ov(([v]) => terra.time.set(v), [day(1873, 1, 26)]); await sync();
    await shot('day');
  }

  // ---- the caption alone
  {
    await reset();
    t('no date, no caption: no foot', !(await page.locator('#foot').isVisible()));
    await ov(() => terra.caption.set({ title: 'The harbour', text: 'Ships came here from the whole coast.' })); await sync();
    t('caption: eyebrow and words, no date, no strip', (await page.getByTestId('foot-eyebrow').isVisible()) && (await page.getByTestId('foot-text').isVisible()) && !(await page.getByTestId('foot-title').isVisible()) && !(await page.getByTestId('date-strip').isVisible()));
    await shot('caption');
    await ov(() => terra.caption.clear()); await sync();
    t('caption: clear hides the foot', !(await page.locator('#foot').isVisible()));
  }

  // ---- modern politics and globe.historical
  {
    await reset({ lon: 25, lat: 27, zoom: 3 });
    const home = await proj(25, 27);
    await probe('snap');
    const everything = [0, 0, W, H];
    const namesWith = (await probe('scan', ...everything, NAME_INK, 4)).n, coastWith = (await probe('scan', ...everything, hex('#0978AB'), 3)).n;
    const borderBox = [home.x - 6, home.y - 14, 12, 28];
    const withBorder = await probe('hash', ...borderBox);
    t('politics: the stand-in draws country names for a runtime-2 pack', namesWith > 30, namesWith);
    await shot('modern');
    await ov(() => terra.globe.set({ historical: true })); await sync();
    const namesWithout = (await probe('scan', ...everything, NAME_INK, 4)).n, coastWithout = (await probe('scan', ...everything, hex('#0978AB'), 3)).n;
    const noBorder = await pg(([b]) => window.__t.scan(b[0], b[1], b[2], b[3], [0xF6, 0xE1, 0xB9], 10), [borderBox]);
    t('historical: country names are gone', namesWithout === 0, { namesWith, namesWithout });
    t('historical: the border (Egypt-Libya, 25 E) is gone: only land colour is left there', (await probe('hash', ...borderBox)) !== withBorder && noBorder.n === Math.ceil(12 * 2) * Math.ceil(28 * 2), noBorder);
    t('historical: coasts stay', coastWithout > 100 && coastWithout >= coastWith * 0.95, { coastWith, coastWithout });
    t('historical: the page asked, the harness reports it', (await pg(() => window.__playground.globe().historical)) === true);
    await shot('historical');
    await ov(() => terra.globe.set({})); await sync();
    t('historical: set({}) brings today\'s politics back', (await probe('hash', ...borderBox)) === withBorder && (await probe('scan', ...everything, NAME_INK, 4)).n === namesWith);
    await ov(() => terra.globe.set({ historical: true })); await sync();
    await page.click('#restart'); await page.waitForFunction(() => window.__playground.ready.overlay); await sync();
    t('historical: ends with the prototype (a restart brings politics back)', (await pg(() => window.__playground.globe().historical)) === false && (await probe('hash', ...borderBox)) === withBorder);
    await reset();
  }

  // ---- everything on one globe, for the eye
  {
    await reset();
    await ov(() => {
      const f = (geometry, properties) => ({ type: 'Feature', geometry, properties: properties || {} });
      const p = (lon, lat) => ({ type: 'Point', coordinates: [lon, lat] });
      const r = (a, b, c, d) => [[a, d], [c, d], [c, b], [a, b], [a, d]];
      terra.geo.set('ghost-sea', [f({ type: 'Polygon', coordinates: [r(22, 30, 29, 33)] })], { style: 'ghost' });
      terra.geo.set('area', [f({ type: 'Polygon', coordinates: [r(22, 37, 29, 41)] })], { style: 'area' });
      terra.geo.set('lake', [f({ type: 'Polygon', coordinates: [r(31, 35, 38, 41), r(33, 37, 36, 39)] })], { style: 'area', color: '#1F7A8C' });
      terra.geo.set('border', [f({ type: 'LineString', coordinates: [[31, 33], [34, 32], [38, 33]] })], { style: 'border' });
      terra.geo.set('track', [f({ type: 'LineString', coordinates: [[24, 26], [30, 28], [36, 27]] })], { style: 'track' });
      terra.geo.set('dots', [p(26, 43), p(28, 44)], { style: 'dot' });
      terra.geo.set('hot', [p(30, 43)], { style: 'dot', color: '#B04234' });
      terra.geo.set('rings', [p(33, 43), p(35, 44)], { style: 'ring' });
      terra.geo.set('names', [f(p(25.2, 39), { label: 'Ephesus' }), f(p(34.5, 40), { label: 'Lake' })], { style: 'label' });
      terra.geo.set('sea', [f(p(24.5, 31.5), { label: 'Old shoreline' })], { style: 'label', font: 'italic' });
      terra.geo.set('region', [f(p(30, 33), { label: 'Mediterranean' })], { style: 'label', font: 'caps', size: 13 });
      terra.geo.set('ship', [p(34, 30)], { style: 'symbol', image: 'symbol.png', width: 28, height: 28 });
      terra.geo.set('depth', [f(p(28, 28), { length: 70, label: '40 fathoms' })], { style: 'sounding' });
    });
    await sync();
    await shot('geo');
    await ov(() => { terra.caption.set({ title: 'Evidence', text: 'A coast as it was drawn then, with the coast of today dashed under it.' }); terra.time.show({ unit: 'year', min: -500, max: 1500, value: 640, ticks: [-300, 0, { at: 640, hollow: true }, 1000], playable: true, rate: 100 }); });
    await sync();
    await shot('scene');
    await reset();
  }

  // ---- taps still go to the page
  {
    await reset();
    await page.mouse.click(stage.x + W / 2, stage.y + H / 2); await sleep(150);
    const taps = await ov(() => window.taps.slice());
    t('a tap on the globe still goes to the page', taps.length === 1 && near(taps[0].lon, 30, 0.5) && near(taps[0].lat, 36, 0.5), taps);
  }

  t('nothing left the machine', requests.every((u) => { try { return LOCAL.has(new URL(u).hostname) || /^(data|blob):/.test(u); } catch (e) { return true; } }), requests.filter((u) => /^BLOCKED/.test(u)));
  t('the globe\'s fonts came from the harness', ['Regular', 'Italic', 'Semibold'].every((f) => requests.some((u) => u.endsWith('/_harness/fonts/LibertinusSerif-' + f + '.otf'))));
  const errs = await pg(() => window.__playground.log.filter((l) => l.level === 'error').map((l) => l.source + ': ' + l.message));
  for (const p of problems) check(tag('no page problem: ' + p), false);
  t('no error reached the harness log', errs.length === 0, errs);
  await context.close();
}

async function runIdle(browser, server) {
  // The idle turn belongs to a pack that does not ask for stillness; ?idleAfter=1 shortens the 8 s wait.
  const context = await browser.newContext({ viewport: { width: 1100, height: 1100 }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const problems = [];
  page.on('pageerror', (e) => problems.push(e.message));
  await page.goto(`${server.url}/preview/${PACK}?device=phone&size=402x874&idleAfter=1`);
  await page.waitForFunction(() => window.__playground && window.__playground.ready.overlay, null, { timeout: 8000 });
  const lon = () => page.evaluate(() => window.__playground.camera().lon);
  const overlay = () => page.frames().find((f) => f.url().includes('mode=overlay'));
  const l0 = await lon();
  await page.waitForFunction((l) => window.__playground.camera().lon < l - 1.5, l0, { timeout: 8000 }).then(() => check('[idle] an untouched globe turns west to east', true), () => check('[idle] an untouched globe turns west to east', false, { l0, now: 'unchanged' }));
  await overlay().evaluate(() => terra.globe.set({ still: true }));
  await sleep(300);
  const s0 = await lon(); await sleep(1200); const s1 = await lon();
  check('[idle] globe.set({ still: true }) stops the turn', s0 === s1, { s0, s1 });
  await overlay().evaluate(() => terra.globe.set({}));
  await sleep(300);
  const r0 = await lon(); await sleep(800);
  check('[idle] globe.set({}) lets it turn again', (await lon()) < r0 - 1, { r0, now: await lon() });
  await page.evaluate(() => window.__playground.setSettings({ reduceMotion: true }));
  await sleep(300);
  const m0 = await lon(); await sleep(1200); const m1 = await lon();
  check('[idle] Reduce Motion stops it', m0 === m1, { m0, m1 });
  await page.evaluate(() => window.__playground.setSettings({ reduceMotion: false }));
  await overlay().evaluate(() => terra.globe.set({ still: true }));
  await sleep(200);
  await page.click('#restart');
  await page.waitForFunction(() => window.__playground.ready.overlay);
  check('[idle] still ends with the prototype', (await page.evaluate(() => window.__playground.globe().still)) === false);
  const n0 = await lon();
  await page.waitForFunction((l) => window.__playground.camera().lon < l - 1, n0, { timeout: 5000 }).then(() => check('[idle] ...and the globe turns again after a restart', true), () => check('[idle] ...and the globe turns again after a restart', false));
  // a touch holds it still for the wait
  const box = await page.locator('#stage').boundingBox();
  await page.mouse.click(box.x + 200, box.y + 300);
  const t0 = await lon(); await sleep(600); const t1 = await lon();
  check('[idle] a touch holds the turn off for the wait', t0 === t1, { t0, t1 });
  for (const p of problems) check('[idle] no page problem: ' + p, false);
  await context.close();
}

(async () => {
  const args = process.argv.slice(2), di = args.indexOf('--device');
  const only = di >= 0 ? args[di + 1] : null;
  if (only && !DEVICES[only]) { console.error('usage: node tools/playground/harness_check.cjs [--device phone|ipad]'); process.exit(2); }
  writeFixture();
  fs.mkdirSync(OUT, { recursive: true });
  for (const f of fs.readdirSync(OUT)) if (f.endsWith('.png') && (!only || f.startsWith(only + '-'))) fs.rmSync(path.join(OUT, f));
  const server = await start({ port: 0, prototypes: FIXTURE });
  const browser = await chromium.launch(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {});
  const shots = [];
  try {
    for (const name of Object.keys(DEVICES)) if (!only || only === name) await runDevice(browser, server, name, shots);
    if (!only || only === 'phone') await runIdle(browser, server);
  } catch (e) {
    check('the check ran to the end', false, e && e.stack || String(e));
  } finally {
    await browser.close();
    await server.close();
  }
  const failed = results.filter((r) => !r.ok);
  fs.writeFileSync(path.join(OUT, 'report.json'), JSON.stringify({ passed: failed.length === 0, checks: results.length, failed: failed.length, results, screenshots: shots }, null, 2) + '\n');
  console.log(`${failed.length ? 'FAIL' : 'PASS'}: ${results.length - failed.length}/${results.length} checks; screenshots in ${path.relative(ROOT, OUT)}/ (${shots.length})`);
  process.exit(failed.length ? 1 : 0);
})();
