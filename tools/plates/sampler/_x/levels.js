// The five scales of the sampler, each a plate of linen worked through the shared lens.
'use strict';
const Levels = (() => {
  const W = Kit.W, H = Kit.H, CX = W / 2, CY = H / 2;
  const C = 14, P = C / 2;                         // a stitch over two threads of the cloth
  const OX = ((CX % C) + C) % C, OY = ((CY % C) + C) % C;   // the plate's centre is a hole
  const D = name => Kit.json(`../../data/${name}`);

  // ---- threads ---------------------------------------------------------------------------------------
  const T = {}, pal = [null];
  const thread = (name, hex, o = {}) => { T[name] = pal.length; pal.push({ rgb: Stitch.rgb(hex), metal: !!o.metal, lot: o.lot }); };
  thread('ink', '#3b2d24'); thread('walnut', '#5e4130'); thread('brown', '#7a5a3e');
  thread('madder', '#a2402e'); thread('madderDk', '#7f2e22'); thread('madderLt', '#c06a50'); thread('terra', '#c98068'); thread('rose', '#d29a87'); thread('blush', '#e0bba8');
  thread('indigo', '#2f4868'); thread('indigoMd', '#4d6b8a'); thread('indigoLt', '#7f9bb2'); thread('blueGrey', '#a7b8c2');
  thread('ochre', '#bf8d3c'); thread('ochreLt', '#d6b064'); thread('wheat', '#dcc07f'); thread('straw', '#e2cc92');
  thread('sageDk', '#5d6f49'); thread('sage', '#879870'); thread('sageLt', '#aab78f'); thread('olive', '#7d7a47'); thread('moss', '#8f8e57');
  thread('oat', '#d7c6a2'); thread('oatDk', '#bda97f'); thread('cream', '#efe7d2');
  thread('plum', '#7b5266'); thread('plumLt', '#a88294');
  thread('gold', '#c9a24e', { metal: true }); thread('goldDk', '#a58233', { metal: true });
  // the charts' letters
  const LEG = { r: T.madder, R: T.madderDk, p: T.rose, g: T.sage, G: T.sageDk, l: T.sageLt, o: T.ochre, y: T.straw,
    b: T.indigo, B: T.indigoLt, k: T.ink, w: T.cream, n: T.walnut, a: T.gold, m: T.oat, t: T.indigoMd, s: T.oatDk, h: T.plumLt };

  const cloth = seed => ({ P, ox: OX, oy: OY, rgb: Stitch.rgb('#e8dfca'), seed });
  const grid = () => Geo.grid(C, OX, OY);
  const fine = () => Geo.grid(P, OX, OY);         // over one thread: lettering, small motifs
  const crossGrid = (G, seed, o = {}) => ({ c: G.c, ox: G.ox, oy: G.oy, ncol: G.ncol, nrow: G.nrow, data: G.data, hw: o.hw ?? 0.21, seed, shadow: o.shadow });

  // ---- names -----------------------------------------------------------------------------------------
  // a name cross-stitched over one thread, centred on (x, y); returns its cells on the fine grid
  function place(F, text, x, y, size, col, gap = 1) {
    const { cells, w, h } = Charts.letters(text, size, gap);
    const i0 = Math.round((x - F.ox) / F.c - w / 2), j0 = Math.round((y - F.oy) / F.c - h / 2);
    const out = cells.map(([i, j]) => [i0 + i, j0 + j]);
    for (const [i, j] of out) if (i >= 0 && j >= 0 && i < F.ncol && j < F.nrow) F.data[j * F.ncol + i] = col;
    return { cells: out, box: [F.ox + i0 * F.c, F.oy + j0 * F.c, w * F.c, h * F.c] };
  }
  // the ground leaves bare linen round a name: cells of the coarse grid near its letters
  function reserve(G, F, names, pad = 1) {
    const clear = new Set();
    for (const n of names) for (const [i, j] of n.cells) {
      for (let dj = -pad; dj <= pad; dj++) for (let di = -pad; di <= pad; di++) {
        const x = F.ox + (i + di + 0.5) * F.c, y = F.oy + (j + dj + 0.5) * F.c;
        const [gi, gj] = Geo.cellAt(G, x, y);
        clear.add(gj * G.ncol + gi);
      }
    }
    for (const k of clear) if (k >= 0 && k < G.data.length) G.data[k] = 0;
    return (x, y) => { const [gi, gj] = Geo.cellAt(G, x, y); return clear.has(gj * G.ncol + gi); };
  }
  function append(S, src) {
    const a = src.a;
    for (let k = 0; k < src.n; k++) { const i = k * 13; S.push(a[i], a[i + 1], a[i + 2], a[i + 3], a[i + 4], a[i + 5], a[i + 6], { s0: a[i + 7], phase: a[i + 8], tier: a[i + 9], shadow: a[i + 10], taper: a[i + 11], mode: a[i + 12] }); }
  }
  // drops freeform stitches whose middle falls where `bad` says
  function filterSegs(S, bad) {
    const a = S.a, out = new Stitch.Segs();
    for (let k = 0; k < S.n; k++) {
      const i = k * 13;
      if (bad((a[i] + a[i + 2]) / 2, (a[i + 1] + a[i + 3]) / 2)) continue;
      if ((out.n + 1) * 13 > out.a.length) { const b = new Float32Array(out.a.length * 2); b.set(out.a); out.a = b; }
      out.a.set(a.subarray(i, i + 13), out.n * 13); out.n++;
    }
    return out;
  }

  // ---- I. the world ----------------------------------------------------------------------------------
  const R1 = 410;
  const world = {
    id: 'world', proj: () => Kit.lens(R1),
    caption: { eyebrow: 'I · Worked in silk upon linen', title: 'The Whole World', text: 'A globe as girls once stitched them, ringed in gold.' },
    sprites: () => {
      const proj = Kit.lens(R1), [x, y] = proj([-44, 29]);
      const s = Geo.chartSize(Charts.M.ship);
      const F = fine(), [i, j] = Geo.cellAt(F, x, y);
      const i0 = i - Math.floor(s.w / 2), j0 = j - s.h + 2;
      const fx = F.ox + i0 * F.c - 10, fy = F.oy + j0 * F.c - 10;
      return { ship: { i0, j0, frame: [Math.floor(fx), Math.floor(fy), Math.ceil(s.w * F.c + 20), Math.ceil((s.h + 3) * F.c + 20)] } };
    },
    layers() {
      const sp = this.sprites();
      return [
        { name: 'ground', opaque: true },
        { name: 'sheen', depth: 0, motion: { kind: 'sheen', period: 9, width: 0.3, angle: 24, color: '#fff2c4', opacity: 0.6 } },
        { name: 'labels', depth: 0.04 },
        { name: 'ship', depth: 0.04, frame: sp.ship.frame, motion: { kind: 'bob', amp: [1.5, 3], period: 7.5, phase: 0.2 } },
      ];
    },
    async plan() {
      const proj = this.proj(), G = grid(), F = fine(), S = new Stitch.Segs(), gold = new Stitch.Segs();
      const topo = await D('countries-50m.json'), landT = await D('land-50m.json');
      const countries = topojson.feature(topo, topo.objects.countries).features;
      const land = topojson.feature(landT, landT.objects.land);
      // the names first: the ground leaves linen round them
      const names = [
        place(F, 'ATLANTIC', ...proj([-38, 17]), 5, T.indigo),
        place(F, 'PACIFIC', ...proj([-117, 12]), 5, T.indigo),
      ];
      // the continents, each in its own wool
      const groupOf = f => {
        const [lon, lat] = d3.geoCentroid(f), n = f.properties.name;
        if (n === 'Greenland' || lat < -58) return 'ice';
        if (n === 'Canada') return 'canada';
        if (n === 'United States of America') return 'usa';
        if (lon < -25) return lat > 7 ? 'mexico' : 'south';
        if (lat > 36 && lon < 45) return 'europe';
        if (lat < 37 && lon < 52) return 'africa';
        return 'asia';
      };
      const COL = { ice: T.cream, canada: T.sageLt, usa: T.sage, mexico: T.moss, south: T.ochreLt, europe: T.plumLt, africa: T.rose, asia: T.oatDk };
      Geo.fill(G, proj, land, T.oatDk);
      const groups = {};
      for (const f of countries) (groups[groupOf(f)] ||= []).push(f);
      for (const g in groups) Geo.fill(G, proj, { type: 'FeatureCollection', features: groups[g] }, COL[g]);
      // the wreath: a vine round the globe, strawberries and leaves counted on it
      const RV = R1 + 60, N = 20;
      for (let k = 0; k < N; k++) {
        const a = -Math.PI / 2 + (k + 0.5) * 2 * Math.PI / N, x = CX + RV * Math.cos(a), y = CY + RV * Math.sin(a);
        const [i, j] = Geo.cellAt(G, x, y), m = k % 2 ? Charts.M.leaf : Charts.M.berry, s = Geo.chartSize(m);
        Geo.stamp(G, m, LEG, i - Math.floor(s.w / 2), j - Math.floor(s.h / 2), { mirror: (k % 4 === 1) !== (Math.cos(a) < 0) });
      }
      const vine = d3.range(0, 2 * Math.PI + 0.01, 0.01).map(a => [CX + RV * Math.cos(a), CY + RV * Math.sin(a)]);
      const vs = new Stitch.Segs();
      Geo.stem(vs, vine, { len: 12, hw: 1.7, col: T.sageDk });
      const stitched = (x, y) => { const [i, j] = Geo.cellAt(G, x, y); return G.data[j * G.ncol + i] > 0; };
      append(S, filterSegs(vs, stitched));
      // two birds on a bough above the globe
      const top = CY - R1 - 250;
      const [ti, tj] = Geo.cellAt(G, CX, top), ts = Geo.chartSize(Charts.M.tree), bs = Geo.chartSize(Charts.M.bird);
      Geo.stamp(G, Charts.M.tree, LEG, ti - Math.floor(ts.w / 2), tj - Math.floor(ts.h / 2));
      Geo.stamp(G, Charts.M.bird, LEG, ti - Math.floor(ts.w / 2) - bs.w - 1, tj - 2);
      Geo.stamp(G, Charts.M.bird, LEG, ti + Math.ceil(ts.w / 2) + 1, tj - 2, { mirror: true });
      // and below, a band of tulips, as a sampler closes above its verse
      const tl = Geo.chartSize(Charts.M.tulip), bandY = CY + R1 + 196;
      for (let k = -4; k <= 4; k++) {
        const [i, j] = Geo.cellAt(G, CX + k * 104, bandY);
        Geo.stamp(G, Charts.M.tulip, { ...LEG, r: k % 2 ? T.indigoMd : T.madder, p: k % 2 ? T.indigoLt : T.rose }, i - 2, j - 3);
      }
      const free = reserve(G, F, names, 1);
      // coasts in backstitch
      for (const l of Geo.lines(proj, land)) if (Geo.length(l) > 40) Geo.backstitch(S, Geo.simplify(l, 1.2), { len: 8, hw: 1.05, col: T.walnut, taper: 0.75 });
      // the seas: water lines in running stitch, following the coasts out
      const M = Geo.mask(2, ctx => { ctx.beginPath(); d3.geoPath(proj, ctx)(land); ctx.fill(); });
      const dist = Geo.blur(Geo.edt(M), M.w, M.h, 3);
      const inside = p => Math.hypot(p[0] - CX, p[1] - CY) < R1 - 7;
      const iso = Geo.isolines(dist, M, [6, 13, 21], inside);
      const style = [{ len: 8, gap: 5, hw: 1.3, col: T.indigoMd }, { len: 8, gap: 7, hw: 1.15, col: T.indigoLt }, { len: 7, gap: 10, hw: 1.0, col: T.blueGrey }];
      const water = new Stitch.Segs();
      iso.forEach((lv, n) => lv.forEach((l, q) => { if (Geo.length(l) > 70) Geo.running(water, Geo.chaikin(l, 1), { ...style[n], seed: 100 * n + q }); }));
      // the globe's lines of latitude and longitude, faint, over the sea only
      const grat = new Stitch.Segs();
      for (const l of Geo.lines(proj, d3.geoGraticule().step([30, 30])())) Geo.running(grat, l, { len: 6, gap: 8, hw: 0.9, col: T.oatDk, seed: 7 });
      const onLand = (x, y) => { const i = Math.floor(x / 2), j = Math.floor(y / 2); return dist[j * M.w + i] < 4; };
      append(S, filterSegs(grat, onLand));
      append(S, filterSegs(water, (x, y) => free(x, y)));
      // the globe ringed in couched gold, tied down with madder
      const ring = r => d3.range(0, 2 * Math.PI + 0.004, 0.004).map(a => [CX + r * Math.cos(a), CY + r * Math.sin(a)]);
      for (const [r, ph] of [[R1 + 11, 0.2], [R1 + 19, 0.7]]) {
        for (const T2 of [S, gold]) Geo.couch(T2, ring(r), { hw: 2.7, col: T.gold, tack: T2 === S ? T.madder : 0, every: 30, tackPhase: ph, phase: ph, shadow: T2 === S ? 1 : 0 });
      }
      // Columbus, at the heart of the lens: a gold knot
      for (const T2 of [S, gold]) Geo.knot(T2, CX, CY, 5.5, T.gold, { shadow: T2 === S ? 1 : 0 });
      // the ship's sprite
      const sp = this.sprites().ship, SH = fine();
      Geo.stamp(SH, Charts.M.ship, LEG, sp.i0, sp.j0);
      const wv = new Stitch.Segs();
      const [sx, sy, sw, sh] = sp.frame;
      for (let r = 0; r < 2; r++) Geo.running(wv, d3.range(sx + 12 + r * 9, sx + sw - 10, 3).map(x => [x, sy + sh - 22 + r * 7 + 2.5 * Math.sin(x / 9 + r * 2)]), { len: 9, gap: 5, hw: 1.2, col: r ? T.indigoLt : T.indigoMd, seed: 41 + r });
      return {
        ground: { linen: cloth(11), pal, grids: [crossGrid(G, 21)], segs: S },
        labels: { linen: null, pal, grids: [crossGrid(F, 31, { hw: 0.22 })], segs: new Stitch.Segs() },
        sheen: { linen: null, pal, grids: [], segs: gold },
        ship: { linen: null, pal, grids: [crossGrid(SH, 41, { hw: 0.22 })], segs: wv },
      };
    },
  };


  // ---- shared stitches --------------------------------------------------------------------------------
  // the land mask (land drawn, lakes cut out) and the distance of the sea from it, at 2 px a sample
  function seaField(draw, blurR = 3) {
    const M = Geo.mask(2, draw);
    return { M, dist: Geo.blur(Geo.edt(M), M.w, M.h, blurR) };
  }
  // water lines: running stitch following the coasts out to sea, each row lighter
  const WATER = [{ len: 8, gap: 5, hw: 1.3, col: 'indigoMd' }, { len: 8, gap: 7, hw: 1.15, col: 'indigoLt' }, { len: 7, gap: 10, hw: 1.0, col: 'blueGrey' }];
  function waterLines(field, at, keep, minLen = 70, rows = WATER) {
    const out = new Stitch.Segs();
    Geo.isolines(field.dist, field.M, at, keep).forEach((lv, n) => lv.forEach((l, q) => {
      if (Geo.length(l) > minLen) Geo.running(out, Geo.chaikin(l, 1), { ...rows[Math.min(n, rows.length - 1)], col: T[rows[Math.min(n, rows.length - 1)].col], seed: 100 * n + q });
    }));
    return out;
  }
  // colours regions so that no two neighbours share a thread
  function patchwork(G, proj, feats, toneOf) {
    const ids = new Uint16Array(G.data.length), save = G.data;
    feats.forEach((f, k) => {
      G.data = new Uint8Array(save.length);
      const cov = Geo.fill(G, proj, f, 1);
      for (let q = 0; q < cov.length; q++) if (cov[q] >= 8) ids[q] = k + 1;
    });
    G.data = save;
    const nb = feats.map(() => new Set());
    for (let j = 0; j < G.nrow; j++) for (let i = 0; i < G.ncol; i++) {
      const a = ids[j * G.ncol + i];
      if (!a) continue;
      for (const [di, dj] of [[1, 0], [0, 1]]) {
        if (i + di >= G.ncol || j + dj >= G.nrow) continue;
        const b = ids[(j + dj) * G.ncol + i + di];
        if (b && b !== a) { nb[a - 1].add(b - 1); nb[b - 1].add(a - 1); }
      }
    }
    const tone = feats.map(() => -1);
    feats.map((f, k) => k).sort((a, b) => nb[b].size - nb[a].size).forEach(k => {
      const tones = toneOf(feats[k]);
      if (typeof tones === 'number') { tone[k] = tones; return; }
      const used = new Set([...nb[k]].map(n => tone[n]));
      tone[k] = tones.find(t => !used.has(t)) ?? tones[k % tones.length];
    });
    for (let q = 0; q < ids.length; q++) if (ids[q]) G.data[q] = tone[ids[q] - 1];
    if (window.TONES_LOG) console.log(JSON.stringify(Object.fromEntries(feats.map((f, k) => [f.properties.name, Object.keys(T).find(n => T[n] === tone[k])]))));
    return ids;
  }
  const FC = features => ({ type: 'FeatureCollection', features });
  // a few named places, as French knots
  function knots(S, proj, places, r, col) {
    for (const p of places) { const [x, y] = proj(p.geometry ? p.geometry.coordinates : p); Geo.knot(S, x, y, r, col); }
  }

  // ---- II. North America -----------------------------------------------------------------------------
  const america = {
    id: 'america', proj: () => Kit.lens(Kit.km(5200)),
    caption: { eyebrow: 'II · A map of the continent', title: 'North America', text: 'The states pieced like a quilt, and Ohio worked in madder.' },
    sprites() {
      const proj = this.proj(), [x, y] = proj([-72.6, 33.9]);
      const s = Geo.chartSize(Charts.M.serpent), F = fine(), [i, j] = Geo.cellAt(F, x, y);
      const i0 = i - Math.floor(s.w / 2), j0 = j - Math.floor(s.h / 2);
      return { serpent: { i0, j0, frame: [Math.floor(F.ox + i0 * F.c - 10), Math.floor(F.oy + j0 * F.c - 10), Math.ceil(s.w * F.c + 20), Math.ceil(s.h * F.c + 20)] } };
    },
    layers() {
      const sp = this.sprites();
      return [
        { name: 'ground', opaque: true },
        { name: 'sheen', depth: 0, motion: { kind: 'sheen', period: 9, width: 0.3, angle: 24, color: '#fff2c4', opacity: 0.6, phase: 0.35 } },
        { name: 'labels', depth: 0.04 },
        { name: 'serpent', depth: 0.04, frame: sp.serpent.frame, motion: { kind: 'bob', amp: [2, 3.5], period: 8.5, phase: 0.6 } },
      ];
    },
    async plan() {
      const proj = this.proj(), G = grid(), F = fine(), S = new Stitch.Segs(), gold = new Stitch.Segs();
      const [topo, landT, land10, states, lakes, rivers, places] = await Promise.all(['countries-50m.json', 'land-50m.json',
        'america-land.geojson', 'america-states.geojson', 'america-lakes.geojson', 'america-rivers.geojson', 'america-places.geojson'].map(D));
      const land50 = topojson.feature(landT, landT.objects.land), countries = topojson.feature(topo, topo.objects.countries).features;
      const bigLakes = FC(lakes.features.filter(f => d3.geoArea(f) > 3e-5));
      const ohio = states.features.find(f => f.properties.name === 'Ohio');
      const names = [
        place(F, 'ATLANTIC', ...proj([-71.5, 27.2]), 7, T.indigo),
        place(F, 'OCEAN', ...proj([-71.5, 25.4]), 7, T.indigo),
        place(F, 'GULF OF MEXICO', ...proj([-90.5, 25.2]), 5, T.indigo),
        place(F, 'HUDSON BAY', ...proj([-86, 59.5]), 5, T.indigo),
      ];
      // land: the world's coast where the continent's 10 m data stops, the 10 m inside
      Geo.fill(G, proj, land50, T.oat);
      Geo.fill(G, proj, land10, T.oat);
      const tones = {
        USA: [T.sage, T.wheat, T.sageLt, T.oat], CAN: [T.oat, T.sageLt, T.cream, T.straw], MEX: [T.rose, T.straw, T.ochreLt, T.moss],
      };
      patchwork(G, proj, states.features, f => f.properties.name === 'Ohio' ? T.terra : tones[f.properties.country] || [T.oat]);
      // the lands beyond the continent, by country
      for (const f of countries) {
        const [lon, lat] = d3.geoCentroid(f), n = f.properties.name;
        if (['United States of America', 'Canada', 'Mexico'].includes(n)) continue;
        Geo.fill(G, proj, f, n === 'Greenland' ? T.cream : lat < 12 ? T.ochreLt : T.moss);
      }
      // the great lakes are linen, like the sea
      const lakeCov = Geo.coverage(G, ctx => { ctx.beginPath(); d3.geoPath(proj, ctx)(bigLakes); ctx.fill(); });
      for (let k = 0; k < lakeCov.length; k++) if (lakeCov[k] >= 8) G.data[k] = 0;
      // a few pines in the northern woods
      for (const [lon, lat, m] of [[-76, 50.5], [-71, 49], [-88, 51.5], [-80, 52.5], [-95, 53.5], [-67, 52]]) {
        const [x, y] = proj([lon, lat]), [i, j] = Geo.cellAt(G, x, y);
        Geo.stamp(G, PINE, { g: T.sageDk, n: T.walnut }, i - 2, j - 3);
      }
      const free = reserve(G, F, names, 1);
      // coasts and lake shores in backstitch
      const coastLines = [...Geo.lines(proj, land10), ...Geo.lines(proj, land50).filter(l => l.some(([x, y]) => proj.invert([x, y])[1] > 74 || proj.invert([x, y])[1] < 9 || proj.invert([x, y])[0] > -52))];
      for (const l of coastLines) if (Geo.length(l) > 40 && Geo.onPlate(l, 20)) Geo.backstitch(S, Geo.simplify(l, 1), { len: 9, hw: 1.1, col: T.walnut, taper: 0.75 });
      for (const l of Geo.lines(proj, bigLakes)) if (Geo.length(l) > 30) Geo.backstitch(S, Geo.simplify(l, 1), { len: 8, hw: 1.0, col: T.indigoMd, taper: 0.75 });
      // the borders between the three countries, in madder running stitch
      const borders = topojson.mesh(topo, topo.objects.countries, (a, b) => a !== b);
      for (const l of Geo.lines(proj, borders)) if (Geo.length(l) > 20) Geo.running(S, Geo.simplify(l, 1), { len: 8, gap: 5, hw: 1.15, col: T.madderDk, seed: 9 });
      // the rivers
      const rv = new Stitch.Segs();
      for (const l of Geo.lines(proj, rivers)) if (Geo.length(l) > 150) Geo.backstitch(rv, Geo.simplify(l, 1.2), { len: 9, hw: 1.0, col: T.indigoMd, taper: 0.7 });
      append(S, filterSegs(rv, (x, y) => free(x, y)));
      // the seas
      const field = seaField(ctx => {
        ctx.beginPath(); d3.geoPath(proj, ctx)(land50); ctx.fill(); ctx.beginPath(); d3.geoPath(proj, ctx)(land10); ctx.fill();
        ctx.globalCompositeOperation = 'destination-out'; ctx.beginPath(); d3.geoPath(proj, ctx)(bigLakes); ctx.fill();
      });
      append(S, filterSegs(waterLines(field, [6, 13, 21], () => true, 60), (x, y) => free(x, y)));
      // the great cities in madder knots, Columbus in gold
      knots(S, proj, places.features.filter(p => p.properties.pop > 3e6 && !/Columbus/.test(p.properties.name)), 4.2, T.madderDk);
      // Ohio edged in couched gold
      for (const l of Geo.lines(proj, ohio)) for (const T2 of [S, gold]) Geo.couch(T2, Geo.chaikin(Geo.simplify(l, 0.8), 1), { hw: 2.4, col: T.gold, tack: T2 === S ? T.madderDk : 0, every: 22, shadow: T2 === S ? 1 : 0 });
      for (const T2 of [S, gold]) Geo.knot(T2, CX, CY, 5.5, T.gold, { shadow: T2 === S ? 1 : 0 });
      // the serpent's sprite, swimming in its own waves
      const sp = this.sprites().serpent, SP = fine();
      Geo.stamp(SP, Charts.M.serpent, LEG, sp.i0, sp.j0);
      return {
        ground: { linen: cloth(12), pal, grids: [crossGrid(G, 22)], segs: S },
        labels: { linen: null, pal, grids: [crossGrid(F, 32, { hw: 0.22 })], segs: new Stitch.Segs() },
        sheen: { linen: null, pal, grids: [], segs: gold },
        serpent: { linen: null, pal, grids: [crossGrid(SP, 42, { hw: 0.22 })], segs: new Stitch.Segs() },
      };
    },
  };
  const PINE = `
..g..
.ggg.
ggggg
.ggg.
ggggg
..n..`;

  // ---- III. Ohio ---------------------------------------------------------------------------------------
  // the neighbours keep the threads the continent's quilt gave them (see america's patchwork), so the
  // dive lands on the same colours
  const NEIGHBOURS = { Michigan: 'sage', Indiana: 'oat', Kentucky: 'sageLt', 'West Virginia': 'oat', Pennsylvania: 'wheat', 'New York': 'sage',
    Ontario: 'sageLt', Illinois: 'wheat', Tennessee: 'wheat', Virginia: 'sage', Maryland: 'sageLt', Wisconsin: 'oat', 'North Carolina': 'sageLt' };
  const ohioLevel = {
    id: 'ohio', proj: () => Kit.lens(Kit.km(520)),
    caption: { eyebrow: 'III · The state, pieced in fields', title: 'Ohio', text: 'Between Lake Erie and the Ohio River, Columbus knotted in gold.' },
    sprites() {
      const proj = this.proj(), [x, y] = proj([-84.55, 42.62]);
      const s = Geo.chartSize(CARDINAL), G = grid(), [i, j] = Geo.cellAt(G, x, y);
      const i0 = i - Math.floor(s.w / 2), j0 = j - Math.floor(s.h / 2);
      return { cardinal: { i0, j0, frame: [Math.floor(G.ox + i0 * G.c - 12), Math.floor(G.oy + j0 * G.c - 12), Math.ceil(s.w * G.c + 24), Math.ceil(s.h * G.c + 24)] } };
    },
    layers() {
      const sp = this.sprites();
      return [
        { name: 'ground', opaque: true },
        { name: 'sheen', depth: 0, motion: { kind: 'sheen', period: 9, width: 0.3, angle: 24, color: '#fff2c4', opacity: 0.6, phase: 0.5 } },
        { name: 'labels', depth: 0.04 },
        { name: 'cardinal', depth: 0.05, frame: sp.cardinal.frame, motion: { kind: 'sway', angle: 2.2, period: 6.5, phase: 0.1, anchor: [sp.cardinal.frame[0] + sp.cardinal.frame[2] / 2, sp.cardinal.frame[1] + sp.cardinal.frame[3] - 14] } },
      ];
    },
    async plan() {
      const proj = this.proj(), G = grid(), F = fine(), S = new Stitch.Segs(), gold = new Stitch.Segs();
      const [land10, states, lakes, rivers, places] = await Promise.all(['america-land.geojson', 'ohio-states.geojson', 'ohio-lakes.geojson', 'ohio-rivers.geojson', 'ohio-places.geojson'].map(D));
      const ohio = states.features.find(f => f.properties.name === 'Ohio');
      const city = n => places.features.find(p => p.properties.name === n);
      const at = n => proj(city(n).geometry.coordinates);
      const names = [
        place(F, 'LAKE ERIE', ...proj([-81.75, 42.12]), 7, T.indigo),
        place(F, 'COLUMBUS', at('Columbus')[0] - 150, at('Columbus')[1] + 52, 7, T.ink),
        place(F, 'CLEVELAND', at('Cleveland')[0] + 18, at('Cleveland')[1] + 30, 5, T.ink),
        place(F, 'TOLEDO', at('Toledo')[0] - 6, at('Toledo')[1] + 30, 5, T.ink),
        place(F, 'DAYTON', at('Dayton')[0], at('Dayton')[1] + 30, 5, T.ink),
        place(F, 'CINCINNATI', at('Cincinnati')[0] + 112, at('Cincinnati')[1] - 2, 5, T.ink),
        place(F, 'AKRON', at('Akron')[0] + 8, at('Akron')[1] + 30, 5, T.ink),
      ];
      Geo.fill(G, proj, land10, T.oat);
      for (const f of states.features) {
        if (f.properties.name === 'Ohio') continue;
        Geo.fill(G, proj, f, T[NEIGHBOURS[f.properties.name] || 'oat']);
      }
      // Ohio pieced in fields: blocks of the counted grid, as the land was surveyed
      const B = 5, bo = [2, 1];
      const blockOf = k => [Math.floor((k % G.ncol + bo[0]) / B), Math.floor((Math.floor(k / G.ncol) + bo[1]) / B)];
      const ohioCov = Geo.fill(G, proj, ohio, k => { const [bi, bj] = blockOf(k); return Stitch.hash(bi, bj, 77) < 0.5 ? T.terra : T.rose; });
      const lakeCov = Geo.coverage(G, ctx => { ctx.beginPath(); d3.geoPath(proj, ctx)(lakes); ctx.fill(); });
      for (let k = 0; k < lakeCov.length; k++) if (lakeCov[k] >= 8) G.data[k] = 0;
      const free = reserve(G, F, names, 1);
      // lake shores in backstitch, the water lines out from them
      for (const l of Geo.lines(proj, lakes)) if (Geo.length(l) > 30 && Geo.onPlate(l, 20)) Geo.backstitch(S, Geo.simplify(l, 0.8), { len: 9, hw: 1.15, col: T.indigoMd, taper: 0.7 });
      const field = seaField(ctx => {
        ctx.beginPath(); d3.geoPath(proj, ctx)(states); ctx.fill(); ctx.beginPath(); d3.geoPath(proj, ctx)(land10); ctx.fill();
        ctx.globalCompositeOperation = 'destination-out'; ctx.beginPath(); d3.geoPath(proj, ctx)(lakes); ctx.fill();
      });
      append(S, filterSegs(waterLines(field, [6, 13, 21], () => true, 60), (x, y) => free(x, y)));
      // rivers: the Ohio couched broad in indigo, its tributaries narrower
      const rv = new Stitch.Segs();
      for (const f of rivers.features) {
        const n = f.properties.name, big = n === 'Ohio', mid = ['Scioto', 'Muskingum', 'Kanawha', 'Wabash', 'Tuscarawas', 'Kentucky', 'Allegheny', 'Monongahela'].includes(n);
        for (const l of Geo.lines(proj, f)) {
          if (Geo.length(l) < 40) continue;
          const line = Geo.chaikin(Geo.simplify(l, 1), 2);
          if (big) Geo.couch(rv, line, { hw: 3.4, col: T.indigoMd, tack: T.indigoLt, every: 26, phase: 0.1 });
          else if (mid) Geo.couch(rv, line, { hw: 2.0, col: T.indigoMd, tack: T.blueGrey, every: 24, phase: 0.6 });
          else Geo.backstitch(rv, line, { len: 9, hw: 1.2, col: T.indigoMd, taper: 0.7 });
        }
      }
      append(S, filterSegs(rv, (x, y) => free(x, y)));
      // the survey's lines quilted across Ohio's fields, along the counted grid
      const quilt = new Stitch.Segs(), inOhio = (x, y) => { const [i, j] = Geo.cellAt(G, x, y); return ohioCov[j * G.ncol + i] >= 8; };
      for (let i = 0; i <= G.ncol; i++) if ((i + bo[0]) % B === 0) Geo.running(quilt, [[G.ox + i * C, 0], [G.ox + i * C, H]], { len: 7, gap: 7, hw: 0.95, col: T.blush, phase: 0, vary: 0, wobble: 0.3, seed: i, tier: 1 });
      for (let j = 0; j <= G.nrow; j++) if ((j + bo[1]) % B === 0) Geo.running(quilt, [[0, G.oy + j * C], [W, G.oy + j * C]], { len: 7, gap: 7, hw: 0.95, col: T.blush, phase: 0, vary: 0, wobble: 0.3, seed: j + 999, tier: 1 });
      append(S, filterSegs(quilt, (x, y) => !inOhio(x, y) || free(x, y) || !inOhio(x + 7, y + 7) || !inOhio(x - 7, y - 7)));
      // Ohio edged in couched gold, as on the continent's map, except where the Ohio River is its edge
      const ohioRiver = Geo.mask(2, ctx => { ctx.lineWidth = 16; ctx.beginPath(); d3.geoPath(proj, ctx)(rivers.features.find(f => f.properties.name === 'Ohio')); ctx.stroke(); });
      const byRiver = (x, y) => ohioRiver.m[Math.floor(y / 2) * ohioRiver.w + Math.floor(x / 2)] > 0;
      for (const l of Geo.lines(proj, ohio)) for (const T2 of [S, gold]) {
        const g = new Stitch.Segs();
        Geo.couch(g, Geo.chaikin(Geo.simplify(l, 0.8), 1), { hw: 2.6, col: T.gold, tack: T2 === S ? T.madderDk : 0, every: 24, shadow: T2 === S ? 1 : 0, tier: 2 });
        append(T2, filterSegs(g, byRiver));
      }
      // towns as French knots; Columbus a gold one in a star of straight stitches
      const towns = places.features.filter(p => p.properties.pop > 60000 && p.properties.name !== 'Columbus');
      knots(S, proj, towns, 5, T.walnut);
      const [cx, cy] = at('Columbus');
      for (const T2 of [S, gold]) {
        for (let k = 0; k < 8; k++) {
          const a = k * Math.PI / 4, r0 = 10, r1 = k % 2 ? 19 : 26;
          T2.push(cx + r0 * Math.cos(a), cy + r0 * Math.sin(a), cx + r1 * Math.cos(a), cy + r1 * Math.sin(a), 1.9, T.gold, 0, { taper: 0.6, tier: 3, shadow: T2 === S ? 1 : 0 });
        }
        Geo.knot(T2, cx, cy, 8, T.gold, { shadow: T2 === S ? 1 : 0, tier: 3 });
      }
      // the cardinal, Ohio's bird, on a bough of its own
      const sp = this.sprites().cardinal, CG = grid();
      Geo.stamp(CG, CARDINAL, { r: T.madder, R: T.madderDk, k: T.ink, o: T.ochre, n: T.walnut, g: T.sage, G: T.sageDk }, sp.i0, sp.j0);
      return {
        ground: { linen: cloth(13), pal, grids: [crossGrid(G, 23)], segs: S },
        labels: { linen: null, pal, grids: [crossGrid(F, 33, { hw: 0.22 })], segs: new Stitch.Segs() },
        sheen: { linen: null, pal, grids: [], segs: gold },
        cardinal: { linen: null, pal, grids: [crossGrid(CG, 43)], segs: new Stitch.Segs() },
      };
    },
  };
  const CARDINAL = `
....r........
...rr........
..rrrr.......
..rrrrr......
.kkkrkr......
okkkrrr......
.kkkrrrr.....
..krrrrRr....
...rrrRRRr...
...rrrRRRRr..
....rrrRRRRr.
.G...rrrrRRRr
nnnnnnonoonRR
.GGnnn..G..RR
..G....GGG...`;

  // ---- IV. Columbus ------------------------------------------------------------------------------------
  const STREAMS = { 'Scioto Riv': 2.8, 'Olentangy Riv': 2.4, 'Big Walnut Crk': 2.0, 'Alum Crk': 1.9, 'Big Darby Crk': 2.0, 'Blacklick Crk': 1.6,
    'Little Darby Crk': 1.5, 'Rocky Fork': 1.4, 'Walnut Crk': 1.6, 'Little Walnut Crk': 1.4 };
  const columbus = {
    id: 'columbus', proj: () => Kit.lens(Kit.km(38)),
    caption: { eyebrow: 'IV · The city in its ring', title: 'Columbus', text: 'The Outerbelt couched in madder, two rivers meeting downtown.' },
    sprites() {
      const proj = this.proj(), [x, y] = proj([-83.046, 40.104]);
      const s = Geo.chartSize(HERON), G = fine(), [i, j] = Geo.cellAt(G, x, y);
      const i0 = i - Math.floor(s.w / 2), j0 = j - s.h + 1;
      return { heron: { i0, j0, frame: [Math.floor(G.ox + i0 * G.c - 10), Math.floor(G.oy + j0 * G.c - 10), Math.ceil(s.w * G.c + 20), Math.ceil(s.h * G.c + 20)] } };
    },
    layers() {
      const sp = this.sprites();
      return [
        { name: 'ground', opaque: true },
        { name: 'sheen', depth: 0, motion: { kind: 'sheen', period: 9, width: 0.3, angle: 24, color: '#fff2c4', opacity: 0.6, phase: 0.65 } },
        { name: 'labels', depth: 0.04 },
        { name: 'heron', depth: 0.04, frame: sp.heron.frame, motion: { kind: 'sway', angle: 1.6, period: 8, phase: 0.3, anchor: [sp.heron.frame[0] + sp.heron.frame[2] / 2, sp.heron.frame[1] + sp.heron.frame[3] - 10] } },
      ];
    },
    async plan() {
      const proj = this.proj(), G = grid(), F = fine(), S = new Stitch.Segs(), gold = new Stitch.Segs();
      const [cityF, roads, streams, water] = await Promise.all(['columbus-city.geojson', 'columbus-roads.geojson', 'columbus-streams.geojson', 'columbus-water.geojson'].map(D));
      const lakes = FC(water.features.filter(f => /Lk|Reservoir/.test(f.properties.name) && d3.geoArea(f) > 1.5e-9));
      const names = [
        place(F, 'COLUMBUS', ...proj([-83.065, 39.93]), 7, T.ink),
        place(F, 'SCIOTO', ...proj([-83.105, 40.075]), 5, T.indigo),
        place(F, 'OLENTANGY', ...proj([-82.97, 40.135]), 5, T.indigo),
        place(F, 'ALUM CREEK', ...proj([-82.905, 40.255]), 5, T.indigo),
      ];
      // the farms all round, pieced and quilted as on the state's map; the towns in oatmeal
      const B = 5, bo = [3, 2];
      const blockOf = k => [Math.floor((k % G.ncol + bo[0]) / B), Math.floor((Math.floor(k / G.ncol) + bo[1]) / B)];
      G.data.fill(0);
      for (let k = 0; k < G.data.length; k++) { const [bi, bj] = blockOf(k); G.data[k] = Stitch.hash(bi, bj, 78) < 0.5 ? T.terra : T.rose; }
      const town = Geo.coverage(G, ctx => { ctx.beginPath(); d3.geoPath(proj, ctx)(cityF); ctx.fill(); });
      // close the city's ragged limits: a cell is town if most of its neighbourhood is
      const isTown = new Uint8Array(town.length);
      for (let j = 0; j < G.nrow; j++) for (let i = 0; i < G.ncol; i++) {
        let s = 0, n = 0;
        for (let dj = -2; dj <= 2; dj++) for (let di = -2; di <= 2; di++) {
          const ii = i + di, jj = j + dj;
          if (ii < 0 || jj < 0 || ii >= G.ncol || jj >= G.nrow) continue;
          s += town[jj * G.ncol + ii]; n += 16;
        }
        if (s / n > 0.5) isTown[j * G.ncol + i] = 1;
      }
      for (let k = 0; k < isTown.length; k++) if (isTown[k]) G.data[k] = T.oat;
      // woods along the rivers and creeks, green through the fields and the town
      const riverFeats = [...streams.features, ...water.features].filter(f => STREAMS[f.properties.name] && !/Lk|Reservoir/.test(f.properties.name));
      const wood = Geo.coverage(G, ctx => { ctx.lineWidth = 26; ctx.lineJoin = 'round'; ctx.beginPath(); d3.geoPath(proj, ctx)(FC(riverFeats)); ctx.stroke(); ctx.fill(); });
      for (let k = 0; k < wood.length; k++) if (wood[k] >= 7) G.data[k] = isTown[k] ? T.sageLt : T.sage;
      const lakeCov = Geo.coverage(G, ctx => { ctx.beginPath(); d3.geoPath(proj, ctx)(lakes); ctx.fill(); });
      for (let k = 0; k < lakeCov.length; k++) if (lakeCov[k] >= 6) G.data[k] = 0;
      const free = reserve(G, F, names, 1);
      const S0 = new Stitch.Segs();
      // the quilting of the farms
      const farm = (x, y) => { const [i, j] = Geo.cellAt(G, x, y), k = j * G.ncol + i; return G.data[k] === T.rose || G.data[k] === T.terra; };
      for (let i = 0; i <= G.ncol; i++) if ((i + bo[0]) % B === 0) Geo.running(S0, [[G.ox + i * C, 0], [G.ox + i * C, H]], { len: 7, gap: 7, hw: 0.95, col: T.blush, phase: 0, vary: 0, wobble: 0.3, seed: i, tier: 0 });
      for (let j = 0; j <= G.nrow; j++) if ((j + bo[1]) % B === 0) Geo.running(S0, [[0, G.oy + j * C], [W, G.oy + j * C]], { len: 7, gap: 7, hw: 0.95, col: T.blush, phase: 0, vary: 0, wobble: 0.3, seed: j + 999, tier: 0 });
      append(S, filterSegs(S0, (x, y) => !farm(x, y) || !farm(x + 7, y + 7) || !farm(x - 7, y - 7)));
      // and the town's blocks quilted finer, in oatmeal
      const S1 = new Stitch.Segs(), inTown = (x, y) => { const [i, j] = Geo.cellAt(G, x, y); return G.data[j * G.ncol + i] === T.oat; };
      for (let i = 0; i <= G.ncol; i++) if ((i + bo[0]) % 3 === 0) Geo.running(S1, [[G.ox + i * C, 0], [G.ox + i * C, H]], { len: 6, gap: 8, hw: 0.85, col: T.brown, phase: 0, vary: 0, wobble: 0.3, seed: i + 3, tier: 0 });
      for (let j = 0; j <= G.nrow; j++) if ((j + bo[1]) % 3 === 0) Geo.running(S1, [[0, G.oy + j * C], [W, G.oy + j * C]], { len: 6, gap: 8, hw: 0.85, col: T.brown, phase: 0, vary: 0, wobble: 0.3, seed: j + 1999, tier: 0 });
      append(S, filterSegs(S1, (x, y) => !inTown(x + 7, y + 7) || !inTown(x - 7, y - 7) || free(x, y)));
      // the reservoirs: linen, a shore of indigo backstitch and a water line within
      for (const l of Geo.lines(proj, lakes)) if (Geo.length(l) > 30) Geo.backstitch(S, Geo.simplify(l, 0.8), { len: 8, hw: 1.15, col: T.indigoMd, taper: 0.7 });
      const field = seaField(ctx => { ctx.fillRect(-10, -10, W + 20, H + 20); ctx.globalCompositeOperation = 'destination-out'; ctx.beginPath(); d3.geoPath(proj, ctx)(lakes); ctx.fill(); }, 1);
      append(S, waterLines(field, [5], () => true, 40, [{ len: 7, gap: 5, hw: 1.15, col: 'indigoLt' }]));
      // rivers and creeks couched in indigo
      // (the rivers are drawn in the census data as narrow shapes: their two banks, stitched, make one thread)
      const rv = new Stitch.Segs();
      for (const f of [...streams.features, ...water.features]) {
        const w = STREAMS[f.properties.name];
        if (!w || /Lk|Reservoir/.test(f.properties.name)) continue;
        for (const l of Geo.lines(proj, f)) if (Geo.length(l) > 12) Geo.backstitch(rv, Geo.chaikin(Geo.simplify(l, 0.7), 1), { len: 8, hw: w * 0.62, col: T.indigoMd, taper: 0.5 });
      }
      append(S, filterSegs(rv, (x, y) => free(x, y)));
      // roads: Broad Street in walnut, High Street couched in ochre (the city's cross), the interstates in madder
      const rd = new Stitch.Segs();
      const HIGH = /^(N|S) High St$/, BROAD = /^(E|W) Broad St$/, FREE = /Outerbelt|^I- \d|Innerbelt|^(North|South|East|West) Fwy$/;
      for (const f of roads.features) if (BROAD.test(f.properties.name) && f.properties.class !== 'interstate')
        for (const l of Geo.lines(proj, f)) Geo.couch(rd, Geo.simplify(l, 0.8), { hw: 1.8, col: T.walnut, tack: T.oatDk, every: 20, tier: 1 });
      for (const f of roads.features) if (HIGH.test(f.properties.name))
        for (const l of Geo.lines(proj, f)) Geo.couch(rd, Geo.simplify(l, 0.8), { hw: 2.3, col: T.ochre, tack: T.walnut, every: 20, tier: 1 });
      for (const f of roads.features) if (f.properties.class === 'interstate' && FREE.test(f.properties.name))
        for (const l of Geo.lines(proj, f)) Geo.couch(rd, Geo.chaikin(Geo.simplify(l, 0.8), 1), { hw: 3.0, col: T.madder, tack: T.madderDk, every: 24, tier: 2 });
      append(S, filterSegs(rd, (x, y) => free(x, y)));
      // Short North, where the film is going: a gold star
      for (const T2 of [S, gold]) {
        for (let k = 0; k < 8; k++) {
          const a = k * Math.PI / 4 + Math.PI / 8, r0 = 11, r1 = k % 2 ? 21 : 29;
          T2.push(CX + r0 * Math.cos(a), CY + r0 * Math.sin(a), CX + r1 * Math.cos(a), CY + r1 * Math.sin(a), 2.0, T.gold, 0, { taper: 0.6, tier: 4, shadow: T2 === S ? 1 : 0 });
        }
        Geo.knot(T2, CX, CY, 8.5, T.gold, { shadow: T2 === S ? 1 : 0, tier: 4 });
      }
      const sp = this.sprites().heron, SG = fine();
      Geo.stamp(SG, HERON, { B: T.blueGrey, t: T.indigoLt, k: T.ink, o: T.ochre }, sp.i0, sp.j0);
      return {
        ground: { linen: cloth(14), pal, grids: [crossGrid(G, 24)], segs: S },
        labels: { linen: null, pal, grids: [crossGrid(F, 34, { hw: 0.22 })], segs: new Stitch.Segs() },
        sheen: { linen: null, pal, grids: [], segs: gold },
        heron: { linen: null, pal, grids: [crossGrid(SG, 44, { hw: 0.22 })], segs: new Stitch.Segs() },
      };
    },
  };
  const HERON = `
...BB......
..BBkB.....
..BBBBoooo.
...BB......
....BB.....
....BB.....
...BB......
..BBB......
..BBBB.....
.BBtBBB....
.BBttBBB...
..BBttBBB..
...BBttBBB.
....BBBBB.B
.....k.k...
.....k.k...
.....k.k...
....kk.kk..`;

  // ---- V. Short North ----------------------------------------------------------------------------------
  const ROADS = { primary: 3.0, secondary: 2.3, tertiary: 2.2, residential: 1.7, unclassified: 1.6, living_street: 1.5, motorway: 3.3, trunk: 3.0,
    motorway_link: 2.2, primary_link: 2.0, secondary_link: 1.8, tertiary_link: 1.6 };
  // the houses that line High Street, stood up as on a sampler: R roof, w wall, b window, d door, c chimney
  const HOUSES = [`
..R..
.RRR.
RRRRR
wbwbw
wwwww
wbdbw`, `
RRRRR
wbwbw
wwwww
wbwbw
wwwww
bwdwb`, `
...c.
.RRc.
RRRRR
wbwbw
wwwww
wwdww`, `
.RRR.
RRRRR
wbbbw
wwwww
wbwbw
wwdww`, `
..R..
.RRR.
RRRRR
wwbww
wbwbw
wbdbw`];
  const shortnorth = {
    id: 'shortnorth', proj: () => Kit.lens(Kit.km(1.3)),
    caption: { eyebrow: 'V · Finished, and signed in the corner', title: 'Short North', text: 'High Street under its arches, every house a few stitches of brick.' },
    sprites() {
      const proj = this.proj(), [x, y] = proj([-82.9998, 39.9893]);
      const s = Geo.chartSize(BUTTERFLY), G = fine(), [i, j] = Geo.cellAt(G, x, y);
      const i0 = i - Math.floor(s.w / 2), j0 = j - Math.floor(s.h / 2);
      return { butterfly: { i0, j0, frame: [Math.floor(G.ox + i0 * G.c - 10), Math.floor(G.oy + j0 * G.c - 10), Math.ceil(s.w * G.c + 20), Math.ceil(s.h * G.c + 20)] } };
    },
    layers() {
      const sp = this.sprites();
      return [
        { name: 'ground', opaque: true },
        { name: 'sheen', depth: 0, motion: { kind: 'sheen', period: 8, width: 0.3, angle: 24, color: '#fff2c4', opacity: 0.65, phase: 0.8 } },
        { name: 'labels', depth: 0.04 },
        { name: 'butterfly', depth: 0.08, frame: sp.butterfly.frame, motion: { kind: 'bob', amp: [2.5, 4], period: 6.5, phase: 0.4 } },
      ];
    },
    async plan() {
      const proj = this.proj(), G = grid(), F = fine(), HG = fine(), S = new Stitch.Segs(), gold = new Stitch.Segs();
      const [bld, roads, green, trees, rail, water] = await Promise.all(['shortnorth-buildings.geojson', 'shortnorth-roads.geojson', 'shortnorth-green.geojson',
        'shortnorth-trees.geojson', 'shortnorth-rail.geojson', 'shortnorth-water.geojson'].map(D));
      // High Street: its x and direction at any height of the plate
      const along = roads.features.filter(f => f.properties.name === 'North High Street').flatMap(f => Geo.lines(proj, f)).flat().sort((a, b) => a[1] - b[1]);
      const atY = y => {
        let k = along.findIndex(p => p[1] >= y); if (k < 1) k = 1; if (k < 0) k = along.length - 1;
        let a = along[k - 1], b = along[k];
        // the two carriageways can interleave: look a little way off for the direction
        const a2 = along[Math.max(0, k - 6)], b2 = along[Math.min(along.length - 1, k + 6)];
        const t = (y - a[1]) / ((b[1] - a[1]) || 1), dx = b2[0] - a2[0], dy = b2[1] - a2[1], h = Math.hypot(dx, dy) || 1;
        return [a[0] + (b[0] - a[0]) * t, y, dx / h, dy / h];
      };
      const highDist = (x, y) => Math.abs(x - atY(y)[0]);
      const names = [
        place(F, 'HIGH STREET', atY(CY - 640)[0] + 300, CY - 640, 7, T.ink),
      ];
      // parks counted in green
      const parks = FC(green.features.filter(f => ['park', 'garden', 'playground', 'pitch', 'forest', 'wood'].includes(f.properties.kind)));
      Geo.fill(G, proj, parks, T.sage, 8);
      const pond = FC(water.features.filter(f => f.geometry.type.includes('Polygon')));
      const pondCov = Geo.coverage(G, ctx => { ctx.beginPath(); d3.geoPath(proj, ctx)(pond); ctx.fill(); });
      for (let k = 0; k < pondCov.length; k++) if (pondCov[k] >= 4) G.data[k] = 0;
      const free = reserve(G, F, names, 1);
      // where the map's sources are written, the cloth is left bare
      const credit = [CX + 470 - 360, CY - 1105 - 26, 372, 36];
      const bare = (x, y) => free(x, y) || (x > credit[0] && x < credit[0] + credit[2] && y > credit[1] && y < credit[1] + credit[3]);
      // the houses along High Street, stood up in counted stitch, each its own brick and roof
      const roadMask = Geo.mask(2, ctx => {
        ctx.lineWidth = 9; ctx.lineCap = 'round';
        ctx.beginPath(); d3.geoPath(proj, ctx)(FC(roads.features.filter(f => f.properties.name !== 'North High Street' && ROADS[f.properties.class]))); ctx.stroke();
      });
      const onRoad = (x0, y0, w, h) => {
        for (let y = y0; y <= y0 + h; y += 4) for (let x = x0; x <= x0 + w; x += 4) if (roadMask.m[Math.floor(y / 2) * roadMask.w + Math.floor(x / 2)]) return true;
        return false;
      };
      const walls = [T.madderLt, T.terra, T.rose, T.madder, T.oat, T.terra, T.straw, T.blush], roofs = [T.madderDk, T.walnut, T.indigoMd, T.ink, T.brown];
      const rh = Geo.rng(61), HW = 5, HH = 6, gap = 18;
      let houses = 0;
      for (const side of [-1, 1]) {
        for (let y = -20; y < H + 20; y += HH * P + 9) {
          const [x] = atY(y + HH * P / 2), x0 = side < 0 ? x - gap - HW * P : x + gap;
          if (onRoad(x0, y, HW * P, HH * P) || bare(x0 + HW * P / 2, y + HH * P / 2)) continue;
          const [i0, j0] = Geo.cellAt(HG, x0, y);
          const chart = HOUSES[Math.floor(rh() * HOUSES.length)];
          Geo.stamp(HG, chart, { R: roofs[Math.floor(rh() * roofs.length)], w: walls[Math.floor(rh() * walls.length)], b: T.straw, d: T.walnut, c: T.brown }, i0, j0, { mirror: rh() < 0.5 });
          houses++;
        }
      }
      // every other building a little block of satin stitch, its stitches across its width, in the colours of brick
      const TONES = {
        house: [T.blush, T.rose, T.oat, T.blush, T.straw, T.rose], flats: [T.rose, T.terra, T.oatDk, T.blush], shop: [T.oat, T.oatDk, T.straw, T.rose],
        big: [T.oat, T.blueGrey, T.oatDk, T.straw, T.cream],
      };
      const kindOf = (k, area) => area > 9000 ? 'big' : /garage|shed|roof|carport/.test(k) ? null : /apartments|dormitory/.test(k) ? 'flats'
        : /commercial|retail|office|yes|industrial|church|school|university|hotel/.test(k) ? 'shop' : 'house';
      const bs = new Stitch.Segs(), built = [];
      bld.features.forEach((f, n) => {
        const rings = Geo.lines(proj, f);
        if (!rings.length || !Geo.onPlate(rings[0], 40)) return;
        const outer = rings[0];
        let area = 0, sx = 0, sy = 0;
        for (let k = 1; k < outer.length; k++) area += (outer[k - 1][0] * outer[k][1] - outer[k][0] * outer[k - 1][1]) / 2;
        area = Math.abs(area);
        const kind = kindOf(f.properties.kind || '', area);
        if (area < 70 || !kind) return;
        const m = outer.length;
        for (const p of outer) { sx += p[0]; sy += p[1]; }
        const mx = sx / m, my = sy / m;
        if (highDist(mx, my) < gap + HW * P + 6 && area < 9000) return;
        let xx = 0, yy = 0, xy = 0;
        for (const [x, y] of outer) { xx += (x - mx) ** 2; yy += (y - my) ** 2; xy += (x - mx) * (y - my); }
        const major = 0.5 * Math.atan2(2 * xy, xx - yy);
        const tones = TONES[kind];
        // neighbouring houses share their brick, as a street's do, the colour drifting from block to block
        const drift = Stitch.vnoise(mx / 230, my / 230, 41) * 0.75 + Stitch.hash(n, 3, 17) * 0.25;
        Geo.satin(bs, rings, { angle: major + Math.PI / 2, spacing: 2.5, hw: 1.45, col: tones[Math.min(tones.length - 1, Math.floor(drift * tones.length))], maxLen: 30, inset: 0.4, min: 2, jitter: 0.4, seed: n, taper: 0.3, shadow: 0.55 });
        built.push(rings);
      });
      append(S, filterSegs(bs, bare));
      // the pond in Goodale Park, satin in indigo
      Geo.satin(S, pond.features.flatMap(f => Geo.lines(proj, f)), { angle: 0.15, spacing: 2.6, hw: 1.45, col: (a, b, r) => r() < 0.5 ? T.indigoMd : T.indigoLt, maxLen: 26, seed: 5 });
      // the railway: a line of walnut with its sleepers
      for (const l of Geo.lines(proj, rail)) {
        if (!Geo.onPlate(l, 20)) continue;
        Geo.backstitch(S, l, { len: 9, hw: 1.15, col: T.walnut, taper: 0.5, tier: 1 });
        const at = Geo.mkAt(l), L = Geo.length(l);
        for (let d = 5; d < L; d += 10) {
          const p = at(d), q = at(Math.min(L, d + 1)), dx = q[0] - p[0], dy = q[1] - p[1], h = Math.hypot(dx, dy) || 1;
          S.push(p[0] - dy / h * 5, p[1] + dx / h * 5, p[0] + dy / h * 5, p[1] - dx / h * 5, 0.85, T.walnut, 0, { tier: 1, taper: 0.5 });
        }
      }
      // the streets couched along their middles; High Street in two threads of ochre, the freeway in madder, alleys a running stitch
      const rd = new Stitch.Segs();
      for (const f of roads.features) {
        const c = f.properties.class, w = ROADS[c];
        if (f.properties.name === 'North High Street') continue;
        for (const l of Geo.lines(proj, f)) {
          if (!Geo.onPlate(l, 30)) continue;
          if (c === 'service') { Geo.running(rd, l, { len: 6, gap: 4, hw: 0.95, col: T.oatDk, seed: 3, tier: 1 }); continue; }
          if (!w) continue;
          const colr = /motorway|trunk/.test(c) ? T.madder : c === 'residential' || c === 'unclassified' || c === 'living_street' ? T.brown : T.walnut;
          Geo.couch(rd, Geo.simplify(l, 0.6), { hw: w, col: colr, tack: /motorway/.test(c) ? T.madderDk : T.oat, every: 22, tier: 1 });
        }
      }
      append(S, filterSegs(rd, bare));
      const hs = d3.range(-30, H + 31, 6).map(y => atY(y));
      for (const off of [-4.2, 4.2]) Geo.couch(S, hs.map(([x, y, dx, dy]) => [x - dy * off, y + dx * off]), { hw: 2.5, col: T.ochre, tack: T.madderDk, every: 17, tackPhase: off > 0 ? 0.5 : 0, tier: 1 });
      // the street trees, each a crown of French knots: those the map knows, and rows along the quieter streets
      const rt = Geo.rng(23), greens = [T.sage, T.sageDk, T.moss, T.sage, T.olive];
      const blocked = Geo.mask(2, ctx => {
        ctx.beginPath(); for (const rings of built) for (const r of rings) { ctx.moveTo(r[0][0], r[0][1]); for (const p of r) ctx.lineTo(p[0], p[1]); } ctx.fill();
        ctx.lineWidth = 10; ctx.lineCap = 'round'; ctx.beginPath(); d3.geoPath(proj, ctx)(FC(roads.features.filter(f => ROADS[f.properties.class] || f.properties.class === 'service'))); ctx.stroke();
      });
      const room = (x, y) => x > 0 && y > 0 && x < W && y < H && !blocked.m[Math.floor(y / 2) * blocked.w + Math.floor(x / 2)];
      const rows = [];
      for (const f of roads.features) {
        if (!['residential', 'tertiary', 'secondary', 'unclassified', 'living_street'].includes(f.properties.class) || f.properties.name === 'North High Street') continue;
        for (const l of Geo.lines(proj, f)) {
          if (!Geo.onPlate(l, 10)) continue;
          const at = Geo.mkAt(l), L = Geo.length(l);
          for (let d = 10 + rt() * 10; d < L - 8; d += 24 + rt() * 10) {
            const p = at(d), q = at(Math.min(L, d + 1)), dx = q[0] - p[0], dy = q[1] - p[1], h = Math.hypot(dx, dy) || 1;
            for (const side of [-1, 1]) {
              if (rt() < 0.18) continue;
              const x = p[0] - dy / h * 12 * side, y = p[1] + dx / h * 12 * side;
              if (room(x, y) && !bare(x, y) && highDist(x, y) > gap + HW * P) rows.push([x, y]);
            }
          }
        }
      }
      for (const [x, y] of rows) {
        const n = 2 + Math.floor(rt() * 2);
        for (let k = 0; k < n; k++) {
          const a = rt() * 2 * Math.PI, r = k ? 2.6 + rt() * 2 : 0;
          Geo.knot(S, x + r * Math.cos(a), y + r * Math.sin(a), 3.2 + rt() * 1.2, greens[Math.floor(rt() * greens.length)], { tier: 3 });
        }
      }
      for (const t of trees.features) {
        const [x, y] = proj(t.geometry.coordinates);
        if (x < -20 || y < -20 || x > W + 20 || y > H + 20 || bare(x, y)) continue;
        if (highDist(x, y) < gap + HW * P) continue;
        const n = 3 + Math.floor(rt() * 2);
        for (let k = 0; k < n; k++) {
          const a = rt() * 2 * Math.PI, r = k ? 3.2 + rt() * 2.6 : 0;
          Geo.knot(S, x + r * Math.cos(a), y + r * Math.sin(a), 3.6 + rt() * 1.4, greens[Math.floor(rt() * greens.length)], { tier: 3 });
        }
      }
      // the arches over High Street, in gold, lamps along them and a knot at each foot
      const y0 = proj([-83.0, 39.9888])[1], y1 = proj([-83.0, 39.9768])[1];
      for (let k = 0; k <= 14; k++) {
        const [x, y, dx, dy] = atY(y0 + (y1 - y0) * k / 14), nx = -dy, ny = dx, half = gap - 2;
        const arc = d3.range(0, 1.0001, 0.05).map(t => {
          const s = (t - 0.5) * 2 * half, bow = (1 - (2 * t - 1) ** 2) * 5;
          return [x + nx * s - dx * bow, y + ny * s - dy * bow];
        });
        for (const T2 of [S, gold]) {
          Geo.couch(T2, arc, { hw: 2.2, col: T.gold, tack: T2 === S ? T.madderDk : 0, every: 9, tier: 4, shadow: T2 === S ? 1 : 0 });
          for (const t of [0, 0.5, 1]) { const p = arc[Math.round(t * (arc.length - 1))]; Geo.knot(T2, p[0], p[1], t === 0.5 ? 2.8 : 3.6, T.gold, { tier: 5, shadow: T2 === S ? 1 : 0 }); }
        }
      }
      const sp = this.sprites().butterfly, BG = fine();
      Geo.stamp(BG, BUTTERFLY, { r: T.madder, p: T.rose, k: T.ink, o: T.ochre, b: T.indigoMd, B: T.indigoLt, y: T.straw }, sp.i0, sp.j0);
      return {
        ground: { linen: cloth(15), pal, grids: [crossGrid(G, 25), crossGrid(HG, 26, { hw: 0.22 })], segs: S },
        labels: { linen: null, pal, grids: [crossGrid(F, 35, { hw: 0.22 })], segs: new Stitch.Segs(),
          after: (ctx, view) => { // the map's sources, written in ink on the linen
            ctx.save(); ctx.setTransform(view.s, 0, 0, view.s, view.tx, view.ty);
            ctx.font = 'italic 25px "IM Fell"'; ctx.fillStyle = 'rgba(70, 50, 38, 0.8)'; ctx.textAlign = 'right';
            ctx.fillText('© OpenStreetMap contributors', credit[0] + credit[2] - 6, credit[1] + 26);
            ctx.restore();
          } },
        sheen: { linen: null, pal, grids: [], segs: gold },
        butterfly: { linen: null, pal, grids: [crossGrid(BG, 45, { hw: 0.22 })], segs: new Stitch.Segs() },
      };
    },
  };
  const BUTTERFLY = `
.bb.....bb.
bBBb...bBBb
bBrBb.bBrBb
bBBBbkbBBBb
.bBBbkbBBb.
..bbbkbbb..
.bpybkbypb.
.bppb.bppb.
..bb...bb..`;
  const list = [world, america, ohioLevel, columbus, shortnorth];
  async function build(level, layer) {
    const L = list[level - 1], plan = await L.plan();
    if (!plan[layer]) throw new Error(`no layer ${layer} at ${L.id}`);
    return plan[layer];
  }
  return { list, build, T, pal };
})();
window.Levels = Levels;
