/* The vine's long walk, საქართველო (runtime 6). One vine, eight thousand years long: it rises from a qvevri
   in a Georgian valley where the oldest wine was made and, as the years play, grows across the globe along
   the ways the grapevine really went: down the Zagros, into Anatolia, to the Levant and Egypt, over the sea
   to Crete and Greece, west with the Phoenicians and the Greeks, up the Rhône, the Mosel and the Danube,
   east along the Silk Road to China and Japan, and in the last five centuries across the oceans to the
   Americas, the Cape, Australia and New Zealand. Over land it is a stem that thickens with age and puts out
   leaves as its tip moves on; over the sea it is a voyage, a fine dashed line (it was carried); a tendril
   writhes at every growing tip; a cluster hangs where it arrives. The leaves wear the season the reader is
   in, each hemisphere its own. Touch any stem or cluster and light runs along the vine from the qvevri to
   it, the way the vine came; touch the qvevri to begin again. Everything is set once, dated, and the globe
   grows it (docs/runtime3.md › dated geography); this script decides and never draws. Nothing is written
   on the screen but the name and, touched, a place and its year; SOURCES.md and the panel say what is true
   and what is drawn. Data: data.js (tools/vine/build.py); art: tools/vine/art.js. */
(function () {
  'use strict';
  var V = window.VINE, D = Math.PI / 180;
  var FIRST = V.origin.year, LAST = V.today;
  var STEM = '#5A3A1C', VOYAGE = '#3B5B73', TRACE = '#D9962BCC', LIGHT = '#FFF6D6';

  // ---- scale: what shows depends on how many points a degree is on this screen, not the zoom alone ----
  var R1 = 489;                               // the globe's radius at zoom 1 (about 176 on a phone, 489 on the 13" iPad)
  function zoomFor(ppd) { return Math.round(ppd / (R1 * D) * 100) / 100; }
  var NEAR = 3.5, MID = 1.1;
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR !== 3.5) return false;
    R1 = r1; NEAR = zoomFor(30); MID = zoomFor(9);
    return true;
  }

  // ---- the season each hemisphere is in, from the reader's own month ----
  var month = new Date().getMonth();
  function season(south) {
    var m = south ? (month + 6) % 12 : month;
    return m >= 3 && m <= 7 ? 'green' : m >= 8 && m <= 10 ? 'autumn' : 'winter';
  }
  function grapesFor(south) { var s = season(south); return s === 'green' ? 'grapes-green.png' : 'grapes-ripe.png'; }

  function unit(p) { var la = p[1] * D, lo = p[0] * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; }
  function arc(a, b) { var u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))); }
  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }
  function said(y) { y = Math.round(y); return y < 1 ? (1 - y) + ' BCE' : y < 1000 ? y + ' CE' : String(y); }
  function point(lon, lat, year, props) { return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat, year] }, properties: props || {} }; }
  function line(coords, props) { return { type: 'Feature', geometry: { type: 'LineString', coordinates: coords }, properties: props || {} }; }

  // ---- everything the globe grows, set once and dated ----
  function plant() {
    // the voyages first, under the stems: the fine dashed line of the vine carried over the sea
    terra.geo.set('voyages', V.voyages.map(function (f) { return line(f.coordinates); }),
                  { style: 'track', color: VOYAGE, lineWidth: 1.3, dated: true });
    // the stems: one layer, every trunk's and twig's land runs, thickening with age; touchable
    terra.geo.set('stems', V.stems.map(function (f) { return line(f.coordinates, { label: V.branches[f.branch].name }); }),
                  { style: 'border', color: STEM, lineWidth: 0.9, dated: true, thickens: { to: 3.4, over: 2500 }, touchable: true });
    // the leaves: by kind and hemisphere (each keeps its own season), large close in, small from the middle
    // distance, a tint from afar
    for (var hemi = 0; hemi < 2; hemi++) {
      var s = season(hemi === 1);
      for (var k = 0; k < 3; k++) {
        var kind = V.leaves.filter(function (l) { return l[3] === k && l[4] === hemi; }).map(function (l) { return point(l[0], l[1], l[2]); });
        var image = 'leaf-' + s + '-' + (k + 1) + '.png';
        terra.geo.set('leaves-near-' + hemi + k, kind, { style: 'symbol', image: image, width: 26, height: 26, minZoom: NEAR, offset: [0, -5], dated: true, bloom: 0.9 });
        terra.geo.set('leaves-mid-' + hemi + k, kind, { style: 'symbol', image: image, width: 12, height: 12, minZoom: MID, maxZoom: NEAR, offset: [0, -2], dated: true, bloom: 0.7 });
      }
      var all = V.leaves.filter(function (l) { return l[4] === hemi && l[3] !== 1; }).map(function (l) { return point(l[0], l[1], l[2]); });
      var far = { green: 'leaf-green-1.png', autumn: 'leaf-autumn-1.png', winter: 'leaf-winter-1.png' }[s];
      terra.geo.set('leaves-far-' + hemi, all, { style: 'symbol', image: far, width: 6.5, height: 6.5, maxZoom: MID, offset: [0, -1], dated: true, bloom: 0.6 });
    }
    // the clusters, hanging where the vine arrives; touchable: light runs to them
    for (hemi = 0; hemi < 2; hemi++) {
      var clusters = V.clusters.filter(function (c) { return (c.lat < 0 ? 1 : 0) === hemi; }).map(function (c) {
        return point(c.lon, c.lat, c.year, { label: c.name + ', ' + said(c.year) });
      });
      terra.geo.set('clusters-near-' + hemi, clusters, { style: 'symbol', image: grapesFor(hemi === 1), width: 18, height: 27, minZoom: NEAR, offset: [0, 11], dated: true, bloom: 1.2, touchable: true });
      terra.geo.set('clusters-mid-' + hemi, clusters, { style: 'symbol', image: grapesFor(hemi === 1), width: 9, height: 13.5, minZoom: MID, maxZoom: NEAR, offset: [0, 5], dated: true, bloom: 0.9, touchable: true });
    }
    // the qvevri, where it all begins; touch it to begin again
    var label = 'The qvevri at ' + V.origin.name + ': touch to begin the walk again';
    terra.geo.set('qvevri-near', [point(V.origin.lon, V.origin.lat, FIRST, { label: label })], { style: 'symbol', image: 'qvevri.png', width: 60, height: 68, minZoom: NEAR, offset: [0, 22], touchable: true });
    terra.geo.set('qvevri-mid', [point(V.origin.lon, V.origin.lat, FIRST, { label: label })], { style: 'symbol', image: 'qvevri.png', width: 26, height: 30, minZoom: MID, maxZoom: NEAR, offset: [0, 10], touchable: true });
    // the tendrils at the growing tips: sprites on dated ways, there while their trunk grows
    V.tips.forEach(function (t) {
      terra.sprites.set('tip-' + t.branch, {
        path: { coordinates: t.coordinates, dates: t.dates },
        looks: [
          { frames: frames('tendril', 8), fps: 5, size: [30, 26], minZoom: NEAR, face: true },
          { frames: frames('tendril', 8), fps: 5, size: [15, 13], minZoom: MID, maxZoom: NEAR, face: true },
          { frames: frames('tendril', 8), fps: 4, size: [8, 7], maxZoom: MID, face: true }
        ],
        label: t.name + ': the vine growing'
      });
    });
  }

  // ---- touch: the way the vine came, lit from the qvevri to the place touched ----
  var year = FIRST, traceTimer = null;
  /** The whole way from the first vine to a point of a branch: the branch back to where it grew from, and so on. */
  function wayBack(branch, index) {
    var out = [];
    for (var guard = 0; guard < 100 && branch != null; guard++) {
      var b = V.branches[branch], way = b.way.slice(0, index + 1).map(function (p) { return [p[0], p[1]]; });
      out = way.concat(out);
      if (!b.from) break;
      branch = b.from[0]; index = b.from[1];
    }
    return out;
  }
  function nearestOn(branch, lon, lat, until) {
    var way = V.branches[branch].way, best = 0, bd = Infinity;
    for (var i = 0; i < way.length; i++) {
      if (way[i][2] > until) break;
      var d = arc([way[i][0], way[i][1]], [lon, lat]);
      if (d < bd) { bd = d; best = i; }
    }
    return best;
  }
  function light(branch, index, name) {
    var coords = wayBack(branch, index);
    if (coords.length < 2) return;
    terra.geo.set('trace', [line(coords)], { style: 'border', color: TRACE, lineWidth: 3.6,
                  flow: { speed: 80, dash: 6, gap: 14, color: LIGHT, width: 2.2 } });
    terra.caption.set({ title: 'The vine\'s long walk', text: name, wikipedia: 'Georgian wine' });
    if (traceTimer) clearTimeout(traceTimer);
    traceTimer = setTimeout(function () { terra.geo.remove('trace'); traceTimer = null; }, 14000);
  }
  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || !t.target || !t.target.layer) return;
    var id = t.target.layer, i = t.target.index;
    if (id === 'stems') {
      var f = V.stems[i];
      if (!f || t.lon == null) return;
      var index = nearestOn(f.branch, t.lon, t.lat, year);
      var p = V.branches[f.branch].way[index];
      light(f.branch, index, V.branches[f.branch].name + ' · ' + said(p[2]));
    } else if (id.indexOf('clusters-') === 0) {
      var hemi = +id.slice(-1);
      var list = V.clusters.filter(function (c) { return (c.lat < 0 ? 1 : 0) === hemi; }), c = list[i];
      if (!c) return;
      light(c.branch, nearestOn(c.branch, c.lon, c.lat, Infinity), c.name + ' · ' + said(c.year));
    } else if (id.indexOf('qvevri-') === 0) {
      terra.geo.remove('trace');
      terra.caption.set({ title: 'The vine\'s long walk', text: '', wikipedia: 'Georgian wine' });
      terra.time.set(FIRST);
      terra.time.play(true);
    }
  });

  // ---- the years: slower as the reader comes close, so the tip walks rather than races ----
  var readerPaused = false;
  terra.on('time', function (e) {
    year = e.value;
    if (e.by === 'reader') readerPaused = !e.playing;
  });
  terra.on('focus', function (f) { if (f.front && !readerPaused && year < LAST) terra.time.play(true); });
  var lastSpeed = 0;
  terra.on('camera', function (c) {
    if (measure(c)) plant();
    var ppd = R1 * c.zoom * D, speed = Math.min(1, Math.max(0.2, 10 / ppd));
    speed = Math.round(speed * 20) / 20;
    if (speed !== lastSpeed) { lastSpeed = speed; terra.time.speed(speed); }
  });

  terra.on('open', function () {
    terra.panel.show({
      title: 'The vine\'s long walk · ვაზის გრძელი გზა',
      wikipedia: 'History of wine',
      text: 'The oldest wine we know of was made in the South Caucasus, in the valleys south of Tbilisi, about eight thousand years ago: jars from the villages of Gadachrili Gora and Shulaveris Gora hold the chemical traces of grape wine from around 6000 BCE, and the vine was being grown there, not gathered. From that valley the grapevine went everywhere it grows today. The vine here is that walk drawn as one plant: it rises from a qvevri, the clay vessel buried to its mouth in which Georgian wine was made then and is made still, and grows along the ways the vine really went, in the years the record gives.',
      sections: [
        { title: 'The way it went', wikipedia: 'Vitis vinifera', text: 'Down the Zagros to Iran by 5400 BCE and into Anatolia; to the Levant and, by 3000 BCE, Egypt; over the sea to Cyprus, Crete and Greece; west with the Phoenicians to Carthage and Cádiz and with the Greeks to Sicily, Italy and Massalia (600 BCE); up the Rhône with Rome to Burgundy, Bordeaux, the Mosel and the Rhine, and up the Danube; round the Black Sea with the Greeks; east along the Silk Road to China by the second century BCE, and on to Korea and Japan. Then, from 1493, across the oceans: Hispaniola and Mexico, Peru, Chile and Mendoza, Brazil, the Cape in 1655, California with the missions, Sydney with the First Fleet in 1788, New Zealand in 1819. The twigs reach the wine regions the record dates, to Ningxia in 1984 and Marlborough in 1973.' },
        { title: 'What is drawn', wikipedia: 'Qvevri', text: 'Over land the vine is a stem, thicker the longer it has grown there; over the sea it is a fine dashed line, because there it was carried, as cuttings and seeds, in ships. The ways are drawn, not records of any one vine\'s travels: they take the coasts, valleys and roads the record names. The years are the earliest evidence at or near each place, rounded; between places they are interpolated and mean nothing exact. The leaves and clusters wear the reader\'s own season, each hemisphere its own, so in October the vine is gold in Europe and in new leaf in Chile. Where the first eastern plantings in the Americas failed and the wine there was made from native grapes, no stem goes. The qvevri and the first vine are an illustration.' },
        { title: 'Touching', text: 'Touch any stem or cluster: light runs along the vine from the qvevri to it, the way it came. Touch the qvevri to begin again. Drag the years; come close and they slow.' }
      ],
      sources: [
        { title: 'McGovern, P. et al. (2017). Early Neolithic wine of Georgia in the South Caucasus. PNAS 114: E10309–E10318.', url: 'https://doi.org/10.1073/pnas.1714728114', note: 'Gadachrili Gora and Shulaveris Gora: tartaric acid in jars of about 6000–5800 BCE; the earliest evidence of wine.' },
        { title: 'McGovern, P. (2003). Ancient Wine: The Search for the Origins of Viniculture. Princeton.', note: 'Hajji Firuz, Godin Tepe, Egypt, the Levant and the Aegean: the spread through the ancient Near East.' },
        { title: 'Unwin, T. (1991). Wine and the Vine: An Historical Geography of Viticulture and the Wine Trade. Routledge.', note: 'The Phoenicians, Greeks and Romans in the west; the Black Sea colonies; the medieval and colonial plantings.' },
        { title: 'Johnson, H. (1989). Vintage: The Story of Wine. Simon & Schuster.', note: 'The colonial dates: Hispaniola, Mexico, Peru, Chile, Brazil, the Cape, California, Australia and New Zealand.' },
        { title: 'Robinson, J. & Harding, J. (eds.) (2015). The Oxford Companion to Wine, 4th ed. Oxford.', note: 'The regions\' first plantings: Parras, Senecú and El Paso; Constantia; Tasmania, the Yarra, Margaret River; Hawke\'s Bay, Central Otago, Marlborough; Yantai, Ningxia.' },
        { title: 'UNESCO: Ancient Georgian traditional Qvevri wine-making method (Intangible Cultural Heritage, 2013)', url: 'https://ich.unesco.org/en/RL/ancient-georgian-traditional-qvevri-wine-making-method-00870', note: 'The qvevri.' }
      ]
    });
  });

  // ---- start: the first spring in the valley ----
  terra.globe.set({ still: true, maxZoom: 16 });
  measure(terra.camera);
  terra.caption.set({ title: 'The vine\'s long walk', text: '', wikipedia: 'Georgian wine' });
  plant();
  terra.time.show({ unit: 'year', pace: V.pace, value: FIRST, ticks: V.ticks, playable: true, playing: true, carry: false });
})();
