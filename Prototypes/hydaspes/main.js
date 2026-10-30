/* The night crossing (runtime 5, docs/runtime3.md): the Hydaspes, 326 BCE, Alexander's crossing against
   Porus, as Arrian tells it (Anabasis 5.9–18). Everything on the globe is a sprite on a dated way, so
   the stage's date moves it: the column leaving its camp in the dark, the boats and hides pushed across
   to the island, the soldiers wading the second channel, the chariots, Porus' line and the horse that
   came round it. The reader drags the column along its way (or the date's strip) and the night moves
   under the finger; Play runs it. Nothing is written on the screen but the name: SOURCES.md and the
   panel (Open) say the rest, with the two sites still argued over. This script decides; it draws
   nothing itself. Data: data.js (tools/hydaspes/build.py); art: tools/hydaspes/art.js; words: notes.js. */
(function () {
  'use strict';
  var H = window.HYDASPES, NOTES = window.HYDASPES_NOTES, D = Math.PI / 180, HR = H.hours;
  var KM_PER_DEG = 110.6;

  // ----- hours of the night as the globe's date -----------------------------------------------------
  // The date runs in years (the label reads 326 BCE); an hour of this night is 1/8766 of one.
  function yr(h) { return H.y0 + h / 8766; }
  function hr(v) { return (v - H.y0) * 8766; }
  function way(w) {
    return { coordinates: w.map(function (p) { return [p[0], p[1]]; }), dates: w.map(function (p) { return yr(p[2]); }) };
  }
  /** A thing standing at a place between two hours. */
  function stand(p, h0, h1) { return { coordinates: [[p[0], p[1]], [p[0], p[1]]], dates: [yr(h0), yr(h1)] }; }

  // ----- scale: what shows depends on points a kilometre on this screen, not the zoom alone --------
  var R1 = 0;
  function measure(c) {
    var r1 = c && c.radius > 0 && c.zoom > 0 ? c.radius / c.zoom : 489;
    if (R1 && Math.abs(r1 - R1) / R1 < 0.08) return false;
    R1 = r1;
    return true;
  }
  /** The zoom at which a kilometre is `ppk` points across. */
  function zoomAtKm(ppk) { return Math.round(ppk * KM_PER_DEG / (R1 * D) * 100) / 100; }
  var PPK = { pool: 1.6, valley: 7, mid: 22, close: 48, deepest: 60 };
  var Z = {};
  function scales() {
    Z.pool = zoomAtKm(PPK.pool); Z.valley = zoomAtKm(PPK.valley); Z.mid = zoomAtKm(PPK.mid); Z.close = zoomAtKm(PPK.close);
    Z.max = Math.min(2048, zoomAtKm(PPK.deepest));
  }

  var frames = Shared.frames;      // the packs' shared script (shared.js, first in the scripts)
  /** Looks by zoom: a glint of fires from afar, the small picture, the drawn one close. */
  function looks(far, mid, close) {
    var out = [];
    if (far) out.push({ frames: frames(far.n, far.f), fps: far.fps || 3, size: far.size, maxZoom: Z.valley });
    if (mid) out.push({ frames: frames(mid.n, mid.f), fps: mid.fps || 4, size: mid.size, minZoom: far ? Z.valley : 0, maxZoom: Z.mid, face: !!mid.face });
    // (the small picture holds to the close zoom: the drawn one, a game's counter at a counter's size, only
    // when the ground is a few kilometres across, so from the valley the camps are fires and the elephants grains)
    if (mid) out[out.length - 1].maxZoom = Z.close;
    if (close) out.push({ frames: frames(close.n, close.f), fps: close.fps || 6, size: close.size, minZoom: mid ? Z.close : (far ? Z.valley : 0), face: !!close.face });
    return out;
  }

  /** Looks that show only from the valley zoom in (a row of elephants from the whole Punjab would bridge the river). */
  function fromValley(ls) { ls[0].minZoom = Z.valley; return ls; }

  // ----- the pictures on the ground: the day under the night, the night thinning at dawn --------------
  var nightShown = -1, veilShown = -1;
  /** The dark over the second channel: there until the column reaches the island's far shore and the
      river opens in front of it; and only as dark as the night is. */
  function veil(h) {
    var gate = h < HR.bank ? 1 : Math.max(0, 1 - (h - HR.bank) / 0.25);
    return gate * night(h);
  }
  function night(h) {
    // the wind and rain fall with the daylight (Arrian); the night thins over the next two hours
    if (h < HR.calm) return 1;
    if (h > HR.calm + 2.2) return 0;
    return 1 - (h - HR.calm) / 2.2;
  }
  function ground(h) {
    var o = Math.round(night(h) * 50) / 50, first = nightShown < 0;
    if (o !== nightShown) {
      nightShown = o;
      // (the first set of each fixes the order: the morning under the night, the close pair over the wide,
      // so the night thins the same inside the close picture as outside it; the close pair shows wherever
      // the wide does, the wide picture having a hole where the close one lies)
      if (first) terra.images.set('wide-day', { image: 'ground-wide-day.png', bounds: H.wide, minZoom: Z.pool, fade: Z.valley - Z.pool });
      terra.images.set('wide-night', { image: 'ground-wide-night.png', bounds: H.wide, minZoom: Z.pool, fade: Z.valley - Z.pool, opacity: o });
      if (first) terra.images.set('close-day', { image: 'ground-close-day.png', bounds: H.close, minZoom: Z.pool, fade: Z.valley - Z.pool });
      terra.images.set('close-night', { image: 'ground-close-night.png', bounds: H.close, minZoom: Z.pool, fade: Z.valley - Z.pool, opacity: o });
    }
    // the dark over the second channel lies over all of them (its first set comes last)
    var v = Math.round(veil(h) * 50) / 50;
    if (v !== veilShown) {
      veilShown = v;
      if (v > 0) terra.images.set('channel-veil', { image: 'channel-veil.png', bounds: H.veil, minZoom: Z.pool, fade: Z.valley - Z.pool, opacity: v });
      else terra.images.remove('channel-veil');
    }
  }

  // ----- everything that lives on the ground, each on its dated way: set once, moved by the date ----
  var P = H.places, W = H.ways;
  function place() {
    var camp = { far: { n: 'camp-far', f: 4, size: [14, 12] }, mid: { n: 'camp-mid', f: 6, size: [52, 39] } };
    // the great camp, then the camp left burning with a few figures among the fires, until Craterus goes
    terra.sprites.set('camp-a', { path: stand(P.campA, HR.dusk - 1, HR.leave), label: "Alexander's camp at the river", touchable: true,
                                  looks: looks(camp.far, camp.mid, { n: 'camp-a', f: 8, size: [150, 104], fps: 5 }) });
    terra.sprites.set('camp-a-left', { path: stand(P.campA, HR.leave, HR.craterus), label: 'The camp left behind, its fires burning', touchable: true,
                                       looks: looks(camp.far, camp.mid, { n: 'camp-decoy', f: 8, size: [150, 104], fps: 5 }) });
    terra.sprites.set('camp-p', { path: stand(P.campP, HR.dusk - 1, HR.porus), label: "Porus' camp on the far bank", touchable: true,
                                  looks: looks(camp.far, camp.mid, { n: 'camp-p', f: 8, size: [160, 109], fps: 5 }) });
    terra.sprites.set('camp-p-left', { path: stand(P.campP, HR.porus, HR.end + 1), label: 'The few left in Porus’ camp', touchable: true,
                                       looks: looks(camp.far, { n: 'camp-mid', f: 6, size: [24, 18] }, { n: 'camp-mid', f: 6, size: [70, 52] }) });
    // the camps' fires on the water nearest them
    var glow = function (w) { return [{ frames: frames('fireglow', 4), fps: 2, size: [Math.round(w * 0.45), Math.round(w * 0.27)], minZoom: Z.valley, maxZoom: Z.mid },
                                     { frames: frames('fireglow', 4), fps: 2, size: [w, Math.round(w * 0.6)], minZoom: Z.mid }]; };
    terra.sprites.set('glow-a', { path: stand(P.glowA, HR.dusk - 1, HR.craterus), label: '', looks: glow(64) });
    terra.sprites.set('glow-p', { path: stand(P.glowP, HR.dusk - 1, HR.porus), label: '', looks: glow(70) });
    terra.sprites.set('elephants', { path: stand(P.waterP, HR.dusk - 1, HR.porus), label: 'The elephants at the water, watching the far bank', touchable: true,
                                     looks: fromValley(looks(null, { n: 'elephants-mid', f: 4, size: [30, 8] }, { n: 'elephants-watch', f: 8, size: [112, 27], fps: 3 })) });
    // the column: out of the camp in the dark, inland, to the grove on the headland
    var col = { far: { n: 'column-far', f: 4, size: [20, 6] }, mid: { n: 'column-mid', f: 6, size: [40, 12], face: true }, close: { n: 'column', f: 8, size: [190, 34], fps: 8, face: true } };
    terra.sprites.set('column', { path: way(W.column), label: 'The column on its night march', touchable: true, looks: looks(col.far, col.mid, null) });
    // close, the column is a thread of clusters a few minutes apart along the same way, so it bends with
    // the way and the headland as a file does; the horse at its head
    for (var k = 0; k < 9; k++) {
      terra.sprites.set('file-' + k, { path: way(shift(W.column, HR.leave + k * 0.07, HR.headland + k * 0.07)), label: k ? '' : 'The column on its night march, the horse at its head', touchable: true,
                                       looks: [{ frames: frames(k ? 'cluster' : 'cluster-horse', 8), fps: 8, size: [30, 30], minZoom: Z.close, face: true }] });
    }
    terra.sprites.set('in-the-grove', { path: stand(P.headland, HR.headland, HR.boats), label: 'The column in the grove, the boats put together', touchable: true,
                                        looks: looks(col.far, { n: 'column-mid', f: 6, size: [40, 12], fps: 1 }, { n: 'formed', f: 4, size: [110, 44], fps: 1.5 }) });
    terra.sprites.set('boats', { path: way(W.boats), label: 'The galleys and the hay-stuffed hides, crossing to the island', touchable: true,
                                 looks: looks({ n: 'boats-mid', f: 4, size: [12, 5] }, { n: 'boats-mid', f: 4, size: [24, 10], face: true }, { n: 'boats', f: 6, size: [120, 54], fps: 5, face: true }) });
    terra.sprites.set('landed', { path: stand(W.onIsland[0], HR.island, HR.formed), label: 'Landed and forming up: on an island, though no one knows it yet', touchable: true,
                                  looks: looks(col.far, { n: 'column-mid', f: 6, size: [40, 12], fps: 1 }, { n: 'formed', f: 4, size: [110, 44], fps: 1.5 }) });
    terra.sprites.set('on-island', { path: way(W.onIsland), label: 'Marching across the island', touchable: true, looks: looks(col.far, col.mid, null) });
    for (var j = 0; j < 4; j++) {
      terra.sprites.set('island-file-' + j, { path: way(shift(W.onIsland, HR.formed + j * 0.05, HR.bank + j * 0.05)), label: '', touchable: true,
                                              looks: [{ frames: frames(j ? 'cluster' : 'cluster-horse', 8), fps: 8, size: [30, 30], minZoom: Z.close, face: true }] });
    }
    // THE MOMENT: the second channel, swollen by the storm, to the chest
    terra.sprites.set('waders', { path: way(W.waders), label: 'Wading the second channel, the water to their chests, shields over their heads', touchable: true,
                                  looks: looks(col.far, { n: 'waders-mid', f: 6, size: [40, 14], face: true }, { n: 'waders', f: 8, size: [170, 130], fps: 6 }) });
    // on the far bank: the horse ahead, the foot following
    terra.sprites.set('advance-horse', { path: way(shift(W.advance, HR.ford, HR.field - 1.0)), label: "Alexander's horse, ahead", touchable: true,
                                         looks: looks({ n: 'cavalry-mid', f: 4, size: [15, 5] }, { n: 'cavalry-mid', f: 4, size: [30, 10], face: true }, { n: 'cavalry', f: 6, size: [130, 44], fps: 8, face: true }) });
    terra.sprites.set('advance-foot', { path: way(shift(W.advance, HR.ford + 0.3, HR.field)), label: 'The foot, following at a steady pace', touchable: true,
                                        looks: looks(col.far, { n: 'phalanx-mid', f: 4, size: [36, 12], face: true }, { n: 'phalanx', f: 6, size: [96, 40], fps: 5, face: true }) });
    // Porus' son's chariots, down the bank and into the clay
    terra.sprites.set('chariots', { path: way(W.chariots), label: "Porus' son's chariots, riding for the landing", touchable: true,
                                    looks: looks({ n: 'chariots-mid', f: 4, size: [15, 5] }, { n: 'chariots-mid', f: 4, size: [44, 14], face: true }, { n: 'chariots', f: 6, size: [130, 40], fps: 8, face: true }) });
    var bog = W.chariots[W.chariots.length - 1];
    terra.sprites.set('chariots-bogged', { path: stand(bog, HR.routed, HR.end + 1), label: 'The chariots, bogged in the clay', touchable: true,
                                           looks: looks(null, { n: 'chariots-mid', f: 4, size: [30, 10], fps: 0.5 }, { n: 'chariots-bogged', f: 4, size: [110, 40], fps: 2 }) });
    // Porus' army to the field, its line facing the landing, then turning on the horse that came round
    var line = { far: { n: 'line-mid', f: 4, size: [30, 8] }, mid: { n: 'line-mid', f: 4, size: [60, 16], face: true } };
    terra.sprites.set('porus', { path: way(W.porus), label: "Porus' army marching to meet him: elephants, foot, horse, chariots", touchable: true,
                                 looks: looks(line.far, line.mid, { n: 'line', f: 6, size: [240, 240], fps: 4 }) });
    terra.sprites.set('porus-line', { path: stand(P.field, HR.field, HR.turn), label: "Porus' line: the elephants a plethron apart, the foot behind", touchable: true,
                                      looks: looks(line.far, line.mid, { n: 'line', f: 6, size: [240, 240], fps: 2 }) });
    terra.sprites.set('porus-turning', { path: stand(P.field, HR.turn, HR.end + 1), label: 'The line turning on the horse that came round it', touchable: true,
                                         looks: looks(line.far, line.mid, { n: 'line-turning', f: 6, size: [240, 240], fps: 4 }) });
    terra.sprites.set('alexander', { path: way(W.alexander), label: "Alexander's Companions, round the left of the line", touchable: true,
                                     looks: looks({ n: 'cavalry-mid', f: 4, size: [15, 5] }, { n: 'cavalry-mid', f: 4, size: [30, 10], face: true }, { n: 'cavalry', f: 6, size: [130, 44], fps: 8, face: true }) });
    terra.sprites.set('coenus', { path: way(W.coenus), label: "Coenus' horse, round the right and behind", touchable: true,
                                  looks: looks({ n: 'cavalry-mid', f: 4, size: [15, 5] }, { n: 'cavalry-mid', f: 4, size: [30, 10], face: true }, { n: 'archers', f: 6, size: [86, 36], fps: 8, face: true }) });
    terra.sprites.set('phalanx', { path: way(W.phalanx), label: 'The phalanx, coming on', touchable: true,
                                   looks: looks(col.far, { n: 'phalanx-mid', f: 4, size: [36, 12], face: true }, { n: 'phalanx', f: 6, size: [96, 40], fps: 5, face: true }) });
    terra.sprites.set('craterus', { path: way(W.craterus), label: 'Craterus crossing from the great camp at last', touchable: true,
                                    looks: looks(col.far, col.mid, col.close) });
    // the storm: rain drifting over the plain's edges, lightning over the hills, from the storm to daybreak
    var rainLooks = [{ frames: frames('rain', 8), fps: 9, size: [70, 110], maxZoom: Z.valley }, { frames: frames('rain', 8), fps: 9, size: [150, 240], minZoom: Z.valley, maxZoom: Z.mid },
                     { frames: frames('rain', 8), fps: 9, size: [250, 400], minZoom: Z.mid }];
    rainAt().forEach(function (p, i) { terra.sprites.set('rain-' + i, { path: stand(p, HR.storm, HR.calm), label: '', looks: rainLooks }); });
    lightningAt().forEach(function (p, i) {
      terra.sprites.set('lightning-' + i, { path: stand(p, HR.storm, HR.calm), label: '',
                                            looks: [{ frames: frames('lightning', 3), fps: 8, size: [400, 400], every: 6 + i * 3.5, for: 0.45 }] });
    });
  }
  /** A way's hours moved to run from h0 to h1 (the horse ahead of the foot on the same road). */
  function shift(w, h0, h1) {
    var a = w[0][2], b = w[w.length - 1][2];
    return w.map(function (p) { return [p[0], p[1], h0 + (p[2] - a) / (b - a) * (h1 - h0)]; });
  }
  function rainAt() {
    var c = [(H.wide[0] + H.wide[2]) / 2, (H.wide[1] + H.wide[3]) / 2], dx = (H.wide[2] - H.wide[0]) * 0.3, dy = (H.wide[3] - H.wide[1]) * 0.3;
    return [[c[0] - dx, c[1] + dy * 0.6], [c[0] + dx * 0.9, c[1] + dy], [c[0] - dx * 0.2, c[1] - dy], [c[0] + dx, c[1] - dy * 0.5], [c[0] - dx * 0.9, c[1] - dy * 0.3], [c[0] + dx * 0.3, c[1] + dy * 0.9]];
  }
  function lightningAt() { return [[73.36, 32.86], [73.50, 32.79], [73.30, 32.72]]; }

  // ----- the night, dragged along the way ------------------------------------------------------------
  var hour = HR.dusk, playing = false, chain = H.chain, at = 0, dragging = false, dragged = false;
  var leaned = {};
  function setHour(h, byHost) {
    var was = hour;
    hour = Math.max(HR.dusk, Math.min(HR.end, h));
    if (byHost) terra.time.set(yr(hour));
    ground(hour);
    // the second channel: the globe comes close on its own, once, as the night reaches it (not while a
    // finger is on the globe: the host drops a flight then; the drag's end asks again)
    if (was < HR.boats && hour >= HR.boats && hour < HR.island) lean('boats');
    if (was < HR.bank && hour >= HR.bank && hour < HR.ford) lean('wade');
  }
  function lean(beat) {
    if (leaned[beat] || dragging) return;
    leaned[beat] = true;
    var p = P.island, c = terra.camera, at = zoomAtKm(Math.min(PPK.deepest, (c ? Math.min(c.width, c.height) : 800) * 0.9 / 5));
    terra.flyTo((p[0] + P.landing[0]) / 2, (p[1] + P.landing[1]) / 2, Math.min(Z.max, at), terra.settings.reduceMotion ? 0 : 1.6);
  }
  /** The chain's index whose hour is nearest the hour now (the finger's search starts there). */
  function indexAt(h) {
    var best = 0;
    for (var i = 0; i < chain.length; i++) if (Math.abs(chain[i][2] - h) < Math.abs(chain[best][2] - h)) best = i;
    return best;
  }
  /** The chain point nearest the finger within a window of the one the night is at, and its hour;
      null when the finger is far from the way (a fingertip and a half). */
  function under(x, y, c) {
    var lo = Math.max(0, at - 30), hi = Math.min(chain.length - 1, at + 30), best = -1, bestD = Infinity, bestQ = null;
    for (var i = lo; i <= hi; i++) {
      var q = terra.project(chain[i][0], chain[i][1], c);
      if (!q || !q.visible) continue;
      var d = Math.hypot(q.x - x, q.y - y);
      if (d < bestD) { bestD = d; best = i; bestQ = q; }
    }
    if (best < 0 || bestD > 70) return null;
    // between this point and the nearer neighbour, by projection
    var h = chain[best][2], j = best;
    for (var k = -1; k <= 1; k += 2) {
      var n = best + k;
      if (n < lo || n > hi) continue;
      var r = terra.project(chain[n][0], chain[n][1], c);
      if (!r) continue;
      var vx = r.x - bestQ.x, vy = r.y - bestQ.y, L = vx * vx + vy * vy;
      if (L < 1e-6) continue;
      var t = ((x - bestQ.x) * vx + (y - bestQ.y) * vy) / L;
      if (t > 0 && t <= 1) { h = chain[best][2] + (chain[n][2] - chain[best][2]) * t; j = t > 0.5 ? n : best; break; }
    }
    return { hour: h, index: j };
  }

  terra.on('touch', function (t) {
    if (!t.target || !t.target.sprite) return;
    var c = terra.camera;
    if (t.phase === 'tap') {
      // the camp at dusk: the column peels away from it
      if ((t.target.sprite === 'camp-a') && hour < HR.leave) { setHour(HR.leave + 0.25, true); if (playing) terra.time.play(false); return; }
      if (NOTES.touched[t.target.sprite]) say(NOTES.touched[t.target.sprite]);
      return;
    }
    if (t.phase === 'began') {
      dragging = true;
      if (!dragged) { dragged = true; say(''); }
      if (playing) terra.time.play(false);
      at = indexAt(hour);
      return;
    }
    if (!dragging || t.lon == null) return;
    var u = under(t.x, t.y, c);
    if (u) { at = u.index; setHour(u.hour, true); }
    if (t.phase === 'ended') { dragging = false; if (hour >= HR.boats && hour < HR.island) lean('boats'); if (hour >= HR.bank && hour < HR.ford) lean('wade'); }
  });

  terra.on('time', function (t) {
    playing = t.playing;
    var h = hr(t.value);
    if (Math.abs(h - hour) > 1e-4) { setHour(h, false); }
    // the night moved by the strip or by play: the tip has done its work
    if (!dragged && h > HR.leave + 0.5) { dragged = true; say(''); }
    // the night done: it waits at the end rather than ending the experience
  });

  terra.on('camera', function (c) {
    if (!c) return;
    if (measure(c)) { scales(); nightShown = -1; ground(hour); place(); }
  });

  // ----- the words --------------------------------------------------------------------------------------
  var caption = { title: NOTES.title, text: '', wikipedia: NOTES.wikipedia };
  function say(text) { terra.caption.set({ title: caption.title, text: text, wikipedia: caption.wikipedia }); }
  terra.on('open', function () { terra.panel.show(NOTES.panel); });
  terra.on('settings', function (s) { terra.caption.set(caption); });

  // ----- the start --------------------------------------------------------------------------------------
  measure(terra.camera);
  scales();
  terra.caption.set(caption);
  terra.globe.set({ historical: true, still: true, maxZoom: Z.max });
  var pace = [[yr(HR.dusk), 0], [yr(HR.leave), 6], [yr(HR.headland), 40], [yr(HR.calm), 64], [yr(HR.ford), 92], [yr(HR.field), 108], [yr(HR.end), 136]];
  terra.time.show({ unit: 'year', pace: pace, value: yr(HR.dusk), playable: true, playing: false,
                    ticks: [HR.dusk, HR.leave, HR.storm, HR.headland, HR.boats, HR.island, HR.bank, HR.ford, HR.routed, HR.field, HR.turn, HR.craterus].map(yr) });
  ground(hour);
  place();
  // the first sight: the valley, both camps and the bend in view (by a doorway the reader is near already)
  // the first sight: the two camps at the centre with the river between them, their fires, the elephants
  // at the water (the bend is a drag away, down the river)
  var mid = [(P.campA[0] + P.campP[0]) / 2 - 0.012, (P.campA[1] + P.campP[1]) / 2 - 0.012], c0 = terra.camera;
  var span = Math.min(c0 ? c0.width : 800, c0 ? c0.height * 0.6 : 800);
  terra.flyTo(mid[0], mid[1], zoomAtKm(Math.max(PPK.valley, Math.min(PPK.close * 0.8, span * 0.9 / 22))), terra.settings.reduceMotion ? 0 : 2.2);
})();
