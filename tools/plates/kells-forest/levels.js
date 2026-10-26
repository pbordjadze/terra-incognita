// Aisling's Forest: the five levels, and the page contract (../kit.js) around them.
//
// Every level draws through Kit's lens (one orthographic projection centred on the film's end), so the plates nest
// exactly and a dive lands each plate's coasts and streets on the wider one's. Units are 3 plate px; at rest the
// phone shows the plate's middle 390×844 units (SCREEN), and the caption sits on the lowest fifth of that.
'use strict';
const DATA = '../data/';
const SCREEN = [(W - W / 1.4) / 2, (H - H / 1.4) / 2, (W + W / 1.4) / 2, (H + H / 1.4) / 2];
const CAPTION_TOP = SCREEN[3] - (SCREEN[3] - SCREEN[1]) / 5;
const C0 = [W / 2, H / 2];
// The lens in units: Kit.lens drawn at a third of its size, so a point lands where Kit.lens puts it, divided by 3.
const lensU = scalePx => d3.geoOrthographic().rotate([-Kit.TARGET[0], -Kit.TARGET[1]]).translate([W / 2, H / 2])
  .scale(scalePx / S).clipAngle(90).precision(0.03);
const ZOOMS = [516, Kit.km(3000), Kit.km(640), Kit.km(41), Kit.km(1.3)];
const loadGeo = n => loadJSON(DATA + n + '.geojson');
const FC = features => ({ type: 'FeatureCollection', features });
// A lon/lat box whose edges follow the meridians and parallels (d3 joins vertices with great circles), wound clockwise.
function geoBox(w, s, e, n, step = 0.1) {
  const ring = [];
  for (let x = w; x < e; x += step) ring.push([x, s]);
  for (let y = s; y < n; y += step) ring.push([e, y]);
  for (let x = e; x > w; x -= step) ring.push([x, n]);
  for (let y = n; y > s; y -= step) ring.push([w, y]);
  ring.push([w, s]);
  return { type: 'Polygon', coordinates: [ring.reverse()] };
}

// The ring of white dots around where the film goes next: the film's circular setting. Its radius is half the
// next plate's resting view, so as the camera dives it opens to frame the plate fading up inside it.
function nextRing(level) {
  if (level >= 5) return null;
  const w = ZOOMS[level - 1], wc = ZOOMS[level]; // the child spans PW * w / wc plate px of this plate
  return PW * w / wc / 1.4 / 2 / S;
}

// The foot of the screen kept calm for the caption: the picture there softened toward one colour, more so lower down.
function calmFoot(col, k) {
  const g = ctx.createLinearGradient(0, CAPTION_TOP - 50, 0, SCREEN[3] + 40);
  g.addColorStop(0, rgba(col, 0)); g.addColorStop(0.55, rgba(col, k * 0.75)); g.addColorStop(1, rgba(col, k));
  ctx.save(); ctx.fillStyle = g; ctx.fillRect(-10, CAPTION_TOP - 50, W + 20, H - CAPTION_TOP + 60); ctx.restore(); css(ctx);
}

// =====================================================================================================
// The leaves that frame each scale: sprays from the screen's upper corners, on their own layers so they sway.
function sprayFrame(anchor, reach) { return [anchor[0] - reach, anchor[1] - reach, reach * 2, reach * 2].map(v => Math.round(v * S)); }
function spray(anchor, dir, n, reach, o = {}) {
  const lt = o.tiles || leafTiles(), end = [anchor[0] + Math.cos(dir) * reach * 0.75, anchor[1] + Math.sin(dir) * reach * 0.75];
  const mid = [(anchor[0] + end[0]) / 2 + Math.sin(dir) * reach * 0.12, (anchor[1] + end[1]) / 2 - Math.cos(dir) * reach * 0.12];
  twig([anchor, mid, end], o.twig ?? 1.6);
  for (let k = 0; k < n; k++) {
    const t = Math.pow((k + 0.5) / n, 0.8), x = anchor[0] + (end[0] - anchor[0]) * t + (mid[0] - (anchor[0] + end[0]) / 2) * 4 * t * (1 - t),
      y = anchor[1] + (end[1] - anchor[1]) * t + (mid[1] - (anchor[1] + end[1]) / 2) * 4 * t * (1 - t);
    const side = k % 2 ? 1 : -1, len = R(...(o.len || [30, 42])) * (1 - 0.35 * t);
    leaf(x, y, dir + side * R(0.5, 1.1), len, len * R(0.44, 0.54), pick(lt), { shA: 0.32 });
  }
}

// =====================================================================================================
// Level 1: the whole Earth, an orb seen through a gap in the canopy, the way Aisling looks out of the forest.
const RAD = ZOOMS[0] / S;
function worldGround(d) {
  const proj = lensU(ZOOMS[0]), C = C0, land = d.land;
  const orb = new Path2D(); orb.arc(C[0], C[1], RAD, 0, Math.PI * 2);
  const ob = [C[0] - RAD - 4, C[1] - RAD - 4, C[0] + RAD + 4, C[1] + RAD + 4];
  paperGround();
  // the forest floor: moss in ochre, olive and chartreuse, freckled, darker toward the plate's edges
  wash(c => c.fillRect(-10, -10, W + 20, H + 20), { c: ['#8c8a34', '#b08a3a', '#7d8a34'], op: 0.8, mottle: 0.45, edge: 0, gran: 0.1, bloom: 0.3, fb: 0.2 });
  wash(c => c.fillRect(-10, -10, W + 20, H + 20), { c: [P.olive2, P.deep], op: 0.3, mottle: 1.0, edge: 0, gran: 0.12, bloom: 0.5, fb: 0.9 });
  moss([0, 0, W, H], { density: 0.45 });
  const vig = ctx.createRadialGradient(C[0], C[1], RAD * 1.25, C[0], C[1], H * 0.62);
  vig.addColorStop(0, 'rgba(60,50,20,0)'); vig.addColorStop(1, 'rgba(45,38,14,0.5)');
  ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = vig; ctx.fillRect(-10, -10, W + 20, H + 20); ctx.globalCompositeOperation = 'source-over';
  const glow = ctx.createRadialGradient(C[0], C[1], RAD * 0.9, C[0], C[1], RAD * 1.85);
  glow.addColorStop(0, 'rgba(250,240,170,0.55)'); glow.addColorStop(1, 'rgba(250,240,170,0)');
  ctx.globalCompositeOperation = 'screen'; ctx.fillStyle = glow; ctx.fillRect(-10, -10, W + 20, H + 20); ctx.globalCompositeOperation = 'source-over';
  // the orb: a disc of pale water light
  ctx.fillStyle = P.paper; ctx.fill(orb);
  const landP = geoP(proj, land, { amp: 0.4, sc: 6, seed: 2 }), landSharp = geoP(proj, land);
  wash(orb, { c: [P.water, '#dfe6c2', P.water2], op: 0.9, edge: 1.6, blur: 9, mottle: 0.25, gran: 0.12, bloom: 0.1, cut: landSharp, bbox: ob });
  // the gyres: the oceans' great slow spirals (their white dots shine on the glints layer)
  for (const g of GYRES) ink(geoP(proj, geoSpiral(g[0], 1.2, g[1], g[2], 200, g[3])), { w: 0.7, col: P.jade, alpha: 0.7, passes: 1 });
  ripple(landSharp, 2.4, { alpha: 0.55, clip: orb }); ripple(landSharp, 5, { alpha: 0.35, clip: orb, lw: 0.4 }); ripple(landSharp, 8, { alpha: 0.25, clip: orb, lw: 0.4, dash: 3 });
  // the land, coloured by what grows on it
  const cbuf = biomeBuffer(proj, d.biome, [Math.floor(ob[0]), Math.floor(ob[1]), Math.ceil(ob[2]), Math.ceil(ob[3])]);
  wash(landP, { cbuf, op: 0.88, edge: 1.1, blur: 2.2, mottle: 0.35, gran: 0.3, bloom: 0.25, cvar: 0.22, bbox: ob });
  // the wooded lands, freckled with tiny round crowns
  ctx.save(); ctx.clip(landSharp);
  for (let y = ob[1]; y < ob[3]; y += 2.6) for (let x = ob[0]; x < ob[2]; x += 2.6) {
    const jx = x + R(-1, 1), jy = y + R(-1, 1), ll = proj.invert([jx, jy]), keep = rnd(), r = R(0.55, 0.95), col = rnd() < 0.15 ? P.ochre : '#4c5a1a', a = R(0.35, 0.6);
    if (!ll || Math.hypot(jx - C[0], jy - C[1]) > RAD - 2) continue;
    const veg = biomeAt(d.biome, ll[0], ll[1], 0); if (veg < 0.45 || keep > veg * 0.75) continue;
    ctx.fillStyle = rgba(col, a); ctx.beginPath(); ctx.arc(jx, jy, r, 0, 7); ctx.fill();
  }
  ctx.restore();
  for (const m of d.extra.ranges) if (m.rank <= 2 || /Sierra Madre|Mackenzie|Cordillera|Brooks|Alaska Range/.test(m.name)) range(proj, m.geometry, { sp: 1.3, len: [3, 9], op: 0.5 });
  for (const r of d.extra.rivers) if (r.rank <= 2) ink(geoP(proj, r.geometry), { w: 0.75, col: P.jade, alpha: 0.75, passes: 1 });
  // the sphere, darker away from the light (upper left)
  ctx.save(); ctx.clip(orb); ctx.globalCompositeOperation = 'multiply';
  const sh = ctx.createRadialGradient(C[0] - RAD * 0.35, C[1] - RAD * 0.4, RAD * 0.2, C[0] - RAD * 0.1, C[1] - RAD * 0.1, RAD * 1.25);
  sh.addColorStop(0, 'rgba(255,255,255,0)'); sh.addColorStop(0.62, 'rgba(200,215,190,0.25)'); sh.addColorStop(1, 'rgba(70,105,90,0.62)');
  ctx.fillStyle = sh; ctx.fillRect(ob[0], ob[1], ob[2] - ob[0], ob[3] - ob[1]); ctx.restore(); css(ctx);
  hatch(shadowBand(landSharp, -1.8, -2.1), { ang: 40, sp: 1.15, len: [1.5, 4], w: 0.32, alpha: 0.5, col: P.umber, bbox: ob });
  ink(landP, { w: 0.55, alpha: 0.85 });
  // the rim: a fine ink circle and a ring of dots
  ink(orb, { w: 1.1, alpha: 0.9 });
  dots(spiralPts(C[0], C[1], RAD + 5, RAD + 5, 1), { gap: 4, dotW: 1.5, dotCol: 'rgba(255,250,225,0.9)' });
  // the wreath: leaves drawn one by one, three rings deep, tips over the rim
  const lt = leafTiles();
  const ringLeaves = (rr, n, lenR, mode) => {
    const a0 = R(0, 6.28);
    for (let k = 0; k < n; k++) {
      const a = a0 + k / n * Math.PI * 2 + R(-0.06, 0.06), r = rr + R(-5, 5), len = R(...lenR), turn = R(-0.6, 0.6), flip = rnd() < 0.5 ? 1 : -1, free = R(0, 6.28);
      const ang = mode === 'in' ? a + Math.PI + turn * 0.8 : mode === 'tan' ? a + flip * Math.PI / 2 + turn : free;
      leaf(C[0] + r * Math.cos(a), C[1] + r * Math.sin(a), ang, len, len * R(0.44, 0.56), pick(lt));
    }
  };
  ringLeaves(RAD + 64, 54, [40, 54], 'any');
  ringLeaves(RAD + 46, 48, [36, 50], 'tan');
  ringLeaves(RAD + 31, 42, [30, 42], 'in');
  // the canopy overhead, its edge hanging into the top of the screen: the gap the orb is seen through
  canopy(0, SCREEN[1] + 64, { n: 34 });
  // on the moss between: fungi on a fallen twig, white flowers, ferns; the bottom stays quiet for the caption
  twig([[96, 300], [134, 290], [168, 270], [188, 252]], 2.2); twig([[134, 290], [146, 264]], 1.3);
  bracket(156, 279, 12, -0.45); bracket(166, 269, 9, -0.55); bracket(148, 286, 7, -0.4);
  for (const [x, y, s] of [[112, 336, 20], [130, 344, 13], [100, 348, 9]]) agaric(x, y, s, { dots: 6 });
  for (const [x, y] of [[146, 352], [160, 360], [92, 372], [150, 372], [400, 360], [416, 372], [386, 378]]) flower(x, y, R(2.4, 3.2));
  fern(84, 400, 70, -1.3, { curl: -0.4 }); fern(470, 392, 62, -1.9, { curl: 0.35 }); fern(450, 404, 50, -1.6, { curl: 0.25 });
  for (const [x, y, a] of [[100, 900, 1.0], [210, 930, 2.5], [360, 910, 4.0], [450, 950, 0.3], [150, 1000, 5.5], [300, 1010, 1.6]]) leaf(x, y, a, R(22, 28), R(10, 13), pick(lt), { inkA: 0.6 });
}
// A canopy of leaves along the top, its lower edge ragged and hung with a fringe of leaves.
function canopy(y0, y1, o = {}) {
  const lt = o.tiles || leafTiles(), edgeY = x => y1 - 22 + 34 * (fbm2(x / 46, 3, 5) - 0.5);
  wash(c => { c.beginPath(); c.moveTo(-10, -10); c.lineTo(W + 10, -10); for (let x = W + 10; x >= -10; x -= 4) c.lineTo(x, edgeY(x)); c.closePath(); c.fill(); },
    { c: ['#5f6224', '#7a6f2c', '#4e5520'], over: true, op: 0.92, edge: 0.5, blur: 4, mottle: 0.5, gran: 0.1, bbox: [-10, -10, W + 10, y1 + 30] });
  for (let k = 0; k < (o.n ?? 40); k++) { // leaves in the canopy's shade, dimmed
    const x = R(-10, W + 10), y = R(-10, y1 - 30), len = R(30, 44), a = R(0, 6.28), t = pick([tiles.olive, tiles.khaki, tiles.deep, tiles.olive]);
    if (y > edgeY(x) - 18) continue;
    leaf(x, y, a, len, len * R(0.44, 0.55), t, { inkA: 0.55, shA: 0.2 });
  }
  ctx.save(); ctx.globalCompositeOperation = 'multiply'; const sh = ctx.createLinearGradient(0, 0, 0, y1); sh.addColorStop(0, 'rgba(50,52,20,0.6)'); sh.addColorStop(0.75, 'rgba(60,60,25,0.12)'); sh.addColorStop(1, 'rgba(60,60,25,0)'); ctx.fillStyle = sh; ctx.fillRect(-10, -10, W + 20, y1 + 10); ctx.restore(); css(ctx);
  for (let x = -14; x < W + 14; x += 0) { // the fringe, hanging over the moss
    const len = R(30, 46), dy = R(4, 16), a = Math.PI / 2 + R(-0.75, 0.75), t = pick(lt); x += R(9, 15);
    leaf(x, edgeY(x) - dy, a, len, len * R(0.44, 0.55), t);
  }
}
const GYRES = [[[-43, 30], 21, 3.2, 1], [[-14, -27], 17, 2.6, -1], [[-148, 30], 22, 3, 1], [[-110, -28], 20, 2.6, -1]];
function worldGlints(d) {
  const proj = lensU(ZOOMS[0]);
  ctx.save(); ctx.beginPath(); ctx.arc(C0[0], C0[1], RAD - 1, 0, 7); ctx.clip();
  for (const g of GYRES) { const pts = geoP2pts(proj, geoSpiral(g[0], 1.2, g[1] - 1.6, g[2], 200 + g[3] * 6, g[3])); dots(pts, { gap: 3.2, dotW: 1.15 }); }
  ctx.restore();
  ringDots(1);
}
function ringDots(level, o = {}) {
  const r = nextRing(level); if (!r) return;
  dots(spiralPts(C0[0], C0[1], r, r, 1, 0.3), { gap: o.gap ?? 2.6, dotW: o.w ?? 1.25, dotCol: o.col || 'rgba(255,252,236,0.95)' });
  ctx.save(); ctx.strokeStyle = rgba(P.ink, 0.35); ctx.lineWidth = 0.35; ctx.beginPath(); ctx.arc(C0[0], C0[1], r + 1.4, 0, 7); ctx.stroke(); ctx.restore();
}
function worldLabels(d) {
  const proj = lensU(ZOOMS[0]);
  const at = (lon, lat) => proj([lon, lat]);
  { const g = at(-43, 30), ap = []; for (let a = 205; a <= 335; a += 5) ap.push([g[0] + 50 * Math.cos(a * Math.PI / 180), g[1] + 4 + 46 * Math.sin(a * Math.PI / 180)]); textOnPath('Atlantic', ap, { size: 11, ls: 0.3, col: '#3f6a5c' }); }
  { const g = at(-124, 18); if (g) label('Pacific', g[0] + 4, g[1], { size: 11, ls: 0.3, col: '#3f6a5c', ang: 0.5 }); }
  const am = at(-100, 49); label('America', am[0] - 10, am[1], { size: 11, ls: 0.18, col: P.umber });
}
function worldFoxglove() { foxglove(452, 852, 170, { lean: -0.12, bells: 16, side: -1 }); fern(440, 852, 56, -1.85, { curl: 0.3 }); }

