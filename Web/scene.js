// What is on the map and where: the projections' forward maths (mirroring cell.glsl), how each
// projection fits the screen, the lettering, the creatures and ornaments placed for a frame.
'use strict';

const R2D = 180 / Math.PI, D2R = Math.PI / 180;
const clamp = (v, a = 0, b = 1) => v < a ? a : v > b ? b : v;
const lerp = (a, b, t) => a + (b - a) * t;
const wrap180 = l => ((l + 540) % 360) - 180;

// lon/lat -> plane (the units cell.glsl uses), or null when the point is out of sight.
// pp: { proj, hemi (offsets of the two worlds), hc (the eastern hemisphere's centre), ovalLon, globe }
function forward(pp, ll) {
  const lam = ll[0] * D2R, phi = ll[1] * D2R;
  if (pp.proj === 0) {
    let p1 = pp.hc[1] * D2R, dl = lam - pp.hc[0] * D2R;
    const cE = Math.sin(p1) * Math.sin(phi) + Math.cos(p1) * Math.cos(phi) * Math.cos(dl);
    const h = cE >= 0 ? 1 : 0;
    if (!h) { p1 = -p1; dl = lam - (pp.hc[0] + 180) * D2R; }
    const cc = clamp(Math.sin(p1) * Math.sin(phi) + Math.cos(p1) * Math.cos(phi) * Math.cos(dl), -1, 1), c = Math.acos(cc), k = c < 1e-5 ? 1 : c / Math.sin(c);
    const s = h ? 1 : -1;
    return [k * Math.cos(phi) * Math.sin(dl) / 1.5707963 + s * pp.hemi[0], -k * (Math.cos(p1) * Math.sin(phi) - Math.sin(p1) * Math.cos(phi) * Math.cos(dl)) / 1.5707963 + s * pp.hemi[1], h];
  }
  if (pp.proj === 1) {
    const dl = wrap180(ll[0] - pp.ovalLon) * D2R, s = Math.sqrt(1 + Math.cos(phi) * Math.cos(dl / 2));
    return [2 * Math.cos(phi) * Math.sin(dl / 2) / s, -Math.sin(phi) / s, 0];
  }
  const dl = lam - pp.globe[0] * D2R, p0 = pp.globe[1] * D2R;
  const cosc = Math.sin(p0) * Math.sin(phi) + Math.cos(p0) * Math.cos(phi) * Math.cos(dl);
  if (cosc < 0.05) return null;
  return [Math.cos(phi) * Math.sin(dl), -(Math.cos(p0) * Math.sin(phi) - Math.sin(p0) * Math.cos(phi) * Math.cos(dl)), 0];
}
// plane -> lon/lat (for the reader's fingers), or null off the map
function inverse(pp, P) {
  if (pp.proj === 0) {
    const h = Math.hypot(P[0] - pp.hemi[0], P[1] - pp.hemi[1]) < Math.hypot(P[0] + pp.hemi[0], P[1] + pp.hemi[1]) ? 1 : 0;
    const q = [P[0] - (h ? 1 : -1) * pp.hemi[0], P[1] - (h ? 1 : -1) * pp.hemi[1]];
    const rho = Math.hypot(q[0], q[1]); if (rho > 1) return null;
    const C = h ? pp.hc : [pp.hc[0] + 180, -pp.hc[1]];
    const X = q[0] * Math.PI / 2, Y = -q[1] * Math.PI / 2, c = rho * Math.PI / 2, p1 = C[1] * D2R, sc = Math.sin(c), cc = Math.cos(c);
    const lat = Math.asin(clamp(cc * Math.sin(p1) + (c > 1e-6 ? Y * sc * Math.cos(p1) / c : 0), -1, 1));
    return [wrap180(C[0] + Math.atan2(X * sc, c * Math.cos(p1) * cc - Y * Math.sin(p1) * sc) * R2D), lat * R2D];
  }
  if (pp.proj === 1) {
    const X = P[0] * Math.SQRT2, Y = -P[1] * Math.SQRT2;
    if (P[0] * P[0] * 0.25 + P[1] * P[1] > 1) return null;
    const z = Math.sqrt(Math.max(1 - X * X / 16 - Y * Y / 4, 0));
    return [wrap180(pp.ovalLon + 2 * Math.atan2(z * X, 2 * (2 * z * z - 1)) * R2D), Math.asin(clamp(z * Y, -1, 1)) * R2D];
  }
  const x = P[0], y = -P[1], rho = Math.hypot(x, y); if (rho > 1) return null;
  const c = Math.asin(rho), sc = Math.sin(c), cc = Math.cos(c), p0 = pp.globe[1] * D2R;
  const lat = Math.asin(clamp(cc * Math.sin(p0) + (rho > 1e-6 ? y * sc * Math.cos(p0) / rho : 0), -1, 1));
  return [wrap180(pp.globe[0] + Math.atan2(x * sc, Math.max(rho, 1e-6) * cc * Math.cos(p0) - y * sc * Math.sin(p0)) * R2D), lat * R2D];
}
// great-circle interpolation between two lon/lat points
function slerpLL(a, b, t) {
  const v = ll => [Math.cos(ll[1] * D2R) * Math.cos(ll[0] * D2R), Math.cos(ll[1] * D2R) * Math.sin(ll[0] * D2R), Math.sin(ll[1] * D2R)];
  const A = v(a), B = v(b), d = clamp(A[0] * B[0] + A[1] * B[1] + A[2] * B[2], -1, 1), w = Math.acos(d);
  if (w < 1e-6) return b.slice();
  const s = Math.sin(w), ka = Math.sin((1 - t) * w) / s, kb = Math.sin(t * w) / s;
  const C = [ka * A[0] + kb * B[0], ka * A[1] + kb * B[1], ka * A[2] + kb * B[2]];
  return [Math.atan2(C[1], C[0]) * R2D, Math.asin(clamp(C[2] / Math.hypot(...C), -1, 1)) * R2D];
}
const smooth01 = (a, b, x) => { const t = clamp((x - a) / (b - a)); return t * t * (3 - 2 * t); };
// Where the projection's centre sits for a look (lon, lat, z): the classic layout when far out;
// zoomed in, the hemisphere (or oval) turns so the place looked at is undistorted.
function projParams(st, lay) {
  const L = st.look, f = smooth01(1.3, 3.2, L.z);
  const pp = { proj: st.proj, hemi: lay.hemi, hc: [70, 0], ovalLon: st.ovalLon, globe: [L.lon, L.lat] };
  if (st.proj === 0) {
    const east = ((wrap180(L.lon) + 200) % 360) < 180;   // the classic split at 20° W
    const dflt = east ? [70, 0] : [-110, 0];
    const c = slerpLL(dflt, [L.lon, L.lat], f);
    pp.hc = east ? c : [wrap180(c[0] + 180), -c[1]];
  } else if (st.proj === 1) {
    pp.ovalLon = st.ovalLon + wrap180(L.lon - st.ovalLon) * f;
  }
  return pp;
}

