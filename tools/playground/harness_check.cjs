'use strict';
// The desktop preview's own check: it writes a runtime-6 pack that uses every tool once (build/harness-fixture/
// preview-fixture/, never in Prototypes/), runs it in headless Chromium at iPhone and iPad sizes, and holds the
// preview to docs/runtime3.md: the doorway starts it, the anchor flight lands, the script's world has no DOM and
// reads its files, geography, the picture and sprites are drawn where terra.project puts them (read off the
// canvas), sprites move and change look with the zoom, touch reaches the script (taps with targets, drags that
// begin on a touchable thing, the one-finger scheme), the camera is told at most 20 times a second and once when it
// stops, the date strip and play, the panel and its closing, Open, and the end leaves the globe as it was.
// Screenshots go to build/harness-check/<device>-*.png and the verdicts to build/harness-check/report.json.
//   node tools/playground/harness_check.cjs [--device iphone|ipad]       (default: both)
//   node tools/playground/harness_check.cjs --write [dir]                (only write the fixture pack; default
//                                                                         build/harness-fixture: then
//                                                                         node tools/playground/check.cjs preview-fixture --root build/harness-fixture)
const fs = require('node:fs'), path = require('node:path'), zlib = require('node:zlib');

const ROOT = path.resolve(__dirname, '../..');
const FIXTURE = path.join(ROOT, 'build', 'harness-fixture');
const OUT = path.join(ROOT, 'build', 'harness-check');
const PACK = 'preview-fixture';

// ---- pictures, drawn here: shapes with 4 x 4 samples a pixel, written as PNG
function crc32(buf) {
  let c, crc = ~0;
  for (let n = 0; n < buf.length; n++) {
    c = (crc ^ buf[n]) & 0xff;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    crc = (crc >>> 8) ^ c;
  }
  return ~crc >>> 0;
}
function encodePng(w, h, rgba) {
  const raw = Buffer.alloc((1 + 4 * w) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (1 + 4 * w)] = 0;
    for (let i = 0; i < 4 * w; i++) raw[y * (1 + 4 * w) + 1 + i] = Math.round(Math.max(0, Math.min(255, rgba[y * 4 * w + i])));
  }
  const chunk = (type, data) => {
    const t = Buffer.concat([Buffer.from(type), data]), len = Buffer.alloc(4), crc = Buffer.alloc(4);
    len.writeUInt32BE(data.length); crc.writeUInt32BE(crc32(t));
    return Buffer.concat([len, t, crc]);
  };
  const head = Buffer.alloc(13);
  head.writeUInt32BE(w, 0); head.writeUInt32BE(h, 4); head.set([8, 6, 0, 0, 0], 8);
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', head), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}
// a picture: layers of [inside(x, y), [r, g, b, a]] composited in order; x, y in pixels
function paint(w, h, layers) {
  const out = new Float64Array(w * h * 4);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    let r = 0, g = 0, b = 0, a = 0;
    for (const [inside, colour] of layers) {
      let cover = 0;
      for (let j = 0; j < 4; j++) for (let i = 0; i < 4; i++) if (inside(x + (i + 0.5) / 4, y + (j + 0.5) / 4)) cover++;
      if (!cover) continue;
      const c = typeof colour === 'function' ? colour(x, y) : colour, A = (c[3] == null ? 1 : c[3]) * cover / 16;
      const oA = A + a * (1 - A);
      r = (c[0] * A + r * a * (1 - A)) / oA; g = (c[1] * A + g * a * (1 - A)) / oA; b = (c[2] * A + b * a * (1 - A)) / oA; a = oA;
    }
    out.set([r, g, b, a * 255], (y * w + x) * 4);
  }
  return encodePng(w, h, out);
}
const disc = (cx, cy, r) => (x, y) => (x - cx) ** 2 + (y - cy) ** 2 <= r * r;
const annulus = (cx, cy, r0, r1) => (x, y) => { const d = (x - cx) ** 2 + (y - cy) ** 2; return d >= r0 * r0 && d <= r1 * r1; };
function polygon(pts) {
  return (x, y) => {
    let inside = false;
    for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
      const [xi, yi] = pts[i], [xj, yj] = pts[j];
      if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
    }
    return inside;
  };
}
const all = () => true;

// The fixture's colours, known to the check: what it looks for on the canvas
const CHART = { w: 240, h: 180 };
const chartColour = (x, y) => [214 - (y / CHART.h) * 40, 168 + (x / CHART.w) * 50, 112 + (x / CHART.w) * 90, 1];
const BOAT_HULL = [92, 52, 30], DOT = [24, 44, 96], DOOR = [178, 74, 46], MARK = [46, 139, 87], SPLASH = [30, 110, 190];

