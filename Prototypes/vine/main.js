/* The vine's long walk, საქართველო (runtime 4). One vine, eight thousand years long: it rises from a
   qvevri in a Georgian valley where the oldest wine was made, and as the years play it grows across
   the globe along the ways the grapevine really went: down the Zagros, into Anatolia, to the Levant
   and Egypt, over the sea to Crete and Greece, west with the Phoenicians and the Greeks, up the Rhône,
   the Mosel, the Danube, east along the Silk Road to China, and in the last five centuries across the
   oceans to Mexico, the Andes, the Cape, California, Australia and New Zealand. Leaves open along the
   stems and clusters hang where it arrives; a tendril curls at each growing tip. The leaves wear the
   season the reader is in (green, then gold and red at the vintage, then bare but for the last ones).
   Touch a cluster to pick it; touch the qvevri to begin again. Nothing is written on the screen but the
   name; SOURCES.md and the panel the chip's Open shows say what is true and what is drawn. */
(function () {
  'use strict';
  var V = window.VINE, D = Math.PI / 180;
  var R1 = 489;
  function zoomFor(ppd) { return ppd / (R1 * D); }
  var NEAR = 3.5, MID = 1.1;                 // zooms (set from the screen): the leaves large; the leaves small; stems alone
  var RATE = 40;                             // years a second: the walk in a little over three minutes
  var STEM = '#5E3B1E', STEM_YOUNG = '#7A5530';
  var FIRST = V.origin.year, LAST = new Date().getUTCFullYear();

  // ---- the season the reader is in ----
  var month = new Date().getMonth();
  var season = month >= 8 && month <= 10 ? 'autumn' : month >= 11 || month <= 2 ? 'winter' : month <= 5 ? 'spring' : 'summer';
  var LEAF = season === 'spring' || season === 'summer' ? 'leaf-green' : 'leaf-autumn';
  var GRAPES = season === 'summer' ? 'grapes-green' : season === 'autumn' ? 'grapes-ripe' : null;
  var SPARSE = season === 'winter' ? 3 : 1;  // in winter, only every third leaf still hangs

  function unit(p) { var la = p[1] * D, lo = p[0] * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; }
  function arc(a, b) { var u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))); }
  function along(a, b, t) {
    var u = unit(a), v = unit(b), w = arc(a, b);
    if (w < 1e-9) return a.slice();
    var sa = Math.sin((1 - t) * w) / Math.sin(w), sb = Math.sin(t * w) / Math.sin(w);
    var p = [u[0] * sa + v[0] * sb, u[1] * sa + v[1] * sb, u[2] * sa + v[2] * sb];
    return [Math.atan2(p[0], p[2]) / D, Math.asin(Math.max(-1, Math.min(1, p[1]))) / D];
  }
  function point(lon, lat, props) { return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat] }, properties: props || {} }; }
  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }
  function said(y) { return y < 1 ? (1 - y) + ' BCE' : String(y); }

  // ---- the stems: each branch drawn as far as the year has taken it ----
  var year = FIRST, drawn = [];
  V.branches.forEach(function (b, i) {
    drawn[i] = { n: -1, tip: null };
    var age = b.start <= -3000 ? 2.4 : b.start <= 0 ? 1.8 : 1.3;
    terra.geo.set('stem-' + i, [], { style: 'border', color: b.start <= 0 ? STEM : STEM_YOUNG, lineWidth: age });
  });
  function grow() {
    V.branches.forEach(function (b, i) {
      var way = b.way, d = drawn[i];
      if (year < b.start) { if (d.n !== 0) { d.n = 0; d.tip = null; terra.geo.set('stem-' + i, [], { style: 'border', color: STEM, lineWidth: 1 }); terra.sprites.remove('tip-' + i); } return; }
      var n = 0;
      while (n < way.length && way[n][2] <= year) n++;
      var coords = way.slice(0, n).map(function (p) { return [p[0], p[1]]; }), tip = null;
      if (n < way.length && n > 0) {
        var a = way[n - 1], c = way[n], t = c[2] > a[2] ? (year - a[2]) / (c[2] - a[2]) : 1;
        tip = along([a[0], a[1]], [c[0], c[1]], Math.max(0, Math.min(1, t)));
        coords.push([Math.round(tip[0] * 1000) / 1000, Math.round(tip[1] * 1000) / 1000]);
      }
      var complete = n >= way.length;
      if (d.n === n && complete) return;      // grown to its end already
      d.n = n;
      var age = b.start <= -3000 ? 2.4 : b.start <= 0 ? 1.8 : 1.3;
      if (coords.length > 1) terra.geo.set('stem-' + i, [{ type: 'LineString', coordinates: coords }], { style: 'border', color: b.start <= 0 ? STEM : STEM_YOUNG, lineWidth: age });
      if (tip && !complete) {
        terra.sprites.set('tip-' + i, { at: tip, label: b.name + ', growing', looks: [
          { frames: frames('tendril-big', 8), fps: 6, size: [44, 44], minZoom: NEAR },
          { frames: frames('tendril', 8), fps: 6, size: [20, 20], minZoom: MID, maxZoom: NEAR },
          { frames: frames('tendril', 8), fps: 6, size: [11, 11], maxZoom: MID }
        ] });
      } else {
        terra.sprites.remove('tip-' + i);
      }
    });
  }

  // ---- the leaves and the clusters ----
  var leavesShown = -1, picked = {}, grapesShown = '';
  function leaves() {
    var count = 0;
    V.leaves.forEach(function (l) { if (l[2] <= year) count++; });
    if (count === leavesShown) return;
    leavesShown = count;
    for (var k = 0; k < 3; k++) {
      var kind = V.leaves.filter(function (l, i) { return l[2] <= year && l[3] === k && i % SPARSE === 0; }).map(function (l) { return point(l[0], l[1]); });
      terra.geo.set('leaves-near-' + k, kind, { style: 'symbol', image: LEAF + '-big-' + (k + 1) + '.png', width: 22, height: 22, minZoom: NEAR, offset: [0, -4] });
      terra.geo.set('leaves-mid-' + k, kind, { style: 'symbol', image: LEAF + '-' + (k + 1) + '.png', width: 10, height: 10, minZoom: MID, maxZoom: NEAR, offset: [0, -2] });
      // from afar, every other leaf, tiny: the vine reads as a vine at the whole Earth
      terra.geo.set('leaves-far-' + k, kind.filter(function (f, i) { return i % 2 === 0; }), { style: 'symbol', image: LEAF + '-' + (k + 1) + '.png', width: 5.5, height: 5.5, maxZoom: MID, offset: [0, -1] });
    }
  }
  function grapes() {
    if (!GRAPES) return;
    var list = V.nodes.filter(function (n) { return n.year <= year && !picked[n.name + n.year]; });
    var key = list.length + ':' + Object.keys(picked).length;
    if (key === grapesShown) return;
    grapesShown = key;
    var features = list.map(function (n) { return point(n.lon, n.lat, { label: n.name + ', ' + said(n.year) }); });
    terra.geo.set('grapes-near', features, { style: 'symbol', image: GRAPES + '-big.png', width: 20, height: 30, minZoom: NEAR, offset: [0, 10], touchable: true });
    terra.geo.set('grapes-mid', features, { style: 'symbol', image: GRAPES + '.png', width: 10, height: 15, minZoom: MID, maxZoom: NEAR, offset: [0, 5], touchable: true });
  }
  var splashes = 0;
  function pick(index) {
    var list = V.nodes.filter(function (n) { return n.year <= year && !picked[n.name + n.year]; }), n = list[index];
    if (!n) return;
    picked[n.name + n.year] = true;
    grapes();
    var id = 'splash-' + (++splashes);
    terra.sprites.set(id, { at: [n.lon, n.lat], label: 'Grapes picked at ' + n.name, looks: [{ frames: frames('splash', 4), fps: 5, size: [24, 24] }] });
    setTimeout(function () { terra.sprites.remove(id); }, 900);
    // the vine bears again, in time
    setTimeout(function () { delete picked[n.name + n.year]; grapes(); }, 40000);
  }

  // ---- the origin: the qvevri ----
  function origin() {
    terra.geo.set('qvevri-near', [point(V.origin.lon, V.origin.lat, { label: 'The qvevri at ' + V.origin.name + ': touch to begin again' })],
                  { style: 'symbol', image: 'qvevri.png', width: 64, height: 72, minZoom: NEAR, offset: [0, 20], touchable: true });
    terra.geo.set('qvevri-mid', [point(V.origin.lon, V.origin.lat, { label: 'The qvevri at ' + V.origin.name + ': touch to begin again' })],
                  { style: 'symbol', image: 'qvevri-small.png', width: 28, height: 32, minZoom: MID, maxZoom: NEAR, offset: [0, 9], touchable: true });
  }
  function restart() {
    picked = {};
    year = FIRST;
    terra.time.set(FIRST);
    terra.time.play(true);
    grow(); leaves(); grapes();
  }

  // ---- touch ----
  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || !t.target) return;
    if (t.target.layer === 'grapes-near' || t.target.layer === 'grapes-mid') pick(t.target.index);
    else if (t.target.layer === 'qvevri-near' || t.target.layer === 'qvevri-mid') restart();
  });
  terra.on('time', function (e) { year = e.value; grow(); leaves(); grapes(); });
  terra.on('focus', function (f) { if (f.front) terra.time.play(true); });
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR !== 3.5) return false;
    R1 = r1; NEAR = zoomFor(30); MID = zoomFor(9);
    return true;
  }
  terra.on('camera', function (c) {
    if (!measure(c)) return;
    leavesShown = -1; grapesShown = ''; drawn.forEach(function (d) { d.n = -1; });
    origin(); grow(); leaves(); grapes();
  });

  terra.on('open', function () {
    terra.panel.show({
      title: 'The vine\'s long walk · ვაზის გრძელი გზა',
      wikipedia: 'History of wine',
      text: 'The oldest wine we know of was made in the South Caucasus, in the valleys south of Tbilisi, about eight thousand years ago: jars from the villages of Gadachrili Gora and Shulaveris Gora hold the chemical traces of grape wine from around 6000 BCE, and the vine was being grown there, not gathered. From that valley the grapevine went everywhere it grows today. The vine here is that walk drawn as one plant: it rises from a qvevri, the clay vessel buried to its mouth in which Georgian wine was made then and is made still, and grows along the ways the vine really went, in the years the record gives.',
      sections: [
        { title: 'The way it went', wikipedia: 'Vitis vinifera', text: 'Down the Zagros to Iran by 5400 BCE and into Anatolia; to the Levant and, by 3000 BCE, Egypt; over the sea to Crete and Greece; west with the Phoenicians to Carthage and Spain and with the Greeks to Sicily, Italy and Marseille (600 BCE); up the Rhône with Rome to Burgundy, Bordeaux, the Mosel and the Rhine, and up the Danube; east along the Silk Road to China by the second century BCE, and on to Japan. Then, from 1493, across the oceans: Hispaniola and Mexico, Peru and Chile, the Cape in 1655, California with the missions, Sydney with the First Fleet in 1788, New Zealand in 1819.' },
        { title: 'What is drawn', wikipedia: 'Qvevri', text: 'The ways are drawn, not records of any one vine\'s travels: the vine went by many hands, cuttings and seeds, often by sea, and the lines take the straightest way between the places the record names. The years are the earliest evidence at or near each place, rounded. The leaves and clusters are the globe\'s own seasons: green in spring and summer, gold and red at the vintage, bare in winter but for the last leaves. The qvevri and the first vine are an illustration.' },
        { title: 'Touching', text: 'Touch a cluster to pick it; the vine bears again. Touch the qvevri to begin the walk again. Drag the years.' }
      ],
      sources: [
        { title: 'McGovern, P. et al. (2017). Early Neolithic wine of Georgia in the South Caucasus. PNAS 114: E10309–E10318.', url: 'https://doi.org/10.1073/pnas.1714728114', note: 'Gadachrili Gora and Shulaveris Gora: tartaric acid in jars of about 6000–5800 BCE; the earliest evidence of wine.' },
        { title: 'McGovern, P. (2003). Ancient Wine: The Search for the Origins of Viniculture. Princeton.', note: 'Hajji Firuz, Godin Tepe, Egypt, the Levant and the Aegean: the spread through the ancient Near East.' },
        { title: 'Unwin, T. (1991). Wine and the Vine: An Historical Geography of Viticulture and the Wine Trade. Routledge.', note: 'The Phoenicians, Greeks and Romans in the west; the medieval and colonial plantings.' },
        { title: 'Johnson, H. (1989). Vintage: The Story of Wine. Simon & Schuster.', note: 'The colonial dates: Hispaniola, Mexico, Peru, Chile, the Cape, California, Australia and New Zealand.' },
        { title: 'UNESCO: Ancient Georgian traditional Qvevri wine-making method (Intangible Cultural Heritage, 2013)', url: 'https://ich.unesco.org/en/RL/ancient-georgian-traditional-qvevri-wine-making-method-00870', note: 'The qvevri.' }
      ]
    });
  });

  // ---- start: the first spring in the valley ----
  terra.globe.set({ still: true, maxZoom: 16 });
  measure(terra.camera);
  terra.caption.set({ title: 'The vine\'s long walk', text: '', wikipedia: 'Georgian wine' });
  origin();
  terra.time.show({ unit: 'year', min: FIRST, max: LAST, value: FIRST, ticks: V.nodes.map(function (n) { return n.year; }),
                    playable: true, rate: RATE, playing: true, carry: false });
  grow(); leaves(); grapes();
})();