// =====================================================================================================
// shared map drawing
function geoDest([lon, lat], brg, dist) {
  const r = Math.PI / 180, f1 = lat * r, dd = dist * r, b = brg * r;
  const f2 = Math.asin(Math.sin(f1) * Math.cos(dd) + Math.cos(f1) * Math.sin(dd) * Math.cos(b));
  const l2 = lon * r + Math.atan2(Math.sin(b) * Math.sin(dd) * Math.cos(f1), Math.cos(dd) - Math.sin(f1) * Math.sin(f2));
  return [l2 / r, f2 / r];
}
function geoSpiral(c, r0, r1, turns, b0 = 0, dir = 1) {
  const pts = [], N = Math.round(turns * 90);
  for (let k = 0; k <= N; k++) { const t = k / N; pts.push(geoDest(c, b0 + dir * t * turns * 360, r1 + (r0 - r1) * t)); }
  return { type: 'LineString', coordinates: pts };
}
function geoP2pts(proj, geom) { return geom.coordinates.map(c => proj(c)).filter(Boolean); }
function biomeAt(b, lon, lat, ch) { const x = Math.floor((lon + 180) / 360 * b.w), y = Math.min(b.h - 1, Math.max(0, Math.floor((90 - lat) / 180 * b.h))); return b.data[(y * b.w + ((x % b.w) + b.w) % b.w) * 4 + ch] / 255; }
// The colours of the land, one per unit: olive where it is wooded, ochre to rust where it is dry, cream where it is
// ice, chartreuse between.
function biomeBuffer(proj, biome, region, o = {}) {
  const w = Math.ceil(W), h = Math.ceil(H), data = new Float32Array(w * h * 3);
  const cChart = hex(o.base || P.chart2), cOlive = hex(o.oliveCol || '#6f7f26'), cOchre = hex(P.ochre2), cRust = hex(P.rust2), cIce = hex('#f6f2e2'), cTundra = hex('#c9c48a');
  const samp = (lon, lat, ch) => {
    const x = (lon + 180) / 360 * biome.w - 0.5, y = (90 - lat) / 180 * biome.h - 0.5;
    const i = Math.floor(x), j = Math.max(0, Math.min(biome.h - 2, Math.floor(y))), fx = x - i, fy = y - j;
    const g = (ii, jj) => biome.data[((jj * biome.w) + ((ii % biome.w) + biome.w) % biome.w) * 4 + ch] / 255;
    return (g(i, j) * (1 - fx) + g(i + 1, j) * fx) * (1 - fy) + (g(i, j + 1) * (1 - fx) + g(i + 1, j + 1) * fx) * fy;
  };
  const [x0, y0, x1, y1] = region.map(Math.round);
  for (let y = Math.max(0, y0); y < Math.min(h, y1); y++) for (let x = Math.max(0, x0); x < Math.min(w, x1); x++) {
    const ll = proj.invert([x + 0.5, y + 0.5]); if (!ll || !isFinite(ll[0])) continue;
    const [lon, lat] = ll; let veg = 0, arid = 0, ice = 0;
    for (const [dx, dy] of [[0, 0], [0.5, 0], [-0.5, 0], [0, 0.5], [0, -0.5]]) { veg += samp(lon + dx, lat + dy, 0); arid += samp(lon + dx, lat + dy, 1); ice += samp(lon + dx, lat + dy, 2); }
    veg /= 5; arid /= 5; ice /= 5;
    let c = cChart;
    c = lerp3(c, cTundra, smooth(52, 66, Math.abs(lat)) * (1 - veg) * 0.8);
    c = lerp3(c, cOlive, Math.pow(veg, 0.75) * (o.olive ?? 0.9));
    if (o.warm) c = lerp3(c, cOchre, o.warm * smooth(0.45, 0.75, fbm2(lon * 0.35, lat * 0.35, 9)));
    c = lerp3(c, cOchre, smooth(0.08, 0.5, arid) * 0.9);
    c = lerp3(c, cRust, smooth(0.5, 0.95, arid) * 0.55);
    c = lerp3(c, cIce, smooth(0.25, 0.7, ice));
    const oi = (y * w + x) * 3; data[oi] = c[0]; data[oi + 1] = c[1]; data[oi + 2] = c[2];
  }
  return { w, h, data };
}
// Thin contour lines in the water, `d` units off a coast.
function ripple(shape, d, o = {}) {
  reset(RX); RX.strokeStyle = '#fff'; RX.lineJoin = 'round';
  RX.lineWidth = 2 * d + (o.lw ?? 0.5); RX.stroke(shape);
  RX.globalCompositeOperation = 'destination-out'; RX.lineWidth = Math.max(0.01, 2 * d - (o.lw ?? 0.5)); RX.stroke(shape);
  if (o.inside) { RX.globalCompositeOperation = 'destination-in'; fillShape(RX, shape); RX.globalCompositeOperation = 'destination-out'; } else fillShape(RX, shape);
  if (o.clip) { reset(BX); BX.fillStyle = '#fff'; fillShape(BX, o.clip); RX.setTransform(1, 0, 0, 1, 0, 0); RX.globalCompositeOperation = 'destination-in'; RX.drawImage(BCV, 0, 0); base(RX); }
  if (o.dash) {
    RX.globalCompositeOperation = 'destination-out'; RX.fillStyle = '#fff';
    for (let x = -H; x < W + H; x += o.dash) { RX.beginPath(); RX.moveTo(x, -10); RX.lineTo(x + o.dash * 0.45, -10); RX.lineTo(x + o.dash * 0.45 + H * 0.6, H + 10); RX.lineTo(x + H * 0.6, H + 10); RX.fill(); }
  }
  RX.setTransform(1, 0, 0, 1, 0, 0); RX.globalCompositeOperation = 'source-in'; RX.fillStyle = o.col || P.jade; RX.fillRect(0, 0, PW, PH);
  ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.globalCompositeOperation = o.comp || 'multiply'; ctx.globalAlpha = o.alpha ?? 0.6; ctx.drawImage(RCV, 0, 0); css(ctx);
}
function axisAngle(proj, geom) {
  const pts = []; d3.geoPath(proj, { moveTo: (x, y) => pts.push([x, y]), lineTo: (x, y) => pts.push([x, y]), closePath() {}, arc() {} })(geom);
  if (pts.length < 3) return null;
  let mx = 0, my = 0; pts.forEach(p => { mx += p[0]; my += p[1]; }); mx /= pts.length; my /= pts.length;
  let sxx = 0, syy = 0, sxy = 0; pts.forEach(([x, y]) => { sxx += (x - mx) ** 2; syy += (y - my) ** 2; sxy += (x - mx) * (y - my); });
  return { ang: 0.5 * Math.atan2(2 * sxy, sxx - syy) * 180 / Math.PI, cx: mx, cy: my, pts };
}
// A mountain range as sweeping bands: a rust wash, long striations along its axis, darker on the shadow side.
function range(proj, geom, o = {}) {
  const ax = axisAngle(proj, geom); if (!ax) return;
  const p = geoP(proj, geom, { amp: o.amp ?? 1.2, sc: 10, seed: 5 }), bb = bboxOf(ax.pts, 6);
  wash(p, { c: o.c || [P.ochre, P.rust2, P.rust], op: o.op ?? 0.55, edge: 1.2, blur: 1.6, mottle: 0.4, bbox: bb });
  hatch(p, { ang: ax.ang, sp: o.sp ?? 1.5, len: o.len || [4, 14], gap: [0.5, 2], w: o.w ?? 0.38, alpha: o.alpha ?? 0.7, col: o.col || P.umber, bbox: bb, bow: 0.8 });
  hatch(shadowBand(p, -2.2, -2.6), { ang: ax.ang + 70, sp: 1.1, len: [1.5, 3.5], w: 0.35, alpha: 0.6, col: P.ink, bbox: bb });
}
// Moss: a sponge of tiny freckles in olive, ochre and dark green, denser where the big field is high.
function moss(area, o = {}) {
  const n = Math.round((area[2] - area[0]) * (area[3] - area[1]) * (o.density ?? 0.5));
  const cols = o.cols || ['#4d5219', '#6b6f24', '#a3872e', '#c3a646', '#3b3c12'];
  for (let k = 0; k < n; k++) {
    const x = R(area[0], area[2]), y = R(area[1], area[3]), keep = rnd(), col = pick(cols), a = R(0.18, 0.45), r = R(0.35, 1.1);
    if (keep > 0.35 + fieldP(FB2p, x, y) * 0.65) continue;
    ctx.fillStyle = rgba(col, a); ctx.beginPath(); ctx.arc(x, y, r, 0, 7); ctx.fill();
  }
}
function city(x, y, o = {}) { // a ring-walled place, in plan
  const r = o.r ?? 3.2;
  ctx.save(); ctx.lineCap = 'round';
  ctx.fillStyle = rgba(P.paper, 0.85); ctx.beginPath(); ctx.arc(x, y, r, 0, 7); ctx.fill();
  ctx.strokeStyle = P.ink; ctx.lineWidth = 0.6; ctx.stroke();
  ctx.fillStyle = P.rust; ctx.beginPath(); ctx.arc(x, y, r * 0.38, 0, 7); ctx.fill();
  ctx.setLineDash([0.1, 1.6]); ctx.lineWidth = 0.9; ctx.strokeStyle = rgba(P.umber, 0.8); ctx.beginPath(); ctx.arc(x, y, r + 2.2, 0, 7); ctx.stroke();
  ctx.restore();
}

// =====================================================================================================
// Level 2: North America, the Great Lakes as five pools of light in the woods, Ohio on the shore of the shallowest.
function americaProj() { return lensU(ZOOMS[1]); }
async function americaLoad() {
  const [land, lakes, rivers, states, borders, topo, biome] = await Promise.all(['america-land', 'america-lakes', 'america-rivers', 'america-states'].map(loadGeo)
    .concat([loadJSON('data/na-borders.geojson'), Kit.json(DATA + 'land-50m.json'), loadImageData('data/biome.png')]));
  return { land, lakes, rivers, states, borders, land50: topojson.feature(topo, topo.objects.land), biome };
}
function americaShapes(d) {
  if (d.shapes) return d.shapes;
  const proj = americaProj(), path = d3.geoPath(proj), area = f => path.area(f);
  const box10 = geoBox(-170, 5, -50, 75, 0.25); // where the 10 m land is
  const land10P = geoP(proj, d.land), land50P = geoP(proj, d.land50), boxP = geoP(proj, box10);
  const fill50Outside = c => { c.save(); const clip = new Path2D(); clip.rect(-20, -20, W + 40, H + 40); clip.addPath(boxP); c.clip(clip, 'evenodd'); c.fill(land50P, 'nonzero'); c.restore(); };
  const big = d.lakes.features.filter(f => area(f) > 14), small = d.lakes.features.filter(f => { const a = area(f); return a > 0.5 && a <= 14; });
  const app = [[-86.9, 33.4], [-85.3, 34.5], [-83.9, 35.4], [-82.3, 36.3], [-80.9, 37.4], [-79.8, 38.4], [-78.9, 39.4], [-77.9, 40.3], [-76.6, 40.9], [-75.3, 41.4], [-74.3, 42.1]];
  const ne = [[-73.7, 42.4], [-72.9, 43.5], [-72.0, 44.2], [-70.9, 44.6], [-69.6, 45.3], [-68.6, 46.0]];
  const landP10 = geoP(proj, d.land, { amp: 0.6, sc: 7, seed: 3 }), landP50 = geoP(proj, d.land50, { amp: 0.6, sc: 7, seed: 3 });
  const landWobbly = c => { c.fill(landP10, 'nonzero'); c.save(); const clip = new Path2D(); clip.rect(-20, -20, W + 40, H + 40); clip.addPath(boxP); c.clip(clip, 'evenodd'); c.fill(landP50, 'nonzero'); c.restore(); };
  d.shapes = { proj, path, landSharp: land10P, landFill: c => { c.fill(land10P, 'nonzero'); fill50Outside(c); }, landWobbly, landP: landP10, landP50, boxP, big, small,
    lakesSharp: geoP(proj, FC(big)), lakesP: geoP(proj, FC(big), { amp: 0.35, sc: 5, seed: 4 }), app, ne,
    ohio: d.states.features.find(f => f.properties.name === 'Ohio') };
  return d.shapes;
}
const SEA_SPIRALS_U = [[[424, 822], 2.4, 2.4, 1], [[-88.2, 26.2], 2.4, 2.1, -1]];
const seaSpirals = proj => SEA_SPIRALS_U.map(([c, ...r]) => [c[0] > 0 ? proj.invert(c) : c, ...r]);
function americaGround(d) {
  const S2 = americaShapes(d), { proj, landSharp, landP, lakesSharp, lakesP, landFill, landWobbly } = S2, frame = c => c.fillRect(-10, -10, W + 20, H + 20);
  paperGround();
  wash(frame, { c: [P.water, '#dde5bf', P.water2], op: 0.85, edge: 1.4, blur: 6, mottle: 0.28, gran: 0.12, cut: landFill });
  for (const [c, r1, t, dir] of seaSpirals(proj)) ink(geoP(proj, geoSpiral(c, 0.3, r1, t, 30, dir)), { w: 0.65, col: P.jade, alpha: 0.65, passes: 1 });
  ripple(landSharp, 2.6, { alpha: 0.55 }); ripple(landSharp, 5.5, { alpha: 0.35, lw: 0.4 }); ripple(landSharp, 9, { alpha: 0.25, lw: 0.4, dash: 3.4 });
  const cbuf = biomeBuffer(proj, d.biome, [0, 0, W, H], { olive: 0.55, oliveCol: '#7f8c2c', warm: 0.45 });
  wash(landWobbly, { cbuf, op: 0.85, edge: 1.0, blur: 2.5, mottle: 0.35, gran: 0.22, bloom: 0.25, cvar: 0.22, cut: lakesSharp });
  wash(lakesP, { c: [P.water, '#e9ecd0', P.water2], op: 0.8, edge: 1.8, blur: 4, mottle: 0.2, gran: 0.1 });
  ripple(lakesSharp, 2.2, { inside: true, alpha: 0.5 }); ripple(lakesSharp, 4.6, { inside: true, alpha: 0.32, lw: 0.4 });
  for (const f of S2.small) { // the north's countless small lakes, freckles of light
    const c = S2.path.centroid(f), rr = Math.max(0.8, Math.sqrt(S2.path.area(f) / Math.PI)), rot = R(0, 3); if (!isFinite(c[0])) continue;
    ctx.save(); ctx.fillStyle = rgba('#eef0d8', 0.9); ctx.strokeStyle = rgba(P.jade, 0.7); ctx.lineWidth = 0.4; ctx.beginPath(); ctx.ellipse(c[0], c[1], rr * 1.15, rr * 0.85, rot, 0, 7); ctx.fill(); ctx.stroke(); ctx.restore();
  }
  // woods: groves of round trees where the land is wooded, spruces in the north, Ohio left open
  const okLand = maskOf(c => { landFill(c); c.globalCompositeOperation = 'destination-out'; c.fill(lakesSharp); c.lineWidth = 7; c.stroke(lakesSharp); c.stroke(landSharp); });
  const appArea = maskOf(c => { c.lineWidth = 40; c.lineCap = 'round'; c.stroke(linePath(smoothPts(S2.app.map(p => proj(p)), 3))); c.lineWidth = 26; c.stroke(linePath(smoothPts(S2.ne.map(p => proj(p)), 3))); });
  const tT = [tiles.olive, tiles.olive, tiles.khaki, tiles.chart, tiles.ochre];
  for (let y = 4, row = 0; y < H; y += 7.4, row++) for (let x = (row % 2) * 3.2; x < W; x += 6.4) {
    const tx = x + R(-0.8, 0.8), ty = y + R(-0.8, 0.8), keep = rnd(), kind = rnd(), h = R(6, 8), r = R(1.7, 2.3), t1 = pick([tiles.olive, tiles.deep, tiles.olive]), t2 = pick(tT);
    if (!okLand(tx, ty) || appArea(tx, ty)) continue;
    const ll = proj.invert([tx, ty]); if (!ll) continue;
    const veg = biomeAt(d.biome, ll[0], ll[1], 0); if (veg < 0.5) continue;
    const grove = fbm2(tx / 34, ty / 34, 17) + (veg - 0.5) * 0.3 - 0.3 * Math.exp(-((tx - C0[0]) ** 2 + (ty - C0[1]) ** 2) / 2600);
    if (grove < 0.6 || keep > 0.93) continue;
    if (ll[1] > 46.5 && kind < 0.8) spruce(tx, ty + 2, h, t1); else roundTree(tx, ty + 2, r, t2, { inkA: 0.55 });
  }
  hillChain(S2.app.map(c => proj(c)), { w: 19, h: 8.5, rows: 2, rowGap: 1.3, step: 0.72, tiles: [tiles.ochre, tiles.straw, tiles.ochre, tiles.khaki, tiles.rust, tiles.straw] });
  hillChain(S2.ne.map(c => proj(c)), { w: 15, h: 7, rows: 2 });
  for (const f of d.rivers.features) { const rk = f.properties.rank ?? 6; ink(geoP(proj, f, { amp: 0.3, sc: 5, seed: 7 }), { w: rk <= 3 ? 1.1 : rk <= 5 ? 0.8 : 0.55, col: P.jade, alpha: 0.8, passes: 1 }); }
  hatch(c => { landFill(c); c.globalCompositeOperation = 'destination-out'; c.translate(-1.8, -2.2); landFill(c); }, { ang: 40, sp: 1.1, len: [1.4, 3.6], w: 0.32, alpha: 0.5, col: P.umber });
  hatch(c => { c.fill(lakesSharp); c.globalCompositeOperation = 'destination-out'; c.translate(1.8, 2.2); c.fill(lakesSharp); }, { ang: 40, sp: 1.1, len: [1.4, 3.6], w: 0.3, alpha: 0.45, col: P.jade });
  ink(landP, { w: 0.6, alpha: 0.85 }); ink(lakesP, { w: 0.55, alpha: 0.8 });
  strokeMasked(c => { c.lineWidth = 0.6; c.lineJoin = 'round'; c.stroke(S2.landP50); }, c => { c.fillRect(-20, -20, W + 40, H + 40); c.globalCompositeOperation = 'destination-out'; c.fill(S2.boxP); }, { col: rgba(P.ink, 0.85) });
  strokeMasked(c => { c.lineWidth = 0.9; c.lineCap = 'round'; c.setLineDash([0.1, 2.4]); c.stroke(geoP(proj, d.borders)); }, c => { c.lineWidth = 3; c.stroke(landSharp); c.stroke(lakesSharp); }, { out: true, col: rgba(P.umber, 0.75) });
  // Ohio, warmed and ringed in rust
  const ohP = geoP(proj, S2.ohio, { amp: 0.4, sc: 5, seed: 8 });
  wash(ohP, { c: [P.straw, P.ochre2], over: true, op: 0.45, edge: 1.2, blur: 2, mottle: 0.3, cut: lakesSharp, bbox: [C0[0] - 60, C0[1] - 60, C0[0] + 60, C0[1] + 60] });
  hatch(rimBand(ohP, 2.4), { ang: -30, sp: 0.9, len: [1, 2.5], w: 0.32, alpha: 0.6, col: P.rust, bbox: [C0[0] - 60, C0[1] - 60, C0[0] + 60, C0[1] + 60] });
  ctx.save(); ctx.setLineDash([0.1, 2.1]); ctx.lineCap = 'round'; ctx.strokeStyle = P.rust; ctx.lineWidth = 1.25; ctx.stroke(ohP); ctx.restore();
  for (const [lon, lat] of AM_CITIES.map(c => [c[1], c[2]])) { const p = proj([lon, lat]); city(p[0], p[1], { r: 2.6 }); }
}
const AM_CITIES = [['Chicago', -87.65, 41.85, 'l'], ['Detroit', -83.05, 42.33, 'l'], ['New York', -74.0, 40.71, 'l'], ['Washington', -77.04, 38.9, 'r'],
  ['Atlanta', -84.39, 33.75, 'r'], ['St. Louis', -90.2, 38.63, 'r'], ['Montréal', -73.57, 45.5, 'l'], ['Nashville', -86.78, 36.16, 'l']];