function pictures() {
  const boat = (sail, dx = 0) => {
    const at = (pts) => polygon(pts.map(([x, y]) => [x + dx, y]));
    return [[at([[6, 26], [70, 26], [86, 18], [80, 34], [14, 36]]), BOAT_HULL],      // the bow to the east (right)
            [at([[40, 4], [40, 24], [66, 24]]), sail],
            [at([[38, 2], [42, 2], [42, 26], [38, 26]]), [60, 40, 24]]];
  };
  return {
    'chart.png': paint(CHART.w, CHART.h, [
      [all, chartColour],
      [(x, y) => x % 30 < 2 || y % 30 < 2, [96, 70, 44, 0.9]],
      [(x, y) => Math.abs(x - y * 1.33) < 6, [40, 90, 160, 0.8]],
      [(x, y) => x < 5 || y < 5 || x > CHART.w - 5 || y > CHART.h - 5, [150, 52, 36]],
    ]),
    'dot.png': paint(24, 24, [[disc(12, 12, 11), [255, 255, 255]], [disc(12, 12, 8.5), DOT]]),
    // a sprite sheet (runtime 6): the boat's two frames side by side, two pixels apart (BOAT_FRAMES)
    'boats.png': paint(178, 44, [...boat([250, 248, 240]), ...boat([236, 214, 170], 90)]),
    'splash.png': paint(40, 40, [[annulus(20, 20, 9, 14), SPLASH.concat(0.9)], [disc(20, 20, 4), SPLASH]]),
    'door.png': paint(52, 52, [[disc(26, 26, 25), [255, 250, 240]], [disc(26, 26, 22), DOOR], [polygon([[17, 40], [17, 22], [26, 13], [35, 22], [35, 40]]), [246, 236, 214]]]),
    'mark.png': paint(36, 36, [[polygon([[18, 2], [34, 18], [18, 34], [2, 18]]), MARK]]),
  };
}

// ---- the pack: invented shapes on the Mediterranean (places named for where they are; nothing else is a fact).
// At zoom 3 a degree is about 9 points on an iPhone, so the shapes are a few degrees across.
const FIX = {
  anchor: { lon: 21, lat: 35.5, zoom: 3 },
  doorway: { lon: 22, lat: 36.5, minZoom: 1.5 },
  area: [[16.5, 35.0], [20.5, 34.6], [21.5, 37.0], [18.0, 38.0], [16.5, 35.0]],
  route: [[23.64, 37.94], [25.4, 36.4], [25.13, 35.34], [29.92, 31.2]],
  outline: [[31.5, 34.2], [35.0, 34.2], [35.0, 36.0], [31.5, 36.0], [31.5, 34.2]],
  ports: [['Piraeus', 23.64, 37.94], ['Heraklion', 25.13, 35.34], ['Alexandria', 29.92, 31.2], ['Valletta', 14.51, 35.9]],
  marks: [[19.0, 33.0], [27.5, 34.0]],
  sounding: [17.0, 33.4],
  chart: [8.5, 37.5, 15.5, 42.5],
  splash: [28.5, 33.2],
  boatClose: 4.5,
  voyage: { coordinates: [[14.51, 35.9], [20.0, 35.0], [25.13, 35.34], [29.92, 31.2]], seconds: 30, loop: 'back' },
};

function mainJs() {
  return `/* The preview's fixture: every tool once (its boat's frames a sprite sheet: runtime 6), and each event echoed to the log for the check. */
(function () {
  'use strict';
  var F = window.FIXTURE, scheme = 'touchables';
  function echo(kind, value) { terra.log('echo ' + kind + ' ' + JSON.stringify(value)); }
  echo('world', { document: typeof document, fetch: typeof fetch, XMLHttpRequest: typeof XMLHttpRequest, navigator: typeof navigator,
                  windowIsGlobal: typeof window === 'object' && window === globalThis, runtime: terra.runtime, pack: terra.pack,
                  setTimeout: typeof setTimeout, camera: !!terra.camera });
  echo('read', { notes: terra.read('notes.txt'), facts: terra.readJSON('data/facts.json'), missing: terra.read('nope.txt'),
                 escape: terra.read('../prototype.json') });

  function feature(geometry, properties) { return { type: 'Feature', geometry: geometry, properties: properties || {} }; }
  terra.images.set('chart', { image: 'chart.png', bounds: F.chart, opacity: 0.9, minZoom: 2.2, fade: 0.4 });
  terra.geo.set('sea', [feature({ type: 'Polygon', coordinates: [F.area] }, { name: 'A shaded sea' })], { style: 'area', color: '#2E7DAF', touchable: true });
  terra.geo.set('route', { type: 'LineString', coordinates: F.route }, { style: 'track' });
  terra.geo.set('outline', [{ type: 'Polygon', coordinates: [F.outline] }], { style: 'ghost', color: '#7A3B2E' });
  var ports = F.ports.map(function (p) { return feature({ type: 'Point', coordinates: [p[1], p[2]] }, { label: p[0] }); });
  terra.geo.set('ports', ports, { style: 'ring', touchable: true });
  terra.geo.set('names', ports, { style: 'label', font: 'italic', size: 14, offset: [0, -14] });
  terra.geo.set('marks', { type: 'MultiPoint', coordinates: F.marks }, { style: 'symbol', image: 'mark.png', width: 16, height: 16 });
  terra.geo.set('depth', [feature({ type: 'Point', coordinates: F.sounding }, { length: 48, label: '48 points down' })], { style: 'sounding' });
  terra.sprites.set('boat', {
    path: F.voyage,
    looks: [{ image: 'dot.png', size: [12, 12], maxZoom: F.boatClose },
            { frames: ['boats.png#0,0,88,44', 'boats.png#90,0,88,44'], fps: 3, size: [44, 22], minZoom: F.boatClose, face: true }],
    label: 'A boat (the fixture\\'s)', touchable: true
  });
  terra.sprites.set('splash', { at: F.splash, looks: [{ image: 'splash.png', size: [20, 20], every: 2, for: 1.4 }] });
  terra.caption.set({ title: 'Preview fixture', text: 'Every runtime-3 tool once. Touch the boat, a port or the shaded sea.' });
  terra.time.show({ unit: 'year', min: 1800, max: 1900, value: 1850, ticks: [1810, 1830, { at: 1860, hollow: true }, 1890], playable: true, rate: 20 });
  terra.globe.set({ still: true });

  var panels = {
    sea: { title: 'A shaded sea', text: 'An area the fixture draws, touchable.\\n\\nIts second paragraph.' },
    boat: { title: 'A boat', text: 'A sprite on a great-circle path, to and fro; a dot from afar and a boat up close.' },
    ports: { title: 'A port', text: 'A ring the fixture draws, touchable.' }
  };
  function show(target) {
    var p = target.sprite ? panels.boat : panels[target.layer];
    if (!p) return;
    terra.panel.show({ title: p.title + (target.index != null ? ' (' + target.index + ')' : ''), text: p.text,
      sections: [{ title: 'What the preview shows', text: 'Geography, a picture, sprites, the date, this panel.' }],
      sources: [{ title: 'docs/runtime3.md', url: 'https://github.com/', note: 'the contract' }] });
    terra.caption.set({ title: 'Preview fixture', text: 'Touched ' + JSON.stringify(target) + '.' });
  }
  terra.on('touch', function (t) {
    echo('touch', t);
    if (t.phase === 'tap' && t.target) show(t.target);
    // a port flies the globe to it
    if (t.phase === 'tap' && t.target && t.target.layer === 'ports') { var p = F.ports[t.target.index]; terra.flyTo(p[1], p[2], 4, 0.4); }
  });
  terra.on('time', function (t) {
    echo('time', t);
    terra.globe.set({ historical: t.value >= 1890, still: true });      // the last decade: today's borders hidden
  });
  terra.on('camera', function (c) { echo('camera', { lon: +c.lon.toFixed(3), lat: +c.lat.toFixed(3), zoom: +c.zoom.toFixed(3) }); });
  terra.on('settings', function (s) { echo('settings', s); });
  terra.on('panel', function (p) { echo('panel', p); });
  terra.on('open', function (p) {
    scheme = scheme === 'one-finger' ? 'touchables' : 'one-finger';
    terra.touch.scheme(scheme);
    echo('open', { scheme: scheme });
  });
  terra.on('close', function () { echo('close', {}); });
  setTimeout(function () { echo('timeout', { ms: 120 }); }, 120);
})();
`;
}

