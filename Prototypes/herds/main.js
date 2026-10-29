/* The herds: the camel family's history as herds of small animals moving on the globe (runtime 5,
   docs/runtime3.md). Every animal is a sprite on a dated way (tools/herds/build.py), so the globe's
   date moves it: the herds walk while the date plays and graze when it stops, divide where their
   lineages part, and go when their line ends. From afar a herd is a small mark; nearer, each animal;
   close, its leader drawn large. The date slows as the reader comes close, so a herd that crosses a
   continent in seconds from afar walks past close up. A touch on an animal goes close to it and says
   what it is; the chip's Open has the whole story and its sources. This script decides; it draws
   nothing itself. Data: data.js (build.py), art.js (tools/herds/draw.cjs), notes.js (the panel). */
(function () {
  'use strict';
  var H = window.HERDS, ART = window.HERD_ART, NOTES = window.HERD_NOTES, D = Math.PI / 180;

  // ----- scale -------------------------------------------------------------------------------------
  // What shows depends on how many points a degree is on this screen (a degree is radius × π/180
  // points, and the globe's radius at zoom 1 is about 176 points on a phone and 489 on the 13-inch
  // iPad), not on the zoom alone, so the herds look the same on either.
  var PPD = { herd: 8, small: 30, close: 62, slow: 12 };
  var MAX_ZOOM = 32;
  var R1 = 0;
  function zoomAt(ppd) { return Math.round(ppd / (R1 * D) * 100) / 100; }
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : 489;
    if (R1 && Math.abs(r1 - R1) / R1 < 0.08) return false;
    R1 = r1;
    return true;
  }

  // ----- the date's pace, as the host keeps it ----------------------------------------------------
  var PACE = H.pace;
  function secondsAt(v) {
    if (v <= PACE[0][0]) return PACE[0][1];
    for (var i = 1; i < PACE.length; i++) {
      if (v <= PACE[i][0]) {
        var a = PACE[i - 1], b = PACE[i];
        return a[1] + (v - a[0]) / (b[0] - a[0]) * (b[1] - a[1]);
      }
    }
    return PACE[PACE.length - 1][1];
  }

  // ----- the animals -------------------------------------------------------------------------------
  var animals = H.animals.map(function (a) {
    var p = a.path;
    return { id: a.id, leads: a.leads, path: p, lineages: a.lineages,
             first: p[0][2], last: p[p.length - 1][2],
             coordinates: { coordinates: p.map(function (q) { return [q[0], q[1]]; }), dates: p.map(function (q) { return q[2]; }) },
             from: secondsAt(p[0][2]) - 2, until: secondsAt(p[p.length - 1][2]) + 2 };
  });
  var byId = {};
  animals.forEach(function (a) { byId[a.id] = a; });

  /** It leads its herd at a date: the one drawn close, and touched. */
  function leads(a, date) {
    var d = Math.min(Math.max(date, a.first), a.last);
    for (var i = 0; i < a.leads.length; i++) if (d >= a.leads[i][0] && d < a.leads[i][1] || d === a.last && d === a.leads[i][1]) return true;
    return false;
  }
  /** The lineage an animal belongs to at a date. */
  function lineageAt(a, date) {
    var lid = a.lineages[0][1];
    for (var i = 1; i < a.lineages.length; i++) if (a.lineages[i][0] <= date) lid = a.lineages[i][1];
    return lid;
  }
  /** What it is at a date: [kind, words]. */
  function kindAt(a, date) {
    var d = Math.min(Math.max(date, a.first), a.last), kinds = H.lineages[lineageAt(a, d)].kinds, k = kinds[0];
    for (var i = 1; i < kinds.length; i++) if (kinds[i][0] <= d) k = kinds[i];
    return [k[1], k[2]];
  }
  /** Where it is at a date (as the globe places it: on the great circle between its two turns). */
  function placeAt(a, date) {
    var p = a.path, n = p.length;
    if (date <= p[0][2]) return [p[0][0], p[0][1]];
    if (date >= p[n - 1][2]) return [p[n - 1][0], p[n - 1][1]];
    for (var i = 1; i < n; i++) {
      if (p[i][2] >= date) {
        var f = p[i][2] > p[i - 1][2] ? (date - p[i - 1][2]) / (p[i][2] - p[i - 1][2]) : 1;
        return Shared.along(p[i - 1], p[i], f);
      }
    }
    return [p[n - 1][0], p[n - 1][1]];
  }
  // ----- how an animal shows ------------------------------------------------------------------------
  function look(set, more, scale) {
    var l = set.frames.length > 1 ? { frames: set.frames } : { image: set.frames[0] };
    l.size = scale ? [Math.round(set.size[0] * scale), Math.round(set.size[1] * scale)] : set.size;
    l.face = true;
    for (var k in more) l[k] = more[k];
    return l;
  }
  /** Its looks by zoom: the herd's mark from afar (the leader's), each animal nearer, the leader drawn
      large close in (the others give way to it there). Grazing, the frames turn slowly. */
  function looksFor(kind, leader, grazing) {
    // (a stride is about a second: the close drawing walks in eight frames, the smaller ones in four)
    var art = ART[kind], pose = grazing ? 'graze' : 'walk', fps = grazing ? 1.4 : 5;
    var zHerd = zoomAt(PPD.herd), zSmall = zoomAt(PPD.small), zClose = zoomAt(PPD.close), looks = [];
    if (leader) looks.push(look(art.herd[pose], { maxZoom: zHerd, fps: fps }));
    looks.push(look(art.tiny[pose], { minZoom: zHerd, maxZoom: zSmall, fps: fps }));
    looks.push(look(art.small[pose], { minZoom: zSmall, maxZoom: zClose, fps: fps }));
    // (drawn for the iPad; a phone's narrower screen takes it smaller)
    if (leader) looks.push(look(art.close[pose], { minZoom: zClose, fps: grazing ? 1.4 : 6 }, R1 < 300 ? 0.66 : 1));
    return looks;
  }

  var shown = {};          // id -> what it was set with
  var date = H.start, playing = false;

  /** Sets, changes or takes away each animal's sprite for the date: only those near their time on the
      globe have one. Called on every change of the date; sets only what changed. */
  function refresh() {
    var now = secondsAt(date), band = Math.round(R1);
    animals.forEach(function (a) {
      if (now < a.from || now > a.until) {
        if (shown[a.id]) { terra.sprites.remove(a.id); delete shown[a.id]; }
        return;
      }
      var k = kindAt(a, date), grazing = !playing || date >= H.today, leader = leads(a, date);
      var key = k[0] + '|' + grazing + '|' + leader + '|' + band;
      if (shown[a.id] === key) return;
      shown[a.id] = key;
      terra.sprites.set(a.id, {
        path: a.coordinates,
        looks: looksFor(k[0], leader, grazing),
        label: leader ? k[1] : '',
        touchable: leader
      });
    });
    bridge();
  }

  // ----- Beringia, land while the strait was closed -------------------------------------------------
  var bridgeShown = 0, bridgeTimer = null;
  function bridge() {
    var land = H.beringia.land.some(function (r) { return date >= r[0] && date <= r[1]; }) ? 1 : 0;
    if (land === bridgeShown) return;
    bridgeShown = land;
    // a short fade (a few steps), or at once under Reduce Motion
    if (bridgeTimer) clearInterval(bridgeTimer);
    var steps = terra.settings.reduceMotion ? 1 : 6, i = 0;
    bridgeTimer = setInterval(function () {
      i++;
      var o = land ? i / steps : 1 - i / steps;
      if (o <= 0) terra.images.remove('beringia');
      else terra.images.set('beringia', { image: H.beringia.image, bounds: H.beringia.bounds, opacity: o });
      if (i >= steps) { clearInterval(bridgeTimer); bridgeTimer = null; }
    }, 90);
  }

  // ----- the globe ---------------------------------------------------------------------------------
  /** The date slows as the reader comes close, so the herd nearest the middle of the view walks at the
      pace of its own legs: close up, its leader's stride (about 22 points a second as drawn); further
      out, faster (a herd crossing a continent in seconds from afar); from the whole Earth, as written. */
  var speed = 1, lastPaced = 0;
  function dateAt(sec) {
    if (sec <= PACE[0][1]) return PACE[0][0];
    for (var i = 1; i < PACE.length; i++) {
      if (sec <= PACE[i][1]) {
        var a = PACE[i - 1], b = PACE[i];
        return a[0] + (sec - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
      }
    }
    return PACE[PACE.length - 1][0];
  }
  function degrees(p, q) { return Shared.arc(p, q) / D; }
  function pace(c) {
    if (!c || !(c.radius > 0)) return;
    lastPaced = Date.now();
    var ppd = c.radius * D, want = 1;
    if (ppd > PPD.herd) {
      // the herd nearest the middle of the view, and how many degrees it covers in a second of playback
      var now = secondsAt(date), best = null, bestD = Infinity;
      animals.forEach(function (a) {
        if (now < a.from || now > a.until || date < a.first || date > a.last) return;
        var p = placeAt(a, date), s = terra.project(p[0], p[1], c);
        if (!s || !s.visible || s.x < 0 || s.y < 0 || s.x > c.width || s.y > c.height) return;
        var d = Math.hypot(s.x - c.cx, s.y - c.cy);
        if (d < bestD) { bestD = d; best = { a: a, p: p }; }
      });
      if (best) {
        var ahead = placeAt(best.a, dateAt(now + 0.5)), perSecond = degrees(best.p, ahead) * 2 * ppd;   // points a second
        var allowed = (R1 < 300 ? 15 : 22) * Math.sqrt(Math.max(1, PPD.close / ppd)) * Math.max(1, ppd < PPD.small ? PPD.small / ppd : 1);
        if (perSecond > allowed) want = allowed / perSecond;
      }
    }
    want = Math.max(0.01, Math.min(1, want));
    if (Math.abs(want - speed) / speed < 0.08) return;
    speed = want;
    terra.time.speed(speed);
  }

  // ----- what the reader touches ------------------------------------------------------------------
  var caption = { title: 'The camel family', text: '' }, near = null;
  function say(text) {
    if (caption.text === text) return;
    caption.text = text;
    terra.caption.set(caption);
  }

  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || !t.target || !t.target.sprite) return;
    var a = byId[t.target.sprite];
    if (!a) return;
    var c = terra.camera, k = kindAt(a, date), here = placeAt(a, date);
    if (near === a.id && c && c.radius * D >= PPD.close * 0.85) {
      // close already: its story, on demand
      showPanel(lineageAt(a, Math.min(Math.max(date, a.first), a.last)));
      return;
    }
    near = a.id;
    say(k[1]);
    terra.flyTo(here[0], here[1], Math.min(MAX_ZOOM, zoomAt(PPD.close * 1.25)), terra.settings.reduceMotion ? 0 : 1.4);
  });

  terra.on('camera', function (c) {
    if (!c) return;
    if (measure(c)) { shown = {}; refresh(); }
    pace(c);
    // far out again, the words go
    if (c.radius * D < PPD.herd && caption.text) { near = null; say(''); }
  });

  terra.on('time', function (t) {
    var was = playing;
    date = t.value;
    playing = t.playing;
    if (t.by === 'reader' && was !== playing) readerPaused = !playing;
    refresh();
    // (the herd in view may change its pace with the date: looked at again twice a second)
    if (playing && Date.now() - lastPaced > 500) pace(terra.camera);
  });

  var readerPaused = false;
  terra.on('focus', function (f) {
    // back in front: the date may run again, unless the reader stopped it
    if (f.front && !readerPaused && date < H.today) terra.time.play(true);
  });

  // ----- the panel: the story and its sources, on demand ------------------------------------------
  function showPanel(first) {
    var sections = NOTES.sections.slice();
    if (first) {
      var i = -1;
      sections.forEach(function (s, j) { if (s.lineage === first || (s.also || []).indexOf(first) >= 0) i = j; });
      if (i > 0) sections.unshift(sections.splice(i, 1)[0]);
    }
    terra.panel.show({ title: NOTES.title, text: NOTES.text,
                       sections: sections.map(function (s) { return { title: s.title, text: s.text }; }),
                       sources: NOTES.sources });
  }
  terra.on('open', function () { showPanel(null); });

  // ----- the start ---------------------------------------------------------------------------------
  measure(terra.camera);
  terra.caption.set(caption);
  terra.globe.set({ historical: true, still: true, maxZoom: MAX_ZOOM });
  terra.time.show({ unit: 'year', pace: H.pace, value: H.start, ticks: H.ticks, playable: true, playing: true });

  // The first sight: close on one of the first camelids, then out, to see it is one of a small herd
  // on the move. (Not under Reduce Motion: there the reader starts where they are, and the date waits.)
  if (!terra.settings.reduceMotion) {
    var one = animals.filter(function (a) { return a.first === H.start && leads(a, H.start); })[0];
    if (one) {
      var p = placeAt(one, H.start);
      terra.flyTo(p[0], p[1], Math.min(MAX_ZOOM, zoomAt(PPD.close * 1.3)), 2.4);
      setTimeout(function () {
        var q = placeAt(one, date);
        terra.flyTo(q[0], q[1] + 0.6, zoomAt(32), 4);
      }, 5200);
    }
  }
})();
