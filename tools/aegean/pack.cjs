'use strict';
// Builds the globe pack of the Aegean leaf, Prototypes/sea-between/, from the leaf's own files:
//   data.js       the chart's sounding grid (depth and land, 350 x 315 cells over 15-35° E, 31-45° N),
//                 its depth contours in longitude and latitude, the leaf's names, the engraving's bounds
//   *.png         the leaf's marginal drawings (whale, blow, skiff, wind, palm) as sprite frames, drawn
//                 by the leaf's own canvas routines (Leaf/leaf.js) in Chromium
//   engraving.png the Aegean of Leaf/data/day.png, for the globe (tools/aegean/pack_engraving.py)
// node tools/aegean/pack.cjs   (Playwright's Chromium, or CHROMIUM_PATH). No network; Leaf/ is only read.
const fs = require('fs'), path = require('path'), { spawnSync } = require('child_process');
const { chromium } = require('playwright');

const ROOT = path.resolve(__dirname, '../..');
const LEAF = path.join(ROOT, 'Leaf');
const OUT = path.join(ROOT, 'Prototypes', 'sea-between');
const PX = 3;            // pixels per point in the sprite pictures (an iPhone's 3x; the iPad's 2x at 1.2 reading)

// ---------------------------------------------------------------------------------------------
// The chart (Leaf/data/chart.js, from tools/aegean/build.py): a local cylindrical chart, longitude
// contracted by a constant cos, so a world point is linear in longitude and latitude.
const chart = (() => { const window = {}; new Function('window', fs.readFileSync(path.join(LEAF, 'data', 'chart.js'), 'utf8'))(window); return window.LeafChart; })();
const P = chart.projection;
const lonOf = x => P.lon0 + x / (P.cos * P.scale);
const latOf = y => P.lat0 - y / P.scale;
const B = chart.bounds; // [x, y, width, height] in chart units

// The grid covers exactly the chart's bounds: 15-35° E, 45-31° N (build.py's BOX and linspace).
const grid = { width: chart.raster.width, height: chart.raster.height,
  west: round(lonOf(B[0]), 6), east: round(lonOf(B[0] + B[2]), 6), north: round(latOf(B[1]), 6), south: round(latOf(B[1] + B[3]), 6) };
const land = Buffer.from(chart.raster.land, 'base64'), depth = Buffer.from(chart.raster.depth, 'base64');
const bits = Buffer.alloc(Math.ceil(land.length / 8));
for (let i = 0; i < land.length; i++) if (land[i] > 127) bits[i >> 3] |= 128 >> (i & 7);

function round(v, d) { const k = 10 ** d; return Math.round(v * k) / k; }
function cell(lon, lat) {
  const x = Math.floor((lon - grid.west) / (grid.east - grid.west) * grid.width);
  const y = Math.floor((grid.north - lat) / (grid.north - grid.south) * grid.height);
  return x < 0 || y < 0 || x >= grid.width || y >= grid.height ? -1 : y * grid.width + x;
}
// Open water: the cell and its neighbours out to reach degrees (at a third, two thirds and all of it).
function sea(lon, lat, reach = 0) {
  for (const r of [reach / 3, reach * 2 / 3, reach]) {
    for (const [dx, dy] of [[0, 0], [r, 0], [-r, 0], [0, r], [0, -r], [r, r], [-r, -r], [r, -r], [-r, r]]) {
      const i = cell(lon + dx, lat + dy);
      if (i < 0 || land[i] > 127) return false;
    }
  }
  return true;
}

// The engraving: painted at 1.65 pixels a chart unit from the chart's corner (leaf.js makeMap), so its
// pixel columns and rows are equal steps of longitude and latitude, and a crop at whole pixels has
// exact bounds. The sheet on the globe is the leaf's Aegean, about what the leaf first shows on an
// iPad (leaf.js home), not the whole prepared chart: beyond it the globe's own map carries on.
const SHEET = { west: 19.5, south: 33.4, east: 30.2, north: 41.4 };
const png = fs.readFileSync(path.join(LEAF, 'data', 'day.png'));
const dayW = png.readUInt32BE(16), dayH = png.readUInt32BE(20), PAINT = 1.65;
const pxX = lon => Math.round(((lon - P.lon0) * P.cos * P.scale - B[0]) * PAINT);
const pxY = lat => Math.round(((P.lat0 - lat) * P.scale - B[1]) * PAINT);
const crop = [pxX(SHEET.west), pxY(SHEET.north), pxX(SHEET.east), pxY(SHEET.south)];
if (crop[0] < 0 || crop[1] < 0 || crop[2] > dayW || crop[3] > dayH) throw new Error('the sheet is outside the painted chart');
const engraving = [round(lonOf(B[0] + crop[0] / PAINT), 6), round(latOf(B[1] + crop[3] / PAINT), 6),
                   round(lonOf(B[0] + crop[2] / PAINT), 6), round(latOf(B[1] + crop[1] / PAINT), 6)];

