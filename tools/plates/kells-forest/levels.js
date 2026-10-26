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
const ZOOMS = [516, Kit.km(3000), Kit.km(760), Kit.km(48), Kit.km(1.3)];
const loadGeo = n => loadJSON(DATA + n + '.geojson');
const FC = features => ({ type: 'FeatureCollection', features });

// The ring of white dots around where the film goes next: the film's circular setting. Its radius is half the
// next plate's resting view, so as the camera dives it opens to frame the plate fading up inside it.
function nextRing(level) {
  if (level >= 5) return null;
  const w = ZOOMS[level - 1], wc = ZOOMS[level]; // the child spans PW * w / wc plate px of this plate
  return PW * w / wc / 1.4 / 2 / S;
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
  // out on the moss: a few loose leaves, twigs, fungi, flowers; the bottom stays quiet for the caption
  twig([[60, 250], [104, 236], [132, 214], [150, 190]], 2); twig([[104, 236], [114, 206]], 1.3);
  bracket(126, 222, 11, -0.5); bracket(134, 212, 8, -0.6);
  twig([[W - 60, 300], [W - 104, 280], [W - 128, 262]], 1.8);
  for (const [x, y, a] of [[118, 300, 0.4], [96, 352, 2.3], [430, 236, 1.2], [446, 330, 4.2], [70, 420, 5.1], [470, 420, 3.6], [150, 186, 1.9], [380, 200, 0.7]]) leaf(x, y, a, R(24, 30), R(11, 14), pick(lt));
  for (const [x, y, s] of [[112, 268, 15], [128, 276, 10], [100, 280, 8]]) agaric(x, y, s, { dots: 5 });
  for (const [x, y] of [[300, 236], [318, 244], [240, 230], [222, 242], [338, 232]]) flower(x, y, R(2.2, 3));
  fern(70, 330, 64, -1.25, { curl: -0.4 }); fern(478, 330, 58, -1.95, { curl: 0.35 });
  for (const [x, y, a] of [[100, 900, 1.0], [210, 930, 2.5], [360, 910, 4.0], [450, 950, 0.3], [150, 1000, 5.5], [300, 1010, 1.6]]) leaf(x, y, a, R(22, 28), R(10, 13), pick(lt), { inkA: 0.6 });
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
  const pa = at(-128, 20); if (pa) label('Pacific', pa[0], pa[1], { size: 11, ls: 0.3, col: '#3f6a5c', ang: 1.1 });
  const am = at(-100, 49); label('America', am[0] - 10, am[1], { size: 11, ls: 0.18, col: P.umber });
}
function worldFoxglove() { foxglove(476, 846, 176, { lean: -0.16, bells: 16, side: -1 }); fern(458, 846, 60, -1.85, { curl: 0.3 }); }

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
// The page contract.
const LEVELS = [
  { id: 'world', caption: { eyebrow: 'The world', title: 'The Earth through the leaves', text: 'Its seas turn in slow spirals, like the carvings on old Irish stones.' },
    load: async () => { const [topo, biome, extra] = await Promise.all([Kit.json(DATA + 'land-50m.json'), loadImageData('data/biome.png'), loadJSON('data/world-extra.json')]);
      return { land: topojson.feature(topo, topo.objects.land), biome, extra }; },
    layers: [
      { name: 'ground', opaque: true, draw: worldGround },
      { name: 'labels', depth: 0.04, draw: worldLabels },
      { name: 'glints', depth: 0.02, box: [C0[0] - RAD - 4, C0[1] - RAD - 4, RAD * 2 + 8, RAD * 2 + 8], motion: { kind: 'breathe', min: 0.35, max: 1, period: 7, phase: 0 }, draw: worldGlints },
      { name: 'foxglove', depth: 0.1, box: [436, 650, 70, 210], motion: { kind: 'sway', angle: 1.6, period: 8, phase: 0.3, anchor: [476, 846] }, draw: worldFoxglove },
      { name: 'spray-left', depth: 0.12, box: [-30, 80, 210, 210], motion: { kind: 'sway', angle: 1.2, period: 9, phase: 0.1, anchor: [40, 130] }, draw: () => spray([40, 130], 0.55, 11, 120) },
      { name: 'spray-right', depth: 0.12, box: [356, 60, 220, 220], motion: { kind: 'sway', angle: 1.2, period: 8.4, phase: 0.6, anchor: [W - 30, 112] }, draw: () => spray([W - 30, 112], Math.PI - 0.6, 11, 124) },
    ] },
  { id: 'america', caption: { eyebrow: 'North America', title: 'Five pools of light', text: 'The Great Lakes lie in the woods of the north, and Ohio on the shore of the shallowest.' },
    load: async () => ({}), layers: [{ name: 'ground', opaque: true, draw: () => paperGround('#c9cf62') }] },
  { id: 'ohio', caption: { eyebrow: 'Ohio', title: 'Between the lake and the river', text: 'Flat fields where the ice once lay, and folded hills where it never reached.' },
    load: async () => ({}), layers: [{ name: 'ground', opaque: true, draw: () => paperGround('#d8cf72') }] },
  { id: 'columbus', caption: { eyebrow: 'Columbus', title: 'Inside the Outerbelt', text: 'A ring road for a wall, the wood outside it, and two rivers meeting under the towers.' },
    load: async () => ({}), layers: [{ name: 'ground', opaque: true, draw: () => paperGround('#e2cf86') }] },
  { id: 'shortnorth', caption: { eyebrow: 'Short North', title: 'Under the arches', text: 'High Street, its lit arches, and every tree its own crown.' },
    load: async () => ({}), layers: [{ name: 'ground', opaque: true, draw: () => paperGround('#ecd58c') }] },
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
// The exporter magnifies <html> for a pass plate; draw the layer again through that transform, at full sharpness.
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
  const mag = new URLSearchParams(location.search).get('mag'); // ?mag=s,cx,cy: look at a pass plate by hand
  if (mag) { const [s, cx, cy] = mag.split(',').map(Number); setView(s, cx - PW / (2 * s), cy - PH / (2 * s)); }
  const t0 = performance.now();
  render();
  console.log(`level ${LEVEL} ${LAYER} drawn in ${((performance.now() - t0) / 1000).toFixed(1)} s`);
  watchMagnify();
}
if (Kit.params().layer !== 'none') addEventListener('DOMContentLoaded', () =>
  start().then(() => { window.platesReady = true; }, e => { console.log('ERROR ' + e.stack); window.platesReady = 'failed: ' + e.message; }));
