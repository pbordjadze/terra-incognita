/* Chalk and Slate: the five plates (the contract is ../kit.js; the drawing kit is chalk-kit.js).

   Every level is drawn through the shared lens, centred where the film ends, so the plates nest
   exactly. The thread of the film is a gold ring: on each plate it marks the next one, sized so that
   as the camera dives it opens just past the screen's edges while the closer plate fades up. That
   closer plate carries the same ring in chalk, out in its margin, so the gold turns to chalk as it
   arrives. On the last plate the ring is the oval the neighbourhood sits in, the way the courtyard
   sits in its ellipse in the film.

   Layers: ground (opaque: slate, washes and all the chalk), labels, sheen (the gold, as a mask for
   the app's band of light), and one small living thing on the world (a red sail) and on Short North
   (the lamps of the High Street arches).                                                          */
'use strict';
const SCALES = [Kit.W * 0.29, Kit.km(3400), Kit.km(600), Kit.km(38), Kit.km(1.3)];
const IDS = ['world', 'america', 'ohio', 'columbus', 'shortnorth'];
const CAPTIONS = [
  { eyebrow: 'I · The World', title: 'Orbis Terrarum', text: 'The whole Earth, chalked on the abbey’s slate.' },
  { eyebrow: 'II · North America', title: 'The Inland Seas', text: 'Five great lakes, and Ohio just below Erie.' },
  { eyebrow: 'III · Ohio', title: 'Between Two Waters', text: 'Erie above, the Ohio below, Columbus at the heart.' },
  { eyebrow: 'IV · Columbus', title: 'The Walled City', text: 'The outerbelt is its wall; two rivers meet downtown.' },
  { eyebrow: 'V · The Neighbourhood', title: 'Short North', text: 'Arches over High Street, brick rows, round trees.' },
];
const lensPx = k => Kit.lens(SCALES[k - 1]);
const lensPt = k => Kit.lens(SCALES[k - 1]).scale(SCALES[k - 1] / DPR).translate([W / 2, H / 2]).clipExtent([[-40, -40], [W + 40, H + 40]]);
// how many of the next plate's points one of this plate's points is
const RATIO = k => SCALES[k] / SCALES[k - 1];

// The gold ring of each plate, around where the next one sits: [rx, ry, dy] in points. Each maps to
// about 233 pt on the next plate (just past the screen's half-width); Short North's oval is the last
// one, 162 x 268 pt, a little above the centre so it clears the caption and, at the end of the last
// drift (1.18x), still sits just inside the screen.
const OVAL = [162, 268, -30];
const RING = { 1: [36, 36, 0], 2: [41, 41, 0], 3: [15, 15, 0], 4: [OVAL[0] / RATIO(4), OVAL[1] / RATIO(4), OVAL[2] / RATIO(4)] };
const RING_W = { 1: 1.15, 2: 1.2, 3: 1.2, 4: 1.35 };
function ringStroke(k) {   // the same strokes on the ground and on the sheen mask
  const rng = mulberry(770 + k), [rx, ry, dy] = RING[k], a0 = rng() * TAU;
  return { pts: circlePts(W / 2, H / 2 + dy, rx, 0, a0, a0 + TAU + 0.5, ry), rng };
}
function drawRing(g, k, color, a) {
  const { pts, rng } = ringStroke(k);
  chalk(g, pts, { w: RING_W[k], a, amp: 0.3, wl: 22, color, rng, closed: false });
}
function ringFrame(k) {
  const [rx, ry, dy] = RING[k], m = 4;
  return [W / 2 - rx - m, H / 2 + dy - ry - m, 2 * (rx + m), 2 * (ry + m)].map(v => Math.round(v * DPR));
}
// the previous plate's gold ring, arrived: the same circle in chalk, in this plate's margin
function arrivedRing(g, k, a = 0.36) {
  const [rx, ry, dy] = RING[k - 1], f = RATIO(k - 1);
  handCircle(g, W / 2, H / 2 + dy * f, rx * f, { ry: ry * f, w: 1.3, a, rng: mulberry(990 + k), wl: 40 });
}

// The ship: St Brendan's red sail on the Atlantic, heading west.
const SHIP_AT = [-34, 44];
function shipBox() { const [x, y] = lensPt(1)(SHIP_AT); return [x - 12, y - 16, 30, 22]; }
function drawShip(g) {
  const [x, y] = lensPt(1)(SHIP_AT), rng = mulberry(4401);
  g.fillStyle = INK.red; g.beginPath(); g.moveTo(x - 2.3, y - 9); g.lineTo(x + 2.6, y - 8.6); g.lineTo(x + 2.9, y - 2.6); g.lineTo(x - 2.6, y - 2.4); g.closePath(); g.fill();
  chalk(g, [[x, y - 11], [x, y - 0.8]], { w: 0.6, a: 0.95, amp: 0, passes: 1, step: 1, rng });
  chalk(g, [[x - 5.5, y - 1.8], [x - 3.5, y + 1.2], [x + 3.5, y + 1.2], [x + 5.8, y - 1.8]], { w: 0.75, a: 0.95, amp: 0.05, passes: 1, step: 1, rng });
  for (let i = 0; i < 3; i++) chalk(g, [[x + 7 + i * 3.4, y + 1 + i * 0.4], [x + 9.2 + i * 3.4, y + 0.6 + i * 0.4]], { w: 0.5, a: 0.6 - i * 0.13, amp: 0, passes: 1, step: 1, rng });
}

function layersOf(k) {
  const px = r => r.map(v => Math.round(v * DPR));
  const sheen = () => ({ name: 'sheen', frame: ringFrame(k), motion: { kind: 'sheen', period: 8, width: 120, angle: 24, color: '#fff3d2', opacity: 0.7 } });
  if (k === 1) return [
    { name: 'ground', opaque: true },
    sheen(),
    { name: 'ship', depth: 0.02, frame: px(shipBox()), motion: { kind: 'bob', amp: [2, 3], period: 7.5, phase: 0.2 } },
  ];
  if (k === 5) return [
    { name: 'ground', opaque: true },
    { name: 'labels', depth: 0.04 },
    { name: 'lamps', frame: px(ARCH_BOX), motion: { kind: 'breathe', min: 0.45, max: 1, period: 6.5, phase: 0 } },
    { name: 'sheen', depth: 0.04, frame: px(INITIAL_BOX), motion: { kind: 'sheen', period: 8, width: 90, angle: 24, color: '#fff3d2', opacity: 0.75 } },
  ];
  return [{ name: 'ground', opaque: true }, { name: 'labels', depth: 0.04 }, sheen()];
}

window.Plates = {
  levels: 5,
  style: {
    fonts: ['fonts/IMFellEnglishSC.ttf', 'fonts/IMFellEnglish-Roman.ttf', 'fonts/IMFellEnglish-Italic.ttf'],
    eyebrow: { font: 'IM_FELL_English_SC', size: 12.5, color: '#d0574a', tracking: 2.9, uppercase: true },
    title: { font: 'IM_FELL_English_Roman', size: 28, color: '#eceee8' },
    text: { font: 'IM_FELL_English_Italic', size: 17, color: '#eceee8c4' },
    scrim: '#0b1015e8',
    accent: '#c94a3c',
  },
  info(level) {
    return { id: IDS[level - 1], caption: CAPTIONS[level - 1], layers: layersOf(level),
      child: level < 5 ? Kit.childRect(lensPx(level), lensPx(level + 1)) : null };
  },
};

// ================================================================ data
const DATA = {};
const D = name => Kit.json(`../data/${name}`);
async function load(level) {
  const need = {
    1: ['land-50m.json', 'america-lakes.geojson'],
    2: ['land-50m.json', 'america-land.geojson', 'america-lakes.geojson', 'america-rivers.geojson', 'america-states.geojson', 'america-places.geojson'],
    3: ['ohio-lakes.geojson', 'ohio-rivers.geojson', 'ohio-states.geojson', 'ohio-places.geojson', 'ohio-roads.geojson', 'america-lakes.geojson', 'america-rivers.geojson', 'america-states.geojson', 'america-land.geojson'],
    4: ['columbus-roads.geojson', 'columbus-water.geojson', 'columbus-streams.geojson', 'columbus-city.geojson'],
    5: ['shortnorth-roads.geojson', 'shortnorth-buildings.geojson', 'shortnorth-water.geojson', 'shortnorth-green.geojson', 'shortnorth-rail.geojson', 'shortnorth-trees.geojson'],
  }[level];
  for (const n of need) DATA[n.replace(/\.(geo)?json$/, '')] = await D(n);
}
const fixName = s => { try { return decodeURIComponent(escape(s)); } catch (e) { return s; } };   // names read as latin-1
// A polygon layer's outlines as open runs, without the edges its clipping box cut along.
function outlinesOff(geo, [x0, y0, x1, y1]) {
  const e = 1e-6, onBox = (p, q) => (Math.abs(p[0] - x0) < e && Math.abs(q[0] - x0) < e) || (Math.abs(p[0] - x1) < e && Math.abs(q[0] - x1) < e) ||
    (Math.abs(p[1] - y0) < e && Math.abs(q[1] - y0) < e) || (Math.abs(p[1] - y1) < e && Math.abs(q[1] - y1) < e);
  const runs = [];
  const ring = r => { let cur = [r[0]]; for (let i = 1; i < r.length; i++) { if (onBox(r[i - 1], r[i])) { if (cur.length > 1) runs.push(cur); cur = [r[i]]; } else cur.push(r[i]); } if (cur.length > 1) runs.push(cur); };
  for (const f of geo.features) { const g = f.geometry; if (!g) continue; (g.type === 'Polygon' ? [g.coordinates] : g.type === 'MultiPolygon' ? g.coordinates : []).forEach(poly => poly.forEach(ring)); }
  return { type: 'MultiLineString', coordinates: runs };
}
function boxRing(proj, [x0, y0, x1, y1], n = 80) {
  const pts = [];
  for (let i = 0; i <= n; i++) pts.push(proj([x0 + (x1 - x0) * i / n, y1]));
  for (let i = 0; i <= n; i++) pts.push(proj([x1, y1 - (y1 - y0) * i / n]));
  for (let i = 0; i <= n; i++) pts.push(proj([x1 - (x1 - x0) * i / n, y0]));
  for (let i = 0; i <= n; i++) pts.push(proj([x0, y0 + (y1 - y0) * i / n]));
  return pts.filter(Boolean);
}
const lineAlong = (proj, lonlats, step = 2) => resample(lonlats.map(p => proj(p)), step, false);
function dedupe(features) {   // TIGER and OSM repeat a road under each of its names
  const seen = new Set();
  return features.filter(f => {
    const g = f.geometry; if (!g) return false;
    const flat = g.type === 'LineString' ? g.coordinates : g.type === 'MultiLineString' ? g.coordinates.flat() : g.coordinates.flat(2);
    const a = flat[0].join(','), b = flat[flat.length - 1].join(','), k = flat.length + ':' + (a < b ? a + b : b + a);
    if (seen.has(k)) return false; seen.add(k); return true;
  });
}
const km2 = f => d3.geoArea(f) * 6371 * 6371;