function americaLabels(d) {
  const { proj } = americaShapes(d);
  for (const [n, lon, lat, side] of AM_CITIES) { const p = proj([lon, lat]); label(n, p[0] + (side === 'r' ? 7 : side === 'l' ? -7 : 0), p[1] - (side === 'u' ? 9 : 0.5), { size: 9.5, align: side === 'r' ? 'left' : side === 'l' ? 'right' : 'center', ls: 0.04 }); }
  const lk = (n, lon, lat, ang = 0, size = 10) => { const p = proj([lon, lat]); label(n, p[0], p[1], { size, ang, col: P.lakeInk, ls: 0.12, haloCol: rgba('#eef0d8', 0.7) }); };
  lk('Superior', -87.6, 47.65, -0.12); lk('Michigan', -87.05, 43.5, -1.45, 9.5); lk('Huron', -82.25, 44.75, 0.95, 9.5); lk('Erie', -80.75, 42.3, -0.55, 9.5); lk('Ontario', -77.85, 43.62, -0.12, 9.5);
  const sea = (n, lon, lat, size = 11, ang = 0) => { const p = proj([lon, lat]); label(n, p[0], p[1], { size, ang, col: P.lakeInk, ls: 0.3, haloCol: rgba(P.water, 0.6) }); };
  { const p = [436, 700]; label('Atlantic', p[0], p[1], { size: 12, ang: -0.45, col: P.lakeInk, ls: 0.3, haloCol: rgba(P.water, 0.6) }); } sea('Gulf of Mexico', -89.0, 27.4, 10.5); sea('Hudson Bay', -84.5, 57.6, 10.5);
  const ap = proj([-80.2, 37.2]); label('Appalachians', ap[0] - 6, ap[1] + 16, { size: 9, ang: -0.62, col: P.umber, ls: 0.22 });
  const oh = proj([-83.3, 40.75]); label('Ohio', oh[0] - 20, oh[1] - 4, { size: 13, col: P.rust, ls: 0.16 });
}
function americaGlints(d) {
  const { proj } = americaShapes(d);
  for (const [c, r1, t, dir] of seaSpirals(proj)) dots(geoP2pts(proj, geoSpiral(c, 0.3, r1 - 0.35, t, 36, dir)), { gap: 3, dotW: 1.1 });
  ringDots(2);
}
// A curragh under a red sail, the film's boat, small on the Atlantic.
function curragh(x, y, s) {
  ctx.save(); ctx.translate(x, y); ctx.lineJoin = 'round'; ctx.lineCap = 'round';
  ctx.strokeStyle = rgba(P.white, 0.9); ctx.lineWidth = 0.6; ctx.setLineDash([0.1, 1.6]);
  ctx.beginPath(); ctx.moveTo(-s * 0.9, s * 0.18); ctx.quadraticCurveTo(-s * 1.6, s * 0.3, -s * 2.4, s * 0.12); ctx.stroke(); ctx.setLineDash([]);
  const hull = new Path2D(); hull.moveTo(-s * 0.7, 0); hull.quadraticCurveTo(0, s * 0.32, s * 0.75, -s * 0.06); hull.lineTo(s * 0.5, s * 0.02); hull.quadraticCurveTo(0, s * 0.12, -s * 0.55, s * 0.02); hull.closePath();
  ctx.fillStyle = '#3a2a16'; ctx.fill(hull); ctx.strokeStyle = P.ink; ctx.lineWidth = 0.4; ctx.stroke(hull);
  ctx.beginPath(); ctx.moveTo(0, 0.1); ctx.lineTo(0, -s * 1.15); ctx.lineWidth = 0.5; ctx.stroke();
  const sail = new Path2D(); sail.moveTo(-s * 0.05, -s * 1.05); sail.quadraticCurveTo(s * 0.55, -s * 0.75, s * 0.48, -s * 0.18); sail.lineTo(-s * 0.4, -s * 0.18); sail.quadraticCurveTo(-s * 0.32, -s * 0.6, -s * 0.05, -s * 1.05); sail.closePath();
  const gr = ctx.createLinearGradient(-s * 0.4, 0, s * 0.5, 0); gr.addColorStop(0, '#e2553a'); gr.addColorStop(1, '#a9271a');
  ctx.fillStyle = gr; ctx.fill(sail); ctx.lineWidth = 0.45; ctx.stroke(sail);
  ctx.strokeStyle = rgba(P.white, 0.75); ctx.lineWidth = 0.35; ctx.beginPath(); ctx.arc(s * 0.05, -s * 0.58, s * 0.16, 0, 6); ctx.stroke();
  ctx.restore();
}
const BOAT = [434, 752, 9];

// =====================================================================================================
// Level 3: Ohio, between the lake and the river: flat fields where the ice lay, folded hills where it never came.
const GLACIAL = [[-78.4, 41.95], [-79.2, 41.55], [-79.9, 41.15], [-80.52, 40.86], [-81.1, 40.84], [-81.5, 40.7], [-81.9, 40.56], [-82.15, 40.45], [-82.33, 40.12],
  [-82.55, 39.76], [-82.85, 39.46], [-83.2, 39.2], [-83.55, 38.96], [-83.9, 38.8], [-84.5, 38.78], [-85.2, 38.68], [-85.9, 38.75], [-86.5, 39.1], [-87.2, 39.3]];
