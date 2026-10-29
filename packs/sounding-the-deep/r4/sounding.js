/* Sounding the deep: HMS Challenger's dated stations, drawn by the globe itself (runtime 2).
   This page draws nothing. It decides, for the globe's date and camera, what the globe shows:
   the stations already reached (filled) and still ahead (hollow), the illustrated transits between
   close stations, the ship where the record puts it, and, when a station is touched, the line let
   down to its recorded depth. Evidence rules from the first version (voyage.js) are kept: a transit
   is drawn only across a gap of two weeks or less that stays off the modern land reference, and
   across a longer gap there is no ship at all. */
(function () {
  'use strict';
  var M = window.ShipMath, D = window.SHIP_DATA;
  var NEAR = 1.5;            // closer than this the voyage shows; further, only the ship, small
  var PT_PER_M = 0.018;      // the hanging line: 18 points to a kilometre of depth (the deepest here, about 120)
  var REACH = 24;            // how near a touch must be to a station or the ship, in points

  function inRing(p, ring) {
    var inside = false;
    for (var i = 0, j = ring.length - 1; i < ring.length; j = i++) {
      var a = ring[i], b = ring[j];
      if ((a[1] > p[1]) !== (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]) inside = !inside;
    }
    return inside;
  }
  function contains(g, p) {
    var polys = g.type === 'Polygon' ? [g.coordinates] : g.type === 'MultiPolygon' ? g.coordinates : [];
    return polys.some(function (poly) { return inRing(p, poly[0]) && !poly.slice(1).some(function (r) { return inRing(p, r); }); });
  }

  var stations = M.prepare(D.observations, (D.land && D.land.features) || [], contains);
  var first = stations[0].day, last = stations[stations.length - 1].day;
  // it starts where the record is dense: the Atlantic crossing from 18 February 1873
  var opening = stations.filter(function (s) { return s.date === '1873-02-18'; })[0] || stations[0];
  var day = opening.day, near = false, sounding = null, raf = 0, drawnPast = -1, shownNote = null;

  function point(lon, lat, props) {
    return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat] }, properties: props || {} };
  }
  function longDate(d) {
    return M.date(d).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });
  }
  function metres(m) { return Math.round(m).toLocaleString('en-GB') + ' m'; }
  /** As the expedition recorded it, in fathoms, then about how far that is (read to the nearest mark). */
  function recorded(s) {
    return s.fathoms.toLocaleString('en-GB') + ' fathoms (about ' + metres(Math.round(s.depth / 10) * 10) + ')';
  }
  function stationName(s) { return s.station.replace(/^CHA-/, ''); }

  function transits() {
    var lines = [];
    for (var i = 0; i < stations.length - 1; i++) {
      var a = stations[i], b = stations[i + 1];
      if (!a.connect) continue;
      var coordinates = [];
      for (var n = 0; n <= 16; n++) { var p = M.interpolate(a, b, n / 16); coordinates.push([p.lon, p.lat]); }
      lines.push({ type: 'LineString', coordinates: coordinates });
    }
    return lines;
  }

  /** What the reader is looking at first, then why it matters, then where to go. */
  function say() {
    var here = M.position(stations, day), title = 'HMS Challenger', text, wikipedia = '';
    if (sounding) {
      // (under Wikipedia's words, the record alone: the depth is Murray and Renard's, not the atlas's prose)
      text = 'Station ' + stationName(sounding) + ', ' + longDate(sounding.day) + ': the line ran out to ' +
        recorded(sounding) + (terra.settings.wikipedia ? '.' : '. Touch another station, or drag the date.');
    } else if (!near) {
      wikipedia = 'Challenger expedition';   // (the voyage's article, in wikipedia.json)
      text = 'A British ship sent out in 1872 to explore the deep ocean. ' + (here.kind === 'recorded' || here.kind === 'interpolated'
        ? 'Zoom in on the little ship to follow it.' : 'This record has no position for it on ' + longDate(day) + '; drag the date.');
    } else if (here.kind === 'gap') {
      text = 'This record has no stations between ' + longDate(M.day(here.previousDate)) + ' and ' + longDate(M.day(here.nextDate)) +
        ', so the ship is not shown.';
    } else if (here.kind === 'outside') {
      text = day < first ? 'Before the first station in this record.' : 'After the last station in this record.';
    } else {
      text = 'Each dot is a station where the crew stopped to measure the depth and bring up samples of the sea floor. ' +
        'Touch one to lower the line.';
    }
    var note = title + '|' + text;
    if (note !== shownNote) { shownNote = note; terra.caption.set({ title: title, text: text, wikipedia: wikipedia }); }
  }
  terra.on('settings', function () { shownNote = null; say(); });

  function draw() {
    var here = M.position(stations, day);
    var past = 0;
    while (past < stations.length && stations[past].day <= day) past++;
    if (past !== drawnPast) {
      drawnPast = past;
      terra.geo.set('ahead', stations.slice(past).map(function (s) { return point(s.lon, s.lat); }), { style: 'ring', minZoom: NEAR });
      terra.geo.set('reached', stations.slice(0, past).map(function (s) { return point(s.lon, s.lat); }), { style: 'dot', minZoom: NEAR });
    }
    var at = here.kind === 'recorded' || here.kind === 'interpolated';
    terra.geo.set('ship', at ? [point(here.lon, here.lat)] : [], { style: 'symbol', image: 'ship.png', width: 26, height: 26 });
    terra.geo.set('name', at ? [point(here.lon, here.lat, { label: 'HMS Challenger' })] : [],
                  { style: 'label', font: 'italic', size: 16, minZoom: NEAR, offset: [0, -38] });
    say();
  }

  function stopLine() {
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
    if (sounding) { sounding = null; terra.geo.set('sounding', [], { style: 'sounding' }); }
  }

  /** The line let down from the station to the depth recorded there, counting as it goes. */
  function lower(s) {
    stopLine();
    sounding = s;
    var full = Math.min(170, s.depth * PT_PER_M);
    var seconds = terra.settings.reduceMotion ? 0 : Math.min(3.2, 0.9 + s.depth / 2400);
    var start = performance.now();
    function step(now) {
      var f = seconds ? Math.min(1, (now - start) / (seconds * 1000)) : 1;
      var e = 1 - (1 - f) * (1 - f);
      terra.geo.set('sounding', [point(s.lon, s.lat, {
        length: full * e,
        label: f < 1 ? Math.round(s.fathoms * e).toLocaleString('en-GB') + ' fathoms' : recorded(s)
      })], { style: 'sounding' });
      raf = f < 1 ? requestAnimationFrame(step) : 0;
    }
    raf = requestAnimationFrame(step);
    say();
  }

  function nearest(x, y, places) {
    var best = null, distance = REACH;
    places.forEach(function (s) {
      var p = terra.project(s.lon, s.lat);
      if (!p || !p.visible) return;
      var d = Math.hypot(p.x - x, p.y - y);
      if (d < distance) { best = s; distance = d; }
    });
    return best;
  }

  terra.on('tap', function (t) {
    if (!terra.camera) return;
    var here = M.position(stations, day);
    var ship = here.kind === 'recorded' || here.kind === 'interpolated' ? [{ lon: here.lon, lat: here.lat }] : [];
    if (terra.camera.zoom < NEAR) {
      // far away, the ship is the only thing here: touching it goes to it
      if (nearest(t.x, t.y, ship)) terra.flyTo(here.lon, here.lat, 2.4, 1.2);
      return;
    }
    var s = nearest(t.x, t.y, stations);
    if (!s && nearest(t.x, t.y, ship) && here.kind === 'recorded') s = here;
    if (s) {
      terra.time.play(false);
      day = s.day;
      terra.time.set(day);
      draw();
      lower(s);
    } else if (nearest(t.x, t.y, ship)) {
      terra.caption.set({ title: 'HMS Challenger', text: 'Between stations, on ' + longDate(day) +
        ': no depth was recorded on this day. The dashed line is an illustration, not its course.' });
      shownNote = null;
    }
  });

  terra.on('time', function (e) {
    day = e.value;
    // a date carried from another prototype outside this record: its nearest end, with the ship there
    if (e.by === 'host' && (day < first || day > last)) {
      day = day < first ? first : last;
      terra.time.set(day);
    }
    // playing on across a gap in the record: on to the next station, rather than months of empty sea
    var here = M.position(stations, day);
    if (e.by === 'play' && here.kind === 'gap') {
      var next = stations.filter(function (s) { return s.day > day; })[0];
      if (next) { day = next.day; terra.time.set(day); }
    }
    if (sounding && Math.floor(day) !== sounding.day) stopLine();
    draw();
  });

  terra.on('camera', function (c) {
    var close = c.zoom >= NEAR;
    if (close !== near) { near = close; say(); }
  });

  terra.on('close', stopLine);

  // the globe as a reference only (no modern borders or names), and still where the reader leaves it
  terra.globe.set({ historical: true, still: true });
  // layers in drawing order, lowest first
  terra.geo.set('transits', transits(), { style: 'track', minZoom: NEAR });
  terra.geo.set('ahead', [], { style: 'ring', minZoom: NEAR });
  terra.geo.set('reached', [], { style: 'dot', minZoom: NEAR });
  terra.geo.set('ship', [], { style: 'symbol', image: 'ship.png', width: 26, height: 26 });
  terra.geo.set('name', [], { style: 'label', font: 'italic', size: 16, minZoom: NEAR, offset: [0, -38] });
  terra.geo.set('sounding', [], { style: 'sounding' });
  terra.time.show({
    unit: 'day', min: first - 2, max: last + 2, value: opening.day,
    ticks: stations.map(function (s) { return s.day; }),
    playable: true, rate: 0.8, playing: true, carry: true
  });
  draw();
})();