// The land's tones, kept the same from plate to plate so a dive never changes the light: land, then
// Ohio a little lit, then the walled city, then the oval.
function landTone(g, path, mottle = true) { wash(g, path, INK.land, 0.32, { mottle }); }
function ohioTone(g, path) { wash(g, path, INK.lit, 0.11, { mottle: false }); }
function cityTone(g, path) { wash(g, path, '#9fb0bf', 0.08, { mottle: false }); }
// Inside Ohio, from Columbus down, the ground is one quiet tone near Ohio's on the plates before.
function innerTone(g) { wash(g, frameRect(), INK.land, 0.3, { mottle: false }); wash(g, frameRect(), INK.lit, 0.08, { mottle: false }); }
// The caption's ground: the chalk eases off over the lowest fifth of the screen at rest, so the
// caption sits on quiet slate.
function captionCalm(g, floor = 0.55) {
  const m = layer(), y0 = H / 2 + 225, y1 = H / 2 + 330, gr = m.createLinearGradient(0, y0, 0, y1);
  gr.addColorStop(0, '#000'); gr.addColorStop(1, `rgba(0,0,0,${floor})`);
  m.fillStyle = '#000'; m.fillRect(-W, -H, 3 * W, y0 + H); m.fillStyle = gr; m.fillRect(-W, y0, 3 * W, 2 * H);
  put(g, m, 'destination-in');
}
function dust(g, n) {
  for (let i = 0; i < n; i++) {
    const x = rr(0, W), y = rr(0, H), r = rr(0.25, 0.7), a = rr(0.04, 0.16);
    if (!g) continue;
    g.globalAlpha = a; g.fillStyle = INK.chalk; g.beginPath(); g.arc(x, y, r / VIEW.s, 0, TAU); g.fill();
  }
  if (g) g.globalAlpha = 1;
}

// ================================================================ 1. the world, an orb in the courtyard's rings
function level1(out, which) {
  const land = topojson.feature(DATA['land-50m'], DATA['land-50m'].objects.land);
  const proj = lensPt(1), R = proj.scale(), cx = W / 2, cy = H / 2;
  const centre = [Kit.TARGET[0], Kit.TARGET[1]];
  const front = (lon, lat) => d3.geoDistance([lon, lat], centre) < Math.PI / 2 - 0.12;
  const orb = new Path2D(); orb.arc(cx, cy, R, 0, TAU);
  const landLines = capture(proj, land).filter(l => l.closed);
  const landPath = toPath(landLines);
  const BIG = new Set(['Lake Superior', 'Lake Michigan', 'Lake Huron', 'Georgian Bay', 'Lake Erie', 'Lake Ontario', 'Great Bear Lake', 'Great Slave Lake', 'Lake Winnipeg', 'Lake Athabasca', 'Lake Manitoba', 'Lake Winnipegosis', 'Lago de Nicaragua']);
  const lakeLines = capture(proj, FC(DATA['america-lakes'].features.filter(f => BIG.has(f.properties.name)))).filter(l => l.closed);
  const lakePath = toPath(lakeLines);
  // placements, independent of what is drawn
  const curls = poisson(46, () => {
    const a = rand() * TAU, rad = Math.sqrt(rand()) * (R - 14), x = cx + Math.cos(a) * rad, y = cy + Math.sin(a) * rad;
    for (const [dx, dy] of [[0, 0], [-9, 0], [9, 0], [0, -6], [0, 6]]) if (inPath(landPath, x + dx, y + dy)) return null;
    const [sx, sy] = proj(SHIP_AT); if (Math.hypot(x - sx, y - sy) < 22) return null;
    return [x, y, rr(3.2, 4.6), rand() < 0.5 ? -1 : 1, rand() < 0.4];
  }, 19);
  const stars = poisson(170, () => { const x = rr(4, W - 4), y = rr(4, H - 4); return Math.hypot(x - cx, y - cy) > R + 30 ? [x, y, rand(), rr(0.4, 1.05), rr(0.25, 0.85), rr(2.6, 4)] : null; }, 13);

  if (which === 'ground') {
    const G = out;
    slate(G, { wash: 0.75 });
    wash(G, orb, INK.water, 0.62);
    { const g = layer(); const gr = g.createRadialGradient(cx - R * 0.42, cy - R * 0.45, R * 0.1, cx - R * 0.2, cy - R * 0.2, R * 1.15);
      gr.addColorStop(0, 'rgba(238,240,234,0.16)'); gr.addColorStop(0.55, 'rgba(238,240,234,0.05)'); gr.addColorStop(1, 'rgba(238,240,234,0)');
      g.fillStyle = gr; g.fill(orb); erodeWith(g, tex('tooth'), 0.7); put(G, g); }
    { const g = layer(); g.save(); g.clip(orb); g.fillStyle = INK.land; g.fill(landPath, 'evenodd'); g.restore();
      put(g, tex('mottle'), 'destination-in'); put(G, g, 'source-over', 0.5); }
    wash(G, lakePath, INK.water, 0.9);
    const ch = layer();
    { const g = layer(); g.save(); g.clip(orb);
      g.setLineDash([0.1, 3.2]); g.lineWidth = 1.05; g.strokeStyle = INK.chalk; g.globalAlpha = 0.34;
      for (const l of capture(proj, d3.geoGraticule().step([30, 30])())) strokePts(g, l.pts, false);
      g.restore(); put(ch, g); }
    rub(ch, landPath, landPath, 5, 0.32, 2.2);
    { const g = layer(); hatch(g, landPath, { gap: 3.1, a: 0.42, w: 0.55, box: [cx - R, cy - R, cx + R, cy + R], angle: -0.95 });
      const m = g.createRadialGradient(cx - R * 0.5, cy - R * 0.5, 0, cx - R * 0.3, cy - R * 0.3, R * 1.5);
      m.addColorStop(0, 'rgba(0,0,0,1)'); m.addColorStop(0.6, 'rgba(0,0,0,0.55)'); m.addColorStop(1, 'rgba(0,0,0,0.1)');
      g.globalCompositeOperation = 'destination-in'; g.fillStyle = m; g.fillRect(-W, -H, 3 * W, 3 * H); put(ch, g); }
    for (const l of landLines) { const A = Math.abs(ringArea(l.pts)); if (A >= 1.5) chalk(ch, l.pts, { w: A > 40 ? 1.0 : 0.75, a: 0.92, amp: 0.3, wl: 10, closed: true }); }
    ch.save(); ch.globalCompositeOperation = 'destination-out'; ch.fill(lakePath, 'evenodd'); ch.restore();
    for (const l of lakeLines) if (Math.abs(ringArea(l.pts)) > 1) chalk(ch, l.pts, { w: 0.75, a: 0.9, amp: 0.15, wl: 8, closed: true });
    for (const [d, a] of [[3.4, 0.5], [6.6, 0.26]]) { const r = offsetRing(landPath, d, 0.7, orb, landPath); erodeWith(r, tex('break')); put(ch, r, 'source-over', a); }
    for (const [x, y, s, dir, dots] of curls) {
      const lit = clamp(1 - Math.hypot(x - (cx - R * 0.4), y - (cy - R * 0.4)) / (R * 1.9));
      curl(ch, x, y, s, { a: 0.25 + 0.4 * lit, dir, dots });
    }
    const RANGES = [
      [[-150, 61.5], [-140, 60.5], [-131, 57], [-124, 51], [-118, 47], [-113, 44.5], [-110, 41], [-107, 37.5], [-106, 33]],
      [[-75.5, 7], [-77.2, 2.5], [-78.6, -2], [-77.2, -8], [-75, -12], [-71, -17], [-69, -22]],
      [[-86, 34.3], [-83.5, 36], [-81, 37.7], [-78.8, 39.5], [-76.7, 41.3], [-74.5, 42.6], [-72, 44.3]],
      [[-104, 20], [-101, 18], [-98, 17]], [[-49, 64.5], [-44, 68], [-40, 72], [-38, 75]], [[-8.6, 31], [-5, 32.3]],
    ];
    const peaks = [];
    for (const line of RANGES) for (const [lon, lat] of line) if (front(lon, lat)) { const [x, y] = proj([lon, lat]); peaks.push([x, y + sr(-0.6, 0.6), sr(4.4, 6.6)]); }
    peaks.sort((a, b) => a[1] - b[1]).forEach(([x, y, h]) => peak(ch, x, y, h, { a: 0.85, lw: 0.65 }));
    // the rim and the carved rings, like the courtyard's ellipse
    chalk(ch, circlePts(cx, cy, R), { w: 1.5, a: 0.95, amp: 0.4, wl: 30, closed: true });
    chalk(ch, circlePts(cx, cy, R + 7), { w: 0.75, a: 0.7, amp: 0.35, wl: 30, closed: true });
    chalk(ch, circlePts(cx, cy, R + 17), { w: 0.75, a: 0.6, amp: 0.35, wl: 30, closed: true });
    { ch.fillStyle = INK.chalk;
      for (let i = 0; i < 150; i++) {
        const t = i / 150 * TAU; if (Math.abs(((t + Math.PI / 4) % (Math.PI / 2)) - Math.PI / 4) < 0.07) continue;
        ch.globalAlpha = sr(0.55, 0.85); ch.beginPath(); ch.arc(cx + Math.cos(t) * (R + 12), cy + Math.sin(t) * (R + 12), 0.8, 0, TAU); ch.fill();
      }
      ch.globalAlpha = 1;
      for (let i = 0; i < 240; i++) {
        const t = i / 240 * TAU, l = i % 10 === 0 ? 4.5 : 2.4;
        chalk(ch, [[cx + Math.cos(t) * (R + 18.5), cy + Math.sin(t) * (R + 18.5)], [cx + Math.cos(t) * (R + 18.5 + l), cy + Math.sin(t) * (R + 18.5 + l)]], { w: 0.5, a: 0.5, amp: 0, passes: 1, step: 1 });
      }
      for (let q = 0; q < 4; q++) {
        const t = -Math.PI / 2 + q * Math.PI / 2, x = cx + Math.cos(t) * (R + 12), y = cy + Math.sin(t) * (R + 12);
        ch.save(); ch.globalCompositeOperation = 'destination-out'; ch.beginPath(); ch.arc(x, y, 6.2, 0, TAU); ch.fill(); ch.restore();
        for (let k = 0; k < 3; k++) { const a = t + k * TAU / 3; chalk(ch, spiral(x + Math.cos(a) * 2.3, y + Math.sin(a) * 2.3, 2.6, 1.3, a + Math.PI, 1), { w: 0.55, a: 0.85, amp: 0, passes: 1, step: 0.8 }); }
        handCircle(ch, x, y, 5.6, { w: 0.6, a: 0.75, passes: 1 });
      }
    }
    // stars: dots and a few spirals
    ch.fillStyle = INK.chalk;
    for (const [x, y, kind, r, a, sz] of stars) {
      if (kind < 0.08) { chalk(ch, spiral(x, y, sz, 1.8, kind * 80, 1), { w: 0.6, a: 0.75, amp: 0, passes: 1, step: 0.8 }); continue; }
      ch.globalAlpha = a; ch.beginPath(); ch.arc(x, y, r, 0, TAU); ch.fill();
    }
    ch.globalAlpha = 1;
    erodeWith(ch, tex('tooth')); put(G, ch);
    const ac = layer(); drawRing(ac, 1, INK.gold, 0.95); erodeWith(ac, tex('tooth'), 0.45); put(G, ac);
    dust(G, 600);
  } else dust(null, 600);
  if (which === 'sheen') { const g = layer(); drawRing(g, 1, '#ffffff', 1); erodeWith(g, tex('tooth'), 0.45); put(out, g); }
  if (which === 'ship') { const g = layer(); drawShip(g); erodeWith(g, tex('tooth'), 0.35); put(out, g); }
}