function writeFixture(base) {
  const dir = path.join(base, PACK);
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(path.join(dir, 'data'), { recursive: true });
  const manifest = {
    id: PACK, title: 'Preview fixture', summary: 'The desktop preview\'s own test pack: every runtime-3 tool once.',
    runtime: 6, presentation: 'globe', scripts: ['data.js', 'main.js'], anchor: FIX.anchor,
    doorway: { lon: FIX.doorway.lon, lat: FIX.doorway.lat, label: 'Preview fixture', look: { image: 'door.png', size: [26, 26], minZoom: FIX.doorway.minZoom } },
    taps: true, shelf: 'current',
  };
  fs.writeFileSync(path.join(dir, 'prototype.json'), JSON.stringify(manifest, null, 2) + '\n');
  fs.writeFileSync(path.join(dir, 'data.js'), '/* the fixture\'s shapes (invented) */\nwindow.FIXTURE = ' + JSON.stringify({
    area: FIX.area, route: FIX.route, outline: FIX.outline, ports: FIX.ports, marks: FIX.marks, sounding: FIX.sounding,
    chart: FIX.chart, splash: FIX.splash, voyage: FIX.voyage, boatClose: FIX.boatClose }) + ';\n');
  fs.writeFileSync(path.join(dir, 'main.js'), mainJs());
  fs.writeFileSync(path.join(dir, 'notes.txt'), 'Read by terra.read.\n');
  fs.writeFileSync(path.join(dir, 'data', 'facts.json'), JSON.stringify({ read: 'by terra.readJSON', n: 3 }) + '\n');
  fs.writeFileSync(path.join(dir, 'SOURCES.md'), '# Sources\n\nEverything in this pack is invented for the preview\'s own check; the pictures are drawn by tools/playground/harness_check.cjs.\n');
  for (const [name, buf] of Object.entries(pictures())) fs.writeFileSync(path.join(dir, name), buf);
  return dir;
}

// A pack that reaches for the DOM, as a pack written for a web page would: the preview must say so, as the phone would.
function writeBroken(base) {
  const dir = path.join(base, 'preview-broken');
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'prototype.json'), JSON.stringify({ id: 'preview-broken', title: 'Preview broken', summary: 'Reaches for the DOM.',
    runtime: 3, presentation: 'globe', scripts: ['main.js', 'after.js'], shelf: 'current' }, null, 2) + '\n');
  fs.writeFileSync(path.join(dir, 'main.js'), '// a web page\'s habit\nterra.log("before");\ndocument.title = "nope";\n');
  fs.writeFileSync(path.join(dir, 'after.js'), 'terra.log("after: the next script still runs");\nterra.geo.set("x", [], { style: "squiggle" });\n');
  return dir;
}

module.exports = { writeFixture, FIX, PACK };