// how each projection sits on the screen at zoom 1
function layoutFor(proj, W, H) {
  const portrait = H > W * 1.15;
  const hemi = proj === 0 ? (portrait ? [0, 1.03] : [1.03, 0]) : [0, 0];
  const ext = proj === 0 ? (portrait ? [2.12, 4.18] : [4.18, 2.12]) : proj === 1 ? [4.2, 2.2] : [2.24, 2.24];
  // the oval is wide: on a tall screen it may run past the sides rather than be tiny
  const fit = Math.min(W * (proj === 1 && portrait ? 1.5 : 0.93) / ext[0], H * (portrait ? 0.84 : 0.8) / ext[1]);
  return { hemi, ext, fit, portrait };
}

// ------------------------------------------------------------------ what is hidden
const FINDS = [
  { id: 'brendan', c: [-31, 37.5], kind: 'brendan', eyebrow: 'A legend of the sea', title: 'The island that swam away',
    text: ['In the <i>Voyage of Saint Brendan</i>, written down in Latin by about the year 900, the monk and his crew land on a bare island in the Atlantic, light a fire to cook, and feel the ground move. The island is a whale. They call it Jasconius, and in the story they come back to it every Easter.',
      'Mapmakers kept a Saint Brendan’s Island in the Atlantic for centuries afterwards, though nobody ever found it.'],
    source: 'Navigatio Sancti Brendani Abbatis, 9th–10th century', link: 'https://en.wikipedia.org/wiki/Voyage_of_Saint_Brendan_the_Abbot' },
  { id: 'hybrasil', c: [-15.5, 51.6], kind: 'phantom', eyebrow: 'A phantom island', title: 'Hy-Brasil',
    text: ['West of Ireland, sea charts drew a round island cut in two by a channel. It appears on a portolan chart by Angelino Dulcert in 1325, as “Bracile”, and kept turning up on maps for about five hundred years.',
      'Expeditions went looking for it, and in legend it shows itself once every seven years. Here it only shows from far away: come closer and it is gone.'],
    source: 'Angelino Dulcert’s portolan chart, Majorca, 1325', link: 'https://en.wikipedia.org/wiki/Hy-Brasil' },
  { id: 'musa', c: [-3.0, 16.8], kind: 'glint', eyebrow: 'From the Catalan Atlas', title: 'The king with the golden nugget',
    text: ['The Catalan Atlas, made on Majorca in 1375 and credited to Abraham Cresques, puts a king on a throne in West Africa holding up a gold nugget. He is Mansa Musa, ruler of Mali.',
      'On his pilgrimage to Mecca in 1324 he spent and gave away so much gold in Cairo that, according to the historian al-Umari, its value there was still low twelve years later.'],
    source: 'Catalan Atlas, 1375; Bibliothèque nationale de France', link: 'https://en.wikipedia.org/wiki/Mansa_Musa' },
  { id: 'serpent', c: [-12.5, 64.2], kind: 'serpent', eyebrow: 'From the Carta Marina', title: 'The sea serpent off Norway',
    text: ['In 1539 the Swedish priest Olaus Magnus printed the Carta Marina, a map of the northern lands and seas full of creatures sailors described to him. Off the Norwegian coast a huge serpent coils around a ship.',
      'He wrote that it lived in caves along the shore near Bergen and came out on summer nights to eat calves, lambs and pigs.'],
    source: 'Olaus Magnus, Carta Marina, Venice, 1539', link: 'https://en.wikipedia.org/wiki/Carta_marina' },
  { id: 'dragons', c: [133, 14], kind: 'dragon', label: 'HC SVNT DRACONES', eyebrow: 'Here are dragons', title: 'HC SVNT DRACONES',
    text: ['“Here be dragons” is almost never on an old map. The Latin <i>hic sunt dracones</i> is known from just two globes: the Hunt–Lenox Globe of about 1510, now in the New York Public Library, and the older engraved ostrich-egg globe it seems to have been copied from.',
      'Both put it near the coast of East Asia.'],
    source: 'Hunt–Lenox Globe, about 1510', link: 'https://en.wikipedia.org/wiki/Here_be_dragons' },
  { id: 'kells', c: [-6.88, 53.73], kind: 'tower', eyebrow: 'Hibernia', title: 'The Book of Kells',
    text: ['Around the year 800, monks of the Columban order, perhaps on Iona and later at Kells in what is now County Meath, made a Gospel book of extraordinary decoration: spirals, interlace, animals woven into knots, pages coloured with orpiment, verdigris, lapis and red lead.',
      'The book survived Viking raids and a theft in 1007. It has been at Trinity College Dublin since the 1660s. The round tower still stands at Kells.'],
    source: 'Book of Kells, about 800; Trinity College Dublin', link: 'https://en.wikipedia.org/wiki/Book_of_Kells' },
  { id: 'nemo', c: [-123.4, -48.9], kind: 'glint', silver: true, eyebrow: 'The loneliest water', title: 'Point Nemo',
    text: ['The point in the ocean farthest from any land: 2,688 kilometres from the nearest island. Often the closest people are the astronauts on the International Space Station passing overhead.',
      'Space agencies aim old spacecraft at the sea around it, so the deep floor below is a spacecraft cemetery.'],
    source: 'Located by Hrvoje Lukatela, 1992', link: 'https://en.wikipedia.org/wiki/Point_Nemo' },
  { id: 'ravenna', c: [12.2, 44.42], kind: 'glint', mosaic: true, eyebrow: 'A glint in Italy', title: 'Ravenna',
    text: ['For a time in the fifth century Ravenna was the capital of the Western Roman Empire, and later of the Ostrogoths and then the Byzantines in Italy. Its churches kept their mosaics, glass and gold fifteen centuries old, and eight of its buildings are now a World Heritage Site.',
      'In San Vitale, finished in 547, the emperor Justinian and the empress Theodora still stand in gold on the walls.'],
    source: 'Early Christian Monuments of Ravenna, UNESCO, 1996', link: 'https://en.wikipedia.org/wiki/Ravenna' },
  { id: 'australis', c: [60, -73], kind: 'none', eyebrow: 'An imagined continent', title: 'Terra Australis Incognita',
    text: ['For two hundred years European world maps drew a vast southern continent, reasoning that the lands of the north needed a weight in the south to balance them. Ortelius’s world map of 1570 labels it <i>Terra Australis nondum cognita</i>, the southern land not yet known.',
      'Antarctica was first sighted in 1820, much smaller and much colder than anyone had drawn it.'],
    source: 'Abraham Ortelius, Typus Orbis Terrarum, 1570', link: 'https://en.wikipedia.org/wiki/Terra_Australis' },
];
const SHIPS = [
  { route: [[-9.5, 38.2], [-16, 27], [-32, 19], [-58, 15]], dur: 150, phase: 0.1 },
  { route: [[-14, 8], [2, -25], [18, -38], [38, -32], [54, -14], [68, 6]], dur: 200, phase: 0.55 },
  { route: [[122, 13], [150, 24], [178, 33], [-158, 36], [-128, 30], [-110, 22]], dur: 230, phase: 0.3 },
];
const WHALES = [{ c: [-140, 32], seed: 0.4 }, { c: [95, -40], seed: 0.75 }];
const ROSES = [{ c: [-45, 27], big: 1 }, { c: [74, -22], big: 1 }, { c: [-150, 6], big: 0 }, { c: [-6, -32], big: 0 }];