// ================================================================ 2. North America: the inland seas
function level2(out, which) {
  const proj = lensPt(2), cx = W / 2, cy = H / 2;
  // land-50m is finer than the 10 m layer simplified for this scale, and has no edge at 5 N or 50 W
  const land = topojson.feature(DATA['land-50m'], DATA['land-50m'].objects.land);
  const landLines = capture(proj, land).filter(l => l.closed && Math.abs(ringArea(l.pts)) > 1.2);
  const landPath = toPath(landLines);
  const GREAT = new Set(['Lake Superior', 'Lake Michigan', 'Lake Huron', 'Georgian Bay', 'Lake Erie', 'Lake Ontario', 'Lake Saint Clair']);
  const lakes = DATA['america-lakes'].features.map(f => { const lines = capture(proj, f).filter(l => l.closed); return { name: f.properties.name, lines, area: lines.reduce((s, l) => s + Math.abs(ringArea(l.pts)), 0) }; })
    .filter(o => o.area > 30 || GREAT.has(o.name));
  const great = lakes.filter(o => GREAT.has(o.name)), small = lakes.filter(o => !GREAT.has(o.name));
  const lakePath = toPath(lakes.flatMap(o => o.lines)), greatPath = toPath(great.flatMap(o => o.lines));
  const seaPath = unionPath(frameRect(), landPath), dryPath = unionPath(landPath, lakePath);
  const ohio = DATA['america-states'].features.find(f => f.properties.name === 'Ohio');
  const ohioLines = capture(proj, ohio).filter(l => l.closed), ohioPath = toPath(ohioLines);
  const oc = centroidOf(ohioLines.flatMap(l => l.pts));
  // placements
  const APP = lineAlong(proj, [[-86.6, 33.9], [-85.2, 35.0], [-83.6, 35.8], [-82.0, 36.9], [-80.6, 37.9], [-79.4, 38.9], [-78.3, 40.0], [-77.0, 41.0], [-75.6, 41.8], [-74.4, 42.4], [-73.6, 43.4], [-72.5, 44.3], [-71.3, 44.6], [-70.0, 45.2]], 1.5);
  const nearApp = (x, y) => { let d = 1e9; for (const p of APP) d = Math.min(d, Math.hypot(p[0] - x, p[1] - y)); return d; };
  const HIGH = [[-83.4, 35.6], [-82.3, 35.8], [-71.3, 44.25]].map(p => proj(p));
  const hills = poisson(90, () => { const p = APP[Math.floor(rand() * APP.length)], x = p[0] + rr(-10, 10), y = p[1] + rr(-7, 7);
    const d = nearApp(x, y); return d < 8.5 && inPath(dryPath, x, y) && !inPath(ohioPath, x, y + 2) ? [x, y, (8.5 - d) / 8.5, rand()] : null; }, 9.5)
    .map(([x, y, k, v]) => HIGH.some(h => Math.hypot(h[0] - x, h[1] - y) < 16) ? ['peak', x, y, 6 + k * 3.5, v] : ['hill', x, y, 9 + k * 4, v]);
  const woods = [];
  for (let row = 0, y = 8; y < cy - 150; row++, y += 12.5) for (let x = 4 + (row % 2) * 5; x < W - 4; x += rr(7, 10.5)) {
    const py = y + (vn(x / 40, row * 3.1) - 0.5) * 7, ll = proj.invert([x, py]);
    if (!ll || ll[1] < 46.4 || !inPath(dryPath, x, py) || !inPath(dryPath, x, py - 9)) continue;
    if (fbm(x / 46, py / 46, 2) < 0.54 || rand() < 0.4) continue;
    woods.push(['spire', x, py, rr(5.2, 7.6), 0]);
  }
  for (let row = 0, y = cy - 80; y < cy + 150; row++, y += 14) for (let x = 6 + (row % 2) * 7; x < W - 4; x += rr(11, 16)) {
    const py = y + (vn(x / 40, row * 2.3 + 50) - 0.5) * 7, ll = proj.invert([x, py]);
    if (!ll || ll[0] < -84.5 || ll[1] > 46 || ll[1] < 34.5 || !inPath(dryPath, x, py) || inPath(ohioPath, x, py) || inPath(ohioPath, x, py - 8)) continue;
    if (nearApp(x, py) < 12) continue;
    if (fbm(x / 36 + 7, py / 36, 2) < 0.58 || rand() < 0.45) continue;
    woods.push(['grove', x, py, rr(1.9, 2.3), Math.floor(rand() * 4)]);
  }
  // the night sea: rolling waves in loose rows, kept off the coasts (the ripples are there)
  const waves = [];
  for (let row = 0, y = 24; y < H - 10; row++, y += rr(36, 44)) for (let x = rr(10, 70); x < W + 50; x += rr(84, 124)) {
    const len = rr(30, 40), px = x + rr(-8, 8), py = y + rr(-4, 4);
    let ok = true;
    for (const [dx, dy] of [[0, 0], [-len, 0], [-len / 2, 0], [10, 0], [0, -20], [-len, -12], [0, 12], [-len, 12], [-len / 2, -18], [-len / 2, 14], [-len * 1.25, 3], [10, -16]]) if (!inPath(seaPath, px + dx, py + dy)) { ok = false; break; }
    if (!ok) continue;
    waves.push([px, py, len, rand()]);
  }
  const BIGC = { 'Chicago': 4.2, 'New York': 4.4, 'Toronto': 3.8, 'Montréal': 3.6, 'Detroit': 3.6, 'Philadelphia': 3.4, 'Washington,  D.C.': 3.4, 'Boston': 3.4 };
  const cities = DATA['america-places'].features.map(f => ({ name: fixName(f.properties.name), p: proj(f.geometry.coordinates) }))
    .filter(c => c.p && c.p[0] > 4 && c.p[0] < W - 4 && c.p[1] > 4 && c.p[1] < H - 4);
  // names: drawn on the labels layer, and wiped from the ground beneath
  const names = [];
  const lab = (t, ll, o = {}) => { const p = proj(ll); if (p) names.push(['t', t, p[0] + (o.dx || 0), p[1] + (o.dy || 0), { size: 10.5, a: 0.82, halo: 3, ...o }]); };
  lab('Superior', [-87.6, 47.65], { rot: -0.08, spacing: 1.2 });
  lab('Michigan', [-87.05, 43.6], { rot: -1.45, spacing: 1.2, size: 9.5 });
  lab('Huron', [-82.25, 44.75], { rot: -0.9, spacing: 1.2, size: 9.5 });
  lab('Erie', [-81.25, 42.18], { rot: -0.42, spacing: 1.2, size: 8.5 });
  lab('Ontario', [-77.9, 43.62], { rot: -0.08, spacing: 1, size: 8.5 });
  lab('Hudson Bay', [-84, 57.8], { size: 12, spacing: 3 });
  lab('Atlantic Ocean', [-66.5, 37.5], { size: 12, spacing: 3.2, rot: -0.35 });
  lab('Mississippi', [-90.65, 37.4], { size: 9, spacing: 1, rot: 1.25, a: 0.62 });
  for (const [name, dx, dy, al] of [['Chicago', -7, 0, 'right'], ['Toronto', 0, -9, 'center'], ['New York', 8, 2, 'left'], ['Montréal', -2, -9, 'right']]) {
    const c = cities.find(c => c.name === name); if (c) names.push(['t', name, c.p[0] + dx, c.p[1] + dy, { size: 9.5, a: 0.72, align: al, font: 'normal', halo: 3 }]);
  }
  names.push(['t', 'OHIO', oc[0], oc[1] + 1, { size: 9, family: "'IM Fell English SC'", font: 'normal', spacing: 1.6, a: 0.95, halo: 3 }]);

  if (which === 'ground') {
    const G = out;
    slate(G, { wash: 0.75 });
    wash(G, seaPath, INK.water, 0.55);
    landTone(G, dryPath);
    wash(G, lakePath, INK.water, 0.85);
    ohioTone(G, ohioPath);
    const ch = layer();
    rub(ch, landPath, dryPath, 6, 0.3, 2.6);
    rub(ch, greatPath, greatPath, 5, 0.18, 2.2);
    for (const [d, a] of [[4, 0.45], [8.5, 0.24], [14, 0.12]]) { const r = offsetRing(landPath, d, 0.75, seaPath); erodeWith(r, tex('break')); put(ch, r, 'source-over', a); }
    for (const [d, a] of [[3.2, 0.42], [6.4, 0.22]]) { const r = offsetRing(greatPath, d, 0.6, greatPath); erodeWith(r, tex('break')); put(ch, r, 'source-over', a); }
    for (const [x, y, len, v] of waves) {
      const a = (0.46 + v * 0.18) * (y > H / 2 + 240 ? 0.6 : 1);
      if (v < 0.4 && inPath(seaPath, x - len * 1.5, y + 7) && inPath(seaPath, x - len * 0.9, y - 4)) seaWave(ch, x - len * 0.62, y + 6, len * 0.6, { dir: 1, a: a * 0.8, w: 0.7 });
      seaWave(ch, x, y, len, { dir: 1, a, w: 0.85 });
    }
    for (const l of landLines) chalk(ch, l.pts, { w: 1.0, a: 0.92, amp: 0.3, wl: 12, closed: true });
    for (const o of great) chalkLines(ch, o.lines, { w: 0.95, a: 0.92, amp: 0.25, wl: 10 });
    for (const o of small) chalkLines(ch, o.lines, { w: 0.55, a: o.area > 60 ? 0.6 : 0.42, amp: 0.15, wl: 8, passes: 1 });
    const major = f => (f.properties.rank || 9) <= 3 || ['Tennessee', 'Wabash', 'Arkansas'].includes(f.properties.name);
    chalkLines(ch, capture(proj, FC(DATA['america-rivers'].features.filter(major))), { w: 0.85, a: 0.72, amp: 0.3, wl: 10, closed: false, passes: 1 });
    chalkLines(ch, capture(proj, FC(DATA['america-rivers'].features.filter(f => !major(f)))), { w: 0.6, a: 0.36, amp: 0.3, wl: 10, closed: false, passes: 1 });
    { const g = layer(); g.save(); g.clip(dryPath, 'evenodd'); dotted(g, capture(proj, DATA['america-states']).filter(l => l.closed), { gap: 2.9, w: 1.0, a: 0.36 }); g.restore(); put(ch, g); }
    rub(ch, ohioPath, ohioPath, 7, 0.4, 2.6);
    chalkLines(ch, ohioLines, { w: 1.15, a: 0.95, amp: 0.2, wl: 10 });
    for (const [k, x, y, s, v] of hills.concat(woods).sort((a, b) => a[2] - b[2])) {
      if (k === 'peak') peak(ch, x, y, s, { a: 0.72, lw: 0.65 });
      else if (k === 'hill') hill(ch, x, y, s, s * (0.42 + v * 0.1), { a: 0.62, bands: 2 });
      else if (k === 'spire') spire(ch, x, y, s, { a: 0.42 });
      else grove(ch, x, y, s, { a: 0.5, fill: 0.12, k: v });
    }
    for (const c of cities) walled(ch, c.p[0], c.p[1], BIGC[c.name] || 2.6, { a: 0.85, w: 0.75 });
    arrivedRing(ch, 2);
    captionCalm(ch, 0.6);
    for (const [, t, x, y, o] of names) chalkText(ch, t, x, y, { ...o, clear: true });
    erodeWith(ch, tex('tooth')); put(G, ch);
    const ac = layer(); drawRing(ac, 2, INK.gold, 0.95); erodeWith(ac, tex('tooth'), 0.45); put(G, ac);
    dust(G, 500);
  }
  if (which === 'labels') { const g = layer(); for (const [, t, x, y, o] of names) chalkText(g, t, x, y, o); erodeWith(g, tex('tooth'), 0.8); put(out, g); }
  if (which === 'sheen') { const g = layer(); drawRing(g, 2, '#ffffff', 1); erodeWith(g, tex('tooth'), 0.45); put(out, g); }
}

