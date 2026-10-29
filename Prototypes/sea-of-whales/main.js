/* Whales in the desert (runtime 4). Nothing is explained: the bones lie in the sand at Wadi Al-Hitan;
   the sea comes back in from the deep, the bones rise and swim; then the water goes out from the
   shore and each whale is left where it was, on the sand, and dries into bones. Touch the sand to
   bring the sea back, touch the dry land to send it away, touch a whale and it dives. The globe
   draws everything: four pictures of the sea (deep to shore, faded in turn), sprites for the whales,
   symbols for the bones. The script keeps its own reckoning of where each whale is along its way.
   What is true and what is drawn: SOURCES.md, and the panel the chip's Open shows. */
(function () {
  'use strict';
  var W = window.WHALES, D = Math.PI / 180;
  var FLOOD = 5.5, EBB = 10, SEA = 70, DESERT = 28;      // seconds: the sea coming, going, staying, gone
  var BANDS = 4, TICK = 50;                              // the pictures, and the fade's step (ms)
  var BLOW = { frames: ['blow-1.png', 'blow-2.png', 'blow-3.png', 'blow-4.png'] };
  var KINDS = {
    basilosaurus: { swim: frames('basilosaurus', 8), mid: frames('basilosaurus-mid', 8), dive: frames('dive', 6),
                    stranded: frames('basilosaurus-stranded', 3), size: [120, 36], midSize: [48, 15], bones: 'bones-big' },
    dorudon: { swim: frames('dorudon', 8), mid: frames('dorudon-mid', 8), dive: frames('dive-small', 6),
               stranded: frames('dorudon-stranded', 3), size: [56, 20], midSize: [24, 9], bones: 'bones-small' }
  };
  var R1 = 489;                                          // the globe's radius at zoom 1 on this screen (the 13-inch iPad's to start)
  function zoomFor(ppd) { return ppd / (R1 * D); }       // the zoom at which a degree is so many points across
  var NEAR = 6.5, MID = 3;                               // zooms (set from the screen): the whale itself; its small self; a blow from afar

  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }
  function point(lon, lat, props) { return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat] }, properties: props || {} }; }

  // ---- where a whale is: the host moves a sprite along its way at a steady pace; this is the same reckoning ----
  function unit(p) { var la = p[1] * D, lo = p[0] * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; }
  function arc(a, b) { var u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))); }
  function along(a, b, t) {
    var u = unit(a), v = unit(b), w = arc(a, b);
    if (w < 1e-9) return a.slice();
    var sa = Math.sin((1 - t) * w) / Math.sin(w), sb = Math.sin(t * w) / Math.sin(w);
    var p = [u[0] * sa + v[0] * sb, u[1] * sa + v[1] * sb, u[2] * sa + v[2] * sb];
    return [Math.atan2(p[0], p[2]) / D, Math.asin(Math.max(-1, Math.min(1, p[1]))) / D];
  }
  /** The place along a way (coordinates, seconds, loop 'back') `elapsed` seconds after it was set. */
  function placeOn(way, elapsed) {
    var c = way.coordinates, lengths = [], total = 0, i;
    for (i = 0; i < c.length - 1; i++) { lengths.push(arc(c[i], c[i + 1])); total += lengths[i]; }
    if (!total) return c[0].slice();
    var f = (elapsed / way.seconds) % 2;
    if (f > 1) f = 2 - f;
    var d = f * total;
    for (i = 0; i < lengths.length; i++) {
      if (d <= lengths[i] || i === lengths.length - 1) return along(c[i], c[i + 1], lengths[i] ? Math.min(1, d / lengths[i]) : 0);
      d -= lengths[i];
    }
    return c[c.length - 1].slice();
  }
  /** The drawn shore's latitude at a longitude (the sea lies north of it). */
  function shoreLat(lon) {
    var s = W.shore;
    if (lon <= s[0][0]) return s[0][1];
    for (var i = 0; i < s.length - 1; i++) {
      if (lon <= s[i + 1][0]) { var t = (lon - s[i][0]) / (s[i + 1][0] - s[i][0]); return s[i][1] + (s[i + 1][1] - s[i][1]) * t; }
    }
    return s[s.length - 1][1];
  }
  function inSea(lon, lat) {
    var b = W.sea.bounds;
    return lon >= b[0] && lon <= b[2] && lat <= b[3] && lat > shoreLat(lon);
  }
  function offShore(p) { return p[1] - shoreLat(p[0]); }   // degrees north of the shore

  // ---- the whales ----
  var whales = W.whales.map(function (w, i) {
    var k = KINDS[w.kind];
    return { id: w.id, kind: w.kind, label: w.label, loop: w.path, seconds: w.seconds, k: k,
             // where it rests when the sea is gone (the first four lie where the bones are found)
             rest: W.bones[i] ? W.bones[i].at.slice() : null, way: null, since: 0, diving: 0 };
  });
  function looksFor(k, which) {
    if (which === 'swim') {
      return [{ frames: k.swim, fps: 7, size: k.size, minZoom: NEAR, face: true },
              { frames: k.mid, fps: 7, size: k.midSize, minZoom: MID, maxZoom: NEAR, face: true },
              { frames: BLOW.frames, fps: 3, size: [16, 16], maxZoom: MID, every: 9 + Math.random() * 4, for: 1.3 }];
    }
    if (which === 'dive') {
      return [{ frames: k.dive, fps: 4, size: k.size, minZoom: NEAR, face: true },
              { frames: k.dive, fps: 4, size: k.midSize, minZoom: MID, maxZoom: NEAR, face: true }];
    }
    // stranded: one still picture of the three, which the ebb steps through
    return [{ image: k.stranded[which], size: k.size, minZoom: NEAR },
            { image: k.stranded[which], size: k.midSize, minZoom: MID, maxZoom: NEAR }];
  }
  function where(w) { return w.way ? placeOn(w.way, (Date.now() - w.since) / 1000) : (w.rest || w.loop[0]); }
  /** The whale's way: from where it rests out onto its loop and back, so a rising whale swims off from its bones. */
  function wayFrom(w, from) {
    var c = w.loop, best = 0, d = Infinity;
    for (var i = 0; i < c.length; i++) { var a = arc(from, c[i]); if (a < d) { d = a; best = i; } }
    var out = [from];
    for (i = 0; i <= c.length; i++) out.push(c[(best + i) % c.length]);
    return { coordinates: out, seconds: w.seconds, loop: 'back' };
  }
  function swim(w, from) {
    w.way = wayFrom(w, from); w.since = Date.now();
    terra.sprites.set(w.id, { path: w.way, looks: looksFor(w.k, 'swim'), label: w.label, touchable: true });
  }
  function strand(w, at, step) {
    w.way = null; w.rest = at;
    terra.sprites.set(w.id, { at: at, looks: looksFor(w.k, step), label: w.label + ', stranded', touchable: false });
  }

  // ---- the bones, drawn by the globe as symbols ----
  // (the bones at the size the whale was drawn: the symbols' layers by zoom, as a sprite's looks)
  function drawBones() {
    var big = [], small = [], far = [];
    whales.forEach(function (w) {
      if (!w.rest || w.way) return;
      (w.kind === 'basilosaurus' ? big : small).push(point(w.rest[0], w.rest[1], { label: w.label + ', bones' }));
    });
    far.push(point(W.valley[0], W.valley[1]));
    terra.geo.set('bones-big', big, { style: 'symbol', image: 'bones.png', width: 112, height: 38, minZoom: NEAR });
    terra.geo.set('bones-big-mid', big, { style: 'symbol', image: 'bones.png', width: 46, height: 16, minZoom: MID, maxZoom: NEAR });
    terra.geo.set('bones-small', small, { style: 'symbol', image: 'bones-2.png', width: 52, height: 19, minZoom: NEAR });
    terra.geo.set('bones-small-mid', small, { style: 'symbol', image: 'bones-2.png', width: 23, height: 8, minZoom: MID, maxZoom: NEAR });
    terra.geo.set('bones-far', (big.length || small.length) ? far : [], { style: 'symbol', image: 'doorway.png', width: 34, height: 13, maxZoom: MID });
  }

  // ---- the sea: four pictures faded in from the deep, out from the shore ----
  var level = [0, 0, 0, 0], fading = 0;
  function paint() {
    for (var k = 0; k < BANDS; k++) {
      terra.images.set('sea-' + (k + 1), { image: 'tethys-' + (k + 1) + '.png', bounds: W.sea.bounds, opacity: level[k], minZoom: 0 });
    }
  }
  /** Fades the bands over `seconds`, in order (0 the deep first; 3 the shore first), to `to`, then `done`. */
  function fade(to, seconds, fromShore, done) {
    clearInterval(fading);
    var start = Date.now(), from = level.slice(), stagger = seconds * 0.55 / (BANDS - 1), rise = seconds - stagger * (BANDS - 1);
    fading = setInterval(function () {
      var t = (Date.now() - start) / 1000, all = true;
      for (var k = 0; k < BANDS; k++) {
        var order = fromShore ? BANDS - 1 - k : k;
        var f = Math.max(0, Math.min(1, (t - order * stagger) / rise));
        f = f * f * (3 - 2 * f);
        level[k] = from[k] + (to - from[k]) * f;
        if (f < 1) all = false;
      }
      paint();
      if (all) { clearInterval(fading); fading = 0; if (done) done(); }
    }, TICK);
  }

  // ---- the cycle ----
  var state = 'desert', timers = [];
  function later(fn, ms) { timers.push(setTimeout(fn, ms)); }
  function clearTimers() { timers.forEach(clearTimeout); timers = []; }

  function flood() {
    if (state !== 'desert') return;
    state = 'flooding';
    clearTimers();
    fade(1, FLOOD, false, function () { state = 'sea'; later(ebb, SEA * 1000); });
    // the bones rise where they lie, once the water reaches the shore; the others come in from the deep
    whales.forEach(function (w, i) {
      if (w.rest) {
        later(function () { terra.sprites.set(w.id, { at: w.rest, looks: looksFor(w.k, 2), label: w.label, touchable: false }); }, FLOOD * 500 + i * 300);
        later(function () { terra.sprites.set(w.id, { at: w.rest, looks: looksFor(w.k, 0), label: w.label, touchable: false }); }, FLOOD * 700 + i * 300);
        later(function () { var from = w.rest; w.rest = null; swim(w, from); drawBones(); }, FLOOD * 900 + i * 300);
      } else {
        later(function () { swim(w, w.loop[0]); }, FLOOD * 300 + i * 400);
      }
    });
  }
  function ebb() {
    if (state !== 'sea') return;
    state = 'ebbing';
    clearTimers();
    fade(0, EBB, true, function () { state = 'desert'; later(flood, DESERT * 1000); });
    // each whale is left where it is as the water under it goes: the ones nearest the shore first
    var order = whales.map(function (w) { return { w: w, at: where(w) }; }).sort(function (a, b) { return offShore(a.at) - offShore(b.at); });
    order.forEach(function (o, i) {
      var when = EBB * 1000 * (0.12 + 0.7 * i / Math.max(1, order.length - 1));
      later(function () { strand(o.w, o.at, 0); }, when);
      later(function () { strand(o.w, o.at, 1); }, when + 3500);
      later(function () { strand(o.w, o.at, 2); }, when + 7000);
      later(function () { terra.sprites.remove(o.w.id); drawBones(); }, when + 11000);
    });
  }

  // ---- touch ----
  var blows = 0;
  terra.on('touch', function (t) {
    if (t.phase !== 'tap') return;
    if (t.target && t.target.sprite) {
      var w = whales.filter(function (x) { return x.id === t.target.sprite; })[0];
      if (w && w.way && !w.diving) {
        w.diving = 1;
        terra.sprites.set(w.id, { path: w.way, looks: looksFor(w.k, 'dive'), label: w.label + ', diving', touchable: true });
        later(function () { w.diving = 0; if (w.way) terra.sprites.set(w.id, { path: w.way, looks: looksFor(w.k, 'swim'), label: w.label, touchable: true }); }, 5000);
      }
      return;
    }
    if (t.lon == null) return;
    if (state === 'sea' && inSea(t.lon, t.lat)) {
      // the water answers: a breath at the touch, as a whale would surface there
      var id = 'blow-' + (++blows);
      terra.sprites.set(id, { at: [t.lon, t.lat], looks: [{ frames: BLOW.frames, fps: 3, size: [18, 18] }], label: 'A whale surfacing' });
      later(function () { terra.sprites.remove(id); }, 1400);
      return;
    }
    if (state === 'desert') flood();
    else if (state === 'sea') ebb();
  });

  terra.on('open', function () {
    terra.panel.show({
      title: 'Whales in the desert',
      text: 'Wadi Al-Hitan, the Valley of the Whales, in Egypt\'s Western Desert, holds hundreds of skeletons of the earliest whales, from about 37 million years ago, when this was the floor of a shallow bay of the Tethys Sea. The long one is Basilosaurus, up to eighteen metres; the smaller one Dorudon, about five. Both still had small hind legs.',
      sections: [
        { title: 'What is drawn', text: 'The sea that comes and goes is an illustration: a generalised late Eocene shore across northern Egypt and Sinai, drawn only over today\'s land, so the Mediterranean and the Red Sea stay as they are. The whales are drawings after the skeletons\' proportions, not reconstructions of colour or skin; their ways through the water are invented. The bones at the valley lie where the record places them; where a whale strands is where it happened to be.' },
        { title: 'Why the whales are in the desert', text: 'The whales did not strand: the sea withdrew over millions of years as the Tethys closed and the land rose, and the bay\'s floor, with its dead, became desert. The withdrawal here is a picture of that, in seconds.' }
      ],
      sources: [
        { title: 'UNESCO World Heritage Centre: Wadi Al-Hitan (Whale Valley), inscribed 2005', url: 'https://whc.unesco.org/en/list/1186/', note: 'The valley, its age (late Eocene), the whales named, and the point this pack places the bones at.' },
        { title: 'Gingerich, P. D., Smith, B. H., Simons, E. L. (1990). Hind limbs of Eocene Basilosaurus: evidence of feet in whales. Science 249: 154–157.', url: 'https://doi.org/10.1126/science.249.4965.154', note: 'The small hind limbs.' },
        { title: 'Uhen, M. D. (2004). Form, function, and anatomy of Dorudon atrox. Papers on Paleontology 34, University of Michigan.', note: 'Dorudon\'s size and proportions.' },
        { title: 'Natural Earth, 1:10m land (public domain)', url: 'https://www.naturalearthdata.com/', note: 'Only to keep the drawn sea off today\'s sea.' }
      ]
    });
  });

  terra.on('close', function () { clearTimers(); clearInterval(fading); });

  /** The screen's size sets the zooms at which the whales are drawn large; a change redraws them. */
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR !== 6.5) return false;
    R1 = r1; NEAR = zoomFor(55); MID = zoomFor(25);
    return true;
  }
  terra.on('camera', function (c) {
    if (!measure(c)) return;
    drawBones();
    whales.forEach(function (w) { if (w.way) terra.sprites.set(w.id, { path: w.way, looks: looksFor(w.k, w.diving ? 'dive' : 'swim'), label: w.label, touchable: true }); });
  });

  // ---- start: the desert, with its bones; then the sea comes ----
  terra.globe.set({ historical: true, still: true, maxZoom: 24 });
  measure(terra.camera);
  terra.caption.set({ title: 'Whales in the desert', text: '' });
  paint();
  drawBones();
  later(flood, 1500);
})();
