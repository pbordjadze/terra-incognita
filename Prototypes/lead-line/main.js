/* The lead line (runtime 4). HMS Challenger sails her whole way round the world, 1872 to 1876, as the
   date plays, never out of sight: between the stations of her record she keeps to her itinerary's
   ports at a steady pace. Wherever the sea is touched, a lead is let down and runs out until it finds
   the floor: a long wait where the water is deep, a short one on the shelf, and the lines stay, a forest
   of soundings of the reader's own. The sounding glass can be dragged anywhere over the sea: the floor
   under it is drawn in contours as it moves, and the lead under its middle reads the depth. Touch the
   ship to follow her; the globe's date and the record's stations are the strip's ticks. The depths
   everywhere are the globe's own (GEBCO, a third of a degree a cell), the stations' depths the
   record's. Nothing is written on the screen but the name and a depth at the foot of each line. */
(function () {
  'use strict';
  // (the packs' shared script: Prototypes/_shared/shared.js, carried as shared.js)
  var unit = Shared.unit, along = Shared.along, frames = Shared.frames;
  var WAY = window.WAY.points, G = window.DEPTHS, D = Math.PI / 180, DAY = 86400000;
  var R1 = 489;
  function zoomFor(ppd) { return ppd / (R1 * D); }
  var NEAR = 4, MID = 1.6;                      // zooms (set from the screen): the ship herself; small; a speck
  var RATE = 9;                                 // days a second: the voyage in about two and a half minutes
  var PT_PER_KM = 14, MAX_LINE = 180;           // the hanging line: points a kilometre of depth, and at most
  var SINK = 0.0012;                            // seconds a metre as the line runs out (a mile takes two seconds)
  var SHEET = ['#2D6A8F', '#2D6A8F', '#4A86A8', '#6FA3BF', '#9BC0D3'];  // the contours' inks, deep to shallow
  var CONTOURS = [200, 1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000];

  // ---- the depths ----
  var depths = (function () {
    var abc = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/', lut = new Uint8Array(128), i;
    for (i = 0; i < 64; i++) lut[abc.charCodeAt(i)] = i;
    var s = G.data.replace(/=+$/, ''), out = new Uint8Array(Math.floor(s.length * 3 / 4)), o = 0, buf = 0, bits = 0;
    for (i = 0; i < s.length; i++) {
      buf = (buf << 6) | lut[s.charCodeAt(i)]; bits += 6;
      if (bits >= 8) { bits -= 8; out[o++] = (buf >> bits) & 255; buf &= (1 << bits) - 1; }
    }
    return out;
  })();
  /** The depth at a place in metres (0: land or the shore). */
  function depthAt(lon, lat) {
    var x = Math.floor(((lon + 180) / 360) * G.width), y = Math.floor(((90 - lat) / 180) * G.height);
    x = ((x % G.width) + G.width) % G.width; y = Math.max(0, Math.min(G.height - 1, y));
    return depths[y * G.width + x] * G.metresPerStep;
  }
  function cell(x, y) { x = ((x % G.width) + G.width) % G.width; y = Math.max(0, Math.min(G.height - 1, y)); return depths[y * G.width + x] * G.metresPerStep; }

  // ---- geometry ----
  function point(lon, lat, props) { return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat] }, properties: props || {} }; }
  function r3(v) { return Math.round(v * 1000) / 1000; }

  // ---- the ship: where the date puts her ----
  var FIRST = WAY[0].d, LAST = WAY[WAY.length - 1].d;
  function at(d) {
    if (d <= FIRST) return { lon: WAY[0].lon, lat: WAY[0].lat, i: 0 };
    if (d >= LAST) { var l = WAY[WAY.length - 1]; return { lon: l.lon, lat: l.lat, i: WAY.length - 1 }; }
    var lo = 0, hi = WAY.length - 1;
    while (hi - lo > 1) { var m = (lo + hi) >> 1; if (WAY[m].d <= d) lo = m; else hi = m; }
    var a = WAY[lo], b = WAY[hi], t = b.d > a.d ? (d - a.d) / (b.d - a.d) : 0, p = along([a.lon, a.lat], [b.lon, b.lat], t);
    return { lon: p[0], lat: p[1], i: lo };
  }
  function shipLooks() {
    return [{ frames: frames('ship', 8), fps: 6, size: [64, 50], minZoom: NEAR, face: true },
            { frames: frames('ship-mid', 8), fps: 6, size: [32, 25], minZoom: MID, maxZoom: NEAR, face: true },
            { image: 'ship-far.png', size: [14, 11], maxZoom: MID, face: true }];
  }

  var date = WAY.filter(function (p) { return p.kind === 'station'; })[0].d, playing = true, shownLeg = -1, following = false;
  /** Sets the ship on the leg the date is in, so the host sails her at the date's pace; placed still
      when the date is still or scrubbed. */
  function sail(by) {
    var h = at(date);
    if (playing && by === 'play' && shownLeg === h.i) return;
    if (playing && h.i < WAY.length - 1) {
      var b = WAY[h.i + 1], seconds = Math.max(0.3, (b.d - date) / RATE);
      terra.sprites.set('ship', { path: { coordinates: [[r3(h.lon), r3(h.lat)], [b.lon, b.lat]], seconds: seconds, loop: false },
                                  looks: shipLooks(), label: 'HMS Challenger', touchable: true });
      shownLeg = h.i;
    } else {
      terra.sprites.set('ship', { at: [h.lon, h.lat], looks: shipLooks(), label: 'HMS Challenger', touchable: true });
      shownLeg = -1;
    }
    if (following && Date.now() - lastFly > 1400) {
      // a flight now and then keeps her in view as she sails, not a flight a frame
      var c = terra.camera;
      if (c) { lastFly = Date.now(); terra.flyTo(h.lon, h.lat, c.zoom, 1.3); }
    }
  }
  var lastFly = 0;

  // ---- the stations: the record's dots, reached or ahead ----
  var stations = WAY.filter(function (p) { return p.kind === 'station'; }), drawnPast = -1;
  function drawStations() {
    var past = 0;
    while (past < stations.length && stations[past].d <= date) past++;
    if (past === drawnPast) return;
    drawnPast = past;
    terra.geo.set('ahead', stations.slice(past).map(function (s) { return point(s.lon, s.lat, { label: 'Station ' + s.name + ', ' + s.fathoms + ' fathoms' }); }), { style: 'ring', color: '#4A5A66', minZoom: MID, touchable: true });
    terra.geo.set('reached', stations.slice(0, past).map(function (s) { return point(s.lon, s.lat, { label: 'Station ' + s.name + ', ' + s.fathoms + ' fathoms' }); }), { style: 'dot', color: '#4A5A66', minZoom: MID, touchable: true });
  }

  // ---- the lead: let down wherever the sea is touched; the lines stay ----
  var leads = [], raf = {}, nextLead = 1;
  function label(m) { return String(Math.round(m)).replace(/\B(?=(\d{3})+(?!\d))/g, ',') + ' m'; }
  function drawLeads() {
    terra.geo.set('leads', leads.map(function (l) { return point(l.lon, l.lat, { length: l.length, label: l.label }); }), { style: 'sounding' });
  }
  /** The lead runs out from the surface at a steady pace until it finds the floor at `metres`. */
  function lower(lon, lat, metres, said) {
    var l = { id: nextLead++, lon: lon, lat: lat, length: 0, label: '', metres: metres };
    leads.push(l);
    if (leads.length > 24) leads.shift();
    var full = Math.min(MAX_LINE, metres / 1000 * PT_PER_KM);
    var seconds = terra.settings.reduceMotion ? 0 : Math.min(14, metres * SINK), start = performance.now();
    function step(now) {
      var f = seconds ? Math.min(1, (now - start) / (seconds * 1000)) : 1;
      l.length = full * f;
      l.label = f < 1 ? '' : (said || label(metres));
      drawLeads();
      if (f < 1) raf[l.id] = requestAnimationFrame(step); else delete raf[l.id];
    }
    raf[l.id] = requestAnimationFrame(step);
  }

  // ---- the sounding glass: dragged over the sea, the floor under it in contours ----
  var glass = null, glassTimer = 0, dragging = false;
  function glassRadius() { var c = terra.camera; return c ? Math.max(1.2, 150 / (c.radius / c.zoom * D) / Math.max(1, c.zoom) * 1.0) : 3; }
  /** The ring of places r degrees (of arc) from a centre. */
  function ring(lon, lat, r, n) {
    var p1 = lat * D, l1 = lon * D, rr = r * D, out = [];
    for (var i = 0; i <= n; i++) {
      var th = (i % n) / n * 2 * Math.PI;
      var p2 = Math.asin(Math.sin(p1) * Math.cos(rr) + Math.cos(p1) * Math.sin(rr) * Math.cos(th));
      var l2 = l1 + Math.atan2(Math.sin(th) * Math.sin(rr) * Math.cos(p1), Math.cos(rr) - Math.sin(p1) * Math.sin(p2));
      out.push([r3(l2 / D), r3(p2 / D)]);
    }
    return out;
  }
  /** The depth contours inside the glass, by marching squares over the grid's cells there. */
  function contours(lon, lat, r) {
    var cw = 360 / G.width, ch = 180 / G.height;
    var x0 = Math.floor((lon - r / Math.cos(lat * D) + 180) / cw) - 1, x1 = Math.ceil((lon + r / Math.cos(lat * D) + 180) / cw) + 1;
    var y0 = Math.floor((90 - lat - r) / ch) - 1, y1 = Math.ceil((90 - lat + r) / ch) + 1;
    if (x1 - x0 > 80) { x0 = Math.floor((lon + 180) / cw) - 40; x1 = x0 + 80; }
    var lines = CONTOURS.map(function () { return []; });
    var c = unit([lon, lat]), cosR = Math.cos(r * D);
    function inside(p) { var u = unit(p); return u[0] * c[0] + u[1] * c[1] + u[2] * c[2] >= cosR; }
    function lonOf(x) { return x * cw - 180; }
    function latOf(y) { return 90 - y * ch; }
    for (var y = y0; y < y1; y++) {
      for (var x = x0; x < x1; x++) {
        var d00 = cell(x, y), d10 = cell(x + 1, y), d01 = cell(x, y + 1), d11 = cell(x + 1, y + 1);
        var mx = lonOf(x + 0.5), my = latOf(y + 0.5);
        if (!inside([mx, my])) continue;
        CONTOURS.forEach(function (level, k) {
          var b = (d00 >= level ? 1 : 0) | (d10 >= level ? 2 : 0) | (d11 >= level ? 4 : 0) | (d01 >= level ? 8 : 0);
          if (b === 0 || b === 15) return;
          function lerp(a, bb, da, db) { var t = db === da ? 0.5 : (level - da) / (db - da); return a + (bb - a) * t; }
          var top = [lerp(lonOf(x), lonOf(x + 1), d00, d10), latOf(y)], right = [lonOf(x + 1), lerp(latOf(y), latOf(y + 1), d10, d11)];
          var bottom = [lerp(lonOf(x), lonOf(x + 1), d01, d11), latOf(y + 1)], left = [lonOf(x), lerp(latOf(y), latOf(y + 1), d00, d01)];
          var segs = {
            1: [[left, top]], 2: [[top, right]], 3: [[left, right]], 4: [[right, bottom]], 5: [[left, top], [right, bottom]],
            6: [[top, bottom]], 7: [[left, bottom]], 8: [[bottom, left]], 9: [[top, bottom]], 10: [[top, right], [bottom, left]],
            11: [[right, bottom]], 12: [[right, left]], 13: [[top, right]], 14: [[left, top]]
          }[b];
          segs.forEach(function (s) { lines[k].push([[r3(s[0][0]), r3(s[0][1])], [r3(s[1][0]), r3(s[1][1])]]); });
        });
      }
    }
    return lines;
  }
  function drawGlass() {
    if (!glass) {
      ['glass-rim', 'glass-deep', 'glass-mid', 'glass-shallow', 'glass-shelf'].forEach(function (id) { terra.geo.set(id, [], { style: 'border' }); });
      terra.geo.set('glass-lead', [], { style: 'sounding' });
      return;
    }
    var r = glassRadius(), lines = contours(glass.lon, glass.lat, r);
    var shelf = lines[0], shallow = lines[1].concat(lines[2]), mid = lines[3].concat(lines[4]), deep = lines[5].concat(lines[6], lines[7], lines[8]);
    terra.geo.set('glass-deep', deep.length ? [{ type: 'MultiLineString', coordinates: deep }] : [], { style: 'border', color: SHEET[0], lineWidth: 0.9 });
    terra.geo.set('glass-mid', mid.length ? [{ type: 'MultiLineString', coordinates: mid }] : [], { style: 'border', color: SHEET[2], lineWidth: 0.9 });
    terra.geo.set('glass-shallow', shallow.length ? [{ type: 'MultiLineString', coordinates: shallow }] : [], { style: 'border', color: SHEET[3], lineWidth: 0.9 });
    terra.geo.set('glass-shelf', shelf.length ? [{ type: 'MultiLineString', coordinates: shelf }] : [], { style: 'border', color: SHEET[4], lineWidth: 1.2 });
    terra.geo.set('glass-rim', [{ type: 'Polygon', coordinates: [ring(glass.lon, glass.lat, r, 48)] }], { style: 'area', color: '#F4F1E8', lineWidth: 1.4, touchable: true });
    var m = depthAt(glass.lon, glass.lat);
    terra.geo.set('glass-lead', [point(glass.lon, glass.lat, { length: Math.min(MAX_LINE, m / 1000 * PT_PER_KM), label: m ? label(m) : 'land' })], { style: 'sounding' });
  }
  function moveGlass(lon, lat) {
    glass = { lon: lon, lat: lat };
    clearTimeout(glassTimer);
    glassTimer = setTimeout(drawGlass, 30);
  }

  // ---- touch ----
  var touching = null;
  terra.on('touch', function (t) {
    if (t.target && t.target.layer === 'glass-rim') {
      if (t.phase === 'began') { dragging = true; touching = t.target; }
      if ((t.phase === 'moved' || t.phase === 'ended') && t.lon != null) moveGlass(t.lon, t.lat);
      if (t.phase === 'ended') dragging = false;
      return;
    }
    if (t.phase !== 'tap') return;
    if (t.target && t.target.sprite === 'ship') {
      // follow her: the globe keeps her in view as she sails; any turn of the globe by hand lets go
      following = !following;
      var h = at(date), c = terra.camera;
      if (following && c) terra.flyTo(h.lon, h.lat, Math.max(c.zoom, NEAR + 0.2), 1.2);
      return;
    }
    if (t.target && (t.target.layer === 'ahead' || t.target.layer === 'reached')) {
      var list = t.target.layer === 'ahead' ? stations.slice(drawnPast) : stations.slice(0, drawnPast), s = list[t.target.index];
      if (s) lower(s.lon, s.lat, s.depth, s.fathoms.toLocaleString('en-GB') + ' fathoms');
      return;
    }
    if (t.lon == null) return;
    var m = depthAt(t.lon, t.lat);
    if (m > 0) lower(t.lon, t.lat, m);
    else if (!glass) { moveGlass(t.lon, t.lat); }
  });

  terra.on('time', function (e) {
    date = e.value; playing = e.playing;
    sail(e.by);
    drawStations();
  });
  var lastCam = null;
  terra.on('camera', function (c) {
    if (measure(c)) { shownLeg = -1; sail('camera'); drawStations(); }
    // the reader turned the globe: stop following
    if (following && lastCam && (Math.abs(c.lon - lastCam.lon) > 0.01 || Math.abs(c.lat - lastCam.lat) > 0.01) && !terra.__flying) {
      var h = at(date), p = terra.project(h.lon, h.lat);
      if (!p || !p.visible || Math.hypot(p.x - c.cx, p.y - c.cy) > c.radius * 0.5) following = false;
    }
    lastCam = c;
    if (glass) { clearTimeout(glassTimer); glassTimer = setTimeout(drawGlass, 60); }
  });
  terra.on('focus', function (f) { if (f.front) terra.time.play(true); });
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR !== 4) return false;
    R1 = r1; NEAR = zoomFor(34); MID = zoomFor(14);
    return true;
  }

  terra.on('open', function () {
    terra.panel.show({
      title: 'The lead line',
      wikipedia: 'Challenger expedition',
      text: 'HMS Challenger left Portsmouth in December 1872 and came home in May 1876, having sailed round the world to find out what the deep sea was: how deep, how cold, what lived there and what lay on its floor. At each of her stations the crew stopped, let a weighted line down until it stopped running, and brought up what the bottom was made of. Here she sails her whole way as the date plays, and the lead is yours: touch the sea anywhere and a line runs out until it finds the floor, slowly where the water is deep.',
      sections: [
        { title: 'What is drawn', wikipedia: 'Depth sounding', text: 'The dots are 94 of her stations with their recorded depths (touch one to lower the line to the depth they found). Between stations she keeps to the ports and turns of her published itinerary, dated to the month, at a steady pace: a drawn way, not her logged course. The depths everywhere else are today\'s (GEBCO, through the globe\'s own maps, about a third of a degree a cell), and a line runs out at about two seconds a mile of depth, far faster than a hemp line did.' },
        { title: 'The glass', wikipedia: 'Bathymetry', text: 'Touch the land to set down the sounding glass, then drag it over the sea: the floor under it is drawn in contours (200 m, then every thousand), and the line under its middle reads the depth there.' }
      ],
      sources: [
        { title: 'Murray, John; Renard, A. F. (1891). Annotated record of the detailed examination of Mn deposits from the HMS Challenger stations. PANGAEA.',
          url: 'https://doi.org/10.1594/PANGAEA.849072', note: 'CC BY 3.0. The 94 dated stations and their recorded depths in fathoms (1.8288 m), as prepared for the first version of this study.' },
        { title: 'Thomson, C. W.; Murray, J. (1885). Report on the scientific results of the voyage of H.M.S. Challenger: Narrative of the cruise.', note: 'The itinerary the ports and dates follow.' },
        { title: 'GEBCO Compilation Group (2024). GEBCO 2024 Grid.', url: 'https://www.gebco.net/', note: 'The depths everywhere, through the globe\'s own terrain map.' },
        { title: 'The ship is an original drawing after her rig, not a particular plate.', note: '' }
      ]
    });
  });
  terra.on('close', function () { Object.keys(raf).forEach(function (k) { cancelAnimationFrame(raf[k]); }); clearTimeout(glassTimer); });

  // ---- start ----
  terra.globe.set({ historical: true, still: true, maxZoom: 14 });
  measure(terra.camera);
  terra.caption.set({ title: 'The lead line', text: '', wikipedia: 'Challenger expedition' });
  terra.geo.set('ahead', [], { style: 'ring' }); terra.geo.set('reached', [], { style: 'dot' });
  terra.geo.set('leads', [], { style: 'sounding' });
  drawGlass();
  terra.time.show({ unit: 'day', min: FIRST - 2, max: LAST + 2, value: date, ticks: stations.map(function (s) { return s.d; }),
                    playable: true, rate: RATE, playing: true, carry: false });
})();