// ================================================================ 3. Ohio, between two waters
function level3(out, which) {
  const proj = lensPt(3), cx = W / 2, cy = H / 2;
  const BOX = [-88.5, 35.0, -77.0, 45.5];
  const boxPath = toPath([boxRing(proj, BOX)]), outPath = unionPath(frameRect(), boxPath);
  const inBox = (g, f) => { g.save(); g.clip(boxPath); f(); g.restore(); };
  const outBox = (g, f) => { g.save(); g.clip(outPath, 'evenodd'); f(); g.restore(); };
  const near = f => { const c = d3.geoCentroid(f); return c[0] > -92 && c[0] < -73 && c[1] > 31 && c[1] < 49; };
  const lakesIn = capture(proj, DATA['ohio-lakes']).filter(l => l.closed);
  const lakesOut = capture(proj, FC(DATA['america-lakes'].features.filter(near))).filter(l => l.closed && Math.abs(ringArea(l.pts)) > 4);
  const lakeInPath = toPath(lakesIn), lakeOutPath = toPath(lakesOut);
  const isWater = (x, y) => (inPath(boxPath, x, y) ? inPath(lakeInPath, x, y) : inPath(lakeOutPath, x, y));
  const shoreIn = capture(proj, outlinesOff(DATA['ohio-lakes'], BOX)), shoreOut = lakesOut;
  const ohio = DATA['ohio-states'].features.find(f => f.properties.name === 'Ohio');
  const ohioLines = capture(proj, ohio).filter(l => l.closed), ohioPath = toPath(ohioLines);
  const ohioRiver = capture(proj, FC(DATA['ohio-rivers'].features.filter(f => f.properties.name === 'Ohio')));
  const riversIn = capture(proj, FC(DATA['ohio-rivers'].features.filter(f => f.properties.name !== 'Ohio')));
  const riversOut = capture(proj, FC(DATA['america-rivers'].features.filter(near)));
  const GLACIER = [[-84.82, 39.10], [-84.45, 39.03], [-84.05, 38.98], [-83.75, 39.03], [-83.4, 39.22], [-83.05, 39.33], [-82.78, 39.52], [-82.56, 39.78], [-82.36, 40.03], [-82.06, 40.34], [-81.75, 40.58], [-81.4, 40.74], [-81.0, 40.84], [-80.52, 40.93]];
  const ung = toPath([GLACIER.concat([[-78.5, 40.95], [-78.5, 33.5], [-86, 33.5], [-86, 39.1]]).map(p => proj(p))]);
  const towns = DATA['ohio-places'].features.map(f => ({ name: fixName(f.properties.name), p: proj(f.geometry.coordinates), f }))
    .filter(t => t.p && t.p[0] > 4 && t.p[0] < W - 4 && t.p[1] > 4 && t.p[1] < H - 4);
  const SKIP = new Set(['Covington', 'Windsor', 'Beaver Falls']);
  const cityPts = towns.map(t => t.p);
  const riverPts = ohioRiver.concat(riversIn).flatMap(l => resample(l.pts, 3, false));
  const nearRiver = (x, y, d) => riverPts.some(p => Math.abs(p[0] - x) < d && Math.abs(p[1] - y) < d);
  // placements: furrowed plots and groves on the till plains, banded hills and groves on the plateau
  const plains = [];
  for (let y = 0; y < H; y += 12.5) for (let x = 0; x < W; x += 12.5) {
    const px = x + 6.25, py = y + 6.25;
    if (!inPath(ohioPath, px, py) || inPath(ung, px, py) || isWater(px, py)) continue;
    if (cityPts.some(p => Math.hypot(p[0] - px, p[1] - py) < 10) || nearRiver(px, py, 5)) continue;
    const n = fbm(px / 34 + 3, py / 34, 2), r = rand(), hz = rand() < 0.5;
    if (r < (n - 0.5) * 0.9) plains.push(['furrow', px, py, hz]);
    else if (r > 0.965) plains.push(['grove', px + rr(-2, 2), py + rr(-2, 2), Math.floor(rand() * 4)]);
  }
  const plateau = [];
  for (let row = 0, y = 0; y < H; row++, y += 9) for (let x = 4 + (row % 2) * 7; x < W; x += rr(12, 16.5)) {
    const px = x + rr(-1.5, 1.5), py = y + (vn(x / 30, row * 1.7) - 0.5) * 4;
    if (!inPath(ung, px, py) || isWater(px, py) || nearRiver(px, py, 4.5)) continue;
    const inside = inPath(ohioPath, px, py) && inPath(ohioPath, px, py - 6);
    if (!inside && rand() < 0.72) continue;
    if (cityPts.some(p => Math.hypot(p[0] - px, p[1] - py) < 10)) continue;
    if (fbm(px / 25, py / 25 + 4, 2) < 0.45 && rand() < 0.7) { if (inside && rand() < 0.4) plateau.push(['grove', px, py, inside, Math.floor(rand() * 4)]); continue; }
    plateau.push(['hill', px, py, inside, rr(9, 16), rr(0.38, 0.5)]);
  }
  const curls = [[-82.3, 41.86], [-81.55, 42.1], [-80.8, 42.27], [-82.95, 41.72], [-79.9, 42.55]].map(ll => proj(ll)).filter(Boolean);
  // names
  const names = [];
  const tl = (name, dx, dy, o = {}) => { const t = towns.find(t => t.name === name); if (t) names.push(['t', name, t.p[0] + dx, t.p[1] + dy, { size: 10.5, font: 'normal', a: 0.85, halo: 3.2, ...o }]); };
  tl('Columbus', 22, 17, { size: 12.5, family: "'IM Fell English SC'", spacing: 1.4, a: 0.95, align: 'left', halo: 4 });
  tl('Cleveland', 2, -16, { size: 11 }); tl('Cincinnati', 0, 15, { size: 11 });
  tl('Toledo', -8, -1, { align: 'right' }); tl('Dayton', -8, 0, { align: 'right' }); tl('Akron', 8, 2, { align: 'left' });
  tl('Detroit', 2, -15, { a: 0.55, size: 10 }); tl('Pittsburgh', -9, 1, { a: 0.55, size: 10, align: 'right' });
  names.push(['p', 'LAKE ERIE', lineAlong(proj, [[-82.75, 41.78], [-81.9, 41.92], [-81.15, 42.15], [-80.45, 42.42]], 2), { size: 11, family: "'IM Fell English SC'", font: 'normal', spacing: 4.2, a: 0.85, halo: 3 }]);
  { const x0 = proj([-84.15, 38.8])[0], x1 = proj([-83.25, 38.7])[0]; let seg = smoothLine(runAlong(ohioRiver, p => p[0] > x0 && p[0] < x1), 8);
    if (seg.length > 4) { if (seg[0][0] > seg[seg.length - 1][0]) seg = seg.reverse(); names.push(['p', 'Ohio  River', seg, { size: 10.5, spacing: 1.4, a: 0.85, offset: -9, halo: 3 }]); } }
  { const a = proj([-82.98, 39.78]), b = proj([-82.98, 39.3]); let seg = smoothLine(runAlong(riversIn, p => p[1] > a[1] && p[1] < b[1] && Math.abs(p[0] - a[0]) < 14), 8);
    if (seg.length > 4) { if (seg[0][1] < seg[seg.length - 1][1]) seg = seg.reverse(); names.push(['p', 'Scioto', seg, { size: 9.5, spacing: 1, a: 0.75, offset: -6, halo: 3 }]); } }
  const st = (t, ll, o = {}) => { const p = proj(ll); if (p) names.push(['t', t, p[0], p[1], { size: 8.5, family: "'IM Fell English SC'", font: 'normal', spacing: 2.4, a: 0.45, halo: 2.5, ...o }]); };
  st('Indiana', [-85.25, 40.45], { rot: -Math.PI / 2 }); st('Michigan', [-84.4, 42.15]); st('Ontario', [-81.6, 42.8]);
  st('Pennsylvania', [-80.08, 41.05], { rot: Math.PI / 2 }); st('West Virginia', [-80.75, 38.75], { rot: -0.6 }); st('Kentucky', [-84.0, 38.25]);

  if (which === 'ground') {
    const G = out;
    slate(G, { wash: 0.75 });
    const waterMask = layer(); waterMask.fillStyle = '#fff';
    inBox(waterMask, () => waterMask.fill(lakeInPath, 'evenodd')); outBox(waterMask, () => waterMask.fill(lakeOutPath, 'evenodd'));
    { const t = layer(); t.fillStyle = INK.land; t.fillRect(-40, -40, W + 80, H + 80); put(t, waterMask, 'destination-out'); put(t, tex('mottle'), 'destination-in'); put(G, t, 'source-over', 0.32); }
    { const t = layer(); t.fillStyle = INK.water; t.fillRect(-40, -40, W + 80, H + 80); put(t, waterMask, 'destination-in'); put(t, tex('mottle'), 'destination-in'); put(G, t, 'source-over', 0.85); }
    ohioTone(G, ohioPath);
    const ch = layer();
    rub(ch, ohioPath, ohioPath, 9, 0.32, 3.2);
    { const g = layer(); g.save(); g.clip(ohioPath); g.clip(unionPath(frameRect(), ung), 'evenodd');
      const step = 12.5;
      for (let x = 0; x < W; x += step) for (let y = 0; y < H; y += sr(4, 9)) { const l = sr(3, 7); if (srand() < 0.45) chalk(g, [[x + sr(-0.3, 0.3), y], [x + sr(-0.3, 0.3), y + l]], { w: 0.45, a: 0.5, amp: 0.05, passes: 1, step: 1 }); y += l; }
      for (let y = 0; y < H; y += step) for (let x = 0; x < W; x += sr(4, 9)) { const l = sr(3, 7); if (srand() < 0.45) chalk(g, [[x, y + sr(-0.3, 0.3)], [x + l, y + sr(-0.3, 0.3)]], { w: 0.45, a: 0.5, amp: 0.05, passes: 1, step: 1 }); x += l; }
      g.restore(); put(ch, g, 'source-over', 0.42); }
    for (const it of plains) {
      if (it[0] === 'furrow') { const [, px, py, hz] = it; for (let k = -2; k <= 2; k++) { const o = k * 1.7; chalk(ch, hz ? [[px - 3.6, py + o], [px + 3.6, py + o]] : [[px + o, py - 3.6], [px + o, py + 3.6]], { w: 0.42, a: 0.26, amp: 0.06, passes: 1, step: 1 }); } }
      else grove(ch, it[1], it[2], 2, { a: 0.6, fill: 0.14, k: it[3] });
    }
    for (const it of plateau.slice().sort((a, b) => a[2] - b[2])) {
      if (it[0] === 'hill') hill(ch, it[1], it[2], it[4], it[4] * it[5], { a: it[3] ? 0.6 : 0.2, bands: 2 });
      else grove(ch, it[1], it[2], 2, { a: 0.6, fill: 0.14, k: it[4] });
    }
    inBox(ch, () => chalkLines(ch, shoreIn, { w: 1.05, a: 0.95, amp: 0.25, wl: 10, closed: false }));
    outBox(ch, () => chalkLines(ch, shoreOut, { w: 1.0, a: 0.9, amp: 0.25, wl: 10 }));
    for (const [d, a] of [[4, 0.42], [8.5, 0.22]]) {
      const r = layer(); r.strokeStyle = '#fff'; r.lineWidth = 2 * d + 0.7;
      inBox(r, () => shoreIn.forEach(l => strokePts(r, l.pts, false))); outBox(r, () => shoreOut.forEach(l => strokePts(r, l.pts, true)));
      r.globalCompositeOperation = 'destination-out'; r.lineWidth = 2 * d - 0.7;
      inBox(r, () => shoreIn.forEach(l => strokePts(r, l.pts, false))); outBox(r, () => shoreOut.forEach(l => strokePts(r, l.pts, true)));
      put(r, waterMask, 'destination-in'); erodeWith(r, tex('break')); put(ch, r, 'source-over', a);
    }
    for (const [x, y] of curls) curl(ch, x, y, sr(3.8, 4.8), { a: 0.5, dir: 1, dots: true });
    const ohioRiverW = doubleLine(ch, ohioRiver, 4.6, 2.4, { a: 0.92, amp: 0.25 });
    inBox(ch, () => chalkLines(ch, riversIn, { w: 0.85, a: 0.72, amp: 0.3, wl: 10, closed: false, passes: 1 }));
    outBox(ch, () => chalkLines(ch, riversOut, { w: 0.8, a: 0.5, amp: 0.3, wl: 10, closed: false, passes: 1 }));
    { const g = layer(); chalkLines(g, ohioLines, { w: 1.2, a: 0.95, amp: 0.2, wl: 10 });
      g.globalCompositeOperation = 'destination-out'; g.lineWidth = 9; for (const q of ohioRiverW) strokePts(g, q, false);
      g.lineWidth = 7; shoreIn.forEach(l => strokePts(g, l.pts, false)); put(ch, g); }
    { const g = layer();
      inBox(g, () => dotted(g, capture(proj, outlinesOff(FC(DATA['ohio-states'].features.filter(f => f.properties.name !== 'Ohio')), BOX)), { a: 0.38 }));
      outBox(g, () => dotted(g, capture(proj, FC(DATA['america-states'].features.filter(near))).filter(l => l.closed), { a: 0.38 }));
      g.globalCompositeOperation = 'destination-out'; g.lineWidth = 6; for (const q of ohioRiverW) strokePts(g, q, false);
      put(g, waterMask, 'destination-out'); g.lineWidth = 4; shoreIn.forEach(l => strokePts(g, l.pts, false));
      put(ch, g); }
    { const g = layer(); g.save(); g.clip(ohioPath); dashed(g, capture(proj, DATA['ohio-roads']), { a: 0.38, w: 0.7 }); g.restore();
      g.save(); g.clip(unionPath(frameRect(), ohioPath), 'evenodd'); dashed(g, capture(proj, DATA['ohio-roads']), { a: 0.2, w: 0.65 }); g.restore();
      put(ch, g); }
    const SIZE = { Columbus: [8.5, 8], Cleveland: [6.4, 6], Cincinnati: [6.4, 6], Toledo: [4.4], Dayton: [4.4], Akron: [4.2], Youngstown: [3.6], Canton: [3.6], Detroit: [6, 5], Pittsburgh: [5.4, 5] };
    for (const t of towns) {
      if (SKIP.has(t.name)) continue;
      const inO = d3.geoContains(ohio, t.f.geometry.coordinates), [r, tw] = SIZE[t.name] || [2.5];
      walled(ch, t.p[0], t.p[1], r, { a: inO ? 0.9 : 0.55, w: r > 5 ? 0.9 : 0.7, tower: tw, rub: r > 5 ? 0.12 : 0 });
    }
    arrivedRing(ch, 3);
    captionCalm(ch, 0.5);
    for (const n of names) n[0] === 't' ? chalkText(ch, n[1], n[2], n[3], { ...n[4], clear: true }) : textOnPath(ch, n[1], n[2], { ...n[3], clear: true });
    erodeWith(ch, tex('tooth')); put(G, ch);
    const ac = layer(); drawRing(ac, 3, INK.gold, 0.95); erodeWith(ac, tex('tooth'), 0.45); put(G, ac);
    dust(G, 500);
  }
  if (which === 'labels') { const g = layer(); for (const n of names) n[0] === 't' ? chalkText(g, n[1], n[2], n[3], n[4]) : textOnPath(g, n[1], n[2], n[3]); erodeWith(g, tex('tooth'), 0.8); put(out, g); }
  if (which === 'sheen') { const g = layer(); drawRing(g, 3, '#ffffff', 1); erodeWith(g, tex('tooth'), 0.45); put(out, g); }
}