// Depth contours (the sounding glass's 250 to 4,000 m), in degrees, simplified to about 300 m.
function simplify(points, tolerance) {
  if (points.length < 3) return points;
  const keep = new Uint8Array(points.length); keep[0] = keep[points.length - 1] = 1;
  const stack = [[0, points.length - 1]];
  const k = Math.cos(37.5 * Math.PI / 180);
  while (stack.length) {
    const [a, b] = stack.pop();
    let best = -1, far = 0;
    const ax = points[a][0] * k, ay = points[a][1], bx = points[b][0] * k, by = points[b][1];
    const dx = bx - ax, dy = by - ay, len = Math.hypot(dx, dy) || 1e-12;
    for (let i = a + 1; i < b; i++) {
      const d = Math.abs((points[i][0] * k - ax) * dy - (points[i][1] - ay) * dx) / len;
      if (d > far) { far = d; best = i; }
    }
    if (far > tolerance && best > 0) { keep[best] = 1; stack.push([a, best], [best, b]); }
  }
  return points.filter((_, i) => keep[i]);
}
let before = 0, after = 0;
const contours = chart.soundings.map(([level, line]) => {
  const pts = line.map(([x, y]) => [lonOf(x), latOf(y)]);
  before += pts.length;
  const s = simplify(pts, 0.003);
  after += s.length;
  return [level, [].concat(...s.map(([lon, lat]) => [round(lon, 3), round(lat, 3)]))];
}).filter(([, f]) => f.length >= 4);

// The leaf's own names (leaf.js geoLabels), less the countries and cities the globe letters itself.
const names = {
  sea: [['Aegean Sea', 24.65, 38.7]],
  regions: [['CRETE', 24.7, 35.10], ['PELOPONNESE', 22.25, 37.56]],
  waters: [['Cyclades', 24.66, 37.14], ['Dodecanese', 27.28, 37.23], ['Sea of Crete', 25.0, 35.83], ['Myrtoan Sea', 23.73, 36.90], ['Saronic Gulf', 23.35, 37.65]],
  islands: [['Euboea', 23.73, 38.64], ['Lesbos', 26.25, 39.16], ['Rhodes', 28.0, 36.14], ['Chios', 26.09, 38.37], ['Samos', 26.83, 37.71],
    ['Naxos', 25.47, 37.02], ['Lemnos', 25.22, 39.93], ['Thasos', 24.66, 40.68], ['Samothrace', 25.53, 40.46], ['Skyros', 24.57, 38.89],
    ['Andros', 24.85, 37.84], ['Ikaria', 26.16, 37.60], ['Kos', 27.23, 36.81], ['Karpathos', 27.17, 35.57], ['Kythira', 22.98, 36.23],
    ['Milos', 24.43, 36.71], ['Paros', 25.16, 37.07], ['Tinos', 25.16, 37.60], ['Mykonos', 25.35, 37.44], ['Syros', 24.90, 37.44],
    ['Patmos', 26.55, 37.32], ['Astypalea', 26.39, 36.57]]
};