const OCEANS = [
  { n: 'OCEANVS ATLANTICVS', c: [-38, 18], s: 1 }, { n: 'MARE PACIFICVM', c: [-150, 22], s: 1.05 },
  { n: 'MAR DEL ZVR', c: [-120, -28], s: 0.9 }, { n: 'OCEANVS INDICVS', c: [80, -12], s: 1 },
  { n: 'OCEANVS AETHIOPICVS', c: [-8, -22], s: 0.85 }, { n: 'MARE GLACIALE', c: [40, 78], s: 0.8 },
  { n: 'MARE MEDITERRANEVM', c: [18.5, 34.3], s: 0.62, min: 1.1 },
];
const CONTINENTS = [
  { n: 'EVROPA', c: [18, 53] }, { n: 'ASIA', c: [92, 48] }, { n: 'AFRICA', c: [20, 6] },
  { n: 'AMERICA SEPTENTRIONALIS', c: [-102, 48] }, { n: 'AMERICA MERIDIONALIS', c: [-60, -12] },
  { n: 'NOVA HOLLANDIA', c: [134, -25] }, { n: 'TERRA AVSTRALIS INCOGNITA', c: [20, -77] }, { n: 'HIBERNIA', c: [-13.2, 54.2], min: 2.6 },
];
const LINES = [{ n: 'CIRCVLVS AEQVINOCTIALIS', lat: 0 }, { n: 'TROPICVS CANCRI', lat: 23.44 }, { n: 'TROPICVS CAPRICORNI', lat: -23.44 },
  { n: 'CIRCVLVS ARCTICVS', lat: 66.56 }, { n: 'CIRCVLVS ANTARCTICVS', lat: -66.56 }];

