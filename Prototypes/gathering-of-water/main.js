/* A gathering of water, on the globe (runtime 3). The study's river network of South America,
   drawn by the globe in its water ink, the great rivers from afar and the small ones only close.
   Touch a river (or brush along them) and its water is followed down to the coast or to an inland
   ending, the tributaries gathered above the touch light up, and the words at the foot say what
   the data supports: the distance along mapped channels, the named rivers it passes, the land that
   drains into it. The study's rules are kept: a touch snaps to the nearest line drawn, distances are
   summed channel lengths, the glint's speed is invented, and only what the dataset links is linked.
   This script decides; the globe draws. */
(function () {
  'use strict';
  var D = Math.PI / 180;
  var IDLE = {
    title: 'A gathering of water',
    text: 'The rivers of South America, each line running on to the coast or to an inland ending. ' +
          'Touch one to follow where its water goes, and to see what gathers there.'
  };

  var data = terra.readJSON('network.json'), places = terra.readJSON('places.json'), about = terra.readJSON('about.json');
  if (!data || !places) {
    terra.caption.set({ title: IDLE.title, text: 'The river network could not be read from this pack.' });
    return;
  }
  var net = WaterNet.build(data), edges = net.edges, LEVELS = data.levels;

  // The globe's water ink (Globe.metal's RIVER and COAST): the network in its river ink, lighter for
  // the smaller rivers; what gathers at a touch in its coast ink; the way down deeper still. Names in
  // the globe's own water and town inks.
  var INK = { trace: '#0B3D66', gathered: '#1C7AB0', brushed: '#1C7AB099', water: '#2A6A99', town: '#4A463F' };
  // Strahler order bands, the great rivers last so they lie on top. Each shows from a reading density
  // in points a degree (as the globe sizes its own names by the screen), so a phone and an iPad show
  // rivers equally far apart: the smallest only close in on a large screen. While water is traced the
  // rest goes quieter, as the study's did, so what is lit stands out.
  var BANDS = [
    { id: 'rivers-3', from: 1, to: 3, ppd: 34, color: '#4E9CCB6B', quiet: '#86A6B84D' },
    { id: 'rivers-4', from: 4, to: 4, ppd: 22, color: '#4E9CCB80', quiet: '#86A6B85C' },
    { id: 'rivers-5', from: 5, to: 5, ppd: 14, color: '#4E9CCB94', quiet: '#86A6B86B' },
    { id: 'rivers-6', from: 6, to: 6, ppd: 9, color: '#4E9CCBB0', quiet: '#86A6B87A' },
    { id: 'rivers-7', from: 7, to: 7, ppd: 6.3, color: '#4E9CCBCC', quiet: '#86A6B88C' },
    { id: 'rivers-8', from: 8, to: 8, ppd: 4.5, color: '#4E9CCBE6', quiet: '#86A6B8A3' },
    { id: 'rivers-9', from: 9, to: 99, ppd: 0, color: '#4E9CCB', quiet: '#86A6B8BF' }
  ];
  BANDS.forEach(function (b) {
    b.edges = edges.filter(function (e) { return e.order >= b.from && e.order <= b.to; }).map(function (e) { return e.i; });
  });
  var NETWORK_BUDGET = 40000, GATHER_BUDGET = 20000;   // vertices (the stage holds 100,000 in all)
  var REACH = 24;                                      // a touch finds a line this many points away (as the study)

  var cam = terra.camera || { lon: -60, lat: -18, zoom: 1.9, cx: 195, cy: 400, radius: 330, width: 390, height: 800 };
  var perZoom = cam.radius / Math.max(0.1, cam.zoom);  // this screen's points a radian at zoom 1
  function zoomAt(ppd) { return ppd / (perZoom * D); }

  // Every layer once, in drawing order (the first set fixes it), its options kept for each set; a
  // layer with a reading density takes its minZoom from this screen.
  var OPTIONS = {}, FEATURES = {}, PPD = {};
  function layer(id, options, ppd) {
    OPTIONS[id] = options;
    FEATURES[id] = [];
    if (ppd != null) { PPD[id] = ppd; options.minZoom = zoomAt(ppd); }
    terra.geo.set(id, [], options);
  }
  function put(id, features) { FEATURES[id] = features; terra.geo.set(id, features, OPTIONS[id]); }
  /** When the screen's size changes (a turn, a split view), each layer's zoom for its density does. */
  function rescale(c) {
    var p = c.radius / Math.max(0.1, c.zoom);
    if (Math.abs(p - perZoom) < perZoom * 0.02) return false;
    perZoom = p;
    Object.keys(PPD).forEach(function (id) { OPTIONS[id].minZoom = zoomAt(PPD[id]); put(id, FEATURES[id]); });
    return true;
  }
  BANDS.forEach(function (b) { layer(b.id, { style: 'border', color: b.color, touchable: true }, b.ppd); });
  layer('brushed', { style: 'border', color: INK.brushed, touchable: true });
  layer('gathered', { style: 'border', color: INK.gathered, touchable: true });
  layer('trace', { style: 'border', color: INK.trace, touchable: true });
  layer('ending', { style: 'dot', color: INK.trace });
  layer('touched', { style: 'ring', color: INK.trace });
  layer('names-great', { style: 'label', font: 'italic', size: 13, color: INK.water, touchable: true }, 5);
  layer('names-rivers', { style: 'label', font: 'italic', size: 11.5, color: INK.water, touchable: true }, 8.5);
  layer('names-towns', { style: 'label', font: 'italic', size: 10, color: INK.town }, 8);
  layer('names-towns-close', { style: 'label', font: 'italic', size: 10, color: INK.town }, 11);
  var RIVERS = { brushed: true, gathered: true, trace: true };
  BANDS.forEach(function (b) { RIVERS[b.id] = true; });

  // ---- the named rivers and towns (the study's orientation aids) ----

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
  // towns as the globe letters its capitals ("· Lima"), so they read as the same kind of name
  ['names-towns', 'names-towns-close'].forEach(function (id) {
    put(id, places.cities.filter(function (c) { return c.ppd === PPD[id]; })
      .map(function (c) { return label(c.at[0], c.at[1], '· ' + c.name); }));
  });

  // ---- the camera: which rivers, how finely, over how much of the globe ----

  var drawn = { lod: 0, cover: undefined, shown: '', bands: {} };

  /** The coarsest drawing level whose tolerance is within a point at the centre of the disc. */
  function lodFor(c) {
    var tolerance = 1 / Math.max(1e-6, c.radius * D);
    for (var i = 0; i < LEVELS.length; i++) if (LEVELS[i] <= tolerance) return i + 1;
    return LEVELS.length;
  }
  /** The screen's extent on the globe [w, s, e, n], or null when the globe's edge is in view. */
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
  /** The smallest order drawn at a zoom: smaller rivers can't be touched there either. */
  function shownOrder(zoom) {
    var m = 99;
    BANDS.forEach(function (b) { if (zoom >= OPTIONS[b.id].minZoom) m = Math.min(m, b.from); });
    return m;
  }

  /** Lines for these stretches at a level, within a box, as the globe takes them, and their vertices. */
  function lines(ids, lod, box, minOrder) {
    var geoms = [], count = 0;
    for (var k = 0; k < ids.length; k++) {
      var e = edges[ids[k]];
      if (e.order < (minOrder || 1) || !meets(e.bounds, box)) continue;
      var l = net.line(e, lod);
      geoms.push({ type: 'LineString', coordinates: l });
      count += l.length;
    }
    return { geoms: geoms, count: count };
  }

  /** Sets the network's layers for the camera, when it shows something they don't hold. */
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
    highlights();
  }

  // ---- the water touched ----

  var current = null;      // { route, gather }: the way down from the touch, and what gathers above it
  var brushed = [];        // earlier ways down, for a while: [{ route, at }]
  var lastHit = -1, lastSample = 0, lastRing = 0, brushing = false, fadeTimer = 0;

  function point(lon, lat) { return { type: 'Point', coordinates: [lon, lat] }; }
  function lastVertex(e) { return e.c[e.c.length - 1]; }

  var quiet = false;
  function quieten(on) {
    if (on === quiet) return;
    quiet = on;
    BANDS.forEach(function (b) { OPTIONS[b.id].color = on ? b.quiet : b.color; put(b.id, FEATURES[b.id]); });
  }

  /** The highlighted water, at the network's level and within its cover. */
  function highlights() {
    var lod = drawn.lod || lodFor(cam), cover = drawn.cover || null;
    quieten(!!current || brushed.length > 0);
    if (current) {
      var route = current.route, hit = route.hit, end = lastVertex(edges[route.end]);
      put('trace', [{ type: 'LineString', coordinates: net.routeLine(route, lod) }]);
      put('touched', [point(hit.lon, hit.lat)]);
      put('ending', [point(end[0], end[1])]);
      if (current.gather) {
        var g = lines(current.gather.ids, lod, cover, shownOrder(cam.zoom));
        if (g.count > GATHER_BUDGET) g = lines(current.gather.ids, 1, cover, shownOrder(cam.zoom));
        g.geoms.push({ type: 'LineString', coordinates: net.line(edges[hit.id], lod, null, hit) });
        put('gathered', g.geoms);
      } else {
        put('gathered', []);
      }
    } else {
      ['trace', 'touched', 'ending', 'gathered'].forEach(function (id) { put(id, []); });
    }
    // the earlier ways down: each partial first stretch, then every stretch below once
    var geoms = [], seen = {};
    brushed.forEach(function (b) {
      var r = b.route;
      geoms.push({ type: 'LineString', coordinates: net.line(edges[r.ids[0]], lod, r.hit, null) });
      for (var k = 1; k < r.ids.length; k++) {
        if (seen[r.ids[k]]) break;     // the rest of this way down is drawn already
        seen[r.ids[k]] = true;
        geoms.push({ type: 'LineString', coordinates: net.line(edges[r.ids[k]], lod) });
      }
    });
    put('brushed', geoms);
  }

  /** Degrees along a line of [lon, lat] (the glint's measure, as the study's map units were). */
  function degrees(line) {
    var d = 0;
    for (var k = 1; k < line.length; k++) d += WaterNet.span(line[k - 1][0], line[k - 1][1], line[k][0], line[k][1]);
    return d;
  }

  /** A bead of light running down the way traced; its speed is invented (the study's: the whole way
      in at most nine seconds). Not under Reduce Motion: it would only stand still. */
  function glint() {
    if (!current || terra.settings.reduceMotion) { terra.sprites.remove('glint'); return; }
    var path = net.routeLine(current.route, 2);
    terra.sprites.set('glint', {
      path: { coordinates: path, seconds: Math.max(0.6, Math.min(9, degrees(path) / 0.35)), loop: true },
      looks: [{ image: 'glint.png', size: [14, 6], face: true }],
      label: 'Which way this water goes (its speed is invented)'
    });
  }

  // While nothing is touched, ink drifts down three great rivers from their named places, slowly,
  // as the study's did: an invitation.
  var drifts = anchors.slice(0, 3).map(function (a) { return net.routeLine(net.downstream(a.hit), 2); });
  var drifting = false;
  function ambient() {
    var on = !current && !brushed.length && !terra.settings.reduceMotion;
    if (on === drifting) return;
    drifting = on;
    drifts.forEach(function (path, i) {
      if (!on) { terra.sprites.remove('drift-' + i); return; }
      terra.sprites.set('drift-' + i, {
        path: { coordinates: path, seconds: Math.min(34, degrees(path) / 0.25), loop: true },
        looks: [{ image: 'glint.png', size: [14, 6], face: true }],
        label: 'Water going down the ' + anchors[i].label + ', drawn (its speed is invented)'
      });
    });
  }

  // ---- the words at the foot: what this is first, then why it is interesting ----

  function commas(n) { return String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ','); }
  function distance(km) { return km < 10 ? 'less than 10 km' : 'about ' + commas(Math.round(km / 10) * 10) + ' km'; }
  function area(a) {
    if (a >= 1e6) return (Math.round(a / 1e5) / 10) + ' million km²';
    if (a >= 1000) { var p = Math.pow(10, Math.floor(Math.log(a) / Math.LN10) - 2); return commas(Math.round(a / p) * p) + ' km²'; }
    return commas(Math.max(10, Math.round(a / 10) * 10)) + ' km²';
  }
  /** "the Amazon near Manaus", "the Negro": a named place on the river, as the study names it. */
  function phrase(a) {
    var near = /·\s*near\s+(.+)$/.exec(a.name);
    return 'the ' + a.label + (near ? ' near ' + near[1] : '');
  }
  function list(words) {
    return words.length < 2 ? words.join('') : words.slice(0, -1).join(', ') + ' and ' + words[words.length - 1];
  }
  /** The named places the water passes below the touch, in the order it reaches them. */
  function passes(route) {
    var found = [];
    anchors.forEach(function (a) {
      var k = route.ids.indexOf(a.hit.id);
      if (k < 0 || (k === 0 && net.along(a.hit) <= net.along(route.hit))) return;
      if (found.some(function (f) { return f.a.label === a.label && f.a.system === a.system; })) return;
      found.push({ a: a, k: k + net.along(a.hit) / 1e6 });
    });
    return found.sort(function (x, y) { return x.k - y.k; }).slice(0, 3).map(function (f) { return phrase(f.a); });
  }

  function describe(c) {
    var route = c.route, e = edges[route.hit.id], out = edges[route.end], system = SYSTEMS[out.i];
    var past = passes(route);
    var title = system ? 'Toward the ' + system + ' system' : 'A river finding its ending';
    var text = 'From here its water runs ' + distance(route.km) + ' along mapped channels to ' +
      (out.inland ? 'an inland ending' : 'the coast') + (past.length ? ', past ' + list(past) : '') + '. ';
    if (c.gather && !c.gather.ids.length) {
      text += 'About ' + area(e.area) + ' of land drains into this stretch; above it, no tributary is large enough to be mapped here.';
    } else {
      text += 'About ' + area(e.area) + ' of land drains into this stretch' + (c.gather ? ', its mapped tributaries lit above it.' : '.');
    }
    if (out.i !== e.i && text.length < 190) {
      text += system ? ' The whole ' + system + ' system, as mapped, drains ' + area(out.area) + '.'
                     : ' Its whole basin, as mapped, covers ' + area(out.area) + '.';
    }
    return { title: title, text: text };
  }

  var said = '';
  function say(c) {
    var k = c.title + '|' + c.text;
    if (k !== said) { said = k; terra.caption.set(c); }
  }

  // ---- touch ----

  /** The nearest line drawn within reach of a touch (a degree is fewer points towards the globe's
      edge, so when the globe says a river was touched and none is found, look a little further). */
  function near(t) {
    if (t.lon == null || t.lat == null) return null;
    var reach = REACH / Math.max(1e-6, cam.radius * D), min = shownOrder(cam.zoom);
    var hit = net.nearest(t.lon, t.lat, reach, min);
    if (!hit && t.target && RIVERS[t.target.layer]) hit = net.nearest(t.lon, t.lat, reach * 4, min);
    return hit;
  }

  /** Follow the water from a hit (and gather what is above it, unless a finger is still brushing). */
  function follow(hit, gather) {
    if (current) {
      brushed.push({ route: current.route, at: Date.now() });
      if (brushed.length > 12) brushed.shift();
    }
    current = { route: net.downstream(hit), gather: gather ? net.upstream(hit) : null };
    highlights();
    glint();
    ambient();
    fade();
    say(describe(current));
  }

  function clear() {
    current = null;
    brushed = [];
    lastHit = -1;
    highlights();
    glint();
    ambient();
    say(IDLE);
  }

  // Earlier ways down go after 24 seconds, as the study's traces did.
  function fade() {
    if (fadeTimer || !brushed.length) return;
    fadeTimer = setTimeout(function () {
      fadeTimer = 0;
      var now = Date.now(), before = brushed.length;
      brushed = brushed.filter(function (b) { return now - b.at < 24000; });
      if (brushed.length !== before) { highlights(); ambient(); }
      fade();
    }, 3000);
  }

  /** A named river, chosen by its name: follow it from its named place, and show all of it. */
  function choose(a) {
    follow(a.hit, true);
    var b = [Infinity, Infinity, -Infinity, -Infinity];
    current.route.ids.concat(current.gather.ids).forEach(function (i) {
      var r = edges[i].bounds;
      b = [Math.min(b[0], r[0]), Math.min(b[1], r[1]), Math.max(b[2], r[2]), Math.max(b[3], r[3])];
    });
    var lat = (b[1] + b[3]) / 2, size = Math.max((b[2] - b[0]) * Math.cos(lat * D), b[3] - b[1], 2) * D;
    var zoom = Math.max(1.4, Math.min(5, 0.6 * Math.min(cam.width, cam.height) / (size * perZoom)));
    terra.flyTo((b[0] + b[2]) / 2, lat, zoom, terra.settings.reduceMotion ? 0 : 1.4);
  }

  terra.on('touch', function (t) {
    if (t.phase === 'tap' && t.target && NAMED[t.target.layer]) {
      var a = NAMED[t.target.layer][t.target.index];
      if (a) { choose(a); return; }
    }
    if (t.phase === 'ended') {
      // the brush lifted: gather what flows to where it last was
      if (brushing && current && !current.gather) { current.gather = net.upstream(current.route.hit); highlights(); say(describe(current)); }
      brushing = false;
      lastHit = -1;
      return;
    }
    var hit = near(t);
    if (t.phase === 'tap') {
      if (hit) follow(hit, true);
      else if (t.lon != null) clear();
      return;
    }
    if (t.phase === 'began') { lastHit = -1; brushing = false; }
    if (!hit) return;
    var now = Date.now();
    if (now - lastRing >= 30) { lastRing = now; put('touched', [point(hit.lon, hit.lat)]); }
    // as the study sampled a brush: a new way down only for a new stretch, at most every 55 ms
    if (hit.id === lastHit || now - lastSample < 55) return;
    lastSample = now;
    lastHit = hit.id;
    brushing = true;
    follow(hit, false);
  });

  // ---- the camera, settings, the panel ----

  var settle = 0, lastBuild = 0;
  terra.on('camera', function (c) {
    if (!c) return;
    cam = c;
    clearTimeout(settle);
    settle = setTimeout(function () { settle = 0; network(cam, false); }, 160);
    // while it moves, only when the view has left what is drawn, and not too often
    if (Date.now() - lastBuild > 400 && !covered(viewBox(c))) network(c, false);
  });
  terra.on('settings', function () { glint(); ambient(); });
  terra.on('open', function () {
    if (about) terra.panel.show(about);
  });
  terra.on('close', function () { clearTimeout(settle); clearTimeout(fadeTimer); });

  terra.globe.set({ still: true });     // a study read closely: the globe doesn't turn by itself
  network(cam, true);
  say(IDLE);
})();