function ohioProj() { return lensU(ZOOMS[2]); }
async function ohioLoad() {
  const names = ['ohio-lakes', 'ohio-rivers', 'ohio-states', 'ohio-roads', 'ohio-places', 'america-land', 'america-lakes', 'america-states'];
  const [lakes, rivers, states, roads, places, naLand, naLakes, naStates] = await Promise.all(names.map(loadGeo));
  const [ring, biome] = await Promise.all([loadJSON('data/columbus-ring.geojson'), loadImageData('data/biome.png')]);
  return { lakes, rivers, states, roads, places, naLand, naLakes, naStates, ring, biome };
}
function ohioShapes(d) {
  if (d.shapes) return d.shapes;
  const proj = ohioProj(), path = d3.geoPath(proj);
  const ohio = d.states.features.find(f => f.properties.name === 'Ohio');
  const box = geoBox(-88.5, 35, -77, 45.5);
  const boxP = geoP(proj, box);
  const farLakes = FC(d.naLakes.features.filter(f => path.area(f) > 20));
  const lakesAll = c => { c.fill(geoP(proj, d.lakes), 'nonzero'); c.save(); const cl = new Path2D(); cl.rect(-20, -20, W + 40, H + 40); cl.addPath(boxP); c.clip(cl, 'evenodd'); c.fill(geoP(proj, farLakes), 'nonzero'); c.restore(); };
  const gpx = smoothPts(GLACIAL.map(c => proj(c)), 3);
  const hillsRegion = c => { c.beginPath(); gpx.forEach((p, i) => i ? c.lineTo(p[0], p[1]) : c.moveTo(p[0], p[1])); c.lineTo(-40, H + 40); c.lineTo(W + 40, H + 40); c.lineTo(W + 40, -40); c.closePath(); c.fill(); };
  d.shapes = { proj, path, ohio, ohSharp: geoP(proj, ohio), ohP: geoP(proj, ohio, { amp: 0.5, sc: 7, seed: 3 }), boxP, lakesAll,
    lakesSharp: geoP(proj, d.lakes), lakesP: geoP(proj, d.lakes, { amp: 0.5, sc: 6, seed: 4 }), farLakesP: geoP(proj, farLakes, { amp: 0.5, sc: 6, seed: 6 }),
    landAll: geoP(proj, d.naLand), gpx, hillsRegion };
  // the hills: a field of distance from where the ice stopped, warped a little; its contours are the bands' crests
  const gw = Math.ceil(W), gh = Math.ceil(H), dist = new Float32Array(gw * gh), inHills = maskOf(hillsRegion);
  const segs = []; for (let i = 1; i < gpx.length; i++) segs.push([gpx[i - 1], gpx[i]]);
  for (let y = 0; y < gh; y++) for (let x = 0; x < gw; x++) {
    let best = 1e9; const px = x + 0.5, py = y + 0.5;
    for (const [a, b] of segs) { const dx = b[0] - a[0], dy = b[1] - a[1], t = Math.max(0, Math.min(1, ((px - a[0]) * dx + (py - a[1]) * dy) / (dx * dx + dy * dy))), qx = a[0] + dx * t - px, qy = a[1] + dy * t - py, dd = qx * qx + qy * qy; if (dd < best) best = dd; }
    const dd = Math.sqrt(best) * (inHills(px, py) ? 1 : -1);
    dist[y * gw + x] = dd + 8 * (fbm2(px / 48, py / 48, 61) - 0.5) + 3.5 * (fbm2(px / 16, py / 16, 62) - 0.5);
  }
  d.shapes.hills = { gw, gh, dist, sp: 11 };
  return d.shapes;
}
// A sheet at 2 px a unit, white where fn(x, y) holds, to fill or clip by.
function sheet(fn) {
  const c = document.createElement('canvas'), w = Math.ceil(W * 2), h = Math.ceil(H * 2); c.width = w; c.height = h;
  const x = c.getContext('2d'), img = x.createImageData(w, h);
  for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) if (fn((i + 0.5) / 2, (j + 0.5) / 2)) img.data[(j * w + i) * 4 + 3] = 255;
  x.putImageData(img, 0, 0);
  return ctx2 => ctx2.drawImage(c, 0, 0, w / 2, h / 2);
}
const BAND_COLS = [P.ochre2, '#c98a34', P.straw, P.rust2, '#d9a447', '#a9a043', '#e2b864', '#b9682c'];
function ohioGround(d) {
  const S3 = ohioShapes(d), { proj, ohSharp, ohP, lakesSharp, lakesP, hillsRegion, gpx } = S3, Hl = S3.hills;
  const dAt = (x, y) => { const i = Math.min(Hl.gw - 1, Math.max(0, Math.floor(x))), j = Math.min(Hl.gh - 1, Math.max(0, Math.floor(y))); return Hl.dist[j * Hl.gw + i]; };
  paperGround();
  // the land in the continent's colours (the same reading of what grows as the plate before), a little paler
  const cbuf = biomeBuffer(proj, d.biome, [0, 0, W, H], { olive: 0.55, oliveCol: '#7f8c2c', warm: 0.45 });
  wash(c => c.fill(S3.landAll, 'nonzero'), { cbuf, op: 0.72, edge: 0.3, blur: 3, mottle: 0.35, gran: 0.1, cvar: 0.2, cut: [ohSharp, S3.lakesAll] });
  // Ohio in daylight: the same land, warmed to straw, as the continent showed it
  wash(ohP, { cbuf, op: 0.72, edge: 1.0, blur: 3, mottle: 0.4, gran: 0.12, bloom: 0.25, cvar: 0.2, cut: lakesSharp });
  wash(ohP, { c: [P.straw, '#e6d27a', P.chart2], over: true, op: 0.4, edge: 1.0, blur: 3, mottle: 0.4, gran: 0.1, c2amt: 0.4, cut: lakesSharp });
  // the till plain: the survey's townships, each its own faint field colour
  const plain = maskOf(c => { c.fill(ohSharp); c.globalCompositeOperation = 'destination-out'; hillsRegion(c); c.fill(lakesSharp); });
  ctx.save(); ctx.clip(ohSharp); ctx.globalCompositeOperation = 'multiply';
  const tw = 8.6, ob = bboxOf([], 0), obb = d3.geoPath(proj).bounds(S3.ohio);
  for (let y = obb[0][1]; y < obb[1][1]; y += tw) for (let x = obb[0][0]; x < obb[1][0]; x += tw) {
    const t = pick([tiles.straw, tiles.chart, tiles.khaki, tiles.lime, tiles.straw, tiles.ochre]), a = R(0.03, 0.09), half = rnd() < 0.4, ang = rnd() < 0.5;
    if (!plain(x + tw / 2, y + tw / 2)) continue;
    ctx.fillStyle = pattern(t); ctx.globalAlpha = a;
    if (half) { if (ang) { ctx.fillRect(x, y, tw / 2, tw); ctx.globalAlpha = a * 0.6; ctx.fillRect(x + tw / 2, y, tw / 2, tw); } else { ctx.fillRect(x, y, tw, tw / 2); ctx.globalAlpha = a * 0.6; ctx.fillRect(x, y + tw / 2, tw, tw / 2); } }
    else ctx.fillRect(x, y, tw, tw);
  }
  ctx.restore(); css(ctx);
  // the old shores of the lake, faint lines on the lake plain
  for (const [dd, a] of [[4, 0.32], [8, 0.26], [12.5, 0.2], [17.5, 0.14]]) ripple(lakesSharp, dd, { alpha: a, lw: 0.35, col: P.umber, dash: 2.6, clip: c => { c.fill(ohSharp); c.globalCompositeOperation = 'destination-out'; hillsRegion(c); } });
  // the hills: bands of ochre, straw and rust between the contours of distance from the ice, vivid in Ohio, pale beyond
  const cIn = { w: Hl.gw, h: Hl.gh, data: new Float32Array(Hl.gw * Hl.gh * 3) }, cols = BAND_COLS.map(hex), pale = ['#d8c891', '#cbbf86'].map(hex);
  const cOut = { w: Hl.gw, h: Hl.gh, data: new Float32Array(Hl.gw * Hl.gh * 3) };
  for (let j = 0; j < Hl.gh; j++) for (let i = 0; i < Hl.gw; i++) {
    const v = Hl.dist[j * Hl.gw + i], k = Math.floor(v / Hl.sp), o = (j * Hl.gw + i) * 3;
    const c = cols[((k % cols.length) + cols.length) % cols.length], pc = pale[((k % 2) + 2) % 2];
    cIn.data[o] = c[0]; cIn.data[o + 1] = c[1]; cIn.data[o + 2] = c[2]; cOut.data[o] = pc[0]; cOut.data[o + 1] = pc[1]; cOut.data[o + 2] = pc[2];
  }
  const inOhioHills = c => { hillsRegion(c); c.globalCompositeOperation = 'destination-in'; c.fill(ohSharp); };
  const inOhioHills2 = inOhioHills;
  wash(inOhioHills2, { cbuf: cIn, over: true, op: 0.62, edge: 0.9, blur: 1.4, mottle: 0.35, gran: 0.12, cvar: 0.14 });
  const nearOhio = c => { c.save(); c.filter = blurPx(16); c.lineWidth = 70; c.lineJoin = 'round'; c.stroke(ohSharp); c.fill(ohSharp); c.restore(); c.filter = 'none'; c.globalCompositeOperation = 'destination-in'; hillsRegion(c); c.fill(S3.landAll, 'nonzero'); c.globalCompositeOperation = 'destination-out'; c.fill(ohSharp); S3.lakesAll(c); };
  wash(nearOhio, { cbuf: cOut, over: true, op: 0.42, edge: 0, blur: 1.4, mottle: 0.3, cvar: 0.1 });
  const shade = sheet((x, y) => { const v = dAt(x, y), f = v / Hl.sp - Math.floor(v / Hl.sp); return v > 0 && f < 0.3; });
  hatch(c => { shade(c); c.globalCompositeOperation = 'destination-in'; inOhioHills(c); }, { ang: 120, sp: 1.0, len: [1, 2.6], w: 0.3, alpha: 0.5, col: P.umber });
  const levels = d3.range(1, 30).map(k => k * Hl.sp);
  const crests = d3.contours().size([Hl.gw, Hl.gh]).thresholds(levels)(Array.from(Hl.dist));
  const crestPath = new Path2D(); for (const cont of crests) for (const poly of cont.coordinates) for (const ring of poly) crestPath.addPath(polyP(ring, { amp: 0.4, sc: 6, seed: cont.value }, true));
  strokeMasked(c => { c.lineWidth = 0.45; c.lineJoin = 'round'; c.stroke(crestPath); }, inOhioHills, { col: rgba(P.umber, 0.78), comp: 'multiply' });
  strokeMasked(c => { c.lineWidth = 0.4; c.lineJoin = 'round'; c.stroke(crestPath); }, nearOhio, { col: rgba(P.umber, 0.32), comp: 'multiply' });
  // where the ice stopped: a dotted line
  ctx.save(); ctx.clip(ohSharp); ctx.setLineDash([0.1, 2.6]); ctx.lineCap = 'round'; ctx.strokeStyle = rgba(P.umber, 0.6); ctx.lineWidth = 0.9; ctx.stroke(linePath(gpx)); ctx.restore(); css(ctx);
  // the lakes: light, contour lines inside their shores
  // the lakes as one sheet of water: the fine lakes inside the data's box, the continent's beyond it, no seam between
  const offEdge = c => { c.fillRect(-20, -20, W + 40, H + 40); c.globalCompositeOperation = 'destination-out'; c.lineWidth = 7; c.stroke(S3.boxP); };
  const lakesWob = c => { c.fill(lakesP, 'nonzero'); c.save(); const cl = new Path2D(); cl.rect(-20, -20, W + 40, H + 40); cl.addPath(S3.boxP); c.clip(cl, 'evenodd'); c.fill(S3.farLakesP, 'nonzero'); c.restore();
    c.save(); c.clip(S3.farLakesP); c.lineWidth = 2; c.stroke(S3.boxP); c.restore(); }; // and the seam along the box closed
  wash(lakesWob, { c: [P.water, '#e9ecd0', P.water2], op: 0.85, edge: 1.6, blur: 5, mottle: 0.22, gran: 0.1 });
  ripple(lakesSharp, 2.6, { inside: true, alpha: 0.5, clip: offEdge }); ripple(lakesSharp, 5.6, { inside: true, alpha: 0.32, lw: 0.4, clip: offEdge }); ripple(lakesSharp, 9.5, { inside: true, alpha: 0.22, lw: 0.4, dash: 3.4, clip: offEdge });
  for (const [c, r] of OH_WAVES) ink(linePath(spiralPts(...proj(c), 0.6, r, 2.3, 0, 1, 0.62)), { w: 0.6, col: P.jade, alpha: 0.6, passes: 1 });
  // small woods: short rows of round trees in the hollows of the hills, and woodlots on the plain
  const inOH = maskOf(inOhioHills);
  for (const cont of d3.contours().size([Hl.gw, Hl.gh]).thresholds(d3.range(0, 30).map(k => (k + 0.62) * Hl.sp))(Array.from(Hl.dist)))
    for (const poly of cont.coordinates) for (const ring of poly) {
      let acc = 0;
      for (let i = 1; i < ring.length; i++) {
        const a = ring[i - 1], b = ring[i], L = Math.hypot(b[0] - a[0], b[1] - a[1]); acc += L; if (acc < 3.4) continue; acc = 0;
        const r = R(1.8, 2.4), t = pick([tiles.olive, tiles.olive, tiles.khaki, tiles.ochre, tiles.rust, tiles.olive]), jx = R(-0.6, 0.6), jy = R(-0.6, 0.6);
        if (!inOH(b[0], b[1]) || fbm2(b[0] / 24, b[1] / 24, 31) < 0.6) continue;
        roundTree(b[0] + jx, b[1] + jy + 2, r, t, { inkA: 0.55 });
      }
    }
  for (let k = 0; k < 300; k++) {
    const x = R(60, W - 60), y = R(300, 820), n = Math.floor(R(3, 8)), pts = []; for (let q = 0; q < n; q++) pts.push([x + R(-4, 4), y + R(-2.5, 2.5), R(1.3, 1.8), pick([tiles.olive, tiles.khaki, tiles.olive])]);
    if (!plain(x, y) || fbm2(x / 30, y / 30, 33) < 0.53) continue;
    pts.sort((a, b) => a[1] - b[1]).forEach(([px, py, r, t]) => { if (plain(px, py)) roundTree(px, py, r, t, { inkA: 0.5 }); });
  }
  // groves in the neighbouring woods, as on the continent, thinning toward the caption
  const nbLand = maskOf(c => { c.fill(S3.landAll, 'nonzero'); c.globalCompositeOperation = 'destination-out'; c.lineWidth = 8; c.stroke(ohSharp); c.fill(ohSharp); S3.lakesAll(c); c.lineWidth = 5; c.stroke(lakesSharp); });
  const gT = [tiles.olive, tiles.olive, tiles.khaki, tiles.chart, tiles.ochre];
  for (let y = 4, row = 0; y < H; y += 7.4, row++) for (let x = (row % 2) * 3.2; x < W; x += 6.4) {
    const tx = x + R(-0.8, 0.8), ty = y + R(-0.8, 0.8), keep = rnd(), r = R(1.7, 2.3), t = pick(gT);
    if (!nbLand(tx, ty)) continue;
    const ll = proj.invert([tx, ty]); if (!ll) continue;
    const veg = biomeAt(d.biome, ll[0], ll[1], 0); if (veg < 0.5) continue;
    if (fbm2(tx / 34, ty / 34, 17) + (veg - 0.5) * 0.3 < 0.62 || keep > 0.78 - 0.45 * smooth(CAPTION_TOP - 60, CAPTION_TOP + 80, ty)) continue;
    roundTree(tx, ty + 2, r, t, { inkA: 0.55 });
  }
  columbusWood(d, proj);
  // the rivers: the Ohio broad and inked, the others fine
  for (const f of d.rivers.features) {
    const pth = geoP(proj, f, { amp: 0.4, sc: 6, seed: 9 });
    if (f.properties.name === 'Ohio') {
      ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round'; ctx.strokeStyle = rgba(P.ink, 0.85); ctx.lineWidth = 4.2; ctx.stroke(pth);
      ctx.strokeStyle = '#dfe6c4'; ctx.lineWidth = 3.0; ctx.stroke(pth); ctx.strokeStyle = rgba(P.jade, 0.55); ctx.lineWidth = 0.5; ctx.setLineDash([3, 2.5, 0.1, 2.5]); ctx.stroke(pth); ctx.restore();
    } else ink(pth, { w: 0.95, col: P.jade, alpha: 0.85, passes: 1 });
  }
  // the interstates, faint pilgrim paths between the walled towns
  ctx.save(); ctx.clip(ohSharp); ctx.setLineDash([2.2, 1.8]); ctx.strokeStyle = rgba(P.umber, 0.32); ctx.lineWidth = 0.55; ctx.stroke(geoP(proj, d.roads)); ctx.restore(); css(ctx);
  // Ohio's ancient earthworks, carved like the stones of the Boyne
  const sm = proj([-83.43, 39.025]); serpentMound(sm[0], sm[1], 26);
  const nw = proj([-82.44, 40.05]); newarkWorks(nw[0] - 6, nw[1] - 4, 10);
  // shores and borders
  hatch(c => { c.fill(lakesSharp); c.globalCompositeOperation = 'destination-out'; c.translate(1.8, 2.2); c.fill(lakesSharp); c.translate(-1.8, -2.2); c.lineWidth = 7; c.stroke(S3.boxP); }, { ang: 40, sp: 1.1, len: [1.4, 3.6], w: 0.3, alpha: 0.45, col: P.jade });
  strokeMasked(c => { c.lineWidth = 0.7; c.lineJoin = 'round'; c.stroke(lakesP); }, c => { c.fill(S3.boxP); c.globalCompositeOperation = 'destination-out'; c.lineWidth = 3; c.stroke(S3.boxP); }, { col: rgba(P.ink, 0.85) });
  strokeMasked(c => { c.lineWidth = 0.7; c.lineJoin = 'round'; c.stroke(S3.farLakesP); }, c => { c.fillRect(-20, -20, W + 40, H + 40); c.globalCompositeOperation = 'destination-out'; c.fill(S3.boxP); }, { col: rgba(P.ink, 0.8) });
  const others = geoP(proj, FC(d.naStates.features.filter(f => f.properties.name !== 'Ohio')));
  strokeMasked(c => { c.lineWidth = 0.7; c.lineCap = 'round'; c.setLineDash([0.1, 2.2]); c.stroke(others); }, c => { c.lineWidth = 3; c.stroke(lakesSharp); S3.lakesAll(c); c.stroke(ohSharp); }, { out: true, col: rgba(P.umber, 0.55) });
  hatch(rimBand(ohP, 3), { ang: -30, sp: 0.9, len: [1, 2.6], w: 0.32, alpha: 0.55, col: P.rust });
  strokeMasked(c => { c.lineWidth = 1.5; c.setLineDash([0.1, 2.6]); c.lineCap = 'round'; c.stroke(ohP); }, c => { c.lineWidth = 2.5; c.stroke(lakesSharp); }, { out: true, col: P.rust });
  // the walled towns, and Columbus walled by its ring road
  for (const [n, lon, lat, r] of OH_TOWNS) { const p = proj([lon, lat]); walledTown(p[0], p[1], r); }
  columbusWall(d, proj);
}
const OH_WAVES = [[[-82.0, 41.95], 9], [[-81.0, 42.25], 11], [[-82.75, 41.68], 6]];
const OH_TOWNS = [['Toledo', -83.55, 41.66, 4.2, 'r'], ['Akron', -81.52, 41.08, 4.2, 'l'], ['Dayton', -84.19, 39.76, 4.6, 'l'], ['Youngstown', -80.65, 41.1, 3.8, 'b'], ['Canton', -81.38, 40.8, 3.8, 'r'],
  ['Cleveland', -81.69, 41.48, 7, 'B'], ['Cincinnati', -84.51, 39.12, 7, 'B'], ['Detroit', -83.05, 42.33, 6, 'u'], ['Pittsburgh', -80.0, 40.44, 5.5, 'b']];