// ================================================================ 4. Columbus, the walled city
function level4(out, which) {
  const proj = lensPt(4);
  const roads = dedupe(DATA['columbus-roads'].features);
  const ringF = roads.filter(f => f.properties.name === 'I- 270').sort((a, b) => JSON.stringify(b.geometry).length - JSON.stringify(a.geometry).length)[0];
  const ringPts = capture(proj, ringF.geometry).flatMap(l => l.pts);
  const ringPath = toPath([ringPts]), RC = centroidOf(resample(ringPts, 3, true));
  const ringNames = new Set(['I- 270', 'Jack Nicklaus Fwy', 'Outerbelt']);
  const local = capture(proj, FC(roads.filter(f => f.properties.class === 'local')));
  const major = capture(proj, FC(roads.filter(f => f.properties.class === 'major')));
  const fwy = capture(proj, FC(roads.filter(f => f.properties.class === 'interstate' && !ringNames.has(f.properties.name))));
  const bigWater = DATA['columbus-water'].features.filter(f => km2(f) > 0.035);
  const waterLines = capture(proj, FC(bigWater)).filter(l => l.closed), waterPath = toPath(waterLines);
  const reservoirs = toPath(capture(proj, FC(bigWater.filter(f => km2(f) > 0.5 && !/Riv|Crk/.test(f.properties.name)))).filter(l => l.closed));
  // gates where the freeways pass the wall; downtown's towers; the Horseshoe
  const gates = [];
  for (const l of fwy) for (let i = 1; i < l.pts.length; i++) {
    const a = l.pts[i - 1], b = l.pts[i];
    if (inPath(ringPath, a[0], a[1]) !== inPath(ringPath, b[0], b[1])) {
      const m = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      if (gates.every(q => Math.hypot(q[0] - m[0], q[1] - m[1]) > 14)) gates.push([...m, Math.atan2(b[1] - a[1], b[0] - a[0])]);
    }
  }
  const DT = proj([-82.9985, 39.9625]), HS = proj([-83.0197, 40.0017]);
  const names = [];
  const lab = (t, ll, o = {}) => { const p = proj(ll); if (p) names.push(['t', t, p[0], p[1], { size: 10, a: 0.82, halo: 3.4, spacing: 1, ...o }]); };
  lab('Scioto', [-83.03, 39.918], { rot: -1.42, size: 10.5 });
  lab('Olentangy', [-83.028, 40.06], { rot: -1.5, size: 10 });
  lab('Alum Creek', [-82.943, 40.035], { rot: -1.45, size: 9, a: 0.6 });
  lab('Big Walnut', [-82.875, 39.935], { rot: -1.0, size: 9, a: 0.55 });
  { const top = Math.min(...ringPts.map(p => p[1])), arc = resample(ringPts, 2, true).filter(p => p[1] < top + 40 && Math.abs(p[0] - RC[0]) < 98).sort((a, b) => a[0] - b[0]);
    if (arc.length > 10) names.push(['p', 'THE OUTERBELT', smoothLine(resample(arc, 2, false), 10), { size: 8.5, family: "'IM Fell English SC'", font: 'normal', spacing: 3, a: 0.62, offset: -13, halo: 3 }]); }

  if (which === 'ground') {
    const G = out;
    slate(G, { wash: 0.6 });
    innerTone(G);
    cityTone(G, ringPath);
    wash(G, waterPath, INK.water, 0.9);
    const ch = layer();
    rub(ch, ringPath, ringPath, 16, 0.22, 7);
    { const g = layer(); plainLines(g, local, 0.42, 1);
      const m = layer(); m.fillStyle = 'rgba(0,0,0,0.42)'; m.fillRect(-40, -40, W + 80, H + 80); m.fillStyle = '#000'; m.fill(ringPath);
      put(g, m, 'destination-in'); erodeWith(g, tex('tooth'), 0.5); put(ch, g, 'source-over', 0.3); }
    chalkLines(ch, capture(proj, DATA['columbus-streams']), { w: 0.55, a: 0.32, amp: 0.25, wl: 9, closed: false, passes: 1 });
    chalkLines(ch, major, { w: 0.75, a: 0.5, amp: 0.2, wl: 12, closed: false, passes: 1 });
    chalkLines(ch, waterLines, { w: 0.7, a: 0.8, amp: 0.15, wl: 8, passes: 1 });
    { const r = offsetRing(reservoirs, 2.4, 0.55, reservoirs); erodeWith(r, tex('break')); put(ch, r, 'source-over', 0.4); }
    doubleLine(ch, fwy, 2.7, 1.1, { a: 0.85, amp: 0.2 });
    doubleLine(ch, [{ pts: ringPts.concat([ringPts[0]]) }], 5.4, 2.6, { a: 0.95, amp: 0.25, wl: 30 });
    { const rs = resample(ringPts, 2.9, true);
      rs.forEach(([x, y], i) => {
        const dx = x - RC[0], dy = y - RC[1], l = Math.hypot(dx, dy) || 1, ux = dx / l, uy = dy / l, L = i % 6 === 0 ? 3.4 : 2.1;
        chalk(ch, [[x + ux * 3.1, y + uy * 3.1], [x + ux * (3.1 + L), y + uy * (3.1 + L)]], { w: 0.5, a: 0.62, amp: 0, passes: 1, step: 1 });
      });
      const inner = rs.map(([x, y]) => { const dx = x - RC[0], dy = y - RC[1], l = Math.hypot(dx, dy) || 1; return [x - dx / l * 7, y - dy / l * 7]; });
      ch.save(); ch.setLineDash([0.1, 2.7]); ch.lineWidth = 1.05; ch.strokeStyle = INK.chalk; ch.globalAlpha = 0.6; strokePts(ch, inner, true); ch.restore(); }
    ch.save(); ch.globalCompositeOperation = 'destination-out'; ch.beginPath(); ch.ellipse(HS[0], HS[1], 5.5, 4.4, 0, 0, TAU); ch.fill(); ch.restore();
    for (const [rx, ry, a] of [[4.6, 3.6, 0.85], [2.8, 2.1, 0.6]]) chalk(ch, circlePts(HS[0], HS[1], rx, 30, Math.PI / 2 + 0.55, Math.PI / 2 + TAU - 0.55, ry), { w: 0.6, a, amp: 0.05, passes: 1, step: 1 });
    const standing = [];
    for (const [x, y, ang] of gates) { const nx = -Math.sin(ang), ny = Math.cos(ang); for (const s of [-1, 1]) standing.push(['gate', x + nx * s * 4.2, y + ny * s * 4.2 + 2.2]); }
    for (const [dx, dy, w, h, top] of [[-10, 3, 4.6, 13, 'flat'], [-3.6, -1, 5.4, 21, 'step'], [3.2, 1, 4.4, 19, 'spire'], [9.2, -2, 4.4, 15, 'flat'], [-7, 9, 8, 6, 'dome'], [6, 8, 4, 11, 'step']])
      standing.push(['tower', DT[0] + dx, DT[1] + dy + 4, w, h, top]);
    // the towers stand only while the city is seen whole: on the deepest pass plate they would be giant
    // slabs, and Short North's own elevations are about to take over
    if (VIEW.s < 5) standing.sort((a, b) => a[2] - b[2]).forEach(s => s[0] === 'gate' ? tower(ch, s[1], s[2], 2.6, 6.5, 'spire', 0.85) : tower(ch, s[1], s[2], s[3], s[4], s[5], 0.92));
    arrivedRing(ch, 4);
    { const m = layer(); m.save(); m.translate(W / 2, H / 2); m.scale(1, 1.9);   // the county fades toward the plate's edges
      const gr = m.createRadialGradient(0, 0, 200, 0, 0, 300); gr.addColorStop(0, '#000'); gr.addColorStop(1, 'rgba(0,0,0,0.45)');
      m.fillStyle = gr; m.fillRect(-W * 3, -H * 3, W * 6, H * 6); m.restore(); put(ch, m, 'destination-in'); }
    captionCalm(ch, 0.6);
    for (const n of names) n[0] === 't' ? chalkText(ch, n[1], n[2], n[3], { ...n[4], clear: true }) : textOnPath(ch, n[1], n[2], { ...n[3], clear: true });
    erodeWith(ch, tex('tooth')); put(G, ch);
    const ac = layer(); drawRing(ac, 4, INK.gold, 0.95); erodeWith(ac, tex('tooth'), 0.45); put(G, ac);
    dust(G, 500);
  }
  if (which === 'labels') { const g = layer(); for (const n of names) n[0] === 't' ? chalkText(g, n[1], n[2], n[3], n[4]) : textOnPath(g, n[1], n[2], n[3]); erodeWith(g, tex('tooth'), 0.8); put(out, g); }
  if (which === 'sheen') { const g = layer(); drawRing(g, 4, '#ffffff', 1); erodeWith(g, tex('tooth'), 0.45); put(out, g); }
}