function shipAt(s, t) {
  if (!s.legs) {
    const dist = (a, b) => { const p1 = a[1] * D2R, p2 = b[1] * D2R, dl = (b[0] - a[0]) * D2R; return Math.acos(clamp(Math.sin(p1) * Math.sin(p2) + Math.cos(p1) * Math.cos(p2) * Math.cos(dl), -1, 1)); };
    s.legs = []; s.tot = 0;
    for (let i = 0; i < s.route.length - 1; i++) { const d = dist(s.route[i], s.route[i + 1]); s.legs.push({ a: s.route[i], b: s.route[i + 1], d }); s.tot += d; }
  }
  let u = ((t / s.dur + s.phase) % 1) * 2, dir = 1;
  if (u > 1) { u = 2 - u; dir = -1; }
  let d = u * s.tot;
  for (const l of s.legs) {
    if (d <= l.d) { const f = d / l.d; let dl = l.b[0] - l.a[0]; if (dl > 180) dl -= 360; if (dl < -180) dl += 360; return { ll: [l.a[0] + dl * f, lerp(l.a[1], l.b[1], f)], dir }; }
    d -= l.d;
  }
  return { ll: s.legs[s.legs.length - 1].b, dir };
}
function moonPhase(now) { return (((now / 86400000 - 10962.76) / 29.530588853) % 1 + 1) % 1; }

class Scene {
  constructor(places) { this.places = places; this.measure = new Map(); }