// The marginal life's places, each checked against the chart's land mask.
const places = {
  // the leaf's skiff starts off Kea (leaf.js boat {x:-36, y:72}); on the globe, where names and the wind
  // crowd the Cyclades, it drifts a little in the open north Aegean instead
  skiff: [[24.50, 39.55], [24.75, 39.35]],
  // broadly south of Crete, along the deep water of the Hellenic Trench (notes.js: not a track)
  whale: [[22.70, 34.60], [23.50, 34.45], [24.40, 34.30], [25.30, 34.20]],
  // the leaf draws its wind strokes at the wind note's place (notes.js: 25.75° E, 38.25° N), over Chios's
  // name here; on the globe they sit a little south-west, in open water north of the Cyclades
  wind: [25.45, 38.00],
  // a few loose strokes over open water among the islands, as the leaf's drifting ones
  streams: [[25.05, 38.85], [24.75, 38.30], [26.09, 36.96], [25.45, 39.45], [24.10, 36.95], [26.50, 36.20]]
};
for (const [lon, lat] of [...places.skiff, ...places.whale, places.wind, ...places.streams]) {
  if (!sea(lon, lat, 0.12)) throw new Error(`${lon}, ${lat} is on or next to land in the chart`);
}
for (const [a, b] of places.whale.slice(1).map((p, i) => [places.whale[i], p])) {
  for (let t = 0; t <= 1; t += 0.05) {
    const lon = a[0] + (b[0] - a[0]) * t, lat = a[1] + (b[1] - a[1]) * t;
    if (!sea(lon, lat, 0.05) || depth[cell(lon, lat)] * 25 < 1500) throw new Error(`whale path at ${lon}, ${lat} is not deep water`);
  }
}

const data = {
  engraving,
  grid: Object.assign(grid, { land: bits.toString('base64'), depth: depth.toString('base64'), metresPerStep: 25 }),
  contours,
  names,
  places
};
fs.mkdirSync(OUT, { recursive: true });
fs.writeFileSync(path.join(OUT, 'data.js'),
  '/* The sea between: data prepared by tools/aegean/pack.cjs from the Aegean leaf (Leaf/data/chart.js).\n' +
  '   engraving: the bounds of engraving.png [west, south, east, north], degrees.\n' +
  '   grid: the sounding grid over [west, east] x [north, south], row by row from the north-west; land one bit a\n' +
  '   cell (most significant first), depth one byte a cell in steps of metresPerStep (255: that or deeper).\n' +
  '   Generalised from NASA\'s GEBCO 2008 grayscale imagery: not survey data (SOURCES.md).\n' +
  '   contours: [metres, [lon, lat, lon, lat, ...]], the same source, generalised. names: the leaf\'s lettering. */\n' +
  'window.SEA = ' + JSON.stringify(data) + ';\n');
console.log(`data.js: ${contours.length} contours, ${before} -> ${after} vertices; engraving pixels ${crop.join(', ')}, bounds ${engraving.join(', ')}`);