// ================================================================ 5. Short North, in the courtyard's oval
// The High Street arches, between these latitudes, about every 46 m; their box for the lamps layer.
const ARCH_LAT = [39.9772, 39.9862], ARCH_STEP = 46 / 111200;
const ARCH_BOX = (() => { const p = lensPt(5), a = p([-83.0068, ARCH_LAT[1] + 0.0004]), b = p([-83.0018, ARCH_LAT[0] - 0.0002]); return [a[0], a[1], b[0] - a[0], b[1] - a[1]]; })();
// "Short North", its initial gold, set low in the oval above the caption.
const NAME_AT = [W / 2 - 70, H / 2 + 160];
const INITIAL_BOX = [NAME_AT[0] - 24, NAME_AT[1] - 26, 30, 44];
function level5(out, which) {
  const proj = lensPt(5);
  const F = FC, greens = DATA['shortnorth-green'].features;
  const lawns = greens.filter(f => ['grass', 'park', 'garden', 'playground'].includes(f.properties.kind));
  const woodsF = greens.filter(f => ['forest', 'wood', 'scrub'].includes(f.properties.kind));
  const pitches = greens.filter(f => f.properties.kind === 'pitch');
  const lawnPath = toPath(capture(proj, F(lawns)).filter(l => l.closed));
  const parkPath = toPath(capture(proj, F(greens.filter(f => f.properties.kind === 'park'))).filter(l => l.closed));
  const woodPath = toPath(capture(proj, F(woodsF)).filter(l => l.closed));
  const pitchLines = capture(proj, F(pitches)).filter(l => l.closed);
  const greenAll = toPath(capture(proj, F(greens)).filter(l => l.closed));
  const pond = DATA['shortnorth-water'].features.filter(f => f.geometry.type.includes('Polygon'));
  const pondLines = capture(proj, F(pond)).filter(l => l.closed), pondPath = toPath(pondLines);
  const ROADW = { motorway: [12, 9], motorway_link: [5.6, 3.6], primary: [9.4, 7], secondary: [8, 5.8], tertiary: [6.6, 4.6], residential: [5.2, 3.4], unclassified: [5.2, 3.4], living_street: [4.4, 2.8], primary_link: [4.6, 3.0], secondary_link: [4.6, 3.0], pedestrian: [4, 2.6] };
  const roads = dedupe(DATA['shortnorth-roads'].features);
  const street = roads.map(f => ({ f, k: f.properties.class, lines: capture(proj, f) })).filter(r => ROADW[r.k] && r.lines.length);
  const service = capture(proj, F(roads.filter(f => f.properties.class === 'service'))).filter(l => polyLen(l.pts) > 24);
  const paths = capture(proj, F(roads.filter(f => ['footway', 'path', 'cycleway', 'track'].includes(f.properties.class))));
  const rail = capture(proj, DATA['shortnorth-rail']);
  const high = street.filter(r => r.f.properties.name === 'North High Street').flatMap(r => r.lines);
  // the arches over High Street, at evenly spaced latitudes
  const arches = [];
  for (let lat = ARCH_LAT[0]; lat <= ARCH_LAT[1]; lat += ARCH_STEP) {
    const y = proj([-83.004, lat])[1], pts = runAlong(high, p => Math.abs(p[1] - y) < 2.5);
    if (pts.length) { const c = centroidOf(pts); arches.push([c[0], c[1]]); }
  }
  // occupancy (roads, buildings, the run of the arches), for placing the trees
  const occupy = document.createElement('canvas'); occupy.width = Math.ceil(W); occupy.height = Math.ceil(H);
  const og = occupy.getContext('2d'); og.strokeStyle = '#fff'; og.fillStyle = '#fff'; og.lineCap = 'round';
  for (const r of street) { og.lineWidth = ROADW[r.k][0] + 2; for (const l of r.lines) strokePts(og, l.pts, false); }
  og.lineWidth = 3.5; for (const l of service) strokePts(og, l.pts, false);
  for (const l of pitchLines) { og.beginPath(); l.pts.forEach((p, i) => i ? og.lineTo(p[0], p[1]) : og.moveTo(p[0], p[1])); og.closePath(); og.fill(); }
  // buildings: houses and shopfronts stand up; the big ones lie in plan
  const SKIP = new Set(['garage', 'garages', 'shed', 'roof', 'carport', 'parking']);
  const elev = [], plan = [];
  const highPts = high.flatMap(l => resample(l.pts, 4, false));
  const nearHigh = (x, y) => highPts.some(p => Math.abs(p[0] - x) < 26 && Math.abs(p[1] - y) < 26);
  for (const f of DATA['shortnorth-buildings'].features) {
    const k = f.properties.kind; if (SKIP.has(k)) continue;
    const ls = capture(proj, f).filter(l => l.closed); if (!ls.length) continue;
    const pts = ls[0].pts, A = Math.abs(ringArea(pts)), c = centroidOf(pts);
    if (c[0] < -10 || c[0] > W + 10 || c[1] < -10 || c[1] > H + 14) continue;
    og.beginPath(); pts.forEach((p, i) => i ? og.lineTo(p[0], p[1]) : og.moveTo(p[0], p[1])); og.closePath(); og.fill(); og.lineWidth = 2.5; og.stroke();
    const xs = pts.map(p => p[0]), wpx = Math.max(...xs) - Math.min(...xs);
    const church = k === 'church' || /Church|Cathedral/.test(f.properties.name || '');
    if (church) { plan.push([pts, c, true]); continue; }
    if (A < 70 && (k === 'house' || k === 'detached' || k === 'terrace' || k === 'yes' || k === 'residential' || k === 'semidetached_house')) {
      const w = clamp(Math.sqrt(A) * 1.05, 4.2, 8.2); elev.push(['house', c[0], c[1] + w * 0.32, w, (vn(c[0] * 3.1, c[1] * 2.3) * 7.77) % 1]);
    } else if (A < 420 && nearHigh(c[0], c[1])) {
      const w = clamp(wpx * 0.92, 5, 22); elev.push(['shop', c[0], c[1] + 3.2, w, clamp(6 + Math.sqrt(A) * 0.28, 7.5, 13), (vn(c[0] * 2.7, c[1] * 1.9) * 9.3) % 1]);
    } else if (A < 160) {
      const w = clamp(Math.sqrt(A) * 1.05, 5, 11); elev.push(['house', c[0], c[1] + w * 0.32, w, (vn(c[0] * 3.1, c[1] * 2.3) * 7.77) % 1]);
    } else plan.push([pts, c, false]);
  }
  const occ = (() => { const d = og.getImageData(0, 0, occupy.width, occupy.height).data; return (x, y) => { const X = Math.round(x), Y = Math.round(y); return X < 0 || Y < 0 || X >= occupy.width || Y >= occupy.height ? true : d[(Y * occupy.width + X) * 4 + 3] > 20; }; })();
  const trees = [];
  for (const f of DATA['shortnorth-trees'].features) {
    const p = proj(f.geometry.coordinates); if (!p || p[0] < -6 || p[0] > W + 6 || p[1] < -6 || p[1] > H + 10) continue;
    if (occ(p[0], p[1]) && !inPath(greenAll, p[0], p[1])) continue;
    if (trees.some(t => Math.abs(t[0] - p[0]) < 4.4 && Math.abs(t[1] - p[1]) < 4.4)) continue;
    trees.push([p[0], p[1], rr(2.3, 3.2)]);
  }
  for (const [x, y, r] of trees) elev.push(['tree', x, y + r * 0.4, r]);
  for (const [x, y] of arches) elev.push(['arch', x, y + 0.5]);
  elev.sort((a, b) => a[2] - b[2]);
  const [ox, oy] = [W / 2, H / 2 + OVAL[2]];
  // names
  const names = [];
  { let seg = runAlong(high, p => p[1] > oy - 266 && p[1] < oy - 170); if (seg.length > 8) { if (seg[0][1] > seg[seg.length - 1][1]) seg = seg.reverse(); names.push(['p', 'High Street', smoothLine(seg, 6), { size: 9.5, family: "'IM Fell English SC'", font: 'normal', spacing: 2.6, a: 0.88, halo: 3.4 }]); } }
  names.push(['t', 'HORT NORTH', NAME_AT[0] + 2, NAME_AT[1] + 2, { size: 17, family: "'IM Fell English SC'", font: 'normal', spacing: 4.2, a: 0.97, align: 'left', halo: 10 }]);
  names.push(['t', '© OpenStreetMap contributors', W / 2 + 150, H / 2 + 236, { size: 7.5, font: 'normal', a: 0.62, align: 'right', halo: 2.5 }]);
  const initial = g => chalkText(g, 'S', NAME_AT[0], NAME_AT[1] - 1, { size: 34, family: "'Uncial Antiqua'", font: 'normal', align: 'right', a: 1, color: INK.gold });

  if (which === 'ground') {
    const G = out;
    slate(G, { wash: 0.55, streak: 0.03 });
    innerTone(G);
    wash(G, lawnPath, '#93a6a8', 0.24);
    wash(G, woodPath, '#0f1a1f', 0.35);
    wash(G, pondPath, INK.water, 0.9);
    const ch = layer();
    rub(ch, parkPath, parkPath, 9, 0.22, 3.5);
    { const g = layer(); g.save(); g.clip(parkPath, 'evenodd'); g.fillStyle = INK.chalk;   // a park's lawn, stippled like the courtyard's dotted ring
      for (let i = 0; i < 9000; i++) { const x = sr(0, W), y = sr(0, H); g.globalAlpha = sr(0.25, 0.6); g.beginPath(); g.arc(x, y, sr(0.35, 0.65), 0, TAU); g.fill(); }
      g.restore(); put(ch, g, 'source-over', 0.6); }
    chalkLines(ch, capture(proj, F(greens.filter(f => f.properties.kind === 'park'))).filter(l => l.closed), { w: 0.6, a: 0.55, amp: 0.25, wl: 12, passes: 1 });
    for (const l of pondLines) chalk(ch, l.pts, { w: 0.7, a: 0.85, amp: 0.1, closed: true, passes: 1 });
    for (const l of pitchLines) chalk(ch, l.pts, { w: 0.6, a: 0.65, amp: 0.1, closed: true, passes: 1 });
    { const t = layer(), sets = street.map(r => ({ r, ws: r.lines.map(l => wobble(resample(l.pts, 1.5, false), 0.25, 18, srand() * 100)) }));
      t.strokeStyle = INK.chalk;
      for (const { r, ws } of sets) { t.lineWidth = ROADW[r.k][0]; for (const q of ws) strokePts(t, q, false); }
      t.globalCompositeOperation = 'destination-out';
      for (const { r, ws } of sets) { t.lineWidth = ROADW[r.k][1]; for (const q of ws) strokePts(t, q, false); }
      t.globalCompositeOperation = 'source-over'; t.setLineDash([2.6, 3]); t.lineWidth = 0.55; t.globalAlpha = 0.45;
      for (const { r, ws } of sets) if (r.k === 'primary' || r.k === 'secondary' || r.k === 'motorway') for (const q of ws) strokePts(t, q, false);
      put(ch, t, 'source-over', 0.85); }
    plainLines(ch, service, 0.5, 0.26);
    { const g = layer(); g.save(); g.clip(greenAll, 'evenodd'); dotted(g, paths, { gap: 2.4, w: 0.9, a: 0.5 }); g.restore(); put(ch, g); }
    for (const l of rail) {   // the railway as a ladder
      const q = resample(l.pts, 2.2, false); chalk(ch, l.pts, { w: 0.6, a: 0.6, passes: 1, amp: 0.1 });
      for (let i = 1; i < q.length - 1; i++) { const a = q[i - 1], b = q[i + 1], L = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1, nx = -(b[1] - a[1]) / L * 1.6, ny = (b[0] - a[0]) / L * 1.6; chalk(ch, [[q[i][0] - nx, q[i][1] - ny], [q[i][0] + nx, q[i][1] + ny]], { w: 0.4, a: 0.45, amp: 0, passes: 1, step: 1 }); }
    }
    for (const [pts, c, church] of plan) {
      chalk(ch, pts, { w: 0.65, a: 0.72, amp: 0.15, closed: true, passes: 1 });
      const p = new Path2D(); pts.forEach((q, i) => i ? p.lineTo(q[0], q[1]) : p.moveTo(q[0], q[1])); p.closePath();
      hatch(ch, p, { gap: 2.1, a: 0.28, w: 0.45, angle: 0.8, box: [c[0] - 70, c[1] - 70, c[0] + 70, c[1] + 70], broken: 0.15 });
      if (church) elev.push(['spire', c[0], c[1] + 3]);
    }
    for (const e of elev.sort((a, b) => a[2] - b[2])) {
      if (e[0] === 'tree') tree(ch, e[1], e[2], e[3], { a: 0.7, fill: 0.06, w: 0.66 });
      else if (e[0] === 'house') house(ch, e[1], e[2], e[3], e[4]);
      else if (e[0] === 'shop') shopfront(ch, e[1], e[2], e[3], e[4], e[5]);
      else if (e[0] === 'spire') tower(ch, e[1], e[2], 4, 11, 'spire', 0.85);
      else if (e[0] === 'arch') {   // an arch spanning the street, standing up in elevation
        const [x, y] = [e[1], e[2]], half = 6.2, h = 7.5;
        ch.save(); ch.globalCompositeOperation = 'destination-out'; ch.lineWidth = 2.2; ch.beginPath(); ch.ellipse(x, y, half, h, 0, Math.PI, TAU); ch.stroke(); ch.restore();
        chalk(ch, circlePts(x, y, half, 24, Math.PI, TAU, h), { w: 0.75, a: 0.9, amp: 0.05, passes: 1, step: 1 });
        chalk(ch, circlePts(x, y, half - 1.1, 20, Math.PI, TAU, h - 1.1), { w: 0.4, a: 0.55, amp: 0.05, passes: 1, step: 1 });
      }
    }
    // the oval: the drawing fades outside it; the ring is the gold ring of the scale before, arrived
    { const m = layer(); m.save(); m.translate(ox, oy); m.scale(1, OVAL[1] / OVAL[0]);
      const gr = m.createRadialGradient(0, 0, OVAL[0] - 6, 0, 0, (OVAL[0] - 6) * 1.13);
      gr.addColorStop(0, '#000'); gr.addColorStop(1, 'rgba(0,0,0,0.3)'); m.fillStyle = gr; m.fillRect(-W * 3, -H * 3, W * 6, H * 6); m.restore();
      put(ch, m, 'destination-in'); }
    { const ring = (k, n = 240) => circlePts(ox, oy, OVAL[0] + k, n, 0, TAU, OVAL[1] + k);
      ch.save(); ch.globalCompositeOperation = 'destination-out'; ch.lineWidth = 15; strokePts(ch, ring(4), true); ch.restore();
      chalk(ch, ring(-1), { w: 1.25, a: 0.9, amp: 0.4, wl: 30, closed: true });
      chalk(ch, ring(9), { w: 0.7, a: 0.6, amp: 0.4, wl: 30, closed: true });
      ch.save(); ch.fillStyle = INK.chalk; for (const [x, y] of ring(4, 180)) { ch.globalAlpha = sr(0.5, 0.8); ch.beginPath(); ch.arc(x, y, 0.75, 0, TAU); ch.fill(); } ch.restore(); }
    for (const n of names) n[0] === 't' ? chalkText(ch, n[1], n[2], n[3], { ...n[4], clear: n[4].halo > 0 }) : textOnPath(ch, n[1], n[2], { ...n[3], clear: true });
    chalkText(ch, 'S', NAME_AT[0], NAME_AT[1] - 1, { size: 34, family: "'Uncial Antiqua'", font: 'normal', align: 'right', halo: 8, clear: true });
    erodeWith(ch, tex('tooth'), 0.85); put(G, ch);
    dust(G, 400);
  }
  if (which === 'labels') { const g = layer(); for (const n of names) n[0] === 't' ? chalkText(g, n[1], n[2], n[3], n[4]) : textOnPath(g, n[1], n[2], n[3]); erodeWith(g, tex('tooth'), 0.8); put(out, g);
    const ac = layer(); initial(ac); erodeWith(ac, tex('tooth'), 0.3); put(out, ac); }
  if (which === 'sheen') { const g = layer(); initial(g); g.globalCompositeOperation = 'source-in'; g.fillStyle = '#fff'; g.fillRect(-W, -H, 3 * W, 3 * H); erodeWith(g, tex('tooth'), 0.3); put(out, g); }
  if (which === 'lamps') {   // each arch's lamps, small dots of light along it
    const g = layer(); g.fillStyle = '#fff6dc';
    for (const [x, y] of arches) for (let i = 0; i <= 8; i++) { const t = Math.PI + Math.PI * i / 8; g.globalAlpha = 0.9; g.beginPath(); g.arc(x + Math.cos(t) * 6.2, y + 0.5 + Math.sin(t) * 7.5, 0.55, 0, TAU); g.fill(); }
    g.globalAlpha = 1; put(out, g);
  }
}

