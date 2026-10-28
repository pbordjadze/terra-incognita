/* Borders on old maps: Europe on eight dated maps, drawn by the globe itself (runtime 2).
   This page draws nothing. For the globe's date it picks the nearest dated map and asks the globe to
   draw that map's borders and names; a chosen country is filled, and its outline on the map before
   stays as a dashed line, so a change reads at a glance. Borders never move between maps: a year with
   no map shows the nearest one and says so. */
(function () {
  'use strict';
  var D = window.TIME_DATA;
  var maps = D.snapshots.slice().sort(function (a, b) { return a.year - b.year; });
  var land = (D.land && D.land.features) || [];
  // the 1945 map's occupation zones, named as a first-time reader would recognise them
  var NAMES = { 'Germany (UK)': 'British zone', 'Germany (USA)': 'American zone', 'Germany (France)': 'French zone', 'Germany (Soviet)': 'Soviet zone' };

  var year = 1945, chosen = 'poland', shownMap = null, shownChosen = null, lastNote = null, notice = null;

  function nameOf(f) { var n = f.properties.NAME || f.properties.name || 'Unnamed'; return NAMES[n] || n; }
  function subjectOf(f) {
    var n = f.properties.NAME || f.properties.name || '';
    if (/^(Poland|Poland-Lithuania|Polish.Lithuanian Commonwealth)$/i.test(n)) return 'poland';
    if (/Ottoman/i.test(n)) return 'ottoman';
    return n;
  }
  function polygons(g) { return g.type === 'Polygon' ? [g.coordinates] : g.type === 'MultiPolygon' ? g.coordinates : []; }
  function inRing(p, ring) {
    var inside = false;
    for (var i = 0, j = ring.length - 1; i < ring.length; j = i++) {
      var a = ring[i], b = ring[j];
      if ((a[1] > p[1]) !== (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]) inside = !inside;
    }
    return inside;
  }
  function contains(g, p) {
    return polygons(g).some(function (poly) { return inRing(p, poly[0]) && !poly.slice(1).some(function (r) { return inRing(p, r); }); });
  }
  /** A ring's area (in square degrees, shrunk with latitude) and centroid. */
  function ringStats(ring) {
    var a = 0, x = 0, y = 0;
    for (var i = 0, j = ring.length - 1; i < ring.length; j = i++) {
      var k = ring[j][0] * ring[i][1] - ring[i][0] * ring[j][1];
      a += k; x += (ring[j][0] + ring[i][0]) * k; y += (ring[j][1] + ring[i][1]) * k;
    }
    if (Math.abs(a) < 1e-12) return { area: 0, at: ring[0] };
    return { area: Math.abs(a / 2) * Math.cos(y / (3 * a) * Math.PI / 180), at: [x / (3 * a), y / (3 * a)] };
  }

  // each map's countries: their features, a place for the name (the largest part's centroid, when it
  // falls inside that part), and their size, so the larger name is placed first
  maps.forEach(function (m) {
    var bySubject = {};
    m.features.forEach(function (f) {
      var id = subjectOf(f);
      var c = bySubject[id] || (bySubject[id] = { id: id, name: nameOf(f), features: [], area: 0, at: null, best: 0 });
      c.features.push(f);
      polygons(f.geometry).forEach(function (poly) {
        var s = ringStats(poly[0]);
        c.area += s.area;
        if (s.area > c.best && inRing(s.at, poly[0])) { c.best = s.area; c.at = s.at; }
      });
    });
    m.countries = Object.keys(bySubject).map(function (k) { return bySubject[k]; })
      .sort(function (a, b) { return b.area - a.area; });
  });

  function nearestMap(y) {
    return maps.reduce(function (best, m) { return Math.abs(m.year - y) < Math.abs(best.year - y) ? m : best; });
  }
  function country(m, id) { return m.countries.filter(function (c) { return c.id === id; })[0] || null; }
  /** The chosen country on the closest earlier map that has it. */
  function before(m, id) {
    for (var i = maps.indexOf(m) - 1; i >= 0; i--) { var c = country(maps[i], id); if (c) return { map: maps[i], country: c }; }
    return null;
  }
  function geometries(c) { return c ? c.features.map(function (f) { return f.geometry; }) : []; }
  function yearLabel(y) { var r = Math.round(y); return r < 1 ? (1 - r) + ' BCE' : String(r); }

  function say() {
    var m = nearestMap(year), y = Math.round(year), exact = y === m.year;
    var which = exact ? 'the ' + m.year + ' map' : 'the ' + m.year + ' map, the nearest to ' + yearLabel(y);
    var only = m.year < 1700 ? ' The maps this early show only the Ottoman Empire.' : '';
    var c = chosen ? country(m, chosen) : null, title, text;
    if (notice) {
      title = notice.title; text = notice.text;
    } else if (c) {
      var prior = before(m, chosen);
      title = c.name;
      text = 'As drawn on ' + which + '.' +
        (prior ? ' The dashed line is ' + prior.country.name + ' on the ' + prior.map.year + ' map.' : '') + only;
    } else if (chosen) {
      var known = maps.map(function (k) { return country(k, chosen); }).filter(Boolean)[0];
      title = known ? known.name : 'Borders on old maps';
      text = (exact ? 'The ' + m.year + ' map' : 'This year shows ' + which + ', which') + ' has no ' + (known ? known.name : 'such country') +
        '. Drag the date to another map.' + only;
    } else {
      title = 'Borders on old maps';
      text = (exact ? 'Europe as drawn on a map of ' + m.year + '.' : 'There is no map for ' + yearLabel(y) + ' here: this is ' + which + '.') +
        ' Tap a country to follow it from map to map; borders change only where a map does.' + only;
    }
    var note = title + '|' + text;
    if (note !== lastNote) { lastNote = note; terra.caption.set({ title: title, text: text }); }
  }

  function draw() {
    var m = nearestMap(year);
    if (m !== shownMap) {
      shownMap = m;
      terra.geo.set('borders', m.features.map(function (f) { return f.geometry; }), { style: 'border' });
      shownChosen = undefined;
    }
    if (shownChosen !== chosen) {
      shownChosen = chosen;
      var c = chosen ? country(m, chosen) : null, prior = chosen ? before(m, chosen) : null;
      terra.geo.set('chosen', geometries(c), { style: 'area', color: '#C17846' });
      terra.geo.set('before', prior ? geometries(prior.country) : [], { style: 'ghost' });
      // names, larger countries first; the chosen one before all, so it always has room
      var named = m.countries.filter(function (k) { return k.at; });
      named.sort(function (a, b) { return (b.id === chosen) - (a.id === chosen) || b.area - a.area; });
      terra.geo.set('names', named.map(function (k) {
        return { type: 'Feature', geometry: { type: 'Point', coordinates: k.at }, properties: { label: k.name } };
      }), { style: 'label', font: 'caps', size: 11 });
    }
    say();
  }

  terra.on('time', function (e) {
    year = e.value;
    notice = null;
    draw();
  });

  terra.on('tap', function (t) {
    if (t.lon == null || t.lat == null) return;
    var m = nearestMap(year), p = [t.lon, t.lat];
    var hit = m.countries.filter(function (c) { return c.features.some(function (f) { return contains(f.geometry, p); }); })[0];
    notice = null;
    if (hit) {
      chosen = hit.id;
    } else if (land.some(function (f) { return contains(f.geometry, p); })) {
      notice = { title: 'Borders on old maps', text: m.year < 1700
        ? 'The ' + m.year + ' map here shows only the Ottoman Empire, so nothing is drawn there.'
        : 'These maps cover Europe; nothing is drawn there on the ' + m.year + ' map.' };
    } else {
      chosen = null;
    }
    draw();
  });

  // the globe as a reference only (no modern borders or names), and still where the reader leaves it
  terra.globe.set({ historical: true, still: true });
  terra.geo.set('borders', [], { style: 'border' });
  terra.geo.set('chosen', [], { style: 'area' });
  terra.geo.set('before', [], { style: 'ghost' });
  terra.geo.set('names', [], { style: 'label', font: 'caps', size: 11 });
  terra.time.show({
    unit: 'year', min: 1500, max: 2000, value: year,
    ticks: maps.map(function (m) { return m.year; }),
    playable: false, carry: true
  });
  draw();
})();