// A ring-walled place in plan, its round tower standing up out of it (the film's flat false perspective).
function walledTown(x, y, r, o = {}) {
  const roofs = [];
  for (let rr = r * 0.78; rr > r * 0.3; rr -= Math.max(1.6, r * 0.17)) { const n = Math.max(5, Math.round(rr * 2 * Math.PI / Math.max(2, r * 0.2))); for (let k = 0; k < n; k++) roofs.push([rr, k / n * 6.283 + rr, pick(['#b45a2e', '#9a4524', '#8a6a48', '#c27a3c'])]); }
  ctx.save(); ctx.lineCap = 'round';
  ctx.fillStyle = 'rgba(40,30,8,0.22)'; ctx.beginPath(); ctx.ellipse(x + 1.2, y + 1.6, r, r * 0.86, 0, 0, 7); ctx.fill();
  const yard = new Path2D(); yard.ellipse(x, y, r, r * 0.86, 0, 0, 7);
  ctx.fillStyle = pattern(tiles.straw); ctx.fill(yard);
  ctx.save(); ctx.clip(yard);
  for (const [rr, a, c] of roofs) { ctx.fillStyle = c; ctx.globalAlpha = 0.85; ctx.fillRect(x + rr * Math.cos(a) - r * 0.07, y + rr * 0.86 * Math.sin(a) - r * 0.05, r * 0.14, r * 0.1); }
  ctx.restore();
  ctx.strokeStyle = P.ink; ctx.globalAlpha = 0.9; ctx.lineWidth = Math.max(0.5, r * 0.07); ctx.stroke(yard);
  ctx.lineWidth = 0.35; ctx.globalAlpha = 0.7; ctx.beginPath(); ctx.ellipse(x, y, r + 1.4, r * 0.86 + 1.4, 0, 0, 7); ctx.stroke();
  const ns = Math.round(r * 3); for (let k = 0; k < ns; k++) { const a = k / ns * 6.283; ctx.beginPath(); ctx.moveTo(x + r * Math.cos(a), y + r * 0.86 * Math.sin(a)); ctx.lineTo(x + (r + 1.4) * Math.cos(a), y + (r * 0.86 + 1.4) * Math.sin(a)); ctx.stroke(); }
  ctx.restore();
  roundTower(x, y, Math.max(1.4, r * 0.22), o.tower ?? r * 1.4, { windows: 1 });
}
// The wild wood around Columbus's wall at this scale: small crowns from the wall outward, thinning into the fields,
// so that diving in, the closer plate's wood fades up over wood.
function columbusWood(d, proj) {
  const pts = d.ring.features[0].geometry.coordinates.map(c => proj(c)), ring = polyP(smoothPts(pts, 3, true), {}), c0 = proj([-82.99, 39.99]);
  const inside = maskOf(c => { c.fill(ring); c.lineWidth = 3; c.stroke(ring); }), wet = maskOf(c => { c.lineWidth = 1.6; c.stroke(geoP(proj, d.rivers)); });
  const T = [], tT = [tiles.olive, tiles.olive, tiles.olive, tiles.khaki, tiles.khaki, tiles.chart, tiles.olive, tiles.deep, tiles.ochre];
  for (let y = c0[1] - 48, row = 0; y < c0[1] + 48; y += 1.55, row++) for (let x = c0[0] - 48 + (row % 2) * 0.85; x < c0[0] + 48; x += 1.7) {
    const tx = x + R(-0.35, 0.35), ty = y + R(-0.35, 0.35), keep = rnd(), r = R(0.9, 1.2), t = pick(tT);
    if (inside(tx, ty) || wet(tx, ty)) continue;
    const dist = Math.hypot(tx - c0[0], (ty - c0[1]) * 0.9), p = 1 - smooth(16, 44, dist);
    if (keep > p * 0.95 || fbm2(tx / 9, ty / 9, 43) < 0.36) continue;
    T.push([tx, ty, r, t]);
  }
  T.sort((a, b) => a[1] - b[1]);
  for (const [x, y, r, t] of T) crown(x, y, r, t, { inkA: 0.32, shA: 0.22, w: 0.18, lobes: 6 });
}
// Columbus at this scale: its ring road drawn as the town wall, roofs inside it, one slim tower downtown.
function columbusWall(d, proj) {
  const pts = d.ring.features[0].geometry.coordinates.map(c => proj(c)), sm = smoothPts(pts, 3, true), ring = polyP(sm, {});
  ctx.save(); ctx.fillStyle = 'rgba(40,30,8,0.2)'; ctx.translate(0.8, 1.1); ctx.fill(ring); ctx.restore(); css(ctx);
  ctx.save(); ctx.fillStyle = pattern(tiles.straw); ctx.fill(ring); ctx.clip(ring);
  const b = bboxOf(sm, 0);
  for (let y = b[1]; y < b[3]; y += 1.6) for (let x = b[0]; x < b[2]; x += 1.9) { const c = pick(['#b45a2e', '#9a4524', '#8a6a48', '#c27a3c', '#a07a52']), a = R(0.35, 0.8), jx = R(-0.4, 0.4); ctx.fillStyle = c; ctx.globalAlpha = a; ctx.fillRect(x + jx, y, 0.9, 0.6); }
  ctx.restore(); css(ctx);
  ctx.save(); ctx.strokeStyle = '#cdb98a'; ctx.lineJoin = 'round'; ctx.lineWidth = 1.1; ctx.stroke(ring);
  ctx.strokeStyle = P.ink; ctx.lineWidth = 0.4; ctx.globalAlpha = 0.85; ctx.stroke(polyP(offsetPts(sm, 0.55), {})); ctx.stroke(polyP(offsetPts(sm, -0.55), {}));
  ctx.restore();
  const dt = proj([-83.0, 39.9615]); roundTower(dt[0], dt[1], 1.5, 9, { windows: 1 });
}
function serpentMound(x, y, sz) {
  const body = []; for (let k = 0; k <= 60; k++) { const t = k / 60; body.push([x - sz * 0.55 + sz * 1.0 * t, y + sz * 0.16 * Math.sin(t * Math.PI * 3.2) - sz * 0.05 * t]); }
  const tail = spiralPts(x - sz * 0.62, y + sz * 0.06, sz * 0.03, sz * 0.12, 1.6, 0, -1).reverse(), head = body[body.length - 1];
  const mound = polyP(spiralPts(x, y, sz * 0.7, sz * 0.7, 1, 0, 1, 0.42).slice(0, -1), { amp: sz * 0.12, sc: sz * 0.4, seed: 77 });
  wash(mound, { c: ['#a9ad4a', '#8a9236'], op: 0.32, edge: 0.6, blur: sz * 0.12, mottle: 0.3, bbox: [x - sz, y - sz, x + sz, y + sz] });
  carve(tail.concat(body), { w: Math.max(1.4, sz * 0.07), step: 0.8, op: 0.6, bbox: [x - sz, y - sz, x + sz, y + sz] });
  ctx.save(); ctx.strokeStyle = P.ink; ctx.lineWidth = 0.5; ctx.globalAlpha = 0.8; ctx.beginPath(); ctx.ellipse(head[0] + sz * 0.08, head[1] - sz * 0.02, sz * 0.07, sz * 0.045, 0, 0, 7); ctx.stroke(); ctx.restore();
}
function newarkWorks(x, y, sz) {
  const c = spiralPts(x, y, sz * 0.32, sz * 0.32, 1, 0, 1), oct = [];
  for (let k = 0; k <= 8; k++) { const a = k / 8 * 6.283 + Math.PI / 8; oct.push([x + sz * 0.95 + sz * 0.3 * Math.cos(a), y + sz * 0.55 + sz * 0.3 * Math.sin(a)]); }
  carve(c, { w: 1.2, step: 0.7, op: 0.55, bbox: [x - sz, y - sz, x + sz * 2, y + sz * 2] });
  carve(oct, { w: 1.1, step: 0.7, op: 0.55, bbox: [x - sz, y - sz, x + sz * 2, y + sz * 2] });
  ctx.save(); ctx.strokeStyle = rgba(P.umber, 0.7); ctx.lineWidth = 0.45; ctx.beginPath(); ctx.moveTo(x + sz * 0.3, y + sz * 0.1); ctx.lineTo(x + sz * 0.7, y + sz * 0.4); ctx.moveTo(x + sz * 0.26, y + sz * 0.2); ctx.lineTo(x + sz * 0.66, y + sz * 0.5); ctx.stroke(); ctx.restore();
}
function ohioLabels(d) {
  const S3 = ohioShapes(d), { proj } = S3;
  for (const [n, lon, lat, r, side] of OH_TOWNS) {
    const [x, y] = proj([lon, lat]), big = side === 'B';
    if (side === 'B') label(n, x, y + r + 9, { size: 11, ls: 0.06 });
    else if (side === 'b') label(n, x, y + r + 7, { size: 9.5, ls: 0.04 });
    else if (side === 'u') label(n, x, y - r - 12, { size: 9.5, ls: 0.04 });
    else label(n, side === 'l' ? x - r - 4 : x + r + 4, y + 1, { size: 9.5, align: side === 'l' ? 'right' : 'left', ls: 0.04 });
  }
  const col = proj([-82.99, 39.96]); label('Columbus', col[0] + 2, col[1] + 22, { size: 13, ls: 0.08 });
  const le = proj([-81.45, 42.02]); label('Lake Erie', le[0], le[1], { size: 13, ls: 0.3, col: P.lakeInk, ang: -0.3, haloCol: rgba('#eef0d8', 0.7) });
  const ohioRiver = d.rivers.features.find(f => f.properties.name === 'Ohio');
  if (ohioRiver) {
    const lines = ohioRiver.geometry.type === 'MultiLineString' ? ohioRiver.geometry.coordinates : [ohioRiver.geometry.coordinates];
    let rp = lines.flat().map(c => proj(c)).filter(p => p && p[0] > 268 && p[0] < 352 && p[1] < 860);
    if (rp.length > 4) { if (rp[0][0] > rp[rp.length - 1][0]) rp = rp.reverse(); textOnPath('Ohio River', offsetPts(smoothPts(rp, 3), 10.5), { size: 10.5, ls: 0.2, col: P.lakeInk }); }
  }
  for (const [n, lon, lat, ang] of [['Michigan', -84.4, 42.75, 0], ['Indiana', -85.3, 40.2, -1.5708], ['Kentucky', -84.2, 38.15, 0], ['West Virginia', -80.95, 38.6, -0.75], ['Pennsylvania', -79.75, 41.25, -1.5708], ['Ontario', -81.6, 42.75, 0]]) { const p = proj([lon, lat]); label(n, p[0], p[1], { size: 9.5, ang, ls: 0.25, col: '#7a6a3a', alpha: 0.8 }); }
  const sm = proj([-83.43, 39.025]); label('Serpent Mound', sm[0] - 2, sm[1] + 16, { size: 8.5, ls: 0.06, col: P.umber });
  const nw = proj([-82.44, 40.05]); label('Newark', nw[0] + 14, nw[1] - 9, { size: 8.5, ls: 0.06, col: P.umber, align: 'left' });
  const sc = proj([-83.0, 39.45]); label('Scioto', sc[0] - 7, sc[1], { size: 8.5, ang: -1.35, ls: 0.12, col: P.lakeInk });
  const oh = proj([-83.85, 40.75]); label('Ohio', oh[0], oh[1], { size: 22, ls: 0.3, col: P.rust, alpha: 0.75 });
}
function ohioGlints(d) {
  const { proj } = ohioShapes(d);
  for (const [c, r] of OH_WAVES) dots(spiralPts(...proj(c), 0.6, r - 1.6, 2.3, 0.3, 1, 0.62), { gap: 2.6, dotW: 1 });
  ringDots(3, { gap: 2.2, w: 1.1 });
}
// A skein of geese, a loose V of ink ticks, crossing the plain.
function geese(x, y, s) {
  const pos = [[0, 0], [-1, 0.62], [-2.05, 1.2], [-3.1, 1.85], [-0.95, -0.66], [-1.95, -1.3], [-3.0, -1.9]];
  ctx.save(); ctx.strokeStyle = P.ink; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  pos.forEach(([dx, dy], k) => {
    const bx = x + dx * s, by = y + dy * s * 0.9, w = s * (0.62 + 0.06 * (k % 3)), lift = s * (0.18 + 0.05 * ((k * 3) % 4));
    ctx.lineWidth = 0.55; ctx.globalAlpha = 0.85;
    ctx.beginPath(); ctx.moveTo(bx - w / 2, by + lift * 0.1); ctx.quadraticCurveTo(bx - w * 0.2, by - lift, bx, by); ctx.quadraticCurveTo(bx + w * 0.2, by - lift, bx + w / 2, by + lift * 0.1); ctx.stroke();
    ctx.fillStyle = P.ink; ctx.beginPath(); ctx.arc(bx + s * 0.06, by + s * 0.02, 0.45, 0, 7); ctx.fill();
  });
  ctx.restore();
}
const GEESE = [190, 440, 6.5];