  // Build what the renderer needs for one frame from a state (see tour.js for its fields).
  view(st, W, H, dpr, time, tctx) {
    const lay = layoutFor(st.proj, W, H);
    const pp = projParams(st, lay);
    const L = st.look, scale = lay.fit * L.z;
    const tp = st.proj === 2 ? [0, 0] : (forward(pp, [L.lon, L.lat]) || [0, 0]);
    const ov = st.proj === 2 ? 0 : L.ov;
    const cam = { x: lerp(tp[0], 0, ov) + (L.dx || 0), y: lerp(tp[1], 0, ov) + (L.dy || 0) };
    const v = {
      W, H, dpr, time, proj: st.proj, hemi: lay.hemi, hc: pp.hc, ovalLon: pp.ovalLon, globe: pp.globe, pp,
      cam, scale, grain: st.grain, ink: st.ink, relief: st.relief,
      reveal: st.reveal, sun: st.sun, sunAz: st.sunAz, stars: [cam.x * scale * 0.25, cam.y * scale * 0.25],
      sprites: [], roses: [], marks: [], knock: [], labels: [],
    };
    const cw = Math.max(4, Math.round(st.grain * dpr)) / dpr, ch = cw * 2;
    const pxPerDeg = scale / (st.proj === 2 ? 57.3 : 90);
    v.ppx = pxPerDeg;
    v.detail = pxPerDeg / cw;
    const toS = P => [(P[0] - v.cam.x) * scale + W / 2, (P[1] - v.cam.y) * scale + H / 2];
    const at = ll => { const P = forward(pp, ll); return P ? Object.assign(toS(P), { h: P[2] }) : null; };
    const vis = (p, m = 120) => p && p[0] > -m && p[0] < W + m && p[1] > -m && p[1] < H + m;
    v.at = at; v.toS = toS; v.lay = lay; v.cwPx = cw; v.chPx = ch;
    const zl = Math.log2(st.look.z), phoneW = Math.min(W, H) < 600;
    // Hibernia's roundel, centred on Ireland
    const ie = at([-7.9, 53.4]);
    v.kells = [-7.9, 53.4, clamp(pxPerDeg * 5.6, ch * 12, Math.min(W, H) * 0.47), st.kells];
    // compass roses and their rhumb lines
    for (const r of ROSES) {
      const p = at(r.c); if (!p) continue;
      const R = Math.max(ch * 3.5, (st.proj === 0 ? lay.fit : lay.fit * 0.8) * 0.17 * Math.pow(st.look.z, 0.45) * (r.big ? 1 : 0.8));
      v.roses.push([p[0], p[1], R, p.h]);
      if (vis(p, R * 1.3)) v.sprites.push({ a: [p[0], p[1], R, 5], b: [1, 0, 0, 0] });
    }
    // the creatures: hand-drawn ASCII art (tools/sprites.py), the same size in characters at any zoom
    v.keepOut = []; v.creatures = [];
    // drawn on a grid k times coarser than the map's, k growing as the map comes closer
    const kArt = Math.max(1, Math.min(4, Math.round((phoneW ? 1.3 : 1.15) * Math.pow(st.look.z, 0.36) * (2.6 / (st.grain || 2.6)))));
    const wanted = [];
    const creature = (art, p, frame, flip, pri) => wanted.push({ art, p, frame, flip, pri });
    for (const s of SHIPS) { const { ll, dir } = shipAt(s, time); const p = at(ll); if (vis(p, 200)) creature('ship', p, time * 1.5 + s.phase * 7, dir < 0, 1); }
    for (const w of WHALES) { const p = at(w.c); if (vis(p, 200) && st.look.z > 1.2) creature('whale', p, (time * 0.7 + w.seed * 3) % 3, w.seed > 0.5, 2); }
    for (const f of FINDS) {
      const p = at(f.c); f.screen = p; if (!vis(p, 260)) continue;
      if (f.kind === 'serpent') { if (st.look.z > 1.5) creature('serpent', p, time * 1.4, false, 3); }
      else if (f.kind === 'dragon') { if (st.look.z > 1.5) creature('dragon', p, time * 1.2, false, 3); }
      else if (f.kind === 'brendan') creature('brendan', p, time * 3, false, 3);
      else if (f.kind === 'phantom') { const fade = clamp((5.5 - st.look.z) / 2.5); if (fade > 0.05) v.sprites.push({ a: [p[0], p[1], ch * (2.2 + 1.2 * zl), 10], b: [1, 0, 0, 0] }); }
      else if (f.kind === 'glint') v.sprites.push({ a: [p[0], p[1], ch * (1.5 + 0.7 * Math.max(zl, 0)), 9], b: [1, Math.abs(f.c[0] * 0.13) % 1, f.mosaic ? 2 : f.silver ? 1 : 0, 0] });
      else if (f.kind === 'tower' && st.kells > 0.3) v.sprites.push({ a: [p[0], p[1] + ch * 0.5, ch * (1.2 + 0.9 * zl), 11], b: [1, 0, 0, 0] });
    }
    // place the creatures, the hidden ones first; one that would overlap another stays away
    wanted.sort((a, b) => b.pri - a.pri);
    for (const c of wanted) {
      const name = c.flip ? c.art + '_flip' : c.art, m = TI.ART && TI.ART[name]; if (!m) continue;
      const k = kArt, ax = Math.floor(c.p[0] / cw), ay = Math.floor(c.p[1] / ch);
      const x0 = (ax - m.anchor[0] * k) * cw, y0 = (ay - m.anchor[1] * k) * ch, box = [x0, y0, x0 + m.w * k * cw, y0 + m.h * k * ch];
      if (box[0] > W || box[1] > H || box[2] < 0 || box[3] < 0) continue;
      if (v.keepOut.some(b => box[0] < b[2] && box[2] > b[0] && box[1] < b[3] && box[3] > b[1])) continue;
      v.creatures.push({ art: name, x: c.p[0], y: c.p[1], frame: ((Math.floor(c.frame) % m.frames) + m.frames) % m.frames, k, x0, y0 });
      v.keepOut.push(box);
    }
    // the ornaments on the sheet: a title ribbon, the moon, and (two hemispheres) the winds
    const sheet = (P, size) => [toS(P)[0], toS(P)[1], size * scale];
    if (st.ornaments > 0) {
      const top = st.proj === 0 ? (lay.portrait ? -2.05 - 0.28 : -1.0 - 0.28) : st.proj === 1 ? -1.0 - 0.22 : -1.0 - 0.18;
      const rb = sheet([0, top], lay.portrait ? 0.62 : 0.55);
      v.sprites.push({ a: [rb[0], rb[1], rb[2], 7], b: [1, 0.16, 0, 0] });
      v.ribbon = rb;
      const mp = st.proj === 0 ? (lay.portrait ? [0.92, -1.9] : [2.0, -0.92]) : st.proj === 1 ? [1.95, -0.95] : [0.95, -0.95];
      const mo = sheet(mp, 0.085);
      v.sprites.push({ a: [mo[0], mo[1], mo[2], 6], b: [1, moonPhase(st.date), 0, 0] });
      if (st.proj === 0) {
        const winds = lay.portrait ? [[-0.92, 0, 1, 'ZEPHYRVS'], [0.92, 0, -1, 'EVRVS']] : [[-2.0, 0.92, 1, 'ZEPHYRVS'], [2.0, 0.92, -1, 'NOTVS'], [-2.0, -0.92, 1, 'BOREAS']];
        v.winds = [];
        for (const w of winds) { const s = sheet([w[0], w[1]], 0.06); v.sprites.push({ a: [s[0], s[1], s[2], 8], b: [w[2], 0, 0, 0] }); v.winds.push({ x: s[0], y: s[1] + s[2] * 2.2, n: w[3] }); }
      }
    }
    v.sprites = v.sprites.slice(0, 16);
    // capitals
    if (v.detail > 1.25 && st.labels > 0) {
      const floor = v.detail > 3.2 ? 0 : v.detail > 2 ? 1.5e6 : 4e6;
      const caps = this.places.capitals.filter(c => c.pop > floor).map(c => ({ c, p: at(c.c) })).filter(o => vis(o.p, 0));
      caps.sort((a, b) => b.c.pop - a.c.pop);
      for (const o of caps.slice(0, 32)) v.marks.push([o.p[0], o.p[1], 0, o.c.pop > 8e6 ? 1 : 0]);
      v.capitals = caps.slice(0, 32);
    }
    if (tctx) this.letter(v, st, tctx, at, vis);
    return v;
  }