// ---------------------------------------------------------------------------------------------
// The sprites, by the leaf's drawing routines in Chromium. Day palette (leaf.js palette[0]).
const page = String.raw`<!doctype html><meta charset="utf-8"><body><script>
const P = {water:'#dce8e3',paper:'#f4f1e8',land:'#eee5d1',coast:'#4b6a6b',ink:'#294652',copper:'#9b4934',line:'#a9c1b7'};
const tau = Math.PI * 2, PX = ${PX};
function sheet(w, h) { const c = document.createElement('canvas'); c.width = Math.round(w * PX); c.height = Math.round(h * PX); const g = c.getContext('2d'); g.scale(PX, PX); return [c, g]; }
// Draw on its own sheet, then lay it down at an alpha (so overlapping strokes do not darken).
function faded(w, h, alpha, draw) { const [c, g] = sheet(w, h), [o, og] = sheet(w, h); draw(g); og.setTransform(1, 0, 0, 1, 0, 0); og.globalAlpha = alpha; og.drawImage(c, 0, 0); return o; }

// leaf.js whale(): the same outline, facing east (as a sprite with face: true is drawn), its flukes
// lifted by 'flex' (-1 to 1) for the swimming frames; no tilt (the globe turns it along its way).
function whale(g, flex) {
  const q = (x, y) => [x, y + (x > 14 ? flex * 7 * ((x - 14) / 36) ** 2 : 0)];
  const M = (x, y) => g.moveTo(...q(x, y)), L = (...a) => g.bezierCurveTo(...q(a[0], a[1]), ...q(a[2], a[3]), ...q(a[4], a[5])), Q = (...a) => g.quadraticCurveTo(...q(a[0], a[1]), ...q(a[2], a[3]));
  g.fillStyle = P.ink; g.strokeStyle = P.ink; g.lineWidth = .65;
  g.beginPath(); M(-43,-8); L(-48,-23,-28,-24,-11,-18); L(6,-16,19,-7,34,-2); Q(43,-10,50,-9); Q(46,-2,38,2); Q(44,7,50,8); Q(39,13,33,4); L(10,6,-9,11,-30,5); Q(-43,3,-43,-8); g.fill();
  g.strokeStyle = P.water; g.lineWidth = 1.1; g.beginPath(); M(-38,-2); Q(-25,4,-8,1); M(-30,-14); Q(-12,-12,2,-5); g.stroke();
  g.fillStyle = P.water; g.beginPath(); g.arc(-35, -10, 1.3, 0, tau); g.fill();
}
function whaleFrame(i, n) {
  const w = 46, h = 24, s = w / 102;
  return faded(w, h, .62, g => { g.translate(w / 2, h / 2); g.scale(-s, s); g.translate(-1, 5); whale(g, Math.sin(i / n * tau)); });
}
// The whale from afar: a small blow and its ring, now and then (original to the pack, in the leaf's ink).
function blowFrame(i) {
  const w = 16, h = 16;
  return faded(w, h, [.9, .95, .8, .5][i], g => {
    g.strokeStyle = P.ink; g.lineCap = 'round';
    g.lineWidth = .8; g.beginPath(); g.ellipse(8, 12.5, 2.2 + i * 1.3, 1 + i * .45, 0, 0, tau); g.stroke();
    const up = [3.5, 7.5, 9.5, 9][i], spread = [.4, 1.4, 2.6, 3.4][i];
    g.lineWidth = .9;
    for (const d of [-1, 0, 1]) { g.beginPath(); g.moveTo(8, 11.5); g.quadraticCurveTo(8 + d * spread * .3, 11.5 - up * .6, 8 + d * spread, 11.5 - up); g.stroke(); }
  });
}
// leaf.js drawSkiff(), its bob and sail as frames of one loop.
function skiff(g, phase, sail) {
  g.rotate(-.22 + Math.sin(phase) * .035); g.translate(0, Math.sin(phase + 1) * 1);
  g.strokeStyle = P.ink; g.lineWidth = .8; g.fillStyle = P.land;
  g.beginPath(); g.moveTo(-12,5); g.quadraticCurveTo(0,13,16,4); g.lineTo(-12,5); g.fill(); g.stroke();
  g.strokeStyle = P.copper; g.beginPath(); g.moveTo(-4,6); g.lineTo(-4,-25); g.stroke();
  g.fillStyle = P.paper; g.strokeStyle = P.ink; g.beginPath(); g.moveTo(-3,-24); g.quadraticCurveTo(6 + Math.sin(phase) * sail, -10, 12 * sail, 2); g.lineTo(-3,2); g.closePath(); g.fill(); g.stroke();
  g.fillStyle = P.copper; g.beginPath(); g.moveTo(-6,-19); g.lineTo(-14 * sail, 2); g.lineTo(-6,2); g.closePath(); g.fill();
  g.strokeStyle = P.line; g.lineWidth = .6;
  for (let i = 0; i < 3; i++) { g.beginPath(); g.moveTo(-19, 10 + i * 4); g.quadraticCurveTo(0, 13 + i * 4, 20, 9 + i * 4); g.stroke(); }
}
function skiffFrame(i, n, sail) {
  const w = 28, h = 32, s = w / 44;
  const [c, g] = sheet(w, h); g.translate(w / 2, h / 2); g.scale(s, s); g.translate(0, 2); skiff(g, i / n * tau, sail); return c;
}
// leaf.js drawWind(): four strokes, at the leaf's .7 alpha.
function wind() {
  const w = 32, h = 30, s = w / 62;
  return faded(w, h, .75, g => {
    g.translate(w / 2, h / 2); g.scale(s, s); g.translate(-3.5, -2.5); g.strokeStyle = P.ink; g.lineWidth = 1.4; g.lineCap = 'round';
    for (let i = 0; i < 4; i++) { g.beginPath(); g.moveTo(-18 + i * 9, -23); g.bezierCurveTo(-25 + i * 9, 0, 20 + i * 3, 5, 8 + i * 8, 28); g.stroke(); }
  });
}
// leaf.js life(): one of its loose strokes drifting south, fading in and out (frame 0 at mid-way, whole).
function streamFrame(i, n) {
  const w = 10, h = 36, u = (i / n + .5) % 1, a = Math.pow(Math.sin(Math.PI * u), .7) * .42;
  return faded(w, h, Math.max(a, .001), g => {
    const s = .8, x = 4, y = 1 + u * (h - 2 - 18 * s);
    g.translate(x, y); g.scale(s, s); g.strokeStyle = P.ink; g.lineWidth = 1; g.lineCap = 'round';
    g.beginPath(); g.moveTo(0, 0); g.bezierCurveTo(-4, 4, 4, 10, 1, 18); g.stroke();
  });
}
// leaf.js palm(), still; its foot at the picture's centre, so the sprite stands on its place.
function palm() {
  const w = 22, h = 50, s = .4;
  const [c, g] = sheet(w, h); g.translate(w / 2, h / 2); g.scale(s, s); g.translate(0, -27);
  g.lineCap = 'round'; g.strokeStyle = P.ink; g.lineWidth = 1.6;
  g.beginPath(); g.moveTo(-4,25); g.quadraticCurveTo(1,7,0,-9); g.moveTo(1,25); g.quadraticCurveTo(5,10,0,-9); g.stroke();
  for (const side of [-1, 1]) for (let i = 0; i < 4; i++) {
    const endX = side * (13 + i * 4), endY = -20 + i * 10;
    g.beginPath(); g.moveTo(0,-9); g.quadraticCurveTo(endX * .6, -30 + i * 5, endX, endY); g.stroke();
    for (let j = 1; j < 7; j++) { const u = j / 7; const xx = 2 * (1 - u) * u * endX * .6 + u * u * endX, yy = (1 - u) * (1 - u) * -9 + 2 * (1 - u) * u * (-30 + i * 5) + u * u * endY; g.beginPath(); g.moveTo(xx, yy); g.lineTo(xx + side * 2.5, yy + 5); g.stroke(); }
  }
  g.beginPath(); g.ellipse(-2, 27, 12, 2, 0, 0, tau); g.stroke();
  return c;
}
window.sprites = () => {
  const out = {}, png = c => c.toDataURL('image/png').split(',')[1], two = i => String(i + 1).padStart(2, '0');
  for (let i = 0; i < 8; i++) out['whale-' + (i + 1) + '.png'] = png(whaleFrame(i, 8));
  for (let i = 0; i < 4; i++) out['blow-' + (i + 1) + '.png'] = png(blowFrame(i));
  for (let i = 0; i < 8; i++) out['skiff-' + (i + 1) + '.png'] = png(skiffFrame(i, 8, 1));
  for (let i = 0; i < 8; i++) out['skiff-folded-' + (i + 1) + '.png'] = png(skiffFrame(i, 8, .16));
  for (let i = 0; i < 16; i++) out['stream-' + two(i) + '.png'] = png(streamFrame(i, 16));
  out['wind.png'] = png(wind());
  out['palm.png'] = png(palm());
  return out;
};
</script>`;

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, args: ['--no-sandbox'] });
  try {
    const tab = await browser.newPage();
    await tab.route('**/*', route => route.abort()); // nothing is fetched
    await tab.setContent(page);
    const files = await tab.evaluate(() => window.sprites());
    for (const old of fs.readdirSync(OUT)) if (/^(whale|blow|skiff|stream)-.*\.png$|^(wind|palm)\.png$/.test(old)) fs.unlinkSync(path.join(OUT, old));
    let bytes = 0;
    for (const [name, b64] of Object.entries(files)) { const buf = Buffer.from(b64, 'base64'); bytes += buf.length; fs.writeFileSync(path.join(OUT, name), buf); }
    console.log(`sprites: ${Object.keys(files).length} pictures, ${bytes} bytes`);
  } finally { await browser.close(); }
  const r = spawnSync('python3', [path.join(__dirname, 'pack_engraving.py'), ...crop.map(String)], { stdio: 'inherit' });
  if (r.status !== 0) process.exit(r.status || 1);
})().catch(e => { console.error(e); process.exit(1); });