// =====================================================================================================
// Level 4: Columbus inside the Outerbelt: a ring road for a wall, the wild wood outside it, and the Scioto and the
// Olentangy meeting under the towers.
function columbusProj() { return lensU(ZOOMS[3]); }
async function columbusLoad() {
  const [roads, water, streams, city, snB, snT, snR] = await Promise.all(['columbus-roads', 'columbus-water', 'columbus-streams', 'columbus-city', 'shortnorth-buildings', 'shortnorth-trees', 'shortnorth-roads'].map(loadGeo));
  return { roads, water, streams, city, snB, snT, snR, ring: await loadJSON('data/columbus-ring.geojson') };
}
const BIG_RIVERS = /Scioto Riv|Olentangy|Alum Crk|Big Walnut|Blacklick|Big Run|Darby|Rocky Fork|Walnut Crk/;
function columbusShapes(d) {
  if (d.shapes) return d.shapes;
  const proj = columbusProj(), path = d3.geoPath(proj);
  const ringPts = smoothPts(d.ring.features[0].geometry.coordinates.map(c => proj(c)), 2, true);
  const bigWater = d.water.features.filter(f => path.area(f) > 2.5);
  const rivers = d.streams.features.filter(f => BIG_RIVERS.test(f.properties.name || ''));
  const inter = d.roads.features.filter(f => f.properties.class === 'interstate' && !['Outerbelt', 'I- 270', 'Jack Nicklaus Fwy'].includes(f.properties.name));
  d.shapes = { proj, path, ringPts, ringP: polyP(ringPts, {}, true), ringW: polyP(ringPts, { amp: 0.5, sc: 8, seed: 3 }, true),
    waterP: geoP(proj, FC(bigWater), { amp: 0.25, sc: 4, seed: 5 }), riverLines: geoP(proj, FC(rivers), { amp: 0.3, sc: 5, seed: 6 }), rivers,
    interP: geoP(proj, FC(inter), { amp: 0.3, sc: 6, seed: 8 }), inter };
  return d.shapes;
}
const COL_WALL = 5.2;
// Short North drawn small inside the city, its roofs, streets and trees, fading out at the edge of its data: what
// the next plate shows in full, so as the camera dives the closer plate fades up over the same streets and trees.
function shortNorthInMiniature(d, proj) {
  const box = [[-83.0145, 39.9675], [-82.9955, 39.9985]].map(c => proj(c)), cx = (box[0][0] + box[1][0]) / 2, cy = (box[0][1] + box[1][1]) / 2;
  const ax = Math.abs(box[1][0] - box[0][0]) / 2, ay = Math.abs(box[1][1] - box[0][1]) / 2;
  const fade = (x, y) => { const n = Math.pow(Math.pow(Math.abs(x - cx) / ax, 6) + Math.pow(Math.abs(y - cy) / ay, 6), 1 / 6); return 1 - smooth(0.82, 1.0, n); };
  ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  const roads = d.snR.features.filter(f => ROAD_W[f.properties.class]);
  for (const f of roads) {
    const c0 = f.geometry.type === 'LineString' ? f.geometry.coordinates[0] : f.geometry.coordinates[0][0], p0 = proj(c0), a = fade(...p0); if (a <= 0) continue;
    const w = ROAD_W[f.properties.class] * ZOOMS[3] / ZOOMS[4]; // the next plate's road widths, at this scale
    const pth = geoP(proj, f); ctx.globalAlpha = a * 0.7; ctx.strokeStyle = P.ink; ctx.lineWidth = w + 0.06; ctx.stroke(pth); ctx.globalAlpha = a; ctx.strokeStyle = '#f1e8c9'; ctx.lineWidth = w; ctx.stroke(pth);
  }
  const cols = ['#c48a62', '#b4865a', '#a8927a', '#b3a68c', '#c07a4c', '#d8d0b4'];
  for (const f of d.snB.features) {
    if (['garage', 'garages', 'shed', 'roof'].includes(f.properties.kind)) continue;
    const ring = (f.geometry.type === 'Polygon' ? f.geometry.coordinates[0] : f.geometry.coordinates[0][0]).map(c => proj(c)), a = fade(...ring[0]), col = pick(cols);
    if (a <= 0) continue;
    const pth = linePath(ring); pth.closePath(); ctx.globalAlpha = a * 0.9; ctx.fillStyle = col; ctx.fill(pth); ctx.globalAlpha = a * 0.5; ctx.strokeStyle = P.umber; ctx.lineWidth = 0.02; ctx.stroke(pth);
  }
  const tc = ['#7c8430', '#9aa040', '#a49b40', '#b8bb48', '#c2b65a', '#5d6322', '#d29a34'], f = ZOOMS[3] / ZOOMS[4];
  // the trees: the mapped ones, and the gardens' trees gathered by the same noise the next plate gathers them by
  const pts = d.snT.features.map(t => proj(t.geometry.coordinates));
  for (let y = -120; y < H + 120; y += 7) for (let x = -120; x < W + 120; x += 7) {
    const hx = x + (hash2(x, y, 5) - 0.5) * 5, hy = y + (hash2(x, y, 6) - 0.5) * 5;
    if (hash2(x, y, 7) > 0.55 || fbm2(hx / 26, hy / 26, 51) < 0.52) continue;
    pts.push([C0[0] + (hx - C0[0]) * f, C0[1] + (hy - C0[1]) * f]);
  }
  for (const t of d.snR.features.filter(g => ['residential', 'tertiary', 'secondary', 'unclassified', 'living_street'].includes(g.properties.class))) {
    const ls = t.geometry.type === 'LineString' ? [t.geometry.coordinates] : t.geometry.coordinates, off = ((ROAD_W[t.properties.class] || 4) / 2 + 3.2) * f;
    for (const l of ls) { const pp = l.map(c => proj(c)); for (let i = 1; i < pp.length; i++) { const a = pp[i - 1], b = pp[i], L = Math.hypot(b[0] - a[0], b[1] - a[1]); if (!L) continue;
      for (let u = 0; u < L; u += 9.5 * f) { const x = a[0] + (b[0] - a[0]) * u / L, y = a[1] + (b[1] - a[1]) * u / L, nx = -(b[1] - a[1]) / L, ny = (b[0] - a[0]) / L;
        for (const sd of [-1, 1]) if (hash2(Math.round(x * 1e3), Math.round(y * 1e3), sd + 9) < 0.45) pts.push([x + nx * sd * off, y + ny * sd * off]); } } }
  }
  pts.sort((a, b) => a[1] - b[1]);
  for (const p of pts) {
    const a = fade(...p), col = pick(tc), r = R(3.4, 5.6) * f; if (a <= 0) continue;
    ctx.globalAlpha = a * 0.3; ctx.fillStyle = '#2a2a0c'; ctx.beginPath(); ctx.arc(p[0] + r * 0.35, p[1] + r * 0.45, r, 0, 7); ctx.fill();
    ctx.globalAlpha = a; ctx.fillStyle = col; ctx.beginPath(); ctx.arc(p[0], p[1], r, 0, 7); ctx.fill();
    ctx.globalAlpha = a * 0.5; ctx.strokeStyle = P.ink; ctx.lineWidth = 0.015; ctx.stroke();
  }
  ctx.restore(); css(ctx);
}
function columbusGround(d) {
  const S4 = columbusShapes(d), { proj, ringP, ringW, waterP, riverLines, interP } = S4;
  const outside = c => { c.fillRect(-10, -10, W + 20, H + 20); c.globalCompositeOperation = 'destination-out'; c.fill(ringP); c.globalCompositeOperation = 'source-over'; };
  paperGround();
  wash(ringW, { c: [P.straw, P.chart2, '#e2cf86'], op: 0.55, edge: 1.0, blur: 4, mottle: 0.35, gran: 0.1 });
  // the suburbs inside the wall, each its own faint wash and a dotted bound
  for (const f of d.city.features.filter(f => f.properties.name !== 'Columbus')) {
    const sp = geoP(proj, f, { amp: 0.4, sc: 6, seed: 12 }), col = pick([P.chart2, '#d9cf7a', '#cfd27a', P.straw]);
    wash(sp, { c: [col], op: 0.3, edge: 1.4, blur: 2, mottle: 0.3, cut: outside });
    ctx.save(); ctx.clip(ringP); ctx.setLineDash([0.1, 2]); ctx.lineCap = 'round'; ctx.strokeStyle = rgba(P.umber, 0.55); ctx.lineWidth = 0.8; ctx.stroke(sp); ctx.restore(); css(ctx);
  }
  // the streets, so many and so fine that they read as the pen's hatching
  const byClass = cl => FC(d.roads.features.filter(f => f.properties.class === cl));
  ctx.save(); ctx.clip(ringP); ctx.lineCap = 'round';
  ctx.strokeStyle = rgba(P.umber, 0.22); ctx.lineWidth = 0.3; ctx.stroke(geoP(proj, byClass('local')));
  ctx.strokeStyle = rgba(P.umber, 0.45); ctx.lineWidth = 0.55; ctx.stroke(geoP(proj, byClass('major'), { amp: 0.2, sc: 5, seed: 7 }));
  ctx.restore(); css(ctx);
  shortNorthInMiniature(d, proj);
  // the wild wood outside the wall
  wash(outside, { c: ['#8f9438', '#a8963e', '#7d8430'], op: 0.55, edge: 0.2, blur: 4, mottle: 0.45, gran: 0.12 });
  wash(waterP, { c: [P.water, '#e9ecd0', P.water2], over: true, op: 0.92, edge: 1.4, blur: 1.2, mottle: 0.2 });
  ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  ctx.strokeStyle = rgba(P.ink, 0.75); ctx.lineWidth = 2.6; ctx.stroke(riverLines);
  ctx.strokeStyle = '#e3e8c6'; ctx.lineWidth = 1.7; ctx.stroke(riverLines); ctx.restore(); css(ctx);
  ink(waterP, { w: 0.45, alpha: 0.75 });
  ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round'; ctx.strokeStyle = rgba(P.ink, 0.7); ctx.lineWidth = 2.2; ctx.stroke(interP); ctx.strokeStyle = '#efe2b8'; ctx.lineWidth = 1.3; ctx.stroke(interP); ctx.restore(); css(ctx);
  // the wood: crowns massed edge to edge, drawn from the back; glades open here and there, and one for the deer
  const inWall = maskOf(ringP), wet = maskOf(c => { c.lineWidth = 6; c.lineCap = 'round'; c.stroke(riverLines); c.fill(waterP); c.lineWidth = 5; c.stroke(interP); });
  const nearRiver = maskOf(c => { c.lineWidth = 16; c.lineCap = 'round'; c.stroke(riverLines); });
  const tT = [tiles.olive, tiles.olive, tiles.olive, tiles.khaki, tiles.khaki, tiles.khaki, tiles.chart, tiles.chart, tiles.lime, tiles.olive, tiles.khaki, tiles.ochre, tiles.straw];
  const trees = [];
  for (let y = 0, row = 0; y < H + 6; y += 6.4, row++) for (let x = (row % 2) * 3.6 - 4; x < W + 6; x += 7.2) {
    const tx = x + R(-1.4, 1.4), ty = y + R(-1.2, 1.2), r1 = R(2.4, 3.2), r2 = R(3.6, 4.8), t = rnd() < 0.012 ? tiles.rust : pick(tT), sp = rnd() < 0.08;
    if (wet(tx, ty)) continue;
    const inside = inWall(tx, ty);
    if (inside && !(nearRiver(tx, ty) && fbm2(tx / 18, ty / 18, 41) > 0.45)) continue;
    if (!inside && (fbm2(tx / 30, ty / 30, 43) < 0.4 + 0.22 * smooth(CAPTION_TOP - 40, SCREEN[3], ty) || Math.hypot(tx - DEER[0], ty - DEER[1] + 5) < 17)) continue;
    trees.push([tx, ty, inside, inside ? r1 : r2, t, sp]);
  }
  trees.sort((a, b) => a[1] - b[1]);
  for (const [tx, ty, inside, r, t, sp] of trees) crown(tx, ty, r, t, { spiral: !inside && sp, inkA: 0.34, shA: 0.22 });
  wash(outside, { c: ['#7f8a30', '#8c8236'], op: 0.26, edge: 0, blur: 3, mottle: 0.4, gran: 0.05 });
  calmFoot('#77802e', 0.42);
  // the wall: the Outerbelt as a palisade, stakes between two inked faces, gate towers where roads and rivers go out
  const S0 = smoothPts(S4.ringPts, 1, true);
  wash(c => { c.lineWidth = COL_WALL; c.lineJoin = 'round'; c.stroke(ringW); }, { c: ['#cdb98a', '#b89f6c'], over: true, op: 0.95, edge: 1.2, blur: 1, mottle: 0.3 });
  ctx.save(); ctx.strokeStyle = P.ink; ctx.lineJoin = 'round';
  ctx.lineWidth = 0.8; ctx.stroke(polyP(offsetPts(S0, COL_WALL / 2), { amp: 0.3, sc: 6, seed: 9 }, true));
  ctx.lineWidth = 0.6; ctx.stroke(polyP(offsetPts(S0, -COL_WALL / 2), { amp: 0.3, sc: 6, seed: 10 }, true));
  let acc = 0; ctx.lineWidth = 0.45; ctx.globalAlpha = 0.75;
  for (let i = 1; i < S0.length; i++) { const a = S0[i - 1], b = S0[i], L = Math.hypot(b[0] - a[0], b[1] - a[1]); if (!L) continue; acc += L; while (acc > 1.7) { acc -= 1.7; const t = 1 - acc / L, x = a[0] + (b[0] - a[0]) * t, y = a[1] + (b[1] - a[1]) * t, nx = -(b[1] - a[1]) / L, ny = (b[0] - a[0]) / L; ctx.beginPath(); ctx.moveTo(x - nx * COL_WALL / 2, y - ny * COL_WALL / 2); ctx.lineTo(x + nx * COL_WALL / 2, y + ny * COL_WALL / 2); ctx.stroke(); } }
  ctx.restore(); css(ctx);
  const gates = [];
  for (const f of S4.inter.concat(S4.rivers.filter(f => /Scioto|Olentangy|Alum|Big Walnut/.test(f.properties.name)))) {
    const lines = f.geometry.type === 'LineString' ? [f.geometry.coordinates] : f.geometry.coordinates;
    for (const l of lines) { const pp = l.map(c => proj(c)); for (let i = 1; i < pp.length; i++) if (inWall(...pp[i - 1]) !== inWall(...pp[i])) { const g = [(pp[i - 1][0] + pp[i][0]) / 2, (pp[i - 1][1] + pp[i][1]) / 2]; if (!gates.some(q => Math.hypot(q[0] - g[0], q[1] - g[1]) < 14)) gates.push(g); } }
  }
  gates.sort((a, b) => a[1] - b[1]);
  for (const [x, y] of gates) roundTower(x, y + 2, 4.2, 6.5, { windows: 1, cap: 0.9, capCol: '#a9542a' });
  // downtown, where the rivers meet: a ring of stone seen from above, its towers standing up out of it
  const dt = proj([-83.0, 39.9615]);
  const yard = new Path2D(); yard.ellipse(dt[0], dt[1], 13, 9.5, 0, 0, 7);
  ctx.save(); ctx.fillStyle = 'rgba(40,30,8,0.22)'; ctx.beginPath(); ctx.ellipse(dt[0] + 1.2, dt[1] + 1.6, 13, 9.5, 0, 0, 7); ctx.fill();
  ctx.fillStyle = '#d6cfb4'; ctx.fill(yard); ctx.clip(yard); ctx.strokeStyle = rgba(P.umber, 0.55); ctx.lineWidth = 0.35;
  for (let r = 3; r < 14; r += 2.4) { ctx.beginPath(); ctx.ellipse(dt[0], dt[1], r, r * 0.73, 0, 0, 7); ctx.stroke(); }
  for (let k = 0; k < 16; k++) { const a = k / 16 * 6.283; ctx.beginPath(); ctx.moveTo(dt[0] + 3 * Math.cos(a), dt[1] + 2.2 * Math.sin(a)); ctx.lineTo(dt[0] + 13 * Math.cos(a), dt[1] + 9.5 * Math.sin(a)); ctx.stroke(); }
  ctx.restore(); css(ctx); ink(yard, { w: 0.8, alpha: 0.9 });
  for (const [dx, dy, w, h] of [[-6, -1, 2.6, 9], [5.5, 0.5, 2.4, 7.5], [-1, 3, 3.2, 13], [9, 4, 2.2, 6], [-9.5, 4, 2.2, 6.5]].sort((a, b) => a[1] - b[1])) roundTower(dt[0] + dx, dt[1] + dy, w, h, { windows: 1 });
}
function columbusLabels(d) {
  const { proj } = columbusShapes(d);
  const lab = (n, lon, lat, ang, o = {}) => { const p = proj([lon, lat]); label(n, p[0], p[1], { size: o.size || 9.5, ang, ls: 0.14, col: o.col || P.lakeInk, haloCol: rgba(o.halo || P.straw, 0.75), ...o }); };
  lab('Scioto', -83.093, 40.043, 1.2); lab('Olentangy', -83.026, 40.07, -1.35); lab('Alum Creek', -82.948, 40.02, -1.42); lab('Big Walnut', -82.894, 39.93, -1.1);
  for (const [n, lon, lat] of [['Upper Arlington', -83.077, 40.012], ['Worthington', -83.02, 40.1], ['Bexley', -82.935, 39.962], ['Grandview', -83.042, 39.979], ['Whitehall', -82.882, 39.969]]) lab(n, lon, lat, 0, { size: 8.5, ls: 0.1, col: '#6a5a30' });
  const S4 = columbusShapes(d), southern = S4.ringPts.filter(p => p[1] > C0[1] + 60 && p[0] > C0[0] - 30 && p[0] < C0[0] + 110).sort((a, b) => a[0] - b[0]);
  if (southern.length > 3) textOnPath('The Outerbelt', offsetPts(smoothPts(southern, 2), -9), { size: 10, ls: 0.3, col: P.umber, haloCol: rgba(P.straw, 0.7) });
  label('Short North', C0[0] + 17, C0[1] - 14, { size: 9, align: 'left', ls: 0.06 });
  const dt = proj([-83.0, 39.9615]); label('Downtown', dt[0] + 16, dt[1] + 4, { size: 9, align: 'left', ls: 0.06, col: P.umber });
}
function columbusGlints() { ringDots(4, { gap: 1.6, w: 0.9 }); }
// A white deer in a glade, looking back: the wood's quiet creature.
function deer(x, y, s) {
  ctx.save(); ctx.translate(x, y); ctx.lineJoin = 'round'; ctx.lineCap = 'round';
  ctx.fillStyle = 'rgba(30,26,8,0.28)'; ctx.beginPath(); ctx.ellipse(s * 0.15, 0.2, s * 0.62, s * 0.12, 0, 0, 7); ctx.fill();
  const body = new Path2D();
  body.moveTo(-s * 0.42, -s * 0.52); body.bezierCurveTo(-s * 0.2, -s * 0.6, s * 0.2, -s * 0.6, s * 0.38, -s * 0.55);
  body.bezierCurveTo(s * 0.46, -s * 0.66, s * 0.5, -s * 0.84, s * 0.52, -s * 0.98); body.lineTo(s * 0.66, -s * 1.02); body.quadraticCurveTo(s * 0.74, -s * 0.94, s * 0.62, -s * 0.88);
  body.quadraticCurveTo(s * 0.56, -s * 0.62, s * 0.44, -s * 0.38); body.lineTo(s * 0.46, 0); body.lineTo(s * 0.38, 0); body.lineTo(s * 0.34, -s * 0.32);
  body.quadraticCurveTo(0, -s * 0.28, -s * 0.3, -s * 0.32); body.lineTo(-s * 0.34, 0); body.lineTo(-s * 0.42, 0); body.lineTo(-s * 0.44, -s * 0.4); body.quadraticCurveTo(-s * 0.52, -s * 0.5, -s * 0.42, -s * 0.52); body.closePath();
  ctx.fillStyle = P.white; ctx.fill(body); ctx.fillStyle = 'rgba(200,190,160,0.4)'; ctx.save(); ctx.clip(body); ctx.fillRect(-s, -s * 0.42, s * 2, s); ctx.restore();
  ctx.strokeStyle = P.ink; ctx.lineWidth = 0.42; ctx.stroke(body);
  ctx.lineWidth = 0.38; for (const sgn of [-1, 1]) { ctx.beginPath(); ctx.moveTo(s * 0.55, -s * 1.0); ctx.quadraticCurveTo(s * (0.5 + sgn * 0.12), -s * 1.25, s * (0.46 + sgn * 0.2), -s * 1.38); ctx.moveTo(s * (0.5 + sgn * 0.06), -s * 1.16); ctx.lineTo(s * (0.5 + sgn * 0.2), -s * 1.2); ctx.stroke(); }
  ctx.fillStyle = P.ink; ctx.beginPath(); ctx.arc(s * 0.6, -s * 0.95, 0.35, 0, 7); ctx.fill();
  ctx.restore();
}
const DEER = [130, 336, 13];

