/* When the sea was low (runtime 4). Touch the sea and the water goes out: the continental shelves come
   up as dry land, as they were when the ice was greatest and the sea stood about 120 m lower: Britain
   joined to Europe over Doggerland, the islands of the Sunda shelf one land, Australia and New Guinea
   one, Asia and America joined across Beringia, the Gulf a river valley. Rivers run across the dry
   shelves to the old shore, a lake fills the Gulf of Carpentaria, and mammoths walk where the North
   Sea and the Bering Strait are. Touch the sea again and it comes back. Nothing is written on the
   screen but the name; SOURCES.md and the panel the chip's Open shows say what is true and drawn. */
(function () {
  'use strict';
  var L = window.LOWSEA, D = Math.PI / 180;
  var R1 = 489;
  function zoomFor(ppd) { return ppd / (R1 * D); }
  var NEAR = 5, MID = 2.2, FAR = 1.2;          // zooms (set from the screen) for the mammoths' three sizes
  var DRAIN = 7, RETURN = 6, TICK = 50;         // seconds the sea takes to go, and to come back

  function frames(name, n) { var out = []; for (var i = 1; i <= n; i++) out.push(name + '-' + i + '.png'); return out; }

  // ---- the shelves: eight pictures, faded together ----
  var level = 0, fading = 0, state = 'sea';
  function paint() {
    L.shelves.forEach(function (s) {
      terra.images.set(s.id, { image: 'shelf-' + s.id + '.png', bounds: s.bounds, opacity: level, minZoom: 0 });
    });
  }
  function fade(to, seconds, done) {
    clearInterval(fading);
    var start = Date.now(), from = level;
    fading = setInterval(function () {
      var f = Math.min(1, (Date.now() - start) / (seconds * 1000)); f = f * f * (3 - 2 * f);
      level = from + (to - from) * f;
      paint();
      if (f >= 1) { clearInterval(fading); fading = 0; if (done) done(); }
    }, TICK);
  }

  // ---- what lives on the dry land: rivers, the lake, the mammoths ----
  function rivers(show) {
    terra.geo.set('rivers', show ? L.rivers.map(function (r) { return { type: 'Feature', geometry: { type: 'LineString', coordinates: r.line }, properties: { label: r.name } }; }) : [],
                  { style: 'border', color: '#3F7FA6', lineWidth: 1.1, minZoom: 1.4 });
    terra.geo.set('lakes', show ? L.lakes.map(function (l) { return { type: 'Feature', geometry: { type: 'Polygon', coordinates: [l.ring] }, properties: { label: l.name } }; }) : [],
                  { style: 'area', color: '#7FB7D6', lineWidth: 0.8, minZoom: 1.4 });
  }
  function looks() {
    return [{ frames: frames('mammoth', 6), fps: 5, size: [48, 40], minZoom: NEAR, face: true },
            { frames: frames('mammoth-small', 6), fps: 5, size: [20, 17], minZoom: MID, maxZoom: NEAR, face: true },
            { frames: frames('mammoth-tiny', 4), fps: 4, size: [10, 8], minZoom: FAR, maxZoom: MID, face: true }];
  }
  function herds(show) {
    L.herds.forEach(function (h, i) {
      for (var k = 0; k < h.count; k++) {
        var id = 'mammoth-' + i + '-' + k;
        if (!show) { terra.sprites.remove(id); continue; }
        // in file: each a little behind the one before, by a short way back along the path
        var c = h.path.slice(), lag = k * 0.35;
        var first = c[0], second = c[1];
        var start = [first[0] - (second[0] - first[0]) * lag * 0.3, first[1] - (second[1] - first[1]) * lag * 0.3 + (k % 2 ? 0.12 : -0.12)];
        terra.sprites.set(id, { path: { coordinates: [start].concat(c), seconds: h.seconds, loop: 'back' }, looks: looks(),
                                label: 'A mammoth, walking where the ' + (h.on === 'doggerland' ? 'North Sea' : 'Bering Strait') + ' is', touchable: false });
      }
    });
  }

  // ---- the cycle ----
  var again = 0;
  function drain() {
    if (state !== 'sea') return;
    state = 'draining';
    clearTimeout(again);
    fade(1, DRAIN, function () { state = 'dry'; again = setTimeout(flood, 60000); });
    setTimeout(function () { rivers(true); herds(true); }, DRAIN * 600);
  }
  function flood() {
    if (state !== 'dry') return;
    state = 'flooding';
    clearTimeout(again);
    rivers(false); herds(false);
    fade(0, RETURN, function () { state = 'sea'; again = setTimeout(drain, 20000); });
  }

  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || t.lon == null) return;
    if (state === 'sea') drain(); else if (state === 'dry') flood();
  });

  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : R1;
    if (Math.abs(r1 - R1) / R1 < 0.08 && NEAR !== 5) return false;
    R1 = r1; NEAR = zoomFor(42); MID = zoomFor(18); FAR = zoomFor(9);
    return true;
  }
  terra.on('camera', function (c) { if (measure(c) && state === 'dry') herds(true); });

  terra.on('open', function () {
    terra.panel.show({
      title: 'When the sea was low',
      wikipedia: 'Last Glacial Maximum',
      text: 'At the height of the last ice age, about 20,000 years ago, so much water was held in the ice sheets that the sea stood about 120 metres lower than today. The shallow seas round the continents were dry land: Britain was joined to Europe across what is now the North Sea (Doggerland), the islands of Indonesia were one land (Sundaland), Australia and New Guinea were one (Sahul), Asia and America were joined across the Bering Strait (Beringia), the Persian Gulf was a river valley. People and animals lived on all of it.',
      sections: [
        { title: 'What is drawn', wikipedia: 'Sundaland', text: 'The land that comes up is the sea floor shallower than 200 metres, the nearest contour Natural Earth draws: the ice age\'s shore lay a little inside it. The rivers are the main ones known from the sea floor\'s valleys, drawn in outline: the Channel River that gathered the Thames, Rhine and Seine; the Elbe running north across the dry North Sea; the great river of the Sunda shelf; the Tigris and Euphrates running on down the Gulf. Lake Carpentaria filled the Gulf of Carpentaria. The mammoths walk where their bones are dredged up, on Doggerland and Beringia; their number and their ways are drawn.' },
        { title: 'Touching', text: 'Touch the sea and it goes out; touch it again and it comes back. Left alone, it comes and goes.' }
      ],
      sources: [
        { title: 'Natural Earth, 1:10m land and bathymetry, 200 m contour (public domain)', url: 'https://www.naturalearthdata.com/', note: 'The shelves.' },
        { title: 'Lambeck, K., et al. (2014). Sea level and global ice volumes from the Last Glacial Maximum to the Holocene. PNAS 111: 15296–15303.', url: 'https://doi.org/10.1073/pnas.1411762111', note: 'The sea about 120–130 m lower at the glacial maximum.' },
        { title: 'Gaffney, V., Thomson, K., Fitch, S. (2007). Mapping Doggerland: the Mesolithic landscapes of the southern North Sea. Archaeopress.', note: 'Doggerland and its rivers.' },
        { title: 'Gupta, S., et al. (2007). Catastrophic flooding origin of shelf valley systems in the English Channel. Nature 448: 342–345.', url: 'https://doi.org/10.1038/nature06018', note: 'The Channel River.' },
        { title: 'Voris, H. K. (2000). Maps of Pleistocene sea levels in Southeast Asia: shorelines, river systems and time durations. Journal of Biogeography 27: 1153–1167.', url: 'https://doi.org/10.1046/j.1365-2699.2000.00489.x', note: 'The Sunda shelf and its rivers.' },
        { title: 'Reeves, J. M., et al. (2008). The sedimentary record of palaeoenvironments and sea-level change in the Gulf of Carpentaria. Palaeogeography 246: 68–85.', note: 'Lake Carpentaria.' }
      ]
    });
  });
  terra.on('close', function () { clearInterval(fading); clearTimeout(again); });

  // ---- start: the sea as it is; a moment later it goes out ----
  terra.globe.set({ still: true, maxZoom: 12 });
  measure(terra.camera);
  terra.caption.set({ title: 'When the sea was low', text: '', wikipedia: 'Doggerland' });
  paint(); rivers(false);
  again = setTimeout(drain, 1800);
})();
