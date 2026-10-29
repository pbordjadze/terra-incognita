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
  var STEM = '#5C4530', EDGE = '#2E2014', STRAND = '#7A5E3F', VOYAGE = '#7A5A33', TRACE = '#E2A94BA8', GLOW = '#F2C86A38', LIGHT = '#FFF0C8A0';
  var QVEVRI_DEG = 1.9;                      // the picture (the rim is two fifths of it, the valley's ground round it)                      // the picture of the qvevri's mouth, in degrees of latitude

  // ---- scale: what shows depends on how many points a degree is on this screen, not the zoom alone ----
  var R1 = 489;                               // the globe's radius at zoom 1 (about 176 on a phone, 489 on the 13" iPad)
  function zoomFor(ppd) { return Math.round(ppd / (R1 * D) * 100) / 100; }
  var NEAR = 0, MID = 0, CLOSE = 0;
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR) return false;
    R1 = r1; NEAR = zoomFor(34); MID = zoomFor(11); CLOSE = zoomFor(120);
    return true;
  }

  // ---- the season each hemisphere is in, from the reader's own month: leaves in spring, green clusters
  // in summer, the vintage in autumn, bare stems and a few dry leaves in winter ----
  var month = new Date().getMonth();
  function season(south) {
    var m = south ? (month + 6) % 12 : month;
    return m >= 2 && m <= 4 ? 'spring' : m >= 5 && m <= 7 ? 'summer' : m >= 8 && m <= 10 ? 'autumn' : 'winter';
  }
  var LEAVES = { spring: 'leaf-green', summer: 'leaf-green', autumn: 'leaf-autumn', winter: 'leaf-winter' };
  var FAR_LEAF = { spring: 'leaf-green-1.png', summer: 'leaf-green-1.png', autumn: 'leaf-autumn-1.png', winter: 'leaf-winter-1.png' };
  function grapesFor(south) { var s = season(south); return s === 'summer' ? 'grapes-green.png' : s === 'autumn' ? 'grapes-ripe.png' : null; }

  function unit(p) { var la = p[1] * D, lo = p[0] * D; return [Math.cos(la) * Math.sin(lo), Math.sin(la), Math.cos(la) * Math.cos(lo)]; }
  function arc(a, b) { var u = unit(a), v = unit(b); return Math.acos(Math.max(-1, Math.min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2]))); }
  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }
  function said(y) { y = Math.round(y); return y < 1 ? (1 - y) + ' BCE' : y < 1000 ? y + ' CE' : String(y); }
  function point(lon, lat, year, props) { return { type: 'Feature', geometry: { type: 'Point', coordinates: [lon, lat, year] }, properties: props || {} }; }
  function line(coords, props) { return { type: 'Feature', geometry: { type: 'LineString', coordinates: coords }, properties: props || {} }; }

  // ---- everything the globe grows, set once and dated ----
  function plant() {
    // the voyages first, under the stems: the fine dashed line of the vine carried over the sea, in the
    // stem's own brown so it is the vine's and not one of the map's lines
    terra.geo.set('voyages', V.voyages.map(function (f) { return line(f.coordinates); }),
                  { style: 'ghost', color: VOYAGE, lineWidth: 1.4, dated: true });
    // the stems: every trunk's and twig's land runs, thickening with age; touchable. Three layers by zoom,
    // so the trunk is a slender line at the whole Earth and a thick old stem close in (a constant width
    // would be a root system from afar and a thread up close)
    var stems = V.stems.map(function (f) { return line(f.coordinates, { label: V.branches[f.branch].name }); });
    // from afar, a tint of leaf along the stems: a soft wide band under the thread, so the vine at the whole
    // Earth is a living colour and not a road
    var haze = season(false) === 'winter' ? '#8A6A3A3A' : season(false) === 'autumn' ? '#C8922E60' : '#6E9A4A60';
    terra.geo.set('stems-haze', stems, { style: 'border', color: haze, lineWidth: 7, dated: true, maxZoom: MID });
    // the stem's bark: a darker edge under each band, a little wider, so the stem has a body and not a line
    terra.geo.set('stems-edge-far', stems, { style: 'border', color: EDGE, lineWidth: 1.3, dated: true, thickens: { to: 3.6, over: 5000 }, maxZoom: MID });
    terra.geo.set('stems-edge', stems, { style: 'border', color: EDGE, lineWidth: 1.6, dated: true, thickens: { to: 4.8, over: 5000 }, minZoom: MID, maxZoom: NEAR });
    terra.geo.set('stems-edge-near', stems, { style: 'border', color: EDGE, lineWidth: 2.9, dated: true, thickens: { to: 8.6, over: 5000 }, minZoom: NEAR });
    // (a stem is full-grown after five thousand years: the first trunks are the thick ones, Rome's half
    // their width, the colonies' shoots thin)
    terra.geo.set('stems-far', stems, { style: 'border', color: STEM, lineWidth: 0.7, dated: true, thickens: { to: 2.6, over: 5000, grain: 0.15 }, touchable: true, maxZoom: MID });
    terra.geo.set('stems', stems, { style: 'border', color: STEM, lineWidth: 0.7, dated: true, thickens: { to: 3.4, over: 5000, grain: 0.3 }, touchable: true, minZoom: MID, maxZoom: NEAR });
    terra.geo.set('stems-near', stems, { style: 'border', color: STEM, lineWidth: 1.7, dated: true, thickens: { to: 6.5, over: 5000, grain: 0.4 }, touchable: true, minZoom: NEAR });
    // the strands: two thinner stems twining round each trunk, over the trunk, from the middle distance in
    var strands = V.strands.map(function (f) { return line(f.coordinates); });
    terra.geo.set('strands', strands, { style: 'border', color: STRAND, lineWidth: 0.5, dated: true, thickens: { to: 1.3, over: 5000, grain: 0.3 }, minZoom: MID, maxZoom: NEAR });
    terra.geo.set('strands-near', strands, { style: 'border', color: STRAND, lineWidth: 0.9, dated: true, thickens: { to: 2.6, over: 5000, grain: 0.4 }, minZoom: NEAR });
    // the leaves: by kind and hemisphere (each keeps its own season), large close in, small from the middle
    // distance, a tint from afar
    // (a winter hemisphere keeps one leaf in three, dry)
    for (var hemi = 0; hemi < 2; hemi++) {
      var s = season(hemi === 1), sparse = s === 'winter' ? 3 : 1;
      for (var k = 0; k < 3; k++) {
        var kind = V.leaves.filter(function (l, i) { return l[3] === k && l[4] === hemi && i % sparse === 0; }).map(function (l) { return point(l[0], l[1], l[2]); });
        var image = LEAVES[s] + '-' + (k + 1) + '.png';
        // (the leaf's stalk is at its foot: the picture sits just above its place on the stem)
        // (each kind its own size and turn, so the leaves along a stem are not one stamp)
        var sz = [0.8, 1, 1.22][k];
        terra.geo.set('leaves-close-' + hemi + k, kind, { style: 'symbol', image: image, width: 46 * sz, height: 46 * sz, minZoom: CLOSE, offset: [2, -15 * sz], dated: true, bloom: 1.2 });
        terra.geo.set('leaves-near-' + hemi + k, kind, { style: 'symbol', image: image, width: 22 * sz, height: 22 * sz, minZoom: NEAR, maxZoom: CLOSE, offset: [1, -7 * sz], dated: true, bloom: 0.9 });
        terra.geo.set('leaves-mid-' + hemi + k, kind, { style: 'symbol', image: image, width: 9.5 * sz, height: 9.5 * sz, minZoom: MID, maxZoom: NEAR, offset: [0.5, -3 * sz], dated: true, bloom: 0.7 });
      }
      // from afar, a few leaves still: one in four, small, so the whole Earth sees a plant and not a cable
      var far = V.leaves.filter(function (l, i) { return l[4] === hemi && i % (4 * sparse) === 0; }).map(function (l) { return point(l[0], l[1], l[2]); });
      terra.geo.set('leaves-far-' + hemi, far, { style: 'symbol', image: FAR_LEAF[s], width: 6, height: 6, maxZoom: MID, offset: [0, -2], dated: true, bloom: 0.6 });
    }
    // the clusters, hanging where the vine arrives, in the hemisphere's summer and autumn; touchable:
    // light runs to them. None from afar: the whole Earth sees the vine as one thread
    for (hemi = 0; hemi < 2; hemi++) {
      var grapes = grapesFor(hemi === 1);
      var clusters = grapes ? V.clusters.filter(function (c) { return (c.lat < 0 ? 1 : 0) === hemi; }).map(function (c) {
        return point(c.lon, c.lat, c.year, { label: c.name + ', ' + said(c.year) });
      }) : [];
      terra.geo.set('clusters-close-' + hemi, clusters, { style: 'symbol', image: grapes || 'grapes-ripe.png', width: 30, height: 45, minZoom: CLOSE, offset: [0, 21], dated: true, bloom: 1.6, touchable: true });
      terra.geo.set('clusters-near-' + hemi, clusters, { style: 'symbol', image: grapes || 'grapes-ripe.png', width: 15, height: 22.5, minZoom: NEAR, maxZoom: CLOSE, offset: [0, 10], dated: true, bloom: 1.2, touchable: true });
      terra.geo.set('clusters-mid-' + hemi, clusters, { style: 'symbol', image: grapes || 'grapes-ripe.png', width: 7, height: 10.5, minZoom: MID, maxZoom: NEAR, offset: [0, 4.5], dated: true, bloom: 0.9, touchable: true });
      // from afar, the trunks' own places only (not the twigs'), tiny: fruit where the record is
      var farClusters = grapes ? V.clusters.filter(function (c) { return (c.lat < 0 ? 1 : 0) === hemi && !V.branches[c.branch].twig; }).map(function (c) {
        return point(c.lon, c.lat, c.year, { label: c.name + ', ' + said(c.year) });
      }) : [];
      terra.geo.set('clusters-far-' + hemi, farClusters, { style: 'symbol', image: grapes || 'grapes-ripe.png', width: 4.5, height: 6.75, maxZoom: MID, offset: [0, 3], dated: true, bloom: 0.7, touchable: true });
    }
    // the qvevri, where it all begins: a picture lying on the globe, so it grows as the reader comes close
    // (a mouth in the earth two degrees across; the stems grow out of it, drawn over it). A tap on it
    // begins again (the pack takes every tap: "taps": true).
    var half = QVEVRI_DEG / 2;
    terra.images.set('qvevri', { image: 'qvevri.png', bounds: [V.origin.lon - half / Math.cos(V.origin.lat * D), V.origin.lat - half,
                                                                V.origin.lon + half / Math.cos(V.origin.lat * D), V.origin.lat + half], opacity: 1 });
    // the ships that carried it: a sail on each long voyage while the years cross that sea, on dated ways
    V.ships.forEach(function (v, i) {
      terra.sprites.set('ship-' + i, {
        path: { coordinates: v.coordinates, dates: v.dates },
        looks: [
          { frames: frames('ship', 2), fps: 1.6, size: [30, 24], minZoom: NEAR, face: true },
          { frames: frames('ship', 2), fps: 1.6, size: [15, 12], minZoom: MID, maxZoom: NEAR, face: true },
          { frames: frames('ship', 2), fps: 1.2, size: [8, 6.5], maxZoom: MID, face: true }
        ],
        label: V.branches[v.branch].name + ': carried by sea'
      });
    });
    // the tendrils at the growing tips: sprites on dated ways, there while their trunk grows
    V.tips.forEach(function (t) {
      terra.sprites.set('tip-' + t.branch, {
        path: { coordinates: t.coordinates, dates: t.dates },
        looks: [
          { frames: frames('tendril', 8), fps: 5, size: [68, 60], minZoom: CLOSE, face: true },
          { frames: frames('tendril', 8), fps: 5, size: [34, 30], minZoom: NEAR, maxZoom: CLOSE, face: true },
          { frames: frames('tendril', 8), fps: 5, size: [15, 13], minZoom: MID, maxZoom: NEAR, face: true },
          { frames: frames('tendril', 8), fps: 4, size: [7, 6], maxZoom: MID, face: true }
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
  /** The way lit: the line in gold with its light running outward, and a spark that leaves the qvevri and
      runs the whole way to the place touched, arriving in a breath or two. */
  var sparks = 0;
  function light(branch, index, name) {
    var coords = wayBack(branch, index);
    if (coords.length < 2) return;
    // the stem itself lit from within: a warm band the stem's width and a little more, with soft gold
    // light running outward along it, narrower than the band
    var ppd = R1 * terra.camera.zoom * D, band = ppd >= 120 ? 10 : ppd >= 34 ? 7 : ppd >= 11 ? 4 : 2.6;
    terra.geo.set('trace-glow', [line(coords)], { style: 'border', color: GLOW, lineWidth: band * 2.6 });
    terra.geo.set('trace', [line(coords)], { style: 'border', color: TRACE, lineWidth: band,
                  flow: { speed: 70, dash: 3, gap: 44, color: LIGHT, width: band * 0.35 } });
    // the spark leaves the qvevri and is under the finger within a breath
    var seconds = Math.min(1.3, 0.5 + coords.length / 240), id = 'spark-' + (++sparks), end = coords[coords.length - 1];
    terra.sprites.set(id, { path: { coordinates: coords, seconds: seconds, loop: false },
                            looks: [{ image: 'spark.png', size: [34, 34] }], label: 'The light running along the vine' });
    setTimeout(function () { terra.sprites.remove(id); }, seconds * 1000 + 300);
    // and where the finger was, the light breathes while the way stays lit
    terra.sprites.set('touched', { at: end, looks: [{ frames: frames('pulse', 6), fps: 7, size: [46, 46] }], label: 'The place touched' });
    terra.caption.set({ title: 'The vine\'s long walk', text: name, wikipedia: 'Georgian wine' });
    if (traceTimer) clearTimeout(traceTimer);
    traceTimer = setTimeout(function () {
      terra.geo.remove('trace'); terra.geo.remove('trace-glow'); terra.sprites.remove('touched'); traceTimer = null;
      terra.caption.set({ title: 'The vine\'s long walk', text: '', wikipedia: 'Georgian wine' });
    }, 11000);
  }
  function restart() {
    terra.geo.remove('trace'); terra.geo.remove('trace-glow');
    terra.sprites.remove('touched');
    terra.caption.set({ title: 'The vine\'s long walk', text: '', wikipedia: 'Georgian wine' });
    terra.time.set(FIRST);
    terra.time.play(true);
  }
  terra.on('touch', function (t) {
    if (t.phase !== 'tap') return;
    if (!t.target) {
      // a tap on the qvevri's mouth begins again; any other tap on the globe is nothing
      if (t.lon != null && arc([t.lon, t.lat], [V.origin.lon, V.origin.lat]) / D <= QVEVRI_DEG * 0.3) restart();
      return;
    }
    if (!t.target.layer) return;
    var id = t.target.layer, i = t.target.index;
    if (id.indexOf('stems') === 0) {
      var f = V.stems[i];
      if (!f || t.lon == null) return;
      var index = nearestOn(f.branch, t.lon, t.lat, year);
      var p = V.branches[f.branch].way[index];
      light(f.branch, index, V.branches[f.branch].name + ' · since ' + said(p[2]));
    } else if (id.indexOf('clusters-') === 0) {
      var hemi = +id.slice(-1);
      var list = V.clusters.filter(function (c) { return (c.lat < 0 ? 1 : 0) === hemi; }), c = list[i];
      if (!c) return;
      light(c.branch, nearestOn(c.branch, c.lon, c.lat, Infinity), c.name + ' · since ' + said(c.year));
    }
  });

  // ---- the years: slower as the reader comes close, so the tip walks rather than races ----
  var readerPaused = false;
  terra.on('time', function (e) {
    year = e.value;
    if (e.by === 'reader') readerPaused = !e.playing;
  });
  terra.on('focus', function (f) { if (f.front && !readerPaused && year < LAST) terra.time.play(true); });
  var lastSpeed = 0, opening = true;
  terra.on('camera', function (c) {
    if (measure(c)) plant();
    // (while the opening pulls back from the qvevri, the years run as written)
    var ppd = R1 * c.zoom * D, speed = opening ? 1 : Math.min(1, Math.max(0.2, 10 / ppd));
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

  // ---- start: close on the qvevri's mouth in the valley, a breath, then the first shoot and the slow
  // pull-back as the vine runs; the reader's own touch ends the pull-back at once ----
  terra.globe.set({ still: true, historical: true, maxZoom: 32 });
  measure(terra.camera);
  terra.caption.set({ title: 'The vine\'s long walk', text: '', wikipedia: 'Georgian wine' });
  plant();
  var still = terra.settings.reduceMotion;
  // close: the qvevri's mouth a quarter of the screen's width (an anchor stops at 8, so the script flies)
  var width0 = terra.camera && terra.camera.width > 0 ? terra.camera.width : 1032;
  var closeZoom = Math.min(32, zoomFor(width0 * 0.25 / (QVEVRI_DEG * 0.41)));
  terra.time.show({ unit: 'year', pace: V.pace, value: FIRST, ticks: V.ticks, playable: true, playing: false, carry: false });
  // (a flight within a second of the reader's touch is dropped: the dive to the qvevri waits a beat after
  // the doorway's tap, which is also the right pause)
  if (!still) setTimeout(function () { terra.flyTo(V.origin.lon, V.origin.lat, closeZoom, 1.8); }, 1100);
  setTimeout(function () { terra.time.play(true); }, still ? 400 : 3600);
  // the camera stays close while the first shoot leaves the mouth, curls and leafs (about ten seconds);
  // then it pulls back over twenty-five, as the vine fills the frame, to about fifty-five degrees across
  // the screen (the Caucasus to the Nile), and on a phone no farther than a degree to twelve points
  setTimeout(function () {
    var width = terra.camera && terra.camera.width > 0 ? terra.camera.width : 1032;
    if (!still) terra.flyTo(43.5, 39, zoomFor(Math.max(12, width / 55)), 28);
  }, still ? 400 : 14500);
  setTimeout(function () { opening = false; lastSpeed = 0; }, 43000);
})();