// ---- the check
const SIZES = { iphone: { width: 393, height: 852 }, ipad: { width: 1032, height: 1376 } };
const LOCAL = new Set(['127.0.0.1', 'localhost']);
const SEA = [221, 243, 253];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const blend = (base, rgb, a) => rgb.map((v, i) => base[i] * (1 - a) + v * a);
const near = (a, b, tol) => a.every((v, i) => Math.abs(v - b[i]) <= tol);
const close = (a, b, tol) => Math.abs(a - b) <= tol;
const unitV = (lon, lat) => { const lo = lon * Math.PI / 180, la = lat * Math.PI / 180; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; };
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const angle = (a, b) => Math.acos(Math.max(-1, Math.min(1, dot(a, b))));
// how far (degrees) a place is from a path of great circles
function offPath(lon, lat, coords) {
  const u = unitV(lon, lat);
  let best = Infinity;
  for (let i = 0; i + 1 < coords.length; i++) {
    const a = unitV(...coords[i]), b = unitV(...coords[i + 1]);
    const n = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]], nl = Math.hypot(...n);
    const nn = n.map((v) => v / nl), k = dot(u, nn), p = u.map((v, j) => v - k * nn[j]), pl = Math.hypot(...p), q = p.map((v) => v / pl);
    const along = Math.abs(angle(a, q) + angle(q, b) - angle(a, b)) < 1e-6;
    best = Math.min(best, along ? Math.asin(Math.abs(k)) : Math.min(angle(u, a), angle(u, b)));
  }
  return best * 180 / Math.PI;
}

