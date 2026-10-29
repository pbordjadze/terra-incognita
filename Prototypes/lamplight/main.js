/* Lamplight: evening at the Himalaya's edge. The globe turns to its own night (the Earth at night);
   over the mountains and the plains below them the night is drawn closer than the globe's map can (the
   relief and the snow by moonlight, the plains' lights as the satellite saw them); the setting sun's
   last light stays a moment on the high snow; then the lamps come on where people live in the hills,
   climbing the valleys band by band, until the last ones, the highest villages lived in through the
   winter, flicker on alone. Touch one of those to hear its name. The words, all on demand, and the
   sources are in the panel (Open) and SOURCES.md; tools/lamplight/build.py draws the pictures. */
(function () {
  'use strict';
  var B = window.LAMPLIGHT_BOUNDS, VILLAGES = window.LAMPLIGHT_VILLAGES;
  var still = terra.settings.reduceMotion;
  // the globe's own night, where this app has it; before it, a pool of evening drawn by the pack
  var night = !!(terra.globe.can && terra.globe.can.night);
  // fading a picture by setting it again, where the app changes it in place (else it blinks: steps)
  var fades = !!(terra.images.can && terra.images.can.fade);
  terra.globe.set({ night: night, historical: true, maxZoom: 24 });

  // ---- pictures: the order they are first set is the order they are drawn in
  var pics = {};
  function picture(id, image, o) {
    pics[id] = { image: image, bounds: B[id], opacity: o.opacity, minZoom: o.minZoom || 0, maxZoom: o.maxZoom || 1e9, fade: o.fade || 0 };
    terra.images.set(id, pics[id]);
  }
  function opacity(id, value) {
    if (!pics[id]) return;
    pics[id].opacity = value;
    terra.images.set(id, pics[id]);
  }
  // a short eased change (docs/runtime3.md: fine for a pack; the globe animates nothing for it)
  var tweens = {};
  function metres(m) { return String(m).replace(/(\d)(?=(\d{3})+$)/, '$1,'); }
  function fade(id, to, seconds) {
    if (!pics[id]) return;
    clearTimeout(tweens[id]);
    if (still || !fades || seconds <= 0) { opacity(id, to); return; }
    var from = pics[id].opacity, t0 = performance.now(), ms = seconds * 1000;
    (function step() {
      var t = Math.min(1, (performance.now() - t0) / ms), e = t * t * (3 - 2 * t);
      opacity(id, from + (to - from) * e);
      if (t < 1) tweens[id] = setTimeout(step, 70);
    })();
  }
  var timers = [];
  function later(seconds, fn) { timers.push(setTimeout(fn, still ? 0 : seconds * 1000)); }

  // ---- the lamps of the highest villages: each its own flicker, lit lowest first
  var FRAMES = ['lamp-1.png', 'lamp-2.png', 'lamp-3.png', 'lamp-4.png', 'lamp-5.png', 'lamp-6.png'];
  function orderOf(k) {   // each lamp its own pace and its own place in the flicker
    var f = FRAMES.slice(k % FRAMES.length).concat(FRAMES.slice(0, k % FRAMES.length));
    return k % 2 ? f.reverse() : f;
  }
  function lightVillages() {
    VILLAGES.slice().sort(function (a, b) { return a.m - b.m; }).forEach(function (v, k) {
      later(k * 0.16, function () {
        terra.sprites.set('village-' + v.name, {
          at: [v.lon, v.lat],
          looks: [{ frames: orderOf(k), fps: 2.2 + (k * 0.37) % 1.6, size: [9, 9], minZoom: 2.6 }],
          label: v.name + ', ' + metres(v.m) + ' metres: a lamp',
          touchable: true
        });
      });
    });
  }

  // ---- evening
  if (!night) {
    picture('pool', 'pool.png', { opacity: still ? 1 : 0 });
    fade('pool', 1, 2.0);
  }
  picture('land', 'land.png', { opacity: still ? 1 : 0 });
  picture('lamps-1', 'lamps-1.png', { opacity: still ? 1 : 0 });
  picture('lamps-2', 'lamps-2.png', { opacity: still ? 1 : 0 });
  picture('lamps-3', 'lamps-3.png', { opacity: still ? 1 : 0 });
  if (!still) {
    // the sun's last light on the snow, as the land darkens; the highest keep it longest
    picture('glow', 'glow.png', { opacity: 0 });
    picture('crown', 'crown.png', { opacity: 0 });
    fade('glow', 1, 1.0);
    fade('crown', 1, 1.0);
  }
  fade('land', 1, night ? 0.9 : 2.0);
  // the lamps climb: the low hills, then the middle hills, then the high valleys
  later(2.0, function () { fade('lamps-1', 1, 1.8); });
  later(3.6, function () { fade('lamps-2', 1, 1.8); });
  later(5.2, function () { fade('lamps-3', 1, 1.8); });
  later(3.2, function () { fade('glow', 0, 3.2); });
  later(5.4, function () { fade('crown', 0, 3.4); });
  later(6.4, lightVillages);
  // closer in, the night drawn closer still (every lamp, not a quarter of them)
  later(9.4, function () {
    terra.images.remove('glow'); terra.images.remove('crown');
    delete pics.glow; delete pics.crown;
    picture('detail', 'detail.jpg', { opacity: 1, minZoom: 5, fade: 1.5 });
  });

  // ---- a village's name, on demand
  var hush = 0;
  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || !t.target || !t.target.sprite) return;
    var name = String(t.target.sprite).replace(/^village-/, '');
    var v = VILLAGES.filter(function (x) { return x.name === name; })[0];
    if (!v) return;
    terra.caption.set({ title: v.name + ' · ' + metres(v.m) + ' m', text: v.line });
    clearTimeout(hush);
    hush = setTimeout(function () { terra.caption.set({}); }, 9000);
  });

  // ---- the longer words and the sources, for whoever asks
  terra.on('open', function () {
    terra.panel.show({
      title: 'Lamplight',
      text: 'Evening at the Himalaya’s edge: the lamps of the people who live in the mountains, thinning as the land climbs to the snow.',
      sections: [
        { title: 'Measured and drawn',
          text: 'The plains’ lights are as NASA’s satellite saw them in 2016. From space the hills are almost dark, though millions live there: their lamps are drawn, one for every 150 people where they live (one for every 600 from further off), from the European Commission’s map of where people live. The relief and the snow are drawn from an elevation model, by moonlight; the rose on the high snow is the setting sun’s last light, drawn from the mountains’ shape.' },
        { title: 'Where people live',
          text: 'Half of Nepal lives on the plain and a third in the middle hills, between 1,000 and 2,500 m. The foothills between them, the Siwaliks, are the emptiest. Above 3,000 m lives about one Nepali in 700.' },
        { title: 'The highest lamps',
          text: 'The last lamps up each valley are villages lived in through the winter: about 4,000 to 4,300 m in Dolpo, and up to 4,570 m at Karzok in Ladakh. Herders on the Tibetan plateau live higher still. Many high villages empty in winter, their people gone down to the towns. Touch a lamp to hear its name and height, measured on the elevation model.' }
      ],
      sources: [
        { title: 'NASA Earth Observatory, Earth at Night 2016 (Black Marble)', url: 'https://earthobservatory.nasa.gov/features/NightLights', note: 'the plains’ lights' },
        { title: 'European Commission JRC, GHSL population 2020 (GHS-POP R2023A), CC BY 4.0', url: 'https://human-settlement.emergency.copernicus.eu/', note: 'where the lamps are' },
        { title: 'Copernicus DEM GLO-90', url: 'https://registry.opendata.aws/copernicus-dem/', note: 'the relief, the snow, the alpenglow and the villages’ heights' },
        { title: 'C. L. Chidi, Human Settlements in High Altitude Region Nepal (2009)', url: 'https://www.nepjol.info/index.php/gjn/article/view/17436', note: 'settlement above 3,000 m' },
        { title: 'S. F. Stevens, Claiming the High Ground (1993)', url: 'https://publishing.cdlib.org/ucpressebooks/public/book/claiming-the-high-ground-sherpas-subsistence-and-environmental-change-in-the-highest-himalaya.html', note: 'the Khumbu’s villages' }
      ]
    });
  });

  terra.on('close', function () {
    timers.forEach(clearTimeout);
    for (var id in tweens) clearTimeout(tweens[id]);
    clearTimeout(hush);
  });
})();