// =====================================================================================================
// Level 5: Short North, under the arches: High Street and its lit arches, roofs of brick and slate, every street
// tree its own crown, and a spiral carved in Thompson Park's lawn.
function snProj() { return lensU(ZOOMS[4]); }
async function snLoad() {
  const names = ['shortnorth-roads', 'shortnorth-buildings', 'shortnorth-green', 'shortnorth-water', 'shortnorth-trees', 'shortnorth-rail'];
  const [roads, buildings, green, water, trees, rail] = await Promise.all(names.map(loadGeo));
  return { roads, buildings, green, water, trees, rail };
}
const ROAD_W = { primary: 6.4, secondary: 5.6, tertiary: 5, residential: 4, unclassified: 3.8, living_street: 3.4, motorway: 9, motorway_link: 4.2, pedestrian: 3 };
function snShapes(d) {
  if (d.shapes) return d.shapes;
  const proj = snProj(), byClass = (...cl) => FC(d.roads.features.filter(f => cl.includes(f.properties.class)));
  const high = d.roads.features.filter(f => f.properties.name === 'North High Street');
  const parks = d.green.features.filter(f => ['park', 'grass', 'pitch', 'playground', 'garden', 'recreation_ground'].includes(f.properties.kind));
  const woods = d.green.features.filter(f => ['forest', 'wood', 'scrub'].includes(f.properties.kind));
  const thompson = d.green.features.find(f => f.properties.name === 'Thompson Park');
  const roadPaths = Object.keys(ROAD_W).map(k => [k, geoP(proj, byClass(k), { amp: 0.2, sc: 9, seed: 6 })]);
  // High Street's centre line in units, south to north, for the arches
  const hp = []; for (const f of high) { const ls = f.geometry.type === 'LineString' ? [f.geometry.coordinates] : f.geometry.coordinates; for (const l of ls) for (const c of l) hp.push(proj(c)); }
  hp.sort((a, b) => b[1] - a[1]);
  d.shapes = { proj, byClass, roadPaths, high: hp, parksP: geoP(proj, FC(parks), { amp: 0.5, sc: 6, seed: 3 }), woodsP: geoP(proj, FC(woods), { amp: 0.5, sc: 6, seed: 4 }),
    thompson: thompson && geoP(proj, thompson), thompsonB: thompson && d3.geoPath(proj).bounds(thompson),
    spiral: thompson && (() => { const [[x0, y0], [x1, y1]] = d3.geoPath(proj).bounds(thompson); return doubleSpiral((x0 + x1) / 2, (y0 + y1) / 2 + 6, Math.min(x1 - x0, y1 - y0) * 0.3, 2.4, 1.2); })(),
    service: geoP(proj, byClass('service'), { amp: 0.15, sc: 8, seed: 7 }), foot: geoP(proj, byClass('footway', 'path', 'cycleway', 'steps', 'track'), { amp: 0.2, sc: 6, seed: 8 }) };
  return d.shapes;
}
// where High Street is at height y (units), and its direction there
function highAt(hp, y) { for (let i = 1; i < hp.length; i++) if (hp[i][1] <= y) { const a = hp[i - 1], b = hp[i], t = (y - a[1]) / (b[1] - a[1] || 1); return [a[0] + (b[0] - a[0]) * t, y, Math.atan2(b[1] - a[1], b[0] - a[0])]; } return null; }
const ARCH_Y = d3.range(0, 14).map(k => 770 - k * 32);   // the arches, about 70 m apart, from the Goodale cap to Fifth Avenue
const CAT = [176, 492, 22];
function snGround(d) {
  const S5 = snShapes(d), { proj, roadPaths } = S5;
  paperGround();
  wash(c => c.fillRect(-10, -10, W + 20, H + 20), { c: [P.chart2, '#e2d892', '#c9cf62'], op: 0.5, edge: 0, blur: 3, mottle: 0.45, gran: 0.1, bloom: 0.25, fb: 0.3 });
  wash(S5.parksP, { c: [P.lime, P.chart, '#a9bd3e'], op: 0.42, edge: 1.2, blur: 1.5, mottle: 0.35 });
  wash(S5.woodsP, { c: ['#8a9236', '#6f7a2c'], op: 0.42, edge: 1.0, blur: 2, mottle: 0.4 });
  // Thompson Park: its lawn carved with a Newgrange double spiral
  if (S5.thompson) {
    const [[x0, y0], [x1, y1]] = S5.thompsonB, sp = S5.spiral;
    ctx.save(); ctx.clip(S5.thompson);
    carve(sp, { w: 3.2, step: 0.9, c: ['#5d6620', '#6b5a22'], op: 0.5, alpha: 0.55, lipCol: 'rgba(250,250,215,0.85)', bbox: [x0, y0, x1, y1] });
    ctx.restore(); css(ctx);
  }
  // the streets: alleys faint, footpaths dotted, the streets pale with inked kerbs, High Street the widest
  ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  ctx.strokeStyle = rgba(P.umber, 0.16); ctx.lineWidth = 2.2; ctx.stroke(S5.service); ctx.strokeStyle = rgba('#efe6c6', 0.85); ctx.lineWidth = 1.5; ctx.stroke(S5.service);
  for (const [k, p] of roadPaths) { ctx.strokeStyle = rgba(P.ink, 0.72); ctx.lineWidth = ROAD_W[k] + 1.1; ctx.stroke(p); }
  for (const [k, p] of roadPaths) { ctx.strokeStyle = k === 'motorway' ? '#e6dcc0' : '#f1e8c9'; ctx.lineWidth = ROAD_W[k]; ctx.stroke(p); }
  ctx.strokeStyle = rgba(P.ochre, 0.55); ctx.lineWidth = 0.45; ctx.setLineDash([3, 3]);
  for (const [k, p] of roadPaths) if (ROAD_W[k] >= 6) ctx.stroke(p);
  ctx.setLineDash([0.1, 2.4]); ctx.strokeStyle = rgba(P.umber, 0.32); ctx.lineWidth = 0.6; ctx.stroke(S5.foot);
  ctx.restore(); css(ctx);
  if (d.rail.features.length) { const rp = geoP(proj, d.rail); ctx.save(); ctx.strokeStyle = rgba(P.ink, 0.7); ctx.lineWidth = 1.6; ctx.stroke(rp); ctx.strokeStyle = '#efe6c6'; ctx.lineWidth = 0.8; ctx.setLineDash([1.2, 1.2]); ctx.stroke(rp); ctx.restore(); css(ctx); }
  // the roofs: brick and slate in the villages, stone along High Street; sheds and garages leave only speckle out
  const bld = d.buildings.features.map(f => ({ f, pts: (f.geometry.type === 'Polygon' ? f.geometry.coordinates[0] : f.geometry.coordinates[0][0]).map(q => proj(q)) }))
    .filter(b => b.pts.every(p => p && p[0] > -20 && p[0] < W + 20 && p[1] > -20 && p[1] < H + 20))
    .filter(b => Math.abs(d3.polygonArea(b.pts)) >= 9 && !['garage', 'garages', 'shed', 'roof'].includes(b.f.properties.kind));
  for (const b of bld) {
    const k = b.f.properties.kind, big = ['commercial', 'retail', 'office', 'hotel', 'university', 'stadium', 'industrial', 'yes'].includes(k) && Math.abs(d3.polygonArea(b.pts)) > 120;
    roof(b.pts, big ? pick([tiles.stone, tiles.straw, tiles.stone]) : k === 'church' ? tiles.slate : pick([tiles.brick, tiles.roof, tiles.roof2, tiles.slate, tiles.roof, tiles.roof2, tiles.roof3]));
  }
  // the trees: every street tree its own crown, the big ones leaf by leaf; a few more in the parks' woods
  const highP = geoP(proj, FC(d.roads.features.filter(f => f.properties.name === 'North High Street')));
  const blocked = maskOf(c => { for (const b of bld) c.fill(polyP(b.pts)); c.lineCap = 'round'; for (const [k, p] of roadPaths) { c.lineWidth = ROAD_W[k] + 1.5; c.stroke(p); } c.lineWidth = 15; c.stroke(highP); c.beginPath(); c.arc(CAT[0] + 3, CAT[1] - 9, 18, 0, 7); c.fill(); if (S5.spiral) { c.lineWidth = 8; c.stroke(linePath(S5.spiral)); } });
  const T = [];
  for (const f of d.trees.features) { const p = proj(f.geometry.coordinates), r = R(3.4, 5.8); if (p && p[0] > -10 && p[0] < W + 10 && p[1] > -10 && p[1] < H + 10 && !blocked(p[0], p[1])) T.push([p[0], p[1], r]); }
  const inWoods = maskOf(S5.woodsP), near = (x, y, dmin) => T.some(q => Math.abs(q[0] - x) < dmin && Math.abs(q[1] - y) < dmin);
  for (let y = 0; y < H; y += 6) for (let x = 0; x < W; x += 6) { const tx = x + R(-2, 2), ty = y + R(-2, 2), r = R(3.4, 5); if (inWoods(tx, ty) && !blocked(tx, ty)) T.push([tx, ty, r]); }
  // street trees along the side streets, both sides, a little apart
  for (const f of S5.byClass('residential', 'tertiary', 'secondary', 'unclassified', 'living_street').features) {
    const ls = f.geometry.type === 'LineString' ? [f.geometry.coordinates] : f.geometry.coordinates, off = (ROAD_W[f.properties.class] || 4) / 2 + 2.2;
    for (const l of ls) { const pp = l.map(c => proj(c)); let acc = R(0, 8);
      for (let i = 1; i < pp.length; i++) { const a = pp[i - 1], b = pp[i], L = Math.hypot(b[0] - a[0], b[1] - a[1]) || 1; let t = acc;
        while (t < L) { const x = a[0] + (b[0] - a[0]) * t / L, y = a[1] + (b[1] - a[1]) * t / L, nx = -(b[1] - a[1]) / L, ny = (b[0] - a[0]) / L;
          for (const sd of [-1, 1]) { const tx = x + nx * sd * off, ty = y + ny * sd * off, keep = rnd(), r = R(3.2, 4.6); if (keep < 0.5 && !blocked(tx, ty) && !near(tx, ty, 4.5)) T.push([tx, ty, r]); }
          t += R(8, 11); }
        acc = t - L; } }
  }
  // and trees in the back gardens, gathered where the old noise says so
  for (let y = 0; y < H; y += 7) for (let x = 0; x < W; x += 7) { const tx = x + R(-2.5, 2.5), ty = y + R(-2.5, 2.5), r = R(3.4, 5.4), keep = rnd(); if (blocked(tx, ty) || keep > 0.55 || fbm2(tx / 26, ty / 26, 51) < 0.52 || near(tx, ty, 5)) continue; T.push([tx, ty, r]); }
  T.sort((a, b) => a[1] - b[1]);
  const crownTiles = [tiles.olive, tiles.olive, tiles.olive, tiles.khaki, tiles.khaki, tiles.chart, tiles.chart, tiles.lime, tiles.ochre, tiles.straw, tiles.olive];
  for (const [x, y, r] of T) { const t = rnd() < 0.02 ? tiles.rust : pick(crownTiles), sp = rnd() < 0.12; crown(x, y, r, t, { leaves: r > 5.2, spiral: r <= 5.2 && sp, inkA: 0.5, shA: 0.3 }); }
  // small vivid things in the gardens and parks: fly agarics, white flowers
  const gardens = maskOf(S5.parksP);
  for (let k = 0, n = 0; k < 5000 && n < 60; k++) { const x = R(0, W), y = R(0, H), r = R(1.4, 2), kind = rnd(); if (!gardens(x, y) || blocked(x, y)) continue; n++; if (kind < 0.3) agaric(x, y, R(3.6, 4.6), { dots: 3 }); else flower(x, y, r, { petals: 5 }); }
  // the arches over High Street, standing up out of the plan, their lights on the glints layer
  for (const y of ARCH_Y) { const a = highAt(S5.high, y); if (a) arch(a[0], a[1], a[2], false); }
  dapple({ k: 0.24, sun: 0.15 });
  calmFoot('#c8c27e', 0.38);
}
// A house from above, the film's way: a roof of mottled tile, its ridge inked along the long side, a short shadow.
function roof(pts, tile) {
  if (pts.length < 4) return;
  let cx = 0, cy = 0; pts.forEach(p => { cx += p[0]; cy += p[1]; }); cx /= pts.length; cy /= pts.length;
  let best = 0, ang = 0; for (let i = 1; i < pts.length; i++) { const dx = pts[i][0] - pts[i - 1][0], dy = pts[i][1] - pts[i - 1][1], L = dx * dx + dy * dy; if (L > best) { best = L; ang = Math.atan2(dy, dx); } }
  const p = linePath(pts); p.closePath(); const pat = pattern(tile);
  let ext = 0; pts.forEach(q => { const dd = Math.abs((q[0] - cx) * Math.cos(ang) + (q[1] - cy) * Math.sin(ang)); ext = Math.max(ext, dd); });
  ctx.save();
  ctx.save(); ctx.translate(1.0, 1.3); ctx.fillStyle = 'rgba(40,30,10,0.24)'; ctx.fill(p); ctx.restore();
  ctx.fillStyle = pat; ctx.fill(p);
  ctx.save(); ctx.clip(p); ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = 'rgba(110,70,30,0.28)';
  ctx.translate(cx, cy); ctx.rotate(ang); ctx.fillRect(-ext - 2, 0, ext * 2 + 4, 40); ctx.restore();
  ctx.strokeStyle = P.umber; ctx.lineJoin = 'round'; ctx.globalAlpha = 0.6; ctx.lineWidth = 0.35; ctx.stroke(p);
  ctx.globalAlpha = 0.45; ctx.lineWidth = 0.3; ctx.beginPath(); ctx.moveTo(cx - Math.cos(ang) * ext * 0.7, cy - Math.sin(ang) * ext * 0.7); ctx.lineTo(cx + Math.cos(ang) * ext * 0.7, cy + Math.sin(ang) * ext * 0.7); ctx.stroke();
  ctx.restore();
}
// An arch over the street, in elevation: two slim posts and a semicircle, its lamps a row of white dots.
const ARCH_W = 15, ARCH_H = 13;
function archGeom(x, y, dir) { // dir: the street's direction; the arch spans across it
  const nx = Math.cos(dir + Math.PI / 2), half = ARCH_W / 2, left = [x - nx * half, y], right = [x + nx * half, y];
  const top = []; for (let k = 0; k <= 24; k++) { const t = k / 24, a = Math.PI * (1 - t); top.push([x + Math.cos(a) * half * Math.sign(nx || 1), y - ARCH_H * 0.55 - Math.sin(a) * ARCH_H * 0.6]); }
  return { left, right, top };
}
function arch(x, y, dir, lights) {
  const g = archGeom(x, y, dir);
  ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  if (!lights) {
    ctx.strokeStyle = 'rgba(40,30,10,0.25)'; ctx.lineWidth = 1.2; ctx.beginPath(); ctx.moveTo(g.left[0] + 1.5, g.left[1] + 0.6); ctx.lineTo(g.right[0] + 1.5, g.right[1] + 0.6); ctx.stroke();
    const shape = new Path2D(); shape.moveTo(g.left[0], g.left[1]); shape.lineTo(g.top[0][0], g.top[0][1]); g.top.forEach(p => shape.lineTo(p[0], p[1])); shape.lineTo(g.right[0], g.right[1]);
    ctx.strokeStyle = P.ink; ctx.lineWidth = 1.5; ctx.stroke(shape); ctx.strokeStyle = '#8a7650'; ctx.lineWidth = 0.7; ctx.stroke(shape);
  } else {
    ctx.save(); ctx.filter = blurPx(1.6); ctx.strokeStyle = 'rgba(255,214,120,0.55)'; ctx.lineWidth = 3; ctx.stroke(linePath(g.top.slice(2, -2))); ctx.restore();
    ctx.setLineDash([0.1, 1.5]); ctx.strokeStyle = 'rgba(255,248,214,1)'; ctx.lineWidth = 1.35; ctx.stroke(linePath(g.top.slice(1, -1)));
  }
  ctx.restore();
}
function snLabels(d) {
  const S5 = snShapes(d);
  const a = highAt(S5.high, 418); if (a) label('High Street', a[0] - 12.5, a[1], { font: '12px "IM Fell English SC"', size: 12, ang: a[2], ls: 0.32, haloCol: rgba('#f1e8c9', 0.9), haloW: 3.6 });
  const lab = (n, lon, lat, o = {}) => { const p = S5.proj([lon, lat]); label(n, p[0], p[1], { size: o.size || 10.5, ls: o.ls ?? 0.2, col: o.col || '#5a4a26', haloCol: rgba(P.straw, 0.75), ...o }); };
  lab('Victorian Village', -83.0083, 39.9805, { size: 11, ang: -0.05 }); lab('Italian Village', -83.0013, 39.9802, { size: 11, ang: -0.05 });
  lab('Goodale Park', -83.0069, 39.9757, { col: P.lakeInk, size: 10 }); lab('Thompson Park', -83.0089, 39.9876, { col: P.lakeInk, size: 9 });
  label('© OpenStreetMap contributors', SCREEN[2] - 12, CAPTION_TOP - 8, { font: 'italic 8px "EB Garamond"', size: 8, align: 'right', ls: 0.02, col: P.umber, haloCol: rgba(P.paper, 0.85), alpha: 0.85 });
}
function snGlints(d) { const S5 = snShapes(d); for (const y of ARCH_Y) { const a = highAt(S5.high, y); if (a) arch(a[0], a[1], a[2], true); } }
const ARCH_BOX = [190, 320, 112, 474];
// Pangur Bán, the white cat, sitting on a garden lawn: curls and spirals, as the film draws him. His tail is its own layer.
function catBody(x, y, s) {
  ctx.save(); ctx.translate(x, y); ctx.lineJoin = 'round'; ctx.lineCap = 'round';
  ctx.fillStyle = 'rgba(30,26,8,0.3)'; ctx.beginPath(); ctx.ellipse(s * 0.12, 0.2, s * 0.5, s * 0.1, 0, 0, 7); ctx.fill();
  const b = new Path2D();
  b.moveTo(-s * 0.32, 0); b.bezierCurveTo(-s * 0.46, -s * 0.32, -s * 0.34, -s * 0.62, -s * 0.12, -s * 0.7);
  b.bezierCurveTo(-s * 0.18, -s * 0.84, -s * 0.12, -s * 0.98, -s * 0.06, -s * 1.02); b.lineTo(-s * 0.02, -s * 1.16); b.lineTo(s * 0.06, -s * 1.04);
  b.quadraticCurveTo(s * 0.12, -s * 1.06, s * 0.16, -s * 1.04); b.lineTo(s * 0.26, -s * 1.15); b.lineTo(s * 0.26, -s * 0.98);
  b.bezierCurveTo(s * 0.34, -s * 0.86, s * 0.28, -s * 0.74, s * 0.18, -s * 0.7); b.bezierCurveTo(s * 0.36, -s * 0.56, s * 0.42, -s * 0.24, s * 0.32, 0); b.closePath();
  ctx.fillStyle = P.white; ctx.fill(b); ctx.save(); ctx.clip(b); ctx.globalCompositeOperation = 'multiply'; ctx.fillStyle = 'rgba(200,190,160,0.45)'; ctx.fillRect(s * 0.06, -s * 1.2, s, s * 1.3); ctx.restore();
  ctx.strokeStyle = P.ink; ctx.lineWidth = 0.5; ctx.stroke(b);
  ctx.lineWidth = 0.4; ctx.stroke(linePath(spiralPts(-s * 0.08, -s * 0.3, s * 0.03, s * 0.2, 1.6, 2.4, 1)));   // a spiral on the haunch
  ctx.beginPath(); ctx.moveTo(-s * 0.06, -s * 0.06); ctx.lineTo(-s * 0.06, -s * 0.36); ctx.moveTo(s * 0.12, 0); ctx.lineTo(s * 0.14, -s * 0.4); ctx.stroke();
  ctx.fillStyle = '#7fae3c'; for (const ex of [0.02, 0.17]) { ctx.beginPath(); ctx.ellipse(s * ex, -s * 0.9, s * 0.035, s * 0.022, 0, 0, 7); ctx.fill(); }
  ctx.fillStyle = '#d9867a'; ctx.beginPath(); ctx.arc(s * 0.1, -s * 0.84, s * 0.018, 0, 7); ctx.fill();
  ctx.restore();
}
function catTail(x, y, s) {
  const pts = []; for (let k = 0; k <= 30; k++) { const t = k / 30; pts.push([x + s * 0.3 + t * s * 0.42, y - s * 0.04 - Math.sin(t * 2.4) * s * 0.22]); }
  const curl = spiralPts(pts[30][0] - s * 0.05, pts[30][1] - s * 0.08, s * 0.01, s * 0.07, 1.2, 0.2, -1);
  const line = pts.concat(curl);
  ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round'; ctx.strokeStyle = P.ink; ctx.lineWidth = s * 0.11; ctx.stroke(linePath(line));
  ctx.strokeStyle = P.white; ctx.lineWidth = s * 0.075; ctx.stroke(linePath(line)); ctx.restore();
}