  // The lettering: Latin for the great seas and lands, names for the rest, laid out so nothing
  // overlaps; each word knocks back the characters under it.
  letter(v, st, x, at, vis) {
    const L = TI.INKS[st.ink].label, k = st.look.z, d = v.detail, W = v.W, H = v.H;
    const items = [], add = (text, p, o) => { if (p && vis(p, 40)) items.push(Object.assign({ text, x: p[0], y: p[1] }, o)); };
    const SC = '"IM Fell English SC", "IM Fell English", Georgia, serif', IT = '"IM Fell English", Georgia, serif';
    const R = v.lay.fit, z = Math.sqrt(k);
    if (st.labels > 0) {
      for (const c of CONTINENTS) if (d < 4 && (!c.min || d > c.min)) add(c.n, at(c.c), { size: clamp(R * 0.056 * z, 10, 26), font: SC, color: L.continent, sp: 0.42, pri: 100 });
      for (const o of OCEANS) if (!o.min || d > o.min) add(o.n, at(o.c), { size: clamp(R * 0.052 * z * o.s, 9.5, 22), font: IT, italic: true, color: L.ocean, sp: 0.3, pri: 90 });
      if (d > 1.3) for (const s of this.places.seas) if (s.r <= (d > 2.4 ? 2 : 1) && s.n !== 'Mediterranean Sea') add(s.n, at(s.c), { size: clamp(9.5 * Math.pow(k, 0.22), 10, 14), font: IT, italic: true, color: L.sea, sp: 0.12, pri: 55 });
      if (d > 0.8) for (const c of this.places.countries) {
        if ((c.min || 4) > 1.6 + d * 1.6) continue;
        add(c.n.toUpperCase(), at(c.c), { size: clamp(8.5 * Math.pow(k, 0.28) + 1, 9, 14), font: SC, color: L.country, sp: 0.14, pri: 60 - c.rank * 3 });
      }
      if (v.capitals) for (const o of v.capitals) add(o.c.n, [o.p[0] + v.cwPx * 1.2, o.p[1]], { size: clamp(9 * Math.pow(k, 0.15), 10, 12.5), font: IT, italic: true, color: L.capital, sp: 0, pri: 40 + Math.log10(o.c.pop + 10), left: true });
      if (d > 0.9 && st.proj !== 2) for (const l of LINES) add(l.n, at([-150 + (l.lat === 0 ? 20 : 0), l.lat + 1.8]), { size: clamp(8.5 * Math.pow(k, 0.2), 9, 12), font: IT, italic: true, color: L.line, sp: 0.16, pri: 20 });
      for (const f of FINDS) if (f.label && d > 1.0) add(f.label, at([f.c[0] + 2, f.c[1] + 6.5]), { size: clamp(9.5 * Math.pow(k, 0.3), 10, 16), font: IT, italic: true, color: L.find, sp: 0.18, pri: 70 });
      if (v.winds) for (const w of v.winds) add(w.n, [w.x, w.y], { size: clamp(10 * z, 9, 20), font: IT, italic: true, color: L.ocean, sp: 0.25, pri: 30 });
    }
    if (v.ribbon && st.ornaments > 0) items.push({ text: st.title || 'TERRA INCOGNITA', x: v.ribbon[0], y: v.ribbon[1] + 1, size: clamp(v.ribbon[2] * 0.11, 11, 64), font: SC, color: L.title, sp: 0.34, pri: 200, keep: true });
    items.sort((a, b) => b.pri - a.pri);
    const placed = (v.keepOut || []).map(b => b.slice());
    x.save(); x.setTransform(v.dpr, 0, 0, v.dpr, 0, 0); x.textBaseline = 'middle'; x.lineJoin = 'round';
    for (const it of items) {
      x.font = `${it.italic ? 'italic ' : ''}${it.size.toFixed(1)}px ${it.font}`;
      const sp = it.sp * it.size, w = this.width(x, it.text, sp, it.size, it.font, it.italic), h = it.size * 0.82;
      const x0 = it.left ? it.x : it.x - w / 2;
      const box = [x0 - 3, it.y - h / 2 - 2, x0 + w + 3, it.y + h / 2 + 2];
      if (box[2] < 0 || box[0] > W || box[3] < 0 || box[1] > H) continue;
      if (!it.keep && placed.some(b => box[0] < b[2] && box[2] > b[0] && box[1] < b[3] && box[3] > b[1])) continue;
      placed.push(box);
      Object.assign(it, { w, x0, spx: sp });
      v.labels.push(it);
      if (v.knock.length < 40) v.knock.push(box);
    }
    x.restore();
  }
  width(x, text, sp, size, font, italic) {
    const key = text + '|' + size.toFixed(1) + font + italic;
    let w = this.measure.get(key);
    if (w === undefined) { w = [...text].reduce((a, c) => a + x.measureText(c).width, 0) + sp * Math.max(0, [...text].length - 1); this.measure.set(key, w); if (this.measure.size > 4000) this.measure.clear(); }
    return w;
  }
  drawLabels(v, x, alpha, renderer) {
    x.setTransform(1, 0, 0, 1, 0, 0);
    x.clearRect(0, 0, x.canvas.width, x.canvas.height);
    // the creatures first, pixel-aligned to the map's grid
    if (renderer && v.creatures) {
      const cwDev = Math.max(4, Math.round(v.grain * v.dpr)), chDev = cwDev * 2;
      x.imageSmoothingEnabled = false;
      for (const c of v.creatures) {
        const bm = renderer.creatureBitmap(c.art, c.frame, c.k, cwDev, chDev, v.ink);
        x.drawImage(bm, Math.round(c.x0 / v.cwPx) * cwDev, Math.round(c.y0 / v.chPx) * chDev);
      }
    }
    if (alpha <= 0) return;
    // each word is set once, halo and letters, into a bitmap of its own; a frame only places them
    x.save(); x.globalAlpha = alpha; x.imageSmoothingEnabled = true;
    for (const l of v.labels) {
      // set at a whole pixel size and scaled the rest of the way, so zooming reuses the bitmaps
      const bm = this.labelBitmap(l, v), k = l.size / bm.size;
      x.drawImage(bm.c, (l.x0 - bm.pad * k) * v.dpr, (l.y - bm.h * k / 2) * v.dpr, bm.c.width * k, bm.c.height * k);
    }
    x.restore();
  }
  labelBitmap(l, v) {
    const size = Math.max(6, Math.round(l.size));
    const key = [l.text, size, l.font, l.italic ? 1 : 0, l.color, v.ink, v.dpr].join('|');
    this.bitmaps = this.bitmaps || new Map();
    let bm = this.bitmaps.get(key);
    if (bm) return bm;
    const font = `${l.italic ? 'italic ' : ''}${size}px ${l.font}`, sp = l.sp * size;
    const m = document.createElement('canvas').getContext('2d');
    m.font = font;
    const widths = [...l.text].map(ch => m.measureText(ch).width);
    const w = widths.reduce((a, b) => a + b, 0) + sp * Math.max(0, widths.length - 1);
    const pad = Math.ceil(size * 0.3), h = Math.ceil(size * 1.5);
    const c = document.createElement('canvas');
    c.width = Math.ceil((w + pad * 2) * v.dpr); c.height = Math.ceil(h * v.dpr);
    const x = c.getContext('2d');
    x.setTransform(v.dpr, 0, 0, v.dpr, 0, 0);
    x.font = font; x.textBaseline = 'middle'; x.lineJoin = 'round';
    x.lineWidth = Math.max(2.2, size * 0.13); x.strokeStyle = TI.INKS[v.ink].label.halo; x.fillStyle = l.color;
    for (const pass of ['stroke', 'fill']) {
      let cx = pad;
      [...l.text].forEach((ch, i) => { pass === 'stroke' ? x.strokeText(ch, cx, h / 2) : x.fillText(ch, cx, h / 2); cx += widths[i] + sp; });
    }
    bm = { c, pad, h, size };
    if (this.bitmaps.size > 600) this.bitmaps.clear();
    this.bitmaps.set(key, bm);
    return bm;
  }
  findAt(v, px, py) {
    let best = null, bd = 1e9;
    for (const c of v.creatures || []) {
      const m = TI.ART[c.art];
      if (px >= c.x0 && px <= c.x0 + m.w * c.k * v.cwPx && py >= c.y0 && py <= c.y0 + m.h * c.k * v.chPx) {
        const f = FINDS.find(f => f.kind === c.art.replace('_flip', ''));
        if (f) return f;
      }
    }
    for (const f of FINDS) {
      if (!f.screen) continue;
      const d = Math.hypot(px - f.screen[0], py - f.screen[1]);
      const r = Math.max(28, f.kind === 'serpent' || f.kind === 'dragon' ? v.chPx * 8 : f.kind === 'brendan' ? v.chPx * 6 : v.chPx * 4);
      if (d < r && d < bd) { bd = d; best = f; }
    }
    return best;
  }
}

window.TI = Object.assign(window.TI || {}, { Scene, forward, inverse, projParams, slerpLL, layoutFor, FINDS, clamp, lerp, smooth01, wrap180 });