// ================================================================ the page
const LEVEL_FNS = [null, level1, level2, level3, level4, level5];
const { level, layer: which } = Kit.params();
let canvas = null;
// A level's ground blends, toward its own edges, into the level before it drawn at the same place and
// scale: by the time a closer plate is opaque but not yet covering the screen, its border is the very
// picture behind it, so no box shows. (Pass plates are the same level magnified and need none.)
const BAND = 0.13;
function edgeMask() {
  const m = layer(), bx = W * BAND, by = H * BAND;
  m.fillStyle = '#fff'; m.fillRect(0, 0, W, H);
  const ramp = (x0, y0, x1, y1) => { const gr = m.createLinearGradient(x0, y0, x1, y1);
    gr.addColorStop(0, 'rgba(255,255,255,0)'); gr.addColorStop(0.5, 'rgba(255,255,255,0.5)'); gr.addColorStop(1, '#fff'); return gr; };
  m.globalCompositeOperation = 'destination-in';
  for (const [x0, y0, x1, y1, rx, ry, rw, rh] of [[0, 0, bx, 0, 0, 0, bx, H], [W, 0, W - bx, 0, W - bx, 0, bx, H], [0, 0, 0, by, 0, 0, W, by], [0, H, 0, H - by, 0, H - by, W, by]]) {
    const t = layer(); t.fillStyle = '#fff'; t.fillRect(0, 0, W, H); t.clearRect(rx, ry, rw, rh); t.fillStyle = ramp(x0, y0, x1, y1); t.fillRect(rx, ry, rw, rh);
    put(m, t, 'destination-in');
  }
  return m;
}
function paint() {
  if (!canvas) {
    canvas = document.createElement('canvas'); canvas.width = DW; canvas.height = DH;
    Object.assign(canvas.style, { position: 'absolute', left: '0', top: '0', width: DW + 'px', height: DH + 'px', display: 'block' });
    document.body.appendChild(canvas);
  }
  const g = canvas.getContext('2d'); g.setTransform(1, 0, 0, 1, 0, 0); g.clearRect(0, 0, DW, DH);
  if (which === 'ground' && level > 1 && VIEW.s === 1) {
    const V0 = VIEW, R = RATIO(level - 1);
    VIEW = { s: R, cx: W / 2, cy: H / 2 };
    resetRandom(level - 1); resetTextures(); MASKS.clear();
    const behind = layer(); LEVEL_FNS[level - 1](behind, 'ground');
    VIEW = V0; resetRandom(level); resetTextures(); MASKS.clear();
    const own = layer(); LEVEL_FNS[level](own, 'ground');
    put(own, edgeMask(), 'destination-in');
    put(g, behind); put(g, own);
    return;
  }
  resetRandom(level); resetTextures();
  viewTransform(g); g.lineCap = 'round'; g.lineJoin = 'round';
  LEVEL_FNS[level](g, which);
}
if (which !== 'none') addEventListener('DOMContentLoaded', async () => {
  try {
    await Promise.all(["20px 'IM Fell English SC'", "italic 20px 'IM Fell English'", "20px 'IM Fell English'", "20px 'Uncial Antiqua'"].map(f => document.fonts.load(f)));
    if (which === 'ground' && level > 1) await load(level - 1);
    await load(level);
    paint();
    window.platesReady = true;
  } catch (e) { console.error(e && e.stack || e); window.platesReady = 'failed: ' + (e && e.message || e); return; }
  // A pass plate: the exporter magnifies the root by a CSS transform. Paint again at that
  // magnification, and undo the transform on the canvas, so the plate is drawn, not blown up.
  let done = '';
  new MutationObserver(() => {
    const t = document.documentElement.style.transform;
    const m = /translate\(\s*([-+\d.e]+)px,\s*([-+\d.e]+)px\)\s*scale\(\s*([-+\d.e]+)\s*\)/.exec(t || '');
    if (!m || t === done) return;
    done = t;
    const tx = +m[1], ty = +m[2], s = +m[3];
    if (!(s > 1.0001)) return;
    VIEW = { s, cx: (DW / 2 - tx) / s / DPR, cy: (DH / 2 - ty) / s / DPR };
    paint();
    Object.assign(canvas.style, { transformOrigin: '0 0', transform: `scale(${1 / s}) translate(${-tx}px, ${-ty}px)` });
  }).observe(document.documentElement, { attributes: true, attributeFilter: ['style'] });
});