// =====================================================================================================
// The page contract.
const LEVELS = [
  { id: 'world', caption: { eyebrow: 'The world', title: 'The Earth through the leaves', text: 'Its seas turn in slow spirals, like the carvings on old Irish stones.' },
    load: async () => { const [topo, biome, extra] = await Promise.all([Kit.json(DATA + 'land-50m.json'), loadImageData('data/biome.png'), loadJSON('data/world-extra.json')]);
      return { land: topojson.feature(topo, topo.objects.land), biome, extra }; },
    layers: [
      { name: 'ground', opaque: true, draw: worldGround },
      { name: 'labels', depth: 0.04, draw: worldLabels },
      { name: 'glints', depth: 0.02, box: [C0[0] - RAD - 4, C0[1] - RAD - 4, RAD * 2 + 8, RAD * 2 + 8], motion: { kind: 'breathe', min: 0.35, max: 1, period: 7, phase: 0 }, draw: worldGlints },
      { name: 'foxglove', depth: 0.1, box: [406, 662, 80, 206], motion: { kind: 'sway', angle: 1.6, period: 8, phase: 0.3, anchor: [452, 852] }, draw: worldFoxglove },
      { name: 'spray-left', depth: 0.12, box: [60, 196, 170, 150], motion: { kind: 'sway', angle: 1.5, period: 9, phase: 0.1, anchor: [84, 222] }, draw: () => spray([84, 222], 0.78, 14, 150, { len: [32, 44] }) },
      { name: 'spray-right', depth: 0.12, box: [316, 186, 172, 150], motion: { kind: 'sway', angle: 1.5, period: 8.4, phase: 0.6, anchor: [W - 84, 212] }, draw: () => spray([W - 84, 212], Math.PI - 0.84, 13, 146, { len: [32, 44] }) },
    ] },
  { id: 'america', caption: { eyebrow: 'North America', title: 'Five pools of light', text: 'The Great Lakes lie in the woods of the north, and Ohio on the shore of the shallowest.' },
    load: americaLoad,
    layers: [
      { name: 'ground', opaque: true, draw: americaGround },
      { name: 'labels', depth: 0.04, draw: americaLabels },
      { name: 'glints', depth: 0.02, motion: { kind: 'breathe', min: 0.4, max: 1, period: 7, phase: 0.2 }, draw: americaGlints },
      { name: 'boat', depth: 0.03, box: [BOAT[0] - 26, BOAT[1] - 14, 40, 20], motion: { kind: 'bob', amp: [3, 4], period: 6.5, phase: 0 }, draw: () => curragh(...BOAT) },
      { name: 'spray-left', depth: 0.12, box: [52, 156, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 9, phase: 0.4, anchor: [70, 176] }, draw: () => spray([70, 176], 0.7, 10, 124, { len: [28, 40] }) },
      { name: 'spray-right', depth: 0.12, box: [344, 150, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 8.2, phase: 0.9, anchor: [W - 70, 170] }, draw: () => spray([W - 70, 170], Math.PI - 0.72, 10, 124, { len: [28, 40] }) },
    ] },
  { id: 'ohio', caption: { eyebrow: 'Ohio', title: 'Between the lake and the river', text: 'Flat fields where the ice once lay, and folded hills where it never reached.' },
    load: ohioLoad,
    layers: [
      { name: 'ground', opaque: true, draw: ohioGround },
      { name: 'labels', depth: 0.04, draw: ohioLabels },
      { name: 'glints', depth: 0.02, motion: { kind: 'breathe', min: 0.4, max: 1, period: 6.5, phase: 0.5 }, draw: ohioGlints },
      { name: 'geese', depth: 0.06, box: [GEESE[0] - 26, GEESE[1] - 18, 36, 34], motion: { kind: 'bob', amp: [7, 3], period: 9, phase: 0.25 }, draw: () => geese(...GEESE) },
      { name: 'spray-left', depth: 0.12, box: [52, 156, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 8.8, phase: 0.7, anchor: [70, 176] }, draw: () => spray([70, 176], 0.72, 10, 120, { len: [28, 40] }) },
      { name: 'spray-right', depth: 0.12, box: [344, 150, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 7.9, phase: 0.2, anchor: [W - 70, 170] }, draw: () => spray([W - 70, 170], Math.PI - 0.7, 10, 120, { len: [28, 40] }) },
    ] },
  { id: 'columbus', caption: { eyebrow: 'Columbus', title: 'Inside the Outerbelt', text: 'A ring road for a wall, the wood outside it, and two rivers meeting under the towers.' },
    load: columbusLoad,
    layers: [
      { name: 'ground', opaque: true, draw: columbusGround },
      { name: 'labels', depth: 0.04, draw: columbusLabels },
      { name: 'glints', depth: 0.02, box: [C0[0] - 12, C0[1] - 12, 24, 24], motion: { kind: 'breathe', min: 0.35, max: 1, period: 6, phase: 0.1 }, draw: columbusGlints },
      { name: 'deer', depth: 0.05, box: [DEER[0] - 12, DEER[1] - 21, 26, 24], motion: { kind: 'bob', amp: [2, 0], period: 8.5, phase: 0.4 }, draw: () => deer(...DEER) },
      { name: 'spray-left', depth: 0.12, box: [52, 156, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 9.2, phase: 0.3, anchor: [70, 176] }, draw: () => spray([70, 176], 0.74, 10, 118, { len: [28, 40] }) },
      { name: 'spray-right', depth: 0.12, box: [344, 150, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 8, phase: 0.8, anchor: [W - 70, 170] }, draw: () => spray([W - 70, 170], Math.PI - 0.7, 10, 118, { len: [28, 40] }) },
    ] },
  { id: 'shortnorth', caption: { eyebrow: 'Short North', title: 'Under the arches', text: 'High Street runs under its arches of lamps, and every tree is its own crown.' },
    load: snLoad,
    layers: [
      { name: 'ground', opaque: true, draw: snGround },
      { name: 'labels', depth: 0.04, draw: snLabels },
      { name: 'lamps', depth: 0.02, box: ARCH_BOX, motion: { kind: 'breathe', min: 0.45, max: 1, period: 5.5, phase: 0 }, draw: snGlints },
      { name: 'cat', depth: 0.05, box: [CAT[0] - 13, CAT[1] - 28, 26, 30], motion: { kind: 'bob', amp: [0, 0.8], period: 5, phase: 0.2 }, draw: () => catBody(...CAT) },
      { name: 'cat-tail', depth: 0.05, box: [CAT[0] + 3, CAT[1] - 14, 20, 16], motion: { kind: 'sway', angle: 9, period: 6.2, phase: 0, anchor: [CAT[0] + CAT[2] * 0.3, CAT[1] - CAT[2] * 0.04] }, draw: () => catTail(...CAT) },
      { name: 'spray-left', depth: 0.12, box: [52, 156, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 9, phase: 0.5, anchor: [70, 176] }, draw: () => spray([70, 176], 0.74, 11, 124, { len: [30, 42] }) },
      { name: 'spray-right', depth: 0.12, box: [344, 150, 150, 130], motion: { kind: 'sway', angle: 1.4, period: 8.3, phase: 0.1, anchor: [W - 70, 170] }, draw: () => spray([W - 70, 170], Math.PI - 0.72, 11, 124, { len: [30, 42] }) },
    ] },
];

window.Plates = {
  levels: LEVELS.length,
  style: {
    fonts: ['fonts/IMFellEnglishSC-Regular.ttf', 'fonts/IMFellEnglish-Italic.ttf', 'fonts/EBGaramond-Regular.ttf'],
    eyebrow: { font: 'IM_FELL_English_SC', size: 13, color: '#f09a6c', tracking: 2.6, uppercase: true },
    title: { font: 'IM_FELL_English_Italic', size: 28, color: '#f4e19a' },
    text: { font: 'EBGaramond-Regular', size: 17, color: '#efe6cb' },
    scrim: '#1d1a0ae0',
    accent: '#cf2f1c',
  },
  info(level) {
    const L = LEVELS[level - 1];
    const child = level < LEVELS.length ? Kit.childRect(Kit.lens(ZOOMS[level - 1]), Kit.lens(ZOOMS[level])) : null;
    const layers = L.layers.map(l => {
      const o = { name: l.name };
      if (l.opaque) o.opaque = true;
      if (l.depth) o.depth = l.depth;
      if (l.box) o.frame = l.box.map(v => Math.round(v * S));
      if (l.motion) { o.motion = { ...l.motion }; if (o.motion.anchor) o.motion.anchor = o.motion.anchor.map(v => Math.round(v * S)); }
      return o;
    });
    return { id: L.id, child, caption: L.caption, layers };
  },
};

// =====================================================================================================
// Drawing a layer. `all` draws every layer, in order, into one picture (for looking at a level by hand).
let LVL, DATA_LOADED, LAYER;
function seedFor(level, name) { let h = level * 7919; for (const ch of name) h = Math.imul(h ^ ch.charCodeAt(0), 16777619) >>> 0; return h; }
function render() {
  ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.clearRect(0, 0, PW, PH); css(ctx);
  const list = LAYER === 'all' ? LVL.layers : LVL.layers.filter(l => l.name === LAYER);
  if (!list.length) throw new Error('no layer ' + LAYER);
  for (const l of list) {
    seedRng(seedFor(LEVEL, l.name)); css(ctx);
    l.draw(DATA_LOADED);
    css(ctx);
    if (l.opaque) paperFinish();
  }
}
// An older exporter magnified <html> with a CSS transform instead of asking for &magnify: draw the layer again
// through that transform, at full sharpness.
function watchMagnify() {
  new MutationObserver(() => {
    const m = /translate\(([-\d.e+]+)px,\s*([-\d.e+]+)px\)\s*scale\(([-\d.e+]+)\)/.exec(document.documentElement.style.transform || '');
    if (!m) return;
    const tx = +m[1], ty = +m[2], s = +m[3];
    if (Math.abs(s - VS) < 1e-9) return;
    setView(s, -tx / s, -ty / s);
    Object.assign(cv.style, { left: VX + 'px', top: VY + 'px', width: PW / s + 'px', height: PH / s + 'px' });
    render();
  }).observe(document.documentElement, { attributes: true, attributeFilter: ['style'] });
}
let LEVEL;
async function start() {
  const q = Kit.params(); LEVEL = q.level; LAYER = q.layer; LVL = LEVELS[LEVEL - 1];
  initCanvases();
  await loadFonts();
  initTiles();
  DATA_LOADED = await LVL.load();
  // a pass plate: the level magnified by q.magnify about plate point (cx, cy), painted at full sharpness here
  if (q.magnify !== 1) setView(q.magnify, q.cx - PW / (2 * q.magnify), q.cy - PH / (2 * q.magnify));
  window.platesMagnifies = true;
  const t0 = performance.now();
  render();
  console.log(`level ${LEVEL} ${LAYER} drawn in ${((performance.now() - t0) / 1000).toFixed(1)} s`);
  watchMagnify();
}
if (Kit.params().layer !== 'none') addEventListener('DOMContentLoaded', () =>
  start().then(() => { window.platesReady = true; }, e => { console.log('ERROR ' + e.stack); window.platesReady = 'failed: ' + e.message; }));
