/* Fires across the sea (runtime 4). On every peak of the Aegean a pyre waits. Touch one and it burns;
   every peak that can see it over the curve of the Earth lights its own in turn, and so the fire runs
   across the sea from island to island, as Aeschylus has Clytemnestra's beacons bring the news of
   Troy's fall to Argos in a night. Which peaks can see which is worked out, not drawn: two fires see
   each other when they are within the sum of their horizons (3.86 km times the square root of each
   height in metres, the usual allowance for the bending of light). The fires burn a while and go out;
   with no one touching, Mount Ida above Troy lights itself and the relay runs. Nothing is written on
   the screen but the name; SOURCES.md and the panel the chip's Open shows say the rest. */
(function () {
  'use strict';
  var P = window.PEAKS, D = Math.PI / 180, R = 6371;
  // How large things are depends on the screen: a degree is radius × π/180 points at zoom 1 (about
  // 489 on the 13-inch iPad, 176 on a phone), so the fire's looks change at a density of points a
  // degree, not at one zoom for every screen (a phone zooms further for the same picture).
  var R1 = 489;
  function zoomFor(ppd) { return ppd / (R1 * D); }
  var NEAR = 7, MID = 3.5;                 // zooms (set from the screen): the fire itself; its small self; an ember
  var SEEN = 2.4, PER_KM = 1 / 110;        // seconds before a watchman lights his, and more the further the fire
  var BURN = 42, SMOKE = 9;                // seconds a fire burns, then smokes
  var LIVE = 48;                           // the newest fires flicker as sprites; older ones burn still, as symbols
  var IDA = 'ida-troad', ARGOS = 'argos';

  function dist(a, b) {
    var la = a.lat * D, lb = b.lat * D, dl = (b.lon - a.lon) * D;
    var x = Math.sin(la) * Math.sin(lb) + Math.cos(la) * Math.cos(lb) * Math.cos(dl);
    return R * Math.acos(Math.max(-1, Math.min(1, x)));
  }
  function horizon(h) { return 3.86 * Math.sqrt(Math.max(1, h)); }
  function point(p, props) { return { type: 'Feature', geometry: { type: 'Point', coordinates: [p.lon, p.lat] }, properties: props || {} }; }

  // ---- who sees whom ----
  var peaks = P.peaks, byId = {};
  peaks.forEach(function (p) { byId[p.id] = p; p.sees = []; });
  for (var i = 0; i < peaks.length; i++) {
    for (var j = i + 1; j < peaks.length; j++) {
      var a = peaks[i], b = peaks[j], d = dist(a, b);
      if (d <= horizon(a.h) + horizon(b.h)) { a.sees.push({ id: b.id, km: d }); b.sees.push({ id: a.id, km: d }); }
    }
  }

  // ---- the pyres, and the fires ----
  var lit = {}, order = [], timers = [], sight = [], quiet = 0;
  function later(fn, ms) { var t = setTimeout(fn, ms); timers.push(t); return t; }
  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }

  function drawPyres() {
    var cold = peaks.filter(function (p) { return !lit[p.id]; }).map(function (p) { return point(p, { label: p.name + ', ' + p.h + ' m' }); });
    terra.geo.set('pyres', cold, { style: 'symbol', image: 'pyre.png', width: 14, height: 11, minZoom: NEAR, touchable: true, offset: [0, -3] });
    terra.geo.set('pyres-mid', cold, { style: 'symbol', image: 'pyre-mid.png', width: 8, height: 6, minZoom: MID, maxZoom: NEAR, touchable: true, offset: [0, -2] });
  }
  function drawStill() {
    // the older fires, burning on as still pictures (the sprites are for the newest)
    var still = order.slice(0, Math.max(0, order.length - LIVE)).map(function (id) { return byId[id]; }).filter(function (p) { return lit[p.id] === 'burning'; });
    terra.geo.set('fires-still', still.map(function (p) { return point(p, { label: p.name + ', burning' }); }),
                  { style: 'symbol', image: 'fire-still.png', width: 30, height: 38, minZoom: NEAR, offset: [0, -14], touchable: true });
    terra.geo.set('fires-still-mid', still.map(function (p) { return point(p); }),
                  { style: 'symbol', image: 'fire-still-mid.png', width: 16, height: 20, minZoom: MID, maxZoom: NEAR, offset: [0, -7] });
    terra.geo.set('fires-still-far', still.map(function (p) { return point(p); }),
                  { style: 'symbol', image: 'ember-2.png', width: 10, height: 10, maxZoom: MID });
  }
  function drawSight() {
    terra.geo.set('sight', sight.length ? [{ type: 'MultiLineString', coordinates: sight }] : [],
                  { style: 'border', color: '#F2A33A88', lineWidth: 0.9, minZoom: 1.2 });
  }
  // (a sprite's place is its picture's middle: the fire's foot is 33 points below the middle of the
  // tall picture, so the picture is set with the foot on the peak)
  function fireSprite(p) {
    terra.sprites.set('fire-' + p.id, { at: [p.lon, p.lat], label: p.name + ', burning', touchable: true, looks: [
      { frames: frames('fire', 12), fps: 10, size: [30, 66], minZoom: NEAR },
      { frames: frames('fire-mid', 12), fps: 10, size: [16, 35], minZoom: MID, maxZoom: NEAR },
      { frames: frames('ember', 6), fps: 4, size: [12, 12], maxZoom: MID }
    ] });
  }
  function smokeOnly(p) {
    terra.sprites.set('fire-' + p.id, { at: [p.lon, p.lat], label: p.name + ', smoking', looks: [
      { frames: frames('smoke', 12), fps: 6, size: [30, 44], minZoom: NEAR },
      { frames: frames('smoke', 12), fps: 6, size: [15, 22], minZoom: MID, maxZoom: NEAR }
    ] });
  }
  function out(p) {
    terra.sprites.remove('fire-' + p.id);
    delete lit[p.id];
    order = order.filter(function (id) { return id !== p.id; });
    drawPyres(); drawStill();
    if (!order.length) { sight = []; drawSight(); quiet = later(function () { light(IDA, null); }, 9000); }
  }

  /** A fire lit on a peak `from` another (or by the reader): it burns, and every watchman who can see
      it lights his own after the time it takes to notice and the further it is. */
  function light(id, from) {
    var p = byId[id];
    if (!p || lit[id]) return;
    clearTimeout(quiet);
    lit[id] = 'burning';
    order.push(id);
    if (order.length > LIVE) { var old = byId[order[order.length - LIVE - 1]]; terra.sprites.remove('fire-' + old.id); }
    fireSprite(p);
    if (from) { sight.push([[from.lon, from.lat], [p.lon, p.lat]]); drawSight(); }
    drawPyres(); drawStill();
    p.sees.forEach(function (s) {
      if (lit[s.id]) return;
      later(function () { if (lit[id] === 'burning') light(s.id, p); }, (SEEN + s.km * PER_KM) * 1000);
    });
    later(function () { lit[id] = 'smoke'; smokeOnly(p); drawStill(); }, BURN * 1000);
    later(function () { out(p); }, (BURN + SMOKE) * 1000);
  }

  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || !t.target) return;
    if (t.target.layer === 'pyres' || t.target.layer === 'pyres-mid') {
      var cold = peaks.filter(function (p) { return !lit[p.id]; })[t.target.index];
      if (cold) light(cold.id, null);
    }
  });

  terra.on('open', function () {
    terra.panel.show({
      title: 'Fires across the sea',
      text: 'In Aeschylus\'s Agamemnon, the news that Troy has fallen reaches Argos the same night by a chain of fires: from Mount Ida above Troy to Lemnos, to Athos, across to Euboea, Messapion, Kithairon, the Megarid, Arachnaion and the palace. Each watchman, seeing a fire on the horizon, lights his own. Here every peak has its pyre, and which can see which is worked out from their heights and the curve of the Earth: two fires see each other within the sum of their horizons, 3.86 kilometres times the square root of each height in metres.',
      sections: [
        { title: 'What is drawn', text: 'The peaks are the highest points of the Aegean\'s islands and of the mountains round it, with their heights; a fire can be seen as far as its horizon allows, and no further, so the chain runs only where it could. The seconds a watchman takes to light his pyre are drawn. The fires go out after a while; left alone, Ida lights itself and the relay runs again.' },
        { title: 'Touching', text: 'Touch any pyre to light it. The fire runs from there to every peak that can see it, and on.' }
      ],
      sources: [
        { title: 'Aeschylus, Agamemnon, lines 281–316 (the beacon speech)', url: 'https://www.perseus.tufts.edu/hopper/text?doc=Perseus:text:1999.01.0004', note: 'The relay\'s stations, in Clytemnestra\'s words.' },
        { title: 'Wikipedia: the islands and mountains named (their coordinates and highest points)', url: 'https://en.wikipedia.org/', note: 'Heights as given there, to the metre; places at the summit where the article is a mountain\'s, else at the island\'s.' },
        { title: 'The distance to the horizon with refraction: about 3.86 km × √(height in m)', url: 'https://en.wikipedia.org/wiki/Horizon', note: 'The geometric horizon is 3.57 km × √h; standard refraction lengthens it by about 8%.' }
      ]
    });
  });

  terra.on('close', function () { timers.forEach(clearTimeout); clearTimeout(quiet); });

  /** The screen's size sets the zooms at which the fires are drawn large; a change redraws them. */
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR !== 7) return false;
    R1 = r1;
    NEAR = zoomFor(90); MID = zoomFor(40);
    return true;
  }
  terra.on('camera', function (c) {
    if (!measure(c)) return;
    drawPyres(); drawStill(); drawSight();
    order.slice(-LIVE).forEach(function (id) { var p = byId[id]; if (lit[id] === 'burning') fireSprite(p); else if (lit[id]) smokeOnly(p); });
  });

  // ---- start: every pyre cold; a moment later, Troy's ----
  terra.globe.set({ still: true, maxZoom: 28 });
  terra.caption.set({ title: 'Fires across the sea', text: '' });
  measure(terra.camera);
  drawPyres(); drawStill(); drawSight();
  // from the workbench or a link, the whole Aegean at a size the fires can be seen at (by a doorway, the
  // reader is already here)
  if (terra.entry.by !== 'doorway') terra.flyTo(25.2, 38.4, Math.min(28, Math.max(3, zoomFor(52))), 1.4);
  quiet = later(function () { light(IDA, null); }, 2200);
})();
