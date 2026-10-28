/* The sea between: the Aegean leaf as a pack drawn by the globe (runtime 3, docs/runtime3.md).
   The leaf's engraving lies on the globe and fades in as the reader comes close; its marginal life
   (wind strokes, a paper skiff, a whale, a palm) lives on it as sprites; its four place notes open in
   the panel with their sources and limits; and the sounding glass reads the chart's generalised
   depth grid wherever the reader taps or drags it. This script draws nothing itself: it tells the
   globe what to show. Data: data.js (tools/aegean/pack.cjs), notes.js (the leaf's own, unchanged). */
(function () {
  'use strict';
  var SEA = window.SEA, NOTES = window.LeafNotes, G = SEA.grid, SHEET = SEA.engraving; // [west, south, east, north]
  var D = Math.PI / 180;

  var CENTRE = { lon: (SHEET[0] + SHEET[2]) / 2, lat: (SHEET[1] + SHEET[3]) / 2 };   // the sheet's
  var MAX_ZOOM = 32;       // the globe lets the pack come this close (it is put back when the pack ends):
                           // the leaf's own first view is about zoom 14 on an iPad and 25 on a phone
  var PT_PER_M = 0.014;    // the sounding line hangs 14 points for each kilometre of depth
  var INK = '#294652', COPPER = '#9B4934', PAPER = '#F4F1E8', COAST = '#4B6A6B';

  // How big the globe is on this screen decides where things show. A degree is radius × π/180 points,
  // and the globe's radius at zoom 1 is about 489 points on the 13-inch iPad but 176 on an iPhone. So the
  // engraving, the names and the drawings come in when a degree is so many points across (`at`), not at
  // one zoom for every screen: the chart shows the same at the same size on either, and the phone zooms
  // further for it. What would need more than MAX_ZOOM (the drawings' larger close looks on a phone) waits.
  var R1 = 0, SHOWN = 2.2, SEEN = 2.6, NEAR = 2.6;
  function zoomFor(ppd) { return Math.round(ppd / (R1 * D) * 100) / 100; }
  function at(ppd, lo, hi) { return Math.max(lo || 0, Math.min(hi || MAX_ZOOM, zoomFor(ppd))); }
  function close(ppd) { var z = zoomFor(ppd); return z <= MAX_ZOOM ? z : 99; }
  function larger(size) { return [Math.round(size[0] * 1.4), Math.round(size[1] * 1.4)]; }
  /** The zoom at which the whole sheet fits this screen with room round it: about 12 on the 13-inch iPad,
      13.5 on an iPhone. The pack flies there itself, once it may zoom past 8 (a manifest anchor may not). */
  function fitZoom(c) {
    var across = (SHEET[2] - SHEET[0]) * Math.cos(CENTRE.lat * D) * D, down = (SHEET[3] - SHEET[1]) * D;
    var width = (c && c.width) || 1032, height = (c && c.height) || 1376;
    var z = Math.min(0.88 * width / (across * R1), 0.6 * height / (down * R1));
    return Math.round(Math.max(4, Math.min(MAX_ZOOM, z)) * 10) / 10;
  }
  /** From a camera: the globe's radius at zoom 1 here; true when it changed enough to arrange again. */
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : 489;
    if (R1 && Math.abs(r1 - R1) / R1 < 0.08) return false;
    R1 = r1;
    SHOWN = at(17, 2.2);                    // the engraving starts to come in (over 0.8 of zoom)
    SEEN = SHOWN + 0.4;                     // half in
    NEAR = Math.max(SEEN, at(22, 2.6));     // large enough to sound
    return true;
  }

  // ----- the chart's sounding grid -------------------------------------------------------------

  function decode64(s) {
    var abc = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/', lut = new Uint8Array(128), i;
    for (i = 0; i < 64; i++) lut[abc.charCodeAt(i)] = i;
    s = s.replace(/=+$/, '');
    var out = new Uint8Array(Math.floor(s.length * 3 / 4)), o = 0, buf = 0, bits = 0;
    for (i = 0; i < s.length; i++) {
      buf = (buf << 6) | lut[s.charCodeAt(i)];
      bits += 6;
      if (bits >= 8) { bits -= 8; out[o++] = (buf >> bits) & 255; buf &= (1 << bits) - 1; }
    }
    return out;
  }
  var landBits = decode64(G.land), depths = decode64(G.depth);

  /** On the sheet lying on the globe (the grid reaches further, under the globe's own map). */
  function within(lon, lat) { return lon >= SHEET[0] && lon <= SHEET[2] && lat >= SHEET[1] && lat <= SHEET[3]; }

  /** What the chart says at a place: land, or the depth rounded to 100 m (null outside the chart). */
  function sound(lon, lat) {
    var x = Math.floor((lon - G.west) / (G.east - G.west) * G.width);
    var y = Math.floor((G.north - lat) / (G.north - G.south) * G.height);
    if (x < 0 || y < 0 || x >= G.width || y >= G.height) return null;
    var i = y * G.width + x;
    if ((landBits[i >> 3] >> (7 - (i & 7))) & 1) return { land: true };
    var step = depths[i];
    return { land: false, metres: Math.round(step * G.metresPerStep / 100) * 100, deeper: step === 255 };
  }

  function thousands(n) { return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ','); }
  function depthWords(r) {
    if (r.deeper) return 'about ' + thousands(r.metres) + ' m or more';
    if (r.metres === 0) return 'less than 50 m';
    return 'about ' + thousands(r.metres) + ' m';
  }
  function depthLabel(r) {
    if (r.land) return 'Land · generalised';
    if (r.deeper) return '~' + thousands(r.metres) + ' m or more · generalised';
    if (r.metres === 0) return 'under 50 m · generalised';
    return '~' + thousands(r.metres) + ' m · generalised';
  }

  // ----- geometry ------------------------------------------------------------------------------

  function point(lon, lat, props) {
    return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat] }, properties: props || {} };
  }
  function unit(lon, lat) {
    var la = lat * D, lo = lon * D;
    return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)];
  }
  function r4(v) { return Math.round(v * 1e4) / 1e4; }

  /** The ring of places r radians from a centre (a small circle on the globe). */
  function circle(lon, lat, r, n) {
    var p1 = lat * D, l1 = lon * D, ring = [];
    for (var i = 0; i <= n; i++) {
      var th = (i % n) / n * 2 * Math.PI;
      var p2 = Math.asin(Math.sin(p1) * Math.cos(r) + Math.cos(p1) * Math.sin(r) * Math.cos(th));
      var l2 = l1 + Math.atan2(Math.sin(th) * Math.sin(r) * Math.cos(p1), Math.cos(r) - Math.sin(p1) * Math.sin(p2));
      ring.push([r4(l2 / D), r4(p2 / D)]);
    }
    return ring;
  }

  // The depth contours, each vertex's place on the globe worked out once.
  var LINES = SEA.contours.map(function (c) {
    var f = c[1], n = f.length / 2, v = new Float64Array(n * 3), box = [180, 90, -180, -90];
    for (var i = 0; i < n; i++) {
      var u = unit(f[2 * i], f[2 * i + 1]);
      v[3 * i] = u[0]; v[3 * i + 1] = u[1]; v[3 * i + 2] = u[2];
      box[0] = Math.min(box[0], f[2 * i]); box[1] = Math.min(box[1], f[2 * i + 1]);
      box[2] = Math.max(box[2], f[2 * i]); box[3] = Math.max(box[3], f[2 * i + 1]);
    }
    return { level: c[0], f: f, v: v, n: n, box: box };
  });

  /** The contours inside the glass, cut at its rim and at the sheet's edge: { major: [lines], minor: [lines] }. */
  function contoursWithin(lon, lat, r) {
    var c = unit(lon, lat), cosR = Math.cos(r), major = [], minor = [];
    var deg = r / D, slack = deg / Math.max(0.2, Math.cos(Math.min(85, Math.abs(lat) + deg) * D));
    LINES.forEach(function (L) {
      var b = L.box;
      if (b[3] < lat - deg || b[1] > lat + deg || b[2] < lon - slack || b[0] > lon + slack) return;
      var into = L.level % 1000 === 0 ? major : minor, run = null, before = 0, f = L.f;
      function dot(i) { return L.v[3 * i] * c[0] + L.v[3 * i + 1] * c[1] + L.v[3 * i + 2] * c[2]; }
      function inside(i, d) { return d >= cosR && within(f[2 * i], f[2 * i + 1]); }
      // where the way from vertex a (inside) to z (outside) leaves the glass or the sheet
      function edge(a, z, da, dz) {
        var x0 = f[2 * a], y0 = f[2 * a + 1], dx = f[2 * z] - x0, dy = f[2 * z + 1] - y0;
        var t = dz < cosR ? (da - cosR) / (da - dz) : 1;
        if (x0 + dx < SHEET[0]) t = Math.min(t, (SHEET[0] - x0) / dx);
        if (x0 + dx > SHEET[2]) t = Math.min(t, (SHEET[2] - x0) / dx);
        if (y0 + dy < SHEET[1]) t = Math.min(t, (SHEET[1] - y0) / dy);
        if (y0 + dy > SHEET[3]) t = Math.min(t, (SHEET[3] - y0) / dy);
        t = Math.max(0, Math.min(1, t));
        return [r4(x0 + dx * t), r4(y0 + dy * t)];
      }
      for (var i = 0; i < L.n; i++) {
        var d = dot(i);
        if (inside(i, d)) {
          if (!run) { run = []; if (i > 0) run.push(edge(i, i - 1, d, before)); }
          run.push([f[2 * i], f[2 * i + 1]]);
        } else if (run) {
          run.push(edge(i - 1, i, before, d));
          if (run.length > 1) into.push(run);
          run = null;
        }
        before = d;
      }
      if (run && run.length > 1) into.push(run);
    });
    return { major: major, minor: minor };
  }

  // ----- the words at the foot -----------------------------------------------------------------

  var TITLE = 'The sea between';
  var FAR = 'An engraved chart of the Aegean lies on the globe here. Come closer to see it.';
  var MID = 'An engraved chart of the Aegean, lying on the globe. Come closer to sound its sea and read its notes.';
  var CLOSE = 'An engraved chart of the Aegean, lying on the globe. Beneath its islands the sea falls ' +
              'thousands of metres: tap the water to sound it, or touch a copper name for its note.';
  var QUALIFIED = 'from GEBCO 2008 imagery (NASA Visible Earth), generalised and rounded to 100 m: not survey data.';
  var near = false, seen = false, glass = null, flash = null, flashTimer = 0, shown = '';

  function say() {
    var title = TITLE, text;
    if (flash) {
      title = flash.title; text = flash.text;
    } else if (glass && glass.reading) {
      title = 'The sounding glass';
      var r = glass.reading;
      text = r.land
        ? 'Land, on the chart’s generalised coast: the glass sounds only the sea. Drag it onto the water, or tap its centre to put it away.'
        : depthWords(r).charAt(0).toUpperCase() + depthWords(r).slice(1) + ' deep here, ' + QUALIFIED +
          ' Drag the glass to sound along; tap its centre to put it away.';
    } else {
      text = near ? CLOSE : seen ? MID : FAR;
    }
    if (title + '|' + text !== shown) { shown = title + '|' + text; terra.caption.set({ title: title, text: text }); }
  }

  function tell(title, text) {
    flash = { title: title, text: text };
    clearTimeout(flashTimer);
    flashTimer = setTimeout(function () { flash = null; say(); }, 7000);
    say();
  }
  function untell() { if (flash) { flash = null; clearTimeout(flashTimer); } }

  // ----- the sounding glass --------------------------------------------------------------------

  /** The glass's radius, in radians on the globe: 36 to 60 points, a lens a fingertip can hold (the leaf's
      is larger, but its page zooms far closer than the globe does). */
  function radius(c) {
    var pts = Math.max(36, Math.min(60, Math.min(c.width, c.height) * 0.06));
    return Math.min(0.08, pts / c.radius);
  }

  var GLASS = { 'glass': 'area', 'glass-minor': 'track', 'glass-major': 'border', 'glass-rim': 'border', 'sounding': 'sounding' };
  function drawGlass() {
    var c = terra.camera;
    if (!glass || !c) return;
    var r = radius(c), ring = circle(glass.lon, glass.lat, r, 72), lines = contoursWithin(glass.lon, glass.lat, r);
    glass.r = r;
    glass.reading = sound(glass.lon, glass.lat) || { land: true };
    terra.geo.set('glass', [{ type: 'Feature', geometry: { type: 'Polygon', coordinates: [ring] }, properties: {} }],
                  { style: 'area', color: PAPER, touchable: true, minZoom: NEAR });
    terra.geo.set('glass-minor', lines.minor.length ? [{ type: 'MultiLineString', coordinates: lines.minor }] : [],
                  { style: 'track', color: INK, lineWidth: 0.8, minZoom: NEAR });
    terra.geo.set('glass-major', lines.major.length ? [{ type: 'MultiLineString', coordinates: lines.major }] : [],
                  { style: 'border', color: COAST, lineWidth: 1, minZoom: NEAR });
    terra.geo.set('glass-rim', [{ type: 'LineString', coordinates: ring }], { style: 'border', color: COPPER, lineWidth: 1.3, minZoom: NEAR });
    var reading = glass.reading;
    terra.geo.set('sounding', [point(glass.lon, glass.lat, {
      length: reading.land ? 0 : Math.round(Math.min(120, reading.metres * PT_PER_M) * 10) / 10,
      label: depthLabel(reading)
    })], { style: 'sounding', minZoom: NEAR });
    say();
  }

  function placeGlass(lon, lat) {
    var m = 0.02;
    lon = Math.min(SHEET[2] - m, Math.max(SHEET[0] + m, lon));
    lat = Math.min(SHEET[3] - m, Math.max(SHEET[1] + m, lat));
    if (!glass) glass = {};
    glass.lon = lon; glass.lat = lat;
  }

  var lastDraw = 0, trailing = 0;
  /** While dragging, at most 25 drawings a second, and always the last place. */
  function drawSoon() {
    var now = Date.now();
    clearTimeout(trailing);
    if (now - lastDraw >= 40) { lastDraw = now; drawGlass(); } else trailing = setTimeout(function () { lastDraw = Date.now(); drawGlass(); }, 40);
  }

  function closeGlass() {
    if (!glass) return;
    glass = null;
    clearTimeout(trailing);
    Object.keys(GLASS).forEach(function (id) { terra.geo.set(id, [], { style: GLASS[id] }); });
    say();
  }

  // ----- the notes ------------------------------------------------------------------------------

  // The leaf's Santorini note describes its own inset drawing, which is not here: on the globe its
  // limit says so, from the same facts (notes.js and SOURCES.md). The other limits hold as written.
  var LIMITS = {
    santorini: 'This note has no drawing here, and the chart’s general coastline (Natural Earth, 1:10 million) leaves out ' +
               'Santorini’s small central islands. Nothing here is a reconstruction of an eruption or a cross-section, and no exact ' +
               'eruption date is asserted.'
  };
  // where each note's copper name goes: its place (the wind's, where its strokes are drawn) and an offset in points
  var NOTE_PLACES = { wind: [0, 21], santorini: [0, 12], vai: [16, 12], deep: [0, 26] };
  function notePlace(n) { return n.id === 'wind' ? SEA.places.wind : n.point; }

  function note(id) { return NOTES.filter(function (n) { return n.id === id; })[0]; }
  function openNote(id) {
    var n = note(id);
    if (!n) return;
    terra.panel.show({
      title: n.name,
      text: [n.title + ' ' + n.deck].concat(n.body).join('\n\n'),
      sections: [{ title: 'The drawing and its limits', text: LIMITS[n.id] || n.limit }],
      sources: n.sources.map(function (s) { return { title: s.title, url: s.url, note: s.text }; })
    });
  }

  var ABOUT = {
    title: 'The sea between',
    text: 'An engraved chart of the Aegean, lying on the globe: present-day, generalised physical geography, with no political ' +
          'boundaries drawn. It is a small authored leaf, not a navigational chart or a complete atlas.\n\n' +
          'An atlas has room for an idle hand.',
    sections: [
      { title: 'What you can do',
        text: 'Tap the water to set down the sounding glass: a line hangs to the generalised depth there, and the depth contours show ' +
              'inside the glass. Drag the glass to sound along the sea; tap its centre to put it away.\n\n' +
              'Touch a copper name, the wind strokes, the palm or the whale to read its note. Touch the paper skiff to fold or raise its sail.' },
      { title: 'What is drawn',
        text: 'The coasts are Natural Earth’s, at 1:10 million. The relief contours and short hachures on the land are derived from ' +
              'elevation in GEBCO 2008 imagery (NASA Visible Earth), not invented mountain symbols: they are an illustration of terrain, not survey ' +
              'contours. The engraving was prepainted from the chart by the leaf’s own drawing code.' },
      { title: 'What is measured',
        text: 'The sounding glass reads a grid of roughly 5 km cells prepared from NASA Visible Earth’s grayscale imagery of GEBCO 2008 bathymetry, rounded to ' +
              '100 m, and its 250 to 4,000 m contours are generalised from the same source. These are legacy imagery derivatives, not ' +
              'current hydrographic survey data: they must not be used to infer local water depths or island-scale coast accuracy. ' +
              'The line hangs 14 points for each kilometre of depth.' },
      { title: 'What is imagined',
        text: 'The wind strokes, the skiff and the moving whale are imaginative marks on the page. They are not weather, transport ' +
              'or wildlife observations. The palm is a small botanical emblem, enlarged far beyond map scale.' },
      { title: 'The notes',
        text: 'Each note’s summary, sources and limits travel together. They were checked against the institutional pages they cite ' +
              'on October 26, 2026; this is not independent scholarly peer review, and none of the notes is a complete regional ' +
              'account. Their broader phrasing is editorial interpretation.' }
    ],
    sources: [
      { title: 'Natural Earth · ne_10m_land, 1:10 million', url: 'https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_10m_land.geojson',
        note: 'Coastlines.' },
      { title: 'Natural Earth · Terms of use', url: 'https://www.naturalearthdata.com/about/terms-of-use/', note: 'Public domain.' },
      { title: 'NASA Visible Earth · Topography, grayscale GEBCO 2008 imagery', url: 'https://eoimages.gsfc.nasa.gov/images/imagerecords/73000/73934/gebco_08_rev_elev_21600x10800.png',
        note: 'Relief: a grayscale increment of 25 m.' },
      { title: 'NASA Visible Earth · Bathymetry, grayscale GEBCO 2008 imagery', url: 'https://eoimages.gsfc.nasa.gov/images/imagerecords/73000/73963/gebco_08_rev_bath_21600x10800.png',
        note: 'Approximate depths: (255 − gray) × 43 m, generalised for this drawing.' },
      { title: 'NASA · Images and media usage', url: 'https://www.nasa.gov/nasa-brand-center/images-and-media/',
        note: 'The terms for NASA imagery.' }
    ]
  };

  // ----- the marginal life ----------------------------------------------------------------------

  function frames(prefix, n, pad) {
    var out = [];
    for (var i = 1; i <= n; i++) out.push(prefix + (pad ? ('0' + i).slice(-2) : String(i)) + '.png');
    return out;
  }
  function turned(list, k) { k = ((k % list.length) + list.length) % list.length; return list.slice(k).concat(list.slice(0, k)); }

  var WHALE = frames('whale-', 8), BLOW = frames('blow-', 4), STREAM = frames('stream-', 16, true);
  var sailUp = true;

  function setSkiff() {
    var skiff = frames(sailUp ? 'skiff-' : 'skiff-folded-', 8);
    terra.sprites.set('skiff', {
      path: { coordinates: SEA.places.skiff, seconds: 150, loop: 'back' },
      looks: [
        { frames: skiff, fps: 1.4, size: larger([28, 32]), minZoom: close(130) },
        { frames: skiff, fps: 1.4, size: [28, 32], minZoom: at(34, 4) },
        { image: skiff[0], size: [18, 21], minZoom: 1.5, maxZoom: at(34, 4) }
      ],
      label: 'A paper skiff, drawn: not a vessel or a route. Touch it to fold or raise its sail.',
      touchable: true
    });
  }

  function life() {
    // loose strokes drifting south over open water, each a little out of step with the others
    SEA.places.streams.forEach(function (place, i) {
      terra.sprites.set('stream-' + (i + 1), {
        at: place,
        looks: [{ frames: turned(STREAM, [0, 2, -2, 3, -3, 1][i % 6]), fps: 3.5 + (i % 3) * 0.5, size: [10, 36], minZoom: at(43, 5) }]
      });
    });
    terra.sprites.set('wind', {
      at: SEA.places.wind,
      looks: [{ image: 'wind.png', size: larger([28, 26]), minZoom: close(130) },
              { image: 'wind.png', size: [28, 26], minZoom: at(34, 4) }],
      label: 'Wind strokes over the Cyclades, drawn: not measured wind. Touch for the note on the summer wind.',
      touchable: true
    });
    terra.sprites.set('palm', {
      at: note('vai').point,
      looks: [{ image: 'palm.png', size: larger([22, 50]), minZoom: close(130) },
              { image: 'palm.png', size: [22, 50], minZoom: at(34, 4) }],
      label: 'A Cretan date palm at Vai, an emblem far larger than the grove. Touch for its note.',
      touchable: true
    });
    terra.sprites.set('whale', {
      path: { coordinates: SEA.places.whale, seconds: 320, loop: 'back' },
      looks: [
        { frames: WHALE, fps: 3, size: larger([46, 24]), minZoom: close(130), face: true, every: 60, for: 38 },
        { frames: WHALE, fps: 3, size: [46, 24], minZoom: at(40, 4.7), face: true, every: 60, for: 38 },
        { frames: BLOW, fps: 3, size: [16, 16], minZoom: 1.6, maxZoom: at(40, 4.7), every: 11, for: 1.4 }
      ],
      label: 'A sperm whale, drawn: an imaginative mark, not a sighting or a track. Touch for the note on the deep water south of Crete.',
      touchable: true
    });
    setSkiff();
  }

  // ----- touch ----------------------------------------------------------------------------------

  var NOTE_LAYERS = {};
  NOTES.forEach(function (n) { NOTE_LAYERS['note-' + n.id] = n.id; });

  function comeClose() { terra.flyTo(CENTRE.lon, CENTRE.lat, fitZoom(terra.camera), 1.4); }

  function touchSprite(id) {
    if (!near) { comeClose(); return; }
    if (id === 'whale') openNote('deep');
    else if (id === 'wind') openNote('wind');
    else if (id === 'palm') openNote('vai');
    else if (id === 'skiff') {
      sailUp = !sailUp;
      setSkiff();
      tell('The paper skiff', 'An imaginative mark on the page, not a vessel or a route. Its sail is ' + (sailUp ? 'raised.' : 'folded.'));
    }
  }

  terra.touch.scheme('touchables'); // a drag that begins on the glass moves it; any other turns the globe
  terra.on('touch', function (t) {
    var target = t.target || null;
    if (t.phase === 'began') {
      if (glass && target && target.layer === 'glass' && t.lon != null) {
        glass.drag = { lon: glass.lon - t.lon, lat: glass.lat - t.lat };
        untell();
      }
      return;
    }
    if (t.phase === 'moved') {
      if (glass && glass.drag && t.lon != null) { placeGlass(t.lon + glass.drag.lon, t.lat + glass.drag.lat); drawSoon(); }
      return;
    }
    if (t.phase === 'ended') {
      if (glass && glass.drag) { glass.drag = null; clearTimeout(trailing); drawGlass(); }
      return;
    }
    if (t.phase !== 'tap') return;
    if (target && target.sprite) { touchSprite(target.sprite); return; }
    if (target && NOTE_LAYERS[target.layer]) { openNote(NOTE_LAYERS[target.layer]); return; }
    if (t.lon == null) return;
    var inside = within(t.lon, t.lat);
    if (!near) { if (inside) comeClose(); return; }
    var onGlass = glass && target && target.layer === 'glass';
    if (onGlass) {
      var centre = terra.project(glass.lon, glass.lat);
      if (centre && Math.hypot(centre.x - t.x, centre.y - t.y) <= 24) { closeGlass(); return; }
    }
    if (!inside) { closeGlass(); return; }
    var here = sound(t.lon, t.lat);
    if (here && here.land && !glass) {
      tell('The sea between', 'Land, on the chart’s generalised coast. Tap the water to sound it.');
      return;
    }
    untell();
    placeGlass(t.lon, t.lat);
    drawGlass();
  });

  // ----- the camera, the chip, the end ----------------------------------------------------------

  terra.on('camera', function (c) {
    if (!c) return;
    if (measure(c)) arrange();
    var closeBy = c.zoom >= NEAR, visible = c.zoom >= SEEN;
    if (closeBy !== near || visible !== seen) {
      near = closeBy; seen = visible;
      if (!near) closeGlass();
      say();
    }
    if (glass && !glass.drag && glass.r && Math.abs(radius(c) - glass.r) / glass.r > 0.04) drawGlass();
  });
  terra.on('open', function () { terra.panel.show(ABOUT); });
  terra.on('close', function () { clearTimeout(flashTimer); clearTimeout(trailing); });

  // ----- what the globe shows -------------------------------------------------------------------

  var c0 = terra.camera;
  measure(c0);
  terra.globe.set({ still: true, maxZoom: MAX_ZOOM });
  terra.flyTo(CENTRE.lon, CENTRE.lat, fitZoom(c0), 1.6);   // the pack's anchor: the whole sheet, on this screen

  // geography, lowest first (the first set of an id fixes its place); names claim room in this order
  terra.geo.set('glass', [], { style: 'area', color: PAPER, touchable: true, minZoom: NEAR });
  terra.geo.set('glass-minor', [], { style: 'track', color: INK, minZoom: NEAR });
  terra.geo.set('glass-major', [], { style: 'border', color: COAST, minZoom: NEAR });
  terra.geo.set('glass-rim', [], { style: 'border', color: COPPER, minZoom: NEAR });
  terra.geo.set('sounding', [], { style: 'sounding', minZoom: NEAR });
  function names(id, list, options) {
    terra.geo.set(id, list.map(function (l) { return point(l[1], l[2], { label: l[0] }); }), options);
  }
  /** The names and the marginal life, for this screen (again when the globe's size changes). */
  function arrange() {
    terra.images.set('leaf', { image: 'engraving.png', bounds: SHEET, opacity: 1, minZoom: SHOWN, fade: 0.8 });
    NOTES.forEach(function (n) {
      terra.geo.set('note-' + n.id, [point(notePlace(n)[0], notePlace(n)[1], { label: n.name })],
                    { style: 'label', color: COPPER, size: 14, minZoom: at(34, 3.6), offset: NOTE_PLACES[n.id] || [0, 0], touchable: true });
    });
    names('name-sea', SEA.names.sea, { style: 'label', font: 'italic', size: 20, color: INK, minZoom: at(25, 3), maxZoom: at(75, 6) });
    names('name-regions', SEA.names.regions, { style: 'label', size: 13, color: INK, minZoom: at(36, 4.2) });
    names('name-waters', SEA.names.waters, { style: 'label', font: 'italic', size: 14, color: INK, minZoom: at(40, 4.5) });
    names('name-islands', SEA.names.islands, { style: 'label', size: 12, color: INK, minZoom: at(47, 5.5), offset: [0, 11] });
    life();
    if (glass) drawGlass();
  }

  arrange();
  near = !!(c0 && c0.zoom >= NEAR);
  seen = !!(c0 && c0.zoom >= SEEN);
  say();
})();
