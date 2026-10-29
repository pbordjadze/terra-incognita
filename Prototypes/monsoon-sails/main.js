/* The monsoon's sails (runtime 4). A year at the foot of the screen; the globe's own season follows it
   (the land greens, the clouds gather). Over the Indian Ocean the wind is drawn as streaks of air:
   from the south-west in summer, from the north-east in winter. Dhows wait in their ports with sails
   furled and go out when the wind blows their way: up the ocean to Arabia and India with the summer
   wind, home to Africa with the winter's. Drag the year and they are where the year puts them; touch a
   port and its ships set sail, if the wind allows. Nothing is written on the screen but the name; what
   is true and what is drawn is in SOURCES.md and in the panel the chip's Open shows. */
(function () {
  'use strict';
  var M = window.MONSOON, D = Math.PI / 180, DAY = 86400000;
  var RATE = 5;                              // days a second while the year plays
  var R1 = 489;                              // the globe's radius at zoom 1 on this screen (the 13-inch iPad's to start)
  function zoomFor(ppd) { return ppd / (R1 * D); }
  var NEAR = 3.2, MID = 1.3;                 // zooms (set from the screen): the dhow itself; its small self; a speck of sail
  var KMPD = 160;                            // a dhow's day's run, in kilometres (a round figure)
  var MIRROR = Math.SQRT1_2;

  // ---- geometry: the host moves a sprite along its way at a steady pace; this is the same reckoning ----
  function unit(p) { var la = p[1] * D, lo = p[0] * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; }
  function arc(a, b) { var u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))); }
  function along(a, b, t) {
    var u = unit(a), v = unit(b), w = arc(a, b);
    if (w < 1e-9) return a.slice();
    var sa = Math.sin((1 - t) * w) / Math.sin(w), sb = Math.sin(t * w) / Math.sin(w);
    var p = [u[0] * sa + v[0] * sb, u[1] * sa + v[1] * sb, u[2] * sa + v[2] * sb];
    return [Math.atan2(p[0], p[2]) / D, Math.asin(Math.max(-1, Math.min(1, p[1]))) / D];
  }
  function lengthOf(c) { var s = 0; for (var i = 0; i < c.length - 1; i++) s += arc(c[i], c[i + 1]); return s; }
  /** The place `f` (0–1) of the way along `c`, and the rest of the way from there. */
  function split(c, f) {
    var total = lengthOf(c), d = Math.max(0, Math.min(1, f)) * total;
    for (var i = 0; i < c.length - 1; i++) {
      var L = arc(c[i], c[i + 1]);
      if (d <= L || i === c.length - 2) {
        var p = along(c[i], c[i + 1], L ? Math.min(1, d / L) : 0);
        return { at: p, rest: [p].concat(c.slice(i + 1)) };
      }
      d -= L;
    }
    return { at: c[c.length - 1], rest: [c[c.length - 1]] };
  }

  // ---- the year ----
  var now = new Date(), YEAR = now.getUTCFullYear();
  function day(m, d) { return Date.UTC(YEAR, m - 1, d) / DAY; }
  var FIRST = day(1, 1), LAST = day(12, 31), DAYS = LAST - FIRST + 1;
  function yday(v) { return ((v - FIRST) % DAYS + DAYS) % DAYS; }           // the day of the year, 0–365
  function monthOf(v) { return new Date((FIRST + yday(v)) * DAY).getUTCMonth(); }  // 0–11
  /** Which way the wind blows: 1 toward the north-east (summer), -1 toward the south-west (winter), with
      how settled it is (the turning months are light). */
  function wind(v) {
    var m = monthOf(v);
    if (m >= 5 && m <= 8) return { dir: 1, strength: 1 };
    if (m >= 10 || m <= 2) return { dir: -1, strength: 1 };
    if (m === 3 || m === 4) return { dir: 1, strength: 0.5 };
    return { dir: -1, strength: 0.5 };                                      // October
  }

  // ---- the dhows: two to a way, each with its year's two voyages ----
  var seed = 7;
  function rnd() { seed = (seed * 9301 + 49297) % 233280; return seed / 233280; }
  var dhows = [];
  M.ways.forEach(function (way, i) {
    var a = M.ports[way.from].at, b = M.ports[way.to].at;
    var up = [a].concat(way.via, [b]), down = up.slice().reverse();
    var days = Math.max(12, Math.round(lengthOf(up) * 6371 / KMPD));
    [0, 1].forEach(function (k) {
      // out with the summer wind (the early sailing in April–May, or the late one in August–September),
      // home with the winter's (November–January)
      var out = k === 0 ? day(4, 8) + Math.round(rnd() * 38) : day(8, 16) + Math.round(rnd() * 32);
      var home = day(11, 12) + Math.round(rnd() * 60);
      if (home + days > LAST + 1) home -= 0; // (a voyage may run into the next year: it wraps)
      dhows.push({ id: 'dhow-' + (i + 1) + '-' + (k + 1), way: way, up: up, down: down, days: days, berth: k,
                   label: 'A dhow of ' + M.ports[way.from].name + ' and ' + M.ports[way.to].name,
                   voyages: [{ dir: 1, dep: yday(out), dur: days }, { dir: -1, dep: yday(home), dur: days }],
                   extra: null, shown: null });
    });
  });

  /** Where a dhow is on day `t` of the year: sailing (which voyage, how far) or in port (which). */
  function reckon(dhow, t) {
    var list = dhow.voyages.slice();
    if (dhow.extra) list.push(dhow.extra);
    for (var i = 0; i < list.length; i++) {
      var v = list[i];
      for (var wrap = -1; wrap <= 1; wrap++) {
        var s = v.dep + wrap * DAYS;
        if (t >= s && t < s + v.dur) return { sailing: true, voyage: v, f: (t - s) / v.dur, key: v.dir + ':' + v.dep };
      }
    }
    // in port: after going up and before coming home it lies at the north-eastern end
    var up = dhow.voyages[0], home = dhow.voyages[1];
    var after = (t - up.dep - up.dur + 2 * DAYS) % DAYS, until = (home.dep - t + 2 * DAYS) % DAYS;
    var atTop = after + until < DAYS + 0.5 && after <= (home.dep - up.dep - up.dur + 2 * DAYS) % DAYS;
    if (dhow.extra) {
      // a voyage the reader sent it on: it lies where that left it until its next planned one
      var e = dhow.extra, since = (t - e.dep - e.dur + 2 * DAYS) % DAYS;
      if (since < DAYS / 2) atTop = e.dir === 1;
    }
    return { sailing: false, port: atTop ? dhow.way.to : dhow.way.from, key: 'port:' + (atTop ? dhow.way.to : dhow.way.from) };
  }

  function looks(sailing) {
    if (sailing) {
      return [{ frames: frames('dhow', 6), fps: 5, size: [44, 36], minZoom: NEAR, face: true },
              { frames: frames('dhow-mid', 6), fps: 5, size: [22, 18], minZoom: MID, maxZoom: NEAR, face: true },
              { image: 'speck.png', size: [9, 9], maxZoom: MID }];
    }
    return [{ frames: frames('dhow-furled', 4), fps: 2, size: [44, 36], minZoom: NEAR },
            { frames: frames('dhow-furled-mid', 4), fps: 2, size: [22, 18], minZoom: MID, maxZoom: NEAR },
            { image: 'speck.png', size: [7, 7], maxZoom: MID }];
  }
  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }

  /** Where a dhow lies in a port: the ships of a way at their own berths, a little apart, so a port
      with several reads as a small fleet. */
  function berth(dhow, portId) {
    var at = M.ports[portId].at, n = M.ways.indexOf(dhow.way);
    return [at[0] + 0.22 * (dhow.berth ? 1 : -1) + 0.06 * (n % 3), at[1] - 0.12 * (n % 2) + 0.07 * dhow.berth];
  }
  var date = null, playing = false;
  /** Puts every dhow where the year has it. While the year plays, a dhow already under way on its path
      is left to the host, which moves it at the year's pace; otherwise it is placed, or set on the
      rest of its way with the seconds the year will take. */
  function place(by) {
    if (date == null) return;
    var t = yday(date);
    dhows.forEach(function (dhow) {
      var r = reckon(dhow, t);
      if (r.sailing) {
        var route = r.voyage.dir === 1 ? dhow.up : dhow.down, here = split(route, r.f);
        if (playing) {
          if (by === 'play' && dhow.shown === r.key + ':path') return;
          var seconds = Math.max(0.5, r.voyage.dur * (1 - r.f) / RATE);
          terra.sprites.set(dhow.id, { path: { coordinates: here.rest, seconds: seconds, loop: false }, looks: looks(true), label: dhow.label + ', under sail', touchable: true });
          dhow.shown = r.key + ':path';
        } else {
          terra.sprites.set(dhow.id, { at: here.at, looks: looks(true), label: dhow.label + ', under sail', touchable: true });
          dhow.shown = r.key + ':at';
        }
      } else {
        if (dhow.shown === r.key) return;
        terra.sprites.set(dhow.id, { at: berth(dhow, r.port), looks: looks(false), label: dhow.label + ', in ' + M.ports[r.port].name, touchable: true });
        dhow.shown = r.key;
      }
    });
  }

  // ---- the wind ----
  var windShown = null;
  function blow() {
    if (date == null) return;
    var w = wind(date), key = w.dir + ':' + w.strength;
    if (key === windShown) return;
    windShown = key;
    var name = w.dir === 1 ? 'wind-ne' : 'wind-sw';
    M.wind.forEach(function (at, i) {
      var every = 3 + (i % 5) * 0.5, show = w.strength === 1 ? 2.8 : 1.6;
      if (w.strength < 1 && i % 2) { terra.sprites.remove('wind-' + i); return; }
      terra.sprites.set('wind-' + i, { at: at, label: 'The wind', looks: [
        { frames: frames(name, 24), fps: 12, size: [80, 80], minZoom: 1.2, every: every, for: show },
        { frames: frames(name, 24), fps: 12, size: [40, 40], maxZoom: 1.2, every: every, for: show }
      ] });
    });
  }

  // ---- the ports, touchable ----
  var portIds = Object.keys(M.ports);
  terra.geo.set('ports', portIds.map(function (id) {
    return { type: 'Feature', geometry: { type: 'Point', coordinates: M.ports[id].at }, properties: { label: M.ports[id].name } };
  }), { style: 'ring', color: '#6B4A2B', touchable: true, minZoom: 1.1 });

  /** A port touched: its ships go out if the wind blows their way; if not, they shake their furled sails. */
  function sail(portId) {
    if (date == null) return;
    var t = yday(date), w = wind(date), went = 0;
    dhows.forEach(function (dhow) {
      var r = reckon(dhow, t);
      if (r.sailing || r.port !== portId) return;
      var dir = portId === dhow.way.from ? 1 : -1;
      if (w.dir === dir) {
        dhow.extra = { dir: dir, dep: t, dur: dhow.days };
        dhow.shown = null;
        went++;
      } else {
        // not this wind: a shake of the furled sail
        terra.sprites.set(dhow.id, { at: berth(dhow, portId), looks: [
          { frames: frames('dhow-furled', 4), fps: 9, size: [44, 36], minZoom: NEAR },
          { frames: frames('dhow-furled-mid', 4), fps: 9, size: [22, 18], minZoom: MID, maxZoom: NEAR },
          { image: 'speck.png', size: [7, 7], maxZoom: MID }], label: dhow.label + ', held in port by the wind', touchable: true });
        dhow.shown = null;
        setTimeout(function () { place('touch'); }, 1300);
      }
    });
    if (went) place('touch');
  }

  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || !t.target) return;
    if (t.target.layer === 'ports') { sail(portIds[t.target.index]); return; }
    if (t.target.sprite && /^dhow-/.test(t.target.sprite)) {
      // closer to the ship, where the reader is
      var c = terra.camera;
      if (c && t.lon != null) terra.flyTo(t.lon, t.lat, Math.max(c.zoom, NEAR + 0.3), 1.1);
    }
  });

  terra.on('time', function (e) {
    date = e.value; playing = e.playing;
    place(e.by);
    blow();
  });
  /** The screen's size sets the zooms at which the ships are drawn large; a change redraws them. */
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR !== 3.2) return false;
    R1 = r1; NEAR = zoomFor(27); MID = zoomFor(11);
    return true;
  }
  terra.on('camera', function (c) {
    if (!measure(c)) return;
    dhows.forEach(function (d) { d.shown = null; });
    windShown = null;
    place('camera'); blow();
  });
  terra.on('focus', function (f) { if (f.front) terra.time.play(true); });

  terra.on('open', function () {
    terra.panel.show({
      title: 'The monsoon\'s sails',
      wikipedia: 'Indian Ocean trade',
      text: 'For two thousand years and more, ships crossed the Indian Ocean on the monsoon: the wind blows from the south-west through the summer and from the north-east through the winter, so a dhow could run before it one way and, after waiting for the wind to turn, before it the other way home. The dhows here wait in port with their sails furled and leave when the wind is theirs: up the ocean to Arabia and India from April and again from August, home to Africa from November. The year at the foot of the screen is also the globe\'s: the land greens and the clouds gather as the summer monsoon comes.',
      sections: [
        { title: 'What is drawn', wikipedia: 'Dhow', text: 'The ports are old ones of the trade: Kilwa, Zanzibar, Mombasa and Mogadishu on the African coast; Aden, Muscat and Sur in Arabia; Siraf in the Gulf; Mandvi in Gujarat and Calicut on the Malabar coast. The ways between them are drawn, as are the ships, their sailing dates (within the seasons) and their day\'s run of about 160 kilometres. The wind is drawn as streaks of air where the seasonal winds blow, not from a weather record.' },
        { title: 'Touching', text: 'Touch a port and its ships set sail, if the wind blows their way; if it does not, they shake their furled sails and stay. Touch a ship to come closer to it.' }
      ],
      sources: [
        { title: 'The Periplus of the Erythraean Sea (first century CE), tr. Lionel Casson, Princeton, 1989', note: 'The earliest sailing directions for the monsoon trade, with its ports and seasons.' },
        { title: 'Alan Villiers, Sons of Sinbad, 1940', note: 'A year under sail in the dhows of Kuwait and Aden, out to Zanzibar on the winter monsoon and home on the summer\'s.' },
        { title: 'India Meteorological Department: the south-west monsoon\'s onset (early June over Kerala) and withdrawal (October), and the north-east monsoon from late October', url: 'https://mausam.imd.gov.in/', note: 'The seasons the drawn wind follows.' }
      ]
    });
  });

  // ---- start ----
  terra.globe.set({ still: true, maxZoom: 12 });
  measure(terra.camera);
  terra.caption.set({ title: 'The monsoon\'s sails', text: '', wikipedia: 'Monsoon' });
  var today = Math.floor(Date.now() / DAY);
  terra.time.show({ unit: 'day', min: FIRST, max: LAST, value: Math.max(FIRST, Math.min(LAST, today)),
                    ticks: [day(4, 8), day(6, 1), day(8, 16), day(10, 20), day(11, 12)], playable: true, rate: RATE, playing: true, carry: false });
})();
