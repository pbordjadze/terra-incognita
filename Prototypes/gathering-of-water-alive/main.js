/* A gathering of water, alive (runtime 4): the rivers of South America on the globe, as in A gathering
   of water, with the water running. Touch a river and light runs from the touch down to the sea while
   the tributaries that gather there light up above it; rest a finger and draw across many, and every
   river crossed lights at once, the lights running together where the rivers meet and growing heavier
   as they gather. A drag turns the globe, a resting finger draws (the `hold` scheme), so a pan is never
   taken for a touch. Close in, a boto, a pink river dolphin, may surface in a branch just lit. At the
   coast the lit water runs on into the sea the way the month's currents carry it (the Amazon's plume,
   carried northwest along the Guianas). The words at the foot are a name; the numbers are in the panel.
   The study's rules are kept: a touch snaps to the nearest line drawn, distances are summed channel
   lengths, the light's speed is invented, and only what the dataset links is linked.
   This script decides; the globe draws. */
(function () {
  'use strict';
  var D = Math.PI / 180;
  var IDLE = { title: 'A gathering of water', text: 'Touch a river. Rest a finger, then draw across many.' };

  var data = terra.readJSON('network.json'), places = terra.readJSON('places.json'), about = terra.readJSON('about.json');
  var sea = terra.readJSON('sea.json') || { outlets: {} }, BOTO = terra.readJSON('boto.json');
  if (!data || !places) {
    terra.caption.set({ title: IDLE.title, text: 'The river network could not be read from this pack.' });
    return;
  }
  var net = WaterNet.build(data), edges = net.edges, LEVELS = data.levels;

  // The globe's water ink, as A gathering of water draws it: the network in its river ink, lighter for
  // the smaller rivers; what gathers in its coast ink; the way down deeper still. Light runs along what
  // is lit, the way the water goes (the globe moves it: `flow`).
  var INK = { trace: '#0B3D66', gathered: '#1C7AB0', sea: '#2F7FA8', water: '#2A6A99' };
  // (dimmed while water is lit, but kept readable on the satellite ground: there are a ton of rivers)
  var BANDS = [
    { id: 'rivers-3', from: 1, to: 3, ppd: 34, width: 0.7, color: '#4E9CCBB3', quiet: '#7FA0B48C' },
    { id: 'rivers-4', from: 4, to: 4, ppd: 22, width: 0.75, color: '#4E9CCBBF', quiet: '#7FA0B499' },
    { id: 'rivers-5', from: 5, to: 5, ppd: 14, width: 0.85, color: '#4E9CCBCC', quiet: '#7FA0B4A6' },
    { id: 'rivers-6', from: 6, to: 6, ppd: 9, width: 1.0, color: '#4E9CCBD9', quiet: '#7FA0B4B3' },
    { id: 'rivers-7', from: 7, to: 7, ppd: 6.3, width: 1.3, color: '#4E9CCBE6', quiet: '#7FA0B4BF' },
    { id: 'rivers-8', from: 8, to: 8, ppd: 4.5, width: 1.7, color: '#4E9CCBF2', quiet: '#7FA0B4CC' },
    { id: 'rivers-9', from: 9, to: 99, ppd: 0, width: 2.2, color: '#4E9CCB', quiet: '#7FA0B4D9' }
  ];
  // The way down, heavier where more lit water runs together (one, two or three, four or more).
  var TIERS = [{ id: 'trace-1', width: 2.3 }, { id: 'trace-2', width: 3.1 }, { id: 'trace-3', width: 4.0 }];
  var WIDTH = { gatheredGreat: 2.4, gathered: 1.6, sea: 1.2 };
  var FLOW = {
    trace: { speed: 30, dash: 2, gap: 22, color: '#CDEBFFD9' },
    gathered: { speed: 24, dash: 1.5, gap: 30, color: '#F4FBFFB3', width: 1.1 },
    sea: { speed: 22, dash: 2, gap: 18, color: '#F2FAFFD9' }
  };
  var GREAT = 8, DEEPEST = 42;
  BANDS.forEach(function (b) {
    b.edges = edges.filter(function (e) { return e.order >= b.from && e.order <= b.to; }).map(function (e) { return e.i; });
  });
  var NETWORK_BUDGET = 40000, GATHER_BUDGET = 20000;   // vertices (the stage holds 100,000 in all)
  var FLOW_BUDGET = 9000;                              // the most lit tributaries' vertices with light running on them
  var REACH = 28;                                      // a touch finds a line this many points away
  var MOST = 40;                                       // the most touches lit at once (the oldest lets go)
  var RUN = 70;                                        // how fast the light runs ahead, in points a second (invented)

  var cam = terra.camera || { lon: -60, lat: -18, zoom: 1.9, cx: 195, cy: 400, radius: 330, width: 390, height: 800 };
  var perZoom = cam.radius / Math.max(0.1, cam.zoom);  // this screen's points a radian at zoom 1
  function zoomAt(ppd) { return ppd / (perZoom * D); }
  function ppdNow() { return cam.radius * D; }         // points a degree now
  function globe() { terra.globe.set({ still: true, maxZoom: Math.max(8, Math.min(32, zoomAt(DEEPEST))) }); }

  var OPTIONS = {}, FEATURES = {}, PPD = {};
  function layer(id, options, ppd) {
    OPTIONS[id] = options;
    FEATURES[id] = [];
    if (ppd != null) { PPD[id] = ppd; options.minZoom = zoomAt(ppd); }
    terra.geo.set(id, [], options);
  }
  function put(id, features) { FEATURES[id] = features; terra.geo.set(id, features, OPTIONS[id]); }
  /** Sets a layer only when what it holds changed (the light growing sets several each tick). */
  var signs = {};
  function putIf(id, features, sign) {
    if (signs[id] === sign) return;
    signs[id] = sign;
    put(id, features);
  }
  function rescale(c) {
    var p = c.radius / Math.max(0.1, c.zoom);
    if (Math.abs(p - perZoom) < perZoom * 0.02) return false;
    perZoom = p;
    Object.keys(PPD).forEach(function (id) { OPTIONS[id].minZoom = zoomAt(PPD[id]); put(id, FEATURES[id]); });
    globe();
    return true;
  }
  BANDS.forEach(function (b) { layer(b.id, { style: 'border', color: b.color, lineWidth: b.width, touchable: true }, b.ppd); });
  layer('gathered', { style: 'border', color: INK.gathered, lineWidth: WIDTH.gathered, flow: FLOW.gathered });
  layer('gathered-great', { style: 'border', color: INK.gathered, lineWidth: WIDTH.gatheredGreat, flow: FLOW.gathered });
  layer('sea', { style: 'border', color: INK.sea + '8C', lineWidth: WIDTH.sea, flow: FLOW.sea });
  TIERS.forEach(function (t) { layer(t.id, { style: 'border', color: INK.trace, lineWidth: t.width, flow: FLOW.trace }); });
  layer('ending', { style: 'dot', color: INK.trace });
  layer('touched', { style: 'ring', color: INK.trace });
  layer('names-great', { style: 'label', font: 'italic', size: 13, color: INK.water, touchable: true }, 5);
  layer('names-rivers', { style: 'label', font: 'italic', size: 11.5, color: INK.water, touchable: true }, 8.5);
  // the drag is the globe's; a finger that rests, then moves, draws across the rivers
  terra.touch.scheme('hold');

  // ---- the named rivers (orientation, and a way to choose one by name) ----

  var SYSTEMS = {}, anchors = [];
  places.rivers.forEach(function (r) {
    var e = edges[r.edge];
    if (!e) return;
    var a = { name: r.name, label: r.label, system: r.system, major: !!r.major, labelAt: r.labelAt || r.at,
              hit: net.onEdge(e, r.at[0], r.at[1]) };
    if (r.system) SYSTEMS[e.outlet] = r.system;
    anchors.push(a);
  });
  var NAMED = { 'names-great': anchors.filter(function (a) { return a.major; }),
                'names-rivers': anchors.filter(function (a) { return !a.major; }) };
  function label(lon, lat, text) { return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat] }, properties: { label: text } }; }
  Object.keys(NAMED).forEach(function (id) {
    put(id, NAMED[id].map(function (a) { return label(a.labelAt[0], a.labelAt[1], a.label); }));
  });

  // ---- the camera: which rivers, how finely, over how much of the globe (as A gathering of water) ----

  var drawn = { lod: 0, cover: undefined, shown: '', bands: {} };
  function lodFor(c) {
    var tolerance = 1 / Math.max(1e-6, c.radius * D);
    for (var i = 0; i < LEVELS.length; i++) if (LEVELS[i] <= tolerance) return i + 1;
    return LEVELS.length;
  }
  function viewBox(c) {
    var w = Infinity, s = Infinity, e = -Infinity, n = -Infinity;
    for (var i = 0; i <= 6; i++) {
      for (var j = 0; j <= 6; j++) {
        var p = terra.unproject(c.width * i / 6, c.height * j / 6, c);
        if (!p) return null;
        if (p.lon < w) w = p.lon; if (p.lon > e) e = p.lon; if (p.lat < s) s = p.lat; if (p.lat > n) n = p.lat;
      }
    }
    return e - w > 180 ? null : [w, s, e, n];
  }
  function grow(b, f) { var x = (b[2] - b[0]) * f, y = (b[3] - b[1]) * f; return [b[0] - x, b[1] - y, b[2] + x, b[3] + y]; }
  function meets(r, box) { return !box || (r[0] <= box[2] && r[2] >= box[0] && r[1] <= box[3] && r[3] >= box[1]); }
  function covered(box) {
    if (drawn.cover === undefined) return false;
    if (drawn.cover === null) return true;
    return !!box && box[0] >= drawn.cover[0] && box[1] >= drawn.cover[1] && box[2] <= drawn.cover[2] && box[3] <= drawn.cover[3];
  }
  function shownOrder(zoom) {
    var m = 99;
    BANDS.forEach(function (b) { if (zoom >= OPTIONS[b.id].minZoom) m = Math.min(m, b.from); });
    return m;
  }
  function lines(ids, lod, box, minOrder, maxOrder) {
    var geoms = [], count = 0;
    for (var k = 0; k < ids.length; k++) {
      var e = edges[ids[k]];
      if (e.order < (minOrder || 1) || e.order > (maxOrder || 99) || !meets(e.bounds, box)) continue;
      var l = net.line(e, lod);
      geoms.push({ type: 'LineString', coordinates: l });
      count += l.length;
    }
    return { geoms: geoms, count: count };
  }
  function network(c, force) {
    if (rescale(c)) { force = true; drawn.bands = {}; }
    var box = viewBox(c), lod = lodFor(c);
    var shown = BANDS.filter(function (b) { return c.zoom >= OPTIONS[b.id].minZoom; });
    var key = shown.map(function (b) { return b.id; }).join();
    if (!force && lod === drawn.lod && key === drawn.shown && covered(box)) return;
    var cover = box ? grow(box, 0.15) : null, sets;
    for (var level = lod; level >= 1; level--) {
      var total = 0;
      sets = shown.map(function (b) { var s = lines(b.edges, level, cover); total += s.count; return s; });
      if (total <= NETWORK_BUDGET) break;
    }
    level = Math.max(1, level);
    var sign = level + '|' + (cover ? cover.map(function (v) { return v.toFixed(2); }).join() : 'all');
    BANDS.forEach(function (b) {
      var i = shown.indexOf(b);
      if (i >= 0 && drawn.bands[b.id] !== sign) { put(b.id, sets[i].geoms); drawn.bands[b.id] = sign; }
      if (i < 0 && drawn.bands[b.id]) { put(b.id, []); drawn.bands[b.id] = ''; }
    });
    drawn.lod = level;
    drawn.cover = cover;
    drawn.shown = key;
    lastBuild = Date.now();
    signs = {};
    water();
    gathered();
  }

  // ---- the water lit: every touch at once ----

  // Each touch: its way down (the stretches to the outlet, each one's length along its line), how far
  // the light has run along it, and whether what gathers above it is lit.
  var sources = [];
  var gatherIds = [], gatherSeen = {};
  var quiet = false;
  function quieten(on) {
    if (on === quiet) return;
    quiet = on;
    BANDS.forEach(function (b) { OPTIONS[b.id].color = on ? b.quiet : b.color; put(b.id, FEATURES[b.id]); });
  }

  function source(hit, gather) {
    var route = net.downstream(hit), legs = [], start = 0;
    for (var k = 0; k < route.ids.length; k++) {
      var id = route.ids[k], len = k === 0 ? Math.max(1e-6, net.length(id) - net.along(hit)) : net.length(id);
      legs.push({ id: id, start: start, len: len });
      start += len;
    }
    var out = edges[route.end], mouth = !out.inland && sea.outlets[route.end] || null;
    return { hit: hit, route: route, legs: legs, length: start, run: 0, gather: !!gather, mouth: mouth, at: Date.now(), boto: false };
  }

  /** The first part of a line, a fraction of its length along it. */
  function cut(line, f) {
    if (f >= 1) return line;
    var total = 0, k, steps = [];
    for (k = 1; k < line.length; k++) { var d = WaterNet.span(line[k - 1][0], line[k - 1][1], line[k][0], line[k][1]); steps.push(d); total += d; }
    var want = Math.max(0, f) * total, out = [line[0]];
    for (k = 1; k < line.length; k++) {
      if (want <= steps[k - 1]) {
        var t = steps[k - 1] ? want / steps[k - 1] : 0, a = line[k - 1], b = line[k];
        out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        return out;
      }
      want -= steps[k - 1];
      out.push(line[k]);
    }
    return out;
  }
  function geom(coordinates) { return { type: 'LineString', coordinates: coordinates }; }
  function point(lon, lat) { return { type: 'Point', coordinates: [lon, lat] }; }

  /** The ways down as far as the light has run, each shared stretch once, heavier as more runs in it;
      the endings reached inland; and, past the coast, the sea's part. */
  function water() {
    var lod = drawn.lod || lodFor(cam), cover = drawn.cover || null;
    quieten(sources.length > 0);
    var tiers = [[], [], []], count = {}, reach = {}, endings = [], seaLines = [], base = lod + '|' + (cover ? cover.join() : '');
    var tierSigns = [base, base, base], endSign = base;
    var tierOf = function (c) { return c >= 4 ? 2 : c >= 2 ? 1 : 0; };
    // how many lit ways run down each stretch, and how far along it the light is
    sources.forEach(function (s) {
      var first = s.legs[0];
      tierSigns[0] += '|h' + s.hit.id + ':' + Math.min(1, s.run / first.len).toFixed(3);
      for (var k = 1; k < s.legs.length; k++) {
        var leg = s.legs[k];
        if (s.run <= leg.start) break;
        count[leg.id] = (count[leg.id] || 0) + 1;
        reach[leg.id] = Math.max(reach[leg.id] || 0, Math.min(1, (s.run - leg.start) / leg.len));
      }
      if (s.run >= s.length) {
        var end = edges[s.route.end];
        if (end.inland) { var v = end.c[end.c.length - 1]; endings.push(point(v[0], v[1])); endSign += '|' + end.i; }
      }
    });
    Object.keys(count).forEach(function (id) { tierSigns[tierOf(count[id])] += '|' + id + ':' + reach[id].toFixed(3); });
    // each way down as long lines, so the light runs on unbroken: a stretch drawn once, a line ending
    // where its weight changes or where it meets a way already drawn
    var emitted = {};
    sources.forEach(function (s) {
      var first = s.legs[0], f = Math.min(1, s.run / first.len), cur = cut(net.line(edges[first.id], lod, s.hit, null), f).slice(), tier = 0;
      function flush() { if (cur && cur.length > 1) tiers[tier].push(geom(cur)); cur = null; }
      if (f < 1) return flush();
      for (var k = 1; k < s.legs.length; k++) {
        var leg = s.legs[k];
        if (s.run <= leg.start) break;
        var whole = net.line(edges[leg.id], lod);
        if (emitted[leg.id]) { cur.push(whole[0]); break; }
        emitted[leg.id] = true;
        var line = cut(whole, reach[leg.id]), t = tierOf(count[leg.id]);
        if (t !== tier) { cur.push(line[0]); flush(); cur = [line[0]]; tier = t; }
        for (var q = 1; q < line.length; q++) cur.push(line[q]);
        if (reach[leg.id] < 1) break;
      }
      flush();
    });
    TIERS.forEach(function (t, i) { putIf(t.id, tiers[i], tierSigns[i]); });
    putIf('ending', endings, endSign);
    // the sea's part: each mouth reached, its water carried on the month's currents
    var mouths = {};
    sources.forEach(function (s) { if (s.mouth && s.run > s.length) mouths[s.route.end] = Math.max(mouths[s.route.end] || 0, s.run - s.length); });
    var seaSign = '';
    Object.keys(mouths).forEach(function (id) {
      var m = sea.outlets[id], paths = m.months[month() - 1] || [];
      var f = Math.min(1, mouths[id] / m.length);
      seaSign += id + ':' + f.toFixed(3) + '|';
      paths.forEach(function (p) { seaLines.push(geom(cut(p, f))); });
    });
    putIf('sea', seaLines, seaSign);
    plume(mouths);
  }

  /** What gathers above the touches whose tributaries are lit, at the network's level and within its
      cover, as chains (each from a headwater down to where it joins one already drawn), so the light
      runs on along each branch into the next. */
  var chains = { small: [], great: [] };
  function rechain() {
    ['small', 'great'].forEach(function (kind) {
      var member = {}, claimed = {}, out = [];
      gatherIds.forEach(function (i) { if ((edges[i].order >= GREAT) === (kind === 'great')) member[i] = true; });
      var tips = gatherIds.filter(function (i) { return member[i] && !edges[i].up.some(function (u) { return member[u]; }); });
      tips.forEach(function (i) {
        var ids = [], j = i, b = edges[i].bounds.slice();
        while (j >= 0 && member[j] && !claimed[j]) {
          claimed[j] = true;
          ids.push(j);
          var r = edges[j].bounds;
          b = [Math.min(b[0], r[0]), Math.min(b[1], r[1]), Math.max(b[2], r[2]), Math.max(b[3], r[3])];
          j = edges[j].next;
        }
        if (ids.length) out.push({ ids: ids, join: j >= 0 && member[j] ? j : -1, bounds: b });
      });
      chains[kind] = out;
    });
  }
  function chainLines(list, lod, box, minOrder) {
    var geoms = [], count = 0;
    list.forEach(function (c) {
      if (!meets(c.bounds, box)) return;
      var out = null;
      for (var k = 0; k < c.ids.length; k++) {
        var e = edges[c.ids[k]];
        if (e.order < minOrder) { if (out && out.length > 1) { geoms.push(geom(out)); count += out.length; } out = null; continue; }
        var l = net.line(e, lod);
        if (!out) out = l.slice();
        else for (var q = 1; q < l.length; q++) out.push(l[q]);
      }
      if (out && c.join >= 0 && edges[c.join].order >= minOrder) out.push(net.line(edges[c.join], lod)[0]);
      if (out && out.length > 1) { geoms.push(geom(out)); count += out.length; }
    });
    return { geoms: geoms, count: count };
  }
  function gathered() {
    var lod = drawn.lod || lodFor(cam), cover = drawn.cover || null, min = shownOrder(cam.zoom), at = lod;
    var sign = lod + '|' + min + '|' + gatherIds.length + '|' + (cover ? cover.join() : '') + '|' +
      sources.filter(function (s) { return s.gather; }).map(function (s) { return s.hit.id; }).join();
    if (signs.gathered === sign) return;
    var small = chainLines(chains.small, at, cover, min), great = chainLines(chains.great, at, cover, Math.max(min, GREAT));
    if (small.count + great.count > GATHER_BUDGET) { at = 1; small = chainLines(chains.small, at, cover, min); great = chainLines(chains.great, at, cover, Math.max(min, GREAT)); }
    // each touch's own stretch above where it touched
    sources.forEach(function (s) {
      if (!s.gather) return;
      (edges[s.hit.id].order >= GREAT ? great : small).geoms.push(geom(net.line(edges[s.hit.id], at, null, s.hit)));
    });
    // the light runs on every lit tributary while there aren't too many to draw it on
    OPTIONS.gathered.flow = small.count <= FLOW_BUDGET ? FLOW.gathered : null;
    OPTIONS['gathered-great'].flow = great.count <= FLOW_BUDGET ? FLOW.gathered : null;
    putIf('gathered', small.geoms, sign);
    putIf('gathered-great', great.geoms, sign);
  }

  /** Lights what gathers above these touches too. */
  function gatherAbove(list) {
    list.forEach(function (s) {
      if (s.gather) return;
      s.gather = true;
      net.upstream(s.hit).ids.forEach(function (i) { if (!gatherSeen[i]) { gatherSeen[i] = true; gatherIds.push(i); } });
    });
    rechain();
    gathered();
  }

  // ---- the light running: a tick while any of it is still on its way ----

  var ticker = 0, lastTick = 0;
  function running() {
    return sources.some(function (s) { return s.run < s.length + (s.mouth ? s.mouth.length : 0); });
  }
  function tick() {
    var now = Date.now(), dt = Math.min(0.2, (now - lastTick) / 1000);
    lastTick = now;
    var base = RUN / Math.max(1e-6, ppdNow());           // degrees a second, at this zoom
    sources.forEach(function (s) {
      var whole = s.length + (s.mouth ? s.mouth.length : 0);
      if (s.run >= whole) return;
      // (no way down takes longer than about eight seconds, however close the reader is)
      s.run = Math.min(whole, s.run + dt * Math.max(base, whole / 8));
    });
    water();
    boto();
    if (!running()) { clearInterval(ticker); ticker = 0; }
  }
  function runLight() {
    if (terra.settings.reduceMotion) {
      // the light arrives: it doesn't travel
      sources.forEach(function (s) { s.run = s.length + (s.mouth ? s.mouth.length : 0); });
      water();
      boto();
      return;
    }
    if (ticker || !running()) return;
    lastTick = Date.now();
    ticker = setInterval(tick, 70);
  }

  /** A touch lit: its way down, and (unless a finger is still drawing) what gathers above it. */
  function light(hit, gather) {
    // a touch on a stretch already touched starts again from there
    for (var i = 0; i < sources.length; i++) {
      if (sources[i].hit.id === hit.id) { sources.splice(i, 1); break; }
    }
    var s = source(hit, false);
    sources.push(s);
    if (sources.length > MOST) sources.shift();
    if (gather) gatherAbove([s]);
    water();
    runLight();
    say();
    return s;
  }

  function clear() {
    sources = [];
    gatherIds = [];
    gatherSeen = {};
    chains = { small: [], great: [] };
    signs = {};
    water();
    gathered();
    put('touched', []);
    botoGone();
    say();
  }

  // ---- the sea: the lit water carried on the month's currents ----

  /** The globe's month (the ground's, the clouds' and the sea's): this month. */
  function month() { return new Date().getMonth() + 1; }
  var plumeShown = {};
  function plume(mouths) {
    Object.keys(sea.outlets).forEach(function (id) {
      var m = sea.outlets[id];
      if (!m.picture) return;
      var f = mouths[id] == null ? 0 : Math.min(1, mouths[id] / m.length), key = 'plume-' + id;
      var opacity = Math.round(Math.min(1, f * 1.6) * 10) / 10 * (m.picture.opacity || 0.8);
      if (plumeShown[key] === opacity) return;
      plumeShown[key] = opacity;
      if (opacity <= 0) { terra.images.remove(key); return; }
      terra.images.set(key, { image: m.picture.images[month() - 1], bounds: m.picture.bounds, opacity: opacity });
    });
  }

  // ---- life: a boto, a pink river dolphin, close in, in a branch just lit ----

  // Botos live in the Amazon's and the Orinoco's lowland rivers (and the Araguaia's, the Tocantins's):
  // a drawn one surfaces, now and then, in a large river just lit, when the reader is close. It is an
  // imaginative mark, not a sighting.
  var RANGE = { Amazon: true, Orinoco: true, Tocantins: true };
  var BOTO_PPD = 12;              // close in: about where the middle-sized rivers show
  var botos = [], botoSeen = false, botoTimer = 0, botoLooked = 0;
  function inRange(e) { return RANGE[SYSTEMS[e.outlet]] && e.order >= 6 && e.bounds[0] > -77.5; }
  /** The newest lit stretch in the boto's range near the middle of the screen, and where on it. */
  function botoPlace() {
    var best = null, cx = cam.cx, cy = cam.cy, limit = Math.min(cam.width, cam.height) * 0.42;
    for (var n = sources.length - 1; n >= 0 && n >= sources.length - 6; n--) {
      var s = sources[n];
      if (s.boto) continue;
      for (var k = 0; k < s.legs.length && s.legs[k].start < s.run; k++) {
        var e = edges[s.legs[k].id];
        if (!inRange(e) || botos.some(function (b) { return b.edge === e.i; })) continue;
        for (var j = 0; j < e.c.length; j += 2) {
          var p = terra.project(e.c[j][0], e.c[j][1], cam);
          if (!p || p.facing < 0.2) continue;
          var d = Math.hypot(p.x - cx, p.y - cy);
          if (d < limit && (!best || d < best.d)) best = { d: d, edge: e.i, j: j, source: s };
        }
      }
      if (best) break;
    }
    return best;
  }
  /** A short way along the river round a vertex (about a quarter of a degree each side) to swim. */
  function swim(e, j) {
    var c = e.c, a = j, b = j, d = 0;
    while (a > 0 && d < 0.22) { d += WaterNet.span(c[a - 1][0], c[a - 1][1], c[a][0], c[a][1]); a--; }
    d = 0;
    while (b < c.length - 1 && d < 0.22) { d += WaterNet.span(c[b][0], c[b][1], c[b + 1][0], c[b + 1][1]); b++; }
    return c.slice(a, b + 1);
  }
  /** Each new light may bring one boto up, close in, in its range, once the light has passed there. */
  function boto() {
    var now = Date.now();
    if (!BOTO || botoTimer || !sources.length || ppdNow() < BOTO_PPD || now - botoLooked < 400) return;
    botoLooked = now;
    var place = botoPlace();
    if (!place) return;
    place.source.boto = true;
    // it surfaces a moment after the light has passed
    botoTimer = setTimeout(function () {
      botoTimer = 0;
      if (!sources.length || ppdNow() < BOTO_PPD * 0.8) return;
      var e = edges[place.edge], path = swim(e, place.j), id = 'boto-' + (botos.length ? (botos[botos.length - 1].n + 1) % 2 : 0);
      botos = botos.filter(function (b) { return b.id !== id; });
      botos.push({ id: id, n: +id.slice(5), edge: e.i });
      if (botos.length > 2) botos.shift();
      terra.sprites.set(id, {
        path: { coordinates: path, seconds: 26 + 10 * Math.random(), loop: 'back' },
        looks: [{ frames: terra.settings.reduceMotion ? [BOTO.still] : BOTO.frames, fps: BOTO.fps, size: BOTO.size,
                  minZoom: zoomAt(BOTO_PPD * 0.8), face: true }],
        label: 'A boto, a pink river dolphin, drawn: not a sighting. Touch it.',
        touchable: true
      });
    }, terra.settings.reduceMotion ? 0 : 1800);
  }
  function botoGone() {
    clearTimeout(botoTimer);
    botoTimer = 0;
    botos.forEach(function (b) { terra.sprites.remove(b.id); });
    botos = [];
  }
  /** The reader touched a boto: it dives, and the foot names it. */
  function touchedBoto(id) {
    terra.sprites.remove(id);
    botos = botos.filter(function (b) { return b.id !== id; });
    botoSeen = true;
    said = '';
    terra.caption.set({ title: 'A boto, a pink river dolphin', text: '' });
  }

  // ---- the words at the foot: a name ----

  function list(words) {
    return words.length < 2 ? words.join('') : words.slice(0, -1).join(', ') + ' and ' + words[words.length - 1];
  }
  var said = '';
  function say() {
    var c = IDLE;
    if (sources.length) {
      var names = [], inland = false, other = false;
      sources.forEach(function (s) {
        var out = edges[s.route.end], name = SYSTEMS[out.i];
        if (name) { if (names.indexOf(name) < 0) names.push(name); }
        else if (out.inland) inland = true;
        else other = true;
      });
      var words = names.slice(0, 3).map(function (n) { return 'the ' + n; });
      if (names.length > 3 || (names.length && (inland || other))) words.push(names.length > 3 ? 'others' : 'other rivers');
      c = { title: words.length ? 'Toward ' + list(words) : inland ? 'Toward an inland ending' : 'Toward the sea', text: '' };
    }
    var k = c.title + '|' + c.text;
    if (k !== said) { said = k; terra.caption.set(c); }
  }

  // ---- the panel: the numbers and the sources, on demand ----

  function commas(n) { return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ','); }
  function distance(km) { return km < 10 ? 'less than 10 km' : 'about ' + commas(Math.round(km / 10) * 10) + ' km'; }
  function area(a) {
    if (a >= 1e6) return (Math.round(a / 1e5) / 10) + ' million km²';
    if (a >= 1000) { var p = Math.pow(10, Math.floor(Math.log(a) / Math.LN10) - 2); return commas(Math.round(a / p) * p) + ' km²'; }
    return commas(Math.max(10, Math.round(a / 10) * 10)) + ' km²';
  }
  function lit() {
    if (!sources.length) return null;
    var lines = sources.slice(-5).reverse().map(function (s) {
      var out = edges[s.route.end], system = SYSTEMS[out.i], e = edges[s.hit.id];
      return (system ? 'Toward the ' + system + ': ' : '') + 'from a touch, ' + distance(s.route.km) + ' along mapped channels to ' +
        (out.inland ? 'an inland ending' : 'the coast') + '; about ' + area(e.area) + ' of land drains into the stretch touched' +
        (out.i !== e.i ? ', and ' + area(out.area) + ' into the whole ' + (system ? system + ' system' : 'basin') + ', as mapped.' : '.');
    });
    if (sources.length > 5) lines.push('And ' + (sources.length - 5) + ' more lit.');
    return { title: 'What is lit', text: lines.join('\n\n') };
  }
  function panel() {
    if (!about) return;
    var content = JSON.parse(JSON.stringify(about)), now = lit();
    if (now) content.sections.unshift(now);
    // (the boto's note first once the reader has met one)
    if (botoSeen) {
      var i = content.sections.findIndex(function (s) { return /boto/i.test(s.title); });
      if (i > 0) content.sections.unshift(content.sections.splice(i, 1)[0]);
    }
    terra.panel.show(content);
  }

  // ---- touch: a tap lights a river; a resting finger draws across many ----

  function near(t) {
    if (t.lon == null || t.lat == null) return null;
    var reach = REACH / Math.max(1e-6, ppdNow()), min = shownOrder(cam.zoom);
    return net.nearest(t.lon, t.lat, reach, min);
  }

  /** A named river, chosen by its name: lit from its named place, all of it shown. */
  function choose(a) {
    var s = light(a.hit, true);
    var b = [Infinity, Infinity, -Infinity, -Infinity];
    s.route.ids.concat(net.upstream(a.hit).ids).forEach(function (i) {
      var r = edges[i].bounds;
      b = [Math.min(b[0], r[0]), Math.min(b[1], r[1]), Math.max(b[2], r[2]), Math.max(b[3], r[3])];
    });
    var lat = (b[1] + b[3]) / 2, size = Math.max((b[2] - b[0]) * Math.cos(lat * D), b[3] - b[1], 2) * D;
    var zoom = Math.max(1.4, Math.min(5, 0.6 * Math.min(cam.width, cam.height) / (size * perZoom)));
    terra.flyTo((b[0] + b[2]) / 2, lat, zoom, terra.settings.reduceMotion ? 0 : 1.4);
  }

  var drawing = null, lastHit = -1, lastSample = 0, lastRing = 0;
  terra.on('touch', function (t) {
    if (t.target && t.target.sprite && /^boto-/.test(t.target.sprite)) { if (t.phase === 'tap') touchedBoto(t.target.sprite); return; }
    if (t.phase === 'tap' && t.target && NAMED[t.target.layer]) {
      var a = NAMED[t.target.layer][t.target.index];
      if (a) { choose(a); return; }
    }
    if (t.phase === 'ended') {
      // the finger lifted: what gathers above everything it crossed lights up
      if (drawing) gatherAbove(drawing);
      drawing = null;
      lastHit = -1;
      put('touched', []);
      return;
    }
    var hit = near(t);
    if (t.phase === 'tap') {
      if (hit) { light(hit, true); put('touched', [point(hit.lon, hit.lat)]); }
      else if (t.lon != null) clear();
      return;
    }
    if (t.phase === 'began') { drawing = []; lastHit = -1; }
    if (!drawing) return;
    var now = Date.now();
    if (now - lastRing >= 30 && t.lon != null) { lastRing = now; put('touched', [point(hit ? hit.lon : t.lon, hit ? hit.lat : t.lat)]); }
    // a new way down only for a new stretch, at most every 55 ms (as the study sampled a brush)
    if (!hit || hit.id === lastHit || now - lastSample < 55) return;
    lastSample = now;
    lastHit = hit.id;
    drawing.push(light(hit, false));
  });

  // ---- the camera, settings, the panel ----

  var settle = 0, lastBuild = 0;
  terra.on('camera', function (c) {
    if (!c) return;
    cam = c;
    clearTimeout(settle);
    settle = setTimeout(function () { settle = 0; network(cam, false); boto(); }, 160);
    if (Date.now() - lastBuild > 400 && !covered(viewBox(c))) network(c, false);
  });
  terra.on('settings', function () { if (terra.settings.reduceMotion) runLight(); });
  terra.on('open', panel);
  terra.on('close', function () { clearTimeout(settle); clearInterval(ticker); clearTimeout(botoTimer); });

  globe();
  network(cam, true);
  say();
})();