async function runDevice(browser, server, device) {
  const results = [], nonLocal = [], pageErrors = [];
  const t = (name, ok, detail) => {
    results.push({ device, name, ok: !!ok, detail: ok ? undefined : detail });
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${device}: ${name}${ok || detail === undefined ? '' : '  ' + JSON.stringify(detail)}`);
  };
  const context = await browser.newContext({ viewport: SIZES[device], deviceScaleFactor: 2 });
  await context.route((url) => !LOCAL.has(url.hostname) && !['data:', 'blob:', 'about:'].includes(url.protocol), (route) => { nonLocal.push(route.request().url()); route.abort(); });
  const page = await context.newPage();
  page.on('pageerror', (e) => pageErrors.push(e.message));
  page.on('console', (m) => { if (m.type() === 'error') pageErrors.push(m.text()); });
  const pg = (fn, arg) => page.evaluate(fn, arg);
  const echoes = (kind, since) => pg(([k, s]) => window.__playground.log
    .filter((l) => l.source === 'script' && l.t >= s && l.message.startsWith('echo ' + k + ' '))
    .map((l) => ({ t: l.t, v: JSON.parse(l.message.slice(6 + k.length)) })), [kind, since || 0]);
  const clock = () => pg(() => window.__playground.clock());
  // until the script has echoed `n` of a kind since then (it may come late while the main thread draws)
  const until = (kind, since, n = 1, ms = 4000) => page.waitForFunction(([k, s0, n0]) => window.__playground.log
    .filter((l) => l.source === 'script' && l.t >= s0 && l.message.startsWith('echo ' + k + ' ')).length >= n0, [kind, since, n], { timeout: ms }).catch(() => {});
  // the echoes of what came before, all in (an echo of something sent after them comes after it)
  const quiet = async () => { const s0 = await clock(); await pg(() => window.__playground.setSettings({})); await until('settings', s0); };
  const pixel = (x, y) => pg(([x0, y0]) => {
    const cv = document.getElementById('globe'), k = window.devicePixelRatio || 1;
    const d = cv.getContext('2d').getImageData(Math.round(x0 * k) - 1, Math.round(y0 * k) - 1, 3, 3).data, s = [0, 0, 0];
    for (let i = 0; i < 9; i++) for (let j = 0; j < 3; j++) s[j] += d[i * 4 + j];
    return s.map((v) => Math.round(v / 9));
  }, [x, y]);
  const project = (lon, lat) => pg(([a, b]) => window.__playground.project(a, b), [lon, lat]);
  const camera = () => pg(() => window.__playground.camera());
  const state = () => pg(() => window.__playground.state());
  const shot = (name) => page.screenshot({ path: path.join(OUT, `${device}-${name}.png`) });
  const rest = async () => {          // the camera still, everything loaded, drawn again at rest
    await page.waitForFunction(() => window.__playground.settled(), null, { timeout: 8000 });
    await sleep(260);
    const f = await pg(() => { window.__playground.redraw(); return window.__playground.frame; });
    await page.waitForFunction((f0) => window.__playground.frame >= f0 + 2, f, { timeout: 4000 });
  };
  const anchor = () => pg((a) => window.__playground.flyTo(a.lon, a.lat, a.zoom, 0), FIX.anchor).then(rest);
  const drag = async (from, to, opts = {}) => {
    if (opts.shift) await page.keyboard.down('Shift');
    await page.mouse.move(from[0], from[1]);
    await page.mouse.down();
    const steps = opts.steps || 8;
    for (let i = 1; i <= steps; i++) {
      await page.mouse.move(from[0] + (to[0] - from[0]) * i / steps, from[1] + (to[1] - from[1]) * i / steps);
      if (opts.pause) await sleep(opts.pause);
    }
    await page.mouse.up();
    if (opts.shift) await page.keyboard.up('Shift');
    await sleep(120);
  };

  try {
    await page.goto(`${server.url}/preview/${PACK}?device=${device}&bare=1&idleAfter=0`);
    await page.waitForFunction(() => window.__playground && window.__playground.ready, null, { timeout: 15000 });
    await page.waitForFunction(() => window.__playground.settled() && window.__playground.doorway().visible, null, { timeout: 8000 });

    // ---- before it runs: the doorway on a turning globe
    const door = await pg(() => {
      const d = window.__playground.doorway(), cv = document.getElementById('globe'), k = window.devicePixelRatio || 1;
      // (left of its middle: the terracotta ring, outside the door's arch)
      const at = (x, y) => Array.from(cv.getContext('2d').getImageData(Math.round(x * k), Math.round(y * k), 1, 1).data).slice(0, 3);
      return Object.assign(d, { ring: at(d.x - 10 * window.__playground.reading(), d.y), camera: window.__playground.camera() });
    });
    t('doorway: drawn before the pack starts, at its place', door.visible && near(door.ring, DOOR, 30), door);
    t('doorway: the camera starts on it, close enough for its look (minZoom 1.5)', close(door.camera.zoom, FIX.doorway.minZoom + 0.25, 1e-6), door.camera);
    await shot('doorway');
    const lon0 = (await camera()).lon;
    await sleep(500);
    const lon1 = (await camera()).lon;
    t('the globe turns west to east when left alone (4° a second)', close(((lon0 - lon1) + 540) % 360 - 180, 2, 0.8), { lon0, lon1 });

    // ---- touching the doorway starts it where the reader is (runtime 4: no flight to the anchor), and
    // globe.set({ still }) stops the turn; then the anchor, as a check flies there
    const d2 = await pg(() => window.__playground.doorway());
    const cam0 = await camera();
    await page.mouse.click(d2.x, d2.y);
    await page.waitForFunction(() => window.__playground.started, null, { timeout: 8000 });
    await rest();
    const cam1 = await camera();
    const entry = (await pg(() => window.__playground.state())).entry;
    // (the globe was still turning, 4° a second, until the script asked it to stop: a few degrees at most)
    t('start by the doorway: the globe stays where the reader is (terra.entry says "doorway"), and globe.set({ still }) stops the turn',
      entry === 'doorway' && Math.abs(((cam1.lon - cam0.lon) + 540) % 360 - 180) < 4 && close(cam1.lat, cam0.lat, 1e-6) && close(cam1.zoom, cam0.zoom, 0.001), { cam0, cam1, entry });
    await anchor();
    const cam = await camera();
    t('flyTo: the flight lands on the anchor', close(cam.lon, FIX.anchor.lon, 0.01) && close(cam.lat, FIX.anchor.lat, 0.01) && close(cam.zoom, FIX.anchor.zoom, 0.001), cam);
    const chip = await pg(() => ({ chip: !document.getElementById('chip').hidden, open: !document.getElementById('chip-open').hidden, start: !document.getElementById('start').hidden }));
    t('the chip shows the pack, with Open once it listens for "open"', chip.chip && chip.open && !chip.start, chip);

    // ---- the script's world
    const world = (await echoes('world'))[0];
    const w = world && world.v;
    t('script world: no DOM, no fetch, no XMLHttpRequest, no navigator', w && w.document === 'undefined' && w.fetch === 'undefined' && w.XMLHttpRequest === 'undefined' && w.navigator === 'undefined', w);
    t('script world: window is the global, runtime 6, pack origin "preview", timers, terra.camera', w && w.windowIsGlobal && w.runtime === 6 && w.pack.origin === 'preview' && w.pack.id === PACK && w.setTimeout === 'function' && w.camera, w);
    const read = (await echoes('read'))[0];
    const r = read && read.v;
    t('terra.read / readJSON: the pack\'s files; null for a missing one and outside the pack', r && r.notes === 'Read by terra.read.\n' && r.facts && r.facts.n === 3 && r.missing === null && r.escape === null, r);
    t('setTimeout runs', (await echoes('timeout')).length === 1);
    const st0 = await echoes('settings');
    t('settings: heard at start', st0.length >= 1 && st0[st0.length - 1].v.textScale === 1, st0);
    const time0 = await echoes('time');
    t('time.show: the first value told by the host', time0.length >= 1 && time0[0].v.by === 'host' && time0[0].v.value === 1850, time0);

    // ---- drawn where terra.project puts it
    const sea = await project(19.5, 37.0);
    const seaPx = await pixel(sea.x, sea.y);
    t('geo area: its fill over the sea (#2E7DAF at 0.28)', near(seaPx, blend(SEA, [46, 125, 175], 0.28), 14), seaPx);
    const lon = 8.5 + 135 / 240 * 7, lat = 42.5 - 75 / 180 * 5, chartAt = await project(lon, lat), chartPx = await pixel(chartAt.x, chartAt.y);
    const want = blend(SEA, [214 - 75 / 180 * 40, 168 + 135 / 240 * 50, 112 + 135 / 240 * 90], 0.9);
    t('picture: lies on the globe by its bounds (a known pixel of it, at 0.9 opacity)', near(chartPx, want, 12), { chartPx, want });
    const outside = await project(8.0, 40.0), outPx = await pixel(outside.x, outside.y);
    t('picture: nothing drawn outside its bounds', near(outPx, SEA, 14) || outPx[0] > 230, outPx);
    let s = await state();
    const boat = s.sprites.find((x) => x.id === 'boat');
    const boatPx = boat && boat.drawn ? await pixel(boat.drawn.x, boat.drawn.y) : null;
    t('sprite: the far look (a dot) where the sprite is, at zoom 3', boat && boat.drawn && boat.drawn.look === 0 && near(boatPx, DOT, 40), { boat, boatPx });
    const ll0 = boat && await pg(([x, y]) => window.__playground.unproject(x, y), [boat.drawn.x, boat.drawn.y]);
    t('sprite: on its path of great circles', ll0 && offPath(ll0.lon, ll0.lat, FIX.voyage.coordinates) < 0.15, ll0 && offPath(ll0.lon, ll0.lat, FIX.voyage.coordinates));
    await sleep(1000);
    s = await state();
    const boat2 = s.sprites.find((x) => x.id === 'boat');
    t('sprite: moves along it', boat2.drawn && Math.hypot(boat2.drawn.x - boat.drawn.x, boat2.drawn.y - boat.drawn.y) > 2, [boat.drawn, boat2.drawn]);
    await shot('anchor');
    const ll1 = await pg(([x, y]) => window.__playground.unproject(x, y), [boat2.drawn.x, boat2.drawn.y]);
    await pg((a) => window.__playground.flyTo(a.lon, a.lat, 5, 0), ll1);
    await rest();
    s = await state();
    const close5 = s.sprites.find((x) => x.id === 'boat');
    t('sprite: the close look (the boat, its frames) at zoom 5, facing its way', close5.drawn && close5.drawn.look === 1 && close5.drawn.flipped === false, close5);
    const hull = close5.drawn && await pixel(close5.drawn.x, close5.drawn.y + 4 * s.reading);
    t('sprite: its picture drawn at the reading scale', hull && near(hull, BOAT_HULL, 50), hull);
    await shot('zoom-5');
    await anchor();

    // ---- touch
    let since = await clock();
    await page.mouse.click(sea.x, sea.y);
    await until('touch', since);
    await sleep(100);
    let touches = await echoes('touch', since);
    t('tap on a touchable area: target { layer, index }', touches.length === 1 && touches[0].v.phase === 'tap' && touches[0].v.target && touches[0].v.target.layer === 'sea' && touches[0].v.target.index === 0
      && close(touches[0].v.lon, 19.5, 0.3) && close(touches[0].v.lat, 37.0, 0.3), touches);
    s = await state();
    const panel = await pg(() => ({ shown: !document.getElementById('panel').hidden, title: document.querySelector('#panel h2').textContent,
      paragraphs: document.querySelectorAll('#panel p').length, link: (document.querySelector('#panel a') || {}).href }));
    t('panel: shown with its title, paragraphs, a section and sources as links', panel.shown && panel.title === 'A shaded sea (0)' && panel.paragraphs === 3 && panel.link === 'https://github.com/', panel);
    await shot('panel');
    since = await clock();
    await page.click('#panel-close');
    await until('panel', since);
    const closed = await echoes('panel', since);
    t('panel: closing it tells the script { open: false }', closed.length === 1 && closed[0].v.open === false && (await state()).panel === null, closed);

    const port = await project(23.64, 37.94), camBefore = await camera();
    since = await clock();
    await drag([port.x, port.y], [port.x + 50, port.y + 30]);
    await quiet();
    touches = await echoes('touch', since);
    const phases = touches.map((x) => x.v.phase);
    t('a drag that begins on a touchable ring is the pack\'s: began, moved…, ended, with its target',
      phases[0] === 'began' && phases[phases.length - 1] === 'ended' && phases.includes('moved') && touches.every((x) => x.v.target && x.v.target.layer === 'ports' && x.v.target.index === 0), phases);
    const camAfter = await camera();
    t('… and the globe stays put', close(camAfter.lon, camBefore.lon, 1e-9) && close(camAfter.lat, camBefore.lat, 1e-9), [camBefore, camAfter]);

    const empty = await project(5.0, 38.5);
    since = await clock();
    await drag([empty.x, empty.y], [empty.x + 40, empty.y]);
    await quiet();
    t('any other drag turns the globe, and the pack hears nothing', (await camera()).lon !== camAfter.lon && (await echoes('touch', since)).length === 0);
    await anchor();

    const landAt = await project(25, 45.5);
    since = await clock();
    await page.mouse.click(landAt.x, landAt.y);
    await until('touch', since);
    await quiet();
    touches = await echoes('touch', since);
    t('with "taps": true, a tap on nothing touchable reaches the pack with target null and its place', touches.length === 1 && touches[0].v.target === null
      && close(touches[0].v.lon, 25, 0.3) && close(touches[0].v.lat, 45.5, 0.3), touches);

    since = await clock();
    await page.click('#chip-open');
    await until('open', since);
    const opened = await echoes('open', since);
    t('Open tells the script "open" (the fixture then asks for the one-finger scheme)', opened.length === 1 && opened[0].v.scheme === 'one-finger' && (await state()).scheme === 'one-finger', opened);
    since = await clock();
    const camOF = await camera();
    await drag([empty.x, empty.y], [empty.x + 40, empty.y + 20]);
    await quiet();
    touches = await echoes('touch', since);
    t('one-finger: a drag on nothing touchable is the pack\'s (target null), the globe stays', touches.length >= 3 && touches[0].v.phase === 'began' && touches[0].v.target === null
      && close((await camera()).lon, camOF.lon, 1e-9), touches.map((x) => x.v.phase));
    since = await clock();
    await drag([empty.x, empty.y], [empty.x + 40, empty.y], { shift: true });
    await quiet();
    t('one-finger: shift-drag turns the globe', (await camera()).lon !== camOF.lon && (await echoes('touch', since)).length === 0);
    await page.click('#chip-open');
    await anchor();

    // ---- the camera told at most 20 times a second while it moves, and once when it stops
    await page.mouse.move(empty.x, empty.y);
    await page.mouse.down();
    since = await clock();
    for (let i = 1; i <= 60; i++) { await page.mouse.move(empty.x + 2 * i, empty.y + i / 6); await sleep(16); }
    await page.mouse.up();
    await sleep(450);
    await quiet();
    // (the host's own times: the echoes come back late while the main thread draws)
    const sent = await pg((s0) => ({ times: window.__playground.cameraSends.filter((x) => x >= s0), lastMove: window.__playground.lastMove() }), since);
    const moving = sent.times.filter((x) => x <= sent.lastMove + 0.001), span = Math.max(sent.lastMove - since, 0.001);
    const gaps = moving.slice(1).map((x, i) => x - moving[i]);
    t('camera events: at most 20 a second while it moves', moving.length >= 5 && Math.min(...gaps) >= 0.0495 && moving.length / span <= 21,
      { n: moving.length, seconds: +span.toFixed(2), minGap: Math.min(...gaps) });
    const stops = sent.times.filter((x) => x > sent.lastMove + 0.001), cams = await echoes('camera', since), last = cams[cams.length - 1], camNow = await camera();
    // (before it, the last move's own report may still be on its way: at most one, behind the throttle's 50 ms and a busy frame)
    const stop = stops[stops.length - 1], trailing = stops.slice(0, -1);
    t('camera events: one more when it stops (0.16 s after), with the camera at rest', stop != null && stop - sent.lastMove >= 0.155 && stop - sent.lastMove < 0.3
      && trailing.length <= 1
      && last && close(last.v.lon, camNow.lon, 0.002) && close(last.v.lat, camNow.lat, 0.002), { stops, lastMove: sent.lastMove, last, camNow });
    await anchor();

    // ---- the date
    const strip = await page.locator('#date-strip').boundingBox();
    since = await clock();
    await page.mouse.click(strip.x + strip.width * 0.25, strip.y + strip.height / 2);
    await until('time', since);
    let times = await echoes('time', since);
    t('date strip: a touch along it sets the date (by "reader")', times.length >= 1 && times[0].v.by === 'reader' && close(times[0].v.value, 1825, 0.6), times);
    since = await clock();
    await page.keyboard.press('ArrowRight');
    await until('time', since);
    await quiet();
    times = await echoes('time', since);
    t('date strip: the arrow keys step to the next tick', times.length === 1 && times[0].v.value === 1830, times);
    since = await clock();
    await page.click('#date-play');
    await sleep(600);
    await page.click('#date-play');
    await quiet();
    times = await echoes('time', since);
    const plays = times.filter((x) => x.v.by === 'play');
    t('date: play plays (by "play", rising, at most 30 a second) and pauses', times[0] && times[0].v.playing === true && plays.length >= 3 && plays.length <= 22
      && plays.every((x, i) => i === 0 || x.v.value > plays[i - 1].v.value) && times[times.length - 1].v.playing === false, { n: plays.length, first: times[0], last: times[times.length - 1] });
    const foot = await pg(() => ({ eyebrow: document.getElementById('foot-eyebrow').textContent, title: document.getElementById('foot-title').textContent }));
    t('the foot: the caption\'s title above the date', foot.eyebrow === 'Preview fixture' && /^18\d\d$/.test(foot.title), foot);
    since = await clock();
    await page.focus('#date-strip');
    await page.keyboard.press('End');
    await until('time', since);
    await quiet();
    await rest();
    const hist = await state();
    await shot('historical');
    since = await clock();
    await page.keyboard.press('Home');
    await until('time', since);
    await quiet();
    t('globe.set({ historical }): today\'s borders and names hidden, and back', hist.globe.historical === true && hist.time.value === 1900 && (await state()).globe.historical === false, hist.globe);

    // ---- a script's own flight
    const piraeus = await project(23.64, 37.94);
    since = await clock();
    await page.mouse.click(piraeus.x, piraeus.y);
    await until('touch', since);
    await page.waitForFunction(() => !window.__playground.flying() && window.__playground.camera().zoom > 3.9, null, { timeout: 4000 }).catch(() => {});
    const flown = await camera();
    t('terra.flyTo: the globe flies where the script says (a port, zoom 4)', close(flown.lon, 23.64, 0.01) && close(flown.lat, 37.94, 0.01) && close(flown.zoom, 4, 0.001), flown);
    await page.click('#panel-close');
    await anchor();

    // ---- settings
    since = await clock();
    await pg(() => window.__playground.setSettings({ textScale: 1.5 }));
    await until('settings', since);
    const st = await echoes('settings', since);
    t('settings: a change is told to the script', st.length === 1 && st[0].v.textScale === 1.5, st);
    await pg(() => window.__playground.setSettings({ textScale: 1 }));

    // ---- the end
    since = await clock();
    await page.click('#chip-close');
    await until('close', since);
    await sleep(200);
    s = await state();
    const after = await pg(() => ({ foot: document.getElementById('foot').hidden, panel: document.getElementById('panel').hidden, chip: document.getElementById('chip').hidden,
      start: document.getElementById('start-button').textContent, door: window.__playground.doorway().visible }));
    t('✕ ends it: the script hears "close"', (await echoes('close', since)).length === 1);
    t('… and the globe is as it was: no layers, pictures, sprites, foot, panel; the doorway back', !s.running && !s.geo.length && !s.images.length && !s.sprites.length
      && after.foot && after.panel && after.chip && after.start === 'Start again' && after.door, { s: { running: s.running, geo: s.geo.length }, after });
    await rest();
    await shot('ended');

    const errors = (await pg(() => window.__playground.log)).filter((l) => l.level === 'error');
    t('no errors logged', errors.length === 0, errors);
  } catch (e) {
    t('the check ran to its end', false, e.message.split('\n')[0]);
  }
  t('no error on the preview page', pageErrors.length === 0, pageErrors);
  t('no request left this machine', nonLocal.length === 0, nonLocal);
  await context.close();
  return results;
}

// terra.js's own timers (as in JavaScriptCore), Reduce Motion, and a pack that reaches for the DOM
async function runOthers(browser, server) {
  const results = [];
  const t = (name, ok, detail) => {
    results.push({ device: 'iphone', name, ok: !!ok, detail: ok ? undefined : detail });
    console.log(`${ok ? 'ok  ' : 'FAIL'} iphone: ${name}${ok || detail === undefined ? '' : '  ' + JSON.stringify(detail)}`);
  };
  const context = await browser.newContext({ viewport: SIZES.iphone, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const logOf = () => page.evaluate(() => window.__playground.log);
  try {
    await page.goto(`${server.url}/preview/${PACK}?device=iphone&bare=1&start=1&timers=host`);
    await page.waitForFunction(() => window.__playground.started, null, { timeout: 10000 });
    await sleep(400);
    let log = await logOf();
    t('timers=host: terra.js keeps the timers through the host, and they fire', log.some((l) => l.message.startsWith('echo timeout ')) && !log.some((l) => l.level === 'error'),
      log.filter((l) => l.level !== 'info'));

    await page.goto(`${server.url}/preview/${PACK}?device=iphone&bare=1&start=1&reduceMotion=1`);
    await page.waitForFunction(() => window.__playground.started && window.__playground.settled(), null, { timeout: 10000 });
    await sleep(300);
    const s = await page.evaluate(() => ({ cam: window.__playground.camera(), boat: window.__playground.state().sprites.find((x) => x.id === 'boat'), at: window.__playground.project(14.51, 35.9) }));
    t('Reduce Motion: the anchor is arrived at, not flown to', close(s.cam.lon, FIX.anchor.lon, 1e-6) && close(s.cam.zoom, FIX.anchor.zoom, 1e-6), s.cam);
    t('Reduce Motion: a sprite holds still at its first place', s.boat && s.boat.drawn && Math.hypot(s.boat.drawn.x - s.at.x, s.boat.drawn.y - s.at.y) < 0.5, s);

    await page.goto(`${server.url}/preview/preview-broken?device=iphone&bare=1`);
    await page.waitForFunction(() => window.__playground.started, null, { timeout: 10000 });
    await sleep(300);
    log = await logOf();
    const err = log.find((l) => l.level === 'error' && /document/.test(l.message));
    t('a pack reaching for the DOM fails as on the phone, with its file and line', err && /^main\.js:3:\d+: ReferenceError: document is not defined/.test(err.message), log);
    t('… and the next script still runs', log.some((l) => /after: the next script still runs/.test(l.message)));
    t('the host refuses what the contract does not have (an unknown style), as an error', log.some((l) => l.source === 'host' && l.level === 'error' && /squiggle/.test(l.message)));
  } catch (e) {
    t('the other checks ran to their end', false, e.message.split('\n')[0]);
  }
  await context.close();
  return results;
}

if (require.main === module) {
  if (process.argv.includes('--write')) {
    const i = process.argv.indexOf('--write'), base = process.argv[i + 1] && !process.argv[i + 1].startsWith('--') ? path.resolve(process.argv[i + 1]) : FIXTURE;
    const dir = writeFixture(base);
    console.log('wrote ' + path.relative(ROOT, dir) + '/  (node tools/playground/check.cjs ' + PACK + ' --root ' + path.relative(ROOT, base) + ')');
  } else {
    (async () => {
      const { chromium } = require('playwright');
      const { start } = require('./serve.cjs');
      const di = process.argv.indexOf('--device'), asked = di >= 0 ? process.argv[di + 1] : null;
      const devices = asked ? [asked === 'phone' ? 'iphone' : asked] : ['iphone', 'ipad'];
      if (devices.some((d) => !SIZES[d])) { console.error('--device is iphone or ipad'); process.exit(2); }
      writeFixture(FIXTURE);
      const brokenBase = path.join(ROOT, 'build', 'harness-fixture-broken');
      writeBroken(brokenBase);
      fs.mkdirSync(OUT, { recursive: true });
      const server = await start({ port: 0, roots: [FIXTURE, brokenBase] });
      const browser = await chromium.launch(process.env.CHROMIUM_PATH ? { executablePath: process.env.CHROMIUM_PATH } : {});
      const results = [];
      try {
        for (const d of devices) results.push(...await runDevice(browser, server, d));
        results.push(...await runOthers(browser, server));
      } finally {
        await browser.close();
        await server.close();
      }
      const failed = results.filter((r) => !r.ok);
      fs.writeFileSync(path.join(OUT, 'report.json'), JSON.stringify({ passed: !failed.length, results }, null, 2) + '\n');
      console.log(`${failed.length ? 'FAIL' : 'PASS'}: ${results.length - failed.length} of ${results.length} checks; screenshots in ${path.relative(ROOT, OUT)}/`);
      process.exit(failed.length ? 1 : 0);
    })().catch((e) => { console.error(e); process.exit(1); });
  }
}
